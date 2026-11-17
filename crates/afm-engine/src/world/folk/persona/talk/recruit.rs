//! Live talks v3, lane N5a: recruiting (`docs/design/live-talks-persona.md`,
//! sections 6.3 to 6.5). Your home visits, calls, messages, his family's
//! call, the official visit's sit-down and a transfer's call and visit play
//! as v3 talks: the ear reads your line, the game decides, the voice
//! answers as him (or as whoever decides for him).
//!
//! What a typed line does in a recruit's talk, all by the game's own rules:
//!
//! - **Your points land on your case** (`case.rs`), never on a mood: a hit
//!   on one of his wants makes your best point in its group, a hit on a
//!   point makes that point, each sized by the chips' own formula
//!   (`case::lands`, `pitch::typed_point`). One point an exchange, as the
//!   chips make them, and no more points than the talk has exchanges
//!   (`cap`). Wording never beats the best chip (lane 9's rule).
//! - **His trust in you** only falls on a line (an insult, a threat, a sting
//!   to his family): no chip in a recruit's talk raises it, so no line does.
//!   A straight answer ("doubted" his thing where your proof is weak) is the
//!   chip's "be straight": no pull, and he believes your promises more.
//! - **Questions** are lane N1's (what he knows and how he tells it); claims
//!   are lane 10's checks, with lane 10's costs when he catches you.
//! - **A promise** counts the moment you say it (question 4): the chip's
//!   promise (`pledge::make`, a transfer's `portal::promise`).
//! - **Offers** are lane N3's (`Deals`, against his best other school); a
//!   yes or a "let me talk to my mom" waits on your confirm, and confirmed
//!   it is made real on the recruiting desk (`real`).
//! - **Asking him to commit** runs the desk's own commitment rule
//!   (`college::board_check`, a transfer's `portal::check`): he says yes
//!   only when he would commit to you today.
//! - **An invitation to visit** waits on your confirm, then the desk sets
//!   it on its first open date, and he may still say no (the built rule).
//! - **The family's answer on its day** (`day`): weighed again with their
//!   say, made real on a yes, and either way a line in your inbox.
//!
//! Every size here is Claude's (proposed, not confirmed).

use super::super::Them;
use super::super::deals::{self, Agreement};
use super::super::ear::{Act, HisAsk, Hit, How, Item, Landing, Reading, Sized, Turn};
use super::super::makeup;
use super::super::room::{Channel, Place, Present, Room, Seat};
use super::super::stores;
use super::super::terms::{Give, Offer, Request, Valuing, Weighed};
use super::super::wants::{Want, WantKind, Willing};
use super::store::Open;
use super::{DEALS, hits};
use crate::calendar::{Date, Loudness};
use crate::world::World;
use crate::world::background::Decider;
use crate::world::folk::claims::{self, Group, Stated, truth::STANDING};
use crate::world::folk::pitch::{self, Typed, With};
use crate::world::folk::pulls::{Ev, Where};
use crate::world::folk::scene::{Kind as SceneKind, PromiseRead, Scene};
use crate::world::folk::{self as folk, case, pledge};
use crate::world::front_office::{self, college::VisitKind};

/// The recruit's talks that play as v3.
pub const KINDS: [SceneKind; 7] = [
    SceneKind::HomeVisit,
    SceneKind::OfficialTalk,
    SceneKind::RecruitCall,
    SceneKind::RecruitMessage,
    SceneKind::FamilyCall,
    SceneKind::PortalCall,
    SceneKind::PortalVisit,
];

/// A flattering line sells at this share of the point it flatters.
/// Proposed, not confirmed.
pub const FLATTER: f32 = 0.5;
/// The most one line can cost his trust in you (the worst chip in a
/// recruit's talk is a caught oversell, lane 10). Proposed, not confirmed.
pub const TRUST_FLOOR: f32 = -0.04;
/// The most points his card lists that you could still make. Proposed, not
/// confirmed.
pub const MAX_POINTS: usize = 8;
/// A line landed with him when its point moved your case at least this
/// share of what your best point could right now (agreed by Cain
/// 2026-11-17, the landing read).
pub const LANDED_SHARE: f32 = 0.5;
/// Less than this on your case, either way, is no move at all.
const MOVED: f32 = 1e-5;

/// Whether a scene is a recruit's talk.
pub fn is_recruit(k: SceneKind) -> bool {
    KINDS.contains(&k)
}

/// Who the talk is with, if it's a recruit's talk.
pub fn with(w: &World, s: &Scene) -> Option<With> {
    if !is_recruit(s.kind) {
        return None;
    }
    pitch::with(w, s)
}

/// Whether he decides for himself.
fn decides_himself(w: &World, x: &With) -> bool {
    x.transfer || w.people[x.i].family.decider == Decider::Himself
}

/// Who answers for him: his decider (family slot 0), or a parent when he
/// decides himself.
pub fn family_voice(w: &World, x: &With) -> (Them, Seat) {
    if decides_himself(w, x) { (Them::Kin(x.id, 1), Seat::Family) } else { (Them::Kin(x.id, 0), Seat::Decider) }
}

/// The room (5.5, 5.6): a living room with whoever decides for him, your
/// office at the end of an official visit with the family, a call or a
/// text with him, his family's call through the one who called, a
/// transfer on his own.
pub fn room(w: &World, s: &Scene) -> Option<Room> {
    let x = with(w, s)?;
    let him = Present { who: Them::Player(x.id), seat: Seat::Across, speaks: true };
    let (fam, seat) = family_voice(w, &x);
    let family = Present { who: fam, seat, speaks: true };
    let with_family = !decides_himself(w, &x);
    let (channel, present) = match (s.kind, x.at) {
        (SceneKind::FamilyCall, _) => (Channel::Through(fam), vec![him, family]),
        (_, Where::Message) => (Channel::Text, vec![him]),
        (_, Where::Call) => (Channel::Phone, vec![him]),
        (_, Where::Home) if with_family => (Channel::InPerson(Place::Home), vec![him, family]),
        (_, Where::Home) => (Channel::InPerson(Place::Home), vec![him]),
        (_, _) if with_family => (Channel::InPerson(Place::Office), vec![him, family]),
        _ => (Channel::InPerson(Place::Office), vec![him]),
    };
    Some(Room { channel, present })
}

/// The case group a want of his is about.
pub fn group_of(k: WantKind) -> Option<Group> {
    Some(match k {
        WantKind::Role | WantKind::Security => Group::PlayingTime,
        WantKind::Money => Group::Money,
        WantKind::Academics => Group::School,
        WantKind::Place | WantKind::Family => Group::Home,
        WantKind::Friends | WantKind::Loyalty => Group::People,
        WantKind::Development | WantKind::Career => Group::League,
        WantKind::Winning | WantKind::Name => Group::Winning,
        _ => return None,
    })
}

/// His card (5.2): his wants (his decider's with his own), the points you
/// could still make to him in this talk in your chips' words, his pride,
/// your promise to him, his family, and what he told you before.
pub fn items(w: &World, s: &Scene, wants: &[Want]) -> Option<Vec<(Item, String)>> {
    let x = with(w, s)?;
    let mut out = Vec::new();
    for (k, v) in wants.iter().enumerate() {
        out.push((Item::Want(k as u8), format!("what he wants: {}", super::items::want_words(v.kind))));
    }
    let cares = pitch::typed_cares(w, s).unwrap_or([1.0; 7]);
    let mut pts = pitch::typed_points(w, s);
    pts.sort_by(|a, b| {
        let c = |k: &claims::Kind| k.group().map_or(0.0, |g| cares[g as usize]);
        c(b).total_cmp(&c(a)).then(a.cmp(b))
    });
    for k in pts.into_iter().take(MAX_POINTS) {
        let Some(g) = k.group() else { continue };
        let words = pitch::typed_point_words(w, s, k).unwrap_or_default();
        out.push((
            Item::Point(k.key().to_string()),
            format!("what you can tell him ({}): {}", g.name(), folk::plain(&words)),
        ));
    }
    out.push((Item::Pride, "his pride".into()));
    if let Some(p) = promise_words(w, &x) {
        out.push((Item::HisWord, format!("your promise to him: {p}")));
    }
    out.push((Item::Family, "his family".into()));
    Some(out)
}

/// Your promise to him in words, if you made one.
fn promise_words(w: &World, x: &With) -> Option<String> {
    if x.transfer {
        return front_office::portal::promised(w, x.id)
            .map(|(k, _)| if k == 0 { "a starting job".into() } else { "a real role".into() });
    }
    let pos = w.people[x.i].player.position;
    pledge::on_case(w, x.i, x.j).map(|k| k.words(pos))
}

/// The point a hit makes, if any: a point on his card, or your best point
/// in the group of the want, or his family's (close to home).
fn target(w: &World, s: &Scene, item: &Item, wants: &[Want]) -> Option<Typed> {
    match item {
        Item::Point(key) => pitch::typed_point(w, s, claims::Kind::from_key(key), None),
        Item::Want(k) => pitch::typed_point(w, s, None, group_of(wants.get(*k as usize)?.kind)),
        Item::Family => pitch::typed_point(w, s, None, Some(Group::Home)),
        _ => None,
    }
}

/// The group a hit is about, when it has one.
fn group_for(item: &Item, wants: &[Want]) -> Option<Group> {
    match item {
        Item::Point(key) => claims::Kind::from_key(key).and_then(|k| k.group()),
        Item::Want(k) => wants.get(*k as usize).and_then(|x| group_of(x.kind)),
        Item::Family => Some(Group::Home),
        _ => None,
    }
}

/// Whether a person in the room hears for his case: him, or his family.
fn on_case(x: &With, who: Them) -> bool {
    matches!(who, Them::Player(id) | Them::Kin(id, _) if id == x.id)
}

/// One hit, sized for a recruit (question 1): the case pull it makes
/// (carried in `Sized.mood`: a recruit has no mood toward you) and what it
/// costs his trust in you. None when it names nothing real for him.
pub fn size(w: &World, s: &Scene, h: &super::super::ear::Hit, wants: &[Want], open: &Open) -> Option<Sized> {
    let x = with(w, s)?;
    if !on_case(&x, h.on) {
        return None;
    }
    let fade = hits::AGAIN_IN_TALK.powi(open.times(h) as i32);
    let ear = 0.6 + 0.8 * makeup::trust(w, h.on) as f32;
    let floor = case::POINT_RANGE.0 * x.scale;
    let pt = target(w, s, &h.item, wants);
    let grouped = group_for(&h.item, wants).is_some();
    let ego = super::z(w, Them::Player(x.id), crate::player::Trait::Ego).max(0.0);
    let temper = super::z(w, Them::Player(x.id), crate::player::Trait::Temper).max(0.0);
    let (trust, lean) = match h.how {
        // A stake reads as reassurance on the recruiting desk, which
        // prices its own pitch (`pitch`).
        How::Reassured | How::Respected | How::Staked => (0.0, pt.map_or(0.0, |p| p.lean)),
        How::Flattered => (0.0, pt.map_or(0.0, |p| FLATTER * p.lean)),
        // A straight answer: no pull (applied as the chip's "be straight").
        How::Doubted => (0.0, 0.0),
        How::Stung => (-0.02 * ear, if grouped { 0.5 * floor } else { 0.0 }),
        How::Insulted => (-(0.07 + 0.04 * ego) * ear, if grouped { floor } else { 0.0 }),
        How::Threatened => (-(0.05 + 0.03 * temper) * ear, if grouped { floor } else { 0.0 }),
    };
    if trust == 0.0 && lean == 0.0 && h.how != How::Doubted {
        return None;
    }
    Some(Sized { hit: h.clone(), trust: trust * fade, mood: lean * fade, faded: fade < 1.0 })
}

/// Whether a hit is on something your case can make a point of: a point,
/// one of his wants, his family.
fn on_point(item: &Item) -> bool {
    matches!(item, Item::Point(_) | Item::Want(_) | Item::Family)
}

/// Holds a line inside the chips' caps: one point an exchange (the best
/// positive pull, else the worst), none once the talk's exchanges are
/// used, and each person's trust no lower than `TRUST_FLOOR`. Only a point
/// pulls: how you said it (your manner, on his pride) lands on his trust
/// beside the point, never in its place. Before, a caring manner's lift
/// was weighed against the point as if it were a pull, won, and left no
/// point to land (the landing read, agreed by Cain 2026-11-17).
pub fn cap(w: &World, s: &Scene, hits: &mut [Sized]) {
    let left = pitch::exchanges_left(w, s);
    for h in hits.iter_mut().filter(|h| !on_point(&h.hit.item)) {
        h.mood = 0.0;
    }
    let best = (0..hits.len()).filter(|&k| hits[k].mood > 0.0).max_by(|&a, &b| hits[a].mood.total_cmp(&hits[b].mood));
    let worst = (0..hits.len()).filter(|&k| hits[k].mood < 0.0).min_by(|&a, &b| hits[a].mood.total_cmp(&hits[b].mood));
    let keep = best.or(worst);
    for (k, h) in hits.iter_mut().enumerate() {
        if left == 0 || Some(k) != keep {
            h.mood = 0.0;
        }
    }
    let mut people: Vec<Them> = hits.iter().map(|h| h.hit.on).collect();
    people.dedup();
    for p in people {
        let sum: f32 = hits.iter().filter(|h| h.hit.on == p).map(|h| h.trust).sum();
        if sum < TRUST_FLOOR {
            let k = TRUST_FLOOR / sum;
            for h in hits.iter_mut().filter(|h| h.hit.on == p) {
                h.trust *= k;
            }
        }
    }
}

/// The point a line makes on your case after `cap`, and the pull it lands
/// at (never over the chip's own size): what `apply` lands and the landing
/// read measures.
fn kept(w: &World, s: &Scene, hits: &[Sized], wants: &[Want]) -> Option<(Typed, f32)> {
    let h = hits.iter().find(|h| h.mood != 0.0)?;
    let p = target(w, s, &h.hit.item, wants)?;
    let lean = if h.mood > 0.0 { h.mood.min(p.lean.max(0.0)) } else { h.mood };
    Some((p, lean))
}

/// The straight answer a line gives when it makes no point (the chip's "be
/// straight"): his doubt on something in a group, with exchanges left.
/// The group, and the point when the line named one.
fn straight(w: &World, s: &Scene, hits: &[Sized], wants: &[Want]) -> Option<(Group, Option<claims::Kind>)> {
    let h = hits.iter().find(|h| h.hit.how == How::Doubted)?;
    let g = group_for(&h.hit.item, wants)?;
    if pitch::exchanges_left(w, s) == 0 {
        return None;
    }
    let kind = match &h.hit.item {
        Item::Point(key) => claims::Kind::from_key(key),
        _ => None,
    };
    Some((g, kind))
}

/// Whether you have something that backs what a hit named, made in this
/// talk already or not.
fn backed(w: &World, s: &Scene, item: &Item, wants: &[Want]) -> bool {
    match item {
        Item::Point(key) => pitch::typed_backed(w, s, claims::Kind::from_key(key), None),
        _ => pitch::typed_backed(w, s, None, group_for(item, wants)),
    }
}

/// A recruit's line read on your case, in the case's own size (the landing
/// read, agreed by Cain 2026-11-17). It landed when its point moved your
/// case at least `LANDED_SHARE` of what your best point could right now,
/// and helped a little when it moved it less. It didn't land when it cost
/// his trust past a player's lower bar (`hits::PLAYER_BARS`) or cost your
/// case, and cost you a little when it cost his trust less. A line that
/// named a point and moved nothing says when the talk's exchanges are
/// spent or nothing of yours backs it; said again in the same talk, it
/// moved nothing. With no point, a straight answer or a warm word that
/// lifts his trust helps a little. Read in `turn` after `cap`, before
/// anything lands.
pub fn landing(w: &World, s: &Scene, room: &Room, reading: &Reading, hits: &[Sized], wants: &[Want]) -> Landing {
    let Some(x) = with(w, s) else { return Landing::Untouched };
    let here = |who: Them| on_case(&x, who) && room.present.iter().any(|p| p.who == who);
    // What the line put to him as a point: warm words or flattery on
    // something your case can make a point of.
    let warm = |how: How| matches!(how, How::Reassured | How::Respected | How::Staked | How::Flattered);
    let named: Vec<&Hit> = reading.hits.iter().filter(|h| here(h.on) && on_point(&h.item) && warm(h.how)).collect();
    let mine: Vec<&Sized> = hits.iter().filter(|h| on_case(&x, h.hit.on)).collect();
    if mine.is_empty() && named.is_empty() {
        return Landing::Untouched;
    }
    let trust: f32 = mine.iter().map(|h| h.trust).sum();
    let moved = kept(w, s, hits, wants).map_or(0.0, |(p, lean)| pitch::typed_moves(w, s, &p, lean));
    let (up, down) = hits::PLAYER_BARS;
    let small = hits::SMALL * up;
    if trust < down || moved < -MOVED {
        return Landing::Missed;
    }
    if moved > MOVED {
        return if moved >= LANDED_SHARE * pitch::typed_best(w, s) { Landing::Landed } else { Landing::Little };
    }
    if trust < -small {
        return Landing::Cost;
    }
    if straight(w, s, hits, wants).is_some() {
        return Landing::Little;
    }
    if !named.is_empty() {
        if pitch::exchanges_left(w, s) == 0 {
            return Landing::Spent;
        }
        if !named.iter().any(|h| backed(w, s, &h.item, wants)) {
            return Landing::Unbacked;
        }
    }
    if trust > small { Landing::Little } else { Landing::Nothing }
}

/// How he takes the point that landed, in words for the voice: his own
/// words, quoted, or his family's (`family`, their name) when they do the
/// talking. "He says that's exactly what he's looking for" came back as
/// "He's exactly what I'm looking for", "He says that point doesn't matter
/// much to him" as "you're right that it doesn't matter much to me", and
/// with his parents answering, as "Andrew says... 'That doesn't matter
/// much to me'" (the talks audit, 2026-11-16).
pub fn reaction_words(ev: Ev, family: Option<&str>) -> String {
    let (says, are, looking, me, believe) = match family {
        Some(f) => {
            let many = f.ends_with("parents");
            (
                format!("{f} {}", if many { "say" } else { "says" }),
                format!("{f} {}", if many { "are" } else { "is" }),
                "we're",
                "us",
                "we don't",
            )
        }
        None => ("He says".to_string(), "He's".to_string(), "I'm", "me", "I don't"),
    };
    match ev {
        // "Came for" on a call read as the coach's reason for calling ("that's
        // what you came for, right?", the talks audit, 2026-11-16).
        Ev::TookWell => format!("{says}: \"That's exactly what {looking} looking for.\""),
        Ev::Polite => format!("{are} polite about it, no more."),
        // "It isn't his thing" came back as "football's not really his thing
        // to talk about" (the talks audit, 2026-11-16).
        Ev::Cool => format!("{says}: \"That doesn't matter much to {me}.\""),
        Ev::Doubted => format!("{says}: \"That matters to {me}, but {believe} believe you can back it up.\""),
        _ => String::new(),
    }
}

/// What the voice may say about the point the line made (after `cap`);
/// `family` is his family's name when they answer the line, and `pitched`
/// whether the line made a point at all (a question or an invite doesn't).
pub fn released(
    w: &World,
    s: &Scene,
    hits: &[Sized],
    wants: &[Want],
    family: Option<&str>,
    pitched: bool,
) -> Vec<String> {
    let mut out = Vec::new();
    if pitched
        && let Some(h) = hits.iter().find(|h| h.mood != 0.0)
        && let Some(p) = target(w, s, &h.hit.item, wants)
        && let Some(ev) = pitch::typed_reaction(w, s, &p, h.mood)
    {
        let words = reaction_words(ev, family);
        if !words.is_empty() {
            out.push(words);
        }
    }
    if pitch::exchanges_left(w, s) == 0
        && hits.iter().any(|h| matches!(h.hit.how, How::Reassured | How::Respected | How::Staked))
    {
        out.push("He has heard your case for today.".into());
    }
    out
}

/// A claim about where he'll stand is a promise about what he'll see
/// (lane 10's rule): a first place or ahead of a named player is a start,
/// anything else a role.
pub fn standing_promise(x: &With, facts: &[Stated]) -> Option<PromiseRead> {
    let f =
        facts.iter().find(|f| STANDING.contains(&f.kind.as_str()) && (f.about.is_none() || f.about == Some(x.id)))?;
    let v = f.value.to_lowercase();
    let first = ["start", "1st", "first", "the ones", "number one", "no. 1", "day one"].iter().any(|k| v.contains(k));
    let kind = if f.kind == "depth_rank" && first || f.kind == "ahead_of_him" { "start" } else { "role" };
    Some(PromiseRead { kind: kind.into(), claim: Some(f.clone()), ..PromiseRead::default() })
}

/// How a promise lands, for the voice, before it's made: his own words,
/// quoted, or his family's (`family`, their name) when they do the
/// talking. "They've heard promises before. They aren't sure they believe
/// yours." came back word for word from his parents (the talks audit,
/// 2026-11-16).
pub fn promise_released(w: &World, s: &Scene, p: &PromiseRead, family: Option<&str>) -> String {
    if !pitch::can_promise_now(w, s) {
        return "You've made him a promise already, or your offer isn't out: this one adds nothing.".into();
    }
    let (says, takes, we, have, are) = match family {
        Some(f) => {
            let many = f.ends_with("parents");
            (
                format!("{f} {}", if many { "say" } else { "says" }),
                format!("{f} {}", if many { "take" } else { "takes" }),
                "we",
                "We've",
                "We're",
            )
        }
        None => ("He says".to_string(), "He takes".to_string(), "I", "I've", "I'm"),
    };
    let we_cap = if we == "I" { "I" } else { "We" };
    match pitch::promise_belief(w, s, &p.kind) {
        Some(b) if b >= pledge::BELIEVES => {
            format!("{says}: \"{we_cap} believe you. That's what {we} wanted to hear.\"")
        }
        Some(_) => format!("{says}: \"{have} heard promises before. {are} not sure {we} believe yours.\""),
        None if p.kind == "usage" && p.usage.is_some() => format!("{takes} in how you'd use him."),
        None => "That isn't a promise this talk can make.".into(),
    }
}

/// Whether he'd do what you ask, where the recruiting desk decides it:
/// commit (its own rule), and shut it down (only once he's yours). None:
/// lane N3's `Asks` decides.
pub fn willing(w: &World, s: &Scene, r: &Request) -> Option<(Willing, String)> {
    let x = with(w, s)?;
    match r {
        Request::Commit => Some(if ready(w, &x) {
            (Willing::Yes, "He's ready: he commits to you.".into())
        } else if !offered(w, &x) {
            (Willing::No, "He'd need your offer before he could commit.".into())
        } else {
            (Willing::No, "He isn't ready to commit: he wants to see his other schools through.".into())
        }),
        Request::ShutDown if !committed_here(w, &x) => {
            Some((Willing::No, "He hasn't committed to you: there's nothing to shut down for you.".into()))
        }
        _ => None,
    }
}

/// Whether your offer is out to him.
fn offered(w: &World, x: &With) -> bool {
    if x.transfer { front_office::portal::offered_to(w, x.id) } else { front_office::college::offered_here(w, x.id) }
}

/// Whether he's committed to you.
fn committed_here(w: &World, x: &With) -> bool {
    if x.transfer {
        front_office::portal::committed_to(w, x.id)
    } else {
        w.people[x.i].cal.commit.is_some_and(|c| c.0 == x.j)
    }
}

/// Whether he'd commit to you today, by the desk's own rule.
pub fn ready(w: &World, x: &With) -> bool {
    if x.transfer { front_office::portal::ready(w, x.id) } else { front_office::college::commit_ready(w, x.j, x.id) }
}

/// Runs a request the recruiting desk owns, once he agreed: commit runs
/// the desk's commitment (`board_check`, `portal::check`). None: lane
/// N3's `Asks::run` runs it.
pub fn run(w: &mut World, s: &Scene, r: &Request) -> Option<Vec<String>> {
    let x = with(w, s)?;
    match r {
        Request::Commit => {
            let said = if x.transfer {
                front_office::portal::check(w, x.id)
            } else {
                front_office::college::board_check(w, x.j, x.id)
            };
            Some(vec![said.unwrap_or_else(|| "He holds off after all.".into())])
        }
        _ => None,
    }
}

/// The first day an official visit could be set for him (a transfer's
/// campus visit), with the desk's word for it.
pub fn visit_day(w: &World, x: &With) -> Option<Date> {
    let dates = if x.transfer {
        front_office::portal::visit_dates(w, x.id, VisitKind::Official)
    } else {
        front_office::college::visit_dates(w, x.id, VisitKind::Official)
    };
    dates.first().map(|d| d.0)
}

/// Sets the visit you invited him to, on its first open date. He may
/// still say no (the desk's own rule).
pub fn set_visit(w: &mut World, s: &Scene) -> String {
    let Some(x) = with(w, s) else { return "That talk is over.".into() };
    let Some(d) = visit_day(w, &x) else { return "There's no open date for his visit.".into() };
    let r = if x.transfer {
        front_office::portal::set_visit(w, x.id, VisitKind::Official, d)
    } else {
        front_office::college::set_visit(w, x.id, VisitKind::Official, d)
    };
    match r {
        Ok(t) | Err(t) => t,
    }
}

/// The visit request's name on a confirm card (`Request::Other`).
pub const VISIT: &str = "official visit";

/// His own asks with what he'd want given (question 3): lane N3's counter
/// to the deal you haven't made, from what he wants most.
pub fn his_ask_gives(w: &World, him: Them, ask: &mut HisAsk) {
    if let Weighed::Counter(c) = DEALS.weigh(w, him, &Offer::default()) {
        ask.gives = c.gives;
        ask.terms = c.terms;
    }
}

/// Makes what lane N3 left real on the recruiting desk (question 4: you
/// confirmed it): his scholarship, the money you said (a NIL promise; a
/// transfer's offer), his brother's offer, how you'd use him. A staff job
/// for his coach is the hiring screen's. Returns the desk's words.
pub fn real(w: &mut World, him: Them, a: &Agreement) -> Vec<String> {
    let Them::Player(id) = him else { return Vec::new() };
    let Some(i) = folk::person(w, id) else { return Vec::new() };
    let transfer = w.people[i].cal.portal.is_some() && w.people[i].stage != crate::world::Stage::HighSchool;
    let mut out = Vec::new();
    let money = a.left.iter().find_map(|g| if let Give::Money(x) = g { Some(*x) } else { None });
    let scholarship = a.left.iter().any(|g| matches!(g, Give::Scholarship(None)));
    let say = |r: Result<String, String>| match r {
        Ok(t) | Err(t) => t,
    };
    if transfer {
        if scholarship || money.is_some() {
            out.push(say(front_office::college::portal_offer(w, id, money, None)));
        }
    } else {
        if scholarship || (money.is_some() && !front_office::college::offered_here(w, id)) {
            out.push(say(front_office::college::offer(w, id)));
        }
        if money.is_some() {
            out.push(say(front_office::college::set_promise(w, id, money, None)));
        }
    }
    for g in &a.left {
        match g {
            Give::Scholarship(Some(b)) => out.push(say(front_office::college::offer(w, *b))),
            Give::StaffJob(_) => out.push("A job for his coach is set on the hiring screen.".into()),
            Give::EarlyEnrol => out.push("Early enrolment is his to choose when he signs.".into()),
            Give::Outside(_) => out.push("Your staff won't touch money outside the rules.".into()),
            _ => {}
        }
    }
    for u in &a.left_terms {
        let made = if transfer {
            front_office::portal::promise_usage(w, id, *u).is_some()
        } else {
            let (j, ear) = (front_office::program(w), claims::Ear::Decider);
            j.and_then(|j| pledge::make_usage(w, i, j, *u, ear)).is_some()
        };
        if made {
            out.push(format!("You promised {}. It's checked once he's yours.", u.words()));
        }
    }
    out
}

/// The game's turn for a recruit's talk, on top of lane N4's (`turn`
/// calls this once it has played the acts): his asks with gives, a
/// family's "let me think" waiting on your confirm, and what the desk
/// decides about commit.
pub fn turn(w: &World, s: &Scene, reading: &Reading, t: &mut Turn) {
    let Some(x) = with(w, s) else { return };
    let him = Them::Player(x.id);
    for a in &mut t.his_asks {
        his_ask_gives(w, him, a);
    }
    for (o, wd) in &t.weighed {
        if let Weighed::Later(_) = wd {
            t.confirm.push(super::super::ear::Confirm::Offer(o.clone()));
        }
    }
    for act in &reading.acts {
        if let Act::Invite(super::super::ear::Invite::Visit) = act {
            match visit_day(w, &x) {
                Some(d) => {
                    // What the coach did, from the recruit's side (it read
                    // "You invite him", the talks audit of 2026-11-16).
                    t.say.heard.push(format!("He invites you to an official visit; the first open date is {}", d));
                    // The invite is his to take, not one he'll "hear more
                    // about" (the general invite's words).
                    t.say.released.retain(|x| x != "He'll hear more about that once it's set.");
                    t.confirm.push(super::super::ear::Confirm::Request(Request::Other(VISIT.into())));
                }
                None => t.say.released.push("There's no open date for a visit now.".into()),
            }
        }
    }
}

/// Makes a recruit's line real (`apply` calls this in place of the
/// one-on-one's trust and mood): trust on whoever was hit, the one point
/// on your case, a straight answer, and every promise said. Returns what
/// the talk's outcome should say.
pub fn apply(w: &mut World, id: u32, t: &Turn, promises: &[PromiseRead], wants: &[Want]) {
    let Some(k) = w.folk.scenes.iter().position(|s| s.id == id) else { return };
    let mut s = w.folk.scenes[k].clone();
    let Some(x) = with(w, &s) else { return };
    for h in &t.hits {
        if h.trust != 0.0 {
            stores::nudge_trust(w, h.hit.on, h.trust);
        }
    }
    if let Some((p, lean)) = kept(w, &s, &t.hits, wants) {
        let delta = pitch::land_typed(w, &mut s, &p, lean);
        s.f[0] += delta;
        s.outcome.push(format!(
            "{}: {}",
            p.group.name(),
            match pitch::typed_reaction(w, &s, &p, lean) {
                Some(Ev::TookWell) => "what he came for.",
                Some(Ev::Doubted) => "he didn't believe you could back it up.",
                Some(Ev::Cool) => "not his thing.",
                _ => "polite.",
            }
        ));
    } else if let Some((g, kind)) = straight(w, &s, &t.hits, wants) {
        pitch::straight_typed(w, &mut s, g, kind);
        s.outcome.push(format!(
            "{}: you were straight with him. No pull, and he believes your word a little more.",
            g.name()
        ));
    }
    // Promises count as you say them; a claim about where he'll stand is
    // one too.
    let facts: Vec<Stated> = t.claims.iter().map(|(f, _)| f.clone()).collect();
    let standing = if promises.is_empty() { standing_promise(&x, &facts) } else { None };
    for p in promises.iter().chain(standing.iter()) {
        if let Some((what, belief)) = pitch::promise_typed(w, &s, &p.kind, p.usage) {
            let words = if belief >= pledge::BELIEVES {
                "They believe you. It binds you."
            } else {
                "They've heard promises before. They aren't sure they believe yours."
            };
            s.outcome.push(format!("You promised {what}. {words} It's checked once he's yours."));
        }
    }
    w.folk.scenes[k] = s;
}

/// Ends a recruit's talk from typed lines as the chips end it: what the
/// talk came to, and a transfer's file.
pub fn end(w: &mut World, s: &mut Scene) {
    let d = s.f[0];
    pitch::end_typed(w, s, d);
}

/// Each evening: the answers recruits' families owe you come due (lane N3's
/// `Owed`), weighed again with their say. A yes is made real (you
/// confirmed the offer when you left it with them); every answer is a
/// line in your inbox.
pub fn day(w: &mut World, d: Date) {
    if w.folk.live.persona.owed.is_empty() {
        return;
    }
    let due = deals::due_where(w, &DEALS, deals::is_recruit);
    for (o, wd) in due {
        let Them::Player(id) = o.with else { continue };
        let Some(i) = folk::person(w, id) else { continue };
        let name = w.people[i].name.clone();
        let offer = folk::plain(&o.offer.words);
        let text = match wd {
            Weighed::Yes => {
                let a = deals::agree(w, &DEALS, o.with, &o.offer);
                let done = real(w, o.with, &a);
                format!("{name}'s family called back: yes to your offer ({offer}). {}", done.join(" "))
            }
            Weighed::Counter(c) => {
                format!("{name}'s family called back about your offer ({offer}): {}", c.asks.join("; "))
            }
            Weighed::Lowball => format!("{name}'s family called back: your offer ({offer}) isn't close."),
            _ => format!("{name}'s family called back: no to your offer ({offer})."),
        };
        folk::post_mine(w, d, Loudness::Small, true, "A family's answer on your offer", &text);
    }
}
