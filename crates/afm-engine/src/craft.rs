//! Individual practice (`docs/design/individual-practice.md`): flaws and
//! tools, a layer of technique under the ratings.
//!
//! A flaw is one specific way of losing one battle ("oversets", "rounds his
//! breaks"); a tool is something added to his game (a counter move, a
//! release package). Each sits inside an engine moment an attribute already
//! owns and moves that attribute's read in one narrow slice, the way
//! Dynamic Links do (`links.rs`): an overset tackle's pass set reads lower
//! only against speed and the counter, and against power he plays to his
//! rating. A full flaw costs what 5 rating points of its owner would cost
//! there, a full tool adds the same (sizes, item 1, confirmed). Severity
//! grows and shrinks smoothly from 0 to 1: no cliffs.
//!
//! Where they come from: at 14, from who he is (his ratings, body and
//! habits lean him toward some), with chance for the rest; then coaching
//! shapes the count year by year, results teach, and players who never
//! fix theirs don't make it. All of it draws from his own keyed stream, so
//! no other random draw in the world moves.
//!
//! The store is small (memory gate): six slots of five bytes per player.

use crate::attributes::Attr;
use crate::player::{Level, Player};
use crate::positions::Position;
use crate::rng::{Rng, clamp};

/// Rating points a full flaw costs, and a full tool adds, in its own
/// situation (sizes, item 1, confirmed: the size of a full link).
pub const FULL: f64 = 5.0;

/// A flaw or a tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Craft {
    // Quarterbacks.
    EyesDown,
    BackFoot,
    FirstRead,
    StaresDown,
    LongMotion,
    PumpFake,
    HardCount,
    BackShoulder,
    // Running backs.
    DucksPickup,
    Upright,
    SlowMesh,
    JumpCut,
    StiffArm,
    // Receivers and tight ends in routes.
    RoundsBreaks,
    TipsBreaks,
    BodyCatcher,
    OneRelease,
    ReleasePackage,
    LateHands,
    SuddenStop,
    // The offensive line and tight ends blocking.
    Oversets,
    CatchesPunch,
    OverToes,
    HighPadBlock,
    HeavyClimb,
    RefitsHands,
    // The defensive line and edges.
    NoPlan,
    HighPadRush,
    ChestBlocker,
    EyesBackfield,
    RealCounter,
    LongArm,
    Spin,
    // Linebackers.
    CatchesBlocks,
    Overruns,
    NoDepth,
    WheelLoss,
    ShockShed,
    AGapRip,
    // Corners.
    Peeks,
    OpensHips,
    Lunges,
    NoHands,
    MirrorPress,
    Bait,
    // Safeties.
    PoorAngles,
    EyesOnQb,
    LateRotation,
    RobberBait,
    // Specialists.
    PlantDrift,
    DropInconsistent,
    WetSnaps,
    CoffinCorner,
    OnsideTouch,
    // Ball carriers and tacklers.
    AnkleDiver,
    CarriesLoose,
}

pub const CRAFT_COUNT: usize = 56;

/// What a craft is: a flaw (a way of losing) or a tool (a way of winning).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Flaw,
    Tool,
}

/// Which players can carry it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    Qb,
    Back,
    /// Receivers and tight ends (routes and catching).
    Catcher,
    /// Tackles, guards, centers and tight ends.
    Blocker,
    /// Edges and defensive tackles.
    Rusher,
    Lb,
    Cb,
    /// Corners and safeties.
    Db,
    S,
    K,
    P,
    Ls,
    /// Every defender and every skill player who tackles: anyone but a
    /// specialist.
    Tackler,
    /// Anyone who carries the ball: backs, receivers, tight ends and
    /// quarterbacks.
    Carrier,
}

impl Who {
    pub fn fits(self, pos: Position) -> bool {
        use Position::*;
        match self {
            Who::Qb => pos == QB,
            Who::Back => pos == RB,
            Who::Catcher => matches!(pos, WR | TE),
            Who::Blocker => matches!(pos, OT | IOL | TE),
            Who::Rusher => matches!(pos, EDGE | DT),
            Who::Lb => pos == LB,
            Who::Cb => pos == CB,
            Who::Db => matches!(pos, CB | S),
            Who::S => pos == S,
            Who::K => pos == K,
            Who::P => pos == P,
            Who::Ls => pos == LS,
            Who::Tackler => matches!(pos, EDGE | DT | LB | CB | S),
            Who::Carrier => matches!(pos, RB | WR | TE | QB),
        }
    }
}

/// One entry of the list (topic 2): its name in plain words, the tell on
/// tape, what it is, who carries it, the rating it belongs to (a focus
/// grows it, topic 5b) and whether it is worked in the film room rather
/// than on the field.
#[derive(Clone, Copy, Debug)]
pub struct Info {
    pub name: &'static str,
    pub tell: &'static str,
    /// What it does on the field, in plain words.
    pub what: &'static str,
    pub kind: Kind,
    pub who: Who,
    pub owner: Attr,
    pub mental: bool,
}

const fn info(
    name: &'static str,
    what: &'static str,
    tell: &'static str,
    kind: Kind,
    who: Who,
    owner: Attr,
    mental: bool,
) -> Info {
    Info { name, tell, what, kind, who, owner, mental }
}

use Kind::{Flaw, Tool};

static INFO: [Info; CRAFT_COUNT] = [
    info(
        "Drops his eyes to the rush",
        "His feel for the pocket shrinks once a rusher is close.",
        "Stares at the rush and misses the open man.",
        Flaw,
        Who::Qb,
        Attr::PocketPresence,
        true,
    ),
    info(
        "Throws off his back foot",
        "His accuracy falls further than it should when the pocket closes.",
        "Balls sail high when the pocket closes.",
        Flaw,
        Who::Qb,
        Attr::Footwork,
        false,
    ),
    info(
        "Locks onto his first read",
        "Each read past the first comes later.",
        "The second read was open and he never got there.",
        Flaw,
        Who::Qb,
        Attr::Decisions,
        true,
    ),
    info(
        "Stares down his target",
        "Defenders read his short throws early.",
        "The linebacker undercuts the slant.",
        Flaw,
        Who::Qb,
        Attr::EyeManipulation,
        true,
    ),
    info(
        "Long motion in traffic",
        "His release slows with a rusher in reach.",
        "The ball comes out low and slow.",
        Flaw,
        Who::Qb,
        Attr::QuickRelease,
        false,
    ),
    info(
        "Pump fake that moves safeties",
        "His eyes and shoulders move the safety on double moves and the second read.",
        "The safety bites on the pump.",
        Tool,
        Who::Qb,
        Attr::EyeManipulation,
        false,
    ),
    info(
        "Hard count that draws flags",
        "Rushers who guess the count jump more often.",
        "The edge jumps before the snap.",
        Tool,
        Who::Qb,
        Attr::FootballIq,
        false,
    ),
    info(
        "Back-shoulder throw",
        "On covered sideline throws he puts it where only his man can turn for it.",
        "The receiver turns back for it and the corner can't.",
        Tool,
        Who::Qb,
        Attr::Touch,
        false,
    ),
    info(
        "Ducks his head in pickup",
        "Blitz pickup misses more against a rusher who moves late.",
        "The linebacker sidesteps him.",
        Flaw,
        Who::Back,
        Attr::PassBlocking,
        false,
    ),
    info(
        "Runs too upright",
        "His leg drive counts for less at first contact.",
        "Goes down at the first hit.",
        Flaw,
        Who::Back,
        Attr::LegDrive,
        false,
    ),
    info(
        "Slow through the mesh",
        "His press to the line starts later.",
        "The hole closes before he gets there.",
        Flaw,
        Who::Back,
        Attr::BallCarrierVision,
        false,
    ),
    info(
        "Jump cut",
        "Makes a filling linebacker miss in the backfield.",
        "The linebacker grabs air in the hole.",
        Tool,
        Who::Back,
        Attr::Elusiveness,
        false,
    ),
    info(
        "Stiff arm",
        "Breaks arm tackles in the open field.",
        "The arm tackle bounces off.",
        Tool,
        Who::Carrier,
        Attr::LegDrive,
        false,
    ),
    info(
        "Rounds his breaks",
        "Separates less on outs and comebacks.",
        "The corner stays on his hip.",
        Flaw,
        Who::Catcher,
        Attr::RouteRunning,
        false,
    ),
    info(
        "Tips his breaks",
        "Gears down before the cut, so defenders read it early.",
        "The corner jumps the route.",
        Flaw,
        Who::Catcher,
        Attr::RouteRunning,
        false,
    ),
    info(
        "Body catcher",
        "Catches less off his frame and under contact.",
        "It bounces off his pads.",
        Flaw,
        Who::Catcher,
        Attr::Catching,
        false,
    ),
    info(
        "One release against press",
        "Loses at the line against a pressing corner.",
        "Jammed at the line on third down.",
        Flaw,
        Who::Catcher,
        Attr::Release,
        false,
    ),
    info(
        "Release package",
        "Wins at the line against press.",
        "Gone off the line before the jam lands.",
        Tool,
        Who::Catcher,
        Attr::Release,
        false,
    ),
    info(
        "Late hands",
        "Wins contested catches: the corner can't time his hands.",
        "The hands come up late and the ball sticks.",
        Tool,
        Who::Catcher,
        Attr::ContestedCatch,
        false,
    ),
    info(
        "Sudden stop",
        "Separates on comebacks and hitches.",
        "Stops on a dime and the corner runs past.",
        Tool,
        Who::Catcher,
        Attr::RouteRunning,
        false,
    ),
    info(
        "Oversets",
        "His pass set reads lower against speed and the inside counter.",
        "Beaten inside by the counter.",
        Flaw,
        Who::Blocker,
        Attr::PassBlocking,
        false,
    ),
    info(
        "Catches instead of punching",
        "His anchor reads lower against power.",
        "Walked back into the quarterback.",
        Flaw,
        Who::Blocker,
        Attr::Anchor,
        false,
    ),
    info(
        "Over his toes",
        "His run block falls off against finesse moves.",
        "The swim leaves him on the ground.",
        Flaw,
        Who::Blocker,
        Attr::RunBlocking,
        false,
    ),
    info(
        "High pad level",
        "Loses on short yardage and double teams.",
        "No push at the goal line.",
        Flaw,
        Who::Blocker,
        Attr::RunBlocking,
        false,
    ),
    info(
        "Heavy feet on the climb",
        "Misses more on the way to the linebacker.",
        "The linebacker scrapes free.",
        Flaw,
        Who::Blocker,
        Attr::BlockingInSpace,
        false,
    ),
    info(
        "Re-fits his hands",
        "Wins back a rep that started to go wrong.",
        "Loses the first punch and still wins the rep.",
        Tool,
        Who::Blocker,
        Attr::HandUse,
        false,
    ),
    info(
        "No plan",
        "His counter comes less often and fits the set worse.",
        "A stalled rush dies.",
        Flaw,
        Who::Rusher,
        Attr::FinesseRush,
        true,
    ),
    info(
        "High pad level",
        "Gets washed out on double teams.",
        "Driven off the ball.",
        Flaw,
        Who::Rusher,
        Attr::Anchor,
        false,
    ),
    info(
        "Lets the blocker into his chest",
        "Sheds less against a punch.",
        "Locked up all game.",
        Flaw,
        Who::Rusher,
        Attr::BlockShedding,
        false,
    ),
    info(
        "Eyes in the backfield",
        "Loses his gap on traps and misdirection.",
        "Trapped.",
        Flaw,
        Who::Rusher,
        Attr::PlayRecognition,
        true,
    ),
    info(
        "Real counter",
        "His counter comes more often and fits the set better.",
        "Wins with the second move.",
        Tool,
        Who::Rusher,
        Attr::FinesseRush,
        false,
    ),
    info(
        "Long arm",
        "His power rush gains against a short set.",
        "The tackle's chest caves in.",
        Tool,
        Who::Rusher,
        Attr::PowerRush,
        false,
    ),
    info(
        "Spin",
        "His finesse rush gains against a tackle who oversets.",
        "Spins inside and he's free.",
        Tool,
        Who::Rusher,
        Attr::FinesseRush,
        false,
    ),
    info(
        "Catches blocks",
        "Sheds less against climbing guards.",
        "Glued to the guard.",
        Flaw,
        Who::Lb,
        Attr::BlockShedding,
        false,
    ),
    info(
        "Overruns the ball",
        "His pursuit angle loses to cutbacks.",
        "The back cuts behind him.",
        Flaw,
        Who::Lb,
        Attr::Pursuit,
        false,
    ),
    info(
        "No depth in his drop",
        "Leaves the window behind him in zone.",
        "The dig fits in behind him.",
        Flaw,
        Who::Lb,
        Attr::ZoneCoverage,
        false,
    ),
    info(
        "Loses the back on the wheel",
        "His man coverage on backs falls off on the wheel.",
        "The back runs free up the sideline.",
        Flaw,
        Who::Lb,
        Attr::ManCoverage,
        false,
    ),
    info(
        "Shock and shed",
        "Sheds blocks with his hands.",
        "Stacks the guard and throws him aside.",
        Tool,
        Who::Lb,
        Attr::BlockShedding,
        false,
    ),
    info(
        "A-gap rip",
        "Wins as a blitzer against the guard.",
        "Rips through the A gap clean.",
        Tool,
        Who::Lb,
        Attr::Blitzing,
        false,
    ),
    info(
        "Peeks into the backfield",
        "His man coverage loses to double moves and play action.",
        "Beaten deep on the sluggo.",
        Flaw,
        Who::Cb,
        Attr::ManCoverage,
        true,
    ),
    info(
        "Opens his hips early",
        "His man coverage loses to inside breaks.",
        "The slant was there all day.",
        Flaw,
        Who::Cb,
        Attr::ManCoverage,
        false,
    ),
    info(
        "Lunges at the jam",
        "Whiffs more in press, and is beaten clean when he does.",
        "On the ground at the line.",
        Flaw,
        Who::Cb,
        Attr::Press,
        false,
    ),
    info(
        "Doesn't play the hands",
        "Defends less at the catch point.",
        "In position and the catch still made.",
        Flaw,
        Who::Db,
        Attr::Catching,
        false,
    ),
    info(
        "Mirror press",
        "Patient feet: press gains without the whiff.",
        "Never lets the receiver off the line clean.",
        Tool,
        Who::Cb,
        Attr::Press,
        false,
    ),
    info(
        "Bait coverage",
        "Shows open and closes, so he reads throws into his zone early.",
        "The quarterback throws it right to him.",
        Tool,
        Who::Cb,
        Attr::Anticipation,
        true,
    ),
    info(
        "Poor angles",
        "His pursuit and open-field tackles lose to cutbacks.",
        "A 12-yard run becomes 60.",
        Flaw,
        Who::S,
        Attr::Pursuit,
        false,
    ),
    info(
        "Eyes stuck on the quarterback",
        "A quarterback's eyes move him more.",
        "The post behind him.",
        Flaw,
        Who::S,
        Attr::PlayRecognition,
        true,
    ),
    info(
        "Late rotation",
        "Turns his disguise to the true coverage late.",
        "The seam is open after the snap.",
        Flaw,
        Who::S,
        Attr::Disguise,
        true,
    ),
    info(
        "Robber bait",
        "Reads in-breaking routes in front of him early.",
        "Undercuts the dig.",
        Tool,
        Who::S,
        Attr::Anticipation,
        true,
    ),
    info(
        "Plant foot drift",
        "Long kicks miss to one side more.",
        "Wide right from 48.",
        Flaw,
        Who::K,
        Attr::LongKickAccuracy,
        false,
    ),
    info("Drop inconsistency", "His punts shank more.", "A 28-yard punt.", Flaw, Who::P, Attr::PuntPower, false),
    info(
        "High snaps in the wet",
        "Snaps go bad in rain and snow.",
        "The holder reaches up in the rain.",
        Flaw,
        Who::Ls,
        Attr::LongSnapping,
        false,
    ),
    info(
        "Pooch and coffin corner",
        "Places punts inside the 20 from midfield.",
        "Out of bounds at the 6.",
        Tool,
        Who::P,
        Attr::PuntPlacement,
        false,
    ),
    info(
        "Onside touch",
        "Wins more onside kicks.",
        "The onside kick hops into his own man's hands.",
        Tool,
        Who::K,
        Attr::KickoffTechnique,
        false,
    ),
    info(
        "Dives at ankles",
        "Misses more tackles in the open field.",
        "Grabs air at the ankles.",
        Flaw,
        Who::Tackler,
        Attr::Tackling,
        false,
    ),
    info(
        "Carries it loose",
        "Holds the ball worse in a crowd.",
        "The ball swings loose on contact.",
        Flaw,
        Who::Carrier,
        Attr::BallSecurity,
        false,
    ),
];

impl Craft {
    pub const ALL: [Craft; CRAFT_COUNT] = {
        let mut out = [Craft::EyesDown; CRAFT_COUNT];
        let mut k = 0;
        while k < CRAFT_COUNT {
            // SAFETY-free: the enum is repr(u8) with variants 0..CRAFT_COUNT.
            out[k] = Craft::from_index_const(k);
            k += 1;
        }
        out
    };

    const fn from_index_const(k: usize) -> Craft {
        use Craft::*;
        const LIST: [Craft; CRAFT_COUNT] = [
            EyesDown,
            BackFoot,
            FirstRead,
            StaresDown,
            LongMotion,
            PumpFake,
            HardCount,
            BackShoulder,
            DucksPickup,
            Upright,
            SlowMesh,
            JumpCut,
            StiffArm,
            RoundsBreaks,
            TipsBreaks,
            BodyCatcher,
            OneRelease,
            ReleasePackage,
            LateHands,
            SuddenStop,
            Oversets,
            CatchesPunch,
            OverToes,
            HighPadBlock,
            HeavyClimb,
            RefitsHands,
            NoPlan,
            HighPadRush,
            ChestBlocker,
            EyesBackfield,
            RealCounter,
            LongArm,
            Spin,
            CatchesBlocks,
            Overruns,
            NoDepth,
            WheelLoss,
            ShockShed,
            AGapRip,
            Peeks,
            OpensHips,
            Lunges,
            NoHands,
            MirrorPress,
            Bait,
            PoorAngles,
            EyesOnQb,
            LateRotation,
            RobberBait,
            PlantDrift,
            DropInconsistent,
            WetSnaps,
            CoffinCorner,
            OnsideTouch,
            AnkleDiver,
            CarriesLoose,
        ];
        LIST[k]
    }

    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(k: usize) -> Option<Craft> {
        (k < CRAFT_COUNT).then(|| Craft::ALL[k])
    }

    pub fn info(self) -> &'static Info {
        &INFO[self.index()]
    }

    pub fn name(self) -> &'static str {
        self.info().name
    }

    pub fn is_flaw(self) -> bool {
        self.info().kind == Kind::Flaw
    }

    pub fn owner(self) -> Attr {
        self.info().owner
    }

    pub fn fits(self, pos: Position) -> bool {
        self.info().who.fits(pos)
    }

    /// The flaws and tools a position can carry.
    pub fn for_position(pos: Position) -> impl Iterator<Item = Craft> {
        Craft::ALL.into_iter().filter(move |c| c.fits(pos))
    }
}

/// One held flaw or tool (five bytes). Severities are 0-255 for 0-1: for a
/// flaw how badly he has it, for a tool how well.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Item {
    /// The craft's index plus one; 0 is an empty slot.
    pub code: u8,
    /// As it shows in games.
    pub sev: u8,
    /// As it shows in practice: a fix shows here first, and games lag.
    pub prac: u8,
    /// Where it was before the work began: under stress the old habit
    /// comes back up to a third of the way (topic 6.3), fading over the
    /// next season.
    pub was: u8,
    /// Weeks it has been worked on (the low seven bits, saturating): the
    /// learning dip reads it. The top bit: worked week by week this year
    /// (your seat's plan), so the year's step leaves it alone.
    pub wk: u8,
}

impl Item {
    pub fn craft(&self) -> Option<Craft> {
        if self.code == 0 { None } else { Craft::from_index(self.code as usize - 1) }
    }
    pub fn sev(&self) -> f64 {
        self.sev as f64 / 255.0
    }
    pub fn prac(&self) -> f64 {
        self.prac as f64 / 255.0
    }
    pub fn was(&self) -> f64 {
        self.was as f64 / 255.0
    }
    /// Weeks it has been worked on.
    pub fn weeks(&self) -> u8 {
        self.wk & 0x7F
    }
    /// Whether it was worked week by week this year.
    pub fn worked(&self) -> bool {
        self.wk & 0x80 != 0
    }
}

fn byte(v: f64) -> u8 {
    clamp((v * 255.0).round(), 0.0, 255.0) as u8
}

pub const SLOTS: usize = 6;

/// Below this a flaw is gone and a tool not yet there (it frees its slot).
pub const GONE: f64 = 0.06;

/// A player's flaws and tools (about 31 bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Crafts {
    pub items: [Item; SLOTS],
    /// Whether they were drawn (an older save draws them once, at its age).
    pub drawn: bool,
    /// The highest level he has played at (1 high school, 2 college, 3
    /// the pros; 0 not yet read): a step up exposes habits the level
    /// below never punished.
    #[serde(default)]
    pub lvl: u8,
}

impl Crafts {
    pub fn iter(&self) -> impl Iterator<Item = (Craft, &Item)> {
        self.items.iter().filter_map(|it| it.craft().map(|c| (c, it)))
    }

    pub fn get(&self, c: Craft) -> Option<&Item> {
        self.items.iter().find(|it| it.code == c as u8 + 1)
    }

    pub fn get_mut(&mut self, c: Craft) -> Option<&mut Item> {
        self.items.iter_mut().find(|it| it.code == c as u8 + 1)
    }

    pub fn has(&self, c: Craft) -> bool {
        self.get(c).is_some()
    }

    /// Whether he carries the craft of this index.
    pub fn has_code(&self, code: u8) -> bool {
        self.items.iter().any(|it| it.code == code + 1)
    }

    /// Severity in games, 0 when he doesn't have it.
    pub fn sev(&self, c: Craft) -> f64 {
        self.get(c).map_or(0.0, |it| it.sev())
    }

    /// The flaws he carries in games (a fixed one kept only for its stress
    /// return doesn't count).
    pub fn flaws(&self) -> usize {
        self.iter().filter(|(c, it)| c.is_flaw() && it.sev() >= GONE).count()
    }

    /// The tools he has in games (one he is still learning doesn't count).
    pub fn tools(&self) -> usize {
        self.iter().filter(|(c, it)| !c.is_flaw() && it.sev() >= GONE).count()
    }

    /// Sets it at a severity (games and practice alike), taking a free
    /// slot or the weakest one's; 0 removes it.
    pub fn set(&mut self, c: Craft, v: f64) {
        if v < GONE {
            self.remove(c);
            return;
        }
        let b = byte(v);
        if let Some(it) = self.get_mut(c) {
            it.sev = b;
            it.prac = b;
            it.was = b;
            return;
        }
        let slot = match self.items.iter().position(|it| it.code == 0) {
            Some(k) => k,
            None => (0..SLOTS).min_by_key(|&k| self.items[k].sev).unwrap_or(0),
        };
        self.items[slot] = Item { code: c as u8 + 1, sev: b, prac: b, was: b, wk: 0 };
    }

    pub fn remove(&mut self, c: Craft) {
        if let Some(it) = self.get_mut(c) {
            *it = Item::default();
        }
    }

    /// Moves a held one by `d` in games and practice (a fix or a habit
    /// growing), keeping where it was for the stress return. Gone below
    /// `GONE`.
    pub fn shift(&mut self, c: Craft, d: f64) {
        if let Some(it) = self.get_mut(c) {
            let v = clamp(it.sev() + d, 0.0, 1.0);
            it.sev = byte(v);
            it.prac = byte(clamp(it.prac() + d, 0.0, 1.0));
            if v < GONE && it.was() < GONE {
                *it = Item::default();
            }
        }
    }

    /// What he wears into a game: each held one's rating points in its
    /// situation, signed (a flaw costs, a tool adds), with the learning dip,
    /// as it shows in games and in practice, and where it was.
    pub fn worn(&self) -> Worn {
        let mut w = Worn::default();
        for (k, it) in self.items.iter().enumerate() {
            let Some(c) = it.craft() else { continue };
            let sign = if c.is_flaw() { -1.0 } else { 1.0 };
            // The learning dip (topic 6.1, sizes item 3): while a fix is new
            // he thinks about it, and the flaw costs up to a quarter more.
            let dip = if c.is_flaw() && (1..=DIP_WEEKS).contains(&it.weeks()) { 1.0 + DIP } else { 1.0 };
            w.code[k] = it.code;
            w.now[k] = (sign * FULL * it.sev() * dip) as f32;
            w.prac[k] = (sign * FULL * it.prac() * dip) as f32;
            w.was[k] = (sign * FULL * it.was()) as f32;
        }
        w
    }
}

/// The learning dip: a quarter more for the first weeks of a fix (sizes,
/// item 3, confirmed: up to a quarter, 2 to 4 weeks; the weeks are
/// proposed, not confirmed).
pub const DIP: f64 = 0.25;
pub const DIP_WEEKS: u8 = 3;

/// A player's flaws and tools as the engine reads them (on `GamePlayer`):
/// for each slot its code, its rating points now (signed) and where it
/// was.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Worn {
    pub code: [u8; SLOTS],
    pub now: [f32; SLOTS],
    #[serde(default)]
    pub prac: [f32; SLOTS],
    pub was: [f32; SLOTS],
}

impl Worn {
    /// On the practice field a fix shows first (topic 6.2): practice snaps
    /// read the practice side.
    pub fn to_practice(&mut self) {
        self.now = self.prac;
    }

    /// Its rating points in its situation under `stress` (0-1): the old
    /// habit returns up to a third of the way (topic 6.3). 0 when he
    /// doesn't carry it.
    #[inline]
    pub fn pts(&self, c: Craft, stress: f64) -> f64 {
        let code = c as u8 + 1;
        for k in 0..SLOTS {
            if self.code[k] == code {
                let (n, w) = (self.now[k] as f64, self.was[k] as f64);
                return n + stress * (w - n) / 3.0;
            }
        }
        0.0
    }

    #[inline]
    pub fn any(&self) -> bool {
        self.code.iter().any(|&c| c != 0)
    }

    /// Sets one at a severity for a test arm (A/B), no stress return.
    pub fn set(&mut self, c: Craft, sev: f64) {
        let code = c as u8 + 1;
        let sign = if c.is_flaw() { -1.0 } else { 1.0 };
        let v = (sign * FULL * clamp(sev, 0.0, 1.0)) as f32;
        let k = self.code.iter().position(|&x| x == code).or_else(|| self.code.iter().position(|&x| x == 0));
        if let Some(k) = k {
            self.code[k] = if sev > 0.0 { code } else { 0 };
            self.now[k] = v;
            self.prac[k] = v;
            self.was[k] = v;
        }
    }

    /// Clears every one (a test arm's "none").
    pub fn clear(&mut self) {
        *self = Worn::default();
    }
}

// --- Where they come from (topic 3) -------------------------------------

/// His own keyed stream for his flaws and tools: nothing else in the world
/// draws differently because of them.
fn stream(p: &Player, salt: u32) -> Rng {
    Rng::new(p.id.wrapping_mul(0x9E37_79B1) ^ (p.age.wrapping_mul(0x85EB_CA6B)) ^ salt)
}

/// How much his body and habits lean him toward a flaw, or toward picking
/// up a tool (topic 3.1): influences, never certainties, 0.7 to 1.4.
fn lean(p: &Player, c: Craft) -> f64 {
    use Craft::*;
    let a = |x: Attr| p.attributes.get(x);
    let pos = p.position;
    let size = crate::body::combine_size(pos);
    let tall = clamp((p.body.height - size.0) / size.1.max(0.5), -2.0, 2.0) / 2.0;
    let heavy = clamp((p.body.weight - size.2) / size.3.max(5.0), -2.0, 2.0) / 2.0;
    let t = |x: crate::tendencies::Tend| (p.tend.get(x) - crate::tendencies::MID) / 9.5;
    // A low rating against his own overall: his weak spot.
    let low = |x: Attr| clamp((p.overall() - a(x)) / 20.0, -1.0, 1.0);
    let v = match c {
        Oversets => 0.6 * low(Attr::Flexibility) + 0.3 * tall,
        Upright => 0.6 * tall + 0.3 * low(Attr::Balance),
        StaresDown => 0.6 * t(crate::tendencies::Tend::Gunslinger) + 0.4 * low(Attr::EyeManipulation),
        FirstRead => 0.7 * low(Attr::FootballIq),
        BackFoot => 0.5 * low(Attr::Composure) + 0.3 * low(Attr::Footwork),
        EyesDown => 0.5 * low(Attr::Composure),
        LongMotion => 0.5 * low(Attr::QuickRelease),
        DucksPickup => 0.5 * low(Attr::Toughness),
        SlowMesh => 0.4 * low(Attr::Acceleration) + 0.3 * t(crate::tendencies::Tend::Patience),
        RoundsBreaks => 0.5 * low(Attr::Agility) + 0.3 * tall,
        TipsBreaks => 0.4 * low(Attr::FootballIq),
        BodyCatcher => 0.6 * -((p.body.hand - 9.5) / 0.5).clamp(-2.0, 2.0) / 2.0,
        OneRelease => 0.4 * low(Attr::HandUse),
        CatchesPunch => 0.5 * low(Attr::HandUse) + 0.3 * low(Attr::Strength),
        OverToes => 0.5 * low(Attr::Balance),
        HighPadBlock | HighPadRush => 0.5 * tall + 0.4 * low(Attr::Flexibility),
        HeavyClimb => 0.4 * low(Attr::Agility) + 0.3 * heavy,
        NoPlan => 0.6 * low(Attr::FootballIq),
        ChestBlocker | CatchesBlocks => 0.6 * low(Attr::HandUse),
        EyesBackfield | EyesOnQb => 0.5 * low(Attr::Discipline) + 0.3 * low(Attr::PlayRecognition),
        Overruns => 0.4 * t(crate::tendencies::Tend::Downhill) + 0.3 * low(Attr::Discipline),
        NoDepth => 0.5 * low(Attr::Anticipation),
        WheelLoss => 0.5 * low(Attr::Speed),
        Peeks => 0.5 * t(crate::tendencies::Tend::JumpRoutes) + 0.4 * low(Attr::Discipline),
        OpensHips => 0.5 * low(Attr::Flexibility) + 0.3 * low(Attr::Agility),
        Lunges => 0.5 * low(Attr::Discipline) + 0.3 * low(Attr::Balance),
        NoHands => 0.5 * low(Attr::BallTracking),
        PoorAngles => 0.5 * low(Attr::PlayRecognition),
        LateRotation => 0.5 * low(Attr::FootballIq),
        AnkleDiver => 0.5 * -heavy + 0.3 * t(crate::tendencies::Tend::BigHit),
        CarriesLoose => 0.6 * -((p.body.hand - 9.5) / 0.5).clamp(-2.0, 2.0) / 2.0,
        // Tools come to the gifted: his strength in what the tool uses.
        PumpFake | HardCount | BackShoulder => -0.5 * low(Attr::FootballIq),
        JumpCut => -0.5 * low(Attr::Agility),
        StiffArm => -0.5 * low(Attr::Strength) + 0.3 * heavy,
        ReleasePackage => -0.5 * low(Attr::HandUse),
        LateHands => -0.5 * low(Attr::BodyControl),
        SuddenStop => -0.5 * low(Attr::Agility),
        RefitsHands => -0.6 * low(Attr::HandUse),
        RealCounter => -0.6 * low(Attr::FootballIq),
        LongArm => -0.5 * low(Attr::Strength) + 0.4 * ((p.body.arm - 34.0) / 1.2).clamp(-1.0, 1.0),
        Spin => -0.5 * low(Attr::Agility),
        ShockShed => -0.6 * low(Attr::HandUse),
        AGapRip => -0.5 * low(Attr::Acceleration),
        MirrorPress => -0.5 * low(Attr::Agility),
        Bait | RobberBait => -0.6 * low(Attr::FootballIq),
        PlantDrift | DropInconsistent | WetSnaps => -0.5 * (p.game.consistency - 10.0) / 10.0,
        CoffinCorner | OnsideTouch => 0.0,
    };
    // His weak spot in what it belongs to leans him toward a flaw (and his
    // strong spot toward a tool).
    let own = if c.is_flaw() { 0.5 * low(c.owner()) } else { -0.5 * low(c.owner()) };
    clamp((v + own).exp2(), 0.7, 1.4)
}

/// The share of his position's flaws a raw 14-year-old carries (topic 3.2:
/// 3 to 5 for a typical freshman, proposed, not confirmed): about three and
/// a half of five, never more than three in four of a short list.
fn raw_share(pos: Position) -> f64 {
    let n = Craft::for_position(pos).filter(|c| c.is_flaw()).count().max(1) as f64;
    (3.6 / n).min(0.72)
}

/// Draws his flaws and tools as a raw 14-year-old (topic 3.1-3.2).
fn draw_raw(p: &mut Player) {
    let mut r = stream(p, 0xC4AF);
    let share = raw_share(p.position);
    let mut out = Crafts { drawn: true, ..Crafts::default() };
    let mut picks: Vec<(Craft, f64)> = Vec::new();
    for c in Craft::for_position(p.position) {
        let l = lean(p, c);
        let u = r.next();
        let sevu = r.next();
        if c.is_flaw() {
            if u < share * l {
                picks.push((c, clamp(0.35 + 0.5 * sevu * l, 0.2, 1.0)));
            }
        } else if u < 0.04 * l {
            picks.push((c, clamp(0.15 + 0.25 * sevu, 0.1, 0.5)));
        }
    }
    picks.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (c, v) in picks.into_iter().take(SLOTS) {
        out.set(c, v);
    }
    p.craft = out;
}

/// The level a player of an age typically plays at, for drawing an older
/// save's flaws through his years.
fn level_at(age: u32) -> Level {
    if age < 18 {
        Level::HS
    } else if age < 22 {
        Level::FBS
    } else {
        Level::NFL
    }
}

/// Draws his flaws and tools if he has none yet: as a raw 14-year-old, then
/// his years since through an average staff (an older save, or a player
/// made older than 14).
pub fn ensure(p: &mut Player) {
    if p.craft.drawn {
        return;
    }
    let age = p.age;
    p.age = 14;
    draw_raw(p);
    for a in 14..age.max(14) {
        p.age = a;
        year(p, 10.0, level_at(a), 0.6, Some(10.0));
    }
    p.age = age;
}

/// The year's change at his checkpoint (topic 3.3-3.5 and 10; every size
/// proposed, not confirmed): coaching fixes his flaws and adds tools, the
/// better the staff, the more he takes coaching and the younger he is; a
/// poor staff lets a habit grow. The staff's points of emphasis (`staff_points`,
/// by its style: `staff` is its mould, 1-20, None for your seat, whose week
/// works them) are worked on the weekly rule (`pace`) over a
/// year's individual time; the rest move slowly. `coaching` is his staff on
/// 1-20, `playing` his share of a starter's snaps (reps teach tools). One
/// your plan worked week by week this year keeps where the weeks put it.
pub fn year(p: &mut Player, coaching: f64, level: Level, playing: f64, staff: Option<f64>) {
    if !p.craft.drawn {
        ensure(p);
        return;
    }
    let mut r = stream(p, 0x7EA2);
    // A better staff fixes more, with less to gain at the top: the square
    // root of the old straight line (a 16 fixes about a third more than an
    // 11, not twice as much).
    let coach = clamp((coaching - 4.0) / 8.0, 0.0, 2.0).sqrt();
    let take = clamp(0.6 + 0.02 * p.personality.coachability, 0.6, 1.0);
    // Young players re-groove fastest; by about 26 a habit is set (topic
    // 6.4, the habits' settle curve).
    let settle = clamp((26.5 - p.age as f64) / 4.5, 0.12, 1.0);
    let lvl = match level {
        Level::HS => 0.75,
        Level::NFL => 1.1,
        _ => 1.0,
    };
    let fix = BACKGROUND * 0.11 * coach * take * settle * lvl;
    let pos = p.position;
    // The staff's own points (topic 10): the same weekly rule as your plan,
    // over a year of individual time at the usual minutes, shared.
    let points = staff.map_or([None, None], |m| staff_points(&p.craft, pos, m));
    let n_points = points.iter().flatten().count().max(1) as f64;
    let teach = Teach {
        teaching: coaching,
        minutes: crate::development::USUAL_FOCUS_MINUTES / n_points,
        buy_in: buy_in(p, 0.5),
    };
    let mut cr = p.craft;
    // A step up a level (topic 3.3): faster, stronger, smarter opponents
    // expose habits the level below never punished (proposed, not
    // confirmed: about a fifth of a raw player's share at each step).
    let rank = match level {
        Level::HS => 1,
        Level::NFL => 3,
        _ => 2,
    };
    if cr.lvl != 0 && rank > cr.lvl {
        let share = raw_share(pos) * EXPOSED;
        for c in Craft::for_position(pos).filter(|c| c.is_flaw()) {
            let l = lean(p, c);
            if !cr.has(c) && r.next() < share * l && cr.items.iter().any(|it| it.code == 0) {
                cr.set(c, clamp(0.2 + 0.3 * r.next() * l, 0.15, 0.6));
            }
        }
    }
    cr.lvl = cr.lvl.max(rank);
    for k in 0..SLOTS {
        let it = cr.items[k];
        let Some(c) = it.craft() else { continue };
        if !c.fits(pos) {
            // A position move leaves the old position's craft behind.
            cr.items[k] = Item::default();
            continue;
        }
        let u = r.next();
        let v = it.sev();
        if it.worked() {
            // Your plan worked it week by week: the weeks moved it, and only
            // the stress return fades.
            cr.items[k].was = byte(v.max(it.prac()) + 0.35 * (it.was() - v.max(it.prac())).max(0.0));
            cr.items[k].wk &= 0x7F;
            if c.is_flaw() && v < GONE && it.prac() < GONE && it.was() < GONE {
                cr.items[k] = Item::default();
            }
            continue;
        }
        let nv = if c.is_flaw() {
            // A poor staff lets a habit grow; otherwise it shrinks, unevenly.
            // Some habits run deeper than others: how easily this one goes
            // for him is his own and fixed (`ease`).
            let grow = if coaching < 7.0 { 0.06 * (7.0 - coaching) / 6.0 } else { 0.0 };
            let worked =
                if points.contains(&Some(c)) { STAFF_WEEKS * lvl * pace(p, c, &teach) * (0.6 + 0.8 * u) } else { 0.0 };
            v - fix * ease(p, c) * (0.4 + 1.2 * u) * (0.6 + 0.4 * v.max(0.3) / 0.3).min(2.0) / 1.6 + grow - worked
        } else {
            // A tool he has grows with reps and teaching.
            v + (0.12 + 0.18 * playing) * coach.min(1.5) * take * (0.5 + u) * settle.max(0.4)
        };
        let nv = clamp(nv, 0.0, 1.0);
        let b = byte(nv);
        cr.items[k].sev = b;
        cr.items[k].prac = b;
        // The stress return fades over a season.
        cr.items[k].was = byte(nv + 0.35 * (it.was() - nv).max(0.0));
        cr.items[k].wk = 0;
        if nv < GONE && c.is_flaw() {
            cr.items[k] = Item::default();
        }
    }
    // A new tool: good teaching and reps (proposed, not confirmed: about
    // about three in ten a year for a starter under a good staff, fewer as he ages).
    // Tools come to the gifted (topic 3.1): his mean lean toward the ones
    // left to learn, squared (0.5 to 2).
    let held_tools = cr.tools();
    let cands: Vec<Craft> = Craft::for_position(pos).filter(|c| !c.is_flaw() && !cr.has(*c)).collect();
    let w: Vec<f64> = cands.iter().map(|&c| lean(p, c)).collect();
    let gift = if w.is_empty() { 1.0 } else { (w.iter().sum::<f64>() / w.len() as f64).powi(2) };
    let p_tool =
        0.3 * gift * coach.min(1.5) * (0.4 + 0.6 * playing) * take * settle.max(0.3) / (1.0 + held_tools as f64);
    if r.next() < p_tool && !cands.is_empty() {
        let tot: f64 = w.iter().sum();
        let mut x = r.next() * tot;
        let mut pick = cands[0];
        for (c, wc) in cands.iter().zip(&w) {
            if x < *wc {
                pick = *c;
                break;
            }
            x -= wc;
        }
        cr.set(pick, 0.2 + 0.2 * r.next());
    }
    // A bad habit under a poor staff: a new flaw now and then.
    if coaching < 7.0 && r.next() < 0.08 * (7.0 - coaching) / 6.0 {
        let cands: Vec<Craft> = Craft::for_position(pos).filter(|c| c.is_flaw() && !cr.has(*c)).collect();
        if !cands.is_empty() {
            let pick = cands[(r.next() * cands.len() as f64) as usize % cands.len()];
            cr.set(pick, 0.25 + 0.25 * r.next());
        }
    }
    p.craft = cr;
}

/// A player moved to a new position picks up the raw flaws of the new
/// spot's technique he never learned (a quarterback who becomes a receiver
/// rounds his breaks), drawn as a raw player's, softened by his age.
pub fn moved(p: &mut Player) {
    if !p.craft.drawn {
        return;
    }
    let mut r = stream(p, 0x3070);
    let pos = p.position;
    let share = raw_share(pos) * 0.7;
    let mut cr = p.craft;
    for k in 0..SLOTS {
        if cr.items[k].craft().is_some_and(|c| !c.fits(pos)) {
            cr.items[k] = Item::default();
        }
    }
    for c in Craft::for_position(pos).filter(|c| c.is_flaw()) {
        let l = lean(p, c);
        if !cr.has(c) && r.next() < share * l {
            cr.set(c, clamp(0.3 + 0.4 * r.next() * l, 0.2, 0.9));
        }
    }
    p.craft = cr;
}

// --- Working on them (topics 5-8 and 10) --------------------------------

/// How much of the year's fixing happens away from a point of emphasis:
/// the habits that shrink with reps and plain coaching (proposed, not
/// confirmed; tuned with `craft_check` so the counts held).
pub const BACKGROUND: f64 = 0.75;

/// Weeks of individual time in a year at the usual minutes, for a staff's
/// own points (proposed, not confirmed: spring, camp and the season's
/// individual periods, less the weeks off).
pub const STAFF_WEEKS: f64 = 12.0;

/// The share of a raw player's flaws a step up a level exposes.
pub const EXPOSED: f64 = 0.4;

/// A week's work on a full flaw at the ideal pace: a willing, coachable
/// (16) 20-year-old with a strong teacher (15) at the usual 30 minutes
/// takes a typical flaw (about half) to hold in games in 12 to 16 weeks
/// (sizes item 2, confirmed); the factors below are proposed, not
/// confirmed.
pub const PACE: f64 = 0.04;

/// Below this in games a flaw holds as fixed (and a tool above
/// `TOOL_HOLD` holds as his).
pub const HOLD: f64 = 0.08;
pub const TOOL_HOLD: f64 = 0.4;

/// How easily a flaw goes for him: his own and fixed, 0.4 to 1.6 (some
/// habits run deeper than others).
pub fn ease(p: &Player, c: Craft) -> f64 {
    0.4 + 1.2 * Rng::new(p.id.wrapping_mul(0x2C1B_3C6D) ^ ((c as u32 + 1) << 8)).next()
}

/// A week of teaching on one point: the teacher's teaching in that group
/// (1-20), the minutes he spends on it, and how much he buys in (0-1).
#[derive(Clone, Copy, Debug)]
pub struct Teach {
    pub teaching: f64,
    pub minutes: f64,
    pub buy_in: f64,
}

/// What a week of work moves a flaw or tool, in severity (topic 6.4: the
/// minutes, the teacher, his coachability and, for a film-room one, his
/// football IQ, and his age; proposed, not confirmed). Steep in the
/// teacher and the player: an average teacher (11) and player (10) take
/// about a year over a typical flaw, a poor teacher (8) two.
pub fn pace(p: &Player, c: Craft, t: &Teach) -> f64 {
    let teach = clamp((t.teaching - 4.0) / 11.0, 0.1, 1.1).powf(1.5);
    let take = clamp((p.personality.coachability - 2.0) / 14.0, 0.2, 1.15).powf(1.2);
    // By about 26 a habit is set: a 30-year-old takes about twice as long
    // (sizes item 2).
    let age = clamp(1.0 - 0.0625 * (p.age as f64 - 22.0).max(0.0), 0.5, 1.0);
    let mins = clamp(t.minutes / crate::development::USUAL_FOCUS_MINUTES, 0.0, 2.0);
    let mind =
        if c.info().mental { clamp(0.5 + (p.attributes.get(Attr::FootballIq) - 40.0) / 50.0, 0.5, 1.3) } else { 1.0 };
    let e = if c.is_flaw() { ease(p, c) } else { 1.0 };
    PACE * teach * take * age * mins * mind * e * clamp(t.buy_in, 0.0, 1.0)
}

/// How much of the work sticks (topic 8.1): his coachability, his trust in
/// you and his coach (0-1), and a veteran's ego (proposed, not confirmed).
pub fn buy_in(p: &Player, trust: f64) -> f64 {
    let pe = &p.personality;
    let vet = clamp((p.age as f64 - 24.0) / 6.0, 0.0, 1.0);
    let ego = (pe.traits.ego as f64 - 10.0).max(0.0);
    clamp(0.55 + 0.02 * (pe.coachability - 10.0) + 0.4 * (trust - 0.5) - 0.015 * ego * vet, 0.2, 1.0)
}

/// The chance a veteran star turns down work on a flaw he doesn't believe
/// he has (topic 8.2): an ego past 12, past 25, the less he trusts you
/// the likelier (proposed, not confirmed: at most about one in two).
pub fn refuse_chance(p: &Player, trust: f64, star: bool) -> f64 {
    let ego = (p.personality.traits.ego as f64 - 12.0).max(0.0) / 8.0;
    let vet = clamp((p.age as f64 - 25.0) / 6.0, 0.0, 1.0);
    let star = if star { 1.0 } else { 0.4 };
    clamp(0.6 * ego * vet * star * (1.0 - trust), 0.0, 0.55)
}

/// The two flaws a staff works first (topic 10): an adapting staff (`mould`
/// low) his biggest, a moulding one (high) the ones its system leans on at
/// his spot (the rating's weight there).
pub fn staff_points(cr: &Crafts, pos: Position, mould: f64) -> [Option<Craft>; 2] {
    let m = clamp((mould - 1.0) / 19.0, 0.0, 1.0);
    let top = Attr::ALL.iter().map(|&a| crate::positions::weight(pos, a)).fold(0.0, f64::max).max(1e-9);
    let mut v: Vec<(Craft, f64)> = cr
        .iter()
        .filter(|(c, it)| c.is_flaw() && it.sev() >= GONE)
        .map(|(c, it)| {
            let w = crate::positions::weight(pos, c.owner()) / top;
            (c, it.sev() * (1.0 - m + m * (0.3 + 0.7 * w)))
        })
        .collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1).then((a.0 as u8).cmp(&(b.0 as u8))));
    [v.first().map(|x| x.0), v.get(1).map(|x| x.0)]
}

/// One point of emphasis (topic 5): a flaw to fix, a tool to add, or a
/// plain rating when the staff has named nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Point {
    Rating(Attr),
    Craft(Craft),
}

impl Point {
    /// Stored as one byte: a rating's index, or 100 plus the craft's.
    pub fn code(self) -> u8 {
        match self {
            Point::Rating(a) => a.index() as u8,
            Point::Craft(c) => 100 + c as u8,
        }
    }

    pub fn from_code(v: u8) -> Option<Point> {
        if v >= 100 {
            Craft::from_index(v as usize - 100).map(Point::Craft)
        } else {
            Attr::ALL.get(v as usize).map(|&a| Point::Rating(a))
        }
    }

    /// The rating it grows (topic 5b): a flaw's or tool's owner.
    pub fn rating(self) -> Attr {
        match self {
            Point::Rating(a) => a,
            Point::Craft(c) => c.owner(),
        }
    }

    pub fn craft(self) -> Option<Craft> {
        match self {
            Point::Craft(c) => Some(c),
            Point::Rating(_) => None,
        }
    }
}

/// How far a fix has landed (topic 6): learning it in drills, working in
/// practice, holding in games.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Learning,
    Practice,
    Games,
}

impl Stage {
    /// In plain words (topic 9b.3).
    pub fn words(self) -> &'static str {
        match self {
            Stage::Learning => "Learning",
            Stage::Practice => "Works in practice",
            Stage::Games => "Fixed in games",
        }
    }
}

/// Where one held flaw or tool stands.
pub fn stage(c: Craft, it: &Item) -> Stage {
    if c.is_flaw() {
        if it.sev() < HOLD {
            Stage::Games
        } else if it.weeks() > DIP_WEEKS && (it.prac() < HOLD || it.prac() < 0.67 * it.was()) {
            Stage::Practice
        } else {
            Stage::Learning
        }
    } else if it.sev() >= TOOL_HOLD {
        Stage::Games
    } else if it.prac() >= TOOL_HOLD {
        Stage::Practice
    } else {
        Stage::Learning
    }
}

/// A week of work on one point (topic 6): practice moves first by the
/// week's pace and games follow a few weeks behind; the first weeks carry
/// the learning dip (`Crafts::worn`). A tool he doesn't have yet starts
/// from nothing; a flaw he doesn't have (the staff named the wrong one)
/// moves nothing. Returns the stage before and after.
pub fn work_week(p: &mut Player, c: Craft, t: &Teach) -> Option<(Stage, Stage)> {
    if !c.fits(p.position) {
        return None;
    }
    let r = pace(p, c, t);
    let cr = &mut p.craft;
    if !cr.has(c) {
        if c.is_flaw() {
            return None;
        }
        // A slot for a new tool: a free one, else the weakest tool's.
        let slot = cr.items.iter().position(|it| it.code == 0).or_else(|| {
            (0..SLOTS).filter(|&k| cr.items[k].craft().is_some_and(|x| !x.is_flaw())).min_by_key(|&k| cr.items[k].sev)
        })?;
        cr.items[slot] = Item { code: c as u8 + 1, ..Item::default() };
    }
    let it = cr.get_mut(c)?;
    let before = stage(c, it);
    let first = it.weeks() == 0;
    if first && c.is_flaw() {
        it.was = it.was.max(it.sev);
    }
    it.wk = (it.weeks() + 1).min(0x7F) | 0x80;
    let lag = if first { 0.0 } else { 0.3 };
    if c.is_flaw() {
        let pr = (it.prac() - r).max(0.0);
        it.prac = byte(pr);
        it.sev = byte(it.sev() - lag * (it.sev() - pr).max(0.0));
    } else {
        let pr = (it.prac() + r).min(1.0);
        it.prac = byte(pr);
        it.sev = byte(it.sev() + lag * (pr - it.sev()).max(0.0));
    }
    let after = stage(c, it);
    Some((before, after))
}
