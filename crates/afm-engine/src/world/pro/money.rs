//! Section 5 topic 2: money. The cap by year, minimum salaries, real
//! contract structure (base salary, signing bonus proration, guarantees,
//! roster, workout and per-game bonuses, incentives, void years) with each
//! year's cap hit and the dead money a release leaves, the rookie scale
//! (2011 on) and the rookie pool before it, and the tags and tenders.

use crate::calendar::Date;
use crate::positions::Position;
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

/// The league minimum by accrued seasons, as the 2025 scale's shares of the
/// cap (real 2025: $840K with none, $960K, $1.03M, $1.1M, $1.17M for 4-6,
/// $1.255M for 7 or more), applied to every year.
pub fn minimum(year: i32, accrued: u32) -> f64 {
    let share = match accrued {
        0 => 0.00301,
        1 => 0.00344,
        2 => 0.00369,
        3 => 0.00394,
        4..=6 => 0.00419,
        _ => 0.0045,
    };
    (price_cap(year) * share / 5_000.0).round() * 5_000.0
}

/// A practice squad salary a week, about a third of the rookie minimum's
/// weekly pay (real 2025: $14,000 a week against $49,400).
pub fn practice_squad_week(year: i32) -> f64 {
    minimum(year, 0) / 17.0 * 0.28
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug, Default)]
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

#[derive(Clone, Debug)]
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
#[derive(Clone, Copy, Debug)]
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
    let mut years: Vec<Year> = Vec::with_capacity(n + t.void_years as usize);
    let mut left_g = total * t.base_guarantee;
    let mut left_i = total * t.injury_guarantee;
    for k in 0..n {
        let yr_pay = pay * w[k] / wsum;
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
    }
}

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
    let spread: Vec<usize> = (k..c.years.len()).take(5).collect();
    for &j in &spread {
        c.years[j].proration += amount / spread.len() as f64;
    }
    c.signing_bonus += amount;
    let saved = amount - amount / spread.len() as f64;
    saved.max(0.0)
}

// --- Market prices ---------------------------------------------------------------------

/// The top five veteran deals at each position as a share of the cap
/// (2025-26, OverTheCap averages; shared with the draft board).
pub fn top5_share(pos: Position) -> f64 {
    match pos {
        Position::QB => 0.234,
        Position::EDGE => 0.158,
        Position::WR => 0.136,
        Position::DT => 0.117,
        Position::CB => 0.108,
        Position::OT => 0.0985,
        Position::S => 0.081,
        Position::IOL => 0.077,
        Position::LB => 0.075,
        Position::RB => 0.074,
        Position::TE => 0.063,
        Position::K => 0.023,
        Position::P => 0.015,
        Position::LS => 0.007,
    }
}

/// What a veteran of this grade costs a year on the open market, as a share
/// of the cap: the minimum at 66 rising to the top five at 90.
pub fn market_share(pos: Position, grade: f64) -> f64 {
    let min = 0.004;
    min + (top5_share(pos) - min) * clamp((grade - 66.0) / 24.0, 0.0, 1.0).powf(1.6)
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
