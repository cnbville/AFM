//! A/B tests: does a body or a rating change the stats, the way it does in
//! real football? (`afm/ab-tests/plan.md`, agreed by Cain 2026-11-06.)
//!
//! Each test changes one thing for every player at a position (or for one
//! team's players, the "my team" view) and plays the same games, on the
//! same seeds, in every arm. Each arm is low, typical or high. A body
//! number moves about 1.5 spreads around the position's real combine mean;
//! a rating moves 15 points; a tendency goes to 4 and 16; a Dynamic Link
//! is off or full. "As recruited" arms swap in players drawn by the game's
//! own generator at that position, split by the body and matched on
//! overall. Bands come from the games themselves: a block jackknife over
//! 50 blocks of games, paired across arms (the same games in each).
//!
//! Same engine and code paths as the game; the game itself changes nothing.

use std::collections::{HashMap, HashSet};

use afm_engine::Rng;
use afm_engine::attributes::{Attr, Attributes, MAX_RATING, MIN_RATING};
use afm_engine::body::{Body, Drills, body_now, forty_weight};
use afm_engine::game::scrimmage::DSlot;
use afm_engine::game::sim::{FoulKind, Move};
use afm_engine::game::stats::{PlayRecord, PlayType};
use afm_engine::game::{Conditions, GamePlayer, GameResult, GameTeam, Venue, quick_game_in};
use afm_engine::links::Link;
use afm_engine::player::{Level, Player, ProspectOptions, generate_prospect};
use afm_engine::positions::{Position, overall_for};
use afm_engine::spots::{Spot, spot_rating};
use afm_engine::tendencies::Tend;
use rayon::prelude::*;

fn clamp(x: f64, lo: f64, hi: f64) -> f64 {
    x.max(lo).min(hi)
}

// ------------------------------------------------------------------ rosters

/// A team and its players as people (`people[i]` is `team.players[i]`).
#[derive(Clone)]
pub struct Squad {
    pub team: GameTeam,
    pub people: Vec<Player>,
}

/// The synthetic league every check uses: 32 teams at the level's spread.
pub fn synthetic(level: Level, seed: u32) -> Vec<Squad> {
    afm_engine::game::synthetic::synthetic_league_people(level, 32, seed)
        .into_iter()
        .map(|(team, people)| Squad { team, people })
        .collect()
}

/// A world's 32 NFL clubs as they would dress today (a world file).
pub fn world_nfl(path: &str) -> Vec<Squad> {
    let w = afm_engine::save::read_world(std::path::Path::new(path)).expect("read the world file");
    (0..w.nfl.len()).filter_map(|t| w.debug_nfl_team(t)).map(|(team, people)| Squad { team, people }).collect()
}

// ------------------------------------------------------------------ changes

/// A body number or a drill a test moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Part {
    Height,
    Weight,
    Arm,
    /// The forty, by his Speed (his acceleration and weight kept).
    Forty,
    /// The ten-yard split, by his Acceleration alone (his first step).
    Split,
    /// The three-cone, by his Agility.
    Cone,
    /// The short shuttle, by his Agility.
    Shuttle,
}

impl Part {
    pub fn unit(self) -> (&'static str, f64) {
        match self {
            Part::Height | Part::Arm => ("inch", 1.0),
            Part::Weight => ("10 lb", 10.0),
            _ => ("0.1 s", 0.1),
        }
    }

    pub fn show(self, v: f64) -> String {
        match self {
            Part::Height => afm_engine::body::height_text(v).replace('"', ""),
            Part::Arm => format!("{v:.2} in"),
            Part::Weight => format!("{v:.0} lb"),
            _ => format!("{v:.2} s"),
        }
    }

    /// His number today.
    pub fn of(self, gp: &GamePlayer, p: &Player) -> f64 {
        let mut q = p.clone();
        q.attributes = gp.attrs;
        match self {
            Part::Height => gp.height,
            Part::Weight => gp.weight,
            Part::Arm => gp.arm,
            Part::Forty => gp.forty(),
            Part::Split => afm_engine::links::ten_split(&gp.attrs, gp.weight),
            Part::Cone => Drills::of(&q).expected().cone,
            Part::Shuttle => Drills::of(&q).expected().shuttle,
        }
    }
}

/// One change to a player.
#[derive(Clone, Debug)]
pub enum Knob {
    /// Set a body number or drill.
    Set(Part, f64),
    /// Add points to his own rating.
    Rating(Attr, f64),
    /// Set a tendency.
    Tend(Tend, f64),
    /// Set a Dynamic Link's strength (0 none, 1 full) after his body is read.
    Link(Link, f64),
    /// Set his hidden consistency (1-20).
    Consistency(f64),
    /// Swap him for a player the generator draws at his position, with this
    /// body number inside the range, at his overall.
    Recruit(Part, f64, f64),
}

/// Who a change reaches: positions, and optionally only one spot (the
/// fullback, the center).
#[derive(Clone, Debug)]
pub struct Who {
    pub pos: Vec<Position>,
    pub spot: Option<Spot>,
}

impl Who {
    pub fn at(pos: &[Position]) -> Who {
        Who { pos: pos.to_vec(), spot: None }
    }

    pub fn has(&self, gp: &GamePlayer) -> bool {
        self.pos.contains(&gp.position) && self.spot.is_none_or(|s| gp.spot == s)
    }
}

/// One arm: changes for groups of players, on every team or only on
/// team 0 (the "my team" view, which plays every game).
#[derive(Clone, Debug)]
pub struct Arm {
    pub label: String,
    pub groups: Vec<(Who, Vec<Knob>)>,
    pub mine: bool,
}

/// Rating that holds a forty for his acceleration and weight.
fn speed_for(forty: f64, at: &Attributes, weight: f64) -> f64 {
    ((5.56 + forty_weight(weight) - forty) / 0.0124 - 0.25 * at.get(Attr::Acceleration)) / 0.75
}

/// A rating (between 1 and 99) that brings `f` to the target, by halving.
fn solve(at: &mut Attributes, a: Attr, target: f64, f: &dyn Fn(&Attributes) -> f64) {
    let (mut lo, mut hi) = (MIN_RATING, MAX_RATING);
    let rising = {
        let mut x = *at;
        x.set(a, hi);
        let up = f(&x);
        x.set(a, lo);
        up > f(&x)
    };
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        at.set(a, mid);
        let v = f(at);
        if (v < target) == rising {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    at.set(a, 0.5 * (lo + hi));
}

/// A drawn player at a position with a body number in range, at an overall:
/// the generator's athlete-first path (`athlete.rs`), the way the world
/// makes its players, keeping only men whose natural position is his, so
/// each comes with whatever his body brings.
fn recruit(gp: &GamePlayer, part: Part, lo: f64, hi: f64, level: Level) -> Option<Player> {
    let target = overall_for(gp.position, &gp.attrs);
    let share = match level {
        Level::HS => 0.86,
        Level::NFL => 1.0,
        _ => 0.95,
    };
    let mut rng = Rng::new(gp.id ^ 0xAB0C_0000 ^ ((lo.max(-1e6) * 64.0) as i64 as u32).wrapping_mul(2_654_435_761));
    let mut aim = target;
    let mut best: Option<(f64, Player)> = None;
    for k in 0..40_000u32 {
        // His own id draws his body, so each try is a new man; the game
        // knows him by the old one's id.
        let cand = (gp.id.wrapping_mul(7919) ^ k.wrapping_mul(2_654_435_761) ^ 0x5EED_0000) & 0x7FFF_FFFF;
        let mut p = generate_prospect(
            &mut rng,
            ProspectOptions {
                id: Some(cand),
                position: None,
                age: Some(14),
                talent_z: Some(afm_engine::game::synthetic::z_for(aim)),
            },
        );
        if p.position != gp.position {
            continue;
        }
        p.age = gp.age;
        for a in Attr::ALL {
            p.attributes.set(a, clamp(afm_engine::debug::ceilings(&p).get(a) * share, MIN_RATING, MAX_RATING));
        }
        let ov = overall_for(gp.position, &p.attributes);
        aim = clamp(aim + 0.5 * (target - ov), 20.0, 99.0);
        let b = body_now(&p);
        let v = match part {
            Part::Height => b.height,
            Part::Weight => b.weight,
            Part::Arm => b.arm,
            Part::Forty => afm_engine::links::forty(&p.attributes, b.weight),
            Part::Split => afm_engine::links::ten_split(&p.attributes, b.weight),
            Part::Cone => Drills::of(&p).expected().cone,
            Part::Shuttle => Drills::of(&p).expected().shuttle,
        };
        if !(lo..=hi).contains(&v) {
            continue;
        }
        let miss = (ov - target).abs();
        if best.as_ref().is_none_or(|b| miss < b.0) {
            best = Some((miss, p));
        }
        if miss <= 1.0 {
            break;
        }
    }
    let (_, mut p) = best?;
    p.tend = afm_engine::tendencies::Tendencies::at_level(
        p.id,
        level,
        &afm_engine::debug::ceilings(&p).clone(),
        p.body.weight,
        p.body.height,
        &p.personality,
    );
    afm_engine::spots::set_spot(&mut p, gp.spot);
    p.id = gp.id;
    Some(p)
}

fn change(gp: &mut GamePlayer, person: &Player, knobs: &[Knob], level: Level) {
    let mut b: Body = body_now(person);
    b.height = gp.height;
    b.weight = gp.weight;
    b.arm = gp.arm;
    let mut at = gp.attrs;
    let mut again = false;
    let mut q = person.clone();
    for k in knobs {
        match *k {
            Knob::Set(part, v) => {
                again = true;
                match part {
                    Part::Height => b.height = v,
                    Part::Weight => b.weight = v,
                    Part::Arm => {
                        b.wingspan += 2.0 * (v - b.arm);
                        b.arm = v;
                    }
                    Part::Forty => at.set(Attr::Speed, clamp(speed_for(v, &at, b.weight), MIN_RATING, MAX_RATING)),
                    Part::Split => {
                        let w = b.weight;
                        solve(&mut at, Attr::Acceleration, v, &|x| afm_engine::links::ten_split(x, w));
                    }
                    Part::Cone | Part::Shuttle => {
                        let f = |x: &Attributes| {
                            let mut p = q.clone();
                            p.attributes = *x;
                            let e = Drills::of(&p).expected();
                            if part == Part::Cone { e.cone } else { e.shuttle }
                        };
                        solve(&mut at, Attr::Agility, v, &f);
                    }
                }
            }
            Knob::Rating(a, d) => {
                again = true;
                at.set(a, clamp(at.get(a) + d, MIN_RATING, MAX_RATING));
            }
            Knob::Tend(t, v) => gp.tend.set(t, v),
            Knob::Consistency(v) => gp.traits.consistency = v,
            Knob::Link(..) => {}
            Knob::Recruit(part, lo, hi) => {
                if let Some(p) = recruit(gp, part, lo, hi, level) {
                    let fresh = GamePlayer::from_player(&p, gp.knowledge, gp.experience);
                    gp.attrs = fresh.attrs;
                    gp.height = fresh.height;
                    gp.weight = fresh.weight;
                    gp.arm = fresh.arm;
                    gp.left_handed = fresh.left_handed;
                    gp.left_footed = fresh.left_footed;
                    gp.traits = fresh.traits;
                    gp.personality = fresh.personality;
                    gp.injury_proneness = fresh.injury_proneness;
                    gp.spot_r = fresh.spot_r;
                    gp.tend = fresh.tend;
                    gp.links = fresh.links;
                    b = body_now(&p);
                    at = gp.attrs;
                    q = p;
                }
            }
        }
        q.attributes = at;
    }
    if again {
        gp.attrs = at;
        gp.height = b.height;
        gp.weight = b.weight;
        gp.arm = b.arm;
        gp.spot_r = Spot::ALL.map(|s| spot_rating(s, &at, b.weight));
        gp.links = afm_engine::links::measure(gp.position, &b, &at, person.game.recovery);
    }
    for k in knobs {
        if let Knob::Link(l, v) = *k {
            gp.links.set(l, v);
        }
    }
}

/// The teams for an arm.
pub fn teams_for(base: &[Squad], arm: &Arm, level: Level) -> Vec<GameTeam> {
    base.par_iter()
        .enumerate()
        .map(|(ti, sq)| {
            let mut t = sq.team.clone();
            if arm.mine && ti != 0 {
                return t;
            }
            for (k, gp) in t.players.iter_mut().enumerate() {
                for (who, knobs) in &arm.groups {
                    if who.has(gp) {
                        change(gp, &sq.people[k], knobs, level);
                    }
                }
            }
            t
        })
        .collect()
}

/// The ids a test watches: its changed group on every team (or team 0).
pub fn focus_of(base: &[Squad], who: &Who, mine: bool) -> HashSet<u32> {
    base.iter()
        .enumerate()
        .filter(|(ti, _)| !mine || *ti == 0)
        .flat_map(|(_, sq)| sq.team.players.iter().filter(|gp| who.has(gp)).map(|gp| gp.id))
        .collect()
}

// ------------------------------------------------------------------ games

/// The league view's pairings (as `tendency_check`): 32 teams.
fn pair(g: usize, n: usize) -> (usize, usize) {
    let k = g / 2;
    let h = (k * 7) % n;
    let a = (k * 13 + 5) % n;
    let a = if a == h { (a + 1) % n } else { a };
    if g.is_multiple_of(2) { (h, a) } else { (a, h) }
}

/// The "my team" view: team 0 plays every game, home and away in turn.
fn pair_mine(g: usize, n: usize) -> (usize, usize) {
    let opp = 1 + (g / 2) % (n - 1);
    if g.is_multiple_of(2) { (0, opp) } else { (opp, 0) }
}

/// One game on its own seed, the same in every arm.
fn one_game(teams: &[GameTeam], level: Level, g: usize, seed: u32, mine: bool) -> (GameResult, usize, usize) {
    let (h, a) = if mine { pair_mine(g, teams.len()) } else { pair(g, teams.len()) };
    let venue =
        Venue { cond: Conditions { crowd: 0.65, ..Conditions::default() }, wear: [0.0, 0.012], ..Venue::default() };
    let r = quick_game_in(&teams[h], &teams[a], level, seed as u64 * 1_000_003 + g as u64, Default::default(), venue);
    (r, h, a)
}

// ------------------------------------------------------------------ stats

/// What a stat counts in one play or game.
pub struct Cx<'a> {
    pub focus: &'a HashSet<u32>,
    pub pos: &'a HashMap<u32, Position>,
    /// My team's side in this game (0 home, 1 away), in the my team view.
    pub mine: Option<usize>,
    /// Each player's weight and forty in this arm (the linebacker job rows).
    pub body: &'a HashMap<u32, [f64; 2]>,
}

impl Cx<'_> {
    #[inline]
    fn f(&self, id: u32) -> bool {
        self.focus.contains(&id)
    }
    #[inline]
    fn fo(&self, id: Option<u32>) -> bool {
        id.is_some_and(|i| self.focus.contains(&i))
    }
    #[inline]
    fn is(&self, id: Option<u32>, p: Position) -> bool {
        id.and_then(|i| self.pos.get(&i)) == Some(&p)
    }
    /// Focus players on the field (both sides).
    fn on(&self, r: &PlayRecord) -> f64 {
        r.on_field.iter().flatten().filter(|i| self.focus.contains(i)).count() as f64
    }
    /// Teams whose numbers count in a game (two in the league view).
    fn teams(&self) -> f64 {
        if self.mine.is_some() { 1.0 } else { 2.0 }
    }
    /// The offense counts (its side is mine, or the league view).
    fn my_off(&self, r: &PlayRecord) -> bool {
        self.mine.is_none_or(|m| r.offense == m)
    }
    fn my_def(&self, r: &PlayRecord) -> bool {
        self.mine.is_none_or(|m| r.offense != m)
    }
}

pub type C = [f64; 5];
type PlayFn = fn(&PlayRecord, &Cx) -> C;
type GameFn = fn(&GameResult, &Cx) -> C;

#[derive(Clone, Copy, PartialEq)]
pub enum Unit {
    /// A share, shown as a percent.
    Pct,
    /// A mean in its own units.
    Num,
}

pub struct Stat {
    pub key: &'static str,
    pub label: &'static str,
    pub unit: Unit,
    pub play: Option<PlayFn>,
    pub game: Option<GameFn>,
    pub value: fn(&C) -> f64,
}

fn ratio(c: &C) -> f64 {
    if c[1] > 0.0 { c[0] / c[1] } else { f64::NAN }
}

fn sd(c: &C) -> f64 {
    if c[0] < 2.0 {
        return f64::NAN;
    }
    let m = c[1] / c[0];
    (c[2] / c[0] - m * m).max(0.0).sqrt()
}

/// A spread with the sample noise taken out: slot 3 holds each game's noise
/// variance (minor bug 287: real game logs are measured the same way).
fn true_sd(c: &C) -> f64 {
    if c[0] < 2.0 {
        return f64::NAN;
    }
    let m = c[1] / c[0];
    (c[2] / c[0] - m * m - c[3] / c[0]).max(0.0).sqrt()
}

/// The NFL passer rating from attempts, completions, yards, touchdowns and
/// interceptions.
fn rating(c: &C) -> f64 {
    let att = c[0];
    if att <= 0.0 {
        return f64::NAN;
    }
    let g = |x: f64| clamp(x, 0.0, 2.375);
    let a = g((c[1] / att - 0.3) * 5.0);
    let b = g((c[2] / att - 3.0) * 0.25);
    let t = g(c[3] / att * 20.0);
    let i = g(2.375 - c[4] / att * 25.0);
    (a + b + t + i) / 6.0 * 100.0
}

#[inline]
fn bf(x: bool) -> f64 {
    x as u8 as f64
}
#[inline]
fn rc(n: f64, d: f64) -> C {
    [n, d, 0.0, 0.0, 0.0]
}
const Z: C = [0.0; 5];

fn live(r: &PlayRecord) -> bool {
    !r.nullified
}
fn dropback(r: &PlayRecord) -> bool {
    live(r) && matches!(r.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble)
}
fn attempt(r: &PlayRecord) -> bool {
    live(r) && r.kind == PlayType::Pass
}
fn thrown(r: &PlayRecord) -> bool {
    attempt(r) && !r.throw_away && r.target.is_some()
}
fn run(r: &PlayRecord) -> bool {
    live(r) && r.kind == PlayType::Run
}
fn scrimmage(r: &PlayRecord) -> bool {
    live(r) && matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble)
}
fn qb_of(r: &PlayRecord) -> Option<u32> {
    r.passer.or(r.carrier)
}
fn middle(r: &PlayRecord) -> bool {
    r.throw_y.is_some_and(|(s, a)| (a - s).abs() < 6.0)
}
/// The 4th quarter or overtime with the score within 8.
fn late_close(r: &PlayRecord) -> bool {
    r.quarter >= 4 && (r.score[r.offense] - r.score[1 - r.offense]).abs() <= 8
}
/// Focus pass rushers' (or his man's) pressure on a play: in pressures,
/// sackers or hits.
fn pressured_by(r: &PlayRecord, id: u32) -> bool {
    r.pressures.contains(&id) || r.sackers.contains(&id) || r.hits.contains(&id)
}
fn win_move(r: &PlayRecord, id: u32) -> Option<Move> {
    r.rush_wins.iter().find(|w| w.0 == id).map(|w| w.1)
}
fn fouls(r: &PlayRecord, c: &Cx, k: &[FoulKind]) -> f64 {
    r.penalties.iter().filter(|p| p.4 && c.f(p.2) && k.contains(&p.0)).count() as f64
}
/// Every play a focus player was on the field for, penalties included.
fn snaps(r: &PlayRecord, c: &Cx) -> f64 {
    if matches!(r.kind, PlayType::Kneel | PlayType::Spike) { 0.0 } else { c.on(r) }
}

macro_rules! play {
    ($k:expr, $l:expr, $u:expr, $f:expr) => {
        Stat { key: $k, label: $l, unit: $u, play: Some($f), game: None, value: ratio }
    };
}
macro_rules! game {
    ($k:expr, $l:expr, $u:expr, $f:expr) => {
        Stat { key: $k, label: $l, unit: $u, play: None, game: Some($f), value: ratio }
    };
}

/// Every stat the tests read.
pub fn stats() -> Vec<Stat> {
    use PlayType as T;
    use Position as P;
    use Unit::*;
    let mut v = vec![
        // --- the quarterback (focus: the quarterbacks)
        play!("qb_scr", "scrambles a dropback", Pct, |r, c| if dropback(r) && c.fo(qb_of(r)) {
            rc(bf(r.kind == T::Scramble), 1.0)
        } else {
            Z
        }),
        play!(
            "qb_scr_yds",
            "yards a scramble",
            Num,
            |r, c| if dropback(r) && r.kind == T::Scramble && c.fo(qb_of(r)) { rc(r.yards, 1.0) } else { Z }
        ),
        game!("qb_runs", "designed QB runs a team game", Num, |g, c| {
            let n = g.stats.plays.iter().filter(|r| run(r) && c.fo(r.carrier) && c.is(r.carrier, P::QB)).count();
            rc(n as f64, c.teams())
        }),
        play!("qb_spy", "dropbacks against a spy", Pct, |r, c| if dropback(r) && c.fo(qb_of(r)) {
            rc(bf(r.defense_call.contains("QB spy")), 1.0)
        } else {
            Z
        }),
        play!("qb_sack", "sacks a dropback", Pct, |r, c| if dropback(r) && c.fo(qb_of(r)) {
            rc(bf(r.kind == T::Sack), 1.0)
        } else {
            Z
        }),
        play!("qb_sack_press", "sacks a pressured dropback", Pct, |r, c| if dropback(r)
            && c.fo(qb_of(r))
            && (r.pressured || r.kind == T::Sack)
        {
            rc(bf(r.kind == T::Sack), 1.0)
        } else {
            Z
        }),
        play!("qb_press", "pressured dropbacks", Pct, |r, c| if dropback(r) && c.fo(qb_of(r)) {
            rc(bf(r.pressured || r.kind == T::Sack), 1.0)
        } else {
            Z
        }),
        play!("qb_epa", "EPA a dropback", Num, |r, c| if dropback(r) && c.fo(qb_of(r)) { rc(r.epa, 1.0) } else { Z }),
        play!("qb_cmp", "completion rate", Pct, |r, c| if attempt(r) && c.fo(r.passer) {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!("qb_ypa", "yards an attempt", Num, |r, c| if attempt(r) && c.fo(r.passer) {
            rc(r.yards, 1.0)
        } else {
            Z
        }),
        play!("qb_int", "interceptions an attempt", Pct, |r, c| if attempt(r) && c.fo(r.passer) {
            rc(bf(r.interception), 1.0)
        } else {
            Z
        }),
        play!("qb_deep_int", "interceptions a deep throw (20+ air)", Pct, |r, c| if thrown(r)
            && c.fo(r.passer)
            && r.air_yards >= 20.0
        {
            rc(bf(r.interception), 1.0)
        } else {
            Z
        }),
        play!("qb_air", "intended air yards", Num, |r, c| if thrown(r) && c.fo(r.passer) {
            rc(r.air_yards, 1.0)
        } else {
            Z
        }),
        play!("qb_deep", "throws 20+ air yards", Pct, |r, c| if thrown(r) && c.fo(r.passer) {
            rc(bf(r.air_yards >= 20.0), 1.0)
        } else {
            Z
        }),
        play!("qb_cmp20", "completions of 20+ an attempt", Pct, |r, c| if attempt(r) && c.fo(r.passer) {
            rc(bf(r.complete && r.yards >= 20.0), 1.0)
        } else {
            Z
        }),
        play!("qb_short_cmp", "completion, under 10 air yards", Pct, |r, c| if thrown(r)
            && c.fo(r.passer)
            && r.air_yards < 10.0
        {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!("qb_mid_cmp", "completion, 10-19 air yards", Pct, |r, c| if thrown(r)
            && c.fo(r.passer)
            && (10.0..20.0).contains(&r.air_yards)
        {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!("qb_deep_cmp", "completion, 20+ air yards", Pct, |r, c| if thrown(r)
            && c.fo(r.passer)
            && r.air_yards >= 20.0
        {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!("qb_ttt", "time to throw, s", Num, |r, c| if thrown(r) && c.fo(r.passer) {
            rc(r.time_to_throw, 1.0)
        } else {
            Z
        }),
        play!("qb_away", "throwaways a dropback", Pct, |r, c| if dropback(r) && c.fo(qb_of(r)) {
            rc(bf(r.kind == T::Pass && r.throw_away), 1.0)
        } else {
            Z
        }),
        play!("qb_check", "throws to a back", Pct, |r, c| if thrown(r) && c.fo(r.passer) {
            rc(bf(c.is(r.target, P::RB)), 1.0)
        } else {
            Z
        }),
        play!("qb_batted", "batted at the line an attempt", Pct, |r, c| if attempt(r) && c.fo(r.passer) {
            rc(bf(r.tend.hands_up.is_some()), 1.0)
        } else {
            Z
        }),
        play!("qb_mid_field", "completion, throws over the middle", Pct, |r, c| if thrown(r)
            && c.fo(r.passer)
            && middle(r)
        {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!("qb_press_cmp", "completion under pressure", Pct, |r, c| if thrown(r) && c.fo(r.passer) && r.pressured {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!("qb_clean_cmp", "completion from a clean pocket", Pct, |r, c| if thrown(r)
            && c.fo(r.passer)
            && !r.pressured
        {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!(
            "qb_press_int",
            "interceptions under pressure",
            Pct,
            |r, c| if attempt(r) && c.fo(r.passer) && r.pressured { rc(bf(r.interception), 1.0) } else { Z }
        ),
        play!("qb_late_cmp", "completion, 4th quarter close", Pct, |r, c| if thrown(r)
            && c.fo(r.passer)
            && late_close(r)
        {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!("qb_move_cmp", "completion on the move", Pct, |r, c| if thrown(r) && c.fo(r.passer) && r.scramble {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!(
            "qb_pa_ypa",
            "play-action yards an attempt",
            Num,
            |r, c| if attempt(r) && c.fo(r.passer) && r.play_action { rc(r.yards, 1.0) } else { Z }
        ),
        play!("qb_fum", "fumbles a dropback", Pct, |r, c| if dropback(r) && c.fo(qb_of(r)) {
            rc(bf(r.fumbled == qb_of(r)), 1.0)
        } else {
            Z
        }),
        play!("qb_inj", "QB injuries per 1,000 dropbacks", Num, |r, c| if dropback(r) && c.fo(qb_of(r)) {
            rc(1000.0 * bf(r.injuries.iter().any(|i| Some(i.id) == qb_of(r))), 1.0)
        } else {
            Z
        }),
        Stat {
            key: "qb_ypa_sd",
            label: "game-to-game spread of a QB's yards an attempt",
            unit: Num,
            play: None,
            game: Some(|g, c| {
                let mut m: HashMap<u32, (f64, f64)> = HashMap::new();
                for r in g.stats.plays.iter().filter(|r| attempt(r) && c.fo(r.passer)) {
                    let e = m.entry(r.passer.unwrap()).or_default();
                    e.0 += 1.0;
                    e.1 += r.yards;
                }
                let mut out = Z;
                for (n, y) in m.values().filter(|v| v.0 >= 15.0) {
                    let x = y / n;
                    out[0] += 1.0;
                    out[1] += x;
                    out[2] += x * x;
                }
                out
            }),
            value: sd,
        },
        Stat {
            key: "qb_epa_form",
            label: "game-to-game form of a QB's EPA a dropback (noise taken out)",
            unit: Num,
            play: None,
            game: Some(|g, c| {
                let mut m: HashMap<u32, (f64, f64, f64)> = HashMap::new();
                for r in g.stats.plays.iter().filter(|r| dropback(r) && c.fo(qb_of(r))) {
                    let e = m.entry(qb_of(r).unwrap()).or_default();
                    e.0 += 1.0;
                    e.1 += r.epa;
                    e.2 += r.epa * r.epa;
                }
                let mut out = Z;
                for (n, y, yy) in m.values().filter(|v| v.0 >= 20.0) {
                    let x = y / n;
                    // The sample noise in this game's mean: the plays' spread over n.
                    let noise = (yy / n - x * x) * n / (n - 1.0) / n;
                    out[0] += 1.0;
                    out[1] += x;
                    out[2] += x * x;
                    out[3] += noise;
                }
                out
            }),
            value: true_sd,
        },
        Stat {
            key: "qb_cmp_form",
            label: "game-to-game form of a QB's completion rate (noise taken out)",
            unit: Pct,
            play: None,
            game: Some(|g, c| {
                let mut m: HashMap<u32, (f64, f64)> = HashMap::new();
                for r in g.stats.plays.iter().filter(|r| attempt(r) && c.fo(r.passer)) {
                    let e = m.entry(r.passer.unwrap()).or_default();
                    e.0 += 1.0;
                    e.1 += bf(r.complete);
                }
                let mut out = Z;
                for (n, k) in m.values().filter(|v| v.0 >= 15.0) {
                    let x = k / n;
                    out[0] += 1.0;
                    out[1] += x;
                    out[2] += x * x;
                    out[3] += x * (1.0 - x) / (n - 1.0);
                }
                out
            }),
            value: true_sd,
        },
        // --- the run game (focus: the ball carriers)
        play!("rb_ypc", "yards a carry", Num, |r, c| if run(r) && c.fo(r.carrier) { rc(r.yards, 1.0) } else { Z }),
        play!("rb_ybc", "yards before contact a carry", Num, |r, c| if run(r) && c.fo(r.carrier) {
            rc(r.before_contact, 1.0)
        } else {
            Z
        }),
        play!("rb_yaco", "yards after contact a carry", Num, |r, c| if run(r) && c.fo(r.carrier) {
            rc((r.yards - r.before_contact).max(0.0), 1.0)
        } else {
            Z
        }),
        play!("rb_mtf", "missed tackles forced a carry", Num, |r, c| if run(r) && c.fo(r.carrier) {
            rc(r.missed_tackles.len() as f64, 1.0)
        } else {
            Z
        }),
        play!("rb_20", "runs of 20+", Pct, |r, c| if run(r) && c.fo(r.carrier) {
            rc(bf(r.yards >= 20.0), 1.0)
        } else {
            Z
        }),
        play!("rb_stuff", "stuffed (no gain or loss)", Pct, |r, c| if run(r) && c.fo(r.carrier) {
            rc(bf(r.yards <= 0.0), 1.0)
        } else {
            Z
        }),
        play!("rb_short", "3rd and 4th and 1 converted", Pct, |r, c| if run(r)
            && c.fo(r.carrier)
            && r.down >= 3
            && r.togo <= 1.5
        {
            rc(bf(r.first_down || r.touchdown), 1.0)
        } else {
            Z
        }),
        play!("rb_fum", "fumbles a touch", Pct, |r, c| {
            let who = if run(r) {
                r.carrier
            } else if attempt(r) && r.complete {
                r.target
            } else {
                None
            };
            if c.fo(who) { rc(bf(r.fumbled == who), 1.0) } else { Z }
        }),
        play!("rb_out", "carries outside the tackles", Pct, |r, c| match (run(r) && c.fo(r.carrier), r.tend.cross) {
            (true, Some((dy, _))) => rc(bf(dy.abs() > 4.0), 1.0),
            _ => Z,
        }),
        play!("rb_inj", "carrier injuries per 1,000 carries", Num, |r, c| if run(r) && c.fo(r.carrier) {
            rc(1000.0 * bf(r.injuries.iter().any(|i| Some(i.id) == r.carrier)), 1.0)
        } else {
            Z
        }),
        play!("rb_epa", "EPA a carry", Num, |r, c| if run(r) && c.fo(r.carrier) { rc(r.epa, 1.0) } else { Z }),
        play!("rb_pp_lost", "pass blocks lost (backs)", Pct, |r, c| {
            let mut o = Z;
            for b in r.blocks.iter().filter(|b| b.2 && c.f(b.0)) {
                o[0] += bf(!b.3);
                o[1] += 1.0;
            }
            o
        }),
        play!("blk_press", "his man pressures the QB, a pass block", Pct, |r, c| {
            let mut o = Z;
            for b in r.blocks.iter().filter(|b| b.2 && c.f(b.0)) {
                o[0] += bf(pressured_by(r, b.1));
                o[1] += 1.0;
            }
            o
        }),
        play!("fb_ypc", "yards a carry with him leading", Num, |r, c| if run(r)
            && c.my_off(r)
            && !c.fo(r.carrier)
            && r.on_field[0].iter().chain(&r.on_field[1]).any(|i| c.f(*i))
        {
            rc(r.yards, 1.0)
        } else {
            Z
        }),
        Stat {
            key: "rb_ypc_sd",
            label: "game-to-game spread of a back's yards a carry",
            unit: Num,
            play: None,
            game: Some(|g, c| {
                let mut m: HashMap<u32, (f64, f64)> = HashMap::new();
                for r in g.stats.plays.iter().filter(|r| run(r) && c.fo(r.carrier) && c.is(r.carrier, P::RB)) {
                    let e = m.entry(r.carrier.unwrap()).or_default();
                    e.0 += 1.0;
                    e.1 += r.yards;
                }
                let mut out = Z;
                for (n, y) in m.values().filter(|v| v.0 >= 10.0) {
                    let x = y / n;
                    out[0] += 1.0;
                    out[1] += x;
                    out[2] += x * x;
                }
                out
            }),
            value: sd,
        },
        // --- receivers (focus: the targets)
        play!("rc_share", "share of targets", Pct, |r, c| if thrown(r) && c.my_off(r) {
            rc(bf(c.fo(r.target)), 1.0)
        } else {
            Z
        }),
        play!("rc_catch", "catch rate a target", Pct, |r, c| if thrown(r) && c.fo(r.target) {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!("rc_ypt", "yards a target", Num, |r, c| if thrown(r) && c.fo(r.target) { rc(r.yards, 1.0) } else { Z }),
        play!("rc_sep", "separation at the catch point, yd", Num, |r, c| if thrown(r) && c.fo(r.target) {
            rc(r.separation, 1.0)
        } else {
            Z
        }),
        play!("rc_tight", "targets into a tight window (under 1 yd)", Pct, |r, c| if thrown(r) && c.fo(r.target) {
            rc(bf(r.separation < 1.0), 1.0)
        } else {
            Z
        }),
        play!("rc_cont", "contested catch rate (under 1 yd)", Pct, |r, c| if thrown(r)
            && c.fo(r.target)
            && r.separation < 1.0
        {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!(
            "rc_deep",
            "catch rate, 20+ air yards",
            Pct,
            |r, c| if thrown(r) && c.fo(r.target) && r.air_yards >= 20.0 { rc(bf(r.complete), 1.0) } else { Z }
        ),
        play!("rc_deep_share", "targets 20+ air yards", Pct, |r, c| if thrown(r) && c.fo(r.target) {
            rc(bf(r.air_yards >= 20.0), 1.0)
        } else {
            Z
        }),
        play!("rc_adot", "depth of target, air yards", Num, |r, c| if thrown(r) && c.fo(r.target) {
            rc(r.air_yards, 1.0)
        } else {
            Z
        }),
        play!("rc_rz", "TDs a red zone target", Pct, |r, c| if thrown(r) && c.fo(r.target) && r.yardline_100 <= 20.0 {
            rc(bf(r.touchdown), 1.0)
        } else {
            Z
        }),
        play!("rc_rz_share", "share of red zone targets", Pct, |r, c| if thrown(r)
            && c.my_off(r)
            && r.yardline_100 <= 20.0
        {
            rc(bf(c.fo(r.target)), 1.0)
        } else {
            Z
        }),
        play!("rc_yac", "yards after the catch", Num, |r, c| if thrown(r) && r.complete && c.fo(r.target) {
            rc(r.yac, 1.0)
        } else {
            Z
        }),
        play!("rc_mtf", "missed tackles forced a catch", Num, |r, c| if thrown(r) && r.complete && c.fo(r.target) {
            rc(r.missed_tackles.len() as f64, 1.0)
        } else {
            Z
        }),
        play!("rc_drop", "drops a target", Pct, |r, c| if thrown(r) && c.fo(r.target) {
            rc(bf(r.drop), 1.0)
        } else {
            Z
        }),
        play!("rc_int", "picks a target", Pct, |r, c| if thrown(r) && c.fo(r.target) {
            rc(bf(r.interception), 1.0)
        } else {
            Z
        }),
        play!("rc_pbu", "broken up a target", Pct, |r, c| if thrown(r) && c.fo(r.target) {
            rc(bf(r.pbu.is_some()), 1.0)
        } else {
            Z
        }),
        play!("rc_middle", "catch rate over the middle, a man within 2 yd", Pct, |r, c| if thrown(r)
            && c.fo(r.target)
            && middle(r)
            && r.separation < 2.0
        {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!("blk_run_won", "run blocks won", Pct, |r, c| {
            let mut o = Z;
            for b in r.blocks.iter().filter(|b| !b.2 && c.f(b.0)) {
                o[0] += bf(b.3);
                o[1] += 1.0;
            }
            o
        }),
        play!("blk_pass_lost", "pass blocks lost", Pct, |r, c| {
            let mut o = Z;
            for b in r.blocks.iter().filter(|b| b.2 && c.f(b.0)) {
                o[0] += bf(!b.3);
                o[1] += 1.0;
            }
            o
        }),
        // --- blockers (focus: the linemen)
        play!("blk_sack", "his man sacks the QB, a pass block", Pct, |r, c| {
            let mut o = Z;
            for b in r.blocks.iter().filter(|b| b.2 && c.f(b.0)) {
                o[0] += bf(r.sackers.contains(&b.1));
                o[1] += 1.0;
            }
            o
        }),
        play!("blk_held", "pass block held, s", Num, |r, c| {
            let mut o = Z;
            for b in r.blocks.iter().filter(|b| b.2 && c.f(b.0)) {
                o[0] += b.4;
                o[1] += 1.0;
            }
            o
        }),
        play!("blk_lost_power", "lost to a power rush, a pass block", Pct, |r, c| {
            let mut o = Z;
            for b in r.blocks.iter().filter(|b| b.2 && c.f(b.0)) {
                o[0] += bf(!b.3 && win_move(r, b.1) == Some(Move::Power));
                o[1] += 1.0;
            }
            o
        }),
        play!("blk_lost_speed", "lost to a speed rush, a pass block", Pct, |r, c| {
            let mut o = Z;
            for b in r.blocks.iter().filter(|b| b.2 && c.f(b.0)) {
                o[0] += bf(!b.3 && win_move(r, b.1) == Some(Move::Speed));
                o[1] += 1.0;
            }
            o
        }),
        play!("ol_hold", "holding a snap (accepted)", Pct, |r, c| rc(
            fouls(r, c, &[FoulKind::OffensiveHolding]),
            snaps(r, c)
        )),
        play!("ol_fs", "false starts a snap", Pct, |r, c| rc(fouls(r, c, &[FoulKind::FalseStart]), snaps(r, c))),
        play!("ol_fs_road", "false starts a snap on the road", Pct, |r, c| if r.offense == 1 {
            rc(fouls(r, c, &[FoulKind::FalseStart]), snaps(r, c))
        } else {
            Z
        }),
        play!("ol_fs_home", "false starts a snap at home", Pct, |r, c| if r.offense == 0 {
            rc(fouls(r, c, &[FoulKind::FalseStart]), snaps(r, c))
        } else {
            Z
        }),
        play!("off_sack", "team sacks a dropback", Pct, |r, c| if dropback(r) && c.my_off(r) {
            rc(bf(r.kind == T::Sack), 1.0)
        } else {
            Z
        }),
        play!("off_press", "team pressured dropbacks", Pct, |r, c| if dropback(r) && c.my_off(r) {
            rc(bf(r.pressured || r.kind == T::Sack), 1.0)
        } else {
            Z
        }),
        play!("off_ypc", "team yards a carry", Num, |r, c| if run(r) && c.my_off(r) { rc(r.yards, 1.0) } else { Z }),
        play!("off_in_ypc", "team yards a carry inside the tackles", Num, |r, c| match (
            run(r) && c.my_off(r),
            r.tend.cross
        ) {
            (true, Some((dy, _))) if dy.abs() <= 4.0 => rc(r.yards, 1.0),
            _ => Z,
        }),
        play!("off_bust", "busts a snap", Pct, |r, c| if scrimmage(r) {
            rc(r.busts.iter().filter(|b| c.f(**b)).count() as f64, c.on(r))
        } else {
            Z
        }),
        play!("bad_snap", "bad snaps a snap", Pct, |r, c| if (scrimmage(r)
            || r.kind == T::Punt
            || r.kind == T::FieldGoal)
            && c.on(r) > 0.0
        {
            rc(bf(r.bad_snap), 1.0)
        } else {
            Z
        }),
        // --- pass rushers and run stoppers (focus: the defenders)
        play!("dl_contact", "first contact with a blocker, s", Num, |r, c| {
            let mut o = Z;
            for &(id, t) in r.contact.iter().filter(|x| c.f(x.0)) {
                let _ = id;
                o[0] += t as f64;
                o[1] += 1.0;
            }
            o
        }),
        play!("dl_shed", "pass blocks beaten (shed)", Pct, |r, c| {
            let mut o = Z;
            for b in r.blocks.iter().filter(|b| b.2 && c.f(b.1)) {
                o[0] += bf(!b.3);
                o[1] += 1.0;
            }
            o
        }),
        play!("dl_win", "rush wins a rush snap", Pct, |r, c| if dropback(r) && c.my_def(r) {
            rc(r.rush_wins.iter().filter(|w| c.f(w.0)).count() as f64, c.on(r))
        } else {
            Z
        }),
        play!("dl_win_t", "time of a rush win, s", Num, |r, c| {
            let mut o = Z;
            for w in r.rush_wins.iter().filter(|w| c.f(w.0)) {
                o[0] += w.2;
                o[1] += 1.0;
            }
            o
        }),
        play!("dl_speed_win", "speed-rush wins a rush snap", Pct, |r, c| if dropback(r) && c.my_def(r) {
            rc(r.rush_wins.iter().filter(|w| c.f(w.0) && w.1 == Move::Speed).count() as f64, c.on(r))
        } else {
            Z
        }),
        play!("dl_power_win", "power-rush wins a rush snap", Pct, |r, c| if dropback(r) && c.my_def(r) {
            rc(r.rush_wins.iter().filter(|w| c.f(w.0) && w.1 == Move::Power).count() as f64, c.on(r))
        } else {
            Z
        }),
        play!("dl_finesse_win", "finesse-rush wins a rush snap", Pct, |r, c| if dropback(r) && c.my_def(r) {
            rc(r.rush_wins.iter().filter(|w| c.f(w.0) && w.1 == Move::Finesse).count() as f64, c.on(r))
        } else {
            Z
        }),
        play!("dl_press", "pressures a rush snap", Pct, |r, c| if dropback(r) && c.my_def(r) {
            let n = r.on_field.iter().flatten().filter(|i| c.f(**i) && pressured_by(r, **i)).count();
            rc(n as f64, c.on(r))
        } else {
            Z
        }),
        play!("dl_hit", "QB hits a rush snap", Pct, |r, c| if dropback(r) && c.my_def(r) {
            rc(r.hits.iter().filter(|i| c.f(**i)).count() as f64, c.on(r))
        } else {
            Z
        }),
        play!("dl_sack", "sacks a rush snap", Pct, |r, c| if dropback(r) && c.my_def(r) {
            rc(r.sackers.iter().filter(|i| c.f(**i)).count() as f64, c.on(r))
        } else {
            Z
        }),
        // Per defense snap, runs included, as the real numbers count them
        // (PFR pressures and sacks over all defense snaps).
        play!("dl_press_snap", "pressures a defense snap", Pct, |r, c| if scrimmage(r) && c.my_def(r) {
            let n = if dropback(r) {
                r.on_field.iter().flatten().filter(|i| c.f(**i) && pressured_by(r, **i)).count()
            } else {
                0
            };
            rc(n as f64, c.on(r))
        } else {
            Z
        }),
        play!("dl_sack_snap", "sacks a defense snap", Pct, |r, c| if scrimmage(r) && c.my_def(r) {
            rc(r.sackers.iter().filter(|i| c.f(**i)).count() as f64, c.on(r))
        } else {
            Z
        }),
        play!("dl_off", "offside a snap", Pct, |r, c| rc(fouls(r, c, &[FoulKind::Offside]), snaps(r, c))),
        play!("dl_bat", "batted passes a rush snap", Pct, |r, c| if attempt(r) && c.my_def(r) {
            rc(bf(r.tend.hands_up.is_some_and(|i| c.f(i))), c.on(r))
        } else {
            Z
        }),
        play!("dl_tfl", "tackles for loss a run snap", Pct, |r, c| if run(r) && c.my_def(r) {
            rc(if r.yards < 0.0 { r.tacklers.iter().filter(|i| c.f(**i)).count() as f64 } else { 0.0 }, c.on(r))
        } else {
            Z
        }),
        play!("dl_run_shed", "run blocks beaten (shed)", Pct, |r, c| {
            let mut o = Z;
            for b in r.blocks.iter().filter(|b| !b.2 && c.f(b.1)) {
                o[0] += bf(!b.3);
                o[1] += 1.0;
            }
            o
        }),
        play!("def_ypc", "yards a carry allowed", Num, |r, c| if run(r) && c.my_def(r) && c.on(r) > 0.0 {
            rc(r.yards, 1.0)
        } else {
            Z
        }),
        // The defense with the focus men on the field, as play-by-play
        // counts it (the linebacker research's team rows).
        play!("def_out_ypc", "yards a carry allowed outside the tackles", Num, |r, c| match (
            run(r) && c.my_def(r) && c.on(r) > 0.0,
            r.tend.cross
        ) {
            (true, Some((dy, _))) if dy.abs() > 4.0 => rc(r.yards, 1.0),
            _ => Z,
        }),
        play!(
            "def_run_succ",
            "run success allowed (EPA above 0)",
            Pct,
            |r, c| if run(r) && c.my_def(r) && c.on(r) > 0.0 { rc(bf(r.epa > 0.0), 1.0) } else { Z }
        ),
        play!("def_cmp", "completion allowed an attempt", Pct, |r, c| if attempt(r) && c.my_def(r) && c.on(r) > 0.0 {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!("def_epa_db", "EPA a dropback allowed", Num, |r, c| if dropback(r) && c.my_def(r) && c.on(r) > 0.0 {
            rc(r.epa, 1.0)
        } else {
            Z
        }),
        play!("def_blitz", "blitz rate a dropback", Pct, |r, c| if dropback(r) && c.my_def(r) && c.on(r) > 0.0 {
            rc(bf(r.blitz), 1.0)
        } else {
            Z
        }),
        play!(
            "def_sack",
            "team sacks a dropback (defense)",
            Pct,
            |r, c| if dropback(r) && c.my_def(r) && c.on(r) > 0.0 { rc(bf(r.kind == T::Sack), 1.0) } else { Z }
        ),
        play!("def_scr_yds", "scramble yards allowed a dropback", Num, |r, c| if dropback(r)
            && c.my_def(r)
            && c.on(r) > 0.0
        {
            rc(if r.kind == T::Scramble { r.yards } else { 0.0 }, 1.0)
        } else {
            Z
        }),
        play!("def_pa_ypa", "play-action yards an attempt allowed", Num, |r, c| if attempt(r)
            && r.play_action
            && c.my_def(r)
            && c.on(r) > 0.0
        {
            rc(r.yards, 1.0)
        } else {
            Z
        }),
        play!("def_bust", "busts a snap (defense)", Pct, |r, c| if scrimmage(r) && c.my_def(r) {
            rc(r.busts.iter().filter(|b| c.f(**b)).count() as f64, c.on(r))
        } else {
            Z
        }),
        play!("def_20", "plays of 20+ allowed a snap", Pct, |r, c| if scrimmage(r) && c.my_def(r) && c.on(r) > 0.0 {
            rc(bf(r.yards >= 20.0), 1.0)
        } else {
            Z
        }),
        // --- tackling (focus: the tacklers)
        play!("tk_rate", "tackles a snap", Pct, |r, c| if scrimmage(r) && c.my_def(r) {
            rc(r.tacklers.iter().filter(|i| c.f(**i)).count() as f64, c.on(r))
        } else {
            Z
        }),
        play!("tk_miss", "missed tackle rate", Pct, |r, c| if scrimmage(r) {
            let m = r.missed_tackles.iter().filter(|x| c.f(x.0)).count() as f64;
            let t = r.tacklers.iter().filter(|i| c.f(**i)).count() as f64;
            rc(m, m + t)
        } else {
            Z
        }),
        play!("tk_yaco", "yards after contact when he makes the first hit", Num, |r, c| if scrimmage(r)
            && r.tacklers.first().is_some_and(|i| c.f(*i))
        {
            rc((r.yards - r.before_contact).max(0.0), 1.0)
        } else {
            Z
        }),
        play!("tk_ff", "forced fumbles a tackle", Pct, |r, c| if scrimmage(r) {
            let t = r.tacklers.iter().filter(|i| c.f(**i)).count() as f64;
            rc(bf(c.fo(r.forced_fumble)), t)
        } else {
            Z
        }),
        play!("tk_far", "his tackles 5+ yards downfield", Pct, |r, c| if scrimmage(r) {
            let t = r.tacklers.iter().filter(|i| c.f(**i)).count() as f64;
            rc(if r.yards >= 5.0 { t } else { 0.0 }, t)
        } else {
            Z
        }),
        // Run support as PFR and play-by-play count it: runs and scrambles,
        // per defense snap he played.
        play!("tk_run_snap", "run tackles a defense snap", Pct, |r, c| if scrimmage(r) && c.my_def(r) {
            let n =
                if matches!(r.kind, T::Run | T::Scramble) { r.tacklers.iter().filter(|i| c.f(**i)).count() } else { 0 };
            rc(n as f64, c.on(r))
        } else {
            Z
        }),
        play!(
            "tk_run_stop",
            "run tackles at 3 yards or less a defense snap",
            Pct,
            |r, c| if scrimmage(r) && c.my_def(r) {
                let n = if matches!(r.kind, T::Run | T::Scramble) && r.yards <= 3.0 {
                    r.tacklers.iter().filter(|i| c.f(**i)).count()
                } else {
                    0
                };
                rc(n as f64, c.on(r))
            } else {
                Z
            }
        ),
        play!("tk_run_long", "his run tackles after 10+ yards", Pct, |r, c| if scrimmage(r)
            && matches!(r.kind, T::Run | T::Scramble)
        {
            let t = r.tacklers.iter().filter(|i| c.f(**i)).count() as f64;
            rc(if r.yards >= 10.0 { t } else { 0.0 }, t)
        } else {
            Z
        }),
        play!("lb_blitz_press", "pressures a blitz snap", Pct, |r, c| if dropback(r) && r.blitz && c.my_def(r) {
            let n = r.on_field.iter().flatten().filter(|i| c.f(**i) && pressured_by(r, **i)).count();
            rc(n as f64, c.on(r))
        } else {
            Z
        }),
        // --- coverage (focus: the nearest defender at the catch point)
        play!("cv_tgt", "targets a coverage snap", Pct, |r, c| if dropback(r) && c.my_def(r) {
            rc(bf(thrown(r) && c.fo(r.defender)), c.on(r))
        } else {
            Z
        }),
        play!("cv_tgt_snap", "targets a defense snap", Pct, |r, c| if scrimmage(r) && c.my_def(r) {
            rc(bf(thrown(r) && c.fo(r.defender)), c.on(r))
        } else {
            Z
        }),
        play!("cv_cmp", "catch rate allowed", Pct, |r, c| if thrown(r) && c.fo(r.defender) {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!("cv_cont", "contested catch rate allowed (under 1 yd)", Pct, |r, c| if thrown(r)
            && c.fo(r.defender)
            && r.separation < 1.0
        {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!("cv_tight", "targets on him into a tight window", Pct, |r, c| if thrown(r) && c.fo(r.defender) {
            rc(bf(r.separation < 1.0), 1.0)
        } else {
            Z
        }),
        play!("cv_ypt", "yards a target allowed", Num, |r, c| if thrown(r) && c.fo(r.defender) {
            rc(r.yards, 1.0)
        } else {
            Z
        }),
        play!("cv_ypc", "yards a catch allowed", Num, |r, c| if thrown(r) && r.complete && c.fo(r.defender) {
            rc(r.yards, 1.0)
        } else {
            Z
        }),
        play!("cv_adot", "depth of targets on him", Num, |r, c| if thrown(r) && c.fo(r.defender) {
            rc(r.air_yards, 1.0)
        } else {
            Z
        }),
        play!("cv_int", "picks a target on him", Pct, |r, c| if thrown(r) && c.fo(r.defender) {
            rc(bf(r.interception), 1.0)
        } else {
            Z
        }),
        play!("cv_his_int", "picks by him a target on him", Pct, |r, c| if thrown(r) && c.fo(r.defender) {
            rc(bf(r.interception && r.interceptor == r.defender), 1.0)
        } else {
            Z
        }),
        play!("cv_pbu", "breakups by him a target on him", Pct, |r, c| if thrown(r) && c.fo(r.defender) {
            rc(bf(r.pbu.is_some() && r.pbu == r.defender), 1.0)
        } else {
            Z
        }),
        play!("cv_deep", "catches of 20+ allowed a target", Pct, |r, c| if thrown(r) && c.fo(r.defender) {
            rc(bf(r.complete && r.yards >= 20.0), 1.0)
        } else {
            Z
        }),
        play!("cv_yac", "yards after the catch allowed", Num, |r, c| if thrown(r) && r.complete && c.fo(r.defender) {
            rc(r.yac, 1.0)
        } else {
            Z
        }),
        play!("cv_sep", "separation allowed, yd", Num, |r, c| if thrown(r) && c.fo(r.defender) {
            rc(r.separation, 1.0)
        } else {
            Z
        }),
        play!("cv_td", "touchdowns a target allowed", Pct, |r, c| if thrown(r) && c.fo(r.defender) {
            rc(bf(r.touchdown), 1.0)
        } else {
            Z
        }),
        Stat {
            key: "cv_rate",
            label: "passer rating allowed",
            unit: Num,
            play: Some(|r, c| {
                if thrown(r) && c.fo(r.defender) {
                    [1.0, bf(r.complete), r.yards, bf(r.touchdown), bf(r.interception)]
                } else {
                    Z
                }
            }),
            game: None,
            value: rating,
        },
        play!("cv_te_cmp", "tight ends' catch rate on him", Pct, |r, c| if thrown(r)
            && c.fo(r.defender)
            && c.is(r.target, P::TE)
        {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!("cv_back_cmp", "backs' and tight ends' catch rate on him", Pct, |r, c| if thrown(r)
            && c.fo(r.defender)
            && (c.is(r.target, P::TE) || c.is(r.target, P::RB))
        {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!("cv_middle", "catch rate allowed over the middle", Pct, |r, c| if thrown(r)
            && c.fo(r.defender)
            && middle(r)
        {
            rc(bf(r.complete), 1.0)
        } else {
            Z
        }),
        play!("cv_pi", "pass interference a coverage snap", Pct, |r, c| if c.my_def(r) {
            rc(fouls(r, c, &[FoulKind::PassInterference]), snaps(r, c))
        } else {
            Z
        }),
        play!("cv_dh", "defensive holding a coverage snap", Pct, |r, c| if c.my_def(r) {
            rc(fouls(r, c, &[FoulKind::DefensiveHolding]), snaps(r, c))
        } else {
            Z
        }),
        // --- kicking (focus: the kicker, punter, snapper or returners)
        play!("k_fg_u40", "field goals under 40 yd", Pct, |r, c| if live(r)
            && r.kind == T::FieldGoal
            && c.fo(r.kicker)
            && r.kick_distance < 40.0
        {
            rc(bf(r.kick_good), 1.0)
        } else {
            Z
        }),
        play!("k_fg40", "field goals 40-49 yd", Pct, |r, c| if live(r)
            && r.kind == T::FieldGoal
            && c.fo(r.kicker)
            && (40.0..50.0).contains(&r.kick_distance)
        {
            rc(bf(r.kick_good), 1.0)
        } else {
            Z
        }),
        play!("k_fg50", "field goals 50+ yd", Pct, |r, c| if live(r)
            && r.kind == T::FieldGoal
            && c.fo(r.kicker)
            && r.kick_distance >= 50.0
        {
            rc(bf(r.kick_good), 1.0)
        } else {
            Z
        }),
        play!("k_50_share", "field goal tries from 50+", Pct, |r, c| if live(r)
            && r.kind == T::FieldGoal
            && c.fo(r.kicker)
        {
            rc(bf(r.kick_distance >= 50.0), 1.0)
        } else {
            Z
        }),
        play!("k_fg_late", "field goals, 4th quarter close", Pct, |r, c| if live(r)
            && r.kind == T::FieldGoal
            && c.fo(r.kicker)
            && late_close(r)
        {
            rc(bf(r.kick_good), 1.0)
        } else {
            Z
        }),
        play!("k_fg_rest", "field goals, the rest", Pct, |r, c| if live(r)
            && r.kind == T::FieldGoal
            && c.fo(r.kicker)
            && !late_close(r)
        {
            rc(bf(r.kick_good), 1.0)
        } else {
            Z
        }),
        play!(
            "k_fg_dist",
            "field goal try distance",
            Num,
            |r, c| if live(r) && r.kind == T::FieldGoal && c.fo(r.kicker) { rc(r.kick_distance, 1.0) } else { Z }
        ),
        play!("k_xp", "extra points made", Pct, |r, c| if live(r) && r.kind == T::ExtraPoint && c.fo(r.kicker) {
            rc(bf(r.kick_good), 1.0)
        } else {
            Z
        }),
        play!("k_blk", "kicks blocked", Pct, |r, c| if live(r)
            && matches!(r.kind, T::FieldGoal | T::ExtraPoint)
            && c.fo(r.kicker)
        {
            rc(bf(r.blocked), 1.0)
        } else {
            Z
        }),
        play!("k_ko_tb", "kickoff touchbacks", Pct, |r, c| if live(r) && r.kind == T::Kickoff && c.fo(r.kicker) {
            rc(bf(r.touchback), 1.0)
        } else {
            Z
        }),
        play!("k_ko_dist", "kickoff distance", Num, |r, c| if live(r) && r.kind == T::Kickoff && c.fo(r.kicker) {
            rc(r.kick_distance, 1.0)
        } else {
            Z
        }),
        play!(
            "p_gross",
            "punt gross yards",
            Num,
            |r, c| if live(r) && r.kind == T::Punt && c.fo(r.kicker) && !r.blocked {
                rc(r.kick_distance, 1.0)
            } else {
                Z
            }
        ),
        play!("p_net", "punt net yards", Num, |r, c| if live(r) && r.kind == T::Punt && c.fo(r.kicker) {
            rc(r.kick_distance - r.return_yards - if r.touchback { 20.0 } else { 0.0 }, 1.0)
        } else {
            Z
        }),
        play!("p_in20", "punts inside the 20", Pct, |r, c| if live(r) && r.kind == T::Punt && c.fo(r.kicker) {
            rc(bf(r.inside20), 1.0)
        } else {
            Z
        }),
        play!("p_tb", "punt touchbacks", Pct, |r, c| if live(r) && r.kind == T::Punt && c.fo(r.kicker) {
            rc(bf(r.touchback), 1.0)
        } else {
            Z
        }),
        play!("p_blk", "punts blocked", Pct, |r, c| if live(r) && r.kind == T::Punt && c.fo(r.kicker) {
            rc(bf(r.blocked), 1.0)
        } else {
            Z
        }),
        play!("st_blk", "punts and kicks blocked (his unit)", Pct, |r, c| if live(r)
            && matches!(r.kind, T::Punt | T::FieldGoal | T::ExtraPoint)
            && c.on(r) > 0.0
        {
            rc(bf(r.blocked), 1.0)
        } else {
            Z
        }),
        play!("rt_kr", "kick return yards", Num, |r, c| if live(r)
            && r.kind == T::Kickoff
            && c.fo(r.returner)
            && !r.touchback
            && !r.fair_catch
        {
            rc(r.return_yards, 1.0)
        } else {
            Z
        }),
        play!("rt_pr", "punt return yards", Num, |r, c| if live(r)
            && r.kind == T::Punt
            && c.fo(r.returner)
            && !r.fair_catch
            && !r.touchback
            && !r.tend.muffed
        {
            rc(r.return_yards, 1.0)
        } else {
            Z
        }),
        play!("rt_20", "returns of 20+", Pct, |r, c| if live(r)
            && matches!(r.kind, T::Kickoff | T::Punt)
            && c.fo(r.returner)
            && !r.fair_catch
            && !r.touchback
        {
            rc(bf(r.return_yards >= 20.0), 1.0)
        } else {
            Z
        }),
        play!("rt_muff", "punts muffed", Pct, |r, c| if live(r) && r.kind == T::Punt && c.fo(r.returner) {
            rc(bf(r.tend.muffed), 1.0)
        } else {
            Z
        }),
        play!("rt_fair", "punts fair caught", Pct, |r, c| if live(r) && r.kind == T::Punt && c.fo(r.returner) {
            rc(bf(r.fair_catch), 1.0)
        } else {
            Z
        }),
        play!("rt_out", "end-zone kicks brought out", Pct, |r, c| match (
            live(r) && r.kind == T::Kickoff && c.fo(r.returner),
            r.tend.deep_kick
        ) {
            (true, Some(out)) => rc(bf(out), 1.0),
            _ => Z,
        }),
        // --- everyone (focus: the group)
        play!("pen", "accepted penalties a snap", Pct, |r, c| {
            rc(r.penalties.iter().filter(|p| p.4 && c.f(p.2)).count() as f64, snaps(r, c))
        }),
        play!("inj", "injuries per 1,000 snaps", Num, |r, c| {
            rc(1000.0 * r.injuries.iter().filter(|i| c.f(i.id)).count() as f64, snaps(r, c))
        }),
        game!("st_snaps", "snaps a player who played", Num, |g, c| {
            let mut played: HashSet<u32> = HashSet::new();
            let mut n = 0.0;
            for r in g.stats.plays.iter().filter(|r| scrimmage(r)) {
                for i in r.on_field.iter().flatten().filter(|i| c.f(**i)) {
                    played.insert(*i);
                    n += 1.0;
                }
            }
            rc(n, played.len() as f64)
        }),
        play!("st_q4_press", "pressures a rush snap, 4th quarter", Pct, |r, c| if dropback(r)
            && c.my_def(r)
            && r.quarter >= 4
        {
            let n = r.on_field.iter().flatten().filter(|i| c.f(**i) && pressured_by(r, **i)).count();
            rc(n as f64, c.on(r))
        } else {
            Z
        }),
        play!("st_q1_press", "pressures a rush snap, 1st quarter", Pct, |r, c| if dropback(r)
            && c.my_def(r)
            && r.quarter == 1
        {
            let n = r.on_field.iter().flatten().filter(|i| c.f(**i) && pressured_by(r, **i)).count();
            rc(n as f64, c.on(r))
        } else {
            Z
        }),
        // --- my team (the my team view: team 0)
        game!("mt_pts_for", "points scored a game", Num, |g, c| match c.mine {
            Some(m) => rc(g.stats.score[m] as f64, 1.0),
            None => rc((g.stats.score[0] + g.stats.score[1]) as f64, 2.0),
        }),
        game!("mt_pts_against", "points allowed a game", Num, |g, c| match c.mine {
            Some(m) => rc(g.stats.score[1 - m] as f64, 1.0),
            None => rc((g.stats.score[0] + g.stats.score[1]) as f64, 2.0),
        }),
        game!("mt_win", "wins (ties half)", Pct, |g, c| match c.mine {
            Some(m) => {
                let (a, b) = (g.stats.score[m], g.stats.score[1 - m]);
                rc(
                    if a > b {
                        1.0
                    } else if a == b {
                        0.5
                    } else {
                        0.0
                    },
                    1.0,
                )
            }
            None => rc(1.0, 2.0),
        }),
        play!("mt_epa_off", "EPA a play, offense", Num, |r, c| if scrimmage(r) && c.my_off(r) && c.mine.is_some() {
            rc(r.epa, 1.0)
        } else {
            Z
        }),
        play!("mt_epa_def", "EPA a play allowed, defense", Num, |r, c| if scrimmage(r)
            && c.my_def(r)
            && c.mine.is_some()
        {
            rc(r.epa, 1.0)
        } else {
            Z
        }),
        play!("mt_expl_for", "explosive plays a snap (runs 10+, passes 20+)", Pct, |r, c| if scrimmage(r)
            && c.my_off(r)
            && c.mine.is_some()
        {
            rc(bf(r.yards >= if r.kind == T::Run { 10.0 } else { 20.0 }), 1.0)
        } else {
            Z
        }),
        play!("mt_expl_against", "explosive plays allowed a snap", Pct, |r, c| if scrimmage(r)
            && c.my_def(r)
            && c.mine.is_some()
        {
            rc(bf(r.yards >= if r.kind == T::Run { 10.0 } else { 20.0 }), 1.0)
        } else {
            Z
        }),
    ];
    v.extend(lb_jobs());
    v
}

// ------------------------------------------------------- linebacker jobs

/// Focus men lined up at defensive slot `S` on this snap (`PlayRecord::roles`).
fn at_slot<'a, const S: u8>(r: &'a PlayRecord, c: &'a Cx) -> impl Iterator<Item = u32> + 'a {
    r.on_field[1].iter().zip(&r.roles[1]).filter(move |(i, s)| **s == S && c.f(**i)).map(|(i, _)| *i)
}
fn n_at<const S: u8>(r: &PlayRecord, c: &Cx) -> f64 {
    at_slot::<S>(r, c).count() as f64
}
fn is_at<const S: u8>(r: &PlayRecord, c: &Cx, id: Option<u32>) -> bool {
    id.is_some_and(|x| at_slot::<S>(r, c).any(|i| i == x))
}
fn j_share<const S: u8>(r: &PlayRecord, c: &Cx) -> C {
    if !(scrimmage(r) && c.my_def(r)) {
        return Z;
    }
    rc(n_at::<S>(r, c), r.on_field[1].iter().filter(|i| c.f(**i)).count() as f64)
}
fn j_rtk<const S: u8>(r: &PlayRecord, c: &Cx) -> C {
    if !(scrimmage(r) && c.my_def(r)) {
        return Z;
    }
    let ok = matches!(r.kind, PlayType::Run | PlayType::Scramble);
    rc(if ok { r.tacklers.iter().filter(|i| is_at::<S>(r, c, Some(**i))).count() as f64 } else { 0.0 }, n_at::<S>(r, c))
}
fn j_stop<const S: u8>(r: &PlayRecord, c: &Cx) -> C {
    if !(scrimmage(r) && c.my_def(r)) {
        return Z;
    }
    let ok = matches!(r.kind, PlayType::Run | PlayType::Scramble) && r.yards <= 3.0;
    rc(if ok { r.tacklers.iter().filter(|i| is_at::<S>(r, c, Some(**i))).count() as f64 } else { 0.0 }, n_at::<S>(r, c))
}
fn j_miss<const S: u8>(r: &PlayRecord, c: &Cx) -> C {
    if !scrimmage(r) {
        return Z;
    }
    let m = r.missed_tackles.iter().filter(|x| is_at::<S>(r, c, Some(x.0))).count() as f64;
    let t = r.tacklers.iter().filter(|i| is_at::<S>(r, c, Some(**i))).count() as f64;
    rc(m, m + t)
}
fn j_tgt<const S: u8>(r: &PlayRecord, c: &Cx) -> C {
    if !(scrimmage(r) && c.my_def(r)) {
        return Z;
    }
    rc(bf(thrown(r) && is_at::<S>(r, c, r.defender)), n_at::<S>(r, c))
}
fn j_cmp<const S: u8>(r: &PlayRecord, c: &Cx) -> C {
    if thrown(r) && is_at::<S>(r, c, r.defender) { rc(bf(r.complete), 1.0) } else { Z }
}
fn j_adot<const S: u8>(r: &PlayRecord, c: &Cx) -> C {
    if thrown(r) && is_at::<S>(r, c, r.defender) { rc(r.air_yards, 1.0) } else { Z }
}
fn j_prs<const S: u8>(r: &PlayRecord, c: &Cx) -> C {
    if !(scrimmage(r) && c.my_def(r)) {
        return Z;
    }
    let n = if dropback(r) { at_slot::<S>(r, c).filter(|i| pressured_by(r, *i)).count() as f64 } else { 0.0 };
    rc(n, n_at::<S>(r, c))
}
fn j_wt<const S: u8>(r: &PlayRecord, c: &Cx) -> C {
    if !(scrimmage(r) && c.my_def(r)) {
        return Z;
    }
    let w: f64 = at_slot::<S>(r, c).map(|i| c.body.get(&i).map_or(0.0, |b| b[0])).sum();
    rc(w, n_at::<S>(r, c))
}
fn j_forty<const S: u8>(r: &PlayRecord, c: &Cx) -> C {
    if !(scrimmage(r) && c.my_def(r)) {
        return Z;
    }
    let w: f64 = at_slot::<S>(r, c).map(|i| c.body.get(&i).map_or(0.0, |b| b[1])).sum();
    rc(w, n_at::<S>(r, c))
}

/// The same rows for each linebacker job (Mike, Sam, Will): how the man who
/// lines up there plays it, against the real jobs on NFL depth charts
/// (`afm/ab-tests/research-linebackers.md`).
fn lb_jobs() -> Vec<Stat> {
    use Unit::*;
    macro_rules! job {
        ($n:literal, $s:expr) => {
            [
                Stat {
                    key: concat!("lj_", $n, "_share"),
                    label: concat!("share of his snaps at ", $n),
                    unit: Pct,
                    play: Some(j_share::<{ $s as u8 }>),
                    game: None,
                    value: ratio,
                },
                Stat {
                    key: concat!("lj_", $n, "_rtk"),
                    label: concat!("run tackles a snap at ", $n),
                    unit: Pct,
                    play: Some(j_rtk::<{ $s as u8 }>),
                    game: None,
                    value: ratio,
                },
                Stat {
                    key: concat!("lj_", $n, "_stop"),
                    label: concat!("run stops (3 yards or less) a snap at ", $n),
                    unit: Pct,
                    play: Some(j_stop::<{ $s as u8 }>),
                    game: None,
                    value: ratio,
                },
                Stat {
                    key: concat!("lj_", $n, "_miss"),
                    label: concat!("missed tackle rate at ", $n),
                    unit: Pct,
                    play: Some(j_miss::<{ $s as u8 }>),
                    game: None,
                    value: ratio,
                },
                Stat {
                    key: concat!("lj_", $n, "_tgt"),
                    label: concat!("targets a snap at ", $n),
                    unit: Pct,
                    play: Some(j_tgt::<{ $s as u8 }>),
                    game: None,
                    value: ratio,
                },
                Stat {
                    key: concat!("lj_", $n, "_cmp"),
                    label: concat!("catch rate allowed at ", $n),
                    unit: Pct,
                    play: Some(j_cmp::<{ $s as u8 }>),
                    game: None,
                    value: ratio,
                },
                Stat {
                    key: concat!("lj_", $n, "_adot"),
                    label: concat!("depth of targets on him at ", $n),
                    unit: Num,
                    play: Some(j_adot::<{ $s as u8 }>),
                    game: None,
                    value: ratio,
                },
                Stat {
                    key: concat!("lj_", $n, "_prs"),
                    label: concat!("pressures a snap at ", $n),
                    unit: Pct,
                    play: Some(j_prs::<{ $s as u8 }>),
                    game: None,
                    value: ratio,
                },
                Stat {
                    key: concat!("lj_", $n, "_wt"),
                    label: concat!("weight of the man at ", $n),
                    unit: Num,
                    play: Some(j_wt::<{ $s as u8 }>),
                    game: None,
                    value: ratio,
                },
                Stat {
                    key: concat!("lj_", $n, "_forty"),
                    label: concat!("forty of the man at ", $n),
                    unit: Num,
                    play: Some(j_forty::<{ $s as u8 }>),
                    game: None,
                    value: ratio,
                },
            ]
        };
    }
    let mut v = Vec::new();
    v.extend(job!("mike", DSlot::Mike));
    v.extend(job!("sam", DSlot::Sam));
    v.extend(job!("will", DSlot::Will));
    v
}

pub mod run;
pub mod table;
