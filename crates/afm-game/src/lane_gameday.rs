//! Lane commands: gameday (docs/final-build/plan.md). `call` answers the
//! commands this lane owns and returns None for any other.
//!
//! Section 9, game day control (`docs/design/game-day.md`): the game day
//! screen (your staff's opponent report, the tapes you study and their
//! hints, your plan and game rules, which snaps stop for you), your game
//! played live snap by snap, and the game page of every game you kept.

pub(crate) mod film;
pub(crate) mod page;

use std::collections::HashMap;

use afm_engine::calendar::{Date, Level as CalLevel};
use afm_engine::game::calling::FourthDown;
use afm_engine::game::live::{
    self, Ask, CallClock, FourthChart, Input, Pending, PlanChange, Settings, StopMode, Tempo, TossChoice, TwoChart,
    View,
};
use afm_engine::game::playbook::{
    Backfield, Coverage, DefCall, DefPersonnel, Formation, FrontLook, Motion, OffCall, PASS_CONCEPTS, Personnel,
    PlayKind, Pressure, RUN_CONCEPTS, Set, Stunt, call_family,
};
use afm_engine::game::stats::{PlayRecord, PlayType};
use afm_engine::game::{GameResult, GameTeam};
use afm_engine::world::World;
use serde_json::{Value, json};

use crate::lane_boards::flow::Wp;
use crate::views::{self, TeamRef};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let r = match cmd {
        "gameday" => game.w().map(hub),
        "gameday_now" => game.w().map(now),
        "gameday_settings" => game.wm().and_then(|w| set_settings(w, args)),
        "gameday_rules" => game.wm().and_then(|w| set_rules(w, args)),
        "gameday_plan" => game.wm().and_then(|w| set_plan(w, args)),
        "gameday_study" => game.wm().and_then(|w| study(w, args)),
        "gameday_hint" => game.wm().and_then(|w| hint(w, args)),
        "gameday_kickoff" => game.wm().and_then(|w| {
            w.start_live()?;
            live_state(w)
        }),
        "live" => game.wm().and_then(live_state),
        "live_answer" => game.wm().and_then(|w| answer(w, args)),
        "live_take" => game.wm().and_then(|w| {
            let l = w.live.as_mut().ok_or("No game is being played.")?;
            l.game.ctl.take_next = true;
            live_state(w)
        }),
        "live_staff" => game.wm().and_then(|w| {
            let l = w.live.as_mut().ok_or("No game is being played.")?;
            l.game.ctl.settings.stop = StopMode::Off;
            l.game.ctl.take_next = false;
            live_state(w)
        }),
        "live_frames" => game.wm().and_then(|w| frames(w, args)),
        "game_page" => game.w().and_then(|w| {
            let k = args.get("index").and_then(Value::as_u64).ok_or("Missing index.")? as usize;
            page::game_page(w, k)
        }),
        _ => return None,
    };
    Some(r)
}

// ------------------------------------------------------------ helpers

fn team_ref(level: CalLevel, t: usize) -> TeamRef {
    match level {
        CalLevel::Nfl => TeamRef::Nfl(t),
        CalLevel::HighSchool => TeamRef::HighSchool(t),
        CalLevel::Other => TeamRef::Other(t),
        _ => TeamRef::College(t),
    }
}

/// Every player on both teams' books: id to (name, position).
pub(crate) fn names(w: &World, teams: &[TeamRef]) -> film::Names {
    let mut m = HashMap::new();
    for &t in teams {
        for k in views::roster_people(w, t) {
            let p = &w.people[k];
            m.insert(p.player.id, (p.name.clone(), p.player.position.name().to_string()));
        }
    }
    m
}

fn clock_text(secs: f64) -> String {
    let s = secs.max(0.0).round() as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

/// "OWN 25", "50", "OPP 30" from yards to the goal line.
fn spot(to_goal: f64) -> String {
    let y = to_goal.round() as i32;
    if y == 50 {
        "50".into()
    } else if y > 50 {
        format!("OWN {}", 100 - y)
    } else {
        format!("OPP {y}")
    }
}

fn down_text(down: u8, togo: f64, to_goal: f64) -> String {
    let d = ["1st", "2nd", "3rd", "4th"].get(down.clamp(1, 4) as usize - 1).copied().unwrap_or("1st");
    let t = if togo >= to_goal - 0.5 {
        "goal".to_string()
    } else if togo < 0.75 {
        "inches".to_string()
    } else {
        format!("{}", togo.round().max(1.0))
    };
    format!("{d} & {t}")
}

fn quarter_text(q: u8, ot: bool) -> String {
    if ot || q > 4 { "OT".into() } else { format!("Q{q}") }
}

/// Your next game: date, opponent, home or away, neutral site, the
/// game's name and why it matters.
pub(crate) struct Next {
    pub(crate) date: Date,
    pub(crate) opp: usize,
    pub(crate) home: bool,
    pub(crate) neutral: bool,
    pub(crate) name: Option<String>,
    pub(crate) why: Vec<String>,
    pub(crate) kick: Option<String>,
}

pub(crate) fn next_game(w: &World) -> Option<Next> {
    let (level, t) = w.seat_team()?;
    // Today's game as the day builds it (playoff rounds included).
    if let Some(s) = w.live.as_ref().filter(|l| l.date == w.today) {
        let home = s.teams[0] == t;
        return Some(Next {
            date: s.date,
            opp: s.teams[if home { 1 } else { 0 }],
            home,
            neutral: s.game.params.neutral,
            name: None,
            why: Vec::new(),
            kick: None,
        });
    }
    let today = w.today;
    match level {
        CalLevel::College => w
            .college_fixtures(Some(t))
            .into_iter()
            .filter(|f| f.date >= today && f.score.is_none() && !f.cancelled)
            .min_by_key(|f| f.date)
            .map(|f| Next {
                date: f.date,
                opp: if f.home == t { f.away } else { f.home },
                home: f.home == t,
                neutral: f.neutral,
                name: f.name.map(str::to_string),
                why: f.why.iter().map(|s| s.to_string()).collect(),
                kick: Some(kick_text(f.kickoff.0 as u32 * 60 + f.kickoff.1 as u32)),
            }),
        CalLevel::Nfl => {
            w.nfl_fixtures(Some(t)).into_iter().find(|f| !f.played && f.when.is_some_and(|d| d >= today)).map(|f| {
                Next {
                    date: f.when.unwrap_or(today),
                    opp: if f.home == t { f.away } else { f.home },
                    home: f.home == t,
                    neutral: false,
                    name: Some(format!("Week {}", f.week)),
                    why: Vec::new(),
                    kick: f.minute.map(|m| kick_text(m as u32)),
                }
            })
        }
        CalLevel::HighSchool => w.hs_fixtures(t).into_iter().find(|f| f.date >= today).map(|f| Next {
            date: f.date,
            opp: if f.home == t { f.away } else { f.home },
            home: f.home == t,
            neutral: false,
            name: f.playoff.then(|| "Playoffs".to_string()),
            why: Vec::new(),
            kick: Some("7:00 pm".into()),
        }),
        CalLevel::Other => {
            afm_engine::world::pro::other::seat_fixtures(w).into_iter().find(|f| f.0 >= today).map(|(date, h, a)| {
                Next {
                    date,
                    opp: if h == t { a } else { h },
                    home: h == t,
                    neutral: false,
                    name: None,
                    why: Vec::new(),
                    kick: None,
                }
            })
        }
        _ => None,
    }
    .or_else(|| {
        // A playoff game today that no schedule lists yet.
        let s = w.seat_setup_today()?;
        let home = s.teams[0] == t;
        Some(Next {
            date: today,
            opp: s.teams[if home { 1 } else { 0 }],
            home,
            neutral: s.params.neutral,
            name: Some("Playoffs".into()),
            why: Vec::new(),
            kick: None,
        })
    })
}

/// Minutes after midnight Eastern to "8:00 pm".
fn kick_text(m: u32) -> String {
    let (h, mi) = ((m / 60) % 24, m % 60);
    let (h12, ap) = match h {
        0 => (12, "am"),
        1..=11 => (h, "am"),
        12 => (12, "pm"),
        _ => (h - 12, "pm"),
    };
    format!("{h12}:{mi:02} {ap}")
}

/// Film time for a week (topic 1, people topic 5's time budget): an NFL
/// head coach about 12 hours of opponent film, a college one the same, a
/// high school coach who also teaches about 6; a tape takes 3. A short
/// week has less (proposed, not confirmed).
fn film_budget(w: &World, game: Date) -> (f64, f64) {
    let base = match w.seat_team().map(|x| x.0) {
        Some(CalLevel::HighSchool) => 6.0,
        _ => 12.0,
    };
    let last = w.seat_games().iter().map(|g| g.date).filter(|d| *d < game).max();
    let days = last.map_or(7, |d| d.days_to(game));
    let k = (days as f64 / 7.0).clamp(0.5, 1.4);
    ((base * k * 2.0).round() / 2.0, 3.0)
}

// ------------------------------------------------------------ the game day screen

/// What Continue needs to know: is your game today, is it under way, and
/// do any snaps stop for you.
fn now(w: &World) -> Value {
    let live = w.live.as_ref().filter(|l| l.date == w.today).map(|l| {
        let mut g = l.game.clone();
        if g.run().pending.is_some() { "live" } else { "final" }
    });
    let today = live.is_some() || w.seat_setup_today().is_some();
    let stop = match w.gameday.settings.stop {
        StopMode::Situations => "situations",
        StopMode::EverySnap => "every",
        StopMode::Off => "off",
    };
    json!({ "today": today, "live": live.unwrap_or("none"), "stop": stop })
}

fn hub(w: &World) -> Value {
    let Some((level, t)) = w.seat_team() else {
        return json!({ "has_game": false, "why": "You have no team." });
    };
    let me = team_ref(level, t);
    let settings = settings_json(&w.gameday.settings);
    let rules = rules_json(w);
    let mut calling = calling_record(w);
    let Some(n) = next_game(w) else {
        calling["season_over"] = json!(true);
        // Between seasons: say when the next games come, not just that
        // none are left.
        let why = if matches!(level, CalLevel::Nfl) && w.today < w.dates.schedule_release {
            format!(
                "Your season is over. The league releases the {} schedule on {}; your next game shows here from then.",
                w.dates.season,
                views::date_text(w.dates.schedule_release)
            )
        } else {
            "Your season is over. Your next game shows here once next season's schedule is set.".to_string()
        };
        return json!({ "has_game": false, "why": why, "me": views::team_brief(w, me), "settings": settings, "rules": rules, "calling": calling });
    };
    let opp = team_ref(level, n.opp);
    let names = names(w, &[me, opp]);
    let today = n.date == w.today;
    // The day's game as it will be played (game morning only).
    let setup = if today { w.seat_setup_today() } else { None };
    let tapes = w.tapes_of(n.opp);
    let side = |tp: &afm_engine::world::tape::Tape| tp.side_of(n.opp).unwrap_or(0);
    let studied = |tp: &afm_engine::world::tape::Tape| {
        w.gameday.studied.iter().any(|s| s.0 == n.date && s.1 == tp.date && [s.2, s.3] == tp.teams)
    };
    let (budget, per) = film_budget(w, n.date);
    let used = tapes.iter().filter(|tp| studied(tp)).count() as f64 * per;
    let plan = &w.gameday.plan;
    let plan_now = plan.date == Some(n.date);
    let added = |tp: &afm_engine::world::tape::Tape, key: &str| {
        plan_now && plan.hints.iter().any(|h| h.0 == tp.date && [h.1, h.2] == tp.teams && h.3 == key)
    };
    // Each tape, and your finds on the tapes you studied (one hint a key:
    // the same habit on several tapes is one stronger hint).
    let mut finds: Vec<(film::Hint, Vec<Value>, bool)> = Vec::new();
    let tape_rows: Vec<Value> = tapes
        .iter()
        .map(|tp| {
            let s = side(tp);
            let other = tp.teams[1 - s as usize];
            let won = tp.score[s as usize] > tp.score[1 - s as usize];
            let st = studied(tp);
            let hints: Vec<Value> = if st {
                film::hints(tp, s, &names)
                    .into_iter()
                    .map(|h| {
                        let on = added(tp, &h.key);
                        let src = json!({ "date": tp.date.0, "home": tp.teams[0], "away": tp.teams[1] });
                        let v = hint_json(&h, &names, on, &src);
                        match finds.iter_mut().find(|f| f.0.key == h.key) {
                            Some(f) => {
                                f.0.hit += h.hit;
                                f.0.of += h.of;
                                f.1.push(src);
                                f.2 |= on;
                            }
                            None => finds.push((h, vec![src], on)),
                        }
                        v
                    })
                    .collect()
            } else {
                Vec::new()
            };
            json!({
                "date": views::date_text(tp.date),
                "key": { "date": tp.date.0, "home": tp.teams[0], "away": tp.teams[1] },
                "vs": tp.names[1 - s as usize],
                "vs_ref": views::team_brief(w, team_ref(level, other)),
                "home": s == 0,
                "score": format!("{} {}-{}", if won { "W" } else if tp.score[0] == tp.score[1] { "T" } else { "L" }, tp.score[s as usize], tp.score[1 - s as usize]),
                "snaps": tp.snaps.iter().filter(|x| !x.null && x.kind <= 1).count(),
                "studied": st,
                "hints": hints,
            })
        })
        .collect();
    let finds: Vec<Value> = finds
        .iter()
        .map(|(h, src, on)| {
            let mut v = hint_json(h, &names, *on, &src[0]);
            v["tapes"] = json!(src.len());
            v["sources"] = json!(src);
            if src.len() > 1 {
                v["text"] = json!(format!("{} Seen on {} tapes ({} of {} in all).", h.text, src.len(), h.hit, h.of));
            }
            v
        })
        .collect();
    // Who is out: your staff's list and theirs as public. Injury reports
    // come out in game week; months ahead there's nothing to report.
    let game_week = w.today.days_to(n.date) <= 7;
    let out_of = |tr: TeamRef| -> Vec<Value> {
        if !game_week {
            return Vec::new();
        }
        let r = views::roster(w, tr);
        r["rows"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .filter(|x| x["status"].as_str().is_some_and(|s| !s.is_empty()))
                    .take(14)
                    .map(|x| json!({ "id": x["id"], "text": format!("{} ({}): {}", x["name"].as_str().unwrap_or(""), x["pos"].as_str().unwrap_or(""), x["status"].as_str().unwrap_or("")) }))
                    .collect()
            })
            .unwrap_or_default()
    };
    let report = film::report(
        &tapes,
        n.opp,
        w.seat_film_study(),
        w.options.seed ^ (n.date.0 as u32).rotate_left(9),
        &names,
        out_of(opp),
    );
    // The plan's choices: their corners, receivers and quarterback (as
    // they dress today, else from their roster), and their returner.
    let options = plan_options(
        w,
        opp,
        setup.as_ref().map(|s| if s.teams[0] == n.opp { &s.home } else { &s.away }),
        &names,
        &tapes,
    );
    let pick = |x: Option<Option<u32>>| match x {
        None => json!("staff"),
        Some(None) => json!("none"),
        Some(Some(id)) => json!(id),
    };
    let weather = setup.as_ref().map(|s| weather_json(&s.params.cond, s.params.rules.level, n.home || n.neutral));
    let live = w.live.as_ref().filter(|l| l.date == w.today).map(|l| {
        let mut g = l.game.clone();
        let step = g.run();
        json!({ "over": step.pending.is_none(), "score": step.view.score, "quarter": quarter_text(step.view.quarter, step.view.overtime), "clock": clock_text(step.view.clock) })
    });
    // Before a team's first game its record is last season's: say so.
    let records = [me, opp].map(|t| {
        let (text, when) = views::season_record(w, t);
        if when == "this season" { text } else { format!("{text} {when}") }
    });
    json!({
        "has_game": true,
        "today": today,
        "date": views::date_text(n.date),
        "in": w.today.days_to(n.date),
        "kick": n.kick,
        "level": match level {
            CalLevel::Nfl => "NFL",
            CalLevel::HighSchool => "High school",
            CalLevel::Other => afm_engine::world::pro::other::league_short(afm_engine::world::pro::other::league_of(t)),
            _ => "College",
        },
        "name": n.name,
        "why": n.why,
        "me": views::team_brief(w, me),
        "opp": views::team_brief(w, opp),
        "home": n.home,
        "neutral": n.neutral,
        "records": records,
        "weather": weather,
        "live": live,
        "can_kick_off": today && setup.is_some(),
        "settings": settings,
        "rules": rules,
        "plan": {
            "now": plan_now,
            "target_cb": if plan_now { pick(plan.target_cb) } else { json!("staff") },
            "bracket": if plan_now { pick(plan.bracket) } else { json!("staff") },
            "spy": if plan_now { json!(plan.spy) } else { Value::Null },
            "kick_away": if plan_now { json!(plan.kick_away) } else { Value::Null },
            "draws": if plan_now { json!(plan.draws) } else { Value::Null },
            "options": options,
            "hints": if plan_now { plan.hints.len() } else { 0 },
        },
        "report": report,
        "tapes": tape_rows,
        "finds": finds,
        "film": { "used": used, "budget": budget, "per_tape": per, "staff_eye": report["staff"] },
        "out_mine": out_of(me),
        "prep_mine": crate::lane_training::prep_report(w),
        "calling": calling,
    })
}

fn hint_json(h: &film::Hint, n: &film::Names, added: bool, src: &Value) -> Value {
    json!({
        "key": h.key,
        "kind": h.kind,
        "unit": h.unit,
        "text": h.text,
        "tip": h.tip,
        "hit": h.hit,
        "of": h.of,
        "snaps": h.snaps,
        "effect": h.effect.words(n),
        "plan": h.effect != film::Effect::Note,
        "added": added,
        "src": src,
    })
}

fn settings_json(s: &Settings) -> Value {
    json!({
        "stop": match s.stop { StopMode::Situations => "situations", StopMode::EverySnap => "every", StopMode::Off => "off" },
        "situations": {
            "third_down": s.situations.third_down,
            "fourth_down": s.situations.fourth_down,
            "red_zone": s.situations.red_zone,
            "goal_line": s.situations.goal_line,
            "two_minute": s.situations.two_minute,
            "decisions": s.situations.decisions,
        },
        "clock": match s.clock { CallClock::Realism => "realism", CallClock::Learning => "learning" },
    })
}

fn rules_json(w: &World) -> Value {
    let r = &w.gameday.rules;
    json!({
        "fourth": match r.fourth { FourthChart::Staff => "staff", FourthChart::Numbers => "numbers", FourthChart::Aggressive => "aggressive", FourthChart::Conservative => "conservative" },
        "two": match r.two { TwoChart::Staff => "staff", TwoChart::Chart => "chart", TwoChart::Kick => "kick" },
        "tempo": tempo_name(r.tempo),
        "level": w.seat_team().map(|x| x.0 == CalLevel::HighSchool),
    })
}

fn tempo_name(t: Tempo) -> &'static str {
    match t {
        Tempo::Normal => "normal",
        Tempo::Hurry => "hurry",
        Tempo::Milk => "milk",
    }
}

fn tempo_of(s: &str) -> Tempo {
    match s {
        "hurry" => Tempo::Hurry,
        "milk" => Tempo::Milk,
        _ => Tempo::Normal,
    }
}

fn weather_json(c: &afm_engine::game::Conditions, level: afm_engine::player::Level, home_end: bool) -> Value {
    let indoor = c.temperature >= 69.9 && c.temperature <= 72.1 && c.wind < 0.5 && !c.rain && !c.snow;
    let sky = if c.snow {
        "snow"
    } else if c.rain {
        "rain"
    } else if indoor {
        "indoors"
    } else {
        "dry"
    };
    // The kicking line from warmups (topic 3): about 50 yards for a good
    // NFL kicker in still air, each mph of wind along the field a yard
    // (proposed, not confirmed; the engine's own make chance decides).
    let base: f64 = match level {
        afm_engine::player::Level::NFL => 53.0,
        afm_engine::player::Level::HS => 38.0,
        _ => 47.0,
    };
    let along = c.wind_along;
    let cold = if c.temperature < 40.0 { (40.0 - c.temperature) / 10.0 } else { 0.0 };
    let alt = c.altitude / 1000.0;
    let with = (base + along.abs() * 0.6 - cold + alt).round();
    let against = (base - along.abs() * 0.6 - cold + alt).round();
    json!({
        "text": if indoor { "Indoors: no wind, 72°F".to_string() } else { format!("{:.0}°F, wind {:.0} mph, {sky}", c.temperature, c.wind) },
        "temperature": c.temperature.round(),
        "wind": c.wind.round(),
        "sky": sky,
        "crowd": (c.crowd * 100.0).round(),
        "kicking": if indoor || along.abs() < 3.0 {
            json!([format!("Field-goal line about {base:.0} yards toward either end")])
        } else {
            let (toward, away) = if (along > 0.0) == home_end { ("the open end", "the closed end") } else { ("the closed end", "the open end") };
            json!([format!("{with:.0} yards with the wind (toward {toward})"), format!("{against:.0} yards into it (toward {away})")])
        },
    })
}

/// The matchup plan's choices.
fn plan_options(
    w: &World,
    opp: TeamRef,
    dressed: Option<&GameTeam>,
    names: &film::Names,
    tapes: &[&afm_engine::world::tape::Tape],
) -> Value {
    use afm_engine::positions::Position;
    // Who showed up on their tapes: starters play the snaps.
    let mut seen: HashMap<u32, u32> = HashMap::new();
    for t in tapes {
        for s in &t.snaps {
            for id in [s.passer, s.target, s.carrier, s.defender, s.returner].into_iter().flatten() {
                *seen.entry(id).or_default() += 1;
            }
        }
    }
    let pool: Vec<(u32, Position)> = match dressed {
        Some(t) => t.players.iter().map(|p| (p.id, p.position)).collect(),
        None => {
            views::roster_people(w, opp).iter().map(|&k| (w.people[k].player.id, w.people[k].player.position)).collect()
        }
    };
    let list = |ps: &[Position], n: usize| -> Vec<Value> {
        let mut v: Vec<(u32, u32)> =
            pool.iter().filter(|x| ps.contains(&x.1)).map(|x| (x.0, seen.get(&x.0).copied().unwrap_or(0))).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.into_iter()
            .take(n)
            .map(|(id, k)| {
                let (nm, p) = names.get(&id).cloned().unwrap_or((format!("No. {id}"), String::new()));
                json!({ "id": id, "name": nm, "pos": p, "seen": k })
            })
            .collect()
    };
    json!({
        "corners": list(&[Position::CB], 4),
        "receivers": list(&[Position::WR, Position::TE], 5),
        "qbs": list(&[Position::QB], 2),
    })
}

/// Your play calling so far this season and in your career's kept games:
/// your own snaps against your staff's (topic 12: the world judges you by
/// results).
fn calling_record(w: &World) -> Value {
    let Some((_, t)) = w.seat_team() else { return Value::Null };
    let mut mine = (0u32, 0.0f64, 0u32);
    let mut staff = (0u32, 0.0f64, 0u32);
    let mut games = 0;
    for g in w.seat_games().iter().filter(|g| g.date.parts().0 >= w.year - 1 || g.date >= w.today.add(-200)) {
        let Some(side) = g.teams.iter().position(|&x| x == t) else { continue };
        let mut any = false;
        for r in &g.stats.plays {
            if !matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble) || r.nullified {
                continue;
            }
            let yours = (r.offense == side && r.yours & 1 != 0) || (r.offense != side && r.yours & 2 != 0);
            let epa = if r.offense == side { r.epa } else { -r.epa };
            let ok = success(r) == (r.offense == side);
            let e = if yours { &mut mine } else { &mut staff };
            e.0 += 1;
            e.1 += epa;
            e.2 += ok as u32;
            any |= yours;
        }
        games += any as u32;
    }
    let row = |x: (u32, f64, u32)| json!({ "snaps": x.0, "epa": if x.0 > 0 { x.1 / x.0 as f64 } else { 0.0 }, "success": if x.0 > 0 { 100.0 * x.2 as f64 / x.0 as f64 } else { 0.0 } });
    json!({ "games": games, "yours": row(mine), "staff": row(staff) })
}

/// A successful snap for the offense: 40% of the distance on 1st down,
/// 60% on 2nd, all of it on 3rd and 4th (the usual definition).
pub(crate) fn success(r: &PlayRecord) -> bool {
    let need = match r.down {
        1 => 0.4 * r.togo,
        2 => 0.6 * r.togo,
        _ => r.togo,
    };
    !r.turnover && (r.touchdown || r.first_down || r.yards >= need)
}

// ------------------------------------------------------------ the week's work

fn set_settings(w: &mut World, a: &Value) -> Reply {
    let s = &mut w.gameday.settings;
    if let Some(v) = a.get("stop").and_then(Value::as_str) {
        s.stop = match v {
            "every" => StopMode::EverySnap,
            "off" => StopMode::Off,
            _ => StopMode::Situations,
        };
    }
    if let Some(x) = a.get("situations") {
        let b = |k: &str, d: bool| x.get(k).and_then(Value::as_bool).unwrap_or(d);
        let t = &mut s.situations;
        t.third_down = b("third_down", t.third_down);
        t.fourth_down = b("fourth_down", t.fourth_down);
        t.red_zone = b("red_zone", t.red_zone);
        t.goal_line = b("goal_line", t.goal_line);
        t.two_minute = b("two_minute", t.two_minute);
        t.decisions = b("decisions", t.decisions);
    }
    if let Some(v) = a.get("clock").and_then(Value::as_str) {
        s.clock = if v == "realism" { CallClock::Realism } else { CallClock::Learning };
    }
    let now = *s;
    // A game under way takes the new settings from its next snap on.
    if let Some(l) = w.live.as_mut() {
        l.game.ctl.settings = now;
    }
    Ok(json!({ "settings": settings_json(&now) }))
}

fn set_rules(w: &mut World, a: &Value) -> Reply {
    let r = &mut w.gameday.rules;
    if let Some(v) = a.get("fourth").and_then(Value::as_str) {
        r.fourth = match v {
            "numbers" => FourthChart::Numbers,
            "aggressive" => FourthChart::Aggressive,
            "conservative" => FourthChart::Conservative,
            _ => FourthChart::Staff,
        };
    }
    if let Some(v) = a.get("two").and_then(Value::as_str) {
        r.two = match v {
            "chart" => TwoChart::Chart,
            "kick" => TwoChart::Kick,
            _ => TwoChart::Staff,
        };
    }
    if let Some(v) = a.get("tempo").and_then(Value::as_str) {
        r.tempo = tempo_of(v);
    }
    Ok(hub(w))
}

/// The week's plan is for your next game; a plan for an older game is
/// dropped.
fn plan_for_next(w: &mut World) -> Result<Date, String> {
    let d = next_game(w).ok_or("No game left on your schedule.")?.date;
    if w.gameday.plan.date != Some(d) {
        w.gameday.plan = Default::default();
        w.gameday.plan.date = Some(d);
    }
    Ok(d)
}

fn pick_of(v: &Value) -> Option<Option<u32>> {
    match v {
        Value::Number(n) => Some(n.as_u64().map(|x| x as u32)),
        Value::String(s) if s == "none" => Some(None),
        _ => None,
    }
}

fn set_plan(w: &mut World, a: &Value) -> Reply {
    if w.live.as_ref().is_some_and(|l| l.date == w.today) {
        return Err("The game is under way: change the plan from the sideline.".into());
    }
    plan_for_next(w)?;
    let p = &mut w.gameday.plan;
    if let Some(v) = a.get("target_cb") {
        p.target_cb = pick_of(v);
    }
    if let Some(v) = a.get("bracket") {
        p.bracket = pick_of(v);
    }
    if let Some(v) = a.get("spy") {
        p.spy = v.as_bool();
    }
    if let Some(v) = a.get("kick_away") {
        p.kick_away = v.as_bool();
    }
    if let Some(v) = a.get("draws") {
        p.draws = v.as_bool();
    }
    Ok(hub(w))
}

fn tape_key(a: &Value) -> Result<(Date, usize, usize), String> {
    let n = |k: &str| a.get(k).and_then(Value::as_u64).ok_or(format!("Missing {k}."));
    Ok((Date(n("date")? as i32), n("home")? as usize, n("away")? as usize))
}

fn study(w: &mut World, a: &Value) -> Reply {
    let (d, h, aw) = tape_key(a)?;
    let next = next_game(w).ok_or("No game left on your schedule.")?;
    if !w.gameday.tapes.iter().any(|t| t.date == d && t.teams == [h, aw]) {
        return Err("That tape is no longer in your film room.".into());
    }
    if w.gameday.studied.iter().any(|s| s.0 == next.date && s.1 == d && [s.2, s.3] == [h, aw]) {
        return Ok(hub(w));
    }
    let (budget, per) = film_budget(w, next.date);
    let used = w.gameday.studied.iter().filter(|s| s.0 == next.date).count() as f64 * per;
    if used + per > budget + 0.01 {
        return Err(format!("Your film time this week is spent: {used:.0} of {budget:.0} hours."));
    }
    w.gameday.studied.push((next.date, d, h, aw));
    // Only this season's study is kept.
    let cut = w.today.add(-200);
    w.gameday.studied.retain(|s| s.0 >= cut);
    Ok(hub(w))
}

fn hint(w: &mut World, a: &Value) -> Reply {
    let (d, h, aw) = tape_key(a)?;
    let key = a.get("key").and_then(Value::as_str).ok_or("Missing key.")?.to_string();
    let add = a.get("add").and_then(Value::as_bool).unwrap_or(true);
    if w.live.as_ref().is_some_and(|l| l.date == w.today) {
        return Err("The game is under way: the plan is set.".into());
    }
    let next = plan_for_next(w)?;
    let (level, t) = w.seat_team().ok_or("You have no team.")?;
    let tape = w.gameday.tapes.iter().find(|x| x.date == d && x.teams == [h, aw]).ok_or("That tape is gone.")?;
    let opp = next_game(w).map(|n| n.opp).ok_or("No game.")?;
    let side = tape.side_of(opp).ok_or("That tape isn't of your next opponent.")?;
    let names = names(w, &[team_ref(level, t), team_ref(level, opp)]);
    let found =
        film::hints(tape, side, &names).into_iter().find(|x| x.key == key).ok_or("That hint is not on this tape.")?;
    if found.effect == film::Effect::Note {
        return Err("Nothing in the plan answers that one: the battles decide it.".into());
    }
    let p = &mut w.gameday.plan;
    p.hints.retain(|x| !(x.0 == d && [x.1, x.2] == [h, aw] && x.3 == key));
    // The same habit on another tape still in the plan keeps its change.
    let still = p.hints.iter().any(|x| x.3 == key);
    if add {
        match found.effect {
            film::Effect::TargetCb(id) => p.target_cb = Some(Some(id)),
            film::Effect::Bracket(id) => p.bracket = Some(Some(id)),
            film::Effect::Spy => p.spy = Some(true),
            film::Effect::KickAway => p.kick_away = Some(true),
            film::Effect::Draws => p.draws = Some(true),
            _ => {}
        }
        p.hints.push((d, h, aw, key, found.snaps));
    } else if !still {
        // Taking a hint out gives its part of the plan back to your staff.
        match found.effect {
            film::Effect::TargetCb(id) if p.target_cb == Some(Some(id)) => p.target_cb = None,
            film::Effect::Bracket(id) if p.bracket == Some(Some(id)) => p.bracket = None,
            film::Effect::Spy if p.spy == Some(true) => p.spy = None,
            film::Effect::KickAway if p.kick_away == Some(true) => p.kick_away = None,
            film::Effect::Draws if p.draws == Some(true) => p.draws = None,
            _ => {}
        }
    }
    let _ = next;
    Ok(hub(w))
}

// ------------------------------------------------------------ the live game

fn off_json(c: &OffCall, today: &[&PlayRecord]) -> Value {
    let f = c.formation;
    let back = match f.backfield {
        Backfield::Shotgun => "shotgun",
        Backfield::Pistol => "pistol",
        Backfield::UnderCenter => "under center",
    };
    let play = c.describe();
    // How it has worked today: the same play from your side.
    let same: Vec<&&PlayRecord> = today.iter().filter(|r| r.call.contains(&play)).collect();
    let yds: f64 = same.iter().map(|r| r.yards).sum();
    let run = matches!(c.kind, PlayKind::Run(_));
    json!({
        "look": format!("{} personnel, {back}, {}", f.personnel.name(), f.set.name()),
        "play": play,
        "why": c.why,
        "family": call_family(c.kind).name(),
        "run": run,
        "kind": match c.kind { PlayKind::Run(_) => "run", PlayKind::Pass(_) => "pass", PlayKind::Rpo(..) => "RPO", PlayKind::Kneel => "kneel", PlayKind::Spike => "spike" },
        "today": if same.is_empty() { Value::Null } else { json!({ "n": same.len(), "yds": yds / same.len() as f64 }) },
    })
}

fn def_json(c: &DefCall, today: &[&PlayRecord]) -> Value {
    let text = c.describe();
    let same: Vec<&&PlayRecord> = today.iter().filter(|r| r.defense_call == text).collect();
    let yds: f64 = same.iter().map(|r| r.yards).sum();
    json!({
        "text": text,
        "personnel": format!("{:?}", c.personnel),
        "coverage": c.coverage.name(),
        "pressure": c.pressure.name(),
        "front": c.front.name(),
        "stunt": c.stunt.name(),
        "disguise": c.disguise,
        "spy": c.spy,
        "man": c.coverage.is_man(),
        "deep": c.coverage.high(),
        "today": if same.is_empty() { Value::Null } else { json!({ "n": same.len(), "yds": yds / same.len() as f64 }) },
    })
}

fn toss_name(t: TossChoice) -> &'static str {
    match t {
        TossChoice::Receive => "receive",
        TossChoice::Defer => "defer",
        TossChoice::Kick => "kick",
        TossChoice::Goal => "goal",
        TossChoice::DefenseFirst => "defense_first",
        TossChoice::OffenseFirst => "offense_first",
    }
}

fn fourth_name(f: FourthDown) -> &'static str {
    match f {
        FourthDown::Go => "go",
        FourthDown::Punt => "punt",
        FourthDown::FieldGoal => "fg",
    }
}

fn view_json(v: &View, mine: usize, rules_q: f64) -> Value {
    let to_goal = 100.0 - v.x;
    json!({
        "quarter": v.quarter,
        "q": quarter_text(v.quarter, v.overtime),
        "clock": clock_text(v.clock),
        "secs": v.clock,
        "overtime": v.overtime,
        "poss": v.poss,
        "ours": v.poss == mine,
        "x": v.x,
        "ball_y": v.ball_y,
        "spot": spot(to_goal),
        "down": v.down,
        "dd": down_text(v.down, v.togo, to_goal),
        "togo": v.togo,
        "to_goal": to_goal,
        "score": v.score,
        "timeouts": v.timeouts,
        "challenges": v.challenges,
        "running": v.running,
        "wp_home": v.wp_home,
        "wp_mine": if mine == 0 { v.wp_home } else { 1.0 - v.wp_home },
        "plays": v.plays,
        "tempo": tempo_name(v.tempo),
        "wind_home": v.wind_home,
        "quarter_len": rules_q,
    })
}

fn pending_json(
    p: &Pending,
    mine: usize,
    rules_q: f64,
    today: &[&PlayRecord],
    names: &film::Names,
    can_defer: bool,
) -> Value {
    let ask = match p.ask {
        Ask::Snap => "snap",
        Ask::Defense => "defense",
        Ask::Pat => "pat",
        Ask::Onside => "onside",
        Ask::Toss => "toss",
        Ask::Challenge => "challenge",
        Ask::Halftime => "halftime",
    };
    let mut offense: Vec<Value> = Vec::new();
    if let Some(c) = &p.staff_off {
        let mut v = off_json(c, today);
        v["staff"] = json!(true);
        offense.push(v);
    }
    offense.extend(p.off_options.iter().map(|c| off_json(c, today)));
    let mut defense: Vec<Value> = Vec::new();
    if let Some(c) = &p.staff_def {
        let mut v = def_json(c, today);
        v["staff"] = json!(true);
        defense.push(v);
    }
    defense.extend(p.def_options.iter().map(|c| def_json(c, today)));
    let toss: Vec<&str> = if p.ask == Ask::Toss {
        if p.view.overtime {
            if p.view.quarter >= 5
                && p.staff_toss.is_some_and(|t| matches!(t, TossChoice::DefenseFirst | TossChoice::OffenseFirst))
            {
                vec!["defense_first", "offense_first"]
            } else {
                vec!["receive", "kick"]
            }
        } else if can_defer {
            vec!["receive", "defer", "kick", "goal"]
        } else {
            vec!["receive", "kick", "goal"]
        }
    } else {
        Vec::new()
    };
    json!({
        "ask": ask,
        "view": view_json(&p.view, mine, rules_q),
        "staff_fourth": p.staff_fourth.map(fourth_name),
        "staff_two": p.staff_two,
        "staff_onside": p.staff_onside,
        "staff_toss": p.staff_toss.map(toss_name),
        "staff_challenge": p.staff_challenge,
        "offense": offense,
        "defense": defense,
        "their_personnel": p.their_personnel.map(|x| x.name()),
        "advice": p.advice.iter().map(|a| page::with_names(a, names)).collect::<Vec<_>>(),
        "ruling": p.ruling.as_ref().map(|r| page::with_names(r, names)),
        "fourth_wp": p.fourth_wp,
        "fg_make": p.fg_make,
        "toss": toss,
    })
}

/// A play for the feed: what happened, who called it and how it changed
/// the game.
fn play_row(r: &PlayRecord, names: &film::Names, sides: &[String; 2], mine: usize, m: &Wp) -> Value {
    let name = |id: u32| names.get(&id).map_or(format!("No. {id}"), |x| x.0.clone());
    let downless = matches!(r.kind, PlayType::Kickoff | PlayType::ExtraPoint | PlayType::TwoPoint);
    let off = r.offense.min(1);
    let key = r.touchdown
        || r.turnover
        || r.safety
        || r.kind == PlayType::Sack
        || r.yards >= 20.0
        || (r.kind == PlayType::FieldGoal)
        || r.officiating.as_ref().is_some_and(|o| o.reviewer != live::Reviewer::None)
        || !r.injuries.is_empty()
        || (r.down == 4 && matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble));
    let wp = page::wp_before(r, m);
    json!({
        "n": r.n,
        "q": quarter_text(r.quarter, r.quarter > 4),
        "clock": clock_text(r.clock),
        "off": sides[off],
        "ours": off == mine,
        "dd": if downless { String::new() } else { format!("{} at {}", down_text(r.down, r.togo, r.yardline_100), spot(r.yardline_100)) },
        "text": views::play_text(r, &name),
        "call": r.call,
        "defense": r.defense_call,
        "why": r.call_why,
        "yours": r.yours,
        "off_sheet": r.off_sheet,
        "staff_call": r.staff_call,
        "epa": r.epa,
        "score": r.score,
        "key": key,
        "td": r.touchdown,
        "turnover": r.turnover,
        "officiating": r.officiating.as_ref().map(|o| page::with_names(&o.note, names)),
        "injuries": r.injuries.iter().map(|i| format!("{} is hurt ({}, about {:.0} weeks)", name(i.id), i.kind.name(), i.weeks)).collect::<Vec<_>>(),
        "wp_home": wp,
        "x": 100.0 - r.yardline_100,
        "gain": r.yards,
        "kind": format!("{:?}", r.kind),
        "who": page::who(r, names),
    })
}

fn live_state(w: &mut World) -> Reply {
    let Some(l) = w.live.as_mut() else {
        return Ok(json!({ "status": "none" }));
    };
    let step = l.game.run();
    let (level, teams, sides_names, date, mine) = (l.level, l.teams, l.names.clone(), l.date, l.game.ctl.team);
    let settings = l.game.ctl.settings;
    let rules_q = l.game.params.rules.quarter;
    let cond = l.game.params.cond;
    let neutral = l.game.params.neutral;
    let take_next = l.game.ctl.take_next;
    // An NFL toss winner may defer only from 2008.
    let can_defer = level != CalLevel::Nfl || l.game.home.staff.year >= 2008;
    let w: &World = w;
    let refs = [team_ref(level, teams[0]), team_ref(level, teams[1])];
    let names = names(w, &refs);
    let r = &step.result;
    let m = Wp::new(w, level, teams, date);
    let today: Vec<&PlayRecord> = r.stats.plays.iter().filter(|p| p.offense == mine).collect();
    let today_def: Vec<&PlayRecord> = r.stats.plays.iter().filter(|p| p.offense != mine && p.offense < 2).collect();
    let pending = step.pending.as_ref().map(|p| {
        let pool = if p.ask == Ask::Defense { &today_def } else { &today };
        pending_json(p, mine, rules_q, pool, &names, can_defer)
    });
    let feed: Vec<Value> = r.stats.plays.iter().map(|p| play_row(p, &names, &sides_names, mine, &m)).collect();
    let log: Vec<Value> =
        r.log.iter().map(|(n, t, what)| json!({ "n": n, "team": t, "text": page::with_names(what, &names) })).collect();
    let totals = page::totals(&r.stats, &sides_names);
    let over = step.pending.is_none();
    let after = if over { Some(page::after_game(w, r, mine, &names, &sides_names, &m)) } else { None };
    let halftime =
        step.pending.as_ref().filter(|p| p.ask == Ask::Halftime).map(|_| page::halftime(r, mine, &names, &sides_names));
    let roster: Vec<Value> = {
        let l = w.live.as_ref().unwrap();
        let them = if mine == 0 { &l.game.away } else { &l.game.home };
        them.players
            .iter()
            .filter(|p| matches!(p.position.name(), "CB" | "WR" | "TE" | "QB"))
            .map(|p| {
                let (nm, pos) =
                    names.get(&p.id).cloned().unwrap_or((format!("No. {}", p.id), p.position.name().into()));
                let touches = r
                    .stats
                    .plays
                    .iter()
                    .filter(|x| x.target == Some(p.id) || x.defender == Some(p.id) || x.carrier == Some(p.id))
                    .count();
                json!({ "id": p.id, "name": nm, "pos": pos, "today": touches })
            })
            .collect()
    };
    Ok(json!({
        "status": if over { "final" } else { "live" },
        "date": views::date_text(date),
        "home": views::team_brief(w, refs[0]),
        "away": views::team_brief(w, refs[1]),
        "names": sides_names,
        "mine": mine,
        "neutral": neutral,
        "view": view_json(&step.view, mine, rules_q),
        "pending": pending,
        "plays": feed,
        "log": log,
        "totals": totals,
        "wp": page::wp_series(&r.stats, &m),
        "settings": settings_json(&settings),
        "take_next": take_next,
        "weather": weather_json(&cond, level_of(level), mine == 0 || neutral),
        "their_players": roster,
        "after": after,
        "halftime": halftime,
        "frames": r.frames.iter().map(|f| f.0).collect::<Vec<_>>(),
        "toss": r.toss.map(|(t, c)| json!({ "winner": sides_names[t], "choice": toss_name(c), "mine": t == mine })),
        "sheet": sheet(),
    }))
}

fn level_of(l: CalLevel) -> afm_engine::player::Level {
    match l {
        CalLevel::Nfl | CalLevel::Other => afm_engine::player::Level::NFL,
        CalLevel::HighSchool => afm_engine::player::Level::HS,
        _ => afm_engine::player::Level::FBS,
    }
}

/// The whole play sheet for the picker: every formation piece and play.
fn sheet() -> Value {
    let runs: Vec<&str> = RUN_CONCEPTS.iter().map(|r| r.name()).collect();
    let passes: Vec<Value> = PASS_CONCEPTS
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.name.starts_with("RPO"))
        .map(|(i, c)| json!({ "i": i, "name": c.name, "family": afm_engine::game::playbook::pass_family(c).name() }))
        .collect();
    let rpos: Vec<Value> = PASS_CONCEPTS
        .iter()
        .enumerate()
        .filter(|(_, c)| c.name.starts_with("RPO"))
        .map(|(i, c)| json!({ "i": i, "name": c.name }))
        .collect();
    json!({
        "personnel": ["11", "12", "21", "10", "22", "13"],
        "backfield": ["under center", "shotgun", "pistol"],
        "set": ["doubles", "trips", "bunch", "stack", "empty", "wing"],
        "motion": ["none", "jet", "orbit", "across", "shift"],
        "runs": runs,
        "passes": passes,
        "rpos": rpos,
        "def_personnel": ["Base", "Nickel", "Dime", "GoalLine"],
        "coverages": Coverage::ALL.iter().map(|c| c.name()).collect::<Vec<_>>(),
        "pressures": ["four-man rush", "linebacker blitz", "DB blitz", "double-A blitz", "simulated pressure", "fire zone"],
        "fronts": ["over", "under", "wide nine", "odd", "tite", "bear"],
        "stunts": ["none", "T-E stunt", "E-T stunt", "tackle twist"],
    })
}

/// A call you built on the full sheet.
fn custom_off(a: &Value) -> Result<OffCall, String> {
    let s = |k: &str| a.get(k).and_then(Value::as_str).unwrap_or("");
    let personnel = match s("personnel") {
        "12" => Personnel::P12,
        "21" => Personnel::P21,
        "10" => Personnel::P10,
        "22" => Personnel::P22,
        "13" => Personnel::P13,
        _ => Personnel::P11,
    };
    let backfield = match s("backfield") {
        "under center" => Backfield::UnderCenter,
        "pistol" => Backfield::Pistol,
        _ => Backfield::Shotgun,
    };
    let set = match s("set") {
        "trips" => Set::Trips,
        "bunch" => Set::Bunch,
        "stack" => Set::Stack,
        "empty" => Set::Empty,
        "wing" => Set::Wing,
        _ => Set::Doubles,
    };
    let motion = match s("motion") {
        "jet" => Motion::Jet,
        "orbit" => Motion::Orbit,
        "across" => Motion::Across,
        "shift" => Motion::Shift,
        _ => Motion::None,
    };
    let dir = if s("dir") == "right" { -1.0 } else { 1.0 };
    let play = s("play");
    let run = |n: &str| RUN_CONCEPTS.iter().copied().find(|r| r.name() == n);
    let pass = |n: &str| PASS_CONCEPTS.iter().position(|c| c.name == n);
    let kind = match s("kind") {
        "run" => PlayKind::Run(run(play).ok_or("Pick a run.")?),
        "rpo" => PlayKind::Rpo(
            run(s("run")).unwrap_or(afm_engine::game::playbook::RunConcept::InsideZone),
            pass(play).ok_or("Pick an RPO.")?,
        ),
        "kneel" => PlayKind::Kneel,
        "spike" => PlayKind::Spike,
        _ => PlayKind::Pass(pass(play).ok_or("Pick a pass concept.")?),
    };
    let strength = if s("strength") == "right" { -1.0 } else { 1.0 };
    let f = Formation { personnel, backfield, set, strength, motion };
    Ok(OffCall::new(f, kind, dir, false))
}

fn custom_def(a: &Value) -> Result<DefCall, String> {
    let s = |k: &str| a.get(k).and_then(Value::as_str).unwrap_or("");
    let personnel = match s("personnel") {
        "Base" => DefPersonnel::Base,
        "Dime" => DefPersonnel::Dime,
        "GoalLine" => DefPersonnel::GoalLine,
        _ => DefPersonnel::Nickel,
    };
    let coverage = Coverage::ALL.iter().copied().find(|c| c.name() == s("coverage")).ok_or("Pick a coverage.")?;
    let pressure = [
        Pressure::Four,
        Pressure::LbBlitz,
        Pressure::DbBlitz,
        Pressure::DoubleA,
        Pressure::Simulated,
        Pressure::FireZone,
    ]
    .into_iter()
    .find(|p| p.name() == s("pressure"))
    .unwrap_or(Pressure::Four);
    let front = [FrontLook::Over, FrontLook::Under, FrontLook::Wide, FrontLook::Odd, FrontLook::Tite, FrontLook::Bear]
        .into_iter()
        .find(|f| f.name() == s("front"))
        .unwrap_or(FrontLook::Over);
    let stunt = [Stunt::TackleEnd, Stunt::EndTackle, Stunt::Twist]
        .into_iter()
        .find(|x| x.name() == s("stunt"))
        .unwrap_or(Stunt::None);
    let mut d = DefCall::new(personnel, coverage, pressure);
    d.front = front;
    d.stunt = stunt;
    d.disguise = a.get("disguise").and_then(Value::as_bool).unwrap_or(false);
    d.spy = a.get("spy").and_then(Value::as_bool).unwrap_or(false);
    Ok(d)
}

fn plan_change(a: &Value, them: &GameTeam) -> PlanChange {
    let at = |v: &Value| -> Option<Option<usize>> {
        match pick_of(v)? {
            None => Some(None),
            Some(id) => them.players.iter().position(|p| p.id == id).map(Some),
        }
    };
    PlanChange {
        target_cb: a.get("target_cb").and_then(at),
        bracket: a.get("bracket").and_then(at),
        spy: a.get("spy").and_then(Value::as_bool),
        kick_away: a.get("kick_away").and_then(Value::as_bool),
        draws: a.get("draws").and_then(Value::as_bool),
    }
}

fn answer(w: &mut World, a: &Value) -> Reply {
    let l = w.live.as_mut().ok_or("No game is being played.")?;
    let step = l.game.run();
    let p = step.pending.ok_or("The game is over.")?;
    let kind = a.get("kind").and_then(Value::as_str).unwrap_or("staff");
    let opt = a.get("option").and_then(Value::as_u64).map(|x| x as usize);
    let offense = |p: &Pending| -> Result<Option<OffCall>, String> {
        if let Some(c) = a.get("custom") {
            return custom_off(c).map(Some);
        }
        Ok(match opt {
            None => None,
            Some(0) if p.staff_off.is_some() => p.staff_off,
            Some(k) => {
                let k = if p.staff_off.is_some() { k - 1 } else { k };
                Some(*p.off_options.get(k).ok_or("No such call.")?)
            }
        })
    };
    let input = match kind {
        "timeout" => {
            if p.view.timeouts[l.game.ctl.team] == 0 {
                return Err("You have no timeouts left in this half.".into());
            }
            Input::Timeout
        }
        "tempo" => Input::Tempo(tempo_of(a.get("tempo").and_then(Value::as_str).unwrap_or("normal"))),
        "plan" | "halftime" => {
            let them = if l.game.ctl.team == 0 { &l.game.away } else { &l.game.home };
            let pc = plan_change(a, them);
            if kind == "halftime" { Input::Halftime(pc) } else { Input::Plan(pc) }
        }
        "call" => {
            let fourth = match a.get("fourth").and_then(Value::as_str) {
                Some("punt") => Some(FourthDown::Punt),
                Some("fg") => Some(FourthDown::FieldGoal),
                Some("go") => Some(FourthDown::Go),
                _ => None,
            };
            let off = if fourth.is_some_and(|f| f != FourthDown::Go) { None } else { offense(&p)? };
            Input::Call { fourth, offense: off, defense: None }
        }
        "defense" => {
            let d = if let Some(c) = a.get("custom") {
                custom_def(c)?
            } else {
                match opt {
                    Some(0) | None => p.staff_def.ok_or("No call to make.")?,
                    Some(k) => *p.def_options.get(k - 1).ok_or("No such call.")?,
                }
            };
            Input::Call { fourth: None, offense: None, defense: Some(d) }
        }
        "pat" => {
            let two = a.get("two").and_then(Value::as_bool).unwrap_or(false);
            Input::Pat { two, offense: if two { offense(&p)? } else { None } }
        }
        "onside" => Input::Onside(a.get("onside").and_then(Value::as_bool).unwrap_or(false)),
        "toss" => Input::Toss(match a.get("choice").and_then(Value::as_str).unwrap_or("defer") {
            "receive" => TossChoice::Receive,
            "kick" => TossChoice::Kick,
            "goal" => TossChoice::Goal,
            "defense_first" => TossChoice::DefenseFirst,
            "offense_first" => TossChoice::OffenseFirst,
            _ => TossChoice::Defer,
        }),
        "challenge" => Input::Challenge(a.get("throw").and_then(Value::as_bool).unwrap_or(false)),
        _ => Input::Staff,
    };
    // Settings may come with the answer (the stop mode changed on the
    // sideline).
    if let Some(stop) = a.get("stop").and_then(Value::as_str) {
        l.game.ctl.settings.stop = match stop {
            "every" => StopMode::EverySnap,
            "off" => StopMode::Off,
            _ => StopMode::Situations,
        };
    }
    l.game.answer(input);
    live_state(w)
}

/// The tracking of one play of your live game, for the 2D and 3D views
/// (Lane 13): the plays since your last decision are kept.
fn frames(w: &mut World, a: &Value) -> Reply {
    let n = a.get("play").and_then(Value::as_u64).ok_or("Missing play.")? as u32;
    let l = w.live.as_mut().ok_or("No game is being played.")?;
    let r: GameResult = l.game.run().result;
    let f = r
        .frames
        .iter()
        .find(|x| x.0 == n)
        .ok_or("That play's tracking is not kept: only the plays since your last call are.")?;
    let play = r.stats.plays.iter().find(|p| p.n == n);
    Ok(json!({
        "play": n,
        "frames": f.1,
        "on_field": play.map(|p| p.on_field.clone()),
        "offense": play.map(|p| p.offense),
        "los": play.map(|p| 100.0 - p.yardline_100),
    }))
}
