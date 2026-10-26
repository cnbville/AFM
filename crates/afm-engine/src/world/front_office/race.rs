//! Each recruit's race as your office followed it (update 2, lane 7, board
//! 8): once a week, every school after a recruit you follow and how much it
//! means to him, so his race can be read month by month afterwards.
//!
//! A college staff follows the recruits on its board; a high school coach
//! follows his own players that colleges recruit. A week's line keeps the
//! six schools that mean most to him, each as his interest from 0 to 100,
//! read against where his race stood the first week (so a school's line
//! can rise and fall on one scale). A recruit leaves the log when he leaves
//! the board, or school.

use crate::calendar::{Date, Weekday};
use crate::world::{Stage, World};

/// Schools kept in a week's line.
pub const SCHOOLS: usize = 6;

/// The most recruits followed at once (a board, or a high school's roster).
const MOST: usize = 400;

/// The log: one entry per recruit followed.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RaceLog {
    pub kids: Vec<KidRace>,
}

/// One recruit's weeks.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KidRace {
    pub id: u32,
    /// The appeal his interest is read against: his best school's, the
    /// first week, less a little (so that school starts in the 80s).
    pub base: f32,
    pub weeks: Vec<RaceWeek>,
}

/// One week's line: (program, interest 0-100), most interested first.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RaceWeek {
    pub date: Date,
    pub schools: Vec<(u16, u8)>,
}

/// How far below his best school's appeal, at the start, the scale's middle
/// sits, and how many appeal points one step of the curve spans (proposed,
/// not confirmed: a school 0.08 ahead, "leaning your way", is about 15
/// interest points ahead near the middle).
const BELOW: f64 = 0.10;
const SPREAD: f64 = 0.06;

/// His interest in a school, 0 to 100, from its appeal.
pub fn interest(base: f32, appeal: f64) -> u8 {
    let x = (appeal - base as f64) / SPREAD;
    (100.0 / (1.0 + (-x).exp())).round().clamp(0.0, 100.0) as u8
}

/// The recruits your seat follows: a college's board (still in high
/// school, prep school or junior college), or a high school's players with
/// a college after them.
fn followed(w: &World) -> Vec<usize> {
    if let Some(j) = super::program(w)
        && w.front_office.college.program == Some(j)
    {
        let ids: Vec<u32> = w.front_office.college.board.iter().map(|t| t.id).collect();
        let at = super::indices(w, &ids);
        let mut v: Vec<usize> = ids
            .iter()
            .filter_map(|id| at.get(id).copied())
            .filter(|&i| {
                let p = &w.people[i];
                matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) || super::college::is_juco_recruit(p)
            })
            .collect();
        v.truncate(MOST);
        return v;
    }
    if let Some(h) = super::school::your_school(w) {
        let mut v: Vec<usize> = w
            .people
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                p.stage == Stage::HighSchool
                    && p.hs == h
                    && (!p.cal.offers.is_empty() || !p.cal.early_offers.is_empty() || p.cal.commit.is_some())
            })
            .map(|(i, _)| i)
            .collect();
        v.truncate(MOST);
        return v;
    }
    Vec::new()
}

/// The week's line on Mondays (and the first day a recruit is followed).
pub(crate) fn day(w: &mut World, d: Date) {
    if w.career.is_none() {
        return;
    }
    let monday = d.weekday() == Weekday::Mon;
    let kids = followed(w);
    let mut fresh: Vec<KidRace> = Vec::new();
    let mut lines: Vec<(u32, RaceWeek)> = Vec::new();
    for &i in &kids {
        let id = w.people[i].player.id;
        let have = w.front_office.race.kids.iter().find(|k| k.id == id);
        if have.is_some() && !monday {
            continue;
        }
        let race = super::college::race_at(w, i);
        if race.is_empty() {
            continue;
        }
        let base = match have {
            Some(k) => k.base,
            None => (race[0].1 - BELOW) as f32,
        };
        let schools: Vec<(u16, u8)> =
            race.iter().take(SCHOOLS).map(|&(j, a)| (j.min(u16::MAX as usize) as u16, interest(base, a))).collect();
        let week = RaceWeek { date: d, schools };
        match have {
            Some(_) => lines.push((id, week)),
            None => fresh.push(KidRace { id, base, weeks: vec![week] }),
        }
    }
    let log = &mut w.front_office.race;
    for (id, week) in lines {
        if let Some(k) = log.kids.iter_mut().find(|k| k.id == id) {
            match k.weeks.last_mut() {
                Some(last) if last.date == week.date => *last = week,
                _ => k.weeks.push(week),
            }
        }
    }
    log.kids.extend(fresh);
    // A recruit who left the board or school leaves the log (weekly).
    if monday {
        let keep: std::collections::HashSet<u32> = kids.iter().map(|&i| w.people[i].player.id).collect();
        w.front_office.race.kids.retain(|k| keep.contains(&k.id));
    }
}

/// A recruit's weeks, if your office followed him.
pub fn of(w: &World, id: u32) -> Option<&KidRace> {
    w.front_office.race.kids.iter().find(|k| k.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interest_runs_zero_to_a_hundred_around_the_start() {
        let base = 1.0f32;
        assert_eq!(interest(base, 1.0), 50);
        assert!(interest(base, 1.1) > 80 && interest(base, 1.1) < 90, "his best school starts in the 80s");
        assert!(interest(base, 0.7) < 2);
        assert_eq!(interest(base, 12.0), 100, "a signed school");
        // "Leaning your way" (0.08 ahead) is a clear gap near the middle.
        assert!(interest(base, 1.04) - interest(base, 0.96) >= 15);
    }
}
