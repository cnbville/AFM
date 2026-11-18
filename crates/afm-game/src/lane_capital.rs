//! Capital's week and scenes (lane C4): the commands behind your donor
//! slots, the tour on an official visit, the names on your buildings and
//! the big moments' data (`afm_engine::world::capital`). The screens come
//! with lanes CS2 (donors) and CS7 (the big moments); these answer them.
//!
//! - `capital_week`: this week's donor slots (each with its scene once it
//!   has one), your bookings, and the development office's list of donors
//!   worth your time, with its reason.
//! - `capital_book` {donor, date "YYYY-MM-DD", dinner}, `capital_unbook`
//!   {donor, date}.
//! - `capital_tour` {kid}: the rooms you can show him with their read, your
//!   staff's usual tour and its read on him, and the tour set if any;
//!   `capital_tour_set` {kid, rooms [three room indexes, 0-12]}.
//! - `capital_names`: the names up on your buildings and rooms, and the
//!   scandals with their calls; `capital_name_call` {id, down}.
//! - `capital_donors` (lane CS2): the fever and your program's donors for the
//!   Donors screen: the office's range in words, loyalty, his lean once met,
//!   wants, circle and when you last met.
//! - `capital_moments`: your program's groundbreakings, reveal videos and
//!   ribbon cuttings, with the donors on stage, the recruits there and
//!   their reactions, and the headline.
//!
//! A donor scene plays on the people commands (`people_scene`,
//! `people_answer`, `people_hand`).

use afm_engine::calendar::Date;
use afm_engine::world::World;
use afm_engine::world::capital::rooms::Room;
use afm_engine::world::capital::{ceremony, names, scenes as cscenes, screen, tour, week};
use afm_engine::world::ground::week::{self as gweek, SlotKind};
use serde_json::{Value, json};

use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        "capital_week" => game.wm().and_then(week_view),
        "capital_book" => game.wm().and_then(|w| {
            let donor = args.get("donor").and_then(Value::as_u64).ok_or("Missing the donor.")? as u32;
            let date = args.get("date").and_then(Value::as_str).and_then(parse_day).ok_or("Missing the day.")?;
            let dinner = args.get("dinner").and_then(Value::as_bool).unwrap_or(true);
            let said = week::book(w, donor, date, dinner)?;
            let mut v = week_view(w)?;
            v["said"] = json!(said);
            Ok(v)
        }),
        "capital_unbook" => game.wm().and_then(|w| {
            let donor = args.get("donor").and_then(Value::as_u64).ok_or("Missing the donor.")? as u32;
            let date = args.get("date").and_then(Value::as_str).and_then(parse_day).ok_or("Missing the day.")?;
            let said = week::unbook(w, donor, date)?;
            let mut v = week_view(w)?;
            v["said"] = json!(said);
            Ok(v)
        }),
        "capital_tour" => game.w().and_then(|w| {
            let kid = args.get("kid").and_then(Value::as_u64).ok_or("Missing the recruit.")? as u32;
            tour_view(w, kid)
        }),
        "capital_tour_set" => game.wm().and_then(|w| {
            let kid = args.get("kid").and_then(Value::as_u64).ok_or("Missing the recruit.")? as u32;
            let rooms: Vec<Room> = args
                .get("rooms")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_u64).filter_map(|r| Room::from_index(r as u8)).collect())
                .unwrap_or_default();
            let said = tour::set(w, kid, &rooms)?;
            let mut v = tour_view(w, kid)?;
            v["said"] = json!(said);
            Ok(v)
        }),
        "capital_names" => game.w().map(names_view),
        "capital_name_call" => game.wm().and_then(|w| {
            let id = args.get("id").and_then(Value::as_u64).ok_or("Missing the call.")? as u32;
            let down = args.get("down").and_then(Value::as_bool).ok_or("Down or not?")?;
            let said = names::call(w, id, down)?;
            let mut v = names_view(w);
            v["said"] = json!(said);
            Ok(v)
        }),
        "capital_moments" => game.w().map(moments_view),
        "capital_donors" => game.w().and_then(donors_view),
        _ => return None,
    })
}

fn parse_day(s: &str) -> Option<Date> {
    let mut it = s.split('-').map(|x| x.parse::<i32>().ok());
    let (y, m, d) = (it.next()??, it.next()??, it.next()??);
    ((1900..=2200).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d))
        .then(|| Date::ymd(y, m as u32, d as u32))
}

fn iso(d: Date) -> String {
    let (y, m, dd) = d.parts();
    format!("{y:04}-{m:02}-{dd:02}")
}

fn donor_json(w: &World, prog: usize, id: u32) -> Value {
    let Some(d) = w.capital.donor(id) else { return Value::Null };
    let you = w.career.as_ref().map(|c| c.coach as u32).unwrap_or(u32::MAX);
    let met = d.bond(you).map_or(0, |b| b.met);
    json!({
        "id": d.id,
        "name": d.name,
        "business": d.business,
        "read": screen::read(w, prog, d, met).words,
        "catch": screen::catch_read(w, prog, d, met),
        "withYou": cscenes::warmth_words(d.warmth_to(you)),
        "met": met,
        "last": d.bond(you).and_then(|b| b.last).map(iso),
    })
}

fn week_view(w: &mut World) -> Reply {
    let prog = week::your_program(w).ok_or("Donors are the head coach's to court.")?;
    gweek::plan::ensure_now(w, prog);
    let w: &World = w;
    let slots: Vec<Value> = w
        .ground
        .week
        .slots
        .iter()
        .filter(|s| matches!(s.kind, SlotKind::Donor | SlotKind::DonorRoom))
        .map(|s| {
            json!({
                "date": iso(s.date),
                "minute": s.minute,
                "kind": s.kind.key(),
                "title": s.title,
                "reason": s.reason,
                "yours": s.yours,
                "donor": s.donor.map(|d| donor_json(w, prog, d)),
                "scene": afm_engine::world::ground::scenes::made(w, s).filter(|&x| x != 0),
                "line": s.line,
            })
        })
        .collect();
    let booked: Vec<Value> = w
        .capital
        .booked
        .iter()
        .map(|b| json!({ "date": iso(b.date), "minute": b.minute, "donor": donor_json(w, prog, b.donor) }))
        .collect();
    let monday = gweek::monday_of(w.today);
    let list: Vec<Value> = week::suggest(w, prog, monday)
        .into_iter()
        .take(12)
        .map(|(id, lead, why)| {
            let mut v = donor_json(w, prog, id);
            v["lead"] = json!(lead);
            v["why"] = json!(why);
            v
        })
        .collect();
    Ok(json!({
        "inSeason": week::in_season(w.today),
        "slots": slots,
        "booked": booked,
        "office": list,
        "caravan": iso(week::caravan_day(w.today.year())),
        "golf": iso(week::golf_day(w.today.year())),
    }))
}

fn tour_view(w: &World, kid: u32) -> Reply {
    let prog = week::your_program(w).ok_or("Tours are a college head coach's call.")?;
    let i = afm_engine::world::folk::person(w, kid).ok_or("No such recruit.")?;
    let pr = &w.programs[prog];
    let p = &w.people[i];
    let rooms: Vec<Value> = tour::tourable(pr)
        .into_iter()
        .map(|r| json!({ "room": r as u8, "name": r.words(), "shown": pr.campus.shown(r) }))
        .collect();
    let usual: Vec<u8> = tour::usual(pr).into_iter().map(|r| r as u8).collect();
    let set = w.capital.tours.iter().find(|t| t.kid == kid && t.program == prog as u32);
    Ok(json!({
        "kid": kid,
        "name": p.name,
        "visit": tour::visit_day(w, i, prog).map(iso),
        "read": tour::staff_read(p, None),
        "rooms": rooms,
        "usual": usual,
        "set": set.map(|t| json!({ "rooms": t.rooms, "words": t.words })),
    }))
}

fn names_view(w: &World) -> Value {
    let Some(prog) = afm_engine::world::ground::week::seat_program(w) else { return json!({ "names": [] }) };
    let list: Vec<Value> = w
        .capital
        .names
        .iter()
        .filter(|n| n.program == prog as u32)
        .map(|n| {
            json!({
                "id": n.id,
                "words": n.words(w),
                "donor": n.donor,
                "room": n.room,
                "since": n.since,
                "down": n.down.map(iso),
            })
        })
        .collect();
    let calls: Vec<Value> = w
        .capital
        .scandals
        .iter()
        .filter(|x| x.program == prog as u32)
        .map(|x| {
            json!({
                "id": x.id,
                "name": x.name,
                "donor": x.donor,
                "day": iso(x.day),
                "what": x.what,
                "down": x.down,
                "yours": x.yours,
                "scene": x.scene,
            })
        })
        .collect();
    json!({ "names": list, "scandals": calls })
}

/// A gift as a share of its drive's goal.
fn share(w: &World, drive: u32, total: f64) -> f64 {
    let goal = w.capital.drives.iter().find(|x| x.id == drive).map_or(0.0, |x| x.goal);
    if goal > 0.0 { (total / goal * 100.0).round() / 100.0 } else { 0.0 }
}

fn moments_view(w: &World) -> Value {
    let Some(prog) = afm_engine::world::ground::week::seat_program(w) else { return json!({ "moments": [] }) };
    let list: Vec<Value> = w
        .capital
        .ceremonies
        .iter()
        .filter(|c| c.program == prog as u32)
        .map(|c: &ceremony::Ceremony| {
            json!({
                "id": c.id,
                "rite": c.rite.words(),
                "day": iso(c.day),
                "held": c.held,
                "building": c.building,
                "rooms": Room::ALL.iter().filter(|r| c.rooms & r.bit() != 0).map(|r| r.words()).collect::<Vec<_>>(),
                "donors": c.donors.iter().map(|(d, total)| json!({
                    "id": d,
                    "name": w.capital.donor(*d).map(|x| x.name.clone()),
                    // No dollars on screen (CP39): his share of the goal.
                    "shareOfGoal": share(w, c.drive, *total),
                })).collect::<Vec<_>>(),
                "kids": c.kids.iter().map(|k| json!({ "kid": k.kid, "words": k.words, "strength": k.strength })).collect::<Vec<_>>(),
                "headline": c.headline,
            })
        })
        .collect();
    json!({ "moments": list })
}

// --- The Donors screen (lane CS2) -------------------------------------------------------------

/// `capital_donors`: the fever and the donors of your program, as the
/// Donors screen shows them. Wealth is the development office's range in
/// words, never a number (CP3, CP39); his lean to NIL or buildings shows
/// once you have met him.
fn donors_view(w: &World) -> Reply {
    let prog = week::your_program(w).ok_or("Donors are the head coach's to court.")?;
    let you = w.career.as_ref().map(|c| c.coach as u32).unwrap_or(u32::MAX);
    let open = afm_engine::world::capital::drive::open_drive(&w.capital, prog).map(|k| &w.capital.drives[k]);
    let mut rows: Vec<(f64, Value)> = afm_engine::world::capital::donors::of(w, prog)
        .into_iter()
        .map(|k| {
            let d = &w.capital.donors[k];
            let b = d.bond(you);
            let met = b.map_or(0, |b| b.met);
            let read = screen::read(w, prog, d, met);
            let warmth = d.warmth_to(you);
            let mates: Vec<&str> = d
                .in_circle()
                .map(|c| {
                    w.capital
                        .donors
                        .iter()
                        .filter(|x| x.alive() && x.id != d.id && x.in_circle() == Some(c))
                        .map(|x| x.name.as_str())
                        .collect()
                })
                .unwrap_or_default();
            let circle = d.in_circle().and_then(|c| w.capital.circle(c));
            let side = (met > 0 || b.is_some_and(|b| b.last.is_some())).then(|| match d.nil_lean {
                x if x >= 0.4 => "Buildings",
                x if x <= -0.4 => "NIL only",
                _ => "Both",
            });
            let rooms: Vec<&str> = d
                .rooms
                .iter()
                .filter_map(|&r| afm_engine::world::capital::donors::ROOMS.get(r as usize).copied())
                .collect();
            let lead = open.is_some_and(|x| x.lead.contains(&d.id));
            let pledging = w.capital.pledges.iter().any(|p| p.donor == d.id && p.program == prog as u32 && p.open());
            let also = d
                .schools()
                .into_iter()
                .find(|s| s.0 != prog as u32)
                .and_then(|s| w.programs.get(s.0 as usize))
                .map(|p| p.name(w.options.names).to_string());
            let seen_days = b.and_then(|b| b.last).map(|l| l.days_to(w.today));
            let v = json!({
                "id": d.id,
                "name": d.name,
                "she": d.she,
                "age": d.age(w.year),
                "business": d.business,
                "alum": d.alum,
                "pro": d.pro,
                "money": match d.money { afm_engine::world::capital::Money::Old => "Old guard", _ => "New money" },
                "read": read.words,
                "band": [read.lo, read.hi],
                "met": met,
                "withYou": if met == 0 && b.is_none_or(|b| b.last.is_none()) { "You haven't met" } else { cscenes::warmth_words(warmth) },
                "warmth": warmth,
                "last": b.and_then(|b| b.last).map(iso),
                "seenDays": seen_days,
                "side": side,
                "wants": d.wants.iter().map(|x| x.short()).collect::<Vec<_>>(),
                "wantsLong": d.wants.iter().map(|x| x.words()).collect::<Vec<_>>(),
                "rooms": rooms,
                "catch": screen::catch_read(w, prog, d, met),
                "kept": b.map_or(0, |b| b.kept),
                "broken": b.map_or(0, |b| b.broken),
                "heard": b.map_or(0, |b| b.heard),
                "coached": b.is_some_and(|b| b.coached),
                "circle": circle.map(|c| json!({ "name": c.name, "old": c.old, "mates": mates })),
                "also": also,
                "lead": lead,
                "pledging": pledging,
                "tied": w.capital.pledges.iter().any(|p| p.donor == d.id && p.program == prog as u32 && p.open() && p.tied == Some(you)),
            });
            // Most worth your attention first: a lead donor, then what the office reads.
            let rank = if lead { 100.0 } else { 0.0 } + (read.hi + read.lo) as f64 + warmth as f64;
            (rank, v)
        })
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    Ok(json!({
        "today": iso(w.today),
        "program": w.programs[prog].name(w.options.names),
        "fever": fever_view(w, prog),
        "donors": rows.into_iter().map(|r| r.1).collect::<Vec<_>>(),
        "booked": w.capital.booked.iter().map(|b| json!({ "date": iso(b.date), "minute": b.minute, "donor": b.donor })).collect::<Vec<_>>(),
    }))
}

/// The fever in words, its moments with how much each still counts, and the
/// day it will have faded if nothing new happens.
fn fever_view(w: &World, prog: usize) -> Value {
    use afm_engine::world::capital::fever;
    let now = fever::fever(w, prog);
    let mut parts: Vec<(f64, &afm_engine::world::capital::Moment)> = w
        .capital
        .moments
        .iter()
        .filter(|m| m.program == prog as u32 && m.date <= w.today)
        .map(|m| (m.what.strength() * fever::fade(m.date.days_to(w.today)), m))
        .filter(|x| x.0 > 0.0)
        .collect();
    parts.sort_by(|a, b| b.0.total_cmp(&a.0));
    let sum: f64 = parts.iter().map(|x| x.0).sum();
    let word = match now {
        x if x < 0.05 => "Cold",
        x if x < 0.2 => "Cool",
        x if x < 0.4 => "Warm",
        x if x < 0.65 => "Hot",
        _ => "Blazing",
    };
    let moments: Vec<Value> = parts
        .iter()
        .take(4)
        .map(|(s, m)| {
            json!({
                "what": m.what.words(),
                "date": iso(m.date),
                // Its share of today's fever, 0-1.
                "share": if sum > 0.0 { now * s / sum } else { 0.0 },
                "who": m.who.and_then(|id| afm_engine::world::marks::name_of(w, id)),
            })
        })
        .collect();
    // The first day it reads below a cool 0.05, stepping a fortnight.
    let mut fades = None;
    if now >= 0.05 {
        let mut day = w.today;
        for _ in 0..60 {
            day = day.add(15);
            if fever::fever_on(&w.capital.moments, prog, day) < 0.05 {
                fades = Some(iso(day));
                break;
            }
        }
    }
    json!({ "value": now, "word": word, "moments": moments, "fades": fades })
}
