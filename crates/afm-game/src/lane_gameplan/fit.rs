//! Fit against your guidelines (G23, board 27): a coordinator candidate's
//! systems ticked against what you ask for when you hire, and your staff's
//! read of each corner and safety on the recruiting or draft board for
//! press-man and zone, with the board in your guideline's order beside the
//! order on talent alone.

use afm_engine::attributes::Attr;
use afm_engine::calendar::Level as Cal;
use afm_engine::game::guide::{self, Guidelines};
use afm_engine::positions::Position;
use afm_engine::world::coach::Role;
use afm_engine::world::folk::staffing;
use afm_engine::world::front_office::college::{self as desk, Standing};
use afm_engine::world::scouting::{self as sc, board};
use afm_engine::world::systems::{FRONTS, OFFENSES, SHELLS};
use afm_engine::world::{Person, World};
use serde_json::{Value, json};

use super::{coach_link, lvl};
use crate::Reply;
use crate::reads::{self, Read};
use crate::views::{TeamRef, class_text, eye_for, mine, reader_key};

const SHELL_NAMES: [&str; 4] = ["Cover 3", "quarters", "man and pressure", "Cover 2"];

fn guidelines(w: &World) -> Guidelines {
    if w.seat_head() {
        w.plans.guide
    } else {
        afm_engine::world::plans::our_team(w).map(|t| t.staff.guide).unwrap_or_default()
    }
}

/// A tick: ✓ fits, ~ in part, ✕ against it.
fn tick(score: f64) -> &'static str {
    if score >= 0.75 {
        "yes"
    } else if score >= 0.5 {
        "part"
    } else {
        "no"
    }
}

fn best(know: &[f64; 11], r: std::ops::Range<usize>) -> usize {
    let s = r.start;
    r.max_by(|&a, &b| know[a].total_cmp(&know[b])).unwrap_or(s)
}

/// Coordinator candidates for a role (`oc` or `dc`) with their fit.
pub(super) fn staff(w: &World, a: &Value) -> Reply {
    let role = match a.get("role").and_then(Value::as_str) {
        Some("oc") => Role::OffensiveCoordinator,
        _ => Role::DefensiveCoordinator,
    };
    let offense = role == Role::OffensiveCoordinator;
    let g = guidelines(w);
    let mut against: Vec<Value> = Vec::new();
    if offense {
        if let Some(s) = g.system.to {
            against.push(json!({ "k": "System", "v": s.name(), "firm": g.system.firm.key() }));
        }
        if let Some(r) = g.run.to {
            against
                .push(json!({ "k": "Run or pass", "v": format!("{:.0}% runs", 100.0 * r), "firm": g.run.firm.key() }));
        }
        if let Some(p) = g.pace.to {
            against.push(json!({ "k": "Tempo", "v": p.name(), "firm": g.pace.firm.key() }));
        }
    } else {
        if let Some(f) = g.front.to {
            against.push(json!({ "k": "Base front", "v": f.name(), "firm": g.front.firm.key() }));
        }
        if let Some(m) = g.man.to {
            against.push(json!({ "k": "Coverage", "v": format!("{:.0}% man", 100.0 * m), "firm": g.man.firm.key() }));
        }
        if let Some(p) = g.pressure.to {
            against.push(
                json!({ "k": "Pressure", "v": format!("{:.0}% 5+ rush", 100.0 * p), "firm": g.pressure.firm.key() }),
            );
        }
    }
    let rows: Vec<Value> = staffing::candidates(w, role)
        .into_iter()
        .take(40)
        .map(|c| {
            let co = &w.coaches[c.coach];
            let know = co.system_know();
            let mut ticks: Vec<Value> = Vec::new();
            let mut score = 0.0;
            let mut n = 0.0;
            let mut add = |k: &str, s: f64, why: String| {
                ticks.push(json!({ "k": k, "t": tick(s), "why": why }));
                score += if s >= 0.75 {
                    1.0
                } else if s >= 0.5 {
                    0.5
                } else {
                    0.0
                };
                n += 1.0;
            };
            let calls = if offense {
                let o = OFFENSES[best(&know, 0..5)];
                if let Some(s) = g.system.to {
                    let i = OFFENSES.iter().position(|&x| x == s).unwrap_or(3);
                    add("System", know[i], format!("calls the {}", OFFENSES[best(&know, 0..5)].name()));
                }
                if let Some(r) = g.run.to {
                    let own = 1.0 - guide::habit_pass(true, o, 10.0);
                    add(
                        "Run or pass",
                        1.0 - (r - own).abs() * 3.0,
                        format!("about {:.0}% runs on his own", 100.0 * own),
                    );
                }
                if let Some(p) = g.pace.to {
                    let own = guide::habit_pace(o);
                    add(
                        "Tempo",
                        1.0 - (p.share() - own).abs() * 1.6,
                        format!("{:.0}% no-huddle on his own", 100.0 * own),
                    );
                }
                format!("{} offense", o.name())
            } else {
                let f = best(&know, 5..7) - 5;
                let s = best(&know, 7..11) - 7;
                if let Some(b) = g.front.to {
                    let i = FRONTS.iter().position(|&x| x == b.front()).unwrap_or(0);
                    add("Front", know[5 + i], format!("runs a {}", if f == 0 { "4-3" } else { "3-4" }));
                }
                if let Some(m) = g.man.to {
                    let own = guide::habit_man(SHELLS[s]);
                    add("Coverage", 1.0 - (m - own).abs() * 2.5, format!("about {:.0}% man on his own", 100.0 * own));
                }
                if let Some(p) = g.pressure.to {
                    let own = guide::habit_pressure(co.personality.ambition);
                    add(
                        "Pressure",
                        1.0 - (p - own).abs() * 3.0,
                        format!("sends five on about {:.0}% on his own", 100.0 * own),
                    );
                }
                format!("{} and {}", if f == 0 { "4-3" } else { "3-4" }, SHELL_NAMES[s])
            };
            let word = if n == 0.0 {
                "No guidelines"
            } else if score / n >= 0.8 {
                "Strong"
            } else if score / n >= 0.5 {
                "Good"
            } else {
                "Poor"
            };
            json!({
                "coach": c.coach, "link": coach_link(w, c.coach), "now": c.now, "age": c.age, "rep": c.rep,
                "calls": calls, "ticks": ticks, "fit": word, "known": c.known,
                "read": [c.read.0.round(), c.read.1.round()],
            })
        })
        .collect();
    Ok(json!({
        "level": w.seat_team().map(|x| lvl(x.0)), "role": if offense { "oc" } else { "dc" },
        "against": against, "rows": rows, "head": w.seat_head(),
    }))
}

fn composite(p: &Person, parts: &[(Attr, f64)]) -> f64 {
    parts.iter().map(|(a, k)| k * p.player.attributes.get(*a)).sum()
}

fn mid(r: Read) -> f64 {
    match r {
        Read::Range(lo, hi) => (lo as f64 + hi as f64) / 2.0,
        Read::NotSeen => 50.0,
    }
}

/// Your staff's read of a defensive back for press-man and for zone, and
/// its overall read: (overall, man, zone), ranges on 1-99.
fn db_reads(w: &World, t: TeamRef, p: &Person, half: f64) -> (Read, Read, Read) {
    let reader = reader_key(w);
    let man = composite(p, &[(Attr::Press, 0.4), (Attr::ManCoverage, 0.4), (Attr::Speed, 0.2)]);
    let zone = composite(p, &[(Attr::ZoneCoverage, 0.6), (Attr::PlayRecognition, 0.4)]);
    let _ = t;
    (
        reads::range(p.player.overall(), half, reader, p.player.id, 99),
        reads::range(man, half, reader, p.player.id, 120),
        reads::range(zone, half, reader, p.player.id, 121),
    )
}

fn tag(man: f64, zone: f64) -> &'static str {
    if man >= zone + 4.0 {
        "Press-man fit"
    } else if zone >= man + 4.0 {
        "Zone fit"
    } else if man >= 60.0 {
        "Fits both"
    } else {
        "Neither stands out"
    }
}

fn standing_words(s: Standing) -> &'static str {
    match s {
        Standing::Open => "Open",
        Standing::CommittedHere => "Committed to you",
        Standing::SignedHere => "Signed with you",
        Standing::CommittedElsewhere => "Committed elsewhere",
        Standing::SignedElsewhere => "Signed elsewhere",
        Standing::Enrolled => "On campus",
        Standing::Gone => "Gone",
    }
}

/// The corners and safeties on your recruiting board (college) or draft
/// board (NFL), read for press-man and zone against your coverage
/// guideline.
pub(super) fn recruits(w: &World, a: &Value) -> Reply {
    let Some(t) = mine(w) else { return Err("You have no team.".into()) };
    let level = w.seat_team().map(|x| x.0).ok_or("You have no team.")?;
    let pos = match a.get("pos").and_then(Value::as_str) {
        Some("S") => Position::S,
        _ => Position::CB,
    };
    let g = guidelines(w);
    let want_man = g.man.to;
    let manish = want_man.is_some_and(|m| m >= 0.4);
    let eye = eye_for(w, t, pos);
    let mut rows: Vec<(f64, f64, Value)> = Vec::new(); // talent, fit-weighted, row
    let kind;
    match (level, t) {
        (Cal::College, TeamRef::College(j)) => {
            kind = "recruits";
            for (k, target) in w.front_office.college.board.iter().enumerate() {
                let Some(i) = sc::place(w, target.id) else { continue };
                let p = &w.people[i];
                if p.player.position != pos {
                    continue;
                }
                let half = 3.0 + (20.0 - eye.clamp(1.0, 20.0)) * 0.2;
                let (o, m, z) = db_reads(w, t, p, half);
                let fitv = if want_man.is_none() {
                    mid(o)
                } else if manish {
                    mid(m)
                } else {
                    mid(z)
                };
                rows.push((
                    mid(o),
                    0.6 * mid(o) + 0.4 * fitv,
                    json!({
                        "id": p.player.id, "name": p.name, "pos": pos.name(), "class": class_text(w, p),
                        "read": o.json(), "man": m.json(), "zone": z.json(), "tag": tag(mid(m), mid(z)),
                        "interest": standing_words(desk::standing(p, j)), "board": k + 1,
                        "fit": if want_man.is_none() { "No coverage guideline" } else { super::identity::FIT_WORDS[super::identity::fit_level((fitv - 60.0) / 6.0)] },
                    }),
                ));
            }
        }
        (Cal::Nfl, TeamRef::Nfl(_)) => {
            kind = "draft";
            let grades = board::seat_grades(w);
            let order = board::ordered(w, &grades);
            for (k, id) in order.iter().take(400).enumerate() {
                let Some(i) = sc::place(w, *id) else { continue };
                let p = &w.people[i];
                if p.player.position != pos {
                    continue;
                }
                let (o, m, z) = db_reads(w, t, p, 5.0);
                let fitv = if want_man.is_none() {
                    mid(o)
                } else if manish {
                    mid(m)
                } else {
                    mid(z)
                };
                rows.push((
                    grades.get(id).copied().unwrap_or(mid(o)),
                    0.6 * mid(o) + 0.4 * fitv,
                    json!({
                        "id": p.player.id, "name": p.name, "pos": pos.name(), "class": class_text(w, p),
                        "read": o.json(), "man": m.json(), "zone": z.json(), "tag": tag(mid(m), mid(z)),
                        "interest": Value::Null, "board": k + 1,
                        "fit": if want_man.is_none() { "No coverage guideline" } else { super::identity::FIT_WORDS[super::identity::fit_level((fitv - 60.0) / 6.0)] },
                    }),
                ));
                if rows.len() >= 30 {
                    break;
                }
            }
        }
        _ => {
            kind = "none";
        }
    }
    // Talent-alone order and the guideline's order.
    let mut by_talent: Vec<usize> = (0..rows.len()).collect();
    by_talent.sort_by(|&x, &y| rows[y].0.total_cmp(&rows[x].0));
    let mut by_fit: Vec<usize> = (0..rows.len()).collect();
    by_fit.sort_by(|&x, &y| rows[y].1.total_cmp(&rows[x].1));
    let mut out: Vec<Value> = Vec::new();
    for (rank, &k) in by_fit.iter().enumerate() {
        let mut r = rows[k].2.clone();
        r["fit_rank"] = json!(rank + 1);
        r["talent_rank"] = json!(by_talent.iter().position(|&x| x == k).map_or(0, |x| x + 1));
        out.push(r);
    }
    Ok(json!({
        "level": lvl(level), "kind": kind, "pos": pos.name(),
        "guideline": want_man.map(|m| format!("{:.0}% man", 100.0 * m)),
        "manish": manish, "rows": out,
        "why": match kind {
            "none" => "High school staffs coach who walks in: there's no board to grade.",
            _ if want_man.is_none() => "Set a coverage guideline on the Identity tab and your staff grades every corner against it.",
            _ => "Your staff reads each one for press-man and for zone; the board's order with your guideline weighs the fit.",
        },
    }))
}
