//! Facilities lane (Cain's yes 2026-11-08): every FBS program's past
//! facilities come from its real buildings and when they opened, and a
//! project pulls recruits from the day ground breaks.

use crate::world::buildings::{self, BUILDINGS};
use crate::world::data::{COLLEGES, Division};
use crate::world::finance::{Owner, Project, ProjectKind};
use crate::world::program::GradeCat;
use crate::world::{World, WorldOptions, staff};

fn world() -> World {
    World::empty(WorldOptions { scale: 0.01, history_years: 0, start_year: 2026, ..Default::default() })
}

/// The data covers every FBS program by its name in `colleges.csv`, and
/// every row has a source and dates that make sense.
#[test]
fn every_fbs_school_has_its_real_buildings_with_sources() {
    let fbs: Vec<&str> = COLLEGES.iter().filter(|r| r.division == Division::FBS).map(|r| r.name.as_str()).collect();
    for name in BUILDINGS.keys() {
        assert!(fbs.contains(&name.as_str()), "{name} is not an FBS school in colleges.csv");
    }
    let missing: Vec<&&str> = fbs.iter().filter(|n| !buildings::has_buildings(n)).collect();
    assert!(missing.is_empty(), "FBS schools with no buildings: {missing:?}");
    for (name, rows) in BUILDINGS.iter() {
        for b in rows {
            assert!(b.groundbreak <= b.opened, "{name} {}: ground broke after it opened", b.project);
            assert!((1950..=2030).contains(&b.opened), "{name} {}: opened {}", b.project, b.opened);
            assert!(b.value_2024 > 0.0, "{name} {}: no value", b.project);
            assert!(b.source.starts_with("https://"), "{name} {}: no source", b.project);
            // A project inside another's figure: that one is the same
            // school's, has the figure, and carries the money alone.
            if let Some(parent) = &b.cost_in {
                let p = rows.iter().find(|x| &x.project == parent);
                let p = p.unwrap_or_else(|| panic!("{name} {}: no row {parent}", b.project));
                assert!(p.cost.is_some() && b.cost.is_none(), "{name} {}: figures", b.project);
                assert!(b.book_cost() == 0.0 && p.book_cost() == p.cost.unwrap());
            }
        }
    }
}

/// Northwestern reads its real buildings, not its seats: the Walter
/// Athletics Center (2018) puts it in the Big Ten's top half by 2025,
/// where the new 35,000-seat Ryan Field had put it 15th of 18.
#[test]
fn northwestern_reads_its_buildings_not_its_seats() {
    let w = world();
    let big_ten: Vec<usize> = (0..w.programs.len())
        .filter(|&i| COLLEGES.iter().any(|r| r.name == w.programs[i].real_name && r.conference == "Big Ten"))
        .collect();
    let at = |i: usize| {
        let mut p = w.programs[i].clone();
        p.division = Division::FBS;
        p.conference = "Big Ten".into();
        buildings::number(&p, 2025).unwrap()
    };
    let nw = big_ten.iter().copied().find(|&i| w.programs[i].real_name == "Northwestern").unwrap();
    let above = big_ten.iter().filter(|&&i| at(i) > at(nw)).count();
    assert!(above < big_ten.len() / 2, "Northwestern has {above} Big Ten schools above it");
}

/// A building's pull fades as it ages (the half-life), and an opening adds
/// its step the season it opens.
#[test]
fn buildings_age_and_openings_step_up() {
    let rows = buildings::of("Northwestern");
    let walter = rows.iter().position(|b| b.project.contains("Walter")).expect("Walter Athletics Center");
    let b = &rows[walter];
    assert!(b.standing(b.opened) > 0.0 && b.standing(b.opened - 1) == 0.0);
    let half = b.standing(b.opened + buildings::HALF_LIFE as i32);
    assert!((half / b.standing(b.opened) - 0.5).abs() < 0.01);
    let w = world();
    let p = w.programs.iter().find(|p| p.real_name == "Northwestern").unwrap();
    assert!(buildings::step(p, walter) > 0.02, "the Walter Center's step {}", buildings::step(p, walter));
}

/// Ground breaks on a project: the facilities grade moves at once to where
/// the finished building puts it, and the yearly book holds it there.
#[test]
fn a_project_pulls_recruits_from_the_groundbreaking() {
    let mut w = world();
    w.recording = true;
    let k = GradeCat::Facilities as usize;
    let fbs: Vec<usize> = (0..w.programs.len()).filter(|&i| w.programs[i].division == Division::FBS).collect();
    let low = *fbs.iter().min_by(|&&a, &&b| w.programs[a].facilities.total_cmp(&w.programs[b].facilities)).unwrap();
    w.programs[low].grades[k] = 56.0;
    let step = (1.0 - w.programs[low].facilities) as f32;
    w.finance.projects.push(Project {
        owner: Owner::College(low),
        kind: ProjectKind::Facility,
        name: "football building".into(),
        cost: 200.0e6,
        public: 0.0,
        pledged: 120.0e6,
        started: 2026,
        opens: 2028,
        capacity: 0,
        step,
        done: false,
        asked: false,
    });
    // Seats and suites alone (a step of nothing) pull no recruits.
    let other = *fbs.iter().find(|&&i| i != low).unwrap();
    w.finance.projects.push(Project {
        owner: Owner::College(other),
        kind: ProjectKind::Facility,
        name: "stadium seats".into(),
        cost: 100.0e6,
        public: 0.0,
        pledged: 50.0e6,
        started: 2026,
        opens: 2028,
        capacity: 0,
        step: 0.0,
        done: false,
        asked: false,
    });
    let (reads, building) = staff::facilities_reads(&w);
    assert!(building[low] && (reads[low] - 1.0).abs() < 1e-6);
    assert!(!building[other] && reads[other] == w.programs[other].facilities.clamp(0.0, 1.0));
    staff::facilities_grade_now(&mut w, low);
    assert!(w.programs[low].grades[k] > 95.0, "grade {}", w.programs[low].grades[k]);
    // Never lower on the news.
    w.programs[low].grades[k] = 99.0;
    staff::facilities_grade_now(&mut w, low);
    assert_eq!(w.programs[low].grades[k], 99.0);
}
