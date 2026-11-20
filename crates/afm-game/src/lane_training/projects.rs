//! Your development projects (the progression update, design items 7-15):
//! a skill project run by his position room and a body project run by the
//! strength staff, started and ended from the Development tab's Focus
//! sub-tab. The same rules as an AI staff's (`progression::start_project`,
//! `start_body`, `Room::slots`, one body project a room as the AI's
//! strength staff runs, `BODY_SLOTS`); every number shown is your staff's
//! read through its eye (`progression::your_project_read`,
//! `your_body_read`), never his room, knack or ceiling.

use afm_engine::attributes::Attr;
use afm_engine::player::Level;
use afm_engine::progression::{self as pg, BodyKind, Room};
use afm_engine::rng::Rng;
use afm_engine::training::seat as ts;
use afm_engine::world::people::Support;
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use crate::reads;
use crate::views::{TeamRef, eye_for, mine, person_index, roster_people};

/// The most project years the read looks ahead.
const LOOK_YEARS: u32 = 4;

pub(crate) fn room_label(r: Room) -> &'static str {
    match r {
        Room::Qb => "QB",
        Room::Rb => "RB",
        Room::Wr => "WR",
        Room::Te => "TE",
        Room::Ol => "O-line",
        Room::Dl => "D-line",
        Room::Lb => "LB",
        Room::Cb => "CB",
        Room::S => "S",
    }
}

const ROOMS: [Room; 9] = [Room::Qb, Room::Rb, Room::Wr, Room::Te, Room::Ol, Room::Dl, Room::Lb, Room::Cb, Room::S];

fn kind_key(k: BodyKind) -> &'static str {
    match k {
        BodyKind::Strength => "strength",
        BodyKind::Speed => "speed",
    }
}

pub(crate) fn kind_of(s: &str) -> Option<BodyKind> {
    match s {
        "strength" => Some(BodyKind::Strength),
        "speed" => Some(BodyKind::Speed),
        _ => None,
    }
}

pub(crate) fn kind_label(k: BodyKind) -> &'static str {
    match k {
        BodyKind::Strength => "Strength project",
        BodyKind::Speed => "Speed project",
    }
}

fn kind_what(k: BodyKind) -> &'static str {
    match k {
        BodyKind::Strength => "The long leash: upper and lower strength, a little balance, and the weight it carries.",
        BodyKind::Speed => "The short leash: a touch of speed, burst, agility and hips.",
    }
}

/// How your position coach reads his knack for new technique, in words.
fn knack_words(z: f64) -> &'static str {
    match pg::tier(z) {
        0 => "takes to new technique quickly",
        1 => "picks up new technique at a normal rate",
        _ => "is slow to take new technique",
    }
}

/// A level for the read: only the pros' stage and high school matter.
fn level_of(w: &World, i: usize) -> Level {
    match w.people[i].stage {
        Stage::Pro => Level::NFL,
        Stage::HighSchool | Stage::PrepSchool => Level::HS,
        _ => Level::FBS,
    }
}

/// Your strength head, 1-20 (an empty post reads as average). A high
/// school has none: its coaches run the weight room.
fn strength_head(w: &World, t: TeamRef, i: usize) -> f64 {
    match t {
        TeamRef::College(c) => 10.0 + 10.0 * Support::z(w.programs[c].support.strength.rating),
        TeamRef::Nfl(c) => w.clubs.get(c).map_or(10.0, |cl| 10.0 + 10.0 * Support::z(cl.support.strength.rating)),
        _ => w.training_team().map_or(9.0, |st| ts::staff_teaching(w, &st, i)),
    }
}

/// Your staff's code for `Prog::staff`: a project he brought from another
/// staff is not running here (`Prog::left_behind`).
fn staff_of(t: TeamRef) -> u32 {
    match t {
        TeamRef::Nfl(c) => pg::staff_code(pg::StaffAt::Nfl, c),
        TeamRef::College(c) => pg::staff_code(pg::StaffAt::College, c),
        TeamRef::HighSchool(c) => pg::staff_code(pg::StaffAt::HighSchool, c),
        TeamRef::Other(c) => u32::MAX - c as u32,
    }
}

/// The years a project has with you: his seasons left (at most four), a
/// pro's next four.
fn years_left(w: &World, i: usize) -> u32 {
    super::craft::seasons_left(w, i).unwrap_or(LOOK_YEARS).clamp(1, LOOK_YEARS)
}

/// A stable stream for this read: your career, him, the skill and the year
/// (a new year is a new look).
fn read_rng(w: &World, id: u32, k: u32) -> Rng {
    let r = crate::views::reader_key(w);
    Rng::new(
        (r as u32 ^ (r >> 32) as u32)
            ^ id.wrapping_mul(0x9E37_79B1)
            ^ k.wrapping_mul(0x85EB_CA6B)
            ^ (w.year as u32) << 20,
    )
}

/// Who is on a project in each of your rooms (yours, or one the staff
/// before you set), by index. One he brought from another staff ended
/// when he moved and holds no slot.
struct Rooms {
    skill: Vec<(Room, Vec<usize>)>,
    body: Vec<(Room, Vec<usize>)>,
}

fn rooms(w: &World, t: TeamRef) -> Rooms {
    let roster = roster_people(w, t);
    let staff = staff_of(t);
    let of = |body: bool| {
        ROOMS
            .iter()
            .map(|&r| {
                let on: Vec<usize> = roster
                    .iter()
                    .copied()
                    .filter(|&k| Room::of(w.people[k].player.position) == Some(r))
                    .filter(|&k| {
                        let pr = &w.people[k].player.prog;
                        !pr.left_behind(staff)
                            && if body { pr.body_project().is_some() } else { pr.project().is_some() }
                    })
                    .collect();
                (r, on)
            })
            .collect::<Vec<_>>()
    };
    Rooms { skill: of(false), body: of(true) }
}

fn on_json(w: &World, k: usize, body: bool) -> Value {
    let p = &w.people[k];
    let pr = &p.player.prog;
    let what = if body {
        pr.body_project().map(|b| kind_label(b).to_string())
    } else {
        pr.project().map(|a| a.label().to_string())
    };
    json!({
        "id": p.player.id,
        "name": p.name,
        "pos": p.player.position.name(),
        "what": what,
        "years": if body { pr.body_years } else { pr.project_years },
        "yours": if body { pr.body_yours } else { pr.yours },
    })
}

/// The room summary for the Training screen: projects in use against each
/// room's slots.
pub(super) fn summary(w: &World) -> Value {
    let Some(t) = mine(w) else { return Value::Null };
    let r = rooms(w, t);
    let rows: Vec<Value> = ROOMS
        .iter()
        .enumerate()
        .map(|(k, &room)| {
            let s = &r.skill[k].1;
            let b = &r.body[k].1;
            json!({
                "room": room_label(room),
                "slots": room.slots(),
                "used": s.len(),
                "on": s.iter().map(|&i| on_json(w, i, false)).collect::<Vec<_>>(),
                "body_slots": pg::BODY_SLOTS,
                "body_used": b.len(),
                "body_on": b.iter().map(|&i| on_json(w, i, true)).collect::<Vec<_>>(),
            })
        })
        .collect();
    let used: usize = r.skill.iter().map(|x| x.1.len()).sum();
    let body: usize = r.body.iter().map(|x| x.1.len()).sum();
    json!({ "rooms": rows, "used": used, "slots": ROOMS.iter().map(|r| r.slots()).sum::<usize>(), "body_used": body, "body_slots": pg::BODY_SLOTS * ROOMS.len() })
}

/// The projects block of his Development tab: what runs, the room's slots
/// and, for each skill and body project he could start, your staff's read.
pub(super) fn tab(w: &World, t: TeamRef, i: usize) -> Value {
    let p = &w.people[i];
    let id = p.player.id;
    let pos = p.player.position;
    let Some(room) = Room::of(pos) else {
        return json!({ "none": "Kickers, punters and snappers have no projects: their rooms have no slots." });
    };
    let level = level_of(w, i);
    let eye = eye_for(w, t, pos);
    let coaching = w.training_team().map_or(9.0, |st| ts::staff_teaching(w, &st, i));
    let years = years_left(w, i);
    let attrs = reads::attributes(w, i);
    let top_of = |a: Attr| match attrs.iter().find(|x| x.0 == a).map(|x| x.1) {
        Some(reads::Read::Range(_, hi)) => hi as f64,
        // Not seen yet: the read takes a middling 50, never his own number.
        _ => 50.0,
    };
    let rs = rooms(w, t);
    let k = ROOMS.iter().position(|&r| r == room).unwrap_or(0);
    // Weeks on a project teach the coach how he takes to that kind of
    // technique (design item 3): his read of it stands in for the clue he
    // gave before any work, on every skill of that kind.
    let worked = project_read(w, t, i).and_then(|(a, z)| Some((pg::kind_of(a)?, z?)));
    let others = |v: &[usize]| v.iter().copied().filter(|&x| x != i).map(|x| on_json(w, x, false)).collect::<Vec<_>>();
    let others_body =
        |v: &[usize]| v.iter().copied().filter(|&x| x != i).map(|x| on_json(w, x, true)).collect::<Vec<_>>();
    let skill_used = rs.skill[k].1.iter().filter(|&&x| x != i).count();
    let body_used = rs.body[k].1.iter().filter(|&&x| x != i).count();
    // Past his fourth pro season a project opens nothing (design item 16).
    let pro_done = level == Level::NFL && pg::stage_mult(&p.player, level) <= 0.0;
    let read_skill = |a: Attr| {
        let mut r = read_rng(w, id, a.index() as u32);
        let (lo, hi, seen) = pg::your_project_read(
            &p.player,
            a,
            pg::ReadOn {
                eye,
                coaching,
                level,
                years,
                seen_top: top_of(a),
                knack_seen: worked.filter(|x| Some(x.0) == pg::kind_of(a)).map(|x| x.1),
            },
            &mut r,
        );
        let (lo, hi) = (lo.round(), hi.round().max(lo.round()));
        json!({
            "key": a.key(),
            "label": a.label(),
            "read": attrs.iter().find(|x| x.0 == a).map(|x| x.1.json()),
            "gain": [lo, hi],
            "knack": knack_words(seen),
            "little": hi < 2.0,
        })
    };
    // What he brought from his last staff ended when he moved: it shows as
    // a note, holds no slot and leaves him free to start.
    let left = p.player.prog.left_behind(staff_of(t));
    let running = p.player.prog.project().filter(|_| !left);
    let skills: Vec<Value> = pg::project_skills(&p.player).into_iter().map(read_skill).collect();
    let skill = json!({
        "on": running.map(|a| {
            let mut v = read_skill(a);
            v["years"] = json!(p.player.prog.project_years);
            v["yours"] = json!(p.player.prog.yours);
            // A dead end your staff sees (design item 15), the same rule as
            // its note at the checkpoint.
            v["stalled"] = json!(pg::you_see_dead_end(&p.player));
            v
        }),
        "options": skills,
        "left": p.player.prog.project().filter(|_| left).map(|a| a.label()),
        "room": room_label(room),
        "slots": room.slots(),
        "used": skill_used,
        "others": others(&rs.skill[k].1),
        "years": years,
        "minutes": 60,
        "cost": (100.0 * (1.0 - pg::PROJECT_COST)).round(),
        "blocked": if pro_done {
            Some("Past his fourth pro season a project opens nothing.".to_string())
        } else if running.is_none() && skill_used >= room.slots() {
            Some(format!("The {} room's {} in use.", room_label(room), if room.slots() == 1 { "project is".to_string() } else { format!("{} projects are", room.slots()) }))
        } else {
            None
        },
    });
    // The strength staff's side.
    let staff = strength_head(w, t, i);
    let age = p.player.age;
    let body_years = (pg::BODY_UNTIL as i64 - age as i64 + 1).clamp(0, LOOK_YEARS as i64) as u32;
    let read_body = |kind: BodyKind| {
        let (rows, lbs) = pg::your_body_read(&p.player, kind, staff, staff, body_years.max(1));
        let total: f64 = rows.iter().map(|r| r.2).sum();
        json!({
            "key": kind_key(kind),
            "label": kind_label(kind),
            "what": kind_what(kind),
            "rows": rows.iter().map(|&(a, lo, hi)| json!({ "label": a.label(), "gain": [round1(lo), round1(hi.max(lo))] })).collect::<Vec<_>>(),
            "lbs": if kind == BodyKind::Strength { json!([lbs.0.round(), lbs.1.round().max(lbs.0.round())]) } else { Value::Null },
            "little": total < 1.0,
        })
    };
    let body_on = p.player.prog.body_project().filter(|_| !left);
    let body = json!({
        "on": body_on.map(|k| {
            let mut v = read_body(k);
            v["years"] = json!(p.player.prog.body_years);
            v["yours"] = json!(p.player.prog.body_yours);
            v["lbs_on"] = json!(p.player.prog.body_lbs);
            v
        }),
        "options": [read_body(BodyKind::Strength), read_body(BodyKind::Speed)],
        "left": p.player.prog.body_project().filter(|_| left).map(kind_label),
        "slots": pg::BODY_SLOTS,
        "used": body_used,
        "others": others_body(&rs.body[k].1),
        "years": body_years,
        "until": pg::BODY_UNTIL,
        "wear": pg::BODY_WEAR,
        "hs_note": level == Level::HS,
        "blocked": if body_years == 0 {
            Some(format!("Past {} his body has done its building: a body project adds nothing.", pg::BODY_UNTIL))
        } else if body_on.is_none() && body_used >= pg::BODY_SLOTS {
            Some(format!("Your strength staff already runs a body project in the {} room.", room_label(room)))
        } else {
            None
        },
    });
    json!({ "skill": skill, "body": body, "may_plan": w.may_plan(id), "eye": super::craft::eye_words(eye) })
}

fn round1(x: f64) -> f64 {
    (x * 2.0).round() / 2.0
}

/// Checks he is yours to plan and returns his index and your team.
fn yours(w: &World, id: u32) -> Result<(usize, TeamRef), String> {
    let t = mine(w).ok_or("You have no team.")?;
    let i = person_index(w, id).ok_or("No such player.")?;
    if !roster_people(w, t).contains(&i) {
        return Err("He isn't on your team.".into());
    }
    if !w.may_plan(id) {
        return Err(format!("His plan is the head coach's. Your job covers {}.", w.your_controls()));
    }
    Ok((i, t))
}

/// Whether his skill project on a skill can start, by the screen's own
/// rule (shared with a talk, talk replies item 13): Err says why not; Ok
/// holds the man whose project it would replace when his room's slots are
/// all taken (the one with the fewest weeks in).
pub(crate) fn skill_check(w: &World, id: u32, key: &str) -> Result<Option<u32>, String> {
    let (i, t) = yours(w, id)?;
    let a = Attr::ALL.iter().copied().find(|a| a.key() == key).ok_or("No such skill.")?;
    let p = &w.people[i].player;
    let room = Room::of(p.position).ok_or("His room has no project slots.")?;
    let staff = staff_of(t);
    if let Some(on) = p.prog.project().filter(|_| !p.prog.left_behind(staff)) {
        return Err(format!("He is already on a {} project. End it first.", on.label().to_lowercase()));
    }
    if !pg::project_skills(p).contains(&a) {
        return Err("That skill isn't one his position builds a project on.".into());
    }
    if level_of(w, i) == Level::NFL && pg::stage_mult(p, Level::NFL) <= 0.0 {
        return Err("Past his fourth pro season a project opens nothing.".into());
    }
    let rs = rooms(w, t);
    let on: &[usize] = rs.skill.iter().find(|r| r.0 == room).map_or(&[], |r| &r.1);
    if on.len() >= room.slots() {
        let today = w.today;
        let least = on.iter().copied().min_by(|&x, &y| {
            pg::project_weeks(&w.people[x].player, today).total_cmp(&pg::project_weeks(&w.people[y].player, today))
        });
        return least.map(|k| Some(w.people[k].player.id)).ok_or_else(|| "His room has no project slots.".into());
    }
    Ok(None)
}

/// Starts his skill project on a skill (`progression::start_project`,
/// yours), inside his room's slots.
pub(super) fn start_skill(w: &mut World, id: u32, key: &str) -> Result<String, String> {
    if skill_check(w, id, key)?.is_some() {
        let (i, _) = yours(w, id)?;
        let room = Room::of(w.people[i].player.position).ok_or("His room has no project slots.")?;
        return Err(format!("The {} room has no free slot: end a project there first.", room_label(room)));
    }
    let (i, t) = yours(w, id)?;
    let a = Attr::ALL.iter().copied().find(|a| a.key() == key).ok_or("No such skill.")?;
    let staff = staff_of(t);
    let today = w.today;
    let p = &mut w.people[i].player;
    settle(p, staff);
    if !pg::start_project(p, a, true) {
        return Err("That skill isn't one his position builds a project on.".into());
    }
    if p.prog.project_on == 0 {
        p.prog.project_on = today.0.clamp(1, u16::MAX as i32) as u16;
    }
    Ok(format!("{} project started. Its first year counts at his next growth checkpoint.", a.label()))
}

/// Before you start one: what he brought from his last staff ends, and his
/// projects are your staff's from here.
fn settle(p: &mut afm_engine::player::Player, staff: u32) {
    for c in pg::moved(p, staff) {
        pg::apply(p, c);
    }
    p.prog.staff = staff;
}

pub(super) fn end_skill(w: &mut World, id: u32) -> Result<String, String> {
    let (i, _) = yours(w, id)?;
    let p = &mut w.people[i].player;
    let a = p.prog.project().ok_or("He has no skill project.")?;
    pg::end_project(p);
    Ok(format!("{} project ended. What it built stays.", a.label()))
}

/// Whether his body project can start, by the screen's own rule (shared
/// with a talk): Err says why not; Ok holds the man whose body project it
/// would replace when your strength staff's slot for his room is in use.
pub(crate) fn body_check(w: &World, id: u32, kind: &str) -> Result<Option<u32>, String> {
    let (i, t) = yours(w, id)?;
    kind_of(kind).ok_or("No such body project.")?;
    let p = &w.people[i].player;
    let room = Room::of(p.position).ok_or("Specialists have no body projects.")?;
    let staff = staff_of(t);
    if let Some(on) = p.prog.body_project().filter(|_| !p.prog.left_behind(staff)) {
        return Err(format!("He is already on a {}. End it first.", kind_label(on).to_lowercase()));
    }
    if p.age > pg::BODY_UNTIL {
        return Err(format!("Past {} his body has done its building.", pg::BODY_UNTIL));
    }
    let rs = rooms(w, t);
    let on: &[usize] = rs.body.iter().find(|r| r.0 == room).map_or(&[], |r| &r.1);
    if on.len() >= pg::BODY_SLOTS {
        return Ok(on.first().map(|&k| w.people[k].player.id));
    }
    Ok(None)
}

/// Starts his body project (`progression::start_body`, yours), one a room
/// at a time on your strength staff.
pub(super) fn start_body(w: &mut World, id: u32, kind: &str) -> Result<String, String> {
    if body_check(w, id, kind)?.is_some() {
        let (i, _) = yours(w, id)?;
        let room = Room::of(w.people[i].player.position).ok_or("Specialists have no body projects.")?;
        return Err(format!(
            "Your strength staff's slot for the {} room is in use: end that one first.",
            room_label(room)
        ));
    }
    let (i, t) = yours(w, id)?;
    let kind = kind_of(kind).ok_or("No such body project.")?;
    let staff = staff_of(t);
    let p = &mut w.people[i].player;
    settle(p, staff);
    if !pg::start_body(p, kind, true) {
        return Err("He can't start a body project.".into());
    }
    Ok(format!("{} started. Its first year counts at his next growth checkpoint.", kind_label(kind)))
}

pub(super) fn end_body(w: &mut World, id: u32) -> Result<String, String> {
    let (i, _) = yours(w, id)?;
    let p = &mut w.people[i].player;
    let k = p.prog.body_project().ok_or("He has no body project.")?;
    pg::end_body(p);
    Ok(format!("{} ended. What it built stays.", kind_label(k)))
}

/// A fixed normal for one clue on him, for this career (it never moves, so
/// a read only sharpens as its clue grows).
fn clue_draw(w: &World, id: u32, salt: u32) -> f64 {
    let r = crate::views::reader_key(w);
    Rng::new((r as u32 ^ (r >> 32) as u32) ^ id.wrapping_mul(0x2C1B_3C6D) ^ salt.wrapping_mul(0x9E37_79B1) ^ 0x0C1E)
        .normal(0.0, 1.0)
}

/// His running skill project on your staff and your position coach's read
/// of how he takes to it (None before `CLUE_WEEKS`).
fn project_read(w: &World, t: TeamRef, i: usize) -> Option<(Attr, Option<f64>)> {
    let p = &w.people[i].player;
    if p.prog.left_behind(staff_of(t)) {
        return None;
    }
    let a = p.prog.project()?;
    let weeks = pg::project_weeks(p, w.today);
    let eye = eye_for(w, t, p.position);
    Some((a, pg::project_clue(p, a, eye, weeks, clue_draw(w, p.id, 1))))
}

fn sure_words(weeks: f64) -> &'static str {
    if weeks < 8.0 {
        "an early read"
    } else if weeks < 26.0 {
        "a fair read"
    } else {
        "a firm read"
    }
}

/// The knack's clues for the Reports sub-tab (design item 3): what his
/// staff has to go on about how fast he takes new technique and how his
/// body takes to a program, in words, never a number or a grade.
pub(super) fn clues(w: &World, t: TeamRef, i: usize) -> Value {
    let p = &w.people[i];
    let id = p.player.id;
    let coach = super::teacher(w, t, i);
    let coach_name = if coach["you"].as_bool() == Some(true) {
        "Your own read".to_string()
    } else {
        coach["name"].as_str().map_or("His position coach".to_string(), str::to_string)
    };
    let mut tech: Vec<Value> = Vec::new();
    if let Some(words) = crate::lane_paths::coachable(w, i) {
        tech.push(json!({
            "who": "Your staff's file",
            "says": words,
            "hint": "A small part of how fast he learns new technique.",
            "sure": "a small part",
        }));
    }
    if let Some((a, z)) = project_read(w, t, i) {
        let weeks = pg::project_weeks(&p.player, w.today);
        let kind = pg::kind_of(a).map_or("new", |k| k.name());
        let says = match z.map(pg::tier) {
            None => "too early to tell: he has only just started on it".to_string(),
            Some(0) => "it's starting to click".to_string(),
            Some(1) => "it's coming along".to_string(),
            Some(_) => "he's still fighting it".to_string(),
        };
        tech.push(json!({
            "who": coach_name,
            "on": format!("{} project", a.label()),
            "says": says,
            "hint": format!("How he takes to new {} technique. It sharpens the longer he works at it, sooner with a sharper coach.", kind.to_lowercase()),
            "sure": if z.is_some() { sure_words(weeks) } else { "no read yet" },
        }));
    }
    let mut body: Vec<Value> = Vec::new();
    // How he filled out in high school against his frame: there is
    // something to read once he has had a year or two of it.
    if p.player.age >= 16 {
        let z = pg::filled_out_clue(&p.player, clue_draw(w, id, 2));
        body.push(json!({
            "who": "His high school years",
            "says": match pg::tier(z) { 0 => "puts weight on easily", 1 => "filled out like most boys his size", _ => "a hard gainer" },
            "hint": "A fair hint at how his body takes to a program.",
            "sure": "a fair hint",
        }));
    }
    let key = match t {
        TeamRef::College(c) => {
            Some(afm_engine::training::testing::team_key(afm_engine::training::rules::Tier::College, c))
        }
        TeamRef::Nfl(c) => Some(afm_engine::training::testing::team_key(afm_engine::training::rules::Tier::Pro, c)),
        TeamRef::HighSchool(c) => {
            Some(afm_engine::training::testing::team_key(afm_engine::training::rules::Tier::High, c))
        }
        TeamRef::Other(_) => None,
    };
    let days = key.map_or(0, |k| p.tests.at(k).count());
    let head = strength_head(w, t, i);
    if let Some(z) = pg::testing_clue(&p.player, head, days, clue_draw(w, id, 3)) {
        body.push(json!({
            "who": "Your strength staff",
            "says": match pg::tier(z) { 0 => "took to the program", 1 => "has moved like most", _ => "hasn't moved much" },
            "hint": "How his body takes to the weight room. Sharper with each testing day, sooner with a better strength coach.",
            "sure": if days == 1 { "after one testing day".to_string() } else { format!("after {days} testing days") },
        }));
    } else {
        body.push(json!({
            "who": "Your strength staff",
            "says": "no testing day with us yet",
            "hint": "How his body takes to the weight room shows in testing days.",
            "sure": "no read yet",
        }));
    }
    json!({ "technique": tech, "body": body })
}

/// His projects for a squad row: the skill project and the body project
/// running on your staff, with who set them.
pub(super) fn row(w: &World, t: TeamRef, i: usize) -> Value {
    let pr = &w.people[i].player.prog;
    if pr.left_behind(staff_of(t)) {
        return Value::Null;
    }
    let skill = pr.project().map(|a| a.label());
    let body = pr.body_project().map(|k| match k {
        BodyKind::Strength => "Strength",
        BodyKind::Speed => "Speed",
    });
    if skill.is_none() && body.is_none() {
        return Value::Null;
    }
    json!({
        "skill": skill,
        "skill_years": pr.project_years,
        "skill_yours": pr.yours,
        "stalled": pg::you_see_dead_end(&w.people[i].player),
        "body": body,
        "body_yours": pr.body_yours,
    })
}
