//! The game's stat book.

use std::collections::HashSet;

use crate::game::quick_game;
use crate::game::synthetic::synthetic_league;
use crate::player::Level;

/// Every player's line lands on his own team, so no player shows up on
/// both sides of a box score. Interception returns used to book the
/// defense's return blockers (and the offense men they blocked, and an
/// offense man forcing the returner's fumble) on the wrong team: about
/// three lines a game, and in the world 2% of season rows named the
/// opponent.
#[test]
fn player_lines_stay_on_their_own_team() {
    for level in [Level::FBS, Level::NFL] {
        let teams = synthetic_league(level, 32, 7);
        let mut ints = 0;
        for g in 0..60 {
            let h = (g * 7) % 32;
            let a = (g * 13 + 5) % 32;
            let a = if a == h { (a + 1) % 32 } else { a };
            let res = quick_game(&teams[h], &teams[a], level, 7_000_003 + g as u64);
            let s = &res.stats;
            ints += s.plays.iter().filter(|r| r.interception).count();
            let roster = [&teams[h], &teams[a]].map(|t| t.players.iter().map(|p| p.id).collect::<HashSet<u32>>());
            for side in 0..2 {
                for id in s.players[side].keys() {
                    assert!(
                        !s.players[1 - side].contains_key(id),
                        "{level:?} game {g}: player {id} has a line on both sides"
                    );
                    assert!(roster[side].contains(id), "{level:?} game {g}: player {id} booked for the other team");
                }
            }
        }
        // The case that broke it has to come up.
        assert!(ints >= 20, "{level:?}: only {ints} interceptions in 60 games");
    }
}

/// Chemistry (X5): the same offense completes more with its receivers in
/// step with the quarterback than with a cold group, by a few points, and a
/// cold secondary gives up more. Typical chemistry changes nothing.
/// Measured over four leagues, since the gap depends on the roster: over
/// 8 leagues (2026-11-16) it ran 2.0 to 3.5 points with the 99 wall and
/// -0.3 to 4.1 without it (mean 2.1), once elite men's ratings came out as
/// uneven as anyone's. One league tests its own quarterback and receivers
/// more than chemistry.
#[test]
fn chemistry_moves_completions_a_little() {
    use crate::positions::Position;
    let level = Level::NFL;
    let with = |chem: f32, pos: &[Position], t: &crate::game::roster::GameTeam| {
        let mut t = t.clone();
        for p in t.players.iter_mut().filter(|p| pos.contains(&p.position) || p.position == Position::QB) {
            p.chem = chem;
        }
        t
    };
    let rate = |off: &crate::game::roster::GameTeam, def: &crate::game::roster::GameTeam| {
        let (mut c, mut a) = (0u32, 0u32);
        for g in 0..120u64 {
            let res = quick_game(off, def, level, 91_000 + g);
            for r in res.stats.plays.iter().filter(|r| r.offense == 0 && r.target.is_some()) {
                a += 1;
                c += r.complete as u32;
            }
        }
        c as f64 / a.max(1) as f64
    };
    let recv = [Position::WR, Position::TE, Position::RB];
    let (mut warm, mut cold) = (0.0, 0.0);
    for seed in 31..35 {
        let teams = synthetic_league(level, 2, seed);
        warm += rate(&with(0.95, &recv, &teams[0]), &teams[1]) / 4.0;
        cold += rate(&with(0.05, &recv, &teams[0]), &teams[1]) / 4.0;
    }
    let gap = warm - cold;
    assert!(gap > 0.01 && gap < 0.08, "warm {warm:.3} cold {cold:.3}");
}

/// Tackles and tackles for loss go in the book the way a box score keeps
/// them (realism audit item 3). A tackle is one man's or two men's: one man
/// has a solo, two have an assist each and no solo. A tackle for loss is a
/// loss of half a yard or more, since yards go in the book whole. Every
/// man in the pile used to get credit, the first man a solo, and any loss
/// counted, even an inch: with the block's straight read of a rating gap
/// that made a 196-tackle season and one of 52.7 tackles for loss.
#[test]
fn tackles_and_losses_are_kept_like_a_box_score() {
    use crate::game::stats::PlayType;
    for level in [Level::FBS, Level::NFL] {
        let teams = synthetic_league(level, 32, 7);
        let (mut shared, mut inches) = (0, 0);
        for g in 0..24 {
            let h = (g * 7) % 32;
            let a = (g * 13 + 5) % 32;
            let a = if a == h { (a + 1) % 32 } else { a };
            let res = quick_game(&teams[h], &teams[a], level, 7_400_003 + g as u64);
            let s = &res.stats;
            let (mut solo, mut assists, mut losses) = (0u32, 0u32, 0.0f64);
            // A play called back and a try are in the log, not in the book.
            for r in s.plays.iter().filter(|r| !r.nullified && r.kind != PlayType::TwoPoint) {
                let n = r.tacklers.len();
                assert!(n <= 2, "{level:?} game {g}: {n} men credited with one tackle");
                if n == 1 {
                    solo += 1;
                } else {
                    assists += n as u32;
                    shared += (n == 2) as u32;
                }
                // A sack is a tackle for loss for the men who share it; any
                // other loss on a scrimmage play is one for its tacklers.
                if !r.sackers.is_empty() {
                    losses += 1.0;
                }
                if matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Scramble) && n > 0 {
                    losses += r.lost() as u32 as f64;
                    inches += (r.yards < 0.0 && !r.lost()) as u32;
                }
            }
            let lines = || s.players.iter().flat_map(|side| side.values());
            assert_eq!(lines().map(|l| l.tackles).sum::<u32>(), solo, "{level:?} game {g}: solo tackles");
            assert_eq!(lines().map(|l| l.assists).sum::<u32>(), assists, "{level:?} game {g}: assists");
            let booked: f64 = lines().map(|l| l.tfl).sum();
            assert!((booked - losses).abs() < 1e-6, "{level:?} game {g}: {booked} tackles for loss, {losses} losses");
        }
        // Both cases have to come up.
        assert!(shared >= 200, "{level:?}: only {shared} shared tackles in 24 games");
        assert!(inches >= 10, "{level:?}: only {inches} losses under half a yard in 24 games");
    }
}

/// A pressure is a hurry, a hit or a sack on a man's line, as the NFL's
/// charters keep it, and a rush snap is once a dropback he rushed, blocked
/// or not (realism audit item 3, the pass rush lane). The book counted only
/// the men first near the passer as pressures, and a rush snap a pass block,
/// so a blitzer nobody blocked had no rush.
#[test]
fn a_sack_or_a_hit_is_a_pressure_on_his_line() {
    use crate::game::stats::PlayType;
    use std::collections::HashMap;
    let teams = synthetic_league(Level::NFL, 32, 7);
    let mut late = 0;
    let mut free = 0;
    for g in 0..12 {
        let h = (g * 7) % 32;
        let a = (g * 13 + 5) % 32;
        let a = if a == h { (a + 1) % 32 } else { a };
        let res = quick_game(&teams[h], &teams[a], Level::NFL, 7_500_003 + g as u64);
        let s = &res.stats;
        let mut want: HashMap<u32, u32> = HashMap::new();
        let mut rushes: HashMap<u32, u32> = HashMap::new();
        for r in s.plays.iter().filter(|r| !r.nullified && r.kind != PlayType::TwoPoint) {
            // A rush snap: once a dropback for every man who rushed, blocked
            // or not.
            let mut rush: Vec<u32> = r.on_field[1]
                .iter()
                .take(16)
                .enumerate()
                .filter(|&(k, _)| (r.rushed >> k) & 1 == 1)
                .map(|(_, &id)| id)
                .collect();
            free += rush.iter().filter(|&&id| !r.blocks.iter().any(|b| b.1 == id && b.2)).count();
            rush.extend(r.blocks.iter().filter(|b| b.2).map(|b| b.1));
            rush.sort_unstable();
            rush.dedup();
            for id in rush {
                *rushes.entry(id).or_default() += 1;
            }
            let mut men: Vec<u32> =
                r.pressures.iter().chain(&r.hits).chain(r.sackers.iter().take(2)).copied().collect();
            men.sort_unstable();
            men.dedup();
            late += men.iter().filter(|id| !r.pressures.contains(id)).count();
            for id in men {
                *want.entry(id).or_default() += 1;
            }
        }
        for (id, l) in s.players.iter().flat_map(|side| side.iter()) {
            assert_eq!(l.pressures, want.get(id).copied().unwrap_or(0), "game {g}: pressures of {id}");
            assert!(l.sacks <= l.pressures as f64, "game {g}: {id} has {} sacks on {} pressures", l.sacks, l.pressures);
            assert_eq!(l.rush_snaps, rushes.get(id).copied().unwrap_or(0), "game {g}: rush snaps of {id}");
            assert!(l.rush_wins <= l.rush_snaps, "game {g}: {id} won {} of {} rushes", l.rush_wins, l.rush_snaps);
        }
    }
    // The cases have to come up.
    assert!(late >= 30, "only {late} hits or sacks by a man not first near the passer in 12 games");
    assert!(free >= 30, "only {free} rushes nobody blocked in 12 games");
}
