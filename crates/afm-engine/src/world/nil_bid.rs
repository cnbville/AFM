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
/// $764K, four-star receivers at the 15 richest $187K, the five-star
/// quarterback at Florida State, $45M, $2.63M, the top class $7.47M).
pub fn star_bid(stars: u8) -> f64 {
    match stars {
        5 => 0.8,
        4 => 0.25,
        3 => 0.07,
        2 => 0.02,
        _ => 0.006,
    }
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
/// it denies 4.5% of the year's reviewed deals and 10% of their dollars
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
        NilBook::read(w.today.year(), Vec::new(), pot_rates(w))
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
    /// The money year (from July 1 of it) it pays.
    pub year: i32,
    /// What he gets: both pots, the third party's part as NIL Go cleared it.
    pub amount: f64,
    pub kind: DealKind,
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

/// December's keep-him offers before the January window (item 9): each AI
/// program prices every starter it wants back (one with a season left who
/// isn't leaving) at the best rival's top bid for him and offers up to its
/// own top bid. A rival is any FBS or FCS program where he would start or
/// rotate, his play read at its level. The offer is his floor on July 1
/// (`DealKind::Keep`), and his stay read weighs it (`stay_read`).
/// Your program's money is yours alone (Cain 2026-11-08), so it makes none.
pub(crate) fn keep_offers(w: &mut World, label: i32) {
    if !super::ncaa::two_pots(w.today) {
        return;
    }
    let yours = super::front_office::college::paying(w);
    let pn = w.programs.len();
    let medians = super::recruiting::nil_medians(w);
    // Each program's play at each spot, best first.
    let mut depth: Vec<[Vec<f64>; POSITION_COUNT]> = vec![Default::default(); pn];
    for p in w.people.iter().filter(|p| p.stage == Stage::College) {
        if let (Some(j), Some(x)) = (p.college, p.last_performance) {
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
    let today = w.today;
    let year = deal_year(today);
    let wr: &World = w;
    let offers: Vec<(usize, f64, f64, f64)> = (0..wr.people.len())
        .into_par_iter()
        .filter_map(|i| {
            let p = &wr.people[i];
            let c = p.college?;
            if p.stage != Stage::College
                || p.last_role != Some(PlayingTime::Starter)
                || p.cal.portal.is_some()
                || p.cut_here().is_some()
                || p.cal.declare_plan == Some(true)
                || Some(c) == yours
                || wr.programs[c].collective.budget <= 0.0
            {
                return None;
            }
            let from = wr.programs[c].division;
            if super::college::no_season_left(p, from, label) {
                return None;
            }
            let perf = p.last_performance?;
            let pos = p.player.position;
            let k = pos.index();
            let stars = super::recruiting::recruiting_stars(p);
            let slots = crate::league::depth_slots(pos).starters;
            let own_edge = perf - medians[from as usize][k];
            let own = top_bid(
                wr.programs[c].collective.budget,
                pos,
                bid_talent(Some(PlayingTime::Starter), stars, if own_edge.is_finite() { own_edge } else { 0.0 }),
            );
            let mut rival = 0.0f64;
            for &r in rivals.iter().filter(|&&r| r != c) {
                let prog = &wr.programs[r];
                let d = prog.division;
                let at =
                    perf - super::recruiting::competition_discount(from) + super::recruiting::competition_discount(d);
                let ahead = depth[r][k].partition_point(|&v| v >= at);
                let role = super::recruiting::role_at(ahead, slots);
                if role == PlayingTime::Backup {
                    continue;
                }
                let m = medians[d as usize][k];
                let edge = if m.is_finite() { at - m } else { 0.0 };
                rival = rival.max(top_bid(prog.collective.budget, pos, bid_talent(Some(role), stars, edge)));
            }
            let keep = own.min(rival);
            (keep > 0.0).then_some((i, keep, rival, own_edge))
        })
        .collect();
    let free = super::recruiting::nil_go_free(w);
    for (i, keep, rival, edge) in offers {
        let j = w.people[i].college.expect("a college player");
        let prog = &w.programs[j];
        let b = prog.collective.budget;
        let s = if b > 0.0 { (super::ncaa::school_pot(prog, today) / b).clamp(0.0, 1.0) } else { 0.0 };
        let top = range_top(w, j, &w.people[i], Some(PlayingTime::Starter), edge.max(-99.0));
        let room = super::ncaa::share_room(prog, year);
        let coll = keep * (1.0 - s);
        let r = review(today, coll, top, room, free == Some(j));
        count(w, coll, &r);
        w.programs[j].collective.moved += r.moved;
        let amount = keep * s + r.cleared + r.moved;
        let p = &mut w.people[i];
        p.cal.nil_keep = Some((amount, rival));
        p.cal.nil_deal = Some(NilDeal { year, amount, kind: DealKind::Keep });
    }
}

/// A world from before the new NIL scheme (`NIL_REV`): this money year's
/// budgets and revenue shares worked out again on the new pots, and NIL
/// Go's book read from them.
pub fn ensure(w: &mut World) {
    if w.nil_rev >= NIL_REV {
        return;
    }
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
    w.nil_rev = NIL_REV;
}

/// The new NIL scheme's version of a world's money (`ensure`).
pub(crate) const NIL_REV: u8 = 1;

/// July 1's book (item 10): NIL Go's comparisons for the coming year,
/// read from the third-party money each player cleared today.
pub(crate) fn read_book(w: &mut World, deals: Vec<(usize, f64)>) {
    let pots = pot_rates(w);
    w.nil_book = NilBook::read(w.today.year(), deals, pots);
}
