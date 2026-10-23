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
            // A snapper hurt in the game is replaced (the next best snap), so
            // only a healthy one must be on the unit.
            let hurt: HashSet<u32> = res.injuries.iter().map(|(_, i)| i.id).collect();
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
                if level != Level::HS && snappers[r.offense].iter().all(|id| !hurt.contains(id)) {
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

/// A team with no long snapper whose best snap is its backup kicker: when
/// the starting kicker is hurt, the backup kicks and someone else snaps, so
/// no man is on the unit twice.
#[test]
fn the_kicker_never_snaps_to_himself() {
    use crate::attributes::Attr;
    use crate::game::flow::kick_unit_men;
    let mut team = synthetic_league(Level::FBS, 1, 21).remove(0);
    team.players.retain(|p| p.position != Position::LS);
    let k1 = team.players.iter().position(|p| p.position == Position::K).unwrap();
    let mut k2 = team.players[k1].clone();
    k2.id = team.players.iter().map(|p| p.id).max().unwrap() + 1;
    k2.attrs.add(Attr::ShortKickAccuracy, -(15.0));
    k2.attrs.add(Attr::LongKickAccuracy, -(15.0));
    k2.attrs.add(Attr::KickPower, -(15.0));
    k2.attrs.add(Attr::Composure, -(15.0));
    k2.attrs.set(Attr::LongSnapping, team.players.iter().map(|p| p.a(Attr::LongSnapping)).fold(0.0, f64::max) + 5.0);
    team.players.push(k2);
    let k2 = team.players.len() - 1;
    // With everyone fit the backup kicker is the snapper.
    let [k, _, sn] = kick_unit_men(&team, &[]);
    assert_eq!((k, sn), (k1, k2));
    // The starter is hurt: the backup kicks, and the snap goes to someone else.
    let [k, p, sn] = kick_unit_men(&team, &[k1]);
    assert_eq!(k, k2);
    assert!(sn != k && sn != p, "kicker {k}, punter {p}, snapper {sn}");
}

/// An arena roster of 24 in a blowout, with its returners and three more
/// hurt and every other man pulled: the pulled starters come back for the
/// return unit, so a kickoff always has someone to field it (a test world's
/// arena game crashed on a kickoff to an empty unit).
#[test]
fn a_thin_roster_fields_its_kick_units_in_a_blowout() {
    use crate::game::flow::kick_unit_with;
    use Position::*;
    let full = synthetic_league(Level::NFL, 1, 7).remove(0);
    let keep = |pos: Position| match pos {
        WR | IOL => 4,
        RB | EDGE | DT | LB => 2,
        CB => 3,
        LS | P => 0,
        _ => 1,
    };
    let mut players = Vec::new();
    for pos in Position::ALL {
        players.extend(full.players.iter().filter(|p| p.position == pos).take(keep(pos)).cloned());
    }
    let team = crate::game::GameTeam::new(full.name.clone(), players, full.staff);
    let n = team.players.len();
    assert_eq!(n, 24);
    let out: Vec<usize> = (0..n).filter(|&i| matches!(team.players[i].position, RB | WR)).take(5).collect();
    let benched: Vec<usize> = (0..n).filter(|i| !out.contains(i)).collect();
    for receiving in [true, false] {
        let on = kick_unit_with(&team, &out, &benched, receiving, 8);
        assert_eq!(on.len(), 8, "receiving {receiving}: {on:?}");
        assert!(on.iter().all(|i| !out.contains(i)), "a hurt man on the unit: {on:?}");
        assert_eq!(on.iter().collect::<HashSet<_>>().len(), 8, "a man listed twice: {on:?}");
    }
}
