//! Your players talk to you before they enter the portal (thread "Fix mass
//! transfers after coaching change", 2026-10-28): a window doesn't take them
//! on its first day; each asks to see you, and on his day he enters only if
//! his lean is still above the bar his draw set.

use super::legacy::seat;
use crate::player::PlayingTime;
use crate::world::clock::PortalWindow;
use crate::world::recruiting;
use crate::world::{Stage, World};

/// Your program with `n` sophomore backups who all want out.
fn leaning_room(n: usize) -> (World, usize, Vec<usize>) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let room: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(n).collect();
    for &i in &room {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.last_role = Some(PlayingTime::Backup);
        q.cal.lean = 0.9;
        w.programs[p].roster.push(i);
    }
    (w, p, room)
}

#[test]
fn your_players_ask_to_see_you_and_the_talk_decides_who_goes() {
    let (mut w, p, room) = leaning_room(6);
    let close = w.today.add(15);
    recruiting::portal_open(&mut w, PortalWindow::Winter, close);
    // Nobody is in yet: each asked to see you.
    for &i in &room {
        assert!(w.people[i].cal.portal.is_none(), "{} entered before his talk", w.people[i].name);
        let id = w.people[i].player.id as u64;
        let talk = w.folk.scenes.iter().find(|s| s.a == id && !s.done).expect("a talk on your calendar");
        assert!(talk.title.ends_with("wants to talk about his future"), "{}", talk.title);
    }
    assert_eq!(w.portal_intents.len(), room.len());
    // Three of them heard what they needed; three didn't.
    for &i in &room[..3] {
        w.people[i].cal.lean = 0.0;
    }
    w.today = close.add(1);
    recruiting::intents_day(&mut w);
    assert!(w.portal_intents.is_empty());
    for &i in &room[..3] {
        let q = &w.people[i];
        assert!(q.cal.portal.is_none() && q.college == Some(p), "{} left after a good talk", q.name);
        assert!(q.cal.lean_events.iter().any(|e| e.1 == "decided to stay after talking it over"));
    }
    for &i in &room[3..] {
        assert!(w.people[i].cal.portal.is_some(), "{} still wanted out and stayed", w.people[i].name);
    }
    // Once they decide where to go, their reasons to leave are spent.
    recruiting::portal_day(&mut w, true);
    for &i in &room[3..] {
        assert!(w.people[i].cal.portal.is_none());
        assert_eq!(w.people[i].cal.lean, 0.0, "{} could be drawn again", w.people[i].name);
    }
}

#[test]
fn a_talk_you_hand_to_staff_is_his_position_coachs_word() {
    let (mut w, _, room) = leaning_room(1);
    let close = w.today.add(15);
    recruiting::portal_open(&mut w, PortalWindow::Winter, close);
    let i = room[0];
    let id = w.people[i].player.id as u64;
    let talk = w.folk.scenes.iter().find(|s| s.a == id && !s.done).map(|s| s.id).unwrap();
    crate::world::folk::scene::hand(&mut w, talk).unwrap();
    assert!(w.people[i].cal.lean_events.iter().any(|e| e.1 == "his position coach asked him to stay"));
}

#[test]
fn he_opens_by_saying_why_he_came() {
    // Minor bug 155: the talk he asked for opens with one of his own portal
    // lines (his mood, his reason), not a mood part that assumes you called
    // him in, and not the one fallback sentence every player shared.
    let (mut w, _, room) = leaning_room(1);
    let close = w.today.add(15);
    recruiting::portal_open(&mut w, PortalWindow::Winter, close);
    let i = room[0];
    let id = w.people[i].player.id as u64;
    let talk = w.folk.scenes.iter().find(|s| s.a == id && !s.done).map(|s| s.id).unwrap();
    let s = crate::world::folk::scene::open(&mut w, talk).unwrap();
    let name = w.people[i].name.clone();
    let said = s.lines.iter().find(|l| l.name == name).map(|l| l.text.clone()).expect("he speaks first");
    let b = crate::world::folk::voice::bank();
    let ours = ["sour", "good", "flat"]
        .iter()
        .any(|m| b.versions(&format!("talk.open.portal.{m}")).iter().any(|v| could_say(v.0, &said)));
    assert!(ours, "his opening isn't a portal line: {said}");
}

/// Whether a line as said could be this version: one wording of each
/// `[a|b]` choice, and each `{fact}` filled with anything.
fn could_say(version: &str, said: &str) -> bool {
    let mut forms = vec![String::new()];
    let mut rest = version;
    while let (Some(a), Some(z)) = (rest.find('['), rest.find(']')) {
        let (head, opts) = (&rest[..a], &rest[a + 1..z]);
        forms = forms.iter().flat_map(|f| opts.split('|').map(move |o| format!("{f}{head}{o}"))).collect();
        rest = &rest[z + 1..];
    }
    let said = said.to_lowercase();
    forms.iter().any(|f| {
        let f = format!("{f}{rest}").to_lowercase();
        let lits: Vec<&str> = f
            .split('{')
            .enumerate()
            .map(|(k, x)| if k == 0 { x } else { x.split_once('}').map_or(x, |y| y.1) })
            .collect();
        let (first, last) = (lits[0], lits[lits.len() - 1]);
        if lits.len() == 1 {
            return said == first;
        }
        if !said.starts_with(first) || !said.ends_with(last) || said.len() < first.len() + last.len() {
            return false;
        }
        let mut at = first.len();
        for l in &lits[1..lits.len() - 1] {
            match said[at..].find(l) {
                Some(p) => at += p + l.len(),
                None => return false,
            }
        }
        at + last.len() <= said.len()
    })
}

#[test]
fn a_said_line_is_read_back_to_its_version() {
    assert!(could_say(
        "I'm thinking about walking away from this {team} team.",
        "I'm thinking about walking away from this Air Force team."
    ));
    assert!(could_say("[Look|Listen], {coach}. I'm out.", "Listen, Coach. I'm out."));
    assert!(could_say("I want to stay.", "I want to stay."));
    assert!(!could_say("I want to stay.", "I want to go."));
    assert!(!could_say("Leaving {team} is hard.", "Staying at Air Force is hard."));
}

#[test]
fn a_player_who_enters_the_portal_leaves_your_nil_budget() {
    // Minor bug 375: `enter_portal` ended his NIL income but his deal stayed
    // in the desk, so the money you could promise was short by it while he
    // shopped. Entering ends his deals as being cut does, and an old offer
    // of yours to him (a transfer you brought in) doesn't give him one again.
    let (mut w, p, room) = leaning_room(2);
    crate::world::front_office::college::ready(&mut w);
    let ids: Vec<u32> = room.iter().map(|&i| w.people[i].player.id).collect();
    // The desk seeded the roster it inherited; these two have these deals.
    w.front_office.college.deals.retain(|d| !ids.contains(&d.0));
    w.front_office.college.coll_deals.retain(|d| !ids.contains(&d.0));
    for &id in &ids {
        w.front_office.college.deals.push((id, 25_000.0));
        w.front_office.college.coll_deals.push((id, 10_000.0));
    }
    // The second came to you through the portal last year, on an offer that is settled.
    let today = w.today;
    w.front_office.college.portal.push(crate::world::front_office::college::PortalTarget {
        id: ids[1],
        added: today.add(-300),
        offered: Some(today.add(-290)),
        nil: Some(200_000.0),
        nil_coll: Some(100_000.0),
        contacted: None,
        leaked: false,
        from: if p == 0 { 1 } else { 0 },
        settled: true,
        talks: Vec::new(),
        visits: Vec::new(),
        feel: None,
        promise: None,
        committed: None,
        called: None,
        case: Default::default(),
    });
    let before = crate::world::front_office::college::budget(&w, p);
    let close = w.today.add(15);
    recruiting::portal_open(&mut w, PortalWindow::Winter, close);
    // They asked to see you first: still yours, deals stand.
    assert_eq!(w.front_office.college.deals.iter().filter(|d| ids.contains(&d.0)).count(), 2);
    // One is talked into staying; the other still wants out.
    w.people[room[0]].cal.lean = 0.0;
    w.today = close.add(1);
    recruiting::intents_day(&mut w);
    assert!(w.people[room[0]].cal.portal.is_none() && w.people[room[1]].cal.portal.is_some());
    let has = |w: &World, id: u32| {
        (
            w.front_office.college.deals.iter().any(|d| d.0 == id),
            w.front_office.college.coll_deals.iter().any(|d| d.0 == id),
        )
    };
    assert_eq!(has(&w, ids[0]), (true, true), "he stayed, so his deals stand");
    assert_eq!(has(&w, ids[1]), (false, false), "he is in the portal, so his deals are gone");
    let after = crate::world::front_office::college::budget(&w, p);
    assert!(
        (before.roster - after.roster - 25_000.0).abs() < 1.0,
        "your budget no longer counts his deal: {} then {}",
        before.roster,
        after.roster
    );
    assert!(
        (before.coll_roster - after.coll_roster) > 0.0 || before.coll_roster == 0.0,
        "nor his collective deal: {} then {}",
        before.coll_roster,
        after.coll_roster
    );
    // The desk's morning run doesn't hand it back from his old offer.
    crate::world::front_office::college::fill_deals(&mut w);
    assert_eq!(has(&w, ids[1]), (false, false), "his old offer gives him no new deal while he shops");
    let again = crate::world::front_office::college::budget(&w, p);
    assert!((again.roster - after.roster).abs() < 1.0, "{} then {}", after.roster, again.roster);
}
