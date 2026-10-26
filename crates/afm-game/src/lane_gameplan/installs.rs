//! Installs (G14, board 23): each two-deep player's grasp of this week's
//! plan now and as your staff expects it by the last practice, the new
//! calls he carries, his film, and the busts that grasp means on game day
//! (the engine's own terms, `plans::bust_rate`). Film tonight, an extra
//! walkthrough or base downs only for the ones who won't be ready.

use afm_engine::attributes::Attr;
use afm_engine::calendar::{Date, Level as Cal};
use afm_engine::game::playbook::Family;
use afm_engine::game::roster::GamePlayer;
use afm_engine::positions::Position;
use afm_engine::training::plan::{Kind, weekday_index};
use afm_engine::training::seat as ts;
use afm_engine::world::World;
use afm_engine::world::plans::{self, Play, WET};
use afm_engine::world::systems;
use serde_json::{Value, json};

use super::{lvl, seat, team_link};
use crate::Reply;
use crate::reads::{self, Read};
use crate::views::{class_text, date_text, person_index, reader_key};

/// The game-ready line: 85% of the plan known (proposed, not confirmed).
const READY: f64 = 0.85;

/// The game week: the plan for a game goes in over the last seven days
/// before it. Further out, the board shows the grasp as it stands.
const WEEK: i32 = 7;

/// Whether the next game is in this game week.
fn in_week(w: &World, next: Option<plans::Next>) -> bool {
    next.is_some_and(|n| n.date.0 - w.today.0 <= WEEK)
}

fn unit_arg(a: &Value) -> usize {
    match a.get("unit").and_then(Value::as_str) {
        Some("defense") => 1,
        Some("kicking") => 2,
        _ => 0,
    }
}

/// How many of a position make the two-deep's rows.
fn two_deep(pos: Position) -> usize {
    match pos {
        Position::WR | Position::IOL | Position::LB | Position::CB | Position::S | Position::EDGE | Position::DT => 3,
        Position::K | Position::P | Position::LS => 1,
        _ => 2,
    }
}

/// The practice days left before the game (today's still to come counts
/// while it is a weekday before the game's eve).
fn days_left(today: Date, game: Date) -> i32 {
    let mut n = 0;
    let mut d = today;
    while d.0 < game.0 - 1 {
        if weekday_index(d) <= 5 {
            n += 1;
        }
        d = d.add(1);
    }
    n.min(5)
}

/// Your staff's projection of a grasp by the last practice: each practice
/// day closes part of the gap, more for a sharp mind (proposed, not
/// confirmed), plus tonight's extra film or walkthrough.
fn by_friday(now: f64, iq: f64, days: i32, boost: f64) -> f64 {
    let r = (0.07 + 0.002 * (iq - 60.0)).clamp(0.03, 0.12);
    (1.0 - (1.0 - now) * (1.0 - r).powi(days) + boost).clamp(0.0, 0.99)
}

/// A game's snaps for a starter and a backup at the level.
fn snaps(level: Cal, starter: bool, unit: usize) -> f64 {
    if unit == 2 {
        return if starter { 6.0 } else { 0.0 };
    }
    let full = match level {
        Cal::Nfl | Cal::Other => 62.0,
        Cal::HighSchool => 50.0,
        _ => 70.0,
    };
    if starter { full } else { 0.18 * full }
}

fn monday_of(d: Date) -> Date {
    let mut m = d;
    while weekday_index(m) != 0 {
        m = m.add(-1);
    }
    m
}

pub(super) fn view(w: &World, a: &Value) -> Reply {
    let s = seat(w)?;
    let unit = unit_arg(a);
    let next = plans::next_game(w);
    let fams = plans::sheet_families(w);
    let unit_fams: Vec<(Family, f64)> = match unit {
        2 => Vec::new(),
        u => fams.iter().copied().filter(|(f, _)| f.offense() == (u == 0)).collect(),
    };
    let wk = &w.plans.week;
    let this = next.is_some_and(|n| wk.game == Some(n.date));
    let week = in_week(w, next);
    let extra_of = |id: u32, x: u8| this && wk.extra.iter().any(|e| e.0 == id && e.1 == x);
    let base_of = |id: u32| this && wk.base_only.contains(&id);
    // Out of the game week nothing is projected: the plan isn't in yet.
    let days = if week { next.map_or(0, |n| days_left(w.today, n.date)) } else { 0 };
    let away = next.map(|n| n.date.0 - w.today.0);
    let load = s.team.plan().1;
    // The week's film minutes on your practice plan.
    let film_min: u32 = w
        .training_team()
        .map(|st| {
            let pw = ts::planned(w, &st, monday_of(w.today));
            pw.week
                .days
                .iter()
                .flat_map(|d| d.periods.iter())
                .filter(|p| p.kind == Kind::Film)
                .map(|p| p.min as u32)
                .sum()
        })
        .unwrap_or(0);
    // This week's sheet plays for the unit, for the new calls each player
    // carries.
    let plays: Vec<Family> = w
        .plans
        .draft
        .as_ref()
        .map(|d| {
            let list = if unit == 1 { &d.def } else { &d.off };
            list.iter()
                .filter(|e| e.marks & WET == 0)
                .map(|e| match e.play {
                    Play::Off(p) => match p.play {
                        afm_engine::training::seat::SheetKind::Run(r) => r.family(),
                        afm_engine::training::seat::SheetKind::Pass(k) => afm_engine::game::playbook::call_family(
                            afm_engine::game::playbook::PlayKind::Pass(k as usize),
                        ),
                        afm_engine::training::seat::SheetKind::Design(_) => Family::Dropback,
                    },
                    Play::Def(c, _) => c.family(),
                })
                .collect()
        })
        .unwrap_or_default();
    // Your staff's read of each mind (a range, never the true number): the
    // projection and the words go by its middle.
    let reader = reader_key(w);
    let read_iq =
        |p: &GamePlayer| match reads::range(p.a(Attr::FootballIq), 6.0, reader, p.id, Attr::FootballIq as usize) {
            Read::Range(lo, hi) => (lo as f64 + hi as f64) / 2.0,
            Read::NotSeen => 60.0,
        };
    let mut rows: Vec<Value> = Vec::new();
    let (mut sum_now, mut sum_fri, mut sum_busts, mut n) = (0.0, 0.0, 0.0, 0usize);
    let mut seen: Vec<u32> = Vec::new();
    // The two-deep as the engine dresses it: your order where you set one,
    // your staff's for the rest (a season's team keeps no order of its own
    // in the offseason).
    let order = plans::depth_order(&s.team, s.level);
    for (pos, ids) in &order {
        let off = systems::offense(*pos);
        let fits = match unit {
            0 => off && !pos.is_specialist(),
            1 => !off && !pos.is_specialist(),
            _ => pos.is_specialist(),
        };
        if !fits {
            continue;
        }
        for (k, &id) in ids.iter().take(two_deep(*pos)).enumerate() {
            if seen.contains(&id) {
                continue;
            }
            let Some(p) = s.team.players.iter().find(|p| p.id == id) else { continue };
            seen.push(id);
            let now = plans::grasp(&s.team, id, unit != 1, &unit_fams).unwrap_or(0.0);
            let iq = read_iq(p);
            let boost = if extra_of(id, 1) { 0.04 } else { 0.0 } + if extra_of(id, 2) { 0.03 } else { 0.0 };
            let fri = by_friday(now, iq, days, boost);
            let starter = k == 0 || (k < two_deep(*pos) / 2 + usize::from(two_deep(*pos) >= 3));
            let mut sn = snaps(s.level, starter, unit);
            if base_of(id) {
                sn *= 0.55;
            }
            let rate = plans::bust_rate(fri, iq, load, p.prep as f64);
            let busts = if week { rate * sn } else { 0.0 };
            let calls = plays.iter().filter(|f| p.know[**f as usize] < 0.6).count();
            let name = person_index(w, id).map(|i| w.people[i].name.clone()).unwrap_or_else(|| "A player".into());
            let class = person_index(w, id).map(|i| class_text(w, &w.people[i])).unwrap_or_default();
            sum_now += now;
            sum_fri += fri;
            sum_busts += busts;
            n += 1;
            rows.push(json!({
                "id": id, "name": name, "pos": pos.name(), "class": class, "role": if starter { "1st" } else { "2nd" },
                "now": super::pct(now), "friday": super::pct(fri), "calls": calls,
                "film": super::num((film_min as f64 / 60.0) + if extra_of(id, 1) { 1.5 } else { 0.0 }, 1),
                "busts": (busts * 10.0).round() / 10.0, "flag": week && fri < READY,
                "extra": { "film": extra_of(id, 1), "walk": extra_of(id, 2), "base": base_of(id) },
                "iq_words": iq_words(iq),
            }));
        }
    }
    // The families on the sheet: the unit's grasp now and by Friday.
    let mut fam_rows: Vec<Value> = unit_fams
        .iter()
        .map(|(f, wt)| {
            let ids: Vec<&GamePlayer> = s.team.players.iter().filter(|p| seen.contains(&p.id)).collect();
            let g = ids.iter().map(|p| p.know[*f as usize] as f64).sum::<f64>() / ids.len().max(1) as f64;
            let iq = ids.iter().map(|p| read_iq(p)).sum::<f64>() / ids.len().max(1) as f64;
            let fri = if week { by_friday(g, iq, days, 0.0) } else { g };
            json!({ "name": f.name(), "calls": wt.round(), "now": super::pct(g), "friday": super::pct(fri), "new": g < 0.7 })
        })
        .collect();
    fam_rows.sort_by(|a, b| a["now"].as_f64().unwrap_or(0.0).total_cmp(&b["now"].as_f64().unwrap_or(0.0)));
    // The week's chart: the logged days, then the projection.
    let monday = monday_of(w.today);
    let mut chart: Vec<Value> = Vec::new();
    let names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let avg_now = if n > 0 { sum_now / n as f64 } else { 0.0 };
    for i in 0..if week { 7 } else { 0 } {
        let d = monday.add(i);
        if next.is_some_and(|nx| d.0 >= nx.date.0) {
            break;
        }
        let logged = w.plans.grasp_log.iter().find(|x| x.0 == d).map(|x| x.1[unit] as f64);
        let v = match logged {
            Some(v) => Some((v, false)),
            None if d.0 > w.today.0 => {
                let ahead = days_left(w.today, d.add(1));
                Some((by_friday(avg_now, 60.0, ahead, 0.0), true))
            }
            None if d == w.today => Some((avg_now, false)),
            None => None,
        };
        if let Some((v, proj)) = v {
            chart.push(json!({ "d": names[i as usize], "v": super::pct(v), "proj": proj }));
        }
    }
    let busts_words = |b: f64| {
        if b < 1.0 {
            "under one bust"
        } else if b < 3.0 {
            "a few busts"
        } else {
            "more busts than a ready team"
        }
    };
    Ok(json!({
        "level": lvl(s.level), "unit": (["offense", "defense", "kicking"][unit]), "head": w.seat_head(),
        "has_game": next.is_some(), "week": week, "away": away,
        "game": next.map(|nx| date_text(nx.date)), "opp": next.map(|nx| team_link(w, s.level, nx.opp)),
        "days": days, "ready": super::pct(READY),
        "rows": rows,
        "total": { "now": super::pct(avg_now), "friday": if n > 0 { super::pct(sum_fri / n as f64) } else { 0.0 }, "busts": (sum_busts * 10.0).round() / 10.0, "words": busts_words(sum_busts) },
        "families": fam_rows,
        "chart": chart,
        "film_hours": super::num(film_min as f64 / 60.0, 1),
        "load": if load > 1.1 { "a heavy plan for the week" } else if load < 0.9 { "a light plan" } else { "a normal week's plan" },
        "sheet": w.plans.draft.is_some(),
    }))
}

/// Your staff's words for how fast he learns, from its read of his mind.
fn iq_words(iq: f64) -> &'static str {
    if iq >= 75.0 {
        "picks it up fast"
    } else if iq >= 60.0 {
        "learns it in a normal week"
    } else {
        "needs the reps"
    }
}

/// Extra film tonight, an extra walkthrough or base downs only for one
/// player this week (`on` false takes it back).
pub(super) fn extra(w: &mut World, a: &Value) -> Result<(), String> {
    let id = a.get("id").and_then(Value::as_u64).ok_or("Missing player.")? as u32;
    let x = a.get("x").and_then(Value::as_str).ok_or("Missing what.")?;
    let on = a.get("on").and_then(Value::as_bool).unwrap_or(true);
    let i = person_index(w, id).ok_or("No such player.")?;
    let pos = w.people[i].player.position;
    if !w.seat_head() && !w.controls(pos) {
        return Err(format!("He isn't in your room. Your job covers {}.", w.your_controls()));
    }
    let next = plans::next_game(w).ok_or("No game ahead this week.")?;
    if !in_week(w, Some(next)) {
        return Err("Extras go in the week of the game.".into());
    }
    let wk = plans::week_mut(w);
    match x {
        "film" | "walk" => {
            let k = if x == "film" { 1 } else { 2 };
            wk.extra.retain(|e| !(e.0 == id && e.1 == k));
            if on {
                wk.extra.push((id, k));
            }
        }
        "base" => {
            wk.base_only.retain(|&e| e != id);
            if on {
                wk.base_only.push(id);
            }
        }
        _ => return Err("Film, walkthrough or base downs.".into()),
    }
    w.refresh_seat_team();
    Ok(())
}
