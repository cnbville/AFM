//! Your development projects from the Development tab (the progression
//! update): the room's slots, one body project a room, ending one, and no
//! true number on the screen.

use afm_engine::progression::Room;
use afm_engine::world::coach::Role;
use serde_json::{Value, json};

use crate::Game;
use crate::views::{mine, roster_people};

fn head_coach_world(name: &str) -> (Game, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("afm-proj-{name}-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    g.call("build_world", &json!({ "seed": 11, "history_years": 1, "scale": 0.03, "engine_from": 2026 })).unwrap();
    let job = g.call("jobs", &json!({ "level": "college" })).unwrap()["jobs"][0]["ref"].clone();
    g.call(
        "start_career",
        &json!({ "name": "Test Coach", "age": 41, "background": "college_assistant", "job": job, "seed": 9 }),
    )
    .unwrap();
    let w = g.wm().unwrap();
    let c = w.career.as_ref().unwrap().coach;
    w.coaches[c].job.as_mut().unwrap().role = Role::HeadCoach;
    // A clean slate: no project a previous staff left running.
    let t = mine(w).unwrap();
    for i in roster_people(w, t) {
        afm_engine::progression::end_project(&mut w.people[i].player);
        afm_engine::progression::end_body(&mut w.people[i].player);
    }
    (g, dir)
}

/// Your roster's ids in a room, youngest first (under the body clock).
fn room_ids(g: &Game, room: Room) -> Vec<u32> {
    let w = g.w().unwrap();
    let t = mine(w).unwrap();
    let mut v: Vec<(u32, u32)> = roster_people(w, t)
        .into_iter()
        .filter(|&i| Room::of(w.people[i].player.position) == Some(room))
        .map(|i| (w.people[i].player.age, w.people[i].player.id))
        .collect();
    v.sort();
    v.into_iter().map(|x| x.1).collect()
}

fn first_skill(g: &mut Game, id: u32) -> String {
    let d = g.call("player_dev", &json!({ "id": id })).unwrap();
    d["projects"]["skill"]["options"][0]["key"].as_str().unwrap().to_string()
}

#[test]
fn your_projects_keep_the_rooms_slots_and_show_only_reads() {
    let (mut g, dir) = head_coach_world("slots");
    // The safeties' room has one skill slot.
    let s = room_ids(&g, Room::S);
    assert!(s.len() >= 2, "two safeties on the roster");
    let k = first_skill(&mut g, s[0]);
    let d = g.call("player_dev_project_start", &json!({ "id": s[0], "what": k })).unwrap();
    assert!(d["said"].as_str().unwrap().contains("started"));
    let on = &d["projects"]["skill"]["on"];
    assert_eq!(on["key"], json!(k));
    assert_eq!(on["yours"], json!(true));
    let gain = on["gain"].as_array().unwrap();
    assert!(gain[0].as_f64().unwrap() <= gain[1].as_f64().unwrap());
    let k2 = first_skill(&mut g, s[1]);
    let e = g.call("player_dev_project_start", &json!({ "id": s[1], "what": k2 })).unwrap_err();
    assert!(e.contains("no free slot"), "{e}");
    let d2 = g.call("player_dev", &json!({ "id": s[1] })).unwrap();
    assert!(d2["projects"]["skill"]["blocked"].is_string());
    // One body project a room on the strength staff, beside a skill one.
    let b = g.call("player_dev_body_start", &json!({ "id": s[0], "what": "strength" })).unwrap();
    assert_eq!(b["projects"]["body"]["on"]["key"], json!("strength"));
    let e = g.call("player_dev_body_start", &json!({ "id": s[1], "what": "speed" })).unwrap_err();
    assert!(e.contains("in use"), "{e}");
    // The Training screen counts them.
    let x = g.call("practice", &json!({})).unwrap();
    let room = x["projects"]["rooms"].as_array().unwrap().iter().find(|r| r["room"] == "S").unwrap().clone();
    assert_eq!(
        (room["used"].clone(), room["slots"].clone(), room["body_used"].clone()),
        (json!(1), json!(1), json!(1))
    );
    // Ending frees the slot for the next man.
    let d = g.call("player_dev_project_end", &json!({ "id": s[0] })).unwrap();
    assert!(d["projects"]["skill"]["on"].is_null());
    g.call("player_dev_project_start", &json!({ "id": s[1], "what": k2 })).unwrap();
    // Two corners fit the corners' two slots; a third doesn't.
    let c = room_ids(&g, Room::Cb);
    assert!(c.len() >= 3, "three corners");
    for &id in &c[..2] {
        let k = first_skill(&mut g, id);
        g.call("player_dev_project_start", &json!({ "id": id, "what": k })).unwrap();
    }
    let k = first_skill(&mut g, c[2]);
    assert!(g.call("player_dev_project_start", &json!({ "id": c[2], "what": k })).is_err());
    // A skill his position doesn't build is refused.
    assert!(g.call("player_dev_project_start", &json!({ "id": s[0], "what": "throwOnMove" })).is_err());
    // No true room, knack or ceiling reaches the page.
    let v: Value = g.call("player_dev", &json!({ "id": s[0] })).unwrap();
    let text = v["projects"].to_string();
    for word in ["ceiling", "limit", "\"room_true\"", "potential"] {
        assert!(!text.contains(word), "{word} in {text}");
    }
    let w = g.w().unwrap();
    let i = crate::views::person_index(w, s[1]).unwrap();
    assert!(w.people[i].player.prog.yours, "your project is marked yours");
    let _ = std::fs::remove_dir_all(dir);
}
