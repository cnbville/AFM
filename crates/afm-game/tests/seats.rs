//! Every seat on a small world (final build lane 10): an assistant's job
//! (what it controls, the head coach's answer), a career without a job
//! (the market), a high school head job at a big school with its
//! squads, its office, its feeders and its JV games, and a club in another
//! league through a season's first games.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

fn with(mut base: Value, more: Value) -> Value {
    for (k, v) in more.as_object().unwrap() {
        base[k] = v.clone();
    }
    base
}

fn find<'a>(v: &'a Value, key: &str, pred: impl Fn(&Value) -> bool) -> Option<&'a Value> {
    v[key].as_array().and_then(|a| a.iter().find(|x| pred(x)))
}

#[test]
fn every_seat_starts_and_the_school_desk_runs() {
    let t = std::time::Instant::now();
    let dir = std::env::temp_dir().join(format!("afm-seats-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 11, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    println!("world built in {:.0}s", t.elapsed().as_secs_f64());
    let fresh = ok(&mut g, "save", json!({ "name": "fresh" }))["saved"].as_str().unwrap().to_string();
    let colleges = ok(&mut g, "jobs", json!({ "level": "college" }));
    let alma = colleges["jobs"][0]["name"].as_str().unwrap().to_string();
    let base = json!({
        "name": "Test Coach", "age": 44, "background": "college_assistant", "home_state": "TX", "seed": 5,
        "profile": { "born": "1982-05-01", "alma_mater": alma, "college_years": "2000-2004" },
    });

    // An assistant: offensive coordinator on a staff with a head coach.
    let staffs = ok(&mut g, "assistant_jobs", json!({ "level": "college" }));
    let team =
        find(&staffs, "jobs", |j| j["coach"].is_object() && j["roles"].as_array().is_some_and(|r| r.len() == 10))
            .expect("a staff with a head coach")
            .clone();
    let st = ok(
        &mut g,
        "start_career_seat",
        with(base.clone(), json!({ "start": "assistant", "role": "oc", "job": team["ref"] })),
    );
    assert_eq!(st["seat"]["head"], false, "{st}");
    assert_eq!(st["seat"]["role"], "Offensive coordinator");
    assert_eq!(st["mine"]["ref"], team["ref"]);
    let depth = ok(&mut g, "depth", json!({}));
    let pos = |p: &str| find(&depth, "positions", |x| x["pos"] == p).unwrap().clone();
    let (qb, cb) = (pos("QB"), pos("CB"));
    assert_eq!(qb["controls"], true);
    assert_eq!(cb["controls"], false);
    let ids = |p: &Value| p["players"].as_array().unwrap().iter().map(|x| x["id"].clone()).rev().collect::<Vec<_>>();
    // The defense is the head coach's call; the offense is your proposal, answered.
    assert!(g.call("set_depth", &json!({ "pos": "CB", "ids": ids(&cb) })).is_err());
    let r = ok(&mut g, "set_depth", json!({ "pos": "QB", "ids": ids(&qb) }));
    assert!(r["note"].as_str().is_some_and(|n| n.contains("overruled") || n.contains("order")), "{}", r["note"]);
    assert_eq!(ok(&mut g, "card", json!({}))["seat"]["role"], "Offensive coordinator");
    // The market knows your job, and the school desk isn't yours.
    let m = ok(&mut g, "market", json!({}));
    assert_eq!(m["job"]["role"], "Offensive coordinator", "{}", m["job"]);
    assert!(g.call("school_office", &json!({})).is_err());

    // Out of work, looking at high school jobs.
    ok(&mut g, "load", json!({ "path": fresh }));
    let st = ok(&mut g, "start_career_seat", with(base.clone(), json!({ "start": "free", "level": "hs" })));
    assert!(st["mine"].is_null(), "{st}");
    assert_eq!(st["seat"]["out_of_work"], true);
    let m = ok(&mut g, "market", json!({}));
    assert_eq!(m["interest"]["hs_head"], true);
    assert_eq!(m["interest"]["college_head"], false);
    let you = ok(&mut g, "you", json!({}));
    assert!(you["title"].as_str().is_some_and(|t| t.starts_with("Out of coaching")), "{}", you["title"]);
    // The days run without a team.
    ok(&mut g, "continue", json!({ "max_days": 40 }));
    assert!(ok(&mut g, "status", json!({}))["you"].is_string());

    // A high school head job at the biggest public school in Texas, with a
    // stop on a college staff. A world this small keeps only two or three
    // schools in the smallest states (Nevada's are its powerhouses), too few
    // for a season's schedule; Texas keeps dozens.
    ok(&mut g, "load", json!({ "path": fresh }));
    let hs = ok(&mut g, "jobs", json!({ "level": "hs" }));
    let big = hs["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|j| j["private"] != true && j["state"] == "TX")
        .max_by_key(|j| j["enrollment"].as_u64().unwrap_or(0))
        .unwrap()
        .clone();
    let enrollment = big["enrollment"].as_u64().unwrap();
    let stop = colleges["jobs"][3]["ref"].clone();
    let st = ok(
        &mut g,
        "start_career_seat",
        with(
            base.clone(),
            json!({ "start": "head", "job": big["ref"], "stops": [{ "employer": stop, "role": "pc0", "from": 2016, "to": 2020 }] }),
        ),
    );
    assert_eq!(st["seat"]["head"], true);
    let you = ok(&mut g, "you", json!({}));
    assert!(
        you["history"].as_array().is_some_and(|h| h.iter().any(|s| s["from"] == 2016)),
        "the stop is on your record: {}",
        you["history"]
    );
    let office = ok(&mut g, "school_office", json!({}));
    assert_eq!(office["people"].as_array().unwrap().len(), 4, "{}", office["people"]);
    assert!(!office["goals"].as_array().unwrap().is_empty(), "the first meeting sets goals");
    assert!(office["teaching"]["total"].as_f64().unwrap() > 30_000.0, "{}", office["teaching"]);
    let area = ok(&mut g, "school_area", json!({}));
    assert!(!area["feeders"].as_array().unwrap().is_empty());
    let sq = ok(&mut g, "school_squads", json!({}));
    let teams = sq["teams"].as_array().unwrap();
    println!("{} students: {} teams", enrollment, teams.len());
    if enrollment >= 450 {
        assert!(teams.len() >= 2, "a big school fields JV: {}", teams.len());
    }
    let varsity = &teams[0];
    assert_eq!(varsity["key"], "varsity");
    // A senior can't play JV.
    if let Some(sr) = varsity["players"].as_array().unwrap().iter().find(|p| p["grade"] == 12) {
        assert!(g.call("set_squad", &json!({ "id": sr["id"], "squad": "jv" })).is_err());
    }
    // A JV sophomore moves up, and back down with the staff's lists.
    if let Some(jv) = teams.iter().find(|t| t["key"] == "jv")
        && let Some(p) = jv["players"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["can"].as_array().is_some_and(|c| c.contains(&json!("varsity"))))
    {
        let r = ok(&mut g, "set_squad", json!({ "id": p["id"], "squad": "varsity" }));
        let moved =
            r["teams"][0]["players"].as_array().unwrap().iter().any(|x| x["id"] == p["id"] && x["moved"] == true);
        assert!(moved, "{}", r["text"]);
        let r = ok(&mut g, "reset_squads", json!({}));
        assert!(!r["teams"][0]["players"].as_array().unwrap().iter().any(|x| x["id"] == p["id"]));
    }
    // Into the season: Thursday's JV games are played on the engine.
    for _ in 0..200 {
        let st = ok(&mut g, "status", json!({}));
        if st["today"].as_str().is_some_and(|d| d.contains("Oct")) {
            break;
        }
        ok(&mut g, "continue", json!({ "max_days": 60 }));
    }
    let sq = ok(&mut g, "school_squads", json!({}));
    if let Some(jv) = find(&sq, "teams", |t| t["key"] == "jv") {
        let played = jv["games"].as_array().unwrap().iter().filter(|g| g["score"].is_array()).count();
        assert!(played >= 2, "JV games by October: {}", jv["games"]);
    }
    let office = ok(&mut g, "school_office", json!({}));
    assert!(
        office["boosters"]["raised"].as_array().unwrap().iter().any(|r| r["x"].as_f64().unwrap_or(0.0) > 0.0),
        "the boosters raised money: {}",
        office["boosters"]
    );

    // A club in another league (first post-release update): the CFL's
    // clubs are in the job list, and its head coach's season plays on the
    // engine with game mornings, kept games and their film.
    ok(&mut g, "load", json!({ "path": fresh }));
    let other = ok(&mut g, "jobs", json!({ "level": "other" }));
    let club = find(&other, "jobs", |j| j["league"] == "CFL").expect("a CFL club in the job list").clone();
    let st = ok(&mut g, "start_career_seat", with(base.clone(), json!({ "start": "head", "job": club["ref"] })));
    assert_eq!(st["mine"]["ref"]["kind"], "other", "{st}");
    assert_eq!(st["seat"]["level"], "other");
    assert!(
        g.call(
            "start_career_seat",
            &with(base.clone(), json!({ "start": "assistant", "role": "pc0", "job": club["ref"] }))
        )
        .is_err()
    );
    let lg = ok(&mut g, "leagues", json!({}));
    assert_eq!(lg["league"]["short"], "CFL", "your own league first");
    let mut played = 0;
    for _ in 0..60 {
        played = g.world.as_ref().map_or(0, |w| w.seat_games().len());
        if played >= 3 {
            break;
        }
        ok(&mut g, "continue", json!({ "max_days": 60 }));
    }
    assert!(played >= 3, "your club's games are kept: {played}");
    let games = ok(&mut g, "games", json!({}));
    let n = games["games"].as_array().unwrap().len();
    let page = ok(&mut g, "game_page", json!({ "index": n - 1 }));
    assert_eq!(page["level"], "CFL");
    let film = ok(&mut g, "film", json!({ "index": n - 1 }));
    assert!(film["teams"].as_array().unwrap().iter().all(|t| t["ref"]["kind"] == "other"), "{}", film["teams"]);
    ok(&mut g, "film_close", json!({}));
    let hub = ok(&mut g, "gameday", json!({}));
    assert_eq!(hub["has_game"], true, "{hub}");
    assert_eq!(hub["level"], "CFL");
    assert!(hub["tapes"].as_array().is_some_and(|t| !t.is_empty()), "film of your next opponent: {}", hub["tapes"]);
    for s in ["team", "roster", "schedule", "staff", "depth", "news", "inbox"] {
        ok(&mut g, s, json!({}));
    }
    println!("done in {:.0}s", t.elapsed().as_secs_f64());
    let _ = std::fs::remove_dir_all(dir);
}
