//! The Identity tab (G1-G3, board 19): your guidelines for the
//! coordinators, each with its firmness, what your roster fits, what each
//! coordinator thinks of it, what a change costs and what you can expect
//! on game day, against the season so far game by game.

use afm_engine::calendar::Level as Cal;
use afm_engine::game::guide::{self, BaseFront, Firm, Guidelines, Pace, Want};
use afm_engine::game::playbook::Personnel;
use afm_engine::game::{OffScheme, PlayType};
use afm_engine::world::World;
use afm_engine::world::folk::coach_key;
use afm_engine::world::plans;
use afm_engine::world::systems::{self, FRONTS, Kind, OFFENSES, SHELLS};
use serde_json::{Value, json};

use super::{Seat, abbr, coach_link, head, is_pass, is_snap, lvl, opp_of, pct, season_games, season_words, seat};
use crate::Reply;
use crate::views::{date_text, reader_key, roster_people, team_brief};

const SHELL_NAMES: [&str; 4] = ["Cover 3", "quarters", "man and pressure", "Cover 2"];
const PERSONNEL: [Personnel; 4] = [Personnel::P10, Personnel::P11, Personnel::P12, Personnel::P21];
pub(crate) const FIT_WORDS: [&str; 4] = ["Poor", "Thin", "Good", "Strong"];

/// The offenses you can ask for at your level: the wing-T at high school.
fn offenses(level: Cal) -> Vec<OffScheme> {
    let mut v = OFFENSES.to_vec();
    if level == Cal::HighSchool {
        v.push(OffScheme::WingT);
    }
    v
}

fn fronts(level: Cal) -> Vec<BaseFront> {
    if matches!(level, Cal::Nfl | Cal::Other) {
        vec![BaseFront::FourThree, BaseFront::ThreeFour]
    } else {
        BaseFront::ALL.to_vec()
    }
}

fn scheme_key(s: OffScheme) -> &'static str {
    ["west_coast", "air_raid", "wide_zone", "power_gap", "spread_option", "wing_t"][s as usize]
}

fn scheme_name(s: OffScheme) -> String {
    let n = s.name();
    let mut c = n.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn pace_key(p: Pace) -> &'static str {
    ["huddle", "mixed", "no_huddle"][p as usize]
}

/// Your staff's read of how a system fits the roster, in points a game
/// against the average of its kind (`systems::roster_fit`), blurred by the
/// staff's eye the way every read is (a fixed lean per reader and system).
pub(crate) fn fit_reads(w: &World, s: &Seat) -> [f64; 11] {
    let roster = roster_people(w, s.tr);
    let v = systems::roster_fit(w, &roster);
    let reader = reader_key(w);
    let mut out = [0.0; 11];
    for kind in [Kind::Offense, Kind::Front, Kind::Shell] {
        let r = kind.range();
        let mean = v[r.clone()].iter().sum::<f64>() / r.len() as f64;
        for slot in r {
            let rel = v[slot] - mean;
            let read = match crate::reads::range(50.0 + 10.0 * rel, 4.0, reader, s.t as u32 ^ 0x5F17, 200 + slot) {
                crate::reads::Read::Range(lo, hi) => (lo as f64 + hi as f64) / 2.0,
                crate::reads::Read::NotSeen => 50.0,
            };
            out[slot] = (read - 50.0) / 10.0;
        }
    }
    out
}

/// A fit read in words: 0 poor, 1 thin, 2 good, 3 strong (proposed, not
/// confirmed: a point a game either side of the average is a strong or a
/// poor fit).
pub(crate) fn fit_level(p: f64) -> usize {
    if p >= 0.8 {
        3
    } else if p >= 0.0 {
        2
    } else if p >= -0.8 {
        1
    } else {
        0
    }
}

fn fit_json(p: f64) -> Value {
    let l = fit_level(p);
    json!({ "word": FIT_WORDS[l], "n": l + 1 })
}

/// A coordinator's knowledge in words.
/// "McConnell knows the 3-4 well", or "You know the 3-4 well".
fn knows(name: &str, k: f64, what: &str) -> String {
    let (verb, adv) = match k {
        x if x >= 0.85 => ("know", " inside out"),
        x if x >= 0.7 => ("know", " well"),
        x if x >= 0.5 => ("know", " in part"),
        _ => ("barely know", ""),
    };
    if name == "you" { format!("You {verb} {what}{adv}") } else { format!("{name} {verb}s {what}{adv}") }
}

/// The line under a guideline's name: the play caller's own habit (yours
/// when you call that side), or whether he agreed with yours.
fn own_sub(name: &str, set: bool, agree: bool, own: &str) -> String {
    if name == "you" {
        format!("your own: {own}")
    } else if !set {
        format!("{name}'s own: {own}")
    } else if agree {
        format!("agreed with {name}")
    } else {
        format!("{name}: {own}")
    }
}

fn best(know: &[f64; 11], r: std::ops::Range<usize>) -> usize {
    let start = r.start;
    r.max_by(|&a, &b| know[a].total_cmp(&know[b])).unwrap_or(start)
}

/// One game's measure for a guideline: (value 0-1, snaps it counts).
fn measure(key: &str, g: &afm_engine::world::seat::SeatGame, side: usize, pick: &Value) -> Option<f64> {
    let plays = g.stats.plays.iter().filter(|r| !r.nullified && is_snap(r.kind));
    let ours = |r: &&afm_engine::game::PlayRecord| r.offense == side;
    let theirs = |r: &&afm_engine::game::PlayRecord| r.offense != side;
    let share = |n: usize, d: usize| (d >= 6).then(|| n as f64 / d as f64);
    match key {
        "front" => {
            let v: Vec<_> = plays.filter(theirs).filter(|r| r.dpers > 0).collect();
            let nickel = matches!(pick.as_str(), Some("4-2-5" | "3-3-5"));
            let n = v.iter().filter(|r| if nickel { matches!(r.dpers, 2 | 3) } else { r.dpers == 1 }).count();
            share(n, v.len())
        }
        "man" => {
            let v: Vec<_> = plays.filter(theirs).filter(|r| is_pass(r.kind)).collect();
            share(v.iter().filter(|r| r.coverage_man).count(), v.len())
        }
        "pressure" => {
            let v: Vec<_> = plays.filter(theirs).collect();
            share(v.iter().filter(|r| r.blitz).count(), v.len())
        }
        "personnel" => {
            let p = pick.as_str()?;
            let v: Vec<_> = plays.filter(ours).collect();
            share(v.iter().filter(|r| r.personnel == p).count(), v.len())
        }
        "pace" => {
            // Outside the two-minute drills and the hurry-up of a team behind late.
            let v: Vec<_> = plays
                .filter(ours)
                .filter(|r| !(matches!(r.quarter, 2 | 4) && r.clock <= 120.0))
                .filter(|r| !(r.quarter == 4 && r.score[side] < r.score[1 - side]))
                .collect();
            share(v.iter().filter(|r| r.hurry).count(), v.len())
        }
        "run" => {
            let v: Vec<_> = plays.filter(ours).collect();
            share(v.iter().filter(|r| r.kind == PlayType::Run).count(), v.len())
        }
        "stick" => {
            let v: Vec<_> = plays.filter(ours).collect();
            let n = v
                .iter()
                .filter(|r| {
                    matches!(r.call_why, "the opening script" | "on the call sheet" | "a shot play off the call sheet")
                })
                .count();
            share(n, v.len())
        }
        _ => None,
    }
}

/// The level's usual share of snaps in the base package and in nickel or
/// dime (real 2023-25: NFL base about 25%, college about 35%, high school
/// about half; proposed, not confirmed).
fn typical_base(level: Cal) -> f64 {
    match level {
        Cal::Nfl | Cal::Other => 0.25,
        Cal::HighSchool => 0.5,
        _ => 0.35,
    }
}

pub(super) fn view(w: &World) -> Reply {
    let s = seat(w)?;
    let is_head = w.seat_head();
    let g = if is_head { w.plans.guide } else { s.team.staff.guide };
    let st = &s.team.staff;
    let college = !matches!(s.level, Cal::Nfl | Cal::Other);
    let (oc, dc) = plans::callers(w);
    let games = season_games(w, &s);
    let fits = fit_reads(w, &s);
    let roster = roster_people(w, s.tr);
    let count = |f: &dyn Fn(afm_engine::positions::Position) -> bool| {
        roster.iter().filter(|&&i| f(w.people[i].player.position)).count()
    };
    use afm_engine::positions::Position as P;
    let (cbs, ss, lbs, tes, rbs, wrs) = (
        count(&|p| p == P::CB),
        count(&|p| p == P::S),
        count(&|p| p == P::LB),
        count(&|p| p == P::TE),
        count(&|p| p == P::RB),
        count(&|p| p == P::WR),
    );
    let know_of = |c: Option<usize>| c.map_or([0.75; 11], |c| w.coaches[c].system_know());
    let oknow = know_of(oc);
    let dknow = know_of(dc);
    let you = |c: Option<usize>| c.is_some_and(|c| w.coaches[c].is_yours());
    let caller_name = |c: Option<usize>, side: &str| match c {
        Some(c) if w.coaches[c].is_yours() => "you".to_string(),
        Some(c) => w.coaches[c].name.split_whitespace().last().unwrap_or("he").to_string(),
        None => format!("the {side}"),
    };
    let oname = caller_name(oc, "offense");
    let dname = caller_name(dc, "defense");
    let off_label = if you(oc) { "Your call" } else { "OC's call" };
    let def_label = if you(dc) { "Your call".to_string() } else { "DC's call".to_string() };
    let dis = |c: Option<usize>, offense: bool| -> Vec<&'static str> {
        c.filter(|&c| !w.coaches[c].is_yours())
            .map(|c| plans::disagreements(w, c, &g, offense).into_iter().map(|x| x.0).collect())
            .unwrap_or_default()
    };
    let odis = dis(oc, true);
    let ddis = dis(dc, false);
    let firm_text = |want: Option<String>, own: String, f: Firm, who: &str| -> String {
        match want {
            None if who == "you" => format!("Nothing said: you call it as you see it ({own})."),
            Some(t) if who == "you" => format!("You call this side yourself: {t} is yours to keep."),
            None => format!("Nothing said: {who} calls his own game ({own})."),
            Some(t) => match f {
                Firm::Firm => format!(
                    "Firm: \"this is how we play\". He moves most of the way (85%) from his own {own} toward {t}."
                ),
                Firm::Lean => format!("A lean: he moves about halfway (55%) from his own {own} toward {t}."),
                Firm::Theirs => format!("His call: a nudge (15%) from his own {own} toward {t}."),
            },
        }
    };
    let firm_of = |f: Firm, set: bool| if set { f.key() } else { "lean" };
    let games_json = |key: &str, pick: &Value| -> Vec<Value> {
        games
            .iter()
            .rev()
            .take(8)
            .rev()
            .filter_map(|(gm, side)| {
                let v = measure(key, gm, *side, pick)?;
                let (_, name) = opp_of(gm, *side);
                Some(json!({ "o": abbr(name, *side == 1), "title": name, "v": pct(v) }))
            })
            .collect()
    };
    let avg = |v: &[Value]| -> Option<f64> {
        (!v.is_empty()).then(|| v.iter().filter_map(|x| x["v"].as_f64()).sum::<f64>() / v.len() as f64)
    };
    let mut rows: Vec<Value> = Vec::new();

    // ---- Defense: the base front.
    // The front he knows best, and his own: what went in this season when
    // nothing is said (his pick for the roster), else the one he knows best.
    let best_front = {
        let f = FRONTS[best(&dknow, 5..7) - 5];
        if f == afm_engine::game::Front::FourThree { BaseFront::FourThree } else { BaseFront::ThreeFour }
    };
    let own_front = match g.front.to {
        None if st.front == afm_engine::game::Front::FourThree => BaseFront::FourThree,
        None => BaseFront::ThreeFour,
        Some(_) => best_front,
    };
    let front_pick = g.front.to.unwrap_or(if st.front == afm_engine::game::Front::FourThree {
        BaseFront::FourThree
    } else {
        BaseFront::ThreeFour
    });
    let fslot = |b: BaseFront| 5 + FRONTS.iter().position(|&x| x == b.front()).unwrap_or(0);
    let nickel_fit =
        |b: BaseFront| if b.nickel_base() { 0.3 * ((cbs + ss) as f64 - 9.0).clamp(-3.0, 3.0) } else { 0.0 };
    let front_fit = |b: BaseFront| fits[fslot(b)] + nickel_fit(b);
    {
        let pick = json!(front_pick.name());
        let gj = games_json("front", &pick);
        let k = g.front.pull().map_or(0.0, |x| x.1);
        let base = typical_base(s.level);
        let expect =
            if front_pick.nickel_base() { (1.0 - base) + k * 0.6 * base } else { base * (1.0 + 0.12 * k * 0.5) };
        let installed = if st.front == front_pick.front() { "installed" } else { "not installed" };
        let ov = systems::overlap(
            Kind::Front,
            fslot(front_pick) - 5,
            FRONTS.iter().position(|&x| x == st.front).unwrap_or(0),
        );
        let agree = !ddis.contains(&"front");
        rows.push(json!({
            "key": "front", "group": "Defense", "name": "Base front",
            "sub": own_sub(&dname, g.front.to.is_some(), agree, own_front.name()),
            "kind": "options",
            "options": fronts(s.level).into_iter().map(|b| json!({ "v": b.name(), "label": b.name(), "own": b == own_front, "fit": FIT_WORDS[fit_level(front_fit(b))] })).collect::<Vec<_>>(),
            "value": g.front.to.map(|b| b.name()),
            "own": own_front.name(),
            "firm": firm_of(g.front.firm, g.front.to.is_some()), "firm_labels": ["Firm", "Lean", def_label],
            "fit": fit_json(front_fit(front_pick)),
            "expect": pct(expect), "unit": format!("in {}", if front_pick.nickel_base() { "nickel" } else { "base" }),
            "season": avg(&gj).map(|x| x.round()), "games": gj, "target": pct(expect), "own_line": Value::Null,
            "measure": if front_pick.nickel_base() { "share of their snaps you played nickel or dime" } else { "share of their snaps you played base" },
            "who": "Defensive coordinator",
            "set_text": match g.front.to { Some(b) => format!("You asked for the {} as the base front ({installed} this season).", b.name()), None => format!("Nothing said: the {} goes in as the staff likes.", own_front.name()) },
            "firm_text": firm_text(g.front.to.map(|b| b.name().to_string()), own_front.name().to_string(), g.front.firm, &dname),
            "kv": [
                { "k": "Your roster · fit", "v": format!("Your staff reads the roster as a {} fit for the {}: {} corners, {} safeties, {} linebackers.", FIT_WORDS[fit_level(front_fit(front_pick))].to_lowercase(), front_pick.name(), cbs, ss, lbs) },
                { "k": "The coordinator's view", "v": if you(dc) { "You call the defense yourself: your guidelines are yours to keep.".to_string() } else if agree { format!("Agrees: {dname} knows the {} best.", best_front.name()) } else { format!("He'd rather not: {dname} knows the {} best.", best_front.name()) } },
                { "k": "How well it gets called", "v": format!("{}.", knows(&dname, dknow[fslot(front_pick)], &format!("the {}", front_pick.name()))) },
                { "k": "If you change it", "v": if (ov - 1.0).abs() < 1e-9 { "No new learning: the same front, more or less nickel.".to_string() } else { format!("About {:.0}% of the front's calls carry over; the rest goes in at the spring install and is new learning for the line and the linebackers.", 100.0 * ov) } },
                { "k": "Elsewhere it shows", "v": "The depth chart's packages, recruiting and the draft board's fit reads for the front seven." },
            ],
        }));
    }

    // ---- Defense: coverage, man share.
    {
        let own = guide::habit_man(st.shell);
        let want = g.man.to;
        let k = g.man.pull().map_or(0.0, |x| x.1);
        let expect = want.map_or(own, |t| guide::expected(own, t, k));
        let near = (0..4)
            .min_by(|&a, &b| {
                (guide::habit_man(SHELLS[a]) - expect).abs().total_cmp(&(guide::habit_man(SHELLS[b]) - expect).abs())
            })
            .unwrap_or(0);
        let gj = games_json("man", &Value::Null);
        let agree = !ddis.contains(&"man");
        let dbest = best(&dknow, 7..11) - 7;
        rows.push(json!({
            "key": "man", "group": "Defense", "name": "Coverage",
            "sub": own_sub(&dname, want.is_some(), agree, &format!("{:.0}% man", 100.0 * own)),
            "kind": "track", "min": 0, "max": 100, "step": 5, "label_lo": "zone", "label_hi": "man",
            "value": want.map(pct), "own": pct(own),
            "firm": firm_of(g.man.firm, want.is_some()), "firm_labels": ["Firm", "Lean", def_label],
            "fit": fit_json(fits[7 + near]),
            "expect": pct(expect), "unit": "% man",
            "season": avg(&gj).map(|x| x.round()), "games": gj, "target": pct(expect), "own_line": pct(own),
            "measure": "man coverage on their throws",
            "who": "Defensive coordinator",
            "set_text": match want { Some(t) => format!("You asked for man coverage on {:.0}% of their throws.", 100.0 * t), None => "Nothing said: the coverage mix is the coordinator's.".to_string() },
            "firm_text": firm_text(want.map(|t| format!("{:.0}%", 100.0 * t)), format!("{:.0}%", 100.0 * own), g.man.firm, &dname),
            "kv": [
                { "k": "Your roster · fit", "v": format!("At about {:.0}% man the secondary plays mostly {}: your staff reads the roster as a {} fit for it ({} corners, {} safeties).", 100.0 * expect, SHELL_NAMES[near], FIT_WORDS[fit_level(fits[7 + near])].to_lowercase(), cbs, ss) },
                { "k": "The coordinator's view", "v": if you(dc) { "You call the defense yourself.".to_string() } else if agree { format!("Agrees: close to the {} he knows best.", SHELL_NAMES[dbest]) } else { format!("He'd rather not: {dname} knows {} best.", SHELL_NAMES[dbest]) } },
                { "k": "How well it gets called", "v": if you(dc) { format!("{}.", knows(&dname, dknow[7 + near], SHELL_NAMES[near])) } else { format!("{}; a coordinator moved off his own coverages calls them as well as he knows them.", knows(&dname, dknow[7 + near], SHELL_NAMES[near])) } },
                { "k": "If you change it", "v": "No install of its own: the same coverages, called more or less often. A big move pulls next spring's install toward the family that plays it." },
                { "k": "Elsewhere it shows", "v": "Recruiting and the draft board grade corners for press-man or zone against it; game day's answers start from it." },
            ],
        }));
    }

    // ---- Defense: pressure.
    {
        let own = guide::habit_pressure(st.defense.aggressiveness);
        let want = g.pressure.to;
        let k = g.pressure.pull().map_or(0.0, |x| x.1);
        let expect = want.map_or(own, |t| guide::expected(own, t, k));
        let gj = games_json("pressure", &Value::Null);
        let agree = !ddis.contains(&"pressure");
        let edges = count(&|p| p == P::EDGE);
        rows.push(json!({
            "key": "pressure", "group": "Defense", "name": "Pressure",
            "sub": own_sub(&dname, want.is_some(), agree, &format!("{:.0}%", 100.0 * own)),
            "kind": "track", "min": 0, "max": 60, "step": 5, "label_lo": "four", "label_hi": "five or more",
            "value": want.map(pct), "own": pct(own),
            "firm": firm_of(g.pressure.firm, want.is_some()), "firm_labels": ["Firm", "Lean", def_label],
            "fit": fit_json(fits[7 + 2] * if expect > own { 1.0 } else { 0.5 }),
            "expect": pct(expect), "unit": "% 5+ rush",
            "season": avg(&gj).map(|x| x.round()), "games": gj, "target": pct(expect), "own_line": pct(own),
            "measure": "snaps with five or more rushers",
            "who": "Defensive coordinator",
            "set_text": match want { Some(t) => format!("You asked for five or more rushers on {:.0}% of snaps.", 100.0 * t), None => "Nothing said: the pressure is the coordinator's.".to_string() },
            "firm_text": firm_text(want.map(|t| format!("{:.0}%", 100.0 * t)), format!("{:.0}%", 100.0 * own), g.pressure.firm, &dname),
            "kv": [
                { "k": "Your roster · fit", "v": format!("Pressure leans on the corners behind it: your staff reads man and pressure as a {} fit ({} corners, {} edge rushers).", FIT_WORDS[fit_level(fits[9])].to_lowercase(), cbs, edges) },
                { "k": "The coordinator's view", "v": if you(dc) { "You call the defense yourself.".to_string() } else if agree { "Agrees: about what his nature brings.".to_string() } else { format!("He'd rather not: {dname} sends {:.0}% on his own.", 100.0 * own) } },
                { "k": "How well it gets called", "v": format!("{}.", knows(&dname, dknow[9], "man and pressure")) },
                { "k": "If you change it", "v": "No new learning: the same pressures, sent more or less often." },
                { "k": "Elsewhere it shows", "v": "The 5+ rush on game day's answers to each personnel group starts here." },
            ],
        }));
    }

    // ---- Offense: the system.
    {
        let best_o = OFFENSES[best(&oknow, 0..5)];
        let own_o = if g.system.to.is_none() && OFFENSES.contains(&st.off_scheme) { st.off_scheme } else { best_o };
        let pick = g.system.to.unwrap_or(st.off_scheme);
        let slot = |o: OffScheme| OFFENSES.iter().position(|&x| x == o).unwrap_or(3);
        let ofit = |o: OffScheme| if o == OffScheme::WingT { fits[3] - 0.2 } else { fits[slot(o)] };
        let ov = systems::overlap(Kind::Offense, slot(pick), slot(st.off_scheme));
        let agree = !odis.contains(&"system");
        let spring = match (g.system.to, g.system.firm) {
            (None, _) => "his call",
            (Some(_), Firm::Firm) => "goes in",
            (Some(_), Firm::Lean) => "likely",
            (Some(_), Firm::Theirs) => "his call",
        };
        let how = {
            let k = if pick == OffScheme::WingT { oknow[3] * 0.75 } else { oknow[slot(pick)] };
            let line = knows(&oname, k, &format!("the {}", scheme_name(pick)));
            if you(oc) {
                format!("{line}.")
            } else {
                format!("{line}; asked to call what he knows less, he calls it as well as he knows it.")
            }
        };
        rows.push(json!({
            "key": "system", "group": "Offense", "name": "System",
            "sub": own_sub(&oname, g.system.to.is_some(), agree, &scheme_name(own_o)),
            "kind": "options",
            "options": offenses(s.level).into_iter().map(|o| json!({ "v": scheme_key(o), "label": scheme_name(o), "own": o == own_o, "fit": FIT_WORDS[fit_level(ofit(o))] })).collect::<Vec<_>>(),
            "value": g.system.to.map(scheme_key), "own": scheme_key(own_o),
            "firm": firm_of(g.system.firm, g.system.to.is_some()), "firm_labels": ["Firm", "Lean", off_label],
            "fit": fit_json(ofit(pick)),
            "expect": spring, "unit": "next spring",
            "season": Value::Null, "games": [], "target": Value::Null, "own_line": Value::Null,
            "measure": format!("the system goes in each spring: {} this season", scheme_name(st.off_scheme)),
            "who": "Offensive coordinator",
            "set_text": match g.system.to { Some(o) => format!("You asked for the {} offense. This season's install: {}.", scheme_name(o), scheme_name(st.off_scheme)), None => format!("Nothing said: {} is installed.", scheme_name(st.off_scheme)) },
            "firm_text": firm_text(g.system.to.map(scheme_name), scheme_name(own_o), g.system.firm, &oname),
            "kv": [
                { "k": "Your roster · fit", "v": format!("Your staff reads the roster as a {} fit for the {} ({} receivers, {} tight ends, {} backs).", FIT_WORDS[fit_level(ofit(pick))].to_lowercase(), scheme_name(pick), wrs, tes, rbs) },
                { "k": "The coordinator's view", "v": if you(oc) { "You call the offense yourself.".to_string() } else if agree { format!("Agrees: {oname} knows the {} best.", scheme_name(best_o)) } else { format!("He'd rather not: {oname} knows the {} best.", scheme_name(best_o)) } },
                { "k": "How well it gets called", "v": how },
                { "k": "If you change it", "v": if (ov - 1.0).abs() < 1e-9 { "Nothing to learn: it's the system in now.".to_string() } else { format!("About {:.0}% of the playbook carries over from the {}; the rest is new learning at the spring install, and the call sheet thins until it goes in.", 100.0 * ov, scheme_name(st.off_scheme)) } },
                { "k": "Elsewhere it shows", "v": "The spring install, the call sheet's plays, recruiting's quarterback and receiver fit." },
            ],
        }));
    }

    // ---- Offense: personnel.
    {
        let season_share = |p: Personnel| -> Option<f64> {
            let (mut n, mut d) = (0usize, 0usize);
            for (gm, side) in &games {
                for r in gm.stats.plays.iter().filter(|r| r.offense == *side && !r.nullified && is_snap(r.kind)) {
                    d += 1;
                    if r.personnel == p.name() {
                        n += 1;
                    }
                }
            }
            (d >= 60).then(|| n as f64 / d as f64)
        };
        let own_p = PERSONNEL
            .iter()
            .copied()
            .max_by(|a, b| season_share(*a).unwrap_or(0.0).total_cmp(&season_share(*b).unwrap_or(0.0)))
            .filter(|p| season_share(*p).is_some())
            .unwrap_or(match st.off_scheme {
                OffScheme::AirRaid => Personnel::P10,
                OffScheme::PowerGap | OffScheme::WingT => Personnel::P21,
                OffScheme::WideZone => Personnel::P12,
                _ => Personnel::P11,
            });
        let pick = g.personnel.to.unwrap_or(own_p);
        let own_share = season_share(pick).unwrap_or(if pick == own_p { 0.55 } else { 0.15 });
        let k = g.personnel.pull().map_or(0.0, |x| x.1);
        let expect = own_share + 0.55 * k * (1.0 - own_share) * 0.85;
        let pfit = |p: Personnel| match p {
            Personnel::P10 => fits[1] + 0.2 * (wrs as f64 - 7.0).clamp(-2.0, 2.0),
            Personnel::P12 => fits[2] + 0.3 * (tes as f64 - 3.0).clamp(-2.0, 2.0),
            Personnel::P21 => fits[3] + 0.3 * (rbs as f64 - 4.0).clamp(-2.0, 2.0),
            _ => fits[0],
        };
        let gj = games_json("personnel", &json!(pick.name()));
        rows.push(json!({
            "key": "personnel", "group": "Offense", "name": "Personnel",
            "sub": own_sub(&oname, g.personnel.to.is_some(), pick == own_p, own_p.name()),
            "kind": "options",
            "options": PERSONNEL.iter().map(|&p| json!({ "v": p.name(), "label": p.name(), "own": p == own_p, "fit": FIT_WORDS[fit_level(pfit(p))] })).collect::<Vec<_>>(),
            "value": g.personnel.to.map(|p| p.name()), "own": own_p.name(),
            "firm": firm_of(g.personnel.firm, g.personnel.to.is_some()), "firm_labels": ["Firm", "Lean", off_label],
            "fit": fit_json(pfit(pick)),
            "expect": pct(expect), "unit": format!("in {}", pick.name()),
            "season": avg(&gj).map(|x| x.round()), "games": gj, "target": pct(expect), "own_line": Value::Null,
            "measure": format!("snaps in {} personnel", pick.name()),
            "who": "Offensive coordinator",
            "set_text": match g.personnel.to { Some(p) => format!("You asked for {} personnel as the base group.", p.name()), None => "Nothing said: the groups are the coordinator's.".to_string() },
            "firm_text": firm_text(g.personnel.to.map(|p| p.name().to_string()), own_p.name().to_string(), g.personnel.firm, &oname),
            "kv": [
                { "k": "Your roster · fit", "v": format!("{} receivers, {} tight ends, {} backs: your staff reads {} personnel as a {} fit.", wrs, tes, rbs, pick.name(), FIT_WORDS[fit_level(pfit(pick))].to_lowercase()) },
                { "k": "The coordinator's view", "v": if you(oc) { "You call the offense yourself.".to_string() } else if pick == own_p { "Agrees: it's his base group.".to_string() } else { format!("He'd rather stay in {}, his base group.", own_p.name()) } },
                { "k": "How well it gets called", "v": "Short yardage and the hurry-up keep their own groups: the guideline wins some of the normal downs." },
                { "k": "If you change it", "v": "No new calls: the same plays from a different group, and the depth chart's next man up gets the snaps." },
                { "k": "Elsewhere it shows", "v": "The depth chart's jumbo and goal line packages, and recruiting's tight end and fullback needs." },
            ],
        }));
    }

    // ---- Offense: tempo.
    {
        let own = guide::habit_pace(st.off_scheme);
        let own_p = if own >= 0.3 { Pace::Mixed } else { Pace::Huddle };
        let k = g.pace.pull().map_or(0.0, |x| x.1);
        let expect = g.pace.to.map_or(own, |p| guide::expected(own, p.share(), k));
        let gj = games_json("pace", &Value::Null);
        let agree = !odis.contains(&"pace");
        rows.push(json!({
            "key": "pace", "group": "Offense", "name": "Tempo",
            "sub": own_sub(&oname, g.pace.to.is_some(), agree, &format!("{:.0}% no-huddle", 100.0 * own)),
            "kind": "options",
            "options": Pace::ALL.iter().map(|&p| json!({ "v": pace_key(p), "label": p.name(), "own": p == own_p, "fit": Value::Null })).collect::<Vec<_>>(),
            "value": g.pace.to.map(pace_key), "own": pace_key(own_p), "own_share": own,
            "shares": Pace::ALL.iter().map(|p| p.share()).collect::<Vec<_>>(),
            "firm": firm_of(g.pace.firm, g.pace.to.is_some()), "firm_labels": ["Firm", "Lean", off_label],
            "fit": fit_json(if g.pace.to == Some(Pace::NoHuddle) { (fits[1] + fits[4]) / 2.0 } else { fits[OFFENSES.iter().position(|&o| o == st.off_scheme).unwrap_or(0)] }),
            "expect": pct(expect), "unit": "% no-huddle",
            "season": avg(&gj).map(|x| x.round()), "games": gj, "target": pct(expect), "own_line": pct(own),
            "measure": "no-huddle snaps outside the two-minute drill",
            "who": "Offensive coordinator",
            "set_text": match g.pace.to { Some(p) => format!("You asked for {} tempo.", p.name().to_lowercase()), None => "Nothing said: the tempo is the system's.".to_string() },
            "firm_text": firm_text(g.pace.to.map(|p| format!("{:.0}% no-huddle", 100.0 * p.share())), format!("{:.0}%", 100.0 * own), g.pace.firm, &oname),
            "kv": [
                { "k": "Your roster · fit", "v": "No-huddle leans on the line's legs and the quarterback's grasp: the conditioning in your practice week carries it." },
                { "k": "The coordinator's view", "v": if you(oc) { "You call the offense yourself.".to_string() } else if agree { "Agrees.".to_string() } else { "He'd rather huddle: his system plays at its own pace.".to_string() } },
                { "k": "How well it gets called", "v": "Without a huddle the defense can't substitute or disguise as much; your own players tire faster too." },
                { "k": "If you change it", "v": "No new calls; game day's tempo rule can still change it for one game." },
                { "k": "Elsewhere it shows", "v": "Game day's tempo rule and the practice week's conditioning." },
            ],
        }));
    }

    // ---- Offense: run or pass.
    {
        let own = 1.0 - guide::habit_pass(college, st.off_scheme, st.offense.aggressiveness);
        let want = g.run.to;
        let k = g.run.pull().map_or(0.0, |x| x.1);
        let expect = want.map_or(own, |t| guide::expected(own, t, k));
        let gj = games_json("run", &Value::Null);
        let agree = !odis.contains(&"run");
        let runfit = if expect >= own { (fits[2] + fits[3]) / 2.0 } else { (fits[0] + fits[1]) / 2.0 };
        rows.push(json!({
            "key": "run", "group": "Offense", "name": "Run or pass",
            "sub": own_sub(&oname, want.is_some(), agree, &format!("{:.0}% runs", 100.0 * own)),
            "kind": "track", "min": 20, "max": 80, "step": 5, "label_lo": "throw", "label_hi": "run",
            "value": want.map(pct), "own": pct(own),
            "firm": firm_of(g.run.firm, want.is_some()), "firm_labels": ["Firm", "Lean", off_label],
            "fit": fit_json(runfit),
            "expect": pct(expect), "unit": "% runs",
            "season": avg(&gj).map(|x| x.round()), "games": gj, "target": pct(expect), "own_line": pct(own),
            "measure": "runs of your snaps (sacks and scrambles count as throws)",
            "who": "Offensive coordinator",
            "set_text": match want { Some(t) => format!("You asked for runs on {:.0}% of snaps.", 100.0 * t), None => "Nothing said: the mix is the coordinator's.".to_string() },
            "firm_text": firm_text(want.map(|t| format!("{:.0}%", 100.0 * t)), format!("{:.0}%", 100.0 * own), g.run.firm, &oname),
            "kv": [
                { "k": "Your roster · fit", "v": format!("Your staff reads the roster as a {} fit for {}.", FIT_WORDS[fit_level(runfit)].to_lowercase(), if expect >= own { "running more" } else { "throwing more" }) },
                { "k": "The coordinator's view", "v": if you(oc) { "You call the offense yourself.".to_string() } else if agree { "Agrees: about his own mix.".to_string() } else { format!("He'd rather run {:.0}%.", 100.0 * own) } },
                { "k": "How well it gets called", "v": "On every down the down, the distance and the score still rule: a third and long is a throw whatever the guideline." },
                { "k": "If you change it", "v": "No new learning: the same plays, called in a different mix." },
                { "k": "Elsewhere it shows", "v": "The call sheet's boxes and the self-scout's tells." },
            ],
        }));
    }

    // ---- Program: stick to the card.
    {
        let own = guide::habit_stick(st.offense.creativity);
        let want = g.stick.to;
        let k = g.stick.pull().map_or(0.0, |x| x.1);
        let expect = want.map_or(own, |t| guide::expected(own, t, k));
        let gj = games_json("stick", &Value::Null);
        let fams = plans::sheet_families(w);
        let grasp = plans::unit_grasp(&s.team, s.level, 0, &fams);
        let gl = if grasp >= 0.85 {
            3.0
        } else if grasp >= 0.75 {
            0.5
        } else if grasp >= 0.65 {
            -0.5
        } else {
            -1.5
        };
        rows.push(json!({
            "key": "stick", "group": "Program", "name": "Stick to the card",
            "sub": if want.is_none() { format!("the play callers' own: {:.0}%", 100.0 * own) } else { "both play callers".to_string() },
            "kind": "track", "min": 40, "max": 100, "step": 5, "label_lo": "feel", "label_hi": "the card",
            "value": want.map(pct), "own": pct(own),
            "firm": firm_of(g.stick.firm, want.is_some()), "firm_labels": ["Firm", "Lean", "Their call"],
            "fit": fit_json(gl),
            "expect": pct(expect), "unit": "% from the card",
            "season": avg(&gj).map(|x| x.round()), "games": gj, "target": pct(expect), "own_line": pct(own),
            "measure": "your snaps called off the call sheet and the script",
            "who": "Both play callers",
            "set_text": match want { Some(t) => format!("You asked for {:.0}% of calls from the card when a card play fits.", 100.0 * t), None => "Nothing said: each play caller sticks to it as he likes.".to_string() },
            "firm_text": firm_text(want.map(|t| format!("{:.0}%", 100.0 * t)), format!("{:.0}%", 100.0 * own), g.stick.firm, "each play caller"),
            "kv": [
                { "k": "Your roster · fit", "v": format!("The offense knows about {:.0}% of this week's card: the more they know it, the more the card pays.", 100.0 * grasp) },
                { "k": "The coordinators' view", "v": "A creative play caller goes off the card more; a firm guideline keeps him on it." },
                { "k": "How well it gets called", "v": "Off the card, a call is the play caller's own read: less practiced this week." },
                { "k": "If you change it", "v": "No new learning." },
                { "k": "Elsewhere it shows", "v": "The call sheet: what your coordinators draft is what gets called." },
            ],
        }));
    }

    // ---- The coordinators.
    let mut coords: Vec<Value> = Vec::new();
    let mut seen: Vec<usize> = Vec::new();
    for (c, offense) in [(oc, true), (dc, false)] {
        let Some(c) = c else { continue };
        if seen.contains(&c) {
            continue;
        }
        seen.push(c);
        let both = oc == dc;
        let co = &w.coaches[c];
        let keys: &[&str] = if both {
            &["front", "man", "pressure", "system", "personnel", "pace", "run"]
        } else if offense {
            &["system", "personnel", "pace", "run"]
        } else {
            &["front", "man", "pressure"]
        };
        let set = keys
            .iter()
            .filter(|k| match **k {
                "front" => g.front.to.is_some(),
                "man" => g.man.to.is_some(),
                "pressure" => g.pressure.to.is_some(),
                "system" => g.system.to.is_some(),
                "personnel" => g.personnel.to.is_some(),
                "pace" => g.pace.to.is_some(),
                _ => g.run.to.is_some(),
            })
            .count();
        let mut against: Vec<(&str, Firm)> = Vec::new();
        if !co.is_yours() {
            if both || offense {
                against.extend(plans::disagreements(w, c, &g, true));
            }
            if both || !offense {
                against.extend(plans::disagreements(w, c, &g, false));
            }
        }
        let firm_n = against.iter().filter(|x| x.1 == Firm::Firm).count();
        let talked = w.plans.talks.iter().filter(|t| t.0 == c).map(|t| t.1).max();
        let talked_recent = talked.is_some_and(|d| w.today.0 - d.0 <= 31);
        let know = co.system_know();
        let knows = if both {
            format!(
                "Knows the {} and {} best",
                scheme_name(OFFENSES[best(&know, 0..5)]),
                SHELL_NAMES[best(&know, 7..11) - 7]
            )
        } else if offense {
            let b = best(&know, 0..5);
            format!("Knows the {} best ({:.0}%)", scheme_name(OFFENSES[b]), 100.0 * know[b])
        } else {
            let f = best(&know, 5..7) - 5;
            let sh = best(&know, 7..11) - 7;
            format!("Knows the {} and {} best", if f == 0 { "4-3" } else { "3-4" }, SHELL_NAMES[sh])
        };
        let trust = w.folk.mine.trust_of(coach_key(c)).unwrap_or(0.5);
        let (mood, warn) = if co.is_yours() {
            ("you call it".to_string(), false)
        } else if firm_n > 0 && !talked_recent {
            ("wants a talk".to_string(), true)
        } else if trust < 0.35 {
            ("frustrated".to_string(), true)
        } else {
            ("fine with them".to_string(), false)
        };
        let role = if co.is_yours() {
            if both {
                "You call both sides".to_string()
            } else if offense {
                "You call the offense".to_string()
            } else {
                "You call the defense".to_string()
            }
        } else if both {
            "Calls both sides".to_string()
        } else if offense {
            "Offensive coordinator".to_string()
        } else {
            "Defensive coordinator".to_string()
        };
        let line = if co.is_yours() {
            "Your guidelines on your own side are yours to keep.".to_string()
        } else if firm_n > 0 {
            match talked.filter(|_| talked_recent) {
                Some(d) => format!("{firm_n} firm against his views · talked through {}", date_text(d)),
                None => format!("{firm_n} firm against his views: his trust slips each month"),
            }
        } else if !against.is_empty() {
            format!("{} against his views, none firm", against.len())
        } else {
            "No guideline against his views".to_string()
        };
        coords.push(json!({
            "id": c, "name": coach_link(w, c), "plain": co.name, "mood": mood, "warn": warn, "role": role,
            "agree": set.saturating_sub(against.len()), "of": set, "knows": knows, "line": line,
            "side": if both { "both" } else if offense { "offense" } else { "defense" },
        }));
    }

    Ok(json!({
        "team": team_brief(w, s.tr),
        "level": lvl(s.level),
        "head": is_head,
        "set": w.plans.guide_set.map(date_text),
        "season": season_words(w),
        "games": games.len(),
        "rows": rows,
        "coords": coords,
        "pulls": { "firm": Firm::Firm.pull(), "lean": Firm::Lean.pull(), "theirs": Firm::Theirs.pull() },
    }))
}

fn firm_arg(v: &Value) -> Firm {
    v.as_str().and_then(Firm::from_key).unwrap_or(Firm::Lean)
}

fn share_arg(v: &Value, lo: f64, hi: f64) -> Result<Option<f64>, String> {
    if v.is_null() {
        return Ok(None);
    }
    let x = v.as_f64().ok_or("Give a share from 0 to 100.")?;
    Ok(Some((x / 100.0).clamp(lo, hi)))
}

/// One guideline change: key, to (None clears it) and firmness.
fn apply_one(g: &mut Guidelines, c: &Value) -> Result<(), String> {
    let key = c.get("key").and_then(Value::as_str).ok_or("Missing key.")?;
    let to = c.get("to").cloned().unwrap_or(Value::Null);
    let firm_given = c.get("firm").filter(|v| !v.is_null());
    macro_rules! upd {
        ($f:ident, $val:expr) => {{
            let firm = firm_given.map(firm_arg).unwrap_or(g.$f.firm);
            if c.get("to").is_some() {
                g.$f = match $val {
                    Some(x) => Want::new(x, firm),
                    None => Want::default(),
                };
            } else {
                g.$f.firm = firm;
            }
        }};
    }
    match key {
        "front" => upd!(front, to.as_str().and_then(|s| BaseFront::ALL.into_iter().find(|b| b.name() == s))),
        "man" => upd!(man, share_arg(&to, 0.0, 1.0)?),
        "pressure" => upd!(pressure, share_arg(&to, 0.0, 0.6)?),
        "system" => upd!(
            system,
            to.as_str().and_then(|s| [
                OffScheme::WestCoast,
                OffScheme::AirRaid,
                OffScheme::WideZone,
                OffScheme::PowerGap,
                OffScheme::SpreadOption,
                OffScheme::WingT
            ]
            .into_iter()
            .find(|&o| scheme_key(o) == s))
        ),
        "personnel" => upd!(personnel, to.as_str().and_then(|s| PERSONNEL.into_iter().find(|p| p.name() == s))),
        "pace" => upd!(pace, to.as_str().and_then(|s| Pace::ALL.into_iter().find(|&p| pace_key(p) == s))),
        "run" => upd!(run, share_arg(&to, 0.2, 0.8)?),
        "stick" => upd!(stick, share_arg(&to, 0.4, 1.0)?),
        _ => return Err(format!("No guideline called {key}.")),
    }
    Ok(())
}

/// Set guidelines: one `{key, to, firm}` or `{changes: [...]}`. Only the
/// head coach; the wing-T only at high school; the 4-2-5 and 3-3-5 not in
/// the NFL.
pub(super) fn set(w: &mut World, args: &Value) -> Result<(), String> {
    head(w)?;
    let level = w.seat_team().map(|x| x.0).ok_or("You have no team.")?;
    let mut g = w.plans.guide;
    match args.get("changes").and_then(Value::as_array) {
        Some(list) => {
            for c in list {
                apply_one(&mut g, c)?;
            }
        }
        None => apply_one(&mut g, args)?,
    }
    if level != Cal::HighSchool && g.system.to == Some(OffScheme::WingT) {
        return Err("The wing-T is a high school offense: no college or NFL staff installs it.".into());
    }
    if matches!(level, Cal::Nfl | Cal::Other) && g.front.to.is_some_and(|b| b.nickel_base()) {
        return Err("NFL staffs list a 4-3 or a 3-4 base; nickel is a package for the depth chart.".into());
    }
    if g != w.plans.guide {
        w.plans.guide = g;
        w.plans.guide_set = Some(w.today);
        w.refresh_seat_team();
    }
    Ok(())
}

/// Talk the identity through with the coordinators: for a month a firm
/// guideline against their views costs a little less trust.
pub(super) fn talk(w: &mut World) -> Result<(), String> {
    head(w)?;
    let (oc, dc) = plans::callers(w);
    let today = w.today;
    let mut n = 0;
    for c in [oc, dc].into_iter().flatten() {
        if w.coaches[c].is_yours() {
            continue;
        }
        w.plans.talks.retain(|t| t.0 != c);
        w.plans.talks.push((c, today));
        n += 1;
    }
    if n == 0 {
        return Err("You call it yourself: there's nobody to talk it through with.".into());
    }
    Ok(())
}
