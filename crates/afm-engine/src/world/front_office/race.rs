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
//!
//! What the log keeps is your staff's read as it stood that week, never his
//! true interest: each school's number carries a lasting lean and a week's
//! own, narrower the more your staff has been in touch with him ([`read`]).

use crate::calendar::{Date, Weekday};
use crate::world::{Stage, World, key_normal};

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

/// One week's line: (program, your staff's read of his interest 0-100),
/// highest read first.
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

/// Who reads a recruit's race, and how close to him.
#[derive(Clone, Copy, Debug)]
pub struct Reader {
    /// Your program (a college seat), if any.
    pub mine: Option<usize>,
    /// A high school coach reading his own player.
    pub own_player: bool,
    /// Calls, visits and camps your staff has had with him.
    pub contacts: usize,
}

/// Your staff's read of his interest in school `j` on day `d` (`v`, the
/// truth): a steady lean of its own for each school and a week's own, the
/// two narrower the more your staff has been in touch with him. A staff
/// reads its own standing better than a rival's; a high school coach hears
/// it from his own player. Proposed, not confirmed: a lasting lean of 2.5
/// points (your school), 4.5 (a rival) or 2 (a coach's own player), a
/// week's own of 3.5, 5 or 1.5, each 8% narrower a contact down to 40%.
pub fn read(r: Reader, id: u32, j: usize, d: Date, v: u8) -> u8 {
    let (lasting, weekly) = if r.own_player {
        (2.0, 1.5)
    } else if r.mine == Some(j) {
        (2.5, 3.5)
    } else {
        (4.5, 5.0)
    };
    let narrow = (1.0 - 0.08 * r.contacts as f64).max(0.4);
    let week = (d.0 / 7) as u64;
    let x = lasting * key_normal(0x5EAD ^ j as u64, id as u64).clamp(-2.0, 2.0)
        + weekly * key_normal(0x5EAE ^ (j as u64) << 20 ^ week, id as u64).clamp(-2.5, 2.5);
    (v as f64 + narrow * x).round().clamp(0.0, 100.0) as u8
}

/// The reader of recruit `i` for your seat today.
pub fn reader(w: &World, i: usize) -> Reader {
    let p = &w.people[i];
    let mine = super::program(w).filter(|&j| w.front_office.college.program == Some(j));
    let own_player = mine.is_none() && super::school::your_school(w).is_some_and(|h| p.hs == h);
    let contacts = mine.map_or(0, |j| {
        let id = p.player.id;
        let desk = &w.front_office.college;
        desk.talks.iter().filter(|t| t.id == id).count()
            + p.cal.visits.iter().filter(|v| v.0 == j && v.1 <= w.today).count()
            + usize::from(desk.visits.iter().any(|v| v.id == id && v.state == super::college::VisitState::Done))
    });
    Reader { mine, own_player, contacts }
}

/// The recruits your seat follows: a college's board (still in high
/// school, prep school or junior college), or a high school's players with
/// a college after them.
fn followed(w: &World) -> Vec<usize> {
    if your_program(w) {
        return board_recruits(w, |_| true, MOST);
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

/// Whether the seat is a college staff with its own desk.
fn your_program(w: &World) -> bool {
    super::program(w).is_some_and(|j| w.front_office.college.program == Some(j))
}

/// Your board's recruits (high school, prep school or junior college) that
/// pass `keep`, at most `most`, each found by id (people are kept in id
/// order: no pass over the world).
fn board_recruits(w: &World, keep: impl Fn(u32) -> bool, most: usize) -> Vec<usize> {
    w.front_office
        .college
        .board
        .iter()
        .filter(|t| keep(t.id))
        .filter_map(|t| w.people.binary_search_by_key(&t.id, |p| p.player.id).ok())
        .filter(|&i| {
            let p = &w.people[i];
            matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) || super::college::is_juco_recruit(p)
        })
        .take(most)
        .collect()
}

/// The week's line on Mondays (and the first day a recruit is followed:
/// a college's new board recruit; a high school coach's players wait for
/// the Monday).
pub(crate) fn day(w: &mut World, d: Date) {
    if w.career.is_none() {
        return;
    }
    let monday = d.weekday() == Weekday::Mon;
    let kids = if monday {
        followed(w)
    } else if your_program(w) {
        let log = &w.front_office.race.kids;
        board_recruits(w, |id| !log.iter().any(|k| k.id == id), MOST.saturating_sub(log.len()))
    } else {
        Vec::new()
    };
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
        let r = reader(w, i);
        let mut schools: Vec<(u16, u8)> = race
            .iter()
            .take(SCHOOLS)
            .map(|&(j, a)| (j.min(u16::MAX as usize) as u16, read(r, id, j, d, interest(base, a))))
            .collect();
        schools.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
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

    #[test]
    fn a_staff_read_moves_week_to_week_and_narrows_with_contacts() {
        let far = Reader { mine: Some(1), own_player: false, contacts: 0 };
        let near = Reader { contacts: 10, ..far };
        let monday = Date::ymd(2027, 1, 4);
        let (mut off_far, mut off_near, mut moved) = (0.0, 0.0, 0);
        for id in 0..400u32 {
            let a = read(far, id, 7, monday, 50) as i32;
            let b = read(far, id, 7, monday.add(7), 50) as i32;
            moved += usize::from(a != b);
            off_far += (a - 50).abs() as f64;
            off_near += (read(near, id, 7, monday, 50) as i32 - 50).abs() as f64;
        }
        // A rival's line is not the truth plus a constant: its weeks move.
        assert!(moved > 300, "{moved} of 400 moved");
        assert!(off_near < 0.6 * off_far, "contacts narrow the read: {off_near} against {off_far}");
        // Your own standing reads closer than a rival's.
        let own: f64 = (0..400u32).map(|id| (read(far, id, 1, monday, 50) as i32 - 50).abs() as f64).sum();
        assert!(own < off_far);
    }
}
