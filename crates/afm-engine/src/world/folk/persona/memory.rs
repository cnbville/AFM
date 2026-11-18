//! What people remember (sections 3.6 and 10.2 to 10.5): the new stores.
//! They live in `claims::Live` (`Folk.live.persona`), the live talks' one
//! saved store, so no lane adds a field to `Folk`. Every list here is empty
//! until a lane fills it, and an empty `Memory` is never written, so a save
//! from before this contract loads and writes the same.
//!
//! Kept elsewhere and not repeated here: what you talked through
//! (`Folk.talked`), your promises (`talks::Promise`, P1), your own lies and
//! trust in your word (`claims::Live`, `lies` and `word`), a seen rule break
//! (`folk/rules.rs`).

use super::Them;
use super::ear::Reading;
use super::knows::Topic;
use super::terms::Offer;
use crate::calendar::Date;
use crate::world::folk::rules::RuleKind;

/// A lie he told you (10.2): a recruit's inflated offer, an agent's bluff,
/// a player covering for a friend, a prospect hiding his case, a GM's
/// smoke. Kept with the day you could find out; some are never found.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HisLie {
    pub by: Them,
    pub topic: Topic,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<Them>,
    pub said: Date,
    /// The first day another source, the facts or the tape could show it;
    /// none when nothing ever will.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shows: Option<Date>,
    /// What he said, in words.
    pub words: String,
    /// You caught it.
    #[serde(default)]
    pub caught: bool,
}

/// What travelled (10.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Passed {
    /// Something you said about a man ("call a teammate soft and he hears
    /// it").
    YourWords,
    /// A promise you made someone else (P1's index in `Folk.promises`).
    Promise(u32),
    /// Something told "between us" (`Secret`).
    Secret(u32),
    /// A rumour: a failed trade talk, a talk that leaked.
    Rumour,
}

/// One hop on the grapevine (10.5): who told whom what, and when. It goes
/// through ties by the listener's loyalty, temper and professionalism.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Heard {
    pub from: Them,
    pub to: Them,
    pub what: Passed,
    pub day: Date,
    /// Hops from you: 1 when he heard it from the man you told.
    pub hops: u8,
    /// It's about this man.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<Them>,
    /// What he heard, in words, with each hop's shade in it.
    pub words: String,
}

/// A small detail the voice added that no system reads (3.6, question 5):
/// his dog's name, the spot back home where he trains. Only the voice reads
/// it, so he stays the same person.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Colour {
    pub who: Them,
    pub day: Date,
    pub note: String,
}

/// Something about a life told in a talk (personal talk, item 7): by him
/// to you (`yours` false; `key` is the thing's kind, `home::Bit::key`), or
/// by you to him (`yours`; your words, short).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Shared {
    pub who: Them,
    pub day: Date,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub yours: bool,
    #[serde(default)]
    pub key: u16,
    pub words: String,
}

/// A rule break nobody on your staff saw (3.1): teammates who were there
/// know it. A seen one stays in `folk/rules.rs`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Unseen {
    pub player: u32,
    pub rule: RuleKind,
    pub day: Date,
    /// The teammates who were there.
    pub there: Vec<u32>,
    /// It reached your staff later (a teammate told, you asked).
    #[serde(default)]
    pub found: bool,
}

/// How you treated him (3.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Treated {
    /// A talk that went hot.
    Hot,
    /// A deal you lowballed.
    Lowballed,
    /// A request you made.
    Asked,
    /// He came to you and you ignored it (question 12).
    Ignored,
    /// You dodged his question.
    Dodged,
    /// You apologised.
    Apologised,
    /// You staked yourself on him (a stake made again soon counts less).
    Staked,
}

/// One thing he remembers about how you treated him.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Treatment {
    pub who: Them,
    pub how: Treated,
    pub day: Date,
}

/// Something said "between us" (10.3), either way.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Secret {
    /// The one who told it.
    pub by: Them,
    /// You told him (you confided); otherwise he told you.
    pub yours: bool,
    pub to: Them,
    pub day: Date,
    pub words: String,
    /// You asked him to keep it quiet.
    #[serde(default)]
    pub asked_quiet: bool,
    /// It was used where he could see it, or it got out.
    #[serde(default)]
    pub out: bool,
}

/// A deal agreed in a talk, with your exact words (3.6). Its promise
/// terms log through P1 as well.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Agreed {
    pub with: Them,
    pub day: Date,
    pub offer: Offer,
}

/// An answer he owes you (section 11, taking time): "let me talk to my
/// mom". The game weighs the offer again on the day, with his decider's
/// and his agent's say (lane N3, `deals::due`).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Owed {
    pub with: Them,
    /// The day you made it, and the day he answers by.
    pub made: Date,
    pub by: Date,
    pub offer: Offer,
}

/// The first number you put on the table with him (section 11, anchors):
/// it moves what he asks for, a little. Kept a season.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Anchor {
    pub with: Them,
    pub day: Date,
    /// Money a year, in dollars.
    pub money: f64,
}

/// Someone who agreed to tell you when something happens (8.3): about one
/// man, or anyone (lane N3; N1's unseen breaks read it through
/// `asks::tells_you`).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Watch {
    pub by: Them,
    pub about: Option<u32>,
    pub day: Date,
}

/// One saved line of a talk: your words, the ear's reading and his reply,
/// so a reload plays the same world (section 14, determinism).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Line {
    pub you: String,
    pub reading: Reading,
    pub his: String,
}

/// One saved talk.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Saved {
    pub with: Them,
    pub day: Date,
    pub lines: Vec<Line>,
}

/// The new stores, kept in `claims::Live`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Memory {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub his_lies: Vec<HisLie>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub grapevine: Vec<Heard>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notebook: Vec<Colour>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unseen: Vec<Unseen>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub treated: Vec<Treatment>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<Secret>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub agreed: Vec<Agreed>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub talks: Vec<Saved>,
    /// The talks open now (lane N4, `talk::store::Open`): who's in the
    /// room, the heat, his patience.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub open: Vec<super::talk::store::Open>,
    /// Answers owed, anchors and watchers (lane N3).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub owed: Vec<Owed>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub anchors: Vec<Anchor>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub watch: Vec<Watch>,
    /// The draft side's stores (lane N5e, `draft::Book`).
    #[serde(skip_serializing_if = "super::draft::Book::is_empty")]
    pub draft: super::draft::Book,
    /// Contract offers sent to your own pro's agent, until he calls back,
    /// and the deals he called back to say yes to, until you answer on his
    /// call (lane N5b).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ready: Vec<Ready>,
    /// What he told you about his life and what you told him about yours
    /// (personal talk, item 7).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub shared: Vec<Shared>,
}

/// A contract offer with your own pro's agent (lane N5b): the day he
/// calls back, his call once it's made (0 before), who answers, the man it
/// is for, and the deal.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Ready {
    #[serde(default)]
    pub scene: u32,
    pub by: crate::calendar::Date,
    pub with: Them,
    pub player: u32,
    pub offer: Offer,
}

impl Memory {
    pub fn is_empty(&self) -> bool {
        self.his_lies.is_empty()
            && self.grapevine.is_empty()
            && self.notebook.is_empty()
            && self.unseen.is_empty()
            && self.treated.is_empty()
            && self.secrets.is_empty()
            && self.agreed.is_empty()
            && self.talks.is_empty()
            && self.open.is_empty()
            && self.owed.is_empty()
            && self.anchors.is_empty()
            && self.watch.is_empty()
            && self.ready.is_empty()
            && self.shared.is_empty()
            && self.draft.is_empty()
    }
}
