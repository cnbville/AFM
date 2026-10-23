//! College eligibility and the long roads through college (final build,
//! lane 5: A3 and S11).
//!
//! The rules by era, all real:
//! - Through the 2025 season every level used four seasons of competition
//!   inside a five-year clock that starts at first full-time collegiate
//!   enrollment. A season he never played in, a redshirt, doesn't count.
//! - **Medical hardship** (NCAA Bylaw 12.8.4, through 2025-26): a season
//!   ended by injury in its first half, with no more than three games or 30%
//!   of the team's games played, doesn't count. Two seasons lost to things
//!   beyond his control (injuries, a redshirt year he was hurt in) can earn
//!   a **sixth year** on the clock (Bylaw 12.8.1.7).
//! - **The 2020 COVID year**: the NCAA (Division I Council, Aug 21, 2020;
//!   Division II and the NAIA and NJCAA alike) gave every fall athlete the
//!   2020 season back and a year on the clock. Seniors who came back in 2021
//!   did not count against the 85 scholarships that one year. An AP review
//!   of 129 FBS rosters found "more than 1,000" scholarship players planning
//!   to use it (Feb 28, 2021), about 8 a team, from seniors who mostly
//!   could have left.
//! - **Five years to play five (S11)**: the Division I Cabinet adopted the
//!   age-based model on June 23-24, 2026 (NCAA.org, "Division I adopts
//!   age-based eligibility model"; ESPN; PBS). From 2026-27 a Division I
//!   player has five seasons inside five years, the clock starting at
//!   full-time enrollment or the academic year after his 19th birthday,
//!   whichever comes first. Redshirt rules, season-of-competition limits
//!   and extension waivers are gone; injuries no longer extend it. Religious
//!   missions, active military duty and maternity still pause the clock.
//!   Players with eligibility left after 2025-26 use whichever model is
//!   better for them (the new one never gives less); a player whose last
//!   season was 2025-26 gets nothing more. Division II, the NAIA and junior
//!   colleges keep their old rules.
//!
//! The long roads (A3), the ones the world lacked:
//! - **Missions**: a Latter-day Saint player may serve a two-year mission,
//!   right after high school (signing first and enrolling on his return,
//!   most of them once the church lowered the age to 18 in October 2012) or
//!   after his first college year. The clock stops while he serves; he comes
//!   back two years older and rusty, and his school holds his place.
//! - **Military service**: a few unsigned seniors enlist, serve a four-year
//!   term, and walk on at 22 (Daniel Rodriguez at Clemson at 24, Nate Boyer
//!   at Texas at 29). Active duty pauses the clock.
//! - **Older walk-ons**: the returned missionaries and veterans who come
//!   back without a scholarship, and walk on.
//! - **A prep school year** (built earlier) no longer costs a year of the
//!   five-year clock, which starts at college enrollment (real through
//!   2025-26; from 2026-27 the age clause can start it, see `clock_start`).
//! - **Moving up from below** (built earlier for FCS to FBS and Division II
//!   to FCS): a Division II standout can now go straight to FBS, and NAIA
//!   and Division III standouts can move up (`recruiting.rs`, the portal).
//! - **Junior college** was built before (`recruiting.rs`).

use super::data::Division;
use super::{Person, Stage};
use crate::player::{DevelopmentEvent, PlayingTime};

/// The first season under the five-year model (2026-27).
pub const FIVE_IN_FIVE_FROM: i32 = 2026;

/// The COVID season: it didn't count, and added a year to the clock.
pub const COVID_SEASON: i32 = 2020;

/// What a two-year mission or a military term is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum AwayKind {
    Mission,
    Military,
}

impl AwayKind {
    pub fn name(self) -> &'static str {
        match self {
            AwayKind::Mission => "a church mission",
            AwayKind::Military => "military service",
        }
    }
}

/// A mission or a military term in progress: what it is, the summer he
/// comes back, and the program holding his place (a signee or a player who
/// left after a college year), if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Away {
    pub kind: AwayKind,
    /// The summer he comes back.
    pub until: i16,
    /// The program holding his place (index into the world's programs).
    pub program: Option<u16>,
}

/// A player's road through college: what moved his clock and why. Every
/// field defaults, so a world saved before the final build loads with none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Roads {
    /// Years his clock was stopped before or during college: a mission, a
    /// military term or a prep school year.
    #[serde(default)]
    pub away_years: u8,
    /// Years added to his clock: the 2020 COVID year, a sixth-year waiver.
    #[serde(default)]
    pub clock_bonus: u8,
    /// He was on a college roster in the fall of 2020.
    #[serde(default)]
    pub covid: bool,
    /// Seasons lost to injury (a medical hardship or a hurt redshirt year),
    /// toward a sixth year.
    #[serde(default)]
    pub injury_seasons: u8,
    /// A sixth year was granted.
    #[serde(default)]
    pub sixth: bool,
    /// A mission or military term in progress.
    #[serde(default)]
    pub away: Option<Away>,
}

/// Whether a season ran under the five-year model at a division.
pub fn five_in_five(season: i32, division: Division) -> bool {
    season >= FIVE_IN_FIVE_FROM && matches!(division, Division::FBS | Division::FCS)
}

/// Seasons of competition a player gets at his division in `season`.
pub fn seasons_allowed(season: i32, division: Division) -> u32 {
    if five_in_five(season, division) { 5 } else { 4 }
}

/// The last season his five-year clock allows: five seasons from the fall
/// after his class, plus the years it stopped and the years added.
pub fn clock_last(p: &Person) -> i32 {
    p.class_of + 4 + p.roads.away_years as i32 + p.roads.clock_bonus as i32
}

/// His year in school in `season` (1 for a first-year player), not
/// counting years away.
pub fn school_year(p: &Person, season: i32) -> i32 {
    season - p.class_of + 1 - p.roads.away_years as i32
}

/// Whether he has no college football left after `season` at `division`:
/// his seasons are used up, or his clock has run out.
pub fn exhausted_after(p: &Person, division: Division, season: i32) -> bool {
    p.college_seasons >= seasons_allowed(season, division) || season >= clock_last(p)
}

/// The division a player's eligibility is read at: his program's, or
/// FBS's for someone between programs (the stricter old rule never applies
/// to the five-year model's players).
pub fn division_of(p: &Person, programs: &[super::program::Program]) -> Division {
    p.college.map_or(Division::FBS, |c| programs[c].division)
}

/// Whether the 2020 COVID year is the only thing keeping him eligible
/// after `season`: without it, his seasons or his clock would be done.
pub fn on_covid_year(p: &Person, division: Division, season: i32) -> bool {
    if !p.roads.covid || exhausted_after(p, division, season) {
        return false;
    }
    // Without the COVID year his 2020 season would have counted (if he
    // played in it) and his clock would end a year sooner.
    p.college_seasons + 1 >= seasons_allowed(season, division) || season + 1 >= clock_last(p)
}

/// The chance a player whose COVID year is all he has left comes back for
/// it: starters most, as the AP's count of 2021's super seniors (about 8 a
/// FBS team) and its team examples (New Mexico 14, Appalachian State 13)
/// show. Claude's pick inside that count (proposed in the final build):
/// starters 60%, rotation 45%, backups 25%, the rest 12%.
pub fn covid_return_chance(p: &Person) -> f64 {
    match p.last_role {
        Some(PlayingTime::Starter) => 0.6,
        Some(PlayingTime::Rotation) => 0.45,
        Some(PlayingTime::Backup) => 0.25,
        _ => 0.12,
    }
}

/// Whether he is a 2020 senior back for his COVID year in 2021, when the
/// NCAA left such players out of the scholarship limits for that year.
pub fn covid_senior(p: &Person, season: i32) -> bool {
    season == COVID_SEASON + 1 && p.roads.covid && (p.college_seasons + 1 >= 4 || COVID_SEASON >= clock_last(p) - 1)
}

/// Whether a season's injury took most of it: a major injury, or half the
/// season's games missed.
pub fn hurt_most_of_season(p: &Person) -> bool {
    p.player.history.last().is_some_and(|h| {
        h.events.has(DevelopmentEvent::MajorInjury)
            || h.games_missed * 2 >= crate::development::season_games(h.level).max(1)
    })
}

/// The medical hardship line: three games or 30% of his team's games,
/// whichever is more (Bylaw 12.8.4.2).
pub fn hardship_games(team_games: u32) -> u32 {
    3.max((0.3 * team_games as f64).floor() as u32)
}

/// The share of a state's people who are Latter-day Saints (church
/// membership against state population, about 2020-24: Utah about 42%,
/// Idaho 19%, Wyoming 9%, Nevada and Arizona 6%, Hawaii and Alaska 5%,
/// Montana, Oregon and Washington 4%; under 2% elsewhere). Pacific
/// Islander families are far more often members (about a third of
/// Tongans and Samoans in the U.S.). Read from the church's own state
/// counts against the Census; rounded.
fn lds_share(state: &str, polynesian: bool) -> f64 {
    let base = match state {
        "UT" => 0.42,
        "ID" => 0.19,
        "WY" => 0.09,
        "NV" | "AZ" => 0.06,
        "HI" | "AK" => 0.05,
        "MT" | "OR" | "WA" => 0.04,
        "NM" | "CO" => 0.025,
        "CA" | "NE" | "ND" | "SD" | "KS" | "OK" | "TX" | "MO" | "VA" | "AR" => 0.015,
        _ => 0.008,
    };
    if polynesian { (base * 2.0_f64).max(0.33) } else { base }
}

/// Whether a person is a Latter-day Saint: a fixed draw against his home
/// state's share, from his id (never stored; the same answer every time).
pub fn latter_day_saint(p: &Person, state_code: &str, seed: u64) -> bool {
    let poly = p.ethnicity == super::ethnicity::Ethnicity::Polynesian;
    super::season::key_uniform(seed ^ 0x1D5_0000, p.player.id as u64) < lds_share(state_code, poly)
}

/// The chance a Latter-day Saint with a college place serves a mission, by
/// the year he would go (Claude's pick, proposed in the final build): 40%
/// before the age dropped to 18 (October 2012), 45% after it, 30% from the
/// NIL era (2021), when leaving for two years began to cost real money.
/// About a third of the church's young men serve; athletes headed to BYU,
/// Utah and Utah State far more often, top recruits elsewhere less.
pub fn mission_chance(year: i32) -> f64 {
    match year {
        ..=2012 => 0.40,
        2013..=2020 => 0.45,
        _ => 0.30,
    }
}

/// Of those who serve, the share who go straight from high school rather
/// than after a first college year: before October 2012 a young man had
/// to be 19, so most played a season first; after it most left at 18.
pub fn mission_from_high_school(year: i32) -> f64 {
    if year <= 2012 { 0.35 } else { 0.75 }
}

/// Whether a stage holds a person away from football for now.
pub fn is_away(p: &Person) -> bool {
    p.stage == Stage::Away
}

/// Of the unsigned graduates good enough to have played somewhere, the
/// share who enlist each year, and of those the share who come back to
/// walk on after a four-year term (Claude's picks, proposed in the final
/// build): about 3% of young men enlist out of high school (the services'
/// own accession counts against graduating classes), and a few in four of
/// the athletes among them try to play again on the GI Bill.
const ENLIST: f64 = 0.03;
const VETERAN_RETURNS: f64 = 0.25;

/// The summer roads (A3), run on the day signees arrive for summer school:
/// missionaries and veterans come home, Latter-day Saint signees and
/// first-year players leave on missions, and a few unsigned graduates
/// enlist. Each road's chance is drawn from the player's id on its own
/// stream, so nothing else in the world moves.
pub(crate) fn summer_roads(w: &mut super::World) {
    use super::season::key_uniform;
    let year = w.year;
    let seed = w.options.seed as u64 ^ ((year as u64) << 24) ^ 0xA3_5A3;
    let today = w.today;
    // Coming home first: a held place becomes a signing, so enrolment takes him in.
    let mut walkers: Vec<usize> = Vec::new();
    for (i, p) in w.people.iter_mut().enumerate() {
        if p.stage != Stage::Away {
            continue;
        }
        let Some(a) = p.roads.away else {
            p.stage = Stage::Done;
            continue;
        };
        if (a.until as i32) > year {
            continue;
        }
        p.roads.away = None;
        w.log.returned += 1;
        let age = p.row_age();
        match a.program.map(|c| c as usize).filter(|&c| w.programs[c].plays(year)) {
            Some(c) => {
                p.cal.signed =
                    Some(super::Signed { program: c, walk_on: p.walk_on, date: today, early_enrollee: false });
                let name = &w.programs[c].real_name;
                p.note(format!("Home from {} at {age}, he joined {name}, which had held his place.", a.kind.name()));
            }
            None => {
                p.stage = Stage::Done;
                walkers.push(i);
            }
        }
    }
    // Home without a place: he walks on at the best program in his home
    // state where he would make the two-deep's back half, as a 21- to
    // 23-year-old freshman.
    for i in walkers {
        let state = w.high_schools[w.people[i].hs].state;
        let ovr = w.people[i].player.overall();
        let mut best: Option<(Division, usize)> = None;
        for (j, prog) in w.programs.iter().enumerate() {
            if prog.state != state || !prog.plays(year) || prog.division == Division::JUCO {
                continue;
            }
            let mut r: Vec<f64> = prog
                .roster
                .iter()
                .filter(|&&k| w.people[k].stage == Stage::College)
                .map(|&k| w.people[k].player.overall())
                .collect();
            if r.is_empty() {
                continue;
            }
            r.sort_by(|a, b| a.total_cmp(b));
            let bar = r[r.len() * 2 / 5];
            if ovr >= bar && best.is_none_or(|(d, _)| (prog.division as usize) < d as usize) {
                best = Some((prog.division, j));
            }
        }
        let p = &mut w.people[i];
        let age = p.row_age();
        match best {
            Some((_, j)) => {
                p.stage = Stage::College;
                p.college = Some(j);
                p.path.push(j);
                p.walk_on = true;
                p.note(format!(
                    "Walked on at {} at {age}, older than every freshman around him.",
                    w.programs[j].real_name
                ));
                w.log.tryouts += 1;
            }
            None => p.note(format!("Back home at {age}, he found no team with a place for him.")),
        }
    }
    // Leaving: missions from signees and first-year players, enlistment
    // from the unsigned graduates near the top of their state.
    let mut top: Vec<Vec<(f64, usize)>> = vec![Vec::new(); w.states.len()];
    for (i, p) in w.people.iter().enumerate() {
        if p.stage == Stage::Done && p.class_of == year && p.path.is_empty() && p.college.is_none() && p.pro.is_none() {
            top[w.high_schools[p.hs].state].push((p.player.overall(), i));
        }
    }
    let mut unsigned: Vec<usize> = Vec::new();
    for list in top.iter_mut() {
        list.sort_by(|a, b| b.0.total_cmp(&a.0));
        unsigned.extend(list.iter().take(40).map(|x| x.1));
    }
    let m = mission_chance(year);
    let h = mission_from_high_school(year);
    for i in 0..w.people.len() {
        let p = &w.people[i];
        let state = w.states[w.high_schools[p.hs].state].code.as_str();
        let u = key_uniform(seed, p.player.id as u64);
        // Top recruits serve less often: the money and the draft clock weigh more.
        let star = if p.stars >= 4 { 0.4 } else { 1.0 };
        let signee = p.stage == Stage::HighSchool && p.class_of == year && p.cal.signed.is_some();
        // A player in the transfer portal is moving schools, not leaving.
        let first_year = p.stage == Stage::College
            && school_year(p, year) == 2
            && p.roads.away_years == 0
            && p.cal.portal.is_none()
            && p.college.is_some_and(|c| w.programs[c].division != Division::JUCO);
        if !(signee || first_year) || !latter_day_saint(p, state, w.options.seed as u64) {
            continue;
        }
        let chance = if signee { m * h } else { m * (1.0 - h) / (1.0 - m * h) } * star;
        if u >= chance {
            continue;
        }
        let program = if signee { p.cal.signed.map(|s| s.program) } else { p.college };
        let p = &mut w.people[i];
        if let Some(s) = p.cal.signed {
            p.walk_on = s.walk_on;
        }
        p.stage = Stage::Away;
        p.cal.signed = None;
        p.college = None;
        p.roads.away_years = p.roads.away_years.saturating_add(2);
        p.roads.away =
            Some(Away { kind: AwayKind::Mission, until: (year + 2) as i16, program: program.map(|c| c as u16) });
        w.log.missions += 1;
        let age = p.row_age();
        let name = program.map_or("", |c| w.programs[c].real_name.as_str());
        p.note(if signee {
            format!("Left at {age} on a two-year church mission before enrolling; {name} held his place.")
        } else {
            format!("Left {name} at {age} after his first year for a two-year church mission; his place is held.")
        });
    }
    for i in unsigned {
        let p = &w.people[i];
        let u = key_uniform(seed ^ 0x5E_4D1, p.player.id as u64);
        let state = w.states[w.high_schools[p.hs].state].code.as_str();
        let lds = latter_day_saint(p, state, w.options.seed as u64);
        let p = &mut w.people[i];
        let age = p.row_age();
        if lds && u < m {
            // A Latter-day Saint nobody signed can still serve and walk on after.
            p.stage = Stage::Away;
            p.roads.away_years = p.roads.away_years.saturating_add(2);
            p.roads.away = Some(Away { kind: AwayKind::Mission, until: (year + 2) as i16, program: None });
            w.log.missions += 1;
            p.note(format!("Left at {age} on a two-year church mission, meaning to walk on when he got home."));
        } else if u < ENLIST * VETERAN_RETURNS {
            p.stage = Stage::Away;
            p.roads.away_years = p.roads.away_years.saturating_add(4);
            p.roads.away = Some(Away { kind: AwayKind::Military, until: (year + 4) as i16, program: None });
            w.log.enlisted += 1;
            p.note(format!("Enlisted at {age} for a four-year term, meaning to play again after it."));
        } else if u < ENLIST {
            p.note(format!("Enlisted in the military at {age}."));
        }
    }
    w.rebuild_rosters();
}
