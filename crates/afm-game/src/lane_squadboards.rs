//! Update 2, lane 9A: the player and roster boards. The depth chart and
//! the needs map in the stadium (V1, V2), the calm roster (V3), a player's
//! shape (V4), his career arc (V5) and his body and testing (V6), at every
//! seat that has a squad: high school, college and the NFL.
//!
//! Every rating here is your staff's range and every level is read against
//! its own players (a high school's state and class, its college division,
//! the NFL). Potential stays words: a projection is the staff's guess from
//! its read and how players like him grew, never his ceiling.

pub mod arc;
pub mod body;
pub mod depth;
pub mod needs;
pub mod roster;
pub mod shape;

use std::collections::HashMap;
use std::sync::Mutex;

use afm_engine::attributes::Attr;
use afm_engine::positions::Position;
use afm_engine::world::norms::{Marks, Pool, Rated};
use afm_engine::world::seat::{KeptRead, SeatGame};
use afm_engine::world::{Person, Stage, World};
use serde_json::{Value, json};

use crate::reads::{self, Read};
use crate::views::{self, TeamRef};
use crate::{Game, Reply};

const COMMANDS: [&str; 8] =
    ["sb_depth", "sb_depth_up", "sb_depth_reset", "sb_needs", "sb_roster", "sb_shape", "sb_arc", "sb_body"];

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    if !COMMANDS.contains(&cmd) {
        return None;
    }
    let id = || args.get("id").and_then(Value::as_u64).map(|v| v as u32).ok_or_else(|| "Missing id.".to_string());
    if cmd == "sb_depth_up" || cmd == "sb_depth_reset" {
        let Some(w) = game.world.as_mut() else { return Some(Err("No world is loaded.".into())) };
        let done = if cmd == "sb_depth_up" { depth::move_up(w, args) } else { depth::reset(w, args).map(|_| None) };
        return Some(done.and_then(|note| {
            let mut v = depth::view(w, args)?;
            v["note"] = json!(note);
            Ok(v)
        }));
    }
    let Some(w) = game.world.as_ref() else { return Some(Err("No world is loaded.".into())) };
    Some(match cmd {
        "sb_depth" => depth::view(w, args),
        "sb_needs" => needs::view(w, args),
        "sb_roster" => roster::view(w),
        "sb_shape" => id().and_then(|i| shape::view(w, i)),
        "sb_arc" => id().and_then(|i| arc::view(w, i)),
        _ => id().and_then(|i| body::view(w, i)),
    })
}

// ---------- shared helpers ----------

/// Your team, at a seat that has a squad.
pub(crate) fn my_team(w: &World) -> Result<TeamRef, String> {
    match views::mine(w) {
        Some(TeamRef::Other(_)) => Err("These boards come with a team in high school, college or the NFL.".into()),
        Some(t) => Ok(t),
        None => Err("You have no team yet.".into()),
    }
}

/// The middle of a read.
pub(crate) fn mid(r: Read) -> Option<f64> {
    match r {
        Read::Range(lo, hi) => Some((lo as f64 + hi as f64) / 2.0),
        Read::NotSeen => None,
    }
}

/// Your staff's overall read of one of your players.
pub(crate) fn own_read(w: &World, t: TeamRef, i: usize) -> Read {
    let p = &w.people[i];
    reads::overall(&p.player, true, views::eye_for(w, t, p.player.position), views::reader_key(w))
}

/// Your staff's read of anyone: narrow for your own players, the public
/// read (wide) for everyone else.
pub(crate) fn any_read(w: &World, i: usize) -> (Read, bool, f64) {
    let p = &w.people[i];
    let team = views::team_of(w, p);
    let own = team.is_some_and(|t| views::is_mine(w, t));
    let eye = team.filter(|_| own).map_or(10.0, |t| views::eye_for(w, t, p.player.position));
    (reads::overall(&p.player, own, eye, views::reader_key(w)), own, eye)
}

/// The pool a team's players are read against: a high school's state and
/// class (every grade: a varsity's starters), its college division, the NFL.
pub(crate) fn team_pool(w: &World, t: TeamRef) -> Option<Pool> {
    use afm_engine::world::norms::tier;
    match t {
        TeamRef::HighSchool(h) => {
            let s = w.high_schools.get(h)?;
            Some(Pool::Hs { state: Some(s.state), class: Some(s.classification), grade: 0 })
        }
        TeamRef::College(c) => Some(Pool::College(tier(w.programs.get(c)?.division))),
        TeamRef::Nfl(_) => Some(Pool::Nfl),
        TeamRef::Other(_) => None,
    }
}

/// The level in words: "Texas 6A", "FBS", "NFL".
pub(crate) fn level_name(w: &World, t: TeamRef) -> String {
    match t {
        TeamRef::HighSchool(h) => {
            let s = &w.high_schools[h];
            format!("{} {}", views::state_name(&w.states[s.state].code), views::hs_class(w, h))
        }
        TeamRef::College(c) => w.programs[c].division.name().to_string(),
        _ => "NFL".into(),
    }
}

/// Starters at a position across the team's level ("FBS starting
/// quarterbacks").
pub(crate) fn starters_words(w: &World, t: TeamRef, pos: Position) -> String {
    let (one, many) = crate::lane_ratings::pos_words(pos);
    match t {
        TeamRef::HighSchool(_) => format!("{} starters at {one}", level_name(w, t)),
        _ => format!("{} starting {many}", level_name(w, t)),
    }
}

/// The level's marks for a rating at a position, read against your team's
/// pool (widened by the norms when the pool is thin).
pub(crate) fn marks(w: &World, t: TeamRef, pos: Position, rated: Rated) -> Option<Marks> {
    crate::lane_ratings::marks_cached(w, team_pool(w, t)?, pos, rated)
}

/// The four ratings that matter most at a position.
pub(crate) fn key_attrs(pos: Position) -> Vec<Attr> {
    let mut key: Vec<(f64, Attr)> =
        Attr::ALL.iter().map(|&a| (afm_engine::positions::weight(pos, a), a)).filter(|x| x.0 > 0.02).collect();
    key.sort_by(|a, b| b.0.total_cmp(&a.0));
    key.into_iter().take(4).map(|x| x.1).collect()
}

/// First name and last name ("Jace", "Hartman"; "Smith Jr." stays whole).
pub(crate) fn first_last(name: &str) -> (String, String) {
    match name.split_once(' ') {
        Some((f, l)) => (f.to_string(), l.to_string()),
        None => (String::new(), name.to_string()),
    }
}

/// His class in full words: "Junior", "Redshirt sophomore", "3rd year".
pub(crate) fn class_full(w: &World, p: &Person) -> String {
    let short = views::class_text(w, p);
    let rs = short.starts_with("RS ");
    let base = match short.trim_start_matches("RS ") {
        "Fr" => "freshman",
        "So" => "sophomore",
        "Jr" => "junior",
        "Sr" => "senior",
        "Rookie" => return "Rookie".into(),
        other => {
            if let Some(n) = other.split(' ').next().and_then(|n| n.parse::<u32>().ok()) {
                return format!("{} year in the league", ordinal(n + 1));
            }
            return other.to_string();
        }
    };
    if rs { format!("Redshirt {base}") } else { cap(base) }
}

pub(crate) fn ordinal(n: u32) -> String {
    let s = match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

pub(crate) fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// Your team's kept games at its level, oldest first, with your side.
pub(crate) fn my_games(w: &World) -> Vec<(&SeatGame, usize)> {
    let Some((level, i)) = w.seat_team() else { return Vec::new() };
    w.seat_games()
        .iter()
        .filter(|g| g.level == level && g.teams.contains(&i))
        .map(|g| (g, if g.teams[0] == i { 0 } else { 1 }))
        .collect()
}

/// A player's snaps in a game on the side of the ball he plays.
pub(crate) fn snaps_in(g: &SeatGame, side: usize, id: u32, pos: Position) -> Option<u32> {
    let l = g.stats.players[side].get(&id)?;
    Some(match views::unit(pos) {
        "OFF" => l.snaps_off,
        "DEF" => l.snaps_def,
        _ => l.snaps_st,
    })
}

/// Your team's snaps on one side of the ball in a game (special teams:
/// every kick).
pub(crate) fn team_snaps(g: &SeatGame, side: usize, pos: Position) -> u32 {
    use afm_engine::game::stats::PlayType;
    let unit = views::unit(pos);
    let logged = g
        .stats
        .plays
        .iter()
        .filter(|r| !r.nullified)
        .filter(|r| match unit {
            "OFF" => r.offense == side && crate::lane_gameplan::is_snap(r.kind),
            "DEF" => r.offense != side && crate::lane_gameplan::is_snap(r.kind),
            _ => matches!(r.kind, PlayType::Punt | PlayType::FieldGoal | PlayType::Kickoff | PlayType::ExtraPoint),
        })
        .count() as u32;
    // A man's own count takes in snaps the log leaves out (a play wiped
    // by a penalty, a kneel), so never less than the most any man played.
    let most = g.stats.players[side]
        .values()
        .map(|l| match unit {
            "OFF" => l.snaps_off,
            "DEF" => l.snaps_def,
            _ => l.snaps_st,
        })
        .max()
        .unwrap_or(0);
    logged.max(most)
}

/// A grade on 0-100 from the game's play grades (60 an average day).
pub(crate) fn game_grade(g: &SeatGame, side: usize, id: u32) -> Option<f64> {
    let l = g.stats.players[side].get(&id)?;
    (l.graded >= 4).then(|| l.grade())
}

/// A letter from a share of the level's starters beaten (0-1).
pub(crate) fn letter(pct: f64) -> &'static str {
    match (pct * 100.0).round() as i32 {
        95.. => "A+",
        88..=94 => "A",
        80..=87 => "A-",
        70..=79 => "B+",
        60..=69 => "B",
        50..=59 => "B-",
        40..=49 => "C+",
        30..=39 => "C",
        20..=29 => "C-",
        10..=19 => "D",
        _ => "F",
    }
}

/// Is he hurt, held out or limited this week.
pub(crate) fn health(w: &World, i: usize) -> (&'static str, String) {
    let id = w.people[i].player.id;
    let (status, why) = crate::lane_training::practice_status(w, id, i);
    if status != "full" {
        return (status, why);
    }
    let s = views::status_text(w, &w.people[i]);
    if s.is_empty() { ("full", String::new()) } else { ("out", s) }
}

/// How ready a man behind the starter is, in a word.
pub(crate) fn readiness(w: &World, i: usize, read: Read, m: Option<&Marks>) -> &'static str {
    let p = &w.people[i];
    if health(w, i).0 == "out" {
        return "Out";
    }
    if p.stage == Stage::College && p.last_role == Some(afm_engine::player::PlayingTime::Redshirt) {
        return "Redshirt";
    }
    match (mid(read), m) {
        (Some(x), Some(m)) if x >= m.starter => "Ready to start",
        (Some(x), Some(m)) if x >= m.backup => "Ready",
        (Some(_), Some(_)) => "Needs reps",
        _ => "Ready",
    }
}

// ---------- the units, ranked against the level ----------

/// Each team's reads at every position, best first: yours (narrow) and
/// the teams you're ranked against (the public read).
pub(crate) struct UnitTable {
    pub mine: Vec<Vec<f32>>,
    pub others: Vec<Vec<Vec<f32>>>,
    /// "in the Big 12", "in the NFL", "in Texas 6A".
    pub among: String,
}

fn team_reads(w: &World, t: TeamRef, own: bool) -> Vec<Vec<f32>> {
    let reader = views::reader_key(w);
    let mut by: Vec<Vec<f32>> = vec![Vec::new(); Position::ALL.len()];
    for k in views::roster_people(w, t) {
        let p = &w.people[k];
        let pos = p.player.position;
        let eye = if own { views::eye_for(w, t, pos) } else { 10.0 };
        if let Some(m) = mid(reads::overall(&p.player, own, eye, reader)) {
            by[pos.index()].push(m as f32);
        }
    }
    for v in &mut by {
        v.sort_by(|a, b| b.total_cmp(a));
    }
    by
}

type UnitKey = (u32, i32, i32, TeamRef, usize);

/// The table for your team, kept for the day.
pub(crate) fn unit_table(w: &World, t: TeamRef) -> std::sync::Arc<UnitTable> {
    static CACHE: Mutex<Option<(UnitKey, std::sync::Arc<UnitTable>)>> = Mutex::new(None);
    let key: UnitKey = (w.options.seed, w.year, w.today.0, t, views::roster_people(w, t).len());
    let mut g = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((k, v)) = g.as_ref()
        && *k == key
    {
        return v.clone();
    }
    let (rivals, among): (Vec<TeamRef>, String) = match t {
        TeamRef::College(c) => {
            let p = &w.programs[c];
            let v = (0..w.programs.len())
                .filter(|&j| j != c && w.programs[j].conference == p.conference && w.programs[j].division == p.division)
                .map(TeamRef::College)
                .collect();
            (v, format!("in the {}", w.conference_name(&p.conference)))
        }
        TeamRef::Nfl(n) => ((0..w.nfl.len()).filter(|&j| j != n).map(TeamRef::Nfl).collect(), "in the NFL".into()),
        TeamRef::HighSchool(h) => {
            let s = &w.high_schools[h];
            let v = (0..w.high_schools.len())
                .filter(|&j| {
                    j != h && w.high_schools[j].state == s.state && w.high_schools[j].classification == s.classification
                })
                .filter(|&j| !w.high_schools[j].roster.is_empty())
                .map(TeamRef::HighSchool)
                .collect();
            (v, format!("in {}", level_name(w, t)))
        }
        TeamRef::Other(_) => (Vec::new(), String::new()),
    };
    let table = std::sync::Arc::new(UnitTable {
        mine: team_reads(w, t, true),
        others: rivals.into_iter().map(|r| team_reads(w, r, false)).collect(),
        among,
    });
    *g = Some((key, table.clone()));
    table
}

/// A unit's starters' average read on one team (None with nobody there).
fn unit_value(by: &[Vec<f32>], spots: &[(Position, usize)]) -> Option<f64> {
    let v: Vec<f64> = spots.iter().flat_map(|&(pos, n)| by[pos.index()].iter().take(n).map(|&x| x as f64)).collect();
    (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
}

/// A unit card: your starters' average read, a letter for where they sit
/// among the level's starters, and the rank among the teams you're
/// measured against.
pub(crate) fn unit_card(w: &World, t: TeamRef, name: &str, spots: &[(Position, usize)]) -> Value {
    let table = unit_table(w, t);
    let Some(value) = unit_value(&table.mine, spots) else {
        return json!({ "name": name, "value": null, "grade": "–", "rank": null, "of": null, "among": table.among });
    };
    // Each starter against the level's starters at his position.
    let mut shares: Vec<f64> = Vec::new();
    for &(pos, n) in spots {
        if let Some(m) = marks(w, t, pos, Rated::Overall) {
            shares.extend(table.mine[pos.index()].iter().take(n).map(|&x| m.below(x as f64)));
        }
    }
    let pct = if shares.is_empty() { 0.5 } else { shares.iter().sum::<f64>() / shares.len() as f64 };
    let rivals: Vec<f64> = table.others.iter().filter_map(|o| unit_value(o, spots)).collect();
    let rank = 1 + rivals.iter().filter(|&&v| v > value).count();
    json!({
        "name": name,
        "value": value.round(),
        "grade": letter(pct),
        "pct": (pct * 100.0).round(),
        "rank": (!rivals.is_empty()).then_some(rank),
        "rank_words": (!rivals.is_empty()).then(|| ordinal(rank as u32)),
        "of": rivals.len() + 1,
        "among": table.among,
    })
}

// ---------- the eight skill families of each position ----------

pub(crate) type Family = (&'static str, &'static [Attr]);

const QB_F: [Family; 8] = {
    use Attr::*;
    [
        ("Arm", &[ArmStrength, DeepAccuracy]),
        ("Accuracy", &[ShortAccuracy, MediumAccuracy, Touch]),
        ("Release", &[QuickRelease, Footwork, BallHandling]),
        ("Pocket", &[PocketPresence, Composure, Toughness]),
        ("Reads", &[FootballIq, Decisions, Anticipation, EyeManipulation]),
        ("On the move", &[ThrowOnMove, Agility, Elusiveness]),
        ("Athlete", &[Speed, Acceleration, BallCarrierVision]),
        ("Leader", &[Communication, Competitiveness, Concentration]),
    ]
};
const RB_F: [Family; 8] = {
    use Attr::*;
    [
        ("Speed", &[Speed, Acceleration]),
        ("Quickness", &[Agility, Balance]),
        ("Vision", &[BallCarrierVision, FootballIq]),
        ("Power", &[LegDrive, Strength, Toughness]),
        ("Elusive", &[Elusiveness, BodyControl]),
        ("Hands", &[Catching, RouteRunning]),
        ("Ball security", &[BallSecurity, Concentration]),
        ("Pass pro", &[PassBlocking, PlayRecognition, Physicality]),
    ]
};
const WR_F: [Family; 8] = {
    use Attr::*;
    [
        ("Speed", &[Speed, Acceleration]),
        ("Quickness", &[Agility, Balance]),
        ("Release", &[Release, Strength]),
        ("Routes", &[RouteRunning, FootballIq, Anticipation]),
        ("Hands", &[Catching, Concentration, ContestedCatch]),
        ("Ball skills", &[BallTracking, BodyControl, Jumping]),
        ("After the catch", &[Elusiveness, BallCarrierVision, BallSecurity]),
        ("Blocking", &[BlockingInSpace, Strength, Toughness]),
    ]
};
const TE_F: [Family; 8] = {
    use Attr::*;
    [
        ("Speed", &[Speed, Acceleration]),
        ("Release", &[Release, Agility]),
        ("Routes", &[RouteRunning, FootballIq, Anticipation]),
        ("Hands", &[Catching, Concentration, ContestedCatch]),
        ("Ball skills", &[BallTracking, BodyControl, Jumping]),
        ("After the catch", &[Elusiveness, LegDrive, BallSecurity]),
        ("Run block", &[RunBlocking, HandUse, Strength]),
        ("Pass pro", &[PassBlocking, BlockingInSpace, Anchor]),
    ]
};
const OL_F: [Family; 8] = {
    use Attr::*;
    [
        ("Pass pro", &[PassBlocking, HandUse]),
        ("Run block", &[RunBlocking, LegDrive]),
        ("Anchor", &[Anchor, Strength]),
        ("Feet", &[Agility, Balance, Flexibility]),
        ("Space", &[BlockingInSpace, Acceleration, Speed]),
        ("Mind", &[FootballIq, PlayRecognition, Communication]),
        ("Finish", &[Physicality, Motor, Toughness]),
        ("Poise", &[Composure, Concentration, Discipline]),
    ]
};
const DL_F: [Family; 8] = {
    use Attr::*;
    [
        ("Get-off", &[Acceleration, Anticipation]),
        ("Power", &[PowerRush, Strength, LegDrive]),
        ("Finesse", &[FinesseRush, Agility, Flexibility]),
        ("Hands", &[HandUse, BlockShedding]),
        ("Run defense", &[Anchor, PlayRecognition, Discipline, FootballIq]),
        ("Pursuit", &[Pursuit, Speed, Motor]),
        ("Tackling", &[Tackling, Physicality]),
        ("Grit", &[Toughness, Competitiveness, Stamina]),
    ]
};
const LB_F: [Family; 8] = {
    use Attr::*;
    [
        ("Run fits", &[PlayRecognition, Discipline, FootballIq]),
        ("Shedding", &[BlockShedding, HandUse, Strength]),
        ("Tackling", &[Tackling, Physicality]),
        ("Range", &[Speed, Pursuit, Acceleration]),
        ("Coverage", &[ZoneCoverage, ManCoverage]),
        ("Blitz", &[Blitzing, FinesseRush, PowerRush]),
        ("Traffic", &[Traffic, Agility, Balance]),
        ("Command", &[Communication, Anticipation, Composure]),
    ]
};
const DB_F: [Family; 8] = {
    use Attr::*;
    [
        ("Speed", &[Speed, Acceleration]),
        ("Hips", &[Agility, Flexibility, Balance]),
        ("Man", &[ManCoverage, Press]),
        ("Zone", &[ZoneCoverage, Disguise, Anticipation]),
        ("Ball skills", &[BallTracking, ContestedCatch, Catching, Jumping]),
        ("Tackling", &[Tackling, Pursuit, Physicality]),
        ("Reads", &[PlayRecognition, FootballIq, Communication]),
        ("Poise", &[Composure, Concentration, Competitiveness]),
    ]
};
const K_F: [Family; 8] = {
    use Attr::*;
    [
        ("Leg", &[KickPower]),
        ("Short range", &[ShortKickAccuracy]),
        ("Long range", &[LongKickAccuracy]),
        ("Trajectory", &[KickTrajectory]),
        ("Kickoffs", &[KickoffTechnique]),
        ("Get-off", &[GetOff]),
        ("Nerve", &[Composure, Concentration]),
        ("Athlete", &[Speed, Tackling]),
    ]
};
const P_F: [Family; 8] = {
    use Attr::*;
    [
        ("Distance", &[PuntPower]),
        ("Placement", &[PuntPlacement]),
        ("Get-off", &[GetOff]),
        ("Hang", &[KickTrajectory]),
        ("Kicking", &[KickPower, ShortKickAccuracy]),
        ("Hands", &[Catching, BallHandling]),
        ("Nerve", &[Composure, Concentration]),
        ("Athlete", &[Speed, Tackling]),
    ]
};
const LS_F: [Family; 8] = {
    use Attr::*;
    [
        ("Long snap", &[LongSnapping]),
        ("Short snap", &[ShotgunSnap]),
        ("Blocking", &[PassBlocking, Anchor]),
        ("Coverage", &[Speed, Pursuit]),
        ("Tackling", &[Tackling, Physicality]),
        ("Nerve", &[Composure, Concentration]),
        ("Strength", &[Strength, HandUse]),
        ("Mind", &[FootballIq, Discipline]),
    ]
};

/// A position's eight skill families and the ratings each is built from.
pub(crate) fn families(pos: Position) -> &'static [Family; 8] {
    match pos {
        Position::QB => &QB_F,
        Position::RB => &RB_F,
        Position::WR => &WR_F,
        Position::TE => &TE_F,
        Position::OT | Position::IOL => &OL_F,
        Position::EDGE | Position::DT => &DL_F,
        Position::LB => &LB_F,
        Position::CB | Position::S => &DB_F,
        Position::K => &K_F,
        Position::P => &P_F,
        Position::LS => &LS_F,
    }
}

/// The low and high of a family from the ratings' reads (None when none
/// of them was seen).
pub(crate) fn family_read(reads: &[(Attr, Read)], f: &Family) -> Option<(f64, f64)> {
    let seen: Vec<(u8, u8)> =
        f.1.iter()
            .filter_map(|a| match reads.iter().find(|x| x.0 == *a).map(|x| x.1) {
                Some(Read::Range(lo, hi)) => Some((lo, hi)),
                _ => None,
            })
            .collect();
    if seen.is_empty() {
        return None;
    }
    let n = seen.len() as f64;
    Some((seen.iter().map(|x| x.0 as f64).sum::<f64>() / n, seen.iter().map(|x| x.1 as f64).sum::<f64>() / n))
}

// ---------- the reads your staff keeps each season ----------

/// Your staff's read of a player today, in the kept form.
pub(crate) fn read_now(w: &World, i: usize, season: i32) -> KeptRead {
    let p = &w.people[i];
    let pos = p.player.position;
    let (ovr, own, eye) = any_read(w, i);
    let attrs = reads::attributes(&p.player, own, eye, views::reader_key(w));
    let pair = |r: Read| match r {
        Read::Range(lo, hi) => [lo, hi],
        Read::NotSeen => [0, 0],
    };
    let mut keys = [[0u8; 3]; 4];
    for (k, a) in key_attrs(pos).into_iter().enumerate() {
        let r = attrs.iter().find(|x| x.0 == a).map_or(Read::NotSeen, |x| x.1);
        let [lo, hi] = pair(r);
        keys[k] = [a.index() as u8, lo, hi];
    }
    let mut fams = [0u8; 8];
    for (k, f) in families(pos).iter().enumerate() {
        fams[k] = family_read(&attrs, f).map_or(0, |(lo, hi)| ((lo + hi) / 2.0).round() as u8);
    }
    KeptRead {
        season: season as i16,
        pos: pos.index() as u8,
        ovr: pair(ovr),
        keys,
        fams,
        wt: afm_engine::body::body_now(&p.player).weight.round() as u16,
    }
}

/// The players your staff follows: your roster, everyone you keep notes
/// on and your scouting board.
fn followed(w: &World) -> Vec<usize> {
    let mut ids: Vec<u32> = Vec::new();
    if let Ok(t) = my_team(w) {
        ids.extend(views::roster_people(w, t).into_iter().map(|k| w.people[k].player.id));
    }
    if let Some(c) = &w.career {
        ids.extend(c.notes.iter().filter(|(_, l)| !l.is_empty()).map(|x| x.0));
    }
    ids.extend(w.scouting.board.order.iter().take(300).copied());
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter().filter_map(|id| views::person_index(w, id)).filter(|&k| w.people[k].stage != Stage::Done).collect()
}

/// Keeps the season's reads once the football year has turned: the read
/// each followed player ended the season on (his overall, his position's
/// key ratings and families, his weight). A career started mid-year, or a
/// save from before the kept reads, starts keeping at the next turn.
pub fn after_days(w: &mut World) {
    let Some(c) = w.career.as_ref() else { return };
    let season = (w.year - 1) as i16;
    if c.kept_through >= season {
        return;
    }
    if c.kept_through == 0 {
        if let Some(c) = w.career.as_mut() {
            c.kept_through = season;
        }
        return;
    }
    let rows: Vec<(u32, KeptRead)> =
        followed(w).into_iter().map(|k| (w.people[k].player.id, read_now(w, k, season as i32))).collect();
    if let Some(c) = w.career.as_mut() {
        for (id, row) in rows {
            c.keep_read(id, row);
        }
        c.kept_through = season;
    }
}

/// A season's kept read for a player, if your staff kept one.
pub(crate) fn kept(w: &World, id: u32, season: i32) -> Option<KeptRead> {
    w.career.as_ref()?.kept_of(id).iter().find(|r| r.season as i32 == season).copied()
}

/// A kept range as JSON (null when nobody saw it).
pub(crate) fn pair_json(p: [u8; 2]) -> Value {
    if p == [0, 0] { Value::Null } else { json!(p) }
}

/// A quantile of a list (sorted in place; 0 when empty).
pub(crate) fn quantile(v: &mut [f64], q: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let x = q.clamp(0.0, 1.0) * (v.len() - 1) as f64;
    let (lo, hi) = (x.floor() as usize, x.ceil() as usize);
    v[lo] + (v[hi] - v[lo]) * (x - lo as f64)
}

/// How much players grow in a year at each age, at each level, from the
/// world's own records (everyone still in football): the 10th, 25th,
/// 50th, 75th and 90th percentile of a year's change in overall. Kept for
/// the season.
pub(crate) fn growth_table(w: &World) -> std::sync::Arc<HashMap<(u32, u8), [f64; 5]>> {
    type G = std::sync::Arc<HashMap<(u32, u8), [f64; 5]>>;
    static CACHE: Mutex<Option<((u32, i32), G)>> = Mutex::new(None);
    let key = (w.options.seed, w.year);
    let mut g = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((k, v)) = g.as_ref()
        && *k == key
    {
        return v.clone();
    }
    let mut by: HashMap<(u32, u8), Vec<f64>> = HashMap::new();
    for p in w.people.iter().step_by(3) {
        for h in &p.player.history {
            if h.age >= 14 && h.age <= 38 {
                by.entry((h.age, tier_of(h.level))).or_default().push(h.overall - h.overall_start);
            }
        }
    }
    let table: HashMap<(u32, u8), [f64; 5]> = by
        .into_iter()
        .filter(|(_, v)| v.len() >= 20)
        .map(|(k, mut v)| {
            let q = [0.1, 0.25, 0.5, 0.75, 0.9].map(|x| quantile(&mut v, x));
            (k, q)
        })
        .collect();
    let table = std::sync::Arc::new(table);
    *g = Some((key, table.clone()));
    table
}

/// 0 high school, 1 college, 2 the NFL.
pub(crate) fn tier_of(l: afm_engine::player::Level) -> u8 {
    use afm_engine::player::Level;
    match l {
        Level::HS => 0,
        Level::NFL => 2,
        _ => 1,
    }
}

/// A year's growth quantiles at an age and level (the nearest age kept).
pub(crate) fn growth_at(table: &HashMap<(u32, u8), [f64; 5]>, age: u32, tier: u8) -> [f64; 5] {
    for d in 0..6 {
        for a in [age + d, age.saturating_sub(d)] {
            if let Some(q) = table.get(&(a, tier)) {
                return *q;
            }
        }
    }
    [0.0; 5]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_run_from_f_to_a_plus() {
        assert_eq!(letter(0.0), "F");
        assert_eq!(letter(0.5), "B-");
        assert_eq!(letter(0.97), "A+");
    }

    #[test]
    fn every_position_has_eight_families() {
        for &pos in Position::ALL.iter() {
            let f = families(pos);
            assert_eq!(f.len(), 8);
            assert!(f.iter().all(|x| !x.1.is_empty()));
        }
    }

    #[test]
    fn quantiles_read_a_sorted_list() {
        let mut v = vec![5.0, 1.0, 3.0, 2.0, 4.0];
        assert_eq!(quantile(&mut v, 0.5), 3.0);
        assert_eq!(quantile(&mut v, 0.0), 1.0);
        assert_eq!(quantile(&mut v, 1.0), 5.0);
    }
}
