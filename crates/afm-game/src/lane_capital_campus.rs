//! Capital's Campus screen (lane CS1, `docs/design/capital.md` CP21, CP37,
//! CP39, board 1): the commands behind the map of your program's buildings.
//!
//! - `capital_campus`: your program's buildings with their rooms and levels
//!   (read against the conference, CP23), a building under way with what it
//!   adds and moves, the open plots, each room against the conference's best
//!   with the athletic director's read on a renovation, the program beside
//!   yours (its rivalry game, else the conference neighbour) and what is
//!   next.
//! - `capital_renovate` {room 0-12}: ask the athletic director to renovate
//!   one room (CP18), then the same page again with his answer in `said`.
//!
//! No dollar figures leave this file (CP39): money reads in words.

use afm_engine::calendar::Date;
use afm_engine::calendar::data::RIVALRIES;
use afm_engine::world::World;
use afm_engine::world::capital::rooms::{self, Room};
use afm_engine::world::capital::{drive, week};
use afm_engine::world::finance::{Owner, Project, ProjectKind};
use serde_json::{Value, json};

use crate::{Game, Reply};

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        "capital_campus" => game.w().and_then(campus_view),
        "capital_renovate" => game.wm().and_then(|w| {
            let r = args
                .get("room")
                .and_then(Value::as_u64)
                .and_then(|r| Room::from_index(r as u8))
                .ok_or("Which room?")?;
            let p = week::your_program(w).ok_or("Renovations are the head coach's to ask for.")?;
            let said = rooms::ask_renovation(w, p, r)?;
            let mut v = campus_view(w)?;
            v["said"] = json!(said);
            Ok(v)
        }),
        _ => return None,
    })
}

fn iso(d: Date) -> String {
    let (y, m, dd) = d.parts();
    format!("{y:04}-{m:02}-{dd:02}")
}

/// A room's level read against the conference's best, in words.
fn read_words(shown: u8) -> (&'static str, &'static str) {
    match shown {
        0 => ("None yet", "bad"),
        1 => ("Far behind", "bad"),
        2 => ("Behind", "bad"),
        3 => ("Middle", ""),
        4 => ("Near the top", "top"),
        _ => ("Best in the conference", "top"),
    }
}

/// The athletic director's read of a renovation, in words.
fn odds_words(p: f64) -> &'static str {
    if p >= 0.7 {
        "he is likely to say yes"
    } else if p >= 0.5 {
        "better than even"
    } else if p >= 0.35 {
        "a coin flip"
    } else {
        "a long shot"
    }
}

/// The program beside yours: the other side of its rivalry game if it has
/// one this year, else the conference neighbour with the closest campus.
fn beside(w: &World, prog: usize) -> Option<(usize, Option<String>)> {
    let year = w.year;
    let me = &w.programs[prog];
    for r in RIVALRIES.iter() {
        let other = if r.a == me.real_name {
            r.b
        } else if r.b == me.real_name {
            r.a
        } else {
            continue;
        };
        if let Some(o) = w.programs.iter().find(|p| p.real_name == other && p.plays(year) && p.division == me.division)
        {
            return Some((o.id, Some(r.name.to_string())));
        }
    }
    let mine = me.campus.number();
    w.programs
        .iter()
        .filter(|p| {
            p.id != prog && p.plays(year) && p.division == me.division && p.conference == me.conference && p.campus.set
        })
        .min_by(|a, b| {
            let (da, db) = ((a.campus.number() - mine).abs(), (b.campus.number() - mine).abs());
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal).then(a.id.cmp(&b.id))
        })
        .map(|p| (p.id, None))
}

/// A building under way: the rooms it holds, which are new and where the
/// others move from, and where it stands.
fn under_way(w: &World, prog: usize, x: &Project) -> Value {
    let year = w.year;
    let p = &w.programs[prog];
    let mask = drive::rooms_of_project(w, prog, x);
    let levels = rooms::opening_levels(p, mask, x.cost as f64, x.opens);
    let d = w
        .capital
        .drives
        .iter()
        .rev()
        .find(|d| d.program == prog as u32 && d.kind == x.name && d.ground == Some(x.started));
    let broke = d.and_then(|d| d.ground_day).map(|g| MONTHS[(g.month() as usize).clamp(1, 12) - 1]);
    let holder =
        |r: Room| p.campus.halls.iter().filter(|h| h.holds(r)).max_by_key(|h| h.opened).map(|h| h.name.clone());
    let rows: Vec<Value> = Room::ALL
        .iter()
        .filter(|r| mask & r.bit() != 0)
        .map(|&r| {
            let new = p.campus.level(r) <= 0.0;
            json!({
                "room": r as u8,
                "name": r.words(),
                "new": new,
                "from": if new { None } else { holder(r) },
                "level": p.campus.shown(r),
                "target": levels[r as usize].round().clamp(1.0, 5.0) as u8,
            })
        })
        .collect();
    let status = match (broke, x.started) {
        (Some(m), s) if s < year => format!("Ground broke in {m} {s}."),
        (Some(m), _) => format!("Ground broke in {m}."),
        (None, _) => "The money is in and the plans are drawn.".to_string(),
    };
    json!({
        "name": x.name,
        "started": x.started,
        "opens": x.opens,
        "status": status,
        "rooms": rows,
        "temporary": p.campus.in_temporary_space(),
    })
}

fn campus_view(w: &World) -> Reply {
    let prog = afm_engine::world::ground::week::seat_program(w)
        .ok_or("Your campus is a college program's. High school and NFL buildings come with their own screen.")?;
    let year = w.year;
    let p = &w.programs[prog];
    if !p.campus.set {
        return Err("The campus is not set yet.".into());
    }
    let head = week::your_program(w).is_some();

    // Your projects: a renovation puts a room's gold square, a building a plot.
    let mine: Vec<&Project> = w
        .finance
        .projects
        .iter()
        .filter(|x| x.owner == Owner::College(prog) && !x.done && x.kind == ProjectKind::Facility)
        .collect();
    let mut renovating: Option<Room> = None;
    let mut buildings: Vec<Value> = Vec::new();
    let mut building_masks: Vec<(String, u16, [f32; rooms::ROOMS])> = Vec::new();
    for x in &mine {
        let n = x.name.strip_suffix(" renovation");
        if let Some(r) = n.and_then(|b| Room::ALL.into_iter().find(|r| r.words() == b)) {
            renovating = Some(r);
        } else {
            let mask = drive::rooms_of_project(w, prog, x);
            building_masks.push((x.name.clone(), mask, rooms::opening_levels(p, mask, x.cost as f64, x.opens)));
            buildings.push(under_way(w, prog, x));
        }
    }

    // Each room sits in the newest building that holds it.
    let owner = |r: Room| -> Option<usize> {
        p.campus
            .halls
            .iter()
            .enumerate()
            .filter(|(_, h)| h.holds(r))
            .max_by_key(|(i, h)| (h.opened, *i))
            .map(|(i, _)| i)
    };
    let room_row = |r: Room| {
        let shown = p.campus.shown(r);
        let mut target = shown;
        let mut note: Option<String> = None;
        if renovating == Some(r) {
            target = (shown + 1).min(5);
            note = Some("renovation this summer".to_string());
        }
        for (name, mask, lv) in &building_masks {
            if mask & r.bit() != 0 {
                target = target.max(lv[r as usize].round().clamp(1.0, 5.0) as u8);
                note = Some(format!("moves to the {}", name.to_lowercase()));
            }
        }
        json!({
            "room": r as u8,
            "name": r.words(),
            "level": shown,
            "target": target,
            "note": note,
            "temporary": p.campus.temporary & r.bit() != 0,
        })
    };
    let mut halls: Vec<Value> = Vec::new();
    let mut placed = 0u16;
    let mut order: Vec<usize> = (0..p.campus.halls.len()).collect();
    order.sort_by_key(|&i| (!p.campus.halls[i].holds(Room::Stadium), p.campus.halls[i].opened, i));
    for i in order {
        let h = &p.campus.halls[i];
        let held: Vec<Value> = Room::ALL
            .iter()
            .filter(|&&r| p.campus.level(r) > 0.0 && owner(r) == Some(i))
            .map(|&r| {
                placed |= r.bit();
                room_row(r)
            })
            .collect();
        if held.is_empty() {
            continue;
        }
        // CP22: a room drops a level about every ten years unless renovated.
        let dated = h.opened > 0 && h.opened <= year - 12;
        halls.push(json!({ "name": h.name, "opened": h.opened, "dated": dated, "rooms": held }));
    }
    let loose: Vec<Value> =
        Room::ALL.iter().filter(|&&r| p.campus.level(r) > 0.0 && placed & r.bit() == 0).map(|&r| room_row(r)).collect();
    if !loose.is_empty() {
        halls.push(json!({ "name": "Elsewhere on campus", "opened": 0, "dated": false, "rooms": loose }));
    }
    let free_plots = (p.campus.open_plots as usize).saturating_sub(buildings.len());

    // The conference's best against yours, room by room.
    let list: Vec<Value> = Room::ALL
        .iter()
        .map(|&r| {
            let shown = p.campus.shown(r);
            let (read, tone) = read_words(shown);
            let blocked =
                if head { rooms::renovation_block(w, prog, r) } else { Some("The head coach asks.".to_string()) };
            json!({
                "room": r as u8,
                "name": r.words(),
                "unlocks": r.unlocks(),
                "level": shown,
                "read": read,
                "tone": tone,
                "can": blocked.is_none(),
                "block": blocked,
                "odds": if blocked.is_none() { Some(odds_words(rooms::renovation_yes(w, prog, r))) } else { None },
                "renovating": renovating == Some(r),
                "temporary": p.campus.temporary & r.bit() != 0,
            })
        })
        .collect();

    // The program beside yours.
    let rival = beside(w, prog).map(|(o, game)| {
        let q = &w.programs[o];
        let rows: Vec<Value> = Room::ALL
            .iter()
            .filter(|&&r| p.campus.level(r) > 0.0 || q.campus.level(r) > 0.0)
            .map(
                |&r| json!({ "room": r as u8, "name": r.words(), "you": p.campus.shown(r), "them": q.campus.shown(r) }),
            )
            .collect();
        // The room where they are furthest ahead, else where you are.
        let gap = |r: &&Room| q.campus.shown(**r) as i32 - p.campus.shown(**r) as i32;
        let ahead = Room::ALL.iter().max_by_key(|r| gap(r)).filter(|r| gap(r) > 0);
        let behind = Room::ALL.iter().min_by_key(|r| gap(r)).filter(|r| gap(r) < 0);
        let line = match (ahead, behind) {
            (Some(&r), _) => format!(
                "Their {} reads {} above yours. Recruits who visit both schools see it.",
                r.words(),
                match gap(&&r) {
                    1 => "a level".to_string(),
                    n => format!("{n} levels"),
                }
            ),
            (None, Some(&r)) => {
                format!("Your {} reads ahead of theirs, and nothing of theirs is ahead of yours.", r.words())
            }
            _ => "The two campuses read the same, room for room.".to_string(),
        };
        let mut rows = rows;
        rows.sort_by_key(|v| {
            std::cmp::Reverse((v["them"].as_i64().unwrap_or(0) - v["you"].as_i64().unwrap_or(0)).abs())
        });
        rows.truncate(5);
        json!({ "id": o, "name": w.program_name(o), "game": game, "line": line, "rooms": rows })
    });

    // What is next.
    let missing: Vec<&str> = Room::ALL.iter().filter(|r| p.campus.level(**r) <= 0.0).map(|r| r.words()).collect();
    let open = drive::open_drive(&w.capital, prog).map(|i| &w.capital.drives[i]);
    let next = if let Some(b) = buildings.first() {
        format!(
            "The {} opens for the {} season.{}",
            b["name"].as_str().unwrap_or("building").to_lowercase(),
            b["opens"],
            if p.campus.in_temporary_space() { " Until then the team works without the rooms it moves." } else { "" }
        )
    } else if let Some(d) = open {
        format!("The drive for the {} is in {}.", d.kind, d.stage.words())
    } else if free_plots > 0 && !missing.is_empty() {
        format!(
            "The open plot has no project. You have no {}.",
            missing.iter().take(3).copied().collect::<Vec<_>>().join(", no ")
        )
    } else if free_plots > 0 {
        "The open plot has no project.".to_string()
    } else {
        "Nothing is planned.".to_string()
    };

    Ok(json!({
        "program": w.program_name(prog),
        "conference": p.conference,
        "today": iso(w.today),
        "year": year,
        "live": p.campus.live,
        "head": head,
        "halls": halls,
        "buildings": buildings,
        "free_plots": free_plots,
        "rooms": list,
        "temporary": p.campus.in_temporary_space(),
        "rival": rival,
        "next": { "text": next, "drive": open.is_some() || !buildings.is_empty() },
    }))
}
