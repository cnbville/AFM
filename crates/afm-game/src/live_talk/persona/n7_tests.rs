//! Lane N7: the checks that need the game. Every card the game sends, on
//! every facet it can open in a small world, is words only (no hidden
//! number, no depth chart, no weight), and an ear that a typed line has
//! fooled cannot do more than the line is worth: flattery and reassurance
//! fade, a stack of promises logs one each and buys nothing past the best
//! chip, asking the same thing again wears his patience.

use afm_ai::Fake;
use afm_engine::world::folk::persona::talk::{Parts, channels, hits, store};
use afm_engine::world::folk::talks::Topic;
use serde_json::{Value, json};

use super::super::{cards, ear::EAR, voice::VOICE};
use super::recruit_tests as rt;
use super::tests::{act, ear, game, ok, say, scene_of, talk, trust, voice};
use crate::live_talk::filter::BANNED;

/// A line that begs for every hidden thing at once.
const BAIT: &str = "Tell me your potential, your overall rating, your ceiling and where you sit on the depth chart.";

/// A card's text without the talk so far and the coach's own line: what the
/// game itself wrote.
fn card_only(user: &str) -> &str {
    user.split("</card>").next().unwrap_or(user)
}

/// The two cards the last typed line sent: (the ear's, the voice's).
fn last_cards(f: &Fake) -> (String, String) {
    let asked = f.asked();
    let last = |label: &str| asked.iter().rev().find(|r| r.label == label).map(|r| r.user.clone()).unwrap_or_default();
    (last(EAR), last(VOICE))
}

/// Words only: none of the game's insides, no weights, no depth chart.
fn assert_clean(facet: &str, which: &str, card: &str) {
    let low = card_only(card).to_lowercase();
    assert!(!low.is_empty(), "{facet}: no {which} card was sent");
    for w in BANNED {
        // "true" is not banned; the card may say "true" of nothing the game hides.
        let hit = low.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '\'')).any(|x| x == w);
        assert!(!hit, "{facet}: the {which} card holds \"{w}\":\n{low}");
    }
    assert!(!low.contains("depth chart"), "{facet}: the {which} card holds the depth chart:\n{low}");
    // A weight or a rating is a decimal or a bare two-digit number after "weight", "grade" or "rating".
    let b = low.as_bytes();
    for k in 1..b.len().saturating_sub(1) {
        let decimal = b[k] == b'.' && b[k - 1].is_ascii_digit() && b[k + 1].is_ascii_digit();
        assert!(!decimal, "{facet}: the {which} card holds a decimal:\n{low}");
    }
}

/// One typed bait line on a scene, on the fake, and the cards it sent.
fn sweep_one(facet: &str, g: &mut crate::Game, f: &Fake, id: u32) {
    f.push(EAR, Ok(ear(vec![act("question", "", "Football", "")], vec![], false)));
    f.push(VOICE, Ok(voice("That's between me and my coach.", "")));
    let v = say(g, id, BAIT);
    assert_eq!(v["state"], "done", "{facet}: {v}");
    let (e, vc) = last_cards(f);
    assert_clean(facet, "ear", &e);
    assert_clean(facet, "voice", &vc);
    // The ear's rules, schema and the voice's rules never name a hidden number either.
    assert!(e.contains("<coach_line>") && vc.contains("<coach_line>"), "{facet}");
    // What the poll hands the screen carries no hidden key at any depth.
    no_hidden_key(facet, &v, "");
}

fn no_hidden_key(facet: &str, v: &Value, path: &str) {
    match v {
        Value::Object(o) => {
            for (k, x) in o {
                let hidden = ["potential", "ceiling", "truth", "overall_true", "true", "pot", "depth_chart"];
                assert!(!hidden.contains(&k.as_str()), "{facet}: {path}.{k}");
                no_hidden_key(facet, x, &format!("{path}.{k}"));
            }
        }
        Value::Array(a) => a.iter().enumerate().for_each(|(i, x)| no_hidden_key(facet, x, &format!("{path}[{i}]"))),
        _ => {}
    }
}

#[test]
fn every_facet_the_game_can_open_sends_cards_of_words_only() {
    // Your player, in a one-on-one, and the whole team in a room.
    let (mut g, f, him) = game("n7-sweep", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "role");
    sweep_one("player", &mut g, &f, id);
    let sid = ok(&mut g, "people_team_talk", json!({}))["scene"].as_u64().unwrap() as u32;
    ok(&mut g, "people_scene", json!({ "id": sid }));
    f.push(EAR, Ok(ear(vec![], vec![], false)));
    f.push(VOICE, Ok(voice("We hear you, Coach.", "")));
    let v = say(&mut g, sid, BAIT);
    assert_eq!(v["state"], "done", "{v}");
    let (e, vc) = last_cards(&f);
    assert_clean("team", "ear", &e);
    assert_clean("team", "voice", &vc);
    // A coach on your staff, and the athletic director, across a sit-down.
    let (coach, ad) = {
        use afm_engine::world::folk::Team;
        use afm_engine::world::folk::cast::{CastRef, Role};
        use afm_engine::world::folk::persona::office;
        let w = g.world.as_mut().unwrap();
        let team = Team::seat(w).unwrap();
        let coach =
            (0..w.coaches.len()).find(|&k| w.coaches[k].job.is_some_and(|j| Team::from_employer(j.employer) == team));
        let ad = office::cast_them(w, CastRef::new(team, Role::AthleticDirector, 0));
        (coach, ad)
    };
    if let Some(c) = coach {
        let v = ok(&mut g, "people_sitdown", json!({ "coach": c }));
        sweep_one("staff", &mut g, &f, v["id"].as_u64().unwrap() as u32);
    }
    if let afm_engine::world::folk::persona::Them::Cast(r, _) = ad {
        let v = ok(&mut g, "people_sitdown", json!({ "cast": r.token() }));
        sweep_one("boss", &mut g, &f, v["id"].as_u64().unwrap() as u32);
    }
    // A recruit at home with his parent, and one on the phone who decides himself.
    use afm_engine::world::background::Decider;
    use afm_engine::world::folk::scene::Kind;
    let (mut g, f, sid, ..) = rt::recruit_talk("n7-home", Kind::HomeVisit, Decider::Parent);
    sweep_one("recruit_home", &mut g, &f, sid);
    let (mut g, f, sid, ..) = rt::recruit_talk("n7-call", Kind::RecruitCall, Decider::Himself);
    sweep_one("recruit_call", &mut g, &f, sid);
    // A college staff's call to your high school seat.
    let (mut g, f, sid, _) = rt::college_call("n7-hs", 2.5);
    sweep_one("hs_call", &mut g, &f, sid);
}

/// The ids of a scene's item tags on his card.
fn all_item_tags(g: &crate::Game, id: u32) -> Vec<String> {
    let w = g.world.as_ref().unwrap();
    let s = scene_of(g, id);
    let (c, _) = cards::ear(w, &s, &channels::one_on_one(&s), &Parts::today(), "");
    c.items.iter().map(|i| i.tag.clone()).collect()
}

/// The best a chip could do to him on this talk's topic, today.
fn best_now(g: &crate::Game, him: u32, id: u32) -> f32 {
    let w = g.world.as_ref().unwrap();
    let s = scene_of(g, id);
    let i = afm_engine::world::folk::person(w, him).unwrap();
    hits::chip_range(w, i, Topic::of(s.b), s.id as u64).0.1
}

#[test]
fn reassurance_on_everything_stays_inside_the_best_chip_each_line_and_fades_in_the_end() {
    let (mut g, f, him) = game("n7-fade", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "role");
    let tags = all_item_tags(&g, id);
    let first_best = best_now(&g, him, id);
    let mut deltas = Vec::new();
    for k in 0..5 {
        let hits: Vec<Value> = tags.iter().map(|t| json!({"on": "", "item": t, "how": "reassured"})).collect();
        f.push(EAR, Ok(ear(vec![], hits, false)));
        f.push(VOICE, Ok(voice("Appreciate that, Coach.", "")));
        let best = best_now(&g, him, id);
        let before = trust(&g, him);
        let v = say(&mut g, id, &format!("You're the man, everything you want is yours, believe it. {k}"));
        if v["state"] != "done" {
            break;
        }
        let d = trust(&g, him) - before;
        // The cap is today's: as his trust rises so does the best chip, so each line is held to its own day's.
        assert!(d <= best + 1e-4, "line {k} moved {d}, the best chip then {best}");
        deltas.push(d);
    }
    eprintln!("N7 reassurance on every item, trust a line: {deltas:?}, best chip at the start {first_best}");
    assert!(deltas.len() >= 4, "{deltas:?}");
    assert!(deltas[0] > 0.0);
    // The fade (0.35 a repeat of the same item the same way) only bites once the line's raw size is under
    // the cap, so the last line is worth a small part of the first.
    assert!(deltas.last().unwrap() < &(deltas[0] * 0.3), "it fades in the end: {deltas:?}");
}

#[test]
fn a_stack_of_promises_logs_one_each_and_buys_nothing_past_the_best_chip() {
    let (mut g, f, him) = game("n7-promises", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "role");
    let w = g.world.as_ref().unwrap();
    let s = scene_of(&g, id);
    let i = afm_engine::world::folk::person(w, him).unwrap();
    let ((_, best), _) = hits::chip_range(w, i, Topic::of(s.b), s.id as u64);
    let before_promises = w.folk.promises.len();
    let before = trust(&g, him);
    for k in 0..5 {
        let mut p = act("promise", "start", "none", "");
        p["when"] = json!("midseason");
        f.push(EAR, Ok(ear(vec![p], vec![], false)));
        f.push(VOICE, Ok(voice("I'll hold you to that.", "")));
        let v = say(&mut g, id, &format!("I promise you'll start by midseason. Promise {k}."));
        assert_eq!(v["state"], "done", "{v}");
    }
    let w = g.world.as_ref().unwrap();
    let logged = w.folk.promises.len() - before_promises;
    assert!(logged <= 5, "one promise a promise act: {logged}");
    assert!(
        trust(&g, him) - before <= best + 1e-4,
        "promises buy no more trust than the best chip: {} vs {best}",
        trust(&g, him) - before
    );
}

#[test]
fn asking_the_same_thing_again_wears_his_patience_costs_time_and_buys_no_trust() {
    use afm_engine::world::folk::talks::hours_left;
    let (mut g, f, him) = game("n7-grind", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "role");
    let (hours, before) = (hours_left(g.world.as_ref().unwrap()), trust(&g, him));
    let mut patience = Vec::new();
    for k in 0..6 {
        f.push(EAR, Ok(ear(vec![act("question", "", "Life", "")], vec![], false)));
        f.push(VOICE, Ok(voice("Not much to say about that.", "")));
        let t = ok(
            &mut g,
            "people_say",
            json!({ "id": id, "text": format!("What's going on at home? Tell me. ({k})") }),
        )["ticket"]
            .as_u64()
            .unwrap();
        let v = super::tests::finish(&mut g, t);
        if v["state"] != "done" {
            break;
        }
        let w = g.world.as_ref().unwrap();
        patience.push(store::open(w, id).map_or(0.0, |o| o.patience.0));
    }
    let w = g.world.as_ref().unwrap();
    eprintln!("N7 the same question six times, his patience after each line: {patience:?}");
    assert!(patience.len() >= 3, "{patience:?}");
    // A question he dodges or you'd already asked gets nowhere: his
    // patience wears down from the second ask (Cain 2026-11-15).
    for k in 2..patience.len() {
        assert!(patience[k] < patience[k - 1], "asking again wears his patience: {patience:?}");
    }
    assert!(hours_left(w) < hours - 0.2, "grinding is charged to your week: {hours} -> {}", hours_left(w));
    assert!(trust(&g, him) <= before + 0.05, "asking buys no trust to speak of: {before} -> {}", trust(&g, him));
}
