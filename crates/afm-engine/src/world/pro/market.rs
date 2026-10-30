//! Section 5 topics 2-5 and the NFL year's money dates: agents (topic 3),
//! how each team prices a player (topic 2), the player's hidden ideal
//! package and the perfect offer with the 85% and seven-day rule (topics 4
//! and 5), free agency with its tampering window and boards, re-signing,
//! tags, tenders and offer sheets, fifth-year options, extensions,
//! holdouts and hold-ins, restructures and cap casualties, reserve/future
//! deals, waivers and the cash spending floor. AI teams run every step
//! under the same rules a manager will. Sizes are Claude's, proposed, not
//! confirmed, unless a comment names the real rule.

use super::{DeadCharge, DeadWhy, Status, grade, money, office::Mode, payroll, release_with, space};
use crate::calendar::{Date, Event, Kind, Level as CalLevel, Loudness};
use crate::league::depth_slots;
use crate::positions::Position;
use crate::rng::{Rng, clamp};
use crate::world::front_office::{self as fo, Duty};
use crate::world::news::count;
use crate::world::season::{key_normal, key_uniform};
use crate::world::wlog::Why;
use crate::world::{Stage, World};

// --- People around the deal ---------------------------------------------------------------

/// A player's hidden goals (topic 3: set by his personality, read only
/// through what he does): the weights he puts on each part of an offer,
/// and the traits that drive holdouts and disputes. Drawn once from his
/// id and personality, so they never change.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Goals {
    pub money: f64,
    pub security: f64,
    pub winning: f64,
    pub role: f64,
    pub city: f64,
    pub home: f64,
    pub loyalty: f64,
    /// Holdout drive: how much being underpaid eats at him.
    pub greed: f64,
    /// Fights in public, fires his agent.
    pub temper: f64,
    /// Negotiates without an agent (Lamar Jackson; rare).
    pub alone: bool,
}

pub fn goals(w: &World, i: usize) -> Goals {
    let p = &w.people[i];
    let id = p.player.id as u64;
    let k = |salt: u64| clamp(0.5 + 0.2 * key_normal(w.options.seed as u64 ^ 0x60A1 ^ salt, id), 0.05, 1.0);
    let per = &p.player.personality;
    let love = (per.love_of_game - 10.0) / 10.0;
    let comp = (per.competitiveness - 10.0) / 10.0;
    Goals {
        money: clamp(k(1) - 0.15 * love + 0.1 * (0.5 - p.bg.money), 0.05, 1.0),
        security: clamp(k(2) + 0.2 * (0.5 - p.bg.money), 0.05, 1.0),
        winning: clamp(k(3) + 0.2 * comp, 0.05, 1.0),
        role: clamp(k(4) + 0.1 * comp, 0.05, 1.0),
        city: k(5),
        home: clamp(0.3 * k(6) + 0.5 * p.stay_home, 0.0, 1.0),
        // Section 6 topic 1: loyalty, greed and temper are his personality's.
        loyalty: clamp(0.5 + 0.45 * per.traits.z(crate::player::Trait::Loyalty) + 0.05 * k(7), 0.05, 1.0),
        greed: clamp(0.5 + 0.45 * per.traits.z(crate::player::Trait::Greed) + 0.05 * k(8), 0.05, 1.0),
        temper: clamp(0.5 + 0.45 * per.traits.z(crate::player::Trait::Temper) + 0.05 * k(9), 0.05, 1.0),
        alone: key_uniform(w.options.seed as u64 ^ 0xA10E, id) < 0.003,
    }
}

/// An agent (topic 3): NFLPA-certified, fee capped at 3%. Agencies run from
/// one-man shops to giants; size brings leverage and information. His
/// honesty sets how often he claims interest that isn't there; GMs learn
/// it from dealing with him, and that history carries across his clients.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Agent {
    pub name: String,
    pub agency: String,
    /// 1 one-man shop, 2 a mid-size agency, 3 a giant.
    pub size: u8,
    pub skill: f64,
    pub honesty: f64,
    pub clients: u32,
    /// Each team's trust in him, -1 to 1 (bluffs called, promises kept).
    pub rel: Vec<f64>,
    pub fired: u32,
}

/// Agencies' names (generated: no real agency is named).
const AGENCY_WORDS: [&str; 12] = [
    "Summit",
    "Pinnacle",
    "Lakeshore",
    "Meridian",
    "Keystone",
    "Bluewater",
    "Ironside",
    "Northstar",
    "Granite",
    "Liberty",
    "Sterling",
    "Harbor",
];

pub(crate) fn staff_agents(w: &mut World) {
    let mut rng = Rng::new(w.options.seed ^ 0xA6E7_7500);
    let n = ((180.0 * w.options.scale.max(0.2)).round() as usize).max(40);
    let teams = w.nfl.len();
    let year = w.year;
    for k in 0..n {
        // Six giants with several agents each, about thirty mid-size
        // agencies, the rest one-man shops (proposed sizes).
        let size = if k < 30 {
            3
        } else if k < 80 {
            2
        } else {
            1
        };
        let eth = if rng.chance(0.25) {
            super::super::ethnicity::Ethnicity::Black
        } else {
            super::super::ethnicity::Ethnicity::White
        };
        let name = super::super::names::person_name(eth, year - rng.int(30, 60), &mut rng);
        let agency = match size {
            3 => format!("{} Sports Group", AGENCY_WORDS[k % 6]),
            2 => format!("{} Athletes", AGENCY_WORDS[(k * 7) % AGENCY_WORDS.len()]),
            _ => format!("{} Sports Management", name.rsplit(' ').next().unwrap_or("Independent")),
        };
        let skill = clamp(rng.normal(9.0 + 2.5 * size as f64, 3.0), 1.0, 20.0);
        w.market.agents.push(Agent {
            name,
            agency,
            size,
            skill,
            honesty: clamp(rng.normal(0.7, 0.18), 0.1, 1.0),
            clients: 0,
            rel: vec![0.0; teams],
            fired: 0,
        });
    }
}

/// Gives a pro an agent when he has none: the giants sign most of the
/// first round, smaller shops the late picks and the undrafted (real
/// shape, proposed shares).
pub(crate) fn sign_agent(w: &mut World, i: usize) {
    if w.market.agents.is_empty() || goals(w, i).alone {
        return;
    }
    let Some(c) = w.people[i].pro.as_ref() else { return };
    if c.agent.is_some() {
        return;
    }
    let round = c.draft.map_or(9, |d| d.1);
    let big = match round {
        1 => 0.75,
        2 | 3 => 0.5,
        4..=7 => 0.3,
        _ => 0.15,
    };
    let id = w.people[i].player.id as u64;
    let u = key_uniform(w.options.seed as u64 ^ 0xA6E1, id ^ (w.year as u64) << 40);
    let want = if u < big {
        3
    } else if u < big + 0.4 {
        2
    } else {
        1
    };
    let pool: Vec<usize> = (0..w.market.agents.len()).filter(|&a| w.market.agents[a].size == want).collect();
    if pool.is_empty() {
        return;
    }
    let v = key_uniform(w.options.seed as u64 ^ 0xA6E2, id ^ (w.year as u64) << 40);
    let a = pool[(v * pool.len() as f64) as usize % pool.len()];
    w.market.agents[a].clients += 1;
    w.people[i].pro.as_mut().unwrap().agent = Some(a);
}

/// A player fires his agent (a bluff that cost him, a deal below his
/// worth) and signs with another.
pub(crate) fn fire_agent(w: &mut World, i: usize, why: &str) {
    let Some(a) = w.people[i].pro.as_ref().and_then(|c| c.agent) else { return };
    w.market.agents[a].fired += 1;
    w.market.agents[a].clients = w.market.agents[a].clients.saturating_sub(1);
    w.people[i].pro.as_mut().unwrap().agent = None;
    w.market.year_mut(w.year).agents_fired += 1;
    let old = w.market.agents[a].name.clone();
    sign_agent(w, i);
    let p = &mut w.people[i];
    let age = p.row_age();
    p.note(format!("Fired his agent {old} at {age} ({why})."));
}

// --- Prices -------------------------------------------------------------------------------------

/// What the league's game tape and stat sheets say about his last season,
/// on the rating scale: (efficiency in context, totals). Efficiency is the
/// engine's per-play grade, which already judges each play against what it
/// faced (a stacked box, a weak line), so a Derrick Henry on a bad team
/// reads well and empty volume doesn't (topic 2 question 1). Totals rank
/// his stat line among his position's starters. None for a season on the
/// stand-in or a small sample.
pub(crate) fn production(w: &World, i: usize) -> Option<(f64, f64)> {
    let p = &w.people[i];
    let id = p.player.id;
    let s = p.pro.as_ref()?.seasons.iter().rev().find(|s| s.year < w.year || s.games > 0)?;
    if s.stats.graded < 150 {
        return None;
    }
    let eff = s.stats.grade();
    let tot = w.market.totals.get(&id).copied().unwrap_or(60.0);
    Some((eff, tot))
}

/// Ranks last season's stat lines at each position (totals), once a
/// year at the league year: the stats-reading GM's view.
pub(crate) fn rank_totals(w: &mut World) {
    let year = w.year;
    let mut by_pos: Vec<Vec<(u32, f64)>> = vec![Vec::new(); crate::positions::POSITION_COUNT];
    for p in &w.people {
        if p.stage != Stage::Pro {
            continue;
        }
        let Some(s) = p.pro.as_ref().and_then(|c| c.seasons.iter().find(|s| s.year == year - 1)) else { continue };
        if s.stats.graded < 150 {
            continue;
        }
        by_pos[p.player.position.index()].push((p.player.id, super::honors::stats_value(&s.stats, p.player.position)));
    }
    w.market.totals.clear();
    for list in by_pos.iter_mut() {
        list.sort_by(|a, b| b.1.total_cmp(&a.1));
        let n = list.len().max(1) as f64;
        for (k, &(id, _)) in list.iter().enumerate() {
            // Top of the position 90, the median 66, the bottom about 45.
            let pct = 1.0 - k as f64 / n;
            w.market.totals.insert(id, 45.0 + 45.0 * pct.powf(1.4));
        }
    }
}

/// Honors in his last three seasons (Pro Bowls, All-Pro twice).
fn honors(w: &World, i: usize) -> f64 {
    let year = w.year;
    w.people[i].pro.as_ref().map_or(0.0, |c| {
        c.seasons
            .iter()
            .filter(|s| s.year >= year - 3)
            .map(|s| if s.pro_bowl { 1.0 } else { 0.0 } + if s.all_pro > 0 { 2.0 } else { 0.0 })
            .sum()
    })
}

/// The league's consensus read of a player, on the rating scale: his tape
/// and his numbers half and half, his name a little.
pub fn consensus(w: &World, i: usize) -> f64 {
    let ovr = w.people[i].player.overall();
    let prod = production(w, i).map_or(0.0, |(e, t)| clamp(0.5 * e + 0.5 * t - ovr, -6.0, 6.0));
    ovr + 0.3 * prod + 0.5 * honors(w, i).min(4.0)
}

/// Where a read sits in the market, on the price curve's grade scale:
/// real contracts are benchmarked on comparables, so a player is priced by
/// his rank among the league's veterans at his position, not by the raw
/// number. The rank, over the starting places at the position across the
/// league, reads off the real pay ladder (`ladder_share`), and the result
/// is the grade `money::market_share` turns into that share of the top
/// five. Thresholds on this scale (a 90 is top-five money, an 80 the top
/// third of the starters) hold whatever the league's talent drifts to.
pub(crate) fn market_grade(w: &World, pos: Position, read: f64) -> f64 {
    let list = &w.market.ladder[pos.index()];
    if list.is_empty() {
        return read;
    }
    // Veterans above him (the list runs best first). Below every veteran
    // there is no comparable, so his place is his rank among everyone at
    // the position, young players included (trial 9 finding 14: a ladder
    // of veterans alone, often shorter than twice the starting places,
    // read a fringe young player as a role player worth about $4M).
    let above = list.partition_point(|&g| g > read);
    let places = (w.market.teams.max(1) * depth_slots(pos).starters.max(1)) as f64;
    let rank = match w.market.depth.get(pos.index()) {
        Some(all) if above == list.len() && !all.is_empty() => all.partition_point(|&g| g > read).max(above),
        _ => above,
    };
    let s = (rank as f64 + 0.5) / places;
    if s > 2.0 {
        // Below every place a team pays for: under the market's floor.
        return (66.0 - 4.0 * (s - 2.0)).max(50.0);
    }
    66.0 + 24.0 * ladder_share(pos, s).powf(1.0 / 1.6)
}

/// The real pay ladder: a contract's average a year as a share of the top
/// five's, by rank over the starting places league-wide (0.5 is the 16th
/// best of 32 starters). Read from OverTheCap's position lists on
/// 2026-10-17: edge rushers 1.0 at the top five, 0.76 at 8th, 0.55 at 16th,
/// 0.32 at 32nd, 0.12 at 64th, 0.06 at 96th (cornerbacks and wide
/// receivers alike); quarterbacks flat near the top (12th still 0.90 of the
/// top five), then a cliff to the bridge starters and backups.
/// Proposed, not confirmed.
fn ladder_share(pos: Position, s: f64) -> f64 {
    const GENERAL: [(f64, f64); 9] = [
        (0.0, 1.15),
        (0.06, 1.0),
        (0.125, 0.78),
        (0.25, 0.58),
        (0.5, 0.36),
        (0.75, 0.2),
        (1.0, 0.11),
        (1.5, 0.055),
        (2.0, 0.0),
    ];
    const QB: [(f64, f64); 8] =
        [(0.0, 1.12), (0.1, 1.0), (0.375, 0.9), (0.5, 0.6), (0.75, 0.3), (1.0, 0.13), (1.5, 0.05), (2.0, 0.0)];
    let pts: &[(f64, f64)] = if pos == Position::QB { &QB } else { &GENERAL };
    let s = s.max(0.0);
    for k in 1..pts.len() {
        let (s1, f1) = pts[k];
        if s <= s1 {
            let (s0, f0) = pts[k - 1];
            return f0 + (f1 - f0) * (s - s0) / (s1 - s0);
        }
    }
    0.0
}

/// The year's pay ladders: every veteran's consensus read at each position
/// (four seasons on the books, or a veteran deal), best first.
pub(crate) fn set_ladders(w: &mut World) {
    let year = w.year;
    let mut lists: Vec<Vec<f64>> = vec![Vec::new(); crate::positions::POSITION_COUNT];
    let mut depth: Vec<Vec<f64>> = vec![Vec::new(); crate::positions::POSITION_COUNT];
    for i in 0..w.people.len() {
        let p = &w.people[i];
        if p.stage != Stage::Pro {
            continue;
        }
        let Some(c) = p.pro.as_ref() else { continue };
        let veteran = c.accrued >= 4
            || c.contract.as_ref().is_some_and(|k| {
                !matches!(
                    k.kind,
                    money::Kind::Rookie | money::Kind::Undrafted | money::Kind::Future | money::Kind::PracticeSquad
                )
            });
        if c.held() || c.status == Status::FreeAgent {
            let read = consensus(w, i);
            depth[p.player.position.index()].push(read);
            if veteran {
                lists[p.player.position.index()].push(read);
            }
        }
    }
    for l in lists.iter_mut().chain(depth.iter_mut()) {
        l.sort_by(|a, b| b.total_cmp(a));
    }
    w.market.ladder = lists;
    w.market.depth = depth;
    w.market.teams = super::super::franchise::active_teams_in(w, year).len();
}

/// What a veteran of this grade costs a year on the open market at his age,
/// never past the position's record deal.
pub(crate) fn market_price(year: i32, pos: Position, view: f64, accrued: u32, age: u32) -> f64 {
    let price = money::price_cap(year) * money::market_share(pos, view).min(money::top5_share(pos) * 1.08);
    (price * decline(pos, age)).max(money::minimum(year, accrued))
}

/// Past his position's prime a team pays for the decline it expects over
/// the deal: about 4% less a year for three years past it, then 9% a year
/// as the cliff nears, to 40% (real: Cousins at 35 at about 88% of the top
/// five's pay, Brees at 39 and Rivers at 38 at about 70%; at 40 about half,
/// and a 40-year-old who has lost a step gets a one-year backup's deal, as
/// Flacco did at 39 and Rodgers a short one at 41; Brady at 43 at 70% after
/// a title is the rare exception).
pub(crate) fn decline(pos: Position, age: u32) -> f64 {
    let past = age.saturating_sub(prime_end(pos)) as f64;
    (1.0 - 0.04 * past - 0.05 * (past - 3.0).max(0.0)).max(0.4)
}

/// The age a position's prime ends at (the class model's decline).
fn prime_end(pos: Position) -> u32 {
    match pos {
        Position::RB => 27,
        Position::QB | Position::K | Position::P | Position::LS => 33,
        Position::OT | Position::IOL => 30,
        _ => 29,
    }
}

/// How a team values a player (topic 2 and topic 7): its own grade, moved
/// toward his tape or his totals by the GM's lean, with each GM bias at its
/// hidden strength, his fit in the team's own systems (topic 8), the age
/// curve over a deal, and then the team's price a year by its need and its
/// mode. Returns (the team's read, its walk-away price a year).
pub(crate) fn worth(w: &World, t: usize, i: usize) -> (f64, f64) {
    let year = w.year;
    let p = &w.people[i];
    let pos = p.player.position;
    let c = p.pro.as_ref();
    let o = &w.clubs[t].office;
    let gm = &o.gm;
    let cons = consensus(w, i);
    let own = grade(w, t, i);
    // 3.7 overconfidence: his own grade over the consensus.
    let mut view = cons + (own - cons) * (0.4 + 0.6 * gm.overconfidence);
    if let Some((eff, tot)) = production(w, i) {
        // 3.5 recency: last season counts more.
        let k = 0.2 + 0.2 * gm.recency;
        view += k * (gm.film_lean * (eff - view) + (1.0 - gm.film_lean) * (tot - view)).clamp(-8.0, 8.0);
    }
    // 3.9 name value.
    view += gm.name_value * 0.8 * honors(w, i).min(4.0);
    if let Some(c) = c {
        let ours = c.team == Some(t);
        // 3.3 "not my guy": the last regime's picks.
        if c.draft.is_some_and(|d| d.3 == t && d.0 < gm.since) {
            view -= 2.0 * gm.not_my_guy;
        }
        // 3.4 loyalty to his own veterans; 3.1 sunk cost on a player he paid.
        if ours && c.seasons.iter().filter(|s| s.team == Some(t)).count() >= 3 {
            view += 1.5 * gm.loyalty;
        }
        if ours && c.contract.as_ref().is_some_and(|k| k.signed >= gm.since && k.apy() > money::price_cap(year) * 0.05)
        {
            view += 2.0 * gm.sunk_cost;
        }
    }
    // 3.8 risk appetite: upside in the young, a floor in the proven.
    let age = p.player.age;
    if age <= 25 {
        view += 1.5 * (gm.risk - 0.5);
    }
    // Topic 8: fit in this team's systems.
    let fits = super::super::systems::fits(&p.player, pos, &w.system_norms);
    view +=
        2.0 * w.clubs[t].office.power.fit_weight() * super::super::systems::fit_to(&fits, pos, &w.clubs[t].installed);
    // The age curve over a deal.
    let end = prime_end(pos);
    if age >= end {
        view -= 1.0 * (age - end + 1) as f64;
    }
    let accrued = c.map_or(0, |c| c.accrued);
    let rel = market_grade(w, pos, view);
    let mut apy = market_price(year, pos, rel, accrued, age);
    // Where he would play here.
    let slots = depth_slots(pos);
    let better =
        w.clubs[t].depth.get(pos.index()).map_or(0, |d| d.iter().filter(|&&j| j != i && grade(w, t, j) > view).count());
    let need = crate::career::need(&w.nfl[t], pos);
    if better < slots.starters {
        apy *= 1.0 + 0.05 * need.clamp(0.0, 3.0);
    } else if better >= slots.starters + slots.rotation {
        apy = apy.min(money::minimum(year, accrued) * 2.0);
    }
    apy *= match o.mode {
        Mode::Contend if better < slots.starters => 1.08,
        Mode::Rebuild if age >= 28 => 0.8,
        Mode::Rebuild if age <= 25 => 1.05,
        _ => 1.0,
    };
    // Styles: a GM who won't pay running backs; 3.10 a pet position.
    if pos == Position::RB && !gm.pays_backs {
        apy = apy.min(money::price_cap(year) * 0.035);
    }
    if gm.fixation == Some(pos) {
        apy *= 1.1;
    }
    // 3.15 an owner who wants a splash after a bad year.
    if o.fans < -0.3 && rel >= 82.0 {
        apy *= 1.0 + 0.1 * o.owner.pr;
    }
    (view, apy.max(money::minimum(year, accrued)))
}

// --- The ideal package and offers ------------------------------------------------------------------

/// What a player wants (topic 5 question 3): a hidden package built from his
/// own weights. Read only through tells, his agent and sit-downs.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Ideal {
    pub apy: f64,
    /// Practical guarantee as a share of the total.
    pub gtd: f64,
    pub years: u32,
    /// Wants a starting job.
    pub start: bool,
    pub opened: Date,
}

/// An offer on the table.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Offer {
    pub team: usize,
    pub apy: f64,
    pub years: u32,
    pub gtd: f64,
    /// A promised starting job (logged; topic 4 question 2).
    pub start: bool,
}

/// A deal agreed and not yet signed: the seven-day window after an offer
/// at 85% of his ideal (another team can still sway him), or a verbal
/// agreement in the tampering window, which waits for the new league year.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Pending {
    pub person: usize,
    pub offer: Offer,
    pub score: f64,
    pub until: Date,
    pub verbal: bool,
}

/// An RFA offer sheet waiting on the old team (five days to match).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Sheet {
    pub person: usize,
    pub offer: Offer,
    pub old: usize,
    pub tender: money::Tender,
    pub until: Date,
}

/// The ideal at the moment his market opens: the consensus price moved by
/// how much money matters to him and his agent's skill, guarantees by his
/// grade and his need for security, the usual length moved by security.
pub(crate) fn ideal(w: &World, i: usize, d: Date) -> Ideal {
    let year = w.year;
    let p = &w.people[i];
    let g = goals(w, i);
    let c = p.pro.as_ref();
    let view = market_grade(w, p.player.position, consensus(w, i));
    let accrued = c.map_or(0, |c| c.accrued);
    let agent = c.and_then(|c| c.agent).map_or(8.0, |a| w.market.agents[a].skill);
    // His money goal and his agent move the ask, never past the record
    // deal at his position (moved for his age as the price is).
    let pos = p.player.position;
    let apy = (market_price(year, pos, view, accrued, p.player.age)
        * (1.0 + 0.2 * (g.money - 0.5))
        * (1.0 + 0.004 * (agent - 10.0)))
        .min(money::record_apy(year, pos) * decline(pos, p.player.age));
    let base = if view >= 85.0 {
        0.55
    } else if view >= 78.0 {
        0.4
    } else if view >= 72.0 {
        0.25
    } else {
        0.05
    };
    let years = money::usual_years(p.player.position, view, p.player.age) as f64 + 1.5 * (g.security - 0.5);
    let slots = depth_slots(p.player.position);
    Ideal {
        apy: apy.max(money::minimum(year, accrued)),
        gtd: clamp(base + 0.2 * (g.security - 0.5), 0.0, 0.8),
        years: clamp(years.round(), 1.0, 5.0) as u32,
        start: g.role > 0.45 && view >= 70.0 && slots.starters > 0,
        opened: d,
    }
}

/// A city's pull (topic 5 question 2, Cain: New York and Miami have a leg
/// up): warm weather, nightlife, a big market. 0-1.
fn city_pull(w: &World, t: usize, year: i32) -> f64 {
    let Some(e) = super::super::franchise::era_in(w, t, year) else { return 0.5 };
    let warm = matches!(e.state, "FL" | "CA" | "AZ" | "NV" | "TX" | "LA" | "GA" | "HI") as u8 as f64;
    let night = match e.city {
        "New York" | "East Rutherford" | "Miami" | "Miami Gardens" | "Las Vegas" | "Los Angeles" | "Inglewood" => 1.0,
        "Chicago" | "New Orleans" | "Atlanta" | "Dallas" | "Arlington" | "Houston" | "Philadelphia" | "Boston"
        | "Foxborough" | "San Francisco" | "Santa Clara" | "Washington" | "Landover" => 0.6,
        _ => 0.2,
    };
    clamp(0.25 + 0.35 * warm + 0.4 * night, 0.0, 1.0)
}

/// Top state income tax rates where each team plays (2025, rounded):
/// none in Florida, Texas, Tennessee, Nevada and Washington.
pub(crate) fn state_tax(w: &World, t: usize, year: i32) -> f64 {
    let Some(e) = super::super::franchise::era_in(w, t, year) else { return 0.05 };
    match e.state {
        "FL" | "TX" | "TN" | "NV" | "WA" => 0.0,
        "CA" => 0.133,
        "NY" => 0.109,
        "NJ" => 0.1075,
        "MN" => 0.0985,
        "OR" => 0.099,
        "MD" => 0.0575,
        "MA" => 0.09,
        "WI" => 0.0765,
        "OH" => 0.035,
        "PA" => 0.0307,
        "IN" => 0.0305,
        "MI" => 0.0425,
        "IL" => 0.0495,
        "AZ" => 0.025,
        "CO" => 0.044,
        "GA" => 0.0539,
        "NC" => 0.0425,
        "LA" => 0.03,
        "MO" => 0.047,
        _ => 0.05,
    }
}

/// How close an offer comes to his ideal, 0 to about 1.1: each part met
/// against what he wants, weighted by his own goals. "Balanced" (Cain): no
/// single term covers the rest, so an offer short on the money or the
/// guarantees he cares about can't pass 85%, whatever the role or the city.
pub(crate) fn score(w: &World, i: usize, o: &Offer, ideal: &Ideal, own: Option<usize>) -> f64 {
    let year = w.year;
    let g = goals(w, i);
    let p = &w.people[i];
    let take = o.apy * (1.0 - state_tax(w, o.team, year)) / (ideal.apy * 0.95).max(1.0);
    let money_q = take.min(1.2);
    let gtd_q = if ideal.gtd < 0.05 { 1.0 } else { (o.gtd / ideal.gtd).min(1.2) };
    let years_q = 1.0 - 0.12 * (o.years as f64 - ideal.years as f64).abs();
    let strength = w.nfl[o.team].strength();
    let win_q = clamp(0.75 + (strength - 80.0) / 12.0, 0.4, 1.15);
    let role_q = if !ideal.start || o.start { 1.0 } else { 0.6 };
    let city_q = 0.85 + 0.3 * city_pull(w, o.team, year);
    let home_q = if super::super::franchise::era_in(w, o.team, year)
        .and_then(|e| super::super::geo::state_index(e.state))
        .is_some_and(|s| s == p.home_state)
    {
        1.1
    } else {
        1.0
    };
    let trust = p.pro.as_ref().map_or(1.0, |c| c.trust);
    let loyal_q = if own == Some(o.team) { 0.9 + 0.25 * trust } else { 1.0 };
    // Section 6 topic 2: a grudge against the team, or a coach he played for.
    let ties_q = super::super::people::ties_q(w, i, o.team);
    let parts = [
        (g.money * 1.6, money_q),
        (g.security, gtd_q),
        (0.4, years_q),
        (g.winning * 0.7, win_q),
        (g.role * 0.7, role_q),
        (g.city * 0.4, city_q),
        (g.home * 0.4, home_q),
        (g.loyalty * 0.6, loyal_q),
        (0.4, ties_q),
        // The NFLPA report card (P1): players read how a club treats them.
        (0.35, crate::world::finance::nfl::card_pull(w, o.team)),
    ];
    let wsum: f64 = parts.iter().map(|x| x.0).sum();
    let mut s = parts.iter().map(|x| x.0 * x.1).sum::<f64>() / wsum.max(1e-6);
    // An agent's first word (C19): the club he called first starts a step
    // ahead (proposed, not confirmed: 2%); the money and fit still decide.
    if !w.folk.backchannel.first.is_empty() && super::super::folk::backchannel::first_word(w, i, o.team) {
        s *= 1.02;
    }
    if (money_q < 0.82 && g.money > 0.3) || (gtd_q < 0.75 && g.security > 0.5) { s.min(0.84) } else { s }
}

/// A team's offer to a player: up to its walk-away price, opening below his
/// ideal and climbing as the market runs; a desperate or contending team
/// goes straight to its number. Guarantees as he asks, up to what the
/// owner will put in escrow (3.13). None when the team is too far away or
/// has no room.
pub(crate) fn make_offer(
    w: &World,
    t: usize,
    i: usize,
    ideal: &Ideal,
    days: i32,
    committed: f64,
    cap_year: i32,
) -> Option<Offer> {
    let year = w.year;
    let (view, walk) = worth(w, t, i);
    if walk < ideal.apy * 0.72 {
        return None;
    }
    let pos = w.people[i].player.position;
    let o = &w.clubs[t].office;
    let need = crate::career::need(&w.nfl[t], pos);
    let eager = need >= 2.5 || o.mode == Mode::Contend && need >= 1.0;
    let climb = clamp(0.88 + 0.015 * days as f64, 0.88, 1.03);
    let apy = if eager { walk.min(ideal.apy * 1.03) } else { walk.min(ideal.apy * climb) };
    let accrued = w.people[i].pro.as_ref().map_or(0, |c| c.accrued);
    let apy = apy.max(money::minimum(year, accrued));
    // Year one's cap hit (the bonus spreads it) has to fit in the year the
    // new money starts (next year for an extension).
    if space(w, t, cap_year) - committed < apy * 0.75 {
        return None;
    }
    let slots = depth_slots(pos);
    let better =
        w.clubs[t].depth.get(pos.index()).map_or(0, |d| d.iter().filter(|&&j| j != i && grade(w, t, j) > view).count());
    let gtd = ideal.gtd.min(0.25 + 0.5 * o.owner.cash_will);
    let years = money::usual_years(pos, market_grade(w, pos, view), w.people[i].player.age);
    Some(Offer { team: t, apy, years, gtd, start: ideal.start && better < slots.starters })
}

/// A deal's terms (topic 4's term board, laid out the way real deals are):
/// most of the guarantee as a signing bonus, the rest as guaranteed base,
/// injury guarantees vesting a year out, roster and workout bonuses (A1,
/// A3), per-game bonuses for older players (A7), incentives on a prove-it
/// deal (A8), void years for a GM who uses them (from 2016), a no-trade
/// clause for an elite quarterback now and then.
pub(crate) fn terms_for(w: &World, i: usize, o: &Offer, start: i32) -> money::Terms {
    // A deal you negotiated keeps the terms you agreed (lane 10).
    if let Some(t) = fo::nfl::seat_terms(w, i, o, start) {
        return t;
    }
    let year = w.year;
    let p = &w.people[i];
    let gm = &w.clubs[o.team].office.gm;
    let mut t = money::Terms::flat(o.years, o.apy);
    let big = o.apy > money::minimum(year, 4) * 3.0;
    // The signing bonus as real top deals size it: about a quarter to a
    // third of the new money (Prescott 2024 33%, Lamar Jackson 2023 28%,
    // Bosa 2023 29%, Watson 2022 20%), the rest of the guarantee as
    // guaranteed base (trial 9 finding 6: two thirds of the guarantee went
    // into the bonus, up to $700M).
    let bonus = if big { (o.gtd * 0.55).min(0.35) } else { 0.0 };
    t.bonus_share = bonus;
    t.base_guarantee = if big { o.gtd - bonus } else { 0.0 };
    t.injury_guarantee = if big && o.years >= 3 { 0.08 } else { 0.0 };
    t.ramp = if o.years >= 3 { 0.06 } else { 0.0 };
    t.roster_share = if o.years >= 3 && year >= 1994 { 0.05 } else { 0.0 };
    t.workout_share = if year >= 1994 { 0.01 } else { 0.0 };
    t.per_game_share = if p.player.age >= 30 && year >= 2006 { 0.04 } else { 0.0 };
    t.incentives = if o.years == 1 && big { o.apy * 0.1 } else { 0.0 };
    t.void_years = if year >= 2016 && big && gm.early_extensions > 0.6 { 2 } else { 0 };
    t.no_trade =
        p.player.position == Position::QB && consensus(w, i) >= 88.0 && key_uniform(0x0D7A, p.player.id as u64) < 0.3;
    // A team with room to spare front-loads (real: the Browns, 49ers and
    // Colts of the late 2010s): the bonus becomes a guaranteed first-year
    // salary and the pay falls year by year, so the room is used now and
    // the later years stay clean (proposed, not confirmed: when more than
    // 10% of the cap would still be free after the deal).
    if big && o.years >= 2 && money::capped(start) {
        let spare = super::space(w, o.team, start) - o.apy - money::price_cap(start) * 0.10;
        if spare > 0.0 {
            t.base_guarantee += t.bonus_share;
            t.bonus_share = 0.0;
            t.ramp = -0.10;
            t.void_years = 0;
        }
    }
    // No deal pays more in its first year than real top deals have
    // (`money::FIRST_YEAR_MAX`); the rest of the bonus becomes guaranteed
    // base in the later years (trial 9 bug 11).
    t.limit_first_year(money::price_cap(start) * money::FIRST_YEAR_MAX);
    t
}

/// Signs a negotiated deal.
pub(crate) fn sign_offer(w: &mut World, i: usize, o: &Offer, d: Date, kind: money::Kind, why: &str) {
    let year = w.year;
    let terms = terms_for(w, i, o, year);
    let deal = money::build(o.team, kind, year, &terms);
    let t = o.team;
    let name = w.nfl[t].name;
    let apy = deal.apy();
    let gtd = deal.guaranteed();
    let (former, accrued) = w.people[i].pro.as_ref().map_or((None, 0), |c| (c.former.filter(|_| c.expired), c.accrued));
    super::settle_ended(w, i);
    let p = &mut w.people[i];
    let c = p.pro.as_mut().unwrap();
    let new = c.teams.last() != Some(&t);
    let was = c.team;
    c.team = Some(t);
    c.status = Status::Roster;
    c.contract = Some(deal);
    c.new_team |= new;
    c.paid = apy > money::price_cap(year) * 0.03;
    c.weeks_unsigned = 0;
    c.ideal = None;
    if o.start {
        c.promise = Some((t, year));
    }
    if new {
        c.teams.push(t);
    }
    let age = p.row_age();
    let years = o.years;
    p.note(format!(
        "{} the {name} at {age} ({why}): {years} year{} at ${:.2}M a year, ${:.1}M guaranteed{}.",
        if was == Some(t) { "Re-signed with" } else { "Signed with" },
        if years == 1 { "" } else { "s" },
        apy / 1e6,
        gtd / 1e6,
        if o.start { ", a starting job promised" } else { "" }
    ));
    if was != Some(t) {
        w.clubs[t].roster.push(i);
    }
    if let Some(f) = former
        && f != t
        && accrued >= money::ufa_seasons(year)
        && kind == money::Kind::Veteran
    {
        super::trades::note_cfa(w, i, f, t, apy, d);
    }
    // 3.12 a meddling owner adopts the big signing.
    let owner = &w.clubs[t].office.owner;
    if apy > money::price_cap(year) * 0.08 && owner.meddling > 0.6 && owner.pet.is_none() {
        w.clubs[t].office.owner.pet = Some(i);
    }
    if apy > money::price_cap(year) * 0.06 {
        let who = super::describe(w, i);
        let loud = if apy > money::price_cap(year) * 0.1 { Loudness::Big } else { Loudness::Medium };
        let text = format!(
            "{who} agrees to {}, ${:.1}M (${:.1}M guaranteed).",
            count(years, "year"),
            apy * years as f64 / 1e6,
            gtd / 1e6
        );
        post(w, d, Some(t), Some(i), loud, Kind::Signing, text);
    }
    if w.logging() {
        w.log_why(crate::world::wlog::why::transaction(w, i, "signed", why));
    }
}

// --- Free agency -------------------------------------------------------------------------------------------

/// Players on the open market priced above the street (the street pool's
/// minimum deals stay with `street_signings`). In the tampering window the
/// market is the unrestricted free agents whose deals run out at the new
/// league year.
pub(crate) fn on_market(w: &World, i: usize, tampering: bool) -> bool {
    let p = &w.people[i];
    if p.stage != Stage::Pro || consensus(w, i) < 66.0 {
        return false;
    }
    let Some(c) = p.pro.as_ref() else { return false };
    if tampering {
        c.status == Status::Roster
            && c.accrued >= money::ufa_seasons(w.year)
            && c.contract.as_ref().is_some_and(|k| k.last_year() < w.year)
    } else {
        c.status == Status::FreeAgent && !c.on_waivers
    }
}

/// One day of the market (topic 5): every team bids on the players on its
/// board it can afford; the player weighs each offer against his ideal. An
/// offer that meets it is signed on the dot; one at 85% or more is agreed
/// with a seven-day window in which a better offer can sway him; below
/// that he waits and reads the market, his ideal coming down toward what
/// is on the table. In the tampering window agreements wait for the new
/// league year (real verbal deals).
pub(crate) fn market_day(w: &mut World, d: Date) {
    let year = w.year;
    let y = w.dates.clone();
    let tampering = d < y.league_year.date;
    let opened = y.tampering_open.map_or(y.league_year.date, |m| m.date);
    let days = opened.days_to(d).max(0);
    // Agreements whose window has run out are signed (verbal ones wait for
    // the league year).
    let due: Vec<Pending> =
        w.market.pending.iter().copied().filter(|p| (!p.verbal && p.until <= d) || (p.verbal && !tampering)).collect();
    w.market.pending.retain(|p| !((!p.verbal && p.until <= d) || (p.verbal && !tampering)));
    for p in due {
        if w.people[p.person].pro.as_ref().is_some_and(|c| c.status == Status::FreeAgent && !c.on_waivers) {
            let why = if p.verbal { "agreed in the tampering window" } else { "after a week to weigh it" };
            sign_offer(w, p.person, &p.offer, d, money::Kind::Veteran, why);
            w.market.year_mut(year).signed_new += 1;
        }
    }
    let mut pool: Vec<(usize, f64)> =
        (0..w.people.len()).filter(|&i| on_market(w, i, tampering)).map(|i| (i, consensus(w, i))).collect();
    pool.sort_by(|a, b| b.1.total_cmp(&a.1));
    let teams: Vec<usize> = (0..w.clubs.len()).filter(|&t| super::super::franchise::active_in(w, t, year)).collect();
    let limit = super::roster_limit(w, d);
    let mut committed = vec![0.0; w.clubs.len()];
    for p in &w.market.pending {
        committed[p.offer.team] += p.offer.apy;
    }
    for (i, _) in pool {
        if w.people[i].pro.as_ref().unwrap().ideal.is_none() {
            let id = ideal(w, i, d);
            w.people[i].pro.as_mut().unwrap().ideal = Some(id);
        }
        let ideal = w.people[i].pro.as_ref().unwrap().ideal.unwrap();
        let former = w.people[i].pro.as_ref().unwrap().teams.last().copied();
        let mut offers: Vec<(Offer, f64)> = Vec::new();
        for &t in &teams {
            if w.clubs[t].roster.len() >= limit || fo::yours(w, t, Duty::FreeAgency) || fo::nfl::talking(w, t, i) {
                continue;
            }
            if let Some(o) = make_offer(w, t, i, &ideal, days, committed[t], year) {
                let s = score(w, i, &o, &ideal, former);
                offers.push((o, s));
            }
        }
        // Your offer, when free agency is yours (lane 10).
        if let Some(x) = fo::nfl::seat_offer(w, i, &ideal, former, &committed) {
            offers.push(x);
        }
        let held = w.market.pending.iter().position(|p| p.person == i);
        let Some(&(best, s)) = offers.iter().max_by(|a, b| a.1.total_cmp(&b.1)) else {
            if held.is_none() {
                decay(w, i, d, None);
            }
            continue;
        };
        if let Some(k) = held {
            // Inside his window: a clearly better offer sways him.
            let cur = w.market.pending[k];
            if w.logging() {
                let outcome = if best.team != cur.offer.team && s > cur.score + 0.03 { "swayed" } else { "held" };
                let id = w.people[i].player.id;
                let why = Why::Market { id, ideal, offers: offers.clone(), best: best.team, score: s, outcome };
                w.log_why(why);
            }
            if best.team != cur.offer.team && s > cur.score + 0.03 {
                committed[cur.offer.team] -= cur.offer.apy;
                committed[best.team] += best.apy;
                w.market.pending[k].offer = best;
                w.market.pending[k].score = s;
                w.market.year_mut(year).swayed += 1;
                let (from, to) = (w.nfl[cur.offer.team].name, w.nfl[best.team].name);
                let p = &mut w.people[i];
                p.note(format!("Swayed from the {from} to the {to} inside his week to decide."));
            }
            continue;
        }
        // The agent's move: with no real competition, a less honest agent
        // claims other interest (topic 3 question 1).
        // Your own offer gets your answer from the agent instead (lane 10).
        // Your own offer gets the agent's answer in your talk, never a bluff
        // (lane 10), whoever runs the rest of free agency (SC6).
        let (best, s) = if s < 0.85
            && offers.len() == 1
            && !fo::yours(w, best.team, Duty::FreeAgency)
            && !fo::nfl::talking(w, best.team, i)
        {
            bluff(w, i, best, &ideal, former, d)
        } else {
            (best, s)
        };
        if w.logging() {
            let outcome = match () {
                _ if s >= 1.0 && tampering => "agrees, to sign at the league year",
                _ if s >= 1.0 => "signs",
                _ if s >= 0.85 => "takes a week to weigh it",
                _ => "waits; his ask comes down",
            };
            let id = w.people[i].player.id;
            let why = Why::Market { id, ideal, offers: offers.clone(), best: best.team, score: s, outcome };
            w.log_why(why);
        }
        if s >= 1.0 {
            w.market.year_mut(year).perfect += 1;
            if tampering {
                w.market.pending.push(Pending {
                    person: i,
                    offer: best,
                    score: s,
                    until: y.league_year.date,
                    verbal: true,
                });
            } else {
                sign_offer(w, i, &best, d, money::Kind::Veteran, "the offer met everything he wanted");
                w.market.year_mut(year).signed_new += 1;
            }
            committed[best.team] += best.apy;
        } else if s >= 0.85 {
            let until = if tampering { y.league_year.date } else { d.add(7) };
            w.market.pending.push(Pending { person: i, offer: best, score: s, until, verbal: tampering });
            committed[best.team] += best.apy;
        } else {
            decay(w, i, d, Some(best.apy));
        }
    }
}

/// Unsigned, he reads the market: his ideal comes down toward the best
/// offer on the table, or about 4% a week with none (proposed, not confirmed).
fn decay(w: &mut World, i: usize, d: Date, best: Option<f64>) {
    let year = w.year;
    let accrued = w.people[i].pro.as_ref().map_or(0, |c| c.accrued);
    let min = money::minimum(year, accrued);
    let c = w.people[i].pro.as_mut().unwrap();
    let Some(id) = c.ideal.as_mut() else { return };
    let weeks = id.opened.days_to(d).max(0) as f64 / 7.0;
    id.apy = match best {
        Some(b) => (id.apy * 0.97).max(b * 1.02).min(id.apy),
        None => id.apy * 0.99,
    }
    .max(min);
    id.gtd *= 0.98;
    if weeks > 6.0 {
        id.start = false;
    }
}

/// An agent's bluff (topic 3 question 1): he claims another team is close.
/// The GM reads it by his judgment and his history with the agent. Believed,
/// the team moves up to its number; called, the offer is cut or pulled,
/// the team trusts the agent less, and the player may fire him.
fn bluff(w: &mut World, i: usize, o: Offer, ideal: &Ideal, former: Option<usize>, d: Date) -> (Offer, f64) {
    let year = w.year;
    let Some(a) = w.people[i].pro.as_ref().and_then(|c| c.agent) else {
        return (o, score(w, i, &o, ideal, former));
    };
    let honesty = w.market.agents[a].honesty;
    let id = w.people[i].player.id as u64;
    if key_uniform(w.options.seed as u64 ^ 0xB1FF ^ d.0 as u64, id) > (1.0 - honesty) * 0.6 {
        return (o, score(w, i, &o, ideal, former));
    }
    w.market.year_mut(year).bluffs += 1;
    // At your club you are the GM (people topic 15): your negotiating
    // reads the bluff in his place.
    let yours = w.seat.filter(|s| s.nfl_team == Some(o.team)).and(w.career.as_ref());
    let gm = yours.and_then(|c| w.coaches[c.coach].yours).map_or(w.clubs[o.team].office.gm.skill, |y| y.negotiating);
    let rel = w.market.agents[a].rel[o.team];
    let read = clamp(0.3 + 0.03 * (gm - 10.0) - 0.4 * rel.min(0.0) - 0.1 * rel.max(0.0), 0.05, 0.9);
    let called = key_uniform(w.options.seed as u64 ^ 0xB1F0 ^ d.0 as u64, id) < read;
    if !called {
        let (_, walk) = worth(w, o.team, i);
        let up = Offer { apy: walk.min(o.apy * 1.08), ..o };
        return (up, score(w, i, &up, ideal, former));
    }
    w.market.year_mut(year).bluffs_called += 1;
    w.market.agents[a].rel[o.team] = clamp(rel - 0.2, -1.0, 1.0);
    let pulled = key_uniform(w.options.seed as u64 ^ 0xB1F1 ^ d.0 as u64, id) < 0.25;
    let team = w.nfl[o.team].name;
    let agent = w.market.agents[a].name.clone();
    {
        let p = &mut w.people[i];
        let age = p.row_age();
        p.note(format!(
            "At {age} his agent {agent} claimed other teams were close; the {team} called the bluff and {}.",
            if pulled { "pulled their offer" } else { "cut their offer" }
        ));
    }
    let temper = goals(w, i).temper;
    if key_uniform(w.options.seed as u64 ^ 0xB1F2 ^ d.0 as u64, id) < 0.15 + 0.3 * temper {
        fire_agent(w, i, "a bluff that cost him an offer");
    }
    if pulled {
        return (o, 0.0);
    }
    let cut = Offer { apy: o.apy * 0.94, ..o };
    (cut, score(w, i, &cut, ideal, former))
}

// --- Before free agency: re-signing, tags and tenders -------------------------------------------------------

/// Contracts ending this league year (the season just played was their
/// last): (team, person).
fn expiring(w: &World) -> Vec<(usize, usize)> {
    let year = w.year;
    let mut v = Vec::new();
    for t in 0..w.clubs.len() {
        for &i in &w.clubs[t].roster {
            if let Some(k) = w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref())
                && k.last_year() < year
            {
                v.push((t, i));
            }
        }
    }
    v
}

/// The team talks with its own expiring players before the market (topic
/// 4): its offer against his ideal, with his loyalty and trust in the
/// team. At 85% he re-signs; otherwise he waits for the tag decision or
/// the market. Talks near the deadline close a little lower (most deals
/// come in the last 48 hours). Players before 1993 had no real market:
/// a team that wants him keeps him.
pub(crate) fn resign_talks(w: &mut World, d: Date, deadline: bool) {
    let year = w.year;
    for (t, i) in expiring(w) {
        // Your own players are yours to re-sign (lane 10), and one you're
        // talking to while your staff runs contracts (SC6).
        if fo::yours(w, t, Duty::Contracts) || fo::nfl::talking(w, t, i) {
            continue;
        }
        let c = w.people[i].pro.as_ref().unwrap();
        if c.tags > 0 && c.contract.as_ref().is_some_and(|k| k.kind == money::Kind::Franchise) && !deadline {
            continue;
        }
        // The season just played accrued at the Super Bowl.
        let accrued = c.accrued;
        // Restricted and exclusive-rights players get tenders, not talks.
        if year >= 1993 && accrued < money::ufa_seasons(year) {
            continue;
        }
        // Teams talk early only with the players they build around; the rest
        // test the market, and some come back from it (proposed, not
        // confirmed: a relative grade of 68).
        let (view, _) = worth(w, t, i);
        if market_grade(w, w.people[i].player.position, view) < 68.0 {
            continue;
        }
        let id = ideal(w, i, d);
        let Some(o) = make_offer(w, t, i, &id, 30, 0.0, year) else { continue };
        let s = score(w, i, &o, &id, Some(t));
        // A money-minded player with no tie to the team wants to see what
        // the market pays (proposed, not confirmed size).
        let g = goals(w, i);
        let test = 0.12 * g.money * (1.0 - g.loyalty);
        let bar = if deadline { 0.82 } else { 0.85 } + test;
        if s >= bar || year < 1993 {
            sign_offer(w, i, &o, d, money::Kind::Veteran, "before free agency");
            w.market.year_mut(year).resigned += 1;
        }
    }
}

/// The tag deadline (from 1993): a team tags the expiring star it can't
/// sign and won't lose: a franchise tag (non-exclusive) when he is worth
/// it, a transition tag now and then for the tier below. Restricted
/// players get tenders: a first- or second-round tender for the good
/// ones, the original round (right of first refusal for the undrafted)
/// otherwise; exclusive-rights players a minimum tender.
pub(crate) fn tags_and_tenders(w: &mut World, d: Date) {
    let year = w.year;
    if year < 1993 {
        return;
    }
    let shares = tag_shares_now(w);
    w.market.tag_shares.push((year, shares));
    w.market.tag_shares.retain(|(y, _)| *y > year - 5);
    let bases = tag_bases(&w.market.tag_shares, year);
    let base = |pos: Position, franchise: bool| bases[pos.index()][usize::from(!franchise)];
    let qb_price = base(Position::QB, true);
    // The best expiring players first; one tag a team a year from 2011, a
    // franchise and a transition tag before (from memory, to check).
    let mut list = expiring(w);
    list.sort_by_cached_key(|&(_, i)| std::cmp::Reverse((consensus(w, i) * 100.0) as i64));
    let mut used: std::collections::HashMap<usize, [bool; 2]> = std::collections::HashMap::new();
    // A tag you gave yourself while your staff runs the tags uses your
    // club's tag (SC6).
    if let Some(me) = fo::club(w) {
        let mut u = [false; 2];
        for &i in &w.clubs[me].roster {
            if let Some(k) =
                w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref()).filter(|k| k.signed == year && k.team == me)
            {
                u[0] |= k.kind == money::Kind::Franchise;
                u[1] |= k.kind == money::Kind::Transition;
            }
        }
        used.insert(me, u);
    }
    for (t, i) in list {
        // Your tags and tenders are yours (lane 10), and a talk of yours or
        // a tag you withdrew stands with the staff on the rest (SC6).
        let id = w.people[i].player.id;
        if fo::yours(w, t, Duty::Tags)
            || fo::nfl::talking(w, t, i)
            || (fo::seat_head_club(w, t) && w.front_office.nfl.withdrawn.iter().any(|x| x.0 == id && x.1 == year))
        {
            continue;
        }
        let p = &w.people[i];
        let c = p.pro.as_ref().unwrap();
        let accrued = c.accrued;
        let pos = p.player.position;
        let (view, walk) = worth(w, t, i);
        let view = market_grade(w, pos, view);
        let own = c.contract.as_ref().map_or(0.0, |k| k.cap_hit(year - 1));
        if accrued >= money::ufa_seasons(year) {
            let times = c.tags;
            let franchise = money::tag_repeat(base(pos, true), own, times, if times >= 2 { qb_price } else { 0.0 });
            let transition = money::tag_repeat(base(pos, false), own, times, 0.0);
            // A one-year tag carries no long-term risk and keeps a core
            // player from walking, so a team pays up to half again its
            // long-term number for it (proposed, not confirmed).
            let u = used.get(&t).copied().unwrap_or_default();
            let (f_open, t_open) = if year >= 2011 { (!u[0] && !u[1], !u[0] && !u[1]) } else { (!u[0], !u[1]) };
            let want_f = f_open && view >= 80.0 && walk * 1.5 >= franchise;
            let want_t = t_open
                && view >= 78.0
                && walk * 1.2 >= transition
                && key_uniform(0x7A65, (year as u64) << 20 ^ i as u64) < 0.15;
            if !want_f && !want_t {
                continue;
            }
            // A team makes room for the tag it wants with the restructures
            // and cap casualties of its winter, as real teams do (trial 7
            // finding 6: teams near the cap at the deadline almost never
            // tagged).
            let first = if want_f { franchise } else { transition };
            if space(w, t, year) <= first {
                clear_room(w, t, d, true, first, true);
            }
            let room = space(w, t, year);
            let (kind, price) = if want_f && room > franchise {
                (money::Kind::Franchise, franchise)
            } else if want_t && room > transition {
                (money::Kind::Transition, transition)
            } else {
                continue;
            };
            used.entry(t).or_default()[(kind != money::Kind::Franchise) as usize] = true;
            let deal = money::build(t, kind, year, &{
                let mut x = money::Terms::flat(1, price);
                x.base_guarantee = 1.0;
                x
            });
            let name = w.nfl[t].name;
            super::settle_ended(w, i);
            let p = &mut w.people[i];
            let c = p.pro.as_mut().unwrap();
            c.contract = Some(deal);
            c.tags += 1;
            let age = p.row_age();
            let what = if kind == money::Kind::Franchise { "franchise" } else { "transition" };
            p.note(format!("Given the {what} tag by the {name} at {age}: ${:.2}M for one year.", price / 1e6));
            let my = w.market.year_mut(year);
            if kind == money::Kind::Franchise {
                my.tags += 1
            } else {
                my.transition += 1
            }
            let who = super::describe(w, i);
            post(
                w,
                d,
                Some(t),
                Some(i),
                Loudness::Medium,
                Kind::Signing,
                format!("{who} gets the {what} tag (${:.1}M).", price / 1e6),
            );
        } else if accrued >= 3 && view >= 66.0 {
            let tender = if view >= 80.0 && walk >= money::tender_price(year, money::Tender::First) {
                money::Tender::First
            } else if view >= 74.0 && walk >= money::tender_price(year, money::Tender::Second) {
                money::Tender::Second
            } else if c.draft.is_some() {
                money::Tender::Original
            } else {
                money::Tender::RightOfFirstRefusal
            };
            let price = money::tender_price(year, tender).max(own * 1.1);
            if walk < price * 0.8 {
                continue;
            }
            let deal = money::build(t, money::Kind::Tender, year, &money::Terms::flat(1, price));
            super::settle_ended(w, i);
            let p = &mut w.people[i];
            let c = p.pro.as_mut().unwrap();
            c.contract = Some(deal);
            c.rfa = Some(tender);
            let age = p.row_age();
            let name = w.nfl[t].name;
            p.note(format!(
                "Tendered as a restricted free agent by the {name} at {age} ({tender:?}, ${:.2}M).",
                price / 1e6
            ));
            w.market.year_mut(year).tenders += 1;
        } else if accrued < 3 && view >= 62.0 {
            let price = money::minimum(year, accrued);
            let deal = money::build(t, money::Kind::Tender, year, &money::Terms::flat(1, price));
            super::settle_ended(w, i);
            w.people[i].pro.as_mut().unwrap().contract = Some(deal);
        }
    }
}

/// This year's franchise and transition tag numbers by position as shares
/// of the cap. The 2011 CBA's tag formula reads five years of top cap
/// numbers as a share of each year's cap (fix list S6.3: the tags ran far
/// over real).
pub(crate) fn tag_shares_now(w: &World) -> Vec<[f64; 2]> {
    let cap = money::price_cap(w.year);
    top_cap_hits(w)
        .iter()
        .map(|l| {
            let f = money::tag_price(l, true, 0.0, 0, 0.0) / cap;
            let t = money::tag_price(l, false, 0.0, 0, 0.0) / cap;
            [f, t]
        })
        .collect()
}

/// The tag prices by position (franchise, transition) from the shares'
/// history: the five-year average from 2011, the year's own before.
pub(crate) fn tag_bases(hist: &[(i32, Vec<[f64; 2]>)], year: i32) -> Vec<[f64; 2]> {
    let cap = money::price_cap(year);
    Position::ALL
        .iter()
        .map(|&pos| {
            let one = |k: usize| {
                let hist: Vec<f64> = hist.iter().map(|(_, v)| v[pos.index()][k]).collect();
                let share = if year >= 2011 {
                    hist.iter().sum::<f64>() / hist.len().max(1) as f64
                } else {
                    *hist.last().unwrap_or(&0.0)
                };
                // A position with too few paid veterans in the world still
                // has a real market (the floor sits well under the top five).
                (share * cap).max(cap * money::top5_share(pos) * 0.45)
            };
            [one(0), one(1)]
        })
        .collect()
}

/// Every contract's cap number this year by position, best first (the
/// tag prices).
pub(crate) fn top_cap_hits(w: &World) -> Vec<Vec<f64>> {
    let year = w.year;
    let mut v: Vec<Vec<f64>> = vec![Vec::new(); crate::positions::POSITION_COUNT];
    for p in &w.people {
        if p.stage != Stage::Pro {
            continue;
        }
        if let Some(k) = p.pro.as_ref().and_then(|c| c.contract.as_ref()) {
            v[p.player.position.index()].push(k.cap_hit(year).max(k.cap_hit(year - 1)));
        }
    }
    for l in v.iter_mut() {
        l.sort_by(|a, b| b.total_cmp(a));
        l.truncate(10);
    }
    v
}

/// The league year opens (March): contracts that ran out end. A void year
/// voids and its bonus lands on this year's cap. Unsigned players hit the
/// market; before 1993 a team kept any veteran it wanted (the right of
/// first refusal and compensation), and in the Plan B years (1989-92) only
/// players left off a team's 37 protected went free.
pub(crate) fn open_league_year(w: &mut World, d: Date) {
    let year = w.year;
    for (t, i) in expiring(w) {
        // The bonus still spread over void years (or unspread) accelerates.
        let dead: f64 = w.people[i]
            .pro
            .as_ref()
            .and_then(|c| c.contract.as_ref())
            .map_or(0.0, |k| k.years.iter().filter(|x| x.year >= year).map(|x| x.proration).sum());
        w.clubs[t].add_dead(year, dead, w.people[i].player.id, DeadWhy::Voided);
        let (view, _) = worth(w, t, i);
        let rel = market_grade(w, w.people[i].player.position, view);
        let keep = if year < 1989 {
            rel >= 62.0
        } else if year < 1993 {
            let protected = w.clubs[t].roster.iter().filter(|&&j| grade(w, t, j) > view).count() < 37;
            protected && rel >= 62.0
        } else {
            false
        };
        if keep {
            let id = ideal(w, i, d);
            let o = make_offer(w, t, i, &id, 30, 0.0, year).unwrap_or(Offer {
                team: t,
                apy: id.apy * 0.9,
                years: id.years,
                gtd: 0.0,
                start: false,
            });
            sign_offer(w, i, &o, d, money::Kind::Veteran, "his team held his rights");
            w.market.year_mut(year).resigned += 1;
            continue;
        }
        let ufa = w.people[i].pro.as_ref().unwrap().accrued >= money::ufa_seasons(year);
        if let Some(k) = w.people[i].pro.as_mut().unwrap().contract.as_mut() {
            // Nothing left to charge: the dead money is booked above.
            for y in k.years.iter_mut() {
                y.proration = 0.0;
                y.guaranteed = 0.0;
                y.injury = 0.0;
            }
        }
        release_with(w, i, d, "his contract ran out", false, false, DeadWhy::Released);
        let c = w.people[i].pro.as_mut().unwrap();
        c.former = Some(t);
        c.expired = true;
        if ufa {
            w.market.year_mut(year).ufa += 1;
        }
    }
    rank_totals(w);
}

/// RFA offer sheets (from 1993, until the April deadline): a team that
/// values a tendered player well above his tender and will give up the
/// tender's pick signs him to a sheet; his team has five days to match
/// (real: offer sheets are rare, about one a year).
pub(crate) fn offer_sheets(w: &mut World, d: Date) {
    let year = w.year;
    let y = w.dates.clone();
    // Sheets whose five days ran out.
    let due: Vec<Sheet> = w.market.sheets.iter().copied().filter(|s| s.until <= d).collect();
    w.market.sheets.retain(|s| s.until > d);
    for s in due {
        let (_, walk) = worth(w, s.old, s.person);
        // Your call on a sheet for your own player (lane 10).
        let matched = fo::nfl::sheet_match(w, &s)
            .unwrap_or_else(|| walk >= s.offer.apy * 0.95 && space(w, s.old, year) > s.offer.apy);
        if matched {
            let o = Offer { team: s.old, ..s.offer };
            sign_offer(w, s.person, &o, d, money::Kind::Veteran, "his team matched an offer sheet");
            w.market.year_mut(year).sheets_matched += 1;
        } else {
            let round = match s.tender {
                money::Tender::First => Some(1),
                money::Tender::Second => Some(2),
                money::Tender::Original => w.people[s.person].pro.as_ref().and_then(|c| c.draft).map(|x| x.1),
                money::Tender::RightOfFirstRefusal => None,
            };
            if let Some(r) = round {
                super::trades::give_pick(w, s.offer.team, s.old, year, r);
            }
            w.people[s.person].pro.as_mut().unwrap().rfa = None;
            super::release_with(w, s.person, d, "left on an offer sheet", false, false, DeadWhy::Released);
            sign_offer(w, s.person, &s.offer, d, money::Kind::Veteran, "an offer sheet his team didn't match");
        }
    }
    if d > y.rfa_deadline || d < y.league_year.date {
        return;
    }
    let rfas: Vec<usize> = (0..w.people.len())
        .filter(|&i| w.people[i].pro.as_ref().is_some_and(|c| c.rfa.is_some() && c.team.is_some()))
        .collect();
    for i in rfas {
        if w.market.sheets.iter().any(|s| s.person == i) {
            continue;
        }
        let c = w.people[i].pro.as_ref().unwrap();
        let old = c.team.unwrap();
        let tender = c.rfa.unwrap();
        let price = c.contract.as_ref().map_or(0.0, |k| k.apy());
        let id = ideal(w, i, d);
        let pick_cost = match tender {
            money::Tender::First => 1.6,
            money::Tender::Second => 1.35,
            _ => 1.15,
        };
        // Teams rarely try: the picks, and a match is likely (real: about
        // one sheet a year). One chance a day for each tendered player.
        if key_uniform(0x5EE7 ^ d.0 as u64, i as u64) > 0.004 {
            continue;
        }
        for t in 0..w.clubs.len() {
            if t == old || !super::super::franchise::active_in(w, t, year) || fo::yours(w, t, Duty::Tags) {
                continue;
            }
            let (_, walk) = worth(w, t, i);
            if walk < price * pick_cost * 1.4 {
                continue;
            }
            let Some(mut o) = make_offer(w, t, i, &id, 20, 0.0, year) else { continue };
            // Terms his team can't easily match (the poison pill).
            o.gtd = (o.gtd + 0.15).min(0.9);
            w.market.sheets.push(Sheet { person: i, offer: o, old, tender, until: d.add(5) });
            w.market.year_mut(year).sheets += 1;
            let who = super::describe(w, i);
            let to = w.nfl[t].name;
            post(
                w,
                d,
                Some(old),
                Some(i),
                Loudness::Medium,
                Kind::Signing,
                format!("{who} signs an offer sheet with the {to}."),
            );
            break;
        }
    }
    if d == y.rfa_deadline {
        for i in 0..w.people.len() {
            if let Some(c) = w.people[i].pro.as_mut()
                && c.rfa.is_some()
                && !w.market.sheets.iter().any(|s| s.person == i)
            {
                c.rfa = None;
            }
        }
    }
}

// --- Under contract: options, extensions, holdouts ------------------------------------------------------------

/// The fifth-year option (2011 CBA on; decided by early May of a round-one
/// pick's fourth season). Its price: before the 2020 CBA the top-ten tier
/// for a top-ten pick and the 3rd-25th average for the rest; from 2021
/// decisions the tiers by Pro Bowls (two: the franchise tag; one: the
/// transition tag) and playing time (prices read as shares of the top five).
pub(crate) fn fifth_year_options(w: &mut World, d: Date) {
    let year = w.year;
    if year < 2014 {
        return;
    }
    let list: Vec<(usize, usize)> = (0..w.people.len())
        .filter_map(|i| {
            let c = w.people[i].pro.as_ref()?;
            let k = c.contract.as_ref()?;
            (k.kind == money::Kind::Rookie
                && k.option.is_none()
                && c.draft.is_some_and(|x| x.1 == 1 && x.0 == year - 3))
            .then_some((c.team?, i))
        })
        .collect();
    for (t, i) in list {
        let price = option_price(w, i);
        let (_, walk) = worth(w, t, i);
        // At your club the choice is yours (lane 10): one you left
        // undecided at the deadline is declined, as the rule has it.
        let take = super::super::front_office::nfl::option_choice(w, t, i).unwrap_or(walk >= price * 0.95);
        set_option(w, t, i, take, price);
    }
    let _ = d;
}

/// Picks up or declines a fifth-year option: picked up, the option year
/// joins the deal.
pub(crate) fn set_option(w: &mut World, t: usize, i: usize, take: bool, price: f64) {
    let year = w.year;
    let c = w.people[i].pro.as_mut().unwrap();
    let k = c.contract.as_mut().unwrap();
    k.option = Some(take);
    if take {
        let last = k.last_year();
        k.years.push(money::Year {
            year: last + 1,
            base: price,
            // Guaranteed for injury until the 2020 CBA, fully after.
            guaranteed: if year >= 2021 { price } else { 0.0 },
            injury: if year >= 2021 { 0.0 } else { price },
            vests: Some(money::vest_day(last + 1)),
            ..Default::default()
        });
        k.years.sort_by_key(|y| y.year);
    }
    let name = w.nfl[t].name;
    let p = &mut w.people[i];
    let age = p.row_age();
    p.note(format!(
        "The {name} {} his fifth-year option at {age} (${:.2}M).",
        if take { "picked up" } else { "declined" },
        price / 1e6
    ));
    let my = w.market.year_mut(year);
    if take { my.options_taken += 1 } else { my.options_declined += 1 }
}

/// A round-one pick's fifth-year option price (see `fifth_year_options`).
pub(crate) fn option_price(w: &World, i: usize) -> f64 {
    let year = w.year;
    let p = &w.people[i];
    let pos = p.player.position;
    let pick = p.pro.as_ref().and_then(|c| c.draft).map_or(32, |d| d.2);
    let bowls = p.pro.as_ref().map_or(0, |c| c.seasons.iter().filter(|s| s.pro_bowl).count());
    let top5 = money::price_cap(year + 1) * money::top5_share(pos);
    if year >= 2021 {
        match bowls {
            0 => top5 * 0.5,
            1 => top5 * 0.72,
            _ => top5 * 0.85,
        }
    } else if pick <= 10 {
        top5 * 0.72
    } else {
        top5 * 0.5
    }
}

/// Extension talks (topic 4, the NFL year): a team extends a good player
/// in his last year, a GM with the Eagles' habit two years out, a rookie
/// star after his third season (not before: 2011 CBA). The player's ideal
/// is his market price, a little less for the security of an early deal.
/// Near a deadline (camp, week 1) both sides bend a little.
pub(crate) fn extension_talks(w: &mut World, d: Date, deadline: bool) {
    let year = w.year;
    let list: Vec<(usize, usize)> = (0..w.people.len())
        .filter_map(|i| {
            let p = &w.people[i];
            let c = p.pro.as_ref()?;
            let t = c.team?;
            let k = c.contract.as_ref()?;
            if !c.held()
                || k.team != t
                || matches!(k.kind, money::Kind::Minimum | money::Kind::Future | money::Kind::PracticeSquad)
            {
                return None;
            }
            // A deal signed this league year is settled (trial 7 finding
            // 11: the summer's later talks extended a one-year extension
            // again; trial 9: 538 players signed or re-signed in the spring
            // and extended by the same team that summer). Only a tag is
            // there to be replaced.
            let tagged = matches!(k.kind, money::Kind::Franchise | money::Kind::Transition);
            if k.signed >= year && !tagged {
                return None;
            }
            let left = k.last_year() - year;
            if tagged && d > w.dates.tag_long_term_deadline.date {
                return None;
            }
            let rookie = k.kind == money::Kind::Rookie;
            let seasons = c.seasons.iter().filter(|s| s.games > 0).count();
            if rookie && (year >= 2011 && seasons < 3) {
                return None;
            }
            let early = w.clubs[t].office.gm.early_extensions;
            (left == 0 || (left == 1 && (early > 0.55 || rookie && seasons >= 3))).then_some((t, i))
        })
        .collect();
    for (t, i) in list {
        // Your own extensions are yours (lane 10), and a talk of yours (SC6).
        if fo::yours(w, t, Duty::Contracts) || fo::nfl::talking(w, t, i) {
            continue;
        }
        let (view, walk) = worth(w, t, i);
        if market_grade(w, w.people[i].player.position, view) < 76.0 {
            continue;
        }
        let mut id = ideal(w, i, d);
        let k = w.people[i].pro.as_ref().unwrap().contract.as_ref().unwrap();
        let left = k.last_year() - year;
        // The new deal replaces what next year's old deal would count, and
        // the team keeps next year's room for its draft class and its own
        // free agents (proposed, not confirmed: 8% of next year's cap).
        let freed = k.cap_hit(year + 1);
        let keep = money::price_cap(year + 1) * 0.08;
        id.apy *= 1.0 - 0.04 * left as f64;
        let Some(mut o) = make_offer(w, t, i, &id, 40, keep - freed, year + 1) else { continue };
        // An extension adds at least two years, except for a player past
        // 32 (trial 9 finding 7: 995 of 4,384 extensions added one year;
        // real one-year extensions are rare and for older veterans).
        if w.people[i].player.age <= 32 {
            o.years = o.years.max(2);
        }
        // A tag is replaced only by a long-term deal (trial 9: 31 tags were
        // "replaced" by a one-year deal); a one-year deal is the tag itself.
        if o.years < 2
            && w.people[i]
                .pro
                .as_ref()
                .and_then(|c| c.contract.as_ref())
                .is_some_and(|k| matches!(k.kind, money::Kind::Franchise | money::Kind::Transition))
        {
            continue;
        }
        o.apy = o.apy.min(walk);
        let s = score(w, i, &o, &id, Some(t));
        let bar = if deadline { 0.82 } else { 0.87 };
        if s < bar {
            continue;
        }
        extend(w, i, &o, d);
        let rookie = w.people[i].pro.as_ref().unwrap().draft.is_some_and(|x| x.0 >= year - 4);
        let my = w.market.year_mut(year);
        if rookie {
            my.rookie_extensions += 1
        } else {
            my.extensions += 1
        }
        let c = w.people[i].pro.as_mut().unwrap();
        c.hold = Hold::None;
    }
}

/// An extension: this year's pay stays, the new deal starts next league
/// year, and the old deal's later bonus money rides along. A tagged
/// player's long-term deal replaces the tag instead and starts this
/// season (real: the tender is withdrawn; the bug check of build 9
/// found the tag's salary paid on top of the new deal's bonus, 51% of
/// the cap in one season).
pub(crate) fn extend(w: &mut World, i: usize, o: &Offer, d: Date) {
    let year = w.year;
    let tagged =
        w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref()).is_some_and(|k| {
            matches!(k.kind, money::Kind::Franchise | money::Kind::Transition) && k.last_year() == year
        });
    let start = if tagged { year } else { year + 1 };
    let terms = terms_for(w, i, o, start);
    let mut new = money::build(o.team, money::Kind::Extension, start, &terms);
    let name = w.nfl[o.team].name;
    let p = &mut w.people[i];
    let c = p.pro.as_mut().unwrap();
    let old = c.contract.take().unwrap();
    let mut years: Vec<money::Year> = old.years.iter().copied().filter(|y| y.year < start).collect();
    // The old bonus keeps counting in the years it was spread over (a year
    // past the new deal's end lands on its last year).
    for oy in old.years.iter().filter(|y| y.year >= start) {
        if let Some(ny) = new.years.iter_mut().find(|n| n.year == oy.year) {
            ny.proration += oy.proration;
        } else if let Some(last) = new.years.last_mut() {
            last.proration += oy.proration;
        }
    }
    // The new bonus is paid now, at signing, in this league year (fix list
    // S6.3: the old deal's bonuses were paid already and are not paid again).
    let bonus = new.years.first().map_or(0.0, |y| y.bonus_paid);
    if let Some(now) = years.iter_mut().rev().find(|y| y.year == year && !y.void) {
        now.bonus_paid += bonus;
        if let Some(y1) = new.years.first_mut() {
            y1.bonus_paid = 0.0;
        }
    }
    years.append(&mut new.years);
    new.years = years;
    new.option = old.option;
    let apy = o.apy;
    c.contract = Some(new);
    c.paid = true;
    let age = p.row_age();
    p.note(if tagged {
        format!(
            "Signed a long-term deal with the {name} at {age}, replacing the tag: {} at ${:.2}M a year.",
            count(o.years, "year"),
            apy / 1e6
        )
    } else {
        format!("Extended by the {name} at {age}: {} at ${:.2}M a year.", count(o.years, "more year"), apy / 1e6)
    });
    if apy > money::price_cap(year) * 0.06 {
        let who = super::describe(w, i);
        post(
            w,
            d,
            Some(o.team),
            Some(i),
            Loudness::Medium,
            Kind::Signing,
            format!("{who} signs an extension: {}, ${:.1}M.", count(o.years, "year"), apy * o.years as f64 / 1e6),
        );
    }
    if w.logging() {
        w.log_why(crate::world::wlog::why::transaction(w, i, "extended", ""));
    }
}

/// Holding out or holding in (topic 3 question 4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Hold {
    #[default]
    None,
    /// At camp but not practising (Brandon Aiyuk 2024).
    In,
    /// Away from the team (CeeDee Lamb 2024).
    Out,
}

/// Camp report day (A4) and each check after: a star paid well under his
/// market weighs how underpaid he is, years left, his leverage (age, the
/// team's need), the fines and his personality. Since the 2020 CBA a
/// player on his rookie deal can't have camp fines forgiven, so most hold
/// in. Each later check a holdout either gets his deal (extension talks),
/// gives in (likelier every week, and in season he misses games), or asks
/// for a trade.
pub(crate) fn holdouts(w: &mut World, d: Date, report_day: bool) {
    let year = w.year;
    let pros: Vec<usize> = (0..w.people.len())
        .filter(|&i| w.people[i].pro.as_ref().is_some_and(|c| c.held() && c.status == Status::Roster))
        .collect();
    for i in pros {
        let c = w.people[i].pro.as_ref().unwrap();
        let t = c.team.unwrap();
        let hold = c.hold;
        if report_day && hold == Hold::None {
            let Some(k) = c.contract.as_ref() else { continue };
            let seasons = c.seasons.iter().filter(|s| s.games > 0).count();
            let view = market_grade(w, w.people[i].player.position, consensus(w, i));
            let market = market_price(year, w.people[i].player.position, view, c.accrued, w.people[i].player.age);
            let under = market / k.apy().max(1.0);
            // Stars only, far under their market (real: a handful a year,
            // Aiyuk and Lamb in 2024).
            if view < 85.0 || under < 2.0 || seasons < 3 {
                continue;
            }
            let g = goals(w, i);
            let age = w.people[i].player.age;
            let leverage = if age <= 29 { 1.0 } else { 0.6 };
            let mood = w.people[i].mood as f64;
            let push =
                g.greed * (under / 3.0).min(1.0) * leverage - 0.3 * g.loyalty * c.trust + 0.1 * g.temper - 0.15 * mood;
            if push < 0.3 {
                continue;
            }
            let rookie = k.kind == money::Kind::Rookie;
            let paid = k.apy();
            let out = if year >= 2020 && rookie { push > 0.55 } else { push > 0.4 };
            let name = w.nfl[t].name;
            let p = &mut w.people[i];
            let c = p.pro.as_mut().unwrap();
            c.hold = if out { Hold::Out } else { Hold::In };
            let row = p.row_age();
            p.note(format!(
                "{} the {} camp at {row}, wanting a new deal (paid ${:.1}M against a market near ${:.1}M).",
                if out { "Held out of" } else { "Held in at" },
                super::possessive(name),
                paid / 1e6,
                market / 1e6
            ));
            let my = w.market.year_mut(year);
            if out {
                my.holdouts += 1
            } else {
                my.holdins += 1
            }
            let who = super::describe(w, i);
            post(
                w,
                d,
                Some(t),
                Some(i),
                Loudness::Medium,
                Kind::News,
                format!("{who} {} for a new contract.", if out { "holds out" } else { "holds in" }),
            );
            continue;
        }
        if hold == Hold::None || report_day {
            continue;
        }
        // A week later: he gives in, more likely the longer it runs and
        // once games are missed (fines, a lost accrued season).
        let since = d.days_to(w.dates.veterans_report).abs() as f64 / 7.0;
        let in_season = d >= w.dates.nfl_kickoff;
        let give = clamp(0.08 + 0.05 * since + if in_season { 0.15 } else { 0.0 }, 0.0, 0.9);
        let id = w.people[i].player.id as u64;
        if key_uniform(0x601D ^ d.0 as u64, id) < give {
            let p = &mut w.people[i];
            p.pro.as_mut().unwrap().hold = Hold::None;
            let age = p.row_age();
            p.note(format!(
                "Ended his {} at {age} without a new deal.",
                if hold == Hold::Out { "holdout" } else { "hold-in" }
            ));
        } else if hold == Hold::Out {
            let c = w.people[i].pro.as_mut().unwrap();
            c.sharpness = clamp(c.sharpness - 0.04, 0.5, 1.0);
            // A rebuilding team or a broken relationship: he asks out.
            if (w.clubs[t].office.mode == Mode::Rebuild || c.trust < 0.5) && !c.wants_out {
                c.wants_out = true;
                let who = super::describe(w, i);
                post(w, d, Some(t), Some(i), Loudness::Medium, Kind::News, format!("{who} requests a trade."));
            }
        }
    }
}

/// A promise of a starting job (topic 4 question 2), checked in week 1: a
/// broken one costs his trust in the team and his agent's goodwill toward it.
pub(crate) fn check_promises(w: &mut World) {
    let year = w.year;
    for i in 0..w.people.len() {
        let Some((t, y)) = w.people[i].pro.as_ref().and_then(|c| c.promise) else { continue };
        if y != year {
            continue;
        }
        let pos = w.people[i].player.position;
        let starts = w.clubs[t]
            .depth
            .get(pos.index())
            .is_some_and(|d| d.iter().take(depth_slots(pos).starters).any(|&j| j == i));
        let here = w.people[i].pro.as_ref().unwrap().team == Some(t);
        let agent = w.people[i].pro.as_ref().unwrap().agent;
        let c = w.people[i].pro.as_mut().unwrap();
        c.promise = None;
        if starts || !here {
            continue;
        }
        c.trust = clamp(c.trust - 0.3, 0.0, 1.0);
        if let Some(a) = agent {
            w.market.agents[a].rel[t] = clamp(w.market.agents[a].rel[t] - 0.1, -1.0, 1.0);
        }
        w.market.year_mut(year).promises_broken += 1;
        let name = w.nfl[t].name;
        let p = &mut w.people[i];
        let age = p.row_age();
        p.note(format!("The {name} broke their promise of a starting job at {age}."));
    }
}

// --- The cap -----------------------------------------------------------------------------------------------

/// Getting under the cap before the league year (topic 7 styles): first the
/// restructures (a GM with the Saints' habit does them every year, with
/// void years from 2016), then the cap casualties: players whose cap saving
/// is worth more than what they bring, cut before their roster bonus is
/// due (A1). From 2006 up to two releases a year can be designated post-June
/// 1, splitting the dead money (A2; proposed, not confirmed start year).
/// Each team keeps room for its rookies and, if contending, for the market.
/// On opening day (`before_league_year` false) every contract counts, not
/// just the top 51, and a team only has to get under (real rule).
pub(crate) fn cap_moves(w: &mut World, d: Date, before_league_year: bool) {
    let year = w.year;
    if !money::cap_books(year) {
        return;
    }
    for t in 0..w.clubs.len() {
        // Your cap is yours until opening day, when the league makes every
        // club get under (lane 10: your cap manager then does what it takes).
        if super::super::franchise::active_in(w, t, year) && !(before_league_year && fo::yours(w, t, Duty::Cap)) {
            clear_room(w, t, d, before_league_year, 0.0, true);
        }
    }
}

/// What a deal puts on a later league year's books: its hit while it
/// runs, and the year after its last, the void-year bonus that lands then
/// (as `payroll` counts it).
fn next_year_charge(k: &money::Contract, y: i32) -> f64 {
    let last = k.last_year();
    if last >= y {
        k.cap_hit(y)
    } else if last + 1 == y {
        k.years.iter().filter(|x| x.year >= y).map(|x| x.proration).sum()
    } else {
        0.0
    }
}

/// One team's cap moves (see `cap_moves`), clearing `extra` on top of its
/// usual target: the price of a tag it means to place (trial 7 finding 6)
/// or of a player it must sign. Without `cuts` it only restructures.
/// Returns the room still missing.
pub(crate) fn clear_room(w: &mut World, t: usize, d: Date, before_league_year: bool, extra: f64, cuts: bool) -> f64 {
    let year = w.year;
    let cap = money::price_cap(year);
    let o = &w.clubs[t].office;
    // Before the league year a team clears room for its draft class
    // (about 4.5% of the cap) and the market, a contender more (proposed,
    // not confirmed: 3% and 6% on top).
    let target =
        if before_league_year { cap * (0.075 + if o.mode == Mode::Contend { 0.03 } else { 0.0 }) } else { 0.0 };
    let habit = if before_league_year { o.gm.restructures } else { 0.0 };
    let mut short = target + extra - space(w, t, year);
    // Void years (from 2016) let even a deal in its last year spread a
    // bonus: a GM with the habit uses them, and any GM who must get
    // under on opening day.
    let voids = if year >= 2016 && (habit > 0.5 || !before_league_year) { 2 } else { 0 };
    // Restructures, biggest base salaries of the core first.
    // Your club's moves, told to you (lane 10): what was done for you.
    let mine = fo::club(w) == Some(t);
    let mut told: Vec<(usize, String)> = Vec::new();
    let mut core: Vec<(usize, f64)> = w.clubs[t]
        .roster
        .iter()
        .filter_map(|&i| {
            let k = w.people[i].pro.as_ref()?.contract.as_ref()?;
            let y = k.year(year)?;
            // On opening day any deal will do.
            let core = before_league_year && y.base > money::minimum(year, 4) * 3.0 && grade(w, t, i) >= 74.0;
            let any = !before_league_year && y.base > money::minimum(year, 4) * 2.0;
            let spreads = k.last_year() > year || voids > 0;
            (k.team == t && spreads && (core || any)).then_some((i, y.base))
        })
        .collect();
    core.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut done = 0;
    let mut kept = Vec::new();
    // Next year's books: its contracts, the void years that land then and
    // its dead money.
    let next = year + 1;
    let mut books_next = if before_league_year { payroll(w, t, next) } else { 0.0 };
    for (i, _) in core {
        if short <= 0.0 && !(habit > 0.7 && done < 2) {
            break;
        }
        let keep = money::minimum(year, 4);
        // Before the league year a club converts only what its later books
        // can carry: no later year's hit past `money::CAP_HIT_MAX` of that
        // year's cap (trial 9 finding 6: restructures stacked hits to 37%),
        // and no more pushed into next year than next year's cap leaves
        // room for. A club already over next year's cap cuts instead (real:
        // a team in cap trouble eats dead money now rather than push more
        // forward). Without it a club restructured into void years every
        // February until they landed at once: $206M of dead money, 74% of
        // the cap, in a test world. On opening day it must get under
        // whatever it costs.
        if before_league_year {
            let k = w.people[i].pro.as_ref().unwrap().contract.as_ref().unwrap();
            let mut trial = k.clone();
            money::restructure(&mut trial, year, keep, voids);
            if trial.years.iter().any(|y| y.year > year && y.cap_hit() > money::price_cap(y.year) * money::CAP_HIT_MAX)
            {
                continue;
            }
            let pushed = next_year_charge(&trial, next) - next_year_charge(k, next);
            if pushed > 0.0 && books_next + pushed > money::price_cap(next) {
                continue;
            }
            books_next += pushed;
        }
        let saved = {
            let k = w.people[i].pro.as_mut().unwrap().contract.as_mut().unwrap();
            money::restructure(k, year, keep, voids)
        };
        if saved > 0.0 {
            short -= saved;
            done += 1;
            kept.push(i);
            w.market.year_mut(year).restructures += 1;
            let p = &mut w.people[i];
            let age = p.row_age();
            p.note(format!(
                "Restructured at {age}: ${:.1}M of salary turned into bonus to clear cap room.",
                saved / 1e6
            ));
            if mine {
                told.push((i, format!("restructured {} (${:.1}M of room)", p.name, saved / 1e6)));
            }
        }
    }
    if short <= 0.0 || !cuts {
        tell_cap_moves(w, d, before_league_year, &told);
        return short;
    }
    // Cap casualties: the most cap saved for the least lost. A team over
    // the cap itself must get under by the new league year (real), so
    // then any player can go.
    // A player just restructured stays: the team kept him on purpose.
    // From June 1 a release splits its dead money by itself (real).
    let over = space(w, t, year) < 0.0;
    let split = super::after_june1(w, d);
    let mut cuts: Vec<(usize, f64, f64, bool)> = w.clubs[t]
        .roster
        .iter()
        .filter_map(|&i| {
            let c = w.people[i].pro.as_ref()?;
            let k = c.contract.as_ref()?;
            if k.team != t || w.clubs[t].office.owner.pet == Some(i) || kept.contains(&i) {
                return None;
            }
            // A deal signed this league year stays before it opens: a team
            // doesn't guarantee a player money in February to cut him in
            // March (real; it restructures or cuts someone else).
            if before_league_year && k.signed >= year {
                return None;
            }
            let save = k.cut_savings(year, d, split);
            let (_, walk) = worth(w, t, i);
            let surplus = walk - k.cap_hit(year);
            // Before the league year only players worth little more than
            // their hit go (real: about 100-150 cap casualties a year);
            // on opening day a team must get under whatever it costs.
            let thin = surplus < 0.2 * k.cap_hit(year);
            // On opening day any saving counts: the hard cap is absolute.
            let worth_it =
                if before_league_year { save > money::minimum(year, 4) && (thin || over) } else { save > 0.0 };
            worth_it.then_some((i, save, surplus, thin))
        })
        .collect();
    cuts.sort_by(|a, b| (a.2 - a.1).total_cmp(&(b.2 - b.1)));
    let mut june1 = 0;
    for (i, save, _, thin) in cuts {
        if short <= 0.0 {
            break;
        }
        // A player worth his deal goes only while the team is over the cap.
        if before_league_year && !thin && short <= target {
            continue;
        }
        let k = w.people[i].pro.as_ref().unwrap().contract.as_ref().unwrap();
        let (now, _) = k.dead_if_cut(year, d, false);
        let designate = !split && year >= 2006 && june1 < 2 && now > save * 0.5;
        let saved = if designate { k.cut_savings(year, d, true) } else { save };
        if mine {
            let (dead, next) = k.dead_if_cut(year, d, designate);
            let name = &w.people[i].name;
            let mut line = format!("released {name}, a cap casualty (${:.1}M saved", saved / 1e6);
            if dead >= 0.05e6 {
                line += &format!(", ${:.1}M dead", dead / 1e6);
            }
            if next >= 0.05e6 {
                line += &format!(" and ${:.1}M next year after June 1", next / 1e6);
            }
            line.push(')');
            told.push((i, line));
        }
        release_with(
            w,
            i,
            d,
            if designate { "a cap casualty, designated post-June 1" } else { "a cap casualty" },
            designate,
            true,
            DeadWhy::CapCut,
        );
        short -= saved;
        if designate {
            june1 += 1;
            w.market.year_mut(year).june1 += 1;
        }
        w.market.year_mut(year).cap_cuts += 1;
    }
    tell_cap_moves(w, d, before_league_year, &told);
    short
}

/// One line to your inbox for the moves your cap manager made on your club
/// (lane 10), every player named a link.
fn tell_cap_moves(w: &mut World, d: Date, before_league_year: bool, told: &[(usize, String)]) {
    if told.is_empty() {
        return;
    }
    let mut list: Vec<&str> = told.iter().map(|(_, s)| s.as_str()).collect();
    let last = if list.len() > 1 { list.pop() } else { None };
    let mut moves = list.join(", ");
    if let Some(l) = last {
        moves = format!("{moves} and {l}");
    }
    let who: Vec<usize> = told.iter().map(|(i, _)| *i).collect();
    let text = if before_league_year {
        format!(
            "Your cap manager {moves}, to keep room for the draft class and the market. Cap moves are with your staff; take them back on your front office page."
        )
    } else if d == w.dates.nfl_kickoff {
        format!(
            "From the first game every contract counts against the cap, not only the top 51: your cap manager {moves} to get under it."
        )
    } else {
        format!("Your cap manager {moves}, to make room to sign a player the roster needs.")
    };
    fo::tell_about(w, d, text, false, &who);
}

// --- After the season: reserve/future deals --------------------------------------------------------------------

/// Reserve/future deals (January, from the day after a team's last game):
/// practice squad players the team still wants and a few young players off
/// the street sign one-year minimum deals for next season; the rest of the
/// squad is let go (real: practice squad deals end with the season).
pub(crate) fn futures(w: &mut World, d: Date, teams: &[usize]) {
    let year = w.year;
    for &t in teams {
        let squad: Vec<usize> = w.clubs[t]
            .roster
            .iter()
            .copied()
            .filter(|&i| w.people[i].pro.as_ref().is_some_and(|c| c.status == Status::PracticeSquad))
            .collect();
        let mut signed = 0;
        for i in squad {
            let keep = grade(w, t, i) >= 58.0 && w.people[i].player.age <= 28;
            if keep {
                future_deal(w, i, t, year);
                signed += 1;
            } else {
                release_with(w, i, d, "his practice squad deal ended", false, false, DeadWhy::Released);
            }
        }
        // Young players off the street to the 90 (proposed: up to about a dozen a team).
        let mut street: Vec<(usize, f64)> = (0..w.people.len())
            .filter(|&i| {
                let p = &w.people[i];
                p.stage == Stage::Pro
                    && p.player.age <= 27
                    && p.pro.as_ref().is_some_and(|c| c.status == Status::FreeAgent && !c.on_waivers)
            })
            .map(|i| (i, grade(w, t, i)))
            .filter(|x| x.1 >= 60.0)
            .collect();
        street.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (i, _) in street.into_iter().take(12usize.saturating_sub(signed).min(4)) {
            if w.people[i].pro.as_ref().is_some_and(|c| c.status == Status::FreeAgent) {
                let c = w.people[i].pro.as_mut().unwrap();
                c.team = None;
                future_deal(w, i, t, year);
            }
        }
    }
    let _ = d;
}

fn future_deal(w: &mut World, i: usize, t: usize, year: i32) {
    let accrued = w.people[i].pro.as_ref().map_or(0, |c| c.accrued);
    let deal =
        money::build(t, money::Kind::Future, year + 1, &money::Terms::flat(1, money::minimum(year + 1, accrued)));
    let name = w.nfl[t].name;
    let p = &mut w.people[i];
    let c = p.pro.as_mut().unwrap();
    let new = c.team != Some(t);
    c.team = Some(t);
    c.status = Status::Roster;
    c.contract = Some(deal);
    if new {
        c.teams.push(t);
        c.new_team = true;
        w.clubs[t].roster.push(i);
    }
    let age = p.row_age();
    p.note(format!("Signed a reserve/future deal with the {name} at {age}."));
    w.market.year_mut(year).futures += 1;
}

// --- Waivers -------------------------------------------------------------------------------------------------------

/// Whether a released player goes on waivers (real): under four accrued
/// seasons always; veterans only after the trade deadline.
pub(crate) fn waivable(w: &World, i: usize, d: Date) -> bool {
    let accrued = w.people[i].pro.as_ref().map_or(0, |c| c.accrued);
    accrued < 4 || (d > w.dates.trade_deadline.date && d <= w.dates.super_bowl)
}

/// The waiver claims (each day at 4pm, real): teams in waiver order (last
/// season's draft order until week 3, then this season's standings) claim
/// a player they see as an upgrade at his position and take over his
/// contract; a claiming team at its limit lets its worst player there go.
pub(crate) fn claims(w: &mut World, d: Date) {
    if !std::mem::take(&mut w.market.waivers) {
        return;
    }
    let year = w.year;
    let mut list: Vec<usize> =
        (0..w.people.len()).filter(|&i| w.people[i].pro.as_ref().is_some_and(|c| c.on_waivers)).collect();
    // In a career a player waived today waits a day on the wire, as the
    // real claim period does, so you can see him and claim (lane 10).
    let held = fo::roster::held_on_wire(w, d);
    if !held.is_empty() {
        list.retain(|&i| !held.contains(&w.people[i].player.id));
        w.market.waivers = true;
    }
    if list.is_empty() {
        return;
    }
    let order = waiver_order(w, d);
    let limit = super::roster_limit(w, d);
    let mut list: Vec<(usize, f64)> = list.into_iter().map(|i| (i, consensus(w, i))).collect();
    list.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (i, _) in list {
        w.people[i].pro.as_mut().unwrap().on_waivers = false;
        let from = w.people[i].pro.as_ref().unwrap().former;
        let pos = w.people[i].player.position;
        let mut claimed = None;
        for &t in &order {
            if Some(t) == from {
                continue;
            }
            // Your claims are yours (lane 10), and stand while your staff
            // works the rest of the wire (SC6).
            if fo::seat_head_club(w, t) {
                if let Some(drop) = fo::roster::seat_claim(w, t, i, limit) {
                    claimed = Some((t, drop));
                    break;
                }
                if fo::yours(w, t, Duty::Waivers) {
                    continue;
                }
            }
            let g = grade(w, t, i);
            let active: Vec<usize> = w.clubs[t]
                .roster
                .iter()
                .copied()
                .filter(|&j| {
                    w.people[j].player.position == pos
                        && w.people[j].pro.as_ref().is_some_and(|c| c.status == Status::Roster && c.out_weeks == 0)
                })
                .collect();
            let worst = active.iter().copied().min_by(|&a, &b| grade(w, t, a).total_cmp(&grade(w, t, b)));
            let worst_g = worst.map_or(0.0, |j| grade(w, t, j));
            let short = active.len() < depth_slots(pos).roster;
            let upgrade = g >= worst_g + 2.0 || (short && g >= 60.0);
            let room = w.clubs[t].roster.len() < limit || worst.is_some();
            let cost = w.people[i].pro.as_ref().unwrap().contract.as_ref().map_or(0.0, |k| k.cap_hit(year));
            if upgrade && room && space(w, t, year) > cost {
                claimed = Some((t, if w.clubs[t].roster.len() >= limit { worst } else { None }));
                break;
            }
        }
        let Some((t, drop)) = claimed else {
            // He clears waivers: his old team owes what was guaranteed.
            let c = w.people[i].pro.as_mut().unwrap();
            if let (Some(k), Some(f)) = (c.contract.take(), from) {
                let owed = k.guaranteed_left(year, d);
                w.clubs[f].add_dead(year, owed, w.people[i].player.id, DeadWhy::Waived);
            }
            continue;
        };
        if let Some(j) = drop {
            release_with(w, j, d, "let go after a waiver claim", false, false, DeadWhy::Released);
            w.people[j].pro.as_mut().unwrap().on_waivers = false;
        }
        let name = w.nfl[t].name;
        let p = &mut w.people[i];
        let c = p.pro.as_mut().unwrap();
        c.team = Some(t);
        c.status = Status::Roster;
        c.teams.push(t);
        c.new_team = true;
        c.weeks_unsigned = 0;
        if let Some(k) = c.contract.as_mut() {
            k.team = t;
        }
        let age = p.row_age();
        p.note(format!("Claimed off waivers by the {name} at {age}."));
        w.clubs[t].roster.push(i);
        w.market.year_mut(year).waiver_claims += 1;
    }
}

/// The waiver order: worst record first (last season's draft order until
/// week 3).
pub(crate) fn waiver_order(w: &World, d: Date) -> Vec<usize> {
    let year = w.year;
    let early = w.dates.nfl_weeks.get(3).is_none_or(|&wk| d < wk);
    let base: Vec<usize> = super::super::nfl_year::draft_order(w).unwrap_or_else(|| (0..w.nfl.len()).collect());
    let mut v: Vec<usize> = base.into_iter().filter(|&t| super::super::franchise::active_in(w, t, year)).collect();
    if !early && let Some(s) = w.nfl_season.as_ref() {
        v.sort_by(|&a, &b| {
            let pa = s.wins[a] as f64 + 0.5 * s.ties[a] as f64;
            let pb = s.wins[b] as f64 + 0.5 * s.ties[b] as f64;
            pa.total_cmp(&pb)
        });
    }
    v
}

// --- Money books --------------------------------------------------------------------------------------------------

/// Incentives and per-game bonuses settle after the regular season (A7,
/// A8): unearned likely incentives and missed per-game bonuses come back as
/// next year's cap credit; earned unlikely incentives are charged to it.
pub(crate) fn settle_incentives(w: &mut World) {
    let year = w.year;
    let games = if year >= 2021 { 17.0 } else { 16.0 };
    for i in 0..w.people.len() {
        let Some(c) = w.people[i].pro.as_ref() else { continue };
        let Some(t) = c.team else { continue };
        let Some(k) = c.contract.as_ref() else { continue };
        let Some(y) = k.year(year).copied() else { continue };
        let s = c.seasons.iter().find(|s| s.year == year);
        let played = s.map_or(0, |s| s.games) as f64;
        let starts = s.map_or(0, |s| s.starts);
        let bowl = s.is_some_and(|s| s.pro_bowl);
        let mut charge = 0.0;
        if y.per_game > 0.0 {
            charge -= y.per_game * (1.0 - played / games).max(0.0);
        }
        if y.nltbe > 0.0 && (starts >= 12 || bowl) {
            charge += y.nltbe;
        }
        if charge != 0.0 {
            let player = w.people[i].player.id;
            w.clubs[t].dead.push(DeadCharge { year: year + 1, amount: charge, player, why: DeadWhy::Incentives });
        }
    }
}

/// The cash spending floor (2011 CBA, from 2013): each team pays out 90%
/// of the cap in cash over each four-year period (2013-16, 2017-20 and on);
/// a team short pays the difference to its players.
pub(crate) fn cash_floor(w: &mut World, d: Date) {
    let year = w.year;
    if year < 2016 || (year - 2016) % 4 != 0 {
        return;
    }
    let from = year - 3;
    let cap: f64 = (from..=year).map(money::price_cap).sum();
    for t in 0..w.clubs.len() {
        if !super::super::franchise::active_in(w, t, year) {
            continue;
        }
        let spent: f64 = w.clubs[t].cash.iter().filter(|x| x.0 >= from && x.0 <= year).map(|x| x.1).sum();
        let short = 0.9 * cap - spent;
        if short <= 0.0 {
            continue;
        }
        w.market.year_mut(year).floor_paid += short;
        let name = w.nfl[t].name;
        post(
            w,
            d,
            Some(t),
            None,
            Loudness::Small,
            Kind::News,
            format!(
                "The {name} fell short of the cash spending floor for {from}-{year} and pay ${:.1}M to their players.",
                short / 1e6
            ),
        );
    }
}

// --- Books for trials ------------------------------------------------------------------------------------------------

/// The market's year in numbers (trial tooling and the Debug Lab).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MarketYear {
    pub year: i32,
    pub ufa: u32,
    pub resigned: u32,
    pub signed_new: u32,
    pub perfect: u32,
    pub swayed: u32,
    pub tags: u32,
    pub transition: u32,
    pub tenders: u32,
    pub sheets: u32,
    pub sheets_matched: u32,
    pub options_taken: u32,
    pub options_declined: u32,
    pub extensions: u32,
    pub rookie_extensions: u32,
    pub restructures: u32,
    pub cap_cuts: u32,
    pub june1: u32,
    pub futures: u32,
    pub waiver_claims: u32,
    /// Practice squad players signed to another team's 53, and injury
    /// settlements at cutdown (section 5 leftovers).
    pub poached: u32,
    /// Practice squad players their own team signed up to its 53 in season.
    pub signed_up: u32,
    pub settlements: u32,
    /// Off-field cases opened and league suspensions handed out (section 6).
    pub trouble: u32,
    pub suspensions: u32,
    /// Section 7: trades between clubs on draft day, undrafted signings in
    /// the rush and the looks the clubs' scouts filed on the class.
    pub draft_trades: u32,
    pub udfa_signed: u32,
    pub scout_looks: u32,
    pub holdouts: u32,
    pub holdins: u32,
    pub bluffs: u32,
    pub bluffs_called: u32,
    pub agents_fired: u32,
    pub promises_broken: u32,
    pub trades: u32,
    pub deadline_trades: u32,
    /// Trades from the season's first week through the deadline.
    pub season_trades: u32,
    /// Where trade calls end (trial 9 finding 10): buyers who called, a
    /// player at the hole the buyer grades above its starter, one a seller
    /// would part with, one the buyer values at the seller's ask (and the
    /// cap allows), talks that failed, no package of picks.
    pub trade_funnel: [u32; 6],
    pub picks_traded: u32,
    pub comp_picks: u32,
    /// Free agents who left their team and signed elsewhere in the
    /// window this year at a qualifying wage (trial 8 finding 8).
    pub cfa_noted: u32,
    /// Of last year's, those still with the new team at the season's end
    /// and valued in a round (counted at this year's award).
    pub cfa_valued: u32,
    /// This year's comp picks from the formula and the supplemental ones.
    pub comp_formula: u32,
    pub comp_supplemental: u32,
    pub other_league: u32,
    pub ipp: u32,
    pub floor_paid: f64,
    /// Payroll over the cap (with carryover) on opening day: lowest,
    /// median and highest team.
    pub cap_use: [f64; 3],
    /// Dead money and carryover on opening day as shares of the cap:
    /// lowest, median and highest team.
    pub dead: [f64; 3],
    pub carryover: [f64; 3],
}

/// The league's market state.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Market {
    pub agents: Vec<Agent>,
    pub pending: Vec<Pending>,
    pub sheets: Vec<Sheet>,
    pub picks: super::trades::PickBook,
    /// Last season's totals read by position (player id → rating scale).
    pub totals: std::collections::HashMap<u32, f64>,
    pub years: Vec<MarketYear>,
    /// Trade value one GM took from another (row from column), consensus
    /// chart points (topic 6 question 3: a GM who fleeced another gets
    /// fewer calls back).
    pub fleece: Vec<Vec<f64>>,
    /// This year's veterans' consensus reads by position, best first
    /// (`set_ladders`): the comparables every price is read against.
    pub ladder: Vec<Vec<f64>>,
    /// Everyone's reads by position, young players included, best first:
    /// how deep a player below every veteran sits (trial 9 finding 14).
    pub depth: Vec<Vec<f64>>,
    /// Active teams this year (the ladder's starting places).
    pub teams: usize,
    /// Someone went on waivers since the last claims pass, so `claims`
    /// skips its daily scan of everyone when nobody did.
    pub waivers: bool,
    /// Each year's franchise and transition tag shares of the cap by
    /// position (the 2011 formula's five-year history).
    pub tag_shares: Vec<(i32, Vec<[f64; 2]>)>,
    /// Off-field cases waiting on the league's ruling (section 6 topic 7).
    pub cases: Vec<super::super::people::Case>,
}

impl Market {
    pub fn year_mut(&mut self, year: i32) -> &mut MarketYear {
        if self.years.last().is_none_or(|y| y.year != year) {
            self.years.push(MarketYear { year, ..Default::default() });
        }
        self.years.last_mut().unwrap()
    }
}

/// Posts an NFL market line about the person `who` (an index in the people
/// list), if any: his name and every club the line names are linked (L2).
pub(crate) fn post(
    w: &mut World,
    d: Date,
    team: Option<usize>,
    who: Option<usize>,
    loud: Loudness,
    kind: Kind,
    text: String,
) {
    let mut e = Event::new(d, CalLevel::Nfl, kind, loud, text);
    if let Some(t) = team {
        e = e.team(t);
    }
    if let Some(k) = who {
        let p = &w.people[k];
        e = e.person(p.player.id, &p.name);
    }
    for (t, club) in w.nfl.iter().enumerate() {
        e = e.club_link(t, club.name);
    }
    w.calendar.post(e);
}
