//! V12, the line battles (a game page's tab): every rep up front from the
//! game's line chart (`game::chart`), in three views: your pass blocking,
//! your run blocking and your pass rush. The pressure and the gaps it came
//! through, the protection and rush calls, each blocker's reps snap by
//! snap, and each matchup with both men's ratings as your staff reads them
//! (yours from practice, theirs from the scouting file or the tape).
//! Games kept before the chart have the won and lost reps only.

use std::collections::HashMap;

use afm_engine::attributes::Attr;
use afm_engine::game::chart::{LineChart, Protection, Rep, Rusher};
use afm_engine::game::stats::{GameStats, PlayRecord, PlayType};
use afm_engine::world::World;
use afm_engine::world::coach::Group;
use afm_engine::world::scouting::readers;
use serde_json::{Value, json};

use super::r1;
use crate::Reply;
use crate::reads::{self, Read};
use crate::views::{self, TeamRef};

/// A rep from our side of it: won, even or lost.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Out {
    Won,
    Even,
    Lost,
}

impl Out {
    fn code(self) -> u8 {
        self as u8
    }
    fn word(self) -> &'static str {
        match self {
            Out::Won => "Won",
            Out::Even => "Even",
            Out::Lost => "Lost",
        }
    }
    fn flip(self) -> Out {
        match self {
            Out::Won => Out::Lost,
            Out::Lost => Out::Won,
            Out::Even => Out::Even,
        }
    }
}

/// Where a pass block ends counts: a rusher who came off it or had the
/// blocker going backwards (leverage 0.6 and up) beat it, the engine's own
/// line for a lost block; one held under 0.2 lost to it; between is even
/// (proposed, not confirmed: about the share of reps PFF grades neutral).
/// A run block: shed is lost, driven or sealed (leverage -0.5 or less, or a
/// pancake) is won.
fn outcome(r: &Rep) -> Out {
    let w = r.lev as f64 / 50.0;
    if r.pass() {
        if r.shed() || w >= 0.6 {
            Out::Lost
        } else if w < 0.2 {
            Out::Won
        } else {
            Out::Even
        }
    } else if r.shed() {
        Out::Lost
    } else if r.pancake() || w <= -0.5 {
        Out::Won
    } else {
        Out::Even
    }
}

/// One rep the board reads, from the chart or (games kept before it) from
/// the play's won and lost blocks.
#[derive(Clone, Copy)]
struct Seen {
    blocker: u32,
    defender: u32,
    out: Out,
    secs: f64,
    mv: Option<u8>,
    double: bool,
    /// The blocker's spot on the line (charted games).
    bpos: Option<u8>,
}

fn reps_of(r: &PlayRecord, pass: bool) -> Vec<Seen> {
    match &r.line {
        Some(c) => c
            .reps
            .iter()
            .filter(|x| x.pass() == pass)
            .map(|x| Seen {
                blocker: x.blocker,
                defender: x.defender,
                out: outcome(x),
                secs: x.secs(),
                mv: pass.then_some(x.mv),
                double: x.double(),
                bpos: Some(x.bpos),
            })
            .collect(),
        None => r
            .blocks
            .iter()
            .filter(|b| b.2 == pass)
            .map(|b| Seen {
                blocker: b.0,
                defender: b.1,
                out: if b.3 { Out::Won } else { Out::Lost },
                secs: b.4,
                mv: r.rush_wins.iter().find(|x| x.0 == b.1).map(|x| x.1 as u8),
                double: false,
                bpos: None,
            })
            .collect(),
    }
}

fn dropback(r: &PlayRecord) -> bool {
    matches!(r.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble)
}

/// The players' names and positions, and who on the game was hurt.
struct Who<'a> {
    w: &'a World,
    names: crate::lane_gameday::film::Names,
}

impl Who<'_> {
    fn name(&self, id: u32) -> String {
        self.names.get(&id).map_or(format!("No. {id}"), |x| x.0.clone())
    }
    fn pos(&self, id: u32) -> String {
        self.names.get(&id).map_or(String::new(), |x| x.1.clone())
    }
    fn person(&self, id: u32) -> Option<usize> {
        views::person_index(self.w, id)
    }
}

/// The move that beat a block, in a line coach's words.
fn move_name(w: &World, defender: u32, mv: u8, chart: Option<&LineChart>) -> &'static str {
    let rusher = chart.and_then(|c| c.rush.iter().find(|r| r.id == defender));
    if let Some(r) = rusher {
        if r.flags & Rusher::STUNT != 0 {
            return "Stunt";
        }
        if r.flags & Rusher::LINEMAN == 0 {
            return match r.gap {
                2 => "A-gap blitz",
                0 | 4 => "Edge blitz",
                _ => "Blitz",
            };
        }
    }
    let p = views::person_index(w, defender).map(|i| &w.people[i].player);
    let edge = p.is_some_and(|p| matches!(p.position.name(), "DE" | "EDGE" | "OLB"));
    match mv {
        0 => "Speed rush",
        1 => {
            if p.is_some_and(|p| afm_engine::body::body_now(p).arm >= 34.0) {
                "Long arm"
            } else {
                "Bull rush"
            }
        }
        _ => {
            if !edge {
                "Swim"
            } else if p.is_some_and(|p| p.attributes.get(Attr::Agility) >= 75.0) {
                "Spin"
            } else {
                "Rip"
            }
        }
    }
}

/// The run hole a carry went through, from where the runner crossed the
/// line (the call's direction when he never got there).
fn hole(r: &PlayRecord) -> usize {
    if let Some((dy, _)) = r.tend.cross {
        return if dy > 4.0 {
            0
        } else if dy > 1.5 {
            1
        } else if dy >= -1.5 {
            2
        } else if dy >= -4.0 {
            3
        } else {
            4
        };
    }
    let c = r.call.to_lowercase();
    let left = c.ends_with("left");
    let right = c.ends_with("right");
    let outside = ["outside zone", "toss", "sweep", "option"].iter().any(|x| c.contains(x));
    if !(left || right) || ["draw", "sneak"].iter().any(|x| c.contains(x)) {
        2
    } else if outside {
        if left { 0 } else { 4 }
    } else if left {
        1
    } else {
        3
    }
}

const RUN_SCHEMES: [(&str, &str); 12] = [
    ("inside zone", "Inside zone"),
    ("zone read", "Inside zone"),
    ("outside zone", "Outside zone"),
    ("duo", "Duo"),
    ("power", "Power"),
    ("counter", "Counter"),
    ("trap", "Trap"),
    ("iso", "Iso"),
    ("buck sweep", "Sweep"),
    ("toss", "Sweep"),
    ("jet sweep", "Sweep"),
    ("option", "Option"),
];

fn scheme(r: &PlayRecord) -> &'static str {
    let c = r.call.to_lowercase();
    if c.contains("draw") {
        return "Draw";
    }
    if c.contains("sneak") {
        return "Sneak";
    }
    RUN_SCHEMES.iter().find(|(k, _)| c.contains(k)).map_or("Other runs", |x| x.1)
}

/// A pass rush call from the chart: how many came and whether two crossed.
fn rush_call(r: &PlayRecord) -> &'static str {
    match &r.line {
        Some(c) => {
            let n = c.rush.len();
            let stunt = c.rush.iter().any(|x| x.flags & Rusher::STUNT != 0);
            match n {
                0..=3 => "Three-man rush",
                4 if stunt => "Four with a stunt",
                4 => "Four-man rush",
                5 => "Five-man pressure",
                _ => "Six or more",
            }
        }
        None => {
            if r.blitz {
                "Pressure"
            } else {
                "Four-man rush"
            }
        }
    }
}

/// "2nd & 7 at OPP 35".
fn situation(r: &PlayRecord) -> String {
    let d = ["1st", "2nd", "3rd", "4th"][(r.down.clamp(1, 4) - 1) as usize];
    let togo = if r.togo >= r.yardline_100 - 0.5 { "goal".to_string() } else { format!("{}", r.togo.round().max(1.0)) };
    format!("{d} & {togo} at {}", views::spot_text(r.yardline_100))
}

/// Your staff's read of a few ratings: ours from practice, theirs from the
/// scouting file when there is one, else from the tape (the band narrows
/// with each game of film: about ±10 from one game, ±4 from six or more;
/// proposed, not confirmed).
fn ratings(w: &World, id: u32, ours: bool, keys: &[Attr], tape_games: usize) -> (Vec<Value>, String) {
    let Some(i) = views::person_index(w, id) else { return (Vec::new(), String::new()) };
    let p = &w.people[i].player;
    let reader = views::reader_key(w);
    if ours {
        let eye = views::team_of(w, &w.people[i]).map_or(10.0, |t| views::eye_for(w, t, p.position));
        let all = reads::attributes(p, true, eye, reader);
        let v = keys
            .iter()
            .map(|&a| {
                let r = all.iter().find(|x| x.0 == a).map_or(Read::NotSeen, |x| x.1);
                json!({ "k": a.label(), "key": format!("{a:?}"), "r": r.json() })
            })
            .collect();
        return (v, "from practice".into());
    }
    let file = readers::seat_staff(w).and_then(|s| readers::read(w, s, id));
    if let Some((_, looks, f)) = file {
        let v = keys
            .iter()
            .map(|&a| {
                let r = f.attrs.get(a.index()).and_then(|x| x.range()).map(|(lo, hi)| json!([lo, hi]));
                json!({ "k": a.label(), "key": format!("{a:?}"), "r": r })
            })
            .collect();
        return (v, format!("from your file, {} {}", looks.len(), if looks.len() == 1 { "look" } else { "looks" }));
    }
    let half = (10.0 - 1.2 * tape_games.saturating_sub(1) as f64).max(4.0);
    let v = keys
        .iter()
        .map(|&a| {
            let r = reads::range(p.attributes.get(a), half, reader ^ 0x7A9E, p.id, a.index());
            json!({ "k": a.label(), "key": format!("{a:?}"), "r": r.json() })
        })
        .collect();
    let games = tape_games.max(1);
    (v, format!("from {} {} of tape", games, if games == 1 { "game" } else { "games" }))
}

/// Games of film your staff has of a team (this one counts once).
fn tape_games(w: &World, team: usize, level: afm_engine::calendar::Level) -> usize {
    1 + w.gameday.tapes.iter().filter(|t| t.level == level && t.teams.contains(&team)).count()
}

/// The board for one of your games.
pub fn board(w: &World, k: usize) -> Reply {
    let g = w.seat_games().get(k).ok_or("That game is not kept.")?;
    let s = &g.stats;
    let mine = w.seat_team().and_then(|(_, t)| g.teams.iter().position(|&x| x == t));
    let us = mine.unwrap_or(0);
    let refs = [super::team_ref(g.level, g.teams[0]), super::team_ref(g.level, g.teams[1])];
    let who = Who { w, names: super::game_names(w, g, &refs) };
    let charted = s.plays.iter().any(|r| r.line.is_some());
    let (key, lname) = super::level_key(w, g.level, g.teams[us]);
    let now = super::season_now(w, g.level);
    let base = w.level_tally(key, now);
    let tapes = tape_games(w, g.teams[1 - us], g.level);
    let ctx =
        Ctx { w, s, us, who: &who, base: base.map(|x| x.1), lname: &lname, tapes, staff: line_coaches(w, refs[us]) };
    Ok(json!({
        "index": k,
        "names": g.names,
        "teams": [views::team_brief(w, refs[0]), views::team_brief(w, refs[1])],
        "us": us,
        "mine": mine,
        "charted": charted,
        "level": lname,
        "baseSeason": base.map(|x| x.0),
        "modes": {
            "pass": ctx.mode(Mode::Pass),
            "run": ctx.mode(Mode::Run),
            "rush": ctx.mode(Mode::Rush),
        },
    }))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Pass,
    Run,
    Rush,
}

struct Ctx<'a> {
    w: &'a World,
    s: &'a GameStats,
    us: usize,
    who: &'a Who<'a>,
    base: Option<afm_engine::world::charts::Tally>,
    lname: &'a str,
    tapes: usize,
    /// Your line coach and your defensive line coach: (name, role, id).
    staff: [Option<(String, &'static str, usize)>; 2],
}

/// Your offensive and defensive line coaches (the coordinator or head
/// coach where there is none).
fn line_coaches(w: &World, t: TeamRef) -> [Option<(String, &'static str, usize)>; 2] {
    let pick = |off: bool| -> Option<(String, &'static str, usize)> {
        let (c, role) = match t {
            TeamRef::College(p) => {
                let pr = &w.programs[p];
                let g = if off { Group::OL } else { Group::DL };
                match pr.position_coaches[g as usize] {
                    Some(c) => (c, if off { "offensive line coach" } else { "defensive line coach" }),
                    None => match if off { pr.oc } else { pr.dc } {
                        Some(c) => (c, if off { "offensive coordinator" } else { "defensive coordinator" }),
                        None => (pr.head_coach?, "head coach"),
                    },
                }
            }
            TeamRef::Nfl(x) => {
                let tm = &w.nfl[x];
                match if off { tm.oc } else { tm.dc } {
                    Some(c) => (c, if off { "offensive coordinator" } else { "defensive coordinator" }),
                    None => (tm.head_coach?, "head coach"),
                }
            }
            TeamRef::HighSchool(h) => (w.high_schools[h].coach?, "head coach"),
            TeamRef::Other(_) => return None,
        };
        if w.career.as_ref().is_some_and(|k| k.coach == c) {
            return None;
        }
        Some((w.coaches.get(c)?.name.clone(), role, c))
    };
    [pick(true), pick(false)]
}

/// Where a man blocked most: his spot on the chart (0 left tackle to 4
/// right tackle, 5 a tight end, 6 a back), else his position's.
fn spot_of(list: &[(usize, Seen)], pos: &str) -> u8 {
    let mut n = [0usize; 8];
    for x in list {
        if let Some(b) = x.1.bpos {
            n[(b as usize).min(7)] += 1;
        }
    }
    match (0..8).max_by_key(|&k| (n[k], std::cmp::Reverse(k))) {
        Some(k) if n[k] > 0 => k as u8,
        _ => match pos {
            "OT" => 0,
            "IOL" => 2,
            "TE" => 5,
            "RB" => 6,
            _ => 7,
        },
    }
}

const SPOTS: [&str; 8] = ["LT", "LG", "C", "RG", "RT", "TE", "RB", ""];

impl Ctx<'_> {
    fn mode(&self, m: Mode) -> Value {
        let w = self.w;
        let us = self.us;
        let them = 1 - us;
        // The snaps: our dropbacks, our runs, or their dropbacks.
        let snaps: Vec<&PlayRecord> = self
            .s
            .plays
            .iter()
            .filter(|r| !r.nullified && r.offense < 2)
            .filter(|r| match m {
                Mode::Pass => r.offense == us && dropback(r),
                Mode::Run => r.offense == us && r.kind == PlayType::Run,
                Mode::Rush => r.offense == them && dropback(r),
            })
            .collect();
        let pass = m != Mode::Run;
        // Every rep, from our side: (snap index, our man, their man, out).
        let mut reps: Vec<(usize, Seen)> = Vec::new();
        for (k, r) in snaps.iter().enumerate() {
            for x in reps_of(r, pass) {
                reps.push((k, x));
            }
        }
        // Our man in a rep (the blocker, or our rusher on defense).
        let our = |x: &Seen| if m == Mode::Rush { x.defender } else { x.blocker };
        let their = |x: &Seen| if m == Mode::Rush { x.blocker } else { x.defender };
        let out_ours = |x: &Seen| if m == Mode::Rush { x.out.flip() } else { x.out };
        // Who got home on a snap (pressure, hit or sack).
        let home = |r: &PlayRecord| -> Vec<u32> {
            let mut v: Vec<u32> = r.pressures.iter().chain(&r.hits).chain(&r.sackers).copied().collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        // Rows: our men by where they line up, with the most reps.
        let mut by_man: HashMap<u32, Vec<(usize, Seen)>> = HashMap::new();
        for &(k, x) in &reps {
            by_man.entry(our(&x)).or_default().push((k, x));
        }
        let mut men: Vec<u32> = by_man.keys().copied().collect();
        men.sort_by_key(|&id| (spot_of(&by_man[&id], &self.who.pos(id)), std::cmp::Reverse(by_man[&id].len()), id));
        // A rusher on defense: the line first, then by reps.
        if m == Mode::Rush {
            men.sort_by_key(|&id| (std::cmp::Reverse(by_man[&id].len()), id));
        }
        men.truncate(9);
        let rows: Vec<Value> = men
            .iter()
            .map(|&id| {
                let list = &by_man[&id];
                let mut cells: Vec<Value> = vec![Value::Null; snaps.len()];
                for (k, r) in snaps.iter().enumerate() {
                    let mine: Vec<&Seen> = list.iter().filter(|x| x.0 == k).map(|x| &x.1).collect();
                    if mine.is_empty() {
                        continue;
                    }
                    // His worst rep of the snap.
                    let worst = mine.iter().max_by_key(|x| out_ours(x).code()).unwrap();
                    let vs = their(worst);
                    let home_now = home(r);
                    let got = if m == Mode::Rush { home_now.contains(&id) } else { home_now.contains(&vs) };
                    cells[k] = json!({
                        "o": out_ours(worst).code(),
                        "vs": vs,
                        "home": got,
                        "mv": worst.mv.map(|v| move_name(w, worst.defender, v, r.line.as_deref())),
                    });
                }
                let (won, even, lost) = count(list.iter().map(|x| out_ours(&x.1)));
                let n = list.len().max(1) as f64;
                // His most frequent opponent.
                let mut opp: HashMap<u32, usize> = HashMap::new();
                for x in list {
                    *opp.entry(their(&x.1)).or_default() += 1;
                }
                let mut opps: Vec<(u32, usize)> = opp.into_iter().collect();
                opps.sort_by_key(|x| (std::cmp::Reverse(x.1), x.0));
                // His spot when the line was charted, else his position.
                let charted = list.iter().any(|x| x.1.bpos.is_some());
                let spot = spot_of(list, &self.who.pos(id));
                json!({
                    "id": id,
                    "name": self.who.name(id),
                    "pos": self.who.pos(id),
                    "spot": if m == Mode::Rush || !charted { self.who.pos(id) } else { SPOTS[spot as usize].to_string() },
                    "cls": self.who.person(id).map(|i| views::class_text(w, &w.people[i])),
                    "read": self.overall(id, true),
                    "cells": cells,
                    "won": won, "even": even, "lost": lost,
                    "pct": r1(100.0 * won as f64 / n),
                    "lostPct": r1(100.0 * lost as f64 / n),
                    "opps": opps.iter().take(3).map(|x| json!({ "id": x.0, "name": self.who.name(x.0), "pos": self.who.pos(x.0), "reps": x.1 })).collect::<Vec<_>>(),
                    "hurt": self.hurt(id),
                })
            })
            .collect();
        // The matchups each row's men had, for the side panel.
        let mut pairs: Vec<Value> = Vec::new();
        for &id in &men {
            let mut opp: HashMap<u32, usize> = HashMap::new();
            for x in &by_man[&id] {
                *opp.entry(their(&x.1)).or_default() += 1;
            }
            let mut opps: Vec<(u32, usize)> = opp.into_iter().collect();
            opps.sort_by_key(|x| (std::cmp::Reverse(x.1), x.0));
            for &(o, _) in opps.iter().take(3) {
                pairs.push(self.pair(m, id, o, &snaps, &by_man[&id], &their, &out_ours));
            }
        }
        json!({
            "key": match m { Mode::Pass => "pass", Mode::Run => "run", Mode::Rush => "rush" },
            "cells": self.cells(m, &snaps, &reps, &out_ours),
            "snaps": snaps.iter().enumerate().map(|(k, r)| self.snap(m, k, r, &reps, &our, &their, &out_ours)).collect::<Vec<_>>(),
            "rows": rows,
            "front": self.front(m, &men, &by_man, &snaps, &their, &out_ours),
            "calls": self.calls(m, &snaps, &reps, &out_ours),
            "pairs": pairs,
            "half": snaps.iter().position(|r| r.quarter >= 3),
            "quarters": (1..=5u8).map(|q| snaps.iter().position(|r| r.quarter == q || (q == 5 && r.quarter > 4))).collect::<Vec<_>>(),
        })
    }

    fn overall(&self, id: u32, ours: bool) -> Value {
        let Some(i) = self.who.person(id) else { return Value::Null };
        let p = &self.w.people[i].player;
        let eye = if ours {
            views::team_of(self.w, &self.w.people[i]).map_or(10.0, |t| views::eye_for(self.w, t, p.position))
        } else {
            10.0
        };
        reads::overall(p, ours, eye, views::reader_key(self.w)).json()
    }

    /// Hurt in this game.
    fn hurt(&self, id: u32) -> Option<String> {
        self.s.plays.iter().flat_map(|r| &r.injuries).find(|x| x.id == id).map(|x| x.kind.name().to_string())
    }

    /// The six numbers across the top.
    fn cells(&self, m: Mode, snaps: &[&PlayRecord], reps: &[(usize, Seen)], out: &dyn Fn(&Seen) -> Out) -> Vec<Value> {
        let n = snaps.len();
        let lost = reps.iter().filter(|x| out(&x.1) == Out::Lost).count();
        let won = reps.iter().filter(|x| out(&x.1) == Out::Won).count();
        let pct = |a: usize, b: usize| if b > 0 { r1(100.0 * a as f64 / b as f64) } else { 0.0 };
        let base = self.base.as_ref();
        let lvl = self.lname;
        match m {
            Mode::Pass | Mode::Rush => {
                let pressured = snaps.iter().filter(|r| r.pressured || r.kind == PlayType::Sack).count();
                let sacks = snaps.iter().filter(|r| r.kind == PlayType::Sack).count();
                let throws: Vec<&&PlayRecord> = snaps.iter().filter(|r| r.kind == PlayType::Pass).collect();
                let ttt = if throws.is_empty() {
                    0.0
                } else {
                    throws.iter().map(|r| r.time_to_throw).sum::<f64>() / throws.len() as f64
                };
                let lp = base.map(|b| r1(100.0 * b.pressured as f64 / b.dropbacks.max(1) as f64));
                let lt =
                    base.filter(|b| b.throws > 0).map(|b| (b.ttt as f64 / b.throws as f64 * 100.0).round() / 100.0);
                let ls = base.map(|b| r1(100.0 * b.sacks as f64 / b.dropbacks.max(1) as f64));
                // A blitz: five or more rushers. Picked up: every rusher was
                // blocked, his man at the snap or caught in a block after it
                // (a slide's linemen take whoever comes to them).
                let blitz: Vec<&&PlayRecord> =
                    snaps.iter().filter(|r| r.line.as_ref().map_or(r.blitz, |c| c.rush.len() >= 5)).collect();
                let picked = blitz
                    .iter()
                    .filter(|r| {
                        r.line.as_ref().is_some_and(|c| {
                            c.rush
                                .iter()
                                .all(|x| x.flags & Rusher::PICKED != 0 || c.reps.iter().any(|p| p.defender == x.id))
                        })
                    })
                    .count();
                let stunts: Vec<&&PlayRecord> = snaps
                    .iter()
                    .filter(|r| r.line.as_ref().is_some_and(|c| c.rush.iter().any(|x| x.flags & Rusher::STUNT != 0)))
                    .collect();
                let stunt_ok = stunts.iter().filter(|r| !r.pressured && r.kind != PlayType::Sack).count();
                let ours = m == Mode::Pass;
                let mut v = vec![
                    json!({ "k": if ours { "Pressure allowed" } else { "Pressure made" }, "v": format!("{}%", pct(pressured, n)), "sub": format!("{pressured} of {n}{}", lp.map_or(String::new(), |x| format!(" · {lvl} {x}%"))), "bad": ours && lp.is_some_and(|x| pct(pressured, n) > x + 5.0) || !ours && lp.is_some_and(|x| pct(pressured, n) < x - 5.0) }),
                    json!({ "k": if ours { "Sacks allowed" } else { "Sacks" }, "v": sacks, "sub": format!("{}% of dropbacks{}", pct(sacks, n), ls.map_or(String::new(), |x| format!(" · {lvl} {x}%"))), "bad": ours && sacks >= 4 }),
                    json!({ "k": if ours { "Time to throw" } else { "Their throw time" }, "v": format!("{:.2} s", ttt), "sub": lt.map_or("No level numbers yet".into(), |x| format!("{lvl} {x:.2} s")), "bad": false }),
                ];
                if ours {
                    // A stunt is handled when it gets no pressure.
                    let free = blitz.len() - picked;
                    let blitz_sub = match (self.charted(snaps), blitz.len(), free) {
                        (false, _, _) => "Not charted".to_string(),
                        (_, 0, _) => "no blitzes".into(),
                        (_, _, 0) => "every rusher blocked".into(),
                        (_, _, 1) => "one with a man left free".into(),
                        _ => format!("{free} with a man left free"),
                    };
                    let hurt = stunts.len() - stunt_ok;
                    let stunt_sub = match (stunts.len(), hurt) {
                        (0, _) => "no stunts".to_string(),
                        (_, 0) => "no pressure off the twist".into(),
                        (_, 1) => "one got pressure".into(),
                        _ => format!("{hurt} got pressure"),
                    };
                    v.push(json!({ "k": "Blitzes picked up", "v": format!("{picked}/{}", blitz.len()), "sub": blitz_sub, "bad": blitz.len() >= 3 && picked * 2 < blitz.len() }));
                    v.push(json!({ "k": "Stunts handled", "v": format!("{stunt_ok}/{}", stunts.len()), "sub": stunt_sub, "bad": stunts.len() >= 3 && stunt_ok * 2 < stunts.len() }));
                    v.push(json!({ "k": "Lost reps", "v": lost, "sub": format!("{}% of {} reps", pct(lost, reps.len()), reps.len()), "bad": pct(lost, reps.len()) > 12.0 }));
                } else {
                    v.push(json!({ "k": "Blitzes", "v": blitz.len(), "sub": format!("{}% of their dropbacks", pct(blitz.len(), n)), "bad": false }));
                    v.push(json!({ "k": "Won reps", "v": won, "sub": format!("{}% of {} reps", pct(won, reps.len()), reps.len()), "bad": false }));
                }
                v
            }
            Mode::Run => {
                let bc = if n > 0 { snaps.iter().map(|r| r.before_contact).sum::<f64>() / n as f64 } else { 0.0 };
                let yds = if n > 0 { snaps.iter().map(|r| r.yards).sum::<f64>() / n as f64 } else { 0.0 };
                let stuffed = snaps.iter().filter(|r| r.yards <= 0.0).count();
                let ten = snaps.iter().filter(|r| r.yards >= 10.0).count();
                let lb = base.filter(|b| b.runs > 0).map(|b| r1(b.before as f64 / b.runs as f64));
                let ly = base.filter(|b| b.runs > 0).map(|b| r1(b.run_yds as f64 / b.runs as f64));
                let lst = base.filter(|b| b.runs > 0).map(|b| r1(100.0 * b.stuffed as f64 / b.runs as f64));
                let l10 = base.filter(|b| b.runs > 0).map(|b| r1(100.0 * b.ten as f64 / b.runs as f64));
                let lv = |x: Option<f64>, unit: &str| x.map_or(String::new(), |x| format!("{} {x}{unit}", self.lname));
                vec![
                    json!({ "k": "Before contact", "v": format!("{:.1} yds", bc), "sub": lv(lb, " yds"), "bad": lb.is_some_and(|x| bc < x - 0.6) }),
                    json!({ "k": "Stuffed runs", "v": stuffed, "sub": format!("{}% of {n}{}", pct(stuffed, n), lst.map_or(String::new(), |x| format!(" · {} {x}%", self.lname))), "bad": lst.is_some_and(|x| pct(stuffed, n) > x + 6.0) }),
                    json!({ "k": "Runs of 10+", "v": ten, "sub": format!("{}%{}", pct(ten, n), l10.map_or(String::new(), |x| format!(" · {} {x}%", self.lname))), "bad": false }),
                    json!({ "k": "Yards a run", "v": format!("{:.1}", yds), "sub": lv(ly, ""), "bad": ly.is_some_and(|x| yds < x - 1.0) }),
                    json!({ "k": "Lost reps", "v": lost, "sub": format!("{}% of {} reps", pct(lost, reps.len()), reps.len()), "bad": pct(lost, reps.len()) > 15.0 }),
                    json!({ "k": "Won reps", "v": won, "sub": format!("{}% driven or sealed", pct(won, reps.len())), "bad": false }),
                ]
            }
        }
    }

    fn charted(&self, snaps: &[&PlayRecord]) -> bool {
        snaps.iter().any(|r| r.line.is_some())
    }

    /// One snap for the play card.
    #[allow(clippy::too_many_arguments)]
    fn snap(
        &self,
        m: Mode,
        k: usize,
        r: &PlayRecord,
        reps: &[(usize, Seen)],
        our: &dyn Fn(&Seen) -> u32,
        their: &dyn Fn(&Seen) -> u32,
        out: &dyn Fn(&Seen) -> Out,
    ) -> Value {
        let name = |id: u32| self.who.name(id);
        let mine: Vec<&Seen> = reps.iter().filter(|x| x.0 == k).map(|x| &x.1).collect();
        let chips: Vec<Value> = mine
            .iter()
            .map(|x| {
                json!({
                    "id": our(x), "name": name(our(x)), "pos": self.who.pos(our(x)),
                    "vs": their(x), "vsName": name(their(x)),
                    "o": out(x).code(), "out": out(x).word(),
                    "mv": x.mv.map(|v| move_name(self.w, x.defender, v, r.line.as_deref())),
                    "t": r1(x.secs), "double": x.double,
                })
            })
            .collect();
        let chart = r.line.as_deref();
        let facts: Vec<[String; 2]> = match m {
            Mode::Pass => vec![
                [
                    "Protection".into(),
                    chart
                        .map_or("Not charted".into(), |c| format!("{} · {} blocking", c.protection.name(), c.blockers)),
                ],
                [
                    "Rushers".into(),
                    chart.map_or_else(
                        || if r.blitz { "Pressure".into() } else { "Four".into() },
                        |c| c.rush.len().to_string(),
                    ),
                ],
                [
                    "Time to throw".into(),
                    if r.kind == PlayType::Pass { format!("{:.1} s", r.time_to_throw) } else { "No throw".into() },
                ],
                [
                    "Pocket".into(),
                    if r.kind == PlayType::Sack {
                        "Sacked".into()
                    } else if r.pressured {
                        "Under pressure".into()
                    } else {
                        "Clean".into()
                    },
                ],
            ],
            Mode::Run => vec![
                ["Scheme".into(), scheme(r).into()],
                ["Hole".into(), ["Left end", "Left tackle", "Middle", "Right tackle", "Right end"][hole(r)].into()],
                ["Before contact".into(), format!("{:.1} yds", r.before_contact)],
                ["Runner".into(), r.carrier.map_or("Not known".into(), name)],
            ],
            Mode::Rush => vec![
                ["Our call".into(), rush_call(r).into()],
                ["Rushers".into(), chart.map_or("Not charted".into(), |c| c.rush.len().to_string())],
                [
                    "Their throw time".into(),
                    if r.kind == PlayType::Pass { format!("{:.1} s", r.time_to_throw) } else { "No throw".into() },
                ],
                [
                    "For us".into(),
                    if r.kind == PlayType::Sack {
                        "Sack".into()
                    } else if r.pressured {
                        "Pressure".into()
                    } else {
                        "Clean pocket".into()
                    },
                ],
            ],
        };
        let home: Vec<u32> = r.pressures.iter().chain(&r.sackers).copied().collect();
        json!({
            "n": r.n,
            "q": super::quarter(r.quarter),
            "clock": super::clock(r.clock),
            "sit": situation(r),
            "text": views::play_text(r, &name),
            "yds": r1(r.yards),
            "to": r.turnover,
            "pr": r.pressured || r.kind == PlayType::Sack,
            "sack": r.kind == PlayType::Sack,
            "home": home,
            "facts": facts,
            "chips": chips,
        })
    }

    /// The front: our men across the line with the man each mostly faced,
    /// and the gaps.
    fn front(
        &self,
        m: Mode,
        men: &[u32],
        by_man: &HashMap<u32, Vec<(usize, Seen)>>,
        snaps: &[&PlayRecord],
        their: &dyn Fn(&Seen) -> u32,
        out: &dyn Fn(&Seen) -> Out,
    ) -> Value {
        let links: Vec<Value> = men
            .iter()
            .enumerate()
            .map(|(k, &id)| {
                let list = &by_man[&id];
                let mut opp: HashMap<u32, usize> = HashMap::new();
                for x in list {
                    *opp.entry(their(&x.1)).or_default() += 1;
                }
                let main = opp.into_iter().max_by_key(|x| (x.1, std::cmp::Reverse(x.0))).map(|x| x.0);
                let vs: Vec<&(usize, Seen)> = list.iter().filter(|x| Some(their(&x.1)) == main).collect();
                let won = vs.iter().filter(|x| out(&x.1) == Out::Won).count();
                let lost = vs.iter().filter(|x| out(&x.1) == Out::Lost).count();
                let pct = |a: usize| if vs.is_empty() { 0.0 } else { r1(100.0 * a as f64 / vs.len() as f64) };
                json!({
                    "a": id, "aName": self.who.name(id), "aPos": self.who.pos(id),
                    "b": main, "bName": main.map(|x| self.who.name(x)), "bPos": main.map(|x| self.who.pos(x)),
                    "reps": vs.len(), "pct": pct(won), "lost": pct(lost),
                    "slot": if m == Mode::Rush { k as u8 } else { spot_of(list, &self.who.pos(id)) },
                })
            })
            .collect();
        // The gaps: where pressure came through (pass), or yards a run
        // through each hole.
        let gaps: Vec<Value> = match m {
            Mode::Run => {
                let mut bins = [(0u32, 0.0f64); 5];
                for r in snaps {
                    let h = hole(r);
                    bins[h].0 += 1;
                    bins[h].1 += r.yards;
                }
                bins.iter().map(|&(n, y)| json!({ "n": n, "v": if n > 0 { r1(y / n as f64) } else { 0.0 } })).collect()
            }
            _ => {
                let mut bins = [0u32; 5];
                for r in snaps {
                    let Some(c) = r.line.as_deref() else { continue };
                    let first = r.sackers.first().or(r.pressures.first()).or(r.hits.first());
                    if let Some(g) = first.and_then(|&id| c.gap_of(id)) {
                        bins[g as usize] += 1;
                    }
                }
                bins.iter().map(|&n| json!({ "n": n, "v": n })).collect()
            }
        };
        json!({ "links": links, "gaps": gaps })
    }

    /// The calls table: protections, run schemes or our rush calls.
    fn calls(&self, m: Mode, snaps: &[&PlayRecord], reps: &[(usize, Seen)], out: &dyn Fn(&Seen) -> Out) -> Value {
        let mut groups: Vec<(&'static str, Vec<usize>)> = Vec::new();
        for (k, r) in snaps.iter().enumerate() {
            let key: &'static str = match m {
                Mode::Pass => r.line.as_deref().map_or("Not charted", |c| match c.protection {
                    Protection::None => "Man",
                    p => p.name(),
                }),
                Mode::Run => scheme(r),
                Mode::Rush => rush_call(r),
            };
            match groups.iter_mut().find(|g| g.0 == key) {
                Some(g) => g.1.push(k),
                None => groups.push((key, vec![k])),
            }
        }
        groups.sort_by_key(|g| std::cmp::Reverse(g.1.len()));
        let pct = |a: usize, b: usize| if b > 0 { (100.0 * a as f64 / b as f64).round() } else { 0.0 };
        let rows: Vec<Value> = groups
            .iter()
            .map(|(name, ks)| {
                let n = ks.len();
                let lost = reps.iter().filter(|x| ks.contains(&x.0) && out(&x.1) == Out::Lost).count();
                match m {
                    Mode::Pass => {
                        let pr = ks.iter().filter(|&&k| snaps[k].pressured || snaps[k].kind == PlayType::Sack).count();
                        json!([name, n, format!("{pr} ({}%)", pct(pr, n)), lost])
                    }
                    Mode::Run => {
                        let y = ks.iter().map(|&k| snaps[k].yards).sum::<f64>() / n.max(1) as f64;
                        let st = ks.iter().filter(|&&k| snaps[k].yards <= 0.0).count();
                        json!([name, n, format!("{:.1}", y), st])
                    }
                    Mode::Rush => {
                        let pr = ks.iter().filter(|&&k| snaps[k].pressured || snaps[k].kind == PlayType::Sack).count();
                        let sk = ks.iter().filter(|&&k| snaps[k].kind == PlayType::Sack).count();
                        json!([name, n, format!("{pr} ({}%)", pct(pr, n)), sk])
                    }
                }
            })
            .collect();
        let (title, cols) = match m {
            Mode::Pass => ("Protection calls", ["Call", "Plays", "Pressure", "Lost reps"]),
            Mode::Run => ("Run calls", ["Scheme", "Runs", "Yards a run", "Stuffed"]),
            Mode::Rush => ("Our rush calls", ["Call", "Plays", "Pressure", "Sacks"]),
        };
        // A plain note from the best and worst of them.
        let note = match m {
            Mode::Pass | Mode::Rush if !groups.is_empty() => {
                let rate = |ks: &Vec<usize>| {
                    ks.iter().filter(|&&k| snaps[k].pressured || snaps[k].kind == PlayType::Sack).count() as f64
                        / ks.len().max(1) as f64
                };
                let many: Vec<&(&str, Vec<usize>)> = groups.iter().filter(|g| g.1.len() >= 3).collect();
                let worst = many.iter().max_by(|a, b| rate(&a.1).total_cmp(&rate(&b.1)));
                let best = many.iter().min_by(|a, b| rate(&a.1).total_cmp(&rate(&b.1)));
                match (best, worst) {
                    (Some(b), Some(x)) if b.0 != x.0 && m == Mode::Pass => format!(
                        "{} held up best ({}% pressure); {} gave up the most ({}%).",
                        b.0,
                        (100.0 * rate(&b.1)).round(),
                        x.0.to_lowercase(),
                        (100.0 * rate(&x.1)).round()
                    ),
                    (Some(b), Some(x)) if b.0 != x.0 => format!(
                        "{} got home most ({}% pressure); {} least ({}%).",
                        x.0,
                        (100.0 * rate(&x.1)).round(),
                        b.0.to_lowercase(),
                        (100.0 * rate(&b.1)).round()
                    ),
                    _ => format!("{} snaps, most of them on one call.", snaps.len()),
                }
            }
            Mode::Run if !groups.is_empty() => {
                let ypc = |ks: &Vec<usize>| ks.iter().map(|&k| snaps[k].yards).sum::<f64>() / ks.len().max(1) as f64;
                let many: Vec<&(&str, Vec<usize>)> = groups.iter().filter(|g| g.1.len() >= 3).collect();
                match many.iter().max_by(|a, b| ypc(&a.1).total_cmp(&ypc(&b.1))) {
                    Some(b) => format!("{} worked best: {:.1} yards a run on {}.", b.0, ypc(&b.1), b.1.len()),
                    None => format!("{} runs, no scheme called three times.", snaps.len()),
                }
            }
            _ => String::new(),
        };
        json!({ "title": title, "cols": cols, "rows": rows, "note": note })
    }

    /// One matchup for the side panel.
    #[allow(clippy::too_many_arguments)]
    fn pair(
        &self,
        m: Mode,
        a: u32,
        b: u32,
        snaps: &[&PlayRecord],
        list: &[(usize, Seen)],
        their: &dyn Fn(&Seen) -> u32,
        out: &dyn Fn(&Seen) -> Out,
    ) -> Value {
        let vs: Vec<&(usize, Seen)> = list.iter().filter(|x| their(&x.1) == b).collect();
        let n = vs.len().max(1) as f64;
        let (won, even, lost) = count(vs.iter().map(|x| out(&x.1)));
        let pct = |c: usize| r1(100.0 * c as f64 / n);
        let win_of = |f: &dyn Fn(&PlayRecord) -> bool| {
            let sub: Vec<&&(usize, Seen)> = vs.iter().filter(|x| f(snaps[x.0])).collect();
            if sub.is_empty() {
                Value::Null
            } else {
                json!(r1(100.0 * sub.iter().filter(|x| out(&x.1) == Out::Won).count() as f64 / sub.len() as f64))
            }
        };
        // Pass: the ratings that decide a pass rep; run: a run rep's.
        let (ka, kb): (&[Attr], &[Attr]) = match m {
            Mode::Pass => {
                (&[Attr::PassBlocking, Attr::Anchor, Attr::HandUse], &[Attr::PowerRush, Attr::FinesseRush, Attr::Speed])
            }
            Mode::Run => (
                &[Attr::RunBlocking, Attr::Strength, Attr::LegDrive],
                &[Attr::BlockShedding, Attr::Strength, Attr::Tackling],
            ),
            Mode::Rush => {
                (&[Attr::PowerRush, Attr::FinesseRush, Attr::Speed], &[Attr::PassBlocking, Attr::Anchor, Attr::HandUse])
            }
        };
        let (ra, fa) = ratings(self.w, a, true, ka, 0);
        let (rb, fb) = ratings(self.w, b, false, kb, self.tapes);
        // When he lost: how long the rep held, and whether the man got home
        // or the ball was out first.
        let lost_at: Vec<Value> = vs
            .iter()
            .filter(|x| out(&x.1) == Out::Lost)
            .map(|x| {
                let r = snaps[x.0];
                let rusher = if m == Mode::Rush { a } else { b };
                let got = r.pressures.contains(&rusher) || r.sackers.contains(&rusher) || r.hits.contains(&rusher);
                json!({ "t": r1(x.1.secs), "home": got, "n": r.n })
            })
            .collect();
        let avg_throw = {
            let t: Vec<f64> =
                vs.iter().map(|x| snaps[x.0]).filter(|r| r.kind == PlayType::Pass).map(|r| r.time_to_throw).collect();
            if t.is_empty() { Value::Null } else { json!(r1(t.iter().sum::<f64>() / t.len() as f64)) }
        };
        // The moves he faced (pass): lost of faced, by move.
        let mut moves: Vec<(&'static str, usize, usize)> = Vec::new();
        if m != Mode::Run {
            for x in &vs {
                let Some(v) = x.1.mv else { continue };
                let nm = move_name(self.w, x.1.defender, v, snaps[x.0].line.as_deref());
                let l = (out(&x.1) == Out::Lost) as usize;
                match moves.iter_mut().find(|y| y.0 == nm) {
                    Some(y) => {
                        y.1 += l;
                        y.2 += 1;
                    }
                    None => moves.push((nm, l, 1)),
                }
            }
            moves.sort_by_key(|y| std::cmp::Reverse(y.2));
        }
        // Run: yards a run by how the rep went.
        let by_out: Vec<Value> = if m == Mode::Run {
            [Out::Won, Out::Even, Out::Lost]
                .iter()
                .map(|&o| {
                    let ys: Vec<f64> = vs.iter().filter(|x| out(&x.1) == o).map(|x| snaps[x.0].yards).collect();
                    json!({ "k": o.word(), "n": ys.len(), "v": if ys.is_empty() { Value::Null } else { json!(r1(ys.iter().sum::<f64>() / ys.len() as f64)) } })
                })
                .collect()
        } else {
            Vec::new()
        };
        let watch: Vec<u32> = vs.iter().filter(|x| out(&x.1) == Out::Lost).map(|x| snaps[x.0].n).collect();
        let quote = self.quote(m, a, b, won, lost, vs.len(), &moves);
        json!({
            "a": a, "b": b,
            "aName": self.who.name(a), "bName": self.who.name(b),
            "aPos": self.who.pos(a), "bPos": self.who.pos(b),
            "reps": vs.len(), "won": won, "even": even, "lost": lost,
            "pct": pct(won), "evenPct": pct(even), "lostPct": pct(lost),
            "ra": ra, "rb": rb, "fromA": fa, "fromB": fb,
            "hurtA": self.hurt(a), "hurtB": self.hurt(b),
            "lostAt": lost_at, "avgThrow": avg_throw,
            "moves": moves.iter().map(|y| json!({ "k": y.0, "lost": y.1, "faced": y.2 })).collect::<Vec<_>>(),
            "byOut": by_out,
            "splits": [
                { "k": "1st half", "v": win_of(&|r: &PlayRecord| r.quarter <= 2) },
                { "k": "2nd half", "v": win_of(&|r: &PlayRecord| r.quarter >= 3) },
                { "k": "Early downs", "v": win_of(&|r: &PlayRecord| r.down <= 2) },
                { "k": "3rd, 4th down", "v": win_of(&|r: &PlayRecord| r.down >= 3) },
            ],
            "watch": watch,
            "quote": quote,
        })
    }

    /// What your line coach says about a matchup, from its numbers.
    #[allow(clippy::too_many_arguments)]
    fn quote(
        &self,
        m: Mode,
        a: u32,
        b: u32,
        won: usize,
        lost: usize,
        reps: usize,
        moves: &[(&'static str, usize, usize)],
    ) -> Value {
        let coach = if m == Mode::Rush { &self.staff[1] } else { &self.staff[0] };
        let Some((name, role, id)) = coach else { return Value::Null };
        let last = |id: u32| self.who.name(id).split_whitespace().last().unwrap_or("").to_string();
        let (me, him) = (last(a), last(b));
        let worst = moves.iter().filter(|y| y.1 > 0).max_by_key(|y| y.1);
        let text = if reps == 0 {
            String::new()
        } else if m == Mode::Rush {
            match lost {
                0 if won * 3 >= reps => {
                    format!("{me} owned {him} today: {won} wins in {reps} reps. Keep him on that side.")
                }
                0 => format!("{me} never lost to {him}, but he didn't win enough either: {won} of {reps}."),
                _ => format!("{him} stoned {me} {lost} times. We need a second move when the first one stalls."),
            }
        } else if lost == 0 {
            format!("{me} didn't lose a rep to {him} in {reps}. That's the standard.")
        } else if let Some(x) = worst {
            format!(
                "{him} beat {me} {} of {} times with the {}. We'll drill the answer to it this week.",
                x.1,
                x.2,
                x.0.to_lowercase()
            )
        } else {
            format!("{me} lost {lost} of {reps} reps to {him}. Too many; we'll look at the film together.")
        };
        json!({ "text": text, "name": name, "role": role, "id": id })
    }
}

fn count(it: impl Iterator<Item = Out>) -> (usize, usize, usize) {
    let mut c = (0, 0, 0);
    for o in it {
        match o {
            Out::Won => c.0 += 1,
            Out::Even => c.1 += 1,
            Out::Lost => c.2 += 1,
        }
    }
    c
}
