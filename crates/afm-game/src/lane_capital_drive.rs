//! The drive screen's commands (lane CS3; `docs/design/capital.md`, CP12,
//! CP37, CP39, board 3): your program's capital campaign for its building,
//! in its real stages, every pledge with the moment that brought it, the
//! lead donors still waiting for you and your AD's read. The calls on it
//! are the engine's (`world::capital::drive`).
//!
//! - `capital_drive`: the view (no dollar figure anywhere, CP39: money
//!   reads as shares of the goal and in words).
//! - `capital_goal` {size "small" | "mid" | "full"}: set the goal with your
//!   AD before the drive goes public.
//! - `capital_public`: take the drive public now (early if under half).
//! - `capital_hands` {on}: work the lead donors yourself, or leave the
//!   drive to the development office.
//! - `capital_borrow` {bigger}: push your AD to borrow and break ground.
//!
//! The overrun, the catches and the building itself are the project
//! screen's (lane CS4).

use afm_engine::calendar::Date;
use afm_engine::world::World;
use afm_engine::world::capital::drive::{self, Drive, Size, Stage};
use afm_engine::world::capital::fever::What;
use afm_engine::world::capital::rooms::Room;
use afm_engine::world::capital::{scenes as cscenes, screen, week};
use afm_engine::world::finance::Owner;
use afm_engine::world::folk::{self, Team, boss};
use serde_json::{Value, json};

use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        "capital_drive" => game.w().and_then(view),
        "capital_goal" => game.wm().and_then(|w| {
            let p = week::your_program(w).ok_or("The drive is the head coach's.")?;
            let size = match args.get("size").and_then(Value::as_str) {
                Some("small") => Size::Small,
                Some("mid") => Size::Mid,
                Some("full") => Size::Full,
                _ => return Err("Which size?".to_string()),
            };
            let said = drive::set_goal(w, p, size)?;
            done(w, said)
        }),
        "capital_public" => game.wm().and_then(|w| {
            let p = week::your_program(w).ok_or("The drive is the head coach's.")?;
            let said = drive::go_public(w, p)?;
            done(w, said)
        }),
        "capital_hands" => game.wm().and_then(|w| {
            let p = week::your_program(w).ok_or("The drive is the head coach's.")?;
            let on = args.get("on").and_then(Value::as_bool).ok_or("On or off?")?;
            let said = drive::hands_on(w, p, on)?;
            done(w, said)
        }),
        "capital_borrow" => game.wm().and_then(|w| {
            let p = week::your_program(w).ok_or("The drive is the head coach's.")?;
            let bigger = args.get("bigger").and_then(Value::as_bool).unwrap_or(false);
            let said = drive::push_borrow(w, p, bigger)?;
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

fn size_key(s: Size) -> &'static str {
    match s {
        Size::Small => "small",
        Size::Mid => "mid",
        Size::Full => "full",
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// The building's name on screen: "The full football building".
fn building(x: &Drive) -> String {
    match x.size {
        Size::Full => format!("The full {}", x.kind),
        Size::Mid => format!("A mid-size {}", x.kind),
        Size::Small => format!("A smaller {}", x.kind),
    }
}

fn stage_key(s: Stage) -> &'static str {
    match s {
        Stage::Study => "study",
        Stage::Quiet => "quiet",
        Stage::Public => "public",
        Stage::Closed => "closed",
    }
}

/// A gift's size in words, from its share of the goal (CP39: no dollars).
fn size_words(share: f64) -> &'static str {
    match share {
        s if s >= 0.08 => "Lead gift",
        s if s >= 0.04 => "A wing",
        s if s >= 0.015 => "A room",
        _ => "A gift",
    }
}

fn pct(x: f64) -> i64 {
    (x * 100.0).round() as i64
}

/// What brought a gift: the moment the fever held the day he said yes, else
/// the stage the drive was in.
fn brought(w: &World, x: &Drive, made: Option<Date>, moment: Option<(What, Option<u32>)>) -> String {
    if let Some((what, who)) = moment {
        let name = who.and_then(|id| folk::person(w, id)).map(|i| w.people[i].name.clone());
        return match (what, name) {
            (What::TopTenPick, Some(n)) => format!("{n}, top-10 pick"),
            (What::Heisman, Some(n)) => format!("{n}, the Heisman"),
            (What::NationalTitle, _) => "National title".into(),
            (What::ConferenceTitle, _) => "Conference title".into(),
            (What::Playoff, _) => "The playoff".into(),
            (What::BowlWin, _) => "Bowl win".into(),
            (What::RivalryWin, _) => "Rivalry win".into(),
            (w2, _) => cap(w2.words()),
        };
    }
    match (made, x.public_on) {
        (Some(m), Some(p)) if m >= p => "Public phase".into(),
        _ => "Quiet phase".into(),
    }
}

/// Your AD's read of the drive, in his own words, from its real state.
fn ad_says(x: &Drive, got: f64) -> (String, String) {
    let share = if x.goal > 0.0 { got / x.goal } else { 0.0 };
    let line = if x.shelved {
        "We tried. The donors weren't there yet. Give it a few years and we'll try again."
    } else if x.stage == Stage::Closed && x.hit == Some(true) {
        if x.bold() {
            "You hit it, and you hit it bold. That's the kind of number I'll go to the president with."
        } else {
            "You hit the number. The building is going up."
        }
    } else if x.stage == Stage::Closed && x.hit == Some(false) {
        "We fell short of the number. The building's smaller than we said in the paper, and people noticed."
    } else if x.stage == Stage::Study {
        "The office is reading the donors. I won't put a number in the paper until the study's in."
    } else if x.stage == Stage::Quiet && share < drive::LAUNCH {
        "The lead gifts come first. We go public at half the goal, not before."
    } else if x.stage == Stage::Quiet {
        "Half is in. We can go public whenever you say."
    } else if x.stalled_until.is_some() {
        "The season cost us. Gifts come slow until we win again."
    } else if x.early {
        "You put the number in the paper early. I backed you. A losing season now and everyone will say so."
    } else if x.bold() {
        "You put a big number in the paper. I backed you. I need the lead gifts in."
    } else {
        "The campaign is in the open. Keep the lead donors warm and the rest follows."
    };
    let fine = if x.stage == Stage::Closed {
        "His trust in you moved when the drive closed."
    } else {
        "A goal hit lifts his trust in you, more if it was bold. A miss of more than 15% costs some of it back."
    };
    (line.to_string(), fine.to_string())
}

fn busy(w: &World, p: usize) -> bool {
    w.finance.projects.iter().any(|x| x.owner == Owner::College(p) && !x.done)
}

fn view(w: &World) -> Reply {
    let p = week::your_program(w).ok_or("The drive is the head coach's to run.")?;
    let name = w.programs[p].name(w.options.names).to_string();
    let live = afm_engine::world::capital::drive::latest(&w.capital, p);
    let out = |drive: Value| json!({ "program": name, "today": iso(w.today), "drive": drive, "ready": !w.capital.drives.is_empty() });
    let Some(x) = live else { return Ok(out(Value::Null)) };

    let you = w.career.as_ref().map(|c| c.coach as u32).unwrap_or(u32::MAX);
    let goal = x.goal.max(1.0);
    let mine: Vec<&afm_engine::world::capital::Pledge> = w.capital.pledges.iter().filter(|q| q.drive == x.id).collect();
    let paid: f64 = mine.iter().map(|q| q.paid).sum::<f64>() + x.crowd;
    let owed: f64 = mine.iter().map(|q| q.left()).sum();
    let pulled: f64 = mine.iter().filter(|q| q.pulled.is_some()).map(|q| (q.total - q.paid).max(0.0)).sum();
    let got = drive::raised(&w.capital, x);
    let years = {
        let ys: Vec<u8> = mine.iter().filter(|q| q.open()).map(|q| q.years).collect();
        (ys.iter().min().copied(), ys.iter().max().copied())
    };

    // The pledges, newest first, then the small gifts.
    let mut gifts: Vec<(Option<Date>, Value)> = mine
        .iter()
        .map(|q| {
            let share = q.total / goal;
            let donor = w.capital.donor(q.donor);
            let lead = x.lead.contains(&q.donor);
            let room = q.room.and_then(Room::from_index).map(|r| cap(r.words()));
            (
                q.made,
                json!({
                    "when": q.made.map(iso),
                    "who": donor.map_or("A donor".to_string(), |d| d.name.clone()),
                    "size": if lead { "Lead gift" } else { size_words(share) },
                    "share": pct(share),
                    "what": room.unwrap_or_else(|| "The building".to_string()),
                    "moment": brought(w, x, q.made, q.moment),
                    "tied": q.tied == Some(you),
                    "state": if q.pulled.is_some() { "pulled" } else if q.open() { "pledged" } else { "paid" },
                }),
            )
        })
        .collect();
    if x.crowd > 0.0 {
        gifts.push((
            x.public_on.or(x.since),
            json!({
                "when": x.public_on.or(x.since).map(iso),
                "who": "The annual fund",
                "size": "Many small gifts",
                "share": pct(x.crowd / goal),
                "what": "The building",
                "moment": if x.public_on.is_some() { "Public launch" } else { "Quiet phase" },
                "tied": false,
                "state": "paid",
            }),
        ));
    }
    gifts.sort_by_key(|g| std::cmp::Reverse(g.0));
    let gifts: Vec<Value> = gifts.into_iter().map(|g| g.1).collect();

    // The lead donors who haven't given: they need you in the room.
    let waiting: Vec<Value> = x
        .lead
        .iter()
        .filter(|id| !x.asked.contains(&(**id, drive::GAVE)))
        .filter_map(|&id| w.capital.donor(id))
        .map(|d| {
            let met = d.bond(you).map_or(0, |b| b.met);
            let rooms: Vec<String> = d
                .rooms
                .iter()
                .filter_map(|&r| Room::from_index(r))
                .filter(|r| x.rooms & r.bit() != 0)
                .take(2)
                .map(|r| r.words().to_string())
                .collect();
            json!({
                "id": d.id,
                "name": d.name,
                "business": d.business,
                "read": screen::read(w, p, d, met).words,
                "withYou": cscenes::warmth_words(d.warmth_to(you)),
                "rooms": rooms,
                "met": met,
            })
        })
        .collect();
    let office = w
        .capital
        .donors
        .iter()
        .filter(|d| d.alive() && (d.home == p as u32 || d.also == Some(p as u32)))
        .filter(|d| !x.lead.contains(&d.id) && !x.asked.contains(&(d.id, drive::GAVE)))
        .count();

    // The stages as a line.
    let stage = x.stage;
    let quiet_to = x.public_on;
    let close_by = x.public_on.map(|d| d.add((drive::PUBLIC_MAX * 30.44) as i32));
    let early_note = if x.early { "Went early" } else { "At half the goal" };
    let stages = json!([
        {
            "key": "study", "label": "Feasibility study",
            "state": if stage > Stage::Study { "done" } else { "now" },
            "date": x.opened.map(iso), "note": "What the donors would give",
        },
        {
            "key": "quiet", "label": "Quiet phase",
            "state": match stage { Stage::Study => "next", Stage::Quiet => "now", _ => "done" },
            "date": quiet_to.or(x.since).map(iso), "note": "Lead gifts",
        },
        {
            "key": "public", "label": if stage == Stage::Public { "Public, now" } else { "Public phase" },
            "state": match stage { Stage::Study | Stage::Quiet => "next", Stage::Public => "now", Stage::Closed => if x.public_on.is_some() { "done" } else { "skipped" } },
            "date": x.public_on.map(iso), "note": if x.public_on.is_some() { early_note } else { "Everyone else, and the small gifts" },
        },
        {
            "key": "close", "label": "The close",
            "state": if stage == Stage::Closed { "done" } else { "next" },
            "date": x.closed.or(close_by).map(iso), "note": if stage == Stage::Closed { if x.shelved { "Shelved" } else if x.hit == Some(true) { "Goal hit" } else { "Short of the goal" } } else { "At the goal, or two public years" },
        },
    ]);

    let (says, fine) = ad_says(x, got);
    let ad = w.programs[p].ad.as_ref().map(|a| a.name.clone());
    let trust = boss::trust(w, Team::College(p));
    let story = {
        let mut s = String::new();
        match x.advised {
            Some(a) if x.size > a => {
                s.push_str(&format!("The study said {} was safe. You went for {}.", a.words(), x.size.words()))
            }
            Some(a) if x.size < a => {
                s.push_str(&format!("The study said {} was there. You set {}.", a.words(), x.size.words()))
            }
            Some(_) => s.push_str(&format!("The study's size: {}.", x.size.words())),
            None => s.push_str("The study said the donors weren't there yet."),
        }
        if x.early {
            s.push_str(" You went public before half was in.");
        }
        s
    };

    // Your calls, only the ones open today.
    let in_study_or_quiet = stage <= Stage::Quiet;
    let sizes: Vec<Value> = if in_study_or_quiet {
        Size::ALL
            .iter()
            .rev()
            .map(|&s| {
                json!({
                    "key": size_key(s), "words": s.words(),
                    "advised": x.advised == Some(s), "current": x.size == s,
                    "bold": x.advised.is_none_or(|a| s > a),
                    "share": pct(s.share()),
                })
            })
            .collect()
    } else {
        vec![]
    };
    let can_borrow = x.ground.is_none()
        && stage >= Stage::Quiet
        && stage != Stage::Closed
        && x.pushed != w.year
        && !busy(w, p)
        && got >= drive::PUSH_BREAK * x.goal;
    let calls = json!({
        "sizes": sizes,
        "public": stage == Stage::Quiet,
        "publicEarly": stage == Stage::Quiet && got < drive::LAUNCH * x.goal,
        "hands": stage != Stage::Closed,
        "handsOn": x.hands_on,
        "borrow": can_borrow,
        "borrowBigger": can_borrow && x.size.up().is_some_and(|up| got >= drive::PUSH_BREAK * drive::goal_for(w, x, up)),
        "borrowBiggerWords": x.size.up().map(|s| s.words()),
    });

    Ok(out(json!({
        "id": x.id,
        "title": building(x),
        "kind": x.kind,
        "size": size_key(x.size),
        "sizeWords": x.size.words(),
        "advised": x.advised.map(|a| a.words()),
        "bold": x.bold(),
        "early": x.early,
        "setByYou": x.set_by_you,
        "stage": stage_key(stage),
        "stageWords": stage.words(),
        "stages": stages,
        "story": story,
        "stalled": x.stalled_until.is_some_and(|s| w.today < s),
        "opened": x.opened.map(iso),
        "publicOn": x.public_on.map(iso),
        "closed": x.closed.map(iso),
        "hit": x.hit,
        "shelved": x.shelved,
        "groundBroke": x.ground_day.map(iso),
        "opens": x.opens,
        "overrun": x.overrun.is_some_and(|o| o.fix.is_none()),
        // The bar, in shares of the goal (CP39).
        "pct": { "paid": pct(paid / goal), "pledged": pct(owed / goal), "pulled": pct(pulled / goal),
                  "toRaise": (100 - pct(got / goal)).max(0), "total": pct(got / goal) },
        "pledgeYears": { "min": years.0, "max": years.1 },
        "gifts": gifts,
        "waiting": waiting,
        "office": office,
        "handsOn": x.hands_on,
        "ad": { "name": ad, "says": says, "fine": fine, "trust": boss::trust_short(trust) },
        "calls": calls,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use afm_engine::world::capital::fever::What;
    use afm_engine::world::capital::pledges;

    fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
        match g.call(cmd, &args) {
            Ok(v) => v,
            Err(e) => panic!("{cmd}: {e}"),
        }
    }

    /// The drive screen on a small world: a drive of ours with pledges, the
    /// bar in shares, the moment on each gift, the lead donors waiting, and
    /// the calls open at each stage. No dollar figure anywhere (CP39).
    #[test]
    fn the_drive_reads_in_shares_with_its_calls() {
        let dir = std::env::temp_dir().join(format!("afm-capital-drive-screen-{}", std::process::id()));
        let mut g = Game::new(dir.clone());
        ok(&mut g, "build_world", json!({ "seed": 23, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
        let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
        let job = jobs["jobs"][0]["ref"].clone();
        ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

        // Set up a drive of ours in its quiet phase, with the donors' gifts.
        let p;
        {
            let w = g.wm().unwrap();
            p = week::your_program(w).expect("a head coach's seat");
            let today = w.today;
            let donors: Vec<u32> =
                w.capital.donors.iter().filter(|d| d.alive() && d.home == p as u32).map(|d| d.id).collect();
            assert!(donors.len() >= 3, "the program has donors");
            let dr = Drive {
                id: 9_000_001,
                program: p as u32,
                kind: "football building".into(),
                full_cost: 100.0e6,
                size: Size::Full,
                advised: Some(Size::Mid),
                goal: 100.0e6,
                set_by_you: true,
                stage: Stage::Quiet,
                opened: Some(today.add(-200)),
                since: Some(today.add(-150)),
                lead: vec![donors[0], donors[1]],
                rooms: 0b11,
                hands_on: true,
                ..Drive::default()
            };
            w.capital.drives.push(dr);
            // A lead gift on a top-10 pick, and a plain one.
            let id1 = pledges::make(w, donors[0], p as u32, 30.0e6, 4, None, Some(1), today.add(-60));
            let id2 = pledges::make(w, donors[2], p as u32, 4.0e6, 3, None, None, today.add(-30));
            for (id, moment) in [(id1, Some((What::TopTenPick, None))), (id2, None)] {
                let q = w.capital.pledges.iter_mut().find(|q| q.id == id).unwrap();
                q.drive = 9_000_001;
                q.moment = moment;
            }
            w.capital.pledges.iter_mut().find(|q| q.id == id1).unwrap().paid = 10.0e6;
            // Gifts mark the donor in the drive's asks (`gifts::give` does).
            let dr = w.capital.drives.last_mut().unwrap();
            dr.asked.push((donors[0], drive::GAVE));
            dr.asked.push((donors[2], drive::GAVE));
        }

        let v = ok(&mut g, "capital_drive", json!({}));
        let d = &v["drive"];
        assert_eq!(d["stage"], "quiet");
        assert_eq!(d["title"], "The full football building");
        // 34M of 100M pledged, 10M of it paid.
        assert_eq!(d["pct"]["paid"], 10);
        assert_eq!(d["pct"]["pledged"], 24);
        assert_eq!(d["pct"]["toRaise"], 66);
        let gifts = d["gifts"].as_array().unwrap();
        assert_eq!(gifts.len(), 2);
        assert_eq!(gifts[0]["moment"], "Quiet phase", "newest first, no moment: the stage");
        assert_eq!(gifts[1]["size"], "Lead gift");
        assert_eq!(gifts[1]["moment"], "A top-10 pick");
        // The second lead donor hasn't given: he waits for you.
        assert_eq!(d["waiting"].as_array().unwrap().len(), 1);
        // Calls open in the quiet phase: the goal's sizes, going public.
        assert_eq!(d["calls"]["sizes"].as_array().unwrap().len(), 3);
        assert_eq!(d["calls"]["public"], true);
        assert_eq!(d["calls"]["publicEarly"], true);
        assert!(d["ad"]["says"].is_string() && d["ad"]["trust"].is_string());
        // No dollar figure on screen.
        assert!(!v.to_string().contains('$'), "{v}");

        // Work it through the office, then set a smaller goal.
        let h = ok(&mut g, "capital_hands", json!({ "on": false }));
        assert_eq!(h["drive"]["handsOn"], false);
        assert!(h["said"].is_string());
        let s = ok(&mut g, "capital_goal", json!({ "size": "mid" }));
        assert_eq!(s["drive"]["size"], "mid");
        assert_eq!(s["drive"]["title"], "A mid-size football building");

        // Go public: early, it says so; then the goal is public.
        let pu = ok(&mut g, "capital_public", json!({}));
        assert_eq!(pu["drive"]["stage"], "public");
        // Early is under half of the (new, smaller) goal.
        let early = s["drive"]["pct"]["total"].as_i64().unwrap() < 50;
        assert_eq!(pu["drive"]["early"], early);
        assert!(pu["said"].as_str().unwrap().contains("public"));
        assert!(g.call("capital_goal", &json!({ "size": "small" })).is_err(), "the goal is public");
        assert_eq!(pu["drive"]["calls"]["sizes"].as_array().unwrap().len(), 0);
        let _ = std::fs::remove_dir_all(dir);
    }
}
