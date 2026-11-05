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
//! - `analytics` (`level`, `year`, `division`, `state`, `group`, `view`,
//!   `mode`, `min`, `sort`, `dir` and the filters) and `analytics_card`
//!   (`id`, `level`, `year`, `division`, `state`, `group`, `view`): the
//!   percentile table by position card view, raw, era index or
//!   opponent-adjusted, and a player's card with the past seasons closest
//!   to his (items 3-14 and 28, `analytics.rs`);
//! - `player_data` (`id`, `level`, `year`): a player's Data tab: facet
//!   grades, splits, his games, the throw map by zone, route map, run
//!   paths, pressure map and where he lined up (items 11, 15, 28, minor bug
//!   44; `deep.rs`);
//! - `champions_more` (`tab` "conferences", "states" or "award"): conference
//!   champions by year, state champions by state and class, and every
//!   winner of one award (items 23-25, `champs.rs`);
//! - `team_data` (`kind`, `id`, `year`), `team_seasons` (`kind`, `id`,
//!   `unit`) and `team_leaders` (`level`, `year`, `division`, `state`,
//!   `stat`, `side`, `view`): a team's Data tab, its stats by season and
//!   every team ranked, the tiers chart and the power ratings with the odds
//!   (lane NB7, items 16-23, `teams.rs`);
//! - `stat_glossary`: every stat in plain words, and the table headers
//!   that name each, for the hover on a stat's name (Cain, 2026-11-03;
//!   `glossary.rs`);
//! - `situations`, `decisions` and `concepts` (`level`, `division`,
//!   `year`, `team` and each page's own): situations, every 4th down and
//!   try against the analysts' model with each head coach's record, and
//!   concepts against coverages (lane NB8, items 25-27; `situ.rs`);
//! - `tally_game` (`year`, `key`): any game's page from the tallies, the
//!   box for anyone and the advanced box, win chance and decisions where
//!   your staff charts it (lane NB8, item 24; `tgame.rs`);
//! - `trends` (`tab` "league", "team" or "player", `level`, `division`,
//!   `state`, `stats`, `kind`, `id`, `all`): the league's years with the
//!   middle half of teams, a team's last seasons, a player against the
//!   usual curve at his spot (lane NB10, item 29; `trends.rs`, the player
//!   tab in `careers.rs`);
//! - `data_compare` (`kind` "players" or "teams", `ids` or `teams`, `line`,
//!   `at`, `year`) and `compare_find` (`q`, `kind`): two to four players or
//!   teams side by side (item 30, `compare.rs`);
//! - `career_comps` and `player_trend` (`id`, `level`, `division`,
//!   `seasons`, `mode`, `drafted`, `n`): the players whose seasons looked
//!   like his and what became of them (item 31, `careers.rs`);
//! - `stat_finder` (`mode` "seasons", "careers", "games" or "streaks",
//!   `level`, `division`, `state`, `from`, `to`, `pos`, `class`, `age`,
//!   `conds`, `cols`, `sort`) and `stat_finder_stats`: every season,
//!   career, game or streak that meets the conditions (item 32,
//!   `finder.rs`).
//!
//! Everything is read from the stat book, the record store
//! (`world::marks`), the players' seasons and the teams' seasons: the
//! Data pages show stats and your staff's charting only, never a hidden
//! rating.

mod analytics;
mod books;
pub(crate) use books::title_case;
mod careers;
mod champs;
mod compare;
mod deep;
mod finder;
pub(crate) mod glossary;
pub(crate) mod situ;
mod teams;
pub(crate) mod tgame;
mod trends;

use afm_engine::calendar::Level as CalLevel;
use afm_engine::world::World;
use afm_engine::world::data::Division;
use afm_engine::world::marks::Tier;
use serde_json::{Value, json};

use crate::views::{self, TeamRef};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    // The glossary needs no world: it answers on any screen.
    if cmd == "stat_glossary" {
        return Some(Ok(glossary::all()));
    }
    let f: fn(&World, &Value) -> Value = match cmd {
        "data_sources" => |w, _| sources(w),
        "stat_history" => books::stat_history,
        "team_book" => books::team_book,
        "analytics" => analytics::table,
        "analytics_card" => analytics::card,
        "player_data" => deep::player_data,
        "champions_more" => champs::more,
        "situations" => situ::situations,
        "decisions" => situ::decisions,
        "concepts" => situ::concepts,
        "tally_game" => tgame::tally_game,
        "team_data" => teams::team_data,
        "team_seasons" => teams::team_seasons,
        "team_leaders" => teams::team_leaders,
        "trends" => trends::trends,
        "player_trend" => careers::player_trend,
        "career_comps" => careers::career_comps,
        "data_compare" => compare::data_compare,
        "compare_find" => compare::compare_find,
        "stat_finder" => finder::stat_finder,
        "stat_finder_stats" => finder::finder_stats,
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

/// A program's division and conference in one season: the season's own,
/// else (an older file, or no season kept) today's. An empty conference is
/// an independent.
pub(crate) fn season_place(w: &World, prog: usize, year: i32) -> (Division, &str) {
    let p = &w.programs[prog];
    match p.seasons.iter().rev().find(|s| s.year == year).and_then(|s| Some((s.division?, s))) {
        Some((d, s)) => (d, s.conference.as_deref().unwrap_or("")),
        None => (p.division, p.conference.as_str()),
    }
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

/// How your seat's charting of a level goes (Data overhaul item 2): how deep
/// it reads (`analytics::depth`), how many days after a game it arrives, and
/// whose it is. The NFL's league tracking is the same for every seat; a
/// level your staff charts follows your analytics head (`analytics::charting`).
/// None where the level is box scores for you.
pub(crate) struct Charting {
    pub depth: u8,
    pub days: u8,
    /// Your analytics head, when it is your own staff's charting.
    pub head: Option<String>,
}

pub(crate) fn charting(w: &World, tier: Tier, division: Option<Division>) -> Option<Charting> {
    if !charted(w, tier, division) {
        return None;
    }
    if tier == Tier::Nfl {
        return Some(Charting { depth: 3, days: 1, head: None });
    }
    let staffer = match views::mine(w) {
        Some(TeamRef::College(p)) => w.programs.get(p).map(|x| &x.support.analytics),
        Some(TeamRef::Nfl(t)) => w.clubs.get(t).map(|c| &c.support.analytics),
        _ => None,
    };
    let (depth, days) = afm_engine::world::analytics::charting(staffer.map_or(0, |s| s.rating));
    Some(Charting { depth, days, head: staffer.map(|s| s.name.clone()).filter(|n| !n.is_empty()) })
}

pub(crate) fn charting_json(c: &Charting) -> Value {
    json!({
        "depth": c.depth,
        "kind": afm_engine::world::analytics::depth_name(c.depth),
        "days": c.days,
        "head": c.head,
    })
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
    // Your own staff's charting: how deep and how soon (item 2).
    let own = Division::ALL.iter().find_map(|&d| charting(w, Tier::College, Some(d)));
    let mut line = line;
    if let Some(c) = &own {
        let who = c.head.as_ref().map_or("Your analytics staff".to_string(), |n| format!("Your analytics head, {n},"));
        let when = if c.days == 1 { "the morning after".to_string() } else { format!("{} days after", c.days) };
        line.push_str(&format!(
            " {who} gives you {} {when} each game.",
            afm_engine::world::analytics::depth_name(c.depth)
        ));
    }
    json!({
        "seat": seat,
        "line": line,
        "charted": charted,
        "charting": own.as_ref().map(charting_json),
        "league": charting_json(&Charting { depth: 3, days: 1, head: None }),
    })
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
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}
