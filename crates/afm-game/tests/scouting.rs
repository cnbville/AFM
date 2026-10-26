//! Scouting and the draft from your seat (section 7, final build lane 7):
//! an NFL club's department, scouting file and draft board through the
//! pre-draft season, the live draft in the war room with a trade and the
//! undrafted rush; then a college staff's recruit and portal boards and a
//! high school coach's calls to colleges.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn an_nfl_war_room_runs_the_draft() {
    let t = std::time::Instant::now();
    let dir = std::env::temp_dir().join(format!("afm-scouting-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 7, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let fresh = ok(&mut g, "save", json!({ "name": "fresh" }))["saved"].as_str().unwrap().to_string();
    // A small world's draft keeps only a sample of the full order: take a
    // job at the club with the earliest pick this spring. The later places
    // shift when March's compensatory picks join the order; the first
    // rounds' places don't.
    let jobs = ok(&mut g, "jobs", json!({ "level": "nfl" }))["jobs"].as_array().unwrap().clone();
    let start = |g: &mut Game, job: &Value| {
        ok(g, "load", json!({ "path": fresh }));
        ok(
            g,
            "start_career",
            json!({ "name": "Test GM", "age": 45, "background": "nfl_assistant", "job": job, "seed": 3 }),
        );
    };
    let mut best: Option<(u64, Value)> = None;
    for j in &jobs {
        start(&mut g, &j["ref"]);
        let picks = ok(&mut g, "draft_board", json!({}))["picks"].clone();
        if let Some(pick) = picks[0]["pick"].as_u64()
            && best.as_ref().is_none_or(|b| pick < b.0)
        {
            best = Some((pick, j["ref"].clone()));
            if picks[0]["round"].as_u64().is_some_and(|r| r <= 2) {
                break;
            }
        }
    }
    let Some((_, job)) = best else { panic!("no open NFL job holds a pick in {} jobs", jobs.len()) };
    start(&mut g, &job);
    // The first morning (the day after the Super Bowl, before it runs)
    // already reads this April's order.
    let first_morning = ok(&mut g, "draft_board", json!({}))["picks"].clone();
    ok(&mut g, "continue", json!({ "max_days": 1 }));
    assert_eq!(ok(&mut g, "draft_board", json!({}))["picks"], first_morning, "the first morning's draft order");
    // The department and the public's word.
    let dept = ok(&mut g, "scouting", json!({}));
    assert!(dept["people"].as_array().unwrap().len() >= 8, "{}", dept["people"]);
    let who = dept["people"][1]["code"].as_u64().unwrap();
    ok(&mut g, "scout_assign", json!({ "who": who, "target": "group1" }));
    ok(&mut g, "scout_instruct", json!({ "key": "small", "on": true }));
    let dept = ok(&mut g, "scouting", json!({}));
    assert!(dept["people"][1]["assignment"].is_string());
    assert!(dept["instructions"].as_array().unwrap().iter().any(|i| i["on"] == json!(true)));
    // The board: the class in your order, tiers, the public's rank.
    let board = ok(&mut g, "draft_board", json!({}));
    let rows = board["rows"].as_array().unwrap();
    assert!(rows.len() > 50, "{} on the board", rows.len());
    let top = rows[0]["id"].as_u64().unwrap();
    let tenth = rows[9]["id"].as_u64().unwrap();
    // Move the tenth to the top and put a player on the visit list.
    let board = ok(&mut g, "board_move", json!({ "id": tenth, "to": 1 }));
    assert_eq!(board["rows"][0]["id"].as_u64(), Some(tenth));
    ok(&mut g, "board_list", json!({ "id": top, "list": 0, "on": true }));
    ok(&mut g, "follow", json!({ "id": top, "why": "Watch his pro day" }));
    // His file: ranges, looks with who took them, character in words.
    let file = ok(&mut g, "scout_file", json!({ "id": top }));
    assert!(file["followed"].as_bool().unwrap());
    // One list: following from the file puts him on the core My players.
    let mine = ok(&mut g, "my_players", json!({}));
    assert!(mine["rows"].as_array().unwrap().iter().any(|r| r["id"].as_u64() == Some(top)), "{}", mine["rows"]);
    if file["seen"].as_bool().unwrap() {
        assert!(file["summary"]["role"].is_string(), "{}", file["summary"]);
        assert!(!file["looks"].as_array().unwrap().is_empty());
    }
    // Ranges come once a look's report is written (a scout writes his
    // all-star week up days later, so a file of only those looks lists them
    // and reads nothing yet): some file near the top of the board reads them.
    let any_range = rows.iter().take(10).any(|r| {
        let f = ok(&mut g, "scout_file", json!({ "id": r["id"] }));
        f["groups"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["attrs"].as_array().unwrap().iter().any(|a| a["read"].is_array()))
    });
    assert!(any_range, "some ratings read near the top of the board");
    // To draft day: Continue stops when the war room opens.
    let mut steps = 0;
    while !ok(&mut g, "draft_live", json!({}))["live"].as_bool().unwrap() && steps < 300 {
        ok(&mut g, "continue", json!({ "max_days": 60 }));
        steps += 1;
    }
    let room = ok(&mut g, "draft_live", json!({}));
    assert!(room["live"].as_bool().unwrap(), "the war room opened ({steps} steps)");
    println!("to the draft in {:.0}s", t.elapsed().as_secs_f64());
    // Continue doesn't move past an open war room, and sends you to it.
    let before = ok(&mut g, "status", json!({}));
    let r = ok(&mut g, "continue", json!({ "max_days": 5 }));
    assert_eq!(r["studio"]["screen"], json!("warroom"));
    assert_eq!(ok(&mut g, "status", json!({}))["date"], before["date"]);
    // The pre-draft page and the public's boards are there.
    ok(&mut g, "predraft", json!({}));
    ok(&mut g, "public_boards", json!({}));
    // Our first pick: on the clock (the AI picks ran to us).
    assert!(
        room["on_clock"].as_bool().unwrap(),
        "next {} slots {} upcoming {} feed {}",
        room["next"],
        room["my_slots"],
        room["upcoming"],
        room["feed"]
    );
    assert!(room["staff_pick"].is_object());
    let first_pick = room["next"].as_u64().unwrap();
    let slots = room["slots"].as_u64().unwrap() as usize;
    let choice = room["board"][0]["id"].as_u64().unwrap();
    let room = ok(&mut g, "draft_pick", json!({ "id": choice }));
    assert_eq!(room["revealed"]["id"].as_u64(), Some(choice));
    assert!(room["picks_made"].as_u64().unwrap() >= first_pick);
    // On to the next one; take an offer if one comes, else let the staff pick.
    let mut room = ok(&mut g, "draft_go", json!({ "until": "yours" }));
    let mut traded = false;
    let mut guard = 0;
    while room["live"].as_bool().unwrap() && !room["rush"].as_bool().unwrap() && guard < 40 {
        if room["on_clock"].as_bool().unwrap() {
            if !traded && !room["offers"].as_array().unwrap().is_empty() {
                room = ok(&mut g, "draft_accept", json!({ "offer": 0 }));
                traded = true;
            } else {
                room = ok(&mut g, "draft_auto", json!({}));
            }
        } else {
            room = ok(&mut g, "draft_go", json!({ "until": "yours" }));
        }
        guard += 1;
    }
    assert!(room["rush"].as_bool().unwrap(), "every pick is in");
    let rec = ok(&mut g, "draft_rush", json!({}));
    assert!(rec["mine"].as_array().unwrap().iter().any(|p| p["id"].as_u64() == Some(choice)), "{}", rec["mine"]);
    // Every pick is in the record (a small world's draft has only a few).
    assert_eq!(rec["all"].as_array().unwrap().len(), slots, "{}", rec["all"]);
    assert!(!ok(&mut g, "draft_live", json!({}))["live"].as_bool().unwrap());
    // The world carries on after the draft.
    ok(&mut g, "continue", json!({ "max_days": 30 }));
    ok(&mut g, "pro_scouting", json!({}));
    ok(&mut g, "lookback", json!({}));
    // Save and load keep the department.
    let saved = ok(&mut g, "save", json!({ "name": "after draft" }))["saved"].as_str().unwrap().to_string();
    ok(&mut g, "load", json!({ "path": saved }));
    assert!(ok(&mut g, "draft_record", json!({}))["mine"].is_array());
    println!("NFL draft in {:.0}s", t.elapsed().as_secs_f64());

    // A college staff: the recruit board with reads, the portal, a file.
    ok(&mut g, "load", json!({ "path": fresh }));
    let job = ok(&mut g, "jobs", json!({ "level": "college" }))["jobs"][0]["ref"].clone();
    ok(
        &mut g,
        "start_career",
        json!({ "name": "Test Coach", "age": 41, "background": "college_assistant", "job": job, "seed": 9 }),
    );
    ok(&mut g, "holiday", json!({ "days": 40 }));
    let rec = ok(&mut g, "recruit_scouting", json!({}));
    let rows = rec["rows"].as_array().unwrap();
    assert!(!rows.is_empty(), "the recruiting board");
    let rid = rows[0]["id"].as_u64().unwrap();
    ok(&mut g, "scout_ask", json!({ "id": rid, "kind": "Film study" }));
    ok(&mut g, "scout_file", json!({ "id": rid }));
    ok(&mut g, "recruit_scouting", json!({ "list": "portal" }));
    let dept = ok(&mut g, "scouting", json!({}));
    assert!(dept["people"].as_array().unwrap().iter().any(|p| p["job"] == json!("Head coach")));
    // Your own player's file reads narrow from practice.
    let roster = ok(&mut g, "roster", json!({}));
    let mine = roster["rows"][0]["id"].as_u64().unwrap();
    let f = ok(&mut g, "scout_file", json!({ "id": mine }));
    assert!(f["mine"].as_bool().unwrap() && f["seen"].as_bool().unwrap(), "{}", f["looks_n"]);

    // A high school coach: your players, the colleges calling, a call made.
    ok(&mut g, "load", json!({ "path": fresh }));
    let hs = ok(&mut g, "jobs", json!({ "level": "hs" }))["jobs"][0]["ref"].clone();
    ok(
        &mut g,
        "start_career",
        json!({ "name": "Test Coach", "age": 35, "background": "hs_coach", "job": hs, "seed": 5 }),
    );
    let side = ok(&mut g, "hs_side", json!({}));
    let players = side["players"].as_array().unwrap();
    assert!(!players.is_empty());
    // A player still unsigned (seniors sign in the winter).
    let pid = players.iter().find(|p| p["signed"].is_null()).expect("an unsigned player")["id"].as_u64().unwrap();
    let r = ok(&mut g, "promote", json!({ "id": pid, "strength": 1 }));
    assert_eq!(r["hs"]["calls"].as_array().unwrap().len(), 1);
    println!("scouting test in {:.0}s", t.elapsed().as_secs_f64());
    let _ = std::fs::remove_dir_all(dir);
}
