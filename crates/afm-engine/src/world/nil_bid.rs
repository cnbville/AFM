//! The new NIL scheme's market (Cain 2026-11-15, "agree on all"; lane A,
//! items 5-11 and 13; spec `afm/nil-scheme/spec.md`). Every player is
//! priced by bidding, not by a going-rate top: each school that wants him
//! has a top bid, its whole NIL budget × his spot's share × his talent, and
//! he is paid what wins him (`price`): the best rival's top bid, never past
//! his own school's. A school keeps its starters the same way before the
//! January window (`keep_offers`). NIL Go reviews each third-party deal
//! against what similar players cleared (`NilBook`, `review`), and money it
//! denies moves into football's revenue share where the cap leaves room.
//! Freaks are never capped: a talent's bid has no ceiling but the
//! school's own money.

use super::data::Division;
use super::program::Program;
use super::{Person, Stage, World};
use crate::calendar::Date;
use crate::player::PlayingTime;
use crate::positions::{POSITION_COUNT, Position};
use rayon::prelude::*;

/// A median starter's share of a school's whole NIL budget by his spot
/// (item 6): The NIL Standard's 2026 Power 4 starter medians over a $25M
/// roster (QB $2M, EDGE $929K, OT $907K, WR $726K, CB $572K, RB $527K, IOL
/// $525K; defensive tackles from ESPN's and 247Sports' $500K-1M). Safeties,
/// linebackers, tight ends and the specialists are Claude's estimates (no
/// Standard figure), long snappers a punter's. A starting lineup at these
/// shares takes about 62% of a budget.
pub fn spot_share(pos: Position) -> f64 {
    match pos {
        Position::QB => 0.08,
        Position::EDGE => 0.037,
        Position::OT => 0.036,
        Position::WR => 0.029,
        Position::DT => 0.028,
        Position::CB => 0.023,
        Position::RB | Position::IOL => 0.021,
        Position::S | Position::LB => 0.018,
        Position::TE => 0.016,
        Position::K => 0.003,
        Position::P | Position::LS => 0.002,
    }
}

/// A rotation player's and a backup's bid against a starter's at his spot
/// (proposed, not confirmed: fitted so the rotation takes about 11% of a
/// budget, as the scheme's check reads it).
pub const BID_ROTATION: f64 = 0.22;
pub const BID_DEPTH: f64 = 0.04;
/// Each point of play over the level's starter median at his spot moves his
/// price 3% (item 7), with no top: the old +15 clamp is gone, the −15
/// floor stays.
pub const BID_PLAY: f64 = 0.03;
pub const BID_FLOOR: f64 = -15.0;

/// A recruit's talent from his stars (item 8), before the chance he plays
/// in his first two years (proposed, not confirmed: fitted on seed 41's
/// class of 2027 so a No. 1 quarterback at a $50M school draws about $2.4M,
/// a typical five-star $750K-1M, a four-star receiver $100-250K at the top
/// programs and a top class $6-9M; measured, Power 4 five-stars at a median
/// $717K (mean $771K), four-star receivers at the 15 richest $132K, the
/// five-star quarterbacks at Florida State and Tennessee, $46M, $2.5-2.6M,
/// the top class $8.26M).
pub fn star_bid(stars: u8) -> f64 {
    match stars {
        5 => 0.8,
        4 => 0.25,
        3 => 0.07,
        2 => 0.02,
        _ => 0.006,
    }
}

/// [`star_bid`] on a smooth scale (`x` stars, 1 to 5), between the whole
/// stars on a log scale: the talent of a recruit whose stars are read
/// against a school's level (`star_at`).
pub fn star_bid_at(x: f64) -> f64 {
    let x = x.clamp(1.0, 5.0);
    let lo = x.floor();
    let t = x - lo;
    let a = star_bid(lo as u8).ln();
    let b = star_bid((lo as u8 + 1).min(5)).ln();
    (a + t * (b - a)).exp()
}

/// A recruit's stars as a school at money level `l` reads them (the
/// Group of 5 signee fix, 2026-11-18): his stars plus the level's lift,
/// how far its starters' stars sit under the Power 4's
/// ([`NilBook::star_lift`]). [`star_bid`] is a Power 4 scale, a recruit's
/// price against a median Power 4 starter, so read nationally a Group of 5
/// school priced its three-star signee, its future starter, at 7% of its
/// starters' money, where a Power 4 school pays its four-star, its own
/// future starter, 25%: the same mistake 2a fixed for players with a
/// season (`recruiting::bid_lifts`).
pub fn star_at(w: &World, l: usize, stars: u8) -> f64 {
    star_bid_at(stars.max(1) as f64 + w.nil_book.star_lift[l.min(3)])
}

/// Each money level's star lift (`star_at`): the mean recruiting stars of
/// the Power 4's college starters less the level's own, never below 0.
pub fn star_lifts(w: &World) -> [f64; 4] {
    let mut sum = [(0.0f64, 0usize); 4];
    for p in w.people.iter() {
        if p.stage == Stage::College
            && p.last_role == Some(PlayingTime::Starter)
            && !p.player.position.is_specialist()
            && let Some(c) = p.college
        {
            let prog = &w.programs[c];
            if prog.collective.budget <= 0.0 && !matches!(prog.division, Division::FBS | Division::FCS) {
                continue;
            }
            let s = &mut sum[level(prog)];
            s.0 += p.stars.max(1) as f64;
            s.1 += 1;
        }
    }
    let mean = |l: usize| (sum[l].1 >= 50).then(|| sum[l].0 / sum[l].1 as f64);
    let mut lift = [0.0; 4];
    if let Some(top) = mean(0) {
        for (l, x) in lift.iter_mut().enumerate().skip(1) {
            *x = mean(l).map_or(0.0, |m| (top - m).max(0.0));
        }
    }
    lift
}

/// His talent as the market prices it: a recruit's stars (None: no college
/// season yet), else his role and his play over the level's starters at his
/// spot (`edge`, points).
pub fn bid_talent(role: Option<PlayingTime>, stars: u8, edge: f64) -> f64 {
    let play = (BID_PLAY * edge.max(BID_FLOOR)).exp();
    match role {
        None => star_bid(stars),
        Some(PlayingTime::Starter) => play,
        Some(PlayingTime::Rotation) => BID_ROTATION * play,
        _ => BID_DEPTH * play,
    }
}

/// A school's top bid for a player (item 6): its whole NIL budget × his
/// spot's share × his talent.
pub fn top_bid(budget: f64, pos: Position, talent: f64) -> f64 {
    budget.max(0.0) * spot_share(pos) * talent
}

/// What a lone bidder offers against its top bid: with no rival bid in
/// sight a school still pays a man it wants this much of what it would go
/// to (proposed, not confirmed).
pub const BID_OPEN: f64 = 0.5;

/// What wins him (item 5): the best rival's top bid, never past this
/// school's own, and with no rival near it this school's opening.
pub fn price(own: f64, rival: f64) -> f64 {
    own.min(rival.max(BID_OPEN * own))
}

/// The chance a recruit plays in his first two years at a school (item 8),
/// from its depth at his spot: `ahead` players there two years on who are
/// better than he will be, `slots` starting places. A place open: 1; else
/// his share of the places among the men he must beat.
pub fn play_chance(ahead: usize, slots: usize) -> f64 {
    (slots.max(1) as f64 / (ahead as f64 + 1.0)).min(1.0)
}

/// A program's money level for NIL Go's comparisons: the Power 4, the rest
/// of FBS, FCS, below.
pub fn level(prog: &Program) -> usize {
    match prog.division {
        Division::FBS if prog.power_four() => 0,
        Division::FBS => 1,
        Division::FCS => 2,
        _ => 3,
    }
}

/// The share of a level's cleared deals NIL Go's range reaches (item 10):
/// a deal priced past what this share of similar players cleared, per unit
/// of their payroll weight, is denied. Aimed at about 8% of reviewed deals
/// (real May-June 2026: 659 of 8,298 deals, $33.7M of $146.6M); on seed 41
/// it denies 3.7% of the year's reviewed deals and 7% of their dollars
/// (`nil_scheme run`, the year to July 2027): under real. Proposed, not
/// confirmed.
pub const NIL_GO_Q: f64 = 0.92;

/// What NIL Go's range adds for each follower he has (B18): the same $1.20
/// the public NIL value counts (proposed, not confirmed).
pub const NIL_GO_FOLLOWER: f64 = 1.2;

/// The market NIL Go reads (item 10): per money level, the top of what
/// similar players cleared from third parties last July, per unit of their
/// weight in a collective's payroll (`recruiting::nil_weight` in two pots:
/// his position, role, stars and play against the level's starters), the
/// year it was read.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NilBook {
    pub year: i32,
    pub top: [f64; 4],
    pub n: [u32; 4],
    /// Each money level's star lift (`star_lifts`), read each July 1.
    #[serde(default)]
    pub star_lift: [f64; 4],
    /// The share of a year's money each money level's AI schools paid
    /// their arrivals (signees and transfers on the deals they came on)
    /// last July 1 (`recruiting::nil_payouts`): the room your desk's roster
    /// split keeps for a class and transfers not yet signed
    /// (`front_office::college::splits`).
    #[serde(default)]
    pub new_share: [f64; 4],
}

impl NilBook {
    /// Read from (level, cleared third-party money / his payroll weight)
    /// for each deal; a level with too few deals reads its programs' pots.
    fn read(year: i32, deals: Vec<(usize, f64)>, pots: [Vec<f64>; 4]) -> NilBook {
        let mut by: [Vec<f64>; 4] = Default::default();
        for (l, x) in deals {
            if x.is_finite() && x > 0.0 {
                by[l].push(x);
            }
        }
        let mut book = NilBook { year, ..Default::default() };
        for l in 0..4 {
            let v = if by[l].len() >= 200 { &mut by[l] } else { &mut pots[l].clone() };
            v.sort_by(|a, b| a.total_cmp(b));
            book.n[l] = v.len() as u32;
            book.top[l] = if v.is_empty() { 0.0 } else { v[((v.len() - 1) as f64 * NIL_GO_Q).round() as usize] };
        }
        book
    }

    /// The book read from the programs' third-party pots alone (a world
    /// from before the scheme, or NIL Go's first July): each program's pot
    /// over its roster's payroll weight, what its July split pays a unit.
    pub fn from_pots(w: &World) -> NilBook {
        NilBook { star_lift: star_lifts(w), ..NilBook::read(w.today.year(), Vec::new(), pot_rates(w)) }
    }
}

/// Each program's third-party pot per unit of its roster's payroll weight
/// (walk-ons left out), by money level.
fn pot_rates(w: &World) -> [Vec<f64>; 4] {
    let medians = super::recruiting::nil_medians(w);
    let mut pots: [Vec<f64>; 4] = Default::default();
    for prog in w.programs.iter().filter(|p| p.collective.budget > 0.0) {
        let units: f64 = prog
            .roster
            .iter()
            .map(|&i| &w.people[i])
            .filter(|p| !p.walk_on)
            .map(|p| super::recruiting::nil_value(p, prog.division, &medians, true))
            .sum();
        if units > 0.0 {
            pots[level(prog)].push((prog.collective.budget - prog.collective.school).max(0.0) / units);
        }
    }
    pots
}

/// The top of NIL Go's range for a third-party deal at program `j` (item
/// 10): what similar players cleared at his level per unit, grown with the
/// market since it was read, × his payroll weight there (`role`: None
/// before his first college season; `edge`: his play over the level's
/// starters at his spot), plus his following. Before NIL Go's first book
/// (its opening weeks in June 2025) it clears every deal.
pub fn range_top(w: &World, j: usize, p: &Person, role: Option<PlayingTime>, edge: f64) -> f64 {
    let book = &w.nil_book;
    let l = level(&w.programs[j]);
    if book.year == 0 || book.top[l] <= 0.0 {
        return f64::INFINITY;
    }
    let grow = super::ncaa::nil_market(w.today.year()) / super::ncaa::nil_market(book.year).max(1e-9);
    let weight =
        super::recruiting::nil_weight(role, p.player.position, super::recruiting::recruiting_stars(p), edge, true);
    book.top[l] * grow.max(1.0) * weight + NIL_GO_FOLLOWER * p.followers as f64
}

/// NIL Go on one third-party deal: what clears, what moves into the
/// school's revenue share, and whether it was reviewed and denied.
#[derive(Clone, Copy, Debug, Default)]
pub struct Review {
    pub cleared: f64,
    pub moved: f64,
    pub reviewed: bool,
    pub denied: bool,
}

/// NIL Go on `coll`, program `j`'s third-party deal with him (B18, items
/// 10 and 11): a reviewed deal past the top of his range is denied and
/// resubmitted at the top; the rest moves into football's revenue share
/// as far as the room under the cap goes (`room`), else it isn't paid
/// that year. `free`: sandbox mode's NIL Go switch.
pub fn review(today: Date, coll: f64, top: f64, room: f64, free: bool) -> Review {
    if free || !super::ncaa::nil_go_reviews(today, coll, coll) {
        return Review { cleared: coll, ..Default::default() };
    }
    let top = top.max(600.0);
    if coll <= top {
        return Review { cleared: coll, reviewed: true, ..Default::default() };
    }
    Review { cleared: top, moved: (coll - top).min(room.max(0.0)), reviewed: true, denied: true }
}

/// Counts one reviewed deal in the year's log (the fit's numbers).
pub fn count(w: &mut World, coll: f64, r: &Review) {
    count_in(&mut w.log, coll, r);
}

/// [`count`] on the log alone.
pub fn count_in(log: &mut super::CalendarLog, coll: f64, r: &Review) {
    if r.reviewed {
        log.nil_go_reviewed += 1;
        log.nil_go_reviewed_money += coll;
        if r.denied {
            log.nil_go_denied += 1;
            log.nil_go_denied_money += coll;
            log.nil_go_moved += r.moved;
        }
    }
}

/// A deal a player has for a coming money year (items 5 and 9): what he
/// signed for as a recruit, landed on as a transfer, or was offered to
/// stay. July 1 pays it.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NilDeal {
    /// The money year (from July 1 of it) it pays next.
    pub year: i32,
    /// What he gets a year: both pots, the third party's part as NIL Go
    /// cleared it.
    pub amount: f64,
    pub kind: DealKind,
    /// A multi-year deal's last money year (item 12); 0 for a one-year
    /// deal. Each July pays a year and moves `year` on until this one.
    #[serde(default)]
    pub through: i32,
}

impl NilDeal {
    /// The last money year it pays.
    pub fn last(&self) -> i32 {
        self.through.max(self.year)
    }

    /// Does it pay money year `y`?
    pub fn pays(&self, y: i32) -> bool {
        self.year <= y && y <= self.last()
    }

    /// A multi-year deal (item 12), with years still to pay.
    pub fn contract(&self) -> bool {
        self.through > 0
    }

    /// The money years still to pay from `year` on.
    pub fn years_left(&self) -> i32 {
        (self.last() - self.year + 1).max(0)
    }

    /// What leaving costs (item 12, the Big 12 template): half the fee
    /// still to pay, due within 30 days; nothing on a one-year deal.
    pub fn buyout(&self) -> f64 {
        if self.contract() { BUYOUT * self.amount * self.years_left() as f64 } else { 0.0 }
    }

    /// After July 1 of `paid`: the next year it pays, or None once done.
    pub fn after_payday(self, paid: i32) -> Option<NilDeal> {
        if self.year > paid {
            Some(self)
        } else if self.through > paid {
            Some(NilDeal { year: paid + 1, ..self })
        } else {
            None
        }
    }
}

/// Multi-year deals (the new NIL scheme, item 12; Cain 2026-11-15, "agree
/// on all"). Real since the House settlement (July 2025): most players
/// sign one year at a time (On3), the blue-chips and the top transfers a
/// school most fears losing are offered two to four, and a transfer out
/// owes a buyout (the Big 12's template: half the remaining fee within 30
/// days), which the school that takes him usually pays and counts on its
/// own cap (NCAA guidance). The sizes below are proposed, not confirmed,
/// fitted on seed 41 so about 5% of recruits sign one (`nil_scheme run`).
///
/// A recruit's talent (`star_bid` × his chance to play early) from which a
/// school offers him a term: a five-star who will play, a four-star who
/// starts early.
pub const TERM_TALENT: f64 = 0.15;
/// A transfer a school offers a term: a starter there, this many points
/// over the level's starters at his spot, with two seasons or more left.
pub const TERM_EDGE: f64 = 3.0;
/// No school guarantees small money for years: a term only on a deal of
/// this much a year or more (proposed, not confirmed).
pub const TERM_FLOOR: f64 = 100_000.0;
/// A recruit's term: three money years, to the draft he can first enter.
pub const TERM_RECRUIT: u8 = 3;
/// The longest term a transfer is offered (his seasons left, up to this).
pub const TERM_MOST: u8 = 3;
/// Later budgets are bound: a school's multi-year deals for any coming
/// money year stay under this share of its NIL budget now.
pub const TERM_BOUND: f64 = 0.35;
/// The buyout: half of what the deal still owes him (Big 12 template).
pub const BUYOUT: f64 = 0.5;

/// What each guaranteed year after the first is worth to him now, as a
/// share of a year's money (item 12: he weighs the guarantee by his
/// makeup). A family that needs the money wants it guaranteed, and so does
/// a loyal man who means to stay; an ambitious or vain one wants to sell
/// himself again after he breaks out; parents or a handler deciding lean
/// to the sure money. At or under 0 he signs for one year only. Proposed,
/// not confirmed.
pub fn guarantee(p: &Person) -> f64 {
    let t = &p.player.personality.traits;
    let need = super::background::hardship(&p.bg);
    let family = if p.family.decider == super::background::Decider::Himself { 0.0 } else { GUARANTEE_FAMILY };
    let x = GUARANTEE_BASE + GUARANTEE_NEED * need + GUARANTEE_TRAIT * (t.loyalty as f64 - 10.0)
        - GUARANTEE_TRAIT * (t.ambition as f64 - 10.0)
        - 0.7 * GUARANTEE_TRAIT * (t.ego as f64 - 10.0)
        + family;
    x.min(GUARANTEE_TOP)
}

pub const GUARANTEE_BASE: f64 = -0.06;
pub const GUARANTEE_NEED: f64 = 0.3;
pub const GUARANTEE_TRAIT: f64 = 0.015;
pub const GUARANTEE_FAMILY: f64 = 0.04;
pub const GUARANTEE_TOP: f64 = 0.3;

/// What a deal of `years` weighs with him against one year of the same
/// money: 1 + his guarantee × the years after the first; 1 when he'd
/// rather sign one year (and so he does).
pub fn term_weight(gv: f64, years: u8) -> f64 {
    1.0 + gv.max(0.0) * (years.max(1) - 1) as f64
}

/// The term he signs for when a school offers `offered` years: all of it
/// when he values the guarantee, else one year.
pub fn term_taken(gv: f64, offered: u8) -> u8 {
    if gv > 0.0 && offered >= 2 { offered } else { 1 }
}

/// A recruit's term from a school: `TERM_RECRUIT` years to a talent past
/// `TERM_TALENT`, in two pots only.
pub fn recruit_term(today: Date, talent: f64) -> u8 {
    if super::ncaa::two_pots(today) && talent >= TERM_TALENT { TERM_RECRUIT } else { 1 }
}

/// A transfer's term from a school: a starter there `TERM_EDGE` or more
/// over its level's starters at his spot, his seasons left up to
/// `TERM_MOST`, in two pots only.
pub fn transfer_term(today: Date, role: PlayingTime, edge: f64, left: u32) -> u8 {
    if super::ncaa::two_pots(today) && role == PlayingTime::Starter && edge >= TERM_EDGE && left >= 2 {
        (left as u8).min(TERM_MOST)
    } else {
        1
    }
}

/// Each program's multi-year money already promised for each coming money
/// year from `from` (index 0) on: the deals of the players it holds.
pub fn committed(w: &World, from: i32) -> Vec<[f64; 4]> {
    let mut v = vec![[0.0; 4]; w.programs.len()];
    for p in w.people.iter().filter(|p| p.stage == Stage::College || p.stage == Stage::HighSchool) {
        let (Some(d), Some(j)) =
            (p.cal.nil_deal.filter(|d| d.contract()), p.college.or(p.cal.signed.as_ref().map(|s| s.program)))
        else {
            continue;
        };
        for (k, y) in (from..from + 4).enumerate() {
            if d.pays(y) && j < v.len() {
                v[j][k] += d.amount;
            }
        }
    }
    v
}

/// The longest term up to `years` that keeps program `j`'s multi-year
/// money for each of its years under `TERM_BOUND` of its budget, with
/// `amount` a year more; 1 when even two years don't fit.
pub fn bound_term(budget: f64, held: &[f64; 4], amount: f64, years: u8) -> u8 {
    let bar = TERM_BOUND * budget.max(0.0);
    let mut t = 1u8;
    for n in 2..=years.min(4) {
        if (0..n as usize).all(|k| held[k] + amount <= bar) {
            t = n;
        }
    }
    t
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DealKind {
    Signed,
    Transfer,
    /// December's keep-him offer (item 9): a floor on his July split.
    Keep,
}

/// The money year a deal made today pays: the next July's.
pub fn deal_year(d: Date) -> i32 {
    if d.month() >= 7 { d.year() + 1 } else { d.year() }
}

/// The market for college players (item 9): each program's play at each
/// spot, best first, and the programs that bid (FBS and FCS, with money).
pub(crate) struct Market {
    depth: Vec<[Vec<f64>; POSITION_COUNT]>,
    rivals: Vec<usize>,
    medians: [[f64; POSITION_COUNT]; super::data::DIVISION_COUNT],
    lifts: Lifts,
}

/// Each money level's lift over its division at each spot, by `level`
/// (`recruiting::bid_lifts`): what a bid takes off a man's play.
pub type Lifts = [[f64; POSITION_COUNT]; 4];

impl Market {
    pub(crate) fn new(w: &World) -> Market {
        let pn = w.programs.len();
        let mut depth: Vec<[Vec<f64>; POSITION_COUNT]> = vec![Default::default(); pn];
        for p in w.people.iter().filter(|p| p.stage == Stage::College) {
            if let (Some(j), Some(x)) = (p.college, season_read(w, p)) {
                depth[j][p.player.position.index()].push(x);
            }
        }
        for d in depth.iter_mut() {
            for v in d.iter_mut() {
                v.sort_by(|a, b| b.total_cmp(a));
            }
        }
        let rivals: Vec<usize> = (0..pn)
            .filter(|&k| {
                let p = &w.programs[k];
                matches!(p.division, Division::FBS | Division::FCS) && p.collective.budget > 0.0
            })
            .collect();
        Market { depth, rivals, medians: super::recruiting::nil_medians(w), lifts: super::recruiting::bid_lifts(w) }
    }

    /// What wins college player `i` from the rest of the market (item 9):
    /// (the best rival's top bid, never past his own school's, that rival
    /// bid, his play over his level's starters at his spot). His own
    /// school's top bid reads his role there (last season's, for December's
    /// keep-him offers; a landed transfer's, the place he landed in); a
    /// rival is any FBS or FCS program where he would start or rotate, his
    /// play read at its level. None with no season's play to read or no
    /// bid.
    pub(crate) fn bid(&self, w: &World, i: usize) -> Option<(f64, f64, f64)> {
        self.bid_read(w, i, true)
    }

    /// [`Market::bid`], with `landed` false reading a landed transfer by his
    /// last role, as December's keep-him offers always have.
    fn bid_read(&self, w: &World, i: usize, landed: bool) -> Option<(f64, f64, f64)> {
        let p = &w.people[i];
        // An early or summer enrollee's latest row is still his high
        // school's: no college play to price yet (as `recruiting::nil_read`).
        if landed
            && p.player
                .history
                .iter()
                .rev()
                .find(|h| h.playing_time != PlayingTime::Unsigned)
                .is_none_or(|h| !h.level.is_college())
        {
            return None;
        }
        let c = p.college?;
        let from = w.programs[c].division;
        // His season as the market reads it, tape and all (item 6).
        let perf = season_read(w, p)?;
        let pos = p.player.position;
        let k = pos.index();
        let stars = super::recruiting::recruiting_stars(p);
        let slots = crate::league::depth_slots(pos).starters;
        let medians = &self.medians;
        // A transfer who hasn't played here yet is read in the place he
        // landed in (`nil_role`, cleared at fall camp).
        let (own_role, own_edge) = match p.cal.nil_role.filter(|_| landed) {
            Some((role, edge)) => (role, edge as f64),
            None => (p.last_role.unwrap_or(PlayingTime::Backup), perf - medians[from as usize][k]),
        };
        // Each bid reads him against the starters at its school's level.
        let read = if own_edge.is_finite() { own_edge } else { 0.0 };
        let own = top_bid(
            w.programs[c].collective.budget,
            pos,
            bid_talent(Some(own_role), stars, read - self.lifts[level(&w.programs[c])][k]),
        );
        let mut rival = 0.0f64;
        for &r in self.rivals.iter().filter(|&&r| r != c) {
            let prog = &w.programs[r];
            let d = prog.division;
            let at = perf - super::recruiting::competition_discount(from) + super::recruiting::competition_discount(d);
            let ahead = self.depth[r][k].partition_point(|&v| v >= at);
            let role = super::recruiting::role_at(ahead, slots);
            if role == PlayingTime::Backup {
                continue;
            }
            let m = medians[d as usize][k];
            let edge = if m.is_finite() { at - m } else { 0.0 } - self.lifts[level(prog)][k];
            rival = rival.max(top_bid(prog.collective.budget, pos, bid_talent(Some(role), stars, edge)));
        }
        let keep = own.min(rival);
        (keep > 0.0).then_some((keep, rival, own_edge))
    }
}

/// Every college player's market price today (`Market::bid`), for the
/// tools: (his index, the price, his play over his level's starters at his
/// spot), for each player at an FBS or FCS program
/// with money who has a season left and isn't leaving (the draft, the
/// portal, a cut).
pub fn market_all(w: &World) -> Vec<(usize, f64, f64)> {
    let market = Market::new(w);
    let label = if w.today < w.dates.fall_camp { w.year - 1 } else { w.year };
    (0..w.people.len())
        .into_par_iter()
        .filter_map(|i| {
            let p = &w.people[i];
            let c = p.college?;
            let prog = &w.programs[c];
            if p.stage != Stage::College
                || !matches!(prog.division, Division::FBS | Division::FCS)
                || prog.collective.budget <= 0.0
                || p.cal.portal.is_some()
                || p.cut_here().is_some()
                || p.cal.declare_plan == Some(true)
                || super::college::no_season_left(p, prog.division, label)
            {
                return None;
            }
            market.bid(w, i).map(|b| (i, b.0, b.2))
        })
        .collect()
}

/// December's keep-him offers before the January window (item 9): each AI
/// program prices every player it wants back (one with a season left who
/// isn't leaving) at the best rival's top bid for him and offers up to its
/// own top bid in the role he has there. Every role, not only starters
/// (2026-11-18): before, a rotation player or backup a rival would start
/// or rotate was left on the split, never his market price; the offer
/// still stands only where it beats his split (`recruiting::NilHeld`). A
/// rival is any FBS or FCS program where he would start or rotate, his
/// play read at its level. The offer is his floor on July 1
/// (`DealKind::Keep`), and his stay read weighs it (`stay_read`).
/// Your program's money is yours alone (Cain 2026-11-08), so it makes none.
pub(crate) fn keep_offers(w: &mut World, label: i32) {
    if !super::ncaa::two_pots(w.today) {
        return;
    }
    let yours = super::front_office::college::paying(w);
    let market = Market::new(w);
    let today = w.today;
    let year = deal_year(today);
    let wr: &World = w;
    let offers: Vec<(usize, f64, f64, f64)> = (0..wr.people.len())
        .into_par_iter()
        .filter_map(|i| {
            let p = &wr.people[i];
            let c = p.college?;
            if p.stage != Stage::College
                || p.last_role.is_none()
                || p.cal.portal.is_some()
                || p.cut_here().is_some()
                || p.cal.declare_plan == Some(true)
                || Some(c) == yours
                || wr.programs[c].collective.budget <= 0.0
            {
                return None;
            }
            if super::college::no_season_left(p, wr.programs[c].division, label) {
                return None;
            }
            market.bid_read(wr, i, false).map(|(keep, rival, edge)| (i, keep, rival, edge))
        })
        .collect();
    let free = super::recruiting::nil_go_free(w);
    for (i, keep, rival, edge) in offers {
        // Under a multi-year deal at or past what keeps him, nothing to offer.
        if w.people[i].cal.nil_deal.is_some_and(|d| d.contract() && d.pays(year) && d.amount >= keep) {
            continue;
        }
        let j = w.people[i].college.expect("a college player");
        let prog = &w.programs[j];
        let b = prog.collective.budget;
        let s = if b > 0.0 { (super::ncaa::school_pot(prog, today) / b).clamp(0.0, 1.0) } else { 0.0 };
        let role = w.people[i].last_role;
        let top = range_top(w, j, &w.people[i], role, edge.max(-99.0));
        let room = super::ncaa::share_room(prog, year);
        let coll = keep * (1.0 - s);
        let r = review(today, coll, top, room, free == Some(j));
        count(w, coll, &r);
        w.programs[j].collective.moved += r.moved;
        let amount = keep * s + r.cleared + r.moved;
        let p = &mut w.people[i];
        // A man under a multi-year deal (item 12) is kept by raising it, as
        // schools renegotiate: past his deal it pays the raise for the rest
        // of the term; at or under it he stays on his deal as written.
        if let Some(d) = p.cal.nil_deal.filter(|d| d.contract() && d.pays(year)) {
            if amount > d.amount {
                p.cal.nil_keep = Some((amount, rival));
                p.cal.nil_deal = Some(NilDeal { amount, ..d });
                let age = p.row_age();
                p.note(format!("At {age} his school raised his multi-year NIL deal to keep him from the portal."));
            }
            continue;
        }
        p.cal.nil_keep = Some((amount, rival));
        p.cal.nil_deal = Some(NilDeal { year, amount, kind: DealKind::Keep, through: 0 });
    }
}

/// A world from before the new NIL scheme (`NIL_REV`): this money year's
/// budgets and revenue shares worked out again on the new pots, and NIL
/// Go's book read from them.
pub fn ensure(w: &mut World) {
    if w.nil_rev >= NIL_REV {
        return;
    }
    if w.nil_rev < 2 {
        refit(w);
    }
    // 3: recruits' stars read at the school's level (`star_at`).
    w.nil_book.star_lift = star_lifts(w);
    w.nil_rev = NIL_REV;
}

/// `ensure` before version 2: the money year's pots again and the book.
fn refit(w: &mut World) {
    let d = w.today;
    // The book on February's signing day set the coming money year's
    // budgets (`staff::update_programs`).
    let year = if d.month() > 2 || (d.month() == 2 && d.day() >= 8) { d.year() } else { d.year() - 1 };
    if super::ncaa::nil_era(year) {
        for p in w.programs.iter_mut() {
            p.collective.budget = super::ncaa::nil_budget(p, year);
            p.collective.school = super::ncaa::revenue_share(p, year);
        }
    }
    if super::ncaa::two_pots(d) {
        w.nil_book = NilBook::from_pots(w);
    }
}

/// The new NIL scheme's version of a world's money (`ensure`).
/// 2: the Group of 5's money refit to real payments (2b, 2026-11-18).
/// 3: recruits' stars read at the school's level (2026-11-18).
pub(crate) const NIL_REV: u8 = 3;

/// July 1's book (item 10): NIL Go's comparisons for the coming year,
/// read from the third-party money each player cleared today.
pub(crate) fn read_book(w: &mut World, deals: Vec<(usize, f64)>, new_share: [f64; 4]) {
    let pots = pot_rates(w);
    w.nil_book = NilBook { star_lift: star_lifts(w), new_share, ..NilBook::read(w.today.year(), deals, pots) };
}

/// How much of his tape the college market adds to his season's play
/// (negotiation item 6), the NFL consensus's own share of production over
/// his rating (`pro::market::consensus`). Proposed, not confirmed.
pub const TAPE_W: f64 = 0.3;
/// The most his tape can move the read, either way (rating points), as at
/// the NFL. Proposed, not confirmed.
pub const TAPE_CAP: f64 = 6.0;

/// What his engine games showed of him this season over his ratings
/// (negotiation item 6), on the rating scale: each read of the results
/// ledger (`results::Ledger`), its season's shown level over his rating
/// shrunk by what the season keeps of it (`Fit::weight` on his game
/// trials, so a few snaps or a noisy read count little), averaged over
/// the reads he was tried at. The ledger is his season's from fall camp
/// to the next, so December's keep offers and the January window see the
/// whole season. Practice trials sit in the same sums; their weight comes
/// from his game trials alone, so a man who never played shows nothing.
/// None with no ledger for this season (a season on the stand-in, or
/// growth from results off).
pub fn tape(w: &World, p: &Person) -> Option<f64> {
    use crate::results::{self, Read};
    let j = p.college.filter(|_| p.stage == Stage::College)?;
    let label = if w.today < w.dates.fall_camp { w.year - 1 } else { w.year };
    let l = w.results.ledgers.get(&p.player.id).filter(|l| l.year == label)?;
    let level = super::division_level(w.programs[j].division);
    let pos = p.player.position;
    let (mut sum, mut reads) = (0.0, 0u32);
    for e in l.reads.iter().filter(|e| e.n > e.np) {
        let f = results::fit(level, Read::from_index(e.read as usize), pos);
        let keep = f.weight((e.n - e.np) as f64, results::CHANGE, e.v as f64);
        if keep <= 0.0 {
            continue;
        }
        sum += keep * e.centred() / e.n as f64;
        reads += 1;
    }
    (reads > 0).then(|| sum / reads as f64)
}

/// His season as the college market reads it for a price (negotiation
/// item 6): his season's play (`last_performance`) moved by his tape
/// (`tape`, `TAPE_W` of it, at most `TAPE_CAP` either way), so a season
/// his games showed below his ratings lowers what other schools would pay.
/// The stars, the draft and development keep reading `last_performance`.
pub fn season_read(w: &World, p: &Person) -> Option<f64> {
    let perf = p.last_performance.get()?;
    Some(perf + TAPE_W * tape(w, p).unwrap_or(0.0).clamp(-TAPE_CAP, TAPE_CAP))
}
