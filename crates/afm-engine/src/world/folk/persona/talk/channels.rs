//! Channels and who's in the room (sections 5.5 and 5.6): what each way of
//! talking allows, how long a line takes on it, who answers, and who can
//! be brought in. The NCAA's contact rules stay `calendar/contact.rs`'s
//! (a recruit's calls and messages, applied by the recruiting lane).
//!
//! Sizes are proposed, not confirmed, except where a built rule sets them:
//! a call is three exchanges and a message one (`calendar/contact.rs`, the
//! built calls and messages).

use super::super::Them;
use super::super::ear::{Act, ActKind, Reading, Show};
use super::super::knows::Topic;
use super::super::room::{Channel, Place, Present, Room, Seat};
use super::super::terms::Decision;
use super::super::wants::Boil;
use super::MAX_LINES;
use crate::world::World;
use crate::world::folk::dialogue::TYPED_CHARS_A_MINUTE;
use crate::world::folk::scene::Scene;
use crate::world::folk::{self as folk, Team};

/// What a channel allows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Allows {
    /// Film and numbers can be shown.
    pub film: bool,
    /// Someone can be brought in or asked to leave.
    pub bring_in: bool,
    /// The most lines before it ends.
    pub lines: u8,
    /// How long your words take, against a sit-down.
    pub pace: f32,
    /// You hear his answer through someone else, with their shade.
    pub secondhand: bool,
}

/// What a channel allows (5.5's table).
pub fn allows(c: Channel) -> Allows {
    let a = |film, bring_in, lines, pace, secondhand| Allows { film, bring_in, lines, pace, secondhand };
    match c {
        Channel::InPerson(_) => a(true, true, MAX_LINES, 1.0, false),
        // A call: three exchanges, as the built calls run.
        Channel::Phone => a(false, false, 3, 1.0, false),
        // A text: one exchange, short and quick.
        Channel::Text => a(false, false, 1, 0.4, false),
        Channel::Video => a(true, false, MAX_LINES / 2, 1.0, false),
        Channel::Through(_) => a(false, false, 3, 1.0, true),
        Channel::Letter => a(false, false, 1, 1.5, false),
        Channel::Group(_) => a(true, true, MAX_LINES / 2, 1.0, false),
    }
}

/// Minutes your words take on a channel (`dialogue::TYPED_CHARS_A_MINUTE`,
/// the built pace of a typed line).
pub fn minutes(c: Channel, words: &str) -> f32 {
    words.chars().count() as f32 / f32::from(TYPED_CHARS_A_MINUTE) * allows(c).pace
}

/// Who else hears it, 0 alone to 1 a room (`TellParts.room`).
pub fn crowd(room: &Room) -> f64 {
    let others = room.present.len().saturating_sub(1) as f64;
    match room.channel {
        Channel::Group(_) => 1.0,
        _ => (others / 3.0).min(1.0),
    }
}

/// Who answers this line, when it isn't the man across (5.6): with his
/// agent in the room the agent talks money and terms; his high school
/// coach answers the questions on his grades, his character and the
/// schools; through someone else, that person answers.
pub fn speaker(room: &Room, reading: &Reading) -> Option<(Them, Seat)> {
    if let Channel::Through(x) = room.channel {
        return Some((x, room.present.iter().find(|p| p.who == x).map_or(Seat::Family, |p| p.seat)));
    }
    let deal = reading.acts.iter().any(|a| matches!(a.kind(), ActKind::Offer | ActKind::Answer | ActKind::Deadline));
    let school = reading.acts.iter().any(|a| {
        matches!(a, Act::Question { topic: Topic::Grades | Topic::Character | Topic::Pitches, about: None, .. })
    });
    room.present
        .iter()
        .filter(|p| p.speaks)
        .find(|p| (p.seat == Seat::Agent && deal) || (p.seat == Seat::HsCoach && school))
        .map(|p| (p.who, p.seat))
}

/// The room for a one-on-one: him, across from you, in your office (or
/// wherever the scene says).
pub fn one_on_one(s: &Scene) -> Room {
    let place = place_of(&s.place);
    Room {
        channel: Channel::InPerson(place),
        present: vec![Present { who: Them::Player(s.a as u32), seat: Seat::Across, speaks: false }],
    }
}

/// A place from a scene's words.
fn place_of(words: &str) -> Place {
    let w = words.to_lowercase();
    if w.contains("locker") {
        Place::LockerRoom
    } else if w.contains("field") || w.contains("practice") {
        Place::Field
    } else if w.contains("sideline") {
        Place::Sideline
    } else if w.contains("home") || w.contains("living room") {
        Place::Home
    } else {
        Place::Office
    }
}

/// Whether someone can be brought into the room: someone on your staff or
/// your roster (a former player's story comes with the recruiting lane).
pub fn can_bring(w: &World, who: Them) -> bool {
    let Some(team) = Team::seat(w) else { return false };
    match who {
        Them::Player(id) => folk::person(w, id).is_some_and(|i| team.roster(w).contains(&i)),
        Them::Coach(c) => w.coaches.get(c).and_then(|c| c.job).is_some_and(|j| Team::from_employer(j.employer) == team),
        _ => false,
    }
}

/// The seat someone you bring takes.
pub fn brought_seat(who: Them) -> Seat {
    match who {
        Them::Coach(_) => Seat::YourStaff,
        _ => Seat::Listener,
    }
}

/// Whether a thing can be shown on a channel: only real things, and film
/// and numbers need a screen.
pub fn can_show(c: Channel, sh: Show) -> bool {
    match sh {
        Show::Facility => {
            matches!(c, Channel::InPerson(Place::Facility | Place::Office | Place::LockerRoom | Place::Field))
        }
        Show::Film | Show::Numbers | Show::Alumni => allows(c).film,
    }
}

/// A person's name for a line.
pub fn name(w: &World, who: Them) -> String {
    match who {
        Them::Player(id) => folk::person(w, id).map_or_else(|| "He".into(), |i| w.people[i].name.clone()),
        Them::Coach(c) => w.coaches.get(c).map_or_else(|| "The coach".into(), |c| c.name.clone()),
        _ => "He".into(),
    }
}

/// A decision in words.
pub fn decision_words(d: Decision) -> &'static str {
    match d {
        Decision::Benched => "he's benched",
        Decision::Starting => "he starts",
        Decision::Suspended => "he's suspended",
        Decision::Cut => "he's cut",
        Decision::Traded => "he's traded",
        Decision::OfferPulled => "the offer is pulled",
        Decision::CoachLetGo => "his coach is let go",
        Decision::PositionMoved(_) => "he's moving positions",
        Decision::Redshirt => "he'll redshirt",
    }
}

/// What you show, in words.
pub fn show_words(sh: Show) -> &'static str {
    match sh {
        Show::Film => "the film",
        Show::Numbers => "the numbers",
        Show::Facility => "the facility",
        Show::Alumni => "the players you've sent on from his spot",
    }
}

/// What boiling over looks like, as the game decided it (the voice says it).
pub fn boil_words(b: Boil) -> &'static str {
    match b {
        Boil::Quiet => "He shuts down and goes quiet.",
        Boil::Loud => "He raises his voice.",
        Boil::WalksOut => "He gets up and walks out.",
    }
}
