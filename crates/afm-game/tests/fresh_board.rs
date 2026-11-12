//! A new staff resets the board it inherits (Cain, 2026-11-05): a new save
//! can start with no offers out, and for a month at a new college job the
//! old staff's offers can be pulled one at a time, by ticks or all at once,
//! with nothing held against you. Off the fresh start a pulled commitment
//! still cools his high school, as before.

use afm_engine::world::front_office::college::{self as desk, Source};
use afm_engine::world::{Stage, World};
use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

fn program(w: &World) -> usize {
    w.seat.and_then(|s| s.program).expect("a college head seat")
}

/// Recruits of the classes still to sign who hold program `j`'s
/// scholarship offer, early ones included, or a scholarship commitment to
/// it, and haven't signed.
fn holding(w: &World, j: usize) -> Vec<u32> {
    let next = desk::next_class(w);
    w.people
        .iter()
        .filter(|p| {
            matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) && p.cal.signed.is_none() && p.class_of >= next
        })
        .filter(|p| {
            p.cal.offers.contains(&j)
                || p.cal.early_offers.iter().any(|o| o.0 == j)
                || p.cal.commit.is_some_and(|c| c.0 == j && !c.2)
        })
        .map(|p| p.player.id)
        .collect()
}

fn signed_with(w: &World, j: usize) -> usize {
    w.people.iter().filter(|p| p.cal.signed.is_some_and(|s| s.program == j && !s.walk_on)).count()
}

fn person(w: &World, id: u32) -> &afm_engine::world::Person {
    w.people.iter().find(|p| p.player.id == id).unwrap()
}

/// A test's commitment: recruit `id` commits to `j`, and his high school
/// is a pipeline of `j`'s.
fn commit(w: &mut World, j: usize, id: u32) {
    let today = w.today;
    let year = w.year;
    let k = w.people.iter().position(|p| p.player.id == id).unwrap();
    w.people[k].cal.commit = Some((j, today, false));
    let hs = w.people[k].hs;
    w.programs[j].hs_pipeline.insert(hs, (0.5, year));
}

/// The old staff's offers the test uses: two pulled by ticks in the fresh
/// start, then after it one pulled plainly and one more made the test's
/// commitment.
const NEED: u64 = 4;

fn start(g: &mut Game, job: &Value, no_offers: bool) {
    ok(
        g,
        "start_career",
        json!({ "name": "Test Coach", "age": 41, "background": "college_assistant", "job": job, "seed": 9, "no_offers": no_offers }),
    );
}

#[test]
fn a_new_staff_resets_the_board() {
    let dir = std::env::temp_dir().join(format!("afm-fresh-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    ok(&mut g, "build_world", json!({ "seed": 7, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let fresh = ok(&mut g, "save", json!({ "name": "fresh" }))["saved"].as_str().unwrap().to_string();
    // The first job down the list whose old staff left more offers out than
    // the test uses (`NEED`), else the one with the most. A small world's
    // numbers move with the engine: on one world the first ten jobs had
    // three at most and the twelfth five, so the search goes on past ten.
    let jobs = ok(&mut g, "jobs", json!({ "level": "college" }))["jobs"].as_array().unwrap().clone();
    let mut job = jobs[0]["ref"].clone();
    let mut most = 0;
    for j in jobs.iter().take(60) {
        ok(&mut g, "open_world", json!({ "path": fresh }));
        start(&mut g, &j["ref"], false);
        // The board takes over the old staff's offers when you first open it.
        let n = ok(&mut g, "recruit_board", json!({}))["fresh"]["offers"].as_u64().unwrap_or(0);
        if n > most {
            (most, job) = (n, j["ref"].clone());
        }
        if most > NEED {
            break;
        }
    }
    ok(&mut g, "open_world", json!({ "path": fresh }));

    // 1. Start with no offers out: every old offer is pulled on day one and
    // leaves the board; the signed stay signed.
    start(&mut g, &job, true);
    let w = g.world.as_ref().unwrap();
    let j = program(w);
    assert!(holding(w, j).is_empty(), "nobody holds the old staff's offer: {:?}", holding(w, j));
    assert!(
        !w.front_office.college.board.iter().any(|t| t.by == Source::Inherited && t.pulled.is_none()),
        "no old offer is left on the board"
    );
    let signed_clean = signed_with(w, j);
    let inbox = ok(&mut g, "inbox", json!({}));
    assert!(inbox.to_string().contains("A clean board"), "the inbox says what happened");
    let board = ok(&mut g, "recruit_board", json!({}));
    assert!(board["fresh"].is_null(), "nothing left for the fresh start to pull");

    // 2. The same world without the switch: the old staff's offers are
    // there, and the fresh start lets you pull them for nothing.
    ok(&mut g, "open_world", json!({ "path": fresh }));
    start(&mut g, &job, false);
    let w = g.world.as_ref().unwrap();
    let j = program(w);
    assert_eq!(signed_with(w, j), signed_clean, "the switch never touches a signed player");
    let board = ok(&mut g, "recruit_board", json!({}));
    let f = board["fresh"].clone();
    let offers = f["offers"].as_u64().expect("the old staff left offers out") as usize;
    assert!(offers as u64 >= NEED, "the test uses {NEED} old offers: {f}");
    assert_eq!(f["days"].as_i64(), Some(i64::from(desk::FRESH_DAYS)), "the window counts today: {f}");
    let rows = board["rows"].as_array().unwrap();
    let old: Vec<&Value> =
        rows.iter().filter(|r| r["inherited"] == json!(true) && r["offered"] == json!(true)).collect();
    assert!(old.iter().all(|r| r["free_pull"] == json!(true)), "every old offer is free to pull now");

    // Two of them by ticks: no pipeline cools, the early offer goes too,
    // and his history says the new staff pulled it.
    let out = desk::inherited_out(g.world.as_ref().unwrap());
    let pick: Vec<u32> = out.iter().take(2).map(|x| x.0).collect();
    // February has few commitments yet: make the first one a commitment, his
    // high school a pipeline of yours, so the free pull shows on both.
    commit(g.world.as_mut().unwrap(), j, pick[0]);
    let w = g.world.as_ref().unwrap();
    let pipes: Vec<f64> =
        pick.iter().map(|&id| w.programs[j].hs_pipeline.get(&person(w, id).hs).map_or(0.0, |e| e.0)).collect();
    let text = ok(&mut g, "recruit_pull_many", json!({ "ids": pick }))["text"].as_str().unwrap().to_string();
    assert!(text.contains("costs you nothing") && text.contains("1 commitment with them"), "{text}");
    let w = g.world.as_ref().unwrap();
    for (k, &id) in pick.iter().enumerate() {
        let p = person(w, id);
        assert!(
            !p.cal.offers.contains(&j) && !p.cal.early_offers.iter().any(|o| o.0 == j),
            "{} lost the offer",
            p.name
        );
        assert!(!p.cal.commit.is_some_and(|c| c.0 == j), "his commitment went with it");
        let now = w.programs[j].hs_pipeline.get(&p.hs).map_or(0.0, |e| e.0);
        assert_eq!(now, pipes[k], "no high school holds it against you");
        assert!(p.notes.last().is_some_and(|n| n.contains("new staff pulled")), "{:?}", p.notes.last());
        let t = w.front_office.college.board.iter().find(|t| t.id == id).unwrap();
        assert!(t.pulled.is_some(), "the pull stays on the board until you take him off");
    }
    // Then off the board together.
    let text = ok(&mut g, "recruit_remove_many", json!({ "ids": pick }))["text"].as_str().unwrap().to_string();
    assert!(text.starts_with("Took 2 recruits off"), "{text}");

    // A re-offer to one of them: his history keeps one first offer.
    ok(&mut g, "recruit_offer", json!({ "id": pick[0] }));
    let firsts = person(g.world.as_ref().unwrap(), pick[0])
        .notes
        .iter()
        .filter(|n| n.contains("first scholarship offer"))
        .count();
    assert!(firsts <= 1, "one first offer in his history, not {firsts}");
    ok(&mut g, "recruit_pull", json!({ "id": pick[0] }));

    // 3. After the fresh start the old staff's offers cost what any pull does.
    let until = g.world.as_ref().unwrap().front_office.college.fresh_until.unwrap();
    while g.world.as_ref().unwrap().today <= until {
        g.world.as_mut().unwrap().advance_day();
    }
    let board = ok(&mut g, "recruit_board", json!({}));
    assert!(board["fresh"].is_null(), "the fresh start is over");
    let left = desk::inherited_out(g.world.as_ref().unwrap());
    assert!(!left.is_empty(), "old offers are still out after the fresh start");
    let one = left.iter().find(|x| !x.1).or(left.first()).unwrap().0;
    let w = g.world.as_ref().unwrap();
    let t = w.front_office.college.board.iter().find(|t| t.id == one).unwrap();
    assert!(!desk::free_pull(w, t), "not free any more");
    let text = ok(&mut g, "recruit_pull", json!({ "id": one }))["text"].as_str().unwrap().to_string();
    assert!(text.starts_with("Pulled the offer to"), "a plain pull now: {text}");

    // One more commitment for the costly pull at the end.
    if let Some(&(id, _)) = desk::inherited_out(g.world.as_ref().unwrap()).iter().find(|x| x.0 != one) {
        commit(g.world.as_mut().unwrap(), j, id);
    }

    // The rest but the commitments, all at once: off the board too, and
    // no longer free.
    let commits = desk::inherited_out(g.world.as_ref().unwrap()).iter().filter(|x| x.1).count();
    if desk::inherited_out(g.world.as_ref().unwrap()).len() > commits {
        let text = ok(&mut g, "recruit_reset", json!({ "keep_commits": true }))["text"].as_str().unwrap().to_string();
        assert!(text.contains("off your board") && !text.contains("costs you nothing"), "{text}");
    }
    let left = desk::inherited_out(g.world.as_ref().unwrap());
    assert_eq!(left.len(), commits, "only the commitments are left");
    assert!(left.iter().all(|x| x.1));
    // A pulled commitment cools his high school again.
    let w = g.world.as_ref().unwrap();
    if let Some(&(id, _)) = left.first() {
        let hs = person(w, id).hs;
        let before = w.programs[j].hs_pipeline.get(&hs).map(|e| e.0);
        let text = ok(&mut g, "recruit_pull", json!({ "id": id }))["text"].as_str().unwrap().to_string();
        assert!(text.contains("won't forget"), "{text}");
        let after = g.world.as_ref().unwrap().programs[j].hs_pipeline.get(&hs).map(|e| e.0);
        if let (Some(b), Some(a)) = (before, after) {
            assert!(a < b, "his high school cools: {b} to {a}");
        }
    } else {
        panic!("the test's commitment is out after the fresh start");
    }
    let _ = std::fs::remove_dir_all(dir);
}
