//! Finding players as a real scouting search (update 2, lane 7): a college
//! staff's Find recruits with its default search, filters joined by "and"
//! and "or", the not-seen switch, the sorts, the chart, Compare and a
//! saved search; the services' stars for the underclasses (minor bug 12);
//! the recruiting map and a recruit's race (boards 7 and 8); an NFL club's
//! Find prospects; a high school coach's Colleges calling.

use std::time::Instant;

use afm_engine::world::front_office::{college as desk, race};
use afm_engine::world::scouting::Kind;
use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

fn timed(g: &mut Game, cmd: &str, args: Value) -> (Value, u128) {
    let t = Instant::now();
    let v = ok(g, cmd, args);
    (v, t.elapsed().as_millis())
}

#[test]
fn a_college_staff_searches_a_class() {
    let dir = std::env::temp_dir().join(format!("afm-find-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 7, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let fresh = ok(&mut g, "save", json!({ "name": "fresh" }))["saved"].as_str().unwrap().to_string();
    let job = ok(&mut g, "jobs", json!({ "level": "college" }))["jobs"][0]["ref"].clone();
    ok(
        &mut g,
        "start_career",
        json!({ "name": "Test Coach", "age": 41, "background": "college_assistant", "job": job, "seed": 9 }),
    );

    // The default search: 3 stars and up or your own state, sorted by the
    // composite rank, the count with its thousands comma.
    let (page, ms) = timed(&mut g, "find_recruits", json!({}));
    assert!(page["default"].as_bool().unwrap());
    assert_eq!(page["sort"], json!("nat"));
    assert!(page["filter_words"].as_str().unwrap().contains(" or "), "{}", page["filter_words"]);
    let rows = page["rows"].as_array().unwrap();
    assert!(!rows.is_empty(), "the default search finds recruits");
    let nats: Vec<u64> = rows.iter().filter_map(|r| r["nat"].as_u64()).collect();
    assert!(nats.windows(2).all(|w| w[0] < w[1]), "sorted by the composite rank: {nats:?}");
    assert!(!page["count_text"].as_str().unwrap().is_empty());
    println!("default search: {} of {} in {ms} ms", page["count"], page["known"]);

    // The juniors and sophomores carry the services' stars (minor bug 12).
    let classes = page["classes"].as_array().unwrap().clone();
    let junior = ok(&mut g, "find_recruits", json!({ "class": classes[1], "filters": [] }));
    let starred = junior["rows"].as_array().unwrap().iter().filter(|r| r["stars"].as_u64().unwrap_or(0) > 0).count();
    assert!(starred > 0, "the class behind the signing one has stars: {}", junior["update"]);
    assert!(junior["services"].as_array().is_some_and(|s| !s.is_empty()), "the services by name");

    // A filter on your staff's reads leaves the not-seen out, or lets them
    // in; either way the kids it can't read are counted.
    let f = json!([{ "k": "now", "op": "ge", "v": 1 }]);
    let left = ok(&mut g, "find_recruits", json!({ "filters": f, "unseen": false }));
    let with = ok(&mut g, "find_recruits", json!({ "filters": f, "unseen": true }));
    assert_eq!(
        with["count"].as_u64().unwrap(),
        left["count"].as_u64().unwrap() + left["unseen"].as_u64().unwrap(),
        "not seen: left out plus kept is everyone"
    );
    for r in left["rows"].as_array().unwrap() {
        assert!(r["read"].is_array(), "a kid kept by a read filter has a read: {r}");
    }
    // "And" and "or": (5 stars or 4 stars) and in-state.
    let f = json!([
        { "k": "cstars", "op": "is", "v": 5 },
        { "k": "cstars", "op": "is", "v": 4, "or": true },
        { "k": "instate", "op": "is", "v": "yes" },
    ]);
    let both = ok(&mut g, "find_recruits", json!({ "filters": f }));
    assert!(both["filter_words"].as_str().unwrap().starts_with('('), "{}", both["filter_words"]);
    for r in both["rows"].as_array().unwrap() {
        assert!(matches!(r["stars"].as_u64(), Some(4 | 5)) && r["instate"] == json!(true), "{r}");
    }
    // Sort by any column, both ways; nothing known goes last.
    let by40 = ok(&mut g, "find_recruits", json!({ "sort": "t40", "filters": [] }));
    let t40: Vec<f64> = by40["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["tests"]["t40"].as_str().and_then(|s| s.parse().ok()))
        .collect();
    assert!(t40.windows(2).all(|w| w[0] <= w[1]), "fastest first: {t40:?}");
    // The chart and Compare.
    let ids: Vec<u64> = by40["rows"].as_array().unwrap().iter().take(3).filter_map(|r| r["id"].as_u64()).collect();
    let c = ok(&mut g, "find_recruits", json!({ "filters": [], "x": "t40", "y": "wt_v", "compare": ids }));
    assert!(c["chart"]["points"].is_array());
    assert_eq!(c["compare"]["players"].as_array().unwrap().len(), ids.len().min(5));
    // The offers drawer from a row.
    if let Some(r) = by40["rows"].as_array().unwrap().iter().find(|r| r["offers"].as_u64().unwrap_or(0) > 0) {
        let o = ok(&mut g, "find_offers", json!({ "id": r["id"] }));
        assert_eq!(o["offers"].as_array().unwrap().len() as u64, r["offers"].as_u64().unwrap());
    }

    // A saved search: what it finds now counts as told; it is listed.
    let spec = page["spec"].clone();
    let s = ok(&mut g, "find_save", json!({ "name": "Home state", "spec": spec }));
    let saved = s["saved"].as_array().unwrap();
    assert_eq!(saved.len(), 1);
    let k = saved[0]["k"].as_u64().unwrap();
    // A services update and a camp pass: a mail may come; the run must hold.
    ok(&mut g, "continue", json!({ "max_days": 60 }));
    let again = ok(&mut g, "find_recruits", json!({ "saved": k }));
    assert_eq!(again["saved"][0]["new"], json!(0), "opening a saved search clears what was new");
    ok(&mut g, "find_forget", json!({ "k": k }));

    // The recruiting map (board 7): your campus, your board at its
    // hometowns, your class against its places.
    let map = ok(&mut g, "recruit_map", json!({}));
    assert_eq!(map["campus"]["at"].as_array().map(Vec::len), Some(2));
    assert_eq!(map["states"].as_array().unwrap().len(), 51);
    for d in map["dots"].as_array().unwrap() {
        assert!(matches!(d["k"].as_str(), Some("c" | "g" | "b" | "n")), "{d}");
    }
    let c = &map["class"];
    assert_eq!(
        c["open"].as_u64().unwrap(),
        c["slots"].as_u64().unwrap().saturating_sub(c["commits"].as_u64().unwrap())
    );
    // A recruit of your own on the board, with your offer: a world this
    // small may leave the coordinator's board empty in March.
    let rows = page["rows"].as_array().unwrap();
    let pick = rows.iter().find(|r| r["offers"].as_u64().unwrap_or(0) > 0).unwrap_or(&rows[0])["id"].clone();
    ok(&mut g, "recruit_add", json!({ "id": pick }));
    let _ = g.call("recruit_offer", &json!({ "id": pick }));
    // The race (board 8): the log keeps each followed recruit's weeks, at
    // most 400 of them; his interest stays on 0-100 and his reasons'
    // weights are your staff's ranges on 1-5.
    ok(&mut g, "holiday", json!({ "days": 21 }));
    let w = g.world.as_ref().unwrap();
    assert!(w.front_office.race.kids.len() <= 400);
    assert!(w.front_office.race.kids.iter().any(|k| k.weeks.len() >= 2), "the weeks give a followed recruit his line");
    let id = w.front_office.race.kids.iter().max_by_key(|k| k.weeks.len()).map(|k| k.id).unwrap();
    let (race, ms) = timed(&mut g, "recruit_race", json!({ "id": id }));
    assert!(race["followed"].as_bool().unwrap());
    for s in race["schools"].as_array().unwrap() {
        for p in s["pts"].as_array().unwrap() {
            assert!((0..=100).contains(&p[1].as_i64().unwrap()), "{s}");
        }
    }
    assert_eq!(race["reasons"].as_array().unwrap().len(), 7);
    for r in race["reasons"].as_array().unwrap() {
        let (lo, hi) = (r["weight"][0].as_u64().unwrap(), r["weight"][1].as_u64().unwrap());
        assert!((1..=hi).contains(&lo) && hi <= 5, "{r}");
    }
    println!("a race in {ms} ms");

    // Nothing hidden on the race: each school's line today is your staff's
    // read, not his true interest; his ties score only what anyone can see
    // (steps of 5); who decides for him, and when, wait for a contact or
    // an announcement.
    let followed: Vec<u32> = g.world.as_ref().unwrap().front_office.race.kids.iter().map(|k| k.id).take(40).collect();
    let (mut same, mut all) = (0usize, 0usize);
    for kid in followed {
        let page = ok(&mut g, "recruit_race", json!({ "id": kid }));
        let w = g.world.as_ref().unwrap();
        let i = w.people.binary_search_by_key(&kid, |p| p.player.id).unwrap();
        let p = &w.people[i];
        let base = race::of(w, kid).unwrap().base;
        for (j, a) in desk::race_at(w, i).into_iter().take(race::SCHOOLS) {
            let truth = race::interest(base, a) as i64;
            let shown = page["schools"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["id"] == json!(j))
                .and_then(|s| s["now"].as_i64());
            if let Some(v) = shown
                && (1..=99).contains(&truth)
            {
                all += 1;
                same += usize::from(v == truth);
            }
        }
        for b in page["reasons"][6]["by"].as_array().unwrap() {
            let score = b["score"].as_f64().unwrap();
            assert!(score % 5.0 == 0.0 && (0.0..=100.0).contains(&score), "his ties: {b}");
        }
        let talked = w.scouting.dept.looks.get(&kid).is_some_and(|v| {
            v.iter().any(|l| {
                matches!(l.kind, Kind::Call | Kind::HomeVisit | Kind::Official | Kind::Unofficial | Kind::Camp)
            })
        });
        let listed = w.scouting.rankings.tops.iter().any(|t| t.id == kid);
        if !talked {
            assert_eq!(page["listens"], json!("Not known yet"), "who decides waits for a contact");
        }
        if !talked && !listed && p.cal.commit.is_none() && p.cal.signed.is_none() {
            assert!(page["decides"].is_null(), "when he decides waits for a signal: {}", page["decides"]);
        }
    }
    // A read lands on his true number now and then by chance (about one
    // point in seven); the truth itself would match every time.
    assert!(
        all > 0 && (all < 2 || same < all) && (all < 12 || same * 3 < all),
        "the race shows reads: {same} of {all} points equal his true interest"
    );
    // An announced top list is in a neutral order, never his.
    let w = g.world.as_ref().unwrap();
    for t in &w.scouting.rankings.tops {
        assert!(t.schools.windows(2).all(|p| p[0] < p[1]), "{:?}", t.schools);
    }
    // The offers drawer: his announced list first, then by when they
    // offered, never by how he feels.
    let kid = w.front_office.race.kids.iter().map(|k| k.id).find(|&id| {
        let i = w.people.binary_search_by_key(&id, |p| p.player.id).unwrap();
        w.people[i].cal.offers.len() >= 3
    });
    if let Some(kid) = kid {
        let o = ok(&mut g, "find_offers", json!({ "id": kid }));
        let rows = o["offers"].as_array().unwrap();
        let listed: Vec<bool> = rows.iter().map(|r| r["listed"].as_bool().unwrap()).collect();
        assert!(listed.windows(2).all(|p| p[0] >= p[1]), "his list first: {listed:?}");
    }

    // Speed: every search reads compact reads, not files.
    let (_, ms) = timed(
        &mut g,
        "find_recruits",
        json!({ "filters": [{ "k": "attr:speed", "op": "ge", "v": 60 }], "unseen": true }),
    );
    assert!(ms < 3000, "a search took {ms} ms");

    // An NFL club's Find prospects.
    ok(&mut g, "load", json!({ "path": fresh }));
    let job = ok(&mut g, "jobs", json!({ "level": "nfl" }))["jobs"][0]["ref"].clone();
    ok(
        &mut g,
        "start_career",
        json!({ "name": "Test GM", "age": 45, "background": "nfl_assistant", "job": job, "seed": 3 }),
    );
    let p = ok(&mut g, "find_prospects", json!({}));
    assert_eq!(p["seat"], json!("nfl"));
    assert!(p["catalog"].as_array().unwrap().iter().any(|g| g["name"] == json!("Public desks")));
    let all = ok(&mut g, "find_prospects", json!({ "filters": [], "unseen": true }));
    assert!(all["count"].as_u64().unwrap() >= p["count"].as_u64().unwrap());
    ok(&mut g, "find_prospects", json!({ "class": p["classes"][1], "filters": [] }));

    // A high school coach's Colleges calling.
    ok(&mut g, "load", json!({ "path": fresh }));
    let hs = ok(&mut g, "jobs", json!({ "level": "hs" }))["jobs"][0]["ref"].clone();
    ok(
        &mut g,
        "start_career",
        json!({ "name": "Test Coach", "age": 35, "background": "hs_coach", "job": hs, "seed": 5 }),
    );
    let side = ok(&mut g, "colleges_calling", json!({}));
    assert!(!side["players"].as_array().unwrap().is_empty());
    for l in side["lines"].as_array().unwrap() {
        assert!(l["caller"]["name"].as_str().is_some_and(|s| !s.is_empty()), "{l}");
        assert!(!l["about"].as_array().unwrap().is_empty());
        assert!(l["at"].is_array() && l["mi"].is_number(), "a calling college on the map: {l}");
    }
    assert_eq!(side["home"]["at"].as_array().map(Vec::len), Some(2));
    // His race as his high school coach sees it.
    if let Some(p) = side["players"].as_array().unwrap().iter().find(|p| p["offers"].as_u64().unwrap_or(0) > 0) {
        let race = ok(&mut g, "recruit_race", json!({ "id": p["id"] }));
        assert_eq!(race["seat"], json!("hs"));
    }
    let _ = std::fs::remove_dir_all(dir);
}
