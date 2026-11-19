//! Ground's map screen and school page (lane S1; spec `docs/design/ground.md`,
//! part A and part N, GR2, GR5, GR6, GR8, GR11, GR70).
//!
//! `ground_map` gives one state's areas the way your staff see them: who
//! leads each (GR8, from the people's standing weighted by talent), where
//! you stand (GR27), what is fogged because your ground hasn't opened it
//! (GR2), the history pins and the rivals' pipelines (GR6), and the area you
//! picked in full: its people by name, what you've signed from it. Only
//! what a staff could know shows: the public view everywhere, the people
//! and the area's words only where you are Known. `ground_school` is a
//! school's own page (GR11). Your men's routes, road days and the week's
//! hours come with lane G3, the overnight list's news order with G4.

use afm_engine::calendar::{Date, Period, recruiting_period};
use afm_engine::world::coach::Role;
use afm_engine::world::folk::facts::state_name;
use afm_engine::world::ground::areas::{self, Pin};
use afm_engine::world::ground::standing::{self, Level};
use afm_engine::world::ground::{Holder, Who, people};
use afm_engine::world::{Stage, World, geo};
use serde_json::{Value, json};

use crate::views::{TeamRef, mine, team_brief, team_colors};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        "ground_map" => game.w().and_then(|w| map(w, args)),
        "ground_school" => game.w().and_then(|w| school(w, args)),
        _ => return None,
    })
}

const WEEKDAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
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

/// Ground is your college seat's game: your program.
fn seat(w: &World) -> Result<usize, String> {
    match mine(w) {
        Some(TeamRef::College(p)) => Ok(p),
        _ => Err("Ground is a college head coach's offseason game.".into()),
    }
}

pub(crate) fn long_date(d: Date) -> String {
    let (y, m, day) = d.parts();
    format!("{}, {} {}, {}", WEEKDAYS[d.weekday().index() as usize], MONTHS[m as usize - 1], day, y)
}

pub(crate) fn short_date(d: Date) -> String {
    let (_, m, day) = d.parts();
    format!("{} {}", MONTHS[m as usize - 1], day)
}

fn period_json(w: &World) -> Value {
    let (name, note) = match recruiting_period(&w.dates, w.today) {
        Period::Contact => ("Contact period", "coaches may visit schools and talk in person"),
        Period::Evaluation => ("Evaluation period", "coaches may watch but not talk off campus"),
        Period::Quiet => ("Quiet period", "contact only on your campus"),
        Period::Dead => ("Dead period", "no in-person contact at all"),
    };
    json!({ "name": name, "note": note })
}

fn who_json(who: Who) -> Value {
    match who {
        Who::Coach(c) => json!({ "kind": "coach", "id": c }),
        Who::Trainer(f) => json!({ "kind": "trainer", "id": f }),
        Who::SevenOnSeven(f) => json!({ "kind": "seven", "id": f }),
        Who::Counselor(h) => json!({ "kind": "counselor", "id": h }),
        Who::Family(p) => json!({ "kind": "family", "id": p }),
        Who::Prep(p) => json!({ "kind": "prep", "id": p }),
    }
}

fn level_json(points: f32) -> Value {
    let l = Level::of(points);
    json!({ "n": l as usize, "name": l.name(), "points": points.round() })
}

/// The people on an area's ground the screens name: each school's head coach.
fn heads(w: &World, schools: &[usize]) -> Vec<(usize, Who)> {
    schools.iter().filter_map(|&h| people::head_coach_of(w, h).map(|c| (h, c))).collect()
}

/// A count of people by level, in words ("Home with 3, Trusted with 4").
fn level_words(counts: [usize; 4]) -> String {
    let mut parts = Vec::new();
    for l in [Level::Home, Level::Trusted, Level::Known] {
        let n = counts[l as usize];
        if n > 0 {
            parts.push(format!("{} with {n}", l.name()));
        }
    }
    if parts.is_empty() { "Nobody there knows them yet".into() } else { format!("{}.", parts.join(", ")) }
}

/// How close the second program must be to call an area contested: within 5
/// standing points of the leader, a fifth of a level (the build's pick,
/// proposed, not confirmed).
const CONTESTED: f32 = 5.0;

/// How many classes of your signees the map shows: this year's and the three before.
const SIGNED_BACK: i32 = 3;

/// The two programs that hold an area best, with their points: the map's
/// owner and, when the second is close, why it is contested (GR8).
fn top_two(w: &World, schools: &[usize], talent: &[f32]) -> Vec<(usize, f32)> {
    let mut all: Vec<(usize, f32)> = (0..w.programs.len())
        .map(|p| (p, standing::area_points(w, p, schools, talent)))
        .filter(|x| x.1 > 0.0)
        .collect();
    all.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    all.truncate(2);
    all
}

fn pin_json(w: &World, p: &Pin) -> Value {
    let (lat, lon) = areas::school_place(w, p.school);
    json!({
        "name": p.name,
        "pos": p.position.name(),
        "stars": p.stars,
        "class": p.class_of,
        "what": p.what,
        "school": p.school,
        "school_name": w.school_name(p.school),
        "at": [lat, lon],
        "player": p.player,
    })
}

fn map(w: &World, args: &Value) -> Reply {
    let prog = seat(w)?;
    let home = w.programs[prog].state;
    let st = args.get("state").and_then(Value::as_str).and_then(geo::state_index).unwrap_or(home);
    let code = geo::STATE_CENTERS[st].0;
    let talent = standing::talents(w);
    let by_area = areas::schools_by_area(w);
    let pins = areas::pins(w, Some(prog));
    let words = areas::characters(w);
    let you = w.career.as_ref().map(|c| c.coach as u32);
    let me = Holder::Program(prog as u32);

    let mut out = Vec::new();
    for &id in areas::in_state(st) {
        let a = areas::area(id);
        let schools = &by_area[id as usize];
        let heads = heads(w, schools);
        let mut counts = [0usize; 4];
        let mut first_met: Option<Date> = None;
        for &(_, who) in &heads {
            let (pts, holder) = standing::staff(w, prog, who);
            counts[Level::of(pts) as usize] += 1;
            if pts >= 25.0
                && let Some(m) = w.ground.get(who, holder)
                && first_met.is_none_or(|d| m.met < d)
            {
                first_met = Some(m.met);
            }
        }
        let known: usize = counts[1] + counts[2] + counts[3];
        let mine_pts = standing::area_points(w, prog, schools, &talent);
        let top = top_two(w, schools, &talent);
        let leader = top.first().map(|&(p, pts)| {
            let [c1, c2] = team_colors(w, TeamRef::College(p));
            json!({
                "id": p, "name": w.program_name(p), "colors": [c1, c2], "yours": p == prog,
                "level": level_json(pts),
                "contested": top.get(1).is_some_and(|s| s.1 >= pts - CONTESTED && s.1 > 0.0),
                "second": top.get(1).map(|s| json!({ "name": w.program_name(s.0), "points": s.1.round() })),
            })
        });
        let fog = known == 0;
        let ch = &words[id as usize];
        // The area's own legends and pipelines for the map (GR6).
        let pin = &pins[id as usize];
        let recent = |p: &&Pin| p.class_of >= w.dates.season - SIGNED_BACK;
        let mut sigs: Vec<Value> = pin.signees.iter().filter(recent).take(24).map(|p| pin_json(w, p)).collect();
        sigs.sort_by(|a, b| b["class"].as_i64().cmp(&a["class"].as_i64()));
        out.push(json!({
            "code": areas::code(id),
            "name": a.name,
            "nickname": a.nickname,
            "codes": a.codes.join(", "),
            "label": [a.label.0, a.label.1],
            "schools": schools.len(),
            "people": heads.len(),
            "fog": fog,
            "leader": leader,
            "you": { "level": level_json(mine_pts), "known": known, "of": heads.len(), "since": first_met.map(short_date) },
            "public": ch.public,
            "words": if fog { Vec::new() } else { ch.words.clone() },
            "signees": sigs,
            "legends": pin.legends.iter().map(|p| pin_json(w, p)).collect::<Vec<_>>(),
            "pipelines": pin.pipelines.iter().filter(|p| p.program != prog).map(|p| {
                let [c1, _] = team_colors(w, TeamRef::College(p.program));
                json!({ "program": p.program, "name": w.program_name(p.program), "color": c1, "strength": p.strength, "at": [p.campus.0, p.campus.1] })
            }).collect::<Vec<_>>(),
        }));
    }

    // The area picked: the one asked for, else the one you hold best.
    let picked = args
        .get("area")
        .and_then(Value::as_str)
        .filter(|c| out.iter().any(|a| a["code"] == *c))
        .map(str::to_string)
        .or_else(|| {
            out.iter()
                .max_by(|a, b| {
                    a["you"]["level"]["points"].as_f64().partial_cmp(&b["you"]["level"]["points"].as_f64()).unwrap()
                })
                .and_then(|a| a["code"].as_str().map(str::to_string))
        });
    let detail = picked
        .as_deref()
        .and_then(areas::find)
        .map(|id| area_detail(w, prog, id, &talent, &by_area[id as usize], &pins));

    // Overnight: the moments people keep of you since last week.
    let since = w.today.add(-7);
    let mut lately: Vec<(Date, Value)> = Vec::new();
    for m in &w.ground.memories {
        let mine = m.holder == me || matches!(m.holder, Holder::Coach(c) if Some(c) == you);
        if !mine {
            continue;
        }
        for mo in m.moments.iter().filter(|x| x.date > since && x.date <= w.today) {
            let school = people::school(w, m.who);
            lately.push((
                mo.date,
                json!({
                    "who": who_json(m.who), "name": people::name(w, m.who), "role": people::role(w, m.who),
                    "words": mo.words, "good": mo.good, "date": short_date(mo.date),
                    "school": school, "area": school.map(|h| areas::code(w.high_schools[h].ground_area)),
                }),
            ));
        }
    }
    lately.sort_by_key(|a| std::cmp::Reverse(a.0));
    lately.truncate(6);

    // The states you can look at: every state, with how many areas it has.
    let states: Vec<Value> = geo::STATE_CENTERS
        .iter()
        .enumerate()
        .filter(|(i, _)| !areas::in_state(*i).is_empty())
        .map(|(i, s)| json!({ "code": s.0, "name": state_name(s.0), "areas": areas::in_state(i).len() }))
        .collect();

    Ok(json!({
        "team": team_brief(w, TeamRef::College(prog)),
        "coach": w.career.as_ref().map(|c| c.you.name.clone()),
        "today": long_date(w.today),
        "season": w.dates.season,
        "period": period_json(w),
        "state": { "code": code, "name": state_name(code), "yours": st == home },
        "states": states,
        "areas": out,
        "picked": picked,
        "detail": detail,
        "lately": lately.into_iter().map(|x| x.1).collect::<Vec<_>>(),
        "since": short_date(since),
        "pending": ["Your men's routes and road days", "Your week's hours", "Friday nights"],
    }))
}

/// One area in full (GR70's right side).
fn area_detail(w: &World, prog: usize, id: u16, talent: &[f32], schools: &[usize], pins: &[areas::Pins]) -> Value {
    let a = areas::area(id);
    let heads = heads(w, schools);
    let mut counts = [0usize; 4];
    let mut rows: Vec<(f32, usize, Who, f32, Holder)> = Vec::new();
    for &(h, who) in &heads {
        let (pts, holder) = standing::staff(w, prog, who);
        counts[Level::of(pts) as usize] += 1;
        rows.push((talent.get(h).copied().unwrap_or(1.0), h, who, pts, holder));
    }
    let known: usize = counts[1] + counts[2] + counts[3];
    let fog = known == 0;
    // Who leads it, and with how many of its people.
    let top = top_two(w, schools, talent).into_iter().next();
    let lead = top.map(|(p, pts)| {
        let mut c = [0usize; 4];
        for &(_, who) in &heads {
            c[standing::level(w, p, who) as usize] += 1;
        }
        let coach = w.programs[p].head_coach.and_then(|k| w.coaches.get(k));
        json!({
            "id": p, "name": w.program_name(p), "yours": p == prog, "level": level_json(pts),
            "line": level_words(c),
            "coach": coach.map(|k| json!({ "name": k.name, "years": k.job.as_ref().map_or(0, |j| (w.year - j.since).max(0)) })),
        })
    });
    // The county line: how many it holds and how many are over a state line.
    let over = a.counties.iter().filter(|c| c.contains(':')).count();
    let st = geo::STATE_CENTERS[a.state].0;
    let counties = format!(
        "{} {} {}{}",
        a.counties.len() - over,
        state_name(st),
        if a.counties.len() - over == 1 { "county" } else { "counties" },
        if over > 0 { format!(", {over} over the line") } else { String::new() }
    );
    // Its people by name, the schools holding the most talent first; hidden
    // while the area is fogged.
    rows.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let people: Vec<Value> = if fog {
        Vec::new()
    } else {
        rows.iter()
            .take(7)
            .map(|&(_, h, who, pts, _)| {
                json!({
                    "who": who_json(who), "name": people::name(w, who), "role": people::role(w, who),
                    "school": h, "level": level_json(pts),
                })
            })
            .collect()
    };
    // The public view shows everywhere: the biggest schools by talent.
    let public: Vec<Value> = {
        let mut s: Vec<usize> = schools.to_vec();
        s.sort_by(|&x, &y| {
            talent.get(y).copied().unwrap_or(1.0).total_cmp(&talent.get(x).copied().unwrap_or(1.0)).then(x.cmp(&y))
        });
        s.iter()
            .take(6)
            .map(|&h| {
                let hs = &w.high_schools[h];
                json!({ "school": h, "name": w.school_name(h), "town": hs.town, "enrollment": hs.enrollment })
            })
            .collect()
    };
    let signed = pins[id as usize].signees.iter().filter(|p| p.class_of >= w.dates.season - SIGNED_BACK).count();
    let trip = areas::trip_to_area(w, prog, id);
    json!({
        "code": areas::code(id),
        "name": a.name,
        "nickname": a.nickname,
        "codes": a.codes.join(", "),
        "counties": counties,
        "fog": fog,
        "lead": lead,
        "people": people,
        "public": public,
        "schools": schools.len(),
        "heads": heads.len(),
        "signed": signed,
        "trip": trip.words(),
        "flight": trip.flight(),
        "a_day": areas::schools_a_day(id),
    })
}

fn kind_of(who: Who) -> &'static str {
    match who {
        Who::Coach(_) => "coach",
        _ => "person",
    }
}

/// A school's own page (GR11).
fn school(w: &World, args: &Value) -> Reply {
    let prog = seat(w)?;
    let id = args.get("id").and_then(Value::as_u64).map(|v| v as usize).ok_or("Missing id.")?;
    let hs = w.high_schools.get(id).ok_or("No such school.")?;
    let state = geo::STATE_CENTERS[hs.state.min(50)].0;
    let area = areas::area(hs.ground_area);
    let [c1, c2] = crate::colors::high_school(id);
    let here = people::at_school(w, id);
    // The people at the school, and what each remembers (only once Known).
    let mut folks = Vec::new();
    let mut known = false;
    for who in &here {
        let (pts, holder) = standing::staff(w, prog, *who);
        let lv = Level::of(pts);
        known |= lv >= Level::Known;
        let role = match who {
            Who::Coach(_) => people::role(w, *who).split(" at ").next().unwrap_or("coach").to_string(),
            _ => "counselor".to_string(),
        };
        let m = w.ground.get(*who, holder);
        folks.push(json!({
            "who": who_json(*who), "kind": kind_of(*who),
            "name": if matches!(who, Who::Counselor(_)) { "Counselor".to_string() } else { people::name(w, *who) },
            "role": if matches!(who, Who::Counselor(_)) { "guidance office".to_string() } else { role }, "level": level_json(pts),
            "why": if lv >= Level::Known { Some(standing::why(w, prog, *who)) } else { None },
            "moment": m.and_then(|m| m.moments.last()).filter(|_| lv >= Level::Known).map(|x| json!({ "words": x.words, "date": short_date(x.date), "good": x.good })),
            "head": matches!(who, Who::Coach(c) if hs.coach == Some(*c as usize)),
        }));
    }
    // Where its kids went: first colleges of those past high school.
    let mut went: std::collections::BTreeMap<usize, (u32, u32)> = Default::default();
    let mut drafted = 0u32;
    for a in w.alumni.iter().filter(|a| a.high_school == id) {
        if let Some(&c) = a.colleges.first() {
            let e = went.entry(c).or_default();
            e.0 += 1;
            if a.pick.is_some() {
                e.1 += 1;
                drafted += 1;
            }
        }
    }
    for p in w.people.iter().filter(|p| p.hs == id && p.stage != Stage::HighSchool) {
        if let Some(&c) = p.path.first() {
            went.entry(c).or_default().0 += 1;
        }
    }
    let mut went: Vec<(usize, (u32, u32))> = went.into_iter().collect();
    went.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(&b.0)));
    let kids: u32 = went.iter().map(|x| x.1.0).sum();
    let went: Vec<Value> = went
        .iter()
        .take(8)
        .map(|&(p, (n, d))| {
            let [pc, _] = team_colors(w, TeamRef::College(p));
            json!({ "program": p, "name": w.program_name(p), "color": pc, "kids": n, "drafted": d, "yours": p == prog })
        })
        .collect();
    // Seasons: the record on file, the last five.
    let seasons: Vec<Value> = hs
        .seasons
        .iter()
        .rev()
        .take(6)
        .map(|s| json!({ "year": s.year, "w": s.wins, "l": s.losses, "title": s.title, "state_title": s.state_title }))
        .collect();
    let head_since = hs.coach.map(|c| {
        let n = hs.seasons.iter().rev().take_while(|s| s.coach == Some(c)).count();
        hs.seasons.last().map_or(w.year, |s| s.year) - n as i32 + 1
    });
    let pins = areas::pins(w, Some(prog)).swap_remove(hs.ground_area as usize);
    let at_school = |p: &&Pin| p.school == id;
    Ok(json!({
        "team": team_brief(w, TeamRef::College(prog)),
        "school": {
            "id": id, "name": w.school_name(id), "town": hs.town, "state": state, "state_name": state_name(state),
            "colors": [c1, c2], "private": hs.private, "enrollment": hs.enrollment, "state_titles": hs.state_titles,
            "big": hs.classification == 0, "powerhouse": hs.is_powerhouse(),
        },
        "area": { "code": areas::code(hs.ground_area), "name": area.name, "nickname": area.nickname },
        "trip": areas::trip_to_school(w, prog, id).words(),
        "known": known,
        "folks": folks,
        "went": went,
        "kids": kids,
        "drafted": drafted,
        "seasons": seasons,
        "head_since": head_since,
        "first_season": hs.seasons.first().map(|s| s.year),
        "signees": pins.signees.iter().filter(at_school).map(|p| pin_json(w, p)).collect::<Vec<_>>(),
        "legends": pins.legends.iter().filter(at_school).map(|p| pin_json(w, p)).collect::<Vec<_>>(),
        "role": Role::HeadCoach.name(),
    }))
}
