//! "What he's after" (talk replies item 14, agreed by Cain 2026-11-19:
//! "I need hints what the other person wants"): a read of his wants for
//! the talk's left panel, for every kind of person. It comes from your
//! read of him (your relationships rating and how long you've known him,
//! `talks::sharpness`) and from what he has told you (the lines of this
//! talk, the wants he raised himself, a question you asked him about what
//! he wants, your past talks with him). Vague at first, sharper as he
//! opens up; a weak read can be wrong (it swaps his first two); never a
//! number. Sizes are proposed, not confirmed.

use super::Them;
use super::knows::Topic;
use super::talk::store;
use super::wants::{Want, WantKind};
use crate::world::World;
use crate::world::folk::talks;

/// Under this sharpness the read is vague words; from it, his wants by
/// name; from `SHARP`, his own specifics.
pub const PLAIN: f64 = 0.35;
pub const SHARP: f64 = 0.65;
/// Below this read of yours, the read can be wrong.
pub const WRONG_BELOW: f64 = 0.55;

/// The read, for the panel: its lines and how sure it is.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct After {
    /// "Your read", "A fair read", "A good read".
    pub sure: &'static str,
    pub lines: Vec<String>,
}

/// A want as a vague read puts it.
pub fn vague(k: WantKind) -> &'static str {
    match k {
        WantKind::Kids | WantKind::Players => "something for his players",
        WantKind::Career | WantKind::Scheme => "a step up",
        WantKind::Money | WantKind::Fee | WantKind::Security => "the deal itself",
        WantKind::Role | WantKind::Development => "a bigger part",
        WantKind::Name | WantKind::Winning => "a bigger name",
        WantKind::Place | WantKind::Family | WantKind::Friends => "the people close to him",
        WantKind::Academics => "school",
        WantKind::Club | WantKind::Goals | WantKind::Booster(_) => "results",
        WantKind::Draft => "the next level",
        WantKind::Loyalty => "to belong somewhere",
    }
}

/// A want as he has put it to you: what would meet it, in plain words.
pub fn specific(k: WantKind) -> String {
    match k {
        WantKind::Kids => "clinic and camp invites for his kids".into(),
        WantKind::Players => "his best players recruited as part of the hire".into(),
        WantKind::Career => "a title and a clear next step".into(),
        WantKind::Scheme => "his say in the scheme, the play-calling".into(),
        WantKind::Money => "more money".into(),
        WantKind::Fee => "his fee".into(),
        WantKind::Security => "years and guarantees".into(),
        WantKind::Role => "a real role, a start".into(),
        WantKind::Development => "a plan that gets him to the next level".into(),
        WantKind::Place => "time at home, being close to it".into(),
        WantKind::Family => "what's best for his family".into(),
        WantKind::Friends => "his friends looked after".into(),
        WantKind::Academics => "his classes and his degree".into(),
        WantKind::Name => "a bigger stage".into(),
        WantKind::Winning => "a team that wins now".into(),
        WantKind::Club => "help for his club".into(),
        WantKind::Goals => "what you'll deliver on his goals".into(),
        WantKind::Booster(x) => x.short().into(),
        WantKind::Draft => "his draft stock".into(),
        WantKind::Loyalty => "your loyalty to him".into(),
    }
}

/// How far he has opened up to you: the lines of this talk, the wants he
/// raised, a question about what he wants, your past talks with him.
fn opened(w: &World, scene: u32, him: Them) -> (f64, Vec<WantKind>, bool) {
    let o = store::open(w, scene);
    let lines = o.map_or(0, |o| o.lines) as f64;
    let raised = o.map(|o| o.raised.clone()).unwrap_or_default();
    let asked = o.is_some_and(|o| o.questions.iter().any(|q| q.0 == Topic::Wants && q.1.is_none_or(|x| x == him)));
    let past = w.folk.live.persona.talks.iter().filter(|t| t.with == him).count().min(4) as f64;
    let open = 0.08 * lines + 0.2 * raised.len() as f64 + if asked { 0.3 } else { 0.0 } + 0.08 * past;
    (open.min(1.0), raised, asked)
}

/// A fixed draw for this read of him this season (0-1).
fn draw(w: &World, him: Them) -> f64 {
    let key =
        format!("{him:?}").bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
    super::super::roll(w, 0xAF7E ^ w.dates.season as u64, key)
}

/// What he's after, as your read has it today.
pub fn read(w: &World, scene: u32, him: Them, wants: &[Want]) -> After {
    let you = talks::sharpness(w).clamp(0.0, 1.0);
    let (open, raised, asked) = opened(w, scene, him);
    let sharp = (0.5 * you + 0.6 * open).min(1.0);
    let mut top: Vec<Want> = wants.iter().copied().filter(|x| !x.met && x.weight > 0.0).collect();
    // A weak read can have it backwards: his second want read as his
    // first. What he has raised himself is never misread.
    if you < WRONG_BELOW
        && top.len() >= 2
        && !raised.contains(&top[0].kind)
        && !asked
        && draw(w, him) < (WRONG_BELOW - you) * 0.6
    {
        top.swap(0, 1);
    }
    let mut lines: Vec<String> = Vec::new();
    let mut push = |s: String| {
        if !s.is_empty() && !lines.contains(&s) {
            lines.push(s);
        }
    };
    // What he has raised himself comes first, as he put it.
    for k in &raised {
        push(specific(*k));
    }
    let n = if sharp >= SHARP { 3 } else { 2 };
    for x in top.iter().take(n) {
        push(if sharp < PLAIN {
            vague(x.kind).into()
        } else if sharp >= SHARP {
            specific(x.kind)
        } else {
            super::wanting::want_words(x.kind).into()
        });
    }
    lines.truncate(4);
    let given: Vec<&str> = wants.iter().filter(|x| x.met).take(2).map(|x| vague(x.kind)).collect();
    if sharp >= PLAIN && !given.is_empty() {
        lines.push(format!("You've given him some of it: {}", given.join(", ")));
    }
    let sure = if sharp >= SHARP {
        "A good read"
    } else if sharp >= PLAIN {
        "A fair read"
    } else {
        "An early read"
    };
    After { sure, lines }
}
