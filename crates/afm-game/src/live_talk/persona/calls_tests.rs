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

/// Lane N7's sweep on the calls: a line that begs for every hidden thing,
/// to his agent and to another club's GM, sends cards of words only.
#[test]
fn the_agent_and_gm_calls_send_cards_of_words_only() {
    use crate::live_talk::persona::talk::n7_tests::{BAIT, assert_clean, last_cards, no_hidden_key};
    let (mut g, f, t, id) = game("sweep", true);
    let f = f.unwrap();
    let plain = |f: &Fake| {
        let quiet = json!({ "acts": [], "hits": [], "dodged": false, "not_talk": false });
        f.push(EAR, Ok(body(&quiet.to_string(), 900, 60)));
        f.push(
            VOICE,
            Ok(body(&json!({ "line": "Let's keep this about the deal.", "colour": "" }).to_string(), 600, 40)),
        );
    };
    let sweep = |g: &mut Game, facet: &str, sid: u32| {
        plain(&f);
        let tk = ok(g, "people_say", json!({ "id": sid, "text": BAIT }))["ticket"].as_u64().unwrap();
        let v = finish(g, tk);
        assert_eq!(v["state"], "done", "{facet}: {v}");
        let (e, vc) = last_cards(&f);
        assert_clean(facet, "ear", &e);
        assert_clean(facet, "voice", &vc);
        no_hidden_key(facet, &v, "");
    };
    let sid = ok(&mut g, "talk_agent", json!({ "id": id }))["scene"].as_u64().unwrap() as u32;
    sweep(&mut g, "agent", sid);
    // Another club's GM, about one of his men.
    let (club, his) = {
        let w = g.world.as_ref().unwrap();
        let club = (0..w.clubs.len())
            .find(|&c| c != t && afm_engine::world::franchise::active_in(w, c, w.year) && !w.clubs[c].roster.is_empty())
            .unwrap();
        (club, w.people[w.clubs[club].roster[0]].player.id)
    };
    let sid = ok(&mut g, "talk_gm", json!({ "club": club, "id": his }))["scene"].as_u64().unwrap() as u32;
    sweep(&mut g, "gm", sid);
}

/// A promise of a pick (Cain 2026-11-15): "we'll take you in the first
/// round" is a real promise to a prospect, judged by the draft: kept if
/// your club took him by that round, broken if he was still there at one
/// of your picks by then, void if he was gone before your first chance or
/// never entered the draft.
#[test]
fn a_promise_of_a_pick_is_kept_broken_or_void_by_the_draft() {
    use afm_engine::world::Stage;
    use afm_engine::world::folk::Name;
    use afm_engine::world::folk::persona::{Them, makeup};
    use afm_engine::world::folk::scene::PromiseRead;
    use afm_engine::world::folk::talks::{self, DraftCall, PromiseKind, PromiseTo, draft_call, draft_round};
    use afm_engine::world::pro::ProCareer;
    assert_eq!(
        (draft_round("in the first round"), draft_round("2nd"), draft_round("round 3"), draft_round("")),
        (1, 2, 3, 7)
    );
    let (mut w, t, _) = world();
    assert!(w.today < w.dates.draft[0].date, "the test world sits before the draft");
    // The test world holds only pros: one of another club's men stands in
    // as a college senior.
    let other = (0..w.clubs.len()).find(|&c| c != t && !w.clubs[c].roster.is_empty()).unwrap();
    let prospect = w.clubs[other].roster.pop().unwrap();
    w.people[prospect].stage = Stage::College;
    w.people[prospect].pro = None;
    let id = w.people[prospect].player.id;
    let made = w.today;
    let said = PromiseRead { kind: "draft".into(), condition_words: "1".into(), ..PromiseRead::default() };
    talks::typed_promise(&mut w, 0, id, &said);
    let k = w.folk.promises.len() - 1;
    assert_eq!((w.folk.promises[k].to, w.folk.promises[k].kind), (PromiseTo::Player(id), PromiseKind::Draft(1)));
    let due = w.folk.promises[k].due;
    assert_eq!(due, w.dates.draft[2].date.add(1), "judged the day after the draft");
    // Your first pick is the 20th, in round 1 (a man of yours stands in).
    let year = w.dates.draft[2].date.year();
    let mine = w.clubs[t].roster[0];
    w.people[mine].pro.as_mut().unwrap().draft = Some((year, 1, 20, t));
    let base: ProCareer = (**w.people[mine].pro.as_ref().unwrap()).clone();
    let went = |w: &mut afm_engine::world::World, d: Option<(i32, u32, u32, usize)>, stage: Stage| {
        let mut c = base.clone();
        c.draft = d;
        w.people[prospect].pro = d.map(|_| Box::new(c));
        w.people[prospect].stage = stage;
        draft_call(w, prospect, 1, made)
    };
    assert_eq!(went(&mut w, Some((year, 1, 20, t)), Stage::Pro), DraftCall::Kept);
    assert_eq!(went(&mut w, Some((year, 1, 8, other)), Stage::Pro), DraftCall::Void, "gone before your pick");
    assert_eq!(went(&mut w, Some((year, 1, 25, other)), Stage::Pro), DraftCall::Broken, "you passed on him");
    assert_eq!(went(&mut w, Some((year, 2, 52, t)), Stage::Pro), DraftCall::Broken, "a round later than you said");
    assert_eq!(went(&mut w, None, Stage::Declared), DraftCall::Broken, "undrafted");
    assert_eq!(went(&mut w, None, Stage::College), DraftCall::Void, "he went back to school");
    // Broken, and he went elsewhere: his trust in you and your name with
    // the agents drop; his new club takes no grudge.
    went(&mut w, Some((year, 1, 25, other)), Stage::Pro);
    let key = makeup::key(Them::Player(id));
    let (trust, agents) = (w.folk.mine.trust_of(key).unwrap_or(0.5), w.folk.mine.name(Name::Agents));
    w.today = due;
    talks::check_promises(&mut w, due);
    assert_eq!(w.folk.promises.iter().rev().find(|p| p.to == PromiseTo::Player(id)).unwrap().kept, Some(false));
    assert!(w.folk.mine.trust_of(key).unwrap() < trust, "his trust in you drops");
    assert!(w.folk.mine.name(Name::Agents) < agents, "the agents hear it");
    // After the draft, a pick can't be promised.
    let before = w.folk.promises.len();
    talks::typed_promise(&mut w, 0, id, &said);
    assert_eq!(w.folk.promises.len(), before);
}
