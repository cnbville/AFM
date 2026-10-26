//! The week's plan against one opponent (G16-G21, board 25: game day's
//! Game plan tab): your answer to each of their personnel groups, snap
//! limits, the "if they do this, we go to that" lines, instructions for
//! this opponent, the quarterback's checks, special teams, the keys to the
//! game and last game's keys checked, and the game rules from your
//! identity with this game's changes.

use afm_engine::game::Film;
use afm_engine::game::calling::Answer;
use afm_engine::game::flow::fg_make;
use afm_engine::game::guide;
use afm_engine::game::playbook::{Personnel, SET_COUNT};
use afm_engine::game::roster::{CHECKS, IFS};
use afm_engine::positions::Position;
use afm_engine::world::World;
use afm_engine::world::plans::{self, ORDERS};
use serde_json::{Value, json};

use super::rules::rules_json;
use super::{head, lvl, pct, seat, team_link};
use crate::Reply;
use crate::lane_training::practice_status;
use crate::views::{date_text, person_index};

const GROUPS: [Personnel; 4] = [Personnel::P11, Personnel::P12, Personnel::P10, Personnel::P21];
const BASE_ORDER: [Personnel; 6] =
    [Personnel::P11, Personnel::P12, Personnel::P21, Personnel::P10, Personnel::P22, Personnel::P13];
const PKG: [&str; 4] = ["staff", "base", "nickel", "dime"];

/// Their offense's snaps by personnel group from the film: (snaps, runs).
fn by_group(f: &Film, p: Personnel) -> (f64, f64) {
    let k = BASE_ORDER.iter().position(|&x| x == p).unwrap_or(0);
    let (mut n, mut r) = (0.0, 0.0);
    for (look, v) in f.look.iter().enumerate() {
        if look / (3 * SET_COUNT) == k {
            n += (v[0] + v[1]) as f64;
            r += v[0] as f64;
        }
    }
    (n, r)
}

fn checks_on(bits: u8) -> [bool; 4] {
    [bits & 1 == 0, bits & 2 == 0, bits & 4 != 0, bits & 8 != 0]
}

fn bits_of(on: [bool; 4]) -> u8 {
    (!on[0] as u8) | (!on[1] as u8) << 1 | (on[2] as u8) << 2 | (on[3] as u8) << 3
}

/// The groups an instruction is for.
fn order_positions(k: usize) -> &'static [Position] {
    match ORDERS[k].1 {
        "QB" => &[Position::QB],
        "ball carriers" => &[Position::RB, Position::WR, Position::TE],
        "pass rushers" => &[Position::EDGE, Position::DT],
        "coverage" => &[Position::CB, Position::S, Position::LB],
        _ => &[Position::LB, Position::S, Position::CB, Position::EDGE, Position::DT],
    }
}

fn order_name(k: usize) -> &'static str {
    [
        "Slide early",
        "Protect the ball",
        "Take your shots",
        "Ball high and tight",
        "Keep contain",
        "Stay home",
        "Wrap up, no big hits",
    ][k]
}

pub(super) fn view(w: &World) -> Reply {
    let s = seat(w)?;
    let Some(next) = plans::next_game(w) else {
        return Ok(json!({ "has": false, "level": lvl(s.level), "why": "No game left on your schedule." }));
    };
    let film = plans::opp_film(w, s.level, next.opp);
    let wk = &w.plans.week;
    let this = wk.game == Some(next.date);
    let st = &s.team.staff;
    let man_own = guide::expected_man(st.shell, &st.guide);
    let rush_own = st.guide.pressure.pull().map_or(guide::habit_pressure(st.defense.aggressiveness), |(t, k)| {
        guide::expected(guide::habit_pressure(st.defense.aggressiveness), t, k)
    });
    let total: f64 = GROUPS.iter().map(|&p| by_group(&film, p).0).sum::<f64>().max(1.0);
    let all: f64 = film.look.iter().map(|v| (v[0] + v[1]) as f64).sum::<f64>().max(1.0);
    let answers: Vec<Value> = GROUPS
        .iter()
        .enumerate()
        .map(|(k, &p)| {
            let (n, r) = by_group(&film, p);
            let a: Answer = if this { wk.answers[k] } else { Answer::default() };
            let staff_pkg = match p {
                Personnel::P10 => "dime",
                Personnel::P11 => "nickel",
                _ => "base",
            };
            let does = if n < 8.0 {
                "seldom on their film".to_string()
            } else {
                format!("run {:.0}% · throw {:.0}%", 100.0 * r / n, 100.0 * (n - r) / n)
            };
            json!({
                "k": k, "name": p.name(), "share": pct(n / all), "snaps": n.round(), "does": does,
                "pkg": PKG[(a.pkg as usize).min(3)], "man": a.man.map(|x| pct(x as f64)), "rush": a.rush.map(|x| pct(x as f64)),
                "staff": { "pkg": staff_pkg, "man": pct(man_own), "rush": pct(rush_own) },
                "of_four": pct(n / total),
            })
        })
        .collect();
    // Snap limits: the two-deep, the ones your trainers flag first.
    let mut limit_rows: Vec<Value> = Vec::new();
    for (pos, ids) in &s.team.order {
        for &id in ids.iter().take(if pos.is_specialist() { 1 } else { 2 }) {
            let Some(i) = person_index(w, id) else { continue };
            if limit_rows.iter().any(|x| x["id"] == id) {
                continue;
            }
            let (practice, note) = practice_status(w, id, i);
            let limit = if this { wk.limits.iter().find(|l| l.0 == id).map(|l| l.1) } else { None };
            if practice == "full" && limit.is_none() {
                continue;
            }
            limit_rows.push(json!({ "id": id, "name": w.people[i].name, "pos": pos.name(), "practice": practice, "note": note, "limit": limit }));
        }
    }
    let two_deep: Vec<Value> = s
        .team
        .order
        .iter()
        .flat_map(|(pos, ids)| ids.iter().take(2).map(move |&id| (*pos, id)))
        .filter_map(|(pos, id)| {
            person_index(w, id).map(|i| json!({ "id": id, "name": w.people[i].name, "pos": pos.name() }))
        })
        .collect();
    // The "if they do this" lines.
    let staff_ifs = plans::staff_ifs(&film);
    let armed: Vec<u8> = wk.ifs.clone().filter(|_| this).unwrap_or_else(|| staff_ifs.clone());
    let ifs: Vec<Value> = IFS
        .iter()
        .enumerate()
        .map(|(k, (when, then))| json!({ "k": k, "when": when, "then": then, "on": armed.contains(&(k as u8)), "staff": staff_ifs.contains(&(k as u8)) }))
        .collect();
    // The quarterback's checks.
    let staff_bits = plans::staff_checks(&film);
    let bits = wk.checks.filter(|_| this).unwrap_or(staff_bits);
    let on = checks_on(bits);
    let staff_on = checks_on(staff_bits);
    let checks: Vec<Value> = CHECKS
        .iter()
        .enumerate()
        .map(|(k, t)| json!({ "k": k, "text": t, "on": on[k], "staff": staff_on[k] }))
        .collect();
    // Instructions for this opponent.
    let roster_at = |ps: &[Position]| -> Vec<Value> {
        s.team
            .order
            .iter()
            .filter(|(p, _)| ps.contains(p))
            .flat_map(|(_, ids)| ids.iter().take(3).copied())
            .filter_map(|id| {
                person_index(w, id)
                    .map(|i| json!({ "id": id, "name": w.people[i].name, "pos": w.people[i].player.position.name() }))
            })
            .collect()
    };
    let orders: Vec<Value> = ORDERS
        .iter()
        .enumerate()
        .map(|(k, (_, group))| {
            let given: Vec<Value> = if this {
                wk.orders
                    .iter()
                    .filter(|o| o.1 as usize == k)
                    .filter_map(|o| person_index(w, o.0).map(|i| json!({ "id": o.0, "name": w.people[i].name })))
                    .collect()
            } else {
                Vec::new()
            };
            json!({ "k": k, "name": order_name(k), "for": group, "given": given, "players": roster_at(order_positions(k)) })
        })
        .collect();
    // Special teams.
    let t = if this { wk.teams } else { Default::default() };
    let kicker = s
        .team
        .players
        .iter()
        .filter(|p| p.position == Position::K)
        .max_by(|a, b| fg_make(a, 40.0).total_cmp(&fg_make(b, 40.0)));
    let range = kicker.map(|k| (20..=65).rev().find(|&d| fg_make(k, d as f64) >= 0.5).unwrap_or(20));
    let teams = json!({
        "kick": t.kick, "punt": t.punt, "ret": t.ret, "pret": t.pret, "fake": t.fake, "onside": t.onside,
        "fg_range": range.map(|d| format!("{d} yards: their {}", (d - 17).max(1))),
    });
    // The keys.
    let keys: Vec<Value> =
        if this { wk.keys.iter().map(|k| json!({ "kind": k.kind, "text": k.text })).collect() } else { Vec::new() };
    let last = w.plans.keys.last().map(|k| {
        json!({ "date": date_text(k.date), "opp": k.opp, "keys": k.keys.iter().map(|(t, met, what)| json!({ "text": t, "met": met, "what": what })).collect::<Vec<_>>(), "met": k.keys.iter().filter(|x| x.1).count(), "of": k.keys.len() })
    });
    let plan_day = plans::plan_day(s.level, next.date);
    let d = w.plans.draft.as_ref().filter(|d| d.game == next.date && d.opp == next.opp);
    let waiting = d.map_or(0, |d| d.status.iter().flatten().filter(|&&x| x == 0).count());
    Ok(json!({
        "has": true, "level": lvl(s.level), "head": w.seat_head(),
        "game": date_text(next.date), "opp": team_link(w, s.level, next.opp), "opp_name": plans::team_name(w, s.level, next.opp), "home": next.home,
        "locked": this && wk.locked,
        "film_snaps": (film.off_snaps + film.def_snaps).round(),
        "answers": answers,
        "limits": limit_rows, "two_deep": two_deep,
        "ifs": ifs, "checks": checks, "orders": orders, "teams": teams,
        "keys": keys, "keys_day": date_text(plan_day), "last_keys": last,
        "rules": rules_json(&w.gameday.rules),
        "week_rules": rules_json(&w.gameday.rules_for(next.date)),
        "rules_changed": matches!(w.gameday.week_rules, Some((dd, _)) if dd == next.date),
        "sheet": { "drafted": d.is_some(), "waiting": waiting },
    }))
}

fn pick_share(v: Option<&Value>) -> Option<Option<f32>> {
    match v {
        None => None,
        Some(Value::Null) => Some(None),
        Some(x) => x.as_f64().map(|p| Some((p / 100.0).clamp(0.0, 1.0) as f32)),
    }
}

/// One change to the week's plan.
pub(super) fn set(w: &mut World, a: &Value) -> Result<(), String> {
    head(w)?;
    let next = plans::next_game(w).ok_or("No game left on your schedule.")?;
    let level = w.seat_team().map(|x| x.0).ok_or("You have no team.")?;
    let film = plans::opp_film(w, level, next.opp);
    let staff_ifs = plans::staff_ifs(&film);
    let staff_bits = plans::staff_checks(&film);
    let wk = plans::week_mut(w);
    if let Some(x) = a.get("answer") {
        let k = x.get("k").and_then(Value::as_u64).ok_or("Missing group.")? as usize;
        let ans = wk.answers.get_mut(k).ok_or("No such group.")?;
        if let Some(p) = x.get("pkg").and_then(Value::as_str) {
            ans.pkg = PKG.iter().position(|&q| q == p).unwrap_or(0) as u8;
        }
        if let Some(m) = pick_share(x.get("man")) {
            ans.man = m;
        }
        if let Some(r) = pick_share(x.get("rush")) {
            ans.rush = r.map(|v| v.min(0.7));
        }
    }
    if let Some(x) = a.get("limit") {
        let id = x.get("id").and_then(Value::as_u64).ok_or("Missing player.")? as u32;
        let n = x.get("n").and_then(Value::as_u64).unwrap_or(0) as u16;
        wk.limits.retain(|l| l.0 != id);
        if n > 0 {
            wk.limits.push((id, n.clamp(5, 120)));
        }
    }
    if let Some(x) = a.get("if") {
        let k = x.get("k").and_then(Value::as_u64).ok_or("Missing line.")? as u8;
        if k as usize >= IFS.len() {
            return Err("No such line.".into());
        }
        let on = x.get("on").and_then(Value::as_bool).unwrap_or(true);
        let mut v = wk.ifs.clone().unwrap_or_else(|| staff_ifs.clone());
        v.retain(|&q| q != k);
        if on {
            v.push(k);
            v.sort_unstable();
        }
        wk.ifs = Some(v);
    }
    if let Some(x) = a.get("check") {
        let k = x.get("k").and_then(Value::as_u64).ok_or("Missing check.")? as usize;
        if k >= CHECKS.len() {
            return Err("No such check.".into());
        }
        let mut on = checks_on(wk.checks.unwrap_or(staff_bits));
        on[k] = x.get("on").and_then(Value::as_bool).unwrap_or(!on[k]);
        wk.checks = Some(bits_of(on));
    }
    if let Some(x) = a.get("order") {
        let id = x.get("id").and_then(Value::as_u64).ok_or("Missing player.")? as u32;
        let k = x.get("k").and_then(Value::as_u64).ok_or("Missing instruction.")? as u8;
        if k as usize >= ORDERS.len() {
            return Err("No such instruction.".into());
        }
        let on = x.get("on").and_then(Value::as_bool).unwrap_or(true);
        wk.orders.retain(|o| !(o.0 == id && o.1 == k));
        if on {
            wk.orders.push((id, k));
        }
    }
    if let Some(x) = a.get("teams") {
        let g = |key: &str, cur: u8, max: u8| x.get(key).and_then(Value::as_u64).map_or(cur, |v| (v as u8).min(max));
        let t = &mut wk.teams;
        t.kick = g("kick", t.kick, 3);
        t.punt = g("punt", t.punt, 2);
        t.ret = g("ret", t.ret, 2);
        t.pret = g("pret", t.pret, 2);
        t.fake = g("fake", t.fake, 2);
        t.onside = g("onside", t.onside, 2);
    }
    if let Some(l) = a.get("lock").and_then(Value::as_bool) {
        wk.locked = l;
        if l {
            wk.checks = Some(wk.checks.unwrap_or(staff_bits));
            wk.ifs = Some(wk.ifs.clone().unwrap_or(staff_ifs));
        }
    }
    w.refresh_seat_team();
    Ok(())
}
