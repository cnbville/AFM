//! The tracked play simulation (topics 1 and 2). Every player is an agent
//! with a position and velocity updated ten times a second. Movement is
//! physics (top speed, acceleration, cutting), each player only reacts to
//! what he can see, blocks are running battles, and tackles are fights
//! that play out step by step.

use super::geom::{DT, V2, back_line, end_line, goal, mid, v2, width};
use super::roster::GamePlayer;
use crate::attributes::{ATTRIBUTE_COUNT, Attr};
use crate::player::Level;
use crate::rng::{Rng, clamp};
use crate::tendencies::Tend;

/// Players on the field at most, for the compact per-tick scans.
const MAX_AGENTS: usize = 32;

/// Each attribute's fatigue weight in `Attr::ALL` order: the same numbers
/// as `Attr::profile().fatigue`, copied once when the game is built, so
/// `Agent::new` reads them straight from one list (world speed lane).
const FATIGUE: [f64; ATTRIBUTE_COUNT] = {
    let mut f = [0.0; ATTRIBUTE_COUNT];
    let mut i = 0;
    while i < ATTRIBUTE_COUNT {
        f[i] = crate::attributes::ATTRIBUTE_PROFILES[Attr::ALL[i] as usize].fatigue;
        i += 1;
    }
    f
};

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

/// The vision cone's test `c.acos() < half` (`Sim::sees`), the same answer
/// for every `c` from -1 to 1 (both callers clamp it there; past 1 the
/// upper bound below would not hold) or NaN, and every `half`, with the arc
/// cosine worked out only when the angle is within a hair of `half` (world
/// speed lane). For 0 <= c <= 1, asin(c) = c + c^3/6 + sum a_n c^(2n+1)
/// with every a_n > 0 and a_n at most 3/40 from n = 2 on, so acos(c) is at
/// most pi/2 - c - c^3/6 and, for c under 1, at least that less the tail
/// bound 0.08 c^5/(1 - c^2); and acos(c) > pi/2 for c < 0. The 1e-9 margins are
/// millions of times the rounding in these few steps (under 1e-15) and the
/// library arc cosine's error (about an ulp, 2e-16),
/// so wherever a bound decides, `c.acos() < half` decides the same. A NaN
/// `c` gives false either way.
#[inline]
fn acos_lt(c: f64, half: f64) -> bool {
    if c >= 0.0 {
        let c2 = c * c;
        let up = std::f64::consts::FRAC_PI_2 - (c + c2 * c * (1.0 / 6.0));
        if up < half - 1e-9 {
            return true;
        }
        if c < 0.9 && up - 0.08 * c2 * c2 * c / (1.0 - c2) > half + 1e-9 {
            return false;
        }
    } else if half < 1.57 {
        return false;
    }
    c.acos() < half
}

/// Weather and setting for one game (topic 4).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
        #[serde(deserialize_with = "crate::save::leaked")]
        name: crate::save::Text,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Agent {
    /// 0: the team snapping or kicking, 1: the other team.
    pub side: usize,
    /// Index into that team's players.
    pub p: usize,
    pub id: u32,
    #[serde(deserialize_with = "crate::save::leaked")]
    pub label: crate::save::Text,
    pub pos: V2,
    pub vel: V2,
    pub face: V2,
    /// His velocity with its length (`Agent::vel_len`), as last worked out:
    /// the same velocity has the same length, so it is not worked out again
    /// (world speed lane). Not saved.
    #[serde(skip)]
    pub vel_memo: (V2, f64),
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
    /// A back on the force: the run came outside him on his side (bug 253).
    #[serde(default)]
    pub force: bool,
    /// A combination block (fix list S6.1): the linebacker he climbs to
    /// once the down lineman is controlled.
    pub climb: Option<usize>,
    /// Ratings with fatigue, injury and momentum in.
    #[serde(with = "serde_big_array::BigArray")]
    pub r: [f64; ATTRIBUTE_COUNT],
    /// How tired he is coming into the snap, 0-1 (old habits come back).
    pub tired: f64,
    /// A pass rusher who beat him, and when he has his feet back under
    /// him: he can't square that man up again before then.
    pub beat_by: Option<usize>,
    pub recover_at: f64,
    /// His top speed before the situation (Dynamic Links: Burner adds to
    /// it on deep routes, breakaways, returns and pursuit).
    #[serde(default)]
    pub vmax0: f64,
    /// Burner's top-speed gain when it applies (a share of top speed) and
    /// Bend's tighter turn at speed (0-1 of a full link).
    #[serde(default)]
    pub burner: f64,
    #[serde(default)]
    pub bend: f64,
    /// The day's talks on him (dialogue D4-D6): trait points of focus
    /// (discipline, concentration) and of composure, and heat (late hits).
    #[serde(default)]
    pub focus20: f64,
    #[serde(default)]
    pub poise20: f64,
    #[serde(default)]
    pub heat: f64,
    /// A safety's hitter read this snap (S-A): his weight over his level's
    /// typical safety in tens of pounds, with his hitting ratings over his
    /// range ones; 0 for everyone else.
    #[serde(default)]
    pub hit: f64,
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
        for (i, v) in gp.attrs.values().enumerate() {
            let fat = 1.0 - FATIGUE[i] * tired;
            r[i] = clamp(v * fat + edge, 1.0, 99.0);
        }
        let vmax = gp.top_speed() * (1.0 - 0.08 * tired);
        use crate::links::Link;
        // Sudden (Dynamic Links): changing direction without slowing, five
        // agility points' worth of cut (0.004 a point above 60).
        let cut = gp.cut() + 0.02 * gp.links.share(&[Link::Sudden]);
        Agent {
            side,
            p,
            id: gp.id,
            label,
            pos,
            vel: V2::default(),
            face: v2(if side == 0 { 1.0 } else { -1.0 }, 0.0),
            vel_memo: (V2::default(), 0.0),
            vmax,
            accel: gp.accel() * (1.0 - 0.1 * tired),
            tau: gp.sprint_tau() * (1.0 + 0.1 * tired),
            cut,
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
            force: false,
            climb: None,
            r,
            tired,
            beat_by: None,
            recover_at: 0.0,
            vmax0: vmax,
            burner: BURNER * gp.links.share(&[Link::Burner]),
            bend: gp.links.share(&[Link::Bend]),
            focus20: 0.0,
            poise20: 0.0,
            heat: 0.0,
            hit: 0.0,
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
            vel_memo: self.vel_memo,
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
            climb: self.climb,
            force: self.force,
            r: self.r,
            tired: self.tired,
            beat_by: self.beat_by,
            recover_at: self.recover_at,
            vmax0: self.vmax0,
            burner: self.burner,
            bend: self.bend,
            focus20: self.focus20,
            poise20: self.poise20,
            heat: self.heat,
            hit: self.hit,
        }
    }

    pub fn brake(&self) -> f64 {
        self.accel * (1.2 + 0.7 * self.cut)
    }

    /// `vel.len()`: from the memo while the velocity is the one it was
    /// worked out for (equal coordinates give the same squares and root; a
    /// NaN never matches).
    #[inline]
    pub fn vel_len(&self) -> f64 {
        if self.vel == self.vel_memo.0 { self.vel_memo.1 } else { self.vel.len() }
    }

    /// Speed up, slow down and cut toward the wanted velocity, within
    /// his acceleration, sprint curve and agility.
    pub fn steer(&mut self, want: V2) {
        let a = self;
        let v = a.vel;
        let speed = a.vel_len();
        let mut want = want;
        // `want.len()` is the root of this sum. Well under his top speed
        // squared (vmax * vmax is rounded up half an ulp at most, and
        // 0.999_999 takes far more back off) the root is under his top speed
        // too, so the cap below can't act and the root isn't needed (world
        // speed lane).
        let wq = want.x * want.x + want.y * want.y;
        let slow = a.vmax > 1.0 && wq < a.vmax * a.vmax * 0.999_999;
        if !slow {
            let wl = wq.sqrt();
            if wl > a.vmax {
                want = want.scale(a.vmax / wl);
            }
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
        // Bend (Dynamic Links): a tight turn at speed, where a cut loses
        // the most.
        let t1 = speed / a.vmax;
        let turn = 1.0 + 0.25 * (1.0 - t1);
        // Bend's part, ((0.04 * bend) * speed) / vmax, is a zero for a man
        // without the link while t1 is finite (a finite speed over a top
        // speed that is neither zero nor NaN), and a zero added to `turn`
        // (never -0) leaves it as it is: worked out only with the link
        // (world speed lane).
        let turn = if a.bend == 0.0 && t1.is_finite() { turn } else { turn + 0.04 * a.bend * speed / a.vmax };
        let side = a.accel * a.cut * DT * turn;
        let perp_c = if pl > side { perp.scale(side / pl) } else { perp };
        a.vel = v + dir.scale(par_c) + perp_c;
        let s = a.vel.len();
        if s > a.vmax {
            a.vel = a.vel.scale(a.vmax / s);
        } else {
            // His speed as worked out, for the next look at it.
            a.vel_memo = (a.vel, s);
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
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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
    /// The defender's edge as `update_blocks` last worked it out, with the
    /// blocker, helper, move and edge rusher's arc it was for (it reads only
    /// ratings, links and habits that hold all play). Not saved.
    #[serde(skip)]
    pub edge_memo: Option<(usize, Option<usize>, Move, bool, f64)>,
}

/// A foul during the play.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Foul {
    pub kind: FoulKind,
    pub side: usize,
    pub player: u32,
    /// Where it happened (x), for spot fouls.
    pub at: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
    /// Taunting or an excessive celebration.
    UnsportsmanlikeConduct,
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
            FoulKind::UnsportsmanlikeConduct => "unsportsmanlike conduct",
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
            FoulKind::UnsportsmanlikeConduct => (15.0, false),
        }
    }
}

/// An injury on a play (topic 6).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
    // Minor bug 145: the body parts real injury reports list that the mix
    // had no kind for (they went to knees and ankles).
    Foot,
    Groin,
    Calf,
    Neck,
    Hip,
    Quad,
}

/// The strength staff's guard on soft-tissue injuries (minor bug 303): 2%
/// fewer muscle strains and backs for each point his strength head stands
/// over the level's typical one, 2% more for each point under, capped at
/// 15% either way, so the level's injury totals stay where they were.
/// Strength work cuts sports injuries to about a third (Lauersen 2014) and
/// Nordic hamstring work halves strains (van Dyk 2019); every college and
/// NFL staff lifts, so the gap between staffs is a small share of that.
/// Proposed, not confirmed: the 2% and the cap.
pub fn strength_guard(head: f64, typical: f64) -> f64 {
    clamp(1.0 - 0.02 * (head - typical), 0.85, 1.15)
}

/// An injury mix with its soft-tissue kinds times `guard` (minor bug 303),
/// and the factor on the injury's chance that keeps every other kind's
/// chance where it was.
pub fn guarded_mix<const N: usize>(mix: &[(InjuryKind, f64); N], guard: f64) -> ([(InjuryKind, f64); N], f64) {
    let total: f64 = mix.iter().map(|e| e.1).sum();
    let soft: f64 = mix.iter().filter(|e| e.0.soft_tissue()).map(|e| e.1).sum();
    let out = mix.map(|(k, wgt)| (k, if k.soft_tissue() { wgt * guard } else { wgt }));
    (out, (total + (guard - 1.0) * soft) / total)
}

/// The game injury mix (`Sim::injury_check`), set so the injuries that cost
/// a week or more fall by body part as real game-status reports do
/// (nflverse 2012-2025, `development_injuries.json`: knee 17.2%, ankle
/// 14.3%, hamstring 12.9%, concussion 11.1%, shoulder 5.6%, foot 4.4%,
/// groin 4.3%, calf 3.8%, back 2.6%, neck 2.3%, hip 2.0%, quad 1.9%, the
/// rest here as bruises, ribs, hands and broken bones). Knees and ankles
/// were 24.7% and 21.1% (minor bug 145); knee against ACL and ankle against
/// high ankle keep their old split.
pub const GAME_INJURIES: [(InjuryKind, f64); 18] = [
    (InjuryKind::Bruise, 23.2),
    (InjuryKind::AnkleSprain, 9.5),
    (InjuryKind::HighAnkleSprain, 3.6),
    (InjuryKind::Knee, 7.9),
    (InjuryKind::Acl, 1.7),
    (InjuryKind::Shoulder, 4.6),
    (InjuryKind::Concussion, 10.7),
    (InjuryKind::Ribs, 5.0),
    (InjuryKind::Hand, 6.1),
    (InjuryKind::Fracture, 3.1),
    (InjuryKind::Back, 2.8),
    (InjuryKind::Hamstring, 7.4),
    (InjuryKind::Foot, 3.1),
    (InjuryKind::Groin, 3.4),
    (InjuryKind::Calf, 3.0),
    (InjuryKind::Neck, 1.8),
    (InjuryKind::Hip, 1.7),
    (InjuryKind::Quad, 1.5),
];

impl InjuryKind {
    /// Muscle strains and backs: what the weight room and the strength
    /// staff's prevention work guard against (minor bug 303).
    pub fn soft_tissue(self) -> bool {
        matches!(
            self,
            InjuryKind::Hamstring | InjuryKind::Back | InjuryKind::Groin | InjuryKind::Calf | InjuryKind::Quad
        )
    }

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
            InjuryKind::Foot => "foot injury",
            InjuryKind::Groin => "groin strain",
            InjuryKind::Calf => "calf strain",
            InjuryKind::Neck => "neck injury",
            InjuryKind::Hip => "hip injury",
            InjuryKind::Quad => "quad strain",
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
            // The new kinds' costs are Claude's picks (proposed, not confirmed).
            InjuryKind::Foot => &[(Attr::Speed, 0.08), (Attr::Acceleration, 0.10), (Attr::Agility, 0.10)],
            InjuryKind::Groin => &[(Attr::Agility, 0.10), (Attr::Acceleration, 0.08), (Attr::KickPower, 0.10)],
            InjuryKind::Calf => &[(Attr::Speed, 0.08), (Attr::Acceleration, 0.10), (Attr::Jumping, 0.08)],
            InjuryKind::Neck => &[(Attr::Tackling, 0.08), (Attr::Strength, 0.06), (Attr::RunBlocking, 0.06)],
            InjuryKind::Hip => &[(Attr::Agility, 0.08), (Attr::Speed, 0.05), (Attr::Acceleration, 0.05)],
            InjuryKind::Quad => &[(Attr::Speed, 0.06), (Attr::Acceleration, 0.08), (Attr::KickPower, 0.10)],
            _ => &[(Attr::Speed, 0.03)],
        }
    }
}

/// A man playing hurt (update 3, F7): what his injury limits plays below
/// itself (`InjuryKind::hampers`: a hamstring costs speed and burst, a hand
/// catching and ball security), and his speed, burst and cut follow from
/// the hampered ratings. A healthy man is passed through untouched.
/// Rating points his body costs a make-a-man-miss check (update 3, A3):
/// elusiveness, agility and speed are rated for his size, so the check
/// reads his weight the way the forty does (`body::forty_weight`, 0.0124 s
/// a rating point; being lighter than 175 lb adds nothing more), against
/// the typical man for that check, so its league average stays put. A
/// 330 lb tackle rated 85 makes fewer men miss than a 200 lb back rated 85,
/// and more than a tackle rated 60.
pub(crate) fn size_cost(weight: f64, typical: f64) -> f64 {
    (crate::body::forty_weight(weight) - crate::body::forty_weight(typical)) / 0.0124
}

pub fn hampered(gp: &GamePlayer, hurt: Option<InjuryKind>) -> std::borrow::Cow<'_, GamePlayer> {
    match hurt {
        None => std::borrow::Cow::Borrowed(gp),
        Some(k) => {
            let mut h = gp.clone();
            for &(a, f) in k.hampers() {
                let v = h.attrs.get(a);
                h.attrs.set(a, v * (1.0 - f));
            }
            std::borrow::Cow::Owned(h)
        }
    }
}

/// What the sim records on a play, for stats, grades and the why.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Notes {
    pub pressures: Vec<(u32, f64)>,
    /// Each pass rusher's first contact with a blocker (id, seconds after
    /// the snap): what the Data section reads as his get-off.
    #[serde(default)]
    pub contact: Vec<(u32, f64)>,
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
    /// Every move a pass rusher tried, his counter too, on plays that keep
    /// frames only (film tapes): what the tape shows of his rush, won or
    /// not (minor bug 317).
    #[serde(skip)]
    pub rush_moves: Vec<(u32, Move)>,
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
    /// What the players' tendencies did this snap.
    pub tend: TendNotes,
    /// The line chart (lane 9), on your games' snaps only: Some from the
    /// snap when the game keeps one.
    #[serde(default)]
    pub chart: Option<Box<super::chart::LineChart>>,
}

/// What the players' tendencies did on a play (`docs/design/tendencies.md`),
/// for the stats each one is checked against.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TendNotes {
    /// The ball carrier on a designed run crossed the line this far from
    /// where the ball was snapped (yards across), this long after the snap.
    pub cross: Option<(f64, f64)>,
    /// A reach for the line to gain or the goal line: who, whether the ball
    /// got there, whether he fumbled it.
    pub reach: Option<(u32, bool, bool)>,
    /// The quarterback slid.
    pub slid: bool,
    /// The carrier stepped out of bounds with a tackler closing.
    pub stepped_out: bool,
    /// A kickoff landing in the end zone: did he bring it out?
    pub deep_kick: Option<bool>,
    /// A punt landing inside the 10: did he field it?
    pub fielded_inside_10: Option<bool>,
    /// A route jump: who, and whether he guessed right.
    pub jump: Option<(u32, bool)>,
    /// The lineman who batted the pass at the line.
    pub hands_up: Option<u32>,
    /// Where the receiving team took over after a kick (yards from its
    /// own goal line), and whether a punt was muffed.
    pub start: Option<f64>,
    pub muffed: bool,
}

/// What the game around a play does to habits (tendencies topic 3.7-3.8),
/// per side of the sim (0 the snapping or returning team).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Mood {
    /// Tendency points every gamble moves: up down two scores late, down
    /// protecting a lead.
    pub gamble: [f64; 2],
    /// How big the moment is, 0-1: a playoff game, a close game late.
    pub stakes: [f64; 2],
    /// Head coach strictness, 1-20: a strict coach's instructions are
    /// followed more often.
    pub strict: [f64; 2],
    /// The line to gain, in the sim's frame (None on returns).
    pub line: Option<f64>,
    /// Where the ball was snapped, across the field.
    pub snap_y: f64,
}

impl Default for Mood {
    fn default() -> Mood {
        Mood { gamble: [0.0; 2], stakes: [0.0; 2], strict: [10.0; 2], line: None, snap_y: mid() }
    }
}

impl Mood {
    /// The same game, seen from the other side.
    pub fn swap(self) -> Mood {
        Mood {
            gamble: [self.gamble[1], self.gamble[0]],
            stakes: [self.stakes[1], self.stakes[0]],
            strict: [self.strict[1], self.strict[0]],
            line: None,
            snap_y: self.snap_y,
        }
    }
}

/// One frame of tracking for the 2D and 3D views (topic 9.3): every player
/// and the ball. The poses, the ball's height and who holds it are view
/// output only (`pose.rs`); the engine never reads them.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Frame {
    pub t: f64,
    pub ball: V2,
    pub players: Vec<(u32, f32, f32)>,
    /// Each player's facing and action, in the order of `players`.
    #[serde(default)]
    pub poses: Vec<super::pose::Pose>,
    /// The ball's height above the turf, yards.
    #[serde(default)]
    pub ball_z: f32,
    #[serde(default)]
    pub ball_mode: super::pose::BallMode,
    /// Who holds the ball (an index into `players`), or `pose::NONE`.
    #[serde(default)]
    pub holder: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// On a designed run, the lane the back chose when he read the hole
    /// (across the field), once he has read it.
    pub lane: Option<f64>,
    /// Route jumpers who guessed wrong: who, the spot he broke for, and
    /// when.
    pub bites: Vec<(usize, V2, f64)>,
    /// The defense plays a two-gap front (a 3-4): its interior linemen
    /// read the block before they pick a gap (S9).
    pub two_gap: bool,
    /// The rusher the protection slides to (A8), and a back or tight end
    /// chipping him before his route, as agent indexes.
    pub help_on: Option<usize>,
    pub chip: Option<(usize, usize)>,
    /// On a designed run: Some(true) for a gap scheme (down blocks and a
    /// puller), Some(false) for a zone scheme, None otherwise.
    pub gap_run: Option<bool>,
    /// A stunt this snap: the penetrator and the looper (topic 1).
    pub stunt: Option<(usize, usize)>,
    /// The level being played: trait-like ratings are read against it.
    pub level: Level,
    /// What the game does to habits this snap (tendencies).
    pub mood: Mood,
    /// Each agent's tendencies this play (`Sim::tend`), worked out on first
    /// use: everything they read is fixed for the play. NaN until then.
    pub(crate) tend_memo: [[std::cell::Cell<f64>; crate::tendencies::TEND_COUNT]; MAX_AGENTS],
    /// Each agent's tackling reach from his arms and from Length
    /// (`Sim::reach_extra`), worked out on first use: they read only his
    /// player, the same all play. None until then.
    pub(crate) reach_memo: [std::cell::Cell<Option<(f64, f64, f64)>>; MAX_AGENTS],
}

/// A new play's empty tendency memo (`Sim::tend_memo`).
pub(crate) fn tend_memo() -> [[std::cell::Cell<f64>; crate::tendencies::TEND_COUNT]; MAX_AGENTS] {
    [const { [const { std::cell::Cell::new(f64::NAN) }; crate::tendencies::TEND_COUNT] }; MAX_AGENTS]
}

/// A new play's empty reach memo (`Sim::reach_memo`).
pub(crate) fn reach_memo() -> [std::cell::Cell<Option<(f64, f64, f64)>>; MAX_AGENTS] {
    [const { std::cell::Cell::new(None) }; MAX_AGENTS]
}

/// How much a run block's outcome leans on the strength gap per point.
/// Fitted (dynasty item 1, 2026-10-15) so FBS team-season yards per carry
/// spread like real FBS (offense and defense sd about 0.64, cfbfastR
/// 2022-24); 0.05 made run defense about 10% too decisive.
const RUN_EDGE: f64 = 0.035;
/// The extra pull a run defender on a block gets when the runner comes at
/// him (fitted with `RUN_EDGE`; 1.0, then 0.5 with leverage only). Minor bug
/// 164: any block not sealed, at least a full 2 a second, so a lineman on a
/// block makes the stop as the runner goes by him (real 43% of first
/// tackles at 2.5 yards, PFF roles, nflverse 2021-24; the engine's 30% at
/// 0.8); proposed, not confirmed. Minor bug 253: 0.9, with the run block
/// starting further ahead (`RUN_BLOCK_START`), so inside runs gain more
/// (NFL test rosters inside the tackles 3.08 to 3.64 a carry, real 4.05)
/// with linemen still making 40% of first tackles (real 43%; proposed, not
/// confirmed).
const SHED_RUNNER: f64 = 0.9;
/// Where a run block starts, w (minor bug 253; -0.4 before): the blocker
/// further ahead, so a lineman who makes the stop does it 2.3 yards on
/// (real 2.5; 2.0 before). On the NFL test rosters with the force fit
/// (`scrimmage.rs`, `FORCE_WIDE`) 4.52 a carry (real 4.50), 18.6% of runs
/// for no gain or a loss (real 18.2%), inside the tackles 3.64 and outside
/// 5.81 (real 4.05 and 5.06; 3.08 and 6.05 before), 3.39 against eight or
/// more in the box (real 3.40; proposed, not confirmed). Past -0.75 it
/// barely moves: -0.8 gave 4.44 and -0.7 4.40 with `SHED_RUNNER` at 1.
const RUN_BLOCK_START: f64 = -0.75;
/// How much of a ball carrier's own evade and break-tackle scores, over his
/// level's typical man at his position, his miss and broken-tackle checks
/// read (minor bug 334): with all of it a back's broken tackles a carry over
/// two 182-carry blocks carried r 0.44 from one block to the next on the
/// NFL test rosters, real 0.21 (nflverse 2018-25), since real broken
/// tackles are mostly the play's luck; at 0.45 r 0.27 (proposed, not
/// confirmed).
const CARRIER_SPREAD: f64 = 0.45;
/// The same pull for a second-level defender (1.5+ yards past the line, a
/// linebacker a blocker climbed to): he sees the runner coming from 3.5
/// yards and gets this much more (minor bug 164, linebackers 21% of first
/// tackles against 28% real; proposed, not confirmed).
const SHED_SECOND: f64 = 1.0;
const SHED_SECOND_RANGE: f64 = 3.5;
/// A run block's push fades: a man driven back goes back for 2 seconds at
/// most (1.6 yards when the block is won outright), and a defender driving
/// his blocker into the backfield does it for 1 second, while the fight
/// still slides toward the runner. It drove on at 1.6 × w yards a second
/// for as long as it lasted, so a tackle was pushed 11 yards downfield in a
/// trace (minor bug 164; real drive blocks move a man 1-3 yards; proposed,
/// not confirmed).
const RUN_PUSH_BACK: f64 = 2.0;
const RUN_PUSH_UP: f64 = 1.0;
/// Within this far of the line a man barely moving is in the phone booth
/// and is always hit; past it a linebacker who waits can still slip a
/// climbing blocker (it was anywhere, so linebackers were blocked to the
/// whistle 1.5 times a run; minor bug 164; proposed, not confirmed).
const PHONE_BOOTH: f64 = 1.5;

/// How far from the sideline a tackler can drive the runner out of bounds
/// rather than take him down in bounds (minor bug 141, was 6).
const PUSH_OUT: f64 = 8.0;

/// How far a free defender in the open field can reach or lunge for the
/// ball carrier on a return. (2.2 before, and 1.6 on returns: runners
/// slipped past squared-up defenders untouched, so nine in ten runs past 20
/// yards went past 40 against two in ten real, and long touchdowns ran near
/// twice real; proposed, not confirmed.)
pub const OPEN_REACH: f64 = 2.5;

/// The same reach on scrimmage plays: arm's length and a lunge step. At
/// 2.5 yards, a full dive, the nearest defender tackled a receiver at the
/// catch from up to three yards away, so NFL yards after the catch ran 4.3
/// a catch against the real 5.2 (nflverse 2021-2025), short at every
/// depth, and yards a carry 3.9 against 4.5 (minor bug 162; 6,000
/// `match_check` games: 4.8 and 4.4 at 2.0). Returns keep `OPEN_REACH`,
/// which minor bug 139's return fixes were measured on (proposed, not
/// confirmed).
pub const SCRIMMAGE_REACH: f64 = 2.0;

/// A block in space holds a defensive back only while the runner comes at
/// him: once the runner is past the line and this many yards to his side,
/// the back works off the block (a receiver's stalk is a mirror, not a
/// drive) at `SPACE_BLOCK_LEAVE` a tick and pursues. Corners stayed in
/// stalk blocks for three seconds and more while inside runs broke the
/// middle, so at the 20 every defender was behind the runner: 45% of NFL
/// designed runs past 20 went past 40 against 21% real (nflverse 2022-24),
/// and 40-64% of inside runs, while tosses and jet sweeps, run at the
/// sideline, were at real (minor bug 163; proposed, not confirmed).
const SPACE_BLOCK_GAP: f64 = 3.0;
/// The chance a tick that he works off it then (proposed, not confirmed).
const SPACE_BLOCK_LEAVE: f64 = 0.6;

/// How far along the runner's path the last man in front aims, as a share
/// of the meeting point: a deeper angle keeps the runner in front of him
/// and inside, so the pursuit catches up (1.1 before; with
/// `SPACE_BLOCK_GAP`, 6,000 `match_check` games: runs of 40+ 0.9% to 0.5%
/// of designed runs, real 0.48%; return touchdowns 0.039 to 0.023 a team
/// game, real 0.024; minor bug 163, proposed, not confirmed).
const LAST_MAN_LEAD: f64 = 1.4;

/// An edge rusher's get-off: the share of it from raw Acceleration (the
/// rest of the 0.6 reads Finesse Rush and Flexibility about the typical
/// edge's, Flexibility's part `EDGE_GETOFF_BEND`), and the share of his
/// speed rush's Speed and Acceleration kept (the rest reads Finesse Rush,
/// Flexibility and Hand Use, Flexibility's part `EDGE_ARC_BEND`). Minor bug
/// 286: they were 0.6 and 1.0, then 0.15 and 0.5 (PR #249), and the 10-yard
/// split was still the edge's strongest body number (+1.68 pts of pressures
/// a defense snap a 0.1 s), where real shows none once talent is known
/// (-0.49, band -0.99 to +0.01). A slow-burst edge also rushes on the
/// typical edge's burst, a quick one on his own (`scrimmage.rs`,
/// `rush_feet`): about +0.9 (pure, 4,000 games). Reading every edge's feet
/// as the typical edge's took it to +0.3 but failed Cain's freak check
/// (light edges out of the top seasons), so his weight and turn stay his
/// own. The edge lane, 2026-11-08; proposed, not confirmed.
const EDGE_BURST: f64 = 0.0;
const EDGE_ARC_LEGS: f64 = 0.25;
/// E1 (Cain's yes 2026-11-08): bend mostly finishes the rush, so the edge's
/// get-off and arc read less Flexibility (0.5 and 0.3 of their craft
/// before) and his speed move 0.05 of it (0.15 before), the rest as
/// Finesse Rush; with the lane's other reads the 3-cone adds +0.31 pts of
/// pressures a 0.1 s (main +0.45, real band -0.06 to +0.11) and its sacks
/// come from the bend reach in `sack_check` (proposed, not confirmed).
const EDGE_GETOFF_BEND: f64 = 0.2;
const EDGE_ARC_BEND: f64 = 0.1;
const EDGE_SPEED_BEND: f64 = 0.05;
/// E1: speed-move points an inch over the typical edge's height a
/// Flexibility point over the typical edge's (real: 0.1 s of 3-cone for his
/// weight is worth +0.11 pts more pressures for each 1.4 in; proposed, not
/// confirmed).
const EDGE_TALL_BEND: f64 = 0.2;
/// E3 (Cain's yes 2026-11-08): power-move points a Strength point over the
/// typical edge's, a 10 lb over the typical edge's weight (real: 5 more
/// bench reps add +0.48 pts of pressures for heavy edges, -0.12 for light
/// ones), counted up to `EDGE_MASS_SPAN` over it, about the real heavy
/// edge's weight. Past it the term stops growing (his strength and mass
/// still count in full in the power move): taken on with weight, 295 lb
/// men at edge reached 34 sacks a season. Proposed, not confirmed.
const EDGE_STRENGTH_MASS: f64 = 0.3;
const EDGE_MASS_SPAN: f64 = 20.0;
/// Bug 328: Agility's share of an edge's finesse move (0.25 before; the
/// rest reads Finesse Rush and Hand Use about the typical edge's). With his
/// turn on the rush also read about the typical edge's the shuttle came to
/// about +0.2 pts of pressures a 0.1 s (real +0.17, band +0.02 to +0.32; it
/// was +0.52), but light, quick edges then fell out of the top seasons
/// (Cain's freak check), so his turn stays his own and the shuttle is about
/// +0.5 again (proposed, not confirmed).
const EDGE_FINESSE_AGILITY: f64 = 0.12;
/// E2 (Cain's yes 2026-11-08): run-tackle reach, yards an inch of arm over
/// the typical edge's, for a quick edge (real: +0.55 to +1.0 pts of run
/// stops a run an inch for quick edges, flat for slow ones, a loose
/// combination fit). At 0.3 every edge's arms added +0.27 pts of run
/// tackles a defense snap an inch (main +0.10, real +0.08, band -0.04 to
/// +0.21), so half of it (proposed, not confirmed).
const EDGE_CLOSE_REACH: f64 = 0.15;

/// A defensive tackle's first step on a pass rush, over the 0.64
/// Acceleration points a pound his 10-yard split shows: the heavy man plays
/// two gaps and reads the run before he rushes, so he gets off later. His
/// power rush still reads his mass like every rusher's, so he wins more bull
/// rushes but gets home less (minor bug 285: real heavier tackles pressure
/// 0.22 pts less a defense snap per 10 lb at the same draft slot; with this
/// the engine gives -0.18 pure and -0.24 as recruited, and +0.18 pts of
/// power wins a rush snap). Proposed, not confirmed.
const TWO_GAP: f64 = 2.7;

/// The chance a linebacker past the line of scrimmage slips a climbing
/// blocker, added to the whiff (bug 305: linebackers made 21% of first
/// run tackles, real 28%; with the fills below 24.5%). Proposed, not
/// confirmed.
const LB_SLIP: f64 = 0.4;

/// The chance a tick a linebacker in a block works off it to a runner who
/// is past the line and going by to his side (bug 305, as `LB_SLIP`; he
/// was blocked to the whistle 1.7 times a run, now 1.0). Proposed, not
/// confirmed.
const LB_LEAVE: f64 = 0.8;

/// Tackling points a pound over the typical man at his position: real
/// heavier linebackers miss 0.53 pts fewer tackles per 10 lb and corners
/// 0.57 (engine -0.36 and -0.57); and per inch of a linebacker's arm
/// (real about -0.2 pts an inch, engine -0.12). Proposed, not confirmed.
const TACKLE_WEIGHT_LB: f64 = 0.28;
const TACKLE_ARM_LB: f64 = 1.2;
const TACKLE_WEIGHT_CB: f64 = 0.27;

/// The hitter's part in a forced fumble, a point of 0.5 Strength + 0.5
/// Tackling about his level's typical man at his position (minor bug 282:
/// at the carrier's 0.025 a safety's
/// Hitting swung forced fumbles 1.06 times real regulars' spread; proposed,
/// not confirmed).
const FUMBLE_HIT: f64 = 0.020;

/// Rating points a road line loses off the snap at full crowd noise.
/// Fitted with the other home-field causes to the real home margin
/// (`scrimmage.rs`, `NOISE_LOAD`; minor bug 143: 24 before; proposed, not
/// confirmed).
const SILENT_COUNT: f64 = 36.0;

/// Burner (Dynamic Links): a full link's top-speed gain where it applies,
/// as five speed points take about 0.047 s off a forty (1.25% of top
/// speed).
pub const BURNER: f64 = 0.0125;

/// Print plays as they run (debug tooling only).
pub static TRACE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Normal draw.
#[inline]
pub fn gauss(rng: &mut Rng) -> f64 {
    rng.normal(0.0, 1.0)
}

/// The separation scan's one-yard bands (`Sim::advance`), along the field
/// and across it, from `BAND_OFF` yards behind the zero line, with a spare
/// band at each end.
const BANDS_X: usize = 176;
const BANDS_Y: usize = 96;
const BAND_OFF: f64 = 24.0;

/// A coordinate's band: shifted by `BAND_OFF`, truncated and held to
/// 1..=n-2 (a NaN goes to band 1, and a NaN spot never pushes anyone). It
/// never falls as the coordinate grows, so spots two or more bands apart
/// are more than a yard apart (less rounding of about 1e-13), where the
/// scan's `d.dot(d) > 0.65` always skips them (world speed lane).
#[inline]
fn band(v: f64, n: usize) -> usize {
    ((v + BAND_OFF) as i64).clamp(1, n as i64 - 2) as usize
}

/// One tick of a player's own movement (`Sim::advance`): it reads and
/// changes only him.
#[inline]
fn step(a: &mut Agent, t: f64) {
    if t < a.down_until {
        a.vel = a.vel.scale(0.5);
    }
    a.pos = a.pos + a.vel.scale(DT);
    // His speed: from the memo when steer left it, and kept for the next
    // steer (the same number as vel.len(); world speed lane).
    let l = if a.vel == a.vel_memo.0 {
        a.vel_memo.1
    } else {
        let l = a.vel.len();
        a.vel_memo = (a.vel, l);
        l
    };
    if l > 1.0 && t >= a.face_lock {
        // vel.norm() with a length over 1: the same two divisions.
        a.face = v2(a.vel.x / l, a.vel.y / l);
    }
}

impl<'a> Sim<'a> {
    pub fn player(&self, i: usize) -> &GamePlayer {
        let a = &self.agents[i];
        &self.teams[a.side][a.p]
    }

    /// Agent `i`'s Dynamic Links edge in a situation the given links act
    /// in, in rating points (a full link is five; stacked by `Links::edge`).
    #[inline]
    pub fn link(&self, i: usize, links: &[crate::links::Link]) -> f64 {
        self.player(i).links.edge(links)
    }

    /// Agent `i`'s tendency this snap (tendencies topics 3.7, 3.8, 4.3): an
    /// instruction he follows sets it; otherwise his own number, slipping
    /// up to a third back toward a coached-down habit under stress (a big
    /// game, tired, low composure), plus the game situation on gambles.
    #[inline]
    pub fn tend(&self, i: usize, t: Tend) -> f64 {
        // Everything it reads (his instructions and whether he obeys them,
        // his numbers, composure and tiredness, the snap's stakes, gamble and
        // spot) is fixed for the play: worked out once and kept (NaN: not
        // yet), the same number every time (world speed lane).
        let Some(memo) = self.tend_memo.get(i) else { return self.tend_fresh(i, t) };
        let cell = &memo[t.index()];
        let v = cell.get();
        if !v.is_nan() {
            return v;
        }
        let v = self.tend_fresh(i, t);
        cell.set(v);
        v
    }

    /// `tend`, worked out.
    fn tend_fresh(&self, i: usize, t: Tend) -> f64 {
        let a = &self.agents[i];
        let gp = &self.teams[a.side][a.p];
        if let Some(v) = gp.orders.sets(t)
            && self.obeys(i)
        {
            return v;
        }
        let base = gp.tend.get(t);
        let was = gp.tend.was[t.index()] as f64;
        let poise = (gp.trait20(Attr::Composure, self.level) + a.poise20).clamp(1.0, 20.0);
        let stress = clamp(self.mood.stakes[a.side] + 0.8 * a.tired + 0.05 * (10.0 - poise), 0.0, 1.0);
        let gamble = if t.gamble() { self.mood.gamble[a.side] } else { 0.0 };
        clamp(base + stress * (was - base) / 3.0 + gamble, 1.0, 20.0)
    }

    /// His tendency as a lean, -1 at 1 to +1 at 20.
    #[inline]
    pub fn lean(&self, i: usize, t: Tend) -> f64 {
        (self.tend(i, t) - crate::tendencies::MID) / 9.5
    }

    /// Does he follow his instructions this snap? Decided once a play, from
    /// a key rather than a draw, so asking changes nothing else.
    pub fn obeys(&self, i: usize) -> bool {
        let a = &self.agents[i];
        let gp = &self.teams[a.side][a.p];
        let p = crate::tendencies::obey_chance(
            gp.trait20(Attr::Discipline, self.level),
            gp.personality.coachability,
            gp.trust,
            self.mood.strict[a.side],
        );
        let mut x = (a.id as u64) << 20 ^ ((self.los * 16.0) as i64 as u64) ^ 0x0BE1_D0E5;
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        ((x >> 11) as f64 / (1u64 << 53) as f64) < p
    }

    /// A trait-like rating of agent `i` on the 1-20 scale, against the level
    /// (see `GamePlayer::trait20`).
    #[inline]
    pub fn trait20(&self, i: usize, a: Attr) -> f64 {
        let base = self.player(i).trait20(a, self.level);
        // The day's talks (dialogue D4-D6): focus on discipline and
        // concentration, poise on composure.
        let ag = &self.agents[i];
        match a {
            Attr::Discipline | Attr::Concentration if ag.focus20 != 0.0 => (base + ag.focus20).clamp(1.0, 20.0),
            Attr::Composure if ag.poise20 != 0.0 => (base + ag.poise20).clamp(1.0, 20.0),
            _ => base,
        }
    }

    pub fn carrier(&self) -> Option<usize> {
        match self.ball {
            Ball::Held(i) => Some(i),
            _ => None,
        }
    }

    /// Burner (Dynamic Links) where it acts, each tick: the carrier past
    /// the line (breakaways and returns), a receiver on the deep part of
    /// his route or running under a long ball, and a defender chasing from
    /// behind (a beaten man's recovery and pursuit). Everywhere else he
    /// runs at his own top speed.
    pub fn link_speeds(&mut self) {
        let carrier = self.carrier();
        let goal = self.goal;
        let los = self.los;
        for i in 0..self.agents.len() {
            let a = &self.agents[i];
            if a.burner <= 0.0 {
                continue;
            }
            let down = (a.pos.x - los) * if a.side == 0 { goal } else { -goal };
            let on = match carrier {
                Some(c) if c == i => down > 2.0 || a.side == self.carrier_side && a.label == "KR",
                Some(c) if self.agents[c].side != a.side => {
                    // Behind the runner, chasing him toward the goal.
                    let cs = if self.agents[c].side == 0 { goal } else { -goal };
                    (self.agents[c].pos.x - a.pos.x) * cs > 1.0
                }
                _ => match &a.job {
                    Job::Route { .. } => down > 9.0,
                    Job::Man { target, .. } => {
                        let r = &self.agents[*target];
                        let ahead = (r.pos.x - a.pos.x) * if r.side == 0 { goal } else { -goal };
                        ahead > 0.5 && (r.pos.x - los).abs() > 9.0
                    }
                    _ => matches!(self.ball, Ball::Air { target: Some(tg), .. } if tg == i) && down > 9.0,
                },
            };
            let vm = a.vmax0 * if on { 1.0 + a.burner } else { 1.0 };
            self.agents[i].vmax = vm;
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
        // The cosine `angle_between` took the arc cosine of: the cone is
        // tested on it (`acos_lt`, the same answer as the angle against
        // `half`; world speed lane).
        let cos = a.face.norm().dot(d.norm()).clamp(-1.0, 1.0);
        let read = if matches!(a.job, Job::Passer) {
            Attr::PocketPresence
        } else if self.carrier() == Some(i) {
            Attr::BallCarrierVision
        } else {
            Attr::PlayRecognition
        };
        let half = 1.2 + 0.006 * (a.a(read) - 50.0);
        acos_lt(cos, half)
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
        let n = self.agents.len();
        // Moving players never moves the ball: the carrier is the same
        // before and after.
        let carrier = self.carrier();
        if n > MAX_AGENTS {
            for a in self.agents.iter_mut() {
                step(a, t);
            }
            self.separate_slow(t, carrier);
            return;
        }
        // Bodies don't pass through each other: free players slide around
        // engaged pairs and teammates. A compact copy, made in the pass that
        // moves each player (it reads only his own new spot and state, so it
        // is the same as a second pass): each side's obstacles (its own
        // players and anyone engaged) as bits, and each obstacle's bit in
        // his one-yard band along the field and across it (`band`). A mover
        // tests only the obstacles in the three bands around him each way,
        // lowest index first: the same men in the same order as his side's
        // whole list, less men two bands or more away, whom the test skipped
        // (clearly apart) without touching anything (world speed lane).
        let mut pos = [V2::default(); MAX_AGENTS];
        let mut mover = [false; MAX_AGENTS];
        let mut own = [0u32; 2];
        let mut bx = [0u32; BANDS_X];
        let mut by = [0u32; BANDS_Y];
        let mut cell = [(0u8, 0u8); MAX_AGENTS];
        for (j, a) in self.agents.iter_mut().enumerate() {
            step(a, t);
            pos[j] = a.pos;
            let engaged = a.engaged.is_some();
            mover[j] = !engaged && a.up(t) && Some(j) != carrier;
            if Some(j) == carrier {
                continue;
            }
            let bit = 1u32 << j;
            for (s, m) in own.iter_mut().enumerate() {
                if engaged || a.side == s {
                    *m |= bit;
                }
            }
            let (cx, cy) = (band(a.pos.x, BANDS_X), band(a.pos.y, BANDS_Y));
            cell[j] = (cx as u8, cy as u8);
            bx[cx] |= bit;
            by[cy] |= bit;
        }
        for i in 0..n {
            if !mover[i] {
                continue;
            }
            let side = self.agents[i].side;
            let pi = pos[i];
            let bit = 1u32 << i;
            let (cx, cy) = (cell[i].0 as usize, cell[i].1 as usize);
            let near = (bx[cx - 1] | bx[cx] | bx[cx + 1]) & (by[cy - 1] | by[cy] | by[cy + 1]);
            // His side's obstacles near him, himself left out.
            let mut m = own[side] & near & !bit;
            let mut push = V2::default();
            let mut moved = false;
            while m != 0 {
                let j = m.trailing_zeros() as usize;
                m &= m - 1;
                let d = pi - pos[j];
                // Clearly apart (well past 0.8 yards): no square root needed.
                if d.dot(d) > 0.65 {
                    continue;
                }
                let l = d.len();
                if l < 0.8 && l > 1e-6 {
                    push = push + d.scale((0.8 - l) / l * 0.5);
                    moved = true;
                }
            }
            pos[i] = pi + push.scale(self.agents[i].traffic());
            self.agents[i].pos = pos[i];
            if moved {
                // His bands follow him for the movers after him. Unpushed,
                // his spot keeps its value (or turns NaN, which pushes no
                // one), so his bands stay right.
                let (nx, ny) = (band(pos[i].x, BANDS_X), band(pos[i].y, BANDS_Y));
                bx[cx] &= !bit;
                by[cy] &= !bit;
                bx[nx] |= bit;
                by[ny] |= bit;
                cell[i] = (nx as u8, ny as u8);
            }
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
        if speed < 2.0 && (self.agents[d].pos.x - self.los) * self.goal < PHONE_BOOTH {
            return true;
        }
        let (ba, da) = (&self.agents[b], &self.agents[d]);
        // Getting hands on a man in space is its own skill (climbing to a
        // linebacker, a screen, a return wall).
        // Dynamic Links: Big man who moves gets to the second level and
        // pulls; a Sudden man slips the hands in space.
        use crate::links::Link;
        let skill = 0.5 * ba.a(Attr::BlockingInSpace) + 0.5 * ba.a(Attr::Agility) + self.link(b, &[Link::BigMover]);
        let slip = 0.5 * da.a(Attr::Agility) + 0.5 * da.a(Attr::BlockShedding) + self.link(d, &[Link::Sudden]);
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
            clamp(0.17 + 0.006 * (slip - skill) + 0.05 * (across - 2.0).max(0.0), 0.05, 0.55)
        };
        // A linebacker at the second level plays the climbing blocker with
        // his hands and leverage, and slips him more often.
        let p_miss = if da.label == "LB" && (da.pos.x - self.los) * self.goal > PHONE_BOOTH {
            (p_miss + LB_SLIP).min(0.7)
        } else {
            p_miss
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
        let (mv, picked) = if pass { self.pick_move(b, d) } else { (Move::Power, None) };
        if pass && self.record.is_some() {
            self.notes.rush_moves.push((self.agents[d].id, mv));
        }
        let (bl, de) = (&self.agents[b], &self.agents[d]);
        // Get-off against the set: the first step decides a lot. The
        // rusher's jump is his burst and his read of the snap; the blocker
        // knows the count.
        // Times the snap (tendencies): a man who guesses the count gets off
        // sooner on the snaps he guesses right (offside is the cost).
        // Dynamic Links: Get-off is the first step itself; Length lands the
        // first hands (on both sides, the longer man's punch lands first).
        // An edge rusher's get-off leans on his rush craft (Finesse Rush and
        // bend) more than raw burst (minor bug 286: real edges' 10-yard
        // split says nothing about their pressures once talent is known).
        // An interior rusher's first step reads his body too: Acceleration
        // as his 10-yard split shows it at his weight, a pound off his
        // level's typical man at his position worth 0.64 Acceleration points
        // (`links::ten_split`; minor bug 285, heavy tackles pressured more).
        // An edge's weight already costs him on his arc (his burst and cut,
        // `game/roster.rs`), and real heavy edges pressure no less.
        use crate::links::Link;
        let edge_rusher = pass && matches!(de.job, Job::Rush { contain, .. } if contain != 0.0);
        let k_acc = if edge_rusher { EDGE_BURST } else { 0.6 };
        let heft = if pass && !edge_rusher {
            let (_, wt, _) = super::synthetic::typical_body(self.level, self.player(d).position);
            let two_gap = if self.player(d).position == crate::positions::Position::DT { TWO_GAP } else { 0.0 };
            (0.64 + two_gap) * k_acc * (de.mass - wt)
        } else {
            0.0
        };
        // The share moved off raw burst reads his craft about his level's
        // typical man at his position, who gets off as before.
        let typ = |a: Attr| super::synthetic::typical(self.level, self.player(d).position, a);
        let craft = if k_acc < 0.6 {
            typ(Attr::Acceleration)
                + (1.0 - EDGE_GETOFF_BEND) * (de.a(Attr::FinesseRush) - typ(Attr::FinesseRush))
                + EDGE_GETOFF_BEND * (de.a(Attr::Flexibility) - typ(Attr::Flexibility))
        } else {
            0.0
        };
        let get_off = de.a(Attr::Acceleration) * k_acc + (0.6 - k_acc) * craft + de.a(Attr::Anticipation) * 0.4 - heft
            + 12.0 * self.lean(d, Tend::SnapTiming)
            + self.link(d, &[Link::GetOff])
            + self.link(d, &[Link::Length]);
        let set = if pass {
            bl.a(Attr::PassBlocking) * 0.5 + bl.a(Attr::Agility) * 0.3 + bl.a(Attr::FootballIq) * 0.2
        } else {
            bl.a(Attr::RunBlocking) * 0.5 + bl.a(Attr::Acceleration) * 0.3 + bl.a(Attr::Strength) * 0.2
        } + self.link(b, &[Link::GetOff])
            + self.link(b, &[Link::Length]);
        // Silent count (trial 4 topic 3): a road line in a loud stadium
        // snaps on a visual cue, a beat late, and the defense times it.
        // A focused lineman reacts to the snap cue sooner.
        let focus = clamp(1.0 - 0.05 * (self.trait20(b, Attr::Concentration) - 10.0), 0.5, 1.5);
        let silent = if bl.side == 0 && self.t < 1.0 { SILENT_COUNT * self.cond.crowd * focus } else { 0.0 };
        // A run block starts even, then the blocker's get-off against the
        // defender's read decides who has the first leverage. It started
        // with the blocker well ahead, -0.3, then the defender a little
        // ahead, 0.15 (trial 8 item 7), which bought the right yards a
        // carry with too many stuffs. With double teams holding the down
        // linemen (fix list S6.1) an even start gave the world's FBS
        // rosters 5.02 a carry in a check but 4.85 in the world (real
        // 5.12), and NFL rosters 3.99 (real 4.50). Trial 9 finding 2: the
        // blocker a little ahead, -0.1, with the defenders' read timed
        // against the level (scrimmage.rs), gives 5.24 and 4.32 on the
        // world's rosters, about 5.05 and 4.45 in the world (proposed, not
        // confirmed). Minor bug 164: with blocked linemen making the stop as
        // the runner goes by (`SHED_RUNNER`), the blocker starts further
        // ahead, -0.4: 4.38 a carry on the test rosters (real 4.50), 16.7%
        // of backs' runs stuffed (real 17.6%; proposed, not confirmed).
        // Minor bug 253: further still (`RUN_BLOCK_START`).
        let w0 = if pass { -0.42 } else { RUN_BLOCK_START }
            + 0.006 * (get_off + silent - set)
            + self.front_fit(de.label, pass)
            + 0.18 * gauss(self.rng);
        let k = self.eng.len();
        if pass {
            let id = self.agents[d].id;
            if !self.notes.contact.iter().any(|c| c.0 == id) {
                self.notes.contact.push((id, self.t));
            }
        }
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
            // The pick's edge is the number the first `update_blocks` tick
            // would work out: no helper yet, the same blocker, move and arc,
            // and nothing it reads changes in a play (world speed lane).
            edge_memo: picked.map(|v| (b, None, mv, edge_rusher, v)),
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

    /// The rusher's move, and that move's edge (`rush_power` less
    /// `block_power`, as `update_blocks` works it out) when one scored above
    /// minus infinity: the same draws and the same pick as before (world
    /// speed lane).
    fn pick_move(&mut self, b: usize, d: usize) -> (Move, Option<f64>) {
        let mut best = (Move::Speed, f64::NEG_INFINITY, None);
        for mv in [Move::Speed, Move::Power, Move::Finesse] {
            let edge = self.rush_power(d, mv) - self.block_power(b, mv, true);
            // Smart rushers pick the right move more often.
            let noise = (11.0 - 0.09 * self.agents[d].a(Attr::FootballIq)).max(2.0) * gauss(self.rng);
            if edge + noise > best.1 {
                best = (mv, edge + noise, Some(edge));
            }
        }
        (best.0, best.2)
    }

    /// Rusher strength for a move, on the rating scale. A speed rush is
    /// the finesse rusher's arc, won with burst and bend; a power rush is
    /// strength through the hands; a finesse move is hands and quickness.
    fn rush_power(&self, d: usize, mv: Move) -> f64 {
        let a = &self.agents[d];
        // The wide arc wins the corner more often; a man who stops to get
        // his hands up gives up some rush (tendencies).
        let edge = matches!(a.job, Job::Rush { contain, .. } if contain != 0.0);
        let arc = if edge { 10.0 * self.lean(d, Tend::Upfield) } else { 0.0 };
        let hands = 2.5 * self.lean(d, Tend::HandsUp);
        // Dynamic Links in the rush: Bend around the corner, Anchor's mass
        // behind the bull rush, Big man who moves on the quick moves inside.
        use crate::links::Link;
        let gift = match mv {
            Move::Speed => self.link(d, &[Link::Bend]),
            Move::Power => self.link(d, &[Link::Anchor, Link::BatteringRam]),
            Move::Finesse => self.link(d, &[Link::BigMover, Link::Sudden]),
        };
        arc - hands
            + gift
            + match mv {
                Move::Speed => {
                    // An edge's arc leans on the craft of it, a quarter of
                    // his raw speed and burst (minor bug 286, `EDGE_ARC_LEGS`).
                    // The craft reads about his level's typical man at his
                    // position, whose arc wins as before.
                    let k = if edge { EDGE_ARC_LEGS } else { 1.0 };
                    let freed = if edge {
                        let typ = |x: Attr| super::synthetic::typical(self.level, self.player(d).position, x);
                        0.35 * (1.0 - k)
                            * ((0.2 * typ(Attr::Speed) + 0.15 * typ(Attr::Acceleration)) / 0.35
                                + (0.75 - EDGE_ARC_BEND) * (a.a(Attr::FinesseRush) - typ(Attr::FinesseRush))
                                + EDGE_ARC_BEND * (a.a(Attr::Flexibility) - typ(Attr::Flexibility))
                                + 0.25 * (a.a(Attr::HandUse) - typ(Attr::HandUse)))
                    } else {
                        0.0
                    };
                    // E1: an edge's bend is mostly his finish (`sack_check`),
                    // so his speed move reads less of it, the rest as rush
                    // craft about the typical man's; and a tall man's bend
                    // counts more at the corner (his pads under the
                    // tackle's hands), above the typical edge's height only.
                    let (bend, tall) = if edge {
                        let g = self.player(d);
                        let typ = |x: Attr| super::synthetic::typical(self.level, g.position, x);
                        let (th, _, _) = super::synthetic::typical_body(self.level, g.position);
                        (
                            EDGE_SPEED_BEND * a.a(Attr::Flexibility)
                                + (0.15 - EDGE_SPEED_BEND)
                                    * (typ(Attr::Flexibility) + a.a(Attr::FinesseRush) - typ(Attr::FinesseRush)),
                            EDGE_TALL_BEND
                                * (g.height - th).max(0.0)
                                * (a.a(Attr::Flexibility) - typ(Attr::Flexibility)),
                        )
                    } else {
                        (0.15 * a.a(Attr::Flexibility), 0.0)
                    };
                    0.4 * a.a(Attr::FinesseRush)
                        + 0.2 * k * a.a(Attr::Speed)
                        + 0.15 * k * a.a(Attr::Acceleration)
                        + bend
                        + 0.1 * a.a(Attr::HandUse)
                        + freed
                        + tall
                }
                Move::Power => {
                    // E3: an edge's strength pays with his mass, above the
                    // typical edge's weight only; at or below it Strength
                    // keeps its full value, so a lean, strong man still
                    // wins with it.
                    let heft = if edge {
                        let (_, tw, _) = super::synthetic::typical_body(self.level, self.player(d).position);
                        let typ = super::synthetic::typical(self.level, self.player(d).position, Attr::Strength);
                        EDGE_STRENGTH_MASS * (a.mass - tw).clamp(0.0, EDGE_MASS_SPAN) / 10.0
                            * (a.a(Attr::Strength) - typ)
                    } else {
                        0.0
                    };
                    0.4 * a.a(Attr::PowerRush)
                        + 0.25 * a.a(Attr::Strength)
                        + 0.15 * a.a(Attr::HandUse)
                        + 0.1 * a.a(Attr::Balance)
                        + 0.1 * a.a(Attr::BlockShedding)
                        + (a.mass - 280.0) * 0.08
                        + heft
                }
                Move::Finesse => {
                    // Bug 328: an edge's Agility at the real size, the rest
                    // of its share read about the typical man's, as craft.
                    let ka = if edge { EDGE_FINESSE_AGILITY } else { 0.25 };
                    let freed = if ka < 0.25 {
                        let typ = |x: Attr| super::synthetic::typical(self.level, self.player(d).position, x);
                        (0.25 - ka)
                            * (typ(Attr::Agility)
                                + 0.6 * (a.a(Attr::FinesseRush) - typ(Attr::FinesseRush))
                                + 0.4 * (a.a(Attr::HandUse) - typ(Attr::HandUse)))
                    } else {
                        0.0
                    };
                    0.45 * a.a(Attr::FinesseRush)
                        + ka * a.a(Attr::Agility)
                        + 0.2 * a.a(Attr::HandUse)
                        + 0.1 * a.a(Attr::BlockShedding)
                        + freed
                }
            }
    }

    fn block_power(&self, b: usize, mv: Move, pass: bool) -> f64 {
        self.block_base(b, mv, pass) + self.block_gift(b, mv, pass)
    }

    /// Dynamic Links in a block: Big man who moves in pass sets against
    /// speed and in space, Anchor against power, Low center of gravity's
    /// leverage in the run game.
    fn block_gift(&self, b: usize, mv: Move, pass: bool) -> f64 {
        use crate::links::Link;
        if !pass {
            return self.link(b, &[Link::LowCenter, Link::Anchor]);
        }
        match mv {
            Move::Speed => self.link(b, &[Link::BigMover]),
            Move::Power => self.link(b, &[Link::Anchor]),
            Move::Finesse => self.link(b, &[Link::BigMover, Link::Length]),
        }
    }

    fn block_base(&self, b: usize, mv: Move, pass: bool) -> f64 {
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
        // A tackle's reach in the hand fight (minor bug 288, Cain's yes
        // 2026-11-07): a rating point an inch of arm over 34 against speed
        // and hands, so a 32-inch tackle loses a little to them.
        let gp = &self.teams[a.side][a.p];
        let reach = match mv {
            Move::Speed | Move::Finesse if gp.position == crate::positions::Position::OT => {
                // 34 inches in the NFL; the other levels' pivot moves with
                // their typical tackle's arm, so their typical man blocks as
                // before.
                let ot = |l| super::synthetic::typical_body(l, crate::positions::Position::OT).2;
                gp.arm - (34.0 + ot(self.level) - ot(Level::NFL))
            }
            _ => 0.0,
        };
        reach
            + match mv {
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
        // Anchor holds the double team; Big man who moves splits it.
        use crate::links::Link;
        0.6 * a.a(Attr::Anchor)
            + 0.2 * a.a(Attr::Strength)
            + 0.2 * a.a(Attr::Balance)
            + (a.mass - 300.0) * 0.08
            + self.link(d, &[Link::Anchor, Link::BigMover])
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
            // Edge of the defender in rating points. It reads only ratings,
            // links and habits that hold all play, so it is worked out again
            // only when the blocker, the helper, the move or an edge
            // rusher's arc changes: the same number otherwise (world speed
            // lane).
            let arc = e.pass && matches!(self.agents[d].job, Job::Rush { contain, .. } if contain != 0.0);
            let mut edge = match e.edge_memo {
                Some((mb, mh, mmv, ma, v)) if mb == b && mh == e.helper && mmv == e.mv && ma == arc => v,
                _ => {
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
                    self.eng[k].edge_memo = Some((b, e.helper, e.mv, arc, edge));
                    edge
                }
            };
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
                    let (mv, picked) = self.pick_move(b, d);
                    if mv != e.mv {
                        // The edge the pick worked out for it: the same
                        // number, as only draws came between (world speed
                        // lane).
                        let to = picked.unwrap_or_else(|| self.rush_power(d, mv) - self.block_power(b, mv, true));
                        if to > edge {
                            self.eng[k].mv = mv;
                            edge = to;
                            if self.record.is_some() {
                                self.notes.rush_moves.push((self.agents[d].id, mv));
                            }
                        }
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
            // A back in a block in space with the runner through the line
            // and going by to his side: he works off it to pursue.
            if !e.pass
                && !reading
                && let Some(c) = carrier
                && let leave = match self.agents[d].label {
                    "CB" | "NB" | "FS" | "SS" => SPACE_BLOCK_LEAVE,
                    "LB" => LB_LEAVE,
                    _ => 0.0,
                }
                && leave > 0.0
                && (self.agents[c].pos.x - self.los) * self.goal > 0.0
                && (self.agents[c].pos.y - self.agents[d].pos.y).abs() > SPACE_BLOCK_GAP
                && self.rng.chance(leave)
            {
                self.release(k);
                continue;
            }
            // The runner coming at him: a defender on a block that isn't
            // sealed fights off it to make the play (shed and tackle), the
            // harder the more leverage he has. A second-level man sees him
            // coming from farther and fights harder.
            // A sealed block (stored at exactly -1) is tested first: both
            // tests are pure, and it skips the root (world speed lane).
            let second = (self.agents[d].pos.x - self.los) * self.goal > 1.5;
            if !e.pass
                && !reading
                && let Some(c) = carrier
                && e.w > -1.0
                && self.agents[c].pos.dist(self.agents[d].pos) < if second { SHED_SECOND_RANGE } else { 2.2 }
            {
                if second {
                    mu += 2.0 * SHED_SECOND * (0.5 + e.w).max(0.2);
                }
                mu += 2.0 * SHED_RUNNER * (0.5 + e.w).max(1.0);
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
                let (bb, dd) = (&self.agents[b], &self.agents[d]);
                let tech = if e.pass { bb.a(Attr::PassBlocking) } else { bb.a(Attr::RunBlocking) };
                // How badly he is beaten, technique and hands against the
                // man's rush and hands: officials call each level to its own
                // standard, so it is read against the man in front of him, not
                // on an absolute scale (it was absolute: holding ran 0.71 a
                // team game in the NFL, 1.49 in FBS and 2.58 in high school,
                // real about 1.26 at each; proposed, not confirmed).
                let rush =
                    if e.pass { dd.a(Attr::PowerRush).max(dd.a(Attr::FinesseRush)) } else { dd.a(Attr::BlockShedding) };
                let beat = 0.5 * (rush + dd.a(Attr::HandUse)) - 0.5 * (tech + bb.a(Attr::HandUse));
                let p = 0.044 * (1.7 - disc / 14.0) * clamp(1.0 + 0.012 * beat, 0.5, 1.6);
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
                // The beaten blocker has lost his feet and his leverage:
                // he can't square the same man up again until he has
                // recovered and is back between him and the quarterback
                // (the edge kept, unless the blocker's recovery wins it
                // back; times proposed, not confirmed).
                if e.pass {
                    let bb = &self.agents[b];
                    let turned = if e.mv == Move::Speed { 0.15 } else { 0.0 };
                    let rec = clamp(
                        0.8 + turned - 0.010 * (bb.a(Attr::Agility) - 60.0) - 0.006 * (bb.a(Attr::PassBlocking) - 60.0),
                        0.35,
                        1.3,
                    );
                    self.agents[b].beat_by = Some(d);
                    self.agents[b].recover_at = t + rec;
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
            let v = if e.pass {
                dir.scale(drive.max(-2.0))
            } else {
                // The surge fades (`RUN_PUSH_BACK`, `RUN_PUSH_UP`): the push
                // along the field stops, the slide toward the runner goes on.
                let fade = |tt: f64| clamp(1.0 - (t - e.start) / tt, 0.0, 1.0);
                if w < 0.0 {
                    dir.scale((drive * fade(RUN_PUSH_BACK)).max(-2.0))
                } else {
                    v2(dir.x * drive * fade(RUN_PUSH_UP), dir.y * drive)
                }
            };
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

    /// Whether pass blocker `b` can take on rusher `d` (with the
    /// quarterback at `q`): a man who beat him must first be squared up
    /// from in front again.
    pub fn can_square(&self, b: usize, d: usize, q: V2) -> bool {
        let a = &self.agents[b];
        if a.beat_by != Some(d) {
            return true;
        }
        self.t >= a.recover_at && a.pos.dist(q) + 0.5 < self.agents[d].pos.dist(q)
    }

    pub fn release(&mut self, k: usize) {
        let e = &mut self.eng[k];
        e.over = true;
        let (b, d, h) = (e.blocker, e.defender, e.helper);
        let t = self.t - e.start;
        let won = e.shed_at.is_none();
        let pass = e.pass;
        if let Some(c) = self.notes.chart.as_mut() {
            let who = [(self.agents[b].id, self.agents[b].label), (self.agents[d].id, self.agents[d].label)];
            c.reps.push(super::chart::Rep::of(e, who, self.agents[d].down_until > self.t, t));
        }
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
        // Read in place, not cloned: nothing changes him before he is
        // steered at the end (world speed lane).
        let me = &self.agents[c];
        let goal = self.goal;
        let vision = me.a(Attr::BallCarrierVision);
        let sight = 7.0 + 0.12 * vision;
        // On a designed run he presses the line at the play's pace, the same
        // for every back, and picks his crease at that pace: the blocks and
        // the box set the run to the line, his speed shows past it (dynasty
        // item 1; was full speed from the handoff).
        // Patient or downhill (tendencies): a patient back presses slower
        // and stays on his press to the line, waiting for the crease (his
        // vision makes it pay); a downhill one presses faster and commits a
        // yard or so behind it (his burst makes that pay).
        let patient = if aim.is_some() { self.lean(c, Tend::Patience) } else { 0.0 };
        let commit = 0.5 + 1.4 * patient.min(0.0);
        // A patient back stops waiting when a free defender is on him.
        let crowded = patient > 0.0
            && self
                .agents
                .iter()
                .any(|d| d.side != me.side && d.up(t) && d.engaged.is_none() && d.pos.dist(me.pos) < 3.0);
        let pressing = self.press > 0.0 && aim.is_some() && (me.pos.x - self.los) * goal < commit && !crowded;
        let press = self.press * (1.0 - 0.25 * patient);
        let speed = if pressing { press.min(me.vmax) } else { me.vmax.max(4.0) };
        // Bounces runs (tendencies): behind the line on a designed run, a
        // bouncer leans off the called hole toward the edge or the cutback;
        // a low one sticks to it. His vision picks which way.
        let behind = aim.is_some() && (me.pos.x - self.los) * goal < 1.0;
        let bounce = if behind { self.lean(c, Tend::Bounce) } else { 0.0 };
        // Contact willingness at the sideline: with a tackler closing, one
        // who avoids contact steps out; one who seeks it turns up.
        let contact = self.lean(c, Tend::Contact);
        // A patient back reads the crease cleanly while he waits.
        let noise = (1.0 - 0.008 * vision).max(0.2) * if pressing { 1.0 - 0.35 * patient } else { 1.0 };
        // The defenders he reckons with, and how free each is: the same
        // for every heading, so worked out once.
        let mut seen: [(V2, f64, f64); 32] = [(V2::default(), 0.0, 0.0); 32];
        let mut n_seen = 0;
        // His eyes as `sees` reads them, the same for every defender: his
        // facing and the half-width of his cone.
        let face_n = me.face.norm();
        let half = {
            let read = if matches!(me.job, Job::Passer) {
                Attr::PocketPresence
            } else if self.carrier() == Some(c) {
                Attr::BallCarrierVision
            } else {
                Attr::PlayRecognition
            };
            1.2 + 0.006 * (me.a(read) - 50.0)
        };
        // The nearest defender he reckons with (`seen` index): his share is
        // summed first when a heading may be dropped (only a bound).
        let (mut nearest, mut near_d) = (usize::MAX, f64::INFINITY);
        for d in self.agents.iter() {
            if d.side == me.side || !d.up(t) {
                continue;
            }
            let dist = d.pos.dist(me.pos);
            if dist > sight + 6.0 {
                continue;
            }
            // Defenders behind him or far away count only if he can see them
            // (`sees`, on the distance already known: past 3 yards nothing is
            // felt, and the cone is only looked at where it decides; the same
            // steps on the same numbers, the arc cosine only near the cone's
            // edge, `acos_lt`).
            if dist > 3.0 && dist > sight * 0.5 {
                let rel = d.pos - me.pos;
                let sees = acos_lt(face_n.dot(v2(rel.x / dist, rel.y / dist)).clamp(-1.0, 1.0), half);
                if !sees {
                    continue;
                }
            }
            let engaged = d.engaged.map(|k| self.eng[k].w);
            let reach = match engaged {
                Some(w) => 0.15 + 0.35 * ((w + 1.0) / 2.0),
                None => 1.0,
            };
            if dist < near_d {
                (near_d, nearest) = (dist, n_seen);
            }
            seen[n_seen] = (d.pos, d.vmax, reach);
            n_seen += 1;
        }
        // How crowded a spot is for him: who gets there before he does.
        let crowd = |p: V2, run: f64| {
            let t_me = me.pos.dist(p) / run.max(1.0);
            seen[..n_seen]
                .iter()
                .map(|&(pos, vmax, reach)| {
                    reach * (((t_me - ((pos.dist(p) - 1.0).max(0.0) / vmax + 0.15)) / 0.35).exp().min(6.0))
                })
                .sum::<f64>()
        };
        // The hole read (bang, bend or bounce): pressing a designed run he
        // reads the called hole a couple of yards from the line and picks
        // his lane, the hole if it is there, else the cutback or the edge,
        // whichever his eyes find; a bouncer leans off the called hole.
        // He then stays in that lane to the line instead of drifting to
        // wherever the bodies are fewest (outside runs ran at two and a
        // half times the real share; sizes proposed, not confirmed).
        let mut aim = aim;
        if let Some(a) = aim {
            let depth = (self.los - me.pos.x) * goal;
            if self.lane.is_none() && depth < 2.5 && self.press > 0.0 {
                let out = if a.y >= self.mood.snap_y { 1.0 } else { -1.0 };
                let lanes = [(a.y, -0.9), (a.y - 3.0 * out, -0.6 * bounce), (a.y + 3.5 * out, -0.6 * bounce)];
                let run = speed.max(4.0);
                let read = 0.8 * (1.0 - 0.008 * vision).max(0.2);
                let mut pick = (f64::INFINITY, a.y);
                for (y, lean) in lanes {
                    let y = clamp(y, 1.0, width() - 1.0);
                    let cost = crowd(v2(self.los + 0.5 * goal, y), run) + lean + read * gauss(self.rng);
                    if cost < pick.0 {
                        pick = (cost, y);
                    }
                }
                self.lane = Some(pick.1);
            }
            if let Some(y) = self.lane {
                aim = Some(v2(a.x, y));
            }
        }
        // Before the read his lane is the called hole.
        let read = self.lane.is_some();
        let lane = aim.map(|a| self.lane.unwrap_or(a.y));
        let bounce = if read { 0.0 } else { bounce };
        // The parts of a heading's score that are the same for every
        // heading, worked out once (the same steps on the same numbers).
        let look = 3.5;
        let vel_len = me.vel.len();
        let t_me = look / speed.max(vel_len * 0.5 + speed * 0.5);
        let field_w = width();
        // Within ten yards of the sideline he reads the angle sooner
        // (six yards and a threat of 0.5 ended 1.6% of NFL plays out
        // of bounds, real 10.7%; fix list S6.2, proposed, not
        // confirmed).
        let near_edge = me.pos.y.min(field_w - me.pos.y) < 10.0;
        let low_half = me.pos.y < mid();
        // Out of bounds when no gain is left: most backs take the
        // sideline rather than a hit for nothing, one who avoids
        // contact sooner, one who seeks it turns up (base proposed,
        // not confirmed: real backs end 4.2% of carries out of
        // bounds, the engine's ended 0.7%).
        let step = (0.5 - 0.6 * contact).max(0.0);
        let vel_n = me.vel.norm();
        let pull = aim.map(|a| (1.2 * (1.0 - 0.6 * bounce), (a - me.pos).norm()));
        let heads = headings();
        // Each heading's noise, drawn in heading order as before (nothing
        // else in the loop draws).
        let mut jolt = [0.0; 15];
        for j in jolt.iter_mut() {
            *j = gauss(self.rng);
        }
        let dir_of = |k: usize| v2(goal * heads[k].0, heads[k].1);
        // Heading k's score but its threat, worked out once: the same steps
        // on the same numbers as scoring it whole (world speed lane).
        struct Part {
            /// The spot 3.5 yards out.
            p: V2,
            gain: f64,
            /// The sideline's cost, and with a tackler closing.
            open: f64,
            closing: f64,
            keep: f64,
            lateral: f64,
            off_lane: f64,
            noise: f64,
            pull: Option<f64>,
        }
        let parts: [Part; 15] = std::array::from_fn(|k| {
            let dir = dir_of(k);
            let p = me.pos + dir.scale(look);
            let gain = dir.x * goal;
            let edge = p.y.min(field_w - p.y);
            let open = if edge < 1.0 { 3.0 * (1.0 - edge) } else { 0.0 };
            // A tackler closing (a threat over 0.3) near the sideline.
            let outward = if low_half { -dir.y } else { dir.y };
            let mut closing = open;
            closing -= 10.0 * step * outward.max(0.0);
            closing += 1.5 * contact.max(0.0) * outward.max(0.0);
            let lateral = dir.y.abs() * 0.9 * bounce;
            // In his lane to the line.
            let off_lane = lane.map_or(0.0, |y| 2.0 * ((p.y - y).abs() - 1.5).max(0.0));
            // He commits to a path: changing course costs speed, so the
            // heading he is on gets a bonus once he is moving.
            let keep = if vel_len > 2.0 { 0.9 * dir.dot(vel_n) } else { 0.0 };
            let shake = noise * jolt[k] * 0.6;
            let tug = pull.map(|(m, wanted)| m * dir.dot(wanted));
            Part { p, gain, open, closing, keep, lateral, off_lane, noise: shake, pull: tug }
        });
        // Its score at a threat and a sideline: the same sum in the same
        // order.
        let score_of = |h: &Part, threat: f64, sideline: f64| {
            let mut score = 1.5 * h.gain - 1.1 * threat - sideline + h.keep + h.lateral - h.off_lane + h.noise;
            if let Some(v) = h.pull {
                score += v;
            }
            score
        };
        let sideline = |h: &Part, threat: f64| if near_edge && threat > 0.3 { h.closing } else { h.open };
        // The sideline at its lowest, at any threat.
        let lowest = |h: &Part| if near_edge && h.closing < h.open { h.closing } else { h.open };
        // One defender's share of the threat at spot p: how soon he gets
        // there against the runner.
        let term = |&(pos, vmax, reach): &(V2, f64, f64), p: V2| {
            let td = (pos.dist(p) - 1.0).max(0.0) / vmax + 0.15;
            let lead = t_me - td;
            reach * (lead / 0.35).exp().min(6.0)
        };
        let seen = &seen[..n_seen];
        // Would heading k at score v be the pick over the best so far:
        // higher, or as high at a lower index (scored in heading order, the
        // first of equal scores won).
        let wins =
            |v: f64, k: usize, best: (f64, Option<usize>)| v > best.0 || (v == best.0 && best.1.is_some_and(|b| k < b));
        let mut best: (f64, Option<usize>) = (f64::NEG_INFINITY, None);
        // Each heading's cap: its score at threat 0 with the sideline at its
        // lowest. While every reach is 0 or more (a pass block won past
        // -1.86 makes one negative) every threat term is too, so no heading
        // scores above its cap, nor above its score on any of its threat's
        // terms summed in order, nor on any one term alone (each sum only
        // grows).
        let mut caps = [f64::NAN; 15];
        if seen.iter().all(|s| s.2 >= 0.0) {
            for (cap, h) in caps.iter_mut().zip(&parts) {
                *cap = score_of(h, 0.0, lowest(h));
            }
        }
        if caps.iter().any(|x| x.is_nan()) {
            // No caps: every heading scored in heading order, as before.
            for (k, h) in parts.iter().enumerate() {
                let mut threat = 0.0;
                for s in seen {
                    threat += term(s, h.p);
                }
                let score = score_of(h, threat, sideline(h, threat));
                if score > best.0 {
                    best = (score, Some(k));
                }
            }
        } else {
            // Highest cap first. A heading whose cap can't win is never
            // scored, and once the highest cap left can't, none can; one
            // whose score on part of its threat (the nearest defender's
            // share alone, then its terms in order) can't win is dropped.
            // The pick is the same heading as scoring them all (world speed
            // lane).
            loop {
                let (mut k, mut cap) = (usize::MAX, f64::NEG_INFINITY);
                for (j, &cj) in caps.iter().enumerate() {
                    if cj > cap {
                        (k, cap) = (j, cj);
                    }
                }
                if k == usize::MAX || !wins(cap, k, best) {
                    break;
                }
                caps[k] = f64::NEG_INFINITY;
                let h = &parts[k];
                let low = lowest(h);
                // Out: his score at this much of the threat, the sideline at
                // its lowest, can't win (a NaN never drops him).
                let out = |threat: f64| {
                    let most = score_of(h, threat, low);
                    best.1.is_some() && !most.is_nan() && !wins(most, k, best)
                };
                let first_share = seen.get(nearest).map(|s| term(s, h.p));
                if let Some(v) = first_share
                    && out(v)
                {
                    continue;
                }
                let mut threat = 0.0;
                let mut dropped = false;
                for (m, s) in seen.iter().enumerate() {
                    threat += match first_share {
                        Some(v) if m == nearest => v,
                        _ => term(s, h.p),
                    };
                    if out(threat) {
                        dropped = true;
                        break;
                    }
                }
                if !dropped {
                    let score = score_of(h, threat, sideline(h, threat));
                    if wins(score, k, best) {
                        best = (score, Some(k));
                    }
                }
            }
        }
        let dir = best.1.map_or(v2(goal, 0.0), dir_of);
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
        let dist = a.pos.dist(cp);
        // In front of the runner and close, he breaks down and squares up:
        // he shuffles to where the runner will cross his spot and comes
        // forward under control, so the runner has to beat him with a move
        // (topic 2.2). Good tacklers keep more speed doing it. Tested first:
        // it reads none of the intercept below, which is pure, so that is
        // worked out only when he chases (world speed lane).
        let ahead = (a.pos.x - cp.x) * self.goal;
        if ahead > 0.4 && ahead < 5.0 && dist < 6.0 && cap >= 1.0 {
            // Breaking down is footwork and feel, not the tackle itself
            // (dynasty item 1: tackling now decides only the finish).
            let feet = 0.5 * a.a(Attr::Agility) + 0.3 * a.a(Attr::Tackling) + 0.2 * a.a(Attr::Pursuit);
            let fwd = clamp(1.5 + 0.04 * (feet - 50.0), 0.8, 3.5);
            // An alley or gap fit holds his depth: at his floor he squares
            // up there and lets the runner come to him (minor bug 164).
            let fwd = match floor {
                Some(f) if (a.pos.x - f) * self.goal <= 0.0 => 0.0,
                _ => fwd,
            };
            let run_x = (cv.x * self.goal).max(0.5);
            let t_meet = ahead / (run_x + fwd);
            let y_aim = clamp(cp.y + cv.y * t_meet, 0.5, width() - 0.5);
            let me = a.pos;
            let lateral = clamp((y_aim - me.y) * 4.0, -a.vmax, a.vmax);
            let want = v2(-self.goal * fwd, lateral);
            self.steer(i, want);
            return;
        }
        let iq = 0.5 * a.a(Attr::PlayRecognition) + 0.5 * a.a(Attr::Pursuit);
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
        // get outside or past him (`LAST_MAN_LEAD`).
        if (a.pos.x - cp.x) * self.goal > 3.0 && dist > 4.0 {
            lead = lead.max(LAST_MAN_LEAD + 0.003 * (iq - 50.0));
        }
        let mut target = cp + cv.scale(tt * lead);
        // An alley player keeps his depth: he meets the runner past the line.
        if let Some(f) = floor
            && (target.x - f) * self.goal < 0.0
        {
            target.x = f;
        }
        // A player out of the play with a low motor stops chasing (his
        // motor is read only then; world speed lane).
        let behind = (a.pos.x - cp.x) * self.goal < -4.0 && dist > 8.0;
        let mut effort = 1.0;
        if behind {
            let motor = self.teams[a.side][a.p].trait20(Attr::Motor, self.level);
            if motor < 9.0 {
                effort = 0.8 + 0.02 * motor;
            }
        }
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
        // The carrier as the scan reads him: nothing in it moves him (it
        // writes only defenders' last tries), so read once (world speed
        // lane).
        let (c_side, c_pos, c_vel) = (self.agents[c].side, self.agents[c].pos, self.agents[c].vel);
        // In space (past the line, no bodies around) a free man can dive.
        let in_space = (c_pos.x - self.los) * self.goal > 3.0;
        // Arm's length standing; a free man can lunge or dive a bit farther
        // at a worse chance (see first_hit).
        let mut new_tacklers = Vec::new();
        for i in 0..self.agents.len() {
            let d = &self.agents[i];
            // His last try is tested first: every test before a draw is
            // pure, so their order changes nothing.
            if d.side == c_side || !d.up(t) || self.fight.contains(&i) || t - d.last_try < 0.7 {
                continue;
            }
            let free_reach = if d.engaged.is_none() && in_space { self.open_reach } else { 1.6 };
            // A sure tackler wraps from farther (wrap range and dives).
            let wrap = if free_reach > 1.6 { clamp(0.01 * (d.a(Attr::Tackling) - 60.0), -0.4, 0.4) } else { 0.0 };
            // Length (Dynamic Links): the long arms add tackling reach, about
            // what five tackling points' wrap range gives (`reach_extra`,
            // with a quick edge's close reach between them, summed in the
            // order they always were).
            let (arms, close, length) = self.reach_extra(i);
            let reach = free_reach + wrap + arms + close + length;
            // Closest approach over the last tick, so fast players can't
            // pass through each other between two samples.
            let rel = d.pos - c_pos;
            let rv = (d.vel - c_vel).scale(DT);
            let rv_len = rv.len();
            // Clearly out of reach all tick: his spot is past reach plus the
            // tick's closing by a micro-yard, so the closest approach is too
            // (far past rounding): skipped before the division and the root,
            // as the test below would skip him. Only within 1000 yards: then
            // far is under 1000 too, so either reach is negative (the test
            // below skips every finite closest approach) or reach and the
            // closing are under 1000 and the rounding is under 1e-11, for
            // any input. A spot that isn't finite takes the test below.
            let q = rel.dot(rel);
            let far = reach + rv_len + 1e-6;
            if q < 1e6 && q > far * far {
                continue;
            }
            let back = if rv_len > 1e-6 { clamp(rel.dot(rv) / rv.dot(rv), 0.0, 1.0) } else { 0.0 };
            let dist = (rel - rv.scale(back)).len();
            if dist > reach {
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
            // Contact willingness (tendencies): a free defender who avoids
            // contact sometimes doesn't take on a bigger man coming at him
            // (the corner who won't tackle in run support).
            let bigger = self.agents[c].mass > d.mass + 15.0 && self.agents[c].vel.len() > 4.0;
            if engaged_w.is_none() && bigger {
                let shy = (-self.lean(i, Tend::Contact) - 0.15).max(0.0);
                if shy > 0.0 && self.rng.chance(0.35 * shy) {
                    self.agents[i].last_try = t;
                    continue;
                }
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

    /// Agent `i`'s tackling reach from his body and his links, as (long
    /// arms, a quick edge's close reach, Length). Real linebackers' arms add
    /// no run tackles (bug 308): theirs show as a surer finish instead
    /// (`body_tackle`). All three read only his player and his level's
    /// typical man, fixed all play, so they are worked out once a play
    /// (world speed lane).
    fn reach_extra(&self, i: usize) -> (f64, f64, f64) {
        let fresh = || {
            let g = self.player(i);
            let arm_reach = if g.position == crate::positions::Position::LB { 0.0 } else { 0.03 };
            (arm_reach * (g.arm - 32.0), self.close_reach(i), 0.05 * g.links.share(&[crate::links::Link::Length]))
        };
        let Some(cell) = self.reach_memo.get(i) else { return fresh() };
        if let Some(v) = cell.get() {
            return v;
        }
        let v = fresh();
        cell.set(Some(v));
        v
    }

    /// E2: a quick edge's long arms reach farther as he closes down the
    /// line, above the typical edge's arm and forty only (yards an inch, at
    /// one forty step of 0.15 s, up to two).
    fn close_reach(&self, i: usize) -> f64 {
        let g = self.player(i);
        if g.position != crate::positions::Position::EDGE {
            return 0.0;
        }
        let (_, tw, ta) = super::synthetic::typical_body(self.level, g.position);
        let typ = |x: Attr| super::synthetic::typical(self.level, g.position, x);
        let t40 =
            5.56 - 0.0124 * (0.75 * typ(Attr::Speed) + 0.25 * typ(Attr::Acceleration)) + crate::body::forty_weight(tw);
        let quick = clamp((t40 - g.forty()) / 0.15, 0.0, 2.0);
        EDGE_CLOSE_REACH * (g.arm - ta).max(0.0) * quick
    }

    /// A tackler's body in his finish, in tackling points over his level's
    /// typical man at his position (L2, C4; bugs 307, 308): real heavier
    /// linebackers and corners miss fewer tackles (-0.53 and -0.57 pts of
    /// misses per 10 lb at the same draft slot), longer-armed linebackers
    /// too (-0.2 pts an inch); safeties' misses are flat by weight.
    fn body_tackle(&self, i: usize) -> f64 {
        use crate::positions::Position as P;
        let g = self.player(i);
        let (_, tw, ta) = super::synthetic::typical_body(self.level, g.position);
        match g.position {
            P::LB => TACKLE_WEIGHT_LB * (g.weight - tw) + TACKLE_ARM_LB * (g.arm - ta),
            P::CB => TACKLE_WEIGHT_CB * (g.weight - tw),
            _ => 0.0,
        }
    }

    fn first_hit(&mut self, c: usize, i: usize, arm_only: bool, lunge: f64) {
        let t = self.t;
        let (r, d) = (self.agents[c].clone(), self.agents[i].clone());
        let closing = (d.vel - r.vel).len();
        let head_on = r.vel.norm().dot((d.pos - r.pos).norm());
        // Whiff: the runner's move against the tackler's technique.
        // Dynamic Links: Sudden makes the first man miss; a Sudden tackler
        // mirrors the cut.
        use crate::links::Link;
        // The make-a-man-miss skill is Elusiveness: real broken tackles
        // don't move with a carrier's weight or 3-cone once talent is known
        // (minor bug 279). Weight still costs him burst, braking and cut,
        // and his agility his cut (`game/roster.rs`). His level's typical
        // man at his position evades as before (it read 0.5 Elusiveness,
        // 0.3 Agility and size_cost from 210 lb), so only the spread within
        // the level changes.
        let pos = self.player(c).position;
        let typ = |a: Attr| super::synthetic::typical(self.level, pos, a);
        let was = 0.3 * (typ(Attr::Agility) - typ(Attr::Elusiveness))
            - size_cost(super::synthetic::typical_body(self.level, pos).1, 210.0);
        let typ_evade = 0.8 * typ(Attr::Elusiveness) + 0.2 * typ(Attr::Speed);
        let own = 0.8 * r.a(Attr::Elusiveness) + 0.2 * r.a(Attr::Speed);
        // His own evade over the typical man's counts `CARRIER_SPREAD` of it
        // (minor bug 334).
        let evade = typ_evade + CARRIER_SPREAD * (own - typ_evade) + was + self.link(c, &[Link::Sudden]);
        let tackle = 0.6 * d.a(Attr::Tackling)
            + 0.2 * d.a(Attr::Agility)
            + 0.2 * d.a(Attr::Pursuit)
            + self.link(i, &[Link::Sudden])
            + self.body_tackle(i);
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
        // The big hit against the sure tackle, and the strip against the
        // wrap (tendencies): a man throwing his body or punching at the
        // ball misses more often, a sure tackler less. Scaled straight on
        // the miss chance, so the league average stays where it was.
        let style = (1.0 + 0.7 * self.lean(i, Tend::BigHit) + 0.25 * self.lean(i, Tend::Strip)).max(0.2);
        let logit = -3.0 + (evade - tackle) / 50.0 + angle + fast + space + arm + dive;
        let p_miss = (style / (1.0 + (-logit).exp())).min(0.95);
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
            // Low center of gravity keeps him up through it.
            let bal = r.a(Attr::Balance) + self.link(c, &[Link::LowCenter]);
            let trip = if head_on < 0.3 { 0.72 } else { 0.88 } + clamp(0.002 * (bal - 60.0), -0.08, 0.08);
            self.agents[c].vel = self.agents[c].vel.scale(trip.min(0.96));
            return;
        }
        if self.notes.first_contact.is_none() {
            self.notes.first_contact = Some(self.agents[c].pos.x);
        }
        // A blocked man's arm usually only grabs at a runner going full
        // speed: it slows him, but he runs through it.
        // His balance against the tackler's will to finish the hit.
        // Low center of gravity and Battering ram run through the arm.
        let gift = if r.vel.len() > 4.0 {
            self.link(c, &[Link::LowCenter, Link::BatteringRam])
        } else {
            self.link(c, &[Link::LowCenter])
        };
        let through = 0.6
            * clamp(1.0 + 0.008 * (r.a(Attr::Balance) + gift - 60.0), 0.75, 1.25)
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
        // A big hitter puts more into it (tendencies).
        let force =
            (r.mass * r.vel.len() + d.mass * d.vel.len()) * 0.5 * (1.0 + 0.2 * share * self.lean(i, Tend::BigHit));
        self.contact_force = self.contact_force.max(force);
        // Ball security against the hit (topic 2), read against the man
        // making it: his strength and tackling against the carrier's ball
        // security. It was read on its own, so NFL carriers, rated far above
        // 60, almost never fumbled (0.27 lost a team game, real 0.45). Now
        // about 0.45 in the NFL and 0.4 in FBS (proposed, not confirmed).
        let carry = r.a(Attr::BallSecurity);
        let hit = d.a(Attr::Strength) * 0.5 + d.a(Attr::Tackling) * 0.5;
        let hit_typ = {
            let pos = self.player(i).position;
            let typ = |a: Attr| super::synthetic::typical(self.level, pos, a);
            0.5 * typ(Attr::Strength) + 0.5 * typ(Attr::Tackling)
        };
        let rain = (if self.cond.rain || self.cond.snow { 1.35 } else { 1.0 }) * (1.0 + 0.3 * self.cond.cold_unused);
        // Going for the strip (tendencies): the punch-out.
        let strip = (1.0 + 1.15 * self.lean(i, Tend::Strip)).max(0.1);
        let p = 0.018
            * (force / 1700.0).powf(1.5)
            * clamp(1.0 + 0.025 * (hit - carry) - (0.025 - FUMBLE_HIT) * (hit - hit_typ), 0.25, 2.5)
            * rain
            * strip;
        if self.notes.fumbled.is_none() && self.rng.chance(p) {
            self.notes.fumbled = Some(r.id);
            self.notes.forced_fumble = Some(d.id);
            self.fumble(c);
            return;
        }
        // Battering ram wears a defense down: the man who takes him on
        // feels it on the next snaps.
        let ram = self.link(c, &[crate::links::Link::BatteringRam]);
        self.agents[i].work += 2.0 * ram * share;
        // A runner who seeks contact takes more of it.
        self.injury_check(c, force, (1.0 + 0.4 * self.lean(c, Tend::Contact)).max(0.2));
        self.injury_check(i, force, 0.7);
    }

    fn fight_tick(&mut self, c: usize) {
        let t = self.t;
        // Taking hits wears a runner down.
        self.agents[c].work += DT * 30.0;
        let r = self.agents[c].clone();
        // Dynamic Links: Low center of gravity stays up through contact
        // (five balance points); Battering ram's mass at speed keeps the
        // legs going (five strength points).
        use crate::links::Link;
        let low = self.link(c, &[Link::LowCenter]);
        let ram = self.link(c, &[Link::BatteringRam]);
        // Pad level and leg drive keep him going; balance keeps him up.
        let resist = 1.0
            + 0.003 * low
            + 0.003 * ram
            + 0.003 * (r.a(Attr::Strength) - 60.0)
            + 0.005 * (r.a(Attr::LegDrive) - 60.0)
            + 0.003 * (r.a(Attr::Balance) - 60.0)
            + 0.002 * (r.a(Attr::Elusiveness) - 60.0)
            + 0.006 * (self.trait20(c, Attr::Competitiveness) - 10.0)
            + 0.15 * self.lean(c, Tend::Contact)
            + (r.mass - 215.0) / 300.0;
        let mut hazard = 0.0;
        let mut hold = 0.0;
        for &i in &self.fight {
            let d = &self.agents[i];
            // The thumper (Battering ram on defense): five tackling points.
            let f = 1.0
                + 0.03 * (d.a(Attr::Tackling) + self.link(i, &[Link::BatteringRam]) - 60.0)
                + 0.012 * (d.a(Attr::Strength) - 60.0)
                + 0.01 * (self.trait20(i, Attr::Competitiveness) - 10.0)
                - 0.12 * self.lean(i, Tend::Strip)
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
            let tk = 0.6 * d.a(Attr::Tackling)
                + 0.4 * d.a(Attr::Strength)
                + (d.mass - 215.0) * 0.08
                + self.link(i, &[Link::BatteringRam, Link::Length]);
            let pos = self.player(c).position;
            let typ = |a: Attr| super::synthetic::typical(self.level, pos, a);
            let tw = super::synthetic::typical_body(self.level, pos).1;
            let typ_rb = 0.5 * typ(Attr::Strength) + 0.5 * typ(Attr::Elusiveness) + (tw - 215.0) * 0.08;
            let own = 0.5 * r.a(Attr::Strength) + 0.5 * r.a(Attr::Elusiveness) + (r.mass - 215.0) * 0.08;
            let rb = typ_rb + CARRIER_SPREAD * (own - typ_rb) + self.link(c, &[Link::BatteringRam, Link::LowCenter]);
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
            if self.reach(c) {
                return;
            }
            self.tackled(c);
            return;
        }
        // The pile moves: legs churning against the weight on him (2.2
        // before minor bug 164: a lineman's stop gained 0.8 yards, real 2.5;
        // proposed, not confirmed).
        let drive = (r.mass * (3.0 + 0.03 * (r.a(Attr::Strength) - 60.0)) - 0.35 * hold) / (r.mass + 0.6 * hold);
        let v = self.agents[c].vel.x * self.goal;
        let nv = (0.6 * v + 0.4 * drive * 2.0).clamp(-1.5, 4.0);
        self.agents[c].vel = v2(nv * self.goal, self.agents[c].vel.y * 0.5);
        for &i in &self.fight.clone() {
            let p = self.agents[c].pos;
            self.agents[i].pos = p + (self.agents[i].pos - p).norm().scale(0.7);
            self.agents[i].vel = self.agents[c].vel;
        }
    }

    /// Reaching the ball out as he goes down (tendencies): short of the line
    /// to gain or the goal line with a tackler on him, a reacher stretches
    /// the ball for it. It gets there if the line is within his arm, and the
    /// ball is out in front of him, away from his body: a hit can knock it
    /// loose (Calvin Johnson at Seattle, 2015). His ball security against
    /// the tackler decides how often. Returns true when it came loose.
    fn reach(&mut self, c: usize) -> bool {
        if self.goal < 0.0 || self.agents[c].side != 0 {
            return false;
        }
        let x = self.agents[c].pos.x;
        let lines = [self.mood.line, Some(goal())];
        let Some(line) = lines
            .into_iter()
            .flatten()
            .filter(|&l| l > x)
            .fold(None, |m: Option<f64>, l| Some(m.map_or(l, |m| m.min(l))))
        else {
            return false;
        };
        let short = line - x;
        if !(0.05..1.6).contains(&short) {
            return false;
        }
        let lean = self.lean(c, Tend::Reach);
        let p = clamp(0.25 + 0.55 * lean, 0.0, 0.97);
        if !self.rng.chance(p) {
            return false;
        }
        let arm = 0.75 + 0.03 * (self.player(c).arm - 32.0);
        let gets = short <= arm;
        let i = self.fight.first().copied().unwrap_or(c);
        let carry = self.agents[c].a(Attr::BallSecurity);
        let hit = self.agents[i].a(Attr::Strength) * 0.5 + self.agents[i].a(Attr::Tackling) * 0.5;
        let loose = self.rng.chance(0.035 * clamp(1.0 + 0.025 * (hit - carry), 0.3, 2.5));
        self.notes.tend.reach = Some((self.agents[c].id, gets && !loose, loose));
        if loose {
            self.progress = self.progress.max(x);
            self.notes.fumbled = Some(self.agents[c].id);
            self.notes.forced_fumble = Some(self.agents[i].id);
            self.agents[c].pos.x = x + arm.min(short + 0.3);
            self.fumble(c);
            return true;
        }
        if gets {
            self.agents[c].pos.x = line + 0.05;
        }
        false
    }

    pub fn tackled(&mut self, c: usize) {
        self.notes.tacklers = self.fight.iter().map(|&i| self.agents[i].id).collect();
        let x = self.agents[c].pos.x;
        if (x - self.progress) * self.goal > 0.0 {
            self.progress = x;
        }
        // Taken down, the pile goes where its weight is going: he falls
        // about 0.3 of a second of it, up to a yard and a half (a quarter
        // second before minor bug 164).
        let mut mv = self.agents[c].vel.scale(self.agents[c].mass);
        let mut mass = self.agents[c].mass;
        for &i in &self.fight {
            mv = mv + self.agents[i].vel.scale(self.agents[i].mass);
            mass += self.agents[i].mass;
        }
        let fall = mv.scale(0.3 / mass);
        let fall = if fall.len() > 1.5 { fall.norm().scale(1.5) } else { fall };
        // Falling forward: going down with his weight going forward, the
        // ball lands ahead of his hips, by the fall and the reach of his
        // body, up to most of a yard more (the spot was his hips: backs
        // gained 1.2 yards a carry after contact, real 2.0, PFR 2021-25;
        // proposed, not confirmed; 0.9 of a yard, then 1.05 with minor bug
        // 164, where a lineman's stop gained 0.8 yards against 2.5 real).
        // Driven back, the spot is where he got.
        let fwd = fall.x * self.goal;
        if fwd > 0.0 {
            let body = 1.05 * (fwd / 0.6).min(1.0);
            let spot = x + (fwd + body) * self.goal;
            if (spot - self.progress) * self.goal > 0.0 {
                self.progress = spot;
            }
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
            let temper = (1.6 - self.player(i).traits.temperament / 12.0).max(0.2)
                * 0.6
                * (1.0 + 0.6 * self.lean(i, Tend::BigHit)).max(0.2)
                * (1.0 + self.agents[i].heat).max(0.3);
            let (id, side) = (self.agents[i].id, self.agents[i].side);
            if self.rng.chance(0.0087 * self.crew.personal * technique) {
                self.notes.fouls.push(Foul { kind: FoulKind::FaceMask, side, player: id, at: x });
            } else if self.rng.chance(0.023 * self.crew.personal * temper) {
                self.notes.fouls.push(Foul { kind: FoulKind::UnnecessaryRoughness, side, player: id, at: x });
            }
        }
        self.dead = true;
        self.ball = Ball::Dead(v2(self.progress, self.agents[c].pos.y));
        // Taken down at the sideline, he lands out of bounds if the fall
        // crosses the line (the clock stops; proposed, not confirmed).
        let y = self.agents[c].pos.y + fall.y;
        if y <= 0.0 || y >= width() {
            self.out_of_bounds = true;
        } else {
            // Caught within eight yards of the sideline, the tackler often
            // drives him out rather than take him down in bounds, surely
            // inside three and a half yards, more when the pile is going
            // that way (fix list S6.2: with the wider step-out above, NFL
            // plays out of bounds 1.6% to 5.3%, real 10.7%; completions
            // 4.2% to 13.9%, real 20.7%. Minor bug 141: six yards left
            // completions at 16.2%, eight puts them at 19%; proposed, not
            // confirmed).
            let edge = y.min(width() - y);
            if edge < PUSH_OUT {
                let out = if y < mid() { -1.0 } else { 1.0 };
                let push = if fall.len() > 0.05 { (fall.y * out / fall.len()).max(0.0) } else { 0.0 };
                let chance = ((PUSH_OUT - edge) / 4.5).min(1.0) * (0.5 + 0.5 * push);
                if self.rng.chance(chance) {
                    self.out_of_bounds = true;
                }
            }
        }
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
            // Who dives on the pile: contact willingness (tendencies).
            let dive = 1.0 + 0.25 * self.lean(i, Tend::Contact);
            let w = (-d / 1.8).exp() * (0.6 + (a.a(Attr::Anticipation) + a.a(Attr::Competitiveness)) / 400.0) * dive;
            wsum[a.side] += w;
            if w > best[a.side].0 {
                best[a.side] = (w, i);
            }
        }
        let side = if self.rng.chance(wsum[0] / (wsum[0] + wsum[1]).max(1e-9)) { 0 } else { 1 };
        let who = best[side].1;
        self.notes.recovered = Some((side, self.agents[who].id));
        let spot = v2(spot.x.clamp(back_line() + 0.1, end_line() - 0.1), spot.y.clamp(0.1, width() - 0.1));
        self.dead = true;
        self.ball = Ball::Dead(spot);
        self.progress = if side == self.agents[c].side { p.x } else { spot.x };
    }

    /// Injury risk from a hit (topic 6): force, fatigue, his proneness and
    /// the turf. Trial 9 finding 13: at 0.0042 a hit, injuries costing a
    /// week or more came to 0.20 a team game in the NFL and 0.17 in FBS,
    /// against about 1.0 and 0.9 real (NFL game-status reports 2012-2025,
    /// about 0.42 episodes a regular's season; NCAA surveillance, 32.1 game
    /// injuries per 1,000 exposures, half past a week:
    /// `docs/research/development-and-injuries.md`); harder hits at the
    /// higher level keep the NFL above college.
    pub fn injury_check(&mut self, i: usize, force: f64, share: f64) {
        let gp = self.player(i);
        let gp_recovery = gp.traits.recovery;
        let prone = 0.55 + gp.injury_proneness / 18.0;
        let tough = gp.trait20(Attr::Toughness, self.level);
        let turf = if self.cond.turf { 1.12 } else { 1.0 };
        // His strength staff's guard on muscle strains and backs (minor bug 303).
        let (mix, guarded) = guarded_mix(&GAME_INJURIES, gp.soft as f64);
        // Tired muscles and slower reactions get hurt more (update 3, F8;
        // proposed, not confirmed: up to 60% more for a man at the 0.25
        // energy floor, about 15% after a long drive).
        let tired = 1.0 + 0.8 * self.agents[i].tired;
        let p = self.injury_scale * 0.022 * share * (force / 1700.0).powf(1.6) * prone * turf * tired * guarded;
        if !self.rng.chance(p) {
            return;
        }
        // The mix by body part (`GAME_INJURIES`).
        let kind = self.rng.weighted(&mix);
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

    /// Keeps this tick for the views when the play is recorded. Output
    /// only: nothing here moves a player or draws a number.
    pub fn record_frame(&mut self) {
        if let Some(fr) = self.record.take() {
            let f = super::pose::frame(self, &fr);
            let mut fr = fr;
            fr.push(f);
            self.record = Some(fr);
        }
    }

    /// Ball carrier out of bounds, in the end zone, or down.
    pub fn check_carrier(&mut self, c: usize) -> bool {
        let a = &self.agents[c];
        let p = a.pos;
        if p.y <= 0.0 || p.y >= width() {
            // Stepped out with a tackler closing (contact willingness).
            let side = a.side;
            self.notes.tend.stepped_out =
                self.agents.iter().any(|d| d.side != side && d.up(self.t) && d.pos.dist(p) < 3.5);
            self.out_of_bounds = true;
            self.dead = true;
            let x = if (p.x - self.progress) * self.goal > 0.0 { p.x } else { self.progress };
            self.progress = x;
            self.ball = Ball::Dead(v2(x, p.y.clamp(0.0, width())));
            return true;
        }
        let goal_line = if self.goal > 0.0 { goal() } else { 0.0 };
        if (p.x - goal_line) * self.goal >= 0.0 {
            self.progress = goal_line;
            self.dead = true;
            self.ball = Ball::Dead(p);
            return true;
        }
        if p.x > end_line() || p.x < back_line() {
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
        mid()
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
        // Set so each one's share out four weeks or more is near its real
        // body part's (nflverse 2012-2025; proposed, not confirmed).
        InjuryKind::Foot => (0.5, 9.5),
        InjuryKind::Groin => (0.5, 5.5),
        InjuryKind::Calf => (0.5, 5.5),
        InjuryKind::Neck => (0.0, 10.0),
        InjuryKind::Hip => (0.0, 7.0),
        InjuryKind::Quad => (0.5, 6.0),
    };
    let u = rng.next();
    lo + (hi - lo) * u * u
}
