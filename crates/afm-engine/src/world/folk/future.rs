//! Your word on his future, in a one-on-one about the portal, his place
//! behind someone or a lost job (playtest bug 15, Cain 2026-11-13):
//!
//! - **A promise backed by a real strength** (Cain's 2b and 2c: "the
//!   previous coach didn't use you, but this attribute is gonna make me use
//!   you more"): a start or a real role next season, with the reason named.
//!   He knows his own testing numbers and roughly where he ranks at his
//!   spot: a strength in the top third there makes him believe you more, one
//!   that isn't makes it sound like a line (`stay_read::promise_belief`).
//! - **Three layers of "go"** (Cain's 3b: "from 'You have no future' to 'If
//!   you stay, you'll be my last choice', leaving a small window open with
//!   the later one"): no future here; you'd play more elsewhere and I'll help
//!   you find a place; my last choice if you stay. He decides there and
//!   then: most go at the next window, a few stay to fight, more with
//!   competitiveness and loyalty, fewer with a big ego, more when he has
//!   nowhere real to go. He is not cut.
//!
//! Sizes proposed, not confirmed, except the belief steps Cain agreed (+0.1
//! for a real strength, -0.15 for one that isn't).

use super::claims::Kind as K;
use super::scene::{Pick, PickKind};
use super::talks::{self, PromiseKind, PromiseTo};
use crate::attributes::Attr;
use crate::player::Trait;
use crate::positions::Position;
use crate::world::{Person, Stage, World};

/// The four words, each its own pick code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Word {
    /// A role next season, with a strength of his named as the reason.
    Backed,
    /// "You don't have a future here."
    NoFuture,
    /// "You'd play more somewhere else; I'll help you find the place."
    Elsewhere,
    /// "If you stay, you'll be my last choice."
    LastChoice,
}

const FIRST_CODE: u32 = 950;

impl Word {
    pub const ALL: [Word; 4] = [Word::Backed, Word::NoFuture, Word::Elsewhere, Word::LastChoice];
    pub fn code(self) -> u32 {
        FIRST_CODE + Word::ALL.iter().position(|x| *x == self).unwrap_or(0) as u32
    }
    pub fn of(code: u32) -> Option<Word> {
        code.checked_sub(FIRST_CODE).and_then(|k| Word::ALL.get(k as usize)).copied()
    }
    /// How often he stays anyway, before his traits (Cain's 3b: the last
    /// choice leaves a small window open).
    fn stays(self) -> f64 {
        match self {
            Word::NoFuture => 0.10,
            Word::Elsewhere => 0.20,
            Word::LastChoice => 0.33,
            Word::Backed => 1.0,
        }
    }
    fn event(self) -> &'static str {
        match self {
            Word::NoFuture => NO_FUTURE,
            Word::Elsewhere => ELSEWHERE,
            Word::LastChoice => LAST_CHOICE,
            // A promise leaves no lean event of its own: the day's read reads it.
            Word::Backed => "",
        }
    }
}

/// The lean events your word leaves.
pub const NO_FUTURE: &str = "his head coach told him he has no future here";
pub const ELSEWHERE: &str = "his head coach told him he'd play more somewhere else";
pub const LAST_CHOICE: &str = "his head coach told him he'd be his last choice";
pub const STAYS: &str = "stays to fight for his place";

/// Where he lands his lean once he has decided to go: he enters at the next
/// window (its draw is u < lean x era).
const GOING_LEAN: f64 = 0.98;
/// And once he has decided to stay.
const STAYING_LEAN: f64 = 0.01;

/// Whether your word decided it this year: Some(true) he goes at the next
/// window, Some(false) he stays. The year's lean starts again at fall camp.
pub fn decided(p: &Person) -> Option<bool> {
    let told = p.cal.lean_events.iter().rposition(|e| matches!(e.1, NO_FUTURE | ELSEWHERE | LAST_CHOICE))?;
    Some(!p.cal.lean_events[told..].iter().any(|e| e.1 == STAYS))
}

/// Whether the four words are on the table: a college player of yours, as
/// head coach, on the portal, a lost job or sitting behind someone.
fn offered(w: &World, i: usize, k: K) -> bool {
    let p = &w.people[i];
    matches!(k, K::Portal | K::Buried | K::LostJob)
        && p.stage == Stage::College
        && p.college.is_some()
        && crate::world::front_office::program(w) == p.college
        && decided(p).is_none()
        // A promise lands once: your word on his role is already given.
        && !w.folk.promises.iter().any(|pr| {
            pr.to == PromiseTo::Player(p.player.id)
                && pr.kept.is_none()
                && matches!(pr.kind, PromiseKind::Start | PromiseKind::Snaps | PromiseKind::Place(_))
        })
}

/// His best physical number against the other men at his spot on your
/// roster, as he knows it from testing: (the word for it, whether it is in
/// the top third there). Alone at his spot, it counts as real.
pub fn strength(w: &World, i: usize) -> (&'static str, bool) {
    let p = &w.people[i];
    let pos = p.player.position;
    let big = matches!(pos, Position::OT | Position::IOL | Position::DT | Position::EDGE);
    let mut list: Vec<(&'static str, Box<dyn Fn(&Person) -> f64>)> = vec![
        ("speed", Box::new(|q: &Person| q.player.attributes.get(Attr::Speed))),
        ("burst", Box::new(|q: &Person| q.player.attributes.get(Attr::Acceleration))),
        ("feet", Box::new(|q: &Person| q.player.attributes.get(Attr::Agility))),
        ("strength", Box::new(|q: &Person| q.player.attributes.get(Attr::Strength))),
        ("leaping ability", Box::new(|q: &Person| q.player.attributes.get(Attr::Jumping))),
    ];
    if big {
        list.push(("size", Box::new(|q: &Person| q.player.body.weight)));
    } else {
        list.push(("length", Box::new(|q: &Person| q.player.body.height)));
    }
    if pos == Position::QB {
        list.push(("arm", Box::new(|q: &Person| q.player.attributes.get(Attr::ArmStrength))));
    }
    let others: Vec<&Person> = w
        .people
        .iter()
        .filter(|q| {
            q.stage == Stage::College
                && q.college == p.college
                && q.player.position == pos
                && q.player.id != p.player.id
        })
        .collect();
    let mut best = ("speed", -1.0);
    for (word, f) in &list {
        let me = f(p);
        let share = if others.is_empty() {
            1.0
        } else {
            others.iter().filter(|q| f(q) <= me).count() as f64 / others.len() as f64
        };
        if share > best.1 {
            best = (word, share);
        }
    }
    (best.0, best.1 >= 2.0 / 3.0)
}

/// The promise the backed word logs: a start when he can see one coming,
/// else a real role.
fn backed_kind(w: &World, i: usize) -> PromiseKind {
    let year = crate::world::staff::cycle_year(w);
    let ctx = crate::world::stay_read::Ctx::new(w, year);
    match crate::world::stay_read::next_fall(w, &ctx, i) {
        crate::world::stay_read::Next::Behind => PromiseKind::Snaps,
        _ => PromiseKind::Start,
    }
}

/// The picks your word adds to a point (none already decided).
pub fn picks(w: &World, i: usize, k: K) -> Vec<Pick> {
    if !offered(w, i, k) {
        return Vec::new();
    }
    let (word, real) = strength(w, i);
    let role = match backed_kind(w, i) {
        PromiseKind::Start => "a start next season",
        _ => "a real role next season",
    };
    let backed_hint = if real {
        format!("Logged: {role}. His {word} really is among the best at his spot, so he believes it more.")
    } else {
        format!("Logged: {role}. His {word} isn't really above the others at his spot: it may sound like a line.")
    };
    let mk = |w: Word, label: String, mv: &str, tone: &str, hint: String| {
        Pick { label, mv: mv.into(), tone: tone.into(), hint, code: w.code(), ..Pick::default() }
            .on(k.key())
            .of(PickKind::Final)
    };
    vec![
        mk(
            Word::Backed,
            format!("\"The last staff didn't use you right. Your {word} is why you'll play for me.\""),
            "Promise",
            "warm",
            backed_hint,
        ),
        mk(
            Word::LastChoice,
            "\"If you stay, you'll be my last choice. Earn your way back.\"".into(),
            "Tell him straight",
            "firm",
            "Most go at the next window; a few stay and can win their way back in practice.".into(),
        ),
        mk(
            Word::Elsewhere,
            "\"You'd play more somewhere else, and I'll help you find the place.\"".into(),
            "Tell him straight",
            "warm",
            "Most go at the next window. Honest and helpful: it costs you no trust.".into(),
        ),
        mk(
            Word::NoFuture,
            "\"I'll be honest with you. You don't have a future here.\"".into(),
            "Tell him straight",
            "firm",
            "He goes at the next window, nearly always; one who stays has little trust left in you. His friends hear it."
                .into(),
        ),
    ]
}

/// Whether he has anywhere real to go: well below the weakest quarter of
/// starters at his spot one level down, nobody will want him.
fn nowhere(w: &World, i: usize) -> bool {
    let p = &w.people[i];
    let Some(c) = p.college else { return true };
    let div = w.programs[c].division;
    let below = match div {
        crate::world::data::Division::FBS => crate::world::data::Division::FCS,
        crate::world::data::Division::FCS => crate::world::data::Division::D2,
        d => d,
    };
    let mut v: Vec<f64> = w
        .people
        .iter()
        .filter(|q| {
            q.stage == Stage::College
                && q.player.position == p.player.position
                && q.last_role == Some(crate::player::PlayingTime::Starter)
                && q.college.is_some_and(|x| w.programs[x].division == below)
        })
        .map(|q| q.player.overall())
        .collect();
    if v.is_empty() {
        return false;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    p.player.overall() < v[v.len() / 4] - 3.0
}

/// Sets his lean to `to`, the day's read taken out first so it isn't
/// undone later; the event carries the step so the lean's dated product
/// still holds.
fn set_lean(p: &mut Person, date: crate::calendar::Date, why: &'static str, to: f64) {
    crate::world::stay_read::apply(p, date, &[]);
    if p.cal.lean <= 0.0 || p.cal.lean_events.is_empty() {
        p.cal.lean_events.clear();
        p.cal.lean = to;
        p.cal.lean_events.push((date, why, to));
    } else {
        let f = to / p.cal.lean;
        p.cal.lean = to;
        p.cal.lean_events.push((date, why, f));
    }
}

/// What your word does: (his answer, the outcome lines).
pub fn answer(w: &mut World, i: usize, word: Word) -> (String, Vec<String>) {
    let today = w.today;
    let id = w.people[i].player.id;
    let name = w.people[i].name.clone();
    let you = w.programs[w.people[i].college.unwrap_or(0)].head_coach.map(|c| c as u32);
    let mut out = Vec::new();
    if word == Word::Backed {
        let (strong, real) = strength(w, i);
        let kind = backed_kind(w, i);
        talks::promise(w, PromiseTo::Player(id), kind, 0);
        if let Some(pr) = w
            .folk
            .promises
            .iter_mut()
            .rev()
            .find(|x| x.to == PromiseTo::Player(id) && x.kind == kind && x.kept.is_none())
        {
            pr.backed = if real { 1 } else { -1 };
        }
        let p = &mut w.people[i];
        if Some(p.trust_in) == you {
            p.trust = (p.trust + if real { 0.03 } else { -0.03 }).clamp(0.0, 1.0);
        }
        out.push(format!("You promised {name} {} and named his {strong} as the reason.", kind.words()));
        let line = if real {
            format!("You noticed my {strong}? Alright, Coach. I'm listening.")
        } else {
            format!("Coach, I've seen the numbers. My {strong} isn't what gets me on the field here.")
        };
        return (line, out);
    }
    let p = &w.people[i];
    let pers = p.player.personality;
    let t = pers.traits;
    let comp = (pers.competitiveness - 10.0) / 10.0;
    let lift = (1.0 + 0.4 * comp + 0.25 * t.z(Trait::Loyalty) - 0.2 * t.z(Trait::Ego)).clamp(0.5, 1.6);
    let room = if nowhere(w, i) { 1.5 } else { 1.0 };
    let chance = (word.stays() * lift * room).clamp(0.02, 0.6);
    let stays = super::roll(w, 0xF07E_0000 ^ word.code() as u64, id as u64 ^ (today.0 as u64) << 32) < chance;
    let mine = Some(w.people[i].trust_in) == you;
    {
        let p = &mut w.people[i];
        set_lean(p, today, word.event(), GOING_LEAN);
        if stays {
            let f = STAYING_LEAN / p.cal.lean;
            p.cal.lean = STAYING_LEAN;
            p.cal.lean_events.push((today, STAYS, f));
        }
        if mine {
            p.trust = match (word, stays) {
                (Word::NoFuture, true) => 0.2,
                (Word::NoFuture, false) => (p.trust - 0.15).max(0.0),
                (Word::LastChoice, true) => (p.trust - 0.1).max(0.0),
                (Word::LastChoice, false) => (p.trust - 0.05).max(0.0),
                _ => p.trust,
            };
        }
    }
    // His friends hear it (no future here; the last choice a little).
    let friends_cost = match word {
        Word::NoFuture => 0.03,
        Word::LastChoice => 0.015,
        _ => 0.0,
    };
    let college = w.people[i].college;
    if friends_cost > 0.0 {
        for f in super::ties::friends(w, id) {
            if let Some(j) = super::person(w, f) {
                let q = &mut w.people[j];
                if q.college == college && Some(q.trust_in) == you {
                    q.trust = (q.trust - friends_cost as f32).max(0.0);
                }
            }
        }
        // A captain told he has no future: the room hears it.
        let c = w.people[i].college.unwrap_or(0);
        if word == Word::NoFuture && w.programs[c].captains.contains(&i) {
            let roster: Vec<usize> =
                (0..w.people.len()).filter(|&j| j != i && w.people[j].college == Some(c)).collect();
            for j in roster {
                let q = &mut w.people[j];
                if q.stage == Stage::College && Some(q.trust_in) == you {
                    q.trust = (q.trust - 0.01).max(0.0);
                }
            }
            out.push("He's a captain: the room heard what you told him.".into());
        }
    }
    let line = match (word, stays) {
        (Word::NoFuture, false) => "Then I'm gone. I'll put my name in.",
        (Word::NoFuture, true) => "You're wrong about me, Coach. I'm not going anywhere. Watch me.",
        (Word::Elsewhere, false) => "I appreciate you being straight with me. I'll look around.",
        (Word::Elsewhere, true) => "I hear you, Coach. I still want to try it here.",
        (Word::LastChoice, false) => "Last choice? Then I'm out.",
        (Word::LastChoice, true) => "Then I'll make you change your mind.",
        _ => "",
    };
    out.push(if stays {
        format!("{name} stays to fight for his place.")
    } else {
        format!("{name} will enter the transfer portal at the next window.")
    });
    (line.into(), out)
}
