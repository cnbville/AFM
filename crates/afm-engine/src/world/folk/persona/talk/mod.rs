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

pub mod channels;
pub mod feel;
pub mod hits;
pub mod items;
pub mod store;
pub mod today;
pub mod who;

// Facet lanes (N5a-N5e): each plays its own talks on this conversation.
pub mod hs_call;
pub mod recruit;

use super::Them;
use super::ear::{Act, Confirm, Ends, HisAsk, Item, Reading, Resolving, Say, Sized, Turn};
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
            t.patience = self.parts.feel.patience(w, him, &open, &t, false);
            t.say.released.push("He doesn't follow what you mean.".into());
            return t;
        }
        let wants = self.parts.wanting.wants(w, him);
        let s = scene(w, self.said.scene);
        let facet = s.map_or(Facet::Other, Facet::of);
        let mut promise_k = 0usize;
        let mut facts: Vec<Stated> = Vec::new();
        for act in &reading.acts {
            match act {
                Act::Question { topic, about, .. } => {
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
                    // Hot past his point and gone quiet: the safe half at most.
                    if open.boiled == Some(Boil::Quiet) && told.answer == Answer::Straight {
                        told.answer = Answer::Partly;
                        told.released.truncate(1);
                    }
                    t.minutes += told.cost.minutes;
                    if told.cost.trust > 0.0 {
                        let item = match about {
                            Some(Them::Player(id)) => Item::Friend(*id),
                            _ => Item::HisWord,
                        };
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
                Act::Offer(o) => {
                    let wd = self.parts.valuing.weigh(w, him, o);
                    match &wd {
                        Weighed::Yes => {
                            t.confirm.push(Confirm::Offer(o.clone()));
                            t.say.released.push("He'd take that.".into());
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
                        Weighed::No => t.say.released.push("He doesn't take it up.".into()),
                    }
                    t.weighed.push((o.clone(), wd));
                }
                Act::Promise { .. } if facet == Facet::Recruit => {
                    // A recruit's promise is the recruiting desk's (`pledge`),
                    // made in `apply`.
                    if let (Some(s), Some(p)) = (s, self.said.promises.get(promise_k)) {
                        t.say.released.push(recruit::promise_released(w, s, p));
                    }
                    promise_k += 1;
                }
                Act::Promise { .. } => {
                    // Logged through P1 in `apply`, in order: its index then.
                    if self.said.promises.get(promise_k).is_some() {
                        t.promised.push((w.folk.promises.len() + t.promised.len()) as u32);
                    }
                    promise_k += 1;
                }
                Act::Request(r) => {
                    // The recruiting desk answers what it owns (commit).
                    let own = s.and_then(|s| recruit::willing(w, s, r));
                    if let Some((_, words)) = &own {
                        t.say.released.push(words.clone());
                    }
                    let wl = own.map_or_else(|| self.parts.acting.willing(w, him, r), |x| x.0);
                    match &wl {
                        Willing::Yes => {
                            // One that costs you waits on your confirm; a
                            // favour of goodwill runs as he agrees (`apply`).
                            if super::asks::costs_you(r) {
                                t.confirm.push(Confirm::Request(r.clone()));
                            }
                            t.say.released.push("He'll do it.".into());
                        }
                        Willing::If(c) => {
                            t.say.released.extend(c.asks.iter().cloned());
                            t.counter = Some(c.clone());
                        }
                        Willing::Later(d) => t.say.released.push(format!("He'll let you know by {}.", d)),
                        Willing::No => {
                            t.say.released.push("He won't do that.".into());
                            if open.refused_before(r) {
                                t.patience.0 -= feel::ASKED_AGAIN;
                            }
                        }
                    }
                    t.asked.push((r.clone(), wl, None));
                }
                Act::Decision(d) => {
                    t.confirm.push(Confirm::Decision(*d));
                    t.say.released.push(format!("You tell him: {}.", channels::decision_words(*d)));
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
                        t.say.released.push(format!("You show him {}.", channels::show_words(*sh)));
                    }
                }
                Act::Answer(_) => {}
                Act::End => t.ends = Some(Ends::You),
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
        if reading.dodged {
            t.hits.push(hits::dodged(w, him));
        }
        if facet == Facet::Other {
            hits::settle(&mut t.hits);
        }
        match (facet, s) {
            (Facet::Recruit, Some(s)) => {
                recruit::cap(w, s, &mut t.hits);
                t.say.released.extend(recruit::released(w, s, &t.hits, &wants));
            }
            (Facet::HsCall, Some(_)) => {}
            (_, Some(s)) => hits::cap(w, s, room, &mut t.hits),
            _ => {}
        }
        // Someone in your building: inside what his meeting's chips can do
        // (lane N5c).
        super::office::cap(w, room, &mut t.hits);
        // His own ask (5.4, question 3): what he wants most that you
        // haven't given, once a talk per want.
        if t.ends.is_none()
            && let Some(want) = wants.iter().find(|x| !x.met && x.weight >= ASK_WEIGHT && !open.raised(x.kind))
            && line >= 1
        {
            t.his_asks.push(HisAsk {
                why: *want,
                question: ask_topic(want.kind),
                asks: vec![ask_words(want.kind)],
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
        let landed = t.hits.iter().any(|h| (h.trust + h.mood).abs() > 0.01) || !t.told.is_empty();
        t.patience = self.parts.feel.patience(w, him, &open, &t, landed);
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
                if h.hit.on != him || h.trust == 0.0 {
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
        // (a recruit's through the recruiting desk, above).
        if let Them::Player(pid) = him
            && facet == Facet::Other
        {
            for p in self.said.promises.iter().chain(standing.iter()) {
                talks::typed_promise(w, id, pid, p);
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
        // that costs you waits on your confirm.
        for (r, wl, _) in &t.asked {
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
        if self.said.reading.dodged {
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

/// His loyalty to the man a question is about, 0-1: their tie.
fn loyalty(w: &World, him: Them, about: Option<Them>) -> f64 {
    let (Them::Player(a), Some(Them::Player(b))) = (him, about) else { return 0.0 };
    if a == b {
        return 0.0;
    }
    w.folk.ties.of_id(a).filter(|t| t.a == b || t.b == b).map(|t| f64::from(t.strength) / 100.0).fold(0.0, f64::max)
}

/// A want heavy enough that he raises it himself. Proposed, not confirmed.
pub const ASK_WEIGHT: f32 = 0.22;

/// What he asks about a want of his.
fn ask_topic(k: WantKind) -> Option<Topic> {
    Some(match k {
        WantKind::Role | WantKind::Development | WantKind::Scheme => Topic::Football,
        WantKind::Money | WantKind::Security | WantKind::Fee | WantKind::Club => Topic::Market,
        WantKind::Career | WantKind::Goals => Topic::Jobs,
        _ => return None,
    })
}

/// His ask in words, for the voice (`HisAsk.asks`).
pub fn ask_words(k: WantKind) -> String {
    match k {
        WantKind::Role => "he asks where he stands and what it takes to play more",
        WantKind::Money => "he asks about his money",
        WantKind::Security => "he asks how safe his spot is",
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
        // A built staff or boss meeting keeps its own picks to finish
        // (lane N5c): the talk ending leaves the meeting open, and the
        // talk's state stays so a man done talking stays done today.
        Facet::Other if super::office::plays(s.kind) && !super::office::talk_only(s.kind) => return,
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
pub fn reply(w: &mut World, id: u32, words: &str, tell: Option<String>, colour: Option<String>) {
    let Some(s) = scene(w, id).cloned() else { return };
    let room = store::room(w, id).unwrap_or_else(|| channels::room_for(w, &s));
    let Some(him) = across(&room) else { return };
    // The one who answered speaks the line (his decider, his agent).
    let name = channels::name(w, him);
    if let Some(sc) = w.folk.scenes.iter_mut().find(|x| x.id == id) {
        // A talk that ended on this line still shows his last words.
        sc.them(&name, words.to_string(), tell);
    }
    let today = w.today;
    let m = &mut w.folk.live.persona;
    store::his_words(m, him, s.date, words);
    if let Some(c) = colour {
        store::note(m, him, today, c);
    }
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
