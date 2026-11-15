//! Live talks v3's cards and the ear's JSON (`docs/design/live-talks-persona.md`,
//! sections 3.7, 5 and 14): the game side of the contract whose world side
//! is `afm_engine::world::folk::persona`. Types, trait stubs and fakes only;
//! lane N4 fills them.
//!
//! Two calls a line. The ear gets an `EarCard` (his real items tagged c1,
//! c2..., the people p1, p2..., the measures m1, m2..., and your line) and
//! answers an `EarReply` in tags. `Resolve` turns the tags back into the
//! game's own people and items through `EarKeys`, which never leave the
//! game, giving the engine's `Reading`. The game resolves it into a `Turn`.
//! The voice gets a `VoiceCard` filled from the turn and answers as him;
//! his words pass `filter::check` before they reach the screen.
//!
//! Both cards follow `card.rs`'s rule: a whitelist of words a seat may see,
//! with no field for potential, true ratings, hidden numbers or the depth
//! chart. They reuse `card::Tagged` and `card::Spoken`. A call that fails
//! is `afm_ai::Failure`, and the talk plays on as the fallbacks in
//! `say/mod.rs` already do.

use afm_ai::Failure;
use afm_engine::world::folk::persona::Them;
use afm_engine::world::folk::persona::ear::{ActKind, How, Item, Reading};
use afm_engine::world::folk::persona::knows::Topic;
use afm_engine::world::folk::persona::terms::DealKind;

use super::card::{Spoken, Tagged};

pub mod cards;
pub mod ear;
pub mod talk;
pub mod voice;

/// What the ear is given: everything as words and tags.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EarCard {
    /// The meeting: what, where, when, at what level, by what channel.
    pub setting: String,
    /// You.
    pub coach: String,
    /// The man across: name, role, class year.
    pub them: String,
    /// Everyone in the room and why, tagged p1, p2... (the man across is
    /// p1), and anyone else your line may name.
    pub people: Vec<Tagged>,
    /// His real items your words can touch, tagged c1, c2...: his wants,
    /// his worries from the fact sheet, his pride, your promises to him,
    /// his friends, what he told you before. A hit must name one.
    pub items: Vec<Tagged>,
    /// The measures a term can be judged on (`usage::MEASURES`), tagged
    /// m1, m2... (cached with the rules). The ear's term shape is
    /// `usage::Usage`'s and joins once that file is on main.
    pub measures: Vec<Tagged>,
    /// His ask or counter on the table, in words.
    pub on_table: Vec<String>,
    pub so_far: Vec<Spoken>,
    /// Your line, quoted (`card::quoted`).
    pub line: String,
}

/// The ear's tags and what they stand for. Never sent.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EarKeys {
    pub people: Vec<(String, Them)>,
    pub items: Vec<(String, Item)>,
    /// A measure's tag and its key in `usage::MEASURES`.
    pub measures: Vec<(String, String)>,
}

/// One act as the ear reads it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EarAct {
    pub act: ActKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<Topic>,
    /// The person it's about, by tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// A claim's kind (`claims::truth::CLAIM_KINDS`) and value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub value: String,
    /// An offer's kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deal: Option<DealKind>,
    /// A request, a decision, a show or an invitation, by its name in the
    /// schema.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub what: String,
    /// A date in your words (a deadline).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub when: String,
    /// The words of the act, short.
    #[serde(default)]
    pub words: String,
}

/// One hit as the ear reads it: an item's tag, how, and on whom (a
/// person's tag; none is the man across).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EarHit {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<String>,
    pub item: String,
    pub how: How,
}

/// The ear's answer, as its JSON schema shapes it.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct EarReply {
    pub acts: Vec<EarAct>,
    pub hits: Vec<EarHit>,
    pub dodged: bool,
    pub not_talk: bool,
}

/// What the voice is given (3.7): who he is as behaviour, where, and what
/// the game released this turn. Words only.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VoiceCard {
    /// The meeting, the place and the channel (the setting changes how he
    /// talks).
    pub setting: String,
    pub coach: String,
    /// Him: name, role, class year.
    pub them: String,
    /// His age, schooling and where he's from, his family's money.
    pub background: String,
    /// His personality as behaviour, in bands ("proud, needs to hear he
    /// matters", "hot-headed", "a pro").
    pub personality: Vec<String>,
    /// His mood, in words.
    pub mood: String,
    /// His football: his Football IQ in words and his years in the game
    /// (section 9).
    pub football: String,
    /// How long he's known you, and how it's gone.
    pub known_you: String,
    /// Who else is in the room and how they sit.
    pub room: Vec<String>,
    /// He'd swear, in this setting.
    pub swears: bool,
    /// A hard topic (a death in his family, an arrest): voiced with care.
    pub care: bool,
    /// The only people and teams he may name.
    pub may_name: Vec<Tagged>,
    /// What the game released this turn (`ear::Say::released`).
    pub released: Vec<String>,
    /// His stance on a hard topic, in words.
    pub stance: String,
    /// The tell your read caught.
    pub tell: String,
    /// He's holding something back (never the secret).
    pub holding_back: bool,
    /// The lie the game chose for him, when he lies.
    pub lie: String,
    /// His counter and his own asks, in words.
    pub asks: Vec<String>,
    /// How hot it is, in words, and what he does about it.
    pub heat: String,
    /// His colour notebook (`memory::Colour`): details he said before.
    pub notebook: Vec<String>,
    pub so_far: Vec<Spoken>,
    /// Your line, quoted.
    pub line: String,
}

/// The ear (lane N4). The real one runs through `tickets.rs` beside the
/// game; no call runs inside a command.
pub trait Ear {
    fn read(&self, card: &EarCard) -> Result<EarReply, Failure>;
}

/// Tags back to the game's people and items (lane N4). A tag the card
/// didn't hold drops the act or the hit (section 14: no invented facts).
pub trait Resolve {
    fn resolve(&self, reply: &EarReply, keys: &EarKeys) -> Reading;
}

/// The voice (lane N4): his reply, before `filter::check`.
pub trait Voice {
    fn say(&self, card: &VoiceCard) -> Result<String, Failure>;
}

/// A fake ear: hands back its reply, or fails.
#[derive(Clone, Debug, Default)]
pub struct FakeEar(pub Option<EarReply>);

impl Ear for FakeEar {
    fn read(&self, _: &EarCard) -> Result<EarReply, Failure> {
        self.0.clone().ok_or(Failure::NoKey)
    }
}

/// A fake resolver: hands back its reading.
#[derive(Clone, Debug, Default)]
pub struct FakeResolve(pub Reading);

impl Resolve for FakeResolve {
    fn resolve(&self, _: &EarReply, _: &EarKeys) -> Reading {
        self.0.clone()
    }
}

/// A fake voice: hands back its line, or fails.
#[derive(Clone, Debug, Default)]
pub struct FakeVoice(pub Option<String>);

impl Voice for FakeVoice {
    fn say(&self, _: &VoiceCard) -> Result<String, Failure> {
        self.0.clone().ok_or(Failure::NoKey)
    }
}

#[cfg(test)]
mod tests;
