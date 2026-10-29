//! Dated events with their why, and the growth step's why.

use serde::Serialize;

use super::WorldLog;
use crate::calendar::Date;
use crate::development::{GrowthWhy, SeasonContext, SeasonReport};
use crate::player::PlayingTime;
use crate::positions::Position;
use crate::world::World;
use crate::world::data::Division;

/// A dated event and its why. Each names its people by id (players by
/// `Player::id`, coaches, programs, schools and NFL teams by index).
#[derive(Clone, Debug, Serialize)]
pub enum Why {
    /// A coach (or the AI staff) moved a player to a new position: his fit
    /// at both, the margin that decided it and what the staff saw.
    PositionMove { id: u32, from: Position, to: Position, from_fit: f64, to_fit: f64, by: &'static str },
    /// A player entered the transfer portal: the lean that built up through
    /// his season (each event and its size) and the reason it gave.
    PortalEntry {
        id: u32,
        program: Option<usize>,
        why: &'static str,
        lean: f64,
        events: Vec<(Date, &'static str, f64)>,
        role: Option<PlayingTime>,
        held: bool,
    },
    /// A transfer's first season at his new school: his role before and now.
    TransferSeason { id: u32, program: usize, division: Division, before: PlayingTime, now: PlayingTime },
    /// A recruit commits or signs: the schools that pulled him most, each
    /// with its appeal and the seven reasons (`recruiting::reason_parts`:
    /// close to home, playing time, NIL money, development, winning and the
    /// name, academics, his ties and feel), each as (appeal points, the
    /// school's score 0-1). Read as the day's appeal, which leaves out the
    /// position room and the year's relationship the signing utility adds.
    Choice { id: u32, stage: &'static str, program: usize, walk_on: bool, offers: usize, top: Vec<Pull> },
    /// An athletic director's yearly review of a college head coach: the
    /// read, its parts, the bar, the chance of firing and what came of it.
    Review {
        coach: usize,
        program: usize,
        tenure: i32,
        last: f64,
        before: f64,
        win_pct_two: f64,
        win_pct_last: f64,
        title: bool,
        roster_direction: f64,
        class_direction: f64,
        buyout: f64,
        money: f64,
        expired: bool,
        contending: bool,
        read: f64,
        bar: f64,
        fire_chance: f64,
        outcome: &'static str,
    },
    /// A coaching search: the opening, how well the employer reads teaching
    /// (the read's spread), the candidates it called in order with their
    /// scores, who took it and why.
    Search {
        employer: crate::world::coach::Employer,
        role: crate::world::coach::Role,
        prestige: f64,
        read_sd: f64,
        called: Vec<(usize, f64)>,
        hired: Option<usize>,
        why: String,
    },
    /// A combine invitee: whether he skipped the workouts, and the numbers
    /// his body and ratings make (`body::Drills::expected`; each club's look
    /// adds its own timing on top).
    Combine { id: u32, program: Option<usize>, skipped: bool, expected: crate::body::Combine },
    /// A free agent's market on a day he had offers: his ideal, every
    /// club's offer and its score, the best, and what he did.
    Market {
        id: u32,
        ideal: crate::world::pro::market::Ideal,
        offers: Vec<(crate::world::pro::market::Offer, f64)>,
        best: usize,
        score: f64,
        outcome: &'static str,
    },
    /// A pro's move: signed, re-signed, extended, released or waived,
    /// traded or claimed, drafted, signed undrafted, retired; with the why
    /// the world gave and the contract it left him on.
    Transaction {
        id: u32,
        team: Option<usize>,
        kind: &'static str,
        why: String,
        contract: Option<crate::world::pro::money::Contract>,
    },
}

/// One school's pull on a recruit (see `Why::Choice`).
#[derive(Clone, Debug, Serialize)]
pub struct Pull {
    pub program: usize,
    pub appeal: f64,
    pub reasons: [(f64, f64); 7],
}

/// The schools that pulled recruit `i` most when he chose `j` (`j` always
/// among them): his offers by appeal, the top three.
pub(crate) fn choice(w: &World, i: usize, j: usize, stage: &'static str, walk_on: bool) -> Why {
    use crate::world::recruiting::{appeal, reason_parts};
    let p = &w.people[i];
    let mut schools: Vec<usize> = p.cal.offers.clone();
    if !schools.contains(&j) {
        schools.push(j);
    }
    let mut pulls: Vec<(usize, f64)> = schools.iter().map(|&k| (k, appeal(w, i, k, None))).collect();
    pulls.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut top: Vec<usize> = pulls.iter().take(3).map(|x| x.0).collect();
    if !top.contains(&j) {
        top.pop();
        top.insert(0, j);
    }
    Why::Choice {
        id: p.player.id,
        stage,
        program: j,
        walk_on,
        offers: p.cal.offers.len(),
        top: top
            .into_iter()
            .map(|k| Pull {
                program: k,
                appeal: pulls.iter().find(|x| x.0 == k).map_or(0.0, |x| x.1),
                reasons: reason_parts(w, i, k, None),
            })
            .collect(),
    }
}

/// A pro's move (see `Why::Transaction`), read after it happened.
pub(crate) fn transaction(w: &World, i: usize, kind: &'static str, why: &str) -> Why {
    let p = &w.people[i];
    let c = p.pro.as_deref();
    Why::Transaction {
        id: p.player.id,
        team: c.and_then(|c| c.team),
        kind,
        why: why.to_string(),
        contract: c.and_then(|c| c.contract.clone()),
    }
}

#[derive(Serialize)]
struct WhyRow<'a> {
    season: i32,
    date: Date,
    why: &'a Why,
}

impl World {
    /// Logs a dated why (nothing without a log).
    pub(crate) fn log_why(&self, why: Why) {
        if let Some(log) = self.wlog.as_deref().filter(|_| self.recording) {
            put(log, (self.year, self.today), why);
        }
    }
}

impl World {
    /// Logs what a store drops to stay small, as it drops it.
    pub(crate) fn log_dropped<'a, T: Serialize + 'a>(&self, store: &'static str, items: impl Iterator<Item = &'a T>) {
        let Some(log) = self.wlog.as_deref().filter(|_| self.recording) else { return };
        for item in items {
            log.put("dropped", &Dropped { season: self.year, date: self.today, store, item });
        }
    }
}

#[derive(Serialize)]
struct Dropped<'a, T: Serialize> {
    season: i32,
    date: Date,
    store: &'static str,
    item: &'a T,
}

/// Logs a dated why where the world is borrowed in parts.
pub(crate) fn put(log: &WorldLog, (season, date): (i32, Date), why: Why) {
    log.put("whys", &WhyRow { season, date, why: &why });
}

/// One development step's why (`development::develop_season` and the world
/// around it).
#[derive(Serialize)]
struct GrowthRow<'a> {
    season: i32,
    date: Date,
    id: u32,
    /// What the world gave the step: level, coaching, role, trouble, the
    /// first-year dip, situation, payday, snaps, the staff's style and
    /// scheme, morale, strength staff and medical support, the plan's focus.
    ctx: &'a SeasonContext,
    /// Whose teaching made `coaching`: (head coach, coordinator, position
    /// coach) at college, the head coach in high school, none for a pro.
    coaches: [Option<usize>; 3],
    report: &'a SeasonReport,
    drew: &'a GrowthWhy,
}

/// Logs one development step (called right after `develop_season` on the
/// same thread, from the loops that run on all cores).
pub(crate) fn growth(
    log: &WorldLog,
    (season, date): (i32, Date),
    id: u32,
    ctx: &SeasonContext,
    coaches: [Option<usize>; 3],
    report: &SeasonReport,
) {
    let Some(drew) = crate::development::take_why() else { return };
    log.put("growth", &GrowthRow { season, date, id, ctx, coaches, report, drew: &drew });
}

/// Which pass is writing the new alumni.
#[derive(Clone, Copy)]
pub(super) enum Alumni {
    Book,
    YearEnd,
}

#[derive(Serialize)]
struct AlumnusRow<'a> {
    season: i32,
    date: Date,
    index: usize,
    when: &'static str,
    alumnus: &'a crate::world::Alumnus,
}

/// The alumni added since the last pass (drafted players, undrafted pros,
/// blue-chips who quit), as they were written.
pub(super) fn alumni_since(w: &World, log: &WorldLog, when: Alumni) {
    let mut m = log.marks.lock().unwrap();
    let when = match when {
        Alumni::Book => "book",
        Alumni::YearEnd => "year end",
    };
    for (k, a) in w.alumni.iter().enumerate().skip(m.alumni) {
        log.put("whys", &AlumnusRow { season: w.year, date: w.today, index: k, when, alumnus: a });
    }
    m.alumni = w.alumni.len();
}

#[derive(Serialize)]
struct Recruiting<'a, T: Serialize> {
    season: i32,
    date: Date,
    list: &'static str,
    row: &'a T,
}

/// The recruiting log's rows added this year (portal moves with both
/// programs, where the five-stars went, transfers' first roles).
pub(super) fn recruiting_since(w: &World, log: &WorldLog) {
    let mut m = log.marks.lock().unwrap();
    let r = &w.recruiting_log;
    let (season, date) = (w.year, w.today);
    for row in r.portal.iter().skip(m.portal) {
        log.put("whys", &Recruiting { season, date, list: "portal", row });
    }
    for row in r.five_stars.iter().skip(m.five_stars) {
        log.put("whys", &Recruiting { season, date, list: "five stars", row });
    }
    m.portal = r.portal.len();
    m.five_stars = r.five_stars.len();
    m.transfer_seasons = r.transfer_seasons.len();
}

/// A fingerprint of each store, as hex.
pub(super) fn fingerprints(w: &World) -> Vec<(&'static str, String)> {
    use super::fp::of;
    let hex = |v: u64| format!("{v:016x}");
    vec![
        ("rng", hex(of(&w.rng))),
        ("people", hex(of(&w.people))),
        ("alumni", hex(of(&w.alumni))),
        ("coaches", hex(of(&w.coaches))),
        ("programs", hex(of(&w.programs))),
        ("high schools", hex(of(&w.high_schools))),
        ("clubs", hex(of(&w.clubs))),
        ("league", hex(of(&w.nfl))),
        ("news", hex(of(&w.news))),
        ("stats", hex(of(&w.stats))),
        ("nfl book", hex(of(&w.nfl_book))),
        ("market", hex(of(&w.market))),
        ("scouting", hex(of(&w.scouting))),
        ("folk", hex(of(&w.folk))),
        ("finance", hex(of(&w.finance))),
        ("voters", hex(of(&w.voters))),
        ("calendar", hex(of(&w.calendar.events))),
    ]
}
