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
    crate::lane_faces::on_rename(&from, &to);
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
    crate::lane_faces::on_delete(&p);
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

/// Your coaches (update 3, SA1): once you have made a coach and started a
/// career, the game keeps him in the game folder's `coaches` folder, apart
/// from any save, so a new career can start with him again as he is or with
/// something changed. The file is the setup page's own record of him.
fn coaches_dir(home: &Path) -> PathBuf {
    home.join("coaches")
}

/// Kept when a career starts with the setup page's record (`coach`).
pub fn save_coach_profile(home: &Path, args: &Value) {
    let Some(coach) = args.get("coach").filter(|c| c.is_object()) else { return };
    let name = args.get("name").and_then(Value::as_str).unwrap_or("").trim();
    if name.is_empty() {
        return;
    }
    let dir = coaches_dir(home);
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let road = BACKGROUNDS.iter().find(|b| Some(b.key) == args.get("background").and_then(Value::as_str));
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let rec = json!({
        "name": name,
        "road": road.map_or("", |b| b.name),
        "saved": stamp,
        "coach": coach,
    });
    let _ =
        std::fs::write(dir.join(format!("{}.json", file_safe(name))), serde_json::to_string(&rec).unwrap_or_default());
}

/// The coaches you have made, newest first.
pub fn coach_profiles(home: &Path) -> Value {
    let mut list: Vec<Value> = std::fs::read_dir(coaches_dir(home))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| {
            let mut v: Value = serde_json::from_str(&std::fs::read_to_string(e.path()).ok()?).ok()?;
            v["file"] = json!(e.path().file_name()?.to_string_lossy());
            Some(v)
        })
        .collect();
    list.sort_by_key(|v| std::cmp::Reverse(v["saved"].as_u64().unwrap_or(0)));
    json!({ "coaches": list })
}

/// Forget one of your coaches (its file only; careers keep him).
pub fn forget_coach_profile(home: &Path, args: &Value) -> Result<Value, String> {
    let file = args.get("file").and_then(Value::as_str).ok_or("Missing file.")?;
    if file.contains(['/', '\\']) || !file.ends_with(".json") {
        return Err("No such coach.".into());
    }
    std::fs::remove_file(coaches_dir(home).join(file)).map_err(|e| e.to_string())?;
    Ok(coach_profiles(home))
}

/// A road into coaching (people topic 15; coaching C3-C7, C10): its
/// starting reputation (1-20), its name and standing (C10's agreed values,
/// shown on the roads screen; how they work comes with the coaching people
/// lane), whether you played pro football, the strengths it locks in (C3,
/// C6) and the level everything else starts at (C4).
struct Background {
    key: &'static str,
    name: &'static str,
    reputation: f64,
    name_rep: f64,
    standing: f64,
    ex_pro: bool,
    /// The locked strengths, highest first: a rating's key, or "pos" (the
    /// position you played), "grp" (the group you came up coaching) or
    /// "groups" (every teaching group), with its value.
    locks: &'static [(&'static str, f64)],
    /// Where every other rating starts (C4: the world's medians for the job
    /// the road last held).
    base: f64,
    /// The group "pos" or "grp" goes to until you pick (board 40's).
    group: &'static str,
}

const BACKGROUNDS: [Background; 11] = [
    Background {
        key: "nfl_star",
        name: "Former NFL star",
        reputation: 13.0,
        name_rep: 17.0,
        standing: 7.0,
        ex_pro: true,
        locks: &[("pos", 19.0), ("network", 18.0), ("motivation", 16.0)],
        base: 12.0,
        group: "db",
    },
    Background {
        key: "nfl_pro_bowl",
        name: "Former NFL Pro Bowler",
        reputation: 11.0,
        name_rep: 14.0,
        standing: 9.0,
        ex_pro: true,
        locks: &[("pos", 18.0), ("network", 17.0), ("motivation", 15.0)],
        base: 13.0,
        group: "lb",
    },
    Background {
        key: "nfl_starter",
        name: "Former NFL starter",
        reputation: 9.0,
        name_rep: 11.0,
        standing: 10.0,
        ex_pro: true,
        locks: &[("pos", 17.0), ("motivation", 15.0), ("relationships", 15.0)],
        base: 14.0,
        group: "wr",
    },
    Background {
        key: "nfl_journeyman",
        name: "Former NFL journeyman",
        reputation: 7.0,
        name_rep: 8.0,
        standing: 9.0,
        ex_pro: true,
        locks: &[("pos", 16.0), ("network", 16.0), ("scheme", 15.0)],
        base: 13.0,
        group: "qb",
    },
    Background {
        key: "college_player",
        name: "Former college player",
        reputation: 5.0,
        name_rep: 5.0,
        standing: 6.0,
        ex_pro: false,
        locks: &[("pos", 15.0), ("recruiting", 15.0)],
        base: 12.0,
        group: "db",
    },
    Background {
        key: "never_played",
        name: "Never played past high school",
        reputation: 3.0,
        name_rep: 2.0,
        standing: 5.0,
        ex_pro: false,
        locks: &[("scheme", 18.0), ("play_calling", 17.0)],
        base: 12.0,
        group: "",
    },
    Background {
        key: "hs_coach",
        name: "High school coach",
        reputation: 4.0,
        name_rep: 4.0,
        standing: 6.0,
        ex_pro: false,
        locks: &[("motivation", 16.0), ("discipline", 16.0), ("groups", 14.0)],
        base: 13.0,
        group: "",
    },
    Background {
        key: "college_assistant",
        name: "College position coach or coordinator",
        reputation: 8.0,
        name_rep: 6.0,
        standing: 10.0,
        ex_pro: false,
        locks: &[("recruiting", 18.0), ("grp", 17.0), ("relationships", 15.0)],
        base: 14.0,
        group: "wr",
    },
    Background {
        key: "nfl_assistant",
        name: "NFL assistant or coordinator",
        reputation: 10.0,
        name_rep: 6.0,
        standing: 13.0,
        ex_pro: false,
        locks: &[("scheme", 18.0), ("play_calling", 17.0), ("grp", 17.0)],
        base: 14.0,
        group: "db",
    },
    Background {
        key: "scout",
        name: "Scout or personnel man",
        reputation: 6.0,
        name_rep: 3.0,
        standing: 8.0,
        ex_pro: false,
        locks: &[("evaluation", 19.0), ("network", 17.0), ("negotiating", 15.0)],
        base: 11.0,
        group: "",
    },
    Background {
        key: "analytics",
        name: "Analytics",
        reputation: 5.0,
        name_rep: 3.0,
        standing: 6.0,
        ex_pro: false,
        locks: &[("evaluation", 19.0), ("game_management", 19.0), ("negotiating", 16.0)],
        base: 11.0,
        group: "",
    },
];

/// The 19 ratings, in the order the setup shows them: the eight teaching
/// groups, then game day, people and building the roster. Each is (key,
/// name, group).
const RATINGS: [(&str, &str, &str); 19] = [
    ("qb", "Quarterbacks", "Teaching"),
    ("rb", "Running backs", "Teaching"),
    ("wr", "Receivers and tight ends", "Teaching"),
    ("ol", "Offensive line", "Teaching"),
    ("dl", "Defensive line", "Teaching"),
    ("lb", "Linebackers", "Teaching"),
    ("db", "Defensive backs", "Teaching"),
    ("st", "Special teams", "Teaching"),
    ("scheme", "Scheme", "Game day"),
    ("game_management", "Game management", "Game day"),
    ("play_calling", "Play calling", "Game day"),
    ("adaptation", "In-game adaptation", "Game day"),
    ("motivation", "Motivation", "People"),
    ("discipline", "Discipline", "People"),
    ("relationships", "Relationships", "People"),
    ("recruiting", "Recruiting", "Building the roster"),
    ("evaluation", "Evaluation", "Building the roster"),
    ("network", "Network", "Building the roster"),
    ("negotiating", "Negotiating", "Building the roster"),
];
const RATING_COUNT: usize = RATINGS.len();

/// A road's position or group pick (C5): the playing roads' position, the
/// assistant roads' group.
fn picks_group(b: &Background) -> Option<&'static str> {
    b.locks.iter().find_map(|l| match l.0 {
        "pos" => Some("pos"),
        "grp" => Some("grp"),
        _ => None,
    })
}

/// The group a road's "pos" or "grp" strength goes to: yours if you picked
/// one, else the road's own.
fn group_of(b: &Background, picked: Option<&str>) -> Option<usize> {
    picks_group(b)?;
    let key = picked.filter(|k| RATINGS[..GROUP_COUNT].iter().any(|r| r.0 == *k)).unwrap_or(b.group);
    RATINGS[..GROUP_COUNT].iter().position(|r| r.0 == key)
}

/// A road's starting ratings and each one's floor (C3: a locked strength
/// can't be spent below the road's value; every other rating can go to 1).
fn starting(b: &Background, picked: Option<&str>) -> (Vec<f64>, Vec<f64>) {
    let mut r = vec![b.base; RATING_COUNT];
    let mut floor = vec![1.0; RATING_COUNT];
    let g = group_of(b, picked);
    for &(key, v) in b.locks {
        let at: Vec<usize> = match key {
            "groups" => (0..GROUP_COUNT).collect(),
            "pos" | "grp" => g.into_iter().collect(),
            k => RATINGS.iter().position(|r| r.0 == k).into_iter().collect(),
        };
        for i in at {
            r[i] = v;
            floor[i] = v;
        }
    }
    (r, floor)
}

fn starting_ratings(b: &Background) -> Vec<f64> {
    starting(b, None).0
}

/// What a road is strongest at, in words ("Evaluation, network").
fn strongest(b: &Background) -> String {
    let words: Vec<String> = b
        .locks
        .iter()
        .take(2)
        .map(|(k, _)| match *k {
            "pos" => "his position".to_string(),
            "grp" => "his group".to_string(),
            "groups" => "every group".to_string(),
            k => RATINGS.iter().find(|r| r.0 == k).map_or(k.to_string(), |r| r.1.to_lowercase()),
        })
        .collect();
    let s = words.join(", ");
    s[..1].to_uppercase() + &s[1..]
}

pub fn options(w: &World) -> Value {
    let mut programs: Vec<&str> = w.programs.iter().map(|p| w.program_name(p.id)).collect();
    programs.sort_unstable();
    programs.dedup();
    let m = afm_engine::world::road_marks::road_marks(w);
    let pair = |(a, b): (f64, f64)| json!([a, b]);
    json!({
        "today": date_text(w.today),
        "year": w.today.year(),
        "backgrounds": BACKGROUNDS.iter().map(|b| {
            let (r, floor) = starting(b, None);
            let locked: f64 = r.iter().zip(&floor).filter(|(_, f)| **f > 1.0).map(|(v, _)| v).sum();
            let free = floor.iter().filter(|f| **f <= 1.0).count();
            json!({
                "key": b.key, "name": b.name, "reputation": b.reputation, "ex_pro": b.ex_pro,
                "name_rep": b.name_rep, "standing": b.standing,
                "locks": b.locks.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>(),
                "base": b.base, "group": b.group, "picks": picks_group(b),
                "locked": locked, "free": free, "pool": r.iter().sum::<f64>(),
                "ratings": r, "floor": floor, "strongest": strongest(b),
            })
        }).collect::<Vec<_>>(),
        "ratings": RATINGS.iter().map(|r| json!({ "key": r.0, "name": r.1, "group": r.2 })).collect::<Vec<_>>(),
        // How you compare (C7, board 40): the world's own coaches at each level.
        "marks": {
            "hs": { "best": m.hs_best, "other_top": m.hs_other_top },
            "college": {
                "groups": m.fbs_group.iter().map(|&g| pair(g)).collect::<Vec<_>>(),
                "coord_scheme": m.fbs_coord_scheme,
                "coord_other": m.coord_other,
            },
            "nfl": { "scheme": pair(m.nfl_scheme), "calling": pair(m.nfl_calling), "eval": pair(m.nfl_eval) },
        },
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
            json!({ "name": co.name, "age": co.age_on(w.today), "record": job_record(w, c), "seasons": years, "id": c })
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
        // The other pro leagues' clubs playing this year or next (later
        // extras topic 5): a club works like an NFL seat on its league's
        // own rules, money and calendar.
        "other" => {
            use afm_engine::world::pro::other as o;
            let mut clubs: Vec<usize> = (0..o::LEAGUE_COUNT as u8)
                .flat_map(|k| o::teams_in(k, w.year).into_iter().chain(o::teams_in(k, w.year + 1)))
                .collect();
            clubs.sort_unstable();
            clubs.dedup();
            clubs
                .into_iter()
                .map(|f| {
                    let k = o::league_of(f);
                    let titles: Vec<i32> =
                        w.others.seasons.iter().filter(|s| s.champion == Some(f as u16)).map(|s| s.year).collect();
                    let last = o::record_of(w, f);
                    let year =
                        if o::format(k, w.year).is_some_and(|x| x.start >= w.today) { w.year } else { w.year + 1 };
                    let opens = o::format(k, year).map(|x| x.start);
                    let mut j = base(TeamRef::Other(f));
                    j["league"] = json!(o::league_short(k));
                    j["league_name"] = json!(o::league_name(k, year));
                    j["city"] = json!(o::city(f, w.year));
                    j["titles"] = json!(titles.len());
                    j["last_title"] = json!(titles.iter().max());
                    j["trophy"] = json!(o::format(k, year).map(|x| x.title));
                    j["prestige"] = json!((o::prestige(f) * 100.0).round() / 100.0);
                    j["prestige_word"] = json!(match k {
                        o::CFL => "Canada's top league",
                        o::AFL1 | o::AFL2 => "Indoor football",
                        _ => "A spring league",
                    });
                    j["pay"] = json!(o::salary(f, afm_engine::world::coach::Role::HeadCoach, w.year).round());
                    j["coach"] = json!(coach(o::head_coach(w, f)));
                    j["expectation"] = json!(match last {
                        Some((_, r)) if r.w > r.l + 3 =>
                            format!("Win the {}.", o::format(k, year).map_or("title", |x| x.title)),
                        Some((_, r)) if r.w >= r.l => "Make the playoffs and win a round.".to_string(),
                        Some(_) => "A winning season and a playoff spot.".to_string(),
                        None =>
                            "Build a club from nothing: a roster by camp and a season that sells tickets.".to_string(),
                    });
                    j["calendar"] = json!(match opens {
                        Some(d) => format!("The {} season opens {}.", o::league_short(k), date_text(d)),
                        None => "No season is set yet.".to_string(),
                    });
                    j["sub"] = json!(format!("{} · {}", o::league_name(k, year), o::city(f, w.year)));
                    (k as f64 * 100.0 - o::prestige(f), j)
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
                    j["class_rank"] = json!(s.classification);
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
                    j["conference"] = json!(w.conference_name(&p.conference));
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
                        w.conference_name(&p.conference),
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
            let at = crate::views::employer_name(w, j.employer);
            let role = j.role.name();
            json!({
                "id": c, "name": co.name, "age": co.age_on(w.today),
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
    let you = you_from(w, args)?;
    let t = crate::views::TeamRef::from_args(w, args.get("job").unwrap_or(&Value::Null))?;
    let job = match t {
        TeamRef::Nfl(i) => Job::Nfl(i),
        TeamRef::College(i) => Job::College(i),
        TeamRef::HighSchool(i) => Job::HighSchool(i),
        TeamRef::Other(f) => Job::Other(f),
    };
    let seed = career_seed(args);
    w.start_career(&you, job, seed);
    let (own, stops) = crate::lane_frontoffice::setup::own_school_and_stops(w, args);
    road_ties(w, args, own, &stops);
    Ok(())
}

/// The career's seed: the page's, else one from the clock.
pub(crate) fn career_seed(args: &Value) -> u32 {
    args.get("seed").and_then(Value::as_u64).map(|s| s as u32).unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(7, |d| d.subsec_nanos() ^ d.as_secs() as u32)
    })
}

/// Whether a road played in college: every road but the one that never
/// played past high school, as the Background step places you (most
/// assistants, scouts and even analysts played somewhere).
pub(crate) fn played_college(background: &str) -> bool {
    BACKGROUNDS.iter().any(|b| b.key == background && b.key != "never_played")
}

/// You, from the setup's answers.
pub(crate) fn you_from(w: &World, args: &Value) -> Result<You, String> {
    let bg_key = args.get("background").and_then(Value::as_str).unwrap_or("college_assistant");
    let b = BACKGROUNDS.iter().find(|b| b.key == bg_key).ok_or("No such background.")?;
    // The position you played or the group you came up coaching (C5).
    let picked = args.get("position").and_then(Value::as_str);
    let group = group_of(b, picked);
    let (base, floor) = starting(b, picked);
    // The road's total as the setup page shows it, in whole points: the
    // locked strengths and the pool together (C3, C4).
    let total: f64 = base.iter().sum::<f64>();
    let mut r: Vec<f64> = args
        .get("ratings")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_f64).collect())
        .filter(|v: &Vec<f64>| v.len() == RATING_COUNT)
        .unwrap_or(base);
    for (x, f) in r.iter_mut().zip(&floor) {
        *x = x.round().clamp(*f, 20.0);
    }
    // The total is the limit: points past it come off the highest ratings
    // above their floor, never off a locked strength's value.
    let mut spent: f64 = r.iter().sum();
    while spent > total + 1e-9 {
        let Some(k) = (0..RATING_COUNT).filter(|&k| r[k] > floor[k]).max_by(|&a, &b| r[a].total_cmp(&r[b])) else {
            break;
        };
        r[k] -= 1.0;
        spent -= 1.0;
    }
    let at = |key: &str| r[RATINGS.iter().position(|x| x.0 == key).unwrap_or(0)];
    let mut teaching = [0.0; GROUP_COUNT];
    teaching.copy_from_slice(&r[..GROUP_COUNT]);
    let s = args.get("style").cloned().unwrap_or(json!({}));
    let state = args.get("home_state").and_then(Value::as_str).unwrap_or("TX");
    let home_state = w.states.iter().position(|x| x.code == state).unwrap_or(0);
    let ethnicity =
        args.get("ethnicity").and_then(Value::as_str).and_then(Ethnicity::from_key).unwrap_or(Ethnicity::White);
    let profile = profile(args);
    // Who calls the plays: the page's choice, else the side you came up
    // teaching (a quarterbacks man calls the offense, a secondary man the
    // defense, a special teams man hands both to his coordinators).
    // Your position or group decides it on the roads that pick one (C5).
    let top = group.unwrap_or_else(|| {
        (0..GROUP_COUNT).max_by(|&x, &y| teaching[x].total_cmp(&teaching[y]).then(y.cmp(&x))).unwrap_or(0)
    });
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
        born: match profile.born {
            Some((y, m, d)) => afm_engine::calendar::Date::ymd(y, m, d),
            None => afm_engine::calendar::born::birth_date(
                w.options.seed as u64 ^ 0x70_0B0E,
                age.clamp(25.0, 75.0) as u32,
                w.year,
            ),
        },
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
        // Read from your road and your people ratings when the career starts.
        aura: 0.0,
        came_up: group.map(|g| afm_engine::world::coach::Group::ALL[g]),
    };
    Ok(you)
}

/// How strong each tie your road brings starts (C9; proposed, not
/// confirmed): as strong as the world's own coaches' ties after a similar
/// career, measured on the trial 10 world in 2026 (`docs/game.md`, update 3
/// lane 2). A coach with ten or more college years holds his ties at a
/// median 0.045-0.073 (FBS, FCS, Division II, NAIA); an FBS coordinator's
/// best tie is a median 0.16.
const TIE_AREA: f64 = 0.06;
const TIE_OWN: f64 = 0.16;
const HS_COACH_TIES: usize = 30;
const ASSISTANT_TIES: usize = 25;

/// The high schools your road gives you ties to (coaching C9): a high
/// school coach about 30 programs in his state (his own area's first, then
/// the state's strongest), a college assistant about 25 in his recruiting
/// area (the states of his college stops, else his home state; the
/// strongest programs, where the prospects are), and every playing road
/// his own high school. They sit on your coach like any coach's ties, fade
/// the same way and grow with the players you coach.
pub(crate) fn road_ties(w: &mut World, args: &Value, own: Option<usize>, stops: &[afm_engine::world::coach::Employer]) {
    use afm_engine::world::coach::Employer;
    let Some(c) = w.career.as_ref().map(|c| c.coach) else { return };
    let bg = args.get("background").and_then(Value::as_str).unwrap_or("");
    let Some(b) = BACKGROUNDS.iter().find(|b| b.key == bg) else { return };
    let home = w.career.as_ref().map_or(0, |c| c.you.home_state);
    let pick = |w: &World, states: &[usize], first_area: Option<usize>, n: usize| -> Vec<usize> {
        let mut v: Vec<usize> = states.iter().flat_map(|&s| w.states[s].schools.iter().copied()).collect();
        v.sort_by(|&a, &z| {
            let (ha, hz) = (&w.high_schools[a], &w.high_schools[z]);
            let near = |h: &afm_engine::world::high_school::HighSchool| first_area.is_some_and(|x| h.area == x);
            near(hz).cmp(&near(ha)).then(hz.prestige.total_cmp(&ha.prestige)).then(a.cmp(&z))
        });
        v.truncate(n);
        v
    };
    let mut ties: Vec<(usize, f64)> = Vec::new();
    match b.key {
        "hs_coach" => {
            let state = own.map_or(home, |h| w.high_schools[h].state);
            let area = own.map(|h| w.high_schools[h].area);
            ties.extend(pick(w, &[state], area, HS_COACH_TIES).into_iter().map(|h| (h, TIE_AREA)));
        }
        "college_assistant" => {
            let mut states: Vec<usize> = stops
                .iter()
                .filter_map(|e| match e {
                    Employer::College(p) => w.programs.get(*p).map(|q| q.state),
                    _ => None,
                })
                .collect();
            states.sort_unstable();
            states.dedup();
            if states.is_empty() {
                states.push(home);
            }
            ties.extend(pick(w, &states, None, ASSISTANT_TIES).into_iter().map(|h| (h, TIE_AREA)));
        }
        _ => {}
    }
    if picks_group(b) == Some("pos")
        && let Some(h) = own
    {
        ties.push((h, TIE_OWN));
    }
    let coach = &mut w.coaches[c];
    for (h, t) in ties {
        let e = coach.ties.entry(h).or_insert(0.0);
        *e = e.max(t);
    }
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
