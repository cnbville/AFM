//! Live talks v3, the shared contract (`docs/design/live-talks-persona.md`,
//! sections 3, 5, 8, 10, 11 and 16.3): everyone you talk to is played as
//! himself, from his real state in the world. This module is types, trait
//! stubs and test fakes only. Nothing here runs in the game yet: lanes N1
//! (people), N3 (deals and requests) and N4 (the conversation) fill the
//! traits, and the promises lane's `usage.rs` holds the terms and measures.
//!
//! The one rule (section 2): the AI reads your words and voices him. The
//! game decides what's true, what he knows, what he says, whether he
//! accepts, what it costs and what happens next. So every type the AI fills
//! (the ear's `Reading`) names only things the game put on his card, and
//! every type the game fills (`Turn`) is what the voice is allowed to say.
//!
//! - `knows`: what a person knows and the four ways he tells it (3.1, 3.2,
//!   10.1). Reuses `scouting::file::source_says` for a trait read and
//!   `claims::Sheet` for what is true about him today.
//! - `wants`: who he is (17 kinds), what he wants and what he can do, heat
//!   and patience (3.3 to 3.5, 8.3).
//! - `memory`: his stores (3.6, 10.2 to 10.5): his lies, the grapevine, the
//!   colour notebook, unseen rule breaks, how you treated him, secrets,
//!   terms agreed, and the saved readings. Kept in `claims::Live` (`Folk.live`),
//!   the live talks' one saved store.
//! - `room`: channels and who's in the room (5.5, 5.6).
//! - `terms`: offers, counters, requests and decisions (8.2, 8.3,
//!   section 11). A term is the promises lane's `usage::Usage` with its
//!   `MEASURES` keys, never a second type; a promise's own record stays
//!   P1's `talks::Promise`.
//! - `deals`: lane N3's `Valuing`: a deal weighed against his best option,
//!   his counter, taking time, anchors, what agreeing makes real.
//! - `asks`: lane N3's `Acting`: whether he'll do what you ask, the action
//!   the game runs, his report back, and what asks you to confirm.
//! - `ear`: the ear's acts and hits (5.1, 5.2), and the game's turn (5.3).
//! - `fake`: fakes for every trait, for the lanes' tests.
//! - `talk`: lane N4, the conversation: the game's turn (`Resolving`), the
//!   hits sized, heat and patience, channels and the room, the saved
//!   readings, and stand-in parts until N1 and N3 land.
//!
//! The voice card, the ear's card and the ear's JSON live in afm-game
//! (`live_talk::persona`), beside the whitelist card and the reply filter
//! they reuse.

pub mod asks;
pub mod deals;
// Lane N5e, scouting and the draft.
pub mod draft;
pub mod ear;
pub mod fake;
pub mod knows;
pub mod memory;
pub mod room;
pub mod talk;
pub mod terms;
pub mod wants;

// Lane N1, people: who he is, what he knows and tells, what he wants, heat
// and patience, the stores, people coming to you.
pub mod coming;
pub mod feel;
pub mod football;
pub mod knowing;
pub mod makeup;
pub mod people;
pub mod stores;
pub mod telling;
pub mod wanting;

// Lane N5c: staff, bosses and money (the facet's people, the team doctor,
// references, staff contracts, your own deal, boosters, compliance).
pub mod office;

use super::cast::CastRef;

/// Anyone you can talk to, by the store that holds him. A recruit, a
/// transfer, a prospect and a pro are all players. The voice never sees
/// this: the cards carry words and tags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Them {
    /// A player at any stage (`Person.player.id`).
    Player(u32),
    /// A member of a player's family, by the player and his place in the
    /// family (0 his decider, then parents, grandparents, siblings, as
    /// `background::Family` lists them).
    Kin(u32, u8),
    /// A coach (`World.coaches`): your staff, another staff, a high school
    /// coach, a search firm's candidate.
    Coach(usize),
    /// A cast person: an AD, a president, an owner, a GM, a booster, a
    /// reporter, the league office (`cast::Role`), with the place among
    /// several, as `talks::PromiseTo::Cast` keeps it.
    Cast(CastRef, u16),
    /// An agent (`market.agents`).
    Agent(usize),
    /// An NFL club's scout (`pro.scouts`).
    ProScout(usize),
    /// Someone the world doesn't store yet (a 7-on-7 coach, a counselor, a
    /// street agent, an official, an investigator): his kind and a key the
    /// lane that draws him sets.
    Drawn(wants::Kind, u64),
}

impl Them {
    /// The same person as P1's promise keeps him, when P1 can hold him.
    pub fn promise_to(self) -> Option<super::talks::PromiseTo> {
        use super::talks::PromiseTo;
        match self {
            Them::Player(id) => Some(PromiseTo::Player(id)),
            Them::Coach(c) => Some(PromiseTo::Coach(c)),
            Them::Cast(r, n) => Some(PromiseTo::Cast(r, n)),
            _ => None,
        }
    }
}
