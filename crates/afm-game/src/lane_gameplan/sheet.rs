//! The call sheet (G7-G13, board 22): what your coordinators drafted on
//! plan day for the next game, box by box and hash by hash, with their
//! reasons, the opening script and what each play tests, the
//! quarterback's favourites, the tendency breakers and the plays repped
//! badly. You approve it part by part, edit a line, or send it back with a
//! note; the bad-weather version sits beside it.

use afm_engine::game::playbook::{Backfield, Coverage, PASS_CONCEPTS, Pressure, RUN_CONCEPTS};
use afm_engine::game::sheet;
use afm_engine::training::seat::{SheetKind, SheetPlay};
use afm_engine::world::World;
use afm_engine::world::plans::{self, BREAKER, CUT, Entry, FAV, NOTES, PARTS, Play, WET, YOURS};
use serde_json::{Value, json};

use super::{coach_link, head, lvl, seat, team_link};
use crate::Reply;
use crate::lane_training::{backfield_of, personnel_of, set_of};
use crate::views::date_text;

pub(crate) const PRESSURES: [Pressure; 6] =
    [Pressure::Four, Pressure::LbBlitz, Pressure::DbBlitz, Pressure::DoubleA, Pressure::Simulated, Pressure::FireZone];

/// The boxes on the grid, in the board's order, and their names.
const GRID: [(u8, &str); 12] = [
    (sheet::FIRST, "1st & 10"),
    (sheet::SECOND_LONG, "2nd & 7+"),
    (sheet::SECOND_MEDIUM, "2nd & 4-6"),
    (sheet::SECOND_SHORT, "2nd & 1-3"),
    (sheet::THIRD_LONG, "3rd & 7+"),
    (sheet::THIRD_MEDIUM, "3rd & 4-6"),
    (sheet::THIRD_SHORT, "3rd & 1-3"),
    (sheet::RED_ZONE, "Red zone"),
    (sheet::GOAL_LINE, "Goal line"),
    (sheet::BACKED_UP, "Backed up"),
    (sheet::FOUR_MINUTE, "Four-minute"),
    (sheet::TWO_MINUTE, "Two-minute"),
];

fn side_arg(a: &Value) -> usize {
    match a.get("side") {
        Some(Value::String(s)) if s == "defense" => 1,
        Some(Value::Number(n)) => n.as_u64().unwrap_or(0).min(1) as usize,
        _ => 0,
    }
}

fn back_words(b: Backfield) -> &'static str {
    match b {
        Backfield::Shotgun => "gun",
        Backfield::Pistol => "pistol",
        Backfield::UnderCenter => "under center",
    }
}

fn play_json(w: &World, i: usize, e: &Entry) -> Value {
    let (_, hash) = sheet::split(e.k);
    let (name, look, kind) = match e.play {
        Play::Off(p) => {
            let what = match p.play {
                SheetKind::Run(r) => r.name().to_string(),
                SheetKind::Pass(k) => {
                    PASS_CONCEPTS.get(k as usize).map_or("a pass".to_string(), |c| c.name.to_string())
                }
                SheetKind::Design(k) => {
                    w.training.designs.get(k as usize).map_or("your design".to_string(), |d| d.design.name.clone())
                }
            };
            let kind = match p.play {
                SheetKind::Run(_) => "run",
                SheetKind::Pass(_) => "pass",
                SheetKind::Design(_) => "design",
            };
            (what, format!("{} {} {}", p.personnel.name(), back_words(p.backfield), p.set.name()), kind)
        }
        Play::Def(c, p) => (c.name().to_string(), p.name().to_string(), "def"),
    };
    json!({
        "i": i, "name": name, "look": look, "kind": kind, "hash": hash,
        "fav": e.marks & FAV != 0, "breaker": e.marks & BREAKER != 0, "cut": e.marks & CUT != 0,
        "yours": e.marks & YOURS != 0, "wet": e.marks & WET != 0,
        "why": e.why, "tests": e.tests,
    })
}

pub(super) fn view(w: &mut World, a: &Value) -> Reply {
    let side = side_arg(a);
    let wet = a.get("wet").and_then(Value::as_bool).unwrap_or(false);
    let s = seat(w)?;
    let level = s.level;
    let qb = plans::starting_qb(&s.team, level);
    drop(s);
    if !plans::ensure_draft(w) {
        return Ok(json!({
            "has": false, "level": lvl(level), "head": w.seat_head(),
            "why": "No game ahead on your schedule: your coordinators draft the sheet on plan day, the week of each game.",
        }));
    }
    let w: &World = w;
    let d = w.plans.draft.as_ref().ok_or("No draft.")?;
    let list = if side == 0 { &d.off } else { &d.def };
    let shown: Vec<(usize, &Entry)> = list.iter().enumerate().filter(|(_, e)| (e.marks & WET != 0) == wet).collect();
    let in_box = |k: u8| -> Vec<Value> {
        shown.iter().filter(|(_, e)| sheet::split(e.k).0 == k).map(|(i, e)| play_json(w, *i, e)).collect()
    };
    let parts: Vec<Value> = PARTS
        .iter()
        .enumerate()
        .map(|(pi, name)| {
            let n = list.iter().filter(|e| e.marks & WET == 0 && plans::part_of(e.k) == pi).count();
            let st = d.status[side][pi];
            let ed = d.edits[side][pi];
            let state = match (st, ed) {
                (_, _) if n == 0 => "Nothing in it".to_string(),
                (1, _) => "Approved".to_string(),
                (2, e) if e > 0 => format!("You edited {e}"),
                (2, _) => "Redrafted".to_string(),
                _ => "Waiting".to_string(),
            };
            let name = if side == 1 && pi == 5 { "Pressure package and two-point" } else { name };
            json!({ "i": pi, "name": name, "n": n, "state": state, "approved": st == 1 || n == 0 })
        })
        .collect();
    let grid: Vec<Value> = GRID
        .iter()
        .map(|&(k, name)| {
            let split = side == 0 && ((1..=6).contains(&k) || k == sheet::SECOND_MEDIUM);
            json!({ "k": k, "name": name, "split": split, "plays": in_box(k) })
        })
        .collect();
    let extras = vec![
        json!({ "k": sheet::SHOTS, "name": if side == 0 { "Shot plays" } else { "Pressure package" }, "plays": in_box(sheet::SHOTS) }),
        json!({ "k": sheet::TWO_POINT, "name": if side == 0 { "Two-point" } else { "Two-point defense" }, "plays": in_box(sheet::TWO_POINT) }),
    ];
    let by = d.by[side].map(|c| if w.coaches[c].is_yours() { "you".to_string() } else { coach_link(w, c) });
    let all_ok = parts.iter().all(|p| p["approved"].as_bool() == Some(true));
    let wet_n = list.iter().filter(|e| e.marks & WET != 0).count();
    let favs: Vec<Value> = plans::favourites(w, qb)
        .into_iter()
        .map(|(c, n, epa)| json!({ "name": PASS_CONCEPTS[c].name, "n": n, "epa": (epa / n as f64 * 100.0).round() / 100.0 }))
        .collect();
    let add = if side == 0 {
        json!({
            "runs": RUN_CONCEPTS.iter().enumerate().map(|(i, r)| json!({ "i": i, "name": r.name() })).collect::<Vec<_>>(),
            "passes": PASS_CONCEPTS.iter().enumerate().map(|(i, c)| json!({ "i": i, "name": c.name })).collect::<Vec<_>>(),
            "personnels": ["11", "12", "21", "10", "22", "13"],
            "backfields": ["shotgun", "under center", "pistol"],
            "sets": ["doubles", "trips", "bunch", "stack", "empty", "wing"],
        })
    } else {
        json!({
            "coverages": Coverage::ALL.iter().enumerate().map(|(i, c)| json!({ "i": i, "name": c.name() })).collect::<Vec<_>>(),
            "pressures": PRESSURES.iter().enumerate().map(|(i, p)| json!({ "i": i, "name": p.name() })).collect::<Vec<_>>(),
        })
    };
    Ok(json!({
        "has": true, "level": lvl(level), "head": w.seat_head(),
        "side": if side == 0 { "offense" } else { "defense" }, "wet": wet, "wet_n": wet_n,
        "game": date_text(d.game), "opp": team_link(w, level, d.opp), "opp_name": plans::team_name(w, level, d.opp),
        "drafted": date_text(d.drafted), "round": d.round, "by": by,
        "early": d.early_look(level), "plan_day": date_text(plans::plan_day(level, d.game)),
        "parts": parts, "all_approved": all_ok,
        "script": { "k": sheet::SCRIPT, "name": "Opening script", "plays": in_box(sheet::SCRIPT) },
        "extras": extras, "grid": grid,
        "notes": NOTES.iter().enumerate().filter(|(i, _)| if side == 0 { *i <= 4 } else { *i == 3 || *i >= 4 }).map(|(i, t)| json!({ "i": i, "text": t, "on": d.notes[side].contains(&(i as u8)) })).collect::<Vec<_>>(),
        "favourites": favs,
        "add": add,
        "stick": super::pct(w.plans.guide.stick.to.unwrap_or(0.85)),
    }))
}

pub(super) fn review(w: &mut World, a: &Value) -> Result<(), String> {
    head(w)?;
    let side = side_arg(a);
    let part = a.get("part").and_then(Value::as_u64).map(|x| (x as usize).min(PARTS.len() - 1));
    let approve = a.get("approve").and_then(Value::as_bool).unwrap_or(true);
    let notes: Vec<u8> = a
        .get("notes")
        .and_then(Value::as_array)
        .map(|v| v.iter().filter_map(|x| x.as_u64()).filter(|&x| (x as usize) < NOTES.len()).map(|x| x as u8).collect())
        .unwrap_or_default();
    if !approve && notes.is_empty() && part.is_none() {
        return Err("Pick a note for your coordinator first.".into());
    }
    plans::review(w, side, part, approve, notes)
}

pub(super) fn edit(w: &mut World, a: &Value) -> Result<(), String> {
    head(w)?;
    let side = side_arg(a);
    let i = a.get("i").and_then(Value::as_u64).ok_or("Missing play.")? as usize;
    let act = a.get("act").and_then(Value::as_str).ok_or("Missing act.")?;
    let add = match a.get("play") {
        Some(p) if act == "add" => Some(if side == 0 {
            let s = |k: &str| p.get(k).and_then(Value::as_str).unwrap_or("").to_string();
            let k = p.get("k").and_then(Value::as_u64).ok_or("Pick a play.")? as usize;
            let play = match s("kind").as_str() {
                "run" => SheetKind::Run(*RUN_CONCEPTS.get(k).ok_or("No such run.")?),
                _ => {
                    if k >= PASS_CONCEPTS.len() {
                        return Err("No such pass.".into());
                    }
                    SheetKind::Pass(k as u16)
                }
            };
            Play::Off(SheetPlay {
                personnel: personnel_of(&s("personnel")),
                backfield: backfield_of(&s("backfield")),
                set: set_of(&s("set")),
                play,
                hash: p.get("hash").and_then(Value::as_u64).unwrap_or(0).min(2) as u8,
            })
        } else {
            let c = p.get("coverage").and_then(Value::as_u64).ok_or("Pick a coverage.")? as usize;
            let r = p.get("pressure").and_then(Value::as_u64).unwrap_or(0) as usize;
            Play::Def(*Coverage::ALL.get(c).ok_or("No such coverage.")?, *PRESSURES.get(r).ok_or("No such pressure.")?)
        }),
        _ => None,
    };
    plans::edit(w, side, i, act, add)
}
