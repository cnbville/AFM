//! Typed facts and the store of lies (live talks, the core; talk facts
//! design, section 14). Only a typed line can state a false fact: a chip
//! never does. So the reading side of a fact is the sheet's, and what is
//! here is the live track's alone: what a stated fact is (`Stated`), what
//! the game finds when it checks one (`Truth`), and the one saved store of
//! the update (`Live`, on `Folk.live`): the false things said that he can
//! only check on a later day, and each agent's and GM's trust in your
//! word.
//!
//! A claim about where he stands ("you're my starter", "you're ahead of
//! him") is a promise about what he'll see (`afm/promises/proposal.md`,
//! section 11, agreed by Cain 2026-11-13 16:29 UTC, replacing the rejected
//! pick 7): `standing_promise` hears it as one, `scene::say` hands it to
//! `talks::typed_promise`, and lane P1 fills that seam (his belief now,
//! judged on his next 2 practice weeks and 2 games). A claim he can only
//! check later (a NIL number at signing) is a lie saved here, judged on its
//! day by `judge`, which costs nothing until P1 fills it too. No lane files
//! a lie yet (lanes 9 and 10 do), so nothing changes in any world today.

use crate::calendar::Date;
use crate::world::World;
use crate::world::folk::scene::{Heard, PromiseRead, Scene, Who};
use crate::world::folk::talks::PromiseTo;
use crate::world::folk::{Team, person};

/// The kinds of fact a typed line can state, as the reader labels them.
/// The reader's schema lists exactly these.
pub const CLAIM_KINDS: [&str; 13] = [
    "depth_rank",
    "ahead_of_him",
    "role",
    "snaps_share",
    "touches",
    "nil_amount",
    "money",
    "injury",
    "status",
    "stat",
    "off_field",
    "interest",
    "record",
];

/// The kinds that say where he stands: his place on the chart, who is
/// ahead of him, his role, his snaps, his touches. Said to your own player
/// about himself, each is a promise about what he'll see (section 11).
pub const STANDING: [&str; 5] = ["depth_rank", "ahead_of_him", "role", "snaps_share", "touches"];

/// A fact stated in a typed line, as the reader labelled it. The AI never
/// says whether it is true: the game checks it (`check::fact`).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Stated {
    /// One of `CLAIM_KINDS`.
    pub kind: String,
    /// The person it is about, by id; none when it is about you, the team
    /// or the program.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub about: Option<u32>,
    /// What was said, in the reader's short words ("1st on the chart").
    pub value: String,
}

/// What the game finds when it checks a typed fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Truth {
    /// True now.
    True,
    /// False now.
    False,
    /// About the future: a promise, judged on its day.
    Later,
    /// True, and already in the price (a contract talk: "everyone knows
    /// about the DUI, it's in the number"; lane 7).
    Priced,
    /// Not on the sheet: the game can't say, so it counts for nothing.
    Unknown,
}

impl Truth {
    /// The mark under your line on the screen (the contract's "marks"), or
    /// none for a fact the game can't check.
    pub fn mark(self) -> Option<&'static str> {
        match self {
            Truth::True => Some("true"),
            Truth::False => Some("false"),
            Truth::Later => Some("later"),
            Truth::Priced => Some("priced"),
            Truth::Unknown => None,
        }
    }
}

/// A false thing said that he can only check on a later day, saved with
/// that day. A false thing he can check as you say it is never saved.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Lie {
    pub to: PromiseTo,
    pub fact: Stated,
    pub said: Date,
    /// The day he can see it (the point's check day, `claims::When::Day`).
    pub sees: Date,
    /// He has seen it (the day came).
    #[serde(default)]
    pub seen: bool,
    /// Who, in words with link marks.
    #[serde(default)]
    pub who: String,
}

/// The live talks' one saved store (`Folk.live`). No later lane adds a field
/// to `Folk`: what it keeps goes here.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Live {
    /// The lies waiting for their day, and the ones seen lately.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lies: Vec<Lie>,
    /// Each agent's and each GM's trust in your word, 0 to 1, by a key the
    /// lane that moves it sets (lane 7). Not `Agent.rel`, which is a club's
    /// trust in the agent, and not `market.fleece`, which AI trades write.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub word: Vec<(u64, f32)>,
}

/// The most lies kept: the oldest seen ones go first.
pub const KEEP_LIES: usize = 120;

/// Where trust in your word starts with someone you have not dealt with.
pub const WORD_START: f32 = 0.5;

impl Live {
    pub fn is_empty(&self) -> bool {
        self.lies.is_empty() && self.word.is_empty()
    }

    /// Someone's trust in your word (`WORD_START` until it moves).
    pub fn word_of(&self, key: u64) -> f32 {
        self.word.iter().find(|x| x.0 == key).map_or(WORD_START, |x| x.1)
    }

    /// Sets someone's trust in your word, held to 0-1.
    pub fn set_word(&mut self, key: u64, v: f32) {
        let v = v.clamp(0.0, 1.0);
        match self.word.iter_mut().find(|x| x.0 == key) {
            Some(x) => x.1 = v,
            None => self.word.push((key, v)),
        }
    }
}

/// Files a lie he can only check later (lanes 9 and 10).
pub fn file_lie(w: &mut World, lie: Lie) {
    let l = &mut w.folk.live.lies;
    l.push(lie);
    if l.len() > KEEP_LIES
        && let Some(k) = l.iter().position(|x| x.seen)
    {
        l.remove(k);
    }
}

/// The daily check beside `talks::check_promises`: each lie whose day has
/// come is seen, and goes to `judge`. Oldest first, by its place in the
/// store, so the order never depends on a map.
pub(crate) fn check_lies(w: &mut World, d: Date) {
    let due: Vec<usize> =
        (0..w.folk.live.lies.len()).filter(|&k| !w.folk.live.lies[k].seen && w.folk.live.lies[k].sees <= d).collect();
    for k in due {
        w.folk.live.lies[k].seen = true;
        let lie = w.folk.live.lies[k].clone();
        judge(w, &lie);
    }
}

/// What a lie costs when he sees it (the promise design, section 11; lane
/// P1): a broken promise's cost, scaled by how far he believed you
/// (`promise::broken_scale`: 40% from a man who never did, all of it from
/// one who counted on it), through `talks::settle_word` with
/// `Reason::LieSeen`. His belief is read from his trust and your record as
/// it reaches him, the promise's other parts at their middle.
fn judge(w: &mut World, lie: &Lie) {
    use crate::world::folk::promise;
    let belief = match lie.to {
        PromiseTo::Player(id) => person(w, id).map_or(1.0, |i| {
            let h = if super::team_of(w, i) == Team::seat(w) {
                promise::Hearer::Own
            } else {
                let j = match Team::seat(w) {
                    Some(Team::College(c)) => c,
                    _ => 0,
                };
                promise::Hearer::Out(j, super::Ear::Himself)
            };
            promise::Parts {
                trust: promise::trust_part(w, i, h),
                record: promise::record(w, i, h),
                likely: 1.0,
                past: promise::past(w, i),
                ear: promise::ear(w, i),
                shape: 1.0,
            }
            .belief() as f32
        }),
        _ => 1.0,
    };
    let cost = promise::broken_scale(belief, 1.0);
    crate::world::folk::talks::settle_word(w, lie.to, false, cost, crate::world::folk::talks::Reason::LieSeen);
}

/// The player a scene is with, when he is on your team: the only one a
/// claim about where he stands is a promise to.
pub fn your_player(w: &World, s: &Scene) -> Option<u32> {
    let seat = Team::seat(w)?;
    s.who.iter().find_map(|x| match x {
        Who::Player(id) => person(w, *id).filter(|&i| super::team_of(w, i) == Some(seat)).map(|_| *id),
        _ => None,
    })
}

/// Whether a stated fact says where `him` stands: a standing kind about
/// him (or about no one named), or who is ahead of him.
pub fn is_standing(f: &Stated, him: u32) -> bool {
    STANDING.contains(&f.kind.as_str()) && (f.about.is_none() || f.about == Some(him) || f.kind == "ahead_of_him")
}

/// A typed line's claim about where your player stands, heard as the
/// promise it is (section 11): the first such claim, when the line made no
/// promise of its own. Its terms are what he'll see: a start for a first
/// place on the chart, else his place; playing ahead of a named teammate; a
/// role (a package or a share of snaps); touches. The claim rides along
/// for P1 (`PromiseRead.claim`); by when is left empty, since the judging
/// window is his next 2 practice weeks and 2 games.
pub fn standing_promise(w: &World, s: &Scene, heard: &Heard) -> Option<PromiseRead> {
    if heard.promise.is_some() {
        return None;
    }
    let him = your_player(w, s)?;
    let f = heard.facts.iter().find(|f| is_standing(f, him))?;
    let v = f.value.to_lowercase();
    let first = ["start", "1st", "first", "the ones", "number one", "no. 1"].iter().any(|x| v.contains(x));
    let kind = match f.kind.as_str() {
        "depth_rank" if first => "start",
        "depth_rank" => "place",
        "ahead_of_him" => "ahead_of",
        "touches" => "touches",
        _ => "role",
    };
    Some(PromiseRead {
        kind: kind.to_string(),
        against: f.about.filter(|&a| a != him),
        claim: Some(f.clone()),
        ..PromiseRead::default()
    })
}
