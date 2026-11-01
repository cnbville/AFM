//! A career on a small world (section 8): build a world, take a college
//! job, set a depth chart and a training focus, play into the season and
//! read every screen; then the same world from a high school job.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn a_college_career_plays_through_its_first_games() {
    let t = std::time::Instant::now();
    let dir = std::env::temp_dir().join(format!("afm-career-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 7, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    println!("world built in {:.0}s", t.elapsed().as_secs_f64());
    let fresh = ok(&mut g, "save", json!({ "name": "fresh world" }))["saved"].as_str().unwrap().to_string();
    let setup = ok(&mut g, "setup", json!({}));
    assert!(setup["backgrounds"].as_array().unwrap().len() >= 10);
    assert_eq!(setup["ratings"].as_array().unwrap().len(), 19);
    // The roads' totals, locked strengths and pool together (coaching C6).
    let totals: Vec<f64> =
        setup["backgrounds"].as_array().unwrap().iter().map(|b| b["pool"].as_f64().unwrap()).collect();
    assert_eq!(totals, [245.0, 258.0, 271.0, 255.0, 234.0, 239.0, 261.0, 274.0, 276.0, 227.0, 230.0]);
    assert!(setup["marks"]["college"]["groups"].as_array().is_some_and(|g| g.len() == 8), "{}", setup["marks"]);
    // The new-career pages' data: towns, coaches on your road, the job file.
    assert!(!ok(&mut g, "towns", json!({ "state": "TX" }))["towns"].as_array().unwrap().is_empty());
    let bg = setup["backgrounds"].as_array().unwrap().iter().find(|b| b["key"] == "college_assistant").unwrap();
    // Two points moved from negotiating (the last rating) to play calling.
    let mut ratings: Vec<f64> = bg["ratings"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
    let pc = setup["ratings"].as_array().unwrap().iter().position(|r| r["key"] == "play_calling").unwrap();
    ratings[pc] += 2.0;
    ratings[18] -= 2.0;
    let like = ok(&mut g, "similar", json!({ "background": "college_assistant", "ratings": ratings }));
    assert!(!like["coaches"].as_array().unwrap().is_empty(), "{like}");
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let file = &jobs["jobs"][0];
    for k in ["line", "record", "stadium", "nil", "coach", "ad", "expectation", "grades", "calendar"] {
        assert!(!file[k].is_null(), "the job file has {k}: {file}");
    }
    let job = file["ref"].clone();
    ok(
        &mut g,
        "start_career",
        json!({
            "name": "Test Coach", "age": 41, "background": "college_assistant", "job": job, "seed": 99,
            "ratings": ratings, "home_state": "TX", "calls": "defense",
            "profile": { "born": "1985-03-14", "hometown": "Austin", "alma_mater": "Texas Tech", "children": [{ "name": "Jake", "born": 2012 }] },
        }),
    );
    let home = ok(&mut g, "home", json!({}));
    assert!(home["team"]["name"].is_string(), "{home}");
    let you = ok(&mut g, "you", json!({}));
    assert_eq!(you["extra"]["play_calling"].as_f64(), Some(ratings[pc]), "{you}");
    assert_eq!(you["extra"]["negotiating"].as_f64(), Some(ratings[18]), "{you}");
    assert_eq!(you["extra"]["calls"], "The defense", "{you}");
    assert_eq!(you["profile"]["hometown"], "Austin");
    assert_eq!(you["profile"]["children"][0][0], "Jake");
    // The save's card: the world's story and your team, without loading it.
    let card = ok(&mut g, "card", json!({}));
    assert!(!card["world"]["story"].as_array().unwrap().is_empty(), "{card}");
    assert!(card["seat"]["team"]["name"].is_string(), "{card}");
    // The recruiting map: every state, its recruiter, and an area you set.
    let map = ok(&mut g, "recruit_map", json!({}));
    let states = map["states"].as_array().unwrap();
    assert_eq!(states.len(), 51);
    assert!(states.iter().all(|s| s["coach"].is_object()), "every state has a recruiter");
    if let Some(c) = map["staff"].as_array().and_then(|a| a.last()) {
        let set = ok(&mut g, "recruit_area", json!({ "state": "TX", "coach": c["id"] }));
        let tx = set["states"].as_array().unwrap().iter().find(|s| s["code"] == "TX").unwrap().clone();
        assert_eq!((tx["coach"]["id"].clone(), tx["set"].clone()), (c["id"].clone(), json!(true)));
        let back = ok(&mut g, "recruit_area", json!({ "state": "TX" }));
        assert!(back["states"].as_array().unwrap().iter().any(|s| s["code"] == "TX" && s["set"] == false));
    }
    assert!(g.call("recruit_area", &json!({ "state": "TX", "coach": 999_999 })).is_err());
    let roster = ok(&mut g, "roster", json!({}));
    let rows = roster["rows"].as_array().unwrap();
    assert!(rows.len() > 40, "{} players", rows.len());
    // Your own players come as ranges, never one number.
    assert!(rows[0]["ovr"].is_array());
    let id = rows[0]["id"].as_u64().unwrap();
    let page = ok(&mut g, "player", json!({ "id": id }));
    assert!(
        page["groups"].as_array().unwrap().iter().all(|g| g["attrs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a["read"].is_array()))
    );
    ok(&mut g, "note_add", json!({ "id": id, "text": "Looks quicker than his film." }));
    assert_eq!(ok(&mut g, "player", json!({ "id": id }))["notes"].as_array().unwrap().len(), 1);
    // Your depth chart: put the last dressed quarterback first (one who is
    // hurt doesn't dress, whatever your order says).
    let depth = ok(&mut g, "depth", json!({}));
    let listed = depth["positions"][0]["listed"].as_u64().unwrap() as usize;
    let qbs: Vec<u64> = depth["positions"][0]["players"]
        .as_array()
        .unwrap()
        .iter()
        .take(listed)
        .map(|p| p["id"].as_u64().unwrap())
        .collect();
    if qbs.len() > 1 {
        let mut order = qbs.clone();
        order.rotate_right(1);
        let d = ok(&mut g, "set_depth", json!({ "pos": "QB", "ids": order }));
        assert_eq!(d["positions"][0]["players"][0]["id"].as_u64().unwrap(), order[0]);
    }
    // Training: a main focus for a player.
    let tr = ok(&mut g, "training", json!({}));
    let key = tr["rows"][0]["options"][0]["key"].as_str().unwrap().to_string();
    let tid = tr["rows"][0]["id"].as_u64().unwrap();
    ok(&mut g, "set_focus", json!({ "id": tid, "key": key }));
    assert_eq!(ok(&mut g, "training", json!({}))["rows"][0]["focus"].as_str(), Some(key.as_str()));
    // Play into the season: Continue until your team has a kept game. It
    // stops the morning of the game, before kickoff, and again once it is
    // played, naming it.
    let mut steps = 0;
    let mut stopped_on_game = false;
    let mut game_morning = false;
    while ok(&mut g, "games", json!({}))["games"].as_array().unwrap().is_empty() && steps < 400 {
        let r = ok(&mut g, "continue", json!({ "max_days": 300 }));
        stopped_on_game |= r["game"].is_object();
        game_morning |= r["morning"] == json!(true)
            && r["why"][0]["text"].as_str().is_some_and(|t| t.starts_with("Game day"))
            && ok(&mut g, "games", json!({}))["games"].as_array().unwrap().is_empty();
        steps += 1;
    }
    assert!(game_morning, "Continue stops the morning of your game");
    assert!(stopped_on_game, "Continue stops on your game day");
    let game = ok(&mut g, "game", json!({ "index": 0 }));
    assert!(game["plays"].as_array().unwrap().len() > 80, "a whole game's plays");
    for cmd in ["schedule", "standings", "staff", "inbox", "news", "calendar", "you", "status"] {
        ok(&mut g, cmd, json!({}));
    }
    // The calendar (update 3, lane 2): whole weeks, the year strip, the
    // picked day and your first game in its cell.
    let cal = ok(&mut g, "calendar_board", json!({}));
    let days = cal["days"].as_array().unwrap();
    assert_eq!(days.len() % 7, 0);
    assert_eq!(days.len() / 7, cal["weeks"].as_array().unwrap().len());
    assert!(cal["strip"]["phases"].as_array().is_some_and(|p| !p.is_empty()), "{}", cal["strip"]);
    let before = ok(&mut g, "calendar_board", json!({ "month": cal["prev"] }));
    let played = |d: &Value| d["game"] == true && d["res"].as_str().is_some_and(|r| r.starts_with(['W', 'L', 'T']));
    assert!(days.iter().chain(before["days"].as_array().unwrap()).any(played), "a played game shows its result");
    assert!(ok(&mut g, "calendar_board", json!({ "month": cal["next"] }))["title"] != cal["title"]);
    assert!(!ok(&mut g, "search", json!({ "q": "test coach" }))["coaches"].as_array().unwrap().is_empty());
    // Every name a line names is a link (L2), and the parts rebuild the line.
    let parts_of = |x: &Value| -> Vec<Value> {
        x["items"].as_array().unwrap().iter().flat_map(|i| i["parts"].as_array().cloned().unwrap_or_default()).collect()
    };
    let news = ok(&mut g, "news", json!({ "limit": 300 }));
    for item in news["items"].as_array().unwrap() {
        let joined: String = item["parts"].as_array().unwrap().iter().map(|p| p["t"].as_str().unwrap()).collect();
        assert_eq!(joined, item["text"].as_str().unwrap());
    }
    let links = parts_of(&news).into_iter().filter(|p| p["go"].is_string()).collect::<Vec<_>>();
    assert!(links.iter().any(|p| p["go"] == "player"), "players are linked in the news");
    assert!(links.iter().any(|p| p["go"] == "team"), "teams are linked in the news");
    let inbox = ok(&mut g, "inbox", json!({ "days": 120 }));
    assert!(parts_of(&inbox).iter().any(|p| p["go"] == "game"), "your result opens its game page");
    // A1: the player page counts his age from his birthday.
    let page = ok(&mut g, "player", json!({ "id": id }));
    assert!(page["born"].is_string(), "{}", page["born"]);
    assert!(page["on_camera"].is_string());
    // The Data section (update 3, lane L20) on the season in progress: the
    // first week's single games are in the books, your team's own book has
    // them, and every Data page answers at each level (the season's closes
    // are checked on the shared world in afm-tools' world test).
    {
        use afm_engine::world::marks::{GAME, Tier, team_list};
        use afm_engine::world::records::{Book, Scope};
        let w = g.world.as_ref().unwrap();
        let games = w
            .stats
            .books
            .iter()
            .filter(|(k, l)| matches!(k.0, Scope::Division(_)) && matches!(k.1, Book::Game(_)) && !l.is_empty())
            .count();
        assert!(games > 0, "college single-game lists");
        let mine = match ok(&mut g, "status", json!({}))["mine"]["ref"]["id"].as_u64() {
            Some(p) => p as usize,
            None => panic!("a college seat"),
        };
        let w = g.world.as_ref().unwrap();
        assert!(
            !team_list(w.marks.team_book(Tier::College, mine), GAME, 7).is_empty(),
            "your team's single-game tackles"
        );
    }
    assert_eq!(ok(&mut g, "data_sources", json!({}))["seat"], "college");
    for level in ["college", "nfl", "hs"] {
        for tab in ["career", "season", "game", "rates", "leaders", "team"] {
            let x = ok(&mut g, "stat_history", json!({ "level": level, "tab": tab }));
            assert_eq!(x["level"], level, "{x}");
            assert!(x["chasing"].is_array() && x["fell"].is_array());
        }
        for group in ["QB", "WR", "OL", "CB", "K"] {
            let a = ok(&mut g, "analytics", json!({ "level": level, "group": group }));
            assert!(a["columns"].is_array() && a["rows"].is_array(), "{level} {group}: {a}");
        }
    }
    let games = ok(&mut g, "stat_history", json!({ "tab": "game" }));
    let row = &games["lists"][0]["rows"][0];
    assert!(row["game"]["score"].is_string() && row["game"]["opp"].is_object(), "a single game opens its game: {row}");
    let book = ok(&mut g, "stat_history", json!({ "tab": "team" }));
    assert!(book["team_book"]["lists"].as_array().is_some_and(|l| !l.is_empty()), "your team's book");
    let qbs = ok(&mut g, "analytics", json!({ "level": "college", "group": "QB", "min": 0 }));
    assert!(qbs["charted"] == true && qbs["total"].as_u64().unwrap() > 0, "your division's charted passers: {qbs}");
    let pct = qbs["rows"][0]["pct"].as_array().unwrap();
    assert!(pct.iter().all(|p| p.is_null() || (0..=100).contains(&p.as_i64().unwrap())));
    let card = ok(&mut g, "analytics_card", json!({ "id": qbs["rows"][0]["id"] }));
    assert!(card["metrics"].as_array().is_some_and(|m| !m.is_empty()), "{card}");
    ok(&mut g, "analytics_card", json!({ "id": id }));
    for tab in ["conferences", "states", "award"] {
        assert!(ok(&mut g, "champions_more", json!({ "tab": tab }))["rows"].is_array());
    }
    ok(&mut g, "team_book", json!({ "kind": "nfl", "id": 0 }));
    // Save and load: the loaded career plays on.
    let saved = ok(&mut g, "save", json!({ "name": "test career" }));
    let path = saved["saved"].as_str().unwrap().to_string();
    ok(&mut g, "load", json!({ "path": path }));
    assert_eq!(ok(&mut g, "player", json!({ "id": id }))["notes"].as_array().unwrap().len(), 1);
    ok(&mut g, "continue", json!({ "max_days": 10 }));
    let lib = ok(&mut g, "library", json!({}));
    assert_eq!(lib["saves"].as_array().unwrap().len(), 2);
    assert!(lib["saves"].as_array().unwrap().iter().any(|s| s["card"]["seat"].is_object()), "{lib}");
    // Rename and delete a save, in the saves folder only.
    let renamed = ok(&mut g, "rename_save", json!({ "path": path, "name": "renamed career" }))["path"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(renamed.ends_with("renamed career.afmsave"), "{renamed}");
    assert!(g.call("delete_save", &json!({ "path": "/etc/hosts" })).is_err());
    ok(&mut g, "delete_save", json!({ "path": renamed }));
    assert_eq!(ok(&mut g, "library", json!({}))["saves"].as_array().unwrap().len(), 1);
    println!("college career in {:.0}s", t.elapsed().as_secs_f64());
    // The same world from a high school job: a staff of teacher-coaches,
    // your games on the engine with their pages, the schedule and standings.
    ok(&mut g, "load", json!({ "path": fresh }));
    let hs = ok(&mut g, "jobs", json!({ "level": "hs" }))["jobs"][0]["ref"].clone();
    ok(
        &mut g,
        "start_career",
        json!({ "name": "Test Coach", "age": 35, "background": "hs_coach", "job": hs, "seed": 5 }),
    );
    assert!(
        ok(&mut g, "staff", json!({}))["rows"].as_array().unwrap().len() >= 4,
        "head coach and at least three assistants"
    );
    let mut steps = 0;
    while ok(&mut g, "games", json!({}))["games"].as_array().unwrap().is_empty() && steps < 400 {
        ok(&mut g, "continue", json!({ "max_days": 300 }));
        steps += 1;
    }
    let game = ok(&mut g, "game", json!({ "index": 0 }));
    assert!(game["plays"].as_array().unwrap().len() > 60, "a whole high school game's plays");
    let sched = ok(&mut g, "schedule", json!({}));
    assert!(sched["rows"].as_array().unwrap().iter().any(|x| x["result"]["us"].is_number()), "{sched}");
    assert!(!ok(&mut g, "standings", json!({}))["groups"].as_array().unwrap().is_empty());
    // Every other school's games run on the engine too, not only yours:
    // each game played is an engine game or a forfeit, never the stand-in,
    // and the engine's team totals cover them all.
    let log = &g.world.as_ref().unwrap().log;
    assert!(log.hs_engine_games > 100, "{} high school engine games", log.hs_engine_games);
    assert_eq!(log.hs_engine_games + log.hs_forfeits, log.hs_games, "a high school game on the stand-in");
    assert_eq!(log.hs_totals.games, 2 * log.hs_engine_games);
    let schools = g.world.as_ref().unwrap().hs_schools_with_lines();
    assert!(schools.len() > 100, "{} schools with players' engine lines this season", schools.len());
    println!("career test in {:.0}s", t.elapsed().as_secs_f64());
    let _ = std::fs::remove_dir_all(dir);
}
