//! A scripted season per seat (final build rule B6): start a career at a
//! high school, a college and an NFL club, Continue through a whole year,
//! open every screen at every stop (and the players, coaches, teams and
//! games those screens name), save and reload, and report every error and
//! every suspicious value ("NaN", "undefined", empty names, "later").
//!
//! Usage: seat_season [world file | "build"] [days] [seats, e.g. hs,college,nfl,other]
//! ("other" is a club in another league: the CFL, arena football or a
//! spring league.)
//! With "build" a small world is made first (seed 7, 2 years, scale 0.05).
//! `AFM_SEAT_OUT=report.json` writes the full report; `AFM_SEAT_SAVE_END=dir`
//! keeps each seat's save from the end of its run.

use std::collections::BTreeSet;
use std::time::Instant;

use afm_game::Game;
use serde_json::{Value, json};

struct Run {
    errors: Vec<String>,
    smells: BTreeSet<String>,
    calls: usize,
    slow: Vec<String>,
}

impl Run {
    fn call(&mut self, g: &mut Game, cmd: &str, args: Value) -> Option<Value> {
        self.calls += 1;
        let t = Instant::now();
        let r = g.call(cmd, &args);
        let ms = t.elapsed().as_millis();
        if ms > 1500 && !matches!(cmd, "continue" | "save" | "load" | "open_world" | "build_world" | "start_career") {
            self.slow.push(format!("{cmd} {args} took {ms} ms"));
        }
        match r {
            Ok(v) => {
                smell(cmd, &v, "", &mut self.smells);
                Some(v)
            }
            Err(e) => {
                self.errors.push(format!("{cmd} {args}: {e}"));
                None
            }
        }
    }
}

/// The draft in your war room, as a person runs it: your first pick from
/// the top of your board, the staff's after that, a trade call taken if one
/// comes, then the undrafted rush.
fn draft(run: &mut Run, g: &mut Game) {
    let mut first = true;
    let mut traded = false;
    for _ in 0..400 {
        let Some(room) = run.call(g, "draft_live", json!({})) else { return };
        if !room["live"].as_bool().unwrap_or(false) {
            return;
        }
        if room["rush"].as_bool().unwrap_or(false) {
            run.call(g, "draft_rush", json!({}));
            run.call(g, "draft_record", json!({}));
            return;
        }
        if !room["on_clock"].as_bool().unwrap_or(false) {
            run.call(g, "draft_go", json!({ "until": "yours" }));
        } else if first && let Some(id) = room["board"][0]["id"].as_u64() {
            first = false;
            run.call(g, "draft_pick", json!({ "id": id }));
        } else if !traded && room["offers"].as_array().is_some_and(|o| !o.is_empty()) {
            traded = true;
            run.call(g, "draft_accept", json!({ "offer": 0 }));
        } else {
            run.call(g, "draft_auto", json!({}));
        }
    }
    run.errors.push("the draft didn't finish in 400 steps".into());
}

/// Values a player should never see.
fn smell(cmd: &str, v: &Value, path: &str, out: &mut BTreeSet<String>) {
    match v {
        Value::String(s) => {
            let low = s.to_lowercase();
            for bad in ["nan", "undefined", "infinity", "todo", "placeholder", "lorem", "built later", "coming soon"] {
                // Whole words for these two: "Loreman" is a real surname.
                let hit = if bad == "nan" || bad == "lorem" {
                    low.split(|c: char| !c.is_alphanumeric()).any(|w| w == bad)
                } else {
                    low.contains(bad)
                };
                if hit {
                    out.insert(format!("{cmd}{path}: \"{}\"", s.chars().take(120).collect::<String>()));
                }
            }
            if (path.ends_with(".name") || path.ends_with(".text")) && s.trim().is_empty() {
                out.insert(format!("{cmd}{path}: empty"));
            }
        }
        Value::Number(n) => {
            if n.as_f64().is_some_and(|x| !x.is_finite()) {
                out.insert(format!("{cmd}{path}: not finite"));
            }
        }
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate().take(400) {
                smell(cmd, x, &format!("{path}[{}]", if i < 3 { i.to_string() } else { "*".into() }), out);
            }
        }
        Value::Object(o) => {
            for (k, x) in o {
                smell(cmd, x, &format!("{path}.{k}"), out);
            }
        }
        _ => {}
    }
}

/// Every id-looking reference in a reply: players, coaches, teams.
fn refs(v: &Value, players: &mut BTreeSet<u64>, coaches: &mut BTreeSet<u64>, teams: &mut BTreeSet<(String, u64)>) {
    match v {
        Value::Object(o) => {
            if let (Some(Value::String(k)), Some(Value::Number(i))) = (o.get("kind"), o.get("id"))
                && matches!(k.as_str(), "nfl" | "college" | "hs" | "other")
            {
                teams.insert((k.clone(), i.as_u64().unwrap_or(0)));
            }
            if let Some(Value::String(go)) = o.get("go").or(o.get("link")) {
                let id = o.get("id").and_then(Value::as_u64);
                match (go.as_str(), id) {
                    ("player", Some(i)) => {
                        players.insert(i);
                    }
                    ("coach", Some(i)) => {
                        coaches.insert(i);
                    }
                    _ => {}
                }
            }
            for (k, x) in o {
                if let Some(i) = x.as_u64() {
                    match k.as_str() {
                        "player_id" | "pid" => {
                            players.insert(i);
                        }
                        "coach_id" | "cid" => {
                            coaches.insert(i);
                        }
                        _ => {}
                    }
                }
                refs(x, players, coaches, teams);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| refs(x, players, coaches, teams)),
        _ => {}
    }
}

fn resident_gb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1).and_then(|x| x.parse::<f64>().ok()))
        })
        .map(|kb| kb / 1024.0 / 1024.0)
        .unwrap_or(0.0)
}

/// The screens every stop opens, with no arguments (your own team).
const SCREENS: &[&str] = &[
    "status",
    "home",
    "inbox",
    "news",
    "calendar",
    "team",
    "roster",
    "schedule",
    "standings",
    "staff",
    "depth",
    "training",
    "practice",
    "plans",
    "designer",
    "squad",
    "boards",
    "recruiting",
    "contracts",
    "games",
    "you",
    "card",
    "money_books",
    "money_league",
];

fn visit_all(run: &mut Run, g: &mut Game, extra: &[(String, Value)]) {
    let mut players = BTreeSet::new();
    let mut coaches = BTreeSet::new();
    let mut teams = BTreeSet::new();
    for s in SCREENS {
        if let Some(v) = run.call(g, s, json!({})) {
            refs(&v, &mut players, &mut coaches, &mut teams);
            if *s == "roster" {
                for r in v["rows"].as_array().into_iter().flatten() {
                    if let Some(i) = r["id"].as_u64() {
                        players.insert(i);
                    }
                }
            }
            if *s == "staff" {
                for r in v["rows"].as_array().into_iter().flatten() {
                    if let Some(i) = r["id"].as_u64() {
                        coaches.insert(i);
                    }
                }
            }
            if *s == "games" {
                let n = v["games"].as_array().map_or(0, Vec::len);
                if n > 0 {
                    run.call(g, "game", json!({ "index": n - 1 }));
                    // The game's film (lane 13): its index, a dozen snaps,
                    // the grade sheet, then the film room closes.
                    if let Some(f) = run.call(g, "film", json!({ "index": n - 1 })) {
                        let ns: Vec<Value> =
                            f["plays"].as_array().into_iter().flatten().step_by(12).map(|p| p["n"].clone()).collect();
                        run.call(g, "film_plays", json!({ "index": n - 1, "n": ns }));
                        run.call(g, "film_grades", json!({ "index": n - 1 }));
                        run.call(g, "film_close", json!({}));
                    }
                }
            }
        }
    }
    for (cmd, args) in extra {
        if let Some(v) = run.call(g, cmd, args.clone()) {
            refs(&v, &mut players, &mut coaches, &mut teams);
        }
    }
    run.call(g, "news", json!({ "mine": true }));
    run.call(g, "search", json!({ "q": "a" }));
    run.call(g, "money_league", json!({ "level": "college" }));
    for &p in players.iter().take(12) {
        run.call(g, "player", json!({ "id": p }));
    }
    // Your own players' Development tab (lane 11).
    if let Some(sq) = run.call(g, "squad", json!({})) {
        for r in sq["rows"].as_array().into_iter().flatten().take(6) {
            if let Some(i) = r["id"].as_u64() {
                run.call(g, "player_dev", json!({ "id": i }));
            }
        }
    }

    for &c in coaches.iter().take(6) {
        run.call(g, "coach", json!({ "id": c }));
    }
    // The other leagues (X7): every league's page, and their clubs named
    // on the screens (a club's page is `league_team`).
    for k in 0..afm_engine::world::pro::other::LEAGUE_COUNT {
        run.call(g, "leagues", json!({ "league": k }));
    }
    for (k, i) in teams.iter().filter(|t| t.0 != "other").take(6) {
        for s in ["team", "roster", "schedule", "staff", "money_books"] {
            run.call(g, s, json!({ "kind": k, "id": i }));
        }
    }
    for (_, i) in teams.iter().filter(|t| t.0 == "other").take(4) {
        run.call(g, "league_team", json!({ "id": i }));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let world = args.get(1).cloned().unwrap_or_else(|| "build".into());
    let days: i32 = args.get(2).and_then(|x| x.parse().ok()).unwrap_or(365);
    let seats: Vec<String> =
        args.get(3).cloned().unwrap_or_else(|| "hs,college,nfl".into()).split(',').map(String::from).collect();
    // Extra commands to open at every stop: AFM_SEAT_EXTRA='cmd1,cmd2?k=v&k2=v2'
    // (arguments after a question mark, numbers as numbers).
    let extra: Vec<(String, Value)> = std::env::var("AFM_SEAT_EXTRA")
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let (cmd, q) = s.split_once('?').unwrap_or((s, ""));
            let mut a = serde_json::Map::new();
            for kv in q.split('&').filter(|x| !x.is_empty()) {
                let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
                let v = v.replace("%20", " ");
                a.insert(k.to_string(), v.parse::<i64>().map_or_else(|_| json!(v), |n| json!(n)));
            }
            (cmd.to_string(), Value::Object(a))
        })
        .collect();
    let dir = std::env::temp_dir().join(format!("afm-seat-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    let t = Instant::now();
    let fresh = if world == "build" {
        g.call("build_world", &json!({ "seed": 7, "history_years": 2, "scale": 0.05 })).expect("build");
        let p = g.call("save", &json!({ "name": "fresh" })).expect("save")["saved"].as_str().unwrap().to_string();
        g.save_path = None;
        // Keep the small world for later runs: AFM_SEAT_KEEP=path.
        if let Ok(keep) = std::env::var("AFM_SEAT_KEEP") {
            let _ = std::fs::copy(&p, &keep);
        }
        p
    } else {
        world
    };
    println!("world ready in {:.0}s", t.elapsed().as_secs_f64());
    let mut report = vec![];
    let mut total_errors = 0;
    for seat in &seats {
        let mut run = Run { errors: vec![], smells: BTreeSet::new(), calls: 0, slow: vec![] };
        let t = Instant::now();
        if let Err(e) = g.call("open_world", &json!({ "path": fresh })) {
            println!("{seat}: can't open the world: {e}");
            total_errors += 1;
            continue;
        }
        let level = if seat == "hs" { "hs" } else { seat.as_str() };
        let jobs = run.call(&mut g, "jobs", json!({ "level": level })).unwrap_or(json!({}));
        let Some(job) = jobs["jobs"].as_array().and_then(|a| a.get(a.len().min(3).saturating_sub(1))).cloned() else {
            println!("{seat}: no jobs listed");
            total_errors += 1;
            continue;
        };
        let bg = match seat.as_str() {
            "hs" => "hs_coach",
            "nfl" | "other" => "nfl_assistant",
            _ => "college_assistant",
        };
        let start = json!({ "name": "Seat Tester", "age": 44, "background": bg, "job": job["ref"], "seed": 3 });
        if run.call(&mut g, "start_career", start.clone()).is_none() {
            // The background key may differ: fall back to the default one.
            let mut s = start;
            s["background"] = json!("college_assistant");
            run.call(&mut g, "start_career", s);
        }
        visit_all(&mut run, &mut g, &extra);
        let begin = g.world.as_ref().map(|w| w.today).unwrap();
        let mut stops = 0;
        let mut longest = 0.0f64;
        let mut longest_span = String::new();
        let mut peak = resident_gb();
        let mut saved_once = false;
        while g.world.as_ref().map(|w| begin.days_to(w.today)).unwrap_or(days) < days && stops < 600 {
            let tc = Instant::now();
            let from = g.world.as_ref().map(|w| w.today.pretty()).unwrap_or_default();
            let r = run.call(&mut g, "continue", json!({ "max_days": 60 }));
            if tc.elapsed().as_secs_f64() > longest {
                longest = tc.elapsed().as_secs_f64();
                let to = g.world.as_ref().map(|w| w.today.pretty()).unwrap_or_default();
                longest_span = format!("{from} to {to}");
            }
            stops += 1;
            peak = peak.max(resident_gb());
            if r.is_none() {
                break;
            }
            // The draft live from your chair (lane 7): Continue stops in the war room.
            if r.as_ref().is_some_and(|v| v["studio"]["screen"] == json!("warroom")) {
                visit_all(&mut run, &mut g, &extra);
                draft(&mut run, &mut g);
            }
            visit_all(&mut run, &mut g, &extra);
            // Save and reload once in season, after the first game.
            let played = g.world.as_ref().map(|w| !w.seat_games().is_empty()).unwrap_or(false);
            if played && !saved_once {
                saved_once = true;
                let before = run.call(&mut g, "status", json!({}));
                if let Some(s) = run.call(&mut g, "save", json!({ "name": format!("seat {seat}") })) {
                    let path = s["saved"].as_str().unwrap_or_default().to_string();
                    run.call(&mut g, "load", json!({ "path": path }));
                    let after = run.call(&mut g, "status", json!({}));
                    if before != after {
                        run.errors.push(format!("status changed across save and load: {before:?} vs {after:?}"));
                    }
                }
            }
        }
        if !saved_once {
            run.errors.push("no game of yours was played in the run, so save and load was not tested".into());
        }
        // The stories wrote your year (lane 8): your hire and games on the
        // timeline, and your office opened.
        if let Some(v) = run.call(&mut g, "stories_story", json!({}))
            && v["entries"].as_array().is_none_or(|a| a.is_empty())
        {
            run.errors.push("your career story has no entries after the run".into());
        }
        // Keep each seat's save from the end of the run for a look in the
        // browser: AFM_SEAT_SAVE_END=dir.
        if let Ok(d) = std::env::var("AFM_SEAT_SAVE_END")
            && let Some(s) = run.call(&mut g, "save", json!({ "name": format!("seat {seat} end") }))
        {
            let to = std::path::Path::new(&d).join(format!("{seat}-end.afmsave"));
            let _ = std::fs::copy(s["saved"].as_str().unwrap_or_default(), to);
        }
        let secs = t.elapsed().as_secs_f64();
        // Your games in the run and how they went: a season with too few
        // games is a bug the call checks alone don't see.
        let (games, won, lost) = g.world.as_ref().map_or((0, 0, 0), |w| {
            let Some((_, me)) = w.seat_team() else { return (0, 0, 0) };
            let mine: Vec<_> = w.seat_games().iter().filter(|x| x.date.0 >= begin.0).collect();
            let won = mine
                .iter()
                .filter(|x| {
                    let side = usize::from(x.teams[1] == me);
                    x.stats.score[side] > x.stats.score[1 - side]
                })
                .count();
            let lost = mine
                .iter()
                .filter(|x| {
                    let side = usize::from(x.teams[1] == me);
                    x.stats.score[side] < x.stats.score[1 - side]
                })
                .count();
            (mine.len(), won, lost)
        });
        println!(
            "\n== {seat}: {} at {}: {stops} stops, {} calls, {secs:.0}s, longest Continue {longest:.1}s ({longest_span}), resident peak {peak:.2} GB; your games {games} ({won}-{lost})",
            job["name"].as_str().unwrap_or("?"),
            jobs["level"].as_str().unwrap_or(level),
            run.calls
        );
        println!("errors: {}", run.errors.len());
        for e in run.errors.iter().take(40) {
            println!("  ERR {e}");
        }
        println!("smells: {}", run.smells.len());
        for s in run.smells.iter().take(40) {
            println!("  SMELL {s}");
        }
        for s in run.slow.iter().take(10) {
            println!("  SLOW {s}");
        }
        total_errors += run.errors.len();
        report
            .push(json!({ "seat": seat, "stops": stops, "calls": run.calls, "secs": secs, "longest_continue": longest, "longest_span": longest_span,
            "peak_gb": peak, "games": games, "won": won, "lost": lost, "errors": run.errors, "smells": run.smells, "slow": run.slow }));
    }
    if let Ok(p) = std::env::var("AFM_SEAT_OUT") {
        let _ = std::fs::write(p, serde_json::to_string_pretty(&report).unwrap_or_default());
    }
    let _ = std::fs::remove_dir_all(dir);
    println!("\ntotal errors: {total_errors}");
    std::process::exit(i32::from(total_errors > 0));
}
