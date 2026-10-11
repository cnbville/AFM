use super::mean;
use crate::player::{Level, Player, ProspectOptions, generate_prospect};
use crate::rng::Rng;
use crate::scouting::{
    Confidence, HINT_MIN_ACCURACY, PotentialTier, ScoutOptions, potential_outlook, scout_player, to_stars,
};

fn players() -> (Vec<Player>, Rng) {
    let mut rng = Rng::new("scouting");
    let ps = (0..2000)
        .map(|_| {
            generate_prospect(&mut rng, ProspectOptions { age: Some(17), talent_z: Some(2.5), ..Default::default() })
        })
        .collect();
    (ps, rng)
}

#[test]
fn gives_better_estimates_from_better_scouts() {
    let (players, mut rng) = players();
    let mut error = |accuracy: f64| {
        mean(
            &players
                .iter()
                .map(|p| {
                    (scout_player(p, ScoutOptions { accuracy, level: Level::HS }, &mut rng).potential - p.potential())
                        .abs()
                })
                .collect::<Vec<_>>(),
        )
    };
    let good = error(18.0);
    let bad = error(4.0);
    assert!(good < bad * 0.75);
}

#[test]
fn is_less_sure_about_younger_players() {
    let young = generate_prospect(&mut Rng::new(1), ProspectOptions { age: Some(15), ..Default::default() });
    let mut veteran = generate_prospect(&mut Rng::new(1), ProspectOptions { age: Some(15), ..Default::default() });
    veteran.age = 28;
    let width = |p: &Player| {
        let r = scout_player(p, ScoutOptions { accuracy: 12.0, level: Level::FBS }, &mut Rng::new(2));
        r.potential_high - r.potential_low
    };
    assert!(width(&young) > width(&veteran));
}

#[test]
fn reports_a_range_that_usually_contains_the_truth() {
    let (players, mut rng) = players();
    let hits = players
        .iter()
        .filter(|p| {
            let r = scout_player(p, ScoutOptions { accuracy: 12.0, level: Level::HS }, &mut rng);
            r.potential_low <= p.potential() && p.potential() <= r.potential_high
        })
        .count();
    assert!(hits as f64 / players.len() as f64 > 0.6);
    assert!(players[0].overall() > 0.0);
}

#[test]
fn maps_ratings_to_half_stars() {
    assert_eq!(to_stars(90.0), 5.0);
    assert_eq!(to_stars(70.0), 3.0);
    assert_eq!(to_stars(50.0), 1.0);
    assert_eq!(to_stars(20.0), 0.5);
}

#[test]
fn shows_only_a_vague_outlook_sharper_with_better_scouts() {
    let (players, mut rng) = players();
    // The tier a perfect read would give.
    let truth = |p: &Player| {
        let r = p.potential();
        if r >= 92.0 {
            PotentialTier::Elite
        } else if r >= 85.0 {
            PotentialTier::ProBowl
        } else if r >= 75.0 {
            PotentialTier::NflStarter
        } else if r >= 65.0 {
            PotentialTier::NflRoster
        } else if r >= 52.0 {
            PotentialTier::College
        } else {
            PotentialTier::HighSchool
        }
    };
    let mut read = |accuracy: f64| {
        let outlooks: Vec<_> = players
            .iter()
            .map(|p| (truth(p), potential_outlook(p, ScoutOptions { accuracy, level: Level::HS }, &mut rng)))
            .collect();
        let right = outlooks.iter().filter(|(t, o)| *t == o.tier).count() as f64 / outlooks.len() as f64;
        let sure = outlooks.iter().filter(|(_, o)| o.confidence != Confidence::Unsure).count();
        let hints = outlooks.iter().map(|(_, o)| o.hints.len()).sum::<usize>();
        (right, sure, hints)
    };
    let (good_right, good_sure, good_hints) = read(19.0);
    let (bad_right, bad_sure, bad_hints) = read(5.0);
    assert!(good_right > bad_right);
    assert!(good_sure > bad_sure);
    assert!(good_hints > 0);
    assert_eq!(bad_hints, 0, "no hints below accuracy {HINT_MIN_ACCURACY}");
}
