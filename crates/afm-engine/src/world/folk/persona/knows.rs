//! What a person knows and what he tells you (sections 3.1, 3.2 and 10.1).
//!
//! He knows the world's own facts, filtered by where he stands: a fact
//! reaches him through a `Source` (he lived it, a close friend, he was
//! there, the tape). How sure he is follows his eye for it, the way sources
//! already work: the truth, plus his misread, plus his shade
//! (`scouting::file::source_says`, lane 6), which becomes everyone's rule.
//! What is true about him today comes from his fact sheet (`claims::Sheet`),
//! never from a new reader of the same thing.
//!
//! He answers one of four ways (`Answer`). The game picks it from his
//! trust in you, his professionalism, his loyalty to the man it's about,
//! his own interest and who else is in the room (`TellParts`). Lane N1
//! fills `Knows` and `Tells`.

use super::Them;
use super::memory::HisLie;
use crate::calendar::Date;
use crate::world::World;
use crate::world::folk::claims;
use crate::world::scouting::file::SourceSays;

/// What a question is about: one row of section 3.1's table, plus football
/// (section 9) and "I don't know" for what nobody stores (8.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Topic {
    /// His troubles at home: a birth, a wedding, a parent ill or dying, a
    /// divorce, money trouble, homesick (`folk/life.rs`).
    Life,
    /// His mood and why (`talks::Reason`).
    Mood,
    /// What he wants (`own_wants`, `decider_wants`, the NFL's `goals`).
    Wants,
    /// His portal thinking (`lean_checkpoint`).
    Portal,
    /// A case and whether he did it (`courts`, `Point::hid`).
    Case,
    /// A rule break, seen or unseen (`folk/rules.rs`, `memory::Unseen`).
    RuleBreak,
    /// Effort and practice (practice battles and grades).
    Effort,
    /// An injury as he feels it, or the diagnosis.
    Injury,
    /// What schools told a recruit (`case.rs`).
    Pitches,
    /// A recruit's NIL offers (`offer_for`, `portal_nil`).
    Nil,
    /// His grades (`ncaa_tier`).
    Grades,
    /// His character (`scouting/own.rs`, `file.rs`, `source_says`).
    Character,
    /// An opponent's habits: its calls, its checks, its plan.
    Opponent,
    /// A pro's market (`pro/market.rs`).
    Market,
    /// A club's needs, cap and grades (`pro/office.rs`).
    Club,
    /// A scout's file (`scouting/file.rs`).
    ScoutFile,
    /// Jobs opening and who's leaving (the carousel).
    Jobs,
    /// Your boss's goals and trust in you (`boss.rs`).
    Goals,
    /// The budget and the boosters' moods (`finance/college.rs`,
    /// `booster_push`).
    Money,
    /// Rules and open cases (`compliance.rs`, `calendar/rules.rs`).
    Rules,
    /// Who told whom what (`memory::Heard`).
    Grapevine,
    /// What he told you before, and whether it was true (`memory::HisLie`).
    HisWord,
    /// Football at his level (section 9): his own snaps, the play log, the
    /// scheme.
    Football,
    /// His life away from football, the small talk (personal talk,
    /// `home.rs`): home, family, where he's from, school, his day off, his
    /// kids. Open to anyone, in layers by how far he opens up.
    Home,
    /// Something nobody in the world stores: "I don't know", or his own
    /// opinion if he'd have one.
    Nothing,
    /// His own job here: how safe it is and his case for keeping it (a
    /// coach of yours: his years, the team since he came, his unit, his
    /// deal; your boss: where you stand with him). Before it, "why should
    /// I keep you?" went to `Jobs`, the openings elsewhere (Cain
    /// 2026-11-16).
    Job,
}

/// How a fact reached him (3.1's "who knows it").
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Source {
    /// It's his own: he lived it, he plans it, he feels it.
    Himself,
    /// His family.
    Family,
    /// A close friend (a tie of 35 and up): the mood, and the why by tie.
    Friend,
    /// He was there: a teammate at the rule break, his unit at practice.
    There,
    /// He sees the room as its leader (a captain).
    Room,
    /// His job shows it: a trainer's diagnosis, a scout's file, a GM's
    /// board, the compliance office's cases.
    Job,
    /// The tape or the play log.
    Tape,
    /// He was at the other team (a transfer, a cut player, a coach who
    /// left).
    WasThere,
    /// Someone told him (`memory::Heard`), with its hops.
    Grapevine,
    /// The public story (the news, a public step in a case).
    Public,
}

/// One thing he knows today, as the game hands it to `Tells`. Built fresh
/// for each question, never saved.
#[derive(Clone, Debug)]
pub struct Known {
    pub topic: Topic,
    /// The man it's about, when it isn't him.
    pub about: Option<Them>,
    pub from: Source,
    /// His eye for it, 1-20, as `source_says` reads it: a teammate's is his
    /// Football IQ and his years with the man, a high school coach's his
    /// recruiting rating, a GM's his judgment.
    pub eye: f64,
    /// The fact sheet's point it is, when it is one (`claims::Kind`).
    pub point: Option<claims::Kind>,
    /// A trait read, when it is one: `source_says`' own answer.
    pub says: Vec<SourceSays>,
    /// The fact as he holds it, in words the voice may use once released:
    /// the truth, his misread and his shade already in it.
    pub words: String,
    /// How sure he is, 0-1.
    pub sure: f32,
    /// The day it happened, or the day it comes.
    pub day: Option<Date>,
}

/// The four ways he answers (3.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Answer {
    /// All of it.
    Straight,
    /// The safe half.
    Partly,
    /// "Ask him yourself."
    #[default]
    Deflects,
    /// Something false: it goes in his lies store.
    Lies,
}

/// What weighs on how he answers (3.2). Engine numbers, never shown.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TellParts {
    /// His trust in you, 0-1.
    pub trust: f64,
    /// His professionalism, 1-20.
    pub professionalism: f64,
    /// His loyalty to the man it's about, 0-1 (0 when it's about himself).
    pub loyalty: f64,
    /// His own stake in the answer, -1 to 1: a recruit overselling an offer
    /// for leverage, an agent selling his client.
    pub interest: f64,
    /// Who else hears it, 0 alone to 1 a room (`room::Room`).
    pub room: f64,
    /// How many times you've pushed him on this man in this talk.
    pub pushed: u8,
}

/// What he tells you, as the game decided it. The voice gets `released`
/// and `holding_back`'s tell, never what he kept.
#[derive(Clone, Debug, Default)]
pub struct Told {
    pub answer: Answer,
    /// What the game released, in words for the voice.
    pub released: Vec<String>,
    /// He held something back: the voice says so through `tell`, never the
    /// secret.
    pub holding_back: bool,
    /// The tell your read caught (`went_words`, the talk's tells).
    pub tell: String,
    /// A lie he told: the game logs it in his store with the day you could
    /// find out.
    pub lie: Option<HisLie>,
    /// What asking cost: minutes, and trust when you ground him on a
    /// teammate.
    pub cost: AskCost,
}

/// What a question costs (3.2).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AskCost {
    pub minutes: f32,
    /// Trust lost, 0 or more.
    pub trust: f32,
}

/// What a person knows (lane N1).
pub trait Knows {
    /// Everything he knows on a topic, about a man or about himself, today.
    fn knows(&self, w: &World, him: Them, topic: Topic, about: Option<Them>) -> Vec<Known>;
}

/// How he tells it (lane N1).
pub trait Tells {
    /// His answer to one question, from what he knows and what weighs on it.
    fn tell(&self, w: &World, him: Them, known: &[Known], parts: &TellParts) -> Told;
}
