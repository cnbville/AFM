//! The fever (Capital lane C3; `docs/design/capital.md`, CP7 and CP8).
//!
//! Big moments set donors giving: a national title most, a Heisman next, a
//! conference title and a top-10 pick the same (CP7: the pick is half a
//! national title), the playoff, a bowl win and a rivalry win less. Each
//! moment is kept with its day; the fever is what is left of them today:
//! each fades to nothing over two years, faster at first, and they stack
//! over a run (CP8). A conference title alone reads 0.39 the day it is won,
//! a quarter of that a year on and about a tenth by the spring after the
//! next season. The drive's gifts read it (`gifts`), and so does the
//! collective's money (`program::update_collective`). Sizes are Claude's
//! (proposed, not confirmed) unless the spec gives them.

use crate::calendar::{Date, Level, Loudness};
use crate::world::World;
use crate::world::data::Division;
use crate::world::schedule::GameKind;

/// A moment that sets donors giving.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum What {
    NationalTitle,
    Heisman,
    ConferenceTitle,
    /// One of the draft's first ten picks.
    TopTenPick,
    Playoff,
    BowlWin,
    RivalryWin,
}

impl What {
    /// How hard it starts the fever (CP7: the conference title and a top-10
    /// pick at half the national title, the Heisman between them; the rest
    /// proposed, not confirmed).
    pub fn strength(self) -> f64 {
        match self {
            What::NationalTitle => 1.0,
            What::Heisman => 0.75,
            What::ConferenceTitle | What::TopTenPick => 0.5,
            What::Playoff => 0.25,
            What::BowlWin => 0.12,
            What::RivalryWin => 0.08,
        }
    }
    pub fn words(self) -> &'static str {
        match self {
            What::NationalTitle => "the national title",
            What::Heisman => "the Heisman",
            What::ConferenceTitle => "the conference title",
            What::TopTenPick => "a top-10 pick",
            What::Playoff => "the playoff",
            What::BowlWin => "the bowl win",
            What::RivalryWin => "the rivalry win",
        }
    }
}

/// A moment at a program.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Moment {
    pub program: u32,
    pub date: Date,
    pub what: What,
    /// The player, for a Heisman or a pick.
    pub who: Option<u32>,
}

/// A moment fades to nothing over this many months (CP8: about two years).
pub const FADE_MONTHS: f64 = 24.0;
/// Moments are kept a little past their fade.
const KEEP_DAYS: i32 = 800;

/// What is left of one moment `days` after it: (1 - t / 24 months)²,
/// so it fades fast at first and is gone at two years.
pub fn fade(days: i32) -> f64 {
    let months = days.max(0) as f64 / 30.44;
    if months >= FADE_MONTHS { 0.0 } else { (1.0 - months / FADE_MONTHS).powi(2) }
}

/// A program's fever today, 0-1: the moments' strengths, each faded, summed
/// and read as 1 - e^-sum, so a run stacks toward the top but never past it.
pub fn fever(w: &World, p: usize) -> f64 {
    fever_on(&w.capital.moments, p, w.today)
}

pub fn fever_on(moments: &[Moment], p: usize, day: Date) -> f64 {
    let sum: f64 = moments
        .iter()
        .filter(|m| m.program == p as u32 && m.date <= day)
        .map(|m| m.what.strength() * fade(m.date.days_to(day)))
        .sum();
    1.0 - (-sum).exp()
}

/// The moment that is lifting a program's fever most on a day (its strength
/// times its fade), if any is still worth a word: what brought a gift.
pub fn brought(w: &World, p: usize, day: Date) -> Option<(What, Option<u32>)> {
    w.capital
        .moments
        .iter()
        .filter(|m| m.program == p as u32 && m.date <= day)
        .map(|m| (m.what.strength() * fade(m.date.days_to(day)), m))
        .filter(|(s, _)| *s >= BROUGHT_MIN)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, m)| (m.what, m.who))
}

/// A moment must still read at least this to be named as what brought a
/// gift (proposed, not confirmed): a conference title is gone at about
/// 18 months, a rivalry win after a few weeks.
pub const BROUGHT_MIN: f64 = 0.04;

/// Every program's fever today, by program.
pub fn all(w: &World) -> Vec<f64> {
    let mut sum = vec![0.0f64; w.programs.len()];
    let day = w.today;
    for m in w.capital.moments.iter().filter(|m| m.date <= day) {
        if let Some(s) = sum.get_mut(m.program as usize) {
            *s += m.what.strength() * fade(m.date.days_to(day));
        }
    }
    sum.into_iter().map(|s| 1.0 - (-s).exp()).collect()
}

/// The day's look for new moments: the college season once it is over (the
/// titles, the playoff, bowls, rivalry games and the Heisman) and the draft
/// on 15 May (its first ten picks, by the college each man came from).
pub(crate) fn day(w: &mut World, d: Date) {
    let mut found: Vec<Moment> = Vec::new();
    if let Some(cs) = w.college.as_ref().filter(|c| c.finished && c.year > w.capital.scanned) {
        let year = cs.year;
        for g in &cs.games {
            let Some(pl) = g.played.as_ref().filter(|x| !x.cancelled) else { continue };
            let (win, lose) = if pl.home_won { (g.home, g.away) } else { (g.away, g.home) };
            let at = |p: usize, what| Moment { program: p as u32, date: g.date, what, who: None };
            match g.kind {
                GameKind::Title | GameKind::Playoff(_) => {
                    // Counted once a season: at the first playoff game (the
                    // title game alone in the years without a bracket).
                    for t in [win, lose] {
                        let first = !cs.games.iter().any(|h| {
                            matches!(h.kind, GameKind::Playoff(_) | GameKind::Title)
                                && h.date < g.date
                                && (h.home == t || h.away == t)
                        });
                        if first {
                            found.push(at(t, What::Playoff));
                        }
                    }
                }
                GameKind::Bowl(_) | GameKind::Celebration => found.push(at(win, What::BowlWin)),
                GameKind::Regular if g.rivalry.is_some() => found.push(at(win, What::RivalryWin)),
                _ => {}
            }
        }
        // A champion's day: the title game where there was one, else the
        // season's last game.
        let last = |p: usize, kind: fn(&GameKind) -> bool| {
            cs.games
                .iter()
                .filter(|g| (g.home == p || g.away == p) && g.played.is_some() && kind(&g.kind))
                .map(|g| g.date)
                .max()
                .or_else(|| cs.games.iter().filter(|g| g.home == p || g.away == p).map(|g| g.date).max())
                .unwrap_or(d)
        };
        for (p, r) in cs.results.iter().enumerate() {
            if r.conference_title {
                let date = last(p, |k| *k == GameKind::ConferenceTitle);
                found.push(Moment { program: p as u32, date, what: What::ConferenceTitle, who: None });
            }
            if r.national_title {
                let date = last(p, |k| matches!(k, GameKind::Title | GameKind::Playoff(_)));
                found.push(Moment { program: p as u32, date, what: What::NationalTitle, who: None });
            }
        }
        for &(award, id, p) in &cs.awards {
            if award == "Heisman Trophy" {
                found.push(Moment { program: p as u32, date: d, what: What::Heisman, who: Some(id) });
            }
        }
        w.capital.scanned = year;
    }
    if d.month() == 5 && d.day() == 15 && w.capital.drafted < w.year {
        let year = w.year;
        for p in &w.people {
            let Some((y, _, pick, _)) = p.pro.as_ref().and_then(|c| c.draft) else { continue };
            if y != year || pick == 0 || pick > 10 {
                continue;
            }
            let Some(&prog) = p.path.last().or(p.college.as_ref()) else { continue };
            if w.programs.get(prog).is_some_and(|x| x.division != Division::JUCO) {
                found.push(Moment { program: prog as u32, date: d, what: What::TopTenPick, who: Some(p.player.id) });
            }
        }
        w.capital.drafted = year;
    }
    let live = w.year >= w.options.start_year;
    for m in &found {
        if live {
            news(w, m);
        }
    }
    // Junior colleges keep no donors.
    found.retain(|m| w.programs.get(m.program as usize).is_some_and(|x| x.division != Division::JUCO));
    w.capital.moments.extend(found);
    w.capital.moments.retain(|m| m.date.days_to(d) <= KEEP_DAYS);
}

/// The moment's news line (CP7: "Donors answer the title"): your program's
/// every moment, and the big ones at the Power 4.
fn news(w: &mut World, m: &Moment) {
    let p = m.program as usize;
    let Some(prog) = w.programs.get(p) else { return };
    let mine = w.seat_program(p);
    let big = m.what.strength() >= 0.5 && prog.division == Division::FBS && prog.power_conference(w.year);
    if !(mine || big) {
        return;
    }
    let name = prog.name(w.options.names).to_string();
    let text = match m.what {
        What::NationalTitle => {
            format!("Donors answer the title: {name}'s development office can't keep up with the calls.")
        }
        What::Heisman => format!("Donors answer the Heisman: {name}'s phones ring all week."),
        What::ConferenceTitle => format!("Donors answer the conference title: gifts to {name} pick up."),
        What::TopTenPick => format!("A top-10 pick for {name}: donors call about the program's next building."),
        What::Playoff => format!("The playoff run has {name}'s donors talking."),
        What::BowlWin => format!("A bowl win warms {name}'s donors."),
        What::RivalryWin => format!("The rivalry win has {name}'s donors in good spirits."),
    };
    let loud = if mine { Loudness::Medium } else { Loudness::Small };
    let e = crate::world::finance::story(w, Level::College, loud, text).program(p);
    crate::world::finance::post(w, e);
}
