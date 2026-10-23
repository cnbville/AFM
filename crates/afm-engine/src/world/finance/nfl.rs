//! An NFL club's books (later extras topic 1): equal national money,
//! local money from its market, stadium and fans, the visiting gate pool,
//! player cash, football and business costs and stadium debt. Attendance
//! by game, season ticket renewals and the waiting list, the blackout
//! rule (1973-2014), stadiums by their real history to 2026 and the
//! world's own after (public votes, new stadiums, renovations, relocation
//! by the owners' vote), Forbes values, sales, and the NFLPA's team report
//! cards (P1).

use super::data;
use super::{Book, Line, Owner, Project, ProjectKind, Spread, dollars, draw, draw_normal, story};
use crate::calendar::{Date, Level, Loudness, Weekday};
use crate::rng::clamp;
use crate::world::World;
use crate::world::franchise::{self, Roof};

pub const NATIONAL: usize = 0;
pub const TICKETS: usize = 1;
pub const PREMIUM: usize = 2;
pub const CONCESSIONS: usize = 3;
pub const SPONSORS: usize = 4;
pub const NAMING: usize = 5;
pub const LOCAL_MEDIA: usize = 6;
pub const PSL: usize = 7;
pub const POOL: usize = 8;
pub const EVENTS: usize = 9;
pub const PLAYERS: usize = 10;
pub const BENEFITS: usize = 11;
pub const COACHES: usize = 12;
pub const SCOUTING: usize = 13;
pub const MEDICAL: usize = 14;
pub const TRAVEL: usize = 15;
pub const AMENITIES: usize = 16;
pub const STADIUM_OPS: usize = 17;
pub const GAME_DAY: usize = 18;
pub const BUSINESS: usize = 19;
pub const DEBT: usize = 20;
pub const N: usize = 21;
/// Lines 0..INCOME are money in.
pub const INCOME: usize = 10;

pub const LINES: [Line; N] = [
    Line { name: "National revenue (TV, NFL Properties)", income: true, spread: Spread::Season },
    Line { name: "Tickets", income: true, spread: Spread::Games },
    Line { name: "Suites and club seats", income: true, spread: Spread::Games },
    Line { name: "Concessions and parking", income: true, spread: Spread::Games },
    Line { name: "Sponsors", income: true, spread: Spread::Even },
    Line { name: "Stadium naming rights", income: true, spread: Spread::Even },
    Line { name: "Local radio and preseason TV", income: true, spread: Spread::Season },
    Line { name: "Personal seat licenses", income: true, spread: Spread::Even },
    Line { name: "Visiting gate pool (net)", income: true, spread: Spread::Day },
    Line { name: "Other stadium events", income: true, spread: Spread::Even },
    Line { name: "Player salaries and bonuses (cash)", income: false, spread: Spread::Season },
    Line { name: "Player benefits", income: false, spread: Spread::Even },
    Line { name: "Coaches and football staff", income: false, spread: Spread::Even },
    Line { name: "Scouting", income: false, spread: Spread::Even },
    Line { name: "Medical, strength and performance", income: false, spread: Spread::Even },
    Line { name: "Team travel", income: false, spread: Spread::Season },
    Line { name: "Player amenities (food, family room)", income: false, spread: Spread::Even },
    Line { name: "Stadium operations and lease", income: false, spread: Spread::Even },
    Line { name: "Game-day staff and security", income: false, spread: Spread::Games },
    Line { name: "Business operations", income: false, spread: Spread::Even },
    Line { name: "Stadium debt and relocation fee", income: false, spread: Spread::Even },
];

/// The stadium a club plays in.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Home {
    pub name: String,
    pub capacity: u32,
    /// The season it opened, and the season of its last big renovation.
    pub opened: i32,
    pub renovated: i32,
    /// Share of its cost paid with public money, and the cost in dollars of the day.
    pub public: f32,
    pub cost: f32,
    pub dome: bool,
}

impl Home {
    pub fn age(&self, year: i32) -> i32 {
        (year - self.opened.max(self.renovated)).max(0)
    }
    /// What a modern building adds to local money: suites, clubs, seats
    /// sold as experiences, fading as it ages (SoFi, AT&T, Allegiant).
    pub fn premium(&self, year: i32) -> f64 {
        let age = self.age(year) as f64;
        0.55 * (-age / 14.0).exp() + 0.08
    }
}

/// One home game's gate.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Gate {
    pub date: Date,
    pub opp: u8,
    pub attendance: u32,
    pub sold_out: bool,
    pub blackout: bool,
    pub preseason: bool,
    pub playoff: bool,
}

/// A closed season's numbers beside its books.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Season {
    pub year: i32,
    pub home_games: u8,
    pub attendance: u32,
    pub sellouts: u8,
    pub blackouts: u8,
    pub price: f32,
    pub renewal: f32,
    pub waitlist: u32,
    /// Forbes-style value in August, dollars.
    pub value: f32,
    pub capacity: u32,
}

/// One club's money.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Club {
    /// Closed league years, oldest first, and their numbers.
    pub years: Vec<Book<N>>,
    pub seasons: Vec<Season>,
    /// The year under way: budget and actual so far.
    pub budget: Book<N>,
    pub now: Book<N>,
    pub months: [f32; 12],
    /// This season's home gates.
    pub gates: Vec<Gate>,
    pub home: Home,
    /// A city it moved to after 2026 (city, state), with its name there.
    pub city: Option<(String, String)>,
    /// Approved to move (city, state, season the new stadium opens).
    pub moving: Option<(String, String, i32)>,
    /// Public votes for a new stadium lost in a row.
    pub votes_lost: u8,
    pub owner_name: String,
    pub owner_since: i32,
    /// The average ticket this season, dollars.
    pub price: f32,
    pub season_tickets: u32,
    pub waitlist: u32,
    pub renewal: f32,
    /// The training building, 0-1, and the season it opened.
    pub facility: f32,
    pub facility_year: i32,
    /// What the owner spends on the players' food, travel and families,
    /// 0-1, and how much he cares about the families (P1).
    pub amenities: f32,
    pub family: f32,
    /// The value last August.
    pub value: f32,
}

impl Club {
    pub fn cash_month(&mut self, month: u32, v: f32) {
        self.months[(month as usize + 11) % 12] += v;
    }
    pub fn revenue(b: &Book<N>) -> f64 {
        b.sum(0..INCOME)
    }
    pub fn costs(b: &Book<N>) -> f64 {
        b.sum(INCOME..N)
    }
}

/// A club's report card from the NFLPA's player survey (P1): the release
/// year, the club, a grade index into `data::GRADES` for each part, and
/// its rank by the average.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Card {
    pub year: i32,
    pub team: u8,
    pub grades: [u8; 11],
    pub rank: u8,
}

/// The metro area's people in millions (Census estimates, about 2023).
pub fn metro(city: &str) -> f64 {
    match city {
        "East Rutherford" | "New York" => 19.5,
        "Los Angeles" | "Inglewood" | "Anaheim" | "Carson" => 12.8,
        "Chicago" => 9.3,
        "Arlington" | "Irving" | "Dallas" => 8.1,
        "Houston" => 7.5,
        "Atlanta" => 6.3,
        "Landover" | "Washington" => 6.3,
        "Philadelphia" => 6.2,
        "Miami Gardens" | "Miami" => 6.1,
        "Glendale" | "Tempe" | "Phoenix" => 5.0,
        "Foxborough" | "Boston" => 4.9,
        "San Francisco" | "Santa Clara" | "Oakland" => 4.7,
        "Detroit" | "Pontiac" => 4.3,
        "Seattle" => 4.0,
        "Minneapolis" => 3.7,
        "San Diego" => 3.3,
        "Tampa" => 3.3,
        "Denver" => 3.0,
        "Baltimore" => 2.8,
        "Charlotte" | "Clemson" => 2.8,
        "St. Louis" => 2.8,
        "Orlando" => 2.8,
        "San Antonio" => 2.6,
        "Pittsburgh" => 2.4,
        "Austin" => 2.4,
        "Portland" => 2.5,
        "Paradise" | "Las Vegas" => 2.3,
        "Cincinnati" => 2.3,
        "Kansas City" => 2.2,
        "Cleveland" => 2.1,
        "Indianapolis" => 2.1,
        "Nashville" => 2.1,
        "Memphis" => 1.3,
        "Salt Lake City" => 1.3,
        "Jacksonville" => 1.7,
        "New Orleans" => 1.3,
        "Orchard Park" | "Buffalo" => 1.2,
        // Green Bay's own metro is small; the Packers draw on the state.
        "Green Bay" => 1.6,
        _ => 2.0,
    }
}

/// Where a club plays this season: its relocated city after 2026, else its era's.
pub fn city(w: &World, t: usize, year: i32) -> (String, String) {
    if let Some(c) = w.finance.nfl.get(t).and_then(|c| c.city.clone()) {
        return c;
    }
    franchise::era(t, year.min(franchise::PRESENT))
        .map_or(("".into(), "".into()), |e| (e.city.to_string(), e.state.to_string()))
}

/// Whether a club's stadium has a roof (the real one to 2026, then the
/// stadium it plays in).
pub fn roof(w: &World, t: usize, year: i32) -> Roof {
    match w.finance.nfl.get(t) {
        Some(c) if year > franchise::PRESENT && !c.home.name.is_empty() => {
            if c.home.dome {
                Roof::Dome
            } else {
                Roof::Open
            }
        }
        _ => franchise::era(t, year).map_or(Roof::Open, |e| e.roof),
    }
}

fn home_from(st: &data::Stint, roof: Roof) -> Home {
    Home {
        name: st.name.to_string(),
        capacity: st.capacity,
        opened: st.opened,
        renovated: st.opened,
        public: st.public as f32,
        cost: (st.cost * 1.0e6) as f32,
        dome: roof != Roof::Open,
    }
}

/// Opens every club's books: owners, stadiums, prices and waiting lists.
pub(crate) fn open(w: &mut World) {
    let year = w.year;
    let n = w.nfl.len().min(32);
    let mut clubs = Vec::with_capacity(n);
    for t in 0..n {
        let mut c = Club::default();
        let st = data::stint(t, year.min(franchise::PRESENT)).or_else(|| data::stint(t, franchise::PRESENT));
        if let Some(st) = st {
            c.home = home_from(st, franchise::era(t, year.min(franchise::PRESENT)).map_or(Roof::Open, |e| e.roof));
        }
        let mut rng = crate::rng::Rng::new(w.options.seed ^ 0x0B00_C500 ^ t as u32);
        let age = rng.int(48, 76);
        c.owner_name =
            crate::world::names::person_name(crate::world::ethnicity::Ethnicity::White, year - age, &mut rng);
        c.owner_since = year - rng.int(0, 25);
        let owner = &w.clubs.get(t).map(|cl| cl.office.owner.clone()).unwrap_or_default();
        c.amenities = clamp(0.35 + 0.4 * owner.cash_will + rng.normal(0.0, 0.15), 0.05, 1.0) as f32;
        c.family = clamp(0.5 + rng.normal(0.0, 0.22), 0.0, 1.0) as f32;
        c.facility_year = year - rng.int(0, 22);
        c.facility = clamp(1.0 - 0.03 * (year - c.facility_year) as f64, 0.2, 1.0) as f32;
        let brand = data::BRAND[t];
        c.price = (data::nfl_ticket(year) * (0.8 + 0.4 * brand)) as f32;
        c.season_tickets = (c.home.capacity as f64 * (0.62 + 0.25 * brand)) as u32;
        // The Packers' list runs past 100,000 names (about 140,000 in 2024).
        c.waitlist = if t == 11 {
            (60_000.0 + 2_000.0 * (year - 1984).max(0) as f64) as u32
        } else {
            (brand * brand * 20_000.0 * rng.range(0.3, 1.2)) as u32
        };
        c.renewal = 0.92;
        clubs.push(c);
    }
    w.finance.nfl = clubs;
    // An older world file: the past years' public numbers are estimated.
    for t in 0..n {
        open_year(w, t, year);
        if franchise::active(t, year) {
            let v = value_of(w, t, year);
            w.finance.nfl[t].value = v as f32;
        }
    }
    for y in (year - 10).max(w.first_year())..year {
        let total = data::league_revenue(y) as f32;
        w.finance.league.push((y, total));
    }
}

/// The local money factor: market, stadium, brand and fans.
fn local_factor(w: &World, t: usize, year: i32) -> f64 {
    let (city, _) = city(w, t, year);
    let m = (metro(&city) / 4.0).powf(0.22);
    let c = &w.finance.nfl[t];
    let stad = 1.0 + 0.9 * c.home.premium(year);
    let brand = 0.72 + 0.56 * data::BRAND[t];
    let fans = w.clubs.get(t).map_or(0.0, |cl| cl.office.fans);
    let lame = if c.moving.is_some() { 0.85 } else { 1.0 };
    m * stad * brand * (1.0 + 0.12 * fans) * lame / 1.32
}

/// The owner's will to spend on football, 0-1: his traits and his money.
pub fn spend_level(w: &World, t: usize) -> f64 {
    let year = w.year;
    let owner = w.nfl.get(t).map_or(10.0, |x| x.owner);
    let f = if w.finance.nfl.len() > t { local_factor(w, t, year) } else { 1.0 };
    clamp(0.55 + 0.03 * (owner - 10.0) + 0.25 * (f - 1.0), 0.3, 1.0)
}

/// The year's budget, set when the league year opens.
pub(crate) fn open_year(w: &mut World, t: usize, year: i32) {
    if !franchise::active(t, year) {
        w.finance.nfl[t].budget = Book::new(year);
        w.finance.nfl[t].now = Book::new(year);
        return;
    }
    let teams = franchise::active_teams(year).len().max(1) as f64;
    let total = data::league_revenue(year);
    let national = total * data::national_share(year) / teams;
    let local = total * (1.0 - data::national_share(year)) / teams * local_factor(w, t, year);
    let pay = data::nfl_pay_index(year);
    let spend = spend_level(w, t);
    let c = &w.finance.nfl[t];
    let naming = c.home.opened >= 1997 || year >= 2003;
    let psl = c.home.age(year) < 22 && c.home.opened >= 1994;
    let events = c.home.public < 0.6;
    let mut b = Book::<N>::new(year);
    b.l[NATIONAL] = national as f32;
    let shares = [
        (TICKETS, 0.33),
        (PREMIUM, 0.20),
        (CONCESSIONS, 0.09),
        (SPONSORS, 0.21),
        (NAMING, if naming { 0.05 } else { 0.0 }),
        (LOCAL_MEDIA, 0.05),
        (PSL, if psl { 0.04 } else { 0.0 }),
        (EVENTS, if events { 0.03 } else { 0.0 }),
    ];
    let sum: f64 = shares.iter().map(|x| x.1).sum();
    for (k, s) in shares {
        b.l[k] = (local * s / sum) as f32;
    }
    let cap = crate::world::pro::money::price_cap(year);
    b.l[PLAYERS] = (cap * 0.96) as f32;
    b.l[BENEFITS] = (cap * 0.2) as f32;
    let coach_pay: f64 = [w.nfl[t].head_coach, w.nfl[t].oc, w.nfl[t].dc]
        .into_iter()
        .flatten()
        .filter_map(|c| w.coaches.get(c).and_then(|c| c.contract))
        .map(|k| k.salary)
        .sum();
    b.l[COACHES] = (coach_pay + 22.0e6 * pay * (0.8 + 0.4 * spend)) as f32;
    b.l[SCOUTING] = (9.0e6 * pay * (0.6 + 0.8 * spend)) as f32;
    b.l[MEDICAL] = (10.0e6 * pay * (0.6 + 0.8 * spend)) as f32;
    let (city, _) = city(w, t, year);
    let west = matches!(
        city.as_str(),
        "Seattle"
            | "San Francisco"
            | "Santa Clara"
            | "Oakland"
            | "Inglewood"
            | "Los Angeles"
            | "Carson"
            | "San Diego"
            | "Paradise"
            | "Glendale"
            | "Tempe"
            | "Anaheim"
    );
    b.l[TRAVEL] = (8.0e6 * pay * if west { 1.35 } else { 1.0 }) as f32;
    b.l[AMENITIES] = (11.0e6 * pay * (0.25 + 0.75 * c.amenities as f64)) as f32;
    b.l[STADIUM_OPS] = (34.0e6 * pay * if c.home.public > 0.6 { 0.8 } else { 1.25 }) as f32;
    b.l[GAME_DAY] = (1.0e6 * pay * 10.0) as f32;
    b.l[BUSINESS] = (local * 0.24) as f32;
    let private = (c.home.cost * (1.0 - c.home.public)) as f64;
    b.l[DEBT] = if c.home.age(year) < 30 { (private * 0.065) as f32 } else { 0.0 };
    let c = &mut w.finance.nfl[t];
    // The business side sets the average ticket by demand: the club's
    // local pull against the league's average ticket, nudged by how the
    // last season sold.
    let sold = c.seasons.last().map_or(0.9, |s| s.sellouts as f64 / s.home_games.max(1) as f64);
    let list = (c.waitlist as f64 / c.home.capacity.max(1) as f64).min(1.0);
    c.price = (data::nfl_ticket(year)
        * (local / (total * (1.0 - data::national_share(year)) / teams)).powf(0.6)
        * (0.92 + 0.1 * sold + 0.06 * list)) as f32;
    c.budget = b.clone();
    c.now = Book::new(year);
    c.now.l[DEBT] = 0.0;
    c.months = [0.0; 12];
    c.gates.clear();
}

/// The month's money: every evenly paid line at its share.
pub(crate) fn month(w: &mut World, m: u32) {
    for c in w.finance.nfl.iter_mut() {
        let mut net = 0.0f32;
        for (k, line) in LINES.iter().enumerate() {
            let v = (c.budget.l[k] as f64 * super::month_share(line.spread, m)) as f32;
            if v == 0.0 {
                continue;
            }
            c.now.l[k] += v;
            net += if line.income { v } else { -v };
        }
        c.cash_month(m, net);
    }
}

/// The day after the Super Bowl: last year's books close and this year's open.
pub(crate) fn year_start(w: &mut World) {
    let year = w.year;
    let last = year - 1;
    let n = w.finance.nfl.len();
    // The visiting gate pool: every club's share of its gate goes to a
    // pool split equally (since 2001; before, to the visitors, which
    // evens out the same way over a season).
    let gates: Vec<f64> = (0..n).map(|t| w.finance.nfl[t].now.l[TICKETS] as f64).collect();
    let active: Vec<usize> =
        (0..n).filter(|&t| w.finance.nfl[t].now.year == last && franchise::active(t, last)).collect();
    let pool: f64 = active.iter().map(|&t| gates[t] * data::VISITOR_SHARE).sum::<f64>() / active.len().max(1) as f64;
    let mut league = 0.0;
    for &t in &active {
        let cash = w.clubs.get(t).and_then(|c| c.cash.iter().find(|x| x.0 == last).map(|x| x.1));
        let c = &mut w.finance.nfl[t];
        c.now.l[POOL] = (pool - gates[t] * data::VISITOR_SHARE) as f32;
        if let Some(cash) = cash {
            c.now.l[PLAYERS] = cash as f32;
        }
        // Lines posted only through the year are trued up to the year's budget.
        for k in [
            NATIONAL,
            SPONSORS,
            NAMING,
            LOCAL_MEDIA,
            PSL,
            EVENTS,
            BENEFITS,
            COACHES,
            SCOUTING,
            MEDICAL,
            TRAVEL,
            AMENITIES,
            STADIUM_OPS,
            BUSINESS,
            DEBT,
        ] {
            c.now.l[k] = c.now.l[k].max(c.budget.l[k] * 0.98);
        }
        let s = season_numbers(c, last);
        c.seasons.push(s);
        let book = std::mem::take(&mut c.now);
        league += Club::revenue(&book);
        c.years.push(book);
    }
    if !active.is_empty() {
        w.finance.league.push((last, league as f32));
    }
    // Stadiums by their real history to 2026, the world's own after.
    for t in 0..n {
        if year <= franchise::PRESENT
            && let Some(st) = data::stint(t, year)
        {
            let roof = franchise::era(t, year).map_or(Roof::Open, |e| e.roof);
            if w.finance.nfl[t].home.name != st.name || w.finance.nfl[t].home.opened != st.opened {
                w.finance.nfl[t].home = home_from(st, roof);
            }
        }
        open_projects(w, t, year);
        open_year(w, t, year);
    }
    if year == franchise::PRESENT {
        for (t, name, seats, opens, cost, public, renovation) in data::PROJECTS_2026 {
            if t >= n {
                continue;
            }
            w.finance.projects.push(Project {
                owner: Owner::Nfl(t),
                kind: if renovation { ProjectKind::Renovation } else { ProjectKind::Stadium },
                name: name.to_string(),
                cost: (cost * 1.0e6) as f32,
                public: (public * 1.0e6) as f32,
                pledged: 0.0,
                started: 2024,
                opens,
                capacity: seats,
                step: 0.0,
                done: false,
                asked: false,
            });
        }
    }
}

fn season_numbers(c: &Club, year: i32) -> Season {
    let reg: Vec<&Gate> = c.gates.iter().filter(|g| !g.preseason).collect();
    Season {
        year,
        home_games: reg.len() as u8,
        attendance: reg.iter().map(|g| g.attendance).sum(),
        sellouts: reg.iter().filter(|g| g.sold_out).count() as u8,
        blackouts: reg.iter().filter(|g| g.blackout).count() as u8,
        price: c.price,
        renewal: c.renewal,
        waitlist: c.waitlist,
        value: c.value,
        capacity: c.home.capacity,
    }
}

/// A finished project opens with the season.
fn open_projects(w: &mut World, t: usize, year: i32) {
    let mut opened = Vec::new();
    for (k, p) in w.finance.projects.iter().enumerate() {
        if p.owner == Owner::Nfl(t) && !p.done && p.opens <= year {
            opened.push(k);
        }
    }
    for k in opened {
        let p = w.finance.projects[k].clone();
        w.finance.projects[k].done = true;
        let moving = w.finance.nfl[t].moving.clone();
        let c = &mut w.finance.nfl[t];
        match p.kind {
            ProjectKind::Stadium => {
                c.home = Home {
                    name: p.name.clone(),
                    capacity: p.capacity,
                    opened: year,
                    renovated: year,
                    public: if p.cost > 0.0 { p.public / p.cost } else { 0.0 },
                    cost: p.cost,
                    dome: true,
                };
                c.votes_lost = 0;
            }
            ProjectKind::Renovation => {
                c.home.renovated = year;
                c.home.capacity = p.capacity.max(1);
                c.home.cost += p.cost;
            }
            ProjectKind::Facility => {
                c.facility = 1.0;
                c.facility_year = year;
            }
        }
        // A club moving opens its new home in its new city.
        if let Some((city, state, when)) = moving
            && when <= year
            && p.kind == ProjectKind::Stadium
        {
            relocate(w, t, &city, &state);
        }
        let name = w.nfl[t].name;
        let text = match p.kind {
            ProjectKind::Facility => format!("The {name} open their new training facility."),
            ProjectKind::Renovation => format!("The {name} open the rebuilt {}.", p.name),
            ProjectKind::Stadium => format!("The {name} open {}: {} seats.", p.name, thousands(p.capacity)),
        };
        let e = story(w, Level::Nfl, Loudness::Big, text).team(t);
        super::post(w, e);
    }
}

/// The club takes its new city's name (after 2026: the old name's
/// nickname stays, as with the Rams, Chargers and Raiders).
fn relocate(w: &mut World, t: usize, city: &str, state: &str) {
    let old = w.nfl[t].name.to_string();
    let nick = old.rsplit(' ').next().unwrap_or(&old).to_string();
    let name = format!("{city} {nick}");
    w.nfl[t].name = Box::leak(name.clone().into_boxed_str());
    let c = &mut w.finance.nfl[t];
    c.city = Some((city.to_string(), state.to_string()));
    c.moving = None;
    let text = format!("The {old} are now the {name}.");
    let e = story(w, Level::Nfl, Loudness::Huge, text).team(t).memorable();
    super::post(w, e);
}

fn thousands(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// What the stadium draws for a game: demand from the fans' mood, the
/// club's pull, the opponent, the record, the weather, the kickoff and the
/// price; a game sells out when demand passes the seats.
pub(crate) fn demand(w: &World, t: usize, opp: usize, d: Date, preseason: bool, playoff: bool) -> f64 {
    let year = w.year;
    let c = &w.finance.nfl[t];
    let fans = w.clubs.get(t).map_or(0.0, |cl| cl.office.fans);
    let brand = data::BRAND[t];
    let (city, _) = city(w, t, year);
    let market = (metro(&city) / 4.0).ln().clamp(-1.2, 1.6);
    let opp_brand = data::BRAND.get(opp).copied().unwrap_or(0.5);
    let (wins, losses, _) = w.nfl_record(t);
    let wp = if wins + losses > 0 { wins as f64 / (wins + losses) as f64 - 0.5 } else { 0.0 };
    // Older eras sold fewer seats: more blackouts in the 1980s and 1990s
    // (a third of games in the mid-1980s, about one in ten in the 2000s,
    // 16 in 2011 and none in 2014), and from the 2000s most games sell out.
    let era = clamp((year - 1984) as f64 / 30.0, 0.0, 1.0);
    let price_rel = c.price as f64 / data::nfl_ticket(year).max(1.0);
    let late = d.month() == 12 || d.month() == 1;
    let cold = late
        && !c.home.dome
        && matches!(
            city.as_str(),
            "Orchard Park"
                | "Green Bay"
                | "Foxborough"
                | "Cleveland"
                | "Pittsburgh"
                | "Chicago"
                | "Kansas City"
                | "Denver"
                | "East Rutherford"
                | "Philadelphia"
                | "Cincinnati"
                | "Baltimore"
                | "Landover"
                | "Washington"
        );
    let mut x = 0.915 + 0.1 * era + 0.06 * brand + 0.07 * fans + 0.025 * market + 0.03 * opp_brand + 0.16 * wp
        - 0.05 * (price_rel - 1.0)
        - if cold { 0.02 } else { 0.0 }
        - if c.moving.is_some() { 0.08 } else { 0.0 };
    if preseason {
        x -= 0.07;
    }
    if playoff {
        x += 0.25;
    }
    x + 0.035 * draw_normal(w, 0x6A7E, (t as u64) << 16 ^ d.0 as u64, year)
}

/// A home game is played: its gate, the blackout check and the money.
pub(crate) fn gate(w: &mut World, t: usize, opp: usize, d: Date, preseason: bool, playoff: bool) -> Gate {
    let year = w.year;
    let x = demand(w, t, opp, d, preseason, playoff);
    let c = &w.finance.nfl[t];
    let cap = c.home.capacity.max(20_000) as f64;
    // The blackout rule (1973-2014): a game not sold out 72 hours before
    // kickoff is off local TV. From 2012 a club could set its bar at 85%.
    // A club a few thousand seats short bought them up itself, or a local
    // sponsor did, to keep the game on TV.
    let rule = !preseason && (1973..=2014).contains(&year);
    let bar = if year >= 2012 { 0.85 } else { 1.0 };
    let bought = rule && x < bar && x >= bar - 0.05;
    let sold_out = x >= 1.0 || bought && bar >= 1.0;
    let att = if x >= 1.0 {
        cap * (1.0 + 0.015 * (x - 1.0).min(1.0))
    } else if bought {
        cap * x.max(bar)
    } else {
        cap * x.max(0.35)
    };
    let blackout = rule && x < bar - 0.05;
    let g = Gate { date: d, opp: opp as u8, attendance: att as u32, sold_out, blackout, preseason, playoff };
    let price = c.price as f64
        * if preseason {
            0.75
        } else if playoff {
            1.6
        } else {
            1.0
        };
    let pay = data::nfl_pay_index(year);
    let tickets = att * price;
    let budget_games = 10.0;
    let prem = c.budget.l[PREMIUM] as f64 / budget_games * (att / cap);
    let conc = c.budget.l[CONCESSIONS] as f64 / budget_games * (att / cap);
    let day = 1.0e6 * pay;
    let c = &mut w.finance.nfl[t];
    c.gates.push(g);
    c.now.l[TICKETS] += tickets as f32;
    c.now.l[PREMIUM] += prem as f32;
    c.now.l[CONCESSIONS] += conc as f32;
    c.now.l[GAME_DAY] += day as f32;
    c.cash_month(d.month(), (tickets + prem + conc - day) as f32);
    if blackout {
        // Off local TV: the fans notice, the local partners too.
        if let Some(cl) = w.clubs.get_mut(t) {
            cl.office.fans = clamp(cl.office.fans - 0.015, -1.0, 1.0);
        }
        let unsold = (cap - att).max(0.0) as u32;
        let name = w.nfl[t].name;
        let opp_name = w.nfl.get(opp).map_or("", |x| x.name);
        let text = format!(
            "Blacked out: the {name}' game against the {opp_name} is off local TV after {} tickets went unsold at the 72-hour deadline.",
            thousands(unsold / 100 * 100)
        );
        let e = story(w, Level::Nfl, Loudness::Small, text).team(t).rule("NFL: blackout rule (1973-2014)");
        super::post(w, e);
    }
    g
}

/// Every dated NFL money step: renewal days (A17), the NFLPA report cards
/// (P1), the owners' meetings (relocation votes, sales), the Packers'
/// public report, Forbes values and the November stadium votes.
pub(crate) fn dated(w: &mut World, d: Date) {
    let year = d.year();
    let renewals: Vec<usize> = w
        .nfl_season
        .as_ref()
        .map(|s| s.ticket_days.iter().filter(|x| x.1 == d).map(|x| x.0).collect())
        .unwrap_or_default();
    for t in renewals {
        renew(w, t);
    }
    if d == Date::ymd(year, 3, 1) && year >= data::CARDS_FROM {
        report_cards(w, year);
    }
    if d == Date::nth_weekday(year, 3, Weekday::Mon, 4) {
        owners_meeting(w);
    }
    if d == Date::nth_weekday(year, 5, Weekday::Tue, 3) {
        sales(w);
    }
    if d == Date::nth_weekday(year, 7, Weekday::Mon, 4) && year >= 1990 {
        packers_report(w);
    }
    if d == Date::ymd(year, 8, 20) && year >= 1991 {
        values(w);
    }
    // Election day: the first Tuesday after the first Monday of November.
    if d == Date::nth_weekday(year, 11, Weekday::Mon, 1).add(1) && year > franchise::PRESENT {
        stadium_votes(w);
    }
}

/// Season ticket renewals (A17): the rate from the fans' mood and the
/// price change; the waiting list fills what is given up, and grows when
/// demand runs past the seats.
fn renew(w: &mut World, t: usize) {
    let year = w.year;
    if t >= w.finance.nfl.len() || !franchise::active(t, year) {
        return;
    }
    let fans = w.clubs.get(t).map_or(0.0, |cl| cl.office.fans);
    let brand = data::BRAND[t];
    let x = demand(w, t, t, w.today, false, false);
    let noise = 0.015 * draw_normal(w, 0x2E4E, t as u64, year);
    let c = &mut w.finance.nfl[t];
    let lame = if c.moving.is_some() { 0.12 } else { 0.0 };
    let rate = clamp(0.885 + 0.07 * fans + 0.04 * (brand - 0.5) + noise - lame, 0.6, 0.99);
    c.renewal = rate as f32;
    let cap = c.home.capacity as f64;
    let target = cap * (0.62 + 0.25 * brand).min(0.9);
    let kept = c.season_tickets as f64 * rate;
    let gap = (target - kept).max(0.0);
    let from_list = gap.min(c.waitlist as f64);
    c.waitlist = (c.waitlist as f64 - from_list + if x > 1.0 { cap * (x - 1.0) * 0.6 } else { 0.0 }).max(0.0) as u32;
    if x < 0.97 {
        c.waitlist = (c.waitlist as f64 * 0.7) as u32;
    }
    c.season_tickets = (kept + from_list + (gap - from_list) * clamp(x - 0.6, 0.0, 1.0)).min(cap * 0.9) as u32;
    let st = c.season_tickets;
    let list = c.waitlist;
    if w.seat_club(t) {
        let name = w.nfl[t].name;
        let text = if list > 0 {
            format!(
                "Season ticket renewals: {:.0}% of the {name}' holders renewed; {} season tickets sold, {} names on the waiting list.",
                rate * 100.0,
                thousands(st),
                thousands(list)
            )
        } else {
            format!(
                "Season ticket renewals: {:.0}% of the {name}' holders renewed; {} season tickets sold and no waiting list.",
                rate * 100.0,
                thousands(st)
            )
        };
        let e = story(w, Level::Nfl, Loudness::Medium, text).team(t).rule("team: tickets (A17)");
        super::post(w, e);
    }
}

/// Forbes-style value: revenue times the era's multiple, more for a big
/// brand and a new stadium.
pub fn value_of(w: &World, t: usize, year: i32) -> f64 {
    let c = &w.finance.nfl[t];
    let rev = c.years.last().map_or_else(|| Club::revenue(&c.budget), Club::revenue);
    let rev = if rev <= 0.0 { data::league_revenue(year) / 32.0 } else { rev };
    rev * data::value_multiple(year) * (0.84 + 0.32 * data::BRAND[t]) * (1.0 + 0.2 * c.home.premium(year))
}

fn values(w: &mut World) {
    let year = w.year;
    let teams = franchise::active_teams(year);
    let mut list: Vec<(f64, usize)> = Vec::new();
    for &t in &teams {
        if t >= w.finance.nfl.len() {
            continue;
        }
        let v = value_of(w, t, year);
        w.finance.nfl[t].value = v as f32;
        if let Some(s) = w.finance.nfl[t].seasons.last_mut()
            && s.year == year - 1
        {
            s.value = v as f32;
        }
        list.push((v, t));
    }
    list.sort_by(|a, b| b.0.total_cmp(&a.0));
    let Some(&(top, best)) = list.first() else { return };
    let avg = list.iter().map(|x| x.0).sum::<f64>() / list.len() as f64;
    let who = if year >= 1998 { "Forbes" } else { "Financial World" };
    let text = format!(
        "{who}'s club values: the {} lead at {}; the average club is worth {}.",
        w.nfl[best].name,
        dollars(top),
        dollars(avg)
    );
    let e = story(w, Level::Nfl, Loudness::Small, text).team(best);
    super::post(w, e);
    if let Some(s) = w.seat.and_then(|s| s.nfl_team)
        && w.seat_club(s)
        && s != best
        && let Some(rank) = list.iter().position(|x| x.1 == s)
    {
        let text = format!(
            "{who} values the {} at {}, {} in the league.",
            w.nfl[s].name,
            dollars(list[rank].0),
            ordinal(rank + 1)
        );
        let e = story(w, Level::Nfl, Loudness::Medium, text).team(s);
        super::post(w, e);
    }
}

pub fn ordinal(n: usize) -> String {
    let s = match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

/// The Packers publish their books each July at the shareholders' meeting:
/// the one club anyone can read.
fn packers_report(w: &mut World) {
    let t = 11;
    let Some(c) = w.finance.nfl.get(t) else { return };
    let Some(b) = c.years.last() else { return };
    let rev = Club::revenue(b);
    let profit = rev - Club::costs(b);
    let text = format!(
        "The Packers' annual report: revenue {}, national revenue {}, operating profit {}.",
        dollars(rev),
        dollars(b.l[NATIONAL] as f64),
        dollars(profit)
    );
    let e = story(w, Level::Nfl, Loudness::Small, text).team(t);
    super::post(w, e);
}

/// Club sales and handovers at the spring meeting: an owner sells (about
/// one club in forty a year, by the 2000-2025 record) or passes the club
/// to his family. The price comes from the value.
fn sales(w: &mut World) {
    let year = w.year;
    for t in franchise::active_teams(year) {
        if t >= w.finance.nfl.len() || w.finance.nfl[t].moving.is_some() {
            continue;
        }
        let tenure = year - w.finance.nfl[t].owner_since;
        let r = draw(w, 0x5A1E, t as u64, year);
        let sale = r < 0.022 && tenure >= 4;
        let heir = !sale && r < 0.034 && tenure >= 15;
        if !sale && !heir {
            continue;
        }
        let mut rng = crate::rng::Rng::new(w.options.seed ^ 0x5A1E_0000 ^ (year as u32) << 6 ^ t as u32);
        let old = w.finance.nfl[t].owner_name.clone();
        let surname = old.rsplit(' ').next().unwrap_or("").to_string();
        let name = if heir {
            let first =
                crate::world::names::person_name(crate::world::ethnicity::Ethnicity::White, year - 45, &mut rng);
            format!("{} {surname}", first.split(' ').next().unwrap_or("Pat"))
        } else {
            let eth = if rng.chance(0.12) {
                crate::world::ethnicity::Ethnicity::Asian
            } else {
                crate::world::ethnicity::Ethnicity::White
            };
            crate::world::names::person_name(eth, year - rng.int(45, 70), &mut rng)
        };
        let price = w.finance.nfl[t].value as f64 * rng.range(1.02, 1.3);
        let c = &mut w.finance.nfl[t];
        c.owner_name = name.clone();
        c.owner_since = year;
        c.amenities = clamp(c.amenities as f64 + rng.normal(0.08, 0.15), 0.05, 1.0) as f32;
        c.family = clamp(rng.normal(0.55, 0.2), 0.0, 1.0) as f32;
        if let Some(cl) = w.clubs.get_mut(t) {
            let mut r2 = rng.fork();
            cl.office.owner = crate::world::pro::office::new_owner(&mut r2);
        }
        let team = w.nfl[t].name;
        let text = if heir {
            format!("{old} hands control of the {team} to {name}.")
        } else {
            format!(
                "The owners approve the sale of the {team}: {name} buys the club from {old} for {}.",
                dollars(price)
            )
        };
        let e = story(w, Level::Nfl, if heir { Loudness::Medium } else { Loudness::Big }, text).team(t);
        super::post(w, e);
    }
}

/// The November votes on public money for a stadium (after 2026): a club
/// whose stadium is past its prime asks its city and state; a yes starts
/// a new stadium or a rebuild, a string of no's opens the road to moving.
fn stadium_votes(w: &mut World) {
    let year = w.year;
    for t in franchise::active_teams(year) {
        if t >= w.finance.nfl.len() {
            continue;
        }
        let busy =
            w.finance.projects.iter().any(|p| p.owner == Owner::Nfl(t) && !p.done && p.kind != ProjectKind::Facility);
        let c = &w.finance.nfl[t];
        if busy || c.moving.is_some() {
            continue;
        }
        let age = c.home.age(year);
        let wants = age >= 24 + (draw(w, 0x57AD, t as u64, 0) * 8.0) as i32;
        if !wants {
            continue;
        }
        let fans = w.clubs.get(t).map_or(0.0, |cl| cl.office.fans);
        let (city, state) = city(w, t, year);
        let small = metro(&city) < 3.0;
        // Rebuild what stands when the bones are good and the owner is
        // careful with money; otherwise ask for a new building.
        let owner = w.clubs.get(t).map(|cl| cl.office.owner.clone()).unwrap_or_default();
        let renovate = c.home.opened >= year - 35 && owner.cash_will < 0.5;
        let grow = data::league_revenue(year) / data::league_revenue(2024);
        let cost = if renovate { 900.0e6 } else { 2_600.0e6 } * grow * (0.8 + 0.4 * draw(w, 0x57AC, t as u64, year));
        let public = clamp(0.35 + 0.2 * draw(w, 0x57AB, t as u64, year) + if small { 0.1 } else { 0.0 }, 0.2, 0.65);
        let yes = clamp(
            0.42 + 0.25 * fans + if small { 0.1 } else { -0.05 } - 0.3 * (public - 0.4) + 0.08 * c.votes_lost as f64,
            0.1,
            0.9,
        );
        let name = w.nfl[t].name;
        if draw(w, 0x57A1, t as u64, year) < yes {
            let opens = year + if renovate { 2 } else { 3 };
            let stadium_name = if renovate { c.home.name.clone() } else { format!("New {} Stadium", city) };
            let capacity = if renovate {
                c.home.capacity.min(70_000)
            } else {
                62_000 + (draw(w, 0x57AE, t as u64, year) * 9_000.0) as u32
            };
            w.finance.projects.push(Project {
                owner: Owner::Nfl(t),
                kind: if renovate { ProjectKind::Renovation } else { ProjectKind::Stadium },
                name: stadium_name,
                cost: cost as f32,
                public: (cost * public) as f32,
                pledged: 0.0,
                started: year,
                opens,
                capacity,
                step: 0.0,
                done: false,
                asked: false,
            });
            let what = if renovate { "a rebuild of their stadium" } else { "a new stadium" };
            let text = format!(
                "Voters in {city}, {state} approve {} in public money for {what} for the {name}, opening in {opens} ({} in all).",
                dollars(cost * public),
                dollars(cost)
            );
            let e = story(w, Level::Nfl, Loudness::Big, text).team(t);
            super::post(w, e);
        } else {
            w.finance.nfl[t].votes_lost += 1;
            let text = format!("Voters in {city}, {state} turn down public money for a new {name} stadium.");
            let e = story(w, Level::Nfl, Loudness::Big, text).team(t);
            super::post(w, e);
        }
    }
}

/// The annual meeting in late March: a club whose city keeps saying no
/// asks to move to a city without a team; 24 of 32 owners must agree and
/// the club pays the relocation fee. It plays out its lease and moves when
/// the new stadium opens (the Raiders, 2017-2020).
fn owners_meeting(w: &mut World) {
    let year = w.year;
    if year <= franchise::PRESENT {
        return;
    }
    for t in franchise::active_teams(year) {
        if t >= w.finance.nfl.len() {
            continue;
        }
        let c = &w.finance.nfl[t];
        if c.votes_lost < 2 || c.moving.is_some() {
            continue;
        }
        let (here, _) = city(w, t, year);
        let taken: Vec<String> = (0..w.finance.nfl.len()).map(|o| city(w, o, year).0).collect();
        let mut best: Option<(f64, &str, &str, f64)> = None;
        for &(c2, st, rank, public) in data::OPEN_CITIES.iter() {
            if taken.iter().any(|x| x == c2)
                || w.finance.nfl.iter().any(|o| o.moving.as_ref().is_some_and(|m| m.0 == c2))
            {
                continue;
            }
            let gain = metro(c2) / metro(&here).max(0.5) * (0.8 + public) - 0.002 * rank as f64;
            if best.is_none_or(|b| gain > b.0) {
                best = Some((gain, c2, st, public));
            }
        }
        let Some((gain, to, st, public)) = best else { continue };
        let owner = w.clubs.get(t).map(|cl| cl.office.owner.clone()).unwrap_or_default();
        let wants = gain > 0.9 && draw(w, 0x40E1, t as u64, year) < 0.25 + 0.3 * owner.pr + 0.1 * c.votes_lost as f64;
        if !wants {
            continue;
        }
        let name = w.nfl[t].name;
        let approved = draw(w, 0x40E2, t as u64, year) < 0.8;
        if !approved {
            let text = format!(
                "The owners turn down the {name}' request to move to {to}: they fall short of the 24 votes needed."
            );
            let e = story(w, Level::Nfl, Loudness::Huge, text).team(t);
            super::post(w, e);
            continue;
        }
        let fee = data::relocation_fee(year, if metro(to) > 6.0 { 3 } else { 20 });
        let grow = data::league_revenue(year) / data::league_revenue(2024);
        let cost = 2_400.0e6 * grow;
        let opens = year + 3;
        w.finance.projects.push(Project {
            owner: Owner::Nfl(t),
            kind: ProjectKind::Stadium,
            name: format!("{to} Stadium"),
            cost: cost as f32,
            public: (cost * public) as f32,
            pledged: 0.0,
            started: year,
            opens,
            capacity: 65_000,
            step: 0.0,
            done: false,
            asked: false,
        });
        let c = &mut w.finance.nfl[t];
        c.moving = Some((to.to_string(), st.to_string(), opens));
        c.budget.l[DEBT] += (fee / 10.0) as f32;
        let text = format!(
            "The owners vote to let the {name} move to {to}, {st}, when the new stadium opens in {opens}. The relocation fee is {}.",
            dollars(fee)
        );
        let e = story(w, Level::Nfl, Loudness::Huge, text).team(t).memorable();
        super::post(w, e);
    }
}

/// The NFLPA's team report cards (P1): players grade their club on each
/// part of the survey from what they live with each day. Public in 2023
/// and 2024; from 2025 an arbitrator's ruling keeps them inside the union,
/// so only the club's own players and the agents know them. A bad card
/// pushes a proud owner to spend.
fn report_cards(w: &mut World, year: i32) {
    let teams = franchise::active_teams(year - 1);
    let mut rows: Vec<(f64, usize, [u8; 11])> = Vec::new();
    for &t in &teams {
        if t >= w.finance.nfl.len() || t >= w.clubs.len() {
            continue;
        }
        let c = &w.finance.nfl[t];
        let cl = &w.clubs[t];
        let o = &cl.office.owner;
        let z = |r: u8| crate::world::people::Support::z(r);
        let fac = c.facility as f64 * 0.7 + 0.3 * (1.0 - c.home.age(year) as f64 / 30.0).max(0.0);
        let am = c.amenities as f64;
        let mood = cl.team_mood as f64;
        let hc = w.nfl[t].head_coach.and_then(|h| w.coaches.get(h));
        let coach = hc.map_or(0.0, |h| (h.ability().relationships - 10.0) / 10.0);
        let raw = [
            0.6 * c.family as f64 + 0.4 * am,
            am,
            0.6 * am + 0.4 * (0.5 + 0.5 * z(cl.support.strength.rating)),
            fac,
            0.6 * fac + 0.4 * (0.5 + 0.5 * z(cl.support.medical.rating)),
            0.5 + 0.5 * z(cl.support.medical.rating),
            fac,
            0.5 + 0.5 * z(cl.support.strength.rating),
            0.5 * am + 0.5 * o.cash_will,
            clamp(0.5 + 0.35 * coach + 0.3 * mood, 0.0, 1.0),
            clamp(
                0.45 + 0.3 * (o.cash_will - 0.5) - 0.35 * (o.meddling - 0.3)
                    + 0.25 * (o.trust - 0.5)
                    + 0.2 * (c.family as f64 - 0.5)
                    + 0.2 * (am - 0.5),
                0.0,
                1.0,
            ),
        ];
        let mut g = [0u8; 11];
        let mut sum = 0.0;
        for (k, &v) in raw.iter().enumerate() {
            let x = clamp(v + 0.08 * draw_normal(w, 0xCA4D, (t as u64) << 4 ^ k as u64, year), 0.0, 1.0);
            sum += x;
            g[k] = (x * (data::GRADES.len() - 1) as f64).round() as u8;
        }
        rows.push((sum, t, g));
    }
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (rank, &(_, t, g)) in rows.iter().enumerate() {
        w.finance.cards.push(Card { year, team: t as u8, grades: g, rank: (rank + 1) as u8 });
    }
    let public = year <= data::CARDS_PUBLIC_TO;
    if public && let (Some(top), Some(bottom)) = (rows.first(), rows.last()) {
        let text = format!(
            "The NFLPA's team report cards are out: the players rank the {} first and the {} last. The {}'s owner gets an {}.",
            w.nfl[top.1].name,
            w.nfl[bottom.1].name,
            w.nfl[bottom.1].name,
            data::GRADES[bottom.2[10] as usize]
        );
        let e = story(w, Level::Nfl, Loudness::Big, text).team(bottom.1).rule("NFLPA: team report cards");
        super::post(w, e);
    } else if year == data::CARDS_PUBLIC_TO + 1 {
        let text = "An arbitrator rules for the league: the NFLPA's team report cards stay inside the union from this year. Players and agents still see them.".to_string();
        let e = story(w, Level::Nfl, Loudness::Medium, text).rule("NFLPA: team report cards");
        super::post(w, e);
    }
    if let Some(s) = w.seat.and_then(|s| s.nfl_team)
        && w.seat_club(s)
        && let Some(r) = rows.iter().position(|x| x.1 == s)
    {
        let g = rows[r].2;
        let worst = (0..11).min_by_key(|&k| g[k]).unwrap_or(0);
        let text = format!(
            "Your players' report card from the NFLPA: {} of {}. Their lowest mark: {} ({}).",
            ordinal(r + 1),
            rows.len(),
            data::CARD_PARTS[worst].to_lowercase(),
            data::GRADES[g[worst] as usize]
        );
        let e = story(w, Level::Nfl, Loudness::Big, text).team(s).rule("NFLPA: team report cards");
        super::post(w, e);
    }
    // A bad card pushes the owner: food, travel and the family room get
    // money at once; a poor building starts a new one (the Commanders and
    // Cardinals after 2023).
    for &(_, t, g) in &rows {
        let pr = w.clubs[t].office.owner.pr;
        let shamed = if public { 0.25 + 0.6 * pr } else { 0.15 + 0.4 * pr };
        let worst_amen = g[0].min(g[1]).min(g[8]);
        if worst_amen <= 4 && draw(w, 0xCA4E, t as u64, year) < shamed {
            let c = &mut w.finance.nfl[t];
            c.amenities = (c.amenities + 0.3).min(1.0);
            c.family = (c.family + 0.2).min(1.0);
        }
        let worst_fac = g[3].min(g[4]).min(g[6]);
        let building =
            w.finance.projects.iter().any(|p| p.owner == Owner::Nfl(t) && !p.done && p.kind == ProjectKind::Facility);
        if worst_fac <= 4 && !building && draw(w, 0xCA4F, t as u64, year) < shamed {
            let grow = data::league_revenue(year) / data::league_revenue(2024);
            let cost = 160.0e6 * grow;
            w.finance.projects.push(Project {
                owner: Owner::Nfl(t),
                kind: ProjectKind::Facility,
                name: "Training facility".to_string(),
                cost: cost as f32,
                public: 0.0,
                pledged: 0.0,
                started: year,
                opens: year + 2,
                capacity: 0,
                step: 0.0,
                done: false,
                asked: false,
            });
            let name = w.nfl[t].name;
            let text = format!(
                "The {name} break ground on a new training facility ({}), after their players' poor marks.",
                dollars(cost)
            );
            let e = story(w, Level::Nfl, Loudness::Medium, text).team(t);
            super::post(w, e);
        }
    }
}

/// The latest card a club's players gave it, if any.
pub fn card(w: &World, t: usize) -> Option<&Card> {
    w.finance.cards.iter().rev().find(|c| c.team as usize == t)
}

/// How a free agent and his agent read a club's card, about 0.94-1.06
/// (the agents see every card, public or not).
pub(crate) fn card_pull(w: &World, t: usize) -> f64 {
    match card(w, t) {
        Some(c) => {
            let avg = c.grades.iter().map(|&g| g as f64).sum::<f64>() / 11.0 / (data::GRADES.len() - 1) as f64;
            0.94 + 0.12 * avg
        }
        None => 1.0,
    }
}

/// The owner's words about money, as the head coach hears them (people
/// topic 12: no meters).
pub fn owner_words(w: &World, t: usize) -> String {
    let Some(cl) = w.clubs.get(t) else { return String::new() };
    let o = &cl.office.owner;
    let c = &w.finance.nfl[t];
    let profit = c.years.last().map_or(0.0, |b| Club::revenue(b) - Club::costs(b));
    let spend = spend_level(w, t);
    let mut s = Vec::new();
    s.push(if spend > 0.8 {
        "\"Whatever wins. Tell me what you need.\""
    } else if spend > 0.6 {
        "\"We'll spend where it makes us better. Make the case.\""
    } else if spend > 0.45 {
        "\"Every dollar has to earn its place.\""
    } else {
        "\"We run this club like a business. Do more with what you have.\""
    });
    if o.cash_will < 0.35 {
        s.push("He doesn't like putting guaranteed money in escrow.");
    } else if o.cash_will > 0.7 {
        s.push("He'll front-load a deal with signing bonus money.");
    }
    if profit < 0.0 {
        s.push("Last year's books ran in the red, and he mentions it.");
    }
    if c.moving.is_some() {
        s.push("His mind is on the move.");
    } else if c.votes_lost > 0 {
        s.push("He talks about the stadium every time you see him.");
    }
    s.join(" ")
}
