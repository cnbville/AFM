//! Who he is, what he wants, what he can do, and how he feels during a talk
//! (sections 3.3 to 3.5, 6 and 8.3). Lane N1 fills `Wanting` and the heat
//! and patience sizes; lane N3 fills `Acting`.
//!
//! His wants come from the stores that already hold them (`own_wants`,
//! `decider_wants`, the NFL's `goals`, a booster's `cast::Want`); a `Want`
//! here only says which one and how much it weighs for him today.

use super::Them;
use super::terms::Request;
use crate::world::World;
use crate::world::folk::cast;

/// Every kind of person you can talk to (section 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    /// Your player at any level (6.1).
    Player,
    /// A player's parent, grandparent or decider (6.2).
    Family,
    /// A high school recruit (6.3).
    Recruit,
    /// A recruit's high school coach (6.4).
    HsCoach,
    /// The rest of a recruit's circle: his 7-on-7 coach, his trainer, a
    /// teacher, his counselor, a handler, a street agent (6.4).
    Circle,
    /// A transfer in the portal (6.5).
    Transfer,
    /// A player's, an NIL or a coach's agent (6.6).
    Agent,
    /// Another club's GM (6.7).
    Gm,
    /// Another team's head coach or assistant (6.7).
    OtherCoach,
    /// Another club's scout or a pro scout (6.7, 6.11).
    OtherScout,
    /// Someone on your staff: coordinators, position coaches, strength,
    /// trainers and doctors, recruiting staff, scouts, the analyst, QC,
    /// academic staff, GAs (6.8).
    Staff,
    /// Your boss: a principal, a school AD, an AD, a president, an owner,
    /// your GM, your head coach in an assistant seat (6.9, 6.14).
    Boss,
    /// A booster, a collective, a donor (6.10).
    Booster,
    /// A draft prospect (6.11).
    Prospect,
    /// An official (6.12).
    Official,
    /// The compliance office, NCAA enforcement, the league or conference
    /// office, an investigator, a state association (6.13, 6.15).
    Compliance,
    /// A reporter or a search firm (6.16, 6.17; the press waits on question 8).
    Outside,
}

/// What a person can want (3.3). Each names the store that holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WantKind {
    /// Playing time, his role as he sees it (`own_wants`, P1's `self_view`).
    Role,
    /// NIL, revenue share, a contract's money (`own_wants`, `goals`).
    Money,
    /// Security: guarantees, years, a job that lasts.
    Security,
    /// Winning.
    Winning,
    /// Being close to home, or a city he likes.
    Place,
    /// Development: coaching that gets him to the next level.
    Development,
    /// Academics and a degree.
    Academics,
    /// The school's or the club's name.
    Name,
    /// Loyalty to where he is.
    Loyalty,
    /// His family's good (what the family weighs, `generate_family`).
    Family,
    /// A friend's or a teammate's good.
    Friends,
    /// An agent's fee.
    Fee,
    /// His own career: a better job, a title, play-calling.
    Career,
    /// His scheme, his say in it.
    Scheme,
    /// His team's value, his cap, his job (a GM).
    Club,
    /// His goals for you, the budget, the program's name (a boss,
    /// `boss.rs`, `draft_goals`).
    Goals,
    /// A booster's want (`cast::Want`).
    Booster(cast::Want),
    /// His kids' futures and his program's name (a high school coach).
    Kids,
}

/// One of his wants today and how much it weighs for him, 0-1. An engine
/// number, never shown; the cards carry it only as words.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Want {
    pub kind: WantKind,
    pub weight: f32,
    /// You've already given it (a promise, a term, an act): what he asks
    /// for next is what he wants most that you haven't given (question 3).
    pub met: bool,
}

/// Something he can do when you ask (3.4, 8.3), as the game runs it. The
/// request is `terms::Request`; this is his answer to it.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Willing {
    /// He'll do it: the game runs the action by its own rules.
    Yes,
    /// He will if you give him something (a counter).
    If(super::terms::Counter),
    /// He'll think about it, and answer by a day (taking time, section 11).
    Later(crate::calendar::Date),
    /// No.
    #[default]
    No,
}

/// What came of a request the game ran: a real action, and his report
/// back in his own words, honest or not.
#[derive(Clone, Debug, Default)]
pub struct Ran {
    /// What the game changed, in words for your screen.
    pub done: Vec<String>,
    /// His report back, released words for the voice.
    pub report: Vec<String>,
    /// His report was honest.
    pub honest: bool,
}

/// Heat during a talk (3.5): 0 calm, 1 he's past the point. Insults, a lie
/// he catches or a broken promise raise it, by his temper. Sizes are lane
/// N1's.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Heat(pub f32);

/// What he does when the heat passes his point. The game decides, never
/// the voice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Boil {
    /// He goes quiet.
    Quiet,
    /// He raises his voice.
    Loud,
    /// He walks out.
    WalksOut,
}

/// Patience during a talk (3.5), for everyone: 1 fresh, 0 done. A long
/// talk that goes nowhere wears it down; asking again after a no costs it
/// (section 11). An agent's built patience seeds it.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Patience(pub f32);

impl Default for Patience {
    fn default() -> Patience {
        Patience(1.0)
    }
}

/// What he wants (lane N1).
pub trait Wanting {
    /// His kind, from where the world holds him.
    fn kind(&self, w: &World, him: Them) -> Kind;
    /// His wants today, the heaviest first.
    fn wants(&self, w: &World, him: Them) -> Vec<Want>;
}

/// What he does when you ask (lane N3).
pub trait Acting {
    /// Whether he'll do it, from his trust, loyalty, leadership and what it
    /// costs or gains him.
    fn willing(&self, w: &World, him: Them, ask: &Request) -> Willing;
    /// Runs what he agreed to, by the game's own rules.
    fn run(&self, w: &mut World, him: Them, ask: &Request) -> Ran;
}
