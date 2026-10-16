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
