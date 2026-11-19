//! The split ceiling (the progression update: `docs/design/progression.md`
//! and `docs/design/progression-everyday.md`, both agreed with Cain on
//! 2026-11-14/15).
//!
//! A technique skill has two tops. The everyday ceiling is today's
//! per-skill ceiling (`Player.ceilings`). The limit is what his body and
//! makeup allow: his tools' rank among players at his position, mapped onto
//! the skill ceilings players there are born with, plus 4, plus the whole
//! gap when that level sits above his own ceiling (design item 4). Room is
//! the limit less the everyday ceiling. Coaching closes room every year,
//! by a hidden knack for that kind of skill (items 1 and 5); a breakout
//! closes it at a click's size (everyday item 5). The limit never holds a
//! natural rise back: a ceiling that passes it lifts it (everyday item 2),
//! and it never takes a rating away when his body fades (everyday item 4).
//! The limit only ever adds room, so it never caps a freak.
//!
//! The body part of the read is his physical ceilings as they are now
//! (item 1), the makeup part his makeup ceilings as they are now: they move
//! with his drift, so the limit grows with him (everyday item 10; ceilings,
//! not today's ratings, so a boy's young reads don't hold his limit down).

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::attributes::{Attr, Attributes};
use crate::player::{Level, Player, PlayingTime};
use crate::positions::{POSITION_COUNT, Position, weight};
use crate::rng::{Rng, clamp};

/// The skills with a limit (design item 6 and everyday item 9). The rest
/// keep no limit and work as before (everyday item 8: gifts and the
/// mental reads, and the skills neither list names).
pub const LIMITED: [Attr; LIMITED_COUNT] = [
    Attr::ManCoverage,
    Attr::ZoneCoverage,
    Attr::Press,
    Attr::Tackling,
    Attr::BlockShedding,
    Attr::ContestedCatch,
    Attr::RouteRunning,
    Attr::Release,
    Attr::PowerRush,
    Attr::FinesseRush,
    Attr::PassBlocking,
    Attr::RunBlocking,
    Attr::Footwork,
    Attr::LegDrive,
    Attr::Pursuit,
    Attr::Blitzing,
    Attr::ThrowOnMove,
];
pub const LIMITED_COUNT: usize = 17;

/// The kinds of technique a knack belongs to (design item 5): one hidden
/// knack each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Coverage,
    Routes,
    Blocking,
    Rushing,
    Passing,
    Tackling,
}

pub const KINDS: [Kind; 6] =
    [Kind::Coverage, Kind::Routes, Kind::Blocking, Kind::Rushing, Kind::Passing, Kind::Tackling];

impl Kind {
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Coverage => "coverage",
            Kind::Routes => "routes",
            Kind::Blocking => "blocking",
            Kind::Rushing => "rushing",
            Kind::Passing => "passing mechanics",
            Kind::Tackling => "tackling",
        }
    }
}

/// A part of the limit's read: a rating's ceiling, or his arm length.
#[derive(Clone, Copy, Debug)]
enum Part {
    A(Attr),
    Arms,
}

/// What a limited skill's knack kind and read are (design item 6, everyday
/// item 9). Hips are flexibility, burst acceleration, hands catching.
fn spec(a: Attr) -> (Kind, &'static [Part], &'static [Part]) {
    use Attr as A;
    use Part::{A as R, Arms};
    const COVER: &[Part] = &[R(A::Speed), R(A::Acceleration), R(A::Agility), R(A::Flexibility)];
    const COVER_M: &[Part] = &[R(A::PlayRecognition)];
    const TACKLE: &[Part] = &[R(A::Strength), R(A::Speed), R(A::Balance)];
    const TACKLE_M: &[Part] = &[R(A::Toughness), R(A::Physicality)];
    const CATCH: &[Part] = &[R(A::Strength), R(A::BodyControl), R(A::Jumping)];
    const CATCH_M: &[Part] = &[R(A::Catching), R(A::Competitiveness)];
    const ROUTE: &[Part] = &[R(A::Agility), R(A::Acceleration), R(A::Balance)];
    const IQ: &[Part] = &[R(A::FootballIq)];
    const RUSH: &[Part] = &[R(A::Acceleration), R(A::Strength), R(A::Flexibility), R(A::Balance), Arms];
    const RUSH_M: &[Part] = &[R(A::Motor), R(A::Competitiveness)];
    const BLOCK: &[Part] = &[R(A::Strength), R(A::Balance), R(A::Agility), Arms];
    const TOUGH: &[Part] = &[R(A::Toughness)];
    const PASS: &[Part] = &[R(A::ArmStrength), R(A::Balance), R(A::Agility)];
    const POISE: &[Part] = &[R(A::Composure)];
    const DRIVE: &[Part] = &[R(A::Strength), R(A::Balance)];
    const CHASE: &[Part] = &[R(A::Speed), R(A::Acceleration)];
    const MOTOR: &[Part] = &[R(A::Motor)];
    const BLITZ: &[Part] = &[R(A::Acceleration), R(A::Agility)];
    const MOVE: &[Part] = &[R(A::Agility), R(A::Balance)];
    match a {
        A::ManCoverage | A::ZoneCoverage | A::Press => (Kind::Coverage, COVER, COVER_M),
        A::Tackling | A::BlockShedding => (Kind::Tackling, TACKLE, TACKLE_M),
        A::ContestedCatch => (Kind::Routes, CATCH, CATCH_M),
        A::RouteRunning | A::Release => (Kind::Routes, ROUTE, IQ),
        A::PowerRush | A::FinesseRush => (Kind::Rushing, RUSH, RUSH_M),
        A::PassBlocking | A::RunBlocking => (Kind::Blocking, BLOCK, TOUGH),
        A::Footwork => (Kind::Passing, PASS, POISE),
        A::LegDrive => (Kind::Tackling, DRIVE, TOUGH),
        A::Pursuit => (Kind::Tackling, CHASE, MOTOR),
        A::Blitzing => (Kind::Rushing, BLITZ, IQ),
        _ => (Kind::Passing, MOVE, POISE), // ThrowOnMove
    }
}

/// The limited skill's place in `LIMITED`.
pub fn limited_index(a: Attr) -> Option<usize> {
    LIMITED.iter().position(|&x| x == a)
}

/// The knack kind of a limited skill.
pub fn kind_of(a: Attr) -> Option<Kind> {
    limited_index(a).map(|_| spec(a).0)
}

/// The most of the read's six parts any skill has.
const PARTS: usize = 7;

fn parts(a: Attr) -> (Vec<(Part, f64)>, usize) {
    let (_, body, makeup) = spec(a);
    // About a third of the read is his makeup (design item 6).
    let mut v: Vec<(Part, f64)> = body.iter().map(|&p| (p, (2.0 / 3.0) / body.len() as f64)).collect();
    v.extend(makeup.iter().map(|&p| (p, (1.0 / 3.0) / makeup.len() as f64)));
    let n = v.len();
    (v, n)
}

fn part_value(c: &Attributes, arm: f64, p: Part) -> f64 {
    match p {
        Part::A(a) => c.get(a),
        Part::Arms => arm,
    }
}

/// Quantiles of the rank map: every 2.5%.
const Q: usize = 41;

/// One limited skill's rank map at one position: the read's parts' mean and
/// spread there, and the tools score's quantiles against the skill
/// ceiling's.
struct RankMap {
    mean: [f64; PARTS],
    sd: [f64; PARTS],
    t: [f64; Q],
    c: [f64; Q],
}

/// Boys drawn for the rank maps, on a fixed stream of the generator itself
/// (`athlete::create`), so the maps follow any change to how players are
/// made and every world reads the same maps.
const FIT_SAMPLE: u32 = 60_000;

fn maps() -> &'static Vec<Option<RankMap>> {
    static MAPS: OnceLock<Vec<Option<RankMap>>> = OnceLock::new();
    MAPS.get_or_init(|| {
        off_pool(|| {
            use rayon::prelude::*;
            let drawn: Vec<(Position, Attributes, f64)> = (0..FIT_SAMPLE)
                .into_par_iter()
                .map(|i| {
                    let mut rng = Rng::new(0x5EED_11A1u32 ^ i.wrapping_mul(0x9E37_79B1));
                    let talent = clamp(
                        crate::player::talent_from_z(rng.normal(0.0, 1.0)),
                        20.0,
                        crate::attributes::RATING_BOUND,
                    );
                    let (ceilings, body, natural, _) = crate::athlete::create(&mut rng, talent);
                    (natural, ceilings, body.arm)
                })
                .collect();
            let mut out = Vec::with_capacity(LIMITED_COUNT * POSITION_COUNT);
            for &a in LIMITED.iter() {
                let (ps, n) = parts(a);
                for pos in Position::ALL {
                    if weight(pos, a) <= 0.0 {
                        out.push(None);
                        continue;
                    }
                    let group: Vec<&(Position, Attributes, f64)> = drawn.iter().filter(|d| d.0 == pos).collect();
                    if group.len() < 200 {
                        out.push(None);
                        continue;
                    }
                    let mut mean = [0.0; PARTS];
                    let mut sd = [1.0; PARTS];
                    for k in 0..n {
                        let xs: Vec<f64> = group.iter().map(|d| part_value(&d.1, d.2, ps[k].0)).collect();
                        let m = xs.iter().sum::<f64>() / xs.len() as f64;
                        let v = xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / xs.len() as f64;
                        mean[k] = m;
                        sd[k] = v.sqrt().max(1e-6);
                    }
                    let score = |c: &Attributes, arm: f64| {
                        (0..n).map(|k| ps[k].1 * (part_value(c, arm, ps[k].0) - mean[k]) / sd[k]).sum::<f64>()
                    };
                    let mut ts: Vec<f64> = group.iter().map(|d| score(&d.1, d.2)).collect();
                    let mut cs: Vec<f64> = group.iter().map(|d| d.1.get(a)).collect();
                    ts.sort_by(|x, y| x.total_cmp(y));
                    cs.sort_by(|x, y| x.total_cmp(y));
                    let at = |v: &[f64], q: f64| v[((q * (v.len() - 1) as f64).round() as usize).min(v.len() - 1)];
                    let mut t = [0.0; Q];
                    let mut c = [0.0; Q];
                    for j in 0..Q {
                        let q = j as f64 / (Q - 1) as f64;
                        t[j] = at(&ts, q);
                        c[j] = at(&cs, q);
                    }
                    out.push(Some(RankMap { mean, sd, t, c }));
                }
            }
            out
        })
    })
}

/// How the read's parts drift with age for a player who grows like his
/// class: the mean ceiling at each age less the mean at 14, per position.
/// The read takes it off, so his limit moves only with his own growth past
/// his class's (a weight-room responder, a body project, makeup that grows
/// faster than his peers'), his rank (design item 4), and not with the
/// drift every boy has (natural rises, maturing reads). Fitted by growing
/// boys of the generator through `develop_season` from 14 to 30 with the
/// split ceiling off for them (high school to 17, college to 21, then the
/// pros, a coach of 10, rotation snaps), so it follows any change to growth.
struct Drift {
    /// [position][part slot][age − 14]: the attributes, then arm length.
    d: Vec<f32>,
}

const DRIFT_FROM: u32 = 14;
const DRIFT_AGES: usize = 17;
const SLOTS: usize = crate::attributes::Attr::ALL.len() + 1;
const DRIFT_SAMPLE: u32 = 24_000;

fn slot(p: Part) -> usize {
    match p {
        Part::A(a) => a.index(),
        Part::Arms => SLOTS - 1,
    }
}

impl Drift {
    fn at(&self, pos: Position, p: Part, age: u32) -> f64 {
        let i = (age.max(DRIFT_FROM) - DRIFT_FROM).min(DRIFT_AGES as u32 - 1) as usize;
        self.d[(pos.index() * SLOTS + slot(p)) * DRIFT_AGES + i] as f64
    }
}

thread_local! {
    /// Set while the drift is fitted: the split ceiling is off for the
    /// fitting boys (no draw, no closing), so the fit never reads itself.
    static FITTING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn fitting() -> bool {
    FITTING.with(|f| f.get())
}

fn drift() -> &'static Drift {
    static DRIFT: OnceLock<Drift> = OnceLock::new();
    DRIFT.get_or_init(|| {
        off_pool(|| {
            use crate::development::{SeasonContext, develop_season};
            use crate::player::{ProspectOptions, generate_prospect};
            use rayon::prelude::*;
            // Each boy's part values at every age (his natural spot at 14).
            let rows: Vec<(Position, Vec<[f32; SLOTS]>)> = (0..DRIFT_SAMPLE)
                .into_par_iter()
                .map(|i| {
                    FITTING.with(|f| f.set(true));
                    let mut rng = Rng::new(0xD21F_7A5Eu32 ^ i.wrapping_mul(0x9E37_79B1));
                    let mut p = generate_prospect(
                        &mut rng,
                        ProspectOptions { id: Some(0xF000_0000 | i), ..Default::default() },
                    );
                    let read = |p: &Player| {
                        let mut v = [0f32; SLOTS];
                        for a in crate::attributes::Attr::ALL {
                            v[a.index()] = p.ceilings.get(a) as f32;
                        }
                        v[SLOTS - 1] = p.body.arm as f32;
                        v
                    };
                    let mut out = vec![read(&p)];
                    for _ in 1..DRIFT_AGES {
                        let level = match p.age {
                            ..=17 => Level::HS,
                            18..=21 => Level::FBS,
                            _ => Level::NFL,
                        };
                        develop_season(&mut p, &SeasonContext::new(level, 10.0, PlayingTime::Rotation), &mut rng);
                        out.push(read(&p));
                    }
                    FITTING.with(|f| f.set(false));
                    (p.position, out)
                })
                .collect();
            let mut d = vec![0f32; POSITION_COUNT * SLOTS * DRIFT_AGES];
            for pos in Position::ALL {
                let group: Vec<&Vec<[f32; SLOTS]>> = rows.iter().filter(|r| r.0 == pos).map(|r| &r.1).collect();
                if group.len() < 100 {
                    continue;
                }
                let n = group.len() as f64;
                for s in 0..SLOTS {
                    let mean = |t: usize| group.iter().map(|r| r[t][s] as f64).sum::<f64>() / n;
                    let base = mean(0);
                    for t in 0..DRIFT_AGES {
                        d[(pos.index() * SLOTS + s) * DRIFT_AGES + t] = (mean(t) - base) as f32;
                    }
                }
            }
            Drift { d }
        })
    })
}

/// The class's drift in one attribute's ceiling at a position from 14 to
/// an age (trial tooling: `prog_check drift`).
pub fn drift_at(pos: Position, a: Attr, age: u32) -> f64 {
    drift().at(pos, Part::A(a), age)
}

/// Runs a fit on its own thread and pool: a fit starts inside the world's
/// parallel work, and a rayon worker waiting on it could otherwise pick up
/// a job that waits on the same fit.
fn off_pool<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|s| {
        s.spawn(|| rayon::ThreadPoolBuilder::new().build().expect("fit pool").install(f)).join().expect("fit")
    })
}

/// The position whose map a limited skill reads for him: his own when it
/// uses the skill, else the position that leans on it most (a convert
/// keeps his room, design item 17: the read is his body and makeup).
fn map_position(pos: Position, a: Attr) -> Position {
    if weight(pos, a) > 0.0 && maps()[limited_index(a).unwrap() * POSITION_COUNT + pos.index()].is_some() {
        return pos;
    }
    Position::ALL
        .iter()
        .copied()
        .filter(|&p| maps()[limited_index(a).unwrap() * POSITION_COUNT + p.index()].is_some())
        .max_by(|x, y| weight(*x, a).total_cmp(&weight(*y, a)))
        .unwrap_or(pos)
}

/// What players with his tools reach in a limited skill, at a position: his
/// tools score's rank mapped onto the skill's ceilings there. Above the
/// sample's best it carries on at the top tenth's slope, so a freak's body
/// keeps adding room (no cap on freaks).
pub fn mapped(p: &Player, a: Attr) -> f64 {
    mapped_on(p, a, &p.ceilings, p.age)
}

/// `mapped` on a given set of ceilings at a given age (his read when the
/// year opened, for `settle`).
fn mapped_on(p: &Player, a: Attr, ceilings: &Attributes, age: u32) -> f64 {
    let Some(li) = limited_index(a) else { return ceilings.get(a) };
    let pos = map_position(p.position, a);
    let Some(m) = &maps()[li * POSITION_COUNT + pos.index()] else { return ceilings.get(a) };
    let (ps, n) = parts(a);
    let d = drift();
    let s: f64 = (0..n)
        .map(|k| {
            let v = part_value(ceilings, p.body.arm, ps[k].0) - d.at(pos, ps[k].0, age);
            ps[k].1 * (v - m.mean[k]) / m.sd[k]
        })
        .sum();
    if s <= m.t[0] {
        return m.c[0];
    }
    if s >= m.t[Q - 1] {
        let j = Q - 5;
        let slope = (m.c[Q - 1] - m.c[j]) / (m.t[Q - 1] - m.t[j]).max(1e-6);
        return m.c[Q - 1] + (s - m.t[Q - 1]) * slope.max(0.0);
    }
    let j = m.t.partition_point(|&x| x <= s).clamp(1, Q - 1);
    let (t0, t1) = (m.t[j - 1], m.t[j]);
    let w = if t1 > t0 { (s - t0) / (t1 - t0) } else { 0.5 };
    m.c[j - 1] + w * (m.c[j] - m.c[j - 1])
}

/// The margin past the rank map every limit gets (design item 4).
pub const MARGIN: f64 = 4.0;

/// His hidden progression state (about 26 bytes): the knacks, and each
/// limit's own part past the rank map.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Prog {
    /// Whether it was drawn (an older save draws it once, at his age).
    pub drawn: bool,
    /// The technique knacks by `Kind`, then the body knack: standard
    /// normals times 25. Hidden: the game shows only staff words.
    pub knack: [i8; 7],
    /// Each limit's part past the rank map, in half points: the margin,
    /// the gap at 14 when his tools' level sat above his ceiling, and every
    /// natural rise past the limit since (everyday item 2).
    pub over: [u8; LIMITED_COUNT],
    /// His skill project (design items 8-12): 0 none, else its skill's
    /// place in `LIMITED` plus 1.
    #[serde(default)]
    pub project: u8,
    /// Years on the current skill project, and the ceiling it has opened
    /// so far (half points).
    #[serde(default)]
    pub project_years: u8,
    #[serde(default)]
    pub project_lift: u8,
    /// Clicks over his career (design item 10).
    #[serde(default)]
    pub clicks: u8,
    /// Your staff set the project (an AI staff never ends it).
    #[serde(default)]
    pub yours: bool,
    /// The trims still to land, by place in `LIMITED` (everyday item 11).
    #[serde(default)]
    pub pending: u32,
    /// His body project (design items 1, 2 and 7): 0 none, 1 strength, 2
    /// speed (`BodyKind`).
    #[serde(default)]
    pub body: u8,
    /// Years on the body project, and what it has added to each of its
    /// ratings so far (half points, in `BodyKind::ratings` order).
    #[serde(default)]
    pub body_years: u8,
    #[serde(default)]
    pub body_used: [u8; 5],
    /// Pounds the strength project has put on him.
    #[serde(default)]
    pub body_lbs: u8,
    /// Your strength staff set it (an AI staff never ends it).
    #[serde(default)]
    pub body_yours: bool,
    /// The day his skill project started (`Date` days, 0 not known: an AI
    /// staff's, or an older save's). Your position coach's read of how he
    /// takes to it sharpens by the weeks since (`project_weeks`). It sits in
    /// the struct's padding, so the store stays the same size.
    #[serde(default)]
    pub project_on: u16,
    /// The staff his projects belong to (`staff_code`): a project is a
    /// staff's own plan, so it ends when he moves to another (0: an older
    /// save, taken as the staff he is with).
    #[serde(default)]
    pub staff: u32,
}

/// Which staff a project belongs to, by level (`staff_code`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaffAt {
    HighSchool,
    College,
    Nfl,
}

/// A staff's code for `Prog::staff`: its level and its index (a high
/// school, a program or a club). Never 0.
pub fn staff_code(at: StaffAt, index: usize) -> u32 {
    let level = match at {
        StaffAt::HighSchool => 1u32,
        StaffAt::College => 2,
        StaffAt::Nfl => 3,
    };
    level << 24 | (index as u32 + 1) & 0x00FF_FFFF
}

impl Prog {
    /// Whether he carries a project another staff set: he has moved since
    /// (a signing, a transfer, the draft, a trade or a cut), and it is no
    /// longer anyone's plan.
    pub fn left_behind(&self, staff: u32) -> bool {
        self.staff != 0 && self.staff != staff && (self.project > 0 || self.body > 0)
    }

    /// The skill his project works, if any.
    pub fn project(&self) -> Option<Attr> {
        (self.project > 0).then(|| LIMITED[(self.project as usize - 1).min(LIMITED_COUNT - 1)])
    }

    /// His body project, if any.
    pub fn body_project(&self) -> Option<BodyKind> {
        match self.body {
            1 => Some(BodyKind::Strength),
            2 => Some(BodyKind::Speed),
            _ => None,
        }
    }
}

/// Turns the split ceiling off (trial and debug tooling: the A/B
/// measurements of the build). On by default.
static OFF: AtomicBool = AtomicBool::new(false);

pub fn set_enabled(on: bool) {
    OFF.store(!on, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    !OFF.load(Ordering::Relaxed)
}

/// The parts that are on (trial tooling, `prog_check`): everyday closing,
/// breakouts, projects, trims. All on by default.
static PARTS_OFF: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
pub const PART_EVERYDAY: u8 = 1;
pub const PART_BREAKOUT: u8 = 2;
pub const PART_PROJECTS: u8 = 4;
pub const PART_TRIMS: u8 = 8;
pub const PART_BODY: u8 = 16;

pub fn set_parts(on: u8) {
    PARTS_OFF.store(!on & 31, Ordering::Relaxed);
}

fn part(p: u8) -> bool {
    !fitting() && enabled() && PARTS_OFF.load(Ordering::Relaxed) & p == 0
}

fn stream(id: u32, salt: u32) -> Rng {
    Rng::new(id.wrapping_mul(0x2C1B_3C6D) ^ salt.wrapping_mul(0x297A_2D39) ^ 0x7A6E_5103)
}

fn to_i8(z: f64) -> i8 {
    (z * 25.0).round().clamp(-127.0, 127.0) as i8
}

/// A technique knack's z (design item 5).
pub fn knack(p: &Player, k: Kind) -> f64 {
    p.prog.knack[k.index()] as f64 / 25.0
}

/// His body knack's z (design item 1): how his body takes to a program.
pub fn body_knack(p: &Player) -> f64 {
    p.prog.knack[6] as f64 / 25.0
}

/// How fast a knack closes room, 0-1 (design item 5): Φ(1.4 × (z − 0.3)).
/// About 1 in 5 has the knack (z past 0.84, pace past 0.77), half are
/// middling and about 1 in 3 have none (z under -0.43, pace under 0.15):
/// `tier`.
pub fn tier(z: f64) -> u8 {
    if z >= 0.84 {
        0
    } else if z >= -0.43 {
        1
    } else {
        2
    }
}

/// The knack's pace.
pub fn pace(z: f64) -> f64 {
    crate::athlete::phi(1.4 * (z - 0.3))
}

/// Draws his knacks and sets each limit if he has none yet (at creation,
/// or once for a player from an older save, from where he is now). On his
/// own keyed stream: no other draw moves.
pub fn ensure(p: &mut Player) {
    if p.prog.drawn || fitting() {
        return;
    }
    let mut r = stream(p.id, 0x4B4E);
    // About a third of a technique knack is coachability (item 5).
    let cz = clamp((p.personality.coachability - 10.0) / 3.5, -3.0, 3.0);
    let own = (1.0f64 - 0.35 * 0.35).sqrt();
    for k in 0..6 {
        p.prog.knack[k] = to_i8(0.35 * cz + own * r.normal(0.0, 1.0));
    }
    p.prog.knack[6] = to_i8(r.normal(0.0, 1.0));
    for (li, &a) in LIMITED.iter().enumerate() {
        let m = mapped(p, a);
        let over = MARGIN + (m - p.ceilings.get(a)).max(0.0);
        p.prog.over[li] = (over * 2.0).ceil().clamp(0.0, 255.0) as u8;
    }
    p.prog.drawn = true;
}

/// His limit in a skill: the rank map plus its own part, never under his
/// ceiling. None for a skill with no limit.
pub fn limit(p: &Player, a: Attr) -> Option<f64> {
    let li = limited_index(a)?;
    if !p.prog.drawn {
        return Some(p.ceilings.get(a));
    }
    Some(limit_of(mapped(p, a), p.prog.over[li] as f64 / 2.0).max(p.ceilings.get(a)))
}

/// A limit from its rank map and its own part: the part counts as a rise
/// from the map, so past the knee it thins like every other rise (physical
/// study item 17). The map reads strength's fat top (rushing, blocking,
/// tackling), and without this a strong man's limit ran to 150.
fn limit_of(mapped: f64, over: f64) -> f64 {
    crate::athlete::top_up(mapped, mapped + over)
}

/// The room between his limit and his everyday ceiling (0 for a skill with
/// no limit).
pub fn room(p: &Player, a: Attr) -> f64 {
    limit(p, a).map_or(0.0, |l| (l - p.ceilings.get(a)).max(0.0))
}

/// The share of a room a year of everyday coaching closes (everyday item
/// 1): a third of a project year's 0.30.
pub const EVERYDAY: f64 = 0.10;

/// Where closing starts to slow (see `near_top`): 55. A trial lever for
/// `prog_check` (`AFM_SLOW_FROM`).
static SLOW_FROM: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0x404B800000000000);

pub fn set_slow_from(c: f64) {
    SLOW_FROM.store(c.to_bits(), Ordering::Relaxed);
}

/// The span of the slowing past `SLOW_FROM`: 5 (0: no slowing). A trial
/// lever for `prog_check` (`AFM_TOP_SLOW`).
static TOP_SLOW: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0x4014000000000000);

pub fn set_top_slow(span: f64) {
    TOP_SLOW.store(span.to_bits(), Ordering::Relaxed);
}

/// How much of a year's closing a ceiling at `c` still gets (Cain's card
/// 2026-11-17, "slow closing near the top"): the nearer the top, the harder
/// each point, as in real skill learning. 1 up to `SLOW_FROM`, then 1 / (1 +
/// past / span): a 60 closes at half pace, a 70 at a quarter, an 80 at a
/// sixth. Never zero, so no one is capped. Everyday closing, breakouts and
/// project years (clicks too) all take it; the weight room and natural
/// rises don't. 55 and 5 are Claude's sizes from the class sim, proposed,
/// not confirmed.
pub fn near_top(c: f64) -> f64 {
    let span = f64::from_bits(TOP_SLOW.load(Ordering::Relaxed));
    if span <= 0.0 {
        return 1.0;
    }
    let from = f64::from_bits(SLOW_FROM.load(Ordering::Relaxed));
    1.0 / (1.0 + (c - from).max(0.0) / span)
}

/// How much the staff's coaching gets out of the room (design item 9):
/// 0.5 + coaching / 20, so a 10 is 1.0 and a 20 is 1.5.
pub fn coach_mult(coaching: f64) -> f64 {
    clamp(0.5 + coaching / 20.0, 0.5, 1.5)
}

/// His snaps' part in everyday closing (everyday item 1): a starter 1,
/// rotation 0.8, a redshirt 0.6, the rest 0.5; a year away from the staff
/// (suspended or dismissed) closes nothing.
pub fn snaps_mult(pt: PlayingTime, trouble: bool) -> f64 {
    if trouble {
        return 0.0;
    }
    match pt {
        PlayingTime::Starter => 1.0,
        PlayingTime::Rotation => 0.8,
        PlayingTime::Redshirt => 0.6,
        _ => 0.5,
    }
}

/// The strength of a year's closing by his stage (design items 11 and 16):
/// full through high school and every college season whatever his age;
/// in the pros half in his first two pro seasons, a fifth in seasons three
/// and four, none after (the pros' everyday part is proposed, not
/// confirmed).
pub fn stage_mult(p: &Player, level: Level) -> f64 {
    if level != Level::NFL {
        return 1.0;
    }
    let seasons = p.history.iter().filter(|h| h.level == Level::NFL).count() + 1;
    match seasons {
        1 | 2 => 0.5,
        3 | 4 => 0.2,
        _ => 0.0,
    }
}

/// Whether his position works a limited skill (only those grow and close).
fn works(p: &Player, a: Attr, hs: bool) -> bool {
    weight(p.position, a) > 0.0 && !crate::development::unrelated(p.position, a, hs)
}

/// The year's everyday closing (everyday item 1): each limited skill his
/// position works closes room × 0.10 × knack pace × coaching × snaps,
/// times his stage. Returns the ceiling points it added.
pub(crate) fn everyday(p: &mut Player, coaching: f64, level: Level, pt: PlayingTime, trouble: bool) -> f64 {
    ensure(p);
    land_trims(p);
    if !part(PART_EVERYDAY) {
        return 0.0;
    }
    let hs = level == Level::HS;
    let base = EVERYDAY * coach_mult(coaching) * snaps_mult(pt, trouble) * stage_mult(p, level);
    if base <= 0.0 {
        return 0.0;
    }
    let mut added = 0.0;
    for &a in LIMITED.iter() {
        if !works(p, a, hs) {
            continue;
        }
        let room = room(p, a);
        if room <= 0.0 {
            continue;
        }
        let k = kind_of(a).unwrap();
        let close = room * base * pace(knack(p, k)) * near_top(p.ceilings.get(a));
        p.ceilings.add(a, close);
        added += close;
    }
    added
}

/// The normal weight room inside the leashes (everyday item 6): how far
/// his body knack moves each physical ceiling from the typical man's path
/// over his college years, in rating points, and the most a big responder
/// gets (design item 2's leashes: strength about 7 for a responder and 14
/// for a big one, speed 1, burst 1.5, agility and hips 2, balance 3).
/// Each half of his strength has the whole strength leash.
pub const WEIGHT_ROOM: [(Attr, f64, f64); 8] = [
    (Attr::Strength, 7.0, 14.0),
    (Attr::LowerBody, 7.0, 14.0),
    (Attr::Speed, 1.0, 1.0),
    (Attr::Acceleration, 1.5, 1.5),
    (Attr::Jumping, 1.5, 1.5),
    (Attr::Agility, 2.0, 2.0),
    (Attr::Flexibility, 2.0, 2.0),
    (Attr::Balance, 3.0, 3.0),
];

/// How a knack z reads as a weight-room response: 0.8 a standard
/// deviation, a non-responder at -1 (the typical gain less a whole leash),
/// the typical man about 0. Mean 0 over the class, so the class's average
/// path (already real) holds. The top is skewed (physical study item 3):
/// real training response runs right-skewed, a few men gaining two or
/// three times the typical man's strength, and those men make the 40-rep
/// benchers. `RESPONSE_SKEW` a z squared past the typical man, less its
/// mean (half of it).
pub fn response(z: f64) -> f64 {
    0.8 * z + RESPONSE_SKEW * (z.max(0.0).powi(2) - 0.5)
}

/// The weight room response's skew (see `response`). Proposed, not
/// confirmed.
pub const RESPONSE_SKEW: f64 = 0.3;

/// The share of the college weight room's work a year at this age does:
/// front-loaded, most of it in the first two years (Jacobson et al. 2013),
/// burst only then (design item 2). Proposed, not confirmed.
pub fn weight_room_share(age: u32, a: Attr) -> f64 {
    let burst = matches!(a, Attr::Acceleration | Attr::Jumping);
    match age {
        ..=17 => 0.0,
        18 => {
            if burst {
                0.55
            } else {
                0.30
            }
        }
        19 => {
            if burst {
                0.45
            } else {
                0.25
            }
        }
        _ if burst => 0.0,
        20 => 0.20,
        21 => 0.15,
        22 => 0.10,
        _ => 0.0,
    }
}

/// The year's normal weight room (everyday item 6): every college and pro
/// player's strength program moves his physical ceilings by his body
/// knack, a big responder up toward his leash and a non-responder down,
/// never under his rating today (a ceiling at his rating just stops his
/// gains). High school isn't a college program. Returns the points moved,
/// up and down summed by size.
pub(crate) fn weight_room(p: &mut Player, level: Level, trouble: bool) -> f64 {
    if level == Level::HS || trouble || !part(PART_BODY) {
        return 0.0;
    }
    ensure(p);
    let z = body_knack(p);
    let mut moved = 0.0;
    for &(a, leash, most) in WEIGHT_ROOM.iter() {
        let share = weight_room_share(p.age, a);
        if share <= 0.0 {
            continue;
        }
        // The bench's fat top (item 3): upper-body strength takes the
        // skewed response and a big responder is never held to the leash's
        // most. The rest keep the plain response inside their leash (lower
        // body too: a skewed squat ran verticals to 56 inches).
        let (r, top) = if a == Attr::Strength { (response(z), f64::INFINITY) } else { (0.8 * z, most / leash) };
        let d = clamp(r, -1.0, top) * leash * share;
        let (now, c) = (p.attributes.get(a), p.ceilings.get(a));
        let to = crate::athlete::top_up_attr(a, c, c + d).max(now);
        p.ceilings.set(a, to);
        moved += (to - c).abs();
    }
    moved
}

/// A click's size on the ceiling (design item 10): 6 to 10 points.
pub const CLICK: (f64, f64) = (6.0, 10.0);

/// A breakout year closes room at a click's size (everyday item 5): the
/// limited skill his position works with the most room (by what the
/// position asks of it) clicks, U(6, 10) on its ceiling, never past his
/// limit, so a raw kid's breakout is bigger than a polished kid's. `mean`
/// (results replacing the dice, item 14): a tested skill takes the
/// chance-weighted mean instead, as the breakout's own lift does. Returns
/// the ceiling points added.
pub(crate) fn breakout(p: &mut Player, level: Level, hit: bool, mean: f64, tested: u128) -> f64 {
    if !part(PART_BREAKOUT) || (!hit && mean <= 0.0) {
        return 0.0;
    }
    ensure(p);
    let hs = level == Level::HS;
    let mut r = stream(p.id, 0xB0 ^ p.age.wrapping_mul(0x51));
    let Some(a) = LIMITED
        .iter()
        .copied()
        .filter(|&a| works(p, a, hs))
        .map(|a| (a, room(p, a) * weight(p.position, a)))
        .filter(|x| x.1 > 0.0)
        .max_by(|x, y| x.1.total_cmp(&y.1))
        .map(|x| x.0)
    else {
        return 0.0;
    };
    let size = r.range(CLICK.0, CLICK.1);
    let add = if crate::results::is_tested(tested, a) {
        mean * (CLICK.0 + CLICK.1) / 2.0
    } else if hit {
        size
    } else {
        0.0
    };
    let add = (add * near_top(p.ceilings.get(a))).min(room(p, a));
    if add > 0.0 {
        p.ceilings.add(a, add);
        return add;
    }
    0.0
}

/// After the year: a natural rise past the limit lifts it (everyday item
/// 2), so neglect later reopens room he had (item 3). Only his ceiling's
/// own rise counts (`before` is his ceilings when the year opened), and
/// only past the higher of his limit when the year opened and now: a limit
/// that falls with his body is left to fall (item 4), `limit` keeps it at
/// his ceiling, and a one-year dip in his body read never ratchets it up.
/// Half points round to the nearest, so no rounding adds room either.
pub(crate) fn settle(p: &mut Player, before: &Attributes) {
    if !p.prog.drawn {
        return;
    }
    let was = p.age.saturating_sub(1);
    for (li, &a) in LIMITED.iter().enumerate() {
        let c = p.ceilings.get(a);
        let rose = c - before.get(a);
        let over = p.prog.over[li] as f64 / 2.0;
        let m = mapped(p, a).max(mapped_on(p, a, before, was));
        let l = limit_of(m, over);
        if c > l && rose > 0.0 {
            // The part that puts the limit at his new ceiling (or one
            // year's rise past the old limit), on the draw's scale.
            let to = c.min(l + rose);
            let raw = crate::athlete::raw_top;
            let need = (raw(to) - raw(m)).max(over);
            p.prog.over[li] = (need * 2.0).round().clamp(0.0, 255.0) as u8;
        }
    }
}

// ---------------------------------------------------------------------------
// Skill projects (design items 8-17)
// ---------------------------------------------------------------------------

/// A project room: the position rooms that hold skill projects, with
/// Cain's counts (design item 8): QB 1, RB 1, WR 2, TE 2, O-line 2,
/// D-line 2, LB 1, CB and nickel 2, safeties 1; kickers, punters and
/// snappers none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Room {
    Qb,
    Rb,
    Wr,
    Te,
    Ol,
    Dl,
    Lb,
    Cb,
    S,
}

impl Room {
    pub fn of(pos: Position) -> Option<Room> {
        Some(match pos {
            Position::QB => Room::Qb,
            Position::RB => Room::Rb,
            Position::WR => Room::Wr,
            Position::TE => Room::Te,
            Position::OT | Position::IOL => Room::Ol,
            Position::EDGE | Position::DT => Room::Dl,
            Position::LB => Room::Lb,
            Position::CB => Room::Cb,
            Position::S => Room::S,
            _ => return None,
        })
    }

    /// Projects the room's position coach can run at once.
    pub fn slots(self) -> usize {
        match self {
            Room::Qb | Room::Rb | Room::Lb | Room::S => 1,
            _ => 2,
        }
    }
}

/// The share of the room a project year closes with the knack and a good
/// coach (design item 9): room × 0.30 × pace × coaching × U(0.7, 1.3).
pub const PROJECT: f64 = 0.30;

/// A click's chance a project year (design item 10): 0.25 × pace.
pub const CLICK_CHANCE: f64 = 0.25;

/// The share of the year's growth his other key technique skills give up
/// to a project (design item 12).
pub const PROJECT_COST: f64 = 0.25;

/// The skills that pay for a project: his position's technique skills,
/// never the project's own. As a mask over `Attr::index`.
pub fn cost_mask(pos: Position, project: Attr) -> u128 {
    let mut m = 0u128;
    for a in Attr::ALL {
        if a != project && a.profile().group == crate::attributes::AttributeGroup::Technical && weight(pos, a) > 0.0 {
            m |= 1u128 << a.index();
        }
    }
    m
}

/// What a project year did (for the world log and the clues).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ProjectYear {
    pub skill: Option<Attr>,
    pub lift: f64,
    pub click: f64,
}

/// A project year at his checkpoint (design items 9-11, 16): his project's
/// ceiling closes room by his knack, his coach and his stage, and with luck
/// it clicks (6 to 10 on the ceiling, half of it on the field within
/// weeks). A project on a skill his position no longer works is dropped.
/// The growth toward the new ceiling runs at the 60-minute focus's pace
/// (`GrowthCtx`), and his other technique skills pay `PROJECT_COST`.
pub(crate) fn project_year(p: &mut Player, coaching: f64, level: Level, trouble: bool) -> ProjectYear {
    let Some(a) = p.prog.project() else { return ProjectYear::default() };
    if !part(PART_PROJECTS) || !works(p, a, level == Level::HS) {
        end_project(p);
        return ProjectYear::default();
    }
    ensure(p);
    let mut r = stream(p.id, 0x9A07 ^ p.age.wrapping_mul(0x1F3));
    let stage = stage_mult(p, level) * if trouble { 0.0 } else { 1.0 };
    let pace = pace(knack(p, kind_of(a).unwrap()));
    let room = room(p, a);
    let mut lift = room * PROJECT * pace * coach_mult(coaching) * r.range(0.7, 1.3) * stage;
    let mut click = 0.0;
    if r.chance(CLICK_CHANCE * pace * stage) {
        click = r.range(CLICK.0, CLICK.1);
        lift += click;
    }
    let slow = near_top(p.ceilings.get(a));
    let (lift, click) = (lift * slow, click * slow);
    let lift = lift.min(room).max(0.0);
    let click = click.min(lift);
    p.ceilings.add(a, lift);
    if click > 0.0 {
        // The light comes on: half of it shows on the field within weeks.
        let c = p.ceilings.get(a);
        let v = (p.attributes.get(a) + 0.5 * click).min(c);
        p.attributes.set(a, v);
        p.prog.clicks = p.prog.clicks.saturating_add(1);
    }
    p.prog.project_years = p.prog.project_years.saturating_add(1);
    p.prog.project_lift = (p.prog.project_lift as f64 + lift * 2.0).round().clamp(0.0, 255.0) as u8;
    ProjectYear { skill: Some(a), lift, click }
}

/// Starts a skill project (you, or an AI staff's pick). Only on a limited
/// skill his position works.
pub fn start_project(p: &mut Player, a: Attr, yours: bool) -> bool {
    let Some(li) = limited_index(a) else { return false };
    if weight(p.position, a) <= 0.0 {
        return false;
    }
    ensure(p);
    if p.prog.project() != Some(a) {
        p.prog.project_years = 0;
        p.prog.project_lift = 0;
        p.prog.project_on = 0;
    }
    p.prog.project = li as u8 + 1;
    p.prog.yours = yours;
    true
}

pub fn end_project(p: &mut Player) {
    p.prog.project = 0;
    p.prog.project_years = 0;
    p.prog.project_lift = 0;
    p.prog.yours = false;
    p.prog.project_on = 0;
}

/// How an AI staff runs projects (design item 13): developers use their
/// slots most years and bet bolder, balanced staffs about half, win-now
/// staffs rarely and only on kids already close. 2.0 to 0.25.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lean(pub f64);

impl Lean {
    pub const DEVELOPER: Lean = Lean(2.0);
    pub const BALANCED: Lean = Lean(1.0);
    pub const WIN_NOW: Lean = Lean(0.25);

    /// The share of a room's open slots it fills a year.
    pub fn fill(self) -> f64 {
        clamp(0.45 * self.0, 0.05, 0.95)
    }

    /// A coach's own lean, from his id (3 in 10 developers, 4 balanced, 3
    /// win-now; proposed, not confirmed: the shares of the class-sim
    /// prototype), moved by his seat: `security` -1 (a hot seat) to 1 (a
    /// new long deal) halves or doubles it.
    pub fn of_coach(id: usize, security: f64) -> Lean {
        let u = stream(id as u32, 0x1EA4).next();
        let own = if u < 0.3 {
            2.0
        } else if u < 0.7 {
            1.0
        } else {
            0.25
        };
        Lean(clamp(own * 2f64.powf(clamp(security, -1.0, 1.0)), 0.125, 4.0))
    }
}

/// A staff's read of what a bet on him in a skill is worth (design items
/// 13 and 15): the room it sees times the knack it reads, from his tools
/// and the clues, never the true values. `sharp` 0-1 is how well the staff
/// reads the knack clues (an AI staff a bit less than yours, so it picks
/// more on tools); `eye` 1-20 its evaluators.
pub fn staff_read(p: &Player, a: Attr, sharp: f64, eye: f64, r: &mut Rng) -> f64 {
    let (room, seen) = staff_view(p, a, sharp, eye, r);
    room.max(0.0) * pace(seen)
}

/// The two halves of a staff's read (`staff_read`): the room it sees (it
/// can be under 0) and the knack z it reads, with the same draws in the
/// same order.
pub fn staff_view(p: &Player, a: Attr, sharp: f64, eye: f64, r: &mut Rng) -> (f64, f64) {
    let room = room(p, a) + r.normal(0.0, room_noise(eye));
    let cz = clamp((p.personality.coachability - 10.0) / 3.5, -3.0, 3.0);
    let k = knack(p, kind_of(a).unwrap());
    // Before any work on it, coachability is the clue he gives; a sharper
    // staff reads more of the rest.
    let seen = 0.35 * cz + sharp * 0.5 * (k - 0.35 * cz) + r.normal(0.0, 0.6);
    (room, seen)
}

/// The spread of a staff's read of his room, by its eye (1-20).
pub fn room_noise(eye: f64) -> f64 {
    clamp(9.0 - 0.3 * eye, 2.0, 9.0)
}

/// The skills a staff weighs a project on for him: the limited skills his
/// position leans on at least half as much as its most-used one (the pool
/// `best_bet` picks from), most-used first.
pub fn project_skills(p: &Player) -> Vec<Attr> {
    let top = LIMITED.iter().map(|&a| weight(p.position, a)).fold(0.0, f64::max);
    if top <= 0.0 {
        return Vec::new();
    }
    let mut v: Vec<Attr> = LIMITED.iter().copied().filter(|&a| weight(p.position, a) >= 0.5 * top).collect();
    v.sort_by(|x, y| weight(p.position, *y).total_cmp(&weight(p.position, *x)));
    v
}

/// What a read of a project stands on: the eye reading him (1-20), his
/// teacher's coaching (1-20), his level, the years it runs, and the
/// staff's read of the rating today.
#[derive(Clone, Copy, Debug)]
pub struct ReadOn {
    pub eye: f64,
    pub coaching: f64,
    pub level: Level,
    pub years: u32,
    pub seen_top: f64,
    /// The position coach's read of his knack from weeks on a project of
    /// the same kind (`project_clue`), in place of the read before any work.
    pub knack_seen: Option<f64>,
}

/// Your staff's read of a skill project for the screens: what it would
/// open on that skill over `years`, low and high, and the knack z it
/// reads. It is built on the staff's own view (`staff_view` at your
/// sharpness), never the true room or knack: the room it sees one spread
/// either way, the knack 0.75 either way, each project year closing
/// `PROJECT` of what's left by that knack, the coach and his stage, plus
/// a click's mean. `seen_top` is the staff's read of the rating today, so
/// the slowing near the top reads its own number, not his ceiling. The
/// spreads are Claude's, proposed, not confirmed.
pub fn your_project_read(p: &Player, a: Attr, on: ReadOn, r: &mut Rng) -> (f64, f64, f64) {
    let ReadOn { eye, coaching, level, years, seen_top, knack_seen } = on;
    let (room, seen) = staff_view(p, a, YOUR_SHARP, eye, r);
    // Once he has worked at that kind of technique, the coach reads how he
    // took to it (design item 3), not the clue he gave before.
    let seen = knack_seen.unwrap_or(seen);
    let sd = room_noise(eye);
    let pro_seasons = p.history.iter().filter(|h| h.level == Level::NFL).count();
    let stage = |k: u32| -> f64 {
        if level != Level::NFL {
            return 1.0;
        }
        match pro_seasons + 1 + k as usize {
            1 | 2 => 0.5,
            3 | 4 => 0.2,
            _ => 0.0,
        }
    };
    let slow = near_top(seen_top);
    let run = |room: f64, z: f64| -> f64 {
        let pace = pace(z);
        let mut left = room.max(0.0);
        let mut total = 0.0;
        for k in 0..years {
            let s = stage(k);
            let mut lift = left * PROJECT * pace * coach_mult(coaching) * s;
            lift += CLICK_CHANCE * pace * s * 0.5 * (CLICK.0 + CLICK.1);
            let lift = (lift * slow).min(left);
            left -= lift;
            total += lift;
        }
        total
    };
    (run(room - sd, seen - 0.75), run(room + sd, seen + 0.75), seen)
}

/// How sharply AI staffs read the knack clues against your staff (design
/// item 15: a bit less sharply).
pub const AI_SHARP: f64 = 0.6;
pub const YOUR_SHARP: f64 = 1.0;

/// The skill a staff would bet on for him and what it reads the bet as
/// worth: his best by the read, weighted by what his position asks of it.
pub fn best_bet(p: &Player, sharp: f64, eye: f64, r: &mut Rng) -> Option<(Attr, f64)> {
    let top = LIMITED.iter().map(|&a| weight(p.position, a)).fold(0.0, f64::max);
    if top <= 0.0 {
        return None;
    }
    LIMITED
        .iter()
        .filter(|&&a| weight(p.position, a) >= 0.5 * top)
        .map(|&a| (a, staff_read(p, a, sharp, eye, r) * weight(p.position, a) / top))
        .max_by(|x, y| x.1.total_cmp(&y.1))
}

/// The least a staff reads a bet as worth before it runs one (ceiling
/// points in reach, after the knack it reads).
pub const BET_FLOOR: f64 = 3.0;

/// Whether an AI staff keeps a project going after this year (design item
/// 15): it ends one that opened under a point and a half a year once it has
/// had two years with it, and one that has used its room; your own staff's
/// read would see a dead end a year sooner.
pub fn ai_keeps(p: &Player) -> bool {
    let Some(a) = p.prog.project() else { return false };
    if room(p, a) < 1.0 {
        return false;
    }
    let years = p.prog.project_years as f64;
    !(years >= 2.0 && (p.prog.project_lift as f64 / 2.0) / years < 1.5)
}

/// Whether your own staff sees a dead end (design item 15: a year sooner
/// than an AI staff): a project that opened under a point and a half a
/// year once it has had a year. Your staff sees it on the field, in how
/// his rating climbs; the screens and the staff's note read the same rule.
pub fn you_see_dead_end(p: &Player) -> bool {
    if p.prog.project().is_none() {
        return false;
    }
    let years = p.prog.project_years as f64;
    years >= 1.0 && (p.prog.project_lift as f64 / 2.0) / years < DEAD_END
}

/// The least a project opens a year before a staff calls it a dead end.
pub const DEAD_END: f64 = 1.5;

// ---------------------------------------------------------------------------
// The room's one-on-one time (design item 8)
// ---------------------------------------------------------------------------

/// The minutes a week a position coach gives each man one-on-one in the
/// usual week (the focus's usual 30, `development::USUAL_FOCUS_MINUTES`). A
/// project takes 60 (design item 8): 30 more than his share.
pub const ONE_ON_ONE_MINUTES: f64 = 30.0;

/// The share of what a position coach's teaching gives a man that comes
/// through his one-on-one time; the rest is the room's drills, meetings and
/// tape, which a project doesn't touch (proposed, not confirmed).
pub const ONE_ON_ONE_SHARE: f64 = 0.25;

/// The position coach's part of a man's coaching (C57: 0.4 in college; a
/// high school's position staff and a club's the same; proposed, not
/// confirmed for those).
pub const ROOM_PART: f64 = 0.4;

/// The share of his position coach's one-on-one time a man keeps when
/// `projects` of the room's `members` run on a skill project (design item
/// 8: a project takes the coach's minutes at the 60-minute focus, so the
/// rest of the room gets a little less). Each project takes 30 minutes
/// past its man's share out of the others' 30 each. A man on a project
/// keeps all of his (his 60 are the project). 1 when nobody runs one.
pub fn room_time(members: usize, projects: usize) -> f64 {
    let rest = members.saturating_sub(projects);
    if projects == 0 || rest == 0 {
        return 1.0;
    }
    let taken = (PROJECT_MINUTES - ONE_ON_ONE_MINUTES) * projects as f64;
    clamp(1.0 - taken / (ONE_ON_ONE_MINUTES * rest as f64), 0.0, 1.0)
}

/// A project's minutes a week with the position coach (design item 8).
pub const PROJECT_MINUTES: f64 = 60.0;

/// What `time` (from `room_time`) takes off his coaching, 1-20: the
/// position coach's part, its one-on-one share, the time he lost. In a
/// room of 9 with 2 projects each other man loses 2 parts in 7 of his
/// one-on-one time (about 9 of his 30 minutes), about 0.34 off a typical
/// 12's coaching: under 1% slower growth in his technique and reads.
pub fn room_coaching_cut(coaching: f64, time: f64) -> f64 {
    coaching * ROOM_PART * ONE_ON_ONE_SHARE * (1.0 - clamp(time, 0.0, 1.0))
}

/// Whether a man runs a skill project once his staff's changes for the
/// checkpoint land (`ai_room`, `moved`).
pub fn runs_after(p: &Player, changes: &[Change]) -> bool {
    let mut on = p.prog.project().is_some();
    for c in changes {
        match c {
            Change::End => on = false,
            Change::Start(_) => on = true,
            _ => {}
        }
    }
    on
}

// ---------------------------------------------------------------------------
// What your staff tells you about a project (the staff's notes)
// ---------------------------------------------------------------------------

/// A project's news at his checkpoint, for your staff's notes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum News {
    /// It clicked (design item 10): half of it shows within weeks.
    Clicked(Attr),
    /// Your staff sees a dead end (`you_see_dead_end`).
    Stalled(Attr),
    /// His position no longer works the skill, so it ended.
    Dropped(Attr),
    /// The body project ended: its leash is used, or his body's clock ran
    /// out (`BODY_UNTIL`).
    BodyDone(BodyKind),
}

/// The news from one man's checkpoint: `skill` and `body` are his projects
/// as the year opened (after his staff's changes), `click` what clicked.
pub fn news(skill: Option<Attr>, body: Option<BodyKind>, p: &Player, click: f64) -> Vec<News> {
    let mut out = Vec::new();
    if let Some(a) = skill {
        if p.prog.project() != Some(a) {
            out.push(News::Dropped(a));
        } else if click > 0.0 {
            out.push(News::Clicked(a));
        } else if you_see_dead_end(p) {
            out.push(News::Stalled(a));
        }
    }
    if let Some(k) = body
        && p.prog.body_project().is_none()
    {
        out.push(News::BodyDone(k));
    }
    out
}

impl News {
    /// The note's words (after his name) and the staff who writes it.
    pub fn words(self) -> (&'static str, String) {
        match self {
            News::Clicked(a) => (
                "position",
                format!(
                    "has had it click on his {} project: it came to him all at once, and some of it already shows",
                    a.label().to_lowercase()
                ),
            ),
            News::Stalled(a) => (
                "position",
                format!(
                    "isn't getting much from his {} project: little came of it this year. Your position coach would end it and give the time to someone else",
                    a.label().to_lowercase()
                ),
            ),
            News::Dropped(a) => (
                "position",
                format!(
                    "is off his {} project: his position no longer works it. What it built stays",
                    a.label().to_lowercase()
                ),
            ),
            News::BodyDone(k) => (
                "strength",
                format!(
                    "is done with his {} project: his body has gone as far as the program takes it. What it built stays",
                    match k {
                        BodyKind::Strength => "strength",
                        BodyKind::Speed => "speed",
                    }
                ),
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// The knack's clues (design item 3): staff words, never a number
// ---------------------------------------------------------------------------

/// Weeks since his skill project started (`Prog::project_on`), or its
/// years on it when the day isn't known.
pub fn project_weeks(p: &Player, today: crate::calendar::Date) -> f64 {
    if p.prog.project_on > 0 {
        ((today.0 - p.prog.project_on as i32) as f64 / 7.0).max(0.0)
    } else {
        52.0 * p.prog.project_years as f64
    }
}

/// The weeks before a position coach reads how he takes to a project
/// (design item 3: "a few weeks into a project"; proposed, not confirmed).
pub const CLUE_WEEKS: f64 = 3.0;

/// Your position coach's read of his knack for the project's kind of
/// technique (design item 3: "it's starting to click", "still fighting
/// it"): his knack plus a miss that shrinks with the weeks on it and the
/// coach's eye. `draw` is a fixed normal for him and the reader, so the
/// read only sharpens as the weeks pass. A typical eye (10) misses by one
/// spread at 3 weeks and a third of one after a year (proposed, not
/// confirmed). None before `CLUE_WEEKS`.
pub fn project_clue(p: &Player, a: Attr, eye: f64, weeks: f64, draw: f64) -> Option<f64> {
    if weeks < CLUE_WEEKS {
        return None;
    }
    let k = kind_of(a)?;
    let sd = clamp(1.5 - 0.05 * eye, 0.5, 1.5) * (CLUE_WEEKS / weeks).sqrt().max(0.3);
    Some(knack(p, k) + sd * draw)
}

/// How much he filled out in high school against his frame (design item
/// 3: "puts weight on easily", "hard gainer"; a fair hint at his body
/// knack): his body knack seen through a 0.6 correlation (proposed, not
/// confirmed). `draw` a fixed normal for him.
pub fn filled_out_clue(p: &Player, draw: f64) -> f64 {
    0.6 * body_knack(p) + 0.8 * draw
}

/// The strength staff's read from its testing days (design item 3: "took
/// to the program", "hasn't moved much"): his body knack with a miss that
/// shrinks with each testing day and the strength head (1-20). A typical
/// head misses by one spread after one day and half of one after four
/// (proposed, not confirmed). None before his first testing day here.
pub fn testing_clue(p: &Player, head: f64, days: usize, draw: f64) -> Option<f64> {
    if days == 0 {
        return None;
    }
    let sd = clamp(1.5 - 0.05 * head, 0.5, 1.5) / (days as f64).sqrt();
    Some(body_knack(p) + sd * draw)
}

/// What an AI staff does with one player's project at the checkpoint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Change {
    End,
    Start(Attr),
    EndBody,
    StartBody(BodyKind),
    /// A project your staff set, on a man who is now on this staff: it
    /// becomes the staff's own.
    Adopt,
    /// Marks his projects as this staff's (`Prog::staff`).
    Staff(u32),
}

/// The changes for a man who has moved to `staff` (`Prog::left_behind`):
/// his last staff's projects end, and his new staff picks at its next
/// checkpoint like anyone else's. A project with no staff yet (an older
/// save) is taken as this staff's. Empty when nothing changes.
pub fn moved(p: &Player, staff: u32) -> Vec<Change> {
    let pr = &p.prog;
    if pr.left_behind(staff) {
        let mut out = Vec::new();
        if pr.project > 0 {
            out.push(Change::End);
        }
        if pr.body > 0 {
            out.push(Change::EndBody);
        }
        out.push(Change::Staff(staff));
        out
    } else if pr.staff == 0 && (pr.project > 0 || pr.body > 0) {
        vec![Change::Staff(staff)]
    } else {
        Vec::new()
    }
}

/// An AI staff's projects for one room at the checkpoint (design items 8,
/// 13-15): it ends the dead ends, then fills its open slots by its lean,
/// best read first. `members` are the room's players; `scale` (0-1) is how
/// much of a full staff's time it has (a high school's smaller staff).
/// Returns each change by its place in `members`. Nothing changes here:
/// the caller applies them (`apply`) when his year opens.
pub fn ai_room(
    members: &[&Player],
    room: Room,
    lean: Lean,
    eye: f64,
    scale: f64,
    staff: u32,
    seed: u32,
) -> Vec<(usize, Change)> {
    let mut out = Vec::new();
    if !part(PART_PROJECTS) {
        return out;
    }
    let mut r = Rng::new(seed ^ 0x4F0E_C7A1);
    let mut running = 0;
    // A man who brought his last staff's projects: they end, hold no slot,
    // and he is this staff's to pick from the next checkpoint on.
    let gone: Vec<bool> = members.iter().map(|p| p.prog.left_behind(staff)).collect();
    for (i, p) in members.iter().enumerate() {
        let m = moved(p, staff);
        if !m.is_empty() {
            out.extend(m.into_iter().map(|c| (i, c)));
        }
        if gone[i] {
            continue;
        }
        // A project your staff set on a man now on an AI staff (he moved, or
        // you left) is that staff's to judge from here: it adopts it, and
        // its own dead-end rule decides it like any other.
        if p.prog.yours || p.prog.body_yours {
            out.push((i, Change::Adopt));
        }
        if p.prog.project().is_some() {
            if ai_keeps(p) {
                running += 1;
            } else {
                out.push((i, Change::End));
            }
        }
    }
    // The strength staff's body projects (design items 7 and 8): its own
    // slot a room, on the men whose bodies trail the spot most.
    if part(PART_BODY) && scale >= 1.0 {
        let body_running =
            members.iter().enumerate().filter(|(i, p)| !gone[*i] && p.prog.body_project().is_some()).count();
        if body_running < BODY_SLOTS {
            let best = members
                .iter()
                .enumerate()
                .filter(|(i, p)| {
                    !gone[*i] && p.prog.drawn && p.prog.body_project().is_none() && p.age <= BODY_UNTIL - 2
                })
                .map(|(i, p)| (i, body_bet(p, eye, &mut r)))
                .filter(|c| c.1 >= BODY_BET_FLOOR)
                .max_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((i, _)) = best
                && r.chance(lean.fill())
            {
                out.push((i, Change::StartBody(BodyKind::Strength)));
                out.push((i, Change::Staff(staff)));
            }
        }
    }
    let open = room.slots().saturating_sub(running);
    if open == 0 {
        return out;
    }
    let mut cands: Vec<(usize, Attr, f64)> = members
        .iter()
        .enumerate()
        .filter(|(i, p)| !gone[*i] && p.prog.drawn && p.prog.project().is_none())
        .filter_map(|(i, p)| best_bet(p, AI_SHARP, eye, &mut r).map(|(a, v)| (i, a, v)))
        .filter(|c| c.2 >= BET_FLOOR)
        .collect();
    // A win-now staff only bets on kids already close to the field.
    if lean.0 < 0.5 {
        let mut ov: Vec<f64> = members.iter().map(|p| p.overall()).collect();
        ov.sort_by(|a, b| b.total_cmp(a));
        let bar = ov.get(ov.len() / 3).copied().unwrap_or(0.0);
        cands.retain(|c| members[c.0].overall() >= bar);
    }
    cands.sort_by(|a, b| b.2.total_cmp(&a.2));
    for c in cands.into_iter().take(open) {
        if r.chance(lean.fill() * clamp(scale, 0.0, 1.0)) {
            out.push((c.0, Change::Start(c.1)));
            out.push((c.0, Change::Staff(staff)));
        }
    }
    out
}

/// Applies an AI staff's change.
pub fn apply(p: &mut Player, c: Change) {
    match c {
        Change::End => end_project(p),
        Change::Start(a) => {
            start_project(p, a, false);
        }
        Change::EndBody => end_body(p),
        Change::Adopt => {
            p.prog.yours = false;
            p.prog.body_yours = false;
        }
        Change::Staff(s) => p.prog.staff = s,
        Change::StartBody(k) => {
            start_body(p, k, false);
        }
    }
}

/// A staff with no rooms to see (the class sim): one player's chance to be
/// picked a year, set so a room's slots fill as `ai_room` fills them
/// (design item 13: about 1.5 a 20-man room a year on the prototype).
/// `staff` is the staff he is with (the class sim has one a level).
pub fn ai_single(p: &mut Player, lean: Lean, eye: f64, hs: bool, coaching: f64, staff: u32) {
    if !part(PART_PROJECTS) {
        return;
    }
    ensure(p);
    // The class sim has no seat: nothing here is yours.
    p.prog.yours = false;
    p.prog.body_yours = false;
    // A new level is a new staff: his last staff's projects end.
    for c in moved(p, staff) {
        apply(p, c);
    }
    p.prog.staff = staff;
    let mut r = stream(p.id, 0x5E1 ^ p.age.wrapping_mul(0x3B));
    // A college strength staff's body project: about one a room at a time
    // (`BODY_SLOTS`), so a man's chance a year is about a room's share.
    if !hs
        && part(PART_BODY)
        && p.prog.body_project().is_none()
        && p.age <= BODY_UNTIL - 2
        && body_bet(p, eye, &mut r) >= BODY_BET_FLOOR
        && r.chance(0.08 * lean.fill())
    {
        start_body(p, BodyKind::Strength, false);
    }
    if p.prog.project().is_some() {
        if !ai_keeps(p) {
            end_project(p);
        }
        return;
    }
    let Some(room) = Room::of(p.position) else { return };
    let Some((a, v)) = best_bet(p, AI_SHARP, eye, &mut r) else { return };
    if v < BET_FLOOR {
        return;
    }
    // About 1.5 bets a 20-man room a year in college before the lean
    // (design item 13, the prototype's 7.5%), about 1 in 20 of a high
    // school room (item 14); the room's slots set the scale.
    let per = room.slots() as f64 / 2.0;
    let base = if hs { 0.02 * clamp(coaching / 10.0, 0.3, 1.5) } else { 0.03 };
    if r.chance(base * per * lean.0) {
        start_project(p, a, false);
    }
}

// ---------------------------------------------------------------------------
// Body projects (design items 1, 2, 7, 8 and 11)
// ---------------------------------------------------------------------------

/// A body project: the strength staff rebuilding his body. Strength is the
/// long leash (both halves, balance and up to 25 lb), speed the short one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BodyKind {
    Strength,
    Speed,
}

impl BodyKind {
    /// The ratings it moves and the most each takes for a big responder
    /// (design item 2's leashes).
    pub fn ratings(self) -> &'static [(Attr, f64)] {
        match self {
            BodyKind::Strength => &[(Attr::Strength, 14.0), (Attr::LowerBody, 14.0), (Attr::Balance, 3.0)],
            BodyKind::Speed => &[
                (Attr::Speed, 1.0),
                (Attr::Acceleration, 1.5),
                (Attr::Jumping, 1.5),
                (Attr::Agility, 2.0),
                (Attr::Flexibility, 2.0),
            ],
        }
    }

    fn code(self) -> u8 {
        match self {
            BodyKind::Strength => 1,
            BodyKind::Speed => 2,
        }
    }
}

/// The most a strength project puts on him (design item 2): 25 lb past
/// where the weight room takes him.
pub const BODY_LBS: f64 = 25.0;

/// Pounds a point of strength ceiling built carries on a 190 lb frame,
/// more on a bigger one by his frame weight, so the design's lanky
/// receiver carries about 15 lb for his 14 and a big responder at tackle
/// reaches the 25 (proposed, not confirmed).
pub const BODY_LBS_PER_POINT: f64 = 1.2;

/// The share of what's left of a body project's leash a year closes with a
/// typical strength staff, so a big responder reaches about his whole
/// leash in four years (design's lanky receiver: about +14 strength).
/// Proposed, not confirmed.
pub const BODY_YEAR: f64 = 0.45;

/// The body clock (design item 11): strength builds to about 24-25, so a
/// body project stops after 25 whatever his level.
pub const BODY_UNTIL: u32 = 25;

/// A body project year's extra wear (design item 7: more injury risk and
/// less in the tank). Proposed, not confirmed.
pub const BODY_WEAR: f64 = 0.25;

/// How much of a body leash his body knack opens (design item 2: about 7
/// of strength's 14 for a typical responder, all of it for a big one,
/// close to none for a non-responder): Φ(1.2 z).
pub fn body_share(z: f64) -> f64 {
    crate::athlete::phi(1.2 * z)
}

/// How far his frame lets a strength project go (design item 2: his
/// height, arms and frame weight): a tall, light frame for his position
/// has the most to fill, a short, heavy one the least, 0.6 to 1.05 (so a
/// lanky big responder's strength leash is the design's 14). Proposed, not
/// confirmed.
pub fn frame_room(p: &Player) -> f64 {
    let (hm, hs, wm, ws) = crate::body::combine_size(p.position);
    let hz = (p.body.height - hm) / hs.max(0.5);
    let wz = (p.body.frame_weight - wm) / ws.max(1.0);
    let az = (p.body.arm - 0.425 * p.body.height) / 0.8;
    clamp(1.0 + 0.15 * (hz - wz) + 0.05 * az, 0.6, 1.05)
}

/// The most this body project can add to a rating for him.
pub fn body_leash(p: &Player, kind: BodyKind, most: f64) -> f64 {
    let frame = if kind == BodyKind::Strength { frame_room(p) } else { 1.0 };
    most * body_share(body_knack(p)) * frame
}

/// Starts a body project (you, or an AI strength staff's pick).
pub fn start_body(p: &mut Player, kind: BodyKind, yours: bool) -> bool {
    if p.position.is_specialist() || p.age > BODY_UNTIL {
        return false;
    }
    ensure(p);
    if p.prog.body_project() != Some(kind) {
        p.prog.body_years = 0;
        p.prog.body_used = [0; 5];
        p.prog.body_lbs = 0;
    }
    p.prog.body = kind.code();
    p.prog.body_yours = yours;
    true
}

/// Ends his body project (what it built stays).
pub fn end_body(p: &mut Player) {
    p.prog.body = 0;
    p.prog.body_yours = false;
}

/// What a body project year did.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyYear {
    pub kind: Option<BodyKind>,
    /// Ceiling points added, all its ratings summed.
    pub lift: f64,
    pub lbs: f64,
}

/// A body project year at his checkpoint (design items 1, 2, 7, 11): each
/// of its ratings' ceilings closes `BODY_YEAR` of what's left of his leash
/// there, by his strength staff (`coach_mult`) and U(0.7, 1.3); a strength
/// project also puts on the pounds the strength built carries
/// (`BODY_LBS_PER_POINT`, at most `BODY_LBS`), with what each pound does to
/// a body (`training::seat::body_change`). It runs on the body's clock, not
/// the school's, and a year away from the staff does nothing. His wear
/// rises `BODY_WEAR` a year. A full leash ends it.
pub(crate) fn body_year(p: &mut Player, level: Level, strength_staff: f64, trouble: bool) -> BodyYear {
    let Some(kind) = p.prog.body_project() else { return BodyYear::default() };
    if !part(PART_BODY) || p.age > BODY_UNTIL || p.position.is_specialist() {
        end_body(p);
        return BodyYear::default();
    }
    if trouble || level == Level::HS && !p.prog.body_yours {
        return BodyYear { kind: Some(kind), ..BodyYear::default() };
    }
    ensure(p);
    let mut r = stream(p.id, 0xB0D7 ^ p.age.wrapping_mul(0x2D1));
    let pace = BODY_YEAR * coach_mult(strength_staff) * r.range(0.7, 1.3);
    let mut lift = 0.0;
    let mut left_all = 0.0;
    for (k, &(a, most)) in kind.ratings().iter().enumerate() {
        let leash = body_leash(p, kind, most);
        let used = p.prog.body_used[k] as f64 / 2.0;
        let left = (leash - used).max(0.0);
        let add = (left * pace).min(left);
        crate::athlete::move_ceiling(&mut p.ceilings, a, add);
        p.prog.body_used[k] = ((used + add) * 2.0).round().clamp(0.0, 255.0) as u8;
        lift += add;
        left_all += left - add;
    }
    let mut lbs = 0.0;
    if kind == BodyKind::Strength {
        let built = (p.prog.body_used[0] as f64 + p.prog.body_used[1] as f64) / 4.0;
        let room = (built * BODY_LBS_PER_POINT * p.body.frame_weight / 190.0).min(BODY_LBS);
        let done = p.prog.body_lbs as f64;
        lbs = (room.floor() - done).max(0.0);
        if lbs > 0.0 {
            crate::training::seat::body_change(p, lbs);
            p.prog.body_lbs = (done + lbs).clamp(0.0, 255.0) as u8;
        }
    }
    p.wear += BODY_WEAR;
    p.prog.body_years = p.prog.body_years.saturating_add(1);
    if left_all < 0.5 && !p.prog.body_yours {
        end_body(p);
    }
    BodyYear { kind: Some(kind), lift, lbs }
}

/// An AI strength staff's read of how far his body trails what his spot
/// asks of it (design item 13: from its own read, never true values): the
/// typical college starter's upper and lower strength at his position less
/// his, with the staff's error, times his frame's room. 0 for a man who
/// doesn't trail.
pub fn body_bet(p: &Player, eye: f64, r: &mut Rng) -> f64 {
    let typ = |a: Attr| crate::game::synthetic::typical_starter(Level::FBS, p.position, a);
    let gap = 0.5 * (typ(Attr::Strength) - p.attributes.get(Attr::Strength))
        + 0.5 * (typ(Attr::LowerBody) - p.attributes.get(Attr::LowerBody));
    let err = r.normal(0.0, 6.0 * (1.5 - clamp(eye / 20.0, 0.0, 1.0)));
    ((gap + err) * frame_room(p)).max(0.0)
}

/// Your strength staff's read of a body project: each rating's gain (low,
/// high) and the pounds (low, high).
pub type BodyRead = (Vec<(Attr, f64, f64)>, (f64, f64));

/// Your strength staff's read of a body project for the screens: for each
/// rating it moves, what it could add over `years` (low, high), and the
/// pounds a strength project would put on him. Nobody knows how a body
/// responds until it is worked (design item 1), so before any years on it
/// the read spans a body knack of -1 to 1; each year on it narrows the read
/// toward how he has responded. The leash's share is `body_share`, the
/// frame `frame_room`, a year closes `BODY_YEAR` of what's left by the
/// staff. The spreads are Claude's, proposed, not confirmed.
pub fn your_body_read(p: &Player, kind: BodyKind, eye: f64, staff: f64, years: u32) -> BodyRead {
    let on = if p.prog.body_project() == Some(kind) { p.prog.body_years as f64 } else { 0.0 };
    let z = body_knack(p);
    let center = z * on / (on + 1.0);
    let half = (1.0 + (10.0 - clamp(eye, 1.0, 20.0)) / 20.0) / (on + 1.0);
    let frame = if kind == BodyKind::Strength { frame_room(p) } else { 1.0 };
    let close = 1.0 - (1.0 - clamp(BODY_YEAR * coach_mult(staff), 0.0, 1.0)).powi(years as i32);
    let mut out = Vec::new();
    for (k, &(a, most)) in kind.ratings().iter().enumerate() {
        let used = if on > 0.0 { p.prog.body_used[k] as f64 / 2.0 } else { 0.0 };
        let left = |zz: f64| (most * body_share(zz) * frame - used).max(0.0) * close;
        out.push((a, left(center - half), left(center + half)));
    }
    let lbs = if kind == BodyKind::Strength && out.len() >= 2 {
        let done = if on > 0.0 { p.prog.body_lbs as f64 } else { 0.0 };
        let built = if on > 0.0 { (p.prog.body_used[0] as f64 + p.prog.body_used[1] as f64) / 4.0 } else { 0.0 };
        let to = |pts: f64| ((built + pts) * BODY_LBS_PER_POINT * p.body.frame_weight / 190.0).min(BODY_LBS) - done;
        (to(0.5 * (out[0].1 + out[1].1)).max(0.0), to(0.5 * (out[0].2 + out[1].2)).max(0.0))
    } else {
        (0.0, 0.0)
    };
    (out, lbs)
}

/// A gap that's worth a strength project to an AI staff (proposed, not
/// confirmed).
pub const BODY_BET_FLOOR: f64 = 8.0;

/// Body projects an AI program's strength staff runs a year for a room
/// (design item 8: the strength staff, not the room, limits them; one a
/// room, about nine a program, proposed, not confirmed).
pub const BODY_SLOTS: usize = 1;

// ---------------------------------------------------------------------------
// The trims (everyday item 11)
// ---------------------------------------------------------------------------

/// What a family's natural ceilings of 60+ give up, so the world keeps
/// today's number of stars with everything on (everyday item 11). A family
/// is one limited skill at one position, never lumped. `[a, b]` takes a
/// points for each point past 60 and b more (most often less) for each
/// point past 75 (`trim_at`, never under 0): the middle band of natural
/// 65-80s gives up the most, a natural 62 almost nothing, and most families'
/// natural 85s little or nothing, since slower closing near the top
/// (`near_top`, Cain's card 2026-11-17) already holds the 80+ count. With
/// both, the 70+ and 80+ counts hold (Claude's shape, proposed, not
/// confirmed). The limit keeps the trimmed points, so investment can win
/// them back. Sized on the class sim with every part on (`prog_check fit`);
/// the build sizes them again on a world. Indexed [position][limited
/// skill].
static TRIMS: [[[f32; 2]; LIMITED_COUNT]; POSITION_COUNT] = TRIM_TABLE;

/// The fitted slopes (`prog_check fit`, see `TRIMS`).
const TRIM_TABLE: [[[f32; 2]; LIMITED_COUNT]; POSITION_COUNT] = {
    let mut t = [[[0.0f32; 2]; LIMITED_COUNT]; POSITION_COUNT];
    // Order of `LIMITED`: man, zone, press, tackling, shedding, contested
    // catch, routes, release, power rush, finesse rush, pass blocking, run
    // blocking, footwork, leg drive, pursuit, blitzing, throw on the move.
    // Order of `Position::ALL`: QB, RB, WR, TE, OT, IOL, EDGE, DT, LB, CB,
    // S, K, P, LS.
    // FITTED_TRIMS_BEGIN
    t[0][12] = [0.11, -0.16]; // QB footwork
    t[0][16] = [0.17, -0.68]; // QB throw on the move
    t[1][10] = [0.07, -0.26]; // RB pass blocking
    t[1][13] = [0.53, -1.13]; // RB leg drive
    t[2][6] = [0.32, -0.92]; // WR routes
    t[2][7] = [0.13, -0.37]; // WR release
    t[3][6] = [0.23, -0.60]; // TE routes
    t[3][11] = [0.28, -0.88]; // TE run blocking
    t[4][10] = [0.59, -0.72]; // OT pass blocking
    t[4][11] = [0.14, -0.22]; // OT run blocking
    t[5][10] = [0.18, -0.78]; // IOL pass blocking
    t[5][11] = [0.06, -0.20]; // IOL run blocking
    t[6][4] = [0.29, -0.73]; // EDGE shedding
    t[6][8] = [0.16, -0.45]; // EDGE power rush
    t[6][9] = [0.09, 0.01]; // EDGE finesse rush
    t[7][4] = [0.25, -0.81]; // DT shedding
    t[7][8] = [0.13, 0.02]; // DT power rush
    t[8][1] = [0.07, -0.65]; // LB zone coverage
    t[8][3] = [0.14, -0.74]; // LB tackling
    t[8][14] = [0.01, -0.07]; // LB pursuit
    t[9][0] = [0.21, -0.31]; // CB man coverage
    t[9][1] = [0.23, -0.78]; // CB zone coverage
    t[10][1] = [0.09, -0.69]; // S zone coverage
    t[10][3] = [0.09, -0.27]; // S tackling
    // FITTED_TRIMS_END
    t
};

/// A trial override of the trims (`prog_check fit`).
static TRIM_OVERRIDE: std::sync::RwLock<Option<Vec<[[f32; 2]; LIMITED_COUNT]>>> = std::sync::RwLock::new(None);

pub fn set_trims(t: Option<Vec<[[f32; 2]; LIMITED_COUNT]>>) {
    *TRIM_OVERRIDE.write().unwrap() = t;
}

pub fn trims() -> Vec<[[f32; 2]; LIMITED_COUNT]> {
    TRIM_OVERRIDE.read().unwrap().clone().unwrap_or_else(|| TRIMS.to_vec())
}

/// A family's two slopes (see `TRIMS`).
pub fn trim(pos: Position, a: Attr) -> [f64; 2] {
    let Some(li) = limited_index(a) else { return [0.0; 2] };
    let t = match TRIM_OVERRIDE.read().unwrap().as_ref() {
        Some(t) => t[pos.index()][li],
        None => TRIMS[pos.index()][li],
    };
    [t[0] as f64, t[1] as f64]
}

/// The points a natural ceiling `c` gives up in a family (see `TRIMS`).
pub fn trim_at(pos: Position, a: Attr, c: f64) -> f64 {
    let [k, top] = trim(pos, a);
    (k * (c - TRIM_FROM).max(0.0) + top * (c - TRIM_STEEP).max(0.0)).max(0.0)
}

/// The share of a trim the limit gives up too (0: the limit keeps every
/// trimmed point). A trial lever for `prog_check` (`AFM_TRIM_LIMIT`).
static TRIM_LIMIT_SHARE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn set_trim_limit_share(f: f64) {
    TRIM_LIMIT_SHARE.store(f.to_bits(), Ordering::Relaxed);
}

/// The natural ceiling a trim starts from.
pub const TRIM_FROM: f64 = 60.0;

/// Where a trim's second slope starts (see `TRIMS`).
pub const TRIM_STEEP: f64 = 75.0;

/// The last age a pending trim lands (his school years).
pub const TRIM_UNTIL: u32 = 22;

/// A new player's knacks and limits, with his trims pending: each lands
/// the first year he plays a spot whose family has one (so a boy moved to
/// corner at 16 gives up corner's points then). An older save's players
/// have none pending and keep their ceilings.
pub fn ensure_new(p: &mut Player) {
    if p.prog.drawn || fitting() {
        return;
    }
    ensure(p);
    if p.age <= 15 {
        p.prog.pending = (1u32 << LIMITED_COUNT) - 1;
    }
}

/// Lands his pending trims at his spot (at each checkpoint, before the
/// year's closing).
pub(crate) fn land_trims(p: &mut Player) {
    if p.prog.pending == 0 {
        return;
    }
    if p.age > TRIM_UNTIL {
        p.prog.pending = 0;
        return;
    }
    if !part(PART_TRIMS) {
        return;
    }
    for (li, &a) in LIMITED.iter().enumerate() {
        if p.prog.pending >> li & 1 == 0 {
            continue;
        }
        if trim(p.position, a) == [0.0; 2] {
            continue;
        }
        p.prog.pending &= !(1 << li);
        let c = p.ceilings.get(a);
        let t = trim_at(p.position, a, c);
        if t <= 0.0 {
            continue;
        }
        // The limit stays where it was before the trim, less the share of
        // the trim it gives up too (`TRIM_LIMIT_SHARE`, a trial lever).
        let f = f64::from_bits(TRIM_LIMIT_SHARE.load(Ordering::Relaxed));
        let keep = ((p.prog.over[li] as f64 / 2.0).max(c - mapped(p, a)) - f * t).max(c - t - mapped(p, a)).max(0.0);
        p.prog.over[li] = (keep * 2.0).ceil().clamp(0.0, 255.0) as u8;
        p.ceilings.set(a, c - t);
        if p.attributes.get(a) > c - t {
            p.attributes.set(a, c - t);
        }
    }
}
