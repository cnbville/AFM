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
        && !crate::world::folk::rules::sitting(w, w.people[i].player.id, w.today)
}

fn elevations_used(w: &World, i: usize) -> u32 {
    w.people[i].pro.as_ref().and_then(|c| c.seasons.iter().find(|s| s.year == w.year)).map_or(0, |s| s.elevations)
}

/// The spots the coverage and return units are built from.
const TEAMS_SPOTS: [Position; 7] =
    [Position::LB, Position::S, Position::CB, Position::WR, Position::RB, Position::TE, Position::EDGE];

/// The elevations for a game (2020 on), two at most, each player within
/// his season limit, and inside the game-day limit as the real rule has
/// it. First a team short of healthy players at a spot (starters and
/// rotation) dresses its best squad players there; then, for special
/// teams, a squad player its staff grades clearly above the weakest
/// backup dressed at a coverage spot takes that backup's place, and a team
/// short of a full game-day list fills it from the squad. Trial 9 finding
/// 11: elevating only for an empty starting spot gave 0-13 a year, where
/// real clubs use one or two most weeks (the special teams swap and its
/// one-point margin proposed, not confirmed).
pub(crate) fn elevate(w: &World, t: usize, out: &mut Vec<(usize, bool)>) {
    let Some((per_game, per_player)) = elevation_limits(w.year) else { return };
    let limit = w.rules.nfl_actives as usize;
    let club = &w.clubs[t];
    let squad: Vec<(usize, f64)> = club
        .roster
        .iter()
        .copied()
        .filter(|&i| healthy(w, i, Status::PracticeSquad) && elevations_used(w, i) < per_player)
        .map(|i| (i, super::grade(w, t, i)))
        .collect();
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
        let mut here: Vec<(usize, f64)> = squad
            .iter()
            .copied()
            .filter(|&(i, _)| w.people[i].player.position == pos && !out.iter().any(|x| x.0 == i))
            .collect();
        here.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (i, _) in here.into_iter().take((s.starters + s.rotation - have).min(per_game - used)) {
            out.push((i, false));
            used += 1;
        }
    }
    // Over the limit: the deepest backups sit (the list was built in depth
    // order, so they are last).
    while out.len() > limit {
        let Some(k) = out.iter().rposition(|x| !x.1 && healthy(w, x.0, Status::Roster)) else { break };
        out.remove(k);
    }
    while used < per_game {
        let free: Vec<&(usize, f64)> = squad.iter().filter(|x| !out.iter().any(|y| y.0 == x.0)).collect();
        if out.len() < limit {
            let Some(&&(i, _)) = free.iter().max_by(|a, b| a.1.total_cmp(&b.1)) else { break };
            out.push((i, false));
            used += 1;
            continue;
        }
        // The weakest dressed backup at a coverage spot (past the starters
        // and rotation there), against the best squad player at one.
        let mut weakest: Option<(usize, f64)> = None;
        for (k, x) in out.iter().enumerate() {
            let pos = w.people[x.0].player.position;
            if x.1 || !TEAMS_SPOTS.contains(&pos) || !healthy(w, x.0, Status::Roster) {
                continue;
            }
            let s = depth_slots(pos);
            let rank = out[..k].iter().filter(|y| w.people[y.0].player.position == pos).count();
            if rank < s.starters + s.rotation {
                continue;
            }
            let g = super::grade(w, t, x.0);
            if weakest.is_none_or(|(_, v)| g < v) {
                weakest = Some((k, g));
            }
        }
        let best = free
            .iter()
            .filter(|x| TEAMS_SPOTS.contains(&w.people[x.0].player.position))
            .max_by(|a, b| a.1.total_cmp(&b.1));
        match (weakest, best) {
            (Some((k, v)), Some(&&(i, g))) if g > v + 1.0 => {
                out[k] = (i, false);
                used += 1;
            }
            _ => break,
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

/// Tuesday in season: a team with an open place on its 53 (an injury sent
/// someone to reserve) fills it at its thinnest spot with the best man it
/// can get: one of its own practice squad (signed up to the 53, the
/// common move), another team's (each team protects its best four from
/// 2020; the poaching team must keep him on its 53, and pay him, for at
/// least three weeks, real), or the street, which the roster fill signs
/// after this. Each is read the way the club reads a free agent
/// (`market::worth`: its own grade, his fit in its systems, the age
/// curve); a squad player gets his upside under 26 and his own squad
/// player a point for knowing the playbook; a street player loses what
/// time out of football costs (the same 3 points at no sharpness as the
/// camp read); a poach needs a point over the rest. Trial 9 finding 5:
/// read by the league's consensus alone, and only for a spot short of its
/// starters, nobody was poached in 25 years, and no team signed up its own
/// squad players in season (the weights proposed, not confirmed; a few
/// dozen poaches a year is from memory, to verify).
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
        // Your four are the ones you protect (lane 10).
        if let Some(mine) = crate::world::front_office::roster::protected(w, t) {
            let mut kept = 0;
            for i in squad {
                if mine.contains(&w.people[i].player.id) && kept < protect {
                    kept += 1;
                } else {
                    open.push((t, i));
                }
            }
            continue;
        }
        for i in squad.into_iter().skip(protect) {
            open.push((t, i));
        }
    }
    // The best three street players at each spot, by the league's read;
    // each club then reads them its own way.
    let mut street: Vec<Vec<(f64, usize)>> = vec![Vec::new(); crate::positions::POSITION_COUNT];
    for i in 0..w.people.len() {
        if w.people[i].pro.as_ref().is_some_and(|c| c.status == Status::FreeAgent && !c.on_waivers) {
            let k = w.people[i].player.position.index();
            street[k].push((market::consensus(w, i), i));
        }
    }
    for list in street.iter_mut() {
        list.sort_by(|a, b| b.0.total_cmp(&a.0));
        list.truncate(3);
    }
    let limit = w.rules.nfl_roster as usize;
    let upside = |w: &World, i: usize| 0.6 * (26.0 - w.people[i].player.age as f64).clamp(0.0, 4.0);
    let mut changed = false;
    for &t in &teams {
        // Your call-ups and signings are yours (lane 10).
        if crate::world::front_office::yours(w, t, crate::world::front_office::Duty::PracticeSquad) {
            continue;
        }
        for _ in 0..3 {
            if super::count(w, t, Status::Roster) >= limit {
                break;
            }
            // The thinnest spot: healthy men on the 53 against the depth
            // the team carries there.
            let thin = |p: Position| {
                let have = w.clubs[t].depth[p.index()].iter().filter(|&&j| healthy(w, j, Status::Roster)).count();
                depth_slots(p).roster as i64 - have as i64
            };
            let pos = Position::ALL.into_iter().max_by_key(|&p| (thin(p), std::cmp::Reverse(p.index()))).unwrap();
            let own = w.clubs[t]
                .roster
                .iter()
                .copied()
                .filter(|&i| w.people[i].player.position == pos && healthy(w, i, Status::PracticeSquad))
                .map(|i| (i, market::worth(w, t, i).0 + upside(w, i) + 1.0))
                .max_by(|a, b| a.1.total_cmp(&b.1));
            let theirs = open
                .iter()
                .filter(|&&(from, i)| from != t && w.people[i].player.position == pos)
                .map(|&(from, i)| {
                    let noise = 0.6 * key_normal(w.options.seed as u64 ^ 0x9AC4 ^ d.0 as u64, i as u64);
                    (from, i, market::worth(w, t, i).0 + upside(w, i) + noise)
                })
                .max_by(|a, b| a.2.total_cmp(&b.2));
            let vet = street[pos.index()]
                .iter()
                .map(|&(_, j)| {
                    let sharp = w.people[j].pro.as_ref().map_or(1.0, |c| c.sharpness);
                    market::worth(w, t, j).0 - 3.0 * (1.0 - sharp)
                })
                .fold(f64::MIN, f64::max);
            let pay = money::minimum(year, 0);
            if super::space(w, t, year) < pay {
                break;
            }
            let mine = own.map_or(f64::MIN, |x| x.1);
            match theirs {
                Some((from, i, g)) if g >= mine.max(vet) + 1.0 && g >= 56.0 => {
                    take(w, from, t, i, d, pay);
                    open.retain(|x| x.1 != i);
                    w.market.year_mut(year).poached += 1;
                }
                _ if own.is_some() && mine >= vet => {
                    sign_up(w, t, own.unwrap().0, pay);
                    open.retain(|x| x.1 != own.unwrap().0);
                }
                // The street is better: the roster fill signs him.
                _ => break,
            }
            changed = true;
        }
    }
    changed
}

/// Practice squad places open to players past the basic test, and the
/// most accrued seasons such a player may have (the NFL's rules, real):
/// from 2014 two players with up to two accrued seasons, from 2016 four,
/// and from 2020 six with any experience (the 2020 and 2021 seasons'
/// rule, kept from 2022). Trial 9 finding 5: without them every squad
/// held only young players, so a street veteran always beat them for an
/// open place on the 53 and almost no squad player was signed up, where
/// real clubs keep veterans on the squad for exactly that.
pub(crate) fn veteran_places(year: i32) -> (usize, u32) {
    match year {
        ..2014 => (0, 0),
        2014..2016 => (2, 2),
        2016..2020 => (4, 2),
        _ => (6, u32::MAX),
    }
}

/// The basic practice squad test: at most two seasons played, three on a
/// squad, 26 or younger.
pub(crate) fn squad_eligible(c: &super::ProCareer, age: u32) -> bool {
    c.seasons.iter().filter(|s| s.games > 0).count() <= 2 && c.ps_seasons < 3 && age <= 26
}

/// Whether a team's squad has room for this player: he passes the basic
/// test, or a veteran place is open and his accrued seasons fit it.
pub(crate) fn squad_room(w: &World, t: usize, i: usize) -> bool {
    let Some(c) = w.people[i].pro.as_ref() else { return false };
    if squad_eligible(c, w.people[i].player.age) {
        return true;
    }
    let (places, accrued) = veteran_places(w.year);
    c.accrued <= accrued && veterans_on_squad(w, t) < places
}

fn veterans_on_squad(w: &World, t: usize) -> usize {
    w.clubs[t]
        .roster
        .iter()
        .filter(|&&j| {
            w.people[j].pro.as_ref().is_some_and(|c| {
                c.status == Status::PracticeSquad && c.team == Some(t) && !squad_eligible(c, w.people[j].player.age)
            })
        })
        .count()
}

/// Tuesday in season, after the 53s are filled: a practice squad below the
/// era's size signs the best street players it has room for by the team's
/// grade (the cutdown's rule, veteran places included), so a squad player
/// signed up or poached is replaced the same week, as real teams do.
/// Weeks left in the season are paid.
pub(crate) fn refill_squads(w: &mut World, d: Date) -> bool {
    let year = w.year;
    let size = w.rules.nfl_practice_squad as usize;
    if size == 0 {
        return false;
    }
    let (places, accrued) = veteran_places(year);
    let free = |w: &World, i: usize| {
        w.people[i].pro.as_ref().is_some_and(|c| c.status == Status::FreeAgent && !c.on_waivers && c.out_weeks == 0)
            && !w.market.pending.iter().any(|x| x.person == i)
    };
    // The best 200 young players and the best 100 veterans who could fill
    // a veteran place.
    let mut young: Vec<usize> = Vec::new();
    let mut vets: Vec<usize> = Vec::new();
    for i in 0..w.people.len() {
        if !free(w, i) {
            continue;
        }
        let c = w.people[i].pro.as_ref().unwrap();
        if squad_eligible(c, w.people[i].player.age) {
            young.push(i);
        } else if places > 0 && c.accrued <= accrued {
            vets.push(i);
        }
    }
    let best = |w: &World, mut v: Vec<usize>, n: usize| {
        v.sort_by(|&a, &b| w.people[b].player.overall().total_cmp(&w.people[a].player.overall()));
        v.truncate(n);
        v
    };
    let mut pool = best(w, young, 200);
    pool.extend(best(w, vets, 100));
    if pool.is_empty() {
        return false;
    }
    let weeks_left = ((w.dates.super_bowl.0 - d.0) as f64 / 7.0).clamp(1.0, w.rules.nfl_games as f64 + 1.0);
    let pay = money::practice_squad_week(year) * weeks_left;
    let mut changed = false;
    for t in super::super::franchise::active_teams(year) {
        if crate::world::front_office::yours(w, t, crate::world::front_office::Duty::PracticeSquad) {
            continue;
        }
        while super::count(w, t, Status::PracticeSquad) < size {
            let Some(k) = (0..pool.len())
                .filter(|&k| free(w, pool[k]) && squad_room(w, t, pool[k]))
                .max_by(|&a, &b| super::grade(w, t, pool[a]).total_cmp(&super::grade(w, t, pool[b])))
            else {
                break;
            };
            let i = pool.swap_remove(k);
            if money::cap_books(year) && super::space(w, t, year) < pay {
                break;
            }
            let deal = money::build(t, money::Kind::PracticeSquad, year, &money::Terms::flat(1, pay));
            let name = super::possessive(w.nfl[t].name);
            let p = &mut w.people[i];
            let c = p.pro.as_mut().unwrap();
            c.team = Some(t);
            c.status = Status::PracticeSquad;
            c.contract = Some(deal);
            c.ps_seasons += 1;
            c.weeks_unsigned = 0;
            if c.teams.last() != Some(&t) {
                c.teams.push(t);
                c.new_team = true;
            }
            let age = p.row_age();
            p.note(format!("Signed to the {name} practice squad in season at {age}."));
            w.clubs[t].roster.push(i);
            changed = true;
        }
    }
    changed
}

/// One of a team's own practice squad signed up to its 53: a minimum deal
/// through the next season, as most are (proposed, not confirmed).
fn sign_up(w: &mut World, t: usize, i: usize, pay: f64) {
    let year = w.year;
    let accrued = w.people[i].pro.as_ref().map_or(0, |c| c.accrued);
    let terms = money::Terms::flat(2, money::minimum(year, accrued).max(pay));
    let deal = money::build(t, money::Kind::Veteran, year, &terms);
    let name = w.nfl[t].name;
    let p = &mut w.people[i];
    let c = p.pro.as_mut().unwrap();
    c.status = Status::Roster;
    c.contract = Some(deal);
    let age = p.row_age();
    p.note(format!("Signed up from the {} practice squad to the 53 at {age}.", super::possessive(name)));
    w.market.year_mut(year).signed_up += 1;
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
    p.note(format!(
        "Signed off the {} practice squad to the {} 53 at {age}.",
        super::possessive(a),
        super::possessive(b)
    ));
    w.clubs[t].roster.push(i);
    let who = super::describe(w, i);
    market::post(
        w,
        d,
        Some(t),
        Some(i),
        crate::calendar::Loudness::Small,
        crate::calendar::Kind::Signing,
        format!("{who} is signed off the {} practice squad by the {b}.", super::possessive(a)),
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
        "Released by the {name} at {age} with an injury settlement (${:.0}K for about {} {}).",
        owed / 1e3,
        weeks as u32,
        crate::world::folk::noun(weeks as u32, "week")
    ));
    w.market.year_mut(year).settlements += 1;
}
