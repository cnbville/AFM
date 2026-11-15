//! The compliance office (section 6.13): your school's director of
//! compliance, drawn (the world keeps no such job), answers "what's
//! allowed?" in plain words from the era's rule book: today's recruiting
//! period, NIL, revenue sharing and the roster limits (`calendar`,
//! `world/ncaa.rs`). Your athletic director knows the same rules.
//!
//! Self-reporting a violation, a waiver and an appeal need stores the
//! world doesn't keep yet (your own violations known to you, waivers,
//! appeals): they are weighed as requests and say so (proposed, not
//! confirmed).

use super::super::Them;
use super::super::knows::{Known, Source, Topic};
use super::super::wants::Kind as WhoKind;
use crate::world::World;
use crate::world::folk::Team;

/// The compliance director at a college: a drawn person keyed by the
/// school. None at the other levels (the NFL's league office and a high
/// school's state association come with their own lanes).
pub fn director(_w: &World, team: Team) -> Option<Them> {
    matches!(team, Team::College(_)).then(|| Them::Drawn(WhoKind::Compliance, key(team)))
}

/// The drawn director's key.
pub fn key(team: Team) -> u64 {
    0xC0_4D11 ^ team.key() << 8
}

/// His name and title, drawn the way the screens draw a staff job
/// (`staffing::draw_person`).
pub fn name(w: &World, who: Them) -> Option<String> {
    let Them::Drawn(WhoKind::Compliance, k) = who else { return None };
    Some(crate::world::folk::staffing::draw_person(w, k, "Director of compliance", 0.0, None).name)
}

/// Whether he is the compliance director or your athletic director: the
/// people who know the rules.
fn knows_rules(w: &World, him: Them) -> bool {
    matches!(him, Them::Drawn(WhoKind::Compliance, _))
        || matches!(him, Them::Cast(r, _) if r.role == crate::world::folk::cast::Role::AthleticDirector
            && Some(r.team) == Team::seat(w))
}

/// The rules today, in words.
pub fn words(w: &World) -> Vec<String> {
    let Some(Team::College(p)) = Team::seat(w) else { return Vec::new() };
    let mut v = Vec::new();
    let period = crate::calendar::recruiting_period(&w.dates, w.today);
    v.push(format!("we're in a {} for recruiting", period.name()));
    let year = w.year;
    v.push(if crate::world::ncaa::nil_era(year) {
        "players can be paid for their name, image and likeness, through the collective and their own deals".into()
    } else {
        "any payment to a recruit or his family is a violation, and so is a booster arranging one".into()
    });
    if crate::world::ncaa::revenue_cap(w.dates.season).is_some() {
        v.push("the school shares revenue with players directly, up to the cap".into());
    }
    let (limit, signees) = crate::world::ncaa::roster_limits(&w.programs[p], w.dates.season);
    if limit > 0 {
        v.push(format!("our roster limit is {limit}, and we can add {signees} new signees"));
    }
    v
}

/// What he knows when you ask about the rules.
pub fn knows(w: &World, him: Them, topic: Topic) -> Vec<Known> {
    if topic != Topic::Rules || !knows_rules(w, him) {
        return Vec::new();
    }
    let v = words(w);
    if v.is_empty() {
        return Vec::new();
    }
    vec![Known {
        topic,
        about: None,
        from: Source::Job,
        eye: 18.0,
        point: None,
        says: Vec::new(),
        words: v.join("; "),
        sure: 0.95,
        day: None,
    }]
}
