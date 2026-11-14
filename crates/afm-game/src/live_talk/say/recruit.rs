//! The college desk typed (live talks lane 10, LT2 and LT3): home visits,
//! calls, messages, the family's call, the official visit's sit-down and a
//! transfer's call and visit. Typed lines land on the talk's own points
//! through the core; what this lane adds:
//!
//! - **A typed promise** lands on the promise it means (one of the five to
//!   a high school recruit, a starting job or a real role to a transfer),
//!   asked first, as the promise menu's chip would. So does a claim about
//!   where he will stand ("you'll start day one"), which is a promise about
//!   what he'll see (`heard`, `pitch::typed_promise`).
//! - **Typed facts** are checked by the engine (`claims::check::recruit`):
//!   a false one he can check now costs on the spot, one he can check only
//!   later is saved as a lie with its day.
//! - **The writer's card** carries his wants only as the reaction the game
//!   picked to the point you made (`writer`), never his own numbers.
//!
//! NIL forms (`fo_say` with desk "nil", LT9) come after live lane 7, whose
//! money facts they reuse: `say` and `facts` still answer "later update".

use afm_engine::world::World;
use afm_engine::world::folk::claims::check::recruit as truth;
use afm_engine::world::folk::claims::truth::STANDING;
use afm_engine::world::folk::pitch;
use afm_engine::world::folk::pulls::Ev;
use afm_engine::world::folk::scene::{Heard, PromiseRead, Scene};
use serde_json::Value;

use crate::Game;
use crate::live_talk::card::Card;

/// Adds this lane's own box to a reply (`live_talk::after`, only with a
/// usable key). A recruit's talk box is the core's scene box.
pub fn flag(_g: &Game, _cmd: &str, _v: &mut Value) {}

/// Adds what this lane's scenes need to the reader's card: nothing, since
/// the promises he can be made are among the talk's options.
pub fn reader(_w: &World, _s: &Scene, _card: &mut Card) {}

/// A typed promise to a recruit or a transfer, or a claim about where he
/// will stand, lands on the promise it means when the talk can reach one:
/// a promise binds you, so it is asked first (the promise picks are
/// final). A promise the talk can't make (one is made already, your offer
/// isn't out) changes nothing: the line lands where the reader put it.
pub fn heard(w: &World, s: &Scene, _reading: &Value, heard: &mut Heard) {
    let Some((_, id, ..)) = truth::who(w, s) else { return };
    let kind = match &heard.promise {
        Some(p) => p.kind.clone(),
        None => {
            let Some(f) = heard
                .facts
                .iter()
                .find(|f| STANDING.contains(&f.kind.as_str()) && (f.about.is_none() || f.about == Some(id)))
            else {
                return;
            };
            let v = f.value.to_lowercase();
            let first =
                ["start", "1st", "first", "the ones", "number one", "no. 1", "day one"].iter().any(|x| v.contains(x));
            let kind = if f.kind == "depth_rank" && first || f.kind == "ahead_of_him" { "start" } else { "role" };
            heard.promise = Some(PromiseRead { kind: kind.into(), claim: Some(f.clone()), ..PromiseRead::default() });
            kind.into()
        }
    };
    if let Some(code) = pitch::typed_promise(s, &kind) {
        heard.pick = code;
    }
}

/// The writer's card: his reaction to the point you made, as the game
/// picked it and his file kept it. Nothing else of what he wants.
pub fn writer(w: &World, s: &Scene, heard: &Heard, card: &mut Card) {
    let Some((_, id, ..)) = truth::who(w, s) else { return };
    let Some(g) = pitch::point_of(heard.pick).and_then(|k| k.group()) else { return };
    let row = w
        .folk
        .pulls
        .of(id)
        .filter(|r| r.on == w.today && r.group() == g)
        .filter(|r| matches!(r.ev, Ev::TookWell | Ev::Polite | Ev::Cool | Ev::Doubted))
        .last();
    if let Some(r) = row {
        card.released.push(reaction(r.ev).into());
    }
}

/// His reaction in words: what he shows, never why.
pub fn reaction(ev: Ev) -> &'static str {
    match ev {
        Ev::TookWell => "He says that's what he came for.",
        Ev::Polite => "He's polite about it, no more.",
        Ev::Cool => "He says it isn't his thing.",
        Ev::Doubted => "It matters to him, and he doesn't believe you can back it up.",
        _ => "",
    }
}

/// `fo_say` on a NIL form.
pub fn say(_g: &mut Game, _args: &Value) -> crate::Reply {
    Err(crate::live_talk::NOT_YET.into())
}

/// Question 5's fact chips on a NIL form, which need no key.
pub fn facts(_g: &Game, _cmd: &str, _v: &mut Value) {}

#[cfg(test)]
mod tests {
    //! Lane 10 on the fake: a typed promise to a recruit lands on the
    //! promise it means, asked first, even when the card's options don't
    //! list it; the writer's card carries his reaction and nothing else of
    //! what he wants.

    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use afm_ai::Fake;
    use afm_ai::fake::body;
    use afm_engine::world::data::Division;
    use afm_engine::world::folk::scene::{self, Kind, Scene, Who};
    use afm_engine::world::folk::{pledge, pulls};
    use afm_engine::world::front_office::college as fc;
    use afm_engine::world::seat::{Job, You};
    use afm_engine::world::{Stage, World, WorldOptions};
    use serde_json::{Value, json};

    use super::super::{READER, WRITER};
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

    fn home(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("afm-live-recruit-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// A game at a college on the fake, and a call to a senior holding
    /// your offer: (the game, the fake, the call, his place, your program).
    fn call(name: &str) -> (Game, Arc<Fake>, u32, usize, usize) {
        let mut w =
            World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
        let mine = w.programs.iter().position(|p| p.division == Division::FBS).unwrap();
        w.start_career(&you(), Job::College(mine), 7);
        w.add_class_aged(17);
        // A senior: the high school class that signs next.
        let i = (0..w.people.len())
            .filter(|&i| w.people[i].stage == Stage::HighSchool)
            .min_by_key(|&i| (w.people[i].class_of, i))
            .unwrap();
        let id = w.people[i].player.id;
        fc::add(&mut w, id).unwrap();
        fc::offer(&mut w, id).unwrap();
        let name_of = w.people[i].name.clone();
        let mut s = Scene::new(Kind::RecruitCall, w.today, format!("A call to {name_of}"));
        s.a = id as u64;
        s.b = mine as u64;
        s.who = vec![Who::Player(id)];
        let sid = scene::add(&mut w, s);
        let mut g = Game::new(home(name));
        g.world = Some(w);
        let f = Arc::new(Fake::new());
        g.ai.use_fake(f.clone());
        g.call("people_scene", &json!({ "id": sid })).unwrap();
        (g, f, sid, i, mine)
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

    fn reading(pick: &str, promise: Value) -> Value {
        let v = json!({ "pick": pick, "also": "none", "tone": "firm", "promise": promise, "claims": [], "named": [], "fit": "clear" });
        body(&v.to_string(), 700, 60)
    }

    #[test]
    fn a_typed_promise_to_a_recruit_asks_first_and_is_the_chips_promise() {
        let (mut g, f, sid, i, mine) = call("promise");
        let s = g.world.as_ref().unwrap().folk.scenes.iter().find(|s| s.id == sid).unwrap().clone();
        assert!(!s.done && scene::takes_typed(&s), "the call takes typed lines");
        // The reader finds no option of the card's for it, and hears the
        // promise: it still lands on the promise it means.
        let promise = json!({ "kind": "early_chance", "by": "none", "condition": "none", "condition_words": "", "against": "none" });
        f.push(READER, Ok(reading("none", promise)));
        let t =
            ok(&mut g, "people_say", json!({ "id": sid, "text": "You'll get your shot on the field as a freshman." }))
                ["ticket"]
                .as_u64()
                .unwrap();
        let v = finish(&mut g, t);
        assert_eq!(v["state"], "unsure", "{v}");
        let r = &v["readings"].as_array().unwrap()[0];
        assert_eq!((r["pick"].as_u64(), r["final"].as_bool()), (Some(731 + 3), Some(true)), "{v}");
        assert!(pledge::on_case(g.world.as_ref().unwrap(), i, mine).is_none(), "nothing until you confirm");
        let t2 = ok(&mut g, "people_say", json!({ "id": sid, "ticket": t, "pick": 734 }))["ticket"].as_u64().unwrap();
        let v = finish(&mut g, t2);
        assert_eq!(v["state"], "done", "{v}");
        assert_eq!(pledge::on_case(g.world.as_ref().unwrap(), i, mine), Some(pledge::Kind::Early));
    }

    #[test]
    fn the_writers_card_for_a_recruit_holds_only_his_reaction() {
        let (mut g, f, sid, i, _) = call("writer");
        let s = g.world.as_ref().unwrap().folk.scenes.iter().find(|s| s.id == sid).unwrap().clone();
        let options = scene::options(&s);
        let k = options
            .iter()
            .take(crate::live_talk::card::MAX_OPTIONS)
            .position(|p| (7100..7300).contains(&p.code))
            .expect("a point to make on the card");
        f.push(READER, Ok(reading(&format!("o{}", k + 1), Value::Null)));
        f.push(WRITER, Ok(body("That means a lot, coach.", 400, 20)));
        let t = ok(&mut g, "people_say", json!({ "id": sid, "text": "Let me tell you what we can do for you here." }))
            ["ticket"]
            .as_u64()
            .unwrap();
        let v = finish(&mut g, t);
        assert_eq!(v["state"], "done", "{v}");
        let w = g.world.as_ref().unwrap();
        let id = w.people[i].player.id;
        let asked = f.asked();
        let card = &asked.iter().rev().find(|r| r.label == WRITER).expect("the writer was asked").user;
        // His reaction, as the game picked it and his file kept it.
        let took: Vec<pulls::Ev> = w.folk.pulls.of(id).filter(|r| r.on == w.today).map(|r| r.ev).collect();
        let shown: Vec<&str> = [pulls::Ev::TookWell, pulls::Ev::Polite, pulls::Ev::Cool, pulls::Ev::Doubted]
            .into_iter()
            .filter(|e| took.contains(e))
            .map(super::reaction)
            .collect();
        // What the game released with this answer is his reaction and
        // nothing more: no want, no weight, no group he weighs.
        let released: Vec<&str> = card
            .split("What your read of him caught with this answer:\n")
            .nth(1)
            .map(|x| x.lines().take_while(|l| l.starts_with("- ")).map(|l| &l[2..]).collect())
            .unwrap_or_default();
        assert!(released.len() <= 1, "{card}");
        for r in &released {
            assert!(shown.contains(r), "only his reaction is released: {r}");
        }
        assert!(shown.is_empty() || released.len() == 1, "his reaction is on the card: {card}");
    }
}
