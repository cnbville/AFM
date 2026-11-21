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
//!
//! What fit reads is measured (scheme-fit item 24, lane SF11): a point of
//! each rating at each job (free and strong safety, Mike, Will and Sam,
//! outside and slot corner apart) is worth what the match engine paid for
//! it in each system (`data/fit_values.csv`, `fit_check values`), so a
//! staff installs and recruits to what wins in its systems.

use super::coach::Group;
use super::season::{key_normal, key_uniform};
use super::{Stage, World};
use crate::attributes::{Attr, Attributes};
use crate::game::{Front, OffScheme, Shell};
use crate::player::Player;
use crate::positions::Position;
use crate::rng::clamp;
use crate::spots::Spot;

pub const OFFENSES: [OffScheme; 5] =
    [OffScheme::WestCoast, OffScheme::AirRaid, OffScheme::WideZone, OffScheme::PowerGap, OffScheme::SpreadOption];
pub const FRONTS: [Front; 2] = [Front::FourThree, Front::ThreeFour];
pub const SHELLS: [Shell; 4] = [Shell::Cover3, Shell::Quarters, Shell::ManPressure, Shell::Cover2];

/// The three kinds of system, in the order of a fit array's slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    Offense,
    Front,
    Shell,
}

impl Kind {
    pub fn range(self) -> std::ops::Range<usize> {
        match self {
            Kind::Offense => 0..5,
            Kind::Front => 5..7,
            Kind::Shell => 7..11,
        }
    }
}

/// What a coach knows of each system, 0-1 (hidden).
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Systems {
    pub know: [f64; 11],
}

/// A program's systems this season and how much of each side's playbook
/// its players already know (1 once a system has had a full year).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
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
pub fn overlap(kind: Kind, a: usize, b: usize) -> f64 {
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

/// What a point of each rating is worth in each system, job by job, in
/// points a game (scheme-fit item 24, lane SF11): measured on the match
/// engine by `fit_check values` (`afm-tools`, `fit::values`). The level's
/// typical team ran every offense, front and coverage family with every
/// watched man's ratings and weight moved at random about his own, and a
/// rating's worth in a system is the slope of the points his side scored
/// (or kept off the board) on it. A system's weight is pooled with its
/// kind's unless the two differ past the noise, so a rating every system
/// pays alike reads the same in each. One line a weight:
/// `system,role,feature,weight` (a role is the measuring team's man, the
/// typical team's: `QB1`, `X`, `LG`, `MIKE`, `NCB`...).
const FIT_VALUES_CSV: &str = include_str!("../../data/fit_values.csv");

/// A feature the table reads: a rating's index, or his weight.
pub const WEIGHT: u8 = u8::MAX;

/// The measuring team's men a job's weights come from: the three
/// receivers' jobs are the one receiver's job, as are the two guards, the
/// two edges and the two outside corners; the nose and the
/// three-technique share the inside men's (their 3-4 weights are the
/// nose's and the ends' work).
fn roles_of(job: Spot) -> &'static [&'static str] {
    match job {
        Spot::QB => &["QB1"],
        Spot::RB => &["RB1"],
        Spot::FB => &["FB"],
        Spot::WR => &["X", "Z", "SLOT"],
        Spot::TE => &["TE1"],
        Spot::LT => &["LT"],
        Spot::RT => &["RT"],
        Spot::G => &["LG", "RG"],
        Spot::C => &["C"],
        Spot::EDGE => &["EDGE1", "EDGE2"],
        Spot::DT | Spot::NT => &["DT1", "DT2"],
        Spot::SAM => &["SAM"],
        Spot::MLB => &["MIKE"],
        Spot::OLB => &["WILL"],
        Spot::CB => &["CB1", "CB2"],
        Spot::NB => &["NCB"],
        Spot::FS => &["FS"],
        Spot::SS => &["SS"],
        Spot::K | Spot::P | Spot::LS => &[],
    }
}

/// The system slot a table name stands for (`OFFENSES`, then `FRONTS`,
/// then `SHELLS`, by their names).
fn slot_named(s: &str) -> Option<usize> {
    OFFENSES
        .iter()
        .map(|o| format!("{o:?}"))
        .chain(FRONTS.iter().map(|f| format!("{f:?}")))
        .chain(SHELLS.iter().map(|x| format!("{x:?}")))
        .position(|n| n == s)
}

fn feature_named(s: &str) -> Option<u8> {
    if s == "Weight" {
        return Some(WEIGHT);
    }
    Attr::ALL.iter().position(|a| format!("{a:?}") == s).map(|k| k as u8)
}

/// The table by job (`Spot` index) and system slot: each weight, and the
/// weights on his ratings summed (what one point on every rating is worth
/// there). By job and kind (`Kind as usize`): the kind's average weight on
/// each rating where it pays (above 0), and their sum, the rating points a
/// man's level is read in (`fit_at`).
struct Table {
    w: Vec<[Vec<(u8, f64)>; 11]>,
    total: Vec<[f64; 11]>,
    level: Vec<[Vec<(u8, f64)>; 3]>,
    level_total: Vec<[f64; 3]>,
}

fn table() -> &'static Table {
    static T: std::sync::OnceLock<Table> = std::sync::OnceLock::new();
    T.get_or_init(|| parse(FIT_VALUES_CSV))
}

fn parse(csv: &str) -> Table {
    let mut raw: std::collections::HashMap<(usize, &str), Vec<(u8, f64)>> = std::collections::HashMap::new();
    for line in csv.lines().skip(1) {
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        if f.len() < 4 {
            continue;
        }
        let (Some(slot), Some(feat), Ok(w)) = (slot_named(f[0]), feature_named(f[2]), f[3].parse::<f64>()) else {
            continue;
        };
        raw.entry((slot, f[1])).or_default().push((feat, w));
    }
    let mut t = Table {
        w: (0..crate::spots::SPOT_COUNT).map(|_| std::array::from_fn(|_| Vec::new())).collect(),
        total: vec![[0.0; 11]; crate::spots::SPOT_COUNT],
        level: (0..crate::spots::SPOT_COUNT).map(|_| std::array::from_fn(|_| Vec::new())).collect(),
        level_total: vec![[0.0; 3]; crate::spots::SPOT_COUNT],
    };
    for job in Spot::ALL {
        let roles = roles_of(job);
        if roles.is_empty() {
            continue;
        }
        for slot in 0..11 {
            let mut sum: Vec<(u8, f64)> = Vec::new();
            for r in roles {
                for &(feat, w) in raw.get(&(slot, *r)).map(Vec::as_slice).unwrap_or(&[]) {
                    match sum.iter_mut().find(|x| x.0 == feat) {
                        Some(x) => x.1 += w,
                        None => sum.push((feat, w)),
                    }
                }
            }
            for x in sum.iter_mut() {
                x.1 /= roles.len() as f64;
            }
            sum.retain(|x| x.1 != 0.0);
            t.total[job.index()][slot] = sum.iter().filter(|x| x.0 != WEIGHT).map(|x| x.1).sum();
            t.w[job.index()][slot] = sum;
        }
        for kind in [Kind::Offense, Kind::Front, Kind::Shell] {
            let r = kind.range();
            let mut mean: Vec<(u8, f64)> = Vec::new();
            for slot in r.clone() {
                for &(feat, w) in t.w[job.index()][slot].iter().filter(|x| x.0 != WEIGHT) {
                    match mean.iter_mut().find(|x| x.0 == feat) {
                        Some(x) => x.1 += w / r.len() as f64,
                        None => mean.push((feat, w / r.len() as f64)),
                    }
                }
            }
            mean.retain(|x| x.1 > 0.0);
            t.level_total[job.index()][kind as usize] = mean.iter().map(|x| x.1).sum();
            t.level[job.index()][kind as usize] = mean;
        }
    }
    t
}

/// The table's weights for a job in a system slot, for the tools and the
/// docs: (feature, points a game per point of the rating, or per pound of
/// his weight).
pub fn job_weights(job: Spot, slot: usize) -> Vec<(u8, f64)> {
    table().w[job.index()][slot].clone()
}

/// The job a man's fit is read at: the spot at his position his ratings
/// and body suit best (`spots::spot_rating`), a fullback only when he is
/// clearly one, as a staff lists a backup (`spots::choose_spots`).
pub fn fit_job(a: &Attributes, weight: f64, height: f64, pos: Position) -> Spot {
    let spots = crate::spots::spots_of(pos);
    let v = |s: Spot| crate::spots::spot_rating(s, a, weight, height) - if s == Spot::FB { 5.0 } else { 0.0 };
    let mut best = spots[0];
    for &s in &spots[1..] {
        if v(s) > v(best) {
            best = s;
        }
    }
    best
}

/// His ratings' worth in one system at a job (points a game: the table's
/// weights on his ratings), and the table's weight on his body weight.
fn score_parts(a: &impl Fn(Attr) -> f64, job: Spot, slot: usize) -> (f64, f64) {
    let mut r = 0.0;
    let mut body = 0.0;
    for &(f, w) in &table().w[job.index()][slot] {
        if f == WEIGHT {
            body = w;
        } else {
            r += w * a(Attr::ALL[f as usize]);
        }
    }
    (r, body)
}

/// His level at a job in one kind of system, before the typical college
/// player's is taken off: the kind's average weights where they pay, on his
/// ratings (`Table::level`).
fn level_score(a: &impl Fn(Attr) -> f64, job: Spot, kind: Kind) -> f64 {
    table().level[job.index()][kind as usize].iter().map(|&(f, w)| w * a(Attr::ALL[f as usize])).sum()
}

/// His fit to each system his position plays in, at a job, in points a
/// game against the average system of its kind: what his ratings are worth
/// there over the typical college player's at his job, less what a man as
/// much better or worse on every rating would gain there (fit is the shape
/// of his game, not his level: a system that makes the whole job count
/// more is no better fit for a better man), and his body weight's worth
/// there over the typical man's. His level is how far he stands above the
/// typical man in rating points, read on what the kind pays at the job.
fn fit_at(a: impl Fn(Attr) -> f64, weight: f64, job: Spot, pos: Position, norm: &JobNorm) -> [f64; 11] {
    let t = table();
    let mut f = [0.0; 11];
    for &kind in kinds(pos) {
        let r = kind.range();
        let per_point = t.level_total[job.index()][kind as usize];
        let level =
            if per_point > 1e-9 { (level_score(&a, job, kind) - norm.level[kind as usize]) / per_point } else { 0.0 };
        let mut raw = [0.0; 11];
        for slot in r.clone() {
            let (x, b) = score_parts(&a, job, slot);
            raw[slot] = x - norm.score[slot] - level * t.total[job.index()][slot] + b * (weight - norm.weight);
        }
        let mean = r.clone().map(|s| raw[s]).sum::<f64>() / r.len() as f64;
        for slot in r {
            f[slot] = raw[slot] - mean;
        }
    }
    f
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

/// The typical college player at a job's position, read in that job's
/// table (S3): his ratings' worth in each system, his weight and his level
/// in each kind (`level_score`). A system that asks for what every corner
/// lacks is not a worse fit for every roster, so fit is read against him.
/// Worked out each spring.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JobNorm {
    pub score: [f64; 11],
    pub weight: f64,
    #[serde(default)]
    pub level: [f64; 3],
}

/// One norm a job (`Spot` index).
pub type SystemNorms = [JobNorm; crate::spots::SPOT_COUNT];

pub(crate) fn system_norms(w: &World) -> SystemNorms {
    let mut sum = [JobNorm::default(); crate::spots::SPOT_COUNT];
    let mut n = [0.0; crate::spots::SPOT_COUNT];
    for p in &w.people {
        if p.stage != Stage::College {
            continue;
        }
        let pos = p.player.position;
        let a = |x: Attr| p.player.attributes.get(x);
        for &job in crate::spots::spots_of(pos) {
            let s = &mut sum[job.index()];
            for &kind in kinds(pos) {
                for slot in kind.range() {
                    s.score[slot] += score_parts(&a, job, slot).0;
                }
                s.level[kind as usize] += level_score(&a, job, kind);
            }
            s.weight += p.player.body.weight;
            n[job.index()] += 1.0;
        }
    }
    for (s, &k) in sum.iter_mut().zip(&n) {
        if k > 0.0 {
            for v in s.score.iter_mut().chain(s.level.iter_mut()) {
                *v /= k;
            }
            s.weight /= k;
        }
    }
    sum
}

/// The least a rating point at a job is taken to be worth when one man's
/// fit is put in rating points (points a game a point, on the readers'
/// scale): a third of a typical lineman's. The Sam and the fullback are
/// worth less (the typical team's nickel takes the Sam's snaps), so their
/// small fits in points don't read as large ones. Proposed, not confirmed.
const MIN_WORTH: f64 = 0.02;

/// One man's fit on the scale every reader of it was sized on (S6: the
/// staff's lean, a recruit's showcase, the market, the screens' words, about
/// 0.45 a spread): his fit in points a game (`fit_at`) as the rating points
/// at his job that would be worth as much (`Table::level_total`, at least
/// `MIN_WORTH`), in tenths, held to 1.5 either way as before.
fn on_rating_scale(f: &[f64; 11], job: Spot, pos: Position) -> [f64; 11] {
    let t = table();
    let mut out = [0.0; 11];
    for &kind in kinds(pos) {
        let per_point = t.level_total[job.index()][kind as usize].max(MIN_WORTH);
        for slot in kind.range() {
            out[slot] = (f[slot] / per_point / 10.0).clamp(-1.5, 1.5);
        }
    }
    out
}

/// A player's fit to every system his position plays in (`fit_at`, at the
/// job his game suits best, `fit_job`), against the average system of each
/// kind, on the readers' scale (`on_rating_scale`). Slots his position
/// doesn't play in are 0.
pub fn fits(p: &Player, pos: Position, norms: &SystemNorms) -> [f64; 11] {
    if kinds(pos).is_empty() {
        return [0.0; 11];
    }
    let job = fit_job(&p.attributes, p.body.weight, p.body.height, pos);
    on_rating_scale(&fit_at(|x| p.attributes.get(x), p.body.weight, job, pos, &norms[job.index()]), job, pos)
}

/// A person's fit to every system (`fits`) on the world's norms, for the
/// tools (`fit_check world`).
pub fn fits_of(w: &World, i: usize) -> [f64; 11] {
    let p = &w.people[i].player;
    fits(p, p.position, &w.system_norms)
}

/// A player's fit to what a team runs as a staff reads it (the NFL search's
/// "Scheme fit"): `fits` and `fit_to` with his attributes as `a` gives them
/// (your scouts' file) and his listed height and weight. None when the
/// file hasn't seen an attribute the systems ask about, or his position
/// plays in none.
pub fn read_fit(
    w: &World,
    pos: Position,
    listed: (f64, f64),
    a: impl Fn(Attr) -> Option<f64>,
    inst: &Installed,
) -> Option<f64> {
    if kinds(pos).is_empty() {
        return None;
    }
    let mut seen = Attributes::default();
    let mut missing = false;
    for x in Attr::ALL {
        match a(x) {
            Some(v) => seen.set(x, v),
            None => seen.set(x, 50.0),
        }
    }
    let (height, weight) = listed;
    let job = fit_job(&seen, weight, height, pos);
    let t = table();
    for &kind in kinds(pos) {
        for slot in kind.range() {
            missing |= t.w[job.index()][slot].iter().any(|&(f, _)| f != WEIGHT && a(Attr::ALL[f as usize]).is_none());
        }
    }
    if missing {
        return None;
    }
    let f = fit_at(|x| seen.get(x), weight, job, pos, &w.system_norms[job.index()]);
    Some(fit_to(&on_rating_scale(&f, job, pos), pos, inst))
}

/// The system a position plays in for a team, in words ("4-3", "Cover 3",
/// "West Coast"): the front for the front seven, the coverage for the
/// secondary, the offense for the offense. None for specialists.
pub fn system_words(pos: Position, inst: &Installed) -> Option<&'static str> {
    let k = kinds(pos);
    if k.contains(&Kind::Offense) {
        Some(inst.offense().name())
    } else if k.contains(&Kind::Front) {
        Some(match inst.front() {
            Front::FourThree => "4-3",
            Front::ThreeFour => "3-4",
        })
    } else if k.contains(&Kind::Shell) {
        Some(match inst.shell() {
            Shell::Cover3 => "Cover 3",
            Shell::Quarters => "quarters",
            Shell::ManPressure => "man and pressure",
            Shell::Cover2 => "Cover 2",
        })
    } else {
        None
    }
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
/// half how much the offense features his position, half his own fit
/// (`f`, his [`fits`]).
pub fn showcase(f: &[f64; 11], pos: Position, inst: &Installed) -> f64 {
    let fit = fit_to(f, pos, inst);
    let feat = if kinds(pos).contains(&Kind::Offense) { feature(pos, inst.off) } else { 0.0 };
    0.5 * feat + 0.5 * fit
}

/// The starters a coordinator weighs when he judges fit, by position and
/// how many: every job the table reads, the nickel with the two outside
/// corners. Each counts as much as the table says his job is worth.
const STARTERS: [(Position, usize); 11] = [
    (Position::QB, 1),
    (Position::RB, 1),
    (Position::WR, 3),
    (Position::TE, 1),
    (Position::OT, 2),
    (Position::IOL, 3),
    (Position::DT, 2),
    (Position::EDGE, 2),
    (Position::LB, 3),
    (Position::CB, 3),
    (Position::S, 2),
];

/// What each system would be worth to a roster (people indexes), in
/// points a game against the average of its kind (S3; update 2, G3: your
/// roster's fit on the Identity tab, at every level): its best men at each
/// position by overall, seated at the jobs that suit them together as a
/// staff seats its starters (`spots::best_order`), each man's fit at his
/// job summed.
pub fn roster_fit(w: &World, roster: &[usize]) -> [f64; 11] {
    let mut by_pos: Vec<Vec<(f64, usize)>> = vec![Vec::new(); crate::positions::POSITION_COUNT];
    for &i in roster {
        let p = &w.people[i];
        by_pos[p.player.position.index()].push((p.player.overall(), i));
    }
    let mut v = [0.0; 11];
    for (pos, n) in STARTERS {
        let list = &mut by_pos[pos.index()];
        list.sort_by(|a, b| b.0.total_cmp(&a.0));
        let men: Vec<usize> = list.iter().take(n).map(|x| x.1).collect();
        let spots = crate::spots::starter_spots(pos);
        let seats: Vec<(usize, Spot)> = if spots.len() > 1 && !men.is_empty() {
            let r = |i: usize, s: Spot| {
                let p = &w.people[i].player;
                crate::spots::spot_rating(s, &p.attributes, p.body.weight, p.body.height)
            };
            let order = crate::spots::best_order(&men, spots, &r);
            order.into_iter().zip(spots).filter_map(|(i, &s)| i.map(|i| (i, s))).collect()
        } else {
            men.iter().map(|&i| (i, crate::spots::spots_of(pos)[0])).collect()
        };
        for (i, job) in seats {
            let p = &w.people[i].player;
            let f = fit_at(|x| p.attributes.get(x), p.body.weight, job, pos, &w.system_norms[job.index()]);
            for &kind in kinds(pos) {
                for slot in kind.range() {
                    v[slot] += f[slot];
                }
            }
        }
    }
    v
}

/// What each system would be worth to this program's roster, in points a
/// game against the average of its kind (S3): its college players' fit.
fn roster_value(w: &World, prog: &super::program::Program) -> [f64; 11] {
    let ids: Vec<usize> = prog.roster.iter().copied().filter(|&i| w.people[i].stage == super::Stage::College).collect();
    roster_fit(w, &ids)
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
    for (k, prog) in w.programs.iter().enumerate() {
        if !prog.plays(w.year) {
            continue;
        }
        let ids: Vec<usize> = prog.roster.iter().copied().filter(|&i| w.people[i].stage == Stage::College).collect();
        let players: Vec<&Player> = ids.iter().map(|&i| &w.people[i].player).collect();
        let spots = crate::spots::choose_spots(&players);
        let mine = w.seat_program(k);
        moves.extend(ids.into_iter().zip(spots));
        if mine {
            named_jobs(w, &mut moves);
        }
    }
    apply_spots(w, moves);
}

/// Training camp in the NFL (L1): each staff lists its spots as the
/// college staffs do each spring, so a man moved to a new job learns it
/// over a camp, your club's named jobs included.
pub(crate) fn set_club_spots(w: &mut World) {
    let mut moves: Vec<(usize, crate::spots::Spot)> = Vec::new();
    for (k, club) in w.clubs.iter().enumerate() {
        let ids: Vec<usize> = club.roster.clone();
        if ids.is_empty() {
            continue;
        }
        let players: Vec<&Player> = ids.iter().map(|&i| &w.people[i].player).collect();
        let spots = crate::spots::choose_spots(&players);
        moves.extend(ids.into_iter().zip(spots));
        if w.seat_club(k) {
            named_jobs(w, &mut moves);
        }
    }
    apply_spots(w, moves);
}

/// Your board's named jobs (linebackers, corners, safeties, and the line's
/// tackles, guards and center; `spots::board_jobs`) replace your staff's
/// for the men you named: a man named at another position's job learns it
/// without leaving his own position's spot.
fn named_jobs(w: &World, moves: &mut [(usize, crate::spots::Spot)]) {
    for (pos, ids) in &w.seat_depth {
        for (&id, &job) in ids.iter().zip(crate::spots::board_jobs(*pos)) {
            if let Some(m) = moves.iter_mut().find(|m| w.people[m.0].player.id == id) {
                m.1 = job;
            }
        }
    }
}

fn apply_spots(w: &mut World, moves: Vec<(usize, crate::spots::Spot)>) {
    for (i, s) in moves {
        let p = &mut w.people[i].player;
        if s.position() == p.position {
            crate::spots::set_spot(p, s);
        } else {
            p.spots_played |= s.bit();
        }
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
        // The head coach's guidelines (update 2, G2) weigh in: a firm one
        // all but decides the system, a lean tips a close call.
        let guide = super::guide::program_guidelines(w, prog);
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
                    + super::guide::install_bonus(&guide, kind, i)
            };
            // What he knows well enough to run, and what the head coach
            // asks for (he runs it as well as he knows it).
            let options: Vec<usize> = (0..r.len())
                .filter(|&i| coach.systems.know[r.start + i] >= 0.5 || super::guide::install_wanted(&guide, kind, i))
                .collect();
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
pub(crate) fn nfl_install(
    w: &World,
    mut inst: Installed,
    oc: Option<usize>,
    dc: Option<usize>,
    guide: &crate::game::guide::Guidelines,
) -> Installed {
    let mut known: [f64; 2] = [1.0, 1.0];
    for (kind, caller) in [(Kind::Offense, oc), (Kind::Front, dc), (Kind::Shell, dc)] {
        let Some(c) = caller else { continue };
        let r = kind.range();
        let know = &w.coaches[c].systems.know;
        // The head coach's guidelines (update 2, G2) weigh in.
        let val = |s: usize| know[s] + super::guide::nfl_bonus(guide, kind, s - r.start);
        let Some(best) = r.clone().max_by(|&a, &b| val(a).total_cmp(&val(b))) else { continue };
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
