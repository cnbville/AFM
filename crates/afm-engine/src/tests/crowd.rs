//! Group talks as a room (thread "Make group talks feel like a room", the
//! nine items): each man takes a line his own way by his group, leaders
//! pull the room, a promise to the room reaches every man, a man called
//! out takes it harder with his friends, the right man answers, a full room
//! you ask stays quiet and a man who held back asks to see you, the room
//! has its own patience, the read is by groups, and a position room meets.

use super::legacy::seat;
use crate::player::PlayingTime;
use crate::positions::Position;
use crate::world::coach::Group as Pos;
use crate::world::folk::persona::Them;
use crate::world::folk::persona::crowd::{self, Bloc};
use crate::world::folk::persona::ear::{Act, Hit, How, Item, Reading, Resolving, Sized};
use crate::world::folk::persona::gameday::group;
use crate::world::folk::persona::knows::Topic;
use crate::world::folk::persona::room::{Channel, Group};
use crate::world::folk::persona::talk::{Conversation, Parts, Said, store};
use crate::world::folk::scene::{self, Kind, PromiseRead, Scene};
use crate::world::folk::{Team, person, ties};
use crate::world::{Stage, World};

/// Your college program with `n` more players on it, half at receiver and
/// half at linebacker, the first of each a captain, everyone calm.
fn team(n: usize) -> (World, Vec<u32>) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let v: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(n).collect();
    for (k, &i) in v.iter().enumerate() {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.last_role = Some(PlayingTime::Backup);
        q.player.position = if k < n / 2 { Position::WR } else { Position::LB };
        w.programs[p].roster.push(i);
    }
    for i in w.programs[p].roster.clone() {
        w.people[i].mood = 0.0;
        w.people[i].trust = 0.5;
    }
    w.programs[p].captains = vec![v[0], v[n / 2]];
    let ids = v.iter().map(|&i| w.people[i].player.id).collect();
    (w, ids)
}

fn meeting(w: &mut World) -> (u32, crate::world::folk::persona::room::Room) {
    let mut s = Scene::new(Kind::TeamMeeting, w.today, "A team meeting");
    s.team = Team::seat(w);
    let sid = scene::add(w, s);
    let s = w.folk.scenes.iter().find(|x| x.id == sid).unwrap().clone();
    let room = group::room(w, &s).expect("a room");
    store::begin(w, sid, room.clone());
    (sid, room)
}

fn said(sid: u32, words: &str, reading: Reading) -> Conversation<'static> {
    Conversation {
        parts: Parts::today(),
        said: Said { scene: sid, words: words.into(), reading, promises: Vec::new() },
    }
}

#[test]
fn each_group_hears_a_line_its_own_way() {
    let (mut w, ids) = team(8);
    for &id in &ids[1..3] {
        let i = person(&w, id).unwrap();
        w.people[i].last_role = Some(PlayingTime::Starter);
    }
    let (sid, room) = meeting(&mut w);
    // "Every job is open": the starters doubted, the backups reassured.
    let r = Reading {
        hits: vec![
            Hit { on: Bloc::Starters.them(), item: Item::Room, how: How::Doubted },
            Hit { on: Bloc::Backups.them(), item: Item::Room, how: How::Reassured },
        ],
        ..Reading::default()
    };
    let r = crowd::expand(&w, &room, r);
    let starter = Them::Player(ids[1]);
    let backup = Them::Player(ids[5]);
    assert!(r.hits.contains(&Hit { on: starter, item: Bloc::Starters.item(), how: How::Doubted }), "{:?}", r.hits);
    assert!(r.hits.contains(&Hit { on: backup, item: Bloc::Backups.item(), how: How::Reassured }));
    assert!(!r.hits.iter().any(|h| h.item == Item::Room), "a group's reading takes the room's place");
    let conv = said(sid, "Every job is open.", r);
    let t = conv.turn(&w, &room, &conv.said.reading);
    let (st, sm, _) = crowd::take(&t.hits, starter);
    let (bt, bm, _) = crowd::take(&t.hits, backup);
    assert!(st + sm < 0.0 && bt + bm > 0.0, "starter {} backup {}", st + sm, bt + bm);
    // The read names a group, never one captain for the room.
    let read = crowd::read(&w, &room, &t.hits, None);
    assert!(!read.heard.contains("on the team"), "{}", read.heard);
}

#[test]
fn a_line_to_the_defense_is_answered_by_a_defensive_leader() {
    let (mut w, ids) = team(8);
    let (sid, room) = meeting(&mut w);
    let r = crowd::expand(
        &w,
        &room,
        Reading {
            hits: vec![Hit { on: Bloc::Defense.them(), item: Item::Room, how: How::Respected }],
            ..Reading::default()
        },
    );
    let open = store::open(&w, sid).unwrap().clone();
    let v = crowd::voices(&w, w.folk.scenes.iter().find(|s| s.id == sid), &room, &r, &open);
    let first = v.first.expect("someone answers");
    let i = match first {
        Them::Player(id) => person(&w, id).unwrap(),
        _ => panic!("a player"),
    };
    assert!(!Pos::of(w.people[i].player.position).offense(), "{first:?} answered for the defense");
    // To the offense, an offensive man.
    let r = crowd::expand(
        &w,
        &room,
        Reading {
            hits: vec![Hit { on: Bloc::Offense.them(), item: Item::Room, how: How::Respected }],
            ..Reading::default()
        },
    );
    let v = crowd::voices(&w, w.folk.scenes.iter().find(|s| s.id == sid), &room, &r, &open);
    let Some(Them::Player(id)) = v.first else { panic!("a player answers") };
    assert!(Pos::of(w.people[person(&w, id).unwrap()].player.position).offense());
    // Asked about by name, he answers himself.
    let r = Reading {
        acts: vec![Act::Question { topic: Topic::Injury, about: Some(Them::Player(ids[3])), words: String::new() }],
        ..Reading::default()
    };
    let v = crowd::voices(&w, w.folk.scenes.iter().find(|s| s.id == sid), &room, &r, &open);
    assert_eq!(v.first, Some(Them::Player(ids[3])));
    assert!(v.own);
}

#[test]
fn a_full_room_you_ask_stays_quiet_and_a_man_who_held_back_comes_later() {
    let (mut w, ids) = team(8);
    let quiet = person(&w, ids[6]).unwrap();
    w.people[quiet].mood = -0.5;
    {
        let t = &mut w.people[quiet].player.personality.traits;
        (t.ego, t.temper, t.leadership, t.professionalism) = (3, 3, 3, 18);
    }
    let (sid, room) = meeting(&mut w);
    let r = Reading {
        acts: vec![Act::Question { topic: Topic::Nothing, about: None, words: String::new() }],
        ..Reading::default()
    };
    let conv = said(sid, "Anyone have a problem with this?", r);
    let t = conv.turn(&w, &room, &conv.said.reading);
    let c = t.crowd.clone().expect("a room's turn");
    assert!(c.silent, "a full room stays quiet: {c:?}");
    assert!(c.held.contains(&Them::Player(ids[6])), "{c:?}");
    let before = w.folk.scenes.len();
    conv.apply(&mut w, &t);
    let asks = w.folk.scenes[before..].iter().find(|s| s.kind == Kind::OneOnOne && s.a == u64::from(ids[6]));
    assert!(asks.is_some(), "he asks to see you the next day");
}

#[test]
fn a_promise_to_the_room_is_to_every_man_in_it() {
    let (mut w, ids) = team(6);
    let (sid, room) = meeting(&mut w);
    let before = w.folk.promises.len();
    let p = PromiseRead { kind: "role".into(), to: Some(group::ROOM), ..PromiseRead::default() };
    let conv = Conversation {
        parts: Parts::today(),
        said: Said {
            scene: sid,
            words: "Whoever works hardest plays.".into(),
            reading: Reading {
                acts: vec![Act::Promise { words: "Whoever works hardest plays".into(), terms: Vec::new() }],
                ..Reading::default()
            },
            promises: vec![p],
        },
    };
    let t = conv.turn(&w, &room, &conv.said.reading);
    conv.apply(&mut w, &t);
    let to: Vec<_> = w.folk.promises[before..].iter().map(|p| p.to).collect();
    assert!(to.len() > 2, "{to:?}");
    for id in &ids {
        assert!(to.contains(&crate::world::folk::talks::PromiseTo::Player(*id)), "{id} has it");
    }
}

#[test]
fn a_man_called_out_takes_it_harder_and_his_friend_feels_it() {
    let (mut w, ids) = team(6);
    ties::bond(&mut w, ids[1], ids[2], 40);
    let (sid, room) = meeting(&mut w);
    let s = w.folk.scenes.iter().find(|x| x.id == sid).cloned();
    let mut hits = vec![Sized {
        hit: Hit { on: Them::Player(ids[1]), item: Item::Pride, how: How::Stung },
        trust: -0.04,
        mood: -0.02,
        faded: false,
    }];
    crowd::adjust(&w, s.as_ref(), &room, &mut hits);
    assert!((hits[0].trust + 0.04 * crowd::PUBLIC).abs() < 1e-6, "{:?}", hits[0]);
    let friend = crowd::take(&hits, Them::Player(ids[2]));
    assert!(friend.0 < 0.0, "his friend takes a share: {friend:?}");
}

#[test]
fn leaders_pull_the_men_close_to_them() {
    let (mut w, ids) = team(6);
    let cap = ids[0];
    {
        let i = person(&w, cap).unwrap();
        w.people[i].player.personality.traits.leadership = 19;
    }
    ties::bond(&mut w, cap, ids[1], 60);
    let (sid, room) = meeting(&mut w);
    let s = w.folk.scenes.iter().find(|x| x.id == sid).cloned();
    let mut hits = vec![Sized {
        hit: Hit { on: Them::Player(cap), item: Item::Room, how: How::Stung },
        trust: -0.05,
        mood: -0.05,
        faded: false,
    }];
    crowd::adjust(&w, s.as_ref(), &room, &mut hits);
    let (t, m, _) = crowd::take(&hits, Them::Player(ids[1]));
    assert!(t + m < 0.0, "his friend follows the captain's read: {}", t + m);
    assert!(t + m > -0.1 * crowd::PULL * 1.01, "only part of the way");
}

#[test]
fn a_room_is_lost_as_a_room() {
    let (mut w, _) = team(6);
    let (sid, room) = meeting(&mut w);
    if let Some(o) = w.folk.live.persona.open.iter_mut().find(|o| o.scene == sid) {
        o.lines = 2;
        o.patience.0 = 0.0001;
    }
    let conv = said(sid, "Let me tell you about my time at a clinic in 1998.", Reading::default());
    let t = conv.turn(&w, &room, &conv.said.reading);
    assert!(t.ends.is_some(), "{:?}", t.patience);
    assert!(t.say.released.iter().any(|l| l == "You've lost the room."), "{:?}", t.say.released);
    assert!(!t.say.released.iter().any(|l| l.contains("He's done talking")));
}

#[test]
fn a_position_room_meets_with_its_coach() {
    let (mut w, _) = team(6);
    let sid = crowd::start_position_room(&mut w, Pos::Receivers).expect("it opens");
    assert_eq!(crowd::start_position_room(&mut w, Pos::Receivers), Ok(sid), "the same meeting while it's open");
    let s = scene::open(&mut w, sid).expect("it opens");
    let room = group::room(&w, &s).expect("a room");
    assert_eq!(room.channel, Channel::Group(Group::Position(Pos::Receivers as u8)));
    for p in crowd::listeners(&room) {
        let Them::Player(id) = p else { continue };
        let i = person(&w, id).unwrap();
        assert_eq!(Pos::of(w.people[i].player.position), Pos::Receivers);
    }
    assert!(group::is_group(&s));
    // Its groups are the starters and the backups, never the sides.
    assert!(!crowd::blocs(&w, &room).contains(&Bloc::Defense));
}

#[test]
fn a_line_that_calls_out_a_group_goes_to_it() {
    assert_eq!(crowd::addressed("Defense, you were flying around."), Some(Bloc::Defense));
    assert_eq!(crowd::addressed("Alright, seniors: I need you to lead."), Some(Bloc::Seniors));
    assert_eq!(crowd::addressed("O-line, that was a clinic."), Some(Bloc::Room(Pos::OL)));
    assert_eq!(crowd::addressed("The defense played hard and I loved it."), None);
    assert_eq!(crowd::addressed("Anyone have a problem with that?"), None);
    let (mut w, _) = team(8);
    let (sid, room) = meeting(&mut w);
    // The ear read it as to the whole room; the words aim it.
    let r =
        Reading { hits: vec![Hit { on: group::ROOM, item: Item::Room, how: How::Respected }], ..Reading::default() };
    let r = crowd::expand(&w, &room, crowd::aim(&w, &room, "Defense, you were flying around.", r));
    assert!(!r.hits.is_empty());
    for h in &r.hits {
        let Them::Player(id) = h.on else { continue };
        let i = person(&w, id).unwrap();
        assert!(!Pos::of(w.people[i].player.position).offense(), "{h:?} reached the offense");
        assert_eq!(h.item, Bloc::Defense.item());
    }
    // The read names the defense, not its rooms.
    let conv = said(sid, "Defense, you were flying around.", r);
    let t = conv.turn(&w, &room, &conv.said.reading);
    let read = crowd::read(&w, &room, &t.hits, None);
    assert!(read.heard.contains("the defense"), "{}", read.heard);
    assert!(!read.heard.contains("linebackers"), "{}", read.heard);
}

#[test]
fn a_question_put_to_the_room_by_its_tag_is_asking_the_room() {
    let r = Reading {
        acts: vec![Act::Question { topic: Topic::Nothing, about: Some(group::ROOM), words: String::new() }],
        ..Reading::default()
    };
    assert!(crowd::asks_room(&r));
    // Opening the floor, whatever the ear took it to be about.
    let r = Reading {
        acts: vec![Act::Question { topic: Topic::Job, about: None, words: "Anyone have a problem with that?".into() }],
        ..Reading::default()
    };
    assert!(crowd::asks_room(&r));
    // A question about the plan is no open floor.
    let r = Reading {
        acts: vec![Act::Question { topic: Topic::Job, about: None, words: "Who starts at corner?".into() }],
        ..Reading::default()
    };
    assert!(!crowd::asks_room(&r));
}

#[test]
fn a_decision_to_the_room_benches_nobody() {
    let (mut w, _) = team(8);
    let (sid, room) = meeting(&mut w);
    let r = Reading {
        acts: vec![Act::Decision(crate::world::folk::persona::terms::Decision::Benched)],
        ..Reading::default()
    };
    let conv = said(sid, "Every job on this team is open. Nobody is safe.", r);
    let t = conv.turn(&w, &room, &conv.said.reading);
    assert!(t.confirm.is_empty(), "{:?}", t.confirm);
}

#[test]
fn in_front_of_the_team_a_bold_backup_still_keeps_it_for_later() {
    let (mut w, ids) = team(8);
    let loud = person(&w, ids[5]).unwrap();
    w.people[loud].mood = -0.8;
    {
        let t = &mut w.people[loud].player.personality.traits;
        (t.ego, t.temper, t.leadership, t.professionalism) = (19, 19, 6, 3);
    }
    let (sid, room) = meeting(&mut w);
    let r = Reading {
        acts: vec![Act::Question { topic: Topic::Nothing, about: None, words: "Any questions?".into() }],
        ..Reading::default()
    };
    let conv = said(sid, "Any questions?", r);
    let t = conv.turn(&w, &room, &conv.said.reading);
    let c = t.crowd.clone().expect("a room's turn");
    assert!(c.silent, "only a leader stands up in front of the team: {c:?}");
    assert!(c.held.contains(&Them::Player(ids[5])), "{c:?}");
}

#[test]
fn a_hard_word_to_the_whole_room_sends_no_one_out() {
    let (mut w, ids) = team(8);
    let hot = person(&w, ids[3]).unwrap();
    w.people[hot].mood = -0.6;
    {
        let t = &mut w.people[hot].player.personality.traits;
        (t.ego, t.temper, t.professionalism) = (20, 20, 1);
    }
    let (sid, room) = meeting(&mut w);
    let r = crowd::expand(
        &w,
        &room,
        Reading { hits: vec![Hit { on: group::ROOM, item: Item::Room, how: How::Threatened }], ..Reading::default() },
    );
    let conv = said(sid, "Every job on this team is open. Nobody is safe.", r);
    let t = conv.turn(&w, &room, &conv.said.reading);
    let c = t.crowd.clone().expect("a room's turn");
    assert!(c.walked.is_empty(), "{c:?}");
}
