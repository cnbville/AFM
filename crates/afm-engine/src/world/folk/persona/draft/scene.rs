//! The draft side's talks as scenes (`scene::Kind::DraftTalk`): how each
//! opens, who is in the room, and its pick list for play without a key
//! (question 6: the same asks and requests through pick lists). With a key
//! every line plays as lane N4's conversation on this facet's parts
//! (`facet::parts`) and the picks stay as the moves list.
//!
//! - His family: a call home at an NFL seat. Their answers are a source in
//!   your file (`own::FAMILY_SOURCE`: a home visit's look that defends him).
//! - Your director of security, a college scout, a pro scout: what they
//!   have on a man, and sending them to find out (`scouts`).
//! - Your draft room on the clock: each man's case, then your pick
//!   (`debate`).
//! - Another club's GM: who he likes (real or smoke, `smoke::gm_says`),
//!   your staff's read of it, and planting your own.
//! - The draft-day call: to the man you took, or one you passed on who
//!   thought it was you.
//!
//! Sizes are Claude's picks, proposed, not confirmed.

use super::super::Them;
use super::super::knows::{Knows, Topic};
use super::super::room::{Channel, Group, Place, Present, Room, Seat};
use super::Sub;
use crate::world::World;
use crate::world::folk::scene::{self, Kind, Scene, Who};
use crate::world::folk::talks;
use crate::world::scouting::own;

/// Talk time a picked question costs (a call is half a sit-down).
/// Proposed, not confirmed.
pub const PICK_HOURS: f32 = 0.25;
/// What a draft-day call does for his word of you. Proposed, not
/// confirmed.
pub const WELCOME: f32 = 0.04;

/// The sub of a draft talk.
pub fn sub(s: &Scene) -> Option<Sub> {
    (s.kind == Kind::DraftTalk).then(|| Sub::of(s.b, s.c)).flatten()
}

/// Opens a draft-side talk: `player` is the man it's about (0 for the
/// draft room and a GM call with no name). Returns the scene's id.
pub fn start(w: &mut World, which: Sub, player: u32) -> Result<u32, String> {
    let t = super::club(w).ok_or("Only an NFL club's war room has these talks.")?;
    let name = |w: &World, id: u32| crate::world::folk::person(w, id).map(|i| w.people[i].name.clone());
    let (title, place, room) = match which {
        Sub::Family => {
            let i = crate::world::folk::person(w, player).ok_or("No such player.")?;
            let p = &w.people[i];
            if !matches!(p.stage, crate::world::Stage::Declared | crate::world::Stage::College) {
                return Err("His family takes calls about the draft only while he's a prospect.".into());
            }
            if w.folk
                .scenes
                .iter()
                .any(|s| sub(s) == Some(Sub::Family) && s.a == player as u64 && s.date.year() == w.today.year())
            {
                return Err("You've called his family already this year.".into());
            }
            let n = p.name.clone();
            let present = vec![
                Present { who: Them::Kin(player, 0), seat: Seat::Across, speaks: false },
                Present { who: Them::Kin(player, 1), seat: Seat::Family, speaks: false },
            ];
            (format!("A call to {n}'s family"), "On the phone".to_string(), Room { channel: Channel::Phone, present })
        }
        Sub::Security => {
            let n = name(w, player).ok_or("No such player.")?;
            let present = vec![Present { who: super::security(t), seat: Seat::Across, speaks: false }];
            (
                format!("Your director of security on {n}"),
                "Your office".to_string(),
                Room { channel: Channel::InPerson(Place::Office), present },
            )
        }
        Sub::Scout(k) => {
            let c = w.clubs.get(t).ok_or("No club.")?;
            let s = c.scouts.get(k).ok_or("No such scout.")?;
            let n = name(w, player).ok_or("No such player.")?;
            let present = vec![Present { who: super::scout(t, k), seat: Seat::Across, speaks: false }];
            (format!("{} on {n}", s.name), "On the phone".to_string(), Room { channel: Channel::Phone, present })
        }
        Sub::ProScout(k) => {
            let s = w.scouting.pro.scouts.get(k).ok_or("No such pro scout.")?;
            let n = name(w, player).ok_or("No such player.")?;
            let present = vec![Present { who: super::pro_scout(k), seat: Seat::Across, speaks: false }];
            (
                format!("{} on {n}", s.name),
                "Your office".to_string(),
                Room { channel: Channel::InPerson(Place::Office), present },
            )
        }
        Sub::Room => {
            if super::debate::candidates(w).is_empty() {
                return Err("You're not on the clock.".into());
            }
            let c = w.clubs.get(t).ok_or("No club.")?;
            let mut order: Vec<usize> = (0..c.scouts.len()).collect();
            order.sort_by_key(|&k| match c.scouts[k].role {
                crate::world::pro::scouts::Role::Director => 0,
                crate::world::pro::scouts::Role::National => 1,
                _ => 2,
            });
            let present: Vec<Present> = order
                .into_iter()
                .take(4)
                .enumerate()
                .map(|(j, k)| Present {
                    who: super::scout(t, k),
                    seat: if j == 0 { Seat::Across } else { Seat::YourStaff },
                    speaks: j > 0,
                })
                .collect();
            if present.is_empty() {
                return Err("Your club has no scouts in the room.".into());
            }
            (
                "The draft room, on the clock".to_string(),
                "The draft room".to_string(),
                Room { channel: Channel::Group(Group::Staff), present },
            )
        }
        Sub::Gm(o) => {
            if o == t || o >= w.clubs.len() {
                return Err("Call another club.".into());
            }
            let present = vec![Present { who: super::gm(w, o), seat: Seat::Across, speaks: false }];
            let club = w.nfl.get(o).map_or("another club", |c| c.name);
            (format!("A call to the {club}"), "On the phone".to_string(), Room { channel: Channel::Phone, present })
        }
        Sub::Call => {
            let n = name(w, player).ok_or("No such player.")?;
            let year = w.year;
            let i = crate::world::folk::person(w, player).ok_or("No such player.")?;
            let took = w.people[i].pro.as_ref().and_then(|c| c.draft).is_some_and(|d| d.0 == year && d.3 == t);
            let passed = w.folk.live.persona.draft.passed.iter().any(|x| x.0 == player && x.1.year() == year);
            if !took && !passed {
                return Err("He isn't a man you took or passed on in this draft.".into());
            }
            let present = vec![Present { who: Them::Player(player), seat: Seat::Across, speaks: false }];
            (format!("The call to {n}"), "On the phone".to_string(), Room { channel: Channel::Phone, present })
        }
    };
    let mut s = Scene::new(Kind::DraftTalk, w.today, title);
    s.stops = false;
    s.place = place;
    s.a = player as u64;
    let (b, c) = which.code();
    s.b = b;
    s.c = c;
    s.who = match room.present.first().map(|p| p.who) {
        Some(Them::Player(id)) => vec![Who::Player(id)],
        _ if player > 0 => vec![Who::Player(player)],
        _ => vec![Who::Group(room_label(which).into())],
    };
    s.outcome.clear();
    let id = scene::add(w, s);
    super::super::talk::store::begin(w, id, room);
    Ok(id)
}

fn room_label(which: Sub) -> &'static str {
    match which {
        Sub::Room => "Your scouts",
        Sub::Gm(_) => "Their general manager",
        Sub::Security => "Your director of security",
        _ => "Your staff",
    }
}

/// The first lines and the pick list.
pub(crate) fn open(w: &mut World, s: &mut Scene) {
    let Some(which) = sub(s) else { return s.end() };
    let id = s.a as u32;
    let them = across_name(w, s);
    match which {
        Sub::Family => s.narrate(format!("{them} picks up. They know why you're calling.")),
        Sub::Security => s.narrate(format!("{them} comes in with a folder.")),
        Sub::Scout(_) | Sub::ProScout(_) => s.narrate(format!("{them} has his notes in front of him.")),
        Sub::Room => s.narrate("You're on the clock. Your scouts are around the table, each with his board."),
        Sub::Gm(_) => s.narrate(format!("{them} picks up on the second ring.")),
        Sub::Call => {
            let took = crate::world::folk::person(w, id)
                .and_then(|i| w.people[i].pro.as_ref()?.draft)
                .is_some_and(|d| Some(d.3) == super::club(w) && d.0 == w.year);
            s.narrate(if took {
                "He picks up. His family is loud behind him."
            } else {
                "He picks up. It's quiet on his end."
            });
        }
    }
    picks(w, s);
}

fn across_name(w: &World, s: &Scene) -> String {
    let room = super::super::talk::store::room(w, s.id);
    room.and_then(|r| super::super::talk::across(&r))
        .map(|who| super::super::talk::channels::name(w, who))
        .unwrap_or_else(|| "He".into())
}

fn picks(w: &World, s: &mut Scene) {
    let Some(which) = sub(s) else { return };
    s.picks.clear();
    match which {
        Sub::Family => {
            for (k, (words, _)) in own::SOURCE_TOPICS.iter().enumerate() {
                s.pick(801 + k as u32, format!("Ask about {}", words.to_lowercase()), "Ask", "calm", "");
            }
            s.pick(806, "\"How has his health been?\"", "Ask", "calm", "");
            s.pick(809, "\"Thanks for your time.\"", "Wrap it up", "calm", "");
        }
        Sub::Security => {
            s.pick(811, "\"What's on him?\"", "Ask", "calm", "");
            s.pick(812, "\"Run the full background on him.\"", "Send him", "firm", "Five days; it goes in his file.");
            s.pick(819, "\"That's all.\"", "Wrap it up", "calm", "");
        }
        Sub::Scout(_) => {
            s.pick(821, "\"What do you have on him?\"", "Ask", "calm", "");
            s.pick(822, "\"What kind of kid is he?\"", "Ask", "calm", "");
            s.pick(823, "\"Find out what kind of man he is.\"", "Send him", "firm", "A school call; about a week.");
            s.pick(824, "\"Go see him again.\"", "Send him", "firm", "A cross-check; about ten days.");
            s.pick(829, "\"Thanks.\"", "Wrap it up", "calm", "");
        }
        Sub::ProScout(_) => {
            s.pick(831, "\"What do you have on him?\"", "Ask", "calm", "");
            s.pick(832, "\"Get him on film.\"", "Send him", "firm", "About three days.");
            s.pick(839, "\"Thanks.\"", "Wrap it up", "calm", "");
        }
        Sub::Room => {
            for (j, id) in super::debate::candidates(w).into_iter().enumerate() {
                let n = crate::world::folk::person(w, id).map_or(String::new(), |i| w.people[i].name.clone());
                s.pick(840 + j as u32, format!("Hear the case for {n}"), "Ask", "calm", "");
                s.pick(850 + j as u32, format!("\"We're taking {n}.\""), "Pick", "firm", "Turns in the card.");
                // A pick can't be undone: a typed line asks first.
                final_last(s);
            }
            s.pick(
                859,
                "\"Your call: make the pick.\"",
                "Hand it over",
                "calm",
                "Your staff takes the top of the board.",
            );
            final_last(s);
            s.pick(858, "Step out of the room", "Wrap it up", "calm", "");
        }
        Sub::Gm(_) => {
            s.pick(861, "\"Who do you like at your pick?\"", "Ask", "calm", "");
            for (j, id) in plant_names(w).into_iter().enumerate() {
                let n = crate::world::folk::person(w, id).map_or(String::new(), |i| w.people[i].name.clone());
                s.pick(862 + j as u32, format!("\"We love {n}.\""), "Plant it", "calm", "Only his club hears it.");
            }
            s.pick(869, "\"Good luck Thursday.\"", "Wrap it up", "calm", "");
        }
        Sub::Call => {
            s.pick(871, "\"Welcome. We've wanted you all spring.\"", "Welcome him", "warm", "");
            s.pick(872, "\"It was close. Keep your head up.\"", "Tell him it was close", "warm", "");
            s.pick(879, "\"Go enjoy your night.\"", "Wrap it up", "calm", "");
        }
    }
}

fn final_last(s: &mut Scene) {
    if let Some(p) = s.picks.last_mut() {
        p.kind = crate::world::folk::scene::PickKind::Final;
    }
}

/// The men you can say you love on a call: the consensus names around
/// your first pick.
fn plant_names(w: &World) -> Vec<u32> {
    let Some(t) = super::club(w) else { return Vec::new() };
    let Some(board) = crate::world::scouting::public::consensus(w) else { return Vec::new() };
    let slot = crate::world::nfl_year::draft_order(w).and_then(|o| o.iter().position(|&x| x == t)).unwrap_or(0);
    board.into_iter().skip(slot.saturating_sub(2)).take(3).collect()
}

/// A pick.
pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32) {
    let Some(which) = sub(s) else { return s.end() };
    let id = s.a as u32;
    let you = scene::your_name(w);
    let them = across_name(w, s);
    let label = s.picks.iter().find(|p| p.code == code).map(|p| p.label.clone()).unwrap_or_default();
    if !label.is_empty() && !label.starts_with("Hear") && !label.starts_with("Step") {
        s.you(&you, label.trim_matches('"').to_string());
    }
    let say = |s: &mut Scene, lines: Vec<String>| {
        if lines.is_empty() {
            s.them(&them, "I don't have anything on that.", None);
        } else {
            s.them(&them, lines.join(" "), None);
        }
    };
    match (which, code) {
        (_, 809 | 819 | 829 | 839 | 858 | 869 | 879) => return close(w, s),
        (Sub::Family, 801..=804) => {
            let topic = (code - 801) as u8;
            let text = family_source(w, id, topic);
            say(s, text.into_iter().collect());
            talks::charge(w, PICK_HOURS);
        }
        (Sub::Family, 806) => {
            let k = super::super::people::People.knows(w, Them::Kin(id, 0), Topic::Injury, Some(Them::Player(id)));
            say(s, k.into_iter().take(1).map(|x| x.words).collect());
            talks::charge(w, PICK_HOURS);
        }
        (Sub::Security, 811) => {
            let t = super::club(w).unwrap_or(0);
            say(s, super::scouts::security_words(w, crate::world::scouting::readers::Staff::Club(t), id));
        }
        (Sub::Security, 812) | (Sub::Scout(_), 823 | 824) | (Sub::ProScout(_), 832) => {
            let topic = match code {
                812 => Topic::Case,
                823 => Topic::Character,
                _ => Topic::Football,
            };
            let him = across(w, s);
            let r = him.and_then(|h| super::scouts::send(w, h, id, topic));
            say(s, r.into_iter().collect());
        }
        (Sub::Scout(_) | Sub::ProScout(_), 821 | 822 | 831) => {
            let topic = if code == 822 { Topic::Character } else { Topic::ScoutFile };
            let him = across(w, s);
            let code = him.and_then(|h| super::scouts::code_of(w, h));
            let mut lines = code.map_or(Vec::new(), |c| super::scouts::answer(w, c, id, topic));
            if lines.is_empty() {
                lines.push("I haven't seen him myself. I can go find out.".into());
            }
            say(s, lines);
        }
        (Sub::Room, 840..=849) => {
            let j = (code - 840) as usize;
            let t = super::club(w).unwrap_or(0);
            if let Some(c) = super::debate::candidates(w).get(j).and_then(|&x| super::debate::case(w, t, x)) {
                s.other(&c.by.1, c.case.clone());
                if let Some((_, n, x)) = &c.against {
                    s.other(n, x.clone());
                }
                for f in &c.flags {
                    s.narrate(f.clone());
                }
            }
        }
        (Sub::Room, 850..=857) => {
            let j = (code - 850) as usize;
            if let Some(&pid) = super::debate::candidates(w).get(j) {
                match crate::world::scouting::live::pick(w, pid) {
                    Ok(()) => s.outcome.push("The card is in.".into()),
                    Err(e) => s.outcome.push(e),
                }
            }
            return close(w, s);
        }
        (Sub::Room, 859) => {
            match crate::world::scouting::live::auto_pick(w) {
                Ok(()) => s.outcome.push("Your staff turned in the card.".into()),
                Err(e) => s.outcome.push(e),
            }
            return close(w, s);
        }
        (Sub::Gm(t), 861) => {
            if let Some((pid, real)) = super::smoke::gm_says(w, t, None) {
                let n = crate::world::folk::person(w, pid).map_or(String::new(), |i| w.people[i].name.clone());
                s.them(&them, format!("Honestly? We love {n}."), None);
                super::smoke::tell(w, t, pid, real);
                let r = super::smoke::read(w, t, pid, true);
                s.narrate(format!("Your staff's read: {}. {}", r.band, r.why.join(" ")));
            } else {
                s.them(&them, "We're still working through it.", None);
            }
        }
        (Sub::Gm(_), 862..=864) => {
            let j = (code - 862) as usize;
            if let (Some(&pid), Some(him)) = (plant_names(w).get(j), across(w, s)) {
                if let Some(x) = super::smoke::plant(w, him, pid) {
                    s.narrate(x);
                }
                s.them(&them, "Good to know.", None);
            }
        }
        (Sub::Call, 871 | 872) => {
            let key = super::super::makeup::key(Them::Player(id));
            let now = super::super::makeup::trust(w, Them::Player(id)) as f32;
            let lift = if code == 871 { WELCOME } else { WELCOME * 0.75 };
            w.folk.mine.set_trust(key, (now + lift).clamp(0.0, 1.0));
            s.them(
                &them,
                if code == 871 { "Thank you, Coach. I won't let you down." } else { "Appreciate the call, Coach." },
                None,
            );
        }
        _ => {}
    }
    picks(w, s);
}

/// The man across from you in the scene's room.
fn across(w: &World, s: &Scene) -> Option<Them> {
    super::super::talk::store::room(w, s.id).and_then(|r| super::super::talk::across(&r))
}

/// His family's answer on a source topic (`own::SOURCE_TOPICS`), kept as
/// a source in your file.
pub fn family_source(w: &mut World, id: u32, topic: u8) -> Option<String> {
    let (_, ask, answer) = own::source_answer(w, id, own::FAMILY_SOURCE, topic).ok()?;
    let year = w.today.year();
    let list = &mut w.scouting.own.sources;
    if !list.iter().any(|x| x.id == id && x.source == own::FAMILY_SOURCE && x.topic == topic && x.date.year() == year) {
        list.push(ask);
        if list.len() > own::MAX_SOURCES {
            list.remove(0);
        }
    }
    Some(answer.text)
}

/// Handed to your staff: the talk doesn't happen.
pub(crate) fn hand(_w: &mut World, s: &mut Scene) {
    s.outcome.push("You left it to your staff.".into());
    s.end();
}

/// Ends the talk.
pub(crate) fn close(w: &mut World, s: &mut Scene) {
    if s.outcome.is_empty() {
        s.outcome.push("The talk is over.".into());
    }
    s.end();
    super::super::talk::store::close(w, s.id);
}

/// After a typed line on a draft talk: a GM's answer goes in the book; his
/// family's answers on what drives and shapes him go in your file.
pub(crate) fn after_line(w: &mut World, id: u32, said: &super::super::talk::Said) {
    use super::super::ear::Act;
    let Some(s) = w.folk.scenes.iter().find(|s| s.id == id) else { return };
    let Some(which) = sub(s) else { return };
    let pid = s.a as u32;
    for a in &said.reading.acts {
        let Act::Question { topic, about, .. } = a else { continue };
        match which {
            Sub::Gm(t) => {
                let ask = match about {
                    Some(Them::Player(x)) => Some(*x),
                    _ => None,
                };
                if let Some((p, real)) = super::smoke::gm_says(w, t, ask) {
                    super::smoke::tell(w, t, p, real);
                }
            }
            Sub::Family => {
                let k = match topic {
                    Topic::Effort => Some(0),
                    Topic::Character | Topic::Mood => Some(2),
                    Topic::Wants | Topic::Money | Topic::Life => Some(3),
                    _ => None,
                };
                if let Some(k) = k {
                    family_source(w, pid, k);
                }
            }
            _ => {}
        }
    }
}
