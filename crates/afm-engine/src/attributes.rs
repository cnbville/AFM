//! Player attributes on a 1-99 scale. Each attribute has a development profile
//! describing how mature it is in a 14-year-old, how quickly it grows, what
//! drives that growth, and when age starts to take it away.
//!
//! The list (67, agreed 2026-10-18) follows what real scouts grade on film,
//! position by position, and each attribute drives a moment in the match
//! engine (`docs/design/attributes.md`). Five more are hidden: consistency,
//! injury proneness, recovery, longevity and temperament (`player.rs`).

pub const MIN_RATING: f64 = 1.0;
pub const MAX_RATING: f64 = 99.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Attr {
    // physical
    Speed,
    Acceleration,
    Agility,
    /// Raw strength: what the weight room builds and the bench press reads.
    /// How strong he plays comes from strength, balance, bend, hands and mass together.
    Strength,
    /// Stays on his feet and in control through contact and at full speed.
    Balance,
    /// Bend: hips and ankles to corner the edge, sink the hips, flip in coverage.
    Flexibility,
    Jumping,
    Stamina,
    // passing
    ArmStrength,
    /// Placement 0-9 air yards.
    ShortAccuracy,
    /// Placement 10-19 air yards, the tight windows.
    MediumAccuracy,
    /// Placement 20+ air yards.
    DeepAccuracy,
    /// Ball flight: over the linebackers, under the safety, soft on short throws.
    Touch,
    /// Accuracy off platform: rolling out, off balance, across his body.
    ThrowOnMove,
    /// How quick and compact his throwing motion is.
    QuickRelease,
    /// Drops, timing with the route, resetting his base.
    Footwork,
    /// Feels the rush he can't see; climbs and slides instead of bailing.
    PocketPresence,
    /// Fakes and exchanges: play action, option fakes, handoffs, pitches.
    BallHandling,
    /// Looks off the safety and moves defenders with his eyes.
    EyeManipulation,
    // running
    BallSecurity,
    /// Sees the hole and the cutback, presses with patience, then commits.
    BallCarrierVision,
    Elusiveness,
    /// Pad level and leg drive: runs through arm tackles, falls forward.
    LegDrive,
    // receiving
    Catching,
    /// Wins the ball when two men are on it, for receivers and defenders alike.
    ContestedCatch,
    /// Reads the ball's flight: over the shoulder, back shoulder, punts.
    BallTracking,
    /// Toe-taps and contorting to a ball off target.
    BodyControl,
    RouteRunning,
    /// Beats press at the line.
    Release,
    // line play
    PassBlocking,
    RunBlocking,
    /// Pulls, climbs, screens, lead and downfield blocks: blocks on moving targets.
    BlockingInSpace,
    /// Punch timing and placement, hands inside, strike and re-strike.
    HandUse,
    /// Holds his ground against the bull rush, double teams and lead blocks.
    Anchor,
    PowerRush,
    FinesseRush,
    BlockShedding,
    // defense
    ManCoverage,
    ZoneCoverage,
    /// Jams and reroutes at the line; the punt vise.
    Press,
    /// Shows one coverage and plays another; baits throws.
    Disguise,
    Tackling,
    /// Takes the right angle, doesn't overrun, keeps leverage on the ball.
    Pursuit,
    /// Times the snap from depth, hits the gap, beats the back's pickup.
    Blitzing,
    /// Slips through the wash and the piles to the ball.
    Traffic,
    // kicking and snapping
    KickPower,
    /// Extra points and field goals inside about 40 yards.
    ShortKickAccuracy,
    /// Field goals from about 40 yards out.
    LongKickAccuracy,
    /// How fast the kick gets up: a low kick gets blocked.
    KickTrajectory,
    /// Kickoff placement and hang: corners, squibs, pooches, onside kicks.
    KickoffTechnique,
    /// Punt distance; the hang comes with it from the ball's flight.
    PuntPower,
    /// Directional punts, the coffin corner, pinning it inside the 20.
    PuntPlacement,
    /// The kicker's or punter's own time from the snap to the kick.
    GetOff,
    /// The long snap's accuracy and speed.
    LongSnapping,
    /// The center's shotgun and pistol snap.
    ShotgunSnap,
    // mental
    /// Knows his job and everyone's, learns and keeps the playbook, makes and hears calls.
    FootballIq,
    /// Diagnoses what he sees: run or pass, screens, route combinations, shells.
    PlayRecognition,
    /// Acts before the play shows: the snap, the break, the QB's eyes.
    Anticipation,
    /// Picks the right option once he sees it.
    Decisions,
    /// Calm, correct execution in the moment; counts more as the stakes rise.
    Composure,
    /// Focus snap after snap: noise, silent counts, late in long drives.
    Concentration,
    /// Plays with his eyes and in his gap; no grabbing, no jumping early.
    Discipline,
    /// Makes and passes on the calls: the QB, center, middle linebacker and safety.
    Communication,
    /// Plays through pain, stands in, goes over the middle.
    Toughness,
    /// Work rate: chases from the back side, keeps rushing, keeps blocking.
    Motor,
    /// The 50/50 ball, the 4th-and-1 pile, the one-on-one.
    Competitiveness,
    /// Willing to hit: strikes on contact, takes on blocks, finishes.
    Physicality,
}

pub const ATTRIBUTE_COUNT: usize = 67;

/// The groups a player's page shows (the proposal's nine, hidden aside).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Category {
    Physical,
    Passing,
    Running,
    Receiving,
    LinePlay,
    Defense,
    KickingAndSnapping,
    Mental,
}

impl Attr {
    pub const ALL: [Attr; ATTRIBUTE_COUNT] = [
        Attr::Speed,
        Attr::Acceleration,
        Attr::Agility,
        Attr::Strength,
        Attr::Balance,
        Attr::Flexibility,
        Attr::Jumping,
        Attr::Stamina,
        Attr::ArmStrength,
        Attr::ShortAccuracy,
        Attr::MediumAccuracy,
        Attr::DeepAccuracy,
        Attr::Touch,
        Attr::ThrowOnMove,
        Attr::QuickRelease,
        Attr::Footwork,
        Attr::PocketPresence,
        Attr::BallHandling,
        Attr::EyeManipulation,
        Attr::BallSecurity,
        Attr::BallCarrierVision,
        Attr::Elusiveness,
        Attr::LegDrive,
        Attr::Catching,
        Attr::ContestedCatch,
        Attr::BallTracking,
        Attr::BodyControl,
        Attr::RouteRunning,
        Attr::Release,
        Attr::PassBlocking,
        Attr::RunBlocking,
        Attr::BlockingInSpace,
        Attr::HandUse,
        Attr::Anchor,
        Attr::PowerRush,
        Attr::FinesseRush,
        Attr::BlockShedding,
        Attr::ManCoverage,
        Attr::ZoneCoverage,
        Attr::Press,
        Attr::Disguise,
        Attr::Tackling,
        Attr::Pursuit,
        Attr::Blitzing,
        Attr::Traffic,
        Attr::KickPower,
        Attr::ShortKickAccuracy,
        Attr::LongKickAccuracy,
        Attr::KickTrajectory,
        Attr::KickoffTechnique,
        Attr::PuntPower,
        Attr::PuntPlacement,
        Attr::GetOff,
        Attr::LongSnapping,
        Attr::ShotgunSnap,
        Attr::FootballIq,
        Attr::PlayRecognition,
        Attr::Anticipation,
        Attr::Decisions,
        Attr::Composure,
        Attr::Concentration,
        Attr::Discipline,
        Attr::Communication,
        Attr::Toughness,
        Attr::Motor,
        Attr::Competitiveness,
        Attr::Physicality,
    ];

    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn key(self) -> &'static str {
        ATTRIBUTE_KEYS[self as usize]
    }

    /// The name a player's page shows.
    pub fn label(self) -> &'static str {
        ATTRIBUTE_LABELS[self as usize]
    }

    #[inline]
    pub fn profile(self) -> &'static AttributeProfile {
        &ATTRIBUTE_PROFILES[self as usize]
    }

    pub fn category(self) -> Category {
        use Attr::*;
        match self {
            Speed | Acceleration | Agility | Strength | Balance | Flexibility | Jumping | Stamina => Category::Physical,
            ArmStrength | ShortAccuracy | MediumAccuracy | DeepAccuracy | Touch | ThrowOnMove | QuickRelease
            | Footwork | PocketPresence | BallHandling | EyeManipulation => Category::Passing,
            BallSecurity | BallCarrierVision | Elusiveness | LegDrive => Category::Running,
            Catching | ContestedCatch | BallTracking | BodyControl | RouteRunning | Release => Category::Receiving,
            PassBlocking | RunBlocking | BlockingInSpace | HandUse | Anchor | PowerRush | FinesseRush
            | BlockShedding => Category::LinePlay,
            ManCoverage | ZoneCoverage | Press | Disguise | Tackling | Pursuit | Blitzing | Traffic => {
                Category::Defense
            }
            KickPower | ShortKickAccuracy | LongKickAccuracy | KickTrajectory | KickoffTechnique | PuntPower
            | PuntPlacement | GetOff | LongSnapping | ShotgunSnap => Category::KickingAndSnapping,
            _ => Category::Mental,
        }
    }
}

/// camelCase names, as in the trial files and the raw export.
pub const ATTRIBUTE_KEYS: [&str; ATTRIBUTE_COUNT] = [
    "speed",
    "acceleration",
    "agility",
    "strength",
    "balance",
    "flexibility",
    "jumping",
    "stamina",
    "armStrength",
    "shortAccuracy",
    "mediumAccuracy",
    "deepAccuracy",
    "touch",
    "throwOnMove",
    "quickRelease",
    "footwork",
    "pocketPresence",
    "ballHandling",
    "eyeManipulation",
    "ballSecurity",
    "ballCarrierVision",
    "elusiveness",
    "legDrive",
    "catching",
    "contestedCatch",
    "ballTracking",
    "bodyControl",
    "routeRunning",
    "release",
    "passBlocking",
    "runBlocking",
    "blockingInSpace",
    "handUse",
    "anchor",
    "powerRush",
    "finesseRush",
    "blockShedding",
    "manCoverage",
    "zoneCoverage",
    "press",
    "disguise",
    "tackling",
    "pursuit",
    "blitzing",
    "traffic",
    "kickPower",
    "shortKickAccuracy",
    "longKickAccuracy",
    "kickTrajectory",
    "kickoffTechnique",
    "puntPower",
    "puntPlacement",
    "getOff",
    "longSnapping",
    "shotgunSnap",
    "footballIq",
    "playRecognition",
    "anticipation",
    "decisions",
    "composure",
    "concentration",
    "discipline",
    "communication",
    "toughness",
    "motor",
    "competitiveness",
    "physicality",
];

pub const ATTRIBUTE_LABELS: [&str; ATTRIBUTE_COUNT] = [
    "Speed",
    "Acceleration",
    "Agility",
    "Strength",
    "Balance",
    "Flexibility",
    "Jumping",
    "Stamina",
    "Arm strength",
    "Short accuracy",
    "Medium accuracy",
    "Deep accuracy",
    "Touch",
    "Throwing on the move",
    "Release speed",
    "Footwork",
    "Pocket presence",
    "Ball handling",
    "Eye manipulation",
    "Ball security",
    "Vision",
    "Elusiveness",
    "Leg drive",
    "Catching",
    "Contested catches",
    "Ball tracking",
    "Body control",
    "Route running",
    "Release off the line",
    "Pass blocking",
    "Run blocking",
    "Blocking in space",
    "Hand use",
    "Anchor",
    "Power rush",
    "Finesse rush",
    "Block shedding",
    "Man coverage",
    "Zone coverage",
    "Press",
    "Disguise",
    "Tackling",
    "Pursuit",
    "Blitzing",
    "Moving through traffic",
    "Kick power",
    "Short kick accuracy",
    "Long kick accuracy",
    "Kick trajectory",
    "Kickoff technique",
    "Punt power",
    "Punt placement",
    "Get-off time",
    "Long snapping",
    "Shotgun snapping",
    "Football IQ",
    "Play recognition",
    "Anticipation",
    "Decisions",
    "Composure",
    "Concentration",
    "Discipline",
    "Communication",
    "Toughness",
    "Motor",
    "Competitiveness",
    "Physicality",
];

/// How an attribute develops: physical ones grow with the body, technical
/// ones with coaching and reps, mental ones with games.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AttributeGroup {
    Physical,
    Technical,
    Mental,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct AttributeProfile {
    pub group: AttributeGroup,
    /// Share of the eventual ceiling a typical 14-year-old already has.
    pub maturity_at_14: f64,
    /// Fraction of the remaining gap to the ceiling closed in a normal year at prime development age.
    pub growth_rate: f64,
    /// Ages (inclusive) over which growth runs at full rate before tapering.
    pub growth_window: (f64, f64),
    /// How much growth depends on coaching and training (0 = innate, 1 = fully coached).
    pub coachability: f64,
    /// How much growth depends on game snaps (0 = not at all, 1 = only learned by playing).
    pub experience: f64,
    /// Typical age decline begins, before player and position adjustments.
    pub decline_age: f64,
    /// Points lost per year one year past decline_age; grows linearly each year after.
    pub decline_rate: f64,
    /// Whether a major injury can permanently cost this attribute.
    pub injury_sensitive: bool,
    /// Share lost on the field when he's spent (topic 2): legs and strength
    /// go first, then sharpness, then skill.
    pub fatigue: f64,
}

#[allow(clippy::too_many_arguments)]
const fn physical(m: f64, g: f64, w: (f64, f64), c: f64, da: f64, dr: f64, inj: bool, fat: f64) -> AttributeProfile {
    AttributeProfile {
        group: AttributeGroup::Physical,
        maturity_at_14: m,
        growth_rate: g,
        growth_window: w,
        coachability: c,
        experience: 0.1,
        decline_age: da,
        decline_rate: dr,
        injury_sensitive: inj,
        fatigue: fat,
    }
}

const fn technical(m: f64, g: f64, w: (f64, f64), c: f64, e: f64, da: f64, dr: f64) -> AttributeProfile {
    AttributeProfile {
        group: AttributeGroup::Technical,
        maturity_at_14: m,
        growth_rate: g,
        growth_window: w,
        coachability: c,
        experience: e,
        decline_age: da,
        decline_rate: dr,
        injury_sensitive: false,
        fatigue: 0.06,
    }
}

#[allow(clippy::too_many_arguments)]
const fn mental(m: f64, g: f64, w: (f64, f64), c: f64, e: f64, da: f64, dr: f64, fat: f64) -> AttributeProfile {
    AttributeProfile {
        group: AttributeGroup::Mental,
        maturity_at_14: m,
        growth_rate: g,
        growth_window: w,
        coachability: c,
        experience: e,
        decline_age: da,
        decline_rate: dr,
        injury_sensitive: false,
        fatigue: fat,
    }
}

/// Indexed by `Attr`. The 25 attributes the game had before keep their
/// profiles; the new ones are set beside the nearest of them (proposed, not
/// confirmed): bend and balance are mostly given and go early, the hand
/// fight and footwork are taught and last, the reads come with games.
pub const ATTRIBUTE_PROFILES: [AttributeProfile; ATTRIBUTE_COUNT] = [
    // Speed is largely genetic: high at 14, modest gains, first thing to go.
    physical(0.8, 0.24, (14.0, 20.0), 0.25, 27.0, 0.9, true, 0.10), // speed
    physical(0.76, 0.24, (14.0, 21.0), 0.35, 27.0, 0.8, true, 0.10), // acceleration
    physical(0.74, 0.24, (14.0, 21.0), 0.35, 27.0, 0.8, true, 0.10), // agility
    // Strength is built in the weight room through the college years.
    physical(0.48, 0.2, (15.0, 24.0), 0.8, 31.0, 0.6, false, 0.12), // strength
    physical(0.66, 0.2, (14.0, 22.0), 0.45, 29.0, 0.7, true, 0.10), // balance
    // Bend is mostly given and stiffens with age.
    physical(0.78, 0.18, (14.0, 20.0), 0.3, 27.0, 0.8, true, 0.06), // flexibility
    physical(0.74, 0.22, (14.0, 21.0), 0.3, 28.0, 0.8, true, 0.06), // jumping
    physical(0.62, 0.22, (14.0, 24.0), 0.7, 30.0, 0.6, false, 0.06), // stamina
    technical(0.58, 0.2, (14.0, 22.0), 0.5, 0.2, 34.0, 0.6),        // armStrength
    technical(0.52, 0.16, (15.0, 27.0), 0.8, 0.6, 36.0, 0.5),       // shortAccuracy
    technical(0.48, 0.15, (15.0, 27.0), 0.8, 0.6, 35.0, 0.6),       // mediumAccuracy
    technical(0.44, 0.14, (15.0, 27.0), 0.7, 0.6, 34.0, 0.6),       // deepAccuracy
    technical(0.5, 0.14, (15.0, 28.0), 0.6, 0.6, 36.0, 0.4),        // touch
    technical(0.55, 0.16, (14.0, 26.0), 0.6, 0.5, 32.0, 0.7),       // throwOnMove
    technical(0.55, 0.15, (14.0, 26.0), 0.7, 0.4, 35.0, 0.4),       // quickRelease
    technical(0.42, 0.16, (15.0, 28.0), 0.85, 0.6, 35.0, 0.5),      // footwork
    technical(0.38, 0.13, (16.0, 31.0), 0.6, 0.8, 36.0, 0.4),       // pocketPresence
    technical(0.5, 0.17, (14.0, 24.0), 0.75, 0.4, 35.0, 0.4),       // ballHandling
    technical(0.35, 0.12, (16.0, 30.0), 0.6, 0.8, 37.0, 0.3),       // eyeManipulation
    technical(0.58, 0.18, (14.0, 24.0), 0.7, 0.4, 32.0, 0.5),       // ballSecurity
    mental(0.55, 0.14, (14.0, 26.0), 0.4, 0.6, 31.0, 0.6, 0.08),    // ballCarrierVision
    technical(0.64, 0.18, (14.0, 22.0), 0.3, 0.4, 27.0, 0.8),       // elusiveness
    technical(0.6, 0.18, (14.0, 23.0), 0.5, 0.4, 29.0, 0.7),        // legDrive
    technical(0.56, 0.17, (14.0, 24.0), 0.6, 0.4, 32.0, 0.5),       // catching
    technical(0.5, 0.16, (14.0, 25.0), 0.5, 0.5, 31.0, 0.6),        // contestedCatch
    technical(0.5, 0.16, (14.0, 25.0), 0.5, 0.6, 33.0, 0.5),        // ballTracking
    technical(0.62, 0.18, (14.0, 22.0), 0.4, 0.4, 29.0, 0.8),       // bodyControl
    technical(0.46, 0.15, (15.0, 26.0), 0.8, 0.6, 32.0, 0.5),       // routeRunning
    technical(0.48, 0.16, (15.0, 25.0), 0.75, 0.5, 31.0, 0.6),      // release
    technical(0.42, 0.15, (15.0, 27.0), 0.8, 0.7, 32.0, 0.5),       // passBlocking
    technical(0.46, 0.16, (15.0, 26.0), 0.8, 0.6, 32.0, 0.5),       // runBlocking
    technical(0.48, 0.16, (15.0, 25.0), 0.75, 0.5, 30.0, 0.7),      // blockingInSpace
    technical(0.42, 0.15, (15.0, 27.0), 0.85, 0.6, 33.0, 0.4),      // handUse
    technical(0.5, 0.17, (15.0, 25.0), 0.6, 0.5, 32.0, 0.5),        // anchor
    technical(0.48, 0.16, (15.0, 26.0), 0.7, 0.6, 31.0, 0.6),       // powerRush
    technical(0.46, 0.16, (15.0, 26.0), 0.75, 0.6, 30.0, 0.7),      // finesseRush
    technical(0.46, 0.16, (15.0, 26.0), 0.7, 0.6, 31.0, 0.6),       // blockShedding
    technical(0.48, 0.16, (15.0, 25.0), 0.7, 0.6, 29.0, 0.8),       // manCoverage
    technical(0.46, 0.15, (15.0, 27.0), 0.7, 0.7, 32.0, 0.5),       // zoneCoverage
    technical(0.48, 0.16, (15.0, 25.0), 0.75, 0.5, 30.0, 0.7),      // press
    technical(0.36, 0.12, (16.0, 29.0), 0.6, 0.8, 34.0, 0.4),       // disguise
    technical(0.52, 0.17, (14.0, 24.0), 0.7, 0.5, 31.0, 0.6),       // tackling
    technical(0.5, 0.16, (14.0, 25.0), 0.6, 0.6, 31.0, 0.6),        // pursuit
    technical(0.45, 0.15, (15.0, 26.0), 0.6, 0.7, 31.0, 0.6),       // blitzing
    technical(0.48, 0.15, (15.0, 26.0), 0.55, 0.7, 31.0, 0.6),      // traffic
    technical(0.62, 0.2, (14.0, 23.0), 0.5, 0.1, 36.0, 0.5),        // kickPower
    technical(0.52, 0.16, (14.0, 28.0), 0.8, 0.4, 38.0, 0.4),       // shortKickAccuracy
    technical(0.48, 0.16, (14.0, 28.0), 0.8, 0.4, 36.0, 0.5),       // longKickAccuracy
    technical(0.55, 0.16, (14.0, 26.0), 0.75, 0.3, 37.0, 0.4),      // kickTrajectory
    technical(0.5, 0.16, (14.0, 27.0), 0.8, 0.3, 37.0, 0.4),        // kickoffTechnique
    technical(0.6, 0.2, (14.0, 24.0), 0.5, 0.1, 36.0, 0.5),         // puntPower
    technical(0.48, 0.16, (14.0, 28.0), 0.8, 0.4, 38.0, 0.4),       // puntPlacement
    technical(0.55, 0.16, (14.0, 26.0), 0.8, 0.3, 37.0, 0.4),       // getOff
    technical(0.45, 0.17, (14.0, 27.0), 0.85, 0.3, 38.0, 0.3),      // longSnapping
    technical(0.5, 0.17, (14.0, 26.0), 0.85, 0.4, 37.0, 0.3),       // shotgunSnap
    // Mental attributes are learned by playing and keep improving into the 30s.
    mental(0.4, 0.13, (16.0, 30.0), 0.4, 0.8, 35.0, 0.5, 0.08), // footballIq
    mental(0.4, 0.13, (16.0, 30.0), 0.4, 0.8, 34.0, 0.5, 0.08), // playRecognition
    mental(0.38, 0.13, (16.0, 30.0), 0.4, 0.8, 34.0, 0.5, 0.08), // anticipation
    mental(0.4, 0.13, (16.0, 30.0), 0.4, 0.8, 35.0, 0.5, 0.08), // decisions
    mental(0.42, 0.12, (16.0, 31.0), 0.4, 0.8, 36.0, 0.4, 0.08), // composure
    mental(0.45, 0.12, (15.0, 28.0), 0.4, 0.6, 35.0, 0.4, 0.10), // concentration
    mental(0.45, 0.12, (15.0, 28.0), 0.5, 0.6, 35.0, 0.3, 0.06), // discipline
    mental(0.38, 0.13, (16.0, 30.0), 0.4, 0.8, 36.0, 0.4, 0.06), // communication
    // The will to compete is mostly who he is.
    mental(0.6, 0.1, (14.0, 24.0), 0.3, 0.4, 33.0, 0.4, 0.04), // toughness
    mental(0.62, 0.08, (14.0, 22.0), 0.3, 0.3, 31.0, 0.6, 0.04), // motor
    mental(0.68, 0.06, (14.0, 22.0), 0.2, 0.2, 34.0, 0.3, 0.02), // competitiveness
    mental(0.6, 0.1, (14.0, 23.0), 0.3, 0.4, 32.0, 0.5, 0.06), // physicality
];

/// One value per attribute. Stored as 4-byte numbers (section 8, memory):
/// a rating needs no more than f32's 7 digits, and the 67 of them for every
/// player and his ceilings were 0.45 GB of a full world as 8-byte numbers.
/// Reads and writes go through f64, so every formula keeps working in f64.
/// A world file written with 8-byte numbers loads into it.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Attributes(#[serde(with = "serde_big_array::BigArray")] [f32; ATTRIBUTE_COUNT]);

impl Attributes {
    pub const fn filled(value: f64) -> Self {
        Attributes([value as f32; ATTRIBUTE_COUNT])
    }

    #[inline]
    pub fn get(&self, a: Attr) -> f64 {
        self.0[a as usize] as f64
    }

    #[inline]
    pub fn set(&mut self, a: Attr, v: f64) {
        self.0[a as usize] = v as f32;
    }

    /// Adds `d` to one attribute.
    #[inline]
    pub fn add(&mut self, a: Attr, d: f64) {
        let i = a as usize;
        self.0[i] = (self.0[i] as f64 + d) as f32;
    }

    /// Every value, in `Attr` order.
    #[inline]
    pub fn values(&self) -> impl Iterator<Item = f64> + '_ {
        self.0.iter().map(|&v| v as f64)
    }

    /// The value at a raw index (`Attr as usize`).
    #[inline]
    pub fn at(&self, i: usize) -> f64 {
        self.0[i] as f64
    }

    #[inline]
    pub fn set_at(&mut self, i: usize, v: f64) {
        self.0[i] = v as f32;
    }
}

impl Default for Attributes {
    fn default() -> Self {
        Attributes::filled(MIN_RATING)
    }
}
