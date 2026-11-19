//! A talk that ends a meeting (thread "Let talking end meetings cleanly",
//! 2026-11-15; Cain: "I don't got a way to cleanly end the convo. All I
//! got is 'skip it'"). Before this, a goodbye ended the talk but left a
//! built staff or boss meeting, or one of the older meetings, open on its
//! own picks, so the only way out was a pick or "Skip it" (and a skipped
//! boss meeting costs his trust).
//!
//! Now, as in a real office, saying goodbye ends the meeting:
//!
//! - nothing is charged for leaving: you were there and you talked;
//! - what you didn't argue stands as it was set (the boss's goals, the
//!   plan, the staff's own routine: the recruiting coordinator still calls
//!   the commits, the position coaches still talk to the flight risks, the
//!   trainers run their plan; `settle`);
//! - what you said already counted, line by line, as it always has;
//! - a real decision the talk didn't settle (keep or let go a holdover, a
//!   firing, a hire, a second opinion, a signee's release, a punishment,
//!   the rituals, a star's ask, a coordinator's case, a booster's pass)
//!   leaves with you: End always closes the talk (talk replies item 12,
//!   Cain 2026-11-19: "When i confirm 'end' on a talk it should close the
//!   convo entirely"). The decision moves to its own screen (a hire to the
//!   staff screen's candidates, a holdover or a firing to your staff, a
//!   punishment to your team rules) with a line in your inbox, and a
//!   candidate's patience runs from that day (`hiring::left_waiting`).
//!   Until then the meeting stayed open on its picks with the talk over.
//!
//! Which meetings hold a decision is Claude's call, proposed, not confirmed.

use crate::world::World;
use crate::world::folk::scene::{self, Kind, Scene};

/// The meetings that hold a decision of yours: when the talk ends without
/// settling it, the meeting's own picks stay as the decision. Proposed,
/// not confirmed.
pub fn decides(s: &Scene) -> bool {
    matches!(
        s.kind,
        Kind::Holdover
            | Kind::Firing
            | Kind::Interview
            | Kind::Proposal
            | Kind::SecondOpinion
            | Kind::StarPower
            | Kind::SigneeRelease
            | Kind::Discipline
            | Kind::Ritual
            | Kind::StaffMeeting
    ) || (s.kind == Kind::Booster && crate::world::folk::backers::is_pass_test(s))
}

/// Whether a scene is one of the meetings that used to stay open on its
/// own picks after the talk (lane N5c's built meetings, the older meetings).
pub fn kept_open(kind: Kind) -> bool {
    (super::super::office::plays(kind) && !super::super::office::talk_only(kind))
        || super::super::meetings::keeps_open(kind)
}

/// The talk ended a meeting: one that holds a decision keeps its picks
/// with the talk over; any other closes, with no cost for leaving, and the
/// staff's routine runs as it would have. Returns false for a scene this
/// doesn't cover.
pub fn close(w: &mut World, id: u32) -> bool {
    let Some(s) = w.folk.scenes.iter().find(|s| s.id == id) else { return false };
    if s.done || !kept_open(s.kind) {
        return false;
    }
    if decides(s) && !s.picks.is_empty() {
        let kind = s.kind;
        let name = across_name(w, s).map(|n| crate::world::folk::plain(&n));
        let (what, whr) = still_yours(kind);
        if kind == Kind::Interview {
            let c = s.a as usize;
            crate::world::folk::hiring::left_waiting(w, c, crate::world::folk::staffing::role_from(s.b));
        }
        scene::leave(w, id, |w, s| {
            settle(w, s);
            s.outcome.insert(
                0,
                match &name {
                    Some(n) => format!("You ended the talk with {n}. {what} is still your call: {whr}."),
                    None => format!("You ended the talk. {what} is still your call: {whr}."),
                },
            );
        });
        super::store::close(w, id);
        let text = match &name {
            Some(n) => format!("Still your call after your talk with {n}: {}, {whr}.", lower(what)),
            None => format!("Still your call: {}, {whr}.", lower(what)),
        };
        let today = w.today;
        crate::world::folk::post_mine(w, today, crate::calendar::Loudness::Small, true, "people: decision", &text);
        return true;
    }
    scene::leave(w, id, |w, s| {
        let name = across_name(w, s);
        settle(w, s);
        s.outcome.insert(
            0,
            match name {
                Some(n) => format!("You wrapped it up with {n}. What you didn't argue stands as it was."),
                None => "You wrapped it up. What you didn't argue stands as it was.".into(),
            },
        );
    });
    super::store::close(w, id);
    true
}

/// The decision a meeting leaves open when you end the talk, and the
/// screen it moves to. Proposed, not confirmed.
fn still_yours(kind: Kind) -> (&'static str, &'static str) {
    match kind {
        Kind::Interview => ("Hiring him", "he's on your staff screen's candidates while he waits for your offer"),
        Kind::Holdover => ("Keeping him", "he stays on your staff until you let him go from the staff screen"),
        Kind::Firing => ("Letting him go", "he stays on your staff until you let him go from the staff screen"),
        Kind::Proposal => ("His proposal", "things stay as they were; take it up with him on the staff screen"),
        Kind::Discipline | Kind::Ritual => ("The punishment", "nothing is set; set it from your team rules"),
        Kind::StaffMeeting => ("The staff's plan", "it runs as the staff set it"),
        _ => ("The decision", "it stands as it was until you take it up again"),
    }
}

fn lower(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_lowercase().collect::<String>() + c.as_str())
}

/// The man across, by name, for the outcome line.
fn across_name(w: &World, s: &Scene) -> Option<String> {
    let room = super::store::room(w, s.id).or_else(|| super::super::office::room(w, s))?;
    let him = super::across(&room)?;
    Some(super::channels::name(w, him))
}

/// What still happens when you leave without settling it: the staff's own
/// routine, the committee's verdict. Never a cost for leaving.
fn settle(w: &mut World, s: &mut Scene) {
    match s.kind {
        Kind::Recruiting | Kind::FlightRisks | Kind::Trainer => crate::world::folk::takeover::settle(w, s),
        Kind::JobInterview => crate::world::folk::dialogue::settle(w, s),
        _ => {}
    }
}
