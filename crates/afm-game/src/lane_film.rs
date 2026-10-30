//! Lane commands: the film room and being there (update 3, lane 9;
//! scouting update SK32-SK34, SK36-SK39, SK43-SK63). `call` answers the
//! commands this lane owns and returns None for any other.
//!
//! One film room for all film (SK54): a player's cut-ups (recruits, your
//! own players, draft prospects) and your next opponent's kept tape, on
//! one weekly ledger of film hours (lane 7's, `film::time`). The session
//! is watched snap by snap; the stills come from the engine's tracking of
//! his real snaps (the opponent's from the kept film), never the 2D view.
//! Only your own and your staff's reads show: ranges, words and counts.

use afm_engine::attributes::{ATTRIBUTE_COUNT, Attr};
use afm_engine::positions::Position;
use afm_engine::world::scouting::film::{self as fm, session, tape, there, time};
use afm_engine::world::scouting::{self as sc, readers, yours};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use crate::lane_gameday::{self as gd, film as gdfilm};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let id = || args.get("id").and_then(Value::as_u64).map(|x| x as u32).ok_or_else(|| "Missing id.".to_string());
    let num = |k: &str| args.get(k).and_then(Value::as_u64).ok_or_else(|| format!("Missing {k}."));
    let r = match cmd {
        "film_home" => game.w().map(home),
        "film_brief" => game.w().and_then(|w| brief(w, id()?, args.get("ask").and_then(Value::as_str))),
        "film_session" => game.w().map(session_view),
        "film_open" => game.wm().and_then(|w| {
            let cut = args
                .get("cut")
                .and_then(Value::as_str)
                .and_then(tape::Cut::from_key)
                .unwrap_or(tape::Cut::BestOpponent);
            let mode = match args.get("mode").and_then(Value::as_str) {
                Some("quick") => session::Mode::Quick,
                _ => session::Mode::Full,
            };
            let ask = args.get("ask").and_then(Value::as_str);
            let said = session::open_player(w, id()?, cut, mode, ask)?;
            Ok(with_said(session_view(w), said))
        }),
        "film_opp" => game.wm().and_then(|w| open_opp(w, args)),
        "film_snap" => game.wm().and_then(|w| step(w, session::play)),
        "film_call" => game.wm().and_then(|w| {
            let k = num("pick")? as u8;
            step(w, |w| session::call(w, k))
        }),
        "film_runback" => game.wm().and_then(|w| step(w, session::runback)),
        "film_moveon" => game.wm().and_then(|w| step(w, session::move_on)),
        "film_mark" => game.wm().and_then(|w| {
            let k = num("tag")? as u8;
            step(w, |w| session::mark(w, k))
        }),
        "film_next" => game.wm().and_then(|w| step(w, session::next)),
        "film_verdict" => game.wm().and_then(|w| {
            let k = num("answer")? as u8;
            let said = session::verdict(w, k)?;
            after_team(w);
            Ok(with_said(last_report(w), said))
        }),
        "film_leave" => game.wm().and_then(|w| {
            let saved = w.scouting.film.open.as_ref().is_some_and(|s| s.seen.iter().any(|v| !v.is_empty()));
            let said = session::close(w)?;
            if !saved {
                return Ok(with_said(json!({ "saved": false }), said));
            }
            after_team(w);
            let mut v = with_said(last_report(w), said);
            v["saved"] = json!(true);
            Ok(v)
        }),
        "film_report" => game.w().and_then(|w| {
            let k = num("k").map(|k| k as usize).unwrap_or(w.scouting.film.reports.len().saturating_sub(1));
            report(w, k)
        }),
        "film_work" => game.wm().and_then(|w| {
            let k = args.get("level").and_then(Value::as_str).ok_or("Missing level.")?;
            let said = time::set_work(w, k)?;
            Ok(with_said(home(w), said))
        }),
        "film_ethic" => game.wm().and_then(|w| {
            let v = args.get("value").and_then(Value::as_f64).ok_or("Missing value.")? as f32;
            let said = time::set_ethic(w, v)?;
            Ok(with_said(home(w), said))
        }),
        "film_queue_move" => game.wm().and_then(|w| {
            let up = args.get("up").and_then(Value::as_bool).unwrap_or(true);
            fm::queue::shift(w, num("k")? as usize, up)?;
            Ok(home(w))
        }),
        "film_queue_skip" => game.wm().and_then(|w| {
            let said = fm::queue::skip(w, num("k")? as usize)?;
            Ok(with_said(home(w), said))
        }),
        "there_stands" => game.wm().and_then(|w| {
            let said = there::stands(w, id()?)?;
            Ok(with_said(session_view(w), said))
        }),
        "there_visit" => game.wm().and_then(|w| {
            let said = there::visit_open(w, id()?)?;
            Ok(with_said(visit_view(w), said))
        }),
        "there_choose" => game.wm().and_then(|w| {
            let k = args.get("key").and_then(Value::as_str).ok_or("Missing key.")?;
            let said = there::visit_choose(w, k)?;
            Ok(with_said(visit_view(w), said))
        }),
        "there_leave" => game.wm().and_then(|w| {
            let said = there::visit_close(w)?;
            Ok(with_said(visit_view(w), said))
        }),
        "there_view" => game.w().map(visit_view),
        "camp" => game.w().map(camp_view),
        "camp_open" => game.wm().and_then(|w| {
            let said = there::camp_open(w)?;
            Ok(with_said(camp_view(w), said))
        }),
        "camp_coach" => game.wm().and_then(|w| {
            let said = there::camp_coach(w, num("group")? as u8)?;
            Ok(with_said(camp_view(w), said))
        }),
        "camp_offer" => game.wm().and_then(|w| {
            let said = there::camp_offer(w, id()?)?;
            Ok(with_said(camp_view(w), said))
        }),
        "camp_close" => game.wm().and_then(|w| {
            let said = there::camp_close(w)?;
            Ok(with_said(camp_view(w), said))
        }),
        "team_camp" => game.wm().and_then(|w| {
            let said = there::team_camp(w, num("program")? as usize)?;
            Ok(with_said(home(w), said))
        }),
        "signing_room" => game.w().map(signing_view),
        "combine_week" => game.w().map(combine_view),
        _ => return None,
    };
    Some(r)
}

fn with_said(mut v: Value, said: String) -> Value {
    v["said"] = json!(said);
    v
}

fn step(w: &mut World, f: impl FnOnce(&mut World) -> Result<(), String>) -> Reply {
    f(w)?;
    Ok(session_view(w))
}

fn name_of(w: &World, id: u32) -> String {
    sc::place(w, id).map_or_else(String::new, |i| w.people[i].name.clone())
}

fn dt(d: afm_engine::calendar::Date) -> String {
    d.pretty()
}

/// Your eye today in a word, and as a share (SK59).
fn eye_today(w: &World) -> (String, f64) {
    let t = time::tiredness(w);
    let word = match t {
        x if x < 0.05 => "Fresh",
        x if x < 0.25 => "Sharp",
        x if x < 0.5 => "Tiring",
        x if x < 0.8 => "Tired",
        _ => "Spent",
    };
    (word.to_string(), ((1.0 - t) * 100.0).round())
}

// ------------------------------------------------------------ the home

fn home(w: &World) -> Value {
    let staff = readers::seat_staff(w);
    let seat = match staff {
        Some(readers::Staff::Club(_)) => "nfl",
        Some(readers::Staff::Program(_)) => "college",
        Some(readers::Staff::School(_)) => "hs",
        None => "",
    };
    let (recruit, prep) = time::base(w);
    let budget = time::budget(w);
    let used = time::used(w);
    let (eye_word, eye_pct) = eye_today(w);
    let fr = &w.scouting.film;
    let ethic = time::ethic(w);
    let fresh = 3.0 + 0.1 * (ethic - 10.0);
    let wear = fr.wear;
    // What waits, in the queue's order (Up and Down move it), then the
    // newest of what's done; a man gone from the game drops off.
    let waiting = fr.queue.iter().enumerate().filter(|(_, q)| q.state == fm::queue::WAITING);
    let done = fr.queue.iter().enumerate().rev().filter(|(_, q)| q.state != fm::queue::WAITING);
    let queue: Vec<Value> = waiting
        .chain(done)
        .filter_map(|(k, q)| sc::place(w, q.id).map(|i| (k, q, &w.people[i])))
        .take(14)
        .map(|(k, q, p)| {
            json!({
                "k": k,
                "id": q.id,
                "name": p.name,
                "pos": p.player.position.name(),
                "meta": school_line(w, p),
                "from": q.from,
                "job": q.job,
                "why": q.why,
                "by": dt(q.by),
                "cut": q.cut.key(),
                "cut_words": q.cut.words(p.player.position),
                "state": q.state,
            })
        })
        .collect();
    let reports: Vec<Value> = fr
        .reports
        .iter()
        .enumerate()
        .rev()
        .take(16)
        .map(|(k, r)| {
            json!({
                "k": k,
                "name": r.name,
                "id": match r.target { fm::Target::Player(id) => Some(id), _ => None },
                "date": dt(r.date),
                "ask": r.ask.as_ref().map(|q| q.text.clone()),
                "verdict": r.verdict.map(|v| ["Yes", "No", "Not sure"][v.min(2) as usize]),
                "settled": r.settled,
                "calls": [r.calls.0, r.calls.1],
                "caught": r.caught.len(),
                "minutes": r.minutes,
                "mode": r.mode.key(),
                "stills": !r.snaps.is_empty(),
            })
        })
        .collect();
    // Your eye by position (SK63): the most-watched first.
    let mut by_pos: Vec<Value> = Position::ALL
        .iter()
        .filter_map(|&p| {
            let h = fr.pos_hours.get(p.index()).copied().unwrap_or(0.0);
            (h >= 0.05).then(|| {
                let word = match h {
                    x if x >= 200.0 => "sharp",
                    x if x >= 80.0 => "sharpening",
                    x if x >= 20.0 => "learning",
                    _ => "new",
                };
                json!({ "pos": p.name(), "hours": (h * 10.0).round() / 10.0, "pct": (h / 200.0 * 100.0).min(100.0).round(), "word": word })
            })
        })
        .collect();
    by_pos.sort_by(|a, b| b["hours"].as_f64().unwrap_or(0.0).total_cmp(&a["hours"].as_f64().unwrap_or(0.0)));
    let year = w.today.year();
    let year_hours = fr.years.iter().find(|x| x.0 == year).map_or(0.0, |x| x.1);
    let finds: Vec<Value> = fr
        .finds
        .iter()
        .rev()
        .take(8)
        .map(|f| json!({ "id": f.id, "name": name_of(w, f.id), "on": f.on, "on_name": name_of(w, f.on), "date": dt(f.date) }))
        .collect();
    let mirror = time::mirror(w);
    let staff_line = if mirror >= 0.3 {
        "Your staff mirror you: their film comes in a day sooner."
    } else if mirror <= -0.3 {
        "Your staff coast with you: their film comes in a day later."
    } else {
        "Your staff work at your pace."
    };
    let word = fr.word.map(|d| format!("\"First one in the building\": your boss heard it on {}.", dt(d)));
    let weeks: Vec<Value> = fr
        .weeks
        .iter()
        .rev()
        .take(10)
        .map(|x| json!({ "monday": dt(x.monday), "work": x.work.words(), "film": (x.film * 10.0).round() / 10.0, "budget": x.budget }))
        .collect();
    let trips: Vec<Value> = there::today(w)
        .into_iter()
        .map(|(id, kind, name, how)| {
            json!({ "id": id, "kind": kind.key(), "what": kind.words(), "name": name, "how": how, "watchers": there::watchers(w, id) })
        })
        .collect();
    // Your next opponent's kept tapes (SK54).
    let opp = opp_tapes(w);
    let open =
        fr.open.as_ref().map(|s| json!({ "name": s.name, "at": s.at, "n": s.snaps.len(), "mode": s.mode.key() }));
    let team_camp = (seat == "hs" && w.today.parts().1 == 6 && !fr.there.team_camps.iter().any(|x| x.0.year() == year))
        .then(|| team_camp_options(w));
    json!({
        "seat": seat,
        "date": dt(w.today),
        "budget": budget,
        "used": (used * 10.0).round() / 10.0,
        "left": (time::left(w) * 10.0).round() / 10.0,
        "recruit": recruit,
        "prep": prep,
        "today": (time::today(w) * 10.0).round() / 10.0,
        "eye_word": eye_word,
        "eye_pct": eye_pct,
        "rested": time::rested(w),
        "rest_until": fr.rest_until.map(dt),
        "work": fr.work.key(),
        "works": time::Work::ALL.iter().map(|x| json!({ "key": x.key(), "label": x.words(), "hours": x.hours() })).collect::<Vec<_>>(),
        "ethic": (ethic * 10.0).round() / 10.0,
        "ethic_set": fr.ethic.is_some(),
        "fresh": (fresh * 10.0).round() / 10.0,
        "wear": wear,
        "wear_word": match wear { x if x < 0.2 => "Fresh", x if x < 0.45 => "Carrying it", x if x < 0.7 => "Worn", _ => "Running on empty" },
        "home_word": match wear { x if x < 0.3 => "Fine", x if x < 0.6 => "Missing you", x if x < 0.85 => "Strained", _ => "At breaking point" },
        "queue": queue,
        "reports": reports,
        "by_pos": by_pos,
        "year_hours": (year_hours * 10.0).round() / 10.0,
        "finds": finds,
        "finds_n": fr.finds.iter().filter(|f| f.date.year() == year).count(),
        "staff_line": staff_line,
        "word": word,
        "weeks": weeks,
        "trips": trips,
        "opp": opp,
        "open": open,
        "visit_open": fr.there.visit.is_some(),
        "camp_day": there::camp_day(w),
        "signing_day": there::signing_today(w),
        "combine": there::combine_group(w).map(|g| g.0),
        "team_camp": team_camp,
    })
}

fn school_line(w: &World, p: &afm_engine::world::Person) -> String {
    let school = match p.stage {
        Stage::HighSchool | Stage::PrepSchool => w.high_schools.get(p.hs).map(|s| w.school_name(s.id).to_string()),
        Stage::College | Stage::Declared => p.college.map(|c| w.program_name(c).to_string()),
        Stage::Pro => p.pro.as_ref().and_then(|c| c.team).and_then(|t| w.nfl.get(t)).map(|t| t.name.to_string()),
        _ => None,
    };
    let stars = if p.stars > 0 && matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) {
        format!(" · {}-star", p.stars)
    } else {
        String::new()
    };
    format!("{} · {}{stars}", p.player.position.name(), school.unwrap_or_default())
}

fn team_camp_options(w: &World) -> Vec<Value> {
    let Some(h) = sc::hs::school(w) else { return Vec::new() };
    let state = w.high_schools[h].state;
    let mut v: Vec<(f64, usize)> = (0..w.programs.len())
        .filter(|&c| w.programs[c].state == state || afm_engine::world::geo::miles(state, w.programs[c].state) <= 300.0)
        .map(|c| (w.programs[c].prestige, c))
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    v.into_iter().take(10).map(|(_, c)| json!({ "program": c, "name": w.program_name(c) })).collect()
}

/// Your next opponent's kept tapes, and the ones you studied.
fn opp_tapes(w: &World) -> Vec<Value> {
    let Some(n) = gd::next_game(w) else { return Vec::new() };
    w.tapes_of(n.opp)
        .into_iter()
        .take(6)
        .map(|t| {
            let side = t.side_of(n.opp).unwrap_or(0);
            let studied = w.gameday.studied.iter().any(|s| s.0 == n.date && s.1 == t.date && [s.2, s.3] == t.teams);
            json!({
                "date": t.date.0, "date_text": dt(t.date), "home": t.teams[0], "away": t.teams[1],
                "label": format!("{} vs {}", t.names[side as usize], t.names[1 - side as usize]),
                "score": format!("{}-{}", t.score[side as usize], t.score[1 - side as usize]),
                "studied": studied,
            })
        })
        .collect()
}

// ------------------------------------------------------------ before the tape

/// The brief for a player (SK45, SK47): the questions on offer, the
/// cut-ups, who would sit in, what it costs.
fn brief(w: &World, id: u32, ask: Option<&str>) -> Reply {
    let i = sc::place(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    let ours = session::is_ours(w, id);
    let qs = session::questions(w, id);
    let mut cuts: Vec<Value> =
        tape::Cut::ALL.iter().map(|c| json!({ "key": c.key(), "label": c.words(p.player.position) })).collect();
    if ours {
        cuts.insert(0, json!({ "key": "ours", "label": tape::Cut::Ours.words(p.player.position) }));
    }
    let film = sc::own::film_cost(w, id).is_ok() || ours;
    Ok(json!({
        "id": id,
        "name": p.name,
        "pos": p.player.position.name(),
        "meta": school_line(w, p),
        "film": film,
        "why_not": sc::own::film_cost(w, id).err(),
        "questions": qs.iter().map(|q| json!({ "key": q.key, "text": q.text, "why": q.why })).collect::<Vec<_>>(),
        "cuts": cuts,
        "snap_min": session::SNAP_MIN,
        "quick_min": session::QUICK_MIN,
        "left": (time::left(w) * 10.0).round() / 10.0,
        "budget": time::budget(w),
        "watchers": there::watchers(w, id),
        "reads": reads_json(
            w,
            id,
            &qs.iter().find(|q| Some(q.key.as_str()) == ask).or(qs.first()).map(|q| q.attrs.clone()).unwrap_or_default(),
            &[]
        ),
    }))
}

/// Your staff's read beside yours on the question's attributes (the two
/// reads, SK23), yours with what this session has caught so far.
fn reads_json(w: &World, id: u32, attrs: &[u8], extra: &[fm::Catch]) -> Value {
    let Some(staff) = readers::seat_staff(w) else { return Value::Null };
    let sr = readers::read(w, staff, id);
    let own = yours::own_read_with(w, staff, id, sr.as_ref().map(|x| &x.2), extra);
    let rows: Vec<Value> = attrs
        .iter()
        .filter(|&&a| (a as usize) < ATTRIBUTE_COUNT)
        .map(|&a| {
            let at = Attr::ALL[a as usize];
            let s = sr.as_ref().and_then(|x| x.2.attrs.get(a as usize)).and_then(|r| r.range());
            let y = own.as_ref().and_then(|o| o.attrs.get(a as usize).copied().flatten()).map(yours::range);
            json!({ "name": at.label(), "staff": s.map(|r| [r.0, r.1]), "you": y.map(|r| [r.0, r.1]) })
        })
        .collect();
    json!({
        "rows": rows,
        "staff_now": sr.as_ref().map(|x| { let r = x.2.now; let r = yours::range(r); [r.0, r.1] }),
        "you_now": own.as_ref().map(|o| { let r = o.range(); [r.0, r.1] }),
    })
}

// ------------------------------------------------------------ the session

fn still_json(s: &tape::Still) -> Value {
    json!({
        "caption": s.caption,
        "los": s.los,
        "ring": s.ring.map(|r| [r.0, r.1]),
        "dots": s.dots.iter().map(|d| json!([d.x, d.y, d.k, d.l])).collect::<Vec<_>>(),
        "lines": s.lines.iter().map(|l| json!({ "k": l.k, "pts": l.pts.iter().map(|p| [p.0, p.1]).collect::<Vec<_>>() })).collect::<Vec<_>>(),
    })
}

/// The game-speed read (SK56): his top speed on the snaps against his
/// timed forty (or what a forty that speed would be).
fn speed_line(w: &World, s: &session::Session) -> Option<String> {
    let fm::Target::Player(id) = s.target else { return None };
    let i = sc::place(w, id)?;
    let watched: Vec<f32> =
        s.snaps.iter().zip(&s.seen).filter(|(_, v)| !v.is_empty()).map(|(x, _)| x.speed).filter(|&v| v > 0.0).collect();
    if watched.len() < 2 {
        return None;
    }
    let top = watched.iter().cloned().fold(0.0f32, f32::max) as f64;
    // A forty from top speed: about 40 / (0.86 * top) seconds.
    let plays = (40.0 / (0.86 * top)).clamp(4.2, 5.8);
    let p = &w.people[i];
    let timed = sc::own::measured(w, i, sc::own::Measure::Forty).map(|m| m.0);
    Some(match timed {
        Some(t) if plays < t - 0.06 => format!("Plays faster than his {t:.2}: about a {plays:.2} on the field."),
        Some(t) if plays > t + 0.06 => format!("Plays slower than his {t:.2}: about a {plays:.2} on the field."),
        Some(t) => format!("Plays to his {t:.2}."),
        None => format!(
            "Plays like a {plays:.2} forty on tape (he hasn't been timed){}.",
            if p.stage == Stage::Pro { "" } else { " " }.trim_end()
        ),
    })
}

fn session_view(w: &World) -> Value {
    let Some(s) = w.scouting.film.open.as_ref() else { return json!({ "open": false }) };
    let at = s.at.min(s.snaps.len().saturating_sub(1));
    let snap = &s.snaps[at];
    let (eye_word, eye_pct) = eye_today(w);
    // The stills as far as the tape has played: the snap; to the stop for a
    // call; all three once it has played.
    let shown = match s.step {
        0 => 1,
        1 => 2,
        _ => 3,
    };
    let stills: Vec<Value> = snap
        .stills
        .iter()
        .enumerate()
        .map(|(k, st)| if k < shown { still_json(st) } else { json!({ "veiled": true, "caption": "" }) })
        .collect();
    let seen = s.seen.get(at).cloned().unwrap_or_default();
    // What your eye caught on this snap, and what half-caught it.
    let mut caught = Vec::new();
    let mut half = None;
    let mut replayed = None;
    let mut slipped = 0;
    if s.step >= 2 {
        for (k, &v) in seen.iter().enumerate() {
            let cue = snap.cues.get(k);
            match v {
                session::CLEAR => {
                    if let Some(&(a, b)) = cue {
                        caught.push(fm::words::cue(a, b as usize).to_string());
                    } else if let Some(c) = &snap.call {
                        caught.push(c.truth.clone());
                    }
                }
                session::HALF => {
                    half = Some(cue.map_or("Something on that snap: run it back?".to_string(), |&(a, _)| {
                        fm::words::hint_of(a).to_string()
                    }));
                }
                session::REPLAYED => {
                    replayed = Some(cue.map_or_else(
                        || snap.call.as_ref().map_or(String::new(), |c| c.truth.clone()),
                        |&(a, b)| fm::words::cue(a, b as usize).to_string(),
                    ));
                }
                session::SLIPPED => slipped += 1,
                _ => {}
            }
        }
    }
    let moved = seen.contains(&session::MOVED_ON) || seen.contains(&session::REPLAY_MISSED);
    let pick = s.picks.get(at).copied().flatten();
    let call_result = match (&snap.call, pick, s.step >= 2) {
        (Some(c), Some(p), true) => Some(
            json!({ "right": p == c.right, "text": if p == c.right { format!("Right: {}", c.truth) } else { format!("Not this time: {}", c.truth) } }),
        ),
        _ => None,
    };
    let sheet: Vec<Value> = s
        .snaps
        .iter()
        .enumerate()
        .filter(|(k, _)| !s.seen[*k].is_empty())
        .map(|(k, x)| {
            json!({
                "no": k + 1,
                "game": s.games.get(x.game as usize).map(|g| g.label.clone()).unwrap_or_default(),
                "sit": x.sit, "job": x.job, "result": x.result,
                "you": s.marks[k].map(|m| session::TAGS[m as usize]),
                "his": s.his.get(k).map(|h| h.iter().filter(|&&b| b).count()),
            })
        })
        .collect();
    let pips: Vec<u8> = (0..s.snaps.len())
        .map(|k| {
            if k == at && s.step < 3 {
                2
            } else if !s.seen[k].is_empty() {
                1
            } else {
                0
            }
        })
        .collect();
    let (extra, _) = session::caught(w, s);
    let attrs = s.ask.as_ref().map(|q| q.attrs.clone()).unwrap_or_default();
    let reads = match s.target {
        fm::Target::Player(id) => reads_json(w, id, &attrs, &extra),
        _ => Value::Null,
    };
    let talk: Vec<Value> = s
        .talk
        .iter()
        .filter(|t| (t.0 as usize) <= at)
        .rev()
        .take(8)
        .map(|t| json!({ "snap": t.0 + 1, "who": t.1, "text": t.2 }))
        .collect();
    let hints: Vec<Value> = s
        .hints
        .iter()
        .map(|(key, text, ks)| {
            let found = ks.iter().any(|&k| s.seen.get(k as usize).is_some_and(|v| v.contains(&session::CLEAR) || v.contains(&session::REPLAYED)));
            json!({ "key": key, "text": if found { text.clone() } else { "Not caught yet".into() }, "found": found, "snaps": ks.len() })
        })
        .collect();
    json!({
        "open": true,
        "target": match s.target { fm::Target::Player(id) => json!({ "player": id }), fm::Target::Team(t) => json!({ "team": t }) },
        "name": s.name,
        "pos": s.pos.name(),
        "cut": if matches!(s.target, fm::Target::Team(_)) { "Their tape".to_string() } else { s.cut.words(s.pos).to_string() },
        "mode": s.mode.key(),
        "ask": s.ask.as_ref().map(|q| json!({ "text": q.text, "why": q.why })),
        "games": s.games.iter().map(|g| g.label.clone()).collect::<Vec<_>>(),
        "at": at,
        "n": s.snaps.len(),
        "cut_n": s.cut_n,
        "step": s.step,
        "pips": pips,
        "snap": {
            "game": s.games.get(snap.game as usize).map(|g| g.label.clone()).unwrap_or_default(),
            "sit": snap.sit,
            "job": if s.step >= 1 { snap.job.clone() } else { String::new() },
            "result": if s.step >= 2 { snap.result.clone() } else { String::new() },
            "chart": if s.step >= 2 { snap.chart.clone() } else { String::new() },
            "stills": stills,
            "call": if s.step == 1 { snap.call.as_ref().map(|c| json!({ "q": c.q, "opts": c.opts })) } else { None },
            "call_result": call_result,
            "after": if s.step >= 2 { snap.after.clone() } else { None },
            "surprise": if s.step >= 2 { snap.surprise.clone() } else { None },
            "caught": caught,
            "half": half,
            "replayed": replayed,
            "moved": moved,
            "slipped": slipped,
            "tired": s.tired.get(at).copied().unwrap_or(false),
            "mark": s.marks.get(at).copied().flatten(),
        },
        "tags": session::TAGS,
        "sheet": sheet,
        "talk": talk,
        "coach": s.coach.as_ref().map(|r| json!({ "name": r.name, "job": r.job })),
        "analyst": s.analyst.as_ref().map(|a| a.0.clone()),
        "numbers": s.numbers,
        "minutes": s.minutes,
        "left": (time::left(w) * 10.0).round() / 10.0,
        "budget": time::budget(w),
        "today": (time::today(w) * 10.0).round() / 10.0,
        "eye_word": eye_word,
        "eye_pct": eye_pct,
        "reads": reads,
        "speed": speed_line(w, s),
        "hints": hints,
        "snap_min": session::SNAP_MIN,
        "runback_min": session::RUNBACK_MIN,
    })
}

/// The report just saved, as the verdict page shows it.
fn last_report(w: &World) -> Value {
    let k = w.scouting.film.reports.len().saturating_sub(1);
    report(w, k).unwrap_or(json!({ "open": false }))
}

/// A report (SK58), with the snaps it rests on for the look-back.
fn report(w: &World, k: usize) -> Reply {
    let r = w.scouting.film.reports.get(k).ok_or("No such report.")?;
    let tags_on: Vec<&str> = r.marks.iter().filter(|m| m.1 >= 2).map(|m| session::TAGS[m.1 as usize]).collect();
    Ok(json!({
        "open": false,
        "report": {
            "k": k,
            "name": r.name,
            "id": match r.target { fm::Target::Player(id) => Some(id), _ => None },
            "team": matches!(r.target, fm::Target::Team(_)),
            "date": dt(r.date),
            "cut": r.cut.key(),
            "mode": r.mode.key(),
            "ask": r.ask.as_ref().map(|q| q.text.clone()),
            "verdict": r.verdict.map(|v| ["Yes", "No", "Not sure"][v.min(2) as usize]),
            "settled": r.settled,
            "calls": [r.calls.0, r.calls.1],
            "caught": r.caught,
            "marks": r.marks.iter().map(|m| json!({ "snap": m.0 + 1, "tag": session::TAGS[m.1 as usize], "line": m.2 })).collect::<Vec<_>>(),
            "carried": tags_on,
            "minutes": r.minutes,
            "coach": r.coach,
            "coach_answer": r.coach_answer.map(|v| ["Yes", "No", "Not sure"][v.min(2) as usize]),
            "analyst": r.analyst,
            "numbers": r.numbers,
            "hints": r.hints,
            "watched": r.watched,
            "snaps": r.snaps.iter().map(|x| json!({ "sit": x.sit, "job": x.job, "result": x.result, "stills": x.stills.iter().map(still_json).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        }
    }))
}

// ------------------------------------------------------------ your next opponent (SK54)

fn open_opp(w: &mut World, a: &Value) -> Reply {
    let n = |k: &str| a.get(k).and_then(Value::as_u64).ok_or(format!("Missing {k}."));
    let (d, h, aw) = (afm_engine::calendar::Date(n("date")? as i32), n("home")? as usize, n("away")? as usize);
    let next = gd::next_game(w).ok_or("No game left on your schedule.")?;
    let (level, me) = w.seat_team().ok_or("You have no team.")?;
    let tp = w
        .gameday
        .tapes
        .iter()
        .find(|t| t.date == d && t.teams == [h, aw])
        .ok_or("That tape is no longer in your film room.")?
        .clone();
    let side = tp.side_of(next.opp).ok_or("That tape isn't of your next opponent.")?;
    let mut names = gd::names(w, &[gd::team_ref(level, me), gd::team_ref(level, next.opp)]);
    gd::add_tape_names(w, &mut names, &[&tp]);
    let hints: Vec<(String, String, Vec<usize>)> =
        gdfilm::hints(&tp, side, &names).into_iter().map(|x| (x.key, x.text, x.snaps)).collect();
    let salt = w.options.seed as u64 ^ (d.0 as u64) << 20;
    let (games, snaps, hints) = fm::opp::team_cut(&tp, side, &hints, salt);
    let mode = match a.get("mode").and_then(Value::as_str) {
        Some("quick") => session::Mode::Quick,
        _ => session::Mode::Full,
    };
    let name = tp.names[side as usize].clone();
    let said = session::open_team(w, next.opp as u16, name, games, snaps, hints, (d, h, aw), mode)?;
    after_team(w);
    Ok(with_said(session_view(w), said))
}

/// A team's session ended: the tape counts as studied for the game plan
/// and what you caught goes into the plan as its hints (SK54).
/// Section 9's study of a tape: three hours (`lane_gameday::film_budget`).
const STUDY_MIN: f64 = 180.0;

fn after_team(w: &mut World) {
    if w.scouting.film.open.is_some() {
        return;
    }
    let Some(r) = w.scouting.film.reports.last() else { return };
    if !matches!(r.target, fm::Target::Team(_)) || r.date != w.today {
        return;
    }
    let hints = r.hints.clone();
    let minutes = r.minutes;
    let Some(key) = r.tape_key else { return };
    let Some(next) = gd::next_game(w) else { return };
    // What you caught goes in the plan; the whole tape counts as studied
    // (every habit on the hub) only after a full study's hours.
    if minutes as f64 >= STUDY_MIN
        && !w.gameday.studied.iter().any(|s| s.0 == next.date && s.1 == key.0 && [s.2, s.3] == [key.1, key.2])
    {
        w.gameday.studied.push((next.date, key.0, key.1, key.2));
    }
    for h in hints {
        let _ = gd::hint(w, &json!({ "date": key.0.0, "home": key.1, "away": key.2, "key": h, "add": true }));
    }
}

// ------------------------------------------------------------ being there

fn visit_view(w: &World) -> Value {
    let v = w.scouting.film.there.visit.as_ref().or(w.scouting.film.there.visits.last());
    let Some(v) = v else { return json!({ "visit": null }) };
    let open = w.scouting.film.there.visit.is_some();
    let opts: Vec<Value> = there::choices(v.kind)
        .iter()
        .map(|c| {
            json!({ "key": c.key, "label": c.label, "place": c.place, "minutes": c.minutes,
                "done": v.done.iter().any(|d| d.0 == c.key), "fits": c.minutes <= v.minutes_left })
        })
        .collect();
    json!({
        "visit": {
            "open": open,
            "id": v.id,
            "name": v.name,
            "kind": v.kind.key(),
            "what": v.kind.words(),
            "date": dt(v.date),
            "left": v.minutes_left,
            "place": v.place,
            "done": v.done.iter().map(|d| json!({ "key": d.0, "place": d.1, "line": d.2 })).collect::<Vec<_>>(),
            "watchers": v.watchers,
            "opts": opts,
        }
    })
}

fn camp_view(w: &World) -> Value {
    let c = w.scouting.film.there.camp.as_ref();
    let person = |id: u32| {
        sc::place(w, id).map(|i| {
            let p = &w.people[i];
            json!({ "id": id, "name": p.name, "pos": p.player.position.name(), "meta": school_line(w, p),
                "offered": readers::seat_staff(w).is_some_and(|s| matches!(s, readers::Staff::Program(c) if p.cal.offers.contains(&c))) })
        })
    };
    json!({
        "camp_day": there::camp_day(w),
        "open": c.is_some(),
        "campers": c.map(|c| c.campers.iter().filter_map(|&id| person(id)).collect::<Vec<_>>()).unwrap_or_default(),
        "group": c.and_then(|c| c.group),
        "groups": afm_engine::world::coach::Group::ALL.iter().map(|g| g.name()).collect::<Vec<_>>(),
        "standouts": c.map(|c| c.standouts.iter().map(|s| json!({ "id": s.0, "name": name_of(w, s.0), "line": s.1 })).collect::<Vec<_>>()).unwrap_or_default(),
        "one_on_ones": c.map(|c| c.one_on_ones.iter().map(|o| json!({ "win": o.0, "lose": o.1, "line": o.2 })).collect::<Vec<_>>()).unwrap_or_default(),
        "offered": c.map(|c| c.offered.clone()).unwrap_or_default(),
    })
}

fn signing_view(w: &World) -> Value {
    let rows: Vec<Value> = there::signings(w)
        .into_iter()
        .enumerate()
        .map(|(k, (id, name, prog, ours, flip, line))| {
            // The papers come in through the day, ceremonies first.
            let minute = 7 * 60 + 30 + (k as u32 * 37) % (10 * 60);
            json!({ "id": id, "name": name, "school": w.program_name(prog), "ours": ours, "flip": flip, "line": line,
                "time": format!("{}:{:02} {}", if minute / 60 > 12 { minute / 60 - 12 } else { minute / 60 }, minute % 60, if minute / 60 >= 12 { "pm" } else { "am" }) })
        })
        .collect();
    json!({ "today": there::signing_today(w), "rows": rows, "seat_hs": sc::hs::school(w).is_some() })
}

fn combine_view(w: &World) -> Value {
    let Some((day, group)) = there::combine_group(w) else { return json!({ "on": false }) };
    let year = w.year;
    let ranks = sc::board::public_ranks(w);
    let order = &w.scouting.board.order;
    let mut rows: Vec<(usize, Value)> = w
        .people
        .iter()
        .filter(|p| p.stage == Stage::Declared && group.contains(&p.player.position))
        .filter_map(|p| {
            let id = p.player.id;
            let r = *ranks.get(&id)?;
            if r >= 330 {
                return None;
            }
            let t = sc::file::tests(&p.player, sc::Kind::Combine, year);
            Some((
                r,
                json!({ "id": id, "name": p.name, "pos": p.player.position.name(), "public": r + 1,
                    "ours": order.iter().position(|&x| x == id).map(|x| x + 1),
                    "forty": (t.forty * 100.0).round() / 100.0, "vertical": (t.vertical * 2.0).round() / 2.0,
                    "broad": t.broad.round(), "bench": t.bench.round(), "cone": (t.cone * 100.0).round() / 100.0,
                    "shuttle": (t.shuttle * 100.0).round() / 100.0 }),
            ))
        })
        .collect();
    rows.sort_by_key(|x| x.0);
    json!({
        "on": true,
        "day": day,
        "group": group.iter().map(|p| p.name()).collect::<Vec<_>>(),
        "rows": rows.into_iter().take(60).map(|x| x.1).collect::<Vec<_>>(),
    })
}
