//! Ground's week (lane G3): the commands behind the meeting at the map and
//! the day on Ground (`afm_engine::world::ground::week`). The screens come
//! with lane S2 (board 2b); these answer it.
//!
//! - `ground_week`: the week's beat, the plan (each man's area, days and
//!   reason), road days left, the week's money as trips, your calls, the
//!   week's scenes, the Monday letter, your hours on the ledger, the film
//!   queue and your assignments.
//! - `ground_road_set` {man, area (code "GA-404" or index), days [0-6]},
//!   `ground_road_clear` {man}, `ground_hand` {on}.
//! - `ground_call` {who}, `ground_take` {id}, `ground_hear` {id}.
//! - `ground_day`: the morning map's overnight list, the in-flight list,
//!   the night calls and the calls waiting.
//! - `ground_week_off` {from "YYYY-MM-DD", weeks}, `ground_may`,
//!   `ground_may_invite` {n}, `ground_beats` {year}.
//! - Lane G5, the scenes: `ground_scenes` (the 15 and what each came to
//!   lately, the spring game's call), `ground_scene` {id} (one scene's
//!   ground part: its stops, the school, the kids, the new names, standing
//!   moved, its tables), `ground_spring_game` {hold}. A scene plays on the
//!   people commands (`people_scene`, `people_answer`, `people_hand`, a
//!   typed line on live talks v3); each slot of `ground_week` names its
//!   scene.
//!
//! A person is "coach:ID", "counselor:SCHOOL", "trainer:ID", "7v7:ID",
//! "family:PLAYER" (the oldest brother) or "prep:N".

use afm_engine::calendar::Date;
use afm_engine::world::World;
use afm_engine::world::ground::areas;
use afm_engine::world::ground::people::{self, Who};
use afm_engine::world::ground::scenes;
use afm_engine::world::ground::standing;
use afm_engine::world::ground::week::{self, DoneKind, beat, phone, plan, road};
use serde_json::{Value, json};

use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        "ground_week" => game.wm().and_then(week_view),
        "ground_road_set" => game.wm().and_then(|w| {
            let man = args.get("man").and_then(Value::as_u64).ok_or("Missing the man.")? as u32;
            let area = area_arg(args.get("area")).ok_or("Missing the area.")?;
            let days: Vec<u32> = args
                .get("days")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_u64).map(|x| x as u32).collect())
                .unwrap_or_default();
            let said = plan::set_road(w, man, area, &days)?;
            let mut v = week_view(w)?;
            v["said"] = json!(said);
            Ok(v)
        }),
        "ground_road_clear" => game.wm().and_then(|w| {
            let man = args.get("man").and_then(Value::as_u64).ok_or("Missing the man.")? as u32;
            let said = plan::clear_road(w, man)?;
            let mut v = week_view(w)?;
            v["said"] = json!(said);
            Ok(v)
        }),
        "ground_hand" => game.wm().and_then(|w| {
            let on = args.get("on").and_then(Value::as_bool).unwrap_or(true);
            let said = plan::hand(w, on)?;
            let mut v = week_view(w)?;
            v["said"] = json!(said);
            Ok(v)
        }),
        "ground_call" => game.wm().and_then(|w| {
            let who = who_arg(args.get("who")).ok_or("Missing the person.")?;
            let said = phone::call(w, who)?;
            Ok(json!({ "said": said, "day": day_view(w) }))
        }),
        "ground_take" => game.wm().and_then(|w| {
            let id = args.get("id").and_then(Value::as_u64).ok_or("Missing the call.")? as u32;
            let said = phone::take(w, id)?;
            Ok(json!({ "said": said, "day": day_view(w) }))
        }),
        "ground_hear" => game.wm().and_then(|w| {
            let id = args.get("id").and_then(Value::as_u64).ok_or("Missing the call.")? as u32;
            let said = phone::hear(w, id)?;
            Ok(json!({ "said": said, "day": day_view(w) }))
        }),
        "ground_day" => game.w().map(day_view),
        "ground_week_off" => game.wm().and_then(|w| {
            let from = args.get("from").and_then(Value::as_str).and_then(parse_day).ok_or("Missing the first day.")?;
            let weeks = args.get("weeks").and_then(Value::as_u64).unwrap_or(1) as u32;
            let said = week::take_off(w, from, weeks)?;
            Ok(json!({ "said": said }))
        }),
        "ground_may" => game.w().map(may_view),
        "ground_may_invite" => game.wm().and_then(|w| {
            let n = args.get("n").and_then(Value::as_u64).unwrap_or(99) as usize;
            let said = plan::may_invite(w, n)?;
            let mut v = may_view(w);
            v["said"] = json!(said);
            Ok(v)
        }),
        "ground_scenes" => game.w().map(scenes_view),
        "ground_scene" => game.w().and_then(|w| {
            let id = args.get("id").and_then(Value::as_u64).ok_or("Missing the scene.")? as u32;
            let s = w.folk.scenes.iter().find(|s| s.id == id).ok_or("No such scene.")?;
            if scenes::kind_of(s).is_none() {
                return Err("That isn't a ground scene.".into());
            }
            Ok(scene_part(w, s))
        }),
        "ground_spring_game" => game.wm().and_then(|w| {
            let hold = args.get("hold").and_then(Value::as_bool).unwrap_or(true);
            let said = scenes::set_spring_game(w, hold)?;
            let mut v = scenes_view(w);
            v["said"] = json!(said);
            Ok(v)
        }),
        "ground_beats" => game.w().map(|w| {
            let year = args.get("year").and_then(Value::as_i64).map_or(w.today.year(), |y| y as i32);
            beats_view(w, year)
        }),
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

fn area_arg(v: Option<&Value>) -> Option<u16> {
    let v = v?;
    if let Some(n) = v.as_u64() {
        return (n < areas::table().len() as u64).then_some(n as u16);
    }
    areas::find(v.as_str()?)
}

/// "coach:12" and the rest.
pub fn who_arg(v: Option<&Value>) -> Option<Who> {
    let s = v?.as_str()?;
    let (k, id) = s.split_once(':')?;
    let id: u32 = id.parse().ok()?;
    Some(match k {
        "coach" => Who::Coach(id),
        "counselor" => Who::Counselor(id),
        "trainer" => Who::Trainer(id),
        "7v7" => Who::SevenOnSeven(id),
        "family" => Who::Family(id),
        "prep" => Who::Prep(u16::try_from(id).ok()?),
        _ => return None,
    })
}

pub fn who_key(who: Who) -> String {
    match who {
        Who::Coach(c) => format!("coach:{c}"),
        Who::Counselor(h) => format!("counselor:{h}"),
        Who::Trainer(f) => format!("trainer:{f}"),
        Who::SevenOnSeven(f) => format!("7v7:{f}"),
        Who::Family(p) => format!("family:{p}"),
        Who::Prep(p) => format!("prep:{p}"),
    }
}

fn area_json(a: u16) -> Value {
    let ar = areas::area(a);
    json!({ "id": a, "code": areas::code(a), "name": ar.name, "nickname": ar.nickname })
}

fn person_json(w: &World, prog: usize, who: Who) -> Value {
    json!({
        "who": who_key(who),
        "name": people::name(w, who),
        "role": people::role(w, who),
        "where": phone::where_words(w, who),
        "level": standing::level(w, prog, who).name(),
    })
}

fn week_view(w: &mut World) -> Reply {
    let prog = week::seat_program(w).ok_or("Ground's week is for a college head coach's seat.")?;
    plan::ensure_now(w, prog);
    let w: &World = w;
    let monday = w.ground.week.monday.unwrap_or(week::monday_of(w.today));
    let b = beat::beat(w, monday);
    let wk = &w.ground.week;
    let roads: Vec<Value> = wk
        .roads
        .iter()
        .map(|r| {
            let t = areas::trip_to_area(w, prog, r.area);
            json!({
                "man": r.man,
                "name": w.coaches.get(r.man as usize).map(|c| c.name.clone()),
                "job": afm_engine::world::folk::facts::coach_role(w, r.man as usize),
                "area": area_json(r.area),
                "days": (0..7).filter(|k| r.days & (1 << k) != 0).collect::<Vec<u32>>(),
                "days_words": plan::days_words(r.days),
                "flight": r.flight,
                "trip": t.words(),
                "why": r.why,
                "yours": r.yours,
            })
        })
        .collect();
    let men: Vec<Value> = road::men(w, prog)
        .into_iter()
        .map(|m| {
            let rule = (0..7).map(|k| monday.add(k)).find_map(|d| road::may_go(w, prog, m, d).err());
            json!({
                "man": m,
                "name": w.coaches[m].name,
                "job": afm_engine::world::folk::facts::coach_role(w, m),
                "out": wk.roads.iter().any(|r| r.man == m as u32),
                "rule": rule,
            })
        })
        .collect();
    let open: Vec<(u32, road::Window)> = (0..7)
        .filter_map(|k| {
            let d = monday.add(k);
            road::window(w, d).filter(|win| road::day_kind(*win, d)).map(|win| (k as u32, win))
        })
        .collect();
    let road_json = match open.first() {
        Some(&(k, win)) => {
            let pool = road::pool(w, prog, win);
            json!({
                "window": win.key(),
                "open_days": open.iter().map(|o| o.0).collect::<Vec<u32>>(),
                "pool": pool.map(|p| p.0),
                "used": pool.map(|p| p.1),
                "left": road::left(w, prog, win),
                "days_ahead": road::open_days_from(w, win, monday.add(k as i32)),
                "closes": iso(window_ends(w, win, monday.add(k as i32))),
                "men_at_once": afm_engine::world::scouting::dept::off_campus_recruiters(&w.programs[prog], monday.add(k as i32)),
            })
        }
        None => Value::Null,
    };
    let p = road::purse(w, prog, &wk.roads);
    let calls: Vec<Value> = wk
        .calls
        .iter()
        .map(|c| {
            let mut v = person_json(w, prog, c.who);
            v["why"] = json!(c.why);
            v["state"] = json!(["to make", "made", "made by your coordinator"][c.state.min(2) as usize]);
            v
        })
        .collect();
    let slots: Vec<Value> = wk
        .slots
        .iter()
        .map(|s| {
            json!({
                "date": iso(s.date),
                "day": s.date.weekday().short(),
                "time": phone::clock(s.minute),
                "kind": s.kind.key(),
                "title": s.title,
                "reason": s.reason,
                "yours": s.yours,
                "line": s.line.clone().or_else(|| scenes::line_for(w, s)).or_else(|| slot_line(w, s)),
                "who": s.who.map(who_key),
                "scene": scenes::made(w, s).filter(|&id| id > 0),
                "sceneKind": scenes::Kind::of_slot(s.kind).map(|k| k.key()),
            })
        })
        .collect();
    let yours = wk.slots.iter().filter(|s| s.yours).count();
    let nights = wk.log.iter().filter(|x| x.kind == DoneKind::NightCall && week::monday_of(x.date) == monday).count();
    let film_q: Vec<Value> = w
        .scouting
        .film
        .queue
        .iter()
        .filter(|q| q.state == afm_engine::world::scouting::film::queue::WAITING)
        .map(|q| json!({ "id": q.id, "from": q.from, "why": q.why, "by": iso(q.by) }))
        .collect();
    let assignments: Vec<Value> = w
        .scouting
        .dept
        .assignments
        .iter()
        .map(|a| json!({ "who": a.who, "target": format!("{:?}", a.target), "since": iso(a.since) }))
        .collect();
    use afm_engine::world::scouting::film::time;
    let (clat, clon) = areas::campus(w, prog);
    let home = afm_engine::world::geo::STATE_CENTERS[w.programs[prog].state.min(50)].0;
    let coordinator = week::coordinator(w, prog).map(|c| {
        json!({
            "id": c,
            "name": w.coaches[c].name,
            "job": afm_engine::world::folk::facts::coach_role(w, c),
        })
    });
    let (fronts, one_call) = fronts_and_call(w, prog);
    Ok(json!({
        "monday": iso(monday),
        "today": iso(w.today),
        "home": {
            "state": home,
            "campus": [clat, clon],
            "areas": areas::in_state(w.programs[prog].state).iter().map(|&a| area_json(a)).collect::<Vec<_>>(),
        },
        "coordinator": coordinator,
        "fronts": fronts,
        "one_call": one_call,
        "beat": {
            "week": b.week, "line": b.line(), "period": b.period, "beat": b.beat, "stake": b.stake,
            "on": b.on.iter().map(|x| json!({ "date": iso(x.0), "what": x.1 })).collect::<Vec<_>>(),
        },
        "handed": wk.handed,
        "roads": roads,
        "men": men,
        "road": road_json,
        "money": { "flights": p.flights, "flown": p.flown, "raised": p.raised, "words": road::flights_words(p.flights) },
        "calls": calls,
        "slots": slots,
        "minutes": { "played": 2.5 + yours as f64 + 0.1 * nights as f64, "handed": 1.0 },
        "letter": plan::letter_for(w, monday).cloned().unwrap_or_default(),
        "hours": {
            "week": wk.hours(monday, false),
            "chosen": wk.hours(monday, true),
            "film": time::film_used(w),
            "free": time::budget(w),
            "left": time::left(w),
            "ledger": wk.ledger.iter().filter(|h| week::monday_of(h.date) == monday).map(|h| json!({
                "date": iso(h.date), "what": h.what, "hours": h.hours, "chosen": h.chosen,
            })).collect::<Vec<_>>(),
        },
        "film_queue": film_q,
        "assignments": assignments,
        "off": wk.off.map(|(a, b)| json!([iso(a), iso(b)])),
    }))
}

/// The last day of the window's run that holds `from` (the day the road
/// closes), looking up to 120 days ahead.
fn window_ends(w: &World, win: road::Window, from: Date) -> Date {
    let mut last = from;
    for k in 0..120 {
        let d = from.add(k);
        if road::window(w, d) == Some(win) {
            last = d;
        } else if k > 3 {
            break;
        }
    }
    last
}

/// The places that want a man this week, one per area the plan sends someone
/// to (your men by name, their days, the coordinator's reason, where you
/// stand with the area's head coaches), and the one call of your list the
/// coordinator would put to you first. Rivals' sign-in sheets come with the
/// race (lane G9).
fn fronts_and_call(w: &World, prog: usize) -> (Vec<Value>, Value) {
    let wk = &w.ground.week;
    let by_area = areas::schools_by_area(w);
    let mut order: Vec<u16> = Vec::new();
    for r in &wk.roads {
        if !order.contains(&r.area) {
            order.push(r.area);
        }
    }
    let fronts: Vec<Value> = order
        .iter()
        .enumerate()
        .map(|(k, &a)| {
            let rows: Vec<&week::Road> = wk.roads.iter().filter(|r| r.area == a).collect();
            let heads: Vec<Who> =
                by_area[a as usize].iter().filter_map(|&h| people::head_coach_of(w, h)).collect();
            let known = heads.iter().filter(|&&who| standing::level(w, prog, who) >= standing::Level::Known).count();
            let bits = rows.iter().fold(0u8, |b, r| b | r.days);
            let yours = rows.iter().any(|r| r.yours);
            let kind = if yours {
                "your call"
            } else if known == 0 {
                "new door"
            } else {
                "the plan"
            };
            let ar = areas::area(a);
            json!({
                "n": k + 1,
                "area": area_json(a),
                "center": [ar.center.0, ar.center.1],
                "label": [ar.label.0, ar.label.1],
                "kind": kind,
                "men": rows.iter().map(|r| json!({ "man": r.man, "name": week::last_name(w, r.man) })).collect::<Vec<_>>(),
                "days": (0..7).filter(|k| bits & (1 << k) != 0).collect::<Vec<u32>>(),
                "days_words": plan::days_words(bits),
                "flight": rows.iter().any(|r| r.flight),
                "why": rows.iter().find(|r| !r.yours).or(rows.first()).map(|r| r.why.clone()).unwrap_or_default(),
                "known": known,
                "of": heads.len(),
                "trip": areas::trip_to_area(w, prog, a).words(),
            })
        })
        .collect();
    let pending: Vec<&week::Planned> = wk.calls.iter().filter(|c| c.state == 0).collect();
    let one = pending.first().map_or(Value::Null, |c| {
        let mut v = person_json(w, prog, c.who);
        v["why"] = json!(c.why);
        v["more"] = json!(pending.len() - 1);
        v["hours"] = json!(phone::CALL_HOURS);
        v
    });
    (fronts, one)
}

/// A slot's one line from the week's log, for a slot no scene played.
fn slot_line(w: &World, s: &week::Slot) -> Option<String> {
    let log = &w.ground.week.log;
    match s.kind {
        week::SlotKind::CoachCall => log
            .iter()
            .rev()
            .filter(|x| matches!(x.kind, DoneKind::Call | DoneKind::CallBack | DoneKind::Missed))
            .find(|x| x.who == s.who && week::monday_of(x.date) == week::monday_of(s.date))
            .map(|x| x.text.clone()),
        week::SlotKind::SchoolVisit => {
            let v: Vec<&week::Done> = log
                .iter()
                .filter(|x| {
                    x.kind == DoneKind::Visit && x.man == s.man && week::monday_of(x.date) == week::monday_of(s.date)
                })
                .collect();
            (!v.is_empty()).then(|| v.iter().map(|x| x.text.clone()).collect::<Vec<_>>().join("; "))
        }
        _ => w
            .ground
            .week
            .ledger
            .iter()
            .find(|h| h.date == s.date && !h.chosen)
            .filter(|_| s.date < w.today || s.date == w.today && w.section() >= 2)
            .map(|h| h.what.clone()),
    }
}

fn day_view(w: &World) -> Value {
    let Some(prog) = week::seat_program(w) else { return Value::Null };
    let d = w.today;
    let nights: Vec<Value> = w
        .ground
        .week
        .log
        .iter()
        .filter(|x| x.kind == DoneKind::NightCall && x.date >= d.add(-1))
        .map(|x| {
            let at = w.seat_zone().to_eastern(afm_engine::calendar::Moment::at(x.date, x.minute / 60, x.minute % 60));
            json!({
                "id": x.id,
                "date": iso(x.date),
                "time": phone::clock(x.minute),
                "man": x.man,
                "text": x.text,
                "matters": x.matters,
                "heard": x.heard,
                "come": w.has_come(at),
                "area": x.area.map(area_json),
            })
        })
        .collect();
    let waiting: Vec<Value> = w
        .ground
        .week
        .phone
        .iter()
        .filter(|c| c.state == 0)
        .map(|c| {
            let mut v = person_json(w, prog, c.who);
            v["id"] = json!(c.id);
            v["date"] = json!(iso(c.date));
            v["time"] = json!(phone::clock(c.minute));
            v["matters"] = json!(c.matters);
            v
        })
        .collect();
    let flights: Vec<Value> = phone::in_flight(w)
        .into_iter()
        .map(|f| json!({ "what": f.what, "lands": f.lands, "date": iso(f.date), "time": f.minute.map(phone::clock) }))
        .collect();
    let today: Vec<Value> = w
        .ground
        .week
        .log
        .iter()
        .filter(|x| x.date == d && x.kind != DoneKind::NightCall)
        .map(|x| json!({ "id": x.id, "time": phone::clock(x.minute), "text": x.text }))
        .collect();
    json!({
        "clock": w.clock_text(),
        "overnight": if w.ground.week.overnight_day == Some(d) { w.ground.week.overnight.clone() } else { Vec::new() },
        "in_flight": flights,
        "night_calls": nights,
        "waiting": waiting,
        "today": today,
        "off": week::off_now(w),
    })
}

fn may_view(w: &World) -> Value {
    let m = &w.ground.week.may;
    let name = |id: u32| {
        afm_engine::world::scouting::place(w, id).map(|i| {
            let q = &w.people[i];
            json!({ "id": id, "name": q.name, "pos": q.player.position.name(), "stars": q.stars })
        })
    };
    json!({
        "date": m.date.map(iso),
        "spots": m.spots,
        "lists": m.lists.iter().map(|(man, ids)| json!({
            "man": man,
            "name": w.coaches.get(*man as usize).map(|c| c.name.clone()),
            "players": ids.iter().filter_map(|&id| name(id)).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "ranked": m.ranked.iter().filter_map(|&id| name(id)).collect::<Vec<_>>(),
        "invited": m.invited,
    })
}

fn beats_view(w: &World, year: i32) -> Value {
    let first = Date::ymd(year, 1, 1).on_or_before(afm_engine::calendar::Weekday::Mon);
    let v: Vec<Value> = (0..35)
        .map(|k| {
            let b = beat::beat(w, first.add(7 * k));
            json!({
                "week": b.week, "monday": iso(b.monday), "period": b.period, "beat": b.beat, "stake": b.stake,
                "on": b.on.iter().map(|x| json!({ "date": iso(x.0), "what": x.1 })).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({ "year": year, "weeks": v })
}

fn played_json(w: &World, prog: Option<usize>, p: &scenes::Played) -> Value {
    json!({
        "date": iso(p.date),
        "time": phone::clock(p.minute),
        "kind": p.kind.key(),
        "name": p.kind.name(),
        "title": p.title,
        "line": p.line,
        "yours": p.yours,
        "by": p.by.and_then(|c| w.coaches.get(c as usize)).map(|c| c.name.clone()),
        "who": p.who.map(who_key),
        "school": p.school.map(|h| w.school_name(h as usize).to_string()),
        "kids": p.kids,
        "leads": p.leads,
        "levels": p.levels.iter().map(|x| json!({
            "who": who_key(x.0),
            "name": people::name(w, x.0),
            "from": standing::Level::from_u8(x.1).name(),
            "to": standing::Level::from_u8(x.2).name(),
        })).collect::<Vec<_>>(),
        "stops": p.stops.iter().map(|x| json!({ "key": x.0, "text": x.1 })).collect::<Vec<_>>(),
        "scene": (p.scene > 0).then_some(p.scene),
        "now": prog.zip(p.who).map(|(g, who)| standing::level(w, g, who).name()),
    })
}

/// The 15 scenes (GR55), what each came to lately, and this year's spring
/// game.
fn scenes_view(w: &World) -> Value {
    let prog = week::seat_program(w);
    let kinds: Vec<Value> =
        scenes::Kind::ALL.iter().map(|k| json!({ "key": k.key(), "name": k.name(), "playsIn": k.built() })).collect();
    let played: Vec<Value> = w.ground.scenes.played.iter().rev().take(40).map(|p| played_json(w, prog, p)).collect();
    let y = beat::dates_for(w, w.today);
    let season = y.spring_game.year();
    json!({
        "kinds": kinds,
        "played": played,
        "springGame": {
            "date": iso(y.spring_game),
            "hold": scenes::holds_spring_game(w, season),
            "choice": season >= scenes::SPRING_GAME_CHOICE && w.today <= y.spring_game,
        },
        "clinic": prog.and_then(|g| scenes::clinic_day(w, g, w.today)).map(iso),
        "convention": iso(scenes::convention_day(y.season)),
        "invited": w.ground.scenes.invited.iter().filter(|x| x.1 == y.season).map(|x| json!({ "who": who_key(x.0), "name": people::name(w, x.0) })).collect::<Vec<_>>(),
    })
}

/// A ground scene's own part for the screens (S3 builds on it): its kind,
/// the school and the person, the stops so far, the kids it touched with
/// the new names marked, standing moved, and where each person stands now.
pub fn scene_part(w: &World, s: &afm_engine::world::folk::scene::Scene) -> Value {
    let prog = week::seat_program(w);
    let st = scenes::state(s);
    let k = scenes::kind_of(s);
    let kid = |id: &u32| {
        let p = afm_engine::world::scouting::place(w, *id).map(|i| &w.people[i]);
        json!({
            "id": id,
            "name": p.map(|p| p.name.clone()),
            "pos": p.map(|p| p.player.position.name()),
            "new": st.leads.contains(id),
        })
    };
    json!({
        "id": s.id,
        "kind": k.map(|k| k.key()),
        "name": k.map(|k| k.name()),
        "time": s.minute.map(phone::clock),
        "who": st.who.map(|who| {
            let mut v = json!({ "key": who_key(who), "name": people::name(w, who) });
            if let Some(g) = prog {
                v["level"] = json!(standing::level(w, g, who).name());
            }
            v
        }),
        "school": st.school.map(|h| json!({ "id": h, "name": w.school_name(h as usize) })),
        "man": st.man.and_then(|c| w.coaches.get(c as usize)).map(|c| c.name.clone()),
        "minutesLeft": st.minutes,
        "stops": st.stops.iter().map(|x| json!({ "key": x.0, "text": x.1 })).collect::<Vec<_>>(),
        "kids": st.kids.iter().map(kid).collect::<Vec<_>>(),
        "levels": st.levels.iter().map(|x| json!({
            "who": who_key(x.0),
            "name": people::name(w, x.0),
            "from": standing::Level::from_u8(x.1).name(),
            "to": standing::Level::from_u8(x.2).name(),
        })).collect::<Vec<_>>(),
        "people": st.people.len(),
        "docs": s.docs.iter().filter(|d| !scenes::is_state(d)).collect::<Vec<_>>(),
        "hand": scenes::hand_label(w, s),
        "done": s.done,
        "outcome": s.outcome,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn people_keys_go_both_ways() {
        for who in [Who::Coach(12), Who::Counselor(4), Who::Trainer(9), Who::SevenOnSeven(3)] {
            assert_eq!(who_arg(Some(&json!(who_key(who)))), Some(who));
        }
        assert_eq!(who_arg(Some(&json!("nobody:1"))), None);
    }
}
