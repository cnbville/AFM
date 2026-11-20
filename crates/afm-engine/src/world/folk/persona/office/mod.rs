//! Live talks v3, lane N5c: staff, bosses and money
//! (`docs/design/live-talks-persona.md`, sections 6.8 to 6.10 and 6.13,
//! 8.2, 8.3 and question 10). The people in your building are played as
//! themselves on lane N4's conversation: your coordinators and position
//! coaches, the team doctor, the academic and compliance directors, your
//! boss (the AD, the principal, the owner, your GM), the boosters, and a
//! candidate's references before you hire.
//!
//! What this lane adds to the shared parts, and nothing else:
//!
//! - who sits across in each of these meetings (`across`, `room`), and two
//!   new talks you call yourself: a sit-down with someone in your building
//!   (`sit`) and a reference call about a coach (`refs`);
//! - what they know beyond lane N1's people (`Office` as `Knows`): the
//!   team doctor's chart (`doctor`), a reference's read of the man he
//!   worked with (`refs`), a position coach's room, the money your boss
//!   holds, and the rules in plain words (`rules`);
//! - deals (`deals`): a coach's job offer through the built hiring talks,
//!   a raise or more years for your own assistant, the play-calling, your
//!   own extension with your boss;
//! - requests that run (`asks`): a second opinion, the pressure to clear a
//!   hurt man, a plan setting from a coordinator, your boss's money, time
//!   and backing, a booster's money for the collective;
//! - trust: a line to someone here moves the trust the world keeps for him
//!   (`folk.mine`, the same store his meeting's chips move), held inside
//!   what a chip can do (`cap`).
//!
//! The one rule holds (section 2): their words never change the world;
//! yours do only through the game's own rules, the same functions a chip
//! runs. Every size here is Claude's, proposed, not confirmed.

pub mod asks;
pub mod deals;
pub mod doctor;
pub mod refs;
pub mod rules;
pub mod sit;

use super::Them;
use super::ear::{Confirm, Reading, Sized, Turn};
use super::knows::{Known, Knows, Source, TellParts, Tells, Told, Topic};
use super::room::{Channel, Place, Present, Room, Seat};
use super::talk::Parts;
use super::terms::{Offer, Valuing, Weighed};
use super::wants::{Kind as WhoKind, Ran, Want, Wanting, Willing};
use crate::world::World;
use crate::world::coach::Role;
use crate::world::folk::cast::{self, CastRef, Role as CastRole};
use crate::world::folk::scene::{Kind, Pick, Scene, Who};
use crate::world::folk::{self as folk, Team};

/// The meetings this lane plays as v3 talks, when a key is usable.
pub const PLAYS: [Kind; 15] = [
    Kind::BossFirst,
    Kind::BossMeeting,
    Kind::Gm,
    Kind::Holdover,
    Kind::StaffReport,
    Kind::StaffLine,
    Kind::StaffMeeting,
    Kind::Interview,
    Kind::Firing,
    Kind::SecondOpinion,
    Kind::Trainer,
    Kind::Academic,
    Kind::Booster,
    Kind::Sitdown,
    Kind::Reference,
];

/// Whether a meeting plays as a v3 talk with someone in your building.
pub fn plays(k: Kind) -> bool {
    PLAYS.contains(&k)
}

/// A talk that is only a talk: it ends when the talk ends. The built
/// meetings keep their own picks to finish (a holdover kept or let go, a
/// second opinion, the goals), so the talk ending leaves them open.
pub fn talk_only(k: Kind) -> bool {
    matches!(k, Kind::Sitdown | Kind::Reference)
}

/// A cast person as the talk holds him: his job and his term in it.
pub fn cast_them(w: &World, r: CastRef) -> Them {
    Them::Cast(r, cast::get(w, r).term)
}

/// The person a scene's `who` entry is, when he can be across from you.
pub fn them_of(w: &World, who: &Who) -> Option<Them> {
    match who {
        Who::Coach(c) => Some(Them::Coach(*c)),
        Who::Cast(tok) => cast::from_token(w, tok).map(|c| Them::Cast(c.r, c.term)),
        _ => None,
    }
}

/// The man across from you: the first coach or cast person the meeting
/// names; a sit-down with the compliance director is his; your boss in a
/// boss meeting; a holdover or a coach in his meeting.
pub fn across(w: &World, s: &Scene) -> Option<Them> {
    if s.kind == Kind::Sitdown && s.b == sit::COMPLIANCE {
        return rules::director(w, s.team.or(Team::seat(w))?);
    }
    if let Some(t) = s.who.iter().find_map(|x| them_of(w, x)) {
        return Some(t);
    }
    let team = s.team.or(Team::seat(w))?;
    match s.kind {
        Kind::BossFirst | Kind::BossMeeting => Some(cast_them(w, CastRef::new(team, cast::boss_role(team), 0))),
        Kind::Gm => Some(cast_them(w, CastRef::new(team, CastRole::Gm, 0))),
        Kind::Trainer | Kind::SecondOpinion => Some(cast_them(w, CastRef::new(team, CastRole::Trainer, 0))),
        Kind::Academic => Some(cast_them(w, CastRef::new(team, CastRole::Academic, 0))),
        Kind::Interview | Kind::Firing | Kind::StaffMeeting | Kind::Holdover => {
            let c = s.a as usize;
            (s.a != u64::MAX && c < w.coaches.len()).then_some(Them::Coach(c))
        }
        _ => None,
    }
}

/// The room for one of this lane's meetings: him across, in the place the
/// meeting names (a reference call is a phone call). The hurt man sits in
/// on his own second opinion: he hears what you ask of the doctor.
pub fn room(w: &World, s: &Scene) -> Option<Room> {
    // The whole holdover staff: every coach in it, each himself.
    if super::gameday::group::holdover_room(s) {
        return super::coaching::holdover_room(w);
    }
    let him = across(w, s)?;
    let channel = if s.kind == Kind::Reference { Channel::Phone } else { Channel::InPerson(place_of(&s.place)) };
    let mut present = vec![Present { who: him, seat: Seat::Across, speaks: false }];
    if s.kind == Kind::SecondOpinion {
        present.push(Present { who: Them::Player(s.a as u32), seat: Seat::Listener, speaks: false });
    }
    Some(Room { channel, present })
}

fn place_of(words: &str) -> Place {
    let w = words.to_lowercase();
    if w.contains("field") || w.contains("practice") {
        Place::Field
    } else if w.contains("locker") {
        Place::LockerRoom
    } else if w.contains("training room") || w.contains("facility") || w.contains("center") {
        Place::Facility
    } else if w.contains("dinner") || w.contains("restaurant") || w.contains("club") {
        Place::Out
    } else {
        Place::Office
    }
}

/// The key the world keeps his trust in you under (`folk.mine`): the
/// store his meeting's chips move.
pub fn key(w: &World, who: Them) -> Option<u64> {
    match who {
        Them::Coach(c) => Some(folk::coach_key(c)),
        Them::Cast(r, term) => Some(r.key(term)),
        Them::Drawn(..) => Some(super::makeup::key(who)),
        _ => None,
    }
    .filter(|_| plays_with(w, who))
}

/// Someone this lane plays: on your staff, your boss, a booster, the
/// compliance office, or a reference you call about a coach.
pub fn plays_with(w: &World, who: Them) -> bool {
    matches!(
        super::makeup::kind(w, who),
        WhoKind::Staff | WhoKind::Boss | WhoKind::Booster | WhoKind::Compliance | WhoKind::OtherCoach
    ) || matches!(who, Them::Cast(r, _) if r.role == CastRole::AthleticDirector)
}

/// His trust in you, 0-1, as the world keeps it.
pub fn trust(w: &World, who: Them) -> f32 {
    super::makeup::trust(w, who) as f32
}

/// Moves his trust in you where the world keeps it. False when he isn't
/// someone this lane plays.
pub fn nudge(w: &mut World, who: Them, by: f32) -> bool {
    let Some(k) = key(w, who) else { return false };
    let now = trust(w, who);
    w.folk.mine.set_trust(k, now + by);
    true
}

/// The most a line moves the trust of someone here, up and down: the boss
/// meeting's warmest chip (+0.02, agreeing to his goals or owning the
/// review) and its worst (-0.05, asking an impatient boss for time).
/// Wording alone never beats the best chip (lane 9's rule). Proposed, not
/// confirmed.
pub const CAP_UP: f32 = 0.02;
pub const CAP_DOWN: f32 = -0.05;

/// Holds each line's hits on someone here inside `CAP_DOWN`..`CAP_UP`.
pub fn cap(w: &World, room: &Room, hits: &mut [Sized]) {
    for p in &room.present {
        if matches!(p.who, Them::Player(_)) || key(w, p.who).is_none() {
            continue;
        }
        let mine: Vec<usize> = (0..hits.len()).filter(|&k| hits[k].hit.on == p.who).collect();
        let sum: f32 = mine.iter().map(|&k| hits[k].trust).sum();
        let k = if sum > CAP_UP {
            CAP_UP / sum
        } else if sum < CAP_DOWN {
            CAP_DOWN / sum
        } else {
            1.0
        };
        for i in mine {
            hits[i].trust *= k;
            hits[i].mood = 0.0;
        }
    }
}

/// The parts a talk with someone here is played with: lane N1's people
/// with what this lane adds, and its deals and requests.
#[derive(Clone, Copy, Debug, Default)]
pub struct Office;

impl Knows for Office {
    fn knows(&self, w: &World, him: Them, topic: Topic, about: Option<Them>) -> Vec<Known> {
        let mut v = super::people::People.knows(w, him, topic, about);
        v.extend(doctor::knows(w, him, topic, about));
        v.extend(refs::knows(w, him, topic, about));
        v.extend(rules::knows(w, him, topic));
        v.extend(room_mood(w, him, topic, about));
        v.extend(money(w, him, topic));
        v
    }
}

impl Tells for Office {
    fn tell(&self, w: &World, him: Them, known: &[Known], parts: &TellParts) -> Told {
        super::people::People.tell(w, him, known, parts)
    }
}

impl Wanting for Office {
    fn kind(&self, w: &World, him: Them) -> WhoKind {
        super::people::People.kind(w, him)
    }
    fn wants(&self, w: &World, him: Them) -> Vec<Want> {
        super::people::People.wants(w, him)
    }
}

impl Valuing for Office {
    fn best(&self, w: &World, him: Them) -> super::terms::Best {
        deals::best(w, him).unwrap_or_else(|| super::talk::DEALS.best(w, him))
    }
    fn weigh(&self, w: &World, him: Them, offer: &Offer) -> Weighed {
        deals::weigh(w, him, offer).unwrap_or_else(|| super::talk::DEALS.weigh(w, him, offer))
    }
    fn why(&self, w: &World, him: Them, offer: &Offer, _wd: &Weighed) -> Vec<String> {
        deals::why(w, him, offer)
    }
}

impl super::wants::Acting for Office {
    fn willing(&self, w: &World, him: Them, ask: &super::terms::Request) -> Willing {
        asks::willing(w, him, ask).unwrap_or_else(|| super::asks::Asks.willing(w, him, ask))
    }
    fn run(&self, w: &mut World, him: Them, ask: &super::terms::Request) -> Ran {
        match asks::run(w, him, ask) {
            Some(r) => r,
            None => super::asks::Asks.run(w, him, ask),
        }
    }
    fn why(&self, w: &World, him: Them, ask: &super::terms::Request, wl: &Willing) -> Vec<String> {
        let mut v = asks::why(w, him, ask);
        // A coordinator's numbers and stance on a plan ask.
        v.extend(asks::words(w, him, ask, wl));
        v
    }
}

/// The parts for this lane's talks.
pub fn parts() -> Parts<'static> {
    static O: Office = Office;
    static F: super::talk::feel::Sized = super::talk::feel::Sized;
    Parts { knows: &O, tells: &O, wanting: &O, valuing: &O, acting: &O, feel: &F }
}

/// What this lane does after a turn is made real (lane N4's `apply`): a
/// reference's reads go on the coach's page and you owe him one, and a
/// boss's no to an ask is kept as the meeting keeps it. Returns lines for
/// the scene.
pub fn after(w: &mut World, id: u32, room: &Room, reading: &Reading, t: &Turn) -> Vec<String> {
    let mut out = Vec::new();
    prune(w);
    let Some(him) = room.present.first().map(|p| p.who) else { return out };
    out.extend(refs::after(w, id, him, reading, t));
    out.extend(asks::after(w, him, t));
    // Under a coach's answer on his scheme or his recruiting, what your
    // staff makes of it (Cain 2026-11-17: item 9): for you, never in his
    // mouth.
    if let Them::Coach(c) = him
        && super::telling::works_for_you(w, him)
    {
        let mut said: Vec<super::knows::Topic> = Vec::new();
        for a in &reading.acts {
            if let super::ear::Act::Question { topic, about, .. } = a
                && about.is_none_or(|x| x == him)
                && !said.contains(topic)
                && let Some(n) = super::coaching::read_note(w, c, *topic)
            {
                said.push(*topic);
                out.push(n);
            }
        }
    }
    // A no to your extension, kept on record with his reasons (your boss
    // across the table, not your head coach when you're his assistant).
    if let Some(she) = deals::your_boss_she(w, him)
        && let Some(team) = Team::seat(w)
    {
        for (o, wd) in &t.weighed {
            if *wd == Weighed::No && deals::extension(o) {
                out.push(deals::refused(w, team, o, she));
            }
        }
    }
    out
}

/// Drops the state of talks whose meeting is over: a built meeting that
/// outlived its talk (`talk::end` keeps it) is closed by its own picks.
pub fn prune(w: &mut World) {
    let live: Vec<u32> = w.folk.scenes.iter().filter(|s| !s.done).map(|s| s.id).collect();
    w.folk.live.persona.open.retain(|o| live.contains(&o.scene));
}

/// Runs what you confirmed in a talk with someone here (question 4): an
/// offer through this lane's deals, a request through its own rules, a
/// decision about a coach (letting him go). None leaves it to the shared
/// run.
pub fn confirm(w: &mut World, id: u32, him: Them, c: &Confirm) -> Option<Vec<String>> {
    match c {
        Confirm::Offer(o) => deals::agree(w, id, him, o),
        Confirm::Request(r) => asks::run(w, him, r).map(|r| r.done),
        // An ask you put to someone else in the room runs on him.
        Confirm::RequestOf(doer, r) => asks::run(w, *doer, r).map(|r| r.done),
        Confirm::Decision(super::terms::Decision::CoachLetGo) => {
            let Them::Coach(c) = him else { return None };
            if !asks::on_your_staff(w, c) {
                return Some(vec!["He isn't on your staff.".into()]);
            }
            Some(vec![crate::world::folk::staffing::fire(w, c, "let go by the head coach")])
        }
        Confirm::Decision(_) | Confirm::Action(_) => None,
    }
}

/// What a position coach sees of his room's mood, or a coordinator of his
/// side's (asked how his room is): the room's band and its unhappiest man.
fn room_mood(w: &World, him: Them, topic: Topic, about: Option<Them>) -> Vec<Known> {
    let (Them::Coach(c), Topic::Mood, None) = (him, topic, about) else { return Vec::new() };
    if !asks::on_your_staff(w, c) {
        return Vec::new();
    }
    let Some(team) = Team::seat(w) else { return Vec::new() };
    let Some(role) = w.coaches[c].job.map(|j| j.role) else { return Vec::new() };
    let mine = |i: usize| {
        let g = crate::world::coach::Group::of(w.people[i].player.position);
        match role {
            Role::PositionCoach(x) => g == x,
            Role::OffensiveCoordinator => g.offense(),
            Role::DefensiveCoordinator => !g.offense(),
            Role::HeadCoach => true,
        }
    };
    let men: Vec<usize> = team.roster(w).into_iter().filter(|&i| mine(i)).collect();
    if men.is_empty() {
        return Vec::new();
    }
    let avg = men.iter().map(|&i| w.people[i].mood as f64).sum::<f64>() / men.len() as f64;
    let band = match avg {
        x if x >= 0.25 => "his room is in a good place, the guys like coming to work",
        x if x >= 0.0 => "his room is fine, nothing he's worried about",
        x if x >= -0.2 => "his room is a little flat right now",
        _ => "his room is down, and it's showing up in meetings",
    };
    let mut words = band.to_string();
    if let Some(&low) = men.iter().min_by(|a, b| w.people[**a].mood.total_cmp(&w.people[**b].mood))
        && w.people[low].mood < -0.2
    {
        words.push_str(&format!("; {} is the one he'd keep an eye on", w.people[low].name));
    }
    let m = super::makeup::makeup(w, him);
    vec![Known {
        topic: Topic::Mood,
        about: None,
        from: Source::Job,
        eye: m.judgement,
        point: None,
        says: Vec::new(),
        words,
        sure: (0.35 + 0.65 * m.judgement / 20.0) as f32,
        day: None,
    }]
}

/// What your boss knows of the money (asked about it): the staff pool and
/// the recruiting budget he has stretched for you, and how the big booster
/// feels about you (`booster_push`).
fn money(w: &World, him: Them, topic: Topic) -> Vec<Known> {
    if topic != Topic::Money || super::makeup::kind(w, him) != WhoKind::Boss {
        return Vec::new();
    }
    let mut parts: Vec<String> = Vec::new();
    let st = &w.folk.staff;
    parts.push(if st.pool_bonus > 0.0 {
        "the staff pool is already above what it was when you came".into()
    } else {
        "the staff pool is where it was when you came".into()
    });
    if st.recruit_bonus > 0.0 {
        parts.push("he's already found more recruiting money for you".into());
    }
    if let Some(Team::College(p)) = Team::seat(w) {
        let push = crate::world::folk::boss::booster_push(w, p);
        parts.push(
            match push {
                x if x >= 0.03 => "the big booster is unhappy and he's letting people know",
                x if x > 0.0 => "the big booster has questions",
                _ => "the big booster is behind you",
            }
            .into(),
        );
    }
    vec![Known {
        topic: Topic::Money,
        about: None,
        from: Source::Job,
        eye: 16.0,
        point: None,
        says: Vec::new(),
        words: parts.join("; "),
        sure: 0.9,
        day: None,
    }]
}

// The two talks you call yourself (`sit`): the scene's own open, answer
// and hand.

pub(crate) fn open(w: &mut World, s: &mut Scene) {
    sit::open(w, s);
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    sit::answer(w, s, code, picked);
}

pub(crate) fn hand(_w: &mut World, s: &mut Scene) {
    s.outcome.push("You let it go for now.".into());
    s.end();
}
