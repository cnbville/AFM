//! Compare (lane NB10, Data overhaul item 30, board 54): two to four
//! players or teams side by side. `data_compare` (`kind`):
//! - "players" (`ids`, `level`, `line` "class", "age", "year" or "career",
//!   `at`): each player's season lined up (his freshman season, the season
//!   he was 20, one year, or his whole career at the level) on his group's
//!   card, with his percentile among that season's field; every season of
//!   each lined up by class and by age; what came next;
//! - "teams" (`teams`, a list of `{kind, id}`, `year`): each team's season
//!   (the year asked for, else its last counted) on every team number,
//!   with its rank and percentile in its group that season.
//!
//! The players' numbers are the stat book's (the cards' numbers that need
//! no depth counts), so any season since the books began compares; the
//! teams' follow your charting as the team pages do.

use std::collections::HashMap;

use afm_engine::world::World;
use afm_engine::world::analytics::Cm;
use afm_engine::world::marks::{self, Tier};
use afm_engine::world::records::{STAT_COUNT, Stat};
use serde_json::{Value, json};

use super::analytics::{self as ay, Place};
use super::careers::{self, Bios, Field};
use super::teams::{self as tm, STATS};
use super::{team_json, tier_key};
use crate::views::{self, TeamRef};

const MOST: usize = 4;

/// A player's whole career at a place: the stat book's numbers summed over
/// his seasons there, his games, and his first and last season.
fn career_sum(w: &World, p: Place, bios: &Bios, id: u32) -> Option<([f64; STAT_COUNT], f64, i32, i32)> {
    let mut all = [0f64; STAT_COUNT];
    let mut games = 0.0;
    let mut years: Vec<i32> = Vec::new();
    let mut add = |line: &dyn Fn(usize) -> f64, g: f64, y: i32| {
        for (k, a) in all.iter_mut().enumerate() {
            *a += line(k);
        }
        games += g;
        years.push(y);
    };
    match p.tier {
        Tier::College => {
            for r in w.stats.career(id).into_iter().filter(|r| Some(r.division) == p.division) {
                add(&|k| r.get(Stat::ALL[k]), r.games as f64, r.year as i32);
            }
        }
        Tier::Nfl => {
            for s in bios.pro(id)?.seasons.iter().filter(|s| s.games > 0) {
                let l = afm_engine::world::records::from_line(&s.stats);
                add(&|k| l[k] as f64, s.games as f64, s.year);
            }
        }
        Tier::Hs => {
            let k = views::person_index(w, id)?;
            for l in w.people[k]
                .hs_lines
                .iter()
                .filter(|l| w.high_schools.get(l.school as usize).map(|h| h.state) == p.state)
            {
                let a = marks::hs_all(&l.s.map(|v| v as i32));
                add(&|k| a[k] as f64, l.games as f64, l.year as i32);
            }
        }
    }
    let (first, last) = (*years.iter().min()?, *years.iter().max()?);
    Some((all, games, first, last))
}

fn line_arg(args: &Value) -> &'static str {
    match args.get("line").and_then(Value::as_str) {
        Some("age") => "age",
        Some("year") => "year",
        Some("career") => "career",
        _ => "class",
    }
}

fn players(w: &World, args: &Value) -> Value {
    let ids: Vec<u32> = args
        .get("ids")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_u64().map(|x| x as u32)).collect())
        .unwrap_or_default();
    let mut ids2: Vec<u32> = Vec::new();
    for id in ids {
        if !ids2.contains(&id) && ids2.len() < MOST {
            ids2.push(id);
        }
    }
    let ids = ids2;
    let Some(&first) = ids.first() else {
        return json!({ "kind": "players", "players": [], "empty": "Add a player to compare." });
    };
    let Some((p0, group)) = careers::place_for(w, first, args) else {
        return json!({ "kind": "players", "players": [], "error": "No numbers for this player." });
    };
    let line = line_arg(args);
    let at = args.get("at").and_then(Value::as_i64).map(|x| x as i32);
    let bios = Bios::new(w);
    let mut fields: HashMap<(u8, Option<&'static str>, Option<usize>), Field> = HashMap::new();
    // Each man is read at his own division (or state) at the first one's
    // level, on the first one's group's card.
    let mut level_args = args.clone();
    level_args["level"] = json!(tier_key(p0.tier));
    level_args["group"] = json!(afm_engine::world::analytics::GROUPS[group]);
    if let Some(o) = level_args.as_object_mut() {
        o.remove("division");
        o.remove("state");
    }
    let mut out = Vec::new();
    let mut card: Vec<Cm> = Vec::new();
    let mut key0 = None;
    for &id in &ids {
        let place = careers::place_for(w, id, &level_args).map(|x| x.0).unwrap_or(p0);
        let key = (place.tier as u8, place.division.map(|d| d.name()), place.state);
        key0.get_or_insert(key);
        let f = fields.entry(key).or_insert_with(|| careers::field(w, place, group));
        if card.is_empty() {
            card = f.card.clone();
        }
        let by = f.by_player();
        let seq = by.get(&id).cloned().unwrap_or_default();
        let step = |s: usize| {
            let y = f.seasons[s].year;
            (y, bios.class(place.tier, id, y).map(|c| (c.0, careers::class_text(place.tier, c))), bios.age(id, y))
        };
        // Every season, lined up by class and by age.
        let seasons: Vec<Value> = seq
            .iter()
            .map(|&(s, i)| {
                let fs = &f.seasons[s];
                let r = &fs.rows[i];
                let (y, class, age) = step(s);
                json!({
                    "year": y, "class": class.as_ref().map(|c| c.1.clone()), "class_n": class.as_ref().map(|c| c.0),
                    "age": age, "team": team_json(w, place.tier, r.team), "games": r.games, "vol": r.vol.round() as i64,
                    "qualified": fs.qual[i],
                    "text": r.vals.iter().zip(&f.card).map(|(v, cm)| v.map(|v| ay::fmt_cell(*cm, v))).collect::<Vec<_>>(),
                    "pct": fs.pcts[i].iter().map(|x| x.map(|x| x.round() as i64)).collect::<Vec<_>>(),
                })
            })
            .collect();
        // The season lined up.
        let chosen = match line {
            "career" => None,
            _ => {
                let target = at.unwrap_or(match line {
                    "age" => seq.first().and_then(|&(s, _)| step(s).2).map_or(20, |a| a as i32),
                    "year" => seq.last().map_or(w.year, |&(s, _)| f.seasons[s].year),
                    _ => 1,
                });
                seq.iter().copied().find(|&(s, _)| {
                    let (y, class, age) = step(s);
                    match line {
                        "age" => age.map(|a| a as i32) == Some(target),
                        "year" => y == target,
                        _ => class.map(|c| c.0 as i32) == Some(target),
                    }
                })
            }
        };
        let (cells, label) = match (line, chosen) {
            ("career", _) => match career_sum(w, place, &bios, id) {
                Some((all, games, a, b)) => {
                    let get = |s: Stat| all[s as usize];
                    let cells: Vec<Value> = f
                        .card
                        .iter()
                        .map(|cm| {
                            let v = cm.value(&get, games);
                            json!({ "text": v.map(|v| ay::fmt_cell(*cm, v)), "v": v, "pct": Value::Null })
                        })
                        .collect();
                    (cells, Some(if a == b { a.to_string() } else { format!("{a}-{b}") }))
                }
                None => (Vec::new(), None),
            },
            (_, Some((s, i))) => {
                let fs = &f.seasons[s];
                let r = &fs.rows[i];
                let cells = r
                    .vals
                    .iter()
                    .zip(&f.card)
                    .zip(&fs.pcts[i])
                    .map(|((v, cm), pc)| {
                        json!({ "text": v.map(|v| ay::fmt_cell(*cm, v)), "v": v, "pct": pc.map(|x| x.round() as i64) })
                    })
                    .collect();
                (cells, Some(fs.year.to_string()))
            }
            _ => (Vec::new(), None),
        };
        let chosen_json = chosen.map(|(s, i)| {
            let fs = &f.seasons[s];
            let (_, class, age) = step(s);
            json!({
                "year": fs.year, "team": team_json(w, place.tier, fs.rows[i].team), "class": class.map(|c| c.1),
                "age": age, "games": fs.rows[i].games, "vol": fs.rows[i].vol.round() as i64,
                "field": fs.qual.iter().filter(|q| **q).count(), "qualified": fs.qual[i],
            })
        });
        out.push(json!({
            "id": id,
            "name": ay::name(w, id),
            "linked": views::person_index(w, id).is_some(),
            "division": place.division.map(|d| d.name()),
            "label": label,
            "season": chosen_json,
            "cells": cells,
            "seasons": seasons,
            "outcome": careers::outcome(w, &bios, id),
        }));
    }
    let f0 = key0.and_then(|k| fields.get(&k));
    json!({
        "kind": "players",
        "level": tier_key(p0.tier),
        "group": afm_engine::world::analytics::GROUPS[group],
        "line": line,
        "at": at,
        "volume": f0.map(|f| f.volume),
        "charted": f0.is_some_and(|f| f.charted),
        "metrics": card.iter().map(|c| careers::metric_json(*c)).collect::<Vec<_>>(),
        "players": out,
    })
}

fn teams(w: &World, args: &Value) -> Value {
    let refs: Vec<TeamRef> = args
        .get("teams")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|t| TeamRef::from_args(w, t).ok()).collect())
        .unwrap_or_default();
    let mut list: Vec<(Tier, usize, TeamRef)> = Vec::new();
    for t in refs {
        if let Some((tier, team)) = tm::tier_of(t)
            && !list.iter().any(|x| x.0 == tier && x.1 == team)
            && list.len() < MOST
        {
            list.push((tier, team, t));
        }
    }
    let asked = args.get("year").and_then(Value::as_i64).map(|y| y as i32);
    let mut cols = Vec::new();
    let mut depth_max = 0u8;
    for &(tier, team, t) in &list {
        let level = tm::level_of(tier);
        let (years, first) = tm::team_years(w, tier, team);
        let year = asked.filter(|y| years.contains(y)).or(first);
        let Some(year) = year else {
            cols.push(json!({ "team": views::team_brief(w, t), "empty": "No season counted yet." }));
            continue;
        };
        let pool = tm::pool(w, level, year);
        let lv = tm::level_numbers(&pool);
        let members = tm::members(w, tier, team, year);
        let group: Vec<_> = pool.iter().filter(|s| members.binary_search(&s.team).is_ok()).collect();
        let me = pool.iter().find(|s| s.team == team as u32);
        let depth = tm::depth_for(w, tier, team, year).unwrap_or(0);
        depth_max = depth_max.max(depth);
        let mut cells = Vec::new();
        for st in STATS {
            let sides: &[usize] = if st.unit == 0 { &[0, 1] } else { &[0] };
            for &side in sides {
                let v = me.filter(|_| st.depth <= depth).and_then(|s| (st.f)(s, side, &lv));
                let up = tm::better(st, side);
                let peers: Vec<f64> = group.iter().filter_map(|s| (st.f)(s, side, &lv)).collect();
                let (rank, pct) = match (v, up) {
                    (Some(v), Some(up)) => {
                        let better = peers.iter().filter(|&&x| if up { x > v + 1e-9 } else { x < v - 1e-9 }).count();
                        let worse = peers.iter().filter(|&&x| if up { x < v - 1e-9 } else { x > v + 1e-9 }).count();
                        let same = peers.len().saturating_sub(better + worse) as f64;
                        let n = peers.len().max(1) as f64;
                        (Some(better + 1), Some((100.0 * (worse as f64 + 0.5 * same) / n).round() as i64))
                    }
                    _ => (None, None),
                };
                cells.push(json!({
                    "key": if side == 1 { format!("{}|d", st.key) } else { st.key.to_string() },
                    "v": v, "text": v.map(|v| tm::text(st.fmt, v)), "rank": rank, "pct": pct,
                }));
            }
        }
        cols.push(json!({
            "team": views::team_brief(w, t),
            "level": tier_key(tier),
            "year": year,
            "years": years,
            "group": tm::group_name(w, tier, team, year),
            "of": group.len(),
            "record": me.map(|s| format!("{}-{}", s.wins, s.losses)),
            "cells": cells,
        }));
    }
    // The rows: every team number, a defense's what it allowed.
    let mut rows = Vec::new();
    for st in STATS {
        if st.depth > depth_max {
            continue;
        }
        let unit = match st.unit {
            2 => "The whole team",
            3 => "Special teams",
            _ => "Offense",
        };
        rows.push(json!({ "key": st.key, "label": st.label, "unit": unit, "h8": st.key }));
        if st.unit == 0 {
            rows.push(json!({ "key": format!("{}|d", st.key), "label": format!("{} allowed", st.label), "unit": "Defense", "h8": st.key }));
        }
    }
    rows.sort_by_key(|r| match r["unit"].as_str() {
        Some("Offense") => 0,
        Some("Defense") => 1,
        Some("The whole team") => 2,
        _ => 3,
    });
    json!({ "kind": "teams", "teams": cols, "rows": rows })
}

pub fn data_compare(w: &World, args: &Value) -> Value {
    match args.get("kind").and_then(Value::as_str) {
        Some("teams") => teams(w, args),
        _ => players(w, args),
    }
}

/// `compare_find` (`q`, `kind`): players or teams by name, for the page's
/// "Add" box: anyone with a season in the books, the ones who left
/// football too.
pub fn compare_find(w: &World, args: &Value) -> Value {
    let q = args.get("q").and_then(Value::as_str).unwrap_or("").trim().to_lowercase();
    if q.len() < 2 {
        return json!({ "items": [] });
    }
    let words: Vec<&str> = q.split_whitespace().collect();
    let hit = |name: &str| {
        let n = name.to_lowercase();
        words.iter().all(|w| n.contains(w))
    };
    let mut items = Vec::new();
    if args.get("kind").and_then(Value::as_str) == Some("teams") {
        for (k, p) in w.programs.iter().enumerate().filter(|(k, _)| hit(w.program_name(*k))).take(8) {
            items.push(
                json!({ "ref": { "kind": "college", "id": k }, "name": w.program_name(k), "sub": p.division.name() }),
            );
        }
        for (k, _) in w.nfl.iter().enumerate().filter(|(k, _)| hit(&w.nfl_team_name(*k))).take(4) {
            items.push(json!({ "ref": { "kind": "nfl", "id": k }, "name": w.nfl_team_name(k), "sub": "NFL" }));
        }
        for (k, h) in w.high_schools.iter().enumerate().filter(|(k, _)| hit(w.school_name(*k))).take(6) {
            let st = w.states.get(h.state).map_or("", |s| s.code.as_str());
            items.push(json!({ "ref": { "kind": "hs", "id": k }, "name": w.school_name(k), "sub": format!("High school, {st}") }));
        }
        return json!({ "items": items });
    }
    let has = |id: u32| !w.stats.career(id).is_empty();
    for x in w.people.iter().filter(|x| hit(&x.name)) {
        let id = x.player.id;
        let pro = x.pro.as_ref().is_some_and(|c| c.seasons.iter().any(|s| s.games > 0));
        if !has(id) && !pro && x.hs_lines.is_empty() {
            continue;
        }
        items.push(json!({ "id": id, "name": x.name, "sub": format!("{} · {}", x.player.position.name(), ay::now_text(w, id)) }));
        if items.len() >= 12 {
            break;
        }
    }
    for a in w.alumni.iter().filter(|a| hit(&a.name)) {
        if items.len() >= 16 {
            break;
        }
        let Some(p) = a.player.as_ref() else { continue };
        if views::person_index(w, p.id).is_some() || !has(p.id) {
            continue;
        }
        items.push(json!({ "id": p.id, "name": a.name, "sub": format!("{} · Out of football", a.position.name()) }));
    }
    json!({ "items": items })
}
