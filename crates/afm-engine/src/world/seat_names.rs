//! The names in your kept games (update 2, lane 9). A player who leaves
//! football at the year's turn leaves the world, but your games from last
//! season are still kept whole: each keeps the names of its players who
//! left, so its pages and boards still name them.

use crate::game::stats::PlayRecord;
use crate::positions::Position;

use super::World;
use super::seat::SeatGame;

/// A player in one of your games who has left the world.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Gone {
    pub id: u32,
    pub name: String,
    pub pos: Position,
}

/// Every player a play names: the ball, the tackle, the blocks and the
/// line chart.
fn play_ids(r: &PlayRecord) -> impl Iterator<Item = u32> + '_ {
    let one = [
        r.passer,
        r.target,
        r.carrier,
        r.defender,
        r.pbu,
        r.interceptor,
        r.forced_fumble,
        r.fumbled,
        r.kicker,
        r.returner,
    ];
    let blocks = r.blocks.iter().flat_map(|b| [b.0, b.1]);
    let line = r.line.iter().flat_map(|c| c.reps.iter().flat_map(|x| [x.blocker, x.defender]));
    one.into_iter()
        .flatten()
        .chain(r.tacklers.iter().copied())
        .chain(r.sackers.iter().copied())
        .chain(blocks)
        .chain(line)
}

/// Before the people who left football are dropped (`map` holds where
/// each person lands, None for those dropped), each of your kept games
/// keeps the names of its players among them.
pub(crate) fn keep(w: &mut World, map: &[Option<usize>]) {
    for k in 0..w.seat_games.len() {
        let g = &w.seat_games[k];
        let mut ids: Vec<u32> = g.stats.players.iter().flat_map(|m| m.keys().copied()).collect();
        ids.extend(g.stats.plays.iter().flat_map(play_ids));
        ids.sort_unstable();
        ids.dedup();
        let gone: Vec<Gone> = ids
            .into_iter()
            .filter(|&id| !g.gone.iter().any(|x| x.id == id))
            .filter_map(|id| {
                let i = super::stories::person_index(w, id)?;
                map.get(i)?.is_none().then(|| {
                    let p = &w.people[i];
                    Gone { id, name: p.name.clone(), pos: p.player.position }
                })
            })
            .collect();
        w.seat_games[k].gone.extend(gone);
    }
}

/// The name and position of a player in a kept game who has left.
pub fn gone(g: &SeatGame, id: u32) -> Option<&Gone> {
    g.gone.iter().find(|x| x.id == id)
}
