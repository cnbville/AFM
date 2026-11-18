//! How much a hit counts (section 5.2, question 1). The ear names a real
//! item on his card and how your words touched it; the game sizes it from
//! how much that item weighs for him today and from his traits, then holds
//! the line inside today's caps: no typed line moves a man further than
//! the best chip he could be given now, or sinks him further than the
//! worst (lane 9's rule, kept).
//!
//! Every size here is proposed, not confirmed.

use super::super::Them;
use super::super::ear::{Hit, How, Item, Sized};
use super::super::room::Room;
use super::super::wants::Want;
use super::store::Open;
use super::{trust_of, z};
use crate::player::Trait;
use crate::world::World;
use crate::world::folk::claims;
use crate::world::folk::scene::{Kind as SceneKind, Scene};
use crate::world::folk::takeover::Tone;
use crate::world::folk::talks::{self, Move, Subject, Topic};
use crate::world::folk::{self as folk, talk_points as tp};

/// A repeat of the same item and the same way in one talk counts this
/// much again, each time. Proposed, not confirmed.
pub const AGAIN_IN_TALK: f32 = 0.35;

// ---------------------------------------------------------------------------
// One strong line against generic praise (agreed 2026-11-15, Cain: "if I
// have a player i know i 100% need to keep ... 'You're my guy, you're going
// to break records for us.' Should be weighed way if said once. Then 3 or 4
// 'You're a good addition to our room.'"). Every size below is proposed,
// not confirmed, and on the "Review this later" list.

/// A stake he fully believes moves his trust and his mood this much: about
/// three times the best a chip does, and outside the line's chip cap.
pub const STAKE_TRUST: f32 = 0.16;
pub const STAKE_MOOD: f32 = 0.2;
/// A stake he doubts costs this share of the trust side ("heard that
/// before").
pub const STAKE_DOUBT: f32 = 0.4;
/// A stake made again in a later talk counts this much at once, back to
/// full over this many days. Made again in the same talk, it counts for
/// nothing: he heard you.
pub const STAKE_AGAIN: f32 = 0.15;
pub const STAKE_FRESH_DAYS: f32 = 42.0;
/// His own promises from you in his record for a stake: each one kept, and
/// each one broken, in the last two years.
pub const STAKE_KEPT: f64 = 0.08;
pub const STAKE_BROKEN: f64 = 0.15;
/// Generic praise (warm words that touch only his pride) counts at this
/// share of a hit, and the first, second, third and fourth praising line of
/// a talk at these shares; from the fifth on, nothing.
pub const PRAISE: f32 = 0.5;
pub const PRAISE_TAPER: [f32; 4] = [1.0, 1.0, 1.0, 0.5];

/// The base (trust, mood) of a hit for an average man on an item that
/// weighs 1. Proposed, not confirmed: the same order as a chip's move
/// (`talks::landing`: an Ask about 0.04 and 0.03, a Promise 0.05 and 0.14,
/// a Criticize about -0.03 and -0.08).
fn base(w: &World, on: Them, how: How) -> (f32, f32) {
    let ego = z(w, on, Trait::Ego).max(0.0);
    let prof = z(w, on, Trait::Professionalism);
    let temper = z(w, on, Trait::Temper).max(0.0);
    let comp = competitive(w, on);
    match how {
        How::Reassured => (0.04, 0.07),
        How::Respected => (0.05 + 0.02 * prof, 0.03),
        // Stung: a competitor takes some of it as a dare.
        How::Stung => (-0.02, -0.06 + 0.04 * comp.max(0.0)),
        How::Insulted => (-0.07 - 0.04 * ego, -0.06 - 0.03 * ego),
        How::Doubted => (-0.04, -0.02 + 0.02 * comp.max(0.0)),
        // Flattery lifts a big ego and a pro sees through it.
        How::Flattered => (-0.01 - 0.02 * prof.max(0.0), 0.03 + 0.03 * ego),
        How::Threatened => (-0.05 - 0.03 * temper, -0.04 + 0.04 * comp),
        // A stake to someone who isn't a player is heard as reassurance;
        // a player's is sized by `stake`.
        How::Staked => (0.04, 0.07),
    }
}

/// Generic praise: warm words that touch nothing of his but his pride.
pub fn praise(h: &Hit) -> bool {
    h.item == Item::Pride && matches!(h.how, How::Reassured | How::Respected | How::Flattered)
}

/// His belief in a stake you put on him, 0.05-0.97: the promise belief's
/// parts (`promise::Parts`) with his own record with you in it, and how
/// true it rings from where he sees himself (a starter buys "you're my
/// guy", a man buried on the depth chart less, unless his ego carries it).
pub fn stake_belief(w: &World, id: u32) -> Option<f64> {
    use crate::world::folk::promise::{self as pr, Hearer};
    use crate::world::folk::talks::PromiseTo;
    let i = folk::person(w, id)?;
    let mine = super::super::makeup::coached_by_you(w, i);
    let mut record = if mine { pr::record(w, i, Hearer::Own) } else { 0.5 };
    for p in w.folk.promises.iter().filter(|p| p.to == PromiseTo::Player(id) && p.made.days_to(w.today) <= 730) {
        match p.kept {
            Some(true) => record += STAKE_KEPT,
            Some(false) => record -= STAKE_BROKEN,
            None => {}
        }
    }
    // His place as he sees it, as P1 reads it (`promise::knows`): before
    // he has seen anything this season, last season's role.
    let first = crate::world::folk::seen::first_now(w, i)
        .unwrap_or(w.people[i].last_role == Some(crate::player::PlayingTime::Starter));
    let place = if first {
        1.0
    } else if pr::share_now(w, i) >= pr::ROLE_SHARE {
        0.8
    } else {
        0.6
    };
    let ego = f64::from(z(w, Them::Player(id), Trait::Ego).max(0.0));
    Some(
        pr::Parts {
            trust: f64::from(trust_of(w, Them::Player(id))),
            record: record.clamp(0.0, 1.0),
            likely: (place + 0.15 * ego).min(1.0),
            past: pr::past(w, i),
            ear: pr::ear(w, i),
            shape: 1.0,
        }
        .belief(),
    )
}

/// A stake on a player, sized by his belief in it: a big lift when he buys
/// it, a little trust lost when he doesn't. Nothing when you already staked
/// yourself on him in this talk; less when you did in a talk lately.
fn stake(w: &World, h: &Hit, open: &Open, id: u32) -> Option<Sized> {
    let b = stake_belief(w, id)? as f32;
    let again = open.hits.iter().any(|x| x.0 == h.on && x.2 == How::Staked);
    let fade = if again {
        0.0
    } else {
        w.folk
            .live
            .persona
            .treated
            .iter()
            .rev()
            .find(|t| t.who == h.on && t.how == super::super::memory::Treated::Staked)
            .map_or(1.0, |t| {
                let d = t.day.days_to(w.today).max(0) as f32;
                (STAKE_AGAIN + (1.0 - STAKE_AGAIN) * d / STAKE_FRESH_DAYS).min(1.0)
            })
    };
    let lift = (b - pr_unsure()) / (1.0 - pr_unsure());
    let ego = z(w, h.on, Trait::Ego).max(0.0);
    let trust = if lift >= 0.0 { STAKE_TRUST * lift } else { STAKE_TRUST * STAKE_DOUBT * lift };
    let mood = STAKE_MOOD * lift.max(0.0) * (1.0 + 0.2 * ego);
    Some(Sized { hit: h.clone(), trust: trust * fade, mood: mood * fade, faded: fade < 1.0 })
}

fn pr_unsure() -> f32 {
    crate::world::folk::promise::UNSURE as f32
}

/// Competitiveness as a z-like score, -1 to 1.
fn competitive(w: &World, on: Them) -> f32 {
    match on {
        Them::Player(id) => {
            folk::person(w, id).map_or(0.0, |i| ((w.people[i].player.personality.competitiveness - 10.0) / 10.0) as f32)
        }
        _ => 0.0,
    }
}

/// How much an item weighs for him today, about 0.4 to 1.7.
pub fn weight(w: &World, on: Them, item: &Item, wants: &[Want]) -> Option<f32> {
    Some(match item {
        Item::Want(k) => {
            let top = wants.iter().map(|x| x.weight).fold(0.0f32, f32::max).max(1e-3);
            let x = wants.get(*k as usize)?;
            0.4 + 1.2 * (x.weight / top)
        }
        Item::Point(key) => {
            let Them::Player(id) = on else { return None };
            let i = folk::person(w, id)?;
            let k = claims::Kind::from_key(key)?;
            let p = claims::player(w, i).get(k)?.clone();
            0.4 + p.mind.min(1.3)
        }
        Item::Pride => 0.6 + 0.4 * z(w, on, Trait::Ego).max(0.0),
        Item::Promise(k) => {
            let p = w.folk.promises.get(*k as usize)?;
            if p.kept == Some(false) { 1.3 } else { 1.0 }
        }
        Item::Friend(f) => {
            let Them::Player(id) = on else { return Some(0.8) };
            let s = w.folk.ties.of_id(id).filter(|t| t.a == *f || t.b == *f).map(|t| t.strength).max()?;
            0.5 + f32::from(s) / 100.0
        }
        Item::Family => {
            0.6 + wants.iter().find(|x| x.kind == super::super::wants::WantKind::Family).map_or(0.2, |x| x.weight)
        }
        Item::HisWord => 0.7,
        // One of the room: a line to the whole team (lane N5d).
        Item::Room => super::super::gameday::group::ROOM_WEIGHT,
    })
}

/// One hit, sized. None when the item isn't real for him (section 14: a
/// hit that names nothing real counts for nothing).
pub fn size(w: &World, h: &Hit, wants: &[Want], open: &Open, line: u8) -> Option<Sized> {
    let wt = weight(w, h.on, &h.item, wants)?;
    if let (How::Staked, Them::Player(id)) = (h.how, h.on) {
        return stake(w, h, open, id);
    }
    let (bt, bm) = base(w, h.on, h.how);
    // His trust in you colours everything (today's landing rule), and a
    // player close to you hears more (coach and player bonds: + 0.3 x
    // closeness).
    let close = match h.on {
        Them::Player(id) => folk::bonds::ear_you(w, id),
        _ => 0.0,
    };
    let ear = 0.6 + 0.8 * trust_of(w, h.on) + close;
    // The same thing, the same way, again: it fades in a talk, and a point
    // you made in an earlier talk counts as `Talked::again` says.
    let times = open.times(h);
    let mut fade = AGAIN_IN_TALK.powi(times as i32);
    if let (Item::Point(key), Them::Player(id)) = (&h.item, h.on)
        && let Some(i) = folk::person(w, id)
        && let Some(k) = claims::Kind::from_key(key)
        && let Some(p) = claims::player(w, i).get(k)
        && times == 0
    {
        let again = w.folk.talked.again(w.today, id, p);
        // A point already made in a talk before still lands a little when
        // the words are new; nothing at all only in the same talk.
        fade = again.max(0.25);
    }
    let noise = match h.on {
        Them::Player(id) => folk::roll_n(w, open.scene as u64 ^ (u64::from(line) << 20), u64::from(id)) as f32 * 0.02,
        _ => 0.0,
    };
    // Generic praise: a little each, and it stops adding up after three or
    // four lines of it in a talk.
    if praise(h) {
        let before = open.hits.iter().filter(|x| x.0 == h.on && x.1 == Item::Pride && praise_how(x.2)).count();
        fade *= PRAISE * PRAISE_TAPER.get(before).copied().unwrap_or(0.0);
    }
    let k = wt * ear * fade;
    Some(Sized { hit: h.clone(), trust: bt * k + noise * fade, mood: bm * k + noise * fade, faded: fade < 1.0 })
}

fn praise_how(how: How) -> bool {
    matches!(how, How::Reassured | How::Respected | How::Flattered)
}

/// One line's own repeats, before the caps: of a man's stakes in one line
/// only the first counts, and of his generic praise only the first (so a
/// line that praises him three ways is still one line of praise).
pub fn settle(hits: &mut [Sized]) {
    let mut seen: Vec<(Them, bool)> = Vec::new();
    for h in hits.iter_mut() {
        let kind = if h.hit.how == How::Staked {
            true
        } else if praise(&h.hit) {
            false
        } else {
            continue;
        };
        if seen.contains(&(h.hit.on, kind)) {
            h.trust = 0.0;
            h.mood = 0.0;
            h.faded = true;
        } else {
            seen.push((h.hit.on, kind));
        }
    }
}

/// A question of his you dodged (5.4): he notices.
pub fn dodged(w: &World, him: Them) -> Sized {
    let ear = 0.6 + 0.8 * trust_of(w, him);
    Sized {
        hit: Hit { on: him, item: Item::HisWord, how: How::Doubted },
        trust: -0.03 * ear,
        mood: -0.01,
        faded: false,
    }
}

/// An apology (5.1): it repairs trust for a broken promise by how real it
/// is, once. Saying sorry for nothing does nothing; saying it again for the
/// same thing counts for little.
pub fn apology(w: &World, him: Them, for_promise: Option<u32>, open: &Open) -> Option<Sized> {
    let k = for_promise?;
    let p = w.folk.promises.get(k as usize)?;
    if p.kept != Some(false) || p.to != him.promise_to()? {
        return None;
    }
    let hit = Hit { on: him, item: Item::Promise(k), how: How::Reassured };
    let times = open.times(&hit);
    let ego = z(w, him, Trait::Ego).max(0.0);
    let fade = AGAIN_IN_TALK.powi(times as i32);
    Some(Sized { hit, trust: (0.05 - 0.02 * ego) * fade, mood: 0.02 * fade, faded: times > 0 })
}

/// Holds each person's line inside today's caps: between the worst and the
/// best a chip could do to him on this talk's topic now. A man the talk
/// isn't about is held to the team's subject.
pub fn cap(w: &World, s: &Scene, room: &Room, hits: &mut [Sized]) {
    for p in &room.present {
        let Them::Player(id) = p.who else { continue };
        let Some(i) = folk::person(w, id) else { continue };
        let topic = if s.kind == SceneKind::OneOnOne && s.a as u32 == id {
            Topic::of(s.b)
        } else {
            Topic::Subject(Subject::Team)
        };
        let ((lo_t, hi_t), (lo_m, hi_m)) = chip_range(w, i, topic, s.id as u64);
        // A stake is sized by his belief and held by its own sizes, not
        // the chips' (agreed 2026-11-15): one strong line weighs more than
        // any chip.
        let mine: Vec<usize> =
            (0..hits.len()).filter(|&k| hits[k].hit.on == p.who && hits[k].hit.how != How::Staked).collect();
        let (st, sm) = mine.iter().fold((0.0, 0.0), |a, &k| (a.0 + hits[k].trust, a.1 + hits[k].mood));
        let kt = scale(st, lo_t, hi_t);
        let km = scale(sm, lo_m, hi_m);
        for k in mine {
            hits[k].trust *= kt;
            hits[k].mood *= km;
        }
    }
}

/// The share of a sum that fits inside [lo, hi].
fn scale(sum: f32, lo: f32, hi: f32) -> f32 {
    if sum > hi && sum > 0.0 {
        (hi.max(0.0) / sum).clamp(0.0, 1.0)
    } else if sum < lo && sum < 0.0 {
        (lo.min(0.0) / sum).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

/// The worst and best (trust, mood) any chip could do to him now on a
/// topic: every move in every tone, as `talks::landing` sizes it.
pub fn chip_range(w: &World, i: usize, topic: Topic, seed: u64) -> ((f32, f32), (f32, f32)) {
    let sheet = claims::player(w, i);
    let subject = topic.subject();
    let mut t = (0.0f32, 0.0f32);
    let mut m = (0.0f32, 0.0f32);
    for mv in Move::ALL {
        let rest = tp::rest(w, i, &sheet, topic, mv, true);
        for tone in [Tone::Calm, Tone::Warm, Tone::Firm, Tone::Fiery] {
            let (dt, dm) = talks::landing(w, i, mv, tone, subject, rest, seed);
            t = (t.0.min(dt), t.1.max(dt));
            m = (m.0.min(dm), m.1.max(dm));
        }
    }
    (t, m)
}

/// What a line must move him by to read as landed, and as not landing:
/// a player's chips move him far more than anyone else's can (a person in
/// your building is held between -0.05 and +0.02 a line, a parent between
/// -0.06 and +0.04), so the bars are his own: half the most a line can do
/// for him, and two-fifths of the worst. Before this one bar (0.06) read
/// everyone, so no line to your boss could ever land (Cain 2026-11-16).
/// Proposed, not confirmed.
pub fn landed_bars(him: Them) -> (f32, f32) {
    match him {
        Them::Player(_) => (tp::LANDED, -0.03),
        Them::Kin(..) => (0.5 * super::super::meetings::PARENT_UP, 0.4 * super::super::meetings::PARENT_DOWN),
        _ => (0.5 * super::super::office::CAP_UP, 0.4 * super::super::office::CAP_DOWN),
    }
}

/// What your read catches from how the line went over, when nothing else
/// set a tell: the way a chip's answer reads him (`talks::read_tell`).
pub fn tell(w: &World, him: Them, hits: &[Sized]) -> String {
    let Them::Player(id) = him else { return String::new() };
    let Some(i) = folk::person(w, id) else { return String::new() };
    let net: f32 = hits.iter().filter(|h| h.hit.on == him).map(|h| h.trust + h.mood).sum();
    let sign = if net > tp::LANDED {
        1
    } else if net < -0.03 {
        -1
    } else {
        0
    };
    talks::read_tell(w, i, sign).unwrap_or_default()
}
