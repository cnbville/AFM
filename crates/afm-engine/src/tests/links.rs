//! Dynamic Links (`docs/design/dynamic-links.md`, "What the build must
//! prove"): how common each link is by level, the stacking rules, and the
//! smooth start with no cliff.

use crate::game::roster::OffScheme;
use crate::game::synthetic::synthetic_team;
use crate::links::{FULL, Link, Links, gift, links_of, prevalence, stacked};
use crate::player::Level;

fn level_share(level: Level) -> f64 {
    let mut all = Vec::new();
    for k in 0..24u32 {
        let t = synthetic_team("T", level, 0.0, OffScheme::WestCoast, 900 + k);
        all.extend(t.players.iter().map(|p| p.links));
    }
    let prev = prevalence(&all);
    prev.iter().map(|x| x.0).sum::<f64>() / prev.len() as f64
}

#[test]
fn links_are_rare_in_high_school_and_common_at_the_top_of_the_nfl() {
    let (hs, fbs, nfl) = (level_share(Level::HS), level_share(Level::FBS), level_share(Level::NFL));
    println!("share of players with each link (mean of 12): HS {hs:.3}, FBS {fbs:.3}, NFL {nfl:.3}");
    assert!(hs < fbs && fbs < nfl, "links rise with the level: {hs:.3} {fbs:.3} {nfl:.3}");
    assert!(nfl < 0.35, "even in the NFL a link is a gift, not the norm: {nfl:.3}");
}

#[test]
fn links_in_one_situation_stack_at_half_and_different_situations_in_full() {
    let mut l = Links::default();
    l.set(Link::Anchor, 1.0);
    l.set(Link::BigMover, 0.6);
    l.set(Link::Bend, 1.0);
    // The same situation: the stronger in full, the next at half.
    assert!((l.edge(&[Link::Anchor, Link::BigMover]) - FULL * 1.3).abs() < 1e-6);
    // Order doesn't matter.
    assert!((l.edge(&[Link::BigMover, Link::Anchor]) - FULL * 1.3).abs() < 1e-6);
    // A different situation counts in full on its own.
    assert!((l.edge(&[Link::Bend]) - FULL).abs() < 1e-6);
    // A link he lacks adds nothing.
    assert_eq!(l.edge(&[Link::Lungs]), 0.0);
    let mut s = [0.2, 1.0, 0.5];
    assert!((stacked(&mut s) - (1.0 + 0.25 + 0.05)).abs() < 1e-9);
    // A staff values a complete athlete more than one trick, with each
    // further link worth less.
    let mut one = Links::default();
    one.set(Link::Burner, 1.0);
    assert!(gift(&l) > gift(&one) && gift(&l) < 3.0);
}

#[test]
fn a_link_is_measured_on_his_body_and_never_on_a_specialist() {
    let t = synthetic_team("T", Level::NFL, 3.0, OffScheme::WestCoast, 77);
    for p in &t.players {
        if p.position.is_specialist() {
            assert!(!p.links.any(), "a kicker, punter or snapper carries no link");
        }
        for (k, &v) in p.links.0.iter().enumerate() {
            assert!((0.0..=1.0).contains(&v), "link {k} out of range: {v}");
        }
    }
    // Measured again from the same player, the same links (no randomness).
    let mut rng = crate::rng::Rng::new(5);
    let pl = crate::player::generate_prospect(&mut rng, Default::default());
    assert_eq!(links_of(&pl), links_of(&pl));
}
