//! Full club finances at every level (later extras, topic 1): each NFL
//! club, college athletic department and high school keeps its own books
//! on its real level's lines, budgeted when its money year opens and
//! filled as it goes (games on their days, the rest month by month). The
//! money moves the world: budgets come from the books, attendance is a
//! market, season tickets renew from fan mood and price, NFL games are
//! blacked out when they don't sell out (1973-2014), facility and stadium
//! projects are real projects, clubs relocate on the owners' vote, clubs
//! are valued and sold, conferences realign by media money and the
//! guarantee games pay the visitors. Also here: the NFLPA team report
//! cards (P1) and the betting lines (C8).
//!
//! Books are compact: a year is a fixed array of lines in dollars (f32),
//! kept for every closed year at the NFL and college, and for this year and
//! the last at a high school.

pub mod college;
pub mod data;
pub mod hs;
pub mod lines;
pub mod nfl;

use std::collections::HashMap;

use crate::calendar::{Date, Event, Kind, Level, Loudness};
use crate::world::World;
use crate::world::season::key_uniform;

/// Every book of the world.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Finance {
    /// Set once the books are opened (the first day the world runs, or the
    /// day an older world file loads).
    pub ready: bool,
    pub nfl: Vec<nfl::Club>,
    pub college: Vec<college::Dept>,
    pub hs: Vec<hs::School>,
    /// Facility and stadium projects under way and finished, newest last.
    pub projects: Vec<Project>,
    /// Each conference's media deal (after 2026 the world's own).
    pub deals: Vec<college::Deal>,
    /// Head coaches' records against the spread: coach id, [covers, misses, pushes].
    pub ats: HashMap<usize, [u16; 3]>,
    /// What the betting market has learned of each team from its games
    /// against the line (`lines::Form`), by `lines::form_key`.
    pub form: HashMap<u32, lines::Form>,
    /// The NFLPA's team report cards (P1): one row a club a year.
    pub cards: Vec<nfl::Card>,
    /// The league's whole revenue by year (dollars), for the public numbers.
    pub league: Vec<(i32, f32)>,
    /// Your high school's booster club ledger (seat only).
    pub seat_hs: Option<hs::Seat>,
    /// The months already posted, so a month is never booked twice.
    pub posted_month: Option<(i32, u32)>,
    /// Programs announced this year as moving up a division with their move.
    pub move_up: Vec<usize>,
}

/// Who a project belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Owner {
    Nfl(usize),
    College(usize),
    Hs(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProjectKind {
    /// A new stadium (NFL) or a new or expanded stadium (college).
    Stadium,
    /// A big renovation of the stadium the club already plays in.
    Renovation,
    /// A training facility, football building or indoor practice field.
    Facility,
}

/// One project: what it is, who pays and when it opens.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Project {
    pub owner: Owner,
    pub kind: ProjectKind,
    pub name: String,
    /// Dollars of the day: the whole cost and the public share of it.
    pub cost: f32,
    pub public: f32,
    /// Donors' pledges toward it (college).
    pub pledged: f32,
    pub started: i32,
    /// The season it opens.
    pub opens: i32,
    /// Seats when it opens (stadiums).
    pub capacity: u32,
    /// How much the facilities number rises when it opens.
    pub step: f32,
    pub done: bool,
    /// Asked for by you (people topic 12).
    pub asked: bool,
}

/// One year's lines and the year they belong to.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Book<const N: usize> {
    pub year: i32,
    #[serde(with = "serde_big_array::BigArray")]
    pub l: [f32; N],
}

impl<const N: usize> Default for Book<N> {
    fn default() -> Self {
        Book { year: 0, l: [0.0; N] }
    }
}

impl<const N: usize> Book<N> {
    pub fn new(year: i32) -> Self {
        Book { year, l: [0.0; N] }
    }
    pub fn sum(&self, r: std::ops::Range<usize>) -> f64 {
        self.l[r].iter().map(|&x| x as f64).sum()
    }
}

/// A line's name and whether it is money in (true) or out.
pub struct Line {
    pub name: &'static str,
    pub income: bool,
    /// How it is paid across the money year, by month from its first
    /// month (0 = posted on game days or on its own day only).
    pub spread: Spread,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Spread {
    /// Twelve equal months.
    Even,
    /// Mostly in the season's months.
    Season,
    /// Posted on game days.
    Games,
    /// Posted on its own day (distributions, donations, bowls).
    Day,
}

/// The money year's month order: the NFL year from March, college and
/// high school from July.
pub fn month_share(spread: Spread, month: u32) -> f64 {
    match spread {
        Spread::Even => 1.0 / 12.0,
        // August to January carry the season: 70%, the rest 30% over six.
        Spread::Season => {
            if matches!(month, 8..=12 | 1) {
                0.7 / 6.0
            } else {
                0.3 / 6.0
            }
        }
        Spread::Games | Spread::Day => 0.0,
    }
}

/// Opens the books the first time the world runs (or an older file loads).
pub(crate) fn ensure(w: &mut World) {
    if w.finance.ready {
        return;
    }
    w.finance.ready = true;
    nfl::open(w);
    college::open(w);
    hs::open(w);
}

/// The money side of a day: every dated step.
pub(crate) fn day(w: &mut World, d: Date) {
    ensure(w);
    let (y, m, dd) = d.parts();
    // Monthly accruals on the first of each month.
    if dd == 1 && w.finance.posted_month != Some((y, m)) {
        w.finance.posted_month = Some((y, m));
        nfl::month(w, m);
        college::month(w, m);
        hs::month(w, m);
    }
    if d == w.dates.start {
        nfl::year_start(w);
    }
    if d == w.dates.money_year {
        college::money_year(w);
        hs::money_year(w);
    }
    nfl::dated(w, d);
    college::dated(w, d);
    hs::dated(w, d);
}

/// Old world files: everything new is filled (later extras topic 1), and
/// the conference moves posted on the year's first day move to 1 July,
/// the day they take effect (conference moves 1a).
pub fn after_load(w: &mut World) {
    ensure(w);
    college::redate_moves(w);
}

/// A coach fired with time left on his deal: the buyout lands on his
/// employer's books (paid over the money year).
pub(crate) fn buyout(w: &mut World, employer: crate::world::coach::Employer, amount: f64) {
    use crate::world::coach::Employer;
    if amount <= 0.0 || !w.finance.ready {
        return;
    }
    match employer {
        Employer::College(p) => {
            if let Some(d) = w.finance.college.get_mut(p) {
                d.now.l[college::BUYOUTS] += amount as f32;
                d.cash_month(w.today.month(), -(amount as f32));
            }
        }
        Employer::Nfl(t) => {
            if let Some(c) = w.finance.nfl.get_mut(t) {
                c.now.l[nfl::COACHES] += amount as f32;
            }
        }
        Employer::HighSchool(_) | Employer::Other(_) => {}
    }
}

/// A stable draw in 0-1 for an entity and a purpose (its own stream: no
/// other result in the world moves).
pub(crate) fn draw(w: &World, what: u64, who: u64, year: i32) -> f64 {
    key_uniform(w.options.seed as u64 ^ 0xF1A2_0000_0000 ^ what << 40 ^ (year as u64) << 20, who)
}

/// A normal draw from the same keyed stream.
pub(crate) fn draw_normal(w: &World, what: u64, who: u64, year: i32) -> f64 {
    let u1 = draw(w, what, who, year).max(1e-9);
    let u2 = draw(w, what ^ 0x55, who, year);
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

pub(crate) fn post(w: &mut World, e: Event) {
    w.calendar.post(e);
}

/// A money story on today's date.
pub(crate) fn story(w: &World, level: Level, loud: Loudness, text: String) -> Event {
    Event::new(w.today, level, Kind::News, loud, text).rule("money")
}

/// Money in words: "$1.2 million", "$480,000", "$3.4 billion".
pub fn dollars(v: f64) -> String {
    let a = v.abs();
    let s = if a >= 1.0e9 {
        format!("${:.2} billion", a / 1.0e9)
    } else if a >= 1.0e6 {
        format!("${:.1} million", a / 1.0e6)
    } else if a >= 1.0e4 {
        format!("${},000", (a / 1000.0).round() as i64)
    } else {
        format!("${}", a.round() as i64)
    };
    if v < 0.0 { format!("-{s}") } else { s }
}

/// The inverse of the standard normal (Acklam), for turning a win chance
/// into a points margin.
pub fn probit(p: f64) -> f64 {
    let p = p.clamp(1e-6, 1.0 - 1e-6);
    const A: [f64; 6] = [
        -3.969683028665376e1,
        2.209460984245205e2,
        -2.759285104469687e2,
        1.383577518672690e2,
        -3.066479806614716e1,
        2.506628277459239,
    ];
    const B: [f64; 5] =
        [-5.447609879822406e1, 1.615858368580409e2, -1.556989798598866e2, 6.680131188771972e1, -1.328068155288572e1];
    const C: [f64; 6] = [
        -7.784894002430293e-3,
        -3.223964580411365e-1,
        -2.400758277161838,
        -2.549732539343734,
        4.374664141464968,
        2.938163982698783,
    ];
    const D: [f64; 4] = [7.784695709041462e-3, 3.224671290700398e-1, 2.445134137142996, 3.754408661907416];
    let lo = 0.02425;
    if p < lo {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= 1.0 - lo {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    }
}
