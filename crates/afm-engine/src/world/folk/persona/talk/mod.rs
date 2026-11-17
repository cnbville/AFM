//! Live talks v3, lane N4: the conversation (`docs/design/live-talks-persona.md`,
//! sections 5, 13 and 14). The game's side of a line: the ear has read
//! your words into acts and hits (`ear::Reading`); this module resolves
//! them into the game's `Turn` (what he says back, his asks, the heat, the
//! end), sizes every hit from his traits and state inside today's caps,
//! and makes it real (`apply`). The voice then says only what the turn
//! released (afm-game, `live_talk::persona`).
//!
//! The one rule (section 2): his words never change the world. Yours do,
//! only through this module, and only by the game's own rules.
//!
//! What a person knows, tells and wants comes through the contract's
//! traits (`Knows`, `Tells`, `Wanting`: lane N1), deals and requests
//! through `Valuing` and `Acting` (lane N3). `Parts` holds whichever
//! implementation the game hands in; `today` is a small stand-in built on
//! today's stores, used until those lanes land.
//!
//! - `hits`: how much each hit counts (question 1), capped per line by
//!   the best and worst chip he could be given now (lane 9's rule:
//!   wording alone never beats the best chip).
//! - `feel`: heat and patience during a talk (3.5).
//! - `channels`: what each channel allows and who answers (5.5, 5.6).
//! - `store`: the open talk's state and the saved readings (3.6, 14).
//! - `today`: the stand-in parts.

pub mod between;
pub mod channels;
pub mod feel;
pub mod hits;
pub mod items;
pub mod stakes;
pub mod standing;
pub mod store;
pub mod today;
pub mod who;
pub mod wrap;

// Facet lanes (N5a-N5e): each plays its own talks on this conversation.
pub mod hs_call;
pub mod recruit;

use super::Them;
use super::ear::{Act, ActKind, Confirm, Ends, HisAsk, Item, Reading, Resolving, Say, Sized, Turn};
use super::knows::{Answer, Knows, TellParts, Tells, Topic};
use super::memory::{HisLie, Line, Secret, Treated};
use super::room::{Room, Seat};
use super::terms::{Valuing, Weighed};
use super::wants::{Acting, Boil, Want, WantKind, Wanting, Willing};
use crate::player::Trait;
use crate::world::World;
use crate::world::folk::claims::{self, Stated, Truth};
use crate::world::folk::scene::{Heard, Kind as SceneKind, PromiseRead, Scene};
use crate::world::folk::{self as folk, talk_points as tp, talks};

/// The parts a talk is played with. Each is a contract trait, so lanes N1
/// and N3 swap their real parts in where the game builds this.
#[derive(Clone, Copy)]
pub struct Parts<'a> {
    pub knows: &'a dyn Knows,
    pub tells: &'a dyn Tells,
    pub wanting: &'a dyn Wanting,
    pub valuing: &'a dyn Valuing,
    pub acting: &'a dyn Acting,
    pub feel: &'a dyn feel::Feeling,
}

impl Parts<'static> {
    /// The parts as built: lane N1's people (`People`: what he knows, how
    /// he tells it, what he wants, heat and patience on his makeup) and
    /// lane N3's deals and requests (`Deals` on N1's wants, `Asks`).
    pub fn today() -> Parts<'static> {
        static P: super::people::People = super::people::People;
        static A: super::asks::Asks = super::asks::Asks;
        static F: feel::Sized = feel::Sized;
        Parts { knows: &P, tells: &P, wanting: &P, valuing: &DEALS, acting: &A, feel: &F }
    }

    /// The old stand-in for every part (today's stores only), for tests.
    pub fn stand_in() -> Parts<'static> {
        static T: today::Today = today::Today;
        static F: feel::Plain = feel::Plain;
        Parts { knows: &T, tells: &T, wanting: &T, valuing: &T, acting: &T, feel: &F }
    }
}

/// Lane N3's deals, weighed on lane N1's wants.
pub const DEALS: super::deals::Deals<'static> = super::deals::Deals { wants: &super::people::People };

/// One line of a talk, as the game plays it: the talk it is in, your
/// words, the ear's reading, and what the ear heard of each promise (lane
/// 9's `PromiseRead`, P1's terms) until the promises lane's `usage::Usage`
/// joins the contract's `Act::Promise`.
#[derive(Clone, Debug, Default)]
pub struct Said {
    pub scene: u32,
    pub words: String,
    pub reading: Reading,
    /// One per `Act::Promise`, in the same order.
    pub promises: Vec<PromiseRead>,
}

/// The conversation: one line resolved with the given parts.
pub struct Conversation<'a> {
    pub parts: Parts<'a>,
    pub said: Said,
}

/// The longest talk kept open, in lines, before he has somewhere to be.
/// Proposed, not confirmed.
pub const MAX_LINES: u8 = 24;

/// Minutes he takes to answer a line, beyond reading your words. Proposed,
/// not confirmed.
pub const ANSWER_MINUTES: f32 = 0.6;

/// The man across from you in a talk: the scene's first person.
pub fn across(room: &Room) -> Option<Them> {
    room.present.first().map(|p| p.who)
}

/// His trust in you, 0-1, from the store that keeps it (lane N1's
/// `makeup::trust`): your own player's trust in you, someone else's player
/// (a recruit, a transfer) through what he's heard of you, a family, a
/// coach, an agent.
pub fn trust_of(w: &World, who: Them) -> f32 {
    match who {
        // A man you don't coach keeps his trust in you apart from his
        // trust in his own head coach (`makeup::trust`).
        Them::Player(_) => super::makeup::trust(w, who) as f32,
        // A parent: where lane N1 keeps it (`folk.mine`, the older
        // meetings' `nudge`).
        Them::Kin(..) => super::makeup::trust(w, who) as f32,
        // Someone in your building: the trust his meeting's chips move
        // (lane N5c).
        _ if super::office::key(w, who).is_some() => super::office::trust(w, who),
        _ => w.folk.live.word_of(store::word_key(who)),
    }
}

/// Which facet lane plays a scene's talk, if one does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facet {
    /// Every other talk: your players' one-on-ones (lane N4) and the
    /// lanes that run on the shared path (N5b-e).
    Other,
    /// A recruit's or a transfer's talk (lane N5a).
    Recruit,
    /// A college staff's call to you, the high school coach (lane N5a).
    HsCall,
}

impl Facet {
    pub fn of(s: &Scene) -> Facet {
        if recruit::is_recruit(s.kind) {
            Facet::Recruit
        } else if hs_call::is_call(s.kind) {
            Facet::HsCall
        } else {
            Facet::Other
        }
    }
}

/// A trait z score for a player (0 for anyone else).
pub(crate) fn z(w: &World, who: Them, t: Trait) -> f32 {
    match who {
        Them::Player(id) => folk::person(w, id).map_or(0.0, |i| w.people[i].player.personality.traits.z(t) as f32),
        _ => 0.0,
    }
}

/// The scene a line is said in.
pub fn scene(w: &World, id: u32) -> Option<&Scene> {
    w.folk.scenes.iter().find(|s| s.id == id)
}

impl Conversation<'_> {
    /// A claim heard on a typed line (lanes 3, 9 and 10's fact checks), as
    /// lane 9 hears it: a claim about where your player stands is a promise
    /// about what he'll see (`claims::truth::standing_promise`).
    fn standing(&self, w: &World, s: &Scene, facts: &[Stated]) -> Option<PromiseRead> {
        if !self.said.promises.is_empty() {
            return None;
        }
        let heard = Heard { words: self.said.words.clone(), facts: facts.to_vec(), ..Heard::default() };
        claims::truth::standing_promise(w, s, &heard)
    }
}

impl Resolving for Conversation<'_> {
    fn turn(&self, w: &World, room: &Room, reading: &Reading) -> Turn {
        let mut t = Turn::default();
        let Some(him) = across(room) else { return t };
        let mut open =
            store::open(w, self.said.scene).cloned().unwrap_or_else(|| store::Open::new(self.said.scene, room));
        // A fresh talk starts from his own patience (lower after a hot
        // talk lately).
        if open.lines == 0 {
            open.patience = self.parts.feel.start(w, him);
        }
        let allows = channels::allows(room.channel);
        t.heat = open.heat;
        t.patience = open.patience;
        t.minutes = channels::minutes(room.channel, &self.said.words) + ANSWER_MINUTES;
        // A line that isn't a person talking does nothing but wear on him
        // (section 14).
        if reading.not_talk {
            t.patience = self.parts.feel.patience(w, him, &open, &t, feel::Progress::Nowhere);
            t.say.released.push("He doesn't follow what you mean.".into());
            return t;
        }
        let wants = self.parts.wanting.wants(w, him);
        let s = scene(w, self.said.scene);
        let facet = s.map_or(Facet::Other, Facet::of);
        let mut promise_k = 0usize;
        let mut facts: Vec<Stated> = Vec::new();
        // Questions in this line: a fresh one he answers gets somewhere; one
        // he dodges or you'd already asked doesn't (Cain 2026-11-15), and
        // what pushing him cost isn't progress either.
        let mut asked_now: Vec<(Topic, Option<Them>)> = Vec::new();
        let (mut answered, mut grind) = (false, false);
        let mut pushed_hits: Vec<usize> = Vec::new();
        // You gave him the floor ("any questions?", "anything else?").
        let mut open_floor = false;
        // A talk to the whole team or a room.
        let in_group = matches!(room.channel, super::room::Channel::Group(_));
        // Your line gives him a role itself, as a promise or a claim on his
        // own place (Cain 2026-11-16: "You're already my first pick at DT,
        // but I need more pressures from you" had him do it for "a real
        // role", then ask whether he had one).
        let gives_role = {
            let claimed: Vec<Stated> = reading
                .acts
                .iter()
                .filter_map(|a| match a {
                    Act::Claim(st) => Some(st.clone()),
                    _ => None,
                })
                .collect();
            let role = |k: &str| matches!(k, "start" | "role" | "ahead_of" | "touches" | "snaps");
            self.said.promises.iter().any(|p| role(&p.kind))
                || s.and_then(|s| self.standing(w, s, &claimed)).is_some_and(|p| role(&p.kind))
        };
        for act in &reading.acts {
            match act {
                // An open floor has nothing to look up: never "he doesn't
                // know" (the talks audit, 2026-11-16: "Any questions?" to
                // the team got "He doesn't know, sir"). He answers with
                // what's on his mind, his own ask below, or that he has
                // none.
                Act::Question { topic: Topic::Nothing, .. } => {
                    open_floor = true;
                    answered = true;
                }
                Act::Question { topic, about, .. } => {
                    let again = open.asked_before(*topic, *about) || asked_now.contains(&(*topic, *about));
                    asked_now.push((*topic, *about));
                    let known = self.parts.knows.knows(w, him, *topic, *about);
                    let pushed = open.pushed(*about) + t.told.len() as u8;
                    let parts = TellParts {
                        trust: f64::from(trust_of(w, him)),
                        professionalism: f64::from(10.0 + 3.0 * z(w, him, Trait::Professionalism)),
                        loyalty: loyalty(w, him, *about),
                        interest: 0.0,
                        room: channels::crowd(room),
                        pushed,
                    };
                    let mut told = self.parts.tells.tell(w, him, &known, &parts);
                    // Asked about his own life and nothing's stored: there's
                    // no news, not a thing he doesn't know ("How's the
                    // family?" is never "I'd rather not guess").
                    if known.is_empty()
                        && about.is_none_or(|a| a == him)
                        && let Some(x) = own_nothing(*topic)
                    {
                        told.released = vec![x.into()];
                    }
                    // Hot past his point and gone quiet: the safe half at most.
                    if open.boiled == Some(Boil::Quiet) && told.answer == Answer::Straight {
                        told.answer = Answer::Partly;
                        told.released.truncate(1);
                    }
                    t.minutes += told.cost.minutes;
                    if again || told.answer == Answer::Deflects {
                        grind = true;
                    } else {
                        answered = true;
                    }
                    if told.cost.trust > 0.0 {
                        let item = match about {
                            Some(Them::Player(id)) => Item::Friend(*id),
                            _ => Item::HisWord,
                        };
                        pushed_hits.push(t.hits.len());
                        t.hits.push(Sized {
                            hit: super::ear::Hit { on: him, item, how: super::ear::How::Doubted },
                            trust: -told.cost.trust,
                            mood: 0.0,
                            faded: false,
                        });
                    }
                    t.say.released.extend(told.released.iter().cloned());
                    t.say.holding_back |= told.holding_back;
                    if !told.tell.is_empty() {
                        t.say.tell = told.tell.clone();
                    }
                    if let Some(l) = &told.lie {
                        t.say.lie = Some(l.words.clone());
                        t.lies.push(l.clone());
                    }
                    t.told.push(told);
                }
                Act::Claim(st) => {
                    let truth = s.map_or(Truth::Unknown, |s| claims::check::fact(w, s, st));
                    if truth == Truth::False {
                        // He knows it isn't so as you say it (lane 9's
                        // rule): it costs what a move against the facts
                        // costs, and it heats him up.
                        let ego = f64::from(z(w, him, Trait::Ego));
                        let cost = claims::sheet::Basis::Against.trust_cost(ego);
                        let item = st.about.filter(|&a| Them::Player(a) != him).map_or(Item::Pride, Item::Friend);
                        t.hits.push(Sized {
                            hit: super::ear::Hit { on: him, item, how: super::ear::How::Doubted },
                            trust: -cost,
                            mood: 0.0,
                            faded: false,
                        });
                        t.say.released.push(format!("He knows that isn't true: {}.", folk::plain(&st.value)));
                    }
                    facts.push(st.clone());
                    t.claims.push((st.clone(), truth));
                }
                // Your boss weighs your own deal as an offer; anything else
                // put to him as terms is an ask or a goal, never a deal he
                // values like a player's (Cain 2026-11-16: "get me the NIL and
                // I'll sign the #1" came back as a player's counter-offer).
                Act::Offer(o) if super::office::deals::not_a_deal(w, him, o) => {}
                Act::Offer(o) => {
                    let wd = self.parts.valuing.weigh(w, him, o);
                    // His own reasons, where the game weighed them, in
                    // place of the plain words.
                    let why = self.parts.valuing.why(w, him, o, &wd);
                    let plain = why.is_empty();
                    t.say.released.extend(why);
                    match &wd {
                        Weighed::Yes => {
                            t.confirm.push(Confirm::Offer(o.clone()));
                            if plain {
                                t.say.released.push("He'd take that.".into());
                            }
                        }
                        Weighed::Counter(c) => {
                            t.say.released.extend(c.asks.iter().cloned());
                            t.counter = Some(c.clone());
                        }
                        Weighed::Later(d) => {
                            t.say.released.push(format!("He wants until {} to think it over.", d));
                        }
                        Weighed::Lowball => {
                            t.say.released.push("He's insulted by the offer.".into());
                            // What a lowball costs (lane N3's `cost`),
                            // held inside the caps with the other hits.
                            let c = super::deals::cost(w, him, &wd, 0);
                            if c.trust != 0.0 {
                                t.hits.push(Sized {
                                    hit: super::ear::Hit { on: him, item: Item::Pride, how: super::ear::How::Stung },
                                    trust: c.trust,
                                    mood: 0.0,
                                    faded: false,
                                });
                            }
                        }
                        Weighed::No if plain => t.say.released.push("He doesn't take it up.".into()),
                        Weighed::No => {}
                    }
                    t.weighed.push((o.clone(), wd));
                }
                Act::Promise { .. } if facet == Facet::Recruit => {
                    // A recruit's promise is the recruiting desk's (`pledge`),
                    // made in `apply`.
                    if let (Some(s), Some(p)) = (s, self.said.promises.get(promise_k)) {
                        let family = channels::speaker(room, reading)
                            .filter(|(_, seat)| *seat == super::room::Seat::Decider)
                            .map(|(x, _)| channels::name(w, x));
                        t.say.released.push(recruit::promise_released(w, s, p, family.as_deref()));
                    }
                    promise_k += 1;
                }
                Act::Promise { .. } => {
                    // Logged through P1 in `apply`, in order: its index then.
                    // Only one that has a home with him (`promise_home`): a
                    // promise to a player, to a parent about his son, or a
                    // stay or a raise to someone in your building.
                    if self.said.promises.get(promise_k).is_some_and(|p| promise_home(w, him, p).is_some()) {
                        t.promised.push((w.folk.promises.len() + t.promised.len()) as u32);
                    }
                    promise_k += 1;
                }
                Act::Commit(c) => {
                    // A commitment to someone you answer to: your boss
                    // writes down a goal he can judge, a booster hears it on
                    // what he wants (`folk::commit`).
                    if let Some(took) = folk::commit::took(w, him, c) {
                        t.say.released.extend(folk::commit::said(w, c, took));
                        t.committed.push((c.clone(), took));
                    }
                }
                Act::Request(r) => {
                    // The recruiting desk answers what it owns (commit).
                    let own = s.and_then(|s| recruit::willing(w, s, r));
                    if let Some((_, words)) = &own {
                        t.say.released.push(words.clone());
                    }
                    let desk = own.is_some();
                    let wl = own.map_or_else(|| self.parts.acting.willing(w, him, r), |x| x.0);
                    // Asked of the whole room, it's the team's to take: the
                    // man who answers for them names no price of his own
                    // (the talks audit, 2026-11-16: "Do I get a real role
                    // out of this, coach?" in front of the team).
                    let wl = match wl {
                        Willing::If(_) if in_group => Willing::Yes,
                        // The role he'd want for it is in the same line.
                        Willing::If(c)
                            if gives_role
                                && !c.gives.is_empty()
                                && c.gives.iter().all(|g| matches!(g, super::terms::Give::Role(_))) =>
                        {
                            Willing::Yes
                        }
                        x => x,
                    };
                    // His own reasons, where the game weighed them (your
                    // boss's and a booster's asks, a coordinator's plan
                    // ask), in place of the plain words.
                    let why = if desk { Vec::new() } else { self.parts.acting.why(w, him, r, &wl) };
                    let plain = why.is_empty();
                    t.say.released.extend(why);
                    match &wl {
                        Willing::Yes => {
                            // One that costs you waits on your confirm; a
                            // favour of goodwill runs as he agrees (`apply`).
                            // Told to the whole room, it's your word to the
                            // team, nothing to confirm ("we go back to basics
                            // this week" became "set the game plan", the talks
                            // audit of 2026-11-16).
                            if super::asks::costs_you(r) && !in_group {
                                t.confirm.push(Confirm::Request(r.clone()));
                            }
                            // Asked of the room, the room answers ("Yes
                            // sir, we'll do it. He'll do it.", the talks
                            // audit of 2026-11-16).
                            if plain {
                                t.say.released.push(if in_group { "They'll do it." } else { "He'll do it." }.into());
                            }
                            // Asked for his read on a man, he gives it now.
                            if let super::terms::Request::Evaluate(pid) = r {
                                t.say.released.extend(super::evaluate::words(w, him, *pid));
                            }
                        }
                        Willing::If(c) => {
                            t.say.released.extend(c.asks.iter().cloned());
                            t.counter = Some(c.clone());
                        }
                        Willing::Later(d) => t.say.released.push(format!(
                            "{} let you know by {}.",
                            if in_group { "They'll" } else { "He'll" },
                            d
                        )),
                        Willing::No => {
                            if plain {
                                t.say
                                    .released
                                    .push(if in_group { "They won't do that." } else { "He won't do that." }.into());
                            }
                            if open.refused_before(r) {
                                t.patience.0 -= feel::ASKED_AGAIN;
                            }
                        }
                    }
                    t.asked.push((r.clone(), wl, None));
                }
                Act::Decision(d) => {
                    t.confirm.push(Confirm::Decision(*d));
                    // What he heard, to react to: his to take or to fight
                    // with his reasons, since nothing is done until you
                    // make it so.
                    t.say.heard.push(format!(
                        "He tells you his decision: {}. Nothing is done until he makes it so",
                        channels::decision_to(*d, him)
                    ));
                }
                Act::Threat { ultimatum, .. } => {
                    if *ultimatum {
                        t.say.released.push("You've put it to him: his call.".into());
                    }
                }
                Act::Deadline(d) => {
                    if let Some((o, _)) = t.weighed.last_mut() {
                        o.until = Some(*d);
                    }
                    for c in t.confirm.iter_mut() {
                        if let Confirm::Offer(o) = c {
                            o.until = Some(*d);
                        }
                    }
                }
                Act::Apology { for_promise, .. } => {
                    if let Some(sz) = hits::apology(w, him, *for_promise, &open) {
                        t.hits.push(sz);
                    }
                }
                Act::Confide(_) | Act::KeepQuiet(_) => {}
                Act::Invite(_) => t.say.released.push("He'll hear more about that once it's set.".into()),
                Act::BringIn(who) => {
                    if !allows.bring_in {
                        t.say.released.push("There's no one else on this call.".into());
                    } else if !channels::can_bring(w, *who) {
                        t.say.released.push("That person isn't here to bring in.".into());
                    } else {
                        t.say.released.push(format!("{} joins you.", channels::name(w, *who)));
                    }
                }
                Act::AskToLeave(_) => {}
                Act::Show(sh) => {
                    if !channels::can_show(room.channel, *sh) {
                        t.say.released.push("You can't show him that from here.".into());
                    } else {
                        t.say.heard.push(format!("He shows you {}", channels::show_words(*sh)));
                    }
                }
                Act::Answer(_) => {}
                Act::End => t.ends = Some(Ends::You),
                // Your word on his future (playtest bug 15): the picks'
                // outcome and odds, decided now, made real in `apply`.
                Act::Future { word, strength } => {
                    if let Some(i) = yours_across(w, him, s) {
                        let plan = folk::future::plan(w, i, *word, strength);
                        if !plan.words().is_empty() {
                            t.say.released.push(plan.words().into());
                        }
                        if plan.ends() {
                            t.ends = Some(Ends::Him(None));
                        }
                        t.future.push(plan);
                    }
                }
                // A change to a promise you made him: the promises
                // design's answers, made real in `apply`.
                Act::Amend(a) => {
                    if let Some(i) = yours_across(w, him, s) {
                        match amended(w, i, a) {
                            Some((k, pa, bar)) => {
                                t.say.released.push(amend_words(pa).into());
                                t.reworked.push((k as u32, pa, bar));
                            }
                            None => t.say.released.push("You have no promise to him for that to change.".into()),
                        }
                    }
                }
            }
        }
        // Every hit on every listener, sized (question 1), then held inside
        // today's caps per person.
        let line = open.lines;
        for h in &reading.hits {
            if !room.present.iter().any(|p| p.who == h.on) {
                continue;
            }
            let sized = match (facet, s) {
                (Facet::Recruit, Some(s)) => recruit::size(w, s, h, &wants, &open),
                (Facet::HsCall, Some(s)) => hs_call::size(w, s, h),
                _ => hits::size(w, h, &wants, &open, line),
            };
            if let Some(sz) = sized {
                t.hits.push(sz);
            }
        }
        // A dodge needs a question to dodge: his own ask on the table (the
        // one his last line raised), or his opening's question. The ear may
        // hear a dodge of something he asked lines ago, or of a demand he
        // never put to you (Cain 2026-11-16: "any other questions?" heard
        // as doubting him); and a question he pressed again on his own made
        // the next line a dodge too, so he pressed it on every line (the
        // talks audit, 2026-11-16). And his last line has to have put it
        // as a question: an ask he said he'd take up later ("once the staff
        // is set I'd like to talk about where I fit in") asked nothing yet,
        // so the coach's next line answered nothing it owed (the last
        // replay, 2026-11-17).
        let on_table =
            (open.asking().is_some() && s.is_none_or(asked_last)) || (open.lines == 0 && s.is_some_and(asked_last));
        if reading.dodged && on_table {
            t.dodged = true;
            t.hits.push(hits::dodged(w, him));
        }
        if facet == Facet::Other {
            hits::settle(&mut t.hits);
        }
        match (facet, s) {
            (Facet::Recruit, Some(s)) => {
                recruit::cap(w, s, &mut t.hits);
                // His family's reaction when they answer the line.
                let family = channels::speaker(room, reading)
                    .filter(|(_, seat)| *seat == super::room::Seat::Decider)
                    .map(|(x, _)| channels::name(w, x));
                // A question, an invite or an ask makes no point about
                // your school, so it draws no reaction to one: "That's
                // exactly what I'm looking for" came back to "How was your
                // game Friday?" (the talks audit, 2026-11-16).
                let pitched = reading.acts.is_empty()
                    || !reading.acts.iter().all(|a| {
                        matches!(a.kind(), ActKind::Question | ActKind::Request | ActKind::Invite | ActKind::End)
                    });
                t.say.released.extend(recruit::released(w, s, &t.hits, &wants, family.as_deref(), pitched));
            }
            (Facet::HsCall, Some(_)) => {}
            (_, Some(s)) => hits::cap(w, s, room, &mut t.hits),
            _ => {}
        }
        // Someone in your building: inside what his meeting's chips can do
        // (lane N5c).
        super::office::cap(w, room, &mut t.hits);
        // A parent, an agent, a trainer, another school's committee in one
        // of the older meetings.
        if let Some(s) = s {
            super::meetings::cap(w, s, room, &mut t.hits);
        }
        // His own ask (5.4, question 3): what he wants most that you
        // haven't given, once a talk per want.
        // A line that puts something to him (an offer, a promise, a
        // request, a decision) gets his answer, not a change of subject:
        // his own concern waits for a later line.
        let put_to_him = reading.acts.iter().any(|a| {
            matches!(
                a,
                Act::Offer(_)
                    | Act::Promise { .. }
                    | Act::Commit(_)
                    | Act::Request(_)
                    | Act::Decision(_)
                    | Act::Future { .. }
            )
        });
        // Never in front of the whole team or a room: a man doesn't raise
        // his role or his job before his teammates (the talks audit,
        // 2026-11-16). Given the floor, he asks what's on his mind even
        // when it weighs less.
        let bar = if open_floor { OPEN_FLOOR_ASK } else { ASK_WEIGHT };
        if t.ends.is_none()
            && !put_to_him
            && !in_group
            && let Some(want) = wants
                .iter()
                .find(|x| !x.met && x.weight >= bar && !open.raised(x.kind) && !asked_about(reading, &open, x.kind))
            && (line >= 1 || open_floor)
            // One ask at a time: with his last ask still on the table he
            // waits for your answer before he raises another (the talks
            // audit, 2026-11-16: your trainer asked about his next step,
            // then whether the team can win, on back-to-back lines).
            && (open_floor || open.asking().is_none())
            // A college coach calling about a player asks about the
            // player, never about his own career ("what you think about
            // my next step", to your high school coach, the talks audit of
            // 2026-11-16): the call's questions are its own (`hs_call`).
            && facet != Facet::HsCall
        {
            t.his_asks.push(HisAsk {
                why: *want,
                question: ask_topic(want.kind),
                asks: vec![ask_words_of(w, him, want.kind)],
                gives: Vec::new(),
                terms: Vec::new(),
            });
        }
        // Someone in your building asks for something the game can do
        // (lane N5c): a raise, a title, the play-calling.
        for a in t.his_asks.iter_mut() {
            a.gives = super::office::asks::his_gives(w, him, a.why.kind);
        }
        match (facet, s) {
            (Facet::Recruit, Some(s)) => recruit::turn(w, s, reading, &mut t),
            (Facet::HsCall, Some(s)) => hs_call::turn(w, s, reading, &mut t),
            _ => {}
        }
        // The stand-in for a lie he catches as you say it.
        let caught = t.claims.iter().filter(|(_, x)| *x == Truth::False).count();
        t.heat = self.parts.feel.heat(w, him, &open, &t, caught);
        let moved =
            t.hits.iter().enumerate().any(|(k, h)| !pushed_hits.contains(&k) && (h.trust + h.mood).abs() > 0.01);
        let progress = if moved || answered {
            feel::Progress::Landed
        } else if grind {
            feel::Progress::Grind
        } else {
            feel::Progress::Nowhere
        };
        t.patience = self.parts.feel.patience(w, him, &open, &t, progress);
        if t.ends.is_none() {
            if let Some(b) = self.parts.feel.boil(w, him, &open, t.heat) {
                if b == Boil::WalksOut {
                    t.ends = Some(Ends::Him(Some(b)));
                }
                t.say.released.push(channels::boil_words(b).into());
            } else if t.patience.0 <= 0.0 {
                t.ends = Some(Ends::Him(None));
                t.say.released.push("He's done talking for today.".into());
            } else if line + 1 >= allows.lines.min(MAX_LINES) || talks::hours_left(w) <= 0.0 {
                t.ends = Some(Ends::Time);
                // The last exchange closes the talk: he answers and says
                // goodbye, with no new question of his (the voice's card
                // says so). Before this a call ran out on his new question
                // and the next line found it over (the talks audit,
                // 2026-11-16).
                t.his_asks.clear();
            }
        }
        t.speaker = channels::speaker(room, reading);
        if let Some(s) = s
            && s.kind == SceneKind::OneOnOne
        {
            t.say.stance = tp::Stance::from_bits(s.c);
        }
        if t.say.tell.is_empty() {
            t.say.tell = hits::tell(w, him, &t.hits);
        }
        let _ = facts;
        t
    }

    fn apply(&self, w: &mut World, t: &Turn) {
        let id = self.said.scene;
        let Some(s) = scene(w, id).cloned() else { return };
        let room = store::room(w, id).unwrap_or_else(|| channels::room_for(w, &s));
        let Some(him) = across(&room) else { return };
        let facet = Facet::of(&s);
        let wants = self.parts.wanting.wants(w, him);
        // What a recruit's talk held before the line (lane 10: a claim he
        // catches costs the line's lift).
        let before = claims::check::before(w, &s);
        // Your words on the scene: what the screen shows (`Said.typed`).
        let you = folk::scene::your_name(w);
        let facts: Vec<Stated> = t.claims.iter().map(|(f, _)| f.clone()).collect();
        let standing = self.standing(w, &s, &facts);
        let point = self.parts.feel.point(w, him);
        let hot = t.heat.0 >= point;
        if let Some(sc) = w.folk.scenes.iter_mut().find(|x| x.id == id) {
            sc.you(&you, self.said.words.clone());
            if let Some(l) = sc.lines.last_mut() {
                l.typed = Some(self.said.words.clone());
            }
        }
        // Every sized hit: trust now, mood on the scene's running total for
        // the man across (it lands when the talk ends, as a chip's does)
        // and at once for the others in the room.
        match facet {
            Facet::Recruit => recruit::apply(w, id, t, &self.said.promises, &wants),
            Facet::HsCall => hs_call::apply(w, id, &self.said.reading, t),
            Facet::Other => {}
        }
        for h in t.hits.iter().filter(|_| facet == Facet::Other) {
            let Them::Player(pid) = h.hit.on else {
                // In a room of them (the parents, the feeder coaches) each
                // one's own trust lands.
                let in_room = super::meetings::is_group(&s) && room.present.iter().any(|p| p.who == h.hit.on);
                if (h.hit.on != him && !in_room) || h.trust == 0.0 {
                    continue;
                }
                // A parent: his own trust and his son's (the older meetings).
                if super::meetings::nudge(w, h.hit.on, h.trust) {
                    continue;
                }
                // Someone in your building: the trust the world keeps for
                // him (lane N5c); your word with anyone else.
                if super::office::nudge(w, h.hit.on, h.trust) {
                    continue;
                }
                let k = store::word_key(h.hit.on);
                let v = w.folk.live.word_of(k) + h.trust;
                w.folk.live.set_word(k, v);
                continue;
            };
            let Some(i) = folk::person(w, pid) else { continue };
            if super::makeup::coached_by_you(w, i) {
                let p = &mut w.people[i];
                p.trust = (p.trust + h.trust).clamp(0.0, 1.0);
            } else if h.trust != 0.0 {
                // Someone else's player: his trust in you, kept by his key.
                let now = super::makeup::trust(w, h.hit.on) as f32;
                w.folk.mine.set_trust(super::makeup::key(h.hit.on), (now + h.trust).clamp(0.0, 1.0));
            }
            // In a group talk every man's mood lands at once (lane N5d).
            if h.hit.on == him && !matches!(room.channel, super::room::Channel::Group(_)) {
                if let Some(sc) = w.folk.scenes.iter_mut().find(|x| x.id == id) {
                    sc.f[0] += h.trust;
                    sc.f[1] += h.mood;
                    if let Item::Point(key) = &h.hit.item {
                        sc.made.push((key.clone(), format!("typed {:?}", h.hit.how).to_lowercase(), h.trust + h.mood));
                    }
                }
            } else {
                talks::mood_event(w, pid, talks::Reason::Talk, h.mood);
            }
        }
        // Promises count the moment you say them (question 4), through P1
        // (a recruit's through the recruiting desk, above): to a player, to
        // a parent about his son, a stay or a raise to someone in your
        // building (`promise_home`).
        if facet == Facet::Other {
            for p in self.said.promises.iter().chain(standing.iter()) {
                match promise_home(w, him, p) {
                    Some(Home::Player(pid)) => talks::typed_promise(w, id, pid, p),
                    Some(Home::To(to, kind)) => talks::promise(w, to, kind, 0),
                    None => {}
                }
            }
        }
        // A commitment he took: the goal on your boss's list, a booster's
        // promise on his want (`folk::commit`).
        for (_, took) in &t.committed {
            if let Some(line) = folk::commit::land(w, him, *took) {
                if let Some(sc) = w.folk.scenes.iter_mut().find(|x| x.id == id) {
                    sc.outcome.push(line);
                }
                folk::commit::show(w, id);
            }
        }
        // Promises your line changes, then your word on his future (after
        // this line's own promises, so a strength named backs the promise
        // it made).
        if facet == Facet::Other
            && let Some(i) = yours_across(w, him, Some(&s))
        {
            let out = rework_and_future(w, i, t);
            if let Some(sc) = w.folk.scenes.iter_mut().find(|x| x.id == id) {
                sc.outcome.extend(out);
            }
        }
        // What an offer leaves behind (lane N3): your first number, an
        // answer he owes you.
        for (o, wd) in &t.weighed {
            super::deals::offered(w, him, o, wd);
        }
        // A recruit's family that takes time waits on your confirm to leave
        // the offer with them (question 4): nothing is owed until you do.
        if facet == Facet::Recruit && t.weighed.iter().any(|(_, x)| matches!(x, Weighed::Later(_))) {
            w.folk.live.persona.owed.retain(|o| o.with != him);
        }
        // Lane 10's costs for a claim he catches now, and a lie he can see
        // only on a day.
        if facet == Facet::Recruit {
            claims::check::landed(w, id, &facts, before);
        }
        // A favour of goodwill he agreed to runs now (`Acting::run`); one
        // that costs you waits on your confirm. Put to the whole room, it's
        // the team's, and the man who answers for them runs nothing of his
        // own ("He agreed to go back to basics", in front of the team).
        let in_group = matches!(room.channel, super::room::Channel::Group(_));
        for (r, wl, _) in t.asked.iter().filter(|_| !in_group) {
            if *wl == Willing::Yes && !super::asks::costs_you(r) {
                let own = scene(w, id).cloned().and_then(|sc| recruit::run(w, &sc, r));
                let done = own.unwrap_or_else(|| self.parts.acting.run(w, him, r).done);
                if let Some(sc) = w.folk.scenes.iter_mut().find(|x| x.id == id) {
                    for l in done {
                        sc.narrate(l);
                    }
                }
            }
        }
        let today = w.today;
        let m = &mut w.folk.live.persona;
        m.his_lies.extend(t.lies.iter().cloned());
        for act in &self.said.reading.acts {
            match act {
                Act::Confide(words) => m.secrets.push(Secret {
                    by: him,
                    yours: true,
                    to: him,
                    day: today,
                    words: words.clone(),
                    asked_quiet: false,
                    out: false,
                }),
                Act::KeepQuiet(_) => {
                    if let Some(x) = m.secrets.iter_mut().rev().find(|x| x.to == him && x.day == today) {
                        x.asked_quiet = true;
                    }
                }
                Act::Apology { .. } => store::treat(m, him, Treated::Apologised, today),
                Act::Request(_) => store::treat(m, him, Treated::Asked, today),
                _ => {}
            }
        }
        // Only a dodge the game counted (his own last line asked).
        if t.dodged {
            store::treat(m, him, Treated::Dodged, today);
        }
        // A stake you made, so one made again soon counts less.
        for h in t.hits.iter().filter(|h| h.hit.how == super::ear::How::Staked) {
            store::treat(m, h.hit.on, Treated::Staked, today);
        }
        if t.weighed.iter().any(|(_, x)| *x == Weighed::Lowball) {
            store::treat(m, him, Treated::Lowballed, today);
        }
        if hot {
            store::treat(m, him, Treated::Hot, today);
        }
        store::save_line(
            m,
            him,
            s.date,
            Line { you: self.said.words.clone(), reading: self.said.reading.clone(), his: String::new() },
        );
        store::advance(w, id, &room, t, &self.said, point);
        // The draft side's rooms (lane N5e): the interview clock, a press,
        // a GM's word, a family's answer for the file.
        super::draft::after_line(w, id, &self.said, t);
        talks::charge(w, t.minutes / 60.0);
        if t.ends.is_some() {
            end(w, id);
        }
    }
}

/// The man across in your one-on-one, when he's your player: his index.
fn yours_across(w: &World, him: Them, s: Option<&Scene>) -> Option<usize> {
    let Them::Player(pid) = him else { return None };
    s.filter(|s| s.kind == SceneKind::OneOnOne)?;
    folk::person(w, pid)
}

/// The promise a change names (the card's tag, else the one that slipped,
/// else the latest open one) and the answer it is.
fn amended(w: &World, i: usize, a: &super::ear::Amend) -> Option<(usize, talks::PAct, Option<f32>)> {
    let pid = w.people[i].player.id;
    let to = talks::PromiseTo::Player(pid);
    let k = a
        .promise
        .map(|k| k as usize)
        .filter(|&k| w.folk.promises.get(k).is_some_and(|p| p.to == to && p.kept.is_none()))
        .or_else(|| talks::slipped(w, pid))
        .or_else(|| folk::promise::fresh(w, pid))?;
    let (pa, bar) = talks::amend_act(w, i, k, a)?;
    Some((k, pa, bar))
}

/// What a change to a promise is, for the voice.
fn amend_words(a: talks::PAct) -> &'static str {
    use talks::PAct as P;
    match a {
        P::TakeBack => "You take back your promise to him.",
        P::Real => "You tell him the real reason: your promise ends there.",
        P::Writing => "You put your promise in writing.",
        P::NewDate | P::Week4 | P::Midseason | P::NextSeason => "Your promise to him has a new date.",
        _ => "Your promise to him has a condition on it now.",
    }
}

/// Makes the line's promise changes and your word on his future real: the
/// outcome lines. Changes run from the last promise back, so one taken
/// back doesn't move the others.
fn rework_and_future(w: &mut World, i: usize, t: &Turn) -> Vec<String> {
    let to = talks::PromiseTo::Player(w.people[i].player.id);
    let name = w.people[i].name.clone();
    let mut out = Vec::new();
    let mut rw = t.reworked.clone();
    rw.sort_by_key(|a| std::cmp::Reverse(a.0));
    rw.dedup_by_key(|x| x.0);
    for (k, a, bar) in rw {
        let k = k as usize;
        if !w.folk.promises.get(k).is_some_and(|p| p.to == to && p.kept.is_none()) {
            continue;
        }
        let words = w.folk.promises[k].kind.words();
        let r = talks::rework(w, i, k, a, bar);
        out.extend(r.outcome);
        if r.gone && a == talks::PAct::TakeBack {
            out.push(format!("You took back your promise to {name}: {words}."));
        } else if r.gone {
            out.push(format!("Your promise to {name} ends: {words}."));
        } else {
            out.push(format!("Your promise to {name} changed: {words}."));
        }
    }
    for plan in &t.future {
        out.extend(folk::future::run(w, i, plan));
    }
    out
}

/// His loyalty to the man a question is about, 0-1: their tie.
fn loyalty(w: &World, him: Them, about: Option<Them>) -> f64 {
    let (Them::Player(a), Some(Them::Player(b))) = (him, about) else { return 0.0 };
    if a == b {
        return 0.0;
    }
    w.folk.ties.of_id(a).filter(|t| t.a == b || t.b == b).map(|t| f64::from(t.strength) / 100.0).fold(0.0, f64::max)
}

/// His answer about his own life when nothing's stored on it: no news.
pub fn own_nothing(topic: Topic) -> Option<&'static str> {
    Some(match topic {
        Topic::Life => "Nothing new at home to tell: his family is doing fine",
        Topic::Mood => "He's doing all right; nothing in particular on his mind",
        Topic::Injury => "He feels fine; nothing's bothering him",
        Topic::Wants | Topic::Portal | Topic::Grades | Topic::Effort => "Nothing new there to tell",
        _ => return None,
    })
}

/// A want heavy enough that he raises it himself. Proposed, not confirmed.
pub const ASK_WEIGHT: f32 = 0.22;

/// A want he raises when you give him the floor ("any questions?").
/// Proposed, not confirmed.
pub const OPEN_FLOOR_ASK: f32 = 0.1;

/// The questions about a want of his.
fn want_topics(k: WantKind) -> &'static [Topic] {
    match k {
        WantKind::Security => &[Topic::Job],
        WantKind::Goals => &[Topic::Goals],
        WantKind::Money | WantKind::Fee => &[Topic::Money, Topic::Market],
        WantKind::Career => &[Topic::Jobs, Topic::Job],
        WantKind::Role => &[Topic::Wants, Topic::Job],
        WantKind::Draft => &[Topic::Market, Topic::ScoutFile],
        _ => &[],
    }
}

/// You already asked him about this want, in this line or earlier in the
/// talk ("convince me to keep you" to a coach whose job is on the line):
/// he answers it and doesn't ask it back ("How safe is my place here,
/// anyway?", the talks audit of 2026-11-16).
fn asked_about(reading: &Reading, open: &store::Open, k: WantKind) -> bool {
    let topics = want_topics(k);
    let now =
        reading.acts.iter().any(|a| matches!(a, Act::Question { topic, about: None, .. } if topics.contains(topic)));
    now || topics.iter().any(|t| open.asked_before(*t, None))
}

/// What he asks about a want of his.
fn ask_topic(k: WantKind) -> Option<Topic> {
    Some(match k {
        WantKind::Role | WantKind::Development | WantKind::Scheme => Topic::Football,
        WantKind::Money | WantKind::Security | WantKind::Fee | WantKind::Club => Topic::Market,
        WantKind::Career => Topic::Jobs,
        WantKind::Goals => Topic::Goals,
        WantKind::Draft => Topic::Market,
        _ => return None,
    })
}

/// His ask in words by who he is: your boss sets the goals you're judged
/// on, so he goes through them and asks how you'll meet them; anyone else
/// asks as `ask_words` says (Cain 2026-11-16: the athletic director asked
/// the coach what he'd be judged on).
pub fn ask_words_of(w: &World, him: Them, k: WantKind) -> String {
    // Your boss sets the goals and judges you on them: the question a real
    // athletic director asks across that desk is what you need from him to
    // get there, once, never what you're judged on or "show me how" after
    // every line (Cain 2026-11-16, the goals meeting).
    if k == WantKind::Goals && super::makeup::kind(w, him) == super::wants::Kind::Boss {
        return "he asks what you need from him to get there".into();
    }
    ask_words(k)
}

/// His ask in words, for the voice (`HisAsk.asks`).
pub fn ask_words(k: WantKind) -> String {
    match k {
        WantKind::Role => "he asks where he stands and what it takes to play more",
        WantKind::Money => "he asks about his money",
        WantKind::Security => "he asks how safe his place here is",
        WantKind::Winning => "he asks if this team can win now",
        WantKind::Place => "he asks about getting home more",
        WantKind::Development => "he asks what you'll do to get him to the next level",
        WantKind::Academics => "he asks about his classes and his degree",
        WantKind::Name => "he asks how the program is seen",
        WantKind::Loyalty => "he asks if you're staying",
        WantKind::Family => "he asks about his family's place in this",
        WantKind::Friends => "he asks about a friend on the team",
        WantKind::Fee => "he asks about the fee",
        WantKind::Career => "he asks about his own next step",
        WantKind::Scheme => "he asks about his say in the scheme",
        WantKind::Club => "he asks about the club's plans",
        WantKind::Goals => "he asks what you're being judged on",
        WantKind::Booster(_) => "he asks what his money is buying",
        WantKind::Kids => "he asks what this means for his kids",
        WantKind::Draft => "he asks what you're hearing about him from the NFL scouts",
    }
    .into()
}

/// Ends the talk: a one-on-one ends the way its chip does
/// (`talks::finish_talk`: his mood lands, the points made are kept).
pub fn end(w: &mut World, id: u32) {
    let Some(k) = w.folk.scenes.iter().position(|s| s.id == id) else { return };
    let mut s = w.folk.scenes[k].clone();
    if s.done {
        return;
    }
    match Facet::of(&s) {
        _ if s.kind == SceneKind::OneOnOne => {
            let pid = s.a as u32;
            talks::finish_talk(w, &mut s, pid);
        }
        Facet::Recruit => recruit::end(w, &mut s),
        Facet::HsCall => hs_call::end(w, &mut s),
        // A built staff or boss meeting (lane N5c) or one of the older
        // meetings: a goodbye ends it with no cost, what you didn't argue
        // stands, and a decision the talk didn't settle stays as the
        // meeting's own picks with the talk over (`wrap`).
        Facet::Other if wrap::kept_open(s.kind) => {
            wrap::close(w, id);
            return;
        }
        Facet::Other => {
            if !super::draft::end(w, &mut s) {
                s.end();
            }
        }
    }
    w.folk.scenes[k] = s;
    store::close(w, id);
}

/// His reply as the voice said it (or the fallback), on the scene and the
/// saved talk; a detail the voice added goes in his notebook (question 5).
/// `by` is who answered the line (`Turn.speaker`) when it isn't him: his
/// parents doing the talking, his agent on the money.
pub fn reply(w: &mut World, id: u32, words: &str, by: Option<Them>, tell: Option<String>, colour: Option<String>) {
    let Some(s) = scene(w, id).cloned() else { return };
    let room = store::room(w, id).unwrap_or_else(|| channels::room_for(w, &s));
    let Some(him) = across(&room) else { return };
    // The one who answered speaks the line, under his own name (the talks
    // audit, 2026-11-16: his parent's answers showed under the kid's name,
    // and the voice wrote "Andrew's parent:" into the line to say who).
    let by = by.filter(|x| *x != him && room.present.iter().any(|p| p.who == *x));
    let words = unlabel(w, &room, words);
    let name = channels::name(w, by.unwrap_or(him));
    if let Some(sc) = w.folk.scenes.iter_mut().find(|x| x.id == id) {
        // Your read shows when it catches something new: "He gives you
        // nothing either way." stood under every reply of a talk with a DT
        // (Cain 2026-11-16).
        let shown = sc.lines.iter().rev().filter(|l| l.by == 2).find_map(|l| l.tell.clone());
        let tell = tell.filter(|x| shown.as_ref() != Some(x));
        // A talk that ended on this line still shows his last words.
        match by {
            Some(_) => sc.other(&name, words.clone()),
            None => sc.them(&name, words.clone(), tell),
        }
    }
    let today = w.today;
    let m = &mut w.folk.live.persona;
    store::his_words(m, him, s.date, &words);
    if let Some(c) = colour {
        store::note(m, him, today, c);
    }
}

/// A line without the names the voice put in front of anyone's words
/// ("Andrew's parent: What we'd like to know..."): the screen shows who
/// speaks.
pub fn unlabel(w: &World, room: &Room, words: &str) -> String {
    let mut out = words.to_string();
    let mut cut = false;
    for p in &room.present {
        let label = format!("{}:", channels::name(w, p.who));
        while let Some(k) = out.find(&label) {
            out.replace_range(k..k + label.len(), "");
            cut = true;
        }
    }
    if cut { out.split_whitespace().collect::<Vec<_>>().join(" ") } else { out }
}

/// The voice's turn as words: everything it may say (`Say`), and his asks.
pub fn say_of(t: &Turn) -> (Say, Vec<String>) {
    let mut asks: Vec<String> = t.his_asks.iter().flat_map(|a| a.asks.iter().cloned()).collect();
    if let Some(c) = &t.counter {
        asks.extend(c.asks.iter().cloned());
    }
    (t.say.clone(), asks)
}

/// A want a hit was on, by its place in his list.
pub fn want_at(wants: &[Want], k: u8) -> Option<&Want> {
    wants.get(k as usize)
}

/// What the hidden-lie store gets when he lies (N1 fills `Told.lie`); kept
/// here so the type is used the same way everywhere.
pub fn lie(by: Them, topic: Topic, words: &str, said: crate::calendar::Date) -> HisLie {
    HisLie { by, topic, about: None, said, shows: None, words: words.to_string(), caught: false }
}

/// Who sits where, for the cards: the man across first.
pub fn seat_of(room: &Room, who: Them) -> Option<Seat> {
    room.present.iter().find(|p| p.who == who).map(|p| p.seat)
}

/// Where a typed promise is kept: a player's own (P1), or P1's promise to
/// someone else of a kind the world judges for him.
pub enum Home {
    /// A player, or a parent's son: P1's typed promise to him.
    Player(u32),
    /// Someone in your building: a stay (you won't leave for another job)
    /// to a coach on your staff or your boss, a raise to a coach on your
    /// staff.
    To(talks::PromiseTo, talks::PromiseKind),
}

/// Where a promise has a home, or None: before this a typed promise to
/// anyone but a player was dropped (Cain 2026-11-16).
pub fn promise_home(w: &World, him: Them, p: &PromiseRead) -> Option<Home> {
    use super::wants::Kind;
    match him {
        Them::Player(pid) => Some(Home::Player(pid)),
        // A promise to his family is about the son (the parent call's chip
        // carries one): P1 keeps it with him.
        Them::Kin(pid, _) => Some(Home::Player(pid)),
        // A coach on your staff: a stay, or a raise or a title.
        Them::Coach(c) if super::makeup::kind(w, him) == Kind::Staff => match p.kind.as_str() {
            "stay" => Some(Home::To(talks::PromiseTo::Coach(c), talks::PromiseKind::Stay)),
            "contract" => Some(Home::To(talks::PromiseTo::Coach(c), talks::PromiseKind::Raise)),
            _ => None,
        },
        Them::Cast(r, n) if p.kind == "stay" && super::makeup::kind(w, him) == Kind::Boss => {
            Some(Home::To(talks::PromiseTo::Cast(r, n), talks::PromiseKind::Stay))
        }
        _ => None,
    }
}

/// Words that put an ask to you now, with no question mark: "tell me
/// where I stand", "I need an answer".
const ASK_NOW: [&str; 12] = [
    "tell me",
    "let me know",
    "i'd like to know",
    "i want to know",
    "i need to know",
    "i'd like to hear",
    "i want to hear",
    "i need to hear",
    "answer me",
    "an answer",
    "waiting on you",
    "be straight with me",
];

/// Whether the man's last line in the scene asked you something now: a
/// question, or words that put it to you ("tell me where I stand"). An
/// ask he said he'd take up later ("once the staff is set I'd like to
/// talk about where I fit in") asks nothing yet.
fn asked_last(s: &Scene) -> bool {
    s.lines.iter().rev().find(|l| l.by != 0).is_some_and(|l| {
        let t = l.text.replace('\u{2019}', "'").to_lowercase();
        l.by != 1 && (t.contains('?') || ASK_NOW.iter().any(|w| t.contains(w)))
    })
}
