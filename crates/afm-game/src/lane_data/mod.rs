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
//! - `your_numbers` and `next_opponent` (`kind`, `id`): your rank in what
//!   decides games at your level with five seasons, the verdict, the
//!   drivers, this week's movers and alerts on your players, and the next
//!   opponent's numbers, tendencies, best players and the matchups that
//!   tilt (lane NB11, items 41 and 33; `yours.rs`);
//! - `prospects` (`level`, `class`, `pos`, `state`) and `prospect_card`
//!   (`id`): the draft's prospects and a high school class, a college
//!   prospect's production card and a high schooler's game log, lines,
//!   testing, offers and stars over time (items 34 and 35; `prospects.rs`);
//! - `draft_data`, `recruit_data` and `portal_data` (`tab`, `level`,
//!   `division` and each page's own): draft outcomes and hit rates, class
//!   rankings and who develops, and the transfer portal by program
//!   (items 36-38; `drafts.rs`);
//! - `coach_data` (`level`, `role`, `window`): coordinators and head
//!   coaches ranked by their units (item 39; `coaches.rs`);
//! - `money_data` (`kind`, `id`, `year`): the cap, NIL and the revenue
//!   share, or a high school's budget, against results (item 40;
//!   `money.rs`).
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
mod coaches;
mod compare;
mod deep;
mod drafts;
mod finder;
pub(crate) mod glossary;
mod money;
mod prospects;
pub(crate) mod situ;
mod teams;
pub(crate) mod tgame;
mod trends;
mod yours;

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
        "draft_data" => drafts::draft_data,
        "recruit_data" => drafts::recruit_data,
        "portal_data" => drafts::portal_data,
        "your_numbers" => yours::your_numbers,
        "next_opponent" => yours::next_opponent,
        "prospects" => prospects::prospects,
        "prospect_card" => prospects::prospect_card,
        "coach_data" => coaches::coach_data,
        "money_data" => money::money_data,
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

/// Whether a level's charted numbers are published (Cain, 2026-11-13: all
/// available data is charted by a third party, so your staff has nothing to
/// do with it). The level sets the card, the same for every seat: the NFL,
/// FBS and FCS are charted snap by snap; D2, D3, NAIA, JUCO and high school
/// are box scores and rankings, the most a third party publishes. Your own
/// team's film is separate (`world/scouting/film`) and still follows your
/// analytics head. Item 2's cap on the lower levels is proposed, not
/// confirmed.
pub(crate) fn charted(_w: &World, tier: Tier, division: Option<Division>) -> bool {
    match tier {
        Tier::Nfl => true,
        Tier::Hs => false,
        Tier::College => matches!(division, Some(Division::FBS | Division::FCS)),
    }
}

/// How a charted level reaches you: how deep it reads and how many days
/// after a game it arrives. A third party publishes it, so it is the same
/// for every seat: full depth the morning after. Your analytics head's
/// rating doesn't touch it. None where the level is box scores.
pub(crate) struct Charting {
    pub depth: u8,
    pub days: u8,
    /// Whose it is: always None now (a third party's), kept for the pages
    /// that print a staffer's name.
    pub head: Option<String>,
}

pub(crate) fn charting(w: &World, tier: Tier, division: Option<Division>) -> Option<Charting> {
    if charted(w, tier, division) {
        return Some(Charting { depth: 3, days: 1, head: None });
    }
    // Your own film: a college seat below FCS charts its own division's
    // games itself, as deep and as soon as your analytics head allows.
    if let (Tier::College, Some(d), Some(TeamRef::College(p))) = (tier, division, views::mine(w)) {
        let staff = &w.programs.get(p)?.support.analytics;
        if w.programs[p].division == d {
            let (depth, days) = afm_engine::world::analytics::charting(staff.rating);
            return Some(Charting { depth, days, head: Some(staff.name.clone()).filter(|n| !n.is_empty()) });
        }
    }
    None
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
    let seat = match views::mine(w) {
        Some(TeamRef::Nfl(_)) | Some(TeamRef::Other(_)) => "nfl",
        Some(TeamRef::College(_)) => "college",
        Some(TeamRef::HighSchool(_)) => "hs",
        None => "none",
    };
    let mut line = "A third party charts every NFL, FBS and FCS snap and publishes it the morning after, the same for every club. D2, D3, NAIA, junior college and high school are box scores and rankings."
        .to_string();
    let charted: Vec<&str> = Division::ALL
        .iter()
        .filter(|&&d| charted(w, Tier::College, Some(d)))
        .map(|d| d.name())
        .chain(std::iter::once("NFL"))
        .collect();
    let own = match views::mine(w) {
        Some(TeamRef::College(p)) if !charted.contains(&w.programs[p].division.name()) => {
            let d = w.programs[p].division;
            charting(w, Tier::College, Some(d)).map(|c| (d, c))
        }
        _ => None,
    };
    if let Some((d, c)) = &own {
        line.push_str(&format!(
            " Your own staff charts {} itself, {}, as soon as your analytics head allows.",
            d.name(),
            afm_engine::world::analytics::depth_name(c.depth)
        ));
    }
    json!({
        "seat": seat,
        "line": line,
        "charted": charted,
        "charting": own.as_ref().map(|(_, c)| charting_json(c)),
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
