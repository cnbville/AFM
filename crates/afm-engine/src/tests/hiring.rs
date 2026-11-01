//! Lane 18, finding and hiring coaches: how fast a coach grows by his job
//! (C59, C61), what a promotion is worth to him (C29, C44), how wide a read
//! is for the evidence behind it (C40, C41), and the junior posts as real
//! coaches (C36).

use crate::rng::Rng;
use crate::world::coach::{CoachTier, Employer, GROUP_COUNT, Group, Job, Junior, Post, Role, generate_coach};
use crate::world::ethnicity::Ethnicity;
use crate::world::folk::coach_look::width;
use crate::world::folk::hiring::promotion_worth;

fn coach(up: Group, role: Option<Role>) -> crate::world::coach::Coach {
    let mut rng = Rng::new(7);
    let mut c = generate_coach(0, "Test Coach".into(), Ethnicity::White, CoachTier::Fbs, 34, &mut rng);
    c.came_up = up;
    c.job = role.map(|r| Job { employer: Employer::College(0), role: r, since: 2020, first: 2021 });
    c
}

#[test]
fn a_position_coach_grows_fastest_in_his_own_room() {
    let c = coach(Group::DL, Some(Role::PositionCoach(Group::DL)));
    let p = c.pace();
    assert_eq!(p[Group::DL as usize], 1.0);
    for g in Group::ALL.iter().filter(|g| **g != Group::DL) {
        assert!(p[*g as usize] < 0.5, "{g:?}: {}", p[*g as usize]);
    }
    // Scheme and game management at half pace; recruiting full at a college.
    assert_eq!(p[GROUP_COUNT], 0.5);
    assert_eq!(p[GROUP_COUNT + 1], 0.5);
    assert_eq!(p[GROUP_COUNT + 4], 1.0);
}

#[test]
fn a_coordinator_grows_across_his_side_and_most_in_the_group_he_coaches() {
    let c = coach(Group::DB, Some(Role::DefensiveCoordinator));
    let p = c.pace();
    assert_eq!(p[Group::DB as usize], 1.5);
    for g in [Group::DL, Group::LB] {
        let v = p[g as usize];
        assert!(v > 0.55 && v < 1.45, "{g:?}: {v}");
    }
    assert!(p[Group::QB as usize] < 0.5);
    assert_eq!(p[GROUP_COUNT], 1.0);
}

#[test]
fn a_head_coach_grows_at_two_thirds_everywhere() {
    let c = coach(Group::QB, Some(Role::HeadCoach));
    let p = c.pace();
    assert!((p[Group::QB as usize] - 2.0 / 3.0).abs() < 1e-9);
    // Groups he didn't come up in: two thirds times his adaptability (0.6-1.4).
    let db = p[Group::DB as usize];
    assert!((0.4 - 1e-9..=0.94).contains(&db), "{db}");
}

#[test]
fn an_analyst_grows_his_scheme_at_full_pace() {
    let mut c = coach(Group::OL, None);
    c.junior = Some(Junior {
        employer: Employer::College(0),
        post: Post::Analyst,
        group: Group::OL,
        since: 2025,
        until: None,
    });
    let p = c.pace();
    assert_eq!(p[GROUP_COUNT], 1.0);
    assert_eq!(p[Group::OL as usize], 1.0);
}

#[test]
fn a_promotion_is_worth_more_to_the_young_and_the_ambitious() {
    assert!((promotion_worth(10.0, 50) - 0.5 * (2.0 - 5.0 / 15.0)).abs() < 1e-9);
    assert_eq!(promotion_worth(10.0, 40), 1.0);
    assert_eq!(promotion_worth(10.0, 62), 0.5);
    assert!(promotion_worth(20.0, 62) > promotion_worth(1.0, 62));
    assert!((promotion_worth(1.0, 62) - 0.23).abs() < 1e-9);
    assert!((promotion_worth(20.0, 62) - 0.8).abs() < 1e-9);
}

#[test]
fn a_strangers_read_is_the_whole_scale_and_ten_hours_make_it_two_wide() {
    assert_eq!(width(0.0), 20.0);
    assert!((width(10.0) - 2.0).abs() < 1e-9);
    assert!(width(3.0) < width(1.0));
}
