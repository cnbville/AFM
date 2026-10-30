//! Game-day talks on the world's side (dialogue D4-D6): the day each team
//! brings into a game (the stakes, the rival, its captains) and, for your
//! team, what your own week adds (the tones you've used, how often you've
//! handed the talk to a captain, last week's result, what their players said
//! in the paper). After your game, your sideline words land on your
//! players' trust in you, and your talks go in the log the next game's wear
//! reads. The talks themselves are the match engine's (`game::talk`), the
//! same for every team.

use super::Team;
use super::voice::{Facts, Pen, Speaker};
use crate::calendar::{Date, Weekday};
use crate::game::GameTeam;
use crate::game::talk::{Day, Move, Pick, Record, Unit};
use crate::player::Trait;
use crate::positions::Position;
use crate::world::World;

/// A talk you gave (pregame or halftime), kept for the wear of your tones.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Given {
    pub date: Date,
    pub pick: Pick,
    pub halftime: bool,
}

/// The day a team brings into a game: the stakes the world reads (0 lets
/// the engine read them from the playoffs and the season's push), whether
/// it is a rivalry game, its captains, the starter it plays without, and
/// for your team your own week.
pub(crate) fn day_for(w: &World, team: Team, opp: Team, stakes: f32, rival: bool, dressed: &GameTeam) -> Day {
    let roster_ids = |ids: &[usize]| -> Vec<u32> { ids.iter().map(|&i| w.people[i].player.id).collect() };
    let captains = match team {
        Team::College(p) => roster_ids(&w.programs[p].captains),
        Team::Nfl(t) => w.clubs.get(t).map(|c| roster_ids(&c.captains)).unwrap_or_default(),
        _ => Vec::new(),
    };
    let hurt = hurt(w, team, dressed);
    let mut d = Day { stakes, rival, captains, hurt, ..Day::default() };
    if Team::seat(w) == Some(team) {
        yours(w, team, opp, &mut d);
    }
    d
}

/// The starter a team plays without today (hurt or sitting): the one the
/// room looks up to most, leadership then years.
fn hurt(w: &World, team: Team, dressed: &GameTeam) -> Option<u32> {
    let here: std::collections::HashSet<u32> = dressed.players.iter().map(|p| p.id).collect();
    team.roster(w)
        .into_iter()
        .filter(|&i| {
            let p = &w.people[i];
            p.last_role == Some(crate::world::PlayingTime::Starter) && !here.contains(&p.player.id)
        })
        .max_by(|&a, &b| {
            let s = |i: usize| {
                let p = &w.people[i].player;
                p.personality.traits.get(Trait::Leadership) + 0.5 * p.age as f64
            };
            s(a).total_cmp(&s(b)).then(w.people[b].player.id.cmp(&w.people[a].player.id))
        })
        .map(|i| w.people[i].player.id)
}

/// Your own week on the day: the wear of your tones over your last four
/// talks, your hand-offs to a captain, last week's result, and what one of
/// theirs said in the paper this week.
fn yours(w: &World, team: Team, opp: Team, d: &mut Day) {
    let log = &w.gameday.talks;
    let recent: Vec<&Given> = log.iter().rev().filter(|g| !g.halftime).take(4).collect();
    for t in crate::game::talk::Tone::ALL {
        let n = log.iter().rev().take(4).filter(|g| g.pick.tone == t && g.pick.mv != Move::Captain).count();
        d.worn[t.index()] = ((n as f32 - 1.0).max(0.0) / 3.0).min(1.0);
    }
    d.handed = recent.iter().filter(|g| g.pick.mv == Move::Captain).count() as u8;
    d.last = match w.seat_games.last() {
        Some(g) if g.date < w.today && g.date.days_to(w.today) <= 16 => {
            let me = match team {
                Team::Nfl(t) | Team::College(t) | Team::High(t) | Team::Other(t) => t,
            };
            let side = if g.teams[0] == me { 0 } else { 1 };
            let s = g.stats.score;
            (s[side] - s[1 - side]).signum() as i8
        }
        _ => 0,
    };
    d.quote = quote(w, opp, w.today);
}

/// Who of theirs talked in the paper about us this week, and about which
/// of our units: a proud starter (ego 14+) a third of the time, keyed by the
/// day so it holds all week (proposed, not confirmed).
pub fn quote(w: &World, opp: Team, day: Date) -> Option<(u32, u8)> {
    let roster = opp.roster(w);
    let mut talkers: Vec<usize> = roster
        .into_iter()
        .filter(|&i| {
            let p = &w.people[i];
            p.player.personality.traits.get(Trait::Ego) >= 14.0
                && p.last_role == Some(crate::world::PlayingTime::Starter)
        })
        .collect();
    talkers.sort_by_key(|&i| w.people[i].player.id);
    let mut monday = day;
    while monday.weekday() != Weekday::Mon {
        monday = monday.add(-1);
    }
    let k = super::roll(w, 0x0E07_E000 ^ monday.0 as u64, opp_key(opp));
    if talkers.is_empty() || k >= 0.34 {
        return None;
    }
    let i = talkers[((k / 0.34) * talkers.len() as f64) as usize % talkers.len()];
    let p = &w.people[i];
    // He talks about the men he plays against.
    let unit = match p.player.position {
        Position::WR | Position::TE => Unit::Secondary,
        Position::RB => Unit::Backers,
        Position::QB => Unit::Secondary,
        Position::OT | Position::IOL => Unit::Front,
        Position::EDGE | Position::DT => Unit::Line,
        Position::LB => Unit::Skill,
        Position::CB | Position::S => Unit::Skill,
        Position::K | Position::P | Position::LS => return None,
    };
    Some((p.player.id, unit.index() as u8))
}

fn opp_key(t: Team) -> u64 {
    match t {
        Team::College(p) => p as u64,
        Team::Nfl(t) => 1 << 32 | t as u64,
        Team::High(h) => 2 << 32 | h as u64,
        Team::Other(f) => 3 << 32 | f as u64,
    }
}

/// A college game's stakes from its tags and kind (dialogue D4; sizes
/// proposed, not confirmed): a playoff spot, two ranked teams, a bowl, a
/// conference title, the playoff itself, a hot seat, a coach back at the
/// school that fired him; and the season's own push.
pub(crate) fn college_stakes(g: &crate::world::schedule::Game, ctx: &crate::game::garbage::Context) -> (f32, bool) {
    use crate::world::schedule::{GameKind, Tag};
    let rival = g.tags.contains(&Tag::Rivalry);
    let mut s: f32 = match g.kind {
        GameKind::Title => 1.0,
        GameKind::Playoff(_) => 0.9,
        GameKind::ConferenceTitle => 0.7,
        GameKind::Bowl(_) | GameKind::Celebration => 0.55,
        _ => 0.25,
    };
    if g.tags.contains(&Tag::PlayoffSpot) {
        s += 0.25;
    }
    if g.tags.contains(&Tag::Ranked) {
        s += 0.1;
    }
    if rival {
        s += 0.25;
    }
    if g.tags.contains(&Tag::CoachReturn) || ctx.grudge {
        s += 0.12;
    }
    if ctx.race {
        s += 0.15;
    }
    if ctx.bowl_push {
        s += 0.1;
    }
    if ctx.hot_seat {
        s += 0.06;
    }
    (s.clamp(0.05, 1.0), rival)
}

/// After your game: each word you gave lands on his trust in you and his
/// mood, your talks go in the log, and the game keeps its talks.
pub(crate) fn after_game(w: &mut World, talks: &Record, mine: usize) {
    let today = w.today;
    for n in &talks.words {
        if n.side as usize != mine || !n.yours {
            continue;
        }
        if let Some(i) = super::person(w, n.id) {
            w.people[i].trust = (w.people[i].trust + n.trust).clamp(0.0, 1.0);
            super::talks::mood_event(w, n.id, super::talks::Reason::Talk, 0.5 * n.trust);
        }
    }
    for (pick, halftime) in [(talks.pregame[mine], false), (talks.halftime[mine], true)] {
        if let Some(pick) = pick {
            w.gameday.talks.push(Given { date: today, pick, halftime });
        }
    }
    let n = w.gameday.talks.len();
    if n > 16 {
        w.gameday.talks.drain(..n - 16);
    }
}

/// A team's coordinators (offense, defense), for the staff's read.
pub fn coordinators(w: &World, t: Team) -> [Option<usize>; 2] {
    match t {
        Team::College(p) => w.programs.get(p).map_or([None; 2], |x| [x.oc, x.dc]),
        Team::Nfl(t) => w.nfl.get(t).map_or([None; 2], |x| [x.oc, x.dc]),
        Team::High(h) => w.high_schools.get(h).map_or([None; 2], |x| [x.oc, x.dc]),
        Team::Other(f) => w.others.teams.get(f).map_or([None; 2], |x| [x.oc, x.dc]),
    }
}

/// What a talk's line needs to know about the moment.
#[derive(Clone, Debug, Default)]
pub struct LineCx {
    /// Their name.
    pub them: String,
    /// At halftime, our margin.
    pub halftime: Option<i32>,
    /// Last week: 1 a win, -1 a loss, 0 none.
    pub last: i8,
    /// The player the talk is about (one player), first name.
    pub player: Option<String>,
    /// The captain you hand it to, first name.
    pub captain: Option<String>,
    pub rival: bool,
    /// Keys the wording: the same seed gives the same line.
    pub seed: u64,
}

fn line_facts(pick: &Pick, cx: &LineCx) -> Facts {
    let mut f = Facts::new();
    f.set("them", cx.them.clone()).set("subject", pick.subject.key());
    match cx.halftime {
        Some(m) => {
            f.set("moment", "halftime").set(
                "lead",
                if m > 0 {
                    "up"
                } else if m < 0 {
                    "down"
                } else {
                    "level"
                },
            );
        }
        None => {
            f.set("moment", "pregame");
        }
    }
    f.set(
        "last",
        match cx.last {
            1 => "win",
            -1 => "loss",
            _ => "none",
        },
    );
    if let Some(p) = &cx.player {
        f.set("player", p.clone());
    }
    if let Some(c) = &cx.captain {
        f.set("captain", c.clone());
    }
    if cx.rival {
        f.set("rival", "yes");
    }
    f
}

/// Your talk in words, roughly (dialogue D4, D5): an opening for the tone,
/// the point for the move and subject, and often a close. The tone is part
/// of your voice; the same seed gives the same line in the preview and in
/// the room.
pub fn talk_line(w: &World, pick: &Pick, cx: &LineCx) -> String {
    if pick.mv == Move::Nothing {
        return String::new();
    }
    let mut who = Speaker::you(w);
    who.tags.push(pick.tone.key());
    let f = line_facts(pick, cx);
    let mut pen = Pen::loose(cx.seed);
    let open = format!("gameday.open.{}", pick.tone.key());
    let point = format!("gameday.{}", pick.mv.key());
    let close = format!("?gameday.close.{}", pick.tone.key());
    pen.line(&[&open, &point, &close], &who, &f)
}

/// What a captain says when you hand him the room (dialogue D4): his own
/// voice (leader, young, loud or quiet, his spot), a few short lines.
pub fn captain_lines(w: &World, i: usize, cx: &LineCx) -> Vec<String> {
    let who = Speaker::player(w, i);
    let f = line_facts(&Pick::default(), cx);
    let mut pen = Pen::loose(cx.seed ^ 0xCA97_A100);
    [
        "gameday.captain.open",
        "gameday.captain.own",
        "gameday.captain.us",
        "gameday.captain.job",
        "gameday.captain.break",
    ]
    .iter()
    .filter_map(|k| pen.part(k, &who, &f))
    .map(|s| super::voice::tidy(&s))
    .collect()
}

/// What their player said in the paper about one of our units.
pub fn quote_line(w: &World, id: u32, unit: Unit, us: &str, day: Date) -> Option<String> {
    let i = super::person(w, id)?;
    let who = Speaker::player(w, i);
    let mut f = Facts::new();
    f.set("us", us.to_string()).set("unit", unit_words(unit));
    let mut pen = Pen::loose(0x0E07_E000 ^ day.0 as u64 ^ (id as u64) << 20);
    pen.part("gameday.quote", &who, &f).map(|s| super::voice::tidy(&s))
}

/// A unit the way players say it.
pub fn unit_words(u: Unit) -> &'static str {
    match u {
        Unit::Qb => "quarterback",
        Unit::Line => "offensive line",
        Unit::Skill => "receivers",
        Unit::Front => "front",
        Unit::Backers => "linebackers",
        Unit::Secondary => "corners",
        Unit::Specialists => "kicker",
    }
}

/// Your word on the sideline to one player, in words (dialogue D6).
pub fn word_line(w: &World, pick: &Pick, mistake: crate::game::talk::Mistake, first: &str, seed: u64) -> String {
    let mut who = Speaker::you(w);
    who.tags.push(pick.tone.key());
    let mut f = Facts::new();
    f.set("player", first.to_string()).set("mistake", mistake.key());
    let mut pen = Pen::loose(seed);
    pen.line(&[&format!("gameday.word.{}", pick.mv.key())], &who, &f)
}

/// A coordinator's read of the room, in his own voice (dialogue D4): `kind`
/// is good, amped, tight, flat, hollow or captain; `player` the man he
/// names.
pub fn staff_line(w: &World, c: usize, kind: &str, player: Option<&str>, seed: u64) -> Option<String> {
    let who = Speaker::coach(w, c);
    let mut f = Facts::new();
    if let Some(p) = player {
        f.set("player", p.to_string());
    }
    let mut pen = Pen::loose(seed ^ c as u64);
    pen.part(&format!("gameday.staff.{kind}"), &who, &f).map(|s| super::voice::tidy(&s))
}
