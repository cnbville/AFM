//! The race (Ground lane G9): the commands behind the rival screen (board
//! 6, lane S6) and the race's signs on `afm_engine::world::ground::race`.
//!
//! - `ground_rivals`: the rival men working your areas, most ground first:
//!   face, program, role, style, the areas you share with where he stands
//!   there in a word, his Home schools, his career and what's between you.
//! - `ground_rival` {man}: one rival man in full, with the visits your
//!   people saw.
//! - `ground_rival_page` {man}: everything board 6 draws about him: the
//!   races he is in with your commits (their signs dated and numbered, what
//!   the kid's coach tells you, your call and your claims), the claims
//!   between you, where he works on the map and your men against him.
//! - `ground_sheet` {school}: a school's sign-in sheet, the last six weeks.
//! - `ground_told` {id}: what a kid's coach tells you about the other
//!   staffs, by your standing with him.
//! - `ground_race`: your wavering commits (signs, your call, how it
//!   landed) and the claims made both ways with how they settled.
//! - `ground_fight` {id}: your call to a wavering commit's family.
//! - `ground_claim` {id, about, topic: job|depth|scheme}: your claim about a
//!   rival to a kid's family; it settles on the facts.

use afm_engine::world::ground::areas;
use afm_engine::world::ground::people::{self, Who};
use afm_engine::world::ground::race::{self, Rival, Topic};
use afm_engine::world::ground::standing::{self, Level};
use afm_engine::world::{World, geo};
use serde_json::{Value, json};

use crate::lane_ground::{long_date, short_date};
use crate::views::{TeamRef, team_colors};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let num = |k: &str| args.get(k).and_then(Value::as_u64);
    Some(match cmd {
        "ground_rivals" => {
            game.w().map(|w| {
                json!({ "today": long_date(w.today), "rivals": race::rivals(w).iter().map(|r| rival_json(w, r)).collect::<Vec<_>>() })
            })
        }
        "ground_rival" => game.w().and_then(|w| {
            let man = num("man").ok_or("Missing the man.")? as u32;
            race::rival(w, man).map(|r| rival_json(w, &r)).ok_or_else(|| "He isn't a rival recruiter.".to_string())
        }),
        "ground_rival_page" => game.w().and_then(|w| {
            let man = num("man").ok_or("Missing the man.")? as u32;
            let r = race::rival(w, man).ok_or_else(|| "He isn't a rival recruiter.".to_string())?;
            let mut v = rival_json(w, &r);
            v["face"] = crate::lane_ground_people::face_of(game, w, Who::Coach(man));
            rival_page(w, &r, v)
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
        "colors": team_colors(w, TeamRef::College(r.program as usize)),
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
        "today": long_date(w.today),
        "wavers": r.wavers.iter().rev().map(|x| json!({
            "id": x.id,
            "name": name_of(w, x.id),
            "rival": x.rival,
            "rival_school": w.programs.get(x.rival as usize).map(|p| p.real_name.clone()),
            "date": x.date.to_string(),
            "date_words": short_date(x.date),
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
            "settles_words": short_date(c.settles),
            "held": c.held,
        })).collect::<Vec<_>>(),
    })
}

fn level_json(l: Level) -> Value {
    json!({ "n": l as usize, "name": l.name() })
}

/// Where a sign came from, in the screen's words.
fn source_words(kind: &str) -> &'static str {
    match kind {
        "sheet" => "a sign-in sheet",
        "visit" => "a visit he took",
        "offer" => "an offer",
        _ => "what people tell you",
    }
}

/// What a claim settles on, in words, and what a false one costs.
fn settles_on(topic: &str, yours: bool) -> String {
    let facts = match topic {
        "job" => "whether the head coach it named is still in the job after the carousel",
        "depth" => "the depth chart at his position",
        _ => "how his game fits that scheme",
    };
    if yours {
        format!("It settles on {facts}. If it isn't true, it costs you with the family and his coach.")
    } else {
        format!("It settles on {facts}. If it isn't true, the family remembers who said it, and so does his coach.")
    }
}

fn claim_row(w: &World, c: &race::Claim) -> Value {
    json!({
        "id": c.kid,
        "name": name_of(w, c.kid),
        "words": c.words,
        "topic": c.topic.key(),
        "said": short_date(c.said),
        "settles": short_date(c.settles),
        "held": c.held,
        "note": settles_on(c.topic.key(), c.man.is_none()),
    })
}

/// Board 6, one rival man: see the module's header.
fn rival_page(w: &World, r: &Rival, mut v: Value) -> Reply {
    let prog = standing::seat_program(w).ok_or("Ground is a college head coach's offseason game.")?;
    let rp = r.program as usize;
    let st = &w.ground.race;

    // The races he is in with your commits: their signs, oldest first.
    let mut races = Vec::new();
    for x in st.wavers.iter().rev().filter(|x| x.rival as usize == rp) {
        let Some(i) = afm_engine::world::scouting::place(w, x.id) else { continue };
        let q = &w.people[i];
        let mut signs: Vec<(afm_engine::calendar::Date, String, String)> = x
            .signs
            .iter()
            .enumerate()
            .map(|(k, text)| match x.when.get(k) {
                Some((d, kind)) => (*d, text.clone(), kind.clone()),
                None => (x.date, text.clone(), String::new()),
            })
            .collect();
        signs.sort_by_key(|s| s.0);
        let fam = people::family_of(w, i);
        let lv = standing::level(w, prog, fam);
        let h = q.hs;
        let (lat, lon) = areas::school_place(w, h);
        let kid_claims: Vec<Value> = [Topic::Job, Topic::Depth, Topic::Scheme]
            .into_iter()
            .map(|t| {
                let made = st
                    .claims
                    .iter()
                    .any(|c| c.kid == x.id && c.by as usize == prog && c.about == r.program && c.topic == t);
                let name = &r.school;
                json!({
                    "topic": t.key(),
                    "made": made,
                    "title": match t {
                        Topic::Job => format!("Tell the family {name}'s head coach won't last"),
                        Topic::Depth => format!("Tell the family {name} will bury him on the depth chart"),
                        Topic::Scheme => format!("Tell the family {name}'s scheme doesn't fit him"),
                    },
                })
            })
            .collect();
        races.push(json!({
            "id": x.id,
            "name": q.name.clone(),
            "position": q.player.position.name(),
            "school": w.school_name(h),
            "school_id": h,
            "date": short_date(x.date),
            "signs": signs.iter().enumerate().map(|(n, s)| json!({
                "n": n + 1, "date": short_date(s.0), "text": s.1, "source": source_words(&s.2),
            })).collect::<Vec<_>>(),
            "told": race::told(w, x.id),
            "family": level_json(lv),
            "open": x.fought.is_none(),
            "said": x.line,
            "your_claims": kid_claims,
            "state": geo::STATE_CENTERS[w.high_schools[h].state].0,
            "at": [lat, lon],
        }));
    }

    // The claims between you, about the kids he is in.
    let his: Vec<Value> = st.claims.iter().rev().filter(|c| c.man == Some(r.man)).map(|c| claim_row(w, c)).collect();
    let yours: Vec<Value> =
        st.claims.iter().rev().filter(|c| c.man.is_none() && c.about == r.program).map(|c| claim_row(w, c)).collect();

    // Where he works, and your men against him.
    let talent = standing::talents(w);
    let by_area = areas::schools_by_area(w);
    let mut against = Vec::new();
    let mut states: Vec<&str> = Vec::new();
    let mut codes: Vec<String> = Vec::new();
    for a in &r.areas {
        let a_ = areas::area(a.0);
        let code = areas::code(a.0);
        let sc = geo::STATE_CENTERS[a_.state].0;
        if !states.contains(&sc) {
            states.push(sc);
        }
        codes.push(code.clone());
        let mine = standing::area_points(w, prog, &by_area[a.0 as usize], &talent);
        let man = w.ground.staff.beats.iter().find(|b| b.area == a.0).map(|b| w.coaches[b.man as usize].name.clone());
        against.push(json!({
            "area": code,
            "name": a_.name,
            "nickname": a.1,
            "man": man,
            "you": level_json(Level::of(mine)),
            "his": a.2,
        }));
    }
    let total = race::rivals(w);
    let mut staffs: Vec<u32> = total.iter().map(|x| x.program).collect();
    staffs.sort_unstable();
    staffs.dedup();
    v["today"] = json!(long_date(w.today));
    v["totals"] =
        json!({ "men": total.len(), "staffs": staffs.len(), "kids": races.len(), "claims": his.len() + yours.len() });
    v["races"] = json!(races);
    v["claims_his"] = json!(his);
    v["claims_yours"] = json!(yours);
    v["map"] = json!({ "states": states, "areas": codes, "program": r.program });
    v["against"] = json!(against);
    Ok(v)
}
