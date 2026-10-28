//! Lane 7's paths and coverage (scouting update SK64-SK66).
//!
//! - **A projected path at your place (SK64).** For a prospect: when he'd
//!   play (your depth chart the year he arrives, each man ahead of him with
//!   your staff's read and the year he leaves), how he fits what you run,
//!   and who teaches him against how he takes coaching. Boards show "best
//!   player" and "best for us" side by side (`for_us`). At a high school
//!   you can't pick your kids: the path is your class against each system.
//! - **Athletes outside football (SK65)**: `outside` lists them for a club
//!   or a college (a high school's own are SK22's).
//! - **Where nobody is looking (SK66)**: the coverage map (where your staff
//!   has eyes, where good players came from, where your rivals look) and
//!   the look-back of the good players nobody on your staff ever saw.
//!
//! Every read is your staff's (`reads`); nothing true reaches a screen.
//! Proposed, not confirmed: a prospect grows from his read now toward his
//! ceiling read over four years, an older college player two points a year
//! and a pro one until 27 and down a point and a half a year past 30; a
//! college player stays for his seasons left, a pro to 32 (a quarterback
//! 36, a kicker 38).

use std::collections::{HashMap, HashSet};

use afm_engine::league::depth_slots;
use afm_engine::positions::Position;
use afm_engine::world::scouting::readers::{self, Staff};
use afm_engine::world::scouting::{self as sc, outside};
use afm_engine::world::systems::{self, FRONTS, Installed, OFFENSES, SHELLS};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use crate::reads;
use crate::views::{self, TeamRef};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let id = || args.get("id").and_then(Value::as_u64).map(|v| v as u32).ok_or_else(|| "Missing id.".to_string());
    Some(match cmd {
        "l7_path" => (|| {
            let w = game.w()?;
            let k = views::person_index(w, id()?).ok_or("No such player.")?;
            Ok(path(w, k).unwrap_or(Value::Null))
        })(),
        "l7_class_fit" => game.w().map(class_fit),
        "l7_coverage" => game.w().map(coverage),
        "l7_outside" => game.w().map(outside_list),
        "l7_outside_sign" => (|| {
            let key = args.get("key").and_then(Value::as_u64).ok_or("Missing key.")?;
            let w = game.wm()?;
            let text = outside::sign(w, key)?;
            Ok(json!({ "text": text }))
        })(),
        _ => return None,
    })
}

/// The man's read now and ceiling (centres) and where he plays.
fn read_of(w: &World, k: usize) -> Option<(f64, f64)> {
    let s = reads::seen(w, k);
    let now = s.now?.0;
    let ceiling = readers::read(w, readers::seat_staff(w)?, w.people[k].player.id).map_or(now, |(_, _, f)| f.ceiling.0);
    Some((now, ceiling.max(now)))
}

/// His read in a later year: a prospect grows toward his ceiling read.
fn grown(now: f64, ceiling: f64, years: i32) -> f64 {
    now + (ceiling - now) * (years.max(0) as f64 / 4.0).min(1.0)
}

/// The year a pro stops playing, by age.
fn pro_last(pos: Position, age: u32, year: i32) -> i32 {
    let last = match pos {
        Position::QB => 36,
        Position::K | Position::P | Position::LS => 38,
        _ => 32,
    };
    year + (last - age as i32).max(0)
}

fn pro_grown(now: f64, age: u32, years: i32) -> f64 {
    let mut r = now;
    for y in 0..years.max(0) {
        let a = age as i32 + y;
        if a < 27 {
            r += 1.0;
        } else if a >= 30 {
            r -= 1.5;
        }
    }
    r
}

/// Fit words from a staff-read fit.
fn fit_words(f: Option<f64>) -> &'static str {
    match f {
        Some(x) if x >= 0.4 => "Fits what we run",
        Some(x) if x <= -0.4 => "Not built for what we run",
        Some(_) => "Fits well enough",
        None => "Not seen enough to say",
    }
}

/// His fit to a team's systems by your staff's read of him.
fn fit(w: &World, k: usize, pos: Position, inst: &Installed) -> Option<f64> {
    let p = &w.people[k];
    let staff = readers::seat_staff(w)?;
    let (_, _, f) = readers::read(w, staff, p.player.id)?;
    systems::read_fit(
        w,
        pos,
        sc::file::listed(&p.player).1,
        |a| f.attrs[a.index()].seen.then_some(f.attrs[a.index()].centre),
        inst,
    )
}

/// How he takes coaching, from your staff's read (trait 1).
fn coachable(w: &World, k: usize) -> Option<&'static str> {
    let staff = readers::seat_staff(w)?;
    let (_, _, f) = readers::read(w, staff, w.people[k].player.id)?;
    let t = f.traits.get(1).filter(|t| t.seen)?;
    Some(match t.centre {
        c if c >= 14.0 => "takes coaching well",
        c if c <= 7.0 => "fights coaching",
        _ => "takes coaching like most",
    })
}

/// A projected path at your place for a prospect (SK64), with the score
/// boards sort "best for us" by.
pub(crate) fn path(w: &World, k: usize) -> Option<Value> {
    let p = &w.people[k];
    let pos = p.player.position;
    let season = w.dates.season;
    let (now, ceiling) = read_of(w, k)?;
    let starters = depth_slots(pos).starters as usize;
    // (arrives, the men at his spot: name, id, read by year, the last year).
    let (arrives, inst, teacher, men): (i32, Installed, Option<usize>, Vec<(usize, i32, Box<dyn Fn(i32) -> f64>)>) =
        match views::mine(w)? {
            TeamRef::College(j) if matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) => {
                let prog = &w.programs[j];
                let mut men: Vec<(usize, i32, Box<dyn Fn(i32) -> f64>)> = Vec::new();
                for &i in &prog.roster {
                    let q = &w.people[i];
                    if q.player.position != pos || i == k {
                        continue;
                    }
                    let Some((n, c)) = read_of(w, i) else { continue };
                    let left = 4i32 - q.college_seasons as i32;
                    let young = q.college_seasons < 2;
                    men.push((
                        i,
                        season + left.max(1) - 1,
                        Box::new(move |y| if young { grown(n, c, y - season) } else { n + 2.0 * (y - season) as f64 }),
                    ));
                }
                // Your commits ahead of him or with him (on your board).
                for i in w.front_office.college.board.iter().filter_map(|t| views::person_index(w, t.id)) {
                    let q = &w.people[i];
                    if i != k
                        && q.player.position == pos
                        && q.cal.commit.is_some_and(|c| c.0 == j)
                        && matches!(q.stage, Stage::HighSchool | Stage::PrepSchool)
                        && let Some((n, c)) = read_of(w, i)
                    {
                        let from = q.class_of;
                        men.push((
                            i,
                            from + 3,
                            Box::new(move |y| if y < from { 0.0 } else { grown(n, c, y - season) }),
                        ));
                    }
                }
                let g = afm_engine::world::coach::Group::of(pos) as usize;
                (p.class_of, prog.installed, prog.position_coaches[g].or(prog.head_coach), men)
            }
            TeamRef::Nfl(t) if matches!(p.stage, Stage::College | Stage::Declared) && p.pro.is_none() => {
                let club = w.clubs.get(t)?;
                let mut men: Vec<(usize, i32, Box<dyn Fn(i32) -> f64>)> = Vec::new();
                let year = w.year;
                for &i in &club.roster {
                    let q = &w.people[i];
                    if q.player.position != pos {
                        continue;
                    }
                    let Some((n, _)) = read_of(w, i) else { continue };
                    let age = q.player.age;
                    men.push((i, pro_last(pos, age, year), Box::new(move |y| pro_grown(n, age, y - year))));
                }
                let arrives = if w.today.parts().1 >= 5 { w.year + 1 } else { w.year };
                (arrives, club.installed, w.nfl.get(t).and_then(|c| c.head_coach), men)
            }
            _ => return None,
        };
    // The first year he'd be among the starters.
    let mut start = None;
    let mut ahead_then: Vec<Value> = Vec::new();
    for y in arrives..arrives + 4 {
        let his = grown(now, ceiling, y - season);
        let ahead: Vec<&(usize, i32, Box<dyn Fn(i32) -> f64>)> =
            men.iter().filter(|m| m.1 >= y && (m.2)(y) > his).collect();
        if y == arrives {
            ahead_then = ahead
                .iter()
                .map(|m| {
                    let q = &w.people[m.0];
                    json!({ "id": q.player.id, "name": q.name, "class": views::class_text(w, q), "read": reads::overall(w, m.0).json(), "leaves": m.1 })
                })
                .collect();
        }
        if ahead.len() < starters {
            start = Some(y);
            break;
        }
    }
    let wait = start.map(|s| s - arrives);
    let when = match wait {
        Some(0) => "Starts as a freshman".to_string(),
        Some(1) => "Plays in year two".to_string(),
        Some(n) => format!("Sits {n} years before he starts"),
        None => "Behind the men he'd join for four years".to_string(),
    };
    let nfl = matches!(views::mine(w), Some(TeamRef::Nfl(_)));
    let when = if nfl { when.replace("as a freshman", "as a rookie") } else { when };
    let risk = !nfl && wait.is_none_or(|n| n >= 2) && p.stars >= 4;
    let fit_v = fit(w, k, p.cal.pro_position.unwrap_or(pos), &inst);
    let teacher_v = teacher.and_then(|c| w.coaches.get(c).map(|x| (c, x.name.clone())));
    let takes = coachable(w, k);
    // Best for us: his read, less a point for each year he waits, plus
    // his fit (proposed, not confirmed).
    let score = grown(now, ceiling, 2) - 1.5 * wait.unwrap_or(4) as f64 + 3.0 * fit_v.unwrap_or(0.0);
    Some(json!({
        "arrives": arrives, "start": start, "when": when, "ahead": ahead_then, "starters": starters,
        "transfer_risk": risk,
        "fit": fit_v.map(|f| (f * 10.0).round() / 10.0), "fit_words": fit_words(fit_v),
        "scheme": if nfl { format!("{} front", if inst.front == 0 { "4-3" } else { "3-4" }) } else { format!("{:?}", OFFENSES[inst.off]) },
        "teacher": teacher_v.map(|(c, n)| json!({ "id": c, "name": n })), "takes": takes,
        "for_us": (score * 10.0).round() / 10.0,
    }))
}

/// At a high school: your class against each system (SK64): the average
/// staff-read fit of your players to each offense, front and shell.
fn class_fit(w: &World) -> Value {
    let Some(TeamRef::HighSchool(h)) = views::mine(w) else { return Value::Null };
    let Some(staff) = readers::seat_staff(w) else { return Value::Null };
    // Your staff's read of each boy, once.
    let boys: Vec<(usize, sc::file::FileRead)> = w.high_schools[h]
        .roster
        .iter()
        .filter_map(|&i| readers::read(w, staff, w.people[i].player.id).map(|r| (i, r.2)))
        .collect();
    let mut out: Vec<Value> = Vec::new();
    for (kind, n) in [("Offense", OFFENSES.len()), ("Front", FRONTS.len()), ("Coverage", SHELLS.len())] {
        let mut rows: Vec<(String, f64, usize)> = Vec::new();
        for s in 0..n {
            let inst = match kind {
                "Offense" => Installed { off: s, ..Installed::default() },
                "Front" => Installed { front: s, ..Installed::default() },
                _ => Installed { shell: s, ..Installed::default() },
            };
            let fits: Vec<f64> = boys
                .iter()
                .filter(|(i, _)| (kind == "Offense") == systems::offense(w.people[*i].player.position))
                .filter_map(|(i, f)| {
                    let p = &w.people[*i].player;
                    systems::read_fit(
                        w,
                        p.position,
                        sc::file::listed(p).1,
                        |a| f.attrs[a.index()].seen.then_some(f.attrs[a.index()].centre),
                        &inst,
                    )
                })
                .collect();
            let name = match kind {
                "Offense" => format!("{:?}", OFFENSES[s]),
                "Front" => format!("{:?}", FRONTS[s]),
                _ => format!("{:?}", SHELLS[s]),
            };
            let avg = if fits.is_empty() { 0.0 } else { fits.iter().sum::<f64>() / fits.len() as f64 };
            rows.push((name, avg, fits.len()));
        }
        rows.sort_by(|a, b| b.1.total_cmp(&a.1));
        out.push(json!({ "kind": kind, "rows": rows.iter().map(|r| json!({ "name": r.0, "fit": (r.1 * 100.0).round() / 100.0, "words": fit_words(Some(r.1)), "read": r.2 })).collect::<Vec<_>>() }));
    }
    json!({ "groups": out })
}

/// The athletes from outside football your staff heard of (SK65).
fn outside_list(w: &World) -> Value {
    let rows: Vec<Value> = outside::list(w)
        .into_iter()
        .map(|a| {
            json!({ "key": a.key, "name": a.name, "sport": a.sport, "from": a.from, "pos": a.pos.name(), "age": a.age,
                "read": [a.read.0, a.read.1], "spotted_by": a.spotted_by })
        })
        .collect();
    json!({ "rows": rows })
}

/// Where your staff has eyes and where nobody is looking (SK66).
fn coverage(w: &World) -> Value {
    let Some(staff) = readers::seat_staff(w) else { return Value::Null };
    let season = w.dates.season;
    let since = afm_engine::calendar::Date::ymd(season - 1, 8, 1);
    // Who your staff looked at this year, by player.
    let mut seen: HashSet<u32> = HashSet::new();
    for (&id, v) in &w.scouting.dept.looks {
        if v.iter().any(|l| l.date >= since) {
            seen.insert(id);
        }
    }
    if let Staff::Club(t) = staff
        && let Some(c) = w.clubs.get(t)
    {
        for (&id, v) in &c.files {
            if v.iter().any(|l| l.date >= since) {
                seen.insert(id);
            }
        }
    }
    let ever = |id: u32| -> bool {
        w.scouting.dept.looks.contains_key(&id)
            || matches!(staff, Staff::Club(t) if w.clubs.get(t).is_some_and(|c| c.files.contains_key(&id)))
    };
    match staff {
        Staff::Club(t) => nfl_coverage(w, t, &seen, &ever),
        Staff::Program(j) => college_coverage(w, j, &seen, &ever),
        Staff::School(h) => hs_coverage(w, h),
    }
}

/// By state: your looks this year, the FBS signees of the last five
/// classes, and your rivals' offers (your conference's programs).
fn college_coverage(w: &World, j: usize, seen: &HashSet<u32>, ever: &dyn Fn(u32) -> bool) -> Value {
    let season = w.dates.season;
    let conf = &w.programs[j].conference;
    let rivals: HashSet<usize> =
        (0..w.programs.len()).filter(|&x| x != j && &w.programs[x].conference == conf).collect();
    let n = w.states.len();
    let (mut eyes, mut came, mut rival) = (vec![0u32; n], vec![0u32; n], vec![0u32; n]);
    let mut missed: Vec<(f64, usize)> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        let st = p.home_state.min(n - 1);
        if seen.contains(&p.player.id) {
            eyes[st] += 1;
        }
        if matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)
            && p.class_of <= season + 2
            && p.cal.offers.iter().any(|o| rivals.contains(o))
        {
            rival[st] += 1;
        }
        // Good players of the last five classes: FBS starters.
        if p.stage == Stage::College
            && p.class_of >= season - 5
            && p.college.is_some_and(|c| w.programs[c].division == afm_engine::world::data::Division::FBS)
        {
            came[st] += 1;
            if p.last_role == Some(afm_engine::player::PlayingTime::Starter)
                && !ever(p.player.id)
                && p.class_of >= season - 3
            {
                missed.push((p.stars as f64 * -1.0 + p.college_seasons as f64, i));
            }
        }
    }
    let rows: Vec<Value> = (0..n)
        .filter(|&s| came[s] + eyes[s] + rival[s] > 0)
        .map(|s| json!({ "state": w.states[s].code, "eyes": eyes[s], "came": came[s], "rivals": rival[s] }))
        .collect();
    missed.sort_by(|a, b| b.0.total_cmp(&a.0));
    json!({
        "seat": "college", "rows": rows,
        "missed": missed.iter().take(25).map(|&(_, i)| miss_row(w, i)).collect::<Vec<_>>(),
        "missed_title": "FBS starters from the last three classes nobody on your staff ever looked at",
    })
}

/// By program: your scouts' looks this year, the draft picks of the last
/// five drafts, and the other clubs' looks.
fn nfl_coverage(w: &World, t: usize, seen: &HashSet<u32>, ever: &dyn Fn(u32) -> bool) -> Value {
    let year = w.year;
    let mut by: HashMap<usize, (u32, u32, u32)> = HashMap::new();
    let mut missed: Vec<(i64, usize)> = Vec::new();
    let others: HashSet<u32> =
        w.clubs.iter().enumerate().filter(|(c, _)| *c != t).flat_map(|(_, c)| c.files.keys().copied()).collect();
    for (i, p) in w.people.iter().enumerate() {
        let Some(c) = p.college.or_else(|| p.path.last().copied()) else { continue };
        let e = by.entry(c).or_default();
        if seen.contains(&p.player.id) {
            e.0 += 1;
        }
        if others.contains(&p.player.id) && matches!(p.stage, Stage::College | Stage::Declared) {
            e.2 += 1;
        }
        if let Some((y, round, _, _)) = p.pro.as_ref().and_then(|x| x.draft)
            && y >= year - 5
        {
            e.1 += 1;
            if y >= year - 1 && round <= 3 && !ever(p.player.id) {
                missed.push((round as i64, i));
            }
        }
        // Undrafted who made a roster last year and nobody of yours saw.
        if p.pro
            .as_ref()
            .is_some_and(|x| x.draft.is_none() && x.seasons.iter().any(|s| s.year == year - 1 && s.games >= 8))
            && p.pro.as_ref().is_some_and(|x| x.seasons.len() <= 2)
            && !ever(p.player.id)
        {
            missed.push((9, i));
        }
    }
    let mut rows: Vec<(usize, (u32, u32, u32))> = by.into_iter().filter(|x| x.1.1 > 0 || x.1.0 > 0).collect();
    rows.sort_by_key(|x| std::cmp::Reverse(x.1.1 * 4 + x.1.0));
    missed.sort_by_key(|x| x.0);
    json!({
        "seat": "nfl",
        "rows": rows.iter().take(80).map(|(c, (e, d, o))| json!({ "school": views::team_brief(w, TeamRef::College(*c)), "division": w.programs[*c].division.name(), "eyes": e, "came": d, "rivals": o })).collect::<Vec<_>>(),
        "missed": missed.iter().take(25).map(|&(_, i)| miss_row(w, i)).collect::<Vec<_>>(),
        "missed_title": "Last year's picks in the first three rounds and undrafted rookies who played, whom nobody on your staff ever saw",
    })
}

/// At a high school: the feeders (middle schools and youth leagues) and
/// whether you have been to them.
fn hs_coverage(w: &World, h: usize) -> Value {
    let feeders = afm_engine::world::front_office::school::feeders(w, h);
    let plans = &w.legacy.hs.feeders;
    let rows: Vec<Value> = feeders
        .iter()
        .map(|f| {
            let been = plans.iter().any(|x| x.name == f.name && (x.clinic || x.assistant || x.playbook || x.equipment));
            json!({ "name": f.name, "youth": f.youth, "coach": f.coach, "boys": f.boys, "share": (f.share * 100.0).round(), "last": f.last, "been": been })
        })
        .collect();
    json!({ "seat": "hs", "feeders": rows })
}

fn miss_row(w: &World, i: usize) -> Value {
    let p = &w.people[i];
    json!({
        "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "stars": p.stars,
        "where": views::where_now(w, p), "home": w.states.get(p.home_state).map(|s| s.code.clone()),
        "draft": p.pro.as_ref().and_then(|x| x.draft).map(|d| json!({ "year": d.0, "round": d.1, "pick": d.2 })),
    })
}
