//! Live talks lane 4, the desk, on a small world: the stat finder, the squad
//! bar, Find, the next opponent, a scout, the report and the Monday leads mail
//! all take typed words through the fake client. Each asks the AI only to
//! read or to write; every field it reads is checked against the page's own
//! lists and refused when the page doesn't know it (never run looser); every
//! reply is walked for hidden numbers; the leads mail composes with and
//! without a key; and a run of the clock makes no call. It builds a world, so
//! it runs in Full tests only. One ignored test runs the same asks on a real
//! world file (`AFM_DESK_WORLD`, a world folder or its first part).

mod common;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use afm_ai::Fake;
use afm_ai::fake::body;
use afm_engine::calendar::Weekday;
use afm_game::Game;
use afm_game::lane_core::mail;
use afm_game::views::{self, TeamRef};
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    let v = g.call(cmd, &args).unwrap_or_else(|e| panic!("{cmd}: {e}"));
    common::nothing_hidden(cmd, &v, "");
    v
}

fn finish(g: &mut Game, ticket: u64) -> Value {
    let t = Instant::now();
    loop {
        let v = ok(g, "ai_poll", json!({ "ticket": ticket }));
        if v["state"] != "wait" || t.elapsed() > Duration::from_secs(10) {
            return v;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Types a question into an ask box and waits for its answer.
fn asked(g: &mut Game, args: Value) -> Value {
    let id = ok(g, "ai_ask", args)["ticket"].as_u64().expect("a ticket");
    finish(g, id)
}

fn reply(f: &Fake, label: &str, v: Value) {
    f.push(label, Ok(body(&v.to_string(), 600, 80)));
}

fn home(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("afm-live-desk-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// A college seat on a small world with a season of numbers.
fn college() -> Game {
    let mut g = Game::new(home("college"));
    ok(&mut g, "build_world", json!({ "seed": 66, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let job = ok(&mut g, "jobs", json!({ "level": "college" }))["jobs"][0]["ref"].clone();
    ok(
        &mut g,
        "start_career",
        json!({ "name": "Test Coach", "age": 41, "background": "college_assistant", "job": job, "seed": 9 }),
    );
    g
}

fn has_ask(v: &Value, kind: &str) -> bool {
    v["ask"].as_array().is_some_and(|a| a.iter().any(|e| e["args"]["kind"] == kind))
}

fn chips<'a>(v: &'a Value, field: &str) -> Vec<&'a Value> {
    v["query"].as_array().map(|q| q.iter().filter(|c| c["field"] == field).collect()).unwrap_or_default()
}

#[test]
fn the_desk_takes_typed_words_and_only_what_the_pages_know() {
    let mut g = college();
    let fake = Arc::new(Fake::new());

    // No key: no box on any page, and the pages read as before.
    let sf = ok(&mut g, "stat_finder", json!({}));
    let fr = ok(&mut g, "find_recruits", json!({}));
    let sq = ok(&mut g, "squad", json!({}));
    for (cmd, v) in [("stat_finder", &sf), ("find_recruits", &fr), ("squad", &sq)] {
        assert!(v.get("ask").is_none(), "{cmd} shows no box with no key");
    }

    // With a key (the fake), each page carries its own box.
    g.ai.use_fake(fake.clone());
    let sf = ok(&mut g, "stat_finder", json!({}));
    let fr = ok(&mut g, "find_recruits", json!({}));
    let sq = ok(&mut g, "squad", json!({}));
    assert!(has_ask(&sf, "analyst") && has_ask(&fr, "find") && has_ask(&sq, "squad"));
    assert_eq!(
        sq["ask"][0]["args"]["prod"],
        json!(sq["qf"].as_array().unwrap().iter().any(|g| g["name"].as_str().unwrap().starts_with("Production")))
    );

    // --- The analyst (LT15): the finder's own form, then one line.
    reply(
        &fake,
        "analyst_reader",
        json!({ "fit": "clear", "find": "seasons", "level": "college", "division": null, "from": 2020, "to": null,
                "pos": "RB", "class": "Fr", "rs": null, "age_min": null, "age_max": null, "min_len": null,
                "conds": [{ "stat": "RushYds", "op": "ge", "value": 1000 }] }),
    );
    fake.push_text("analyst_writer", "Nobody has done it yet.");
    let a = asked(&mut g, json!({ "kind": "analyst", "text": "which freshman backs ran for 1,000 yards since 2020?" }));
    assert_eq!(a["state"], "done", "{a}");
    let fields: Vec<&str> = a["query"].as_array().unwrap().iter().map(|c| c["field"].as_str().unwrap()).collect();
    assert_eq!(fields, ["find", "level", "from", "pos", "class", "cond"]);
    assert_eq!(chips(&a, "cond")[0]["stat"], "RushYds");
    assert!(!a["line"].as_str().unwrap().is_empty());
    // The reader got the finder's own numbers, the schema, and no result.
    let reqs = fake.asked();
    let reader = reqs.iter().rev().find(|r| r.label == "analyst_reader").unwrap();
    assert!(reader.rules.contains("RushYds") && reader.schema.is_some());
    assert!(reader.user.contains("<question>which freshman backs"));
    assert!(!reader.rules.to_lowercase().contains("potential:"), "no field for potential");

    // The form is checked: nothing the finder doesn't know is run looser.
    let before = fake.asked_count();
    for bad in [
        json!({ "fit": "clear", "find": "seasons", "level": null, "division": null, "from": null, "to": null, "pos": null,
                "class": null, "rs": null, "age_min": null, "age_max": null, "min_len": null,
                "conds": [{ "stat": "TruePotential", "op": "ge", "value": 80 }] }),
        json!({ "fit": "clear", "find": "seasons", "level": "pro", "division": null, "from": null, "to": null, "pos": null,
                "class": null, "rs": null, "age_min": null, "age_max": null, "min_len": null, "conds": [] }),
        json!({ "fit": "clear", "find": "streaks", "level": null, "division": null, "from": null, "to": null, "pos": null,
                "class": null, "rs": null, "age_min": null, "age_max": null, "min_len": null, "conds": [] }),
        json!({ "fit": "clear", "find": "seasons", "level": null, "division": "Premier", "from": null, "to": null, "pos": null,
                "class": null, "rs": null, "age_min": null, "age_max": null, "min_len": null, "conds": [] }),
        json!({ "fit": "clear", "find": "seasons", "level": null, "division": null, "from": 2030, "to": 2020, "pos": null,
                "class": null, "rs": null, "age_min": null, "age_max": null, "min_len": null, "conds": [] }),
        json!({ "fit": "not_a_search", "find": null, "level": null, "division": null, "from": null, "to": null, "pos": null,
                "class": null, "rs": null, "age_min": null, "age_max": null, "min_len": null, "conds": [] }),
    ] {
        reply(&fake, "analyst_reader", bad.clone());
        let a = asked(&mut g, json!({ "kind": "analyst", "text": "something" }));
        assert_eq!(a["state"], "none", "{bad} -> {a}");
        assert!(a["query"].is_null());
    }
    let nine: Vec<Value> = (0..9).map(|_| json!({ "stat": "Rush", "op": "ge", "value": 1 })).collect();
    reply(
        &fake,
        "analyst_reader",
        json!({ "fit": "clear", "find": "seasons", "level": null, "division": null, "from": null, "to": null, "pos": null,
                "class": null, "rs": null, "age_min": null, "age_max": null, "min_len": null, "conds": nine }),
    );
    assert_eq!(asked(&mut g, json!({ "kind": "analyst", "text": "x" }))["state"], "none");
    // Six refused readings made six calls and no writer call.
    assert_eq!(fake.asked_count() - before, 7, "a refused reading makes no second call");
    // A box with no words, and a box with too many.
    assert!(g.call("ai_ask", &json!({ "kind": "analyst", "text": "  " })).is_err());
    assert!(g.call("ai_ask", &json!({ "kind": "analyst", "text": "x".repeat(401) })).is_err());

    // --- The squad bar (LT15): the bar's own fields.
    reply(
        &fake,
        "squad_reader",
        json!({ "fit": "clear", "rules": [{ "k": "a0", "op": "<", "v": 60 }, { "k": "age", "op": "≤", "v": 20 }],
                "pos": ["WR", "CB", "WR"], "q": "" }),
    );
    let s = asked(&mut g, json!({ "kind": "squad", "text": "WRs or CBs under 60", "prod": false }));
    assert_eq!(s["state"], "done", "{s}");
    assert_eq!(chips(&s, "pos")[0]["value"], json!(["WR", "CB"]));
    assert_eq!(chips(&s, "a0")[0]["op"], "<");
    assert_eq!(chips(&s, "age")[0]["op"], "≤");
    for bad in [
        json!({ "fit": "clear", "rules": [{ "k": "potential", "op": ">", "v": 70 }], "pos": [], "q": "" }),
        json!({ "fit": "clear", "rules": [{ "k": "a999", "op": ">", "v": 70 }], "pos": [], "q": "" }),
        json!({ "fit": "clear", "rules": [{ "k": "a0", "op": "!=", "v": 70 }], "pos": [], "q": "" }),
        json!({ "fit": "clear", "rules": [{ "k": "p:g", "op": ">", "v": 3 }], "pos": [], "q": "" }),
        json!({ "fit": "clear", "rules": (0..5).map(|_| json!({ "k": "a0", "op": ">", "v": 1 })).collect::<Vec<_>>(), "pos": [], "q": "" }),
        json!({ "fit": "clear", "rules": [], "pos": ["KR"], "q": "" }),
        json!({ "fit": "clear", "rules": [], "pos": [], "q": "" }),
        json!({ "fit": "not_a_filter", "rules": [], "pos": [], "q": "" }),
    ] {
        reply(&fake, "squad_reader", bad.clone());
        let s = asked(&mut g, json!({ "kind": "squad", "text": "x", "prod": false }));
        assert_eq!(s["state"], "none", "{bad} -> {s}");
    }
    // Production only where the bar offers it (`prod` false here).
    let reqs = fake.asked();
    let squad_reader = reqs.iter().rev().find(|r| r.label == "squad_reader").unwrap();
    assert!(!squad_reader.rules.contains("p:g"), "no production field without production");
    assert!(!squad_reader.rules.contains("stars") || squad_reader.rules.contains("Recruiting stars"));

    // --- Find (LT20): the seat's catalog, every field and option checked.
    reply(
        &fake,
        "find_reader",
        json!({ "fit": "clear", "pos": "S", "level": "hs", "class": null, "sure": "fair", "unseen": null, "q": null,
                "filters": [{ "k": "t40", "op": "le", "v": "4.55", "v2": "" }, { "k": "medical", "op": "ge", "v": "Cleared", "v2": "" },
                            { "k": "state", "op": "is", "v": "TX", "v2": "" }, { "k": "committed", "op": "is", "v": "no", "v2": "" }] }),
    );
    let f = asked(&mut g, json!({ "kind": "find", "text": "a safety from Texas, fast, healthy, not committed" }));
    assert_eq!(f["state"], "done", "{f}");
    assert_eq!(chips(&f, "pos")[0]["value"], "S");
    assert_eq!(chips(&f, "t40")[0]["value"], json!(4.55));
    assert_eq!(chips(&f, "medical")[0]["value"], json!(3), "an option is its place in the list");
    assert_eq!(chips(&f, "state")[0]["value"], "TX");
    assert_eq!(chips(&f, "committed")[0]["value"], json!(0));
    for bad in [
        json!({ "fit": "clear", "pos": null, "level": null, "class": null, "sure": null, "unseen": null, "q": null,
                "filters": [{ "k": "ceiling", "op": "ge", "v": "5", "v2": "" }] }),
        json!({ "fit": "clear", "pos": null, "level": null, "class": null, "sure": null, "unseen": null, "q": null,
                "filters": [{ "k": "medical", "op": "ge", "v": "Elite", "v2": "" }] }),
        json!({ "fit": "clear", "pos": null, "level": null, "class": null, "sure": null, "unseen": null, "q": null,
                "filters": [{ "k": "state", "op": "ge", "v": "TX", "v2": "" }] }),
        json!({ "fit": "clear", "pos": null, "level": null, "class": null, "sure": null, "unseen": null, "q": null,
                "filters": [{ "k": "pos", "op": "is", "v": "S", "v2": "" }] }),
        json!({ "fit": "clear", "pos": null, "level": null, "class": null, "sure": null, "unseen": null, "q": null,
                "filters": [{ "k": "t40", "op": "between", "v": "4.4", "v2": "fast" }] }),
        json!({ "fit": "clear", "pos": "Kicker", "level": null, "class": null, "sure": null, "unseen": null, "q": null, "filters": [] }),
        json!({ "fit": "clear", "pos": null, "level": "nfl", "class": null, "sure": null, "unseen": null, "q": null, "filters": [] }),
        json!({ "fit": "clear", "pos": null, "level": null, "class": 1999, "sure": null, "unseen": null, "q": null, "filters": [] }),
    ] {
        reply(&fake, "find_reader", bad.clone());
        let f = asked(&mut g, json!({ "kind": "find", "text": "x" }));
        assert_eq!(f["state"], "none", "{bad} -> {f}");
    }

    // --- No call is made while the clock runs; the ask boxes make none on their own.
    let calls_before = fake.asked_count();
    ok(&mut g, "holiday", json!({ "days": 5 }));
    assert_eq!(fake.asked_count(), calls_before, "a run of the clock makes no call");
    let _ = ok(&mut g, "stat_finder", json!({}));
    let _ = ok(&mut g, "find_recruits", json!({}));
    assert_eq!(fake.asked_count(), calls_before, "loading a page makes no call");

    // --- A scout and the report (LT13, LT14) on a file your staff has read.
    let mine = {
        let w = g.world.as_ref().unwrap();
        let Some(TeamRef::College(pi)) = views::mine(w) else { panic!("a college seat") };
        w.programs[pi].roster.iter().map(|&k| w.people[k].player.id).collect::<Vec<u32>>()
    };
    let seen = mine.iter().copied().find(|&id| ok(&mut g, "scout_file", json!({ "id": id }))["seen"] == true);
    if let Some(id) = seen {
        let file = ok(&mut g, "scout_file", json!({ "id": id }));
        assert!(has_ask(&file, "report") && has_ask(&file, "scout"), "a read file carries both boxes: {}", file["ask"]);
        // A scout answers in his words; a number the file doesn't hold is dropped.
        fake.push_text("scout_writer", "I have him somewhere around 9999 and he is a lock.");
        let a = asked(&mut g, json!({ "kind": "scout", "player": id, "text": "how good is he really?" }));
        assert!(a["state"] == "none" || !a["text"].as_str().unwrap().contains("9999"), "{a}");
        fake.push_text("scout_writer", "He is a plain, hard worker and I would like to see him once more.");
        let a = asked(&mut g, json!({ "kind": "scout", "player": id, "text": "tell me about him" }));
        assert_eq!(a["text"], "He is a plain, hard worker and I would like to see him once more.");
        // The card the writer got has words, no hidden number.
        let reqs = fake.asked();
        let w = reqs.iter().rev().find(|r| r.label == "scout_writer").unwrap();
        for hidden in ["potential:", "ceiling:", "truth"] {
            assert!(!w.user.to_lowercase().contains(hidden), "{hidden} in {}", w.user);
        }
        // The report: a heading the file never gave drops the writer's words for the set lines.
        fake.push_text("report_writer", "## His real ceiling\nA star.");
        let r = asked(&mut g, json!({ "kind": "report", "player": id }));
        assert_eq!(r["state"], "done", "{r}");
        let text = r["text"].as_str().unwrap();
        assert!(!text.contains("His real ceiling") && text.starts_with("## "), "the fallback is the set lines: {text}");
        fake.push_text("report_writer", "## Where he is now\nA plain read, as far as the file says.");
        let r = asked(&mut g, json!({ "kind": "report", "player": id }));
        assert!(r["text"].as_str().unwrap().contains("A plain read"), "{r}");
    } else {
        println!("no read file on the small world's squad: the scout and report are checked on the real world");
    }
    // A player your staff has no file on: no answer.
    assert!(g.call("ai_ask", &json!({ "kind": "report", "player": 4_000_000_000u64 })).is_err());
}

#[test]
fn the_monday_leads_mail_composes_with_no_key_and_asks_once_with_one() {
    let mut g = college();
    let fake = Arc::new(Fake::new());
    // The first Monday in October of the year the history ended in: the
    // season's numbers are in, and the mail is filed as the clock would.
    let (monday, mine) = {
        let w = g.world.as_mut().unwrap();
        let Some(TeamRef::College(pi)) = views::mine(w) else { panic!("a college seat") };
        let mut d = afm_engine::calendar::Date::ymd(w.today.year(), 10, 1);
        while d.weekday() != Weekday::Mon {
            d = d.add(1);
        }
        w.today = d.add(1);
        w.career.as_mut().unwrap().desk.through = Some(d.add(-1));
        mail::after_days(w);
        let mine: Vec<u32> = w.programs[pi].roster.iter().map(|&k| w.people[k].player.id).collect();
        (d, mine)
    };
    let all = ok(&mut g, "mail", json!({}))["rows"].as_array().unwrap().clone();
    let Some(row) = all.iter().find(|r| r["subject"].as_str().is_some_and(|s| s.starts_with("Leads:"))) else {
        println!("no outliers on the small world on {monday:?}: the mail is checked on the real world");
        return;
    };
    let id = row["id"].as_str().unwrap().to_string();

    // No key: the whole mail is the game's own table, and no box.
    let m = ok(&mut g, "letter", json!({ "id": id }));
    assert!(m.get("ask").is_none());
    let rows = m["table"].as_array().unwrap().len() - 1;
    assert!((1..=8).contains(&rows), "{rows} leads");
    assert_eq!(m["table"][0][4], "Best numbers");
    // None of them is already on your squad.
    let ids = {
        let c = g.world.as_ref().unwrap().career.as_ref().unwrap();
        c.desk.mails.iter().find(|x| x.id.to_string() == id).unwrap().ids.clone()
    };
    assert_eq!(ids.len(), rows);
    assert!(ids.iter().all(|i| !mine.contains(i)), "a lead is never your own player");
    let mut seen = ids.clone();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), ids.len(), "no one twice");

    // With a key: opening it asks once, for the writer's part.
    g.ai.use_fake(fake.clone());
    let m = ok(&mut g, "letter", json!({ "id": id }));
    let entry = &m["ask"][0];
    assert_eq!(entry["auto"], true);
    assert_eq!(entry["args"]["kind"], "leads");
    assert_eq!(entry["args"]["mail"], id.as_str());
    let first = ids[0];
    let second = ids[1.min(ids.len() - 1)];
    let picks = json!({ "picks": [{ "id": format!("l{}", ids.iter().position(|&x| x == second).unwrap() + 1), "why": "He stands out at the top of the level." },
                                   { "id": "l1", "why": "His potential is huge." },
                                   { "id": "l99", "why": "Not a lead." }] });
    reply(&fake, "leads_writer", picks);
    let calls = fake.asked_count();
    let a = asked(&mut g, entry["args"].clone());
    assert_eq!(a["state"], "done");
    assert_eq!(a["reload"], true);
    assert_eq!(fake.asked_count(), calls + 1);
    let m = ok(&mut g, "letter", json!({ "id": id }));
    assert!(m.get("ask").is_none(), "written once, never asked again");
    assert_eq!(m["table"][0][4], "Why he stands out");
    assert_eq!(m["table"].as_array().unwrap().len() - 1, rows, "every lead is still in the mail");
    let c = g.world.as_ref().unwrap().career.as_ref().unwrap();
    let stored = c.desk.mails.iter().find(|x| x.id.to_string() == id).unwrap();
    assert_eq!(stored.leads, 2);
    assert_eq!(stored.ids[0], second, "his picks come first");
    assert!(m["table"][1][4].as_str().unwrap().contains("He stands out at the top of the level."));
    // A why that names a banned word gives way to the game's own numbers.
    if ids.len() > 1 && first != second {
        let row = stored.ids.iter().position(|&x| x == first).unwrap() + 1;
        assert!(!m["table"][row][4].as_str().unwrap().contains("potential"), "{}", m["table"][row][4]);
    }
    // Asking again is a reload and makes no call.
    let calls = fake.asked_count();
    let again = asked(&mut g, json!({ "kind": "leads", "mail": id }));
    assert_eq!((again["state"].as_str(), again["reload"].as_bool()), (Some("done"), Some(true)));
    assert_eq!(fake.asked_count(), calls);
    // A mail that has cleared is refused.
    assert!(g.call("ai_ask", &json!({ "kind": "leads", "mail": "12345" })).is_err());
}

/// The same asks on a real world file (`AFM_DESK_WORLD`: a world folder or its
/// first part): the stat finder, the squad bar, Find, the next opponent, a scout
/// and the report, then the leads mail and what its outlier pass costs. Times
/// and answers are printed. Not run in CI.
#[test]
#[ignore]
fn the_desk_on_a_real_world() {
    let Ok(path) = std::env::var("AFM_DESK_WORLD") else {
        println!("skipped: AFM_DESK_WORLD is not set");
        return;
    };
    let mut g = Game::new(home("real"));
    let t = Instant::now();
    let to = ok(&mut g, "import_world", json!({ "path": path }))["path"].as_str().unwrap().to_string();
    ok(&mut g, "open_world", json!({ "path": to }));
    println!("world opened in {:?}", t.elapsed());
    let job = ok(&mut g, "jobs", json!({ "level": "college" }))["jobs"][0]["ref"].clone();
    ok(
        &mut g,
        "start_career",
        json!({ "name": "Test Coach", "age": 41, "background": "college_assistant", "job": job, "seed": 9 }),
    );
    let fake = Arc::new(Fake::new());
    g.ai.use_fake(fake.clone());
    println!("today {:?}", g.world.as_ref().unwrap().today);

    // The stat finder: a real search by words.
    reply(
        &fake,
        "analyst_reader",
        json!({ "fit": "clear", "find": "seasons", "level": "college", "division": null, "from": 2015, "to": null,
                "pos": "RB", "class": null, "rs": null, "age_min": null, "age_max": null, "min_len": null,
                "conds": [{ "stat": "RushYds", "op": "ge", "value": 1000 }] }),
    );
    fake.push_text("analyst_writer", "A few backs have done it.");
    let t = Instant::now();
    let a = asked(&mut g, json!({ "kind": "analyst", "text": "backs with 1,000 rushing yards since 2015" }));
    println!("analyst {:?}: {} rows, line {:?}", t.elapsed(), a["rows"].as_array().map_or(0, Vec::len), a["line"]);
    assert_eq!(a["state"], "done", "{a}");
    // The finder's own answer to the same form, for comparison.
    for (yds, from) in [(1800, 2015), (1000, 2015), (1500, 1990)] {
        let own = ok(
            &mut g,
            "stat_finder",
            json!({ "find": "seasons", "level": "college", "from": from, "pos": "RB", "conds": [{ "stat": "RushYds", "op": ">=", "v": yds }] }),
        );
        println!(
            "finder {yds} yds since {from}: total {} {}",
            own["total"],
            own.to_string().chars().take(200).collect::<String>()
        );
    }
    println!("analyst answer: {}", a.to_string().chars().take(500).collect::<String>());

    // The squad bar.
    reply(
        &fake,
        "squad_reader",
        json!({ "fit": "clear", "rules": [{ "k": "age", "op": "≤", "v": 20 }], "pos": ["WR"], "q": "" }),
    );
    let s = asked(&mut g, json!({ "kind": "squad", "text": "young receivers", "prod": false }));
    println!("squad: {} chips, line {:?}", s["query"].as_array().map_or(0, Vec::len), s["line"]);
    assert_eq!(s["state"], "done", "{s}");

    // Find, on a recruit list.
    reply(
        &fake,
        "find_reader",
        json!({ "fit": "clear", "pos": "QB", "level": null, "class": null, "sure": null, "unseen": null, "q": null, "filters": [] }),
    );
    let t = Instant::now();
    let f = asked(&mut g, json!({ "kind": "find", "text": "quarterbacks" }));
    println!("find {:?}: {}", t.elapsed(), f.to_string().chars().take(400).collect::<String>());
    assert!(f["state"] == "done" || f["state"] == "none", "{f}");

    // A scout and the report, on a player the staff has read.
    let me = {
        let w = g.world.as_ref().unwrap();
        let Some(TeamRef::College(pi)) = views::mine(w) else { panic!("a college seat") };
        w.programs[pi].roster.iter().map(|&k| w.people[k].player.id).collect::<Vec<u32>>()
    };
    let mut done = 0;
    for id in me.iter().take(40) {
        let file = ok(&mut g, "scout_file", json!({ "id": id }));
        if file["seen"] != true {
            continue;
        }
        assert!(has_ask(&file, "scout") && has_ask(&file, "report"), "{id}");
        fake.push_text("scout_writer", "I only have what is in the file.");
        let r = asked(&mut g, json!({ "kind": "scout", "player": id, "text": "what do you think of him?" }));
        assert_eq!(r["state"], "done", "{r}");
        let rep = asked(&mut g, json!({ "kind": "report", "player": id }));
        assert_eq!(rep["state"], "done", "{rep}");
        if done == 0 {
            println!("scout: {}\nreport:\n{}", r["text"], rep["text"]);
        }
        done += 1;
        if done == 3 {
            break;
        }
    }
    println!("{done} players asked about");

    for gi in [0, 1, 2, 5] {
        let t = ok(&mut g, "analytics", json!({ "group": gi, "limit": 500 }));
        let rows = t["rows"].as_array().cloned().unwrap_or_default();
        let with_pct = rows
            .iter()
            .filter(|r| r["pct"].as_array().is_some_and(|p| p.iter().any(|x| x.as_i64().is_some_and(|x| x >= 90))))
            .count();
        println!(
            "analytics group {gi}: year {} rows {} with a 90th+ {} keys {:?}",
            t["year"],
            rows.len(),
            with_pct,
            rows.first().and_then(|r| r.as_object()).map(|o| o.keys().collect::<Vec<_>>())
        );
    }
    // The leads mail: the first Monday on or after today inside September to January.
    let mut d = afm_engine::calendar::Date::ymd(g.world.as_ref().unwrap().today.year(), 1, 1);
    while d.weekday() != Weekday::Mon {
        d = d.add(1);
    }
    let t = Instant::now();
    {
        let w = g.world.as_mut().unwrap();
        let saved = w.today;
        w.today = d.add(1);
        w.career.as_mut().unwrap().desk.through = Some(d.add(-1));
        mail::after_days(w);
        w.today = saved;
    }
    println!("mail composed on {d:?} in {:?}", t.elapsed());
    let all = ok(&mut g, "mail", json!({}))["rows"].as_array().unwrap().clone();
    let leads = all.iter().find(|r| r["subject"].as_str().is_some_and(|s| s.starts_with("Leads:")));
    println!("leads mail present: {}", leads.is_some());
    if let Some(row) = leads {
        let id = row["id"].as_str().unwrap().to_string();
        let m = ok(&mut g, "letter", json!({ "id": id }));
        println!("letter: {} leads", m["table"].as_array().map_or(0, Vec::len).saturating_sub(1));
        for row in m["table"].as_array().unwrap().iter().skip(1) {
            println!("  {}", row);
        }
        let ids: Vec<String> = (1..=8).map(|n| format!("l{n}")).collect();
        let picks: Vec<Value> =
            ids.iter().map(|i| json!({ "id": i, "why": "He stands out on the numbers shown." })).collect();
        reply(&fake, "leads_writer", json!({ "picks": picks }));
        let t = Instant::now();
        let r = asked(&mut g, json!({ "kind": "leads", "mail": id }));
        println!("leads writer {:?}: {}", t.elapsed(), r["state"]);
        let m2 = ok(&mut g, "letter", json!({ "id": id }));
        println!("after: ask {:?}, header {:?}, first row {:?}", m2["ask"], m2["table"][0], m2["table"][1]);
    }
    // The next opponent, once the season has a game ahead: the clock runs on
    // (the game's own holiday step) until the page has an opponent.
    let t = Instant::now();
    let mut days = 0;
    let no = loop {
        let no = ok(&mut g, "next_opponent", json!({}));
        if has_ask(&no, "opponent") || days >= 260 {
            break no;
        }
        ok(&mut g, "holiday", json!({ "days": 20 }));
        days += 20;
    };
    println!("next_opponent after {days} days ({:?}): has_ask {}", t.elapsed(), has_ask(&no, "opponent"));
    if has_ask(&no, "opponent") {
        let args =
            no["ask"].as_array().unwrap().iter().find(|e| e["args"]["kind"] == "opponent").unwrap()["args"].clone();
        println!("opponent args {args}");
        reply(&fake, "opponent_reader", json!({ "fit": "clear", "side": "offense", "spots": ["3rd & 7-10"] }));
        fake.push_text("opponent_writer", "They pass more than the level does there.");
        let mut a = args.clone();
        a["text"] = json!("what do they do on third and long?");
        let t = Instant::now();
        let o = asked(&mut g, a);
        println!("opponent {:?}: {}", t.elapsed(), o.to_string().chars().take(900).collect::<String>());
    }
}
