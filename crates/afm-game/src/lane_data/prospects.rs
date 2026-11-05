//! Prospects (Data overhaul items 34 and 35, board 56): what a draft
//! analyst and a recruiting analyst keep, from public numbers only.
//! - `prospects` (`level` "college" or "hs", `class`, `pos`, `state`): the
//!   draft's prospects by the public board with their production markers,
//!   or a high school class by the services' latest order;
//! - `prospect_card` (`id`): a college prospect's production card (his
//!   seasons with his share of the team's production, the dominator
//!   rating, breakout age and an age-adjusted dominator, his analytics
//!   markers with percentiles, his testing and size against the real
//!   combine at his spot, the seasons most like his with how those men
//!   turned out, and your staff's read as a range), or a high schooler's
//!   (his game log, his season lines with state and class percentiles,
//!   camp testing, offers and visits, and his stars over time).
//!
//! Never a hidden rating or a true potential: the staff's read is the
//! range the scouting screens show.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use afm_engine::calendar::Date;
use afm_engine::positions::{POSITION_COUNT, Position};
use afm_engine::world::hs_year::{HsLine, HsStat};
use afm_engine::world::records::{SeasonRecord, Stat};
use afm_engine::world::scouting::{Kind, board, file, readers};
use afm_engine::world::tally::prospects::{HsGame, Rank};
use afm_engine::world::{Person, Stage, Visit, World};
use serde_json::{Value, json};

use crate::views::{self, TeamRef};

// ---------------------------------------------------------------------
// The real combine at each spot.

const COMBINE_CSV: &str = include_str!("../../../../data/real-world/combine_by_position.csv");

/// Height, weight, forty, bench, vertical, broad, cone, shuttle: (mean, sd)
/// at each spot (2000-2025 combines).
type Spot = Option<[(f64, f64); 8]>;
static REAL: LazyLock<[Spot; POSITION_COUNT]> = LazyLock::new(|| {
    let mut out = [None; POSITION_COUNT];
    for line in COMBINE_CSV.lines().skip(1) {
        let c: Vec<&str> = line.split(',').collect();
        let Some(pos) = Position::ALL.iter().position(|p| p.name() == c[0]) else { continue };
        let num = |k: usize| c.get(k).and_then(|x| x.parse::<f64>().ok()).unwrap_or(0.0);
        let mut v = [(0.0, 1.0); 8];
        for (j, slot) in v.iter_mut().enumerate() {
            *slot = (num(2 + 2 * j), num(3 + 2 * j).max(1e-6));
        }
        out[pos] = Some(v);
    }
    out
});

/// The standard normal's cumulative chance (Abramowitz and Stegun 7.1.26).
fn phi(z: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.3275911 * z.abs() / std::f64::consts::SQRT_2);
    let y = 1.0
        - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t + 0.254829592)
            * t
            * (-(z * z) / 2.0).exp();
    if z >= 0.0 { 0.5 * (1.0 + y) } else { 0.5 * (1.0 - y) }
}

/// A test's percentile at his spot among real combine invitees (100 the
/// best; the times read the other way).
fn real_pct(pos: Position, k: usize, x: f64) -> Option<i64> {
    let (m, sd) = REAL[pos as usize]?[k];
    if m <= 0.0 {
        return None;
    }
    let z = (x - m) / sd;
    let low_better = matches!(k, 2 | 6 | 7);
    let p = if low_better { 1.0 - phi(z) } else { phi(z) };
    Some((100.0 * p).round().clamp(0.0, 100.0) as i64)
}

const TESTS: [(&str, &str); 6] = [
    ("forty", "40-yard dash"),
    ("bench", "Bench press"),
    ("vertical", "Vertical jump"),
    ("broad", "Broad jump"),
    ("cone", "3-cone drill"),
    ("shuttle", "20-yard shuttle"),
];

// ---------------------------------------------------------------------
// Production shares.

/// The team numbers a share is read against.
const SHARE: [Stat; 7] =
    [Stat::RecYds, Stat::RecTd, Stat::RushYds, Stat::RushTd, Stat::Sacks, Stat::Tfl, Stat::PassYds];

/// What a team made each season: the stat book's lines of every man on the
/// program, summed (only the seasons asked for are read).
fn team_totals(w: &World, keys: &HashSet<(u16, i16)>) -> HashMap<(u16, i16), [f64; 7]> {
    let mut out: HashMap<(u16, i16), [f64; 7]> = HashMap::new();
    let years: HashSet<i32> = keys.iter().map(|k| k.1 as i32).collect();
    let idx = &w.stats.year_index;
    for (n, &(y, start)) in idx.iter().enumerate() {
        if !years.contains(&y) {
            continue;
        }
        let end = idx.get(n + 1).map_or(w.stats.seasons.len(), |x| x.1);
        for r in &w.stats.seasons[start.min(end)..end] {
            if !keys.contains(&(r.program, r.year)) {
                continue;
            }
            let e = out.entry((r.program, r.year)).or_default();
            for (k, s) in SHARE.iter().enumerate() {
                e[k] += r.s.get(*s as usize) as f64;
            }
        }
    }
    out
}

/// A season's dominator rating: his share of the team's production at his
/// spot, yards and touchdowns weighed alike (PlayerProfiler's); a pass
/// rusher's share of the sacks and tackles for loss (proposed, not
/// confirmed). None for a quarterback, a lineman or a kicker.
fn dominator(pos: Position, r: &SeasonRecord, t: &[f64; 7]) -> Option<f64> {
    let share = |a: f64, b: f64| if b > 0.0 { a / b } else { 0.0 };
    let g = |s: Stat| r.get(s);
    match pos {
        Position::WR | Position::TE => {
            (t[0] > 0.0).then(|| 0.5 * (share(g(Stat::RecYds), t[0]) + share(g(Stat::RecTd), t[1])))
        }
        Position::RB => {
            let yds = t[0] + t[2];
            let td = t[1] + t[3];
            (yds > 0.0).then(|| {
                0.5 * (share(g(Stat::RushYds) + g(Stat::RecYds), yds) + share(g(Stat::RushTd) + g(Stat::RecTd), td))
            })
        }
        Position::EDGE | Position::DT | Position::LB => {
            let all = t[4] + t[5];
            (all > 0.0).then(|| share(g(Stat::Sacks) + g(Stat::Tfl), all))
        }
        _ => None,
    }
}

/// The share that marks a breakout (proposed, not confirmed: 20% for
/// receivers and backs, 15% for tight ends, as PlayerProfiler; 20% of the
/// sacks and tackles for loss for a front seven man).
fn breakout_mark(pos: Position) -> f64 {
    if pos == Position::TE { 0.15 } else { 0.20 }
}

/// The dominator read against his age (proposed, not confirmed: each year
/// younger than 21 on 1 September adds a tenth, each year older takes one
/// off).
fn age_adjusted(dom: f64, age: f64) -> f64 {
    dom * (1.0 + 0.1 * (21.0 - age)).clamp(0.5, 1.5)
}

fn age_at(born: Date, d: Date) -> f64 {
    (d.0 - born.0) as f64 / 365.25
}

/// A season's line in words, by spot.
fn line_cells(pos: Position, r: &SeasonRecord) -> Vec<Value> {
    let g = |s: Stat| r.get(s);
    let f = |l: &str, v: f64| json!([l, v.round() as i64]);
    let mut v = vec![f("G", r.games as f64)];
    match pos {
        Position::QB => {
            v.push(json!(["Cmp-Att", format!("{}-{}", g(Stat::Cmp), g(Stat::Att))]));
            v.extend([f("Yds", g(Stat::PassYds)), f("TD", g(Stat::PassTd)), f("Int", g(Stat::Int))]);
            v.push(f("Rush yds", g(Stat::RushYds)));
        }
        Position::RB => {
            v.extend([f("Car", g(Stat::Rush)), f("Yds", g(Stat::RushYds)), f("TD", g(Stat::RushTd))]);
            v.extend([f("Rec", g(Stat::Rec)), f("Rec yds", g(Stat::RecYds))]);
        }
        Position::WR | Position::TE => {
            v.extend([f("Tgt", g(Stat::Targets)), f("Rec", g(Stat::Rec)), f("Yds", g(Stat::RecYds))]);
            v.push(f("TD", g(Stat::RecTd)));
        }
        Position::OT | Position::IOL => {
            v.extend([f("Snaps", g(Stat::SnapsOff)), f("Press allowed", g(Stat::PressuresAllowed))]);
            v.push(f("Sacks allowed", g(Stat::SacksAllowed)));
        }
        Position::EDGE | Position::DT | Position::LB => {
            v.extend([f("Tkl", g(Stat::Tackles)), f("TFL", g(Stat::Tfl)), f("Sacks", g(Stat::Sacks))]);
            v.push(f("Pressures", g(Stat::Pressures)));
        }
        Position::CB | Position::S => {
            v.extend([f("Tkl", g(Stat::Tackles)), f("Tgt", g(Stat::TgtAllowed)), f("Yds", g(Stat::YdsAllowed))]);
            v.extend([f("Int", g(Stat::Ints)), f("PBU", g(Stat::Pbus))]);
        }
        Position::K => v.extend([f("FG", g(Stat::FgMade)), f("FGA", g(Stat::FgAtt)), f("Long", g(Stat::FgLong))]),
        Position::P | Position::LS => v.extend([f("Punts", g(Stat::Punts)), f("Net", g(Stat::PuntNet))]),
    }
    v
}

/// The markers analysts carry from college to the NFL at each spot, from
/// his stat book line (proposed, not confirmed: the list per spot).
fn markers(pos: Position, r: &SeasonRecord) -> Vec<Value> {
    let g = |s: Stat| r.get(s);
    let rate = |a: f64, b: f64| (b > 0.0).then(|| a / b);
    let mut v: Vec<(&str, &str, Option<f64>, &str)> = Vec::new();
    match pos {
        Position::QB => {
            let att = g(Stat::Att);
            v.push((
                "Adjusted yards an attempt",
                "PcAya",
                rate(g(Stat::PassYds) + 20.0 * g(Stat::PassTd) - 45.0 * g(Stat::Int), att),
                "two",
            ));
            v.push(("Completion over expected", "PcCpoe", rate(g(Stat::Cmp) - g(Stat::XCmp), att), "pct"));
            v.push(("Big-time throw rate", "PcBtt", rate(g(Stat::BigTime), g(Stat::Dropbacks)), "pct"));
            v.push(("Turnover-worthy rate", "PcTwp", rate(g(Stat::TurnoverWorthy), g(Stat::Dropbacks)), "pct"));
        }
        Position::RB => {
            v.push(("Yards after contact a carry", "PcYco", rate(g(Stat::Yaco), g(Stat::Rush)), "two"));
            v.push((
                "Missed tackles forced a touch",
                "PcMtf",
                rate(g(Stat::ForcedMissed), g(Stat::Rush) + g(Stat::Rec)),
                "two",
            ));
            v.push(("Explosive run rate", "PcExplRun", rate(g(Stat::ExplosiveRuns), g(Stat::Rush)), "pct"));
        }
        Position::WR | Position::TE => {
            v.push(("Yards a route run", "PcYprr", rate(g(Stat::RecYds), g(Stat::Routes)), "two"));
            v.push(("Targets a route", "PcTprr", rate(g(Stat::Targets), g(Stat::Routes)), "pct"));
            v.push(("Drop rate", "PcDrop", rate(g(Stat::Drops), g(Stat::Targets)), "pct"));
        }
        Position::OT | Position::IOL => {
            v.push((
                "Pressure rate allowed",
                "PcPressAllowed",
                rate(g(Stat::PressuresAllowed), g(Stat::PbSnaps)),
                "pct",
            ));
            v.push(("Run block win rate", "PcRbwr", rate(g(Stat::RbWins), g(Stat::RbSnaps)), "pct"));
        }
        Position::EDGE | Position::DT => {
            v.push(("Pass rush win rate", "PcPrwr", rate(g(Stat::RushWins), g(Stat::RushSnaps)), "pct"));
            v.push(("Pressure rate", "PcPressRate", rate(g(Stat::Pressures), g(Stat::RushSnaps)), "pct"));
        }
        Position::LB => {
            v.push(("Stops a game", "PcStops", rate(g(Stat::Stops), r.games as f64), "one"));
            v.push((
                "Missed tackle rate",
                "PcMissed",
                rate(g(Stat::Missed), g(Stat::Tackles) + g(Stat::Missed)),
                "pct",
            ));
        }
        Position::CB | Position::S => {
            v.push(("Yards a coverage snap", "PcYpcs", rate(g(Stat::YdsAllowed), g(Stat::CovSnaps)), "two"));
            v.push((
                "Plays on the ball a target",
                "PcBall",
                rate(g(Stat::Ints) + g(Stat::Pbus), g(Stat::TgtAllowed)),
                "pct",
            ));
        }
        _ => {}
    }
    v.into_iter()
        .filter_map(|(label, key, x, fmt)| {
            let x = x?;
            let text = match fmt {
                "pct" => format!("{:.1}%", 100.0 * x),
                "one" => format!("{x:.1}"),
                _ => format!("{x:.2}"),
            };
            Some(json!({ "label": label, "key": key, "value": text }))
        })
        .collect()
}

fn draft_year(w: &World) -> i32 {
    let d = w.today;
    if d.month() >= 5 { d.year() + 1 } else { d.year() }
}

fn draft_day(year: i32) -> Date {
    Date::ymd(year, 4, 25)
}

/// How a man the seasons are like turned out: his draft and his NFL
/// games so far.
fn outcome(w: &World, id: u32) -> Value {
    let pick = w.tally_classes().picks.iter().find(|p| p.id == id);
    let drafted = pick.map(|p| json!({ "year": p.year, "round": p.round, "pick": p.pick, "club": views::team_brief(w, TeamRef::Nfl(p.club as usize)) }));
    let nfl = w.tally_classes().signees.iter().find(|s| s.id == id).map(|s| s.out.nfl);
    json!({
        "drafted": drafted,
        "nfl_games": nfl.as_ref().map(|c| c.games),
        "nfl_starts": nfl.as_ref().map(|c| c.starts),
        "now": views::person_index(w, id).map(|k| views::status_text(w, &w.people[k])),
    })
}

/// Your staff's read, as the scouting screens show it.
fn staff_read(w: &World, id: u32) -> Value {
    readers::seat_staff(w).map_or(Value::Null, |s| crate::lane_scouting::brief_read(w, s, id))
}

fn is_hs(p: &Person) -> bool {
    matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)
}

// ---------------------------------------------------------------------
// `prospect_card`.

pub fn prospect_card(w: &World, args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_u64).map(|x| x as u32) else {
        return json!({ "error": "Which player?" });
    };
    let Some(k) = views::person_index(w, id) else {
        return json!({ "error": "That player is no longer in football." });
    };
    let p = &w.people[k];
    let (lh, lw) = file::listed(&p.player);
    let mut out = json!({
        "id": id,
        "name": p.name,
        "pos": p.player.position.name(),
        "team": views::team_of(w, p).map(|t| views::team_brief(w, t)),
        "class": views::class_text(w, p),
        "hometown": views::hometown(w, p),
        "age": format!("{:.1}", age_at(p.born, w.today)),
        "listed": { "height": views::ft_in(lh), "weight": lw },
        "stars": p.stars,
        "read": staff_read(w, id),
    });
    let more = if is_hs(p) { hs_card(w, p) } else { college_card(w, k) };
    if let (Some(o), Value::Object(m)) = (out.as_object_mut(), more) {
        o.extend(m);
    }
    out
}

fn college_card(w: &World, k: usize) -> Value {
    let p = &w.people[k];
    let id = p.player.id;
    let pos = p.player.position;
    let career = w.stats.career(id);
    let keys: HashSet<(u16, i16)> = career.iter().map(|r| (r.program, r.year)).collect();
    let totals = team_totals(w, &keys);
    let mut breakout: Option<(i16, f64)> = None;
    let mut best_adj: Option<f64> = None;
    let seasons: Vec<Value> = career
        .iter()
        .map(|r| {
            let age = age_at(p.born, Date::ymd(r.year as i32, 9, 1));
            let dom = totals.get(&(r.program, r.year)).and_then(|t| dominator(r.pos, r, t));
            if let Some(d) = dom {
                if breakout.is_none() && d >= breakout_mark(r.pos) {
                    breakout = Some((r.year, age));
                }
                let a = age_adjusted(d, age);
                best_adj = Some(best_adj.map_or(a, |b: f64| b.max(a)));
            }
            json!({
                "year": r.year,
                "team": views::team_brief(w, TeamRef::College(r.program as usize)),
                "division": r.division.name(),
                "pos": r.pos.name(),
                "age": format!("{age:.1}"),
                "line": line_cells(r.pos, r),
                "dominator": dom.map(|d| format!("{:.1}%", 100.0 * d)),
                "age_adjusted": dom.map(|d| format!("{:.1}%", 100.0 * age_adjusted(d, age))),
                "markers": markers(r.pos, r),
            })
        })
        .collect();
    let last = career.last().copied();
    // His analytics card for the last season: markers with percentiles and
    // the seasons most like his.
    let card = last.map(|r| super::analytics::card(w, &json!({ "id": id, "level": "college", "year": r.year })));
    let metrics = card.as_ref().and_then(|c| c.get("metrics")).cloned().unwrap_or(Value::Null);
    let like: Vec<Value> = card
        .as_ref()
        .and_then(|c| c.get("like"))
        .and_then(Value::as_array)
        .map(|v| {
            v.iter()
                .map(|l| {
                    let mut l = l.clone();
                    if let Some(oid) = l.get("id").and_then(Value::as_u64) {
                        l["outcome"] = outcome(w, oid as u32);
                    }
                    l
                })
                .collect()
        })
        .unwrap_or_default();
    // Testing: what your staff's file holds from the combine and pro days
    // (the scouting file's rule), and his size against the real combine.
    let looks = readers::seat_staff(w).and_then(|s| readers::read(w, s, id)).map(|x| x.1).unwrap_or_default();
    let tests: Vec<Value> = [Kind::Combine, Kind::ProDay]
        .iter()
        .filter_map(|&kind| {
            let l = looks.iter().rev().find(|l| l.kind == kind)?;
            let c = file::tests(&p.player, kind, l.date.year());
            let vals = [c.forty, c.bench.round(), c.vertical, c.broad.round(), c.cone, c.shuttle];
            Some(json!({
                "where": kind.name(),
                "date": views::date_text(l.date),
                "rows": TESTS.iter().zip(vals).enumerate().map(|(j, ((key, label), x))| json!({
                    "key": key, "label": label, "value": x, "pct": real_pct(pos, 2 + j, x),
                })).collect::<Vec<_>>(),
            }))
        })
        .collect();
    let measured = !tests.is_empty();
    let (h, wt) = if measured {
        let b = afm_engine::body::body_now(&p.player);
        (b.height, b.weight)
    } else {
        file::listed(&p.player)
    };
    let size = json!({
        "measured": measured,
        "height": views::ft_in(h),
        "weight": wt.round(),
        "height_pct": real_pct(pos, 0, h),
        "weight_pct": real_pct(pos, 1, wt),
    });
    let dy = draft_year(w);
    let public = if matches!(p.stage, Stage::College | Stage::Declared) {
        board::public_ranks(w).get(&id).copied()
    } else {
        None
    };
    let ours = w.scouting.board.order.iter().position(|&x| x == id).map(|r| r + 1);
    json!({
        "kind": "college",
        "seasons": seasons,
        "breakout": breakout.map(|b| json!({ "year": b.0, "age": format!("{:.1}", b.1) })),
        "best_age_adjusted": best_adj.map(|a| format!("{:.1}%", 100.0 * a)),
        "metrics": metrics,
        "metrics_year": last.map(|r| r.year),
        "like": like,
        "tests": tests,
        "size": size,
        "draft": {
            "year": dy,
            "age": format!("{:.1}", age_at(p.born, draft_day(dy))),
            "public": public,
            "ours": ours,
            "declared": p.stage == Stage::Declared,
        },
    })
}

// ---------------------------------------------------------------------
// The high school card.

/// The counts a high schooler's line shows at his spot.
fn hs_keys(pos: Position) -> &'static [(HsStat, &'static str)] {
    use HsStat::*;
    match pos {
        Position::QB => &[(PassYds, "Pass yds"), (PassTd, "Pass TD"), (Int, "Int"), (RushYds, "Rush yds")],
        Position::RB => &[(RushYds, "Rush yds"), (RushTd, "Rush TD"), (RecYds, "Rec yds")],
        Position::WR | Position::TE => &[(Rec, "Rec"), (RecYds, "Rec yds"), (RecTd, "Rec TD")],
        Position::EDGE | Position::DT | Position::LB => &[(Tackles, "Tkl"), (Tfl, "TFL"), (Sacks, "Sacks")],
        Position::CB | Position::S => &[(Tackles, "Tkl"), (Ints, "Int"), (Pbus, "PBU")],
        Position::K => &[(FgMade, "FG"), (XpMade, "XP")],
        _ => &[],
    }
}

fn game_cells(g: &HsGame) -> Vec<Value> {
    use HsStat::*;
    let all = [
        (Cmp, "Cmp"),
        (Att, "Att"),
        (PassYds, "Pass yds"),
        (PassTd, "Pass TD"),
        (Int, "Int"),
        (Rush, "Car"),
        (RushYds, "Rush yds"),
        (RushTd, "Rush TD"),
        (Rec, "Rec"),
        (RecYds, "Rec yds"),
        (RecTd, "Rec TD"),
        (Tackles, "Tkl"),
        (Tfl, "TFL"),
        (Sacks, "Sacks"),
        (Ints, "Int"),
        (Pbus, "PBU"),
        (FgMade, "FG"),
        (XpMade, "XP"),
    ];
    all.iter()
        .filter_map(|&(s, l)| {
            let v = g.get(s as u8);
            (v != 0.0).then(|| json!([l, if v.fract() == 0.0 { json!(v as i64) } else { json!(v) }]))
        })
        .collect()
}

fn rank_json(r: &Rank) -> Value {
    json!({
        "date": views::date_text(r.date),
        "stars": r.stars,
        "rank": r.rank,
        "pos_rank": r.pos_rank,
        "state_rank": r.state_rank,
        "rating": format!("{:.4}", r.rating as f64 / 10_000.0),
    })
}

/// A value's percentile among others (100 the most).
fn pct_among(v: f64, all: &[f64]) -> Option<i64> {
    (all.len() >= 5).then(|| {
        let below = all.iter().filter(|&&x| x < v).count() as f64;
        let same = all.iter().filter(|&&x| x == v).count() as f64;
        (100.0 * (below + 0.5 * same) / all.len() as f64).round() as i64
    })
}

fn hs_card(w: &World, p: &Person) -> Value {
    let id = p.player.id;
    let pos = p.player.position;
    // His seasons in the prospects files (four years at most).
    let seasons: Vec<i32> = w.tally_seasons().into_iter().filter(|&s| s >= p.class_of - 5).collect();
    let mut ranks: Vec<Rank> = Vec::new();
    let mut games: Vec<Value> = Vec::new();
    for &s in seasons.iter().rev() {
        let Some(row) = w.tally_prospect(s, id) else { continue };
        ranks.extend(row.ranks.iter().copied());
        let mut gs: Vec<&HsGame> = row.games.iter().collect();
        gs.sort_by_key(|g| std::cmp::Reverse(g.date));
        for g in gs {
            let site = ["vs", "at", "vs"][g.site.min(2) as usize];
            games.push(json!({
                "season": s,
                "date": views::date_text(g.date),
                "opp": views::team_brief(w, TeamRef::HighSchool(g.opp as usize)),
                "site": site,
                "neutral": g.site == 2,
                "score": g.score,
                "won": g.score[0] > g.score[1],
                "played": g.played,
                "line": game_cells(g),
                "key": g.key,
            }));
        }
    }
    ranks.sort_by_key(|r| r.date);
    ranks.dedup_by_key(|r| r.date);
    // His season lines, each with the state's and the class's percentiles
    // at his spot that year (every kid with a line there).
    let keys = hs_keys(pos);
    let lines: Vec<&HsLine> = p.hs_lines.iter().collect();
    let years: HashSet<i16> = lines.iter().map(|l| l.year).collect();
    let my_state = |l: &HsLine| w.high_schools.get(l.school as usize).map(|h| h.state);
    // (year, state) -> each key's values; (year, class) the same.
    let mut by_state: HashMap<(i16, usize), Vec<Vec<f64>>> = HashMap::new();
    let mut by_class: HashMap<i16, Vec<Vec<f64>>> = HashMap::new();
    if !keys.is_empty() {
        for q in &w.people {
            for l in &q.hs_lines {
                if l.pos != pos || !years.contains(&l.year) || l.games == 0 {
                    continue;
                }
                let vals: Vec<f64> = keys.iter().map(|k| l.get(k.0)).collect();
                if let Some(st) = my_state(l) {
                    let e = by_state.entry((l.year, st)).or_insert_with(|| vec![Vec::new(); keys.len()]);
                    for (j, v) in vals.iter().enumerate() {
                        e[j].push(*v);
                    }
                }
                if q.class_of == p.class_of {
                    let e = by_class.entry(l.year).or_insert_with(|| vec![Vec::new(); keys.len()]);
                    for (j, v) in vals.iter().enumerate() {
                        e[j].push(*v);
                    }
                }
            }
        }
    }
    let season_lines: Vec<Value> = lines
        .iter()
        .rev()
        .map(|l| {
            let st = my_state(l);
            let cells: Vec<Value> = keys
                .iter()
                .enumerate()
                .map(|(j, (s, label))| {
                    let v = l.get(*s);
                    let sp = st.and_then(|st| by_state.get(&(l.year, st))).and_then(|x| pct_among(v, &x[j]));
                    let cp = by_class.get(&l.year).and_then(|x| pct_among(v, &x[j]));
                    json!({ "label": label, "value": v, "state_pct": sp, "class_pct": cp })
                })
                .collect();
            json!({
                "year": l.year,
                "school": views::team_brief(w, TeamRef::HighSchool(l.school as usize)),
                "pos": l.pos.name(),
                "games": l.games,
                "cells": cells,
                "state": st.and_then(|s| w.states.get(s)).map(|s| s.code.clone()),
            })
        })
        .collect();
    // Camp and track testing as timed that day.
    use afm_engine::world::times::{At, Event, Timed};
    let times: Vec<Value> = w
        .times
        .of(id)
        .iter()
        .rev()
        .map(|t| {
            let event = match t.event {
                Event::Forty => "40-yard dash",
                Event::Split => "10-yard split",
                Event::M100 => "100 meters",
                Event::M200 => "200 meters",
                #[allow(unreachable_patterns)]
                _ => "Sprint",
            };
            let timed = match t.timed {
                Timed::Laser => "laser",
                Timed::Fat => "fully automatic",
                Timed::Hand => "hand",
            };
            let at = match t.at {
                At::Meet => "a track meet".to_string(),
                At::StateMeet => "the state meet".to_string(),
                At::Camp(prog) => format!("{} camp", views::team_name(w, TeamRef::College(prog as usize))),
                At::Regional => "a regional showcase".to_string(),
                #[allow(unreachable_patterns)]
                _ => "a national showcase".to_string(),
            };
            json!({
                "event": event, "timed": timed, "secs": format!("{:.2}", t.secs), "date": views::date_text(t.date),
                "at": at, "lb": t.lb,
                "pct": (t.event == Event::Forty).then(|| real_pct(pos, 2, t.secs as f64)).flatten(),
            })
        })
        .collect();
    // Offers and visits.
    let mut offers: Vec<(Option<Date>, usize)> = p.cal.early_offers.iter().map(|&(prog, d)| (Some(d), prog)).collect();
    for &prog in &p.cal.offers {
        if !offers.iter().any(|o| o.1 == prog) {
            offers.push((None, prog));
        }
    }
    let offers: Vec<Value> = offers
        .iter()
        .map(|(d, prog)| {
            json!({ "team": views::team_brief(w, TeamRef::College(*prog)), "date": d.map(views::date_text) })
        })
        .collect();
    let visits: Vec<Value> = p
        .cal
        .visits
        .iter()
        .rev()
        .map(|&(prog, d, v)| {
            let kind = match v {
                Visit::Official => "Official visit",
                Visit::Unofficial => "Unofficial visit",
                Visit::DropIn => "Coach's drop-in",
                Visit::Home => "Home visit",
            };
            json!({ "team": views::team_brief(w, TeamRef::College(prog)), "date": views::date_text(d), "kind": kind })
        })
        .collect();
    let commit = p.cal.commit.map(|(prog, d, walk_on)| {
        json!({ "team": views::team_brief(w, TeamRef::College(prog)), "date": views::date_text(d), "walk_on": walk_on })
    });
    json!({
        "kind": "hs",
        "class_of": p.class_of,
        "school": views::team_brief(w, TeamRef::HighSchool(p.hs)),
        "state": w.states.get(p.home_state).map(|s| s.code.clone()),
        "ranks": ranks.iter().map(rank_json).collect::<Vec<_>>(),
        "games": games,
        "lines": season_lines,
        "times": times,
        "offers": offers,
        "visits": visits,
        "commit": commit,
        "signed": p.cal.signed.is_some(),
    })
}

// ---------------------------------------------------------------------
// `prospects`.

pub fn prospects(w: &World, args: &Value) -> Value {
    let pos =
        args.get("pos").and_then(Value::as_str).and_then(|s| Position::ALL.iter().copied().find(|p| p.name() == s));
    match args.get("level").and_then(Value::as_str) {
        Some("hs") => hs_list(w, args, pos),
        _ => college_list(w, pos),
    }
}

fn college_list(w: &World, pos: Option<Position>) -> Value {
    let dy = draft_year(w);
    let public = board::public_ranks(w);
    let mut ids: Vec<(usize, u32)> = public.iter().map(|(&id, &r)| (r, id)).collect();
    // The declared who aren't on the public board come after it.
    for p in &w.people {
        if p.stage == Stage::Declared && !public.contains_key(&p.player.id) {
            ids.push((usize::MAX, p.player.id));
        }
    }
    ids.sort_unstable();
    let picked: Vec<(usize, usize)> = ids
        .iter()
        .filter_map(|&(r, id)| views::person_index(w, id).map(|k| (r, k)))
        .filter(|&(_, k)| pos.is_none_or(|x| w.people[k].player.position == x))
        .take(300)
        .collect();
    let keys: HashSet<(u16, i16)> = picked
        .iter()
        .flat_map(|&(_, k)| w.stats.career(w.people[k].player.id).into_iter().map(|r| (r.program, r.year)))
        .collect();
    let totals = team_totals(w, &keys);
    let ours: HashMap<u32, usize> = w.scouting.board.order.iter().enumerate().map(|(n, &id)| (id, n + 1)).collect();
    let rows: Vec<Value> = picked
        .iter()
        .map(|&(r, k)| {
            let p = &w.people[k];
            let id = p.player.id;
            let career = w.stats.career(id);
            let mut breakout = None;
            let mut best: Option<f64> = None;
            let mut last_dom = None;
            for s in &career {
                let age = age_at(p.born, Date::ymd(s.year as i32, 9, 1));
                if let Some(d) = totals.get(&(s.program, s.year)).and_then(|t| dominator(s.pos, s, t)) {
                    if breakout.is_none() && d >= breakout_mark(s.pos) {
                        breakout = Some(age);
                    }
                    best = Some(best.map_or(d, |b: f64| b.max(d)));
                    last_dom = Some(d);
                }
            }
            json!({
                "id": id,
                "name": p.name,
                "pos": p.player.position.name(),
                "team": views::team_of(w, p).map(|t| views::team_brief(w, t)),
                "class": views::class_text(w, p),
                "age": format!("{:.1}", age_at(p.born, draft_day(dy))),
                "public": (r != usize::MAX).then_some(r),
                "ours": ours.get(&id),
                "declared": p.stage == Stage::Declared,
                "games": career.iter().map(|s| s.games as u32).sum::<u32>(),
                "dominator": last_dom.map(|d| format!("{:.1}%", 100.0 * d)),
                "best": best.map(|d| format!("{:.1}%", 100.0 * d)),
                "breakout": breakout.map(|a| format!("{a:.1}")),
                "stars": p.stars,
            })
        })
        .collect();
    json!({ "level": "college", "draft": dy, "rows": rows, "pos": pos.map(|p| p.name()) })
}

fn hs_list(w: &World, args: &Value, pos: Option<Position>) -> Value {
    let d = w.today;
    // The class still to sign by default: next year's seniors after
    // February's signing day.
    let first = if d.month() >= 3 { d.year() + 1 } else { d.year() };
    let class = args.get("class").and_then(Value::as_i64).map(|c| c as i32).unwrap_or(first);
    let state = args
        .get("state")
        .and_then(Value::as_str)
        .and_then(|c| w.states.iter().position(|s| s.code.eq_ignore_ascii_case(c)));
    let open = &w.tallies.prospects;
    let mut rows: Vec<(u32, Value)> = Vec::new();
    for (&id, r) in &open.last {
        let Some(k) = views::person_index(w, id) else { continue };
        let p = &w.people[k];
        if p.class_of != class
            || pos.is_some_and(|x| p.player.position != x)
            || state.is_some_and(|s| p.home_state != s)
        {
            continue;
        }
        let games = open.rows.get(&id).map_or(0, |x| x.games.iter().filter(|g| g.played).count());
        rows.push((
            r.rank,
            json!({
                "id": id,
                "name": p.name,
                "pos": p.player.position.name(),
                "school": views::team_brief(w, TeamRef::HighSchool(p.hs)),
                "state": w.states.get(p.home_state).map(|s| s.code.clone()),
                "rank": rank_json(r),
                "offers": p.cal.offers.len().max(p.cal.early_offers.len()),
                "commit": p.cal.commit.map(|c| views::team_brief(w, TeamRef::College(c.0))),
                "games": games,
            }),
        ));
    }
    rows.sort_by_key(|x| x.0);
    let classes: Vec<i32> = (first..first + 4).collect();
    json!({
        "level": "hs",
        "class": class,
        "classes": classes,
        "pos": pos.map(|p| p.name()),
        "state": state.and_then(|s| w.states.get(s)).map(|s| s.code.clone()),
        "rows": rows.into_iter().take(300).map(|x| x.1).collect::<Vec<_>>(),
    })
}
