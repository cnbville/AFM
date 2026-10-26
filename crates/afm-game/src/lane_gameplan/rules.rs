//! The standing game rules and the call words (G4, G5; board 20, the
//! Identity tab's second half). The 4th-down chart with your own cells,
//! the two-point chart, the timeout rule and the four-minute plan; game
//! day starts from them each week and can change them for one game. The
//! call words: since when they are ours, who outside knows them, and what
//! a change costs.

use afm_engine::attributes::Attr;
use afm_engine::calendar::Level as Cal;
use afm_engine::game::calling::fourth_ev;
use afm_engine::game::flow::fg_make;
use afm_engine::game::live::{
    self, CHART_COLS, CHART_ROWS, FourMinute, FourthChart, Tempo, TimeoutRule, TwoChart, chart_says_two, win_prob,
};
use afm_engine::game::tables::real_fourth_go_rate;
use afm_engine::game::{GamePlayer, PlayType, Rules};
use afm_engine::positions::Position;
use afm_engine::world::World;
use afm_engine::world::live_seat::GameRules;
use afm_engine::world::plans;
use serde_json::{Value, json};

use super::{Seat, head, is_snap, lvl, season_games, season_words, seat, team_link};
use crate::Reply;
use crate::views::{date_text, person_index};

/// The chart's rows as yards to go and its columns as yards to the goal,
/// at each cell's middle.
const ROW_TOGO: [f64; 7] = [1.0, 2.0, 3.0, 4.5, 6.5, 9.0, 13.0];
const COL_YL: [f64; 9] = [90.0, 75.0, 65.0, 55.0, 45.0, 35.0, 25.0, 15.0, 5.0];

pub(crate) fn fourth_key(f: FourthChart) -> &'static str {
    match f {
        FourthChart::Staff => "staff",
        FourthChart::Numbers => "numbers",
        FourthChart::Aggressive => "aggressive",
        FourthChart::Conservative => "conservative",
    }
}

pub(crate) fn two_key(t: TwoChart) -> &'static str {
    match t {
        TwoChart::Staff => "staff",
        TwoChart::Chart => "chart",
        TwoChart::Kick => "kick",
    }
}

pub(crate) fn tempo_key(t: Tempo) -> &'static str {
    match t {
        Tempo::Normal => "normal",
        Tempo::Hurry => "hurry",
        Tempo::Milk => "milk",
    }
}

pub(crate) fn timeouts_key(t: TimeoutRule) -> &'static str {
    match t {
        TimeoutRule::Staff => "staff",
        TimeoutRule::KeepTwo => "keep_two",
        TimeoutRule::KeepOne => "keep_one",
        TimeoutRule::Free => "free",
    }
}

pub(crate) fn four_key(f: FourMinute) -> &'static str {
    match f {
        FourMinute::Staff => "staff",
        FourMinute::RunItOut => "run",
        FourMinute::KeepAttacking => "attack",
    }
}

/// Your field goal kicker: the best leg at 40 yards.
fn kicker(s: &Seat) -> Option<&GamePlayer> {
    s.team
        .players
        .iter()
        .filter(|p| p.position == Position::K)
        .max_by(|a, b| fg_make(a, 40.0).total_cmp(&fg_make(b, 40.0)))
        .or_else(|| s.team.players.iter().max_by(|a, b| a.a(Attr::KickPower).total_cmp(&b.a(Attr::KickPower))))
}

/// One cell's numbers, tied at the half: (convert, win chance going,
/// punting, kicking (0 out of range), the field goal's make).
fn cell_numbers(s: &Seat, r: usize, c: usize) -> (f64, f64, f64, f64, f64) {
    let (togo, yl) = (ROW_TOGO[r], COL_YL[c]);
    let dist = yl + 17.0;
    let make = if dist > 68.0 { 0.0 } else { kicker(s).map_or(0.0, |k| fg_make(k, dist)) };
    let (conv, go, punt, fg) = fourth_ev(yl, togo.min(yl), make);
    let rules = Rules::for_level(plans::level_of(s.level));
    let len = 4.0 * rules.quarter;
    let wp = |ev: f64| win_prob(0.0, ev, len / 2.0, len);
    (conv, wp(go), wp(punt), if make > 0.0 { wp(fg) } else { 0.0 }, make)
}

/// What a preset says in a cell: 1 go, 2 field goal, 3 punt (the engine's
/// own rule, `flow::chart_fourth`; the staff's call from the real go rates).
fn preset_cell(s: &Seat, f: FourthChart, r: usize, c: usize) -> u32 {
    let (_, go, punt, fg, make) = cell_numbers(s, r, c);
    let kick = if make > 0.3 && fg >= punt { (2, fg) } else { (3, punt) };
    match f {
        FourthChart::Staff => {
            let college = s.level != Cal::Nfl;
            let yl = COL_YL[c];
            if real_fourth_go_rate(ROW_TOGO[r].min(yl), yl, 0, college) >= 0.5 { 1 } else { kick.0 }
        }
        _ => {
            let edge = match f {
                FourthChart::Aggressive => -0.02,
                FourthChart::Conservative => 0.04,
                _ => 0.0,
            };
            if go >= kick.1 + edge { 1 } else { kick.0 }
        }
    }
}

fn own_cell(cells: &[u32; 7], r: usize, c: usize) -> u32 {
    (cells[r] >> (2 * c)) & 3
}

const CALL: [&str; 4] = ["", "Go", "FG", "Punt"];

fn grid(s: &Seat, r: &GameRules) -> (Vec<Value>, [u32; 3]) {
    let mut tally = [0u32; 3];
    let rows = (0..CHART_ROWS.len())
        .map(|ri| {
            let cells: Vec<Value> = (0..CHART_COLS.len())
                .map(|ci| {
                    let own = own_cell(&r.cells, ri, ci);
                    let base = preset_cell(s, r.fourth, ri, ci);
                    let v = if own != 0 { own } else { base };
                    tally[(v - 1) as usize] += 1;
                    let (conv, go, punt, fg, make) = cell_numbers(s, ri, ci);
                    let kick = if make > 0.3 && fg >= punt { ("the field goal", fg) } else { ("punting", punt) };
                    let d = 100.0 * (go - kick.1);
                    let why = if d >= 0.0 {
                        format!("Going wins {:.1} points of win chance more than {}.", d, kick.0)
                    } else {
                        format!("{} wins {:.1} points of win chance more than going.", capital(kick.0), -d)
                    };
                    json!({
                        "v": CALL[v as usize], "own": own != 0, "base": CALL[base as usize],
                        "why": why, "conv": super::pct(conv), "go": super::pct(go), "punt": super::pct(punt),
                        "fg": if make > 0.0 { Value::from(super::pct(fg)) } else { Value::Null }, "make": super::pct(make),
                    })
                })
                .collect();
            json!({ "name": CHART_ROWS[ri], "cells": cells })
        })
        .collect();
    (rows, tally)
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

pub(crate) fn rules_json(r: &GameRules) -> Value {
    json!({
        "fourth": fourth_key(r.fourth), "two": two_key(r.two), "tempo": tempo_key(r.tempo),
        "timeouts": timeouts_key(r.timeouts), "four": four_key(r.four),
        "cells": r.cells.iter().filter(|&&x| x != 0).count() > 0,
    })
}

pub(super) fn view(w: &World) -> Reply {
    let s = seat(w)?;
    let r = w.gameday.rules;
    let (rows, tally) = grid(&s, &r);
    let games = season_games(w, &s);
    // This season against the rules.
    let (mut tries, mut conv, mut punts, mut fgs) = (0, 0, 0, 0);
    let (mut two_t, mut two_m) = (0, 0);
    let (mut halves, mut with_two) = (0, 0);
    let (mut leads, mut kept) = (0, 0);
    for (g, side) in &games {
        for p in g.stats.plays.iter().filter(|p| p.offense == *side && !p.nullified) {
            if p.down == 4 {
                if is_snap(p.kind) {
                    tries += 1;
                    if p.first_down || p.touchdown {
                        conv += 1;
                    }
                } else if p.kind == PlayType::Punt {
                    punts += 1;
                } else if p.kind == PlayType::FieldGoal {
                    fgs += 1;
                }
            }
            if p.kind == PlayType::TwoPoint && !p.why.starts_with("Overtime shootout") {
                two_t += 1;
                if p.touchdown && !p.turnover {
                    two_m += 1;
                }
            }
        }
        for h in 0..2 {
            halves += 1;
            if g.stats.timeouts_left[h][*side] >= 2 {
                with_two += 1;
            }
        }
        let led = g.stats.plays.iter().any(|p| {
            p.quarter == 4 && p.clock <= 240.0 && p.offense == *side && {
                let lead = p.score[*side] - p.score[1 - *side];
                (1..=8).contains(&lead)
            }
        });
        if led {
            leads += 1;
            if g.stats.score[*side] > g.stats.score[1 - *side] {
                kept += 1;
            }
        }
    }
    let season = vec![
        json!({ "k": "4th downs converted", "v": format!("{conv} of {tries}"), "sub": format!("{tries} tries, {punts} punts, {fgs} field goals") }),
        json!({ "k": "Two-point tries", "v": format!("{two_m} of {two_t}"), "sub": if r.two == TwoChart::Chart { "by the chart".to_string() } else if r.two == TwoChart::Kick { "you kick every time".to_string() } else { "your staff's call".to_string() } }),
        json!({ "k": "Halves ended with two timeouts", "v": format!("{with_two} of {halves}"), "sub": match r.timeouts { TimeoutRule::KeepTwo => "your rule: keep two", TimeoutRule::KeepOne => "your rule: keep one", TimeoutRule::Free => "your rule: use them freely", TimeoutRule::Staff => "your staff's call" } }),
        json!({ "k": "Four-minute leads kept", "v": format!("{kept} of {leads}"), "sub": "ahead by 1-8 with the ball, four minutes left" }),
    ];
    // The two-point chart's margins after the touchdown.
    let behind: Vec<Value> = (1..=18).map(|m| json!({ "m": m, "go": chart_says_two(-m) })).collect();
    let ahead: Vec<Value> = (1..=19).map(|m| json!({ "m": m, "go": chart_says_two(m) })).collect();
    // The call words (G5).
    let words = w.plans.words;
    let first = w.career.as_ref().map_or(w.today.year(), |c| c.started.year());
    let knowers = plans::knowers(w);
    let upcoming = plans::upcoming(w);
    let holders: Vec<Value> = knowers
        .iter()
        .map(|k| {
            let name = match (k.person, k.coach) {
                (Some(i), _) => format!("[[p:{}|{}]]", w.people[i].player.id, k.name),
                (_, Some(c)) => super::coach_link(w, c),
                _ => k.name.clone(),
            };
            let when = k.team.and_then(|t| upcoming.iter().find(|n| n.opp == t)).map(|n| date_text(n.date));
            json!({
                "name": name, "what": k.what, "side": k.side,
                "now": match k.team { Some(t) => team_link(w, s.level, t), None => k.now.clone() },
                "when": when, "share": super::pct(k.share * 0.11),
            })
        })
        .collect();
    let sides: Vec<Value> = (0..2)
        .map(|side| {
            let n = knowers.iter().filter(|k| k.side == side).count();
            json!({
                "side": side, "name": (["Offense", "Defense"][side]),
                "since": if words.since[side] > 0 { words.since[side] } else { first },
                "changed": words.changed[side].map(date_text),
                "fresh": words.changed[side].is_some_and(|d| w.today.0 - d.0 <= 10),
                "knowers": n,
                "leak": upcoming.first().map(|nx| super::pct(plans::leak(w, nx.opp)[side])),
            })
        })
        .collect();
    let next = plans::next_game(w);
    let week = next.map(|n| {
        let changed = matches!(w.gameday.week_rules, Some((d, _)) if d == n.date);
        json!({ "date": date_text(n.date), "opp": team_link(w, s.level, n.opp), "changed": changed, "rules": rules_json(&w.gameday.rules_for(n.date)) })
    });
    Ok(json!({
        "level": lvl(s.level),
        "head": w.seat_head(),
        "rules": rules_json(&r),
        "cols": CHART_COLS,
        "grid": rows,
        "tally": { "go": tally[0], "fg": tally[1], "punt": tally[2] },
        "kicker": kicker(&s).and_then(|k| person_index(w, k.id).map(|i| json!({ "id": k.id, "name": w.people[i].name }))),
        "season": season,
        "season_words": season_words(w),
        "behind": behind,
        "ahead": ahead,
        "words": sides,
        "holders": holders,
        "week": week,
        "learn": super::pct(plans::WORDS_LEARN * 70.0),
    }))
}

fn fourth_of(v: &str) -> FourthChart {
    match v {
        "numbers" => FourthChart::Numbers,
        "aggressive" => FourthChart::Aggressive,
        "conservative" => FourthChart::Conservative,
        _ => FourthChart::Staff,
    }
}

/// Changes a set of rules from the arguments.
pub(crate) fn change(r: &mut GameRules, a: &Value) -> Result<(), String> {
    if let Some(v) = a.get("fourth").and_then(Value::as_str) {
        r.fourth = fourth_of(v);
        // A preset starts the chart over (your own cells go).
        if a.get("keep_cells").and_then(Value::as_bool) != Some(true) {
            r.cells = [0; 7];
        }
    }
    if let Some(v) = a.get("two").and_then(Value::as_str) {
        r.two = match v {
            "chart" => TwoChart::Chart,
            "kick" => TwoChart::Kick,
            _ => TwoChart::Staff,
        };
    }
    if let Some(v) = a.get("tempo").and_then(Value::as_str) {
        r.tempo = match v {
            "hurry" => Tempo::Hurry,
            "milk" => Tempo::Milk,
            _ => Tempo::Normal,
        };
    }
    if let Some(v) = a.get("timeouts").and_then(Value::as_str) {
        r.timeouts = match v {
            "keep_two" => TimeoutRule::KeepTwo,
            "keep_one" => TimeoutRule::KeepOne,
            "free" => TimeoutRule::Free,
            _ => TimeoutRule::Staff,
        };
    }
    if let Some(v) = a.get("four").and_then(Value::as_str) {
        r.four = match v {
            "run" => FourMinute::RunItOut,
            "attack" => FourMinute::KeepAttacking,
            _ => FourMinute::Staff,
        };
    }
    if a.get("clear_cells").and_then(Value::as_bool) == Some(true) {
        r.cells = [0; 7];
    }
    if let Some(c) = a.get("cell").and_then(Value::as_array) {
        let n = |i: usize| c.get(i).and_then(Value::as_u64).map(|x| x as usize);
        let (row, col) = (n(0).ok_or("Missing row.")?, n(1).ok_or("Missing column.")?);
        if row >= CHART_ROWS.len() || col >= CHART_COLS.len() {
            return Err("No such cell on the chart.".into());
        }
        let v = match c.get(2) {
            Some(Value::String(s)) => match s.as_str() {
                "go" | "Go" => 1,
                "fg" | "FG" => 2,
                "punt" | "Punt" => 3,
                _ => 0,
            },
            Some(x) => x.as_u64().unwrap_or(0).min(3) as u32,
            None => 0,
        };
        r.cells[row] = (r.cells[row] & !(3 << (2 * col))) | (v << (2 * col));
    }
    let _ = live::chart_at;
    Ok(())
}

/// The standing rules (the Plans page, the head coach), or one game's
/// (`week`: game day, for your next game; `reset` puts the standing rules
/// back for it).
pub(crate) fn set(w: &mut World, a: &Value) -> Result<(), String> {
    if a.get("week").and_then(Value::as_bool) == Some(true) {
        let next = plans::next_game(w).ok_or("No game left on your schedule.")?;
        if a.get("reset").and_then(Value::as_bool) == Some(true) {
            w.gameday.week_rules = None;
            return Ok(());
        }
        let mut r = w.gameday.rules_for(next.date);
        change(&mut r, a)?;
        w.gameday.week_rules = if r == w.gameday.rules { None } else { Some((next.date, r)) };
        return Ok(());
    }
    head(w)?;
    let mut r = w.gameday.rules;
    change(&mut r, a)?;
    w.gameday.rules = r;
    Ok(())
}

/// New call words for a side, or undo the change.
pub(super) fn words(w: &mut World, a: &Value) -> Result<(), String> {
    head(w)?;
    let side = a.get("side").and_then(Value::as_u64).unwrap_or(0) as usize;
    let undo = a.get("act").and_then(Value::as_str) == Some("undo");
    plans::change_words(w, side, undo)
}
