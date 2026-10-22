//! Starting a career (section 8, topics 1-3 and 10): the world library,
//! the setup's choices, save files and bug packs.

use std::path::{Path, PathBuf};

use afm_engine::tendencies::Style;
use afm_engine::world::World;
use afm_engine::world::coach::{GROUP_COUNT, Yours};
use afm_engine::world::ethnicity::Ethnicity;
use afm_engine::world::seat::{Job, Profile, You};
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
            let card = info.as_ref().map_or(Value::Null, |m| crate::cards::read(&path, m));
            let secs = when.duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
            (
                when,
                json!({
                    "path": path.display().to_string(),
                    "name": path.file_stem().map(|s| s.to_string_lossy().to_string()),
                    "mb": (size as f64 / 1e6).round(),
                    "meta": info,
                    "card": card,
                    "modified": secs,
                }),
            )
        })
        .collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.0));
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
    let w = crate::files::read_world(Path::new(path)).map_err(|e| format!("Could not open {path}: {e}"))?;
    // A library world written before cards gets its card beside it, so the
    // library shows its story from now on without opening it.
    let p = Path::new(path);
    if path.ends_with(".afmworld")
        && w.career.is_none()
        && let Ok(meta) = crate::files::read_meta(p)
        && crate::cards::read(p, &meta).is_null()
    {
        crate::cards::write_beside(p, &w, &meta);
    }
    Ok(w)
}

/// A save in your saves folder, by its path: nothing outside it is touched.
fn save_in(g: &Game, path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path);
    let dir = g.home.join("saves").canonicalize().map_err(|e| e.to_string())?;
    let full = p.canonicalize().map_err(|_| "That save isn't there any more.".to_string())?;
    if full.parent() != Some(dir.as_path()) || full.extension().is_none_or(|x| x != "afmsave") {
        return Err("Only saves in your saves folder can be renamed or deleted.".into());
    }
    Ok(full)
}

/// Renames a save (the load page).
pub fn rename_save(g: &Game, args: &Value) -> Result<Value, String> {
    let from = save_in(g, args.get("path").and_then(Value::as_str).ok_or("Missing path.")?)?;
    let name = file_safe(args.get("name").and_then(Value::as_str).unwrap_or(""));
    if name.is_empty() {
        return Err("Give the save a name.".into());
    }
    let to = from.with_file_name(format!("{name}.afmsave"));
    if to.exists() && !same_file(&to, &from) {
        return Err(format!("There is already a save called {name}."));
    }
    std::fs::rename(&from, &to).map_err(|e| format!("Could not rename the save: {e}"))?;
    Ok(json!({ "path": to.display().to_string() }))
}

/// Whether two paths are one file: on a Mac's disk a rename that only
/// changes letter case finds the old file under the new name.
fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (a.metadata(), b.metadata()) {
            (Ok(x), Ok(y)) => x.dev() == y.dev() && x.ino() == y.ino(),
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy())
    }
}

/// Deletes a save (the load page asks first).
pub fn delete_save(g: &Game, args: &Value) -> Result<Value, String> {
    let p = save_in(g, args.get("path").and_then(Value::as_str).ok_or("Missing path.")?)?;
    std::fs::remove_file(&p).map_err(|e| format!("Could not delete the save: {e}"))?;
    Ok(json!({ "deleted": true }))
}

/// Shows a file in Finder (or the folder in the system's file browser):
/// only the game folder and what is in it, else the saves folder.
pub fn reveal(g: &Game, args: &Value) -> Result<Value, String> {
    let home = g.home.canonicalize().map_err(|e| e.to_string())?;
    let p = args
        .get("path")
        .and_then(Value::as_str)
        .and_then(|p| PathBuf::from(p).canonicalize().ok())
        .filter(|p| p.starts_with(&home))
        .unwrap_or_else(|| home.join("saves"));
    let r = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg("-R").arg(&p).spawn()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("explorer").arg(format!("/select,{}", p.display())).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(if p.is_dir() { &p } else { p.parent().unwrap_or(&p) }).spawn()
    };
    let mut child = r.map_err(|e| format!("Could not open the folder: {e}"))?;
    std::thread::spawn(move || child.wait());
    Ok(json!({ "shown": p.display().to_string() }))
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
    /// Extra points from the average on (teaching, scheme, game management,
    /// motivation, discipline, recruiting, network, relationships, play
    /// calling, in-game adaptation, evaluation, negotiating).
    lean: [f64; 12],
}

const BACKGROUNDS: [Background; 11] = [
    Background {
        key: "nfl_star",
        name: "Former NFL star",
        reputation: 13.0,
        average: 9.5,
        ex_pro: true,
        lean: [-1.0, -1.0, -1.0, 3.0, 0.0, 2.0, 5.0, 3.0, -1.0, 0.0, 0.5, 1.0],
    },
    Background {
        key: "nfl_pro_bowl",
        name: "Former NFL Pro Bowler",
        reputation: 11.0,
        average: 9.5,
        ex_pro: true,
        lean: [-0.5, -0.5, -0.5, 2.0, 0.5, 1.5, 4.0, 2.0, -0.5, 0.0, 0.5, 1.0],
    },
    Background {
        key: "nfl_starter",
        name: "Former NFL starter",
        reputation: 9.0,
        average: 9.5,
        ex_pro: true,
        lean: [0.0, 0.0, 0.0, 1.5, 0.5, 1.0, 3.0, 1.5, 0.0, 0.5, 0.5, 0.5],
    },
    Background {
        key: "nfl_journeyman",
        name: "Former NFL journeyman",
        reputation: 7.0,
        average: 9.5,
        ex_pro: true,
        lean: [0.5, 0.0, 0.0, 1.0, 1.0, 0.5, 2.0, 1.0, 0.0, 0.5, 0.5, 0.5],
    },
    Background {
        key: "college_player",
        name: "Former college player",
        reputation: 5.0,
        average: 9.0,
        ex_pro: false,
        lean: [0.5, 0.0, 0.0, 1.0, 1.0, 1.0, 0.5, 1.0, 0.0, 0.0, 0.0, 0.0],
    },
    Background {
        key: "never_played",
        name: "Never played past high school",
        reputation: 3.0,
        average: 9.0,
        ex_pro: false,
        lean: [0.0, 2.0, 1.5, 0.0, 0.0, -1.0, -2.0, 0.0, 2.0, 1.0, 0.0, -1.0],
    },
    Background {
        key: "hs_coach",
        name: "High school coach",
        reputation: 4.0,
        average: 9.5,
        ex_pro: false,
        lean: [1.5, 0.5, 0.5, 1.5, 1.0, 0.0, -1.5, 1.0, 1.0, 0.5, 0.0, -1.5],
    },
    Background {
        key: "college_assistant",
        name: "College position coach or coordinator",
        reputation: 8.0,
        average: 11.0,
        ex_pro: false,
        lean: [1.0, 1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 1.0, -1.0],
    },
    Background {
        key: "nfl_assistant",
        name: "NFL assistant or coordinator",
        reputation: 10.0,
        average: 11.5,
        ex_pro: false,
        lean: [1.0, 1.5, 1.0, 0.0, 0.5, -2.0, 1.0, 0.0, 1.5, 1.5, 0.5, 0.0],
    },
    Background {
        key: "scout",
        name: "Scout or personnel man",
        reputation: 6.0,
        average: 10.0,
        ex_pro: false,
        lean: [-1.0, 0.0, 0.0, -0.5, 0.0, 2.5, 2.5, 0.5, -1.5, -0.5, 4.0, 1.5],
    },
    Background {
        key: "analytics",
        name: "Analytics",
        reputation: 5.0,
        average: 10.0,
        ex_pro: false,
        lean: [-1.0, 2.0, 3.0, -1.0, 0.0, -1.0, 0.0, -0.5, 0.5, 1.5, 1.5, 1.0],
    },
];

/// The 19 ratings a background's pool spreads over, in the order the setup
/// shows them: the eight teaching groups, then game day, people and
/// building the roster. Each is (key, name, group, its place in `lean`).
const RATINGS: [(&str, &str, &str, usize); 19] = [
    ("qb", "Quarterbacks", "Teaching", 0),
    ("rb", "Running backs", "Teaching", 0),
    ("wr", "Receivers", "Teaching", 0),
    ("ol", "Offensive line", "Teaching", 0),
    ("dl", "Defensive line", "Teaching", 0),
    ("lb", "Linebackers", "Teaching", 0),
    ("db", "Defensive backs", "Teaching", 0),
    ("st", "Special teams", "Teaching", 0),
    ("scheme", "Scheme", "Game day", 1),
    ("game_management", "Game management", "Game day", 2),
    ("play_calling", "Play calling", "Game day", 8),
    ("adaptation", "In-game adaptation", "Game day", 9),
    ("motivation", "Motivation", "People", 3),
    ("discipline", "Discipline", "People", 4),
    ("relationships", "Relationships", "People", 7),
    ("recruiting", "Recruiting", "Building the roster", 5),
    ("evaluation", "Evaluation", "Building the roster", 10),
    ("network", "Network", "Building the roster", 6),
    ("negotiating", "Negotiating", "Building the roster", 11),
];
const RATING_COUNT: usize = RATINGS.len();

/// Whole points, as the setup shows and spends them.
fn starting_ratings(b: &Background) -> Vec<f64> {
    RATINGS.iter().map(|r| (b.average + b.lean[r.3]).round().clamp(1.0, 20.0)).collect()
}

/// What a background leans toward most, in words ("Network, motivation").
fn strongest(b: &Background) -> String {
    const NAMES: [&str; 12] = [
        "teaching",
        "scheme",
        "game management",
        "motivation",
        "discipline",
        "recruiting",
        "network",
        "relationships",
        "play calling",
        "in-game adaptation",
        "evaluation",
        "negotiating",
    ];
    let mut k: Vec<usize> = (0..12).collect();
    k.sort_by(|&x, &y| b.lean[y].total_cmp(&b.lean[x]).then(x.cmp(&y)));
    let s = format!("{}, {}", NAMES[k[0]], NAMES[k[1]]);
    s[..1].to_uppercase() + &s[1..]
}

pub fn options(w: &World) -> Value {
    let mut programs: Vec<&str> = w.programs.iter().map(|p| w.program_name(p.id)).collect();
    programs.sort_unstable();
    programs.dedup();
    json!({
        "today": date_text(w.today),
        "year": w.today.year(),
        "backgrounds": BACKGROUNDS.iter().map(|b| {
            let r = starting_ratings(b);
            json!({
                "key": b.key, "name": b.name, "reputation": b.reputation, "ex_pro": b.ex_pro,
                "pool": r.iter().sum::<f64>(), "ratings": r, "strongest": strongest(b),
            })
        }).collect::<Vec<_>>(),
        "ratings": RATINGS.iter().map(|r| json!({ "key": r.0, "name": r.1, "group": r.2 })).collect::<Vec<_>>(),
        "states": w.states.iter().map(|s| json!({ "code": s.code, "name": crate::views::state_name(&s.code) })).collect::<Vec<_>>(),
        "programs": programs,
        "style": [
            { "key": "risk", "name": "Risk", "low": "Conservative", "high": "Aggressive" },
            { "key": "freedom", "name": "Structure or freedom", "low": "Do your job", "high": "Freelance" },
            { "key": "physical", "name": "Physical", "low": "Protective", "high": "Physical" },
            { "key": "ball_hunting", "name": "Ball hunting", "low": "Sound", "high": "Ball hunting" },
            { "key": "mould", "name": "Mould or adapt", "low": "Adapt to him", "high": "Mould him" },
        ],
    })
}

/// A state's towns and their high schools, for your hometown and school.
pub fn towns(w: &World, state: &str) -> Value {
    let Some(st) = w.states.iter().find(|s| s.code == state) else { return json!({ "towns": [] }) };
    let mut by: std::collections::BTreeMap<&str, Vec<&str>> = std::collections::BTreeMap::new();
    for &k in &st.schools {
        let h = &w.high_schools[k];
        by.entry(h.town.as_str()).or_default().push(w.school_name(k));
    }
    json!({ "towns": by.into_iter().map(|(t, s)| json!({ "town": t, "schools": s })).collect::<Vec<_>>() })
}

/// What the boss expects from a new head coach, in words, from how big the
/// job is at its level and how last season went (proposed, not confirmed).
pub(crate) fn college_expectation(rel: f64) -> &'static str {
    match rel {
        r if r > 0.85 => "Win the conference and reach the playoff. Anything less is a down year.",
        r if r > 0.65 => "Contend for the conference title and reach the playoff within two years.",
        r if r > 0.45 => "A winning season and a bowl game.",
        r if r > 0.25 => "Show progress: a bowl game within three years.",
        _ => "Compete, recruit and build. The board knows it's a long job.",
    }
}

pub(crate) fn nfl_expectation(wins: u32, round: u32, patience: f64) -> String {
    let base = match (wins, round) {
        (_, r) if r >= 3 => "Win the Super Bowl: the roster is built for now.",
        (w, _) if w >= 11 => "Win in the playoffs: the roster is built to contend.",
        (w, _) if w >= 9 => "Make the playoffs.",
        (w, _) if w >= 6 => "Get back over .500 and push for the playoffs.",
        _ => "Rebuild through the draft.",
    };
    let owner = match patience {
        p if p < 0.3 => " The owner is impatient.",
        p if p > 0.7 => " The owner gives a new coach time.",
        _ => "",
    };
    format!("{base}{owner}")
}

pub(crate) fn hs_expectation(prestige: f64, titles: u32) -> &'static str {
    match prestige {
        p if p > 0.8 || titles >= 5 => "Win the state title. The town expects a deep playoff run every year.",
        p if p > 0.6 => "Make the playoffs and win a round or two.",
        p if p > 0.4 => "A winning season and a playoff spot.",
        _ => "Compete in your district and keep kids coming out for the team.",
    }
}

/// Every head coaching job at a level, open to pick (section 8, topic 2),
/// with what its file shows: last season, the size of the job, the coach
/// you would replace and your boss.
pub fn jobs(w: &World, level: &str) -> Value {
    use crate::views::{job_record, prestige_words, season_record, team_line};
    let d = &w.dates;
    let coach = |c: Option<usize>| {
        c.and_then(|c| w.coaches.get(c).map(|co| (c, co))).map(|(c, co)| {
            let years = co.job.map_or(0, |j| (w.year - j.since).max(0));
            json!({ "name": co.name, "age": co.age, "record": job_record(w, c), "seasons": years, "id": c })
        })
    };
    let base = |t: TeamRef| {
        let [c1, c2] = team_colors(w, t);
        let (record, note) = season_record(w, t);
        json!({ "ref": t.json(), "name": team_name(w, t), "colors": [c1, c2], "line": team_line(w, t),
                "record": record, "record_note": note })
    };
    let mut v: Vec<(f64, Value)> = match level {
        "nfl" => {
            let cap = afm_engine::world::pro::money::price_cap(w.year);
            let calendar = format!(
                "The combine opens {}; free agency {}; the draft {}.",
                date_text(d.combine_start),
                date_text(d.league_year.date),
                date_text(d.draft[0].date)
            );
            (0..w.nfl.len())
                .filter(|&t| w.clubs.get(t).is_some())
                .map(|t| {
                    let club = &w.clubs[t];
                    let last = w.nfl_seasons.iter().rev().find(|s| s.team == t);
                    let titles: Vec<i32> = w.nfl_champions.iter().filter(|x| x.1 == t).map(|x| x.0).collect();
                    let room = cap + club.carryover - afm_engine::world::pro::payroll(w, t, w.year);
                    let mut j = base(TeamRef::Nfl(t));
                    j["conference"] = json!(crate::views::nfl_division(w, t).map(|n| &n[..3]));
                    j["division"] = json!(crate::views::nfl_division(w, t));
                    j["playoffs"] = json!(last.map(|s| crate::views::NFL_ROUNDS[s.round.min(5) as usize]));
                    j["titles"] = json!(titles.len());
                    j["last_title"] = json!(titles.iter().max());
                    j["cap_room"] = json!(room.round());
                    j["coach"] = json!(coach(w.nfl[t].head_coach));
                    j["gm"] = json!({ "name": club.office.gm.name, "since": club.office.gm.since });
                    j["expectation"] = json!(nfl_expectation(
                        last.map_or(8, |s| s.wins),
                        last.map_or(0, |s| s.round),
                        club.office.owner.patience
                    ));
                    j["calendar"] = json!(calendar);
                    j["sub"] = json!(crate::views::nfl_division(w, t).unwrap_or("NFL"));
                    let wins = last.map_or(0.0, |s| s.wins as f64);
                    (-(titles.len() as f64) * 0.01 - wins, j)
                })
                .collect()
        }
        "hs" => {
            let calendar = format!(
                "Offseason workouts, and spring practice where the state allows it. School starts {}.",
                date_text(d.hs_school_start)
            );
            w.high_schools
                .iter()
                .filter(|s| s.prestige > 0.45 || s.real)
                .map(|s| {
                    let mut j = base(TeamRef::HighSchool(s.id));
                    let code = &w.states[s.state].code;
                    j["state"] = json!(code);
                    j["class"] = json!(crate::views::hs_class(w, s.id));
                    j["town"] = json!(s.town);
                    j["prestige"] = json!((s.prestige * 100.0).round() / 100.0);
                    j["prestige_word"] = json!(prestige_words(s.prestige));
                    j["enrollment"] = json!(s.enrollment);
                    j["titles"] = json!(s.state_titles);
                    j["private"] = json!(s.private);
                    j["coach"] = json!(coach(s.coach));
                    j["expectation"] = json!(hs_expectation(s.prestige, s.state_titles));
                    j["calendar"] = json!(calendar);
                    j["sub"] = json!(format!("{}, {} · {}", s.town, code, crate::views::hs_class(w, s.id)));
                    (-s.prestige, j)
                })
                .collect()
        }
        _ => {
            let calendar = if w.today > d.signing_day {
                format!("National signing day is done. Spring practice opens {}.", date_text(d.spring_practice.0))
            } else {
                format!("National signing day is {}.", date_text(d.signing_day))
            };
            w.programs
                .iter()
                // A program that plays this year, with enough players to dress
                // a team: one that can't is cancelled every game day.
                .filter(|p| {
                    (p.base.is_some() && p.division != afm_engine::world::data::Division::JUCO
                        || p.division == afm_engine::world::data::Division::JUCO)
                        && p.plays(w.year)
                        && p.roster.len() >= 22
                })
                .map(|p| {
                    let mut j = base(TeamRef::College(p.id));
                    let mut grades: Vec<(&str, f64)> = afm_engine::world::program::GradeCat::ALL
                        .iter()
                        .map(|g| (g.name(), p.grades[*g as usize].round()))
                        .collect();
                    grades.sort_by(|a, b| b.1.total_cmp(&a.1));
                    j["division"] = json!(p.division.name());
                    j["conference"] = json!(p.conference);
                    j["city"] = json!(format!("{}, {}", p.city, w.states[p.state].code));
                    j["state"] = json!(w.states[p.state].code);
                    j["prestige"] = json!((p.prestige_rel * 100.0).round() / 100.0);
                    j["prestige_word"] = json!(prestige_words(p.prestige_rel));
                    j["stadium"] = json!(p.stadium);
                    j["nil"] = json!(p.collective.budget.round());
                    j["coach"] = json!(coach(p.head_coach));
                    j["ad"] = json!(p.ad.as_ref().map(|a| json!({ "name": a.name, "since": a.since })));
                    j["expectation"] = json!(college_expectation(p.prestige_rel));
                    j["grades"] = json!(grades);
                    j["titles"] = json!(p.titles);
                    j["calendar"] = json!(calendar);
                    j["sub"] = json!(format!(
                        "{} · {} · {}, {}",
                        p.division.name(),
                        p.conference,
                        p.city,
                        w.states[p.state].code
                    ));
                    (p.division as usize as f64 * 10.0 - p.prestige, j)
                })
                .collect()
        }
    };
    v.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1["name"].as_str().cmp(&b.1["name"].as_str())));
    json!({ "level": level, "jobs": v.into_iter().map(|x| x.1).collect::<Vec<_>>() })
}

/// Three coaches in the world on a road like yours (people topic 15, the
/// ratings step's side panel): college and NFL head coaches and
/// coordinators who played in the NFL if you did, who came up coaching the
/// group you teach best, nearest your starting reputation. Public facts
/// only; a high school coach's road shows high school head coaches.
pub fn similar(w: &World, args: &Value) -> Value {
    use afm_engine::world::coach::{Employer, Group, Role};
    let bg_key = args.get("background").and_then(Value::as_str).unwrap_or("college_assistant");
    let Some(b) = BACKGROUNDS.iter().find(|b| b.key == bg_key) else { return json!({ "coaches": [] }) };
    let r: Vec<f64> = args
        .get("ratings")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_f64).collect())
        .filter(|v: &Vec<f64>| v.len() == RATING_COUNT)
        .unwrap_or_else(|| starting_ratings(b));
    let top = (0..GROUP_COUNT).max_by(|&x, &y| r[x].total_cmp(&r[y]).then(y.cmp(&x))).unwrap_or(0);
    let hs = b.key == "hs_coach";
    let mut v: Vec<(f64, usize)> = w
        .coaches
        .iter()
        .filter(|c| !c.retired && c.ex_pro == b.ex_pro)
        .filter_map(|c| {
            let j = c.job?;
            let fits = match j.employer {
                Employer::HighSchool(_) => hs && j.role == Role::HeadCoach,
                _ => !hs && !matches!(j.role, Role::PositionCoach(_)),
            };
            fits.then(|| {
                let off = if c.came_up as usize == top { 0.0 } else { 3.0 };
                ((c.reputation - b.reputation).abs() + off, c.id)
            })
        })
        .collect();
    v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let coaches: Vec<Value> = v
        .iter()
        .take(3)
        .map(|&(_, c)| {
            let co = &w.coaches[c];
            let j = co.job.expect("filtered on a job");
            let at = match j.employer {
                Employer::HighSchool(s) => w.school_name(s).to_string(),
                Employer::College(p) => w.program_name(p).to_string(),
                Employer::Nfl(t) => w.nfl_team_name(t),
            };
            let role = j.role.name();
            json!({
                "id": c, "name": co.name, "age": co.age,
                "job": format!("{}{}, {at}", role[..1].to_uppercase(), &role[1..]),
                "came_up": format!("came up coaching {}", Group::ALL[co.came_up as usize].name()),
                "reputation": crate::views::reputation_words(co.reputation),
            })
        })
        .collect();
    json!({ "coaches": coaches })
}

fn f(args: &Value, k: &str, d: f64) -> f64 {
    args.get(k).and_then(Value::as_f64).unwrap_or(d)
}

fn text(args: &Value, k: &str, max: usize) -> String {
    args.get(k).and_then(Value::as_str).unwrap_or("").trim().chars().take(max).collect()
}

/// Your profile from the setup's You step.
/// A photo the page made: a small base64 JPEG and nothing else, so a save
/// passed around can't carry anything into the page.
pub(crate) fn photo_ok(s: &str) -> bool {
    s.len() < 400_000
        && s.strip_prefix("data:image/jpeg;base64,")
            .is_some_and(|b| b.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'/' | b'=')))
}

fn profile(args: &Value) -> Profile {
    let p = args.get("profile").cloned().unwrap_or(json!({}));
    let born = p.get("born").and_then(Value::as_str).and_then(|d| {
        let mut it = d.split('-').map(|x| x.parse::<i64>().ok());
        let (y, m, day) = (it.next()??, it.next()??, it.next()??);
        ((1900..2010).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&day))
            .then_some((y as i32, m as u32, day as u32))
    });
    let children = p
        .get("children")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|c| {
                    let name: String = c.get("name")?.as_str()?.trim().chars().take(40).collect();
                    let born = c.get("born")?.as_i64()? as i32;
                    (!name.is_empty()).then_some((name, born))
                })
                .take(8)
                .collect()
        })
        .unwrap_or_default();
    // A photo is a small JPEG the page made; anything else is dropped.
    let photo = p.get("photo").and_then(Value::as_str).filter(|s| photo_ok(s)).map(str::to_string);
    Profile {
        born,
        hometown: text(&p, "hometown", 60),
        high_school: text(&p, "high_school", 80),
        alma_mater: text(&p, "alma_mater", 80),
        college_years: text(&p, "college_years", 20),
        degree: text(&p, "degree", 60),
        status: text(&p, "status", 20),
        spouse: text(&p, "spouse", 60),
        roots: text(&p, "roots", 120),
        children,
        height: text(&p, "height", 20),
        build: text(&p, "build", 60),
        hair: text(&p, "hair", 60),
        facial_hair: text(&p, "facial_hair", 60),
        photo,
    }
}

pub fn start(w: &mut World, args: &Value) -> Result<(), String> {
    if w.career.is_some() {
        return Err("This world already has a career. Open a library world to start another.".into());
    }
    let bg_key = args.get("background").and_then(Value::as_str).unwrap_or("college_assistant");
    let b = BACKGROUNDS.iter().find(|b| b.key == bg_key).ok_or("No such background.")?;
    let base = starting_ratings(b);
    // The pool as the setup page shows it, in whole points.
    let pool: f64 = base.iter().sum::<f64>();
    let mut r: Vec<f64> = args
        .get("ratings")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_f64).collect())
        .filter(|v: &Vec<f64>| v.len() == RATING_COUNT)
        .unwrap_or(base);
    for x in r.iter_mut() {
        *x = x.round().clamp(1.0, 20.0);
    }
    // The pool is the limit: more than it buys is scaled back.
    let spent: f64 = r.iter().sum();
    if spent > pool + 1e-9 {
        let k = pool / spent;
        r.iter_mut().for_each(|x| *x = (*x * k).clamp(1.0, 20.0));
    }
    let at = |key: &str| r[RATINGS.iter().position(|x| x.0 == key).unwrap_or(0)];
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
    let profile = profile(args);
    // Who calls the plays: the page's choice, else the side you came up
    // teaching (a quarterbacks man calls the offense, a secondary man the
    // defense, a special teams man hands both to his coordinators).
    let top = (0..GROUP_COUNT).max_by(|&x, &y| teaching[x].total_cmp(&teaching[y]).then(y.cmp(&x))).unwrap_or(0);
    let side = if top < 4 {
        "offense"
    } else if top < GROUP_COUNT - 1 {
        "defense"
    } else {
        "none"
    };
    let calls = args.get("calls").and_then(Value::as_str).unwrap_or(side);
    // Your age from your date of birth, on today's date.
    let age = match profile.born {
        Some((y, m, d)) => {
            let (ty, tm, td) = w.today.parts();
            (ty - y - i32::from((tm, td) < (m, d))) as f64
        }
        None => f(args, "age", 42.0),
    };
    let you = You {
        name: args
            .get("name")
            .and_then(Value::as_str)
            .filter(|n| !n.trim().is_empty())
            .unwrap_or("Coach")
            .trim()
            .to_string(),
        age: age.clamp(25.0, 75.0) as u32,
        ethnicity,
        home_state,
        ex_pro: b.ex_pro,
        reputation: b.reputation,
        teaching,
        scheme: at("scheme"),
        game_management: at("game_management"),
        motivation: at("motivation"),
        discipline: at("discipline"),
        recruiting: at("recruiting"),
        network: at("network"),
        relationships: at("relationships"),
        road: b.name.to_string(),
        style: Style {
            risk: f(&s, "risk", 10.0).clamp(1.0, 20.0),
            freedom: f(&s, "freedom", 10.0).clamp(1.0, 20.0),
            physical: f(&s, "physical", 10.0).clamp(1.0, 20.0),
            ball_hunting: f(&s, "ball_hunting", 10.0).clamp(1.0, 20.0),
            mould: f(&s, "mould", 10.0).clamp(1.0, 20.0),
        },
        yours: Yours {
            play_calling: at("play_calling"),
            adaptation: at("adaptation"),
            evaluation: at("evaluation"),
            negotiating: at("negotiating"),
            calls_offense: matches!(calls, "offense" | "both"),
            calls_defense: matches!(calls, "defense" | "both"),
        },
        profile,
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
