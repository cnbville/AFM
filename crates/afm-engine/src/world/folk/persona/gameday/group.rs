//! Group talks on lane N4's conversation (5.5 "a group: every listener
//! hears every line", 6.1, 6.12): the team talk, the first team meeting and
//! the leadership council. The room is the whole roster (or the council),
//! a captain first: he answers questions and asks, as a captain sees the
//! room (lane N1's eye). A line to the whole room is one hit per man on
//! `Item::Room`, each sized for him and held inside his own chip range on
//! the team's subject (`hits::cap`), the way a chip's team talk lands.
//!
//! The council is its own meeting (`start_council`): without a key its
//! chip asks how the room is, and each member tells what he sees of the
//! unhappiest men by lane N1's rules (straight, partly, or not at all).

use super::super::Them;
use super::super::ear::{Hit, Item, Reading, Sized};
use super::super::knows::{Answer, Knows, TellParts, Tells, Topic};
use super::super::people::People;
use super::super::room::{Channel, Group, Present, Room, Seat};
use super::super::talk::{self, z};
use super::super::wants::Kind as PKind;
use crate::player::Trait;
use crate::world::World;
use crate::world::folk::scene::{self, Kind, Scene, Who};
use crate::world::folk::{self as folk, Team, takeover, talks};

/// The tag the ear's card gives the whole room in a group talk.
pub const ROOM: Them = Them::Drawn(PKind::Player, 0x524F_4F4D);

/// A line to the whole room counts this much of a word to one man on the
/// same thing. Proposed, not confirmed.
pub const ROOM_WEIGHT: f32 = 0.5;

/// The most men a council meeting asks about. Proposed, not confirmed.
pub const ASK_ABOUT: usize = 3;

/// Whether a scene is a group talk.
pub fn is_group(s: &Scene) -> bool {
    matches!(s.kind, Kind::TeamTalk | Kind::TeamMeeting | Kind::Council)
}

/// The council's members on the roster: your leadership council, else the
/// captains.
pub fn council(w: &World, team: Team) -> Vec<usize> {
    let roster = team.roster(w);
    let named: Vec<usize> =
        w.folk.rules.council.iter().filter_map(|&id| folk::person(w, id)).filter(|i| roster.contains(i)).collect();
    if named.is_empty() { takeover::captains_of(w, team) } else { named }
}

/// The men who lead a group scene's room: the council, or the captains.
pub fn leads(w: &World, s: &Scene) -> Vec<Them> {
    let Some(team) = s.team.or(Team::seat(w)) else { return Vec::new() };
    let l = if s.kind == Kind::Council { council(w, team) } else { takeover::captains_of(w, team) };
    l.into_iter().map(|i| Them::Player(w.people[i].player.id)).collect()
}

/// The room for a group scene: a captain first, then the rest.
pub fn room(w: &World, s: &Scene) -> Option<Room> {
    if !is_group(s) {
        return None;
    }
    let team = s.team.or(Team::seat(w))?;
    let (group, lead, rest) = match s.kind {
        Kind::Council => (Group::Council, council(w, team), Vec::new()),
        _ => (Group::Team, takeover::captains_of(w, team), team.roster(w)),
    };
    let mut present: Vec<Present> = Vec::new();
    for i in lead.into_iter().chain(rest) {
        let who = Them::Player(w.people[i].player.id);
        if !present.iter().any(|p| p.who == who) {
            present.push(Present { who, seat: Seat::Listener, speaks: false });
        }
    }
    (!present.is_empty()).then_some(Room { channel: Channel::Group(group), present })
}

/// Every hit on the whole room becomes one hit on each man in it, on
/// `Item::Room`.
pub fn expand(room: &Room, mut r: Reading) -> Reading {
    let mut out: Vec<Hit> = Vec::new();
    for h in r.hits.drain(..) {
        if h.on == ROOM {
            for p in &room.present {
                if !out.iter().any(|x| x.on == p.who && x.item == Item::Room && x.how == h.how) {
                    out.push(Hit { on: p.who, item: Item::Room, how: h.how });
                }
            }
        } else {
            out.push(h);
        }
    }
    r.hits = out;
    r
}

/// How a group line went over: the man it moved most each way, by name,
/// past 0.02 (the team talk's own `MOVED`).
pub fn landed(w: &World, hits: &[Sized]) -> Option<String> {
    let mut by: Vec<(u32, f32)> = Vec::new();
    for h in hits {
        let Them::Player(id) = h.hit.on else { continue };
        match by.iter_mut().find(|x| x.0 == id) {
            Some(x) => x.1 += h.trust + h.mood,
            None => by.push((id, h.trust + h.mood)),
        }
    }
    if by.is_empty() {
        return None;
    }
    let name = |id: u32| folk::person(w, id).map(|i| folk::player_link(w, i)).unwrap_or_default();
    let best = by.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1)).filter(|x| x.1 > 0.02);
    let worst = by.iter().copied().min_by(|a, b| a.1.total_cmp(&b.1)).filter(|x| x.1 < -0.02);
    let up = by.iter().filter(|x| x.1 > 0.01).count();
    let down = by.iter().filter(|x| x.1 < -0.01).count();
    Some(match (best, worst) {
        (Some(b), Some(x)) => format!("{} nods along; {} stares at the floor.", name(b.0), name(x.0)),
        (Some(b), None) if up > 2 * down => format!("It lands with the room. {} is the first to answer it.", name(b.0)),
        (Some(b), None) => format!("{} takes it best.", name(b.0)),
        (None, Some(x)) if down > 2 * up => format!("The room goes quiet. {} doesn't look up at all.", name(x.0)),
        (None, Some(x)) => format!("{} takes it worst.", name(x.0)),
        (None, None) => "They take it in.".into(),
    })
}

/// Opens a meeting with your leadership council (or your captains when you
/// have none): once a week, half an hour of your week.
pub fn start_council(w: &mut World) -> Result<u32, String> {
    let team = Team::seat(w).ok_or("You have no team.")?;
    if let Some(s) = w.folk.scenes.iter().find(|s| !s.done && s.kind == Kind::Council) {
        return Ok(s.id);
    }
    if council(w, team).is_empty() {
        return Err("You have no council or captains to meet.".into());
    }
    let wk = w.today.add(-w.today.weekday().index());
    if w.folk.scenes.iter().any(|s| s.kind == Kind::Council && s.done && s.date >= wk) {
        return Err("You met the council this week.".into());
    }
    if !talks::spend(w, 0.5) {
        return Err("Your week is full.".into());
    }
    let mut s = Scene::new(Kind::Council, w.today, "The leadership council");
    s.stops = false;
    s.team = Some(team);
    s.place = "Your office".into();
    Ok(scene::add(w, s))
}

/// The council meeting's opening: who's there, and the chips.
pub fn council_open(w: &mut World, s: &mut Scene) {
    let Some(team) = s.team.or(Team::seat(w)) else {
        s.end();
        return;
    };
    let members = council(w, team);
    let names: Vec<String> = members.iter().map(|&i| folk::player_link(w, i)).collect();
    s.who.push(Who::Group("The leadership council".into()));
    for &i in &members {
        s.who.push(Who::Player(w.people[i].player.id));
    }
    s.narrate(format!("Your office. Around the table: {}.", names.join(", ")));
    s.pick(600, "How's the room?", "ask", "calm", "They tell you what they see, as far as they will.");
    s.pick(601, "That's all for today.", "end", "calm", "");
}

/// The council meeting's chips: 600 asks how the room is, 601 ends it.
pub fn council_answer(w: &mut World, s: &mut Scene, code: u32) {
    s.you(&scene::your_name(w), if code == 600 { "How's the room?" } else { "That's all for today." });
    if code != 600 {
        s.end();
        return;
    }
    let Some(team) = s.team.or(Team::seat(w)) else {
        s.end();
        return;
    };
    for line in room_read(w, team) {
        match line {
            (Some(by), text) => s.them(&by, text, None),
            (None, text) => s.narrate(text),
        }
    }
    s.picks.retain(|p| p.code != 600);
    if s.picks.is_empty() {
        s.end();
    }
}

/// What the council tells you of the unhappiest men on the roster (up to
/// `ASK_ABOUT`): for each, the member who sees him best answers by lane
/// N1's rules, in front of the others (a group, so the room weighs on it).
pub fn room_read(w: &World, team: Team) -> Vec<(Option<String>, String)> {
    let members = council(w, team);
    let mut low: Vec<usize> =
        team.roster(w).into_iter().filter(|i| !members.contains(i) && w.people[*i].mood < -0.1).collect();
    low.sort_by(|a, b| w.people[*a].mood.total_cmp(&w.people[*b].mood));
    let mut out = Vec::new();
    for i in low.into_iter().take(ASK_ABOUT) {
        let man = Them::Player(w.people[i].player.id);
        let best = members
            .iter()
            .map(|&m| {
                let who = Them::Player(w.people[m].player.id);
                (m, People.knows(w, who, Topic::Mood, Some(man)))
            })
            .filter(|(_, k)| !k.is_empty())
            .max_by(|a, b| {
                let e = |k: &[super::super::knows::Known]| k.iter().map(|x| x.eye).fold(0.0, f64::max);
                e(&a.1).total_cmp(&e(&b.1))
            });
        let Some((m, known)) = best else { continue };
        let who = Them::Player(w.people[m].player.id);
        let parts = TellParts {
            trust: f64::from(talk::trust_of(w, who)),
            professionalism: f64::from(10.0 + 3.0 * z(w, who, Trait::Professionalism)),
            loyalty: tie(w, w.people[m].player.id, w.people[i].player.id),
            interest: 0.0,
            room: 1.0,
            pushed: 0,
        };
        let told = People.tell(w, who, &known, &parts);
        let name = w.people[m].name.clone();
        match told.answer {
            Answer::Straight | Answer::Partly | Answer::Lies if !told.released.is_empty() => {
                let text = told.lie.as_ref().map_or_else(|| told.released.join(" "), |l| l.words.clone());
                out.push((Some(name), folk::plain(&text)));
            }
            _ => out.push((None, format!("Asked about {}, {name} won't say.", w.people[i].name))),
        }
    }
    if out.is_empty() {
        out.push((None, "Nobody names anyone. As far as they'll say, the room is fine.".into()));
    }
    out
}

/// Two players' tie, 0-1.
fn tie(w: &World, a: u32, b: u32) -> f64 {
    w.folk.ties.of_id(a).filter(|t| t.a == b || t.b == b).map(|t| f64::from(t.strength) / 100.0).fold(0.0, f64::max)
}
