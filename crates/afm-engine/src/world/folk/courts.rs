//! Courts and appeals (people topic 7, the build 9 leftover): trouble runs
//! the real process of its level and era. The police and the courts first
//! (the arrest, the charge, court dates, the outcome), then the league or
//! the association, then the club or school. Every step has its date, and
//! appeals take weeks: an NFL suspension goes to the hearing officer, a
//! team suspension past the CBA's four weeks to an arbitrator, an on-field
//! fine to the appeals officers.
//!
//! What is stored is the open cases only (a few dozen in the whole league
//! at a time) and the newest closed ones. Rates no source publishes are
//! Claude's picks (proposed, not confirmed).

use crate::calendar::{Date, Level, Loudness, Weekday};
use crate::world::World;

/// One case on its way through the courts or an appeal.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Case {
    pub id: u32,
    pub level: Level,
    /// What it is, in words ("a DUI arrest").
    pub what: String,
    pub opened: Date,
    /// The next step's day and what it is.
    pub next: Date,
    pub step: Step,
    /// Decided when it opens, revealed on its day.
    pub guilty: bool,
    pub outcome: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Step {
    /// Arrested: the charge comes within weeks.
    Arrest,
    /// Charged: the first court date.
    Charged,
    /// In court: the plea, the trial or the dismissal.
    Court,
    /// A league suspension under appeal to the hearing officer.
    Appeal {
        games: u8,
    },
    /// A team suspension past the CBA cap, before an arbitrator.
    Grievance {
        games: u8,
    },
    /// An on-field fine before the appeals officers (dollars).
    FineAppeal {
        amount: u32,
    },
    Closed,
}

impl Step {
    pub fn words(self) -> String {
        match self {
            Step::Arrest => "arrested; charges pending".into(),
            Step::Charged => "charged; a court date is set".into(),
            Step::Court => "in court".into(),
            Step::Appeal { games } => format!("appealing a {games}-game suspension"),
            Step::Grievance { games } => format!("grievance over a {games}-game team suspension"),
            Step::FineAppeal { amount } => format!("appealing a ${} fine", super::facts::thousands(amount)),
            Step::Closed => "closed".into(),
        }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Courts {
    pub open: Vec<Case>,
    /// The newest closed cases (120 kept), for player pages and the news.
    pub closed: Vec<Case>,
    /// Mirrored league cases already in the courts (person index and ruling
    /// day), so each is opened once.
    pub seen: Vec<(u32, Date)>,
    /// Your players' police matters already in the courts (player id and
    /// which of his incidents): one incident is one case.
    pub amateur: Vec<(u32, i32)>,
}

impl Courts {
    /// An open case for a player, in words (the takeover's conduct report).
    pub fn open_case(&self, id: u32) -> Option<String> {
        self.open.iter().find(|c| c.id == id).map(|c| format!("{}: {}", c.what, c.step.words()))
    }
}

fn close(w: &mut World, k: usize, words: String) {
    let mut c = w.folk.courts.open.remove(k);
    c.step = Step::Closed;
    c.outcome = Some(words);
    w.folk.courts.closed.push(c);
    let n = w.folk.courts.closed.len();
    if n > 120 {
        w.folk.courts.closed.drain(..n - 120);
    }
}

fn mine(w: &World, id: u32) -> bool {
    super::Team::seat(w).is_some_and(|t| t.roster(w).iter().any(|&i| w.people[i].player.id == id))
}

/// A team suspension past the CBA's cap goes to an arbitrator (topic 8):
/// he can cut it to four weeks and award back pay (about half the time,
/// proposed, not confirmed).
pub(crate) fn grievance(w: &mut World, id: u32, games: u32) {
    let today = w.today;
    let guilty = super::roll(w, 0x6E1E, id as u64 ^ today.0 as u64) < 0.5;
    w.folk.courts.open.push(Case {
        id,
        level: Level::Nfl,
        what: "a team suspension".into(),
        opened: today,
        next: today.add(35),
        step: Step::Grievance { games: games.min(255) as u8 },
        guilty,
        outcome: None,
    });
    w.folk.mine.hard = (w.folk.mine.hard + 0.1).min(1.0);
}

/// An on-field fine appealed (P2): heard in about two weeks; the appeals
/// officers cut about a third of them, usually by half.
pub(crate) fn fine_appeal(w: &mut World, id: u32, amount: u32, d: Date) {
    let upheld = super::roll(w, 0xF1A9, id as u64 ^ d.0 as u64) >= 0.33;
    w.folk.courts.open.push(Case {
        id,
        level: Level::Nfl,
        what: "an on-field fine".into(),
        opened: d,
        next: d.add(14),
        step: Step::FineAppeal { amount },
        guilty: upheld,
        outcome: None,
    });
}

/// A league suspension handed down (`people::rule`): the player appeals
/// most of them. PED and drug suspensions are almost never cut; conduct
/// cases sometimes are (proposed, not confirmed: one in five, by a game).
pub(crate) fn suspension(w: &mut World, i: usize, games: u32, what: &str, ped: bool) {
    let id = w.people[i].player.id;
    let today = w.today;
    let appeal = super::roll(w, 0xA99E, id as u64 ^ today.0 as u64) < 0.75;
    if !appeal {
        return;
    }
    let upheld = super::roll(w, 0xA99F, id as u64 ^ today.0 as u64) >= if ped { 0.04 } else { 0.2 };
    w.folk.courts.open.push(Case {
        id,
        level: Level::Nfl,
        what: what.to_string(),
        opened: today,
        next: today.add(14 + (super::roll(w, 0xA9A0, id as u64) * 14.0) as i32),
        step: Step::Appeal { games: games.min(255) as u8 },
        guilty: upheld,
        outcome: None,
    });
}

/// Mirrors new league cases into the courts: the arrest, the charge
/// (about three weeks later), court (two to six months), the outcome the
/// league case already drew.
fn mirror(w: &mut World) {
    let fresh: Vec<(usize, &'static str, Date, bool)> = w
        .market
        .cases
        .iter()
        .filter(|c| !matches!(c.kind, crate::world::people::Trouble::Ped | crate::world::people::Trouble::Gambling))
        .map(|c| (c.person, c.kind.words(), c.ruling, c.guilty))
        .collect();
    for (i, what, ruling, guilty) in fresh {
        let Some(p) = w.people.get(i) else { continue };
        let id = p.player.id;
        if w.folk.courts.seen.contains(&(id, ruling)) {
            continue;
        }
        w.folk.courts.seen.push((id, ruling));
        let today = w.today;
        w.folk.courts.open.push(Case {
            id,
            level: Level::Nfl,
            what: what.to_string(),
            opened: today,
            next: today.add(21),
            step: Step::Arrest,
            guilty,
            outcome: None,
        });
    }
    let n = w.folk.courts.seen.len();
    if n > 400 {
        w.folk.courts.seen.drain(..n - 400);
    }
}

/// Opens a case for a college or high school player of yours whose trouble
/// is a police matter (the season's roll, `amateur_kind`).
pub(crate) fn amateur(w: &mut World, i: usize, what: &str, nth: i32) {
    let police =
        what.contains("arrest") || what.contains("DUI") || what.contains("theft") || what.contains("drug case");
    if !police {
        return;
    }
    let id = w.people[i].player.id;
    if w.folk.courts.open.iter().any(|c| c.id == id) || w.folk.courts.amateur.contains(&(id, nth)) {
        return;
    }
    w.folk.courts.amateur.push((id, nth));
    let n = w.folk.courts.amateur.len();
    if n > 400 {
        w.folk.courts.amateur.drain(..n - 400);
    }
    let today = w.today;
    let level = if w.people[i].stage == crate::world::Stage::College { Level::College } else { Level::HighSchool };
    let guilty = super::roll(w, 0xC0A7, id as u64 ^ today.0 as u64) < 0.5;
    w.folk.courts.open.push(Case {
        id,
        level,
        what: what.to_string(),
        opened: today,
        next: today.add(20),
        step: Step::Arrest,
        guilty,
        outcome: None,
    });
}

fn step(w: &mut World, d: Date) {
    let mut k = 0;
    while k < w.folk.courts.open.len() {
        let c = w.folk.courts.open[k].clone();
        if c.next > d {
            k += 1;
            continue;
        }
        let name = super::person(w, c.id).map_or_else(|| "A former player".into(), |i| w.people[i].name.clone());
        let yours = mine(w, c.id);
        let roll = super::roll(w, 0xC0B1 ^ d.0 as u64, c.id as u64);
        let (next, words): (Option<(Step, i32)>, Option<String>) = match c.step {
            Step::Arrest => {
                if !c.guilty && roll < 0.3 {
                    (None, Some(format!("Prosecutors declined to charge {name} ({}).", c.what)))
                } else {
                    (
                        Some((Step::Charged, 30 + (roll * 60.0) as i32)),
                        Some(format!("{name} is formally charged ({}).", c.what)),
                    )
                }
            }
            Step::Charged => (Some((Step::Court, 30 + (roll * 90.0) as i32)), None),
            Step::Court => {
                let out = if c.guilty {
                    if roll < 0.7 { "pleaded to a lesser charge" } else { "was convicted" }
                } else if roll < 0.6 {
                    "had the charges dropped"
                } else {
                    "was acquitted"
                };
                (None, Some(format!("{name} {out} ({}).", c.what)))
            }
            Step::Appeal { games } => {
                if c.guilty {
                    (None, Some(format!("The hearing officer upheld {name}'s {games}-game suspension.")))
                } else {
                    let cut = (games as u32).saturating_sub(1).max(games as u32 / 2);
                    if let Some(i) = super::person(w, c.id)
                        && let Some(pc) = w.people[i].pro.as_mut()
                    {
                        pc.suspended = pc.suspended.saturating_sub(games as u32 - cut);
                    }
                    (
                        None,
                        Some(format!(
                            "On appeal, {name}'s suspension was cut from {games} {} to {cut}.",
                            super::noun(games, "game")
                        )),
                    )
                }
            }
            Step::Grievance { games } => {
                if c.guilty {
                    (None, Some(format!("The arbitrator upheld your {games}-game suspension of {name}.")))
                } else {
                    if let Some(i) = super::person(w, c.id)
                        && let Some(pc) = w.people[i].pro.as_mut()
                    {
                        pc.suspended = pc.suspended.saturating_sub(games as u32 - 4);
                        pc.earnings += 0.02 * (games as f64 - 4.0);
                    }
                    (
                        None,
                        Some(format!(
                            "The arbitrator cut {name}'s team suspension to four weeks and awarded back pay."
                        )),
                    )
                }
            }
            Step::FineAppeal { amount } => {
                if c.guilty {
                    (None, Some(format!("{name}'s ${} fine was upheld on appeal.", super::facts::thousands(amount))))
                } else {
                    if let Some(i) = super::person(w, c.id)
                        && let Some(pc) = w.people[i].pro.as_mut()
                    {
                        pc.earnings += amount as f64 / 2.0e6;
                    }
                    (None, Some(format!("{name}'s fine was cut in half on appeal.")))
                }
            }
            Step::Closed => (None, None),
        };
        if let Some(text) = &words {
            if let Some(i) = super::person(w, c.id) {
                w.people[i].note(text.clone());
            }
            if yours {
                super::post_mine(w, d, Loudness::Small, false, "people: courts", text);
                super::note(w, "rules", "The courts".into(), text.clone(), None);
            }
        }
        match next {
            Some((s, days)) => {
                let cc = &mut w.folk.courts.open[k];
                cc.step = s;
                cc.next = d.add(days);
                k += 1;
            }
            None => close(w, k, words.unwrap_or_default()),
        }
    }
}

pub(crate) fn day(w: &mut World, d: Date) {
    if d.weekday() == Weekday::Wed {
        mirror(w);
        // Your college or high school players in trouble with the police
        // this season run the courts too.
        if let Some(t) = super::Team::seat(w)
            && t.level() != Level::Nfl
        {
            // This season's incident only: a dismissal for team rules is no
            // police matter, and an old note is an old case.
            let found: Vec<(usize, String, i32)> = t
                .roster(w)
                .into_iter()
                .filter(|&i| w.people[i].cal.trouble)
                .filter_map(|i| {
                    let notes = &w.people[i].notes;
                    let n = notes.iter().rev().find(|n| {
                        n.starts_with("Off-field trouble")
                            || n.starts_with("Dismissed from the team")
                            || n.starts_with("Removed from the team")
                    })?;
                    let what = n.strip_prefix("Off-field trouble")?;
                    let nth = notes.iter().filter(|n| n.starts_with("Off-field trouble")).count() as i32;
                    Some((i, what.split(": ").nth(1).unwrap_or("").trim_end_matches('.').to_string(), nth))
                })
                .collect();
            for (i, what, nth) in found {
                amateur(w, i, &what, nth);
            }
        }
    }
    step(w, d);
}
