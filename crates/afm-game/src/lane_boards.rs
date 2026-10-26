//! Lane commands: the game boards (update 2, lane 9). `call` answers the
//! commands this lane owns and returns None for any other.
//!
//! Six Press Box boards against the game's own data at every seat:
//!
//!   board_flow     V10, a game's flow: the win chance snap by snap, the
//!                  drives on the field, the swing plays, the final.
//!   board_passmap  V11, a passer's throws: where each went, to whom, how
//!                  deep, off what pocket clock, against his level.
//!   board_battles  V12, a game's line battles: every rep up front, the
//!                  protection and the rush, each matchup.
//!   board_cap      V14, where the money goes: the NFL cap by player and
//!                  year; in college the NIL budget and the scholarships.
//!   board_locker   V15, the locker room: who's close to whom, the moods,
//!                  the leaders, the friction.
//!   board_polls    V16, the race: college polls week by week, the NFL's
//!                  playoff picture, your state's high school rankings.
//!
//! Ratings are always your staff's ranges; potential is never shown.

mod battles;
mod cap;
pub mod flow;
mod locker;
mod passmap;
mod polls;

use afm_engine::calendar::Level as CalLevel;
use afm_engine::world::World;
use afm_engine::world::seat::SeatGame;
use serde_json::Value;

use crate::views::TeamRef;
use crate::{Game, Reply};

const COMMANDS: [&str; 6] =
    ["board_flow", "board_passmap", "board_battles", "board_cap", "board_locker", "board_polls"];

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    if !COMMANDS.contains(&cmd) {
        return None;
    }
    let Some(w) = game.world.as_ref() else { return Some(Err("No world is loaded.".into())) };
    let index = || args.get("index").and_then(Value::as_u64).map(|v| v as usize).ok_or("Missing index.".to_string());
    Some(match cmd {
        "board_flow" => index().and_then(|k| flow::board(w, k)),
        "board_passmap" => match args.get("id").and_then(Value::as_u64) {
            Some(id) => passmap::board(w, id as u32),
            None => Err("Missing id.".into()),
        },
        "board_battles" => index().and_then(|k| battles::board(w, k)),
        "board_cap" => cap::board(w, args),
        "board_locker" => locker::board(w),
        "board_polls" => polls::board(w, args),
        _ => return None,
    })
}

/// Both teams' names for a game, with anyone in its plays who has left
/// either team since (a senior gone after the bowl) found among the
/// world's people, and anyone who has left football since kept with the
/// game (`seat_names`).
pub(crate) fn game_names(w: &World, g: &SeatGame, refs: &[TeamRef]) -> crate::lane_gameday::film::Names {
    let mut names = crate::lane_gameday::names(w, refs);
    for r in &g.stats.plays {
        let ids = [
            r.passer,
            r.target,
            r.carrier,
            r.defender,
            r.pbu,
            r.interceptor,
            r.forced_fumble,
            r.fumbled,
            r.kicker,
            r.returner,
        ];
        let blocks = r.blocks.iter().flat_map(|b| [b.0, b.1]);
        let line = r.line.iter().flat_map(|c| c.reps.iter().flat_map(|x| [x.blocker, x.defender]));
        for id in ids
            .into_iter()
            .flatten()
            .chain(r.tacklers.iter().copied())
            .chain(r.sackers.iter().copied())
            .chain(blocks)
            .chain(line)
        {
            if !names.contains_key(&id)
                && let Some(i) = crate::views::person_index(w, id)
            {
                let p = &w.people[i];
                names.insert(id, (p.name.clone(), p.player.position.name().to_string()));
            }
        }
    }
    // Those who have left football since, kept with the game.
    for x in &g.gone {
        names.entry(x.id).or_insert_with(|| (x.name.clone(), x.pos.name().to_string()));
    }
    names
}

/// A seat game's side as a screens team.
pub(crate) fn team_ref(level: CalLevel, t: usize) -> TeamRef {
    match level {
        CalLevel::Nfl => TeamRef::Nfl(t),
        CalLevel::HighSchool => TeamRef::HighSchool(t),
        CalLevel::Other => TeamRef::Other(t),
        _ => TeamRef::College(t),
    }
}

/// "3:05" from seconds.
pub(crate) fn clock(secs: f64) -> String {
    // College and high school overtime has no clock.
    if secs >= 1.0e6 {
        return String::new();
    }
    let s = secs.max(0.0).round() as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

/// "Q3", "OT".
pub(crate) fn quarter(q: u8) -> String {
    if q > 4 { if q == 5 { "OT".into() } else { format!("{}OT", q - 4) } } else { format!("Q{q}") }
}

/// The engine level of a seat game's teams, for the win chance and the
/// level's numbers.
pub(crate) fn game_level(w: &World, level: CalLevel, teams: [usize; 2]) -> afm_engine::player::Level {
    use afm_engine::player::Level as L;
    match level {
        CalLevel::Nfl | CalLevel::Other => L::NFL,
        CalLevel::HighSchool => L::HS,
        _ => {
            let d = teams.map(|t| w.programs.get(t).map(|p| p.division)).into_iter().flatten().min();
            d.map_or(L::FBS, afm_engine::world::division_level)
        }
    }
}

/// The key of a level's numbers (`world::charts`) and its name ("FBS",
/// "NFL", "Texas").
pub(crate) fn level_key(w: &World, level: CalLevel, team: usize) -> (u16, String) {
    use afm_engine::world::charts;
    match level {
        CalLevel::Nfl | CalLevel::Other => (charts::NFL, "NFL".into()),
        CalLevel::HighSchool => {
            let s = w.high_schools.get(team).map_or(0, |h| h.state);
            (
                charts::HS + s as u16,
                w.states.get(s).map_or("State", |x| afm_engine::world::folk::facts::state_name(&x.code)).to_string(),
            )
        }
        _ => {
            let d = w.programs.get(team).map(|p| p.division).unwrap_or(afm_engine::world::data::Division::FBS);
            let name = match charts::college_key(d) {
                charts::FBS => "FBS",
                charts::FCS => "FCS",
                _ => "Small college",
            };
            (charts::college_key(d), name.into())
        }
    }
}

/// The season a level's numbers are filed under today. The world's year
/// turns the day after the Super Bowl, so it is the NFL's season until
/// then.
pub(crate) fn season_now(w: &World, level: CalLevel) -> i32 {
    match level {
        CalLevel::College => w.college_year().unwrap_or(w.year),
        _ => w.year,
    }
}

/// The season a game counts in (the NFL's runs into February, college
/// bowls into January).
pub(crate) fn season_of(level: CalLevel, d: afm_engine::calendar::Date) -> i32 {
    match level {
        CalLevel::Nfl if d.month() <= 2 => d.year() - 1,
        CalLevel::College if d.month() == 1 => d.year() - 1,
        _ => d.year(),
    }
}

/// Rounds to one decimal for the screens.
pub(crate) fn r1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}
