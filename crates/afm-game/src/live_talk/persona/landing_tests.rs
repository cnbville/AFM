//! The landing read on the fake (item 8 of the ear audit, agreed by Cain
//! 2026-11-17): "Heard as" says how a line really went. In every room a
//! warm line lands, a harsh one doesn't and a task gets no landing words;
//! a recruit's point said with care still moves your case, and once the
//! talk's exchanges are spent the read says so; a line that touches three
//! things swings no further with his day than a line that touches one.
//! A greeting is no dodge and gets no landing words (a goodbye held back
//! to ask first is no greeting), how you said it is never one of his
//! things, and his tell shows only past a bar (Cain's screenshot,
//! 2026-11-18).

use std::sync::Arc;

use afm_ai::Fake;
use afm_ai::fake::body;
use afm_engine::world::background::Decider;
use afm_engine::world::folk::persona::Them;
use afm_engine::world::folk::persona::ear::{Hit, How, Item, Landing, Sized};
use afm_engine::world::folk::persona::talk::{Parts, channels, hits, recruit};
use afm_engine::world::folk::scene::{self, Kind, Scene};
use afm_engine::world::folk::{Team, case, facts, pitch, takeover};
use serde_json::{Value, json};

use super::super::{cards, ear::EAR, voice::VOICE};
use super::recruit_tests as rt;
use super::tests as tt;
use crate::Game;

/// The tags and words of the ear card's items for a talk.
fn items(g: &Game, s: &Scene, one_on_one: bool) -> Vec<(String, String)> {
    let w = g.world.as_ref().unwrap();
    let (c, _) = if one_on_one {
        cards::ear(w, s, &channels::one_on_one(s), &Parts::today(), "")
    } else {
        cards::ear(w, s, &super::room_of(w, s), &super::parts_of(s.kind), "")
    };
    c.items.iter().map(|i| (i.tag.clone(), i.text.clone())).collect()
}

/// The first item whose words start so.
fn tag(items: &[(String, String)], starts: &str) -> String {
    items.iter().find(|x| x.1.starts_with(starts)).map(|x| x.0.clone()).unwrap_or_else(|| panic!("{items:?}"))
}

/// One line on the fake: the ear's reading (with a manner when given), his
/// words, and the screen's answer.
fn line(g: &mut Game, f: &Fake, sid: u32, text: &str, acts: Vec<Value>, hits: Vec<Value>, manner: Value) -> Value {
    let mut r = json!({ "acts": acts, "hits": hits, "dodged": false, "not_talk": false });
    if !manner.is_null() {
        r["manner"] = manner;
    }
    f.push(EAR, Ok(body(&r.to_string(), 1200, 120)));
    f.push(VOICE, Ok(tt::voice("Okay, Coach.", "")));
    let t = tt::ok(g, "people_say", json!({ "id": sid, "text": text }))["ticket"].as_u64().unwrap();
    let v = tt::finish(g, t);
    assert_eq!(v["state"], "done", "{text}: {v}");
    v
}

fn heard(v: &Value) -> String {
    v["heard"].as_str().unwrap_or_default().to_string()
}

/// One line on the fake with the ear's whole reading and his reply.
fn said(g: &mut Game, f: &Fake, sid: u32, text: &str, reading: Value, his: &str) -> Value {
    f.push(EAR, Ok(body(&reading.to_string(), 1200, 120)));
    f.push(VOICE, Ok(tt::voice(his, "")));
    let t = tt::ok(g, "people_say", json!({ "id": sid, "text": text }))["ticket"].as_u64().unwrap();
    let v = tt::finish(g, t);
    assert_eq!(v["state"], "done", "{text}: {v}");
    v
}

/// The tell under his last reply.
fn last_tell(g: &Game, sid: u32) -> Option<String> {
    rt::scene_of(g, sid).lines.iter().rev().find(|l| l.by == 2).and_then(|l| l.tell.clone())
}

fn hit(item: &str, how: &str) -> Value {
    json!({ "on": "", "item": item, "how": how })
}

/// A task: something you ask of him that touches nothing of his.
fn task(what: &str, words: &str) -> Value {
    let mut a = tt::act("request", what, "Nothing", "");
    a["words"] = json!(words);
    a
}

/// A sit-down with a coordinator you kept (the holdover meeting), opened.
fn staff_talk(name: &str) -> (Game, Arc<Fake>, u32) {
    let (mut g, f, _) = tt::game(name, true);
    let w = g.world.as_mut().unwrap();
    let team = Team::seat(w).unwrap();
    let me = w.career.as_ref().unwrap().coach;
    let c = facts::assistants(w, team).into_iter().map(|x| x.0).find(|&c| c != me).expect("a staff coach");
    let old = (0..w.coaches.len()).find(|&k| k != me && k != c && w.coaches[k].job.is_none()).unwrap_or(0);
    let today = w.today;
    let t = w.folk.takeover.get_or_insert_with(takeover::Takeover::default);
    t.team = Some(team);
    t.started = Some(today);
    t.old_coach = Some(old);
    t.kept.clear();
    t.let_go.clear();
    w.folk.staff.interviewed.retain(|x| x.0 != c);
    let mut s = Scene::new(Kind::Holdover, today, "Meeting the offensive coordinator");
    s.a = c as u64;
    s.stops = false;
    s.place = "Your office".into();
    let sid = scene::add(w, s);
    tt::ok(&mut g, "people_scene", json!({ "id": sid }));
    (g, f.unwrap(), sid)
}

/// Your first meeting with your athletic director, opened.
fn boss_talk(name: &str) -> (Game, Arc<Fake>, u32) {
    let (mut g, f, _) = tt::game(name, true);
    let w = g.world.as_mut().unwrap();
    let mut s = Scene::new(Kind::BossFirst, w.today, "Meeting the athletic director");
    s.team = Team::seat(w);
    s.stops = false;
    s.place = "His office".into();
    let sid = scene::add(w, s);
    tt::ok(&mut g, "people_scene", json!({ "id": sid }));
    (g, f.unwrap(), sid)
}

/// A call from your player's family, opened.
fn parent_talk(name: &str) -> (Game, Arc<Fake>, u32) {
    let (mut g, f, him) = tt::game(name, true);
    let w = g.world.as_mut().unwrap();
    let mut s = Scene::new(Kind::ParentCall, w.today, "A call from his family");
    s.a = him as u64;
    s.stops = false;
    s.place = "On the phone".into();
    let sid = scene::add(w, s);
    tt::ok(&mut g, "people_scene", json!({ "id": sid }));
    (g, f.unwrap(), sid)
}

/// Your one-on-one with your player, opened.
fn player_talk(name: &str) -> (Game, Arc<Fake>, u32) {
    let (mut g, f, him) = tt::game(name, true);
    let sid = tt::talk(&mut g, him, "role");
    (g, f.unwrap(), sid)
}

/// The card's point that would move your case the most right now, by tag.
fn best_point(g: &Game, sid: u32) -> (String, f32) {
    let w = g.world.as_ref().unwrap();
    let s = rt::scene_of(g, sid);
    let (_, keys) = cards::ear(w, &s, &super::room_of(w, &s), &super::parts_of(s.kind), "");
    keys.items
        .iter()
        .filter_map(|(tag, item)| {
            let Item::Point(key) = item else { return None };
            let p = pitch::typed_point(w, &s, afm_engine::world::folk::claims::Kind::from_key(key), None)?;
            Some((tag.clone(), pitch::typed_moves(w, &s, &p, p.lean)))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .expect("a point of yours on his card")
}

/// The landing read's yardstick for a recruit's talk, on the turn's own
/// wants.
fn best_now(g: &Game, sid: u32) -> f32 {
    let w = g.world.as_ref().unwrap();
    let s = rt::scene_of(g, sid);
    let room = super::room_of(w, &s);
    let him = afm_engine::world::folk::persona::talk::across(&room).expect("a recruit across");
    let wants = super::parts_of(s.kind).wanting.wants(w, him);
    recruit::best_now(w, &s, &wants)
}

fn case_total(g: &Game, i: usize, j: usize) -> f32 {
    case::of(&g.world.as_ref().unwrap().people[i], j).map_or(0.0, |c| c.total())
}

const CARING: &str = r#"{"heat": "even", "language": "clean", "aim": "out", "care": "caring", "say": "tell"}"#;

/// A warm manner the man across takes well (a lift on his trust), as the
/// ear would write it: who he is decides which (a steady professional
/// hears little in plain care, and likes being asked).
fn warm_manner(g: &Game, sid: u32) -> Option<Value> {
    use afm_engine::world::folk::persona::manner::{self, Manner};
    let w = g.world.as_ref().unwrap();
    let room = super::room_of(w, &rt::scene_of(g, sid));
    let him = afm_engine::world::folk::persona::talk::across(&room)?;
    let set = manner::setting(w, &room);
    [("even", "caring", "tell"), ("even", "caring", "ask"), ("even", "neutral", "ask"), ("firm", "caring", "ask")]
        .into_iter()
        .find(|(heat, care, say)| {
            let t = manner::took(w, him, &Manner::from_words(heat, "clean", "out", care, say), &set);
            t.as_.how().is_some() && t.trust > 0.0
        })
        .map(|(heat, care, say)| json!({ "heat": heat, "language": "clean", "aim": "out", "care": care, "say": say }))
}

#[test]
fn every_room_reads_a_warm_line_a_harsh_one_and_a_task() {
    type Open = fn(&str) -> (Game, Arc<Fake>, u32);
    let rooms: [(&str, Open); 4] =
        [("player", player_talk), ("staff", staff_talk), ("boss", boss_talk), ("parent", parent_talk)];
    for (room, open) in rooms {
        // Warm: your player is told every good thing at once (held to the
        // best chip he could be given); anyone else is reassured on what he
        // wants most.
        let (mut g, f, sid) = open(&format!("landing-warm-{room}"));
        let s = rt::scene_of(&g, sid);
        let card = items(&g, &s, room == "player");
        let warm: Vec<Value> = if room == "player" {
            card.iter().map(|x| hit(&x.0, "reassured")).collect()
        } else {
            vec![hit(&tag(&card, "what he wants"), "reassured")]
        };
        let v = line(&mut g, &f, sid, "You matter here, and we'll get you what you need.", vec![], warm, Value::Null);
        assert!(heard(&v).ends_with("it landed"), "{room}, a warm line lands: {v}");
        // Harsh: an insult to his pride.
        let (mut g, f, sid) = open(&format!("landing-harsh-{room}"));
        let s = rt::scene_of(&g, sid);
        let pride = tag(&items(&g, &s, room == "player"), "his pride");
        let v = line(
            &mut g,
            &f,
            sid,
            "That's pathetic, and you know it.",
            vec![],
            vec![hit(&pride, "insulted")],
            Value::Null,
        );
        assert!(heard(&v).ends_with("it didn't land"), "{room}, a harsh line doesn't: {v}");
        // A task touches nothing of his, said warmly or not: what the line
        // was, no landing words ("Heard as a request.").
        let (mut g, f, sid) = open(&format!("landing-task-{room}"));
        let care = warm_manner(&g, sid).unwrap_or_else(|| serde_json::from_str(CARING).unwrap());
        let v = line(&mut g, &f, sid, "Get me a report on it by Friday.", vec![task("report", "")], vec![], care);
        assert_eq!(heard(&v), "a request", "{room}, a task gets no landing words: {v}");
    }
}

#[test]
fn a_recruit_reads_on_your_case_and_your_care_never_takes_the_points_place() {
    // Your best point said plainly, then said with care: the same pull on
    // your case, and it lands either way. Before, the caring manner's lift
    // took the point's place and nothing moved.
    let mut moved = Vec::new();
    for (k, manner) in [Value::Null, serde_json::from_str(CARING).unwrap()].into_iter().enumerate() {
        let (mut g, f, sid, i, _, mine) =
            rt::recruit_talk(&format!("landing-rcare-{k}"), Kind::HomeVisit, Decider::Parent);
        let (point, best) = best_point(&g, sid);
        let now = best_now(&g, sid);
        assert!(best > 0.0 && best >= 0.5 * now, "your best point on his card moves your case: {best} of {now}");
        let before = case_total(&g, i, mine);
        let v = line(&mut g, &f, sid, "Our degree opens doors.", vec![], vec![hit(&point, "reassured")], manner);
        moved.push(case_total(&g, i, mine) - before);
        assert!(heard(&v).ends_with("it landed"), "your best point lands: {v}");
    }
    assert!(moved[1] > 0.0, "said with care, the point still moves your case: {moved:?}");
    assert!((moved[0] - moved[1]).abs() < 1e-6, "and by as much as said plainly: {moved:?}");
    // An insult costs his trust past the bar; a task touches nothing.
    let (mut g, f, sid, ..) = rt::recruit_talk("landing-rharsh", Kind::HomeVisit, Decider::Parent);
    let pride = tag(&items(&g, &rt::scene_of(&g, sid), false), "his pride");
    let v = line(&mut g, &f, sid, "That's a soft question.", vec![], vec![hit(&pride, "insulted")], Value::Null);
    assert!(heard(&v).ends_with("it didn't land"), "{v}");
    // A warm word on his pride alone moves nothing with a recruit (only how
    // you said it reaches his trust): it touched him, and didn't move him.
    let (mut g, f, sid, ..) = rt::recruit_talk("landing-rpride", Kind::HomeVisit, Decider::Parent);
    let pride = tag(&items(&g, &rt::scene_of(&g, sid), false), "his pride");
    let v = line(&mut g, &f, sid, "You're a special player.", vec![], vec![hit(&pride, "reassured")], Value::Null);
    assert!(heard(&v).ends_with("it didn't move him"), "{v}");
    let (mut g, f, sid, ..) = rt::recruit_talk("landing-rtask", Kind::HomeVisit, Decider::Parent);
    let ask = task("other", "send me your transcript");
    let care = warm_manner(&g, sid).unwrap_or_else(|| serde_json::from_str(CARING).unwrap());
    let v = line(&mut g, &f, sid, "Send me your transcript this week.", vec![ask], vec![], care);
    assert_eq!(heard(&v), "a request", "{v}");
}

#[test]
fn once_the_exchanges_are_spent_the_read_says_so() {
    let (mut g, f, sid, ..) = rt::recruit_talk("landing-spent", Kind::HomeVisit, Decider::Parent);
    let s = rt::scene_of(&g, sid);
    // Every point you can make, then his wants (each your best point left
    // in its group): one point an exchange.
    let mut pitch_items: Vec<String> =
        items(&g, &s, false).into_iter().filter(|x| x.1.starts_with("what you can tell him")).map(|x| x.0).collect();
    pitch_items.extend(items(&g, &s, false).into_iter().filter(|x| x.1.starts_with("what he wants")).map(|x| x.0));
    let first = pitch_items[0].clone();
    for p in pitch_items {
        if pitch::exchanges_left(g.world.as_ref().unwrap(), &rt::scene_of(&g, sid)) == 0 {
            break;
        }
        line(&mut g, &f, sid, "Here's what we can do for you.", vec![], vec![hit(&p, "reassured")], Value::Null);
    }
    assert_eq!(pitch::exchanges_left(g.world.as_ref().unwrap(), &rt::scene_of(&g, sid)), 0, "the case is made");
    let v = line(&mut g, &f, sid, "One more thing about us.", vec![], vec![hit(&first, "reassured")], Value::Null);
    assert!(heard(&v).ends_with("you've made your case for today"), "{v}");
}

#[test]
fn a_line_that_touches_three_things_swings_no_further_with_his_day() {
    // The same lines on eight days (the talk's scene moves his day's draw):
    // one thing, and the same thing with two more beside it (one up, one
    // down, so neither line nears a cap). His day moves both by the same.
    let nets = |three: bool| -> Vec<f32> {
        (0..8u32)
            .map(|k| {
                let (mut g, f, him) = tt::game(&format!("landing-day-{three}-{k}"), true);
                let f = f.unwrap();
                g.world.as_mut().unwrap().folk.next_scene += 13 * k;
                let sid = tt::talk(&mut g, him, "role");
                let s = tt::scene_of(&g, sid);
                let card = items(&g, &s, true);
                let mut hs = vec![hit(&tag(&card, "his family"), "respected")];
                if three {
                    hs.push(hit(&tag(&card, "his pride"), "doubted"));
                    let mind = card.iter().find(|x| x.1.starts_with("on his mind")).map_or_else(
                        || card.iter().filter(|x| x.1.starts_with("what he wants")).nth(1).unwrap().0.clone(),
                        |x| x.0.clone(),
                    );
                    hs.push(hit(&mind, "reassured"));
                }
                let held = |g: &Game| {
                    let w = g.world.as_ref().unwrap();
                    tt::trust(g, him) + w.folk.scenes.iter().find(|x| x.id == sid).unwrap().f[1]
                };
                let before = held(&g);
                line(&mut g, &f, sid, "Your family's raised a good one.", vec![], hs, Value::Null);
                held(&g) - before
            })
            .collect()
    };
    let spread = |v: &[f32]| v.iter().copied().fold(f32::MIN, f32::max) - v.iter().copied().fold(f32::MAX, f32::min);
    let (one, three) = (nets(false), nets(true));
    eprintln!("one thing {one:?}\nthree things {three:?}");
    assert!(spread(&one) > 0.02, "his day moves a line: {one:?}");
    assert!(
        spread(&three) <= 1.25 * spread(&one) + 0.003,
        "three things swing no further than one: {} against {}",
        spread(&three),
        spread(&one)
    );
}

/// What Jalon Perry opened with in Cain's screenshot (2026-11-18).
const OPENING: &str =
    "Walked in with a coffee and a sore back. Ready. Tell me what I'm getting paid and how it's figured.";

/// His opening asks you something (Cain's screenshot, word for word).
fn he_opens_asking(g: &mut Game, sid: u32) {
    let w = g.world.as_mut().unwrap();
    let sc = w.folk.scenes.iter_mut().find(|x| x.id == sid).unwrap();
    match sc.lines.iter_mut().rev().find(|l| l.by == 2) {
        Some(l) => l.text = OPENING.into(),
        None => sc.them("Jalon Perry", OPENING, None),
    }
}

#[test]
fn a_greeting_is_no_dodge_and_says_nothing_of_how_it_landed() {
    // "First time meeting, nice to meet you", said to an opening that
    // asked what he'd be paid, was heard as "you didn't answer his
    // question; respected on his pride; hard to tell how it landed", and
    // "He gives you nothing either way" stood under his warm reply. The
    // ear here hears a dodge, and the line is said in a manner he takes
    // well.
    let (mut g, f, him) = tt::game("landing-greeting", true);
    let f = f.unwrap();
    let sid = tt::talk(&mut g, him, "role");
    he_opens_asking(&mut g, sid);
    let care = warm_manner(&g, sid).expect("a warm manner he takes well");
    let t0 = tt::trust(&g, him);
    let greeting = json!({ "acts": [], "hits": [], "dodged": true, "not_talk": false, "manner": care });
    let v = said(
        &mut g,
        &f,
        sid,
        "First time meeting, nice to meet you.",
        greeting,
        "Nice to meet you too, Coach. So what am I getting paid?",
    );
    assert_eq!(heard(&v), "", "no dodge, nothing of his touched, no landing words: {v}");
    assert_eq!(last_tell(&g, sid), None, "no tell under his warm reply");
    assert!(tt::trust(&g, him) > t0, "he took your care, and paid nothing for a dodge that wasn't");
    // His question waited for the next line: he asked it again, and a line
    // that moves on doesn't answer it.
    let away = json!({ "acts": [], "hits": [], "dodged": true, "not_talk": false });
    let v = said(&mut g, &f, sid, "Let's talk about how you're settling in.", away, "Fine.");
    assert!(heard(&v).contains("you didn't answer his question"), "{v}");
    // How you said it is never one of his things: a warm line on his family
    // said with care names his family, never his pride.
    let (mut g, f, sid) = player_talk("landing-manner");
    let family = tag(&items(&g, &rt::scene_of(&g, sid), true), "his family");
    let care = warm_manner(&g, sid).expect("a warm manner he takes well");
    let v = line(&mut g, &f, sid, "Your family raised a good one.", vec![], vec![hit(&family, "respected")], care);
    assert!(heard(&v).starts_with("respected on his family"), "{v}");
    assert!(!heard(&v).contains("pride") && !heard(&v).contains("how you said it"), "{v}");
}

#[test]
fn a_goodbye_to_his_opening_question_is_no_greeting() {
    // The game holds a goodbye back to ask "Do you want to continue?", so
    // the line reaches the turn with no act left; it still made one, and
    // said first to his opening question it can dodge it.
    let (mut g, f, him) = tt::game("landing-goodbye", true);
    let f = f.unwrap();
    let sid = tt::talk(&mut g, him, "role");
    he_opens_asking(&mut g, sid);
    let bye = json!({ "acts": [{ "act": "end" }], "hits": [], "dodged": true, "not_talk": false });
    let v = said(&mut g, &f, sid, "Okay, that's all for now.", bye, "Alright, Coach.");
    assert!(heard(&v).contains("you didn't answer his question"), "{v}");
    assert!(g.ai.persona.wrap_asked(sid), "the page asks whether to wrap up");
}

#[test]
fn his_tell_shows_only_past_a_bar() {
    let (g, _, him) = tt::game("landing-tell", false);
    let w = g.world.as_ref().unwrap();
    let him = Them::Player(him);
    for l in [Landing::Untouched, Landing::Little, Landing::Nothing, Landing::Cost] {
        assert_eq!(hits::tell(w, him, l), "", "{l:?}: nothing past his words to catch");
    }
    assert!(!hits::tell(w, him, Landing::Landed).is_empty());
    assert!(!hits::tell(w, him, Landing::Missed).is_empty());
}

#[test]
fn the_bars_read_small_moves_in_plain_words() {
    let bars = hits::PLAYER_BARS;
    assert_eq!(hits::read(0.031, bars), Landing::Landed);
    assert_eq!(hits::read(0.012, bars), Landing::Little);
    assert_eq!(hits::read(0.0, bars), Landing::Nothing);
    assert_eq!(hits::read(-0.012, bars), Landing::Cost);
    assert_eq!(hits::read(-0.031, bars), Landing::Missed);
    // A line with nothing on him touched nothing of his; one that did and
    // moved nothing says so.
    let him = Them::Player(1);
    assert_eq!(hits::landing(him, &[], &[]), Landing::Untouched);
    let faded =
        Sized { hit: Hit { on: him, item: Item::Pride, how: How::Reassured }, trust: 0.0, mood: 0.0, faded: true };
    assert_eq!(hits::landing(him, std::slice::from_ref(&faded), &[]), Landing::Nothing);
    // How you said it alone touches nothing of his; beside a hit that did,
    // it counts in how the line went.
    let manner =
        Sized { hit: Hit { on: him, item: Item::Manner, how: How::Reassured }, trust: 0.02, mood: 0.02, faded: false };
    assert_eq!(hits::landing(him, std::slice::from_ref(&manner), &[]), Landing::Untouched);
    assert_eq!(hits::landing(him, &[faded, manner], &[]), Landing::Landed);
    assert_eq!(Landing::Nothing.words("him"), "it didn't move him");
    assert_eq!(Landing::Untouched.words("him"), "");
    // His tell shows only past one of his bars.
    assert_eq!((Landing::Landed.sign(), Landing::Little.sign(), Landing::Missed.sign()), (1, 0, -1));
}
