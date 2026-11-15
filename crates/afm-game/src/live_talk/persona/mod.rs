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
use afm_engine::positions::Position;
use afm_engine::world::folk::persona::Them;
use afm_engine::world::folk::persona::ear::{ActKind, How, Item, Reading};
use afm_engine::world::folk::persona::knows::Topic;
use afm_engine::world::folk::persona::terms::{DealKind, Give, PickIf};
use afm_engine::world::folk::usage::{self, Usage};

use super::card::{Spoken, Tagged};

pub mod calls;
pub mod cards;
pub mod ear;
pub mod talk;
pub mod view;
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
    /// `EarTerm`, the usage reader's labels (`say::usage_schema`).
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
    /// An offer's or a promise's things, and how you'll use him (lane N3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gives: Vec<EarGive>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terms: Vec<EarTerm>,
}

/// A usage term as the ear reads it: the usage reader's labels
/// (`say::usage_schema`), the measure by its tag (m1...) or its key.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct EarTerm {
    pub measure: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<f32>,
    /// "game", "share" or "none".
    pub per: String,
    pub most: bool,
    pub spot: String,
    pub down: u8,
    pub inside: u8,
    pub far: u8,
    pub package: String,
    pub look: String,
    /// Judged on his whole season (lane N2's season terms): an incentive's,
    /// an NIL clause's or a pick's condition.
    pub season: bool,
}

/// The measure keys the ear may name: the usage measures, then lane N2's
/// season-only keys (stat lines, honors, ranks, starts, wins...).
pub fn measure_keys() -> Vec<&'static str> {
    use afm_engine::world::folk::terms;
    let mut v: Vec<&str> = usage::MEASURES.iter().map(|m| m.key).collect();
    let more = terms::LINES
        .iter()
        .map(|l| l.key)
        .chain(terms::HONORS.iter().map(|h| h.0))
        .chain(terms::RANKS.iter().map(|r| r.0))
        .chain(terms::SEASON_KEYS.iter().map(|k| k.0));
    for k in more {
        if !v.contains(&k) {
            v.push(k);
        }
    }
    v.push("none");
    v
}

impl EarTerm {
    /// The term the game can hold you to for a man at `pos`
    /// (`usage::read`): None when the measure doesn't fit his spot, isn't
    /// one the game keeps, or is the other team's call.
    pub fn usage(&self, pos: Position, keys: &EarKeys) -> Option<Usage> {
        let key = keys.measures.iter().find(|m| m.0 == self.measure).map_or(self.measure.as_str(), |m| m.1.as_str());
        if key == "none" {
            return None;
        }
        if self.season {
            return afm_engine::world::folk::terms::read(Some(pos), key, self.number, self.most);
        }
        if usage::SHOWN.contains(&key) {
            return None;
        }
        let share = match self.per.as_str() {
            "game" => Some(false),
            "share" => Some(true),
            _ => None,
        };
        let pkg = crate::live_talk::say::PACKAGES.iter().find(|x| x.0 == self.package).map_or(0, |x| x.1);
        let look = crate::live_talk::say::LOOKS.iter().find(|x| x.0 == self.look).map_or(0, |x| x.1);
        let spot = if self.spot.is_empty() { "any" } else { self.spot.as_str() };
        usage::read(pos, key, self.number, share, self.most, spot, self.down, self.inside, self.far, pkg, look)
    }
}

/// The things a deal can hold (`terms::Give`), by their keys in the ear's
/// schema.
pub const GIVE_KEYS: [&str; 34] = [
    "scholarship",
    "scholarship_for",
    "money",
    "years",
    "guaranteed",
    "start",
    "role",
    "place",
    "position",
    "no_redshirt",
    "jersey",
    "early_enrol",
    "time_away",
    "tutor",
    "staff_job",
    "council",
    "stay",
    "pick",
    "favour",
    "outside",
    // Lane N5b, negotiating.
    "bonus",
    "incentives",
    "void_years",
    "no_trade",
    "player",
    "their_pick",
    "draft",
    "if_games",
    "if_starts",
    "if_pro_bowl",
    "if_snaps",
    "revenue_share",
    "title",
    "play_calling",
];

/// One thing in a deal as the ear reads it: its key, the number you said,
/// and the person it's for by tag (a brother's scholarship).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct EarGive {
    pub give: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub who: Option<String>,
}

impl EarGive {
    /// The game's give, with the person resolved through the keys; None
    /// for a key the schema doesn't hold, or a number it needs and lacks.
    pub fn give(&self, keys: &EarKeys) -> Option<Give> {
        let n = self.number;
        let small = |x: f64| x.round().clamp(0.0, 99.0) as u8;
        let who = self.who.as_ref().and_then(|t| keys.people.iter().find(|p| &p.0 == t)).and_then(|p| match p.1 {
            Them::Player(id) => Some(id),
            _ => None,
        });
        Some(match self.give.as_str() {
            "scholarship" => Give::Scholarship(None),
            "scholarship_for" => Give::Scholarship(Some(who?)),
            "money" => Give::Money(n?.max(0.0)),
            "years" => Give::Years(small(n?).clamp(1, 7)),
            "guaranteed" => Give::Guaranteed(n?.max(0.0)),
            "start" => Give::Role(0),
            "role" => Give::Role(1),
            "place" => Give::Role(2),
            "position" => Give::Position(small(n?)),
            "no_redshirt" => Give::NoRedshirt,
            "jersey" => Give::Jersey(small(n?)),
            "early_enrol" => Give::EarlyEnrol,
            "time_away" => Give::TimeAway,
            "tutor" => Give::Tutor,
            "staff_job" => Give::StaffJob(None),
            "council" => Give::Council,
            "stay" => Give::Stay,
            "pick" => Give::Pick(small(n?).clamp(1, 7)),
            "favour" => Give::Favour,
            "outside" => Give::Outside(n.unwrap_or(0.0).max(0.0)),
            "bonus" => Give::Bonus(n?.max(0.0)),
            "incentives" => Give::Incentives(n?.max(0.0)),
            "void_years" => Give::VoidYears(small(n?).min(4)),
            "no_trade" => Give::NoTrade,
            "player" => Give::Player(who?),
            "their_pick" => Give::TheirPick(small(n?).clamp(1, 7)),
            "draft" => Give::Draft(n?.round() as i32),
            "if_games" => Give::PickIf(PickIf::Games(small(n?))),
            "if_starts" => Give::PickIf(PickIf::Starts(small(n?))),
            "if_pro_bowl" => Give::PickIf(PickIf::ProBowl),
            "if_snaps" => Give::PickIf(PickIf::Snaps(small(n?).clamp(1, 99))),
            "revenue_share" => Give::RevenueShare(n?.max(0.0)),
            "title" => Give::Title,
            "play_calling" => Give::Calling,
            _ => return None,
        })
    }
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
