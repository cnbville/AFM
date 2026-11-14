//! What a player sees of his place (the promises design,
//! `afm/promises/proposal.md` section 2; Cain 2026-11-13): never your depth
//! chart. He reads his place from his practice group in his last two
//! practice weeks (the ones, the twos, or apart from the two-deep;
//! `SeatTraining::groups`) and from his starts and snaps in his last two
//! games. A chart change you undo before the week's practice is set never
//! shows. Out of season, with no practice and no games, he knows the last
//! thing he saw: last season's role.
//!
//! Every promise check and every read of "is he the starter" that a player
//! would make reads this, never `seat_depth` (lane P1). Your staff, your
//! boss and boosters still read the chart.

use crate::calendar::Date;
use crate::game::stats::StatLine;
use crate::player::PlayingTime;
use crate::positions::Position;
use crate::training::seat::GROUP_ONES;
use crate::world::World;

pub use super::claims::player::START_SHARE;

/// Sightings that make his place: the last two practice weeks and the last
/// two games (section 2's windows; proposed, not confirmed).
pub const WINDOW: usize = 2;

/// How far back his games count as what he saw last (out of season, the
/// last thing he saw is last fall's).
pub const SEASON_DAYS: i32 = 240;

/// Whether a position plays on offense (its snaps are offense snaps).
pub fn offense(pos: Position) -> bool {
    matches!(pos, Position::QB | Position::RB | Position::WR | Position::TE | Position::OT | Position::IOL)
}

/// His snaps in a game line on his side of the ball.
pub fn snaps(l: &StatLine, offense: bool) -> u32 {
    if offense { l.snaps_off } else { l.snaps_def }
}

/// He was out hurt on a day (your training room's list).
pub fn hurt_on(w: &World, id: u32, d: Date) -> bool {
    w.training.hurt.iter().any(|h| h.id == id && h.from <= d && d <= h.until)
}

/// His practice groups, newest first: (the week's day, his group).
pub fn weeks(w: &World, id: u32) -> Vec<(Date, u8)> {
    w.training
        .groups
        .iter()
        .rev()
        .filter(|x| x.0 <= w.today)
        .filter_map(|x| x.1.iter().find(|y| y.0 == id).map(|y| (x.0, y.1)))
        .collect()
}

/// One of your games as he saw it.
#[derive(Clone, Debug)]
pub struct Game {
    pub date: Date,
    /// Out hurt that day: the game says nothing about your word.
    pub hurt: bool,
    /// His line, None when he didn't play.
    pub line: Option<StatLine>,
    /// The most snaps any of your men played on his side of the ball.
    pub most: u32,
    pub offense: bool,
    /// A named teammate's snaps, and whether he was out hurt.
    pub mate: Option<(u32, bool)>,
}

impl Game {
    pub fn snaps(&self) -> u32 {
        self.line.as_ref().map_or(0, |l| snaps(l, self.offense))
    }
    /// He started it, or as good as: half the most snaps on his side.
    pub fn started(&self) -> bool {
        self.most > 0 && self.snaps() as f64 >= START_SHARE * self.most as f64
    }
    /// His share of the most snaps on his side.
    pub fn share(&self) -> f64 {
        if self.most == 0 { 0.0 } else { self.snaps() as f64 / self.most as f64 }
    }
    /// Carries plus targets.
    pub fn touches(&self) -> u32 {
        self.line.as_ref().map_or(0, |l| l.rush + l.targets)
    }
}

/// Your games from `from` to `to` (both kept) that he was with you for,
/// oldest first, with a named teammate's snaps when `mate` is given.
pub fn games(w: &World, id: u32, from: Date, to: Date, mate: Option<u32>) -> Vec<Game> {
    let Some(i) = super::person(w, id) else { return Vec::new() };
    let Some(team) = super::Team::seat(w) else { return Vec::new() };
    let pos = w.people[i].player.position;
    let off = offense(pos);
    let since = w.training.arrived.iter().find(|x| x.0 == id).map(|x| x.1);
    let mut out = Vec::new();
    for g in w.seat_games.iter().filter(|g| g.date >= from && g.date <= to && g.date <= w.today) {
        let Some(side) = super::claims::side_of(g, team) else { continue };
        let line = g.stats.players[side].get(&id).cloned();
        let hurt = hurt_on(w, id, g.date);
        // A game before he came (or one he isn't known to have been here
        // for) says nothing.
        if line.is_none() && !hurt && since.is_none_or(|d| d > g.date) {
            continue;
        }
        let most = g.stats.players[side].values().map(|l| snaps(l, off)).max().unwrap_or(0);
        let mate = mate.map(|m| (g.stats.players[side].get(&m).map_or(0, |l| snaps(l, off)), hurt_on(w, m, g.date)));
        out.push(Game { date: g.date, hurt, line, most, offense: off, mate });
    }
    out.sort_by_key(|g| g.date);
    out
}

/// His place as he sees it now: with the ones or a starter in most of his
/// last two practice weeks and last two games of this season (the newest
/// sighting breaks a tie). None when he has seen nothing yet this season.
pub fn first_now(w: &World, i: usize) -> Option<bool> {
    let p = &w.people[i];
    if p.player.position.is_specialist() {
        return None;
    }
    let id = p.player.id;
    let mut seen: Vec<(Date, bool)> = weeks(w, id).into_iter().take(WINDOW).map(|x| (x.0, x.1 == GROUP_ONES)).collect();
    let open = w.today.add(-SEASON_DAYS);
    let mut gs: Vec<Game> = games(w, id, open, w.today, None).into_iter().filter(|g| !g.hurt).collect();
    gs.reverse();
    seen.extend(gs.into_iter().take(WINDOW).map(|g| (g.date, g.started())));
    if seen.is_empty() {
        return None;
    }
    seen.sort_by_key(|x| std::cmp::Reverse(x.0));
    let firsts = seen.iter().filter(|x| x.1).count();
    let n = seen.len();
    Some(if 2 * firsts == n { seen[0].1 } else { 2 * firsts > n })
}

/// A starter in his own eyes: what he has seen this season, else last
/// season's role (the last thing he saw).
pub fn starter(w: &World, i: usize) -> bool {
    first_now(w, i).unwrap_or(w.people[i].last_role == Some(PlayingTime::Starter))
}

/// The men at his spot who work with the ones in his newest practice week
/// (not him): the men he has to pass.
pub fn ahead(w: &World, i: usize) -> Vec<u32> {
    let p = &w.people[i];
    let (id, pos) = (p.player.id, p.player.position);
    let Some((_, g)) = w.training.groups.iter().rev().find(|x| x.0 <= w.today && x.1.iter().any(|y| y.0 == id)) else {
        return Vec::new();
    };
    g.iter()
        .filter(|x| {
            x.0 != id && x.1 == GROUP_ONES && super::person(w, x.0).is_some_and(|j| w.people[j].player.position == pos)
        })
        .map(|x| x.0)
        .collect()
}

/// He worked with the ones in each of his last two practice weeks.
pub fn with_the_ones(w: &World, id: u32) -> bool {
    let g: Vec<u8> = weeks(w, id).into_iter().take(WINDOW).map(|x| x.1).collect();
    g.len() == WINDOW && g.iter().all(|&x| x == GROUP_ONES)
}

/// The day a chart change reaches him: the next practice week that has
/// him in a different group than the one before (None while it hasn't).
pub fn group_changed(w: &World, id: u32, since: Date) -> Option<(Date, u8)> {
    let mut ws = weeks(w, id);
    ws.reverse();
    let before = ws.iter().rev().find(|x| x.0 < since).map(|x| x.1);
    ws.into_iter().find(|x| x.0 >= since && Some(x.1) != before)
}
