//! Who plays: game-day players, the coaching staff, schemes, the depth chart,
//! two-way players (topic 5.1), fatigue and rotations (topic 6).

use crate::attributes::{Attr, Attributes};
use crate::body::body_now;
use crate::player::{GameTraits, Level, Personality, Player};
use crate::positions::{POSITION_COUNT, Position, overall_for};
use crate::rng::clamp;
use crate::spots::{self, SPOT_COUNT, Spot, fill_in, spot_rating};

use super::calling::Film;
use super::playbook::{Backfield, FAMILY_COUNT, Personnel, PlayKind, Set};
use super::rules::{TRAIT_STEP, trait_norm};

/// A player as the match engine sees him on game day.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GamePlayer {
    pub id: u32,
    /// Listed position.
    pub position: Position,
    pub age: u32,
    pub attrs: Attributes,
    /// Inches and pounds today.
    pub height: f64,
    pub weight: f64,
    pub arm: f64,
    /// Throwing hand and kicking foot: a lefty quarterback's blind side is
    /// his right, and a lefty punter's spin is harder to field.
    pub left_handed: bool,
    pub left_footed: bool,
    pub traits: GameTraits,
    pub personality: Personality,
    pub injury_proneness: f64,
    /// How well he knows this team's playbook, 0-1 (topic 3).
    pub knowledge: f64,
    /// What he knows of each concept family, 0-1 (plays and tactics topic
    /// 4): a third is knowing his job, two thirds understanding why, all of
    /// it mastery.
    pub know: [f32; FAMILY_COUNT],
    /// Seasons of real game experience.
    pub experience: f64,
    /// How much his coach lets him freelance, 0-1 (topic 3.1). Moves with
    /// what happens when he does.
    pub trust: f64,
    /// His listed spot, the spots he has been listed at (bits) and his
    /// rating at every spot (fix list topic 3).
    pub spot: Spot,
    pub spots_played: u32,
    pub spot_r: [f64; SPOT_COUNT],
    /// His tendencies (`docs/design/tendencies.md`) and his coach's
    /// instructions to him.
    pub tend: crate::tendencies::Tendencies,
    pub orders: crate::tendencies::Orders,
    /// Chemistry with his unit, 0-1 (X5): a receiver's with his starting
    /// quarterback, a quarterback's with his top targets, a defensive
    /// back's with the starting secondary. 0.45 is typical and changes
    /// nothing (`world::chemistry`).
    #[serde(default = "typical_chem")]
    pub chem: f32,
    /// His Dynamic Links today (`links.rs`): each a modest edge in its own
    /// situations.
    #[serde(default)]
    pub links: crate::links::Links,
    /// His practice grades as his staff read them, in rating points a
    /// staff adds when it orders the depth chart (section 6b topic 8).
    #[serde(default)]
    pub impression: f64,
    /// His week's prep (update 2, lane 4, TR38): his mental errors on game
    /// day against a prepared player's (busts, wrong fits, late reads; 1
    /// changes nothing).
    #[serde(default = "no_prep")]
    pub prep: f32,
    /// His mood today, -1 to 1 (people topic 3): a talk lands on how he
    /// came in (dialogue D4). 0 changes nothing.
    #[serde(default)]
    pub mood: f32,
    /// How much he believes his head coach, 0-1 (your players' trust in
    /// you): how much of a talk reaches him (dialogue D4).
    #[serde(default = "half")]
    pub belief: f32,
}

fn half() -> f32 {
    0.5
}

fn typical_chem() -> f32 {
    TYPICAL_CHEM
}

fn no_prep() -> f32 {
    1.0
}

/// A player's game-day prep factor from his latest week's word.
fn prep_of(p: &Player) -> f32 {
    crate::training::prep::game_factor(crate::training::prep::latest(&p.train)) as f32
}

/// The chemistry where nothing changes.
pub const TYPICAL_CHEM: f32 = 0.45;

impl GamePlayer {
    /// `knowledge` is his grasp of this team's playbook (0-1) and
    /// `experience` his seasons of real games; the caller knows both.
    pub fn from_player(p: &Player, knowledge: f64, experience: f64) -> GamePlayer {
        let b = body_now(p);
        GamePlayer {
            id: p.id,
            position: p.position,
            age: p.age,
            attrs: p.attributes,
            height: b.height,
            weight: b.weight,
            arm: b.arm,
            left_handed: b.left_handed,
            left_footed: b.left_footed,
            traits: p.game,
            personality: p.personality,
            injury_proneness: p.injury_proneness,
            knowledge: clamp(knowledge, 0.0, 1.0),
            know: [clamp(knowledge, 0.0, 1.0) as f32; FAMILY_COUNT],
            experience,
            trust: 0.5,
            spot: p.spot,
            spots_played: p.spots_played | p.spot.bit(),
            spot_r: Spot::ALL.map(|s| spot_rating(s, &p.attributes, b.weight)),
            tend: p.tend,
            orders: Default::default(),
            chem: TYPICAL_CHEM,
            links: crate::links::measure(p.position, &b, &p.attributes, p.game.recovery),
            impression: crate::training::impression(&p.train),
            prep: prep_of(p),
            mood: 0.0,
            belief: 0.5,
        }
    }

    /// The week's growth reaches a kept game roster (section 6b): his
    /// ratings, body and practice impression as they stand now.
    pub fn refresh(&mut self, p: &Player) {
        let b = body_now(p);
        self.attrs = p.attributes;
        self.height = b.height;
        self.weight = b.weight;
        self.arm = b.arm;
        self.spot_r = Spot::ALL.map(|s| spot_rating(s, &p.attributes, b.weight));
        self.tend = p.tend;
        self.impression = crate::training::impression(&p.train);
        self.prep = prep_of(p);
        self.links = crate::links::measure(p.position, &b, &p.attributes, p.game.recovery);
        if self.position != p.position {
            self.position = p.position;
            self.spot = p.spot;
            self.spots_played |= p.spot.bit();
        }
    }

    /// Rating points he gives up at a position he isn't listed at: the
    /// least he'd lose at one of its spots (3.3). Nothing at a spot he has
    /// played.
    pub fn cross_cost(&self, pos: Position) -> f64 {
        spots::spots_of(pos)
            .iter()
            .filter(|&&s| s != Spot::FB)
            .map(|&s| if self.spots_played & s.bit() != 0 { 0.0 } else { fill_in(s, self.spot) })
            .fold(f64::INFINITY, f64::min)
    }

    #[inline]
    pub fn a(&self, a: Attr) -> f64 {
        self.attrs.get(a)
    }

    pub fn overall_at(&self, pos: Position) -> f64 {
        overall_for(pos, &self.attrs)
    }

    /// An attribute on the 1-20 scale the engine's hidden traits used
    /// (five rating points a step).
    #[inline]
    pub fn on20(&self, a: Attr) -> f64 {
        clamp(self.a(a) / 5.0, 1.0, 20.0)
    }

    /// His real 40 time from speed, burst and weight (same fit as the combine).
    pub fn forty(&self) -> f64 {
        5.56 - 0.0124 * (0.75 * self.a(Attr::Speed) + 0.25 * self.a(Attr::Acceleration))
            + crate::body::forty_weight(self.weight)
    }

    /// Top speed, yards per second. A 4.35 forty gives about 10.7 (21.9 mph),
    /// a 5.2 lineman about 9, in line with Next Gen Stats tracking. With the
    /// sprint build-up below, a player runs his own forty in the sim.
    pub fn top_speed(&self) -> f64 {
        40.0 / (self.forty() - self.sprint_tau() + 0.06).max(3.4)
    }

    /// How fast he builds to top speed: the time constant of his sprint, in
    /// seconds. Fit so the sim gives real 10-yard splits (about 0.345 of the
    /// forty: 1.5 for a 4.35 runner, 1.8 for a 5.2 lineman).
    pub fn sprint_tau(&self) -> f64 {
        clamp(0.78 + 0.003 * (60.0 - self.a(Attr::Acceleration)) + 0.0006 * (self.weight - 220.0), 0.55, 1.0)
    }

    /// Acceleration, yards per second squared: his grip for braking and
    /// cutting. The burst from a standing start is `sprint_tau`.
    pub fn accel(&self) -> f64 {
        // Fit with `drills` so the sim's 5-10-5 shuttle and 3-cone move per
        // point as the combine model does (`body::combine`), the way the 40
        // was fit (dynasty item 1; was 0.035 a point, which made a quick
        // player 3-8 times quicker in the sim than at the combine). Below the
        // pro range (most high school players) the gap narrows (proposed,
        // not confirmed).
        let ac = self.a(Attr::Acceleration);
        let ac = if ac >= 60.0 { 4.65 + 0.008 * (ac - 60.0) } else { 4.65 + 0.0046 * (ac - 60.0) };
        clamp(ac - 0.006 * (self.weight - 220.0), 2.2, 7.5)
    }

    /// How hard he can cut, as a share of his acceleration spent sideways.
    pub fn cut(&self) -> f64 {
        // As with acceleration, the gap narrows below the pro range, so a
        // high school athlete still cuts like an athlete.
        let ag = self.a(Attr::Agility);
        // Fit with `drills` as above (was 0.011 a point).
        let ag = if ag >= 60.0 { 0.99 + 0.004 * (ag - 60.0) } else { 0.99 + 0.0022 * (ag - 60.0) };
        clamp(ag - 0.002 * (self.weight - 220.0), 0.4, 1.7)
    }

    /// A trait-like rating (discipline, toughness, motor, composure,
    /// concentration) on the 1-20 scale, read against his level: 10 is the
    /// level's typical player, as the old hidden traits were.
    #[inline]
    pub fn trait20(&self, a: Attr, level: Level) -> f64 {
        clamp(10.0 + (self.a(a) - trait_norm(level, a)) / TRAIT_STEP, 1.0, 20.0)
    }

    /// Composure, 1-20, against his level.
    pub fn composure(&self, level: Level) -> f64 {
        self.trait20(Attr::Composure, level)
    }

    /// His tendency at a moment outside a tracked play (a kick decision):
    /// his own number, slipping back toward a coached-down habit under
    /// stress, plus the game situation on gambles (as `Sim::tend`).
    /// Instructions are checked where they apply.
    pub fn tend_now(&self, t: crate::tendencies::Tend, gamble: f64, stakes: f64, energy: f64, level: Level) -> f64 {
        let base = self.tend.get(t);
        let was = self.tend.was[t.index()] as f64;
        let stress = clamp(stakes + 0.8 * (1.0 - energy) + 0.05 * (10.0 - self.composure(level)), 0.0, 1.0);
        let g = if t.gamble() { gamble } else { 0.0 };
        clamp(base + stress * (was - base) / 3.0 + g, 1.0, 20.0)
    }

    /// Does he follow his orders on this play? A draw only when he has one.
    pub fn follows(&self, strictness: f64, level: Level, rng: &mut crate::rng::Rng) -> bool {
        rng.chance(crate::tendencies::obey_chance(
            self.trait20(Attr::Discipline, level),
            self.personality.coachability,
            self.trust,
            strictness,
        ))
    }
}

/// Offensive systems (topic 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OffScheme {
    WestCoast,
    AirRaid,
    /// Shanahan-style wide zone with play action and boots.
    WideZone,
    /// Pro-style power and gap runs.
    PowerGap,
    /// Spread with zone read.
    SpreadOption,
    /// High school wing-T: misdirection runs, little passing.
    WingT,
}

impl OffScheme {
    pub const ALL: [OffScheme; 6] = [
        OffScheme::WestCoast,
        OffScheme::AirRaid,
        OffScheme::WideZone,
        OffScheme::PowerGap,
        OffScheme::SpreadOption,
        OffScheme::WingT,
    ];
    pub fn name(self) -> &'static str {
        ["West Coast", "Air Raid", "wide zone", "power gap", "spread option", "wing-T"][self as usize]
    }
    /// How much more (or less) than an average team this system passes.
    pub fn pass_lean(self) -> f64 {
        [0.02, 0.09, -0.03, -0.06, -0.02, -0.25][self as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Front {
    FourThree,
    ThreeFour,
}

/// Coverage family a defensive coordinator leans on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Shell {
    /// Single-high Cover 3 (Seattle).
    Cover3,
    /// Two-high quarters.
    Quarters,
    /// Man and pressure (Cover 1, Cover 0).
    ManPressure,
    /// Tampa 2 and Cover 2.
    Cover2,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Caller {
    /// Play-calling quality, 1-20 (hidden ability): the mean of his skills.
    pub skill: f64,
    /// How fast and how well he adjusts during a game, 1-20.
    pub adaptation: f64,
    /// 1-20: deep shots, blitzes, fourth downs. A style, not a quality.
    pub aggressiveness: f64,
    /// Plays and tactics topic 6.2, 1-20 each: reading what the other side
    /// shows and what beats it; situational knowledge (down, distance,
    /// field zone); creativity (calls that fit his players, breaking his
    /// own tendencies).
    pub reading: f64,
    pub situational: f64,
    pub creativity: f64,
    /// Under pressure (behind late, a job on the line), -1 tightens up, +1
    /// gambles (topic 6.5), from his personality.
    pub nerve: f64,
}

impl Caller {
    pub fn average() -> Caller {
        Caller::new(10.0, 10.0, 10.0)
    }
    /// A caller whose skills all sit at his overall quality.
    pub fn new(skill: f64, adaptation: f64, aggressiveness: f64) -> Caller {
        Caller { skill, adaptation, aggressiveness, reading: skill, situational: skill, creativity: skill, nerve: 0.0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Staff {
    pub offense: Caller,
    pub defense: Caller,
    /// Head coach clock and fourth-down judgment, 1-20 (topic 4.2).
    pub game_management: f64,
    /// Head coach strictness, 1-20 (topic 3.1). Visible: coaches build a
    /// reputation for it.
    pub strictness: f64,
    pub off_scheme: OffScheme,
    pub front: Front,
    pub shell: Shell,
    /// How many plays the team carries: high school small, NFL big (topic 3).
    pub playbook: f64,
    /// Head coach sportsmanship, 1-20 (hidden): how soon he takes his
    /// starters out of a decided game (dynasty item 3).
    pub sportsmanship: f64,
    /// The defensive coordinator's personnel habit (S8): how often he meets
    /// three receivers with nickel instead of staying in base. Real NFL
    /// teams answered 11 personnel with nickel 88% of the time.
    pub nickel: f64,
    /// The season: what the era plays (RPOs, motion, the triple option).
    pub year: i32,
    /// The staff's film study, 1-20 (topic 5.1): coordinators plus quality
    /// control. How much of the opponent's tendencies it finds.
    pub film_study: f64,
    /// The install (calendar T6.2, plays and tactics topic 4.6): the share
    /// of the playbook in a normal week's game plan, 0.35-1. A bigger plan
    /// carries more calls and more mental errors for players who can't
    /// absorb it.
    pub install: f64,
    /// The head coach's style (tendencies): what he wants from his players'
    /// habits and the instructions he gives.
    pub style: crate::tendencies::Style,
    /// The head coach's guidelines (update 2, G1-G2): how he wants the team
    /// to play, each with a firmness. They lean the play callers.
    #[serde(default)]
    pub guide: super::guide::Guidelines,
    /// The head coach's motivation, 1-20 (dialogue D4): how his talks carry
    /// and how well he reads his room.
    #[serde(default = "ten")]
    pub motivation: f64,
    /// The head coach's temper, 1-20: a hot one leans to the fiery talk.
    #[serde(default = "ten")]
    pub temper: f64,
}

fn ten() -> f64 {
    10.0
}

impl Staff {
    pub fn average(off_scheme: OffScheme) -> Staff {
        Staff {
            offense: Caller::average(),
            defense: Caller::average(),
            game_management: 10.0,
            strictness: 10.0,
            off_scheme,
            front: Front::FourThree,
            shell: Shell::Cover3,
            playbook: 1.0,
            sportsmanship: 10.0,
            install: INSTALL,
            style: Default::default(),
            nickel: 0.88,
            year: 2026,
            film_study: 10.0,
            guide: Default::default(),
            motivation: 10.0,
            temper: 10.0,
        }
    }
}

/// A team for one game.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GameTeam {
    pub name: String,
    pub players: Vec<GamePlayer>,
    pub staff: Staff,
    /// What the team has shown this season (topic 5.1): the other staff
    /// studies it.
    pub film: Film,
    /// How long the offensive line has played together, 0-1 (topic 4.9):
    /// cleaner calls, stunts passed off.
    pub line_chem: f64,
    /// How new each concept family still is to defenses at this level, 0-1
    /// (topic 9): a fresh idea is read a beat late.
    pub novelty: [f64; FAMILY_COUNT],
    /// This week's preparation time against a normal week (calendar T6.2):
    /// a short week (a Thursday game after a Saturday) about 0.8, after a
    /// bye about 1.1.
    pub week: f64,
    /// Your own depth chart (section 8, topic 6): at each position you
    /// ordered, the player ids you listed, best first. Empty for every AI
    /// team, whose staff orders it.
    #[serde(default)]
    pub order: Vec<(Position, Vec<u32>)>,
    /// Your week of practice (section 6b): kickoff energy, packages and
    /// the call sheet. Empty for AI teams.
    #[serde(default)]
    pub prep: Prep,
    /// The day as the world sees it for this team (dialogue D4): the
    /// stakes, the rival, the paper, the week, the captains. The default is
    /// a plain game.
    #[serde(default)]
    pub day: super::talk::Day,
}

/// A call sheet play: the formation's personnel, backfield and set, and the play.
pub type SheetCall = (Personnel, Backfield, Set, PlayKind);

/// A defensive call sheet play (update 2, G8): the coverage and the
/// pressure; the front follows the offense's set as the coordinator reads it.
pub type DefSheetCall = (super::playbook::Coverage, super::playbook::Pressure);

/// What your practice week sends into the game (section 6b): kickoff
/// energy from the late week's load, your packages (board, player ids in
/// slot order), and the call sheet (situation, plays) with how closely
/// the coordinator sticks to it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Prep {
    pub fresh: f64,
    pub packages: Vec<(u8, Vec<u32>)>,
    pub sheet: Vec<(u8, Vec<SheetCall>)>,
    pub stick: f64,
    /// How much a road crowd gets to the offense against an average
    /// staff's week (TR28: the speakers at practice; 1 changes nothing).
    #[serde(default = "one")]
    pub noise: f64,
    /// Pre-snap fouls against an average staff's week (TR29: the officials'
    /// rules clinic; 1 changes nothing).
    #[serde(default = "one")]
    pub rules: f64,
    /// The defensive call sheet by situation, first choice first (update
    /// 2, G8), and how closely the defensive coordinator sticks to it.
    #[serde(default)]
    pub dsheet: Vec<(u8, Vec<DefSheetCall>)>,
    #[serde(default)]
    pub dstick: f64,
    /// The bad-weather version of both sheets (G13): called in rain, snow
    /// or a strong wind.
    #[serde(default)]
    pub wet: Vec<(u8, Vec<SheetCall>)>,
    #[serde(default)]
    pub dwet: Vec<(u8, Vec<DefSheetCall>)>,
    /// New call words this week (G5), offense then defense: the extra
    /// chance a snap that a player busts while the words are new.
    #[serde(default)]
    pub learn: [f64; 2],
    /// The share of our offensive and defensive calls this week's opponent
    /// can read from people who left with our call words (G5).
    #[serde(default)]
    pub leak: [f64; 2],
    /// Snap limits this week (G17): player id and his most snaps.
    #[serde(default)]
    pub limits: Vec<(u32, u16)>,
    /// Instructions given for this opponent (G19), kept so a refresh takes
    /// them back before giving the new ones.
    #[serde(default)]
    pub orders: Vec<(u32, crate::tendencies::Instruction)>,
    /// Our answer to each of their personnel groups this week (G16): 11,
    /// 12, 10 and 21 personnel.
    #[serde(default)]
    pub answers: [super::calling::Answer; 4],
    /// Special teams this week (G18).
    #[serde(default)]
    pub teams: Teams,
    /// The quarterback's checks this week (G19), bits of `CHECKS`: set
    /// means the check is switched from the staff's usual.
    #[serde(default)]
    pub checks: u8,
    /// The "if they do this, we go to that" lines armed this week (G20),
    /// by kind (`IFS`).
    #[serde(default)]
    pub ifs: Vec<u8>,
    /// Package depth (G15): per board, the second and third man for each
    /// slot in slot order (0 none).
    #[serde(default)]
    pub backups: Vec<(u8, Vec<u32>)>,
    /// The defensive line rotation (G15): player id and his role (1 the
    /// whole series, 2 every other series, 3 rush downs only).
    #[serde(default)]
    pub rotation: Vec<(u32, u8)>,
    /// Players kept to base downs this week (G14): off the field on third
    /// down, passing downs and the two-minute drill.
    #[serde(default)]
    pub base_only: Vec<u32>,
    /// Grasp added this week by an extra film session or walkthrough
    /// (G14): player id and the knowledge added (taken back on a refresh).
    #[serde(default)]
    pub boosts: Vec<(u32, f32)>,
    /// The staff's own install before this week's prep weighs on it, kept
    /// so a refresh sets the install again instead of compounding it.
    #[serde(default)]
    pub install_base: Option<f64>,
    /// The know each boost really added, by family (taken back exactly).
    #[serde(default)]
    pub boost_added: Vec<(u32, Vec<f32>)>,
    /// The quarterback's checks and the "if" lines were set by your plan
    /// (`checks`, `ifs`). Otherwise the staff reads them from the
    /// opponent's film at kickoff, the same way for every team.
    #[serde(default)]
    pub planned: bool,
}

/// The quarterback's checks at the line this week (G19), in the order of
/// `Prep::checks`' bits. The first two are on unless switched off, the
/// last two off unless switched on.
pub const CHECKS: [&str; 4] = [
    "Out of a run into a loaded box",
    "Out of a throw, to the run, against a light box with two deep",
    "Hot throw against Cover 0",
    "To the run against two high on early downs",
];

/// The staff's own checks for an opponent (G19), as bits of `CHECKS`: the
/// hot throw when their film shows Cover 0 on one snap in twelve or more,
/// the run against two high when half their coverages or more are two
/// deep. Every staff reads its opponent this way.
pub fn staff_checks(film: &Film) -> u8 {
    use super::playbook::Coverage;
    let all: f32 = film.cov.iter().sum::<f32>().max(1.0);
    let zero = film.cov[Coverage::Cover0 as usize] / all;
    let two: f32 = Coverage::ALL.iter().filter(|c| c.high() == 2).map(|c| film.cov[*c as usize]).sum::<f32>() / all;
    (if zero >= 0.08 { 4 } else { 0 }) | (if two >= 0.5 { 8 } else { 0 })
}

/// The staff's own "if they do this" lines for an opponent (G20): the ones
/// their film says could come up.
pub fn staff_ifs(film: &Film) -> Vec<u8> {
    let snaps = film.def_snaps.max(1.0);
    let mut v = vec![2, 5];
    if film.blitz / snaps >= 0.25 {
        v.insert(0, 0);
    }
    let runs: f32 = film.look.iter().map(|x| x[0]).sum();
    let plays: f32 = film.look.iter().map(|x| x[0] + x[1]).sum();
    if plays > 0.0 && runs / plays >= 0.45 {
        v.push(3);
    }
    v
}

/// The "if they do this, we go to that" lines (G20), by kind: the trigger
/// and the move.
pub const IFS: [(&str, &str); 6] = [
    ("They blitz on 40% of passing downs", "Screens and draws behind it"),
    ("Their quarterback scrambles three times", "Spy him on passing downs"),
    ("Their top receiver passes 60 yards", "Bracket him"),
    ("They run for 5 a carry after 10 carries", "A safety down in the box"),
    ("They play six or fewer in the box on 60% of snaps", "Run it more"),
    ("A corner gives up two catches of 15 yards", "Attack him"),
];

/// Special teams for the week (G18). 0 is the staff's usual each time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Teams {
    /// Kickoffs: 1 deep, 2 squib, 3 pooch.
    pub kick: u8,
    /// Punts: 1 directional, 2 coffin corner in their end.
    pub punt: u8,
    /// Kickoff returns: 1 bring it out, 2 take the touchback.
    pub ret: u8,
    /// Punt returns: 1 set up the return, 2 safe (fair catch).
    pub pret: u8,
    /// A fake punt and an onside kick: 1 only if the look shows, 2 called
    /// at the first chance (once).
    pub fake: u8,
    pub onside: u8,
}

fn one() -> f64 {
    1.0
}

impl Default for Prep {
    fn default() -> Prep {
        Prep {
            fresh: 1.0,
            packages: Vec::new(),
            sheet: Vec::new(),
            stick: 0.0,
            noise: 1.0,
            rules: 1.0,
            dsheet: Vec::new(),
            dstick: 0.0,
            wet: Vec::new(),
            dwet: Vec::new(),
            learn: [0.0; 2],
            leak: [0.0; 2],
            limits: Vec::new(),
            orders: Vec::new(),
            answers: Default::default(),
            teams: Teams::default(),
            checks: 0,
            ifs: Vec::new(),
            backups: Vec::new(),
            rotation: Vec::new(),
            base_only: Vec::new(),
            boosts: Vec::new(),
            install_base: None,
            boost_added: Vec::new(),
            planned: false,
        }
    }
}

impl Prep {
    pub fn package(&self, board: u8) -> Option<&[u32]> {
        self.packages.iter().find(|(b, ids)| *b == board && !ids.is_empty()).map(|(_, v)| v.as_slice())
    }
}

/// A normal week's install, as a share of the playbook (proposed, not
/// confirmed).
pub const INSTALL: f64 = 0.75;

impl GameTeam {
    /// A team with no film, a settled line, nothing new and a normal week.
    pub fn new(name: String, players: Vec<GamePlayer>, staff: Staff) -> GameTeam {
        GameTeam {
            name,
            players,
            staff,
            film: Film::default(),
            line_chem: 0.6,
            novelty: [0.0; FAMILY_COUNT],
            week: 1.0,
            order: Vec::new(),
            prep: Prep::default(),
            day: Default::default(),
        }
    }

    /// The head coach's instructions to his players (tendencies topic 4.3),
    /// as AI staffs give them (proposed, not confirmed): a conservative
    /// coach tells his returners to kneel deep and fair-catch inside the 10
    /// and his quarterback to protect the ball, an aggressive one tells his
    /// quarterback to take shots; any coach who sees a habit that hurts the
    /// player (his tools don't pay for it) orders it away when his style
    /// doesn't want it. A sharper staff (film study) notices smaller ones.
    pub fn give_orders(&mut self, level: Level) {
        use crate::tendencies::{Instruction as I, Tend as T, misfit};
        let st = self.staff.style;
        let eye = 3.5 - 0.1 * (self.staff.film_study - 10.0);
        for p in self.players.iter_mut() {
            let mut o = Vec::new();
            let off = |t: T, p: &GamePlayer| misfit(t, p.tend.get(t), &p.attrs, level);
            let pos = p.position;
            if matches!(pos, Position::RB | Position::WR | Position::CB | Position::S)
                && (st.risk <= 7.0 || off(T::ReturnRisk, p) > eye)
            {
                o.push(I::KneelDeeperThan(if st.risk <= 5.0 { 1.0 } else { 4.0 }));
                o.push(I::FairCatchInsideTen);
            }
            if pos == Position::QB {
                if st.risk <= 8.0 && off(T::Gunslinger, p) > 0.5 * eye {
                    o.push(I::ProtectBall);
                } else if st.risk >= 14.0 && off(T::Gunslinger, p) < -0.5 * eye {
                    o.push(I::TakeShots);
                }
                if st.physical <= 10.0 && p.tend.get(T::Contact) >= 14.0 {
                    o.push(I::Slide);
                }
            }
            if matches!(pos, Position::QB | Position::RB | Position::WR | Position::TE)
                && st.risk < 15.0
                && off(T::Reach, p) > eye
            {
                o.push(I::HighAndTight);
            }
            if pos == Position::EDGE
                && (off(T::Upfield, p) > eye || st.freedom <= 6.0 && p.tend.get(T::Upfield) >= 14.0)
            {
                o.push(I::KeepContain);
            }
            if matches!(pos, Position::EDGE | Position::DT)
                && (off(T::SnapTiming, p) > eye || st.freedom <= 5.0 && p.tend.get(T::SnapTiming) >= 14.0)
            {
                o.push(I::NoSnapGuessing);
            }
            if matches!(pos, Position::LB | Position::S | Position::CB | Position::EDGE | Position::DT)
                && st.physical <= 12.0
                && (off(T::BigHit, p) > eye || st.ball_hunting <= 7.0 && p.tend.get(T::Strip) >= 15.0)
            {
                o.push(I::WrapUp);
            }
            if matches!(pos, Position::LB | Position::S | Position::CB)
                && st.freedom <= 12.0
                && off(T::JumpRoutes, p) > eye
            {
                o.push(I::StayHome);
            }
            p.orders = crate::tendencies::Orders(o);
        }
    }

    /// The plan the staff takes into this game and the load it puts on the
    /// players (T6.2). A short week cuts the plan, but only half the
    /// shortfall: real staffs trim the install and still see more mental
    /// errors. After a bye they add a little. Returns (the share of the
    /// playbook called, the load against a normal week's learning).
    pub fn plan(&self) -> (f64, f64) {
        let w = self.week.max(0.3);
        let planned = (self.staff.install * (1.0 - 0.5 * (1.0 - w))).clamp(0.2, 1.0);
        (planned, planned / w)
    }
}

/// How much of a game plan a player can absorb in a normal week, by his
/// awareness (T6.2): about all of a full plan at 75, two thirds at 60,
/// under half at 50 (proposed, not confirmed).
pub fn absorb(awareness: f64) -> f64 {
    ((awareness - 30.0) / 45.0).clamp(0.2, 1.3)
}

/// Days since a team's last game to its preparation (T6.2): four days or
/// fewer is a very short week, five a Thursday game, a week normal, a bye
/// more (proposed, not confirmed).
pub fn prep_week(days: Option<i32>) -> f64 {
    match days {
        None => 1.15,
        Some(d) if d <= 4 => 0.65,
        Some(5) => 0.8,
        Some(6) => 0.92,
        Some(d) if d <= 9 => 1.0,
        Some(d) if d <= 13 => 1.1,
        Some(_) => 1.15,
    }
}

/// The depth chart: players (indexes into `players`) in order at each spot.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Depth {
    pub at: [Vec<usize>; POSITION_COUNT],
    /// Kickoff kickers, best leg first (fix list 1.8). With three or four
    /// kickers the strongest leg kicks off and the field goal kicker
    /// (accuracy and leg, the K list) kicks field goals.
    pub kickoff: Vec<usize>,
    /// Long snapper, holder, kick and punt returners.
    pub snapper: usize,
    pub holder: usize,
    pub kick_returners: Vec<usize>,
    pub punt_returner: usize,
}

impl Depth {
    pub fn list(&self, pos: Position) -> &[usize] {
        &self.at[pos.index()]
    }
}

/// How many of each spot a team carries on game day.
fn wanted(pos: Position) -> usize {
    match pos {
        Position::QB => 2,
        Position::RB => 3,
        Position::WR => 6,
        Position::TE => 3,
        Position::OT => 3,
        Position::IOL => 4,
        Position::EDGE => 4,
        Position::DT => 4,
        Position::LB => 5,
        Position::CB => 5,
        Position::S => 3,
        // A backup at each specialist spot, so an injury falls to the next leg.
        Position::K | Position::P | Position::LS => 2,
    }
}

fn offense(pos: Position) -> bool {
    matches!(pos, Position::QB | Position::RB | Position::WR | Position::TE | Position::OT | Position::IOL)
}

/// Rating points a two-way corner or receiver must beat the starters by
/// on the other side (3.5): good enough at both, after what playing a spot
/// he doesn't practice costs him.
pub const CROSS_LIST: f64 = 3.0;

/// The head coach's depth chart (topic 5.1). Each spot lists the players
/// at that position first, best first. A player from another position in
/// the same group (fix list 3.1-3.2: tackles and guards, ends and tackles,
/// corners and safeties) is listed there only when he beats the players he
/// would pass after what the unfamiliar spot costs him (3.3), and never
/// ahead of others if he starts at his own spot. Another group's player is
/// listed only when the group runs out of bodies for its starters, an
/// emergency (3.6). A player is added to the other side's chart when he
/// would beat the starters there by `two_way_margin` rating points: small
/// high schools play their best athletes both ways (3.10); college and the
/// NFL never do (99), except a corner who is good enough at receiver, or a
/// receiver at corner (3.5, 3.11).
///
/// The kicking jobs go to the kickers and punters a team carries: with only
/// one of them, he does both (fix list 1.9), and only a team with neither
/// asks the best leg on the roster. A listed long snapper snaps; he plays
/// nowhere else (fix list 1.7).
pub fn depth_chart(team: &GameTeam, two_way_margin: f64) -> Depth {
    let mut d = staff_depth_chart(team, two_way_margin);
    // Your order goes first where you set one (section 8, topic 6): the
    // players you listed, in your order, then the staff's list for the rest.
    for (pos, ids) in &team.order {
        let mut list: Vec<usize> = ids.iter().filter_map(|id| team.players.iter().position(|p| p.id == *id)).collect();
        for &i in &d.at[pos.index()] {
            if !list.contains(&i) {
                list.push(i);
            }
        }
        d.at[pos.index()] = list;
    }
    d
}

/// The depth chart as the staff orders it.
fn staff_depth_chart(team: &GameTeam, two_way_margin: f64) -> Depth {
    let ps = &team.players;
    let mut d = Depth::default();
    let has_legs = ps.iter().any(|p| p.position.kicks());
    let has_snapper = ps.iter().any(|p| p.position == Position::LS);
    let own = |pos: Position| -> Vec<(f64, usize)> {
        let mut v: Vec<(f64, usize)> = ps
            .iter()
            .enumerate()
            .filter(|(_, p)| p.position == pos)
            .map(|(i, p)| (p.overall_at(pos) + p.impression, i))
            .collect();
        v.sort_by(|a, b| b.0.total_cmp(&a.0));
        v
    };
    // Starters at their own spot stay there.
    let mut starting = vec![false; ps.len()];
    for pos in Position::ALL {
        for (_, i) in own(pos).into_iter().take(starters(pos)) {
            starting[i] = true;
        }
    }
    for pos in Position::ALL {
        let mut list = own(pos);
        let n_own = list.len();
        let same_side = |p: &GamePlayer| match pos {
            // A kicker kicks and a punter punts: the other leg is only the
            // backup there; a team with neither asks the best leg it has.
            Position::K | Position::P => !has_legs || p.position.kicks(),
            // A listed long snapper snaps; without one, anyone can.
            Position::LS => !has_snapper || p.position == Position::LS,
            _ => !p.position.is_specialist() && spots::group(p.position) == spots::group(pos),
        };
        let kicks = pos.kicks() && has_legs;
        let mut cross: Vec<(f64, usize)> = ps
            .iter()
            .enumerate()
            .filter(|(i, p)| p.position != pos && same_side(p) && !starting[*i])
            .map(|(i, p)| (p.overall_at(pos) - if kicks { 100.0 } else { p.cross_cost(pos) }, i))
            .collect();
        cross.sort_by(|a, b| b.0.total_cmp(&a.0));
        list.extend(cross);
        list.sort_by(|a, b| b.0.total_cmp(&a.0));
        let want = wanted(pos).max(n_own);
        list.truncate(want);
        // Cover inside the group: anyone left in it, starters elsewhere last.
        if list.len() < want {
            let mut more: Vec<(f64, usize)> = ps
                .iter()
                .enumerate()
                .filter(|(i, p)| same_side(p) && !list.iter().any(|l| l.1 == *i))
                .map(|(i, p)| (p.overall_at(pos) - if starting[i] { 50.0 } else { 0.0 }, i))
                .collect();
            more.sort_by(|a, b| b.0.total_cmp(&a.0));
            list.extend(more.into_iter().take(want - list.len()));
        }
        // An emergency (3.6): the group can't field its starters, so the
        // closest fit on this side of the ball from another group, rated at
        // his true fit there, starters elsewhere last.
        if list.len() < starters(pos) && !pos.is_specialist() {
            let mut more: Vec<(f64, usize)> = ps
                .iter()
                .enumerate()
                .filter(|(i, p)| {
                    !p.position.is_specialist()
                        && offense(p.position) == offense(pos)
                        && !list.iter().any(|l| l.1 == *i)
                })
                .map(|(i, p)| (p.overall_at(pos) - if starting[i] { 50.0 } else { 0.0 }, i))
                .collect();
            more.sort_by(|a, b| b.0.total_cmp(&a.0));
            list.extend(more.into_iter().take(starters(pos) - list.len()));
        }
        d.at[pos.index()] = list.into_iter().map(|c| c.1).collect();
    }
    // Two-way players: the other side's best athletes can start here too.
    for pos in Position::ALL.into_iter().filter(|p| !p.is_specialist()) {
        let list = &d.at[pos.index()];
        let worst_starter =
            list.iter().take(starters(pos)).map(|&i| ps[i].overall_at(pos)).fold(f64::INFINITY, f64::min);
        // The corner and receiver exception (3.5) holds at every level.
        let swap = |p: &GamePlayer| {
            p.position == Position::CB && pos == Position::WR || p.position == Position::WR && pos == Position::CB
        };
        let mut extra: Vec<(f64, usize)> = ps
            .iter()
            .enumerate()
            .filter(|(_, p)| offense(p.position) != offense(pos) && !p.position.is_specialist())
            .map(|(i, p)| (p.overall_at(pos), i, swap(p)))
            .filter(|&(r, _, sw)| {
                let margin = if sw { two_way_margin.min(CROSS_LIST) } else { two_way_margin };
                r > worst_starter + margin || list.len() < starters(pos)
            })
            .map(|(r, i, _)| (r, i))
            .collect();
        if extra.is_empty() {
            continue;
        }
        extra.sort_by(|a, b| b.0.total_cmp(&a.0));
        let want = list.len().max(wanted(pos));
        let mut merged: Vec<(f64, usize)> = list
            .iter()
            .map(|&i| (ps[i].overall_at(pos) - if ps[i].position == pos { 0.0 } else { ps[i].cross_cost(pos) }, i))
            .chain(extra.into_iter().take(starters(pos)))
            .collect();
        merged.sort_by(|a, b| b.0.total_cmp(&a.0));
        merged.dedup_by_key(|m| m.1);
        d.at[pos.index()] = merged.into_iter().take(want).map(|m| m.1).collect();
    }
    let best = |f: &dyn Fn(&GamePlayer) -> f64, skip: &[usize]| -> usize {
        (0..ps.len()).filter(|i| !skip.contains(i)).max_by(|&a, &b| f(&ps[a]).total_cmp(&f(&ps[b]))).unwrap_or(0)
    };
    // The kicking legs (fix list 1.8, 1.9). The K and P lists already rank
    // every leg at each job, the listed specialist first: a team with only
    // a kicker or only a punter has him do both, rated at each from the
    // same leg. The kickoff goes to the best leg among the kickers.
    let legs = |pos: Position| -> Vec<usize> { (0..ps.len()).filter(|&i| ps[i].position == pos).collect() };
    let mut kickoff = legs(Position::K);
    if kickoff.is_empty() {
        kickoff = legs(Position::P);
    }
    if kickoff.is_empty() {
        kickoff = d.list(Position::K).to_vec();
    }
    kickoff.sort_by(|&a, &b| ps[b].a(Attr::KickPower).total_cmp(&ps[a].a(Attr::KickPower)));
    d.kickoff = kickoff;
    // The long snapper (fix list 1.7): the listed snapper, or else the best
    // snapper among players not already starting at the spots that take
    // the field together with him.
    let busy: Vec<usize> =
        [Position::QB, Position::K, Position::P].iter().flat_map(|p| d.list(*p).iter().take(1).copied()).collect();
    d.snapper = d
        .list(Position::LS)
        .iter()
        .copied()
        .find(|&i| ps[i].position == Position::LS && !busy.contains(&i))
        .unwrap_or_else(|| best(&|p| p.a(Attr::LongSnapping) + 0.1 * p.a(Attr::FootballIq), &busy));
    // The holder: the punter, unless he is the one kicking; else the backup quarterback.
    let kicker = d.list(Position::K).first().copied();
    d.holder = d
        .list(Position::P)
        .iter()
        .copied()
        .find(|&i| ps[i].position == Position::P && Some(i) != kicker && i != d.snapper)
        .unwrap_or_else(|| d.list(Position::QB)[1.min(d.list(Position::QB).len() - 1)]);
    let returner = |p: &GamePlayer| {
        0.4 * p.a(Attr::Speed)
            + 0.25 * p.a(Attr::Elusiveness)
            + 0.2 * p.a(Attr::BallCarrierVision)
            + 0.15 * p.a(Attr::BallTracking)
    };
    let mut kr: Vec<usize> =
        d.list(Position::WR).iter().chain(d.list(Position::RB)).chain(d.list(Position::CB)).copied().collect();
    kr.sort_by(|&a, &b| returner(&ps[b]).total_cmp(&returner(&ps[a])));
    kr.dedup();
    // Starters rarely return kicks: a return is extra hits for a man the
    // offense or defense needs. Backups do it unless there are none
    // (proposed, not confirmed).
    let qb1 = d.list(Position::QB).first().copied();
    kr.retain(|&i| Some(i) != qb1);
    let starting: Vec<usize> = [Position::RB, Position::WR, Position::CB]
        .iter()
        .flat_map(|&p| d.list(p).iter().take(starters(p)).copied())
        .collect();
    let backups: Vec<usize> = kr.iter().copied().filter(|i| !starting.contains(i)).collect();
    if backups.len() >= 2 {
        kr = backups;
    }
    d.kick_returners = kr.iter().take(2).copied().collect();
    d.punt_returner = kr
        .iter()
        .copied()
        .max_by(|&a, &b| {
            let f = |p: &GamePlayer| returner(p) + 0.3 * p.a(Attr::Catching);
            f(&ps[a]).total_cmp(&f(&ps[b]))
        })
        .unwrap_or(0);
    d
}

/// Starters at each spot in a base set.
pub fn starters(pos: Position) -> usize {
    match pos {
        Position::QB | Position::RB | Position::TE | Position::K | Position::P | Position::LS => 1,
        Position::WR => 3,
        Position::OT => 2,
        Position::IOL => 3,
        Position::EDGE | Position::DT | Position::LB | Position::CB | Position::S => 2,
    }
}

/// Energy drained by one play, before his stamina (topic 6). Running and
/// fighting on every snap tires linemen and defenders most.
pub fn drain(stamina: f64, work: f64) -> f64 {
    work * (1.55 - 0.011 * stamina).max(0.4)
}

/// Energy regained between plays: a little on the field, more on the bench.
pub const RECOVER_ON_FIELD: f64 = 0.012;
pub const RECOVER_ON_BENCH: f64 = 0.05;
