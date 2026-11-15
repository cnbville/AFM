//! Live talks lane 9, one-on-ones typed (LT1): a typed line with reading X
//! is chip X (any tone, as a click takes it), a line on another point is
//! that point's chip and then the move, a typed repeat counts as the
//! clicked repeat, none of the 40 exploit lines lands above the best chip
//! and every false fact is caught as it is said, a typed promise takes the
//! reading's kind and is logged once, and it is judged on its day in a
//! save loaded with no key.

use serde_json::Value;

use super::legacy::seat;
use crate::player::PlayingTime;
use crate::training::seat::Hurt;
use crate::world::folk::claims::{self, check, truth::Truth};
use crate::world::folk::scene::{self, Heard, Kind, PromiseRead, Scene};
use crate::world::folk::talks::{self, Move, PromiseKind, Subject, Topic};
use crate::world::{Stage, World};

/// The lane's check sets: the game reads them, and this test lands the
/// exploits on a real talk.
const SET: &str = include_str!("../../../afm-game/src/live_talk/checks/one_on_one.json");

/// A college player of yours at a given position, with his role.
fn yours(w: &mut World, like: Option<usize>, role: PlayingTime) -> (usize, u32) {
    let p = w.seat.and_then(|s| s.program).unwrap();
    if !w.people.iter().any(|q| q.stage == Stage::HighSchool) {
        w.add_class_aged(19);
    }
    let year = w.year;
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.last_role = Some(role);
        q.trust = 0.55;
        q.mood = -0.1;
    }
    if let Some(j) = like {
        w.people[i].player.position = w.people[j].player.position;
    }
    w.programs[p].roster.push(i);
    let id = w.people[i].player.id;
    for (_, ids) in w.seat_depth.iter_mut() {
        ids.retain(|&x| x != id);
    }
    (i, id)
}

/// A world with your backup (hurt this week) and the starter at his spot.
fn room() -> (World, u32, u32) {
    let mut w = seat("college");
    let (i, him) = yours(&mut w, None, PlayingTime::Backup);
    let (_, mate) = yours(&mut w, Some(i), PlayingTime::Starter);
    let today = w.today;
    w.training.hurt.push(Hurt {
        id: him,
        kind: None,
        from: today.add(-2),
        until: today.add(9),
        how: "a tight hamstring".into(),
    });
    (w, him, mate)
}

fn talk(w: &mut World, him: u32, subject: Subject) -> u32 {
    let id = talks::start_talk(w, him, subject).unwrap();
    scene::open(w, id).unwrap();
    id
}

fn scene_of(w: &World, id: u32) -> Scene {
    w.folk.scenes.iter().find(|s| s.id == id).unwrap().clone()
}

fn typed(code: u32, tone: &str, words: &str) -> Heard {
    Heard { pick: code, tone: tone.into(), words: words.into(), ..Heard::default() }
}

fn bytes(w: &World) -> Vec<u8> {
    rmp_serde::to_vec_named(w).unwrap()
}

/// A world as saved, loaded again: every arm of a test starts from the same
/// save (an in-memory world and its save can read apart).
fn load(b: &[u8]) -> World {
    rmp_serde::from_slice(b).unwrap()
}

fn untyped(w: &mut World) {
    for s in &mut w.folk.scenes {
        for l in &mut s.lines {
            l.typed = None;
        }
    }
}

fn move_of(code: u32) -> Option<Move> {
    (100..190).contains(&code).then(|| Move::ALL[((code - 100) / 10) as usize])
}

/// What a talk has moved so far: his trust plus his mood.
fn gain(w: &World, id: u32) -> f32 {
    let s = scene_of(w, id);
    s.f[0] + s.f[1]
}

const TONES: [&str; 4] = ["calm", "warm", "firm", "fiery"];

#[test]
fn a_one_on_one_takes_a_typed_line() {
    let (mut w, him, _) = room();
    let id = talk(&mut w, him, Subject::Role);
    assert!(scene::takes_typed(&scene_of(&w, id)), "the box shows in a one-on-one");
    const { assert!(talks::ONE_ON_ONE_TYPED) };
}

#[test]
fn a_typed_move_is_the_clicked_one_in_any_tone() {
    let (w0, him, _) = room();
    let base = bytes(&w0);
    let mut b0 = load(&base);
    let id = talk(&mut b0, him, Subject::Role);
    let picks: Vec<u32> = scene_of(&b0, id).picks.iter().map(|p| p.code).collect();
    assert!(picks.len() >= 3, "{picks:?}");
    for &code in &picks {
        for (k, tone) in TONES.iter().enumerate() {
            let (mut a, mut b) = (load(&base), load(&base));
            let (ia, ib) = (talk(&mut a, him, Subject::Role), talk(&mut b, him, Subject::Role));
            // The chip in that tone (the screen lets any tone of a move be
            // taken), against the line read as that move in that tone.
            let click = move_of(code).map_or(code, |_| code - code % 10 + k as u32);
            // Ending the talk with no line of yours writes one for your
            // words (the core's rule), so the worlds differ by that line.
            if move_of(code).is_none() && k > 0 || code == 900 {
                continue;
            }
            scene::answer(&mut a, ia, click).unwrap();
            let said = scene::say(&mut b, ib, typed(code, tone, "Look, here's where it is with you."), true);
            let said = said.unwrap_or_else(|e| panic!("{code} {tone}: {e}"));
            assert!(said.heard.is_none());
            untyped(&mut b);
            assert!(bytes(&a) == bytes(&b), "{code} in a {tone} tone: typed is the chip");
        }
    }
}

#[test]
fn a_line_on_another_point_is_its_chip_then_the_move() {
    let (w0, him, _) = room();
    let base = bytes(&w0);
    let mut probe = load(&base);
    let id = talk(&mut probe, him, Subject::Role);
    let turns = talks::typed_turns(&probe, &scene_of(&probe, id));
    let now = Topic::of(scene_of(&probe, id).b);
    assert!(turns.iter().any(|t| t.topic == Topic::Subject(Subject::Team)), "{turns:?}");
    assert!(turns.iter().all(|t| t.topic != now), "never the talk's own topic");
    assert!(turns.iter().filter(|t| t.mv.is_some() && !t.fin).count() >= 3, "{turns:?}");
    for t in turns.iter().filter(|t| t.mv.is_some() && !t.fin) {
        let (mut a, mut b) = (load(&base), load(&base));
        let (ia, ib) = (talk(&mut a, him, Subject::Role), talk(&mut b, him, Subject::Role));
        talks::change_topic(&mut a, ia, t.topic).unwrap();
        scene::answer(&mut a, ia, t.code.unwrap()).unwrap();
        let code = talks::typed_turn(&mut b, ib, t.topic, t.mv).unwrap().expect("the move is on the new table");
        assert_eq!(Some(code), t.code);
        let tone = scene_of(&b, ib).picks.iter().find(|p| p.code == code).unwrap().tone.clone();
        scene::say(&mut b, ib, typed(code, &tone, "Different thing. I want you leading this room."), false).unwrap();
        untyped(&mut b);
        assert!(bytes(&a) == bytes(&b), "{}: typed is the topic chip, then the move", t.label);
    }
}

#[test]
fn a_typed_repeat_counts_as_the_clicked_repeat() {
    // The same move on the same topic in a second talk, a week on: typed,
    // it adds what the clicked repeat adds and no more.
    let (w0, him, _) = room();
    let base = bytes(&w0);
    let (mut a, mut b) = (load(&base), load(&base));
    // He is hurt: the talk is on his injury, a point (`Talked` keeps it).
    let first_move = |w: &mut World, id: u32| loop {
        let s = scene_of(w, id);
        if let Some(p) = s.picks.iter().find(|p| move_of(p.code).is_some()) {
            break p.code;
        }
        // He asked something first: the same answer in both arms.
        scene::answer(w, id, s.picks[0].code).unwrap();
    };
    let mut code = 0;
    for w in [&mut a, &mut b] {
        let id = talk(w, him, Subject::Role);
        assert!(matches!(Topic::of(scene_of(w, id).b), Topic::Point(_)), "on a point");
        code = first_move(w, id);
        scene::answer(w, id, code).unwrap();
        if !scene_of(w, id).done {
            scene::answer(w, id, 900).unwrap();
        }
        w.today = w.today.add(3);
    }
    let (ia, ib) = (talk(&mut a, him, Subject::Role), talk(&mut b, him, Subject::Role));
    let (ca, cb) = (first_move(&mut a, ia), first_move(&mut b, ib));
    assert_eq!((ca, cb), (code, code), "the same move again");
    scene::answer(&mut a, ia, code).unwrap();
    let tone = scene_of(&b, ib).picks.iter().find(|p| p.code == code).unwrap().tone.clone();
    scene::say(&mut b, ib, typed(code, &tone, "Same thing as last time, I mean it."), false).unwrap();
    assert_eq!(gain(&a, ia), gain(&b, ib), "the same gain");
    untyped(&mut b);
    assert!(bytes(&a) == bytes(&b));
}

/// A reader's claims in the world's ids: p1 is him, p2 the starter at his
/// spot.
fn stated(c: &Value, him: u32, mate: u32) -> claims::Stated {
    claims::Stated {
        kind: c["kind"].as_str().unwrap().to_string(),
        about: match c["person"].as_str() {
            Some("p1") => Some(him),
            Some("p2") => Some(mate),
            _ => None,
        },
        value: c["value"].as_str().unwrap_or("").to_string(),
    }
}

/// The move an option on the set's card means, from its words' brackets.
fn card_move(set: &Value, code: u64) -> Option<Move> {
    let o = set["card"]["options"].as_array()?.iter().find(|o| o["code"].as_u64() == Some(code))?;
    let t = o["text"].as_str()?;
    let name = t.rsplit_once('(')?.1.trim_end_matches(')');
    Move::ALL.into_iter().find(|m| m.name() == name)
}

#[test]
fn no_exploit_lands_above_the_best_chip_and_every_false_fact_is_caught() {
    let set: Value = serde_json::from_str(SET).unwrap();
    let exploits = set["exploits"].as_array().unwrap();
    assert_eq!(exploits.len(), 40);
    let (w0, him, mate) = room();
    let base = bytes(&w0);
    let (mut landed, mut caught, mut promised) = (0, 0, 0);
    for e in exploits {
        let text = e["text"].as_str().unwrap();
        // A line that is no line (or means nothing on the table) lands
        // nothing: the reader's check set scores that part.
        let Some(m) = e["want"].as_u64().and_then(|c| card_move(&set, c)) else { continue };
        // The talk on a subject that offers the move.
        // The talk on a topic that offers the move: his points first, then
        // the subjects; none in this room, and it can't land here.
        let w0 = load(&base);
        let topics: Vec<Topic> =
            talks::subjects_for(&w0, him).into_iter().filter_map(|(k, _)| Topic::from_key(k)).collect();
        let on = |w: &mut World, t: Topic| {
            let id = talk(w, him, Subject::Role);
            if Topic::of(scene_of(w, id).b) != t {
                talks::change_topic(w, id, t).unwrap();
            }
            id
        };
        let Some(subject) = topics.into_iter().find(|&t| {
            let mut p = load(&base);
            let id = on(&mut p, t);
            scene_of(&p, id).picks.iter().any(|p| move_of(p.code) == Some(m))
        }) else {
            continue;
        };
        let talk = on;
        let mut probe = load(&base);
        let id = talk(&mut probe, subject);
        let table = scene_of(&probe, id).picks.clone();
        // The best chip: every move on the table in every tone.
        let mut best = f32::MIN;
        for p in table.iter().filter(|p| move_of(p.code).is_some()) {
            for k in 0..4 {
                let mut a = load(&base);
                let ia = talk(&mut a, subject);
                scene::answer(&mut a, ia, p.code - p.code % 10 + k).unwrap();
                best = best.max(gain(&a, ia));
            }
        }
        let code = table.iter().find(|p| move_of(p.code) == Some(m)).unwrap().code;
        let mut b = load(&base);
        let ib = talk(&mut b, subject);
        let mut h = typed(code, e["tone"].as_str().unwrap_or("calm"), text);
        h.facts = e["claims"].as_array().unwrap().iter().map(|c| stated(c, him, mate)).collect();
        if let Some(p) = e["promise"].as_object() {
            h.promise = Some(PromiseRead {
                kind: p["kind"].as_str().unwrap().into(),
                by: p["by"].as_str().unwrap().replace("none", ""),
                condition: p["condition"].as_str().unwrap().replace("none", ""),
                condition_words: p["condition_words"].as_str().unwrap().into(),
                ..PromiseRead::default()
            });
        }
        let s = scene_of(&b, ib);
        let truths: Vec<Truth> = h.facts.iter().map(|f| check::fact(&b, &s, f)).collect();
        let before = b.folk.promises.len();
        let said = scene::say(&mut b, ib, h.clone(), true).unwrap_or_else(|e| panic!("{text}: {e}"));
        landed += 1;
        let g = gain(&b, ib);
        assert!(g <= best + 1e-6, "{text}: {g} above the best chip {best}");
        // A false fact he can check as you say it is caught then: the move
        // rests against the facts (praise excepted, priced by its truth).
        let lie = truths.contains(&Truth::False) && m != Move::Praise;
        let shows = said.outcome.iter().any(|o| o.contains("isn't so"));
        assert_eq!(lie, shows, "{text}: {truths:?}");
        caught += lie as u32;
        // No lie is saved in a one-on-one: everything he can check, he
        // checks now; where he stands is a promise.
        assert!(b.folk.live.lies.is_empty(), "{text}");
        let made = b.folk.promises.len() - before;
        assert!(made <= 1, "{text}: one promise at most");
        if truths.contains(&Truth::Later) || h.promise.is_some() {
            assert_eq!(made, 1, "{text}: a claim about where he stands or a promise is logged");
            promised += 1;
        }
    }
    assert!(landed >= 15, "{landed}");
    assert!(caught >= 6, "the set's lies are false in this room: {caught}");
    assert!(promised >= 5, "{promised}");
}

#[test]
fn a_typed_promise_takes_the_readings_kind_and_is_logged_once() {
    let (mut w, him, _) = room();
    let id = talk(&mut w, him, Subject::Role);
    let code = scene_of(&w, id).picks.iter().find(|p| move_of(p.code) == Some(Move::Promise)).unwrap().code;
    let mut h = typed(code, "warm", "I'm not going anywhere, and neither is your spot here.");
    h.promise = Some(PromiseRead { kind: "stay".into(), by: "next_season".into(), ..PromiseRead::default() });
    let before = w.folk.promises.len();
    scene::say(&mut w, id, h, true).unwrap();
    assert_eq!(w.folk.promises.len(), before + 1, "one promise, not two");
    assert_eq!(w.folk.promises.last().unwrap().kind, PromiseKind::Stay, "the reading's kind, not the chip's");
}

#[test]
fn a_typed_promise_is_judged_on_its_day_in_a_save_loaded_with_no_key() {
    let (mut w, him, _) = room();
    let id = talk(&mut w, him, Subject::Role);
    // "You're my starter": a claim about where he stands, heard as a
    // promise about what he'll see (the promise design, section 11).
    let code = scene_of(&w, id).picks.iter().find(|p| move_of(p.code) == Some(Move::Straight)).unwrap().code;
    let mut h = typed(code, "calm", "You're still my starter. Nothing changed.");
    h.facts = vec![claims::Stated { kind: "depth_rank".into(), about: Some(him), value: "starter".into() }];
    scene::say(&mut w, id, h, false).unwrap();
    let k = w.folk.promises.len() - 1;
    let due = w.folk.promises[k].due;
    assert_eq!(w.folk.promises[k].kind, PromiseKind::Start);
    // Saved and loaded: the typed words are only on the log; nothing of the
    // reading is kept, and the game plays on with no key.
    let saved = bytes(&w);
    let mut back: World = load(&saved);
    let mut twin: World = load(&saved);
    assert!(back.folk.scenes.iter().all(|s| s.heard.is_none()));
    let k2 = back.folk.promises.len() - 1;
    assert_eq!(back.folk.promises[k2].kept, None);
    let trust = back.people[crate::world::folk::person(&back, him).unwrap()].trust;
    let start = back.today;
    for x in [&mut back, &mut twin] {
        let mut d = start;
        while d <= due {
            x.today = d;
            talks::day(x, d);
            d = d.add(1);
        }
    }
    let p = &back.folk.promises[k2];
    assert!(p.kept.is_some(), "judged on its day");
    assert!(back.folk.promises[k2].due <= due && twin.folk.promises[k2].kept == p.kept, "the same verdict twice");
    let after = back.people[crate::world::folk::person(&back, him).unwrap()].trust;
    assert_eq!(after, twin.people[crate::world::folk::person(&twin, him).unwrap()].trust);
    // A day before it was due, it waited.
    let mut early: World = load(&saved);
    let mut d = early.today;
    while d < due {
        early.today = d;
        talks::day(&mut early, d);
        d = d.add(1);
    }
    assert_eq!(early.folk.promises[k2].kept, None, "not before its day");
    let _ = (trust, Kind::OneOnOne);
}
