//! A coach's read on a player you asked him to look at (Cain 2026-11-16:
//! "I want you to take a look at Strand, I want to decide if we should cut
//! him"). The ear hears it as `Request::Evaluate`; a coach on your staff
//! takes it on and answers from his own read, in the same line.
//!
//! His read is the scouting file's: his own looks when he has him in his
//! room (`readers::own_roster`: the position coach sees him every week, as
//! do you), else the staff's file, and he says which. He places the man
//! among your men at his spot by the staff's reads, says where he could get
//! to (the file's ceiling tier and how sure), and makes his call: the
//! staff's roster call (`folk::roster_call`, staff urgency 2026-11-21),
//! against the level's marks, where he'll be when he's out of time, the
//! clock and the portal at his spot, with the coach's own bar. Words only:
//! no rating, potential or depth chart reaches the voice.

use super::Them;
use super::office::asks::on_your_staff;
use crate::positions::Position;
use crate::world::folk::roster_call;
use crate::world::scouting::file::{self, Link, Speak};
use crate::world::scouting::{own, readers};
use crate::world::{Stage, World};

/// A coach's read on one of your players, or another man he was asked
/// about.
#[derive(Clone, Debug)]
pub struct Read {
    /// His own looks (true) or the staff's file.
    pub own: bool,
    /// Where he places him among your men at his spot (1 the best) and how
    /// many there are; None for a man not on your roster.
    pub place: Option<(usize, usize)>,
    pub spot: Position,
    /// The ceiling in the staff's words, and how sure.
    pub tier: &'static str,
    pub sure: &'static str,
    /// His call on the man (staff urgency), for a man on your roster.
    pub call: Option<roster_call::Call>,
}

fn speak(stage: Stage) -> Speak {
    match stage {
        Stage::HighSchool | Stage::PrepSchool => Speak::Recruit,
        Stage::Pro => Speak::Pro,
        _ => Speak::Prospect,
    }
}

/// His read on the man, None when he isn't a coach on your staff or nobody
/// on your staff has read the man yet.
pub fn read(w: &World, him: Them, id: u32) -> Option<Read> {
    let Them::Coach(c) = him else { return None };
    if !on_your_staff(w, c) {
        return None;
    }
    let staff = readers::seat_staff(w)?;
    let i = crate::world::scouting::place(w, id)?;
    let (rds, _, f) = readers::read(w, staff, id)?;
    let mine = rds.iter().position(|r| r.link == Link::Coach(c)).and_then(|r| own::reader_overall(w, i, &f, r));
    let now = mine.map_or(f.now.0, |m| m.0);
    let ceiling = f.ceiling.0 + (now - f.now.0);
    let pos = w.people[i].player.position;
    let mut read = Read {
        own: mine.is_some(),
        place: None,
        spot: pos,
        tier: file::tier_words(speak(w.people[i].stage), ceiling),
        sure: file::sure_words(f.ceiling.1),
        call: None,
    };
    // Everyone on your roster at his spot, the hurt men too.
    let roster = crate::world::folk::Team::seat(w).map(|t| t.roster(w)).unwrap_or_default();
    if !roster.contains(&i) {
        return Some(read);
    }
    let men: Vec<u32> =
        roster.iter().filter(|&&k| w.people[k].player.position == pos).map(|&k| w.people[k].player.id).collect();
    // The others at his spot as the staff reads them, best first.
    let others: Vec<f64> =
        men.iter().filter(|&&x| x != id).filter_map(|&x| readers::read(w, staff, x).map(|r| r.2.now.0)).collect();
    let place = 1 + others.iter().filter(|&&x| x > now).count();
    read.place = Some((place, others.len() + 1));
    read.call = roster_call::seat_call(w, i, c);
    Some(read)
}

fn nth(n: usize) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// His read in words, for the voice to say (what the game released).
pub fn words(w: &World, him: Them, id: u32) -> Vec<String> {
    let Some(i) = crate::world::folk::person(w, id) else { return Vec::new() };
    let name = w.people[i].name.clone();
    let Some(r) = read(w, him, id) else {
        return match him {
            Them::Coach(c) if on_your_staff(w, c) => {
                vec![format!("He hasn't seen enough of {name} to give you a real read yet.")]
            }
            _ => Vec::new(),
        };
    };
    let spot = crate::world::folk::claims::words::spot(r.spot);
    let mut v = vec![if r.own {
        format!("He has had {name} in his room and gives you his own read.")
    } else {
        format!("{name} isn't in his room, so he goes by the staff's file on him.")
    }];
    if let Some((place, of)) = r.place {
        v.push(format!("He has him {} of your {of} at {spot}.", nth(place)));
    }
    let at = match w.people[i].stage {
        Stage::Pro => "As high as he could get",
        Stage::HighSchool | Stage::PrepSchool => "What he could become",
        _ => "His ceiling as a pro",
    };
    v.push(format!("{at}: {} ({} of that).", r.tier, r.sure));
    v.push(match &r.call {
        Some(call) => roster_call::words(w, call),
        None if r.place.is_some() => "He has no call on him yet: he's out of time here either way.".into(),
        None => "He isn't one of ours, so that's his read and no more.".into(),
    });
    v
}
