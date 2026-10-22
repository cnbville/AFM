//! Dynamic Links (`docs/design/dynamic-links.md`): physical gifts that
//! stack. A link is two to four body traits measured against his position
//! on the real combine numbers; the weakest part sets its strength, so one
//! elite trait alone makes no link. It grows smoothly from about the top
//! 30% at his position (every part) to full near the top 5%, with no cliff.
//!
//! A link is read from his body and athletic traits as they are today, so
//! it changes over a career: a weight program can build Anchor, age takes
//! Burner first, a major injury that costs speed can cost a link. Nothing is
//! drawn for it, so no other random stream moves.
//!
//! On the field a full link is worth about five rating points, and only in
//! its own situations (`game/`): each trait keeps doing its own job, and the
//! link adds only what the traits give together. Links in different
//! situations each count in full; in the same situation the strongest counts
//! in full, the next at half (and a third at a quarter).

use std::sync::LazyLock;

use crate::attributes::{Attr, Attributes};
use crate::body::{Body, body_now};
use crate::player::Player;
use crate::positions::{POSITION_COUNT, Position};
use crate::rng::{Rng, clamp};

pub const LINK_COUNT: usize = 12;

/// Rating points a full link is worth in its own situations (agreed,
/// question 13).
pub const FULL: f64 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Link {
    Highpointer,
    Burner,
    Sudden,
    LowCenter,
    BatteringRam,
    Length,
    BigMover,
    Anchor,
    GetOff,
    Bend,
    Lungs,
    PocketFrame,
}

/// One body trait a link is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Height,
    /// Short for his position.
    Short,
    Weight,
    /// Heavy for his height.
    HeavyForHeight,
    Arm,
    Wingspan,
    Hand,
    /// The forty.
    Speed,
    Acceleration,
    Agility,
    Balance,
    Strength,
    Flexibility,
    Jumping,
    /// Jumping power, the broad jump.
    Explosion,
    Stamina,
    Recovery,
}

impl Link {
    pub const ALL: [Link; LINK_COUNT] = [
        Link::Highpointer,
        Link::Burner,
        Link::Sudden,
        Link::LowCenter,
        Link::BatteringRam,
        Link::Length,
        Link::BigMover,
        Link::Anchor,
        Link::GetOff,
        Link::Bend,
        Link::Lungs,
        Link::PocketFrame,
    ];

    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Link::Highpointer => "Highpointer",
            Link::Burner => "Burner",
            Link::Sudden => "Sudden",
            Link::LowCenter => "Low center of gravity",
            Link::BatteringRam => "Battering ram",
            Link::Length => "Length",
            Link::BigMover => "Big man who moves",
            Link::Anchor => "Anchor",
            Link::GetOff => "Get-off",
            Link::Bend => "Bend",
            Link::Lungs => "Lungs",
            Link::PocketFrame => "Pocket frame",
        }
    }

    /// Where it gives the edge, in the agreed words.
    pub fn edge(self) -> &'static str {
        match self {
            Link::Highpointer => {
                "Balls in the air: jump balls, back-shoulder throws, fades in the red zone; a tall corner or safety plays the ball at its highest point."
            }
            Link::Burner => {
                "Separation on deep routes, breakaway runs and returns, a beaten defender's recovery, pursuit."
            }
            Link::Sudden => {
                "Changing direction without slowing: separation at the break, making the first man miss, mirroring a route."
            }
            Link::LowCenter => {
                "Hard to knock over: staying up through contact, the low man winning leverage."
            }
            Link::BatteringRam => {
                "Contact at speed: breaking arm tackles, wearing a defense down late; on defense, the thumper's hits."
            }
            Link::Length => {
                "First contact in blocking and rushing, press at the line, batted passes, tackling reach, the catch window."
            }
            Link::BigMover => {
                "Pass sets against speed, pulling and reaching the second level; on defense, shooting gaps and splitting double teams."
            }
            Link::Anchor => "The bull rush and double teams, on both sides; short-yardage push.",
            Link::GetOff => "The first step off the snap.",
            Link::Bend => {
                "Dipping under the tackle's hands around the corner without slowing; a tight turn at speed."
            }
            Link::Lungs => "Staying fresh late in drives and games, fewer breaks, the fourth quarter.",
            Link::PocketFrame => {
                "Seeing over the line, fewer batted passes, shrugging off arm tackles in the pocket."
            }
        }
    }

    pub fn parts(self) -> &'static [Part] {
        use Part::*;
        match self {
            Link::Highpointer => &[Height, Arm, Jumping],
            Link::Burner => &[Speed, Acceleration],
            Link::Sudden => &[Acceleration, Agility, Balance],
            Link::LowCenter => &[Short, HeavyForHeight, Balance, Strength],
            Link::BatteringRam => &[Weight, Speed, Strength],
            Link::Length => &[Arm, Wingspan, Hand],
            Link::BigMover => &[Weight, Acceleration, Agility],
            Link::Anchor => &[Weight, Strength, Balance],
            Link::GetOff => &[Acceleration, Explosion],
            Link::Bend => &[Flexibility, Balance, Speed],
            Link::Lungs => &[Stamina, Recovery],
            Link::PocketFrame => &[Height, Weight, Hand],
        }
    }

    /// Whether a position can carry the link: Big man who moves is judged
    /// among linemen, Pocket frame is a quarterback's; kickers, punters and
    /// snappers carry none.
    pub fn fits(self, pos: Position) -> bool {
        if pos.is_specialist() {
            return false;
        }
        match self {
            Link::BigMover => matches!(pos, Position::OT | Position::IOL | Position::DT),
            Link::PocketFrame => pos == Position::QB,
            _ => true,
        }
    }
}

/// A player's links, each 0 (none) to 1 (full).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Links(pub [f32; LINK_COUNT]);

impl Links {
    #[inline]
    pub fn get(&self, l: Link) -> f64 {
        self.0[l.index()] as f64
    }

    pub fn set(&mut self, l: Link, v: f64) {
        self.0[l.index()] = clamp(v, 0.0, 1.0) as f32;
    }

    pub fn any(&self) -> bool {
        self.0.iter().any(|&v| v > 0.0)
    }

    /// The links he has, strongest first.
    pub fn list(&self) -> Vec<(Link, f64)> {
        let mut v: Vec<(Link, f64)> =
            Link::ALL.iter().map(|&l| (l, self.get(l))).filter(|x| x.1 > 0.0).collect();
        v.sort_by(|a, b| b.1.total_cmp(&a.1));
        v
    }

    /// His edge in rating points in a situation the given links act in:
    /// the strongest in full, the next at half, a third at a quarter.
    #[inline]
    pub fn edge(&self, links: &[Link]) -> f64 {
        let mut s = [0.0f64; 4];
        for (k, &l) in links.iter().take(4).enumerate() {
            s[k] = self.get(l);
        }
        FULL * stacked(&mut s[..links.len().min(4)])
    }

    /// The same as a 0-1 share of a full link (for edges not on the rating
    /// scale).
    #[inline]
    pub fn share(&self, links: &[Link]) -> f64 {
        self.edge(links) / FULL
    }
}

/// What his gifts are worth to a staff, in full links (D3): the strongest
/// counts in full and each next one a little less (a complete athlete is
/// worth more than one trick, but not twelve times more). Real staffs pay
/// first for what can't be coached: size, length, speed and burst.
pub fn gift(links: &Links) -> f64 {
    let mut v: Vec<f64> = links.0.iter().map(|&x| x as f64).filter(|&x| x > 0.0).collect();
    v.sort_by(|a, b| b.total_cmp(a));
    v.iter().zip([1.0, 0.6, 0.4, 0.3, 0.2, 0.15, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1]).map(|(x, w)| x * w).sum()
}

/// Stacking in one situation (agreed, question 15): the strongest counts
/// in full, the next at half, a third at a quarter.
pub fn stacked(s: &mut [f64]) -> f64 {
    s.sort_by(|a, b| b.total_cmp(a));
    s.iter().zip([1.0, 0.5, 0.25, 0.125]).map(|(v, w)| v * w).sum()
}

/// A link part's strength from his standing at his position (z against
/// the combine): nothing below about the top 30%, full near the top 5%,
/// smooth between.
fn part_strength(z: f64) -> f64 {
    const Z30: f64 = 0.5244;
    const Z05: f64 = 1.6449;
    let x = clamp((z - Z30) / (Z05 - Z30), 0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The real combine by position (nflverse 2000-2025,
/// `data/real-world/combine_by_position.csv`): height, weight, forty and
/// broad jump means and spreads, and the arm, hand and wingspan the body
/// model grows with height (`body.rs`).
#[derive(Clone, Copy, Debug)]
struct Combine {
    ht: (f64, f64),
    wt: (f64, f64),
    arm: (f64, f64),
    wing: (f64, f64),
    hand: (f64, f64),
    forty: (f64, f64),
    broad: (f64, f64),
}

const COMBINE_CSV: &str = include_str!("../../../data/real-world/combine_by_position.csv");

fn lineman(pos: Position) -> bool {
    matches!(pos, Position::OT | Position::IOL | Position::DT | Position::EDGE)
}

static COMBINE: LazyLock<[Combine; POSITION_COUNT]> = LazyLock::new(|| {
    let mut out = [Combine {
        ht: (73.0, 2.0),
        wt: (230.0, 20.0),
        arm: (31.5, 1.2),
        wing: (76.0, 2.6),
        hand: (9.4, 0.4),
        forty: (4.8, 0.15),
        broad: (112.0, 6.0),
    }; POSITION_COUNT];
    for line in COMBINE_CSV.lines().skip(1) {
        let f: Vec<&str> = line.split(',').collect();
        let Some(pos) = Position::ALL.iter().copied().find(|p| p.name() == f[0]) else { continue };
        let num = |k: usize| f.get(k).and_then(|s| s.parse::<f64>().ok());
        let c = &mut out[pos.index()];
        if let (Some(h), Some(hs)) = (num(2), num(3)) {
            c.ht = (h, hs);
            let off = if lineman(pos) {
                0.3
            } else if pos == Position::QB {
                -0.5
            } else {
                0.0
            };
            let arm = (0.425 * h + off, ((0.425 * hs).powi(2) + 0.8f64.powi(2)).sqrt());
            c.arm = arm;
            c.wing = (2.03 * arm.0 + 12.3, ((2.03 * arm.1).powi(2) + 1.0).sqrt());
            let hand = 9.45 + 0.08 * (h - 74.0) + if lineman(pos) { 0.35 } else { 0.0 };
            c.hand = (hand, ((0.08 * hs).powi(2) + 0.35f64.powi(2)).sqrt());
        }
        if let (Some(w), Some(ws)) = (num(4), num(5)) {
            c.wt = (w, ws);
        }
        // The tested spread less the test-day noise (`body::combine`), so
        // a player's standing is his ability, not one run.
        if let (Some(m), Some(sd)) = (num(6), num(7)) {
            c.forty = (m, (sd * sd - 0.05 * 0.05).max(0.003).sqrt());
        }
        if let (Some(m), Some(sd)) = (num(12), num(13)) {
            c.broad = (m, (sd * sd - 9.0).max(4.0).sqrt());
        }
    }
    out
});

/// The athletic traits the combine has no test for, read against the
/// position's combine class as the game makes it: draft prospects at about
/// 22, near their grown ratings (the player model's own calibration).
const NORM_ATTRS: [Attr; 7] = [
    Attr::Acceleration,
    Attr::Agility,
    Attr::Balance,
    Attr::Strength,
    Attr::Flexibility,
    Attr::Jumping,
    Attr::Stamina,
];

static NORMS: LazyLock<[[(f64, f64); NORM_ATTRS.len()]; POSITION_COUNT]> = LazyLock::new(|| {
    use crate::player::{ProspectOptions, generate_prospect};
    let mut out = [[(60.0, 10.0); NORM_ATTRS.len()]; POSITION_COUNT];
    for pos in Position::ALL {
        if pos.is_specialist() {
            continue;
        }
        let mut rng = Rng::new(0x4C1A_0000 ^ pos.index() as u32);
        let n = 300;
        let mut sum = [0.0f64; NORM_ATTRS.len()];
        let mut sq = [0.0f64; NORM_ATTRS.len()];
        for k in 0..n {
            let target = clamp(70.0 + rng.normal(0.0, 5.0), 50.0, 95.0);
            let p = generate_prospect(
                &mut rng,
                ProspectOptions {
                    id: Some(0x7F00_0000 + (pos.index() as u32) * 1000 + k),
                    position: Some(pos),
                    age: Some(14),
                    talent_z: Some(crate::game::synthetic::z_for(target)),
                },
            );
            for (j, &a) in NORM_ATTRS.iter().enumerate() {
                let v = 0.96 * p.ceilings.get(a);
                sum[j] += v;
                sq[j] += v * v;
            }
        }
        for j in 0..NORM_ATTRS.len() {
            let m = sum[j] / n as f64;
            out[pos.index()][j] = (m, (sq[j] / n as f64 - m * m).max(4.0).sqrt());
        }
    }
    out
});

fn norm_z(pos: Position, a: Attr, v: f64) -> f64 {
    let j = NORM_ATTRS.iter().position(|&x| x == a).unwrap_or(0);
    let (m, sd) = NORMS[pos.index()][j];
    (v - m) / sd
}

/// His forty from speed, burst and weight, without the test-day noise
/// (the combine's fit, `body::combine`).
pub fn forty(attrs: &Attributes, weight: f64) -> f64 {
    5.56 - 0.0124 * (0.75 * attrs.get(Attr::Speed) + 0.25 * attrs.get(Attr::Acceleration)) + 0.0033 * (weight - 220.0)
}

fn burst(attrs: &Attributes) -> f64 {
    attrs.get(Attr::Jumping).max(0.95 * attrs.get(Attr::Acceleration))
}

/// His vertical jump, inches, without the test-day noise.
pub fn vertical(attrs: &Attributes, weight: f64) -> f64 {
    20.9 + 0.182 * burst(attrs) - 0.035 * (weight - 220.0)
}

/// His broad jump, inches, without the test-day noise.
pub fn broad(attrs: &Attributes, weight: f64) -> f64 {
    87.6 + 0.448 * burst(attrs) - 0.089 * (weight - 220.0)
}

/// His standing at his position in one part (z against the combine).
pub fn part_z(part: Part, pos: Position, b: &Body, attrs: &Attributes, recovery: f64) -> f64 {
    let c = &COMBINE[pos.index()];
    let z = |v: f64, (m, sd): (f64, f64)| (v - m) / sd;
    match part {
        Part::Height => z(b.height, c.ht),
        Part::Short => -z(b.height, c.ht),
        Part::Weight => z(b.weight, c.wt),
        Part::HeavyForHeight => (z(b.weight, c.wt) - 0.5 * z(b.height, c.ht)) / 0.866,
        Part::Arm => z(b.arm, c.arm),
        Part::Wingspan => z(b.wingspan, c.wing),
        Part::Hand => z(b.hand, c.hand),
        Part::Speed => -z(forty(attrs, b.weight), c.forty),
        Part::Explosion => z(broad(attrs, b.weight), c.broad),
        Part::Acceleration => norm_z(pos, Attr::Acceleration, attrs.get(Attr::Acceleration)),
        Part::Agility => norm_z(pos, Attr::Agility, attrs.get(Attr::Agility)),
        Part::Balance => norm_z(pos, Attr::Balance, attrs.get(Attr::Balance)),
        Part::Strength => norm_z(pos, Attr::Strength, attrs.get(Attr::Strength)),
        Part::Flexibility => norm_z(pos, Attr::Flexibility, attrs.get(Attr::Flexibility)),
        Part::Jumping => norm_z(pos, Attr::Jumping, attrs.get(Attr::Jumping)),
        Part::Stamina => norm_z(pos, Attr::Stamina, attrs.get(Attr::Stamina)),
        Part::Recovery => (recovery - 10.0) / 3.5,
    }
}

/// One link's strength, 0-1: the weakest of its parts.
pub fn strength(link: Link, pos: Position, b: &Body, attrs: &Attributes, recovery: f64) -> f64 {
    if !link.fits(pos) {
        return 0.0;
    }
    link.parts().iter().map(|&p| part_strength(part_z(p, pos, b, attrs, recovery))).fold(1.0, f64::min)
}

/// Every link from a body and traits, measured at a position.
pub fn measure(pos: Position, b: &Body, attrs: &Attributes, recovery: f64) -> Links {
    let mut out = Links::default();
    if pos.is_specialist() {
        return out;
    }
    for l in Link::ALL {
        out.set(l, strength(l, pos, b, attrs, recovery));
    }
    out
}

/// A player's links today, at his listed position.
pub fn links_of(p: &Player) -> Links {
    measure(p.position, &body_now(p), &p.attributes, p.game.recovery)
}

/// A player's links at another position (a position move reads him
/// against that position's combine).
pub fn links_at(p: &Player, pos: Position) -> Links {
    measure(pos, &body_now(p), &p.attributes, p.game.recovery)
}

/// Words for a link's strength.
pub fn strength_words(s: f64) -> &'static str {
    if s >= 0.85 {
        "full"
    } else if s >= 0.5 {
        "strong"
    } else if s >= 0.2 {
        "partial"
    } else {
        "a hint"
    }
}

/// The measured numbers a staff names a link by: "6'4\", 33 3/4\" arms,
/// 38\" vertical".
pub fn numbers(link: Link, b: &Body, attrs: &Attributes) -> String {
    use crate::body::height_text;
    let quarter = |x: f64| {
        let q = (x * 4.0).round() / 4.0;
        let whole = q.floor();
        let frac = match ((q - whole) * 4.0).round() as i32 {
            1 => " 1/4",
            2 => " 1/2",
            3 => " 3/4",
            _ => "",
        };
        format!("{}{}\"", whole as i64, frac)
    };
    let half = |x: f64| {
        let h = (x * 2.0).round() / 2.0;
        if h.fract() == 0.0 { format!("{}\"", h as i64) } else { format!("{}\"", h) }
    };
    let ht = height_text(b.height);
    let wt = format!("{} lb", b.weight.round() as i64);
    let forty = format!("{:.2} forty", forty(attrs, b.weight));
    let vert = format!("{} vertical", half(vertical(attrs, b.weight)));
    let broad_in = broad(attrs, b.weight).round() as i64;
    let broad = format!("{}'{}\" broad jump", broad_in / 12, broad_in % 12);
    let arms = format!("{} arms", quarter(b.arm));
    let wing = format!("{} wingspan", quarter(b.wingspan));
    let hand = format!("{} hands", quarter(b.hand));
    let split = format!("{:.2} ten-yard split", ten_split(attrs, b.weight));
    let parts: Vec<String> = match link {
        Link::Highpointer => vec![ht, arms, vert],
        Link::Burner => vec![forty, split],
        Link::Sudden => vec![split, format!("{:.2} short shuttle", shuttle(attrs, b.weight))],
        Link::LowCenter => vec![ht, wt],
        Link::BatteringRam => vec![wt, forty],
        Link::Length => vec![arms, wing, hand],
        Link::BigMover => vec![wt, split],
        Link::Anchor => vec![wt, format!("{} reps", bench(attrs, b).round() as i64)],
        Link::GetOff => vec![split, broad],
        Link::Bend => vec![format!("{:.2} three-cone", cone(attrs, b.weight)), forty],
        Link::Lungs => vec!["fresh in the fourth quarter".to_string()],
        Link::PocketFrame => vec![ht, wt, hand],
    };
    parts.join(", ")
}

/// The ten-yard split (about 0.345 of the forty for a fast man, more for a
/// slow starter), the shuttle, the three-cone and the bench, without the
/// test-day noise (the combine's fits, `body::combine`).
pub fn ten_split(attrs: &Attributes, weight: f64) -> f64 {
    let tau = clamp(0.78 + 0.003 * (60.0 - attrs.get(Attr::Acceleration)) + 0.0006 * (weight - 220.0), 0.55, 1.0);
    forty(attrs, weight) * (0.29 + 0.07 * tau)
}

fn shuttle(attrs: &Attributes, weight: f64) -> f64 {
    let quick = 0.7 * attrs.get(Attr::Agility) + 0.3 * attrs.get(Attr::Acceleration);
    4.59 - 0.0045 * quick + 0.0039 * (weight - 220.0)
}

fn cone(attrs: &Attributes, weight: f64) -> f64 {
    let bend = 0.55 * attrs.get(Attr::Agility) + 0.25 * attrs.get(Attr::Acceleration) + 0.2 * attrs.get(Attr::Flexibility);
    7.58 - 0.007 * bend + 0.0062 * (weight - 220.0)
}

fn bench(attrs: &Attributes, b: &Body) -> f64 {
    clamp(8.05 + 0.137 * attrs.get(Attr::Strength) + 0.073 * (b.weight - 220.0) - 0.8 * (b.arm - 32.5), 0.0, 50.0)
}

/// How often each link is at least partial (above 0.2) and full (0.85) in
/// a group of players, for the checks.
pub fn prevalence(links: &[Links]) -> [(f64, f64); LINK_COUNT] {
    let n = links.len().max(1) as f64;
    let mut out = [(0.0, 0.0); LINK_COUNT];
    for l in links {
        for (k, &v) in l.0.iter().enumerate() {
            if v >= 0.2 {
                out[k].0 += 1.0 / n;
            }
            if v >= 0.85 {
                out[k].1 += 1.0 / n;
            }
        }
    }
    out
}
