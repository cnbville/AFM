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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Formation {
    pub personnel: Personnel,
    pub backfield: Backfield,
    /// Three receivers to one side.
    pub trips: bool,
    /// +1: strength to the offense's left (higher y), -1: to the right.
    pub strength: f64,
}

/// Where a receiver lines up wide on a side, from the sideline in.
fn wide_y(side: f64, ball_y: f64, from_sideline: f64) -> f64 {
    let y = if side > 0.0 { WIDTH - from_sideline } else { from_sideline };
    // Never closer to the ball than 7 yards.
    if side > 0.0 { y.max(ball_y + 7.0) } else { y.min(ball_y - 7.0) }
}

/// Alignment of every spot: offsets from the ball, x back from the line.
pub fn align(f: &Formation, role: ORole, los: f64, ball_y: f64) -> V2 {
    let s = f.strength;
    let at = |dx: f64, dy: f64| v2(los + dx, ball_y + dy);
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
            if f.trips {
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
            Route::Screen => "screen",
            Route::Bubble => "bubble",
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
            Route::Post(_) => (&[(0.0, 0.0), (25.0, -12.0)], false),
            Route::Corner(_) => (&[(0.0, 0.0), (14.0, 11.0)], false),
            Route::Flat => (&[(1.5, 3.0), (3.0, 9.0), (4.0, 12.5), (14.0, 13.5)], false),
            Route::Wheel => (&[(1.0, 4.0), (4.0, 8.0), (40.0, 9.0)], false),
            Route::Drag => (&[(2.5, 0.0), (3.5, -30.0)], false),
            Route::Sit(_) => (&[(0.0, 0.0)], true),
            Route::Swing => (&[(0.5, 4.0), (2.0, 8.0)], true),
            Route::Bubble => (&[(-1.0, 3.0), (-0.5, 6.0), (10.0, 7.0)], false),
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
                v2(x, (start.y + dy * out).clamp(1.0, WIDTH - 1.0))
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
    PassConcept {
        name: "fades",
        drop: Drop::Quick,
        protection: Protection::Six,
        play_action: false,
        routes: &[(X, Fade), (Z, Fade), (Slot, Slant), (TE, Sit(4.0)), (Slot2, Slant), (TE2, Flat)],
        reads: &[X, Z, Slot, TE],
        depth: PassDepth::Quick,
    },
];

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
}

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
        }
    }
    /// Where the back aims, in yards from the center toward the play side,
    /// and whether the scheme pulls a backside guard (gap schemes).
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
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayKind {
    Run(RunConcept),
    Pass(usize),
    Kneel,
    Spike,
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
}

impl OffCall {
    pub fn describe(&self) -> String {
        match self.kind {
            PlayKind::Run(r) => format!("{} {}", r.name(), if self.dir > 0.0 { "left" } else { "right" }),
            PlayKind::Pass(i) => PASS_CONCEPTS[i].name.to_string(),
            PlayKind::Kneel => "kneel".into(),
            PlayKind::Spike => "spike".into(),
        }
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
}

impl Coverage {
    pub fn name(self) -> &'static str {
        ["Cover 0", "Cover 1", "Cover 2", "Tampa 2", "Cover 3", "Cover 4", "2-man", "Cover 6"][self as usize]
    }
    pub fn is_man(self) -> bool {
        matches!(self, Coverage::Cover0 | Coverage::Cover1 | Coverage::Cover2Man)
    }
    /// Deep safeties.
    pub fn high(self) -> u8 {
        match self {
            Coverage::Cover0 => 0,
            Coverage::Cover1 | Coverage::Cover3 => 1,
            _ => 2,
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
}

impl Pressure {
    pub fn extra(self) -> u8 {
        match self {
            Pressure::Four | Pressure::Simulated => 0,
            Pressure::LbBlitz | Pressure::DbBlitz => 1,
            Pressure::DoubleA => 2,
        }
    }
    pub fn name(self) -> &'static str {
        ["four-man rush", "linebacker blitz", "DB blitz", "double-A blitz", "simulated pressure"][self as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DefCall {
    pub personnel: DefPersonnel,
    pub coverage: Coverage,
    pub pressure: Pressure,
    /// Safeties rotate after the snap from the shell they showed.
    pub disguise: bool,
}

impl DefCall {
    pub fn describe(&self) -> String {
        format!("{}, {}", self.coverage.name(), self.pressure.name())
    }
}
