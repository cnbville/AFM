//! Package depth and the line rotation (G15, board 24): each package's
//! spots three deep (the first man from the package board, the second and
//! third yours or your staff's), how much of the game each package plays,
//! where it is thin, and the defensive line's rotation by series.

use afm_engine::positions::Position;
use afm_engine::spots::group;
use afm_engine::training::seat::Board;
use afm_engine::world::World;
use afm_engine::world::plans;
use serde_json::{Value, json};

use super::{head, is_snap, lvl, pct, season_games, seat};
use crate::Reply;
use crate::lane_training::{boards, practice_status};
use crate::reads;
use crate::views::{class_text, eye_for, person_index, reader_key};

const PACKAGES: [(Board, &str); 5] = [
    (Board::Nickel, "Defense"),
    (Board::Dime, "Defense"),
    (Board::GoalLineD, "Defense"),
    (Board::Jumbo, "Offense"),
    (Board::GoalLineO, "Offense"),
];

/// Snaps a lineman plays in a series by his role (the board's tiles:
/// about 5.6 snaps a defensive series; proposed, not confirmed).
const SERIES_SNAPS: [f64; 4] = [4.2, 5.6, 2.8, 1.6];

fn picks_of(w: &World, b: Board) -> Vec<u32> {
    w.plans.depth.iter().find(|(x, _)| *x == b).map(|(_, v)| v.clone()).unwrap_or_default()
}

pub(super) fn view(w: &World, a: &Value) -> Reply {
    let s = seat(w)?;
    let reader = reader_key(w);
    let bj = boards(w, &json!({}))?;
    let games = season_games(w, &s);
    let depth = w.seat_game_depth().unwrap_or_default();
    let order_at = |pos: Position| depth.iter().find(|(p, _)| *p == pos).map(|(_, v)| v.clone()).unwrap_or_default();
    // A package's staff men up: the dressed order at the spot, then the
    // rest of the roster in the same group (corners and safeties, ends and
    // tackles), best at the spot, the way the engine fills a spot once the
    // listed men are used (`game/flow.rs`, `pick`).
    let team = w.seat_game_team().map(|(t, _)| t);
    let staff_at = |pos: Position| -> Vec<u32> {
        let mut v = order_at(pos);
        if let Some(t) = &team {
            let mut rest: Vec<(f64, u32)> = t
                .players
                .iter()
                .filter(|p| !p.position.is_specialist() && group(p.position) == group(pos) && !v.contains(&p.id))
                .map(|p| (p.overall_at(pos), p.id))
                .collect();
            rest.sort_by(|a, b| b.0.total_cmp(&a.0));
            v.extend(rest.into_iter().map(|r| r.1));
        }
        v
    };
    let who = |id: u32| -> Option<Value> {
        let i = person_index(w, id)?;
        let p = &w.people[i];
        let eye = eye_for(w, s.tr, p.player.position);
        let (practice, note) = practice_status(w, id, i);
        let wk = &w.plans.week;
        let this = plans::next_game(w).is_some_and(|n| wk.game == Some(n.date));
        let limit = if this { wk.limits.iter().find(|l| l.0 == id).map(|l| l.1) } else { None };
        Some(json!({
            "id": id, "name": p.name, "pos": p.player.position.name(), "class": class_text(w, p),
            "read": reads::overall(&p.player, true, eye, reader).json(), "practice": practice, "note": note,
            "limit": limit,
        }))
    };
    // Each package's share of the snaps this season.
    let share = |b: Board| -> Option<f64> {
        let (mut n, mut d) = (0usize, 0usize);
        for (g, side) in &games {
            for r in g.stats.plays.iter().filter(|r| !r.nullified && is_snap(r.kind)) {
                let def = r.offense != *side;
                let hit = match b {
                    Board::Nickel => def && r.dpers == 2,
                    Board::Dime => def && r.dpers == 3,
                    Board::GoalLineD => def && r.dpers == 4,
                    Board::Jumbo => !def && r.personnel == "13" && r.yardline_100 > 3.0,
                    Board::GoalLineO => {
                        !def && r.yardline_100 <= 3.0 && matches!(r.personnel.as_str(), "22" | "13" | "23")
                    }
                    _ => false,
                };
                let count = match b {
                    Board::Nickel | Board::Dime | Board::GoalLineD => def && r.dpers > 0,
                    _ => !def,
                };
                if count {
                    d += 1;
                    if hit {
                        n += 1;
                    }
                }
            }
        }
        (d >= 30).then(|| n as f64 / d as f64)
    };
    let open = a
        .get("board")
        .and_then(Value::as_str)
        .and_then(Board::parse)
        .filter(|b| PACKAGES.iter().any(|x| x.0 == *b))
        .unwrap_or(Board::Nickel);
    let mut list: Vec<Value> = Vec::new();
    let mut spots_open: Vec<Value> = Vec::new();
    for (b, group) in PACKAGES {
        let board = bj["boards"].as_array().and_then(|v| v.iter().find(|x| x["key"] == b.name())).cloned();
        let Some(board) = board else { continue };
        let mine = picks_of(w, b);
        let slots = board["slots"].as_array().cloned().unwrap_or_default();
        let firsts: Vec<u32> = slots.iter().filter_map(|x| x["player"]["id"].as_u64()).map(|x| x as u32).collect();
        let mut thin = 0;
        let mut spots: Vec<Value> = Vec::new();
        let mut used: Vec<u32> = firsts.clone();
        for (k, sl) in slots.iter().enumerate() {
            let pos =
                Position::ALL.iter().copied().find(|p| Some(p.name()) == sl["pos"].as_str()).unwrap_or(Position::CB);
            let first = sl["player"]["id"].as_u64().map(|x| x as u32);
            // The second and third: yours, else your staff's next men up at
            // the spot (the depth order, then the group, past the firsts).
            let mut staff = staff_at(pos).into_iter().filter(|id| !used.contains(id));
            let mut deep: Vec<(Option<u32>, bool)> = Vec::new();
            for n in 0..2 {
                let set = mine.get(2 * k + n).copied().filter(|&x| x != 0);
                match set {
                    Some(id) => deep.push((Some(id), true)),
                    None => deep.push((staff.next(), false)),
                }
            }
            for (id, _) in &deep {
                if let Some(id) = id {
                    used.push(*id);
                }
            }
            let second_ok = deep[0].0.and_then(|id| person_index(w, id).map(|i| practice_status(w, id, i).0 != "out"));
            if second_ok != Some(true) {
                thin += 1;
            }
            let mut picks =
                vec![json!({ "n": 1, "p": first.and_then(who), "yours": board["yours"].as_bool().unwrap_or(false) })];
            for (n, (id, yours)) in deep.iter().enumerate() {
                picks.push(json!({ "n": n + 2, "p": id.and_then(who), "yours": yours }));
            }
            spots.push(json!({ "k": k, "slot": sl["slot"], "pos": pos.name(), "picks": picks, "thin": second_ok != Some(true) }));
        }
        let sh = share(b);
        let ready = if thin == 0 {
            "two deep and ready".to_string()
        } else if thin == 1 {
            "1 thin spot at second string".to_string()
        } else {
            format!("{thin} thin spots at second string")
        };
        list.push(json!({ "key": b.name(), "label": b.label(), "group": group, "share": sh.map(pct), "ready": ready, "thin": thin, "yours": !mine.is_empty() }));
        if b == open {
            spots_open = spots;
        }
    }
    // The pool for each position on the open package.
    let mut pool: Vec<Value> = Vec::new();
    let positions: Vec<Position> = open.slots().iter().map(|s| s.1).collect();
    let roster = crate::views::roster_people(w, s.tr);
    for pos in Position::ALL.iter().copied().filter(|p| positions.contains(p)) {
        let mut v: Vec<(f64, Value)> = roster
            .iter()
            .copied()
            .filter(|&i| w.people[i].player.position == pos)
            .filter_map(|i| {
                let id = w.people[i].player.id;
                let j = who(id)?;
                let mid = j["read"]
                    .as_array()
                    .map_or(0.0, |r| (r[0].as_f64().unwrap_or(0.0) + r[1].as_f64().unwrap_or(0.0)) / 2.0);
                Some((mid, j))
            })
            .collect();
        v.sort_by(|a, b| b.0.total_cmp(&a.0));
        pool.push(json!({ "pos": pos.name(), "players": v.into_iter().map(|x| x.1).collect::<Vec<_>>() }));
    }
    // The line rotation by series.
    let mut line: Vec<Value> = Vec::new();
    let mut total = 0.0;
    let mut listed: Vec<u32> = Vec::new();
    for pos in [Position::EDGE, Position::DT] {
        // A man on both lists rotates at the first of them only.
        let ids: Vec<u32> = order_at(pos).into_iter().filter(|id| !listed.contains(id)).take(4).collect();
        listed.extend(&ids);
        for (k, id) in ids.into_iter().enumerate() {
            let role = w.plans.rotation.iter().find(|r| r.0 == id).map_or(0, |r| r.1);
            let cells: Vec<&str> = (0..12)
                .map(|series| match role {
                    1 => "whole",
                    2 => {
                        if series % 2 == 0 {
                            "whole"
                        } else {
                            ""
                        }
                    }
                    3 => "rush",
                    _ => {
                        if k < 2 || series % 3 == 2 {
                            "staff"
                        } else {
                            ""
                        }
                    }
                })
                .collect();
            let per = match role {
                0 => {
                    if k < 2 {
                        SERIES_SNAPS[0]
                    } else {
                        1.5
                    }
                }
                2 => SERIES_SNAPS[2],
                r => SERIES_SNAPS[r as usize],
            };
            total += per * 12.0;
            if let Some(mut j) = who(id) {
                j["role"] = json!(role);
                j["cells"] = json!(cells);
                j["snaps"] = json!((per * 12.0).round());
                j["string"] = json!(if k < 2 { "1st" } else { "2nd" });
                line.push(j);
            }
        }
    }
    Ok(json!({
        "level": lvl(s.level), "head": w.seat_head(),
        "packages": list, "open": open.name(), "label": open.label(),
        "spots": spots_open, "pool": pool,
        "line": line, "line_snaps": total.round(),
        "series": 12,
        "rules": [
            "A whole-series man stays on when he tires: the drive is his.",
            "Every other series: he plays one and sits the next.",
            "Rush downs only: third and long, second and long and the two-minute drill.",
            "Your staff's call: the engine spells the tired man for the next one at the spot.",
        ],
    }))
}

/// The second or third man at a package's spot (`n` 2 or 3; id 0 gives
/// it back to the staff).
pub(super) fn set(w: &mut World, a: &Value) -> Result<(), String> {
    head(w)?;
    let b = a.get("board").and_then(Value::as_str).and_then(Board::parse).ok_or("No such package.")?;
    let slot = a.get("slot").and_then(Value::as_u64).ok_or("Missing spot.")? as usize;
    let n = a.get("n").and_then(Value::as_u64).ok_or("Second or third?")? as usize;
    if !(2..=3).contains(&n) || slot >= b.slots().len() {
        return Err("No such spot.".into());
    }
    let id = a.get("id").and_then(Value::as_u64).unwrap_or(0) as u32;
    if id != 0 && person_index(w, id).is_none() {
        return Err("No such player.".into());
    }
    let len = 2 * b.slots().len();
    let mut v = picks_of(w, b);
    v.resize(len, 0);
    v[2 * slot + n - 2] = id;
    w.plans.depth.retain(|(x, _)| *x != b);
    if v.iter().any(|&x| x != 0) {
        w.plans.depth.push((b, v));
    }
    w.refresh_seat_team();
    Ok(())
}

/// A lineman's role in the rotation: 0 the staff's, 1 the whole series, 2
/// every other series, 3 rush downs only.
pub(super) fn rotation(w: &mut World, a: &Value) -> Result<(), String> {
    head(w)?;
    let id = a.get("id").and_then(Value::as_u64).ok_or("Missing player.")? as u32;
    let role = a.get("role").and_then(Value::as_u64).unwrap_or(0).min(3) as u8;
    let i = person_index(w, id).ok_or("No such player.")?;
    if !matches!(w.people[i].player.position, Position::EDGE | Position::DT) {
        return Err("The rotation is the defensive line's.".into());
    }
    w.plans.rotation.retain(|r| r.0 != id);
    if role != 0 {
        w.plans.rotation.push((id, role));
    }
    w.refresh_seat_team();
    Ok(())
}
