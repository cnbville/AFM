//! Live talks v3, lane N5e: scouting and the draft (`world::folk::persona::
//! draft`). These prove that the whiteboard reads a prospect's football
//! mind into your file and costs the room's clock, that a coached answer
//! shades your read until you press it, that the people you send find
//! things out on their own days and answer from their own looks, that a
//! prospect's word of you follows him into the rush and onto your roster,
//! that your staff's smoke read never reads the truth, and that smoke told
//! to one GM reaches only his club.

use super::legacy::seat;
use crate::world::folk::persona::draft::{self, Sub, coached, facet, remember, scouts, smoke, whiteboard};
use crate::world::folk::persona::knows::Topic;
use crate::world::folk::persona::terms::Request;
use crate::world::folk::persona::wants::Willing;
use crate::world::folk::persona::{Them, makeup};
use crate::world::folk::scene::{self, Kind, Scene, Who};
use crate::world::scouting::own::Topic as Room;
use crate::world::scouting::readers::{self, Staff};
use crate::world::scouting::{self as sc, Kind as LookKind, predraft};
use crate::world::{Stage, World};

/// Your NFL club and a declared prospect your area scout has seen.
fn club() -> (World, usize, u32) {
    let mut w = seat("nfl");
    let t = draft::club(&w).expect("an NFL seat");
    let year = w.year;
    crate::world::pro::scouts::staff(&mut w, false, year);
    assert!(!w.clubs[t].scouts.is_empty());
    let i = w.people.iter().position(|p| p.stage == Stage::Pro).expect("a pro to make a prospect");
    let p = &mut w.people[i];
    p.stage = Stage::Declared;
    p.pro = None;
    let id = p.player.id;
    let d = w.today;
    sc::dept::add(&mut w, id, 0, LookKind::School, d.add(-20));
    (w, t, id)
}

/// An interview room open with him today, `minutes` long.
fn room(w: &mut World, id: u32, minutes: i64) -> u32 {
    let mut s = Scene::new(Kind::CombineTalk, w.today, "Combine interview: him");
    s.stops = false;
    s.a = id as u64;
    s.c = minutes;
    s.f[0] = minutes as f32;
    s.who = vec![Who::Player(id)];
    s.outcome.clear();
    scene::add(w, s)
}

fn sd(w: &World, t: usize, id: u32, a: crate::attributes::Attr) -> Option<f64> {
    let (_, _, f) = readers::read(w, Staff::Club(t), id)?;
    let r = &f.attrs[a.index()];
    r.seen.then_some(r.sd)
}

#[test]
fn the_whiteboard_reads_his_football_mind_and_costs_the_clock() {
    use crate::attributes::Attr;
    let (mut w, t, id) = club();
    let parts = facet::parts();
    // Only a man in your room goes to the board.
    assert_eq!(parts.acting.willing(&w, Them::Player(id), &Request::Whiteboard), Willing::No);
    let s = room(&mut w, id, 30);
    assert!(facet::in_room(&w, id));
    assert_eq!(parts.acting.willing(&w, Them::Player(id), &Request::Whiteboard), Willing::Yes);
    let before = sd(&w, t, id, Attr::FootballIq);
    let ran = parts.acting.run(&mut w, Them::Player(id), &Request::Whiteboard);
    assert!(ran.done.len() == 2, "what he had, and how he put it across: {:?}", ran.done);
    let sc = w.folk.scenes.iter().find(|x| x.id == s).unwrap();
    // Six minutes of an 18-minute room, scaled to this 30-minute one.
    let m = (whiteboard::MINUTES * 30.0 / 18.0).round() as i64;
    assert_eq!(sc.c, 30 - m, "the session's minutes come off the clock");
    assert_eq!(w.folk.live.persona.draft.boards.len(), 1);
    // The board is a read with its own spread, never the true number: it
    // tightens the file's read of his football IQ.
    let after = sd(&w, t, id, Attr::FootballIq).expect("the whiteboard reads football IQ");
    assert!(before.is_none_or(|b| after < b), "{before:?} -> {after}");
    let (_, _, f) = readers::read(&w, Staff::Club(t), id).unwrap();
    let i = sc::place(&w, id).unwrap();
    let truth = w.people[i].player.attributes.get(Attr::FootballIq);
    assert!(f.attrs[Attr::FootballIq.index()].centre != truth, "a read, not the truth");
    // No time left, no session.
    let s2 = w.folk.scenes.iter_mut().find(|x| x.id == s).unwrap();
    s2.c = 2;
    let ran = parts.acting.run(&mut w, Them::Player(id), &Request::Whiteboard);
    assert!(ran.done[0].contains("isn't time"), "{:?}", ran.done);
    assert_eq!(w.folk.live.persona.draft.boards.len(), 1);
}

#[test]
fn a_coached_answer_shades_the_read_until_you_press_it() {
    let (mut w, _, id) = club();
    let i = sc::place(&w, id).unwrap();
    // A declared prospect has been prepped; the whiteboard can't be.
    assert!(coached::prep(&w, i) >= 0.3, "{}", coached::prep(&w, i));
    let d = w.today;
    let first = coached::shade(&w, id, d, Room::Family);
    assert!(first > 0.0);
    assert_eq!(coached::shade(&w, id, d, Room::Whiteboard), 0.0);
    // Each follow-up peels half the script off.
    coached::press(&mut w, id, Room::Family);
    let pressed = coached::shade(&w, id, d, Room::Family);
    assert!((pressed - first * coached::PEEL).abs() < 1e-9, "{first} -> {pressed}");
    // The press is kept for that room's day only.
    assert_eq!(coached::shade(&w, id, d.add(1), Room::Family), first);
    // A recruit on a visit is barely coached.
    w.people[i].stage = Stage::HighSchool;
    assert!(coached::prep(&w, i) <= 0.3);
}

#[test]
fn the_people_you_send_find_out_on_their_own_days() {
    let (mut w, t, id) = club();
    let parts = facet::parts();
    let scout = draft::scout(t, 0);
    let sec = draft::security(t);
    assert!(scouts::yours(&w, scout) && scouts::yours(&w, sec));
    // Another club's GM is not yours to send.
    let gm = draft::gm(&w, (t + 1) % w.nfl.len());
    assert!(!scouts::yours(&w, gm));
    let ask = Request::FindOut(id, Topic::Character);
    assert_eq!(parts.acting.willing(&w, scout, &ask), Willing::Yes);
    let ran = parts.acting.run(&mut w, scout, &ask);
    assert!(ran.done[0].contains("days"), "{:?}", ran.done);
    // Asking twice keeps one question.
    parts.acting.run(&mut w, scout, &ask);
    assert_eq!(w.folk.live.persona.draft.questions.len(), 1);
    // Security runs a background check before it has anything to say.
    let words = scouts::security_words(&w, Staff::Club(t), id);
    assert!(words[0].contains("haven't run"), "{words:?}");
    parts.acting.run(&mut w, sec, &Request::FindOut(id, Topic::Case));
    let q = w.folk.live.persona.draft.questions[1];
    assert_eq!(q.kind, LookKind::Background);
    // Nothing lands before its day; on it, each is a look in your file
    // under his own name.
    let looks = |w: &World| w.scouting.dept.looks.get(&id).map_or(0, |v| v.len());
    let n = looks(&w);
    let d = w.today;
    w.today = d.add(1);
    draft::day(&mut w, d.add(1));
    assert_eq!(looks(&w), n);
    w.today = d.add(scouts::SCHOOL_DAYS);
    draft::day(&mut w, d.add(scouts::SCHOOL_DAYS));
    assert_eq!(looks(&w), n + 2);
    let codes: Vec<u32> = w.scouting.dept.looks[&id].iter().map(|l| l.who).collect();
    assert!(codes.contains(&readers::SECURITY), "{codes:?}");
    assert!(w.folk.live.persona.draft.questions.iter().all(|q| q.done));
    let words = scouts::security_words(&w, Staff::Club(t), id);
    assert!(!words[0].contains("haven't run"), "{words:?}");
}

#[test]
fn his_word_of_you_follows_him_into_the_rush_and_onto_your_roster() {
    let (mut w, t, id) = club();
    let i = sc::place(&w, id).unwrap();
    assert_eq!(remember::rush_pull(&w, i, t), 0.0, "a man you never talked to: the world is unchanged");
    w.folk.mine.set_trust(makeup::key(Them::Player(id)), 0.9);
    assert!(remember::rush_pull(&w, i, t) > 0.0);
    assert_eq!(remember::rush_pull(&w, i, (t + 1) % w.nfl.len()), 0.0, "only your club's call");
    // A talk reads his word of you, not his trust in his own coach (N4's
    // reader for someone you don't coach).
    w.people[i].trust = 0.1;
    let read = crate::world::folk::persona::talk::trust_of(&w, Them::Player(id));
    assert!((read - 0.9).abs() < 1e-6, "{read}");
    // He joins you: half the way from where he stands with his new coach
    // to his word of you, once.
    let you = readers::you(&w).unwrap() as u32;
    w.people[i].stage = Stage::Pro;
    w.people[i].trust_in = you;
    w.people[i].trust = 0.5;
    w.clubs[t].roster.push(i);
    remember::week(&mut w);
    assert!((w.people[i].trust - 0.7).abs() < 1e-6, "{}", w.people[i].trust);
    remember::week(&mut w);
    assert!((w.people[i].trust - 0.7).abs() < 1e-6, "once");
}

#[test]
fn your_staffs_smoke_read_never_reads_the_truth() {
    let (mut w, t, id) = club();
    let other = (t + 1) % w.nfl.len();
    let d = w.today;
    let leak = |real| predraft::Leak { team: other, player: id, date: d, real, yours: false, to: None };
    w.scouting.predraft.leaks.push(leak(true));
    let a = smoke::read(&w, other, id, false);
    w.scouting.predraft.leaks[0] = leak(false);
    let b = smoke::read(&w, other, id, false);
    assert_eq!(a.odds, b.odds, "the same read whether it was real or not");
    assert!(!a.why.is_empty() && !a.band.is_empty());
    assert!((0.0..=1.0).contains(&a.odds));
}

#[test]
fn smoke_told_to_one_gm_reaches_only_his_club() {
    let (mut w, t, id) = club();
    let n = w.nfl.len();
    let (told, deaf) = ((t + 1) % n, (t + 2) % n);
    let parts = facet::parts();
    let gm = draft::gm(&w, told);
    assert_eq!(parts.acting.willing(&w, gm, &Request::Plant(id)), Willing::Yes);
    let ran = parts.acting.run(&mut w, gm, &Request::Plant(id));
    assert!(!ran.done.is_empty());
    let l = w.scouting.predraft.leaks.last().unwrap();
    assert_eq!((l.team, l.to, l.real, l.yours), (t, Some(told), false, true));
    // Your club picks right behind both of them.
    let slot = |x: usize| Some(if x == t { 5 } else { 0 });
    assert!(predraft::leak_pull(&w, told, id, &slot, 0) > 0.0, "the club you told hears it");
    assert_eq!(predraft::leak_pull(&w, deaf, id, &slot, 0), 0.0, "no other club does");
}

#[test]
fn the_draft_side_talks_open_with_picks_that_do_things() {
    let (mut w, t, id) = club();
    // Your security director: what he has, and sending him.
    let s = draft::scene::start(&mut w, Sub::Security, id).unwrap();
    let open = scene::open(&mut w, s).unwrap();
    assert!(open.picks.iter().any(|p| p.code == 812), "{:?}", open.picks);
    let after = scene::answer(&mut w, s, 812).unwrap();
    assert!(!after.done);
    assert_eq!(w.folk.live.persona.draft.questions.len(), 1);
    let done = scene::answer(&mut w, s, 819).unwrap();
    assert!(done.done);
    // His family, once a year, only while he's a prospect.
    let f = draft::scene::start(&mut w, Sub::Family, id).unwrap();
    scene::open(&mut w, f).unwrap();
    scene::answer(&mut w, f, 801).unwrap();
    assert!(draft::scene::start(&mut w, Sub::Family, id).is_err(), "once a year");
    // The draft room needs you on the clock.
    assert!(draft::scene::start(&mut w, Sub::Room, id).is_err());
    // A scout tells you what he has on him from his own look.
    let sc0 = draft::scene::start(&mut w, Sub::Scout(0), id).unwrap();
    scene::open(&mut w, sc0).unwrap();
    let said = scene::answer(&mut w, sc0, 821).unwrap();
    assert!(said.lines.iter().any(|l| l.by != 1 && !l.text.is_empty()));
    let _ = t;
}
