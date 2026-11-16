//! The ground report's data (GR77; the screen is lane S8, board 8b): your
//! offseason by name ("Your first spring"), the goals as verdicts with why,
//! your finds with their bloom steps, your ground February to August by
//! area (the headline area called out, lost ground marked), who outworked
//! whom in each area in signs (each program's dated contacts with the
//! area's recruits, February to the verdict), your men's records, the miss
//! and a setback, your AD's word and the paper. Built at the verdict and
//! kept; nothing here reads a hidden number.

use super::goals::{self, Verdict};
use super::{Slot, standing};
use crate::calendar::Date;
use crate::world::World;
use crate::world::folk::Team;
use crate::world::ground::areas;

/// One goal on the report.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GoalLine {
    pub slot: Slot,
    pub words: String,
    pub verdict: Verdict,
    /// Why, from the world ("Auburn flipped him on the Fourth").
    pub why: String,
    pub bold: bool,
    pub public: bool,
    pub dropped: bool,
}

/// One area, February to August.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AreaLine {
    pub area: u16,
    pub name: String,
    /// Your standing in words then and now, and the points.
    pub feb: String,
    pub aug: String,
    pub moved: f32,
    /// Lost a level.
    pub lost: bool,
    /// Who leads it now.
    pub leader: Option<String>,
}

/// Who outworked whom in an area: your contacts with its recruits against
/// the busiest other staff's, February to the verdict (signs, not secrets).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Outworked {
    pub area: u16,
    pub name: String,
    pub yours: u32,
    pub rival: Option<(String, u32)>,
}

/// One find on the report.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FindLine {
    pub kid: u32,
    pub name: String,
    pub pos: String,
    pub stars_then: u8,
    pub stars_now: u8,
    /// Committed to you, signed, lost, or still open.
    pub state: String,
    pub steps: u8,
    pub blooms: Vec<String>,
}

/// The report.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub year: i32,
    pub date: Date,
    /// "Your first spring", "Your third spring at Georgia State".
    pub title: String,
    pub goals: Vec<GoalLine>,
    pub finds: Vec<FindLine>,
    pub areas: Vec<AreaLine>,
    /// The area that moved most (its index in `areas`).
    pub headline: Option<usize>,
    pub outworked: Vec<Outworked>,
    /// Your men's records: (coach, name, finds, blooms, big blooms).
    pub men: Vec<(u32, String, u16, u16, u16)>,
    /// The miss from your own ground, and its fix.
    pub miss: Option<String>,
    pub setback: Option<String>,
    /// Your AD's word.
    pub ad: String,
    /// The paper's take.
    pub paper: String,
}

fn spring_words(n: usize) -> &'static str {
    match n {
        1 => "first",
        2 => "second",
        3 => "third",
        4 => "fourth",
        5 => "fifth",
        _ => "",
    }
}

/// Builds the report at the verdict.
pub(crate) fn build(w: &World, prog: usize, d: Date) -> Report {
    let c = &w.ground.campaign;
    let year = c.year;
    let class = c.class;
    let springs = 1 + c.reports.iter().filter(|r| r.year < year).count();
    let title = match spring_words(springs) {
        "" => format!("Your {year} spring at {}", w.program_name(prog)),
        s if springs == 1 => format!("Your {s} spring"),
        s => format!("Your {s} spring at {}", w.program_name(prog)),
    };
    // The goals: this spring's, each with its reading now.
    let goals: Vec<GoalLine> = c
        .goals
        .iter()
        .filter(|h| h.year == year || (h.open() && h.aim.classes().any(|x| x == class)))
        .map(|h| {
            let part = h.parts.iter().find(|p| p.class == class);
            let r = goals::read(w, prog, &h.aim, class);
            let share = if h.dropped.is_some() {
                0.0
            } else {
                part.and_then(|p| p.pace).unwrap_or(if r.waiting { 0.5 } else { r.share })
            };
            GoalLine {
                slot: h.slot,
                words: goals::words(w, &h.aim),
                verdict: Verdict::of(share),
                why: if h.dropped.is_some() { "You dropped it.".into() } else { why(w, prog, &h.aim, &r) },
                bold: h.bold > 0.0,
                public: h.told.press,
                dropped: h.dropped.is_some(),
            }
        })
        .collect();
    // Finds for this class and the one before it.
    let finds: Vec<FindLine> = w
        .ground
        .finds
        .iter()
        .filter(|f| f.prog as usize == prog && f.class_of >= class - 1 && f.class_of <= class)
        .map(|f| {
            let p = crate::world::folk::person(w, f.kid).map(|i| &w.people[i]);
            let state = match p.and_then(goals::went) {
                _ if f.lost.is_some() => "lost to a rival".to_string(),
                Some(j) if j == prog => {
                    if p.is_some_and(|p| p.cal.signed.is_some()) { "signed with you" } else { "committed to you" }
                        .into()
                }
                Some(j) => format!("committed to {}", w.program_name(j)),
                None => "still open".into(),
            };
            FindLine {
                kid: f.kid,
                name: f.name.clone(),
                pos: f.pos.name().to_string(),
                stars_then: f.stars,
                stars_now: p.map_or(f.stars, |p| p.stars),
                state,
                steps: f.steps,
                blooms: f.blooms.iter().map(|b| b.words.clone()).collect(),
            }
        })
        .collect();
    // Your ground February to August.
    let talent = standing::talents(w);
    let by_area = areas::schools_by_area(w);
    let mut rows: Vec<AreaLine> = Vec::new();
    for &(a, then) in &c.feb {
        let schools = &by_area[a as usize];
        let now = standing::area_points(w, prog, schools, &talent);
        let leader = standing::leader(w, schools, &talent).map(|(l, _)| w.program_name(l).to_string());
        let lv = |p: f32| standing::Level::of(p).name().to_string();
        rows.push(AreaLine {
            area: a,
            name: areas::area(a).name.to_string(),
            feb: lv(then),
            aug: lv(now),
            moved: now - then,
            lost: standing::Level::of(now) < standing::Level::of(then),
            leader,
        });
    }
    let headline = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.moved > 0.5)
        .max_by(|a, b| a.1.moved.total_cmp(&b.1.moved))
        .map(|x| x.0);
    let outworked = outworked(w, prog, &c.feb.iter().map(|x| x.0).collect::<Vec<_>>(), d);
    let men = w
        .ground
        .men
        .iter()
        .filter(|m| m.finds > 0)
        .map(|m| {
            (
                m.coach,
                w.coaches.get(m.coach as usize).map_or(String::new(), |c| c.name.clone()),
                m.finds,
                m.blooms,
                m.big,
            )
        })
        .collect();
    let miss = w.ground.misses.iter().rev().find(|m| m.date.year() >= year - 1).map(|m| {
        let to = m.went.map_or(String::new(), |j| format!(" at {}", w.program_name(j as usize)));
        format!("{} ({}, {}){to}: {}. {}", m.name, m.pos.name(), w.school_name(m.school as usize), m.what, m.why)
    });
    let setback = w
        .ground
        .finds
        .iter()
        .filter(|f| f.prog as usize == prog)
        .filter_map(|f| f.setback.as_ref().map(|s| (s.0, format!("{}: {}", f.name, s.2))))
        .filter(|s| s.0.year() >= year - 1)
        .max_by_key(|s| s.0)
        .map(|s| s.1);
    let team = Team::College(prog);
    let b = crate::world::folk::boss::boss(w, team);
    let made = goals.iter().filter(|g| g.slot == Slot::Ad && g.verdict >= Verdict::Mostly).count();
    let his = goals.iter().filter(|g| g.slot == Slot::Ad).count();
    let ad = format!(
        "{}: \"{}\" {}",
        b.name,
        if his == 0 {
            "We'll see where the class ends up."
        } else if made == his {
            "That's what I asked for. Keep it going to signing day."
        } else if made > 0 {
            "Some of it's there. I'll be watching December."
        } else {
            "That isn't the spring we talked about."
        },
        crate::world::folk::boss::trust_words(crate::world::folk::boss::trust(w, team), b.she)
    );
    let paper = paper(w, prog, class, &goals);
    Report { year, date: d, title, goals, finds, areas: rows, headline, outworked, men, miss, setback, ad, paper }
}

/// Why a goal reads what it reads, from the world: the reading, and for a
/// kid, where he stands.
fn why(w: &World, prog: usize, a: &goals::Aim, r: &goals::Read) -> String {
    let d = a.def();
    let mut s = r.words.clone();
    if d.subj == goals::Subj::Area {
        let st = w.ground.campaign.feb.iter().find(|x| x.0 as u32 == a.of).map(|x| x.1);
        if let Some(p) = st {
            s.push_str(&format!(" (it was {} in February)", standing::Level::of(p).name()));
        }
    }
    if let Some(l) = w.ground.campaign.lost.iter().rev().find(|x| x.0 == a.class && x.2.is_some())
        && matches!(d.id, 8 | 42 | 31 | 32 | 33)
        && let Some(i) = crate::world::folk::person(w, l.1)
    {
        s.push_str(&format!("; {} went to {}", w.people[i].name, w.program_name(l.2.unwrap_or(0) as usize)));
    }
    let _ = prog;
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str() + ".",
        None => String::new(),
    }
}

/// Each area's contacts with its recruits by program (campus visits, home
/// visits and drop-ins, the dated signs), February to the verdict: yours
/// against the busiest other staff's.
fn outworked(w: &World, prog: usize, list: &[u16], d: Date) -> Vec<Outworked> {
    let from = super::Spring::of(d.year()).meeting;
    let mut out = Vec::new();
    let mut by: std::collections::HashMap<u16, std::collections::HashMap<usize, u32>> = Default::default();
    for p in
        w.people.iter().filter(|p| matches!(p.stage, crate::world::Stage::HighSchool | crate::world::Stage::PrepSchool))
    {
        let Some(h) = w.high_schools.get(p.hs) else { continue };
        if !list.contains(&h.ground_area) {
            continue;
        }
        let e = by.entry(h.ground_area).or_default();
        for v in p.cal.visits.iter().filter(|v| v.1 >= from && v.1 <= d) {
            *e.entry(v.0).or_default() += 1;
        }
    }
    // Your staff's own road this spring counts too: each school visit of
    // your men is a sign the coaches saw.
    for m in &w.ground.memories {
        if m.holder != crate::world::ground::Holder::Program(prog as u32) || m.year.0 != d.year() {
            continue;
        }
        if let crate::world::ground::Who::Coach(_) = m.who
            && let Some(h) = crate::world::ground::people::school(w, m.who)
            && let Some(s) = w.high_schools.get(h)
            && list.contains(&s.ground_area)
        {
            *by.entry(s.ground_area).or_default().entry(prog).or_default() += m.year.1 as u32;
        }
    }
    for &a in list {
        let Some(e) = by.get(&a) else { continue };
        let yours = e.get(&prog).copied().unwrap_or(0);
        let rival = e
            .iter()
            .filter(|x| *x.0 != prog)
            .max_by(|x, y| x.1.cmp(y.1).then(y.0.cmp(x.0)))
            .map(|(&j, &n)| (w.program_name(j).to_string(), n));
        if yours == 0 && rival.is_none() {
            continue;
        }
        out.push(Outworked { area: a, name: areas::area(a).name.to_string(), yours, rival });
    }
    out.sort_by(|a, b| {
        (b.yours + b.rival.as_ref().map_or(0, |r| r.1)).cmp(&(a.yours + a.rival.as_ref().map_or(0, |r| r.1)))
    });
    out.truncate(8);
    out
}

/// The local paper's take: a public goal first, else the class's place.
fn paper(w: &World, prog: usize, class: i32, goals: &[GoalLine]) -> String {
    if let Some(g) = goals.iter().find(|g| g.public) {
        return format!(
            "The paper: \"{} said it at media days: {}. {}.\"",
            crate::world::folk::scene::your_name(w),
            lower(&g.words),
            match g.verdict {
                Verdict::Done => "He's there",
                Verdict::Mostly => "Most of the way",
                Verdict::Partly => "Some of the way",
                Verdict::Missed => "Not close",
            }
        );
    }
    let scores = goals::class_scores(w, class);
    let conf = &w.programs[prog].conference;
    let mut r = 0;
    for &(_, j) in &scores {
        if &w.programs[j].conference == conf {
            r += 1;
            if j == prog {
                let n = goals::class_of(w, prog, class).len();
                return format!("The paper: \"{n} commitments, a class running {} in the conference.\"", ordinal(r));
            }
        }
    }
    String::new()
}

fn lower(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn ordinal(n: u32) -> String {
    let suf = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suf}")
}
