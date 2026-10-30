//! The coach page (world reader lane R7), on R5's entity template: who he
//! is in plain words, his hidden ratings (debug), every job he held as
//! lanes over the years, every job's why (how he got it and how it ended,
//! from the news, the world log's searches and reviews and his own job
//! record), his coaching tree up and down, his unit records and buzz.

use std::collections::HashMap;

use serde_json::{Map, Value, json};

use crate::book::{Book, Filters, json_sql, q};
use crate::inst::{
    GROUPS, capital, cell, coach_name, has, i, kind_words, n, news_of, pct, program_name, record, rows, s,
};

/// An employer's name and page: `College` 12 → the program's.
pub fn employer(b: &Book, kind: &str, idx: i64) -> (String, String) {
    match kind {
        "College" => (program_name(b, idx), format!("e/programs/{idx}")),
        "HighSchool" => (
            b.name_of("high_schools", &json!(idx)).unwrap_or_else(|| format!("High school {idx}")),
            format!("e/high_schools/{idx}"),
        ),
        "Nfl" => {
            let t = ["nfl", "clubs"].into_iter().find(|t| has(b, t));
            let name = t.and_then(|t| b.name_of(t, &json!(idx))).unwrap_or_else(|| format!("NFL club {idx}"));
            (name, t.map_or(String::new(), |t| format!("e/{t}/{idx}")))
        }
        _ => (format!("Other league team {idx}"), String::new()),
    }
}

/// A role in words: `HeadCoach` → "head coach", `PositionCoach` with its
/// group.
pub fn role_words(role: &str) -> String {
    match role {
        "HeadCoach" => "head coach".into(),
        "OffensiveCoordinator" => "offensive coordinator".into(),
        "DefensiveCoordinator" => "defensive coordinator".into(),
        r => match r.strip_prefix("PositionCoach:") {
            Some(g) if g.len() <= 3 => format!("{g} coach"),
            Some(g) => format!("{} coach", g.to_lowercase()),
            None => kind_words(r),
        },
    }
}

/// A job's role from its record: `role`, or a position coach's group
/// (`role__PositionCoach`) as "PositionCoach:DL".
fn role_of(r: &Map<String, Value>, prefix: &str) -> String {
    let role = s(r.get(&format!("{prefix}role")));
    if !role.is_empty() {
        return role;
    }
    let g = s(r.get(&format!("{prefix}role__PositionCoach")));
    if g.is_empty() { String::new() } else { format!("PositionCoach:{g}") }
}

/// The tone of a role's band.
fn role_tone(role: &str) -> &'static str {
    match role {
        "HeadCoach" => "head",
        "OffensiveCoordinator" => "oc",
        "DefensiveCoordinator" => "dc",
        _ => "pc",
    }
}

/// The employer of a job record: its kind and index from the
/// `employer__<Kind>` (or `job__employer__<Kind>`) columns.
fn emp_of(r: &Map<String, Value>, prefix: &str) -> Option<(String, i64)> {
    ["College", "HighSchool", "Nfl", "Other"]
        .into_iter()
        .find_map(|k| i(r.get(&format!("{prefix}employer__{k}"))).map(|x| (k.to_string(), x)))
}

/// One job: employer, role, years, record, titles, how it ended.
#[derive(Clone)]
struct Job {
    kind: String,
    emp: i64,
    role: String,
    from: i64,
    to: i64,
    w: i64,
    l: i64,
    titles: i64,
    ended: String,
}

fn jobs(b: &Book, row: &Map<String, Value>) -> Vec<Job> {
    let own = row.get("_row").cloned().unwrap_or(Value::Null);
    let mut out: Vec<Job> = Vec::new();
    if has(b, "coaches__history") {
        for r in rows(b, "SELECT * FROM coaches__history WHERE _parent = ?1 ORDER BY _i", &[json_sql(&own)]) {
            let Some((kind, emp)) = emp_of(&r, "") else { continue };
            out.push(Job {
                kind,
                emp,
                role: role_of(&r, ""),
                from: i(r.get("from")).unwrap_or(0),
                to: i(r.get("to")).unwrap_or(0),
                w: i(r.get("wins")).unwrap_or(0),
                l: i(r.get("losses")).unwrap_or(0),
                titles: i(r.get("titles")).unwrap_or(0),
                ended: s(r.get("ended")),
            });
        }
    }
    if let Some((kind, emp)) = emp_of(row, "job__") {
        let year = b
            .select_maps("SELECT year FROM world LIMIT 1", &[])
            .ok()
            .and_then(|r| r.first().and_then(|m| i(m.get("year"))))
            .unwrap_or(0);
        out.push(Job {
            kind,
            emp,
            role: role_of(row, "job__"),
            from: i(row.get("job__since")).unwrap_or(year),
            to: year,
            w: i(row.get("wins")).unwrap_or(0),
            l: i(row.get("losses")).unwrap_or(0),
            titles: i(row.get("titles")).unwrap_or(0),
            ended: "current".into(),
        });
    }
    out
}

/// Who he is in plain words.
fn words(b: &Book, row: &Map<String, Value>, js: &[Job], f: &Filters) -> Vec<Value> {
    let mut items = Vec::new();
    let mut t = format!("{}, {}", s(row.get("name")), s(row.get("age")));
    let eth = s(row.get("ethnicity"));
    if !eth.is_empty() {
        t.push_str(&format!(", {eth}"));
    }
    if i(row.get("ex_pro")) == Some(1) {
        t.push_str(", a former pro");
    }
    let up = s(row.get("came_up"));
    if !up.is_empty() {
        let up = match up.as_str() {
            "QB" => "quarterbacks".to_string(),
            "RB" => "running backs".into(),
            "OL" => "the offensive line".into(),
            "DL" => "the defensive line".into(),
            "LB" => "linebackers".into(),
            "DB" => "defensive backs".into(),
            x => x.to_lowercase(),
        };
        t.push_str(&format!(", came up coaching {up}"));
    }
    if f.debug
        && let Some(h) = i(row.get("home_state"))
    {
        let code =
            rows(b, "SELECT code FROM state_codes WHERE idx = ?1", &[h.into()]).first().map(|r| s(r.get("code")));
        t.push_str(&format!(", from {} (hidden)", code.unwrap_or_else(|| h.to_string())));
    }
    t.push('.');
    items.push(json!({ "text": t }));
    let pos = s(row.get("played__position"));
    if !pos.is_empty() {
        let col = i(row.get("played__college"));
        items.push(json!({ "text": format!("He played {pos}{}{}.",
            col.map_or(String::new(), |c| format!(" at {}", program_name(b, c))),
            i(row.get("played__last")).map_or(String::new(), |y| format!(" until {y}"))),
            "link": col.map(|c| format!("e/programs/{c}")) }));
    }
    if let Some(rep) = n(row.get("reputation")) {
        items.push(json!({ "text": format!("Reputation {rep:.1} of 20: {}.", rep_words(rep)) }));
    }
    match js.last().filter(|j| j.ended == "current") {
        Some(j) => {
            let (name, link) = crate::coach::employer(b, &j.kind, j.emp);
            let mut t = format!("Now {} at {name} since {}", role_words(&j.role), j.from);
            if j.role == "HeadCoach" {
                t.push_str(&format!(", {} there", record(j.w, j.l)));
            }
            if let Some(c) = n(row.get("contract__salary")) {
                t.push_str(&format!("; paid ${:.2}M a year", c / 1e6));
                if let Some(th) = i(row.get("contract__through")) {
                    t.push_str(&format!(" through {th}"));
                }
            }
            if let Some(p) = i(row.get("prove_it")) {
                t.push_str(&format!("; on a prove-it year in {p}"));
            }
            t.push('.');
            items.push(json!({ "text": t, "link": link }));
        }
        None if i(row.get("retired")) == Some(1) => items.push(json!({ "text": "Retired." })),
        None => items.push(json!({ "text": "Out of work." })),
    }
    let head: Vec<&Job> = js.iter().filter(|j| j.role == "HeadCoach").collect();
    if !head.is_empty() {
        let (w, l): (i64, i64) = head.iter().fold((0, 0), |a, j| (a.0 + j.w, a.1 + j.l));
        let titles = i(row.get("career_titles")).unwrap_or(0);
        items.push(json!({ "text": format!("As a head coach: {} in {} job{} ({}), {titles} title{}.",
            record(w, l), head.len(), if head.len() == 1 { "" } else { "s" }, pct(w, l), if titles == 1 { "" } else { "s" }) }));
    }
    if let Some(m) = i(row.get("mentor")) {
        items.push(
            json!({ "text": format!("He learned under {}.", coach_name(b, m)), "link": format!("e/coaches/{m}") }),
        );
    }
    items
}

/// Reputation in words, as the game shows it (`folk::rep_words`).
fn rep_words(r: f64) -> &'static str {
    match r {
        x if x >= 17.0 => "one of the best in the game",
        x if x >= 14.0 => "highly regarded",
        x if x >= 11.0 => "well regarded",
        x if x >= 8.0 => "solid",
        x if x >= 5.0 => "little known",
        _ => "unproven",
    }
}

/// His hidden ratings as meters with his level's median as a ghost.
fn ratings(b: &Book, row: &Map<String, Value>) -> Option<Value> {
    let teach: Vec<f64> = match row.get("ability__teaching") {
        Some(Value::String(t)) => serde_json::from_str(t).unwrap_or_default(),
        _ => Vec::new(),
    };
    if teach.is_empty() {
        return None;
    }
    // His level's medians among coaches in the same role.
    // A position coach's record names his group, not a role: he is set
    // against every position coach.
    let role = s(row.get("job__role"));
    let same = if !role.is_empty() {
        "job__role = ?1"
    } else if !s(row.get("job__role__PositionCoach")).is_empty() {
        "job__role__PositionCoach IS NOT NULL AND ?1 = ''"
    } else {
        // Out of work or retired: set against every coach.
        "?1 = ''"
    };
    let med = |c: &str| -> Option<f64> {
        let sql = format!(
            "SELECT {c} AS v FROM coaches WHERE {same} AND {c} IS NOT NULL ORDER BY v LIMIT 1
              OFFSET (SELECT COUNT(*) / 2 FROM coaches WHERE {same} AND {c} IS NOT NULL)"
        );
        rows(b, &sql, &[role.clone().into()]).first().and_then(|r| n(r.get("v")))
    };
    let item = |k: &str, v: f64, ghost: Option<f64>| json!({ "k": k, "v": (v * 10.0).round() / 10.0, "max": 20, "ghost": ghost, "ghost_label": "median for his role", "debug": true });
    let teaching: Vec<Value> =
        teach.iter().enumerate().map(|(k, v)| item(GROUPS.get(k).copied().unwrap_or("?"), *v, None)).collect();
    let mut rest = Vec::new();
    for (k, c) in [
        ("Scheme", "ability__scheme"),
        ("Game management", "ability__game_management"),
        ("Motivation", "ability__motivation"),
        ("Discipline", "ability__discipline"),
        ("Recruiting", "ability__recruiting"),
        ("Network", "ability__network"),
        ("Relationships", "ability__relationships"),
        ("Learning ceiling", "learning_ceiling"),
        ("Sportsmanship", "sportsmanship"),
    ] {
        if let Some(v) = n(row.get(c)) {
            rest.push(item(k, v, med(c)));
        }
    }
    let mut person = Vec::new();
    for (k, c) in
        [("Ambition", "personality__ambition"), ("Loyalty", "personality__loyalty"), ("Temper", "personality__temper")]
    {
        if let Some(v) = n(row.get(c)) {
            person.push(json!({ "k": k, "v": (v * 10.0).round() / 10.0, "max": 20 }));
        }
    }
    let hc = {
        let m = |c: &str| n(row.get(c)).unwrap_or(10.0);
        0.3 * teach.iter().sum::<f64>() / teach.len() as f64
            + 0.2 * m("ability__scheme")
            + 0.15 * m("ability__game_management")
            + 0.15 * m("ability__motivation")
            + 0.1 * m("ability__discipline")
            + 0.1 * m("ability__relationships")
    };
    let mut groups = vec![
        json!({ "name": "Teaching by group (hidden)", "items": teaching }),
        json!({ "name": format!("The rest (hidden); head coach value {hc:.1}"), "items": rest }),
    ];
    if !person.is_empty() {
        groups.push(json!({ "name": "Personality", "items": person }));
    }
    Some(
        json!({ "kind": "meters", "title": "His ratings (debug)", "note": "1-20; the tick is the median among coaches in his role today. Head coach value = 0.3 teaching + 0.2 scheme + 0.15 game management + 0.15 motivation + 0.1 discipline + 0.1 relationships", "groups": groups, "debug": true }),
    )
}

/// His jobs as lanes: one lane per level he worked at.
fn lanes(b: &Book, js: &[Job]) -> Option<Value> {
    if js.is_empty() {
        return None;
    }
    let level = |j: &Job| -> String {
        match j.kind.as_str() {
            "College" => rows(b, "SELECT division FROM programs WHERE id = ?1", &[j.emp.into()])
                .first()
                .map(|r| format!("College ({})", s(r.get("division"))))
                .unwrap_or_else(|| "College".into()),
            "HighSchool" => "High school".into(),
            "Nfl" => "NFL".into(),
            _ => "Other leagues".into(),
        }
    };
    let mut lanes: Vec<(String, Vec<Value>)> = Vec::new();
    for (k, j) in js.iter().enumerate() {
        // Back-to-back jobs share their switch year: draw each to the year
        // before the next one starts.
        let to = js.get(k + 1).map_or(j.to, |nx| if nx.from <= j.to && nx.from > j.from { nx.from - 1 } else { j.to });
        let (name, link) = employer(b, &j.kind, j.emp);
        let mut sub = role_words(&j.role);
        if j.role == "HeadCoach" && j.w + j.l > 0 {
            sub.push_str(&format!(", {}", record(j.w, j.l)));
        }
        if j.ended != "current" && !j.ended.is_empty() {
            sub.push_str(&format!("; {}", j.ended));
        }
        let lv = level(j);
        let item = json!({ "from": j.from, "to": to.max(j.from), "text": name, "sub": sub, "tone": role_tone(&j.role), "link": link });
        match lanes.iter_mut().find(|l| l.0 == lv) {
            Some(l) => l.1.push(item),
            None => lanes.push((lv, vec![item])),
        }
    }
    Some(json!({
        "kind": "bands", "title": "His jobs over the years",
        "note": "One lane per level; each band a job with its role, record (head coaches only: assistants are not credited) and how it ended",
        "lanes": lanes.into_iter().map(|(l, items)| json!({ "label": l, "items": items })).collect::<Vec<_>>(),
        "keys": [{"tone": "head", "text": "Head coach"}, {"tone": "oc", "text": "Offensive coordinator"}, {"tone": "dc", "text": "Defensive coordinator"}, {"tone": "pc", "text": "Position coach"}],
    }))
}

/// The world log's searches that hired him and the reviews of his jobs,
/// by year: (year, words).
fn log_whys(b: &Book, id: i64) -> (HashMap<i64, Vec<String>>, HashMap<i64, Vec<String>>) {
    let mut hired: HashMap<i64, Vec<String>> = HashMap::new();
    let mut reviewed: HashMap<i64, Vec<String>> = HashMap::new();
    let Some(t) = b.table("log_whys") else { return (hired, reviewed) };
    let year = t.pick(&["season", "year"]).unwrap_or("season");
    // R4's book names a why's fields `why__<Kind>__<field>`; older books
    // dropped the `why__`.
    let w = if t.has("why__Search__hired") || t.has("why__Review__coach") { "why__" } else { "" };
    if t.has(&format!("{w}Search__hired")) {
        let sql = format!(
            "SELECT {} AS y, {w}Search__why AS why, {w}Search__prestige AS p, {w}Search__read_sd AS sd FROM log_whys WHERE {w}Search__hired = ?1",
            q(year)
        );
        for r in rows(b, &sql, &[id.into()]) {
            let mut t = s(r.get("why"));
            if let Some(sd) = n(r.get("sd")) {
                t.push_str(&format!(" (the employer read teaching to ±{sd:.1})"));
            }
            hired.entry(i(r.get("y")).unwrap_or(0)).or_default().push(t);
        }
    }
    if t.has(&format!("{w}Review__coach")) {
        let sql = format!(
            "SELECT {} AS y, {w}Review__outcome AS o, {w}Review__read AS rd, {w}Review__bar AS bar, {w}Review__fire_chance AS fc,
                    {w}Review__tenure AS ten, {w}Review__win_pct_last AS wl, {w}Review__last AS last FROM log_whys WHERE {w}Review__coach = ?1",
            q(year)
        );
        for r in rows(b, &sql, &[id.into()]) {
            let t = format!(
                "AD's review in year {}: read {:.2} against a bar of {:.2}, a {:.0}% chance of firing, {} (won {:.0}% last season, {:+.0} points of win share against expected)",
                s(r.get("ten")),
                n(r.get("rd")).unwrap_or(0.0),
                n(r.get("bar")).unwrap_or(0.0),
                100.0 * n(r.get("fc")).unwrap_or(0.0),
                s(r.get("o")),
                100.0 * n(r.get("wl")).unwrap_or(0.0),
                100.0 * n(r.get("last")).unwrap_or(0.0)
            );
            reviewed.entry(i(r.get("y")).unwrap_or(0)).or_default().push(t);
        }
    }
    (hired, reviewed)
}

/// Every job with how he got it and how it ended.
fn job_whys(b: &Book, id: i64, js: &[Job]) -> Option<Value> {
    if js.is_empty() {
        return None;
    }
    let news = news_of(b, "Coach", id, None, 400);
    let (hired, reviewed) = log_whys(b, id);
    let in_years = |kinds: &[&str], a: i64, z: i64, emp: &str| -> Vec<String> {
        news.iter()
            .filter(|r| {
                let k = s(r.get("kind"));
                let y = i(r.get("year")).unwrap_or(0);
                kinds.iter().any(|x| k.starts_with(x))
                    && (a..=z).contains(&y)
                    && (emp.is_empty() || s(r.get("text")).contains(emp))
            })
            .map(|r| s(r.get("text")))
            .collect()
    };
    let mut out = Vec::new();
    for j in js.iter().rev() {
        let (name, link) = employer(b, &j.kind, j.emp);
        // How he got it: the hiring news that names him and the job, the
        // world log's search.
        let mut got: Vec<String> = in_years(&["CoachHired"], j.from - 1, j.from, &name);
        for y in [j.from - 1, j.from] {
            if let Some(v) = hired.get(&y) {
                got.extend(v.iter().cloned());
            }
        }
        // How it ended: his job record's words, the firing or leaving news,
        // the AD's reviews.
        let mut end: Vec<String> = Vec::new();
        if j.ended != "current" {
            end.push(format!("{}.", capital(&j.ended)));
            end.extend(in_years(&["CoachFired", "CoachLeft"], j.to - 1, j.to + 1, ""));
        }
        if j.kind == "College" && j.role == "HeadCoach" {
            for y in j.from..=j.to {
                if let Some(v) = reviewed.get(&y) {
                    end.extend(v.iter().map(|t| format!("{y}: {t}")));
                }
            }
        }
        got.dedup();
        end.dedup();
        out.push(json!([
            if j.from == j.to { j.from.to_string() } else { format!("{}-{}", j.from, j.to) },
            cell(name, link),
            role_words(&j.role),
            if j.role == "HeadCoach" { record(j.w, j.l) } else { String::new() },
            j.titles,
            if got.is_empty() { "No hiring news kept for it".to_string() } else { got.join(" ") },
            if j.ended == "current" { "Still there".to_string() } else { end.join(" ") },
        ]));
    }
    let note = if hired.is_empty() && reviewed.is_empty() {
        "How he got each job is the hiring news that names him; how it ended is his job record's words with the news. The world log's searches (each candidate's score) and the AD's reviews (read, bar, chance of firing) need the log, which this book doesn't have."
    } else {
        "How he got each job: the hiring news and the world log's search (who was called, why he was picked). How it ended: his job record's words, the news, and each AD review (read against the bar, the chance of firing)."
    };
    Some(json!({
        "kind": "table", "title": "Every job and its why", "note": note,
        "cols": [{"name": "years", "label": "Years"}, {"name": "at", "label": "Where"}, {"name": "role", "label": "Role"},
                 {"name": "record", "label": "W-L"}, {"name": "titles", "label": "Titles", "num": true},
                 {"name": "got", "label": "How he got it", "minw": 280}, {"name": "ended", "label": "How it ended", "minw": 280}],
        "rows": out, "dense": true
    }))
}

/// His tree: the mentors above him and everyone who learned under him,
/// three generations down.
fn tree(b: &Book, row: &Map<String, Value>) -> Option<Value> {
    let id = i(row.get("id"))?;
    let all = rows(
        b,
        "SELECT id, name, mentor, COALESCE(job__role, 'PositionCoach:' || job__role__PositionCoach) AS role, wins, losses, career_titles AS t, retired FROM coaches",
        &[],
    );
    let by: HashMap<i64, &Map<String, Value>> = all.iter().filter_map(|r| Some((i(r.get("id"))?, r))).collect();
    let mut kids: HashMap<i64, Vec<i64>> = HashMap::new();
    for r in &all {
        if let (Some(c), Some(m)) = (i(r.get("id")), i(r.get("mentor"))) {
            kids.entry(m).or_default().push(c);
        }
    }
    let sub = |c: i64| -> String {
        let Some(r) = by.get(&c) else { return String::new() };
        let mut t = match s(r.get("role")) {
            x if x.is_empty() => {
                if i(r.get("retired")) == Some(1) {
                    "retired".to_string()
                } else {
                    "out of work".to_string()
                }
            }
            x => crate::coach::role_words(&x),
        };
        let t_ = i(r.get("t")).unwrap_or(0);
        if t_ > 0 {
            t.push_str(&format!(", {t_} title{}", if t_ == 1 { "" } else { "s" }));
        }
        t
    };
    fn down(
        c: i64,
        depth: usize,
        kids: &HashMap<i64, Vec<i64>>,
        name: &dyn Fn(i64) -> String,
        sub: &dyn Fn(i64) -> String,
    ) -> Value {
        let mut ks: Vec<i64> = kids.get(&c).cloned().unwrap_or_default();
        ks.sort_by_key(|k| std::cmp::Reverse(kids.get(k).map_or(0, Vec::len)));
        let ks: Vec<Value> =
            if depth < 3 { ks.iter().map(|k| down(*k, depth + 1, kids, name, sub)).collect() } else { Vec::new() };
        json!({ "text": name(c), "sub": sub(c), "link": format!("e/coaches/{c}"), "kids": ks })
    }
    let name = |c: i64| by.get(&c).map_or_else(|| format!("Coach {c}"), |r| s(r.get("name")));
    let mut root = down(id, 0, &kids, &name, &sub);
    root["me"] = json!(true);
    let mut up = Vec::new();
    let mut m = i(row.get("mentor"));
    while let Some(x) = m {
        if up.len() >= 8 || x == id {
            break;
        }
        up.push(json!({ "text": name(x), "sub": sub(x), "link": format!("e/coaches/{x}") }));
        m = by.get(&x).and_then(|r| i(r.get("mentor")));
    }
    // Everyone below him, every generation.
    let mut total = 0;
    let mut stack: Vec<i64> = kids.get(&id).cloned().unwrap_or_default();
    let mut seen = std::collections::HashSet::new();
    while let Some(k) = stack.pop() {
        if seen.insert(k) {
            total += 1;
            stack.extend(kids.get(&k).cloned().unwrap_or_default());
        }
    }
    let direct = kids.get(&id).map_or(0, Vec::len);
    if up.is_empty() && direct == 0 {
        return None;
    }
    Some(json!({
        "kind": "tree", "title": "His coaching tree",
        "note": if direct == 0 {
            "No one has learned under him yet; above him, the head coaches he learned under longest".to_string()
        } else {
            format!("{direct} learned under him directly, {total} in all down his tree; above him, the head coaches he learned under longest. Three generations shown, the biggest branches first")
        },
        "up": up, "root": root,
    }))
}

/// His unit records: how his players grew against everyone at their level
/// and age, group by group (`coach::UnitRecord`).
fn units(b: &Book, row: &Map<String, Value>) -> Option<Value> {
    if !has(b, "coaches__units") {
        return None;
    }
    let own = row.get("_row").cloned().unwrap_or(Value::Null);
    let out: Vec<Value> = rows(
        b,
        "SELECT _i, growth, seasons, moved_up FROM coaches__units WHERE _parent = ?1 ORDER BY _i",
        &[json_sql(&own)],
    )
    .iter()
    .filter(|r| n(r.get("seasons")).is_some_and(|x| x > 0.0))
    .map(|r| {
        let (g, se) = (n(r.get("growth")).unwrap_or(0.0), n(r.get("seasons")).unwrap_or(0.0));
        json!([
            GROUPS.get(i(r.get("_i")).unwrap_or(0) as usize).copied().unwrap_or("?"),
            (g / (se + 8.0) * 100.0).round() / 100.0,
            (se * 10.0).round() / 10.0,
            n(r.get("moved_up")).map(|x| (x * 10.0).round() / 10.0)
        ])
    })
    .collect();
    if out.is_empty() {
        return None;
    }
    Some(
        json!({ "kind": "table", "title": "His teaching record", "note": "What schools can see: his players' growth against everyone at their level and age (growth read = growth / (seasons + 8)), the seasons behind it (fading 0.85 a year) and how many moved up a level",
        "cols": [{"name": "group", "label": "Group"}, {"name": "read", "label": "Growth read", "num": true}, {"name": "seasons", "label": "Seasons", "num": true}, {"name": "moved", "label": "Moved up", "num": true}],
        "rows": out, "dense": true }),
    )
}

/// His buzz from unit stats by season (`stats.buzz`).
fn buzz(b: &Book, id: i64) -> Option<Value> {
    let t = ["stats__buzz"].into_iter().find(|t| has(b, t))?;
    let out: Vec<Value> =
        rows(b, &format!("SELECT * FROM {t} WHERE coach = ?1 ORDER BY year DESC LIMIT 40"), &[id.into()])
            .iter()
            .map(|r| {
                let p = i(r.get("program"));
                json!([
                    s(r.get("year")),
                    p.map_or(json!(""), |p| cell(program_name(b, p), format!("e/programs/{p}"))),
                    if i(r.get("offense")) == Some(1) { "offense" } else { "defense" },
                    n(r.get("pct")).map(|x| (x * 1000.0).round() / 10.0),
                    n(r.get("gain")).map(|x| (x * 100.0).round() / 100.0),
                    s(r.get("why"))
                ])
            })
            .collect();
    if out.is_empty() {
        return None;
    }
    Some(
        json!({ "kind": "table", "title": "His buzz from unit stats", "note": "Each season his unit ranked high (or low) in EPA a play in its division; the coordinator gets it all, the head coach half, halving every year",
        "cols": [{"name": "year", "label": "Season"}, {"name": "at", "label": "Program"}, {"name": "side", "label": "Side"}, {"name": "pct", "label": "Top %", "num": true},
                 {"name": "gain", "label": "Buzz", "num": true}, {"name": "why", "label": "Why", "minw": 260}],
        "rows": out, "dense": true }),
    )
}

/// His reputation each year, from the world log's coach rows.
fn rep_line(b: &Book, id: i64) -> Option<Value> {
    if !has(b, "log_coaches") {
        return None;
    }
    let pts: Vec<Value> = rows(
        b,
        "SELECT season, json_extract(more('log_coaches', _more), '$.coach.reputation') AS r FROM log_coaches WHERE id = ?1 ORDER BY season",
        &[id.into()],
    )
    .iter()
    .filter_map(|r| Some(json!([i(r.get("season"))?, n(r.get("r"))?])))
    .collect();
    if pts.len() < 2 {
        return None;
    }
    Some(
        json!({ "kind": "chart", "type": "line", "title": "His reputation each year", "note": "From the world log's yearly coach rows (1-20)", "x": "Year",
                 "series": [{ "name": "Reputation", "points": pts }] }),
    )
}

/// The blocks a coach page adds after the header.
pub fn blocks(b: &Book, row: &Map<String, Value>, f: &Filters) -> Vec<Value> {
    let id = i(row.get("id")).unwrap_or(0);
    let js = jobs(b, row);
    let mut out =
        vec![json!({ "kind": "list", "title": "In plain words", "items": words(b, row, &js, f), "dense": true })];
    if f.debug {
        out.extend(ratings(b, row));
    }
    out.extend(lanes(b, &js));
    out.extend(job_whys(b, id, &js));
    out.extend(tree(b, row));
    out.extend(rep_line(b, id));
    out.extend(units(b, row));
    out.extend(buzz(b, id));
    crate::inst::tidy(&mut out);
    out
}
