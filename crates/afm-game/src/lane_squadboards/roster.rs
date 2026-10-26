//! V3, the roster at a calm glance: every man by position group with his
//! number, his years, your staff's read, his last six games, his share of
//! the snaps, his mood and his money (NIL, his contract, or his offers),
//! each group's starters ranked against the level. Built on the training
//! lane's squad rows, so the other squad views read the same men.

use std::collections::HashMap;

use afm_engine::positions::Position;
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use super::{class_full, game_grade, health, my_games, my_team, snaps_in, team_snaps, unit_card};
use crate::Reply;
use crate::views::{self, TeamRef};

/// The season a date falls in: a game in January belongs to the fall
/// before.
fn season_of(d: afm_engine::calendar::Date) -> i32 {
    if d.month() <= 6 { d.year() - 1 } else { d.year() }
}

/// A group's key, its name, its positions and its starters by position.
type Group = (&'static str, &'static str, &'static [Position], &'static [(Position, usize)]);

/// The rail's groups and the starters each is ranked by.
const GROUPS: [Group; 9] = {
    use Position::*;
    [
        ("QB", "Quarterbacks", &[QB], &[(QB, 1)]),
        ("RB", "Running backs", &[RB], &[(RB, 1)]),
        ("WR", "Wide receivers", &[WR], &[(WR, 3)]),
        ("TE", "Tight ends", &[TE], &[(TE, 1)]),
        ("OL", "Offensive line", &[OT, IOL], &[(OT, 2), (IOL, 3)]),
        ("DL", "Defensive line", &[EDGE, DT], &[(EDGE, 2), (DT, 2)]),
        ("LB", "Linebackers", &[LB], &[(LB, 3)]),
        ("DB", "Defensive backs", &[CB, S], &[(CB, 2), (S, 2)]),
        ("ST", "Specialists", &[K, P, LS], &[(K, 1), (P, 1), (LS, 1)]),
    ]
};

fn group_of(pos: Position) -> usize {
    GROUPS.iter().position(|g| g.2.contains(&pos)).unwrap_or(8)
}

/// Money in a few characters: $850K, $12.5M.
pub(crate) fn short_money(d: f64) -> String {
    if d >= 1_000_000.0 {
        let m = d / 1_000_000.0;
        if m >= 10.0 { format!("${m:.0}M") } else { format!("${m:.1}M") }
    } else if d >= 1000.0 {
        format!("${:.0}K", d / 1000.0)
    } else {
        format!("${d:.0}")
    }
}

/// His years as pips and words: high school's four grades, college's
/// five-year clock, an NFL contract's years.
fn years(w: &World, i: usize) -> Value {
    let p = &w.people[i];
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => {
            let seniors = if w.today < w.dates.hs_school_start { w.year } else { w.year + 1 };
            let grade = 4 - (p.class_of - seniors).clamp(0, 3);
            json!({ "n": 4, "done": grade, "words": class_full(w, p), "left": format!("Class of {}", p.class_of) })
        }
        Stage::College => {
            let five = w.changes.rules(w.year).five_in_five;
            let limit = if five { 5 } else { 4 };
            let used = p.college_seasons as i32;
            let clock = (p.class_of + 5 - w.year).max(0);
            let left = (limit - used).min(clock).max(0);
            let done = (w.year - p.class_of + 1).clamp(1, 5);
            let words = match left {
                0 => "Last season".to_string(),
                1 => "This season left".to_string(),
                n => format!("{n} seasons left"),
            };
            json!({ "n": 5, "done": done, "words": class_full(w, p), "left": words, "redshirt": p.redshirt_used })
        }
        _ => {
            let k = p.pro.as_ref().and_then(|c| c.contract.as_ref());
            match k {
                Some(k) => {
                    let total = k.real_years().max(1) as i32;
                    let last = k.last_year();
                    let done = (total - (last - w.year)).clamp(1, total);
                    json!({ "n": total.min(7), "done": done.min(7), "words": class_full(w, p), "left": format!("Through {last}") })
                }
                None => json!({ "n": 0, "done": 0, "words": class_full(w, p), "left": "No contract" }),
            }
        }
    }
}

pub(crate) fn view(w: &World) -> Reply {
    let t = my_team(w)?;
    let squad = crate::lane_training::squad(w)?;
    let nums: HashMap<u32, u8> =
        crate::lane_core::people::jerseys(w, t).into_iter().map(|(k, n)| (w.people[k].player.id, n)).collect();
    // The season the board reads: this one once it has a game, else the last.
    let games = my_games(w);
    let now = season_of(w.today);
    let season = if games.iter().any(|(g, _)| season_of(g.date) == now) { now } else { now - 1 };
    let this: Vec<_> = games.iter().filter(|(g, _)| season_of(g.date) == season).copied().collect();
    let last6: Vec<_> = games.iter().rev().take(6).rev().copied().collect();
    let level = match t {
        TeamRef::HighSchool(_) => "hs",
        TeamRef::Nfl(_) => "nfl",
        _ => "college",
    };
    let mut rows: Vec<Value> = Vec::new();
    for mut r in squad["rows"].as_array().cloned().unwrap_or_default() {
        let id = r["id"].as_u64().unwrap_or(0) as u32;
        let Some(i) = views::person_index(w, id) else { continue };
        let p = &w.people[i];
        let pos = p.player.position;
        r["num"] = json!(nums.get(&id));
        r["rail"] = json!(GROUPS[group_of(pos)].0);
        r["years"] = years(w, i);
        let (h, why) = health(w, i);
        r["health"] = json!(h);
        r["health_why"] = json!(why);
        // His last six of your games: a grade, or null when he didn't play.
        r["last6"] = json!(
            last6
                .iter()
                .map(|(g, side)| {
                    let snaps = snaps_in(g, *side, id, pos).unwrap_or(0)
                        + g.stats.players[*side].get(&id).map_or(0, |l| l.snaps_st);
                    if snaps == 0 { Value::Null } else { json!(game_grade(g, *side, id).map(f64::round)) }
                })
                .collect::<Vec<_>>()
        );
        let (mut his, mut all) = (0u32, 0u32);
        for (g, side) in &this {
            his += snaps_in(g, *side, id, pos).unwrap_or(0);
            all += team_snaps(g, *side, pos);
        }
        r["snaps"] = json!(his);
        r["snaps_pct"] = json!((all > 0).then(|| (100.0 * his as f64 / all as f64).round()));
        r["unhappy"] = json!(p.mood <= -0.15);
        r["money"] = match level {
            "college" => {
                json!({ "label": "NIL", "value": (p.nil_income > 0.0).then(|| short_money(p.nil_income)), "note": "a year" })
            }
            "nfl" => {
                let k = p.pro.as_ref().and_then(|c| c.contract.as_ref());
                json!({ "label": "Cap", "value": k.map(|k| short_money(k.cap_hit(w.year))), "note": k.map(|k| format!("{} a year through {}", short_money(k.apy()), k.last_year())) })
            }
            _ => {
                let n = views::offers_of(p).len();
                json!({ "label": "Offers", "value": (n > 0).then(|| n.to_string()), "note": (n > 0).then(|| format!("{n} college offer{}", if n == 1 { "" } else { "s" })) })
            }
        };
        // This season's line (or the last one he played).
        let seasons = crate::lane_core::pages::seasons(w, i);
        let line = seasons.iter().rev().find(|s| s["year"].as_i64() == Some(season as i64)).or(seasons.last());
        r["line"] = json!(
            line.map(|s| json!({ "year": s["year"], "games": s["games"], "line": s["line"], "level": s["level"] }))
        );
        r["class_full"] = json!(class_full(w, p));
        rows.push(r);
    }
    let groups: Vec<Value> = GROUPS
        .iter()
        .map(|(key, name, positions, spots)| {
            let men: Vec<&Value> = rows.iter().filter(|r| r["rail"] == *key).collect();
            let mut mix: Vec<(String, usize)> = Vec::new();
            for r in &men {
                let c = r["class"].as_str().unwrap_or("").trim_start_matches("RS ").to_string();
                match mix.iter_mut().find(|x| x.0 == c) {
                    Some(x) => x.1 += 1,
                    None => mix.push((c, 1)),
                }
            }
            mix.sort_by_key(|x| crate::lane_squadboards::roster::class_order(&x.0));
            json!({
                "key": key, "name": name, "count": men.len(),
                "positions": positions.iter().map(|p| p.name()).collect::<Vec<_>>(),
                "unit": unit_card(w, t, key, spots),
                "mix": mix.into_iter().map(|(c, n)| json!({ "class": c, "n": n })).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(json!({
        "team": squad["team"],
        "level": level,
        "level_name": super::level_name(w, t),
        "season": season,
        "season_words": if season == now { "this season".to_string() } else { format!("the {season} season") },
        "games": this.len(),
        "last6_dates": last6.iter().map(|(g, side)| json!({ "date": views::date_text(g.date), "opp": g.names[1 - side] })).collect::<Vec<_>>(),
        "groups": groups,
        "rows": rows,
    }))
}

/// Classes in their order: freshmen to seniors, rookies to veterans.
pub(crate) fn class_order(c: &str) -> i32 {
    match c {
        "Fr" => 1,
        "So" => 2,
        "Jr" => 3,
        "Sr" => 4,
        "Rookie" => 0,
        _ => c.split(' ').next().and_then(|n| n.parse().ok()).unwrap_or(50),
    }
}
