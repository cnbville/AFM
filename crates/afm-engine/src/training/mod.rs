//! Section 6b: training (`docs/design/training.md`). How a practice is
//! built and what each part grows, the week and the year on each level's
//! real rules, the time budget, each player's plan, load and practice
//! injuries, practice reps as battles, and growth landing as the work
//! happens (choice 1, confirmed).
//!
//! How growth lands: the yearly step (`development::develop_season`)
//! still decides a player's year at his checkpoint: ceilings, form,
//! injuries, ageing and how fast he grows. With `defer` it keeps the
//! growth itself back and stores the year's rate here (`Train`). Each week
//! his team's practice lands a share of it, family by family
//! (`land`): the share the level's calendar gives that week, moved by
//! what the week's practice really worked (`plan`, `work`). Shares adding
//! to one give exactly the yearly step's growth, so the default plan of an
//! average staff keeps the calibration (topic 12); a plan moves a young
//! player's year by up to about 15% either way (choice 2). At the next
//! checkpoint whatever the calendar never reached lands (`close_year`) and
//! his season row gets the rating he really ended on.

pub mod offseason;
pub mod plan;
pub mod practice;
pub mod rules;
pub mod seat;
pub mod week;
pub mod work;

use crate::attributes::{Attr, AttributeGroup, Category};
use crate::development::{GrowthCtx, declines, grow_share, growth_age_factor};
use crate::player::Player;
use crate::positions::{Position, overall_for};
use crate::rng::{Rng, clamp};

/// What a period grows (topic 1): the attribute families of training.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Family {
    /// Strength and balance: the weight room.
    Strength,
    /// Speed, acceleration, agility, flexibility and jumping.
    Speed,
    /// Stamina: conditioning and the volume of practice.
    Conditioning,
    /// The skills of his position, taught in individual and group periods.
    Technique,
    /// Tackling, shedding, anchoring, running through contact, holding the
    /// ball: what only padded, live and thud reps teach.
    Contact,
    /// Kicking, punting and snapping.
    Kicking,
    /// Reading, knowing, deciding: meetings, film, walkthroughs, team reps.
    Mental,
}

pub const FAMILIES: usize = 7;

impl Family {
    pub const ALL: [Family; FAMILIES] = [
        Family::Strength,
        Family::Speed,
        Family::Conditioning,
        Family::Technique,
        Family::Contact,
        Family::Kicking,
        Family::Mental,
    ];
    pub fn name(self) -> &'static str {
        ["strength", "speed", "conditioning", "technique", "contact", "kicking", "mental"][self as usize]
    }
    pub fn label(self) -> &'static str {
        ["Strength", "Speed and movement", "Conditioning", "Technique", "Contact", "Kicking", "Mental"][self as usize]
    }
}

/// The family an attribute grows in.
pub fn family_of(a: Attr) -> Family {
    use Attr::*;
    match a {
        Strength | Balance => Family::Strength,
        Speed | Acceleration | Agility | Flexibility | Jumping => Family::Speed,
        Stamina => Family::Conditioning,
        Tackling | BlockShedding | BallSecurity | LegDrive | Anchor | RunBlocking | PowerRush | Press | Traffic
        | Physicality | Toughness => Family::Contact,
        _ if a.category() == Category::KickingAndSnapping => Family::Kicking,
        _ if a.profile().group == AttributeGroup::Mental => Family::Mental,
        _ => Family::Technique,
    }
}

/// A year's growth rate as the yearly step left it, packed small (every
/// player in the world carries one; memory gate).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GrowthYear {
    /// His age at the checkpoint; 0 when nothing is pending.
    pub age: u8,
    /// Bit 0 high school, 1 college, 2 redshirt year.
    pub flags: u8,
    /// Coaching as it sticks to him: 0.5 + v / 250.
    pub coach: u8,
    /// Playing time and level: v / 200.
    pub snaps: u8,
    /// His mood and his room's: 0.8 + v / 600.
    pub morale: u8,
    /// Strength staff and professionalism: 0.9 + v / 1200.
    pub physical: u8,
    /// The year's form times its injuries: v / 4000.
    pub form: u16,
}

/// A player's training store (section 6b, about 34 bytes): the year's
/// growth still landing, how much of each family's year the calendar has
/// reached, growth waiting to land, his running load and his practice
/// grade.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Train {
    pub year: GrowthYear,
    /// The calendar's reach into the year, per family, in 1/10000ths.
    pub covered: [u16; FAMILIES],
    /// Growth earned and not yet landed, per family, in 1/2000ths of a
    /// year (an AI player's growth lands in a few installments a block).
    #[serde(default)]
    pub due: [u8; FAMILIES],
    /// Bloom at the checkpoint in quarter years (development age = age - bloom).
    pub bloom4: i8,
    /// His position at the checkpoint: the attributes it uses grow
    /// through the year, the rest settle at the checkpoint.
    #[serde(default)]
    pub pos: u8,
    /// Load: this week's and the last weeks' (chronic), in tens of units.
    #[serde(default)]
    pub load: [u8; 2],
    /// His practice grade as his coaches see it, 1-100 (0: none yet).
    #[serde(default)]
    pub grade: u8,
}

fn q(v: f64, lo: f64, scale: f64) -> u8 {
    clamp(((v - lo) * scale).round(), 0.0, 255.0) as u8
}

/// Whether an attribute's growth lands through training: the ones his
/// position uses. The others (a receiver's tackling) settle at the
/// checkpoint as before.
pub fn defers(pos: Position, key: Attr) -> bool {
    crate::positions::weight(pos, key) > 0.0
}

impl Train {
    /// The year opened at a checkpoint with growth held back.
    pub(crate) fn open(g: &GrowthCtx, age: u32, development_age: f64, pos: Position, load: [u8; 2]) -> Train {
        let flags = g.hs as u8 | (g.college as u8) << 1 | (g.redshirt as u8) << 2;
        let bloom = age as f64 - development_age;
        Train {
            year: GrowthYear {
                age: age.clamp(1, 255) as u8,
                flags,
                coach: q(g.coaching_boost, 0.5, 250.0),
                snaps: q(g.snaps_mult, 0.0, 200.0),
                morale: q(g.morale_mult, 0.8, 600.0),
                physical: q(g.physical_mult, 0.9, 1200.0),
                form: clamp((g.form * g.growth_multiplier * 4000.0).round(), 0.0, 65535.0) as u16,
            },
            covered: [0; FAMILIES],
            due: [0; FAMILIES],
            bloom4: clamp((bloom * 4.0).round(), -127.0, 127.0) as i8,
            pos: pos.index() as u8,
            load,
            grade: 0,
        }
    }

    pub fn pending(&self) -> bool {
        self.year.age > 0
    }

    /// The share of each family's year the calendar has reached, 0-1.
    pub fn landed(&self, f: Family) -> f64 {
        self.covered[f as usize] as f64 / 10000.0
    }

    pub fn position(&self) -> Position {
        Position::ALL[(self.pos as usize).min(Position::ALL.len() - 1)]
    }

    /// Acute against chronic load: above about 1.3 a sudden jump.
    pub fn spike(&self) -> f64 {
        let (a, c) = (self.load[0] as f64, self.load[1] as f64);
        if c < 3.0 { if a > 6.0 { 1.6 } else { 1.0 } } else { a / c }
    }

    /// Takes a week's load (in units; tens stored), the chronic load
    /// following over about four weeks.
    pub fn add_load(&mut self, units: f64) {
        let a = clamp(units / 10.0, 0.0, 255.0);
        let c = 0.75 * self.load[1] as f64 + 0.25 * a;
        self.load = [a.round() as u8, c.round() as u8];
    }

    fn ctx(&self, work_ethic: f64, focus: Option<Attr>, coachability: f64) -> GrowthCtx {
        let y = &self.year;
        GrowthCtx {
            coaching_boost: 0.5 + y.coach as f64 / 250.0,
            snaps_mult: y.snaps as f64 / 200.0,
            work_mult: 0.75 + work_ethic * 0.025,
            form: y.form as f64 / 4000.0,
            growth_multiplier: 1.0,
            morale_mult: 0.8 + y.morale as f64 / 600.0,
            physical_mult: 0.9 + y.physical as f64 / 1200.0,
            hs: y.flags & 1 != 0,
            college: y.flags & 2 != 0,
            redshirt: y.flags & 4 != 0,
            focus,
            focus_take: clamp(0.6 + 0.02 * coachability, 0.6, 1.0),
        }
    }

    fn development_age(&self) -> f64 {
        self.year.age as f64 - self.bloom4 as f64 / 4.0
    }
}

/// Whether an attribute's growth this year lands through training: his
/// checkpoint position uses it, it still grows, and it doesn't decline.
fn deferred(p: &Player, key: Attr) -> bool {
    let t = &p.train;
    let pos = t.position();
    defers(pos, key)
        && growth_age_factor(t.development_age(), key.profile().growth_window) > 0.01
        && !declines(pos, p.longevity, key, t.year.age as f64)
}

/// A cheap normal draw (four uniforms), for the many small installments.
fn quick_normal(rng: &mut Rng) -> f64 {
    (rng.next() + rng.next() + rng.next() + rng.next() - 2.0) * 1.732_050_8
}

/// Lands one installment: `share` of each family's year (shares over a
/// year add to about one; a plan moves them). `focus` is his plan's main
/// focus. The noise comes from its own keyed stream, so nothing else in
/// the world draws differently. Returns the overall gained.
pub fn land(p: &mut Player, share: &[f64; FAMILIES], focus: Option<Attr>, key: u64) -> f64 {
    let done: [f64; FAMILIES] = std::array::from_fn(|f| p.train.landed(Family::ALL[f]) - share[f]);
    land_after(p, share, &done, focus, key)
}

/// `land` with the share of each family already landed given (`done`).
pub fn land_after(
    p: &mut Player,
    share: &[f64; FAMILIES],
    done: &[f64; FAMILIES],
    focus: Option<Attr>,
    key: u64,
) -> f64 {
    if !p.train.pending() || share.iter().all(|&s| s <= 0.0) {
        return 0.0;
    }
    let before = p.overall();
    let g = p.train.ctx(p.personality.work_ethic, focus, p.personality.coachability);
    let dev_age = p.train.development_age();
    let mut rng = Rng::new((key ^ (p.id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x7A11) as u32);
    let (mut attrs, mut ceil) = (p.attributes, p.ceilings);
    for a in Attr::ALL {
        let s = share[family_of(a) as usize];
        if s <= 0.0 || !deferred(p, a) {
            continue;
        }
        let af = growth_age_factor(dev_age, a.profile().growth_window);
        let rate = g.rate(a, af);
        let noise = 0.5 * s.sqrt() * quick_normal(&mut rng);
        grow_share(&mut attrs, &mut ceil, a, rate, s, done[family_of(a) as usize], noise);
    }
    p.attributes = attrs;
    p.ceilings = ceil;
    p.overall() - before
}

/// Adds the calendar's reach this week to each family (capped at the
/// whole year), and returns the part of `base` still inside the year.
pub fn cover(t: &mut Train, base: &[f64; FAMILIES]) -> [f64; FAMILIES] {
    let mut out = [0.0; FAMILIES];
    for f in 0..FAMILIES {
        let left = (10000 - t.covered[f].min(10000) as i32) as f64 / 10000.0;
        let b = base[f].min(left).max(0.0);
        out[f] = b;
        t.covered[f] = (t.covered[f] as f64 + b * 10000.0).round().min(10000.0) as u16;
    }
    out
}

/// Adds earned growth to what waits to land; true when it should land now
/// (a family has a twelfth of a year waiting).
pub fn owe(t: &mut Train, s: &[f64; FAMILIES]) -> bool {
    let mut full = false;
    for f in 0..FAMILIES {
        let v = t.due[f] as f64 + s[f] * 2000.0;
        if v >= 255.0 {
            full = true;
        }
        t.due[f] = clamp(v.round(), 0.0, 255.0) as u8;
        full |= t.due[f] >= 167;
    }
    full
}

/// Lands what waits, and clears it.
pub fn settle(p: &mut Player, focus: Option<Attr>, key: u64) -> f64 {
    let due = p.train.due.map(|v| v as f64 / 2000.0);
    p.train.due = [0; FAMILIES];
    land(p, &due, focus, key)
}

/// Closes a year at the next checkpoint (topic 12): what waits lands, what
/// the calendar never reached lands now at the staff's default, and his
/// last season row ends on the rating he really reached.
pub fn close_year(p: &mut Player, focus: Option<Attr>) {
    if !p.train.pending() {
        return;
    }
    settle(p, focus, 0xC1);
    let mut rest = [0.0; FAMILIES];
    for (f, r) in rest.iter_mut().enumerate() {
        *r = (1.0 - p.train.covered[f] as f64 / 10000.0).max(0.0);
    }
    let done: [f64; FAMILIES] = std::array::from_fn(|f| p.train.landed(Family::ALL[f]));
    land_after(p, &rest, &done, focus, 0xC105E);
    let load = p.train.load;
    p.train = Train { load, ..Train::default() };
    let now = p.overall();
    if let Some(h) = p.history.last_mut() {
        h.overall = now;
    }
}

/// The overall he ends the year on if the whole year's growth lands at
/// the default (no noise): the season row's end until the year closes.
pub fn projected_overall(p: &Player) -> f64 {
    let g = p.train.ctx(p.personality.work_ethic, None, p.personality.coachability);
    let dev_age = p.train.development_age();
    let (mut attrs, mut ceil) = (p.attributes, p.ceilings);
    for a in Attr::ALL {
        if !deferred(p, a) {
            continue;
        }
        let af = growth_age_factor(dev_age, a.profile().growth_window);
        let rate = g.rate(a, af);
        grow_share(&mut attrs, &mut ceil, a, rate, 1.0, 0.0, 0.0);
    }
    overall_for(p.position, &attrs)
}

/// The growth still ahead of him this year, in points of his overall, if
/// the rest of the year lands at the default (a debug and trial read; the
/// game shows only words).
pub fn ahead(p: &Player) -> f64 {
    if !p.train.pending() {
        return 0.0;
    }
    let mut q = p.clone();
    let mut rest = [0.0; FAMILIES];
    for (f, r) in rest.iter_mut().enumerate() {
        *r = (1.0 - q.train.covered[f] as f64 / 10000.0).max(0.0) + q.train.due[f] as f64 / 2000.0;
    }
    let done: [f64; FAMILIES] =
        std::array::from_fn(|f| q.train.landed(Family::ALL[f]) - q.train.due[f] as f64 / 2000.0);
    let before = q.overall();
    land_after(&mut q, &rest, &done, None, 1);
    q.overall() - before
}

/// His practice grades read as rating points on the depth chart (topic 8):
/// a grade of 60 is average; each 10 points over is about 1.2 rating
/// points, at most 3 either way (proposed, not confirmed).
pub fn impression(t: &Train) -> f64 {
    if t.grade == 0 {
        return 0.0;
    }
    clamp((t.grade as f64 - 60.0) / 10.0 * 1.2, -3.0, 3.0)
}
