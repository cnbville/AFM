//! The NFL head coach's hot seat (`staff::midseason`): an owner fires in
//! season only from a public seat at least a week old, a first season is
//! counted as a first season, and a bad start says little about a man the
//! owner just hired.

use crate::calendar::Date;
use crate::world::coach::{Employer, Job, Role};
use crate::world::folk::staffing::Seat;
use crate::world::news::NewsKind;
use crate::world::nfl_year::NflSeason;
use crate::world::staff::nfl_in_season;
use crate::world::{World, WorldOptions};

fn tiny() -> World {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    w.start_recording();
    w
}

/// Club 0's season so far, every other club at .500, and its head coach
/// in his `nth` season under an owner of middling patience.
fn stage(w: &mut World, wins: u32, losses: u32, nth: i32) -> usize {
    let n = w.nfl.len();
    let year = w.year;
    let games = wins + losses;
    let mut s = NflSeason { year, started: true, ..Default::default() };
    s.active = vec![true; n];
    s.wins = vec![games / 2; n];
    s.losses = vec![games - games / 2; n];
    s.ties = vec![0; n];
    s.expect = vec![games as f64 / 2.0; n];
    s.wins[0] = wins;
    s.losses[0] = losses;
    w.nfl_season = Some(s);
    w.nfl[0].owner = 10.0;
    w.nfl[0].win_share = vec![0.5];
    let hc = w.nfl[0].head_coach.expect("a staffed club");
    let first = year - nth + 1;
    w.coaches[hc].job = Some(Job { employer: Employer::Nfl(0), role: Role::HeadCoach, since: first - 1, first });
    hc
}

fn fired(w: &World, hc: usize) -> bool {
    w.coaches[hc].job.is_none()
}

fn seat_news(w: &World) -> usize {
    w.news.iter().filter(|n| n.kind == NewsKind::CoachSeat).count()
}

#[test]
fn a_season_in_the_job_counts_from_the_first_he_coaches() {
    let nfl = Employer::Nfl(0);
    // Hired in January 2026 for the 2026 season: the label is 2025.
    let j = Job { employer: nfl, role: Role::HeadCoach, since: 2025, first: 2026 };
    assert_eq!(j.season_no(2026), 1);
    assert_eq!(j.season_no(2028), 3);
    // An interim named during 2026 coaches 2026.
    let j = Job { employer: nfl, role: Role::HeadCoach, since: 2026, first: 2026 };
    assert_eq!(j.season_no(2026), 1);
    assert_eq!(j.season_no(2027), 2);
    // An older save has no first season: the label's next.
    let j = Job { employer: nfl, role: Role::HeadCoach, since: 2025, first: 0 };
    assert_eq!(j.season_no(2026), 1);
}

#[test]
fn a_first_year_head_coach_at_1_3_is_not_on_the_hot_seat() {
    let mut w = tiny();
    let hc = stage(&mut w, 1, 3, 1);
    // Every Monday draws again: none of two hundred fires him or warns him.
    let d0 = w.today;
    for k in 0..200 {
        w.today = d0.add(7 * k);
        nfl_in_season(&mut w);
        assert!(!fired(&w, hc), "fired at 1-3 in his first season");
    }
    assert!(w.folk.staff.seats.is_empty());
    assert_eq!(seat_news(&w), 0);
    // The same start in his fourth season is public the first Monday.
    let hc = stage(&mut w, 1, 3, 4);
    nfl_in_season(&mut w);
    assert!(!fired(&w, hc), "the owner can't act the day the seat goes hot");
    assert!(w.folk.staff.seat(0, hc).is_some());
    assert_eq!(seat_news(&w), 1);
}

#[test]
fn a_collapse_deep_into_a_first_season_does_reach_the_seat() {
    let mut w = tiny();
    let hc = stage(&mut w, 1, 10, 1);
    nfl_in_season(&mut w);
    assert!(w.folk.staff.seat(0, hc).is_some(), "1-10 is a hot seat even in a first season");
}

#[test]
fn nobody_is_fired_in_season_without_a_week_on_a_public_seat() {
    let mut w = tiny();
    let hc = stage(&mut w, 0, 8, 4);
    let d0 = w.today;
    let mut hot: Option<Date> = None;
    let mut out = None;
    for k in 0..400 {
        w.today = d0.add(k);
        nfl_in_season(&mut w);
        if hot.is_none() && w.folk.staff.seat(0, hc).is_some() {
            hot = Some(w.today);
        }
        if fired(&w, hc) {
            out = Some(w.today);
            break;
        }
    }
    let hot = hot.expect("0-8 in his fourth season is a hot seat");
    let out = out.expect("an 0-8 team's owner acts sooner or later");
    assert!(hot.days_to(out) >= 7, "fired {} days after the seat went hot", hot.days_to(out));
    assert!(w.folk.staff.seats.is_empty(), "the seat ends with the job");
    let line = w.news.iter().rev().find(|n| n.kind == NewsKind::CoachFired).expect("the firing is news");
    assert!(line.text.contains("into his 4th season"), "{}", line.text);
}

#[test]
fn a_seat_cools_when_the_team_steadies() {
    let mut w = tiny();
    let hc = stage(&mut w, 6, 4, 4);
    let today = w.today;
    w.folk.staff.seats.push(Seat { team: 0, coach: hc as u32, since: today.add(-200), carried: true });
    nfl_in_season(&mut w);
    assert!(!fired(&w, hc));
    assert!(w.folk.staff.seats.is_empty(), "6-4 ends the talk");
}
