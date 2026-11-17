//! Support staff as full departments (`docs/design/support-departments.md`,
//! agreed by Cain 2026-11-17, "agree on all"): each support head runs a
//! department of stored people (assistant strength coaches and interns,
//! dietitians, sports scientists, assistant trainers, team physicians, a
//! behavioral health clinician and a pain management specialist, academic
//! advisers and tutors, player development staff, NFL analysts), sized by
//! level, money and era, hired and lost on the support market.
//!
//! A player gets his department's work as a blend of the head's skill
//! (60%) and how much of his people's time reaches him (40%, question 1).
//! Each person covers a number of players (question 2); starters come
//! first, then the rotation, then everyone else, so a thin staff
//! shortchanges the backups. Every read is taken around the level's
//! typical department, so a typical department gives exactly what the
//! single head gave before (question 3): only the spread between rich and
//! poor staffs grows.
//!
//! What is new on top: the dietitian moves weight plans faster (never a
//! ceiling, question 4), the sports scientist cuts soft-tissue injuries
//! (up to 8%, capped with the strength guard at 20%, question 5), the
//! clinician brings a man back from a hard event sooner (up to 30%,
//! question 6), college analysts (the junior posts, C36) do film study's
//! budget part, and the strength staff runs the report-day conditioning
//! test. High schools have a full-time athletic trainer, a part-time one or
//! none, at the CATCH ON study's shares.
//!
//! Sizes are Claude's unless the spec or Cain set them (proposed, not
//! confirmed): the size ranges inside the spec's table, the eras, each
//! job's rating offset and pay share, what an uncovered player gets
//! (`LOW`, `FLOOR`), the reach of jobs Q2 doesn't name, and the size of
//! each new read.

use super::World;
use super::data::DIVISION_COUNT;
use super::people::{self, Dept, Staffer, Support, Who, Yardstick};
use super::season::key_uniform;
use crate::rng::{Rng, clamp};

/// A job in a support department below its head.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Job {
    StrengthAssistant,
    StrengthIntern,
    Dietitian,
    SportsScientist,
    /// An assistant athletic trainer (graduate assistant trainers too).
    Trainer,
    /// A team physician, contracted (or the university's).
    Physician,
    /// A behavioral health clinician.
    Clinician,
    PainSpecialist,
    /// An academic adviser or learning specialist.
    Adviser,
    Tutor,
    /// Player development (life skills, career, money).
    Development,
    /// An NFL analytics staffer (college analysts are the junior posts).
    Analyst,
}

impl Job {
    pub const ALL: [Job; 12] = [
        Job::StrengthAssistant,
        Job::StrengthIntern,
        Job::Dietitian,
        Job::SportsScientist,
        Job::Trainer,
        Job::Physician,
        Job::Clinician,
        Job::PainSpecialist,
        Job::Adviser,
        Job::Tutor,
        Job::Development,
        Job::Analyst,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Job::StrengthAssistant => "Assistant strength coach",
            Job::StrengthIntern => "Strength intern",
            Job::Dietitian => "Sports dietitian",
            Job::SportsScientist => "Sports scientist",
            Job::Trainer => "Assistant athletic trainer",
            Job::Physician => "Team physician",
            Job::Clinician => "Behavioral health clinician",
            Job::PainSpecialist => "Pain management specialist",
            Job::Adviser => "Academic adviser",
            Job::Tutor => "Tutor",
            Job::Development => "Player development",
            Job::Analyst => "Analyst",
        }
    }

    /// The head he works under.
    pub fn dept(self) -> Dept {
        match self {
            Job::StrengthAssistant | Job::StrengthIntern | Job::Dietitian | Job::SportsScientist => Dept::Strength,
            Job::Trainer | Job::Physician | Job::Clinician | Job::PainSpecialist => Dept::Medical,
            Job::Adviser | Job::Tutor => Dept::Academic,
            Job::Development => Dept::Engagement,
            Job::Analyst => Dept::Analytics,
        }
    }

    /// Players one of them covers (question 2: a trainer about 30, a
    /// strength assistant about 25, a dietitian and a sports scientist
    /// about 60, an academic adviser about 40, a tutor 10). Development
    /// 50 and a clinician 90 are proposed, not confirmed. Physicians and the
    /// pain specialist see the men sent to them: they lift the department's
    /// quality, not its reach. An NFL analyst is one of the four men a
    /// club's film and numbers need.
    pub fn reach(self) -> f64 {
        match self {
            Job::StrengthAssistant | Job::StrengthIntern => 25.0,
            Job::Dietitian | Job::SportsScientist => 60.0,
            Job::Trainer => 30.0,
            Job::Adviser => 40.0,
            Job::Tutor => 10.0,
            Job::Development => 50.0,
            Job::Clinician => 90.0,
            Job::Physician | Job::PainSpecialist => 0.0,
            Job::Analyst => 1.0,
        }
    }

    /// His rating's centre against a fresh head at the same post
    /// (proposed, not confirmed).
    fn offset(self) -> f64 {
        match self {
            Job::StrengthAssistant | Job::Trainer => -1.5,
            Job::StrengthIntern => -4.0,
            Job::Tutor => -3.0,
            Job::Physician => 1.0,
            Job::PainSpecialist => 0.0,
            Job::Dietitian | Job::SportsScientist | Job::Clinician => -0.5,
            Job::Adviser | Job::Development | Job::Analyst => -1.0,
        }
    }

    /// His pay as a share of his head's going rate (proposed, not
    /// confirmed).
    fn pay_share(self) -> f64 {
        match self {
            Job::StrengthAssistant | Job::Dietitian => 0.3,
            Job::StrengthIntern => 0.08,
            Job::SportsScientist | Job::Analyst => 0.35,
            Job::Trainer => 0.4,
            Job::Physician | Job::PainSpecialist => 0.6,
            Job::Clinician => 0.55,
            Job::Adviser | Job::Development => 0.5,
            Job::Tutor => 0.12,
        }
    }

    /// The job whose best man is first up when his head's post opens (an
    /// assistant trainer for the head trainer's job).
    pub fn ladder(self) -> bool {
        matches!(self, Job::StrengthAssistant | Job::Trainer | Job::Adviser | Job::Development | Job::Analyst)
    }

    /// The share of women in the job (proposed, not confirmed; from the
    /// professions' own counts: dietitians about 80% women, athletic
    /// trainers about half, few in a football weight room).
    fn women(self) -> f64 {
        match self {
            Job::StrengthAssistant | Job::StrengthIntern => 0.08,
            Job::Dietitian => 0.75,
            Job::SportsScientist => 0.25,
            Job::Trainer => 0.35,
            Job::Physician | Job::PainSpecialist => 0.2,
            Job::Clinician => 0.6,
            Job::Adviser | Job::Tutor => 0.6,
            Job::Development => 0.35,
            Job::Analyst => 0.12,
        }
    }

    /// People in 2026 by pay level (Power 4, Group of 5, FCS, Division II
    /// with the NAIA and junior colleges, Division III, NFL), low to high
    /// with the money: the spec's table (`support-departments.md`, part 2),
    /// split by job where it gives a department's total (proposed, not
    /// confirmed).
    fn range(self, lvl: usize) -> (f64, f64) {
        let r: [(f64, f64); 6] = match self {
            Job::StrengthAssistant => [(2.0, 3.0), (2.0, 3.0), (1.0, 2.0), (0.0, 0.0), (0.0, 0.0), (2.0, 4.0)],
            Job::StrengthIntern => [(1.0, 2.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0)],
            Job::Dietitian => [(2.0, 3.0), (1.0, 1.0), (0.0, 1.0), (0.0, 0.0), (0.0, 0.0), (1.0, 2.0)],
            Job::SportsScientist => [(1.0, 3.0), (0.0, 1.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (1.0, 2.0)],
            Job::Trainer => [(3.0, 5.0), (2.0, 2.0), (1.0, 2.0), (0.0, 1.0), (0.0, 0.0), (3.0, 3.0)],
            Job::Physician => [(2.0, 2.0), (1.0, 1.0), (1.0, 1.0), (0.0, 0.0), (0.0, 0.0), (2.0, 2.0)],
            Job::Clinician => [(0.0, 1.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (1.0, 1.0)],
            Job::PainSpecialist => [(0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (1.0, 1.0)],
            Job::Adviser => [(1.0, 3.0), (0.0, 1.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0)],
            Job::Tutor => [(4.0, 10.0), (2.0, 4.0), (1.0, 2.0), (0.0, 1.0), (0.0, 0.0), (0.0, 0.0)],
            Job::Development => [(1.0, 3.0), (0.0, 1.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (1.0, 1.0)],
            Job::Analyst => [(0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (1.0, 6.0)],
        };
        r[lvl.min(5)]
    }

    /// The share of 2026's numbers a year had (the spec's eras: dietitians
    /// rare before the NCAA's unlimited meals in 2014 and the NFL's hires
    /// after, sports science from about 2015, the NFL's clinicians and
    /// pain specialists from 2019, analysts swelling through the 2010s;
    /// proposed, not confirmed).
    fn era(self, year: i32, lvl: usize) -> f64 {
        let ramp = |a: i32, b: i32| clamp((year - a) as f64 / (b - a) as f64, 0.0, 1.0);
        let nfl = lvl == 5;
        match self {
            Job::Dietitian if nfl => 0.15 + 0.85 * ramp(2008, 2020),
            Job::Dietitian if lvl == 0 => 0.2 + 0.8 * ramp(2011, 2019),
            Job::Dietitian => ramp(2014, 2022),
            Job::SportsScientist => ramp(2012, 2020),
            Job::Clinician if nfl => {
                if year >= 2019 {
                    1.0
                } else {
                    0.25
                }
            }
            Job::Clinician => ramp(2014, 2020),
            Job::PainSpecialist => {
                if year >= 2019 {
                    1.0
                } else {
                    0.0
                }
            }
            Job::Analyst => 0.2 + 0.8 * ramp(2003, 2023),
            Job::Development => 0.3 + 0.7 * ramp(2005, 2016),
            Job::Adviser | Job::Tutor => 0.6 + 0.4 * ramp(1990, 2008),
            Job::StrengthAssistant | Job::StrengthIntern | Job::Trainer | Job::Physician => {
                0.75 + 0.25 * ramp(1990, 2010)
            }
        }
    }

    /// A Division II-and-below or a shared FCS dietitian's share of his
    /// time on football.
    fn share(self, lvl: usize) -> f32 {
        match (self, lvl) {
            (_, 3) | (Job::Dietitian, 2) => 0.5,
            _ => 1.0,
        }
    }
}

/// Someone in a support department below its head.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Aide {
    pub job: Job,
    pub name: String,
    #[serde(default)]
    pub she: bool,
    /// Hidden 1-20.
    pub rating: u8,
    pub born: i32,
    pub since: i32,
    /// Dollars a year.
    pub pay: f32,
    /// His share of his time on football: 1, or a half where he works for
    /// the whole department (Division II and below, a shared dietitian, a
    /// part-time high school trainer).
    #[serde(default = "one")]
    pub share: f32,
}

fn one() -> f32 {
    1.0
}

impl Aide {
    /// The head he becomes when he steps up.
    pub fn as_head(&self, year: i32) -> Staffer {
        Staffer {
            name: self.name.clone(),
            she: self.she,
            rating: self.rating,
            since: year,
            born: self.born,
            pay: self.pay,
            coach: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Sizes
// ---------------------------------------------------------------------------

/// A team's pay level: 0-4 a college's (`people::pay_level`), 5 an NFL
/// club.
pub fn level(w: &World, who: Who) -> usize {
    match who {
        Who::Prog(k) => people::pay_level(w, k),
        Who::Club(_) => 5,
    }
}

/// Where a team's money stands within its level, 0-1 (the median 0.5):
/// a college's football staff line for the head coach's departments and
/// its athletic budget for the trainers and academics, an NFL owner's
/// spending level.
fn money(w: &World, who: Who, d: Dept, ys: &Yardstick) -> f64 {
    clamp(0.5 + 0.55 * people::money_ratio(w, who, d, ys).ln(), 0.0, 1.0)
}

/// What a head's share of his time on football is: one shared strength
/// coach and one or two trainers for a Division II department, one
/// trainer for every Division III sport (proposed, not confirmed).
fn head_share(d: Dept, lvl: usize) -> f64 {
    match (d, lvl) {
        (_, 3) => 0.5,
        (Dept::Strength, 4) => 0.4,
        (Dept::Medical, 4) => 0.3,
        (_, 4) => 0.5,
        _ => 1.0,
    }
}

fn team_key(who: Who) -> u64 {
    match who {
        Who::Prog(k) => k as u64,
        Who::Club(t) => 0x8000 ^ t as u64,
    }
}

/// How many of a job a team keeps this year: the level's range by the
/// team's money, times the era, a keyed half-person either way so teams
/// with the same money differ (held from year to year).
pub(crate) fn target(w: &World, who: Who, job: Job, ys: &Yardstick) -> usize {
    let lvl = level(w, who);
    let (lo, hi) = job.range(lvl);
    if hi <= 0.0 {
        return 0;
    }
    let m = money(w, who, job.dept(), ys);
    let x = (lo + (hi - lo) * m) * job.era(w.year, lvl);
    let u = key_uniform(0xDE97_0000 ^ team_key(who), job as u64);
    (x + 0.9 * (u - 0.5)).round().clamp(0.0, hi) as usize
}

fn hire(w: &World, who: Who, job: Job, rng: &mut Rng, ys: &Yardstick) -> Aide {
    let year = w.year;
    let lvl = level(w, who);
    let budget = people::hire_level(w, who);
    let fresh = people::fresh_mean(budget, people::era_of(year));
    let head = job.dept().post(people::support_of(w, who)).rating;
    // Heads propose their own people: a good one draws better (topic 10).
    let lean = if head > 0 { 0.25 * (head as f64 - fresh) } else { 0.0 };
    let eth = super::pro::scouts::staff_ethnicity(rng, year);
    let born = year - rng.int(23, 55);
    let she = rng.chance(job.women());
    let name = super::folk::cast::person_name(eth, born, she, rng);
    let rating = clamp(rng.normal(fresh + job.offset() + lean, 2.8), 1.0, 20.0).round() as u8;
    let d = job.dept();
    let rate = if d == Dept::Academic && lvl == 5 { 0.0 } else { d.going_rate(lvl) };
    let pay = rate
        * people::money_ratio(w, who, d, ys).clamp(0.35, 10.0).powf(0.8)
        * super::finance::data::coach_index(year)
        * job.pay_share();
    Aide { job, name, she, rating, born, since: year, pay: pay as f32, share: job.share(lvl) }
}

/// Puts a team's people in their order: by job, the best first.
fn tidy(people: &mut [Aide]) {
    people.sort_by(|a, b| (a.job as u8).cmp(&(b.job as u8)).then(b.rating.cmp(&a.rating)).then(a.since.cmp(&b.since)));
}

/// Brings one team's jobs to this year's sizes: the weakest go where
/// there are too many, fresh people come where there are too few.
fn size_team(w: &mut World, who: Who, rng: &mut Rng, ys: &Yardstick) -> (u16, u16) {
    let mut people = std::mem::take(&mut people::support_mut(w, who).people);
    let (mut hired, mut gone) = (0u16, 0u16);
    for job in Job::ALL {
        let want = target(w, who, job, ys);
        let have = people.iter().filter(|a| a.job == job).count();
        if have > want {
            // The weakest of the job go.
            let mut idx: Vec<usize> = (0..people.len()).filter(|&k| people[k].job == job).collect();
            idx.sort_by_key(|&k| (people[k].rating, std::cmp::Reverse(people[k].since)));
            let drop: std::collections::HashSet<usize> = idx.into_iter().take(have - want).collect();
            let mut k = 0;
            people.retain(|_| {
                let keep = !drop.contains(&k);
                k += 1;
                keep
            });
            gone += (have - want) as u16;
        }
        for _ in have..want {
            people.push(hire(w, who, job, rng, ys));
            hired += 1;
        }
    }
    tidy(&mut people);
    people::support_mut(w, who).people = people;
    (hired, gone)
}

/// The departments' year at the support market (NFL camp, after the heads
/// are filled): people retire (from 60, all by 72) or leave (one in ten a
/// year: another field, a move the game doesn't keep), pay keeps up, each
/// job comes to the year's size; then the high schools' trainers and the
/// year's typical departments (proposed, not confirmed: the rates).
pub(crate) fn yearly(w: &mut World, rng: &mut Rng, ys: &Yardstick, grow: f64) {
    let year = w.year;
    let (mut hired, mut gone) = (0u16, 0u16);
    for who in people::teams(w) {
        let s = people::support_mut(w, who);
        let before = s.people.len();
        s.people.retain(|a| {
            let age = year - a.born;
            let retire = age >= 72 || (age >= 60 && rng.chance(0.06 + 0.04 * (age - 60) as f64));
            !(retire || rng.chance(0.1))
        });
        gone += (before - s.people.len()) as u16;
        for a in s.people.iter_mut() {
            a.pay *= grow as f32;
        }
        let (h, g) = size_team(w, who, rng, ys);
        hired += h;
        gone += g;
    }
    let row = w.support_market.row(year);
    row.aides_hired = row.aides_hired.saturating_add(hired);
    row.aides_gone = row.aides_gone.saturating_add(gone);
    ops_staff(w);
    hs_trainers(w, rng);
    w.support_market.typical = typical_table(w);
}

/// Fills any team's empty departments (an older file, a new team) and the
/// year's table, without the year's turnover.
pub(crate) fn ensure(w: &mut World) {
    if !w.support_market.typical.is_empty() && people::teams(w).iter().all(|&who| staffed(w, who)) {
        return;
    }
    let mut rng = Rng::new(w.options.seed ^ 0xDE97_A11D ^ (w.year as u32).wrapping_mul(7919));
    let ys = people::yardstick(w);
    for who in people::teams(w) {
        if !staffed(w, who) {
            size_team(w, who, &mut rng, &ys);
        }
    }
    ops_staff(w);
    hs_trainers(w, &mut rng);
    w.support_market.typical = typical_table(w);
}

/// A team has been staffed once (its heads are hired and the year's
/// market has run on it, or it has people).
fn staffed(w: &World, who: Who) -> bool {
    let s = people::support_of(w, who);
    s.strength.rating == 0 || !s.people.is_empty() || s.staffed
}

/// Each college's junior analysts and quality control (C36), counted and
/// read for film study (their scheme), kept on the program for the year.
fn ops_staff(w: &mut World) {
    use super::coach::{Employer, Post};
    let mut tally: Vec<(u32, f64)> = vec![(0, 0.0); w.programs.len()];
    for c in super::staff::live(w) {
        if c.retired {
            continue;
        }
        let Some(j) = c.junior_now() else { continue };
        if !matches!(j.post, Post::Analyst | Post::QualityControl) {
            continue;
        }
        if let Employer::College(p) = j.employer
            && p < tally.len()
        {
            tally[p].0 += 1;
            tally[p].1 += c.ability().scheme;
        }
    }
    for (p, (n, sum)) in tally.into_iter().enumerate() {
        let s = &mut w.programs[p].support;
        s.ops = [n.min(255) as u8, if n > 0 { clamp(sum / n as f64, 1.0, 20.0).round() as u8 } else { 0 }];
        s.staffed = true;
    }
    for c in w.clubs.iter_mut() {
        c.support.staffed = true;
    }
}

/// The best man on a team's ladder for a head's post: (his place, rating).
pub(crate) fn best_ladder(w: &World, who: Who, d: Dept) -> Option<(usize, u8)> {
    people::support_of(w, who)
        .people
        .iter()
        .enumerate()
        .filter(|(_, a)| a.job.ladder() && a.job.dept() == d)
        .max_by_key(|(k, a)| (a.rating, usize::MAX - k))
        .map(|(k, a)| (k, a.rating))
}

/// A level's rank, best first: the NFL, Power 4, Group of 5, FCS, Division
/// II and below, Division III.
fn rank(lvl: usize) -> i32 {
    if lvl == 5 { 0 } else { lvl as i32 + 1 }
}

/// The best assistants elsewhere a head's post moves up, best first: his
/// own level, one above or one below (an assistant trainer at a Power 4
/// school becomes a Group of 5 head trainer). (rating, team, place).
pub(crate) fn ladder_elsewhere(w: &World, who: Who, d: Dept, all: &[Who], over: u8) -> Vec<(u8, Who, usize)> {
    let r = rank(level(w, who));
    let mut v: Vec<(u8, Who, usize)> = Vec::new();
    for &o in all {
        if o == who || (rank(level(w, o)) - r).abs() > 1 {
            continue;
        }
        for (k, a) in people::support_of(w, o).people.iter().enumerate() {
            if a.job.ladder() && a.job.dept() == d && a.rating > over {
                v.push((a.rating, o, k));
            }
        }
    }
    v.sort_by(|a, b| b.0.cmp(&a.0).then(team_key(a.1).cmp(&team_key(b.1))).then(a.2.cmp(&b.2)));
    v.truncate(3);
    v
}

/// Takes a man out of a team's department (he stepped up or followed his
/// head) and hires his replacement.
pub(crate) fn take(w: &mut World, who: Who, k: usize, rng: &mut Rng, ys: &Yardstick) -> Aide {
    let a = people::support_mut(w, who).people.remove(k);
    let job = a.job;
    let fresh = hire(w, who, job, rng, ys);
    let s = people::support_mut(w, who);
    s.people.push(fresh);
    tidy(&mut s.people);
    a
}

/// A new strength head brings one or two of his assistants (the spec,
/// part 6): each of his old team's strength assistants comes half the
/// time, at most two, and the weakest of the new team's go to make room.
pub(crate) fn bring(w: &mut World, from: Who, to: Who, rng: &mut Rng, ys: &Yardstick) -> u16 {
    let mut moved = 0;
    let mut k = 0;
    while k < people::support_of(w, from).people.len() && moved < 2 {
        let a = &people::support_of(w, from).people[k];
        if a.job == Job::StrengthAssistant && rng.chance(0.5) {
            let mut man = take(w, from, k, rng, ys);
            man.since = w.year;
            let s = people::support_mut(w, to);
            s.people.push(man);
            moved += 1;
            // Room for him: the weakest assistant goes.
            let n = s.people.iter().filter(|a| a.job == Job::StrengthAssistant).count();
            let year = w.year;
            if n > target(w, to, Job::StrengthAssistant, ys).max(moved as usize) {
                let s = people::support_mut(w, to);
                if let Some((j, _)) = s
                    .people
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| a.job == Job::StrengthAssistant && a.since < year)
                    .min_by_key(|(_, a)| a.rating)
                {
                    s.people.remove(j);
                }
            }
            tidy(&mut people::support_mut(w, to).people);
            // The list moved under us: look again from the start.
            k = 0;
            continue;
        }
        k += 1;
    }
    moved
}

// ---------------------------------------------------------------------------
// High school trainers
// ---------------------------------------------------------------------------

/// A high school's athletic trainer: full-time, part-time or none (the
/// CATCH ON study, 2015-2018: public 37%, 32%, 31%; private 27%, 28%,
/// 45%), the richer school more likely to have one; drawn once by the
/// school's key, filled again when he retires or leaves (proposed, not
/// confirmed: the money's pull).
fn hs_trainers(w: &mut World, rng: &mut Rng) {
    let year = w.year;
    for h in 0..w.high_schools.len() {
        let s = &w.high_schools[h];
        let u = key_uniform(0x4A75_7EA1, s.id as u64);
        let score = clamp(u - 0.3 * (s.resources - 0.5), 0.0, 1.0);
        let (full, part) = if s.private { (0.27, 0.55) } else { (0.37, 0.69) };
        let share = if score < full {
            1.0
        } else if score < part {
            0.5
        } else {
            0.0
        };
        let keep = w.high_schools[h].trainer.as_ref().is_some_and(|a| {
            let age = year - a.born;
            a.share == share && !(age >= 70 || (age >= 60 && rng.chance(0.08)) || rng.chance(0.08))
        });
        if share == 0.0 {
            w.high_schools[h].trainer = None;
        } else if !keep {
            let res = w.high_schools[h].resources;
            let eth = super::pro::scouts::staff_ethnicity(rng, year);
            let born = year - rng.int(23, 58);
            let she = rng.chance(0.5);
            let name = super::folk::cast::person_name(eth, born, she, rng);
            let rating = clamp(rng.normal(8.5 + 3.0 * res + 1.5 * people::era_of(year), 2.8), 1.0, 20.0).round() as u8;
            let pay = (if share >= 1.0 { 52_000.0 } else { 18_000.0 }) * super::finance::data::coach_index(year);
            w.high_schools[h].trainer =
                Some(Aide { job: Job::Trainer, name, she, rating, born, since: year, pay: pay as f32, share });
        }
    }
}

// ---------------------------------------------------------------------------
// What the departments give
// ---------------------------------------------------------------------------

/// What a player gets with no one's time: group work in the weight room,
/// a trainer he has to find (proposed, not confirmed).
pub const LOW: f64 = 4.0;
/// The share of a full program an uncovered player still gets (proposed,
/// not confirmed).
pub const FLOOR: f64 = 0.35;

/// A department's work as the reads take it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    /// The head strength coach, his assistants and interns.
    Strength,
    /// The head athletic trainer, his trainers and the physicians.
    Medical,
    /// The academic head, advisers and tutors.
    Academic,
    /// Player engagement and development.
    Engagement,
    /// The head of analytics and his analysts (college: the junior
    /// analysts and quality control).
    Analytics,
    /// The dietitians.
    Diet,
    /// The sports scientists.
    Science,
    /// The behavioral health clinician.
    Clinic,
}

pub const AREAS: usize = 8;

impl Area {
    pub const ALL: [Area; AREAS] = [
        Area::Strength,
        Area::Medical,
        Area::Academic,
        Area::Engagement,
        Area::Analytics,
        Area::Diet,
        Area::Science,
        Area::Clinic,
    ];

    /// The head blended in at 60% (question 1); the new reads are the
    /// people's own.
    fn head(self) -> Option<Dept> {
        match self {
            Area::Strength => Some(Dept::Strength),
            Area::Medical => Some(Dept::Medical),
            Area::Academic => Some(Dept::Academic),
            Area::Engagement => Some(Dept::Engagement),
            Area::Analytics => Some(Dept::Analytics),
            Area::Diet | Area::Science | Area::Clinic => None,
        }
    }

    fn jobs(self) -> &'static [Job] {
        match self {
            Area::Strength => &[Job::StrengthAssistant, Job::StrengthIntern],
            Area::Medical => &[Job::Trainer, Job::Physician, Job::PainSpecialist],
            Area::Academic => &[Job::Adviser, Job::Tutor],
            Area::Engagement => &[Job::Development],
            Area::Analytics => &[Job::Analyst],
            Area::Diet => &[Job::Dietitian],
            Area::Science => &[Job::SportsScientist],
            Area::Clinic => &[Job::Clinician],
        }
    }

    /// The head's own reach, as his job's people.
    fn head_reach(self) -> f64 {
        match self {
            Area::Strength => 25.0,
            Area::Medical => 30.0,
            Area::Academic => 40.0,
            Area::Engagement => 50.0,
            Area::Analytics => 1.0,
            _ => 0.0,
        }
    }
}

/// A team's department as the reads see it: its head (0 none), the mean
/// rating of the hands-on people, the players their time covers, and the
/// players there are.
#[derive(Clone, Copy, Debug)]
struct Shape {
    head: f64,
    q: f64,
    time: f64,
    n: f64,
}

fn shape(s: &Support, area: Area, lvl: usize, roster: usize, college: bool) -> Shape {
    let head = area.head().map_or(0.0, |d| {
        let r = d.post(s).rating;
        if r == 0 { 10.0 } else { r as f64 }
    });
    let (mut sum, mut cnt, mut time) = (0.0, 0.0, 0.0);
    if let Some(d) = area.head() {
        let hs = head_share(d, lvl);
        sum += head;
        cnt += 1.0;
        time += area.head_reach() * hs;
    }
    for a in s.people.iter().filter(|a| area.jobs().contains(&a.job)) {
        sum += a.rating as f64;
        cnt += 1.0;
        time += a.job.reach() * a.share as f64;
    }
    // College film study's analysts are the junior posts (C36).
    if area == Area::Analytics && college && s.ops[0] > 0 {
        sum += s.ops[1] as f64 * s.ops[0] as f64;
        cnt += s.ops[0] as f64;
        time += s.ops[0] as f64;
    }
    let n = match area {
        // A club's film and numbers need about four men, a college's eight
        // (proposed, not confirmed).
        Area::Analytics => {
            if college {
                8.0
            } else {
                4.0
            }
        }
        _ => roster.max(1) as f64,
    };
    Shape { head, q: if cnt > 0.0 { sum / cnt } else { LOW }, time, n }
}

impl Shape {
    /// The people's part for a player with this share of a full program.
    fn people(&self, s: f64) -> f64 {
        s * self.q + (1.0 - s) * LOW
    }
    fn mean_share(&self) -> f64 {
        FLOOR + (1.0 - FLOOR) * (self.time / self.n).min(1.0)
    }
    /// The department's number for a share: the head at 60%, the people
    /// at 40% (question 1); the people alone where there's no head.
    fn d(&self, area: Area, s: f64) -> f64 {
        if area.head().is_some() { 0.6 * self.head + 0.4 * self.people(s) } else { self.people(s) }
    }
    /// The shares of the three priority tiers (starters, the rotation,
    /// everyone else), given how many players each holds.
    fn tiers(&self, counts: [usize; 3]) -> [f64; 3] {
        let mut above = 0.0;
        let mut out = [FLOOR; 3];
        for (k, &c) in counts.iter().enumerate() {
            let c = c as f64;
            let covered = if c > 0.0 { clamp((self.time - above) / c, 0.0, 1.0) } else { 1.0 };
            out[k] = FLOOR + (1.0 - FLOOR) * covered;
            above += c;
        }
        out
    }
}

/// Where a team's reads are taken around: its division, the NFL or the
/// high schools.
fn key_of(w: &World, who: Who) -> usize {
    match who {
        Who::Prog(k) => w.programs[k].division as usize,
        Who::Club(_) => DIVISION_COUNT,
    }
}

const HS_KEY: usize = DIVISION_COUNT + 1;

fn roster(w: &World, who: Who) -> usize {
    match who {
        Who::Prog(k) => w.programs[k].roster.len(),
        Who::Club(t) => w.clubs[t].roster.len(),
    }
}

fn shape_of(w: &World, who: Who, area: Area) -> Shape {
    shape(people::support_of(w, who), area, level(w, who), roster(w, who), matches!(who, Who::Prog(_)))
}

/// Each level's typical department: (the mean head, the mean department
/// number) by area, over the teams with players, each team at its mean
/// share; the high schools' trainers last.
fn typical_table(w: &World) -> Vec<[(f32, f32); AREAS]> {
    let mut sum = vec![[(0.0f64, 0.0f64, 0.0f64); AREAS]; HS_KEY + 1];
    for who in people::teams(w) {
        if roster(w, who) == 0 {
            continue;
        }
        let key = key_of(w, who);
        for (a, area) in Area::ALL.iter().enumerate() {
            let sh = shape_of(w, who, *area);
            let e = &mut sum[key][a];
            e.0 += sh.head;
            e.1 += sh.d(*area, sh.mean_share());
            e.2 += 1.0;
        }
    }
    // The high schools (`hs_medical`): the trainer's work.
    let (mut hs, mut n) = (0.0, 0.0);
    for s in w.high_schools.iter().filter(|s| !s.roster.is_empty()) {
        hs += hs_work(s);
        n += 1.0;
    }
    sum[HS_KEY][Area::Medical as usize] = (0.0, hs, n);
    sum.into_iter()
        .map(|row| row.map(|(h, d, n)| if n > 0.0 { ((h / n) as f32, (d / n) as f32) } else { (10.0, 0.0) }))
        .collect()
}

/// The typical (head, department) for a team's level and area, if the
/// market has run.
fn typical(w: &World, key: usize, area: Area) -> Option<(f64, f64)> {
    let t = w.support_market.typical.get(key)?[area as usize];
    (t.1 > 0.0).then_some((t.0 as f64, t.1 as f64))
}

/// A team's department read for each priority tier, on the scale its
/// reads take.
///
/// For a department with a head (strength, medical, academic, engagement,
/// analytics) it is a head's rating, 1-20: the level's typical head plus
/// how far this department's work stands from the typical one, so a
/// typical department reads as today's typical head and every built read
/// keeps its formula. For the new reads (diet, science, clinic) it is the
/// points over the level's typical department (0 typical).
///
/// Before the market has run (an older file's first days, a new world's
/// first months) it reads the head alone, as before.
pub fn tiers(w: &World, who: Who, area: Area, counts: [usize; 3]) -> [f64; 3] {
    let sh = shape_of(w, who, area);
    let shares = sh.tiers(counts);
    match typical(w, key_of(w, who), area) {
        Some((h, d)) => shares.map(|s| {
            let x = sh.d(area, s) - d;
            if area.head().is_some() { clamp(h + x, 1.0, 20.0) } else { x }
        }),
        None => [if area.head().is_some() { sh.head } else { 0.0 }; 3],
    }
}

/// The read for the team's first tier: the men who play (starters, the
/// injured; a game's players, an injury's care).
pub fn first(w: &World, who: Who, area: Area) -> f64 {
    let n = roster(w, who);
    tiers(w, who, area, [n.min(30), n.saturating_sub(30), 0])[0]
}

/// The read for the team as a whole (its mean share).
pub fn team(w: &World, who: Who, area: Area) -> f64 {
    let sh = shape_of(w, who, area);
    let s = sh.mean_share();
    match typical(w, key_of(w, who), area) {
        Some((h, d)) => {
            let x = sh.d(area, s) - d;
            if area.head().is_some() { clamp(h + x, 1.0, 20.0) } else { x }
        }
        None => {
            if area.head().is_some() {
                sh.head
            } else {
                0.0
            }
        }
    }
}

/// The level's typical head for a team's area (the reads' yardstick), or
/// the head himself before the market has run.
pub fn typical_head(w: &World, who: Who, area: Area) -> Option<f64> {
    typical(w, key_of(w, who), area).map(|t| t.0)
}

/// A priority tier from his role: starters first, then the rotation,
/// then everyone else.
pub fn tier_of(role: crate::player::PlayingTime) -> usize {
    use crate::player::PlayingTime as P;
    match role {
        P::Starter => 0,
        P::Rotation => 1,
        _ => 2,
    }
}

/// A high school's trainer's work: a full-time one his rating, a
/// part-time one half his and half no one's, none no one's.
fn hs_work(s: &super::high_school::HighSchool) -> f64 {
    match &s.trainer {
        Some(a) => a.share as f64 * a.rating as f64 + (1.0 - a.share as f64) * LOW,
        None => LOW,
    }
}

/// A high school player's medical read, -1 to 1: the high schools' built
/// -0.3 (most schools have a trainer part time or none), moved by his
/// school's trainer against the typical school's, so the high schools'
/// average stays where it was.
pub fn hs_medical(w: &World, hs: usize) -> f64 {
    let Some(s) = w.high_schools.get(hs) else { return -0.3 };
    match typical(w, HS_KEY, Area::Medical) {
        Some((_, d)) => clamp(-0.3 + (hs_work(s) - d) / 10.0, -1.0, 1.0),
        None => -0.3,
    }
}

// ---------------------------------------------------------------------------
// The new reads
// ---------------------------------------------------------------------------

/// The dietitians' factor on a weight plan's pace (question 4): 2% a
/// point of work over the level's typical department, 20% at most either
/// way (proposed, not confirmed). Weight only, never a ceiling.
pub fn diet_pace(points: f64) -> f64 {
    clamp(1.0 + 0.02 * points, 0.8, 1.2)
}

/// The sports scientists' guard on soft-tissue injuries (question 5): 0.8%
/// fewer a point of work over the level's typical department, 8% at most
/// either way, read through the sports science lab (a great one in a level
/// 1 lab reads about half as much; part 4). `lab` is the lab's level, 0-5.
pub fn science_guard(points: f64, lab: f64) -> f64 {
    let lab = clamp(0.4 + 0.12 * lab, 0.4, 1.0);
    clamp(1.0 - 0.008 * points * lab, 0.92, 1.08)
}

/// The strength guard and the sports scientists' together, capped at 20%
/// either way (question 5).
pub fn soft_guard(strength: f64, science: f64) -> f64 {
    clamp(strength * science, 0.8, 1.2)
}

/// How much faster a hard event's weight fades (question 6): 3% a point
/// of the clinician's work over the level's typical, up to 30% faster,
/// 15% slower at most with none where the typical team has one (proposed,
/// not confirmed).
pub fn clinic_speed(points: f64) -> f64 {
    clamp(1.0 + 0.03 * points, 0.85, 1.3)
}

/// The team a person's departments are: his college's or his NFL club's.
pub fn team_of(p: &super::Person) -> Option<Who> {
    use super::Stage;
    match p.stage {
        Stage::College => p.college.map(Who::Prog),
        Stage::Pro => p.pro.as_ref().and_then(|c| c.team).map(Who::Club),
        _ => None,
    }
}

/// A person's clinician's fade speed for hard events (1 with none to read).
pub fn clinic_for(w: &World, i: usize) -> f64 {
    let Some(p) = w.people.get(i) else { return 1.0 };
    match team_of(p) {
        Some(who) if valid(w, who) => clinic_speed(team(w, who, Area::Clinic)),
        _ => 1.0,
    }
}

/// A program index or club still in range (a team the world dropped).
pub fn valid(w: &World, who: Who) -> bool {
    match who {
        Who::Prog(k) => k < w.programs.len(),
        Who::Club(t) => t < w.clubs.len(),
    }
}

/// A college's film study budget part (the spec, part 5): its analysts'
/// work against the division's typical, 1.5% a point, 15% at most either
/// way (proposed, not confirmed).
pub fn film_budget(w: &World, prog: usize) -> f64 {
    let who = Who::Prog(prog);
    match typical_head(w, who, Area::Analytics) {
        Some(h) => clamp(1.0 + 0.015 * (team(w, who, Area::Analytics) - h), 0.85, 1.15),
        None => 1.0,
    }
}

/// The strength staff's factor on the report-day conditioning test's
/// failures (the spec, part 5): 3% fewer a point over the level's typical
/// department, 30% at most either way (proposed, not confirmed).
pub fn conditioning(w: &World, who: Who) -> f64 {
    match typical_head(w, who, Area::Strength) {
        Some(h) => clamp(1.0 - 0.03 * (team(w, who, Area::Strength) - h), 0.7, 1.3),
        None => 1.0,
    }
}

/// The soft-tissue guard a team's game day or practice week reads: the
/// strength department's guard (minor bug 303) times the sports
/// scientists', capped together. `lab` is the sports science lab's level.
pub fn game_guard(w: &World, who: Who, lab: f64) -> f64 {
    let head = first(w, who, Area::Strength);
    let typ = typical_head(w, who, Area::Strength).unwrap_or(head);
    let strength = crate::game::sim::strength_guard(head, typ);
    soft_guard(strength, science_guard(first(w, who, Area::Science), lab))
}

/// Your seat's team for the departments, if it has them.
pub fn seat_team(w: &World) -> Option<Who> {
    let s = w.seat?;
    s.nfl_team.map(Who::Club).or(s.program.map(Who::Prog))
}

/// Everyone in a team's departments, for the staff screen and the tools.
pub fn people_of(w: &World, who: Who) -> &[Aide] {
    &people::support_of(w, who).people
}

/// Each team's clinician fade speed at once, for a pass over many people
/// (`folk::talks::reapply`).
pub struct ClinicTable {
    progs: Vec<f32>,
    clubs: Vec<f32>,
}

impl ClinicTable {
    pub fn new(w: &World) -> ClinicTable {
        let speed = |who: Who| clinic_speed(team(w, who, Area::Clinic)) as f32;
        ClinicTable {
            progs: (0..w.programs.len()).map(|k| speed(Who::Prog(k))).collect(),
            clubs: (0..w.clubs.len()).map(|t| speed(Who::Club(t))).collect(),
        }
    }
    /// The speed for a person by id (1 with no team).
    pub fn of(&self, w: &World, id: u32) -> f32 {
        let Some(i) = super::folk::person(w, id) else { return 1.0 };
        match team_of(&w.people[i]) {
            Some(Who::Prog(k)) => self.progs.get(k).copied().unwrap_or(1.0),
            Some(Who::Club(t)) => self.clubs.get(t).copied().unwrap_or(1.0),
            None => 1.0,
        }
    }
}
