//! Terms, offers, counters, requests and decisions (sections 8.2 to 8.4 and
//! 11). Lane N3 values them (`Valuing`); lane N2 and the promises thread
//! judge them (the measure readers).
//!
//! A term is a measure, a number from your words, a scope, a horizon and
//! "as long as" (8.4). The same term serves promises, incentives, NIL
//! clauses, conditional picks and promises to coaches and bosses. A
//! promise's own record stays P1's `talks::Promise`; its `promise::Terms`
//! (by when, the condition, ahead of whom) is what a term's horizon and
//! "as long as" reuse, so a term said as a promise logs through P1
//! unchanged.

use super::Them;
use super::wants::Want;
use crate::calendar::Date;
use crate::world::World;
use crate::world::folk::promise;

/// What a term is judged on: one measure reader's key, as the promises
/// thread and lane N2 name them (8.4: "his carries inside the 5", "stunts
/// he's in", "the field goal line"). A key, not an enum, so the readers own
/// their list and this contract never repeats it. `who` is the man it's
/// about when it isn't the person you're talking to (ahead of a named
/// teammate, his brother's offer).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Measure {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub who: Option<u32>,
}

/// Over what a term's number is counted (8.4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Scope {
    /// Every game: "at least 1 in 4 of his rushes", judged game by game.
    #[default]
    Game,
    /// Over the season.
    Season,
    /// Once, by a date ("this is good till Friday", an early enrolment).
    By(Date),
    /// Done at once (a scholarship offer, a jersey number).
    Once,
}

/// Which way the number holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Bound {
    /// At least the number.
    #[default]
    AtLeast,
    /// At most ("the kicker won't punt from inside the 40").
    AtMost,
    /// Exactly it (his spot, a jersey number, a role).
    Is,
}

/// One term (8.4).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Term {
    pub measure: Measure,
    /// The number from your words: a count, a share, a yard line, dollars
    /// in the contract's own unit. 0 for a term with no number.
    pub number: f32,
    pub bound: Bound,
    pub scope: Scope,
    /// From when, as P1 counts it (`promise::By`), and for how many
    /// seasons from there (0 this one, as `talks::promise`'s horizon).
    pub from: promise::By,
    pub seasons: u8,
    /// "As long as": the condition on him, P1's (`promise::Cond` with its
    /// stat and bar in `promise::Terms`).
    pub cond: promise::Cond,
    pub cond_stat: promise::Stat,
    pub cond_bar: f32,
    /// Your exact words for it, kept as you said them (3.6).
    pub words: String,
}

/// What a deal is for: what the game does with it once agreed (8.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DealKind {
    /// A promise: logged through P1 (`talks::Promise`).
    #[default]
    Promise,
    /// A scholarship offer.
    Scholarship,
    /// An NIL deal or revenue share, with its clauses.
    Nil,
    /// An NFL contract: years, money, guarantees, bonus, incentives.
    Contract,
    /// An extension, a restructure, a pay cut.
    Restructure,
    /// A trade: players, picks, conditional picks, cap room.
    Trade,
    /// A staff contract: pay, years, title, play-calling, a promotion.
    Staff,
    /// Your own contract, money or goals with your boss.
    Yours,
    /// Access, a say, a building (a booster).
    Booster,
    /// Network favours (a high school coach: a clinic, camp invites).
    Favour,
    /// Outside the rules (`finals.rs`): read as one, with its risk.
    Outside,
}

/// An offer: terms you put together, or he does (11).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Offer {
    pub kind: DealKind,
    pub terms: Vec<Term>,
    /// A deadline on the offer: a real date, or your own ("good till
    /// Friday").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<Date>,
}

/// How he takes an offer, against his best option (11).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Weighed {
    /// Yes (it still asks you to confirm when it does something real,
    /// question 4).
    Yes,
    /// Not yet: his counter.
    Counter(Counter),
    /// He'll take time ("let me talk to my mom"): his decider and his agent
    /// get their say, and he answers by a day.
    Later(Date),
    /// A lowball: no, and it costs warmth.
    Lowball,
    /// No.
    #[default]
    No,
}

/// His counter (11, question 3): built by the game from what he wants most
/// that you haven't given, sized to close the gap. The AI only says it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Counter {
    /// The terms he wants changed or added.
    pub asks: Vec<Term>,
    /// The want behind it.
    pub why: Option<Want>,
}

/// His best option, the thing every deal is valued against (11).
#[derive(Clone, Debug, PartialEq)]
pub enum Best {
    /// Staying as he is.
    Stay,
    /// Another school's case for him (`case.rs`), by program.
    School(usize),
    /// Another club's offer, by club.
    Club(usize),
    /// The portal.
    Portal,
    /// Free agency.
    FreeAgency,
    /// Another job (a coach).
    Job(usize),
    /// Something else, in words.
    Other(String),
}

/// What you ask someone to do (8.3). `about` is the man it involves (the
/// teammate to talk to, the recruit to host).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Request {
    /// Mentor a young player (`ties.rs`).
    Mentor(u32),
    /// Talk to a teammate: the game runs a talk between them.
    TalkTo(u32),
    /// Recruit a friend, bring him on a visit, host a recruit.
    Recruit(u32),
    Host(u32),
    /// Keep something quiet.
    KeepQuiet,
    /// Change position (the position's index), play special teams.
    Position(u8),
    SpecialTeams,
    /// Make weight, go to class, summer school.
    Weight(f32),
    Class,
    /// Tell you when something happens (about a man, or anyone).
    TellMe(Option<u32>),
    /// Report to workouts or camp (a holdout).
    Report,
    /// Take a pay cut, restructure.
    PayCut,
    /// Play hurt: the medical system decides the risk.
    PlayHurt,
    /// Commit, shut it down, enrol early.
    Commit,
    ShutDown,
    EnrolEarly,
    /// A high school coach: steer a kid, open the door, send a kid.
    Steer(u32),
    /// An agent: take an offer to his client, hold off, give a number.
    TakeOffer,
    HoldOff,
    GiveNumber,
    /// A GM: a heads-up, not to our division.
    HeadsUp(Option<u32>),
    NotOurDivision(u32),
    /// A scout: a trip, a file, a question to find out (question 10).
    Scout(u32),
    /// A coordinator: a plan setting, by the plan's own key.
    Plan(String),
    /// An assistant: target or call a recruit.
    Target(u32),
    /// A position coach: work with a player.
    WorkWith(u32),
    /// A doctor: a second opinion, clearing a player.
    SecondOpinion(u32),
    /// Your boss: money, time, a hire, backing with the boosters.
    Money,
    Time,
    Hire(usize),
    Backing,
    /// A booster: fund the collective, a building, back off.
    Fund,
    BackOff,
    /// Compliance or the league: self-report, a rules question, a waiver,
    /// an appeal.
    SelfReport,
    RulesQuestion,
    Waiver(u32),
    Appeal(u32),
    /// A captain: calm a teammate.
    Calm(u32),
    /// Something else, in your words: a talking point only.
    Other(String),
}

/// A decision you tell him (5.1): the game runs the real action once you
/// confirm (question 4), and he takes it by his traits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Decision {
    Benched,
    Starting,
    Suspended,
    Cut,
    Traded,
    OfferPulled,
    CoachLetGo,
    PositionMoved(u8),
    Redshirt,
}

/// Values deals (lane N3).
pub trait Valuing {
    /// His best option today.
    fn best(&self, w: &World, him: Them) -> Best;
    /// How he takes an offer against it.
    fn weigh(&self, w: &World, him: Them, offer: &Offer) -> Weighed;
}
