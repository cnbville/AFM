//! Lane 18, finding and hiring coaches: let your coordinator go, search
//! every coach for the job (no cap, filters, pages), read a stranger as the
//! whole scale and sharpen it with film, offer low and get a warning, not
//! the end, then make an offer he can weigh; coaches to watch come back.

mod common;

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

fn hold(g: &mut Game, id: u64) {
    let mut s = ok(g, "people_scene", json!({ "id": id }));
    for _ in 0..16 {
        if s["done"] == true {
            return;
        }
        let code = s["picks"][0]["code"].as_u64().unwrap();
        s = ok(g, "people_answer", json!({ "id": id, "code": code }));
    }
    if s["done"] != true {
        ok(g, "people_hand", json!({ "id": id }));
    }
}

#[test]
fn a_college_coach_finds_reads_and_offers_any_coach() {
    let dir = std::env::temp_dir().join(format!("afm-hiring-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    common::build_world(&mut g, json!({ "seed": 21, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

    // Let the defensive coordinator go to open his job.
    let staff = ok(&mut g, "people_staff", json!({}));
    let dc = staff["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["role"].as_str().unwrap_or("").to_lowercase().contains("defensive coordinator"))
        .and_then(|r| r["coach"].as_u64());
    if let Some(dc) = dc {
        let s = ok(&mut g, "people_fire", json!({ "coach": dc }));
        hold(&mut g, s["scene"].as_u64().unwrap());
    }
    let staff = ok(&mut g, "people_staff", json!({}));
    assert!(staff["openings"].as_array().unwrap().iter().any(|o| o["key"] == "dc"), "{}", staff["openings"]);

    // Every coach who could take it, paged: no 24 cap.
    let s = ok(&mut g, "hiring_search", json!({ "role": "dc" }));
    let total = s["total"].as_u64().unwrap();
    assert!(total > 100, "{total}");
    assert!(s["rows"].as_array().unwrap().len() <= 50);
    assert!(s["coord"] == true && s["rows"][0]["scheme"].is_array(), "C26: {}", s["rows"][0]);
    let s2 = ok(&mut g, "hiring_search", json!({ "role": "dc", "page": 1 }));
    assert_ne!(s["rows"][0]["coach"], s2["rows"][0]["coach"]);
    // Filters narrow it: high school coaches, out of work, juniors.
    for (k, v) in [("level", "hs"), ("job", "junior"), ("job", "none")] {
        let f = ok(&mut g, "hiring_search", json!({ "role": "dc", k: v }));
        let n = f["total"].as_u64().unwrap();
        assert!(n > 0 && n < total, "{k}={v}: {n} of {total}");
    }

    // A stranger reads as the whole scale (his teaching down to half as
    // wide where a coordinator of yours is strong, C58); film narrows it.
    let hs = ok(&mut g, "hiring_search", json!({ "role": "dl", "level": "hs" }));
    // A man under 58: from 58 anyone can see he has peaked (learning stops
    // there, `coach_look::peaked`), whatever the read.
    let c = hs["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["age"].as_u64().is_some_and(|a| a < 57))
        .expect("a high school line coach under 57")["coach"]
        .as_u64()
        .unwrap();
    let p = ok(&mut g, "hiring_coach", json!({ "coach": c }));
    for f in p["facets"].as_array().unwrap() {
        if f["group"] == "Teaching" {
            assert!(f["read"][1].as_i64().unwrap() - f["read"][0].as_i64().unwrap() >= 9, "{f}");
        } else {
            assert_eq!(f["read"], json!([1, 20]), "{f}");
        }
    }
    assert!(p["potential"].as_str().unwrap().starts_with("Too early"), "{}", p["potential"]);
    let wide = |p: &Value| {
        p["facets"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["group"] == "Teaching")
            .map(|f| f["read"][1].as_i64().unwrap() - f["read"][0].as_i64().unwrap())
            .min()
            .unwrap()
    };
    let before = wide(&p);
    let r = ok(&mut g, "hiring_film", json!({ "coach": c }));
    assert!(wide(&r["page"]) < before, "{}", r["page"]["facets"]);

    // An offer far under his ask is a warning, and the talks go on.
    let x = ok(&mut g, "hiring_coach", json!({ "coach": c }));
    let job = x["openings"].as_array().unwrap().iter().find(|o| o["key"] == "dc").unwrap().clone();
    let asks = job["asksValue"].as_f64().unwrap();
    let low =
        ok(&mut g, "hiring_offer", json!({ "coach": c, "role": "dc", "salary": (asks * 0.4).round(), "years": 2 }));
    assert!(low["page"]["talking"] == true || low["page"]["talk"].is_string(), "{low}");
    // A good offer with a promise and a clause: he takes it, thinks it over
    // or names his number; never an error.
    let good = ok(
        &mut g,
        "hiring_offer",
        json!({ "coach": c, "role": "dc", "salary": (asks * 1.4).round(), "years": 3, "promises": ["calling"], "clauses": [{ "mark": 2, "reward": 2 }] }),
    );
    assert!(good["text"].is_string(), "{good}");
    ok(&mut g, "holiday", json!({ "days": 3 }));
    ok(&mut g, "hiring_coach", json!({ "coach": c }));

    // Coaches to watch.
    let w = ok(&mut g, "hiring_watch", json!({ "level": "college", "year": 2025 }));
    assert!(w["rows"].is_array() && w["rooms"].is_array(), "{w}");
    std::fs::remove_dir_all(&dir).ok();
}

/// Graduate assistants and analysts (C36; GAPS "you can't see or fire your
/// graduate assistants"): they are on the Staff page with their pay, a
/// raise and Let go reach them, letting one go owes nothing and opens the
/// post, and the open post is filled by a hire from the list or by a
/// position coach bringing one in.
#[test]
fn a_college_coach_raises_lets_go_and_hires_graduate_assistants() {
    let dir = std::env::temp_dir().join(format!("afm-hiring-ga-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    common::build_world(&mut g, json!({ "seed": 21, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

    let staff = ok(&mut g, "people_staff", json!({}));
    let gas = |s: &Value| -> Vec<Value> {
        s["juniors"].as_array().unwrap().iter().filter(|r| r["postKey"] == "ga").cloned().collect()
    };
    let open = |s: &Value| -> u64 {
        s["juniorPosts"].as_array().unwrap().iter().find(|p| p["key"] == "ga").and_then(|p| p["open"].as_u64()).unwrap()
    };
    let before = gas(&staff);
    assert!(!before.is_empty(), "graduate assistants on the Staff page: {}", staff["juniors"]);
    let open0 = open(&staff);
    // They aren't listed twice (support staff below).
    let ga = before[0]["coach"].as_u64().unwrap();
    assert!(!staff["support"].as_array().unwrap().iter().any(|d| d["link"] == before[0]["link"]));

    // The coach page's moves: a raise and Let go.
    let moves = ok(&mut g, "moves", json!({ "kind": "coach", "id": ga }));
    let ks: Vec<&str> = moves.as_array().unwrap().iter().filter_map(|m| m["k"].as_str()).collect();
    assert!(ks.contains(&"raise") && ks.contains(&"fire"), "{moves}");

    // A raise lifts his pay above the post's.
    let pay = before[0]["salary"].as_str().unwrap().to_string();
    let r = ok(&mut g, "people_raise", json!({ "coach": ga, "share": 0.2 }));
    let row = gas(&r["staff"]).into_iter().find(|x| x["coach"] == ga).unwrap();
    assert_ne!(row["salary"].as_str().unwrap(), pay, "{}", r["text"]);

    // Let go: nothing owed, the post opens.
    let s = ok(&mut g, "people_fire", json!({ "coach": ga }));
    hold(&mut g, s["scene"].as_u64().unwrap());
    let staff = ok(&mut g, "people_staff", json!({}));
    assert_eq!(gas(&staff).len(), before.len() - 1);
    assert_eq!(open(&staff), open0 + 1);
    assert!(staff["owed"].is_null(), "a junior post owes nothing: {}", staff["owed"]);

    // Who could take it: former players and young coaches, with their ties.
    let c = ok(&mut g, "people_junior_candidates", json!({ "post": "ga" }));
    let rows = c["rows"].as_array().unwrap();
    assert!(rows.len() >= 5, "{}", c);
    assert!(rows.iter().all(|r| r["age"].as_u64().unwrap() <= 28), "{c}");
    let pick = rows.iter().find(|r| r["takes"] == true).expect("someone takes it");
    let mut a = json!({ "post": "ga" });
    if pick["coach"].is_u64() {
        a["coach"] = pick["coach"].clone();
    } else {
        a["former"] = pick["former"].clone();
    }
    let h = ok(&mut g, "people_junior_hire", a.clone());
    assert_eq!(gas(&h["staff"]).len(), before.len(), "{}", h["text"]);
    assert_eq!(open(&h["staff"]), open0);
    // He is on your staff now: he can't be hired again.
    assert!(g.call("people_junior_hire", &a).is_err());

    // Let another go, and the linebackers coach brings one in.
    let next = gas(&h["staff"])[0]["coach"].as_u64().unwrap();
    let s = ok(&mut g, "people_fire", json!({ "coach": next }));
    hold(&mut g, s["scene"].as_u64().unwrap());
    let p = ok(&mut g, "people_junior_pick", json!({ "post": "ga", "group": "lb" }));
    let rows = gas(&p["staff"]);
    assert_eq!(open(&p["staff"]), open0, "{}", p["text"]);
    assert!(rows.iter().any(|r| r["room"] == "Linebackers"), "{}", p["staff"]["juniors"]);

    let _ = std::fs::remove_dir_all(dir);
}
