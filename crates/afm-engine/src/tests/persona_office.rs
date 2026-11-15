//! Live talks v3, lane N5c: staff, bosses and money
//! (`world::folk::persona::office`). The proofs: the team doctor reads a
//! hurt man's chart and never clears him when the chip wouldn't; a
//! reference call puts his read on the coach's page and you owe him one;
//! a line to someone in your building moves the trust the world keeps for
//! him, held inside the chips; your boss says yes in words exactly when
//! his meeting would, with the same effect; a coordinator moves the plan
//! by the staff meeting's own step; and a sit-down with no key plays as
//! chips on the same conversation and ends when you end it.

use super::legacy::seat;
use crate::calendar::Date;
use crate::player::PlayingTime;
use crate::training::seat::Hurt;
use crate::world::coach::Role;
use crate::world::folk::boss::{self, AskKind};
use crate::world::folk::cast::{self, CastRef, Role as CastRole};
use crate::world::folk::coach_look;
use crate::world::folk::persona::Them;
use crate::world::folk::persona::ear::{Act, Hit, How, Item, Reading, Resolving, Sized, Turn};
use crate::world::folk::persona::knows::{Answer, Told, Topic};
use crate::world::folk::persona::office::{self, asks, doctor, refs, sit};
use crate::world::folk::persona::talk::{Conversation, Said, store};
use crate::world::folk::persona::terms::Request;
use crate::world::folk::persona::wants::Willing;
use crate::world::folk::{Team, scene};
use crate::world::{Stage, World};

/// A college world with your program and one of your players hurt for
/// three weeks.
fn hurt() -> (World, Team, u32) {
    let mut w = seat("college");
    w.add_class_aged(19);
    let team = Team::seat(&w).unwrap();
    let p = w.seat.and_then(|s| s.program).unwrap();
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let year = w.year;
    let q = &mut w.people[i];
    q.stage = Stage::College;
    q.college = Some(p);
    q.class_of = year - 1;
    q.college_seasons = 1;
    q.last_role = Some(PlayingTime::Starter);
    w.programs[p].roster.push(i);
    let id = w.people[i].player.id;
    let today = w.today;
    w.training.hurt.push(Hurt { id, kind: None, from: today, until: Date(today.0 + 21), how: "practice".into() });
    (w, team, id)
}

fn trainer(w: &World, team: Team) -> Them {
    office::cast_them(w, CastRef::new(team, CastRole::Trainer, 0))
}

#[test]
fn the_doctor_reads_the_chart_and_never_clears_a_man_the_chip_wouldnt() {
    let (mut w, team, id) = hurt();
    let doc = trainer(&w, team);
    assert!(doctor::is_doctor(&w, doc));
    let k = doctor::knows(&w, doc, Topic::Injury, Some(Them::Player(id)));
    assert_eq!(k.len(), 1, "he knows the man's chart");
    assert!(k[0].words.contains("out"), "{}", k[0].words);
    assert!(k[0].day.is_some(), "the day he's back is on it");
    // Words never clear a man the chip wouldn't: a staff with good
    // judgment never rushes him.
    let clear = asks::willing(&w, doc, &Request::Clear(id)).unwrap();
    if !crate::world::folk::life::would_rush(&w, team) {
        assert_eq!(clear, Willing::No);
        assert_eq!(doctor::clear_score(&w, doc, team), 0.0);
    } else {
        let yes = doctor::clear_score(&w, doc, team) >= doctor::CLEARS;
        assert_eq!(clear == Willing::Yes, yes);
    }
    // A second opinion runs the meeting's own: his trust in you grows.
    assert_eq!(asks::willing(&w, doc, &Request::SecondOpinion(id)), Some(Willing::Yes));
    let i = crate::world::folk::person(&w, id).unwrap();
    let before = w.people[i].trust;
    let ran = office::parts().acting.run(&mut w, doc, &Request::SecondOpinion(id));
    assert!(!ran.done.is_empty());
    assert!(w.people[i].trust > before);
}

#[test]
fn a_reference_call_puts_his_read_on_the_coachs_page_and_you_owe_him() {
    let mut w = seat("college");
    let found = (0..w.coaches.len()).find_map(|c| coach_look::sources(&w, c).into_iter().next().map(|s| (c, s.1)));
    let Some((c, src)) = found else { return };
    let owed = w.folk.network.ledger.len();
    let id = refs::start(&mut w, c, &src.code()).unwrap();
    assert_eq!(w.folk.network.ledger.len(), owed + 1, "you owe him for the call");
    let him = refs::them(&w, src);
    let s = scene::open(&mut w, id).unwrap();
    assert_eq!(office::across(&w, &s), Some(him));
    assert!(s.picks.iter().any(|p| p.code == sit::REF));
    // A straight answer about his football: one read per facet, each
    // inside 1-20, kept on the coach's page with its facet.
    let reading = Reading {
        acts: vec![Act::Question { topic: Topic::Football, about: Some(Them::Coach(c)), words: String::new() }],
        ..Reading::default()
    };
    let told = Told { answer: Answer::Straight, released: vec!["he's good".into()], ..Told::default() };
    let t = Turn { told: vec![told], ..Turn::default() };
    if refs::agenda(&w, him, c).is_none() {
        return;
    }
    let had = coach_look::look_mut(&mut w, c).asked.len();
    let out = refs::after(&mut w, id, him, &reading, &t);
    let n = refs::facets(&w, c, Topic::Football).len();
    let asked = &coach_look::look_mut(&mut w, c).asked;
    assert_eq!(asked.len() - had, n);
    assert!(asked.iter().skip(had).all(|a| a.facet.is_some()));
    assert_eq!(out.len(), 1);
    for f in refs::facets(&w, c, Topic::Football) {
        let r = refs::read(&w, him, c, f).unwrap();
        assert!((1.0..=20.0).contains(&r));
    }
}

#[test]
fn a_line_to_someone_in_your_building_is_held_inside_the_chips() {
    let (w, team, _) = hurt();
    let doc = trainer(&w, team);
    let room = office::room(&w, &scene::Scene::new(scene::Kind::Trainer, w.today, "x")).unwrap();
    assert_eq!(room.present[0].who, doc);
    let hit = |trust: f32| Sized {
        hit: Hit { on: doc, item: Item::HisWord, how: How::Respected },
        trust,
        mood: 0.1,
        faded: false,
    };
    let mut up = vec![hit(0.08), hit(0.04)];
    office::cap(&w, &room, &mut up);
    let sum: f32 = up.iter().map(|h| h.trust).sum();
    assert!((sum - office::CAP_UP).abs() < 1e-6, "{sum}");
    assert!(up.iter().all(|h| h.mood == 0.0), "no mood for someone in your building");
    let mut down = vec![hit(-0.3)];
    office::cap(&w, &room, &mut down);
    assert!((down[0].trust - office::CAP_DOWN).abs() < 1e-6);
}

#[test]
fn trust_moves_where_the_world_keeps_it() {
    let (mut w, team, _) = hurt();
    let doc = trainer(&w, team);
    let before = office::trust(&w, doc);
    assert!(office::nudge(&mut w, doc, 0.02));
    assert!((office::trust(&w, doc) - before - 0.02).abs() < 1e-5);
    let r = CastRef::new(team, CastRole::Trainer, 0);
    let key = cast::get(&w, r).key();
    assert!(w.folk.mine.trust_of(key).is_some(), "the store his meeting's chips move");
}

#[test]
fn your_boss_says_yes_in_words_exactly_when_his_meeting_would() {
    let mut w = seat("college");
    let team = Team::seat(&w).unwrap();
    let bossman = office::cast_them(&w, CastRef::new(team, cast::boss_role(team), 0));
    let chip = boss::ask_yes(&w, team, AskKind::StaffMoney) && !boss::asked_lately(&w, AskKind::StaffMoney);
    let words = asks::willing(&w, bossman, &Request::Money("more for the staff".into())).unwrap();
    assert_eq!(words == Willing::Yes, chip);
    // The same effect as the chip.
    let mut by_chip = seat("college");
    let (yes, _) = boss::ask_now(&mut by_chip, team, AskKind::StaffMoney);
    let ran = office::parts().acting.run(&mut w, bossman, &Request::Money("more for the staff".into()));
    assert!(!ran.done.is_empty());
    assert_eq!(w.folk.staff.pool_bonus, by_chip.folk.staff.pool_bonus);
    assert_eq!(yes, chip);
}

#[test]
fn a_coordinator_moves_the_plan_by_the_staff_meetings_step() {
    let mut w = seat("college");
    let oc = (0..w.coaches.len()).find(|&c| {
        asks::on_your_staff(&w, c) && w.coaches[c].job.is_some_and(|j| j.role == Role::OffensiveCoordinator)
    });
    let Some(oc) = oc else { return };
    let him = Them::Coach(oc);
    assert_eq!(asks::willing(&w, him, &Request::Plan("run the ball more".into())), Some(Willing::Yes));
    assert_eq!(asks::willing(&w, him, &Request::Plan("bring more pressure".into())), Some(Willing::No));
    let mut by_chip = seat("college");
    crate::world::folk::dialogue::shift_plan(&mut by_chip, 2, 1.0);
    office::parts().acting.run(&mut w, him, &Request::Plan("run the ball more".into()));
    assert_eq!(w.plans.guide.run.to, by_chip.plans.guide.run.to);
}

#[test]
fn a_sit_down_with_no_key_plays_as_chips_and_ends_when_you_end_it() {
    let (mut w, team, id) = hurt();
    let doc = trainer(&w, team);
    let sid = sit::start(&mut w, doc, None).unwrap();
    let s = scene::open(&mut w, sid).unwrap();
    assert!(s.picks.iter().any(|p| p.code == sit::INJURY), "a chip for the hurt man");
    assert_eq!(sit::hurt_men(&w, &s), vec![id]);
    let lines = s.lines.len();
    let s = scene::answer(&mut w, sid, sit::INJURY).unwrap();
    assert!(s.lines.len() > lines, "he answers");
    assert!(!s.done);
    assert!(!s.picks.is_empty(), "the chips are back");
    assert!(store::open(&w, sid).is_some(), "the talk is open");
    let s = scene::answer(&mut w, sid, sit::END).unwrap();
    assert!(s.done);
    assert!(store::open(&w, sid).is_none());
}

#[test]
fn a_typed_line_to_the_doctor_plays_on_the_office_parts() {
    let (mut w, team, id) = hurt();
    let doc = trainer(&w, team);
    let sid = sit::start(&mut w, doc, None).unwrap();
    let s = scene::open(&mut w, sid).unwrap();
    let room = office::room(&w, &s).unwrap();
    store::begin(&mut w, sid, room.clone());
    let reading = Reading {
        acts: vec![Act::Question { topic: Topic::Injury, about: Some(Them::Player(id)), words: String::new() }],
        ..Reading::default()
    };
    let conv = Conversation {
        parts: office::parts(),
        said: Said { scene: sid, words: "Where's he at?".into(), reading, promises: Vec::new() },
    };
    let t = conv.turn(&w, &room, &conv.said.reading);
    assert_eq!(t.told.len(), 1);
    conv.apply(&mut w, &t);
    assert!(store::open(&w, sid).is_some_and(|o| o.lines == 1));
}

#[test]
fn a_goal_in_your_assistants_deal_is_a_clause_on_the_season() {
    use crate::world::folk::persona::terms::{DealKind, Offer};
    use crate::world::folk::terms::On;
    let mut w = seat("college");
    let c =
        (0..w.coaches.len()).find(|&c| asks::on_your_staff(&w, c) && w.folk.staff.deals.iter().any(|d| d.coach == c));
    let Some(c) = c else { return };
    let wins = crate::world::folk::terms::read(None, "wins", Some(9.0), false).unwrap();
    assert!(wins.season);
    let o = Offer { kind: DealKind::Staff, terms: vec![wins], ..Offer::default() };
    let out = office::deals::agree(&mut w, 0, Them::Coach(c), &o).unwrap();
    assert!(!out.is_empty());
    let cl = w.folk.clauses.last().expect("the goal is on the books");
    assert_eq!(cl.on, On::Staff);
    assert_eq!(cl.coach, Some(c));
    assert!(cl.pays > 0.0);
}
