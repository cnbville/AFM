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

/// Minor bugs 332 and 333: a look-back means the same on the draft board and
/// in a recruiting class (a random order holds about 3 in 10 in both), the
/// board's rule is still the out-on-a-limb distance, and an AI scout's record
/// sits between chance and a little past the real draft order (47%).
#[test]
fn look_backs_hold_at_real_rates() {
    use crate::world::pro::scouts::held_chance;
    use crate::world::scouting::board::limb;
    use crate::world::scouting::lookback::{BOARD, Graded, Verdict, verdict};
    let graded = |ours: u32, outcome: u32, size: u16| Graded {
        year: 2026,
        id: ours,
        name: String::new(),
        pos: crate::positions::Position::WR,
        ours,
        public: None,
        pick: None,
        team: None,
        lead: None,
        looks: 5,
        outcome: Some(outcome),
        numbers: None,
        size,
    };
    // The board keeps its rule exactly.
    for ours in 1..=300u32 {
        for out in [1u32, ours.saturating_sub(13).max(1), ours, ours + 11, ours + 12, 300] {
            let old = match limb(ours as usize - 1, Some(out.min(300) as usize - 1)) {
                1 => Verdict::TooLow,
                -1 => Verdict::TooHigh,
                _ => Verdict::Held,
            };
            assert_eq!(verdict(&graded(ours, out.min(300), BOARD)), old, "rank {ours}, outcome {out}");
        }
    }
    // A random order: the share that holds, board and class alike.
    let mut rng = Rng::new("lookback");
    let mut luck = |n: u16| {
        let (mut held, mut all) = (0u32, 0u32);
        for _ in 0..400 {
            let mut out: Vec<u32> = (1..=n as u32).collect();
            for k in (1..out.len()).rev() {
                out.swap(k, rng.int(0, k as i32) as usize);
            }
            for (k, &o) in out.iter().enumerate() {
                held += (verdict(&graded(k as u32 + 1, o, n)) == Verdict::Held) as u32;
                all += 1;
            }
        }
        held as f64 / all as f64
    };
    let (board, class) = (luck(300), luck(20));
    assert!((0.27..0.35).contains(&board), "a random board holds {board:.3}");
    assert!((0.24..0.34).contains(&class), "a random class of 20 holds {class:.3}");
    // An AI scout's chance: above chance at the worst eye, rising with it,
    // the best just past the real draft order.
    for j in 1..20u8 {
        assert!(held_chance(j + 1) > held_chance(j));
    }
    assert!(held_chance(1) > 0.33 && held_chance(10) > 0.42 && held_chance(10) < 0.46 && held_chance(20) < 0.53);
}
