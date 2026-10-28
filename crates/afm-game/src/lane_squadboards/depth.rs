//! V1, the depth chart in the stadium: each formation's spots on the field
//! with the starter and the two behind him, the unit cards ranked against
//! the level, and the picked spot's man in full (his staff read against the
//! level's starters, his key ratings, his snaps and grade last game, his
//! coach's word and the depth behind him). The flat boards and lane 5's
//! packages stay as they were; this reads the same depth.

use std::collections::HashMap;

use afm_engine::positions::Position;
use afm_engine::training::seat::Board;
use afm_engine::world::World;
use afm_engine::world::norms::Rated;
use serde_json::{Value, json};

use super::{
    class_full, first_last, game_grade, health, key_attrs, level_name, marks, mid, my_games, my_team, own_read,
    readiness, snaps_in, starters_words, team_snaps, unit_card,
};
use crate::Reply;
use crate::reads;
use crate::views::{self, TeamRef};

/// A tab's key, its name, and its formations (board key, name).
type Tab = (&'static str, &'static str, &'static [(&'static str, &'static str)]);

/// The stadium's tabs and the formations under each (the board keys of
/// `lane_training::boards`).
const TABS: [Tab; 4] = [
    ("offense", "Offense", &[("offense", "Base"), ("jumbo", "Jumbo"), ("goal_line_o", "Goal line")]),
    (
        "defense",
        "Defense",
        &[("defense", "Base"), ("nickel", "Nickel"), ("dime", "Dime"), ("goal_line_d", "Goal line")],
    ),
    (
        "special",
        "Special teams",
        &[
            ("specialists", "Specialists"),
            ("field_goal", "Field goal"),
            ("kick_cover", "Kickoff coverage"),
            ("punt_cover", "Punt coverage"),
        ],
    ),
    ("returns", "Returners", &[("kick_return", "Kickoff return"), ("punt_return", "Punt return")]),
];

/// Lane 5's packages: their second and third men are set on its page.
const LANE5: [&str; 5] = ["nickel", "dime", "goal_line_d", "jumbo", "goal_line_o"];

/// The tab and formation asked for (a board key alone finds its tab).
fn pick(args: &Value) -> (usize, usize) {
    let form = args.get("form").and_then(Value::as_str);
    if let Some(f) = form {
        for (t, tab) in TABS.iter().enumerate() {
            if let Some(k) = tab.2.iter().position(|x| x.0 == f) {
                return (t, k);
            }
        }
    }
    let t = args.get("tab").and_then(Value::as_str).and_then(|s| TABS.iter().position(|x| x.0 == s)).unwrap_or(0);
    (t, 0)
}

/// A spot's name in words: "Left tackle", "Wide receiver (Z)".
fn spot_words(slot: &str, pos: Position) -> String {
    let named = match slot {
        "QB" => "Quarterback",
        "RB" => "Running back",
        "FB" => "Fullback",
        "SLOT" => "Slot receiver",
        "TE" if pos == Position::TE => "Tight end",
        "LT" => "Left tackle",
        "LG" => "Left guard",
        "C" => "Center",
        "RG" => "Right guard",
        "RT" => "Right tackle",
        "LE" => "Left end",
        "RE" => "Right end",
        "DT" => "Defensive tackle",
        "NT" => "Nose tackle",
        "SAM" => "Strongside linebacker",
        "MIKE" => "Middle linebacker",
        "WILL" => "Weakside linebacker",
        "CB" => "Cornerback",
        "NICKEL" => "Nickel back",
        "DIME" => "Dime back",
        "FS" => "Free safety",
        "SS" => "Strong safety",
        "K" => "Kicker",
        "P" => "Punter",
        "LS" => "Long snapper",
        "H" => "Holder",
        "KR" => "Kick returner",
        "PR" => "Punt returner",
        "PP" => "Personal protector",
        "GUNNER" => "Gunner",
        "JAM" => "Jammer",
        _ => "",
    };
    if !named.is_empty() {
        return named.to_string();
    }
    let what = super::cap(crate::lane_ratings::pos_words(pos).0);
    format!("{what} ({slot})")
}

/// One man on a spot.
fn man(w: &World, t: TeamRef, i: usize, num: Option<u8>) -> Value {
    let p = &w.people[i];
    let pos = p.player.position;
    let (first, last) = first_last(&p.name);
    let read = own_read(w, t, i);
    let m = marks(w, t, pos, Rated::Overall);
    let (h, why) = health(w, i);
    let b = afm_engine::body::body_now(&p.player);
    json!({
        "id": p.player.id, "name": p.name, "first": first, "last": last, "num": num,
        "pos": pos.name(), "class": views::class_text(w, p), "class_full": class_full(w, p),
        "size": format!("{} · {:.0}", views::ft_in(b.height), b.weight), "home": views::hometown(w, p),
        "read": read.json(), "health": h, "health_why": why,
        "ready": readiness(w, i, read, m.as_ref()),
    })
}

/// The staff's list at a position: the depth order (yours where set,
/// else your staff's), then the rest of the position group by your read,
/// a man from another position a step behind (he pays to cross-train).
fn staff_list(
    w: &World,
    t: TeamRef,
    roster: &[usize],
    order: &[(Position, Vec<u32>)],
    yours: &[(Position, Vec<u32>)],
    pos: Position,
) -> Vec<u32> {
    let mut v: Vec<u32> =
        yours.iter().chain(order.iter()).find(|(p, _)| *p == pos).map(|(_, ids)| ids.clone()).unwrap_or_default();
    let group = afm_engine::spots::group(pos);
    let mut rest: Vec<(f64, u32)> = roster
        .iter()
        .copied()
        .filter(|&i| {
            let q = w.people[i].player.position;
            (q == pos || (!q.is_specialist() && afm_engine::spots::group(q) == group))
                && !v.contains(&w.people[i].player.id)
        })
        .map(|i| {
            let q = w.people[i].player.position;
            let m = mid(own_read(w, t, i)).unwrap_or(40.0) - if q == pos { 0.0 } else { 8.0 };
            (m, w.people[i].player.id)
        })
        .collect();
    rest.sort_by(|a, b| b.0.total_cmp(&a.0));
    v.extend(rest.into_iter().map(|x| x.1));
    v
}

pub(crate) struct Spot {
    pub slot: String,
    pub pos: Position,
    pub x: f64,
    pub y: f64,
    pub controls: bool,
    pub ids: [Option<u32>; 3],
}

/// The formation's spots three deep: the first man from the board, the
/// second and third from lane 5's package page where it keeps them, else
/// the staff's next men up (every spot's second before any third).
pub(crate) fn spots(w: &World, t: TeamRef, form: &str) -> Result<(Vec<Spot>, bool), String> {
    let bj = crate::lane_training::boards(w, &json!({}))?;
    let board = bj["boards"]
        .as_array()
        .and_then(|v| v.iter().find(|b| b["key"] == form))
        .cloned()
        .ok_or("No such formation.")?;
    let yours_board = board["yours"].as_bool().unwrap_or(false)
        || (LANE5.contains(&form)
            && w.plans.depth.iter().any(|(b, v)| Some(*b) == Board::parse(form) && v.iter().any(|&x| x != 0)));
    let slots = board["slots"].as_array().cloned().unwrap_or_default();
    let pos_of =
        |s: &Value| Position::ALL.iter().copied().find(|p| Some(p.name()) == s["pos"].as_str()).unwrap_or(Position::QB);
    let mut out: Vec<Spot> = slots
        .iter()
        .map(|s| Spot {
            slot: s["slot"].as_str().unwrap_or("").to_string(),
            pos: pos_of(s),
            x: s["x"].as_f64().unwrap_or(0.0),
            y: s["y"].as_f64().unwrap_or(0.0),
            controls: s["controls"].as_bool().unwrap_or(false),
            ids: [s["player"]["id"].as_u64().map(|x| x as u32), None, None],
        })
        .collect();
    // Lane 5's own second and third men.
    if LANE5.contains(&form)
        && let Ok(v) = crate::lane_gameplan::depth::view(w, &json!({ "board": form }))
    {
        for sp in v["spots"].as_array().into_iter().flatten() {
            let k = sp["k"].as_u64().unwrap_or(99) as usize;
            if let Some(s) = out.get_mut(k) {
                for pk in sp["picks"].as_array().into_iter().flatten() {
                    let n = pk["n"].as_u64().unwrap_or(0) as usize;
                    if (2..=3).contains(&n) {
                        s.ids[n - 1] = pk["p"]["id"].as_u64().map(|x| x as u32);
                    }
                }
            }
        }
        return Ok((out, yours_board));
    }
    let roster = views::roster_people(w, t);
    let order = w.seat_game_depth().unwrap_or_default();
    let mut used: Vec<u32> = out.iter().filter_map(|s| s.ids[0]).collect();
    let mut lists: HashMap<Position, Vec<u32>> = HashMap::new();
    for n in 1..3 {
        for s in out.iter_mut() {
            let list = lists.entry(s.pos).or_insert_with(|| staff_list(w, t, &roster, &order, &w.seat_depth, s.pos));
            let id = list.iter().copied().find(|id| !used.contains(id));
            if let Some(id) = id {
                used.push(id);
            }
            s.ids[n] = id;
        }
    }
    Ok((out, yours_board))
}

/// The coach's word on a spot, from what the staff sees this week.
fn coach_note(w: &World, t: TeamRef, men: &[Option<usize>; 3], last_grade: Option<f64>) -> String {
    let name = |i: usize| first_last(&w.people[i].name).1;
    let Some(a) = men[0] else { return "Nobody is listed here: the staff will slide the next man over.".into() };
    let (h, why) = health(w, a);
    let second = men[1];
    if h == "out" {
        return match second {
            Some(b) => format!("{} is out ({why}). {} starts if he can't go.", name(a), name(b)),
            None => format!("{} is out ({why}), and nobody behind him is listed.", name(a)),
        };
    }
    if h == "limited" {
        return format!("{} is limited this week: {why}.", name(a));
    }
    let ra = mid(own_read(w, t, a)).unwrap_or(0.0);
    if let Some(b) = second {
        let rb = mid(own_read(w, t, b)).unwrap_or(0.0);
        if rb >= ra - 1.0 {
            return format!("{} is pushing him: the two reads overlap, and the job is open every week.", name(b));
        }
        let m = marks(w, t, w.people[b].player.position, Rated::Overall);
        if readiness(w, b, own_read(w, t, b), m.as_ref()) == "Needs reps" {
            return format!("Nobody behind {} is ready yet: {} needs reps.", name(a), name(b));
        }
    }
    match last_grade {
        Some(g) if g >= 75.0 => format!("{} graded {g:.0} last game: one of his best.", name(a)),
        Some(g) if g < 55.0 => format!("{} graded {g:.0} last game. The staff wants a cleaner week from him.", name(a)),
        _ => match second {
            Some(b) => format!("{} is the starter and {} is next up.", name(a), name(b)),
            None => format!("{} is the starter, with nobody ready behind him.", name(a)),
        },
    }
}

/// The unit cards of a tab.
fn units(w: &World, t: TeamRef, tab: &str) -> Vec<Value> {
    use Position::*;
    let list: &[(&str, &[(Position, usize)])] = match tab {
        "offense" => &[
            ("QB", &[(QB, 1)]),
            ("RB", &[(RB, 1)]),
            ("WR", &[(WR, 3)]),
            ("TE", &[(TE, 1)]),
            ("OL", &[(OT, 2), (IOL, 3)]),
            ("Offense", &[(QB, 1), (RB, 1), (WR, 3), (TE, 1), (OT, 2), (IOL, 3)]),
        ],
        "defense" => &[
            ("Edge", &[(EDGE, 2)]),
            ("Interior", &[(DT, 2)]),
            ("LB", &[(LB, 3)]),
            ("CB", &[(CB, 2)]),
            ("S", &[(S, 2)]),
            ("Defense", &[(EDGE, 2), (DT, 2), (LB, 3), (CB, 2), (S, 2)]),
        ],
        _ => &[("K", &[(K, 1)]), ("P", &[(P, 1)]), ("LS", &[(LS, 1)]), ("Specialists", &[(K, 1), (P, 1), (LS, 1)])],
    };
    list.iter().map(|(n, s)| unit_card(w, t, n, s)).collect()
}

pub(crate) fn view(w: &World, args: &Value) -> Reply {
    let t = my_team(w)?;
    let (ti, fi) = pick(args);
    let (tab, tab_label, forms) = TABS[ti];
    let (form, form_label) = forms[fi];
    let (spots, yours) = spots(w, t, form)?;
    let nums: HashMap<usize, u8> = crate::lane_core::people::jerseys(w, t).into_iter().collect();
    let idx = |id: Option<u32>| id.and_then(|id| views::person_index(w, id));
    let last = my_games(w).last().copied();
    let mut slots: Vec<Value> = Vec::new();
    for (k, s) in spots.iter().enumerate() {
        let men: [Option<usize>; 3] = [idx(s.ids[0]), idx(s.ids[1]), idx(s.ids[2])];
        let picks: Vec<Value> =
            men.iter().map(|m| m.map_or(Value::Null, |i| man(w, t, i, nums.get(&i).copied()))).collect();
        // The starter in full.
        let detail = men[0].map(|i| {
            let p = &w.people[i];
            let pos = p.player.position;
            let read = own_read(w, t, i);
            let m = marks(w, t, pos, Rated::Overall);
            let band = match (read, m.as_ref()) {
                (reads::Read::Range(lo, hi), Some(m)) => {
                    let (word, line) = crate::lane_ratings::band(m, lo, hi, &starters_words(w, t, pos));
                    json!({ "word": word, "line": line })
                }
                _ => Value::Null,
            };
            let attrs = reads::attributes(w, i);
            let keys: Vec<Value> = key_attrs(pos)
                .into_iter()
                .map(|a| {
                    let r = attrs.iter().find(|x| x.0 == a).map_or(reads::Read::NotSeen, |x| x.1);
                    json!({ "label": a.label(), "key": a.key(), "read": r.json() })
                })
                .collect();
            let (snaps, of, grade) = match last {
                Some((g, side)) => (
                    snaps_in(g, side, p.player.id, pos),
                    Some(team_snaps(g, side, pos)),
                    game_grade(g, side, p.player.id),
                ),
                None => (None, None, None),
            };
            json!({
                "band": band, "keys": keys,
                "snaps": snaps, "team_snaps": of, "grade": grade.map(f64::round),
                "coach": crate::lane_training::teacher(w, t, i),
                "note": coach_note(w, t, &men, grade),
            })
        });
        slots.push(json!({
            "k": k, "slot": s.slot, "pos": s.pos.name(), "pos_full": spot_words(&s.slot, s.pos),
            "x": s.x, "y": s.y, "controls": s.controls, "picks": picks, "detail": detail,
            "group": starters_words(w, t, s.pos),
        }));
    }
    let last_game = last.map(|(g, side)| {
        let opp = g.names[1 - side].clone();
        json!({ "date": views::date_text(g.date), "opp": opp, "score": [g.stats.score[side], g.stats.score[1 - side]] })
    });
    Ok(json!({
        "team": views::team_brief(w, t),
        "level": level_name(w, t),
        "head": w.seat_head(),
        "tab": tab, "tab_label": tab_label, "form": form, "form_label": form_label,
        "tabs": TABS.iter().map(|(k, l, f)| json!({ "key": k, "label": l, "forms": f.iter().map(|(a, b)| json!({ "key": a, "label": b })).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "package": LANE5.contains(&form),
        "base": matches!(form, "offense" | "defense" | "specialists"),
        "yours": yours,
        "next": views::next_game(w, t),
        "last_game": last_game,
        "slots": slots,
        "units": units(w, t, tab),
    }))
}

/// Moves the man at depth `n` (2 or 3) on a spot up one: on a base board
/// it reorders his position's depth (an assistant proposes, as on the
/// flat board); on a package it changes the unit's first man or lane 5's
/// second and third.
pub(crate) fn move_up(w: &mut World, args: &Value) -> Result<Option<String>, String> {
    let t = my_team(w)?;
    let (ti, fi) = pick(args);
    let form = TABS[ti].2[fi].0;
    let k = args.get("slot").and_then(Value::as_u64).ok_or("Missing spot.")? as usize;
    let n = args.get("n").and_then(Value::as_u64).ok_or("Missing depth.")? as usize;
    if !(2..=3).contains(&n) {
        return Err("Only a man behind the starter moves up.".into());
    }
    let (spots, _) = spots(w, t, form)?;
    let s = spots.get(k).ok_or("No such spot.")?;
    let (Some(up), Some(down)) = (s.ids[n - 1], s.ids[n - 2]) else {
        return Err("Nobody is there to move.".into());
    };
    if matches!(form, "offense" | "defense" | "specialists") {
        let pos = s.pos;
        if !w.controls(pos) {
            return Err(format!("That's not yours to set. Your job covers {}.", w.your_controls()));
        }
        let order = w.seat_game_depth().unwrap_or_default();
        let roster = views::roster_people(w, t);
        let mut list = staff_list(w, t, &roster, &order, &w.seat_depth, pos);
        let a = list.iter().position(|&x| x == down);
        let b = list.iter().position(|&x| x == up);
        match (a, b) {
            (Some(a), Some(b)) => list.swap(a, b),
            (Some(a), None) => list.insert(a, up),
            _ => return Err("Those two aren't on this position's depth.".into()),
        }
        let note = w.propose_depth(pos, list)?;
        w.refresh_seat_team();
        return Ok(note);
    }
    let b = Board::parse(form).ok_or("No such unit.")?;
    if !w.seat_head() && !b.slots().iter().all(|x| w.controls(x.1)) {
        return Err(format!("That unit is the head coach's. Your job covers {}.", w.your_controls()));
    }
    if n == 2 {
        let mut firsts: Vec<u32> = spots.iter().map(|x| x.ids[0].unwrap_or(0)).collect();
        firsts[k] = up;
        w.training.packages.retain(|(x, _)| *x != b);
        w.training.packages.push((b, firsts));
    }
    if LANE5.contains(&form) {
        let len = 2 * b.slots().len();
        let mut v: Vec<u32> = w.plans.depth.iter().find(|(x, _)| *x == b).map(|(_, v)| v.clone()).unwrap_or_default();
        v.resize(len, 0);
        if n == 2 {
            v[2 * k] = down;
        } else {
            v[2 * k] = up;
            v[2 * k + 1] = down;
        }
        w.plans.depth.retain(|(x, _)| *x != b);
        w.plans.depth.push((b, v));
    } else if n == 3 {
        return Err("Your staff fills the men behind the first on this unit.".into());
    }
    w.refresh_seat_team();
    Ok(None)
}

/// Gives a formation back to the staff: your order at its positions on a
/// base board, your men on a package (lane 5's second and third too).
pub(crate) fn reset(w: &mut World, args: &Value) -> Result<(), String> {
    my_team(w)?;
    let (ti, fi) = pick(args);
    let form = TABS[ti].2[fi].0;
    crate::lane_training::board_set(w, &json!({ "board": form, "reset": true }))?;
    if LANE5.contains(&form)
        && let Some(b) = Board::parse(form)
    {
        w.plans.depth.retain(|(x, _)| *x != b);
        w.refresh_seat_team();
    }
    Ok(())
}
