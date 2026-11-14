//! The Attributes page's data (`attr_view`) on a small world at a college
//! seat: every skill row has the agreed shape, your own read judges where
//! there is one and your staff's stands in where there is none, a recruit
//! nobody has looked at returns Not seen rows without a panic, a kicker's
//! default view is the families only, and a second player gives the gaps.

use afm_engine::attributes::Attr;
use afm_engine::world::Stage;
use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

fn mid(r: &Value) -> Option<f64> {
    let a = r.as_array()?;
    Some((a[0].as_f64()? + a[1].as_f64()?) / 2.0)
}

/// One answer's rules, whatever the player.
fn check(v: &Value) {
    let skills = v["skills"].as_array().unwrap();
    assert_eq!(skills.len(), 67, "all 67 skills");
    let fams = v["default_view"]["families"].as_array().unwrap();
    assert_eq!(fams.len(), 8, "the eight families");
    let also = v["default_view"]["also"].as_array().unwrap();
    let in_play: Vec<&str> =
        skills.iter().filter(|s| s["in_play"] == true).map(|s| s["key"].as_str().unwrap()).collect();
    // The default view is the families' skills plus the also list, nothing else.
    let mut shown: Vec<&str> = fams
        .iter()
        .flat_map(|f| f["skills"].as_array().unwrap())
        .map(|k| k.as_str().unwrap())
        .chain(also.iter().map(|a| a["key"].as_str().unwrap()))
        .collect();
    shown.sort();
    shown.dedup();
    let mut p = in_play.clone();
    p.sort();
    assert_eq!(shown, p, "{}: the default view is the in-play skills", v["pos"]);
    for a in also {
        assert!(a["why"].as_str().is_some_and(|w| !w.is_empty()), "a reason for {}", a["key"]);
    }
    for s in skills {
        let (staff, mine) = (mid(&s["staff"]), mid(&s["mine"]));
        // His own read judges where he has one; the staff's stands in.
        let (judge, want) = match (mine, staff) {
            (Some(m), _) => ("mine", Some(m)),
            (None, Some(x)) => ("staff", Some(x)),
            (None, None) => ("none", None),
        };
        assert_eq!(s["judge"], judge, "{}", s["key"]);
        assert_eq!(s["mid"].as_f64(), want, "{}", s["key"]);
        assert!(s["weight_line"].as_str().is_some(), "{}", s["key"]);
        if want.is_none() {
            assert!(s["band"].is_null() && s["moved"].is_null(), "{}: Not seen has no band or arrow", s["key"]);
        } else if !s["starter"].is_null() {
            assert!(s["band"].as_str().is_some(), "{}: a read against marks has a band", s["key"]);
            // A backup can out-rate a starter at one skill (the marks rank by overall), so only
            // the starter and elite marks are ordered.
            assert!(s["starter"].as_f64() <= s["elite"].as_f64(), "{}", s["key"]);
        }
    }
    assert!(v["strip"]["potential"]["mine"].is_null(), "no own potential read yet");
}

#[test]
fn the_attributes_page_data() {
    let dir = std::env::temp_dir().join(format!("afm-attrview-{}", std::process::id()));
    let mut g = Game::new(dir);
    ok(&mut g, "build_world", json!({ "seed": 17, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let base =
        json!({ "name": "Test Coach", "age": 46, "background": "college_assistant", "seed": 5, "calls": "none" });
    let job = ok(&mut g, "jobs", json!({ "level": "college" }))["jobs"][0]["ref"].clone();
    let mut a = base.clone();
    a["job"] = job;
    ok(&mut g, "start_career", a);
    // A few days of looks so the staff has read his own men.
    for _ in 0..14 {
        ok(&mut g, "day", json!({}));
    }

    let roster = ok(&mut g, "roster", json!({}));
    let rows = roster["rows"].as_array().unwrap().clone();
    let mine_ids: Vec<u32> = rows.iter().filter_map(|r| r["id"].as_u64()).map(|x| x as u32).collect();
    let on = |pos: &str| rows.iter().find(|r| r["pos"] == pos).and_then(|r| r["id"].as_u64()).map(|x| x as u32);
    let (kicker, qb) = (on("K"), on("QB"));
    let (hs_ids, other_ids): (Vec<u32>, Vec<u32>) = {
        let w = g.world.as_ref().unwrap();
        let pick = |stage: Stage, step: usize, n: usize| -> Vec<u32> {
            w.people.iter().filter(|p| p.stage == stage).map(|p| p.player.id).step_by(step).take(n).collect()
        };
        (pick(Stage::HighSchool, 331, 5), pick(Stage::Pro, 211, 3))
    };
    assert!(!mine_ids.is_empty());

    // Your own roster: rows have the shape and the rules.
    let first = mine_ids[0];
    let v = ok(&mut g, "attr_view", json!({ "id": first }));
    check(&v);
    assert!(v["pool_words"].is_string(), "{}", v["pool_words"]);
    assert!(v["with"].is_null());
    let judged = v["skills"].as_array().unwrap().iter().filter(|s| s["judge"] != "none").count();
    println!("{} reads of 67 on his own man {}", judged, v["name"]);
    for id in mine_ids.iter().step_by(9).take(8) {
        check(&ok(&mut g, "attr_view", json!({ "id": id })));
    }

    // A recruit, a pro: no panic, the same shape.
    for id in hs_ids.iter().chain(other_ids.iter()) {
        let v = ok(&mut g, "attr_view", json!({ "id": id }));
        check(&v);
    }
    let unseen = hs_ids.iter().any(|id| {
        let v = ok(&mut g, "attr_view", json!({ "id": id }));
        v["skills"].as_array().unwrap().iter().all(|s| s["judge"] == "none")
    });
    println!("a fully unseen recruit among the sample: {unseen}");

    // A kicker's default view is the families only.
    if let Some(k) = kicker {
        let v = ok(&mut g, "attr_view", json!({ "id": k }));
        check(&v);
        assert_eq!(v["default_view"]["also"].as_array().unwrap().len(), 0);
        let n = v["skills"].as_array().unwrap().iter().filter(|s| s["in_play"] == true).count();
        assert_eq!(n, 10);
    }

    // Compare: a second player gives his rows and the gaps.
    if let Some(q) = qb {
        let v = ok(&mut g, "attr_view", json!({ "id": q, "with": first }));
        let w = &v["with"];
        assert_eq!(w["id"].as_u64(), Some(first as u64));
        assert_eq!(w["skills"].as_array().map(Vec::len), Some(67));
        for gap in w["gaps"].as_array().unwrap() {
            let key = gap["key"].as_str().unwrap();
            let a = v["skills"].as_array().unwrap().iter().find(|s| s["key"] == key).unwrap();
            let b = w["skills"].as_array().unwrap().iter().find(|s| s["key"] == key).unwrap();
            let d = a["mid"].as_f64().unwrap() - b["mid"].as_f64().unwrap();
            assert!((gap["gap"].as_f64().unwrap() - d).abs() < 0.06, "{key}");
        }
    }

    // The hover card carries the weight line for a rating, not for the overall.
    let c = ok(&mut g, "rr_card", json!({ "p": first, "k": Attr::Speed.key(), "lo": 60, "hi": 66 }));
    assert!(c["weight_line"].as_str().is_some_and(|s| s.contains("overall rating")), "{}", c["weight_line"]);
    let c = ok(&mut g, "rr_card", json!({ "p": first, "k": "ovr", "lo": 60, "hi": 66 }));
    assert!(c["weight_line"].is_null());

    // A player who is gone, and no id.
    assert!(g.call("attr_view", &json!({ "id": 4_000_000_000u64 })).is_err());
    assert!(g.call("attr_view", &json!({})).is_err());
}
