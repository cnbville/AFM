//! Live talks v3, lane N5d: game day and the room. A line to the whole
//! room reaches every man in it; the council meets once a week and tells
//! you what it sees; your speech's touches count only on real things; an
//! official warns before he flags an argument; a decision you confirm in a
//! talk is made real.

use super::legacy::seat;
use crate::player::PlayingTime;
use crate::positions::Position;
use crate::world::folk::persona::Them;
use crate::world::folk::persona::ear::{Hit, How, Item, Reading};
use crate::world::folk::persona::gameday::{decide, group, official, speech};
use crate::world::folk::persona::room::{Channel, Group};
use crate::world::folk::persona::terms::Decision;
use crate::world::folk::scene::{self, Kind, Scene};
use crate::world::folk::{Team, rules};
use crate::world::{Stage, World};

/// Your college program with `n` more players on it.
fn team(n: usize) -> (World, Vec<u32>) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let v: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(n).collect();
    for &i in &v {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.last_role = Some(PlayingTime::Backup);
        q.player.position = Position::WR;
        q.mood = 0.0;
        w.programs[p].roster.push(i);
    }
    let ids = v.iter().map(|&i| w.people[i].player.id).collect();
    (w, ids)
}

#[test]
fn a_line_to_the_room_reaches_every_man_in_it() {
    let (w, ids) = team(6);
    let mut s = Scene::new(Kind::TeamTalk, w.today, "A team talk");
    s.team = Team::seat(&w);
    let room = group::room(&w, &s).expect("a room");
    assert_eq!(room.channel, Channel::Group(Group::Team));
    let roster = Team::seat(&w).unwrap().roster(&w);
    assert_eq!(room.present.len(), roster.len(), "the whole roster, once each");
    // The captains come first.
    let leads = group::leads(&w, &s);
    for (k, l) in leads.iter().enumerate() {
        assert_eq!(room.present[k].who, *l);
    }
    // A hit on the room is one hit on each man; a hit on one man stays his.
    let one = Them::Player(ids[0]);
    let r = Reading {
        hits: vec![
            Hit { on: group::ROOM, item: Item::Pride, how: How::Reassured },
            Hit { on: one, item: Item::Pride, how: How::Stung },
        ],
        ..Reading::default()
    };
    let out = crate::world::folk::persona::crowd::expand(&w, &room, r);
    assert_eq!(out.hits.iter().filter(|h| h.item == Item::Room).count(), room.present.len());
    assert!(out.hits.iter().all(|h| h.on != group::ROOM));
    assert!(out.hits.contains(&Hit { on: one, item: Item::Pride, how: How::Stung }));
    // A one-on-one isn't a group talk.
    assert!(group::room(&w, &Scene::new(Kind::OneOnOne, w.today, "A talk")).is_none());
}

#[test]
fn the_council_meets_once_a_week_and_tells_you_what_it_sees() {
    let (mut w, ids) = team(8);
    w.folk.rules.council = ids[..2].to_vec();
    let low = crate::world::folk::person(&w, ids[5]).unwrap();
    w.people[low].mood = -0.7;
    let sid = group::start_council(&mut w).expect("it opens");
    assert_eq!(group::start_council(&mut w), Ok(sid), "the open meeting, not a second one");
    let s = scene::open(&mut w, sid).expect("it opens");
    assert!(s.picks.iter().any(|p| p.code == 600) && s.picks.iter().any(|p| p.code == 601));
    let n = s.lines.len();
    let s = scene::answer(&mut w, sid, 600).expect("asked");
    assert!(s.lines.len() > n + 1, "your line and their answers");
    assert!(!s.picks.iter().any(|p| p.code == 600), "asked once");
    let s = scene::answer(&mut w, sid, 601).expect("ended");
    assert!(s.done);
    assert_eq!(group::start_council(&mut w), Err("You met the council this week.".into()));
    // Who it talks about: the unhappiest men off the council.
    let read = group::room_read(&w, Team::seat(&w).unwrap());
    assert!(!read.is_empty() && read.len() <= group::ASK_ABOUT);
}

#[test]
fn a_speech_counts_only_what_is_real_of_his() {
    let (w, ids) = team(3);
    let id = ids[0];
    let t = speech::tilts(
        &w,
        crate::game::talk::Tone::Calm,
        &[
            (id, How::Stung, Item::Pride),
            (id, How::Reassured, Item::Point("no_such_point".into())),
            (id, How::Reassured, Item::Promise(9999)),
            (ids[1], How::Insulted, Item::Pride),
        ],
    );
    assert_eq!(t.len(), 2, "{t:?}");
    let a = t.iter().find(|x| x.id == id).unwrap();
    assert_eq!(a.pick.mv, crate::game::talk::Move::Criticize);
    assert!((0.2..=speech::MAX_SHARE).contains(&a.share), "{}", a.share);
    let b = t.iter().find(|x| x.id == ids[1]).unwrap();
    assert_eq!(b.pick.tone, crate::game::talk::Tone::Fiery, "an insult is never calm");
    // Two real touches take more of him than one, never past the most.
    let two = speech::tilts(
        &w,
        crate::game::talk::Tone::Calm,
        &[(id, How::Stung, Item::Pride), (id, How::Reassured, Item::HisWord)],
    );
    assert!(two[0].share > a.share && two[0].share <= speech::MAX_SHARE);
    // A sideline word: all of the plain word, more when it touched his.
    let plain = speech::word_share(&w, id, None);
    assert_eq!(plain, speech::PLAIN_SHARE);
    let real = speech::word_share(&w, id, Some(&Item::Pride));
    assert!(real > plain && real <= 1.0);
    assert_eq!(speech::word_share(&w, id, Some(&Item::Promise(9999))), plain);
}

#[test]
fn an_official_warns_before_he_flags_an_argument() {
    use official::{Ruling, Went, flag_odds, rule};
    assert_eq!(flag_odds(Went::Ask, 3, 1.5), 0.0, "asking never draws a flag");
    assert_eq!(flag_odds(Went::Argue, 0, 1.5), 0.0, "a warning first");
    assert!(flag_odds(Went::Argue, 1, 1.0) > 0.0);
    assert!(flag_odds(Went::Abuse, 0, 1.0) > flag_odds(Went::Argue, 1, 1.0));
    assert!(flag_odds(Went::Abuse, 1, 1.0) > flag_odds(Went::Abuse, 0, 1.0));
    assert!(flag_odds(Went::Abuse, 1, 1.4) > flag_odds(Went::Abuse, 1, 0.7), "a tight crew flags more");
    assert!(flag_odds(Went::Abuse, 5, 9.0) <= 0.95);
    assert_eq!(rule(Went::Argue, 0, 1.0, 0.0), Ruling::Warns);
    assert_eq!(rule(Went::Ask, 0, 1.0, 0.0), Ruling::Answers);
    assert_eq!(rule(Went::Abuse, 0, 1.0, 0.1), Ruling::Flags);
    assert_eq!(rule(Went::Abuse, 0, 1.0, 0.99), Ruling::Warns);
    // The ear's reading: doubt is an argument, an insult abuse.
    let r = |how| Reading { hits: vec![Hit { on: Them::Player(1), item: Item::HisWord, how }], ..Reading::default() };
    assert_eq!(official::went(&Reading::default()), Went::Ask);
    assert_eq!(official::went(&r(How::Doubted)), Went::Argue);
    assert_eq!(official::went(&r(How::Insulted)), Went::Abuse);
}

#[test]
fn a_suspension_you_confirm_sits_him_once() {
    let (mut w, ids) = team(2);
    let him = Them::Player(ids[0]);
    let said = decide::run(&mut w, him, Decision::Suspended).expect("this lane's");
    assert!(said[0].contains("sits your next game"), "{said:?}");
    let team = Team::seat(&w).unwrap();
    assert!(rules::sitting(&w, ids[0], rules::after_games(&w, team, 1)));
    let again = decide::run(&mut w, him, Decision::Suspended).unwrap();
    assert!(again[0].contains("already"), "{again:?}");
    assert_eq!(w.folk.out.iter().filter(|o| o.0 == ids[0]).count(), 1);
    // The rest belong to their own lanes.
    assert!(decide::run(&mut w, him, Decision::Cut).is_none());
}

/// Put to the room, an ask about a man by name is the man's who answers to
/// take on himself (a captain to mentor the freshman: it runs, and "He'll
/// do it."), and one that names nobody is the room's ("They'll do it.", and
/// nothing of his own runs).
#[test]
fn an_ask_put_to_the_room_runs_only_when_it_names_a_man() {
    use crate::world::folk::persona::asks;
    use crate::world::folk::persona::ear::{Act, Resolving};
    use crate::world::folk::persona::talk::{Conversation, Parts, Said, store};
    use crate::world::folk::persona::terms::Request;
    use crate::world::folk::persona::wants::Willing;
    use crate::world::folk::{person, ties};
    let (mut w, ids) = team(8);
    w.folk.rules.council = ids[..2].to_vec();
    let (cap, kid) = (ids[0], ids[7]);
    let (ci, ki) = (person(&w, cap).unwrap(), person(&w, kid).unwrap());
    (w.people[ci].player.age, w.people[ki].player.age) = (22, 18);
    w.people[ci].trust = 0.9;
    {
        let t = &mut w.people[ci].player.personality.traits;
        (t.leadership, t.professionalism, t.ego) = (18, 15, 8);
    }
    w.people[ci].player.personality.work_ethic = 16.0;
    let sid = group::start_council(&mut w).expect("it opens");
    let s = scene::open(&mut w, sid).expect("it opens");
    let room = group::room(&w, &s).expect("a room");
    assert_eq!(room.channel, Channel::Group(Group::Council));
    assert_eq!(room.present[0].who, Them::Player(cap), "the council's first man answers for it");
    store::begin(&mut w, sid, room.clone());
    assert!(asks::names_a_man(&Request::Mentor(kid)) && !asks::names_a_man(&Request::Class));
    let reading = Reading { acts: vec![Act::Request(Request::Mentor(kid))], ..Reading::default() };
    let conv = Conversation {
        parts: Parts::today(),
        said: Said { scene: sid, words: "Take the freshman under your wing.".into(), reading, promises: Vec::new() },
    };
    let t = conv.turn(&w, &room, &conv.said.reading);
    assert_eq!(t.asked[0].1, Willing::Yes, "{:?}", t.asked);
    assert!(t.say.released.iter().any(|l| l == "He'll do it."), "{:?}", t.say.released);
    conv.apply(&mut w, &t);
    assert_eq!(ties::mentor_of(&w, kid), Some(cap), "the captain who answered took him on");
    // A room ask: the room takes it on, and the man who answers runs none of
    // it as his own.
    let reading = Reading { acts: vec![Act::Request(Request::Class)], ..Reading::default() };
    let conv = Conversation {
        parts: Parts::today(),
        said: Said { scene: sid, words: "Every one of you goes to class.".into(), reading, promises: Vec::new() },
    };
    let t = conv.turn(&w, &room, &conv.said.reading);
    assert_eq!(t.asked[0].1, Willing::Yes, "{:?}", t.asked);
    assert!(t.say.released.iter().any(|l| l == "They'll do it."), "{:?}", t.say.released);
    assert!(t.confirm.is_empty(), "{:?}", t.confirm);
    conv.apply(&mut w, &t);
    let s = w.folk.scenes.iter().find(|x| x.id == sid).unwrap();
    assert!(!s.lines.iter().any(|l| l.text.contains("agreed to go to class")), "{:?}", s.lines);
}
