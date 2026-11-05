//! The coaching pipeline (minor bugs 252, 243 and 244): an off-field
//! man's standing moves with his staff's seasons, a head coach promotes his
//! own man when he rates him ready, his staff follows him to a new head
//! job, and coaches let go from big staffs wait out a year as analysts.

use std::collections::HashSet;

use crate::rng::Rng;
use crate::world::coach::{CoachTier, Employer, Group, Junior, Post, Role, generate_coach};
use crate::world::data::Division;
use crate::world::ethnicity::Ethnicity;
use crate::world::staff::{analysts_from_the_street, bring_his_staff, hire, leave, ready_at_home, slot};
use crate::world::{World, WorldOptions};

fn tiny() -> World {
    World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() })
}

fn fbs(w: &World) -> Vec<usize> {
    (0..w.programs.len())
        .filter(|&p| w.programs[p].division == Division::FBS && w.programs[p].head_coach.is_some())
        .collect()
}

#[test]
fn an_analysts_standing_moves_with_his_staffs_seasons() {
    let mut rng = Rng::new(11);
    let mut c = generate_coach(0, "Test Analyst".into(), Ethnicity::White, CoachTier::Fbs, 28, &mut rng);
    c.reputation = 3.0;
    let shown = c.ability().head_coach_value();
    for _ in 0..8 {
        c.junior_reputation(0.0);
    }
    assert!(c.reputation > 3.0 + 0.5 * (shown - 3.0), "{} toward {shown}", c.reputation);
    assert!(c.reputation <= shown + 1e-9);
    assert_eq!(c.recent.len(), 6, "his staff's form counts in his buzz");
}

#[test]
fn a_head_coach_promotes_his_own_man_when_he_rates_him_ready() {
    let mut w = tiny();
    let p = fbs(&w)[0];
    let g = Group::DB;
    let c = (0..w.coaches.len())
        .find(|&c| w.coaches[c].junior_now().is_some_and(|j| j.employer == Employer::College(p)))
        .expect("an off-field man at an FBS program");
    w.coaches[c].junior =
        Some(Junior { employer: Employer::College(p), post: Post::Analyst, group: g, since: 2035, until: None });
    // His fellow off-field men in that room are far from ready.
    for k in 0..w.coaches.len() {
        if k != c && w.coaches[k].junior_now().is_some_and(|j| j.employer == Employer::College(p) && j.group == g) {
            w.coaches[k].ability.teaching[g as usize] = 1.0;
        }
    }
    w.coaches[c].ability.teaching[g as usize] = 20.0;
    assert_eq!(ready_at_home(&mut w, p, g, &HashSet::new()), Some(c));
    // A man well below the room's bar waits.
    w.coaches[c].ability.teaching[g as usize] = 3.0;
    assert_eq!(ready_at_home(&mut w, p, g, &HashSet::new()), None);
}

#[test]
fn his_staff_follows_a_head_coach_to_his_new_job() {
    let mut w = tiny();
    let list = fbs(&w);
    let (p, q) = (list[0], list[1]);
    let hc = w.programs[q].head_coach.unwrap();
    let old = w.programs[p].head_coach.unwrap();
    leave(&mut w, old, "fired");
    leave(&mut w, hc, "left for another job");
    hire(&mut w, hc, Employer::College(p), Role::HeadCoach);
    let before: Vec<(Role, Option<usize>)> =
        crate::world::staff::college_roles().skip(1).map(|r| (r, slot(&w.programs[q], r))).collect();
    let (mut moved, mut queue) = (HashSet::new(), Vec::new());
    let took = bring_his_staff(&mut w, hc, p, &mut moved, &mut queue);
    assert!(!took.is_empty(), "some of his men came");
    assert!(took.len() < before.len(), "not all of them");
    for r in &took {
        let f = before.iter().find(|x| x.0 == *r).unwrap().1.unwrap();
        assert_eq!(slot(&w.programs[p], *r), Some(f), "{r:?} is his old man");
        assert_eq!(slot(&w.programs[q], *r), None, "{r:?} is open at his old job");
        assert!(queue.iter().any(|o| o.employer == Employer::College(q) && o.role == *r));
    }
}

#[test]
fn coaches_let_go_from_big_staffs_wait_as_analysts() {
    let mut w = tiny();
    let list = fbs(&w);
    // Ten position coaches let go; a few analyst posts open.
    let mut gone = Vec::new();
    for &p in list.iter().take(10) {
        if let Some(c) = w.programs[p].position_coaches[Group::LB as usize] {
            leave(&mut w, c, "fired");
            gone.push(c);
        }
    }
    for &p in list.iter().skip(10).take(6) {
        if let Some(c) = (0..w.coaches.len()).find(|&c| {
            w.coaches[c].junior_now().is_some_and(|j| j.employer == Employer::College(p) && j.post == Post::Analyst)
        }) {
            w.coaches[c].junior = None;
            w.coaches[c].retired = true;
        }
    }
    analysts_from_the_street(&mut w);
    let landed: Vec<Junior> = gone.iter().filter_map(|&c| w.coaches[c].junior_now()).collect();
    assert!(!landed.is_empty(), "some took an analyst post");
    for j in &landed {
        assert_eq!(j.post, Post::Analyst);
        assert_eq!(j.group, Group::LB, "in the room he coached");
        let Employer::College(p) = j.employer else { panic!("a college post") };
        assert_eq!(w.programs[p].division, Division::FBS);
    }
}

#[test]
fn a_man_let_go_from_an_fbs_staff_holds_out_a_cycle_for_a_job_near_his_level() {
    use crate::world::staff::{Opening, weigh_offer};
    let mut w = tiny();
    let p = fbs(&w)[0];
    let c = w.programs[p].position_coaches[Group::DB as usize].expect("a DB coach");
    leave(&mut w, c, "let go by the new staff");
    let at = |w: &World, d: Division| (0..w.programs.len()).find(|&q| w.programs[q].division == d).unwrap();
    let offer =
        |q: usize| Opening { employer: Employer::College(q), role: Role::PositionCoach(Group::DB), prestige: 0.3 };
    let (d2, fcs) = (at(&w, Division::D2), at(&w, Division::FCS));
    assert!(weigh_offer(&mut w, c, offer(d2)).is_none(), "two levels down waits");
    assert!(weigh_offer(&mut w, c, offer(fcs)).is_some(), "one level down he takes");
    // From the next carousel he takes what comes.
    w.coaches[c].history.last_mut().unwrap().to -= 1;
    assert!(weigh_offer(&mut w, c, offer(d2)).is_some());
}
