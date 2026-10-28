//! The two engine tools: replaying a logged game snap by snap from the book,
//! and playing any two programs now on the engine, with the play by play.

use afm_engine::game::{Conditions, Venue, quick_game_in};
use afm_engine::world::{World, division_level};
use serde_json::{Value, json};

use crate::book::{Book, Filters, cell_text, json_sql, label, q};
use crate::pages::{fmt, fmt_id, push};

const ORDER_COLS: [&str; 6] = ["n", "seq", "play", "play_n", "idx", "snap"];
const SPOT_COLS: [&str; 5] = ["yardline_100", "yardline", "yard_line", "ball_on", "spot"];

/// Replay a logged game: pick a game, then step through its snaps.
pub fn replay_page(b: &mut Book, mut p: Value, f: &Filters, args: &Value) -> Result<Value, String> {
    let (Some(games), Some(plays)) = (b.find("games").map(|t| t.name.clone()), b.find("plays").map(|t| t.name.clone()))
    else {
        p["verdict"] = json!(
            "This book has no logged games: it was built from a world file without its world log. A world built with the log (lane R1) gives every game snap by snap."
        );
        return Ok(p);
    };
    let Some(game) = args.get("game").filter(|v| !v.is_null()) else {
        p["verdict"] = json!("Pick a game: search a team, or page through the games below.");
        let first = b.rows(&games, f, None, None, false, 0, 50)?;
        push(
            &mut p,
            json!({ "kind": "rows", "title": "Games", "table": games, "first": first, "open": "tools/replay?game=" }),
        );
        return Ok(p);
    };
    let t = b.table(&plays).expect("found");
    let gc = t.pick(&["game_id", "game"]).ok_or("The plays table has no game id.")?;
    let order = t.pick(&ORDER_COLS).map(q).unwrap_or_else(|| "rowid".into());
    let spot = t.pick(&SPOT_COLS).map(str::to_string);
    let sql = format!("SELECT * FROM {} WHERE {} = ?1 ORDER BY {order}", q(&plays), q(gc));
    let rows = b.select_maps(&sql, &[json_sql(game)])?;
    let title = b.name_of(&games, game).unwrap_or_else(|| format!("Game {}", fmt_id(game)));
    p["title"] = json!(format!("Replay: {title}"));
    p["verdict"] = json!(format!("{} snaps, logged as they were played. Step through them one by one.", rows.len()));
    let cols: Vec<String> = b
        .table(&plays)
        .expect("found")
        .cols
        .iter()
        .filter(|c| *c != gc && (f.debug || !b.is_debug(&plays, c)))
        .cloned()
        .collect();
    let snaps: Vec<Value> = rows
        .iter()
        .map(|r| Value::Array(cols.iter().map(|c| r.get(c).cloned().unwrap_or(Value::Null)).collect()))
        .collect();
    push(
        &mut p,
        json!({ "kind": "replay", "title": "Snap by snap", "cols": cols.iter().map(|c| json!({"name": c, "label": label(c)})).collect::<Vec<_>>(), "snaps": snaps, "spot": spot }),
    );
    Ok(p)
}

/// The picker for playing two teams now.
pub fn play_page(b: &mut Book, mut p: Value, args: &Value) -> Result<Value, String> {
    let has_programs = b.find("programs").is_some();
    p["verdict"] = json!(if has_programs {
        "Pick two programs; the engine plays them now from the world file, as the world stands at its last save. The first game loads the world (about a minute and 4 GB on the M1)."
    } else {
        "This book has no programs table to pick from yet; the engine tool needs it."
    });
    p["play"] =
        json!({ "table": b.find("programs").map(|t| t.name.clone()), "home": args["home"], "away": args["away"] });
    Ok(p)
}

/// Plays two programs (their world indexes) on the engine now.
pub fn play(w: &World, home: usize, away: usize, seed: u64) -> Result<Value, String> {
    let n = w.programs.len();
    if home >= n || away >= n || home == away {
        return Err("Pick two different programs of this world.".into());
    }
    let (h, a) = (
        w.debug_game_team(home).ok_or("The home team has no roster.")?,
        w.debug_game_team(away).ok_or("The road team has no roster.")?,
    );
    let level = division_level(w.programs[home].division);
    let venue = Venue { cond: Conditions { crowd: 0.65, ..Conditions::default() }, ..Venue::default() };
    let t = std::time::Instant::now();
    let r = quick_game_in(&h, &a, level, seed, Default::default(), venue);
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    let names = [w.program_name(home).to_string(), w.program_name(away).to_string()];
    let s = &r.stats;
    let tot = |k: usize| {
        let x = &s.team[k];
        json!([
            names[k],
            x.points,
            x.plays,
            fmt(&json!(x.yards)),
            fmt(&json!(x.pass_yards)),
            fmt(&json!(x.rush_yards)),
            x.first_downs,
            format!("{}/{}", x.third_conv, x.third_att),
            x.turnovers,
            x.sacks,
            x.penalties,
            fmt(&json!(x.epa))
        ])
    };
    let plays: Vec<Value> = s
        .plays
        .iter()
        .map(|p| {
            json!([
                p.n,
                format!("Q{} {}:{:02}", p.quarter, (p.clock / 60.0) as u32, (p.clock % 60.0) as u32),
                format!("{}-{}", p.score[0], p.score[1]),
                names[p.offense.min(1)],
                format!("{}&{:.0}", p.down, p.togo),
                format!("{:.0}", p.yardline_100),
                format!("{:?}", p.kind),
                p.call,
                p.defense_call,
                format!("{:+.1}", p.yards),
                format!("{:+.2}", p.epa),
                p.why
            ])
        })
        .collect();
    let mut page = crate::pages::new_page(
        &format!("{} {} - {} {}", names[0], s.score[0], s.score[1], names[1]),
        "Tools · Play two teams now",
    );
    page["verdict"] = json!(format!(
        "Played now on the engine in {ms:.0} ms (seed {seed}){}: {} {}, {} {}.",
        if s.overtime { ", in overtime" } else { "" },
        names[0],
        s.score[0],
        names[1],
        s.score[1]
    ));
    push(
        &mut page,
        json!({ "kind": "table", "title": "Box score",
        "cols": [{"name": "t", "label": "Team"}, {"name": "p", "label": "Points", "num": true}, {"name": "pl", "label": "Plays", "num": true}, {"name": "y", "label": "Yards", "num": true},
                 {"name": "py", "label": "Pass", "num": true}, {"name": "ry", "label": "Rush", "num": true}, {"name": "fd", "label": "First downs", "num": true},
                 {"name": "3d", "label": "Third downs", "num": true}, {"name": "to", "label": "Turnovers", "num": true}, {"name": "sk", "label": "Sacks", "num": true},
                 {"name": "pen", "label": "Penalties", "num": true}, {"name": "epa", "label": "EPA", "num": true}],
        "rows": [tot(0), tot(1)] }),
    );
    push(
        &mut page,
        json!({ "kind": "table", "title": format!("Play by play ({} snaps), each with its why", plays.len()),
        "cols": [{"name": "n", "label": "#", "num": true}, {"name": "c", "label": "Clock"}, {"name": "s", "label": "Score"}, {"name": "o", "label": "Ball"},
                 {"name": "d", "label": "Down"}, {"name": "y", "label": "Yards to go in", "num": true}, {"name": "k", "label": "Kind"}, {"name": "call", "label": "Call"},
                 {"name": "def", "label": "Defense"}, {"name": "g", "label": "Gain", "num": true}, {"name": "e", "label": "EPA", "num": true}, {"name": "w", "label": "Why"}],
        "rows": plays }),
    );
    Ok(page)
}

/// The world file a book was built from: named in its meta, or beside it
/// with the same name, or in the game's library.
pub fn world_path(b: &Book, game_worlds: &std::path::Path) -> Option<std::path::PathBuf> {
    let m = crate::pages::meta(b);
    for k in ["world_path", "world_file", "world"] {
        if let Some(p) = m.get(k).and_then(cell_text).map(std::path::PathBuf::from).filter(|p| p.exists()) {
            return Some(p);
        }
    }
    let stem = b.path.file_stem()?.to_string_lossy().into_owned();
    let stem = stem.strip_suffix(".afmworld").unwrap_or(&stem).to_string();
    let beside = b.path.with_file_name(format!("{stem}.afmworld"));
    if beside.exists() {
        return Some(beside);
    }
    let lib = game_worlds.join(format!("{stem}.afmworld"));
    lib.exists().then_some(lib)
}
