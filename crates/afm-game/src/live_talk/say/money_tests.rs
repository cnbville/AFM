//! The money desks on the fake: a line typed on a pro's contract page plays
//! on the call with his agent, and the next line picks the same call up; a
//! line on the trade desk calls their GM; a line on a recruit's NIL promise
//! is a call to him by the contact rules. With no key there is no box, and
//! no chips anywhere.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use afm_ai::Fake;
use afm_ai::fake::body;
use afm_engine::world::data::Division;
use afm_engine::world::folk::persona::negotiate::{AGENT, GM, contract};
use afm_engine::world::folk::scene::Kind;
use afm_engine::world::front_office::college as fc;
use afm_engine::world::front_office::nfl;
use afm_engine::world::pro::{Status, money};
use afm_engine::world::seat::{Job, You};
use afm_engine::world::{Stage, World, WorldOptions};
use serde_json::{Value, json};

use crate::Game;
use crate::live_talk::persona::{ear::EAR, voice::VOICE};

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

fn home(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("afm-live-desks-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// You run an NFL club; a veteran of yours with an agent whose deal just
/// ran out: (the world, your club, his id).
fn nfl_world() -> (World, usize, u32) {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    let t = (0..w.clubs.len())
        .filter(|&t| afm_engine::world::franchise::active_in(&w, t, w.year))
        .max_by_key(|&t| w.clubs[t].roster.len())
        .unwrap();
    w.start_career(&you(), Job::Nfl(t), 7);
    w.today = w.dates.tag_window_open;
    let i = *w.clubs[t]
        .roster
        .iter()
        .filter(|&&i| w.people[i].pro.as_ref().unwrap().status == Status::Roster && contract::has_agent(&w, i))
        .find(|&&i| (4.0e6..15.0e6).contains(&nfl::market_read(&w, i).0))
        .unwrap();
    let year = w.year;
    let c = w.people[i].pro.as_mut().unwrap();
    c.accrued = 5;
    c.contract = Some(money::build(t, money::Kind::Veteran, year - 3, &money::Terms::flat(3, 6.0e6)));
    let id = w.people[i].player.id;
    (w, t, id)
}

fn game(name: &str, w: World, fake: bool) -> (Game, Option<Arc<Fake>>) {
    let mut g = Game::new(home(name));
    g.world = Some(w);
    let f = fake.then(|| {
        let f = Arc::new(Fake::new());
        g.ai.use_fake(f.clone());
        f
    });
    (g, f)
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

/// The ear hears nothing it can use, and the voice says one line.
fn quiet(f: &Fake, line: &str) {
    let v = json!({ "acts": [], "hits": [], "dodged": false, "not_talk": false });
    f.push(EAR, Ok(body(&v.to_string(), 900, 60)));
    f.push(VOICE, Ok(body(&json!({ "line": line, "colour": "" }).to_string(), 900, 60)));
}

/// Sends a line on a desk's box: (the call's scene, his answer).
fn say_on(g: &mut Game, f: &Fake, flag: &Value, text: &str) -> (u32, Value) {
    quiet(f, "Let me think on that.");
    let mut args = flag["args"].clone();
    args["text"] = json!(text);
    let r = ok(g, flag["cmd"].as_str().unwrap(), args);
    let sid = r["scene"].as_u64().unwrap_or_else(|| panic!("no call: {r}")) as u32;
    let v = finish(g, r["ticket"].as_u64().unwrap());
    (sid, v)
}

#[test]
fn a_line_on_his_contract_page_plays_on_the_call_with_his_agent() {
    let (w, _, id) = nfl_world();
    let (mut g, f) = game("contract", w, true);
    let f = f.unwrap();
    let page = ok(&mut g, "fo_talk", json!({ "id": id }));
    assert_eq!(page["say"]["cmd"], "fo_say", "{page}");
    assert_eq!(page["say"]["args"]["desk"], "contract");
    assert!(page.get("facts").is_none(), "no chips");
    let (sid, v) = say_on(&mut g, &f, &page["say"], "We want him back. What's his number?");
    assert_eq!(v["state"], "done", "{v}");
    let w = g.world.as_ref().unwrap();
    let s = w.folk.scenes.iter().find(|s| s.id == sid).unwrap();
    assert!(s.kind == Kind::Negotiation && s.b == AGENT && s.a == u64::from(id));
    // The agent's call opened first, so its lede comes before your line.
    assert!(s.lines.len() >= 2, "{:?}", s.lines.len());
    // The ear was told who his client is.
    let name = w.people[afm_engine::world::folk::person(w, id).unwrap()].name.clone();
    assert!(f.asked().iter().any(|r| r.label == EAR && r.user.contains(&name)));
    // The next line picks up the same call.
    let (again, _) = say_on(&mut g, &f, &page["say"], "Think it over.");
    assert_eq!(again, sid);
}

#[test]
fn a_line_on_the_trade_desk_calls_their_gm() {
    let (w, t, _) = nfl_world();
    let club = (0..w.clubs.len()).find(|&c| c != t && afm_engine::world::franchise::active_in(&w, c, w.year)).unwrap();
    let (mut g, f) = game("trade", w, true);
    let f = f.unwrap();
    let page = ok(&mut g, "fo_trade_team", json!({ "team": club }));
    assert_eq!(page["say"]["args"]["desk"], "trade", "{page}");
    assert_eq!(page["say"]["args"]["club"], json!(club));
    let (sid, v) = say_on(&mut g, &f, &page["say"], "Are you open to moving down in the draft?");
    assert_eq!(v["state"], "done", "{v}");
    let w = g.world.as_ref().unwrap();
    let s = w.folk.scenes.iter().find(|s| s.id == sid).unwrap();
    assert!(s.kind == Kind::Negotiation && s.b == GM && s.c == club as i64);
}

#[test]
fn a_line_on_his_nil_promise_is_a_call_by_the_contact_rules() {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    let mine = w.programs.iter().position(|p| p.division == Division::FBS).unwrap();
    w.start_career(&you(), Job::College(mine), 7);
    w.add_class_aged(17);
    let i = (0..w.people.len())
        .filter(|&i| w.people[i].stage == Stage::HighSchool)
        .min_by_key(|&i| (w.people[i].class_of, i))
        .unwrap();
    let id = w.people[i].player.id;
    fc::add(&mut w, id).unwrap();
    fc::offer(&mut w, id).unwrap();
    let (mut g, f) = game("nil", w, true);
    let f = f.unwrap();
    let file = ok(&mut g, "recruit_file", json!({ "id": id }));
    let flag = &file["say"];
    assert_eq!(flag["args"]["desk"], "nil", "{file}");
    // A senior in 2037 may take a call today: the line plays on it.
    assert!(afm_engine::world::folk::reach::call_rule(g.world.as_ref().unwrap(), id).is_ok());
    let (sid, v) = say_on(&mut g, &f, flag, "We'd like to talk about what we can do for you and your family.");
    assert_eq!(v["state"], "done", "{v}");
    let s = g.world.as_ref().unwrap().folk.scenes.iter().find(|s| s.id == sid).unwrap().clone();
    assert!(s.kind == Kind::RecruitCall && s.a == u64::from(id));
    // The same call carries the next line.
    let (again, _) = say_on(&mut g, &f, flag, "Talk it over with your mom.");
    assert_eq!(again, sid);
    // Once that call is over, the contact rules hold: one call a day. The
    // box says so and keeps your words.
    let w = g.world.as_mut().unwrap();
    w.folk.scenes.iter_mut().find(|s| s.id == sid).unwrap().done = true;
    let mut args = flag["args"].clone();
    args["text"] = json!("One more thing.");
    let r = ok(&mut g, "fo_say", args);
    assert_eq!(r["state"], "none", "{r}");
    assert!(r["note"].as_str().is_some_and(|n| n.contains("today")), "{r}");
}

#[test]
fn with_no_key_the_desks_have_no_box_and_no_chips() {
    let (w, t, id) = nfl_world();
    let club = (0..w.clubs.len()).find(|&c| c != t && afm_engine::world::franchise::active_in(&w, c, w.year)).unwrap();
    let (mut g, _) = game("off", w, false);
    let page = ok(&mut g, "fo_talk", json!({ "id": id }));
    assert!(page.get("say").is_none() && page.get("facts").is_none() && page.get("call").is_none());
    let trade = ok(&mut g, "fo_trade_team", json!({ "team": club }));
    assert!(trade.get("say").is_none());
    assert_eq!(g.call("fo_say", &json!({ "desk": "contract", "id": id, "text": "Hi." })), Err("No key is set.".into()));
    // The desk's own box still works: his ask is on the page.
    assert!(!page["kind"].is_null());
}

#[test]
fn with_v3_off_the_desks_have_no_box() {
    let (w, _, id) = nfl_world();
    let (mut g, _) = game("v3off", w, true);
    g.ai.persona_talks = false;
    let page = ok(&mut g, "fo_talk", json!({ "id": id }));
    assert!(page.get("say").is_none());
    assert!(g.call("fo_say", &json!({ "desk": "contract", "id": id, "text": "Hi." })).is_err());
}
