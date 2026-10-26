//! The search reads what your staff knows, never a true rating.

use afm_engine::attributes::Attr;
use afm_engine::world::scouting::readers::Staff;
use afm_engine::world::{Stage, World};
use serde_json::json;

use super::fields::{self, Ctx, Seat, Sure, Val};
use super::{query, staff};
use crate::Game;
use crate::views::{self, TeamRef};

/// The public marks that come from a kid's body and athletic tools the way
/// a meet, a stopwatch or a scale gives them: they may move when his tools
/// move (his weight carries his strength).
const PUBLIC_MARKS: [&str; 16] = [
    "t40", "t10", "tsh", "t3c", "tvj", "tbj", "tbp", "m100", "m200", "hurdles", "shot", "hoops", "wrestle", "sport",
    "wt_v", "wt",
];

fn college_world(name: &str) -> (Game, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("afm-find-{name}-{}", std::process::id()));
    let mut g = Game::new(dir.clone());
    g.call("build_world", &json!({ "seed": 11, "history_years": 1, "scale": 0.03, "engine_from": 2026 })).unwrap();
    let job = g.call("jobs", &json!({ "level": "college" })).unwrap()["jobs"][0]["ref"].clone();
    g.call(
        "start_career",
        &json!({ "name": "Test Coach", "age": 41, "background": "college_assistant", "job": job, "seed": 9 }),
    )
    .unwrap();
    (g, dir)
}

fn values(w: &World, j: usize, i: usize, catalog: &[fields::Group]) -> Vec<(String, Val, bool)> {
    staff::forget();
    let mut ctx = Ctx::new(w, Seat::College(j), Staff::Program(j), Sure::Any, false);
    ctx.reads = staff::reads(w, Staff::Program(j), &[w.people[i].player.id]);
    ctx.ranks = fields::Ranks::of_class(w, w.people[i].class_of);
    catalog.iter().flat_map(|g| g.fields.iter()).map(|f| (f.k.clone(), fields::value(&ctx, i, &f.k), f.staff)).collect()
}

#[test]
fn a_kid_your_staff_has_not_seen_reads_not_seen() {
    let (mut g, dir) = college_world("unseen");
    let w = g.world.as_ref().unwrap();
    let Some(TeamRef::College(j)) = views::mine(w) else { panic!("a college seat") };
    // A high school kid nobody on your staff has looked at.
    let i = (0..w.people.len())
        .find(|&i| {
            let p = &w.people[i];
            p.stage == Stage::HighSchool && p.stars >= 2 && !staff::has_file(w, Staff::Program(j), p.player.id)
        })
        .expect("an unseen ranked kid");
    let catalog = fields::catalog(Seat::College(j), w);
    let before = values(w, j, i, &catalog);
    for (k, v, staff) in &before {
        // (None: a tendency his position doesn't have.)
        if *staff {
            assert!(matches!(v, Val::Unseen | Val::None), "{k} reads your staff's file, and there is none: {v:?}");
        }
    }
    // Make him a different player underneath: every attribute 25 higher.
    let w = g.world.as_mut().unwrap();
    for a in Attr::ALL {
        let v = w.people[i].player.attributes.get(a);
        w.people[i].player.attributes.set(a, (v + 25.0).min(99.0));
    }
    let w = g.world.as_ref().unwrap();
    let after = values(w, j, i, &catalog);
    let moved: Vec<String> = before
        .iter()
        .zip(&after)
        .filter(|((k, a, _), (_, b, _))| !PUBLIC_MARKS.contains(&k.as_str()) && a != b)
        .map(|((k, a, _), (_, b, _))| format!("{k}: {a:?} to {b:?}"))
        .collect();
    assert!(moved.is_empty(), "moved with his true ratings: {moved:?}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_seen_kid_reads_his_file() {
    let (g, dir) = college_world("seen");
    let w = g.world.as_ref().unwrap();
    let Some(TeamRef::College(j)) = views::mine(w) else { panic!("a college seat") };
    let st = Staff::Program(j);
    let Some(i) = (0..w.people.len()).find(|&i| {
        let p = &w.people[i];
        p.stage == Stage::HighSchool && staff::has_file(w, st, p.player.id)
    }) else {
        // A world with no files yet has nothing to check here.
        let _ = std::fs::remove_dir_all(dir);
        return;
    };
    let id = w.people[i].player.id;
    let (_, _, file) = afm_engine::world::scouting::readers::read(w, st, id).expect("his file");
    staff::forget();
    let mut ctx = Ctx::new(w, Seat::College(j), st, Sure::Any, false);
    ctx.reads = staff::reads(w, st, &[id]);
    for a in Attr::ALL {
        let k = format!("attr:{}", a.key());
        let f = &file.attrs[a.index()];
        match fields::value(&ctx, i, &k) {
            Val::N(x) => assert_eq!(x, f.centre.round().clamp(1.0, 99.0), "{k} is the file's read"),
            Val::Unseen => assert!(!f.seen, "{k} unseen while the file has it"),
            v => panic!("{k}: {v:?}"),
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn and_or_groups_read_as_written() {
    let (g, dir) = college_world("andor");
    let w = g.world.as_ref().unwrap();
    let Some(TeamRef::College(j)) = views::mine(w) else { panic!("a college seat") };
    let catalog = fields::catalog(Seat::College(j), w);
    let q = query::Query::parse(
        &json!([
            { "k": "cstars", "op": "ge", "v": 4 },
            { "k": "state", "op": "is", "v": "TX", "or": true },
            { "k": "miles", "op": "le", "v": 300 },
        ]),
        &catalog,
    );
    assert_eq!(q.words(), "(Composite stars at least 4 or State is TX) and Miles from campus at most 300");
    let ctx = Ctx::new(w, Seat::College(j), Staff::Program(j), Sure::Any, false);
    for (i, p) in w.people.iter().enumerate().filter(|(_, p)| p.stage == Stage::HighSchool).take(3000) {
        let code = w.states.get(p.home_state).map_or("", |s| s.code.as_str());
        let want = (p.stars >= 4 || code == "TX") && ctx.miles(i).round() <= 300.0;
        assert_eq!(q.pass(&ctx, i), want, "{} {code} {}", p.stars, ctx.miles(i));
    }
    let _ = std::fs::remove_dir_all(dir);
}
