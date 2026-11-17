//! The donors (Capital lane C1; `docs/design/capital.md`, section 1).
//!
//! Each college program keeps 15 to 30 named donors (fewer below the
//! Power 4): wealth (hidden; the development office reads it as a range,
//! `screen`), age, loyalty to the school, wants, a circle and a memory of
//! each coach he has dealt with. Old money against new money. The 1 to 5
//! boosters the talks already know (`folk::cast`, `Role::Booster`) are the
//! same men, at the top of the list. Donors age, some die and leave estate
//! gifts (paid by lane C3), and new ones join. Some have ties to two schools
//! (CP28). Sizes are Claude's (proposed, not confirmed).

use super::Capital;
use crate::calendar::Date;
use crate::rng::{Rng, clamp};
use crate::world::World;
use crate::world::data::Division;
use crate::world::ethnicity::Ethnicity;
use crate::world::folk::Team;
use crate::world::folk::cast::{self, CastRef, Role, Want};

/// Where his money came from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Money {
    /// Family money, a business two generations old: he moves in circles and
    /// hears what his friends hear.
    #[default]
    Old,
    /// Made it himself (tech, private equity, energy, an NFL contract): he
    /// keeps his own counsel.
    New,
}

/// What a donor remembers of one coach (his memory of you, CP2-CP5).
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Bond {
    pub coach: u32,
    /// 0 cold to 1 he'd follow him anywhere; 0.5 a stranger.
    pub warmth: f32,
    /// Times they have met (lane C4's slots add them).
    pub met: u16,
    /// Promises to him kept and broken.
    pub kept: u8,
    pub broken: u8,
    /// Broken promises he heard of through his circle.
    pub heard: u8,
    /// The last day they met.
    pub last: Option<Date>,
    /// The coach coached him (an ex-pro's college coaches).
    pub coached: bool,
}

/// A donor. (Later lanes add fields with `#[serde(default)]`.)
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Donor {
    pub id: u32,
    pub name: String,
    pub she: bool,
    pub eth: Ethnicity,
    pub born: i32,
    /// His school (a program index).
    pub home: u32,
    /// A second school that can court him too (CP28), and how much of his
    /// giving it pulls, 0-1.
    pub also: Option<u32>,
    pub also_pull: f32,
    pub money: Money,
    /// His business, in words ("an oil and gas company out of Midland").
    pub business: String,
    /// A graduate of his school.
    pub alum: bool,
    /// Hidden: what he could truly give over a drive, in dollars.
    pub capacity: f64,
    /// Hidden: what his assets show on paper (the screening reads this; far
    /// above `capacity` when he is stretched thin, CP3).
    pub paper: f64,
    /// His bond with the school, 0-1.
    pub affinity: f32,
    /// What he wants (the nine kinds, `cast::Want`).
    pub wants: Vec<Want>,
    /// The rooms he would put money into, in the spec's CP17 order (0 the
    /// locker room ... 12 the stadium; `ROOMS`).
    pub rooms: Vec<u8>,
    /// Hidden: -1 gives only to NIL, 1 only to buildings (lane C3, CP9).
    pub nil_lean: f32,
    /// Hidden: how much he likes to collect on a gift later, 0-1 (lane C3,
    /// CP10).
    pub collects: f32,
    /// His circle (`Circle`): None until the draw, Some(0) when he moves in
    /// none (ids start at 1). Read it with `in_circle`.
    pub circle: Option<u32>,
    /// Traits 1-20 (as the cast's).
    pub ego: f32,
    pub warmth: f32,
    pub honesty: f32,
    pub meddling: f32,
    pub patience: f32,
    /// The booster slot and term he holds in the talks (`folk::cast`).
    pub cast: Option<(u8, u16)>,
    /// The player id of an ex-pro (CP6).
    pub pro: Option<u32>,
    pub joined: i32,
    /// The year he left the list: died, or drifted off.
    pub left: Option<i32>,
    pub died: bool,
    /// His memory of each coach he has dealt with, by coach.
    pub bonds: Vec<Bond>,
}

impl Donor {
    pub fn age(&self, year: i32) -> i32 {
        year - self.born
    }
    /// On the list: alive and still giving.
    pub fn alive(&self) -> bool {
        self.left.is_none()
    }
    /// Stretched thin: his paper wealth is well above what he can give.
    pub fn stretched(&self) -> bool {
        self.paper > 1.8 * self.capacity
    }
    pub fn bond(&self, coach: u32) -> Option<&Bond> {
        self.bonds.iter().find(|b| b.coach == coach)
    }
    /// His warmth toward a coach (0.5 for one he hasn't dealt with).
    pub fn warmth_to(&self, coach: u32) -> f32 {
        self.bond(coach).map_or(STRANGER, |b| b.warmth)
    }
    pub(crate) fn bond_mut(&mut self, coach: u32) -> &mut Bond {
        if let Some(k) = self.bonds.iter().position(|b| b.coach == coach) {
            return &mut self.bonds[k];
        }
        self.bonds.push(Bond { coach, warmth: STRANGER, ..Default::default() });
        self.bonds.last_mut().unwrap()
    }
    /// The schools that can court him, with the share of his giving each
    /// pulls (CP28).
    pub fn schools(&self) -> Vec<(u32, f32)> {
        match self.also {
            Some(q) => vec![(self.home, 1.0 - self.also_pull), (q, self.also_pull)],
            None => vec![(self.home, 1.0)],
        }
    }
}

/// A circle of donors who talk (CP4).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Circle {
    pub id: u32,
    pub home: u32,
    /// "the country club's Saturday foursome".
    pub name: String,
    /// An old-money circle (closer: word travels in full).
    pub old: bool,
}

/// An estate gift left when a donor died (lane C3 pays it).
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Estate {
    pub donor: u32,
    pub program: u32,
    pub year: i32,
    pub amount: f64,
    /// The room it is restricted to, if any (CP17 order).
    pub room: Option<u8>,
    pub paid: bool,
}

/// The rooms of the spec's table (CP17), in its order.
pub const ROOMS: [&str; 13] = [
    "locker room",
    "weight room",
    "sports science lab",
    "recovery center",
    "nutrition center",
    "film theater",
    "indoor field",
    "recruiting lounge",
    "players' lounge",
    "academic center",
    "content studio",
    "trophy hall and NFL wall",
    "stadium",
];

/// A coach a donor hasn't dealt with.
pub const STRANGER: f32 = 0.5;
/// What breaking your word to a donor costs his warmth toward you, and what
/// keeping it adds (CP4).
pub const BROKEN_SELF: f32 = -0.25;
pub const KEPT_SELF: f32 = 0.08;
/// What a circle mate loses when he hears of it: old money in full, new
/// money hardly at all ("new money doesn't care"), times 0.6 in a new-money
/// circle (looser ties).
pub const HEARD_OLD: f32 = -0.12;
pub const HEARD_NEW: f32 = -0.02;
pub const LOOSE_CIRCLE: f32 = 0.6;
/// Chance he moves in a circle: old money, new money.
pub const IN_CIRCLE_OLD: f64 = 0.85;
pub const IN_CIRCLE_NEW: f64 = 0.4;
/// Chance a new donor has ties to a second school in his state (CP28).
pub const TWO_SCHOOLS: f64 = 0.07;
/// Chance a donor's paper wealth is stretched thin (CP3), and how far.
pub const STRETCHED: f64 = 0.12;
/// Each year a living donor drifts off the list (moved, lost interest).
pub const DRIFT: f64 = 0.03;
/// Mortality: a Gompertz curve near the SSA period life table (0.3% at 50,
/// 2% at 70, 5% at 80, 13% at 90), times 0.8 for the wealthy.
pub const GOMPERTZ_A: f64 = 2.78e-5;
pub const GOMPERTZ_B: f64 = 0.094;
pub const WEALTHY_MORTALITY: f64 = 0.8;
/// An estate gift: chance 0.2 + 0.5 x his affinity; size a share of his
/// capacity, 0.15 + 0.35 x affinity.
pub const ESTATE_BASE: f64 = 0.2;
pub const ESTATE_AFFINITY: f64 = 0.5;
/// Dead donors stay in the store this many years (their estates), then go.
pub const KEEP_DEAD: i32 = 3;

/// How many donors a program keeps: the Power 4 22-30, the rest of the FBS
/// 15-22, FCS 8-14, Division II, NAIA and Division III 3-5, by prestige;
/// junior colleges none.
pub fn target(w: &World, p: usize) -> usize {
    let prog = &w.programs[p];
    let pr = prog.prestige.clamp(0.0, 1.0);
    let (lo, span) = match prog.division {
        Division::FBS if prog.power_conference(w.year) => (22.0, 8.0),
        Division::FBS => (15.0, 7.0),
        Division::FCS => (8.0, 6.0),
        Division::D2 | Division::NAIA | Division::D3 => (3.0, 2.0),
        Division::JUCO => return 0,
    };
    (lo + span * pr).round() as usize
}

/// The median capacity of a new donor at a program, in dollars (lognormal
/// around it, sd 1.1 in logs; 1.3 for new money).
fn median_capacity(w: &World, p: usize, year: i32) -> f64 {
    let prog = &w.programs[p];
    let base = match prog.division {
        Division::FBS if prog.power_conference(year) => 4.0e6,
        Division::FBS => 1.2e6,
        Division::FCS => 0.35e6,
        _ => 0.12e6,
    };
    base * (0.6 + 0.8 * prog.prestige.clamp(0.0, 1.0)) * dollars(year)
}

/// Dollars of a year against 2026 (3% a year).
pub fn dollars(year: i32) -> f64 {
    1.03f64.powi(year - 2026)
}

/// The share of new donors who are new money: 30% in 1990, 50% by 2026.
fn new_money_share(year: i32) -> f64 {
    clamp(0.3 + 0.2 * (year - 1990) as f64 / 36.0, 0.3, 0.55)
}

const OLD_BUSINESS: [&str; 16] = [
    "the family's car dealerships",
    "a bank his grandfather founded",
    "cattle and land",
    "the family's timber company",
    "a beer distributorship",
    "commercial real estate downtown",
    "an insurance agency three generations old",
    "a law firm with his name on the door",
    "the family's grocery chain",
    "oil and gas royalties",
    "a construction company",
    "a trucking company",
    "a medical practice and the land around it",
    "the family's furniture business",
    "farmland across three counties",
    "a regional hardware chain",
];
const NEW_BUSINESS: [&str; 14] = [
    "a software company he sold",
    "a private equity firm",
    "a hedge fund",
    "an oil and gas company he built",
    "a chain of urgent care clinics",
    "a home builder",
    "a logistics company",
    "a fintech startup",
    "a fast food franchise group",
    "a pipeline company",
    "a cybersecurity firm",
    "a solar developer",
    "a medical device company",
    "a car wash chain",
];
const OLD_CIRCLES: [&str; 8] = [
    "the country club's Saturday foursome",
    "the old downtown bank board",
    "the deacons' table at First Baptist",
    "the hunting lease out west of town",
    "the lettermen's golf outing",
    "the old families on the hill",
    "the cattlemen's lunch",
    "the Rotary's old guard",
];
const NEW_CIRCLES: [&str; 4] =
    ["the tech founders' dinner", "the private equity crowd", "the developers' table", "the young alumni board"];

fn trait_draw(rng: &mut Rng, m: f64) -> f32 {
    clamp(rng.normal(m, 3.5), 1.0, 20.0) as f32
}

/// The rooms a donor would give to: one or two, by who he is (an ex-pro
/// the weight room and the locker room first).
pub(crate) fn pick_rooms(rng: &mut Rng, money: Money, alum: bool, pro: bool) -> Vec<u8> {
    let mut pool: Vec<(u8, f64)> = if pro {
        vec![(0, 1.5), (1, 2.0), (3, 1.0), (4, 0.6), (8, 0.6), (11, 0.8), (2, 0.4)]
    } else {
        vec![
            (0, 1.2),
            (1, 1.0),
            (2, 0.4),
            (3, 0.5),
            (4, 0.4),
            (5, 0.5),
            (6, 0.8),
            (7, 0.7),
            (8, 0.5),
            (9, if alum { 0.8 } else { 0.5 }),
            (10, if money == Money::New { 0.8 } else { 0.3 }),
            (11, 0.6),
            (12, if money == Money::Old { 1.2 } else { 0.8 }),
        ]
    };
    let n = if rng.chance(0.5) { 2 } else { 1 };
    let mut out = Vec::new();
    for _ in 0..n {
        let r = rng.weighted(&pool);
        out.push(r);
        pool.retain(|x| x.0 != r);
    }
    out
}

/// One or two of the nine wants, weighted like a booster's (a facility
/// layer's donors lean to a building).
fn pick_wants(rng: &mut Rng, meddling: f32, honesty: f32, patience: f32, year: i32) -> Vec<Want> {
    let mut pool: Vec<(Want, f64)> = vec![
        (Want::BeatRival, 1.0),
        (Want::Access, 1.0),
        (Want::SayInHires, 0.4 + 0.04 * meddling as f64),
        (Want::SayOnPlayer, 0.2 + 0.04 * meddling as f64),
        (Want::Clean, 0.5 + 0.03 * honesty as f64),
        (Want::Building, 1.2),
        (Want::OldSchool, 0.4),
        (Want::WinNow, 0.4 + 0.04 * (20.0 - patience as f64)),
    ];
    if year >= 2021 {
        pool.push((Want::NilDeals, 0.7));
    }
    let n = if rng.chance(0.55) { 2 } else { 1 };
    let mut out = Vec::new();
    for _ in 0..n {
        let x = rng.weighted(&pool);
        out.push(x);
        pool.retain(|y| y.0 != x);
    }
    out
}

/// A second school in his state for a donor of `p` (CP28): another
/// four-year program, weighted by prestige.
fn second_school(w: &World, p: usize, rng: &mut Rng) -> Option<usize> {
    let st = w.programs[p].state;
    let pool: Vec<(usize, f64)> = w
        .programs
        .iter()
        .enumerate()
        .filter(|(q, o)| *q != p && o.state == st && matches!(o.division, Division::FBS | Division::FCS))
        .map(|(q, o)| (q, 0.05 + o.prestige.max(0.0)))
        .collect();
    if pool.is_empty() { None } else { Some(rng.weighted(&pool)) }
}

/// A new donor at program `p` (no id yet): `joined` is the year he joined,
/// `n` a key that makes him his own man.
pub(crate) fn draw_donor(w: &World, p: usize, year: i32, joined: i32, n: u64) -> Donor {
    let key = (p as u64) << 32 ^ (joined as u64) << 16 ^ n;
    // Fold the high half in before the seed is cut to 32 bits, so the
    // program (bits 32 and up) makes each school's donors their own.
    let h = (key ^ 0xD0_0000).wrapping_mul(0x2545_F491_4F6C_DD1D);
    let mut rng = Rng::new(w.options.seed ^ (h ^ h >> 32) as u32);
    let money = if rng.chance(new_money_share(joined)) { Money::New } else { Money::Old };
    let alum = rng.chance(if money == Money::Old { 0.75 } else { 0.62 });
    let she = rng.chance(0.12);
    let eth = cast::heritage(Role::Booster, &mut rng);
    let age_join = (rng.normal(if money == Money::Old { 58.0 } else { 48.0 }, 9.0)).clamp(32.0, 85.0) as i32;
    let born = joined - age_join;
    let name = cast::person_name(eth, born, she, &mut rng);
    let sd = if money == Money::New { 1.3 } else { 1.1 };
    let capacity = (median_capacity(w, p, joined) * rng.log_normal(sd)).clamp(25_000.0, 3.0e9);
    let paper = if rng.chance(STRETCHED) { capacity * rng.range(2.0, 4.0) } else { capacity * rng.log_normal(0.1) };
    let affinity = clamp(rng.normal(if alum { 0.6 } else { 0.4 }, 0.15), 0.05, 1.0) as f32;
    let ego = trait_draw(&mut rng, 10.0);
    let warmth = trait_draw(&mut rng, 10.5);
    let honesty = trait_draw(&mut rng, 11.0);
    let meddling = trait_draw(&mut rng, 9.0);
    let patience = trait_draw(&mut rng, 10.0);
    let wants = pick_wants(&mut rng, meddling, honesty, patience, year);
    let rooms = pick_rooms(&mut rng, money, alum, false);
    let lean = if money == Money::New { -0.2 } else { 0.3 };
    let nil_lean = if year >= 2021 { clamp(rng.normal(lean, 0.45), -1.0, 1.0) as f32 } else { 1.0 };
    let collects = clamp(rng.normal(0.25 + 0.02 * (meddling as f64 - 9.0), 0.2), 0.0, 1.0) as f32;
    let mut business = if money == Money::Old { rng.pick(&OLD_BUSINESS) } else { rng.pick(&NEW_BUSINESS) }.to_string();
    let mut also = None;
    let mut also_pull = 0.0;
    if rng.chance(TWO_SCHOOLS)
        && let Some(q) = second_school(w, p, &mut rng)
    {
        also = Some(q as u32);
        also_pull = rng.range(0.2, 0.6) as f32;
        business = format!("{business}, made in {}", w.programs[q].city);
    }
    Donor {
        id: 0,
        name,
        she,
        eth,
        born,
        home: p as u32,
        also,
        also_pull,
        money,
        business,
        alum,
        capacity,
        paper,
        affinity,
        wants,
        rooms,
        nil_lean,
        collects,
        circle: None,
        ego,
        warmth,
        honesty,
        meddling,
        patience,
        cast: None,
        pro: None,
        joined,
        left: None,
        died: false,
        bonds: Vec::new(),
    }
}

/// A donor from a booster the talks know (the same man: name, age,
/// traits, wants and business from `folk::cast`).
fn from_booster(w: &World, p: usize, c: &cast::Cast) -> Donor {
    let year = w.year;
    let mut d = draw_donor(w, p, year, year, 0xB005_0000 ^ (c.r.slot as u64) << 8 ^ c.term as u64);
    d.name = c.name.clone();
    d.she = c.she;
    d.eth = c.eth;
    d.born = year - c.age as i32;
    d.joined = c.since.min(year);
    d.ego = c.ego as f32;
    d.warmth = c.warmth as f32;
    d.honesty = c.honesty as f32;
    d.meddling = c.meddling as f32;
    d.patience = c.patience as f32;
    d.cast = Some((c.r.slot, c.term));
    if let Some(b) = &c.booster {
        d.business = b.business.clone();
        d.wants = b.wants.clone();
        // A booster's yearly gift is a sliver of what he could give: about
        // 30 years of it, more for the big giver.
        let yearly = b.yearly.max(0.0) * if b.big { 40.0 } else { 30.0 };
        d.capacity = d.capacity.max(yearly);
        if d.paper < d.capacity {
            d.paper = d.capacity * 1.05;
        }
        if b.wants.contains(&Want::Building) && !d.rooms.contains(&12) {
            d.rooms.push(12);
        }
    }
    d.affinity = d.affinity.max(0.6);
    d
}

fn add(cap: &mut Capital, mut d: Donor) -> u32 {
    d.id = cap.id();
    let id = d.id;
    cap.donors.push(d);
    id
}

/// The colleges that keep donors (junior colleges don't).
fn keeps(w: &World, p: usize) -> bool {
    w.programs[p].division != Division::JUCO
}

/// The yearly step on 1 July (and the first day, to build the store): the
/// boosters synced, the dead and drifting gone, new donors in to the
/// program's target, circles filled.
pub(crate) fn year(w: &mut World) {
    let year = w.year;
    let first = w.capital.year == 0;
    // Aging, deaths, estates and drift.
    if !first {
        let mut leave: Vec<(usize, bool)> = Vec::new();
        for (k, d) in w.capital.donors.iter().enumerate() {
            if !d.alive() {
                continue;
            }
            let age = d.age(year) as f64;
            let q = (GOMPERTZ_A * (GOMPERTZ_B * age).exp() * WEALTHY_MORTALITY).min(0.4);
            let u = super::draw(w, 1, (d.id as u64) << 16 ^ year as u64);
            if u < q {
                leave.push((k, true));
            } else if d.pro.is_none()
                && d.cast.is_none()
                && super::draw(w, 2, (d.id as u64) << 16 ^ year as u64) < DRIFT
            {
                leave.push((k, false));
            }
        }
        for (k, died) in leave {
            let (id, home, aff, capa) = {
                let d = &w.capital.donors[k];
                (d.id, d.home, d.affinity as f64, d.capacity)
            };
            w.capital.donors[k].left = Some(year);
            w.capital.donors[k].died = died;
            if died && super::draw(w, 3, id as u64) < ESTATE_BASE + ESTATE_AFFINITY * aff {
                let room = w.capital.donors[k].rooms.first().copied();
                w.capital.estates.push(Estate {
                    donor: id,
                    program: home,
                    year,
                    amount: capa * (0.15 + 0.35 * aff),
                    room,
                    paid: false,
                });
            }
        }
        // The long dead and the drifted go; a pledge or estate keeps nothing
        // that needs him past that.
        w.capital.donors.retain(|d| d.left.is_none_or(|y| y + KEEP_DEAD > year));
    }
    let n = w.programs.len();
    for p in 0..n {
        if !keeps(w, p) {
            continue;
        }
        sync_boosters(w, p);
        let have = w.capital.donors.iter().filter(|d| d.alive() && d.home == p as u32 && d.pro.is_none()).count();
        let want = target(w, p);
        for k in have..want {
            // The first build spreads the joined years over the last two
            // decades; later the newcomers join this year.
            let joined =
                if first { year - (super::draw(w, 4, (p as u64) << 8 ^ k as u64) * 20.0) as i32 } else { year };
            let d = draw_donor(w, p, year, joined, k as u64 ^ (year as u64) << 8);
            add(&mut w.capital, d);
        }
        fill_circles(w, p);
    }
    w.capital.donors.sort_by_key(|d| d.id);
}

/// The booster slots the talks know at `p` are donors: a new man in a slot
/// is a new donor; the man he replaced stays on as a plain donor.
fn sync_boosters(w: &mut World, p: usize) {
    let team = Team::College(p);
    for c in cast::all(w, team, Role::Booster) {
        let slot = c.r.slot;
        let here =
            w.capital.donors.iter().position(|d| d.alive() && d.home == p as u32 && d.cast == Some((slot, c.term)));
        if here.is_some() {
            continue;
        }
        for d in w.capital.donors.iter_mut().filter(|d| d.home == p as u32 && d.cast.is_some_and(|x| x.0 == slot)) {
            d.cast = None;
        }
        let d = from_booster(w, p, &c);
        add(&mut w.capital, d);
    }
    // Slots the program no longer has (its prestige fell): the men stay.
    let slots = cast::slots(w, team, Role::Booster);
    for d in w.capital.donors.iter_mut().filter(|d| d.home == p as u32 && d.cast.is_some_and(|x| x.0 >= slots)) {
        d.cast = None;
    }
}

/// Circles at `p`: an old-money circle per seven donors (at least one), a
/// new-money one past twelve donors; donors without one join by money.
fn fill_circles(w: &mut World, p: usize) {
    let donors: Vec<usize> = (0..w.capital.donors.len())
        .filter(|&k| {
            let d = &w.capital.donors[k];
            d.alive() && d.home == p as u32 && d.circle.is_none()
        })
        .collect();
    if donors.is_empty() {
        return;
    }
    let size = w.capital.donors.iter().filter(|d| d.alive() && d.home == p as u32).count();
    let olds = (size / 7).max(1);
    let news = (size / 12).min(2);
    let have_old = w.capital.circles.iter().filter(|c| c.home == p as u32 && c.old).count();
    let have_new = w.capital.circles.iter().filter(|c| c.home == p as u32 && !c.old).count();
    for (k, old) in (have_old..olds).map(|k| (k, true)).chain((have_new..news).map(|k| (k, false))) {
        let list: &[&str] = if old { &OLD_CIRCLES } else { &NEW_CIRCLES };
        let name = list[(k + p) % list.len()].to_string();
        let id = w.capital.id();
        w.capital.circles.push(Circle { id, home: p as u32, name, old });
    }
    let olds: Vec<u32> = w.capital.circles.iter().filter(|c| c.home == p as u32 && c.old).map(|c| c.id).collect();
    let news: Vec<u32> = w.capital.circles.iter().filter(|c| c.home == p as u32 && !c.old).map(|c| c.id).collect();
    for k in donors {
        let (id, money, pro) = {
            let d = &w.capital.donors[k];
            (d.id, d.money, d.pro.is_some())
        };
        let u = super::draw(w, 5, id as u64);
        let v = super::draw(w, 6, id as u64);
        let pick = match money {
            Money::Old if u < IN_CIRCLE_OLD => olds.get((v * olds.len() as f64) as usize).copied(),
            Money::New if u < IN_CIRCLE_NEW && !pro => {
                if news.is_empty() || v < 0.25 {
                    olds.get((v * olds.len() as f64) as usize).copied()
                } else {
                    news.get((v * news.len() as f64) as usize).copied()
                }
            }
            _ => None,
        };
        let d = &mut w.capital.donors[k];
        d.circle = pick;
        // Looked at once: a donor outside any circle stays outside.
        if pick.is_none() {
            d.circle = Some(0);
        }
    }
    w.capital.circles.sort_by_key(|c| c.id);
}

impl Donor {
    /// His circle (`circle` holds 0 once he was looked at and joined none), if he is in one.
    pub fn in_circle(&self) -> Option<u32> {
        self.circle.filter(|&c| c != 0)
    }
}

/// The living donors a program can court: its own and those with ties to
/// it (CP28), by index in the store.
pub fn of(w: &World, p: usize) -> Vec<usize> {
    let p = p as u32;
    (0..w.capital.donors.len())
        .filter(|&k| {
            let d = &w.capital.donors[k];
            d.alive() && (d.home == p || d.also == Some(p))
        })
        .collect()
}

/// The donor who is a booster in the talks (his slot and term), if any.
pub fn of_booster(w: &World, r: CastRef, term: u16) -> Option<u32> {
    let Team::College(p) = r.team else { return None };
    w.capital.donors.iter().find(|d| d.home == p as u32 && d.cast == Some((r.slot, term))).map(|d| d.id)
}

/// Your word to a donor kept or broken (CP4): his warmth toward the coach
/// moves; a broken one spreads through his circle, in full to old money,
/// hardly at all to new money.
pub fn word(w: &mut World, donor: u32, coach: u32, kept: bool) {
    let Some(d) = w.capital.donor_mut(donor) else { return };
    let circle = d.in_circle();
    let b = d.bond_mut(coach);
    if kept {
        b.kept = b.kept.saturating_add(1);
        b.warmth = (b.warmth + KEPT_SELF).min(1.0);
        return;
    }
    b.broken = b.broken.saturating_add(1);
    b.warmth = (b.warmth + BROKEN_SELF).max(0.0);
    let Some(c) = circle else { return };
    let loose = w.capital.circle(c).is_some_and(|c| !c.old);
    for m in w.capital.donors.iter_mut().filter(|m| m.id != donor && m.alive() && m.in_circle() == Some(c)) {
        let size = if m.money == Money::Old { HEARD_OLD } else { HEARD_NEW } * if loose { LOOSE_CIRCLE } else { 1.0 };
        let b = m.bond_mut(coach);
        b.heard = b.heard.saturating_add(1);
        b.warmth = (b.warmth + size).max(0.0);
    }
}

/// Your word to a booster the talks know (`talks::check_promises`): his
/// donor's memory and his circle (CP4). Only a college booster is a donor.
pub(crate) fn booster_word(w: &mut World, r: CastRef, term: u16, kept: bool) {
    if r.role != Role::Booster {
        return;
    }
    let Some(coach) = w.career.as_ref().map(|c| c.coach as u32) else { return };
    if let Some(id) = of_booster(w, r, term) {
        word(w, id, coach, kept);
    }
}

/// A meeting with a donor (lane C4's slots call it): the count and the day.
pub fn met(w: &mut World, donor: u32, coach: u32, day: Date) {
    if let Some(d) = w.capital.donor_mut(donor) {
        let b = d.bond_mut(coach);
        b.met = b.met.saturating_add(1);
        b.last = Some(day);
    }
}
