//! Section 5 leftovers, built 2026-10-19 (`docs/design/team-management.md`,
//! "The NFL year, month by month"): practice squad elevations, the four
//! protected practice squad players and poaching from other squads, and
//! injury settlements for players cut while hurt. The rules are the real
//! ones by era; the AI's habits are Claude's, proposed, not confirmed.

use super::{DeadWhy, Status, market, money};
use crate::calendar::Date;
use crate::league::depth_slots;
use crate::positions::Position;
use crate::world::World;
use crate::world::season::key_normal;

/// Game-day elevations from the practice squad (2020 on): two a game, and
/// each player at most twice a season in 2020-21, three times from 2022.
pub(crate) fn elevation_limits(year: i32) -> Option<(usize, u32)> {
    match year {
        ..=2019 => None,
        2020 | 2021 => Some((2, 2)),
        _ => Some((2, 3)),
    }
}

/// Practice squad players a team may protect each week (2020 on).
pub(crate) fn protected_count(year: i32) -> usize {
    if year >= 2020 { 4 } else { 0 }
}

fn healthy(w: &World, i: usize, status: Status) -> bool {
    w.people[i]
        .pro
        .as_ref()
        .is_some_and(|c| c.status == status && c.out_weeks == 0 && crate::world::people::available(c))
}

fn elevations_used(w: &World, i: usize) -> u32 {
    w.people[i].pro.as_ref().and_then(|c| c.seasons.iter().find(|s| s.year == w.year)).map_or(0, |s| s.elevations)
}

/// The elevations for a game: a team short of healthy players at a spot
/// (starters and rotation) dresses its best practice squad players there,
/// two at most, each within his season limit. Elevated players come on top
/// of the game-day limit, as the real rule allows.
pub(crate) fn elevate(w: &World, t: usize, out: &mut Vec<(usize, bool)>) {
    let Some((per_game, per_player)) = elevation_limits(w.year) else { return };
    let club = &w.clubs[t];
    let mut used = 0;
    for pos in Position::ALL {
        if used >= per_game {
            break;
        }
        let s = depth_slots(pos);
        let have = out.iter().filter(|x| w.people[x.0].player.position == pos).count();
        if have >= s.starters + s.rotation {
            continue;
        }
        let mut squad: Vec<usize> = club
            .roster
            .iter()
            .copied()
            .filter(|&i| {
                w.people[i].player.position == pos
                    && healthy(w, i, Status::PracticeSquad)
                    && elevations_used(w, i) < per_player
                    && !out.iter().any(|x| x.0 == i)
            })
            .collect();
        squad.sort_by(|&a, &b| super::grade(w, t, b).total_cmp(&super::grade(w, t, a)));
        for i in squad.into_iter().take((s.starters + s.rotation - have).min(per_game - used)) {
            out.push((i, false));
            used += 1;
        }
    }
}

/// After a game: count each elevated player's elevation.
pub(crate) fn count_elevation(w: &mut World, i: usize) {
    let year = w.year;
    if let Some(c) = w.people[i].pro.as_mut()
        && c.status == Status::PracticeSquad
    {
        c.season_mut(year).elevations += 1;
    }
}

/// Tuesday in season: a team short at a spot looks at other teams'
/// practice squads first. Each team protects its best four (2020 on); the
/// rest can be signed to another team's 53, which must keep him (and pay
/// him) for at least three weeks (real). A poach needs a real gain over the
/// street, so most weeks bring none and a season a few dozen (real: a few
/// dozen a year).
pub(crate) fn poach(w: &mut World, d: Date) -> bool {
    let year = w.year;
    if !money::capped(year) && year < 1990 {
        return false;
    }
    let teams = super::super::franchise::active_teams(year);
    let protect = protected_count(year);
    // Each team's protected list: its best squad players by its own grade.
    let mut open: Vec<(usize, usize)> = Vec::new();
    for &t in &teams {
        let mut squad: Vec<usize> =
            w.clubs[t].roster.iter().copied().filter(|&i| healthy(w, i, Status::PracticeSquad)).collect();
        squad.sort_by(|&a, &b| super::grade(w, t, b).total_cmp(&super::grade(w, t, a)));
        for i in squad.into_iter().skip(protect) {
            open.push((t, i));
        }
    }
    // The best street player at each spot, by the league's read.
    let mut street = [0.0f64; crate::positions::POSITION_COUNT];
    for i in 0..w.people.len() {
        if w.people[i].pro.as_ref().is_some_and(|c| c.status == Status::FreeAgent && !c.on_waivers) {
            let k = w.people[i].player.position.index();
            street[k] = street[k].max(market::consensus(w, i));
        }
    }
    let limit = w.rules.nfl_roster as usize;
    let mut changed = false;
    for &t in &teams {
        if super::count(w, t, Status::Roster) >= limit {
            continue;
        }
        for pos in Position::ALL {
            let s = depth_slots(pos);
            let healthy_here: Vec<f64> = w.clubs[t].depth[pos.index()]
                .iter()
                .filter(|&&j| healthy(w, j, Status::Roster))
                .map(|&j| super::grade(w, t, j))
                .collect();
            if healthy_here.len() >= s.starters + s.rotation {
                continue;
            }
            // The best open squad player at the spot, by this team's grade,
            // against the best on the street.
            let best = open
                .iter()
                .filter(|&&(from, i)| from != t && w.people[i].player.position == pos)
                .map(|&(from, i)| {
                    let noise = 0.6 * key_normal(w.options.seed as u64 ^ 0x9AC4 ^ d.0 as u64, i as u64);
                    (from, i, market::consensus(w, i) + noise)
                })
                .max_by(|a, b| a.2.total_cmp(&b.2));
            let Some((from, i, g)) = best else { continue };
            if g < street[pos.index()] + 2.0 || g < 56.0 {
                continue;
            }
            let pay = money::minimum(year, 0);
            if super::space(w, t, year) < pay {
                continue;
            }
            take(w, from, t, i, d, pay);
            open.retain(|x| x.1 != i);
            w.market.year_mut(year).poached += 1;
            changed = true;
            break;
        }
    }
    changed
}

fn take(w: &mut World, from: usize, t: usize, i: usize, d: Date, pay: f64) {
    let year = w.year;
    w.clubs[from].roster.retain(|&x| x != i);
    let mut terms = money::Terms::flat(1, pay);
    // Three weeks' pay is guaranteed (real: the poaching team keeps him on
    // its 53 for three weeks).
    terms.base_guarantee = 3.0 / 18.0;
    let deal = money::build(t, money::Kind::Veteran, year, &terms);
    let (a, b) = (w.nfl[from].name, w.nfl[t].name);
    let p = &mut w.people[i];
    let c = p.pro.as_mut().unwrap();
    c.team = Some(t);
    c.status = Status::Roster;
    c.contract = Some(deal);
    c.new_team = true;
    c.teams.push(t);
    let age = p.row_age();
    p.note(format!("Signed off the {a}'s practice squad to the {b}'s 53 at {age}."));
    w.clubs[t].roster.push(i);
    let who = super::describe(w, i);
    market::post(
        w,
        d,
        Some(t),
        crate::calendar::Loudness::Small,
        crate::calendar::Kind::Signing,
        format!("{who} is signed off the {a}'s practice squad by the {b}."),
    );
}

/// An injury settlement (real): a player cut while hurt is paid for the
/// weeks he is expected to miss, and the team owes it on this year's cap.
pub(crate) fn settle_injury(w: &mut World, t: usize, i: usize) {
    let year = w.year;
    let Some(c) = w.people[i].pro.as_ref() else { return };
    if c.out_weeks == 0 {
        return;
    }
    let weeks = (c.out_weeks as f64).min(w.rules.nfl_games as f64);
    let base = c.contract.as_ref().and_then(|k| k.year(year)).map_or(money::minimum(year, 0), |y| y.base);
    let owed = base * weeks / w.rules.nfl_games as f64;
    let id = w.people[i].player.id;
    w.clubs[t].add_dead(year, owed, id, DeadWhy::Settlement);
    let name = w.nfl[t].name;
    let p = &mut w.people[i];
    let age = p.row_age();
    p.note(format!(
        "Released by the {name} at {age} with an injury settlement (${:.0}K for about {} weeks).",
        owed / 1e3,
        weeks as u32
    ));
    w.market.year_mut(year).settlements += 1;
}
