//! The big-moment reveals (screens topic 9, Cain 2026-11-14): the job you
//! are given comes with its facts, a commitment brings his finalists' caps
//! and the why-lines, a walk-on's commitment brings nothing, a reveal is
//! shown once and can be played again, and a job change brings its own.

use afm_engine::world::Stage;
use afm_game::Game;
use afm_game::views::{self, TeamRef};
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

#[test]
fn the_big_moments_carry_their_facts_and_play_again() {
    let dir = std::env::temp_dir().join(format!("afm-big-moments-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 12, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }));
    let job = jobs["jobs"][0]["ref"].clone();
    ok(&mut g, "start_career", json!({ "name": "Test Coach", "job": job, "seed": 3 }));

    // The job you are given: the room's colours, the boss's bar, last season.
    let first = ok(&mut g, "reveals", json!({}));
    let hire = &first[0];
    assert_eq!(hire["kind"], "hire", "{first}");
    assert_eq!(hire["kicker"], "The job is yours", "{first}");
    assert!(!hire["verdict"].as_str().unwrap().is_empty(), "the boss's bar is the verdict: {first}");
    assert!(hire["facts"].as_array().is_some_and(|f| !f.is_empty()), "last season is a fact: {first}");
    assert_eq!(hire["quiet"], false);

    // It is kept: the list names it, and it plays again on its own.
    let list = ok(&mut g, "reveal_list", json!({}));
    assert_eq!(list.as_array().unwrap().len(), 1, "{list}");
    let again = ok(&mut g, "reveal_again", json!({ "n": list[0]["n"] }));
    assert_eq!(again[0]["name"], hire["name"], "{again}");
    assert_eq!(ok(&mut g, "reveal_again", json!({ "n": 999 })), json!([]));

    ok(&mut g, "continue", json!({}));
    ok(&mut g, "reveals", json!({}));

    // A recruit with three other offers commits to us today, and a walk-on
    // does too: the first gets the hat table, the second gets nothing.
    let (star, walk) = {
        let w = g.world.as_mut().unwrap();
        let Some(TeamRef::College(pi)) = views::mine(w) else { panic!("a college seat") };
        let today = w.today;
        let others: Vec<usize> = (0..w.programs.len()).filter(|&i| i != pi).take(3).collect();
        let ids: Vec<usize> = w
            .people
            .iter()
            .enumerate()
            .filter(|(_, p)| p.stage == Stage::HighSchool && p.cal.commit.is_none() && p.cal.signed.is_none())
            .map(|(k, _)| k)
            .take(2)
            .collect();
        assert_eq!(ids.len(), 2, "two high school recruits");
        let (a, b) = (ids[0], ids[1]);
        w.people[a].stars = 4;
        w.people[a].cal.offers = others;
        w.people[a].cal.commit = Some((pi, today, false));
        w.people[b].stars = 2;
        w.people[b].cal.commit = Some((pi, today, true));
        (w.people[a].name.clone(), w.people[b].name.clone())
    };
    ok(&mut g, "continue", json!({}));
    let got = ok(&mut g, "reveals", json!({}));
    let list = got.as_array().unwrap();
    let r = list.iter().find(|r| r["name"] == star.as_str()).unwrap_or_else(|| panic!("his reveal: {got}"));
    assert_eq!(r["kind"], "commit");
    assert!(r["hats"].as_array().unwrap().len() >= 2, "his finalists' caps are on the table: {r}");
    let chosen = r["chosen"].as_u64().unwrap() as usize;
    assert!(chosen < r["hats"].as_array().unwrap().len());
    assert_eq!(r["hats"][chosen]["name"], job_name(&mut g), "the chosen cap is yours: {r}");
    assert!(r["verdict"].as_str().unwrap().contains("over 3 other offers"), "{r}");
    assert!(!r["token"].as_str().unwrap().is_empty(), "his position is the token: {r}");
    assert!(!list.iter().any(|r| r["name"] == walk.as_str()), "a walk-on had no offer, so no reveal: {got}");
    std::fs::remove_dir_all(dir).ok();
}

/// The name of your program.
fn job_name(g: &mut Game) -> Value {
    let w = g.world.as_ref().unwrap();
    json!(views::team_name(w, views::mine(w).unwrap()))
}
