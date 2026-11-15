//! The CFL scouts and drafts (SK77): the class is people before the draft,
//! the Bureau ranks it three times, the clubs file looks all year and pick
//! from their own reads, the Global Draft follows. Builds a small world, so
//! the quick check skips it (full tests run it).

use crate::world::pro::cfl;
use crate::world::pro::other::{self as o, CFL};
use crate::world::{Stage, World, WorldOptions};

fn world() -> World {
    World::new(WorldOptions { seed: 77, scale: 0.004, history_years: 2, engine_from: Some(2100), ..Default::default() })
}

#[test]
fn the_cfl_scouts_and_drafts_its_class() {
    let w = world();
    let year = 2025;
    let teams = o::teams_in(CFL, year).len();
    let picks: Vec<&o::Pick> = w.others.draft.iter().filter(|p| p.year == year && !p.global).collect();
    assert_eq!(picks.len(), cfl::rounds(year, false) as usize * teams, "the {year} draft's picks");
    // Each pick is a person from the class made last August, now on his
    // club's roster or with his rights held (back at school); a few were
    // cut in camp and left football (and the world compacts them away).
    let (mut back, mut gone) = (0, 0);
    for p in &picks {
        let Some(i) = crate::world::scouting::place(&w, p.id) else {
            gone += 1;
            continue;
        };
        let person = &w.people[i];
        assert!(person.notes.iter().any(|n| n.contains("CFL draft")), "{} has no draft note", person.name);
        match person.stage {
            Stage::CflPool => {
                back += 1;
                assert!(w.others.rights.contains_key(&p.id), "{} is back at school with no rights held", person.name);
            }
            Stage::Pro => assert!(w.others.national(p.id), "{} reported but isn't a national", person.name),
            Stage::Done => {
                if gone == 0 {
                    println!("{} drafted and gone: {:?}", person.name, person.notes);
                }
                gone += 1;
            }
            s => panic!("{} drafted and now {s:?}: {:?}", person.name, person.notes),
        }
    }
    println!("{} picks: {back} back at school, {gone} gone", picks.len());
    assert!(gone * 3 < picks.len(), "{gone} of {} picks gone", picks.len());
    assert!(back > 0 && back < picks.len() / 3, "{back} picks back at school");
    // The Bureau's three editions for that draft.
    let eds: Vec<u8> = w.others.bureau.iter().filter(|b| b.draft == year).map(|b| b.edition).collect();
    assert_eq!(eds, vec![0, 1, 2]);
    assert!(w.others.bureau.iter().all(|b| b.ids.len() == 20));
    // The clubs don't draft by the truth: a better man (by his true
    // overall) is passed over somewhere in the first two rounds.
    let first: Vec<f64> = picks
        .iter()
        .take(2 * teams)
        .filter_map(|p| crate::world::scouting::place(&w, p.id).map(|i| w.people[i].player.overall()))
        .collect();
    assert!(first.windows(2).any(|x| x[1] > x[0] + 1.0), "picks fell in true order: {first:?}");
    // The Global Draft: two rounds, mostly kickers and punters.
    let global: Vec<&o::Pick> = w.others.draft.iter().filter(|p| p.year == year && p.global).collect();
    assert_eq!(global.len(), 2 * teams);
    let kickers = global.iter().filter(|p| p.pos.kicks()).count();
    assert!(kickers * 3 >= global.len(), "{kickers} kickers and punters of {}", global.len());
    // Next spring's class is already people with files from the fall.
    let next = cfl::class(&w, year + 1, false);
    assert!(next.len() >= 100, "next class: {}", next.len());
    let filed = w.others.teams.iter().map(|t| t.files.len()).max().unwrap_or(0);
    assert!(filed > 30, "a club's file holds {filed}");
    assert!(w.others.teams[o::teams_in(CFL, year)[0]].scouts.len() == 4);
}
