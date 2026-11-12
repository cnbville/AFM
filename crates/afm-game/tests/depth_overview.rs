//! The position depth overview's one change to the game's answers (Cain,
//! 2026-11-12): the transfer portal's position filter takes the position
//! menu's groups (OL, DL, DB, Specialists) as well as one position, so the
//! strip's pick follows to the portal. A name that is neither takes in
//! nobody, which the strip uses to ask for the portal's counts without the
//! entrants' rows.

use afm_engine::calendar::Date;
use afm_engine::world::{PortalEntry, Stage};
use afm_game::Game;
use serde_json::{Value, json};

fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
    match g.call(cmd, &args) {
        Ok(v) => v,
        Err(e) => panic!("{cmd}: {e}"),
    }
}

/// The positions of the entrants a portal answer lists, sorted.
fn positions(x: &Value) -> Vec<String> {
    let mut v: Vec<String> =
        x["entrants"].as_array().unwrap().iter().map(|e| e["pos"].as_str().unwrap().to_string()).collect();
    v.sort();
    v
}

/// The filters the test asks for and the positions each one takes in.
const ASKS: [(&str, &[&str]); 7] = [
    ("ALL", &["QB", "OT", "IOL", "DT", "LB", "K"]),
    ("OL", &["OT", "IOL"]),
    ("DL", &["EDGE", "DT"]),
    ("Specialists", &["K", "P", "LS"]),
    ("QB", &["QB"]),
    ("OT", &["OT"]),
    ("NONE", &[]),
];

#[test]
fn the_portal_takes_a_group_as_a_position() {
    let dir = std::env::temp_dir().join(format!("afm-depth-{}", std::process::id()));
    let mut g = Game::new(dir);
    ok(&mut g, "build_world", json!({ "seed": 7, "history_years": 1, "scale": 0.04, "engine_from": 2026 }));
    let job = ok(&mut g, "jobs", json!({ "level": "college" }))["jobs"][0]["ref"].clone();
    ok(
        &mut g,
        "start_career",
        json!({ "name": "Test Coach", "age": 41, "background": "college_assistant", "job": job, "seed": 9, "no_offers": true }),
    );
    let ask = |g: &mut Game, pos: &str| ok(g, "portal_desk", json!({ "pos": pos, "division": "ALL" }));

    // The portal as the world has it: the answer lists at most 400 entrants, so
    // the test reads the count of those that match (`shown`) before and after.
    let before: Vec<Value> = ASKS.iter().map(|(name, _)| ask(&mut g, name)).collect();

    // One college player of each of a few positions, at other programs, goes into the portal.
    let w = g.world.as_mut().unwrap();
    let mine = w.seat.and_then(|s| s.program).expect("a college head seat");
    let today = w.today;
    let mut put: Vec<&str> = Vec::new();
    for want in ["OT", "IOL", "QB", "LB", "DT", "K"] {
        let k = w.people.iter().position(|p| {
            p.stage == Stage::College
                && p.college.is_some_and(|c| c != mine)
                && p.cal.portal.is_none()
                && p.player.position.name() == want
        });
        if let Some(k) = k {
            w.people[k].cal.portal = Some(PortalEntry {
                entered: today,
                decide_by: Date(today.0 + 10),
                why: "test",
                held: false,
                looking: false,
            });
            put.push(want);
        }
    }
    assert!(put.len() >= 4, "the small world has players at most positions: {put:?}");

    let all = ask(&mut g, "ALL");
    for ((name, takes), was) in ASKS.iter().zip(&before) {
        let now = ask(&mut g, name);
        let added = put.iter().filter(|p| takes.contains(p)).count() as u64;
        let (a, b) = (now["shown"].as_u64().unwrap(), was["shown"].as_u64().unwrap());
        assert_eq!(a, b + added, "{name}: the players put in the portal that match are counted");
        // Whoever is listed is in the group.
        if *name != "ALL" {
            for pos in positions(&now) {
                assert!(takes.contains(&pos.as_str()), "{name} lists a {pos}");
            }
        }
        // The portal's counts and your own players do not depend on the filter.
        assert_eq!(now["in_portal"], all["in_portal"], "{name}: the counts don't depend on the filter");
        assert_eq!(now["roster"], all["roster"], "{name}: your own players are not filtered");
    }
    assert!(ask(&mut g, "NONE")["entrants"].as_array().unwrap().is_empty(), "a name that is no position lists nobody");
}
