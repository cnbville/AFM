//! The offer chain at your college (scouting update SK25-SK31).
//!
//! - **The chain (SK26).** Every Monday each position coach brings you the
//!   recruits he wants offered: the best of his group on your program's
//!   board by your staff's read, down to the line his position's open spots
//!   draw, at most two a week. His side's coordinator watches the film
//!   first (a film look in the file), and the recommendation reaches you
//!   once his look is written up. Nothing goes out without you: you sign
//!   or decline each one, or sign the whole list. With the board handed to
//!   your staff, their offers and pulls wait on the same list.
//! - **Your film review (SK25).** Every offer you make or sign carries a
//!   film look of your own in his file.
//! - **The final word (SK28).** You can offer anyone, a kid your staff
//!   passed on too, and pull anyone.
//! - **Overruling (SK29).** Going against a coach is kept with his read
//!   and how hard he argued. His trust in you dips, more the harder he
//!   argued. Three years on the look-back settles it: if you were right,
//!   his respect for your eye grows; if he was, it costs more. A coach
//!   overruled again and again works his area with less heart (fewer
//!   calls) and listens to other jobs (`staff::weigh_offer`).
//! - **His record (SK30).** Every call he made, offered or passed, with
//!   how the kid turned out, for his coach page.
//! - **Passed on (SK31).** Every recruit your staff passes on whom the
//!   services rate (four stars and up, or above the kids they want) goes
//!   to a list with who passed, when and why; the ones where the public
//!   and your staff disagree most are flagged, both ways.
//!
//! Proposed, not confirmed: an FBS staff keeps about four live offers for
//! each open spot at a position (FCS three, below that two and a half); a
//! coach argues from 1 to 20 by how far his read clears the line; the
//! look-back calls a kid a hit when he starts at your level or higher, or
//! is drafted.

use std::collections::HashMap;

use super::college as desk;
use super::{Duty, Owner, indices, note, tell};
use crate::calendar::{Date, Weekday};
use crate::player::PlayingTime;
use crate::positions::{POSITION_COUNT, Position};
use crate::world::coach::Group;
use crate::world::data::Division;
use crate::world::folk::coach_key;
use crate::world::recruiting::{self as rec, recruiting_stars};
use crate::world::scouting::readers::{self, COACH, Staff};
use crate::world::scouting::{self, Kind, dept, lines};
use crate::world::{Stage, World};

/// Your word on a recommendation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Answer {
    #[default]
    Waiting,
    Signed,
    Declined,
    /// He committed or signed elsewhere before you answered.
    Lapsed,
}

/// A coach's recommendation on your list.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Rec {
    pub id: u32,
    pub coach: usize,
    pub date: Date,
    /// The day it reaches you: after the coordinator's film look is written.
    pub ready: Date,
    /// The coordinator who watched his film.
    pub film: Option<usize>,
    /// A pull of a live offer (a staff-run board), not an offer.
    pub pull: bool,
    /// Your staff's read of him that day, and the position's line.
    pub read: (u8, u8),
    pub line: u8,
    /// How hard he argues, 1-20.
    pub conviction: u8,
    pub why: String,
    /// His read is far above the public's word on him.
    pub gem: bool,
    pub answer: Answer,
    pub answered: Option<Date>,
}

/// One call on a coach's record.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Call {
    pub id: u32,
    pub coach: usize,
    pub date: Date,
    /// He wanted him offered (false: he passed on him).
    pub wanted: bool,
    pub conviction: u8,
    /// You went the other way.
    pub overruled: bool,
    /// Three years on: whether the coach was right.
    pub settled: Option<bool>,
    /// How the kid turned out, in words.
    pub turned: Option<String>,
}

/// A recruit your staff passed on.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Passed {
    pub id: u32,
    pub coach: usize,
    pub date: Date,
    pub why: String,
    pub stars: u8,
    pub read: (u8, u8),
    /// The public and your staff disagree most on him.
    pub flag: bool,
    /// You put him back on the board.
    #[serde(default)]
    pub back: bool,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Chain {
    pub program: Option<usize>,
    pub recs: Vec<Rec>,
    pub calls: Vec<Call>,
    pub passed: Vec<Passed>,
}

/// Live offers an FBS staff keeps for each open spot (see the module doc).
fn offers_per_spot(d: Division) -> f64 {
    match d {
        Division::FBS => 4.0,
        Division::FCS => 3.0,
        _ => 2.5,
    }
}

/// The best stars a program recruits with a real chance (a blue blood any
/// kid; a small program three stars and down).
fn reach(prestige: f64) -> u8 {
    (2.0 + (3.0 * prestige).ceil()).clamp(3.0, 5.0) as u8
}

/// His side's coordinator: offense, defense or the kicking game.
fn coordinator_for(w: &World, j: usize, pos: Position) -> Option<usize> {
    let p = &w.programs[j];
    let g = Group::of(pos);
    match g {
        Group::Specialists => p.position_coaches[g as usize],
        _ if g.offense() => p.oc,
        _ => p.dc,
    }
    .or(p.head_coach)
}

/// What stands out in a read, best and worst, weighed by the position.
fn strengths(f: &scouting::file::FileRead, pos: Position) -> (String, String) {
    let mut v: Vec<(f64, crate::attributes::Attr)> = crate::attributes::Attr::ALL
        .iter()
        .map(|&a| (crate::positions::weight(pos, a) * (f.centres.get(a) - f.now.0), a))
        .filter(|x| x.0.abs() > 0.0)
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    let best = v.first().map_or("", |x| lines::attr_phrase(x.1, true));
    let worst = v.last().map_or("", |x| lines::attr_phrase(x.1, false));
    (best.to_string(), worst.to_string())
}

fn range(r: (f64, f64)) -> (u8, u8) {
    scouting::yours::range(r)
}

fn coach_name(w: &World, c: usize) -> String {
    w.coaches.get(c).map_or_else(|| "Your staff".to_string(), |c| c.name.clone())
}

/// The program the chain runs for: your college job.
fn program(w: &World) -> Option<usize> {
    super::program(w)
}

/// The week's work: Mondays.
pub(crate) fn day(w: &mut World, j: usize, d: Date) {
    if w.front_office.chain.program != Some(j) {
        let calls = std::mem::take(&mut w.front_office.chain.calls);
        w.front_office.chain = Chain { program: Some(j), calls, ..Default::default() };
    }
    if d.parts().1 == 9 && d.parts().2 == 1 {
        settle(w, d);
    }
    if d.weekday() != Weekday::Mon {
        return;
    }
    lapse(w, j);
    let ready_now = w.front_office.chain.recs.iter().filter(|r| r.answer == Answer::Waiting && r.ready <= d).count();
    let added = recommend(w, j, d);
    let n = w.front_office.chain.recs.iter().filter(|r| r.answer == Answer::Waiting && r.ready <= d.add(7)).count();
    if added > 0 || (n > 0 && ready_now > 0) {
        let staff = w.front_office.owner(Duty::Board) == Owner::Staff;
        let text = if staff {
            format!(
                "Your recruiting staff has {} waiting for your signature: nothing goes out until you sign.",
                count(n, "offer or pull", "offers and pulls")
            )
        } else {
            format!(
                "Your position coaches bring you {}, each after his coordinator's film. Sign or decline each, or sign the list.",
                count(n, "recruit to offer", "recruits to offer")
            )
        };
        tell(w, d, text, false);
    }
}

fn count(n: usize, one: &str, many: &str) -> String {
    if n == 1 { format!("1 {one}") } else { format!("{n} {many}") }
}

/// Recommendations a kid has slipped away from: lapsed.
fn lapse(w: &mut World, j: usize) {
    let ids: Vec<u32> =
        w.front_office.chain.recs.iter().filter(|r| r.answer == Answer::Waiting).map(|r| r.id).collect();
    let at = indices(w, &ids);
    let gone: Vec<u32> = ids
        .into_iter()
        .filter(|id| {
            at.get(id).is_none_or(|&i| {
                let p = &w.people[i];
                p.cal.signed.is_some_and(|s| s.program != j)
                    || p.cal.commit.is_some_and(|c| c.0 != j)
                    || !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)
            })
        })
        .collect();
    for r in w.front_office.chain.recs.iter_mut() {
        if r.answer == Answer::Waiting && gone.contains(&r.id) && !r.pull {
            r.answer = Answer::Lapsed;
        }
    }
    // The list keeps a season of answers.
    let cut = w.today.add(-400);
    w.front_office.chain.recs.retain(|r| r.answer == Answer::Waiting || r.answered.is_none_or(|a| a >= cut));
    w.front_office.chain.passed.retain(|p| p.date >= cut);
}

/// The position coaches' recommendations and passes for the week.
fn recommend(w: &mut World, j: usize, d: Date) -> usize {
    let staff = Staff::Program(j);
    let outlook = rec::class_outlook(w, j);
    let sign_year = rec::next_class(w);
    let prog = &w.programs[j];
    let (div, top) = (prog.division, reach(prog.prestige));
    let per = offers_per_spot(div);
    let board = &w.front_office.college.board;
    let live: HashMap<u32, bool> = board.iter().map(|t| (t.id, t.offered.is_some() && t.pulled.is_none())).collect();
    let chain = &w.front_office.chain;
    let asked: std::collections::HashSet<u32> =
        chain.recs.iter().map(|r| r.id).chain(chain.passed.iter().map(|p| p.id)).collect();
    // Each position's recruits with a read: (centre, sd, person, stars).
    let mut by_pos: Vec<Vec<(f64, f64, usize, u8)>> = vec![Vec::new(); POSITION_COUNT];
    let mut offered_at = [0usize; POSITION_COUNT];
    for i in dept::watch(w, j) {
        let p = &w.people[i];
        if !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) || p.class_of > sign_year + 1 {
            continue;
        }
        if p.cal.signed.is_some() || p.cal.commit.is_some_and(|c| c.0 != j) {
            continue;
        }
        let pos = p.player.position.index();
        if live.get(&p.player.id) == Some(&true) {
            offered_at[pos] += 1;
        }
        let Some((_, _, f)) = readers::read(w, staff, p.player.id) else { continue };
        by_pos[pos].push((f.now.0, f.now.1, i, recruiting_stars(p)));
    }
    let waiting = |pos: usize| {
        chain
            .recs
            .iter()
            .filter(|r| r.answer == Answer::Waiting && !r.pull)
            .filter(|r| scouting::place(w, r.id).is_some_and(|i| w.people[i].player.position.index() == pos))
            .count()
    };
    let mut per_coach: HashMap<usize, usize> = HashMap::new();
    let mut new_recs: Vec<Rec> = Vec::new();
    let mut new_passed: Vec<Passed> = Vec::new();
    let mut pulls: Vec<Rec> = Vec::new();
    for (pos, list) in by_pos.iter_mut().enumerate() {
        if list.is_empty() {
            continue;
        }
        list.sort_by(|a, b| b.0.total_cmp(&a.0));
        let need = outlook.need[pos];
        let cap = (per * need as f64).round() as usize;
        let line = list[cap.clamp(1, list.len()) - 1].0;
        let mut slots = cap.saturating_sub(offered_at[pos] + waiting(pos));
        let half = list.len() / 2;
        for (rank, &(c, sd, i, stars)) in list.iter().enumerate() {
            let p = &w.people[i];
            let id = p.player.id;
            if asked.contains(&id) {
                continue;
            }
            let Some(coach) = dept::evaluator_for(w, j, i) else { continue };
            let on = live.get(&id) == Some(&true);
            let pos_name = p.player.position;
            if !on
                && need > 0
                && slots > 0
                && c >= line
                && stars <= top
                && per_coach.get(&coach).copied().unwrap_or(0) < 2
            {
                let Some((_, _, f)) = readers::read(w, staff, id) else { continue };
                let (best, worst) = strengths(&f, pos_name);
                let conviction = (8.0 + 2.5 * (c - line) / sd.max(2.0)).clamp(1.0, 20.0) as u8;
                let gem = stars <= 2 && rank * 4 < list.len();
                let why = format!(
                    "{}: {best}{}. {} at {pos_name} with {} open in the class.",
                    coach_name(w, coach),
                    if worst.is_empty() { String::new() } else { format!("; {worst}") },
                    if gem {
                        "Better than his stars say: one of the best on our board"
                    } else {
                        "One of the best on our board"
                    },
                    need
                );
                new_recs.push(Rec {
                    id,
                    coach,
                    date: d,
                    ready: d.add(2),
                    film: coordinator_for(w, j, pos_name),
                    pull: false,
                    read: range((c, sd)),
                    line: line.round() as u8,
                    conviction,
                    why,
                    gem,
                    answer: Answer::Waiting,
                    answered: None,
                });
                *per_coach.entry(coach).or_default() += 1;
                slots -= 1;
            } else if !on && c < line - 2.0 && (stars >= 4 || stars > top) && new_passed.len() < 8 {
                let Some((_, _, f)) = readers::read(w, staff, id) else { continue };
                let (_, worst) = strengths(&f, pos_name);
                let why = format!(
                    "{}: {}{}; his read sits under our {pos_name} line.",
                    coach_name(w, coach),
                    if worst.is_empty() { "not enough there" } else { &worst },
                    if stars > top { ", and a kid this rated won't look at us" } else { "" }
                );
                new_passed.push(Passed {
                    id,
                    coach,
                    date: d,
                    why,
                    stars,
                    read: range((c, sd)),
                    flag: stars >= 4 && rank >= half,
                    back: false,
                });
            } else if on && c < line - 6.0 && w.front_office.owner(Duty::Board) == Owner::Staff {
                // A staff-run board: an offer the staff wants back.
                let committed = p.cal.commit.is_some_and(|x| x.0 == j);
                if !committed && !chain.recs.iter().any(|r| r.id == id && r.pull && r.answer == Answer::Waiting) {
                    pulls.push(Rec {
                        id,
                        coach,
                        date: d,
                        ready: d,
                        film: None,
                        pull: true,
                        read: range((c, sd)),
                        line: line.round() as u8,
                        conviction: (8.0 + 2.5 * (line - c) / sd.max(2.0)).clamp(1.0, 20.0) as u8,
                        why: format!(
                            "{}: his read has fallen well under our {pos_name} line; the offer should come back.",
                            coach_name(w, coach)
                        ),
                        gem: false,
                        answer: Answer::Waiting,
                        answered: None,
                    });
                }
            }
        }
    }
    // The coordinator watches each one's film first.
    for r in &new_recs {
        if let (Some(c), Some(i)) = (r.film, scouting::place(w, r.id)) {
            let kind = if w.people[i].stage == Stage::College { Kind::CollegeFilm } else { Kind::HsFilm };
            dept::add(w, r.id, COACH + c as u32, kind, d);
        }
    }
    let n = new_recs.len() + pulls.len();
    let chain = &mut w.front_office.chain;
    chain.recs.extend(new_recs);
    chain.recs.extend(pulls);
    chain.passed.extend(new_passed);
    n
}

/// The recommendations waiting for you today.
pub fn waiting(w: &World) -> Vec<&Rec> {
    let today = w.today;
    w.front_office.chain.recs.iter().filter(|r| r.answer == Answer::Waiting && r.ready <= today).collect()
}

/// Your film look on a recruit you offer (SK25).
pub(crate) fn your_film(w: &mut World, id: u32) {
    let (Some(you), Some(i)) = (readers::you(w), scouting::place(w, id)) else { return };
    let kind = if w.people[i].stage == Stage::College { Kind::CollegeFilm } else { Kind::HsFilm };
    dept::add(w, id, COACH + you as u32, kind, w.today);
}

/// You offer a recruit yourself: your film look, and the record of any
/// coach you overrule by it (a kid he passed on).
pub(crate) fn on_offer(w: &mut World, id: u32) {
    your_film(w, id);
    let today = w.today;
    let chain = &mut w.front_office.chain;
    if let Some(r) = chain.recs.iter_mut().find(|r| r.id == id && !r.pull && r.answer == Answer::Waiting) {
        r.answer = Answer::Signed;
        r.answered = Some(today);
        let c = Call {
            id,
            coach: r.coach,
            date: today,
            wanted: true,
            conviction: r.conviction,
            overruled: false,
            settled: None,
            turned: None,
        };
        chain.calls.push(c);
        return;
    }
    if let Some(k) = chain.passed.iter().position(|p| p.id == id) {
        let p = chain.passed.remove(k);
        overrule(w, p.coach, id, false, 10);
    }
}

/// You pull an offer a coach of yours recommended (SK28): overruling him.
pub(crate) fn on_pull(w: &mut World, id: u32) {
    let today = w.today;
    let chain = &mut w.front_office.chain;
    if let Some(r) = chain.recs.iter_mut().find(|r| r.id == id && r.pull && r.answer == Answer::Waiting) {
        r.answer = Answer::Signed;
        r.answered = Some(today);
        return;
    }
    let by = chain.calls.iter().rev().find(|c| c.id == id && c.wanted && !c.overruled).map(|c| (c.coach, c.conviction));
    if let Some((c, conv)) = by {
        overrule(w, c, id, true, conv);
    }
}

/// Going against a coach: kept on his record, and his trust in you dips,
/// more the harder he argued.
fn overrule(w: &mut World, coach: usize, id: u32, wanted: bool, conviction: u8) {
    let today = w.today;
    w.front_office.chain.calls.push(Call {
        id,
        coach,
        date: today,
        wanted,
        conviction,
        overruled: true,
        settled: None,
        turned: None,
    });
    w.folk.mine.nudge(coach_key(coach), -(0.01 + 0.03 * conviction as f32 / 20.0));
    let n = overruled_recent(w, coach);
    if n == 3 || n == 5 {
        let name = coach_name(w, coach);
        tell(
            w,
            today,
            format!("{name} has been overruled {n} times in two years. It shows: he works his area with less heart."),
            false,
        );
    }
}

/// Signs a recommendation: the offer (or pull) goes out.
pub fn sign(w: &mut World, id: u32) -> Result<String, String> {
    let j = program(w).ok_or("You have no college job.")?;
    let today = w.today;
    let r = w
        .front_office
        .chain
        .recs
        .iter()
        .find(|r| r.id == id && r.answer == Answer::Waiting && r.ready <= today)
        .cloned()
        .ok_or("Nothing of his waits for your word.")?;
    let text = if r.pull {
        desk::pull_by_chain(w, j, id)?
    } else {
        let why = format!("{} recommended him; you signed it.", coach_name(w, r.coach));
        desk::offer_by_chain(w, j, id, why)?
    };
    let chain = &mut w.front_office.chain;
    if let Some(x) = chain.recs.iter_mut().find(|x| x.id == id && x.answer == Answer::Waiting) {
        x.answer = Answer::Signed;
        x.answered = Some(today);
    }
    if !r.pull {
        chain.calls.push(Call {
            id,
            coach: r.coach,
            date: today,
            wanted: true,
            conviction: r.conviction,
            overruled: false,
            settled: None,
            turned: None,
        });
        your_film(w, id);
    }
    Ok(text)
}

/// Declines a recommendation: an overrule, kept on his record.
pub fn decline(w: &mut World, id: u32) -> Result<String, String> {
    let today = w.today;
    let r = w
        .front_office
        .chain
        .recs
        .iter_mut()
        .find(|r| r.id == id && r.answer == Answer::Waiting)
        .ok_or("Nothing of his waits for your word.")?;
    r.answer = Answer::Declined;
    r.answered = Some(today);
    let (coach, conv, pull) = (r.coach, r.conviction, r.pull);
    let name = coach_name(w, coach);
    let who = scouting::place(w, id).map_or_else(String::new, |i| w.people[i].name.clone());
    if pull {
        // Keeping an offer your staff wanted back.
        overrule(w, coach, id, false, conv);
    } else {
        overrule(w, coach, id, true, conv);
    }
    let how = match conv {
        15.. => "He argued hard for him and takes it badly.",
        9..=14 => "He wanted him and lets you know.",
        _ => "He shrugs it off.",
    };
    let text = format!("You declined {name}'s word on {who}. {how}");
    note(w, text.clone(), Some(id));
    Ok(text)
}

/// Signs every recommendation waiting today.
pub fn sign_all(w: &mut World) -> Result<String, String> {
    let ids: Vec<u32> = waiting(w).iter().map(|r| r.id).collect();
    if ids.is_empty() {
        return Err("Nothing waits for your word.".into());
    }
    let mut done = 0;
    for id in ids {
        if sign(w, id).is_ok() {
            done += 1;
        }
    }
    Ok(format!("Signed {}.", count(done, "recommendation", "recommendations")))
}

/// Puts a recruit your staff passed on back on the board (SK31).
pub fn back_on_board(w: &mut World, id: u32) -> Result<String, String> {
    let k = w.front_office.chain.passed.iter().position(|p| p.id == id).ok_or("He isn't on the passed list.")?;
    let text = desk::add(w, id)?;
    // He stays on the list, marked: offering him later still overrules.
    w.front_office.chain.passed[k].back = true;
    Ok(text)
}

/// Overrules of a coach in the last two years.
pub fn overruled_recent(w: &World, coach: usize) -> usize {
    let since = w.today.add(-730);
    w.front_office.chain.calls.iter().filter(|c| c.coach == coach && c.overruled && c.date >= since).count()
}

/// How much heart a coach puts into his area: 1, down to 0.6 for a coach
/// overruled again and again.
pub fn heart(w: &World, coach: usize) -> f64 {
    (1.0 - 0.1 * overruled_recent(w, coach).saturating_sub(2) as f64).max(0.6)
}

/// A coach's record for his page (SK30): his calls, newest first.
pub fn record(w: &World, coach: usize) -> Vec<&Call> {
    w.front_office.chain.calls.iter().rev().filter(|c| c.coach == coach).collect()
}

/// How a kid turned out three years on: (a hit at your level, the words).
fn turned(w: &World, id: u32, mine: Division) -> (bool, String) {
    let Some(i) = scouting::place(w, id) else { return (false, "out of football".into()) };
    let p = &w.people[i];
    if let Some((year, round, _, _)) = p.pro.as_ref().and_then(|x| x.draft) {
        return (true, format!("drafted in round {round} in {year}"));
    }
    if p.pro.is_some() || p.stage == Stage::Pro {
        return (true, "in the NFL".into());
    }
    let Some(c) = p.college else {
        return (false, "never played college football".into());
    };
    let div = w.programs[c].division;
    let level = crate::world::folk::tips::rank(div) >= crate::world::folk::tips::rank(mine);
    let name = w.program_name(c);
    match p.last_role {
        Some(PlayingTime::Starter) if level => (true, format!("a starter at {name}")),
        Some(PlayingTime::Starter) => (false, format!("a starter at {name}, below your level")),
        Some(PlayingTime::Rotation) => (false, format!("a rotation player at {name}")),
        _ => (false, format!("a backup at {name}")),
    }
}

/// September 1: calls three years old are settled.
fn settle(w: &mut World, d: Date) {
    let Some(j) = program(w) else { return };
    let div = w.programs[j].division;
    let due: Vec<usize> = w
        .front_office
        .chain
        .calls
        .iter()
        .enumerate()
        .filter(|(_, c)| c.settled.is_none() && c.date.add(3 * 365) <= d)
        .map(|(k, _)| k)
        .collect();
    let mut lines: Vec<String> = Vec::new();
    for k in due {
        let c = w.front_office.chain.calls[k].clone();
        let (hit, words) = turned(w, c.id, div);
        let he_right = c.wanted == hit;
        if c.overruled {
            w.folk.mine.nudge(coach_key(c.coach), if he_right { -0.05 } else { 0.03 });
            let who = scouting::place(w, c.id).map_or_else(|| "a recruit".to_string(), |i| w.people[i].name.clone());
            lines.push(format!(
                "{} on {who} ({words}): {}",
                coach_name(w, c.coach),
                if he_right { "he was right and you overruled him" } else { "you were right to overrule him" }
            ));
        }
        let x = &mut w.front_office.chain.calls[k];
        x.settled = Some(he_right);
        x.turned = Some(words);
    }
    if !lines.is_empty() {
        let shown = lines.iter().take(4).cloned().collect::<Vec<_>>().join("; ");
        tell(w, d, format!("Three years on, the calls you overruled: {shown}."), false);
    }
    // The record keeps ten years.
    let cut = d.add(-3650);
    w.front_office.chain.calls.retain(|c| c.date >= cut);
}

/// Whether you hold the board yourself (else your staff's offers wait).
pub fn board_is_yours(w: &World) -> bool {
    w.front_office.owner(Duty::Board) == Owner::You
}
