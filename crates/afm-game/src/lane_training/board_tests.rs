//! A spot set on a base board saves the position it touched and no other
//! (playtest bug 12): the rest of the board stays with your staff.

use afm_engine::positions::Position;
use afm_engine::world::coach::Role;
use serde_json::{Value, json};

use crate::Game;

/// A small college world with you in the head coach's seat, so every position
/// on the board is yours to set.
fn head_coach_world(name: &str) -> (Game, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("afm-board-{name}-{}", std::process::id()));
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
    (g, dir)
}

/// The base offense as the screens get it.
fn offense(g: &mut Game) -> Value {
    let x = g.call("boards", &json!({})).unwrap();
    x["boards"].as_array().unwrap().iter().find(|b| b["key"] == "offense").unwrap().clone()
}

/// The man at each spot, by id.
fn men(board: &Value) -> Vec<Value> {
    board["slots"].as_array().unwrap().iter().map(|s| s["player"]["id"].clone()).collect()
}

/// The first two spots at a position.
fn two_at(board: &Value, pos: &str) -> (usize, usize) {
    let at: Vec<usize> = board["slots"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, s)| s["pos"] == pos && !s["player"].is_null())
        .map(|(k, _)| k)
        .collect();
    assert!(at.len() >= 2, "two {pos} spots with a man on them: {at:?}");
    (at[0], at[1])
}

/// The positions you have set an order at.
fn yours(g: &Game) -> Vec<Position> {
    let mut v: Vec<Position> = g.w().unwrap().seat_depth.iter().map(|(p, _)| *p).collect();
    v.sort_by_key(|p| p.index());
    v
}

fn ids_at(g: &Game, pos: Position) -> Vec<u32> {
    g.w().unwrap().seat_depth.iter().find(|(p, _)| *p == pos).map(|(_, v)| v.clone()).unwrap_or_default()
}

#[test]
fn a_spot_set_with_positions_saves_only_those_positions() {
    let (mut g, dir) = head_coach_world("positions");
    assert!(yours(&g).is_empty(), "a new career starts with your staff's order everywhere");

    // Swap the two tackles and name the position: only the tackles are yours.
    let b = offense(&mut g);
    let (l, r) = two_at(&b, "OT");
    let mut ids = men(&b);
    ids.swap(l, r);
    let (first, second) = (ids[l].as_u64().unwrap() as u32, ids[r].as_u64().unwrap() as u32);
    g.call("board_set", &json!({ "board": "offense", "ids": ids, "positions": ["OT"] })).unwrap();
    assert_eq!(yours(&g), vec![Position::OT], "one drop marks one position as yours");
    assert_eq!(ids_at(&g, Position::OT)[..2], [first, second], "the order he set");

    // A second drop at another position adds that one and still no more.
    let b = offense(&mut g);
    let (a, c) = two_at(&b, "WR");
    let mut ids = men(&b);
    ids.swap(a, c);
    g.call("board_set", &json!({ "board": "offense", "ids": ids, "positions": ["WR"] })).unwrap();
    assert_eq!(
        yours(&g),
        vec![Position::WR, Position::OT],
        "the tackles stay his, the receivers join them, nothing else"
    );

    // The receivers' drop left the tackles' order as he set it.
    assert_eq!(ids_at(&g, Position::OT)[..2], [first, second]);

    // Give it back to the staff clears the board, and an empty list names
    // nothing, so nothing is written.
    g.call("board_set", &json!({ "board": "offense", "reset": true })).unwrap();
    assert!(yours(&g).is_empty(), "give it back to the staff clears the board");
    let ids = men(&offense(&mut g));
    g.call("board_set", &json!({ "board": "offense", "ids": ids, "positions": [] })).unwrap();
    assert!(yours(&g).is_empty(), "an empty list writes nothing");

    // A name the game doesn't know is refused, and nothing is written.
    let ids = men(&offense(&mut g));
    let err = g.call("board_set", &json!({ "board": "offense", "ids": ids, "positions": ["OT", "XX"] })).unwrap_err();
    assert!(err.contains("No such position"), "{err}");
    assert!(yours(&g).is_empty());

    // The buttons that set a whole board name no position and still write
    // every one on it, as before.
    let ids = men(&offense(&mut g));
    g.call("board_set", &json!({ "board": "offense", "ids": ids })).unwrap();
    let all = yours(&g);
    for p in [Position::QB, Position::RB, Position::WR, Position::TE, Position::OT, Position::IOL] {
        assert!(all.contains(&p), "a whole-board save writes {p:?}: {all:?}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// Cain, 2026-11-15: a man who is out (hurt, suspended, ineligible) can be
/// put anywhere on your depth chart. He stays where you put him on every
/// view of it, and on game day the next man who can go plays his place.
#[test]
fn a_man_who_is_out_keeps_the_place_you_give_him() {
    let (mut g, dir) = head_coach_world("hurt");
    let flat = |g: &mut Game| -> Vec<u32> {
        let d = g.call("depth", &json!({})).unwrap();
        let p = d["positions"].as_array().unwrap().iter().find(|p| p["pos"] == "TE").unwrap().clone();
        p["players"].as_array().unwrap().iter().map(|x| x["id"].as_u64().unwrap() as u32).collect()
    };
    let sb = |g: &mut Game| -> Vec<Option<u32>> {
        let v = g.call("sb_depth", &json!({ "tab": "offense", "form": "offense" })).unwrap();
        let s = v["slots"].as_array().unwrap().iter().find(|s| s["pos"] == "TE").unwrap().clone();
        s["picks"].as_array().unwrap().iter().map(|p| p["id"].as_u64().map(|x| x as u32)).collect()
    };
    let board_te = |g: &mut Game| {
        offense(g)["slots"].as_array().unwrap().iter().find(|s| s["pos"] == "TE").unwrap()["player"]["id"]
            .as_u64()
            .map(|x| x as u32)
    };
    let game_te = |g: &Game| -> Vec<u32> {
        g.w().unwrap().seat_game_depth().unwrap().into_iter().find(|x| x.0 == Position::TE).unwrap().1
    };
    // The staff's first tight end, sent off hurt for two months.
    let before = flat(&mut g);
    assert!(before.len() >= 3, "three tight ends: {before:?}");
    let te = before[0];
    {
        let w = g.wm().unwrap();
        let today = w.today;
        w.training.hurt.push(afm_engine::training::seat::Hurt {
            id: te,
            kind: None,
            from: today,
            until: today.add(60),
            how: "test".into(),
        });
        w.refresh_seat_team();
    }
    assert!(!game_te(&g).contains(&te), "a man who is out doesn't dress");
    // The staff's chart drops him to the bottom; your Move up, one place at
    // a time as the hover card does, takes him back to the top.
    let mut at = flat(&mut g).iter().position(|&x| x == te).unwrap();
    assert!(at > 0, "the staff plays the next man");
    while at > 0 {
        let mut ids = flat(&mut g);
        ids.swap(at, at - 1);
        g.call("set_depth", &json!({ "pos": "TE", "ids": ids })).unwrap();
        let now = flat(&mut g).iter().position(|&x| x == te).unwrap();
        assert_eq!(now, at - 1, "he moved up one place and stayed there");
        at = now;
    }
    assert_eq!(flat(&mut g)[0], te, "the flat chart keeps him first");
    assert_eq!(board_te(&mut g), Some(te), "the board shows him at his spot");
    assert_eq!(sb(&mut g)[0], Some(te), "the stadium view shows him at his spot");
    let game = game_te(&g);
    assert!(!game.contains(&te), "he doesn't dress while he is out");
    assert_eq!(Some(&game[0]), flat(&mut g).get(1), "the next man in your order plays his place");
    // Back healthy, he plays where you put him.
    {
        let w = g.wm().unwrap();
        w.training.hurt.retain(|h| h.id != te);
        w.refresh_seat_team();
    }
    assert_eq!(game_te(&g)[0], te, "healthy again, he starts");

    // The same on the stadium view: out again, given back to the staff, then
    // dropped into the second string and moved up to start.
    {
        let w = g.wm().unwrap();
        let today = w.today;
        w.training.hurt.push(afm_engine::training::seat::Hurt {
            id: te,
            kind: None,
            from: today,
            until: today.add(60),
            how: "test".into(),
        });
    }
    g.call("sb_depth_reset", &json!({ "tab": "offense", "form": "offense" })).unwrap();
    assert_ne!(sb(&mut g)[0], Some(te), "the staff starts the next man");
    let v = g.call("sb_depth", &json!({ "tab": "offense", "form": "offense" })).unwrap();
    let k = v["slots"].as_array().unwrap().iter().position(|s| s["pos"] == "TE").unwrap();
    g.call("sb_depth_put", &json!({ "tab": "offense", "form": "offense", "slot": k, "n": 2, "pid": te })).unwrap();
    assert_eq!(sb(&mut g)[1], Some(te), "dropped into the second string, he holds it");
    g.call("sb_depth_up", &json!({ "tab": "offense", "form": "offense", "slot": k, "n": 2 })).unwrap();
    assert_eq!(sb(&mut g)[0], Some(te), "moved up, he is the starter");
    assert_eq!(flat(&mut g)[0], te, "and first on the flat chart");
    assert!(!game_te(&g).contains(&te), "still out today");
    let _ = std::fs::remove_dir_all(dir);
}
