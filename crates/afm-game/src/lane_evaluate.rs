//! Lane commands: the scouting fresh look (update 2, lane 8, SK1-SK22 and
//! minor bugs 18-19), in the Press Box look. `call` answers the commands
//! this lane owns and returns None for any other.
//!
//! - `eval_home`: your seat's evaluation desk. Every seat: your trips, your
//!   eye's record, board meetings and who was right in them, your hard
//!   minimums. An NFL club: the May review of your scouts, the scout
//!   market, your service, the analytics staff's record. A college: your
//!   personnel staff and its market, NFL scouts on your campus, your word
//!   and each club's credit for it, your pro day, the advisory list and
//!   your stay-or-go talks. A high school: the athletes in your building.
//! - `eval_file` (`id`): one player's lane 8 side of his file: what his
//!   tape holds, the days you could go, the interview rooms open, your
//!   service's grade, a minimum he falls under, your word on him.
//! - The moves: `eval_review_call`, `eval_review_close`, `eval_sign`,
//!   `eval_hire`, `eval_let_go`, `eval_trip`, `eval_interview`,
//!   `eval_settle`, `eval_minimum`, `eval_service`, `eval_word`,
//!   `eval_openness`, `eval_advisory`, `eval_talk`, `eval_athlete`.
//!
//! Only your staff's reads show: ranges, words and counts.

use std::collections::HashMap;

use afm_engine::positions::Position;
use afm_engine::world::eligibility::{division_of, exhausted_after};
use afm_engine::world::pro::scouts::{LookKind, Role, Scout};
use afm_engine::world::scouting::own::{self, Call, Measure, Service, Topic, TripKind};
use afm_engine::world::scouting::readers::{self, Staff};
use afm_engine::world::scouting::{self as sc, footage, lookback};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use crate::views::{self, date_text};
use crate::{Game, Reply};

const COMMANDS: [&str; 18] = [
    "eval_home",
    "eval_file",
    "eval_review_call",
    "eval_review_close",
    "eval_sign",
    "eval_hire",
    "eval_let_go",
    "eval_trip",
    "eval_interview",
    "eval_interview_talk",
    "eval_settle",
    "eval_minimum",
    "eval_service",
    "eval_word",
    "eval_openness",
    "eval_advisory",
    "eval_talk",
    "eval_athlete",
];

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    if !COMMANDS.contains(&cmd) {
        return None;
    }
    let num = |k: &str| args.get(k).and_then(Value::as_u64);
    let text = |k: &str| args.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let id = || num("id").map(|v| v as u32).ok_or_else(|| "Missing id.".to_string());
    Some(match cmd {
        "eval_home" => game.w().map(home),
        "eval_file" => (|| file(game.w()?, id()?))(),
        // The room as a timed talk (lane 14, D8): a scene you play.
        "eval_interview_talk" => (|| {
            let w = game.wm()?;
            afm_engine::world::folk::dialogue::start_combine_talk(w, id()?, &text("room"))
                .map(|sid| json!({ "scene": sid }))
        })(),
        _ => (|| {
            let w = game.wm()?;
            let said = match cmd {
                "eval_review_call" => {
                    let c = Call::from_key(&text("call")).ok_or("Unknown call.")?;
                    own::review_call(w, num("place").ok_or("Missing place.")? as u8, c)?
                }
                "eval_review_close" => own::close_review(w)?,
                "eval_sign" => own::sign(w, num("k").ok_or("Missing scout.")? as usize, num("place").map(|p| p as u8))?,
                "eval_hire" => own::hire_personnel(w, num("key").ok_or("Missing key.")?)?,
                "eval_let_go" => own::let_go_personnel(w, num("post").ok_or("Missing post.")? as usize)?,
                "eval_trip" => {
                    let kind = TripKind::from_key(&text("kind")).ok_or("Unknown trip.")?;
                    own::go(w, id()?, kind, num("send").map(|s| s as u32))?
                }
                "eval_interview" => {
                    let spent: Vec<(Topic, u8)> = own::TOPICS
                        .iter()
                        .map(|&t| {
                            (
                                t,
                                args.get("spent").and_then(|s| s.get(t.key())).and_then(Value::as_u64).unwrap_or(0)
                                    as u8,
                            )
                        })
                        .filter(|x| x.1 > 0)
                        .collect();
                    own::interview(w, id()?, &text("room"), spent)?
                }
                "eval_settle" => own::settle(
                    w,
                    num("meeting").ok_or("Missing meeting.")? as usize,
                    id()?,
                    num("side").unwrap_or(3) as u8,
                )?,
                "eval_minimum" => {
                    let pos =
                        Position::ALL.iter().copied().find(|p| p.name() == text("pos")).ok_or("Unknown position.")?;
                    let m = Measure::from_key(&text("measure")).ok_or("Unknown measure.")?;
                    let v = args.get("value").and_then(Value::as_f64);
                    own::set_minimum(w, pos, m, v, args.get("drop").and_then(Value::as_bool).unwrap_or(false))?
                }
                "eval_service" => own::set_service(w, Service::from_key(&text("service")).ok_or("Unknown service.")?)?,
                "eval_word" => own::tell(
                    w,
                    num("club").ok_or("Missing club.")? as usize,
                    id()?,
                    num("strength").unwrap_or(0) as u8,
                )?,
                "eval_openness" => own::set_openness(w, num("level").unwrap_or(1) as u8)?,
                "eval_advisory" => {
                    own::request_advisory(w, id()?, args.get("on").and_then(Value::as_bool).unwrap_or(true))?
                }
                "eval_talk" => own::stay_or_go(w, id()?, num("say").unwrap_or(1) as u8)?,
                _ => own::recruit_athlete(w, num("key").ok_or("Missing key.")?)?,
            };
            let mut out = match args.get("id").and_then(Value::as_u64) {
                Some(p) if cmd != "eval_athlete" => file(w, p as u32)?,
                _ => home(w),
            };
            out["said"] = json!(said);
            Ok(out)
        })(),
    })
}

// --- small words --------------------------------------------------------------------------

fn money_k(k: u32) -> String {
    if k >= 1000 { format!("${:.2}M", k as f64 / 1000.0) } else { format!("${k}k") }
}

fn record(held: u32, high: u32, low: u32) -> Value {
    let n = held + high + low;
    json!({ "held": held, "high": high, "low": low, "n": n, "words": if n == 0 { "No look-backs yet".to_string() } else { format!("{held} of {n} held up") } })
}

/// A record from his past jobs: how many of his settled reads held up.
fn career(held: u32, missed: u32) -> Value {
    let n = held + missed;
    json!({ "held": held, "high": 0, "low": 0, "n": n, "missed": missed, "words": if n == 0 { "No look-backs yet".to_string() } else { format!("{held} of {n} held up") } })
}

fn contacts_words(c: f32) -> &'static str {
    match c {
        c if c >= 0.7 => "Knows everyone in his region",
        c if c >= 0.4 => "Well connected in his region",
        c if c > 0.0 => "Still building his contacts",
        _ => "New to his region",
    }
}

fn cred_words(c: f64) -> &'static str {
    match c {
        c if c >= 0.7 => "Trusts your word",
        c if c >= 0.45 => "Listens to you",
        c if c >= 0.25 => "Takes your word with salt",
        _ => "Doesn't believe you",
    }
}

fn person_row(w: &World, i: usize) -> Value {
    let p = &w.people[i];
    json!({ "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "class": views::class_text(w, p) })
}

fn reader_name(w: &World, staff: Staff, code: u32) -> String {
    readers::reader(w, staff, code, 0).map_or("Your staff".into(), |r| r.name)
}

fn seat_word(staff: Staff) -> &'static str {
    match staff {
        Staff::Club(_) => "nfl",
        Staff::Program(_) => "college",
        Staff::School(_) => "hs",
    }
}

// --- the desk -------------------------------------------------------------------------------

fn home(w: &World) -> Value {
    let Some(staff) = readers::seat_staff(w) else { return json!({ "seat": Value::Null }) };
    let o = &w.scouting.own;
    // The latest first: planned days ahead, then the ones done.
    let mut by_day: Vec<_> = o.trips.iter().collect();
    by_day.sort_by_key(|t| std::cmp::Reverse(t.date.0));
    let trips: Vec<Value> = by_day
        .into_iter()
        .take(24)
        .filter_map(|t| {
            let i = sc::place(w, t.id)?;
            Some(json!({
                "date": date_text(t.date), "player": person_row(w, i), "kind": t.kind.words(),
                "done": t.done, "seen": t.seen, "sent": t.sent.map(|c| reader_name(w, staff, c)),
            }))
        })
        .collect();
    // Your eye: your own looks settled by the look-back (SK14).
    let gain = o.eye_gain;
    let eye = json!({
        "credited": o.eye_credited.len(),
        "words": match gain {
            g if g >= 2.0 => "Your own looks have made you a much sharper judge.",
            g if g >= 0.8 => "Your own looks have sharpened your eye.",
            g if g > 0.0 => "Your first look-backs are teaching you something.",
            _ => "Your own looks haven't had their look-backs yet.",
        },
    });
    let rec = own::meeting_record(w);
    let mut right: Vec<(u32, u32, u32)> = rec.iter().map(|(&k, &(r, wr))| (k, r, wr)).collect();
    right.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)).then(a.0.cmp(&b.0)));
    let meeting_record: Vec<Value> = right
        .into_iter()
        .take(8)
        .map(|(k, r, wr)| json!({ "name": reader_name(w, staff, k), "right": r, "wrong": wr }))
        .collect();
    let meetings: Vec<Value> = o
        .meetings
        .iter()
        .enumerate()
        .rev()
        .take(2)
        .map(|(m, x)| {
            let items: Vec<Value> = x
                .items
                .iter()
                .map(|it| {
                    json!({
                        "id": it.id, "name": it.name, "pos": it.pos.name(), "by": it.by_name, "manner": manner_words(it.conviction),
                        "high": { "name": it.high.1, "read": [it.high.2, it.high.3] },
                        "low": { "name": it.low.1, "read": [it.low.2, it.low.3] },
                        "settled": it.settled,
                    })
                })
                .collect();
            json!({ "index": m, "date": date_text(x.date), "kind": x.kind, "items": items })
        })
        .collect();
    let minimums: Vec<Value> = o
        .minimums
        .iter()
        .map(|m| json!({ "pos": m.pos.name(), "measure": m.measure.key(), "words": m.measure.words(), "value": m.value, "drop": m.drop }))
        .collect();
    let mut out = json!({
        "seat": seat_word(staff),
        "year": w.year,
        "today": date_text(w.today),
        "trips": trips,
        "eye": eye,
        "meetings": meetings,
        "meeting_record": meeting_record,
        "minimums": minimums,
        "measures": own::MEASURES.iter().map(|m| json!({ "key": m.key(), "words": m.words() })).collect::<Vec<_>>(),
        "positions": Position::ALL.iter().filter(|p| !p.is_specialist()).map(|p| p.name()).collect::<Vec<_>>(),
    });
    match staff {
        Staff::Club(t) => out["nfl"] = nfl_desk(w, t),
        Staff::Program(c) => out["college"] = college_desk(w, c),
        Staff::School(h) => out["hs"] = hs_desk(w, h),
    }
    out
}

/// How a presenter stands by his read, in words a room learns (an NFL
/// scout's conviction; a college staff's people have none recorded).
fn manner_words(conviction: u8) -> &'static str {
    match conviction {
        0 => "",
        15.. => "pounds the table",
        1..=6 => "hedges",
        _ => "",
    }
}

fn scout_row(w: &World, s: &Scout, place: Option<usize>) -> Value {
    let areas = w
        .seat
        .and_then(|x| x.nfl_team)
        .and_then(|t| w.clubs.get(t))
        .map_or(1, |c| c.scouts.iter().filter(|s| s.role == Role::Area).count().max(1));
    // His regions on your staff; a man on the market has none yet.
    let area = (s.role == Role::Area && place.is_some()).then(|| {
        afm_engine::world::pro::scouts::covers(s.region, areas)
            .into_iter()
            .map(|r| crate::lane_scouting::REGION_NAMES[r as usize])
            .collect::<Vec<_>>()
            .join(", ")
    });
    let clubs: Vec<Value> = s
        .clubs
        .iter()
        .filter_map(|&c| {
            w.nfl.get(c as usize).map(|t| json!({ "name": t.name.to_string(), "ref": { "kind": "nfl", "id": c } }))
        })
        .collect();
    json!({
        "place": place,
        "name": s.name,
        "job": s.job(),
        "age": s.age(w.year),
        "since": s.since,
        "area": area,
        "contacts": (s.role == Role::Area).then(|| contacts_words(s.contacts)),
        "pay": money_k(s.pay.max(1)),
        "market": money_k(s.market_pay(w.year)),
        "underpaid": s.pay > 0 && s.market_pay(w.year) as f64 > 1.15 * s.pay as f64,
        "record": career(s.held as u32, s.missed as u32),
        "clubs": clubs,
    })
}

fn nfl_desk(w: &World, t: usize) -> Value {
    let o = &w.scouting.own;
    let club = &w.clubs[t];
    // Your scouts' look-backs with you, and their record before.
    let with_you = |row: &mut Value, k: usize, s: &Scout| {
        let (h, hi, lo) = lookback::record_of(&w.scouting.records, k as u32);
        row["record"] = record(h, hi, lo);
        let n = (s.held + s.missed) as u32;
        row["before"] = json!((n > 0).then(|| format!("{} of {n} held up before", s.held)));
    };
    let staff: Vec<Value> = club
        .scouts
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let mut row = scout_row(w, s, Some(k));
            with_you(&mut row, k, s);
            row
        })
        .collect();
    let review = o.review.as_ref().map(|r| {
        let scouts: Vec<Value> = club
            .scouts
            .iter()
            .enumerate()
            .map(|(k, s)| {
                let mut row = scout_row(w, s, Some(k));
                with_you(&mut row, k, s);
                let call = r.calls.iter().find(|c| c.0 as usize == k).map_or(Call::Undecided, |c| c.1);
                row["call"] = json!(call.key());
                row["call_words"] = json!(call.words());
                row
            })
            .collect();
        let hires: Vec<Value> = r
            .hires
            .iter()
            .map(|(p, s)| {
                let mut row = scout_row(w, s, None);
                row["post"] = json!(if *p == 255 {
                    "A new post".to_string()
                } else {
                    format!("In place of {}", club.scouts.get(*p as usize).map_or("", |s| s.name.as_str()))
                });
                row
            })
            .collect();
        let calls: Vec<Value> = [Call::Renew, Call::Raise, Call::Promote, Call::LetGo]
            .iter()
            .map(|c| json!({ "key": c.key(), "words": c.words() }))
            .collect();
        json!({ "due": date_text(r.due), "scouts": scouts, "hires": hires,
            "calls": calls })
    });
    // The screen sorts the market by what a club can see (the pay his
    // record asks), never by the hidden standing the store keeps.
    let mut by_pay: Vec<(usize, &Scout)> = own::market(w).iter().enumerate().collect();
    by_pay.sort_by_key(|&(k, s)| (std::cmp::Reverse(s.market_pay(w.year)), k));
    let market: Vec<Value> = by_pay
        .into_iter()
        .take(40)
        .map(|(k, s)| {
            let mut row = scout_row(w, s, None);
            row["k"] = json!(k);
            row
        })
        .collect();

    let svc = own::service_of(w, t);
    let (h, hi, lo) = lookback::numbers_record(&w.scouting.records);
    let options: Vec<Value> = [Service::National, Service::Blesto, Service::None]
        .iter()
        .map(|s| json!({ "key": s.key(), "words": s.words() }))
        .collect();
    json!({
        "review": review,
        "market": market,
        "staff": staff,
        "service": {
            "now": svc.key(), "words": svc.words(), "next": o.service_next.map(|s| s.words()),
            "fee": format!("${:.0}k a year", own::service_fee(w.year) / 1000.0),
            "options": options,
        },
        "analytics": {
            "has": readers::has_analytics(w, Staff::Club(t)),
            "name": club.support.analytics.name,
            "record": record(h, hi, lo),
        },
    })
}

fn college_desk(w: &World, c: usize) -> Value {
    let o = &w.scouting.own;
    let staff: Vec<Value> = readers::personnel(w, c)
        .into_iter()
        .enumerate()
        .map(|(k, p)| json!({ "post": k, "name": p.name, "job": p.job, "since": p.since }))
        .collect();
    let market: Vec<Value> = own::personnel_market(w)
        .into_iter()
        .map(|h| {
            let past: Vec<Value> = h
                .past
                .iter()
                .filter(|&&q| (q as usize) < w.programs.len())
                .map(|&q| json!({ "name": w.program_name(q as usize).to_string(), "ref": { "kind": "college", "id": q } }))
                .collect();
            json!({
                "key": h.key, "name": h.name, "job": own::PERSONNEL_JOBS[h.job as usize % own::PERSONNEL_JOBS.len()],
                "age": w.year - h.born, "past": past, "record": career(h.held as u32, h.missed as u32),
            })
        })
        .collect();
    // NFL scouts on your campus this year (SK19): their traffic by player.
    let since = afm_engine::calendar::Date::ymd(w.year, 8, 1);
    let mut traffic: HashMap<u32, (u32, u32, Vec<u8>)> = HashMap::new();
    for (t, club) in w.clubs.iter().enumerate() {
        for (&id, looks) in &club.files {
            let recent: Vec<&afm_engine::world::pro::scouts::Look> = looks.iter().filter(|l| l.date >= since).collect();
            if recent.is_empty() {
                continue;
            }
            let Some(i) = sc::place(w, id) else { continue };
            if w.people[i].college != Some(c) {
                continue;
            }
            let e = traffic.entry(id).or_default();
            if recent.iter().any(|l| l.kind == LookKind::School) {
                e.0 += 1;
                e.2.push(t as u8);
            }
            // A club that sent its head coach, a coordinator or its GM.
            e.1 += recent.iter().any(|l| matches!(l.kind, LookKind::Coach | LookKind::Gm)) as u32;
        }
    }
    let mut campus: Vec<(u32, u32, u32)> = traffic.iter().map(|(&id, x)| (id, x.0, x.1)).collect();
    campus.sort_by(|a, b| (b.1 + 2 * b.2).cmp(&(a.1 + 2 * a.2)).then(a.0.cmp(&b.0)));
    let campus: Vec<Value> = campus
        .into_iter()
        .take(25)
        .filter_map(|(id, clubs, heads)| {
            let i = sc::place(w, id)?;
            let words: Vec<Value> = o
                .words
                .iter()
                .filter(|x| x.id == id)
                .map(|x| json!({ "club": w.nfl.get(x.club as usize).map(|t| t.name.to_string()), "strength": x.strength, "held": x.held }))
                .collect();
            let mut row = person_row(w, i);
            row["clubs"] = json!(clubs);
            row["heads"] = json!(heads);
            row["words"] = json!(words);
            Some(row)
        })
        .collect();
    let clubs: Vec<Value> = (0..w.nfl.len())
        .filter(|&t| afm_engine::world::franchise::active_teams_in(w, w.year).contains(&t))
        .map(|t| {
            let cr = own::cred(w, t);
            json!({ "club": t, "name": w.nfl[t].name.to_string(), "cred": (cr * 100.0).round(), "words": cred_words(cr) })
        })
        .collect();
    let id = w.programs[c].id as u16;
    let pro_day = w.scouting.pro_days.iter().rev().find(|x| x.program == id).map(|x| {
        json!({
            "year": x.year, "clubs": x.clubs, "heads": x.heads, "tested": x.tested,
            "head_clubs": x.head_clubs.iter().filter_map(|&t| w.nfl.get(t as usize).map(|n| json!({ "name": n.name.to_string(), "ref": { "kind": "nfl", "id": t } }))).collect::<Vec<_>>(),
            "players": x.players.iter().filter_map(|&p| sc::place(w, p).map(|i| person_row(w, i))).collect::<Vec<_>>(),
        })
    });
    // Your underclassmen with an NFL read: the advisory list and your talks (SK21).
    let year = w.year;
    let sent: Vec<u32> = o.advisory.iter().filter(|x| x.1 == year).map(|x| x.0).collect();
    let mut juniors: Vec<Value> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            p.college == Some(c) && p.stage == Stage::College && p.cal.draft_read.is_some_and(|r| r.round <= 7)
        })
        // Any underclassman with college football left, by the same test the
        // advisory committee uses (a redshirt junior too).
        .filter(|(_, p)| !exhausted_after(p, division_of(p, &w.programs), year))
        .map(|(i, p)| {
            let r = p.cal.draft_read.unwrap();
            let mut row = person_row(w, i);
            row["nfl_word"] = json!(format!("Round {}", r.round));
            row["sent"] = json!(sent.contains(&p.player.id));
            row["grade"] = json!(p.cal.advisory.map(|g| match (g, year >= 2014) {
                (1, _) => "First round".to_string(),
                (2, _) => "Second round".to_string(),
                (_, true) => "Return to school".to_string(),
                (r @ 3..=7, false) => format!("Round {r}"),
                _ => "Not drafted".to_string(),
            }));
            row["talk"] = json!(own::talk_of(w, p.player.id));
            // Sorted by the word he has (the round), never the hidden score.
            row["score"] = json!(-(r.round as f64));
            row
        })
        .collect();
    juniors.sort_by(|a, b| b["score"].as_f64().unwrap_or(0.0).total_cmp(&a["score"].as_f64().unwrap_or(0.0)));
    for j in juniors.iter_mut() {
        if let Some(o) = j.as_object_mut() {
            o.remove("score");
        }
    }
    json!({
        "personnel": staff,
        "personnel_market": market,
        "openness": o.openness,
        "campus": campus,
        "clubs": clubs,
        "pro_day": pro_day,
        "advisory": {
            "limit": own::advisory_limit(year),
            "committee": year >= 1994,
            "sent": sent.len(),
            "juniors": juniors,
        },
    })
}

fn hs_desk(w: &World, _h: usize) -> Value {
    let o = &w.scouting.own;
    let athletes: Vec<Value> = own::athletes(w)
        .into_iter()
        .map(|a| {
            json!({
                "key": a.key, "name": a.name, "sport": own::SPORTS[a.sport].0, "pos": a.pos.name(),
                "class_of": a.class_of, "tools": [a.tools.0, a.tools.1], "spotted_by": a.spotted_by,
            })
        })
        .collect();
    let talks: Vec<Value> = o
        .athletes
        .iter()
        .rev()
        .take(12)
        .map(|t| {
            let who = t.id.and_then(|id| sc::place(w, id)).map(|i| person_row(w, i));
            json!({ "date": date_text(t.date), "yes": t.yes, "player": who, "name": t.name, "sport": t.sport })
        })
        .collect();
    json!({ "athletes": athletes, "talks": talks })
}

// --- one player's file ----------------------------------------------------------------------

fn file(w: &World, id: u32) -> Reply {
    let staff = readers::seat_staff(w).ok_or("Start a career first.")?;
    let i = sc::place(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    let pos = p.player.position;
    let read = readers::read(w, staff, id);
    // What his tape holds (SK1-SK5), once your staff has a file.
    let tape: Vec<String> = match &read {
        Some((_, looks, _)) => {
            let (good, reel) = afm_engine::world::scouting::file::tape_flags(looks);
            footage::reasons(&footage::footage(w, i), pos, good, reel)
        }
        None => Vec::new(),
    };
    let trips: Vec<Value> = own::trip_kinds(staff)
        .iter()
        .map(|&k| match own::trip_day(w, id, k) {
            Ok(d) => json!({ "kind": k.key(), "words": k.words(), "date": date_text(d) }),
            Err(e) => json!({ "kind": k.key(), "words": k.words(), "why": e }),
        })
        .collect();
    let planned: Vec<Value> = w
        .scouting
        .own
        .trips
        .iter()
        .filter(|t| t.id == id)
        .map(|t| json!({ "date": date_text(t.date), "kind": t.kind.words(), "done": t.done, "seen": t.seen, "sent": t.sent.map(|c| reader_name(w, staff, c)) }))
        .collect();
    let rooms: Vec<Value> = own::rooms(w, id).into_iter().map(|(r, m)| json!({ "room": r, "minutes": m })).collect();
    let interviews: Vec<Value> = w
        .scouting
        .own
        .interviews
        .iter()
        .filter(|x| x.id == id)
        .map(|x| {
            json!({
                "date": date_text(x.date), "room": x.room, "minutes": x.minutes,
                "answers": x.answers.iter().map(|(t, l)| json!({ "topic": t.words(), "line": l })).collect::<Vec<_>>(),
                "quiz": x.quiz.map(|q| format!("He drew back {} of {} assignments.", q.0, q.1)),
            })
        })
        .collect();
    let topics: Vec<Value> = own::TOPICS.iter().map(|t| json!({ "key": t.key(), "words": t.words() })).collect();
    let mut out = json!({
        "id": id,
        "name": p.name,
        "pos": pos.name(),
        "seat": seat_word(staff),
        "tape": tape,
        "trips": trips,
        "planned": planned,
        "rooms": rooms,
        "interviews": interviews,
        "topics": topics,
        "under": own::under(w, i, &w.scouting.own.minimums).map(|m| format!("Under your {} line at {} ({}).", m.measure.words().to_lowercase(), m.pos.name(), m.value)),
    });
    match staff {
        Staff::Club(t) => {
            let svc = own::service_of(w, t);
            out["service"] = json!(own::service_grade(w, svc, i).map(|g| json!({ "name": svc.words(), "grade": g })));
        }
        Staff::Program(c) if p.college == Some(c) && matches!(p.stage, Stage::College | Stage::Declared) => {
            let clubs: Vec<Value> = afm_engine::world::franchise::active_teams_in(w, w.year)
                .into_iter()
                .filter(|&t| t < w.nfl.len())
                .map(|t| {
                    let cr = own::cred(w, t);
                    let given = w
                        .scouting
                        .own
                        .words
                        .iter()
                        .find(|x| x.club as usize == t && x.id == id && x.date.year() == w.year)
                        .map(|x| x.strength);
                    json!({ "club": t, "name": w.nfl[t].name.to_string(), "words": cred_words(cr), "given": given })
                })
                .collect();
            out["word"] = json!({ "clubs": clubs, "strengths": ["Worth a look", "Can play for you", "A sure thing"] });
            out["talk"] = json!(own::talk_of(w, id));
        }
        _ => {}
    }
    out["send"] = json!(sendable(w, staff));
    Ok(out)
}

/// The staff you can send in your place (SK13): your scouts at a club, your
/// assistants at a college.
fn sendable(w: &World, staff: Staff) -> Vec<Value> {
    match staff {
        Staff::Club(t) => w.clubs.get(t).map_or(Vec::new(), |c| {
            c.scouts.iter().enumerate().map(|(k, s)| json!({ "code": k, "name": s.name, "job": s.job() })).collect()
        }),
        Staff::Program(c) => {
            let g = &w.programs[c];
            [g.oc, g.dc]
                .into_iter()
                .chain(g.position_coaches)
                .flatten()
                .map(|k| json!({ "code": readers::COACH + k as u32, "name": w.coaches[k].name, "job": "Assistant coach" }))
                .collect()
        }
        Staff::School(_) => Vec::new(),
    }
}
