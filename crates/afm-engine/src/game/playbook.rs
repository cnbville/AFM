//! The playbook (topic 3): personnel, formations, routes, pass and run
//! concepts, fronts, coverages and pressures. A play is a full set of
//! assignments; the battles come out of them.

use super::geom::{V2, WIDTH, v2};

/// Offensive spots on one snap. Linemen are left tackle to right tackle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ORole {
    QB,
    RB,
    FB,
    /// Split end, on the weak side.
    X,
    /// Flanker, on the strong side.
    Z,
    Slot,
    Slot2,
    TE,
    TE2,
    TE3,
    LT,
    LG,
    C,
    RG,
    RT,
}

impl ORole {
    pub fn is_line(self) -> bool {
        matches!(self, ORole::LT | ORole::LG | ORole::C | ORole::RG | ORole::RT)
    }
    pub fn is_receiver(self) -> bool {
        matches!(self, ORole::X | ORole::Z | ORole::Slot | ORole::Slot2 | ORole::TE | ORole::TE2 | ORole::TE3)
    }
    pub fn is_back(self) -> bool {
        matches!(self, ORole::RB | ORole::FB)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Personnel {
    /// 1 back, 1 tight end, 3 receivers.
    P11,
    P12,
    P21,
    P10,
    P22,
    P13,
}

impl Personnel {
    pub fn roles(self) -> &'static [ORole] {
        use ORole::*;
        match self {
            Personnel::P11 => &[RB, X, Z, Slot, TE],
            Personnel::P12 => &[RB, X, Z, TE, TE2],
            Personnel::P21 => &[RB, FB, X, Z, TE],
            Personnel::P10 => &[RB, X, Z, Slot, Slot2],
            Personnel::P22 => &[RB, FB, X, TE, TE2],
            Personnel::P13 => &[RB, X, TE, TE2, TE3],
        }
    }
    pub fn name(self) -> &'static str {
        ["11", "12", "21", "10", "22", "13"][self as usize]
    }
    pub fn wide_receivers(self) -> usize {
        self.roles().iter().filter(|r| matches!(r, ORole::X | ORole::Z | ORole::Slot | ORole::Slot2)).count()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Backfield {
    UnderCenter,
    Shotgun,
    Pistol,
}

/// How the receivers stand (plays and tactics topic 1): two by two, three to
/// one side, a bunch (three in a cluster, which picks man defenders), a
/// stack (one behind the other), empty (the back split out) or a wing (a
/// tight end or back off the line outside the end man).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Set {
    Doubles,
    Trips,
    Bunch,
    Stack,
    Empty,
    Wing,
}

pub const SET_COUNT: usize = 6;

impl Set {
    pub fn name(self) -> &'static str {
        ["doubles", "trips", "bunch", "stack", "empty", "wing"][self as usize]
    }
}

/// Motion before the snap (topic 3.1): a jet man crossing at speed, an
/// orbit behind the quarterback, a receiver moving across the formation, or
/// a late shift of the strength.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Motion {
    None,
    Jet,
    Orbit,
    Across,
    Shift,
}

impl Motion {
    pub fn name(self) -> &'static str {
        ["", "jet motion", "orbit motion", "motion across", "shift"][self as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Formation {
    pub personnel: Personnel,
    pub backfield: Backfield,
    pub set: Set,
    /// +1: strength to the offense's left (higher y), -1: to the right.
    pub strength: f64,
    pub motion: Motion,
}

impl Formation {
    /// A plain two-by-two look with no motion.
    pub fn plain(personnel: Personnel, backfield: Backfield, strength: f64) -> Formation {
        Formation { personnel, backfield, set: Set::Doubles, strength, motion: Motion::None }
    }
    /// Three receivers to the strength.
    pub fn trips(&self) -> bool {
        matches!(self.set, Set::Trips | Set::Bunch)
    }
    /// The man who goes in motion: the slot, else the flanker.
    pub fn motion_man(&self) -> Option<ORole> {
        if self.motion == Motion::None {
            return None;
        }
        let roles = self.personnel.roles();
        if self.motion == Motion::Orbit || self.motion == Motion::Shift {
            return roles.iter().copied().find(|r| matches!(r, ORole::TE)).or(Some(ORole::Z));
        }
        roles.iter().copied().find(|r| *r == ORole::Slot).or(Some(ORole::Z))
    }
    /// A key for the look the defense sees (tendencies, topic 7): personnel,
    /// backfield and set.
    pub fn look(&self) -> usize {
        (self.personnel as usize * 3 + self.backfield as usize) * SET_COUNT + self.set as usize
    }
}

pub const LOOK_COUNT: usize = 6 * 3 * SET_COUNT;

/// Where a receiver lines up wide on a side, from the sideline in.
fn wide_y(side: f64, ball_y: f64, from_sideline: f64) -> f64 {
    let y = if side > 0.0 { WIDTH - from_sideline } else { from_sideline };
    // Never closer to the ball than 7 yards.
    if side > 0.0 { y.max(ball_y + 7.0) } else { y.min(ball_y - 7.0) }
}

/// Alignment of every spot: offsets from the ball, x back from the line.
/// Motion is settled before this: an "across" man already stands on his new
/// side, a jet man is caught mid-motion behind the line.
pub fn align(f: &Formation, role: ORole, los: f64, ball_y: f64) -> V2 {
    let s = f.strength;
    let at = |dx: f64, dy: f64| v2(los + dx, ball_y + dy);
    // The jet man at the snap: a step behind the line, just short of the
    // ball, at full speed across (an orbit man is behind the quarterback).
    if f.motion_man() == Some(role) {
        match f.motion {
            Motion::Jet => return at(-1.6, 2.2 * s),
            Motion::Orbit if role != ORole::TE => return at(-6.5, -1.5 * s),
            _ => {}
        }
    }
    // A bunch: the point man on the line about eight yards outside the
    // ball, two more a yard and a half behind on either side of him. The
    // third man is the second slot, else the tight end (detached).
    if f.set == Set::Bunch {
        let third = if f.personnel.roles().contains(&ORole::Slot2) { ORole::Slot2 } else { ORole::TE };
        let py = ball_y + s * 8.5;
        match role {
            ORole::Slot => return v2(los - 0.8, py),
            ORole::Z => return v2(los - 2.3, py + 1.4 * s),
            r if r == third => return v2(los - 2.3, py - 1.4 * s),
            _ => {}
        }
    }
    // A stack: the slot directly behind the flanker.
    if f.set == Set::Stack && role == ORole::Slot {
        return v2(los - 2.6, wide_y(s, ball_y, 11.0));
    }
    if f.set == Set::Stack && role == ORole::Z {
        return v2(los - 0.8, wide_y(s, ball_y, 11.0));
    }
    // Empty: the back splits out to the weak slot.
    if f.set == Set::Empty && role == ORole::RB {
        let x = wide_y(-s, ball_y, 9.0);
        return v2(los - 1.0, ball_y + 0.5 * (x - ball_y));
    }
    // A wing: the second tight end (or the fullback) a step outside and
    // behind the first.
    if f.set == Set::Wing && matches!(role, ORole::TE2 | ORole::FB) {
        let roles = f.personnel.roles();
        let wing = if roles.contains(&ORole::TE2) { ORole::TE2 } else { ORole::FB };
        if role == wing {
            return at(-1.3, 5.6 * s);
        }
    }
    match role {
        ORole::C => at(-0.4, 0.0),
        ORole::LG => at(-0.6, 1.4),
        ORole::RG => at(-0.6, -1.4),
        ORole::LT => at(-0.8, 2.8),
        ORole::RT => at(-0.8, -2.8),
        ORole::QB => match f.backfield {
            Backfield::UnderCenter => at(-1.0, 0.0),
            Backfield::Shotgun => at(-5.0, 0.0),
            Backfield::Pistol => at(-4.0, 0.0),
        },
        ORole::RB => match f.backfield {
            Backfield::UnderCenter => at(-7.0, 0.0),
            Backfield::Shotgun => at(-5.0, -1.5 * s),
            Backfield::Pistol => at(-6.8, 0.0),
        },
        ORole::FB => at(-4.5, 0.0),
        ORole::TE => at(-0.8, 4.3 * s),
        ORole::TE2 => {
            if f.personnel == Personnel::P13 {
                at(-1.5, 5.5 * s)
            } else {
                at(-0.8, -4.3 * s)
            }
        }
        ORole::TE3 => at(-0.8, -4.3 * s),
        ORole::Z => v2(los - 1.2, wide_y(s, ball_y, 9.0)),
        ORole::X => v2(los - 0.8, wide_y(-s, ball_y, 9.0)),
        ORole::Slot => {
            let z = wide_y(s, ball_y, 9.0);
            v2(los - 1.0, ball_y + 0.45 * (z - ball_y) + 2.0 * s)
        }
        ORole::Slot2 => {
            if f.trips() {
                let z = wide_y(s, ball_y, 9.0);
                v2(los - 1.0, ball_y + 0.75 * (z - ball_y))
            } else {
                let x = wide_y(-s, ball_y, 9.0);
                v2(los - 1.0, ball_y + 0.5 * (x - ball_y) - 2.0 * s)
            }
        }
    }
}

/// Routes (topic 3). Depths in yards past the line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Route {
    Block,
    /// Block first; release to a checkdown if nobody comes.
    CheckRelease,
    Go,
    Fade,
    Seam,
    Slant,
    Out(f64),
    In(f64),
    Curl(f64),
    Hitch(f64),
    Comeback(f64),
    Post(f64),
    Corner(f64),
    Flat,
    Wheel,
    Drag,
    /// Find the hole in the zone at this depth.
    Sit(f64),
    Swing,
    Screen,
    Bubble,
    /// A receiver screen: a step back and inside, behind linemen who release.
    Tunnel,
    /// The back's angle route: out for two steps, then back inside.
    Angle,
}

impl Route {
    pub fn name(self) -> &'static str {
        match self {
            Route::Block => "block",
            Route::CheckRelease => "check-release",
            Route::Go => "go",
            Route::Fade => "fade",
            Route::Seam => "seam",
            Route::Slant => "slant",
            Route::Out(_) => "out",
            Route::In(_) => "dig",
            Route::Curl(_) => "curl",
            Route::Hitch(_) => "hitch",
            Route::Comeback(_) => "comeback",
            Route::Post(_) => "post",
            Route::Corner(_) => "corner",
            Route::Flat => "flat",
            Route::Wheel => "wheel",
            Route::Drag => "drag",
            Route::Sit(_) => "sit",
            Route::Swing => "swing",
            Route::Screen | Route::Tunnel => "screen",
            Route::Bubble => "bubble",
            Route::Angle => "angle",
        }
    }

    /// Waypoints from the receiver's start, as (downfield, outside) pairs,
    /// and whether he settles at the end (true) or keeps running.
    pub fn shape(self) -> (&'static [(f64, f64)], bool) {
        match self {
            Route::Block | Route::CheckRelease | Route::Screen => (&[], true),
            Route::Go => (&[(45.0, 1.5)], false),
            Route::Fade => (&[(3.0, 1.0), (40.0, 4.0)], false),
            Route::Seam => (&[(45.0, -1.0)], false),
            Route::Slant => (&[(2.0, 0.0), (9.0, -7.0), (14.0, -14.0)], false),
            Route::Out(_) => (&[(0.0, 0.0), (0.0, 10.0), (4.0, 11.0)], false),
            Route::In(_) => (&[(0.0, 0.0), (0.0, -20.0)], false),
            Route::Curl(_) => (&[(0.0, 0.0), (-2.0, -1.5)], true),
            Route::Hitch(_) => (&[(0.0, 0.0), (-1.0, 0.0)], true),
            Route::Comeback(_) => (&[(0.0, 0.0), (-3.0, 3.0)], true),
            Route::Post(_) => (&[(0.0, 0.0), (16.0, -16.0)], false),
            Route::Corner(_) => (&[(0.0, 0.0), (14.0, 11.0)], false),
            Route::Flat => (&[(1.5, 3.0), (3.0, 9.0), (4.0, 12.5), (14.0, 13.5)], false),
            Route::Wheel => (&[(1.0, 4.0), (4.0, 8.0), (40.0, 9.0)], false),
            Route::Drag => (&[(2.5, 0.0), (3.5, -30.0)], false),
            Route::Sit(_) => (&[(0.0, 0.0)], true),
            Route::Swing => (&[(0.5, 4.0), (2.0, 8.0)], true),
            Route::Bubble => (&[(-1.0, 3.0), (-0.5, 6.0), (10.0, 7.0)], false),
            Route::Tunnel => (&[(-1.5, -1.5), (-1.0, -5.0)], true),
            Route::Angle => (&[(1.0, 3.0), (2.0, 4.0), (5.0, 0.5), (6.5, -6.0)], false),
        }
    }

    /// The stem depth for routes that take one.
    pub fn depth(self) -> f64 {
        match self {
            Route::Out(d)
            | Route::In(d)
            | Route::Curl(d)
            | Route::Hitch(d)
            | Route::Comeback(d)
            | Route::Post(d)
            | Route::Corner(d)
            | Route::Sit(d) => d,
            _ => 0.0,
        }
    }

    /// Absolute waypoints from a start point; `out` is +1 when outside is
    /// toward higher y.
    pub fn points(self, start: V2, los: f64, out: f64) -> (Vec<V2>, bool) {
        let (shape, settle) = self.shape();
        let d = self.depth();
        let pts = shape
            .iter()
            .map(|&(dx, dy)| {
                let x = if d > 0.0 { los + d + dx } else { start.x + 1.0 + dx };
                // Receivers keep room to work at the sideline, as they are
                // coached to (two and a half yards; it was one, and outs
                // ended on the line).
                v2(x, (start.y + dy * out).clamp(2.5, WIDTH - 2.5))
            })
            .collect();
        (pts, settle)
    }

    pub fn blocks(self) -> bool {
        matches!(self, Route::Block)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drop {
    /// Three-step or catch-and-throw from the gun.
    Quick,
    /// Five-step.
    Medium,
    /// Seven-step or a long play-action fake.
    Deep,
    /// Boot or rollout away from the run fake.
    Boot,
    /// Fake a drop, let the rush through, dump it to the back.
    Screen,
    /// Catch the snap and throw at once to a receiver running wide.
    Bubble,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protection {
    /// Five linemen, the back checks before releasing.
    Standard,
    /// Six: the back stays in.
    Six,
    /// Seven: back and tight end stay in (shot plays).
    Max,
}

pub struct PassConcept {
    pub name: &'static str,
    pub drop: Drop,
    pub protection: Protection,
    pub play_action: bool,
    pub routes: &'static [(ORole, Route)],
    /// The quarterback's progression, first read first.
    pub reads: &'static [ORole],
    /// Shorter list on goal-line and short yardage.
    pub depth: PassDepth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassDepth {
    Screen,
    Quick,
    Medium,
    Deep,
}

use ORole::*;
use Route::*;

pub const PASS_CONCEPTS: &[PassConcept] = &[
    PassConcept {
        name: "slant-flat",
        drop: Drop::Quick,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(X, Slant), (Z, Slant), (Slot, Flat), (Slot2, Slant), (TE, Sit(6.0)), (TE2, Flat)],
        reads: &[Z, X, Slot, TE, RB],
        depth: PassDepth::Quick,
    },
    PassConcept {
        name: "stick",
        drop: Drop::Quick,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(X, Go), (Z, Go), (Slot, Sit(6.0)), (Slot2, Hitch(6.0)), (TE, Flat), (TE2, Sit(6.0))],
        reads: &[Slot, TE, Slot2, X, RB],
        depth: PassDepth::Quick,
    },
    PassConcept {
        name: "hitches",
        drop: Drop::Quick,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(X, Hitch(6.0)), (Z, Hitch(6.0)), (Slot, Seam), (Slot2, Seam), (TE, Sit(5.0)), (TE2, Flat)],
        reads: &[X, Z, Slot, TE, RB],
        depth: PassDepth::Quick,
    },
    PassConcept {
        name: "mesh",
        drop: Drop::Medium,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(X, Corner(11.0)), (Z, Go), (Slot, Drag), (TE, Drag), (Slot2, Drag), (TE2, Sit(8.0))],
        reads: &[Slot, TE, X, Slot2, RB],
        depth: PassDepth::Medium,
    },
    PassConcept {
        name: "smash",
        drop: Drop::Medium,
        protection: Protection::Standard,
        play_action: false,
        routes: &[
            (X, Hitch(6.0)),
            (Z, Hitch(6.0)),
            (Slot, Corner(11.0)),
            (Slot2, Corner(11.0)),
            (TE, Sit(9.0)),
            (TE2, Flat),
        ],
        reads: &[Slot, Z, Slot2, X, TE, RB],
        depth: PassDepth::Medium,
    },
    PassConcept {
        name: "flood",
        drop: Drop::Medium,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(Z, Go), (Slot, Out(11.0)), (TE, Flat), (X, In(14.0)), (Slot2, Out(11.0)), (TE2, Flat)],
        reads: &[Slot, TE, X, RB],
        depth: PassDepth::Medium,
    },
    PassConcept {
        name: "curl-flat",
        drop: Drop::Medium,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(X, Curl(12.0)), (Z, Curl(12.0)), (Slot, Flat), (Slot2, Flat), (TE, Sit(9.0)), (TE2, Out(5.0))],
        reads: &[Z, X, Slot, TE, RB],
        depth: PassDepth::Medium,
    },
    PassConcept {
        name: "dagger",
        drop: Drop::Deep,
        protection: Protection::Six,
        play_action: false,
        routes: &[(Slot, Seam), (Z, In(16.0)), (X, Go), (TE, Sit(7.0)), (Slot2, In(16.0)), (TE2, Out(6.0))],
        reads: &[Z, Slot, X, TE],
        depth: PassDepth::Deep,
    },
    PassConcept {
        name: "four verticals",
        drop: Drop::Deep,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(X, Go), (Z, Go), (Slot, Seam), (Slot2, Seam), (TE, Seam), (TE2, Out(5.0))],
        reads: &[Slot, Slot2, TE, Z, X, RB],
        depth: PassDepth::Deep,
    },
    PassConcept {
        name: "Y-cross",
        drop: Drop::Deep,
        protection: Protection::Six,
        play_action: false,
        routes: &[(TE, In(15.0)), (Z, Post(14.0)), (X, Go), (Slot, Comeback(15.0)), (Slot2, Curl(12.0)), (TE2, Flat)],
        reads: &[Z, TE, Slot, X],
        depth: PassDepth::Deep,
    },
    PassConcept {
        name: "play-action shot",
        drop: Drop::Deep,
        protection: Protection::Max,
        play_action: true,
        routes: &[(Z, Post(14.0)), (X, In(16.0)), (Slot, Go), (TE, Corner(12.0)), (Slot2, Go), (TE2, Block)],
        reads: &[Z, X, Slot, TE],
        depth: PassDepth::Deep,
    },
    PassConcept {
        name: "play-action boot",
        drop: Drop::Boot,
        protection: Protection::Standard,
        play_action: true,
        routes: &[
            (TE, Flat),
            (X, Comeback(14.0)),
            (Z, Corner(12.0)),
            (Slot, Drag),
            (Slot2, In(12.0)),
            (TE2, Drag),
            (FB, Flat),
        ],
        reads: &[TE, FB, Z, X, Slot],
        depth: PassDepth::Medium,
    },
    PassConcept {
        name: "play-action dig",
        drop: Drop::Medium,
        protection: Protection::Six,
        play_action: true,
        routes: &[(X, In(14.0)), (Z, Go), (Slot, Sit(10.0)), (TE, Sit(8.0)), (Slot2, In(12.0)), (TE2, Flat)],
        reads: &[X, TE, Slot, Z],
        depth: PassDepth::Medium,
    },
    PassConcept {
        name: "RB screen",
        drop: Drop::Screen,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(X, Go), (Z, Go), (Slot, Go), (Slot2, Go), (TE, Seam), (RB, Screen)],
        reads: &[RB],
        depth: PassDepth::Screen,
    },
    PassConcept {
        name: "bubble screen",
        drop: Drop::Bubble,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(Slot, Bubble), (Z, Block), (X, Hitch(5.0)), (Slot2, Bubble), (TE, Block)],
        reads: &[Slot, Slot2, X],
        depth: PassDepth::Screen,
    },
    // Speed outs: the quick out, the most thrown route in the real NFL
    // (about one target in six under 10 air yards). Outside receivers break
    // flat at four yards; the slot sits over the ball (proposed, not
    // confirmed: the full play library is a later section).
    PassConcept {
        name: "speed outs",
        drop: Drop::Quick,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(X, Out(4.0)), (Z, Out(4.0)), (Slot, Sit(6.0)), (Slot2, Out(4.0)), (TE, Seam), (TE2, Flat)],
        reads: &[Z, X, Slot, TE, RB],
        depth: PassDepth::Quick,
    },
    PassConcept {
        name: "fades",
        drop: Drop::Quick,
        protection: Protection::Six,
        play_action: false,
        routes: &[(X, Fade), (Z, Fade), (Slot, Slant), (TE, Sit(4.0)), (Slot2, Slant), (TE2, Flat)],
        reads: &[X, Z, Slot, TE],
        depth: PassDepth::Quick,
    },
    // The library from real playbooks (plays and tactics topic 1). Drive:
    // a shallow cross with a dig behind it, the West Coast staple.
    PassConcept {
        name: "drive",
        drop: Drop::Medium,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(Slot, Drag), (Z, In(12.0)), (X, Go), (TE, Sit(6.0)), (Slot2, Out(8.0)), (TE2, Flat)],
        reads: &[Slot, Z, TE, RB],
        depth: PassDepth::Medium,
    },
    // Shallow cross: the split end drags under a dig and a clear-out.
    PassConcept {
        name: "shallow cross",
        drop: Drop::Medium,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(X, Drag), (Slot, In(10.0)), (Z, Go), (TE, Sit(7.0)), (Slot2, Corner(12.0)), (TE2, Flat)],
        reads: &[X, Slot, TE, RB],
        depth: PassDepth::Medium,
    },
    // Levels: two in-breakers at two depths stretch a hook defender.
    PassConcept {
        name: "levels",
        drop: Drop::Medium,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(Slot, In(5.0)), (Z, In(12.0)), (X, Go), (TE, Flat), (Slot2, In(5.0)), (TE2, Sit(5.0))],
        reads: &[Z, Slot, TE, RB],
        depth: PassDepth::Medium,
    },
    // Spacing: five men sit in the windows of the underneath zones.
    PassConcept {
        name: "spacing",
        drop: Drop::Quick,
        protection: Protection::Standard,
        play_action: false,
        routes: &[
            (X, Hitch(5.0)),
            (Z, Hitch(5.0)),
            (Slot, Sit(5.0)),
            (TE, Sit(5.0)),
            (Slot2, Sit(5.0)),
            (TE2, Flat),
            (RB, Flat),
        ],
        reads: &[Slot, TE, RB, Z],
        depth: PassDepth::Quick,
    },
    // Snag: the outside man sits inside at five, the slot runs a corner,
    // the tight end the flat: a triangle on the flat defender.
    PassConcept {
        name: "snag",
        drop: Drop::Quick,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(Z, Sit(5.0)), (Slot, Corner(10.0)), (TE, Flat), (X, Slant), (Slot2, Flat), (TE2, Sit(5.0))],
        reads: &[Slot, Z, TE, X, RB],
        depth: PassDepth::Quick,
    },
    // Mills: a post over a dig, built to beat quarters.
    PassConcept {
        name: "mills",
        drop: Drop::Deep,
        protection: Protection::Six,
        play_action: false,
        routes: &[(Z, Post(12.0)), (Slot, In(14.0)), (X, Go), (TE, Sit(7.0)), (Slot2, In(14.0)), (TE2, Flat)],
        reads: &[Z, Slot, TE],
        depth: PassDepth::Deep,
    },
    // Y-option: the tight end works against his defender's leverage.
    PassConcept {
        name: "Y-option",
        drop: Drop::Medium,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(TE, Sit(7.0)), (X, Go), (Z, Out(12.0)), (Slot, Curl(10.0)), (Slot2, Seam), (TE2, Flat)],
        reads: &[TE, Slot, Z, RB],
        depth: PassDepth::Medium,
    },
    // Texas: the back's angle route under a seam and an out.
    PassConcept {
        name: "Texas",
        drop: Drop::Medium,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(RB, Angle), (TE, Seam), (Slot, Out(10.0)), (Z, Go), (X, In(12.0)), (Slot2, Seam)],
        reads: &[TE, RB, X, Slot],
        depth: PassDepth::Medium,
    },
    // Slot wheel: the slot wheels up the sideline under a post; it beats
    // man coverage.
    PassConcept {
        name: "slot wheel",
        drop: Drop::Medium,
        protection: Protection::Six,
        play_action: false,
        routes: &[(Slot, Wheel), (Z, Post(10.0)), (X, In(12.0)), (TE, Sit(6.0)), (Slot2, Slant), (TE2, Flat)],
        reads: &[Slot, Z, X, TE],
        depth: PassDepth::Medium,
    },
    // Tunnel screen: the flanker steps back and comes inside behind the
    // releasing linemen.
    PassConcept {
        name: "tunnel screen",
        drop: Drop::Screen,
        protection: Protection::Standard,
        play_action: false,
        routes: &[(Z, Tunnel), (Slot, Block), (X, Go), (Slot2, Go), (TE, Seam)],
        reads: &[Z],
        depth: PassDepth::Screen,
    },
    // The pass halves of run-pass options (RPO): the quarterback reads one
    // defender at the mesh and throws only if he steps up for the run.
    PassConcept {
        name: "RPO glance",
        drop: Drop::Bubble,
        protection: Protection::Standard,
        play_action: true,
        routes: &[(X, Post(7.0)), (Z, Post(7.0)), (Slot, Go), (Slot2, Go)],
        reads: &[X, Z],
        depth: PassDepth::Quick,
    },
    PassConcept {
        name: "RPO bubble",
        drop: Drop::Bubble,
        protection: Protection::Standard,
        play_action: true,
        routes: &[(Slot, Bubble), (Z, Block), (X, Block), (Slot2, Bubble)],
        reads: &[Slot, Slot2],
        depth: PassDepth::Screen,
    },
    PassConcept {
        name: "RPO slant",
        drop: Drop::Bubble,
        protection: Protection::Standard,
        play_action: true,
        routes: &[(X, Slant), (Z, Slant), (Slot, Go), (Slot2, Slant)],
        reads: &[Z, X, Slot2],
        depth: PassDepth::Quick,
    },
];

/// The pass halves of RPOs are called only inside an RPO.
pub fn is_rpo_half(c: &PassConcept) -> bool {
    c.name.starts_with("RPO")
}

/// A concept's index by name.
/// Plays built in the designer (topic 8), numbered after the library.
static DESIGNED: std::sync::RwLock<Vec<&'static PassConcept>> = std::sync::RwLock::new(Vec::new());

/// A pass concept by number: the library's, then designed plays.
pub fn pass_concept(i: usize) -> &'static PassConcept {
    if i < PASS_CONCEPTS.len() {
        &PASS_CONCEPTS[i]
    } else {
        DESIGNED.read().map(|d| d[i - PASS_CONCEPTS.len()]).unwrap_or(&PASS_CONCEPTS[0])
    }
}

/// Adds a designed play to the playbook and returns its number. Designs
/// are few and live for the whole run, so each is kept for good.
pub fn register_concept(c: PassConcept) -> usize {
    let mut d = DESIGNED.write().unwrap_or_else(|e| e.into_inner());
    d.push(Box::leak(Box::new(c)));
    PASS_CONCEPTS.len() + d.len() - 1
}

pub fn concept(name: &str) -> usize {
    PASS_CONCEPTS.iter().position(|c| c.name == name).unwrap_or(0)
}

/// A route for a role in a concept; unlisted roles block or check-release.
pub fn route_for(c: &PassConcept, role: ORole) -> Route {
    if let Some(&(_, r)) = c.routes.iter().find(|(o, _)| *o == role) {
        return r;
    }
    match role {
        RB => match c.protection {
            Protection::Standard => CheckRelease,
            _ => Block,
        },
        TE | TE2 | TE3 | FB => match c.protection {
            Protection::Max => Block,
            _ => {
                if role == TE {
                    Sit(6.0)
                } else {
                    Block
                }
            }
        },
        r if r.is_line() => Block,
        _ => Go,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RunConcept {
    InsideZone,
    OutsideZone,
    Power,
    Counter,
    Draw,
    Sneak,
    /// Inside zone with the quarterback reading the backside end.
    ZoneRead,
    Toss,
    /// Wing-T buck sweep: both guards pull.
    BuckSweep,
    /// Downhill double teams with no puller, the back reading the Mike.
    Duo,
    /// The playside defensive tackle is left alone and trapped by the
    /// backside guard.
    Trap,
    /// The fullback leads on the linebacker in the hole.
    Iso,
    /// The jet motion man takes the handoff at full speed to the edge.
    JetSweep,
    /// The quarterback carries on power, the back leads.
    QbPower,
    /// The quarterback attacks the edge and pitches off the end man.
    SpeedOption,
    /// Dive, keep or pitch off two defenders (wing-T, flexbone).
    TripleOption,
}

pub const RUN_CONCEPTS: [RunConcept; 16] = [
    RunConcept::InsideZone,
    RunConcept::OutsideZone,
    RunConcept::Power,
    RunConcept::Counter,
    RunConcept::Draw,
    RunConcept::Sneak,
    RunConcept::ZoneRead,
    RunConcept::Toss,
    RunConcept::BuckSweep,
    RunConcept::Duo,
    RunConcept::Trap,
    RunConcept::Iso,
    RunConcept::JetSweep,
    RunConcept::QbPower,
    RunConcept::SpeedOption,
    RunConcept::TripleOption,
];

impl RunConcept {
    pub fn name(self) -> &'static str {
        match self {
            RunConcept::InsideZone => "inside zone",
            RunConcept::OutsideZone => "outside zone",
            RunConcept::Power => "power",
            RunConcept::Counter => "counter",
            RunConcept::Draw => "draw",
            RunConcept::Sneak => "QB sneak",
            RunConcept::ZoneRead => "zone read",
            RunConcept::Toss => "toss",
            RunConcept::BuckSweep => "buck sweep",
            RunConcept::Duo => "duo",
            RunConcept::Trap => "trap",
            RunConcept::Iso => "iso",
            RunConcept::JetSweep => "jet sweep",
            RunConcept::QbPower => "QB power",
            RunConcept::SpeedOption => "speed option",
            RunConcept::TripleOption => "triple option",
        }
    }
    /// The quarterback carries (or may).
    pub fn qb_run(self) -> bool {
        matches!(self, RunConcept::Sneak | RunConcept::QbPower)
    }
    /// Gap schemes: linemen down-block and someone pulls or leads.
    pub fn gap_scheme(self) -> bool {
        matches!(
            self,
            RunConcept::Power
                | RunConcept::Counter
                | RunConcept::BuckSweep
                | RunConcept::Trap
                | RunConcept::Iso
                | RunConcept::QbPower
                | RunConcept::Duo
        )
    }
    /// Where the back aims, in yards from the center toward the play side,
    /// and whether the scheme pulls a backside guard (gap schemes).
    /// The pace a back presses the line at, yards per second: zone runs
    /// patient, gap runs downhill, tosses and sneaks at full speed (0).
    /// Proposed, not confirmed (Claude's reading of NGS time to the line,
    /// about 2.8 s from the snap).
    /// Slow to the hole, fast through it: about two thirds of what it was
    /// (backs reached the line 0.7 s before the real Next Gen time; with
    /// the later handoff they reach it at about 2.55 s, real 2.76;
    /// proposed, not confirmed).
    pub fn press(self) -> f64 {
        match self {
            RunConcept::InsideZone | RunConcept::ZoneRead | RunConcept::OutsideZone => 4.1,
            RunConcept::Power | RunConcept::Counter | RunConcept::BuckSweep => 4.7,
            RunConcept::Duo | RunConcept::Trap | RunConcept::Iso => 4.55,
            // The quarterback keeps from the snap behind his pullers; the
            // Next Gen time is the backs' (at 4.55 QB power gained 0.7 a
            // carry on the test teams).
            RunConcept::QbPower => 7.0,
            RunConcept::Draw => 3.9,
            RunConcept::Sneak | RunConcept::Toss | RunConcept::JetSweep => 0.0,
            RunConcept::SpeedOption | RunConcept::TripleOption => 0.0,
        }
    }

    pub fn aim(self) -> (f64, u8) {
        match self {
            RunConcept::InsideZone | RunConcept::ZoneRead => (1.8, 0),
            RunConcept::OutsideZone => (5.5, 0),
            RunConcept::Power => (2.8, 1),
            RunConcept::Counter => (3.0, 2),
            RunConcept::Draw => (0.8, 0),
            RunConcept::Sneak => (0.4, 0),
            RunConcept::Toss => (8.0, 0),
            RunConcept::BuckSweep => (6.5, 2),
            RunConcept::Duo => (1.2, 0),
            RunConcept::Trap => (0.9, 1),
            RunConcept::Iso => (1.0, 0),
            RunConcept::JetSweep => (11.0, 0),
            RunConcept::QbPower => (2.6, 1),
            RunConcept::SpeedOption => (7.5, 0),
            RunConcept::TripleOption => (1.5, 0),
        }
    }

    /// The family of concepts a player learns it with (topic 4).
    pub fn family(self) -> Family {
        match self {
            RunConcept::InsideZone | RunConcept::Duo | RunConcept::Draw | RunConcept::Sneak => Family::InsideZone,
            RunConcept::OutsideZone | RunConcept::Toss | RunConcept::JetSweep => Family::OutsideZone,
            RunConcept::Power
            | RunConcept::Counter
            | RunConcept::BuckSweep
            | RunConcept::Trap
            | RunConcept::Iso
            | RunConcept::QbPower => Family::Gap,
            RunConcept::ZoneRead | RunConcept::SpeedOption | RunConcept::TripleOption => Family::Option,
        }
    }
}

/// Concept families (topic 4.1): players learn parts, not plays, and what
/// they know of a family carries to every play built from it and across
/// systems that share it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    InsideZone,
    OutsideZone,
    Gap,
    Option,
    Rpo,
    Quick,
    Dropback,
    PlayAction,
    Screen,
    /// Defense: the front's fits, single-high and two-high zones, man, and
    /// pressures with stunts.
    Front,
    SingleHigh,
    TwoHigh,
    Man,
    Pressure,
}

pub const FAMILY_COUNT: usize = 14;

impl Family {
    pub const ALL: [Family; FAMILY_COUNT] = [
        Family::InsideZone,
        Family::OutsideZone,
        Family::Gap,
        Family::Option,
        Family::Rpo,
        Family::Quick,
        Family::Dropback,
        Family::PlayAction,
        Family::Screen,
        Family::Front,
        Family::SingleHigh,
        Family::TwoHigh,
        Family::Man,
        Family::Pressure,
    ];
    pub fn name(self) -> &'static str {
        [
            "inside zone",
            "outside zone",
            "gap runs",
            "option",
            "RPOs",
            "quick game",
            "dropback",
            "play action",
            "screens",
            "front and fits",
            "single-high zone",
            "two-high zone",
            "man",
            "pressures and stunts",
        ][self as usize]
    }
    pub fn offense(self) -> bool {
        (self as usize) < Family::Front as usize
    }
}

/// The family of a pass concept.
pub fn pass_family(c: &PassConcept) -> Family {
    if is_rpo_half(c) {
        Family::Rpo
    } else if c.depth == PassDepth::Screen {
        Family::Screen
    } else if c.play_action {
        Family::PlayAction
    } else if c.depth == PassDepth::Quick {
        Family::Quick
    } else {
        Family::Dropback
    }
}

/// The family of an offensive call.
pub fn call_family(k: PlayKind) -> Family {
    match k {
        PlayKind::Run(r) => r.family(),
        PlayKind::Pass(i) => pass_family(pass_concept(i)),
        PlayKind::Rpo(..) => Family::Rpo,
        _ => Family::Quick,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayKind {
    Run(RunConcept),
    Pass(usize),
    /// A run-pass option: the run, and the quick throw the quarterback
    /// pulls to if the read defender steps up (college linemen may be three
    /// yards downfield on it, NFL linemen one).
    Rpo(RunConcept, usize),
    Kneel,
    Spike,
}

impl PlayKind {
    pub fn is_run(self) -> bool {
        matches!(self, PlayKind::Run(_))
    }
    pub fn is_pass(self) -> bool {
        matches!(self, PlayKind::Pass(_))
    }
}

/// The offense's call.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OffCall {
    pub formation: Formation,
    pub kind: PlayKind,
    /// +1: run to the offense's left (higher y), -1 to the right.
    pub dir: f64,
    /// Hurry-up tempo.
    pub tempo: bool,
    /// The second play of a "kill" call (topic 3.3): the quarterback picks
    /// one of the two at the line.
    pub alt: Option<PlayKind>,
    /// Why the caller chose it (topic 6.6), in a few words.
    pub why: &'static str,
}

impl OffCall {
    pub fn new(formation: Formation, kind: PlayKind, dir: f64, tempo: bool) -> OffCall {
        OffCall { formation, kind, dir, tempo, alt: None, why: "" }
    }
    pub fn describe(&self) -> String {
        let m = self.formation.motion;
        let pre = if m == Motion::None { String::new() } else { format!("{}, ", m.name()) };
        let body = match self.kind {
            PlayKind::Run(r) => format!("{} {}", r.name(), if self.dir > 0.0 { "left" } else { "right" }),
            PlayKind::Pass(i) => pass_concept(i).name.to_string(),
            PlayKind::Rpo(r, i) => format!("{} with {}", r.name(), pass_concept(i).name),
            PlayKind::Kneel => "kneel".into(),
            PlayKind::Spike => "spike".into(),
        };
        format!("{pre}{body}")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Coverage {
    Cover0,
    Cover1,
    Cover2,
    Tampa2,
    Cover3,
    Cover4,
    Cover2Man,
    Cover6,
    /// Cover 1 with a robber: a safety or linebacker sits in the middle
    /// hole at ten yards instead of taking the tight end.
    Cover1Robber,
    /// Pattern-matching Cover 3: the curl-flat defenders carry the second
    /// receiver up the seam and the corners carry the first.
    Cover3Match,
}

pub const COVERAGE_COUNT: usize = 10;

impl Coverage {
    pub const ALL: [Coverage; COVERAGE_COUNT] = [
        Coverage::Cover0,
        Coverage::Cover1,
        Coverage::Cover2,
        Coverage::Tampa2,
        Coverage::Cover3,
        Coverage::Cover4,
        Coverage::Cover2Man,
        Coverage::Cover6,
        Coverage::Cover1Robber,
        Coverage::Cover3Match,
    ];
    pub fn name(self) -> &'static str {
        [
            "Cover 0",
            "Cover 1",
            "Cover 2",
            "Tampa 2",
            "Cover 3",
            "Cover 4",
            "2-man",
            "Cover 6",
            "Cover 1 robber",
            "Cover 3 match",
        ][self as usize]
    }
    pub fn is_man(self) -> bool {
        matches!(self, Coverage::Cover0 | Coverage::Cover1 | Coverage::Cover2Man | Coverage::Cover1Robber)
    }
    /// Deep safeties.
    pub fn high(self) -> u8 {
        match self {
            Coverage::Cover0 => 0,
            Coverage::Cover1 | Coverage::Cover3 | Coverage::Cover1Robber | Coverage::Cover3Match => 1,
            _ => 2,
        }
    }
    /// The family a defender learns it with (topic 4).
    pub fn family(self) -> Family {
        if self.is_man() {
            Family::Man
        } else if self.high() == 2 {
            Family::TwoHigh
        } else {
            Family::SingleHigh
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DefPersonnel {
    Base,
    Nickel,
    Dime,
    GoalLine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pressure {
    /// Four rushers.
    Four,
    /// A linebacker blitzes.
    LbBlitz,
    /// A corner or safety blitzes.
    DbBlitz,
    /// Two linebackers through the A gaps.
    DoubleA,
    /// Four rushers, but a linebacker comes and a lineman drops.
    Simulated,
    /// A fire zone: five rushers (a linebacker and a nickel or safety, an
    /// end dropping), three deep and three under behind it.
    FireZone,
}

impl Pressure {
    pub fn extra(self) -> u8 {
        match self {
            Pressure::Four | Pressure::Simulated => 0,
            Pressure::LbBlitz | Pressure::DbBlitz | Pressure::FireZone => 1,
            Pressure::DoubleA => 2,
        }
    }
    pub fn name(self) -> &'static str {
        ["four-man rush", "linebacker blitz", "DB blitz", "double-A blitz", "simulated pressure", "fire zone"]
            [self as usize]
    }
}

/// Where the defensive line lines up (topic 1: each lineman in a real
/// technique). Over: the three-technique to the strength; under: to the
/// weak side, the strong end tight; wide: the ends outside the tight end
/// (wide nine); odd: a nose on the center and five-techniques; tite: a nose
/// and two four-i's (closes the B gaps against inside runs); bear: the
/// center and both guards covered, so nobody climbs free.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrontLook {
    Over,
    Under,
    Wide,
    Odd,
    Tite,
    Bear,
}

impl FrontLook {
    pub fn name(self) -> &'static str {
        ["over", "under", "wide nine", "odd", "tite", "bear"][self as usize]
    }
}

/// A line stunt (topic 1): the tackle crashes out and the end loops in
/// (T-E), the end crashes in and the tackle loops out (E-T), or the two
/// tackles twist. A line that passes it off cleanly stops it; one that
/// doesn't leaves the looper free.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stunt {
    None,
    TackleEnd,
    EndTackle,
    Twist,
}

impl Stunt {
    pub fn name(self) -> &'static str {
        ["", "T-E stunt", "E-T stunt", "tackle twist"][self as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DefCall {
    pub personnel: DefPersonnel,
    pub coverage: Coverage,
    pub pressure: Pressure,
    /// Safeties rotate after the snap from the shell they showed.
    pub disguise: bool,
    pub front: FrontLook,
    pub stunt: Stunt,
    /// A linebacker spies a running quarterback (topic 5.4).
    pub spy: bool,
}

impl DefCall {
    pub fn new(personnel: DefPersonnel, coverage: Coverage, pressure: Pressure) -> DefCall {
        DefCall {
            personnel,
            coverage,
            pressure,
            disguise: false,
            front: FrontLook::Over,
            stunt: Stunt::None,
            spy: false,
        }
    }
    pub fn describe(&self) -> String {
        let mut s = format!("{}, {}", self.coverage.name(), self.pressure.name());
        if self.stunt != Stunt::None {
            s.push_str(&format!(", {}", self.stunt.name()));
        }
        if self.spy {
            s.push_str(", QB spy");
        }
        s
    }
}
