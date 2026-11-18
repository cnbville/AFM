//! The ear's reading and the game's turn (sections 5.1 to 5.4).
//!
//! Two calls a line (agreed 2026-11-14): the ear reads your line, the game
//! decides, the voice answers as him. The ear's `Reading` is everything the
//! game acts on: the acts in your line (a fixed list, because the game must
//! do something with each; what's inside each is open) and the hits (which
//! of his real things your words touched, and how). Every hit names an item
//! on his card (`Item`): a hit that names nothing real counts for nothing.
//! The reading is saved with the talk (`memory::Line`), so a reload plays
//! the same world.
//!
//! The game's `Turn` (5.3) resolves every act and hit: what he says back,
//! his counter and his own asks, the heat, whether he ends the talk. The
//! voice says only what's on it. Lane N4 fills `Resolving`.

use super::Them;
use super::knows::{Told, Topic};
use super::memory::HisLie;
use super::room::Seat;
use super::terms::{Counter, Decision, Offer, Request, Weighed};
use super::wants::{Boil, Heat, Patience, Ran, Want, Willing};
use crate::calendar::Date;
use crate::world::World;
use crate::world::folk::claims::{Stated, Truth};
use crate::world::folk::talk_points::Stance;

/// The kinds of act (5.1), without what's inside: the ear's schema lists
/// these, so its cache holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActKind {
    Question,
    Claim,
    Offer,
    Promise,
    Request,
    Decision,
    Threat,
    Deadline,
    Apology,
    Confide,
    KeepQuiet,
    Invite,
    BringIn,
    AskToLeave,
    Show,
    Answer,
    End,
    /// Your word on his future (the four words of `folk::future`).
    Future,
    /// A change to a promise you made him.
    Amend,
    /// A commitment to someone you answer to: a goal (`folk::commit`).
    Commit,
    /// You tell him something about your own life away from football
    /// (personal talk, item 6).
    Opens,
    /// A hello, "nice to meet you", "how you doing", welcome: the small
    /// talk people open with (personal talk, first meetings).
    Greet,
}

/// What you can show (5.1): only real things.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Show {
    /// Film of him, of your scheme, of a player like him.
    Film,
    /// Numbers: his, your team's, your alumni's.
    Numbers,
    /// The facility.
    Facility,
    /// Your alumni at his spot (`claims::Alumni`).
    Alumni,
}

/// An invitation (5.1), by the calendar's rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Invite {
    Visit,
    Dinner,
    /// A meeting with someone.
    Meet(Them),
}

/// How you answer his ask or counter.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Reply {
    Accept,
    Counter(Offer),
    Refuse,
}

/// One act in your line, with what's inside it, resolved to the game's own
/// people and items by the card's keys.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Act {
    /// A question: what he knows and what he'll say (3.1, 3.2).
    Question {
        topic: Topic,
        about: Option<Them>,
        words: String,
    },
    /// A claim: checked against the world (lanes 3, 9 and 10's fact checks).
    Claim(Stated),
    /// An offer: valued against his best option (11).
    Offer(Offer),
    /// A promise: logged with its terms through P1, his belief decided.
    Promise {
        words: String,
        /// How you'll use him, when your words name a measure.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        terms: Vec<crate::world::folk::usage::Usage>,
    },
    /// A request: whether he'll do it, then the game runs it (8.3).
    Request(Request),
    /// Telling him a decision: run once you confirm (question 4).
    Decision(Decision),
    /// A threat or an ultimatum: weighed by your leverage and his temper.
    Threat {
        ultimatum: bool,
        words: String,
    },
    /// A deadline on an offer.
    Deadline(Date),
    /// An apology: repairs trust by how real it is (you keep the next one).
    Apology {
        for_promise: Option<u32>,
        words: String,
    },
    /// You tell him something private.
    Confide(String),
    /// Asking him to keep quiet.
    KeepQuiet(Option<Them>),
    Invite(Invite),
    /// Bringing someone in, or asking someone to leave (5.6).
    BringIn(Them),
    AskToLeave(Them),
    Show(Show),
    /// Answering his ask.
    Answer(Reply),
    End,
    /// Your word on his future (playtest bug 15, `folk::future`): a role
    /// backed by a strength of his you named (`strength`, a key of
    /// `future::STRENGTHS`, empty for none), or one of the three ways to
    /// tell him to go.
    Future {
        word: crate::world::folk::future::Word,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        strength: String,
    },
    /// A change to a promise you made him (the promises design, sections
    /// 5 and 6).
    Amend(Amend),
    /// A commitment to someone you answer to: a goal he can hold you to
    /// (`folk::commit`).
    Commit(crate::world::folk::commit::Commitment),
    /// You tell him something about your own life away from football:
    /// where you grew up, your family, your own story (personal talk, item
    /// 6), in a few words.
    Opens(String),
    /// A hello or a little small talk before anything else: never a dodge
    /// of his question (personal talk, first meetings).
    Greet,
}

/// How a line changes a promise you made him.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmendHow {
    /// "Forget I said it."
    TakeBack,
    /// A later date.
    Later,
    /// A condition on it.
    Condition,
    /// The real reason it hasn't happened.
    Reason,
    /// Putting it in writing.
    Writing,
}

/// A change to a promise: which one (P1's index; none for the latest open
/// one), how, and what the line named for it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Amend {
    #[serde(default)]
    pub promise: Option<u32>,
    pub how: AmendHow,
    /// The new date, in the promise reader's words (`midseason`,
    /// `next_season`), empty for none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub by: String,
    /// The condition's kind (lane 9's list), empty for none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub condition: String,
    /// A stat bar you named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar: Option<f32>,
}

impl Act {
    pub fn kind(&self) -> ActKind {
        match self {
            Act::Question { .. } => ActKind::Question,
            Act::Claim(_) => ActKind::Claim,
            Act::Offer(_) => ActKind::Offer,
            Act::Promise { .. } => ActKind::Promise,
            Act::Commit(_) => ActKind::Commit,
            Act::Request(_) => ActKind::Request,
            Act::Decision(_) => ActKind::Decision,
            Act::Threat { .. } => ActKind::Threat,
            Act::Deadline(_) => ActKind::Deadline,
            Act::Apology { .. } => ActKind::Apology,
            Act::Confide(_) => ActKind::Confide,
            Act::KeepQuiet(_) => ActKind::KeepQuiet,
            Act::Invite(_) => ActKind::Invite,
            Act::BringIn(_) => ActKind::BringIn,
            Act::AskToLeave(_) => ActKind::AskToLeave,
            Act::Show(_) => ActKind::Show,
            Act::Answer(_) => ActKind::Answer,
            Act::End => ActKind::End,
            Act::Future { .. } => ActKind::Future,
            Act::Amend(_) => ActKind::Amend,
            Act::Opens(_) => ActKind::Opens,
            Act::Greet => ActKind::Greet,
        }
    }
}

/// A real thing on his card a hit can touch (5.2).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Item {
    /// One of his wants (`wants::WantKind`'s place in his list).
    Want(u8),
    /// A worry: a point on his fact sheet by its key (`claims::Kind::key`).
    Point(String),
    /// His pride.
    Pride,
    /// A promise you made him (P1's index in `Folk.promises`).
    Promise(u32),
    /// A friend or a teammate.
    Friend(u32),
    /// His family.
    Family,
    /// Something he told you before.
    HisWord,
    /// His life away from football, and what he has told you of it
    /// (personal talk): a kind question, you remembering, you opening up.
    Life,
    /// The team as a whole: a line to the room in a group talk (lane N5d),
    /// heard by each man as one of the room.
    Room,
}

/// How your words landed on it (5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum How {
    Reassured,
    Respected,
    Stung,
    Insulted,
    Doubted,
    Flattered,
    Threatened,
    /// You staked yourself on him, said straight ("you're my guy, you're
    /// going to break records for us"): a commitment he weighs by how much
    /// he believes you (agreed 2026-11-15; `talk::hits::stake_belief`).
    Staked,
}

/// One hit: which real item, how, and on whom in the room.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Hit {
    pub on: Them,
    pub item: Item,
    pub how: How,
}

/// The ear's reading of one line, resolved and saved.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Reading {
    pub acts: Vec<Act>,
    pub hits: Vec<Hit>,
    /// You dodged his question (5.4): a hit of its own.
    pub dodged: bool,
    /// The line isn't a person talking (section 14): it does nothing.
    pub not_talk: bool,
    /// How you said it (coach voice, section 1): none from a reading saved
    /// before it, or when the ear gave none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manner: Option<super::manner::Manner>,
}

/// A hit as the game sized it, from how much the item weighs for him and
/// his traits, inside today's caps (section 14, question 1).
#[derive(Clone, Debug, PartialEq)]
pub struct Sized {
    pub hit: Hit,
    /// Trust and mood moved, signed.
    pub trust: f32,
    pub mood: f32,
    /// A repeat that faded (`Folk.talked`).
    pub faded: bool,
}

/// What the game asks you to confirm before it does something real
/// (question 4).
#[derive(Clone, Debug, PartialEq)]
pub enum Confirm {
    Offer(Offer),
    Decision(Decision),
    Request(Request),
}

/// One of his own asks this turn (5.4): built from his wants.
#[derive(Clone, Debug, PartialEq)]
pub struct HisAsk {
    pub why: Want,
    /// A question he asks you ("who's ahead of me at corner?").
    pub question: Option<Topic>,
    /// What he asks for, in words.
    pub asks: Vec<String>,
    /// The same as things the game can do, and how he wants to be used.
    pub gives: Vec<super::terms::Give>,
    pub terms: Vec<crate::world::folk::usage::Usage>,
}

/// How the talk ends this turn, if it does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ends {
    /// You ended it.
    You,
    /// He ended it, or the heat boiled over.
    Him(Option<Boil>),
    /// Its time ran out.
    Time,
}

/// What the voice may say this turn, in words (the voice card's fill).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Say {
    /// Everything the game released: answers, his counter and asks, his
    /// take on a decision, his report on a request.
    pub released: Vec<String>,
    /// His stance on a hard topic (`talk_points::Stance`).
    pub stance: Option<Stance>,
    /// The tell your read caught.
    pub tell: String,
    /// He's holding something back.
    pub holding_back: bool,
    /// When he lies, the voice gets the lie, never the truth.
    pub lie: Option<String>,
    /// What the coach just did, in the man's own terms, for the voice to
    /// react to (a decision on him, what he showed him): never words the
    /// voice says as its own.
    pub heard: Vec<String>,
}

/// The game's turn (5.3).
#[derive(Clone, Debug, Default)]
pub struct Turn {
    /// One answer per question, in order.
    pub told: Vec<Told>,
    /// Each claim and what checking it found.
    pub claims: Vec<(Stated, Truth)>,
    /// Each offer and how he took it.
    pub weighed: Vec<(Offer, Weighed)>,
    /// Promises logged this turn (P1's indexes).
    pub promised: Vec<u32>,
    /// Each request and his answer, and what ran.
    pub asked: Vec<(Request, Willing, Option<Ran>)>,
    /// His own counter beyond an offer's.
    pub counter: Option<Counter>,
    pub his_asks: Vec<HisAsk>,
    pub hits: Vec<Sized>,
    /// What waits on your confirm.
    pub confirm: Vec<Confirm>,
    /// Lies he told this turn, for his store.
    pub lies: Vec<HisLie>,
    /// Who answers: the man across, or someone who speaks for him.
    pub speaker: Option<(Them, Seat)>,
    pub heat: Heat,
    pub patience: Patience,
    pub ends: Option<Ends>,
    /// Minutes the line took.
    pub minutes: f32,
    pub say: Say,
    /// Your word on his future, decided (`folk::future::plan`).
    pub future: Vec<crate::world::folk::future::Plan>,
    /// Promises your line changes: (P1's index, the answer it is, a stat
    /// bar you named), made real through `talks::rework`.
    pub reworked: Vec<(u32, crate::world::folk::talks::PAct, Option<f32>)>,
    /// You didn't answer the question his last line asked (5.4).
    pub dodged: bool,
    /// Each commitment and how he took it (`folk::commit`).
    pub committed: Vec<(crate::world::folk::commit::Commitment, crate::world::folk::commit::Took)>,
    /// How each man in the room took your manner (coach voice), sized.
    pub manner: Vec<(Them, super::manner::Took)>,
    /// What he told you about his life for the first time this line
    /// (personal talk, item 7): by kind, with its words.
    pub shared: Vec<(u16, String)>,
    /// Something personal landed this line (item 8).
    pub personal: bool,
    /// He went short on his troubles, asked too soon.
    pub closed: bool,
}

/// The game's side of a line (lane N4).
pub trait Resolving {
    /// Resolves every act and hit of one line, without changing the world.
    fn turn(&self, w: &World, room: &super::room::Room, reading: &Reading) -> Turn;
    /// Makes the turn real: stores, trust, promises, the actions you
    /// confirmed.
    fn apply(&self, w: &mut World, turn: &Turn);
}
