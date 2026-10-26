//! The head coach's guidelines (update 2, lane 5: G1-G3). A head coach
//! tells his coordinators how he wants the team to play: a base front, how
//! much man coverage and pressure, an offensive system, a personnel group,
//! a tempo, a run share and how closely to stick to the call sheet. Each
//! comes with a firmness. Guidelines lean the coordinators and never bind
//! them: a 4-3 team still goes to nickel against three receivers, and a
//! coordinator still answers the down, the distance and the score. But a
//! firm one pulls hard enough that a roster built for man coverage doesn't
//! end up in zone drops.
//!
//! The pull is the same for every team: the AI's head coaches set theirs
//! from their own leanings (`world::guide`). A coordinator asked to call
//! what he knows less well calls it as well as he knows it (`systems.rs`
//! knowledge, through the caller's fit), never with a flat penalty.
//!
//! Sizes are Claude's picks (proposed, not confirmed).

use super::playbook::Personnel;
use super::roster::{Front, OffScheme, Shell};
use crate::rng::{Rng, clamp};

/// How hard a guideline pulls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Firm {
    /// "This is how we play": the coordinator moves most of the way.
    Firm,
    /// A lean: about halfway.
    #[default]
    Lean,
    /// The coordinator's call: a nudge.
    Theirs,
}

impl Firm {
    pub const ALL: [Firm; 3] = [Firm::Firm, Firm::Lean, Firm::Theirs];
    /// The share of the gap between the coordinator's own habit and the
    /// guideline that he closes (the boards' 85, 55 and 15).
    pub fn pull(self) -> f64 {
        match self {
            Firm::Firm => 0.85,
            Firm::Lean => 0.55,
            Firm::Theirs => 0.15,
        }
    }
    pub fn key(self) -> &'static str {
        ["firm", "lean", "theirs"][self as usize]
    }
    pub fn from_key(k: &str) -> Option<Firm> {
        Firm::ALL.into_iter().find(|f| f.key() == k)
    }
}

/// The base front a head coach asks for. The 4-2-5 and 3-3-5 are nickel
/// fronts played as the base (college and high school mostly).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BaseFront {
    FourThree,
    ThreeFour,
    FourTwoFive,
    ThreeThreeFive,
}

impl BaseFront {
    pub const ALL: [BaseFront; 4] =
        [BaseFront::FourThree, BaseFront::ThreeFour, BaseFront::FourTwoFive, BaseFront::ThreeThreeFive];
    pub fn name(self) -> &'static str {
        ["4-3", "3-4", "4-2-5", "3-3-5"][self as usize]
    }
    /// The line it installs: four down or three down.
    pub fn front(self) -> Front {
        match self {
            BaseFront::FourThree | BaseFront::FourTwoFive => Front::FourThree,
            BaseFront::ThreeFour | BaseFront::ThreeThreeFive => Front::ThreeFour,
        }
    }
    /// A nickel front as the base: a fifth defensive back on early downs
    /// even against two receivers.
    pub fn nickel_base(self) -> bool {
        matches!(self, BaseFront::FourTwoFive | BaseFront::ThreeThreeFive)
    }
}

/// The tempo a head coach wants from his offense.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Pace {
    Huddle,
    Mixed,
    NoHuddle,
}

impl Pace {
    pub const ALL: [Pace; 3] = [Pace::Huddle, Pace::Mixed, Pace::NoHuddle];
    pub fn name(self) -> &'static str {
        ["Huddle", "Mixed", "No-huddle"][self as usize]
    }
    /// The share of snaps (outside the two-minute drill) without a huddle.
    pub fn share(self) -> f64 {
        match self {
            Pace::Huddle => 0.03,
            Pace::Mixed => 0.2,
            Pace::NoHuddle => 0.6,
        }
    }
}

/// One guideline: what the head coach wants (None: nothing said) and how
/// firmly.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, bound(deserialize = "T: serde::Deserialize<'de>"))]
pub struct Want<T> {
    pub to: Option<T>,
    pub firm: Firm,
}

impl<T> Default for Want<T> {
    fn default() -> Want<T> {
        Want { to: None, firm: Firm::default() }
    }
}

impl<T: Copy> Want<T> {
    pub fn new(to: T, firm: Firm) -> Want<T> {
        Want { to: Some(to), firm }
    }
    /// What he wants and how hard it pulls.
    pub fn pull(&self) -> Option<(T, f64)> {
        self.to.map(|t| (t, self.firm.pull()))
    }
}

/// A head coach's guidelines. Shares are 0-1: man coverage of the
/// coverage calls, five or more rushers of the defensive snaps, runs of
/// the offensive calls, sheet calls of the snaps where a sheet play fits.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Guidelines {
    pub front: Want<BaseFront>,
    pub man: Want<f64>,
    pub pressure: Want<f64>,
    pub system: Want<OffScheme>,
    pub personnel: Want<Personnel>,
    pub pace: Want<Pace>,
    pub run: Want<f64>,
    pub stick: Want<f64>,
}

impl Guidelines {
    pub fn is_empty(&self) -> bool {
        *self == Guidelines::default()
    }
    /// The defensive side's guidelines dropped (the head coach calls the
    /// defense himself: he calls his own game).
    pub fn without_defense(mut self) -> Guidelines {
        self.front = Want::default();
        self.man = Want::default();
        self.pressure = Want::default();
        self
    }
    /// The offense's guidelines dropped (its play caller calls his own
    /// game). Sticking to the card is the program's, for both callers: it
    /// stays while the other side still has a caller to hold to it.
    pub fn without_offense(mut self) -> Guidelines {
        self.system = Want::default();
        self.personnel = Want::default();
        self.pace = Want::default();
        self.run = Want::default();
        self
    }
}

/// Where a coordinator lands: his own habit moved toward the guideline by
/// its pull.
pub fn expected(own: f64, want: f64, pull: f64) -> f64 {
    own + pull * (want - own)
}

/// A coverage family's man share (`calling::shell_mix`): Cover 0, Cover 1,
/// Cover 1 robber and 2-man of all coverage calls.
pub fn habit_man(shell: Shell) -> f64 {
    let mix = super::calling::shell_mix(shell);
    let all: f64 = mix.iter().map(|x| x.1).sum();
    mix.iter().filter(|x| x.0.is_man()).map(|x| x.1).sum::<f64>() / all.max(1.0)
}

/// A defensive coordinator's own share of snaps with five or more
/// rushers, from his aggressiveness (about what `call_defense` sends over
/// a game's downs: the base rate, a little more on third and medium, and
/// Cover 0's double-A).
pub fn habit_pressure(aggressiveness: f64) -> f64 {
    clamp(0.24 + 0.012 * (aggressiveness - 10.0), 0.05, 0.6)
}

/// A play caller's own pass share over a game: the level's real rate,
/// his system's lean and his aggressiveness (`call_offense`).
pub fn habit_pass(college: bool, scheme: OffScheme, aggressiveness: f64) -> f64 {
    clamp((if college { 0.5 } else { 0.58 }) + scheme.pass_lean() + 0.006 * (aggressiveness - 10.0), 0.15, 0.85)
}

/// A system's own no-huddle share outside the two-minute drill (S10).
pub fn habit_pace(scheme: OffScheme) -> f64 {
    match scheme {
        OffScheme::AirRaid | OffScheme::SpreadOption => 0.35,
        _ => 0.06,
    }
}

/// How closely a play caller sticks to the call sheet on his own: a
/// creative one calls off the card more (proposed, not confirmed).
pub fn habit_stick(creativity: f64) -> f64 {
    clamp(0.86 - 0.012 * (creativity - 10.0), 0.6, 0.97)
}

/// The share of man coverage a defense ends up in, given its coverage
/// family and the guideline.
pub fn expected_man(shell: Shell, g: &Guidelines) -> f64 {
    let own = habit_man(shell);
    g.man.pull().map_or(own, |(t, k)| expected(own, t, k))
}

/// Move the man-coverage share of a set of coverage weights to `target`,
/// keeping the shape inside man and inside zone (the coordinator still
/// picks the coverage that fits the down).
pub fn steer_man(w: &mut [(super::playbook::Coverage, f64)], shift: f64) {
    let man: f64 = w.iter().filter(|x| x.0.is_man()).map(|x| x.1).sum();
    let all: f64 = w.iter().map(|x| x.1).sum();
    if all <= 0.0 || shift == 0.0 {
        return;
    }
    let now = man / all;
    let target = clamp(now + shift, 0.02, 0.97);
    let (km, kz) = (
        if man > 0.0 { target * all / man } else { 1.0 },
        if all - man > 0.0 { (1.0 - target) * all / (all - man) } else { 1.0 },
    );
    for x in w.iter_mut() {
        x.1 *= if x.0.is_man() { km } else { kz };
    }
    // No man weight at all to scale: give Cover 1 the share.
    if man <= 0.0
        && target > 0.0
        && let Some(x) = w.iter_mut().find(|x| x.0 == super::playbook::Coverage::Cover1)
    {
        x.1 = target * all;
    }
}

/// A head coach's guidelines drawn for a synthetic team (match checks):
/// about half of head coaches say something on each side, mostly as a
/// lean, rarely the opposite of what the staff already does (proposed,
/// not confirmed).
pub fn draw(rng: &mut Rng, college: bool, scheme: OffScheme, shell: Shell, aggressiveness: f64) -> Guidelines {
    let firm = |rng: &mut Rng| *rng.pick(&[Firm::Firm, Firm::Lean, Firm::Lean, Firm::Lean, Firm::Theirs]);
    let mut g = Guidelines::default();
    if rng.chance(0.5) {
        let own = habit_man(shell);
        g.man = Want::new(clamp(own + rng.normal(0.0, 0.12), 0.1, 0.75), firm(rng));
    }
    if rng.chance(0.4) {
        let own = habit_pressure(aggressiveness);
        g.pressure = Want::new(clamp(own + rng.normal(0.0, 0.08), 0.08, 0.5), firm(rng));
    }
    if rng.chance(0.45) {
        let own = 1.0 - habit_pass(college, scheme, 10.0);
        g.run = Want::new(clamp(own + rng.normal(0.0, 0.06), 0.25, 0.65), firm(rng));
    }
    if college && rng.chance(0.2) {
        g.front = Want::new(BaseFront::FourTwoFive, firm(rng));
    }
    if rng.chance(0.15) {
        g.pace = Want::new(if rng.chance(0.5) { Pace::Mixed } else { Pace::Huddle }, firm(rng));
    }
    g
}
