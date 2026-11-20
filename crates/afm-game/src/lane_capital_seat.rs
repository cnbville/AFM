//! Capital's Your seat screen (lane CS6, `docs/design/capital.md` CP32-CP37,
//! CP39, board 6): the rooms of the seats that have no donors. A high
//! school coach sees bond night (where the vote stands and what moved it,
//! your work in town, the booster club's builds); an NFL head coach sees
//! the players' report card by room, his leverage with the owner and the
//! rooms he may ask for.
//!
//! - `capital_seat`: the view. Every number is a share of a vote, points
//!   of it, a level or a word; no dollar figure (CP39) and no price.
//! - `capital_bond_ask`: ask the superintendent and the board for a stadium
//!   bond (once a money year, `hs::ask_bond`).
//! - `capital_town_work` `{ work }`: one piece of your work in town before the
//!   vote (`hs::town_work`: a slot of your week).
//!
//! The NFL owner is asked in the owner meeting (`folk::boss`, CP35); this
//! screen shows what he would be asked for and how he leans.

use afm_engine::calendar::{Date, Level};
use afm_engine::world::World;
use afm_engine::world::capital::hs::{self, Work};
use afm_engine::world::capital::nfl;
use afm_engine::world::finance::data::GRADES;
use afm_engine::world::front_office::school::PROJECTS;
use serde_json::{Value, json};

use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        "capital_seat" => game.w().and_then(view),
        "capital_bond_ask" => game.wm().and_then(|w| {
            let said = hs::ask_bond(w)?;
            let mut v = view(w)?;
            v["said"] = json!(said);
            Ok(v)
        }),
        "capital_town_work" => game.wm().and_then(|w| {
            let key = args.get("work").and_then(Value::as_str).ok_or("Missing the work.")?;
            let said = hs::town_work(w, key)?;
            let mut v = view(w)?;
            v["said"] = json!(said);
            Ok(v)
        }),
        _ => return None,
    })
}

/// Your high school, when you coach one.
fn school_seat(w: &World) -> Option<usize> {
    w.seat.filter(|s| s.level == Level::HighSchool).and_then(|s| s.program)
}

/// Your NFL club, when you coach one.
fn club_seat(w: &World) -> Option<usize> {
    w.seat.filter(|s| s.level == Level::Nfl).and_then(|s| s.nfl_team)
}

fn iso(d: Date) -> String {
    let (y, m, dd) = d.parts();
    format!("{y:04}-{m:02}-{dd:02}")
}

fn view(w: &World) -> Reply {
    if let Some(h) = school_seat(w) {
        let seat = hs::seat_json(w).ok_or("Your school's rooms are not set yet. They come with the first season.")?;
        return Ok(high_school(w, h, seat));
    }
    if let Some(t) = club_seat(w) {
        let seat =
            nfl::seat_json(w).ok_or("Your club's rooms are not set yet. They come with the first league year.")?;
        return Ok(pro(w, t, seat));
    }
    Err("Your seat is for a high school or NFL head coach. A college program's rooms are on Campus, Donors and The drive.".into())
}

// ---------------------------------------------------------------------------
// High school
// ---------------------------------------------------------------------------

/// The rows under "What moved the vote": each part of the expected yes
/// share that is big enough to read, in points of the vote.
fn moved(w: &World, h: usize, bond: &Value) -> Vec<Value> {
    let m = &bond["moved"];
    let pts = |k: &str| m[k].as_f64().unwrap_or(0.0);
    let games = m["games"].as_u64().unwrap_or(0) as u32;
    let wins = (m["win"].as_f64().unwrap_or(0.5) * games as f64).round() as u32;
    let state = &w.states[w.high_schools[h].state].code;
    let mut out = Vec::new();
    let mut add = |what: String, points: f64| {
        if points.abs() >= 0.5 {
            out.push(json!({ "when": "", "what": what, "points": points }));
        }
    };
    let title = m["title"].as_bool().unwrap_or(false);
    add(
        if games == 0 {
            "The district's football history".to_string()
        } else {
            format!("The district's season: {wins}-{}{}", games - wins, if title { ", state champions" } else { "" })
        },
        pts("season"),
    );
    add("Crowds against the schools' size".to_string(), pts("crowds"));
    add(format!("The town's money and its football ({state})"), pts("town"));
    add("The tax bill".to_string(), pts("tax"));
    // Your work in town, in the order you did it.
    for t in bond["work"].as_array().into_iter().flatten() {
        let words = t["words"].as_str().unwrap_or("");
        out.push(json!({ "when": t["date"], "what": format!("Your work in town: {words}"), "points": t["points"] }));
    }
    out
}

fn booster_words(share: f64) -> &'static str {
    if share >= 0.9 {
        "has the money ready"
    } else if share >= 0.65 {
        "has about three quarters of it"
    } else if share >= 0.35 {
        "has about half of it"
    } else if share >= 0.1 {
        "has about a quarter of it"
    } else {
        "has little set aside for it"
    }
}

/// The booster club's builds: the rooms it has paid for and the next on
/// your wish list that is a room, with how much of it the club has.
fn boosters(w: &World, h: usize) -> Value {
    let desk = &w.front_office.school;
    if desk.school != Some(h) {
        return Value::Null;
    }
    let b = &desk.boosters;
    let room = |k: usize| !hs::project_rooms(k).is_empty();
    let built: Vec<Value> = b
        .done
        .iter()
        .rev()
        .filter(|(_, k)| room(*k as usize))
        .take(4)
        .map(|(y, k)| json!({ "year": y, "name": PROJECTS[*k as usize].0 }))
        .collect();
    let next =
        b.wish.iter().map(|&k| k as usize).find(|&k| room(k) && !b.done.iter().any(|(_, d)| *d as usize == k)).map(
            |k| {
                let cost = PROJECTS[k].1 * 1.03f64.powi(w.year - 2026);
                let share = (b.balance / cost.max(1.0)).clamp(0.0, 1.0);
                json!({ "name": PROJECTS[k].0, "has": booster_words(share), "share": (share * 100.0).round() as i64 })
            },
        );
    json!({ "built": built, "next": next, "members": b.members })
}

fn work_options(w: &World) -> Value {
    let Some(id) = hs::your_bond(w).filter(|b| b.pending()).map(|b| b.id) else { return Value::Null };
    let b = hs::your_bond(w).expect("your bond");
    let today = w.today;
    let left = today.days_to(b.day);
    let week = today.add(-today.weekday().index());
    let slot = w.capital.town.iter().any(|t| t.bond == id && t.date.add(-t.date.weekday().index()) == week);
    let open = (0..=hs::WORK_DAYS).contains(&left);
    let opens = b.day.add(-hs::WORK_DAYS);
    let list: Vec<Value> = [Work::Rotary, Work::PhoneBank, Work::Forum]
        .iter()
        .map(|&k| {
            let (hours, lift, times) = k.size();
            let done = w.capital.town.iter().filter(|t| t.bond == id && t.work == k).count() as u8;
            let why = if !open {
                if left > hs::WORK_DAYS { "Not yet." } else { "The vote is in." }
            } else if done >= times {
                "As often as it helps."
            } else if slot {
                "You gave the campaign your slot this week."
            } else {
                ""
            };
            json!({
                "key": k.key(),
                "words": k.words(),
                "hours": hours,
                "points": (lift as f64 * 1000.0).round() / 10.0,
                "done": done,
                "times": times,
                "can": why.is_empty(),
                "why": why,
            })
        })
        .collect();
    json!({
        "open": open,
        "opens": iso(opens),
        "slot_used": slot,
        "list": list,
    })
}

fn high_school(w: &World, h: usize, mut seat: Value) -> Value {
    let school = &w.high_schools[h];
    let state = &w.states[school.state].code;
    let bond = seat["bond"].clone();
    let mut v = json!({
        "seat": "hs",
        "today": iso(w.today),
        "school": w.school_name(h),
        "town": format!("{}, {state}", school.town),
        "private": school.private,
        "rooms": seat["rooms"].take(),
        "last": seat["last"].take(),
        "can_ask": seat["can_ask"],
        "asked": seat["asked"],
        "why_not": seat["why_not"].take(),
        "odds": seat["odds"].take(),
        "district": seat["district"].take(),
        "boosters": boosters(w, h),
    });
    if !bond.is_null() {
        let mut b = bond.clone();
        b["moved"] = Value::Array(moved(w, h, &bond));
        let opens_after = if b["package"] == "new" { 2 } else { 1 };
        if b["opens"].is_null() {
            let day = hs::your_bond(w).map(|x| x.day).unwrap_or(w.today);
            b["would_open"] = json!(day.year() + opens_after);
        }
        b["day_iso"] = json!(hs::your_bond(w).map(|x| iso(x.day)));
        v["work"] = work_options(w);
        v["bond"] = b;
    } else {
        v["bond"] = Value::Null;
    }
    v
}

// ---------------------------------------------------------------------------
// NFL
// ---------------------------------------------------------------------------

/// What a room's players say, by the room and the grade on its card line.
fn said(room: &str, grade: Option<&str>) -> String {
    let Some(g) = grade else { return String::new() };
    let at = GRADES.iter().position(|x| *x == g).unwrap_or(6);
    let tone = if at <= 4 {
        0
    } else if at <= 8 {
        1
    } else {
        2
    };
    let lines: [&str; 3] = match room {
        "locker" => [
            "Cramped and dated. Players dress around each other.",
            "Fine. Nothing anyone brags about.",
            "Players like coming in.",
        ],
        "weight" => [
            "Old racks. Half the room lifts at their own gyms.",
            "Does the job, with a line at the busy hours.",
            "Everything they need, and room to use it.",
        ],
        "recovery" | "science" => [
            "Always a wait for treatment.",
            "The staff is good. The tubs and tables are tight.",
            "The best part of the building.",
        ],
        "nutrition" => [
            "Meals are an afterthought.",
            "Breakfast and lunch are good. Dinner is takeout.",
            "Players eat like athletes.",
        ],
        "lounge" => [
            "No space for kids on game day.",
            "There is a room for families, and it fills up.",
            "Families feel at home on game day.",
        ],
        _ => return String::new(),
    };
    format!("\"{}\"", lines[tone])
}

fn pro(w: &World, t: usize, mut seat: Value) -> Value {
    let rooms: Vec<Value> = seat["rooms"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|r| {
                    let mut r = r.clone();
                    let grade = r["grade"].as_str().map(str::to_string);
                    r["said"] = json!(said(r["key"].as_str().unwrap_or(""), grade.as_deref()));
                    r
                })
                .collect()
        })
        .unwrap_or_default();
    let year = w.today.parts().0;
    json!({
        "seat": "nfl",
        "today": iso(w.today),
        "club": w.nfl[t].name.to_string(),
        "year": year,
        "rooms": rooms,
        "leverage": seat["leverage"].take(),
        "share": seat["leverage_share"].take(),
        "why": seat["leverage_why"].take(),
        "offers": seat["offers"].take(),
        "asked": seat["asked"].take(),
        "build": seat["build"].take(),
        "owner": seat["owner"].take(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
        match g.call(cmd, &args) {
            Ok(v) => v,
            Err(e) => panic!("{cmd}: {e}"),
        }
    }

    /// The Your seat screen on a small world: a high school coach sees his
    /// rooms, a bond on the ballot with the poll, what moved it and his work
    /// in town (one slot a week); an NFL head coach sees the report card by
    /// room, his leverage and the rooms he may ask for. No dollar figure
    /// (CP39), and a college seat is pointed to its own screens.
    #[test]
    fn the_seat_screen_reads_a_high_school_and_an_nfl_seat() {
        use afm_engine::world::capital::hs::{Bond, Package};
        use afm_engine::world::capital::nfl::{ClubRooms, ROOMS_NFL};
        use afm_engine::world::finance::nfl::Card;

        let dir = std::env::temp_dir().join(format!("afm-capital-seat-screen-{}", std::process::id()));
        let mut g = Game::new(dir.clone());
        ok(&mut g, "build_world", json!({ "seed": 23, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
        let fresh = ok(&mut g, "save", json!({ "name": "fresh" }))["saved"].as_str().unwrap().to_string();
        let no_seat = g.call("capital_seat", &json!({}));
        assert!(no_seat.is_err(), "no career, no seat");

        // A high school seat.
        let job = ok(&mut g, "jobs", json!({ "level": "hs" }))["jobs"][0]["ref"].clone();
        ok(
            &mut g,
            "start_career",
            json!({ "name": "Test Coach", "age": 35, "background": "hs_coach", "job": job, "seed": 5 }),
        );
        ok(&mut g, "holiday", json!({ "days": 3 }));
        let h = {
            let w = g.wm().unwrap();
            let h = school_seat(w).expect("a school seat");
            w.high_schools[h].private = false;
            h
        };
        let v = ok(&mut g, "capital_seat", json!({}));
        assert_eq!(v["seat"], "hs");
        assert_eq!(v["rooms"].as_array().unwrap().len(), 5);
        assert!(v["bond"].is_null());
        assert!(v["can_ask"].is_boolean());
        // A bond on the ballot in the window: the poll, the line and your work.
        {
            let w = g.wm().unwrap();
            let day = w.today.add(40);
            w.capital.bonds.push(Bond {
                id: 9_000_001,
                schools: vec![h as u32],
                package: Package::Renovation,
                name: "Stadium renovation".into(),
                seats: 6000,
                cost: 8.0e6,
                called: w.today,
                day,
                needs: 0.5,
                asked: false,
                work: 0.0,
                yes: None,
                passed: false,
                opens: None,
                opened: false,
            });
        }
        let v = ok(&mut g, "capital_seat", json!({}));
        let b = &v["bond"];
        assert_eq!(b["needs"], 50);
        assert!(b["poll"].as_i64().is_some_and(|p| (30..80).contains(&p)), "{b}");
        assert!(!b["outlook"].as_str().unwrap().is_empty());
        assert_eq!(v["work"]["open"], true);
        assert_eq!(v["work"]["list"].as_array().unwrap().len(), 3);
        assert!(v["work"]["list"][0]["can"].as_bool().unwrap());
        let before = b["poll"].as_i64().unwrap();
        let done = ok(&mut g, "capital_town_work", json!({ "work": "rotary" }));
        assert!(done["said"].as_str().unwrap().contains("Rotary"), "{done}");
        let moved = done["bond"]["moved"].as_array().unwrap();
        assert!(moved.iter().any(|m| m["what"].as_str().unwrap().contains("Your work in town")), "{moved:?}");
        assert!(done["bond"]["poll"].as_i64().unwrap() >= before, "work only helps");
        assert!(g.call("capital_town_work", &json!({ "work": "phone_bank" })).is_err(), "one slot a week");
        assert_eq!(done["work"]["list"][1]["can"], false);
        assert!(!done.to_string().contains('$'), "no dollar figure on screen: {done}");

        // An NFL seat.
        ok(&mut g, "load", json!({ "path": fresh }));
        let job = ok(&mut g, "jobs", json!({ "level": "nfl" }))["jobs"][0]["ref"].clone();
        ok(
            &mut g,
            "start_career",
            json!({ "name": "Test GM", "age": 45, "background": "nfl_assistant", "job": job, "seed": 3 }),
        );
        let t = {
            let w = g.wm().unwrap();
            let t = club_seat(w).expect("a club seat");
            if w.capital.clubs.len() < w.nfl.len() {
                w.capital.clubs.resize(w.nfl.len(), ClubRooms::default());
            }
            let mut c = ClubRooms { set: true, ..Default::default() };
            for (k, r) in ROOMS_NFL.iter().enumerate() {
                c.level[*r as usize] = 1.0 + k as f32 * 0.5;
            }
            w.capital.clubs[t] = c;
            w.finance.cards.push(Card { year: w.year, team: t as u8, grades: [3; 11], rank: 20 });
            t
        };
        let _ = t;
        let v = ok(&mut g, "capital_seat", json!({}));
        assert_eq!(v["seat"], "nfl");
        assert_eq!(v["rooms"].as_array().unwrap().len(), ROOMS_NFL.len());
        let weight = v["rooms"].as_array().unwrap().iter().find(|r| r["key"] == "weight").unwrap();
        assert_eq!(weight["grade"], "D");
        assert!(weight["said"].as_str().unwrap().contains("Old racks"), "{weight}");
        assert!(v["leverage"].as_str().is_some_and(|s| !s.is_empty()));
        assert!(v["share"].as_f64().is_some_and(|s| (0.0..=1.0).contains(&s)));
        assert!(v["offers"].is_array());
        assert!(v["offers"].as_array().unwrap().len() <= 3);
        assert!(!v.to_string().contains('$'), "no dollar figure on screen: {v}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn grades_read_in_three_tones() {
        assert!(said("weight", Some("D")).contains("Old racks"));
        assert!(said("weight", Some("B-")).contains("Does the job"));
        assert!(said("locker", Some("A-")).contains("like coming in"));
        assert_eq!(said("weight", None), "");
        assert_eq!(said("indoor", Some("B")), "");
    }
}
