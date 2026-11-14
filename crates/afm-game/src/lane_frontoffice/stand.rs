//! Where you stand with a transfer, and why he chose another school (Cain
//! 2026-11-14: "agree on all, go ahead with this build"; mock-ups in the
//! project folder, `afm/why-passed/`).
//!
//! The engine splits what each program means to him into seven reasons that
//! add up to his race (`front_office::portal::stand`, `reasons`). Nothing
//! here shows those numbers. A screen gets your staff's read of how much
//! each reason counts with him (a range on 1 to 5 that narrows with every
//! talk and visit, and with what your talks showed of what pulls him), the
//! plain facts at each school (miles, the job he'd have, money in words
//! against yours, the public grades, the calls and visits), who wins each
//! row, and where your staff learned it. Who wins a row is what anyone
//! could see from the facts; his own feel for a place never shows.
//!
//! When he picks another school, your desk kept where you stood the last
//! morning it read his race (`PortalTarget::stand`); the message that tells
//! you carries his own word on why, the rows the winner took, the ones you
//! won and what might have done it. With no talk between you he gives only
//! the short version, and the rest is your staff's guess.

use afm_engine::calendar::Date;
use afm_engine::player::PlayingTime;
use afm_engine::world::folk::claims::Group;
use afm_engine::world::folk::pulls::Ev;
use afm_engine::world::front_office::college::{PortalTarget, Stand, StandRow, VisitState, coordinator};
use afm_engine::world::front_office::portal;
use afm_engine::world::program::GradeCat;
use afm_engine::world::{Stage, World, geo};
use serde_json::{Value, json};

use crate::views::{self, TeamRef};

/// The seven reasons, in the engine's order.
pub(crate) const REASONS: [&str; 7] = [
    "Close to home",
    "The job he'd have",
    "NIL money",
    "Coaching and development",
    "Winning and the name",
    "Academics",
    "Ties and the staffs' work",
];

/// The talk groups that speak to each reason (what his file reads).
const GROUPS: [Group; 7] =
    [Group::Home, Group::PlayingTime, Group::Money, Group::League, Group::Winning, Group::School, Group::People];

fn short(d: Date) -> String {
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (_, m, day) = d.parts();
    format!("{} {day}", M[(m as usize).clamp(1, 12) - 1])
}

fn ord(n: usize) -> String {
    let s = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

/// Your calls, home visit and campus talk with him, and his visit to you:
/// how well your staff knows him.
fn contacts(t: &PortalTarget) -> usize {
    t.talks.len() + t.visits.iter().filter(|v| v.state == VisitState::Done).count()
}

/// Your staff's read of how much each reason counts with him: the race
/// page's read of his file (talk facts lane L7, `pulls::race_weights`), a
/// range on 1 to 5 that narrows with what your talks showed and never
/// reads his own wants.
fn weight_read(w: &World, i: usize) -> [[u8; 2]; 7] {
    let mut out = [[1u8, 5u8]; 7];
    for (k, r) in super::pulls::race_weights(w, i).into_iter().take(7).enumerate() {
        out[k] = r;
    }
    out
}

/// The truth your staff's read is centred on, on the same 1 to 5 scale, and
/// the read's middle: what turns a reason's true points into the size your
/// staff gives it.
fn read_scale(w: &World, i: usize, reads: &[[u8; 2]; 7]) -> [f64; 7] {
    let six = portal::transfer_weights(w, i);
    let mut wts = [0.0; 7];
    wts[..6].copy_from_slice(&six);
    wts[6] = six.iter().sum::<f64>() / 6.0;
    let top = wts.iter().copied().fold(0.01, f64::max);
    let mut out = [1.0; 7];
    for k in 0..7 {
        let truth = 1.0 + 4.0 * (wts[k] / top).clamp(0.0, 1.0);
        let mid = (reads[k][0] as f64 + reads[k][1] as f64) / 2.0;
        out[k] = mid / truth;
    }
    out
}

/// Where your staff learned what each reason counts with him.
fn sources(w: &World, i: usize) -> [String; 7] {
    let p = &w.people[i];
    let id = p.player.id;
    let agent = afm_engine::world::front_office::college::family_read(w, i).decider
        == afm_engine::world::background::Decider::Handler;
    let mut out: [String; 7] = [
        "Public".into(),
        "Their depth charts".into(),
        if agent { "His agent".into() } else { "His people".into() },
        "Public".into(),
        "Public".into(),
        "Public".into(),
        "Your recruiter".into(),
    ];
    for (k, g) in GROUPS.iter().enumerate() {
        if let Some(x) = super::pulls::race_source(w, i, k) {
            out[k] = x;
        } else if w.folk.pulls.of(id).any(|r| r.group() == *g && r.ev == Ev::Guess) {
            out[k] = "Your recruiter's guess".into();
        }
    }
    out
}

/// A reason's name inside a sentence ("NIL money" keeps its capitals).
fn lower(k: usize) -> String {
    if k == 2 { REASONS[k].to_string() } else { REASONS[k].to_lowercase() }
}

fn role_words(r: PlayingTime) -> &'static str {
    match r {
        PlayingTime::Starter => "Starts",
        PlayingTime::Rotation => "Rotation",
        _ => "Backup",
    }
}

/// Players at his position on a program now with a season left after this
/// one (anyone can count them on a roster).
fn ahead_at(w: &World, j: usize, i: usize) -> usize {
    let pos = w.people[i].player.position;
    w.programs[j]
        .roster
        .iter()
        .filter(|&&k| {
            k != i
                && w.people[k].stage == Stage::College
                && w.people[k].player.position == pos
                && w.people[k].college_seasons < 4
        })
        .count()
}

fn miles(w: &World, i: usize, j: usize) -> f64 {
    let p = &w.people[i];
    let pr = &w.programs[j];
    let home = match w.high_schools.get(p.hs) {
        Some(h) => geo::place(h.state, &h.town),
        None => geo::place(p.home_state, ""),
    };
    geo::miles_between(home, geo::campus(pr.state, &pr.city)).round()
}

/// Their money against yours, in words (no dollar figures).
fn money_words(theirs: f64, yours: f64) -> String {
    if theirs <= 0.0 {
        return "No money".into();
    }
    if yours <= 0.0 {
        return "Pays him; you don't".into();
    }
    let r = theirs / yours;
    match r {
        r if r >= 2.5 => format!("About {:.0} times yours", r),
        r if r >= 1.7 => "About twice yours".into(),
        r if r >= 1.15 => "More than yours".into(),
        r if r >= 0.85 => "About the same as yours".into(),
        r if r >= 0.6 => "A bit less than yours".into(),
        r if r >= 0.4 => "About half of yours".into(),
        r if r >= 0.28 => "About a third of yours".into(),
        r if r >= 0.18 => "About a quarter of yours".into(),
        _ => "Far less than yours".into(),
    }
}

/// What you promised him, as he weighs it, and whether NIL Go held back
/// part of your collective deal.
fn your_money(t: &PortalTarget, weighed: f64) -> String {
    let promised = t.nil.unwrap_or(0.0) + t.nil_coll.unwrap_or(0.0);
    if promised <= 0.0 {
        "No money: the place alone".into()
    } else if weighed + 1.0 < promised * 0.95 {
        "Yours; NIL Go cleared only part of the collective deal".into()
    } else {
        "Yours".into()
    }
}

/// The calls and visits at a program, in words, and a public score for who
/// has done the most with him (calls, his visit, a promise).
fn work(t: &PortalTarget, row: &StandRow, yours: bool) -> (String, f64) {
    if yours {
        let calls = t.talks.iter().filter(|x| x.at == 0).count();
        let mut v: Vec<String> = Vec::new();
        v.push(match calls {
            0 => "No call yet".into(),
            1 => "1 call".into(),
            n => format!("{n} calls"),
        });
        if t.talks.iter().any(|x| x.at == 1) {
            v.push("a home visit".into());
        }
        let visit = t.visits.iter().find(|x| x.state == VisitState::Done && x.kind.key() == "official").map(|x| x.date);
        if let Some(d) = visit.or(row.hosted) {
            v.push(format!("his visit {}", short(d)));
        }
        if t.promise.is_some() {
            v.push("a promise".into());
        }
        let score = calls.min(3) as f64
            + if t.talks.iter().any(|x| x.at == 1) { 1.0 } else { 0.0 }
            + if visit.or(row.hosted).is_some() { 2.0 } else { 0.0 }
            + if t.promise.is_some() { 1.0 } else { 0.0 };
        (v.join(", "), score)
    } else {
        match row.hosted {
            Some(d) => (format!("A call, his visit {}", short(d)), 3.0),
            None => ("A call".into(), 1.0),
        }
    }
}

/// One row's fact at one program.
fn fact(w: &World, i: usize, k: usize, row: &StandRow, yours: Option<f64>, you: bool, t: &PortalTarget) -> String {
    let j = row.program;
    let g = |c: GradeCat| w.programs[j].grades[c as usize];
    let pos = w.people[i].player.position.name();
    match k {
        0 => format!("{:.0} miles", miles(w, i, j)),
        1 => {
            let n = ahead_at(w, j, i);
            format!("{}: {n} {pos}{} with a year left", role_words(row.role), if n == 1 { "" } else { "s" })
        }
        2 if you => your_money(t, row.nil as f64),
        2 => money_words(row.nil as f64, yours.unwrap_or(0.0)),
        3 => format!("Pro pipeline {:.0}, coaches {:.0}", g(GradeCat::ProPipeline), g(GradeCat::CoachPrestige)),
        4 => format!("{} · title chances {:.0}", w.programs[j].division.name(), g(GradeCat::TitleChances)),
        5 => format!("Academics {:.0}", g(GradeCat::Academics)),
        _ => work(t, row, you).0,
    }
}

/// Who wins each row among `rows`: the most points for the six reasons
/// anyone can see the facts of, the most work done with him for the last.
fn winners(t: &PortalTarget, rows: &[StandRow], me: usize) -> [usize; 7] {
    let mut out = [0usize; 7];
    for (k, o) in out.iter_mut().enumerate() {
        let score = |r: &StandRow| if k == 6 { work(t, r, r.program == me).1 } else { r.parts[k] as f64 };
        *o = (0..rows.len()).max_by(|&a, &b| score(&rows[a]).total_cmp(&score(&rows[b])).then(b.cmp(&a))).unwrap_or(0);
    }
    out
}

/// Your staff's line: what is costing you against `other` (or what puts
/// you ahead), sized by your staff's read of what counts with him, and
/// what could move it.
fn staff_line(
    w: &World,
    i: usize,
    t: &PortalTarget,
    you: &StandRow,
    other: &StandRow,
    scale: &[f64; 7],
    reads: &[[u8; 2]; 7],
) -> (String, String) {
    let j = other.program;
    let them = w.program_name(j).to_string();
    let gap = |k: usize| (other.parts[k] - you.parts[k]) as f64 * scale[k];
    let lead = you.place < other.place;
    let k = if lead {
        (0..7).min_by(|&a, &b| gap(a).total_cmp(&gap(b))).unwrap_or(1)
    } else {
        (0..7).max_by(|&a, &b| gap(a).total_cmp(&gap(b))).unwrap_or(1)
    };
    let first = w.people[i].name.split_whitespace().next().unwrap_or("He").to_string();
    let says = match (lead, k) {
        (true, _) => format!("We lead {them}. {} is our edge with {first}.", REASONS[k]),
        (false, 0) => format!(
            "It's home. {them} is {:.0} miles from his family, we're {:.0}.",
            miles(w, i, j),
            miles(w, i, you.program)
        ),
        (false, 1) => format!(
            "It's the job. He'd {} at {them}; here he'd {}.",
            match other.role {
                PlayingTime::Starter => "start",
                PlayingTime::Rotation => "rotate",
                _ => "back up",
            },
            match you.role {
                PlayingTime::Starter => "start too, but the rest of their pitch is better",
                PlayingTime::Rotation => "rotate",
                _ => "back up",
            }
        ),
        (false, 2) => format!("It's the money. {}'s offer is {}.", them, money_rel(other.nil, you.nil)),
        (false, 3) => format!("It's development: {them}'s record with pros and its coaches."),
        (false, 4) => format!("It's the name: {them} wins more and gets seen more."),
        (false, 5) => format!("It's the school: {them}'s academics."),
        _ => format!("It's the work: {them} has done more with him than we have."),
    };
    // Money you lead on that barely counts with him.
    let money_mid = (reads[2][0] + reads[2][1]) as f64 / 2.0;
    let money_note = if !lead && k != 2 && you.parts[2] > other.parts[2] && money_mid <= 2.5 {
        " Our money beats theirs, but it barely counts with him."
    } else {
        ""
    };
    let move_it = if lead {
        "Hold the lead: he says yes once it is as big as he needs.".to_string()
    } else {
        match k {
            0 => "Nothing changes the miles. Win him on the rest.".into(),
            1 if t.promise.is_none() => "Promise him the starting job, if he'd believe it.".into(),
            1 => "Your promise is out. Only his trust in it can grow.".into(),
            2 => "More money: the gap is real to him.".into(),
            6 => "Call him, and get him on campus.".into(),
            _ => "Make the case on a call or his visit: your talks can still move it a little.".into(),
        }
    };
    let guess = if contacts(t) == 0 { "A first read, before you've talked to him: " } else { "" };
    (format!("{guess}{says}{money_note}"), move_it)
}

/// Their money against yours, lower case, for "their offer is ...".
fn money_rel(theirs: f32, yours: f32) -> String {
    match (theirs > 0.0, yours > 0.0) {
        (false, _) => "nothing, against ours".into(),
        (true, false) => "real money, and we offered none".into(),
        _ => money_words(theirs as f64, yours as f64).to_lowercase().replace("yours", "ours"),
    }
}

/// The "Where you stand" panel on a transfer's file: None until you're in
/// his race (an offer out) and he is shopping.
pub(crate) fn panel(w: &World, j: usize, i: usize, t: &PortalTarget) -> Option<Value> {
    let s = portal::stand(w, i)?;
    Some(read(w, j, i, t, &s))
}

fn read(w: &World, j: usize, i: usize, t: &PortalTarget, s: &Stand) -> Value {
    let you = s.rows.iter().find(|r| r.program == j).cloned();
    let mut cols: Vec<StandRow> = Vec::new();
    if let Some(y) = &you {
        cols.push(y.clone());
    }
    cols.extend(s.rows.iter().filter(|r| r.program != j).cloned());
    let reads = weight_read(w, i);
    let scale = read_scale(w, i, &reads);
    let src = sources(w, i);
    let wins = winners(t, &cols, j);
    let yours_money = you.as_ref().map(|y| y.nil as f64);
    // Rows in the order your staff thinks they count.
    let mut order: Vec<usize> = (0..7).collect();
    order.sort_by(|&a, &b| {
        let m = |k: usize| reads[k][0] as u16 + reads[k][1] as u16;
        m(b).cmp(&m(a)).then(a.cmp(&b))
    });
    let rows: Vec<Value> = order
        .iter()
        .map(|&k| {
            let cells: Vec<Value> = cols
                .iter()
                .enumerate()
                .map(|(c, r)| json!({ "fact": fact(w, i, k, r, yours_money, r.program == j, t), "win": wins[k] == c }))
                .collect();
            json!({ "k": k, "name": REASONS[k], "weight": reads[k], "cells": cells, "source": src[k] })
        })
        .collect();
    let columns: Vec<Value> = cols
        .iter()
        .map(|r| {
            json!({ "team": views::team_brief(w, TeamRef::College(r.program)), "name": w.program_name(r.program), "place": r.place, "yours": r.program == j })
        })
        .collect();
    // Against the best other school: the rows you take and the ones you lose.
    let other = cols.iter().filter(|r| r.program != j).min_by_key(|r| r.place).cloned();
    let (mut ahead, mut behind) = (Vec::new(), Vec::new());
    let (mut says, mut move_it) = (String::new(), String::new());
    if let (Some(y), Some(o)) = (&you, &other) {
        for &k in &order {
            let (a, b) =
                if k == 6 { (work(t, y, true).1, work(t, o, false).1) } else { (y.parts[k] as f64, o.parts[k] as f64) };
            if a > b {
                ahead.push(lower(k));
            } else if b > a {
                behind.push(lower(k));
            }
        }
        (says, move_it) = staff_line(w, i, t, y, o, &scale, &reads);
    }
    let who = coordinator(w, j).map(|c| w.coaches[c].name.clone());
    json!({
        "place": you.as_ref().map(|y| y.place), "place_text": you.as_ref().map(|y| ord(y.place as usize)),
        "schools": s.schools, "versus": other.map(|o| w.program_name(o.program).to_string()),
        "ahead": ahead.into_iter().take(3).collect::<Vec<_>>(), "behind": behind.into_iter().take(3).collect::<Vec<_>>(),
        "columns": columns, "rows": rows, "contacts": contacts(t),
        "who": who, "says": says, "move": move_it,
    })
}

/// His own word on why he chose `to`, by what weighed most for him there
/// over you.
fn his_words(k: usize, role: PlayingTime) -> &'static str {
    match k {
        0 => "It's close to home. My family can get to every game.",
        1 => match role {
            PlayingTime::Starter => "They gave me the job. I need to be on the field.",
            _ => "I'll play right away there. I need snaps.",
        },
        2 => "Honestly, the money. It means a lot for my family.",
        3 => "Their coaches develop guys for the league. That's my shot.",
        4 => "They win, and everybody watches them.",
        5 => "The degree matters to me.",
        _ => "I trust those coaches. They were on me every day.",
    }
}

/// The message when a transfer you offered chooses `to`: the body's
/// lines (the first goes on from his name: "chose ... today") and the why,
/// as your staff has it. None without a kept read of his race.
pub(crate) fn lost(w: &World, j: usize, i: usize, t: &PortalTarget, to: usize) -> Option<(Vec<String>, Vec<String>)> {
    let s = t.stand.as_ref()?;
    let you = s.rows.iter().find(|r| r.program == j)?;
    let them = w.program_name(to).to_string();
    let mut body = vec![format!(
        "chose {them} today. We had offered him, and we were {} of his {} when we last read his race.",
        ord(you.place as usize),
        s.schools
    )];
    let Some(win) = s.rows.iter().find(|r| r.program == to) else {
        body.push(format!(
            "{them} wasn't among his top schools when we last read him: something moved late. We never heard what."
        ));
        return Some((body, Vec::new()));
    };
    // What weighed most for him there over you (he knows his own mind).
    let gap = |k: usize| (win.parts[k] - you.parts[k]) as f64;
    let mut ks: Vec<usize> = (0..7).collect();
    ks.sort_by(|&a, &b| gap(b).total_cmp(&gap(a)));
    let top = ks[0];
    let talked = contacts(t) > 0;
    if talked {
        body.push(format!("I called him after he told us. In his words: \"{}\"", his_words(top, win.role)));
    } else {
        body.push(
            "I called him after he told us. He kept it short: \"It was just the best fit for me.\" We had barely talked to him, so the rest is our read."
                .into(),
        );
    }
    let fact_line = |k: usize| -> String {
        match k {
            0 => format!("Home: {them} is {:.0} miles from his family, we're {:.0}.", miles(w, i, to), miles(w, i, j)),
            1 => format!(
                "The job: he {} at {them}; here he'd have {}.",
                match win.role {
                    PlayingTime::Starter => "starts",
                    PlayingTime::Rotation => "rotates",
                    _ => "backs up",
                },
                match you.role {
                    PlayingTime::Starter => "started too",
                    PlayingTime::Rotation => "rotated",
                    _ => "backed up",
                }
            ),
            2 => format!("The money: {them}'s offer was {}.", money_rel(win.nil, you.nil)),
            3 => format!("Development: {them}'s record with pros and its coaches."),
            4 => format!("The name: {them} wins more and gets seen more."),
            5 => format!("The school: {them}'s academics."),
            _ => format!("The work: {them} did more with him than we did."),
        }
    };
    let mut why = Vec::new();
    let lead = if talked { "" } else { "Our guess: " };
    why.push(format!("{lead}{}", fact_line(top)));
    if gap(ks[1]) > 0.25 * gap(top) && gap(ks[1]) > 0.0 {
        why.push(fact_line(ks[1]));
    }
    let won: Vec<usize> = (0..6).filter(|&k| you.parts[k] > win.parts[k]).collect();
    if !won.is_empty() {
        let names: Vec<String> = won.iter().map(|&k| lower(k)).collect();
        let worth: f64 = won.iter().map(|&k| (you.parts[k] - win.parts[k]) as f64).sum();
        let lost: f64 = (0..7).map(gap).filter(|&g| g > 0.0).sum();
        let little = if worth < 0.5 * lost { ". They counted little with him" } else { "" };
        why.push(format!("Where we beat them: {}{little}.", names.join(", ")));
    }
    let move_it = match top {
        0 => "Nothing we could change: the miles are the miles.",
        1 => "What might have done it: a starting job he believed.",
        2 => "What might have done it: more money.",
        6 => "What might have done it: more calls and his visit.",
        _ => "What might have done it: little. It was what they are.",
    };
    why.push(move_it.into());
    Some((body, why))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_reads_in_words() {
        assert_eq!(money_words(0.0, 5.0), "No money");
        assert_eq!(money_words(5.0, 0.0), "Pays him; you don't");
        assert_eq!(money_words(1.0, 1.0), "About the same as yours");
        assert_eq!(money_words(1.0, 3.0), "About a third of yours");
        assert_eq!(money_rel(1.0, 3.0), "about a third of ours");
        assert_eq!(money_rel(1.0, 1.0), "about the same as ours");
        assert_eq!(money_rel(2.0, 1.0), "about twice ours");
        assert_eq!(money_rel(2.0, 0.0), "real money, and we offered none");
    }

    #[test]
    fn places_read_as_words() {
        assert_eq!(ord(1), "1st");
        assert_eq!(ord(4), "4th");
        assert_eq!(ord(12), "12th");
        assert_eq!(ord(22), "22nd");
    }
}
