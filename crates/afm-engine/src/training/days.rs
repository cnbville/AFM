//! Practice days (the six-part day, `docs/design/day-sections.md` topic 4
//! item 2 and topic 6): a practice injury shows after that day's practice,
//! not in a weekly batch. Each team practices at its level's real hours on
//! its own clock: NFL clubs late morning to midafternoon (camp in the
//! morning), college programs by their own custom (some in the morning,
//! most from about 4 PM), high schools after the last bell (mornings in the
//! summer heat). Every NFL club's practice day draws its own share of the
//! week's injury risk when its practice ends; college and high school
//! injuries are rolled with the week's plan and show when their day's
//! practice ends. At your seat you set your own practice hour.
//!
//! Proposed, not confirmed: the practice hours (NFL in season 11:15 AM to
//! 1:30 PM local, camp 9:15 to 11:30 AM; college mornings 8:45 to 11:30 AM
//! for one program in five by its custom, afternoons 3:45 to 6:30 PM; high
//! school 3:30 to 6:30 PM, and 7:30 to 10:30 AM before August 15), the NFL's
//! practice days (the three days before a game that are not the day after
//! one; in camp every day but a preseason game's day and the day after it),
//! the shares (a third of the season week's risk a practice day, a sixth of
//! camp's) and the hours you may set (high school after a 3 PM bell in the
//! school year, college inside 6 AM to 9 PM, the NFL 8 AM to 3 PM starts).

use super::rules::Tier;
use crate::calendar::zone::{self, DAY};
use crate::calendar::{Date, Level, Moment, Zone};
use crate::world::World;
use crate::world::franchise;
use crate::world::season::key_uniform;

/// An NFL club's zone (its town in this season).
pub(crate) fn club_zone(w: &World, t: usize) -> Zone {
    let (town, st) = crate::world::finance::nfl::city(w, t, w.year);
    Zone::of_place(&st, &town, None)
}

/// The day minute a practice ending at `local` (minutes after local
/// midnight) in `zone` ends, on world day `d`.
fn ends(zone: Zone, d: Date, local: u16) -> u16 {
    let et = local as i32 + 60 * zone.behind(d);
    ((et - 5 * 60).rem_euclid(DAY as i32)) as u16
}

/// Whether NFL club `t` practices on `d`, and the share of the week's
/// practice risk the day carries.
fn nfl_practice(w: &World, t: usize, d: Date) -> Option<f64> {
    let y = &w.dates;
    if d < y.veterans_report || d > y.super_bowl {
        return None;
    }
    let games = club_games(w, t);
    let played_yesterday = games.contains(&d.add(-1));
    if games.contains(&d) || played_yesterday {
        return None;
    }
    if d < y.cutdown.date {
        // Camp: every day but a preseason game's and the day after it.
        return Some(1.0 / 6.0);
    }
    // In season: the three days before the next game, a third each.
    let next = games.iter().copied().filter(|&g| g > d).min()?;
    (2..=4).contains(&d.days_to(next)).then_some(1.0 / 3.0)
}

/// A club's game days this season: preseason, regular season and the
/// playoff rounds it is still alive for.
fn club_games(w: &World, t: usize) -> Vec<Date> {
    let mut v: Vec<Date> = crate::world::pro::preseason_dates(w);
    let y = &w.dates;
    if let Some(s) = w.nfl_season.as_ref() {
        for (k, wk) in s.weeks.iter().enumerate() {
            for (g, &(a, b)) in wk.iter().enumerate() {
                if (a == t || b == t)
                    && let Some(x) = s.slots.get(k).and_then(|sl| sl.get(g))
                {
                    v.push(x.at.date);
                }
            }
        }
        // The playoffs: a seeded club still in plays the next round's day.
        let alive = s.seed.get(t).is_some_and(|x| x.is_some()) && s.out_round.get(t).is_none_or(|&r| r == 0);
        if alive {
            v.extend(s.round_games.iter().filter(|g| g.2 == t || g.3 == t).map(|g| g.4.date));
            for day in [y.wild_card, y.divisional, y.conference_title_games, y.super_bowl] {
                v.push(day);
            }
        }
    }
    v
}

/// How long a practice runs on the field, by level (minutes).
pub fn practice_length(tier: Tier) -> u16 {
    match tier {
        Tier::High => 180,
        Tier::College => 165,
        Tier::Pro => 135,
    }
}

/// A college program that practices in the morning by its custom (one in
/// five; Purdue under Ryan Walters, South Carolina's spring of 2026).
fn morning_program(w: &World, t: usize) -> bool {
    key_uniform(w.options.seed as u64 ^ 0x4A31, t as u64) < 0.2
}

/// Before the school year: a high school practices in the cool of the
/// morning (the states' heat rules).
fn summer(d: Date) -> bool {
    let (_, m, day) = d.parts();
    m == 6 || m == 7 || (m == 8 && day < 15)
}

/// When a team's practice starts on `d`, minutes after its local midnight:
/// your own hour at your seat, the staff's otherwise.
pub fn practice_start(w: &World, tier: Tier, t: usize, d: Date) -> u16 {
    let level = match tier {
        Tier::High => Level::HighSchool,
        Tier::College => Level::College,
        Tier::Pro => Level::Nfl,
    };
    let at = match w.training.today_at {
        Some((day, at)) if day == d => at,
        _ => w.training.practice_at,
    };
    if w.seat_team() == Some((level, t))
        && let Some(at) = at
    {
        // An hour set in the summer moves inside the school year's rules.
        let (from, to) = start_limits(tier, d);
        return at.clamp(from, to);
    }
    staff_start(w, tier, t, d)
}

/// Your staff's practice start on `d` (local minutes).
pub fn staff_start(w: &World, tier: Tier, t: usize, d: Date) -> u16 {
    match tier {
        Tier::High if summer(d) => 7 * 60 + 30,
        Tier::High => 15 * 60 + 30,
        Tier::College if morning_program(w, t) => 8 * 60 + 45,
        Tier::College => 15 * 60 + 45,
        Tier::Pro if d < w.dates.cutdown.date => 9 * 60 + 15,
        Tier::Pro => 11 * 60 + 15,
    }
}

/// The hours you may set your practice to start (local minutes, from and
/// to): high school after the last bell in the school year, college inside
/// the players' free night hours, the NFL as the clubs do.
pub fn start_limits(tier: Tier, d: Date) -> (u16, u16) {
    let end_by = |last_end: u16| last_end - practice_length(tier);
    match tier {
        Tier::High if summer(d) => (6 * 60, end_by(21 * 60)),
        Tier::High => (15 * 60, end_by(21 * 60)),
        Tier::College => (6 * 60, end_by(21 * 60)),
        Tier::Pro => (8 * 60, 15 * 60),
    }
}

/// When a college or high school team's practice on `d` ends, Eastern.
pub(crate) fn practice_end(w: &World, tier: Tier, t: usize, d: Date) -> Moment {
    let level = if tier == Tier::High { Level::HighSchool } else { Level::College };
    let local = practice_start(w, tier, t, d) + practice_length(tier);
    w.team_zone(level, t).to_eastern(Moment::at(d, local / 60, local % 60))
}

/// When your team's practice on `d` ends, Eastern (5 AM without a seat).
pub(crate) fn seat_practice_end(w: &World, d: Date) -> Moment {
    let Some((level, t)) = w.seat_team() else { return zone::moment(d, 0) };
    let tier = match level {
        Level::HighSchool => Tier::High,
        Level::Nfl => Tier::Pro,
        _ => Tier::College,
    };
    let local = practice_start(w, tier, t, d) + practice_length(tier);
    w.team_zone(level, t).to_eastern(Moment::at(d, local / 60, local % 60))
}

/// The clubs practicing on `d` and when each practice ends.
fn nfl_sessions(w: &World, d: Date) -> Vec<(usize, u16, f64)> {
    if w.nfl.is_empty() || w.clubs.is_empty() {
        return Vec::new();
    }
    franchise::active_teams_in(w, w.year)
        .into_iter()
        .filter_map(|t| {
            let share = nfl_practice(w, t, d)?;
            let local = practice_start(w, Tier::Pro, t, d) + practice_length(Tier::Pro);
            Some((t, ends(club_zone(w, t), d, local), share))
        })
        .collect()
}

/// The injuries rolled with the week's plan whose practice ends on world
/// day `d`: their day minutes.
fn pending_minutes(w: &World, d: Date) -> impl Iterator<Item = u16> + '_ {
    let t = &w.training;
    t.pending
        .iter()
        .map(|x| x.0)
        .chain(t.others_pending.iter().map(|x| x.0))
        .filter_map(move |at| zone::day_minute(d, at))
}

/// The day's practice steps: each distinct minute a team's practice ends.
pub(crate) fn practice_steps(w: &World, d: Date) -> Vec<u16> {
    let mut v: Vec<u16> = nfl_sessions(w, d).into_iter().map(|x| x.1).collect();
    v.extend(pending_minutes(w, d));
    v.sort_unstable();
    v.dedup();
    v
}

/// The practices ending at day minute `t`: their injuries.
pub(crate) fn practice_at(w: &mut World, d: Date, t: u16) {
    let now = zone::moment(d, t);
    // College and high school: the week's rolls come out (and any left
    // behind, from a save made before the six-part day).
    let due = |at: Moment| at <= now;
    let (mine, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut w.training.pending).into_iter().partition(|x| due(x.0));
    w.training.pending = keep;
    for (_, hurt, note) in mine {
        let (from, id) = (hurt.from, hurt.id);
        // Already out (hurt in a game since the roll): the longer layoff
        // stands, and the trainers write only when practice adds time.
        match w.training.hurt.iter_mut().find(|h| h.id == id && h.until > from) {
            Some(h) if hurt.until > h.until => {
                *h = hurt;
                w.training.note(from, id, "trainers", note);
            }
            Some(_) => {}
            None => {
                w.training.hurt.push(hurt);
                w.training.note(from, id, "trainers", note);
            }
        }
    }
    let (others, keep): (Vec<_>, Vec<_>) =
        std::mem::take(&mut w.training.others_pending).into_iter().partition(|x| due(x.0));
    w.training.others_pending = keep;
    for (_, id, back, kind) in others {
        let e = w.training.others_hurt.entry(id).or_insert((back, Some(kind)));
        if back > e.0 {
            *e = (back, Some(kind));
        }
    }
    let sessions = nfl_sessions(w, d);
    let mut by_share: Vec<(f64, Vec<usize>)> = Vec::new();
    for (club, _, share) in sessions.into_iter().filter(|x| x.1 == t) {
        match by_share.iter_mut().find(|x| x.0 == share) {
            Some(x) => x.1.push(club),
            None => by_share.push((share, vec![club])),
        }
    }
    for (share, clubs) in by_share {
        crate::world::clock::timed("pro practice", || crate::world::pro::games::practice_injuries(w, d, &clubs, share));
    }
}
