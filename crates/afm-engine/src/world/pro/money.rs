//! Section 5 topic 2: money. The cap by year, minimum salaries, real
//! contract structure (base salary, signing bonus proration, guarantees,
//! roster, workout and per-game bonuses, incentives, void years) with each
//! year's cap hit and the dead money a release leaves, the rookie scale
//! (2011 on) and the rookie pool before it, and the tags and tenders.

use crate::calendar::Date;
use crate::positions::{POSITION_COUNT, Position};
use crate::rng::clamp;

/// The cap every price is read against. The real cap from 1994 (the
/// uncapped 2010 priced as 2009), Cain's growth rule after 2026, and before
/// 1994, when there was no cap, a notional one for prices: payrolls grew
/// about 6.5% a year into 1994 (proposed, not confirmed).
pub fn price_cap(year: i32) -> f64 {
    use crate::calendar::rules::{future_cap, real_salary_cap};
    match real_salary_cap(year) {
        Some(Some(c)) => c,
        Some(None) => 123.0e6,
        None if year < 1994 => 34.608e6 * 1.065f64.powi(year - 1994),
        None => future_cap(year),
    }
}

/// Whether a hard cap binds this league year (1994 on, not 2010).
pub fn capped(year: i32) -> bool {
    year >= 1994 && year != 2010
}

/// Whether teams keep their books the cap's way (the top 51 until the
/// first game, room kept for the season, cuts and restructures): from 1994
/// on, the uncapped 2010 included, when owners still budgeted against the
/// 2009 cap and teams cut and restructured as usual (trial 7 finding 5).
pub fn cap_books(year: i32) -> bool {
    year >= 1994
}

/// The accrued seasons that make a player an unrestricted free agent: four
/// from 1993, six in the uncapped 2010 (real: four and five became
/// restricted that year).
pub fn ufa_seasons(year: i32) -> u32 {
    if year == 2010 { 6 } else { 4 }
}

/// The league minimum by credited seasons (three games on full pay status
/// each; none, 1, 2, 3, 4-6, 7 or more), from the CBAs' own scales (minor
/// bug B24; credited, not accrued, from the batch's second review): the 2011 CBA's $375K, $450K,
/// $525K, $600K, $685K, $810K (7-9) and $910K (10 or more) in 2011, $15K a
/// year more to 2019; the
/// 2020 CBA's $610K to $1.05M in 2020, then 2025's $840K, $960K, $1.03M,
/// $1.1M, $1.17M and $1.255M less or more $45K a year from 2021 to 2030
/// ($885K with none in 2026). Before 2011 and after 2030 the nearest
/// scale moves with the cap.
pub fn minimum(year: i32, credited: u32) -> f64 {
    const Y2011: [f64; 7] = [375e3, 450e3, 525e3, 600e3, 685e3, 810e3, 910e3];
    const Y2020: [f64; 6] = [610e3, 675e3, 750e3, 825e3, 910e3, 1_050e3];
    const Y2025: [f64; 6] = [840e3, 960e3, 1_030e3, 1_100e3, 1_170e3, 1_255e3];
    let k = match credited {
        0..=3 => credited as usize,
        4..=6 => 4,
        _ => 5,
    };
    // The 2011 scale's own tier for ten seasons or more (review of B24).
    let k11 = if credited >= 10 { 6 } else { k };
    let scaled = |base: f64, from: i32| (base * price_cap(year) / price_cap(from) / 5_000.0).round() * 5_000.0;
    match year {
        ..=2010 => scaled(Y2011[k11], 2011),
        2011..=2019 => Y2011[k11] + 15e3 * (year - 2011) as f64,
        2020 => Y2020[k],
        2021..=2030 => Y2025[k] + 45e3 * (year - 2025) as f64,
        _ => scaled(Y2025[k] + 45e3 * 5.0, 2030),
    }
}

/// A practice squad player's weekly minimum with two or fewer accrued
/// seasons (minor bug B24): the 2011 CBA's $5,700 to $8,000 (2019), the
/// 2020 CBA's $8,400 (2020) to $16,750 (2030), $13,750 in 2026. Before
/// 2011 and after 2030 it moves with the cap.
pub fn practice_squad_week(year: i32) -> f64 {
    const WEEK: [f64; 20] = [
        5_700.0, 5_700.0, 6_000.0, 6_300.0, 6_600.0, 6_900.0, 7_200.0, 7_600.0, 8_000.0, 8_400.0, 9_200.0, 11_500.0,
        12_000.0, 12_500.0, 13_000.0, 13_750.0, 14_500.0, 15_250.0, 16_000.0, 16_750.0,
    ];
    match year {
        ..=2010 => (WEEK[0] * price_cap(year) / price_cap(2011) / 50.0).round() * 50.0,
        2011..=2030 => WEEK[(year - 2011) as usize],
        _ => (WEEK[19] * price_cap(year) / price_cap(2030) / 50.0).round() * 50.0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    Rookie,
    Undrafted,
    Veteran,
    /// A one-year deal at the minimum (the veteran benefit applies).
    Minimum,
    Franchise,
    Transition,
    /// A restricted free agent's tender.
    Tender,
    /// A reserve/future deal from the end of a season.
    Future,
    PracticeSquad,
    Extension,
}

/// One league year of a contract.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Year {
    pub year: i32,
    pub base: f64,
    /// Base salary guaranteed at signing.
    pub guaranteed: f64,
    /// Base salary guaranteed for injury only, until it vests.
    pub injury: f64,
    /// The day the injury guarantee becomes full (on the roster that day).
    pub vests: Option<Date>,
    pub roster_bonus: f64,
    pub workout: f64,
    /// Per-game roster bonuses if he is active every game.
    pub per_game: f64,
    /// Likely and not likely to be earned incentives.
    pub ltbe: f64,
    pub nltbe: f64,
    /// This year's share of the signing and option bonuses.
    pub proration: f64,
    /// A year that voids before it is played: it only carries proration.
    pub void: bool,
    /// Bonus money paid out in this league year (the signing bonus in the
    /// year it is signed, a restructure's bonus in its year). Fix list S6.3:
    /// a bonus is paid once.
    pub bonus_paid: f64,
}

impl Year {
    pub fn cap_hit(&self) -> f64 {
        self.base + self.proration + self.roster_bonus + self.workout + self.per_game + self.ltbe
    }
    pub fn cash(&self) -> f64 {
        self.pay() + self.bonus_paid
    }
    /// This year's pay without bonus money.
    pub fn pay(&self) -> f64 {
        if self.void { 0.0 } else { self.base + self.roster_bonus + self.workout + self.per_game }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Contract {
    pub team: usize,
    pub kind: Kind,
    /// League year it was signed in.
    pub signed: i32,
    pub years: Vec<Year>,
    pub signing_bonus: f64,
    pub no_trade: bool,
    /// Rookie offset language: a cut rookie's new pay offsets what he is owed.
    pub offsets: bool,
    /// A round-one rookie deal's fifth-year option (2011 on): None until
    /// decided, then whether it was picked up.
    pub option: Option<bool>,
    /// The first league year of this deal's new money: the signing year for
    /// a new deal, the year after for an extension (OverTheCap's "new money"
    /// headline; fix list S6.3).
    pub new_from: i32,
    /// Incentives with terms (live talks lane N2): each pays on a season
    /// term, likely ones on the cap now, the rest charged when earned.
    /// Only your deals carry them (lane N5b makes them).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clauses: Vec<Incentive>,
}

/// An incentive with a term (lane N2): what pays, in which league year,
/// and whether it was likely to be earned when signed (met the season
/// before, the NFL's rule).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Incentive {
    pub year: i32,
    pub term: crate::world::folk::usage::Usage,
    pub pays: f64,
    pub likely: bool,
    /// Settled after the regular season: earned or not.
    #[serde(default)]
    pub earned: Option<bool>,
}

impl Contract {
    /// Adds an incentive with a term: a likely one counts on that year's cap
    /// now (as `ltbe`), an unlikely one only when earned.
    pub fn add_incentive(&mut self, inc: Incentive) {
        if inc.likely
            && let Some(y) = self.years.iter_mut().find(|y| y.year == inc.year)
        {
            y.ltbe += inc.pays;
        }
        self.clauses.push(inc);
    }
}

impl Contract {
    pub fn year(&self, y: i32) -> Option<&Year> {
        self.years.iter().find(|x| x.year == y)
    }
    /// The last year he is under contract (void years left out).
    pub fn last_year(&self) -> i32 {
        self.years.iter().filter(|x| !x.void).map(|x| x.year).max().unwrap_or(self.signed)
    }
    pub fn real_years(&self) -> u32 {
        self.years.iter().filter(|x| !x.void && x.year >= self.new_from).count() as u32
    }
    pub fn cap_hit(&self, y: i32) -> f64 {
        self.year(y).map_or(0.0, Year::cap_hit)
    }
    /// Total new money over the real years: bonuses plus pay.
    pub fn total(&self) -> f64 {
        self.signing_bonus + self.years.iter().filter(|x| x.year >= self.new_from).map(Year::pay).sum::<f64>()
    }
    /// Average a year: the headline number agents sell.
    pub fn apy(&self) -> f64 {
        self.total() / self.real_years().max(1) as f64
    }
    /// What he is sure to get at signing: the bonus and guaranteed base.
    pub fn guaranteed(&self) -> f64 {
        self.signing_bonus + self.years.iter().filter(|x| x.year >= self.new_from).map(|x| x.guaranteed).sum::<f64>()
    }
    /// Pay still guaranteed from league year `y` on (vested injury
    /// guarantees count once the day has passed).
    pub fn guaranteed_left(&self, y: i32, today: Date) -> f64 {
        self.years
            .iter()
            .filter(|x| x.year >= y && !x.void)
            .map(|x| x.guaranteed + if x.vests.is_some_and(|v| v <= today) { x.injury } else { 0.0 })
            .sum()
    }
    /// Dead money if released in league year `y`: every later year's
    /// proration accelerates and guaranteed base is owed. A post-June 1
    /// release (or designation) keeps this year's proration this year and
    /// moves the rest to next year: (this year, next year).
    pub fn dead_if_cut(&self, y: i32, today: Date, june1: bool) -> (f64, f64) {
        let now: f64 = self.year(y).map_or(0.0, |x| x.proration);
        let later: f64 = self.years.iter().filter(|x| x.year > y).map(|x| x.proration).sum();
        let owed = self.guaranteed_left(y, today);
        if june1 { (now + owed, later) } else { (now + later + owed, 0.0) }
    }
    /// The cap saved by releasing him in year `y`.
    pub fn cut_savings(&self, y: i32, today: Date, june1: bool) -> f64 {
        self.cap_hit(y) - self.dead_if_cut(y, today, june1).0
    }
}

/// What a deal is made of, before it is laid out by year.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Terms {
    pub years: u32,
    /// New money a year.
    pub apy: f64,
    /// Signing bonus as a share of the total.
    pub bonus_share: f64,
    /// Base salary guaranteed at signing beyond the bonus, as a share of the total.
    pub base_guarantee: f64,
    /// Injury-only guarantees that vest in a later March, as a share of the total.
    pub injury_guarantee: f64,
    /// How much each year's base grows over the last (back-loading).
    pub ramp: f64,
    /// Roster, workout and per-game bonuses as shares of each year's pay.
    pub roster_share: f64,
    pub workout_share: f64,
    pub per_game_share: f64,
    /// Not likely to be earned incentives a year, on top of the money.
    pub incentives: f64,
    pub void_years: u32,
    pub no_trade: bool,
    pub offsets: bool,
    /// Year one's pay set low (real: a big signing bonus and a base near
    /// the minimum in year one, the rest in the later years); 0 lays year
    /// one out like the others.
    #[serde(default)]
    pub first_base: f64,
}

impl Terms {
    /// A plain deal: no bonus, flat, nothing guaranteed.
    pub fn flat(years: u32, apy: f64) -> Terms {
        Terms {
            years: years.max(1),
            apy,
            bonus_share: 0.0,
            base_guarantee: 0.0,
            injury_guarantee: 0.0,
            ramp: 0.0,
            roster_share: 0.0,
            workout_share: 0.0,
            per_game_share: 0.0,
            incentives: 0.0,
            void_years: 0,
            no_trade: false,
            offsets: false,
            first_base: 0.0,
        }
    }

    pub fn total(&self) -> f64 {
        self.apy * self.years as f64
    }
    /// Signing bonus plus guaranteed base: the practical guarantee.
    pub fn guaranteed(&self) -> f64 {
        self.total() * (self.bonus_share + self.base_guarantee)
    }
    /// Year one's share of the pay outside the bonus, as `build` ramps it.
    fn first_share(&self) -> f64 {
        let pay = self.total() * (1.0 - self.bonus_share);
        if self.first_base > 0.0 && self.years > 1 && pay > 0.0 {
            return (self.first_base / pay).min(1.0);
        }
        1.0 / (0..self.years.max(1) as i32).map(|k| (1.0 + self.ramp).powi(k)).sum::<f64>()
    }
    /// Year one's cash: the signing bonus and the first year's pay.
    pub fn first_year_cash(&self) -> f64 {
        let bonus = self.total() * self.bonus_share;
        bonus + (self.total() - bonus) * self.first_share()
    }
    /// Keeps year one's cash at or under `limit` by moving signing bonus
    /// into guaranteed base, which `build` pays out in the later years. The
    /// guarantee stays the same.
    pub fn limit_first_year(&mut self, limit: f64) {
        let total = self.total();
        if self.years <= 1 || total <= 0.0 || self.first_year_cash() <= limit {
            return;
        }
        let f1 = self.first_share();
        let bonus = ((limit - total * f1) / (1.0 - f1)).clamp(0.0, total * self.bonus_share);
        self.base_guarantee += self.bonus_share - bonus / total;
        self.bonus_share = bonus / total;
    }
}

/// The year's day injury guarantees for the next league year vest: the
/// fifth day of the league year, as most real deals write it (proposed, not
/// confirmed: real deals vary from the first to the fifth day).
pub fn vest_day(league_year: i32) -> Date {
    Date::ymd(league_year, 3, 16)
}

/// Lays a deal out by year: the bonus prorated over at most five years
/// (void years included), the base ramped, guarantees on the first years.
pub fn build(team: usize, kind: Kind, start: i32, t: &Terms) -> Contract {
    let n = t.years.max(1) as usize;
    let total = t.total();
    let bonus = total * t.bonus_share;
    let pay = total - bonus;
    // Each year's pay, ramped: weights 1, 1+r, (1+r)^2 ...
    let w: Vec<f64> = (0..n).map(|k| (1.0 + t.ramp).powi(k as i32)).collect();
    let wsum: f64 = w.iter().sum();
    // A low first year: its pay as written, the rest ramped over the others.
    let first = (t.first_base > 0.0 && n > 1).then(|| t.first_base.min(pay));
    let rest: f64 = w[1..].iter().sum();
    let mut years: Vec<Year> = Vec::with_capacity(n + t.void_years as usize);
    let mut left_g = total * t.base_guarantee;
    let mut left_i = total * t.injury_guarantee;
    for k in 0..n {
        let yr_pay = match first {
            Some(f) if k == 0 => f,
            Some(f) => (pay - f) * w[k] / rest,
            None => pay * w[k] / wsum,
        };
        let roster = yr_pay * t.roster_share;
        let workout = yr_pay * t.workout_share;
        let per_game = yr_pay * t.per_game_share;
        let base = yr_pay - roster - workout - per_game;
        let g = left_g.min(base);
        left_g -= g;
        let i = left_i.min(base - g);
        left_i -= i;
        years.push(Year {
            year: start + k as i32,
            base,
            guaranteed: g,
            injury: i,
            vests: (i > 0.0).then(|| vest_day(start + k as i32)),
            roster_bonus: roster,
            workout,
            per_game,
            ltbe: 0.0,
            nltbe: t.incentives,
            proration: 0.0,
            void: false,
            bonus_paid: 0.0,
        });
    }
    for k in 0..t.void_years as i32 {
        years.push(Year { year: start + n as i32 + k, void: true, ..Default::default() });
    }
    let spread = years.len().min(5);
    for y in years.iter_mut().take(spread) {
        y.proration = bonus / spread as f64;
    }
    if let Some(y) = years.first_mut() {
        y.bonus_paid = bonus;
    }
    Contract {
        team,
        kind,
        signed: start,
        years,
        signing_bonus: bonus,
        no_trade: t.no_trade,
        offsets: t.offsets,
        option: None,
        new_from: start,
        clauses: Vec::new(),
    }
}

/// The most a deal pays in its first year, bonus and pay, as a share of
/// that year's cap: Prescott's 2021 extension paid $75M against a $182.5M
/// cap, 41% (the real high; his 2024 deal paid $81.25M, 32%).
pub const FIRST_YEAR_MAX: f64 = 0.41;

/// The highest cap hit a club lets a restructure build in a later year, as
/// a share of that year's cap: Watson's 2025 hit, $72.9M against $279.2M,
/// 26%, is the real high (Prescott 2024 22%).
pub const CAP_HIT_MAX: f64 = 0.26;

/// A restructure (the Saints' habit, topic 7): this year's base above the
/// minimum turns into a signing bonus spread over the years left (void
/// years added when few are left). Saves cap now, costs it later.
pub fn restructure(c: &mut Contract, y: i32, keep: f64, add_void: u32) -> f64 {
    let Some(k) = c.years.iter().position(|x| x.year == y && !x.void) else { return 0.0 };
    let amount = (c.years[k].base - keep).max(0.0);
    if amount <= 0.0 {
        return 0.0;
    }
    // Void years only fill the spread up to its five-year limit (real).
    let have = c.years.iter().filter(|x| x.year >= y).count() as u32;
    let add_void = add_void.min(5u32.saturating_sub(have));
    let last = c.years.iter().map(|x| x.year).max().unwrap_or(y);
    for v in 0..add_void as i32 {
        c.years.push(Year { year: last + 1 + v, void: true, ..Default::default() });
    }
    c.years[k].base -= amount;
    c.years[k].bonus_paid += amount;
    c.years[k].guaranteed = c.years[k].guaranteed.max(0.0).min(c.years[k].base);
    // The converted pay is a bonus now: an injury guarantee covers only the base left.
    c.years[k].injury = c.years[k].injury.max(0.0).min(c.years[k].base - c.years[k].guaranteed);
    let spread: Vec<usize> = (k..c.years.len()).take(5).collect();
    for &j in &spread {
        c.years[j].proration += amount / spread.len() as f64;
    }
    c.signing_bonus += amount;
    let saved = amount - amount / spread.len() as f64;
    saved.max(0.0)
}

// --- Market prices ---------------------------------------------------------------------

/// The first and last league years of the real position markets.
pub const REAL_FROM: i32 = 1994;
pub const REAL_TO: i32 = 2026;

/// The real top five veteran deals at each position as a share of the cap,
/// league year by league year (realism audit item 3, Cain 2026-11-15: "make
/// it mirror real life"). Each year is the five biggest average-a-year
/// deals in force that season over that season's cap, the way OverTheCap
/// lists a position (2025: Prescott $60M, Allen, Burrow, Lawrence and Love
/// $55M on a $279.2M cap, 0.201). From OverTheCap's records of 53,739
/// deals (nflverse `historical_contracts`, fetched 2026-11-15; the table
/// and its script are `data/real-world/nfl_top5_shares.csv` and
/// `scripts/nfl_top5_shares.py`). Its records thin out going back: a year
/// with fewer than 40 deals in force at a position is the average of
/// itself and its neighbours, and before a position's first year with 10
/// deals (2009 for kickers, 2012 for punters, 2013 for snappers) its first
/// such year holds. Some real moves: running backs 0.114 in 1999 (Barry
/// Sanders and Emmitt Smith at 14-16% each), 0.089 in 2011 (Peterson's
/// 11.4% deal), 0.058 in 2025; quarterbacks 0.122 in 2007, 0.232 in 2023;
/// receivers 0.078 in 2010, 0.128 in 2022. Order: `Position::ALL`.
#[rustfmt::skip]
pub const REAL_TOP5: [[f64; POSITION_COUNT]; 33] = [
    // QB     RB     WR     TE     OT    IOL   EDGE     DT     LB     CB      S      K      P     LS
    [0.1525, 0.1010, 0.0702, 0.0449, 0.0875, 0.0457, 0.0788, 0.0924, 0.0848, 0.1031, 0.0413, 0.0180, 0.0297, 0.0084], // 1994
    [0.1532, 0.1010, 0.0702, 0.0449, 0.0875, 0.0457, 0.0788, 0.0924, 0.0848, 0.1031, 0.0413, 0.0180, 0.0297, 0.0084], // 1995
    [0.1609, 0.1010, 0.0702, 0.0449, 0.0875, 0.0457, 0.0788, 0.0924, 0.0848, 0.1031, 0.0413, 0.0180, 0.0297, 0.0084], // 1996
    [0.1596, 0.1052, 0.0702, 0.0449, 0.0875, 0.0457, 0.0788, 0.0924, 0.0848, 0.1031, 0.0413, 0.0180, 0.0297, 0.0084], // 1997
    [0.1572, 0.1113, 0.0702, 0.0449, 0.0875, 0.0457, 0.0788, 0.0924, 0.0848, 0.1031, 0.0413, 0.0180, 0.0297, 0.0084], // 1998
    [0.1448, 0.1143, 0.0762, 0.0449, 0.0875, 0.0457, 0.0788, 0.0924, 0.0848, 0.1096, 0.0413, 0.0180, 0.0297, 0.0084], // 1999
    [0.1448, 0.1104, 0.0831, 0.0449, 0.0875, 0.0457, 0.0788, 0.0924, 0.0835, 0.1135, 0.0413, 0.0180, 0.0297, 0.0084], // 2000
    [0.1425, 0.1027, 0.0908, 0.0449, 0.0875, 0.0457, 0.0788, 0.0908, 0.0794, 0.1155, 0.0413, 0.0180, 0.0297, 0.0084], // 2001
    [0.1407, 0.0975, 0.0914, 0.0449, 0.0875, 0.0435, 0.0801, 0.0832, 0.0750, 0.1043, 0.0413, 0.0180, 0.0297, 0.0084], // 2002
    [0.1408, 0.0925, 0.0944, 0.0449, 0.0875, 0.0408, 0.0848, 0.0749, 0.0736, 0.0997, 0.0413, 0.0180, 0.0297, 0.0084], // 2003
    [0.1459, 0.0898, 0.0974, 0.0449, 0.0846, 0.0374, 0.0879, 0.0670, 0.0735, 0.0965, 0.0400, 0.0180, 0.0297, 0.0084], // 2004
    [0.1560, 0.0846, 0.1021, 0.0440, 0.0774, 0.0404, 0.0874, 0.0657, 0.0705, 0.0923, 0.0395, 0.0180, 0.0297, 0.0084], // 2005
    [0.1307, 0.0803, 0.0884, 0.0447, 0.0705, 0.0482, 0.0816, 0.0659, 0.0641, 0.0766, 0.0408, 0.0180, 0.0297, 0.0084], // 2006
    [0.1224, 0.0745, 0.0892, 0.0444, 0.0677, 0.0417, 0.0869, 0.0639, 0.0604, 0.0748, 0.0425, 0.0180, 0.0297, 0.0084], // 2007
    [0.1216, 0.0712, 0.0852, 0.0469, 0.0704, 0.0575, 0.0917, 0.0668, 0.0612, 0.0824, 0.0438, 0.0180, 0.0297, 0.0084], // 2008
    [0.1249, 0.0671, 0.0803, 0.0496, 0.0836, 0.0542, 0.1046, 0.0699, 0.0637, 0.0866, 0.0435, 0.0180, 0.0297, 0.0084], // 2009
    [0.1329, 0.0657, 0.0778, 0.0556, 0.0873, 0.0613, 0.1022, 0.0930, 0.0699, 0.0902, 0.0587, 0.0226, 0.0297, 0.0084], // 2010
    [0.1388, 0.0892, 0.0942, 0.0593, 0.0917, 0.0673, 0.1061, 0.1048, 0.0802, 0.0904, 0.0666, 0.0277, 0.0297, 0.0084], // 2011
    [0.1517, 0.0896, 0.1042, 0.0630, 0.0915, 0.0701, 0.1126, 0.1002, 0.0801, 0.0911, 0.0673, 0.0277, 0.0294, 0.0084], // 2012
    [0.1659, 0.0878, 0.1112, 0.0618, 0.0859, 0.0684, 0.1120, 0.0912, 0.0786, 0.0987, 0.0680, 0.0270, 0.0283, 0.0082], // 2013
    [0.1535, 0.0812, 0.1028, 0.0617, 0.0815, 0.0653, 0.1061, 0.0973, 0.0726, 0.1105, 0.0652, 0.0258, 0.0267, 0.0079], // 2014
    [0.1501, 0.0808, 0.1055, 0.0604, 0.0815, 0.0624, 0.1037, 0.1069, 0.0738, 0.1033, 0.0626, 0.0250, 0.0246, 0.0075], // 2015
    [0.1480, 0.0746, 0.0982, 0.0604, 0.0804, 0.0652, 0.1071, 0.1106, 0.0679, 0.0941, 0.0683, 0.0252, 0.0227, 0.0072], // 2016
    [0.1469, 0.0603, 0.0965, 0.0562, 0.0790, 0.0674, 0.1023, 0.1031, 0.0692, 0.0908, 0.0716, 0.0240, 0.0212, 0.0068], // 2017
    [0.1648, 0.0657, 0.0947, 0.0524, 0.0818, 0.0703, 0.1059, 0.1044, 0.0652, 0.0830, 0.0686, 0.0237, 0.0197, 0.0065], // 2018
    [0.1785, 0.0677, 0.1002, 0.0499, 0.0855, 0.0691, 0.1115, 0.0987, 0.0826, 0.0771, 0.0732, 0.0239, 0.0190, 0.0063], // 2019
    [0.1882, 0.0742, 0.1095, 0.0610, 0.1007, 0.0718, 0.1184, 0.1010, 0.0787, 0.0908, 0.0721, 0.0233, 0.0180, 0.0061], // 2020
    [0.2214, 0.0806, 0.1189, 0.0768, 0.1172, 0.0879, 0.1364, 0.1123, 0.0967, 0.1017, 0.0856, 0.0257, 0.0198, 0.0070], // 2021
    [0.2271, 0.0689, 0.1284, 0.0716, 0.1027, 0.0821, 0.1220, 0.1116, 0.0819, 0.0961, 0.0826, 0.0246, 0.0170, 0.0065], // 2022
    [0.2320, 0.0646, 0.1190, 0.0686, 0.1019, 0.0822, 0.1232, 0.1105, 0.0834, 0.0890, 0.0772, 0.0245, 0.0154, 0.0069], // 2023
    [0.2178, 0.0592, 0.1261, 0.0625, 0.1055, 0.0783, 0.1139, 0.1094, 0.0718, 0.0857, 0.0734, 0.0231, 0.0127, 0.0060], // 2024
    [0.2006, 0.0582, 0.1252, 0.0612, 0.0990, 0.0819, 0.1491, 0.0969, 0.0700, 0.0954, 0.0749, 0.0212, 0.0134, 0.0058], // 2025
    [0.1919, 0.0686, 0.1246, 0.0604, 0.0954, 0.0817, 0.1478, 0.1131, 0.0692, 0.1055, 0.0734, 0.0219, 0.0130, 0.0059], // 2026
];

/// A real year's top-five share (`REAL_TOP5`): the first year before 1994,
/// the last after 2026 (the world's own market moves it from there:
/// `market::top5`).
pub fn real_top5(year: i32, pos: Position) -> f64 {
    REAL_TOP5[(year.clamp(REAL_FROM, REAL_TO) - REAL_FROM) as usize][pos.index()]
}

/// What a veteran of this grade costs a year on the open market, as a share
/// of the cap, given the position's top-five share this year: the minimum
/// at 66, the top five at 90, and on past it with no end (realism audit
/// items 1 and 4, Cain 2026-11-15): the grade rises with how far he stands
/// above the market's best (`market::market_grade`), so a new record is as
/// big as his edge and the clubs bidding for him make it. Before, the curve
/// stopped at 1.08 times the top five (about grade 91.2), so a 98 cost what
/// a 92 did; real records jumped far past that: Mahomes' 2020 deal ($45M)
/// was 1.29 times Wilson's record, 1.34 times the top five.
pub fn market_share(top5: f64, grade: f64) -> f64 {
    let min = 0.004;
    min + (top5 - min) * ((grade - 66.0) / 24.0).max(0.0).powf(1.6)
}

/// Contract length a team offers by age and grade (proposed, not
/// confirmed: stars in their prime four or five years, a 30-year-old two,
/// a fringe player one).
pub fn usual_years(pos: Position, grade: f64, age: u32) -> u32 {
    let prime_end = match pos {
        Position::RB => 27,
        Position::QB | Position::K | Position::P | Position::LS => 33,
        Position::OT | Position::IOL => 30,
        _ => 29,
    };
    let left = prime_end - age as i32;
    let base = if grade >= 82.0 {
        5
    } else if grade >= 76.0 {
        3
    } else if grade >= 70.0 {
        2
    } else {
        1
    };
    clamp((base.min(left + 2)) as f64, 1.0, 5.0) as u32
}

// --- Rookie deals ----------------------------------------------------------------------

/// A drafted rookie's deal: the slot's money under the era's rules. From
/// 2011 the rookie scale fixes four years at every pick (round one has a
/// fifth-year option) at about 5.1% of the cap a year for the first pick,
/// 1.5% at the end of round one and 0.4% in round seven. Before 2011 picks
/// were negotiated inside each team's rookie pool, and the top of round one
/// cost about twice as much (real: the first pick at about 8% in 1995 and
/// 10% in 2008-10), with six-year deals at the top of the draft, five in
/// the rest of round one, three or four after (proposed, not confirmed:
/// from the real top deals).
pub fn rookie_terms(year: i32, pick: u32, round: u32) -> Terms {
    let cap = price_cap(year);
    let scale = rookie_scale_share(pick);
    let (share, years) = if year >= 2011 {
        (scale, 4)
    } else {
        let p = pick as f64;
        let top = if p <= 32.0 { 1.0 + 1.0 * (1.0 - (p - 1.0) / 31.0) } else { 1.0 };
        let years = match (pick, round) {
            (1..=16, _) => 6,
            (_, 1) => 5,
            (_, 2 | 3) => 4,
            _ => 3,
        };
        (scale * top, if year < 1994 { years.min(4) } else { years })
    };
    let apy = (cap * share).max(minimum(year, 0));
    // Signing bonus and guarantees by round (real 2025 shape: first
    // picks about 65% bonus and fully guaranteed, late first-rounders
    // partly, day-two picks a bonus and some year-two money, day three a
    // small bonus).
    let (bonus, guar) = match round {
        1 if pick <= 20 => (0.65, 0.35),
        1 => (0.55, 0.2),
        2 => (0.35, 0.1),
        3 => (0.2, 0.0),
        _ => (0.08, 0.0),
    };
    let mut t = Terms::flat(years, apy);
    t.bonus_share = bonus;
    t.base_guarantee = guar;
    t.ramp = 0.1;
    t.offsets = round == 1;
    t
}

/// The 2026 rookie scale's yearly value as a share of the cap: a straight
/// line through the real slots.
pub fn rookie_scale_share(pick: u32) -> f64 {
    let p = pick as f64;
    if p <= 32.0 {
        0.0513 - (0.0513 - 0.0150) * (p - 1.0) / 31.0
    } else if p <= 64.0 {
        0.0120 - (0.0120 - 0.0058) * (p - 33.0) / 31.0
    } else {
        (0.0058 - 0.0018 * (p - 64.0) / 193.0).max(0.004)
    }
}

/// An undrafted rookie's deal: three years at the minimum and a signing
/// bonus a team uses to win him in the rush (real 2025: mostly $5K-$25K,
/// up to about $250K for the most wanted).
pub fn undrafted_terms(year: i32, wanted: f64) -> Terms {
    let min = minimum(year, 0);
    let bonus = price_cap(year) * 0.0009 * clamp(wanted, 0.0, 1.0).powi(2) + price_cap(year) * 0.00002;
    let mut t = Terms::flat(3, min + bonus / 3.0);
    t.bonus_share = bonus / (3.0 * t.apy);
    t
}

/// An undrafted rookie's deal from the rush (section 7, topic 12): three
/// years at the minimum, the bonus from the club's pool and any base
/// salary it guarantees on top (real: no limit on guarantees).
pub fn undrafted_offer(year: i32, bonus: f64, guarantee: f64) -> Terms {
    let min = minimum(year, 0);
    let mut t = Terms::flat(3, min + bonus / 3.0);
    t.bonus_share = bonus / (3.0 * t.apy);
    t.base_guarantee = guarantee / (3.0 * t.apy);
    t
}

// --- Tags and tenders ----------------------------------------------------------------------

/// A tag's price: the average of the top five (franchise) or top ten
/// (transition) cap numbers at the position, or 120% of his own last pay
/// if higher. A second tag costs 120% of the last tag, a third 144%
/// (or the quarterback price if higher).
pub fn tag_price(top: &[f64], franchise: bool, own: f64, times: u32, qb_price: f64) -> f64 {
    let k = if franchise { 5 } else { 10 };
    let mut v: Vec<f64> = top.to_vec();
    v.sort_by(|a, b| b.total_cmp(a));
    let avg = v.iter().take(k).sum::<f64>() / v.len().clamp(1, k) as f64;
    match times {
        0 => avg.max(own * 1.2),
        1 => avg.max(own * 1.2),
        _ => (own * 1.44).max(qb_price).max(avg),
    }
}

/// A tag's price from the formula's base: at least 120% of his own last
/// pay; a second tag 120% of the last, a third 144% (or the quarterback
/// price if higher).
pub fn tag_repeat(base: f64, own: f64, times: u32, qb_price: f64) -> f64 {
    match times {
        0 | 1 => base.max(own * 1.2),
        _ => (own * 1.44).max(qb_price).max(base),
    }
}

/// A restricted free agent's tender (2025 shares of the cap: first round
/// $7.46M, second $5.35M, the original round $3.26M).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Tender {
    First,
    Second,
    Original,
    RightOfFirstRefusal,
}

pub fn tender_price(year: i32, t: Tender) -> f64 {
    let share = match t {
        Tender::First => 0.0267,
        Tender::Second => 0.0191,
        Tender::Original | Tender::RightOfFirstRefusal => 0.0117,
    };
    price_cap(year) * share
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minor bug 270 and realism audit item 4: the top of the pay ladder
    /// pays past the top five's average and keeps rising with no end.
    #[test]
    fn the_best_at_a_position_can_pass_the_top_five() {
        for pos in [Position::QB, Position::EDGE, Position::WR] {
            let top = real_top5(2026, pos);
            assert!((market_share(top, 90.0) - top).abs() < 1e-9);
            assert!(market_share(top, 91.0) > top);
            assert!(market_share(top, 95.0) > top * 1.2);
            assert!(market_share(top, 99.0) > market_share(top, 95.0));
        }
    }

    /// Realism audit item 3: the markets are each year's real ones.
    #[test]
    fn position_markets_follow_their_era() {
        assert!((real_top5(2025, Position::QB) - 0.2006).abs() < 1e-9);
        assert!(real_top5(1999, Position::RB) > 0.11 && real_top5(2025, Position::RB) < 0.06);
        assert!(real_top5(1980, Position::QB) == real_top5(1994, Position::QB));
        assert!(real_top5(2040, Position::WR) == real_top5(2026, Position::WR));
    }

    #[test]
    fn deals_charge_the_cap_as_written() {
        let mut t = Terms::flat(4, 20.0e6);
        t.bonus_share = 0.4;
        t.base_guarantee = 0.1;
        t.void_years = 1;
        let c = build(3, Kind::Veteran, 2024, &t);
        assert_eq!(c.real_years(), 4);
        assert!((c.total() - 80.0e6).abs() < 1.0);
        assert!((c.apy() - 20.0e6).abs() < 1.0);
        // The 32M bonus over five years, the void year included.
        assert!((c.years[0].proration - 6.4e6).abs() < 1.0);
        let cap: f64 = c.years.iter().map(Year::cap_hit).sum();
        assert!((cap - 80.0e6).abs() < 1.0);
        let today = Date::ymd(2025, 3, 1);
        let (now, next) = c.dead_if_cut(2025, today, false);
        assert!((now - 4.0 * 6.4e6).abs() < 1.0, "{now}");
        assert_eq!(next, 0.0);
        let (now, next) = c.dead_if_cut(2025, today, true);
        assert!((now - 6.4e6).abs() < 1.0 && (next - 3.0 * 6.4e6).abs() < 1.0);
        assert!(price_cap(1990) < price_cap(1994) && price_cap(2027) > price_cap(2026));
        assert!(minimum(2025, 0) > 800_000.0 && minimum(2025, 7) > minimum(2025, 0));
    }
}
