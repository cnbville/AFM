//! Lane 6, people: a new job's first two weeks at every level. The welcome
//! letter carries the takeover's schedule, the meetings open on their day,
//! a talk's move takes any tone with your staff's read, a meeting can be
//! handed to the staff, and every people page's data comes back. Then the
//! clock runs a month: the staff holds what you didn't, nothing panics.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

/// Holds a scene to its end with the first answer each step.
fn hold(g: &mut Game, id: u64) -> Value {
    let mut s = ok(g, "people_scene", json!({ "id": id }));
    for _ in 0..16 {
        if s["done"] == true {
            break;
        }
        let picks = s["picks"].as_array().unwrap();
        assert!(!picks.is_empty(), "a scene waiting on you offers answers: {s}");
        let code = picks[0]["code"].as_u64().unwrap();
        s = ok(g, "people_answer", json!({ "id": id, "code": code }));
        assert!(!s["lines"].as_array().unwrap().is_empty(), "{s}");
    }
    assert_eq!(s["done"], true, "the scene ends: {s}");
    s
}

fn takeover(level: &str, seed: u64) {
    let dir = std::env::temp_dir().join(format!("afm-people-{level}-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": seed, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let jobs = ok(&mut g, "jobs", json!({ "level": level }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

    // One letter on day one: the welcome, with the first two weeks in it.
    let mail = ok(&mut g, "mail", json!({}));
    assert_eq!(mail["unread"], 1, "{level}: {mail}");
    let id = mail["rows"][0]["id"].as_str().unwrap().to_string();
    let letter = ok(&mut g, "letter", json!({ "id": id }));
    let table = letter["table"].as_array().unwrap();
    assert!(table.len() >= 6, "{level}: the welcome lists the meetings: {letter}");
    assert!(table[1][1].as_str().unwrap().contains("people_scene"), "{letter}");

    // The hub: meetings waiting, the takeover file, the boss.
    let a = ok(&mut g, "people_agenda", json!({}));
    assert_eq!(a["takeover"], true, "{level}: {a}");
    let waiting = a["waiting"].as_array().unwrap().clone();
    assert!(waiting.len() >= 5, "{level}: {a}");
    let t = ok(&mut g, "people_takeover", json!({}));
    assert!(!t["meetings"].as_array().unwrap().is_empty(), "{t}");

    // A later meeting stays shut until its day, and can't be answered yet.
    if let Some(later) = waiting.iter().find(|s| s["today"] == false && s["late"] == false) {
        let s = ok(&mut g, "people_scene", json!({ "id": later["id"] }));
        assert_eq!(s["future"], true, "{s}");
        assert!(g.call("people_answer", &json!({ "id": later["id"], "code": 900 })).is_err());
    }

    // Today's first meeting, held to the end; the next one handed over.
    let today: Vec<&Value> = waiting.iter().filter(|s| s["today"] == true).collect();
    assert!(!today.is_empty(), "{level}: the first day has a meeting: {a}");
    let first = hold(&mut g, today[0]["id"].as_u64().unwrap());
    assert!(!first["outcome"].as_array().unwrap().is_empty() || first["handed"].is_null(), "{first}");
    if let Some(next) = today.get(1) {
        let h = ok(&mut g, "people_hand", json!({ "id": next["id"] }));
        assert_eq!(h["done"], true);
        assert!(h["handed"].is_string(), "{h}");
    }
    // The pinned items under Continue open the room.
    let st = ok(&mut g, "status", json!({}));
    for p in st["pinned"].as_array().unwrap().iter().filter(|p| p["go"] == "people_scene") {
        assert!(p["id"].is_u64(), "{p}");
    }

    // A one-on-one: the staff's read, any tone, then the end.
    let room = ok(&mut g, "people_room", json!({}));
    let rows = room["rows"].as_array().unwrap();
    assert!(rows.len() > 10, "{level}: {room}");
    let pid = rows[0]["id"].as_u64().unwrap();
    let pp = ok(&mut g, "people_player", json!({ "id": pid }));
    assert!(pp["mood"]["words"].is_string(), "{pp}");
    let sid = ok(&mut g, "people_talk", json!({ "id": pid, "subject": "role" }))["scene"].as_u64().unwrap();
    let s = ok(&mut g, "people_scene", json!({ "id": sid }));
    let mv = s["picks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["code"].as_u64().is_some_and(|c| (100..190).contains(&c)))
        .cloned();
    if let Some(mv) = mv {
        let c = mv["code"].as_u64().unwrap();
        let fiery = c - c % 10 + 3;
        let read = ok(&mut g, "people_expect", json!({ "id": sid, "code": fiery }));
        assert!(read.is_null() || read["what"].is_string(), "{read}");
        let after = ok(&mut g, "people_answer", json!({ "id": sid, "code": fiery }));
        assert!(after["lines"].as_array().unwrap().len() > s["lines"].as_array().unwrap().len(), "{after}");
    }
    let s = ok(&mut g, "people_scene", json!({ "id": sid }));
    if s["done"] != true {
        ok(&mut g, "people_hand", json!({ "id": sid }));
    }

    // Every page's data.
    ok(&mut g, "people_rules_set", json!({ "key": "curfew", "strict": 3 }));
    let r = ok(&mut g, "people_rules", json!({}));
    assert!(!r["kinds"].as_array().unwrap().is_empty(), "{r}");
    let b = ok(&mut g, "people_boss", json!({}));
    assert!(b["boss"]["link"].is_string(), "{b}");
    let staff = ok(&mut g, "people_staff", json!({}));
    if level != "hs" {
        assert!(!staff["rows"].as_array().unwrap().is_empty(), "{level}: {staff}");
        let c = ok(&mut g, "people_candidates", json!({ "role": "oc" }));
        assert!(c["rows"].is_array(), "{c}");
    }
    ok(&mut g, "people_contacts", json!({}));
    ok(&mut g, "people_league", json!({}));
    let log = ok(&mut g, "people_log", json!({}));
    assert!(!log["rows"].as_array().unwrap().is_empty(), "{log}");
    // A person around the team, from a scene's cast.
    let a = ok(&mut g, "people_agenda", json!({}));
    let tok = a["waiting"]
        .as_array()
        .unwrap()
        .iter()
        .chain(a["recent"].as_array().unwrap())
        .flat_map(|s| s["who"].as_array().unwrap().iter())
        .find_map(|w| w["tok"].as_str().map(str::to_string));
    if let Some(tok) = tok {
        let c = ok(&mut g, "people_cast", json!({ "tok": tok }));
        assert!(c["name"].is_string() && c["reputation"].is_array(), "{c}");
    }

    // A month on: the staff holds what you left, and the takeover is done.
    ok(&mut g, "holiday", json!({ "days": 30 }));
    let a = ok(&mut g, "people_agenda", json!({}));
    let left: Vec<&Value> =
        a["waiting"].as_array().unwrap().iter().filter(|s| s["late"] == true && s["stops"] == true).collect();
    assert!(left.is_empty(), "{level}: nothing that stops is left behind: {left:?}");
    // (An NFL club's first team meeting waits for the offseason program in
    // April: the CBA allows no work with players before it.)
    let t = ok(&mut g, "people_takeover", json!({}));
    let held = t["meetings"].as_array().unwrap().iter().filter(|s| s["late"] == true || s["today"] == true);
    assert!(held.clone().count() >= 8 && held.clone().all(|s| s["done"] == true), "{level}: {t}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_college_takeover_runs_its_first_two_weeks() {
    takeover("college", 11);
}

#[test]
fn an_nfl_takeover_runs_its_first_two_weeks() {
    takeover("nfl", 12);
}

#[test]
fn a_high_school_takeover_runs_its_first_two_weeks() {
    takeover("hs", 13);
}
