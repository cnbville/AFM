//! Finds, blooms, setbacks and misses (GR66, GR67, part G; lane G4).
//!
//! - A kid becomes your find when you act first: your offer while the
//!   world still rates him low (his stars and his other offers), not when
//!   you hear first. A find you lose to a rival stays your find. It is
//!   credited to the man who heard first and to you as the one who decided.
//! - A find blooms in steps when the world catches up: a small bloom (a
//!   fourth star, or an offer from a program a level up), a bloom (a
//!   starting job by his second college season, or all-conference), a big
//!   bloom (drafted, or All-American). The lower the world rated him when
//!   you acted, the more it pays: the person who told you (points inside
//!   the gates, the moment in his words, his kid held up), your man (his
//!   record), your name as a finder (noteworthy, never driving), and the
//!   news when it's big.
//! - Setbacks are felt: the flip (he signs elsewhere: still your find, and
//!   your class misses him), the grades (a prep school or a junior college),
//!   the bust (four years on, no bloom: the teller's word didn't hold).
//! - A miss: each September, a kid from your own ground nobody of yours
//!   ever looked at who bloomed somewhere else, with who knew and why you
//!   didn't hear.
//!
//! Everything is kept for the ground report (lanes G6 and S8). The sizes
//! are the spec's starting sizes or the build's picks (proposed, not
//! confirmed, tuned in GR61's trial worlds).

use super::people::{self, Who};
use super::standing::{self, Level};
use crate::calendar::{Date, Event, Kind, Level as CalLevel, Loudness};
use crate::positions::Position;
use crate::world::{Stage, World};

/// One find.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Find {
    pub kid: u32,
    pub name: String,
    pub pos: Position,
    pub school: u32,
    pub class_of: i32,
    /// Your program when you offered.
    pub prog: u32,
    /// The day you offered him.
    pub date: Date,
    /// What the world thought of him then: his stars and his other offers.
    pub stars: u8,
    pub offers: u8,
    /// The highest level among his other offers then (a small bloom needs
    /// an offer from above it and above yours).
    #[serde(default)]
    pub top: u8,
    /// How much a bloom pays (1.0 a two-star with no offers down to 0.4 a
    /// three-star with five).
    pub pay: f32,
    /// Who told you (a ground person), or the tipper's name, or how.
    pub teller: Option<Who>,
    pub tipper: Option<String>,
    /// The man who heard first, and you, who decided.
    pub man: Option<u32>,
    pub you: u32,
    /// Bloom steps reached: 0 none, 1 small, 2 bloom, 3 big.
    pub steps: u8,
    pub blooms: Vec<Bloom>,
    /// He signed with another program (the flip): when and where.
    pub lost: Option<(Date, u32)>,
    /// A setback, if one came.
    pub setback: Option<(Date, Setback, String)>,
}

/// One bloom step.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Bloom {
    pub date: Date,
    pub step: u8,
    pub words: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Setback {
    /// He signed with another program.
    Flip,
    /// He didn't qualify: a prep school or a junior college.
    Grades,
    /// Four years on, no bloom.
    Bust,
}

impl Setback {
    pub fn key(self) -> &'static str {
        match self {
            Setback::Flip => "flip",
            Setback::Grades => "grades",
            Setback::Bust => "bust",
        }
    }
}

/// A kid from your own ground nobody of yours saw, who bloomed elsewhere.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Miss {
    pub kid: u32,
    pub name: String,
    pub pos: Position,
    pub school: u32,
    pub class_of: i32,
    /// The September it was counted.
    pub date: Date,
    /// Where he went.
    pub went: Option<u32>,
    /// What he did there.
    pub what: String,
    /// Who knew: his coach, and your level with him.
    pub coach: Option<Who>,
    pub level: u8,
    /// Why you didn't hear, in words.
    pub why: String,
}

/// A man's record as a finder.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Man {
    pub coach: u32,
    pub finds: u16,
    pub blooms: u16,
    pub big: u16,
}

/// What a bloom step pays the teller (standing points before the find's
/// pay) and your name as a finder (build's picks).
pub const TELLER: [f32; 3] = [2.0, 4.0, 8.0];
pub const FINDER: [f32; 3] = [0.5, 1.0, 2.0];
/// Misses counted a September, at most.
pub const MISSES: usize = 12;
/// Misses kept in all.
pub const MISSES_KEPT: usize = 60;

/// The highest stars and other offers that still count as the world
/// rating him low at your level: three stars at FBS, two below it (build's
/// pick); five other offers.
fn low(w: &World, prog: usize) -> u8 {
    if w.programs[prog].division == crate::world::data::Division::FBS { 3 } else { 2 }
}
const LOW_OFFERS: usize = 5;

/// How much a find pays: a kid a star under the line with no offers 1.0,
/// a kid at the line 0.7, less 0.06 for each other offer, never under 0.4.
pub fn pay(stars: u8, line: u8, offers: usize) -> f32 {
    let base = if stars < line { 1.0 } else { 0.7 };
    (base - 0.06 * offers as f32).clamp(0.4, 1.0)
}

/// A program's level for "an offer from a level up": its division, and a
/// power conference above the rest of FBS.
fn tier(w: &World, j: usize) -> u8 {
    let p = &w.programs[j];
    2 * crate::world::folk::tips::rank(p.division) + p.power_conference(w.year) as u8
}

/// Other programs that offered him.
fn others(w: &World, i: usize, prog: usize) -> Vec<usize> {
    let p = &w.people[i];
    let mut v: Vec<usize> = p.cal.offers.iter().chain(p.cal.early_offers.iter().map(|x| &x.0)).copied().collect();
    v.sort_unstable();
    v.dedup();
    v.retain(|&j| j != prog);
    v
}

/// Your offer went out (`front_office::college::offer_at`): he's your find
/// if the world still rates him low.
pub(crate) fn on_offer(w: &mut World, prog: usize, id: u32, d: Date) {
    if super::leads::seat(w) != Some(prog) {
        return;
    }
    if w.ground.finds.iter().any(|f| f.kid == id) {
        return;
    }
    let Some(i) = crate::world::scouting::place(w, id) else { return };
    let p = &w.people[i];
    if !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) {
        return;
    }
    let line = low(w, prog);
    let n = others(w, i, prog).len();
    if p.stars > line || n > LOW_OFFERS {
        return;
    }
    let lead = super::leads::lead(w, id);
    let man = lead.and_then(|l| l.man).or_else(|| {
        crate::world::scouting::dept::recruiter_for(w, prog, i)
            .or_else(|| crate::world::scouting::dept::evaluator_for(w, prog, i))
            .map(|c| c as u32)
    });
    let you = w.career.as_ref().map_or(0, |c| c.coach as u32);
    let f = Find {
        kid: id,
        name: p.name.clone(),
        pos: p.player.position,
        school: p.hs as u32,
        class_of: p.class_of,
        prog: prog as u32,
        date: d,
        stars: p.stars,
        offers: n as u8,
        top: others(w, i, prog).into_iter().map(|j| tier(w, j)).max().unwrap_or(0),
        pay: pay(p.stars, line, n),
        teller: lead.and_then(|l| l.from),
        tipper: lead.and_then(|l| l.tipper.clone()),
        man,
        you,
        steps: 0,
        blooms: Vec::new(),
        lost: None,
        setback: None,
    };
    w.ground.finds.push(f);
    if let Some(c) = man {
        man_mut(w, c).finds += 1;
    }
}

fn man_mut(w: &mut World, c: u32) -> &mut Man {
    let g = &mut w.ground;
    let k = match g.men.binary_search_by_key(&c, |m| m.coach) {
        Ok(k) => k,
        Err(k) => {
            g.men.insert(k, Man { coach: c, ..Man::default() });
            k
        }
    };
    &mut g.men[k]
}

/// A man's record as a finder.
pub fn man(w: &World, c: u32) -> Man {
    w.ground
        .men
        .binary_search_by_key(&c, |m| m.coach)
        .ok()
        .map_or(Man { coach: c, ..Man::default() }, |k| w.ground.men[k])
}

/// The bloom step a kid has reached, and its words.
fn reached(w: &World, i: usize, f: &Find, prog: usize) -> (u8, String) {
    let p = &w.people[i];
    let honors = w.stats.honors.get(&f.kid);
    if let Some((year, round, _, _)) = p.pro.as_ref().and_then(|x| x.draft) {
        let _ = year;
        return (3, format!("drafted in round {round}"));
    }
    if honors.is_some_and(|h| !h.all_america.is_empty()) {
        return (3, "an All-American".into());
    }
    if honors.is_some_and(|h| h.all_conference > 0) {
        return (2, "all-conference".into());
    }
    let college: Vec<&crate::player::SeasonRecord> = p
        .player
        .history
        .iter()
        .filter(|r| !matches!(r.level, crate::player::Level::HS | crate::player::Level::NFL))
        .take(2)
        .collect();
    if college.iter().any(|r| r.playing_time == crate::player::PlayingTime::Starter) {
        return (2, "a starter by his second season".into());
    }
    if p.stars >= 4 && p.stars > f.stars {
        return (1, format!("a {}-star now", p.stars));
    }
    let mine = tier(w, prog).max(f.top);
    if let Some(j) = others(w, i, prog).into_iter().find(|&j| tier(w, j) > mine) {
        return (1, format!("{} offered him", w.program_name(j)));
    }
    (0, String::new())
}

/// Each day for your seat: blooms, flips and grades; on September 1 the
/// busts and the misses from your ground.
pub(crate) fn day(w: &mut World, prog: usize, d: Date) {
    let season = w.dates.season;
    let sept = d.parts().1 == 9 && d.parts().2 == 1;
    for k in 0..w.ground.finds.len() {
        let f = w.ground.finds[k].clone();
        if f.prog as usize != prog {
            continue;
        }
        let Some(i) = crate::world::scouting::place(w, f.kid) else { continue };
        let p = &w.people[i];
        // The flip: he signed with someone else. Still your find.
        if f.lost.is_none()
            && let Some(s) = p.cal.signed
            && s.program != prog
        {
            let words =
                format!("He signed with {}. He's still your find; your class misses him.", w.program_name(s.program));
            let x = &mut w.ground.finds[k];
            x.lost = Some((d, s.program as u32));
            x.setback.get_or_insert((d, Setback::Flip, words));
        }
        // The grades: a prep school or a junior college.
        let p = &w.people[i];
        let juco = p.stage == Stage::College
            && p.college.is_some_and(|c| w.programs[c].division == crate::world::data::Division::JUCO);
        if f.setback.is_none() && f.class_of <= season && (p.stage == Stage::PrepSchool || juco) {
            let words = "He didn't qualify and went the long way; you keep first call when he's ready.".to_string();
            w.ground.finds[k].setback = Some((d, Setback::Grades, words));
        }
        let (step, words) = reached(w, i, &f, prog);
        if step > f.steps {
            for s in f.steps + 1..=step {
                let w_s = if s == step { words.clone() } else { step_words(s).into() };
                bloom(w, prog, k, s, w_s, d);
            }
        }
        // The bust: four years past his class with no bloom.
        if sept && step == 0 && f.class_of + 4 <= season && w.ground.finds[k].setback.is_none() {
            let words = "Four years on, he never bloomed.".to_string();
            w.ground.finds[k].setback = Some((d, Setback::Bust, words));
            super::leads::settle(w, f.kid, false);
        }
    }
    if sept {
        misses(w, prog, d);
    }
}

fn step_words(s: u8) -> &'static str {
    match s {
        1 => "a small bloom",
        2 => "a bloom",
        _ => "a big bloom",
    }
}

/// One bloom step pays out.
fn bloom(w: &mut World, prog: usize, k: usize, step: u8, words: String, d: Date) {
    let f = w.ground.finds[k].clone();
    w.ground.finds[k].steps = step;
    w.ground.finds[k].blooms.push(Bloom { date: d, step, words: words.clone() });
    let s = (step as usize - 1).min(2);
    // Your name as a finder (noteworthy, never driving).
    w.ground.finder += FINDER[s] * f.pay;
    // Your man's record.
    if let Some(c) = f.man {
        let m = man_mut(w, c);
        if step == 2 {
            m.blooms += 1;
        }
        if step == 3 {
            m.big += 1;
        }
    }
    // The person who told you: his kid held up, and he keeps the moment.
    if step == 1 || f.steps == 0 {
        super::leads::settle(w, f.kid, true);
    }
    let first = f.name.split_whitespace().last().unwrap_or(&f.name).to_string();
    if let Some(who) = f.teller {
        let said = match step {
            1 => format!("Everybody's calling about {first} now. You were first."),
            2 => format!("You were the first to believe in my {first}."),
            _ => format!("You were the first to believe in my kid. Look at {first} now."),
        };
        standing::bloom(w, prog, who, TELLER[s] * f.pay, said.clone(), step == 3, d);
        let subject = format!("{}: {words}", f.name);
        standing::post_call(w, who, d, subject, vec![format!("\"{said}\"")], 1);
    }
    // The news names the program that believed first when it's big.
    if step == 3 {
        let school = w.program_name(f.prog as usize).to_string();
        let then = if f.offers == 0 {
            format!("the first school to offer him, as a {}-star with no other offer", f.stars.max(1))
        } else {
            format!("an early believer, offering him as a {}-star", f.stars.max(1))
        };
        let text = format!("{} is {words}. {school} was {then}.", f.name);
        w.calendar.post(
            Event::new(d, CalLevel::Recruiting, Kind::News, Loudness::Small, text)
                .person(f.kid, &f.name)
                .program(f.prog as usize),
        );
    }
}

/// September 1: kids from your own ground (the areas where your staff has
/// people) in their first two college seasons who bloomed somewhere else
/// and whom nobody of yours ever looked at, with who knew and why you
/// didn't hear.
fn misses(w: &mut World, prog: usize, d: Date) {
    let season = w.dates.season;
    let holders = standing::holders(w, prog);
    let mut areas: Vec<u16> = w
        .ground
        .memories
        .iter()
        .filter(|m| holders.contains(&m.holder))
        .filter_map(|m| people::school(w, m.who))
        .filter_map(|h| w.high_schools.get(h).map(|x| x.ground_area))
        .collect();
    areas.sort_unstable();
    areas.dedup();
    if areas.is_empty() {
        return;
    }
    let mine = crate::world::folk::tips::rank(w.programs[prog].division);
    let mut found: Vec<(u32, usize, String)> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        if p.stage != Stage::College || p.class_of < season - 2 || p.class_of > season - 1 {
            continue;
        }
        let Some(c) = p.college else { continue };
        if c == prog || crate::world::folk::tips::rank(w.programs[c].division) < mine {
            continue;
        }
        let Some(h) = w.high_schools.get(p.hs) else { continue };
        if areas.binary_search(&h.ground_area).is_err() {
            continue;
        }
        let id = p.player.id;
        if w.scouting.dept.looks.contains_key(&id) || super::leads::lead(w, id).is_some() {
            continue;
        }
        if w.ground.misses.iter().any(|m| m.kid == id) {
            continue;
        }
        let honors = w.stats.honors.get(&id);
        let started = p
            .player
            .history
            .iter()
            .filter(|r| !matches!(r.level, crate::player::Level::HS | crate::player::Level::NFL))
            .take(2)
            .any(|r| r.playing_time == crate::player::PlayingTime::Starter);
        let (score, what) = if honors.is_some_and(|h| h.all_conference > 0) {
            (20, format!("made all-conference at {}", w.program_name(c)))
        } else if started {
            (10, format!("started at {}", w.program_name(c)))
        } else {
            continue;
        };
        found.push((score + (5 - p.stars.min(5)) as u32, i, what));
    }
    found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    found.truncate(MISSES);
    for (_, i, what) in found {
        let p = &w.people[i];
        let coach = people::head_coach_of(w, p.hs);
        let level = coach.map_or(Level::Stranger, |who| standing::level(w, prog, who));
        let school = w.school_name(p.hs).to_string();
        let last = coach.and_then(|who| {
            holders.iter().filter_map(|&h| w.ground.get(who, h)).filter(|m| m.contacts > 0).map(|m| m.last).max()
        });
        let why = match (level, last) {
            (Level::Stranger, None) => {
                format!("His coach at {school} is a Stranger to you: nobody on your staff has ever been to his school.")
            }
            (Level::Stranger, Some(l)) => format!(
                "His coach at {school} is a Stranger to you: nobody on your staff has been to his school since {}.",
                l.year()
            ),
            (lv, _) => {
                format!("You're {} with his coach at {school}, and nobody of yours ever looked at him.", lv.name())
            }
        };
        let m = Miss {
            kid: p.player.id,
            name: p.name.clone(),
            pos: p.player.position,
            school: p.hs as u32,
            class_of: p.class_of,
            date: d,
            went: p.college.map(|c| c as u32),
            what,
            coach,
            level: level as u8,
            why,
        };
        w.ground.misses.push(m);
    }
    let n = w.ground.misses.len();
    if n > MISSES_KEPT {
        w.ground.misses.drain(0..n - MISSES_KEPT);
    }
}

/// Your name as a finder in words (noteworthy, never driving).
pub fn finder_words(w: &World) -> &'static str {
    match w.ground.finder {
        x if x >= 12.0 => "Coaches send you their late bloomers: you're known for finding kids.",
        x if x >= 5.0 => "Word is getting around that you find kids others miss.",
        x if x > 0.0 => "A few coaches have noticed you believed early.",
        _ => "No name as a finder yet.",
    }
}
