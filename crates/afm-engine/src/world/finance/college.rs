//! A college athletic department's books (later extras topic 1):
//! conference and playoff money, tickets with the priority-point seat
//! donations, gifts, NCAA money, media rights and licensing, basketball and
//! the other sports, student fees and the university's support, against
//! football's coaches, staff, recruiting and travel, scholarships, revenue
//! sharing (from July 2025) and the House back damages, facility debt,
//! guarantee games, bowl trips and the other sports. The money year runs
//! July to June. Also: attendance by game, renewals, the AD's facility
//! projects, conference media deals after 2026 and realignment by media
//! money, and the conference moves' own days (1a) and reach (1b).

use super::data;
use super::{Book, Line, Owner, Project, ProjectKind, Spread, dollars, draw, draw_normal, story};
use crate::calendar::{Date, Event, Kind, Level, Loudness};
use crate::rng::clamp;
use crate::world::World;
use crate::world::data::Division;
use crate::world::schedule::GameKind;

pub const CONFERENCE: usize = 0;
pub const POSTSEASON: usize = 1;
pub const TICKETS: usize = 2;
pub const SEAT_DONATIONS: usize = 3;
pub const GIFTS: usize = 4;
pub const NCAA: usize = 5;
pub const LICENSING: usize = 6;
pub const GUARANTEES_IN: usize = 7;
pub const OTHER_SPORTS: usize = 8;
pub const STUDENT_FEES: usize = 9;
pub const SUPPORT: usize = 10;
pub const COACHES: usize = 11;
pub const BUYOUTS: usize = 12;
pub const STAFF: usize = 13;
pub const RECRUITING: usize = 14;
pub const SCHOLARSHIPS: usize = 15;
pub const REV_SHARE: usize = 16;
pub const HOUSE: usize = 17;
pub const DEBT: usize = 18;
pub const TRAVEL: usize = 19;
pub const GUARANTEES_OUT: usize = 20;
pub const BOWL_TRIP: usize = 21;
pub const OTHER_COSTS: usize = 22;
pub const ADMIN: usize = 23;
pub const N: usize = 24;
pub const INCOME: usize = 11;

pub const LINES: [Line; N] = [
    Line { name: "Conference distribution", income: true, spread: Spread::Even },
    Line { name: "Bowl and playoff allowances", income: true, spread: Spread::Day },
    Line { name: "Football tickets", income: true, spread: Spread::Games },
    Line { name: "Seat donations (priority points)", income: true, spread: Spread::Day },
    Line { name: "Other gifts", income: true, spread: Spread::Even },
    Line { name: "NCAA distribution", income: true, spread: Spread::Day },
    Line { name: "Media rights, licensing and sponsors", income: true, spread: Spread::Even },
    Line { name: "Guarantee games received", income: true, spread: Spread::Games },
    Line { name: "Basketball and other sports", income: true, spread: Spread::Season },
    Line { name: "Student fees", income: true, spread: Spread::Even },
    Line { name: "University support", income: true, spread: Spread::Even },
    Line { name: "Football coaches' pay", income: false, spread: Spread::Even },
    Line { name: "Buyouts and exit fees", income: false, spread: Spread::Day },
    Line { name: "Football support staff", income: false, spread: Spread::Even },
    Line { name: "Recruiting", income: false, spread: Spread::Season },
    Line { name: "Scholarships (all sports)", income: false, spread: Spread::Even },
    Line { name: "Revenue sharing with players", income: false, spread: Spread::Even },
    Line { name: "House settlement back damages", income: false, spread: Spread::Even },
    Line { name: "Facility debt", income: false, spread: Spread::Even },
    Line { name: "Football travel", income: false, spread: Spread::Season },
    Line { name: "Guarantee games paid", income: false, spread: Spread::Games },
    Line { name: "Bowl and playoff trips", income: false, spread: Spread::Day },
    Line { name: "Other sports' operations", income: false, spread: Spread::Even },
    Line { name: "Administration and upkeep", income: false, spread: Spread::Even },
];

/// The rule tag every conference move carries (1b reads it).
pub const MOVE_RULE: &str = "conference realignment";

/// A closed season's numbers beside its books.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Season {
    pub year: i32,
    pub home_games: u8,
    pub attendance: u32,
    pub sellouts: u8,
    pub price: f32,
    pub renewal: f32,
    pub season_tickets: u32,
    pub reserves: f32,
}

/// One athletic department.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Dept {
    pub years: Vec<Book<N>>,
    pub seasons: Vec<Season>,
    pub budget: Book<N>,
    pub now: Book<N>,
    pub months: [f32; 12],
    pub price: f32,
    pub season_tickets: u32,
    pub renewal: f32,
    /// The average yearly donation a season ticket carries.
    pub priority: f32,
    /// Money kept from good years (negative: owed to the university).
    pub reserves: f32,
    /// Basketball's pull, 0-1, moving a little each year.
    pub hoops: f32,
    /// Student athletic fee per student a year (2024 dollars).
    pub fee: f32,
    /// Football's spending against its level's best, 0-1: what the books
    /// let the staff, recruiting and support buy.
    pub level: f32,
    /// The season it joined its conference, for a newcomer's partial share.
    pub joined: i32,
    /// This season's home games: gates, guarantees.
    pub home_games: u8,
    pub sellouts: u8,
    pub attendance: u32,
    /// The money year you last asked the athletic director for a facility.
    pub asked: i32,
}

impl Dept {
    pub fn cash_month(&mut self, month: u32, v: f32) {
        // The money year opens in July.
        self.months[(month as usize + 5) % 12] += v;
    }
    pub fn revenue(b: &Book<N>) -> f64 {
        b.sum(0..INCOME)
    }
    pub fn costs(b: &Book<N>) -> f64 {
        b.sum(INCOME..N)
    }
}

/// A conference's media deal after 2026: what it pays each member a year,
/// when it runs out, and what the networks pay for a member's pull.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Deal {
    pub conference: String,
    pub per_school: f32,
    pub until: i32,
    pub k: f32,
}

/// Each FBS conference's deal as it stood in 2026, and when it runs out
/// (public reporting: ESPN-SEC to 2034, the Big Ten's to 2030, the Big 12
/// to 2031, the ACC's grant of rights to 2036; the others proposed, not
/// confirmed).
const DEAL_ENDS: [(&str, i32); 9] = [
    ("SEC", 2034),
    ("Big Ten", 2030),
    ("Big 12", 2031),
    ("ACC", 2036),
    ("Pac-12", 2031),
    ("American", 2031),
    ("Mountain West", 2032),
    ("Sun Belt", 2031),
    ("MAC", 2027),
];

/// A program's level: 0 the power conferences of the day, 1 the rest of
/// FBS, 2 FCS, 3 below.
pub fn tier(w: &World, p: usize, year: i32) -> u8 {
    let prog = &w.programs[p];
    match prog.division {
        Division::FBS => {
            let c = prog.conference.as_str();
            let power = match c {
                "SEC" | "Big Ten" | "Big 12" | "ACC" | "Big Eight" | "SWC" => true,
                "Pac-12" | "Pac-10" => year <= 2023,
                "Big East" => year <= 2012,
                _ => prog.real_name == "Notre Dame",
            };
            if power { 0 } else { 1 }
        }
        Division::FCS => 2,
        _ => 3,
    }
}

/// A school's pull with the networks: its fans, name and wins, and the
/// size of its home state's market.
pub fn media_value(w: &World, p: usize) -> f64 {
    let prog = &w.programs[p];
    let pop = state_people(&w.states[prog.state].code);
    let market = (pop / 6.0).powf(0.25);
    (0.55 * prog.fans.powf(1.5) + 0.25 * prog.prestige + 0.2 * prog.tradition) * market
}

/// A state's people in millions (2020 Census, rounded).
fn state_people(code: &str) -> f64 {
    match code {
        "CA" => 39.5,
        "TX" => 29.1,
        "FL" => 21.5,
        "NY" => 20.2,
        "PA" => 13.0,
        "IL" => 12.8,
        "OH" => 11.8,
        "GA" => 10.7,
        "NC" => 10.4,
        "MI" => 10.1,
        "NJ" => 9.3,
        "VA" => 8.6,
        "WA" => 7.7,
        "AZ" => 7.2,
        "MA" => 7.0,
        "TN" => 6.9,
        "IN" => 6.8,
        "MD" => 6.2,
        "MO" => 6.2,
        "WI" => 5.9,
        "CO" => 5.8,
        "MN" => 5.7,
        "SC" => 5.1,
        "AL" => 5.0,
        "LA" => 4.7,
        "KY" => 4.5,
        "OR" => 4.2,
        "OK" => 4.0,
        "CT" => 3.6,
        "UT" => 3.3,
        "IA" => 3.2,
        "NV" => 3.1,
        "AR" => 3.0,
        "MS" => 3.0,
        "KS" => 2.9,
        "NM" => 2.1,
        "NE" => 2.0,
        "ID" => 1.8,
        "WV" => 1.8,
        "HI" => 1.5,
        "NH" | "ME" => 1.4,
        "MT" | "RI" => 1.1,
        "DE" => 1.0,
        "SD" | "ND" => 0.8,
        "AK" | "DC" | "VT" | "WY" => 0.7,
        _ => 2.0,
    }
}

/// The average ticket in 2024 dollars by level and pull (students sit
/// cheap): about $70 at the biggest programs, $25 in the Group of 5, $15
/// in FCS, $8 in Division II and less below.
fn ticket(w: &World, p: usize, year: i32) -> f64 {
    let prog = &w.programs[p];
    let base = match tier(w, p, year) {
        0 => 22.0 + 60.0 * prog.fans * prog.fans,
        1 => 14.0 + 20.0 * prog.fans,
        2 => 10.0 + 10.0 * prog.fans,
        _ => match prog.division {
            Division::D2 => 8.0,
            Division::NAIA => 6.0,
            _ => 5.0,
        },
    };
    base * data::college_index(year)
}

/// Opens every department's books.
pub(crate) fn open(w: &mut World) {
    let year = w.year;
    let n = w.programs.len();
    let mut depts = Vec::with_capacity(n);
    for p in 0..n {
        let prog = &w.programs[p];
        let mut d = Dept { joined: year - 10, renewal: 0.9, ..Default::default() };
        d.hoops = clamp(0.25 + 0.45 * prog.prestige + 0.3 * draw_normal(w, 0x4005, p as u64, 0), 0.0, 1.0) as f32;
        let t = tier(w, p, year);
        d.fee = match t {
            0 => 60.0 * draw(w, 0xFEE0, p as u64, 0),
            1 => 280.0 + 320.0 * draw(w, 0xFEE1, p as u64, 0),
            2 => 300.0 + 400.0 * draw(w, 0xFEE2, p as u64, 0),
            _ => 120.0 + 200.0 * draw(w, 0xFEE3, p as u64, 0),
        } as f32;
        d.price = ticket(w, p, year) as f32;
        d.season_tickets = (prog.stadium as f64 * clamp(0.15 + 0.6 * prog.fans, 0.05, 0.7)) as u32;
        d.priority = priority(w, p, year) as f32;
        d.reserves = 0.0;
        depts.push(d);
    }
    w.finance.college = depts;
    // The money year runs July to June: before 1 July the books open on
    // the year that ends then.
    let fy = if w.today < w.dates.money_year { year - 1 } else { year };
    for p in 0..n {
        open_year(w, p, fy);
    }
    if year >= 2026 {
        open_deals(w, year);
    }
}

/// What a season ticket's seat donation runs at a school: nothing much
/// where seats go begging, hundreds of dollars a seat where they don't.
fn priority(w: &World, p: usize, year: i32) -> f64 {
    let prog = &w.programs[p];
    let pull = clamp((prog.fans - 0.45) / 0.55, 0.0, 1.0);
    let top = match tier(w, p, year) {
        0 => 900.0,
        1 => 120.0,
        2 => 40.0,
        _ => 0.0,
    };
    top * pull * pull * data::college_index(year)
}

/// Football scholarships by level (equivalencies counted as full ones).
fn football_aid(d: Division) -> f64 {
    match d {
        Division::FBS => 85.0,
        Division::FCS => 63.0,
        Division::D2 => 36.0,
        Division::NAIA => 24.0,
        Division::JUCO => 85.0,
        Division::D3 => 0.0,
    }
}

/// The year's budget, set when the money year opens in July.
pub(crate) fn open_year(w: &mut World, p: usize, year: i32) {
    let prog = &w.programs[p];
    let t = tier(w, p, year);
    let idx = data::college_index(year);
    let coach_idx = data::coach_index(year);
    let fans = prog.fans;
    let mut b = Book::<N>::new(year);
    let dept = &w.finance.college[p];
    // Money in.
    let conf = match w.finance.deals.iter().find(|d| d.conference == prog.conference) {
        Some(d) if prog.division == Division::FBS => d.per_school as f64,
        _ => data::conference_payout(&prog.conference, prog.division, year),
    };
    // A newcomer gets part of a share for its first years (the Big 12's
    // half shares in 2023-24, the Pac-12's and the SEC's ramps).
    let share = match year - dept.joined {
        ..=0 => 0.6,
        1 => 0.75,
        2 => 0.9,
        _ => 1.0,
    };
    b.l[CONFERENCE] = (conf * share) as f32;
    let home = expected_home_games(w, p);
    let att = expected_crowd(w, p) * home;
    b.l[TICKETS] = (att * dept.price as f64 * 0.85) as f32;
    b.l[SEAT_DONATIONS] = (dept.season_tickets as f64 * dept.priority as f64) as f32;
    let nil_era = year >= 2021;
    b.l[GIFTS] = (prog.collective.boosters() * if nil_era { 0.45 } else { 0.9 } * idx.min(1.2)) as f32;
    b.l[NCAA] = data::ncaa_distribution(prog.division, dept.hoops as f64, year) as f32;
    b.l[LICENSING] = (match t {
        0 => 4.0e6 + 34.0e6 * fans.powf(1.6),
        1 => 1.0e6 + 3.0e6 * fans,
        2 => 0.2e6 + 0.8e6 * fans,
        _ => 0.03e6 + 0.1e6 * fans,
    } * idx) as f32;
    b.l[OTHER_SPORTS] = (match t {
        0 => 4.0e6 + 32.0e6 * dept.hoops as f64,
        1 => 1.0e6 + 6.0e6 * dept.hoops as f64,
        2 => 0.4e6 + 2.0e6 * dept.hoops as f64,
        _ => 0.05e6 + 0.3e6 * dept.hoops as f64,
    } * idx) as f32;
    b.l[STUDENT_FEES] = (prog.enrollment as f64 * dept.fee as f64 * data::hs_index(year)) as f32;
    // Money out: football first.
    let staff_ids =
        [prog.head_coach, prog.oc, prog.dc].into_iter().chain(prog.position_coaches.iter().copied()).flatten();
    let pay: f64 = staff_ids.filter_map(|c| w.coaches.get(c).and_then(|c| c.contract)).map(|k| k.salary).sum::<f64>();
    let pay = if pay > 0.0 { pay * coach_idx } else { default_pay(prog.division, t) * coach_idx };
    b.l[COACHES] = pay as f32;
    let room = clamp(dept.reserves as f64 / 20.0e6, -0.3, 0.3);
    let spend = clamp(0.35 + 0.45 * prog.prestige + 0.2 * fans + room, 0.1, 1.0);
    let (staff_top, recruit_top, travel) = match t {
        0 => (24.0e6, 5.0e6, 5.0e6),
        1 => (5.0e6, 0.8e6, 2.2e6),
        2 => (1.4e6, 0.3e6, 0.8e6),
        _ => (0.25e6, 0.05e6, 0.2e6),
    };
    b.l[STAFF] = (staff_top * spend * idx) as f32;
    b.l[RECRUITING] = (recruit_top * spend * idx) as f32;
    b.l[TRAVEL] = (travel * idx) as f32;
    let coa = 48_000.0 * data::hs_index(year);
    b.l[SCHOLARSHIPS] = (football_aid(prog.division) * coa * 3.2) as f32;
    let opt = match t {
        0 => 0.95,
        1 => 0.25 + 0.3 * fans,
        _ => 0.0,
    };
    b.l[REV_SHARE] = (data::share_cap(year) * opt) as f32;
    if (2025..=2034).contains(&year) && matches!(prog.division, Division::FBS | Division::FCS) {
        b.l[HOUSE] = match t {
            0 => 2.5e6,
            1 => 1.0e6,
            _ => 0.45e6,
        } as f32;
    }
    let debt: f64 = w
        .finance
        .projects
        .iter()
        .filter(|x| x.owner == Owner::College(p) && x.started + 22 > year)
        .map(|x| (x.cost - x.pledged - x.public).max(0.0) as f64 * 0.075)
        .sum();
    b.l[DEBT] = (debt
        + prog.facilities
            * match t {
                0 => 9.0e6,
                1 => 2.0e6,
                2 => 0.5e6,
                _ => 0.1e6,
            }
            * idx) as f32;
    b.l[GUARANTEES_OUT] = 0.0;
    // The other sports and the offices take what is left, down to the
    // floor every department carries (Title IX's women's sports, the
    // NCAA's sport minimums); the university covers the gap up to its
    // level's usual share (USA Today's NCAA finances: Power 4 schools take
    // little, most others half or more).
    let floor = match (t, prog.division) {
        (0, _) => 34.0e6,
        (1, _) => 14.0e6,
        (2, _) => 7.0e6,
        (_, Division::D2) => 3.0e6,
        (_, Division::JUCO) => 0.7e6,
        _ => 1.6e6,
    } * idx;
    let income = b.sum(0..INCOME);
    let football = b.sum(INCOME..N);
    let left = income - football;
    let cap = match t {
        0 => 0.12,
        1 => 0.6,
        2 => 0.75,
        _ => 0.92,
    };
    let need = (floor - left).max(0.0);
    let support = need.min(cap * (football + floor));
    b.l[SUPPORT] = support as f32;
    let other = (left + support).max(floor * 0.8);
    // A rich department keeps a little back; the rest goes to the sports.
    let keep = if left > floor { 0.04 * (left - floor) } else { 0.0 };
    b.l[OTHER_COSTS] = ((other - keep) * 0.72) as f32;
    b.l[ADMIN] = ((other - keep) * 0.28) as f32;
    let level = clamp((b.l[STAFF] + b.l[RECRUITING]) as f64 / ((staff_top + recruit_top) * idx), 0.0, 1.0);
    let d = &mut w.finance.college[p];
    d.level = level as f32;
    d.budget = b;
    d.now = Book::new(year);
    d.months = [0.0; 12];
    d.home_games = 0;
    d.sellouts = 0;
    d.attendance = 0;
}

fn default_pay(d: Division, t: u8) -> f64 {
    match (t, d) {
        (0, _) => 16.0e6,
        (1, _) => 4.5e6,
        (2, _) => 1.4e6,
        (_, Division::D2) => 0.5e6,
        _ => 0.3e6,
    }
}

fn expected_home_games(w: &World, p: usize) -> f64 {
    match w.college.as_ref() {
        Some(cs) if cs.year == w.year => {
            cs.games.iter().filter(|g| g.home == p && g.site.is_none() && g.kind == GameKind::Regular).count() as f64
        }
        _ => 6.0,
    }
}

/// The crowd a program draws on an average Saturday.
fn expected_crowd(w: &World, p: usize) -> f64 {
    let prog = &w.programs[p];
    let x = 0.2 + 0.62 * prog.fans + 0.12 * prog.recent + 0.06;
    let seats = prog.crowd_seats() as f64;
    (x * seats).min(prog.stadium as f64)
}

/// The month's money.
pub(crate) fn month(w: &mut World, m: u32) {
    for d in w.finance.college.iter_mut() {
        let mut net = 0.0f32;
        for (k, line) in LINES.iter().enumerate() {
            let v = (d.budget.l[k] as f64 * super::month_share(line.spread, m)) as f32;
            if v == 0.0 {
                continue;
            }
            d.now.l[k] += v;
            net += if line.income { v } else { -v };
        }
        d.cash_month(m, net);
    }
}

/// 1 July: last money year closes, this one opens; the conferences' deals
/// run on and renew; the ADs decide on facility projects; realignment.
pub(crate) fn money_year(w: &mut World) {
    let year = w.year;
    let n = w.finance.college.len().min(w.programs.len());
    for p in 0..n {
        close_year(w, p);
    }
    if year >= 2026 {
        if w.finance.deals.is_empty() {
            open_deals(w, year);
        } else {
            run_deals(w, year);
        }
    }
    for p in 0..n {
        let prog = &w.programs[p];
        let price = ticket(w, p, year) as f32;
        let prio = priority(w, p, year) as f32;
        let hoops = clamp(
            w.finance.college[p].hoops as f64
                + 0.08 * draw_normal(w, 0x4006, p as u64, year)
                + 0.02 * (prog.prestige - 0.5),
            0.0,
            1.0,
        ) as f32;
        let d = &mut w.finance.college[p];
        d.price = price;
        d.priority = prio;
        d.hoops = hoops;
        open_projects(w, p, year);
        open_year(w, p, year);
    }
    facility_decisions(w, year);
    if year >= 2027 {
        realign(w, year);
    }
}

fn close_year(w: &mut World, p: usize) {
    let d = &mut w.finance.college[p];
    if d.now.year == 0 {
        return;
    }
    // Lines paid only through the year are trued up to its budget.
    for k in [
        CONFERENCE,
        GIFTS,
        LICENSING,
        OTHER_SPORTS,
        STUDENT_FEES,
        SUPPORT,
        COACHES,
        STAFF,
        RECRUITING,
        SCHOLARSHIPS,
        REV_SHARE,
        HOUSE,
        DEBT,
        TRAVEL,
        OTHER_COSTS,
        ADMIN,
    ] {
        d.now.l[k] = d.now.l[k].max(d.budget.l[k] * 0.98);
    }
    let net = Dept::revenue(&d.now) - Dept::costs(&d.now);
    d.reserves = (d.reserves as f64 * 0.9 + net) as f32;
    let s = Season {
        year: d.now.year,
        home_games: d.home_games,
        attendance: d.attendance,
        sellouts: d.sellouts,
        price: d.price,
        renewal: d.renewal,
        season_tickets: d.season_tickets,
        reserves: d.reserves,
    };
    d.seasons.push(s);
    let book = std::mem::take(&mut d.now);
    d.years.push(book);
}

/// The money a program's books let it spend in a year, all sports (what
/// a buyout is weighed against).
pub fn athletic_budget(w: &World, p: usize) -> Option<f64> {
    let d = w.finance.college.get(p)?;
    (d.budget.year != 0).then(|| Dept::costs(&d.budget))
}

/// Football's spending level from the books, 0-1 (support staff buy).
pub fn football_level(w: &World, p: usize) -> Option<f64> {
    let d = w.finance.college.get(p)?;
    (d.budget.year != 0).then_some(d.level as f64)
}

/// Before a day's games: the closing lines (C8).
pub(crate) fn pregame(w: &mut World, idx: &[usize]) {
    let Some(cs) = w.college.as_ref() else { return };
    let lines: Vec<(usize, Option<[f32; 2]>)> = idx
        .iter()
        .map(|&k| {
            let g = &cs.games[k];
            (k, super::lines::college(w, g.home, g.away, g.site.is_some()))
        })
        .collect();
    let cs = w.college.as_mut().unwrap();
    for (k, l) in lines {
        cs.games[k].line = l;
    }
}

/// A college game's gate: demand from the fans, the program's recent
/// years, the opponent, the rivalry, homecoming, the record so far, the
/// weather and the kickoff; the stadium's seats cap it.
fn crowd(w: &World, k: usize) -> (u32, bool) {
    let cs = w.college.as_ref().unwrap();
    let g = &cs.games[k];
    let year = cs.year;
    let home = &w.programs[g.home];
    let away = &w.programs[g.away];
    if let Some(site) = g.site {
        // A bowl, a title game or a neutral-site game: the two fan bases in
        // a big stadium.
        let x = 0.45 + 0.35 * (home.fans + away.fans) / 2.0 + if g.kind == GameKind::Title { 0.2 } else { 0.0 };
        let cap = if site.indoor { 70_000.0 } else { 65_000.0 };
        return ((cap * x.min(1.0)) as u32, x >= 1.0);
    }
    let r = &cs.results[g.home];
    let wp = if r.games() > 0 { r.win_pct() - 0.5 } else { 0.0 };
    let played = g.played;
    let mut x = 0.2
        + 0.62 * home.fans
        + 0.12 * home.recent
        + 0.12 * away.fans
        + 0.25 * wp
        + if g.rivalry.is_some() { 0.12 } else { 0.0 }
        + if g.homecoming { 0.04 } else { 0.0 }
        + if g.conference { 0.03 } else { 0.0 }
        + if g.kind == GameKind::ConferenceTitle || matches!(g.kind, GameKind::Playoff(_)) { 0.2 } else { 0.0 };
    if let Some(pl) = played {
        if pl.rain {
            x -= 0.05;
        }
        if pl.snow {
            x -= 0.04;
        }
        if pl.temperature < 32.0 {
            x -= 0.03;
        }
        if pl.temperature > 92.0 {
            x -= 0.02;
        }
    }
    if g.kickoff.0 < 12 {
        x -= 0.03;
    }
    if !matches!(g.date.weekday(), crate::calendar::Weekday::Sat) {
        x -= 0.06;
    }
    x += 0.04 * draw_normal(w, 0xA77E, k as u64, year);
    let seats = home.crowd_seats() as f64;
    let people = x.max(0.15) * seats;
    let cap = home.stadium as f64;
    let sold = people >= cap;
    (people.min(cap * 1.01) as u32, sold)
}

/// After a day's games: crowds, gates, guarantee games, bowl money, the
/// lines settled and the upsets.
pub(crate) fn played(w: &mut World, idx: &[usize]) {
    let year = w.year;
    for &k in idx {
        let Some(cs) = w.college.as_ref() else { return };
        let g = &cs.games[k];
        let Some(pl) = g.played else { continue };
        if pl.cancelled {
            continue;
        }
        let (home, away, kind, neutral, line, date) = (g.home, g.away, g.kind, g.site.is_some(), g.line, g.date);
        let (att, sold) = crowd(w, k);
        w.college.as_mut().unwrap().games[k].attendance = att;
        let month = date.month();
        if !neutral && home < w.finance.college.len() {
            let price = w.finance.college[home].price as f64;
            let gate = att as f64 * price * 0.85;
            let d = &mut w.finance.college[home];
            d.now.l[TICKETS] += gate as f32;
            d.home_games += 1;
            d.attendance += att;
            if sold {
                d.sellouts += 1;
            }
            d.cash_month(month, gate as f32);
            // A guarantee game: the bigger program buys the home game.
            let (th, ta) = (tier(w, home, year), tier(w, away, year));
            if kind == GameKind::Regular && !w.college.as_ref().unwrap().games[k].conference && ta > th {
                let fee = data::guarantee(ta, year) as f32 * (0.8 + 0.4 * draw(w, 0x6A4A, k as u64, year) as f32);
                w.finance.college[home].now.l[GUARANTEES_OUT] += fee;
                w.finance.college[home].cash_month(month, -fee);
                if away < w.finance.college.len() {
                    w.finance.college[away].now.l[GUARANTEES_IN] += fee;
                    w.finance.college[away].cash_month(month, fee);
                }
            }
        }
        // Bowls and playoff games: the conference pays the trip's allowance;
        // the school buys its ticket block and pays the travel (many bowl
        // trips lose money).
        if matches!(kind, GameKind::Bowl(_) | GameKind::Playoff(_) | GameKind::Title | GameKind::Celebration) {
            for p in [home, away] {
                if p >= w.finance.college.len() {
                    continue;
                }
                let t = tier(w, p, year);
                let idx = data::college_index(year);
                let (allow, trip) = match (t, kind) {
                    (_, GameKind::Playoff(_) | GameKind::Title) if w.programs[p].division == Division::FBS => {
                        (3.0e6, 2.6e6)
                    }
                    (0, _) => (2.2e6, 2.6e6),
                    (1, _) => (0.9e6, 1.1e6),
                    _ => (0.15e6, 0.25e6),
                };
                let d = &mut w.finance.college[p];
                d.now.l[POSTSEASON] += (allow * idx) as f32;
                d.now.l[BOWL_TRIP] += (trip * idx) as f32;
                d.cash_month(month, ((allow - trip) * idx) as f32);
            }
        }
        if let Some(line) = line {
            super::lines::learn(w, Level::College, [home, away], line, pl.score);
            let coach = |p: usize| w.programs[p].sideline.or(w.programs[p].head_coach);
            let names = [
                w.programs[home].name(w.options.names).to_string(),
                w.programs[away].name(w.options.names).to_string(),
            ];
            let fbs = w.programs[home].division == Division::FBS || w.programs[away].division == Division::FBS;
            if fbs || w.seat_program(home) || w.seat_program(away) {
                let about = if w.seat_program(away) { away } else { home };
                super::lines::settle(
                    w,
                    Level::College,
                    line,
                    pl.score,
                    [coach(home), coach(away)],
                    names,
                    (None, Some(about)),
                );
            } else {
                let c = [coach(home), coach(away)];
                let margin = pl.score[0] as f64 - pl.score[1] as f64 + line[0] as f64;
                let r = if margin > 0.0 {
                    [0, 1]
                } else if margin < 0.0 {
                    [1, 0]
                } else {
                    [2, 2]
                };
                for s in 0..2 {
                    if let Some(c) = c[s] {
                        w.finance.ats.entry(c).or_default()[r[s]] += 1;
                    }
                }
            }
        }
    }
}

/// Spring renewals: each program's season ticket holders renew for the
/// fall with their seat donation; a bad season dents both.
pub(crate) fn dated(w: &mut World, d: Date) {
    let year = d.year();
    // The renewal deadlines run from late March to early May.
    if (Date::ymd(year, 3, 25)..=Date::ymd(year, 5, 10)).contains(&d) {
        let n = w.finance.college.len().min(w.programs.len());
        for p in 0..n {
            let day = Date::ymd(year, 3, 25).add((draw(w, 0x2E4C, p as u64, 0) * 46.0) as i32);
            if day == d {
                renew(w, p);
            }
        }
    }
    // The NCAA's distributions go out in April.
    if d == Date::ymd(year, 4, 20) {
        for dept in w.finance.college.iter_mut() {
            let v = dept.budget.l[NCAA];
            dept.now.l[NCAA] += v;
            dept.cash_month(4, v);
        }
    }
    // Public schools' NCAA financial reports are published each winter.
    if d == Date::ymd(year, 1, 20)
        && let Some(p) = w.seat.and_then(|s| s.program)
        && w.seat_program(p)
        && let Some(b) = w.finance.college[p].years.last()
    {
        let rev = Dept::revenue(b);
        let cost = Dept::costs(b);
        let name = w.programs[p].name(w.options.names).to_string();
        let text = format!(
            "{name}'s NCAA financial report for {}-{:02}: revenue {}, expenses {}, university support {}.",
            b.year,
            (b.year + 1) % 100,
            dollars(rev),
            dollars(cost),
            dollars(b.l[SUPPORT] as f64)
        );
        let e = story(w, Level::College, Loudness::Medium, text).program(p);
        super::post(w, e);
    }
}

fn renew(w: &mut World, p: usize) {
    let year = w.year;
    let prog = &w.programs[p];
    let last = prog.seasons.last();
    let wp = last.map_or(0.5, |s| s.wins as f64 / (s.wins + s.losses).max(1) as f64);
    let noise = 0.02 * draw_normal(w, 0x2E4D, p as u64, year);
    let rate = clamp(0.8 + 0.12 * prog.fans + 0.15 * (wp - 0.5) + noise, 0.55, 0.98);
    let seats = prog.crowd_seats() as f64;
    let cap = prog.stadium as f64 * 0.7;
    let demand = seats * clamp(0.15 + 0.6 * prog.fans + 0.2 * (wp - 0.5), 0.05, 0.75);
    let d = &mut w.finance.college[p];
    d.renewal = rate as f32;
    let kept = d.season_tickets as f64 * rate;
    d.season_tickets = (kept + (demand - kept).max(0.0) * 0.6).min(cap) as u32;
    // The seat donation follows the same season: a bad year dents it twice.
    let donations = d.season_tickets as f64 * d.priority as f64 * clamp(0.85 + 0.3 * (wp - 0.5), 0.6, 1.1);
    d.now.l[SEAT_DONATIONS] += donations as f32;
    d.cash_month(w.today.month(), donations as f32);
    if w.seat_program(p) {
        let name = w.programs[p].name(w.options.names).to_string();
        let st = w.finance.college[p].season_tickets;
        let text = format!(
            "{name} season ticket renewals: {:.0}% renewed, {} season tickets, {} in seat donations.",
            rate * 100.0,
            st,
            dollars(donations)
        );
        let e = story(w, Level::College, Loudness::Medium, text).program(p);
        super::post(w, e);
    }
}

/// The ADs decide on facility projects each July: a department with money
/// in reserve and donors behind it builds (an indoor field, a football
/// building, a weight room or locker room), two to three years out; the
/// facilities number moves when it opens.
fn facility_decisions(w: &mut World, year: i32) {
    if !w.recording {
        return;
    }
    let n = w.finance.college.len().min(w.programs.len());
    for p in 0..n {
        if w.finance.projects.iter().any(|x| x.owner == Owner::College(p) && !x.done) || !w.programs[p].plays(year) {
            continue;
        }
        let prog = &w.programs[p];
        let t = tier(w, p, year);
        let fans = prog.fans;
        let nil_share = clamp((prog.collective.boosters() / 25.0e6).sqrt(), 0.0, 1.0);
        let scale = match prog.division {
            Division::FBS => 1.0,
            Division::FCS => 0.55,
            Division::D2 => 0.35,
            _ => 0.25,
        };
        let target = clamp(scale * (0.3 + 0.4 * fans) + 0.3 * nil_share, 0.0, 1.0);
        let gap = target - prog.facilities;
        let reserves = w.finance.college[p].reserves as f64;
        let big = prog.collective.big_booster.is_some();
        let chance = clamp(
            0.03 + 0.6 * gap.max(0.0) + if big { 0.05 } else { 0.0 } + clamp(reserves / 40.0e6, -0.05, 0.1),
            0.0,
            0.4,
        );
        if gap <= 0.02 || draw(w, 0xFAC1, p as u64, year) >= chance {
            continue;
        }
        let step = clamp(gap + 0.08 * draw(w, 0xFAC2, p as u64, year), 0.04, 0.2);
        let (kind, base) = match t {
            0 => ("football building", 140.0e6),
            1 => ("football operations center", 40.0e6),
            2 => ("indoor practice facility", 14.0e6),
            _ => ("weight room and locker room", 3.0e6),
        };
        let cost = base * (step / 0.12) * data::college_index(year);
        let pledged = cost * clamp(0.4 + 0.3 * nil_share + if big { 0.15 } else { 0.0 }, 0.2, 0.85);
        let opens = year + 2 + (draw(w, 0xFAC3, p as u64, year) < 0.4) as i32;
        w.finance.projects.push(Project {
            owner: Owner::College(p),
            kind: ProjectKind::Facility,
            name: kind.to_string(),
            cost: cost as f32,
            public: 0.0,
            pledged: pledged as f32,
            started: year,
            opens,
            capacity: 0,
            step: step as f32,
            done: false,
            asked: false,
        });
        if w.seat_program(p) || prog.division == Division::FBS && t == 0 {
            let name = prog.name(w.options.names).to_string();
            let text = format!(
                "{name} breaks ground on a {} new {kind}, opening in {opens}; donors have pledged {}.",
                dollars(cost),
                dollars(pledged)
            );
            let loud = if w.seat_program(p) { Loudness::Big } else { Loudness::Small };
            let e = story(w, Level::College, loud, text).program(p);
            super::post(w, e);
        }
    }
}

/// A project opens with the money year.
fn open_projects(w: &mut World, p: usize, year: i32) {
    let ready: Vec<usize> = (0..w.finance.projects.len())
        .filter(|&k| {
            let x = &w.finance.projects[k];
            x.owner == Owner::College(p) && !x.done && x.opens <= year
        })
        .collect();
    for k in ready {
        let x = w.finance.projects[k].clone();
        w.finance.projects[k].done = true;
        let prog = &mut w.programs[p];
        prog.facilities = clamp(prog.facilities + x.step as f64, 0.02, 1.0);
        if x.kind == ProjectKind::Stadium && x.capacity > 0 {
            prog.stadium = x.capacity;
        }
        if w.recording {
            prog.story.push((year, format!("New football facilities: a {} {} opens.", dollars(x.cost as f64), x.name)));
        }
        if w.seat_program(p) {
            let name = w.programs[p].name(w.options.names).to_string();
            let e = story(w, Level::College, Loudness::Big, format!("{name}'s new {} opens.", x.name)).program(p);
            super::post(w, e);
        }
    }
}

/// The FBS conferences' deals as they stood when the world takes over.
fn open_deals(w: &mut World, year: i32) {
    let mut confs: Vec<String> = w
        .programs
        .iter()
        .filter(|p| p.division == Division::FBS && !p.conference.contains("Independent"))
        .map(|p| p.conference.clone())
        .collect();
    confs.sort();
    confs.dedup();
    let mut deals = Vec::new();
    for c in confs {
        let members: Vec<usize> = (0..w.programs.len())
            .filter(|&p| w.programs[p].conference == c && w.programs[p].division == Division::FBS)
            .collect();
        if members.is_empty() {
            continue;
        }
        let per = data::conference_payout(&c, Division::FBS, year);
        let mean = members.iter().map(|&p| media_value(w, p)).sum::<f64>() / members.len() as f64;
        let until = DEAL_ENDS.iter().find(|d| d.0 == c).map_or(year + 5, |d| d.1);
        deals.push(Deal { conference: c, per_school: per as f32, until, k: (per / mean.max(0.05)) as f32 });
    }
    w.finance.deals = deals;
}

/// The deals run on with their yearly raises; one that runs out is sold
/// again for what its members are worth now.
fn run_deals(w: &mut World, year: i32) {
    for k in 0..w.finance.deals.len() {
        let c = w.finance.deals[k].conference.clone();
        let members: Vec<usize> = (0..w.programs.len())
            .filter(|&p| w.programs[p].conference == c && w.programs[p].division == Division::FBS)
            .collect();
        if members.is_empty() {
            continue;
        }
        let d = &mut w.finance.deals[k];
        if year <= d.until {
            d.per_school *= 1.035;
            continue;
        }
        let mean = members.iter().map(|&p| media_value(w, p)).sum::<f64>() / members.len() as f64;
        let d = &mut w.finance.deals[k];
        let old = d.per_school as f64;
        let new = d.k as f64 * mean * 1.035f64.powi(year - 2026);
        d.per_school = new as f32;
        d.until = year + 7;
        let until = d.until;
        let dir = if new > old * 1.05 {
            "a raise"
        } else if new < old * 0.95 {
            "a cut"
        } else {
            "about the same"
        };
        let text = format!("The {c} signs a new media deal through {until}: {} a school a year, {dir}.", dollars(new));
        let loud = if tier_name_power(&c) { Loudness::Big } else { Loudness::Medium };
        let e = story(w, Level::College, loud, text).rule(MOVE_RULE);
        super::post(w, e);
    }
}

fn tier_name_power(c: &str) -> bool {
    matches!(c, "SEC" | "Big Ten" | "Big 12" | "ACC")
}

/// Realignment by media money (after 2026): a conference adds a school
/// that raises what each member is paid, a school goes where it is paid
/// more after its exit fee, and a conference that falls under eight
/// football members fills from below. Moves are announced in the summer
/// and take effect the next season; the history before 2026 is the real
/// one. How often and by how much is proposed, not confirmed: the 2010-2024
/// moves ran about four a year, in waves around new media deals.
pub(crate) fn realign(w: &mut World, year: i32) {
    let pending: Vec<usize> = w.realign.iter().map(|r| r.0).collect();
    let idx = data::college_index(year);
    let mut moves: Vec<(f64, usize, String)> = Vec::new();
    let deals = w.finance.deals.clone();
    let members = |w: &World, c: &str| -> Vec<usize> {
        (0..w.programs.len())
            .filter(|&p| {
                w.programs[p].conference == c
                    && w.programs[p].division == Division::FBS
                    && !w.realign.iter().any(|r| r.0 == p)
            })
            .collect()
    };
    for p in 0..w.programs.len() {
        let prog = &w.programs[p];
        if prog.division != Division::FBS || pending.contains(&p) || prog.real_name == "Notre Dame" || !prog.plays(year)
        {
            continue;
        }
        let own = deals.iter().find(|d| d.conference == prog.conference);
        let own_pay =
            own.map_or(data::conference_payout(&prog.conference, Division::FBS, year), |d| d.per_school as f64);
        let v = media_value(w, p);
        for d in &deals {
            if d.conference == prog.conference {
                continue;
            }
            let m = members(w, &d.conference);
            if m.len() >= 20 || m.is_empty() {
                continue;
            }
            let mean = m.iter().map(|&q| media_value(w, q)).sum::<f64>() / m.len() as f64;
            // The conference only takes a school that raises its members' pay.
            if v < mean * 0.97 && m.len() >= 10 {
                continue;
            }
            let new_pay = d.k as f64 * (mean * m.len() as f64 + v) / (m.len() + 1) as f64 * 1.035f64.powi(year - 2026);
            let exit = own_pay * if tier(w, p, year) == 0 { 2.0 } else { 0.8 };
            let gain = 0.85 * new_pay - own_pay - exit / 5.0;
            if gain > 4.0e6 * idx {
                moves.push((gain / own_pay.max(1.0e6), p, d.conference.clone()));
            }
        }
    }
    moves.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut taken: Vec<usize> = Vec::new();
    let mut joining: Vec<String> = Vec::new();
    let mut done = 0;
    for (score, p, c) in moves {
        if done >= 3 || taken.contains(&p) || joining.iter().filter(|x| **x == c).count() >= 2 {
            continue;
        }
        let chance = clamp(0.12 + 0.25 * score, 0.0, 0.6);
        if draw(w, 0x2EA1, p as u64, year) >= chance {
            continue;
        }
        taken.push(p);
        joining.push(c.clone());
        announce(w, p, &c, false);
        done += 1;
    }
    // A conference that has fallen under eight football members fills
    // from below: the best of FCS moves up, or a Group of 5 school moves over.
    for d in deals {
        let m = members(w, &d.conference);
        let incoming = w.realign.iter().filter(|r| r.1 == d.conference).count();
        if m.len() + incoming >= 8 {
            continue;
        }
        let mut best: Option<(f64, usize)> = None;
        for p in 0..w.programs.len() {
            let prog = &w.programs[p];
            if pending.contains(&p)
                || w.realign.iter().any(|r| r.0 == p)
                || !prog.plays(year)
                || prog.conference == d.conference
            {
                continue;
            }
            let ok = prog.division == Division::FCS || (prog.division == Division::FBS && tier(w, p, year) == 1);
            if !ok {
                continue;
            }
            let near = if prog.state == w.programs[m.first().copied().unwrap_or(p)].state { 0.1 } else { 0.0 };
            let s = media_value(w, p) + near;
            if best.is_none_or(|b| s > b.0) {
                best = Some((s, p));
            }
        }
        if let Some((_, p)) = best
            && draw(w, 0x2EA2, p as u64, year) < 0.6
        {
            let up = w.programs[p].division == Division::FCS;
            announce(w, p, &d.conference, up);
        }
    }
}

fn announce(w: &mut World, p: usize, conf: &str, move_up: bool) {
    let season = w.year;
    let from = w.programs[p].conference.clone();
    let name = w.programs[p].real_name.clone();
    w.realign.push((p, conf.to_string()));
    if move_up {
        w.finance.move_up.push(p);
    }
    let text = if move_up {
        format!("{name} accepts an invitation to move up to FBS and join the {conf}, from next season.")
    } else {
        format!("{name} accepts an invitation to leave the {from} for the {conf}, from next season.")
    };
    w.programs[p].story.push((season, text.clone()));
    w.log.rule_changes.push(text.clone());
    w.changes.applied.push(crate::calendar::rules::AppliedChange {
        change: crate::calendar::Change::ConferenceSwitch,
        season,
        detail: format!("{name}: {from} to {conf}"),
    });
    let loud = if tier_name_power(conf) || tier_name_power(&from) { Loudness::Big } else { Loudness::Medium };
    let e = Event::new(w.today, Level::College, Kind::RuleChange, loud, text).program(p).rule(MOVE_RULE);
    w.calendar.post(e);
}

/// The new football year: the moves announced last summer switch the
/// schedules over, and each is posted on 1 July, the day it takes effect.
pub(crate) fn switch(w: &mut World) {
    let season = w.year;
    for (p, conf) in std::mem::take(&mut w.realign) {
        let old = w.programs[p].conference.clone();
        let old_d = w.programs[p].division;
        let up = w.finance.move_up.contains(&p);
        if up {
            w.programs[p].division = Division::FBS;
        }
        w.programs[p].conference = conf.clone();
        if let Some(d) = w.finance.college.get_mut(p) {
            d.joined = season;
            // The exit fee leaves with the school.
            let exit = data::conference_payout(&old, old_d, season)
                * if matches!(old.as_str(), "SEC" | "Big Ten" | "Big 12" | "ACC") { 2.0 } else { 0.8 };
            if !up {
                d.now.l[BUYOUTS] += exit as f32;
            }
        }
        let name = w.programs[p].real_name.clone();
        let text = if up {
            format!("{name} officially moves up from FCS to FBS today, in the {conf}.")
        } else {
            format!("{name} officially leaves the {old} for the {conf} today.")
        };
        post_move(w, p, season, text, old_d.min(w.programs[p].division));
    }
    w.finance.move_up.clear();
}

/// A move's own day (1a): posted on 1 July of its season.
pub(crate) fn post_move(w: &mut World, p: usize, season: i32, text: String, d: Division) {
    let loud = match d {
        Division::FBS => Loudness::Big,
        Division::FCS => Loudness::Medium,
        _ => Loudness::Small,
    };
    let e =
        Event::new(Date::ymd(season, 7, 1), Level::College, Kind::RuleChange, loud, text).program(p).rule(MOVE_RULE);
    w.calendar.post(e);
}

/// A real move's words for its own day: "X officially leaves the A for the
/// B today." None when the line isn't a move.
pub fn officially(text: &str) -> Option<String> {
    let t = text.strip_suffix('.')?;
    if let Some((name, rest)) = t.split_once(" leaves the ") {
        if let Some((from, to)) = rest.split_once(" for the ") {
            return Some(format!("{name} officially leaves the {from} for the {to} today."));
        }
        if let Some(from) = rest.strip_suffix(" to play as an independent") {
            return Some(format!("{name} officially leaves the {from} today to play as an independent."));
        }
    }
    if let Some((name, to)) = t.split_once(" ends its years as an independent and joins the ") {
        return Some(format!("{name} officially joins the {to} today, ending its years as an independent."));
    }
    for dir in [" moves up from ", " moves down from "] {
        if let Some((name, rest)) = t.split_once(dir)
            && let Some((levels, conf)) = rest.split_once(" and plays ")
        {
            return Some(format!(
                "{name} officially{}{levels} today, {conf}.",
                dir.trim_end_matches(' ').to_string() + " "
            ));
        }
    }
    None
}

/// Old world files: the real moves posted on the football year's first
/// day move to 1 July, worded for the day.
pub(crate) fn redate_moves(w: &mut World) {
    let mut moved = Vec::new();
    for (date, list) in w.calendar.events.iter_mut() {
        let mut keep = Vec::new();
        for e in std::mem::take(list) {
            let is_move =
                e.kind == Kind::RuleChange && e.level == Level::College && e.program.is_some() && e.rule != MOVE_RULE;
            match officially(&e.text).filter(|_| is_move) {
                Some(text) => moved.push((date.year(), e, text)),
                None => keep.push(e),
            }
        }
        *list = keep;
    }
    w.calendar.events.retain(|_, l| !l.is_empty());
    for (year, mut e, text) in moved {
        e.text = text;
        e.rule = MOVE_RULE;
        e.when = crate::calendar::Moment { date: Date::ymd(year, 7, 1), minute: 9 * 60 };
        w.calendar.post(e);
    }
    for e in w.calendar.history.iter_mut() {
        let is_move =
            e.kind == Kind::RuleChange && e.level == Level::College && e.program.is_some() && e.rule != MOVE_RULE;
        if is_move && let Some(text) = officially(&e.text) {
            let y = e.when.date.year();
            e.text = text;
            e.rule = MOVE_RULE;
            e.when = crate::calendar::Moment { date: Date::ymd(y, 7, 1), minute: 9 * 60 };
        }
    }
}

/// Whether a conference move reaches your inbox (1b): FBS moves reach
/// every college coach; FCS and lower only a coach at that level; a high
/// school or NFL coach none. None when the line isn't a move.
pub fn move_reaches(w: &World, e: &Event) -> Option<bool> {
    if e.rule != MOVE_RULE {
        return None;
    }
    let Some(seat) = w.seat else { return Some(true) };
    let Some(p) = e.program.filter(|&p| p < w.programs.len()) else {
        // A media deal: power conference news reaches college coaches.
        return Some(seat.level == Level::College && e.loud >= Loudness::Big);
    };
    let d = w.programs[p].division;
    let fbs = d == Division::FBS || e.text.contains("FBS") || e.text.contains("Division I-A");
    Some(match seat.level {
        Level::College => fbs || seat.program.and_then(|q| w.programs.get(q)).is_some_and(|q| q.division == d),
        _ => false,
    })
}

/// Asking your AD for a facility project (people topic 12): he reads the
/// department's money, the donors and your standing.
pub fn ask_facility(w: &mut World, p: usize) -> Result<String, String> {
    let year = w.year;
    if w.finance.projects.iter().any(|x| x.owner == Owner::College(p) && !x.done) {
        return Err("A project is already under way.".into());
    }
    if w.finance.college.get(p).is_some_and(|d| d.asked == year) {
        return Err("You asked this year already.".into());
    }
    let prog = &w.programs[p];
    let t = tier(w, p, year);
    let reserves = w.finance.college[p].reserves as f64;
    let nil_share = clamp((prog.collective.boosters() / 25.0e6).sqrt(), 0.0, 1.0);
    let wp =
        prog.seasons.iter().rev().take(2).map(|s| s.wins as f64 / (s.wins + s.losses).max(1) as f64).sum::<f64>() / 2.0;
    let room = 1.0 - prog.facilities;
    let yes =
        clamp(0.15 + 0.4 * room + 0.25 * nil_share + 0.3 * (wp - 0.5) + clamp(reserves / 30.0e6, -0.2, 0.2), 0.05, 0.9);
    let ad = prog.ad.as_ref().map_or("The athletic director".to_string(), |a| a.name.clone());
    let name = prog.name(w.options.names).to_string();
    if draw(w, 0xA5C0, p as u64, year) >= yes {
        // Remember the ask, so it can't be asked again this year.
        w.finance.college[p].asked = year;
        let why = if reserves < 0.0 {
            "The department ran in the red last year."
        } else if wp < 0.45 {
            "Donors want to see the team win first."
        } else {
            "The donors aren't there for it yet."
        };
        return Ok(format!("{ad}: \"Not this year.\" {why}"));
    }
    let step = clamp(0.06 + 0.4 * room * 0.3, 0.05, 0.16);
    let (kind, base) = match t {
        0 => ("football building", 140.0e6),
        1 => ("football operations center", 40.0e6),
        2 => ("indoor practice facility", 14.0e6),
        _ => ("weight room and locker room", 3.0e6),
    };
    let cost = base * (step / 0.12) * data::college_index(year);
    let pledged = cost * clamp(0.35 + 0.3 * nil_share, 0.2, 0.8);
    w.finance.projects.push(Project {
        owner: Owner::College(p),
        kind: ProjectKind::Facility,
        name: kind.to_string(),
        cost: cost as f32,
        public: 0.0,
        pledged: pledged as f32,
        started: year,
        opens: year + 2,
        capacity: 0,
        step: step as f32,
        done: false,
        asked: true,
    });
    let text = format!("{name} approves a {} {kind} you asked for, opening in {}.", dollars(cost), year + 2);
    let e = story(w, Level::College, Loudness::Big, text.clone()).program(p);
    super::post(w, e);
    Ok(format!("{ad}: \"Let's build it.\" {text}"))
}
