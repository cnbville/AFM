//! The detectors (S2, S6, S9): they read the world's own state and notice;
//! they never bend an outcome. Each kind's detector opens a story when the
//! world shows it (a quarterback battle when two are close in camp, a hot
//! seat when the record and the boss's patience say so), feeds it facts as
//! they happen, and closes it on the fact that answers its question (the
//! first start, the firing, the signature). Looks run at three speeds:
//! after every game (`games.rs`), every day for what happens on a day
//! (`daily.rs`: hirings and firings, arrests, commitments, the portal, the
//! draft, the market), and on Mondays for what builds over weeks
//! (`weekly.rs`: seats, battles, races, arcs, the season's end).
//!
//! The world tells stories about every FBS and FCS program, every NFL
//! club, the other leagues' clubs you coach in, and the high schools people
//! follow (yours, the national names, each state's best and your state's
//! top 25); a smaller program gets one when you coach there or face it.
//!
//! Thresholds are Claude's picks (proposed, not confirmed), set against
//! how often real seasons tell each story.

pub mod daily;
pub mod games;
pub mod weekly;

use std::collections::BTreeMap;

use super::{K, Part, Spot, Who, close, notice, open_id, subj, tcode};
use crate::calendar::{Date, Level};
use crate::player::PlayingTime;
use crate::positions::Position;
use crate::world::data::Division;
use crate::world::stories::TeamKey;
use crate::world::{Stage, World};

/// What the detectors remember between looks.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Watch {
    /// Each team's head coach and coordinators at the last look (team code
    /// to coach + 1, 0 for none).
    pub(crate) heads: BTreeMap<u64, [u32; 3]>,
    /// This season's run of results per team (team code to the streak: a
    /// win streak above zero, a losing streak below) and the season.
    pub(crate) runs: BTreeMap<u64, i16>,
    pub(crate) runs_year: i32,
    /// Kickers' tries this season (player id to made, tries, and the last
    /// three games' made and tries).
    pub(crate) kicks: BTreeMap<u32, [u16; 4]>,
    /// Recruits followed (player id to the commitment's program + 1, 1 once
    /// he has signed, his stars, and his high school + 1).
    pub(crate) recruits: BTreeMap<u32, [u32; 4]>,
    /// The season the followed recruits were last picked.
    pub(crate) recruits_on: Option<Date>,
    /// Court and league cases seen open (player id to the day it opened).
    pub(crate) cases: BTreeMap<u32, Date>,
    /// College incidents waiting for their day in the news (day, player id).
    pub(crate) incidents: Vec<(Date, u32)>,
    /// Players with a story flag at the last look (holdout, trade request,
    /// suspension, portal): player id to the flags.
    pub(crate) flags: BTreeMap<u32, u8>,
    /// The high schools people follow this season (sorted) and the season.
    pub(crate) hs: Vec<u32>,
    pub(crate) hs_year: i32,
    /// Life events and second opinions already told (player id, day).
    pub(crate) told: Vec<(u32, Date)>,
    /// The QB who started each team's last game (team code to player id).
    pub(crate) starters: BTreeMap<u64, u32>,
    /// Each rivalry's run (the pair to the winner's team code and wins in
    /// a row), carried from season to season.
    pub(crate) rivals: BTreeMap<u64, (u64, u16)>,
    /// The pairs that met this season and who won (team code, 0 a tie).
    pub(crate) met: BTreeMap<u64, u64>,
    /// Conference moves already told (program ids), NCAA cases seen per
    /// program, and projects seen.
    pub(crate) realign: Vec<u32>,
    pub(crate) ncaa: BTreeMap<u32, u32>,
    pub(crate) projects: u32,
    /// The consensus board last read (its date).
    pub(crate) board: Option<Date>,
    /// Offers to you already told (employer code, day made).
    pub(crate) offers: Vec<(u64, Date)>,
    /// This offseason's portal entrants per program (entrants, starters)
    /// and the football year they count for.
    pub(crate) exodus: BTreeMap<u32, [u16; 2]>,
    pub(crate) exodus_year: i32,
}

/// Flags on `Watch::flags`.
pub(crate) const F_HOLDOUT: u8 = 1;
pub(crate) const F_WANTS_OUT: u8 = 2;
pub(crate) const F_SUSPENDED: u8 = 4;
pub(crate) const F_PORTAL: u8 = 8;
pub(crate) const F_OPTED: u8 = 16;
/// An off-field incident this season already queued for the news.
pub(crate) const F_INCIDENT: u8 = 64;
/// Franchise or transition tagged this league year.
pub(crate) const F_TAGGED: u8 = 128;

/// Once a day, before the stories step (`storylines::day`).
pub(crate) fn day(w: &mut World, d: Date) {
    if w.storylines.watch.hs_year != w.year && d >= w.dates.fall_camp.add(-30) {
        follow_high_schools(w);
    }
    daily::day(w, d);
    if d.weekday() == crate::calendar::Weekday::Mon {
        weekly::monday(w, d);
    }
}

/// The high schools people follow: yours, the national names, each
/// state's best three and your state's top 25 (once a season).
fn follow_high_schools(w: &mut World) {
    let mut by_state: BTreeMap<usize, Vec<(f64, usize)>> = BTreeMap::new();
    let mut v: Vec<u32> = Vec::new();
    for s in &w.high_schools {
        if s.national {
            v.push(s.id as u32);
        }
        by_state.entry(s.state).or_default().push((s.prestige, s.id));
    }
    let mine = match crate::world::stories::seat_team(w) {
        Some(TeamKey::Hs(h)) => w.high_schools.get(h as usize).map(|s| (s.state, h)),
        _ => None,
    };
    for (state, mut list) in by_state {
        list.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let n = if mine.is_some_and(|m| m.0 == state) { 25 } else { 3 };
        v.extend(list.into_iter().take(n).map(|x| x.1 as u32));
    }
    if let Some((_, h)) = mine {
        v.push(h);
    }
    v.sort_unstable();
    v.dedup();
    w.storylines.watch.hs = v;
    w.storylines.watch.hs_year = w.year;
}

/// Whether the world tells stories about a team.
pub(crate) fn on(w: &World, t: TeamKey) -> bool {
    match t {
        TeamKey::College(p) => {
            w.programs.get(p as usize).is_some_and(|x| matches!(x.division, Division::FBS | Division::FCS))
                || mine(w, t)
        }
        TeamKey::Nfl(_) => true,
        TeamKey::Hs(h) => w.storylines.watch.hs.binary_search(&h).is_ok() || mine(w, t),
        TeamKey::Other(_) => mine(w, t),
    }
}

/// Your team or a team you face this season.
pub(crate) fn mine(w: &World, t: TeamKey) -> bool {
    crate::world::stories::seat_team(w) == Some(t)
}

/// A player's id.
pub(crate) fn pid(w: &World, i: usize) -> u32 {
    w.people[i].player.id
}

/// The team a player is on now.
pub(crate) fn team_of(w: &World, i: usize) -> Option<TeamKey> {
    let p = &w.people[i];
    match p.stage {
        Stage::College => p.college.map(|c| TeamKey::College(c as u32)),
        Stage::Pro => p.pro.as_ref().and_then(|c| c.team).map(|t| TeamKey::Nfl(t as u16)),
        Stage::HighSchool => Some(TeamKey::Hs(p.hs as u32)),
        _ => None,
    }
}

/// A team's roster (person indices).
pub(crate) fn roster(w: &World, t: TeamKey) -> Vec<usize> {
    super::folk_team(t).roster(w)
}

/// A team's players at a position, best first.
pub(crate) fn at(w: &World, t: TeamKey, pos: Position) -> Vec<usize> {
    if let TeamKey::Nfl(n) = t
        && let Some(c) = w.clubs.get(n as usize)
        && let Some(d) = c.depth.get(pos as usize)
        && !d.is_empty()
    {
        return d.clone();
    }
    let mut v: Vec<usize> = roster(w, t).into_iter().filter(|&i| w.people[i].player.position == pos).collect();
    v.sort_by(|&a, &b| w.people[b].player.overall().total_cmp(&w.people[a].player.overall()).then(a.cmp(&b)));
    v
}

/// Whether a player started last season (the incumbent in a battle).
pub(crate) fn started(w: &World, i: usize) -> bool {
    w.people[i].last_role == Some(PlayingTime::Starter)
}

/// The head coach's index.
pub(crate) fn head(w: &World, t: TeamKey) -> Option<usize> {
    super::tell::head_coach(w, t)
}

/// A coach's career wins as a head coach, across every stint.
pub(crate) fn career_wins(w: &World, c: usize) -> u32 {
    let co = &w.coaches[c];
    let past: u32 = co.history.iter().filter(|s| s.role == crate::world::coach::Role::HeadCoach).map(|s| s.wins).sum();
    past + if co.job.is_some_and(|j| j.role == crate::world::coach::Role::HeadCoach) { co.wins } else { 0 }
}

/// A story's subject for a player, a coach, a team's season, two teams.
pub(crate) fn s_player(id: u32) -> u64 {
    id as u64
}
pub(crate) fn s_coach(c: usize) -> u64 {
    1 << 40 | c as u64
}
pub(crate) fn s_team(t: TeamKey) -> u64 {
    tcode(t)
}
pub(crate) fn s_season(t: TeamKey, year: i32) -> u64 {
    subj(tcode(t), year as u64)
}
pub(crate) fn s_pair(a: TeamKey, b: TeamKey) -> u64 {
    let (x, y) = (tcode(a), tcode(b));
    subj(x.min(y), x.max(y))
}

/// Closes the open story of a kind about a subject.
pub(crate) fn end(w: &mut World, kind: K, subject: u64, ending: &str, text: Option<String>) {
    if let Some(id) = open_id(w, kind, subject) {
        close(w, id, ending, text);
    }
}

/// Every open story of a kind (ids and subjects), for the closing looks.
pub(crate) fn open_of(w: &World, kind: K) -> Vec<(u32, u64)> {
    let k = kind.id();
    w.storylines.open.iter().filter(|s| s.kind == k && s.phase.open()).map(|s| (s.id, s.subject)).collect()
}

/// A player-led story's spot with his team.
pub(crate) fn player_spot(w: &World, kind: K, i: usize) -> Spot {
    let id = pid(w, i);
    let mut s = Spot::new(kind, s_player(id)).lead(Who::Player(id));
    if let Some(t) = team_of(w, i) {
        s = s.team(t);
        if let Some(c) = head(w, t) {
            s = s.with(Who::Coach(c as u32), Part::Decider);
        }
    }
    s
}

/// A team story's spot with its head coach as the decider.
pub(crate) fn team_spot(w: &World, kind: K, t: TeamKey, subject: u64) -> Spot {
    let mut s = Spot::new(kind, subject).team(t).with(Who::Team(t), Part::Other);
    if let Some(c) = head(w, t) {
        s = s.with(Who::Coach(c as u32), Part::Decider);
    }
    s
}

/// A coach-led story's spot with his boss.
pub(crate) fn coach_spot(_w: &World, kind: K, c: usize, t: TeamKey) -> Spot {
    Spot::new(kind, s_coach(c)).team(t).lead(Who::Coach(c as u32)).with(Who::Boss(t), Part::Boss)
}

/// Opens (or feeds) a story, where the kind happens at the team's level.
pub(crate) fn tell(w: &mut World, s: Spot) -> Option<u32> {
    let def = s.kind.def();
    let fits = def.levels == super::catalogue::REC || s.team.is_none_or(|t| def.at(t.level()));
    if !fits {
        return None;
    }
    notice(w, s)
}

/// A team's name for facts.
pub(crate) fn tname(w: &World, t: TeamKey) -> String {
    crate::world::stories::team_in_text(w, t)
}

/// A person's name.
pub(crate) fn pname(w: &World, i: usize) -> String {
    w.people[i].name.clone()
}

/// A coach's name.
pub(crate) fn cname(w: &World, c: usize) -> String {
    w.coaches.get(c).map_or_else(String::new, |c| c.name.clone())
}

/// A college program's team key.
pub(crate) fn college(p: usize) -> TeamKey {
    TeamKey::College(p as u32)
}

/// A level's games this season are under way.
pub(crate) fn in_season(w: &World, level: Level) -> bool {
    let d = w.today;
    match level {
        Level::College => {
            w.college.as_ref().is_some_and(|c| c.started && !c.finished)
                && w.dates.cfb_weeks.first().is_some_and(|&f| d >= f)
        }
        Level::Nfl => w.dates.nfl_weeks.first().is_some_and(|&f| d >= f) && d <= w.dates.super_bowl,
        Level::HighSchool => {
            w.hs_season.as_ref().is_some_and(|s| s.on_engine || !s.results.is_empty())
                && d >= w.dates.hs_school_start
                && d.month() >= 8
                && d.month() <= 12
        }
        _ => false,
    }
}
