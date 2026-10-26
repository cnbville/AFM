//! The search reads what your staff knows, never a true rating.

use afm_engine::attributes::Attr;
use afm_engine::world::scouting::readers::Staff;
use afm_engine::world::scouting::services;
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

    // A school filter matches the whole name (R5): "Michigan" never finds a
    // kid whose only offer is from Michigan State.
    let names: Vec<&str> = (0..w.programs.len()).map(|k| w.program_name(k)).collect();
    let short = names
        .iter()
        .copied()
        .find(|n| names.iter().any(|m| m.starts_with(&format!("{n} "))))
        .expect("a program named inside another's name");
    let pool: Vec<usize> = (0..w.people.len())
        .filter(|&i| w.people[i].stage == Stage::HighSchool && !views::offers_of(&w.people[i]).is_empty())
        .collect();
    for typed in [short.to_string(), short.to_uppercase(), format!("  {short} ")] {
        let q =
            query::Query::parse(&json!([{ "k": "offer_from", "op": "is", "v": typed }]), &catalog).settled(&ctx, &pool);
        let mut longer = 0;
        for &i in &pool {
            let offers = views::offers_of(&w.people[i]);
            let want = offers.iter().any(|&c| w.program_name(c) == short);
            longer += usize::from(!want && offers.iter().any(|&c| w.program_name(c).starts_with(&format!("{short} "))));
            assert_eq!(
                q.pass(&ctx, i),
                want,
                "{typed:?}: {:?}",
                offers.iter().map(|&c| w.program_name(c)).collect::<Vec<_>>()
            );
        }
        println!("{short}: {longer} kids with only a longer name's offer left out");
    }
    // Part of a name (no program's whole name) still finds the names whose
    // words start with it.
    let part: String = short.split_whitespace().next().unwrap_or(short).chars().take(4).collect();
    let whole = names.iter().any(|n| n.eq_ignore_ascii_case(&part));
    let q = query::Query::parse(&json!([{ "k": "offer_from", "op": "is", "v": part }]), &catalog).settled(&ctx, &pool);
    for &i in pool.iter().filter(|_| !whole) {
        let lines: Vec<String> =
            views::offers_of(&w.people[i]).iter().map(|&c| w.program_name(c).to_lowercase()).collect();
        let want = lines.iter().any(|l| l.split_whitespace().any(|w| w.starts_with(&part.to_lowercase())));
        assert_eq!(q.pass(&ctx, i), want, "{part:?}: {lines:?}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_saved_search_tells_each_player_once() {
    let (mut g, dir) = college_world("saved");
    // Everyone your staff knows: more than any cap would keep (R1).
    let page = g.call("find_recruits", &json!({ "filters": [] })).unwrap();
    g.call("find_save", &json!({ "name": "Everyone", "spec": page["spec"] })).unwrap();
    let w = g.world.as_mut().unwrap();
    let k = w.career.as_ref().unwrap().desk.watches.len() - 1;
    let all = w.career.as_ref().unwrap().desk.watches[k].seen.clone();
    assert_eq!(all.len() as u64, page["count"].as_u64().unwrap(), "every match counts as told, none cut");
    assert!(all.windows(2).all(|p| p[0] < p[1]), "kept sorted");
    // As if a third of them were new since the last look.
    let new: Vec<u32> = all.iter().copied().step_by(3).collect();
    w.career.as_mut().unwrap().desk.watches[k].seen.retain(|id| !new.contains(id));
    let mails = |w: &World| {
        w.career
            .as_ref()
            .unwrap()
            .desk
            .mails
            .iter()
            .filter(|m| m.subject.starts_with("Your search \"Everyone\""))
            .count()
    };
    let today = w.today;
    // The last services update before today.
    let u = (today.year() - 1..=today.year())
        .flat_map(services::update_days)
        .map(|u| u.0)
        .filter(|&d| d < today.add(-1))
        .max()
        .expect("an update before today");
    // The update's own day hasn't run while it is today: no look yet (R4).
    w.today = u;
    super::saved::after_days(w, u.add(-1));
    assert_eq!(mails(w), 0, "today's trigger waits for the day to run");
    // Once it has run, one mail names exactly the new ones.
    w.today = today;
    super::saved::after_days(w, u.add(-1));
    assert_eq!(mails(w), 1);
    let x = &w.career.as_ref().unwrap().desk.watches[k];
    assert_eq!(x.fresh, new, "the new ones, each once");
    assert_eq!(x.seen, all, "told: today's matches");
    // The same days again (a step that re-reads them): nothing new, no mail.
    super::saved::after_days(w, u.add(-1));
    assert_eq!(mails(w), 1, "a player already told is never told again");
    let _ = std::fs::remove_dir_all(dir);
}
