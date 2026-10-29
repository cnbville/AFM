//! Front office: the college desk's commands (lane 10). The recruiting
//! board, offers, visits, the transfer portal and NIL for your program,
//! through `afm_engine::world::front_office::college`. Every rating is
//! your staff's read as a range, potential only in words, and a recruit's
//! interest in your staff's words, never a number.

use std::collections::HashMap;

use afm_engine::calendar::Date;
use afm_engine::player::PlayingTime;
use afm_engine::positions::Position;
use afm_engine::world::background::Decider;
use afm_engine::world::front_office::college::{self as desk, Source, Standing, VisitKind, VisitState};
use afm_engine::world::front_office::{Duty, Owner, chain, dollars};
use afm_engine::world::program::NilTrend;
use afm_engine::world::{Person, Stage, Visit, World, geo, ncaa};
use serde_json::{Value, json};

use crate::views::{self, TeamRef};
use crate::{Game, Reply, colors, reads};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let reply = match cmd {
        "recruit_board" => with(game, board),
        "recruit_map" => with(game, map),
        "recruit_area" => set_area(game, args),
        "recruit_file" => with_args(game, args, file),
        "visits_desk" => with(game, visits),
        "portal_desk" => with_args(game, args, portal),
        "nil_desk" => with(game, nil),
        "recruit_chain" => with(game, chain_view),
        "chain_sign" | "chain_decline" | "chain_sign_all" | "chain_back" => chain_act(game, cmd, args),
        "recruit_add" | "recruit_remove" | "recruit_move" | "recruit_priority" | "recruit_offer" | "recruit_pull"
        | "recruit_promise" | "visit_set" | "visit_cancel" | "portal_offer" | "portal_pull" | "portal_forget"
        | "portal_contact" | "keep_talk" | "nil_set" | "nil_usual" | "nil_clear" | "college_duty" | "college_fill"
        | "college_cut" => act(game, cmd, args),
        _ => return None,
    };
    Some(reply)
}

/// Your word on the chain's list (SK26, SK31).
fn chain_act(game: &mut Game, cmd: &str, args: &Value) -> Reply {
    let w = game.world.as_mut().ok_or("No world is loaded.")?;
    program(w)?;
    desk::ready(w);
    let text = match cmd {
        "chain_sign" => chain::sign(w, id_arg(args)?)?,
        "chain_decline" => chain::decline(w, id_arg(args)?)?,
        "chain_back" => chain::back_on_board(w, id_arg(args)?)?,
        _ => chain::sign_all(w)?,
    };
    Ok(json!({ "text": text }))
}

fn with(game: &mut Game, f: fn(&World, usize) -> Reply) -> Reply {
    let w = game.world.as_mut().ok_or("No world is loaded.")?;
    desk::ready(w);
    f(w, program(w)?)
}

fn with_args(game: &mut Game, args: &Value, f: fn(&World, usize, &Value) -> Reply) -> Reply {
    let w = game.world.as_mut().ok_or("No world is loaded.")?;
    desk::ready(w);
    f(w, program(w)?, args)
}

fn program(w: &World) -> Result<usize, String> {
    match views::mine(w) {
        Some(TeamRef::College(j)) => Ok(j),
        _ => Err("The college desk is for a college head coach.".into()),
    }
}

fn id_arg(args: &Value) -> Result<u32, String> {
    args.get("id").and_then(Value::as_u64).map(|v| v as u32).ok_or_else(|| "Missing id.".into())
}

fn money_arg(args: &Value, k: &str) -> Option<f64> {
    args.get(k).and_then(Value::as_f64)
}

/// Every action: the desk's answer in words, for a toast.
fn act(game: &mut Game, cmd: &str, args: &Value) -> Reply {
    let w = game.world.as_mut().ok_or("No world is loaded.")?;
    let j = program(w)?;
    desk::ready(w);
    let text = match cmd {
        "recruit_add" => desk::add(w, id_arg(args)?)?,
        "recruit_remove" => desk::remove(w, id_arg(args)?)?,
        "recruit_move" => {
            let to = args.get("to").and_then(Value::as_u64).ok_or("Missing place.")? as usize;
            desk::reorder(w, id_arg(args)?, to)?
        }
        "recruit_priority" => {
            desk::set_priority(w, id_arg(args)?, args.get("on").and_then(Value::as_bool).unwrap_or(true))?
        }
        "recruit_offer" => desk::offer(w, id_arg(args)?)?,
        "recruit_pull" => desk::pull(w, id_arg(args)?)?,
        "recruit_promise" => desk::set_promise(w, id_arg(args)?, money_arg(args, "nil"))?,
        "visit_set" | "visit_cancel" => {
            let kind =
                args.get("kind").and_then(Value::as_str).and_then(VisitKind::from_key).ok_or("Missing visit kind.")?;
            if cmd == "visit_cancel" {
                desk::cancel_visit(w, id_arg(args)?, kind)?
            } else {
                let d = args.get("date").and_then(Value::as_i64).ok_or("Missing date.")? as i32;
                match desk::set_visit(w, id_arg(args)?, kind, Date(d)) {
                    Ok(t) => t,
                    // A no is an answer, not an error: the screen shows it.
                    Err(t) => return Ok(json!({ "text": t, "declined": true })),
                }
            }
        }
        "portal_offer" => desk::portal_offer(w, id_arg(args)?, money_arg(args, "nil"))?,
        "portal_pull" => desk::portal_pull(w, id_arg(args)?)?,
        "portal_forget" => desk::portal_forget(w, id_arg(args)?)?,
        "portal_contact" => desk::portal_contact(w, id_arg(args)?)?,
        "keep_talk" => desk::keep_talk(w, id_arg(args)?)?,
        "college_cut" => desk::cut(w, id_arg(args)?)?,
        "nil_set" => desk::set_deal(w, id_arg(args)?, money_arg(args, "amount").ok_or("Missing amount.")?)?,
        "nil_usual" => {
            let deals = desk::usual_deals(w, j);
            w.front_office.college.deals.clear();
            w.front_office.college.deals_year = desk::deals_year(w);
            let mut n = 0;
            for (id, x) in deals {
                if desk::set_deal(w, id, (x / 500.0).round() * 500.0).is_ok() {
                    n += 1;
                }
            }
            format!(
                "{n} {} set at the collective's usual split. Adjust any of them.",
                afm_engine::world::folk::noun(n, "deal")
            )
        }
        "nil_clear" => {
            w.front_office.college.deals.clear();
            "Every deal cleared.".into()
        }
        "college_duty" => {
            let duty = args.get("duty").and_then(Value::as_str).and_then(Duty::from_key).ok_or("Missing duty.")?;
            if !matches!(duty, Duty::Board | Duty::Visits | Duty::Portal | Duty::Nil) {
                return Err("That duty isn't the college desk's.".into());
            }
            let yours = args.get("owner").and_then(Value::as_str) == Some("you");
            let was = w.front_office.owner(duty);
            w.front_office.set_owner(duty, if yours { Owner::You } else { Owner::Staff });
            if duty == Duty::Board && yours && was == Owner::Staff {
                // Taking the board back: your staff's offers become yours.
                desk::seed(w, j);
            }
            let who = desk::coordinator_name(w, j);
            if yours {
                format!("{}: yours now.", duty.name())
            } else {
                format!("{}: your staff's now. {}", duty.name(), staff_line(&who, duty))
            }
        }
        "college_fill" => {
            let on = args.get("on").and_then(Value::as_bool).unwrap_or(true);
            w.front_office.fill_board = on;
            if on {
                "Your coordinator fills the spots your board leaves open.".into()
            } else {
                "Only your own offers go out.".into()
            }
        }
        _ => return Err(format!("Unknown command: {cmd}")),
    };
    Ok(json!({ "text": text }))
}

fn staff_line(who: &str, d: Duty) -> String {
    match d {
        Duty::Board => format!("{who} ranks the board; his offers and pulls wait for your signature each week."),
        Duty::Visits => format!("{who} brings the board's top names in for visits."),
        Duty::Portal => "Your staff chases the transfers who fill the two-deep's holes.".into(),
        _ => "Your collective pays the roster by what each player means to the team.".into(),
    }
}

// --- Words ------------------------------------------------------------------------------

/// The services' stars (a kicker, punter or snapper gets 3 at most from
/// them; his specialist camps' own stars are shown beside).
/// The offer chain (SK26-SK31): your staff's recommendations waiting for
/// your word, the ones you answered, and the recruits they passed on.
fn chain_view(w: &World, _j: usize) -> Reply {
    let c = &w.front_office.chain;
    let who = |id: u32| -> Option<(usize, &Person)> {
        let k = views::person_index(w, id)?;
        Some((k, &w.people[k]))
    };
    let coach = |c: usize| json!({ "id": c, "name": w.coaches.get(c).map(|x| x.name.as_str()).unwrap_or("") });
    let argue = |n: u8| match n {
        15.. => "Bangs the table for him",
        9..=14 => "Wants him",
        _ => "Would take him",
    };
    let rec = |r: &chain::Rec| -> Option<Value> {
        let (k, p) = who(r.id)?;
        Some(json!({
            "id": r.id, "name": p.name, "pos": p.player.position.name(), "cls": views::class_text(w, p),
            "stars": stars_of(p), "hs": hs_brief(w, p.hs), "coach": coach(r.coach),
            "film": r.film.map(coach), "date": short_date(r.date), "read": [r.read.0, r.read.1], "line": r.line,
            "now": reads::overall(w, k).json(), "yours": reads::mine(w, k).json(),
            "conviction": r.conviction, "argue": argue(r.conviction), "why": r.why, "gem": r.gem, "pull": r.pull,
            "answer": format!("{:?}", r.answer), "answered": r.answered.map(short_date),
        }))
    };
    let waiting: Vec<Value> = chain::waiting(w).into_iter().filter_map(rec).collect();
    let answered: Vec<Value> =
        c.recs.iter().rev().filter(|r| r.answer != chain::Answer::Waiting).take(30).filter_map(rec).collect();
    let passed: Vec<Value> = c
        .passed
        .iter()
        .rev()
        .filter_map(|x| {
            let (k, p) = who(x.id)?;
            Some(json!({
                "id": x.id, "name": p.name, "pos": p.player.position.name(), "cls": views::class_text(w, p),
                "stars": x.stars, "hs": hs_brief(w, p.hs), "coach": coach(x.coach), "date": short_date(x.date),
                "why": x.why, "then": [x.read.0, x.read.1], "now": reads::overall(w, k).json(),
                "yours": reads::mine(w, k).json(), "flag": x.flag, "back": x.back,
                "where": views::where_now(w, p),
            }))
        })
        .collect();
    let gems = waiting.iter().filter(|r| r["gem"] == true).count();
    Ok(json!({
        "board_yours": chain::board_is_yours(w),
        "waiting": waiting, "answered": answered, "passed": passed, "gems": gems,
        "flags": c.passed.iter().filter(|x| x.flag).count(),
    }))
}

fn stars_of(p: &Person) -> u8 {
    p.stars
}

fn camp_stars(p: &Person) -> Option<u8> {
    (p.player.position.is_specialist() && p.camp_stars > 0 && p.camp_stars != p.stars).then_some(p.camp_stars)
}

fn name(w: &World, j: usize) -> String {
    w.program_name(j).to_string()
}

fn brief(w: &World, j: usize) -> Value {
    views::team_brief(w, TeamRef::College(j))
}

fn hs_brief(w: &World, h: usize) -> Value {
    views::team_brief(w, TeamRef::HighSchool(h))
}

fn date(d: Date) -> String {
    views::date_text(d)
}

fn short_date(d: Date) -> String {
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (_, m, day) = d.parts();
    format!("{} {day} {}", d.weekday().short(), M[(m - 1) as usize])
}

fn month(d: Date) -> &'static str {
    const M: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    M[(d.month() - 1) as usize]
}

fn standing_key(s: Standing) -> &'static str {
    match s {
        Standing::Open => "open",
        Standing::CommittedHere => "committed",
        Standing::SignedHere => "signed",
        Standing::CommittedElsewhere => "committed_elsewhere",
        Standing::SignedElsewhere => "signed_elsewhere",
        Standing::Enrolled => "enrolled",
        Standing::Gone => "gone",
    }
}

fn source_word(s: Source) -> &'static str {
    match s {
        Source::You => "You",
        Source::Coordinator => "Coordinator",
        Source::Inherited => "Staff, before you",
    }
}

/// The services' public ranks of a class as last printed (update 2, minor
/// bug 14): by position nationally, in each state, and overall.
struct Ranks {
    by_id: HashMap<u32, (u32, u32, u32)>,
}

impl Ranks {
    fn new(w: &World, classes: &[i32]) -> Ranks {
        let mut by_id = HashMap::new();
        for &c in classes {
            let Some(r) = afm_engine::world::scouting::services::class_ranks(w, c) else { continue };
            let mut pos = [0u32; afm_engine::positions::POSITION_COUNT];
            let mut state: HashMap<usize, u32> = HashMap::new();
            for (n, &id) in r.ids.iter().enumerate() {
                let Some(i) = afm_engine::world::scouting::place(w, id) else { continue };
                let p = &w.people[i];
                let k = p.player.position.index();
                pos[k] += 1;
                let st = state.entry(p.home_state).or_insert(0);
                *st += 1;
                by_id.insert(id, (pos[k], *st, n as u32 + 1));
            }
        }
        Ranks { by_id }
    }

    fn text(&self, w: &World, p: &Person) -> String {
        match self.by_id.get(&p.player.id) {
            Some(&(k, s, _)) => {
                let st = w.states.get(p.home_state).map_or("", |x| x.code.as_str());
                format!("No. {k} {} · No. {s} in {}", p.player.position.name(), views::state_name(st))
            }
            None => "Unranked by the services".into(),
        }
    }

    fn overall(&self, p: &Person) -> Option<u32> {
        self.by_id.get(&p.player.id).map(|r| r.2)
    }
}

/// Your staff's read of a recruit and its potential words, from his
/// scouting file (minor bug 54: the same read every screen shows), and
/// your own read beside it (SK23).
fn staff_read(w: &World, p: &Person) -> (Value, Value, Value) {
    let Some(k) = views::person_index(w, p.player.id) else { return (Value::Null, Value::Null, Value::Null) };
    let s = reads::seen(w, k);
    (s.overall().json(), reads::potential_words(w, k), s.mine().json())
}

/// The coach who reads him: his position coach on your staff.
fn reader_name(w: &World, j: usize, pos: Position) -> String {
    let prog = &w.programs[j];
    let g = afm_engine::world::coach::Group::of(pos);
    prog.position_coaches[g as usize]
        .or(if g.offense() { prog.oc } else { prog.dc })
        .map_or_else(|| "your staff".to_string(), |c| w.coaches[c].name.clone())
}

fn reader_role(w: &World, j: usize, pos: Position) -> String {
    let prog = &w.programs[j];
    let g = afm_engine::world::coach::Group::of(pos);
    match prog.position_coaches[g as usize] {
        Some(_) => g.coach_title().to_string(),
        None if g.offense() => "offensive coordinator".into(),
        None => "defensive coordinator".into(),
    }
}

/// His interest in your program in your staff's words: a headline, a
/// tone, a reason, and who leads.
pub(crate) struct Interest {
    pub(crate) words: String,
    pub(crate) tone: &'static str,
    pub(crate) reason: String,
    pub(crate) leader: Option<usize>,
}

pub(crate) fn interest(w: &World, j: usize, i: usize, offered: bool) -> Interest {
    let p = &w.people[i];
    let s = desk::standing(p, j);
    let first = p.name.split_whitespace().next().unwrap_or("He").to_string();
    match s {
        Standing::SignedHere if p.cal.signed.is_some_and(|x| x.walk_on) => {
            return Interest {
                words: "Walking on with you".into(),
                tone: "good",
                reason: "A preferred walk-on: a roster place without a scholarship.".into(),
                leader: Some(j),
            };
        }
        Standing::SignedHere => {
            return Interest {
                words: "Signed with you".into(),
                tone: "good",
                reason: "His letter is in. He's yours.".into(),
                leader: Some(j),
            };
        }
        Standing::Enrolled => {
            return Interest {
                words: "On campus".into(),
                tone: "good",
                reason: "Enrolled with you.".into(),
                leader: Some(j),
            };
        }
        Standing::SignedElsewhere => {
            let x = p.cal.signed.unwrap();
            let (words, reason) = if x.walk_on {
                (
                    format!("Walking on at {}", name(w, x.program)),
                    "A preferred walk-on there: no letter, but his mind is made up.",
                )
            } else {
                (format!("Signed with {}", name(w, x.program)), "A signed letter binds him.")
            };
            return Interest { words, tone: "bad", reason: reason.into(), leader: Some(x.program) };
        }
        Standing::Gone => {
            return Interest { words: "Off the market".into(), tone: "bad", reason: String::new(), leader: None };
        }
        _ => {}
    }
    let race = desk::race_at(w, i);
    let mine = if offered { race.iter().find(|r| r.0 == j).map(|r| r.1) } else { None };
    let yours = mine.unwrap_or_else(|| desk::appeal_of(w, i, j) - 0.12);
    let best = race.iter().find(|r| r.0 != j).copied();
    let hold = desk::hold(w, i);
    let reason = reason(w, j, i, best.map(|b| b.0));
    if let Some(c) = p.cal.commit {
        if c.0 == j {
            let pushing = best.filter(|b| b.1 > yours - 0.5 * hold);
            return match pushing {
                Some(b) => Interest {
                    words: format!("Committed to you, {} pushing", name(w, b.0)),
                    tone: "warn",
                    reason: format!("{} keeps calling. Keep him close.", name(w, b.0)),
                    leader: Some(j),
                },
                None => Interest {
                    words: "Committed to you".into(),
                    tone: "good",
                    reason: format!("{first} committed on {}. He sounds settled.", short_date(c.1)),
                    leader: Some(j),
                },
            };
        }
        let theirs = race.iter().find(|r| r.0 == c.0).map_or(0.0, |r| r.1);
        let listening = yours > theirs - hold;
        return Interest {
            words: if listening {
                format!("Committed to {}, listening", name(w, c.0))
            } else {
                format!("Committed to {}, firm", name(w, c.0))
            },
            tone: if listening { "warn" } else { "bad" },
            reason: if listening {
                format!("He still takes your calls. {reason}")
            } else {
                format!("He committed on {}. It would take something big.", short_date(c.1))
            },
            leader: Some(c.0),
        };
    }
    let Some(b) = best else {
        return Interest {
            words: if offered { "Your offer is his only one".into() } else { "Nobody has offered".into() },
            tone: if offered { "good" } else { "" },
            reason,
            leader: offered.then_some(j),
        };
    };
    let gap = yours - b.1;
    let other = name(w, b.0);
    if !offered {
        return Interest {
            words: if gap > -0.05 { "Would listen to an offer".into() } else { format!("Looking at {other}") },
            tone: "",
            reason,
            leader: Some(b.0),
        };
    }
    let (words, tone) = match gap {
        g if g > 0.08 => ("Leaning your way".to_string(), "good"),
        g if g > 0.025 => ("You lead".to_string(), "good"),
        g if g > -0.025 => (format!("Even with {other}"), "warn"),
        g if g > -0.08 => ("Listening, not leaning".to_string(), "warn"),
        _ => (format!("Behind {other}, for now"), "bad"),
    };
    Interest { words, tone, reason, leader: Some(if gap > 0.025 { j } else { b.0 }) }
}

/// The one thing your staff would say about where he leans.
fn reason(w: &World, j: usize, i: usize, rival: Option<usize>) -> String {
    let p = &w.people[i];
    let wants = desk::family_read(w, i);
    let miles_you = geo::miles(p.home_state, w.programs[j].state);
    let decider = match wants.decider {
        Decider::Himself => "He",
        Decider::Parent => "His parents",
        Decider::Grandparent => "His grandparents",
        Decider::Handler => "The trainer around him",
    };
    let visited = |x: usize, v: Visit| p.cal.visits.iter().any(|e| e.0 == x && e.2 == v);
    if let Some(r) = rival {
        let rn = name(w, r);
        if visited(r, Visit::Official) && !visited(j, Visit::Official) {
            return format!("He took an official visit to {rn}; you haven't hosted him.");
        }
        let miles_them = geo::miles(p.home_state, w.programs[r].state);
        if wants.close_to_home > 0.35 && miles_them + 150.0 < miles_you {
            return format!(
                "{decider} want{} him close to home: {rn} is {:.0} miles away, you {:.0}.",
                if decider == "He" { "s" } else { "" },
                miles_them,
                miles_you
            );
        }
        if wants.close_to_home > 0.35 && miles_you + 150.0 < miles_them {
            return format!("Home is {miles_you:.0} miles from you: being the close school helps.");
        }
        if ncaa::nil_era(w.year) && wants.money > 0.3 {
            let theirs = desk::usual_offer(w, r, p.player.id);
            let ours = w
                .front_office
                .college
                .board
                .iter()
                .find(|t| t.id == p.player.id)
                .and_then(|t| t.nil)
                .unwrap_or_else(|| desk::usual_offer(w, j, p.player.id));
            if theirs > 1.4 * ours + 10_000.0 {
                return format!(
                    "Money matters to {}: {rn}'s collective can pay more.",
                    if decider == "He" { "him".to_string() } else { "the family".to_string() }
                );
            }
            if ours > 1.4 * theirs + 10_000.0 {
                return "Your NIL number is the best he has.".into();
            }
        }
        if wants.playing_time > 0.35 {
            return "He wants to play early: he's counting who is ahead of him.".into();
        }
        if wants.academics > 0.35 {
            return "Academics weigh with the family.".into();
        }
        if w.programs[r].prestige_rel > w.programs[j].prestige_rel + 0.1 && wants.prestige > 0.25 {
            return format!("{rn}'s name and its titles pull at him.");
        }
        if w.programs[j].prestige_rel > w.programs[r].prestige_rel + 0.1 {
            return "Your program's name carries weight with him.".into();
        }
    }
    if p.childhood_team == Some(j) {
        return "He grew up cheering for you.".into();
    }
    if visited(j, Visit::Official) {
        return "He's seen your campus on an official visit.".into();
    }
    if miles_you <= 250.0 {
        return format!("He lives {miles_you:.0} miles away: you can be around him often.");
    }
    "Your staff is still learning what matters to him.".into()
}

/// His recruitment's schools for the file: best first, each with what
/// happened between them.
fn schools(w: &World, j: usize, i: usize) -> Vec<Value> {
    let p = &w.people[i];
    let race = desk::race_at(w, i);
    // Who leads, or the top two neck and neck (inside a few hundredths).
    let close = race.len() > 1 && race[1].1 > race[0].1 - 0.025;
    // His top six, and your program where it stands when it is further down.
    let yours_at = race.iter().position(|r| r.0 == j).filter(|&k| k >= 6);
    race.iter()
        .enumerate()
        .filter(|&(n, _)| n < 6 || Some(n) == yours_at)
        .map(|(n, &(x, _))| {
            let mut bits: Vec<String> = Vec::new();
            if p.cal.signed.is_some_and(|s| s.program == x) {
                bits.push("Signed".into());
            } else if let Some(c) = p.cal.commit.filter(|c| c.0 == x) {
                bits.push(format!("Committed {}", short_date(c.1)));
            } else {
                bits.push("Offer".into());
            }
            if let Some(v) = p.cal.visits.iter().find(|v| v.0 == x && v.2 == Visit::Official) {
                bits.push(format!("official visit {}", short_date(v.1)));
            } else if x == j
                && let Some(v) =
                    w.front_office.college.visits.iter().find(|v| v.id == p.player.id && v.state == VisitState::Set)
            {
                bits.push(format!("{} set {}", v.kind.name().to_lowercase(), short_date(v.date)));
            } else if let Some(v) = p.cal.visits.iter().find(|v| v.0 == x && v.2 == Visit::Unofficial) {
                bits.push(format!("unofficial in {}", month(v.1)));
            } else if p.cal.visits.iter().any(|v| v.0 == x && v.2 == Visit::Home) {
                bits.push("home visit".into());
            } else if p.cal.early_offers.iter().any(|o| o.0 == x) {
                bits.push("offered early".into());
            }
            let pill = match n {
                0 | 1 if close => "Neck and neck",
                0 if race.len() > 1 => "Leads",
                _ => "",
            };
            json!({ "n": n + 1, "team": brief(w, x), "status": bits.join(" · "), "pill": pill, "yours": x == j })
        })
        .collect()
}

fn visits_of(w: &World, j: usize, p: &Person) -> Vec<Value> {
    let mut v: Vec<(Date, Value)> = Vec::new();
    for e in p.cal.visits.iter().filter(|e| e.0 == j) {
        let kind = match e.2 {
            Visit::Official => "Official visit",
            Visit::Unofficial => "Unofficial visit",
            Visit::Home => "Home visit",
            Visit::DropIn => "Drop-in at practice",
        };
        v.push((e.1, json!({ "date": date(e.1), "d": e.1.0, "kind": kind, "state": "Done", "note": "" })));
    }
    for x in w.front_office.college.visits.iter().filter(|x| x.id == p.player.id && x.state != VisitState::Done) {
        let state = match x.state {
            VisitState::Set => "Set",
            VisitState::Declined => "Declined",
            VisitState::Off => "Off",
            VisitState::Done => "Done",
        };
        v.push((x.date, json!({ "date": date(x.date), "d": x.date.0, "kind": x.kind.name(), "key": x.kind.key(), "state": state, "note": x.note })));
    }
    v.sort_by_key(|x| x.0);
    v.into_iter().map(|x| x.1).collect()
}

fn next_step(w: &World, p: &Person) -> Option<(Date, String)> {
    w.front_office
        .college
        .visits
        .iter()
        .filter(|v| v.id == p.player.id && v.state == VisitState::Set && v.date > w.today)
        .min_by_key(|v| v.date)
        .map(|v| (v.date, v.kind.name().to_string()))
}

/// One recruit as a board row.
#[allow(clippy::too_many_arguments)]
fn recruit_row(
    w: &World,
    j: usize,
    i: usize,
    ranks: &Ranks,
    t: Option<&desk::Target>,
    place: Option<usize>,
    committable: Option<bool>,
) -> Value {
    let p = &w.people[i];
    let offered = t.is_some_and(|t| t.offered.is_some() && t.pulled.is_none());
    let (read, pot, yours) = staff_read(w, p);
    let it = interest(w, j, i, offered);
    let s = desk::standing(p, j);
    let hs = p.hs;
    let usual = if ncaa::nil_era(w.year) { desk::usual_offer(w, j, p.player.id) } else { 0.0 };
    let next = next_step(w, p);
    let offers = views::offers_of(p).len();
    json!({
        "id": p.player.id,
        "name": p.name,
        "pos": p.player.position.name(),
        "cls": views::class_text(w, p),
        "class_of": p.class_of,
        "juco": p.stage == Stage::College,
        "stars": stars_of(p),
        "camp": camp_stars(p),
        "rank": ranks.text(w, p),
        "nat": ranks.overall(p),
        "hs": if p.stage == Stage::College { p.college.map(|c| brief(w, c)) } else { Some(hs_brief(w, hs)) },
        "hs_colors": colors::high_school(hs),
        "town": views::hometown(w, p),
        "ht": afm_engine::body::height_text(afm_engine::body::body_now(&p.player).height),
        "wt": afm_engine::body::body_now(&p.player).weight.round(),
        "read": read,
        "yours": yours,
        // Best for us beside best player (SK64): the projected path's score.
        "for_us": views::person_index(w, p.player.id).and_then(|k| crate::lane_paths::path(w, k)).map(|v| v["for_us"].clone()),
        "pot": pot,
        "standing": standing_key(s),
        "interest": it.words,
        "tone": it.tone,
        "reason": it.reason,
        "leader": it.leader.map(|x| brief(w, x)),
        "offers": offers,
        "offered": offered,
        "offered_on": t.and_then(|t| t.offered).map(date),
        "pulled": t.and_then(|t| t.pulled).map(date),
        "by": t.map(|t| source_word(t.by)),
        "why": t.and_then(|t| t.why.clone()),
        "priority": t.is_some_and(|t| t.priority),
        "nil": t.and_then(|t| t.nil),
        "usual": usual,
        "qualifies": desk::qualifies(w, j, i),
        "committable": committable,
        "place": place.map(|k| k + 1),
        "next": next.map(|n| json!({ "date": short_date(n.0), "what": n.1 })),
        "on_board": t.is_some(),
        "reader": reader_name(w, j, p.player.position),
    })
}

// --- The board --------------------------------------------------------------------------

fn class_plan(w: &World, j: usize, rows: &[(usize, &desk::Target)]) -> Value {
    let o = desk::outlook(w, j);
    let mut by_pos: Vec<Value> = Vec::new();
    let mut commits = 0;
    let mut signed = 0;
    for pos in desk::BOARD_ORDER {
        let k = pos.index();
        let mut c = 0;
        let mut s = 0;
        let mut off = 0;
        for &(i, t) in rows {
            let p = &w.people[i];
            if p.player.position != pos || p.class_of != o.sign_year {
                continue;
            }
            match desk::standing(p, j) {
                Standing::CommittedHere => c += 1,
                Standing::SignedHere => s += 1,
                _ if t.offered.is_some() && t.pulled.is_none() => off += 1,
                _ => {}
            }
        }
        commits += c;
        signed += s;
        let need = o.need[k];
        by_pos.push(json!({ "pos": pos.name(), "need": need, "committed": c, "signed": s, "offers": off, "open": need.saturating_sub(c + s) }));
    }
    let (rank, of) = class_rank(w, j, o.sign_year);
    json!({
        "year": o.sign_year,
        "rank": rank,
        "rank_of": of,
        "slots": o.slots,
        "room": o.room,
        "limit": o.limit,
        "signees": o.signees,
        "whole_roster": o.whole_roster,
        "commits": commits,
        "signed": signed,
        "by_pos": by_pos,
    })
}

/// The services' running team ranking of a class: commitments and
/// signees scored the way the composite services do (a star's rough
/// rating, the best counting most), among your division's programs.
fn class_rank(w: &World, j: usize, sign_year: i32) -> (Option<u32>, usize) {
    let (rank, of, _, _) = class_ranks(w, j, sign_year);
    (rank, of)
}

/// The same ranking nationally and in your conference: (national rank,
/// programs in your division, conference rank, programs in your
/// conference).
fn class_ranks(w: &World, j: usize, sign_year: i32) -> (Option<u32>, usize, Option<u32>, usize) {
    let div = w.programs[j].division;
    let mut by: HashMap<usize, Vec<f64>> = HashMap::new();
    for p in w.people.iter() {
        if p.class_of != sign_year || p.player.position.is_specialist() {
            continue;
        }
        let at = match (p.cal.signed, p.cal.commit) {
            (Some(s), _) if !s.walk_on => s.program,
            (None, Some(c)) if !c.2 => c.0,
            _ => continue,
        };
        if w.programs[at].division != div {
            continue;
        }
        let r = match p.stars {
            5 => 99.0,
            4 => 92.0,
            3 => 85.0,
            2 => 78.0,
            _ => 70.0,
        };
        by.entry(at).or_default().push(r);
    }
    let mut scores: Vec<(f64, usize)> = by
        .into_iter()
        .map(|(x, mut v)| {
            v.sort_by(|a, b| b.total_cmp(a));
            (v.iter().enumerate().map(|(n, r)| r * (-((n as f64).powi(2)) / 200.0).exp()).sum(), x)
        })
        .collect();
    scores.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let of = w.programs.iter().filter(|p| p.division == div).count();
    let conf = &w.programs[j].conference;
    let in_conf: Vec<usize> = scores.iter().map(|s| s.1).filter(|&x| w.programs[x].conference == *conf).collect();
    let conf_of = w.programs.iter().filter(|p| p.conference == *conf && p.plays(w.year)).count();
    (
        scores.iter().position(|s| s.1 == j).map(|k| k as u32 + 1),
        of,
        in_conf.iter().position(|&x| x == j).map(|k| k as u32 + 1),
        conf_of,
    )
}

fn duties(w: &World, j: usize) -> Value {
    let who = desk::coordinator_name(w, j);
    let list: Vec<Value> = [Duty::Board, Duty::Visits, Duty::Portal, Duty::Nil]
        .into_iter()
        .map(|d| {
            json!({
                "key": d.key(),
                "name": d.name(),
                "yours": w.front_office.owner(d) == Owner::You,
                "staff": staff_line(&who, d),
            })
        })
        .collect();
    json!({ "list": list, "fill": w.front_office.fill_board, "coordinator": who.trim_start_matches("Your recruiting coordinator, ").trim_end_matches(',') })
}

fn calendar(w: &World) -> Value {
    let (period, rule) = desk::period_today(w);
    let dates: Vec<Value> = desk::key_dates(w)
        .into_iter()
        .take(14)
        .map(|(d, t)| json!({ "date": short_date(d), "full": date(d), "in": w.today.days_to(d), "text": t }))
        .collect();
    let next = desk::next_signing(w);
    json!({
        "today": date(w.today),
        "period": period,
        "rule": rule,
        "dates": dates,
        "signing": next.map(|(d, n)| json!({ "date": short_date(d), "full": date(d), "in": w.today.days_to(d), "name": n })),
    })
}

fn board(w: &World, j: usize) -> Reply {
    let sign_year = desk::next_class(w);
    let ranks = Ranks::new(w, &[sign_year, sign_year + 1, sign_year + 2]);
    let d = &w.front_office.college;
    let ids: Vec<u32> = d.board.iter().map(|t| t.id).collect();
    let at = index_map(w, &ids);
    let mut list: Vec<(usize, &desk::Target)> = Vec::new();
    for t in &d.board {
        if let Some(&i) = at.get(&t.id) {
            list.push((i, t));
        }
    }
    // Committable: commitments first, then your order, up to the class's places.
    let o = desk::outlook(w, j);
    let mut order: Vec<usize> = (0..list.len())
        .filter(|&k| {
            list[k].1.offered.is_some() && list[k].1.pulled.is_none() && w.people[list[k].0].class_of == o.sign_year
        })
        .collect();
    order.sort_by_key(|&k| desk::standing(&w.people[list[k].0], j) != Standing::CommittedHere);
    // A yes needs a place in the class and one at his position; a
    // commitment you hold keeps his.
    let mut committable: HashMap<usize, bool> = HashMap::new();
    let mut n = 0;
    let mut at_pos = vec![0usize; o.need.len()];
    for k in order {
        let s = desk::standing(&w.people[list[k].0], j);
        if matches!(s, Standing::CommittedHere | Standing::Open) {
            let pos = w.people[list[k].0].player.position.index();
            let fits = s == Standing::CommittedHere || (n < o.slots && at_pos[pos] < o.need[pos]);
            committable.insert(k, fits);
            n += 1;
            at_pos[pos] += 1;
        }
    }
    let rows: Vec<Value> = list
        .iter()
        .enumerate()
        .map(|(k, &(i, t))| recruit_row(w, j, i, &ranks, Some(t), Some(k), committable.get(&k).copied()))
        .collect();
    let plan = class_plan(w, j, &list);
    let homes: Vec<Value> = home_weekends(w, j);
    Ok(json!({
        "team": brief(w, j),
        "conference": w.programs[j].conference,
        "division": w.programs[j].division.name(),
        "class": plan,
        "calendar": calendar(w),
        "duties": duties(w, j),
        "rows": rows,
        "homes": homes,
        "nil_era": ncaa::nil_era(w.year),
        "nil_yours": w.front_office.owner(Duty::Nil) == Owner::You,
        "budget": budget_json(w, j),
        "classes": [sign_year, sign_year + 1, sign_year + 2],
    }))
}

// --- The map (screens topic 7) -----------------------------------------------------------

/// You give a state to an assistant's area, or back to the staff's split.
fn set_area(game: &mut Game, args: &Value) -> Reply {
    let w = game.world.as_mut().ok_or("No world is loaded.")?;
    let j = program(w)?;
    let code = args.get("state").and_then(Value::as_str).unwrap_or("");
    let st = w.states.iter().position(|s| s.code == code).ok_or("No such state.")? as u8;
    let coach = args.get("coach").and_then(Value::as_u64).map(|c| c as usize);
    if coach.is_some_and(|c| !area_staff(w, j).contains(&c)) {
        return Err("He isn't one of your on-field assistants.".into());
    }
    afm_engine::world::scouting::dept::set_area(w, st, coach);
    map(w, j)
}

/// Your on-field assistants, who recruit an area each.
fn area_staff(w: &World, j: usize) -> Vec<usize> {
    let mut v: Vec<usize> = w.programs[j].position_coaches.iter().flatten().copied().collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// The country for this class (screens topic 7; board 7 of update 2): your
/// board, its commits and offers, who leans elsewhere, every prospect at
/// your level, your conference rivals' commits, and the assistant whose
/// area each state is, by state; and on the real map (latitude and
/// longitude: the screen draws it) your board at each prospect's hometown,
/// the talent's metro areas, the rivals' campuses and your class so far.
fn map(w: &World, j: usize) -> Reply {
    let o = desk::outlook(w, j);
    let year = o.sign_year;
    let n = w.states.len();
    let pr = &w.programs[j];
    let campus = geo::campus(pr.state, &pr.city);
    let floor = level_floor(pr.division);
    let mut board = vec![0u32; n];
    let mut commits = vec![0u32; n];
    let mut offers = vec![0u32; n];
    let mut leaning = vec![0u32; n];
    let mut all = vec![0u32; n];
    let mut rivals = vec![0u32; n];
    let mut top: Vec<Vec<Value>> = vec![Vec::new(); n];
    let mut dots: Vec<Value> = Vec::new();
    let d = &w.front_office.college;
    let ids: Vec<u32> = d.board.iter().map(|t| t.id).collect();
    let at = index_map(w, &ids);
    let home_of = |p: &Person| -> (usize, &str) {
        match w.high_schools.get(p.hs) {
            Some(h) => (h.state.min(n - 1), h.town.as_str()),
            None => (p.home_state.min(n - 1), ""),
        }
    };
    for t in &d.board {
        let Some(&i) = at.get(&t.id) else { continue };
        let p = &w.people[i];
        if p.class_of != year {
            continue;
        }
        let s = p.home_state.min(n - 1);
        board[s] += 1;
        let offered = t.offered.is_some() && t.pulled.is_none();
        let st = desk::standing(p, j);
        let here = matches!(st, Standing::CommittedHere | Standing::SignedHere | Standing::Enrolled);
        if here {
            commits[s] += 1;
        }
        if offered {
            offers[s] += 1;
        }
        let it = interest(w, j, i, offered);
        if it.tone == "bad" {
            leaning[s] += 1;
        }
        let kind = if here {
            "c"
        } else if it.tone == "bad" {
            "b"
        } else if offered {
            "g"
        } else {
            "n"
        };
        let (hs, town) = home_of(p);
        let place = geo::place(hs, town);
        dots.push(json!({
            "id": p.player.id, "name": p.name, "k": kind, "at": [round3(place.0), round3(place.1)],
            "mi": geo::miles_between(campus, place).round(), "st": w.states[s].code,
        }));
        if top[s].len() < 6 {
            top[s].push(json!({
                "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "stars": stars_of(p),
                "town": views::hometown(w, p), "words": it.words, "tone": it.tone, "k": kind,
            }));
        }
    }
    // Every prospect at your level, by state and by metro area; your class
    // so far; your conference rivals' commits.
    let conf = &pr.conference;
    let mut towns: HashMap<(usize, &str), u32> = HashMap::new();
    let mut rival_towns: HashMap<(usize, &str), u32> = HashMap::new();
    let mut class: Vec<u32> = Vec::new();
    let mut stars_in: Vec<u8> = Vec::new();
    // (The plan's open slots already leave out the signees.)
    let mut signed_n = 0usize;
    for p in &w.people {
        if p.class_of != year || !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool | Stage::College) {
            continue;
        }
        if p.stage == Stage::College && p.cal.signed.is_none() {
            continue;
        }
        let s = p.home_state.min(n - 1);
        if stars_of(p) >= floor || (floor <= 1 && !views::offers_of(p).is_empty()) {
            all[s] += 1;
            *towns.entry(home_of(p)).or_default() += 1;
        }
        let to = match (p.cal.signed, p.cal.commit) {
            (Some(x), _) if !x.walk_on => Some(x.program),
            (None, Some(c)) if !c.2 => Some(c.0),
            _ => None,
        };
        if to == Some(j) {
            class.push(p.player.id);
            stars_in.push(stars_of(p));
            signed_n += usize::from(p.cal.signed.is_some());
        }
        if let Some(c) = to
            && c != j
            && w.programs[c].conference == *conf
        {
            rivals[s] += 1;
            *rival_towns.entry(home_of(p)).or_default() += 1;
        }
    }
    // How much of each lies within 250 and 500 miles of campus.
    let near = |towns: &HashMap<(usize, &str), u32>| -> [u32; 2] {
        let mut v = [0u32; 2];
        for (&(s, town), &c) in towns {
            let mi = geo::miles_between(campus, geo::place(s, town));
            if mi <= 250.0 {
                v[0] += c;
            }
            if mi <= 500.0 {
                v[1] += c;
            }
        }
        v
    };
    let all_near = near(&towns);
    let rivals_near = near(&rival_towns);
    let staff = area_staff(w, j);
    let coach_json = |c: usize| {
        let g = w.programs[j].position_coaches.iter().position(|x| *x == Some(c));
        json!({ "id": c, "name": w.coaches[c].name, "job": g.map_or("Head coach".to_string(), |g| format!("{} coach", afm_engine::world::coach::Group::ALL[g].name())) })
    };
    let set: std::collections::HashSet<u8> = w.scouting.dept.areas.iter().flat_map(|a| a.1.iter().copied()).collect();
    let states: Vec<Value> = (0..n)
        .map(|s| {
            let c = afm_engine::world::scouting::dept::area_coach(w, j, s as u8);
            let code = &w.states[s].code;
            // The state's biggest town, and how far it is from campus.
            let main = afm_engine::world::high_school::state_towns(code).first().map(|t| {
                let mi = geo::miles_between(campus, (t.lat as f64, t.lon as f64)).round();
                json!({ "town": t.town, "mi": mi })
            });
            json!({
                "code": code, "board": board[s], "commits": commits[s], "offers": offers[s],
                "leaning": leaning[s], "all": all[s], "rivals": rivals[s],
                "coach": c.map(coach_json), "set": set.contains(&(s as u8)), "top": top[s], "main": main,
            })
        })
        .collect();
    let areas: Vec<Value> = staff
        .iter()
        .map(|&c| {
            let mut v = coach_json(c);
            v["states"] = json!(
                (0..n)
                    .filter(|&s| afm_engine::world::scouting::dept::area_coach(w, j, s as u8) == Some(c))
                    .map(|s| w.states[s].code.clone())
                    .collect::<Vec<_>>()
            );
            v
        })
        .collect();
    let metros: Vec<Value> = metro_areas(w, towns)
        .into_iter()
        .map(|(name, at, v)| json!({ "name": name, "at": [round3(at.0), round3(at.1)], "v": v }))
        .collect();
    let campuses: Vec<Value> = w
        .programs
        .iter()
        .enumerate()
        .filter(|(k, x)| *k != j && x.conference == *conf && x.plays(w.year))
        .map(|(k, x)| {
            let at = geo::campus(x.state, &x.city);
            json!({ "id": k, "name": w.program_name(k), "at": [round3(at.0), round3(at.1)] })
        })
        .collect();
    // Your class: commits against the places, by stars at your level, its
    // public rank nationally and in your conference, and its average
    // composite rating.
    let top_band = floor + 1;
    let split = [
        stars_in.iter().filter(|&&s| s >= top_band).count(),
        stars_in.iter().filter(|&&s| s == floor).count(),
        stars_in.iter().filter(|&&s| s < floor).count(),
    ];
    let (rank, of, conf_rank, conf_of) = class_ranks(w, j, year);
    let comp = afm_engine::world::scouting::services::class_ranks(w, year)
        .map(|c| afm_engine::world::scouting::services::composite(w, c))
        .unwrap_or_default();
    let rated: Vec<f64> = class.iter().filter_map(|id| comp.get(id).map(|c| c.1)).collect();
    let average = (!rated.is_empty()).then(|| (rated.iter().sum::<f64>() / rated.len() as f64 * 1000.0).round() / 10.0);
    let next = desk::next_signing(w);
    Ok(json!({
        "year": year,
        "home": w.states[pr.state].code,
        "campus": { "name": w.program_name(j), "city": pr.city, "at": [round3(campus.0), round3(campus.1)] },
        "level": pr.division.name(),
        "floor": floor,
        "conference": w.conference_name(conf),
        "rivals_n": w.programs.iter().filter(|p| p.conference == *conf && p.plays(w.year)).count().saturating_sub(1),
        "states": states,
        "staff": areas,
        "dots": dots,
        "metros": metros,
        "campuses": campuses,
        "all_near": all_near,
        "rivals_near": rivals_near,
        "class": {
            "commits": class.len(), "slots": o.slots + signed_n, "open": o.slots.saturating_sub(class.len() - signed_n),
            "split": [
                { "k": format!("{top_band}+ stars"), "v": split[0] },
                { "k": format!("{floor} star{}", if floor == 1 { "" } else { "s" }), "v": split[1] },
                { "k": match floor { 0 | 1 => "Unrated".to_string(), 2 => "1 star or none".to_string(), f => format!("{} or fewer", f - 1) }, "v": split[2] },
            ],
            "rank": rank, "rank_of": of, "conf_rank": conf_rank, "conf_of": conf_of, "average": average,
            "signing": next.map(|(d, n)| json!({ "date": short_date(d), "name": n })),
        },
    }))
}

/// The stars a staff at each level counts as its talent: three and up for
/// the FBS, two and up for the FCS, any star or any offer below.
fn level_floor(d: afm_engine::world::data::Division) -> u8 {
    use afm_engine::world::data::Division;
    match d {
        Division::FBS => 3,
        Division::FCS => 2,
        _ => 1,
    }
}

fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

/// Prospects' hometowns gathered into metro areas: the biggest town takes
/// in every town within 35 miles. The 60 biggest areas, by prospects.
fn metro_areas(w: &World, towns: HashMap<(usize, &str), u32>) -> Vec<(String, (f64, f64), u32)> {
    let mut list: Vec<((usize, &str), u32)> = towns.into_iter().collect();
    list.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut areas: Vec<(String, (f64, f64), u32)> = Vec::new();
    for ((s, town), c) in list {
        let at = geo::place(s, town);
        if let Some(m) = areas.iter_mut().find(|m| geo::miles_between(m.1, at) <= 35.0) {
            m.2 += c;
        } else if areas.len() < 400 {
            let code = w.states.get(s).map_or("", |x| x.code.as_str());
            let name = if town.is_empty() { code.to_string() } else { format!("{town}, {code}") };
            areas.push((name, at, c));
        }
    }
    areas.sort_by_key(|a| std::cmp::Reverse(a.2));
    areas.truncate(60);
    areas
}

fn home_weekends(w: &World, j: usize) -> Vec<Value> {
    let set: Vec<(Date, u32)> = w
        .front_office
        .college
        .visits
        .iter()
        .filter(|v| v.state == VisitState::Set && v.kind == VisitKind::Official)
        .map(|v| (v.date.add(1), v.id))
        .collect();
    desk::home_games(w, j)
        .into_iter()
        .filter(|(d, _)| *d > w.today)
        .map(|(d, opp)| {
            let n = set.iter().filter(|s| s.0 == d).count();
            json!({ "date": short_date(d), "opp": brief(w, opp), "officials": n })
        })
        .collect()
}

fn index_map(w: &World, ids: &[u32]) -> HashMap<u32, usize> {
    let want: std::collections::HashSet<u32> = ids.iter().copied().collect();
    w.people.iter().enumerate().filter(|(_, p)| want.contains(&p.player.id)).map(|(i, p)| (p.player.id, i)).collect()
}

// --- One recruit's file -----------------------------------------------------------------

fn file(w: &World, j: usize, args: &Value) -> Reply {
    let id = id_arg(args)?;
    let i = views::person_index(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    let ranks = Ranks::new(w, &[p.class_of]);
    let t = w.front_office.college.board.iter().find(|t| t.id == id);
    let mut v = recruit_row(w, j, i, &ranks, t, w.front_office.college.board.iter().position(|t| t.id == id), None);
    let (_, pot, _) = staff_read(w, p);
    v["pot"] = pot;
    v["schools"] = json!(schools(w, j, i));
    v["visits"] = json!(visits_of(w, j, p));
    v["reader_role"] = json!(reader_role(w, j, p.player.position));
    // What the rules allow now, by kind, with the next days or why not.
    let options: Vec<Value> = VisitKind::ALL
        .into_iter()
        .map(|k| {
            let days = desk::visit_dates(w, id, k);
            let why = if days.is_empty() {
                // The reason on the first day it would otherwise have been possible.
                let mut why = String::new();
                for n in 1..=180 {
                    if let Some(r) = desk::visit_rule(w, i, k, w.today.add(n)) {
                        if why.is_empty() || r.contains("used") || r.contains("One ") || r.contains("signed") {
                            why = r;
                        }
                        if why.contains("used") || why.contains("One ") || why.contains("signed") {
                            break;
                        }
                    }
                }
                why
            } else {
                String::new()
            };
            let set = w.front_office.college.visits.iter().find(|x| x.id == id && x.kind == k && x.state == VisitState::Set && x.date >= w.today);
            json!({
                "key": k.key(),
                "name": k.name(),
                "set": set.map(|x| json!({ "d": x.date.0, "date": date(x.date), "note": x.note })),
                "days": days.iter().map(|(d, w2)| json!({ "d": d.0, "date": short_date(*d), "word": w2 })).collect::<Vec<_>>(),
                "why": why,
            })
        })
        .collect();
    v["options"] = json!(options);
    let offers: Vec<Value> = {
        let mut early = p.cal.early_offers.clone();
        early.sort_by_key(|o| o.1);
        early.iter().map(|o| json!({ "team": brief(w, o.0), "date": short_date(o.1) })).collect()
    };
    v["early"] = json!(offers);
    let fam = desk::family_read(w, i);
    v["gpa"] = json!(format!("{:.2}", fam.gpa));
    v["test"] = json!(p.cal.test_score);
    v["decides"] = json!(match fam.decider {
        Decider::Himself => "He decides for himself.",
        Decider::Parent => "A parent has the final say.",
        Decider::Grandparent => "His grandparents raised him; they decide with him.",
        Decider::Handler => "A trainer close to the family steers it.",
    });
    v["schools_n"] = json!(desk::race_at(w, i).len());
    v["officials_used"] = json!(p.cal.visits.iter().filter(|e| e.2 == Visit::Official).count());
    v["miles"] = json!(geo::miles(p.home_state, w.programs[j].state).round());
    v["followers"] = json!(p.followers);
    v["camera"] = json!(afm_engine::world::fame::aura_read(
        p,
        views::eye_for(w, TeamRef::College(j), p.player.position),
        views::reader_key(w)
    ));
    v["nil_era"] = json!(ncaa::nil_era(w.year));
    v["nil_yours"] = json!(w.front_office.owner(Duty::Nil) == Owner::You);
    v["board_yours"] = json!(w.front_office.owner(Duty::Board) == Owner::You);
    v["visits_yours"] = json!(w.front_office.owner(Duty::Visits) == Owner::You);
    v["nil_max"] = json!(ncaa::position_nil_max(p.player.position));
    v["budget"] = budget_json(w, j);
    v["high_school"] = json!(w.school_name(p.hs));
    // Your own film session (bug 82): its hours, or why not today.
    v["film_now"] = afm_engine::world::scouting::own::film_cost(w, p.player.id).ok().map_or(Value::Null, |(_, h, _)| {
        json!({ "hours": h, "left": afm_engine::world::scouting::own::film_left(w), "blocked": afm_engine::world::scouting::own::film_blocked(w, p.player.id) })
    });
    Ok(v)
}

// --- Visits -----------------------------------------------------------------------------

fn visits(w: &World, j: usize) -> Reply {
    let d = &w.front_office.college;
    let ids: Vec<u32> = d.visits.iter().map(|v| v.id).collect();
    let at = index_map(w, &ids);
    let mut list: Vec<&desk::Plan> = d.visits.iter().collect();
    list.sort_by_key(|v| v.date);
    let row = |v: &desk::Plan| -> Option<Value> {
        let i = *at.get(&v.id)?;
        let p = &w.people[i];
        let state = match v.state {
            VisitState::Set => "Set",
            VisitState::Done => "Done",
            VisitState::Declined => "Declined",
            VisitState::Off => "Off",
        };
        Some(json!({
            "id": v.id, "name": p.name, "pos": p.player.position.name(), "stars": stars_of(p),
            "cls": views::class_text(w, p), "town": views::hometown(w, p),
            "kind": v.kind.name(), "key": v.kind.key(), "date": short_date(v.date), "d": v.date.0,
            "in": w.today.days_to(v.date), "state": state, "note": v.note,
            "interest": interest(w, j, i, w.front_office.college.board.iter().any(|t| t.id == v.id && t.offered.is_some() && t.pulled.is_none())).words,
        }))
    };
    let ahead: Vec<Value> =
        list.iter().filter(|v| v.state == VisitState::Set && v.date > w.today).filter_map(|v| row(v)).collect();
    let past: Vec<Value> = list
        .iter()
        .rev()
        .filter(|v| !(v.state == VisitState::Set && v.date > w.today))
        .take(80)
        .filter_map(|v| row(v))
        .collect();
    // Your program's visits the world recorded (your staff's, before you).
    let mut taken = 0;
    let mut officials = 0;
    let class = desk::next_class(w);
    for p in w.people.iter() {
        if matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) && p.class_of == class {
            for e in p.cal.visits.iter().filter(|e| e.0 == j) {
                taken += 1;
                if e.2 == Visit::Official {
                    officials += 1;
                }
            }
        }
    }
    // The NCAA calendar ahead, period by period (16 weeks).
    let y = &w.dates;
    let mut weeks: Vec<Value> = Vec::new();
    let mut day = w.today;
    while day.weekday() != afm_engine::calendar::Weekday::Mon {
        day = day.add(-1);
    }
    for _ in 0..16 {
        let start = day;
        let mut days = Vec::new();
        for k in 0..7 {
            let dd = start.add(k);
            if dd > y.super_bowl {
                break;
            }
            days.push(json!({ "d": dd.0, "wd": dd.weekday().short(), "day": dd.parts().2, "period": afm_engine::calendar::recruiting_period(y, dd).name(), "past": dd < w.today, "today": dd == w.today }));
        }
        if days.is_empty() {
            break;
        }
        weeks.push(json!({ "start": short_date(start), "days": days }));
        day = start.add(7);
    }
    Ok(json!({
        "team": brief(w, j),
        "ahead": ahead,
        "past": past,
        "homes": home_weekends(w, j),
        "calendar": calendar(w),
        "weeks": weeks,
        "class": class,
        "taken": taken,
        "officials": officials,
        "duties": duties(w, j),
        "junior_officials": w.changes.rules(y.season).junior_officials,
    }))
}

// --- The portal -------------------------------------------------------------------------

fn eligibility_left(w: &World, p: &Person) -> u32 {
    let limit: u32 = if w.changes.rules(w.year).five_in_five { 5 } else { 4 };
    limit.saturating_sub(p.college_seasons)
}

/// The seasons a transfer could play from next season on: the season in
/// play counts until January 2 settles it, next season's rules apply, and
/// the five-year clock from high school runs out too. The windows from
/// December to January follow the season just played; the spring and
/// graduate windows come after the football year turned.
fn seasons_ahead(w: &World, p: &Person) -> u32 {
    let m = w.today.month();
    let next = if m == 12 || m <= 1 { w.year + 1 } else { w.year };
    let limit: u32 = if w.changes.rules(next).five_in_five { 5 } else { 4 };
    let used = p.college_seasons + p.cal.count_pending as u32;
    let clock = (p.class_of + 5 - next).max(0) as u32;
    limit.saturating_sub(used).min(clock)
}

fn portal(w: &World, j: usize, args: &Value) -> Reply {
    let pos = args.get("pos").and_then(Value::as_str).filter(|s| *s != "ALL");
    let div = args.get("division").and_then(Value::as_str).filter(|s| *s != "ALL");
    let today = w.today;
    let d = &w.front_office.college;
    let one_time = w.changes.rules(w.year + 1).one_time_transfer;
    let mine_div = w.programs[j].division;
    let mut entrants: Vec<(f64, Value)> = Vec::new();
    let mut in_portal = 0;
    for p in w.people.iter() {
        let Some(e) = p.cal.portal else { continue };
        if p.stage != Stage::College || e.held && e.entered.0 > today.0 + 3650 {
            continue;
        }
        let Some(c) = p.college else { continue };
        if c == j {
            continue;
        }
        in_portal += 1;
        if pos.is_some_and(|x| x != p.player.position.name()) {
            continue;
        }
        if div.is_some_and(|x| x != w.programs[c].division.name()) {
            continue;
        }
        let t = d.portal.iter().find(|t| t.id == p.player.id && !t.settled);
        let sits = !one_time
            && mine_div == afm_engine::world::data::Division::FBS
            && w.programs[c].division != afm_engine::world::data::Division::JUCO
            && !p.cal.graduated;
        let read = views::person_index(w, p.player.id).map_or(reads::Read::NotSeen, |k| reads::overall(w, k));
        let key = p.last_performance.unwrap_or(p.player.overall()) * 0.0
            + stars_of(p) as f64 * 10.0
            + if p.last_role == Some(PlayingTime::Starter) { 30.0 } else { 0.0 };
        entrants.push((
            key,
            json!({
                "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "stars": stars_of(p),
                "from": brief(w, c), "division": w.programs[c].division.name(), "cls": views::class_text(w, p),
                "left": seasons_ahead(w, p), "role": p.last_role.map(views::role_words), "read": read.json(),
                "why": e.why, "entered": short_date(e.entered), "decide": short_date(e.decide_by),
                "days": today.days_to(e.decide_by).max(0), "held": e.held,
                "offered": t.is_some_and(|t| t.offered.is_some()), "nil": t.and_then(|t| t.nil),
                "contacted": t.is_some_and(|t| t.contacted.is_some()), "sits": sits,
                "usual": if ncaa::nil_era(w.year) { w.programs[j].collective.budget / 40.0 } else { 0.0 },
            }),
        ));
    }
    entrants.sort_by(|a, b| b.0.total_cmp(&a.0));
    let shown = entrants.len();
    let entrants: Vec<Value> = entrants.into_iter().take(400).map(|e| e.1).collect();
    // Your own roster: who might leave, in your staff's words.
    let mut roster: Vec<Value> = Vec::new();
    for &i in &w.programs[j].roster {
        let p = &w.people[i];
        if p.stage != Stage::College || p.college != Some(j) {
            continue;
        }
        let talk = d.talks.iter().find(|t| t.id == p.player.id);
        let lean = p.cal.lean;
        let (risk, tone) = if p.cal.portal.is_some() {
            ("In the portal", "bad")
        } else if lean > 0.25 || p.cal.whispered {
            ("Whispers he may leave", "bad")
        } else if lean > 0.1 {
            ("Restless", "warn")
        } else if lean > 0.03 {
            ("Thinking about his role", "warn")
        } else {
            ("Settled", "")
        };
        // What your staff hears: the strongest pull away this year (the
        // first event is his role at fall camp, the base), and the strongest
        // reason he has to stay.
        let ev = &p.cal.lean_events;
        let push = ev.iter().skip(1).filter(|e| e.2 > 1.0).max_by(|a, b| a.2.total_cmp(&b.2)).map(|e| e.1);
        let hold = ev.iter().skip(1).filter(|e| e.2 < 1.0).min_by(|a, b| a.2.total_cmp(&b.2)).map(|e| e.1);
        let why = push.or_else(|| ev.first().map(|e| e.1)).unwrap_or("");
        roster.push(json!({
            "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "cls": views::class_text(w, p),
            "role": p.last_role.map(views::role_words), "read": reads::overall(w, i).json(), "yours": reads::mine(w, i).json(),
            "risk": risk, "tone": tone, "why": why, "hold": hold, "talk": talk.map(|t| json!({ "date": short_date(t.date), "answer": t.answer })),
            "in_portal": p.cal.portal.is_some(), "nil": p.nil_income, "stars": stars_of(p),
            "rank": if p.cal.portal.is_some() { 0 } else if lean > 0.25 || p.cal.whispered { 1 } else if lean > 0.1 { 2 } else if lean > 0.03 { 3 } else { 4 },
        }));
    }
    // Players elsewhere with whispers about them: a quiet word is possible.
    let mut whispers: Vec<(u8, Value)> = Vec::new();
    for p in w.people.iter() {
        if p.stage != Stage::College || p.cal.portal.is_some() || !p.cal.whispered || p.college == Some(j) {
            continue;
        }
        let Some(c) = p.college else { continue };
        let t = d.portal.iter().find(|t| t.id == p.player.id && !t.settled);
        whispers.push((
            stars_of(p),
            json!({
                "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "from": brief(w, c),
                "division": w.programs[c].division.name(), "cls": views::class_text(w, p), "stars": stars_of(p),
                "role": p.last_role.map(views::role_words), "read": views::person_index(w, p.player.id).map_or(reads::Read::NotSeen, |k| reads::overall(w, k)).json(),
                "contacted": t.and_then(|t| t.contacted).map(short_date), "leaked": t.is_some_and(|t| t.leaked),
            }),
        ));
    }
    whispers.sort_by_key(|x| std::cmp::Reverse(x.0));
    let whispers: Vec<Value> = whispers.into_iter().take(150).map(|x| x.1).collect();
    // Your targets and how they went.
    let ids: Vec<u32> = d.portal.iter().map(|t| t.id).collect();
    let at = index_map(w, &ids);
    let targets: Vec<Value> = d
        .portal
        .iter()
        .filter_map(|t| {
            let i = *at.get(&t.id)?;
            let p = &w.people[i];
            let status = if p.cal.portal.is_some() {
                if t.offered.is_some() { "Offered, deciding".to_string() } else { "In the portal".to_string() }
            } else if p.college == Some(j) && t.from != j {
                "Transferred to you".to_string()
            } else if p.college.is_some_and(|c| c != t.from) {
                format!("Chose {}", name(w, p.college.unwrap()))
            } else if p.stage == Stage::Done {
                "Left football".to_string()
            } else if t.contacted.is_some() {
                "Reached out; hasn't entered".to_string()
            } else {
                "Watching".to_string()
            };
            Some(json!({
                "id": t.id, "name": p.name, "pos": p.player.position.name(), "from": brief(w, t.from),
                "status": status, "offered": t.offered.map(short_date), "nil": t.nil,
                "decide": p.cal.portal.map(|e| short_date(e.decide_by)), "settled": t.settled,
            }))
        })
        .collect();
    let windows: Vec<Value> = desk::windows(w)
        .into_iter()
        .map(|(o, c, n)| json!({ "name": n, "opens": short_date(o), "closes": short_date(c), "open": o <= today && today <= c, "in": today.days_to(o) }))
        .collect();
    let rule = w.changes.rules(w.year).transfer;
    use afm_engine::calendar::rules::TransferRule;
    let rule_text = match rule {
        TransferRule::Permission => {
            "Before the portal: a player needs his school's release, and a transfer into FBS sits a year."
        }
        TransferRule::OpenPortal => {
            "The portal is open year round; a transfer into FBS sits a year unless he graduated."
        }
        TransferRule::Windows => "Two windows a year, winter and spring; a coaching change opens its own.",
        TransferRule::January => "One window in January, and a coaching change's own.",
    };
    let sit_text = if one_time {
        "Undergraduate transfers play at once (the one-time exception, 2021)."
    } else {
        "A transfer into FBS sits out a season unless he graduated or comes from a junior college."
    };
    Ok(json!({
        "team": brief(w, j),
        "windows": windows,
        "rule": rule_text,
        "sit": sit_text,
        "entrants": entrants,
        "shown": shown,
        "in_portal": in_portal,
        "roster": roster,
        "whispers": whispers,
        "targets": targets,
        "talks": d.talks.len(),
        "yours": w.front_office.owner(Duty::Portal) == Owner::You,
        "nil_yours": w.front_office.owner(Duty::Nil) == Owner::You,
        "nil_era": ncaa::nil_era(w.year),
        "budget": budget_json(w, j),
        "duties": duties(w, j),
    }))
}

// --- NIL --------------------------------------------------------------------------------

fn budget_json(w: &World, j: usize) -> Value {
    let b = desk::budget(w, j);
    json!({
        "total": b.total, "revenue_share": b.revenue_share, "collective": b.collective,
        "roster": b.roster, "recruits": b.recruits, "portal": b.portal, "left": b.left,
        "total_text": dollars(b.total), "left_text": dollars(b.left),
    })
}

fn nil(w: &World, j: usize) -> Reply {
    let reader = views::reader_key(w);
    let d = &w.front_office.college;
    let year = desk::deals_year(w);
    let usual: HashMap<u32, f64> = desk::usual_deals(w, j).into_iter().collect();
    let deals: HashMap<u32, f64> =
        if d.deals_year == year { d.deals.iter().copied().collect() } else { HashMap::new() };
    let mut rows: Vec<Value> = Vec::new();
    for &i in &w.programs[j].roster {
        let p = &w.people[i];
        if p.stage != Stage::College || p.college != Some(j) {
            continue;
        }
        let eye = views::eye_for(w, TeamRef::College(j), p.player.position);
        rows.push(json!({
            "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "cls": views::class_text(w, p),
            "role": p.last_role.map(views::role_words), "read": reads::overall(w, i).json(), "yours": reads::mine(w, i).json(),
            "usual": usual.get(&p.player.id).copied().unwrap_or(0.0), "deal": deals.get(&p.player.id).copied(),
            "now": p.nil_income, "stars": stars_of(p), "left": eligibility_left(w, p),
            "max": ncaa::position_nil_max(p.player.position) * 2.5,
            // Public: his followers and the brand money he earns himself,
            // apart from the collective (lane 1's fame); his presence on
            // camera only in his position coach's words.
            "followers": p.followers,
            "brand": (p.brand >= 1000.0).then(|| (p.brand as f64 / 1000.0).round() * 1000.0),
            "camera": afm_engine::world::fame::aura_read(p, eye, reader),
        }));
    }
    let ids: Vec<u32> = d.board.iter().filter(|t| t.nil.is_some()).map(|t| t.id).collect();
    let at = index_map(w, &ids);
    let promised: Vec<Value> = d
        .board
        .iter()
        .filter(|t| t.nil.is_some() && t.offered.is_some() && t.pulled.is_none())
        .filter_map(|t| {
            let p = &w.people[*at.get(&t.id)?];
            Some(json!({ "id": t.id, "name": p.name, "pos": p.player.position.name(), "stars": stars_of(p), "nil": t.nil, "standing": standing_key(desk::standing(p, j)) }))
        })
        .collect();
    let ids: Vec<u32> = d.portal.iter().filter(|t| t.nil.is_some()).map(|t| t.id).collect();
    let at = index_map(w, &ids);
    let portal: Vec<Value> = d
        .portal
        .iter()
        .filter(|t| t.nil.is_some() && t.offered.is_some() && !t.settled)
        .filter_map(|t| {
            let p = &w.people[*at.get(&t.id)?];
            Some(json!({ "id": t.id, "name": p.name, "pos": p.player.position.name(), "nil": t.nil, "from": brief(w, t.from) }))
        })
        .collect();
    let prog = &w.programs[j];
    let history: Vec<Value> = prog
        .era
        .ledger
        .iter()
        .rev()
        .take(6)
        .map(|e| json!({ "season": e.season, "budget": e.nil_budget, "boosters": e.boosters }))
        .collect();
    Ok(json!({
        "team": brief(w, j),
        "era": ncaa::nil_era(w.today.year()) || w.today >= Date::ymd(ncaa::NIL_START, 6, 1),
        "house": w.today.year() >= ncaa::HOUSE_START,
        "yours": w.front_office.owner(Duty::Nil) == Owner::You,
        "year": year,
        "pay_day": short_date(Date::ymd(if w.today >= Date::ymd(w.today.year(), 7, 1) { w.today.year() + 1 } else { w.today.year() }, 7, 1)),
        "budget": budget_json(w, j),
        "rows": rows,
        "promised": promised,
        "portal": portal,
        "trend": match prog.collective.trend {
            NilTrend::Growing => "Growing",
            NilTrend::Steady => "Steady",
            NilTrend::Falling => "Falling",
            NilTrend::BoosterLeft => "A booster left",
        },
        "trend_line": match prog.collective.trend {
            NilTrend::Growing => "Boosters are giving more after the wins.",
            NilTrend::Steady => "The booster base gives about what it gave last year.",
            NilTrend::Falling => "Donations are falling after the losing seasons.",
            NilTrend::BoosterLeft => "A big booster walked away; the base carries it now.",
        },
        "big_booster": prog.collective.big_booster.is_some(),
        "history": history,
        "duties": duties(w, j),
    }))
}
