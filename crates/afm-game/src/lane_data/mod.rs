//! Lane commands: the Data section (update 3, lane L20; boards 43 and 46 in
//! Press Box). `call` answers the section's commands and returns None for
//! any other:
//! - `data_sources`: what your seat sees (item 3): league-wide tracking in
//!   the NFL, your staff's charting of every game in your college division,
//!   box scores in high school;
//! - `stat_history` (`level`, `division`, `conference`, `state`, `tab`,
//!   `stat`, `team`): careers, single seasons, single games, rate records
//!   and each year's leader at every level, with the era column, the real
//!   NFL record on top of the NFL's lists, who is chasing a record now, the
//!   records that fell this season and the team's own book beside (items
//!   15-22, `books.rs`);
//! - `team_book` (`kind`, `id`): a team's own career, season and game marks
//!   (item 17);
//! - `analytics` (`level`, `year`, `division`, `group`, `min`, `sort`) and
//!   `analytics_card` (`id`, `level`, `year`): the percentile table and a
//!   player's card with the past seasons closest to his (items 7 and 8,
//!   `analytics.rs`);
//! - `champions_more` (`tab` "conferences", "states" or "award"): conference
//!   champions by year, state champions by state and class, and every
//!   winner of one award (items 23-25, `champs.rs`).
//!
//! Everything is read from the stat book, the record store
//! (`world::marks`), the players' seasons and the teams' seasons: the
//! Data pages show stats and your staff's charting only, never a hidden
//! rating.

mod analytics;
mod books;
pub(crate) use books::title_case;
mod champs;

use afm_engine::calendar::Level as CalLevel;
use afm_engine::world::World;
use afm_engine::world::data::Division;
use afm_engine::world::marks::Tier;
use serde_json::{Value, json};

use crate::views::{self, TeamRef};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let f: fn(&World, &Value) -> Value = match cmd {
        "data_sources" => |w, _| sources(w),
        "stat_history" => books::stat_history,
        "team_book" => books::team_book,
        "analytics" => analytics::table,
        "analytics_card" => analytics::card,
        "champions_more" => champs::more,
        _ => return None,
    };
    let Some(w) = game.world.as_ref() else { return Some(Err("No world is loaded.".into())) };
    Some(Ok(f(w, args)))
}

/// Your seat's level, as the record store's tiers (a club in another
/// league reads the NFL's).
pub(crate) fn seat_tier(w: &World) -> Tier {
    match w.seat.map(|s| s.level) {
        Some(CalLevel::Nfl) | Some(CalLevel::Other) => Tier::Nfl,
        Some(CalLevel::HighSchool) => Tier::Hs,
        _ => Tier::College,
    }
}

/// The level a page asks for, else your seat's.
pub(crate) fn tier_arg(w: &World, args: &Value) -> Tier {
    match args.get("level").and_then(Value::as_str) {
        Some("nfl") => Tier::Nfl,
        Some("hs") => Tier::Hs,
        Some("college") => Tier::College,
        _ => seat_tier(w),
    }
}

pub(crate) fn tier_key(t: Tier) -> &'static str {
    match t {
        Tier::College => "college",
        Tier::Nfl => "nfl",
        Tier::Hs => "hs",
    }
}

/// The division a page asks for, else yours, else FBS.
pub(crate) fn division_arg(w: &World, args: &Value) -> Division {
    let mine = match views::mine(w) {
        Some(TeamRef::College(p)) => w.programs[p].division,
        _ => Division::FBS,
    };
    args.get("division").and_then(Value::as_str).and_then(Division::parse).unwrap_or(mine)
}

/// The state a page asks for (its code or index), else your school's,
/// else the first.
pub(crate) fn state_arg(w: &World, args: &Value) -> usize {
    let asked = match args.get("state") {
        Some(Value::String(c)) => w.states.iter().position(|s| s.code.eq_ignore_ascii_case(c)),
        Some(Value::Number(n)) => n.as_u64().map(|n| n as usize).filter(|&n| n < w.states.len()),
        _ => None,
    };
    asked.unwrap_or_else(|| match views::mine(w) {
        Some(TeamRef::HighSchool(h)) => w.high_schools[h].state,
        _ => 0,
    })
}

/// A team at a level, for links.
pub(crate) fn team_ref(w: &World, tier: Tier, team: usize) -> Option<TeamRef> {
    match tier {
        Tier::College => (team < w.programs.len()).then_some(TeamRef::College(team)),
        Tier::Nfl => (team < w.nfl.len()).then_some(TeamRef::Nfl(team)),
        Tier::Hs => (team < w.high_schools.len()).then_some(TeamRef::HighSchool(team)),
    }
}

pub(crate) fn team_json(w: &World, tier: Tier, team: usize) -> Value {
    team_ref(w, tier, team).map_or(Value::Null, |t| views::team_brief(w, t))
}

/// Whether your staff sees a level's charted numbers (item 3): the NFL's
/// tracking is league-wide and published, so every seat has it; a college
/// staff charts every game in its own division; an NFL club's analytics
/// staff buys the charting of FBS and FCS for the draft; high school is box
/// scores (your own film is the only charting, and the store keeps none of
/// it). Proposed, not confirmed: the NFL seat's college charting and every
/// seat's NFL tracking.
pub(crate) fn charted(w: &World, tier: Tier, division: Option<Division>) -> bool {
    match tier {
        Tier::Nfl => true,
        Tier::Hs => false,
        Tier::College => match (views::mine(w), division) {
            (Some(TeamRef::College(p)), Some(d)) => w.programs[p].division == d,
            (Some(TeamRef::Nfl(_)) | Some(TeamRef::Other(_)), Some(d)) => matches!(d, Division::FBS | Division::FCS),
            _ => false,
        },
    }
}

/// What your seat's numbers come from, in a line for every Data page.
pub(crate) fn sources(w: &World) -> Value {
    let (seat, line) = match views::mine(w) {
        Some(TeamRef::Nfl(_)) | Some(TeamRef::Other(_)) => (
            "nfl",
            "League-wide tracking charts every NFL snap. Your analytics staff buys the charting of every FBS and FCS game; the other divisions and high school are box scores.".to_string(),
        ),
        Some(TeamRef::College(p)) => (
            "college",
            format!(
                "Your staff charts every game in {}. The NFL's tracking is published; the other divisions and high school are box scores.",
                w.programs[p].division.name()
            ),
        ),
        Some(TeamRef::HighSchool(_)) => (
            "hs",
            "Box scores, with your own film and what you trade for. The NFL's tracking is published; college is box scores.".to_string(),
        ),
        None => ("none", "Box scores, and the NFL's published tracking.".to_string()),
    };
    let charted: Vec<&str> = Division::ALL
        .iter()
        .filter(|&&d| charted(w, Tier::College, Some(d)))
        .map(|d| d.name())
        .chain(std::iter::once("NFL"))
        .collect();
    json!({ "seat": seat, "line": line, "charted": charted })
}

/// A number as the books write it: whole, or with one decimal (half sacks,
/// rates).
pub(crate) fn fmt_num(v: f64) -> String {
    if (v - v.round()).abs() < 0.05 { format_thousands(v.round() as i64) } else { format!("{v:.1}") }
}

fn format_thousands(n: i64) -> String {
    let s = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}
