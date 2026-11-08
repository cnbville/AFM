//! Lane 9, the screens' core: the inbox files and clears its mail, a letter
//! opens, is answered and saved, notes and "my players" keep, and every core
//! page's data comes back on a small world at a college seat.

use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn the_desk_files_answers_and_clears_its_mail() {
    let dir = std::env::temp_dir().join(format!("afm-core-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 11, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

    // The welcome letter is waiting, unread, with its big moment, beside
    // your No. 2's first note of the guided week (lane 8, H12).
    let mail = ok(&mut g, "mail", json!({}));
    assert_eq!(mail["unread"], 2, "{mail}");
    let welcome =
        mail["rows"].as_array().unwrap().iter().find(|r| r["head"] == "school").unwrap_or_else(|| panic!("{mail}"));
    let id = welcome["id"].as_str().unwrap().to_string();
    assert_eq!(ok(&mut g, "status", json!({}))["reveals"], 1);
    let reveals = ok(&mut g, "reveals", json!({}));
    assert_eq!(reveals[0]["kind"], "hire", "{reveals}");
    assert_eq!(ok(&mut g, "status", json!({}))["reveals"], 0);
    let letter = ok(&mut g, "letter", json!({ "id": id }));
    assert_eq!(letter["head"], "school", "{letter}");
    assert_eq!(ok(&mut g, "mail", json!({}))["unread"], 1);

    // Saved mail survives the 30-day clearing; the rest goes.
    ok(&mut g, "mail_set", json!({ "id": id, "saved": true }));
    ok(&mut g, "holiday", json!({ "days": 45 }));
    // Every page finds people by binary search on their id (`person_index`).
    let w = g.world.as_ref().unwrap();
    assert!(w.people.windows(2).all(|p| p[0].player.id < p[1].player.id), "people stay in id order");
    let mail = ok(&mut g, "mail", json!({}));
    let rows = mail["rows"].as_array().unwrap();
    assert!(rows.iter().any(|r| r["id"] == id.as_str()), "the saved welcome letter stays");
    assert!(rows.len() > 1, "new mail arrived: {mail}");
    for r in rows.iter().filter(|r| r["saved"] == false) {
        assert!(r["clears_in"].is_null() || r["clears_in"].as_i64().unwrap() <= 10, "{r}");
    }
    // Every keyword the filter offers counts real mail; search finds letters.
    let keyed: i64 = mail["keywords"].as_array().unwrap().iter().map(|k| k["count"].as_i64().unwrap()).sum();
    assert!(keyed > 0, "{mail}");
    assert_eq!(ok(&mut g, "mail", json!({ "q": "zzzz-no-such-word" }))["rows"].as_array().unwrap().len(), 0);

    // An answer does what it says once, and is kept.
    if let Some(r) = rows.iter().find(|r| r["answers"].as_u64().unwrap_or(0) > 0 && r["answered"].is_null()) {
        let a = ok(&mut g, "mail_answer", json!({ "id": r["id"], "pick": 0 }));
        assert_eq!(a["answered"], 0, "{a}");
        assert!(
            g.call("mail_answer", &json!({ "id": r["id"], "pick": 0 })).is_err()
                || ok(&mut g, "letter", json!({ "id": r["id"] }))["answered"] == 0
        );
    }

    // My players, notes and the finder.
    let roster = ok(&mut g, "roster", json!({}));
    let pid = roster["rows"][0]["id"].as_u64().unwrap();
    assert_eq!(ok(&mut g, "list_add", json!({ "id": pid }))["listed"], true);
    ok(
        &mut g,
        "note_write",
        json!({ "text": "Quick feet.", "page": "Player page", "about": { "kind": "player", "id": pid }, "about_name": "him" }),
    );
    let mine = ok(&mut g, "my_players", json!({}));
    assert_eq!(mine["rows"].as_array().unwrap().len(), 1, "{mine}");
    let notes = ok(&mut g, "notes_all", json!({}));
    assert!(notes["notes"].as_array().unwrap().iter().any(|n| n["text"] == "Quick feet."), "{notes}");
    let found = ok(&mut g, "finder", json!({ "filters": [{ "field": "listed", "value": true }] }));
    assert_eq!(found["total"], 1, "{found}");
    assert_eq!(
        ok(&mut g, "list_remove", json!({ "id": pid }))["listed"],
        true,
        "a player with a note stays on the list"
    );

    // Every core page answers.
    let p = ok(&mut g, "profile", json!({ "id": pid }));
    for k in ["level_rating", "measurables", "key_mental", "fit", "seasons", "with_you", "recruiting"] {
        assert!(p.get(k).is_some(), "the player page has {k}");
    }
    assert!(p["key_mental"].as_array().unwrap().len() <= 5);
    assert!(p.get("potential_true").is_none() && p.get("ceiling").is_none(), "nothing hidden");
    let home = ok(&mut g, "dashboard", json!({}));
    assert!(!home["slides"].as_array().unwrap().is_empty(), "{home}");
    // The Home hero's three slides come first (lane 16), the season's own after.
    let kinds: Vec<&str> = home["slides"].as_array().unwrap().iter().filter_map(|s| s["kind"].as_str()).collect();
    assert_eq!(kinds[..3], ["stadium", "story", "room"], "{kinds:?}");
    let a = &home["slides"][0]["hero"];
    assert!(a["road"].is_array() && a["team"]["name"].is_string() && a["captains"].is_array(), "{a}");
    assert!(home["slides"][1]["hero"]["bars"].as_array().is_some_and(|b| !b.is_empty()), "{}", home["slides"][1]);
    assert!(home["slides"][2]["hero"]["facts"].as_array().is_some_and(|f| !f.is_empty()), "{}", home["slides"][2]);
    // Everyone in the team photo appears once.
    let c = &home["slides"][2]["hero"];
    let mut photo: Vec<u64> = ["back", "mid", "st", "caps"]
        .iter()
        .flat_map(|k| c[*k].as_array().into_iter().flatten().filter_map(|p| p["id"].as_u64()))
        .collect();
    let all = photo.len();
    photo.sort_unstable();
    photo.dedup();
    assert_eq!(photo.len(), all, "a player twice in the team photo: {c}");
    assert!(home["know"].as_array().is_some_and(|k| k.len() <= 3), "{}", home["know"]);
    for (cmd, args) in [
        ("peek", json!({ "kind": "player", "id": pid })),
        ("find", json!({ "q": "Te" })),
        ("leaders", json!({})),
        ("situations", json!({})),
        ("decisions", json!({})),
        ("concepts", json!({})),
        ("your_numbers", json!({})),
        ("next_opponent", json!({})),
        ("prospects", json!({})),
        ("prospects", json!({ "level": "hs" })),
        ("prospect_card", json!({ "id": pid })),
        ("draft_data", json!({})),
        ("draft_data", json!({ "tab": "clubs" })),
        ("recruit_data", json!({})),
        ("recruit_data", json!({ "tab": "pipelines" })),
        ("portal_data", json!({})),
        ("coach_data", json!({})),
        ("coach_data", json!({ "level": "nfl", "role": "hc", "window": 0 })),
        ("coach_data", json!({ "level": "hs" })),
        ("money_data", json!({})),
        ("money_data", json!({ "kind": "nfl", "id": 3 })),
        ("money_data", json!({ "kind": "hs", "id": 5 })),
        ("history", json!({})),
        ("history", json!({ "tab": "drafts" })),
        ("brackets", json!({})),
        ("feed", json!({ "kind": "national" })),
        ("calendar_month", json!({})),
        ("career_page", json!({})),
        ("team_page", json!({})),
        ("compare", json!({ "ids": [pid, roster["rows"][1]["id"]] })),
    ] {
        ok(&mut g, cmd, args);
    }
    let coach = home["boss"]["name"].as_str().unwrap_or_default().to_string();
    assert!(!coach.is_empty());
    let csv = ok(&mut g, "export_text", json!({ "name": "My Table!", "text": "a,b\n1,2\n" }));
    let path = csv["path"].as_str().unwrap();
    assert!(path.ends_with("my-table.csv"), "{path}");
    assert_eq!(std::fs::read_to_string(path).unwrap(), "a,b\n1,2\n");

    // The no-fire override (Settings, Testing): off at the start, marks the
    // career when turned on, keeps the mark when turned off, and the save's
    // header carries it to Load game.
    let nf = ok(&mut g, "no_fire", json!({}));
    assert!(nf["career"] == true && nf["on"] == false && nf["since"].is_null(), "{nf}");
    let nf = ok(&mut g, "no_fire_set", json!({ "on": true }));
    assert!(nf["on"] == true && nf["since"].is_string(), "{nf}");
    let nf = ok(&mut g, "no_fire_set", json!({ "on": false }));
    assert!(nf["on"] == false && nf["since"].is_string() && nf["spared"].as_array().unwrap().is_empty(), "{nf}");
    ok(&mut g, "save", json!({ "name": "no-fire test" }));
    let lib = ok(&mut g, "library", json!({}));
    assert!(lib["saves"].as_array().unwrap().iter().any(|s| s["meta"]["no_fire"].is_string()), "{lib}");

    // Sandbox mode (Settings, a test switch; no-fire is one of its
    // switches): the no-fire turn above marked the career, every other
    // switch waits; on, the frame shows it; the jump plays days; the save's
    // header carries the mark to Load game.
    let sb = ok(&mut g, "sandbox", json!({}));
    assert!(sb["career"] == true && sb["on"] == true && sb["since"].is_string(), "{sb}");
    let lifted =
        |sb: &Value, k: &str| sb["switches"].as_array().unwrap().iter().any(|s| s["key"] == k && s["lifted"] == true);
    assert!(!lifted(&sb, "film_hours") && !lifted(&sb, "no_fire"), "{sb}");
    let sb = ok(&mut g, "sandbox_switch", json!({ "key": "film_hours", "on": true }));
    assert!(sb["on"] == true && lifted(&sb, "film_hours") && !lifted(&sb, "nil_budget"), "{sb}");
    assert!(g.call("sandbox_switch", &json!({ "key": "nothing", "on": true })).is_err());
    assert_eq!(ok(&mut g, "status", json!({}))["sandbox"], true);
    let before = ok(&mut g, "sandbox", json!({}))["today"].as_str().unwrap().to_string();
    let r = ok(&mut g, "sandbox_jump", json!({ "days": 3 }));
    assert!(r["days"].as_i64().unwrap() >= 3, "{r}");
    let after = ok(&mut g, "sandbox", json!({}))["today"].as_str().unwrap().to_string();
    assert!(after > before, "{before} -> {after}");
    assert!(g.call("sandbox_jump", &json!({ "until": before })).is_err(), "not into the past");
    ok(&mut g, "save", json!({ "name": "sandbox test" }));
    let lib = ok(&mut g, "library", json!({}));
    assert!(lib["saves"].as_array().unwrap().iter().any(|s| s["meta"]["sandbox"].is_string()), "{lib}");
    let sb = ok(&mut g, "sandbox_set", json!({ "on": false }));
    assert!(sb["on"] == false && sb["since"].is_string(), "the mark stays: {sb}");
    assert!(g.call("sandbox_jump", &json!({ "days": 3 })).is_err(), "the jump needs sandbox mode on");
    assert_eq!(ok(&mut g, "status", json!({}))["sandbox"], false);
    let _ = std::fs::remove_dir_all(dir);
}

/// A title keeps the level it was won at (minor bug 293): a program that moved
/// up to FBS keeps its FCS titles as FCS on the world card, the History
/// champions, its banner, its team page, and its job file ("2 FCS titles").
#[test]
fn titles_keep_the_level_they_were_won_at() {
    use afm_engine::world::data::Division;
    use afm_game::views::TeamRef;
    let dir = std::env::temp_dir().join(format!("afm-titles-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 11, "history_years": 2, "scale": 0.04, "engine_from": 2026 }));
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job =
        jobs["jobs"].as_array().unwrap().iter().find(|j| j["division"] == "FBS").unwrap_or_else(|| panic!("{jobs}"));
    let p = job["ref"]["id"].as_u64().unwrap() as usize;

    // Every season on record was an FCS title, won before the move up.
    let w = g.world.as_mut().unwrap();
    let prog = &mut w.programs[p];
    assert_eq!(prog.division, Division::FBS);
    for s in prog.seasons.iter_mut() {
        s.division = Some(Division::FCS);
        s.national_title = true;
    }
    let n = prog.seasons.len();
    assert!(n >= 2, "{n} seasons");
    prog.titles = n as u32;
    let name = w.program_name(p).to_string();
    let w = g.world.as_ref().unwrap();

    // The world card's national champions and story leave them out.
    let card = afm_game::cards::card(w);
    assert!(card["world"]["nc"].as_array().unwrap().iter().all(|r| r[0] != name.as_str()), "{}", card["world"]);
    assert!(card["world"]["story"].to_string().find(&name).is_none(), "{}", card["world"]["story"]);

    // History's champions file them under FCS.
    let champs = ok(&mut g, "history", json!({ "tab": "champions" }));
    let mine: Vec<&Value> =
        champs["body"].as_array().unwrap().iter().filter(|r| r["team"]["name"] == name.as_str()).collect();
    assert_eq!(mine.len(), n, "{champs}");
    assert!(mine.iter().all(|r| r["level"] == "FCS"), "{mine:?}");

    // The banner calls them FCS titles, never national ones.
    let w = g.world.as_ref().unwrap();
    let banner = afm_game::lane_core::hero::slides(w, TeamRef::College(p))[0].clone();
    let text = format!("{} {}", banner["title"], banner["sub"]);
    assert!(text.contains("FCS titles") && !text.contains("national"), "{text}");

    // The team page's seasons and the job file say FCS too.
    let page = ok(&mut g, "team_page", json!({ "kind": "college", "id": p }));
    assert!(page["seasons"].as_array().unwrap().iter().all(|s| s["nat_word"] == "FCS"), "{}", page["seasons"]);
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"].as_array().unwrap().iter().find(|j| j["ref"]["id"] == p).unwrap();
    assert_eq!(job["titles_text"], format!("{n} FCS titles"), "{job}");
    let _ = std::fs::remove_dir_all(dir);
}
