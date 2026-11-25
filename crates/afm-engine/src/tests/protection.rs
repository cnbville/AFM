//! Pass protection (realism audit item 3, the pass rush lane): the linemen
//! take the men on the line in order across the formation, so the tackle
//! has the end however wide he lines up, and a lineman left over or a back
//! takes a blitzer. The four-man rush lane: against a man coming round the
//! edge the tackle sets back and lets him come.

use crate::game::chart::Rusher;
use crate::game::geom::v2;
use crate::game::scrimmage::{DSlot, pair_in_order, set_spot};
use crate::game::stats::PlayType;
use crate::game::synthetic::synthetic_league;
use crate::game::{GameSetup, play_game};
use crate::player::Level;

/// Five linemen two yards apart, the center on the ball.
const LINE: [f64; 5] = [-4.0, -2.0, 0.0, 2.0, 4.0];

#[test]
fn the_linemen_take_the_line_in_order() {
    // An over front: a wide end, a 3-technique outside the guard, a nose
    // shaded on the center and the other end. The nearest man not yet taken,
    // from one sideline to the other and inside 3.2 yards, gave the tackle
    // the 3-technique and the guard the nose, and the wide end was nobody's.
    let front = [-5.5, -2.6, 0.8, 5.0];
    assert_eq!(pair_in_order(&LINE, &front, None, 0.0), [Some(0), Some(1), Some(2), None, Some(3)]);
    // An end split out as far as a slot receiver is still the tackle's.
    let wide = [-9.0, -2.6, 0.8, 5.0];
    assert_eq!(pair_in_order(&LINE, &wide, None, 0.0)[0], Some(0));
    // Nobody crosses a teammate, whatever the front.
    for front in [[-6.0, -1.0, 1.0, 6.0], [-3.0, -1.0, 3.0, 7.0], [-7.0, -5.0, 0.0, 2.5], [0.5, 2.5, 4.5, 6.5]] {
        let got = pair_in_order(&LINE, &front, None, 0.0);
        let men: Vec<usize> = got.iter().flatten().copied().collect();
        assert_eq!(men, [0, 1, 2, 3], "{front:?}: {got:?}");
    }
}

#[test]
fn a_line_a_man_short_leaves_the_widest_rusher() {
    // Six on the line against five: every lineman has a man, and the one
    // left is the man farthest from the ball, the longest way from the
    // passer.
    let six = [-6.0, -3.0, -1.0, 1.0, 3.0, 6.5];
    assert_eq!(pair_in_order(&LINE, &six, None, 0.0), [Some(0), Some(1), Some(2), Some(3), Some(4)]);
    let six = [-6.5, -3.0, -1.0, 1.0, 3.0, 6.0];
    assert_eq!(pair_in_order(&LINE, &six, None, 0.0), [Some(1), Some(2), Some(3), Some(4), Some(5)]);
    // With the ball on the left hash of his rushers, the far man is the
    // one on the right.
    assert_eq!(pair_in_order(&LINE, &six, None, -3.0), [Some(0), Some(1), Some(2), Some(3), Some(4)]);
}

#[test]
fn a_spare_lineman_takes_a_blitzer_inside_his_reach() {
    // The center, with no man on the line, has the linebacker through his
    // gap, and never one coming off the edge.
    assert_eq!(pair_in_order(&[0.0], &[0.5, 9.0], Some(3.2), 0.0), [Some(0)]);
    assert_eq!(pair_in_order(&[0.0], &[5.0], Some(3.2), 0.0), [None]);
    // Two spare linemen and two blitzers in reach: each takes the one on
    // his own side.
    assert_eq!(pair_in_order(&[0.0, 2.0], &[-1.0, 3.0], Some(3.2), 0.0), [Some(0), Some(1)]);
    // No rushers, or no blockers.
    assert_eq!(pair_in_order(&LINE, &[], None, 0.0), [None; 5]);
    assert!(pair_in_order(&[], &[1.0], None, 0.0).is_empty());
}

/// On a straight dropback a blocker has each end at the snap. The old
/// hand-out left the edge to nobody on about one of his rushes in seven.
#[test]
fn a_blocker_has_the_end_at_the_snap() {
    let t = synthetic_league(Level::NFL, 4, 71);
    let (mut ends, mut had) = (0u32, 0u32);
    for k in 0..4u64 {
        let (h, a) = if k % 2 == 0 { (0, 1) } else { (2, 3) };
        let mut s = GameSetup::new(&t[h], &t[a], Level::NFL, 900 + k);
        s.keep = true;
        let r = play_game(&s);
        for p in &r.stats.plays {
            // A screen lets the rush come, a boot leaves the end for the
            // quarterback, and on a run-pass option the line run blocks.
            let dropback = matches!(p.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble);
            if !dropback
                || ["screen", "boot", "naked", " with "].iter().any(|x| p.call.contains(x))
                || p.roles[1].len() != p.on_field[1].len()
            {
                continue;
            }
            let Some(c) = p.line.as_deref() else { continue };
            for (i, id) in p.on_field[1].iter().enumerate() {
                if p.roles[1][i] != DSlot::LE as u8 && p.roles[1][i] != DSlot::RE as u8 {
                    continue;
                }
                if let Some(q) = c.rush.iter().find(|q| q.id == *id) {
                    ends += 1;
                    had += (q.flags & Rusher::PICKED != 0) as u32;
                }
            }
        }
    }
    assert!(ends >= 300, "only {ends} end rushes in four games");
    assert!(had as f64 >= 0.97 * ends as f64, "a blocker had {had} of {ends} ends at the snap");
}

#[test]
fn the_tackle_sets_back_on_the_line_to_the_passer() {
    // A seven-step drop from the line at 30: the passer sets up at 23, and
    // the tackle kicks back 0.3 of the way, to 27.9, onto the line from the
    // end to the passer.
    let launch = v2(23.0, 0.0);
    let end = v2(30.6, 8.0);
    let spot = set_spot(30.0, launch, end).expect("a set point");
    assert!((spot.x - 27.9).abs() < 1e-9, "{spot:?}");
    let (a, b) = (spot - end, launch - end);
    assert!((a.x * b.y - a.y * b.x).abs() < 1e-9, "{spot:?} is off the line to the passer");
    assert!(spot.y > launch.y && spot.y < end.y, "{spot:?}");
    // A quick drop of 4 yards sets shallower: 1.2 yards back.
    let quick = set_spot(30.0, v2(26.0, 0.0), end).expect("a set point");
    assert!((quick.x - 28.8).abs() < 1e-9, "{quick:?}");
    // Once the end is within half a yard of the set depth the tackle takes
    // him where he is.
    assert!(set_spot(30.0, launch, v2(28.3, 6.0)).is_none());
    assert!(set_spot(30.0, launch, v2(28.5, 6.0)).is_some());
}

/// On a straight dropback the end meets his blocker behind the line, well
/// after the men inside meet theirs. The tackle used to mirror him from the
/// snap at the line, as he mirrors a man over him (`set_spot`). In these
/// four games the ends meet theirs 1.02 s after the snap and the inside men
/// 0.22 s; with the set turned off, 0.61 s.
#[test]
fn the_end_meets_his_blocker_behind_the_line() {
    let t = synthetic_league(Level::NFL, 4, 71);
    let (mut ends, mut inside) = (Vec::new(), Vec::new());
    for k in 0..4u64 {
        let (h, a) = if k % 2 == 0 { (0, 1) } else { (2, 3) };
        let mut s = GameSetup::new(&t[h], &t[a], Level::NFL, 900 + k);
        s.keep = true;
        let r = play_game(&s);
        for p in &r.stats.plays {
            let dropback = matches!(p.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble);
            if !dropback
                || ["screen", "boot", "naked", " with "].iter().any(|x| p.call.contains(x))
                || p.roles[1].len() != p.on_field[1].len()
            {
                continue;
            }
            for (i, id) in p.on_field[1].iter().enumerate() {
                let Some(&(_, at)) = p.contact.iter().find(|c| c.0 == *id) else { continue };
                let role = p.roles[1][i];
                if role == DSlot::LE as u8 || role == DSlot::RE as u8 {
                    ends.push(at as f64);
                } else if [DSlot::DTL, DSlot::DTR, DSlot::NT].iter().any(|s| role == *s as u8) {
                    inside.push(at as f64);
                }
            }
        }
    }
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
    let (e, n) = (mean(&ends), mean(&inside));
    assert!(ends.len() >= 300 && inside.len() >= 300, "{} ends and {} inside men blocked", ends.len(), inside.len());
    assert!(e >= 0.8, "the end met his blocker {e:.2} s after the snap");
    assert!(e >= n + 0.5, "the end met his blocker at {e:.2} s, the men inside at {n:.2} s");
}
