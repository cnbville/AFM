//! Lane 9 on the fake and with no key: a typed line in a one-on-one lands
//! on the move it means in the tone it was said, a line on another point
//! turns the talk there first, a promise asks first, the writer's card
//! carries the talk's topic, and with no key the chips play as ever.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use afm_ai::Fake;
use afm_ai::fake::body;
use afm_engine::player::PlayingTime;
use afm_engine::world::data::Division;
use afm_engine::world::folk::scene::Scene;
use afm_engine::world::folk::talks::{Move, Subject, Topic};
use afm_engine::world::seat::{Job, You};
use afm_engine::world::{Stage, World, WorldOptions};
use serde_json::{Value, json};

use super::{TURN, card, decode_turn};
use crate::Game;
use crate::live_talk::say::{READER, WRITER};

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

/// The tiny world with your career at a college and a backup of yours.
fn world() -> (World, u32) {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    let job = Job::College(w.programs.iter().position(|p| p.division == Division::FBS).unwrap());
    w.start_career(&you(), job, 7);
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let q = &mut w.people[i];
    q.stage = Stage::College;
    q.college = Some(p);
    q.class_of = year - 1;
    q.college_seasons = 1;
    q.last_role = Some(PlayingTime::Backup);
    q.trust = 0.55;
    w.programs[p].roster.push(i);
    let id = w.people[i].player.id;
    (w, id)
}

fn game(name: &str, fake: bool) -> (Game, Option<Arc<Fake>>, u32) {
    let p: PathBuf = std::env::temp_dir().join(format!("afm-live-1on1-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    let mut g = Game::new(p);
    let (w, him) = world();
    g.world = Some(w);
    let f = fake.then(|| {
        let f = Arc::new(Fake::new());
        g.ai.use_fake(f.clone());
        f
    });
    (g, f, him)
}

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    g.call(cmd, &args).unwrap_or_else(|e| panic!("{cmd}: {e}"))
}

fn finish(g: &mut Game, ticket: u64) -> Value {
    let t = Instant::now();
    loop {
        let v = ok(g, "ai_poll", json!({ "ticket": ticket }));
        if v["state"] != "wait" || t.elapsed() > Duration::from_secs(5) {
            return v;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn reading(pick: &str, tone: &str) -> Value {
    let v = json!({ "pick": pick, "also": "none", "tone": tone, "promise": null, "claims": [], "named": [], "fit": "clear" });
    body(&v.to_string(), 900, 60)
}

fn scene_of(g: &Game, id: u32) -> Scene {
    g.world.as_ref().unwrap().folk.scenes.iter().find(|s| s.id == id).unwrap().clone()
}

/// A one-on-one opened through the screen.
fn talk(g: &mut Game, him: u32, subject: &str) -> u32 {
    let sid = ok(g, "people_talk", json!({ "id": him, "subject": subject }))["scene"].as_u64().unwrap() as u32;
    ok(g, "people_scene", json!({ "id": sid }));
    sid
}

/// The reader's tag for a code on the scene's card as it stands.
fn tag(g: &Game, id: u32, code: u32) -> String {
    let w = g.world.as_ref().unwrap();
    let (_, keys) = card(w, &scene_of(g, id)).unwrap();
    keys.option_tag(code).unwrap().to_string()
}

fn move_of(code: u32) -> Option<Move> {
    (100..190).contains(&code).then(|| Move::ALL[((code - 100) / 10) as usize])
}

#[test]
fn a_typed_line_lands_on_its_move_in_its_tone() {
    let (mut g, f, him) = game("move", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "role");
    let v = ok(&mut g, "people_scene", json!({ "id": id }));
    assert_eq!(v["say"]["cmd"], "people_say", "the box shows: {v}");
    let ask = scene_of(&g, id).picks.iter().find(|p| move_of(p.code) == Some(Move::Ask)).unwrap().code;
    f.push(READER, Ok(reading(&tag(&g, id, ask), "fiery")));
    f.push(WRITER, Ok(body("Honestly, Coach? I'm frustrated, but I'm here.", 600, 30)));
    let t = ok(&mut g, "people_say", json!({ "id": id, "text": "What's really eating at you, man? Talk to me." }))["ticket"]
        .as_u64()
        .unwrap();
    let v = finish(&mut g, t);
    assert_eq!(v["state"], "done", "{v}");
    assert!(v["heard"].as_str().unwrap().ends_with("fiery"), "{v}");
    let s = scene_of(&g, id);
    let mine = s.lines.iter().rev().find(|l| l.by == 1).unwrap();
    assert_eq!(mine.typed.as_deref(), Some("What's really eating at you, man? Talk to me."));
    // Said fiery, it is the Ask move in the fiery tone (code 103).
    assert!(s.c & (1 << 2) != 0, "the Ask move is used");
    assert!(s.picks.iter().all(|p| move_of(p.code) != Some(Move::Ask)), "a move counts once a talk");
    // The writer's card carried the talk's topic and how it went over.
    let asked = f.asked();
    let writer = asked.iter().rev().find(|r| r.label == WRITER).unwrap();
    assert!(writer.user.contains("The talk is about: his role"), "{}", writer.user);
}

#[test]
fn a_line_on_another_point_turns_the_talk_and_a_promise_asks_first() {
    let (mut g, f, him) = game("turn", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "role");
    let w = g.world.as_ref().unwrap();
    let (c, keys) = card(w, &scene_of(&g, id)).unwrap();
    assert!(c.options.iter().any(|o| o.tag.starts_with('t')), "turns on the card");
    assert!(c.options.len() <= crate::live_talk::card::MAX_OPTIONS + super::MAX_TURNS);
    // The leadership council: the team's Hand move.
    let (t, code) = keys
        .options
        .iter()
        .find(|(_, c)| decode_turn(*c) == Some((Topic::Subject(Subject::Team), Some(Move::Hand))))
        .cloned()
        .expect("a turn to the team, handing him a council seat");
    assert!(code >= TURN);
    f.push(READER, Ok(reading(&t, "warm")));
    f.push(WRITER, Ok(body("I won't let you down, Coach.", 600, 30)));
    let ticket = ok(
        &mut g,
        "people_say",
        json!({ "id": id, "text": "Different subject. I want you on the leadership council." }),
    )["ticket"]
        .as_u64()
        .unwrap();
    let v = finish(&mut g, ticket);
    assert_eq!(v["state"], "done", "{v}");
    let s = scene_of(&g, id);
    assert_eq!(Topic::of(s.b), Topic::Subject(Subject::Team), "the talk turned");
    let w = g.world.as_ref().unwrap();
    assert!(w.folk.rules.council.contains(&him), "the council seat, as its chip gives it");
    // A promise asks first: its readings come back to confirm.
    let promise = s.picks.iter().find(|p| move_of(p.code) == Some(Move::Promise)).map(|p| p.code);
    if let Some(code) = promise {
        f.push(READER, Ok(reading(&tag(&g, id, code), "calm")));
        let ticket =
            ok(&mut g, "people_say", json!({ "id": id, "text": "Your place here is safe with me." }))["ticket"]
                .as_u64()
                .unwrap();
        let v = finish(&mut g, ticket);
        assert_eq!(v["state"], "unsure", "{v}");
        assert_eq!(v["readings"][0]["final"], true);
        let before = g.world.as_ref().unwrap().folk.promises.len();
        let t2 =
            ok(&mut g, "people_say", json!({ "id": id, "ticket": ticket, "pick": code }))["ticket"].as_u64().unwrap();
        f.push(WRITER, Ok(body("I'll hold you to that.", 500, 20)));
        assert_eq!(finish(&mut g, t2)["state"], "done");
        assert_eq!(g.world.as_ref().unwrap().folk.promises.len(), before + 1, "one promise");
    }
}

#[test]
fn with_no_key_a_one_on_one_has_no_box_and_the_chips_play() {
    let (mut g, _, him) = game("nokey", false);
    let id = talk(&mut g, him, "role");
    let v = ok(&mut g, "people_scene", json!({ "id": id }));
    assert!(v.get("say").is_none(), "{v}");
    assert_eq!(g.call("people_say", &json!({ "id": id, "text": "Hello" })), Err("No key is set.".into()));
    let code = scene_of(&g, id).picks[0].code;
    assert!(ok(&mut g, "people_answer", json!({ "id": id, "code": code }))["lines"].is_array());
}
