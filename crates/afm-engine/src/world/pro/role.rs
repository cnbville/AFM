//! A pro's role in the last season he played (code oddity 29).
//!
//! `Person::last_role` is a college player's last season's role from his
//! games (`college::follow_chart`, `college::finish`, the rule in
//! `college::real_role`). Nothing wrote it for a pro, so every reader of it
//! (the talks' "is he the starter" when he has seen nothing yet this
//! season, the promise ranks, the press, team rules, storylines, fame and
//! the rest) read his last college season. It is written here from his pro
//! seasons: at the end of each football year (`clock::rollover`), when
//! another league's season ends (`other::finish`), and once for a world or
//! save from before (`World::after_load`, `PRO_ROLE_REV`).
//!
//! Only the games he played are read, never a depth chart: a player never
//! sees the chart (Cain 2026-11-13; `folk::seen`). The sizes are Claude's
//! (proposed, not confirmed).

use crate::player::PlayingTime;
use crate::world::college::real_role;
use crate::world::{Stage, World};

use super::games::NFL_SEASON_SNAPS;

/// The version of the pros' roles in a world or save: a file from before
/// reads 0 and has them written when it loads.
pub(crate) const PRO_ROLE_REV: u8 = 1;

/// His role in the newest season he played (`season_of`).
#[cfg(test)]
pub(crate) fn season_role(w: &World, i: usize) -> Option<PlayingTime> {
    season_of(w, i).map(|x| x.0)
}

/// His role in the newest season he played, NFL or another league's: the
/// one he saw on the field.
///
/// - An NFL season is read by his snaps on his side of the ball, playoffs
///   included, per game he dressed, against a full starter's
///   (`NFL_SEASON_SNAPS` over the era's games): half is a starter, a fifth
///   the rotation, less a backup (`real_role`'s shares). Never by the row's
///   starts: those are the club's listed starters, whoever lined up. A
///   season the stand-in played (before `WorldOptions::engine_from`) keeps
///   no snaps, so its starts are all it has.
/// - A season on a club's books without a game: a backup when most of it
///   was on the roster (not hurt), else the practice squad. A season on
///   injured reserve, or unsigned, says nothing new: the one before stands.
/// - Another league's season by its starts (a start there is half his
///   side's top snaps in a game; its rows keep no snaps), so it reads a
///   starter or a backup (`write_league` reads a roster man who never
///   dressed).
/// - A kicker, punter or long snapper who dressed is the starter: one of
///   each dresses.
///
/// None when he has no such pro season (a rookie keeps his college role,
/// which his college games gave him).
///
/// With the year of the season it read.
fn season_of(w: &World, i: usize) -> Option<(PlayingTime, i32)> {
    let p = &w.people[i];
    let c = p.pro.as_ref()?;
    let pos = p.player.position;
    // His weeks on the roster not hurt (full pay status counts reserve).
    let roster = |s: &super::ProSeason| s.full_pay_weeks.saturating_sub(s.ir_weeks);
    let nfl = c.seasons.iter().rev().find(|s| s.games_all() > 0 || s.ps_weeks > 0 || roster(s) > 0);
    let other = c.other.iter().rev().find(|s| s.games > 0);
    // The newer season; in one year an NFL season ends after the others'.
    let other_newer = match (nfl, other) {
        (Some(n), Some(o)) => o.year > n.year,
        (None, Some(_)) => true,
        _ => false,
    };
    if other_newer {
        let s = other?;
        if pos.is_specialist() {
            return Some((PlayingTime::Starter, s.year));
        }
        let full = super::other::format(s.league, s.year).map_or(0, |f| f.games as u32);
        let games = s.games as u32;
        return Some((real_role(s.starts as u32, games, full.max(games), 0.0), s.year));
    }
    let s = nfl?;
    let year = s.year;
    let games = s.games_all();
    if games == 0 {
        return Some((if roster(s) > s.ps_weeks { PlayingTime::Backup } else { PlayingTime::PracticeSquad }, year));
    }
    if pos.is_specialist() {
        return Some((PlayingTime::Starter, year));
    }
    let season = w.changes.rules(s.year).nfl_games.max(1);
    if w.options.engine_from.is_some_and(|from| s.year < from) {
        return Some((real_role(s.starts_all(), games, season.max(games), 0.0), year));
    }
    let off = crate::world::folk::seen::offense(pos);
    let playoff = s.playoffs.as_ref().map_or(0, |l| if off { l.snaps_off } else { l.snaps_def });
    let snaps = s.snaps[if off { 0 } else { 1 }] + playoff;
    let full = NFL_SEASON_SNAPS as f64 / season as f64 * games as f64;
    Some((real_role(0, games, 0, snaps as f64 / full), year))
}

/// Writes the role of each of these pros from the season he last played.
/// A role read from a newer season stands (`ProCareer::role_year`):
/// another league's season he sat out on its roster keeps no row, so the
/// year's end would otherwise read the one before it.
pub(crate) fn write(w: &mut World, people: &[usize]) {
    for &i in people {
        if w.people[i].stage != Stage::Pro {
            continue;
        }
        let Some((r, year)) = season_of(w, i) else { continue };
        let p = &mut w.people[i];
        let Some(c) = p.pro.as_mut() else { continue };
        if year >= c.role_year {
            c.role_year = year;
            p.last_role = Some(r);
        }
    }
}

/// Writes the role of each man on another league's roster as its `year`
/// ends: the season he played, or a backup when he was on it and never
/// dressed (that league keeps no row without a game).
pub(crate) fn write_league(w: &mut World, men: &[usize], year: i32) {
    for &i in men {
        if w.people[i].stage != Stage::Pro {
            continue;
        }
        let dressed = w.people[i].pro.as_ref().is_some_and(|c| c.other.iter().any(|s| s.year == year && s.games > 0));
        if dressed {
            write(w, &[i]);
        } else {
            let p = &mut w.people[i];
            p.last_role = Some(PlayingTime::Backup);
            if let Some(c) = p.pro.as_mut() {
                c.role_year = c.role_year.max(year);
            }
        }
    }
}

/// Writes every pro's role: the football year is over.
pub(crate) fn write_all(w: &mut World) {
    let pros: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::Pro).collect();
    write(w, &pros);
}
