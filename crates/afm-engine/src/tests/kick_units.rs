//! Kicking units on the field.

use std::collections::HashSet;

use crate::game::quick_game;
use crate::game::stats::PlayType;
use crate::game::synthetic::synthetic_league;
use crate::player::Level;
use crate::positions::Position;

/// Field goal, extra point and punt units are 11 men with the long snapper
/// (and the holder on a kick), so the snapper logs his games. He used to be
/// left off the list: he played no snaps, his seasons went uncounted and a
/// starting snapper's year read as a redshirt.
#[test]
fn long_snapper_is_on_the_kicking_units() {
    for level in [Level::HS, Level::FBS, Level::NFL] {
        let teams = synthetic_league(level, 32, 11);
        let mut kicks = 0;
        for g in 0..12 {
            let (h, a) = (g * 2, g * 2 + 1);
            let res = quick_game(&teams[h], &teams[a], level, 9_100_007 + g as u64);
            let s = &res.stats;
            let snappers = [h, a].map(|t| {
                teams[t].players.iter().filter(|p| p.position == Position::LS).map(|p| p.id).collect::<Vec<u32>>()
            });
            let mut kicked = [false; 2];
            for r in &s.plays {
                if !matches!(r.kind, PlayType::FieldGoal | PlayType::ExtraPoint | PlayType::Punt) {
                    continue;
                }
                kicks += 1;
                let on = &r.on_field[0];
                let distinct: HashSet<u32> = on.iter().copied().collect();
                assert_eq!(on.len(), 11, "{level:?} game {g}: {:?} unit has {} men", r.kind, on.len());
                assert_eq!(distinct.len(), 11, "{level:?} game {g}: a man listed twice on the {:?} unit", r.kind);
                if level != Level::HS {
                    assert!(
                        snappers[r.offense].iter().any(|id| on.contains(id)),
                        "{level:?} game {g}: no long snapper on the {:?} unit",
                        r.kind
                    );
                }
                kicked[r.offense] = true;
            }
            for side in 0..2 {
                if kicked[side] {
                    for id in &snappers[side] {
                        let st = s.players[side].get(id).map_or(0, |l| l.snaps_st);
                        assert!(st > 0, "{level:?} game {g}: long snapper {id} logged no snaps");
                    }
                }
            }
        }
        assert!(kicks >= 100, "{level:?}: only {kicks} kicks in 12 games");
    }
}
