//! The split ceiling, projects and the trims (the progression update,
//! `docs/design/progression.md` and `progression-everyday.md`).

use crate::attributes::Attr;
use crate::development::{SeasonContext, develop_season};
use crate::player::{Level, Player, PlayingTime, ProspectOptions, generate_prospect};
use crate::positions::Position;
use crate::progression::{self, LIMITED, Room};
use crate::rng::Rng;

fn boy(id: u32, pos: Position) -> Player {
    let mut rng = Rng::new(id);
    generate_prospect(&mut rng, ProspectOptions { id: Some(id), position: Some(pos), age: None, talent_z: None })
}

fn athlete(id: u32) -> Player {
    let mut rng = Rng::new(id ^ 0x77);
    generate_prospect(&mut rng, ProspectOptions { id: Some(id), position: None, age: None, talent_z: None })
}

#[test]
fn limits_never_sit_under_the_ceiling_and_knacks_are_his_own() {
    for id in 0..400u32 {
        let p = athlete(9_000_000 + id);
        assert!(p.prog.drawn);
        for &a in LIMITED.iter() {
            let l = progression::limit(&p, a).unwrap();
            assert!(l >= p.ceilings.get(a) - 1e-9, "{a:?}: limit {l} under ceiling {}", p.ceilings.get(a));
        }
        // The same boy draws the same knacks.
        assert_eq!(athlete(9_000_000 + id).prog.knack, p.prog.knack);
    }
}

#[test]
fn knack_shares_follow_the_design() {
    // Design item 5: about 1 in 5 has it, half middling, about 1 in 3 none.
    let (mut has, mut none, mut n) = (0, 0, 0);
    for id in 0..4000u32 {
        let p = athlete(9_100_000 + id);
        for k in progression::KINDS {
            match progression::tier(progression::knack(&p, k)) {
                0 => has += 1,
                2 => none += 1,
                _ => {}
            }
            n += 1;
        }
    }
    let (has, none) = (has as f64 / n as f64, none as f64 / n as f64);
    assert!((0.14..0.26).contains(&has), "has the knack {has}");
    assert!((0.25..0.40).contains(&none), "no knack {none}");
}

#[test]
fn a_freak_body_keeps_adding_room() {
    // No cap on freaks: past the best boy in the fit the map carries on up.
    let mut p = boy(9_200_001, Position::CB);
    let before = progression::mapped(&p, Attr::ManCoverage);
    for a in [Attr::Speed, Attr::Acceleration, Attr::Agility, Attr::Flexibility, Attr::PlayRecognition] {
        p.ceilings.set(a, 99.0);
    }
    let top = progression::mapped(&p, Attr::ManCoverage);
    assert!(top > before, "{top} against {before}");
    assert!(top >= 80.0, "a freak's tools map high: {top}");
}

#[test]
fn everyday_coaching_closes_room_by_the_knack_and_never_passes_the_limit() {
    let mut gained = Vec::new();
    for id in 0..300u32 {
        let mut p = boy(9_300_000 + id, Position::CB);
        let mut rng = Rng::new(id);
        let mut total = 0.0;
        for age in 14..22 {
            let level = if age < 18 { Level::HS } else { Level::FBS };
            let before = p.ceilings.get(Attr::ManCoverage);
            develop_season(&mut p, &SeasonContext::new(level, 15.0, PlayingTime::Starter), &mut rng);
            let why = crate::development::take_why().unwrap();
            total += why.room_closed;
            let l = progression::limit(&p, Attr::ManCoverage).unwrap();
            assert!(p.ceilings.get(Attr::ManCoverage) <= l + 1e-9);
            let _ = before;
        }
        gained.push((progression::tier(progression::knack(&p, progression::Kind::Coverage)), total));
    }
    let mean = |v: Vec<f64>| v.iter().sum::<f64>() / v.len().max(1) as f64;
    let high = mean(gained.iter().filter(|g| g.0 == 0).map(|g| g.1).collect());
    let low = mean(gained.iter().filter(|g| g.0 == 2).map(|g| g.1).collect());
    assert!(high > 2.0 * low.max(0.01), "knack {high} against none {low}");
}

#[test]
fn a_natural_rise_lifts_the_limit_and_neglect_reopens_room() {
    let mut p = boy(9_400_001, Position::CB);
    let l0 = progression::limit(&p, Attr::ManCoverage).unwrap();
    // A natural rise past the limit (everyday item 2).
    let before = p.ceilings;
    p.ceilings.set(Attr::ManCoverage, (l0 + 5.0).min(99.0));
    progression::settle(&mut p, &before);
    let l1 = progression::limit(&p, Attr::ManCoverage).unwrap();
    assert!(l1 >= p.ceilings.get(Attr::ManCoverage) - 1e-9);
    // Neglect lowers the everyday ceiling, never the limit (item 3).
    p.ceilings.add(Attr::ManCoverage, -6.0);
    let l2 = progression::limit(&p, Attr::ManCoverage).unwrap();
    assert!((l2 - l1).abs() < 0.51, "{l2} against {l1}");
    assert!(progression::room(&p, Attr::ManCoverage) >= 5.4);
}

#[test]
fn a_falling_body_never_takes_ratings_away() {
    let mut p = boy(9_500_001, Position::CB);
    let (r, c) = (p.attributes.get(Attr::ManCoverage), p.ceilings.get(Attr::ManCoverage));
    let before = p.ceilings;
    p.ceilings.add(Attr::Speed, -25.0);
    p.ceilings.add(Attr::Acceleration, -25.0);
    progression::settle(&mut p, &before);
    assert_eq!(p.attributes.get(Attr::ManCoverage), r);
    assert_eq!(p.ceilings.get(Attr::ManCoverage), c);
    assert!(progression::limit(&p, Attr::ManCoverage).unwrap() >= c);
}

#[test]
fn a_project_opens_his_ceiling_and_his_other_skills_pay() {
    // The same corners with and without a man coverage project at 18-21.
    let (mut with, mut without, mut other_with, mut other_without) = (0.0, 0.0, 0.0, 0.0);
    let n = 200u32;
    for id in 0..n {
        for proj in [false, true] {
            let mut p = boy(9_600_000 + id, Position::CB);
            let mut rng = Rng::new(id);
            for _ in 14..18 {
                develop_season(&mut p, &SeasonContext::new(Level::HS, 10.0, PlayingTime::Starter), &mut rng);
            }
            let (m0, z0) = (p.attributes.get(Attr::ManCoverage), p.attributes.get(Attr::ZoneCoverage));
            if proj {
                assert!(progression::start_project(&mut p, Attr::ManCoverage, true));
            }
            for _ in 18..22 {
                develop_season(&mut p, &SeasonContext::new(Level::FBS, 14.0, PlayingTime::Starter), &mut rng);
                let l = progression::limit(&p, Attr::ManCoverage).unwrap();
                assert!(p.ceilings.get(Attr::ManCoverage) <= l + 1e-9);
            }
            let (m, z) = (p.attributes.get(Attr::ManCoverage) - m0, p.attributes.get(Attr::ZoneCoverage) - z0);
            if proj {
                with += m;
                other_with += z;
            } else {
                without += m;
                other_without += z;
            }
        }
    }
    let n = n as f64;
    assert!(with / n > without / n + 3.0, "project {} against {}", with / n, without / n);
    assert!(other_with < other_without, "his other key skills pay: {other_with} against {other_without}");
}

#[test]
fn room_counts_are_cains() {
    assert_eq!(Room::Qb.slots(), 1);
    assert_eq!(Room::Rb.slots(), 1);
    assert_eq!(Room::Wr.slots(), 2);
    assert_eq!(Room::Te.slots(), 2);
    assert_eq!(Room::Ol.slots(), 2);
    assert_eq!(Room::Dl.slots(), 2);
    assert_eq!(Room::Lb.slots(), 1);
    assert_eq!(Room::Cb.slots(), 2);
    assert_eq!(Room::S.slots(), 1);
    assert_eq!(Room::of(Position::K), None);
    assert_eq!(Room::of(Position::LS), None);
}

#[test]
fn trims_land_at_his_spot_and_keep_the_limit_where_it_was() {
    let mut checked = 0;
    assert!(progression::trim(Position::CB, Attr::ManCoverage)[0] > 0.0);
    // Nothing at 60, something at 70, and a higher natural ceiling always
    // keeps a higher trimmed one.
    let at = |c| progression::trim_at(Position::CB, Attr::ManCoverage, c);
    assert!(at(60.0) == 0.0 && at(70.0) > 0.0);
    for pos in Position::ALL {
        for a in progression::LIMITED {
            let at = |c: f64| progression::trim_at(pos, a, c);
            assert!((0..60).all(|i| {
                let c = 50.0 + i as f64;
                at(c) >= 0.0 && c + 1.0 - at(c + 1.0) > c - at(c)
            }));
        }
    }
    for id in 0..2000u32 {
        let mut p = boy(9_700_000 + id, Position::CB);
        let (c, l) = (p.ceilings.get(Attr::ManCoverage), progression::limit(&p, Attr::ManCoverage).unwrap());
        progression::land_trims(&mut p);
        if c > progression::TRIM_FROM {
            let t = at(c);
            assert!((p.ceilings.get(Attr::ManCoverage) - (c - t)).abs() < 1e-4, "{c} less {t}");
            assert!(progression::limit(&p, Attr::ManCoverage).unwrap() >= l - 0.51);
            checked += 1;
        } else {
            assert_eq!(p.ceilings.get(Attr::ManCoverage), c);
        }
        // Landed once.
        let c1 = p.ceilings.get(Attr::ManCoverage);
        progression::land_trims(&mut p);
        assert_eq!(p.ceilings.get(Attr::ManCoverage), c1);
    }
    assert!(checked > 20);
}

#[test]
fn closing_slows_near_the_top_and_never_stops() {
    assert_eq!(progression::near_top(50.0), 1.0);
    assert!((progression::near_top(60.0) - 0.5).abs() < 1e-9);
    assert!(progression::near_top(80.0) < progression::near_top(70.0));
    assert!(progression::near_top(140.0) > 0.0);
}

#[test]
fn an_older_save_draws_it_once_from_where_he_is() {
    let mut p = boy(9_800_001, Position::WR);
    p.prog = Default::default();
    let c = p.ceilings;
    progression::ensure(&mut p);
    assert!(p.prog.drawn);
    progression::land_trims(&mut p);
    assert_eq!(p.ceilings, c, "no trim on a player from an older save");
    let bytes = rmp_serde::to_vec_named(&p).unwrap();
    let back: Player = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(back.prog, p.prog);
}

/// One receiver's four college years at 18-22 with or without a strength
/// project: (strength ceiling, weight, expected forty) at 22.
fn receiver_years(id: u32, project: bool, knack: f64) -> (f64, f64, f64) {
    let mut p = boy(id, Position::WR);
    let mut rng = Rng::new(id ^ 0xB0D);
    for _ in 14..18 {
        develop_season(&mut p, &SeasonContext::new(Level::HS, 10.0, PlayingTime::Starter), &mut rng);
    }
    progression::ensure(&mut p);
    p.prog.knack[6] = (knack * 25.0).round() as i8;
    if project {
        assert!(progression::start_body(&mut p, progression::BodyKind::Strength, true));
    }
    for _ in 18..22 {
        develop_season(&mut p, &SeasonContext::new(Level::FBS, 12.0, PlayingTime::Starter), &mut rng);
    }
    let d = crate::body::Drills::of(&p).expected();
    (p.ceilings.get(Attr::Strength), crate::body::body_now(&p).weight, d.forty)
}

#[test]
fn a_strength_project_builds_a_big_responder_and_barely_moves_a_non_responder() {
    // The design's lanky burner (item 2's example): a big responder's
    // strength ceiling rises about 14, he carries about 15 lb more and runs
    // about 0.05 slower; a non-responder gains almost nothing.
    let n = 120u32;
    let mut out = Vec::new();
    for knack in [1.6, -1.6] {
        let (mut ds, mut dw, mut df) = (0.0, 0.0, 0.0);
        for id in 0..n {
            let a = receiver_years(9_900_000 + id, false, knack);
            let b = receiver_years(9_900_000 + id, true, knack);
            ds += b.0 - a.0;
            dw += b.1 - a.1;
            df += b.2 - a.2;
        }
        let n = n as f64;
        out.push((ds / n, dw / n, df / n));
        eprintln!("knack {knack}: strength ceiling {:+.1}, weight {:+.1} lb, forty {:+.3}", ds / n, dw / n, df / n);
    }
    let (big, none) = (out[0], out[1]);
    assert!(big.0 > 9.0 && big.0 < 16.0, "big responder {:.1}", big.0);
    assert!(big.1 > 8.0 && big.1 < 20.0, "big responder's pounds {:.1}", big.1);
    assert!(big.2 > 0.02 && big.2 < 0.09, "big responder's forty {:+.3}", big.2);
    assert!(none.0 < 1.5 && none.1 < 3.0, "non-responder {:.1} and {:.1} lb", none.0, none.1);
}

fn on(eye: f64, level: Level, years: u32) -> progression::ReadOn {
    progression::ReadOn { eye, coaching: 12.0, level, years, seen_top: 60.0 }
}

#[test]
fn your_staffs_project_read_is_a_range_on_its_own_view() {
    // The screens' read (your development projects): the low end never
    // tops the high end, the same stream gives the same read and
    // staff_read still multiplies the two halves.
    for id in 0..60u32 {
        let mut p = boy(9_000 + id, Position::CB);
        progression::ensure(&mut p);
        let skills = progression::project_skills(&p);
        assert!(skills.contains(&Attr::ManCoverage), "a corner's pool has man coverage: {skills:?}");
        for &a in &skills {
            let read = |eye: f64| progression::your_project_read(&p, a, on(eye, Level::FBS, 3), &mut Rng::new(id));
            let (lo, hi, _) = read(10.0);
            assert!(lo >= 0.0 && lo <= hi, "{a:?}: {lo} to {hi}");
            assert_eq!(read(10.0), read(10.0));
            let (room, seen) = progression::staff_view(&p, a, 1.0, 10.0, &mut Rng::new(id));
            let r = progression::staff_read(&p, a, 1.0, 10.0, &mut Rng::new(id));
            assert_eq!(r, room.max(0.0) * progression::pace(seen));
        }
        // A strength read narrows after years on it and never goes negative.
        let (rows, lbs) = progression::your_body_read(&p, progression::BodyKind::Strength, 10.0, 10.0, 3);
        assert!(rows.iter().all(|r| r.1 >= 0.0 && r.1 <= r.2 + 1e-9));
        assert!(lbs.0 <= lbs.1 + 1e-9 && lbs.1 <= progression::BODY_LBS);
    }
    // A sharper eye sees his room with less noise.
    assert!(progression::room_noise(20.0) < progression::room_noise(10.0));
    // Past his fourth pro season a project reads nothing.
    let mut p = boy(9_999, Position::WR);
    progression::ensure(&mut p);
    let mut rng = Rng::new(5);
    for _ in 0..5 {
        develop_season(&mut p, &SeasonContext::new(Level::NFL, 10.0, PlayingTime::Starter), &mut rng);
    }
    assert!(p.history.iter().filter(|h| h.level == Level::NFL).count() >= 4);
    let (_, hi, _) = progression::your_project_read(&p, Attr::RouteRunning, on(10.0, Level::NFL, 4), &mut Rng::new(1));
    assert_eq!(hi, 0.0);
}

#[test]
fn an_ai_staff_adopts_a_project_your_staff_set() {
    // You leave the seat and an AI staff takes your team: from its next
    // checkpoint it adopts the projects, and its dead-end rule judges them
    // like its own.
    let staff = progression::staff_code(progression::StaffAt::College, 7);
    let mut a = boy(9_500, Position::S);
    progression::ensure(&mut a);
    assert!(progression::start_project(&mut a, Attr::Tackling, true));
    assert!(progression::start_body(&mut a, progression::BodyKind::Strength, true));
    a.prog.staff = staff;
    let out = progression::ai_room(&[&a], Room::S, progression::Lean::BALANCED, 10.0, 1.0, staff, 3);
    assert!(out.contains(&(0, progression::Change::Adopt)), "{out:?}");
    for (_, c) in out {
        progression::apply(&mut a, c);
    }
    assert!(!a.prog.yours && !a.prog.body_yours);
}

#[test]
fn a_project_ends_when_he_moves_to_a_new_staff() {
    // A project is his staff's plan: a man who moves (a signing, a
    // transfer, the draft) brings none, holds no slot, and his new staff
    // picks at its next checkpoint like anyone else's.
    let old = progression::staff_code(progression::StaffAt::HighSchool, 3);
    let new = progression::staff_code(progression::StaffAt::College, 3);
    assert_ne!(old, new);
    let mut a = boy(9_600, Position::S);
    progression::ensure(&mut a);
    assert!(progression::start_project(&mut a, Attr::Tackling, false));
    assert!(progression::start_body(&mut a, progression::BodyKind::Strength, false));
    a.prog.staff = old;
    assert!(a.prog.left_behind(new) && !a.prog.left_behind(old));
    let out = progression::ai_room(&[&a], Room::S, progression::Lean::DEVELOPER, 10.0, 1.0, new, 3);
    assert!(out.contains(&(0, progression::Change::End)), "{out:?}");
    assert!(out.contains(&(0, progression::Change::EndBody)), "{out:?}");
    assert!(!out.iter().any(|c| matches!(c.1, progression::Change::Start(_) | progression::Change::StartBody(_))));
    for (_, c) in out {
        progression::apply(&mut a, c);
    }
    assert!(a.prog.project().is_none() && a.prog.body_project().is_none());
    assert_eq!(a.prog.staff, new);
    // On his old staff it would have run on.
    let mut b = boy(9_601, Position::S);
    progression::ensure(&mut b);
    assert!(progression::start_project(&mut b, Attr::Tackling, false));
    b.prog.staff = old;
    assert!(progression::moved(&b, old).is_empty());
    // An older save's project (no staff yet) is taken as this staff's.
    let mut c = boy(9_602, Position::S);
    progression::ensure(&mut c);
    assert!(progression::start_project(&mut c, Attr::Tackling, false));
    assert_eq!(progression::moved(&c, new), vec![progression::Change::Staff(new)]);
    // The class sim: a new level is a new staff.
    progression::ai_single(&mut b, progression::Lean::BALANCED, 10.0, false, 10.0, new);
    assert_eq!(b.prog.staff, new);
    assert!(b.prog.project() != Some(Attr::Tackling) || b.prog.project_years == 0);
}
