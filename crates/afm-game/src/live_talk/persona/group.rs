//! The ear's card for a group talk (lane N5d): the team talk, the first
//! team meeting and the leadership council. Everyone in the room hears
//! every line (5.5). The card tags the whole room p0 with its one item c0
//! (a line to everyone), then the captains or the council, then the men
//! most on the room's mind (the unhappiest first), each with his own items
//! named for him: his pride, what's on his mind, your promises to him.
//!
//! Group talks (thread "Make group talks feel like a room", item 1): after
//! the whole room the card names the room's groups (the starters, the
//! backups, the seniors, the newcomers, the offense, the defense, each
//! position room: `crowd::blocs`), so a line can land on each group its own
//! way ("every job is open": the starters doubted, the backups reassured)
//! and a line to one group ("Defense, ...") goes to that group. Who answers
//! is the game's (`crowd::voices`), never the card's.

use afm_engine::world::World;
use afm_engine::world::folk::persona::Them;
use afm_engine::world::folk::persona::crowd;
use afm_engine::world::folk::persona::ear::Item;
use afm_engine::world::folk::persona::gameday::group::{self, ROOM};
use afm_engine::world::folk::persona::meetings;
use afm_engine::world::folk::persona::room::Room;
use afm_engine::world::folk::scene::Scene;
use afm_engine::world::folk::{self as folk, claims, talk_points as tp};

use super::cards::{self, person};
use super::{EarCard, EarKeys};
use crate::live_talk::card::{Tagged, quoted};

/// The most men the card names beyond the captains. Proposed, not
/// confirmed.
pub const MAX_NAMED: usize = 12;
/// The most leaders (captains or council members) it names. Proposed, not
/// confirmed.
pub const MAX_LEADS: usize = 8;

/// The ear's card and its keys for one line to a group.
pub fn ear(w: &World, s: &Scene, room: &Room, line: &str) -> (EarCard, EarKeys) {
    let mut keys = EarKeys::default();
    let mut c = EarCard {
        setting: cards::setting(w, s, room),
        coach: cards::coach(w),
        them: format!(
            "{}: everyone hears every line. p0 is the whole room and c0 a line meant for all of them. The groups \
             in the room are tagged too: a line to one group (\"Defense, ...\", \"Seniors, ...\") is a hit on that \
             group's tag with c0, and a line that lands differently on different groups gets one hit per group, each \
             its own way (\"every job is open\": the starters doubted, the backups reassured), with no hit on p0. A \
             promise made to the room or a group: about is the tag of who it's made to (p0 for everyone); to one man, \
             his tag; for ahead_of, the teammate he'd play ahead of. A question to one man by name: about is his tag",
            cards::channel_words(room.channel)
        ),
        so_far: cards::so_far(s),
        line: quoted(line),
        ..EarCard::default()
    };
    // The room's calendar is your team's (`talk::year`).
    if let Some(p) = room.present.first() {
        c.year = afm_engine::world::folk::persona::talk::year::words(w, p.who);
    }
    c.people.push(Tagged { tag: "p0".into(), text: "the whole room".into() });
    keys.people.push(("p0".into(), ROOM));
    c.items.push(Tagged { tag: "c0".into(), text: meetings::whole_words(s).into() });
    keys.items.push(("c0".into(), Item::Room));
    // The room's groups, each its own tag.
    let lvl = crowd::level(w);
    let blocs = crowd::blocs(w, room);
    for b in &blocs {
        let tag = format!("p{}", keys.people.len());
        let n = crowd::members(w, room, *b).len();
        c.people
            .push(Tagged { tag: tag.clone(), text: format!("{} ({n} of them, a group in the room)", b.words(lvl)) });
        keys.people.push((tag, b.them()));
    }
    // The captains (or the council) first, as the room has them; then the
    // men most on the room's mind.
    let leads = group::leads(w, s);
    let lead: Vec<Them> = room.present.iter().map(|p| p.who).filter(|x| leads.contains(x)).take(MAX_LEADS).collect();
    let mut rest: Vec<(Them, f32)> = room
        .present
        .iter()
        .filter(|p| !lead.contains(&p.who))
        .filter_map(|p| match p.who {
            Them::Player(id) => folk::person(w, id).map(|i| (p.who, w.people[i].mood)),
            // A parent or another coach in the room (the older meetings).
            _ => Some((p.who, 0.0)),
        })
        .collect();
    rest.sort_by(|a, b| a.1.total_cmp(&b.1));
    let n_lead = lead.len();
    let named: Vec<Them> = lead.into_iter().chain(rest.into_iter().take(MAX_NAMED).map(|x| x.0)).collect();
    for (k, who) in named.iter().enumerate() {
        let tag = format!("p{}", keys.people.len());
        let why = if k < n_lead { "a leader of the room" } else { "listening" };
        c.people.push(Tagged { tag: tag.clone(), text: format!("{} ({why})", person(w, *who)) });
        keys.people.push((tag, *who));
        items_of(w, *who, &mut c, &mut keys);
    }
    // A staff room: each coach's recruits, so whoever answers can name
    // them (Cain 2026-11-17).
    for p in &room.present {
        let Them::Coach(co) = p.who else { continue };
        if !afm_engine::world::folk::persona::telling::works_for_you(w, p.who) {
            continue;
        }
        for i in afm_engine::world::folk::persona::coaching::named(w, co) {
            let who = Them::Player(w.people[i].player.id);
            if keys.people.iter().any(|(_, x)| *x == who) {
                continue;
            }
            let tag = format!("p{}", keys.people.len());
            c.people.push(Tagged {
                tag: tag.clone(),
                text: format!("{} (a recruit of {})", person(w, who), person(w, p.who)),
            });
            keys.people.push((tag, who));
        }
    }
    (c, keys)
}

/// One man's items, each named for him.
fn items_of(w: &World, who: Them, c: &mut EarCard, keys: &mut EarKeys) {
    let Them::Player(id) = who else { return };
    let Some(i) = folk::person(w, id) else { return };
    let name = w.people[i].name.clone();
    let mut add = |item: Item, text: String| {
        let tag = format!("c{}", keys.items.len());
        c.items.push(Tagged { tag: tag.clone(), text });
        keys.items.push((tag, item));
    };
    add(Item::Pride, format!("{name}'s pride"));
    let sheet = claims::player(w, i);
    if let Some(p) = sheet.points.iter().find(|p| p.whose != claims::Whose::Yours && tp::talkable(p.kind) && p.known) {
        add(
            Item::Point(p.kind.key().to_string()),
            format!("{name}: on his mind, {}", folk::plain(&tp::worry_words(p))),
        );
    }
    let to = who.promise_to();
    for (k, p) in w.folk.promises.iter().enumerate().filter(|(_, p)| Some(p.to) == to && p.kept != Some(true)) {
        let state = if p.kept == Some(false) { "broken" } else { "open" };
        add(Item::Promise(k as u32), format!("your promise to {name} ({state}): {}", p.kind.words()));
    }
}
