//! Coach and player bonds (`world/folk/bonds.rs`, `docs/design/coach-bonds.md`):
//! the stages and kinds, weekly growth on a small world with the caps, the
//! moments, and the reads the built systems take.

use crate::game::{OffScheme, synthetic::synthetic_team};
use crate::player::Level;
use crate::positions::Position;
use crate::world::data::Division;
use crate::tests::legacy::seat;
use crate::world::folk::bonds::{self, Bond, Kind, Met, Step, What};
use crate::world::{Stage, World};

fn b(close: f32, trust: f32, hard: bool) -> Bond {
    Bond { coach: 0, close, best: close, trust, fit: 1.0, met: Met::Inherited, flags: hard as u8, extra: 0, weeks: 0 }
}

/// A small world with your career at an FBS program and rooms of
/// sophomores on yours and two others (the empty world has no players).
fn world() -> World {
    let mut w = seat("college");
    let mine = w.seat.and_then(|s| s.program).expect("your program");
    let mut progs = vec![mine];
    progs.extend(
        (0..w.programs.len())
            .filter(|&j| j != mine && w.programs[j].division == Division::FBS && w.programs[j].head_coach.is_some())
            .take(2),
    );
    w.add_class_aged(19);
    let free: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).collect();
    let mut k = 0;
    for &j in &progs {
        for n in 0..44 {
            let Some(&i) = free.get(k) else { break };
            k += 1;
            let q = &mut w.people[i];
            q.stage = Stage::College;
            q.college = Some(j);
            q.college_seasons = 1;
            q.player.position = Position::ALL[n % Position::ALL.len()];
            w.programs[j].roster.push(i);
        }
    }
    w
}

/// Run the bonds' weeks from fall camp on.
fn weeks(w: &mut World, n: i32) {
    let start = w.dates.fall_camp;
    for k in 0..n {
        let d = start.add(7 * k);
        w.today = d;
        bonds::week(w, d);
    }
}

/// College players with at least one coach working with them.
fn college_players(w: &World) -> Vec<usize> {
    (0..w.people.len())
        .filter(|&i| w.people[i].stage == Stage::College && !bonds::coaches_of(w, i).is_empty())
        .collect()
}

#[test]
fn stages_need_trust_and_a_hard_moment_at_the_top() {
    assert_eq!(Step::of(&b(0.1, 0.9, true)), Step::JustMet);
    assert_eq!(Step::of(&b(0.2, 0.2, false)), Step::KnowsYou);
    assert_eq!(Step::of(&b(0.4, 0.5, false)), Step::KnowsYou, "Trusts you needs trust 0.55");
    assert_eq!(Step::of(&b(0.4, 0.6, false)), Step::TrustsYou);
    assert_eq!(Step::of(&b(0.6, 0.7, false)), Step::InHisCorner);
    assert_eq!(Step::of(&b(0.9, 0.9, false)), Step::InHisCorner, "the top stage needs a hard moment");
    assert_eq!(Step::of(&b(0.9, 0.9, true)), Step::Family);
    assert_eq!(Step::Family.words(Kind::Father), "Like a father");
    assert_eq!(Step::Family.words(Kind::Grandfather), "Like a grandfather");
    assert_eq!(Step::Family.words(Kind::Brother), "Like a big brother");
    // The coach's way with people.
    assert!((bonds::rel_factor(1.0) - 0.6).abs() < 1e-6);
    assert!((bonds::rel_factor(10.0) - 1.0).abs() < 1e-6);
    assert!((bonds::rel_factor(20.0) - 1.4).abs() < 1e-6);
    assert_eq!(bonds::raised_from("grandparents, poor").map(|x| x.1), Some(true));
    assert!(bonds::raised_from("").is_none());
}

#[test]
fn need_and_kind_by_his_home_and_the_age_gap() {
    let w = world();
    let mut needs = Vec::new();
    for i in college_players(&w) {
        let (n, dad) = bonds::need(&w.people[i]);
        assert!((0.9..=1.5).contains(&n), "need {n}");
        if dad {
            assert!((n - 0.9).abs() < 1e-6);
        }
        needs.push(n);
        for (c, _) in bonds::coaches_of(&w, i) {
            let gap = w.coaches[c].age as i32 - w.people[i].player.age as i32;
            let k = bonds::kind(&w, i, c);
            match gap {
                g if g < 12 => assert_eq!(k, Kind::Brother),
                g if g >= 40 => assert_eq!(k, Kind::Grandfather),
                _ => assert!(matches!(k, Kind::Father | Kind::Family)),
            }
        }
    }
    assert!(!needs.is_empty());
    assert!(needs.iter().any(|&n| n > 1.0), "some kids need a father figure more");
}

#[test]
fn bonds_grow_with_time_inside_the_caps() {
    let mut w = world();
    let players = college_players(&w);
    assert!(players.len() > 50, "{} college players", players.len());
    weeks(&mut w, 30);
    let (mut head, mut pos, mut n) = (0.0f64, 0.0f64, 0usize);
    let mut any = 0;
    for &i in &players {
        let id = w.people[i].player.id;
        let Some(list) = w.folk.bonds.by.get(&id) else { continue };
        for b in list {
            assert!(b.close >= 0.0 && b.close <= bonds::TIME_TOP + 1e-6, "time alone tops out: {}", b.close);
            assert!(b.close <= b.trust + bonds::OVER_TRUST + 1e-6, "close {} over trust {}", b.close, b.trust);
            assert!((0.6..=1.4).contains(&b.fit), "fit {}", b.fit);
            if b.close > 0.0 {
                any += 1;
            }
        }
        // Position coach against head coach, same player.
        let now = bonds::coaches_of(&w, i);
        let h = now.iter().find(|x| x.1 == bonds::Seat::Head).map(|x| x.0);
        let p = now.iter().find(|x| x.1 == bonds::Seat::Position).map(|x| x.0);
        if let (Some(h), Some(p)) = (h, p) {
            head += bonds::close(&w, id, Some(h)) as f64;
            pos += bonds::close(&w, id, Some(p)) as f64;
            n += 1;
        }
    }
    assert!(any > players.len() / 2, "{any} bonds grew");
    assert!(n > 20, "{n} players with both coaches");
    assert!(pos > head * 1.3, "the position coach sees him most: {:.3} vs {:.3}", pos / n as f64, head / n as f64);
}

#[test]
fn weeks_are_the_same_on_any_run() {
    let mut a = world();
    let mut b = world();
    weeks(&mut a, 6);
    weeks(&mut b, 6);
    assert_eq!(serde_json::to_string(&a.folk.bonds).unwrap(), serde_json::to_string(&b.folk.bonds).unwrap());
}

/// One of your own players with a bond to you.
fn mine(w: &World) -> (u32, usize) {
    let me = w.career.as_ref().unwrap().coach;
    let i = college_players(w)
        .into_iter()
        .find(|&i| bonds::coaches_of(w, i).iter().any(|x| x.0 == me))
        .expect("a player of yours");
    (w.people[i].player.id, me)
}

#[test]
fn a_hard_moment_waits_for_you_and_counts_when_you_sit_down() {
    let mut w = world();
    weeks(&mut w, 4);
    let (id, me) = mine(&w);
    let before = bonds::close(&w, id, Some(me));
    // Raise his trust so the move isn't held under the trust cap.
    if let Some(list) = w.folk.bonds.by.get_mut(&id) {
        for b in list.iter_mut() {
            b.trust = 0.9;
        }
    }
    bonds::hard_moment(&mut w, id, What::LossThere, What::LossAlone, 0.2, -0.08);
    assert!(w.folk.bonds.waiting.iter().any(|x| x.id == id && x.coach == me as u32), "waits on you");
    assert!(!bonds::bond(&w, id, me).unwrap().hard());
    bonds::sat_down(&mut w, id);
    let after = bonds::bond(&w, id, me).unwrap();
    assert!(after.hard(), "being there opens the top stage");
    assert!(after.close > before + 0.19, "{before} -> {}", after.close);
    assert!(w.folk.bonds.waiting.iter().all(|x| x.id != id));
    let ms = &w.folk.bonds.moments[&id];
    assert!(ms.iter().any(|m| m.what == What::LossThere && m.coach == me as u32));
    assert_eq!(bonds::with_you(&w, id).map(|x| x.1.contains(&What::LossThere)), Some(true));
}

#[test]
fn a_betrayal_hurts_more_the_closer_he_was() {
    let mut w = world();
    weeks(&mut w, 4);
    let (id, me) = mine(&w);
    if let Some(b) = w.folk.bonds.by.get_mut(&id).and_then(|l| l.iter_mut().find(|b| b.coach == me as u32)) {
        b.close = 0.6;
        b.trust = 0.8;
    }
    assert!((bonds::betrayal(&w, id, Some(me)) - 1.6).abs() < 1e-5);
    assert!((bonds::loss_risk(&w, id) - 1.0).abs() < 1e-6, "nobody left him yet");
    bonds::left_him(&mut w, id, Some(me), What::Cut);
    let b = bonds::bond(&w, id, me).unwrap();
    assert!(b.close < 0.6 - 0.15 * 1.6 + 0.01, "a cut costs x(1+close): {}", b.close);
    assert!(w.folk.bonds.burned.contains_key(&id), "burned: guarded with the next coach");
}

#[test]
fn the_room_runs_colder_for_the_next_coach_and_close_men_stay() {
    let mut w = world();
    weeks(&mut w, 4);
    let (id, me) = mine(&w);
    let i = crate::world::folk::person(&w, id).unwrap();
    let j = w.people[i].college.unwrap();
    let roster: Vec<usize> = (0..w.people.len()).filter(|&k| w.people[k].college == Some(j)).collect();
    for &k in &roster {
        let pid = w.people[k].player.id;
        if let Some(b) = w.folk.bonds.by.get_mut(&pid).and_then(|l| l.iter_mut().find(|b| b.coach == me as u32)) {
            b.close = 0.5;
            b.trust = 0.8;
        }
    }
    let cold = bonds::successor_cold(&w, &roster, me as u32);
    assert!(cold > 0.03 && cold <= 0.15 + 1e-6, "cold {cold}");
    assert!(bonds::stay_factor(&w, i) < 0.99, "a man close to his coaches stays");
    assert!(bonds::coachability_lift(&w, i) >= 0.0 && bonds::coachability_lift(&w, i) <= 3.0);
}

#[test]
fn the_engine_reads_trust_alone_without_a_bond() {
    let t = synthetic_team("A", Level::NFL, 0.5, OffScheme::WestCoast, 3);
    for p in &t.players {
        assert_eq!(p.bond, 0.0);
        assert!((p.obey_trust() - p.trust).abs() < 1e-12, "no bond, no change to the engine");
    }
    let mut p = t.players[0].clone();
    p.bond = 0.6;
    assert!((p.obey_trust() - p.trust - 0.4).abs() < 1e-6);
}

#[test]
fn an_empty_store_is_not_written() {
    let w = world();
    let v = serde_json::to_value(&w.folk).unwrap();
    assert!(v.get("bonds").is_none());
    let mut w = w;
    weeks(&mut w, 2);
    let s = serde_json::to_string(&w.folk.bonds).unwrap();
    let back: bonds::Bonds = serde_json::from_str(&s).unwrap();
    assert_eq!(back.by.len(), w.folk.bonds.by.len());
}
