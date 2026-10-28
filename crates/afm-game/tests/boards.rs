//! The game boards (update 2, lane 9) on a small world: a college seat and
//! an NFL seat each play into the season; every board answers, the money
//! boards add up to the contracts and the NIL deals, the line chart and the
//! throws are kept on your games and your coming opponents' film only.

use afm_engine::game::PlayType;
use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("not a number: {v}"))
}

/// A small world, built and saved once, opened by each seat's test in a
/// game folder of its own.
fn world(tag: &str) -> (Game, String) {
    static FRESH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let base = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("afm-boards-{}", std::process::id()));
    let p = FRESH
        .get_or_init(|| {
            let mut g = Game::new(base.join("world"));
            ok(&mut g, "build_world", json!({ "seed": 13, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
            ok(&mut g, "save", json!({ "name": "fresh" }))["saved"].as_str().unwrap().to_string()
        })
        .clone();
    (Game::new(base.join(tag)), p)
}

fn start(g: &mut Game, path: &str, level: &str, bg: &str) {
    ok(g, "open_world", json!({ "path": path }));
    let job = ok(g, "jobs", json!({ "level": level }))["jobs"][0]["ref"].clone();
    ok(g, "start_career", json!({ "name": "Board Coach", "age": 44, "background": bg, "job": job, "seed": 3 }));
}

/// Continues until `n` of your games are kept; the last one's index. An
/// NFL club with a pick drafts live (the war room holds Continue): the
/// staff makes your picks. An NFL seat hired in February also stops for its
/// first meetings and every big signing of the spring, every two or three
/// days in this small world, so the count leaves room for the world's draw.
fn play_to(g: &mut Game, n: usize) -> u64 {
    for _ in 0..150 {
        let games = ok(g, "games", json!({}))["games"].as_array().unwrap().clone();
        if games.len() >= n {
            return games.iter().map(|x| x["index"].as_u64().unwrap()).max().unwrap();
        }
        ok(g, "continue", json!({ "max_days": 30 }));
        draft_through(g);
    }
    panic!("never kept {n} games");
}

fn draft_through(g: &mut Game) {
    for _ in 0..200 {
        let room = ok(g, "draft_live", json!({}));
        if !room["live"].as_bool().unwrap_or(false) {
            return;
        }
        if room["rush"].as_bool().unwrap_or(false) {
            ok(g, "draft_rush", json!({}));
        } else if room["on_clock"].as_bool().unwrap_or(false) {
            ok(g, "draft_auto", json!({}));
        } else {
            ok(g, "draft_go", json!({ "until": "yours" }));
        }
    }
    panic!("the draft didn't finish");
}

/// The line chart and the throws sit on your games and your schedule's
/// film only: your coming opponents', and a team you just played until the
/// next film the room keeps drops it.
fn kept_where_they_should_be(g: &Game) {
    let w = g.world.as_ref().unwrap();
    let games = w.seat_games();
    assert!(!games.is_empty());
    for sg in games {
        let drops = sg
            .stats
            .plays
            .iter()
            .filter(|p| matches!(p.kind, PlayType::Pass | PlayType::Sack) && !p.nullified)
            .collect::<Vec<_>>();
        let charted = drops.iter().filter(|p| p.line.is_some()).count();
        assert!(charted * 10 >= drops.len() * 9, "{charted} of {} dropbacks charted in your game", drops.len());
    }
    let (level, me) = w.seat_team().unwrap();
    let other = |h: usize, a: usize| if h == me { a } else { h };
    let mut schedule = w.coming_opponents();
    if level == afm_engine::calendar::Level::Nfl {
        schedule.extend(w.nfl_fixtures(Some(me)).iter().map(|f| other(f.home, f.away)));
    } else {
        schedule.extend(w.college_fixtures(Some(me)).iter().map(|f| other(f.home, f.away)));
    }
    for t in &w.gameday.tapes {
        assert!(
            t.teams.iter().any(|x| schedule.contains(x)),
            "film of {:?} kept, not a team on your schedule ({schedule:?})",
            t.names
        );
    }
}

/// Every board answers at a seat, its numbers sane.
fn boards(g: &mut Game, k: u64) {
    let flow = ok(g, "board_flow", json!({ "index": k }));
    let wp = flow["wp"].as_array().unwrap();
    assert!(wp.len() > 100, "{} snaps on the chart", wp.len());
    for x in wp {
        let p = f(&x[2]);
        assert!((0.0..=1.0).contains(&p), "{x}");
    }
    let page = ok(g, "game_page", json!({ "index": k }));
    let us = flow["us"].as_u64().unwrap() as usize;
    let score: Vec<i64> = page["sides"].as_array().unwrap().iter().map(|s| s["score"].as_i64().unwrap()).collect();
    let fin: Vec<i64> = flow["final"]["score"].as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect();
    let mine = page["mine"].as_u64().unwrap_or(us as u64) as usize;
    assert_eq!(fin, vec![score[mine], score[1 - mine]], "the final on the board is the game's");
    let bat = ok(g, "board_battles", json!({ "index": k }));
    assert_eq!(bat["charted"], true, "your game is charted");
    assert!(!bat["modes"]["pass"]["snaps"].as_array().unwrap().is_empty());
    for m in ["pass", "run", "rush"] {
        for r in bat["modes"][m]["rows"].as_array().unwrap() {
            let n = r["won"].as_u64().unwrap() + r["even"].as_u64().unwrap() + r["lost"].as_u64().unwrap();
            assert!(n > 0, "{m}: a row with no reps");
        }
    }
    // The pass map of the quarterback who threw most in it.
    let gp = ok(g, "game", json!({ "index": k }));
    let qb = gp["sides"][mine]["passing"].as_array().and_then(|a| a.first()).and_then(|r| r["id"].as_u64());
    if let Some(id) = qb {
        let pm = ok(g, "board_passmap", json!({ "id": id }));
        assert_ne!(pm["source"], "none");
        assert!(!pm["throws"].as_array().unwrap().is_empty());
    }
    let lock = ok(g, "board_locker", json!({}));
    let nodes = lock["nodes"].as_array().unwrap();
    assert!(!nodes.is_empty() && nodes.len() <= 18);
    for e in lock["edges"].as_array().unwrap() {
        assert!(e["a"].as_u64().unwrap() < nodes.len() as u64 && e["b"].as_u64().unwrap() < nodes.len() as u64);
    }
    for c in lock["chem"].as_array().unwrap() {
        if let Some(v) = c["v"].as_f64() {
            assert!((0.0..=100.0).contains(&v), "{c}");
        }
    }
    let polls = ok(g, "board_polls", json!({}));
    assert!(polls["hero"].as_array().is_some_and(|h| !h.is_empty()), "{polls}");
    for t in polls["tabs"].as_array().unwrap().clone() {
        ok(g, "board_polls", json!({ "tab": t["k"] }));
    }
}

#[test]
fn college_boards_and_the_nil_budget() {
    let (mut g, path) = world("college");
    start(&mut g, &path, "college", "college_assistant");
    let k = play_to(&mut g, 2);
    kept_where_they_should_be(&g);
    boards(&mut g, k);
    // The NIL budget: the blocks are what goes to the roster, and the
    // budget splits into the roster, the promises and what's left.
    let cap = ok(&mut g, "board_cap", json!({}));
    assert_eq!(cap["level"], "college");
    let b = &cap["budget"];
    let blocks = cap["blocks"].as_array().unwrap();
    let sum: f64 = blocks.iter().map(|x| f(&x["hit"])).sum();
    // The blocks are the deals set, or the usual split when they show it.
    let shown = if b["usualRoster"].is_null() { f(&b["roster"]) } else { f(&b["usualRoster"]) };
    assert!((sum - shown).abs() <= blocks.len() as f64, "blocks {sum} against the roster's {shown}");
    // What's left is the deals desk's own figure.
    let parts = f(&b["roster"]) + f(&b["recruits"]) + f(&b["portal"]) + f(&b["left"]);
    assert!((parts - f(&b["total"])).abs() <= 2.0, "{b}");
    if !b["usualRoster"].is_null() {
        let parts = shown + f(&b["recruits"]) + f(&b["portal"]) + f(&b["leftUsual"]);
        assert!((parts - f(&b["total"])).abs() <= 2.0, "{b}");
    }
    let s = &cap["scholarships"];
    assert_eq!(s["onAid"].as_u64().unwrap() + s["walkOns"].as_u64().unwrap(), s["roster"].as_u64().unwrap());
    assert!(s["counted"].as_u64().unwrap() <= s["roster"].as_u64().unwrap());
}

#[test]
fn nfl_boards_and_the_cap() {
    let (mut g, path) = world("nfl");
    start(&mut g, &path, "nfl", "nfl_assistant");
    let k = play_to(&mut g, 2);
    kept_where_they_should_be(&g);
    boards(&mut g, k);
    // The cap: every block is a contract's hit this year; with the dead
    // money they are what's committed, and the space is what's left.
    let year = ok(&mut g, "board_cap", json!({}))["year"].as_i64().unwrap();
    for y in [year, year + 1] {
        let cap = ok(&mut g, "board_cap", json!({ "year": y }));
        assert_eq!(cap["level"], "nfl");
        let blocks = cap["blocks"].as_array().unwrap();
        let dead = cap["dead"].as_array().unwrap();
        // Until the first game only the top 51 count.
        let sum: f64 = blocks.iter().filter(|x| x["counts"] != false).map(|x| f(&x["hit"])).sum::<f64>()
            + dead.iter().map(|x| f(&x["amount"])).sum::<f64>();
        let committed = f(&cap["committed"]);
        assert!(
            (sum - committed).abs() <= (blocks.len() + dead.len()) as f64,
            "{y}: blocks and dead money {sum} against {committed} committed"
        );
        assert!(
            (f(&cap["cap"]) - committed - f(&cap["space"])).abs() <= 2.0,
            "{y}: the space is the cap less what's committed"
        );
        assert!(blocks.iter().all(|x| f(&x["hit"]) > 0.0));
    }
}
