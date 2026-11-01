//! V9 at an NFL club: the war room's data (the clock, the picks, the
//! offers, the reveal) and on top of it the board in tiers with the chance
//! each man is there at your pick, your staff's mock of the picks before
//! yours, your needs, the trade calls priced on your GM's chart and the
//! picks so far against your board.

use std::collections::{HashMap, HashSet};

use afm_engine::positions::Position;
use afm_engine::world::World;
use afm_engine::world::pro::scouts::Role;
use afm_engine::world::scouting::{self as sc, board, live, readers};
use serde_json::{Value, json};

use super::sim::{self, DRAFT_POS};
use crate::Reply;
use crate::lane_scouting::{row, war_room};
use crate::views::{TeamRef, team_brief};

/// The board's cards: your top so many, the gone ones struck through.
const CARDS: usize = 40;
/// Players on your board the analytics staff watches for a trade call.
const WATCH: usize = 15;

/// A starting spot's need, 0-100, from your staff's verdict on it.
fn verdict_need(v: &str) -> f64 {
    match v {
        "Hole" => 100.0,
        "Thin" => 75.0,
        "Average" => 45.0,
        "Solid" => 20.0,
        _ => 5.0,
    }
}

/// Your needs by position, highest first: each spot's verdict now (60%)
/// and next fall (40%), the weakest spot at a position deciding it, for the
/// positions a club drafts for (not the kicker or punter). Your staff's
/// reads of your own players (the needs map, V2).
pub(crate) fn own_needs(w: &World) -> Vec<(String, f64)> {
    let Ok(v) = crate::lane_squadboards::needs::view(w, &json!({})) else { return Vec::new() };
    let mut by: Vec<(String, f64)> = Vec::new();
    for s in v["spots"].as_array().into_iter().flatten() {
        let pos = s["pos"].as_str().unwrap_or("").to_string();
        // The kicker and the punter are needs too, but not the board's fit.
        if !DRAFT_POS.iter().any(|p| p.name() == pos) {
            continue;
        }
        let now = verdict_need(s["verdicts"][0].as_str().unwrap_or(""));
        let next = verdict_need(s["verdicts"][1].as_str().unwrap_or(""));
        let n = 0.6 * now + 0.4 * next;
        match by.iter_mut().find(|x| x.0 == pos) {
            Some(x) => x.1 = x.1.max(n),
            None => by.push((pos, n)),
        }
    }
    by.sort_by(|a, b| b.1.total_cmp(&a.1));
    by
}

/// A club's top needs as anyone reads them off its depth chart.
fn public_needs(w: &World, club: usize) -> Vec<&'static str> {
    let mut v: Vec<(f64, Position)> = DRAFT_POS.iter().map(|&p| (sim::need(w, club, p), p)).collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    v.into_iter().filter(|x| x.0 > 0.0).take(3).map(|x| x.1.name()).collect()
}

fn director(w: &World, t: usize) -> String {
    w.clubs
        .get(t)
        .and_then(|c| c.scouts.iter().find(|s| s.role == Role::Director))
        .map_or("Your director".into(), |s| s.name.clone())
}

fn last_name(n: &str) -> &str {
    n.rsplit(' ').next().unwrap_or(n)
}

pub fn view(w: &World) -> Reply {
    let mut out = war_room(w)?;
    let Some(l) = &w.scouting.live else { return Ok(out) };
    let me = l.team;
    let run = &l.run;
    let now = (run.next as usize).saturating_sub(1);
    // Your next pick (board index): the one on the clock if it's yours.
    let target = (now..run.board.len()).find(|&k| run.board[k].0 == me);
    let teams = afm_engine::world::franchise::active_teams_in(w, run.year).len().max(1);
    // Your board's grades for everyone still there and everyone taken, so a
    // man keeps his number on the board when he's gone.
    let mine = run.boards.get(me);
    let mut grades: HashMap<u32, f64> = live::grades(w, l);
    for &(i, _, _, _) in &run.picked {
        if let Some(&g) = mine.and_then(|b| b.grade.get(&i)) {
            grades.insert(w.people[i].player.id, g);
        }
    }
    let order = board::ordered(w, &grades);
    let picked: HashMap<usize, (u32, usize)> = run.picked.iter().map(|&(i, pk, t, _)| (i, (pk, t))).collect();
    let public: HashMap<u32, usize> = run.public_rank.iter().map(|(&i, &r)| (w.people[i].player.id, r)).collect();
    let b = &w.scouting.board;
    let needs = own_needs(w);
    let top3: HashSet<&str> = needs.iter().take(3).map(|x| x.0.as_str()).collect();
    // The staff's mock: to your pick and to each slot a club offers you.
    let mut checks: Vec<usize> = target.into_iter().collect();
    for o in &l.offers {
        checks.push(o.their_slot);
    }
    let left_ids: Vec<(usize, u32)> = order.iter().filter_map(|&id| sc::place(w, id).map(|i| (i, id))).collect();
    let watch: Vec<usize> = left_ids
        .iter()
        .filter(|(i, id)| {
            !picked.contains_key(i) && !b.is_dnd(*id) && top3.contains(w.people[*i].player.position.name())
        })
        .take(WATCH)
        .map(|x| x.0)
        .collect();
    let mock = sim::run(w, run, me, &checks, &watch, me as u64 * 7919 + run.year as u64);
    let there = |i: usize| match target {
        Some(t) if !picked.contains_key(&i) => Some(mock.there_at(t, i)),
        _ => None,
    };
    // The cards and the tiers.
    let staff = readers::Staff::Club(me);
    let mut cards: Vec<Value> = Vec::new();
    let mut tiers: Vec<(u8, u32, f64)> = Vec::new();
    for (k, &(i, id)) in left_ids.iter().take(CARDS).enumerate() {
        let p = &w.people[i];
        let tier = b.tier_of(id).unwrap_or_else(|| board::tier_of_rank(k, teams));
        let gone = picked.get(&i);
        let pub_rank = public.get(&id).copied();
        let medical = readers::read(w, staff, id).map(|(_, _, f)| f.medical.grade);
        let character = mine.and_then(|b| b.risk.get(&i)).copied();
        let mut flags = Vec::new();
        if w.scouting.predraft.lists.iter().any(|x| x.0 == id && x.1 == 0) {
            flags.push("V");
        }
        if medical.is_some_and(|g| g >= 2) {
            flags.push("M");
        }
        if character.is_some_and(|r| r >= 1.0) {
            flags.push("C");
        }
        let th = there(i);
        let mut r = row(w, i);
        r["rank"] = json!(k + 1);
        r["tier"] = json!(tier);
        r["grade_words"] = json!(board::round_words(tier));
        r["public"] = json!(pub_rank.map(|x| x + 1));
        r["limb"] = json!(board::limb(k, pub_rank));
        r["dnd"] = json!(b.is_dnd(id));
        r["fit"] = json!(gone.is_none() && top3.contains(p.player.position.name()));
        r["flags"] = json!(flags);
        r["character"] = json!(match character {
            Some(r) if r >= 1.0 => "Concern",
            Some(r) if r >= 0.5 => "Some questions",
            Some(_) => "No concerns",
            None => "Not read yet",
        });
        r["there"] = json!(th.map(|x| (x * 100.0).round()));
        r["gone"] = json!(gone.map(|&(pk, t)| json!({ "pick": pk, "team": team_brief(w, TeamRef::Nfl(t)) })));
        cards.push(r);
        match tiers.iter_mut().find(|x| x.0 == tier) {
            Some(x) => {
                if gone.is_none() {
                    x.1 += 1;
                    x.2 += th.unwrap_or(1.0);
                }
            }
            None => tiers.push((tier, u32::from(gone.is_none()), if gone.is_none() { th.unwrap_or(1.0) } else { 0.0 })),
        }
    }
    tiers.sort_by_key(|x| x.0);
    let tiers: Vec<Value> = tiers
        .iter()
        .map(|&(t, left, exp)| {
            let (name, desc) = board::TIERS[(t as usize - 1).min(7)];
            json!({ "tier": t, "name": name, "desc": desc, "left": left, "exp": (exp * 10.0).round() / 10.0 })
        })
        .collect();
    // Before your pick: the staff's mock of each slot, with the club's needs.
    let away = target.map_or(0, |t| t - now);
    let strip: Vec<Value> = target
        .map(|t| now..t)
        .into_iter()
        .flatten()
        .filter(|&k| run.board[k].0 != me)
        .take(6)
        .map(|k| {
            let club = run.board[k].0;
            let guess = mock.likely(k).map(|(i, share)| {
                let mut r = row(w, i);
                r["share"] = json!((share * 100.0).round());
                r["rank"] = json!(order.iter().position(|&x| x == w.people[i].player.id).map(|x| x + 1));
                r
            });
            json!({
                "pick": k + 1, "slot": k, "round": run.board[k].1, "team": team_brief(w, TeamRef::Nfl(club)),
                "needs": public_needs(w, club), "guess": guess, "on_clock": k == now,
            })
        })
        .collect();
    let left_cards: Vec<&Value> = cards.iter().filter(|c| c["gone"].is_null() && c["dnd"] == json!(false)).collect();
    let exp_gone: f64 = left_cards.iter().map(|c| 1.0 - c["there"].as_f64().unwrap_or(100.0) / 100.0).sum();
    // Your pick: the best need-fit likely there, else the best likely there.
    let likely = |c: &&&Value| c["there"].as_f64().unwrap_or(100.0) >= 50.0;
    let ours = left_cards
        .iter()
        .filter(likely)
        .find(|c| c["fit"] == json!(true))
        .map(|c| (c, true))
        .or_else(|| left_cards.iter().find(likely).map(|c| (c, false)))
        .map(|(c, fit)| json!({ "id": c["id"], "name": c["name"], "pos": c["pos"], "there": c["there"], "fit": fit }));
    // The trade calls on your GM's chart, with what the room says.
    let classic = w.clubs.get(me).is_none_or(|c| c.office.gm.classic_chart);
    let chart = |slot: usize| afm_engine::world::pro::trades::chart(classic, slot as f64 + 1.0);
    let future = |r: u32| {
        let mid = ((r - 1) as usize * teams + teams / 2) as f64;
        0.85 * afm_engine::world::pro::trades::chart(classic, mid)
    };
    let need_word = needs.first().map_or("starter".to_string(), |n| n.0.clone());
    let calls: Vec<Value> = l
        .offers
        .iter()
        .enumerate()
        .map(|(k, o)| {
            let give = chart(now);
            let get = chart(o.their_slot) + o.slot_extra.map_or(0.0, chart) + o.future.map_or(0.0, future);
            let any = mock.any_at(o.their_slot);
            let tens = (any * 10.0).round() as u32;
            let says = if watch.is_empty() {
                json!({ "who": director(w, me), "says": "Nobody we need is on the board anyway. Take the value." })
            } else if tens >= 6 {
                json!({ "who": "Analytics", "says": format!("A need-fit is still there at {} in {tens} of 10 of our mocks.", o.their_slot + 1) })
            } else {
                json!({ "who": last_name(&director(w, me)).to_string(), "says": format!("Too far down for the {need_word} we need: {tens} of 10 mocks have one left at {}.", o.their_slot + 1) })
            };
            json!({
                "k": k, "from": team_brief(w, TeamRef::Nfl(o.from)), "their_pick": o.their_slot + 1,
                "give": give.round(), "get": get.round(), "need_fit": (any * 100.0).round(), "line": says,
            })
        })
        .collect();
    // The picks so far, newest first, against your board.
    let picks: Vec<Value> = run
        .picked
        .iter()
        .rev()
        .take(40)
        .map(|&(i, pk, t, _)| {
            let id = w.people[i].player.id;
            let rank = order.iter().position(|&x| x == id).map(|x| x + 1);
            json!({
                "pick": pk, "team": team_brief(w, TeamRef::Nfl(t)), "mine": t == me,
                "id": id, "name": w.people[i].name, "pos": w.people[i].player.position.name(),
                "board": rank, "early": rank.is_some_and(|r| r as i64 - pk as i64 >= 4),
            })
        })
        .collect();
    out["dr"] = json!({
        "target": target.map(|t| t + 1),
        "away": away,
        "cards": cards,
        "tiers": tiers,
        "strip": strip,
        "strip_more": target.map_or(0, |t| (now..t).filter(|&k| run.board[k].0 != me).count().saturating_sub(6)),
        "ours": ours,
        "gone_note": { "left": left_cards.len(), "likely_go": (exp_gone * 10.0).round() / 10.0, "off_board": ((away as f64 - exp_gone).max(0.0) * 10.0).round() / 10.0 },
        "needs": needs.iter().take(6).enumerate().map(|(k, n)| json!({ "pos": n.0, "v": n.1.round(), "top": k < 3 })).collect::<Vec<_>>(),
        "calls": calls,
        "picks": picks,
        "runs": sim::RUNS,
        "chart": if classic { "classic" } else { "surplus" },
    });
    Ok(out)
}
