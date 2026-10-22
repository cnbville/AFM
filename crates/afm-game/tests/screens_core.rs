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

    // The welcome letter is waiting, unread, with its big moment.
    let mail = ok(&mut g, "mail", json!({}));
    assert_eq!(mail["unread"], 1, "{mail}");
    let id = mail["rows"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(ok(&mut g, "status", json!({}))["reveals"], 1);
    let reveals = ok(&mut g, "reveals", json!({}));
    assert_eq!(reveals[0]["kind"], "hire", "{reveals}");
    assert_eq!(ok(&mut g, "status", json!({}))["reveals"], 0);
    let letter = ok(&mut g, "letter", json!({ "id": id }));
    assert_eq!(letter["head"], "school", "{letter}");
    assert_eq!(ok(&mut g, "mail", json!({}))["unread"], 0);

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
    for (cmd, args) in [
        ("peek", json!({ "kind": "player", "id": pid })),
        ("find", json!({ "q": "Te" })),
        ("leaders", json!({})),
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
    let _ = std::fs::remove_dir_all(dir);
}
