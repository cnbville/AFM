//! Offers, counters, requests and decisions (sections 8.2, 8.3 and 11).
//! Lane N3 values them (`Valuing`).
//!
//! A term (8.4: a measure, a number from your words, a scope, a horizon and
//! "as long as") is the promises lane's `usage::Usage` with its `MEASURES`
//! keys, never a second type here. It sits on `Offer`, `Counter` (and so
//! `wants::Willing::If`), `ear::Act::Promise` and `ear::HisAsk` (lane N3).
//! The rest of a deal (8.2's off-field things: a scholarship, money, years,
//! a role, a jersey) is a `Give`. A promise's own record stays P1's
//! `talks::Promise`. Lane N3 weighs deals in `deals.rs` and runs requests
//! in `asks.rs`.

use super::Them;
use super::wants::Want;
use crate::calendar::Date;
use crate::world::World;
use crate::world::folk::usage::Usage;

/// One thing in a deal that isn't a usage term (8.2), as the game can do
/// or judge it. Money is in dollars a year: an engine number, never shown
/// as a figure unless you named it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Give {
    /// A scholarship: for him (None), or for someone close to him (his
    /// brother, a friend: by id).
    Scholarship(Option<u32>),
    /// Money a year: NIL, revenue share, a contract's average.
    Money(f64),
    /// Years on a contract.
    Years(u8),
    /// Guaranteed money, in all.
    Guaranteed(f64),
    /// A role: 0 a start, 1 a real role, 2 a place on the team.
    Role(u8),
    /// A position (its index): he plays there, or stays there.
    Position(u8),
    /// No redshirt, an early chance.
    NoRedshirt,
    /// A jersey number.
    Jersey(u8),
    EarlyEnrol,
    TimeAway,
    Tutor,
    /// A staff job: for his high school coach, or a coach (a spot by the
    /// staff's own index when known).
    StaffJob(Option<usize>),
    /// A seat on the leadership council.
    Council,
    /// Your promise to stay.
    Stay,
    /// A draft pick by round; a condition is a usage term on the offer.
    Pick(u8),
    /// A clinic, camp invites, a look at his kids (a high school coach).
    Favour,
    /// Money outside the rules (`finals.rs`): read as one, with its risk.
    Outside(f64),
    /// A coach's title: co-coordinator, assistant head coach (lane N5c,
    /// `hiring::PROMISE_TITLE`).
    Title,
    /// A coordinator calls his side's plays (lane N5c,
    /// `hiring::PROMISE_CALLING`).
    Calling,
}

impl Give {
    /// It's a promise: logged through P1 and believed as one, not done at
    /// once.
    pub fn promised(&self) -> bool {
        matches!(self, Give::Role(_) | Give::Position(_) | Give::NoRedshirt | Give::Stay)
    }

    /// In words for the voice and the confirm card (no figure unless the
    /// words carry yours).
    pub fn words(&self) -> String {
        match self {
            Give::Scholarship(None) => "a scholarship".into(),
            Give::Scholarship(Some(_)) => "a scholarship for someone close to him".into(),
            Give::Money(_) => "more money".into(),
            Give::Years(y) => format!("{y} years"),
            Give::Guaranteed(_) => "more of it guaranteed".into(),
            Give::Role(0) => "a starting job".into(),
            Give::Role(1) => "a real role".into(),
            Give::Role(_) => "a place on the team".into(),
            Give::Position(p) => format!(
                "a spot at {}",
                crate::world::folk::claims::words::spot(
                    crate::positions::Position::ALL[*p as usize % crate::positions::POSITION_COUNT]
                )
            ),
            Give::NoRedshirt => "no redshirt".into(),
            Give::Jersey(n) => format!("number {n}"),
            Give::EarlyEnrol => "early enrolment".into(),
            Give::TimeAway => "time away".into(),
            Give::Tutor => "a tutor".into(),
            Give::StaffJob(_) => "a staff job".into(),
            Give::Council => "a seat on the leadership council".into(),
            Give::Stay => "your word that you'll stay".into(),
            Give::Pick(r) => format!("a round {r} pick"),
            Give::Favour => "a clinic and camp invites".into(),
            Give::Outside(_) => "money outside the rules".into(),
            Give::Title => "a better title".into(),
            Give::Calling => "the play-calling for his side".into(),
        }
    }
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
    /// Your exact words for it (3.6).
    pub words: String,
    /// What's in it beyond usage terms (8.2).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub gives: Vec<Give>,
    /// How you'll use him (8.4): judged on the play log through P1.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub terms: Vec<Usage>,
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
    /// What he wants changed or added, in words for the voice.
    pub asks: Vec<String>,
    /// The same as things the game can do: what he wants given.
    pub gives: Vec<Give>,
    /// And how he wants to be used.
    pub terms: Vec<Usage>,
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
    /// Your scout or security director: find this out about him (lane N5e).
    FindOut(u32, super::knows::Topic),
    /// A prospect in your interview room: go to the whiteboard (lane N5e).
    Whiteboard,
    /// A GM or an agent: plant word that you like this man (lane N5e).
    Plant(u32),
    /// A coordinator: a plan setting, by the plan's own key.
    Plan(String),
    /// An assistant: target or call a recruit.
    Target(u32),
    /// A position coach: work with a player.
    WorkWith(u32),
    /// A doctor: a second opinion on a player's injury.
    SecondOpinion(u32),
    /// A doctor: clear a hurt player to play sooner than the timeline
    /// (question 10, lane N5c): the pressure to clear a man.
    Clear(u32),
    /// Your boss: money (what for, in your words: the staff pool, recruiting,
    /// the collective, a facility), time, a hire, backing with the boosters.
    Money(String),
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
