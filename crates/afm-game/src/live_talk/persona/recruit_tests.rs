//! Lane N5a on the fake: a recruit's talk and a college staff's call play
//! as v3 talks. A point lands on your case at no more than the chip's
//! size and never raises his trust, a promise counts as you say it,
//! asking him to commit runs the desk's own rule, his family's "let me
//! think" waits on your confirm and answers on its day, and your words to
//! a college staff become the call's word for the trail and your
//! credibility.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use afm_ai::Fake;
use afm_ai::fake::body;
use afm_engine::world::background::Decider;
use afm_engine::world::data::Division;
use afm_engine::world::folk::case;
use afm_engine::world::folk::persona::Them;
use afm_engine::world::folk::persona::talk::{Parts, recruit};
use afm_engine::world::folk::scene::{self, Kind, Scene, Who};
use afm_engine::world::folk::{pitch, pledge};
use afm_engine::world::front_office::college as fc;
use afm_engine::world::seat::{Job, You};
use afm_engine::world::{Stage, World, WorldOptions};
use serde_json::{Value, json};

use super::super::{ear::EAR, voice::VOICE};
use crate::Game;

fn you() -> You {
    let teaching = vec![12.0; afm_engine::world::coach::GROUP_COUNT];
    serde_json::from_value(json!({
        "name": "Test Coach", "age": 45, "ethnicity": "White", "home_state": 0, "ex_pro": false, "reputation": 60.0,
        "teaching": teaching, "scheme": 12.0, "game_management": 12.0,
        "motivation": 12.0, "discipline": 12.0, "recruiting": 12.0, "network": 12.0, "relationships": 12.0,
        "style": { "risk": 10.0, "freedom": 10.0, "physical": 10.0, "ball_hunting": 10.0, "mould": 10.0 },
        "yours": { "play_calling": 12.0, "adaptation": 12.0, "evaluation": 12.0, "negotiating": 12.0 }
    }))
    .unwrap()
}

pub(super) fn home(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("afm-live-n5a-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// A game at a college on the fake, and a talk of `kind` with a senior
/// holding your offer: (the game, the fake, the talk, his place, his id,
/// your program).
pub(super) fn recruit_talk(name: &str, kind: Kind, decider: Decider) -> (Game, Arc<Fake>, u32, usize, u32, usize) {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    let mine = w.programs.iter().position(|p| p.division == Division::FBS).unwrap();
    w.start_career(&you(), Job::College(mine), 7);
    w.add_class_aged(17);
    let i = (0..w.people.len())
        .filter(|&i| w.people[i].stage == Stage::HighSchool && w.people[i].family().decider == decider)
        .min_by_key(|&i| (w.people[i].class_of, i))
        .expect("a recruit whose family decides this way");
    let id = w.people[i].player.id;
    fc::add(&mut w, id).unwrap();
    fc::offer(&mut w, id).unwrap();
    let mut s = Scene::new(kind, w.today, "A talk".to_string());
    s.a = id as u64;
    s.b = mine as u64;
    s.who = vec![Who::Player(id)];
    let sid = scene::add(&mut w, s);
    let mut g = Game::new(home(name));
    g.world = Some(w);
    let f = Arc::new(Fake::new());
    g.ai.use_fake(f.clone());
    g.call("people_scene", &json!({ "id": sid })).unwrap();
    (g, f, sid, i, id, mine)
}

pub(super) fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    g.call(cmd, &args).unwrap_or_else(|e| panic!("{cmd}: {e}"))
}

pub(super) fn finish(g: &mut Game, ticket: u64) -> Value {
    let t = Instant::now();
    loop {
        let v = ok(g, "ai_poll", json!({ "ticket": ticket }));
        if v["state"] != "wait" || t.elapsed() > Duration::from_secs(5) {
            return v;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

pub(super) fn act(kind: &str, what: &str, value: &str, about: &str) -> Value {
    json!({ "act": kind, "topic": "Nothing", "about": about, "claim": "none", "value": value, "deal": "none",
        "what": what, "when": "", "words": "", "gives": [], "terms": [] })
}

pub(super) fn ear(acts: Vec<Value>, hits: Vec<Value>) -> Value {
    body(&json!({ "acts": acts, "hits": hits, "dodged": false, "not_talk": false }).to_string(), 1200, 120)
}

pub(super) fn voice(line: &str) -> Value {
    body(&json!({ "line": line, "colour": "" }).to_string(), 900, 60)
}

/// Says one line on the fake: the ear's reading, then his words.
pub(super) fn say(g: &mut Game, f: &Fake, sid: u32, text: &str, reading: Value) -> Value {
    f.push(EAR, Ok(reading));
    f.push(VOICE, Ok(voice("Okay, Coach.")));
    let t = ok(g, "people_say", json!({ "id": sid, "text": text }))["ticket"].as_u64().unwrap();
    finish(g, t)
}

pub(super) fn scene_of(g: &Game, sid: u32) -> Scene {
    g.world.as_ref().unwrap().folk.scenes.iter().find(|s| s.id == sid).unwrap().clone()
}

fn case_total(g: &Game, i: usize, j: usize) -> f32 {
    case::of(&g.world.as_ref().unwrap().people[i], j).map_or(0.0, |c| c.total())
}

#[test]
fn a_point_lands_on_the_case_no_bigger_than_the_chip_and_never_raises_trust() {
    let (mut g, f, sid, i, id, mine) = recruit_talk("point", Kind::HomeVisit, Decider::Parent);
    assert!(super::plays(&g, &scene_of(&g, sid)), "a home visit plays as v3");
    // The first of his wants that a point of yours speaks to: its tag on
    // the card (his wants come first, c1, c2...), and the chip's own size.
    let (tag, chip, trust) = {
        let w = g.world.as_ref().unwrap();
        let s = scene_of(&g, sid);
        let wants = Parts::today().wanting.wants(w, Them::Player(id));
        let k = (0..wants.len())
            .find(|&k| {
                recruit::group_of(wants[k].kind).and_then(|gr| pitch::typed_point(w, &s, None, Some(gr))).is_some()
            })
            .expect("a want your case speaks to");
        let p = pitch::typed_point(w, &s, None, recruit::group_of(wants[k].kind)).unwrap();
        let trust = afm_engine::world::folk::persona::makeup::trust(w, Them::Player(id));
        (format!("c{}", k + 1), p.lean, trust)
    };
    let before = case_total(&g, i, mine);
    let hit = json!({ "on": "", "item": tag, "how": "reassured" });
    let v =
        say(&mut g, &f, sid, "You'd be home in two hours, and your mom can make every game.", ear(vec![], vec![hit]));
    assert_eq!(v["state"], "done", "{v}");
    let moved = case_total(&g, i, mine) - before;
    assert!(moved <= chip.max(0.0) + 1e-6, "no bigger than the chip: {moved} > {chip}");
    let w = g.world.as_ref().unwrap();
    let after = afm_engine::world::folk::persona::makeup::trust(w, Them::Player(id));
    assert!(after <= trust + 1e-6, "no chip raises a recruit's trust, so no line does");
    // His own trust in his high school coach is untouched.
    let s = scene_of(&g, sid);
    assert!(!s.done, "a home visit has more to say");
    assert!(s.lines.iter().any(|l| l.typed.is_some()), "your words are on the talk");
    // The same point again in the same talk adds nothing.
    let before = case_total(&g, i, mine);
    let hit = json!({ "on": "", "item": tag, "how": "reassured" });
    say(&mut g, &f, sid, "Home is close. That matters.", ear(vec![], vec![hit]));
    let again = case_total(&g, i, mine) - before;
    assert!(again <= moved + 1e-6, "a second point never beats the first");
}

#[test]
fn a_question_draws_no_reaction_to_a_pitch() {
    // The talks audit, 2026-11-16: "That's exactly what I'm looking for"
    // was on his card for "How was your game Friday?", and "Polite enough,
    // Coach" came back to "How's his year going?".
    let reaction = |card: &str| {
        ["looking for.\"", "polite about it", "doesn't matter much", "back it up"].iter().any(|r| card.contains(r))
    };
    let mut cards = Vec::new();
    for (name, asked) in [("pitch-made", false), ("pitch-asked", true)] {
        let (mut g, f, sid, _, id, _) = recruit_talk(name, Kind::HomeVisit, Decider::Parent);
        let tag = {
            let w = g.world.as_ref().unwrap();
            let s = scene_of(&g, sid);
            let wants = Parts::today().wanting.wants(w, Them::Player(id));
            let k = (0..wants.len())
                .find(|&k| {
                    recruit::group_of(wants[k].kind).and_then(|gr| pitch::typed_point(w, &s, None, Some(gr))).is_some()
                })
                .expect("a want your case speaks to");
            format!("c{}", k + 1)
        };
        let hit = json!({ "on": "", "item": tag, "how": "reassured" });
        let mut q = act("question", "", "", "");
        q["topic"] = json!("Football");
        let (text, acts) =
            if asked { ("How's his year going?", vec![q]) } else { ("You'd be home in two hours.", vec![]) };
        let v = say(&mut g, &f, sid, text, ear(acts, vec![hit]));
        assert_eq!(v["state"], "done", "{v}");
        cards.push(f.asked().into_iter().rev().find(|r| r.label == VOICE).map(|r| r.user).unwrap_or_default());
    }
    assert!(reaction(&cards[0]), "a point draws a reaction: {}", cards[0]);
    assert!(!reaction(&cards[1]), "a question draws none: {}", cards[1]);
}

#[test]
fn his_parents_answer_under_their_own_name_with_no_label_in_the_words() {
    let (mut g, f, sid, _, id, _) = recruit_talk("speaker", Kind::HomeVisit, Decider::Parent);
    let parent = {
        let w = g.world.as_ref().unwrap();
        afm_engine::world::folk::persona::talk::channels::name(w, Them::Kin(id, 0))
    };
    // His parents do the talking: asked what matters most, they answer,
    // and the line is theirs on the screen. The voice put the name in
    // front (the talks audit, 2026-11-16): it goes.
    let mut q = act("question", "", "", "");
    q["topic"] = json!("Wants");
    f.push(EAR, Ok(ear(vec![q], vec![])));
    f.push(VOICE, Ok(voice(&format!("{parent}: Getting him to the next level matters most to us, Coach."))));
    let t =
        ok(&mut g, "people_say", json!({ "id": sid, "text": "What matters most to you in this decision?" }))["ticket"]
            .as_u64()
            .unwrap();
    let v = finish(&mut g, t);
    assert_eq!(v["state"], "done", "{v}");
    let s = scene_of(&g, sid);
    let his = s.lines.iter().rev().find(|l| l.by > 1).unwrap();
    assert_eq!((his.by, his.name.as_str()), (3, parent.as_str()), "{:?}", s.lines);
    assert_eq!(his.text, "Getting him to the next level matters most to us, Coach.");
    let card = f.asked().into_iter().rev().find(|r| r.label == VOICE).map(|r| r.user).unwrap_or_default();
    assert!(card.contains(&format!("This line is answered by {parent}")), "{card}");
}

#[test]
fn a_promise_to_a_recruit_counts_as_you_say_it() {
    let (mut g, f, sid, i, _, mine) = recruit_talk("promise", Kind::RecruitCall, Decider::Himself);
    let v = say(
        &mut g,
        &f,
        sid,
        "You'll get your shot on the field as a freshman.",
        ear(vec![act("promise", "early_chance", "", "")], vec![]),
    );
    assert_eq!(v["state"], "done", "{v}");
    assert!(v.get("confirm").is_none(), "a promise needs no confirm: {v}");
    assert_eq!(pledge::on_case(g.world.as_ref().unwrap(), i, mine), Some(pledge::Kind::Early));
    // How it lands is his own words, quoted: "They've heard promises
    // before" came back word for word (the talks audit, 2026-11-16).
    let card = f.asked().into_iter().rev().find(|r| r.label == VOICE).map(|r| r.user).unwrap_or_default();
    assert!(
        card.contains("He says: \"I believe you.") || card.contains("He says: \"I've heard promises before."),
        "{card}"
    );
    assert!(!card.contains("They've heard") && !card.contains("They believe"), "{card}");
}

#[test]
fn asking_him_to_commit_runs_the_desks_own_rule() {
    let (mut g, f, sid, i, id, mine) = recruit_talk("commit", Kind::RecruitCall, Decider::Himself);
    let ready = {
        let w = g.world.as_ref().unwrap();
        recruit::ready(w, &recruit::with(w, &scene_of(&g, sid)).unwrap())
    };
    say(&mut g, &f, sid, "Commit to us today.", ear(vec![act("request", "commit", "", "")], vec![]));
    let w = g.world.as_ref().unwrap();
    let committed = w.people[i].cal.commit.is_some_and(|c| c.0 == mine);
    assert_eq!(committed, ready, "he commits exactly when the desk says he would");
    let _ = id;
}

#[test]
fn a_family_that_takes_time_waits_on_your_confirm_and_answers_on_its_day() {
    let (mut g, f, sid, _, id, _) = recruit_talk("owed", Kind::HomeVisit, Decider::Parent);
    let mut offer = act("offer", "", "", "");
    offer["deal"] = json!("Nil");
    offer["words"] = json!("forty thousand a year in NIL");
    offer["gives"] = json!([{ "give": "money", "number": 40000, "who": "" }]);
    let v = say(&mut g, &f, sid, "We can do forty thousand a year in NIL.", ear(vec![offer], vec![]));
    assert_eq!(v["state"], "done", "{v}");
    let owed = |g: &Game| {
        g.world.as_ref().unwrap().folk.live.persona.owed.iter().filter(|o| o.with == Them::Player(id)).count()
    };
    let Some(list) = v.get("confirm").and_then(Value::as_array) else {
        // He answered at once (a yes, a counter or a no): nothing is owed.
        assert_eq!(owed(&g), 0, "{v}");
        return;
    };
    assert_eq!(owed(&g), 0, "nothing is owed before you confirm");
    let c = &list[0];
    ok(&mut g, c["cmd"].as_str().unwrap(), c["args"].clone());
    let by = {
        let w = g.world.as_ref().unwrap();
        w.folk.live.persona.owed.iter().find(|o| o.with == Them::Player(id)).map(|o| o.by)
    };
    if let Some(by) = by {
        let w = g.world.as_mut().unwrap();
        w.today = by;
        recruit::day(w, by);
        assert_eq!(owed(&g), 0, "their answer came on its day");
    }
}

#[test]
fn with_v3_off_a_recruits_talk_plays_as_lane_ten() {
    let (mut g, _, sid, ..) = recruit_talk("off", Kind::RecruitCall, Decider::Himself);
    g.ai.persona_talks = false;
    assert!(!super::plays(&g, &scene_of(&g, sid)));
}

/// A high school seat, and a college staff's call about your player with
/// a worry about his grades (a player whose grade point average is below
/// `below`).
pub(super) fn college_call(name: &str, below: f64) -> (Game, Arc<Fake>, u32, u32) {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    w.add_class_aged(17);
    let i = (0..w.people.len())
        .find(|&i| {
            w.people[i].stage == Stage::HighSchool && afm_engine::world::scouting::file::gpa(&w.people[i]) < below
        })
        .expect("a player with grades to worry about");
    let h = w.people[i].hs;
    w.start_career(&you(), Job::HighSchool(h), 7);
    let id = w.people[i].player.id;
    let j = w.programs.iter().position(|p| p.division == Division::FBS).unwrap();
    let mut s = Scene::new(Kind::CollegeCall, w.today, "A college calls".to_string());
    s.a = id as u64;
    s.b = j as u64;
    s.c = 1;
    s.who = vec![Who::Player(id)];
    let sid = scene::add(&mut w, s);
    let mut g = Game::new(home(name));
    g.world = Some(w);
    let f = Arc::new(Fake::new());
    g.ai.use_fake(f.clone());
    g.call("people_scene", &json!({ "id": sid })).unwrap();
    (g, f, sid, id)
}

fn claim(value: &str) -> Value {
    let mut a = act("claim", "", value, "p2");
    a["claim"] = json!("stat");
    a
}

#[test]
fn covering_for_his_grades_in_words_is_the_covering_word() {
    let (mut g, f, sid, id) = college_call("cover", 2.5);
    if !scene::takes_typed(&scene_of(&g, sid)) {
        return;
    }
    let v = say(
        &mut g,
        &f,
        sid,
        "He's a 3.6 student, grades are no problem.",
        ear(vec![claim("3.6 GPA"), act("end", "", "", "")], vec![]),
    );
    assert_eq!(v["state"], "done", "{v}");
    let w = g.world.as_ref().unwrap();
    let c = w.folk.dialogue.calls.iter().rev().find(|c| c.id == id).expect("the call's word");
    assert_eq!((c.word, c.concern), (2, 1), "you covered for his grades");
    assert!(scene_of(&g, sid).done);
}

#[test]
fn being_straight_about_his_grades_in_words_is_the_straight_word() {
    let (mut g, f, sid, id) = college_call("straight", 2.5);
    if !scene::takes_typed(&scene_of(&g, sid)) {
        return;
    }
    say(
        &mut g,
        &f,
        sid,
        "His grades need work, I won't lie.",
        ear(vec![claim("grades need work"), act("end", "", "", "")], vec![]),
    );
    let w = g.world.as_ref().unwrap();
    let c = w.folk.dialogue.calls.iter().rev().find(|c| c.id == id).expect("the call's word");
    assert_eq!((c.word, c.concern), (1, 1), "you were straight");
}

/// A call is weighed as said, not boiled down to one of the chip's three
/// words (Cain 2026-11-15): a line that covers for his grades and then
/// owns that they need work keeps both, and a bigger lie weighs more.
#[test]
fn a_college_call_keeps_everything_you_said_weighed() {
    use afm_engine::world::folk::claims::Stated;
    use afm_engine::world::folk::persona::ear::{Act, Reading};
    use afm_engine::world::folk::persona::talk::hs_call;
    let (mut g, f, sid, id) = college_call("mixed", 2.4);
    if !scene::takes_typed(&scene_of(&g, sid)) {
        return;
    }
    {
        let w = g.world.as_ref().unwrap();
        let s = scene_of(&g, sid);
        let claim = |v: &str| Reading {
            acts: vec![Act::Claim(Stated { kind: "stat".into(), about: Some(id), value: v.into() })],
            ..Reading::default()
        };
        // His real average, then a stretch of 0.4 over it and one of 1.5.
        let gpa = afm_engine::world::scouting::file::gpa(&w.people[afm_engine::world::folk::person(w, id).unwrap()]);
        let small = hs_call::weigh(w, &s, &claim(&format!("{:.1} GPA", gpa + 0.4))).cover;
        let big = hs_call::weigh(w, &s, &claim(&format!("{:.1} GPA", (gpa + 1.5).min(4.0)))).cover;
        assert!(big > small && small > 0.0, "a bigger lie weighs more: {small} {big}");
    }
    say(
        &mut g,
        &f,
        sid,
        "He's a 3.6 student. Well, honestly his grades need work.",
        ear(vec![claim("3.6 GPA"), claim("grades need work"), act("end", "", "", "")], vec![]),
    );
    let w = g.world.as_ref().unwrap();
    let c = w.folk.dialogue.calls.iter().rev().find(|c| c.id == id).expect("the call's word");
    let said = c.said.expect("weighed as said");
    assert!(said.cover > 0.0 && said.straight > 0.0, "both kept: {said:?}");
    assert_eq!(c.concern, 1);
    let s = scene_of(&g, sid);
    assert!(s.outcome.iter().any(|x| x.contains("two different things")), "{:?}", s.outcome);
}
