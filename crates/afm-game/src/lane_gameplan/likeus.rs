//! How they played teams like us (G22, board 26: game day's Report): the
//! games on your next opponent's tapes against offenses (or defenses) most
//! like yours, what they did differently in those games, what worked
//! against them, what changed after halftime, and your coordinator's
//! suggestion for the sheet.

use afm_engine::game::playbook::{Coverage, PASS_CONCEPTS, RUN_CONCEPTS};
use afm_engine::game::sheet;
use afm_engine::training::seat::{SheetKind, SheetPlay};
use afm_engine::world::World;
use afm_engine::world::plans::{self, Play, WET};
use afm_engine::world::tape::{Snap, Tape};
use serde_json::{Value, json};

use super::{head, is_pass, is_snap, lvl, pct, season_games, seat, team_link};
use crate::Reply;
use crate::lane_training::{backfield_of, personnel_of, set_of};
use crate::views::date_text;

/// An offense's habits: run share, 11, 12, 10 and 21 personnel, the gun,
/// play action on throws.
type Shape = [f64; 7];

fn off_shape(snaps: &[(&str, bool, bool)]) -> Option<Shape> {
    // (look, run, play action)
    let n = snaps.len();
    if n < 25 {
        return None;
    }
    let share = |f: &dyn Fn(&(&str, bool, bool)) -> bool| snaps.iter().filter(|x| f(x)).count() as f64 / n as f64;
    let passes = snaps.iter().filter(|x| !x.1).count().max(1) as f64;
    Some([
        share(&|x| x.1),
        share(&|x| x.0.starts_with("11")),
        share(&|x| x.0.starts_with("12")),
        share(&|x| x.0.starts_with("10")),
        share(&|x| x.0.starts_with("21")),
        share(&|x| x.0.contains(" gun ") || x.0.contains("gun")),
        snaps.iter().filter(|x| !x.1 && x.2).count() as f64 / passes,
    ])
}

/// A defense's habits: man on throws, five or more rushers, two high on
/// first down, Cover 3 (zone single high) on third.
fn def_shape(snaps: &[(bool, bool, bool, u8, bool)]) -> Option<Shape> {
    // (man, blitz, two high, down, a throw)
    let n = snaps.len();
    if n < 25 {
        return None;
    }
    let thr = snaps.iter().filter(|x| x.4).count().max(1) as f64;
    let first = snaps.iter().filter(|x| x.3 == 1).count().max(1) as f64;
    Some([
        snaps.iter().filter(|x| x.4 && x.0).count() as f64 / thr,
        snaps.iter().filter(|x| x.1).count() as f64 / n as f64,
        snaps.iter().filter(|x| x.3 == 1 && x.2).count() as f64 / first,
        0.0,
        0.0,
        0.0,
        0.0,
    ])
}

fn similarity(a: &Shape, b: &Shape, weights: &[f64; 7]) -> f64 {
    let d: f64 = a.iter().zip(b).zip(weights).map(|((x, y), w)| w * (x - y).powi(2)).sum::<f64>().sqrt();
    (1.0 - d / 0.6).clamp(0.0, 1.0)
}

fn two_high(cov: &str) -> bool {
    Coverage::ALL.iter().find(|c| c.name() == cov).is_some_and(|c| c.high() == 2)
}

fn tape_snaps(t: &Tape, off: u8) -> impl Iterator<Item = &Snap> {
    t.snaps.iter().filter(move |s| s.off == off && s.kind <= 1 && !s.null)
}

fn rate(v: &[&Snap], f: &dyn Fn(&Snap) -> bool, of: &dyn Fn(&Snap) -> bool) -> Option<f64> {
    let d: Vec<&&Snap> = v.iter().filter(|s| of(s)).collect();
    (d.len() >= 5).then(|| d.iter().filter(|s| f(s)).count() as f64 / d.len() as f64)
}

fn per(v: &[&Snap], of: &dyn Fn(&Snap) -> bool) -> Option<f64> {
    let d: Vec<&&Snap> = v.iter().filter(|s| of(s)).collect();
    (d.len() >= 5).then(|| d.iter().map(|s| s.yards as f64).sum::<f64>() / d.len() as f64)
}

pub(super) fn view(w: &World, a: &Value) -> Reply {
    let s = seat(w)?;
    let defenses = a.get("side").and_then(Value::as_str) == Some("defense");
    let Some(next) = plans::next_game(w) else {
        return Ok(json!({ "has": false, "level": lvl(s.level), "why": "No game left on your schedule." }));
    };
    let opp = next.opp;
    let opp_name = plans::team_name(w, s.level, opp);
    // Our own shape from this season's games.
    let games = season_games(w, &s);
    let ours: Option<Shape> = if defenses {
        let v: Vec<(bool, bool, bool, u8, bool)> = games
            .iter()
            .flat_map(|(g, side)| {
                g.stats.plays.iter().filter(move |r| r.offense != *side && !r.nullified && is_snap(r.kind)).map(|r| {
                    let cov = r.defense_call.split(',').next().unwrap_or("");
                    (r.coverage_man, r.blitz, two_high(cov), r.down, is_pass(r.kind))
                })
            })
            .collect();
        def_shape(&v)
    } else {
        let v: Vec<(&str, bool, bool)> = games
            .iter()
            .flat_map(|(g, side)| {
                g.stats
                    .plays
                    .iter()
                    .filter(move |r| r.offense == *side && !r.nullified && is_snap(r.kind))
                    .map(|r| (r.call.as_str(), r.kind == afm_engine::game::PlayType::Run, r.play_action))
            })
            .collect();
        off_shape(&v)
    };
    let weights: [f64; 7] =
        if defenses { [1.5, 1.5, 1.0, 0.0, 0.0, 0.0, 0.0] } else { [2.0, 1.0, 1.0, 1.0, 1.0, 0.6, 0.8] };
    // Their tapes: each game, the other team's shape against ours.
    let tapes: Vec<&Tape> = w.gameday.tapes.iter().filter(|t| t.teams.contains(&opp)).collect();
    let mut rows: Vec<(f64, &Tape, u8)> = Vec::new(); // similarity, tape, the opponent's side
    for t in &tapes {
        let them = if t.teams[0] == opp { 0u8 } else { 1u8 };
        let other = 1 - them;
        let shape = if defenses {
            let v: Vec<(bool, bool, bool, u8, bool)> =
                tape_snaps(t, them).map(|x| (x.man, x.blitz, two_high(&x.coverage), x.down, x.kind == 1)).collect();
            def_shape(&v)
        } else {
            let v: Vec<(&str, bool, bool)> =
                tape_snaps(t, other).map(|x| (x.look.as_str(), x.kind == 0, x.play_action)).collect();
            off_shape(&v)
        };
        let sim = match (ours.as_ref(), shape.as_ref()) {
            (Some(o), Some(sh)) => similarity(o, sh, &weights),
            _ => 0.0,
        };
        rows.push((sim, t, them));
    }
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    let like: Vec<&(f64, &Tape, u8)> = rows.iter().filter(|r| r.0 >= 0.55).take(4).collect();
    // The opponent's side of the ball we look at: their defense against
    // offenses like ours, their offense against defenses like ours.
    fn their(t: &Tape, them: u8, defenses: bool) -> Vec<&Snap> {
        if defenses { tape_snaps(t, them).collect() } else { tape_snaps(t, 1 - them).collect() }
    }
    let all_snaps: Vec<&Snap> = rows.iter().flat_map(|(_, t, th)| their(t, *th, defenses)).collect();
    let like_snaps: Vec<&Snap> = like.iter().flat_map(|(_, t, th)| their(t, *th, defenses)).collect();
    let mut cmp: Vec<Value> = Vec::new();
    let mut row = |k: &str, unit: &str, f: &dyn Fn(&[&Snap]) -> Option<f64>, scale: f64| {
        let (u, l) = (f(&all_snaps), f(&like_snaps));
        if let (Some(u), Some(l)) = (u, l) {
            let fmt = |x: f64| if scale == 100.0 { format!("{:.0}%", 100.0 * x) } else { format!("{x:.1}") };
            cmp.push(json!({ "k": k, "unit": unit, "usual": fmt(u), "like": fmt(l), "diff": ((l - u) * scale * 10.0).round() / 10.0, "big": ((l - u) * scale).abs() >= if scale == 100.0 { 8.0 } else { 0.8 } }));
        }
    };
    if !defenses {
        row("Two high on 1st down", "of snaps", &|v| rate(v, &|s| two_high(&s.coverage), &|s| s.down == 1), 100.0);
        row("Man coverage", "of throws", &|v| rate(v, &|s| s.man, &|s| s.kind == 1), 100.0);
        row("Five or more rushers", "of snaps", &|v| rate(v, &|s| s.blitz, &|_| true), 100.0);
        row(
            "Cover 3 on 3rd down",
            "of snaps",
            &|v| rate(v, &|s| s.coverage.starts_with("Cover 3"), &|s| s.down == 3),
            100.0,
        );
        row("Yards a carry allowed", "yards", &|v| per(v, &|s| s.kind == 0), 1.0);
        row("Play-action yards a throw", "yards", &|v| per(v, &|s| s.kind == 1 && s.play_action), 1.0);
    } else {
        row("Runs on 1st down", "of snaps", &|v| rate(v, &|s| s.kind == 0, &|s| s.down == 1), 100.0);
        row("Yards a carry", "yards", &|v| per(v, &|s| s.kind == 0), 1.0);
        row("Yards a throw", "yards", &|v| per(v, &|s| s.kind == 1), 1.0);
        row("Play action", "of throws", &|v| rate(v, &|s| s.play_action, &|s| s.kind == 1), 100.0);
        row("Plays of 20 yards", "of snaps", &|v| rate(v, &|s| s.yards >= 20, &|_| true), 100.0);
        row("3rd downs converted", "of tries", &|v| rate(v, &|s| s.first || s.td, &|s| s.down == 3), 100.0);
    }
    // Game by game.
    let games_json: Vec<Value> = rows
        .iter()
        .map(|(sim, t, them)| {
            let other = 1 - *them as usize;
            let v = their(t, *them, defenses);
            let ypp = if v.is_empty() { 0.0 } else { v.iter().map(|s| s.yards as f64).sum::<f64>() / v.len() as f64 };
            json!({
                "date": date_text(t.date), "vs": team_link(w, s.level, t.teams[other]), "vs_name": t.names[other],
                "sim": pct(*sim), "like": *sim >= 0.55 && like.iter().any(|l| std::ptr::eq(l.1, *t)),
                "score": format!("{}-{}", t.score[*them as usize], t.score[other]),
                "won": t.score[*them as usize] > t.score[other],
                "ypp": (ypp * 10.0).round() / 10.0,
                "man": rate(&v, &|s| s.man, &|s| s.kind == 1).map(pct), "rush": rate(&v, &|s| s.blitz, &|_| true).map(pct),
            })
        })
        .collect();
    // What worked: the like-us side's best plays (offense) or the
    // coverages that held them (defense).
    let mut worked: Vec<Value> = Vec::new();
    let mut suggest: Option<Value> = None;
    let draft = w.plans.draft.as_ref().filter(|d| d.game == next.date);
    let on_sheet = |name: &str| -> bool {
        draft.is_some_and(|d| {
            let list = if defenses { &d.def } else { &d.off };
            list.iter().filter(|e| e.marks & WET == 0).any(|e| match e.play {
                Play::Off(p) => match p.play {
                    SheetKind::Run(r) => name.starts_with(r.name()),
                    SheetKind::Pass(k) => PASS_CONCEPTS.get(k as usize).is_some_and(|c| name == c.name),
                    SheetKind::Design(_) => false,
                },
                Play::Def(c, _) => name == c.name(),
            })
        })
    };
    if !defenses {
        let mut by: Vec<(String, String, u32, f64, f64)> = Vec::new(); // play, look, n, yards, epa
        for (_, t, them) in &like {
            for x in tape_snaps(t, 1 - *them) {
                let name = x.play.trim_end_matches(" left").trim_end_matches(" right").to_string();
                if name.is_empty() {
                    continue;
                }
                match by.iter_mut().find(|b| b.0 == name) {
                    Some(b) => {
                        b.2 += 1;
                        b.3 += x.yards as f64;
                        b.4 += x.epa as f64;
                    }
                    None => by.push((name, x.look.clone(), 1, x.yards as f64, x.epa as f64)),
                }
            }
        }
        by.retain(|b| b.2 >= 3);
        by.sort_by(|a, b| (b.4 / b.2 as f64).total_cmp(&(a.4 / a.2 as f64)));
        for b in by.iter().take(4) {
            let sheet_has = on_sheet(&b.0);
            worked.push(json!({ "name": b.0, "look": b.1, "n": b.2, "ypp": (b.3 / b.2 as f64 * 10.0).round() / 10.0, "on_sheet": sheet_has }));
            if suggest.is_none() && !sheet_has && sheet_play_of(&b.0, &b.1).is_some() {
                suggest = Some(
                    json!({ "name": b.0, "look": b.1, "text": format!("{} gained {:.1} a play on {} snaps for offenses like ours against them. Put it on the first-down list.", b.0, b.3 / b.2 as f64, b.2) }),
                );
            }
        }
    } else {
        let mut by: Vec<(String, u32, f64)> = Vec::new();
        for (_, t, them) in &like {
            // Their offense against defenses like ours: the coverage the
            // like-us defense played and what it gave up (their snaps).
            for x in tape_snaps(t, *them) {
                if x.coverage.is_empty() {
                    continue;
                }
                match by.iter_mut().find(|b| b.0 == x.coverage) {
                    Some(b) => {
                        b.1 += 1;
                        b.2 += x.yards as f64;
                    }
                    None => by.push((x.coverage.clone(), 1, x.yards as f64)),
                }
            }
        }
        by.retain(|b| b.1 >= 4);
        by.sort_by(|a, b| (a.2 / a.1 as f64).total_cmp(&(b.2 / b.1 as f64)));
        for b in by.iter().take(4) {
            worked.push(json!({ "name": b.0, "look": "coverage", "n": b.1, "ypp": (b.2 / b.1 as f64 * 10.0).round() / 10.0, "on_sheet": on_sheet(&b.0) }));
        }
    }
    // After halftime.
    let mut half: Vec<String> = Vec::new();
    if like_snaps.len() >= 30 {
        let first: Vec<&Snap> = like_snaps.iter().copied().filter(|s| s.q <= 2).collect();
        let second: Vec<&Snap> = like_snaps.iter().copied().filter(|s| s.q >= 3).collect();
        if !defenses {
            if let (Some(a), Some(b)) =
                (rate(&first, &|s| s.man, &|s| s.kind == 1), rate(&second, &|s| s.man, &|s| s.kind == 1))
                && (b - a).abs() >= 0.08
            {
                half.push(format!(
                    "Man coverage went from {:.0}% to {:.0}% of throws after halftime.",
                    100.0 * a,
                    100.0 * b
                ));
            }
            if let (Some(a), Some(b)) = (rate(&first, &|s| s.blitz, &|_| true), rate(&second, &|s| s.blitz, &|_| true))
                && (b - a).abs() >= 0.08
            {
                half.push(format!("Five or more rushers went from {:.0}% to {:.0}% of snaps.", 100.0 * a, 100.0 * b));
            }
        } else if let (Some(a), Some(b)) =
            (rate(&first, &|s| s.kind == 0, &|_| true), rate(&second, &|s| s.kind == 0, &|_| true))
            && (b - a).abs() >= 0.08
        {
            half.push(format!("They ran on {:.0}% of snaps before halftime and {:.0}% after.", 100.0 * a, 100.0 * b));
        }
        if half.is_empty() {
            half.push("Nothing much changed after halftime in those games.".into());
        }
    }
    let (_, dc) = plans::callers(w);
    let (oc, _) = plans::callers(w);
    let by = if defenses { dc } else { oc };
    Ok(json!({
        "has": true, "level": lvl(s.level), "head": w.seat_head(),
        "side": if defenses { "defense" } else { "offense" },
        "opp": team_link(w, s.level, opp), "opp_name": opp_name,
        "tapes": tapes.len(), "like_n": like.len(), "ours": ours.is_some(),
        "rows": cmp, "games": games_json, "worked": worked, "half": half,
        "suggest": suggest,
        "by": by.map(|c| if w.coaches[c].is_yours() { "You".to_string() } else { super::coach_link(w, c) }),
    }))
}

/// A tape's play and look as a sheet play (a library run or pass).
fn sheet_play_of(name: &str, look: &str) -> Option<SheetPlay> {
    let play = if let Some(r) = RUN_CONCEPTS.iter().copied().find(|r| name.starts_with(r.name())) {
        SheetKind::Run(r)
    } else {
        SheetKind::Pass(PASS_CONCEPTS.iter().position(|c| c.name == name)? as u16)
    };
    let words: Vec<&str> = look.split_whitespace().collect();
    let personnel = personnel_of(words.first().copied().unwrap_or("11"));
    let back = if look.contains("under center") {
        "under center"
    } else if look.contains("pistol") {
        "pistol"
    } else {
        "shotgun"
    };
    let set = words
        .iter()
        .rev()
        .find(|x| matches!(**x, "doubles" | "trips" | "bunch" | "stack" | "empty" | "wing"))
        .copied()
        .unwrap_or("doubles");
    Some(SheetPlay { personnel, backfield: backfield_of(back), set: set_of(set), play, hash: 0 })
}

/// Your coordinator's suggestion taken: the play goes on the first-down
/// list, from either hash.
pub(super) fn add(w: &mut World, a: &Value) -> Result<(), String> {
    head(w)?;
    let name = a.get("name").and_then(Value::as_str).ok_or("Missing play.")?;
    let look = a.get("look").and_then(Value::as_str).unwrap_or("11 gun doubles");
    let p = sheet_play_of(name, look).ok_or("That play isn't in your playbook.")?;
    plans::add_to_box(w, 0, sheet::key(sheet::FIRST, 0), 0, Play::Off(p))
}
