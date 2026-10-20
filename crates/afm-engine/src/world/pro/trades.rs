//! Section 5 topic 6: trades, and the draft picks they move. Each GM prices
//! picks on his own chart (the classic Jimmy Johnson chart or a flatter
//! surplus-value chart), future picks discounted by his urgency; a player
//! is worth his value to the team over his remaining years minus what he
//! costs. Real rules: trades from the new league year to the deadline
//! (the Tuesday after week 9 in 2026), picks up to three drafts ahead
//! (compensatory picks tradable since 2017), no salary retention (the
//! remaining signing bonus accelerates onto the trading team's cap), a
//! physical can void a deal. Also the compensatory picks (topic 5 question
//! 4). Sizes are Claude's, proposed, not confirmed.

use super::market::{consensus, post, worth};
use super::office::Mode;
use super::{Status, money, space};
use crate::calendar::{Date, Kind, Loudness};
use crate::league::depth_slots;
use crate::positions::Position;
use crate::rng::clamp;
use crate::world::World;
use crate::world::season::key_uniform;
use std::collections::HashMap;

/// A compensatory pick (end of rounds 3-7).
#[derive(Clone, Copy, Debug)]
pub struct CompPick {
    pub year: i32,
    pub team: usize,
    pub owner: usize,
    pub round: u32,
    /// The value of the free agent it replaces, for the order in the round.
    pub value: f64,
    /// Free agency, supplemental (fewer than 32 earned), or the Rooney rule.
    pub kind: CompKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompKind {
    FreeAgency,
    Supplemental,
    Diversity,
}

/// A trade on the books.
#[derive(Clone, Debug)]
pub struct Trade {
    pub date: Date,
    pub buyer: usize,
    pub seller: usize,
    /// The player's id (a person's place moves when the world is compacted).
    pub player: u32,
    /// (draft year, round, original team) from buyer to seller.
    pub picks: Vec<(i32, u32, usize)>,
    pub text: String,
}

/// Who owns each pick and which compensatory picks exist.
#[derive(Clone, Debug, Default)]
pub struct PickBook {
    /// (draft year, round, original team) → owner, for traded picks.
    pub owner: HashMap<(i32, u32, usize), usize>,
    pub comp: Vec<CompPick>,
    pub trades: Vec<Trade>,
    /// Qualifying free agents who changed teams: (league year, person,
    /// old team, new team, money a year).
    pub cfa: Vec<(i32, usize, usize, usize, f64)>,
    /// Minority coaches hired away as head coaches: (year, old team, offseasons there).
    pub diversity: Vec<(i32, usize, u32)>,
}

impl PickBook {
    pub fn owner_of(&self, year: i32, round: u32, orig: usize) -> usize {
        self.owner.get(&(year, round, orig)).copied().unwrap_or(orig)
    }
}

/// Hands over a pick (an offer sheet's compensation, a trade).
pub(crate) fn give_pick(w: &mut World, from: usize, to: usize, year: i32, round: u32) {
    // The giver's own pick in that round if he still has it, else one he
    // holds from another team.
    let teams = w.nfl.len();
    let orig = if w.market.picks.owner_of(year, round, from) == from {
        Some(from)
    } else {
        (0..teams).find(|&o| w.market.picks.owner_of(year, round, o) == from)
    };
    if let Some(o) = orig {
        w.market.picks.owner.insert((year, round, o), to);
    }
}

/// The draft board with owners: each (original team, round) slot handed to
/// the pick's owner, and this year's compensatory picks in place of the
/// keyed ones once free agency has awarded any.
pub(crate) fn board_owners(w: &World, year: i32, board: &mut Vec<(usize, u32)>, base: &[usize]) {
    let comp: Vec<&CompPick> = w.market.picks.comp.iter().filter(|c| c.year == year).collect();
    if !comp.is_empty() {
        let mut full: Vec<(usize, u32)> = Vec::new();
        let rounds = super::super::franchise::draft_rounds(year);
        for round in 1..=rounds {
            for t in super::super::franchise::round_order(year, round, base) {
                full.push((t, round));
            }
            let mut extra: Vec<&&CompPick> = comp.iter().filter(|c| c.round == round).collect();
            extra.sort_by(|a, b| {
                (a.kind == CompKind::Diversity).cmp(&(b.kind == CompKind::Diversity)).then(b.value.total_cmp(&a.value))
            });
            for c in extra {
                full.push((c.owner, round | COMP));
            }
        }
        let scale = w.options.scale;
        *board = if scale >= 1.0 {
            full
        } else {
            let n = ((full.len() as f64 * scale).round() as usize).max(1);
            (0..n).map(|i| full[((i as f64 / scale) as usize).min(full.len() - 1)]).collect()
        };
    }
    for slot in board.iter_mut() {
        if slot.1 & COMP != 0 {
            slot.1 &= !COMP;
            continue;
        }
        slot.0 = w.market.picks.owner_of(year, slot.1, slot.0);
    }
}

/// Marks a compensatory slot while the board is built (owners are already set).
const COMP: u32 = 1 << 16;

// --- Charts ---------------------------------------------------------------------------------------------

/// The classic chart (Jimmy Johnson's, as teams used it from the 1990s):
/// pick 1 3,000 points, the end of round one about 590, round seven a few.
const CLASSIC: [(f64, f64); 23] = [
    (1.0, 3000.0),
    (2.0, 2600.0),
    (3.0, 2200.0),
    (4.0, 1800.0),
    (5.0, 1700.0),
    (8.0, 1400.0),
    (10.0, 1300.0),
    (16.0, 1000.0),
    (20.0, 850.0),
    (24.0, 740.0),
    (32.0, 590.0),
    (40.0, 500.0),
    (48.0, 420.0),
    (64.0, 270.0),
    (80.0, 190.0),
    (96.0, 116.0),
    (112.0, 76.0),
    (128.0, 42.0),
    (160.0, 27.0),
    (192.0, 14.0),
    (224.0, 2.0),
    (260.0, 1.0),
    (400.0, 0.5),
];

/// A flatter surplus-value chart (shaped after the public surplus charts:
/// the first pick about twice the 32nd), on the same 3,000 scale.
const SURPLUS: [(f64, f64); 14] = [
    (1.0, 3000.0),
    (5.0, 2300.0),
    (10.0, 1900.0),
    (16.0, 1650.0),
    (32.0, 1400.0),
    (48.0, 1150.0),
    (64.0, 980.0),
    (96.0, 700.0),
    (128.0, 500.0),
    (160.0, 380.0),
    (192.0, 280.0),
    (224.0, 200.0),
    (260.0, 150.0),
    (400.0, 60.0),
];

/// A pick's points on a GM's own chart (draft-day trades, section 7).
pub(crate) fn chart(classic: bool, pick: f64) -> f64 {
    if classic { chart_at(&CLASSIC, pick) } else { chart_at(&SURPLUS, pick) }
}

fn chart_at(table: &[(f64, f64)], pick: f64) -> f64 {
    let p = pick.max(1.0);
    for k in 1..table.len() {
        let (a, va) = table[k - 1];
        let (b, vb) = table[k];
        if p <= b {
            let f = (p - a) / (b - a);
            return va + f * (vb - va);
        }
    }
    table.last().unwrap().1
}

/// A pick's value on a GM's chart: this year's picks at their slot (the
/// order is set once the season ends), later years at mid-round, cut by
/// his urgency (a contender wants now).
fn pick_value(w: &World, t: usize, year: i32, round: u32, orig: usize) -> f64 {
    let teams = super::super::franchise::active_teams(w.year).len().max(1) as f64;
    let o = &w.clubs[t].office;
    let slot = if year == w.year && w.today < w.dates.draft[0].date {
        super::super::nfl_year::draft_order(w)
            .and_then(|order| order.iter().position(|&x| x == orig))
            .map_or(teams / 2.0, |k| k as f64 + 1.0)
    } else {
        teams / 2.0
    };
    let pick = (round - 1) as f64 * teams + slot;
    let v = if o.gm.classic_chart { chart_at(&CLASSIC, pick) } else { chart_at(&SURPLUS, pick) };
    let urgency: f64 = match o.mode {
        Mode::Contend => 0.3,
        Mode::Middle => 0.18,
        Mode::Rebuild => 0.08,
    };
    let ahead = (year - w.year - if w.today < w.dates.draft[0].date { 0 } else { 1 }).max(0);
    v * (1.0 - urgency).powi(ahead)
}

/// What a player is worth to a team in chart points (topic 6 question 2,
/// "delicate"): its value of him a year over his remaining years minus his
/// cap cost, as a share of the cap (10,000 points for a full cap-year of
/// surplus, so about two late firsts for a fifth of the cap). Proposed,
/// not confirmed.
fn player_points(w: &World, t: usize, i: usize) -> f64 {
    let year = w.year;
    let (_, apy) = worth(w, t, i);
    let Some(k) = w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref()) else { return 0.0 };
    let age = w.people[i].player.age;
    let mut pts = 0.0;
    for y in year..=k.last_year() {
        let decline = if age as i32 + (y - year) >= 30 { 0.88f64.powi(y - year) } else { 1.0 };
        let cost = k.year(y).map_or(0.0, |x| x.base + x.roster_bonus + x.workout + x.per_game);
        pts += 10_000.0 * (apy * decline - cost) / money::price_cap(y);
    }
    pts
}

// --- The trade market ------------------------------------------------------------------------------------

/// When a trade window opens: the offseason's (the new league year,
/// before the draft, camp, cutdown week), each week of the season before
/// the deadline, and deadline week.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Window {
    Offseason,
    Season,
    Deadline,
}

/// One trade window: a contender or middling team with a hole at a
/// starting spot calls teams with a starter-grade player there that they
/// value less (a rebuilding team, a surplus, a player who asked out). The
/// seller asks its own value of him, more from a GM who was fleeced by
/// this buyer before; the buyer pays with picks only up to its own value.
/// A deal can still die in talks or on a physical. Trial 9 finding 10,
/// traced with `MarketYear::trade_funnel`: about 40 calls a year, a deal
/// both sides value in half of them and a third of talks failing gave
/// 10-17 trades a year, against real several dozen (Front Office Sports
/// counts 15-22 players traded in season, kickoff to the deadline, in
/// 2020-24). So clubs call more in each window, and every week of the
/// season before the deadline is a window too, as real GMs call through
/// September and October (the rates proposed, not confirmed).
pub(crate) fn trade_window(w: &mut World, d: Date, window: Window) {
    let year = w.year;
    let june1 = super::after_june1(w, d);
    let teams: Vec<usize> = (0..w.clubs.len()).filter(|&t| super::super::franchise::active(t, year)).collect();
    if w.market.fleece.len() < w.nfl.len() {
        w.market.fleece = vec![vec![0.0; w.nfl.len()]; w.nfl.len()];
    }
    let mut moved: Vec<usize> = Vec::new();
    for &buyer in &teams {
        if w.clubs[buyer].office.mode == Mode::Rebuild {
            continue;
        }
        // Its biggest hole at a starting spot (an injury opens one in season).
        let mut holes: Vec<(Position, f64)> = Position::ALL
            .iter()
            .filter(|p| !p.is_specialist())
            .map(|&p| (p, crate::career::need(&w.nfl[buyer], p)))
            .filter(|x| x.1 >= 1.2)
            .collect();
        holes.sort_by(|a, b| b.1.total_cmp(&a.1));
        let Some(&(pos, _)) = holes.first() else { continue };
        let roll = key_uniform(w.options.seed as u64 ^ 0x7EAD ^ d.0 as u64, buyer as u64);
        let calls = match window {
            Window::Offseason => 0.45,
            Window::Season => 0.1,
            Window::Deadline => 0.45,
        } * if w.clubs[buyer].office.mode == Mode::Contend { 1.5 } else { 1.0 };
        if roll > calls {
            continue;
        }
        w.market.year_mut(year).trade_funnel[0] += 1;
        let mut reached = 0;
        let worst = w.clubs[buyer].depth[pos.index()]
            .iter()
            .take(depth_slots(pos).starters)
            .map(|&j| super::grade(w, buyer, j))
            .fold(f64::MAX, f64::min);
        let mut best: Option<(usize, usize, f64, f64)> = None;
        for &seller in &teams {
            if seller == buyer
                || w.market.fleece[buyer][seller] > 1500.0 && key_uniform(0xF1EE, (buyer * 64 + seller) as u64) < 0.6
            {
                continue;
            }
            for &i in w.clubs[seller].depth.get(pos.index()).map_or(&[][..], |v| v.as_slice()) {
                if moved.contains(&i) {
                    continue;
                }
                let c = w.people[i].pro.as_ref().unwrap();
                let k = match c.contract.as_ref() {
                    Some(k) if !k.no_trade && k.team == seller => k,
                    _ => continue,
                };
                if w.clubs[seller].office.owner.pet == Some(i)
                    || c.status != Status::Roster
                    || c.out_weeks > 0
                    || c.teams.len() > 1 && c.new_team
                {
                    continue;
                }
                // Draft picks this year can't be traded until they sign; a just-signed free agent rarely.
                if k.signed == year && k.kind != money::Kind::Rookie {
                    continue;
                }
                let gb = super::grade(w, buyer, i);
                if gb < worst + 2.0 {
                    continue;
                }
                reached = reached.max(1);
                let surplus_seller = {
                    let sm = w.clubs[seller].office.mode;
                    let starters = depth_slots(pos).starters;
                    let rank = w.clubs[seller].depth[pos.index()].iter().position(|&j| j == i).unwrap_or(9);
                    sm == Mode::Rebuild && w.people[i].player.age >= 26 || rank >= starters || c.wants_out
                };
                if !surplus_seller {
                    continue;
                }
                reached = reached.max(2);
                let vb = player_points(w, buyer, i);
                let vs = player_points(w, seller, i) * if c.wants_out { 0.85 } else { 1.0 };
                let ask = vs.max(30.0) * (1.05 + (w.market.fleece[buyer][seller] / 5000.0).clamp(0.0, 0.3));
                if vb < ask {
                    continue;
                }
                let cost = k.cap_hit(year) - k.year(year).map_or(0.0, |x| x.proration);
                if space(w, buyer, year) < cost {
                    continue;
                }
                // The seller must stay under the cap too (real): the bonus it
                // keeps lands on its cap as his cap number comes off.
                if space(w, seller, year) + k.cap_hit(year) < kept_bonus(k, year, june1).0 {
                    continue;
                }
                reached = reached.max(3);
                if best.is_none_or(|b| vb - ask > b.2) {
                    best = Some((seller, i, vb - ask, ask));
                }
            }
        }
        {
            let f = &mut w.market.year_mut(year).trade_funnel;
            for k in 1..=reached {
                f[k] += 1;
            }
        }
        let Some((seller, i, _, ask)) = best else { continue };
        // Talks can fail; a physical can void it (proposed, not confirmed).
        if key_uniform(w.options.seed as u64 ^ 0x7EAE ^ d.0 as u64, i as u64) < 0.35 {
            w.market.year_mut(year).trade_funnel[4] += 1;
            shopping_rumor(w, i, seller, d);
            continue;
        }
        let Some(package) = package(w, buyer, seller, ask) else {
            w.market.year_mut(year).trade_funnel[5] += 1;
            continue;
        };
        if key_uniform(w.options.seed as u64 ^ 0x7EAF ^ d.0 as u64, i as u64) < 0.02 {
            let who = super::describe(w, i);
            post(
                w,
                d,
                Some(seller),
                Loudness::Medium,
                Kind::Transfer,
                format!("A trade for {who} is voided after a failed physical."),
            );
            continue;
        }
        execute(w, d, buyer, seller, i, package, window);
        moved.push(i);
    }
}

/// The bonus a seller keeps when it trades a player: his proration from
/// this league year on lands on its cap this year, or, from June 1 on, this
/// year's share stays and the later years' land next year, as with a
/// release (real; trial 7 finding 18: a trade once left a seller with half
/// the cap in dead money in one year, and it could no longer field a team).
/// Returns (this year, next year).
fn kept_bonus(k: &money::Contract, year: i32, june1: bool) -> (f64, f64) {
    let share = |pick: &dyn Fn(i32) -> bool| k.years.iter().filter(|y| pick(y.year)).map(|y| y.proration).sum::<f64>();
    if june1 { (share(&|y| y == year), share(&|y| y > year)) } else { (share(&|y| y >= year), 0.0) }
}

/// Picks offered (year, round, original team) and their cost to the buyer.
type Package = (Vec<(i32, u32, usize)>, f64);

/// The buyer's picks that meet the seller's ask on the seller's chart at
/// the lowest cost on the buyer's own: the cheapest single pick, else the
/// cheapest pair, over the next three drafts.
fn package(w: &World, buyer: usize, seller: usize, ask: f64) -> Option<Vec<(i32, u32, usize)>> {
    let year = w.year;
    let first = if w.today < w.dates.draft[0].date { year } else { year + 1 };
    let teams = w.nfl.len();
    let mut own: Vec<((i32, u32, usize), f64, f64)> = Vec::new();
    for y in first..first + 3 {
        for round in 1..=7u32 {
            for orig in 0..teams {
                if w.market.picks.owner_of(y, round, orig) != buyer
                    || !super::super::franchise::active(orig, y.min(year + 1))
                {
                    continue;
                }
                let vs = pick_value(w, seller, y, round, orig);
                let vb = pick_value(w, buyer, y, round, orig);
                own.push(((y, round, orig), vs, vb));
            }
        }
    }
    let single = own.iter().filter(|x| x.1 >= ask).min_by(|a, b| a.2.total_cmp(&b.2)).map(|x| (vec![x.0], x.2));
    let mut pair: Option<Package> = None;
    for a in 0..own.len() {
        for b in a + 1..own.len() {
            if own[a].1 + own[b].1 >= ask {
                let cost = own[a].2 + own[b].2;
                if pair.as_ref().is_none_or(|p| cost < p.1) {
                    pair = Some((vec![own[a].0, own[b].0], cost));
                }
            }
        }
    }
    match (single, pair) {
        (Some(s), Some(p)) => Some(if s.1 <= p.1 { s.0 } else { p.0 }),
        (Some(s), None) => Some(s.0),
        (None, Some(p)) => Some(p.0),
        _ => None,
    }
}

fn execute(
    w: &mut World,
    d: Date,
    buyer: usize,
    seller: usize,
    i: usize,
    picks: Vec<(i32, u32, usize)>,
    window: Window,
) {
    let year = w.year;
    // The seller keeps the bonus already paid: its remaining proration
    // accelerates onto the seller's cap; the buyer takes the salary.
    let june1 = super::after_june1(w, d);
    let (now, later) =
        w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref()).map_or((0.0, 0.0), |k| kept_bonus(k, year, june1));
    let id = w.people[i].player.id;
    w.clubs[seller].add_dead(year, now, id, super::DeadWhy::Traded);
    w.clubs[seller].add_dead(year + 1, later, id, super::DeadWhy::Traded);
    if let Some(k) = w.people[i].pro.as_mut().unwrap().contract.as_mut() {
        for y in k.years.iter_mut().filter(|y| y.year >= year) {
            y.proration = 0.0;
        }
    }
    let consensus_given: f64 = picks
        .iter()
        .map(|&(y, r, _)| chart_at(&CLASSIC, ((r - 1) * 32 + 16) as f64) * if y > year { 0.85 } else { 1.0 })
        .sum();
    for &(y, r, o) in &picks {
        w.market.picks.owner.insert((y, r, o), seller);
    }
    let fair = consensus(w, i);
    let got = (fair - 75.0).max(0.0) * 120.0;
    w.market.fleece[seller][buyer] += (consensus_given - got).max(0.0);
    w.market.fleece[buyer][seller] += (got - consensus_given).max(0.0);
    // Named with the team he played for (trial 7 finding 17).
    let who = super::describe(w, i);
    super::transfer(w, i, buyer, "traded");
    let to = w.nfl[buyer].name;
    let from = w.nfl[seller].name;
    let what: Vec<String> = picks.iter().map(|&(y, r, _)| format!("a {y} round {r} pick")).collect();
    let text = format!("The {from} trade {who} to the {to} for {}.", what.join(" and "));
    post(w, d, Some(buyer), if fair >= 82.0 { Loudness::Big } else { Loudness::Medium }, Kind::Transfer, text.clone());
    let my = w.market.year_mut(year);
    my.trades += 1;
    my.picks_traded += picks.len() as u32;
    if window == Window::Deadline {
        my.deadline_trades += 1;
    }
    if window != Window::Offseason {
        my.season_trades += 1;
    }
    let c = w.people[i].pro.as_mut().unwrap();
    c.wants_out = false;
    c.hold = super::market::Hold::None;
    let player = w.people[i].player.id;
    w.market.picks.trades.push(Trade { date: d, buyer, seller, player, picks, text });
}

/// A shopping effort that leaks (topic 6 question 4): the player hears it;
/// a loyal veteran takes it worst.
fn shopping_rumor(w: &mut World, i: usize, seller: usize, d: Date) {
    if key_uniform(0x5A0B ^ d.0 as u64, i as u64) > 0.3 {
        return;
    }
    let loyal = super::market::goals(w, i).loyalty;
    let name = w.nfl[seller].name;
    let p = &mut w.people[i];
    let c = p.pro.as_mut().unwrap();
    c.trust = clamp(c.trust - 0.1 - 0.2 * loyal, 0.0, 1.0);
    let age = p.row_age();
    p.note(format!("Learned at {age} that the {name} had been shopping him."));
    let who = super::describe(w, i);
    post(w, d, Some(seller), Loudness::Small, Kind::News, format!("Reports: the {name} have been shopping {who}."));
}

// --- Compensatory picks ------------------------------------------------------------------------------------

/// Notes a qualifying free agent (topic 5 question 4, real rules): an
/// unrestricted free agent whose deal ran out (not one who was cut) signing
/// with a new team before the cut-off (the Monday after the draft;
/// proposed, not confirmed).
pub(crate) fn note_cfa(w: &mut World, i: usize, from: usize, to: usize, apy: f64, d: Date) {
    let year = w.year;
    if year < 1993 || d > w.dates.draft[2].date.add(3) || d < w.dates.league_year.date {
        return;
    }
    if apy < money::minimum(year, 4) * 1.5 {
        return;
    }
    w.market.picks.cfa.push((year, i, from, to, apy));
    w.market.year_mut(year).cfa_noted += 1;
}

/// The comp picks, awarded at the league meeting for this year's draft
/// from last year's free agents (OverTheCap's public method): each
/// qualifying free agent gets a value from his money a year (ranked
/// against every contract in the league), his share of snaps (25% or
/// more) and his honors (All-Pro 20, Pro Bowl 5); the league's top 5% is a
/// third-round value, then fourth (10%), fifth (15%), sixth (25%) and
/// seventh (35%). Each free agent gained cancels one lost of the same round
/// (else the best lower one, else the lowest higher one); a team gets a
/// pick for each loss left, at most four, and the league 32 (the best
/// values); short of 32, the rest go as supplemental seventh-round picks
/// in draft order. A free agent cut by his new team before the season ends
/// doesn't count.
pub(crate) fn award_comp_picks(w: &mut World, d: Date) {
    let year = w.year;
    if year < 1994 {
        return;
    }
    let last = year - 1;
    // Every contract's value points in the league last season.
    let mut apys: Vec<f64> = Vec::new();
    for p in &w.people {
        if let Some(k) =
            p.pro.as_ref().filter(|c| c.seasons.iter().any(|s| s.year == last)).and_then(|c| c.contract.as_ref())
        {
            apys.push(k.apy());
        }
    }
    apys.sort_by(|a, b| a.total_cmp(b));
    let n = apys.len().max(1) as f64;
    let value = |w: &World, i: usize, apy: f64| -> f64 {
        let rank = apys.partition_point(|&x| x < apy) as f64;
        let c = w.people[i].pro.as_ref();
        let s = c.and_then(|c| c.seasons.iter().find(|s| s.year == last));
        let snaps = s.map_or(0.0, |s| (s.snaps[0] + s.snaps[1]) as f64 / super::games::NFL_SEASON_SNAPS as f64 * 100.0);
        let honors = s.map_or(0.0, |s| {
            if s.all_pro == 1 {
                20.0
            } else if s.pro_bowl {
                5.0
            } else {
                0.0
            }
        });
        rank + if snaps >= 25.0 { snaps.min(100.0) } else { 0.0 } + honors
    };
    let mut all: Vec<f64> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        if let Some(k) =
            p.pro.as_ref().filter(|c| c.seasons.iter().any(|s| s.year == last)).and_then(|c| c.contract.as_ref())
        {
            all.push(value(w, i, k.apy()));
        }
    }
    all.sort_by(|a, b| b.total_cmp(a));
    let round_of = |v: f64| -> Option<u32> {
        let pct = all.partition_point(|&x| x > v) as f64 / all.len().max(1) as f64;
        match pct {
            p if p < 0.05 => Some(3),
            p if p < 0.10 => Some(4),
            p if p < 0.15 => Some(5),
            p if p < 0.25 => Some(6),
            p if p < 0.35 => Some(7),
            _ => None,
        }
    };
    let _ = n;
    let teams = w.nfl.len();
    let mut lost: Vec<Vec<(u32, f64)>> = vec![Vec::new(); teams];
    let mut gained: Vec<Vec<(u32, f64)>> = vec![Vec::new(); teams];
    let mut valued = 0;
    for &(y, i, from, to, apy) in &w.market.picks.cfa {
        if y != last {
            continue;
        }
        // Still with his new team when the season ended.
        let stayed =
            w.people[i].pro.as_ref().is_some_and(|c| c.seasons.iter().any(|s| s.year == last && s.team == Some(to)));
        if !stayed {
            continue;
        }
        let v = value(w, i, apy);
        let Some(r) = round_of(v) else { continue };
        valued += 1;
        lost[from].push((r, v));
        gained[to].push((r, v));
    }
    let mut awards: Vec<(usize, u32, f64)> = Vec::new();
    for t in 0..teams {
        let mut l = lost[t].clone();
        l.sort_by(|a, b| b.1.total_cmp(&a.1));
        let mut g = gained[t].clone();
        g.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (gr, _) in g {
            let k = l
                .iter()
                .position(|x| x.0 == gr)
                .or_else(|| l.iter().position(|x| x.0 > gr))
                .or_else(|| l.iter().rposition(|x| x.0 < gr));
            if let Some(k) = k {
                l.remove(k);
            }
        }
        for (r, v) in l.into_iter().take(4) {
            awards.push((t, r, v));
        }
    }
    // Last year's free agents are settled.
    w.market.picks.cfa.retain(|x| x.0 >= year);
    awards.sort_by(|a, b| b.2.total_cmp(&a.2));
    awards.truncate(32);
    let earned = awards.len();
    for &(t, r, v) in &awards {
        w.market.picks.comp.push(CompPick { year, team: t, owner: t, round: r, value: v, kind: CompKind::FreeAgency });
    }
    // Supplemental picks to 32, in draft order.
    if let Some(order) = super::super::nfl_year::draft_order(w) {
        for (k, &t) in order.iter().take(32usize.saturating_sub(earned)).enumerate() {
            w.market.picks.comp.push(CompPick {
                year,
                team: t,
                owner: t,
                round: 7,
                value: -(k as f64),
                kind: CompKind::Supplemental,
            });
        }
    }
    // The diversity picks (from 2021 drafts; topic 5 question 4b, Cain's
    // reading): longer than two offseasons with the team, a third-rounder;
    // shorter, by the losing team's standing: a worse team picks higher.
    let hires: Vec<(i32, usize, u32)> =
        w.market.picks.diversity.iter().copied().filter(|x| x.0 == year - 1 || x.0 == year - 2).collect();
    let order = super::super::nfl_year::draft_order(w).unwrap_or_default();
    for (_, t, tenure) in hires {
        if year < 2021 {
            continue;
        }
        let round = if tenure > 2 {
            3
        } else {
            let k = order.iter().position(|&x| x == t).unwrap_or(16) as f64 / order.len().max(1) as f64;
            if k < 0.34 {
                3
            } else if k < 0.67 {
                4
            } else {
                5
            }
        };
        w.market.picks.comp.push(CompPick { year, team: t, owner: t, round, value: 0.0, kind: CompKind::Diversity });
    }
    let total = w.market.picks.comp.iter().filter(|c| c.year == year).count() as u32;
    let supplemental =
        w.market.picks.comp.iter().filter(|c| c.year == year && c.kind == CompKind::Supplemental).count() as u32;
    let m = w.market.year_mut(year);
    m.comp_picks = total;
    m.cfa_valued = valued;
    m.comp_formula = earned as u32;
    m.comp_supplemental = supplemental;
    post(
        w,
        d,
        None,
        Loudness::Small,
        Kind::Date,
        format!("{total} compensatory picks are awarded for the {year} draft."),
    );
}

/// A minority assistant hired away as a head coach (the 2020 resolution):
/// his old team earns the diversity picks.
pub(crate) fn note_diversity_hire(w: &mut World, old: usize, offseasons: u32) {
    let year = w.year;
    if year >= 2020 {
        w.market.picks.diversity.push((year, old, offseasons));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_charts_fall_and_the_surplus_chart_is_flatter() {
        let mut last = (f64::MAX, f64::MAX);
        for p in 1..=260 {
            let c = (chart_at(&CLASSIC, p as f64), chart_at(&SURPLUS, p as f64));
            assert!(c.0 <= last.0 && c.1 <= last.1, "pick {p}");
            last = c;
        }
        // Real anchors: pick 1 is about five times pick 32 on the classic
        // chart and about twice on the surplus charts.
        let classic = chart_at(&CLASSIC, 1.0) / chart_at(&CLASSIC, 32.0);
        let surplus = chart_at(&SURPLUS, 1.0) / chart_at(&SURPLUS, 32.0);
        assert!((4.5..5.5).contains(&classic) && (1.8..2.4).contains(&surplus), "{classic} {surplus}");
    }
}
