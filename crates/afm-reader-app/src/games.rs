//! The game, club and draft pages (lane R8): one game with its box, player
//! lines, drives, scoring plays, win chance and play by play with its
//! whys; an NFL club with its seasons, money and people; one club season;
//! a draft class; an agent. Every name opens its own page.

use std::collections::{HashMap, HashSet};

use rusqlite::types::Value as Sql;
use serde_json::{Map, Value, json};

use crate::book::{Book, Filters, cell_text};
use crate::pages::{fmt, new_page, push, world_name};

type Row = Map<String, Value>;

fn all(b: &Book, sql: &str, vals: &[Sql]) -> Vec<Row> {
    b.select_maps(sql, vals).unwrap_or_default()
}

fn one(b: &Book, sql: &str, vals: &[Sql]) -> Option<Row> {
    all(b, sql, vals).into_iter().next()
}

fn num(r: &Row, k: &str) -> f64 {
    r.get(k).and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0.0)
}

fn int(r: &Row, k: &str) -> i64 {
    num(r, k).round() as i64
}

fn has(r: &Row, k: &str) -> bool {
    r.get(k).is_some_and(|v| !v.is_null())
}

fn text(r: &Row, k: &str) -> String {
    r.get(k).and_then(cell_text).unwrap_or_default()
}

/// A JSON list kept as text (`"[16,28]"`).
fn list(r: &Row, k: &str) -> Vec<Value> {
    match r.get(k) {
        Some(Value::Array(a)) => a.clone(),
        Some(Value::String(s)) => serde_json::from_str(s).unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn i(x: i64) -> Sql {
    Sql::Integer(x)
}

fn one_dp(x: f64) -> String {
    fmt(&json!((x * 10.0).round() / 10.0))
}

fn money(x: f64) -> String {
    if x.abs() >= 1e9 {
        format!("${:.2}B", x / 1e9)
    } else if x.abs() >= 1e6 {
        format!("${:.1}M", x / 1e6)
    } else {
        format!("${}", fmt(&json!(x.round())))
    }
}

/// "12:05" from seconds.
fn mmss(secs: f64) -> String {
    let s = secs.round() as i64;
    format!("{}:{:02}", s / 60, s % 60)
}

/// "only 3%", "only 1 in 600": a small share in words.
fn share(x: f64) -> String {
    if x >= 0.01 {
        format!("{:.0}%", 100.0 * x)
    } else if x > 0.0 {
        format!("1 in {:.0}", 1.0 / x)
    } else {
        "none".into()
    }
}

/// The world's enum words as plain words: "PracticeSquad" is "practice squad".
fn plain(s: &str) -> String {
    let mut out = String::new();
    for (k, c) in s.chars().enumerate() {
        if c.is_uppercase() && k > 0 {
            out.push(' ');
        }
        out.extend(c.to_lowercase());
    }
    match out.as_str() {
        "reserve" => "reserve list".into(),
        "future" => "futures".into(),
        "franchise" => "franchise tag".into(),
        _ => out,
    }
}

/// The statuses of a man still in the NFL.
const NFL_NOW: [&str; 3] = ["Roster", "PracticeSquad", "Reserve"];

fn plural(n: i64, one: &str, many: &str) -> String {
    format!("{} {}", fmt(&json!(n)), if n == 1 { one } else { many })
}

/// "Q3 7:42" from the quarter and the seconds left in it.
fn clock(q: i64, secs: f64) -> String {
    let s = secs.max(0.0).round() as i64;
    let qn = if q >= 5 { if q == 5 { "OT".to_string() } else { format!("{}OT", q - 4) } } else { format!("Q{q}") };
    format!("{qn} {}:{:02}", s / 60, s % 60)
}

/// A spot as the offense sees it: "own 25", "50", "opp 12".
fn spot(yl100: f64) -> String {
    let y = yl100.round() as i64;
    match y {
        50 => "50".into(),
        y if y > 50 => format!("own {}", 100 - y),
        y => format!("opp {y}"),
    }
}

fn ordinal(n: i64) -> &'static str {
    match n {
        1 => "1st",
        2 => "2nd",
        3 => "3rd",
        _ => "4th",
    }
}

// ---------------------------------------------------------------- names

const NFL_NAME: &str = "SELECT COALESCE((SELECT CASE WHEN (SELECT fictional_league FROM options) = 1 THEN f.fictional ELSE f.name END FROM engine_franchises f WHERE f.team = ?1 AND ?2 BETWEEN f.\"from\" AND f.\"to\"), (SELECT name FROM nfl WHERE _row = ?1)) AS n";

/// A team's name in a season, by the game log's stream: an NFL club by
/// the name of its era, a program, a high school.
pub fn team_name(b: &Book, stream: &str, season: i64, idx: i64) -> String {
    let got = match stream {
        "nfl" => one(b, NFL_NAME, &[i(idx), i(season)])
            .or_else(|| one(b, "SELECT name AS n FROM nfl WHERE _row = ?1", &[i(idx)])),
        "college" => one(b, "SELECT COALESCE(real_name, fictional_name) AS n FROM programs WHERE _row = ?1", &[i(idx)]),
        "hs" => one(b, "SELECT COALESCE(name, fictional_name) AS n FROM high_schools WHERE _row = ?1", &[i(idx)]),
        _ => None,
    };
    got.map(|r| text(&r, "n")).filter(|s| !s.is_empty()).unwrap_or_else(|| format!("Team {idx}"))
}

/// A team's page: an NFL club's season, a program, a high school.
fn team_link(stream: &str, season: i64, idx: i64) -> String {
    match stream {
        "nfl" => format!("club/{idx}/{season}"),
        "college" => format!("e/programs/{idx}"),
        "hs" => format!("e/high_schools/{idx}"),
        _ => String::new(),
    }
}

/// "Giants" from "New York Giants"; a program keeps its whole name.
fn short(stream: &str, name: &str) -> String {
    if stream == "nfl" || stream == "other" {
        name.rsplit(' ').next().unwrap_or(name).to_string()
    } else {
        name.to_string()
    }
}

/// People by Player::id, wherever the book keeps them: their name and
/// their page.
struct People(HashMap<i64, (String, String)>);

impl People {
    fn of(b: &Book, ids: &HashSet<i64>) -> People {
        let mut m = HashMap::new();
        if ids.is_empty() {
            return People(m);
        }
        let list = ids.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
        for (table, sql) in [
            ("people", format!("SELECT player__id AS id, name FROM people WHERE player__id IN ({list})")),
            ("alumni", format!("SELECT player__id AS id, name FROM alumni WHERE player__id IN ({list})")),
            ("", format!("SELECT id, MAX(name) AS name FROM log_people WHERE id IN ({list}) GROUP BY id")),
        ] {
            for r in all(b, &sql, &[]) {
                let link = if table.is_empty() { String::new() } else { format!("e/{table}/{}", int(&r, "id")) };
                m.entry(int(&r, "id")).or_insert((text(&r, "name"), link));
            }
        }
        People(m)
    }

    fn name(&self, id: i64) -> String {
        self.0.get(&id).map(|x| x.0.clone()).unwrap_or_else(|| format!("#{id}"))
    }

    fn link(&self, id: i64) -> String {
        self.0.get(&id).map(|x| x.1.clone()).unwrap_or_default()
    }

    /// Every `#123` in a why, as the person's name.
    fn words(&self, s: &str) -> String {
        let mut out = String::new();
        let mut it = s.char_indices().peekable();
        while let Some((at, c)) = it.next() {
            if c == '#' {
                let digits: String = s[at + 1..].chars().take_while(char::is_ascii_digit).collect();
                if let Ok(id) = digits.parse::<i64>()
                    && let Some((n, _)) = self.0.get(&id)
                {
                    out.push_str(n);
                    for _ in 0..digits.len() {
                        it.next();
                    }
                    continue;
                }
            }
            out.push(c);
        }
        out
    }

    /// The names map a view uses to link `#123`.
    fn map(&self) -> Value {
        Value::Object(self.0.iter().map(|(k, (n, l))| (k.to_string(), json!([n, l]))).collect())
    }
}

fn ids_in(s: &str, out: &mut HashSet<i64>) {
    let mut rest = s;
    while let Some(at) = rest.find('#') {
        let d: String = rest[at + 1..].chars().take_while(char::is_ascii_digit).collect();
        if let Ok(x) = d.parse() {
            out.insert(x);
        }
        rest = &rest[at + 1..];
    }
}

// ---------------------------------------------------------------- the game

/// A logged game's snaps, from the game log's play rows.
pub fn plays_of(b: &Book, g: &Row) -> Vec<Value> {
    if b.table("log_plays").is_none() {
        return Vec::new();
    }
    let sql = "SELECT _row FROM log_plays WHERE season = ?1 AND stream = ?2 AND date = ?3 AND home = ?4 AND away = ?5 LIMIT 1";
    let vals = [
        i(int(g, "season")),
        Sql::Text(text(g, "stream")),
        Sql::Text(text(g, "date")),
        i(int(g, "home")),
        i(int(g, "away")),
    ];
    let Some(r) = one(b, sql, &vals) else { return Vec::new() };
    b.more_of("log_plays", &r["_row"])
        .and_then(|m| m.get("plays").cloned())
        .and_then(|p| p.as_array().cloned())
        .unwrap_or_default()
}

fn pf(p: &Value, k: &str) -> f64 {
    p.get(k).and_then(Value::as_f64).unwrap_or(0.0)
}

fn pi(p: &Value, k: &str) -> i64 {
    pf(p, k).round() as i64
}

fn pb(p: &Value, k: &str) -> bool {
    p.get(k).is_some_and(|v| v.as_bool().unwrap_or(false) || v.as_f64().is_some_and(|x| x != 0.0))
}

/// The box's numbers, one team game each, and what they mean.
const BOX: [(&str, &str, &str); 27] = [
    ("Points", "points", "points"),
    ("First downs", "first_downs", ""),
    ("Plays", "plays", "plays"),
    ("Total yards", "yards", ""),
    ("Passing yards (net of sacks)", "pass_yards", "pass_yards"),
    ("Completions", "cmp", "cmp"),
    ("Pass attempts", "att", "att"),
    ("Sacks taken", "sacks", "sacks"),
    ("Sack yards lost", "sack_yds", "sack_yards"),
    ("Rushing yards", "rush_yards", "rush_yards"),
    ("Carries", "carries", "carries"),
    ("Successful plays", "successes", "success"),
    ("Explosive plays", "explosive", ""),
    ("Third downs", "third_att", "third"),
    ("Third downs converted", "third_conv", "third_conv"),
    ("Fourth-down tries", "fourth_att", "fourth_go"),
    ("Fourth downs converted", "fourth_conv", "fourth_conv"),
    ("Red-zone trips", "red_zone_trips", ""),
    ("Red-zone touchdowns", "red_zone_tds", ""),
    ("Interceptions thrown", "ints", "int"),
    ("Fumbles lost", "fumbles_lost", "fumbles_lost"),
    ("Penalties", "penalties", "penalties"),
    ("Penalty yards", "penalty_yds", "penalty_yards"),
    ("Time of possession", "clock_possession", ""),
    ("Punts", "punts", ""),
    ("Field goals made of tried", "fg_made", ""),
    ("Expected points added", "epa", "epa"),
];

fn box_val(r: &Row, k: &str) -> f64 {
    match k {
        "clock_possession" => num(r, k) / 60.0,
        _ => num(r, k),
    }
}

/// The real figure for a box number, one team game, by level.
fn real_box(stream: &str, div: &str) -> Map<String, Value> {
    let file = if stream == "nfl" { "nfl_game_targets.json" } else { "college_game_targets.json" };
    let Some(v) = afm_reader::real_in::file(file).and_then(|t| serde_json::from_str::<Value>(t).ok()) else {
        return Map::new();
    };
    let mut m = match stream {
        "nfl" => v["per_team_game"].as_object().cloned().unwrap_or_default(),
        "college" => {
            let key = match div {
                "FCS" => "fcs",
                "D2" => "ii",
                "D3" => "iii",
                _ => "fbs",
            };
            v["divisions"][key]["per_team_game"].as_object().cloned().unwrap_or_default()
        }
        _ => Map::new(),
    };
    let pts = if stream == "nfl" {
        v["points_per_team_game"].clone()
    } else {
        m.get("final_score").cloned().unwrap_or(Value::Null)
    };
    m.insert("points".into(), pts);
    m
}

pub fn game(b: &mut Book, row: i64, f: &Filters) -> Result<Value, String> {
    let g = one(b, "SELECT * FROM log_games WHERE _row = ?1", &[i(row)])
        .ok_or_else(|| format!("No game {row} in this book."))?;
    let (stream, season) = (text(&g, "stream"), int(&g, "season"));
    let (home, away) = (int(&g, "home"), int(&g, "away"));
    let names = list(&g, "names");
    let name_of = |side: usize, idx: i64| {
        names
            .get(side)
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| team_name(b, &stream, season, idx))
    };
    let (hn, an) = (name_of(0, home), name_of(1, away));
    let (hs, as_) = (short(&stream, &hn), short(&stream, &an));
    let sides = [hs.clone(), as_.clone()];
    let (hp, ap) = (int(&g, "home_pts"), int(&g, "away_pts"));
    let played = has(&g, "home_pts");
    let level = match stream.as_str() {
        "nfl" => "NFL".to_string(),
        "college" => "College".into(),
        "hs" => "High school".into(),
        _ => text(&g, "league").chars().next().map(|_| text(&g, "league")).unwrap_or_else(|| "Other league".into()),
    };
    let kind = text(&g, "kind");
    let title = if played { format!("{an} {ap}, {hn} {hp}") } else { format!("{an} at {hn}") };
    let mut p = new_page(&title, &format!("{} · Games · {title}", world_name(b)));

    // The day: when, where, the weather, the line and how it went against it.
    let mut facts: Vec<String> = Vec::new();
    let when = [text(&g, "date"), text(&g, "kickoff")]
        .iter()
        .filter(|s| !s.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if !when.is_empty() {
        facts.push(when);
    }
    let mut site = Vec::new();
    if has(&g, "neutral") {
        site.push("a neutral site".to_string());
    }
    let town = [text(&g, "site__town"), text(&g, "site__state")]
        .iter()
        .filter(|s| !s.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if !town.is_empty() {
        site.push(town);
    }
    if has(&g, "indoor") || has(&g, "site__indoor") || has(&g, "weather__indoor") {
        site.push("indoors".into());
    } else {
        if has(&g, "conditions__temperature") {
            site.push(format!("{}°F", int(&g, "conditions__temperature")));
        }
        if num(&g, "conditions__wind") >= 1.0 {
            site.push(format!("wind {} mph", int(&g, "conditions__wind")));
        }
        if has(&g, "conditions__rain") {
            site.push("rain".into());
        }
        if has(&g, "conditions__snow") {
            site.push("snow".into());
        }
    }
    if !site.is_empty() {
        facts.push(site.join(", "));
    }
    if has(&g, "attendance") {
        facts.push(format!("{} in the stands", fmt(&json!(int(&g, "attendance")))));
    }
    let line = list(&g, "line");
    if let (Some(sp), Some(tot)) = (line.first().and_then(Value::as_f64), line.get(1).and_then(Value::as_f64)) {
        let fav = if sp < 0.0 { &hs } else { &as_ };
        let mut s = if sp == 0.0 {
            format!("Line: a pick'em, total {tot}")
        } else {
            format!("Line: {fav} by {}, total {tot}", sp.abs())
        };
        if played {
            let m = (hp - ap) as f64;
            let cover = m + sp;
            let ats = if cover.abs() < 0.01 {
                "a push against the spread".to_string()
            } else if cover > 0.0 {
                format!("{hs} covered")
            } else {
                format!("{as_} covered")
            };
            let ou = ((hp + ap) as f64 - tot).signum();
            s.push_str(&format!(
                ": {ats}, and the {} points went {}",
                hp + ap,
                if ou > 0.0 {
                    "over"
                } else if ou < 0.0 {
                    "under"
                } else {
                    "exactly on the total"
                }
            ));
        }
        facts.push(s);
    }
    let mut tags = vec![level.clone(), format!("{season} season")];
    if !kind.is_empty() {
        tags.push(kind.clone());
    }
    if has(&g, "week") {
        tags.push(format!("week {}", int(&g, "week")));
    }
    for (k, t) in [
        ("ot", "overtime"),
        ("rivalry", "rivalry"),
        ("conference_game", "conference game"),
        ("homecoming", "homecoming"),
        ("senior_day", "senior day"),
        ("standin", "stand-in result, no snaps"),
        ("forfeit", "forfeit"),
    ] {
        if has(&g, k) && num(&g, k) != 0.0 {
            tags.push(t.into());
        }
    }
    if has(&g, "tv") {
        tags.push(format!("TV {}", text(&g, "tv")));
    }
    let divs = list(&g, "divisions");
    let div = divs.first().and_then(Value::as_str).unwrap_or("").to_string();
    if !div.is_empty() {
        tags.push(divs.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" vs "));
    }
    let mut actions = Vec::new();
    for (idx, nm) in [(away, &an), (home, &hn)] {
        let l = team_link(&stream, season, idx);
        if !l.is_empty() {
            actions.push(json!({ "text": nm, "link": l }));
        }
    }
    let plays = plays_of(b, &g);
    if !plays.is_empty() {
        actions.push(json!({ "text": "Replay snap by snap", "link": format!("tools/replay?game={row}") }));
    }
    let boxes = all(b, "SELECT * FROM log_games__box WHERE _parent = ?1 ORDER BY _i", &[i(row)]);
    let mut figures = vec![json!({ "label": as_, "value": ap }), json!({ "label": hs, "value": hp })];
    if boxes.len() == 2 {
        figures.push(
            json!({ "label": "Yards", "value": format!("{} – {}", int(&boxes[1], "yards"), int(&boxes[0], "yards")) }),
        );
    }
    figures.push(
        json!({ "label": "Snaps", "value": if plays.is_empty() { int(&g, "snaps") } else { plays.len() as i64 } }),
    );
    push(
        &mut p,
        json!({ "kind": "hero", "label": format!("{level} game · {}", if kind.is_empty() { "game".to_string() } else { kind.clone() }),
                "title": title, "text": facts.join(" · "), "tags": tags, "actions": actions, "figures": figures }),
    );
    if !text(&g, "why").is_empty() {
        push(&mut p, json!({ "kind": "note", "text": format!("Why this game: {}", text(&g, "why")) }));
    }
    if !played || has(&g, "standin") && num(&g, "standin") != 0.0 {
        p["verdict"] = json!(if played {
            "A stand-in result: the world decided this game without the engine, so it has a score but no box, lines or snaps."
        } else {
            "Not played yet."
        });
        return Ok(p);
    }

    // Who played, by Player::id, for every name below.
    let lines = all(b, "SELECT * FROM log_games__players WHERE _parent = ?1 ORDER BY _i, _j", &[i(row)]);
    let scoring = all(b, "SELECT * FROM log_games__scoring WHERE _parent = ?1 ORDER BY _i", &[i(row)]);
    let pens = all(b, "SELECT * FROM log_games__penalties WHERE _parent = ?1 ORDER BY _i", &[i(row)]);
    let injuries = all(b, "SELECT * FROM log_games__injuries WHERE _parent = ?1 ORDER BY _i", &[i(row)]);
    let mut ids: HashSet<i64> = HashSet::new();
    for r in &lines {
        ids.insert(int(r, "id"));
    }
    for r in scoring.iter().chain(&pens).chain(&injuries) {
        for k in ["carrier", "passer", "target", "kicker", "returner", "player"] {
            if has(r, k) {
                ids.insert(int(r, k));
            }
        }
    }
    for pl in &plays {
        if let Some(w) = pl.get("why").and_then(Value::as_str) {
            ids_in(w, &mut ids);
        }
    }
    let calls = all(b, "SELECT * FROM log_games__officiating__calls WHERE _parent = ?1 ORDER BY _i", &[i(row)]);
    for r in &calls {
        ids_in(&text(r, "note"), &mut ids);
    }
    let who = People::of(b, &ids);
    let side_of = |r: &Row| if num(r, "team") != 0.0 { 1 } else { 0 };

    // What stands out: each team's numbers against every team game of the
    // level that season.
    let level_rows = if stream == "college" && !div.is_empty() {
        all(
            b,
            "SELECT x.* FROM log_games__box x JOIN log_games g ON g._row = x._parent WHERE g.stream = ?1 AND g.season = ?2 AND COALESCE(g.standin, 0) = 0 AND json_extract(g.divisions, '$[' || x._i || ']') = ?3",
            &[Sql::Text(stream.clone()), i(season), Sql::Text(div.clone())],
        )
    } else {
        all(
            b,
            "SELECT x.* FROM log_games__box x JOIN log_games g ON g._row = x._parent WHERE g.stream = ?1 AND g.season = ?2 AND COALESCE(g.standin, 0) = 0",
            &[Sql::Text(stream.clone()), i(season)],
        )
    };
    let group =
        if stream == "college" && !div.is_empty() { format!("{div} {season}") } else { format!("{level} {season}") };
    let real = real_box(&stream, &div);
    if boxes.len() == 2 && !level_rows.is_empty() {
        let n = level_rows.len() as f64;
        let show = |k: &str, v: f64| if k == "clock_possession" { mmss(v * 60.0) } else { one_dp(v) };
        let mut items: Vec<(f64, Value)> = Vec::new();
        for (label, k, rk) in BOX
            .iter()
            .filter(|(_, k, _)| !["successes", "third_att", "fourth_att", "plays", "carries", "att", "cmp"].contains(k))
        {
            let avg = level_rows.iter().map(|r| box_val(r, k)).sum::<f64>() / n;
            for (s, bx) in boxes.iter().enumerate() {
                let v = box_val(bx, k);
                // As many or more, as few or fewer: ties count against rarity.
                let at_most = level_rows.iter().filter(|r| box_val(r, k) <= v + 1e-9).count() as f64 / n;
                let at_least = level_rows.iter().filter(|r| box_val(r, k) >= v - 1e-9).count() as f64 / n;
                let (rare, word) = if at_least <= 0.05 {
                    (at_least, "as many")
                } else if at_most <= 0.05 {
                    (at_most, "as few")
                } else {
                    continue;
                };
                let real_txt =
                    real.get(*rk).and_then(Value::as_f64).map(|x| format!("; real {}", one_dp(x))).unwrap_or_default();
                let word = if *k == "epa" || *k == "clock_possession" {
                    if word == "as many" { "as high" } else { "as low" }
                } else {
                    word
                };
                items.push((
                    rare,
                    json!({ "text": format!("{}: {} {}, and only {} of {group} team games had {word}", sides[s], show(k, v), label.to_lowercase(), share(rare)),
                            "why": format!("World average {} a team game in {group} ({} team games){real_txt}.", show(k, avg), level_rows.len()) }),
                ));
            }
        }
        items.sort_by(|a, b| a.0.total_cmp(&b.0));
        let total = items.len();
        let list_items: Vec<Value> = items.into_iter().take(10).map(|x| x.1).collect();
        push(
            &mut p,
            json!({ "kind": "list", "title": "What stands out", "note": format!("Each team's numbers in the top or bottom 5% of {group} team games"),
                    "items": list_items, "empty": "Nothing in this game's box sits in the top or bottom 5% of the level's team games.", "more": { "n": total } }),
        );
    }

    // The score by quarter.
    let qs = all(b, "SELECT _0 AS h, _1 AS a FROM log_games__quarters WHERE _parent = ?1 ORDER BY _i", &[i(row)]);
    if !qs.is_empty() {
        let mut cols = vec![json!({ "name": "team", "label": "Team" })];
        for (k, _) in qs.iter().enumerate() {
            let l = if k < 4 { format!("Q{}", k + 1) } else { "OT".into() };
            cols.push(json!({ "name": l, "label": l, "num": true }));
        }
        cols.push(json!({ "name": "final", "label": "Final", "num": true }));
        let mut ar = vec![json!(an)];
        ar.extend(qs.iter().map(|q| json!(int(q, "a"))));
        ar.push(json!(ap));
        let mut hr = vec![json!(hn)];
        hr.extend(qs.iter().map(|q| json!(int(q, "h"))));
        hr.push(json!(hp));
        let links: Vec<Value> = [away, home].iter().map(|x| json!(team_link(&stream, season, *x))).collect();
        push(
            &mut p,
            json!({ "kind": "table", "title": "Score by quarter", "cols": cols, "rows": [ar, hr], "links": links }),
        );
    }

    // The win chance, snap by snap, and the drives.
    if !plays.is_empty() {
        let qlen = if plays.iter().map(|p| pf(p, "clock")).fold(0.0, f64::max) > 720.5 { 900.0 } else { 720.0 };
        let mut pts: Vec<Value> = Vec::new();
        let mut marks: Vec<Value> = Vec::new();
        let mut ot_k = 0.0;
        let home_wp = |p: &Value, k: &str| {
            let w = pf(p, k);
            if pi(p, "offense") == 0 { w } else { 1.0 - w }
        };
        let t_of = |p: &Value, ot_k: &mut f64| {
            let q = pi(p, "quarter");
            if q <= 4 {
                ((q - 1) as f64 * qlen + (qlen - pf(p, "clock"))) / 60.0
            } else {
                *ot_k += 0.25;
                4.0 * qlen / 60.0 + *ot_k
            }
        };
        if let Some(p0) = plays.first() {
            pts.push(json!([0.0, (1000.0 * home_wp(p0, "wp_before")).round() / 10.0]));
        }
        for (k, pl) in plays.iter().enumerate() {
            let t = t_of(pl, &mut ot_k);
            let w = if pl.get("wp_after").is_some() {
                home_wp(pl, "wp_after")
            } else if k + 1 == plays.len() {
                if hp > ap {
                    1.0
                } else if hp < ap {
                    0.0
                } else {
                    0.5
                }
            } else {
                continue;
            };
            pts.push(json!([(t * 100.0).round() / 100.0, (1000.0 * w).round() / 10.0]));
        }
        for r in &scoring {
            let q = int(r, "q");
            let t = if q <= 4 { ((q - 1) as f64 * qlen + (qlen - num(r, "clock"))) / 60.0 } else { 4.0 * qlen / 60.0 };
            let after = list(r, "after");
            marks.push(json!({ "t": (t * 100.0).round() / 100.0, "side": side_of(r),
                                "text": format!("{} {}", sides[side_of(r)], text(r, "what")),
                                "score": format!("{}–{}", after.get(1).map(fmt).unwrap_or_default(), after.first().map(fmt).unwrap_or_default()) }));
        }
        let swings = plays
            .iter()
            .filter(|pl| pl.get("wp_after").is_some())
            .map(|pl| (pf(pl, "wp_after") - pf(pl, "wp_before")).abs())
            .fold(0.0, f64::max);
        push(
            &mut p,
            json!({ "kind": "winline", "title": format!("{}'{} chance to win, snap by snap", hs, if hs.ends_with('s') { "" } else { "s" }),
                    "note": format!("The engine's own read after each snap (its win chance model); the biggest single swing was {:.0} points", 100.0 * swings),
                    "home": hs, "away": as_, "points": pts, "marks": marks, "quarters": (qs.len().max(4)), "qmin": qlen / 60.0 }),
        );
    }
    let drives = all(b, "SELECT * FROM log_games__drives WHERE _parent = ?1 ORDER BY _i", &[i(row)]);
    if !drives.is_empty() {
        let ds: Vec<Value> = drives
            .iter()
            .map(|d| {
                json!({ "side": side_of(d), "q": int(d, "q"), "clock": clock(int(d, "q"), num(d, "clock")),
                        "start": (num(d, "start") * 10.0).round() / 10.0, "end": (num(d, "end") * 10.0).round() / 10.0,
                        "plays": int(d, "plays"), "secs": int(d, "secs"), "result": text(d, "result"), "points": int(d, "points"),
                        "first": int(d, "first"), "last": int(d, "last") })
            })
            .collect();
        let by = |s: usize| ds.iter().filter(|d| d["side"] == s).collect::<Vec<_>>();
        let per = |s: usize| {
            let d = by(s);
            let pts: i64 = d.iter().map(|x| x["points"].as_i64().unwrap_or(0)).sum();
            let st: f64 = d.iter().map(|x| x["start"].as_f64().unwrap_or(0.0)).sum::<f64>() / d.len().max(1) as f64;
            format!(
                "{}: {} drives, {:.2} points a drive, average start {}",
                sides[s],
                d.len(),
                pts as f64 / d.len().max(1) as f64,
                spot(100.0 - st)
            )
        };
        push(
            &mut p,
            json!({ "kind": "drives", "title": "Every drive", "note": format!("{} · {}", per(1), per(0)), "sides": sides, "drives": ds }),
        );
    }

    // Scoring plays, each in words.
    if !scoring.is_empty() {
        let rows: Vec<Value> = scoring
            .iter()
            .map(|r| {
                let s = side_of(r);
                let yds = num(r, "yards").round() as i64;
                let what = text(r, "what");
                let mut how = match text(r, "kind").as_str() {
                    "Pass" => {
                        format!("{} {yds}-yard pass from {}", who.name(int(r, "target")), who.name(int(r, "passer")))
                    }
                    "Run" | "Scramble" => format!("{} {yds}-yard run", who.name(int(r, "carrier"))),
                    "FieldGoal" => format!("{} {yds}-yard field goal", who.name(int(r, "kicker"))),
                    k if has(r, "returner") => {
                        format!("{} {yds}-yard {}", who.name(int(r, "returner")), k.to_lowercase())
                    }
                    k => format!("{} ({})", what, k.to_lowercase()),
                };
                if has(r, "try__kind") {
                    let good = num(r, "try__good") != 0.0;
                    how.push_str(&match text(r, "try__kind").as_str() {
                        "kick" => if good { " (kick good)" } else { " (kick missed)" }.to_string(),
                        k => format!(" ({k} try {})", if good { "good" } else { "failed" }),
                    });
                }
                let after = list(r, "after");
                json!([
                    clock(int(r, "q"), num(r, "clock")),
                    sides[s],
                    what,
                    how,
                    format!(
                        "{}–{}",
                        after.get(1).map(fmt).unwrap_or_default(),
                        after.first().map(fmt).unwrap_or_default()
                    )
                ])
            })
            .collect();
        push(
            &mut p,
            json!({ "kind": "table", "title": "Scoring plays", "note": format!("Score shown {as_}–{hs}"),
                    "cols": [{"name":"when","label":"When"},{"name":"team","label":"Team"},{"name":"what","label":"What"},{"name":"how","label":"How","minw":320},{"name":"score","label":"Score","num":true}],
                    "rows": rows }),
        );
    }

    // The team box against the level and real.
    if boxes.len() == 2 {
        let n = level_rows.len().max(1) as f64;
        let rows: Vec<Value> = BOX
            .iter()
            .map(|(label, k, rk)| {
                let avg = level_rows.iter().map(|r| box_val(r, k)).sum::<f64>() / n;
                let shown = |r: &Row| match *k {
                    "fg_made" => format!("{}/{}", int(r, "fg_made"), int(r, "fg_att")),
                    "clock_possession" => mmss(num(r, k)),
                    _ => one_dp(box_val(r, k)),
                };
                let avg_txt = if *k == "clock_possession" { mmss(avg * 60.0) } else { one_dp(avg) };
                json!([
                    label,
                    shown(&boxes[1]),
                    shown(&boxes[0]),
                    avg_txt,
                    real.get(*rk).and_then(Value::as_f64).map(one_dp).unwrap_or_else(|| "–".into())
                ])
            })
            .collect();
        let real_from = if stream == "nfl" { "nflverse 2021-25" } else { "cfbfastR 2022-24" };
        push(
            &mut p,
            json!({ "kind": "table", "title": "Team box", "note": format!("Each team's game against the average {group} team game in the world and real ({real_from}); sacks, interceptions and fumbles are the offense's"),
                    "cols": [{"name":"stat","label":"Stat"},{"name":"a","label":as_,"num":true},{"name":"h","label":hs,"num":true},
                             {"name":"w","label":format!("World avg ({group})"),"num":true},{"name":"r","label":"Real avg","num":true}],
                    "rows": rows }),
        );
    }

    // Player lines, by what each did.
    if !lines.is_empty() {
        player_tables(&mut p, &lines, &sides, &who);
    }

    // The play by play with every why.
    if !plays.is_empty() {
        let mut out: Vec<Value> = Vec::new();
        for (k, pl) in plays.iter().enumerate() {
            let off = pi(pl, "offense").clamp(0, 1) as usize;
            let score = pl.get("score").and_then(Value::as_array).cloned().unwrap_or_default();
            let next = plays
                .get(k + 1)
                .and_then(|n| n.get("score"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_else(|| vec![json!(hp), json!(ap)]);
            let mut flags: Vec<&str> = Vec::new();
            if score != next {
                flags.push("score");
            }
            if pb(pl, "turnover") || pb(pl, "interception") {
                flags.push("turnover");
            }
            if pl.get("penalties").is_some() || pl.get("nullified").is_some() {
                flags.push("flag");
            }
            if pf(pl, "yards") >= 20.0 || pf(pl, "epa").abs() >= 2.5 {
                flags.push("big");
            }
            let kind = pl.get("kind").and_then(Value::as_str).unwrap_or("");
            if pi(pl, "down") == 4 && !["Punt", "FieldGoal", "Kickoff", "ExtraPoint", "NoPlay", "Kneel"].contains(&kind)
            {
                flags.push("fourth");
            }
            let wp = pl.get("wp_after").and_then(Value::as_f64).map(|w| if off == 0 { w } else { 1.0 - w });
            let s_after: Vec<i64> = next.iter().map(|x| x.as_f64().unwrap_or(0.0) as i64).collect();
            out.push(json!({
                "n": pi(pl, "n"), "q": pi(pl, "quarter"), "clock": clock(pi(pl, "quarter"), pf(pl, "clock")), "side": off,
                "down": if ["Kickoff", "ExtraPoint", "TwoPoint", "Onside"].contains(&kind) { Value::Null } else { json!(format!("{} & {}", ordinal(pi(pl, "down")), if pf(pl, "togo") >= pf(pl, "yardline_100") - 0.5 && pf(pl, "yardline_100") < 10.0 { "goal".to_string() } else { pf(pl, "togo").round().max(1.0).to_string() })) },
                "spot": spot(pf(pl, "yardline_100")), "kind": kind,
                "why": pl.get("why").and_then(Value::as_str).unwrap_or(""),
                "call": pl.get("call"), "call_why": pl.get("call_why"), "defense": pl.get("defense_call"),
                "epa": pl.get("epa").and_then(Value::as_f64).map(|x| (x * 100.0).round() / 100.0),
                "wp": wp.map(|w| (w * 1000.0).round() / 10.0), "score": format!("{}–{}", s_after.get(1).unwrap_or(&0), s_after.first().unwrap_or(&0)),
                "flags": flags,
            }));
        }
        push(
            &mut p,
            json!({ "kind": "pbp", "title": "Play by play", "note": format!("Every snap with the engine's why, the call and why it was made, the defense, EPA and {hs}'s chance after it. Score shown {as_}–{hs}; names open the player"),
                    "sides": sides, "plays": out, "names": who.map() }),
        );
    }

    // The rest of the day: penalties, injuries, calls, timeouts and the coaches' choices.
    if !pens.is_empty() {
        let rows: Vec<Value> = pens
            .iter()
            .map(|r| {
                json!([
                    format!("Q{}", int(r, "q")),
                    sides[side_of(r)],
                    text(r, "kind"),
                    who.name(int(r, "player")),
                    if !has(r, "accepted") {
                        "declined or offset".to_string()
                    } else if has(r, "no_play") {
                        format!("{} yards, no play", int(r, "yards"))
                    } else {
                        format!("{} yards", int(r, "yards"))
                    },
                    int(r, "n")
                ])
            })
            .collect();
        let links: Vec<Value> = pens.iter().map(|r| json!(who.link(int(r, "player")))).collect();
        let by = |s: usize| pens.iter().filter(|r| side_of(r) == s && has(r, "accepted")).count();
        push(
            &mut p,
            json!({ "kind": "table", "title": format!("Penalties ({})", pens.len()), "note": format!("Accepted: {as_} {}, {hs} {}", by(1), by(0)),
                    "cols": [{"name":"q","label":"Q"},{"name":"t","label":"Team"},{"name":"k","label":"Foul"},{"name":"p","label":"Player"},{"name":"y","label":"Enforced"},{"name":"n","label":"Snap","num":true}],
                    "rows": rows, "links": links }),
        );
    }
    if !injuries.is_empty() {
        let rows: Vec<Value> = injuries
            .iter()
            .map(|r| {
                json!([
                    format!("Q{}", int(r, "q")),
                    sides[side_of(r)],
                    who.name(int(r, "player")),
                    text(r, "kind"),
                    format!("{:.1}", num(r, "weeks")),
                    if has(r, "plays_on") { "played on" } else { "left the game" },
                    int(r, "n")
                ])
            })
            .collect();
        let links: Vec<Value> = injuries.iter().map(|r| json!(who.link(int(r, "player")))).collect();
        push(
            &mut p,
            json!({ "kind": "table", "title": format!("Injuries ({})", injuries.len()),
                    "cols": [{"name":"q","label":"Q"},{"name":"t","label":"Team"},{"name":"p","label":"Player"},{"name":"k","label":"Injury"},{"name":"w","label":"Weeks out","num":true},{"name":"o","label":"Then"},{"name":"n","label":"Snap","num":true}],
                    "rows": rows, "links": links }),
        );
    }
    let mut ref_pairs: Vec<Value> = Vec::new();
    for (k, l) in [
        ("officiating__crew__holding", "Crew's holding rate (1 = league norm)"),
        ("officiating__crew__pass_interference", "Crew's interference rate"),
        ("officiating__crew__personal", "Crew's personal-foul rate"),
        ("officiating__crew__pre_snap", "Crew's pre-snap rate"),
        ("officiating__close_calls", "Close calls"),
        ("officiating__booth_reviews", "Booth reviews"),
        ("officiating__overturned", "Overturned"),
        ("officiating__wrong", "Wrong calls"),
        ("officiating__wrong_left_standing", "Wrong calls left standing"),
    ] {
        if has(&g, k) {
            ref_pairs.push(json!({ "k": l, "v": fmt(&g[k]) }));
        }
    }
    let ch = list(&g, "officiating__challenges");
    if !ch.is_empty() {
        let won = list(&g, "officiating__challenges_won");
        let w = |s: usize| won.get(s).map(fmt).unwrap_or_else(|| "0".into());
        ref_pairs.push(json!({ "k": "Challenges (won)", "v": format!("{as_} {} ({}), {hs} {} ({})", fmt(&ch.get(1).cloned().unwrap_or(json!(0))), w(1), fmt(&ch[0]), w(0)) }));
    }
    let mut ref_items: Vec<Value> = calls
        .iter()
        .map(|r| {
            json!({ "text": format!("Snap {}: {}", int(r, "n"), who.words(&text(r, "note"))),
                    "why": format!("{}{}{}", text(r, "call"), if has(r, "overturned") { " · overturned" } else { "" }, if has(r, "wrong") { " · the call was wrong" } else { "" }) })
        })
        .collect();
    if !ref_pairs.is_empty() || !ref_items.is_empty() {
        let mut blocks = Vec::new();
        if !ref_pairs.is_empty() {
            blocks.push(json!({ "kind": "kv", "title": "The officials", "pairs": ref_pairs }));
        }
        if ref_items.is_empty() {
            ref_items.push(json!({ "text": "No call was reviewed." }));
        }
        blocks.push(json!({ "kind": "list", "title": "Reviewed calls", "items": ref_items, "dense": true }));
        push(&mut p, json!({ "kind": "pair", "blocks": blocks }));
    }
    let between = all(b, "SELECT * FROM log_games__between WHERE _parent = ?1 ORDER BY _i", &[i(row)]);
    let decisions = all(b, "SELECT * FROM log_games__decisions WHERE _parent = ?1 ORDER BY _i", &[i(row)]);
    if !between.is_empty() || !decisions.is_empty() {
        let bi: Vec<Value> = between
            .iter()
            .map(|r| json!({ "text": who.words(&text(r, "what")), "why": if has(r, "after") { format!("After snap {}", int(r, "after")) } else { "Before kickoff".into() } }))
            .collect();
        let di: Vec<Value> = decisions
            .iter()
            .map(|r| json!({ "text": text(r, "what"), "why": format!("{} · snap {}", sides[side_of(r)], int(r, "n")) }))
            .collect();
        let mut blocks = vec![
            json!({ "kind": "list", "title": "Between snaps", "note": "Toss, timeouts and challenges", "items": bi, "dense": true }),
        ];
        if !di.is_empty() {
            blocks.push(json!({ "kind": "list", "title": "The coaches' big choices", "items": di, "dense": true }));
        }
        push(&mut p, json!({ "kind": "pair", "blocks": blocks }));
    }
    let trust = all(b, "SELECT * FROM log_games__trust WHERE _parent = ?1 ORDER BY _i", &[i(row)]);
    if !trust.is_empty() {
        let items: Vec<Value> = trust
            .iter()
            .map(|r| json!({ "text": format!("{}: the coaches' trust {} {:.2}", who.name(int(r, "player")), if num(r, "change") < 0.0 { "fell" } else { "rose" }, num(r, "change").abs()),
                             "link": who.link(int(r, "player")), "why": sides[side_of(r)].clone() }))
            .collect();
        push(&mut p, json!({ "kind": "list", "title": "Trust earned and lost", "items": items, "dense": true }));
    }
    let winner = if hp > ap { &hn } else { &an };
    p["verdict"] = json!(if hp == ap {
        format!("A {hp}–{ap} tie.")
    } else {
        format!("{winner} won by {}. {} snaps, every one with its why below.", (hp - ap).abs(), plays.len())
    });
    let _ = f;
    Ok(p)
}

/// One player-line table: who, then the columns, for the rows `keep` takes.
struct Line {
    title: &'static str,
    note: &'static str,
    keep: fn(&Row) -> bool,
    order: fn(&Row) -> f64,
    cols: &'static [(&'static str, fn(&Row) -> Value)],
}

fn ratio(r: &Row, a: &str, b: &str, scale: f64) -> Value {
    let d = num(r, b);
    if d <= 0.0 { Value::Null } else { json!(((num(r, a) / d) * scale * 10.0).round() / 10.0) }
}

fn of(r: &Row, k: &str) -> Value {
    if has(r, k) { json!((num(r, k) * 10.0).round() / 10.0) } else { json!(0) }
}

fn frac(r: &Row, a: &str, b: &str) -> Value {
    json!(format!("{}/{}", int(r, a), int(r, b)))
}

const LINES: [Line; 8] = [
    Line {
        title: "Passing",
        note: "Yards are the passer's own, before sack yards come off; CPOE is completions over expected per 100 throws",
        keep: |r| num(r, "att") > 0.0,
        order: |r| num(r, "pass_yds"),
        cols: &[
            ("C/Att", |r| frac(r, "cmp", "att")),
            ("Yards", |r| of(r, "pass_yds")),
            ("TD", |r| of(r, "pass_td")),
            ("Int", |r| of(r, "int")),
            ("Sacked", |r| of(r, "sacked")),
            ("Y/A", |r| ratio(r, "pass_yds", "att", 1.0)),
            ("CPOE", |r| {
                let a = num(r, "att");
                if a > 0.0 {
                    json!(((num(r, "cmp") - num(r, "x_cmp")) / a * 1000.0).round() / 10.0)
                } else {
                    Value::Null
                }
            }),
            ("aDOT", |r| ratio(r, "air_yds_att", "att", 1.0)),
            ("Time to throw", |r| ratio(r, "ttt", "dropbacks", 1.0)),
            ("Pressured", |r| of(r, "pressured")),
            ("Big-time throws", |r| of(r, "big_time")),
            ("Turnover-worthy", |r| of(r, "turnover_worthy")),
            ("EPA", |r| of(r, "pass_epa")),
        ],
    },
    Line {
        title: "Rushing",
        note: "Before contact and after it, per carry",
        keep: |r| num(r, "rush") > 0.0,
        order: |r| num(r, "rush_yds"),
        cols: &[
            ("Carries", |r| of(r, "rush")),
            ("Yards", |r| of(r, "rush_yds")),
            ("Y/C", |r| ratio(r, "rush_yds", "rush", 1.0)),
            ("TD", |r| of(r, "rush_td")),
            ("Long", |r| of(r, "rush_long")),
            ("Before contact /c", |r| ratio(r, "ybc", "rush", 1.0)),
            ("After contact /c", |r| ratio(r, "yaco", "rush", 1.0)),
            ("Missed tackles forced", |r| of(r, "forced_missed")),
            ("Success %", |r| ratio(r, "rush_success", "rush", 100.0)),
            ("First downs", |r| of(r, "rush_first")),
            ("Fumbles", |r| of(r, "fumbles")),
            ("EPA", |r| of(r, "rush_epa")),
        ],
    },
    Line {
        title: "Receiving",
        note: "Separation is the average at the catch point, in yards",
        keep: |r| num(r, "targets") > 0.0 || num(r, "rec") > 0.0,
        order: |r| num(r, "rec_yds"),
        cols: &[
            ("Targets", |r| of(r, "targets")),
            ("Catches", |r| of(r, "rec")),
            ("Yards", |r| of(r, "rec_yds")),
            ("TD", |r| of(r, "rec_td")),
            ("Long", |r| of(r, "rec_long")),
            ("After catch", |r| of(r, "yac")),
            ("Separation", |r| ratio(r, "sep_sum", "targets", 1.0)),
            ("Air yards", |r| of(r, "tgt_air")),
            ("Drops", |r| of(r, "drops")),
            ("Contested", |r| frac(r, "contested_rec", "contested_tgt")),
            ("First downs", |r| of(r, "rec_first")),
            ("EPA", |r| of(r, "rec_epa")),
        ],
    },
    Line {
        title: "Defense",
        note: "Tackling, the pass rush and coverage (targets, catches and yards allowed)",
        keep: |r| num(r, "snaps_def") > 0.0 || num(r, "tackles") > 0.0,
        order: |r| num(r, "tackles") + num(r, "assists") + 3.0 * num(r, "sacks") + 3.0 * num(r, "ints"),
        cols: &[
            ("Snaps", |r| of(r, "snaps_def")),
            ("Tackles", |r| of(r, "tackles")),
            ("Assists", |r| of(r, "assists")),
            ("For loss", |r| of(r, "tfl")),
            ("Stops", |r| of(r, "stops")),
            ("Missed", |r| of(r, "missed")),
            ("Sacks", |r| of(r, "sacks")),
            ("Hits", |r| of(r, "qb_hits")),
            ("Pressures", |r| of(r, "pressures")),
            ("Int", |r| of(r, "ints")),
            ("Breakups", |r| of(r, "pbus")),
            ("Forced fumbles", |r| of(r, "forced_fumbles")),
            ("Targeted", |r| of(r, "tgt_allowed")),
            ("Allowed", |r| frac(r, "cmp_allowed", "tgt_allowed")),
            ("Yards allowed", |r| of(r, "yds_allowed")),
            ("TD allowed", |r| of(r, "td_allowed")),
        ],
    },
    Line {
        title: "In the trenches",
        note: "Block and rush wins per snap: how each lineman's battles went",
        keep: |r| num(r, "pb_snaps") + num(r, "rb_snaps") + num(r, "rush_snaps") > 0.0,
        order: |r| num(r, "pb_snaps") + num(r, "rb_snaps") + num(r, "rush_snaps"),
        cols: &[
            ("Pass-block snaps", |r| of(r, "pb_snaps")),
            ("Pass-block win %", |r| ratio(r, "pb_wins", "pb_snaps", 100.0)),
            ("Pressures allowed", |r| of(r, "pressures_allowed")),
            ("Sacks allowed", |r| of(r, "sacks_allowed")),
            ("Run-block snaps", |r| of(r, "rb_snaps")),
            ("Run-block win %", |r| ratio(r, "rb_wins", "rb_snaps", 100.0)),
            ("Rush snaps", |r| of(r, "rush_snaps")),
            ("Rush win %", |r| ratio(r, "rush_wins", "rush_snaps", 100.0)),
            ("Speed wins", |r| of(r, "speed_wins")),
            ("Power wins", |r| of(r, "power_wins")),
            ("Finesse wins", |r| of(r, "finesse_wins")),
        ],
    },
    Line {
        title: "Kicking and punting",
        note: "Clutch kicks are the ones that decide the game late",
        keep: |r| num(r, "fg_att") + num(r, "xp_att") + num(r, "kickoffs") + num(r, "punts") > 0.0,
        order: |r| num(r, "fg_att") + num(r, "punts"),
        cols: &[
            ("Field goals", |r| frac(r, "fg_made", "fg_att")),
            ("Long", |r| of(r, "fg_long")),
            ("Extra points", |r| frac(r, "xp_made", "xp_att")),
            ("Clutch", |r| frac(r, "clutch_made", "clutch_att")),
            ("Kickoffs", |r| of(r, "kickoffs")),
            ("Touchbacks", |r| of(r, "ko_tb")),
            ("Punts", |r| of(r, "punts")),
            ("Gross avg", |r| ratio(r, "punt_yds", "punts", 1.0)),
            ("Net avg", |r| ratio(r, "punt_net", "punts", 1.0)),
            ("Inside 20", |r| of(r, "punt_in20")),
        ],
    },
    Line {
        title: "Returns",
        note: "Kick and punt returns",
        keep: |r| num(r, "kr") + num(r, "pr") > 0.0,
        order: |r| num(r, "kr_yds") + num(r, "pr_yds"),
        cols: &[
            ("Kick returns", |r| of(r, "kr")),
            ("Yards", |r| of(r, "kr_yds")),
            ("Avg", |r| ratio(r, "kr_yds", "kr", 1.0)),
            ("TD", |r| of(r, "kr_td")),
            ("Punt returns", |r| of(r, "pr")),
            ("Yards", |r| of(r, "pr_yds")),
            ("Avg", |r| ratio(r, "pr_yds", "pr", 1.0)),
            ("TD", |r| of(r, "pr_td")),
        ],
    },
    Line {
        title: "Snaps and grades",
        note: "Everyone who played: snaps by unit and the engine's average play grade",
        keep: |_| true,
        order: |r| num(r, "snaps_off") + num(r, "snaps_def") + num(r, "snaps_st") / 3.0,
        cols: &[
            ("Offense", |r| of(r, "snaps_off")),
            ("Defense", |r| of(r, "snaps_def")),
            ("Special teams", |r| of(r, "snaps_st")),
            ("Grade", |r| ratio(r, "grade_sum", "graded", 1.0)),
            ("Graded snaps", |r| of(r, "graded")),
            ("Busts", |r| of(r, "busts")),
            ("Penalties", |r| of(r, "penalties")),
            ("Top speed (mph)", |r| of(r, "top_speed")),
        ],
    },
];

fn player_tables(p: &mut Value, lines: &[Row], sides: &[String; 2], who: &People) {
    let mut tabs: Vec<Value> = Vec::new();
    for l in &LINES {
        let mut rows: Vec<&Row> = lines.iter().filter(|r| (l.keep)(r)).collect();
        if rows.is_empty() {
            continue;
        }
        // Away first, as the score reads; the busiest first within a team.
        rows.sort_by(|a, b| int(b, "_i").cmp(&int(a, "_i")).then((l.order)(b).total_cmp(&(l.order)(a))));
        let mut cols = vec![
            json!({ "name": "player", "label": "Player", "minw": 170 }),
            json!({ "name": "pos", "label": "Pos" }),
            json!({ "name": "team", "label": "Team" }),
        ];
        cols.extend(l.cols.iter().map(|(n, _)| json!({ "name": n, "label": n, "num": true })));
        let out: Vec<Value> = rows
            .iter()
            .map(|r| {
                let mut v = vec![
                    json!(who.name(int(r, "id"))),
                    json!(text(r, "pos")),
                    json!(sides[int(r, "_i").clamp(0, 1) as usize]),
                ];
                v.extend(l.cols.iter().map(|(_, f)| f(r)));
                Value::Array(v)
            })
            .collect();
        let links: Vec<Value> = rows.iter().map(|r| json!(who.link(int(r, "id")))).collect();
        tabs.push(json!({ "name": l.title, "note": l.note, "cols": cols, "rows": out, "links": links }));
    }
    push(
        p,
        json!({ "kind": "tabtables", "title": "Player lines", "note": "Every player's game: pick a part of it", "tabs": tabs }),
    );
}

// ---------------------------------------------------------------- NFL club

const ROUND_WORDS: [&str; 6] = [
    "missed the playoffs",
    "lost in the wild card round",
    "lost in the divisional round",
    "lost the conference title game",
    "lost the Super Bowl",
    "won the Super Bowl",
];

fn round_word(r: i64) -> &'static str {
    ROUND_WORDS[r.clamp(0, 5) as usize]
}

fn coach_name(b: &Book, id: i64) -> String {
    one(b, "SELECT name FROM coaches WHERE id = ?1", &[i(id)]).map(|r| text(&r, "name")).unwrap_or_default()
}

/// The book lines of an NFL club's year: income and costs by line.
fn lines_named(b: &Book, level: &str) -> Vec<(String, bool)> {
    all(b, "SELECT name, income FROM engine_finance_lines WHERE level = ?1 ORDER BY i", &[Sql::Text(level.into())])
        .iter()
        .map(|r| (text(r, "name"), num(r, "income") != 0.0))
        .collect()
}

fn sum_lines(l: &[Value], names: &[(String, bool)], income: bool) -> f64 {
    l.iter().zip(names).filter(|(_, (_, inc))| *inc == income).map(|(v, _)| v.as_f64().unwrap_or(0.0)).sum()
}

/// Every pick on record, with where the player went to school.
const DRAFTED: &str = "WITH dr AS (
    SELECT p.player__id AS id, p.name, p.player__position AS pos, p.pro__draft AS draft, p.college AS college, p.pro__team AS now_team, p.pro__status AS status, 'people' AS tbl, p.x_overall AS ovr FROM people p WHERE p.pro__draft IS NOT NULL
    UNION ALL SELECT a.player__id, a.name, a.position, a.pro__draft, NULL, NULL, 'retired', 'alumni', a.overall FROM alumni a WHERE a.pro__draft IS NOT NULL)
  SELECT dr.id, dr.name, dr.pos, json_extract(dr.draft, '$[0]') AS year, json_extract(dr.draft, '$[1]') AS round, json_extract(dr.draft, '$[2]') AS pick,
         json_extract(dr.draft, '$[3]') AS team, dr.status, dr.now_team, dr.tbl, dr.ovr,
         (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = dr.college) AS college,
         (SELECT conference FROM programs WHERE _row = dr.college) AS conference
    FROM dr";

pub fn club(b: &mut Book, team: i64, f: &Filters) -> Result<Value, String> {
    let c = one(b, "SELECT * FROM nfl WHERE _row = ?1", &[i(team)])
        .ok_or_else(|| format!("No NFL club {team} in this book."))?;
    let name = text(&c, "name");
    let mut p = new_page(&name, &format!("{} · NFL · {name}", world_name(b)));
    let seasons = all(b, "SELECT * FROM nfl_seasons WHERE team = ?1 ORDER BY year", &[i(team)]);
    let fin = one(b, "SELECT * FROM finance__nfl WHERE _row = ?1", &[i(team)]);
    let titles: Vec<i64> = all(b, "SELECT _0 AS y FROM nfl_champions WHERE _1 = ?1 ORDER BY _0", &[i(team)])
        .iter()
        .map(|r| int(r, "y"))
        .collect();
    let (w, l, t) =
        seasons.iter().fold((0, 0, 0), |a, s| (a.0 + int(s, "wins"), a.1 + int(s, "losses"), a.2 + int(s, "ties")));
    let mut text_parts = Vec::new();
    if let Some(fr) = &fin {
        text_parts.push(format!(
            "{} ({} seats, opened {})",
            text(fr, "home__name"),
            fmt(&json!(int(fr, "home__capacity"))),
            int(fr, "home__opened")
        ));
        text_parts.push(format!("owner {} since {}", text(fr, "owner_name"), int(fr, "owner_since")));
    }
    let hc = coach_name(b, int(&c, "head_coach"));
    if !hc.is_empty() {
        text_parts.push(format!("head coach {hc}"));
    }
    let mut figures = vec![
        json!({ "label": "Record on file", "value": format!("{w}-{l}{}", if t > 0 { format!("-{t}") } else { String::new() }) }),
        json!({ "label": "Playoff years", "value": seasons.iter().filter(|s| int(s, "round") > 0).count() }),
        json!({ "label": "Titles", "value": titles.len() }),
    ];
    if let Some(fr) = &fin {
        figures.push(json!({ "label": "Value", "value": money(num(fr, "value")) }));
    }
    let last = seasons.last().map(|s| int(s, "year"));
    let mut actions = Vec::new();
    if let Some(y) = last {
        actions.push(json!({ "text": format!("{y} season"), "link": format!("club/{team}/{y}") }));
    }
    actions.push(json!({ "text": "Every NFL club", "link": "area/N1" }));
    push(
        &mut p,
        json!({ "kind": "hero", "label": "NFL club", "title": name, "text": text_parts.join(" · "),
                "tags": titles.iter().map(|y| format!("Champions {y}")).collect::<Vec<_>>(), "figures": figures, "actions": actions }),
    );

    // Every season as a strip, then the eras.
    if !seasons.is_empty() {
        let items: Vec<Value> = seasons
            .iter()
            .map(|s| {
                let y = int(s, "year");
                json!({ "year": y, "w": int(s, "wins"), "l": int(s, "losses"), "t": int(s, "ties"), "round": int(s, "round"),
                        "text": format!("{y}: {}-{}{}, {} (coach {})", int(s, "wins"), int(s, "losses"), if int(s, "ties") > 0 { format!("-{}", int(s, "ties")) } else { String::new() }, round_word(int(s, "round")), coach_name(b, int(s, "coach"))),
                        "link": format!("club/{team}/{y}") })
            })
            .collect();
        push(
            &mut p,
            json!({ "kind": "strip", "title": "Every season", "note": "Colour by win share; a ring for the playoffs, gold for a title. Open a season for its games, lines and books", "items": items }),
        );
    }
    let eras = all(b, "SELECT * FROM engine_franchises WHERE team = ?1 ORDER BY \"from\"", &[i(team)]);
    if !eras.is_empty() {
        let rows: Vec<Value> = eras
            .iter()
            .map(|e| {
                json!([
                    int(e, "from"),
                    if int(e, "to") >= 9000 { json!("now") } else { json!(int(e, "to")) },
                    text(e, "name"),
                    format!("{}, {}", text(e, "city"), text(e, "state")),
                    text(e, "conference"),
                    text(e, "roof"),
                    text(e, "fictional")
                ])
            })
            .collect();
        push(
            &mut p,
            json!({ "kind": "table", "title": "Eras", "note": "The franchise on its real dates; the last column is the name the fictional-league option shows",
                    "cols": [{"name":"from","label":"From","fmt":"year"},{"name":"to","label":"To","fmt":"year"},{"name":"n","label":"Name"},{"name":"c","label":"Home"},{"name":"conf","label":"Conference"},{"name":"r","label":"Roof"},{"name":"fic","label":"Fictional name"}],
                    "rows": rows }),
        );
    }

    // The money against the league.
    let names = lines_named(b, "nfl");
    let years = all(
        b,
        "SELECT y.year, y.l, (SELECT AVG(x.n) FROM (SELECT (SELECT SUM(j.value) FROM json_each(z.l) j WHERE j.key < 10) AS n FROM finance__nfl__years z WHERE z.year = y.year) x) AS league
           FROM finance__nfl__years y WHERE y._parent = ?1 ORDER BY y.year",
        &[i(team)],
    );
    if !years.is_empty() && !names.is_empty() {
        let mut club_pts = Vec::new();
        let mut lg_pts = Vec::new();
        let mut rows = Vec::new();
        for y in &years {
            let l = list(y, "l");
            let rev = sum_lines(&l, &names, true);
            let cost = sum_lines(&l, &names, false);
            club_pts.push(json!([int(y, "year"), (rev / 1e6).round()]));
            lg_pts.push(json!([int(y, "year"), (num(y, "league") / 1e6).round()]));
            rows.push(json!([
                int(y, "year"),
                money(rev),
                money(cost),
                money(rev - cost),
                money(num(y, "league")),
                format!(
                    "{:.0}%",
                    100.0
                        * (l.get(10).and_then(Value::as_f64).unwrap_or(0.0)
                            + l.get(11).and_then(Value::as_f64).unwrap_or(0.0))
                        / rev.max(1.0)
                )
            ]));
        }
        push(
            &mut p,
            json!({ "kind": "chart", "type": "line", "title": "Income against the league ($M)", "note": "The club's income each year and the average club's",
                    "x": "Year", "series": [{"name": name, "points": club_pts}, {"name": "League average", "points": lg_pts, "style": "dash"}] }),
        );
        rows.reverse();
        push(
            &mut p,
            json!({ "kind": "table", "title": "The books by year", "note": "Player share is cash pay and benefits over income (real about 44-48% under the CBA)",
                    "cols": [{"name":"y","label":"Year","fmt":"year"},{"name":"r","label":"Income","num":true},{"name":"c","label":"Costs","num":true},{"name":"n","label":"Net","num":true},{"name":"l","label":"League avg income","num":true},{"name":"s","label":"Player share","num":true}],
                    "rows": rows, "links": years.iter().rev().map(|y| json!(format!("club/{team}/{}", int(y, "year")))).collect::<Vec<_>>() }),
        );
    }
    let crowds = all(b, "SELECT * FROM finance__nfl__seasons WHERE _parent = ?1 ORDER BY year DESC", &[i(team)]);
    if !crowds.is_empty() {
        let rows: Vec<Value> = crowds
            .iter()
            .map(|s| {
                json!([
                    int(s, "year"),
                    int(s, "home_games"),
                    fmt(&json!((num(s, "attendance") / num(s, "home_games").max(1.0)).round())),
                    format!(
                        "{:.0}%",
                        100.0 * num(s, "attendance") / (num(s, "capacity") * num(s, "home_games")).max(1.0)
                    ),
                    int(s, "sellouts"),
                    int(s, "blackouts"),
                    format!("${:.0}", num(s, "price")),
                    format!("{:.0}%", 100.0 * num(s, "renewal")),
                    fmt(&json!(int(s, "waitlist"))),
                    money(num(s, "value"))
                ])
            })
            .collect();
        push(
            &mut p,
            json!({ "kind": "table", "title": "Crowds, tickets and value", "note": "Per home game; renewal is season-ticket holders who came back",
                    "cols": [{"name":"y","label":"Year","fmt":"year"},{"name":"g","label":"Home games","num":true},{"name":"a","label":"Per game","num":true},{"name":"f","label":"Full","num":true},{"name":"s","label":"Sellouts","num":true},{"name":"b","label":"Blackouts","num":true},{"name":"p","label":"Avg ticket","num":true},{"name":"r","label":"Renewal","num":true},{"name":"w","label":"Waitlist","num":true},{"name":"v","label":"Value","num":true}],
                    "rows": rows }),
        );
    }

    // The people: roster, depth, the office.
    roster(b, &mut p, team, f);
    let mut office: Vec<Value> = Vec::new();
    if let Some(fr) = &fin {
        office.push(
            json!({ "k": "Owner", "v": format!("{} (since {})", text(fr, "owner_name"), int(fr, "owner_since")) }),
        );
    }
    for (k, l) in [("head_coach", "Head coach"), ("oc", "Offensive coordinator"), ("dc", "Defensive coordinator")] {
        if has(&c, k) {
            office.push(json!({ "k": l, "v": coach_name(b, int(&c, k)), "link": format!("e/coaches/{}", int(&c, k)) }));
        }
    }
    if let Some(cl) = one(b, "SELECT * FROM clubs WHERE _row = ?1", &[i(team)]) {
        if has(&cl, "office__gm__name") {
            office.push(json!({ "k": "General manager", "v": format!("{} (since {})", text(&cl, "office__gm__name"), int(&cl, "office__gm__since")) }));
        }
        for (k, l) in [
            ("office__power", "Who decides"),
            ("office__fans", "Fans"),
            ("team_mood", "Team mood"),
            ("support__strength__name", "Strength coach"),
            ("support__medical__name", "Head of medicine"),
            ("support__analytics__name", "Analytics"),
        ] {
            if has(&cl, k) && (f.debug || !b.is_debug("clubs", k)) {
                office.push(json!({ "k": l, "v": fmt(&cl[k]) }));
            }
        }
        if f.debug {
            for k in [
                "office__gm__skill",
                "office__gm__film_lean",
                "office__gm__risk",
                "office__owner__meddling",
                "office__owner__patience",
                "office__owner__cash_will",
            ] {
                if has(&cl, k) {
                    office.push(json!({ "k": crate::book::label(k), "v": fmt(&cl[k]), "debug": true }));
                }
            }
        }
    }
    if !office.is_empty() {
        push(&mut p, json!({ "kind": "kv", "title": "Who runs it", "pairs": office }));
    }

    // Picks, trades and the last games.
    let picks = all(
        b,
        &format!("{DRAFTED} WHERE json_extract(dr.draft, '$[3]') = ?1 ORDER BY year DESC, round, pick LIMIT 80"),
        &[i(team)],
    );
    if !picks.is_empty() {
        let rows: Vec<Value> = picks
            .iter()
            .map(|r| {
                json!([
                    int(r, "year"),
                    format!("{}.{:02}", int(r, "round"), int(r, "pick")),
                    text(r, "name"),
                    text(r, "pos"),
                    text(r, "college"),
                    plain(&text(r, "status"))
                ])
            })
            .collect();
        let links: Vec<Value> = picks.iter().map(|r| json!(format!("e/{}/{}", text(r, "tbl"), int(r, "id")))).collect();
        push(
            &mut p,
            json!({ "kind": "table", "title": format!("Draft picks on record ({})", picks.len()), "note": "Round.pick; open the year's whole class from the draft page",
                    "cols": [{"name":"y","label":"Year","fmt":"year"},{"name":"p","label":"Pick"},{"name":"n","label":"Player"},{"name":"pos","label":"Pos"},{"name":"c","label":"College"},{"name":"s","label":"Now"}],
                    "rows": rows, "links": links }),
        );
    }
    let trades = all(
        b,
        "SELECT date, text FROM market__picks__trades WHERE buyer = ?1 OR seller = ?1 ORDER BY date DESC LIMIT 40",
        &[i(team)],
    );
    if !trades.is_empty() {
        let items: Vec<Value> = trades
            .iter()
            .map(|r| json!({ "text": text(r, "text"), "why": crate::pages::day_text(int(r, "date")) }))
            .collect();
        push(&mut p, json!({ "kind": "list", "title": "Trades", "items": items, "dense": true }));
    }
    let games = all(
        b,
        "SELECT _row, season, date, kind, week, home, away, home_pts, away_pts, neutral FROM log_games WHERE stream = 'nfl' AND (home = ?1 OR away = ?1) AND home_pts IS NOT NULL ORDER BY date DESC LIMIT 25",
        &[i(team)],
    );
    if !games.is_empty() {
        let (rows, links) = game_rows(b, &games, team);
        push(
            &mut p,
            json!({ "kind": "table", "title": "Latest games", "cols": game_cols(), "rows": rows, "links": links }),
        );
    }
    p["verdict"] = json!(format!(
        "{name}: {w}-{l}{} over {} seasons on file, {} playoff years, {}.",
        if t > 0 { format!("-{t}") } else { String::new() },
        seasons.len(),
        seasons.iter().filter(|s| int(s, "round") > 0).count(),
        if titles.is_empty() { "no title".to_string() } else { plural(titles.len() as i64, "title", "titles") }
    ));
    Ok(p)
}

fn game_cols() -> Value {
    json!([{"name":"d","label":"Date"},{"name":"k","label":"Game"},{"name":"o","label":"Opponent"},{"name":"r","label":"Result"},{"name":"s","label":"Score","num":true}])
}

fn game_rows(b: &Book, games: &[Row], team: i64) -> (Vec<Value>, Vec<Value>) {
    let mut rows = Vec::new();
    let mut links = Vec::new();
    for g in games {
        let home = int(g, "home") == team;
        let opp = if home { int(g, "away") } else { int(g, "home") };
        let (us, them) =
            if home { (int(g, "home_pts"), int(g, "away_pts")) } else { (int(g, "away_pts"), int(g, "home_pts")) };
        let kind = text(g, "kind");
        let game = if kind == "regular" || kind.is_empty() { format!("Week {}", int(g, "week")) } else { kind };
        rows.push(json!([
            text(g, "date"),
            game,
            format!(
                "{}{}",
                if has(g, "neutral") {
                    "vs "
                } else if home {
                    "vs "
                } else {
                    "at "
                },
                team_name(b, "nfl", int(g, "season"), opp)
            ) + if has(g, "neutral") { " (neutral site)" } else { "" },
            if us > them {
                "W"
            } else if us < them {
                "L"
            } else {
                "T"
            },
            format!("{us}–{them}")
        ]));
        links.push(json!(format!("game/{}", int(g, "_row"))));
    }
    (rows, links)
}

fn roster(b: &Book, p: &mut Value, team: i64, f: &Filters) {
    let show_ovr = f.debug || !b.is_debug("people", "x_overall");
    let rows = all(
        b,
        "SELECT player__id AS id, name, player__position AS pos, player__age AS age, pro__status AS status, pro__contract__kind AS deal, pro__earnings AS earned, pro__accrued AS accrued, x_overall AS ovr,
                pro__draft AS draft, (SELECT COALESCE(real_name, fictional_name) FROM programs WHERE _row = people.college) AS college
           FROM people WHERE pro__team = ?1 ORDER BY CASE pro__status WHEN 'Roster' THEN 0 WHEN 'Reserve' THEN 1 WHEN 'PracticeSquad' THEN 2 ELSE 3 END, x_overall DESC",
        &[i(team)],
    );
    if rows.is_empty() {
        return;
    }
    let mut cols = vec![
        json!({"name":"n","label":"Player","minw":170}),
        json!({"name":"pos","label":"Pos"}),
        json!({"name":"a","label":"Age","num":true}),
        json!({"name":"s","label":"Status"}),
    ];
    if show_ovr {
        cols.push(json!({"name":"o","label":"Overall","num":true}));
    }
    cols.extend([
        json!({"name":"d","label":"Deal"}),
        json!({"name":"dr","label":"Drafted"}),
        json!({"name":"c","label":"College"}),
        json!({"name":"ac","label":"Accrued","num":true}),
        json!({"name":"e","label":"Career pay","num":true}),
    ]);
    let out: Vec<Value> = rows
        .iter()
        .map(|r| {
            let d = list(r, "draft");
            let drafted = if d.len() >= 3 {
                format!("{} {}.{:02}", fmt(&d[0]), fmt(&d[1]), d[2].as_i64().unwrap_or(0))
            } else {
                "undrafted".into()
            };
            let mut v = vec![
                json!(text(r, "name")),
                json!(text(r, "pos")),
                json!(int(r, "age")),
                json!(plain(&text(r, "status"))),
            ];
            if show_ovr {
                v.push(json!(one_dp(num(r, "ovr"))));
            }
            v.extend([
                json!(plain(&text(r, "deal"))),
                json!(drafted),
                json!(text(r, "college")),
                json!(int(r, "accrued")),
                json!(money(num(r, "earned"))),
            ]);
            Value::Array(v)
        })
        .collect();
    let links: Vec<Value> = rows.iter().map(|r| json!(format!("e/people/{}", int(r, "id")))).collect();
    let on = rows.iter().filter(|r| text(r, "status") == "Roster").count();
    push(
        p,
        json!({ "kind": "table", "title": format!("Roster ({on} on the 53, {} in all)", rows.len()), "note": "The 53 first, then the reserve lists and the practice squad",
                "cols": cols, "rows": out, "links": links }),
    );
    let depth = all(
        b,
        "SELECT d._i, d._j, d.id, d.role, d.rating, d.age, p.name, p.player__position AS pos FROM nfl__depth d LEFT JOIN people p ON p.player__id = d.id WHERE d._parent = ?1 ORDER BY d._i, d._j",
        &[i(team)],
    );
    if !depth.is_empty() {
        let mut groups: Vec<(i64, Vec<&Row>)> = Vec::new();
        for r in &depth {
            match groups.last_mut() {
                Some((g, v)) if *g == int(r, "_i") => v.push(r),
                _ => groups.push((int(r, "_i"), vec![r])),
            }
        }
        let show = f.debug || !b.is_debug("nfl__depth", "rating");
        let items: Vec<Value> = groups
            .iter()
            .map(|(_, v)| {
                let pos = v.iter().map(|r| text(r, "pos")).find(|s| !s.is_empty()).unwrap_or_default();
                let who: Vec<String> = v
                    .iter()
                    .map(|r| {
                        let tag = match text(r, "role").as_str() {
                            "Starter" => "",
                            "Rotation" => " (rotation)",
                            _ => " (backup)",
                        };
                        if show {
                            format!("{}{tag} {}", text(r, "name"), one_dp(num(r, "rating")))
                        } else {
                            format!("{}{tag}", text(r, "name"))
                        }
                    })
                    .collect();
                json!({ "text": format!("{pos}: {}", who.join(", ")) })
            })
            .collect();
        push(
            p,
            json!({ "kind": "list", "title": "Depth chart", "note": "Starters first, as the coaches set it", "items": items, "dense": true }),
        );
    }
}

// ---------------------------------------------------------------- club season

pub fn club_season(b: &mut Book, team: i64, year: i64, f: &Filters) -> Result<Value, String> {
    let name = team_name(b, "nfl", year, team);
    let title = format!("{name}, {year}");
    let mut p = new_page(&title, &format!("{} · NFL · {name} · {year}", world_name(b)));
    let s = one(b, "SELECT * FROM nfl_seasons WHERE team = ?1 AND year = ?2", &[i(team), i(year)]);
    let games = all(
        b,
        "SELECT _row, season, date, kind, week, home, away, home_pts, away_pts, attendance, neutral FROM log_games WHERE stream = 'nfl' AND season = ?2 AND (home = ?1 OR away = ?1) ORDER BY date",
        &[i(team), i(year)],
    );
    let (mut pf_, mut pa_) = (0, 0);
    for g in games.iter().filter(|g| text(g, "kind") == "regular" && has(g, "home_pts")) {
        let home = int(g, "home") == team;
        pf_ += if home { int(g, "home_pts") } else { int(g, "away_pts") };
        pa_ += if home { int(g, "away_pts") } else { int(g, "home_pts") };
    }
    let mut figures = Vec::new();
    let mut words = Vec::new();
    if let Some(s) = &s {
        figures.push(json!({ "label": "Record", "value": format!("{}-{}{}", int(s, "wins"), int(s, "losses"), if int(s, "ties") > 0 { format!("-{}", int(s, "ties")) } else { String::new() }) }));
        words.push(format!("{} {}", name, round_word(int(s, "round"))));
        let hc = coach_name(b, int(s, "coach"));
        if !hc.is_empty() {
            words.push(format!("coach {hc}"));
        }
    }
    if pf_ + pa_ > 0 {
        figures.push(json!({ "label": "Points for", "value": pf_ }));
        figures.push(json!({ "label": "Points against", "value": pa_ }));
    }
    let mut actions = vec![json!({ "text": format!("All of {name}"), "link": format!("club/{team}") })];
    for y in [year - 1, year + 1] {
        if one(b, "SELECT 1 AS x FROM nfl_seasons WHERE team = ?1 AND year = ?2", &[i(team), i(y)]).is_some() {
            actions.push(json!({ "text": format!("{y}"), "link": format!("club/{team}/{y}") }));
        }
    }
    actions.push(json!({ "text": format!("{year} draft"), "link": format!("draft/{year}") }));
    push(
        &mut p,
        json!({ "kind": "hero", "label": "NFL club season", "title": title, "text": words.join(" · "), "figures": figures, "actions": actions, "tags": [] }),
    );

    if !games.is_empty() {
        let (rows, links) = game_rows(b, &games, team);
        push(
            &mut p,
            json!({ "kind": "table", "title": format!("Schedule and results ({})", games.len()), "note": "Preseason, the season and the playoffs; open a game for its box and snaps", "cols": game_cols(), "rows": rows, "links": links }),
        );
    }

    // The team's games against the league's that season, and real.
    let ours = all(
        b,
        "SELECT x.* FROM log_games__box x JOIN log_games g ON g._row = x._parent WHERE g.stream = 'nfl' AND g.season = ?2 AND g.kind = 'regular' AND ((x._i = 0 AND g.home = ?1) OR (x._i = 1 AND g.away = ?1))",
        &[i(team), i(year)],
    );
    let league = all(
        b,
        "SELECT x.* FROM log_games__box x JOIN log_games g ON g._row = x._parent WHERE g.stream = 'nfl' AND g.season = ?1 AND g.kind = 'regular'",
        &[i(year)],
    );
    if !ours.is_empty() && !league.is_empty() {
        let real = real_box("nfl", "");
        let avg = |rs: &[Row], k: &str| rs.iter().map(|r| box_val(r, k)).sum::<f64>() / rs.len() as f64;
        let rows: Vec<Value> = BOX
            .iter()
            .map(|(label, k, rk)| {
                let (o, lg) = (avg(&ours, k), avg(&league, k));
                let rank = {
                    // The club's place among the 32 by this number a game.
                    let mut per: HashMap<String, (f64, f64)> = HashMap::new();
                    let _ = &mut per;
                    Value::Null
                };
                let _ = rank;
                json!([
                    label,
                    one_dp(o),
                    one_dp(lg),
                    real.get(*rk).and_then(Value::as_f64).map(one_dp).unwrap_or_else(|| "–".into())
                ])
            })
            .collect();
        push(
            &mut p,
            json!({ "kind": "table", "title": "A game on average", "note": format!("{} regular-season games against the league's average team game in {year} and real (nflverse 2021-25)", ours.len()),
                    "cols": [{"name":"s","label":"Stat"},{"name":"o","label":name,"num":true},{"name":"l","label":"League avg","num":true},{"name":"r","label":"Real avg","num":true}],
                    "rows": rows }),
        );
    }

    // Who played: each man's season line.
    let lines = all(
        b,
        "SELECT s.*, p.name, p.player__id AS id, p.player__position AS pos FROM people__pro__seasons s JOIN people p ON p._row = s._parent WHERE s.team = ?1 AND s.year = ?2 ORDER BY s.snaps DESC",
        &[i(team), i(year)],
    );
    if !lines.is_empty() {
        let lead = |k: &str, label: &str| {
            lines
                .iter()
                .filter(|r| num(r, k) > 0.0)
                .max_by(|a, b| num(a, k).total_cmp(&num(b, k)))
                .map(|r| json!({ "text": format!("{label}: {} ({}), {}", text(r, "name"), text(r, "pos"), one_dp(num(r, k))), "link": format!("e/people/{}", int(r, "id")) }))
        };
        let items: Vec<Value> = [
            ("stats__pass_yds", "Passing yards"),
            ("stats__rush_yds", "Rushing yards"),
            ("stats__rec_yds", "Receiving yards"),
            ("stats__tackles", "Tackles"),
            ("stats__sacks", "Sacks"),
            ("stats__ints", "Interceptions"),
            ("stats__fg_made", "Field goals"),
        ]
        .iter()
        .filter_map(|(k, l)| lead(k, l))
        .chain(lines.iter().filter(|r| has(r, "pro_bowl") && num(r, "pro_bowl") != 0.0).map(|r| json!({ "text": format!("Pro Bowl: {} ({})", text(r, "name"), text(r, "pos")), "link": format!("e/people/{}", int(r, "id")) })))
        .chain(lines.iter().filter(|r| has(r, "all_pro") && num(r, "all_pro") != 0.0).map(|r| json!({ "text": format!("All-Pro: {} ({})", text(r, "name"), text(r, "pos")), "link": format!("e/people/{}", int(r, "id")) })))
        .collect();
        push(&mut p, json!({ "kind": "list", "title": "Leaders and honors", "items": items, "dense": true }));
        let rows: Vec<Value> = lines
            .iter()
            .map(|r| {
                json!([
                    text(r, "name"),
                    text(r, "pos"),
                    int(r, "games"),
                    int(r, "starts"),
                    int(r, "snaps"),
                    format!("{}/{}", int(r, "stats__cmp"), int(r, "stats__att")),
                    int(r, "stats__pass_yds"),
                    int(r, "stats__pass_td"),
                    int(r, "stats__int"),
                    int(r, "stats__rush"),
                    int(r, "stats__rush_yds"),
                    int(r, "stats__rec"),
                    int(r, "stats__rec_yds"),
                    int(r, "stats__rush_td") + int(r, "stats__rec_td"),
                    int(r, "stats__tackles") + int(r, "stats__assists"),
                    one_dp(num(r, "stats__sacks")),
                    int(r, "stats__ints"),
                    money(num(r, "pay"))
                ])
            })
            .collect();
        let links: Vec<Value> = lines.iter().map(|r| json!(format!("e/people/{}", int(r, "id")))).collect();
        let n = |x: &str, l: &str| json!({ "name": x, "label": l, "num": true });
        push(
            &mut p,
            json!({ "kind": "table", "title": format!("Season lines ({})", lines.len()), "note": "Everyone on the club that season, most snaps first",
                    "cols": [{"name":"n","label":"Player","minw":170},{"name":"pos","label":"Pos"},n("g","G"),n("gs","GS"),n("sn","Snaps"),n("ca","C/Att"),n("py","Pass yds"),n("ptd","Pass TD"),n("int","Int"),
                             n("ru","Rush"),n("ry","Rush yds"),n("rec","Rec"),n("recy","Rec yds"),n("td","TD"),n("tk","Tackles"),n("sk","Sacks"),n("di","Picks"),n("pay","Pay")],
                    "rows": rows, "links": links }),
        );
    }

    // The books and the crowds that year.
    let names = lines_named(b, "nfl");
    if let Some(y) = one(b, "SELECT l FROM finance__nfl__years WHERE _parent = ?1 AND year = ?2", &[i(team), i(year)]) {
        let l = list(&y, "l");
        let lg = all(b, "SELECT l FROM finance__nfl__years WHERE year = ?1", &[i(year)]);
        let lgl: Vec<Vec<Value>> = lg.iter().map(|r| list(r, "l")).collect();
        let rows: Vec<Value> = names
            .iter()
            .enumerate()
            .map(|(k, (n, inc))| {
                let v = l.get(k).and_then(Value::as_f64).unwrap_or(0.0);
                let a = lgl.iter().map(|x| x.get(k).and_then(Value::as_f64).unwrap_or(0.0)).sum::<f64>()
                    / lgl.len().max(1) as f64;
                json!([n, if *inc { "income" } else { "cost" }, money(v), money(a)])
            })
            .collect();
        let rev = sum_lines(&l, &names, true);
        let cost = sum_lines(&l, &names, false);
        push(
            &mut p,
            json!({ "kind": "table", "title": format!("The books: {} in, {} out, {} net", money(rev), money(cost), money(rev - cost)), "note": "Each line against the league's average club",
                    "cols": [{"name":"l","label":"Line"},{"name":"k","label":""},{"name":"v","label":name,"num":true},{"name":"a","label":"League avg","num":true}],
                    "rows": rows }),
        );
    }
    let gates = all(
        b,
        "SELECT date, opp, attendance, sold_out, blackout, preseason, playoff FROM finance__nfl__gates WHERE _parent = ?1 ORDER BY date",
        &[i(team)],
    );
    let gates: Vec<&Row> = gates
        .iter()
        .filter(|g| {
            crate::pages::day_text(int(g, "date")).starts_with(&year.to_string())
                || crate::pages::day_text(int(g, "date")).starts_with(&format!("{}-0", year + 1))
        })
        .collect();
    if !gates.is_empty() {
        let rows: Vec<Value> = gates
            .iter()
            .map(|g| {
                let kind = if has(g, "playoff") {
                    "playoff"
                } else if has(g, "preseason") {
                    "preseason"
                } else {
                    "regular"
                };
                json!([
                    crate::pages::day_text(int(g, "date")),
                    team_name(b, "nfl", year, int(g, "opp")),
                    kind,
                    fmt(&json!(int(g, "attendance"))),
                    if has(g, "sold_out") {
                        "sold out"
                    } else if has(g, "blackout") {
                        "blacked out"
                    } else {
                        ""
                    }
                ])
            })
            .collect();
        push(
            &mut p,
            json!({ "kind": "table", "title": "Home gates", "cols": [{"name":"d","label":"Date"},{"name":"o","label":"Opponent"},{"name":"k","label":"Game"},{"name":"a","label":"Crowd","num":true},{"name":"s","label":""}], "rows": rows }),
        );
    }
    let picks = all(
        b,
        &format!(
            "{DRAFTED} WHERE json_extract(dr.draft, '$[3]') = ?1 AND json_extract(dr.draft, '$[0]') = ?2 ORDER BY round, pick"
        ),
        &[i(team), i(year)],
    );
    if !picks.is_empty() {
        let items: Vec<Value> = picks
            .iter()
            .map(|r| json!({ "text": format!("{}.{:02} {} ({}, {})", int(r, "round"), int(r, "pick"), text(r, "name"), text(r, "pos"), text(r, "college")), "link": format!("e/{}/{}", text(r, "tbl"), int(r, "id")), "why": plain(&text(r, "status")) }))
            .collect();
        push(
            &mut p,
            json!({ "kind": "list", "title": format!("The {year} draft class"), "items": items, "dense": true }),
        );
    }
    p["verdict"] = json!(match &s {
        Some(s) => format!(
            "{name} went {}-{}{} in {year} and {}.",
            int(s, "wins"),
            int(s, "losses"),
            if int(s, "ties") > 0 { format!("-{}", int(s, "ties")) } else { String::new() },
            round_word(int(s, "round"))
        ),
        None => format!("No season on file for {name} in {year}."),
    });
    let _ = f;
    Ok(p)
}

// ---------------------------------------------------------------- draft class

pub fn draft(b: &mut Book, year: i64, f: &Filters) -> Result<Value, String> {
    let title = format!("The {year} NFL draft");
    let mut p = new_page(&title, &format!("{} · NFL · {title}", world_name(b)));
    let picks =
        all(b, &format!("{DRAFTED} WHERE json_extract(dr.draft, '$[0]') = ?1 ORDER BY round, pick"), &[i(year)]);
    let years: Vec<i64> = all(b, &format!("{DRAFTED} WHERE year IS NOT NULL GROUP BY year ORDER BY year"), &[])
        .iter()
        .map(|r| int(r, "year"))
        .collect();
    let mut actions: Vec<Value> = Vec::new();
    for y in [year - 1, year + 1] {
        if years.contains(&y) {
            actions.push(json!({ "text": format!("{y} draft"), "link": format!("draft/{y}") }));
        }
    }
    actions.push(json!({ "text": "The draft area", "link": "area/N20" }));
    let first = picks
        .first()
        .map(|r| {
            format!(
                "first pick {} ({}, {}) to {}",
                text(r, "name"),
                text(r, "pos"),
                text(r, "college"),
                team_name(b, "nfl", year, int(r, "team"))
            )
        })
        .unwrap_or_default();
    push(
        &mut p,
        json!({ "kind": "hero", "label": "Draft class", "title": title, "text": first,
                "figures": [{"label":"Picks on record","value":picks.len()},{"label":"Rounds","value":picks.iter().map(|r| int(r, "round")).max().unwrap_or(0)},
                            {"label":"Still in the NFL","value":picks.iter().filter(|r| NFL_NOW.contains(&text(r, "status").as_str())).count()}],
                "actions": actions, "tags": years.iter().map(|y| y.to_string()).collect::<Vec<_>>() }),
    );
    if picks.is_empty() {
        p["verdict"] = json!(format!(
            "No pick on record for {year}: the world keeps picks on the players drafted, so years before its people were drafted show nothing."
        ));
        return Ok(p);
    }
    let show_ovr = f.debug || !b.is_debug("people", "x_overall");
    // The board: every pick by round.
    let board: Vec<Value> = picks
        .iter()
        .map(|r| {
            json!({ "round": int(r, "round"), "pick": int(r, "pick"), "club": team_name(b, "nfl", year, int(r, "team")), "name": text(r, "name"), "pos": text(r, "pos"),
                    "college": text(r, "college"), "status": plain(&text(r, "status")),
                    "now": if has(r, "now_team") { team_name(b, "nfl", year, int(r, "now_team")) } else { String::new() },
                    "ovr": if show_ovr && has(r, "ovr") { json!(one_dp(num(r, "ovr"))) } else { Value::Null },
                    "link": format!("e/{}/{}", text(r, "tbl"), int(r, "id")) })
        })
        .collect();
    push(
        &mut p,
        json!({ "kind": "picks", "title": "The board", "note": "Every pick by round, with where he is now", "picks": board }),
    );

    // Positions by round against real.
    let bands = [
        ("round_1", "Round 1", 1, 1),
        ("round_2", "Round 2", 2, 2),
        ("round_3", "Round 3", 3, 3),
        ("rounds_4_7", "Rounds 4-7", 4, 7),
    ];
    let pos_list = ["QB", "RB", "WR", "TE", "OT", "IOL", "EDGE", "DT", "LB", "CB", "S", "K", "P"];
    let mut rows = Vec::new();
    for pos in pos_list {
        let mut r = vec![json!(pos)];
        for (band, _, lo, hi) in bands {
            let n = picks.iter().filter(|x| (lo..=hi).contains(&int(x, "round"))).count().max(1) as f64;
            let w =
                picks.iter().filter(|x| (lo..=hi).contains(&int(x, "round")) && text(x, "pos") == pos).count() as f64;
            let real = one(
                b,
                &format!("SELECT pct_{pos} AS v FROM real_draft_positions_2000_2025 WHERE band = ?1"),
                &[Sql::Text(band.into())],
            )
            .map(|x| num(&x, "v"));
            r.push(json!(format!("{:.0}%", 100.0 * w / n)));
            r.push(json!(real.map(|x| format!("{x:.0}%")).unwrap_or_else(|| "–".into())));
        }
        rows.push(Value::Array(r));
    }
    let mut cols = vec![json!({"name":"pos","label":"Pos"})];
    for (_, l, _, _) in bands {
        cols.push(json!({ "name": l, "label": l, "num": true }));
        cols.push(json!({ "name": format!("{l} real"), "label": "real", "num": true, "tone": "real" }));
    }
    push(
        &mut p,
        json!({ "kind": "table", "title": "Positions by round against real", "note": "Share of the round's picks; real from nflverse drafts 2000-2025", "cols": cols, "rows": rows }),
    );

    // Colleges and conferences, against that year's real draft.
    let mut by_college: HashMap<String, (i64, i64)> = HashMap::new();
    for r in &picks {
        let e = by_college
            .entry(if text(r, "college").is_empty() { "No college".into() } else { text(r, "college") })
            .or_default();
        e.0 += 1;
        if int(r, "round") == 1 {
            e.1 += 1;
        }
    }
    let mut colleges: Vec<(String, (i64, i64))> = by_college.into_iter().collect();
    colleges.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let world_rows: Vec<Value> = colleges.iter().take(20).map(|(c, (n, r1))| json!([c, n, r1])).collect();
    let real = all(
        b,
        "SELECT college, picks, round1 FROM real_nfl_draft_by_college WHERE season = ?1 ORDER BY picks DESC, round1 DESC LIMIT 20",
        &[i(year)],
    );
    let real_rows: Vec<Value> =
        real.iter().map(|r| json!([text(r, "college"), int(r, "picks"), int(r, "round1")])).collect();
    let cc = json!([{"name":"c","label":"College"},{"name":"n","label":"Picks","num":true},{"name":"r1","label":"Round 1","num":true}]);
    push(
        &mut p,
        json!({ "kind": "pair", "blocks": [
            { "kind": "table", "title": "The world's colleges", "cols": cc, "rows": world_rows },
            { "kind": "table", "title": format!("Real {year} draft"), "note": "Pro Football Reference via nflverse", "cols": cc, "rows": real_rows, "empty": "No real draft on file for this year." } ] }),
    );
    let mut conf: HashMap<String, i64> = HashMap::new();
    for r in &picks {
        *conf.entry(if text(r, "conference").is_empty() { "–".into() } else { text(r, "conference") }).or_default() +=
            1;
    }
    let mut conf: Vec<(String, i64)> = conf.into_iter().collect();
    conf.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let items: Vec<Value> = conf.iter().map(|(c, n)| json!({ "text": format!("{c}: {n}") })).collect();
    push(&mut p, json!({ "kind": "list", "title": "By conference (today's)", "items": items, "dense": true }));
    p["verdict"] = json!(format!(
        "{} on record for {year}; {} of them still in the NFL.",
        plural(picks.len() as i64, "pick", "picks"),
        picks.iter().filter(|r| NFL_NOW.contains(&text(r, "status").as_str())).count()
    ));
    Ok(p)
}

// ---------------------------------------------------------------- agent

pub fn agent(b: &mut Book, row: i64, f: &Filters) -> Result<Value, String> {
    let a = one(b, "SELECT * FROM market__agents WHERE _row = ?1", &[i(row)])
        .ok_or_else(|| format!("No agent {row} in this book."))?;
    let name = text(&a, "name");
    let mut p = new_page(&name, &format!("{} · NFL · Agents · {name}", world_name(b)));
    let size = match int(&a, "size") {
        3 => "a giant agency",
        2 => "a mid-size agency",
        _ => "a one-man shop",
    };
    let clients = all(
        b,
        "SELECT player__id AS id, name, player__position AS pos, player__age AS age, pro__team AS team, pro__status AS status, pro__contract__kind AS deal,
                pro__contract__signing_bonus AS bonus, pro__earnings AS earned, pro__draft AS draft, x_overall AS ovr
           FROM people WHERE pro__agent = ?1 ORDER BY pro__earnings DESC",
        &[i(row)],
    );
    let year = one(b, "SELECT MAX(year) AS y FROM nfl_seasons", &[]).map(|r| int(&r, "y")).unwrap_or(0);
    let mut figures = vec![
        json!({ "label": "Clients now", "value": clients.len() }),
        json!({ "label": "Clients (record)", "value": int(&a, "clients") }),
    ];
    let signed = clients.iter().filter(|r| text(r, "status") == "Roster").count();
    figures.push(json!({ "label": "On a 53", "value": signed }));
    figures.push(
        json!({ "label": "Career pay of clients", "value": money(clients.iter().map(|r| num(r, "earned")).sum()) }),
    );
    let mut tags = vec![text(&a, "agency"), size.to_string()];
    if has(&a, "fired") && num(&a, "fired") != 0.0 {
        tags.push(format!("fired by {}", plural(int(&a, "fired"), "client", "clients")));
    }
    push(
        &mut p,
        json!({ "kind": "hero", "label": "Agent", "title": name, "text": format!("{name} of {}, {size}.", text(&a, "agency")), "tags": tags, "figures": figures,
                "actions": [{"text":"Every agent","link":"area/N17"}] }),
    );
    if f.debug || !b.is_debug("market__agents", "skill") {
        push(
            &mut p,
            json!({ "kind": "kv", "title": "How he works", "pairs": [{"k":"Skill (1-20)","v":one_dp(num(&a, "skill"))},{"k":"Honesty (0-1)","v":fmt(&a["honesty"])}] }),
        );
    }
    let show_ovr = f.debug || !b.is_debug("people", "x_overall");
    if !clients.is_empty() {
        let mut cols = vec![
            json!({"name":"n","label":"Client","minw":170}),
            json!({"name":"pos","label":"Pos"}),
            json!({"name":"a","label":"Age","num":true}),
            json!({"name":"c","label":"Club"}),
            json!({"name":"s","label":"Status"}),
            json!({"name":"d","label":"Deal"}),
        ];
        if show_ovr {
            cols.push(json!({"name":"o","label":"Overall","num":true}));
        }
        cols.extend([
            json!({"name":"dr","label":"Drafted"}),
            json!({"name":"b","label":"Signing bonus","num":true}),
            json!({"name":"e","label":"Career pay","num":true}),
        ]);
        let rows: Vec<Value> = clients
            .iter()
            .map(|r| {
                let d = list(r, "draft");
                let drafted = if d.len() >= 3 {
                    format!("{} {}.{:02}", fmt(&d[0]), fmt(&d[1]), d[2].as_i64().unwrap_or(0))
                } else {
                    "undrafted".into()
                };
                let mut v = vec![
                    json!(text(r, "name")),
                    json!(text(r, "pos")),
                    json!(int(r, "age")),
                    json!(if has(r, "team") { team_name(b, "nfl", year, int(r, "team")) } else { String::new() }),
                    json!(plain(&text(r, "status"))),
                    json!(plain(&text(r, "deal"))),
                ];
                if show_ovr {
                    v.push(json!(one_dp(num(r, "ovr"))));
                }
                v.extend([
                    json!(drafted),
                    json!(if has(r, "bonus") { money(num(r, "bonus")) } else { String::new() }),
                    json!(money(num(r, "earned"))),
                ]);
                Value::Array(v)
            })
            .collect();
        let links: Vec<Value> = clients.iter().map(|r| json!(format!("e/people/{}", int(r, "id")))).collect();
        push(
            &mut p,
            json!({ "kind": "table", "title": format!("Clients ({})", clients.len()), "note": "Most career pay first", "cols": cols, "rows": rows, "links": links }),
        );
        let mut pos: HashMap<String, i64> = HashMap::new();
        for r in &clients {
            *pos.entry(text(r, "pos")).or_default() += 1;
        }
        let mut pos: Vec<(String, i64)> = pos.into_iter().collect();
        pos.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        push(
            &mut p,
            json!({ "kind": "list", "title": "Clients by position", "items": pos.iter().map(|(k, n)| json!({ "text": format!("{k}: {n}") })).collect::<Vec<_>>(), "dense": true }),
        );
    }
    let rel = list(&a, "rel");
    if !rel.is_empty() {
        let mut rows: Vec<(f64, Value, Value)> = rel
            .iter()
            .enumerate()
            .map(|(k, v)| {
                let x = v.as_f64().unwrap_or(0.0);
                let word = if x <= -0.25 {
                    "sour"
                } else if x < 0.0 {
                    "cool"
                } else if x > 0.25 {
                    "warm"
                } else if x > 0.0 {
                    "good"
                } else {
                    "neutral"
                };
                (x, json!([team_name(b, "nfl", year, k as i64), format!("{x:+.2}"), word]), json!(format!("club/{k}")))
            })
            .collect();
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        push(
            &mut p,
            json!({ "kind": "table", "title": "Standing with each club", "note": "Below zero after hard talks, holdouts or a client released; sourest first",
                    "cols": [{"name":"c","label":"Club"},{"name":"v","label":"Standing","num":true},{"name":"w","label":""}],
                    "rows": rows.iter().map(|x| x.1.clone()).collect::<Vec<_>>(), "links": rows.iter().map(|x| x.2.clone()).collect::<Vec<_>>() }),
        );
    }
    p["verdict"] = json!(format!(
        "{name} has {} now ({signed} on a 53-man roster).",
        plural(clients.len() as i64, "client", "clients")
    ));
    Ok(p)
}

// ---------------------------------------------------------------- replay

/// The replay tool on the game log (G37): pick a game, then step through
/// its snaps with the names and the whys.
pub fn replay(b: &mut Book, mut p: Value, game: Option<i64>) -> Result<Value, String> {
    let Some(row) = game else {
        let games = all(
            b,
            "SELECT g._row, g.season, g.stream, g.date, g.kind, g.home, g.away, g.home_pts, g.away_pts FROM log_games g
              WHERE EXISTS (SELECT 1 FROM log_plays lp WHERE lp.season = g.season AND lp.stream = g.stream AND lp.date = g.date AND lp.home = g.home AND lp.away = g.away)
              ORDER BY g.date DESC, g._row LIMIT 300",
            &[],
        );
        let rows: Vec<Value> = games
            .iter()
            .map(|g| {
                let (s, y) = (text(g, "stream"), int(g, "season"));
                json!([
                    text(g, "date"),
                    s.clone(),
                    text(g, "kind"),
                    format!(
                        "{} {}, {} {}",
                        team_name(b, &s, y, int(g, "away")),
                        int(g, "away_pts"),
                        team_name(b, &s, y, int(g, "home")),
                        int(g, "home_pts")
                    )
                ])
            })
            .collect();
        let links: Vec<Value> = games.iter().map(|g| json!(format!("tools/replay?game={}", int(g, "_row")))).collect();
        p["verdict"] =
            json!(format!("{} games with every snap logged, latest first. Pick one to step through it.", games.len()));
        push(
            &mut p,
            json!({ "kind": "table", "title": "Games with snaps", "cols": [{"name":"d","label":"Date"},{"name":"s","label":"Level"},{"name":"k","label":"Game"},{"name":"r","label":"Result","minw":320}],
                    "rows": rows, "links": links }),
        );
        return Ok(p);
    };
    let g = one(b, "SELECT * FROM log_games WHERE _row = ?1", &[i(row)])
        .ok_or_else(|| format!("No game {row} in this book."))?;
    let (stream, season) = (text(&g, "stream"), int(&g, "season"));
    let sides = [
        short(&stream, &team_name(b, &stream, season, int(&g, "home"))),
        short(&stream, &team_name(b, &stream, season, int(&g, "away"))),
    ];
    let plays = plays_of(b, &g);
    let mut ids = HashSet::new();
    for pl in &plays {
        if let Some(w) = pl.get("why").and_then(Value::as_str) {
            ids_in(w, &mut ids);
        }
    }
    let who = People::of(b, &ids);
    let cols = [
        "n",
        "quarter",
        "clock",
        "offense",
        "down",
        "togo",
        "yardline_100",
        "kind",
        "call",
        "why",
        "epa",
        "wp_before",
        "wp_after",
        "score",
    ];
    let snaps: Vec<Value> = plays
        .iter()
        .map(|pl| {
            Value::Array(
                cols.iter()
                    .map(|c| match *c {
                        "offense" => json!(sides[pi(pl, "offense").clamp(0, 1) as usize]),
                        "why" => json!(who.words(pl.get("why").and_then(Value::as_str).unwrap_or(""))),
                        "clock" => json!(clock(pi(pl, "quarter"), pf(pl, "clock"))),
                        "score" => json!(
                            pl.get("score")
                                .and_then(Value::as_array)
                                .map(|s| s.iter().map(fmt).collect::<Vec<_>>().join("–"))
                                .unwrap_or_default()
                        ),
                        "togo" | "yardline_100" | "epa" | "wp_before" | "wp_after" => pl
                            .get(*c)
                            .and_then(Value::as_f64)
                            .map(|x| json!((x * 100.0).round() / 100.0))
                            .unwrap_or(Value::Null),
                        c => pl.get(c).cloned().unwrap_or(Value::Null),
                    })
                    .collect(),
            )
        })
        .collect();
    p["title"] = json!(format!("Replay: {} at {}, {season}", sides[1], sides[0]));
    p["verdict"] = json!(format!(
        "{} snaps, logged as they were played. Step through them one by one; the score reads {}–{}.",
        snaps.len(),
        sides[0],
        sides[1]
    ));
    push(
        &mut p,
        json!({ "kind": "links", "items": [{ "text": "The game page", "sub": "Box, lines, drives and the play by play", "link": format!("game/{row}") }] }),
    );
    let labels = [
        "Snap",
        "Q",
        "Clock",
        "Ball",
        "Down",
        "To go",
        "Yards to goal",
        "Play",
        "Call",
        "What happened",
        "EPA",
        "Ball's win chance before",
        "Ball's win chance after",
        "Score",
    ];
    push(
        &mut p,
        json!({ "kind": "replay", "title": "Snap by snap", "cols": cols.iter().zip(labels).map(|(c, l)| json!({"name": c, "label": l})).collect::<Vec<_>>(),
                "snaps": snaps, "spot": "yardline_100" }),
    );
    Ok(p)
}
