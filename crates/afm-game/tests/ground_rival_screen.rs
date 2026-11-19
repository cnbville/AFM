//! Lane S6, the rival screen: the page's data on top of lane G9's race (his
//! face and style in words, the race for a kid of yours in dated and
//! numbered signs, the claims between you, where he works and your men
//! against him) and the commands the screen's buttons call.

mod common;

use afm_engine::calendar::Date;
use afm_engine::world::ground::race::{Claim, Topic, Waver};
use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn a_rival_page_carries_what_the_board_draws() {
    let dir = std::env::temp_dir().join(format!("afm-ground-rival-screen-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    common::build_world(&mut g, json!({ "seed": 22, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    assert!(g.call("ground_rival_page", &json!({ "man": 0 })).is_err(), "no seat, no page");
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

    // A man on another college staff, and a high school kid to race for.
    let (man, program, kid, today) = {
        let w = g.world.as_ref().unwrap();
        let mine = afm_engine::world::ground::standing::seat_program(w).unwrap();
        let man = (0..w.coaches.len())
            .find(|&c| {
                matches!(w.coaches[c].job.map(|j| j.employer),
                    Some(afm_engine::world::coach::Employer::College(p)) if p != mine)
            })
            .expect("a rival man");
        let program = match w.coaches[man].job.unwrap().employer {
            afm_engine::world::coach::Employer::College(p) => p,
            _ => unreachable!(),
        };
        let kid = w
            .people
            .iter()
            .find(|q| q.stage == afm_engine::world::Stage::HighSchool)
            .map(|q| q.player.id)
            .expect("a recruit");
        (man as u32, program as u32, kid, w.today)
    };

    // Before any race: a page with the man and nothing in his races.
    let x = ok(&mut g, "ground_rival_page", json!({ "man": man }));
    assert_eq!(x["man"], man);
    assert!(x["face"]["id"].is_u64() && x["face"]["initials"].is_string(), "his head");
    assert_eq!(x["colors"].as_array().unwrap().len(), 2);
    assert!(x["style"].is_string() && x["style_line"].is_string());
    assert!(x["today"].is_string() && x["totals"]["men"].is_u64());
    assert!(x["races"].as_array().unwrap().is_empty());
    assert_eq!(x["map"]["program"], program);

    // A commit of yours with rival signs on him, and a claim each way.
    {
        let w = g.world.as_mut().unwrap();
        let mine = afm_engine::world::ground::standing::seat_program(w).unwrap() as u32;
        w.ground.race.wavers.push(Waver {
            id: kid,
            rival: program,
            date: today,
            signs: vec!["Their man was at his school".into(), "He took an official visit".into()],
            when: vec![(today.add(-9), "sheet".into()), (today.add(-3), "visit".into())],
            fought: None,
            line: None,
        });
        let claim = |by: u32, man: Option<u32>, about: u32, topic: Topic| Claim {
            kid,
            by,
            man,
            about,
            topic,
            said: today,
            settles: Date::ymd(today.year() + 1, 12, 15),
            held: None,
            words: "Georgia State's head coach will be gone in a year.".into(),
            named: None,
        };
        w.ground.race.claims.push(claim(program, Some(man), mine, Topic::Job));
        w.ground.race.claims.push(claim(mine, None, program, Topic::Depth));
    }
    let x = ok(&mut g, "ground_rival_page", json!({ "man": man }));
    let races = x["races"].as_array().unwrap();
    assert_eq!(races.len(), 1, "one kid in a race with him");
    let r = &races[0];
    assert_eq!(r["id"], kid);
    // Signs: oldest first, numbered, each with where it came from.
    let signs = r["signs"].as_array().unwrap();
    assert_eq!(signs.len(), 2);
    assert_eq!(signs[0]["n"], 1);
    assert_eq!(signs[0]["source"], "a sign-in sheet");
    assert_eq!(signs[1]["source"], "a visit he took");
    assert!(signs.iter().all(|s| s["date"].is_string() && s["text"].is_string()));
    assert!(r["open"] == true && r["family"]["name"].is_string());
    // Your claim options: three topics, the one you made marked.
    let opts = r["your_claims"].as_array().unwrap();
    assert_eq!(opts.len(), 3);
    assert!(opts.iter().any(|o| o["topic"] == "depth" && o["made"] == true));
    assert!(opts.iter().any(|o| o["topic"] == "job" && o["made"] == false));
    assert_eq!(x["claims_his"].as_array().unwrap().len(), 1);
    assert_eq!(x["claims_yours"].as_array().unwrap().len(), 1);
    assert!(x["claims_his"][0]["note"].as_str().unwrap().contains("settles"));
    assert!(x["claims_his"][0]["held"].is_null());
    // Never a number of his: the page is words.
    common::nothing_hidden("ground_rival_page", &x, "");

    // The buttons: your call, and a claim about him.
    let said = ok(&mut g, "ground_fight", json!({ "id": kid }));
    assert!(said["said"].is_string());
    let said = ok(&mut g, "ground_claim", json!({ "id": kid, "about": program, "topic": "job" }));
    assert!(said["said"].is_string());
    let x = ok(&mut g, "ground_rival_page", json!({ "man": man }));
    assert_eq!(x["races"][0]["open"], false, "your call is made");
    assert!(x["races"][0]["your_claims"].as_array().unwrap().iter().filter(|o| o["made"] == true).count() == 2);
    assert_eq!(x["claims_yours"].as_array().unwrap().len(), 2);

    // The list page reads the same race with dated words.
    let l = ok(&mut g, "ground_rivals", json!({}));
    assert!(l["today"].is_string());
    for r in l["rivals"].as_array().unwrap() {
        assert!(r["colors"].as_array().unwrap().len() == 2 && r["style"].is_string());
    }
    let b = ok(&mut g, "ground_race", json!({}));
    assert!(b["wavers"][0]["date_words"].is_string() && b["claims"][0]["settles_words"].is_string());
    let _ = std::fs::remove_dir_all(dir);
}
