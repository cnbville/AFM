//! Capital's project screen (lane CS4, `docs/design/capital.md` CP24, CP26,
//! CP37, CP39, board 4): the one building going up for your program. Its
//! rooms and what each gives, where it stands against the day it opens, a
//! cost overrun with the three answers (CP24), the temporary space the team
//! works in while it goes up (CP26) and the strings on it: names on the
//! building and the donors' catches (CP10, CP25).
//!
//! - `capital_project`: the view. No dollar figure anywhere (CP39): the cost
//!   reads as shares and words.
//! - `capital_overrun` {fix "cut" | "wait" | "borrow"}: answer the overrun
//!   (`drive::fix_overrun`), then the same page with the answer in `said`.
//! - `capital_catch` {id, yes}: grant or turn down a donor's catch
//!   (`gifts::answer_catch`), then the same page.
//!
//! The drive itself (goal, pledges, going public) is the drive screen's
//! (lane CS3); the rooms' map is the campus screen's (CS1).

use afm_engine::calendar::Date;
use afm_engine::world::World;
use afm_engine::world::capital::drive::{self, Drive, Fix};
use afm_engine::world::capital::gifts::{self, CatchKind};
use afm_engine::world::capital::rooms::{self, Room};
use afm_engine::world::capital::{screen, week};
use afm_engine::world::finance::{Owner, ProjectKind};
use serde_json::{Value, json};

use crate::lane_capital_campus::after;
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        "capital_project" => game.w().and_then(view),
        "capital_overrun" => game.wm().and_then(|w| {
            let p = week::your_program(w).ok_or("The head coach answers the builder.")?;
            let fix = match args.get("fix").and_then(Value::as_str) {
                Some("cut") => Fix::CutRooms,
                Some("wait") => Fix::Wait,
                Some("borrow") => Fix::Borrow,
                _ => return Err("Cut rooms, wait, or borrow?".to_string()),
            };
            let said = drive::fix_overrun(w, p, fix)?;
            done(w, said)
        }),
        "capital_catch" => game.wm().and_then(|w| {
            week::your_program(w).ok_or("The head coach takes a donor's call.")?;
            let id = args.get("id").and_then(Value::as_u64).ok_or("Which call?")? as u32;
            let yes = args.get("yes").and_then(Value::as_bool).ok_or("Grant it or not?")?;
            let said = gifts::answer_catch(w, id, yes)?;
            done(w, said)
        }),
        _ => return None,
    })
}

/// The view again, with what happened.
fn done(w: &World, said: String) -> Reply {
    let mut v = view(w)?;
    v["said"] = json!(said);
    Ok(v)
}

fn iso(d: Date) -> String {
    let (y, m, dd) = d.parts();
    format!("{y:04}-{m:02}-{dd:02}")
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn pct(x: f64) -> i64 {
    (x * 100.0).round() as i64
}

/// Your program's building under way: its drive and the project in the books.
fn under_way(w: &World, prog: usize) -> Option<(&Drive, usize)> {
    w.capital.drives.iter().rev().filter(|d| d.program == prog as u32 && d.ground.is_some()).find_map(|d| {
        let j = drive::project_of(w, d)?;
        let y = &w.finance.projects[j];
        (y.owner == Owner::College(prog) && y.kind == ProjectKind::Facility && !y.done).then_some((d, j))
    })
}

fn view(w: &World) -> Reply {
    let prog = afm_engine::world::ground::week::seat_program(w)
        .ok_or("Your building is a college program's. High school and NFL buildings come with their own screen.")?;
    let head = week::your_program(w).is_some();
    let name = w.programs[prog].name(w.options.names).to_string();
    let Some((x, j)) = under_way(w, prog) else {
        return Ok(json!({
            "program": name, "today": iso(w.today), "head": head, "building": Value::Null,
            "ready": !w.capital.drives.is_empty(),
        }));
    };
    let p = &w.programs[prog];
    let y = &w.finance.projects[j];
    let you = w.career.as_ref().map(|c| c.coach as u32).unwrap_or(u32::MAX);
    let today = w.today;

    // The building's name: a donor's if one is up, else what it is.
    let named = w.capital.names.iter().find(|n| n.drive == x.id && n.room.is_none() && n.up());
    let title = named.map_or_else(|| cap(&x.kind), |n| cap(&n.words(w)));

    // The timeline: ground to the day it is ready, with a year more if it was set back.
    let ground = x.ground_day.unwrap_or(today);
    let nominal = ground.add(drive::build_days(&x.kind));
    let nominal_season = nominal.year() + if nominal.month() > 7 { 1 } else { 0 };
    let extra = (y.opens - nominal_season).max(0);
    let ready = nominal.add(365 * extra);

    // The overrun, once the builder has found it.
    let over = x.overrun;
    let found = over.filter(|o| o.found <= today);
    let open_decision = found.filter(|o| o.fix.is_none());
    let dropped = open_decision.map_or_else(Vec::new, |o| drive::cut_preview(x, o.share).1);

    // The rooms it holds: new or moved, today's level and what it opens at.
    let mask = drive::rooms_of_project(w, prog, y);
    let levels = rooms::opening_levels(p, mask, y.cost as f64, y.opens);
    let holder =
        |r: Room| p.campus.halls.iter().filter(|h| h.holds(r)).max_by_key(|h| h.opened).map(|h| h.name.clone());
    let row = |r: Room, cut: bool| {
        let new = p.campus.level(r) <= 0.0;
        let shown = p.campus.shown(r);
        json!({
            "room": r as u8,
            "name": r.words(),
            "unlocks": r.unlocks(),
            "new": new,
            "from": if new { None } else { holder(r) },
            "level": shown,
            "target": if cut { shown } else { after(p, r, levels[r as usize] as f64).max(shown) },
            "cut": cut,
            "wouldCut": dropped.contains(&r),
            "temporary": p.campus.temporary & r.bit() != 0,
        })
    };
    let mut rooms_rows: Vec<Value> = Room::ALL.iter().filter(|r| mask & r.bit() != 0).map(|&r| row(r, false)).collect();
    // Rooms an answered overrun already cut, kept on the page greyed.
    if found.is_some_and(|o| o.fix == Some(Fix::CutRooms)) {
        let first = drive::rooms_for(p, &x.kind, x.size);
        rooms_rows
            .extend(Room::ALL.iter().filter(|r| first & r.bit() != 0 && mask & r.bit() == 0).map(|&r| row(r, true)));
    }

    // The answer to a cost overrun, with what each would do.
    let decision = open_decision.map(|o| {
        let several = x.rooms.count_ones() > 1;
        let cut_words: Vec<&str> = dropped.iter().map(|r| r.words()).collect();
        json!({
            "share": pct(o.share as f64),
            "found": iso(o.found),
            "by": iso(o.found.add(drive::ANSWER_DAYS)),
            "canAnswer": head,
            "options": [
                {
                    "key": "cut", "title": "Cut rooms",
                    "what": if several { format!("The {} go to the open plot as a later project.", cut_words.join(" and the ")) } else { "There is only one room to cut: wait or borrow.".to_string() },
                    "cost": "It opens on time. The rooms you give up are not built now.",
                    "can": several,
                    "drops": dropped.iter().map(|r| *r as u8).collect::<Vec<_>>(),
                },
                {
                    "key": "wait", "title": "Wait and raise more",
                    "what": format!("Open for the {} season instead of {}.", y.opens + 1, y.opens),
                    "cost": "The drive reopens for a year to raise the overrun. One more recruiting class sees a building under way, not a finished one.",
                    "can": true,
                },
                {
                    "key": "borrow", "title": "Borrow the gap",
                    "what": "Your AD takes it in bonds.",
                    "cost": "It opens on time. Football's staff pool is smaller for years.",
                    "can": true,
                },
            ],
            "ifWait": iso(ready.add(365)),
        })
    });
    let answered = found.and_then(|o| o.fix).map(|f| {
        json!({
            "share": pct(over.map_or(0.0, |o| o.share as f64)),
            "fix": match f { Fix::CutRooms => "cut", Fix::Wait => "wait", Fix::Borrow => "borrow" },
            "words": match f {
                Fix::CutRooms => "The smallest rooms were cut to stay on budget.",
                Fix::Wait => "It opens a year later while the drive raises the rest.",
                Fix::Borrow => "Your AD borrowed it. The staff pool will feel it for years.",
            },
        })
    });

    // While it goes up (CP26): the rooms the team does without.
    let without: Vec<&str> =
        Room::ALL.iter().filter(|r| p.campus.temporary & r.bit() != 0).map(|r| r.words()).collect();

    // How it is paid, in shares: pledges against the cost, then bonds.
    let paid = json!({
        "pledged": pct(if y.cost > 0.0 { (y.pledged / y.cost) as f64 } else { 0.0 }).clamp(0, 100),
        "bonds": x.borrowed > 0.0,
        "setBack": x.set_back,
    });

    // The strings: names up on it, then each donor's gift with his call.
    let names: Vec<Value> = w
        .capital
        .names
        .iter()
        .filter(|n| n.drive == x.id && n.program == prog as u32)
        .map(|n| {
            json!({
                "words": n.words(w), "room": n.room.and_then(Room::from_index).map(|r| r.words()),
                "since": n.since, "down": n.down.map(iso),
                "donor": w.capital.donor(n.donor).map(|d| d.name.clone()),
            })
        })
        .collect();
    let mut gifts: Vec<(f64, bool, Value)> = w
        .capital
        .pledges
        .iter()
        .filter(|q| q.drive == x.id)
        .filter_map(|q| {
            let d = w.capital.donor(q.donor)?;
            let met = d.bond(you).map_or(0, |b| b.met);
            let read = screen::catch_read(w, prog, d, met);
            let catch = w.capital.catches.iter().find(|c| c.pledge == q.id && c.called.is_some());
            let (state, call) = match (q.pulled.is_some(), catch) {
                (true, _) => ("pulled", Value::Null),
                (_, Some(c)) if c.answer.is_none() => (
                    "call",
                    json!({
                        "id": c.id, "ask": c.kind.words(),
                        "by": c.called.map(|x| iso(x.add(gifts::CATCH_DAYS))),
                        "adWinces": c.kind == CatchKind::SayInHire,
                        "mayPull": read == "Likes to collect.",
                    }),
                ),
                (_, Some(c)) if c.answer == Some(true) => ("granted", json!({ "ask": c.kind.words() })),
                (_, Some(c)) => ("refused", json!({ "ask": c.kind.words() })),
                _ => ("quiet", Value::Null),
            };
            let waiting = state == "call";
            Some((
                q.total,
                waiting,
                json!({
                    "donor": d.id, "name": d.name, "business": d.business, "state": state, "call": call,
                    "read": read,
                    "tied": q.tied == Some(you),
                    "share": pct(q.total / x.goal.max(1.0)),
                    "room": q.room.and_then(Room::from_index).map(|r| r.words()),
                }),
            ))
        })
        .collect();
    gifts.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.total_cmp(&a.0)));
    let more = gifts.len().saturating_sub(8);
    let gifts: Vec<Value> = gifts.into_iter().take(8).map(|g| g.2).collect();

    Ok(json!({
        "program": name,
        "today": iso(today),
        "head": head,
        "building": {
            "id": x.id,
            "title": title,
            "kind": x.kind,
            "size": x.size.words(),
            "opens": y.opens,
            "ground": iso(ground),
            "ready": iso(ready),
            "found": found.map(|o| iso(o.found)),
            "rooms": rooms_rows,
            "decision": decision,
            "answered": answered,
            "paid": paid,
            "without": without,
            "temporary": !without.is_empty(),
            "names": names,
            "gifts": gifts,
            "moreGifts": more,
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use afm_engine::world::capital::drive::{Overrun, Size, Stage};
    use afm_engine::world::capital::gifts::Catch;
    use afm_engine::world::capital::pledges;
    use afm_engine::world::finance::Project;

    fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
        match g.call(cmd, &args) {
            Ok(v) => v,
            Err(e) => panic!("{cmd}: {e}"),
        }
    }

    /// The project screen on a small world: a building of ours under way
    /// with an overrun to answer and a donor's call, in shares and words
    /// (no dollar anywhere, CP39); each answer works and shows what it did.
    #[test]
    fn the_project_screen_answers_an_overrun_and_a_donors_call() {
        let dir = std::env::temp_dir().join(format!("afm-capital-project-screen-{}", std::process::id()));
        let mut g = Game::new(dir.clone());
        ok(&mut g, "build_world", json!({ "seed": 23, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
        let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
        let job = jobs["jobs"][0]["ref"].clone();
        ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));
        let empty = ok(&mut g, "capital_project", json!({}));
        assert!(empty["building"].is_null(), "nothing is going up yet");
        assert!(g.call("capital_overrun", &json!({ "fix": "wait" })).is_err());

        let (p, catch_id);
        {
            let w = g.wm().unwrap();
            p = week::your_program(w).expect("a head coach's seat");
            let today = w.today;
            let donors: Vec<u32> =
                w.capital.donors.iter().filter(|d| d.alive() && d.home == p as u32).map(|d| d.id).collect();
            assert!(donors.len() >= 2, "the program has donors");
            let dr = Drive {
                id: 9_000_002,
                program: p as u32,
                kind: "football building".into(),
                full_cost: 100.0e6,
                size: Size::Full,
                advised: Some(Size::Full),
                goal: 75.0e6,
                set_by_you: true,
                stage: Stage::Closed,
                closed: Some(today.add(-200)),
                hit: Some(true),
                opened: Some(today.add(-600)),
                ground: Some(w.year),
                ground_day: Some(today.add(-190)),
                opens: Some(w.year + 2),
                rooms: Room::ALL.iter().take(4).fold(0, |m, r| m | r.bit()),
                overrun: Some(Overrun { share: 0.18, found: today.add(-5), fix: None, told: true }),
                ..Drive::default()
            };
            w.capital.drives.push(dr);
            w.finance.projects.push(Project {
                owner: Owner::College(p),
                kind: ProjectKind::Facility,
                name: "football building".into(),
                cost: 100.0e6,
                public: 0.0,
                pledged: 75.0e6,
                started: w.year,
                opens: w.year + 2,
                capacity: 0,
                step: 0.0,
                done: false,
                asked: true,
            });
            let id1 = pledges::make(w, donors[0], p as u32, 30.0e6, 4, None, Some(1), today.add(-300));
            let id2 = pledges::make(w, donors[1], p as u32, 4.0e6, 3, None, None, today.add(-290));
            for id in [id1, id2] {
                w.capital.pledges.iter_mut().find(|q| q.id == id).unwrap().drive = 9_000_002;
            }
            catch_id = 9_000_100;
            w.capital.catches.push(Catch {
                id: catch_id,
                pledge: id1,
                donor: donors[0],
                program: p as u32,
                kind: CatchKind::WalkOn,
                due: today.add(-3),
                called: Some(today.add(-2)),
                answer: None,
            });
        }

        let v = ok(&mut g, "capital_project", json!({}));
        let b = &v["building"];
        assert_eq!(b["title"], "Football building");
        assert_eq!(b["rooms"].as_array().unwrap().len(), 4);
        let dec = &b["decision"];
        assert_eq!(dec["share"], 18);
        assert_eq!(dec["options"].as_array().unwrap().len(), 3);
        assert_eq!(dec["options"][0]["can"], true);
        assert!(!dec["options"][0]["drops"].as_array().unwrap().is_empty(), "cut shows what goes");
        let gifts = b["gifts"].as_array().unwrap();
        assert_eq!(gifts.len(), 2);
        assert_eq!(gifts[0]["state"], "call", "the call that waits for you comes first");
        assert_eq!(gifts[0]["call"]["id"], catch_id);
        assert!(!v.to_string().contains('$'), "no dollar figure on screen: {v}");

        // Grant the donor's call: it is answered and warms him.
        let c = ok(&mut g, "capital_catch", json!({ "id": catch_id, "yes": true }));
        assert!(c["said"].as_str().unwrap().starts_with("Granted"));
        assert_eq!(c["building"]["gifts"][0]["state"], "granted");
        assert!(g.call("capital_catch", &json!({ "id": catch_id, "yes": false })).is_err(), "answered once");

        // Cut rooms: they leave the building and stay on the page, greyed.
        let rooms_before = b["rooms"].as_array().unwrap().len();
        let r = ok(&mut g, "capital_overrun", json!({ "fix": "cut" }));
        assert!(r["building"]["decision"].is_null());
        assert_eq!(r["building"]["answered"]["fix"], "cut");
        let rows = r["building"]["rooms"].as_array().unwrap();
        let cut = rows.iter().filter(|x| x["cut"] == true).count();
        assert!(cut >= 1 && rows.len() >= rooms_before, "cut rooms stay, greyed: {rows:?}");
        assert!(g.call("capital_overrun", &json!({ "fix": "wait" })).is_err(), "one answer");
        assert!(g.call("capital_overrun", &json!({ "fix": "nonsense" })).is_err());
        assert!(!r.to_string().contains('$'));
        let _ = std::fs::remove_dir_all(dir);
    }
}
