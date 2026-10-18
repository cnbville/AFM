//! Depth-chart spots (fix list topic 3): the real spots inside each listed
//! position. A team lists a left and a right tackle, guards and a center, a
//! nose tackle, a middle and outside linebackers, a slot corner, a free and
//! a strong safety, and a fullback. Every player is rated at each spot of
//! his group from his attributes; a staff sets its spots each spring, and a
//! man asked to play a spot he has never been listed at plays below his
//! rating there (3.3). The spot leans are proposed, not confirmed.

use crate::attributes::Attr::{self, *};
use crate::attributes::Attributes;
use crate::player::Player;
use crate::positions::{Position, overall_for};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Spot {
    #[default]
    QB,
    RB,
    FB,
    WR,
    TE,
    LT,
    RT,
    G,
    C,
    EDGE,
    DT,
    NT,
    MLB,
    OLB,
    CB,
    NB,
    FS,
    SS,
    K,
    P,
    LS,
}

pub const SPOT_COUNT: usize = 21;

impl Spot {
    pub const ALL: [Spot; SPOT_COUNT] = [
        Spot::QB,
        Spot::RB,
        Spot::FB,
        Spot::WR,
        Spot::TE,
        Spot::LT,
        Spot::RT,
        Spot::G,
        Spot::C,
        Spot::EDGE,
        Spot::DT,
        Spot::NT,
        Spot::MLB,
        Spot::OLB,
        Spot::CB,
        Spot::NB,
        Spot::FS,
        Spot::SS,
        Spot::K,
        Spot::P,
        Spot::LS,
    ];

    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        [
            "QB", "RB", "FB", "WR", "TE", "LT", "RT", "G", "C", "EDGE", "DT", "NT", "MLB", "OLB", "CB", "NB", "FS",
            "SS", "K", "P", "LS",
        ][self as usize]
    }

    /// The listed position this spot belongs to.
    pub fn position(self) -> Position {
        use Position as P;
        match self {
            Spot::QB => P::QB,
            Spot::RB | Spot::FB => P::RB,
            Spot::WR => P::WR,
            Spot::TE => P::TE,
            Spot::LT | Spot::RT => P::OT,
            Spot::G | Spot::C => P::IOL,
            Spot::EDGE => P::EDGE,
            Spot::DT | Spot::NT => P::DT,
            Spot::MLB | Spot::OLB => P::LB,
            Spot::CB | Spot::NB => P::CB,
            Spot::FS | Spot::SS => P::S,
            Spot::K => P::K,
            Spot::P => P::P,
            Spot::LS => P::LS,
        }
    }

    #[inline]
    pub fn bit(self) -> u32 {
        1 << self as u32
    }
}

/// The spots of a listed position.
pub fn spots_of(pos: Position) -> &'static [Spot] {
    use Position as P;
    match pos {
        P::QB => &[Spot::QB],
        P::RB => &[Spot::RB, Spot::FB],
        P::WR => &[Spot::WR],
        P::TE => &[Spot::TE],
        P::OT => &[Spot::LT, Spot::RT],
        P::IOL => &[Spot::G, Spot::C],
        P::EDGE => &[Spot::EDGE],
        P::DT => &[Spot::DT, Spot::NT],
        P::LB => &[Spot::MLB, Spot::OLB],
        P::CB => &[Spot::CB, Spot::NB],
        P::S => &[Spot::FS, Spot::SS],
        P::K => &[Spot::K],
        P::P => &[Spot::P],
        P::LS => &[Spot::LS],
    }
}

/// The position groups of 3.1: a player is listed only inside his own group
/// (OL takes tackles, guards and centers; DL ends and tackles; DB corners
/// and safeties).
pub fn group(pos: Position) -> u8 {
    use Position as P;
    match pos {
        P::QB => 0,
        P::RB => 1,
        P::WR => 2,
        P::TE => 3,
        P::OT | P::IOL => 4,
        P::EDGE | P::DT => 5,
        P::LB => 6,
        P::CB | P::S => 7,
        P::K => 8,
        P::P => 9,
        P::LS => 10,
    }
}

/// What a spot asks beyond its position's overall: the left tackle's feet
/// and bend against speed rushers, the right tackle's run game, the guard's
/// anchor and pulls, the center's calls and shotgun snap, the middle
/// linebacker's reads (Cain: as hard as quarterback), the free safety's
/// range, the strong safety's run support, the nickel's quickness, the nose
/// tackle's size and anchor.
fn lean(spot: Spot) -> &'static [(Attr, f64)] {
    match spot {
        Spot::RB => &[(BallCarrierVision, 0.3), (Elusiveness, 0.3), (Speed, 0.25), (Balance, 0.15)],
        Spot::LT => &[(PassBlocking, 0.4), (Agility, 0.2), (Flexibility, 0.15), (HandUse, 0.15), (Speed, 0.1)],
        Spot::RT => &[(RunBlocking, 0.4), (Strength, 0.25), (PassBlocking, 0.2), (Anchor, 0.15)],
        Spot::G => &[(RunBlocking, 0.35), (Strength, 0.3), (Anchor, 0.25), (BlockingInSpace, 0.1)],
        Spot::C => &[
            (FootballIq, 0.25),
            (Communication, 0.25),
            (ShotgunSnap, 0.2),
            (RunBlocking, 0.1),
            (PassBlocking, 0.1),
            (Agility, 0.1),
        ],
        Spot::DT => &[(Acceleration, 0.35), (FinesseRush, 0.2), (PowerRush, 0.15), (Agility, 0.15), (HandUse, 0.15)],
        Spot::NT => &[(Anchor, 0.4), (Strength, 0.35), (BlockShedding, 0.25)],
        Spot::MLB => &[
            (PlayRecognition, 0.3),
            (FootballIq, 0.25),
            (Communication, 0.15),
            (Tackling, 0.15),
            (Traffic, 0.15),
        ],
        Spot::OLB => &[(Speed, 0.35), (ZoneCoverage, 0.25), (Blitzing, 0.2), (Pursuit, 0.2)],
        Spot::CB => &[(Speed, 0.35), (ManCoverage, 0.35), (Press, 0.15), (Jumping, 0.15)],
        Spot::NB => &[
            (Agility, 0.35),
            (Acceleration, 0.25),
            (ManCoverage, 0.15),
            (Anticipation, 0.1),
            (Tackling, 0.1),
            (Blitzing, 0.05),
        ],
        Spot::FS => &[
            (Speed, 0.35),
            (ZoneCoverage, 0.3),
            (PlayRecognition, 0.15),
            (BallTracking, 0.1),
            (Communication, 0.1),
        ],
        Spot::SS => &[(Tackling, 0.35), (PlayRecognition, 0.25), (Strength, 0.15), (Traffic, 0.15), (Physicality, 0.1)],
        _ => &[],
    }
}

/// How much a spot's lean moves the rating, per point of lean above his
/// group's other spot.
const LEAN: f64 = 0.35;

fn lean_value(spot: Spot, a: &Attributes, weight: f64) -> f64 {
    let v: f64 = lean(spot).iter().map(|&(x, w)| a[x] * w).sum();
    // A nose tackle's size: 330 pounds counts like a 90.
    if spot == Spot::NT { 0.8 * v + 0.2 * ((weight - 240.0) / 1.0).clamp(20.0, 99.0) } else { v }
}

/// His rating at a spot, in the position's rating points.
pub fn spot_rating(spot: Spot, a: &Attributes, weight: f64) -> f64 {
    match spot {
        // The fullback is a blocker who catches: half a tight end's game,
        // half lead blocking.
        Spot::FB => {
            let lead = 0.35 * a[BlockingInSpace]
                + 0.15 * a[RunBlocking]
                + 0.15 * a[Strength]
                + 0.1 * a[Anchor]
                + 0.05 * a[LegDrive]
                + 0.2 * a[Catching];
            0.5 * overall_for(Position::TE, a) + 0.5 * lead
        }
        _ => {
            let pos = spot.position();
            let base = overall_for(pos, a);
            let sibs: Vec<Spot> = spots_of(pos).iter().copied().filter(|&s| s != spot && s != Spot::FB).collect();
            if sibs.is_empty() || lean(spot).is_empty() {
                return base;
            }
            let other = sibs.iter().map(|&s| lean_value(s, a, weight)).sum::<f64>() / sibs.len() as f64;
            base + LEAN * (lean_value(spot, a, weight) - other) / 2.0
        }
    }
}

/// Rating points a man listed at `from` loses at a spot he has never been
/// listed at (3.3): footwork from the other side, and at center the snap
/// and the line calls, at middle linebacker the defensive calls. Interior
/// tackles move between the nose and the three-technique freely, safeties
/// nearly so (proposed, not confirmed).
pub fn fill_in(spot: Spot, from: Spot) -> f64 {
    match (spot, from) {
        (Spot::DT, Spot::NT) | (Spot::NT, Spot::DT) => 0.0,
        (Spot::FS, Spot::SS) | (Spot::SS, Spot::FS) => 1.5,
        (Spot::C, _) => 6.0,
        (Spot::MLB, _) => 4.0,
        (Spot::QB | Spot::K | Spot::P | Spot::LS, _) => 8.0,
        _ => 3.0,
    }
}

/// The spots a staff lists its starters at, by position, best player first.
pub fn starter_spots(pos: Position) -> &'static [Spot] {
    use Position as P;
    match pos {
        P::OT => &[Spot::LT, Spot::RT],
        P::IOL => &[Spot::C, Spot::G, Spot::G],
        P::DT => &[Spot::DT, Spot::NT],
        P::LB => &[Spot::MLB, Spot::OLB, Spot::OLB],
        P::CB => &[Spot::CB, Spot::CB, Spot::NB],
        P::S => &[Spot::FS, Spot::SS],
        _ => &[],
    }
}

/// A staff sets its spots (each spring, and for a new roster): the starters
/// at each position go to the spots that suit them best together, every
/// backup to his own best spot. Returns each player's spot, in order; the
/// caller lists him there (`set_spot`), and a spot once listed stays
/// familiar.
pub fn choose_spots(players: &[&Player]) -> Vec<Spot> {
    let mut out: Vec<Spot> = players.iter().map(|p| p.spot).collect();
    // A left-handed starter's blind side is his right: the best pass
    // protector plays right tackle (Tua's Dolphins, Young's 49ers).
    let lefty = players
        .iter()
        .filter(|p| p.position == Position::QB)
        .max_by(|a, b| a.overall().total_cmp(&b.overall()))
        .is_some_and(|q| q.body.left_handed);
    let blind = |s: Spot| match s {
        Spot::LT if lefty => Spot::RT,
        Spot::RT if lefty => Spot::LT,
        s => s,
    };
    let r = |p: &Player, s: Spot| spot_rating(blind(s), &p.attributes, p.body.weight);
    for pos in Position::ALL {
        let mut idx: Vec<usize> = (0..players.len()).filter(|&i| players[i].position == pos).collect();
        if idx.is_empty() {
            continue;
        }
        idx.sort_by(|&a, &b| players[b].overall().total_cmp(&players[a].overall()));
        let spots = starter_spots(pos);
        let k = spots.len().min(idx.len());
        let mut done = 0;
        if k > 1 {
            let order = best_order(&idx[..k], spots, &|i, s| r(players[i], s));
            for (j, i) in order.into_iter().enumerate() {
                if let Some(i) = i {
                    out[i] = spots[j];
                }
            }
            done = k;
        }
        for &i in &idx[done..] {
            // A fullback is a fullback only when he's clearly one.
            let v = |s: Spot| r(players[i], s) - if s == Spot::FB { 5.0 } else { 0.0 };
            out[i] = spots_of(pos).iter().copied().max_by(|&a, &b| v(a).total_cmp(&v(b))).unwrap_or(spots_of(pos)[0]);
        }
    }
    out
}

/// List him at a spot; the spot stays familiar.
pub fn set_spot(p: &mut Player, s: Spot) {
    p.spot = s;
    p.spots_played |= s.bit();
}

/// The best way to put `list` at `spots` (at most one man a spot): for
/// each spot, in order, the man who plays it. Every arrangement is tried
/// (at most 5 men); ties keep the given order.
pub fn best_order(list: &[usize], spots: &[Spot], value: &dyn Fn(usize, Spot) -> f64) -> Vec<Option<usize>> {
    struct Search<'a> {
        list: &'a [usize],
        spots: &'a [Spot],
        value: &'a dyn Fn(usize, Spot) -> f64,
        out: Vec<Option<usize>>,
        best: (f64, Vec<Option<usize>>),
    }
    impl Search<'_> {
        fn go(&mut self, j: usize, total: f64) {
            if j == self.list.len().min(self.spots.len()) {
                if total > self.best.0 {
                    self.best = (total, self.out.clone());
                }
                return;
            }
            for k in 0..self.spots.len() {
                if self.out[k].is_none() {
                    let v = (self.value)(self.list[j], self.spots[k]);
                    self.out[k] = Some(self.list[j]);
                    self.go(j + 1, total + v);
                    self.out[k] = None;
                }
            }
        }
    }
    let m = spots.len();
    let mut s = Search { list, spots, value, out: vec![None; m], best: (f64::MIN, vec![None; m]) };
    s.go(0, 0.0);
    s.best.1
}
