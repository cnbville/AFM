//! Colleges that start or restart football after 2026 (SK68), at the real
//! rate: the National Football Foundation counted 89 colleges adding
//! football in 2008-22 and 62 in the ten years to 2026, about six a year,
//! most of them in the NAIA and Division III. A school says so, hires its
//! coach, has a build year of practices and no games (its first class, its
//! walk-ons, junior college and portal finds), then plays. A Division I
//! school starts at FCS (UTSA, Georgia State, Charlotte, Kennesaw State);
//! a school that dropped football can bring it back (UAB in 2017).

use crate::calendar::{Event, Kind, Level, Loudness};
use crate::rng::Rng;
use crate::world::World;
use crate::world::data::{AcademicTier, CollegeRow, Division};

/// New programs a year (NFF, real).
pub const PER_YEAR: f64 = 6.0;

/// The level a new program starts at (proposed, not confirmed: the NFF's
/// counts lead with the NAIA and Division III; about one a year starts in
/// Division I).
pub const LEVELS: [(Division, f64); 5] =
    [(Division::NAIA, 0.32), (Division::D3, 0.30), (Division::D2, 0.18), (Division::FCS, 0.12), (Division::JUCO, 0.08)];

/// Seasons from the school's word to its build year and to its first
/// games (UTSA hired Larry Coker in 2009 and played in 2011; Charlotte
/// practiced in 2012 and played in 2013).
pub const TO_BUILD: i32 = 1;
pub const TO_FIRST: i32 = 2;

/// Small colleges in growing towns that may start football once the real
/// list runs out (proposed, not confirmed): about 130, so a world keeps
/// adding programs at the real rate for some forty years. Names follow
/// the commonest small-college patterns, in real towns of the states
/// that grow.
pub fn generated(seed: u32) -> Vec<CollegeRow> {
    let mut rng = Rng::new(seed ^ 0x6E_C011);
    let growth: Vec<(usize, f64)> = (0..51).map(|s| (s, super::school::young_growth(s).max(0.0) + 0.001)).collect();
    let kinds: [(Division, f64); 4] =
        [(Division::NAIA, 0.35), (Division::D3, 0.30), (Division::JUCO, 0.25), (Division::D2, 0.10)];
    let mut out: Vec<CollegeRow> = Vec::new();
    let mut tries = 0;
    while out.len() < 130 && tries < 4000 {
        tries += 1;
        let s = rng.weighted(&growth);
        let code = crate::world::geo::STATE_CENTERS[s].0;
        let towns = crate::world::high_school::state_towns(code);
        let big: Vec<&crate::world::high_school::TownRow> = towns.iter().filter(|t| t.population >= 15_000).collect();
        if big.is_empty() {
            continue;
        }
        let town = big[rng.int(0, big.len() as i32 - 1) as usize];
        let division = rng.weighted(&kinds);
        let name = match (division, rng.int(0, 3)) {
            (Division::JUCO, 0 | 1) => format!("{} Community College", town.town),
            (Division::JUCO, _) => format!("{} College", town.county.trim_end_matches(" County")),
            (_, 0) => format!("{} Christian University", town.town),
            (_, 1) => format!("University of {}", town.town),
            (_, 2) => format!("{} College", town.town),
            _ => format!("{} Wesleyan", town.town),
        };
        if out.iter().any(|r| r.name == name) || town.county.is_empty() && division == Division::JUCO {
            continue;
        }
        out.push(CollegeRow {
            name,
            division,
            conference: "Independent".to_string(),
            city: town.town.clone(),
            state: code.to_string(),
            enrollment: (1200.0 * rng.log_normal(0.5)).clamp(400.0, 9000.0) as u32,
            national_titles: 0,
            last_5yr_win_pct: None,
            stadium_capacity: None,
            academic_tier: AcademicTier::Other,
        });
    }
    out
}

/// A program that has never played or has stopped, free to start: dark
/// this season and next, not already starting.
fn free(w: &World, p: usize) -> bool {
    let prog = &w.programs[p];
    let y = w.year;
    !prog.plays(y) && !prog.plays(y + 1) && !prog.plays(y + TO_FIRST) && w.new_teams.program(p).is_none()
}

/// Where a program could start: a Division I school (or a dropped Division
/// I program) at FCS, the rest at their own level.
fn fits(prog: &crate::world::program::Program, level: Division) -> bool {
    match level {
        Division::FCS => matches!(prog.division, Division::FBS | Division::FCS),
        d => prog.division == d,
    }
}

/// Programs that start at each level this year, as a Poisson draw on the
/// world's own stream.
pub(crate) fn year(w: &mut World) {
    let year = w.year;
    let mut rng = Rng::new(w.options.seed ^ 0x0C01_1E6E ^ (year as u32).wrapping_mul(2_654_435_761));
    let n = poisson(&mut rng, PER_YEAR);
    for _ in 0..n {
        let mut level = rng.weighted(&LEVELS);
        let mut pool: Vec<usize> =
            (0..w.programs.len()).filter(|&p| free(w, p) && fits(&w.programs[p], level)).collect();
        if pool.is_empty() {
            // The level's list has run out: another level's school starts.
            pool = (0..w.programs.len()).filter(|&p| free(w, p)).collect();
            let Some(&p) = pool.first() else { return };
            level =
                if matches!(w.programs[p].division, Division::FBS) { Division::FCS } else { w.programs[p].division };
            pool.retain(|&q| fits(&w.programs[q], level));
        }
        // A bigger school has the students and the money to add football
        // (proposed, not confirmed: weight by enrollment).
        let weights: Vec<(usize, f64)> = pool.iter().map(|&p| (p, (w.programs[p].enrollment as f64).sqrt())).collect();
        if weights.is_empty() {
            continue;
        }
        let p = rng.weighted(&weights);
        start(w, p, level, year + TO_BUILD, year + TO_FIRST);
    }
}

pub(crate) fn poisson(rng: &mut Rng, mean: f64) -> u32 {
    let l = (-mean).exp();
    let mut k = 0;
    let mut p = 1.0;
    loop {
        p *= rng.range(0.0, 1.0);
        if p <= l || k > 40 {
            return k;
        }
        k += 1;
    }
}

/// A school starts football: it fields a team from its build year (it
/// recruits, signs and practices) and plays from `first`, as an
/// independent at its level until a conference takes it.
pub fn start(w: &mut World, p: usize, level: Division, build: i32, first: i32) {
    let year = w.year;
    let restart = (crate::world::data::HISTORY_FIRST..year).any(|y| w.programs[p].plays(y));
    let prog = &mut w.programs[p];
    match prog.dark.iter_mut().find(|d| d.1 >= build) {
        Some(d) if d.0 < build => d.1 = build - 1,
        Some(d) => *d = (i32::MIN, i32::MIN),
        None => {}
    }
    prog.dark.retain(|d| d.0 != i32::MIN || d.1 != i32::MIN);
    prog.division = level;
    prog.conference = "Independent".to_string();
    prog.base = Some((level, "Independent".to_string()));
    prog.build_year = (build < first).then_some(build);
    // A new program starts with no tradition and a stadium of its level's
    // size (proposed, not confirmed).
    if !restart {
        prog.tradition = 0.0;
    }
    prog.stadium = prog.stadium.min(match level {
        Division::FBS | Division::FCS => 15_000,
        _ => 4_000,
    });
    prog.story.push((year, if restart { "Brought football back".into() } else { "Started football".into() }));
    let name = prog.real_name.clone();
    w.new_teams.programs.push(super::NewProgram { program: p, announced: year, build, first, restart });
    let level_name = crate::world::program::division_era_name(level, first);
    let text = if restart {
        format!(
            "{name} brings football back: a build year in {build}, then its first games in {first}, in {level_name}."
        )
    } else {
        format!(
            "{name} will start football: a coach to hire, a build year of practices in {build} and its first games in {first}, in {level_name}."
        )
    };
    let d = w.today;
    w.calendar.post(Event::new(d, Level::College, Kind::News, Loudness::Big, text).program(p));
}

/// The share of a full squad a build-year team carries (proposed, not
/// confirmed: Charlotte practiced its build year with about 80).
const BUILD_SQUAD: f64 = 0.75;

/// Junior college players a build-year team takes at most.
const JUCO_FINDS: usize = 12;

/// Walk-on tryouts and finds before summer school at each college in its
/// build year: the best seniors nobody signed, nearest first, and
/// junior college players with a season or more behind them. They enroll
/// with the summer's signees.
pub(crate) fn tryouts(w: &mut World) {
    use crate::world::Stage;
    let year = w.year;
    let today = w.today;
    let building: Vec<usize> =
        w.new_teams.programs.iter().filter(|n| n.build == year && n.first > year).map(|n| n.program).collect();
    for p in building {
        let level = w.programs[p].division;
        let state = w.programs[p].state;
        let coming = w.people.iter().filter(|x| x.cal.signed.is_some_and(|s| s.program == p)).count();
        let have = w.programs[p].roster.len() + coming;
        let target = (crate::world::recruiting::level_roster(level) * BUILD_SQUAD) as usize;
        if have >= target {
            continue;
        }
        let mut want = target - have;
        let free = |x: &crate::world::Person| x.cal.signed.is_none() && x.cal.commit.is_none();
        // Junior college finds first: older bodies for a team of freshmen.
        let mut found = 0usize;
        if level != Division::JUCO {
            let mut juco: Vec<(usize, f64)> = (0..w.people.len())
                .filter(|&i| {
                    let x = &w.people[i];
                    x.stage == Stage::College
                        && free(x)
                        && x.college_seasons >= 1
                        && x.college.is_some_and(|c| w.programs[c].division == Division::JUCO)
                })
                .map(|i| (i, w.people[i].player.overall() + if w.people[i].home_state == state { 3.0 } else { 0.0 }))
                .collect();
            juco.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            for &(i, _) in juco.iter().take(JUCO_FINDS.min(want)) {
                sign(w, i, p, today, false);
                found += 1;
            }
            want -= found;
        }
        // Walk-on tryouts: the seniors nobody signed, from home first (by
        // June the last signing stage has sent them home as done).
        let mut seniors: Vec<(usize, f64)> = (0..w.people.len())
            .filter(|&i| {
                let x = &w.people[i];
                let unsigned = x.stage == Stage::HighSchool
                    || (x.stage == Stage::Done && x.path.is_empty() && x.college.is_none() && x.pro.is_none());
                unsigned && x.class_of == year && free(x)
            })
            .map(|i| (i, w.people[i].player.overall() + if w.people[i].home_state == state { 6.0 } else { 0.0 }))
            .collect();
        seniors.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let walk = seniors.len().min(want);
        for &(i, _) in seniors.iter().take(walk) {
            sign(w, i, p, today, true);
        }
        let name = w.programs[p].real_name.clone();
        let text = format!(
            "{name} holds walk-on tryouts for its build year: {walk} high school seniors and {found} junior college players join the {have} already on the way."
        );
        w.calendar.post(Event::new(today, Level::College, Kind::News, Loudness::Small, text).program(p));
    }
}

fn sign(w: &mut World, i: usize, p: usize, date: crate::calendar::Date, walk_on: bool) {
    let name = w.programs[p].real_name.clone();
    let x = &mut w.people[i];
    x.cal.signed = Some(crate::world::Signed { program: p, walk_on, date, early_enrollee: false });
    x.note(if walk_on {
        format!("Walked on at {name} in its build year, after its tryouts.")
    } else {
        format!("Signed with {name} out of junior college for its build year.")
    });
}
