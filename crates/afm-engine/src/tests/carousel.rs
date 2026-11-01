//! The coaching carousel's minor bugs before the world rebuild: a coach's
//! jobs close on the season he last coached (128).

use crate::world::coach::{Employer, Role};
use crate::world::staff::{hire, leave};
use crate::world::{World, WorldOptions};

fn tiny() -> World {
    World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() })
}

/// A head coach of a program (the first, or the first other than `not`).
fn head_of(w: &World, not: Option<usize>) -> (usize, usize) {
    (0..w.programs.len())
        .filter(|&p| Some(p) != not)
        .find_map(|p| w.programs[p].head_coach.map(|c| (p, c)))
        .expect("a staffed program")
}

/// 128: a move between the Super Bowl and fall camp closes the old job with
/// the season he last coached, the year his new job begins, so the two never
/// overlap by a season; a high school head coach moving to a college staff
/// the same spring does the same. Once fall camp opens, the season under way
/// is his.
#[test]
fn a_spring_move_closes_the_old_job_with_the_season_he_last_coached() {
    let mut w = tiny();
    let year = w.year;
    w.today = w.dates.fall_camp.add(-60);
    let (p, c) = head_of(&w, None);
    let (q, d) = head_of(&w, Some(p));
    leave(&mut w, d, "fired");
    leave(&mut w, c, "left for another job");
    hire(&mut w, c, Employer::College(q), Role::HeadCoach);
    let old = w.coaches[c].history.last().unwrap().clone();
    assert_eq!(old.employer, Employer::College(p));
    assert_eq!(old.to, year - 1, "his last season was last fall's");
    assert_eq!(w.coaches[c].job.unwrap().since, old.to, "the new job begins the year the old one ends");

    let h = (0..w.high_schools.len()).find(|&h| w.high_schools[h].coach.is_some()).unwrap();
    let k = w.high_schools[h].coach.unwrap();
    leave(&mut w, k, "left for another job");
    hire(&mut w, k, Employer::College(p), Role::HeadCoach);
    let hs = w.coaches[k].history.last().unwrap().clone();
    assert_eq!(hs.employer, Employer::HighSchool(h));
    assert_eq!(hs.to, w.coaches[k].job.unwrap().since);

    // In the season, the year is the season's.
    w.today = w.dates.fall_camp.add(30);
    leave(&mut w, k, "fired during the season");
    assert_eq!(w.coaches[k].history.last().unwrap().to, year);
}
