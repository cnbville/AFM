//! Lane CS2, the Donors screen: `capital_donors` carries what the screen
//! draws (the fever, your donors with the office's range in words, loyalty,
//! wants and circle), on top of lane C1's donors and lane C4's week.

mod common;

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn the_donors_screen_carries_what_it_draws() {
    let dir = std::env::temp_dir().join(format!("afm-capital-donors-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    common::build_world(&mut g, json!({ "seed": 22, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    assert!(g.call("capital_donors", &json!({})).is_err(), "no seat, no donors");
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

    let v = ok(&mut g, "capital_donors", json!({}));
    assert!(v["today"].is_string() && v["program"].is_string());
    // The fever is a word with moments that explain it.
    let f = &v["fever"];
    assert!(["Cold", "Cool", "Warm", "Hot", "Blazing"].contains(&f["word"].as_str().unwrap()));
    assert!((0.0..=1.0).contains(&f["value"].as_f64().unwrap()));
    let share: f64 = f["moments"].as_array().unwrap().iter().map(|m| m["share"].as_f64().unwrap()).sum();
    assert!(share <= f["value"].as_f64().unwrap() + 1e-9);
    // Donors: wealth in words, never a dollar figure (CP39).
    let donors = v["donors"].as_array().unwrap();
    assert!(!donors.is_empty(), "a program has donors");
    for d in donors {
        assert!(d["name"].is_string() && d["business"].is_string());
        let read = d["read"].as_str().unwrap();
        assert!(!read.contains('$') && !read.chars().any(|c| c.is_ascii_digit()), "{read}");
        let w = d["warmth"].as_f64().unwrap();
        assert!((0.0..=1.0).contains(&w));
        assert!(["Old guard", "New money"].contains(&d["money"].as_str().unwrap()));
        // His lean shows only once you have met him; hidden numbers never leave.
        if d["met"].as_u64().unwrap() == 0 && d["last"].is_null() {
            assert!(d["side"].is_null(), "side is for a donor you've met");
        }
        for hidden in ["capacity", "paper", "nil_lean", "collects"] {
            assert!(d.get(hidden).is_none(), "{hidden} must not leave the engine");
        }
    }
    // Book a lunch for a Tuesday-to-Friday in the first weeks the office allows.
    let id = donors[0]["id"].clone();
    let today = v["today"].as_str().unwrap();
    let mut booked = false;
    for k in 1..40 {
        let day = afm_engine::calendar::Date::ymd(
            today[..4].parse().unwrap(),
            today[5..7].parse().unwrap(),
            today[8..10].parse().unwrap(),
        )
        .add(k);
        let (y, m, d) = day.parts();
        let iso = format!("{y:04}-{m:02}-{d:02}");
        if let Ok(r) = g.call("capital_book", &json!({ "donor": id, "date": iso, "dinner": false })) {
            assert!(r["said"].is_string());
            booked = true;
            let again = ok(&mut g, "capital_donors", json!({}));
            assert!(again["booked"].as_array().unwrap().iter().any(|b| b["donor"] == id && b["date"] == iso));
            break;
        }
    }
    // Some seasons only allow the suite (September to December): then there is nothing to book.
    if !booked {
        assert!(today[5..7].parse::<u32>().unwrap() >= 9, "a day in January to August books");
    }
    let _ = std::fs::remove_dir_all(dir);
}
