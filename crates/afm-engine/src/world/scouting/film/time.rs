//! Film time (SK59-SK63): one weekly ledger of your film hours on the real
//! calendar, set by how hard you work, your eye tiring over a long day,
//! your work ethic, and what a locked-in coach earns.
//!
//! The ledger is lane 7's (`own::film_week`): every film hour you spend,
//! a player's cut-up, your opponent's tape for the game plan, your own
//! practice film, comes off it; there is no second budget. In season the
//! week's film includes your game prep (section 9's opponent tape) and
//! about 3 more hours for recruits at normal hours; after signing day, in
//! the spring and summer it opens up to 15-25; May is an FBS film month
//! (no off-campus recruiting for head coaches since 2008); an NFL club's
//! film months are February to April; a high school coach's the summer.
//! The hours past a steady 65-hour week all go to film (SK61). Sizes no
//! source gives are Claude's, proposed, not confirmed.

use super::session::Mode;
use crate::calendar::{Date, Weekday};
use crate::positions::Position;
use crate::world::World;
use crate::world::scouting::readers::{self, Staff};

/// How hard you work this week (SK61).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Work {
    Cruising,
    #[default]
    Steady,
    LockedIn,
    Junkie,
}

impl Work {
    pub const ALL: [Work; 4] = [Work::Cruising, Work::Steady, Work::LockedIn, Work::Junkie];

    /// Hours in the building a week.
    pub fn hours(self) -> f32 {
        match self {
            Work::Cruising => 55.0,
            Work::Steady => 65.0,
            Work::LockedIn => 80.0,
            Work::Junkie => 95.0,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Work::Cruising => "cruising",
            Work::Steady => "steady",
            Work::LockedIn => "locked_in",
            Work::Junkie => "junkie",
        }
    }

    pub fn from_key(k: &str) -> Option<Work> {
        Work::ALL.into_iter().find(|w| w.key() == k)
    }

    pub fn words(self) -> &'static str {
        match self {
            Work::Cruising => "Cruising",
            Work::Steady => "Steady",
            Work::LockedIn => "Locked in",
            Work::Junkie => "Film junkie",
        }
    }
}

/// A week you worked.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Week {
    pub monday: Date,
    pub work: Work,
    pub film: f32,
    pub budget: f32,
}

pub fn monday(d: Date) -> Date {
    let mut d = d;
    while d.weekday() != Weekday::Mon {
        d = d.add(-1);
    }
    d
}

/// The week's film at normal hours: (recruit and prospect film, game
/// prep). Game prep only in a week with a game of yours.
pub fn base(w: &World) -> (f32, f32) {
    let d = w.today;
    let (_, m, day) = d.parts();
    let wk = monday(d);
    let game_week = w.seat_games().iter().any(|g| g.date >= wk && g.date < wk.add(7))
        || crate::world::plans::upcoming(w).first().is_some_and(|g| g.date < wk.add(7));
    let dt = &w.dates;
    match readers::seat_staff(w) {
        Some(Staff::Club(_)) => {
            let recruit = if d >= dt.nfl_kickoff && d <= dt.super_bowl {
                3.0
            } else if (2..=4).contains(&m) && d < dt.draft[0].date {
                20.0
            } else if (d >= dt.summer_break.0 && d <= dt.summer_break.1) || d >= dt.rookies_report {
                4.0
            } else {
                10.0
            };
            (recruit, if game_week { 12.0 } else { 0.0 })
        }
        Some(Staff::Program(p)) => {
            let fbs = w.programs[p].division == crate::world::data::Division::FBS;
            let recruit = if d >= dt.fall_camp && d <= dt.bowl_season.1 {
                3.0
            } else {
                match m {
                    1 => 6.0,
                    2 if day <= 7 => 6.0,
                    2..=4 => 15.0,
                    5 if fbs && w.year >= 2008 => 25.0,
                    5 => 18.0,
                    6 => 15.0,
                    7 => 20.0,
                    _ => 6.0,
                }
            };
            (recruit, if game_week { 12.0 } else { 0.0 })
        }
        Some(Staff::School(_)) => {
            let recruit = match m {
                8..=11 => 3.0,
                6 | 7 => 18.0,
                _ => 6.0,
            };
            (recruit, if game_week { 6.0 } else { 0.0 })
        }
        None => (0.0, 0.0),
    }
}

/// The week's film hours: normal hours plus every hour past a steady
/// week (SK61); a cruising week gives up film first, down to 40%.
pub fn budget(w: &World) -> f32 {
    let (r, p) = base(w);
    if r + p <= 0.0 {
        return 0.0;
    }
    let extra = w.scouting.film.work.hours() - Work::Steady.hours();
    let b = (r + p + extra).max(0.4 * (r + p));
    if rested(w) { 0.0 } else { b }
}

/// Film hours spent this week.
pub fn used(w: &World) -> f32 {
    w.scouting.own.film_week.filter(|x| x.0 == monday(w.today)).map_or(0.0, |x| x.1)
}

pub fn left(w: &World) -> f32 {
    (budget(w) - used(w)).max(0.0)
}

/// Film hours today.
pub fn today(w: &World) -> f32 {
    w.scouting.film.today.filter(|x| x.0 == w.today).map_or(0.0, |x| x.1)
}

/// Your work ethic (SK62), 1-20.
pub fn ethic(w: &World) -> f64 {
    w.scouting.film.ethic.map_or(12.0, |e| e as f64)
}

/// Sent home to rest after wearing yourself out.
pub fn rested(w: &World) -> bool {
    w.scouting.film.rest_until.is_some_and(|d| w.today < d)
}

/// How tired your eye is, 0-1 (SK59): nothing for the first 3 to 4 hours
/// of the day (longer with a strong work ethic), then fewer things
/// half-catch your eye and more slip by; weeks of wear make it worse.
pub fn tiredness(w: &World) -> f64 {
    let e = ethic(w);
    let start = 3.0 + 0.1 * (e - 10.0);
    let over = (today(w) as f64 - start).max(0.0) / (2.0 + 0.1 * (e - 10.0)).max(1.0);
    (over + 0.3 * w.scouting.film.wear as f64).clamp(0.0, 1.0)
}

/// What your hours at a position taught your eye (SK63): about a point
/// of evaluation after 200 hours of film there, at most 1.5.
pub fn position_edge(w: &World, pos: Position) -> f64 {
    let h = w.scouting.film.pos_hours.get(pos.index()).copied().unwrap_or(0.0) as f64;
    1.5 * (1.0 - (-h / 200.0).exp()) - 0.5 * w.scouting.film.wear as f64
}

/// Can you watch now? A game from the stands is its own day; tape needs
/// film hours left and a coach who isn't home resting.
pub fn can_watch(w: &World, mode: Mode) -> Result<(), String> {
    if rested(w) {
        return Err("You're home resting after wearing yourself out: no film until you're back.".into());
    }
    if mode == Mode::Live {
        return Ok(());
    }
    let need = if mode == Mode::Quick { super::session::QUICK_MIN } else { super::session::SNAP_MIN } / 60.0;
    if left(w) + 0.01 < need {
        return Err(format!(
            "Your film hours are spent this week ({:.0} of {:.0}): work longer hours, give it to your staff, or watch on Monday.",
            used(w),
            budget(w)
        ));
    }
    Ok(())
}

/// Spends film hours: the week's ledger, the day, the position, the year.
pub fn spend(w: &mut World, hours: f32, pos: Position) -> Result<(), String> {
    if rested(w) {
        return Err("You're home resting: no film until you're back.".into());
    }
    if left(w) + 0.01 < hours {
        return Err(format!("Your film hours are spent this week ({:.0} of {:.0}).", used(w), budget(w)));
    }
    charge(w, hours, Some(pos));
    Ok(())
}

/// Counts film hours on the ledger without the check (a tape studied for
/// the game plan was already checked against `left`).
pub fn charge(w: &mut World, hours: f32, pos: Option<Position>) {
    let d = w.today;
    let wk = monday(d);
    let u = used(w);
    w.scouting.own.film_week = Some((wk, u + hours));
    let t = today(w);
    let fr = &mut w.scouting.film;
    fr.today = Some((d, t + hours));
    if let Some(p) = pos {
        if fr.pos_hours.len() <= p.index() {
            fr.pos_hours.resize(p.index() + 1, 0.0);
        }
        fr.pos_hours[p.index()] += hours;
    }
    let y = d.year();
    match fr.years.iter_mut().find(|x| x.0 == y) {
        Some(x) => x.1 += hours,
        None => fr.years.push((y, hours)),
    }
}

/// Sets how hard you work from this week on.
pub fn set_work(w: &mut World, k: &str) -> Result<String, String> {
    readers::you(w).ok_or("Start a career first.")?;
    let wk = Work::from_key(k).ok_or("No such work level.")?;
    w.scouting.film.work = wk;
    Ok(format!("{}: about {:.0} hours a week. {:.0} film hours this week.", wk.words(), wk.hours(), budget(w)))
}

/// Sets your work ethic in your profile (SK62): once a year, and it moves
/// with how you really work after that.
pub fn set_ethic(w: &mut World, v: f32) -> Result<String, String> {
    readers::you(w).ok_or("Start a career first.")?;
    w.scouting.film.ethic = Some(v.clamp(1.0, 20.0));
    Ok(format!("Your work ethic: {:.0} of 20.", v.clamp(1.0, 20.0)))
}

/// How the staff mirror you (SK63), -1 to 1: the last eight weeks' hours
/// against a steady week. A locked-in head coach lifts his staff's film
/// work; a cruising one lets them coast.
pub fn mirror(w: &World) -> f64 {
    let v: Vec<f32> = w.scouting.film.weeks.iter().rev().take(8).map(|x| x.work.hours()).collect();
    if v.is_empty() {
        return 0.0;
    }
    let m = v.iter().sum::<f32>() / v.len() as f32;
    ((m - Work::Steady.hours()) / 30.0).clamp(-1.0, 1.0) as f64
}

/// Days your staff's film work takes, after mirroring you: a day sooner
/// under a locked-in head coach, a day later under a cruising one.
pub fn staff_days(w: &World, days: i32) -> i32 {
    let m = mirror(w);
    (days
        - if m >= 0.3 {
            1
        } else if m <= -0.3 {
            -1
        } else {
            0
        })
    .max(1)
}

/// The week turns over: last week logged, wear from hours past what your
/// ethic carries, your ethic moving with how you really work (SK62), a
/// worn-out coach sent home, the word around the building (SK63).
pub(crate) fn week(w: &mut World, d: Date) {
    let Some(_) = readers::you(w) else { return };
    let last = d.add(-7);
    let film = w.scouting.own.film_week.filter(|x| x.0 == last).map_or(0.0, |x| x.1);
    let work = w.scouting.film.work;
    let e = ethic(w);
    let cap = 60.0 + 2.0 * (e as f32 - 10.0);
    let hours = work.hours();
    let b = budget(w);
    let fr = &mut w.scouting.film;
    fr.weeks.push(Week { monday: last, work, film, budget: b });
    if fr.weeks.len() > 156 {
        fr.weeks.remove(0);
    }
    fr.wear = if hours > cap {
        (fr.wear + (hours - cap) / 150.0).min(1.0)
    } else {
        (fr.wear - (cap - hours) / 100.0 - 0.03).max(0.0)
    };
    // Your ethic moves with how you really work: about a point a year of
    // locked-in weeks, less for cruising.
    let drift = 0.02 * (hours - Work::Steady.hours()) / 15.0;
    fr.ethic = Some((e as f32 + drift).clamp(1.0, 20.0));
    let wear = fr.wear;
    // Worn out: exhaustion sends you home for a few days; a long run of it
    // costs your family's patience and some of your drive (burnout).
    if wear >= 0.6 {
        let u = crate::world::season::key_uniform(w.options.seed as u64 ^ 0xF17E, d.0 as u64);
        if u < ((wear - 0.6) * 0.5) as f64 {
            w.scouting.film.rest_until = Some(d.add(4));
            w.scouting.film.work = Work::Steady;
            crate::world::folk::post_mine(
                w,
                d,
                crate::calendar::Loudness::Medium,
                false,
                "film: worn out",
                "You were sent home with exhaustion after weeks of long nights in the film room: four days' rest, no film.",
            );
        } else if wear >= 0.85 {
            crate::world::folk::post_mine(
                w,
                d,
                crate::calendar::Loudness::Small,
                false,
                "film: family",
                "Your family hasn't seen you at dinner in weeks.",
            );
        }
    }
    let worn =
        w.scouting.film.weeks.iter().rev().take(6).filter(|x| x.work == Work::Junkie).count() >= 6 && wear >= 0.95;
    if worn {
        let e = ethic(w) as f32;
        w.scouting.film.ethic = Some((e - 1.0).max(1.0));
        w.scouting.film.work = Work::Steady;
        w.scouting.film.wear = 0.6;
        crate::world::folk::post_mine(
            w,
            d,
            crate::calendar::Loudness::Medium,
            false,
            "film: burnout",
            "Burned out: you dropped back to a steady week, and some of the old drive went with it.",
        );
    }
    // The word gets around (SK63): ten weeks of 80 hours or more in the
    // last twelve, and your boss hears "first one in the building".
    let long = w.scouting.film.weeks.iter().rev().take(12).filter(|x| x.work.hours() >= 80.0).count();
    let since = w.scouting.film.word.map_or(9999, |x| x.days_to(d));
    if long >= 10 && since >= 180 {
        w.scouting.film.word = Some(d);
        if let Some(t) = crate::world::folk::Team::seat(w) {
            let b = crate::world::folk::boss::boss(w, t);
            w.folk.mine.nudge(b.key(), 0.02);
        }
        crate::world::folk::post_mine(
            w,
            d,
            crate::calendar::Loudness::Small,
            false,
            "film: word",
            "Word around the building: you're the first one in and the last one out. Your boss has heard it.",
        );
    }
}
