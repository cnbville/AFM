//! Rating ranges and your staff's quality (update 2, lane 3): what a
//! rating means at a level reads in order (a typical backup under a
//! typical starter under the elite line), a high school pool too small to
//! read widens, the combine's expected numbers move the right way with each
//! rating and with weight, and your read of your staff stays on the 1-20
//! scale, narrows with seasons of work and covers your own staff only. An
//! assistant who moves signs a new deal at his new job's pay.

use super::legacy::seat;
use crate::body::Drills;
use crate::positions::Position;
use crate::world::coach::{Employer, Role};
use crate::world::data::Division;
use crate::world::folk::staffing;
use crate::world::norms::{self, Pool, Rated};
use crate::world::staff_read::{self as sr, Dept};
use crate::world::{Stage, World};

/// The tiny test world has no history, so nobody is in high school or
/// college yet: four high school classes are added and every other player
/// joins a college roster, each position spread over twenty FBS and twenty
/// FCS programs.
fn with_college(w: &mut World) {
    for age in 15..=18 {
        w.add_class_aged(age);
    }
    let tier = |d: Division| -> Vec<usize> {
        (0..w.programs.len()).filter(|&p| w.programs[p].division == d && w.programs[p].plays(w.year)).take(20).collect()
    };
    // FBS and FCS in turn, so a position with fewer players than forty
    // rosters (22 quarterbacks) still gives each level half of them.
    let progs: Vec<usize> =
        tier(Division::FBS).into_iter().zip(tier(Division::FCS)).flat_map(|(a, b)| [a, b]).collect();
    let mut next = std::collections::HashMap::new();
    for i in 0..w.people.len() {
        if w.people[i].stage != Stage::HighSchool {
            continue;
        }
        // The rest play for their high schools (rosters are set on the
        // high school year's own days).
        if i % 2 == 1 {
            let h = w.people[i].hs;
            if !w.high_schools[h].roster.contains(&i) {
                w.high_schools[h].roster.push(i);
            }
            continue;
        }
        let k = next.entry(w.people[i].player.position).or_insert(0usize);
        let c = progs[*k % progs.len()];
        *k += 1;
        w.people[i].stage = Stage::College;
        w.people[i].college = Some(c);
        w.programs[c].roster.push(i);
    }
}

#[test]
fn marks_read_in_order_at_every_level() {
    let mut w = seat("college");
    with_college(&mut w);
    for pool in [Pool::Nfl, Pool::College(0), Pool::College(1)] {
        for pos in [Position::QB, Position::WR, Position::IOL, Position::CB] {
            let m = norms::marks(&w, pool, pos, Rated::Overall).unwrap_or_else(|| panic!("{pool:?} {pos:?}: no marks"));
            assert!(m.backup <= m.starter && m.starter <= m.elite, "{pool:?} {pos:?}: {m:?}");
            assert!(m.starters.windows(2).all(|x| x[0] <= x[1]), "{pool:?} {pos:?}: starters out of order");
            assert_eq!(m.below(0.0), 0.0);
            assert_eq!(m.below(100.0), 1.0);
            let e = m.below(m.elite);
            assert!((0.75..=0.95).contains(&e), "{pool:?} {pos:?}: {e} of starters below the elite line");
        }
    }
    // A high school pool reads against his class and widens when it is too
    // small to say anything.
    let hs = (0..w.people.len()).filter(|&k| w.people[k].stage == Stage::HighSchool).take(40);
    let mut read = 0;
    for k in hs {
        let pool = norms::pool_of(&w, k).expect("a high school player has a pool");
        let pos = w.people[k].player.position;
        if let Some(m) = norms::marks(&w, pool, pos, Rated::Overall) {
            read += 1;
            assert!(!m.starters.is_empty());
            assert!(m.pool != pool || m.starters.len() >= 12, "{pool:?}: used with {} starters", m.starters.len());
        }
    }
    assert!(read > 0, "no high school pool could be read");
}

#[test]
fn the_combine_reads_the_right_way_from_each_rating() {
    let w = seat("college");
    let p = &w.people[0];
    let base = Drills { accel: 40.0, ..Drills::of(&p.player) };
    let at = |f: &dyn Fn(&mut Drills)| {
        let mut d = base;
        f(&mut d);
        d.expected()
    };
    assert!(at(&|d| d.speed = 90.0).forty < at(&|d| d.speed = 50.0).forty, "speed: forty");
    assert!(at(&|d| d.accel = 85.0).forty < at(&|d| d.accel = 45.0).forty, "acceleration: forty");
    let (quick, slow) = (at(&|d| d.agility = 90.0), at(&|d| d.agility = 50.0));
    assert!(quick.shuttle < slow.shuttle && quick.cone < slow.cone, "agility: shuttle and 3-cone");
    assert!(at(&|d| d.strength = 90.0).bench > at(&|d| d.strength = 50.0).bench, "strength: bench");
    let (high, low) = (at(&|d| d.jumping = 90.0), at(&|d| d.jumping = 50.0));
    assert!(high.vertical > low.vertical && high.broad > low.broad, "jumping: vertical and broad");
    // Speed is for his size: the same rating runs slower in a bigger body.
    let light = at(&|d| {
        d.speed = 75.0;
        d.weight = 185.0;
    });
    let heavy = at(&|d| {
        d.speed = 75.0;
        d.weight = 305.0;
    });
    assert!(heavy.forty > light.forty, "weight: forty");
}

#[test]
fn your_read_of_your_staff_stays_on_the_scale_and_narrows_with_seasons() {
    for level in ["college", "nfl", "hs"] {
        let w = seat(level);
        let all = sr::members(&w);
        assert!(all.iter().any(|m| m.dept == Dept::Coaches), "{level}: no coaches on your staff");
        for m in &all {
            assert!(!m.facets.is_empty(), "{level}: {} has no ratings", m.title);
            assert!(
                !m.keys.is_empty() && m.keys.len() <= 2 && m.keys.iter().all(|&k| k < m.facets.len()),
                "{level}: {}",
                m.title
            );
            assert!(sr::member(&w, m.key).is_some(), "{level}: {} isn't found again by his key", m.title);
            for f in 0..m.facets.len() {
                let fc = &m.facets[f];
                let Some((lo, hi)) = sr::now(&w, m, f) else {
                    assert!(
                        !fc.seen() && !fc.why_not.is_empty(),
                        "{level}: {} {} unseen without a reason",
                        m.title,
                        fc.key
                    );
                    continue;
                };
                assert!(1 <= lo && lo <= hi && hi <= 20, "{level}: {} {}: {lo}-{hi}", m.title, fc.key);
                assert!(!sr::read_words(lo, hi).is_empty() && !sr::sure_words(lo, hi).is_empty());
                // Ten seasons of work narrow every read to a few points.
                let (a, b) = sr::read(&w, m, f, w.today.add(10 * 365)).expect("a seen rating stays seen");
                assert!(
                    b - a <= (hi - lo).max(3),
                    "{level}: {} {}: {a}-{b} ten seasons on, {lo}-{hi} now",
                    m.title,
                    fc.key
                );
                if let Some(j) = sr::job_marks(&w, m, f) {
                    assert!(
                        j.n > 0 && j.q25 <= j.q50 && j.q50 <= j.q75 && j.q75 <= j.q90,
                        "{level}: {} {}: {j:?}",
                        m.title,
                        fc.key
                    );
                    assert!((1.0..=20.0).contains(&j.q25) && (1.0..=20.0).contains(&j.q90));
                }
            }
        }
        // Other teams' staffs stay reputation only.
        let mine: Vec<usize> = all.iter().filter_map(|m| m.coach).collect();
        let other = (0..w.coaches.len()).find(|c| !mine.contains(c) && w.coaches[*c].job.is_some());
        if let Some(c) = other {
            assert!(sr::of_coach(&w, c).is_none(), "{level}: another team's coach has a read");
        }
    }
}

#[test]
fn a_coach_who_moves_signs_a_new_deal_at_his_new_jobs_pay() {
    let mut w = seat("college");
    staffing::ensure(&mut w);
    let pay = |w: &World, c: usize| {
        let j = w.coaches[c].job.expect("an assistant job");
        crate::world::staff::salary(w, j.employer, j.role)
    };
    // An older save kept a deal from his last school: it is signed again once.
    let c = w.folk.staff.deals[0].coach;
    w.folk.staff.deals[0].salary = 1000.0;
    w.folk.staff.repriced = false;
    staffing::ensure(&mut w);
    let now = w.folk.staff.deal(c).expect("a deal").salary;
    assert!((0.5..=2.0).contains(&(now / pay(&w, c))), "repriced to {now}, his job pays {}", pay(&w, c));
    // A Division III position coach hired by an FBS program leaves his deal
    // behind and signs one at FBS pay from his first day.
    let college = |c: usize, d: Division| {
        w.coaches[c].job.is_some_and(|j| {
            matches!(j.role, Role::PositionCoach(_))
                && matches!(j.employer, Employer::College(p) if w.programs[p].division == d)
        })
    };
    let c = (0..w.coaches.len())
        .find(|&c| college(c, Division::D3) && w.folk.staff.deal(c).is_some())
        .expect("a Division III position coach with a deal");
    let (old, role) = (w.folk.staff.deal(c).unwrap().salary, w.coaches[c].job.unwrap().role);
    let to = (0..w.programs.len()).rev().find(|&p| w.programs[p].division == Division::FBS).expect("an FBS program");
    crate::world::staff::leave(&mut w, c, "left for another job");
    crate::world::staff::hire(&mut w, c, Employer::College(to), role);
    let now = w.folk.staff.deal(c).expect("a new deal").salary;
    assert!(now > 2.0 * old, "new deal {now}, old {old}");
    assert!((0.5..=2.0).contains(&(now / pay(&w, c))), "new deal {now}, his job pays {}", pay(&w, c));
}
