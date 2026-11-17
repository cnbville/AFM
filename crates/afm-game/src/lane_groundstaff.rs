//! Your staff's ground and the assistant seat (Ground lane G10): the
//! commands behind the staff carousel and the assistant's desk (boards 7b
//! and 9, lane S7) on `afm_engine::world::ground::staff`.
//!
//! - `ground_staff`: the rail of your men (face, job, areas with their
//!   level, mood, style, record, road days, who's after him), the
//!   coordinator's proposal and the areas left empty, rides, the men who
//!   left, the misses.
//! - `ground_staff_man` {man}: one man's ground in full, with his signs.
//! - `ground_staff_areas` {man, areas ["GA-404", ...]}: you set his areas.
//! - `ground_staff_ride` {young, veteran, weeks}, `ground_staff_ride_end`
//!   {young}.
//! - `ground_carousel` {man?}: the warning for a man another school wants
//!   (the first one waiting when no man is named): the offer and the
//!   timeline, what pulls him each way, his line, your counters with how far
//!   each reaches and its side effect, who could take his area, what leaves
//!   with him on the map, the commits who'd waver, and the rail.
//! - `ground_counter` {man, kind: raise|title|promise}, `ground_let_go`
//!   {man}, `ground_waver_call` {id}.
//! - `ground_signs` {coach}: a candidate's ground in signs.
//! - `ground_desk`: the assistant's desk; `ground_push` {id, case},
//!   `ground_push_back` {id}, `ground_let_ride` {id}, `ground_desk_goal`
//!   {k}, `ground_route` {schools [ids]}.

use afm_engine::world::World;
use afm_engine::world::coach::Role;
use afm_engine::world::folk::staffing;
use afm_engine::world::ground::people::{self, Holder, Who};
use afm_engine::world::ground::staff::{self, beats, carousel, desk, men};
use afm_engine::world::ground::{areas, standing};
use serde_json::{Value, json};

use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let man = |k: &str| args.get(k).and_then(Value::as_u64).map(|x| x as usize);
    Some(match cmd {
        "ground_staff" => game.wm().and_then(|w| {
            staff::ready(w);
            staff_view(w)
        }),
        "ground_staff_man" => game.w().and_then(|w| {
            let c = man("man").ok_or("Missing the man.")?;
            man_view(w, c)
        }),
        "ground_staff_areas" => game.wm().and_then(|w| {
            let c = man("man").ok_or("Missing the man.")?;
            let list: Vec<u16> = args
                .get("areas")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(area_arg).collect())
                .unwrap_or_default();
            let said = beats::set_areas(w, c, &list)?;
            said_with(staff_view(w)?, said)
        }),
        "ground_staff_ride" => game.wm().and_then(|w| {
            let y = man("young").ok_or("Missing the young man.")?;
            let v = man("veteran").ok_or("Missing the veteran.")?;
            let weeks = args.get("weeks").and_then(Value::as_u64).unwrap_or(4) as u32;
            let said = beats::ride(w, y, v, weeks)?;
            said_with(staff_view(w)?, said)
        }),
        "ground_staff_ride_end" => game.wm().and_then(|w| {
            let y = man("young").ok_or("Missing the young man.")?;
            let said = beats::end_ride(w, y)?;
            said_with(staff_view(w)?, said)
        }),
        "ground_carousel" => game.w().and_then(|w| carousel_view(w, man("man"))),
        "ground_counter" => game.wm().and_then(|w| {
            let c = man("man").ok_or("Missing the man.")?;
            let kind = args
                .get("kind")
                .and_then(Value::as_str)
                .and_then(carousel::Counter::from_key)
                .ok_or("A raise, a title or a promise.")?;
            let said = carousel::counter(w, c, kind)?;
            said_with(staff_view(w)?, said)
        }),
        "ground_let_go" => game.wm().and_then(|w| {
            let c = man("man").ok_or("Missing the man.")?;
            let said = carousel::let_go(w, c)?;
            said_with(staff_view(w)?, said)
        }),
        "ground_waver_call" => game.wm().and_then(|w| {
            let id = args.get("id").and_then(Value::as_u64).ok_or("Missing the kid.")? as u32;
            let said = carousel::call_commit(w, id)?;
            Ok(json!({ "said": said }))
        }),
        "ground_signs" => game.w().and_then(|w| {
            let c = man("coach").ok_or("Missing the coach.")?;
            let prog = staff::seat_program(w).ok_or("Signs are for a college seat.")?;
            if c >= w.coaches.len() {
                return Err("No such coach.".into());
            }
            Ok(json!({ "coach": c, "name": w.coaches[c].name, "signs": men::signs(w, prog, c) }))
        }),
        "ground_desk" => game.w().and_then(desk_view),
        "ground_push" => game.wm().and_then(|w| {
            let id = args.get("id").and_then(Value::as_u64).ok_or("Missing the kid.")? as u32;
            let case = args.get("case").and_then(Value::as_str).unwrap_or("");
            let said = desk::push(w, id, case)?;
            said_with(desk_view(w)?, said)
        }),
        "ground_push_back" => game.wm().and_then(|w| {
            let id = args.get("id").and_then(Value::as_u64).ok_or("Missing the kid.")? as u32;
            let said = desk::push_back(w, id)?;
            said_with(desk_view(w)?, said)
        }),
        "ground_let_ride" => game.wm().and_then(|w| {
            let id = args.get("id").and_then(Value::as_u64).ok_or("Missing the kid.")? as u32;
            let said = desk::let_ride(w, id)?;
            said_with(desk_view(w)?, said)
        }),
        "ground_desk_goal" => game.wm().and_then(|w| {
            let k = man("k").ok_or("Pick a goal.")?;
            let said = desk::pick_goal(w, k)?;
            said_with(desk_view(w)?, said)
        }),
        "ground_route" => game.wm().and_then(|w| {
            let schools: Vec<u32> = args
                .get("schools")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_u64).map(|x| x as u32).collect())
                .unwrap_or_default();
            let said = desk::route(w, &schools)?;
            said_with(desk_view(w)?, said)
        }),
        _ => return None,
    })
}

const TERMS: [&str; 4] = ["good terms", "grateful", "angry", "gone"];
const STATES: [&str; 3] = ["waiting", "stays", "goes"];

fn said_with(mut v: Value, said: String) -> Reply {
    v["said"] = json!(said);
    Ok(v)
}

fn area_arg(v: &Value) -> Option<u16> {
    if let Some(n) = v.as_u64() {
        return (n < areas::table().len() as u64).then_some(n as u16);
    }
    areas::find(v.as_str()?)
}

fn face(w: &World, c: usize) -> Value {
    let who = Who::Coach(c as u32);
    json!({ "looks": people::looks(w, who).0, "heritage": format!("{:?}", people::heritage(w, who)) })
}

/// One man on the rail (GR76: his ground, mood and record).
fn rail_row(w: &World, prog: usize, c: usize) -> Value {
    let Some(g) = men::ground(w, prog, c) else { return Value::Null };
    json!({
        "man": c,
        "name": g.name,
        "role": g.role,
        "age": g.age,
        "face": face(w, c),
        "areas": g.areas.iter().map(|a| json!({
            "area": a.area, "code": a.code, "nickname": a.nickname, "level": a.level,
            "known": a.known, "trusted": a.trusted, "home": a.home, "heads": a.heads,
            "why": a.why, "yours": a.yours, "with": a.with,
        })).collect::<Vec<_>>(),
        "levels": { "known": g.levels.0, "trusted": g.levels.1, "home": g.levels.2 },
        "mood": g.mood.words,
        "mood_reasons": g.mood.reasons,
        "mood_lifts": g.mood.lifts,
        "style": g.style,
        "record": { "offered": g.record.0, "passed": g.record.1, "right": g.record.2, "overruled": g.record.3 },
        "credited": g.credited.len(),
        "days": { "spring": g.days.0, "fall": g.days.1, "week": g.days.2 },
        "ride": g.ride.map(|r| json!({ "with": r.0, "young": r.1 })),
        "courted": g.courted,
    })
}

fn staff_view(w: &World) -> Reply {
    let prog = staff::seat_program(w).ok_or("Your staff's ground is for a college seat.")?;
    let s = &w.ground.staff;
    let rail: Vec<Value> = staff::men(w, prog).into_iter().map(|c| rail_row(w, prog, c)).collect();
    let empty: Vec<Value> = s
        .proposal
        .empty
        .iter()
        .map(|&a| json!({ "area": a, "code": areas::code(a), "nickname": areas::area(a).nickname }))
        .collect();
    let left: Vec<Value> = s
        .left
        .iter()
        .rev()
        .take(8)
        .map(|l| {
            json!({
                "man": l.man,
                "name": w.coaches.get(l.man as usize).map(|c| c.name.clone()),
                "date": l.date.to_string(),
                "terms": TERMS[l.terms.min(3) as usize],
                "why": l.why,
            })
        })
        .collect();
    let misses: Vec<Value> = s
        .misses
        .iter()
        .rev()
        .take(10)
        .filter_map(|m| {
            let i = afm_engine::world::scouting::place(w, m.id)?;
            Some(json!({
                "id": m.id,
                "name": w.people[i].name,
                "stars": m.stars,
                "area": areas::area(m.area).nickname,
                "to": w.program_name(m.to as usize),
            }))
        })
        .collect();
    let rides: Vec<Value> = s
        .rides
        .iter()
        .map(|r| json!({ "young": r.young, "veteran": r.veteran, "to": r.to.to_string(), "days": r.days }))
        .collect();
    let waiting: Vec<u32> = s.courted.iter().filter(|x| x.state == 0).map(|x| x.man).collect();
    Ok(json!({
        "head": staff::you_are_head(w),
        "men": rail,
        "proposal": { "date": s.proposal.date.map(|d| d.to_string()), "letter": s.proposal.letter },
        "empty": empty,
        "rides": rides,
        "left": left,
        "misses": misses,
        "courted": waiting,
        "february_meeting": staff::february_meeting(w, w.today).to_string(),
    }))
}

fn man_view(w: &World, c: usize) -> Reply {
    let prog = staff::seat_program(w).ok_or("Your staff's ground is for a college seat.")?;
    let g = men::ground(w, prog, c).ok_or("No such coach.")?;
    let mut v = serde_json::to_value(&g).map_err(|e| e.to_string())?;
    v["face"] = face(w, c);
    v["signs"] = json!(men::signs(w, prog, c));
    v["style_why"] = json!(g.style_why);
    Ok(v)
}

/// The carousel screen (GR76, board 7b).
fn carousel_view(w: &World, man: Option<usize>) -> Reply {
    let prog = staff::seat_program(w).ok_or("Your staff's ground is for a college seat.")?;
    let s = &w.ground.staff;
    let x = match man {
        Some(c) => s.courted.iter().rev().find(|x| x.man == c as u32),
        None => s.courted.iter().find(|x| x.state == 0),
    };
    let rail: Vec<Value> = staff::men(w, prog).into_iter().map(|c| rail_row(w, prog, c)).collect();
    let Some(x) = x else {
        return Ok(json!({ "courted": Value::Null, "rail": rail }));
    };
    let c = x.man as usize;
    let co = &w.coaches[c];
    let mood = men::mood(w, prog, c);
    let today = w.today;
    let es = w.dates.early_signing.filter(|&e| e >= today);
    // What pulls him each way.
    let mut away: Vec<String> = x.why.split(" and ").map(str::to_string).collect();
    away.extend(mood.reasons.iter().cloned());
    let stay = carousel::keeps_him(w, prog, c);
    let counters: Vec<Value> = [carousel::Counter::Raise, carousel::Counter::Title, carousel::Counter::Promise]
        .iter()
        .map(|&k| {
            let r = carousel::reach(w, c, k);
            json!({
                "kind": k.key(),
                "reach": r.map(carousel::reach_words),
                "side_effect": carousel::side_effect(w, c, k),
            })
        })
        .collect();
    let leaves: Vec<Value> = men::what_leaves(w, prog, c)
        .into_iter()
        .map(|(a, now, after)| {
            json!({ "area": a, "code": areas::code(a), "nickname": areas::area(a).nickname, "now": now, "if_he_goes": after })
        })
        .collect();
    // The commits who'd waver: committed kids from his areas.
    let wavers: Vec<Value> = w
        .people
        .iter()
        .filter(|q| q.cal.signed.is_none() && q.cal.commit.is_some_and(|k| k.0 == prog))
        .filter(|q| beats::recruiter_for_school(w, prog, q.hs) == Some(c))
        .take(12)
        .map(|q| json!({ "id": q.player.id, "name": q.name, "stars": q.stars, "pos": q.player.position.to_string() }))
        .collect();
    let shaken: Vec<Value> = s
        .wavers
        .iter()
        .filter(|v| v.man == x.man)
        .map(|v| json!({ "id": v.id, "until": v.until.to_string(), "called": v.called, "line": v.line }))
        .collect();
    Ok(json!({
        "courted": {
            "man": c,
            "name": co.name,
            "role": afm_engine::world::folk::facts::coach_role(w, c),
            "face": face(w, c),
            "to": men::employer_name(w, x.employer),
            "job": x.role.name(),
            "why": x.why,
            "made": x.made.to_string(),
            "until": x.until.to_string(),
            "days_to_early_signing": es.map(|e| today.days_to(e)),
            "state": STATES[x.state.min(2) as usize],
            "line": x.line,
            "pulls_away": away,
            "pulls_stay": stay,
            "mood": mood.words,
            "read": men::style(w, c).1,
            "counters": counters,
            "counter": x.counter,
        },
        "takers": takers(w, prog, c),
        "leaves": leaves,
        "wavers": wavers,
        "shaken": shaken,
        "rail": rail,
    }))
}

/// Who could take his area if he goes (GR76): candidates for his job on
/// teaching, ground and price, and one the rules bar.
fn takers(w: &World, prog: usize, c: usize) -> Vec<Value> {
    let role = w.coaches[c].job.map_or(Role::PositionCoach(w.coaches[c].came_up), |j| j.role);
    let list = staffing::candidates(w, role, &staffing::Filter::default()).list;
    let by_area = areas::schools_by_area(w);
    let talent = standing::talents(w);
    let his: Vec<u16> = beats::his_areas(w, prog, c).unwrap_or_default();
    let mut scored: Vec<(f32, &staffing::Candidate)> = list
        .iter()
        .map(|k| {
            let g: f32 = his
                .iter()
                .map(|&a| beats::holder_points(w, Holder::Coach(k.coach as u32), &by_area[a as usize], &talent))
                .sum();
            (g, k)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.coach.cmp(&b.1.coach)));
    let mut out: Vec<Value> = Vec::new();
    let mut add = |g: f32, k: &staffing::Candidate, tag: &str, w: &World| {
        if out.iter().any(|v| v["coach"] == json!(k.coach)) {
            return;
        }
        let barred = afm_engine::world::folk::backchannel::hs_hire(w, k.coach).err();
        out.push(json!({
            "coach": k.coach,
            "name": k.name,
            "now": k.now,
            "teaching": k.read,
            "ground": afm_engine::world::ground::Level::of(g / his.len().max(1) as f32).name(),
            "asks": k.asks,
            "signs": men::signs(w, prog, k.coach),
            "tag": tag,
            "barred": barred,
        }));
    };
    for (g, k) in scored.iter().take(3) {
        add(*g, k, "ground", w);
    }
    if let Some((g, k)) = scored
        .iter()
        .max_by(|a, b| (a.1.read.0 as u16 + a.1.read.1 as u16).cmp(&(b.1.read.0 as u16 + b.1.read.1 as u16)))
    {
        add(*g, k, "teaching", w);
    }
    if let Some((g, k)) = scored.iter().find(|x| afm_engine::world::folk::backchannel::hs_hire(w, x.1.coach).is_err()) {
        add(*g, k, "barred", w);
    }
    out
}

fn desk_view(w: &World) -> Reply {
    let v = desk::view(w).ok_or("The desk is the college assistant's.")?;
    let mut out = serde_json::to_value(&v).map_err(|e| e.to_string())?;
    out["face"] = face(w, v.boss_id);
    out["memory_level"] = json!(v.level);
    Ok(out)
}
