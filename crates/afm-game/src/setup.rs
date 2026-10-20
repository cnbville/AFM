//! Starting a career (section 8, topics 1-3 and 10): the world library,
//! the setup's choices, save files and bug packs.

use std::path::{Path, PathBuf};

use afm_engine::calendar::Level as CalLevel;
use afm_engine::tendencies::Style;
use afm_engine::world::World;
use afm_engine::world::coach::GROUP_COUNT;
use afm_engine::world::ethnicity::Ethnicity;
use afm_engine::world::seat::{Job, You};
use serde_json::{Value, json};

use crate::Game;
use crate::views::{TeamRef, date_text, team_colors, team_name};

/// A world or save file on disk.
fn files(dir: &Path, ext: &str) -> Vec<Value> {
    let mut v: Vec<(std::time::SystemTime, Value)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == ext))
        .map(|e| {
            let meta = e.metadata().ok();
            let when = meta.as_ref().and_then(|m| m.modified().ok()).unwrap_or(std::time::UNIX_EPOCH);
            let size = meta.map_or(0, |m| m.len());
            let path = e.path();
            let info = crate::files::read_meta(&path).ok();
            (
                when,
                json!({
                    "path": path.display().to_string(),
                    "name": path.file_stem().map(|s| s.to_string_lossy().to_string()),
                    "mb": (size as f64 / 1e6).round(),
                    "meta": info,
                }),
            )
        })
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v.into_iter().map(|x| x.1).collect()
}

/// The world library and your saves (section 8, topic 1): each library
/// world is a read-only save on the morning after the 2026 Super Bowl.
pub fn library(g: &Game) -> Value {
    json!({
        "home": g.home.display().to_string(),
        "worlds": files(&g.home.join("worlds"), "afmworld"),
        "saves": files(&g.home.join("saves"), "afmsave"),
    })
}

pub fn read(path: &str) -> Result<World, String> {
    crate::files::read_world(Path::new(path)).map_err(|e| format!("Could not open {path}: {e}"))
}

pub fn write(w: &World, path: &Path) -> Result<(), String> {
    crate::files::write_world(w, path).map_err(|e| format!("Could not save {}: {e}", path.display()))
}

pub fn file_safe(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == ' ' { c } else { '_' })
        .collect::<String>()
        .trim()
        .to_string()
}

/// A background (people topic 15): its starting reputation (1-20), the
/// average rating its pool buys, its lean, and whether you played pro
/// football. Proposed, not confirmed: every number here.
struct Background {
    key: &'static str,
    name: &'static str,
    reputation: f64,
    average: f64,
    ex_pro: bool,
    /// Extra points on (teaching, scheme, game management, motivation,
    /// discipline, recruiting, network, relationships) from the average.
    lean: [f64; 8],
}

const BACKGROUNDS: [Background; 11] = [
    Background {
        key: "nfl_star",
        name: "Former NFL star",
        reputation: 13.0,
        average: 9.5,
        ex_pro: true,
        lean: [-1.0, -1.0, -1.0, 3.0, 0.0, 2.0, 5.0, 3.0],
    },
    Background {
        key: "nfl_pro_bowl",
        name: "Former NFL Pro Bowler",
        reputation: 11.0,
        average: 9.5,
        ex_pro: true,
        lean: [-0.5, -0.5, -0.5, 2.0, 0.5, 1.5, 4.0, 2.0],
    },
    Background {
        key: "nfl_starter",
        name: "Former NFL starter",
        reputation: 9.0,
        average: 9.5,
        ex_pro: true,
        lean: [0.0, 0.0, 0.0, 1.5, 0.5, 1.0, 3.0, 1.5],
    },
    Background {
        key: "nfl_journeyman",
        name: "Former NFL journeyman",
        reputation: 7.0,
        average: 9.5,
        ex_pro: true,
        lean: [0.5, 0.0, 0.0, 1.0, 1.0, 0.5, 2.0, 1.0],
    },
    Background {
        key: "college_player",
        name: "Former college player",
        reputation: 5.0,
        average: 9.0,
        ex_pro: false,
        lean: [0.5, 0.0, 0.0, 1.0, 1.0, 1.0, 0.5, 1.0],
    },
    Background {
        key: "never_played",
        name: "Never played past high school",
        reputation: 3.0,
        average: 9.0,
        ex_pro: false,
        lean: [0.0, 2.0, 1.5, 0.0, 0.0, -1.0, -2.0, 0.0],
    },
    Background {
        key: "hs_coach",
        name: "High school coach",
        reputation: 4.0,
        average: 9.5,
        ex_pro: false,
        lean: [1.5, 0.5, 0.5, 1.5, 1.0, 0.0, -1.5, 1.0],
    },
    Background {
        key: "college_assistant",
        name: "College position coach or coordinator",
        reputation: 8.0,
        average: 11.0,
        ex_pro: false,
        lean: [1.0, 1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.5],
    },
    Background {
        key: "nfl_assistant",
        name: "NFL assistant or coordinator",
        reputation: 10.0,
        average: 11.5,
        ex_pro: false,
        lean: [1.0, 1.5, 1.0, 0.0, 0.5, -2.0, 1.0, 0.0],
    },
    Background {
        key: "scout",
        name: "Scout or personnel man",
        reputation: 6.0,
        average: 10.0,
        ex_pro: false,
        lean: [-1.0, 0.0, 0.0, -0.5, 0.0, 2.5, 2.5, 0.5],
    },
    Background {
        key: "analytics",
        name: "Analytics",
        reputation: 5.0,
        average: 10.0,
        ex_pro: false,
        lean: [-1.0, 2.0, 3.0, -1.0, 0.0, -1.0, 0.0, -0.5],
    },
];

/// The 15 ratings a background's pool spreads over (eight teaching groups
/// and seven others).
const RATING_COUNT: usize = GROUP_COUNT + 7;

fn starting_ratings(b: &Background) -> Vec<f64> {
    let mut v = Vec::with_capacity(RATING_COUNT);
    for _ in 0..GROUP_COUNT {
        v.push((b.average + b.lean[0]).clamp(1.0, 20.0));
    }
    for k in 1..8 {
        v.push((b.average + b.lean[k]).clamp(1.0, 20.0));
    }
    v
}

pub fn options(w: &World) -> Value {
    let groups: Vec<&str> = afm_engine::world::coach::Group::ALL.iter().map(|g| g.coach_title()).collect();
    let states: Vec<&str> = w.states.iter().map(|s| s.code.as_str()).collect();
    json!({
        "today": date_text(w.today),
        "backgrounds": BACKGROUNDS.iter().map(|b| {
            let r = starting_ratings(b);
            json!({ "key": b.key, "name": b.name, "reputation": b.reputation, "pool": r.iter().sum::<f64>().round(), "ratings": r })
        }).collect::<Vec<_>>(),
        "rating_names": groups.iter().map(|g| format!("Teaching: {g}")).chain(["Scheme", "Game management", "Motivation", "Discipline", "Recruiting", "Network", "Relationships"].iter().map(|s| s.to_string())).collect::<Vec<_>>(),
        "states": states,
        "style": [
            { "key": "risk", "name": "Risk", "low": "Conservative", "high": "Aggressive" },
            { "key": "freedom", "name": "Structure or freedom", "low": "Do your job", "high": "Freelance" },
            { "key": "physical", "name": "Physical", "low": "Protective", "high": "Physical" },
            { "key": "ball_hunting", "name": "Ball hunting", "low": "Sound", "high": "Ball hunting" },
            { "key": "mould", "name": "Mould or adapt", "low": "Adapt to him", "high": "Mould him" },
        ],
    })
}

/// Every head coaching job at a level, open to pick (section 8, topic 2).
pub fn jobs(w: &World, level: &str) -> Value {
    let item = |t: TeamRef, sub: String, sort: f64| {
        let [c1, c2] = team_colors(w, t);
        (sort, json!({ "ref": t.json(), "name": team_name(w, t), "sub": sub, "colors": [c1, c2] }))
    };
    let mut v: Vec<(f64, Value)> = match level {
        "nfl" => (0..w.nfl.len())
            .filter(|&t| w.clubs.get(t).is_some())
            .map(|t| item(TeamRef::Nfl(t), "NFL".into(), 0.0))
            .collect(),
        "hs" => w
            .high_schools
            .iter()
            .filter(|s| s.prestige > 0.45 || s.real)
            .map(|s| {
                item(
                    TeamRef::HighSchool(s.id),
                    format!("{}, {} · class {}", s.town, w.states[s.state].code, s.classification + 1),
                    -s.prestige,
                )
            })
            .collect(),
        _ => w
            .programs
            .iter()
            .filter(|p| {
                p.base.is_some() && p.division != afm_engine::world::data::Division::JUCO
                    || p.division == afm_engine::world::data::Division::JUCO
            })
            .map(|p| {
                item(
                    TeamRef::College(p.id),
                    format!("{} · {}", p.division.name(), p.conference),
                    p.division as usize as f64 * 10.0 - p.prestige,
                )
            })
            .collect(),
    };
    v.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1["name"].as_str().cmp(&b.1["name"].as_str())));
    json!({ "level": level, "jobs": v.into_iter().map(|x| x.1).collect::<Vec<_>>() })
}

fn f(args: &Value, k: &str, d: f64) -> f64 {
    args.get(k).and_then(Value::as_f64).unwrap_or(d)
}

pub fn start(w: &mut World, args: &Value) -> Result<(), String> {
    if w.career.is_some() {
        return Err("This world already has a career. Open a library world to start another.".into());
    }
    let bg_key = args.get("background").and_then(Value::as_str).unwrap_or("college_assistant");
    let b = BACKGROUNDS.iter().find(|b| b.key == bg_key).ok_or("No such background.")?;
    let base = starting_ratings(b);
    let pool: f64 = base.iter().sum();
    let mut r: Vec<f64> = args
        .get("ratings")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_f64).collect())
        .filter(|v: &Vec<f64>| v.len() == RATING_COUNT)
        .unwrap_or(base);
    for x in r.iter_mut() {
        *x = x.clamp(1.0, 20.0);
    }
    // The pool is the limit: more than it buys is scaled back.
    let spent: f64 = r.iter().sum();
    if spent > pool + 0.01 {
        let k = pool / spent;
        r.iter_mut().for_each(|x| *x = (*x * k).clamp(1.0, 20.0));
    }
    let mut teaching = [0.0; GROUP_COUNT];
    teaching.copy_from_slice(&r[..GROUP_COUNT]);
    let s = args.get("style").cloned().unwrap_or(json!({}));
    let state = args.get("home_state").and_then(Value::as_str).unwrap_or("TX");
    let home_state = w.states.iter().position(|x| x.code == state).unwrap_or(0);
    let ethnicity = match args.get("ethnicity").and_then(Value::as_str).unwrap_or("white") {
        "black" => Ethnicity::Black,
        "hispanic" => Ethnicity::Hispanic,
        "polynesian" => Ethnicity::Polynesian,
        "asian" => Ethnicity::Asian,
        _ => Ethnicity::White,
    };
    let you = You {
        name: args
            .get("name")
            .and_then(Value::as_str)
            .filter(|n| !n.trim().is_empty())
            .unwrap_or("Coach")
            .trim()
            .to_string(),
        age: f(args, "age", 42.0).clamp(25.0, 75.0) as u32,
        ethnicity,
        home_state,
        ex_pro: b.ex_pro,
        reputation: b.reputation,
        teaching,
        scheme: r[GROUP_COUNT],
        game_management: r[GROUP_COUNT + 1],
        motivation: r[GROUP_COUNT + 2],
        discipline: r[GROUP_COUNT + 3],
        recruiting: r[GROUP_COUNT + 4],
        network: r[GROUP_COUNT + 5],
        relationships: r[GROUP_COUNT + 6],
        style: Style {
            risk: f(&s, "risk", 10.0).clamp(1.0, 20.0),
            freedom: f(&s, "freedom", 10.0).clamp(1.0, 20.0),
            physical: f(&s, "physical", 10.0).clamp(1.0, 20.0),
            ball_hunting: f(&s, "ball_hunting", 10.0).clamp(1.0, 20.0),
            mould: f(&s, "mould", 10.0).clamp(1.0, 20.0),
        },
    };
    let t = crate::views::TeamRef::from_args(w, args.get("job").unwrap_or(&Value::Null))?;
    let job = match t {
        TeamRef::Nfl(i) => Job::Nfl(i),
        TeamRef::College(i) => Job::College(i),
        TeamRef::HighSchool(i) => Job::HighSchool(i),
    };
    let seed = args.get("seed").and_then(Value::as_u64).map(|s| s as u32).unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(7, |d| d.subsec_nanos() ^ d.as_secs() as u32)
    });
    w.start_career(&you, job, seed);
    let _ = CalLevel::Nfl;
    Ok(())
}

/// "Report a bug" (section 8, topic 10): the save as it is now, your note
/// and the screen you were on, in a folder you can drop into a thread.
pub fn bug_pack(g: &Game, args: &Value) -> Result<Value, String> {
    let w = g.world.as_ref().ok_or("No world is loaded.")?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let dir: PathBuf = g.home.join("bugs").join(format!("bug-{stamp}"));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let note = json!({
        "note": args.get("note").and_then(Value::as_str).unwrap_or(""),
        "screen": args.get("screen").cloned().unwrap_or(Value::Null),
        "date": date_text(w.today),
        "seed": w.options.seed,
        "career_seed": w.career.as_ref().map(|c| c.seed),
    });
    std::fs::write(dir.join("note.json"), serde_json::to_string_pretty(&note).unwrap_or_default())
        .map_err(|e| e.to_string())?;
    write(w, &dir.join("world.afmsave"))?;
    Ok(json!({ "folder": dir.display().to_string() }))
}
