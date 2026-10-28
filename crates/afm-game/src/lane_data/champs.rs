//! History's new tabs (items 23-25): every winner of one award, conference
//! champions by year, and state champions by state and class.

use afm_engine::world::World;
use afm_engine::world::hs_playoffs;
use serde_json::{Value, json};

use super::{division_arg, state_arg};
use crate::views::{self, TeamRef};

pub fn more(w: &World, args: &Value) -> Value {
    match args.get("tab").and_then(Value::as_str).unwrap_or("conferences") {
        "states" => states(w, args),
        "award" => award(w, args),
        _ => conferences(w, args),
    }
}

fn coach(w: &World, c: Option<usize>) -> Value {
    c.and_then(|c| w.coaches.get(c).map(|x| json!({ "id": c, "name": x.name }))).unwrap_or(Value::Null)
}

/// Conference champions by year in a division (item 24). A title is filed
/// by that season's division and conference (programs move); a season
/// from an older file reads the program's now.
fn conferences(w: &World, args: &Value) -> Value {
    let d = division_arg(w, args);
    let mut rows: Vec<Value> = Vec::new();
    for p in &w.programs {
        for s in p.seasons.iter().filter(|s| s.conference_title && s.division.unwrap_or(p.division) == d) {
            let conf =
                s.conference_won.clone().or_else(|| s.conference.clone()).unwrap_or_else(|| p.conference.clone());
            rows.push(json!({
                "year": s.year,
                "conference": conf,
                "team": views::team_brief(w, TeamRef::College(p.id)),
                "record": format!("{}-{}", s.wins, s.losses),
                "national": s.national_title,
                "coach": coach(w, s.coach),
            }));
        }
    }
    rows.sort_by(|a, b| {
        b["year"].as_i64().cmp(&a["year"].as_i64()).then(a["conference"].as_str().cmp(&b["conference"].as_str()))
    });
    let mut confs: Vec<&str> = rows.iter().filter_map(|r| r["conference"].as_str()).collect();
    confs.sort_unstable();
    confs.dedup();
    let confs: Vec<String> = confs.into_iter().map(str::to_string).collect();
    json!({
        "tab": "conferences",
        "division": d.name(),
        "divisions": afm_engine::world::data::Division::ALL.iter().map(|d| d.name()).collect::<Vec<_>>(),
        "conferences": confs,
        "rows": rows,
    })
}

/// A state's champions by year and class (item 25). A title from before
/// lane L20 names the school's class now.
fn states(w: &World, args: &Value) -> Value {
    let s = state_arg(w, args);
    let code = w.states.get(s).map_or("", |x| x.code.as_str());
    let f = hs_playoffs::format(code);
    let mut rows: Vec<Value> = Vec::new();
    for h in w.high_schools.iter().filter(|h| h.state == s) {
        for x in h.seasons.iter().filter(|x| x.state_title) {
            let class =
                x.title.clone().or_else(|| f.class_label(h.classification).map(str::to_string)).unwrap_or_default();
            rows.push(json!({
                "year": x.year,
                "class": class,
                "team": views::team_brief(w, TeamRef::HighSchool(h.id)),
                "record": format!("{}-{}", x.wins, x.losses),
                "coach": coach(w, x.coach),
            }));
        }
    }
    rows.sort_by(|a, b| {
        b["year"].as_i64().cmp(&a["year"].as_i64()).then(a["class"].as_str().cmp(&b["class"].as_str()))
    });
    let mut classes: Vec<String> = rows.iter().filter_map(|r| r["class"].as_str().map(str::to_string)).collect();
    classes.sort();
    classes.dedup();
    json!({
        "tab": "states",
        "state": code,
        "states": w.states.iter().map(|x| x.code.as_str()).collect::<Vec<_>>(),
        "classes": classes,
        "rows": rows,
    })
}

/// Every winner of one award (item 23), college and NFL.
fn award(w: &World, args: &Value) -> Value {
    let mut names: Vec<(&str, &str)> = Vec::new();
    for h in w.stats.honors.values() {
        for (_, a) in &h.awards {
            names.push(("College", a));
        }
    }
    for a in &w.nfl_book.awards {
        names.push(("NFL", a.1));
    }
    names.sort_unstable();
    names.dedup();
    let pick = args.get("award").and_then(Value::as_str);
    let level = args.get("award_level").and_then(Value::as_str);
    let (lv, name) = names
        .iter()
        .find(|(l, n)| Some(*n) == pick && level.is_none_or(|x| x == *l))
        .or(names.first())
        .copied()
        .unwrap_or(("College", ""));
    let mut rows: Vec<Value> = Vec::new();
    if lv == "College" {
        for (id, h) in &w.stats.honors {
            for (y, _) in h.awards.iter().filter(|(_, a)| *a == name) {
                // His program that season, from the stat book.
                let team = w
                    .stats
                    .career(*id)
                    .iter()
                    .rev()
                    .find(|r| r.year as i32 == *y)
                    .map(|r| views::team_brief(w, TeamRef::College(r.program as usize)));
                rows.push(json!({
                    "year": y, "id": id, "name": h.name, "pos": h.pos.map(|p| p.name()),
                    "linked": views::person_index(w, *id).is_some(), "team": team,
                }));
            }
        }
    } else {
        for a in w.nfl_book.awards.iter().filter(|a| a.1 == name) {
            rows.push(json!({
                "year": a.0, "id": a.2, "name": a.3,
                "pos": views::person_index(w, a.2).map(|k| w.people[k].player.position.name()),
                "linked": views::person_index(w, a.2).is_some(),
                "team": views::team_brief(w, TeamRef::Nfl(a.4)),
            }));
        }
    }
    rows.sort_by_key(|r| std::cmp::Reverse(r["year"].as_i64().unwrap_or(0)));
    json!({
        "tab": "award",
        "award": name,
        "award_level": lv,
        "awards": names.iter().map(|(l, n)| json!({ "level": l, "name": n })).collect::<Vec<_>>(),
        "rows": rows,
    })
}
