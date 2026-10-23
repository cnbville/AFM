//! The big tables (S8): season leaders at every level and the player
//! finder, sorted, filtered and paged by the engine so a table of fifty
//! thousand rows scrolls like one of fifty. The page asks for a window of
//! rows; sorting, the second sort, filters, per-game and per-snap views and
//! the CSV file are all done here.

use std::path::Path;

use afm_engine::positions::Position;
use afm_engine::world::hs_year::HsStat;
use afm_engine::world::records::{STAT_COUNT, Stat};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use super::desk;
use crate::views::{self, TeamRef};

/// A column: key, short label, what it is, and how it is shown
/// (0 whole number, 1 one decimal, 2 percent).
pub struct Col {
    pub key: &'static str,
    pub label: &'static str,
    pub long: &'static str,
    pub fmt: u8,
}

const fn c(key: &'static str, label: &'static str, long: &'static str, fmt: u8) -> Col {
    Col { key, label, long, fmt }
}

/// Every stat column the tables know, in the Columns menu's order.
pub const COLS: &[Col] = &[
    c("Games", "G", "Games", 0),
    c("Cmp", "Cmp", "Completions", 0),
    c("Att", "Att", "Pass attempts", 0),
    c("Pct", "Pct", "Completion percentage", 2),
    c("PassYds", "Yds", "Passing yards", 0),
    c("Ypa", "Y/A", "Yards per attempt", 1),
    c("PassTd", "TD", "Touchdown passes", 0),
    c("Int", "Int", "Interceptions thrown", 0),
    c("Sacked", "Sk", "Times sacked", 0),
    c("AirYds", "Air", "Air yards", 0),
    c("BigTime", "BTT", "Big-time throws", 0),
    c("TurnoverWorthy", "TWP", "Turnover-worthy plays", 0),
    c("Rush", "Car", "Carries", 0),
    c("RushYds", "Yds", "Rushing yards", 0),
    c("Ypc", "Y/C", "Yards per carry", 1),
    c("RushTd", "TD", "Rushing touchdowns", 0),
    c("RushLong", "Lng", "Longest run", 0),
    c("Yaco", "YAC", "Yards after contact", 0),
    c("ForcedMissed", "MTF", "Missed tackles forced", 0),
    c("ExplosiveRuns", "Exp", "Runs of 15+ yards", 0),
    c("FumblesLost", "FL", "Fumbles lost", 0),
    c("Targets", "Tgt", "Targets", 0),
    c("Rec", "Rec", "Receptions", 0),
    c("RecYds", "Yds", "Receiving yards", 0),
    c("Ypr", "Y/R", "Yards per catch", 1),
    c("RecTd", "TD", "Touchdown catches", 0),
    c("RecLong", "Lng", "Longest catch", 0),
    c("Drops", "Drp", "Drops", 0),
    c("Yac", "YAC", "Yards after catch", 0),
    c("PbSnaps", "PB", "Pass-block snaps", 0),
    c("PressuresAllowed", "PrA", "Pressures allowed", 0),
    c("SacksAllowed", "SkA", "Sacks allowed", 0),
    c("Tackles", "Tkl", "Tackles", 0),
    c("Tfl", "TFL", "Tackles for loss", 1),
    c("Sacks", "Sk", "Sacks", 1),
    c("Pressures", "Prs", "Pressures", 0),
    c("Stops", "Stp", "Stops", 0),
    c("Missed", "Mis", "Missed tackles", 0),
    c("ForcedFumbles", "FF", "Forced fumbles", 0),
    c("Ints", "Int", "Interceptions", 0),
    c("Pbus", "PBU", "Passes broken up", 0),
    c("TgtAllowed", "TgtA", "Targets allowed", 0),
    c("YdsAllowed", "YdsA", "Yards allowed in coverage", 0),
    c("FgMade", "FGM", "Field goals made", 0),
    c("FgAtt", "FGA", "Field goal attempts", 0),
    c("FgPct", "FG%", "Field goal percentage", 2),
    c("Fg50Made", "50+", "Field goals of 50+", 0),
    c("FgLong", "Lng", "Longest field goal", 0),
    c("XpMade", "XP", "Extra points", 0),
    c("Punts", "Pnt", "Punts", 0),
    c("PuntAvg", "Net", "Net yards a punt", 1),
    c("Returns", "Ret", "Returns", 0),
    c("ReturnYds", "RetYds", "Return yards", 0),
    c("ReturnTd", "RetTD", "Return touchdowns", 0),
    c("Snaps", "Snaps", "Snaps", 0),
];

/// The column sets the "View" picker offers.
pub const VIEWS: &[(&str, &[&str])] = &[
    ("Passing", &["Games", "Cmp", "Att", "Pct", "PassYds", "Ypa", "PassTd", "Int", "Sacked"]),
    ("Rushing", &["Games", "Rush", "RushYds", "Ypc", "RushTd", "RushLong", "Yaco", "ForcedMissed", "FumblesLost"]),
    ("Receiving", &["Games", "Targets", "Rec", "RecYds", "Ypr", "RecTd", "RecLong", "Drops", "Yac"]),
    ("Defense", &["Games", "Tackles", "Tfl", "Sacks", "Pressures", "Ints", "Pbus", "ForcedFumbles"]),
    ("Blocking", &["Games", "PbSnaps", "PressuresAllowed", "SacksAllowed", "Snaps"]),
    ("Kicking", &["Games", "FgMade", "FgAtt", "FgPct", "Fg50Made", "FgLong", "XpMade"]),
    ("Punting", &["Games", "Punts", "PuntAvg"]),
    ("Returns", &["Games", "Returns", "ReturnYds", "ReturnTd"]),
    ("Advanced passing", &["Games", "Att", "AirYds", "BigTime", "TurnoverWorthy", "Sacked"]),
];

fn stat_of(key: &str) -> Option<Stat> {
    Stat::ALL.iter().copied().find(|s| s.key() == key)
}

/// Counting stats divide in the per-game and per-snap views; rates don't.
fn is_rate(key: &str) -> bool {
    matches!(key, "Pct" | "Ypa" | "Ypc" | "Ypr" | "FgPct" | "PuntAvg" | "RushLong" | "RecLong" | "FgLong" | "Games")
}

/// One season's line in a table.
struct Line {
    id: u32,
    name: String,
    pos: Position,
    team: Option<TeamRef>,
    games: u8,
    s: [f32; STAT_COUNT],
}

fn value(l: &Line, key: &str) -> f64 {
    let g = |s: Stat| l.s[s as usize] as f64;
    let div = |a: f64, b: f64| if b > 0.0 { a / b } else { 0.0 };
    match key {
        "Games" => l.games as f64,
        "Pct" => 100.0 * div(g(Stat::Cmp), g(Stat::Att)),
        "Ypa" => div(g(Stat::PassYds), g(Stat::Att)),
        "Ypc" => div(g(Stat::RushYds), g(Stat::Rush)),
        "Ypr" => div(g(Stat::RecYds), g(Stat::Rec)),
        "FgPct" => 100.0 * div(g(Stat::FgMade), g(Stat::FgAtt)),
        "PuntAvg" => div(g(Stat::PuntNet), g(Stat::Punts)),
        "Snaps" => g(Stat::SnapsOff) + g(Stat::SnapsDef) + g(Stat::SnapsSt),
        k => stat_of(k).map_or(0.0, g),
    }
}

/// A rate needs enough tries to lead (the real books' minimums, per game).
fn qualifies(l: &Line, key: &str, nfl: bool) -> bool {
    let g = |s: Stat| l.s[s as usize] as f64;
    let games = l.games.max(1) as f64;
    match key {
        "Pct" | "Ypa" => g(Stat::Att) >= if nfl { 14.0 } else { 15.0 } * games,
        "Ypc" => g(Stat::Rush) >= 6.25 * games,
        "Ypr" => g(Stat::Rec) >= 1.875 * games,
        "FgPct" => g(Stat::FgAtt) >= games,
        "PuntAvg" => g(Stat::Punts) >= 3.6 * games,
        _ => true,
    }
}

fn shown(l: &Line, key: &str, per: &str) -> f64 {
    let v = value(l, key);
    if is_rate(key) {
        return v;
    }
    match per {
        "game" => v / l.games.max(1) as f64,
        "snap" => {
            let snaps = value(l, "Snaps");
            if key == "Snaps" {
                v
            } else if snaps > 0.0 {
                100.0 * v / snaps
            } else {
                0.0
            }
        }
        _ => v,
    }
}

fn hs_stat(k: HsStat) -> Stat {
    match k {
        HsStat::SnapsOff => Stat::SnapsOff,
        HsStat::SnapsDef => Stat::SnapsDef,
        HsStat::Att => Stat::Att,
        HsStat::Cmp => Stat::Cmp,
        HsStat::PassYds => Stat::PassYds,
        HsStat::PassTd => Stat::PassTd,
        HsStat::Int => Stat::Int,
        HsStat::Rush => Stat::Rush,
        HsStat::RushYds => Stat::RushYds,
        HsStat::RushTd => Stat::RushTd,
        HsStat::Rec => Stat::Rec,
        HsStat::RecYds => Stat::RecYds,
        HsStat::RecTd => Stat::RecTd,
        HsStat::Tackles => Stat::Tackles,
        HsStat::Tfl => Stat::Tfl,
        HsStat::Sacks => Stat::Sacks,
        HsStat::Ints => Stat::Ints,
        HsStat::Pbus => Stat::Pbus,
        HsStat::FgMade => Stat::FgMade,
        HsStat::FgAtt => Stat::FgAtt,
        HsStat::XpMade => Stat::XpMade,
    }
}

const HS_STATS: [HsStat; 21] = [
    HsStat::SnapsOff,
    HsStat::SnapsDef,
    HsStat::Att,
    HsStat::Cmp,
    HsStat::PassYds,
    HsStat::PassTd,
    HsStat::Int,
    HsStat::Rush,
    HsStat::RushYds,
    HsStat::RushTd,
    HsStat::Rec,
    HsStat::RecYds,
    HsStat::RecTd,
    HsStat::Tackles,
    HsStat::Tfl,
    HsStat::Sacks,
    HsStat::Ints,
    HsStat::Pbus,
    HsStat::FgMade,
    HsStat::FgAtt,
    HsStat::XpMade,
];

/// The seasons a scope has: college divisions, the NFL, a state's high
/// schools. Returns the lines and whether they are the NFL's.
fn lines(w: &World, scope: &str, year: i32, state: Option<usize>) -> Vec<Line> {
    let name = |id: u32| views::person_index(w, id).map(|k| w.people[k].name.clone());
    match scope {
        "nfl" => w
            .people
            .iter()
            .filter_map(|p| {
                let c = p.pro.as_ref()?;
                let s = c.seasons.iter().find(|s| s.year == year && s.games > 0)?;
                Some(Line {
                    id: p.player.id,
                    name: p.name.clone(),
                    pos: p.player.position,
                    team: s.team.map(TeamRef::Nfl),
                    games: s.games.min(255) as u8,
                    s: afm_engine::world::records::from_line(&s.stats),
                })
            })
            .collect(),
        "hs" => w
            .people
            .iter()
            .flat_map(|p| {
                p.hs_lines.iter().filter(move |l| l.year as i32 == year).filter_map(move |l| {
                    let school = l.school as usize;
                    if state.is_some_and(|st| w.high_schools.get(school).is_none_or(|h| h.state != st)) {
                        return None;
                    }
                    let mut s = [0f32; STAT_COUNT];
                    for k in HS_STATS {
                        s[hs_stat(k) as usize] = l.get(k) as f32;
                    }
                    Some(Line {
                        id: p.player.id,
                        name: p.name.clone(),
                        pos: l.pos,
                        team: Some(TeamRef::HighSchool(school)),
                        games: l.games,
                        s,
                    })
                })
            })
            .collect(),
        _ => {
            use afm_engine::world::data::Division;
            let div = match scope {
                "college" => None,
                s => Some(
                    Division::ALL.iter().copied().find(|d| d.name().eq_ignore_ascii_case(s)).unwrap_or(Division::FBS),
                ),
            };
            w.stats
                .year(year)
                .iter()
                .filter(|r| div.is_none_or(|d| r.division == d))
                .map(|r| Line {
                    id: r.id,
                    name: name(r.id).unwrap_or_else(|| "A former player".into()),
                    pos: r.pos,
                    team: Some(TeamRef::College(r.program as usize)),
                    games: r.games,
                    s: r.s.to_array(),
                })
                .collect()
        }
    }
}

/// The latest season with lines in a scope.
fn last_year(w: &World, scope: &str) -> i32 {
    match scope {
        // The season under way once anyone has played in it.
        "nfl" => {
            w.year
                - i32::from(
                    !w.people.iter().any(|p| {
                        p.pro.as_ref().is_some_and(|c| c.seasons.iter().any(|s| s.year == w.year && s.games > 0))
                    }),
                )
        }
        "hs" => {
            w.year
                - i32::from(
                    !w.people.iter().any(|p| p.hs_lines.iter().any(|l| i32::from(l.year) == w.year && l.games > 0)),
                )
        }
        _ => w.stats.year_index.last().map_or(w.year - 1, |x| x.0),
    }
}

fn pos_group(pos: Position) -> &'static str {
    use Position as P;
    match pos {
        P::QB => "QB",
        P::RB => "RB",
        P::WR => "WR",
        P::TE => "TE",
        P::OT | P::IOL => "OL",
        P::EDGE | P::DT => "DL",
        P::LB => "LB",
        P::CB | P::S => "DB",
        P::K | P::P | P::LS => "ST",
    }
}

/// A filter from the filter builder: field, op and value.
fn passes(w: &World, l: &Line, f: &Value, per: &str) -> bool {
    let field = f.get("field").and_then(Value::as_str).unwrap_or("");
    let op = f.get("op").and_then(Value::as_str).unwrap_or(">=");
    let v = f.get("value");
    let text = v.and_then(Value::as_str).unwrap_or("");
    let num = v.and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())));
    let cmp = |x: f64| {
        let Some(n) = num else { return true };
        match op {
            ">=" => x >= n,
            "<=" => x <= n,
            ">" => x > n,
            "<" => x < n,
            _ => (x - n).abs() < 1e-6,
        }
    };
    match field {
        "pos" => text.is_empty() || l.pos.name() == text || pos_group(l.pos) == text,
        "team" => l.team.is_some_and(|t| views::team_name(w, t).to_lowercase().contains(&text.to_lowercase())),
        "conf" => matches!(l.team, Some(TeamRef::College(p)) if w.programs[p].conference == text),
        "listed" => desk::is_listed(w, l.id),
        "noted" => desk::has_note(w, l.id),
        "mine" => l.team.is_some_and(|t| views::is_mine(w, t)),
        "name" => l.name.to_lowercase().contains(&text.to_lowercase()),
        k => cmp(shown(l, k, per)),
    }
}

/// Season leaders (S8): `scope` fbs, fcs, d2, naia, d3, college, nfl or hs;
/// `year`; `sort` a column key and `dir`; `sort2`/`dir2` the second sort;
/// `per` total, game or snap; `filters`; `offset` and `limit` for the
/// window the page draws.
pub fn leaders(w: &World, args: &Value) -> Value {
    let (rows, total, cols, year, scope) =
        leader_rows(w, args, args.get("limit").and_then(Value::as_u64).unwrap_or(100) as usize);
    let years: Vec<i32> = match scope.as_str() {
        "nfl" | "hs" => (w.options.start_year - 3..=last_year(w, &scope)).rev().collect(),
        _ => w.stats.year_index.iter().map(|x| x.0).rev().collect(),
    };
    json!({
        "scope": scope,
        "year": year,
        "years": years,
        "total": total,
        "offset": args.get("offset").and_then(Value::as_u64).unwrap_or(0),
        "rows": rows,
        "columns": cols.iter().map(|k| col_json(k)).collect::<Vec<_>>(),
        "all_columns": COLS.iter().map(|c| json!({ "key": c.key, "label": c.label, "long": c.long, "fmt": c.fmt })).collect::<Vec<_>>(),
        "views": VIEWS.iter().map(|(n, cols)| json!({ "name": n, "cols": cols })).collect::<Vec<_>>(),
        "conferences": conferences(w),
    })
}

fn conferences(w: &World) -> Vec<String> {
    let mut v: Vec<String> = w
        .programs
        .iter()
        .filter(|p| p.division == afm_engine::world::data::Division::FBS)
        .map(|p| p.conference.clone())
        .collect();
    v.sort();
    v.dedup();
    v
}

fn col_json(k: &str) -> Value {
    COLS.iter().find(|c| c.key == k).map_or(
        json!({ "key": k, "label": k, "long": k, "fmt": 0 }),
        |c| json!({ "key": c.key, "label": c.label, "long": c.long, "fmt": c.fmt }),
    )
}

fn leader_rows(w: &World, args: &Value, limit: usize) -> (Vec<Value>, usize, Vec<String>, i32, String) {
    let default_scope = match views::mine(w) {
        Some(TeamRef::Nfl(_)) => "nfl".to_string(),
        Some(TeamRef::HighSchool(_)) => "hs".to_string(),
        Some(TeamRef::College(p)) => w.programs[p].division.name().to_lowercase(),
        None => "fbs".into(),
    };
    let scope = args.get("scope").and_then(Value::as_str).map(str::to_string).unwrap_or(default_scope);
    let year = args.get("year").and_then(Value::as_i64).map_or_else(|| last_year(w, &scope), |y| y as i32);
    let state = match (scope.as_str(), args.get("state").and_then(Value::as_str)) {
        ("hs", Some(code)) => w.states.iter().position(|s| s.code == code),
        ("hs", None) => match views::mine(w) {
            Some(TeamRef::HighSchool(h)) => Some(w.high_schools[h].state),
            _ => None,
        },
        _ => None,
    };
    let view = args.get("view").and_then(Value::as_str).unwrap_or("Passing");
    let cols: Vec<String> = match args.get("columns").and_then(Value::as_array) {
        Some(a) if !a.is_empty() => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        _ => VIEWS.iter().find(|v| v.0 == view).unwrap_or(&VIEWS[0]).1.iter().map(|s| s.to_string()).collect(),
    };
    let sort = args.get("sort").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| {
        cols.iter()
            .find(|k| !matches!(k.as_str(), "Games" | "Cmp" | "Att" | "Pct" | "Rush" | "Targets" | "Rec"))
            .cloned()
            .unwrap_or_else(|| cols[0].clone())
    });
    let dir = args.get("dir").and_then(Value::as_i64).unwrap_or(-1);
    let sort2 = args.get("sort2").and_then(Value::as_str).map(str::to_string);
    let dir2 = args.get("dir2").and_then(Value::as_i64).unwrap_or(-1);
    let per = args.get("per").and_then(Value::as_str).unwrap_or("total").to_string();
    let filters: Vec<Value> = args.get("filters").and_then(Value::as_array).cloned().unwrap_or_default();
    let nfl = scope == "nfl";
    let mut all: Vec<Line> = lines(w, &scope, year, state)
        .into_iter()
        .filter(|l| qualifies(l, &sort, nfl))
        .filter(|l| value(l, &sort) != 0.0 || is_rate(&sort))
        .filter(|l| filters.iter().all(|f| passes(w, l, f, &per)))
        .collect();
    let key = |l: &Line, k: &str| -> f64 {
        match k {
            "name" | "team" | "pos" => 0.0,
            _ => shown(l, k, &per),
        }
    };
    all.sort_by(|a, b| {
        let o = match sort.as_str() {
            "name" => a.name.cmp(&b.name),
            "pos" => a.pos.name().cmp(b.pos.name()),
            _ => key(a, &sort).total_cmp(&key(b, &sort)),
        };
        let o = if dir < 0 { o.reverse() } else { o };
        o.then_with(|| match &sort2 {
            Some(k) => {
                let o2 = key(a, k).total_cmp(&key(b, k));
                if dir2 < 0 { o2.reverse() } else { o2 }
            }
            None => std::cmp::Ordering::Equal,
        })
    });
    let total = all.len();
    let offset = (args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize).min(total);
    let rows: Vec<Value> = all
        .iter()
        .enumerate()
        .skip(offset)
        .take(limit)
        .map(|(rank, l)| {
            let cells: serde_json::Map<String, Value> =
                cols.iter().map(|k| (k.clone(), json!((shown(l, k, &per) * 10.0).round() / 10.0))).collect();
            json!({
                "rank": rank + 1,
                "id": l.id,
                "name": l.name,
                "pos": l.pos.name(),
                "team": l.team.map(|t| views::team_brief(w, t)),
                "cells": cells,
                "listed": desk::is_listed(w, l.id),
                "noted": desk::has_note(w, l.id),
                "mine": l.team.is_some_and(|t| views::is_mine(w, t)),
            })
        })
        .collect();
    (rows, total, cols, year, scope)
}

// ---- The player finder ----

struct Found {
    k: usize,
}

fn finder_value(w: &World, k: usize, field: &str) -> Value {
    let p = &w.people[k];
    match field {
        "name" => json!(p.name),
        "pos" => json!(p.player.position.name()),
        "age" => json!(p.player.age),
        "stars" => json!(p.stars),
        "class" => json!(p.class_of),
        "ht" => json!((p.player.body.height * 10.0).round() / 10.0),
        "wt" => json!(p.player.body.weight.round()),
        "state" => json!(w.states.get(p.home_state).map(|s| s.code.as_str()).unwrap_or("")),
        "level" => json!(level_word(p.stage)),
        _ => Value::Null,
    }
}

fn level_word(s: Stage) -> &'static str {
    match s {
        Stage::HighSchool | Stage::PrepSchool => "High school",
        Stage::College => "College",
        Stage::Declared => "Draft pool",
        Stage::Pro => "NFL",
        Stage::Done => "Out of football",
        Stage::Away => "Away from football",
    }
}

fn finder_passes(w: &World, k: usize, f: &Value) -> bool {
    let p = &w.people[k];
    let field = f.get("field").and_then(Value::as_str).unwrap_or("");
    let op = f.get("op").and_then(Value::as_str).unwrap_or("=");
    let v = f.get("value");
    let text = v.and_then(Value::as_str).unwrap_or("");
    let num = v.and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())));
    let cmp = |x: f64| {
        let Some(n) = num else { return true };
        match op {
            ">=" => x >= n,
            "<=" => x <= n,
            ">" => x > n,
            "<" => x < n,
            _ => (x - n).abs() < 1e-6,
        }
    };
    match field {
        "pos" => text.is_empty() || p.player.position.name() == text || pos_group(p.player.position) == text,
        "level" => text.is_empty() || level_word(p.stage) == text,
        "state" => text.is_empty() || w.states.get(p.home_state).is_some_and(|s| s.code == text),
        "name" => p.name.to_lowercase().contains(&text.to_lowercase()),
        "listed" => desk::is_listed(w, p.player.id),
        "noted" => desk::has_note(w, p.player.id),
        "mine" => views::team_of(w, p).is_some_and(|t| views::is_mine(w, t)),
        "team" => {
            views::team_of(w, p).is_some_and(|t| views::team_name(w, t).to_lowercase().contains(&text.to_lowercase()))
        }
        "town" => views::hometown(w, p).to_lowercase().starts_with(&text.to_lowercase()),
        "age" => cmp(p.player.age as f64),
        "stars" => cmp(p.stars as f64),
        "class" => cmp(p.class_of as f64),
        "ht" => cmp(p.player.body.height),
        "wt" => cmp(p.player.body.weight),
        _ => true,
    }
}

fn found(w: &World, q: &Value) -> Vec<Found> {
    let filters: Vec<Value> = q.get("filters").and_then(Value::as_array).cloned().unwrap_or_default();
    // The default finder looks at players still in football.
    w.people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.stage != Stage::Done)
        .filter(|(k, _)| filters.iter().all(|f| finder_passes(w, *k, f)))
        .map(|(k, _)| Found { k })
        .collect()
}

/// The ids a saved search finds (for watches), at most `cap`.
pub fn finder_ids(w: &World, q: &Value, cap: usize) -> Vec<u32> {
    found(w, q).into_iter().take(cap).map(|f| w.people[f.k].player.id).collect()
}

/// The player finder (S6, S8): every player in football by any filter,
/// with his public facts. Nothing hidden: stars, body, class, where.
pub fn finder(w: &World, args: &Value) -> Value {
    let mut rows = found(w, args);
    let sort = args.get("sort").and_then(Value::as_str).unwrap_or("stars");
    let dir = args.get("dir").and_then(Value::as_i64).unwrap_or(-1);
    let sort2 = args.get("sort2").and_then(Value::as_str);
    let dir2 = args.get("dir2").and_then(Value::as_i64).unwrap_or(-1);
    let cmpv = |a: &Value, b: &Value| match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => x.total_cmp(&y),
        _ => a.as_str().unwrap_or("").cmp(b.as_str().unwrap_or("")),
    };
    rows.sort_by(|a, b| {
        let o = cmpv(&finder_value(w, a.k, sort), &finder_value(w, b.k, sort));
        let o = if dir < 0 { o.reverse() } else { o };
        o.then_with(|| match sort2 {
            Some(s2) => {
                let o2 = cmpv(&finder_value(w, a.k, s2), &finder_value(w, b.k, s2));
                if dir2 < 0 { o2.reverse() } else { o2 }
            }
            None => std::cmp::Ordering::Equal,
        })
    });
    let total = rows.len();
    let offset = (args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize).min(total);
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(100) as usize;
    let out: Vec<Value> = rows[offset..(offset + limit).min(total)]
        .iter()
        .map(|f| {
            let p = &w.people[f.k];
            let team = views::team_of(w, p);
            json!({
                "id": p.player.id,
                "name": p.name,
                "pos": p.player.position.name(),
                "age": p.player.age,
                "stars": p.stars,
                "class": p.class_of,
                "level": level_word(p.stage),
                "ht": views::ft_in(p.player.body.height),
                "wt": p.player.body.weight.round(),
                "state": w.states.get(p.home_state).map(|s| s.code.clone()),
                "team": team.map(|t| views::team_brief(w, t)),
                "where": views::where_now(w, p),
                "listed": desk::is_listed(w, p.player.id),
                "noted": desk::has_note(w, p.player.id),
            })
        })
        .collect();
    json!({ "total": total, "offset": offset, "rows": out })
}

/// The table as a CSV file in the game folder's exports (S8).
pub fn csv(w: &World, home: &Path, args: &Value) -> Result<Value, String> {
    let kind = args.get("table").and_then(Value::as_str).unwrap_or("leaders");
    let mut out = String::new();
    let q = |s: &str| {
        if s.contains(',') || s.contains('"') { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() }
    };
    let name = if kind == "finder" {
        let mut a = args.clone();
        a["limit"] = json!(20000);
        let v = finder(w, &a);
        out.push_str("Name,Pos,Age,Stars,Class,Level,Height,Weight,State,Team\n");
        for r in v["rows"].as_array().into_iter().flatten() {
            let f = |k: &str| r[k].as_str().map(str::to_string).unwrap_or_else(|| r[k].to_string());
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{}\n",
                q(&f("name")),
                f("pos"),
                f("age"),
                f("stars"),
                f("class"),
                q(&f("level")),
                q(&f("ht")),
                f("wt"),
                f("state").replace("null", ""),
                q(r["team"]["name"].as_str().unwrap_or(""))
            ));
        }
        "players".to_string()
    } else {
        let (rows, _, cols, year, scope) = leader_rows(w, args, 20000);
        out.push_str("Rank,Name,Pos,Team");
        for k in &cols {
            out.push(',');
            out.push_str(COLS.iter().find(|c| c.key == k).map_or(k.as_str(), |c| c.long));
        }
        out.push('\n');
        for r in &rows {
            out.push_str(&format!(
                "{},{},{},{}",
                r["rank"],
                q(r["name"].as_str().unwrap_or("")),
                r["pos"].as_str().unwrap_or(""),
                q(r["team"]["name"].as_str().unwrap_or(""))
            ));
            for k in &cols {
                out.push(',');
                out.push_str(&r["cells"][k].to_string());
            }
            out.push('\n');
        }
        format!("{scope}-{year}-leaders")
    };
    export(home, &name, &out)
}

/// Writes a table the screen built as a CSV file in the game folder's
/// exports (any lane's table uses it through `export_text`).
pub fn export(home: &Path, name: &str, text: &str) -> Result<Value, String> {
    let safe: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(60)
        .collect();
    let dir = home.join("exports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.csv", if safe.is_empty() { "table" } else { safe.as_str() }));
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(json!({ "path": path.display().to_string() }))
}
