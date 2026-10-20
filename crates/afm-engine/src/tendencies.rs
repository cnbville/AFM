//! Player tendencies (`docs/design/tendencies.md`): how often a player picks
//! one option when the play gives him a choice, never how well it works. His
//! attributes decide that. Fifteen tendencies in eight groups, each on the
//! 1-20 scale with 10.5 the real NFL average and the ends the real spread
//! among NFL starters, 2016-2025 (`data/real-world/tendency_spreads.json`).
//!
//! Where they come from (age 14), how results, coaching, the staff's style,
//! age and injuries change them, how a staff reads them, the instructions a
//! coach gives and what fits a team: all here. The match engine reads them
//! at their moments (`game::sim::Sim::tend`).

use crate::attributes::{Attr, Attributes};
use crate::game::roster::{Front, OffScheme};
use crate::player::{Level, Personality};
use crate::positions::{POSITION_COUNT, Position};
use crate::rng::{Rng, clamp, round_half_up};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Tend {
    // Ball carriers: anyone with the ball.
    /// Leaves the called hole, outside or cutting back (high), against
    /// hitting it as called (low).
    Bounce,
    /// Seeks contact (high): runs through people, turns up at the sideline,
    /// takes on blocks, dives on piles. Avoids it (low): slides, steps out,
    /// "alligator arms" over the middle, runs around blocks.
    Contact,
    /// Reaches the ball out for the line to gain or the goal line with a
    /// defender closing.
    Reach,
    // Backs.
    /// Patient (high): presses the line and waits for the crease. Downhill
    /// (low): one cut and go.
    Patience,
    // Quarterbacks.
    /// Leaves the pocket (high), against staying in it.
    Scramble,
    /// Throws into tight windows and downfield (high), against the checkdown.
    Gunslinger,
    /// Holds for the big play (high), against getting it out.
    Hold,
    // Returners.
    /// Brings it out from deep and fields what he could let go (high).
    ReturnRisk,
    // Pass rushers.
    /// Rushes upfield on the wide arc (high), against keeping contain.
    Upfield,
    /// Guesses the count at the snap.
    SnapTiming,
    /// Gets his hands up in the throwing lane instead of rushing through.
    HandsUp,
    // Tacklers.
    /// Goes for the strip instead of the wrap.
    Strip,
    /// The big hit (high), against the sure tackle (low).
    BigHit,
    // Linebackers and safeties.
    /// Flies downhill on his first read (high), against reading first.
    Downhill,
    // Coverage.
    /// Jumps routes on the quarterback's eyes.
    JumpRoutes,
}

pub const TEND_COUNT: usize = 15;

impl Tend {
    pub const ALL: [Tend; TEND_COUNT] = [
        Tend::Bounce,
        Tend::Contact,
        Tend::Reach,
        Tend::Patience,
        Tend::Scramble,
        Tend::Gunslinger,
        Tend::Hold,
        Tend::ReturnRisk,
        Tend::Upfield,
        Tend::SnapTiming,
        Tend::HandsUp,
        Tend::Strip,
        Tend::BigHit,
        Tend::Downhill,
        Tend::JumpRoutes,
    ];

    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn key(self) -> &'static str {
        match self {
            Tend::Bounce => "bouncesRuns",
            Tend::Contact => "contactWillingness",
            Tend::Reach => "reachesBallOut",
            Tend::Patience => "patientOrDownhill",
            Tend::Scramble => "scrambles",
            Tend::Gunslinger => "gunslinger",
            Tend::Hold => "holdsForBigPlay",
            Tend::ReturnRisk => "returnerRisk",
            Tend::Upfield => "rushesUpfield",
            Tend::SnapTiming => "timesTheSnap",
            Tend::HandsUp => "getsHandsUp",
            Tend::Strip => "goesForStrip",
            Tend::BigHit => "bigHit",
            Tend::Downhill => "fliesDownhill",
            Tend::JumpRoutes => "jumpsRoutes",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Tend::Bounce => "Bounces runs",
            Tend::Contact => "Contact willingness",
            Tend::Reach => "Reaches the ball out",
            Tend::Patience => "Patient or downhill",
            Tend::Scramble => "Scrambles",
            Tend::Gunslinger => "Gunslinger or checkdown",
            Tend::Hold => "Holds for the big play",
            Tend::ReturnRisk => "Returner risk",
            Tend::Upfield => "Rushes upfield or keeps contain",
            Tend::SnapTiming => "Times the snap",
            Tend::HandsUp => "Gets his hands up",
            Tend::Strip => "Goes for the strip",
            Tend::BigHit => "Big hit or sure tackle",
            Tend::Downhill => "Flies downhill or reads first",
            Tend::JumpRoutes => "Jumps routes",
        }
    }

    /// The plain words at the low and the high end.
    pub fn ends(self) -> (&'static str, &'static str) {
        match self {
            Tend::Bounce => ("hits the called hole", "bounces runs"),
            Tend::Contact => ("avoids contact", "seeks contact"),
            Tend::Reach => ("never reaches", "always reaches"),
            Tend::Patience => ("downhill", "patient"),
            Tend::Scramble => ("stays in the pocket", "leaves the pocket"),
            Tend::Gunslinger => ("checks it down", "gunslinger"),
            Tend::Hold => ("gets it out", "holds for the big play"),
            Tend::ReturnRisk => ("plays it safe", "brings everything out"),
            Tend::Upfield => ("keeps contain", "rushes upfield"),
            Tend::SnapTiming => ("waits for the ball", "guesses the count"),
            Tend::HandsUp => ("rushes through", "gets his hands up"),
            Tend::Strip => ("wraps up", "goes for the strip"),
            Tend::BigHit => ("sure tackle", "big hit"),
            Tend::Downhill => ("reads first", "flies downhill"),
            Tend::JumpRoutes => ("stays home", "jumps routes"),
        }
    }

    /// The real stat it is checked against (topic 4.2).
    pub fn stat(self) -> &'static str {
        match self {
            Tend::Bounce => "Carries outside the tackles",
            Tend::Contact => "Runs ending out of bounds",
            Tend::Reach => "Fumbles near the line",
            Tend::Patience => "Time behind the line",
            Tend::Scramble => "Scramble rate",
            Tend::Gunslinger => "Tight-window throws",
            Tend::Hold => "Time to throw",
            Tend::ReturnRisk => "Kicks brought out from the end zone",
            Tend::Upfield => "Scramble yards allowed",
            Tend::SnapTiming => "Offside and neutral zone flags",
            Tend::HandsUp => "Batted passes",
            Tend::Strip => "Forced fumbles per tackle",
            Tend::BigHit => "Missed tackle rate",
            Tend::Downhill => "Tackles for loss",
            Tend::JumpRoutes => "Interceptions per target",
        }
    }

    /// A gamble: down two scores late every player takes more of these,
    /// protecting a lead fewer (topic 3.8). Patience, contact and getting
    /// the hands up aren't gambles.
    pub fn gamble(self) -> bool {
        !matches!(self, Tend::Patience | Tend::Contact | Tend::HandsUp)
    }

    /// The positions that own it (topic 2): the rest still carry it for
    /// the rare play (a lineman with a fumble, a kicker making a tackle),
    /// but the player page shows it only where it is his.
    pub fn owned_by(self, pos: Position) -> bool {
        use Position as P;
        match self {
            Tend::Bounce | Tend::Patience => matches!(pos, P::RB),
            Tend::Contact | Tend::Reach => matches!(pos, P::QB | P::RB | P::WR | P::TE | P::LB | P::CB | P::S),
            Tend::Scramble | Tend::Gunslinger | Tend::Hold => pos == P::QB,
            Tend::ReturnRisk => matches!(pos, P::RB | P::WR | P::CB | P::S),
            Tend::Upfield | Tend::SnapTiming | Tend::HandsUp => matches!(pos, P::EDGE | P::DT),
            Tend::Strip | Tend::BigHit => matches!(pos, P::EDGE | P::DT | P::LB | P::CB | P::S),
            Tend::Downhill => matches!(pos, P::LB | P::S),
            Tend::JumpRoutes => matches!(pos, P::LB | P::CB | P::S),
        }
    }
}

/// The middle of the scale: the real NFL average.
pub const MID: f64 = 10.5;

/// Spread of the tendency numbers among regulars at each level, as an sd on
/// the 1-20 scale. NFL starters run from about 4 to 16 (the real p5 to p95);
/// high school is about half again as wide (topic 3.2, proposed width).
pub fn level_sd(level: Level) -> f64 {
    match level {
        Level::NFL => 3.2,
        Level::FBS => 3.8,
        Level::FCS => 4.0,
        Level::SmallCollege => 4.2,
        Level::HS => 4.8,
    }
}

/// A player's tendencies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tendencies {
    /// What he does now, 1-20.
    pub v: [f64; TEND_COUNT],
    /// The number before coaching moved it: a coached-down habit leaves a
    /// trace he slips back toward under stress (topic 3.7).
    pub was: [f64; TEND_COUNT],
    /// Contact willingness he lost after a major injury, coming back over a
    /// season or two (topic 3.6).
    pub contact_dip: f64,
}

impl Default for Tendencies {
    fn default() -> Tendencies {
        Tendencies { v: [MID; TEND_COUNT], was: [MID; TEND_COUNT], contact_dip: 0.0 }
    }
}

impl Tendencies {
    #[inline]
    pub fn get(&self, t: Tend) -> f64 {
        let v = self.v[t.index()];
        if t == Tend::Contact { clamp(v - self.contact_dip, 1.0, 20.0) } else { v }
    }

    pub fn set(&mut self, t: Tend, x: f64) {
        let x = clamp(x, 1.0, 20.0);
        self.v[t.index()] = x;
        self.was[t.index()] = x;
    }

    /// Where a 14-year-old starts (topic 3.1): mostly who he is, drawn at
    /// random, with modest leans from his body and mind, none more than 3.
    /// From his own id stream, so no other draw moves. Centred so the
    /// national class averages the middle (the leans read ratings against
    /// a fixed 70, which a class of 14-year-olds sits below).
    pub fn at_14(id: u32, ceilings: &Attributes, weight: f64, height: f64, pers: &Personality) -> Tendencies {
        let raw = Tendencies::drawn(id, ceilings, weight, height, pers);
        let mut t = Tendencies::default();
        for k in Tend::ALL {
            t.set(k, round_half_up(raw.v[k.index()] - CENTER_14[k.index()]));
        }
        t
    }

    fn drawn(id: u32, ceilings: &Attributes, weight: f64, height: f64, pers: &Personality) -> Tendencies {
        let mut r = Rng::new(id ^ 0x7E4D_3C21);
        let c = |a: Attr| (ceilings[a] - 70.0) / 12.0;
        let lean = |x: f64| clamp(x, -3.0, 3.0);
        let compete = (pers.competitiveness - 10.0) / 3.5;
        let disc = c(Attr::Discipline);
        let fast = 0.5 * c(Attr::Speed) + 0.5 * c(Attr::Acceleration);
        let big = (weight - 215.0) / 40.0;
        let mut t = Tendencies::default();
        for k in Tend::ALL {
            let l = match k {
                Tend::Bounce => 1.6 * fast + 0.5 * c(Attr::Elusiveness),
                Tend::Contact => 1.2 * big + 0.8 * c(Attr::Strength) + 0.8 * c(Attr::Toughness),
                Tend::Reach => 0.8 * compete - 0.5 * disc,
                Tend::Patience => 1.0 * c(Attr::BallCarrierVision) - 0.8 * c(Attr::Acceleration),
                Tend::Scramble => 1.8 * fast,
                Tend::Gunslinger => 0.8 * c(Attr::ArmStrength) + 0.6 * compete - 0.6 * disc,
                Tend::Hold => 0.6 * c(Attr::Strength) + 0.5 * compete,
                Tend::ReturnRisk => 1.0 * fast + 0.6 * compete - 0.5 * disc,
                Tend::Upfield => 1.2 * fast - 0.5 * disc,
                Tend::SnapTiming => 0.9 * c(Attr::Anticipation) - 0.9 * disc,
                Tend::HandsUp => 1.0 * (height - 75.0) / 2.5,
                Tend::Strip => 0.7 * compete,
                Tend::BigHit => 1.0 * c(Attr::Physicality) + 0.5 * big,
                Tend::Downhill => 0.9 * fast - 0.6 * disc,
                Tend::JumpRoutes => 0.8 * c(Attr::Anticipation) + 0.5 * compete - 0.8 * disc,
            };
            t.set(k, r.normal(MID, 4.4) + lean(l));
        }
        t
    }

    /// Drawn at a level's spread for a player made for the level (synthetic
    /// teams, tools), with the same leans, centred so the level's owners
    /// average the middle (the real league average).
    pub fn at_level(
        id: u32,
        level: Level,
        attrs: &Attributes,
        weight: f64,
        height: f64,
        pers: &Personality,
    ) -> Tendencies {
        let base = Tendencies::drawn(id, attrs, weight, height, pers);
        let mut t = Tendencies::default();
        let squeeze = level_sd(level) / 4.8;
        let center = match level {
            Level::NFL => &CENTER_NFL,
            Level::FBS => &CENTER_FBS,
            Level::FCS => &CENTER_FCS,
            Level::SmallCollege => &CENTER_SMALL,
            Level::HS => &CENTER_HS,
        };
        for k in Tend::ALL {
            t.set(k, round_half_up(MID + (base.v[k.index()] - MID) * squeeze - center[k.index()]));
        }
        t
    }
}

// How far the drawn numbers sit from the middle, in `Tend::ALL` order:
// among a class of 14-year-olds, and among each level's synthetic owners
// (measured with `tendency_check LEVEL 2 1 centers` before centring).
const CENTER_14: [f64; TEND_COUNT] =
    [-2.92, -2.21, 1.26, -1.37, -2.54, -0.70, -1.22, -0.72, -1.09, -0.25, -1.28, 0.07, -2.29, -0.35, -0.23];
const CENTER_NFL: [f64; TEND_COUNT] =
    [0.88, -0.23, 0.33, -0.20, -0.67, 1.34, 0.09, 0.55, 0.30, 0.50, 0.16, 0.33, 0.58, 0.25, 0.27];
const CENTER_FBS: [f64; TEND_COUNT] =
    [-1.29, -1.87, 0.72, -0.52, -2.02, 0.99, -0.64, -0.25, -0.57, 0.30, -0.36, 0.19, -0.74, -0.17, 0.08];
const CENTER_FCS: [f64; TEND_COUNT] =
    [-1.90, -2.26, 0.95, -0.66, -2.27, 0.81, -0.82, -0.57, -0.92, 0.24, -0.60, 0.21, -1.32, -0.34, 0.05];
const CENTER_SMALL: [f64; TEND_COUNT] =
    [-2.20, -2.51, 1.16, -0.77, -2.42, 0.86, -1.00, -0.83, -1.23, 0.21, -0.75, 0.23, -1.72, -0.50, 0.04];
const CENTER_HS: [f64; TEND_COUNT] =
    [-2.57, -2.93, 1.67, -1.21, -2.92, 0.70, -1.12, -0.88, -2.15, 0.15, -0.78, 0.00, -2.35, 0.34, 0.24];

impl Tendencies {}

// ------------------------------------------------------------------ fit

/// What his tools support: the number at which the habit neither costs nor
/// pays at his level (topic 1.3). A slow back who bounces sits above it; a
/// fast one who never bounces below. Tendencies no attribute settles stay
/// where they are.
pub fn ideal(t: Tend, a: &Attributes, level: Level) -> Option<f64> {
    let norm = crate::game::rules::trait_norm(level, Attr::Speed) + 8.0;
    let rel = |x: f64| (x - norm) / 6.0;
    let v = match t {
        Tend::Bounce => {
            MID + 1.8 * rel(0.4 * a[Attr::Speed] + 0.3 * a[Attr::Acceleration] + 0.3 * a[Attr::Elusiveness])
                + 0.6 * rel(a[Attr::BallCarrierVision])
        }
        Tend::Patience => MID + 1.2 * rel(a[Attr::BallCarrierVision]) - 1.2 * rel(a[Attr::Acceleration]),
        Tend::Scramble => MID + 2.0 * rel(0.6 * a[Attr::Speed] + 0.4 * a[Attr::ThrowOnMove]),
        Tend::Gunslinger => {
            MID + 1.2 * rel(0.4 * a[Attr::MediumAccuracy] + 0.3 * a[Attr::ArmStrength] + 0.3 * a[Attr::Anticipation])
                - 0.6 * rel(a[Attr::Decisions]).min(0.0)
        }
        Tend::Hold => MID + 1.2 * rel(0.5 * a[Attr::PocketPresence] + 0.5 * a[Attr::Strength]),
        Tend::ReturnRisk => {
            MID + 1.5 * rel(0.4 * a[Attr::Speed] + 0.3 * a[Attr::BallCarrierVision] + 0.3 * a[Attr::Elusiveness])
        }
        Tend::Upfield => MID + 1.5 * rel(0.5 * a[Attr::Speed] + 0.5 * a[Attr::Flexibility]),
        Tend::SnapTiming => MID + 1.5 * rel(a[Attr::Anticipation]) - 0.5,
        Tend::Strip => MID + 1.2 * rel(0.5 * a[Attr::Tackling] + 0.5 * a[Attr::HandUse]),
        Tend::BigHit => MID + 1.5 * rel(a[Attr::Tackling]) - 0.5,
        Tend::Downhill => MID + 1.5 * rel(a[Attr::PlayRecognition]),
        Tend::JumpRoutes => MID + 1.5 * rel(0.6 * a[Attr::Anticipation] + 0.4 * a[Attr::PlayRecognition]) - 0.5,
        Tend::Reach => MID + 1.2 * rel(a[Attr::BallSecurity]) - 1.0,
        Tend::Contact | Tend::HandsUp => return None,
    };
    Some(clamp(v, 1.0, 20.0))
}

/// How far a habit sits from what his tools support, signed: positive when
/// he does it more than his tools pay for. Zero where nothing settles it.
pub fn misfit(t: Tend, value: f64, a: &Attributes, level: Level) -> f64 {
    ideal(t, a, level).map_or(0.0, |i| value - i)
}

// ---------------------------------------------------------------- style

/// A staff's style (the "Your coaching style" part of the spec): each
/// setting 1-20 from one end to the other. Tempo and run-pass mix are game
/// plan settings, not here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    /// Conservative (1) to aggressive (20): Jim Tressel against Bruce Arians.
    pub risk: f64,
    /// Structure (1, "do your job") to freedom (20, Ed Reed freelancing).
    pub freedom: f64,
    /// Protective (1, rugby tackling) to physical (20).
    pub physical: f64,
    /// Sound (1, keep it in front) to ball hunting (20, the punch-out).
    pub ball_hunting: f64,
    /// Adapt (1, build around what he does) to mould (20, coach every habit
    /// to the system).
    pub mould: f64,
}

impl Default for Style {
    fn default() -> Style {
        Style { risk: MID, freedom: MID, physical: MID, ball_hunting: MID, mould: MID }
    }
}

impl Style {
    /// A unit's style: the head coach sets the program's, the coordinator
    /// brings his own, and the unit lands two-thirds toward the head coach
    /// (setting 6).
    pub fn blend(head: &Style, coordinator: &Style) -> Style {
        let m = |h: f64, c: f64| h + (c - h) / 3.0;
        Style {
            risk: m(head.risk, coordinator.risk),
            freedom: m(head.freedom, coordinator.freedom),
            physical: m(head.physical, coordinator.physical),
            ball_hunting: m(head.ball_hunting, coordinator.ball_hunting),
            mould: m(head.mould, coordinator.mould),
        }
    }

    /// The pull's size: a moulding staff doubles it, an adapting one halves it.
    pub fn mould_factor(&self) -> f64 {
        2f64.powf((self.mould - MID) / 9.5)
    }

    /// Where this style pulls a tendency, if it pulls it at all.
    pub fn target(&self, t: Tend) -> Option<f64> {
        let s = match t {
            Tend::ReturnRisk | Tend::Gunslinger | Tend::Hold | Tend::Reach => self.risk,
            Tend::Bounce | Tend::Scramble | Tend::SnapTiming | Tend::Downhill => self.freedom,
            Tend::JumpRoutes => 0.5 * self.freedom + 0.5 * self.ball_hunting,
            Tend::Contact | Tend::BigHit => self.physical,
            Tend::Strip | Tend::HandsUp => self.ball_hunting,
            Tend::Patience | Tend::Upfield => return None,
        };
        // A style sits anywhere on 1-20; it pulls toward the matching
        // number, a little short of the ends.
        Some(clamp(MID + 0.8 * (s - MID), 2.0, 19.0))
    }
}

/// Schemes pull too (setting 5): a timing offense pulls quarterbacks toward
/// getting it out, a vertical one toward the shot; a one-gap front pulls
/// rushers upfield and linebackers downhill, a two-gap front toward contain
/// and reading first. In tendency points the target moves.
pub fn scheme_target(t: Tend, off: OffScheme, front: Front) -> Option<f64> {
    use OffScheme as O;
    match t {
        Tend::Hold => match off {
            O::WestCoast => Some(MID - 3.0),
            O::AirRaid => Some(MID - 1.5),
            O::WideZone | O::PowerGap => Some(MID + 2.5),
            _ => None,
        },
        Tend::Gunslinger => match off {
            O::WestCoast => Some(MID - 2.0),
            O::WideZone | O::PowerGap => Some(MID + 1.5),
            _ => None,
        },
        Tend::Scramble => match off {
            O::SpreadOption => Some(MID + 3.0),
            _ => None,
        },
        Tend::Upfield | Tend::Downhill => match front {
            Front::FourThree => Some(MID + 2.0),
            Front::ThreeFour => Some(MID - 2.0),
        },
        _ => None,
    }
}

// ------------------------------------------------------- yearly change

/// What one year brings to his habits (topic 3).
#[derive(Clone, Copy, Debug)]
pub struct YearCtx {
    pub level: Level,
    /// His position coach's teaching, 1-20 (the blend development uses).
    pub teaching: f64,
    /// The unit's style, and the scheme it runs, if he has a team.
    pub style: Option<Style>,
    pub scheme: Option<(OffScheme, Front)>,
    /// He played this year (results only teach a man on the field), 0-1.
    pub played: f64,
    pub major_injury: bool,
    /// His speed lost this year, rating points (aging backs and
    /// quarterbacks adjust).
    pub speed_lost: f64,
    /// What his tools support, on average at his position and level, less
    /// the middle (`Centres`): taken off his ideal so the average man at
    /// each position sits at 10.5. Zero: read raw.
    pub centre: [f64; TEND_COUNT],
}

/// Habits closer than this to what his tools support are his style, not a
/// mistake: results, coaching and the staff's pull leave them (trial 8
/// finding 2: with every gap taught away, NFL habits spread 1.1-2.8 against
/// the agreed 3.2). Trial 9 finding 3: at 3 points the spread still could
/// not get much past the band, since everything outside it was pulled to
/// its edge (NFL 0.96-2.37 in the world; 1.85-2.59 over a career in the
/// unit test); at 5 the same test keeps 3.0-3.5.
pub const STYLE_BAND: f64 = 5.0;

/// The average of what players' tools support at each level and position
/// (trial 8 finding 3: `ideal` read every attribute against one speed norm,
/// so fast positions sat above the middle: backs' bounce 13.5 at FBS).
pub struct Centres {
    sum: Vec<[f64; TEND_COUNT]>,
    n: Vec<f64>,
}

fn level_index(level: Level) -> usize {
    match level {
        Level::HS => 0,
        Level::SmallCollege => 1,
        Level::FCS => 2,
        Level::FBS => 3,
        Level::NFL => 4,
    }
}

impl Centres {
    pub fn new() -> Centres {
        Centres { sum: vec![[0.0; TEND_COUNT]; 5 * POSITION_COUNT], n: vec![0.0; 5 * POSITION_COUNT] }
    }

    pub fn add(&mut self, level: Level, pos: Position, a: &Attributes) {
        let k = level_index(level) * POSITION_COUNT + pos.index();
        for t in Tend::ALL {
            if let Some(v) = ideal(t, a, level) {
                self.sum[k][t.index()] += v - MID;
            }
        }
        self.n[k] += 1.0;
    }

    /// The average less the middle at a level and position (zero with
    /// nobody there).
    pub fn get(&self, level: Level, pos: Position) -> [f64; TEND_COUNT] {
        let k = level_index(level) * POSITION_COUNT + pos.index();
        if self.n[k] < 1.0 {
            return [0.0; TEND_COUNT];
        }
        self.sum[k].map(|s| s / self.n[k])
    }
}

impl Default for Centres {
    fn default() -> Centres {
        Centres::new()
    }
}

/// One year of change (topic 3.3-3.6). From his own stream, so no other
/// draw in development moves.
pub fn develop_year(t: &mut Tendencies, a: &Attributes, pers: &Personality, age: u32, ctx: &YearCtx, id: u32) {
    let mut r = Rng::new(id.wrapping_mul(0x9E37_79B1) ^ age.wrapping_mul(0x85EB_CA6B) ^ 0x7E4D);
    // Habits settle by about 26 and barely move after (topic 3.6).
    let settle = clamp((26.5 - age as f64) / 4.5, 0.12, 1.0);
    // Football IQ sets how fast results teach (topic 3.3).
    let iq = crate::game::rules::trait_norm(ctx.level, Attr::FootballIq);
    let learn = clamp(1.0 + (a[Attr::FootballIq] - iq) / 25.0, 0.4, 1.6) * ctx.played;
    // Coaching a hurtful habit down (topic 3.4): about 3 a year for a strong
    // coach (16+) with a coachable player (15+), about 1.5 for an average
    // pair, nothing (or worse) under 8.
    let coach = clamp(0.375 * (ctx.teaching - 8.0) * (0.45 + 0.037 * pers.coachability), -0.5, 3.4);
    let mould = ctx.style.map_or(1.0, |s| s.mould_factor());
    for k in Tend::ALL {
        let i = k.index();
        let v = t.v[i];
        let mut natural = 0.0;
        let mut coached = 0.0;
        if let Some(id) = ideal(k, a, ctx.level) {
            let id = clamp(id - ctx.centre[i], 1.0, 20.0);
            let gap = id - v;
            // Inside the band a habit is his style; only what lies beyond
            // it costs him.
            let beyond = (gap.abs() - STYLE_BAND).max(0.0);
            // Results teach: about a point a year when they are clear.
            natural += gap.signum() * beyond.min(1.0 * learn) * if beyond > 1.0 { 1.0 } else { 0.5 };
            // Coaching takes off what hurts him, to the edge of the band; a
            // weak coach lets it grow.
            if beyond > 0.0 {
                coached += if coach >= 0.0 {
                    gap.signum() * (beyond - natural.abs()).max(0.0).min(coach)
                } else {
                    -gap.signum() * -coach
                };
            }
        }
        // The staff's style and scheme pull, at a third of the harm fix, to
        // the edge of the same band.
        let pull = (coach.max(0.5) / 3.0) * mould;
        for target in [ctx.style.and_then(|s| s.target(k)), ctx.scheme.and_then(|(o, f)| scheme_target(k, o, f))]
            .into_iter()
            .flatten()
        {
            let gap = target - v;
            natural += gap.signum() * (gap.abs() - STYLE_BAND).max(0.0).min(pull);
        }
        // A little drift of his own while he is young.
        natural += r.normal(0.0, 0.35) * settle;
        let mut dv = (natural + coached) * settle;
        // Aging: lost speed takes the scramble and the bounce with it.
        if matches!(k, Tend::Scramble | Tend::Bounce | Tend::ReturnRisk) && ctx.speed_lost > 0.0 {
            dv -= 0.25 * ctx.speed_lost;
        }
        let nv = clamp(v + dv, 1.0, 20.0);
        t.v[i] = nv;
        // The coached part leaves a trace: the old number keeps only the
        // moves he made on his own.
        let traced = coached != 0.0 || (t.was[i] - v).abs() > 0.01;
        t.was[i] = if traced { clamp(t.was[i] + natural * settle, 1.0, 20.0) } else { nv };
    }
    // A major injury: contact willingness dips a season or two and mostly
    // comes back (topic 3.6).
    t.contact_dip *= 0.4;
    if ctx.major_injury {
        t.contact_dip += r.range(2.0, 4.0);
    }
    if t.contact_dip < 0.1 {
        t.contact_dip = 0.0;
    }
}

// ------------------------------------------------------- what you see

/// How a tendency reads on his page, against his current position (topic
/// 2): the same man can be a 15 for contact among receivers and an 11 among
/// corners. Position means in tendency points.
pub fn position_mean(t: Tend, pos: Position) -> f64 {
    use Position as P;
    if t != Tend::Contact {
        return MID;
    }
    MID + match pos {
        P::QB => -1.5,
        P::RB => 1.5,
        P::WR => -1.0,
        P::TE => 1.0,
        P::OT | P::IOL | P::DT | P::EDGE => 2.0,
        P::LB => 2.0,
        P::CB => -1.5,
        P::S => 1.0,
        P::K | P::P | P::LS => -4.0,
    }
}

pub fn shown(t: Tend, value: f64, pos: Position) -> f64 {
    clamp(value - (position_mean(t, pos) - MID), 1.0, 20.0)
}

/// A plain word for a number.
pub fn word(value: f64) -> &'static str {
    match value {
        x if x < 4.5 => "almost never",
        x if x < 8.0 => "rarely",
        x if x < 13.0 => "sometimes",
        x if x < 16.5 => "often",
        _ => "almost always",
    }
}

/// A staff's read of one tendency (topic 4.1): a range on the 1-20 scale
/// plus a word. It narrows with film and the staff's quality: a junior with
/// 8 games on tape might read 12-17, your own starter after a season 15-16.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TendRead {
    pub low: u8,
    pub high: u8,
    pub word: &'static str,
}

pub fn scout(value: f64, games: f64, accuracy: f64, rng: &mut Rng) -> TendRead {
    let skill = 1.4 - 0.04 * accuracy;
    let sd = clamp(5.0 / (1.0 + games).sqrt() * skill * 2.2, 0.35, 4.0);
    let est = clamp(value + rng.normal(0.0, sd), 1.0, 20.0);
    let half = (1.1 * sd).max(0.5);
    TendRead {
        low: clamp((est - half).round(), 1.0, 20.0) as u8,
        high: clamp((est + half).round(), 1.0, 20.0) as u8,
        word: word(est),
    }
}

// -------------------------------------------------------- instructions

/// A coach's instruction to one player (topic 4.3), each a real coaching
/// instruction with what it must move in the engine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Instruction {
    /// Kick returners: take a knee if he catches it deeper than this many
    /// yards in the end zone.
    KneelDeeperThan(f64),
    /// Punt returners: fair catch inside the 10, let the rest bounce.
    FairCatchInsideTen,
    /// Quarterbacks: slide, don't take hits.
    Slide,
    /// Quarterbacks: take shots.
    TakeShots,
    /// Quarterbacks: protect the ball.
    ProtectBall,
    /// Ball carriers: ball high and tight, no reaching.
    HighAndTight,
    /// Pass rushers: keep contain.
    KeepContain,
    /// Pass rushers: no snap guessing.
    NoSnapGuessing,
    /// Tacklers: wrap up, no strips or big hits.
    WrapUp,
    /// Coverage: stay home, don't jump routes.
    StayHome,
}

impl Instruction {
    pub fn label(self) -> String {
        match self {
            Instruction::KneelDeeperThan(d) => format!("Kneel if deeper than {d:.0} yards"),
            Instruction::FairCatchInsideTen => "Fair catch inside the 10, let it bounce".into(),
            Instruction::Slide => "Slide, don't take hits".into(),
            Instruction::TakeShots => "Take shots".into(),
            Instruction::ProtectBall => "Protect the ball".into(),
            Instruction::HighAndTight => "Ball high and tight, no reaching".into(),
            Instruction::KeepContain => "Keep contain".into(),
            Instruction::NoSnapGuessing => "No snap guessing".into(),
            Instruction::WrapUp => "Wrap up, no strips or big hits".into(),
            Instruction::StayHome => "Stay home, don't jump routes".into(),
        }
    }

    /// The tendencies it sets, and to what, when he follows it.
    pub fn sets(self) -> &'static [(Tend, f64)] {
        match self {
            Instruction::KneelDeeperThan(_) | Instruction::FairCatchInsideTen => &[(Tend::ReturnRisk, 3.0)],
            Instruction::Slide => &[(Tend::Contact, 1.0)],
            Instruction::TakeShots => &[(Tend::Gunslinger, 18.0), (Tend::Hold, 16.0)],
            Instruction::ProtectBall => &[(Tend::Gunslinger, 3.0), (Tend::Hold, 5.0)],
            Instruction::HighAndTight => &[(Tend::Reach, 1.0)],
            Instruction::KeepContain => &[(Tend::Upfield, 2.0)],
            Instruction::NoSnapGuessing => &[(Tend::SnapTiming, 1.0)],
            Instruction::WrapUp => &[(Tend::Strip, 1.0), (Tend::BigHit, 2.0)],
            Instruction::StayHome => &[(Tend::JumpRoutes, 1.0)],
        }
    }
}

/// A player's instructions: at most a few, as real coaches give them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Orders(pub Vec<Instruction>);

impl Orders {
    pub fn has(&self, f: impl Fn(&Instruction) -> bool) -> bool {
        self.0.iter().any(f)
    }

    /// The number an instruction sets for this tendency, if any.
    pub fn sets(&self, t: Tend) -> Option<f64> {
        self.0.iter().flat_map(|i| i.sets().iter()).find(|x| x.0 == t).map(|x| x.1)
    }

    pub fn kneel_depth(&self) -> Option<f64> {
        self.0.iter().find_map(|i| if let Instruction::KneelDeeperThan(d) = i { Some(*d) } else { None })
    }
}

/// How often he follows an instruction (topic 4.3): discipline (1-20
/// against his level), coachability and his trust in the coach (0-1). A
/// disciplined, coachable player nearly every time; one with low discipline
/// about two times in three. A stricter coach's are followed more often.
pub fn obey_chance(discipline20: f64, coachability: f64, trust: f64, strictness: f64) -> f64 {
    clamp(
        0.86 + 0.022 * (discipline20 - 10.0)
            + 0.012 * (coachability - 10.0)
            + 0.12 * (trust - 0.5)
            + 0.006 * (strictness - 10.0),
        0.45,
        0.99,
    )
}

// ------------------------------------------------------------- fit notes

/// How much a habit is worth to a team (topic 1.3), in rough rating points
/// on his position: fit with his own attributes and with the coach's style.
/// The overall leaves tendencies out; a team's grade adds this.
pub fn team_fit(t: &Tendencies, a: &Attributes, pos: Position, level: Level, style: &Style) -> f64 {
    let mut sum = 0.0;
    for k in Tend::ALL.into_iter().filter(|k| k.owned_by(pos)) {
        let v = t.get(k);
        let own = -0.18 * misfit(k, v, a, level).abs();
        // An adapting coach counts fit to his style less (setting 4).
        let weight = 0.5 + 0.5 * (style.mould - 1.0) / 19.0;
        let theirs = style.target(k).map_or(0.0, |s| -0.06 * weight * (v - s).abs());
        sum += own + theirs;
    }
    // A corner who will tackle reads as quality for almost every team, even
    // though it costs hits (topic 1.3).
    if matches!(pos, Position::CB | Position::S) {
        sum += 0.12 * (t.get(Tend::Contact) - position_mean(Tend::Contact, pos));
    }
    sum
}

/// A short staff note for a board (topic 4.5), from the staff's read: the
/// habit to fix that costs him most, or the habit that fits the style best.
pub fn fit_note(t: &Tendencies, a: &Attributes, pos: Position, level: Level, style: &Style) -> Option<String> {
    let owned: Vec<Tend> = Tend::ALL.into_iter().filter(|k| k.owned_by(pos)).collect();
    let worst = owned
        .iter()
        .map(|&k| (k, misfit(k, t.get(k), a, level)))
        .filter(|x| x.1.abs() >= 3.5)
        .max_by(|x, y| x.1.abs().total_cmp(&y.1.abs()));
    if let Some((k, m)) = worst {
        let why = if m > 0.0 { "more than his tools pay for" } else { "his tools could do more" };
        let (lo, hi) = k.ends();
        return Some(format!("Habit to fix: {}, {why}", if m > 0.0 { hi } else { lo }));
    }
    let best = owned
        .iter()
        .filter_map(|&k| style.target(k).map(|s| (k, s, t.get(k))))
        .filter(|x| (x.1 - MID).abs() >= 3.0 && (x.2 - MID) * (x.1 - MID) > 0.0 && (x.2 - x.1).abs() < 3.0)
        .max_by(|x, y| (x.1 - MID).abs().total_cmp(&(y.1 - MID).abs()));
    best.map(|(k, s, _)| {
        let (lo, hi) = k.ends();
        format!("Fits your style: {}", if s > MID { hi } else { lo })
    })
}
