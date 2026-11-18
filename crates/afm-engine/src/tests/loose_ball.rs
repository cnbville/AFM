//! The loose ball (engine gaps lane EG2, `docs/design/engine-gaps.md` items
//! 4-7): fumbles stay live by each level's rules, snaps go on the ground,
//! and the ball's last owner is the team the drive goes on with.

use crate::game::quick_game;
use crate::game::rules::{FumbleRule, Rules};
use crate::game::sim::FumbleHow;
use crate::game::stats::PlayType;
use crate::game::synthetic::synthetic_league;
use crate::player::Level;

/// Who may advance a fumble, by the book: the NFL keeps a fumbling team's
/// recovery to the fumbler on fourth down, on a try and after the
/// two-minute warning, college on fourth down and a try, high school lets
/// anyone; a high school fumble forward out of bounds came back to its spot
/// only from 2025, and the states on college rules play the NCAA's.
#[test]
fn fumble_rules_by_level() {
    assert_eq!(Rules::for_level(Level::NFL).fumble_rule(), FumbleRule::NFL);
    assert_eq!(Rules::for_level(Level::FBS).fumble_rule(), FumbleRule::NCAA);
    assert_eq!(Rules::for_level(Level::HS).fumble_rule(), FumbleRule::NFHS);
    assert!(!Rules::for_year(Level::HS, 2020, false).fumble_rule().forward_back);
    assert!(Rules::for_year(Level::HS, 2025, false).fumble_rule().forward_back);
    assert_eq!(Rules::high_school_on_college_rules().fumble_rule(), FumbleRule::NCAA);
    assert!(FumbleRule::NFL.late && !FumbleRule::NCAA.late && !FumbleRule::NFHS.fourth_down);
}

/// Fumbles play out: some go back for a score, some go out of bounds, the
/// defense runs some back, snaps go on the ground under center and in the
/// shotgun, and every lost fumble was recovered by the defense of the play
/// (a fumble the offense kept was the offense's, or out of bounds).
#[test]
fn loose_balls_play_out() {
    let (mut fumbles, mut out, mut ran, mut scores, mut snaps) = (0, 0, 0, 0, [0, 0]);
    for level in [Level::NFL, Level::FBS] {
        let teams = synthetic_league(level, 32, 11);
        for g in 0..120u64 {
            let h = (g as usize * 7) % 32;
            let a = (g as usize * 13 + 5) % 32;
            let a = if a == h { (a + 1) % 32 } else { a };
            let res = quick_game(&teams[h], &teams[a], level, 4_100_007 + g);
            for p in &res.stats.plays {
                let Some(note) = p.fumble else { continue };
                if p.fumbled.is_none() || matches!(p.kind, PlayType::Kickoff | PlayType::Punt) {
                    continue;
                }
                fumbles += 1;
                out += note.out as usize;
                let d = 1 - p.offense;
                let by_d = p.recovered.is_some_and(|id| p.side_of(id) == Some(d));
                if by_d && note.ran >= 1.0 {
                    ran += 1;
                }
                if p.defense_td && !p.interception {
                    scores += 1;
                    assert!(p.turnover, "game {g}: a fumble return for a score without a turnover: {}", p.why);
                }
                if note.how == FumbleHow::Snap {
                    snaps[p.call.contains("under center") as usize] += 1;
                }
                if p.fumble_lost && !p.nullified {
                    assert!(by_d || note.out, "game {g}: a lost fumble the defense never had: {}", p.why);
                }
                if by_d && !p.interception && !p.nullified && !p.fumble_lost && !p.touchdown {
                    panic!("game {g}: the defense came up with it and the offense kept it: {}", p.why);
                }
            }
        }
    }
    assert!(fumbles >= 300, "only {fumbles} fumbles in 240 games");
    assert!(out >= 5, "only {out} of {fumbles} fumbles went out of bounds");
    assert!(ran >= 20, "the defense ran back only {ran} of {fumbles} fumbles");
    assert!(scores >= 1, "no fumble went back for a score in 240 games");
    assert!(snaps[0] >= 10 && snaps[1] >= 10, "fumbled snaps in the gun and under center: {snaps:?}");
}
