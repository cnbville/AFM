//! The race (Ground lane G9): the commands behind the rival screen (board
//! 6, lane S6) and the race's signs on `afm_engine::world::ground::race`.
//!
//! - `ground_rivals`: the rival men working your areas, most ground first:
//!   face, program, role, style, the areas you share with where he stands
//!   there in a word, his Home schools, his career and what's between you.
//! - `ground_rival` {man}: one rival man in full, with the visits your
//!   people saw.
//! - `ground_sheet` {school}: a school's sign-in sheet, the last six weeks.
//! - `ground_told` {id}: what a kid's coach tells you about the other
//!   staffs, by your standing with him.
//! - `ground_race`: your wavering commits (signs, your call, how it
//!   landed) and the claims made both ways with how they settled.
//! - `ground_fight` {id}: your call to a wavering commit's family.
//! - `ground_claim` {id, about, topic: job|depth|scheme}: your claim about a
//!   rival to a kid's family; it settles on the facts.

use afm_engine::world::World;
use afm_engine::world::ground::people::{self, Who};
use afm_engine::world::ground::race::{self, Rival, Topic};
use serde_json::{Value, json};

use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let num = |k: &str| args.get(k).and_then(Value::as_u64);
    Some(match cmd {
        "ground_rivals" => {
            game.w().map(|w| json!({ "rivals": race::rivals(w).iter().map(|r| rival_json(w, r)).collect::<Vec<_>>() }))
        }
        "ground_rival" => game.w().and_then(|w| {
            let man = num("man").ok_or("Missing the man.")? as u32;
            race::rival(w, man).map(|r| rival_json(w, &r)).ok_or_else(|| "He isn't a rival recruiter.".to_string())
        }),
        "ground_sheet" => game.w().and_then(|w| {
            let h = num("school").ok_or("Missing the school.")? as usize;
            if h >= w.high_schools.len() {
                return Err("No such school.".into());
            }
            let d = w.today;
            let v = race::sheet(w, h, d.add(-race::SHEET_DAYS), d);
            Ok(json!({
                "school": w.school_name(h),
                "visits": v.iter().map(|x| json!({
                    "program": x.program, "school": x.school, "man": x.man, "name": x.name, "date": x.date.to_string(),
                })).collect::<Vec<_>>(),
            }))
        }),
        "ground_told" => game.w().and_then(|w| {
            let id = num("id").ok_or("Missing the recruit.")? as u32;
            Ok(json!({ "id": id, "lines": race::told(w, id) }))
        }),
        "ground_race" => game.w().map(race_json),
        "ground_fight" => game.wm().and_then(|w| {
            let id = num("id").ok_or("Missing the recruit.")? as u32;
            let said = race::fight(w, id)?;
            let mut v = race_json(w);
            v["said"] = json!(said);
            Ok(v)
        }),
        "ground_claim" => game.wm().and_then(|w| {
            let id = num("id").ok_or("Missing the recruit.")? as u32;
            let about = num("about").ok_or("Missing the program.")? as u32;
            let topic = args
                .get("topic")
                .and_then(Value::as_str)
                .and_then(Topic::from_key)
                .ok_or("Pick job, depth or scheme.")?;
            let said = race::claim(w, id, about, topic)?;
            let mut v = race_json(w);
            v["said"] = json!(said);
            Ok(v)
        }),
        _ => return None,
    })
}

fn face(w: &World, c: u32) -> Value {
    let who = Who::Coach(c);
    json!({ "looks": people::looks(w, who).0, "heritage": format!("{:?}", people::heritage(w, who)) })
}

fn rival_json(w: &World, r: &Rival) -> Value {
    json!({
        "man": r.man,
        "name": r.name,
        "face": face(w, r.man),
        "program": r.program,
        "school": r.school,
        "role": r.role,
        "style": r.style,
        "style_line": r.style_line,
        "areas": r.areas.iter().map(|a| json!({ "area": a.0, "nickname": a.1, "level": a.2 })).collect::<Vec<_>>(),
        "home": r.home,
        "career": r.career,
        "history": r.history,
        "seen": r.seen,
    })
}

fn name_of(w: &World, id: u32) -> String {
    afm_engine::world::scouting::place(w, id).map_or_else(String::new, |i| w.people[i].name.clone())
}

fn race_json(w: &World) -> Value {
    let r = &w.ground.race;
    json!({
        "wavers": r.wavers.iter().rev().map(|x| json!({
            "id": x.id,
            "name": name_of(w, x.id),
            "rival": x.rival,
            "rival_school": w.programs.get(x.rival as usize).map(|p| p.real_name.clone()),
            "date": x.date.to_string(),
            "signs": x.signs,
            "fought": x.fought.is_some(),
            "line": x.line,
        })).collect::<Vec<_>>(),
        "claims": r.claims.iter().rev().map(|c| json!({
            "id": c.kid,
            "name": name_of(w, c.kid),
            "by": w.programs.get(c.by as usize).map(|p| p.real_name.clone()),
            "yours": c.man.is_none(),
            "man": c.man,
            "about": w.programs.get(c.about as usize).map(|p| p.real_name.clone()),
            "topic": c.topic.key(),
            "words": c.words,
            "said": c.said.to_string(),
            "settles": c.settles.to_string(),
            "held": c.held,
        })).collect::<Vec<_>>(),
    })
}
