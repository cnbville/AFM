//! Coaching systems (dynasty item 2, S1-S7 and S11): what each coach knows,
//! which system a program runs and why, what a switch costs, and how fit
//! shows in recruiting. A system is an offense (West Coast, air raid, wide
//! zone, power gap, spread option), a front (one-gap 4-3, two-gap 3-4) or a
//! coverage family (Cover 3, quarters, man pressure, Cover 2).
//!
//! Real life settles the shape: a coach carries the system he grew up in
//! (S1), picks up his head coach's (S2), fits it to his players when the
//! gain is clear (S3), reads fit better the better he is (S4), pays a year
//! of learning for a switch (S5), and recruits to it (S6, S7). Fit is worth
//! a point or three, less than talent (S11). Sizes are proposed, not
//! confirmed.

use super::coach::Group;
use super::season::{key_normal, key_uniform};
use super::{Stage, World};
use crate::attributes::Attr::{self, *};
use crate::game::{Front, OffScheme, Shell};
use crate::player::Player;
use crate::positions::Position;
use crate::rng::clamp;

pub const OFFENSES: [OffScheme; 5] =
    [OffScheme::WestCoast, OffScheme::AirRaid, OffScheme::WideZone, OffScheme::PowerGap, OffScheme::SpreadOption];
pub const FRONTS: [Front; 2] = [Front::FourThree, Front::ThreeFour];
pub const SHELLS: [Shell; 4] = [Shell::Cover3, Shell::Quarters, Shell::ManPressure, Shell::Cover2];

/// The three kinds of system, in the order of a fit array's slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Offense,
    Front,
    Shell,
}

impl Kind {
    fn range(self) -> std::ops::Range<usize> {
        match self {
            Kind::Offense => 0..5,
            Kind::Front => 5..7,
            Kind::Shell => 7..11,
        }
    }
}

/// What a coach knows of each system, 0-1 (hidden).
#[derive(Clone, Copy, Debug, Default)]
pub struct Systems {
    pub know: [f64; 11],
}

/// A program's systems this season and how much of each side's playbook
/// its players already know (1 once a system has had a full year).
#[derive(Clone, Copy, Debug)]
pub struct Installed {
    /// Slots into `OFFENSES`, `FRONTS` and `SHELLS`.
    pub off: usize,
    pub front: usize,
    pub shell: usize,
    pub off_known: f64,
    pub def_known: f64,
    /// Who installed each (offense, front, shell); None before the first.
    pub by: [Option<usize>; 3],
}

impl Default for Installed {
    fn default() -> Self {
        Installed { off: 0, front: 0, shell: 0, off_known: 1.0, def_known: 1.0, by: [None; 3] }
    }
}

impl Installed {
    pub fn offense(&self) -> OffScheme {
        OFFENSES[self.off]
    }
    pub fn front(&self) -> Front {
        FRONTS[self.front]
    }
    pub fn shell(&self) -> Shell {
        SHELLS[self.shell]
    }
    /// The slot in a fit array for each kind.
    fn slot(&self, kind: Kind) -> usize {
        match kind {
            Kind::Offense => self.off,
            Kind::Front => 5 + self.front,
            Kind::Shell => 7 + self.shell,
        }
    }
}

/// The home-system leanings of each group a coach can come up in (S1):
/// quarterback and receiver coaches lean to passing systems, line coaches
/// to zone or gap runs, DB coaches to coverage families, DL coaches to
/// fronts and pressure. The other side of the ball he learns from the
/// common mix (about a third of college fronts are 3-4).
fn leanings(g: Group) -> [f64; 11] {
    let (off, front, shell): ([f64; 5], [f64; 2], [f64; 4]) = match g {
        Group::QB => ([0.45, 0.35, 0.05, 0.0, 0.15], [0.65, 0.35], [0.3, 0.25, 0.2, 0.25]),
        Group::Receivers => ([0.4, 0.45, 0.05, 0.0, 0.1], [0.65, 0.35], [0.3, 0.25, 0.2, 0.25]),
        Group::RB => ([0.1, 0.05, 0.3, 0.3, 0.25], [0.65, 0.35], [0.3, 0.25, 0.2, 0.25]),
        Group::OL => ([0.05, 0.05, 0.4, 0.4, 0.1], [0.65, 0.35], [0.3, 0.25, 0.2, 0.25]),
        Group::DL => ([0.25, 0.2, 0.2, 0.2, 0.15], [0.55, 0.45], [0.3, 0.1, 0.45, 0.15]),
        Group::LB => ([0.25, 0.2, 0.2, 0.2, 0.15], [0.5, 0.5], [0.4, 0.2, 0.25, 0.15]),
        Group::DB => ([0.25, 0.2, 0.2, 0.2, 0.15], [0.7, 0.3], [0.25, 0.35, 0.1, 0.3]),
        Group::Specialists => ([0.25, 0.2, 0.2, 0.2, 0.15], [0.65, 0.35], [0.3, 0.25, 0.2, 0.25]),
    };
    let mut w = [0.0; 11];
    w[..5].copy_from_slice(&off);
    w[5..7].copy_from_slice(&front);
    w[7..].copy_from_slice(&shell);
    w
}

fn weighted(w: &[f64], u: f64, skip: Option<usize>) -> usize {
    let total: f64 = w.iter().enumerate().filter(|(i, _)| Some(*i) != skip).map(|x| x.1).sum();
    let mut x = u * total;
    for (i, &v) in w.iter().enumerate() {
        if Some(i) == skip {
            continue;
        }
        if x < v {
            return i;
        }
        x -= v;
    }
    w.iter().enumerate().rposition(|(i, &v)| v > 0.0 && Some(i) != skip).unwrap_or(0)
}

/// A new coach's systems (S1, S4): his home system in each kind from the
/// group he came up in, known well; one more he knows partly; the rest a
/// little. Better scheme coaches know more of the others. Keyed draws, so
/// the rest of the world's draws keep their values.
pub fn draw(id: usize, came_up: Group, scheme: f64, salt: u64) -> Systems {
    let lean = leanings(came_up);
    let u = |k: u64| key_uniform(salt ^ 0x5157_3A11, (id as u64) << 8 | k);
    let breadth = 0.025 * (scheme - 10.0);
    let mut know = [0.0; 11];
    for (n, kind) in [Kind::Offense, Kind::Front, Kind::Shell].into_iter().enumerate() {
        let r = kind.range();
        let w = &lean[r.clone()];
        let home = weighted(w, u(10 * n as u64 + 1), None);
        let second = weighted(w, u(10 * n as u64 + 2), Some(home));
        for (i, slot) in r.clone().enumerate() {
            know[slot] = if i == home {
                0.85 + 0.15 * u(10 * n as u64 + 3)
            } else if i == second {
                clamp(0.4 + 0.3 * u(10 * n as u64 + 4) + breadth, 0.2, 0.8)
            } else {
                clamp(0.1 + 0.25 * u(100 + slot as u64) + breadth, 0.0, 0.6)
            };
        }
    }
    Systems { know }
}

/// An assistant picks up part of his head coach's systems each year he
/// works under him (S2, the confirmed "ideas spread" topic).
pub fn learn_from(own: &mut Systems, head: &Systems) {
    for (k, h) in own.know.iter_mut().zip(head.know) {
        *k += 0.15 * (h - *k).max(0.0);
    }
}

/// How much of one system's playbook carries over to another (S5): the
/// share of plays and rules the two have in common (proposed, not
/// confirmed; the wide zone grew out of the West Coast, the spread option
/// shares the zone runs).
fn overlap(kind: Kind, a: usize, b: usize) -> f64 {
    if a == b {
        return 1.0;
    }
    let (a, b) = (a.min(b), a.max(b));
    match kind {
        Kind::Offense => match (a, b) {
            (0, 1) => 0.6,
            (0, 2) => 0.5,
            (0, 3) => 0.3,
            (0, 4) => 0.3,
            (1, 2) => 0.2,
            (1, 3) => 0.1,
            (1, 4) => 0.4,
            (2, 3) => 0.4,
            (2, 4) => 0.5,
            _ => 0.3,
        },
        Kind::Front => 0.5,
        Kind::Shell => match (a, b) {
            (0, 1) => 0.5,
            (0, 2) => 0.5,
            (0, 3) => 0.4,
            (1, 2) => 0.3,
            (1, 3) => 0.6,
            _ => 0.3,
        },
    }
}

/// A player's score in what one system asks of his position (weighted
/// attributes; body weight for a two-gap nose).
fn score(p: &Player, pos: Position, slot: usize) -> f64 {
    use Position as P;
    let a = |x: Attr| p.attributes[x];
    let mix = |w: &[(Attr, f64)]| w.iter().map(|&(x, v)| a(x) * v).sum::<f64>();
    let heavy = clamp((p.body.weight - 250.0) / 1.2, 20.0, 99.0);
    match (pos, slot) {
        (P::QB, 0) => {
            mix(&[(ShortAccuracy, 0.3), (Decisions, 0.2), (Anticipation, 0.2), (QuickRelease, 0.15), (Footwork, 0.15)])
        }
        (P::QB, 1) => mix(&[
            (ShortAccuracy, 0.2),
            (MediumAccuracy, 0.2),
            (DeepAccuracy, 0.15),
            (ArmStrength, 0.15),
            (QuickRelease, 0.15),
            (Decisions, 0.15),
        ]),
        (P::QB, 2) => {
            mix(&[(ThrowOnMove, 0.3), (BallHandling, 0.25), (MediumAccuracy, 0.2), (Speed, 0.15), (Agility, 0.1)])
        }
        (P::QB, 3) => mix(&[
            (ArmStrength, 0.3),
            (DeepAccuracy, 0.2),
            (PocketPresence, 0.2),
            (BallHandling, 0.15),
            (Footwork, 0.15),
        ]),
        (P::QB, 4) => mix(&[
            (Speed, 0.3),
            (Decisions, 0.2),
            (BallHandling, 0.15),
            (Elusiveness, 0.15),
            (ShortAccuracy, 0.1),
            (Agility, 0.1),
        ]),
        (P::RB, 0) => mix(&[(Catching, 0.35), (BallCarrierVision, 0.25), (RouteRunning, 0.2), (Agility, 0.2)]),
        (P::RB, 1) => mix(&[(Catching, 0.4), (PassBlocking, 0.35), (Speed, 0.25)]),
        (P::RB, 2) => mix(&[(BallCarrierVision, 0.45), (Acceleration, 0.3), (Agility, 0.25)]),
        (P::RB, 3) => mix(&[
            (LegDrive, 0.3),
            (Balance, 0.2),
            (Acceleration, 0.2),
            (BallSecurity, 0.15),
            (BallCarrierVision, 0.15),
        ]),
        (P::RB, 4) => mix(&[(Speed, 0.4), (Acceleration, 0.3), (BallCarrierVision, 0.3)]),
        (P::WR, 0) => mix(&[(RouteRunning, 0.45), (Catching, 0.3), (Agility, 0.15), (FootballIq, 0.1)]),
        (P::WR, 1) => mix(&[(RouteRunning, 0.3), (Speed, 0.3), (Catching, 0.25), (FootballIq, 0.15)]),
        (P::WR, 2) => mix(&[(Speed, 0.4), (BlockingInSpace, 0.3), (RouteRunning, 0.3)]),
        (P::WR, 3) => mix(&[(BlockingInSpace, 0.35), (ContestedCatch, 0.3), (Catching, 0.2), (Strength, 0.15)]),
        (P::WR, 4) => mix(&[(Speed, 0.45), (BlockingInSpace, 0.3), (Catching, 0.25)]),
        (P::TE, 0) => mix(&[(Catching, 0.4), (RouteRunning, 0.4), (RunBlocking, 0.2)]),
        (P::TE, 1) => mix(&[(Catching, 0.5), (RouteRunning, 0.5)]),
        (P::TE, 2) => mix(&[(RunBlocking, 0.35), (Agility, 0.25), (Catching, 0.25), (BlockingInSpace, 0.15)]),
        (P::TE, 3) => mix(&[(RunBlocking, 0.45), (HandUse, 0.2), (Strength, 0.2), (Anchor, 0.15)]),
        (P::TE, 4) => mix(&[(BlockingInSpace, 0.4), (Catching, 0.3), (Speed, 0.3)]),
        (P::OT | P::IOL, 0) => mix(&[(PassBlocking, 0.55), (Agility, 0.2), (RunBlocking, 0.15), (HandUse, 0.1)]),
        (P::OT | P::IOL, 1) => mix(&[(PassBlocking, 0.7), (Anchor, 0.15), (Agility, 0.15)]),
        (P::OT | P::IOL, 2) => mix(&[(Agility, 0.3), (BlockingInSpace, 0.3), (RunBlocking, 0.2), (Speed, 0.2)]),
        (P::OT | P::IOL, 3) => {
            mix(&[(RunBlocking, 0.4), (Strength, 0.3), (Anchor, 0.1), (BlockingInSpace, 0.1), (HandUse, 0.1)])
        }
        (P::OT | P::IOL, 4) => mix(&[(RunBlocking, 0.4), (Agility, 0.25), (BlockingInSpace, 0.2), (Strength, 0.15)]),
        (P::DT, 5) => {
            mix(&[(Acceleration, 0.35), (FinesseRush, 0.2), (PowerRush, 0.15), (Agility, 0.15), (Anticipation, 0.15)])
        }
        (P::DT, 6) => mix(&[(Anchor, 0.25), (BlockShedding, 0.2), (HandUse, 0.2), (Strength, 0.15)]) + 0.2 * heavy,
        (P::EDGE, 5) => {
            mix(&[(FinesseRush, 0.25), (PowerRush, 0.2), (Acceleration, 0.2), (BlockShedding, 0.2), (Anchor, 0.15)])
        }
        (P::EDGE, 6) => {
            mix(&[(Speed, 0.25), (FinesseRush, 0.25), (Flexibility, 0.2), (ZoneCoverage, 0.15), (Pursuit, 0.15)])
        }
        (P::LB, 5) => mix(&[(Speed, 0.3), (Pursuit, 0.25), (PlayRecognition, 0.25), (ZoneCoverage, 0.2)]),
        (P::LB, 6) => mix(&[(Tackling, 0.3), (BlockShedding, 0.25), (Traffic, 0.2), (Anchor, 0.15), (Strength, 0.1)]),
        (P::CB, 7) => mix(&[(Speed, 0.4), (ZoneCoverage, 0.35), (Anticipation, 0.15), (BallTracking, 0.1)]),
        (P::CB, 8) => mix(&[(ZoneCoverage, 0.35), (PlayRecognition, 0.25), (Tackling, 0.25), (ManCoverage, 0.15)]),
        (P::CB, 9) => mix(&[(ManCoverage, 0.45), (Speed, 0.35), (Press, 0.2)]),
        (P::CB, 10) => mix(&[(Press, 0.3), (ZoneCoverage, 0.3), (Tackling, 0.3), (Strength, 0.1)]),
        (P::S, 7) => mix(&[(Speed, 0.45), (ZoneCoverage, 0.25), (BallTracking, 0.15), (PlayRecognition, 0.15)]),
        (P::S, 8) => mix(&[(PlayRecognition, 0.35), (Tackling, 0.25), (ZoneCoverage, 0.25), (Anticipation, 0.15)]),
        (P::S, 9) => mix(&[(ManCoverage, 0.5), (Speed, 0.3), (Tackling, 0.2)]),
        (P::S, 10) => mix(&[(Speed, 0.35), (ZoneCoverage, 0.35), (BallTracking, 0.15), (PlayRecognition, 0.15)]),
        (P::LB, 9) => mix(&[(ManCoverage, 0.5), (Speed, 0.35), (Blitzing, 0.15)]),
        (P::LB, 7 | 8 | 10) => mix(&[(ZoneCoverage, 0.6), (PlayRecognition, 0.4)]),
        (P::DT | P::EDGE, 9) => mix(&[(Acceleration, 0.4), (FinesseRush, 0.3), (PowerRush, 0.3)]),
        (P::DT | P::EDGE, 7 | 8 | 10) => mix(&[(BlockShedding, 0.5), (PowerRush, 0.25), (FinesseRush, 0.25)]),
        _ => 0.0,
    }
}

/// Whether a position plays in the offense's system.
pub fn offense(pos: Position) -> bool {
    kinds(pos).contains(&Kind::Offense)
}

/// Which kinds of system a position plays in.
fn kinds(pos: Position) -> &'static [Kind] {
    use Position as P;
    match pos {
        P::QB | P::RB | P::WR | P::TE | P::OT | P::IOL => &[Kind::Offense],
        P::DT | P::EDGE | P::LB => &[Kind::Front, Kind::Shell],
        P::CB | P::S => &[Kind::Shell],
        _ => &[],
    }
}

/// Each position's average score in each system among college players
/// (S3): a system that asks for what every corner lacks (tackling) is not
/// a worse fit for every roster, so fit is read against the typical player
/// at the position. Worked out each spring.
pub type SystemNorms = [[f64; 11]; crate::positions::POSITION_COUNT];

pub(crate) fn system_norms(w: &World) -> SystemNorms {
    let mut sum = [[0.0; 11]; crate::positions::POSITION_COUNT];
    let mut n = [0.0; crate::positions::POSITION_COUNT];
    for p in &w.people {
        if p.stage != Stage::College {
            continue;
        }
        let pos = p.player.position;
        for &kind in kinds(pos) {
            for slot in kind.range() {
                sum[pos.index()][slot] += score(&p.player, pos, slot);
            }
        }
        n[pos.index()] += 1.0;
    }
    for (row, &k) in sum.iter_mut().zip(&n) {
        if k > 0.0 {
            for v in row.iter_mut() {
                *v /= k;
            }
        }
    }
    sum
}

/// A player's fit to every system his position plays in: his score there
/// over the typical player's at his position, against his average across
/// that kind, in tenths of a rating point scale (about -1 to 1). Slots his
/// position doesn't play in are 0.
pub fn fits(p: &Player, pos: Position, norms: &SystemNorms) -> [f64; 11] {
    let mut f = [0.0; 11];
    for &kind in kinds(pos) {
        let r = kind.range();
        // On the stack: recruiting weighs this for every offer.
        let mut buf = [0.0; 11];
        let s = &mut buf[..r.len()];
        for (i, slot) in r.clone().enumerate() {
            s[i] = score(p, pos, slot) - norms[pos.index()][slot];
        }
        let mean = s.iter().sum::<f64>() / s.len() as f64;
        for (i, slot) in r.enumerate() {
            f[slot] = clamp((s[i] - mean) / 10.0, -1.5, 1.5);
        }
    }
    f
}

/// His fit to what a program runs now (S6): the mean over the kinds his
/// position plays in.
pub fn fit_to(f: &[f64; 11], pos: Position, inst: &Installed) -> f64 {
    let k = kinds(pos);
    if k.is_empty() {
        return 0.0;
    }
    k.iter().map(|&kind| f[inst.slot(kind)]).sum::<f64>() / k.len() as f64
}

/// How much a system features a position (S7): receivers want the ball
/// thrown, backs want carries, a tight end in the air raid barely plays.
fn feature(pos: Position, off: usize) -> f64 {
    use Position as P;
    const T: [[f64; 5]; 4] = [
        // WC, AR, WZ, PG, SO
        [0.5, 1.0, -0.3, -0.6, 0.0], // WR
        [0.0, -0.6, 0.6, 0.6, 0.3],  // RB
        [0.5, -0.8, 0.4, 0.4, -0.2], // TE
        [0.4, 0.8, 0.0, -0.3, 0.0],  // QB
    ];
    match pos {
        P::WR => T[0][off],
        P::RB => T[1][off],
        P::TE => T[2][off],
        P::QB => T[3][off],
        _ => 0.0,
    }
}

/// How well a program's systems show a recruit off (S7), about -1 to 1:
/// half how much the offense features his position, half his own fit.
pub fn showcase(p: &Player, inst: &Installed, norms: &SystemNorms) -> f64 {
    let pos = p.position;
    let f = fits(p, pos, norms);
    let fit = fit_to(&f, pos, inst);
    let feat = if kinds(pos).contains(&Kind::Offense) { feature(pos, inst.off) } else { 0.0 };
    0.5 * feat + 0.5 * fit
}

/// The starters a coordinator weighs when he judges fit, by position:
/// (position, how many, weight per player). The quarterback counts most.
const WEIGH: [(Position, usize, f64); 11] = [
    (Position::QB, 1, 3.0),
    (Position::RB, 1, 1.0),
    (Position::WR, 3, 1.0),
    (Position::TE, 1, 1.0),
    (Position::OT, 2, 1.0),
    (Position::IOL, 3, 1.0),
    (Position::DT, 2, 1.0),
    (Position::EDGE, 2, 1.0),
    (Position::LB, 3, 1.0),
    (Position::CB, 2, 1.0),
    (Position::S, 2, 1.0),
];

/// Points a game one unit of weighted lineup fit is worth (S11): the best
/// and worst system for a typical roster end up 1-3 points apart.
const FIT_POINTS: f64 = 2.5;

/// What each system would be worth to this roster, in points a game
/// against the average of its kind (S3).
fn roster_value(w: &World, prog: &super::program::Program) -> [f64; 11] {
    let mut by_pos: Vec<Vec<(f64, usize)>> = vec![Vec::new(); crate::positions::POSITION_COUNT];
    for &i in &prog.roster {
        let p = &w.people[i];
        if p.stage != super::Stage::College {
            continue;
        }
        by_pos[p.player.position.index()].push((p.player.overall(), i));
    }
    let mut v = [0.0; 11];
    let mut weight = [0.0; 3];
    for (pos, n, wt) in WEIGH {
        let list = &mut by_pos[pos.index()];
        list.sort_by(|a, b| b.0.total_cmp(&a.0));
        for &(_, i) in list.iter().take(n) {
            let p = &w.people[i];
            let f = fits(&p.player, pos, &w.system_norms);
            for &kind in kinds(pos) {
                for slot in kind.range() {
                    v[slot] += wt * f[slot];
                }
                weight[kind as usize] += wt;
            }
        }
    }
    for kind in [Kind::Offense, Kind::Front, Kind::Shell] {
        let wt = weight[kind as usize].max(1.0);
        for slot in kind.range() {
            v[slot] *= FIT_POINTS * 4.0 / wt;
        }
    }
    v
}

/// Each spring every college staff settles its systems (S3-S5): the play
/// caller weighs every system he knows well enough to run against this
/// roster, reading fit as well as his scheme sense lets him (S4). A new
/// caller installs the system he likes best; a caller who stays switches
/// only for a clear gain (more than a point a game after the learning
/// cost). A switch leaves the players knowing only the shared part of
/// the playbook for a year (S5).
/// Spring: each staff sets its depth-chart spots (fix list topic 3): left
/// and right tackle, guards and center, nose and three-technique, middle
/// and outside linebackers, the slot corner, free and strong safety, the
/// fullback. A spot once listed stays familiar to him.
pub(crate) fn set_spots(w: &mut World) {
    let mut moves: Vec<(usize, crate::spots::Spot)> = Vec::new();
    for prog in &w.programs {
        if !prog.plays(w.year) {
            continue;
        }
        let ids: Vec<usize> = prog.roster.iter().copied().filter(|&i| w.people[i].stage == Stage::College).collect();
        let players: Vec<&Player> = ids.iter().map(|&i| &w.people[i].player).collect();
        let spots = crate::spots::choose_spots(&players);
        moves.extend(ids.into_iter().zip(spots));
    }
    for (i, s) in moves {
        crate::spots::set_spot(&mut w.people[i].player, s);
    }
}

pub(crate) fn spring_install(w: &mut World) {
    w.system_norms = system_norms(w);
    let seed = w.options.seed as u64 ^ ((w.year as u64) << 20);
    let mut plans = Vec::new();
    for prog in &w.programs {
        if !prog.plays(w.year) {
            continue;
        }
        // Before a roster is there to judge, a staff runs what it knows.
        let value = if prog.roster.len() >= 30 { roster_value(w, prog) } else { [0.0; 11] };
        let mut inst = prog.installed;
        // A year on: last spring's new playbook is known by now.
        let mut known: [f64; 2] = [1.0, 1.0];
        for (kind, caller) in [(Kind::Offense, prog.oc), (Kind::Front, prog.dc), (Kind::Shell, prog.dc)]
            .map(|(k, c)| (k, c.or(prog.head_coach)))
        {
            let Some(c) = caller else { continue };
            let coach = &w.coaches[c];
            let scheme = coach.ability().scheme;
            let r = kind.range();
            let cur = inst.slot(kind) - r.start;
            let read_sd = 0.8 * (20.0 - scheme) / 10.0;
            let est = |i: usize| {
                let slot = r.start + i;
                // Running a system he knows less well costs him on Saturdays.
                value[slot] - 2.0 * (1.0 - coach.systems.know[slot])
                    + read_sd * key_normal(seed ^ (prog.id as u64) << 4, (c as u64) << 8 | slot as u64)
            };
            let options: Vec<usize> = (0..r.len()).filter(|&i| coach.systems.know[r.start + i] >= 0.5).collect();
            let best = options.iter().copied().max_by(|&a, &b| est(a).total_cmp(&est(b)));
            let Some(best) = best else { continue };
            let new_caller = inst.by[kind as usize] != Some(c);
            let cost = |i: usize| 1.5 * (1.0 - overlap(kind, cur, i));
            let pick = if inst.by[kind as usize].is_none() {
                best
            } else if new_caller {
                // A new coordinator brings what he wants to run, the
                // learning cost weighed in.
                options
                    .iter()
                    .copied()
                    .max_by(|&a, &b| (est(a) - cost(a)).total_cmp(&(est(b) - cost(b))))
                    .unwrap_or(cur)
            } else if best != cur && est(best) - cost(best) > est(cur) + 1.0 {
                best
            } else {
                cur
            };
            let side = if kind == Kind::Offense { 0 } else { 1 };
            if inst.by[kind as usize].is_some() && pick != cur {
                known[side] = known[side].min(overlap(kind, cur, pick));
            }
            match kind {
                Kind::Offense => inst.off = pick,
                Kind::Front => inst.front = pick,
                Kind::Shell => inst.shell = pick,
            }
            inst.by[kind as usize] = Some(c);
        }
        inst.off_known = known[0];
        inst.def_known = known[1];
        plans.push((prog.id, inst));
    }
    let mut mix = [0u32; 11];
    for (p, inst) in plans {
        let old = w.programs[p].installed;
        if old.by.iter().all(|b| b.is_some()) && (old.off, old.front, old.shell) != (inst.off, inst.front, inst.shell) {
            w.log.system_switches += 1;
        }
        if w.programs[p].division == super::data::Division::FBS {
            for s in [inst.off, 5 + inst.front, 7 + inst.shell] {
                mix[s] += 1;
            }
        }
        w.programs[p].installed = inst;
    }
    w.log.fbs_systems = mix;
}

/// An NFL staff's systems for the year (section 5): each coordinator runs
/// the system he knows best; a new caller's playbook is known as far as it
/// shares the old one (S5). Proposed, not confirmed: NFL staffs don't weigh
/// the roster's fit as college staffs do in spring.
pub(crate) fn nfl_install(w: &World, mut inst: Installed, oc: Option<usize>, dc: Option<usize>) -> Installed {
    let mut known: [f64; 2] = [1.0, 1.0];
    for (kind, caller) in [(Kind::Offense, oc), (Kind::Front, dc), (Kind::Shell, dc)] {
        let Some(c) = caller else { continue };
        let r = kind.range();
        let know = &w.coaches[c].systems.know;
        let Some(best) = r.clone().max_by(|&a, &b| know[a].total_cmp(&know[b])) else { continue };
        let pick = best - r.start;
        let cur = inst.slot(kind) - r.start;
        let side = if kind == Kind::Offense { 0 } else { 1 };
        if inst.by[kind as usize].is_some() && pick != cur {
            known[side] = known[side].min(overlap(kind, cur, pick));
        }
        match kind {
            Kind::Offense => inst.off = pick,
            Kind::Front => inst.front = pick,
            Kind::Shell => inst.shell = pick,
        }
        inst.by[kind as usize] = Some(c);
    }
    inst.off_known = known[0];
    inst.def_known = known[1];
    inst
}
