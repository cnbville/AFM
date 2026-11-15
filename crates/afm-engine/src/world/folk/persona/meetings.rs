//! Live talks v3: the older meetings on the conversation (thread "Move old
//! meetings off chips", 2026-11-15; `docs/design/live-talks-persona.md`,
//! sections 6.1, 6.2, 6.4, 6.7 to 6.9, 6.11, 6.16 and 16.1: "Parent calls
//! and discipline become conversations", "Interviews ... become
//! conversations"). Before this, a typed line in these meetings was read
//! onto one of the meeting's chips (lane 3's `scene::say`). Now each one
//! plays on lane N4's conversation with the real person across, in his own
//! facet: what he knows and tells, what he wants, deals and requests (lanes
//! N1 and N3, with lane N5c's office parts for the people in your building
//! and other coaches), the hits sized inside what a chip could do, his heat
//! and patience.
//!
//! What this module adds, and nothing else:
//!
//! - which meetings play this way (`KINDS`) and who sits across in each
//!   (`room`): a player, his parent on the phone, a signee with his family
//!   on the line, a coach, your boss, a search committee, an agent, a
//!   trainer who calls, or a room of them (the captains, the parents, the
//!   feeder coaches: `is_group`);
//! - who else a line may name (`named`): the players the meeting is about
//!   and the men on the sheet in front of you;
//! - the caps for the people no other lane holds (`cap`): a parent, an
//!   agent, a trainer, another school's committee;
//! - where a parent's trust lives (`nudge`): his own, kept in `folk.mine`
//!   as lane N1 reads it, and his son's, as a parent's call moves it;
//! - a job interview's talk counts in its score (`after`, `TALK_SCORE`).
//!
//! The built meetings keep their own picks to finish: the talk ending leaves
//! the meeting open (`keeps_open`), the way lane N5c's meetings do. With no
//! key nothing here runs, and the chips and the line bank play as before.
//! Every size here is Claude's, proposed, not confirmed.

use super::Them;
use super::ear::{Reading, Sized, Turn};
use super::room::{Channel, Group, Place, Present, Room, Seat};
use super::wants::Kind as WhoKind;
use crate::world::World;
use crate::world::background::Decider;
use crate::world::folk::cast::{self, Role as CastRole};
use crate::world::folk::scene::{Kind, Scene, Who};
use crate::world::folk::{self as folk, Team, facts, takeover};

/// The older meetings that play on the conversation with a key. Trip,
/// JobCall and Family never took a typed line (nobody stored is across:
/// a trip you go on or skip, a game director or search firm on the line,
/// your own family), and HireReaction is the papers (`scene::NO_BOX`).
pub const KINDS: [Kind; 18] = [
    Kind::Plan,
    Kind::Tour,
    Kind::Recruiting,
    Kind::FlightRisks,
    Kind::Leaders,
    Kind::Parents,
    Kind::Feeder,
    Kind::OldCoach,
    Kind::SigneeRelease,
    Kind::ParentCall,
    Kind::Discipline,
    Kind::ReachOut,
    Kind::StarPower,
    Kind::Ritual,
    Kind::Proposal,
    Kind::JobInterview,
    Kind::Backchannel,
    Kind::CombineTalk,
];

/// Whether a meeting is one of these.
pub fn plays(k: Kind) -> bool {
    KINDS.contains(&k)
}

/// Whether the talk ending leaves the meeting open for its own picks (every
/// one but the interview room, which files itself when its clock runs out:
/// lane N5e's `draft::room`).
pub fn keeps_open(k: Kind) -> bool {
    plays(k) && k != Kind::CombineTalk
}

/// A room of people: the captains, the parents, the feeder coaches.
pub fn is_group(s: &Scene) -> bool {
    matches!(s.kind, Kind::Leaders | Kind::Parents | Kind::Feeder)
}

/// The whole room in words, for the ear's card (its one item for a line
/// meant for everyone).
pub fn whole_words(s: &Scene) -> &'static str {
    match s.kind {
        Kind::Leaders => "the team's leaders as a whole",
        Kind::Parents => "the parents as a whole",
        Kind::Feeder => "the feeder coaches as a whole",
        _ => "the team as a whole",
    }
}

/// The key a trainer who calls is drawn under: the former player he is
/// (`World.formers`).
const FORMER: u64 = 0x464F_524D_0000_0000;

/// The parent who talks for a player: whoever decides for him when that's
/// a parent or grandparent (family slot 0), else the first parent (1).
pub fn parent(w: &World, id: u32) -> Them {
    let slot = folk::person(w, id).map_or(1, |i| match w.people[i].family.decider {
        Decider::Parent | Decider::Grandparent => 0,
        _ => 1,
    });
    Them::Kin(id, slot)
}

/// The person a scene's `who` entry is, when he can be across from you.
fn them_of(w: &World, who: &Who) -> Option<Them> {
    match who {
        Who::Player(id) => Some(Them::Player(*id)),
        _ => super::office::them_of(w, who),
    }
}

fn one(who: Them) -> Present {
    Present { who, seat: Seat::Across, speaks: false }
}

fn listener(who: Them) -> Present {
    Present { who, seat: Seat::Listener, speaks: false }
}

/// A place from the meeting's words.
fn place_of(words: &str, or: Place) -> Place {
    let w = words.to_lowercase();
    if w.contains("locker") {
        Place::LockerRoom
    } else if w.contains("weight room") || w.contains("field") || w.contains("practice") {
        Place::Field
    } else if w.contains("facility") || w.contains("stadium") || w.contains("around") {
        Place::Facility
    } else if w.contains("cafeteria") || w.contains("hotel") || w.contains("suite") || w.contains("dinner") {
        Place::Out
    } else if w.contains("office") || w.contains("staff room") {
        Place::Office
    } else {
        or
    }
}

/// How you talk in a meeting: a call when its words say so, else in person.
fn channel_of(s: &Scene) -> Channel {
    let p = s.place.to_lowercase();
    if p.contains("phone") || p.contains("a call") || p.contains("cell") {
        Channel::Phone
    } else {
        Channel::InPerson(place_of(&s.place, Place::Office))
    }
}

/// Who is in the room for one of these meetings, him across first. None
/// when nobody stored is there to talk to (a briefing with no staff), and
/// then the meeting takes no typed line.
pub fn room(w: &World, s: &Scene) -> Option<Room> {
    if !plays(s.kind) {
        return None;
    }
    let team = s.team.or(Team::seat(w));
    match s.kind {
        // The captains, one room (lane N5d's council seat).
        Kind::Leaders => {
            let team = team?;
            let present: Vec<Present> = takeover::captains_of(w, team)
                .into_iter()
                .map(|i| listener(Them::Player(w.people[i].player.id)))
                .collect();
            (!present.is_empty()).then_some(Room { channel: Channel::Group(Group::Council), present })
        }
        // The parents' meeting: a parent for each player on the roster.
        Kind::Parents => {
            let team = team?;
            let present: Vec<Present> =
                team.roster(w).into_iter().map(|i| listener(parent(w, w.people[i].player.id))).collect();
            (!present.is_empty()).then_some(Room { channel: Channel::Group(Group::Parents), present })
        }
        // The feeder schools' coaches and the youth league's man.
        Kind::Feeder => {
            let team = team?;
            let present: Vec<Present> = cast::all(w, team, CastRole::Feeder)
                .into_iter()
                .chain(cast::all(w, team, CastRole::Youth))
                .map(|c| listener(Them::Cast(c.r, c.term)))
                .collect();
            (!present.is_empty()).then_some(Room { channel: Channel::Group(Group::Coaches), present })
        }
        // His parent calls about him: you talk to the parent.
        Kind::ParentCall => Some(Room { channel: Channel::Phone, present: vec![one(parent(w, s.a as u32))] }),
        // A signee on the phone with his family on the line: whoever decides
        // for him answers the deal and what other schools told them.
        Kind::SigneeRelease => {
            let id = s.a as u32;
            let i = folk::person(w, id)?;
            let mut present = vec![Present { who: Them::Player(id), seat: Seat::Across, speaks: true }];
            let (fam, seat) = match w.people[i].family.decider {
                Decider::Himself => (Them::Kin(id, 1), Seat::Family),
                _ => (Them::Kin(id, 0), Seat::Decider),
            };
            present.push(Present { who: fam, seat, speaks: true });
            Some(Room { channel: Channel::Phone, present })
        }
        // A call from someone you know (C15, C18, C19): a coach about a
        // player, a trainer with a kid, an agent with a client.
        Kind::Backchannel => {
            let who = match s.b {
                18 => Them::Drawn(WhoKind::Circle, FORMER | (s.c.max(0) as u64)),
                19 => Them::Agent(s.c.max(0) as usize),
                _ => s.who.iter().find_map(|x| match x {
                    Who::Coach(c) => Some(Them::Coach(*c)),
                    _ => None,
                })?,
            };
            Some(Room { channel: Channel::Phone, present: vec![one(who)] })
        }
        // The interview room: the prospect, on its clock (lane N5e).
        Kind::CombineTalk => Some(Room {
            channel: Channel::InPerson(place_of(&s.place, Place::Office)),
            present: vec![one(Them::Player(s.a as u32))],
        }),
        // Your job interview: the whole committee hears every line.
        Kind::JobInterview => {
            let mut v: Vec<Them> = s.who.iter().filter_map(|x| them_of(w, x)).collect();
            v.dedup();
            let first = *v.first()?;
            let mut present = vec![one(first)];
            present.extend(v.into_iter().skip(1).map(listener));
            Some(Room { channel: Channel::InPerson(place_of(&s.place, Place::Office)), present })
        }
        // Everyone else: the first person the meeting names, else (a
        // briefing your staff gives) your first assistant.
        _ => {
            let him = s.who.iter().find_map(|x| them_of(w, x)).or_else(|| {
                matches!(s.kind, Kind::Recruiting | Kind::FlightRisks | Kind::Plan)
                    .then(|| team.and_then(|t| facts::assistants(w, t).first().map(|&(c, _)| Them::Coach(c))))
                    .flatten()
            })?;
            Some(Room { channel: channel_of(s), present: vec![one(him)] })
        }
    }
}

/// Whether one of these meetings has someone across to talk to.
pub fn has_room(w: &World, s: &Scene) -> bool {
    room(w, s).is_some_and(|r| !r.present.is_empty())
}

/// The most men off the sheet the ear's card names. Proposed, not
/// confirmed.
pub const MAX_ON_SHEET: usize = 12;

/// Who else a line may name, for the ear's card: the players the meeting is
/// about (a staff proposal's two men, the player a coach calls about, a
/// signee's or a call's kid) and the men on the sheet in front of you (the
/// list of who might leave, the commits who wobble, the captains' words).
pub fn named(w: &World, s: &Scene) -> Vec<(Them, &'static str)> {
    let mut v: Vec<(Them, &'static str)> = Vec::new();
    if !plays(s.kind) {
        return v;
    }
    let room = room(w, s);
    let here = |who: Them| room.as_ref().is_some_and(|r| r.present.iter().any(|p| p.who == who));
    let push = |v: &mut Vec<(Them, &'static str)>, who: Them, why: &'static str| {
        if !here(who) && !v.iter().any(|x| x.0 == who) {
            v.push((who, why));
        }
    };
    for x in &s.who {
        if let Who::Player(id) = x {
            push(&mut v, Them::Player(*id), "the player this is about");
        }
    }
    if matches!(s.kind, Kind::ParentCall | Kind::Backchannel) && s.a != 0 {
        push(&mut v, Them::Player(s.a as u32), "the player this is about");
    }
    let mut on_sheet = 0;
    for d in &s.docs {
        for r in &d.rows {
            let Some(id) = r.pid else { continue };
            if on_sheet == MAX_ON_SHEET {
                return v;
            }
            if folk::person(w, id).is_some() {
                push(&mut v, Them::Player(id), "on the sheet in front of you");
                on_sheet += 1;
            }
        }
    }
    v
}

/// A name for someone only this lane draws: a trainer who calls (the
/// former player he is).
pub fn name(w: &World, who: Them) -> Option<String> {
    match who {
        Them::Drawn(WhoKind::Circle, k) if k & 0xFFFF_FFFF_0000_0000 == FORMER => {
            w.formers.get((k & 0xFFFF_FFFF) as u32).map(|x| x.name.clone())
        }
        _ => None,
    }
}

/// The most a line moves the trust of someone no other lane holds (a
/// parent, an agent, a trainer, another school's committee), down and up.
/// A parent: the parent call's own chips on his family (-0.06 for "then
/// talk to someone who does", +0.04 for hearing him out; the +0.06 chip
/// carries a promise, which a typed line logs on its own). The rest: the
/// office's caps (lane N5c's `CAP_DOWN`, `CAP_UP`). Proposed, not
/// confirmed.
pub const PARENT_DOWN: f32 = -0.06;
pub const PARENT_UP: f32 = 0.04;

/// Holds each line inside what a chip can do for the people this lane
/// adds: no mood (the world keeps none for them), and trust between the
/// caps. Players and the people in your building are held by their own
/// lanes (`hits::cap`, `office::cap`).
pub fn cap(w: &World, s: &Scene, room: &Room, hits: &mut [Sized]) {
    if !plays(s.kind) {
        return;
    }
    for p in &room.present {
        if matches!(p.who, Them::Player(_)) || super::office::key(w, p.who).is_some() {
            continue;
        }
        let (lo, hi) = match p.who {
            Them::Kin(..) => (PARENT_DOWN, PARENT_UP),
            _ => (super::office::CAP_DOWN, super::office::CAP_UP),
        };
        let mine: Vec<usize> = (0..hits.len()).filter(|&k| hits[k].hit.on == p.who).collect();
        let sum: f32 = mine.iter().map(|&k| hits[k].trust).sum();
        let k = if sum > hi {
            hi / sum
        } else if sum < lo {
            lo / sum
        } else {
            1.0
        };
        for i in mine {
            hits[i].trust *= k;
            hits[i].mood = 0.0;
        }
    }
}

/// Moves a parent's trust in you where lane N1 reads it (`folk.mine`, by
/// his key), and his son's, as a parent's call or the parents' meeting
/// moves it with a chip (the family takes it home). False for anyone else.
pub fn nudge(w: &mut World, who: Them, by: f32) -> bool {
    let Them::Kin(id, _) = who else { return false };
    let now = super::makeup::trust(w, who) as f32;
    w.folk.mine.set_trust(super::makeup::key(who), now + by);
    if let Some(i) = folk::person(w, id)
        && super::makeup::coached_by_you(w, i)
    {
        let p = &mut w.people[i];
        p.trust = (p.trust + by).clamp(0.0, 1.0);
    }
    true
}

/// How much the committee's trust moved in a job interview's talk counts in
/// its score (`dialogue::interview_answer`: an answer on what they want is
/// 0.45, a miss -0.3), and the most the talk can add or take. Proposed, not
/// confirmed.
pub const TALK_SCORE: f32 = 3.0;
pub const TALK_SCORE_MAX: f32 = 0.45;

/// After a line in one of these meetings: a job interview keeps what the
/// talk did to the committee for its score (`Scene.f[3]`, the one slot the
/// interview leaves free), and talks whose meeting is over are dropped.
pub fn after(w: &mut World, id: u32, room: &Room, _reading: &Reading, t: &Turn) {
    super::office::prune(w);
    let Some(k) = w.folk.scenes.iter().position(|s| s.id == id) else { return };
    if w.folk.scenes[k].kind != Kind::JobInterview {
        return;
    }
    let net: f32 = t.hits.iter().filter(|h| room.present.iter().any(|p| p.who == h.hit.on)).map(|h| h.trust).sum();
    let s = &mut w.folk.scenes[k];
    s.f[3] = (s.f[3] + net * TALK_SCORE).clamp(-TALK_SCORE_MAX, TALK_SCORE_MAX);
}
