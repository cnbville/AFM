//! Ground's people (lane G7): the commands the person screen (lane S5)
//! reads. A person's page in words (`ground::card`): who he is, his face,
//! what he wants, his ties, your standing and its why, the moments he
//! keeps, his kids with you, your word and his, favors either way. Never a
//! hidden number.

use afm_engine::world::World;
use afm_engine::world::ground::{Who, card, people, standing};
use serde_json::{Value, json};

use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        "ground_person" => game.w().and_then(|w| person(w, args)),
        "ground_family" => game.w().and_then(|w| family(w, args)),
        "ground_school_people" => game.w().and_then(|w| school(w, args)),
        _ => return None,
    })
}

fn prog(w: &World) -> Result<usize, String> {
    standing::seat_program(w).ok_or_else(|| "Ground's people are for a college staff.".to_string())
}

/// The person an argument names: `kind` (coach, trainer, seven, counselor,
/// family, prep) and `id`.
pub fn who(args: &Value) -> Result<Who, String> {
    let id = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as u32;
    Ok(match args.get("kind").and_then(Value::as_str).unwrap_or("coach") {
        "coach" => Who::Coach(id),
        "trainer" => Who::Trainer(id),
        "seven" => Who::SevenOnSeven(id),
        "counselor" => Who::Counselor(id),
        "family" => Who::Family(id),
        "prep" => Who::Prep(id as u16),
        k => return Err(format!("No such kind of person: {k}.")),
    })
}

/// A person's kind and id as the commands take them.
pub fn key(who: Who) -> Value {
    let (k, id) = match who {
        Who::Coach(c) => ("coach", c),
        Who::Trainer(f) => ("trainer", f),
        Who::SevenOnSeven(f) => ("seven", f),
        Who::Counselor(h) => ("counselor", h),
        Who::Family(p) => ("family", p),
        Who::Prep(k) => ("prep", k as u32),
    };
    json!({ "kind": k, "id": id })
}

fn page(w: &World, prog: usize, who: Who) -> Value {
    let c = card::card(w, prog, who);
    let mut v = serde_json::to_value(&c).unwrap_or(Value::Null);
    v["key"] = key(who);
    if let Some(ties) = v.get_mut("ties").and_then(Value::as_array_mut) {
        for (t, x) in c.ties.iter().zip(ties.iter_mut()) {
            x["key"] = key(t.who);
        }
    }
    v
}

fn person(w: &World, args: &Value) -> Reply {
    let prog = prog(w)?;
    Ok(page(w, prog, who(args)?))
}

/// A recruit's family, by the recruit's player id.
fn family(w: &World, args: &Value) -> Reply {
    let prog = prog(w)?;
    let id = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as u32;
    let i = afm_engine::world::folk::person(w, id).ok_or("No such player.")?;
    Ok(page(w, prog, people::family_of(w, i)))
}

/// Everyone around a high school: its coaches and counselor, then the
/// families, trainers and 7v7 men around its best kids.
fn school(w: &World, args: &Value) -> Reply {
    let prog = prog(w)?;
    let h = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as usize;
    if h >= w.high_schools.len() {
        return Err("No such school.".into());
    }
    let mut list: Vec<Who> = people::at_school(w, h);
    for i in people::kids_at(w, h, 6) {
        list.push(people::family_of(w, i));
        list.extend(people::trainer_of(w, i));
        list.extend(people::seven_of(w, i));
    }
    let mut seen: Vec<Who> = Vec::new();
    let rows: Vec<Value> = list
        .into_iter()
        .filter(|x| {
            let new = !seen.contains(x);
            seen.push(*x);
            new
        })
        .map(|x| {
            json!({
                "key": key(x),
                "name": people::name(w, x),
                "role": people::role(w, x),
                "kind": people::kind(w, x).words(),
                "level": standing::level(w, prog, x).name(),
                "why": standing::why(w, prog, x),
            })
        })
        .collect();
    Ok(json!({ "school": w.school_name(h), "people": rows }))
}
