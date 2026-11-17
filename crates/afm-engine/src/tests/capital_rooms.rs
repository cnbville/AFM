//! Capital lane C2: the rooms and the campus (`docs/design/capital.md`,
//! CP17-CP19, CP21-CP23, CP26 and campus life in recruiting).

use crate::world::buildings::{self, Kind};
use crate::world::capital::rooms::{self, AGE_A_YEAR, ROOMS, Room};
use crate::world::data::Division;
use crate::world::finance::{Owner, Project, ProjectKind};
use crate::world::{World, WorldOptions};

fn world() -> World {
    World::empty(WorldOptions { scale: 0.01, history_years: 0, start_year: 2026, ..Default::default() })
}

/// The world at its handover season, its rooms live.
fn live() -> World {
    let mut w = world();
    w.year = w.options.start_year;
    rooms::age(&mut w);
    w
}

fn fbs(w: &World, name: &str) -> usize {
    w.programs.iter().position(|p| p.real_name == name && p.division == Division::FBS).expect(name)
}

/// The thirteen rooms keep the spec's CP17 order (lane C1's donors store a
/// room want as its index) and share out the whole number.
#[test]
fn the_rooms_keep_the_spec_order_and_share_the_number() {
    assert_eq!(Room::ALL.len(), ROOMS);
    for (k, r) in Room::ALL.iter().enumerate() {
        assert_eq!(*r as usize, k);
        assert_eq!(Room::from_index(k as u8), Some(*r));
    }
    assert_eq!(Room::Locker as u8, 0);
    assert_eq!(Room::Stadium as u8, 12);
    let total: f64 = Room::ALL.iter().map(|r| r.weight()).sum();
    assert!((total - 1.0).abs() < 1e-9, "weights sum to {total}");
}

/// Every program's rooms give back its facilities number (CP19: the old
/// number is the trade's read, worked out from the rooms), and every room
/// has a read and a typical value once the rooms are read.
#[test]
fn every_program_s_rooms_give_its_number() {
    let w = world();
    for p in &w.programs {
        assert!(p.campus.set, "{} has no rooms", p.real_name);
        let gap = (p.campus.number() - p.facilities).abs();
        assert!(gap < 1e-4, "{}: rooms {} against {}", p.real_name, p.campus.number(), p.facilities);
        for r in Room::ALL.iter().filter(|r| r.everyday()) {
            assert!(p.campus.level(*r) > 0.0, "{} has no {}", p.real_name, r.words());
        }
    }
    let osu = &w.programs[fbs(&w, "Ohio State")];
    assert!(osu.campus.typical.iter().all(|&t| t >= 0.0) && osu.campus.typical[Room::Weight as usize] > 0.0);
    assert!(!osu.campus.halls.is_empty());
}

/// When every room stands at the number, each shared-out effect is the
/// number itself (CP19: the rooms share out today's effect and add nothing
/// on top), and every new tool reads neutral against a typical room.
#[test]
fn even_rooms_give_today_s_effects_exactly() {
    let mut w = world();
    let j = fbs(&w, "Iowa");
    let p = &mut w.programs[j];
    p.campus.level = [3.0; ROOMS];
    p.campus.typical = [0.6; ROOMS];
    p.campus.read = [4.0; ROOMS];
    p.campus.typical_read = [4.0; ROOMS];
    p.campus.temporary = 0;
    let n = p.campus.number();
    assert!((n - 0.6).abs() < 1e-6);
    for f in [rooms::physical(p), rooms::teaching(p), rooms::academic(p), rooms::film(p)] {
        assert!((f - n).abs() < 1e-6, "{f} against {n}");
    }
    let e = 1e-6;
    assert!(rooms::visit_lift(p).abs() < e && rooms::game_day_lift(p).abs() < e);
    assert!((rooms::soft_tissue_guard(p) - 1.0).abs() < e && rooms::recovery_days(p).abs() < e);
    assert!((rooms::nil_lift(p) - 1.0).abs() < e && rooms::mood_lift(p).abs() < e);
    assert!((rooms::pitch_lift(p) - 1.0).abs() < e && (rooms::body_plan_rate(p) - 1.0).abs() < e);
    // A weight room two levels above the rest puts its gain in the body.
    p.campus.level[Room::Weight as usize] = 5.0;
    assert!(rooms::physical(p) > rooms::teaching(p) + 0.2);
}

/// A real building holds its kind's rooms and what its note names.
#[test]
fn real_buildings_hold_their_rooms() {
    let all: Vec<_> = buildings::BUILDINGS.values().flatten().collect();
    for b in all.iter().filter(|b| b.kind == Kind::IndoorPractice) {
        assert!(rooms::rooms_of(b) & (1 << Room::IndoorField as u16) != 0, "{}", b.project);
    }
    for b in all.iter().filter(|b| b.kind == Kind::FootballBuilding) {
        let m = rooms::rooms_of(b);
        for r in [Room::Locker, Room::Weight] {
            assert!(m & (1 << r as u16) != 0, "{} has no {}", b.project, r.words());
        }
    }
    // Every stadium project is a stadium room.
    for b in all.iter().filter(|b| b.kind == Kind::Stadium) {
        assert!(rooms::rooms_of(b) & (1 << Room::Stadium as u16) != 0);
    }
    // More money a room, a higher level, between 1.5 and 5.
    assert!(rooms::built_level(25.0e6) > 4.9 && rooms::built_level(4.0e6) == 3.0);
    assert!(rooms::built_level(0.1e6) == 1.5);
}

/// The rooms live from the season the world hands over: each year a room
/// loses a tenth of a level (CP22, a level about every ten years), and the
/// number is read from them.
#[test]
fn live_rooms_age_a_level_a_decade() {
    let mut w = live();
    let j = fbs(&w, "Oregon");
    assert!(w.programs[j].campus.live);
    w.programs[j].campus.level[Room::Weight as usize] = 5.0;
    let before = w.programs[j].campus.number();
    for _ in 0..10 {
        rooms::age(&mut w);
    }
    let after = w.programs[j].campus.level(Room::Weight);
    assert!((after - (5.0 - 10.0 * AGE_A_YEAR as f64)).abs() < 1e-4, "after ten years {after}");
    assert!(w.programs[j].facilities < before);
    assert!((w.programs[j].facilities - w.programs[j].campus.number()).abs() < 1e-9);
}

/// A room is read against the best in its conference (CP23): the best
/// reads 5, a room a level behind reads 4.
#[test]
fn rooms_read_against_the_conference_best() {
    let mut w = world();
    let year = w.year;
    let sec: Vec<usize> =
        (0..w.programs.len()).filter(|&i| w.programs[i].conference == "SEC" && w.programs[i].plays(year)).collect();
    let (a, b) = (sec[0], sec[1]);
    for &i in &sec {
        w.programs[i].campus.level[Room::Weight as usize] = 3.0;
    }
    w.programs[a].campus.level[Room::Weight as usize] = 5.0;
    w.programs[b].campus.level[Room::Weight as usize] = 4.0;
    rooms::read_all(&mut w);
    assert_eq!(w.programs[a].campus.shown(Room::Weight), 5);
    assert_eq!(w.programs[b].campus.shown(Room::Weight), 4);
    assert_eq!(w.programs[sec[2]].campus.shown(Room::Weight), 3);
    // Georgia-style: a rival builds a better one and yours reads lower.
    w.programs[sec[2]].campus.level[Room::Weight as usize] = 5.0;
    w.programs[a].campus.level[Room::Weight as usize] = 4.1;
    rooms::read_all(&mut w);
    assert_eq!(w.programs[a].campus.shown(Room::Weight), 4);
}

/// A renovation lifts its one room a level; a new building lifts its rooms
/// to 4 or 5 and goes on the map (CP18, CP21); `rise` says what it adds.
#[test]
fn renovations_and_new_buildings_lift_rooms() {
    let mut w = live();
    let j = fbs(&w, "Kansas");
    w.programs[j].campus.level[Room::Locker as usize] = 2.0;
    let name = rooms::renovation_name(Room::Locker);
    let r = rooms::rise(&w.programs[j], &name, 5.0e6, 2026);
    let before = w.programs[j].campus.number();
    rooms::open(&mut w, j, &name, 5.0e6, 2026);
    assert!((w.programs[j].campus.level(Room::Locker) - 3.0).abs() < 1e-6);
    assert!((w.programs[j].facilities - before - r).abs() < 1e-6);
    let halls = w.programs[j].campus.halls.len();
    rooms::open(&mut w, j, "football building", 180.0e6, 2026);
    for rm in [Room::Weight, Room::Nutrition, Room::RecruitingLounge] {
        assert!(w.programs[j].campus.level(rm) >= 4.0, "{} at {}", rm.words(), w.programs[j].campus.level(rm));
    }
    assert_eq!(w.programs[j].campus.halls.len(), halls + 1);
    assert_eq!(w.programs[j].campus.since[Room::Weight as usize], 2026);
}

/// In the build year the team works in temporary space (CP26): its rooms
/// read a level lower for their tools and the room's mood pays a little;
/// the stadium and a summer renovation don't.
#[test]
fn the_build_year_is_temporary_space() {
    let mut w = live();
    let j = fbs(&w, "Purdue");
    let year = w.year;
    rooms::read_all(&mut w);
    let mood = rooms::mood_lift(&w.programs[j]);
    let weight = w.programs[j].campus.value(Room::Weight);
    let push = |w: &mut World, name: &str, opens: i32| {
        w.finance.projects.push(Project {
            owner: Owner::College(j),
            kind: ProjectKind::Facility,
            name: name.into(),
            cost: 1.0e8,
            public: 0.0,
            pledged: 0.0,
            started: year - 1,
            opens,
            capacity: 0,
            step: 0.1,
            done: false,
            asked: false,
        });
    };
    push(&mut w, &rooms::renovation_name(Room::Locker), year + 1);
    rooms::read_all(&mut w);
    assert!(!w.programs[j].campus.in_temporary_space(), "a summer renovation is no temporary space");
    push(&mut w, "football building", year + 1);
    rooms::read_all(&mut w);
    let c = &w.programs[j].campus;
    assert!(c.in_temporary_space());
    assert!((c.value(Room::Weight) - (weight - 0.2).max(0.0)).abs() < 1e-6);
    assert!(rooms::mood_lift(&w.programs[j]) < mood - 0.03);
}

/// Most recruits weigh campus life a little and some a lot.
#[test]
fn campus_life_matters_a_little_to_most_and_a_lot_to_some() {
    let w = World::empty(WorldOptions { scale: 0.02, history_years: 0, start_year: 2026, ..Default::default() });
    let mut v: Vec<f64> = w.people.iter().take(4000).map(rooms::campus_life_want).collect();
    v.sort_by(f64::total_cmp);
    let median = v[v.len() / 2];
    let top = v[v.len() * 95 / 100];
    assert!(median < 0.06, "median {median}");
    assert!(top > 0.1, "95th {top}");
}

/// You ask for one room's renovation: no room, no renovation; a yes puts
/// it in the books for the summer.
#[test]
fn you_can_ask_for_a_renovation() {
    let mut w = live();
    crate::world::finance::college::open(&mut w);
    let j = fbs(&w, "Rutgers");
    w.programs[j].campus.level[Room::ContentStudio as usize] = 0.0;
    assert!(rooms::ask_renovation(&mut w, j, Room::ContentStudio).is_err());
    w.programs[j].campus.level[Room::Locker as usize] = 2.0;
    w.finance.projects.retain(|x| x.owner != Owner::College(j) || x.done);
    let mut yes = false;
    for _ in 0..12 {
        let said = rooms::ask_renovation(&mut w, j, Room::Locker);
        if said.as_ref().is_ok_and(|s| s.contains("this summer")) {
            yes = true;
            break;
        }
        eprintln!("{said:?}");
        w.year += 1;
    }
    assert!(yes, "never a yes");
    assert!(w.finance.projects.iter().any(|x| x.owner == Owner::College(j) && x.name == "locker room renovation"));
}
