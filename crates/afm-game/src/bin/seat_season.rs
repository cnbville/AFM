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
//! keeps each seat's save from the end of its run. `AFM_SEAT_LEAN=1` only
//! plays (no screens at the stops), for long runs; `AFM_SEAT_TEAM=Allen`
//! takes that team's job instead of the third on the list; `AFM_SEAT_STORIES=1`
//! prints your team's stories by family per season and the lines that
//! reached the same person twice in a season (the storylines' design
//! checks).

use std::collections::BTreeSet;
use std::time::Instant;

use afm_game::Game;
use serde_json::{Value, json};

struct Run {
    errors: Vec<String>,
    smells: BTreeSet<String>,
    calls: usize,
    slow: Vec<String>,
    /// Stops that looked at finding players: turns the fields tried.
    finds: usize,
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

/// Your team's stories by family and season, the world's by family, and
/// the lines that reached the same person twice in a season.
/// `since`: the run's first day; the line checks read only what the run
/// itself told (a save's history may come from an older build).
fn story_counts(w: &afm_engine::world::World, seat: &str, since: afm_engine::calendar::Date) {
    use afm_engine::world::storylines::{BeatKind, Family, Who, catalogue};
    use std::collections::BTreeMap;
    let me = afm_engine::world::stories::seat_team(w);
    let all: Vec<_> = w.storylines.open.iter().chain(&w.storylines.ended).collect();
    let mut seasons: BTreeMap<i32, BTreeMap<&str, (u32, u32)>> = BTreeMap::new();
    for s in all.iter().filter(|s| s.you.is_some() || me.is_some_and(|t| s.about(t))) {
        let e = seasons.entry(s.season).or_default().entry(s.family().name()).or_default();
        e.0 += 1;
        e.1 += u32::from(s.public);
    }
    println!("\n{seat}: your team's stories by season (opened, of them public)");
    for (y, fams) in &seasons {
        let n: u32 = fams.values().map(|x| x.0).sum();
        let row: Vec<String> = Family::ALL
            .iter()
            .filter_map(|f| fams.get(f.name()).map(|x| format!("{} {}/{}", f.name(), x.0, x.1)))
            .collect();
        println!("  {y}: {n} stories: {}", row.join(", "));
    }
    let mut world: BTreeMap<i32, BTreeMap<&str, (u32, u32)>> = BTreeMap::new();
    for &(y, kind, opened, public) in &w.storylines.counts {
        let e = world.entry(y).or_default().entry(catalogue::def(kind).family.name()).or_default();
        e.0 += opened;
        e.1 += public;
    }
    println!("{seat}: the world's stories by season (opened/public)");
    for (y, fams) in &world {
        let (n, p): (u32, u32) = fams.values().fold((0, 0), |a, x| (a.0 + x.0, a.1 + x.1));
        let row: Vec<String> = fams.iter().map(|(f, x)| format!("{f} {}/{}", x.0, x.1)).collect();
        println!("  {y}: {n} opened, {p} public: {}", row.join(", "));
    }
    // A bank line told twice to the same person in one season.
    let told = |b: &BeatKind| {
        !matches!(b, BeatKind::Fact | BeatKind::Move | BeatKind::Private | BeatKind::Denial | BeatKind::Seed)
    };
    let mut seen: std::collections::HashMap<(Who, i32), Vec<&str>> = std::collections::HashMap::new();
    let (mut twice, mut lines) = (Vec::new(), 0);
    for s in &all {
        let Some(lead) = s.lead() else { continue };
        for b in s.beats.iter().filter(|b| told(&b.what) && !b.text.is_empty() && b.date.0 >= since.0) {
            lines += 1;
            let season = if b.date.month() <= 2 { b.date.year() - 1 } else { b.date.year() };
            let v = seen.entry((lead, season)).or_default();
            if v.contains(&b.text.as_str()) {
                twice.push(format!("{season} {}: {}", s.def().key, b.text));
            } else {
                v.push(&b.text);
            }
        }
    }
    println!("{seat}: {lines} told lines, {} reached the same person twice in a season", twice.len());
    for t in twice.iter().take(12) {
        println!("  TWICE {}", t.chars().take(160).collect::<String>());
    }
    // The same line twice inside one story.
    let mut again = Vec::new();
    for s in &all {
        let mut v: Vec<&str> = Vec::new();
        for b in s.beats.iter().filter(|b| told(&b.what) && !b.text.is_empty() && b.date.0 >= since.0) {
            if v.iter().any(|t| *t == b.text) {
                again.push(format!("{} #{}: {}", s.def().key, s.id, b.text));
            }
            v.push(&b.text);
        }
    }
    println!("{seat}: {} lines repeated inside one story", again.len());
    for t in again.iter().take(6) {
        println!("  AGAIN {}", t.chars().take(160).collect::<String>());
    }
    // How long stories run, and how they end, by family.
    let today = w.today;
    let open: Vec<_> = w.storylines.open.iter().filter(|s| s.phase.open()).collect();
    let mut ages: Vec<i32> = open.iter().map(|s| s.opened.days_to(today)).collect();
    ages.sort_unstable();
    let q = |p: f64| ages.get(((ages.len() as f64 - 1.0) * p).round() as usize).copied().unwrap_or(0);
    println!(
        "{seat}: {} open stories, age in days: median {}, 90th {}, max {}; {} older than a year",
        open.len(),
        q(0.5),
        q(0.9),
        q(1.0),
        ages.iter().filter(|&&a| a > 365).count()
    );
    let mut old: Vec<_> = open.iter().filter(|s| s.opened.days_to(today) > 365).collect();
    old.sort_by_key(|s| s.opened);
    for s in old.iter().take(6) {
        println!(
            "  OLD {} #{} since {}: {} beats, heat {:.0}, {}",
            s.def().key,
            s.id,
            s.opened.pretty(),
            s.beats.len(),
            s.heat,
            afm_engine::world::storylines::view::question(w, s)
        );
    }
    let mut ends: BTreeMap<&str, (u32, u32, i64)> = BTreeMap::new();
    for s in all.iter().filter(|s| !s.phase.open()) {
        let e = ends.entry(s.family().name()).or_default();
        if s.phase == afm_engine::world::storylines::Phase::Fizzled {
            e.1 += 1;
        } else {
            e.0 += 1;
        }
        e.2 += i64::from(s.opened.days_to(s.closed.unwrap_or(today)));
    }
    let row: Vec<String> =
        ends.iter().map(|(f, x)| format!("{f} {}/{} ({}d)", x.0, x.1, x.2 / i64::from((x.0 + x.1).max(1)))).collect();
    println!("{seat}: ended by family (answered/fizzled, mean days): {}", row.join(", "));
    // Questions asking for a name the story can't fill, by kind and name
    // (stories from the run itself; the world file's older ones apart).
    let mut gaps: std::collections::BTreeMap<String, (u32, u32)> = std::collections::BTreeMap::new();
    let mut older = 0;
    for s in &all {
        let g = afm_engine::world::storylines::tell::question_gaps(w, s);
        if g.is_empty() {
            continue;
        }
        if s.opened < since {
            older += 1;
            continue;
        }
        let e = gaps.entry(format!("{} needs {}", s.def().key, g.join(", "))).or_insert((0, s.id));
        e.0 += 1;
    }
    let total: u32 = gaps.values().map(|x| x.0).sum();
    println!("{seat}: {total} stories from the run ask for a name they can't fill ({older} from before it)");
    let mut by: Vec<_> = gaps.into_iter().collect();
    by.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(&b.0)));
    for (k, (n, id)) in by.iter().take(10) {
        println!("  GAP {k}: {n} (e.g. #{id})");
    }
}

/// Values a player should never see.
fn smell(cmd: &str, v: &Value, path: &str, out: &mut BTreeSet<String>) {
    match v {
        Value::String(s) => {
            let low = s.to_lowercase();
            for bad in ["nan", "undefined", "infinity", "todo", "placeholder", "lorem", "built later", "coming soon"] {
                // Whole words for these: "Loreman" and "Todora" are real surnames.
                let hit = if matches!(bad, "nan" | "lorem" | "todo") {
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
            // A meeting's narration (`by` 0) has no speaker's name.
            let narration = o.get("by").and_then(Value::as_u64) == Some(0);
            for (k, x) in o {
                if narration && k == "name" {
                    continue;
                }
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

/// Screens a club in another league doesn't have: it practises on its own
/// staff's plan and runs its staff's playbook (its sidebar leaves them out).
const NOT_OTHER: &[&str] = &["training", "practice", "plans", "designer", "squad", "boards", "recruiting", "contracts"];

fn visit_all(run: &mut Run, g: &mut Game, extra: &[(String, Value)], seat: &str) {
    let mut players = BTreeSet::new();
    let mut coaches = BTreeSet::new();
    let mut teams = BTreeSet::new();
    for s in SCREENS {
        if seat == "other" && NOT_OTHER.contains(s) {
            continue;
        }
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
    // The storylines (update 2, lane 1): this week, every story it lists,
    // the season's chapters, your career's, the jobs and "Previously".
    if let Some(v) = run.call(g, "storylines_week", json!({})) {
        let ids: BTreeSet<u64> = ["mine", "following", "ended", "others"]
            .iter()
            .flat_map(|k| v[*k].as_array().into_iter().flatten().filter_map(|c| c["id"].as_u64()))
            .collect();
        for &id in ids.iter().take(10) {
            if let Some(st) = run.call(g, "storylines_story", json!({ "id": id })) {
                refs(&st, &mut players, &mut coaches, &mut teams);
            }
        }
    }
    for cmd in ["storylines_chapters", "storylines_career", "storylines_jobs", "storylines_previously"] {
        run.call(g, cmd, json!({}));
    }
    // The chases (update 2, lane 2): your legacy, your level's five, the
    // strip their screens show and the ambition picker, answered the way a
    // player would: the first three, the first said out loud.
    if seat != "other" {
        for cmd in ["legacy_page", "legacy_chases", "legacy_strip"] {
            if let Some(v) = run.call(g, cmd, json!({})) {
                refs(&v, &mut players, &mut coaches, &mut teams);
            }
        }
        if let Some(p) = run.call(g, "legacy_ambitions", json!({}))
            && p["open"] == json!(true)
        {
            let room = p["max"].as_u64().unwrap_or(3) as usize - p["held"].as_array().map_or(0, Vec::len);
            let ids: Vec<u64> =
                p["offers"].as_array().into_iter().flatten().filter_map(|o| o["id"].as_u64()).take(room).collect();
            for (k, id) in ids.iter().enumerate() {
                run.call(g, "legacy_pick", json!({ "id": id, "on": true }));
                if k == 0 {
                    run.call(g, "legacy_loud", json!({ "id": id, "loud": true }));
                }
            }
            run.call(g, "legacy_keep", json!({ "later": false }));
        }
    }
    // Your staff and the ranges (update 2, lane 3): the quality page, each
    // member's detail and ratings page with his key reads' cards, and the
    // cards for a few of your players' ranges.
    if let Some(q) = run.call(g, "staff_quality", json!({}))
        && q["none"] != json!(true)
    {
        let rows: Vec<Value> = q["depts"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|d| d["rows"].as_array().cloned().unwrap_or_default())
            .collect();
        for r in rows.iter().take(40) {
            let who = r["who"].clone();
            run.call(g, "staff_member", json!({ "who": who }));
            run.call(g, "coach_ratings", json!({ "who": who }));
            for f in r["reads"].as_array().into_iter().flatten() {
                if let Some(n) = f["now"].as_array() {
                    run.call(g, "rr_staff", json!({ "who": who, "k": f["key"], "lo": n[0], "hi": n[1] }));
                }
            }
        }
        if let Some(ro) = run.call(g, "roster", json!({})) {
            for p in ro["rows"].as_array().into_iter().flatten().take(6) {
                if let Some(o) = p["ovr"].as_array() {
                    for k in ["ovr", "speed", "strength", "agility", "jumping", "playRecognition"] {
                        run.call(g, "rr_card", json!({ "p": p["id"], "k": k, "lo": o[0], "hi": o[1] }));
                    }
                }
            }
        }
    }
    // Finding players (update 2, lane 7): the seat's search, its filters,
    // sorts, chart, Compare and drawers; a high school's Colleges calling.
    for v in find_stops(run, g, seat) {
        refs(&v, &mut players, &mut coaches, &mut teams);
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
    if let Some(sq) = if seat == "other" { None } else { run.call(g, "squad", json!({})) } {
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

/// One stop's look at finding players (update 2, lane 7): the default
/// search, then seven of its fields as filters (alone with "not seen" both
/// ways, and joined by "or" to the default's) and as sorts both ways, the
/// other classes and levels, the chart, Compare, the offers and insider
/// drawers, and one saved search kept for the inbox (forgotten and saved
/// again now and then). The fields turn from stop to stop, so a season
/// tries every one. A high school coach opens Colleges calling.
fn find_stops(run: &mut Run, g: &mut Game, seat: &str) -> Vec<Value> {
    let cmd = match seat {
        "college" => "find_recruits",
        "nfl" => "find_prospects",
        "hs" => {
            // Colleges calling, its map's data, and the races of two of
            // your players with offers (board 8).
            let mut out: Vec<Value> = run.call(g, "colleges_calling", json!({})).into_iter().collect();
            let ids: Vec<u64> = out
                .first()
                .and_then(|p| p["players"].as_array())
                .into_iter()
                .flatten()
                .filter(|p| p["offers"].as_u64().unwrap_or(0) > 0)
                .filter_map(|p| p["id"].as_u64())
                .take(2)
                .collect();
            for id in ids {
                out.extend(run.call(g, "recruit_race", json!({ "id": id })));
            }
            return out;
        }
        _ => return Vec::new(),
    };
    let Some(page) = run.call(g, cmd, json!({})) else { return Vec::new() };
    let fields: Vec<Value> = page["catalog"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|c| c["fields"].as_array().cloned().unwrap_or_default())
        .collect();
    let mut out = Vec::new();
    run.finds += 1;
    let defaults: Vec<Value> = page["filters"].as_array().cloned().unwrap_or_default();
    for n in 0..fields.len().min(7) {
        let f = &fields[(run.finds * 7 + n) % fields.len()];
        let k = f["k"].clone();
        let filter = match f["kind"].as_str() {
            Some("num") if n % 2 == 0 => json!({ "k": k, "op": "ge", "v": 0 }),
            Some("num") => json!({ "k": k, "op": "between", "v": -1000, "v2": 100_000 }),
            Some("word") => json!({ "k": k, "op": "ge", "v": 1 }),
            Some("flag") => json!({ "k": k, "op": "is", "v": if n % 2 == 0 { "yes" } else { "no" } }),
            _ => json!({ "k": k, "op": "is", "v": "a" }),
        };
        run.call(g, cmd, json!({ "filters": [filter.clone()], "unseen": n % 2 == 0 }));
        let mut joined = defaults.clone();
        let mut or = filter.clone();
        or["or"] = json!(true);
        joined.push(or);
        run.call(g, cmd, json!({ "filters": joined, "page": 1 }));
        if f["kind"] != json!("text") {
            run.call(g, cmd, json!({ "filters": [], "sort": k, "dir": if n % 2 == 0 { 1 } else { -1 } }));
        }
    }
    for c in page["classes"].as_array().into_iter().flatten().skip(1) {
        out.extend(run.call(g, cmd, json!({ "class": c, "filters": [] })));
    }
    let levels: &[&str] = if seat == "nfl" { &["fbs", "fcs", "small"] } else { &["juco", "portal"] };
    for l in levels {
        run.call(g, cmd, json!({ "level": l }));
    }
    let nums: Vec<&Value> = fields.iter().filter(|f| f["kind"] == json!("num")).map(|f| &f["k"]).collect();
    let rows: Vec<Value> = page["rows"].as_array().cloned().unwrap_or_default();
    let ids: Vec<u64> = rows.iter().filter_map(|r| r["id"].as_u64()).take(3).collect();
    if nums.len() > 1 {
        let x = nums[run.finds % nums.len()];
        let y = nums[(run.finds + 1) % nums.len()];
        run.call(g, cmd, json!({ "filters": [], "x": x, "y": y, "compare": ids, "picked": ids }));
    }
    if let Some(r) = rows.iter().find(|r| r["offers"].as_u64().unwrap_or(0) > 0) {
        out.extend(run.call(g, "find_offers", json!({ "id": r["id"] })));
    }
    // The recruiting map and three recruits' races (boards 7 and 8).
    if seat == "college" {
        out.extend(run.call(g, "recruit_map", json!({})));
        let board: Vec<u64> = run
            .call(g, "recruit_board", json!({}))
            .and_then(|b| b["rows"].as_array().cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(|r| r["id"].as_u64())
            .skip(run.finds % 5)
            .take(3)
            .collect();
        for id in board {
            out.extend(run.call(g, "recruit_race", json!({ "id": id })));
        }
    }
    if let Some(k) = page["insiders"].as_array().and_then(|d| d.first()).map(|d| d["k"].clone()) {
        out.extend(run.call(g, "find_insider", json!({ "desk": k })));
    }
    match page["saved"].as_array().and_then(|s| s.first()).map(|s| s["k"].clone()) {
        Some(k) if run.finds % 10 == 5 => {
            run.call(g, "find_forget", json!({ "k": k }));
        }
        Some(k) => {
            run.call(g, cmd, json!({ "saved": k }));
        }
        None => {
            run.call(g, "find_save", json!({ "name": "Seat check", "spec": page["spec"] }));
        }
    }
    out.push(page);
    out
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
    let lean = std::env::var_os("AFM_SEAT_LEAN").is_some();
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
        let mut run = Run { errors: vec![], smells: BTreeSet::new(), calls: 0, slow: vec![], finds: 0 };
        let t = Instant::now();
        if let Err(e) = g.call("open_world", &json!({ "path": fresh })) {
            println!("{seat}: can't open the world: {e}");
            total_errors += 1;
            continue;
        }
        let level = if seat == "hs" { "hs" } else { seat.as_str() };
        let jobs = run.call(&mut g, "jobs", json!({ "level": level })).unwrap_or(json!({}));
        // A team by name (AFM_SEAT_TEAM=Allen), else the third on the list.
        let named = std::env::var("AFM_SEAT_TEAM").ok().and_then(|n| {
            jobs["jobs"].as_array().and_then(|a| a.iter().find(|j| j["name"].as_str() == Some(n.as_str())).cloned())
        });
        let Some(job) =
            named.or_else(|| jobs["jobs"].as_array().and_then(|a| a.get(a.len().min(3).saturating_sub(1))).cloned())
        else {
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
        if !lean {
            visit_all(&mut run, &mut g, &extra, seat);
        }
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
                if !lean {
                    visit_all(&mut run, &mut g, &extra, seat);
                }
                draft(&mut run, &mut g);
            }
            // A story meeting stopped Continue (storylines): read it, then
            // answer as the screen's first pick would (lean runs too, so a
            // stop never waits).
            if let Some(id) = r.as_ref().and_then(|v| v["story"].as_u64())
                && let Some(m) = run.call(&mut g, "storylines_meeting", json!({ "id": id }))
                && let Some(code) = m["opts"][0]["code"].as_u64()
            {
                run.call(&mut g, "people_answer", json!({ "id": id, "code": code }));
            }
            // One move a stop in your hottest story that has one, the way a
            // player would answer from its page.
            if let Some(v) = run.call(&mut g, "storylines_week", json!({})) {
                let first = v["mine"].as_array().into_iter().flatten().find_map(|c| c["id"].as_u64());
                if let Some(id) = first
                    && let Some(st) = run.call(&mut g, "storylines_story", json!({ "id": id }))
                    && let Some(code) = st["moves"][0]["code"].as_u64()
                {
                    run.call(&mut g, "storylines_move", json!({ "id": id, "code": code }));
                }
            }
            if lean {
                continue;
            }
            visit_all(&mut run, &mut g, &extra, seat);
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
        if !saved_once && !lean {
            run.errors.push("no game of yours was played in the run, so save and load was not tested".into());
        }
        if std::env::var_os("AFM_SEAT_STORIES").is_some()
            && let Some(w) = g.world.as_ref()
        {
            story_counts(w, seat, begin);
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
