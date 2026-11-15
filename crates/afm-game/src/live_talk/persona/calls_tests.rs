//! Lane N5b on the fake: a call to a pro's agent plays as a v3 talk. His
//! number said back is a yes that waits on your confirm; the confirm signs
//! him through the desk. With no key there is no call.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use afm_ai::Fake;
use afm_ai::fake::body;
use afm_engine::world::folk::persona::negotiate::contract;
use afm_engine::world::front_office::nfl;
use afm_engine::world::pro::{Status, money};
use afm_engine::world::seat::{Job, You};
use afm_engine::world::{World, WorldOptions};
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

/// The tiny world with you running an NFL club, and a veteran of yours
/// with an agent whose deal just ran out.
fn world() -> (World, usize, u32) {
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

fn game(name: &str, fake: bool) -> (Game, Option<Arc<Fake>>, usize, u32) {
    let p: PathBuf = std::env::temp_dir().join(format!("afm-live-n5b-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    let mut g = Game::new(p);
    let (w, t, id) = world();
    g.world = Some(w);
    let f = fake.then(|| {
        let f = Arc::new(Fake::new());
        g.ai.use_fake(f.clone());
        f
    });
    (g, f, t, id)
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

#[test]
fn a_call_to_his_agent_signs_him_on_your_confirm() {
    let (mut g, f, t, id) = game("agent", true);
    let f = f.unwrap();
    // His contract page offers the call.
    let page = ok(&mut g, "fo_talk", json!({ "id": id }));
    assert_eq!(page["call"]["cmd"], "talk_agent", "{}", page["call"]);
    let sid = ok(&mut g, "talk_agent", json!({ "id": id }))["scene"].as_u64().unwrap() as u32;
    let scene = ok(&mut g, "people_scene", json!({ "id": sid }));
    assert!(scene.to_string().contains("on the line"), "{scene}");
    let ask = nfl::asking(g.world.as_ref().unwrap(), id).unwrap();
    let gives = vec![
        json!({ "give": "money", "number": ask.apy, "who": "" }),
        json!({ "give": "years", "number": ask.years, "who": "" }),
        json!({ "give": "guaranteed", "number": ask.total() * ask.gtd, "who": "" }),
    ];
    let act = json!({ "act": "offer", "topic": "none", "about": "", "claim": "none", "value": "", "deal": "Contract",
        "what": "", "when": "", "words": "his number", "gives": gives, "terms": [] });
    let v = json!({ "acts": [act], "hits": [], "dodged": false, "not_talk": false });
    f.push(EAR, Ok(body(&v.to_string(), 1200, 120)));
    f.push(VOICE, Ok(body(&json!({ "line": "You have a deal.", "colour": "" }).to_string(), 900, 60)));
    let tk =
        ok(&mut g, "people_say", json!({ "id": sid, "text": "We'll meet your number." }))["ticket"].as_u64().unwrap();
    let v = finish(&mut g, tk);
    assert_eq!(v["state"], "done", "{v}");
    // The ear was told who his client is.
    let asked = f.asked();
    let name = {
        let w = g.world.as_ref().unwrap();
        w.people[afm_engine::world::folk::person(w, id).unwrap()].name.clone()
    };
    assert!(asked.iter().any(|r| r.label == EAR && r.user.contains(&name)), "the client is on the ear's card");
    // Nothing signed before the confirm.
    let w = g.world.as_ref().unwrap();
    let i = afm_engine::world::folk::person(w, id).unwrap();
    assert!(w.people[i].pro.as_ref().unwrap().contract.as_ref().unwrap().last_year() < w.year);
    let c = &v["confirm"][0];
    assert!(c["text"].as_str().unwrap().starts_with("Make the offer: "), "{v}");
    ok(&mut g, "talk_confirm", c["args"].clone());
    let w = g.world.as_ref().unwrap();
    let k = w.people[i].pro.as_ref().unwrap().contract.as_ref().unwrap();
    assert!(k.team == t && k.last_year() >= w.year, "signed on the confirm");
}

#[test]
fn with_no_key_there_is_no_call() {
    let (mut g, _, _, id) = game("off", false);
    assert!(g.call("talk_agent", &json!({ "id": id })).is_err());
    let page = ok(&mut g, "fo_talk", json!({ "id": id }));
    assert!(page.get("call").is_none());
}
