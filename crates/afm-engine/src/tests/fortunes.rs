//! Program fortunes lane P1 (`docs/design/program-fortunes.md` items 1-4,
//! Cain 2026-11-24): what a recruit reads moves with results and coaches.

use crate::world::coach::{Employer, Role, Stint};
use crate::world::data::Division;
use crate::world::fortunes::{self, COACH_PIPELINE};
use crate::world::program::{self, ProgramSeason};
use crate::world::{World, WorldOptions};

fn world() -> World {
    World::empty(WorldOptions { scale: 0.02, history_years: 0, start_year: 2030, ..Default::default() })
}

/// Two FBS programs that both have a head coach.
fn two_fbs(w: &World) -> (usize, usize) {
    let mut fbs = (0..w.programs.len())
        .filter(|&j| w.programs[j].division == Division::FBS && w.programs[j].head_coach.is_some());
    (fbs.next().unwrap(), fbs.next().unwrap())
}

fn season(year: i32, wins: u32, losses: u32, title: bool) -> ProgramSeason {
    ProgramSeason {
        year,
        wins,
        losses,
        conference_title: title,
        national_title: title,
        coach: None,
        strength: 0.0,
        totals: Default::default(),
        conference_won: None,
        conference: None,
        division: Some(Division::FBS),
        points: [0, 0],
    }
}

fn head_stint(x: usize, from: i32, to: i32) -> Stint {
    Stint {
        employer: Employer::College(x),
        role: Role::HeadCoach,
        from,
        to,
        wins: 0,
        losses: 0,
        titles: 0,
        ended: "left",
    }
}

/// Item 1: the name is half the all-time part and half the results of the
/// last 20 seasons, so a big name that stops winning fades and a riser
/// builds one without a title.
#[test]
fn the_name_reads_the_results_a_recruit_has_lived_through() {
    let w = world();
    let (j, _) = two_fbs(&w);
    let mut giant = w.programs[j].clone();
    giant.legacy = 0.9;
    giant.seasons.clear();
    // No recorded seasons: the lived half reads the all-time part, so
    // nothing jumps when recorded history begins.
    assert!((program::lived(&giant, 2026, None) - 0.9).abs() < 1e-12);
    // Twenty 3-9 seasons: the lived half is the losing seasons' score.
    for y in 2006..=2025 {
        giant.seasons.push(season(y, 3, 9, false));
    }
    let losing = program::season_score(Division::FBS, 0.25, false, false);
    assert!((program::lived(&giant, 2025, None) - losing).abs() < 1e-9);
    let name = (1.0 - program::LIVED_SHARE) * giant.legacy + program::LIVED_SHARE * losing;
    assert!(name < 0.66, "a giant twenty years without a winning season still reads {name:.2}");

    // A small name with five straight 11-2 seasons and no title.
    let mut riser = w.programs[j].clone();
    riser.legacy = 0.4;
    riser.seasons = (2021..=2025).map(|y| season(y, 11, 2, false)).collect();
    let lived = program::lived(&riser, 2025, None);
    assert!(lived > 0.6, "five 11-2 seasons read {lived:.2}");
    // Recent seasons weigh more: the same run 15 years ago counts far less.
    let mut old = riser.clone();
    old.seasons = (2006..=2010).map(|y| season(y, 11, 2, false)).collect();
    assert!(program::lived(&old, 2025, None) < 0.5 * (lived - 0.4) + 0.4);
    // This season's score when its book isn't closed yet.
    assert!(program::lived(&riser, 2026, Some(1.4)) > lived);
}

/// Item 2: a head coach's own picks count in his school's pro pipeline
/// until his seasons there have fed four drafts.
#[test]
fn a_coachs_own_picks_count_in_his_new_schools_pipeline() {
    let mut w = world();
    let (bama, lsu) = two_fbs(&w);
    let c = w.programs[bama].head_coach.unwrap();
    w.programs[lsu].draft_picks = (2001..=2005).flat_map(|y| [(y, 1), (y, 1), (y, 1)]).collect();
    w.programs[bama].draft_picks = (2004..=2011).map(|y| (y, 7)).collect();
    let mut job = w.coaches[c].job.unwrap();
    job.employer = Employer::College(bama);
    job.role = Role::HeadCoach;
    job.since = 2006;
    job.first = 2007;
    w.coaches[c].job = Some(job);
    w.coaches[c].history.clear();
    // A first-time head coach: the school's own picks.
    assert!((fortunes::pipeline(&w, bama, 2007) - 4.0).abs() < 1e-9);
    // Saban from LSU: each of the last four drafts reads half his LSU
    // seasons' picks (nine points a draft).
    w.coaches[c].history.push(head_stint(lsu, 1999, 2004));
    let with = fortunes::pipeline(&w, bama, 2007);
    let want = 4.0 * ((1.0 - COACH_PIPELINE) * 1.0 + COACH_PIPELINE * 9.0);
    assert!((with - want).abs() < 1e-9, "{with} against {want}");
    // Two seasons in, his own two drafts are the school's alone.
    let two_in = fortunes::pipeline(&w, bama, 2009);
    assert!((two_in - (2.0 + 2.0 * ((1.0 - COACH_PIPELINE) + COACH_PIPELINE * 9.0))).abs() < 1e-9);
    // Four seasons in, every draft is his own here.
    assert!((fortunes::pipeline(&w, bama, 2011) - 4.0).abs() < 1e-9);
}

/// Item 2: title chances read his record above his rosters; a man who was
/// never a college head coach brings none.
#[test]
fn title_chances_read_the_head_coachs_record_above_his_rosters() {
    let mut w = world();
    let (here, before) = two_fbs(&w);
    let c = w.programs[here].head_coach.unwrap();
    let mut job = w.coaches[c].job.unwrap();
    job.employer = Employer::College(here);
    job.role = Role::HeadCoach;
    job.since = 2029;
    job.first = 2030;
    w.coaches[c].job = Some(job);
    w.coaches[c].history.clear();
    w.coaches[c].recent = vec![0.0, 0.0, 0.2, 0.25, 0.15];
    // A coordinator's results are his head coach's.
    assert_eq!(fortunes::head_record(&w, c, 2029), 0.0);
    // Three seasons as a head coach elsewhere, hired here after them.
    w.coaches[c].history.push(head_stint(before, 2026, 2029));
    assert!((fortunes::head_record(&w, c, 2029) - 0.2).abs() < 1e-9);
    // One head-coaching season reads that season only.
    w.coaches[c].history[0] = head_stint(before, 2028, 2029);
    assert!((fortunes::head_record(&w, c, 2029) - 0.15).abs() < 1e-9);
}

/// Item 4: a transfer's bond with a staff grows at its coaches' recruiting
/// rate, and a coach who recruited him out of high school and lost him
/// starts ahead of a stranger.
#[test]
fn a_coach_who_recruited_him_out_of_high_school_starts_ahead_in_the_portal() {
    let mut w = world();
    w.year = 2030;
    let (j, x) = two_fbs(&w);
    let c = w.programs[j].head_coach.unwrap();
    // Nobody on its staff has a past at the other schools.
    let staff: Vec<usize> = w.programs[j].position_coaches.iter().flatten().copied().chain([c]).collect();
    for k in staff {
        w.coaches[k].history.clear();
    }
    let elsewhere =
        (0..w.programs.len()).find(|&y| y != j && y != x && w.programs[y].division == Division::FBS).unwrap();
    let i = 0;
    w.people[i].class_of = 2028;
    w.people[i].cal.offers = vec![x];
    w.people[i].cal.early_offers.clear();
    w.people[i].cal.signed = None;
    let (calls, old) = fortunes::transfer_bond(&w, i, j);
    assert!(calls > 0.0);
    assert_eq!(old, 0.0);
    // He was head coach at a school that offered him in his recruiting fall.
    w.coaches[c].history.push(head_stint(x, 2025, 2028));
    let (_, old) = fortunes::transfer_bond(&w, i, j);
    assert!(old > 0.0);
    // A school that didn't offer him built no bond.
    w.people[i].cal.offers = vec![elsewhere];
    let (_, none) = fortunes::transfer_bond(&w, i, j);
    assert_eq!(none, 0.0);
    // A better recruiter's calls build more.
    w.coaches[c].ability.recruiting = 18.0;
    w.coaches[c].ability.relationships = 18.0;
    let (better, _) = fortunes::transfer_bond(&w, i, j);
    assert!(better > calls);
}

/// Lane P3 (question 11): a school's standing for a coach's bond is its
/// level and its name, so a Group of 5 school with the biggest name in its
/// league stands below a Power 4 school with a middling one, and a name
/// that grows with wins closes the gap.
#[test]
fn a_school_stands_by_its_level_and_its_name() {
    use crate::world::recruiting::bond_standing;
    let w = world();
    let (j, _) = two_fbs(&w);
    let year = 2030;
    let mut power = w.programs[j].clone();
    power.conference = "SEC".into();
    power.real_name = "Test State".into();
    let mut group_five = power.clone();
    group_five.conference = "Sun Belt".into();
    let mut fcs = power.clone();
    fcs.division = Division::FCS;
    power.prestige_rel = 0.5;
    group_five.prestige_rel = 0.6;
    fcs.prestige_rel = 1.0;
    assert!(power.power_conference(year) && !group_five.power_conference(year));
    // The best Group of 5 name stands below a middling Power 4 one, and
    // the best FCS name below the Group of 5's.
    assert!(bond_standing(&group_five, year) < bond_standing(&power, year));
    assert!(bond_standing(&fcs, year) < bond_standing(&group_five, year));
    // The national names stand alike, and the gap a great Group of 5
    // recruiter faces against one takes nearly all of his bond above the
    // typical coach's.
    power.prestige_rel = 0.9;
    let mut other = power.clone();
    other.prestige_rel = 1.0;
    assert_eq!(bond_standing(&power, year), bond_standing(&other, year));
    let before = bond_standing(&group_five, year);
    assert!(fortunes::held_bond(bond_standing(&power, year) - before) < 0.25);
    // A bigger name moves it up, and the gap closes with it.
    group_five.prestige_rel = 0.85;
    assert!(bond_standing(&group_five, year) > before);
    assert!(fortunes::held_bond(bond_standing(&power, year) - bond_standing(&group_five, year)) > 0.4);
}
