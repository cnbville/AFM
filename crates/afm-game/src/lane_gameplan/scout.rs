//! The self-scout (G6, board 21): what your own film gives away. Run share
//! (or man coverage) by down and distance against the level, how often a
//! defense guessing from your habits would be right, by personnel, by
//! formation, by hash, and the looks that tell. Best done on a bye week:
//! the full study reads the tells from fewer snaps for the rest of the
//! season.

use afm_engine::calendar::Level as Cal;
use afm_engine::game::PlayRecord;
use afm_engine::game::PlayType;
use afm_engine::game::playbook::Formation;
use afm_engine::world::World;
use afm_engine::world::plans::{self, BREAKER, Play};
use serde_json::{Value, json};

use super::{abbr, is_pass, is_snap, lvl, num, opp_of, pct, season_games, season_words, seat};
use crate::Reply;
use crate::views::date_text;

const DOWNS: [&str; 7] = ["1st & 10", "2nd & 7+", "2nd & 4-6", "2nd & 1-3", "3rd & 7+", "3rd & 4-6", "3rd & 1-3"];

/// The level's run share by down and distance (real play-by-play, 2021-25:
/// the NFL from nflverse, FBS from cfbfastR; high school from tracked
/// varsity seasons; proposed, not confirmed).
fn run_base(level: Cal) -> [f64; 7] {
    match level {
        Cal::Nfl | Cal::Other => [0.50, 0.35, 0.48, 0.65, 0.08, 0.22, 0.60],
        Cal::HighSchool => [0.65, 0.52, 0.62, 0.75, 0.25, 0.45, 0.75],
        _ => [0.55, 0.43, 0.55, 0.68, 0.12, 0.30, 0.66],
    }
}

/// The level's man coverage on throws by down and distance (charting
/// services' shares, about 30% in the NFL; proposed, not confirmed).
fn man_base(level: Cal) -> [f64; 7] {
    match level {
        Cal::Nfl | Cal::Other => [0.28, 0.26, 0.27, 0.30, 0.33, 0.32, 0.38],
        Cal::HighSchool => [0.35, 0.33, 0.35, 0.38, 0.40, 0.40, 0.45],
        _ => [0.30, 0.28, 0.30, 0.32, 0.35, 0.34, 0.40],
    }
}

/// How often a defense guessing run or pass from your down-and-distance
/// habits is right at the level (proposed, not confirmed).
fn guess_base(level: Cal) -> f64 {
    match level {
        Cal::Nfl | Cal::Other => 0.62,
        Cal::HighSchool => 0.68,
        _ => 0.64,
    }
}

fn bucket(r: &PlayRecord) -> Option<usize> {
    let t = r.togo;
    Some(match r.down {
        1 => 0,
        2 if t >= 7.0 => 1,
        2 if t >= 4.0 => 2,
        2 => 3,
        3 if t >= 7.0 => 4,
        3 if t >= 4.0 => 5,
        3 => 6,
        _ => return None,
    })
}

/// A snap the self-scout counts: a run or a throw outside the last two
/// minutes of each half and garbage time (real staffs cut both).
fn counts(r: &PlayRecord) -> bool {
    !r.nullified
        && is_snap(r.kind)
        && !(matches!(r.quarter, 2 | 4) && r.clock <= 120.0)
        && r.garbage == [0, 0]
        && r.down >= 1
}

fn formation_group(call: &str) -> &'static str {
    if call.contains(" empty ") {
        "Empty"
    } else if call.contains(" pistol ") {
        "Pistol"
    } else if call.contains(" under center ") {
        "Under center"
    } else if call.contains(" trips ") || call.contains(" bunch ") {
        "Gun trips"
    } else if call.contains(" doubles ") {
        "Gun doubles"
    } else {
        "Gun, other sets"
    }
}

const FORMATIONS: [&str; 6] = ["Pistol", "Gun doubles", "Gun trips", "Gun, other sets", "Under center", "Empty"];

/// Which way a snap went as the offense looks: 1 left, 2 right, 0 not
/// known (a run's call names it; a throw's arrival across the field).
fn way(r: &PlayRecord) -> u8 {
    if r.kind == PlayType::Run {
        let c = r.call.trim_end_matches(" (audible)");
        if c.ends_with(" left") {
            return 1;
        }
        if c.ends_with(" right") {
            return 2;
        }
        return 0;
    }
    match r.throw_y {
        Some((a, b)) if b > a + 2.0 => 1,
        Some((a, b)) if b < a - 2.0 => 2,
        _ => 0,
    }
}

fn zone(yl: f64) -> usize {
    match yl {
        y if y >= 90.0 => 0,
        y if y >= 60.0 => 1,
        y if y >= 40.0 => 2,
        y if y > 20.0 => 3,
        _ => 4,
    }
}

const ZONES: [&str; 5] =
    ["Backed up, own 1-10", "Own 11-40", "Midfield, own 41 to their 40", "Their 39-21", "Red zone"];

fn read_of(share: f64, n: usize, base: f64) -> String {
    if n < 8 {
        "too few snaps to read".into()
    } else if share >= 0.75 {
        "a run tell".into()
    } else if share <= 0.25 {
        "a pass tell".into()
    } else if share - base >= 0.12 {
        "run-heavy".into()
    } else if base - share >= 0.12 {
        "pass-heavy".into()
    } else {
        "balanced".into()
    }
}

pub(super) fn view(w: &World, args: &Value) -> Reply {
    let s = seat(w)?;
    let defense = args.get("side").and_then(Value::as_str) == Some("defense");
    let games = season_games(w, &s);
    let level = s.level;
    let next = plans::next_game(w);
    let next_in = next.map(|n| n.date.0 - w.today.0);
    let studied = plans::studied(w);
    let study = w.plans.study.filter(|_| studied).map(date_text);
    let base = if defense { man_base(level) } else { run_base(level) };
    // The snaps: ours on offense; theirs against our defense (their throws
    // for the coverage).
    let snaps: Vec<(&PlayRecord, usize, usize)> = games
        .iter()
        .enumerate()
        .flat_map(|(gi, (g, side))| {
            g.stats
                .plays
                .iter()
                .filter(move |r| counts(r) && ((r.offense == *side) != defense))
                .map(move |r| (r, *side, gi))
        })
        .collect();
    let hit = |r: &PlayRecord| if defense { r.coverage_man } else { r.kind == PlayType::Run };
    let counted = |r: &PlayRecord| !defense || is_pass(r.kind);
    // By down and distance.
    let mut by = [(0usize, 0usize); 7];
    for (r, _, _) in snaps.iter().filter(|x| counted(x.0)) {
        if let Some(b) = bucket(r) {
            by[b].1 += 1;
            if hit(r) {
                by[b].0 += 1;
            }
        }
    }
    let downs: Vec<Value> = DOWNS
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let (h, n) = by[i];
            let v = if n > 0 { h as f64 / n as f64 } else { 0.0 };
            let diff = 100.0 * (v - base[i]);
            let note = if n < 6 {
                format!("{n} snaps: too few to read")
            } else if diff.abs() >= 10.0 {
                format!("{}{:.0} against the level: easy to read", if diff > 0.0 { "+" } else { "−" }, diff.abs())
            } else {
                format!("{}{:.0} against the level", if diff >= 0.0 { "+" } else { "−" }, diff.abs())
            };
            json!({ "label": name, "n": n, "v": if n > 0 { Value::from(pct(v)) } else { Value::Null }, "avg": pct(base[i]), "diff": diff.round(), "note": note, "flag": n >= 6 && diff.abs() >= 10.0 })
        })
        .collect();
    // The guess rate: a defense calling run or pass (or man or zone) by
    // your season's majority in each down's box.
    let major: Vec<bool> = by.iter().map(|(h, n)| *n > 0 && *h * 2 >= *n).collect();
    let mut per_game: Vec<Value> = Vec::new();
    let (mut right_all, mut all) = (0usize, 0usize);
    for (gi, (g, side)) in games.iter().enumerate() {
        let (mut right, mut n) = (0usize, 0usize);
        for (r, _, _) in snaps.iter().filter(|x| x.2 == gi && counted(x.0)) {
            let Some(b) = bucket(r) else { continue };
            n += 1;
            if hit(r) == major[b] {
                right += 1;
            }
        }
        right_all += right;
        all += n;
        if n >= 10 {
            let (_, name) = opp_of(g, *side);
            per_game.push(json!({ "o": abbr(name, *side == 1), "title": name, "v": pct(right as f64 / n as f64) }));
        }
    }
    let gbase = if defense { 1.0 - base.iter().sum::<f64>() / 7.0 } else { guess_base(level) };
    let overall = if all > 0 { right_all as f64 / all as f64 } else { 0.0 };
    let worst = (0..7).filter(|&i| by[i].1 >= 6).max_by(|&a, &b| {
        let d = |i: usize| (by[i].0 as f64 / by[i].1 as f64 - base[i]).abs();
        d(a).total_cmp(&d(b))
    });
    let read = if all < 30 {
        format!(
            "{} snaps so far {}: too few for a defense to read you, or for you to read yourself.",
            all,
            season_words(w)
        )
    } else {
        let mut t = format!(
            "A defense guessing {} from your down-and-distance habits is right on {:.0}% of snaps (the {} usual {:.0}%).",
            if defense { "man or zone" } else { "run or pass" },
            100.0 * overall,
            super::level_words(level),
            100.0 * gbase
        );
        if let Some(i) = worst {
            let v = by[i].0 as f64 / by[i].1 as f64;
            let d = 100.0 * (v - base[i]);
            if d.abs() >= 10.0 {
                t.push_str(&format!(
                    " {} gives you away most: {:.0}% {} against the level's {:.0}%.",
                    DOWNS[i],
                    100.0 * v,
                    if defense { "man" } else { "runs" },
                    100.0 * base[i]
                ));
            }
        }
        t
    };
    let overall_base = base.iter().sum::<f64>() / 7.0;
    let mut tables: Vec<Value> = Vec::new();
    if !defense {
        // By personnel.
        let mut pers: Vec<(String, usize, usize)> = Vec::new();
        let mut forms: Vec<(&str, usize, usize)> = FORMATIONS.iter().map(|f| (*f, 0, 0)).collect();
        let mut hashes = [(0usize, 0usize, 0usize, 0usize); 3]; // snaps, runs, to the field, known way
        for (r, _, _) in &snaps {
            let run = r.kind == PlayType::Run;
            match pers.iter_mut().find(|x| x.0 == r.personnel) {
                Some(x) => {
                    x.1 += 1;
                    x.2 += run as usize;
                }
                None => pers.push((r.personnel.clone(), 1, run as usize)),
            }
            let f = formation_group(&r.call);
            if let Some(x) = forms.iter_mut().find(|x| x.0 == f) {
                x.1 += 1;
                x.2 += run as usize;
            }
            let h = match r.hash {
                1 => 0,
                3 => 1,
                2 => 2,
                _ => continue,
            };
            hashes[h].0 += 1;
            hashes[h].1 += run as usize;
            let wy = way(r);
            if h != 1 && wy != 0 {
                hashes[h].3 += 1;
                // The field is the wide side: right from the left hash.
                if (h == 0 && wy == 2) || (h == 2 && wy == 1) {
                    hashes[h].2 += 1;
                }
            }
        }
        pers.sort_by_key(|x| std::cmp::Reverse(x.1));
        let share = |r: usize, n: usize| if n > 0 { r as f64 / n as f64 } else { 0.0 };
        tables.push(json!({
            "title": "By personnel", "cols": ["Group", "Snaps", "Runs", "Read"],
            "rows": pers.iter().filter(|x| !x.0.is_empty()).map(|(p, n, r)| json!({ "cells": [p, n, format!("{:.0}%", 100.0 * share(*r, *n)), read_of(share(*r, *n), *n, overall_base)], "flag": *n >= 8 && (share(*r, *n) >= 0.75 || share(*r, *n) <= 0.25) })).collect::<Vec<_>>(),
        }));
        tables.push(json!({
            "title": "By formation", "cols": ["Formation", "Snaps", "Runs", "Read"],
            "rows": forms.iter().filter(|x| x.1 > 0).map(|(f, n, r)| json!({ "cells": [f, n, format!("{:.0}%", 100.0 * share(*r, *n)), read_of(share(*r, *n), *n, overall_base)], "flag": *n >= 8 && (share(*r, *n) >= 0.75 || share(*r, *n) <= 0.25) })).collect::<Vec<_>>(),
        }));
        let names = ["Left hash", "Middle", "Right hash"];
        tables.push(json!({
            "title": "By hash", "cols": ["Hash", "Snaps", "Runs", "To the field"], "note": "The level: about 58% of plays go to the wide side from a hash.",
            "rows": (0..3).filter(|&h| hashes[h].0 > 0).map(|h| {
                let (n, r, fld, known) = hashes[h];
                let to = if h == 1 { "no wide side".to_string() } else if known >= 6 { format!("{:.0}%", 100.0 * share(fld, known)) } else { format!("{known} known") };
                json!({ "cells": [names[h], n, format!("{:.0}%", 100.0 * share(r, n)), to], "flag": h != 1 && known >= 10 && share(fld, known) >= 0.72 })
            }).collect::<Vec<_>>(),
        }));
    } else {
        // Against their personnel, by field zone, by hash: snaps, man on
        // throws, five or more rushers.
        let mut pers: Vec<(String, usize, usize, usize, usize)> = Vec::new(); // snaps, throws, man, blitz
        let mut zones = [(0usize, 0usize, 0usize, 0usize); 5];
        let mut hashes = [(0usize, 0usize, 0usize, 0usize); 3];
        for (r, _, _) in &snaps {
            let thr = is_pass(r.kind) as usize;
            let man = (thr == 1 && r.coverage_man) as usize;
            let bl = r.blitz as usize;
            match pers.iter_mut().find(|x| x.0 == r.personnel) {
                Some(x) => {
                    x.1 += 1;
                    x.2 += thr;
                    x.3 += man;
                    x.4 += bl;
                }
                None => pers.push((r.personnel.clone(), 1, thr, man, bl)),
            }
            let z = &mut zones[zone(r.yardline_100)];
            z.0 += 1;
            z.1 += thr;
            z.2 += man;
            z.3 += bl;
            let h = match r.hash {
                1 => 0,
                3 => 1,
                2 => 2,
                _ => continue,
            };
            hashes[h].0 += 1;
            hashes[h].1 += thr;
            hashes[h].2 += man;
            hashes[h].3 += bl;
        }
        pers.sort_by_key(|x| std::cmp::Reverse(x.1));
        let sh = |a: usize, b: usize| {
            if b > 0 { format!("{:.0}%", 100.0 * a as f64 / b as f64) } else { "no throws".into() }
        };
        let flag = |man: usize, thr: usize, bl: usize, n: usize| {
            (thr >= 8 && man as f64 / thr as f64 >= 0.6) || (n >= 8 && bl as f64 / n as f64 >= 0.5)
        };
        tables.push(json!({
            "title": "Against their personnel", "cols": ["Their group", "Snaps", "Man", "5+ rush"],
            "rows": pers.iter().filter(|x| !x.0.is_empty()).map(|(p, n, t, m, b)| json!({ "cells": [p, n, sh(*m, *t), sh(*b, *n)], "flag": flag(*m, *t, *b, *n) })).collect::<Vec<_>>(),
        }));
        tables.push(json!({
            "title": "By field zone", "cols": ["Zone", "Snaps", "Man", "5+ rush"],
            "rows": (0..5).filter(|&z| zones[z].0 > 0).map(|z| { let (n, t, m, b) = zones[z]; json!({ "cells": [ZONES[z], n, sh(m, t), sh(b, n)], "flag": flag(m, t, b, n) }) }).collect::<Vec<_>>(),
        }));
        let names = ["Left hash", "Middle", "Right hash"];
        tables.push(json!({
            "title": "By hash", "cols": ["Hash", "Snaps", "Man", "5+ rush"],
            "rows": (0..3).filter(|&h| hashes[h].0 > 0).map(|h| { let (n, t, m, b) = hashes[h]; json!({ "cells": [names[h], n, sh(m, t), sh(b, n)], "flag": flag(m, t, b, n) }) }).collect::<Vec<_>>(),
        }));
    }
    // The tells.
    let draft = w.plans.draft.as_ref();
    let breakers_for = |look: usize| -> Vec<String> {
        draft
            .map(|d| {
                d.off
                    .iter()
                    .filter(|e| e.marks & BREAKER != 0)
                    .filter_map(|e| match e.play {
                        Play::Off(p) => {
                            let mut f = Formation::plain(p.personnel, p.backfield, 1.0);
                            f.set = p.set;
                            (f.look() == look).then(|| plans::play_name(&p, &w.training.designs))
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let n_games = games.len();
    let tells: Vec<Value> = if defense {
        // Their personnel and the down: man or pressure called nearly
        // every time.
        let mut cells: Vec<(String, usize, usize, usize, usize, usize)> = Vec::new(); // pers, bucket group, throws, man, snaps, blitz
        for (r, _, _) in &snaps {
            let Some(b) = bucket(r) else { continue };
            let grp = if b == 0 {
                0
            } else if b <= 3 {
                1
            } else {
                2
            };
            let thr = is_pass(r.kind) as usize;
            match cells.iter_mut().find(|x| x.0 == r.personnel && x.1 == grp) {
                Some(x) => {
                    x.2 += thr;
                    x.3 += (thr == 1 && r.coverage_man) as usize;
                    x.4 += 1;
                    x.5 += r.blitz as usize;
                }
                None => cells.push((
                    r.personnel.clone(),
                    grp,
                    thr,
                    (thr == 1 && r.coverage_man) as usize,
                    1,
                    r.blitz as usize,
                )),
            }
        }
        let (n_t, edge) = if studied { (6, 0.6) } else { (8, 0.65) };
        let mut v: Vec<Value> = Vec::new();
        for (p, grp, t, m, n, b) in cells {
            let when = ["first down", "second down", "third down"][grp];
            if t >= n_t && m as f64 / t as f64 >= edge {
                let sh = m as f64 / t as f64;
                v.push(json!({ "name": format!("Man against {p} on {when}"), "what": format!("{m} of {t} throws in man"), "n": t, "share": pct(sh), "strength": if sh >= 0.85 { "Strong" } else { "Clear" }, "seen": Value::Null, "of": n_games, "breakers": [] }));
            }
            if n >= n_t && b as f64 / n as f64 >= 0.5 {
                let sh = b as f64 / n as f64;
                v.push(json!({ "name": format!("Pressure against {p} on {when}"), "what": format!("five or more on {b} of {n} snaps"), "n": n, "share": pct(sh), "strength": if sh >= 0.7 { "Strong" } else { "Clear" }, "seen": Value::Null, "of": n_games, "breakers": [] }));
            }
        }
        v
    } else {
        plans::tells(w, false)
            .into_iter()
            .map(|t| {
                let rs = t.run_share();
                let sh = rs.max(1.0 - rs);
                let br = breakers_for(t.look);
                json!({
                    "name": plans::look_words(t.look),
                    "what": if rs >= 0.5 { format!("runs on {} of {} snaps", t.runs, t.snaps()) } else { format!("throws on {} of {} snaps", t.passes, t.snaps()) },
                    "kind": if rs >= 0.5 { "run" } else { "pass" },
                    "n": t.snaps(), "share": pct(sh), "seen": t.games, "of": n_games,
                    "strength": if sh >= 0.85 { "Strong" } else { "Clear" },
                    "breakers": br,
                })
            })
            .collect()
    };
    let breakers_on = draft.map_or(0, |d| d.off.iter().filter(|e| e.marks & BREAKER != 0).count());
    Ok(json!({
        "level": lvl(level),
        "side": if defense { "defense" } else { "offense" },
        "season": season_words(w),
        "games": n_games,
        "snaps": snaps.len(),
        "measure": if defense { "man coverage on their throws" } else { "runs" },
        "downs": downs,
        "guess": { "games": per_game, "line": pct(gbase), "overall": if all > 0 { Value::from(pct(overall)) } else { Value::Null } },
        "read": read,
        "tables": tables,
        "tells": tells,
        "study": study,
        "studied": studied,
        "next_in": next_in,
        "bye": next_in.is_none_or(|d| d > 8),
        "breakers": breakers_on,
        "head": w.seat_head(),
        "cut": "Snaps in the last two minutes of each half and in garbage time are left out, as real staffs do.",
        "edge": if studied { "6 snaps and 70%" } else { "8 snaps and 75%" },
        "pts": num(overall_base * 100.0, 0),
    }))
}
