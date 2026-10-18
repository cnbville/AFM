//! The tracked play simulation (topics 1 and 2). Every player is an agent
//! with a position and velocity updated ten times a second. Movement is
//! physics (top speed, acceleration, cutting), each player only reacts to
//! what he can see, blocks are running battles, and tackles are fights
//! that play out step by step.

use super::geom::{DT, MID, V2, WIDTH, v2};
use super::roster::GamePlayer;
use crate::attributes::{ATTRIBUTE_COUNT, Attr};
use crate::player::Level;
use crate::rng::{Rng, clamp};

/// Players on the field at most, for the compact per-tick scans.
const MAX_AGENTS: usize = 32;

/// The ball carrier's 15 candidate headings (cosine, sine), worked out once.
fn headings() -> &'static [(f64, f64); 15] {
    static H: std::sync::OnceLock<[(f64, f64); 15]> = std::sync::OnceLock::new();
    H.get_or_init(|| {
        let mut h = [(0.0, 0.0); 15];
        for (k, x) in h.iter_mut().enumerate() {
            let ang = (k as f64 - 7.0) * 0.19;
            *x = (ang.cos(), ang.sin());
        }
        h
    })
}

/// Weather and setting for one game (topic 4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Conditions {
    /// Degrees Fahrenheit.
    pub temperature: f64,
    /// Miles per hour, and the direction it blows toward along the field
    /// (+1 toward the end zone the home team defends in the first quarter).
    pub wind: f64,
    pub wind_along: f64,
    pub rain: bool,
    pub snow: bool,
    /// Feet above sea level.
    pub altitude: f64,
    pub turf: bool,
    /// 0-1: how loud the home crowd is. On a snap it is the noise the
    /// offense plays into (0 for the home offense).
    pub crowd: f64,
    /// 0-1, on a snap: how unused the offense is to this cold (a warm
    /// climate or dome team in a freezing game); grip and throws suffer.
    pub cold_unused: f64,
}

impl Default for Conditions {
    fn default() -> Self {
        Conditions {
            temperature: 60.0,
            wind: 5.0,
            wind_along: 0.0,
            rain: false,
            snow: false,
            altitude: 500.0,
            turf: false,
            crowd: 0.6,
            cold_unused: 0.0,
        }
    }
}

/// An officiating crew's tendencies (topic 4): multipliers on how often
/// each kind of foul is called when it happens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crew {
    pub holding: f64,
    pub pass_interference: f64,
    pub pre_snap: f64,
    pub personal: f64,
}

impl Crew {
    pub fn draw(rng: &mut Rng) -> Crew {
        let mut m = || clamp(rng.normal(1.0, 0.15), 0.6, 1.5);
        Crew { holding: m(), pass_interference: m(), pre_snap: m(), personal: m() }
    }
}

/// What a player is doing on this snap.
#[derive(Clone, Debug, PartialEq)]
pub enum Job {
    Idle,
    /// Quarterback with the ball on a pass play.
    Passer,
    /// Pass protection on a rusher, or the next free one.
    PassBlock {
        target: Option<usize>,
    },
    /// Run block a defender, or climb to the second level.
    RunBlock {
        target: Option<usize>,
        pull: bool,
    },
    /// Running a route: waypoints, next waypoint, whether he settles.
    Route {
        pts: Vec<V2>,
        next: usize,
        settle: bool,
        name: &'static str,
        release_delay: f64,
    },
    /// A back taking a handoff: where he aims.
    Handoff {
        aim: V2,
        mesh: V2,
        at: f64,
    },
    /// Quarterback carrying out a run fake.
    Fake {
        to: V2,
    },
    /// Has the ball.
    Carrier,
    /// Rushing the passer through a lane, keeping contain on his side.
    Rush {
        lane: f64,
        contain: f64,
        mv: Move,
    },
    /// Playing a run gap (y at the line).
    Gap {
        y: f64,
    },
    /// Man coverage on a receiver.
    Man {
        target: usize,
        cushion: f64,
        lag: f64,
    },
    /// Zone: a landmark and how deep the zone is. A matched zone carries a
    /// receiver who goes vertical through it (pattern matching).
    Zone {
        spot: V2,
        deep: bool,
        width: f64,
        matched: bool,
    },
    /// Spying the quarterback (plays and tactics topic 5.4): mirror him
    /// from the second level, come at once if he runs.
    Spy,
    /// The pitch man on an option, keeping his relation to the quarterback.
    PitchMan,
    /// Chasing the ball.
    Pursue,
    /// Blocking for a ball carrier (returns, after a catch, after a turnover).
    Escort,
    /// Down and out of the play.
    Out,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Speed,
    Power,
    Finesse,
}

impl Move {
    pub fn name(self) -> &'static str {
        ["speed rush", "bull rush", "spin move"][self as usize]
    }
}

#[derive(Clone, Debug)]
pub struct Agent {
    /// 0: the team snapping or kicking, 1: the other team.
    pub side: usize,
    /// Index into that team's players.
    pub p: usize,
    pub id: u32,
    pub label: &'static str,
    pub pos: V2,
    pub vel: V2,
    pub face: V2,
    pub vmax: f64,
    pub accel: f64,
    /// Sprint time constant: speed builds as 1 - e^(-t/tau).
    pub tau: f64,
    pub cut: f64,
    pub mass: f64,
    pub job: Job,
    /// On the ground until this time.
    pub down_until: f64,
    pub engaged: Option<usize>,
    /// Effort spent this play, for fatigue.
    pub work: f64,
    /// When he knows what the play is (run or pass).
    pub read_at: f64,
    /// When he reacts to a thrown ball.
    pub ball_react: f64,
    /// He keeps his eyes where he chose (a backpedal facing the quarterback)
    /// until this time, whatever way he moves.
    pub face_lock: f64,
    /// A blown assignment this snap (topic 3).
    pub bust: bool,
    /// Last tackle attempt time on the current carrier.
    pub last_try: f64,
    /// Top speed reached this play, yards per second.
    pub top: f64,
    /// He has read the screen and is going for the back.
    pub smell: bool,
    /// A rusher on a stunt waits until this time before he loops.
    pub hold_until: f64,
    /// His run fit (plays and tactics topic 1): 0 none, 1 force (contain
    /// and turn it in), 2 fill his gap, 3 the alley from depth; the gap he
    /// owns at the line; and whether he misfits this snap.
    pub fit: u8,
    pub gap_y: f64,
    pub misfit: bool,
    /// Ratings with fatigue, injury and momentum in.
    pub r: [f64; ATTRIBUTE_COUNT],
}

impl Agent {
    /// How far bodies around him push him off his line: a player who moves
    /// well through traffic slides past them (proposed, not confirmed).
    #[inline]
    pub fn traffic(&self) -> f64 {
        clamp(1.0 - 0.008 * (self.a(Attr::Traffic) - 60.0), 0.55, 1.35)
    }

    pub fn new(side: usize, p: usize, gp: &GamePlayer, label: &'static str, pos: V2, energy: f64, edge: f64) -> Agent {
        // Fatigue lowers speed, strength and sharpness (topic 2).
        let tired = 1.0 - energy;
        let mut r = [0.0; ATTRIBUTE_COUNT];
        for (i, v) in gp.attrs.0.iter().copied().enumerate() {
            let fat = 1.0 - Attr::ALL[i].profile().fatigue * tired;
            r[i] = clamp(v * fat + edge, 1.0, 99.0);
        }
        let vmax = gp.top_speed() * (1.0 - 0.08 * tired);
        Agent {
            side,
            p,
            id: gp.id,
            label,
            pos,
            vel: V2::default(),
            face: v2(if side == 0 { 1.0 } else { -1.0 }, 0.0),
            vmax,
            accel: gp.accel() * (1.0 - 0.1 * tired),
            tau: gp.sprint_tau() * (1.0 + 0.1 * tired),
            cut: gp.cut(),
            mass: gp.weight,
            job: Job::Idle,
            down_until: -1.0,
            engaged: None,
            work: 0.0,
            read_at: 0.5,
            ball_react: f64::INFINITY,
            face_lock: 0.0,
            bust: false,
            last_try: -9.0,
            top: 0.0,
            smell: false,
            hold_until: 0.0,
            fit: 0,
            gap_y: 0.0,
            misfit: false,
            r,
        }
    }
    /// How hard he can stop, yards per second squared: his grip and his
    /// footwork (agility), as the shuttle measures it.
    /// A copy of him for looking ahead: everything but his job, so a
    /// route's waypoints are not copied.
    pub fn without_job(&self) -> Agent {
        Agent {
            side: self.side,
            p: self.p,
            id: self.id,
            label: self.label,
            pos: self.pos,
            vel: self.vel,
            face: self.face,
            vmax: self.vmax,
            accel: self.accel,
            tau: self.tau,
            cut: self.cut,
            mass: self.mass,
            job: Job::Idle,
            down_until: self.down_until,
            engaged: self.engaged,
            work: self.work,
            read_at: self.read_at,
            ball_react: self.ball_react,
            face_lock: self.face_lock,
            bust: self.bust,
            last_try: self.last_try,
            top: self.top,
            smell: self.smell,
            hold_until: self.hold_until,
            fit: self.fit,
            gap_y: self.gap_y,
            misfit: self.misfit,
            r: self.r,
        }
    }

    pub fn brake(&self) -> f64 {
        self.accel * (1.2 + 0.7 * self.cut)
    }

    /// Speed up, slow down and cut toward the wanted velocity, within
    /// his acceleration, sprint curve and agility.
    pub fn steer(&mut self, want: V2) {
        let a = self;
        let v = a.vel;
        let speed = v.len();
        let mut want = want;
        let wl = want.len();
        if wl > a.vmax {
            want = want.scale(a.vmax / wl);
        }
        let dv = want - v;
        let dir = if speed > 0.3 { v.scale(1.0 / speed) } else { dv.norm() };
        let par = dv.dot(dir);
        let perp = dv - dir.scale(par);
        // Speeding up: the sprint curve (fast early, easing to top speed).
        let up = ((a.vmax - speed).max(0.0) / a.tau).max(a.accel * 0.2) * DT;
        let down = a.brake() * DT;
        let par_c = par.clamp(-down, up);
        let pl = perp.len();
        let side = a.accel * a.cut * DT * (1.0 + 0.25 * (1.0 - speed / a.vmax));
        let perp_c = if pl > side { perp.scale(side / pl) } else { perp };
        a.vel = v + dir.scale(par_c) + perp_c;
        let s = a.vel.len();
        if s > a.vmax {
            a.vel = a.vel.scale(a.vmax / s);
        }
        a.work += DT * 0.35 * (0.2 + 0.8 * s / 9.0);
        if s > a.top {
            a.top = s;
        }
    }

    #[inline]
    pub fn a(&self, a: Attr) -> f64 {
        self.r[a.index()]
    }
    #[inline]
    pub fn up(&self, t: f64) -> bool {
        t >= self.down_until && !matches!(self.job, Job::Out)
    }
}

/// A block in progress (topic 1): `w` runs from -1 (the blocker has won:
/// driven, sealed or stoned) to +1 (the defender has shed him).
#[derive(Clone, Debug)]
pub struct Engagement {
    pub blocker: usize,
    pub defender: usize,
    pub helper: Option<usize>,
    pub w: f64,
    pub start: f64,
    pub pass: bool,
    pub mv: Move,
    pub over: bool,
    /// The blocker grabbed him when beaten (a hold, called or not).
    pub held: bool,
    /// He already had his one moment to grab when first beaten.
    pub grab_checked: bool,
    pub shed_at: Option<f64>,
    /// The rusher already went to his counter move.
    pub countered: bool,
}

/// A foul during the play.
#[derive(Clone, Debug, PartialEq)]
pub struct Foul {
    pub kind: FoulKind,
    pub side: usize,
    pub player: u32,
    /// Where it happened (x), for spot fouls.
    pub at: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FoulKind {
    OffensiveHolding,
    DefensiveHolding,
    PassInterference,
    OffensivePassInterference,
    RoughingPasser,
    UnnecessaryRoughness,
    FaceMask,
    IllegalBlock,
    FalseStart,
    Offside,
    DelayOfGame,
    IllegalFormation,
    IntentionalGrounding,
    /// A lineman more than the level's limit downfield when a pass is
    /// thrown past the line (an RPO thrown late).
    IneligibleDownfield,
}

impl FoulKind {
    pub fn name(self) -> &'static str {
        match self {
            FoulKind::OffensiveHolding => "offensive holding",
            FoulKind::DefensiveHolding => "defensive holding",
            FoulKind::PassInterference => "pass interference",
            FoulKind::OffensivePassInterference => "offensive pass interference",
            FoulKind::RoughingPasser => "roughing the passer",
            FoulKind::UnnecessaryRoughness => "unnecessary roughness",
            FoulKind::FaceMask => "face mask",
            FoulKind::IllegalBlock => "illegal block in the back",
            FoulKind::FalseStart => "false start",
            FoulKind::Offside => "offside",
            FoulKind::DelayOfGame => "delay of game",
            FoulKind::IllegalFormation => "illegal formation",
            FoulKind::IntentionalGrounding => "intentional grounding",
            FoulKind::IneligibleDownfield => "ineligible man downfield",
        }
    }
    /// Yards, and whether it is an automatic first down for the offense.
    pub fn yards(self) -> (f64, bool) {
        match self {
            FoulKind::OffensiveHolding | FoulKind::IllegalBlock | FoulKind::OffensivePassInterference => (10.0, false),
            FoulKind::DefensiveHolding => (5.0, true),
            FoulKind::PassInterference => (15.0, true),
            FoulKind::RoughingPasser | FoulKind::UnnecessaryRoughness | FoulKind::FaceMask => (15.0, true),
            FoulKind::FalseStart
            | FoulKind::Offside
            | FoulKind::DelayOfGame
            | FoulKind::IllegalFormation
            | FoulKind::IneligibleDownfield => (5.0, false),
            FoulKind::IntentionalGrounding => (10.0, false),
        }
    }
}

/// An injury on a play (topic 6).
#[derive(Clone, Debug, PartialEq)]
pub struct Injury {
    pub side: usize,
    pub p: usize,
    pub id: u32,
    pub kind: InjuryKind,
    /// Weeks he will miss.
    pub weeks: f64,
    /// Whether he plays on hurt (his toughness, the stakes, his coach).
    pub plays_on: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InjuryKind {
    Bruise,
    AnkleSprain,
    HighAnkleSprain,
    Hamstring,
    Knee,
    Acl,
    Shoulder,
    Concussion,
    Ribs,
    Hand,
    Fracture,
    Back,
}

impl InjuryKind {
    pub fn name(self) -> &'static str {
        match self {
            InjuryKind::Bruise => "bruise",
            InjuryKind::AnkleSprain => "ankle sprain",
            InjuryKind::HighAnkleSprain => "high ankle sprain",
            InjuryKind::Hamstring => "hamstring strain",
            InjuryKind::Knee => "knee sprain",
            InjuryKind::Acl => "torn ACL",
            InjuryKind::Shoulder => "shoulder injury",
            InjuryKind::Concussion => "concussion",
            InjuryKind::Ribs => "rib injury",
            InjuryKind::Hand => "hand injury",
            InjuryKind::Fracture => "broken bone",
            InjuryKind::Back => "back injury",
        }
    }
    /// The attributes it costs while he plays through it, as a share.
    pub fn hampers(self) -> &'static [(Attr, f64)] {
        match self {
            InjuryKind::AnkleSprain | InjuryKind::HighAnkleSprain => {
                &[(Attr::Speed, 0.08), (Attr::Agility, 0.12), (Attr::Acceleration, 0.08)]
            }
            InjuryKind::Hamstring => &[(Attr::Speed, 0.10), (Attr::Acceleration, 0.10)],
            InjuryKind::Knee => &[(Attr::Agility, 0.10), (Attr::Speed, 0.06)],
            InjuryKind::Shoulder => {
                &[(Attr::Tackling, 0.10), (Attr::RunBlocking, 0.08), (Attr::HandUse, 0.08), (Attr::ArmStrength, 0.10)]
            }
            InjuryKind::Ribs => &[(Attr::Strength, 0.06), (Attr::BallSecurity, 0.08)],
            InjuryKind::Hand => &[
                (Attr::Catching, 0.10),
                (Attr::BallSecurity, 0.08),
                (Attr::HandUse, 0.08),
                (Attr::ShortAccuracy, 0.08),
                (Attr::MediumAccuracy, 0.08),
                (Attr::DeepAccuracy, 0.08),
            ],
            InjuryKind::Back => &[(Attr::Strength, 0.08), (Attr::Agility, 0.06)],
            _ => &[(Attr::Speed, 0.03)],
        }
    }
}

/// What the sim records on a play, for stats, grades and the why.
#[derive(Clone, Debug, Default)]
pub struct Notes {
    pub pressures: Vec<(u32, f64)>,
    pub hits: Vec<u32>,
    pub sackers: Vec<u32>,
    pub tacklers: Vec<u32>,
    pub missed: Vec<(u32, u32)>,
    pub first_contact: Option<f64>,
    pub forced_fumble: Option<u32>,
    pub fumbled: Option<u32>,
    pub recovered: Option<(usize, u32)>,
    /// Blocks: (blocker, defender, pass, won, time held, gave up pressure).
    pub blocks: Vec<(u32, u32, bool, bool, f64, bool)>,
    /// Rush wins inside 2.5 s, with the move.
    pub rush_wins: Vec<(u32, Move, f64)>,
    pub busts: Vec<u32>,
    pub freelance: Vec<(u32, bool)>,
    pub fouls: Vec<Foul>,
    pub injuries: Vec<Injury>,
    pub top_speed: Vec<(u32, f64)>,
    pub why: Vec<String>,
    /// An RPO the quarterback pulled and threw; an option pitched.
    pub rpo_pulled: bool,
    pub pitched: bool,
    /// Run fits missed this snap (plays and tactics topic 1).
    pub misfits: Vec<u32>,
    /// A stunt the line passed off or didn't (topic 4.9).
    pub stunt: Option<bool>,
}

/// One frame of tracking for the 2D view (topic 9.3): every player and the ball.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub t: f64,
    pub ball: V2,
    pub players: Vec<(u32, f32, f32)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ball {
    /// Held by an agent.
    Held(usize),
    /// In the air from a throw or kick.
    Air {
        from: V2,
        to: V2,
        release: f64,
        arrive: f64,
        target: Option<usize>,
        height: f64,
    },
    Loose(V2),
    Dead(V2),
}

/// A play in progress.
pub struct Sim<'a> {
    pub teams: [&'a [GamePlayer]; 2],
    pub agents: Vec<Agent>,
    pub eng: Vec<Engagement>,
    pub ball: Ball,
    pub t: f64,
    pub los: f64,
    pub rng: &'a mut Rng,
    pub notes: Notes,
    pub record: Option<Vec<Frame>>,
    pub cond: Conditions,
    pub crew: Crew,
    /// Direction the carrier's team is going (+1 the snapping team, -1 after a turnover).
    pub goal: f64,
    pub carrier_side: usize,
    pub dead: bool,
    pub out_of_bounds: bool,
    /// Farthest forward spot of the ball carrier (forward progress).
    pub progress: f64,
    pub contact_force: f64,
    pub injury_scale: f64,
    pub down_on_fall: bool,
    /// Who tackled in the current fight.
    pub fight: Vec<usize>,
    pub fight_since: f64,
    /// The back on a screen: linemen who leak out block for him before the catch.
    pub screen: Option<usize>,
    /// How far a free defender in the open field can reach for the carrier.
    pub open_reach: f64,
    /// On a designed run, the pace every back takes to the line until he
    /// commits to a crease (yards per second; 0 on other plays).
    pub press: f64,
    /// The defense plays a two-gap front (a 3-4): its interior linemen
    /// read the block before they pick a gap (S9).
    pub two_gap: bool,
    /// On a designed run: Some(true) for a gap scheme (down blocks and a
    /// puller), Some(false) for a zone scheme, None otherwise.
    pub gap_run: Option<bool>,
    /// A stunt this snap: the penetrator and the looper (topic 1).
    pub stunt: Option<(usize, usize)>,
    /// The level being played: trait-like ratings are read against it.
    pub level: Level,
}

/// How much a run block's outcome leans on the strength gap per point.
/// Fitted (dynasty item 1, 2026-10-15) so FBS team-season yards per carry
/// spread like real FBS (offense and defense sd about 0.64, cfbfastR
/// 2022-24); 0.05 made run defense about 10% too decisive.
const RUN_EDGE: f64 = 0.035;
/// The extra pull a defender with leverage gets when the runner comes at
/// him (fitted with `RUN_EDGE`; 1.0 before).
const SHED_RUNNER: f64 = 0.5;

/// How far a free defender in the open field can reach or lunge for the
/// ball carrier, on scrimmage plays and returns alike. (2.2 before, and 1.6
/// on returns: runners slipped past squared-up defenders untouched, so
/// nine in ten runs past 20 yards went past 40 against two in ten real,
/// and long touchdowns ran near twice real; proposed, not confirmed.)
pub const OPEN_REACH: f64 = 2.5;

/// Rating points a road line loses off the snap at full crowd noise.
/// Fitted with the other home-field causes to the real home margin
/// (proposed, not confirmed).
const SILENT_COUNT: f64 = 24.0;

/// Print plays as they run (debug tooling only).
pub static TRACE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Normal draw.
#[inline]
pub fn gauss(rng: &mut Rng) -> f64 {
    rng.normal(0.0, 1.0)
}

impl<'a> Sim<'a> {
    pub fn player(&self, i: usize) -> &GamePlayer {
        let a = &self.agents[i];
        &self.teams[a.side][a.p]
    }

    /// A trait-like rating of agent `i` on the 1-20 scale, against the level
    /// (see `GamePlayer::trait20`).
    #[inline]
    pub fn trait20(&self, i: usize, a: Attr) -> f64 {
        self.player(i).trait20(a, self.level)
    }

    pub fn carrier(&self) -> Option<usize> {
        match self.ball {
            Ball::Held(i) => Some(i),
            _ => None,
        }
    }

    pub fn ball_pos(&self) -> V2 {
        match self.ball {
            Ball::Held(i) => self.agents[i].pos,
            Ball::Air { from, to, release, arrive, .. } => {
                let k = clamp((self.t - release) / (arrive - release).max(0.01), 0.0, 1.0);
                from + (to - from).scale(k)
            }
            Ball::Loose(p) | Ball::Dead(p) => p,
        }
    }

    /// Can `i` see point `p`? A cone of about 140 degrees in front, with
    /// peripheral vision widened by what he reads with (topic 2.1): a ball
    /// carrier his vision, a quarterback in the pocket his feel for it,
    /// everyone else play recognition. Anything within two yards is felt.
    pub fn sees(&self, i: usize, p: V2) -> bool {
        let a = &self.agents[i];
        let d = p - a.pos;
        if d.len() < 2.0 {
            return true;
        }
        let ang = a.face.angle_between(d);
        let read = if matches!(a.job, Job::Passer) {
            Attr::PocketPresence
        } else if self.carrier() == Some(i) {
            Attr::BallCarrierVision
        } else {
            Attr::PlayRecognition
        };
        let half = 1.2 + 0.006 * (a.a(read) - 50.0);
        ang < half
    }

    /// Closest distance between two agents over the last tick, so players
    /// closing fast can't pass through each other between two samples.
    pub fn closest(&self, i: usize, j: usize) -> f64 {
        let rel = self.agents[i].pos - self.agents[j].pos;
        let rv = (self.agents[i].vel - self.agents[j].vel).scale(DT);
        let back = if rv.len() > 1e-6 { clamp(rel.dot(rv) / rv.dot(rv), 0.0, 1.0) } else { 0.0 };
        (rel - rv.scale(back)).len()
    }

    /// Steer an agent toward a velocity within his physical limits (topic 2):
    /// speeding up is slower near top speed, braking is quicker, and
    /// cutting spends acceleration sideways, more for stiff players.
    pub fn steer(&mut self, i: usize, want: V2) {
        self.agents[i].steer(want);
    }

    /// Head for a point at full speed, easing in to stop on it.
    pub fn go_to(&mut self, i: usize, target: V2, urgency: f64) {
        let a = &self.agents[i];
        let d = target - a.pos;
        let dist = d.len();
        let speed = (a.vmax * urgency).min(dist * 2.2);
        self.steer(i, d.norm().scale(speed));
    }

    pub fn advance(&mut self) {
        let t = self.t;
        for i in 0..self.agents.len() {
            let a = &mut self.agents[i];
            if t < a.down_until {
                a.vel = a.vel.scale(0.5);
            }
            a.pos = a.pos + a.vel.scale(DT);
            if a.vel.len() > 1.0 && t >= a.face_lock {
                a.face = a.vel.norm();
            }
        }
        // Bodies don't pass through each other: free players slide around
        // engaged pairs and teammates.
        let n = self.agents.len();
        let carrier = self.carrier();
        if n > MAX_AGENTS {
            self.separate_slow(t, carrier);
            return;
        }
        // A compact copy, so the pair scan stays in cache: each side's
        // obstacles (its own players and anyone engaged), in index order.
        let mut pos = [V2::default(); MAX_AGENTS];
        let mut mover = [false; MAX_AGENTS];
        let mut obst = [[0u8; MAX_AGENTS]; 2];
        let mut n_obst = [0usize; 2];
        for (j, a) in self.agents.iter().enumerate() {
            pos[j] = a.pos;
            let engaged = a.engaged.is_some();
            mover[j] = !engaged && a.up(t) && Some(j) != carrier;
            if Some(j) == carrier {
                continue;
            }
            for (s, list) in obst.iter_mut().enumerate() {
                if engaged || a.side == s {
                    list[n_obst[s]] = j as u8;
                    n_obst[s] += 1;
                }
            }
        }
        for i in 0..n {
            if !mover[i] {
                continue;
            }
            let side = self.agents[i].side;
            let pi = pos[i];
            let mut push = V2::default();
            for &j in &obst[side][..n_obst[side]] {
                let j = j as usize;
                if i == j {
                    continue;
                }
                let d = pi - pos[j];
                // Clearly apart (well past 0.8 yards): no square root needed.
                if d.dot(d) > 0.65 {
                    continue;
                }
                let l = d.len();
                if l < 0.8 && l > 1e-6 {
                    push = push + d.scale((0.8 - l) / l * 0.5);
                }
            }
            pos[i] = pi + push.scale(self.agents[i].traffic());
            self.agents[i].pos = pos[i];
        }
    }

    /// The same separation for more players than the compact scan holds.
    fn separate_slow(&mut self, t: f64, carrier: Option<usize>) {
        let n = self.agents.len();
        for i in 0..n {
            if self.agents[i].engaged.is_some() || !self.agents[i].up(t) || Some(i) == carrier {
                continue;
            }
            let mut push = V2::default();
            for j in 0..n {
                if i == j || Some(j) == carrier {
                    continue;
                }
                let obstacle = self.agents[j].engaged.is_some() || self.agents[j].side == self.agents[i].side;
                if !obstacle {
                    continue;
                }
                let d = self.agents[i].pos - self.agents[j].pos;
                if d.dot(d) > 0.65 {
                    continue;
                }
                let l = d.len();
                if l < 0.8 && l > 1e-6 {
                    push = push + d.scale((0.8 - l) / l * 0.5);
                }
            }
            let k = self.agents[i].traffic();
            self.agents[i].pos = self.agents[i].pos + push.scale(k);
        }
    }

    // ---------------------------------------------------------------- blocks

    /// Start a block.
    /// A block in space: the defender may slip it (he sees it coming and
    /// dodges or rips through) and the blocker needs a moment to recover.
    /// A lineman in the phone booth almost always gets hit.
    pub fn lands_block(&mut self, b: usize, d: usize) -> bool {
        let t = self.t;
        // A set blocker gets his hands back up quicker than one running.
        let wait = if self.agents[b].vel.len() < 2.5 { 0.15 } else { 0.3 };
        if t - self.agents[b].last_try < wait {
            return false;
        }
        let speed = self.agents[d].vel.len();
        if matches!(self.agents[d].label, "DE" | "DT" | "NT") && speed < 3.0 {
            return true;
        }
        // Anyone barely moving yet is in the phone booth too: there is no
        // room or speed to slip the block.
        if speed < 2.0 {
            return true;
        }
        let (ba, da) = (&self.agents[b], &self.agents[d]);
        // Getting hands on a man in space is its own skill (climbing to a
        // linebacker, a screen, a return wall).
        let skill = 0.5 * ba.a(Attr::BlockingInSpace) + 0.5 * ba.a(Attr::Agility);
        let slip = 0.5 * da.a(Attr::Agility) + 0.5 * da.a(Attr::BlockShedding);
        // A defender running straight at him is easy to get hands on; one
        // crossing his face is not. Only the speed across counts.
        let u = if ba.vel.len() > 1.0 { ba.vel.norm() } else { (da.pos - ba.pos).norm() };
        let across = (da.vel.x * u.y - da.vel.y * u.x).abs();
        // A blocker who is set and square, with the man running straight at
        // him, rarely whiffs (a kick return wall, a pass set): only a dip or
        // a rip gets by (proposed, not confirmed).
        let set = ba.vel.len() < 2.5 && across < 2.0 && da.vel.dot((ba.pos - da.pos).norm()) > 2.0;
        let p_miss = if set {
            clamp(0.06 + 0.004 * (slip - skill), 0.02, 0.25)
        } else {
            clamp(0.22 + 0.006 * (slip - skill) + 0.05 * (across - 2.0).max(0.0), 0.05, 0.55)
        };
        if self.rng.chance(p_miss) {
            self.agents[b].last_try = t;
            // Slipping a block costs him his momentum: he has to dip or
            // side-step to get by.
            self.agents[d].vel = self.agents[d].vel.scale(0.6);
            return false;
        }
        true
    }

    pub fn engage(&mut self, b: usize, d: usize, pass: bool) {
        if self.agents[d].engaged.is_some() {
            // Help on a block already going: a double team.
            let k = self.agents[d].engaged.unwrap();
            if self.eng[k].helper.is_none() && self.eng[k].blocker != b {
                self.eng[k].helper = Some(b);
                self.agents[b].engaged = Some(k);
            }
            return;
        }
        let mv = if pass { self.pick_move(b, d) } else { Move::Power };
        let (bl, de) = (&self.agents[b], &self.agents[d]);
        // Get-off against the set: the first step decides a lot. The
        // rusher's jump is his burst and his read of the snap; the blocker
        // knows the count.
        let get_off = de.a(Attr::Acceleration) * 0.6 + de.a(Attr::Anticipation) * 0.4;
        let set = if pass {
            bl.a(Attr::PassBlocking) * 0.5 + bl.a(Attr::Agility) * 0.3 + bl.a(Attr::FootballIq) * 0.2
        } else {
            bl.a(Attr::RunBlocking) * 0.5 + bl.a(Attr::Acceleration) * 0.3 + bl.a(Attr::Strength) * 0.2
        };
        // Silent count (trial 4 topic 3): a road line in a loud stadium
        // snaps on a visual cue, a beat late, and the defense times it.
        // A focused lineman reacts to the snap cue sooner.
        let focus = clamp(1.0 - 0.05 * (self.trait20(b, Attr::Concentration) - 10.0), 0.5, 1.5);
        let silent = if bl.side == 0 && self.t < 1.0 { SILENT_COUNT * self.cond.crowd * focus } else { 0.0 };
        let w0 = if pass { -0.42 } else { -0.3 }
            + 0.006 * (get_off + silent - set)
            + self.front_fit(de.label, pass)
            + 0.18 * gauss(self.rng);
        let k = self.eng.len();
        self.eng.push(Engagement {
            blocker: b,
            defender: d,
            helper: None,
            w: w0,
            start: self.t,
            pass,
            mv,
            over: false,
            held: false,
            grab_checked: false,
            shed_at: None,
            countered: false,
        });
        self.agents[b].engaged = Some(k);
        self.agents[d].engaged = Some(k);
    }

    /// How the front's technique fits the play (S9), in block-win units:
    /// a two-gap interior lineman holds his ground against a gap scheme but
    /// gets stretched by zone and rushes the passer a beat late; a one-gap
    /// lineman shoots into the backfield against zone and gets washed down
    /// by a gap scheme (sizes proposed, not confirmed).
    fn front_fit(&self, label: &str, pass: bool) -> f64 {
        let dl = matches!(label, "DT" | "DE");
        if !dl {
            return 0.0;
        }
        let two_gap = self.two_gap && label == "DT";
        match (pass, self.gap_run, two_gap) {
            (true, _, true) => -0.03,
            (false, Some(true), true) => 0.05,
            (false, Some(false), true) => -0.02,
            (false, Some(true), false) => -0.02,
            (false, Some(false), false) => 0.03,
            _ => 0.0,
        }
    }

    fn pick_move(&mut self, b: usize, d: usize) -> Move {
        let mut best = (Move::Speed, f64::NEG_INFINITY);
        for mv in [Move::Speed, Move::Power, Move::Finesse] {
            let edge = self.rush_power(d, mv) - self.block_power(b, mv, true);
            // Smart rushers pick the right move more often.
            let noise = (11.0 - 0.09 * self.agents[d].a(Attr::FootballIq)).max(2.0) * gauss(self.rng);
            if edge + noise > best.1 {
                best = (mv, edge + noise);
            }
        }
        best.0
    }

    /// Rusher strength for a move, on the rating scale. A speed rush is
    /// the finesse rusher's arc, won with burst and bend; a power rush is
    /// strength through the hands; a finesse move is hands and quickness.
    fn rush_power(&self, d: usize, mv: Move) -> f64 {
        let a = &self.agents[d];
        match mv {
            Move::Speed => {
                0.4 * a.a(Attr::FinesseRush)
                    + 0.2 * a.a(Attr::Speed)
                    + 0.15 * a.a(Attr::Acceleration)
                    + 0.15 * a.a(Attr::Flexibility)
                    + 0.1 * a.a(Attr::HandUse)
            }
            Move::Power => {
                0.4 * a.a(Attr::PowerRush)
                    + 0.25 * a.a(Attr::Strength)
                    + 0.15 * a.a(Attr::HandUse)
                    + 0.1 * a.a(Attr::Balance)
                    + 0.1 * a.a(Attr::BlockShedding)
                    + (a.mass - 280.0) * 0.08
            }
            Move::Finesse => {
                0.45 * a.a(Attr::FinesseRush)
                    + 0.25 * a.a(Attr::Agility)
                    + 0.2 * a.a(Attr::HandUse)
                    + 0.1 * a.a(Attr::BlockShedding)
            }
        }
    }

    fn block_power(&self, b: usize, mv: Move, pass: bool) -> f64 {
        let a = &self.agents[b];
        // How strong he plays: raw strength with his hands, balance and
        // mass, not the bench press alone.
        if !pass {
            return 0.45 * a.a(Attr::RunBlocking)
                + 0.2 * a.a(Attr::Strength)
                + 0.15 * a.a(Attr::HandUse)
                + 0.1 * a.a(Attr::Balance)
                + 0.1 * a.a(Attr::Acceleration)
                + (a.mass - 300.0) * 0.08;
        }
        let pb = a.a(Attr::PassBlocking);
        match mv {
            // Mirroring the arc: feet, bend and a punch that lands.
            Move::Speed => {
                0.5 * pb + 0.2 * a.a(Attr::Agility) + 0.15 * a.a(Attr::Flexibility) + 0.15 * a.a(Attr::HandUse)
            }
            // The anchor against the bull rush.
            Move::Power => {
                0.4 * pb
                    + 0.3 * a.a(Attr::Anchor)
                    + 0.15 * a.a(Attr::Strength)
                    + 0.15 * a.a(Attr::Balance)
                    + (a.mass - 300.0) * 0.08
            }
            // Hands against hands.
            Move::Finesse => {
                0.45 * pb + 0.3 * a.a(Attr::HandUse) + 0.15 * a.a(Attr::Agility) + 0.1 * a.a(Attr::FootballIq)
            }
        }
    }

    fn shed_power(&self, d: usize) -> f64 {
        let a = &self.agents[d];
        // Stack (hands and anchor), read, then shed.
        0.4 * a.a(Attr::BlockShedding)
            + 0.2 * a.a(Attr::HandUse)
            + 0.15 * a.a(Attr::Anchor)
            + 0.1 * a.a(Attr::Strength)
            + 0.15 * a.a(Attr::PlayRecognition)
            + (a.mass - 280.0) * 0.08
    }

    /// How well a defender holds the point against a double team: anchor,
    /// strength, balance and mass, on the rating scale.
    fn anchor(&self, d: usize) -> f64 {
        let a = &self.agents[d];
        0.6 * a.a(Attr::Anchor) + 0.2 * a.a(Attr::Strength) + 0.2 * a.a(Attr::Balance) + (a.mass - 300.0) * 0.08
    }

    /// One tick of every block. Pass blocks drift toward the quarterback as
    /// the rusher wins ground; run blocks move with whoever is winning.
    pub fn update_blocks(&mut self, qb: Option<usize>, carrier: Option<usize>) {
        let t = self.t;
        for k in 0..self.eng.len() {
            if self.eng[k].over {
                continue;
            }
            let e = self.eng[k].clone();
            let (b, d) = (e.blocker, e.defender);
            if !self.agents[b].up(t) || !self.agents[d].up(t) {
                self.release(k);
                continue;
            }
            // Edge of the defender in rating points.
            let mut edge = if e.pass {
                self.rush_power(d, e.mv) - self.block_power(b, e.mv, true)
            } else {
                self.shed_power(d) - self.block_power(b, e.mv, false)
            };
            if let Some(h) = e.helper {
                // A stout man holds the point against two (proposed, not
                // confirmed: 0.3 of his anchor above 60).
                edge -= 0.6 * self.block_power(h, e.mv, e.pass) - 20.0 - 0.3 * (self.anchor(d) - 60.0);
            }
            // A stalled rush: once a rep, the rusher goes to his counter
            // when his plan says it beats the set better (topic 1).
            if e.pass && !e.countered && e.w < -0.2 && t - e.start > 0.8 {
                self.eng[k].countered = true;
                let plan = clamp(
                    0.45 + 0.008 * (self.agents[d].a(Attr::FootballIq) - 60.0)
                        + 0.03 * (self.trait20(d, Attr::Motor) - 10.0),
                    0.15,
                    0.85,
                );
                if self.rng.chance(plan) {
                    let mv = self.pick_move(b, d);
                    if mv != e.mv && self.rush_power(d, mv) - self.block_power(b, mv, true) > edge {
                        self.eng[k].mv = mv;
                        edge = self.rush_power(d, mv) - self.block_power(b, mv, true);
                    }
                }
            }
            // Run defenders must first see the run to fight toward it.
            let reading = !e.pass && t < self.agents[d].read_at;
            let base = if e.pass {
                0.0
            } else if reading {
                -0.12
            } else {
                0.0
            };
            let mut mu = base + if e.pass { 0.032 } else { RUN_EDGE } * edge;
            // The runner already past him: he turns and runs after him, and
            // the blocker can't hang on (that would be holding).
            if !e.pass
                && let Some(c) = carrier
                && (self.agents[c].pos.x - self.agents[d].pos.x) * self.goal > 1.5
                && self.rng.chance(
                    0.18 * clamp(
                        1.0 + 0.04 * (self.trait20(d, Attr::Motor) - 10.0)
                            - 0.006 * (self.agents[b].a(Attr::HandUse) - 60.0),
                        0.5,
                        1.6,
                    ),
                )
            {
                self.release(k);
                continue;
            }
            // The runner coming at him: a defender with leverage on the
            // block fights off it to make the play (shed and tackle).
            if !e.pass
                && !reading
                && let Some(c) = carrier
                && self.agents[c].pos.dist(self.agents[d].pos) < 2.2
                && e.w > 0.0
            {
                // (Any leverage at all, and twice as hard as it was: real
                // runners meet first contact about a yard and a half past
                // the line, the engine's at three and a half; proposed, not
                // confirmed.)
                mu += 2.0 * SHED_RUNNER * (0.5 + e.w).max(0.2);
            }
            if TRACE.load(std::sync::atomic::Ordering::Relaxed) && (t - e.start) < DT * 1.5 {
                println!(
                    "BLOCK {}->{} pass {} mv {:?} edge {:.1} mu {:.2} w {:.2}",
                    self.agents[b].label, self.agents[d].label, e.pass, e.mv, edge, mu, e.w
                );
            }
            let sd = if e.pass { 0.58 } else { 0.6 };
            let w = e.w + mu * DT + sd * DT.sqrt() * gauss(self.rng);
            self.eng[k].w = w;
            self.agents[b].work += DT * if e.pass { 0.5 } else { 0.9 };
            self.agents[d].work += DT * if e.pass { 1.0 } else { 0.9 };
            // A beaten blocker may grab him (topic 4).
            if w > 0.75 && !e.grab_checked {
                // One moment of decision when he is first beaten.
                self.eng[k].grab_checked = true;
                let disc = self.trait20(b, Attr::Discipline);
                let tech =
                    if e.pass { self.agents[b].a(Attr::PassBlocking) } else { self.agents[b].a(Attr::RunBlocking) };
                // Raised a third (offensive holding ran 0.88 a team game against
                // about 1.26 real, illegal hands included; proposed, not confirmed).
                // Hands outside the frame get caught (hand use).
                let hands = clamp(1.0 - 0.012 * (self.agents[b].a(Attr::HandUse) - 60.0), 0.5, 1.6);
                let p = 0.046 * (1.7 - disc / 14.0) * (1.5 - tech / 100.0) * hands;
                if self.rng.chance(p) {
                    self.eng[k].held = true;
                    self.eng[k].w = 0.35;
                    let called = 0.42 * self.crew.holding;
                    if self.rng.chance(called) {
                        let id = self.agents[b].id;
                        let at = self.agents[b].pos.x;
                        let side = self.agents[b].side;
                        self.notes.fouls.push(Foul { kind: FoulKind::OffensiveHolding, side, player: id, at });
                    }
                    continue;
                }
            }
            if w >= 1.0 {
                // Shed.
                self.eng[k].shed_at = Some(t);
                // A win counts only while the quarterback still holds the ball.
                if e.pass && t - e.start < 2.5 && qb.is_some() {
                    let id = self.agents[d].id;
                    self.notes.rush_wins.push((id, e.mv, t - e.start));
                }
                self.release(k);
                // He comes off the block where he won it.
                let push = match e.mv {
                    Move::Speed => 0.6,
                    _ => 0.3,
                };
                let toward =
                    qb.or(carrier).map(|q| (self.agents[q].pos - self.agents[d].pos).norm()).unwrap_or(v2(-1.0, 0.0));
                self.agents[d].vel = toward.scale(push * self.agents[d].vmax * 0.4);
                continue;
            }
            if w <= -1.0 && !e.pass {
                // Driven off the ball or sealed. Now and then he is put on the
                // ground; otherwise the blocker stays on him and keeps him
                // sealed (a won block is sustained, not let go).
                // A pancake: the blocker's finish and strength against the
                // man's balance (proposed, not confirmed).
                let finish = clamp(
                    1.0 + 0.04 * (self.trait20(b, Attr::Physicality) - 10.0)
                        + 0.006 * (self.agents[b].a(Attr::Strength) - 60.0)
                        - 0.008 * (self.agents[d].a(Attr::Balance) - 60.0),
                    0.4,
                    1.8,
                );
                if self.rng.chance(0.25 * finish * DT / 0.5) {
                    self.agents[d].down_until = t + 0.6 + 0.6 * self.rng.next();
                    self.release(k);
                    self.notes.blocks.push((self.agents[b].id, self.agents[d].id, false, true, t - e.start, false));
                    continue;
                }
                self.eng[k].w = -1.0;
            }
            // Where the pair goes.
            let (bp, dp) = (self.agents[b].pos, self.agents[d].pos);
            let target = if e.pass {
                qb.map(|q| self.agents[q].pos).unwrap_or(bp)
            } else {
                carrier.map(|c| self.agents[c].pos).unwrap_or(dp + v2(-self.goal, 0.0))
            };
            let dir = (target - dp).norm();
            let drive = if e.pass {
                // Bull rushes collapse the pocket; speed rushes bend the edge.
                let power = if e.mv == Move::Power { 1.4 } else { 0.7 };
                (0.25 + 0.65 * (w + 1.0) / 2.0) * power
            } else {
                1.6 * w
            };
            let v = dir.scale(drive.max(-2.0));
            let lateral = if e.pass && e.mv == Move::Speed { dir.perp().scale(0.0) } else { V2::default() };
            let mid = (bp + dp).scale(0.5);
            let off = (dp - bp).norm();
            let mid = mid + (v + lateral).scale(DT);
            self.agents[b].pos = mid - off.scale(0.45);
            self.agents[d].pos = mid + off.scale(0.45);
            self.agents[b].vel = v;
            self.agents[d].vel = v;
            if let Some(h) = e.helper {
                self.agents[h].pos = self.agents[b].pos + v2(0.0, if self.agents[h].pos.y > bp.y { 0.7 } else { -0.7 });
                self.agents[h].vel = v;
            }
        }
    }

    pub fn release(&mut self, k: usize) {
        let e = &mut self.eng[k];
        e.over = true;
        let (b, d, h) = (e.blocker, e.defender, e.helper);
        let t = self.t - e.start;
        let won = e.shed_at.is_none();
        let pass = e.pass;
        self.agents[b].engaged = None;
        self.agents[d].engaged = None;
        if let Some(h) = h {
            self.agents[h].engaged = None;
        }
        if e.shed_at.is_some() {
            let (bid, did) = (self.agents[b].id, self.agents[d].id);
            self.notes.blocks.push((bid, did, pass, won, t, false));
        }
    }

    /// End every block still going (play over), crediting the blockers.
    pub fn close_blocks(&mut self) {
        for k in 0..self.eng.len() {
            if !self.eng[k].over {
                let e = self.eng[k].clone();
                let (bid, did) = (self.agents[e.blocker].id, self.agents[e.defender].id);
                self.notes.blocks.push((bid, did, e.pass, e.w < 0.6, self.t - e.start, false));
                self.release(k);
            }
        }
    }

    // ------------------------------------------------------------ ball carrier

    /// The runner picks his path (topic 2): every tick he weighs a fan of
    /// headings by how much ground they gain and how soon defenders he can
    /// see would get there. Vision decides how many he accounts for and how
    /// well; a designed aim point holds until he reaches the line.
    pub fn run_carrier(&mut self, c: usize, aim: Option<V2>) {
        let t = self.t;
        let me = self.agents[c].clone();
        let goal = self.goal;
        let vision = me.a(Attr::BallCarrierVision);
        let sight = 7.0 + 0.12 * vision;
        // On a designed run he presses the line at the play's pace, the same
        // for every back, and picks his crease at that pace: the blocks and
        // the box set the run to the line, his speed shows past it (dynasty
        // item 1; was full speed from the handoff).
        let pressing = self.press > 0.0 && aim.is_some() && (me.pos.x - self.los) * goal < 0.5;
        let speed = if pressing { self.press.min(me.vmax) } else { me.vmax.max(4.0) };
        let mut best = (f64::NEG_INFINITY, v2(goal, 0.0));
        let noise = (1.0 - 0.008 * vision).max(0.2);
        // The defenders he reckons with, and how free each is: the same
        // for every heading, so worked out once.
        let mut seen: [(V2, f64, f64); 32] = [(V2::default(), 0.0, 0.0); 32];
        let mut n_seen = 0;
        for d in self.agents.iter() {
            if d.side == me.side || !d.up(t) {
                continue;
            }
            let dist = d.pos.dist(me.pos);
            if dist > sight + 6.0 {
                continue;
            }
            // Defenders behind him or far away count only if he can see them.
            if dist > 3.0 && !self.sees(c, d.pos) && dist > sight * 0.5 {
                continue;
            }
            let engaged = d.engaged.map(|k| self.eng[k].w);
            let reach = match engaged {
                Some(w) => 0.15 + 0.35 * ((w + 1.0) / 2.0),
                None => 1.0,
            };
            seen[n_seen] = (d.pos, d.vmax, reach);
            n_seen += 1;
        }
        for &(cos, sin) in headings() {
            let dir = v2(goal * cos, sin);
            let look = 3.5;
            let p = me.pos + dir.scale(look);
            let t_me = look / speed.max(me.vel.len() * 0.5 + speed * 0.5);
            let mut threat = 0.0;
            for &(pos, vmax, reach) in &seen[..n_seen] {
                let td = (pos.dist(p) - 1.0).max(0.0) / vmax + 0.15;
                let lead = t_me - td;
                threat += reach * (lead / 0.35).exp().min(6.0);
            }
            let gain = dir.x * goal;
            let edge = p.y.min(WIDTH - p.y);
            let sideline = if edge < 1.0 { 3.0 * (1.0 - edge) } else { 0.0 };
            // He commits to a path: changing course costs speed, so the
            // heading he is on gets a bonus once he is moving.
            let keep = if me.vel.len() > 2.0 { 0.9 * dir.dot(me.vel.norm()) } else { 0.0 };
            let mut score = 1.5 * gain - 1.1 * threat - sideline + keep + noise * gauss(self.rng) * 0.6;
            if let Some(a) = aim {
                let wanted = (a - me.pos).norm();
                score += 1.2 * dir.dot(wanted);
            }
            if score > best.0 {
                best = (score, dir);
            }
        }
        let dir = best.1;
        // With the ball in one arm he runs a little under his top speed.
        let pace = if pressing { speed.min(self.agents[c].vmax * 0.95) } else { self.agents[c].vmax * 0.95 };
        self.steer(c, dir.scale(pace));
        let x = self.agents[c].pos.x;
        if (x - self.progress) * goal > 0.0 {
            self.progress = x;
        }
    }

    /// Defenders take angles to the ball carrier (topic 2). Smart ones aim
    /// where he is going, the rest chase where he is; lazy ones ease off.
    pub fn pursue(&mut self, i: usize, c: usize) {
        self.pursue_cap(i, c, 1.0, None);
    }

    /// Pursuit at a share of full speed (a safety coming down under control).
    pub fn pursue_cap(&mut self, i: usize, c: usize, cap: f64, floor: Option<f64>) {
        let a = &self.agents[i];
        let cp = self.agents[c].pos;
        let cv = self.agents[c].vel;
        let iq = 0.5 * a.a(Attr::PlayRecognition) + 0.5 * a.a(Attr::Pursuit);
        let dist = a.pos.dist(cp);
        // The intercept point: where he can meet the runner at full speed.
        // Smart pursuers take most of that angle, others aim short of it.
        let rel = cp - a.pos;
        let s = a.vmax.max(1.0);
        let (qa, qb, qc) = (cv.dot(cv) - s * s, 2.0 * rel.dot(cv), rel.dot(rel));
        let disc = qb * qb - 4.0 * qa * qc;
        let meet = if qa.abs() > 1e-6 && disc >= 0.0 {
            let r1 = (-qb - disc.sqrt()) / (2.0 * qa);
            let r2 = (-qb + disc.sqrt()) / (2.0 * qa);
            [r1, r2].into_iter().filter(|&r| r > 0.0).fold(f64::INFINITY, f64::min)
        } else {
            f64::INFINITY
        };
        let tt = if meet.is_finite() { meet.min(4.0) } else { dist / s };
        let mut lead = clamp(0.92 + 0.004 * (iq - 60.0), 0.75, 1.05);
        // The last man in front takes a deeper angle so the runner can't
        // get outside or past him.
        if (a.pos.x - cp.x) * self.goal > 3.0 && dist > 4.0 {
            lead = lead.max(1.1 + 0.003 * (iq - 50.0));
        }
        let mut target = cp + cv.scale(tt * lead);
        // An alley player keeps his depth: he meets the runner past the line.
        if let Some(f) = floor
            && (target.x - f) * self.goal < 0.0
        {
            target.x = f;
        }
        // In front of the runner and close, he breaks down and squares up:
        // he shuffles to where the runner will cross his spot and comes
        // forward under control, so the runner has to beat him with a move
        // (topic 2.2). Good tacklers keep more speed doing it.
        let ahead = (a.pos.x - cp.x) * self.goal;
        if ahead > 0.4 && ahead < 5.0 && dist < 6.0 && cap >= 1.0 {
            // Breaking down is footwork and feel, not the tackle itself
            // (dynasty item 1: tackling now decides only the finish).
            let feet = 0.5 * a.a(Attr::Agility) + 0.3 * a.a(Attr::Tackling) + 0.2 * a.a(Attr::Pursuit);
            let fwd = clamp(1.5 + 0.04 * (feet - 50.0), 0.8, 3.5);
            let run_x = (cv.x * self.goal).max(0.5);
            let t_meet = ahead / (run_x + fwd);
            let y_aim = clamp(cp.y + cv.y * t_meet, 0.5, WIDTH - 0.5);
            let me = a.pos;
            let lateral = clamp((y_aim - me.y) * 4.0, -a.vmax, a.vmax);
            let want = v2(-self.goal * fwd, lateral);
            self.steer(i, want);
            return;
        }
        let motor = self.teams[a.side][a.p].trait20(Attr::Motor, self.level);
        // A player out of the play with a low motor stops chasing.
        let behind = (a.pos.x - cp.x) * self.goal < -4.0 && dist > 8.0;
        let mut effort = if behind && motor < 9.0 { 0.8 + 0.02 * motor } else { 1.0 };
        // Facing the runner up close he breaks down to make the tackle
        // instead of flying past; good tacklers keep more speed.
        let facing = (a.pos.x - cp.x) * self.goal > 0.0;
        if facing && dist < 4.0 {
            let feet = 0.5 * a.a(Attr::Agility) + 0.3 * a.a(Attr::Tackling) + 0.2 * a.a(Attr::Pursuit);
            effort *= clamp(0.55 + 0.004 * feet, 0.6, 0.95);
        }
        // Full speed at the moving aim point (go_to would ease into it).
        let me = self.agents[i].pos;
        let vm = self.agents[i].vmax * effort * cap;
        let dir = (target - me).norm();
        let dir = if dir.len() < 0.5 { (cp - me).norm() } else { dir };
        self.steer(i, dir.scale(vm));
    }

    /// Tackling (topic 2.2). A free defender in reach tries to tackle: he can
    /// whiff on a move, or wrap up and start a fight where momentum,
    /// strength and help decide how far the runner falls forward.
    pub fn tackles(&mut self, c: usize) {
        let t = self.t;
        // Arm's length standing; a free man can lunge or dive a bit farther
        // at a worse chance (see first_hit).
        let mut new_tacklers = Vec::new();
        for i in 0..self.agents.len() {
            let d = &self.agents[i];
            if d.side == self.agents[c].side || !d.up(t) || self.fight.contains(&i) {
                continue;
            }
            // In space (past the line, no bodies around) a free man can dive.
            let in_space = (self.agents[c].pos.x - self.los) * self.goal > 3.0;
            let free_reach = if d.engaged.is_none() && in_space { self.open_reach } else { 1.6 };
            // A sure tackler wraps from farther (wrap range and dives).
            let wrap = if free_reach > 1.6 { clamp(0.01 * (d.a(Attr::Tackling) - 60.0), -0.4, 0.4) } else { 0.0 };
            let reach = free_reach + wrap + 0.03 * (self.player(i).arm - 32.0);
            // Closest approach over the last tick, so fast players can't
            // pass through each other between two samples.
            let rel = d.pos - self.agents[c].pos;
            let rv = (d.vel - self.agents[c].vel).scale(DT);
            let back = if rv.len() > 1e-6 { clamp(rel.dot(rv) / rv.dot(rv), 0.0, 1.0) } else { 0.0 };
            let dist = (rel - rv.scale(back)).len();
            if dist > reach || t - d.last_try < 0.7 {
                continue;
            }
            // A blocked defender can reach out with an arm while engaged.
            let engaged_w = d.engaged.map(|k| self.eng[k].w);
            if let Some(w) = engaged_w
                && (w < -0.2 || !self.rng.chance(0.25 + 0.3 * w))
            {
                continue;
            }
            if engaged_w.is_some() && dist > reach - 0.55 {
                continue;
            }
            new_tacklers.push((i, engaged_w.is_some(), (dist - (reach - 0.55)).max(0.0)));
        }
        for (i, arm_only, lunge) in new_tacklers {
            self.agents[i].last_try = t;
            if self.fight.is_empty() {
                self.first_hit(c, i, arm_only, lunge);
            } else {
                // Help arrives.
                self.fight.push(i);
                self.hit_effects(c, i, 0.5);
            }
            if self.dead || self.carrier() != Some(c) {
                return;
            }
        }
        if !self.fight.is_empty() {
            self.fight_tick(c);
        }
    }

    fn first_hit(&mut self, c: usize, i: usize, arm_only: bool, lunge: f64) {
        let t = self.t;
        let (r, d) = (self.agents[c].clone(), self.agents[i].clone());
        let closing = (d.vel - r.vel).len();
        let head_on = r.vel.norm().dot((d.pos - r.pos).norm());
        // Whiff: the runner's move against the tackler's technique.
        let evade = 0.5 * r.a(Attr::Elusiveness) + 0.3 * r.a(Attr::Agility) + 0.2 * r.a(Attr::Speed);
        let tackle = 0.6 * d.a(Attr::Tackling) + 0.2 * d.a(Attr::Agility) + 0.2 * d.a(Attr::Pursuit);
        // Out of control when arriving too fast, behind when chasing.
        let angle = if head_on > 0.5 {
            -0.3
        } else if head_on < -0.3 {
            0.3
        } else {
            0.0
        };
        let fast = if closing > 9.0 { 0.25 } else { 0.0 };
        let space = if r.vel.len() > 5.0 { 0.25 } else { -0.2 };
        let arm = if arm_only { 1.5 } else { 0.0 };
        let dive = 2.0 * lunge;
        let logit = -3.0 + (evade - tackle) / 50.0 + angle + fast + space + arm + dive;
        let p_miss = 1.0 / (1.0 + (-logit).exp());
        if TRACE.load(std::sync::atomic::Ordering::Relaxed) {
            println!(
                "HIT t={:.1} {} at ({:.1},{:.1}) evade {:.0} tackle {:.0} head_on {:.2} closing {:.1} lunge {:.2} arm {} p_miss {:.2}",
                t, d.label, r.pos.x, r.pos.y, evade, tackle, head_on, closing, lunge, arm_only, p_miss
            );
        }
        if self.rng.chance(p_miss) {
            self.notes.missed.push((d.id, r.id));
            self.agents[i].down_until = t + 0.4 + 0.6 * self.rng.next();
            // A dive at his legs from the side or behind can still trip
            // him up or knock him off stride.
            // Balance keeps him on stride.
            let trip =
                if head_on < 0.3 { 0.72 } else { 0.88 } + clamp(0.002 * (r.a(Attr::Balance) - 60.0), -0.08, 0.08);
            self.agents[c].vel = self.agents[c].vel.scale(trip.min(0.96));
            return;
        }
        if self.notes.first_contact.is_none() {
            self.notes.first_contact = Some(self.agents[c].pos.x);
        }
        // A blocked man's arm usually only grabs at a runner going full
        // speed: it slows him, but he runs through it.
        // His balance against the tackler's will to finish the hit.
        let through = 0.6
            * clamp(1.0 + 0.008 * (r.a(Attr::Balance) - 60.0), 0.75, 1.25)
            * clamp(1.0 - 0.025 * (self.trait20(i, Attr::Physicality) - 10.0), 0.8, 1.2);
        if arm_only && r.vel.len() > 3.0 && self.rng.chance(through.min(0.9)) {
            self.agents[c].vel = self.agents[c].vel.scale(0.6);
            return;
        }
        self.fight.push(i);
        self.fight_since = t;
        self.hit_effects(c, i, 1.0);
    }

    /// Contact: momentum exchange, fumble and injury checks.
    fn hit_effects(&mut self, c: usize, i: usize, share: f64) {
        let (r, d) = (self.agents[c].clone(), self.agents[i].clone());
        // Momentum along the runner's line: m*v of both.
        let along = if r.vel.len() > 0.1 { r.vel.norm() } else { v2(self.goal, 0.0) };
        let pr = r.mass * r.vel.dot(along);
        let pd = d.mass * d.vel.dot(along);
        let v = (pr + pd * share) / (r.mass + d.mass * share);
        self.agents[c].vel = along.scale(v.max(-1.0));
        let force = (r.mass * r.vel.len() + d.mass * d.vel.len()) * 0.5;
        self.contact_force = self.contact_force.max(force);
        // Ball security against the hit (topic 2). Nearly doubled: fumbles
        // lost ran 0.2 a team game in the NFL against 0.45 real (proposed,
        // not confirmed).
        let carry = r.a(Attr::BallSecurity);
        let hit = d.a(Attr::Strength) * 0.5 + d.a(Attr::Tackling) * 0.5;
        let rain = (if self.cond.rain || self.cond.snow { 1.35 } else { 1.0 }) * (1.0 + 0.3 * self.cond.cold_unused);
        let p =
            0.016 * (force / 1700.0).powf(1.5) * (1.0 + (60.0 - carry) * 0.035).max(0.25) * (0.7 + hit / 160.0) * rain;
        if self.notes.fumbled.is_none() && self.rng.chance(p) {
            self.notes.fumbled = Some(r.id);
            self.notes.forced_fumble = Some(d.id);
            self.fumble(c);
            return;
        }
        self.injury_check(c, force, 1.0);
        self.injury_check(i, force, 0.7);
    }

    fn fight_tick(&mut self, c: usize) {
        let t = self.t;
        // Taking hits wears a runner down.
        self.agents[c].work += DT * 30.0;
        let r = self.agents[c].clone();
        // Pad level and leg drive keep him going; balance keeps him up.
        let resist = 1.0
            + 0.003 * (r.a(Attr::Strength) - 60.0)
            + 0.005 * (r.a(Attr::LegDrive) - 60.0)
            + 0.003 * (r.a(Attr::Balance) - 60.0)
            + 0.002 * (r.a(Attr::Elusiveness) - 60.0)
            + 0.006 * (self.trait20(c, Attr::Competitiveness) - 10.0)
            + (r.mass - 215.0) / 300.0;
        let mut hazard = 0.0;
        let mut hold = 0.0;
        for &i in &self.fight {
            let d = &self.agents[i];
            let f = 1.0
                + 0.03 * (d.a(Attr::Tackling) - 60.0)
                + 0.012 * (d.a(Attr::Strength) - 60.0)
                + 0.01 * (self.trait20(i, Attr::Competitiveness) - 10.0)
                + (d.mass - 215.0) / 300.0;
            hazard += 3.1 * f.max(0.3);
            hold += d.mass * f.max(0.3);
        }
        hazard /= resist.max(0.4);
        let n = self.fight.len();
        // One man alone may lose his grip: a broken tackle.
        if n == 1 && t - self.fight_since > 0.15 {
            let i = self.fight[0];
            let d = &self.agents[i];
            let tk = 0.6 * d.a(Attr::Tackling) + 0.4 * d.a(Attr::Strength) + (d.mass - 215.0) * 0.08;
            let rb = 0.5 * r.a(Attr::Strength) + 0.5 * r.a(Attr::Elusiveness) + (r.mass - 215.0) * 0.08;
            let p_break = 0.026 * (1.0 + (rb - tk) / 60.0).max(0.05);
            if self.rng.chance(p_break) {
                if TRACE.load(std::sync::atomic::Ordering::Relaxed) {
                    println!("BREAK t={t:.1} from {} p {p_break:.3}", d.label);
                }
                let id = d.id;
                self.notes.missed.push((id, r.id));
                self.agents[i].down_until = t + 0.5;
                self.fight.clear();
                return;
            }
        }
        if self.rng.chance(1.0 - (-hazard * DT).exp()) {
            self.tackled(c);
            return;
        }
        // The pile moves: legs churning against the weight on him.
        let drive = (r.mass * (2.2 + 0.03 * (r.a(Attr::Strength) - 60.0)) - 0.35 * hold) / (r.mass + 0.6 * hold);
        let v = self.agents[c].vel.x * self.goal;
        let nv = (0.6 * v + 0.4 * drive * 2.0).clamp(-1.5, 4.0);
        self.agents[c].vel = v2(nv * self.goal, self.agents[c].vel.y * 0.5);
        for &i in &self.fight.clone() {
            let p = self.agents[c].pos;
            self.agents[i].pos = p + (self.agents[i].pos - p).norm().scale(0.7);
            self.agents[i].vel = self.agents[c].vel;
        }
    }

    pub fn tackled(&mut self, c: usize) {
        self.notes.tacklers = self.fight.iter().map(|&i| self.agents[i].id).collect();
        let x = self.agents[c].pos.x;
        if (x - self.progress) * self.goal > 0.0 {
            self.progress = x;
        }
        // Face masks and late hits ride on the tackle. Unnecessary roughness
        // stands for every 15-yard personal foul after the whistle (late hits,
        // hits on a defenceless man, horse collars, taunting): real NFL
        // about 0.47 a team game with them, face masks 0.18 (Claude's
        // reading of 2023, proposed, not confirmed; they were 0 and 0.09).
        if let Some(&i) = self.fight.first() {
            // A face mask is technique (discipline); the late hit after
            // the whistle is temper (hidden temperament).
            let disc = self.trait20(i, Attr::Discipline);
            let technique = (1.6 - disc / 12.0).max(0.2) * 0.6;
            let temper = (1.6 - self.player(i).traits.temperament / 12.0).max(0.2) * 0.6;
            let (id, side) = (self.agents[i].id, self.agents[i].side);
            if self.rng.chance(0.0087 * self.crew.personal * technique) {
                self.notes.fouls.push(Foul { kind: FoulKind::FaceMask, side, player: id, at: x });
            } else if self.rng.chance(0.023 * self.crew.personal * temper) {
                self.notes.fouls.push(Foul { kind: FoulKind::UnnecessaryRoughness, side, player: id, at: x });
            }
        }
        self.dead = true;
        self.ball = Ball::Dead(v2(self.progress, self.agents[c].pos.y));
    }

    /// The ball is out: a scramble for it near where it fell.
    pub fn fumble(&mut self, c: usize) {
        let p = self.agents[c].pos;
        let bounce = v2(self.rng.range(-3.0, 3.0), self.rng.range(-3.0, 3.0));
        let spot = p + bounce;
        // Who gets there first: closeness, awareness and luck.
        let mut wsum = [0.0, 0.0];
        let mut best = [(0.0, 0usize); 2];
        for (i, a) in self.agents.iter().enumerate() {
            if !a.up(self.t) && i != c {
                continue;
            }
            let d = a.pos.dist(spot);
            let w = (-d / 1.8).exp() * (0.6 + (a.a(Attr::Anticipation) + a.a(Attr::Competitiveness)) / 400.0);
            wsum[a.side] += w;
            if w > best[a.side].0 {
                best[a.side] = (w, i);
            }
        }
        let side = if self.rng.chance(wsum[0] / (wsum[0] + wsum[1]).max(1e-9)) { 0 } else { 1 };
        let who = best[side].1;
        self.notes.recovered = Some((side, self.agents[who].id));
        let spot = v2(spot.x.clamp(-9.9, 109.9), spot.y.clamp(0.1, WIDTH - 0.1));
        self.dead = true;
        self.ball = Ball::Dead(spot);
        self.progress = if side == self.agents[c].side { p.x } else { spot.x };
    }

    /// Injury risk from a hit (topic 6): force, fatigue, his proneness and
    /// the turf.
    pub fn injury_check(&mut self, i: usize, force: f64, share: f64) {
        let gp = self.player(i);
        let gp_recovery = gp.traits.recovery;
        let prone = 0.55 + gp.injury_proneness / 18.0;
        let tough = gp.trait20(Attr::Toughness, self.level);
        let turf = if self.cond.turf { 1.12 } else { 1.0 };
        let p = self.injury_scale * 0.0042 * share * (force / 1700.0).powf(1.6) * prone * turf;
        if !self.rng.chance(p) {
            return;
        }
        let kind = self.rng.weighted(&[
            (InjuryKind::Bruise, 22.0),
            (InjuryKind::AnkleSprain, 13.0),
            (InjuryKind::HighAnkleSprain, 5.0),
            (InjuryKind::Knee, 10.0),
            (InjuryKind::Acl, 2.2),
            (InjuryKind::Shoulder, 9.0),
            (InjuryKind::Concussion, 8.0),
            (InjuryKind::Ribs, 5.0),
            (InjuryKind::Hand, 6.0),
            (InjuryKind::Fracture, 3.0),
            (InjuryKind::Back, 3.0),
            (InjuryKind::Hamstring, 6.0),
        ]);
        // Some bodies heal faster (hidden recovery; proposed, not confirmed).
        let weeks = injury_weeks(kind, self.rng) * (1.3 - 0.03 * gp_recovery);
        // Playing through it is a choice (topic 6): toughness against how bad it is.
        let plays_on = kind != InjuryKind::Concussion
            && kind != InjuryKind::Acl
            && kind != InjuryKind::Fracture
            && self.rng.chance(clamp(0.15 + 0.035 * tough - 0.12 * weeks, 0.0, 0.95));
        let (side, p, id) = (self.agents[i].side, self.agents[i].p, self.agents[i].id);
        self.notes.injuries.push(Injury { side, p, id, kind, weeks, plays_on });
    }

    /// Debug tooling: print the play as it runs.
    pub fn trace(&self) {
        if !TRACE.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        eprintln!("{}", self.snapshot());
    }

    /// Every player's spot, speed and job (debug traces).
    pub fn snapshot(&self) -> String {
        let mut line = format!("t={:.1} ball={:?}", self.t, self.ball_pos());
        for (i, a) in self.agents.iter().enumerate() {
            let job = format!("{:?}", a.job);
            let job: String = job.chars().take(14).collect();
            line.push_str(&format!(
                "\n  {:>2} s{} {:<3} ({:5.1},{:5.1}) v{:4.1} {}{}{}",
                i,
                a.side,
                a.label,
                a.pos.x,
                a.pos.y,
                a.vel.len(),
                job,
                if let Some(k) = a.engaged { format!(" ENG{}w{:.2}", k, self.eng[k].w) } else { String::new() },
                if !a.up(self.t) { " DOWN" } else { "" }
            ));
        }
        line
    }

    pub fn record_frame(&mut self) {
        let ball = self.ball_pos();
        if let Some(fr) = &mut self.record {
            fr.push(Frame {
                t: self.t,
                ball,
                players: self.agents.iter().map(|a| (a.id, a.pos.x as f32, a.pos.y as f32)).collect(),
            });
        }
    }

    /// Ball carrier out of bounds, in the end zone, or down.
    pub fn check_carrier(&mut self, c: usize) -> bool {
        let a = &self.agents[c];
        let p = a.pos;
        if p.y <= 0.0 || p.y >= WIDTH {
            self.out_of_bounds = true;
            self.dead = true;
            let x = if (p.x - self.progress) * self.goal > 0.0 { p.x } else { self.progress };
            self.progress = x;
            self.ball = Ball::Dead(v2(x, p.y.clamp(0.0, WIDTH)));
            return true;
        }
        let goal_line = if self.goal > 0.0 { 100.0 } else { 0.0 };
        if (p.x - goal_line) * self.goal >= 0.0 {
            self.progress = goal_line;
            self.dead = true;
            self.ball = Ball::Dead(p);
            return true;
        }
        if p.x > 110.0 || p.x < -10.0 {
            self.dead = true;
            self.out_of_bounds = true;
            self.ball = Ball::Dead(p);
            return true;
        }
        if self.down_on_fall && self.t < a.down_until {
            self.dead = true;
            self.ball = Ball::Dead(p);
            return true;
        }
        false
    }

    pub fn mid() -> f64 {
        MID
    }
}

/// Weeks out by injury, from real return-to-play ranges (Claude's picks,
/// see docs/decisions.md).
pub fn injury_weeks(kind: InjuryKind, rng: &mut Rng) -> f64 {
    let (lo, hi) = match kind {
        InjuryKind::Bruise => (0.0, 1.0),
        InjuryKind::AnkleSprain => (0.0, 3.0),
        InjuryKind::HighAnkleSprain => (2.0, 7.0),
        InjuryKind::Hamstring => (1.0, 5.0),
        InjuryKind::Knee => (1.0, 6.0),
        InjuryKind::Acl => (36.0, 56.0),
        InjuryKind::Shoulder => (0.0, 8.0),
        InjuryKind::Concussion => (0.5, 3.0),
        InjuryKind::Ribs => (0.0, 4.0),
        InjuryKind::Hand => (0.0, 5.0),
        InjuryKind::Fracture => (4.0, 12.0),
        InjuryKind::Back => (0.0, 4.0),
    };
    let u = rng.next();
    lo + (hi - lo) * u * u
}
