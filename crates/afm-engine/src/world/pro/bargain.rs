//! The offer builder (the trade finder, TF1-TF20): one set of rules for
//! every trade, yours and the AI's (TF20). A buyer's top price is his own
//! value of the player (`trades::player_points`: his read, his chart, his
//! position's age curve, his race and his doctors). He aims below it by
//! how hard he bargains and how much the seller must sell (TF9, TF18), he
//! opens below his aim (TF7), and he pays in full packages: any number of
//! picks over three drafts, a player back the seller values more than he
//! does, a conditional pick, a swap of this year's picks (TF5, TF6).
//!
//! The sizes are Claude's, proposed, not confirmed: the dealing styles'
//! spread, the bargaining share, the opener, the leverage weights, the
//! doctors' misread and the owner's sign-off odds.

use super::office::{Mode, Power};
use super::{money, space};
use crate::calendar::Date;
use crate::rng::clamp;
use crate::world::World;
use crate::world::front_office::deals::{self, Condition, Pick};
use crate::world::season::{key_normal, key_uniform};

/// A GM's hidden dealing style (TF18): how often he trades and how hard he
/// bargains, each 0.15-0.85. Drawn from the man himself, so it stays with
/// him from club to club and needs nothing stored.
#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub deals: f64,
    pub haggle: f64,
}

pub fn style(w: &World, t: usize) -> Style {
    let gm = &w.clubs[t].office.gm;
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in gm.name.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
    }
    h ^= gm.since as u64;
    Style { deals: 0.15 + 0.7 * key_uniform(h, 0xDEA1), haggle: 0.15 + 0.7 * key_uniform(h, 0x4A66) }
}

/// A club's chance of the playoffs as a front office reads it (TF10): the
/// record so far and its strength for the games left, against the usual
/// cut line (about 56% of the games for 7 of 16 places). Before the season
/// a club goes by its own plan: a contender expects to be in it, a
/// rebuilding club doesn't.
pub fn race(w: &World, t: usize) -> f64 {
    let by_mode = || match w.clubs[t].office.mode {
        Mode::Contend => 0.7,
        Mode::Rebuild => 0.15,
        _ => 0.45,
    };
    let Some(s) = w.nfl_season.as_ref().filter(|s| s.year == w.year && s.started && t < s.wins.len()) else {
        return by_mode();
    };
    let games = s.weeks.iter().filter(|wk| wk.iter().any(|g| g.0 == t || g.1 == t)).count().max(1) as f64;
    let played = (s.wins[t] + s.losses[t] + s.ties[t]) as f64;
    let z = s.z.get(t).copied().unwrap_or(0.0);
    let p = crate::world::power::phi(0.45 * z);
    let left = (games - played).max(0.0);
    let expect = s.wins[t] as f64 + 0.5 * s.ties[t] as f64 + p * left;
    let sd = (left * p * (1.0 - p)).sqrt() + 0.8;
    crate::world::power::phi((expect - 0.56 * games) / sd)
}

/// A club's doctors' read of a player's record (TF15), on the medical
/// grades' scale (0.6 a minor concern, 1.4 major, 2.4 a failed physical):
/// the truth plus their own lasting misread.
pub fn doctors(w: &World, t: usize, i: usize) -> f64 {
    let truth = crate::world::scouting::file::medical_truth(&w.people[i]);
    truth + 0.35 * key_normal(w.options.seed as u64 ^ 0xD0C7_0125 ^ t as u64, w.people[i].player.id as u64)
}

/// What a club takes off a player's worth for his record: nothing for a
/// clean one, up to 30% for a failed one.
pub fn health_cut(read: f64) -> f64 {
    clamp((read - 0.6) * 0.15, 0.0, 0.3)
}

/// The physical (TF15): the buyer's doctors examine him and compare it
/// with what they read before the deal. Most pass; a fresh finding on a
/// bad record has the buyer ask for less, a failed one voids the trade.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Exam {
    Passed,
    /// Something new: the buyer wants a pick off.
    Revised,
    Failed,
}

pub fn exam(w: &World, buyer: usize, i: usize, key: u64) -> (Exam, f64) {
    let truth = crate::world::scouting::file::medical_truth(&w.people[i]);
    let now = truth + 0.2 * key_normal(w.options.seed as u64 ^ 0xE8A3 ^ key, w.people[i].player.id as u64);
    let new = now - doctors(w, buyer, i);
    let verdict = if now >= 2.4 && new >= 0.5 {
        Exam::Failed
    } else if now >= 1.4 && new >= 0.3 {
        Exam::Revised
    } else {
        Exam::Passed
    };
    (verdict, now)
}

/// How much of the seller's own value the league knows it can't hold out
/// for (TF9): a trade request, a holdout, a cap squeeze, his last year.
/// Each named, for the screens.
pub fn leverage(w: &World, seller: usize, i: usize) -> (f64, Vec<&'static str>) {
    let Some(c) = w.people[i].pro.as_ref() else { return (0.0, vec![]) };
    let mut keep = 1.0;
    let mut why = Vec::new();
    if c.wants_out {
        keep *= 0.5;
        why.push("He asked to be traded");
    }
    if c.hold != super::market::Hold::None {
        keep *= 0.7;
        why.push("He's holding out");
    }
    if c.contract.as_ref().is_some_and(|k| k.last_year() == w.year) {
        keep *= 0.75;
        why.push("It's the last year of his deal");
    }
    if money::capped(w.year) && super::cap_room(w, seller, w.year) < 0.0 {
        keep *= 0.7;
        why.push("Your club is over the cap");
    }
    (1.0 - keep, why)
}

/// The seller's walk-away price as the league reads it: his value to the
/// seller less what the seller can't hold out for.
pub fn reservation(w: &World, seller: usize, i: usize) -> f64 {
    let own = super::trades::player_points(w, seller, i);
    if own <= 0.0 { own } else { own * (1.0 - leverage(w, seller, i).0) }
}

/// The share of the gap between the seller's walk-away and his own top a
/// GM keeps for his club (0.36-0.64 by his style).
pub fn keeps(s: Style) -> f64 {
    0.3 + 0.4 * s.haggle
}

/// What a buyer aims to pay without a rival pushing him: the seller's
/// walk-away plus the part of the gap he gives up.
pub fn aim(w: &World, buyer: usize, seller: usize, i: usize, top: f64) -> f64 {
    let r = reservation(w, seller, i).max(0.0);
    let k = keeps(style(w, buyer));
    if top > r { r + (top - r) * (1.0 - k) } else { top * (1.0 - 0.5 * k) }
}

/// His opener (TF7): 70-90% of his aim, the harder bargainer lower.
pub fn opener(w: &World, buyer: usize, aim: f64) -> f64 {
    aim * (0.9 - 0.2 * style(w, buyer).haggle)
}

/// Whether a deal is big enough for an owner's word (TF16): a star
/// (consensus 82+) or a first-round pick in it.
pub fn blockbuster(w: &World, i: usize, picks: &[Pick]) -> bool {
    super::market::consensus(w, i) >= 82.0 || picks.iter().any(|p| p.round == 1)
}

/// Whether an owner who has the final say signs off (TF16): a splash pulls
/// him in, a rebuild or a shaky GM holds him back. Only clubs whose power
/// sits with the owner ask him. Keyed, so asking again gives one answer.
pub fn owner_yes(w: &World, t: usize, buying: bool, key: u64) -> Option<bool> {
    let o = &w.clubs[t].office;
    if o.power != Power::Owner {
        return None;
    }
    let ow = &o.owner;
    let p = if buying {
        0.55 + 0.25 * ow.pr + 0.2 * (ow.trust - 0.5) - if o.mode == Mode::Rebuild { 0.25 } else { 0.0 }
    } else {
        0.6 + 0.25 * (1.0 - ow.pr) + 0.2 * (ow.trust - 0.5)
    };
    Some(key_uniform(w.options.seed as u64 ^ 0x0E7E_5A55 ^ t as u64, key) < clamp(p, 0.15, 0.95))
}

/// What a buyer gives: picks over three drafts, a player back, a condition
/// on one of the picks, a swap of this year's picks.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Package {
    pub picks: Vec<Pick>,
    /// Players coming back to the seller (ids).
    pub back: Vec<u32>,
    /// A condition on one of the buyer's picks, read on the player traded:
    /// met, the pick moves up a round.
    pub cond: Option<(Pick, Condition)>,
    /// A swap of this year's picks: (the seller's pick, the buyer's better one).
    pub swap: Option<(Pick, Pick)>,
    /// What it costs the buyer on his chart, and is worth to the seller on his.
    pub cost: f64,
    pub worth: f64,
}

/// Limits on what a package may hold.
#[derive(Clone, Copy, Debug, Default)]
pub struct Limits {
    /// Players may come back.
    pub back: bool,
    /// Only picks in this draft (draft night).
    pub draft_only: Option<i32>,
    /// Conditions on picks (they settle after the season).
    pub cond: bool,
    /// A swap of this year's picks.
    pub swap: bool,
}

/// Whether club `t` can trade player `i` today: on its roster, healthy,
/// under contract, no no-trade clause, not the owner's pet, not a free
/// agent it signed this league year.
pub fn movable(w: &World, t: usize, i: usize) -> bool {
    let Some(c) = w.people[i].pro.as_ref() else { return false };
    let Some(k) = c.contract.as_ref() else { return false };
    c.team == Some(t)
        && k.team == t
        && !k.no_trade
        && c.status == super::Status::Roster
        && c.out_weeks == 0
        && w.clubs[t].office.owner.pet != Some(i)
        && !(k.signed == w.year && k.kind != money::Kind::Rookie)
        && k.last_year() >= w.year
}

/// The buyer's package worth `value` on his own chart, built from what the
/// seller values most for what it costs him: a player back he values less
/// than the seller would at a spot the seller needs, then his picks, the
/// biggest that fits first, then a condition or a swap for what's left.
pub fn build(w: &World, buyer: usize, seller: usize, i: usize, value: f64, lim: Limits) -> Option<Package> {
    if value <= 0.0 {
        return None;
    }
    let year = w.year;
    let mut left = value;
    let mut pk = Package::default();
    if lim.back {
        let share = super::trades::paid_share(w, w.today);
        let his = w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref());
        let room = space(w, seller, year) + his.map_or(0.0, |k| super::trades::left_to_pay(k, year, share));
        let mut best: Option<(usize, f64, f64)> = None;
        for &j in &w.clubs[buyer].roster {
            if j == i || !movable(w, buyer, j) {
                continue;
            }
            let pos = w.people[j].player.position;
            if pos.is_specialist() || crate::career::need(&w.nfl[seller], pos) < 1.0 {
                continue;
            }
            let k = w.people[j].pro.as_ref().unwrap().contract.as_ref().unwrap();
            if money::capped(year) && super::trades::left_to_pay(k, year, share) > room {
                continue;
            }
            let vb = super::trades::player_points(w, buyer, j);
            if vb < -20.0 || vb > 0.6 * value {
                continue;
            }
            let vs = super::trades::player_points(w, seller, j);
            if vs - vb < 60.0 {
                continue;
            }
            if best.is_none_or(|b| vs - vb > b.2 - b.1) {
                best = Some((j, vb, vs));
            }
        }
        if let Some((j, vb, vs)) = best {
            pk.back.push(w.people[j].player.id);
            left -= vb.max(0.0);
            pk.cost += vb.max(0.0);
            pk.worth += vs;
        }
    }
    let mut avail: Vec<(Pick, f64)> = deals::picks_of(w, buyer)
        .into_iter()
        .filter(|p| lim.draft_only.is_none_or(|y| p.year == y))
        .map(|p| {
            let v = deals::pick_points(w, buyer, &p);
            (p, v)
        })
        .collect();
    avail.sort_by(|a, b| b.1.total_cmp(&a.1));
    while pk.picks.len() < 5 {
        let Some(k) = avail.iter().position(|x| x.1 <= left * 1.08 && x.1 > 0.0) else { break };
        let (p, v) = avail.remove(k);
        left -= v;
        pk.cost += v;
        pk.worth += deals::pick_points(w, seller, &p);
        pk.picks.push(p);
        let smallest = avail.iter().map(|x| x.1).fold(f64::MAX, f64::min);
        if left < 15.0 || left < 0.35 * smallest {
            break;
        }
    }
    // A condition on one of his later picks: what it's likely to become.
    if lim.cond && left > 15.0 {
        let mut best: Option<(Pick, Condition, f64)> = None;
        for p in pk.picks.iter().filter(|p| p.round >= 2 && p.year > year && !p.comp) {
            let up = Pick { round: p.round - 1, ..*p };
            let gain = deals::pick_points(w, buyer, &up) - deals::pick_points(w, buyer, p);
            for c in [Condition::Games(12), Condition::Starts(10), Condition::ProBowl] {
                let exp = deals::met_chance(w, buyer, i, c) * gain;
                if exp <= left * 1.1 && exp >= 0.3 * left && best.is_none_or(|b| exp > b.2) {
                    best = Some((*p, c, exp));
                }
            }
        }
        if let Some((p, c, exp)) = best {
            pk.cond = Some((p, c));
            left -= exp;
            pk.cost += exp;
            let up = Pick { round: p.round - 1, ..p };
            pk.worth += deals::met_chance(w, seller, i, c)
                * (deals::pick_points(w, seller, &up) - deals::pick_points(w, seller, &p));
        }
    }
    // A swap of this year's picks once the order is set: his earlier pick
    // in a round for the seller's later one.
    if lim.swap && left > 15.0 && w.today < w.dates.draft[0].date && lim.draft_only.is_none_or(|y| y == year) {
        let mut best: Option<(Pick, Pick, f64)> = None;
        for round in 2..=crate::world::franchise::draft_rounds(year) {
            let theirs = Pick { year, round, orig: buyer, comp: false };
            let mine = Pick { year, round, orig: seller, comp: false };
            if w.market.picks.owner_of(year, round, buyer) != buyer
                || w.market.picks.owner_of(year, round, seller) != seller
                || pk.picks.contains(&theirs)
            {
                continue;
            }
            let gain = deals::pick_points(w, buyer, &theirs) - deals::pick_points(w, buyer, &mine);
            if gain > 5.0 && gain <= left * 1.2 && best.is_none_or(|b| (left - gain).abs() < (left - b.2).abs()) {
                best = Some((mine, theirs, gain));
            }
        }
        if let Some((mine, theirs, gain)) = best {
            pk.swap = Some((mine, theirs));
            pk.cost += gain;
            pk.worth += deals::pick_points(w, seller, &theirs) - deals::pick_points(w, seller, &mine);
        }
    }
    (!pk.picks.is_empty() || !pk.back.is_empty()).then_some(pk)
}

/// A date a few days on, never past the deadline (offers on deadline day
/// come within hours).
pub fn days_on(w: &World, d: Date, n: i32) -> Date {
    let dl = w.dates.trade_deadline.date;
    if d >= dl { d } else { d.add(n).min(dl) }
}

/// A club's record this season (wins, losses, ties), once it has started.
pub fn record(w: &World, t: usize) -> Option<(u32, u32, u32)> {
    let s = w.nfl_season.as_ref().filter(|s| s.year == w.year && s.started && t < s.wins.len())?;
    Some((s.wins[t], s.losses[t], s.ties[t]))
}

/// The bonus you keep on this year's cap if you trade player `i` today.
pub fn kept_now(w: &World, i: usize) -> f64 {
    let june1 = super::after_june1(w, w.today);
    w.people[i]
        .pro
        .as_ref()
        .and_then(|c| c.contract.as_ref())
        .map_or(0.0, |k| super::trades::kept_bonus(k, w.year, june1).0)
}

/// How much player `i` would lift club `t` at his spot: his grade there
/// less its weakest man among its usual starters (him left out). 2+ is the
/// upgrade a buyer calls about.
pub fn upgrade(w: &World, t: usize, i: usize) -> f64 {
    let pos = w.people[i].player.position;
    let n = crate::league::depth_slots(pos).starters.max(1);
    let worst = w.clubs[t]
        .depth
        .get(pos.index())
        .map(|d| d.iter().filter(|&&j| j != i).take(n).map(|&j| super::grade(w, t, j)).fold(f64::MAX, f64::min))
        .unwrap_or(f64::MAX);
    let worst = if worst == f64::MAX { 40.0 } else { worst };
    super::grade(w, t, i) - worst
}
