//! Moving players through the world: high school transfers and powerhouse
//! pulls (topic 2), exposure and recruiting (topics 4 and 5), JUCO and prep
//! school (topic 6, 1.6), the draft, the transfer portal, tampering and NIL
//! (topic 8).

use rayon::prelude::*;

use super::background::Decider;
use super::data::{DIVISION_COUNT, Division};
use super::front_office as fo;
use super::news::count;
use super::program::{GradeCat, Program, record_impact};
use super::season::{key_normal, key_uniform};
use super::{Alumnus, CommitTiming, Person, Stage, World, division_level, geo};
use crate::calendar::{Date, Event, Kind, Level as CalLevel, Loudness, Moment};
use crate::career::{PIPELINE, Step};
use crate::player::{Level, PlayingTime};
use crate::positions::{POSITION_COUNT, Position};
use crate::rng::{Rng, clamp};
use crate::scouting::{ScoutOptions, scout_player};

// --- High school transfers ----------------------------------------------------------

/// Powerhouses pull standouts (topic 2): exposure, coaching, a better shot at
/// a big offer, sometimes a family move or an NIL perk. Loyalty, family ties,
/// cost, crowded depth charts and a strong local coach hold them back. A kid
/// may also move to escape a bad home (rare). Each state's transfer rule
/// decides whether he sits.
pub(crate) fn high_school_transfers(w: &mut World) {
    let year = w.year;
    w.rebuild_rosters();
    // Standouts by state: rising sophomores to seniors in the top 3% of their age.
    let mut by_state: Vec<Vec<usize>> = vec![Vec::new(); 51];
    // Found on all cores, then listed in person order (world speed lane).
    let found: Vec<(usize, usize)> = w
        .people
        .par_iter()
        .enumerate()
        .with_min_len(4096)
        .filter(|(_, p)| p.stage == Stage::HighSchool && (15..=17).contains(&p.school_age()))
        .map(|(i, p)| (p.home_state, i))
        .collect();
    for (s, i) in found {
        by_state[s].push(i);
    }
    let mut moves: Vec<(usize, usize, bool, bool)> = Vec::new();
    // The private powers' hunts this year (SK67): player id to school.
    let hunted: std::collections::HashMap<u32, usize> = w
        .hunt
        .ai
        .iter()
        .filter(|h| h.level == super::hunt::ai::SCHOOL && h.open())
        .map(|h| (h.id, h.team as usize))
        .collect();
    // The boarding academies fill their own classes first (bug 126).
    let mut taken: std::collections::HashSet<usize> = std::collections::HashSet::new();
    boarding_intake(w, &by_state, &hunted, &mut taken, &mut moves, 15..=17);
    for s in 0..51 {
        let list = &by_state[s];
        if list.is_empty() {
            continue;
        }
        let mut ovr: Vec<f64> = list.iter().map(|&i| w.people[i].player.overall()).collect();
        ovr.sort_by(|a, b| b.total_cmp(a));
        let bar = ovr[(ovr.len() as f64 * 0.03) as usize];
        // A boarding academy picks its own players; it isn't a powerhouse a family moves to.
        let powerhouses: Vec<usize> = w.states[s]
            .schools
            .iter()
            .copied()
            .filter(|&h| w.high_schools[h].is_powerhouse() && !boarding(&w.high_schools[h]))
            .collect();
        let nil_state = w.states[s].hs_nil;
        for &i in list {
            if taken.contains(&i) {
                continue;
            }
            let p = &w.people[i];
            let cur = &w.high_schools[p.hs];
            let standout = p.player.overall() >= bar;
            // Escaping a bad home: a rare pull that isn't about football.
            let escape = p.bg.safety < 0.3 && p.bg.home_stability < 0.35;
            if !standout && !escape {
                continue;
            }
            // Out of state, only the national powerhouses pull, and the family moves with him.
            let national = standout && p.player.overall() >= ovr[0] - 2.0 && w.rng.chance(0.15);
            let target = if national {
                let nat: Vec<usize> = w
                    .high_schools
                    .iter()
                    .filter(|h| h.national && h.state != s && !boarding(h))
                    .map(|h| h.id)
                    .collect();
                if nat.is_empty() { None } else { Some(nat[w.rng.int(0, nat.len() as i32 - 1) as usize]) }
            } else if !powerhouses.is_empty() {
                Some(powerhouses[w.rng.int(0, powerhouses.len() as i32 - 1) as usize])
            } else {
                None
            };
            // A private power hunting him goes after him itself (SK67).
            let hunter = hunted.get(&p.player.id).copied().filter(|&h| h != p.hs && !boarding(&w.high_schools[h]));
            let Some(t) = hunter.or(target) else { continue };
            if t == p.hs || cur.is_powerhouse() && !escape {
                continue;
            }
            let dest = &w.high_schools[t];
            // A hunter from another state is a move out of state too.
            let out_of_state = dest.state != s;
            let gain = (dest.prestige - cur.prestige).max(0.0);
            let mut pull = 0.3 * gain;
            // C8: a famous coach's name draws transfers (a star's name half
            // again as many; proposed, not confirmed).
            pull *= 1.0 + dest.coach.map_or(0.0, |c| w.coaches[c].name_pull(w.dates.season) / 6.0);
            if nil_state || (out_of_state && w.states[dest.state].hs_nil) {
                pull *= 1.4;
            }
            // A strong local coach keeps his stars often.
            if let Some(c) = cur.coach {
                let coach = &w.coaches[c];
                if coach.ability().relationships >= 13.0 && coach.reputation >= 11.0 {
                    pull *= 0.35;
                }
            }
            pull *= 1.0 - 0.45 * p.bg.home_stability;
            if dest.private && !out_of_state {
                pull *= 0.45 + 0.55 * p.bg.money;
            }
            // Crowded depth chart at his position.
            let ahead = dest
                .roster
                .iter()
                .filter(|&&j| {
                    w.people[j].player.position == p.player.position
                        && w.people[j].player.overall() > p.player.overall()
                })
                .count();
            if ahead >= crate::league::depth_slots(p.player.position).starters {
                pull *= 0.5;
            }
            pull *= 1.0 + (p.player.personality.competitiveness - 10.0) * 0.04;
            // A year of the hunter's courting (proposed, not confirmed: 1.8 times).
            if hunter == Some(t) {
                pull *= 1.8;
            }
            if escape {
                pull += 0.015;
            }
            if w.rng.chance(clamp(pull, 0.0, 0.4)) {
                // A move out of state moves the family, and he enrolls fresh.
                let family_moves = out_of_state || escape && w.rng.chance(0.5);
                moves.push((i, t, family_moves, out_of_state));
            }
        }
    }
    // A kid moved today settles in before anything else moves him.
    let moved: std::collections::HashSet<usize> = moves.iter().map(|m| m.0).collect();
    for (i, t, family_moved, national) in moves {
        let from = w.people[i].hs;
        let from_name = w.school_name(from).to_string();
        let to_name = w.school_name(t).to_string();
        let st = w.high_schools[t].state;
        let rule = w.states[st].transfer;
        let boards = boarding(&w.high_schools[t]);
        let p = &mut w.people[i];
        let first = p.transfers == 0;
        // An out-of-state boarder is a new enrollee, not a transfer inside the state.
        p.sit_out = if family_moved || (national && boards) || (rule.free_once && first) { 0.0 } else { rule.sit_out };
        if boards {
            let age = p.player.age;
            p.note(format!("At {age} he left home to board at {to_name}."));
        }
        p.transfers += 1;
        p.hs = t;
        if family_moved {
            p.home_state = st;
        }
        let age = p.player.age;
        let sit = if p.sit_out > 0.0 {
            format!(" and had to sit out part of the season under {}'s transfer rule", w.states[st].code)
        } else {
            String::new()
        };
        let why = if p.bg.safety < 0.3 && p.bg.home_stability < 0.35 {
            " to get away from a rough home situation"
        } else {
            ""
        };
        p.note(format!("At {age} he transferred from {from_name} to powerhouse {to_name}{why}{sit}."));
    }
    // Boarding away from home costs homesick kids (topic 4): one who wants
    // to be near home may go back after a year (proposed, not confirmed: up
    // to 30% a year for the most homesick).
    // The boarders found on all cores: each one's own move changes no one
    // else's place on the list (world speed lane).
    let boarders: Vec<usize> = (0..w.people.len())
        .into_par_iter()
        .with_min_len(4096)
        .filter(|&i| {
            let p = &w.people[i];
            !(p.stage != Stage::HighSchool
                || p.school_age() > 17
                || !boarding(&w.high_schools[p.hs])
                || moved.contains(&i))
        })
        .collect();
    for i in boarders {
        let p = &w.people[i];
        let homesick = clamp((p.family.own_wants.close_to_home - 0.12) * 2.5, 0.0, 0.3);
        if p.home_state == w.high_schools[p.hs].state || !w.rng.chance(homesick) {
            continue;
        }
        let home = p.home_state;
        let options: Vec<usize> = w.states[home].schools.clone();
        if options.is_empty() {
            continue;
        }
        let t = options[w.rng.int(0, options.len() as i32 - 1) as usize];
        let from_name = w.school_name(p.hs).to_string();
        let to_name = w.school_name(t).to_string();
        let p = &mut w.people[i];
        p.hs = t;
        p.sit_out = 0.0;
        let age = p.player.age;
        p.note(format!("Homesick at {age}, he left {from_name} and came home to play for {to_name}."));
    }
    // Crowded powerhouse depth charts push some kids out again for playing time.
    // The backups at powerhouses found on all cores, in person order; the
    // world's stream is drawn for each in turn as before (world speed lane).
    let buried: Vec<usize> = (0..w.people.len())
        .into_par_iter()
        .with_min_len(4096)
        .filter(|&i| {
            let p = &w.people[i];
            !(p.stage != Stage::HighSchool
                || !(16..=17).contains(&p.player.age)
                || p.last_role != Some(PlayingTime::Backup)
                || moved.contains(&i))
                && w.high_schools[p.hs].is_powerhouse()
        })
        .collect();
    for i in buried {
        let p = &w.people[i];
        let cur = &w.high_schools[p.hs];
        if !w.rng.chance(0.12) {
            continue;
        }
        // A boarder buried at the academy goes home to play.
        let state = if boarding(cur) { p.home_state } else { cur.state };
        let options: Vec<usize> = w.states[state]
            .schools
            .iter()
            .copied()
            .filter(|&h| !w.high_schools[h].is_powerhouse() && w.high_schools[h].prestige > 0.35)
            .collect();
        if options.is_empty() {
            continue;
        }
        let t = options[w.rng.int(0, options.len() as i32 - 1) as usize];
        let from_name = w.school_name(p.hs).to_string();
        let to_name = w.school_name(t).to_string();
        let rule = w.states[state].transfer;
        let p = &mut w.people[i];
        p.sit_out = if rule.free_once && p.transfers == 0 { 0.0 } else { rule.sit_out };
        p.transfers += 1;
        p.hs = t;
        let age = p.player.age;
        p.note(format!("At {age}, buried on {from_name}'s depth chart, he transferred to {to_name} to play."));
    }
    // Reclassifying: a few repeat a year or finish early to time their
    // college entry. Each on his own stream, so on all cores.
    w.people.par_iter_mut().with_min_len(4096).for_each(|p| {
        if p.stage != Stage::HighSchool || p.school_age() != 16 {
            return;
        }
        let r = p.rng.next();
        if r < 0.004 && p.player.overall() > 52.0 {
            p.class_of -= 1;
            p.note("Reclassified to graduate a year early.");
        } else if r > 0.995 && p.years_off_class() <= 0 {
            // Never twice: a boy already a year old for his class stays put.
            p.class_of += 1;
            p.note("Reclassified to repeat a year of high school.");
        }
    });
    let _ = year;
}

/// Places a boarding academy fills in each class, and the miles at which a
/// parent who wants his son near home counts the distance in full (bug 126;
/// both proposed, not confirmed). IMG's football program holds about 100
/// high schoolers over four grades: freshmen on class day
/// (`boarding_freshmen`), the older grades' open places each June.
const BOARDING_CLASS: usize = 25;
const BOARDING_MILES: f64 = 800.0;

/// A boarding academy's own recruiting (bug 126): it has no neighbourhood,
/// so its staff read the standouts of every state (the top 3% of each age,
/// not already at a powerhouse) and offer their open places best first,
/// each with aid. The family decides on the usual pulls; family money still
/// counts, and so does the distance for a parent sending a 15-year-old away.
/// Keyed draws only, so the world's stream doesn't move.
fn boarding_intake(
    w: &World,
    by_state: &[Vec<usize>],
    hunted: &std::collections::HashMap<u32, usize>,
    taken: &mut std::collections::HashSet<usize>,
    moves: &mut Vec<(usize, usize, bool, bool)>,
    ages: std::ops::RangeInclusive<u32>,
) {
    let academies: Vec<usize> = w.high_schools.iter().filter(|h| boarding(h)).map(|h| h.id).collect();
    if academies.is_empty() {
        return;
    }
    let seed = w.options.seed as u64 ^ 0xB0A2_0000 ^ ((w.year as u64) << 32);
    // The board by school age: every state's standouts of that age.
    // Each state's standouts found on all cores, then boarded in state
    // order (world speed lane).
    let per_state: Vec<Vec<Vec<usize>>> = by_state
        .par_iter()
        .map(|list| {
            ages.clone()
                .map(|age| {
                    let mut ovr: Vec<f64> = list
                        .iter()
                        .filter(|&&i| w.people[i].school_age() == age)
                        .map(|&i| w.people[i].player.overall())
                        .collect();
                    if ovr.is_empty() {
                        return Vec::new();
                    }
                    ovr.sort_by(|a, b| b.total_cmp(a));
                    let bar = ovr[(ovr.len() as f64 * 0.03) as usize];
                    list.iter()
                        .copied()
                        .filter(|&i| {
                            let p = &w.people[i];
                            let cur = &w.high_schools[p.hs];
                            p.school_age() == age && p.player.overall() >= bar && !cur.is_powerhouse() && !boarding(cur)
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    let mut board: Vec<Vec<usize>> = vec![Vec::new(); ages.clone().count()];
    for lists in per_state {
        for (k, v) in lists.into_iter().enumerate() {
            board[k].extend(v);
        }
    }
    for &t in &academies {
        let dest = &w.high_schools[t];
        let staff = seed ^ t as u64;
        for (k, age) in ages.clone().enumerate() {
            let have = dest.roster.iter().filter(|&&j| w.people[j].school_age() == age).count();
            let mut open = BOARDING_CLASS.saturating_sub(have);
            if open == 0 {
                continue;
            }
            // The academy's staff read, with the hunt's noise.
            let mut ranked: Vec<(usize, f64)> = board[k]
                .iter()
                .filter(|i| !taken.contains(i))
                .map(|&i| {
                    let p = &w.people[i];
                    (i, p.player.overall() + 2.0 * key_normal(staff, p.player.id as u64))
                })
                .collect();
            ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            for (i, _) in ranked {
                if open == 0 {
                    break;
                }
                let p = &w.people[i];
                let cur = &w.high_schools[p.hs];
                let gain = (dest.prestige - cur.prestige).max(0.0);
                let mut pull = 0.3 * gain;
                pull *= 1.0 + dest.coach.map_or(0.0, |c| w.coaches[c].name_pull(w.dates.season) / 6.0);
                if w.states[p.home_state].hs_nil || w.states[dest.state].hs_nil {
                    pull *= 1.4;
                }
                // Every offer carries aid, but boarding still costs a family.
                pull *= 0.5 + 0.5 * p.bg.money;
                let miles = geo::miles(p.home_state, dest.state);
                // Whoever decides for a minor weighs the distance.
                pull *= clamp(1.0 - p.family.decider_wants.close_to_home * miles / BOARDING_MILES, 0.3, 1.0);
                if let Some(c) = cur.coach {
                    let coach = &w.coaches[c];
                    if coach.ability().relationships >= 13.0 && coach.reputation >= 11.0 {
                        pull *= 0.35;
                    }
                }
                pull *= 1.0 - 0.45 * p.bg.home_stability;
                let ahead = dest
                    .roster
                    .iter()
                    .filter(|&&j| {
                        w.people[j].player.position == p.player.position
                            && w.people[j].player.overall() > p.player.overall()
                    })
                    .count();
                if ahead >= crate::league::depth_slots(p.player.position).starters {
                    pull *= 0.5;
                }
                pull *= 1.0 + (p.player.personality.competitiveness - 10.0) * 0.04;
                if hunted.get(&p.player.id) == Some(&t) {
                    pull *= 1.8;
                }
                if key_uniform(staff ^ 1, p.player.id as u64) < clamp(pull, 0.0, 0.4) {
                    moves.push((i, t, false, true));
                    taken.insert(i);
                    open -= 1;
                }
            }
        }
    }
}

/// Class day: the boarding academies take their freshmen from the new
/// class, as they take the older grades in June (bug 126).
pub(crate) fn boarding_freshmen(w: &mut World) {
    let mut by_state: Vec<Vec<usize>> = vec![Vec::new(); 51];
    for (i, p) in w.people.iter().enumerate() {
        if p.stage == Stage::HighSchool && p.school_age() == 14 {
            by_state[p.home_state].push(i);
        }
    }
    let mut taken = std::collections::HashSet::new();
    let mut moves = Vec::new();
    boarding_intake(w, &by_state, &std::collections::HashMap::new(), &mut taken, &mut moves, 14..=14);
    for (i, t, _, _) in moves {
        let to_name = w.school_name(t).to_string();
        let p = &mut w.people[i];
        p.hs = t;
        p.sit_out = 0.0;
        let age = p.player.age;
        p.note(format!("At {age} he left home to board at {to_name}."));
    }
}

/// A boarding academy (IMG Academy): students come from anywhere and live
/// on campus; tuition is about $90k a year.
pub(crate) fn boarding(h: &super::high_school::HighSchool) -> bool {
    h.national && h.name.starts_with("IMG Academy")
}

// --- College exits: awards, the draft ------------------------------------------------

/// What a veteran at a position costs, as a share of the salary cap: the
/// average of the five best-paid (OverTheCap via nflverse, contracts signed
/// 2022 and later, `engine_targets.json` `contracts_cap_pct`). Tackles are
/// the mean of left and right; the interior line of guards and centers.
fn veteran_top5(pos: Position) -> f64 {
    match pos {
        Position::QB => 0.234,
        Position::EDGE => 0.158,
        Position::WR => 0.136,
        Position::DT => 0.117,
        Position::CB => 0.108,
        Position::OT => 0.0985,
        Position::S => 0.081,
        Position::IOL => 0.077,
        Position::LB => 0.075,
        Position::RB => 0.074,
        Position::TE => 0.063,
        Position::K => 0.023,
        Position::P => 0.015,
        Position::LS => 0.007,
    }
}

/// The average veteran price over the eleven non-specialist positions.
const VETERAN_MEAN: f64 = 0.111;

/// The league minimum, as a share of the cap.
const VETERAN_MIN: f64 = 0.004;

/// What a prospect would cost on the open market once he is what his grade
/// says, as a share of the cap: the minimum for a backup, up to the top-five
/// price for an elite one (proposed, not confirmed: the curve's bend).
fn market_value(top5: f64, grade: f64) -> f64 {
    VETERAN_MIN + (top5 - VETERAN_MIN) * clamp((grade - 66.0) / 24.0, 0.0, 1.0).powf(1.6)
}

/// What clubs pay for a position on draft day, on the same scale as
/// [`veteran_top5`]: the veteran price, moved by how real clubs spend first
/// round picks (2000-2025, `draft_positions_2000_2025.csv`). Tackles and
/// corners go in round one well beyond their veteran pay (tackles 12.1% of
/// first-round picks, corners 11.9%), receivers and edge rushers less than
/// theirs (13.4% and 16.8%): a class holds many receivers, and the long,
/// athletic tackles and corners a club can't find later are few. Trial 9
/// finding 8: priced on veteran pay alone, round one took receivers 22.3%,
/// edge rushers 20.5%, tackles 6.1% and corners 7.7% (proposed, not
/// confirmed: each step sized to the gap in grade points).
fn draft_top5(pos: Position) -> f64 {
    match pos {
        Position::QB => 0.234,
        Position::EDGE => 0.148,
        Position::OT => 0.135,
        Position::CB => 0.131,
        Position::DT => 0.112,
        Position::WR => 0.110,
        Position::TE => 0.090,
        Position::RB => 0.088,
        Position::LB => 0.080,
        Position::S => 0.075,
        Position::IOL => 0.075,
        Position::K => 0.023,
        Position::P => 0.015,
        Position::LS => 0.007,
    }
}

/// Board points per share of the cap a year (sized so the value term is
/// about the old flat premiums at the top of the board: a quarterback
/// graded 85 about 12 points, an edge graded 90 about 7).
const BOARD_PER_CAP: f64 = 140.0;

/// What a position is worth on draft day, in board points (trial 4 topic 6):
/// each team values a prospect by what his position is worth on draft day
/// ([`draft_top5`]) against a player of the same grade at an average
/// position. A
/// quarterback's market is twice an edge rusher's, so a quarterback graded
/// a few points lower still goes first; a back's or a tight end's is below
/// average, so they slide. The rookie deal is the same at a pick whatever
/// the position, so what a team saves is this value too (it shows in the
/// pick's reason).
fn draft_position_value(pos: Position, grade: f64) -> f64 {
    BOARD_PER_CAP * (market_value(draft_top5(pos), grade) - market_value(VETERAN_MEAN, grade))
}

/// What a pick costs a year on the 2026 rookie scale, as a share of the
/// cap (nflverse contracts, `rookie_scale_2026.csv`; the 2025 cap of
/// $279.2 million): a straight line through the real slots, about 5.1% for
/// the first pick, 1.5% at the end of round one, 0.4% in round seven.
fn rookie_cap_share(pick: u32) -> f64 {
    let p = pick as f64;
    if p <= 32.0 {
        0.0513 - (0.0513 - 0.0150) * (p - 1.0) / 31.0
    } else if p <= 64.0 {
        0.0120 - (0.0120 - 0.0058) * (p - 33.0) / 31.0
    } else {
        (0.0058 - 0.0018 * (p - 64.0) / 193.0).max(0.004)
    }
}

/// A flat discount on kickers, punters and long snappers in the December
/// read, on top of the position value (which shrinks in later rounds), so
/// the public's boards and a specialist's own round read put them where the
/// draft takes them (proposed, not confirmed). On draft day the board's
/// own scale places them instead (`PositionAligner::for_draft`).
fn specialist_draft_discount(pos: Position) -> f64 {
    match pos {
        Position::K | Position::P => 2.0,
        Position::LS => 5.0,
        _ => 0.0,
    }
}

fn competition_discount(d: Division) -> f64 {
    match d {
        Division::FBS => 0.0,
        Division::FCS => 1.0,
        Division::D2 | Division::JUCO => 5.0,
        Division::NAIA => 6.0,
        Division::D3 => 7.0,
    }
}

/// The first-team All-America by position.
const ALL_AMERICA: [(Position, usize); 13] = [
    (Position::QB, 1),
    (Position::RB, 2),
    (Position::WR, 3),
    (Position::TE, 1),
    (Position::OT, 2),
    (Position::IOL, 3),
    (Position::EDGE, 2),
    (Position::DT, 2),
    (Position::LB, 3),
    (Position::CB, 2),
    (Position::S, 2),
    (Position::K, 1),
    (Position::P, 1),
];

/// Credits an impact player to his program's pipelines and to the staff who
/// coached him: their ties to his high school travel with them (note 4).
fn credit_impact(w: &mut World, i: usize, c: usize, weight: f64, year: i32) {
    // Pipelines and staff ties grow in the spin-up too (minor bug 131); only
    // the coaches' visible record waits for recorded history.
    let (state, hs) = (w.people[i].home_state, w.people[i].hs);
    let area = w.high_schools[hs].area;
    record_impact(&mut w.programs[c], state, hs, area, weight, year);
    let group = super::coach::Group::of(w.people[i].player.position);
    let prog = &w.programs[c];
    let coord = if group.offense() { prog.oc } else { prog.dc };
    let staff = [prog.head_coach, coord, prog.position_coaches[group as usize]];
    for coach in staff.into_iter().flatten() {
        let t = w.coaches[coach].ties.entry(hs).or_insert(0.0);
        *t = clamp(*t + 0.2 * weight * (1.0 - *t), 0.0, 1.0);
    }
    // His position coach and coordinator get the visible credit (note 15B).
    if !w.recording {
        return;
    }
    for (coach, share) in [(prog.position_coaches[group as usize], 1.0), (coord, 0.5)] {
        if let Some(coach) = coach {
            w.coaches[coach].units[group as usize].moved_up += share * weight;
        }
    }
}

/// All-conference teams and national honors, the day the All-America teams
/// come out (A25). Only impact grows a pipeline: all-conference or better,
/// any draft pick, an undrafted pro signing (topic 4, note 4).
pub(crate) fn honors(w: &mut World) {
    let year = w.year;
    let mut honors: Vec<(usize, f64, &'static str)> = Vec::new();
    let mut all_america: Vec<(usize, Division)> = Vec::new();
    for division in [Division::FBS, Division::FCS, Division::D2, Division::NAIA, Division::D3] {
        let mut perf: Vec<(f64, usize)> = w
            .people
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                p.stage == Stage::College
                    && p.last_role == Some(PlayingTime::Starter)
                    && p.college.is_some_and(|c| w.programs[c].division == division)
            })
            .filter_map(|(i, p)| p.last_performance.map(|v| (v, i)))
            .collect();
        perf.sort_by(|a, b| b.0.total_cmp(&a.0));
        // The first-team All-America by position, as the real teams are
        // picked (25 a level: a quarterback, two backs, three receivers, a
        // tight end, five linemen, four up front, three linebackers, four
        // backs, a kicker and a punter; proposed, not confirmed).
        let mut first_team: Vec<usize> = Vec::new();
        for (pos, n) in ALL_AMERICA {
            first_team.extend(perf.iter().filter(|x| w.people[x.1].player.position == pos).take(n).map(|x| x.1));
        }
        for &i in &first_team {
            honors.push((i, 1.0, "was a first-team All-American"));
            all_america.push((i, division));
        }
        // The second and third teams and honorable mentions: the rest of
        // the level's best two percent.
        let leaders = (perf.len() as f64 * 0.02).ceil() as usize;
        for &(_, i) in perf.iter().take(leaders) {
            if !first_team.contains(&i) {
                honors.push((i, 0.75, "was among his level's top performers"));
            }
        }
        // All-conference: about the top tenth of the conference's starters,
        // judged against the conference, not the nation.
        let mut by_conf: std::collections::BTreeMap<(Division, &str), Vec<(f64, usize)>> =
            std::collections::BTreeMap::new();
        for &(v, i) in &perf {
            let c = w.people[i].college.unwrap();
            by_conf.entry((w.programs[c].division, w.programs[c].conference.as_str())).or_default().push((v, i));
        }
        for list in by_conf.values() {
            let n = (list.len() as f64 * 0.1).ceil() as usize;
            for &(_, i) in list.iter().take(n) {
                if !honors.iter().any(|h| h.0 == i) {
                    honors.push((i, 0.5, "was named all-conference"));
                }
            }
        }
    }
    // Regular signees who stayed and played grow a pipeline too (fix list
    // 2.7): high school coaches send players where earlier ones were
    // treated well. Proposed, not confirmed: a starter counts 0.15 a
    // season, a rotation player 0.07 (an all-conference season is 0.5).
    // The spin-up grows them too (minor bug 131).
    let honored: std::collections::HashSet<usize> = honors.iter().map(|h| h.0).collect();
    let regulars: Vec<(usize, usize, f64)> = w
        .people
        .iter()
        .enumerate()
        .filter(|(i, p)| p.stage == Stage::College && p.transfers == 0 && !p.juco && !honored.contains(i))
        .filter_map(|(i, p)| {
            let weight = match p.last_role {
                Some(PlayingTime::Starter) => 0.15,
                Some(PlayingTime::Rotation) => 0.07,
                _ => return None,
            };
            Some((i, p.college?, weight))
        })
        .collect();
    for (i, c, weight) in regulars {
        let (state, hs) = (w.people[i].home_state, w.people[i].hs);
        let area = w.high_schools[hs].area;
        record_impact(&mut w.programs[c], state, hs, area, weight, year);
    }
    let n_all_america = honors.iter().filter(|h| h.1 >= 1.0).count();
    if w.recording {
        for &(i, d) in &all_america {
            super::records::honor(w, i, year, Some(d), None);
        }
        for &(i, weight, _) in &honors {
            if weight == 0.5 {
                super::records::honor(w, i, year, None, None);
            }
        }
    }
    for (i, weight, what) in honors {
        let c = w.people[i].college.unwrap();
        credit_impact(w, i, c, weight, year);
        let age = w.people[i].row_age();
        let school = w.program_name(c).to_string();
        w.people[i].note(format!("At {age} he {what} at {school}."));
    }
    // A world's first season has no rows to judge yet (part D 29): no teams named.
    if n_all_america == 0 {
        return;
    }
    let text = format!("All-America teams are named: {n_all_america} players honored nationally.");
    w.calendar.post(Event::new(w.today, CalLevel::College, Kind::Award, Loudness::Medium, text));
}

/// When a player has used up his college football after the season before
/// `draft_year`: his seasons (four, five at Division I from 2026-27) or his
/// five-year clock, with its stops and extensions (`eligibility.rs`).
fn exhausted(p: &Person, programs: &[Program], draft_year: i32) -> bool {
    let d = super::eligibility::division_of(p, programs);
    super::eligibility::exhausted_after(p, d, draft_year - 1)
}

/// December draft reads (T3.1, T3.2): NFL staffs and the advisory committee
/// read every draft-eligible player once the regular season ends. The read
/// drives bowl opt-outs and each underclassman's January choice, which he
/// makes now and acts on by the deadlines.
pub(crate) fn draft_reads(w: &mut World) {
    // The clubs' fall (section 7): school calls on the spring list (last
    // December's reads), then cross-checks, before the reads are redone.
    super::pro::scouts::fall(w);
    let year = w.year;
    let draft_year = year + 1;
    // The hidden-gem footprint (world item 9): this season's flags, as the
    // scouts' own stat services show them in December.
    let feet = super::records::current_footprints(w);
    let mut rows: Vec<(usize, f64, f64)> = Vec::new();
    for i in 0..w.people.len() {
        let p = &w.people[i];
        if p.stage != Stage::College {
            continue;
        }
        let Some(c) = p.college else { continue };
        let division = w.programs[c].division;
        if division == Division::JUCO || draft_year - p.class_of < 3 {
            continue;
        }
        let level = division_level(division);
        // The league's tape leans the same way for everyone (SK2, SK4).
        let tape_lean = {
            let f = super::scouting::footage::footage(w, i);
            super::scouting::footage::scout_tilt(&f, 12.0, false, false)
        };
        let p = &mut w.people[i];
        // Pro scouts list him where they see him playing on Sundays and
        // rate him there; he keeps playing his college position until he
        // leaves (trial 7 finding 12: a quarterback projected at end went
        // back to his team listed at end).
        let to = crate::athlete::better_position(&p.player, crate::athlete::Eye::NFL_SCOUTS, 0.005);
        p.cal.pro_position = to;
        let projected = to.map(|pos| {
            let mut q = p.player.clone();
            crate::athlete::set_position(&mut q, pos);
            q
        });
        // A small-school player whose numbers flag him (this season or
        // last) gets a second look: more scouts on his film, a truer read.
        // The numbers can lie (a weak schedule), so the film decides
        // (F4: the second look reads at accuracy 18 against everyone
        // else's 13; it was 8, a wider error; 18 proposed, not confirmed).
        let id = p.player.id;
        let flagged = division != Division::FBS
            && (feet.get(&id).is_some_and(|&f| f != 0) || !w.stats.footprint_since(id, year - 1).is_empty());
        let why = if flagged {
            let mut why: Vec<String> =
                w.stats.footprint_since(id, year - 1).iter().flat_map(|r| super::records::footprint_text(r)).collect();
            if let Some(&f) = feet.get(&id) {
                let probe = super::records::SeasonRecord {
                    id,
                    year: year as i16,
                    division,
                    pos: w.people[i].player.position,
                    program: c as u16,
                    games: 0,
                    footprint: f,
                    sos: 0.0,
                    s: Default::default(),
                };
                why.extend(super::records::footprint_text(&probe));
            }
            why
        } else {
            Vec::new()
        };
        let p = &mut w.people[i];
        if !why.is_empty() {
            let age = p.row_age();
            p.note(format!("At {age} pro scouts took a second look at his film: {}.", why.join("; ")));
        }
        let accuracy = if flagged { 18.0 } else { 13.0 };
        let seen = projected.as_ref().unwrap_or(&p.player);
        let rep = scout_player(seen, ScoutOptions { accuracy, level }, &mut p.rng);
        let youth = clamp((23.0 - p.player.age as f64) / 3.0, 0.0, 1.0);
        let flags: f64 = p
            .player
            .history
            .iter()
            .map(|s| {
                if s.events.has(crate::player::DevelopmentEvent::Dismissed) {
                    PIPELINE.red_flag_discount.dismissed
                } else if s.events.has(crate::player::DevelopmentEvent::Suspended) {
                    PIPELINE.red_flag_discount.suspended
                } else {
                    0.0
                }
            })
            .sum();
        let grade =
            rep.overall + (0.25 + 0.2 * youth) * (rep.potential - rep.overall) - competition_discount(division) - flags
                + tape_lean;
        // The board's own position values (part C 2): the December read
        // ranks a long snapper as the draft does, far down the board, so
        // combine invites go to real prospects. The round's weight follows
        // the grade.
        let pos = seen.position;
        let rf = if grade >= 78.0 {
            1.0
        } else if grade >= 72.0 {
            0.6
        } else {
            0.3
        };
        let score = grade + draft_position_value(pos, grade) * rf - specialist_draft_discount(pos);
        rows.push((i, grade, score));
    }
    let mut board: Vec<f64> = rows.iter().map(|r| r.2).collect();
    board.sort_by(|a, b| b.total_cmp(a));
    let slots = (PIPELINE.draft_picks * w.options.scale).round().max(1.0) as usize;
    for &(i, grade, score) in &rows {
        let rank = board.partition_point(|&v| v > score) + 1;
        let round = if rank <= slots { crate::career::draft_round(rank as u32, slots as u32) } else { 8 };
        let p = &mut w.people[i];
        p.cal.draft_read = Some(super::DraftRead { grade, score, round });
        p.cal.declare_plan = None;
        if exhausted(p, &w.programs, draft_year) {
            continue;
        }
        // His first lean; the advisory grade in mid-December decides it (B 37).
        let chance = declare_chance(p, round, draft_year);
        p.cal.declare_plan = Some(p.rng.chance(chance));
    }
}

/// An underclassman's chance to declare from his projected round (T3.1):
/// projected high picks go; the rest mostly return. Family money pulls a
/// mid-round prospect. About 80-150 underclassmen declare a year, and only
/// a handful of them with no draft grade (NFL special-eligibility lists,
/// 2019-2025; proposed, not confirmed).
fn declare_chance(p: &Person, round: u32, draft_year: i32) -> f64 {
    let chance = match round {
        1 | 2 => 0.85,
        3 | 4 => 0.35 + 0.02 * (p.family.decider_wants.money * 10.0),
        5..=7 => 0.08,
        _ => 0.0005,
    };
    // From 2021 staying is paid (part B 11): he weighs the NIL money he
    // would give up against a year of his round's rookie pay, so a
    // mid-round prospect earning well stays (real underclass entrants fell
    // to about 58 in 2024). Rookie pay a year by round, roughly the 2024
    // scale: about $4M in round 1, $1.6M in round 2, $1.2M in rounds 3-4,
    // $1M later (proposed, not confirmed).
    let rookie_year = match round {
        1 => 4.0e6,
        2 => 1.6e6,
        3 | 4 => 1.2e6,
        _ => 1.0e6,
    };
    let stay_pay = if super::ncaa::nil_era(draft_year - 1) { p.nil_income } else { 0.0 };
    // Section 6 topic 1: an ambitious player goes sooner (proposed, not
    // confirmed: up to a quarter more or less likely).
    let ambition = 1.0 + 0.25 * p.player.personality.traits.z(crate::player::Trait::Ambition);
    clamp(chance * clamp(1.0 - 0.6 * stay_pay / rookie_year, 0.25, 1.0) * ambition, 0.0, 0.97)
}

/// The NFL's College Advisory Committee returns its grades in mid-December
/// (part B 37), before the January deadline: first round, second round, or
/// "return to school", read with the committee's own error. The grade
/// decides each underclassman's plan; the read from the staffs was his
/// first lean.
pub(crate) fn advisory_grades(w: &mut World) {
    let draft_year = w.year + 1;
    let seed = w.options.seed as u64 ^ ((w.year as u64) << 16) ^ 0xDEC1;
    use super::scouting::own;
    // The committee began in 1994 (SK21, by era): before that a junior went
    // on his own and his agent's word.
    let committee = w.year >= 1994;
    // Who asks: any underclassman with a draftable read or a plan to go.
    let mut asking: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            let p = &w.people[i];
            p.stage == Stage::College
                && !exhausted(p, &w.programs, draft_year)
                && p.cal.draft_read.is_some_and(|r| r.round <= 7 || p.cal.declare_plan == Some(true))
        })
        .collect();
    // From 2014 five names a school, the coach choosing whom to send (an AI
    // staff sends its five best reads), more only by exception for a likely
    // first-rounder; yours are the names you sent.
    let seat_prog = super::scouting::readers::seat_staff(w)
        .and_then(|s| if let super::scouting::readers::Staff::Program(p) = s { Some(p) } else { None });
    let limit = own::advisory_limit(w.year);
    let mut sent = vec![false; asking.len()];
    if let Some(cap) = limit {
        let mine: std::collections::HashSet<u32> =
            w.scouting.own.advisory.iter().filter(|x| x.1 == w.year).map(|x| x.0).collect();
        let mut by_school: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
        for (k, &i) in asking.iter().enumerate() {
            if let Some(c) = w.people[i].college {
                by_school.entry(c).or_default().push(k);
            }
        }
        for (c, mut ks) in by_school {
            if Some(c) == seat_prog {
                for k in ks {
                    sent[k] = mine.contains(&w.people[asking[k]].player.id);
                }
                continue;
            }
            let score = |k: usize| w.people[asking[k]].cal.draft_read.map_or(0.0, |r| r.score);
            ks.sort_by(|&a, &b| score(b).total_cmp(&score(a)).then(a.cmp(&b)));
            for (n, &k) in ks.iter().enumerate() {
                let first = w.people[asking[k]].cal.draft_read.is_some_and(|r| r.round <= 1);
                sent[k] = n < cap || first;
            }
        }
    } else {
        sent.iter_mut().for_each(|x| *x = committee);
    }
    // Your stay-or-go talks, weighed on each plan (SK21).
    let talks: Vec<f64> = asking
        .iter()
        .map(|&i| {
            let p = &w.people[i];
            match own::talk_of(w, p.player.id) {
                Some(say) if p.college == seat_prog && seat_prog.is_some() => own::talk_factor(w, i, say),
                _ => 1.0,
            }
        })
        .collect();
    let three = w.year >= 2014;
    let mut graded = 0u32;
    for (k, i) in asking.drain(..).enumerate() {
        let p = &mut w.people[i];
        let Some(read) = p.cal.draft_read else { continue };
        let guess =
            (read.round as f64 + 1.5 * key_normal(seed ^ 0xAD, p.player.id as u64)).round().clamp(1.0, 8.0) as u32;
        let round = if !sent[k] {
            // No grade: his agent's word and the talk around him, looser.
            p.cal.advisory = None;
            (read.round as f64 + 2.4 * key_normal(seed ^ 0xAE, p.player.id as u64)).round().clamp(1.0, 8.0) as u32
        } else if three {
            // Three answers: first round, second round or return to school.
            p.cal.advisory = Some(guess.min(3) as u8);
            guess
        } else {
            p.cal.advisory = Some(guess as u8);
            guess
        };
        let chance = clamp(declare_chance(p, round, draft_year) * talks[k], 0.0, 0.97);
        p.cal.declare_plan = Some(p.rng.chance(chance));
        if !sent[k] {
            continue;
        }
        graded += 1;
        let age = p.row_age();
        let grade = match (p.cal.advisory.unwrap_or(8), three) {
            (1, _) => "a first-round grade".to_string(),
            (2, _) => "a second-round grade".to_string(),
            (_, true) => "a \"return to school\" grade".to_string(),
            (r @ 3..=7, false) => format!("a round {r} grade"),
            _ => "a \"not drafted\" grade".to_string(),
        };
        p.note(format!("At {age} the NFL's advisory committee gave him {grade}."));
    }
    if w.recording && graded > 0 {
        let text = format!("The NFL's College Advisory Committee returns its grades to {graded} underclassmen.");
        let d = w.today;
        w.calendar.post(Event::new(d, CalLevel::Nfl, Kind::DraftChoice, Loudness::Small, text));
    }
}

/// The draft decisions (T3.1): early entrants announce on their own days
/// from December (part D 17), the rest by the hard deadline, when seniors'
/// college careers end too; a few withdraw after the advisory read, by the
/// NCAA's date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum Declare {
    /// A day before the deadline: those whose own day it is.
    Soft,
    Deadline,
    Withdraw,
}

pub(crate) fn declarations(w: &mut World, phase: Declare) {
    let year = w.year;
    let draft_year = year + 1;
    let date = w.today;
    // Teams still playing: their players declare once their season is over.
    let playing: std::collections::HashSet<usize> = w.college.as_ref().map_or_else(Default::default, |c| {
        c.games.iter().filter(|g| g.played.is_none()).flat_map(|g| [g.home, g.away]).collect()
    });
    // Each team's last game played, for the days players announce after it.
    let mut last_game: std::collections::HashMap<usize, Date> = std::collections::HashMap::new();
    if let Some(c) = w.college.as_ref() {
        for g in c.games.iter().filter(|g| g.played.is_some()) {
            for t in [g.home, g.away] {
                let e = last_game.entry(t).or_insert(g.date);
                *e = (*e).max(g.date);
            }
        }
    }
    let deadline = w.dates.declare_deadline;
    let advisory = Date::ymd(year, 12, 16);
    let seed = w.options.seed as u64 ^ ((year as u64) << 16) ^ 0xDEC1;
    let mut events: Vec<(usize, String, Loudness, u32, String)> = Vec::new();
    // Who the walk can act on, found on all cores: each choice changes only
    // the player himself, so no one else's place on the list (world speed
    // lane).
    let cands: Vec<usize> = (0..w.people.len())
        .into_par_iter()
        .with_min_len(4096)
        .filter(|&i| {
            let p = &w.people[i];
            match phase {
                Declare::Soft | Declare::Deadline => {
                    p.stage == Stage::College && p.cal.portal.is_none() && p.college.is_some()
                }
                Declare::Withdraw => p.stage == Stage::Declared,
            }
        })
        .collect();
    for i in cands {
        let p = &w.people[i];
        match phase {
            Declare::Soft | Declare::Deadline => {
                if p.stage != Stage::College || p.cal.portal.is_some() {
                    continue;
                }
                let Some(c) = p.college else { continue };
                let Some(read) = p.cal.draft_read else {
                    // Junior college careers end by signing with a four-year school, or here.
                    if phase == Declare::Deadline
                        && w.programs[c].division == Division::JUCO
                        && draft_year - p.class_of >= 5
                    {
                        let p = &mut w.people[i];
                        p.stage = Stage::Done;
                        p.note("His junior college years ran out and his playing days ended.");
                    }
                    continue;
                };
                // A team still in the playoff declares after its last game
                // (T3: the title game's players get until about January 24).
                // A player who opted out is done with his season already.
                if phase == Declare::Deadline && playing.contains(&c) && !p.cal.opted_out {
                    continue;
                }
                let senior = exhausted(p, &w.programs, draft_year);
                // The 2020 COVID year (A3): a senior whose extra year is all
                // he has left decides by the deadline whether to use it.
                let division = super::eligibility::division_of(p, &w.programs);
                // Each decides once: on the deadline, or after his team's
                // last game when it played on past it.
                let first_pass = date == deadline || last_game.get(&c).is_some_and(|g| *g > deadline);
                let covid_choice = !senior
                    && phase == Declare::Deadline
                    && first_pass
                    && p.cal.declare_plan != Some(true)
                    && super::eligibility::on_covid_year(p, division, year);
                let declines = covid_choice
                    && key_uniform(seed ^ 0xC0_71D, p.player.id as u64) >= super::eligibility::covid_return_chance(p);
                if covid_choice && !declines {
                    w.log.covid_returns += 1;
                    let p = &mut w.people[i];
                    let age = p.row_age();
                    p.note(format!("At {age} he chose to come back for his extra COVID year."));
                    continue;
                }
                if senior || declines {
                    if phase == Declare::Deadline {
                        let p = &mut w.people[i];
                        p.stage = Stage::Declared;
                        let age = p.row_age();
                        if declines {
                            w.log.covid_declined += 1;
                            p.note(format!(
                                "At {age} he passed on his extra COVID year and entered the draft pool as his college career ended."
                            ));
                        } else {
                            p.note(format!("His college career over at {age}, he entered the draft pool."));
                        }
                    }
                    continue;
                }
                if p.cal.declare_plan != Some(true) {
                    continue;
                }
                // Each announces on his own day (part D 17): one who opted
                // out with his opt-out; the rest after their last game and
                // the advisory grades, the decisive ones sooner, all by the
                // deadline.
                if phase == Declare::Soft {
                    let own = if p.cal.opted_out {
                        p.cal.opt_out_on.unwrap_or(date)
                    } else if playing.contains(&c) {
                        continue;
                    } else {
                        let from = last_game.get(&c).map_or(advisory, |d| d.add(1)).max(advisory);
                        let avail = from.days_to(deadline).max(1) as f64;
                        let comp = p.player.personality.competitiveness;
                        let x = key_uniform(seed, p.player.id as u64).powf(clamp(1.5 + 0.05 * (comp - 10.0), 1.0, 2.0));
                        from.add((avail * x).floor() as i32)
                    };
                    if own > date {
                        continue;
                    }
                }
                let p = &mut w.people[i];
                p.stage = Stage::Declared;
                let age = p.row_age();
                let round = read.round;
                let proj =
                    if round <= 7 { format!("projected round {round}") } else { "projected to go undrafted".into() };
                p.note(format!("Declared early for the NFL draft at {age} ({proj})."));
                w.log.declared += 1;
                bump_day(&mut w.log.declared_by_day, date);
                let loud = if round == 1 { Loudness::Medium } else { Loudness::Small };
                events.push((
                    c,
                    format!("{} of {} declares for the NFL draft ({proj}).", p.name, w.programs[c].real_name),
                    loud,
                    p.player.id,
                    p.name.clone(),
                ));
            }
            Declare::Withdraw => {
                // A player who opted out of the postseason for the draft has
                // left college football: he stays in.
                if p.stage != Stage::Declared || exhausted(p, &w.programs, draft_year) || p.cal.opted_out {
                    continue;
                }
                let Some(read) = p.cal.draft_read else { continue };
                // The advisory committee's read comes back: a poor one sends
                // some home. The draws are keyed, so a second withdrawal day
                // (the title game's players) leaves earlier choices as they were.
                // He declared against his advisory grade; some think again.
                let advisory = p.cal.advisory.map_or(read.round as f64, |a| a as f64);
                if advisory >= 5.0 && key_uniform(seed ^ 0xAE, p.player.id as u64) < 0.3 {
                    let p = &mut w.people[i];
                    p.stage = Stage::College;
                    // He is back for next season: no second declaration this winter.
                    p.cal.declare_plan = Some(false);
                    // He has eligibility left again: no all-star game this winter.
                    p.cal.all_star = None;
                    let age = p.row_age();
                    p.note(format!(
                        "Withdrew from the draft at {age} after a poor advisory grade and returned to school."
                    ));
                    w.log.withdrew += 1;
                    bump_day(&mut w.log.withdrew_by_day, date);
                    if let Some(c) = p.college {
                        events.push((
                            c,
                            format!("{} withdraws from the draft and returns to {}.", p.name, w.programs[c].real_name),
                            Loudness::Small,
                            p.player.id,
                            p.name.clone(),
                        ));
                    }
                }
            }
        }
    }
    for (c, text, loud, id, name) in events {
        w.calendar.post(
            Event::new(date, CalLevel::College, Kind::DraftChoice, loud, text)
                .program(c)
                .rule("NFL: underclassman deadline")
                .person(id, &name)
                .school_link(c, &w.programs[c].real_name),
        );
    }
    if phase == Declare::Withdraw {
        w.rebuild_rosters();
    }
}

/// The NFL combine invite list (A10): the best of the draft pool, by the
/// staffs' December reads (about 320 a year).
pub(crate) fn combine_invites(w: &mut World) {
    // The declared underclassmen were on nobody's spring list: the clubs
    // write them up from film now (section 7, topic 3).
    super::pro::scouts::underclassmen(w);
    // Last year's invites are done with: a pro is never invited again (trial
    // 9: the flag stayed on, so veterans "tested among the best" every year).
    w.people.par_iter_mut().with_min_len(4096).for_each(|p| p.cal.combine_invite = false);
    let mut pool: Vec<(f64, usize)> = w
        .people
        .par_iter()
        .enumerate()
        .with_min_len(4096)
        .filter(|(_, p)| p.stage == Stage::Declared)
        .filter_map(|(i, p)| p.cal.draft_read.map(|r| (r.score, i)))
        .collect();
    pool.sort_by(|a, b| b.0.total_cmp(&a.0));
    // The league invites about 330 a year whatever the class (nflverse
    // 2000-2026: 319-337, 464 in 2021's virtual year; minor bug 97: the
    // draftable reads plus 60 gave 214-217), so the best 330 of the pool by
    // the staffs' reads, and anyone they read as draftable past that.
    let draftable = w
        .people
        .iter()
        .filter(|p| p.stage == Stage::Declared && p.cal.draft_read.is_some_and(|r| r.round <= 7))
        .count();
    let n = ((330.0 * w.options.scale).round() as usize).max(draftable).max(1).min(pool.len());
    for &(_, i) in pool.iter().take(n) {
        w.people[i].cal.combine_invite = true;
    }
    w.log.combine_invites += n as u32;
    // No line while the world spins up and nobody has declared yet.
    if n > 0 {
        let text = format!("The NFL invites {n} prospects to the scouting combine.");
        w.calendar.post(Event::new(w.today, CalLevel::Nfl, Kind::Date, Loudness::Medium, text));
    }
}

/// The combine (A10, A11): the measurements and interviews go into every
/// club's file (section 7, topic 10: the flat grade shift is gone, so a
/// club that knew a player learns little and one that didn't learns a
/// lot). The best testers at each position make the news.
pub(crate) fn combine(w: &mut World) {
    use crate::attributes::Attr;
    let invited: Vec<usize> = (0..w.people.len())
        .filter(|&i| w.people[i].cal.combine_invite && w.people[i].stage == Stage::Declared)
        .collect();
    if invited.is_empty() {
        return;
    }
    super::pro::scouts::combine(w, &invited);
    if w.logging() {
        for &i in &invited {
            let p = &w.people[i];
            let skipped = w.scouting.predraft.has(p.player.id, super::scouting::predraft::SKIP_COMBINE);
            let expected = crate::body::Drills::of(&p.player).expected();
            w.log_why(super::wlog::Why::Combine { id: p.player.id, program: p.college, skipped, expected });
        }
    }
    let test = |p: &Person| {
        let a = &p.player.attributes;
        (a.get(Attr::Speed) + a.get(Attr::Acceleration) + a.get(Attr::Agility) + a.get(Attr::Strength)) / 4.0
    };
    let mut by_pos: std::collections::HashMap<Position, Vec<f64>> = std::collections::HashMap::new();
    for &i in &invited {
        by_pos.entry(w.people[i].player.position).or_default().push(test(&w.people[i]));
    }
    let stats: std::collections::HashMap<Position, (f64, f64)> = by_pos
        .into_iter()
        .map(|(k, v)| {
            let m = v.iter().sum::<f64>() / v.len() as f64;
            let sd = (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64).sqrt().max(0.5);
            (k, (m, sd))
        })
        .collect();
    let mut risers = Vec::new();
    for &i in &invited {
        let p = &mut w.people[i];
        let (m, sd) = stats[&p.player.position];
        if (test(p) - m) / sd >= 1.8 {
            risers.push(i);
            let age = p.row_age();
            p.note(format!("Tested among the best at his position at the combine at {age}."));
        }
    }
    if let Some(&i) = risers.first() {
        let p = &w.people[i];
        let text = format!("{} lights up the combine.", p.name);
        let e = Event::new(w.today, CalLevel::Nfl, Kind::News, Loudness::Small, text).person(p.player.id, &p.name);
        w.calendar.post(e);
    }
}

/// The college all-star games in the order they name their rosters: the
/// Senior Bowl picks first every year, whatever the game dates, then the
/// Shrine Bowl, the HBCU Legacy Bowl and the NFLPA Collegiate Bowl.
pub(crate) const ALL_STAR_ORDER: [&str; 4] =
    ["Senior Bowl", "East-West Shrine Bowl", "HBCU Legacy Bowl", "NFLPA Collegiate Bowl"];

/// Whether a college all-star game is played before a draft: the NFLPA
/// Collegiate Bowl from 2012, the HBCU Legacy Bowl from 2022.
/// The mark of a player who turned an all-star invitation down (part C 5).
pub(crate) const DECLINED: &str = "declined";

pub(crate) fn all_star_held(game: &str, draft_year: i32) -> bool {
    match game {
        "HBCU Legacy Bowl" => draft_year >= 2022,
        "NFLPA Collegiate Bowl" => draft_year >= 2012,
        _ => true,
    }
}

/// College all-star games (A9): who is invited, by the staffs' reads.
/// The games name their rosters together, in `ALL_STAR_ORDER`: the Senior
/// Bowl takes the best seniors, the Shrine Bowl the next tier, the HBCU
/// Legacy Bowl (2022 on) players from HBCU programs, the NFLPA Collegiate
/// Bowl the tier after. Real rosters are filled from fall invitations, so a
/// senior whose college season is over is invited on his December draft
/// read, before the January deadline puts him in the draft pool.
pub(crate) fn all_star_invites(w: &mut World, game: &'static str) {
    let year = w.year;
    if !all_star_held(game, year + 1) {
        return;
    }
    let draft_year = year + 1;
    // The HBCU conferences: SWAC and MEAC in FCS, SIAC and CIAA in Division II (part B 42).
    let hbcu = |c: usize| matches!(w.programs[c].conference.as_str(), "SWAC" | "MEAC" | "SIAC" | "CIAA");
    // Teams still playing (bracket rounds not drawn yet included): their
    // seniors are invited once their season is over.
    let playing: Vec<bool> = match w.college.as_ref() {
        Some(cs) => (0..w.programs.len()).map(|t| super::college::still_playing(cs, t)).collect(),
        None => vec![false; w.programs.len()],
    };
    let done_senior = |p: &Person| {
        p.stage == Stage::College
            && p.cal.portal.is_none()
            && p.college.is_some_and(|c| !playing[c])
            && exhausted(p, &w.programs, draft_year)
    };
    let mut pool: Vec<(f64, usize)> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| (p.stage == Stage::Declared || done_senior(p)) && p.cal.all_star.is_none())
        // Seniors, and underclassmen who have graduated; any declared
        // underclassman from the 2024 games, when the NFL opened the
        // all-star games to them (part C 4; NFL, November 2023).
        .filter(|(_, p)| exhausted(p, &w.programs, draft_year) || p.cal.graduated || draft_year >= 2024)
        .filter(|(_, p)| game != "HBCU Legacy Bowl" || p.college.is_some_and(hbcu))
        .filter_map(|(i, p)| p.cal.draft_read.map(|r| (r.score, i)))
        .collect();
    pool.sort_by(|a, b| b.0.total_cmp(&a.0));
    // The games before this one in the order have taken their players
    // already, so each takes the best left.
    let size = match game {
        "Senior Bowl" => 110.0,
        "East-West Shrine Bowl" => 110.0,
        "HBCU Legacy Bowl" => 90.0,
        _ => 100.0,
    };
    let n = ((size * w.options.scale).round() as usize).max(1);
    // Each of the two teams carries one kicker, one punter and one long
    // snapper (from memory, to verify): two of each at most, however well
    // the December read rates them.
    let per_specialist = ((2.0 * w.options.scale).round() as usize).max(1);
    let mut specialists: Vec<crate::positions::Position> = Vec::new();
    let mut count = 0;
    let seed = w.options.seed as u64 ^ ((draft_year as u64) << 20) ^ 0xA115;
    for (rank, &(_, i)) in pool.iter().enumerate() {
        if count as usize == n {
            break;
        }
        let pos = w.people[i].player.position;
        if pos.is_specialist() {
            if specialists.iter().filter(|&&q| q == pos).count() >= per_specialist {
                continue;
            }
            specialists.push(pos);
        }
        // A projected first-rounder often skips the week, with nothing to
        // gain and an injury to risk (part C 5; proposed, not confirmed: the
        // best 15 of the pool 60%, the rest of round one 35%, round two 10%,
        // anyone else 2%). He is not asked by the games after it either.
        let round = w.people[i].cal.draft_read.map_or(8, |r| r.round);
        let skip = match round {
            1 if (rank as f64) < 15.0 * w.options.scale => 0.6,
            1 => 0.35,
            2 => 0.1,
            _ => 0.02,
        };
        if key_uniform(seed ^ game.len() as u64, w.people[i].player.id as u64) < skip {
            let p = &mut w.people[i];
            p.cal.all_star = Some(DECLINED);
            let age = p.row_age();
            p.note(format!("Turned down the {game} at {age} to prepare for the draft on his own."));
            w.log.all_star_declined += 1;
            continue;
        }
        w.people[i].cal.all_star = Some(game);
        count += 1;
    }
    w.log.all_star_invites += count;
    if count > 0 {
        let text = format!("The {game} names its rosters ({count} players).");
        w.calendar.post(Event::new(w.today, CalLevel::College, Kind::Date, Loudness::Small, text));
    }
}

/// An all-star game's practice week: scouts get a close look, so a player's
/// grade moves toward what he really is. A senior invited before the
/// January deadline plays as a college player still.
pub(crate) fn all_star_game(w: &mut World, game: &'static str) {
    let mut n = 0;
    let players: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            let p = &w.people[i];
            p.cal.all_star == Some(game) && matches!(p.stage, Stage::Declared | Stage::College)
        })
        .collect();
    super::pro::scouts::all_star(w, game, &players);
    for p in w
        .people
        .iter_mut()
        .filter(|p| p.cal.all_star == Some(game) && matches!(p.stage, Stage::Declared | Stage::College))
    {
        let level = p.college.map_or(Level::FBS, |_| Level::FBS);
        let rep = scout_player(&p.player, ScoutOptions { accuracy: 16.0, level }, &mut p.rng);
        if let Some(r) = p.cal.draft_read.as_mut() {
            let shift = 0.3 * (rep.overall - (r.grade - 0.0));
            let shift = clamp(shift, -2.0, 2.0);
            r.grade += shift;
            r.score += shift;
        }
        let age = p.row_age();
        p.note(format!("Played in the {game} at {age}."));
        n += 1;
    }
    if n > 0 {
        let text = format!("The {game} is played; scouts watched {n} prospects up close all week.");
        w.calendar.post(Event::new(w.today, CalLevel::College, Kind::Game, Loudness::Small, text));
    }
}

/// The NFL draft (T2): three days in April, team by team (note 7), from the
/// pool that declared in January, in the order of last season's standings.
/// Each team picks from its own board: the consensus grade (aligned across
/// positions, since positions mature at different ages), seen through its
/// front office, plus what the position is worth and what the team needs.
///
/// With your seat at an NFL club the draft runs live in your war room
/// (section 7, topic 11): the AI picks run up to your club's turn and the
/// draft waits for you (`world::scouting::live`). Without one it runs whole,
/// pick by pick on the same steps.
/// The night a round is drafted on (T1.5): round 1 on the first, rounds 2
/// and 3 on the second, 4 to 7 on the last day.
pub(crate) fn night_of(round: u32) -> usize {
    match round {
        1 => 0,
        2 | 3 => 1,
        _ => 2,
    }
}

/// The draft's steps on world day `d`: each night's opening minute (the
/// six-part day: the three days run on their own days).
pub(crate) fn draft_steps(w: &World, d: Date) -> Vec<u16> {
    let under_way = draft_under_way(w);
    let mut v: Vec<u16> = w
        .dates
        .draft
        .iter()
        .enumerate()
        .filter(|&(k, _)| k == 0 || under_way)
        .filter_map(|(_, &m)| crate::calendar::zone::day_minute(d, m))
        .collect();
    // Each pick still to come today, at its own minute (the war room makes
    // its own picks while it holds the clock).
    if let Some(run) = w.draft_run.as_ref() {
        let stamps = pick_stamps(run.year, &run.board, w.options.scale, w.dates.draft);
        v.extend(
            stamps
                .iter()
                .skip(run.next.saturating_sub(1) as usize)
                .filter_map(|&m| crate::calendar::zone::day_minute(d, m)),
        );
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// A draft night opens at its minute: the first sets the draft up (or opens
/// your war room), and each makes its own rounds' picks.
pub(crate) fn draft_at(w: &mut World, d: Date, t: u16) {
    for k in 0..3 {
        if crate::calendar::zone::day_minute(d, w.dates.draft[k]) == Some(t) && (k == 0 || draft_under_way(w)) {
            super::clock::timed("draft", || draft_night(w, k));
        }
    }
    if w.draft_run.is_some() {
        super::clock::timed("draft", || draft_due(w, crate::calendar::zone::moment(d, t)));
    }
}

fn draft_under_way(w: &World) -> bool {
    w.draft_run.is_some() || w.scouting.live.as_ref().is_some_and(|l| l.waiting)
}

/// Night `k` of the draft opens: the first sets the draft up (or opens your
/// war room), a later one opens your war room again. The clubs' own picks
/// come at their minutes (`draft_due`).
fn draft_night(w: &mut World, k: usize) {
    if k == 0 && !draft_under_way(w) && w.scouting.live.is_none() {
        let mut run = draft_begin(w);
        if super::scouting::live::start(w, &mut run) {
            return;
        }
        w.draft_run = Some(run);
    }
    // Your war room opens again for its next night.
    if w.scouting.live.as_ref().is_some_and(|l| l.waiting) {
        super::scouting::live::open_night(w, k);
    }
}

/// The picks whose minute has come, each on the books at its minute; the
/// undrafted rush after the last pick.
fn draft_due(w: &mut World, now: Moment) {
    let Some(mut run) = w.draft_run.take() else { return };
    let stamps = pick_stamps(run.year, &run.board, w.options.scale, w.dates.draft);
    while !run.done() && stamps.get(run.next as usize - 1).is_some_and(|&m| m <= now) {
        draft_step(w, &mut run, None, None);
    }
    commit_picks(w, &mut run);
    if run.done() {
        draft_end(w, run, None);
    } else {
        w.draft_run = Some(run);
    }
}

/// The minute each pick on the board is made (T1.5): round 1 on the first
/// night, rounds 2-3 on the second, 4-7 on the last day. A pick takes three
/// fifths of its round's clock on average in rounds 1-3 and half of it on
/// the last day (real recent drafts, from memory: round 1 about 6 minutes a
/// pick and done near 11:20 PM ET, round 2 about 4, round 3 about 3, the
/// last day about 2.5 and done between 6:30 and 7 PM), after the
/// commissioner's ten-minute opening and with five minutes between rounds
/// on the same day. Rounds are counted on the board itself, so the
/// compensatory picks lengthen rounds 3-7 as they do; in a smaller world
/// each pick stands for the real picks it thins out. A day that would run
/// into the next one or past midnight is squeezed to fit (the older drafts).
pub(crate) fn pick_stamps(year: i32, board: &[(usize, u32)], scale: f64, starts: [Moment; 3]) -> Vec<Moment> {
    let stands_for = 1.0 / scale.clamp(0.01, 1.0);
    let day = night_of;
    let mut at = [10.0f64, 0.0, 0.0];
    let mut mins = Vec::with_capacity(board.len());
    let mut last = 0;
    for &(_, r) in board {
        let j = day(r);
        if last != 0 && r != last && day(last) == j {
            at[j] += 5.0;
        }
        mins.push((j, at[j]));
        let share = if r <= 3 { 0.6 } else { 0.5 };
        at[j] += share * super::scouting::live::clock_minutes(year, r) as f64 * stands_for;
        last = r;
    }
    let room = |j: usize| -> f64 {
        let end = if j < 2 && starts[j + 1].date == starts[j].date {
            starts[j + 1]
        } else {
            Moment::at(starts[j].date, 23, 59)
        };
        (starts[j].date.days_to(end.date) * 1440 + end.minute as i32 - starts[j].minute as i32) as f64 - 10.0
    };
    let squeeze: Vec<f64> = (0..3).map(|j| (room(j) / at[j].max(1.0)).min(1.0)).collect();
    mins.into_iter().map(|(j, m)| starts[j].add_minutes((m * squeeze[j]) as i32)).collect()
}

/// A draft under way: the order, who's left, every club's board and what has
/// happened so far. Kept on the world while your war room is open, so a save
/// made mid-draft carries on (people keep their places until the draft ends).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DraftRun {
    pub year: i32,
    /// Each slot's club and round.
    pub board: Vec<(usize, u32)>,
    /// Who's left, best first by the public grade (aligned).
    pub pool: Vec<(usize, f64)>,
    pub boards: Vec<super::pro::scouts::Board>,
    pub public_rank: std::collections::HashMap<usize, usize>,
    /// Each pick: the player, the pick, the club and why.
    pub picked: Vec<(usize, u32, usize, String)>,
    pub traded_up: std::collections::HashSet<u32>,
    /// Draft-day trades: (buyer, seller, pick, what was given, future pick).
    pub swaps: Vec<(usize, usize, u32, String, Option<u32>)>,
    pub slides: Vec<(usize, usize, u32)>,
    /// The next pick, from 1.
    pub next: u32,
    /// The clubs' hunts (SK67): (club, the man's place in the people list).
    #[serde(default)]
    pub hunts: Vec<(usize, usize)>,
    /// The last pick on the books (each night's picks join their clubs
    /// that night).
    #[serde(default)]
    pub committed: u32,
}

impl DraftRun {
    pub fn slots(&self) -> usize {
        self.board.len()
    }
    pub fn done(&self) -> bool {
        self.next as usize > self.board.len() || self.pool.is_empty()
    }
}

/// Sets the draft up: the class moves to its pro positions, the pool in
/// public order, the slots with their owners, and every club's board.
pub(crate) fn draft_begin(w: &mut World) -> DraftRun {
    let draft_year = w.year;
    let seed = w.options.seed as u64;
    // The draft class moves to where the pro scouts project it.
    if w.logging() {
        for p in w.people.iter().filter(|p| p.stage == Stage::Declared) {
            if let Some(to) = p.cal.pro_position {
                let from = p.player.position;
                w.log_why(super::wlog::Why::PositionMove {
                    id: p.player.id,
                    from,
                    to,
                    from_fit: crate::positions::overall_for(from, &p.player.attributes),
                    to_fit: crate::positions::overall_for(to, &p.player.attributes),
                    by: "the pro scouts' projection",
                });
            }
        }
    }
    for p in w.people.iter_mut().filter(|p| p.stage == Stage::Declared) {
        if let Some(to) = p.cal.pro_position.take() {
            let from = p.player.position;
            crate::athlete::set_position(&mut p.player, to);
            p.note(format!("Played {from} in college; pro scouts project him at {to}."));
        }
    }
    let mut declared: Vec<(usize, f64)> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        if p.stage == Stage::Declared
            && let Some(r) = p.cal.draft_read
        {
            declared.push((i, r.grade));
        }
    }
    let grades: std::collections::HashMap<usize, f64> = declared.iter().copied().collect();
    // This year's teams in last season's order; the draft's rounds and
    // expansion places by era (see `franchise`).
    let base: Vec<usize> = super::nfl_year::draft_order(w)
        .unwrap_or_else(|| crate::career::draft_order(&w.nfl, &mut w.rng))
        .into_iter()
        .filter(|&t| super::franchise::active_in(w, t, draft_year))
        .collect();
    let mut board = super::franchise::draft_board(
        draft_year,
        &base,
        w.options.scale,
        seed,
        &super::franchise::first_draft_in(w, draft_year),
    );
    // Traded picks to their owners and free agency's compensatory picks.
    super::pro::trades::board_owners(w, draft_year, &mut board, &base);
    let declared_ids: Vec<usize> = declared.iter().map(|d| d.0).collect();
    let align = crate::level_rating::PositionAligner::for_draft(
        &declared_ids.iter().map(|&i| (w.people[i].player.position, grades[&i])).collect::<Vec<_>>(),
        full_draft(w, board.len()),
    );
    let mut pool: Vec<(usize, f64)> = declared_ids
        .iter()
        .map(|&i| {
            let g = align.align(w.people[i].player.position, grades[&i])
                + 1.0 * key_normal(seed ^ draft_year as u64, w.people[i].player.id as u64);
            (i, g)
        })
        .collect();
    pool.sort_by(|a, b| b.1.total_cmp(&a.1));
    // Each club picks from its own board, read from its own scouts' files
    // (section 7, topic 9); the public grade only fills in what it barely saw.
    let mut boards = super::pro::scouts::boards(w, &pool, &align);
    super::scouting::predraft::on_boards(w, &board, &mut boards);
    let public_rank = super::pro::scouts::public_ranks(&pool);
    let places = super::scouting::places(w);
    let hunts = super::hunt::ai::club_hunts(w, draft_year)
        .into_iter()
        .filter_map(|(t, id)| places.get(&id).map(|&i| (t, i)))
        .collect();
    DraftRun { year: draft_year, board, pool, boards, public_rank, next: 1, hunts, ..Default::default() }
}

/// The draft's picks at full size: a small world's few picks stand for a
/// whole draft, and its class is a bigger share of a full one than its
/// picks are, so specialists are placed against the full draft (a small
/// world's 13 picks once took a kicker 5th).
pub(crate) fn full_draft(w: &World, slots: usize) -> usize {
    (slots as f64 / w.options.scale.max(0.01)).round() as usize
}

/// A club's value of a prospect at a pick (value, position worth, need,
/// board grade), or None if it won't take him.
pub(crate) fn draft_value(w: &World, run: &DraftRun, t: usize, i: usize, pick: u32) -> Option<(f64, f64, f64, f64)> {
    let round = run.board.get(pick as usize - 1).map_or(7, |s| s.1);
    let rf = match round {
        1 | 2 => 1.0,
        3 | 4 => 0.6,
        _ => 0.3,
    };
    let b = run.boards.get(t)?;
    if b.dnd.contains(&i) {
        return None;
    }
    let g = *b.grade.get(&i)?;
    let pos = w.people[i].player.position;
    // Kickers, punters and long snappers: a team drafts one only when its
    // own is below par or aging, and even then most are found after the
    // draft (fix list 1.11; proposed, not confirmed).
    let need_here = crate::career::need(&w.nfl[t], pos);
    if pos.is_specialist() && need_here <= 0.0 {
        return None;
    }
    // Nor does a club take two at one specialist position in one draft.
    if pos.is_specialist() && run.picked.iter().any(|x| x.2 == t && w.people[x.0].player.position == pos) {
        return None;
    }
    // The GM's style (trial 4 topic 6.5): some draft for need, some
    // take the best player left (proposed, not confirmed: a need weight
    // from 0.5 to 1.5, his own for his career).
    let style = 0.5 + key_uniform(0xD4A7, w.nfl[t].id as u64);
    let need = need_here * if round <= 2 { 0.5 } else { 1.0 } * style;
    // A quarterback's premium is about need: a team happy with its starter
    // doesn't spend a first-round pick on another.
    // At the very top any team without a franchise QB takes the best one.
    let top_qb = clamp(1.0 - (pick as f64 - 1.0) / 8.0, 0.0, 1.0);
    let qb_gate = if pos == Position::QB { clamp((need_here + 2.0) / 4.0, 0.2, 1.0).max(top_qb) } else { 1.0 };
    let pv = draft_position_value(pos, g) * rf * qb_gate;
    // At the very top, a great player at a lesser position is worth less than
    // his grade says: a generational WR or safety rarely goes first.
    let lesser = !matches!(pos, Position::QB | Position::EDGE | Position::OT);
    let top = clamp(1.0 - (pick as f64 - 1.0) / 10.0, 0.0, 1.0);
    let damp = if lesser { 0.35 * (g - 85.0).max(0.0) * top } else { 0.0 };
    // The man a club hunts (SK67): it takes him at its pick, and trades up
    // for him, more readily.
    let hunt = if run.hunts.contains(&(t, i)) { super::hunt::HUNT_EDGE } else { 0.0 };
    Some((g + pv + need - damp + hunt, pv, need, g))
}

/// One pick. `choice` is the player your war room took (a place in the
/// people list) when the pick is yours; `seat` is your club, which the AI
/// never trades with on its own (offers to you come through the war room).
/// Returns false once the draft is over.
pub(crate) fn draft_step(w: &mut World, run: &mut DraftRun, seat: Option<usize>, choice: Option<usize>) -> bool {
    if run.done() {
        return false;
    }
    let draft_year = run.year;
    let seed = w.options.seed as u64;
    let pick = run.next;
    let slot = pick as usize - 1;
    let round = run.board[slot].1;
    // Clubs look at the best few hundred left by the public boards
    // (their own sleepers among them).
    let left: Vec<usize> = run.pool.iter().take(300).map(|x| x.0).collect();
    // A trade on the clock (topic 11), from each GM's chart, his tier
    // and a rival's need.
    let promised: Vec<(usize, u32)> = run.swaps.iter().filter_map(|s| s.4.map(|r| (s.0, r))).collect();
    if choice.is_none() && seat != Some(run.board[slot].0) {
        let value = |t: usize, i: usize| draft_value(w, run, t, i, pick).map(|x| x.0);
        if let Some(sw) = super::pro::scouts::trade_on_clock(w, &run.board, slot, &left, &value, &promised, seat) {
            apply_swap(run, pick, &sw);
        }
    }
    let t = run.board[slot].0;
    let found = match choice {
        Some(i) => run.pool.iter().position(|x| x.0 == i).map(|idx| {
            let (_, pv, need, g) = draft_value(w, run, t, i, pick).unwrap_or((
                0.0,
                0.0,
                0.0,
                run.boards.get(t).and_then(|b| b.grade.get(&i).copied()).unwrap_or(0.0),
            ));
            (idx, 0.0, pv, need, g)
        }),
        None => {
            let mut best: Option<(usize, f64, f64, f64, f64)> = None;
            for (idx, &(i, _)) in run.pool.iter().enumerate().take(300) {
                let Some((v, pv, need, g)) = draft_value(w, run, t, i, pick) else { continue };
                if best.is_none_or(|b| v > b.1) {
                    best = Some((idx, v, pv, need, g));
                }
            }
            best
        }
    };
    let Some((idx, _, pv, need, g)) = found else {
        run.next = run.board.len() as u32 + 1;
        return false;
    };
    // 3.16 an owner who overrules the draft room in round one for a big
    // name or a local hero (proposed, not confirmed: up to 15% of a
    // meddler's first-round picks). Your own picks are yours.
    let mut overruled = false;
    let idx = match w.clubs.get(t).map(|c| &c.office.owner) {
        Some(o)
            if choice.is_none()
                && round == 1
                && key_uniform(seed ^ 0x0A3E ^ draft_year as u64, pick as u64) < o.draft_override * 0.15 =>
        {
            let state = super::franchise::era_in(w, t, draft_year).and_then(|e| super::geo::state_index(e.state));
            match run.pool.iter().take(40).position(|&(j, _)| {
                let p = &w.people[j];
                p.stars >= 5 || state.is_some_and(|s| p.home_state == s && p.stars >= 4)
            }) {
                Some(k) if k != idx => {
                    overruled = true;
                    k
                }
                _ => idx,
            }
        }
        _ => idx,
    };
    let (i, public) = run.pool.remove(idx);
    let g = if overruled { run.boards[t].grade.get(&i).copied().unwrap_or(public) } else { g };
    let pos = w.people[i].player.position;
    let team = &w.nfl[t];
    let why = if choice.is_some() {
        "your war room's pick".to_string()
    } else if overruled {
        "the owner overruled the draft room for a name he wanted".to_string()
    } else if run.traded_up.contains(&pick) && run.hunts.contains(&(t, i)) {
        format!("they traded up to pick {pick} for the man they had chased all year")
    } else if run.traded_up.contains(&pick) {
        format!("they traded up to pick {pick} to take him")
    } else if need >= 1.0 {
        let weakest = team.weakest_starter(pos).map_or(0.0, |x| x.0);
        format!(
            "they needed {} (their weakest starter there was rated {weakest:.0})",
            crate::text::with_article(pos.name())
        )
    } else if pv >= 2.0 {
        let save = (market_value(veteran_top5(pos), g) - rookie_cap_share(pick)) * 279.2;
        format!(
            "{} is a premium position: on his rookie deal he would save about ${save:.0} million a year against a veteran of his grade",
            crate::text::with_article(pos.name())
        )
    } else {
        "he was the best player left on their board".to_string()
    };
    let rank = run.public_rank.get(&i).copied().unwrap_or(usize::MAX);
    let why = format!("{why}; their board grade {g:.1}, public {public:.1} ({})", ordinal_rank(rank));
    let player = &w.people[i].player;
    w.nfl[t].depth[pos.index()].push(crate::league::Member::background(player.overall(), player.age));
    run.slides.push((i, rank, pick));
    run.picked.push((i, pick, t, why));
    run.next += 1;
    !run.done()
}

/// Puts a trade on the clock on the board: the buyer takes this slot, the
/// seller the buyer's, plus what was added.
pub(crate) fn apply_swap(run: &mut DraftRun, pick: u32, sw: &super::pro::scouts::Swap) {
    let slot = pick as usize - 1;
    run.board[slot].0 = sw.buyer;
    run.board[sw.their_slot].0 = sw.seller;
    let mut what = format!("pick {}", sw.their_slot + 1);
    let mut future = None;
    match sw.extra {
        Some(super::pro::scouts::Extra::Slot(q)) => {
            run.board[q].0 = sw.seller;
            what += &format!(" and pick {}", q + 1);
        }
        Some(super::pro::scouts::Extra::Future(r)) => {
            future = Some(r);
            what += &format!(" and a {} round {r} pick", run.year + 1);
        }
        None => {}
    }
    run.swaps.push((sw.buyer, sw.seller, pick, what, future));
    run.traded_up.insert(pick);
}

/// After the last pick: the trades on the books, the undrafted rush (with
/// your war room's offers when you made them), every signing on its hour
/// and in the news, and the departments turning over.
pub(crate) fn draft_end(
    w: &mut World,
    mut run: DraftRun,
    seat_rush: Option<(usize, Vec<(usize, super::pro::scouts::Offer)>)>,
) -> Vec<(usize, super::pro::scouts::Offer)> {
    // The last night's picks go on the books first.
    commit_picks(w, &mut run);
    let draft_year = run.year;
    let y = w.dates.clone();
    let DraftRun { board, pool, boards: club_boards, picked, slides, .. } = run;
    let slots = board.len();
    // What it took to go in rounds 1-2 and 3-4 this year, as overall ratings:
    // staffs read their own underclassmen against it (fix list 1.2).
    let mut drafted: Vec<f64> = picked.iter().map(|x| w.people[x.0].player.overall()).collect();
    drafted.sort_by(|a, b| b.total_cmp(a));
    if !drafted.is_empty() {
        let end_of = |r: u32| board.iter().rposition(|x| x.1 <= r).map_or(0, |k| k + 1);
        let at = |r: u32| drafted[end_of(r).min(drafted.len() - 1)];
        w.draft_bars = Some((at(2), at(4)));
    }
    for (i, rank, pick) in slides {
        super::pro::scouts::note_slide(w, i, rank, pick);
    }
    // The undrafted rush (topic 12): each club calls the best left on its
    // own board with its bonus pool; each player picks his path.
    let left: Vec<usize> = pool.iter().map(|x| x.0).collect();
    let offers: Vec<(usize, super::pro::scouts::Offer)> = if w.clubs.is_empty() {
        let n = ((slots as f64 * PIPELINE.udfa_ratio).round() as usize).min(left.len());
        let cap = super::pro::money::price_cap(draft_year);
        (0..n)
            .map(|k| {
                let wanted = 1.0 - k as f64 / n.max(1) as f64;
                let bonus = cap * 0.0009 * wanted * wanted + cap * 0.00002;
                let t = super::pro::undrafted_team(w, left[k]);
                (left[k], super::pro::scouts::Offer { team: t, bonus, guarantee: 0.0 })
            })
            .collect()
    } else {
        let limit = w.rules.nfl_offseason as usize;
        // Sandbox mode's roster limits: every call your war room makes has
        // room on your club (AI clubs keep the 90).
        let seat_calls =
            seat_rush.as_ref().filter(|s| super::front_office::roster_lifted(w, s.0)).map(|s| (s.0, s.1.len()));
        let mut room: Vec<usize> = (0..w.nfl.len())
            .map(|t| {
                if t < w.clubs.len() && super::franchise::active_in(w, t, draft_year) {
                    // The picks are on the roster already.
                    let r = limit.saturating_sub(w.clubs[t].roster.len());
                    match seat_calls {
                        Some((st, n)) if st == t => r.max(n),
                        _ => r,
                    }
                } else {
                    0
                }
            })
            .collect();
        super::pro::scouts::rush(w, &club_boards, &left, &mut room, seat_rush.as_ref())
    };
    let udfa = offers.len();
    w.market.year_mut(draft_year).udfa_signed += udfa as u32;
    let by_offer: std::collections::HashMap<usize, super::pro::scouts::Offer> = offers.iter().copied().collect();
    // The draft classes' tally (lane NB1).
    if w.tallying() {
        let rows: Vec<(usize, u32, u32, usize)> =
            picked.iter().map(|(i, pk, t, _)| (*i, *pk, board[*pk as usize - 1].1, *t)).collect();
        w.tally_draft(draft_year, &rows);
    }
    // Those nobody signed: a rookie minicamp tryout or two first (topic 12),
    // then the end of the road for most.
    let unsigned: Vec<usize> = pool.iter().map(|x| x.0).filter(|i| !by_offer.contains_key(i)).collect();
    let tried = super::scouting::pro::minicamp_tryouts(w, &unsigned, &club_boards);
    let mut events: Vec<Event> = Vec::new();
    for &(i, _) in pool.iter() {
        match by_offer.get(&i) {
            Some(&o) => book_signing(w, i, None, Some(o), &board, draft_year, y.udfa_rush, &mut events),
            None if tried.contains_key(&i) => {}
            None => {
                let p = &mut w.people[i];
                p.stage = Stage::Done;
                p.note("Went undrafted and unsigned, and his football career ended.");
            }
        }
    }
    for e in events {
        w.calendar.post(e);
    }
    let udfa_text = format!("The undrafted free agent rush: {udfa} players sign within hours of the last pick.");
    w.calendar
        .post(Event::new(y.udfa_rush.date, CalLevel::Nfl, Kind::Signing, Loudness::Medium, udfa_text).at(y.udfa_rush));
    super::scouting::after_draft(w, &board, &tried);
    super::pro::after_draft(w);
    super::pro::scouts::after_rush(w);
    super::franchise::clear_dark(w);
    offers
}

/// One signing on the books: a pick (`how`: pick, club, why) or an
/// undrafted free agent (`offer`), at its minute `when`.
#[allow(clippy::too_many_arguments)]
fn book_signing(
    w: &mut World,
    i: usize,
    how: Option<(u32, usize, String)>,
    offer: Option<super::pro::scouts::Offer>,
    board: &[(usize, u32)],
    draft_year: i32,
    when: Moment,
    events: &mut Vec<Event>,
) {
    let season = draft_year - 1;
    let c = w.people[i].college.unwrap();
    let school = w.program_name(c).to_string();
    let pick = how.as_ref().map(|h| h.0);
    let round = pick.map(|pk| board[pk as usize - 1].1);
    if let Some(r) = round {
        w.programs[c].draft_picks.push((draft_year, r));
    }
    let weight = match round {
        Some(1) => 1.0,
        Some(2 | 3) => 0.8,
        Some(_) => 0.5,
        None => 0.25,
    };
    credit_impact(w, i, c, weight, season);
    let team_name = how.as_ref().map(|h| w.nfl[h.1].name);
    let team_id = how.as_ref().map(|h| h.1);
    // An undrafted player signs where the rush took him.
    let udfa_team = offer.map(|o| o.team);
    let udfa_name = udfa_team.map_or("", |t| w.nfl[t].name);
    let p = &mut w.people[i];
    p.stage = Stage::Pro;
    let age = p.row_age();
    match (how, round, team_name) {
        (Some((pk, _, why)), Some(r), Some(team)) => {
            p.note(format!("Drafted by the {team} in round {r} (pick {pk}) out of {school} at {age}: {why}."));
            let loud = match pk {
                1 => Loudness::Huge,
                2..=10 => Loudness::Big,
                _ if r == 1 => Loudness::Medium,
                _ => Loudness::Small,
            };
            let text = format!("Pick {pk}: the {team} take {} ({}, {school}).", p.name, p.player.position);
            let mut e = Event::new(when.date, CalLevel::Nfl, Kind::DraftPick, loud, text)
                .at(when)
                .program(c)
                .person(p.player.id, &p.name)
                .school_link(c, &school);
            if pk == 1 {
                e = e.memorable();
            }
            if let Some(t) = team_id {
                e = e.team(t).club_link(t, team);
            }
            events.push(e);
        }
        _ => p.note(format!("Signed with the {udfa_name} as an undrafted free agent out of {school} at {age}.")),
    }
    if w.recording {
        let mut a = Alumnus::from_person(p, draft_year, pick, false);
        a.nfl_team = team_name;
        w.alumni.push(a);
    }
    match (team_id, pick, round, udfa_team) {
        (Some(t), Some(pk), Some(r), _) => super::pro::join_drafted(w, i, t, draft_year, pk, r),
        (_, _, _, Some(t)) => {
            let o = offer.unwrap();
            let terms = super::pro::money::undrafted_offer(draft_year, o.bonus, o.guarantee);
            super::pro::join_undrafted(w, i, t, draft_year, &terms)
        }
        _ => {}
    }
}

/// The picks made since the last ones on the books (the six-part day: each
/// night's picks join their clubs that night and come out at their own
/// minutes): the trades on the clock, then each pick.
pub(crate) fn commit_picks(w: &mut World, run: &mut DraftRun) {
    let draft_year = run.year;
    let y = w.dates.clone();
    let from = run.committed;
    // Every pick's minute, from the board's own rounds (T1.5).
    let stamps = pick_stamps(draft_year, &run.board, w.options.scale, y.draft);
    // The trades on the clock, on the books and in the news, each at its
    // pick's minute: the deal is announced as the buyer goes on the clock.
    let upto = if run.done() { run.board.len() as u32 } else { run.next.saturating_sub(1) };
    let swaps: Vec<_> = run.swaps.iter().filter(|s| s.2 > from && s.2 <= upto).cloned().collect();
    for (buyer, seller, pick, what, future) in swaps {
        let night = stamps.get(pick as usize - 1).map_or(y.draft[0], |m| m.add_minutes(-2));
        if let Some(r) = future {
            super::pro::trades::give_pick(w, buyer, seller, draft_year + 1, r);
        }
        let text = format!(
            "The {} trade up to pick {pick} with the {}, giving {what}.",
            w.nfl[buyer].name, w.nfl[seller].name
        );
        let picks = future.map(|r| vec![(draft_year + 1, r, buyer)]).unwrap_or_default();
        w.market.picks.trades.push(super::pro::trades::Trade {
            date: night.date,
            buyer,
            seller,
            player: u32::MAX,
            picks,
            text: text.clone(),
        });
        let my = w.market.year_mut(draft_year);
        my.draft_trades += 1;
        let loud = if pick <= 10 { Loudness::Medium } else { Loudness::Small };
        w.calendar.post(
            Event::new(night.date, CalLevel::Nfl, Kind::Transfer, loud, text)
                .at(night)
                .team(buyer)
                .club_link(buyer, w.nfl[buyer].name)
                .club_link(seller, w.nfl[seller].name),
        );
    }
    let mut events: Vec<Event> = Vec::new();
    let picks: Vec<(usize, u32, usize, String)> = run.picked.iter().filter(|x| x.1 > from).cloned().collect();
    for (i, pk, t, why) in picks {
        let when = stamps.get(pk as usize - 1).copied().unwrap_or(y.draft[2]);
        book_signing(w, i, Some((pk, t, why)), None, &run.board, draft_year, when, &mut events);
    }
    for e in events {
        w.calendar.post(e);
    }
    run.committed = run.next.saturating_sub(1).max(from);
}

/// "12th on the public boards", or "unranked" for a player outside them.
fn ordinal_rank(rank: usize) -> String {
    if rank == usize::MAX {
        return "unranked publicly".to_string();
    }
    let n = rank + 1;
    let suffix = match (n % 10, n % 100) {
        (1, 11) | (2, 12) | (3, 13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix} on the public boards")
}

// --- Recruiting ---------------------------------------------------------------------

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
struct Recruit {
    person: usize,
    /// How good his film is, in rating points, aligned by position.
    film: f64,
    /// The services' shared guess at his upside.
    projection: f64,
    visibility: f64,
    gpa: f64,
    juco_mover: bool,
    viral: bool,
    /// His physical gifts as measured (Dynamic Links, D3), in full links.
    #[serde(default)]
    gift: f64,
    /// How far film narrows him (SK1; 0 reads as 1).
    #[serde(default)]
    wide: f64,
    /// The services' shared lean from his tape (SK2, SK5), rating points.
    #[serde(default)]
    svc: f64,
}

/// What a full link is worth to a college staff, in film standard
/// deviations (D3; real staffs offer the freak before the services rank
/// him: about half a standard deviation for one full link, Claude's pick).
const STAFF_GIFT: f64 = 0.5;
/// What the services give the same gift, once they have seen him test.
const SERVICE_GIFT: f64 = 0.15;

/// Out-of-state offers a junior college makes in a year (proposed, not confirmed).
const JUCO_OUT_OF_STATE_OFFERS: usize = 15;

/// NCAA initial eligibility (note 6). Division I: a 2.3 core GPA qualifies;
/// 2.0-2.299 is an academic redshirt (he can sign, get aid and practice, but
/// not play in year one); under 2.0 he can't enroll. Division II: 2.2 and a
/// 2.0 partial qualifier. Junior colleges take anyone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum Eligibility {
    Qualifier,
    AcademicRedshirt,
    NonQualifier,
}

/// Whether a recruit's transcript meets his division's academic line.
/// Division I asks 2.3 (2.0 for an academic redshirt) and Division II 2.2.
/// Division III has no initial-eligibility rule: the school's own admission
/// decides (`admissions_bar`). The NAIA runs its own eligibility center: a
/// 2.3 high school GPA, or two of a 2.0 GPA, an 18 ACT and the top half of
/// his class (minor bug B8). Here 2.0 qualifies, as most who reach it meet
/// the test or the class rank (proposed, not confirmed); under it he can
/// still enroll but sits his first year. Junior colleges take anyone.
pub(crate) fn ncaa_tier(d: Division, gpa: f64) -> Eligibility {
    let full = match d {
        Division::JUCO | Division::D3 => return Eligibility::Qualifier,
        Division::NAIA => return if gpa >= 2.0 { Eligibility::Qualifier } else { Eligibility::AcademicRedshirt },
        Division::D2 => 2.2,
        _ => 2.3,
    };
    if gpa >= full {
        Eligibility::Qualifier
    } else if gpa >= 2.0 {
        Eligibility::AcademicRedshirt
    } else {
        Eligibility::NonQualifier
    }
}

/// The core GPA the NCAA reads (A30): through the class of 2020 a test score
/// had to match the GPA on the Division I sliding scale (2.3 with a 980 SAT,
/// 3.55 with 400; the 2021 and 2022 classes had tests waived, and they were
/// dropped in 2023). A score short of the scale drops him a tier: a would-be
/// qualifier becomes an academic redshirt (a partial qualifier), and below
/// that he can't enroll.
pub(crate) fn eligibility_gpa(gpa: f64, test: Option<u32>, sign_year: i32) -> f64 {
    let Some(score) = test else { return gpa };
    if sign_year > 2020 {
        return gpa;
    }
    let needed = 2.3 + (980.0 - score as f64) / 464.0;
    if gpa >= needed {
        gpa
    } else if gpa >= 2.3 {
        2.29
    } else {
        gpa.min(1.99)
    }
}

/// A school's academic support, 0-1: tutors, summer school, learning
/// specialists. It follows the athletic department's resources.
fn academic_support(prog: &Program) -> f64 {
    clamp(0.3 + 0.7 * prog.facilities, 0.0, 1.0)
}

/// Real players per FBS roster by position, 2022-24 (fix list topic 1:
/// ESPN rosters with CFBD recruiting links, about 134 teams a year). A staff
/// scales them to its own roster limit. The data lists most linemen and
/// backs only as OL, DL or DB; the teams that list the exact spot carry 48%
/// tackles among linemen, 52% edges in the front and 46% corners in the
/// secondary, which splits the groups (proposed, not confirmed).
fn real_roster(pos: Position) -> f64 {
    match pos {
        Position::QB => 5.8,
        Position::RB => 8.5,
        Position::WR => 16.2,
        Position::TE => 7.4,
        Position::OT => 19.5 * 0.48,
        Position::IOL => 19.5 * 0.52,
        Position::EDGE => 17.5 * 0.52,
        Position::DT => 17.5 * 0.48,
        Position::LB => 14.6,
        Position::CB => 22.2 * 0.46,
        Position::S => 22.2 * 0.54,
        Position::K => 3.5,
        Position::P => 2.0,
        Position::LS => 2.5,
    }
}

/// The real FBS roster these counts add up to.
const REAL_ROSTER_TOTAL: f64 = 119.7;

/// A full squad at each level, walk-ons included (part C 7): about 120 in
/// FBS before the House limit, 108 in FCS, 115 in Division II, 107 in
/// Division III, 100 in the NAIA and 80 at a junior college (NCAA
/// participation counts over teams, from memory, to verify).
pub(crate) fn level_roster(d: Division) -> f64 {
    match d {
        Division::FBS => REAL_ROSTER_TOTAL,
        Division::FCS => 108.0,
        Division::D2 => 115.0,
        Division::D3 => 107.0,
        Division::NAIA => 100.0,
        Division::JUCO => 80.0,
    }
}

/// Specialists a staff keeps on scholarship (fix list 1.6): about one kicker
/// and one punter. The other kickers, the second punter and the long
/// snappers walk on (real: 65-82% of FBS specialists have no recruiting
/// record). Other levels do the same (proposed, not confirmed).
fn specialist_scholarships(pos: Position) -> f64 {
    match pos {
        Position::K | Position::P => 1.0,
        _ => 0.0,
    }
}

/// A staff's class plan (fix list 1.1, 1.2, 1.6): how many it signs at each
/// position this year, on scholarship and as walk-ons, and the room it has.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct ClassPlan {
    scholarship: [usize; POSITION_COUNT],
    walk_on: [usize; POSITION_COUNT],
    /// Scholarship signees it can take this year.
    slots: usize,
    /// Roster places it keeps for walk-ons.
    walk_on_room: usize,
    /// Places under the limit open next fall, signees and walk-ons together.
    room: usize,
    /// The limit counts the whole roster (the House era), so a walk-on
    /// needs a place under it too.
    whole_roster: bool,
    /// Places open at each position next fall for a late walk-on: the
    /// depth a staff carries there less the players it expects back. Under
    /// a whole-roster limit that depth is the plan's target; under a
    /// scholarship limit it is a real full squad's, walk-ons included.
    open_at: [usize; POSITION_COUNT],
}

impl ClassPlan {
    /// A walk-on place for a recruit whose commitment fell through on
    /// signing day: one the plan keeps at his position, or a place at his
    /// position on the roster as it stands after the matching (`taken`:
    /// scholarships and walk-ons in the class; `at`: the same at his
    /// position).
    fn late_walk_on(&self, i: usize, taken: [usize; 2], at: [usize; 2]) -> bool {
        let planned = self.walk_on_open(i, taken[1], at);
        let under_limit = !self.whole_roster || taken[0] + taken[1] < self.room;
        planned || (under_limit && at[0] + at[1] < self.open_at[i])
    }

    /// A planned walk-on place at a position: under a scholarship limit
    /// the plan counts walk-ons alone; under a whole-roster limit a
    /// position's walk-ons fill what its signees leave of its target.
    fn walk_on_open(&self, i: usize, taken_walk_on: usize, at: [usize; 2]) -> bool {
        let shared = self.whole_roster && !Position::ALL[i].is_specialist();
        let at_pos = if shared { at[0] + at[1] } else { at[1] };
        taken_walk_on < self.walk_on_room && at_pos < self.walk_on[i]
    }
}

/// The share of programs a year where a student makes the team at an open
/// tryout (topic 5.5; proposed, not confirmed: one in twelve in FBS and FCS,
/// one in six below, where squads are bigger and the bar lower).
fn tryout_sticks(d: Division) -> f64 {
    match d {
        Division::FBS | Division::FCS => 0.08,
        Division::JUCO => 0.0,
        _ => 0.16,
    }
}

/// Fall camp (topic 5): walk-ons who played their way into the two-deep go
/// on scholarship where one is free (5.4), and a few students make a team
/// at an open tryout, the best of the state's new graduates who never
/// signed (5.5). Division III and the leagues without athletic aid (the Ivy
/// League, the Pioneer League, the Patriot League before 2013: `ncaa::aid`)
/// give none, so their walk-ons stay as they are.
pub(crate) fn camp_walk_ons(w: &mut World) {
    let season = w.year;
    w.rebuild_rosters();
    // Your program's walk-ons are your call (lane B): your staff names the
    // ones who earned it, and you hold your own tryout.
    let mine = fo::program(w);
    let mut word: Option<(usize, Vec<usize>, usize)> = None;
    let mut awards: Vec<(usize, usize)> = Vec::new();
    for (j, prog) in w.programs.iter().enumerate() {
        let no_aid = prog.division == Division::D3 || super::ncaa::aid(prog, season) != super::ncaa::Aid::Athletic;
        if !prog.plays(season) || no_aid {
            continue;
        }
        let limit = super::ncaa::roster_limits(prog, season).0;
        let college = |i: &usize| w.people[*i].stage == Stage::College;
        let on_aid = prog.roster.iter().filter(|i| college(i) && !w.people[**i].walk_on).count();
        let free = limit.saturating_sub(on_aid);
        if free == 0 {
            continue;
        }
        let mut cands: Vec<(f64, usize)> = prog
            .roster
            .iter()
            .copied()
            .filter(|i| college(i))
            // His role from a season here, not his high school's (lane B).
            .filter(|&i| w.people[i].walk_on && fo::walk_ons::in_two_deep(w, i, j))
            .map(|i| (w.people[i].last_performance.unwrap_or(w.people[i].player.overall()), i))
            .collect();
        cands.sort_by(|a, b| b.0.total_cmp(&a.0));
        // A team keeps one long snapper on scholarship at most (trial 8
        // finding 9: real teams usually have one): a walk-on snapper earns
        // one only when no snapper on the roster has it.
        let snap = |i: usize| w.people[i].player.position == Position::LS;
        let mut snapper_on_aid = prog.roster.iter().any(|&i| college(&i) && !w.people[i].walk_on && snap(i));
        let mut given = 0;
        let mut earned = Vec::new();
        for &(_, i) in &cands {
            if given == free {
                break;
            }
            if snap(i) {
                if snapper_on_aid {
                    continue;
                }
                snapper_on_aid = true;
            }
            earned.push(i);
            given += 1;
        }
        if Some(j) == mine {
            let whole = super::ncaa::whole_roster_limit(prog, season);
            word = Some((j, earned, if whole { usize::MAX } else { free }));
        } else {
            awards.extend(earned.into_iter().map(|i| (j, i)));
        }
    }
    if let Some((_, earned, free)) = word {
        fo::walk_ons::camp_word(w, &earned, free);
    }
    for (j, i) in awards {
        let name = w.programs[j].real_name.clone();
        let fbs = w.programs[j].division == Division::FBS;
        let p = &mut w.people[i];
        p.walk_on = false;
        let age = p.row_age();
        p.note(format!(
            "Earned a scholarship at {name} in fall camp at {age}, after playing his way into the two-deep."
        ));
        w.log.walk_on_scholarships += 1;
        if fbs && w.recording && p.last_role == Some(PlayingTime::Starter) {
            let text = format!("{name} puts former walk-on {} ({}) on scholarship.", p.name, p.player.position);
            let e = Event::new(w.today, CalLevel::College, Kind::Signing, Loudness::Small, text)
                .program(j)
                .person(p.player.id, &p.name)
                .school_link(j, &name);
            w.calendar.post(e);
        }
    }
    // Open tryouts: the best new graduates in each state who never signed.
    let mut by_state: Vec<Vec<(f64, usize)>> = vec![Vec::new(); w.states.len()];
    for (i, p) in w.people.iter().enumerate() {
        if p.stage == Stage::Done && p.class_of == season && p.path.is_empty() && p.college.is_none() {
            let s = w.high_schools[p.hs].state;
            by_state[s].push((p.player.overall(), i));
        }
    }
    for list in by_state.iter_mut() {
        list.sort_by(|a, b| b.0.total_cmp(&a.0));
        list.truncate(20);
    }
    let seed = w.options.seed as u64 ^ (season as u64) << 20 ^ 0x7A1C;
    for j in 0..w.programs.len() {
        let prog = &w.programs[j];
        if !prog.plays(season) || key_uniform(seed, j as u64) >= tryout_sticks(prog.division) || Some(j) == mine {
            continue;
        }
        let Some((_, i)) = by_state[prog.state].first().copied() else { continue };
        by_state[prog.state].remove(0);
        let name = prog.real_name.clone();
        let p = &mut w.people[i];
        p.stage = Stage::College;
        p.college = Some(j);
        p.path.push(j);
        p.walk_on = true;
        let age = p.row_age();
        p.note(format!("Made the team at {name} at an open tryout at {age}, a student nobody had recruited."));
        w.log.tryouts += 1;
    }
    w.rebuild_rosters();
}

/// Whether a player takes a place under his program's limit (F17). The
/// House era's limit counts the whole roster. Before it (and at the FCS
/// programs that stayed out, D2 and junior colleges) the limit counts
/// players on aid, and walk-ons sit outside it, except the specialists the
/// class plan keeps in its own walk-on room under the limit.
pub(crate) fn takes_a_place(p: &Person, whole_roster: bool, season: i32) -> bool {
    // A 2020 senior back for his COVID year didn't count against the 2021
    // scholarship limits (the NCAA's one-year exemption, A3).
    if super::eligibility::covid_senior(p, season) {
        return false;
    }
    // A player you cut (minor bug 60) is off the roster and leaves at the
    // next window: his place goes to next year's class.
    if p.cut_here().is_some() {
        return false;
    }
    if whole_roster {
        // The House settlement's designated student-athletes (real): a
        // walk-on on a roster in 2024-25, or promised a place in the 2025
        // class, never counts against the limit at any school.
        !(p.walk_on && p.class_of <= super::ncaa::HOUSE_START)
    } else {
        !p.walk_on || p.player.position.is_specialist()
    }
}

/// Each program's places taken under its limit in `season`, the season the
/// portal's entrants play next (`takes_a_place`): the players who return
/// and the recruits signed for it. A senior with no season left, a player
/// in the portal himself and a junior college player signed with a
/// four-year school are leaving, so their places are open to transfers, as
/// a staff counts them (minor bug 131: with them counted, a full roster in
/// December had no room until its seniors left in January, and the good
/// transfers it would have taken went to the FCS).
pub(crate) fn counted_rosters(w: &World, season: i32) -> Vec<usize> {
    // Whole counts, worked out on all cores (world speed lane).
    let mut n: Vec<usize> = w
        .programs
        .par_iter()
        .enumerate()
        .map(|(j, prog)| {
            let whole = super::ncaa::whole_roster_limit(prog, season);
            prog.roster
                .iter()
                .filter(|&&i| {
                    let p = &w.people[i];
                    takes_a_place(p, whole, season)
                        && p.cal.portal.is_none()
                        && p.cal.declare_plan != Some(true)
                        && p.cal.signed.is_none_or(|s| s.program == j)
                        && !super::college::no_season_left(p, prog.division, season - 1)
                })
                .count()
        })
        .collect();
    let signees: Vec<usize> = w
        .people
        .par_iter()
        .with_min_len(4096)
        .filter_map(|p| {
            // A signee not yet enrolled, counted at his new school (a junior
            // college mover is still on his old roster, not counted there).
            let s = p.cal.signed?;
            if p.college == Some(s.program) {
                return None;
            }
            let prog = w.programs.get(s.program)?;
            let counted = match (super::ncaa::whole_roster_limit(prog, season), s.walk_on) {
                (_, false) => true,
                (true, true) => p.class_of > super::ncaa::HOUSE_START,
                (false, true) => p.player.position.is_specialist(),
            };
            counted.then_some(s.program)
        })
        .collect();
    for j in signees {
        n[j] += 1;
    }
    n
}

/// When the staff reads its roster for a signing stage (T5.2): what this
/// season still adds to everyone's count and whether the winter portal is
/// still ahead. The seasons a player gets come from his division and the
/// season (`eligibility::seasons_allowed`, minor bug B9).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
struct RosterRead {
    stage: SignStage,
}

impl RosterRead {
    /// Seasons this season still adds: in the summer it hasn't been played,
    /// in December it counts unless a redshirt keeps it (settled January 2).
    fn pending(self, p: &Person) -> u32 {
        match self.stage {
            SignStage::Summer(_) => 1,
            SignStage::Early => p.cal.count_pending as u32,
            _ => 0,
        }
    }

    /// Whether the winter transfer window is still to come.
    fn portal_ahead(self) -> bool {
        matches!(self.stage, SignStage::Summer(_) | SignStage::Early)
    }
}

/// The staff's read of how likely a player is to leave before a season:
/// out of eligibility, an early NFL entrant, or a transfer out (fix list
/// 1.2). `seasons` is what he will have used by then, `draft_year` the draft
/// he would enter. `bars` are the overalls of the 64th and 138th best
/// players taken in the last draft: a player at that level now is read like
/// a round 1-2 or round 3-4 prospect, with the chances of declaring the
/// draft itself uses; a December read (`plan`) replaces the guess.
#[allow(clippy::too_many_arguments)]
fn leaving(
    p: &Person,
    d: Division,
    seasons: u32,
    draft_year: i32,
    limit: u32,
    bars: Option<(f64, f64)>,
    plan: Option<bool>,
    portal: bool,
) -> f64 {
    if d == Division::JUCO {
        // Junior college players move up or stop after two seasons.
        return if seasons >= 2 { 1.0 } else { 0.0 };
    }
    if seasons >= limit || draft_year > super::eligibility::clock_last(p) {
        return 1.0;
    }
    let early = if draft_year - p.class_of >= 3 {
        match plan {
            Some(true) => 0.9,
            Some(false) => 0.03,
            None => {
                let ovr = p.player.overall();
                match bars {
                    Some((r2, _)) if ovr >= r2 => 0.85,
                    Some((_, r4)) if ovr >= r4 => 0.35,
                    _ => 0.04,
                }
            }
        }
    } else {
        0.0
    };
    let transfer = if portal {
        match p.last_role {
            Some(PlayingTime::Backup) => PIPELINE.portal_entry.backup,
            Some(PlayingTime::Rotation) => PIPELINE.portal_entry.rotation,
            _ => 0.0,
        }
    } else {
        0.0
    };
    1.0 - (1.0 - early) * (1.0 - transfer)
}

/// The chances a player is gone before next fall, and before the fall after
/// (fix list 1.2, T5.2). In the summer and December the seniors, early
/// entrants and winter transfers are still on the roster: the staff signs
/// for the roster it will have, not the one it has.
fn gone(p: &Person, d: Division, sign_year: i32, bars: Option<(f64, f64)>, read: RosterRead) -> (f64, f64) {
    let seasons = p.college_seasons + read.pending(p);
    // By February the entrants have left; those still here are staying.
    let declared = !matches!(read.stage, SignStage::Final | SignStage::Late(_));
    let next = leaving(
        p,
        d,
        seasons,
        sign_year,
        super::eligibility::seasons_allowed(sign_year, d),
        bars,
        if declared { p.cal.declare_plan } else { Some(false) },
        read.portal_ahead(),
    );
    let after = leaving(
        p,
        d,
        seasons + 1,
        sign_year + 1,
        super::eligibility::seasons_allowed(sign_year + 1, d),
        bars,
        None,
        true,
    );
    (next, 1.0 - (1.0 - next) * (1.0 - after))
}

/// How many to sign toward a target: this year's hole, or half the hole two
/// years out (the other half is next year's class), whichever is bigger.
/// The two-year half rounds down, so a small room (two snappers) isn't
/// topped up a year early every year (proposed, not confirmed).
fn plan_count(target: f64, now: f64, staying: f64) -> usize {
    let this_year = crate::rng::round_half_up(target - now);
    let ahead = ((target - staying) / 2.0).floor();
    this_year.max(ahead).max(0.0) as usize
}

/// The most scholarship signees a class takes under the House roster limit
/// (part D 10; proposed, not confirmed).
const HOUSE_CLASS_MAX: usize = 30;

#[allow(clippy::too_many_arguments)]
fn class_plan(
    prog: &Program,
    people: &[Person],
    signed: &[usize],
    sign_year: i32,
    seed: u64,
    bars: Option<(f64, f64)>,
    read: RosterRead,
) -> ClassPlan {
    // A school without a team that fall signs no one and takes no walk-ons.
    if !prog.plays(sign_year) {
        return ClassPlan::default();
    }
    let (limit, signees) = super::ncaa::roster_limits(prog, sign_year);
    let scale = limit as f64 / REAL_ROSTER_TOTAL;
    // [scholarship, walk-on] players expected next fall, and those the staff
    // expects to stay two years.
    let mut now = [[0.0; 2]; POSITION_COUNT];
    let mut stay = [[0.0; 2]; POSITION_COUNT];
    let mut returning = 0.0;
    let whole_roster = super::ncaa::whole_roster_limit(prog, sign_year);
    for &i in &prog.roster {
        let p = &people[i];
        // A player in the portal is leaving: since entrants keep shopping
        // after signing day (minor bug 337), his place is open to the class.
        if p.stage != Stage::College || p.cal.portal.is_some() {
            continue;
        }
        let (k, wo) = (p.player.position.index(), p.walk_on as usize);
        let (next, two) = gone(p, prog.division, sign_year, bars, read);
        now[k][wo] += 1.0 - next;
        stay[k][wo] += 1.0 - two;
        if takes_a_place(p, whole_roster, sign_year) {
            returning += 1.0 - next;
        }
    }
    // Recruits who signed in an earlier period and haven't enrolled are
    // part of next fall's roster and of this class.
    let mut signed_class = 0;
    for &i in signed {
        let p = &people[i];
        let Some(s) = p.cal.signed else { continue };
        let (k, wo) = (p.player.position.index(), s.walk_on as usize);
        now[k][wo] += 1.0;
        stay[k][wo] += 1.0;
        let counted = match (whole_roster, s.walk_on) {
            (_, false) => true,
            (true, true) => p.class_of > super::ncaa::HOUSE_START,
            (false, true) => p.player.position.is_specialist(),
        };
        if counted {
            returning += 1.0;
        }
        if !s.walk_on {
            signed_class += 1;
        }
    }
    let mut plan = ClassPlan { whole_roster, ..Default::default() };
    for pos in Position::ALL {
        let k = pos.index();
        let mut target = real_roster(pos) * scale;
        if pos.is_specialist() {
            // Each staff has its own habit: 3.1 kickers a roster means some carry 3, some 4.
            // Under a scholarship limit the walk-ons fill out a real squad's
            // specialists, as at every other position.
            let habit = key_uniform(seed ^ 0x5EC1_A115 ^ k as u64, prog.id as u64);
            let squad =
                if whole_roster { target } else { real_roster(pos) * level_roster(prog.division) / REAL_ROSTER_TOTAL };
            let total = (squad + habit).floor();
            let on_scholarship = specialist_scholarships(pos).min(total);
            plan.scholarship[k] = plan_count(on_scholarship, now[k][0], stay[k][0]);
            // Everyone at the position counts against the total: a walk-on
            // put on scholarship in fall camp still snaps or kicks there
            // (trial 7 finding 14: counting walk-ons alone kept adding
            // snappers, 4.6 a roster against a real 2.5).
            let signed = plan.scholarship[k] as f64;
            plan.walk_on[k] = plan_count(total, now[k][0] + now[k][1] + signed, stay[k][0] + stay[k][1] + signed);
            target = total;
        } else {
            // Under a scholarship limit the target is the position's share
            // of the players on aid, so only they count against it; walk-ons
            // sit outside the limit. Under the House limit every body takes
            // a place (minor bug 105: counting walk-ons here left FBS
            // rosters about 67 on aid and classes near 2,500 a year).
            let (now_k, stay_k) =
                if whole_roster { (now[k][0] + now[k][1], stay[k][0] + stay[k][1]) } else { (now[k][0], stay[k][0]) };
            plan.scholarship[k] = plan_count(target, now_k, stay_k);
            if whole_roster {
                // The places a class leaves open at a position go to
                // walk-ons on partial aid or none (fix below).
                plan.walk_on[k] = plan.scholarship[k];
            } else {
                // Preferred walk-ons fill out a real squad for the level
                // (topic 5.1), on top of the scholarship signees.
                let squad = real_roster(pos) * level_roster(prog.division) / REAL_ROSTER_TOTAL;
                let signed = plan.scholarship[k] as f64;
                plan.walk_on[k] = plan_count(squad, now[k][0] + now[k][1] + signed, stay[k][0] + stay[k][1] + signed);
            }
        }
        let depth =
            if plan.whole_roster { target } else { real_roster(pos) * level_roster(prog.division) / REAL_ROSTER_TOTAL };
        plan.open_at[k] = crate::rng::round_half_up(depth - now[k][0] - now[k][1]).max(0.0) as usize;
    }
    let room = limit.saturating_sub(returning.round() as usize);
    plan.room = room;
    // Under a scholarship limit only the specialists' walk-on places sit
    // under it; the other walk-ons are outside it (F17).
    let under: usize = Position::ALL
        .into_iter()
        .filter(|p| p.is_specialist())
        .map(|p| plan.walk_on[p.index()])
        .sum::<usize>()
        .min(room);
    let outside: usize = Position::ALL
        .into_iter()
        .filter(|p| !whole_roster && !p.is_specialist())
        .map(|p| plan.walk_on[p.index()])
        .sum();
    plan.walk_on_room = under + outside;
    // The era's yearly signing cap (11.4) bounds the class. Under the
    // House roster limit the new places fill over several classes, through
    // walk-ons and the portal; a high school class stays near its usual
    // size (part D 10: real 2025 and 2026 classes stayed about 20-30).
    let usual = if whole_roster { HOUSE_CLASS_MAX } else { usize::MAX };
    plan.slots = (room - under).min(signees.saturating_sub(signed_class)).min(usual.saturating_sub(signed_class));
    // Under the House limit the places the class can't fill go to
    // walk-ons on partial aid or none, signed in the final period after
    // the January portal window has had its turn (proposed, not
    // confirmed; real Power 4 rosters reached the 105 limit in 2025).
    if whole_roster {
        plan.walk_on_room = room - plan.slots;
    }
    plan
}

/// What a kicking camp measures (fix list 1.4): leg power, accuracy by
/// distance, get-off and the mental side, weighted as the position rates
/// them. A long snapper is measured on his snap.
fn specialist_grade(p: &crate::player::Player) -> f64 {
    p.overall()
}

/// Camp star tiers for specialists (fix list 1.5), by national rank at each
/// spot in a class: 5 stars for the top 25 (ProKicker's own definition, a
/// top-25 prospect in the country), 4 to 100, 3 to 250, 2 to 500. About
/// 100 kickers a year join FBS rosters, 250 FBS and FCS, 500 any college
/// (proposed, not confirmed).
const CAMP_TIERS: [(u8, f64); 4] = [(5, 25.0), (4, 100.0), (3, 250.0), (2, 500.0)];

/// The snapping camps' tiers (part D 8): a class sends about 30-40 long
/// snappers to FBS rosters and as many to FCS, so their 5 stars go to about
/// the top 12, 4 to 40, 3 to 100 and 2 to 250 (Rubio and Kohl's style
/// rankings; proposed, not confirmed, to verify).
const SNAP_TIERS: [(u8, f64); 4] = [(5, 12.0), (4, 40.0), (3, 100.0), (2, 250.0)];

/// The most stars the recruiting services give a kicker, punter or long
/// snapper (fix list 1.5): the general sites rarely rate one above 3 stars.
pub(crate) const SERVICE_SPECIALIST_MAX: u8 = 3;

/// The services' stars down a class ranked best first (`specialist` marks
/// the kickers, punters and long snappers): each tier of `quotas` (stars,
/// players) goes to that many players at the regular positions, and a
/// specialist ranked among them gets the tier's stars up to
/// `SERVICE_SPECIALIST_MAX` without using a place.
/// The stars with S5's speed floors: a floored kid is raised to his floor,
/// and each tier's lowest-ranked kids without a floor there step down a
/// tier until it holds its quota again (a tier that is all floors keeps
/// them, a few over).
fn service_stars_floored(specialist: &[bool], floors: &[u8], quotas: &[(u8, usize)]) -> Vec<u8> {
    let mut out = service_stars(specialist, quotas);
    for (k, &f) in floors.iter().enumerate() {
        if f > out[k] && !specialist[k] {
            out[k] = f;
        }
    }
    for (t, &(stars, quota)) in quotas.iter().enumerate() {
        let next = quotas.get(t + 1).map_or(0, |q| q.0);
        let mut count = out.iter().zip(specialist).filter(|(s, sp)| **s == stars && !**sp).count();
        let mut k = out.len();
        while count > quota && k > 0 {
            k -= 1;
            if out[k] == stars && !specialist[k] && floors[k] < stars {
                out[k] = next;
                count -= 1;
            }
        }
    }
    out
}

fn service_stars(specialist: &[bool], quotas: &[(u8, usize)]) -> Vec<u8> {
    let mut out = vec![0u8; specialist.len()];
    let (mut tier, mut used) = (0, 0);
    for (k, &spec) in specialist.iter().enumerate() {
        while tier < quotas.len() && used >= quotas[tier].1 {
            tier += 1;
            used = 0;
        }
        let Some(&(stars, _)) = quotas.get(tier) else { break };
        if spec {
            out[k] = stars.min(SERVICE_SPECIALIST_MAX);
        } else {
            out[k] = stars;
            used += 1;
        }
    }
    out
}

/// How far the recruiting services' read of a kid's upside sits from the
/// truth, the same at every look (A12, final build). Two real causes:
/// physical maturity, since the services rank what a body does at 17 and
/// an older kid in his class (a late-summer birthday held back, or simply
/// born in September) looks a year further along than he will end up
/// (the relative-age effect in recruiting rankings); and the services'
/// own shared misreads of a kid (the camp circuit, one big offer, a level
/// of competition). Claude's pick, proposed in the final build: 1.6
/// points of upside per year older than the middle of his class, and a
/// lasting misread of 9 points (one standard deviation), sized so about
/// 62% of 5-stars are drafted, as in real classes. A first try at 3.4
/// moved the rate only from 87.6% to 84.5%: at the very top of a class of
/// 250,000 the services need a lot of noise before a kid who won't make
/// it ranks among the best 32. Speed update S9: with recorded times and
/// S5 driving stars, the misread is 6 points (proposed, not confirmed; the
/// share of 5-stars drafted is checked on the review list, R2). Minor bug
/// 101 tried sharing it with every staff's film (7 points): five-stars
/// drafted fell to the real 63%, but staffs then placed boys less by what
/// they are, so FBS lost 1.5 points of true talent, four-stars drafted fell
/// to 17% (real 22%), FBS receivers came out 59% Black (real 64%) and more
/// colleges had a pick. The services alone carry it.
fn service_bias(p: &Person, world_seed: u64) -> f64 {
    let older = if p.born == crate::calendar::born::UNKNOWN {
        0.0
    } else {
        let start = crate::calendar::Date::ymd(p.class_of - 19, 9, 1);
        ((start.0 - p.born.0) as f64 / 365.25 + 0.5).clamp(-1.5, 1.5)
    };
    SERVICE_MATURITY * older + SERVICE_MISREAD * key_normal(world_seed ^ 0x5E2_B1A5, p.player.id as u64)
}

const SERVICE_MATURITY: f64 = 1.6;
const SERVICE_MISREAD: f64 = 6.0;

/// How many of a class of 250,000 the services rate at 5, 4, 3 and 2
/// stars (minor bug 17). Five- and four-stars at ESPN's 2013-2026 means,
/// 46.3 and 300.6 a class (`rates.json` recruits_per_star_per_class, the
/// count the world's checks read; minor bug 269: the 247 Composite's 32
/// and 350 put the five-stars far under it), and about 2,600 three-stars
/// signing with FBS, FCS and Division II programs each year (2,200 of them
/// FBS; CFBD's composite ratings of the 2022-2025 signees), with more who
/// sign nowhere rated. The two-stars fill the rated pool to its old size,
/// so who stays in view of the staffs (2+ stars) holds. The old shares
/// (2,000 three-stars and 4,000 two-stars) left SEC squads half 3-stars
/// short and full of 2-stars, which the composite almost never gives an
/// SEC signee. The standalone class sim keeps `PIPELINE.star_shares`.
const SERVICE_TIERS: [(u8, f64); 4] =
    [(5, 46.0 / 250_000.0), (4, 301.0 / 250_000.0), (3, 2_800.0 / 250_000.0), (2, 3_236.0 / 250_000.0)];

/// How the services' standard drifts from class to class (minor bug 102):
/// how many kids a service grades a five- or four-star moves with its
/// evaluators' standard, which wanders and comes back over a few classes
/// (ESPN gave 53-58 five-stars a class in 2013-2015, 27-33 in 2019-2022
/// and 49-68 in 2024-2026; `rates.json` recruits_per_star_per_class). The
/// standard is a slow walk from class to class (this much of last class's
/// lean carries over); each tier's count spreads by its share below, the
/// real spread over the mean (ESPN 2013-2026: five-stars 0.26, four-stars
/// 0.20; three-stars proposed, not confirmed), around the same average
/// counts. The two-stars fill the rated pool to its usual size.
const STANDARD_CARRY: f64 = 0.8;
const STANDARD_SPREAD: [f64; 3] = [0.26, 0.20, 0.10];

/// The class of `class_of`'s star counts at full reach: `SERVICE_TIERS`
/// moved by the services' standard that class (minor bug 102), keyed by the
/// world and the class, so every rating of a class uses the same counts.
fn service_tiers(world_seed: u64, class_of: i32, class: f64) -> [(u8, usize); 4] {
    let mut z = 0.0;
    for k in 0..40 {
        z += STANDARD_CARRY.powi(k) * key_normal(world_seed ^ 0x57A2_DA2D, (class_of - k) as u64);
    }
    z *= (1.0 - STANDARD_CARRY * STANDARD_CARRY).sqrt();
    let rated = (SERVICE_TIERS.iter().map(|t| t.1).sum::<f64>() * class).round() as usize;
    let mut out = [(2u8, 0usize); 4];
    let mut used = 0;
    for k in 0..3 {
        let (stars, share) = SERVICE_TIERS[k];
        let sd = STANDARD_SPREAD[k];
        let n = (share * class * (sd * z - sd * sd / 2.0).exp()).round() as usize;
        out[k] = (stars, n);
        used += n;
    }
    out[3] = (SERVICE_TIERS[3].0, rated.saturating_sub(used));
    out
}

/// The services' read of a kid's offer list (minor bug 17), 0-1: the
/// school that holds his commitment, or before he commits the best school
/// that has offered him, by the level recruits read in it and how big a
/// name it is at that level. Real rankings lean on it: analysts watch the
/// tape of a kid whom Power programs offer, a list of SEC offers moves a
/// kid up a tier while an FCS-only list keeps him below the FBS signees,
/// and a commitment says who really took him (lane 10 follow-up: read as
/// his best offer, a kid who committed to a Group of 5 school with a
/// Power 4 offer stayed a four-star, and the Group of 5's four-stars were
/// no better than the SEC's three-stars). Proposed, not confirmed.
fn offer_sheet(programs: &[Program], p: &Person, year: i32) -> f64 {
    let weight = |j: usize| {
        let g = &programs[j];
        recruit_level(g, year) * (0.5 + 0.5 * g.prestige_rel.clamp(0.0, 1.0))
    };
    if let Some(c) = p.cal.commit.filter(|c| !c.2) {
        return weight(c.0);
    }
    p.cal.offers.iter().copied().chain(p.cal.early_offers.iter().map(|o| o.0)).map(weight).fold(0.0, f64::max)
}

/// What the offer list adds to the services' score, in film standard
/// deviations for a top offer (minor bug 17; proposed, not confirmed).
/// Sized on the 2026 recruiting year of the 30-year world: at 3 the
/// December rating gives the summer's FBS commits 1% five-stars, 10% four,
/// 68% three, 18% two and 2% unrated (the composite's FBS signees of
/// 2022-2025: 1.5%, 10%, 74%, 6% and 8%), and the commits of the biggest
/// names at their level 4%, 27%, 64%, 5% and none (SEC signees: 6%, 32%,
/// 55%, 0.3%). At 1 the big names' commits were 10% two-stars and 5%
/// unrated. Lane 10 follow-up: at 3 the Group of 5 still signed 2.7
/// four-stars a school over four classes (real 0.9) and only 28 three-stars
/// (real 61), while 3-stars went to junior colleges and Division II by the
/// hundreds; the composite follows who offers and who takes a kid more
/// closely than that, so it is 6.
const OFFER_READ: f64 = 6.0;

/// The services' score for a kid (D3, minor bug 17): mostly film and their
/// projection, less for a kid they haven't seen, his tested gifts once he
/// has been to their camps, and his offer list. A kid with real offers gets
/// his tape watched, so the offers lift what the services see of him.
fn star_score(film: f64, projection: f64, visibility: f64, gift: f64, sheet: f64, film_sd: f64) -> f64 {
    let seen = visibility.max(sheet);
    0.25 * film + 0.75 * projection - 12.0 * (1.0 - seen).powi(2)
        + SERVICE_GIFT * film_sd * gift * visibility
        + OFFER_READ * film_sd * sheet
}

/// The stars staffs recruit a player on (fix list 1.5): the services' for
/// most, the camps' own for a kicker, punter or long snapper.
pub(crate) fn recruiting_stars(p: &Person) -> u8 {
    if p.player.position.is_specialist() { p.camp_stars } else { p.stars }
}

/// What a walk-on spot costs a recruit: his family's tuition, as at the
/// Pioneer League schools, which give no athletic aid. Schools that give
/// none anyway (`ncaa::aid`, the season he would arrive) cost no tuition
/// extra. Out of state tuition costs about 1.77 times as much (a public
/// four-year school's full cost, $44,090 against $24,920 in state, College
/// Board 2024-25), so walk-ons are mostly local (fix list 5.2; private
/// schools charge everyone the same, not modeled).
fn walk_on_cost(p: &Person, prog: &Program, season: i32) -> f64 {
    let away = if prog.state == p.home_state { 1.0 } else { 1.77 };
    match super::ncaa::aid(prog, season) {
        super::ncaa::Aid::Athletic => 0.8 * (1.0 - 0.6 * p.bg.money) * away,
        _ => 0.0,
    }
}

/// What a recruit gives up by taking a walk-on place over a scholarship
/// offer he has, besides the money (trial 7 finding 4): a promised place on
/// the team, the last turn at reps, and a staff with nothing invested in
/// him. It weighs by the level of the scholarship he turns down: walking on
/// at a Power Four school over an FCS or Division II scholarship is the
/// common preferred walk-on story, while turning down an FBS scholarship to
/// walk on is rare, so blue-chips with FBS offers sign. Charged only when a
/// walk-on place is weighed against a real scholarship offer; a recruit
/// with none loses nothing. Kickers, punters and snappers often walk on at
/// big programs, so they don't feel it. Proposed, not confirmed.
fn scholarship_given_up(pos: Position, scholarship_at: &Program) -> f64 {
    if pos.is_specialist() { 0.0 } else { WALK_ON_GIVE_UP * level_value(scholarship_at.division) }
}

/// What turning down an FBS scholarship to walk on costs (see above).
const WALK_ON_GIVE_UP: f64 = 1.4;

/// What a starter at his position earns at a major program, middle of the
/// real range (CBS via SI, 2024; `data/real-world/rates.json`). Tight ends
/// and specialists aren't in the data; long snappers take the specialists'
/// rate (proposed, not confirmed).
fn position_nil(pos: Position) -> f64 {
    match pos {
        Position::QB => 650_000.0,
        Position::OT | Position::IOL => 425_000.0,
        Position::EDGE | Position::DT => 425_000.0,
        Position::RB => 250_000.0,
        Position::WR => 187_500.0,
        Position::TE => 150_000.0,
        Position::LB => 200_000.0,
        Position::CB | Position::S => 172_500.0,
        Position::K | Position::P | Position::LS => 25_000.0,
    }
}

/// Buying past the going rate (minor bug 61): a reach program whose money
/// ranks at least this many places above its standing in its division
/// attacks the status quo. Texas Tech, Oregon in its rise and Miami in 2024
/// spent like top-five programs from outside the top 20 (proposed, not
/// confirmed).
const BUY_RANKS: usize = 10;
/// The most a buying school puts into one blue-chip recruit, as a share of
/// its NIL money: Texas Tech's 2025 deal with the 5-star tackle Felix Ojo,
/// reported at $5.1M over three years (about $1.7M a year), is 6% of a
/// $28M year, a guess at a big spender's revenue share and collective
/// (proposed, not confirmed).
const BUY_SHARE: f64 = 0.06;

/// A collective budget that pays major-program market rates.
fn nil_scale(d: Division) -> f64 {
    match d {
        Division::FBS => 10.0e6,
        Division::FCS => 0.4e6,
        _ => 0.1e6,
    }
}

fn level_value(d: Division) -> f64 {
    match d {
        Division::FBS => 1.0,
        Division::FCS => 0.55,
        Division::D2 => 0.3,
        Division::NAIA => 0.25,
        Division::D3 => 0.2,
        Division::JUCO => 0.35,
    }
}

/// A recruit's own feel for each school (dynasty item 11.1): the campus, the
/// people he met, a friend there, how a visit went. It differs from kid to
/// kid and school to school, so two 5-stars rank the same schools
/// differently. The spread (sd, in utility points; the old one was 0.04)
/// is sized so the top program signs about 14% of a class's FBS 5-stars,
/// as in 247Sports' 2024 and 2025 classes (Ohio State 5 of 36; Georgia and
/// Texas 4 of 32), not the 30-36% of long worlds before (proposed, not
/// confirmed).
const FEEL_SD: f64 = 0.07;

/// How deep down each position board a staff recruits all year (11.18):
/// three targets per spot in its plan, plus three, the way real staffs
/// offer about three recruits per place in a class (proposed, not confirmed).
const RECRUITED_PER_SPOT: usize = 3;
const RECRUITED_EXTRA: usize = 3;

/// Official visits a recruit takes: the NCAA's limit of five to June 30,
/// 2023; since July 1, 2023 there is no limit, one a school, and the
/// weekends of his June and his fall hold about five (proposed, not
/// confirmed), so a recruit still saves them for the schools he likes best.
pub(crate) const OFFICIAL_VISITS: usize = 5;

/// The NCAA's cap on a recruit's official visits on a date: five to June
/// 30, 2023, none since (unlimited, one a school, from July 1, 2023).
pub fn official_cap(d: Date) -> Option<usize> {
    (d < Date::ymd(2023, 7, 1)).then_some(OFFICIAL_VISITS)
}

/// Whether any transfer who is academically eligible plays at once: the
/// NCAA's vote of April 17, 2024 freed every transfer for the 2024 season
/// on, those who had already moved included (minor bug B11). A move from
/// September on counts toward the next season.
pub fn multi_transfers_free(day: Date) -> bool {
    let season = if day.month() >= 9 { day.year() + 1 } else { day.year() };
    season >= 2024
}

/// A college player's earlier transfers between four-year schools: the
/// schools on his path before the one he is at now, junior colleges left
/// out (minor bug B11: high school moves don't count).
pub fn four_year_transfers(w: &World, p: &Person) -> u32 {
    let n = p.path.len().saturating_sub(1);
    p.path[..n].iter().filter(|&&c| w.programs.get(c).is_some_and(|x| x.division != Division::JUCO)).count() as u32
}

/// Whether a transfer sits out his first season at his new school. Before
/// the one-time exception (2021) a transfer into FBS sat a year, unless he
/// came from a junior college or, from 2006 (the graduate transfer
/// exception), had graduated. From 2021 a first undergraduate transfer
/// plays at once, but a second one into Division I still sat until April
/// 17, 2024 (minor bug B11).
pub fn transfer_sits(day: Date, one_time: bool, to: Division, from: Division, graduated: bool, before: u32) -> bool {
    let grad_free = graduated && day.year() >= 2006;
    if from == Division::JUCO || grad_free {
        return false;
    }
    if !one_time {
        return to == Division::FBS;
    }
    before >= 1 && matches!(to, Division::FBS | Division::FCS) && !multi_transfers_free(day)
}

/// The FBS's limit on official visitors a school hosts in a cycle, and the
/// cycle's first day (minor bug B2): 56 a year from August 1 to July 31,
/// from April 2020 (Proposal 2019-36) April 1 to March 31, 70 from April
/// 2023; waived from April 2024 and dropped for 2026-27 (Proposal 2026-4).
/// FCS schools have no school limit here (proposed, not confirmed).
pub fn school_official_cap(d: Date) -> Option<(usize, Date)> {
    if d >= Date::ymd(2024, 4, 1) {
        return None;
    }
    let y = d.year();
    if d >= Date::ymd(2020, 4, 1) {
        let start = if d.month() >= 4 { Date::ymd(y, 4, 1) } else { Date::ymd(y - 1, 4, 1) };
        return Some((if start >= Date::ymd(2023, 4, 1) { 70 } else { 56 }, start));
    }
    Some((56, if d.month() >= 8 { Date::ymd(y, 8, 1) } else { Date::ymd(y - 1, 8, 1) }))
}

/// Official visitors each program has hosted since `start`.
pub(crate) fn officials_hosted(w: &World, start: Date) -> Vec<usize> {
    let mut n = vec![0usize; w.programs.len()];
    for p in &w.people {
        for v in &p.cal.visits {
            if v.2 == super::Visit::Official && v.1 >= start && v.0 < n.len() {
                n[v.0] += 1;
            }
        }
    }
    n
}

/// A redshirt's portal rate before his stars (11.17), on the same scale as
/// the backup's 0.18 and the rotation player's 0.07 in `PIPELINE`. These
/// rates go through the portal's bell-shaped draw, so the real yearly
/// chance is far smaller: about 1.1% here against 2.7% for a backup. A
/// redshirt freshman usually waits a year, so he leaves less often than an
/// older backup (proposed, not confirmed).
pub(crate) const REDSHIRT_PORTAL: f64 = 0.12;

/// How much more a recruit cares about playing early, by his stars (dynasty
/// item 11.3): a top recruit expects to play by his second year and leave
/// for the NFL after his third. 1.0 up to two stars, then +0.08 a star, so
/// a 5-star weighs his position room 1.24 times as much. At +0.25 a star
/// 5-stars scattered to 22 programs a class, far more than the real 15-17
/// (proposed, not confirmed).
pub(crate) fn early_weight(stars: u8) -> f64 {
    1.0 + 0.08 * (stars as f64 - 2.0).max(0.0)
}

/// How close a school feels (fix list 2.1): the first few hundred miles, a
/// drive for his parents on game day, count most; past about 800 miles it
/// barely matters. 1 at home, 0.5 at 300 miles, 0.12 at 800 (proposed, not
/// confirmed; it was a straight line out to 1,500 miles).
pub(crate) fn closeness(miles: f64) -> f64 {
    1.0 / (1.0 + (miles / 300.0).powi(2))
}

/// Miles from his home, his high school's town, to a program's campus
/// (minor bug 255). Before, a recruit read every school in his own state as
/// 60 miles off and every other state from its center, so a Jacksonville
/// kid read Miami, about 330 miles away, at 0.96 and Georgia Southern,
/// about 150, at 0.52 (Florida to Georgia's center, 289 miles), and the big
/// states' kids stayed home far more than real (Florida's FBS players in
/// Florida 39-46% against 24, Texas's 60-62% against 47). A town the towns
/// table lacks reads its state's center, as before.
pub(crate) fn home_miles(schools: &[super::high_school::HighSchool], p: &Person, prog: &Program) -> f64 {
    campus_miles(home_place(schools, p.hs, p.home_state), prog)
}

/// Where a recruit lives: his high school's town, or his state's center
/// when the school or its town is unknown.
pub(crate) fn home_place(schools: &[super::high_school::HighSchool], hs: usize, home_state: usize) -> (f64, f64) {
    match schools.get(hs) {
        Some(h) => geo::place(h.state, &h.town),
        None => {
            let (_, lat, lon) = geo::STATE_CENTERS[home_state.min(50)];
            (lat, lon)
        }
    }
}

/// Miles from a place to a program's campus.
pub(crate) fn campus_miles(place: (f64, f64), prog: &Program) -> f64 {
    geo::miles_between(place, geo::campus(prog.state, &prog.city))
}

/// How much a recruit leans toward the school he grew up cheering for
/// (fix list 2.3). Proposed, not confirmed.
const CHILDHOOD_LEAN: f64 = 0.18;

/// Where a recruit's read of a program's name stops growing (dynasty item
/// 8, Cain 2026-10-29): every program at or above 85% of the old leader's
/// name reads the same, and so does the 5-star pull, its cube. Below that a
/// bigger name still counts for more. In test worlds a bend at 75% left
/// the national names small gaps that kept the same three programs on top;
/// the tie, with home fading (minor bug 89), cut the share of seasons the
/// same three held the most talent from 81% to 61% (real 51%).
const NAME_TIE: f64 = 0.85;

/// How much home still pulls a recruit (minor bug 89): closeness, his
/// stay-home lean and the team he grew up with count less as power programs
/// from outside his state offer him. A kid courted from across the country
/// has seen the national stage and weighs more than home; a kid with one
/// out-of-state power offer barely changes. 1 with none, falling toward
/// the floor: `HOME_FADE_FLOOR + (1 - floor) × e^(-offers / HOME_FADE_OFFERS)`.
/// Sized against the real share of FBS signees who stay in their home
/// state, by stars (classes 2010-2025, sportsdataverse `cfb_recruits` matched
/// to cfbfastR rosters): 2-stars 34%, 3-stars 33%, 4-stars 40%, 5-stars 45%.
/// The world's blue chips signed in-state 55-59% before. Calibrated on seven
/// 30-year worlds, blue chips at their signing school in the last two
/// classes (real 41%, 5-stars 45%): no fade 62% (5-stars 73%), a floor of
/// 0.4 over 5 offers 46-47%, 0.5 over 3 offers 43-49%, 0.3 over 4 offers
/// 40-41% (5-stars 46-56%). Proposed, not confirmed.
const HOME_FADE_FLOOR: f64 = 0.3;
const HOME_FADE_OFFERS: f64 = 4.0;

/// How much home pulls him, from the floor to 1 ([`HOME_FADE_FLOOR`]): his
/// offers (the early ones and the last stage's) from power-conference
/// programs outside his home state.
pub(crate) fn home_weight(programs: &[Program], p: &Person, year: i32) -> f64 {
    let mut seen: Vec<usize> = Vec::with_capacity(32);
    for j in p.cal.early_offers.iter().map(|o| o.0).chain(p.cal.offers.iter().copied()) {
        if !seen.contains(&j) {
            seen.push(j);
        }
    }
    let away =
        seen.iter().filter(|&&j| programs[j].state != p.home_state && programs[j].power_conference(year)).count();
    HOME_FADE_FLOOR + (1.0 - HOME_FADE_FLOOR) * (-(away as f64) / HOME_FADE_OFFERS).exp()
}

/// How much a school appeals to a recruit, from the wants of whoever decides (topic 5).
#[allow(clippy::too_many_arguments)]
fn utility(
    norms: &super::systems::SystemNorms,
    schools: &[super::high_school::HighSchool],
    p: &Person,
    wants: &super::background::Wants,
    prog: &Program,
    year: i32,
    nil_offer: f64,
    eval_rank: f64,
    home: f64,
) -> f64 {
    let fits = super::systems::fits(&p.player, p.player.position, norms);
    utility_fit(&fits, schools, p, wants, prog, year, nil_offer, eval_rank, home)
}

/// [`utility`] from his fits to every system, read once (`systems::fits`).
/// `home` is how much home still pulls him ([`home_weight`]; 1 for a
/// transfer).
#[allow(clippy::too_many_arguments)]
fn utility_fit(
    fits: &[f64; 11],
    schools: &[super::high_school::HighSchool],
    p: &Person,
    wants: &super::background::Wants,
    prog: &Program,
    year: i32,
    nil_offer: f64,
    eval_rank: f64,
    home: f64,
) -> f64 {
    let g = |c: GradeCat| prog.grades[c as usize] / 100.0;
    let close = closeness(home_miles(schools, p, prog));
    let crisis = p.bg.story.iter().rev().take(1).any(|s| s.contains("death") || s.contains("lost a job"));
    let close_w = wants.close_to_home + if crisis { 0.15 } else { 0.0 };
    let prestige = (g(GradeCat::TitleChances)
        + g(GradeCat::StadiumAtmosphere)
        + g(GradeCat::Tradition)
        + g(GradeCat::BrandExposure))
        / 4.0;
    // Money on a real scale (note 13): $3.5M is clearly more than $2M. How
    // much it matters is his family's need, which ambition pulls against.
    let money = (nil_offer.max(0.0) / 1.0e6).powf(0.6);
    let need = clamp(0.25 + 0.75 * (1.0 - p.bg.money) + if crisis { 0.2 } else { 0.0 }, 0.1, 1.2);
    let drive = clamp((p.family.ambition - 10.0) / 10.0, -1.0, 1.0);
    let money_w = wants.money * need * (1.0 - 0.35 * drive);
    // What the ambitious kid weighs instead: the best path to the pros.
    let path_w = 0.35 * drive.max(0.0) * wants.money;
    let playing = g(GradeCat::PlayingTime) * (0.6 + 0.4 * eval_rank);
    let academics = g(GradeCat::Academics) * (0.5 + 0.5 * clamp((p.bg.gpa - 2.5) / 1.5, 0.0, 1.0));
    let development = (g(GradeCat::ProPipeline) + g(GradeCat::CoachPrestige)) / 2.0;
    // Every recruit wants the biggest name he can get at a level; how much
    // more depends on what he (or his decider) wants. A name stops growing
    // among the national names (dynasty item 8, `NAME_TIE`): past 85% of the
    // leader of 30 years ago a program is one of the blue bloods, not a
    // bigger one, so a run of titles can't lift it clear of the rest of the
    // elite.
    let brand = prog.prestige_rel.min(NAME_TIE);
    let lv = recruit_level(prog, year);
    let mut u = 1.6 * lv * (0.5 + wants.prestige + wants.development)
        + (0.25 + wants.prestige) * brand * (0.4 + 0.6 * lv)
        + 0.1 * g(GradeCat::Facilities)
        + wants.prestige * prestige
        + money_w * money * 1.5
        + home * close_w * close
        + wants.playing_time * playing
        + wants.academics * academics
        + (wants.development + path_w) * development
        + path_w * playing;
    // No athletic scholarships the season he would arrive (`ncaa::aid`): the
    // Pioneer League gives none (the family pays), the Ivy League and the
    // Patriot League before 2013 only need-based aid. A scholarship offer
    // elsewhere usually wins.
    match super::ncaa::aid(prog, year + 1) {
        super::ncaa::Aid::None => u -= 0.8 * (1.0 - 0.6 * p.bg.money),
        super::ncaa::Aid::NeedBased => u -= 0.4,
        super::ncaa::Aid::Athletic => {}
    }
    // Teammates and older kids from his school already there: a pipeline school feels familiar.
    if let Some(&(v, _)) = prog.hs_pipeline.get(&p.hs) {
        u += 0.12 * v;
    }
    // The program's stories this week (S4): a scandal or a hot seat is
    // rivals' material in his living room, a rising story a lift.
    u += prog.story_push as f64;
    // SK20: a pro day the league's head men come to sells the path to the
    // pros, more to the kid who wants it.
    u += prog.pro_day as f64 * (0.6 + wants.development + path_w);
    // His own pull to stay in his state, and the team he grew up with
    // (fix list 2.2, 2.3; the lean is proposed, not confirmed), both fading
    // as the country courts him (minor bug 89).
    if prog.state == p.home_state {
        u += home * p.stay_home;
    }
    if p.childhood_team == Some(prog.id) {
        u += home * CHILDHOOD_LEAN;
    }
    if p.family.older_brother_at == Some(prog.id) {
        u += if p.family.follows_brother { 0.15 } else { -0.05 };
    }
    // A system that shows him off (S7): a receiver leans to a passing
    // offense, a back to a running one.
    u += SHOWCASE * super::systems::showcase(fits, p.player.position, &prog.installed);
    // A 5-star is courted by everyone but signs with the few programs that
    // win titles and fill first rounds (about 15-17 sign one a year, real
    // 2024-25, 247Sports; dynasty item 11.1, proposed, not confirmed).
    if p.stars == 5 {
        u += FIVE_STAR_PULL * brand.powi(3) * lv;
    }
    u
}

/// How much more a 5-star leans to the very top programs (X1).
const FIVE_STAR_PULL: f64 = 0.25;

/// The level a recruit reads in a school (lane 10 follow-up, minor bug
/// 17): an FBS school outside the power conferences that year is a step
/// below the rest of FBS, with less TV, a narrower way into the playoff
/// and fewer draft picks. Without it a four-star weighed a Group of 5
/// offer like a Power 4 one, separated only by the name, and the Group of
/// 5 signed 4.9 four-stars a school over four classes against a real 0.9
/// (247Sports composite, 2022-25; 230 of 272 had a Power 4 offer).
/// Proposed, not confirmed.
fn recruit_level(prog: &Program, year: i32) -> f64 {
    let lv = level_value(prog.division);
    if prog.division == Division::FBS && !prog.power_conference(year) { lv * GROUP_OF_FIVE_LEVEL } else { lv }
}

/// The Group of 5's level against the rest of FBS (1.0) and the FCS (0.55):
/// proposed, not confirmed.
const GROUP_OF_FIVE_LEVEL: f64 = 0.85;

/// How much a system that shows a recruit off counts, at full fit (S7;
/// a third of the childhood lean, proposed, not confirmed).
const SHOWCASE: f64 = 0.06;

/// A staff's lean to a recruit who suits its systems, in standard
/// deviations of evaluation per unit of fit (S6).
const FIT_EVAL: f64 = 0.3;

/// What each of the services' stars above two is worth on a staff's board,
/// in film standard deviations (lane 10 follow-up, minor bug 17). Without
/// it staffs read only their own looks and passed on a third of the
/// four-stars, who fell to the Group of 5 and the FCS (4.2 and 0.9 a school
/// over four classes against a real 0.9 and 0.1, 247Sports composite
/// 2022-25), though most had two or more Power 4 offers. Proposed, not
/// confirmed.
const STAR_EVAL: f64 = 0.5;

/// What a desperate school (S7, S7a) adds for a timed burner, in film
/// standard deviations, half again with two or more signs (proposed, not
/// confirmed). A head coach is on the hot seat (a sign) from a third of the
/// pressure the staff feels (`pressure`, from results under expectation).
const BURNER_EVAL: f64 = 1.0;
/// How much of its national and overflow reach a staff reads for a recruit
/// outside its region, by his stars (0 to 5; minor bug 246): unrated and
/// 1-star kids a tenth, 2-stars 0.3, 3-stars 0.7, blue chips all. Real
/// staffs find far recruits through the services' lists; before, a staff
/// read every recruit in a far state alike, so 29 of North Dakota State's
/// 52 players from 500 miles or more were unrated Californians, Texans and
/// Arizonans (real: Minnesota, the Dakotas, Wisconsin and Iowa), and Iowa
/// had 48% from 500 miles or more (real 20%): a thin region filled its
/// board from far states. Proposed, not confirmed.
const FAR_READ: [f64; 6] = [0.1, 0.1, 0.3, 0.7, 1.0, 1.0];
const HOT_SEAT: f64 = 0.34;

/// What a recruit or transfer sees of a roster beyond the school's grades
/// (trial 3 item 4, Raichel 2026-10-14): the room at his own position, and
/// the units his position leans on. A QB wants receivers and a line, a corner
/// wants a pass rush. Weights are proposed, not confirmed.
///
/// The room is live (dynasty item 11.2): a recruit who commits, or a
/// transfer who lands, joins it at once, so the next recruit at his position
/// sees him. A recruit who flips leaves it again.
#[derive(serde::Serialize, serde::Deserialize)]
struct RosterView {
    /// Per program and position: (overall, college seasons played, person) of each player.
    players: Vec<Vec<Vec<(f64, u32, usize)>>>,
    /// Per program and position: the starters' average overall.
    unit: Vec<[f64; POSITION_COUNT]>,
    /// Per division and position: the typical starters' average.
    norm: [[f64; POSITION_COUNT]; DIVISION_COUNT],
    division: Vec<usize>,
    /// Per program and position: bumped whenever the room changes, so a
    /// fit read once can be kept until it does.
    ver: Vec<[u32; POSITION_COUNT]>,
}

/// The units each position leans on, with weights.
fn linked(pos: Position) -> &'static [(Position, f64)] {
    use Position::*;
    match pos {
        QB => &[(WR, 0.35), (OT, 0.25), (IOL, 0.2), (TE, 0.2)],
        RB => &[(IOL, 0.45), (OT, 0.35), (TE, 0.2)],
        WR => &[(QB, 0.75), (OT, 0.25)],
        TE => &[(QB, 0.7), (OT, 0.3)],
        OT | IOL => &[(QB, 0.4), (RB, 0.3), (IOL, 0.15), (OT, 0.15)],
        EDGE => &[(DT, 0.5), (CB, 0.3), (LB, 0.2)],
        DT => &[(EDGE, 0.5), (LB, 0.5)],
        LB => &[(DT, 0.6), (EDGE, 0.4)],
        CB => &[(EDGE, 0.5), (S, 0.3), (DT, 0.2)],
        S => &[(CB, 0.5), (LB, 0.3), (EDGE, 0.2)],
        K | P | LS => &[],
    }
}

impl RosterView {
    /// The rosters as they stand, without the players in `gone` (portal entrants on their way out).
    fn new(programs: &[Program], people: &[Person], gone: &std::collections::HashSet<usize>) -> Self {
        let mut players = Vec::with_capacity(programs.len());
        let mut unit = Vec::with_capacity(programs.len());
        let mut sums = [[(0.0, 0.0); POSITION_COUNT]; DIVISION_COUNT];
        for prog in programs {
            let mut r: Vec<Vec<(f64, u32, usize)>> = vec![Vec::new(); POSITION_COUNT];
            for &i in prog.roster.iter().filter(|i| !gone.contains(i)) {
                let q = &people[i];
                r[q.player.position.index()].push((q.player.overall(), q.college_seasons, i));
            }
            let mut u = [0.0; POSITION_COUNT];
            for pos in Position::ALL {
                let v = &mut r[pos.index()];
                v.sort_by(|a, b| b.0.total_cmp(&a.0));
                let n = crate::game::roster::starters(pos).min(v.len());
                if n > 0 {
                    u[pos.index()] = v[..n].iter().map(|x| x.0).sum::<f64>() / n as f64;
                    let e = &mut sums[prog.division as usize][pos.index()];
                    e.0 += u[pos.index()];
                    e.1 += 1.0;
                }
            }
            players.push(r);
            unit.push(u);
        }
        let norm = sums.map(|d| d.map(|(t, n)| if n > 0.0 { t / n } else { 0.0 }));
        let division = programs.iter().map(|p| p.division as usize).collect();
        let ver = vec![[0; POSITION_COUNT]; programs.len()];
        RosterView { players, unit, norm, division, ver }
    }

    /// A commitment or a transfer joins program `j`'s room (11.2).
    fn join(&mut self, j: usize, pos: Position, who: usize, overall: f64, seasons: u32) {
        self.players[j][pos.index()].push((overall, seasons, who));
        self.ver[j][pos.index()] += 1;
    }

    /// A recruit who flips leaves his old room (11.2).
    fn leave(&mut self, j: usize, pos: Position, who: usize) {
        self.players[j][pos.index()].retain(|e| e.2 != who);
        self.ver[j][pos.index()] += 1;
    }

    /// How a program's roster suits him, in utility points. `me` is how good
    /// he expects to be when he wants to start, `within` how many seasons the
    /// players ahead must have left to still be there, and `their_growth`
    /// how much they improve meanwhile. Recruits judge themselves two years
    /// on (about 3 points a year against 1 for older players). `early` is how
    /// much more than usual he cares about playing early (11.3). `who` is
    /// himself at `pos`, left out of the room.
    #[allow(clippy::too_many_arguments)]
    fn fit(
        &self,
        j: usize,
        who: usize,
        pos: Position,
        wants: &super::background::Wants,
        me: f64,
        within: u32,
        their_growth: f64,
        early: f64,
    ) -> f64 {
        let n = crate::game::roster::starters(pos) as f64;
        let ahead = self.players[j][pos.index()]
            .iter()
            .filter(|&&(o, seasons, i)| i != who && seasons <= within && o + their_growth > me)
            .count() as f64;
        let room = if ahead < n { 1.0 - 0.5 * ahead / n } else { -((ahead - n + 1.0) / n).min(1.0) };
        let d = self.division[j];
        let cast: f64 = linked(pos)
            .iter()
            .map(|&(q, wt)| wt * clamp((self.unit[j][q.index()] - self.norm[d][q.index()]) / 4.0, -1.0, 1.0))
            .sum();
        room_weight(wants, early) * room + (0.04 + 0.15 * wants.development) * cast
    }

    /// The quarterbacks between a quarterback recruit and the starting job
    /// at `j` (minor bug 130), in blue-chip seasons: each rival's pedigree
    /// (`QB_PEDIGREE` by his stars; the starter at least `QB_STARTER`)
    /// times the share of the recruit's first three seasons he will still
    /// be there (a classmate or this year's freshman all three, a junior
    /// one, a senior none). One quarterback plays, and a family reads the
    /// depth chart it can see, not true ratings.
    fn qb_rivals(&self, j: usize, who: usize, season: i32, people: &[Person]) -> f64 {
        let room = &self.players[j][Position::QB.index()];
        // Gone already: declared for the draft, or in the portal.
        let gone = |q: &Person| q.stage == Stage::Declared || q.cal.portal.is_some();
        let starter = room
            .iter()
            .filter(|e| e.2 != who && people[e.2].college == Some(j) && !gone(&people[e.2]))
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|e| e.2);
        let mut sum = 0.0;
        for &(_, _, i) in room {
            let q = &people[i];
            if i == who || gone(q) {
                continue;
            }
            let mut pedigree = QB_PEDIGREE[recruiting_stars(q).min(5) as usize];
            if Some(i) == starter {
                pedigree = pedigree.max(QB_STARTER);
            }
            let left = (5 - super::eligibility::school_year(q, season)).clamp(0, 3) as f64 / 3.0;
            sum += pedigree * left;
        }
        sum.min(QB_RIVALS_CAP)
    }

    /// A recruit's room at `j` (`fit`), and for a quarterback his path to
    /// the starting job (minor bug 130).
    #[allow(clippy::too_many_arguments)]
    fn recruit_fit(
        &self,
        j: usize,
        who: usize,
        pos: Position,
        wants: &super::background::Wants,
        me: f64,
        early: f64,
        season: i32,
        people: &[Person],
    ) -> f64 {
        let fit = self.fit(j, who, pos, wants, me, 2, 2.0, early);
        if pos != Position::QB {
            return fit;
        }
        fit - QB_PATH * room_weight(wants, early) * self.qb_rivals(j, who, season, people)
    }
}

/// How much a recruit weighs his room at a program: more when he wants to
/// play, and more for a better recruit (11.3).
fn room_weight(wants: &super::background::Wants, early: f64) -> f64 {
    (0.06 + 0.2 * wants.playing_time) * early
}

/// A quarterback recruit's read of the rivals for the job (minor bug 130):
/// blue-chip quarterbacks pile into rooms that already hold young blue-chip
/// quarterbacks without it (a stacking index of 1.24-1.36 at the 10 programs
/// with the most blue chips, real 0.67; the top five took 21-24% of
/// blue-chip quarterbacks, real 14%; sportsdataverse cfb_recruits,
/// 2005-2025). Pedigree by stars (a four-star half a five-star's), the
/// starter at least half one, at most two rivals' worth, and one knob for
/// the whole read against the room weight (proposed, not confirmed).
const QB_PEDIGREE: [f64; 6] = [0.0, 0.0, 0.0, 0.0, 0.5, 1.0];
const QB_STARTER: f64 = 0.5;
const QB_RIVALS_CAP: f64 = 2.0;
const QB_PATH: f64 = 1.0;

/// What staffs judge a recruit on before they align it by position: his
/// season's play, or for a kicker or punter what the kicking camps measure
/// (fix list 1.4).
pub(crate) fn film_raw(p: &Person, played: &[f64]) -> f64 {
    if p.player.position.kicks() {
        return specialist_grade(&p.player);
    }
    let now = p.last_performance.unwrap_or(p.player.overall());
    let f = if p.stage == Stage::HighSchool { played.get(p.home_state).copied().unwrap_or(1.0) } else { 1.0 };
    if f >= 1.0 {
        return now;
    }
    // Minor bug 20: the season's row is written before its games, so while
    // they are played his film is last season's, giving way week by week.
    let rows = &p.player.history;
    let start = rows.last().map_or(now, |h| h.overall_start);
    let before = rows.len().checked_sub(2).and_then(|k| rows[k].performance.get()).unwrap_or(start);
    let now = p.last_performance.unwrap_or(start);
    (1.0 - f) * before + f * now
}

/// How much of each state's high school regular season has been played
/// (minor bug 20), 1 outside the season: film holds only the games played.
pub(crate) fn hs_played(w: &World) -> Vec<f64> {
    let Some(hs) = w.hs_season.as_ref() else { return vec![1.0; w.states.len()] };
    hs.states
        .iter()
        .map(|st| {
            if st.developed && !st.done {
                st.weeks.iter().filter(|&&g| g < w.today).count() as f64 / st.weeks.len().max(1) as f64
            } else {
                1.0
            }
        })
        .collect()
}

/// Kicking and snapping camps (fix list 1.5, 1.7). Kickers and punters, and
/// each high school's snapper who isn't a ranked prospect at his own
/// position, are seen at camps (more often when the family can pay, as in
/// visibility) and ranked against their own spot by what the camp measures.
/// Stars follow the national rank at the spot (`CAMP_TIERS`) and are the
/// camps' own (`camp_stars`), apart from the services'. Kickers and punters
/// are re-ranked at each rating, the latest standing. Snappers are found
/// once a class, at its first rating (`find_snappers`): a ranked snapper is
/// recruited as a long snapper from then on and keeps those stars.
fn camp_rankings(
    w: &mut World,
    pool: &mut [Recruit],
    seniors: &[usize],
    seed: u64,
    film_scale: (f64, f64),
    find_snappers: bool,
) {
    // Each high school's snapper: the best snapper not at QB, K or P, as on game day.
    // The class that just signed is still on the rosters until June: skip it.
    let class = seniors.iter().map(|&k| w.people[pool[k].person].class_of).max().unwrap_or(i32::MIN);
    let mut snapper: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let schools = if find_snappers { &w.high_schools[..] } else { &[] };
    for s in schools {
        let best = s
            .roster
            .iter()
            .copied()
            .filter(|&i| w.people[i].class_of >= class)
            .filter(|&i| !matches!(w.people[i].player.position, Position::QB | Position::K | Position::P))
            .max_by(|&a, &b| {
                let f = |i: usize| {
                    let p = &w.people[i].player;
                    p.attributes.get(crate::attributes::Attr::LongSnapping)
                        + 0.1 * p.attributes.get(crate::attributes::Attr::FootballIq)
                };
                f(a).total_cmp(&f(b))
            });
        if let Some(i) = best {
            snapper.insert(s.id, i);
        }
    }
    let spots = [Position::K, Position::P, Position::LS];
    let mut lists: [Vec<(f64, usize)>; 3] = Default::default();
    for &k in seniors {
        let i = pool[k].person;
        // A new rating replaces the camps' last one, except a long snapper's.
        if find_snappers || w.people[i].player.position != Position::LS {
            w.people[i].camp_stars = 0;
        }
        let p = &w.people[i];
        let spot = if p.player.position.kicks() {
            p.player.position
        } else if snapper.get(&p.hs) == Some(&i)
            && p.stars == 0
            && !p.regular_talent
            && p.player.body.weight <= crate::athlete::LS_MAX_WEIGHT
        {
            // A 300-pound center who snaps on Fridays projects as a
            // lineman; the camps rank snappers with a snapper's build.
            Position::LS
        } else {
            continue;
        };
        let id = p.player.id as u64;
        if key_uniform(seed ^ 0xCA4D, id) >= 0.3 + 0.7 * p.bg.money {
            continue;
        }
        let grade = if spot == Position::LS {
            crate::positions::overall_for(Position::LS, &p.player.attributes)
        } else {
            specialist_grade(&p.player)
        };
        let s = spots.iter().position(|&x| x == spot).unwrap_or(0);
        lists[s].push((grade + 1.5 * key_normal(seed ^ 0xCA4E, id), k));
    }
    let scale = w.options.scale;
    let mut snappers: Vec<usize> = Vec::new();
    for (s, list) in lists.iter_mut().enumerate() {
        list.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (rank, &(_, k)) in list.iter().enumerate() {
            let tiers = if spots[s] == Position::LS { &SNAP_TIERS } else { &CAMP_TIERS };
            let Some(&(stars, _)) = tiers.iter().find(|t| (rank as f64) < (t.1 * scale).round().max(1.0)) else {
                break;
            };
            let p = &mut w.people[pool[k].person];
            p.camp_stars = stars;
            if spots[s] == Position::LS {
                let from = p.player.position;
                crate::athlete::set_position(&mut p.player, Position::LS);
                let age = p.player.age;
                p.note(format!(
                    "At {age} the snapping camps ranked him a {stars}-star long snapper; colleges recruited him there, not at {from}."
                ));
                snappers.push(k);
            }
        }
    }
    // A new long snapper's film is his snap, aligned like everyone else's,
    // and the services take a new look at his upside.
    let grades: Vec<f64> = snappers.iter().map(|&k| w.people[pool[k].person].player.overall()).collect();
    if grades.len() >= 2 {
        let m = grades.iter().sum::<f64>() / grades.len() as f64;
        let sd = (grades.iter().map(|g| (g - m).powi(2)).sum::<f64>() / grades.len() as f64).sqrt().max(1.0);
        for (&k, &g) in snappers.iter().zip(&grades) {
            pool[k].film = film_scale.0 + film_scale.1 * (g - m) / sd;
        }
    }
    for &k in &snappers {
        let p = &mut w.people[pool[k].person];
        let rep = scout_player(&p.player, ScoutOptions { accuracy: 12.0, level: Level::HS }, &mut p.rng);
        pool[k].projection = rep.potential;
        p.cal.service_read = Some(rep.potential);
    }
}

/// The dated offer passes before a class's first commitment wave (part B
/// 34): after the sophomore season (January), at the summer camps, on
/// September 1 of the junior year (written offers), after the junior
/// season (January) and at junior days. Each pass returns how many offers
/// the prospect at national rank r holds by then: (most, rank scale).
/// Proposed, not confirmed, at full class size: by junior days the top
/// prospect holds about 24, the 100th about 21, the 1,000th about 8 and the
/// 3,000th about 1. Offers to sophomores grew in the 2000s; before 2005 the
/// first three passes carry a quarter of their offers, rising to all of
/// them by 2015.
pub(crate) const EARLY_OFFER_PASSES: [(f64, f64); 5] =
    [(6.0, 40.0), (10.0, 120.0), (15.0, 300.0), (20.0, 600.0), (24.0, 900.0)];

/// One dated offer pass for the class of `class_of` (part B 34). Staffs
/// read the class on film and on what the services see in him, rank it,
/// and each prospect picks up offers up to his rank's count, from the
/// programs that fit him: the best programs offer the best prospects
/// anywhere, others the ones within their reach and footprint. The
/// services' read made here is the one their later ratings drift from.
pub(crate) fn early_offers(w: &mut World, d: Date, class_of: i32, pass: usize) {
    let Some(&(most, scale_r)) = EARLY_OFFER_PASSES.get(pass) else { return };
    let scale = w.options.scale;
    let era = if pass <= 2 { clamp((d.year() - 2000) as f64 / 15.0 + 0.25, 0.25, 1.0) } else { 1.0 };
    let most = most * era;
    // Film first: only the top of the class gets a look.
    let played = hs_played(w);
    // Read on all cores, in person order (world speed lane).
    let mut class: Vec<(f64, usize)> = w
        .people
        .par_iter()
        .enumerate()
        .with_min_len(4096)
        .filter(|(_, p)| p.stage == Stage::HighSchool && p.class_of == class_of && p.cal.signed.is_none())
        .map(|(i, p)| (film_raw(p, &played), i))
        .collect();
    if class.len() < 10 {
        return;
    }
    class.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    class.truncate(((class.len() as f64) * 0.06).ceil() as usize);
    let idx: Vec<usize> = {
        let mut v: Vec<usize> = class.iter().map(|c| c.1).collect();
        v.sort_unstable();
        v
    };
    // The services' look, on each prospect's own stream (all cores).
    let persons = super::pick_mut(&mut w.people, idx.iter().copied());
    let reads: Vec<(usize, f64)> = persons
        .into_par_iter()
        .zip(idx.par_iter())
        .map(|(p, &i)| {
            let rep = scout_player(&p.player, ScoutOptions { accuracy: 12.0, level: Level::HS }, &mut p.rng);
            let read = match p.cal.service_read {
                Some(prev) => prev + 0.5 * (rep.potential - prev),
                None => rep.potential,
            };
            p.cal.service_read = Some(read);
            (i, 0.6 * film_raw(p, &played) + 0.4 * read)
        })
        .collect();
    let mut ranked = reads;
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    // The four-year programs that offer early, best first.
    let mut progs: Vec<(f64, usize)> = w
        .programs
        .iter()
        .filter(|p| matches!(p.division, Division::FBS | Division::FCS) && p.plays(w.year))
        // Your own program offers only whom you offer (lane 10).
        .filter(|p| !fo::yours_program(w, p.id, fo::Duty::Board))
        .map(|p| (p.prestige + if p.division == Division::FBS { 1.0 } else { 0.0 }, p.id))
        .collect();
    progs.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let np = progs.len().max(1) as f64;
    let pct: Vec<(usize, f64)> = progs.iter().enumerate().map(|(k, &(_, j))| (j, 1.0 - k as f64 / np)).collect();
    let seed = w.options.seed as u64 ^ (d.0 as u64) << 18 ^ 0xE0FF;
    let mut posts: Vec<(usize, String, u32, String)> = Vec::new();
    for (r, &(i, _)) in ranked.iter().enumerate() {
        let want = (most * (-(r as f64) / (scale_r * scale)).exp()).floor() as usize;
        let have = w.people[i].cal.early_offers.len();
        if want <= have {
            continue;
        }
        let p = &w.people[i];
        // The level he projects to: the top prospect fits anyone.
        let target = clamp(1.0 - r as f64 / (2500.0 * scale), 0.1, 1.0);
        let id = p.player.id as u64;
        let mut fits: Vec<(f64, usize)> = pct
            .iter()
            .filter(|(j, _)| !p.cal.early_offers.iter().any(|o| o.0 == *j))
            .map(|&(j, pj)| {
                let prog = &w.programs[j];
                let close = closeness(home_miles(&w.high_schools, p, prog));
                let score = -6.0 * (pj - target - 0.05).max(0.0)
                    + 1.5 * prog.footprint[p.home_state]
                    + 0.5 * close
                    + 0.4 * key_normal(seed ^ j as u64, id);
                (score, j)
            })
            .collect();
        fits.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let first = have == 0;
        let new: Vec<usize> = fits.iter().take(want - have).map(|f| f.1).collect();
        let Some(&top) = new.first() else { continue };
        let (name, pos, age) = (p.name.clone(), p.player.position, p.row_age());
        // His grade in the school year the day falls in (it ends in May).
        let school_year_end = d.year() + (d.month() >= 8) as i32;
        let grade = match class_of - school_year_end {
            2.. => "sophomore",
            1 => "junior",
            _ => "senior",
        };
        let p = &mut w.people[i];
        for &j in &new {
            p.cal.early_offers.push((j, d));
        }
        if first {
            let school = w.programs[top].real_name.clone();
            p.note(format!("At {age} he got his first scholarship offer, from {school}, as a {grade}."));
            if (r as f64) < 100.0 * scale {
                posts.push((
                    top,
                    format!("{school} offers {grade} {pos} {name}, his first scholarship offer."),
                    p.player.id,
                    name.clone(),
                ));
            }
        }
    }
    for (j, text, id, name) in posts {
        let e = Event::new(d, CalLevel::Recruiting, Kind::Offer, Loudness::Small, text)
            .program(j)
            .person(id, &name)
            .school_link(j, &w.programs[j].real_name);
        w.calendar.post(e);
    }
}

/// A fired head coach's verbal commitments reconsider on the day he goes
/// (part B 33): many open their recruitment again before the early period,
/// the most wanted most often; a loyal kid, or one who lives close, holds
/// on more. Proposed, not confirmed: 45% of 5-stars, 35% of 4-stars, 25% of
/// 3-stars and 15% of the rest, eased by loyalty and closeness.
pub(crate) fn decommit_after_firing(w: &mut World, prog: usize) {
    let d = w.today;
    let seed = w.options.seed as u64 ^ (d.0 as u64) << 16 ^ 0xDEC0;
    let school = w.programs[prog].real_name.clone();
    let pstate = w.programs[prog].state;
    // Who reopens, read on all cores (each by his own key), then applied in order.
    let reopens: Vec<(usize, u8)> = w
        .people
        .par_iter()
        .enumerate()
        .filter_map(|(i, p)| {
            if p.stage != Stage::HighSchool || p.cal.signed.is_some() || p.cal.commit.is_none_or(|c| c.0 != prog) {
                return None;
            }
            let stars = recruiting_stars(p);
            let base = match stars {
                5 => 0.45,
                4 => 0.35,
                3 => 0.25,
                _ => 0.15,
            };
            let pers = &p.player.personality;
            let loyal = clamp(1.0 - 0.04 * (pers.love_of_game - 10.0) - 0.03 * (pers.coachability - 10.0), 0.5, 1.4);
            let near = if geo::miles(p.home_state, pstate) <= 250.0 { 0.7 } else { 1.0 };
            (key_uniform(seed, p.player.id as u64) < base * loyal * near).then_some((i, stars))
        })
        .collect();
    let mut posts = Vec::new();
    for (i, stars) in reopens {
        let p = &mut w.people[i];
        let (name, pos, age) = (p.name.clone(), p.player.position, p.row_age());
        p.cal.commit = None;
        // He waits to see who is hired and decides at a signing date.
        p.cal.commit_timing = Some(if key_uniform(seed ^ 1, p.player.id as u64) < 0.7 {
            CommitTiming::Early
        } else {
            CommitTiming::Late
        });
        p.note(format!("At {age} he reopened his recruitment when {school} fired its head coach."));
        w.log.decommits += 1;
        w.log.blue_broken += (stars >= 4) as u32;
        if stars >= 4 {
            posts.push((
                format!("{stars}-star {pos} {name} decommits from {school} after the coach's firing."),
                p.player.id,
                name.clone(),
            ));
        }
    }
    // Signees whose head coach is gone (Jan 1 start): from the early
    // signing period to national signing day the ones who ask are mostly
    // let out, as schools do since the NLI ended (proposed, not confirmed).
    if signees_waiting(w, d) {
        let asks: Vec<usize> = (0..w.people.len())
            .filter(|&i| {
                let p = &w.people[i];
                matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)
                    && p.cal.signed.is_some_and(|s| s.program == prog && !s.walk_on)
                    && signee_asks(p, prog, d, seed)
            })
            .collect();
        for i in asks {
            if key_uniform(seed ^ 0x4E11, w.people[i].player.id as u64) >= 0.9 {
                continue;
            }
            let p = &w.people[i];
            let (name, pos, stars, id) = (p.name.clone(), p.player.position, recruiting_stars(p), p.player.id);
            release_signee(w, i, &school);
            w.log.signee_releases += 1;
            if stars >= 4 {
                posts.push((format!("{stars}-star {pos} signee {name} is let out of his signing with {school} after the coaching change."), id, name));
            }
        }
    }
    for (text, id, name) in posts {
        let e = Event::new(d, CalLevel::Recruiting, Kind::Commitment, Loudness::Small, text)
            .program(prog)
            .person(id, &name)
            .school_link(prog, &w.programs[prog].real_name);
        w.calendar.post(e);
    }
}

/// Whether `d` falls between this cycle's early signing period and its
/// national signing day: December signees are signed and can still be let
/// out to sign elsewhere (none before the early period began in 2017).
pub(crate) fn signees_waiting(w: &World, d: Date) -> bool {
    w.dates.early_signing.is_some_and(|e| d >= e) && d < w.dates.signing_day
}

/// A signee's tie to the staff that signed him, 0-1 (Jan 1 start, proposed,
/// not confirmed): how long he was theirs and how often they had him on
/// campus, by his loyalty.
pub(crate) fn signee_tie(p: &Person, prog: usize, today: Date) -> f64 {
    let since = p
        .cal
        .signed
        .filter(|s| s.program == prog)
        .map(|s| s.committed.unwrap_or(s.date))
        .or(p.cal.commit.filter(|c| c.0 == prog).map(|c| c.1))
        .unwrap_or(today);
    let years = (since.days_to(today).max(0) as f64 / 365.0).min(3.0);
    let visits = p.cal.visits.iter().filter(|v| v.0 == prog).count().min(6) as f64;
    let loyal = p.player.personality.traits.get(crate::player::Trait::Loyalty);
    clamp((0.25 * years + 0.08 * visits) * (0.6 + 0.04 * loyal), 0.0, 1.0)
}

/// Whether a December signee asks out once his head coach is gone: about
/// one in six of a class, the best and the closest to the old staff most
/// (proposed, not confirmed).
pub(crate) fn signee_asks(p: &Person, prog: usize, today: Date, seed: u64) -> bool {
    let base = match recruiting_stars(p) {
        5 => 0.30,
        4 => 0.22,
        3 => 0.14,
        _ => 0.08,
    };
    key_uniform(seed ^ 0x5161, p.player.id as u64) < base * (0.5 + signee_tie(p, prog, today))
}

/// Lets a signee out of his signing: he is a recruit again and signs on
/// national signing day.
pub(crate) fn release_signee(w: &mut World, i: usize, school: &str) {
    let p = &mut w.people[i];
    p.cal.signed = None;
    p.cal.commit = None;
    p.cal.commit_timing = Some(CommitTiming::Late);
    let age = p.row_age();
    p.note(format!("At {age} he was let out of his signing with {school} after its head coach left."));
}

/// The services' class rankings on a signing day (part B 32): each class
/// scored the way the composite services do, its best signees counting
/// most (a recruit's points fall off down the class), the top three named.
pub(crate) fn class_rankings_news(w: &mut World, sign_year: i32, label: &str) {
    let mut by_prog: std::collections::HashMap<usize, Vec<f64>> = std::collections::HashMap::new();
    for p in w.people.iter() {
        // Signed, or already on campus as a January enrollee.
        let (program, walk_on) = match (p.cal.signed, p.college) {
            (Some(sg), _) => (sg.program, sg.walk_on),
            (None, Some(c)) if p.stage == Stage::College && p.path.len() == 1 && p.college_seasons == 0 => (c, false),
            _ => continue,
        };
        if p.class_of != sign_year || walk_on || p.player.position.is_specialist() {
            continue;
        }
        // A star's rough composite rating on a 100 scale.
        let rating = match p.stars {
            5 => 99.0,
            4 => 92.0,
            3 => 85.0,
            2 => 78.0,
            _ => 70.0,
        };
        by_prog.entry(program).or_default().push(rating);
    }
    let mut scores: Vec<(f64, usize, [u32; 3])> = by_prog
        .into_iter()
        .map(|(j, mut v)| {
            v.sort_by(|a, b| b.total_cmp(a));
            let score: f64 =
                v.iter().enumerate().map(|(n, r)| r * (-((n as f64).powi(2)) / (2.0 * 10.0f64.powi(2))).exp()).sum();
            let at = |r: f64| v.iter().filter(|&&x| x == r).count() as u32;
            (score, j, [at(99.0), at(92.0), v.len() as u32])
        })
        .collect();
    scores.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    // Each program keeps its class score: an athletic director reads a
    // coach's direction from it (trial 4 topic 4). February's replaces December's.
    for &(score, j, _) in &scores {
        let list = &mut w.programs[j].class_scores;
        match list.last_mut() {
            Some(last) if last.0 == sign_year => last.1 = score,
            _ => list.push((sign_year, score)),
        }
    }
    // The services rank FBS classes; junior colleges and the lower
    // divisions have rankings of their own, which nobody headlines (trial 7
    // finding 13).
    scores.retain(|s| w.programs[s.1].division == Division::FBS);
    if scores.len() < 3 {
        return;
    }
    let names: Vec<String> = scores
        .iter()
        .take(3)
        .map(|(_, j, c)| {
            format!(
                "{} ({}, {}, {})",
                w.programs[*j].real_name,
                count(c[2], "signee"),
                count(c[0], "five-star"),
                count(c[1], "four-star")
            )
        })
        .collect();
    let text =
        format!("{label}: the services' top recruiting classes are 1. {}, 2. {}, 3. {}.", names[0], names[1], names[2]);
    let d = w.today;
    let mut e = Event::new(d, CalLevel::Recruiting, Kind::Signing, Loudness::Big, text).program(scores[0].1);
    for &(_, j, _) in scores.iter().take(3) {
        e = e.school_link(j, &w.programs[j].real_name);
    }
    w.calendar.post(e);
}

/// The day's commitment announcements (T5.2): the verbal commitments
/// decided at the last wave, each on the recruit's own day.
pub(crate) fn announce_commitments(w: &mut World, d: Date) {
    if w.announcements.is_empty() {
        return;
    }
    let (today, later): (Vec<_>, Vec<_>) = w.announcements.iter().copied().partition(|a| a.0 <= d);
    w.announcements = later;
    for (_, i, id, j, walk_on) in today {
        let Some(p) = w.people.get(i) else { continue };
        if p.player.id != id || p.stage != Stage::HighSchool || p.cal.commit.is_some() {
            continue;
        }
        commit_now(w, i, j, d, walk_on);
    }
}

/// A recruit gives his verbal commitment to program `j` on day `d`: his
/// wave's (T5.2), or the day your offer cleared what he needed (the
/// satisfaction bar), when he may be leaving another commitment.
pub(crate) fn commit_now(w: &mut World, i: usize, j: usize, d: Date, walk_on: bool) {
    let month = d.month() as usize - 1;
    let p = &w.people[i];
    let prog_name = w.program_name(j).to_string();
    let flipped = p.cal.commit.filter(|c| c.0 != j).map(|c| w.program_name(c.0).to_string());
    // His senior season: once his state's season has started, the yearly
    // step has aged him, and the note gives that season's age, the age on
    // its row (as the all-state notes do).
    let started =
        w.hs_season.as_ref().is_some_and(|h| h.states.get(w.high_schools[p.hs].state).is_some_and(|s| s.developed));
    // Read as he chose, before his commitment's hold is on him (bug 118).
    let why = w.logging().then(|| super::wlog::why::choice(w, i, j, "commit", walk_on));
    let p = &mut w.people[i];
    p.cal.commit = Some((j, d, walk_on));
    // A flip is counted as a flip, as signing day counts one, not as a
    // second commitment.
    if flipped.is_some() {
        w.log.flips += 1;
        // A blue chip's flip breaks his verbal (minor bug 270).
        w.log.blue_broken += (recruiting_stars(p) >= 4) as u32;
    } else {
        w.log.commits += 1;
        w.log.commits_by_month[month] += 1;
        w.log.blue_verbal += ((2..=10).contains(&month) && recruiting_stars(p) >= 4) as u32;
    }
    let age = p.player.age - started as u32;
    let stars = p.stars;
    let name = p.name.clone();
    let when = if started || d.month() >= 9 { "during his senior season" } else { "before his senior season" };
    p.note(match &flipped {
        Some(old) => format!("Flipped his commitment from {old} to {prog_name} at {age}, {when}."),
        None => format!("Committed to {prog_name} at {age}, {when}."),
    });
    if stars >= 4 {
        let pos = w.people[i].player.position;
        let text = match &flipped {
            Some(old) => format!("{stars}-star {pos} {name} flips from {old} to {prog_name}."),
            None => format!("{stars}-star {pos} {name} commits to {prog_name}."),
        };
        let loud = if stars == 5 { Loudness::Medium } else { Loudness::Small };
        let id = w.people[i].player.id;
        let e = Event::new(d, CalLevel::Recruiting, Kind::Commitment, loud, text)
            .program(j)
            .person(id, &name)
            .school_link(j, &prog_name);
        w.calendar.post(e);
    }
    if let Some(why) = why {
        w.log_why(why);
    }
}

/// Whether a recruit of class `class_of` may take an official visit on day
/// `d` of football year `season` (T5, NCAA rules): only the class signing
/// next, from the first day of classes of his senior year (September 1
/// here) in every era, and from 2018 (`junior_officials`) also in the
/// junior spring, April 1 to the end of June (the dead period from late
/// June closes it in practice). Nobody else takes one.
pub(crate) fn official_visit_allowed(season: i32, d: Date, class_of: i32, junior_officials: bool) -> bool {
    if class_of != season + 1 {
        return false;
    }
    if d >= Date::ymd(season, 9, 1) {
        return true;
    }
    junior_officials && d >= Date::ymd(season, 4, 1) && d <= Date::ymd(season, 6, 30)
}

/// What a recruiting day is (`recruiting_day`), for each recruit's plan.
struct VisitDay<'a> {
    d: Date,
    season: i32,
    sign_year: i32,
    junior_officials: bool,
    fall: bool,
    official_day: bool,
    game_day: bool,
    drop_day: bool,
    junior_day: bool,
    home_day: bool,
    fall_officials: bool,
    seed: u64,
    vj: Option<usize>,
    planned: Option<&'a (usize, std::collections::HashSet<u32>)>,
    home_today: &'a std::collections::HashSet<usize>,
    home_tomorrow: &'a std::collections::HashSet<usize>,
}

/// A recruit's day of visits (`plan_visits`): the offers whose official
/// visit he would take, then his other visits.
type VisitPlan = (Vec<usize>, Vec<(usize, super::Visit)>);

/// One recruit's visits today (`recruiting_day`): the offers whose official
/// visit he would take, in order, before the school cap is counted (none
/// unless his roll and his own cap say he takes one), and his other visits
/// in the order they go in. None when he has no day.
fn plan_visits(w: &World, i: usize, c: &VisitDay) -> Option<VisitPlan> {
    use super::Visit;
    let VisitDay {
        d,
        season,
        sign_year,
        junior_officials,
        fall,
        official_day,
        game_day,
        drop_day,
        junior_day,
        home_day,
        fall_officials,
        seed,
        vj,
        planned,
        home_today,
        home_tomorrow,
    } = *c;
    let p = &w.people[i];
    if p.stage != Stage::HighSchool
        || p.cal.signed.is_some()
        || (p.cal.offers.is_empty() && p.cal.early_offers.is_empty())
        || p.class_of > sign_year + 1
    {
        return None;
    }
    if junior_day {
        // Junior days are for the class signing next (in the spring of
        // the football year, the juniors).
        if p.class_of != sign_year {
            return None;
        }
        let id = p.player.id as u64;
        let pick = p.cal.early_offers.iter().map(|o| o.0).find(|&j| {
            Some(j) != vj
                && geo::miles(p.home_state, w.programs[j].state) <= 500.0
                && !p.cal.visits.iter().any(|v| v.0 == j)
                && key_uniform(seed ^ 4 ^ j as u64, id) < 0.3
        });
        return pick.map(|j| (Vec::new(), vec![(j, Visit::Unofficial)]));
    }
    // The class signing next: seniors from September, juniors before.
    let senior = p.class_of == sign_year && fall;
    let id = p.player.id as u64;
    let mut official: Vec<usize> = Vec::new();
    let mut new: Vec<(usize, Visit)> = Vec::new();
    if official_day && official_visit_allowed(season, d, p.class_of, junior_officials) {
        // The official you set and he accepted holds one of his five.
        // Counted by school: sandbox mode's contact rules can give your
        // program more than one, and they hold a single place.
        let vs = &p.cal.visits;
        let taken = (0..vs.len())
            .filter(|&k| {
                vs[k].2 == Visit::Official && !vs[..k].iter().any(|u| u.0 == vs[k].0 && u.2 == Visit::Official)
            })
            .count()
            + usize::from(planned.is_some_and(|(pj, ids)| {
                ids.contains(&p.player.id) && !vs.iter().any(|u| u.0 == *pj && u.2 == Visit::Official)
            }));
        let yours_set = |j: usize| planned.is_some_and(|(pj, ids)| *pj == j && ids.contains(&p.player.id));
        if official_cap(d).is_none_or(|cap| taken < cap) && key_uniform(seed ^ 1, id) < 0.12 {
            official = p
                .cal
                .offers
                .iter()
                .copied()
                .filter(|&j| {
                    Some(j) != vj
                        && !yours_set(j)
                        && !p.cal.visits.iter().any(|v| v.0 == j && v.2 == Visit::Official)
                        && (!fall_officials || home_tomorrow.contains(&j))
                })
                .collect();
        }
    }
    if game_day {
        for &j in p.cal.offers.iter().filter(|&&j| home_today.contains(&j) && Some(j) != vj) {
            if geo::miles(p.home_state, w.programs[j].state) <= 250.0 && key_uniform(seed ^ 2 ^ j as u64, id) < 0.25 {
                new.push((j, Visit::Unofficial));
                break;
            }
        }
    }
    if drop_day && senior {
        for &j in p.cal.offers.iter().filter(|&&j| Some(j) != vj) {
            let committed = p.cal.commit.is_some_and(|c| c.0 == j);
            let near = geo::miles(p.home_state, w.programs[j].state) <= 250.0;
            if (near || committed) && key_uniform(seed ^ 3 ^ j as u64, id) < if committed { 0.4 } else { 0.2 } {
                new.push((j, Visit::DropIn));
            }
        }
    }
    if home_day && p.class_of == sign_year {
        for &j in p.cal.offers.iter().filter(|&&j| Some(j) != vj) {
            if p.cal.visits.iter().any(|v| v.0 == j && v.2 == Visit::Home) {
                continue;
            }
            let committed = p.cal.commit.is_some_and(|c| c.0 == j);
            let near = geo::miles(p.home_state, w.programs[j].state) <= 400.0;
            let chance = if committed {
                0.08
            } else if near && p.stars >= 3 {
                0.015
            } else {
                0.0
            };
            if chance > 0.0 && key_uniform(seed ^ 5 ^ (j as u64) << 8, id) < chance {
                new.push((j, Visit::Home));
            }
        }
    }
    (!official.is_empty() || !new.is_empty()).then_some((official, new))
}

/// The day's recruiting actions (T5.3, T5.4), only where the NCAA period
/// allows them: official visits on Fridays outside dead periods, for the
/// class signing next once it may take them (`official_visit_allowed`); in
/// the fall they go to a program at home that weekend, the way fall
/// officials are built around home games. Game-day unofficial visits to
/// nearby programs that offered, and staff drop-ins at high school games in
/// evaluation weeks, most of all for their own commitments. Proposed, not
/// confirmed: a weekend official visit chance of 12% for a recruit with
/// offers left to see, 25% for a nearby home game, 20% a week for a drop-in
/// (40% for a commitment). Head coaches' home visits in the winter contact
/// periods (lane 14): 8% a day for a commitment, 1.5% for a near three-star
/// or better holding an offer, also proposed, not confirmed.
pub(crate) fn recruiting_day(w: &mut World, d: Date) {
    use super::Visit;
    use crate::calendar::Weekday;
    let y = &w.dates;
    let period = crate::calendar::recruiting_period(y, d);
    let wd = d.weekday();
    let fall = (9..=11).contains(&d.month());
    let official_day = wd == Weekday::Fri && period.official_visits();
    let game_day = wd == Weekday::Sat && period.unofficial_visits() && fall;
    let drop_day = wd == Weekday::Fri && period == crate::calendar::Period::Evaluation;
    // Junior days (part B 34): Saturdays in February and March, juniors
    // visit the schools that offered them early.
    let junior_day = wd == Weekday::Sat && d >= y.junior_days.0 && d <= y.junior_days.1;
    // Head coaches sit in seniors' living rooms in the winter contact
    // periods (lane 14, D7): their commits and the near prospects they
    // offered, one visit a school.
    let home_day = period == crate::calendar::Period::Contact && wd != Weekday::Sun && matches!(d.month(), 11 | 12 | 1);
    if !(official_day || game_day || drop_day || junior_day || home_day) {
        return;
    }
    let season = y.season;
    let sign_year = season + 1;
    let junior_officials = w.changes.rules(season).junior_officials;
    // Programs at home on a day.
    let home_on = |day: Date| -> std::collections::HashSet<usize> {
        w.college
            .as_ref()
            .map_or_else(Default::default, |c| c.games.iter().filter(|g| g.date == day).map(|g| g.home).collect())
    };
    let home_today = if game_day { home_on(d) } else { Default::default() };
    // A fall official visit is the weekend of a home game: Friday night in,
    // Saturday's game, home on Sunday.
    let fall_officials = official_day && fall;
    let home_tomorrow = if fall_officials { home_on(d.add(1)) } else { Default::default() };
    let seed = w.options.seed as u64 ^ (d.0 as u64) << 20 ^ 0x5649;
    // Your program's visits are the ones you set (lane 10): nobody comes on his own.
    let vj = fo::college::visits_program(w);
    let planned = fo::college::planned_officials(w);
    // The FBS school limit on official visitors, where one applied (B2).
    let school_cap = if official_day { school_official_cap(d) } else { None };
    let mut hosted = school_cap.map(|(_, start)| officials_hosted(w, start));
    let capped: Vec<bool> = if school_cap.is_some() {
        w.programs.iter().map(|p| p.division == Division::FBS).collect()
    } else {
        Vec::new()
    };
    let mut posts = Vec::new();
    let mut homes = Vec::new();
    let (mut off, mut unoff, mut drops) = (0u32, 0u32, 0u32);
    // Each recruit's day planned on all cores (world speed lane): his own
    // visits and keyed rolls only. The official visit's school cap counts
    // the visitors taken before him, so his pick among the offers that pass
    // everything else is made here, in person order, as the visits go in.
    let day = VisitDay {
        d,
        season,
        sign_year,
        junior_officials,
        fall,
        official_day,
        game_day,
        drop_day,
        junior_day,
        home_day,
        fall_officials,
        seed,
        vj,
        planned: planned.as_ref(),
        home_today: &home_today,
        home_tomorrow: &home_tomorrow,
    };
    let world: &World = w;
    let plans: Vec<(usize, VisitPlan)> = (0..w.people.len())
        .into_par_iter()
        .with_min_len(4096)
        .filter_map(|i| plan_visits(world, i, &day).map(|plan| (i, plan)))
        .collect();
    for (i, (official, rest)) in plans {
        let mut new: Vec<(usize, Visit)> = Vec::new();
        if let Some(&j) = official
            .iter()
            .find(|&&j| !school_cap.is_some_and(|(cap, _)| capped[j] && hosted.as_ref().is_some_and(|h| h[j] >= cap)))
        {
            new.push((j, Visit::Official));
        }
        new.extend(rest);
        if new.is_empty() {
            continue;
        }
        let p = &w.people[i];
        let stars = p.stars;
        let name = p.name.clone();
        let p = &mut w.people[i];
        for (j, v) in new {
            p.cal.visits.push((j, d, v));
            match v {
                Visit::Official => {
                    off += 1;
                    if let Some(h) = hosted.as_mut() {
                        h[j] += 1;
                    }
                    if stars >= 4 {
                        posts.push((
                            j,
                            format!("{stars}-star {name} takes an official visit to {}.", w.programs[j].real_name),
                            p.player.id,
                            name.clone(),
                        ));
                    }
                }
                Visit::Home => {
                    homes.push((i, j));
                    drops += 1;
                }
                Visit::Unofficial => unoff += 1,
                Visit::DropIn => drops += 1,
            }
        }
    }
    w.log.official_visits += off;
    w.log.unofficial_visits += unoff;
    w.log.drop_ins += drops;
    for (i, j) in homes {
        super::folk::dialogue::ai_home_visit(w, i, j, d);
    }
    for (j, text, id, name) in posts {
        w.calendar.post(
            Event::new(d, CalLevel::Recruiting, Kind::Visit, Loudness::Small, text)
                .program(j)
                .rule("NCAA: official visits")
                .person(id, &name)
                .school_link(j, &w.programs[j].real_name),
        );
    }
}

/// The recruiting stages of a class (T5): summer commitments before senior
/// seasons, the early signing period in December (from the class of 2018)
/// or, before it, January enrollees signing aid agreements, and February
/// signing day. Each stage runs the same matching on the recruits still
/// unsigned; only the recruits whose own timing (T5.2) has come decide.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum SignStage {
    /// A commitment wave before the senior season ends (T5.2): 0 to 3.
    Summer(u8),
    Early,
    JanuaryEnrollees,
    Final,
    /// A late signing day after February's (lane B): each Wednesday until
    /// summer enrollment, the recruits still looking sign where a place has
    /// opened; `true` on enrollment day itself, the last, when whoever is
    /// still unsigned goes to prep school, a junior college or stops.
    Late(bool),
}

/// The late signing days after February's signing day (lane B, the Jan 1
/// start): the class still signing, its last day (summer enrollment) and
/// the film read signing day made, so a late recruit is read as he was
/// then, against his whole class.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LateSigning {
    pub(crate) sign_year: i32,
    /// February's signing day, and summer enrollment, the last late day.
    pub(crate) from: Date,
    pub(crate) last: Date,
    pos: Vec<(f64, f64)>,
    all: (f64, f64),
}

/// Whether a recruit nobody signed in February keeps looking for a place
/// until summer enrollment (lane B): a senior, prep school or junior
/// college player some staff had on its board or offered. Everyone else
/// stops at signing day, as before, so the late pool stays the size of the
/// real late market (proposed, not confirmed).
fn keeps_looking(on_board: bool, offered: bool) -> bool {
    on_board || offered
}

fn month_name(m: u32) -> &'static str {
    const M: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    M[(m.clamp(1, 12) - 1) as usize]
}

/// When a recruit makes up his mind (T5.2), from his personality and family:
/// a kid who decides for himself and loves the game commits early; a family
/// that enjoys the attention, or a handler shopping him, waits; an older
/// brother at a school pulls a commitment early. A 4-star with offers in
/// hand commits sooner than most (part D 11: about 60-70% of a real top 300
/// is committed before the senior season); a 5-star can afford to wait, and
/// a lower recruit's offers come later. Proposed, not confirmed, for a
/// 4-star: 8% commit after junior days, 14% after the spring evaluation
/// period, 30% after June's camps and visits (52% before the senior season,
/// with those waiting on an offer pushed later), 10% at the end of August,
/// 8% in the fall, 12% at the early period, 8% wait for February.
fn commit_timing(p: &Person, seed: u64, juco: bool) -> CommitTiming {
    let id = p.player.id as u64;
    if juco {
        return if key_uniform(seed ^ 0x7132, id) < 0.6 { CommitTiming::Early } else { CommitTiming::Late };
    }
    let f = &p.family;
    let z = 0.8 * key_normal(seed ^ 0x7131, id) + if f.decider == Decider::Himself { 0.25 } else { 0.0 }
        - if f.parent_style == super::background::ParentStyle::SocialMedia { 0.3 } else { 0.0 }
        - if f.decider == Decider::Handler { 0.3 } else { 0.0 }
        + 0.03 * (p.player.personality.love_of_game - 10.0)
        + if f.older_brother_at.is_some() { 0.15 } else { 0.0 }
        + match recruiting_stars(p) {
            4 => 0.15,
            5 | 3 => 0.0,
            _ => -0.2,
        };
    if z > 1.34 {
        CommitTiming::Summer(0)
    } else if z > 0.81 {
        CommitTiming::Summer(1)
    } else if z > -0.11 {
        CommitTiming::Summer(2)
    } else if z > -0.35 {
        CommitTiming::Summer(3)
    } else if z > -0.57 {
        CommitTiming::Summer(4)
    } else if z > -1.1 {
        CommitTiming::Early
    } else {
        CommitTiming::Late
    }
}

/// Why a recruit ready to sign in December waits for February instead
/// (part D 14; real FBS classes sign about 85-90% in December since 2018):
/// his school changed head coaches this cycle and he wants to meet the new
/// staff, his grades or test score are still short of the line, or a
/// 5-star's family saves the choice for a signing-day show. Proposed, not
/// confirmed: half of those with a new coach, 60% of borderline students,
/// 40% of those 5-stars, and 3% of everyone else.
fn waits_for_february(p: &Person, prog: Option<&Program>, gpa: f64, seed: u64, year: i32) -> bool {
    let id = p.player.id as u64;
    let u = key_uniform(seed ^ 0xFEB, id);
    let new_coach = prog.is_some_and(|prog| prog.hc_changed == year);
    // Division I's line: 2.3 from the class of 2016, 2.0 before.
    let line = if year + 1 >= 2016 && prog.is_some_and(|prog| matches!(prog.division, Division::FBS | Division::FCS)) {
        2.3
    } else {
        2.0
    };
    let borderline = eligibility_gpa(gpa, p.cal.test_score, year + 1) < line + 0.1;
    let show = recruiting_stars(p) == 5
        && (p.family.parent_style == super::background::ParentStyle::SocialMedia
            || p.family.decider == Decider::Handler);
    let chance = if new_coach {
        0.5
    } else if borderline {
        0.6
    } else if show {
        0.4
    } else {
        0.03
    };
    u < chance
}

/// The share of signees who enroll in January (proposed, not confirmed):
/// a few in the 1990s, about 15% of FBS signees by 2016, then 30-45% with
/// the early signing period; blue-chips and good students most often.
fn early_enrollee_chance(sign_year: i32, stars: u8, gpa: f64, division: Division) -> f64 {
    let era = match sign_year {
        ..=1999 => 0.03,
        2000..=2017 => 0.03 + 0.12 * (sign_year - 2000) as f64 / 17.0,
        _ => (0.30 + 0.025 * (sign_year - 2018) as f64).min(0.45),
    };
    let level = match division {
        Division::FBS => 1.0,
        Division::FCS => 0.4,
        _ => 0.0,
    };
    let stars = match stars {
        5 => 1.6,
        4 => 1.3,
        3 => 1.0,
        _ => 0.6,
    };
    let grades = if gpa >= 2.6 { 1.0 } else { 0.35 };
    clamp(era * level * stars * grades, 0.0, 0.9)
}

/// How hard a committed recruit holds to his school (T5.1), in utility
/// points: loyal kids hold on; a head coach leaving since he committed
/// halves it.
fn stickiness(p: &Person, prog: &Program, year: i32) -> f64 {
    let pers = &p.player.personality;
    let loyal = clamp(0.6 + 0.04 * (pers.love_of_game - 10.0) + 0.03 * (pers.coachability - 10.0), 0.3, 1.0);
    let changed = prog.hc_changed == year;
    0.18 * loyal * if changed { 0.5 } else { 1.0 }
}

/// A signing stage for the class that signs after `season` (T5). Its
/// counters (commitments, flips, decommits, signees) go to that season's
/// calendar log even when February signing day falls after the Super Bowl,
/// in the next football year, so each log holds one signing day. Its news
/// (commitments, flips, signings) is filed under that season too, while it
/// still shows on the day it happened.
pub(crate) fn signing_stage(w: &mut World, stage: SignStage, season: i32) {
    let past = if w.log.year == season { None } else { w.log_history.iter().rposition(|l| l.year == season) };
    if let Some(k) = past {
        std::mem::swap(&mut w.log, &mut w.log_history[k]);
        w.calendar.filing = Some(season);
    }
    run_signing_stage(w, stage, season);
    if let Some(k) = past {
        std::mem::swap(&mut w.log, &mut w.log_history[k]);
        w.calendar.filing = None;
    }
}

/// A late signing day (lane B): each Wednesday after February's signing
/// day until summer enrollment, and enrollment day itself, the recruits
/// still looking sign where a place has opened, by February's matching.
/// The counts go in this football year's log.
impl LateSigning {
    /// Whether a late recruit decides from this late day on (lane B): each
    /// takes his own week between signing day and summer enrollment, from
    /// one key, the way real spring decisions come as aid packages land
    /// (the Division III deposit day is May 1, junior college classes
    /// fill in summer). Spread evenly, proposed, not confirmed: on one
    /// day, seed 74's whole late class signed the first Wednesday.
    fn ready(&self, p: &Person, today: Date, seed: u64) -> bool {
        let weeks = (self.from.days_to(self.last) / 7).max(1) as f64;
        let week = 1.0 + (key_uniform(seed ^ 0x1A7E_5EED, p.player.id as u64) * weeks).floor();
        self.from.days_to(today) as f64 >= 7.0 * week - 6.0
    }
}

pub(crate) fn late_signing_day(w: &mut World) {
    let Some(l) = &w.late_signing else { return };
    let d = w.today;
    let (from, last, sign_year) = (l.from, l.last, l.sign_year);
    // Enrollment day closes the search, or the first day after it if a
    // world reaches it some other way.
    let closing = d >= last;
    if d <= from || (!closing && d.weekday() != crate::calendar::Weekday::Wed) {
        return;
    }
    run_signing_stage(w, SignStage::Late(closing), sign_year - 1);
    if closing {
        w.late_signing = None;
    }
}

/// Offers read one by one before the matching reads the rest of every
/// board at once on all cores (trial 8 finding 10): the busy stages make
/// about 2.5 million offers, deep down whole boards, the quiet ones about
/// 40,000, fewer than reading every board would cost.
const READ_ALL_AFTER: usize = 100_000;

/// How visible a recruit is to the services and the staffs (A12, A32): his
/// school's exposure, his coach's network, the camps his family can pay
/// for, his tape, his state's football traffic, a viral clip, a father who
/// played, all-state honors and his high school coach's calls. Returns the
/// visibility and whether a clip of his went viral.
fn recruit_visibility(w: &World, p: &Person, film: f64, all_mean_sd: (f64, f64), seed: u64, promo: f64) -> (f64, bool) {
    let juco_mover = p.stage == Stage::College;
    let hs = &w.high_schools[p.hs];
    let school_exposure = if juco_mover {
        0.7
    } else if p.stage == Stage::PrepSchool {
        0.6
    } else {
        hs.prestige
    };
    let network = if juco_mover {
        p.college.and_then(|c| w.programs[c].head_coach).map_or(0.5, |c| w.coaches[c].ability().network / 20.0)
    } else {
        hs.coach.map_or(0.4, |c| w.coaches[c].ability().network / 20.0)
    };
    // The camps and showcases he has tested at (S6d): none reads as the
    // old floor, two or more as full.
    let camps = 0.3 + 0.35 * super::times::tested_at(w, p.player.id).min(2) as f64;
    let tape = clamp((film - all_mean_sd.0) / (3.0 * all_mean_sd.1) + 0.3, 0.0, 1.0);
    let traffic = 0.1 * (w.states[p.home_state].culture + 0.6);
    let viral_odds = if p.family.parent_style == super::background::ParentStyle::SocialMedia { 0.06 } else { 0.02 };
    // A viral clip gets him a first look only: staffs still judge his film.
    let went_viral = !juco_mover
        && key_normal(seed ^ 11, p.player.id as u64) > 0.0
        && key_normal(seed ^ 13, p.player.id as u64).abs() < viral_odds * 2.5;
    let viral = if went_viral { 0.25 } else { 0.0 };
    let bloodline = match p.family.bloodline {
        super::background::Bloodline::NflPlayer => 0.15,
        super::background::Bloodline::CollegePlayer => 0.05,
        super::background::Bloodline::None => 0.0,
    };
    // All-state honors (A32) put a name in front of every staff in the state.
    let honors = 0.04 * p.cal.all_state.min(2) as f64;
    let visibility = clamp(
        0.05 + 0.3 * school_exposure
            + 0.2 * network
            + 0.15 * camps
            + 0.25 * tape
            + traffic
            + viral
            + bloodline
            + honors
            + promo,
        0.0,
        1.0,
    );
    (visibility, went_viral)
}

/// The services rank an underclass (update 2, minor bug 12): from a kid's
/// sophomore year they put out stars for the classes behind the one
/// signing, at their winter, spring, summer and fall updates. They look
/// at the kids whose film stands out in the class and the ones the
/// colleges have already offered or the state has honored, read each the
/// way they read a senior (film and their projection, drifting halfway
/// toward each new look), and hand out the stars of a senior class scaled
/// by `reach`, the share of each of the 5, 4, 3 and 2 star places a class
/// this young fills. Each read runs on its own keyed stream, so the kids'
/// own streams are not touched.
pub(crate) fn rate_underclass(w: &mut World, class_of: i32, d: Date, reach: [f64; 4]) {
    let world_seed = w.options.seed as u64;
    let seed = world_seed ^ (class_of as u64) << 20;
    let mut pos_stats: [(f64, f64, f64); POSITION_COUNT] = [(0.0, 0.0, 0.0); POSITION_COUNT];
    let mut members: Vec<usize> = Vec::new();
    let played = hs_played(w);
    // The class's film read on all cores, then summed in person order, so
    // the sums are the same as one walk (world speed lane).
    let films: Vec<(usize, usize, f64)> = w
        .people
        .par_iter()
        .enumerate()
        .with_min_len(4096)
        .filter(|(_, p)| p.stage == Stage::HighSchool && p.class_of == class_of && p.cal.signed.is_none())
        .map(|(i, p)| (i, p.player.position.index(), film_raw(p, &played)))
        .collect();
    for (i, k, f) in films {
        let s = &mut pos_stats[k];
        s.0 += 1.0;
        s.1 += f;
        s.2 += f * f;
        members.push(i);
    }
    if members.len() < 10 {
        return;
    }
    let pos_mean_sd: Vec<(f64, f64)> = pos_stats
        .iter()
        .map(|&(k, s, s2)| {
            if k < 2.0 {
                (45.0, 6.0)
            } else {
                let m = s / k;
                (m, ((s2 / k - m * m).max(1.0)).sqrt())
            }
        })
        .collect();
    let all_mean_sd = {
        let (k, s, s2) = pos_stats.iter().fold((0.0, 0.0, 0.0), |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2));
        let m = s / k.max(1.0);
        (m, ((s2 / k.max(1.0) - m * m).max(1.0)).sqrt())
    };
    let film_of = |p: &Person| {
        let (m, sd) = pos_mean_sd[p.player.position.index()];
        all_mean_sd.0 + all_mean_sd.1 * (film_raw(p, &played) - m) / sd
    };
    // Who they look at: the top of the class on film, and every kid a
    // college offered, the state honored, the camps ranked or they rated
    // before.
    let mut by_film: Vec<(f64, usize)> =
        members.par_iter().with_min_len(256).map(|&i| (film_of(&w.people[i]), i)).collect();
    by_film.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let top = ((by_film.len() as f64) * 0.08).ceil() as usize;
    let mut looked: Vec<usize> = by_film.iter().take(top).map(|x| x.1).collect();
    for &(_, i) in by_film.iter().skip(top) {
        let p = &w.people[i];
        if p.stars > 0
            || p.camp_stars > 0
            || p.cal.all_state > 0
            || !p.cal.offers.is_empty()
            || !p.cal.early_offers.is_empty()
            || p.cal.commit.is_some()
        {
            looked.push(i);
        }
    }
    // S5: the class's fastest timed kids, whatever their film.
    let mut class_n = [0.0; POSITION_COUNT];
    for (n, s) in class_n.iter_mut().zip(&pos_stats) {
        *n = s.0;
    }
    let floors = super::times::speed_floors(w, &members, &class_n);
    looked.extend(floors.keys().copied());
    looked.sort_unstable();
    looked.dedup();
    let promos = super::scouting::hs::promos(w);
    let wr: &World = w;
    let scored: Vec<(usize, f64, f64)> = looked
        .par_iter()
        .map(|&i| {
            let p = &wr.people[i];
            let film = film_of(p);
            let promo = promos.get(&p.player.id).copied().unwrap_or(0.0);
            let (vis, _) = recruit_visibility(wr, p, film, all_mean_sd, seed, promo);
            let mut rng = Rng::new(p.player.id ^ (d.0 as u32).rotate_left(11) ^ 0x51A7_0000);
            let rep = scout_player(&p.player, ScoutOptions { accuracy: 12.0, level: Level::HS }, &mut rng);
            let seen = rep.potential + service_bias(p, world_seed);
            let read = match p.cal.service_read {
                Some(prev) => prev + 0.5 * (seen - prev),
                None => seen,
            };
            let gift = crate::links::gift(&super::times::seen_links(p, &super::times::seen(wr, p)));
            let sheet = offer_sheet(&wr.programs, p, wr.year);
            (i, read, star_score(film, read, vis, gift, sheet, all_mean_sd.1))
        })
        .collect();
    for &i in &members {
        w.people[i].stars = 0;
    }
    for &(i, read, _) in &scored {
        w.people[i].cal.service_read = Some(read);
    }
    let mut order: Vec<&(usize, f64, f64)> = scored.iter().collect();
    order.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)));
    let class = 250_000.0 * w.options.scale;
    let specialist: Vec<bool> = order.iter().map(|x| w.people[x.0].player.position.is_specialist()).collect();
    let quotas: Vec<(u8, usize)> = service_tiers(world_seed, class_of, class)
        .iter()
        .zip(reach)
        .map(|(&(stars, n), r)| (stars, (n as f64 * r).round() as usize))
        .collect();
    let floor: Vec<u8> = order.iter().map(|x| floors.get(&x.0).copied().unwrap_or(0)).collect();
    for (x, stars) in order.iter().zip(service_stars_floored(&specialist, &floor, &quotas)) {
        w.people[x.0].stars = stars;
    }
    underclass_camps(w, &members, seed, reach[0]);
}

/// An underclass's kickers, punters and snappers (update 2, lane 7): the
/// general services leave them unrated, as in life, and the kicking camps
/// rank the ones they have seen against their own spot, on lists as short
/// as the services' (`short`, the share of a full class's tiers). The
/// signing stages' camps (`camp_rankings`) take over from the junior
/// spring. Claude's pick, proposed: the camps see the same kids the
/// signing stages' camps would (the family's money), on the same tiers.
fn underclass_camps(w: &mut World, members: &[usize], seed: u64, short: f64) {
    let spots = [Position::K, Position::P, Position::LS];
    let mut lists: [Vec<(f64, usize)>; 3] = Default::default();
    for &i in members {
        let p = &w.people[i];
        let Some(s) = spots.iter().position(|&x| x == p.player.position) else { continue };
        let id = p.player.id as u64;
        if key_uniform(seed ^ 0xCA4D, id) >= 0.3 + 0.7 * p.bg.money {
            continue;
        }
        let grade = if spots[s] == Position::LS {
            crate::positions::overall_for(Position::LS, &p.player.attributes)
        } else {
            specialist_grade(&p.player)
        };
        lists[s].push((grade + 1.5 * key_normal(seed ^ 0xCA4E, id), i));
    }
    for &i in members {
        let p = &mut w.people[i];
        if p.player.position.is_specialist() {
            p.stars = 0;
            p.camp_stars = 0;
        }
    }
    let scale = w.options.scale * short;
    for (s, list) in lists.iter_mut().enumerate() {
        list.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let tiers = if spots[s] == Position::LS { &SNAP_TIERS } else { &CAMP_TIERS };
        for (rank, &(_, i)) in list.iter().enumerate() {
            let Some(&(stars, _)) = tiers.iter().find(|t| (rank as f64) < (t.1 * scale).round().max(1.0)) else {
                break;
            };
            w.people[i].camp_stars = stars;
        }
    }
}

fn run_signing_stage(w: &mut World, stage: SignStage, season: i32) {
    let norms = w.system_norms;
    let year = season;
    let sign_year = year + 1;
    let today = w.today;
    // The stage that rates the whole class and records its stars.
    let rating = matches!(stage, SignStage::Early | SignStage::JanuaryEnrollees);
    let summer = matches!(stage, SignStage::Summer(_));
    // A late signing day (lane B) runs February's matching on the recruits
    // still looking; only its last day closes their search.
    let late_day = matches!(stage, SignStage::Late(_));
    let last_day = stage == SignStage::Late(true);
    let final_like = stage == SignStage::Final || late_day;
    let late_norms = if late_day { w.late_signing.clone() } else { None };
    if late_day && late_norms.is_none() {
        return;
    }
    let seed = w.options.seed as u64 ^ (sign_year as u64) << 20;
    // Your desk (lane 10): your board's offers, your visits and your NIL
    // promises. Your recruits decide on them by the same code as on any
    // AI staff's offer.
    let mine = fo::college::stage_board(w);
    let vj = fo::college::visits_program(w);
    let n = w.people.len();
    let mut pool: Vec<Recruit> = Vec::new();
    // Film aligned by position: each position's top looks the same.
    let mut pos_stats: [(f64, f64, f64); POSITION_COUNT] = [(0.0, 0.0, 0.0); POSITION_COUNT];
    let mut candidates: Vec<usize> = Vec::new();
    let played = hs_played(w);
    // Each person read on all cores (world speed lane), then added up in
    // person order, so the sums are the same as one walk.
    let world: &World = w;
    let looks: Vec<(usize, bool, Option<f64>)> = (0..n)
        .into_par_iter()
        .with_min_len(4096)
        .filter_map(|i| {
            let p = &world.people[i];
            let unsigned = p.cal.signed.is_none();
            // The late pool belongs to its own class's late days, never the
            // next class's waves (lane B); a late day still counts it in its
            // class for the fastest kids' floors.
            let in_class = p.stage == Stage::HighSchool && p.class_of <= sign_year && (late_day || !p.cal.late);
            let senior = in_class && unsigned;
            let prep = p.stage == Stage::PrepSchool && unsigned && stage == SignStage::Final;
            // A junior college player still in the portal decides there
            // (minor bug 337), not as a mover.
            let juco = p.stage == Stage::College
                && unsigned
                && p.cal.portal.is_none()
                && !matches!(stage, SignStage::Summer(_))
                && p.college.is_some_and(|c| world.programs[c].division == Division::JUCO)
                && p.college_seasons + p.cal.count_pending as u32 >= 1;
            let candidate = if late_day {
                // A late day weighs only the recruits still looking (lane B),
                // each from his own week on.
                p.cal.late
                    && unsigned
                    && p.cal.portal.is_none()
                    && matches!(p.stage, Stage::HighSchool | Stage::PrepSchool | Stage::College)
                    && (last_day || late_norms.as_ref().is_some_and(|l| l.ready(p, today, seed)))
            } else {
                senior || prep || juco
            };
            // Film is aligned against the whole class, signed or not.
            let film = in_class.then(|| film_raw(p, &played));
            (candidate || film.is_some()).then_some((i, candidate, film))
        })
        .collect();
    for (i, candidate, film) in looks {
        if candidate {
            candidates.push(i);
        }
        if let Some(f) = film {
            let s = &mut pos_stats[w.people[i].player.position.index()];
            s.0 += 1.0;
            s.1 += f;
            s.2 += f * f;
        }
    }
    if candidates.is_empty() {
        if last_day {
            w.late_signing = None;
        }
        return;
    }
    let mut pos_mean_sd: Vec<(f64, f64)> = pos_stats
        .iter()
        .map(|&(k, s, s2)| {
            if k < 2.0 {
                (45.0, 6.0)
            } else {
                let m = s / k;
                (m, ((s2 / k - m * m).max(1.0)).sqrt())
            }
        })
        .collect();
    let mut all_mean_sd = {
        let (k, s, s2) = pos_stats.iter().fold((0.0, 0.0, 0.0), |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2));
        let m = s / k.max(1.0);
        (m, ((s2 / k.max(1.0) - m * m).max(1.0)).sqrt())
    };
    // A late day reads film against the whole class, as signing day did.
    if let Some(n) = &late_norms {
        pos_mean_sd = n.pos.clone();
        all_mean_sd = n.all;
    }
    if stage == SignStage::Final {
        w.late_signing = Some(LateSigning {
            sign_year,
            from: today,
            last: crate::calendar::YearDates::new(sign_year).summer_enrollment,
            pos: pos_mean_sd.clone(),
            all: all_mean_sd,
        });
    }
    // Your calls to colleges as a high school coach (section 7, topic 16).
    // SK13: a head coach seen at his games puts a name on the map.
    let seen_trips = super::scouting::own::seen_lift(w);
    let promos = super::scouting::hs::promos(w);
    let wr: &World = w;
    let recruits: Vec<Recruit> = candidates
        .par_iter()
        .map(|&i| {
            // (Visibility and film read only; the person's own stream isn't touched here.)
            let p = &wr.people[i];
            let raw = film_raw(p, &played);
            let (m, sd) = pos_mean_sd[p.player.position.index()];
            let film = all_mean_sd.0 + all_mean_sd.1 * (raw - m) / sd;
            let juco_mover = p.stage == Stage::College;
            let promo =
                promos.get(&p.player.id).copied().unwrap_or(0.0) + seen_trips.get(&p.player.id).copied().unwrap_or(0.0);
            let (visibility, went_viral) = recruit_visibility(wr, p, film, all_mean_sd, seed, promo);
            let gift = crate::links::gift(&super::times::seen_links(p, &super::times::seen(wr, p)));
            // What his tape holds (SK1, SK2, SK5): every staff and service
            // sees the same film, so they lean the same way. A kid seen at
            // the big camps and showcases has been seen against good
            // players; one seen only on his own reels reads high.
            let foot = super::scouting::footage::footage(w, i);
            let vs_good = p.camp_stars > 0 || p.stars >= 4 || juco_mover;
            // (His film is already his season's play, so his form isn't
            // counted twice.)
            let lean = super::scouting::footage::scout_tilt(&foot, 10.0, vs_good, false) - 0.3 * foot.luck;
            let reel = if foot.reels && !juco_mover { 2.5 * (1.0 - p.visibility.clamp(0.0, 1.0)) } else { 0.0 };
            let film = film + lean + reel;
            let wide = super::scouting::footage::film_factor(&foot);
            Recruit {
                person: i,
                film,
                projection: 0.0,
                visibility,
                gpa: p.bg.gpa,
                juco_mover,
                viral: went_viral,
                gift,
                wide,
                svc: lean + reel,
            }
        })
        .collect();
    pool.extend(recruits);
    // Older brothers already playing college football, from the same high school (topic 5).
    let mut alumni_of: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    let in_college: Vec<(usize, usize)> = w
        .people
        .par_iter()
        .with_min_len(4096)
        .filter_map(|p| if p.stage == Stage::College { p.college.map(|c| (p.hs, c)) } else { None })
        .collect();
    for (hs, c) in in_college {
        alumni_of.entry(hs).or_default().push(c);
    }
    for r in pool.iter() {
        if r.juco_mover || !rating {
            continue;
        }
        let p = &mut w.people[r.person];
        if let Some(list) = alumni_of.get(&p.hs)
            && p.family.older_brother_at.is_none()
            && p.rng.chance(0.04)
        {
            let c = list[p.rng.int(0, list.len() as i32 - 1) as usize];
            p.family.older_brother_at = Some(c);
        }
    }
    let dbg_t = std::time::Instant::now();
    // Services' upside guess: one shared projection. Their opinion of a kid
    // drifts: each new look moves it halfway toward what they now see,
    // instead of starting over (part B 6). A junior college mover is a new
    // evaluation. Each read uses the recruit's own random stream, so it runs
    // on all cores.
    let world_seed = w.options.seed as u64;
    let persons = super::pick_mut(&mut w.people, pool.iter().map(|r| r.person));
    persons.into_par_iter().zip(pool.par_iter_mut()).for_each(|(p, r)| {
        // After signing day the services have stopped looking (lane B):
        // a late recruit keeps their last read.
        if late_day && let Some(prev) = p.cal.service_read {
            r.projection = prev;
            return;
        }
        let rep = scout_player(&p.player, ScoutOptions { accuracy: 12.0, level: Level::HS }, &mut p.rng);
        // What the services misread about a kid stays misread look after
        // look (A12): their errors are shared and lasting, not fresh noise
        // that averages away.
        let seen = rep.potential + service_bias(p, world_seed) + r.svc;
        r.projection = match p.cal.service_read {
            Some(prev) if !r.juco_mover => prev + 0.5 * (seen - prev),
            _ => seen,
        };
        p.cal.service_read = Some(r.projection);
        p.visibility = r.visibility;
        p.bg.exposure_network = r.visibility;
        if r.viral && rating {
            let age = p.row_age();
            p.note(format!("At {age} a highlight clip went viral and got him a first look from recruiters."));
        }
    });

    // --- Star ratings: the services only rank kids they have seen -------------------------
    let seniors: Vec<usize> = (0..pool.len()).filter(|&k| !pool[k].juco_mover).collect();
    // S5: the class's fastest timed kids are owed stars (and S7's desperate
    // schools gamble on them).
    let floors = {
        let mut class_n = [0.0; POSITION_COUNT];
        for (n, s) in class_n.iter_mut().zip(&pos_stats) {
            *n = s.0;
        }
        let kids: Vec<usize> = seniors.iter().map(|&k| pool[k].person).collect();
        super::times::speed_floors(w, &kids, &class_n)
    };
    let class = 250_000.0 * w.options.scale;
    // The services rate the class at each summer wave and again in the
    // senior season. Each rating replaces the last, so a kid holds only the
    // stars of the latest one (February adds no new stars).
    let rate_now = matches!(stage, SignStage::Summer(_)) || rating;
    if rate_now {
        let mut by_star: Vec<usize> = seniors.clone();
        // The services rank mostly from film and their projection; they
        // see a kid's measurables only once he has tested at their camps,
        // so they fall behind on a physical freak (D3). They also read his
        // offer list (minor bug 17).
        let sheets: Vec<f64> =
            pool.par_iter().with_min_len(256).map(|r| offer_sheet(&w.programs, &w.people[r.person], year)).collect();
        let score: Vec<f64> = pool
            .iter()
            .zip(&sheets)
            .map(|(r, &sheet)| star_score(r.film, r.projection, r.visibility, r.gift, sheet, all_mean_sd.1))
            .collect();
        by_star.sort_by(|&a, &b| score[b].total_cmp(&score[a]));
        let specialist: Vec<bool> =
            by_star.iter().map(|&k| w.people[pool[k].person].player.position.is_specialist()).collect();
        let quotas: Vec<(u8, usize)> = service_tiers(w.options.seed as u64, sign_year, class).to_vec();
        let floor: Vec<u8> = by_star.iter().map(|&k| floors.get(&pool[k].person).copied().unwrap_or(0)).collect();
        for (&k, stars) in by_star.iter().zip(service_stars_floored(&specialist, &floor, &quotas)) {
            w.people[pool[k].person].stars = stars;
        }
        // Kickers, punters and long snappers get their own stars from the
        // camps (fix list 1.5); the camps find the snappers once a class,
        // at its first rating.
        camp_rankings(w, &mut pool, &seniors, seed, all_mean_sd, stage == SignStage::Summer(0));
        // The services' rankings as printed (update 2, minor bugs 12-15).
        super::scouting::services::record(w, sign_year, today);
    }
    // When each recruit makes up his mind (T5.2).
    let persons = super::pick_mut(&mut w.people, pool.iter().map(|r| r.person));
    persons.into_par_iter().zip(pool.par_iter()).for_each(|(p, r)| {
        if p.cal.commit_timing.is_none() {
            p.cal.commit_timing = Some(commit_timing(p, seed, r.juco_mover));
        }
    });

    // Senior year: borderline students with offers coming get pushed over an
    // NCAA line by summer school and retaken courses (note 6). Big programs
    // chase this hardest for blue-chips.
    for &k in seniors.iter().filter(|_| rating) {
        let i = pool[k].person;
        let p = &mut w.people[i];
        let stars = recruiting_stars(p);
        if stars < 3 {
            continue;
        }
        let gpa = p.bg.gpa;
        let line = if (1.85..2.0).contains(&gpa) {
            2.0
        } else if (2.15..2.3).contains(&gpa) {
            2.3
        } else {
            continue;
        };
        let chance = 0.3
            + 0.03 * (p.player.personality.work_ethic - 10.0)
            + if stars >= 4 { 0.15 } else { 0.0 }
            + 0.15 * p.bg.school_resources;
        if p.rng.chance(clamp(chance, 0.05, 0.85)) {
            p.bg.gpa = line + p.rng.range(0.0, 0.05);
            pool[k].gpa = p.bg.gpa;
            let age = p.row_age();
            p.note(format!(
                "At {age}, summer school and retaken courses lifted his core GPA from {gpa:.2} to {:.2}.",
                p.bg.gpa
            ));
        }
    }

    if w.recording && rating {
        // The general services' ranking; the specialists' camp stars are their own scale.
        let mut counts = [0u32; 6];
        for &k in &seniors {
            let p = &w.people[pool[k].person];
            if !p.player.position.is_specialist() {
                counts[p.stars as usize] += 1;
            }
        }
        w.signing_classes.push((sign_year, counts));
    }
    // Rogue boosters pick the recruits they will pay (11.6).
    if rating {
        super::ncaa::before_matching(w, &seniors.iter().map(|&k| pool[k].person).collect::<Vec<_>>());
    }
    // Committed and signed blue-chips pull their high school teammates (T5.3).
    let mut friends: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    // Found on all cores, then listed in person order (world speed lane).
    let pulls: Vec<(usize, usize)> = w
        .people
        .par_iter()
        .with_min_len(4096)
        .filter_map(|p| {
            if p.stars >= 4 && p.stage == Stage::HighSchool && p.class_of == sign_year {
                p.cal.signed.map(|s| s.program).or(p.cal.commit.map(|c| c.0)).map(|j| (p.hs, j))
            } else {
                None
            }
        })
        .collect();
    for (hs, j) in pulls {
        friends.entry(hs).or_default().push(j);
    }
    let junior_officials = w.changes.rules(year).junior_officials;

    let dbg_lap = |what: &str| {
        if std::env::var("AFM_DEBUG").is_ok() {
            eprintln!("    {what}: {:.2}s", dbg_t.elapsed().as_secs_f64());
        }
    };
    dbg_lap("pool and stars");
    // --- Who each program sees and wants -----------------------------------------------
    // Only kids with real college film or upside get looked at.
    let mut interest: Vec<usize> = (0..pool.len()).collect();
    interest.sort_by(|&a, &b| {
        let ka = pool[a].film.max(pool[a].projection - 6.0);
        let kb = pool[b].film.max(pool[b].projection - 6.0);
        kb.total_cmp(&ka)
    });
    interest.truncate(((class * 0.25) as usize).max(50).min(pool.len()));
    // A recruit with a verbal commitment stays in view of his school.
    let mut looked_at = vec![false; pool.len()];
    for &k in &interest {
        looked_at[k] = true;
    }
    // Every rated recruit stays in view of the staffs in his own state (fix list 2.4).
    interest.extend((0..pool.len()).filter(|&k| {
        let p = &w.people[pool[k].person];
        !looked_at[k] && (p.cal.commit.is_some() || recruiting_stars(p) >= 2)
    }));
    // A compact copy of what staffs look at, so the scan stays in cache.
    struct Cand {
        k: usize,
        state: usize,
        hs: usize,
        area: usize,
        stars: u8,
        vis: f64,
        gpa: f64,
        /// The GPA the NCAA reads, with the test score in the test years.
        elig_gpa: f64,
        /// A strict school's special admit: grades trending up, good character
        /// reads, a stable home or a mentor, no off-field trouble (note 6).
        grows_out: bool,
        film: f64,
        proj: f64,
        gift: f64,
        /// How far film narrows him (SK1).
        wide: f64,
        /// His listed height and weight (SK17's minimums read these).
        listed: (f64, f64),
        id: u64,
        pos: Position,
        juco_mover: bool,
        /// The program he holds a verbal commitment to.
        committed: Option<usize>,
        /// His fit to each system (S6).
        fit: [f64; 11],
        /// His S5 speed floor: 3 for the fastest 1 in 100 timed at his
        /// position, 4 for 1 in 1,000, else 0.
        fast: u8,
    }
    let cands: Vec<Cand> = interest
        .par_iter()
        .map(|&k| {
            let r = &pool[k];
            let p = &w.people[r.person];
            Cand {
                k,
                state: p.home_state,
                hs: p.hs,
                area: w.high_schools[p.hs].area,
                stars: recruiting_stars(p),
                vis: r.visibility,
                gpa: r.gpa,
                elig_gpa: eligibility_gpa(r.gpa, p.cal.test_score, sign_year),
                grows_out: {
                    let pers = &p.player.personality;
                    let read = |k: u64| 1.5 * key_normal(seed ^ k, p.player.id as u64);
                    let trouble = p.player.history.iter().any(|h| {
                        h.events.has(crate::player::DevelopmentEvent::Suspended)
                            || h.events.has(crate::player::DevelopmentEvent::Dismissed)
                    });
                    p.gpa_trend > 0.02
                        && pers.work_ethic + read(31) >= 11.0
                        && pers.coachability + read(37) >= 10.0
                        && (p.bg.home_stability >= 0.5 || p.bg.mentor)
                        && !trouble
                },
                film: r.film,
                proj: r.projection,
                gift: r.gift,
                wide: if r.wide > 0.0 { r.wide } else { 1.0 },
                listed: super::scouting::file::listed(&p.player),
                id: p.player.id as u64,
                pos: p.player.position,
                juco_mover: r.juco_mover,
                committed: p.cal.commit.map(|x| x.0),
                fit: super::systems::fits(&p.player, p.player.position, &w.system_norms),
                fast: floors.get(&r.person).copied().unwrap_or(0),
            }
        })
        .collect();
    let programs = &w.programs;
    let schools = &w.high_schools;
    // S7a: the schools whose good players keep moving up to bigger
    // programs: the Group of 5 and FCS (proposed, not confirmed: by level,
    // as the portal's real flow runs, not by each school's own losses).
    let poached: Vec<bool> = programs
        .iter()
        .map(|g| matches!(g.division, Division::FBS | Division::FCS) && !g.power_conference(year))
        .collect();
    let late = matches!(stage, SignStage::JanuaryEnrollees | SignStage::Final | SignStage::Late(_));
    let coaches = &w.coaches;
    // Each staff's class plan per position (fix list 1.1, 1.2, 1.6).
    let habit_seed = w.options.seed as u64;
    let bars = w.draft_bars;
    let seat_prog = super::scouting::readers::seat_staff(w)
        .and_then(|s| if let super::scouting::readers::Staff::Program(p) = s { Some(p) } else { None });
    let own_mins = w.scouting.own.minimums.clone();
    let read = RosterRead { stage };
    let mut signed_by: Vec<Vec<usize>> = vec![Vec::new(); programs.len()];
    // Found on all cores, then listed in person order (world speed lane).
    let signees: Vec<(usize, usize)> = w
        .people
        .par_iter()
        .enumerate()
        .with_min_len(4096)
        .filter_map(|(i, p)| p.cal.signed.filter(|_| p.stage != Stage::College).map(|s| (s.program, i)))
        .collect();
    for (j, i) in signees {
        signed_by[j].push(i);
    }
    let mut plans: Vec<ClassPlan> = programs
        .par_iter()
        .zip(signed_by.par_iter())
        .map(|(prog, signed)| class_plan(prog, &w.people, signed, sign_year, habit_seed, bars, read))
        .collect();
    // A board per position (fix list 1.1): each staff ranks the recruits it
    // sees against the others at the same spot. Kickers, punters and long
    // snappers are ranked on what the camps measure (fix list 1.4).
    type Board = Vec<(usize, f64, bool)>;
    let mut boards: Vec<Vec<Board>> = programs
        .par_iter()
        .zip(plans.par_iter())
        .map(|(prog, plan)| {
            // The staff's evaluators (section 7, topic 14): the position
            // coach of each recruit's group and the head coach, each with
            // his own eye and his own lasting lean on the kid.
            let hc_eye = prog.head_coach.map(|c| (coaches[c].evaluation(), coaches[c].id as u64));
            // SK17: the head coach's own hard lines on a listed height or
            // weight (yours, if this is your program).
            let mins: Vec<super::scouting::own::Minimum> = if seat_prog == Some(prog.id) {
                own_mins.clone()
            } else {
                prog.head_coach
                    .map(|c| super::scouting::own::ai_minimums(coaches[c].id as u64 ^ 0xC011))
                    .unwrap_or_default()
            }
            .into_iter()
            .filter(|m| {
                m.drop
                    && matches!(
                        m.measure,
                        super::scouting::own::Measure::Height | super::scouting::own::Measure::Weight
                    )
            })
            .collect();
            let eyes: Vec<Option<(f64, u64)>> = prog
                .position_coaches
                .iter()
                .map(|pc| pc.map(|c| (coaches[c].evaluation(), coaches[c].id as u64)))
                .collect();
            // A head coach on the hot seat: recent results under expectation.
            let pressure = prog
                .head_coach
                .map_or(0.0, |c| clamp(-3.0 * coaches[c].recent.last().copied().unwrap_or(0.0), 0.0, 1.0));
            let pseed = seed ^ (prog.id as u64).wrapping_mul(0x1000_0001);
            // S7a: how desperate the school is, by the signs it shows.
            let signs_at = |pos: Position| {
                u8::from(late && plan.scholarship[pos.index()] > 0)
                    + u8::from(pressure >= HOT_SEAT)
                    + u8::from(poached[prog.id])
            };
            // The staff's own ties to high schools, carried from their old jobs (note 4).
            let mut ties: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();
            for c in [prog.head_coach, prog.oc, prog.dc].into_iter().chain(prog.position_coaches).flatten() {
                for (&h, &t) in &coaches[c].ties {
                    let e = ties.entry(h).or_insert(0.0);
                    *e = e.max(t);
                }
            }
            // Its own region, where it reads every recruit (minor bug 246).
            let region = super::program::region(prog);
            let academic = super::program::academic_reach(prog);
            let mut seen: Vec<(usize, f64, bool, Position)> = Vec::new();
            for c in &cands {
                // A JUCO player moves up to a four-year school or nowhere.
                if c.juco_mover && prog.division == Division::JUCO {
                    continue;
                }
                // A staff always keeps its own commitments in view.
                let ours = c.committed == Some(prog.id);
                // Staffs don't waste offers on blue-chips far above their level.
                if !ours
                    && ((matches!(prog.division, Division::D2 | Division::NAIA | Division::D3) && c.stars >= 4)
                        || (prog.division == Division::FCS && c.stars >= 5))
                {
                    continue;
                }
                // Outside its region a staff reads rated recruits (minor bug
                // 246): the services' stars are how a name 1,000 miles off
                // reaches it ([`FAR_READ`]); its region and its pipeline to
                // his state count in full.
                let near = (region[c.state] + 0.6 * prog.pipeline[c.state]).min(1.0);
                // An academic name or an academy reads the students it can
                // admit wherever they live, whatever their stars (minor bug
                // 256).
                let admits = c.gpa >= prog.admissions_bar && academic > 0.0;
                let far_read = FAR_READ[c.stars.min(5) as usize];
                let far_read = if admits { far_read.max(academic) } else { far_read };
                let reach = near.max(far_read * prog.footprint[c.state]);
                // A blue-chip is seen by everyone; others only inside a
                // footprint (minor bug 246: a 3-star's 0.45 counted in every
                // state, so about half of all FBS staffs had every 3-star in
                // the country on their boards, and Iowa's filled from
                // Georgia, Florida and Alabama).
                let bonus = if c.stars >= 4 { 1.0 } else { 0.15 * c.stars as f64 * reach };
                // Pipelines to his school and his area, and the staff's ties, open doors.
                let hs_pipe = prog.hs_pipeline.get(&c.hs).map_or(0.0, |v| v.0);
                let area_pipe = prog.area_pipeline.get(&c.area).map_or(0.0, |v| v.0);
                let tie = ties.get(&c.hs).copied().unwrap_or(0.0);
                let local = hs_pipe.max(0.6 * area_pipe).max(0.7 * tie);
                let see = (reach + 1.2 * local) * c.vis * 1.6 + bonus;
                // A staff knows every rated recruit in its own state (fix list 2.4).
                let home = c.state == prog.state && c.stars >= 2;
                // S7: a desperate school reads the track lists in its own
                // footprint for burners.
                let signs = if c.fast >= 3 { signs_at(c.pos) } else { 0 };
                let burner = signs > 0 && (c.state == prog.state || prog.footprint[c.state] >= 0.2);
                if !ours && !home && !burner && see < 1.0 && key_uniform(pseed, c.id) > see {
                    continue;
                }
                // The NCAA floor can't be bent; a school's own bar can, a few times a class.
                let tier = ncaa_tier(prog.division, c.elig_gpa);
                if tier == Eligibility::NonQualifier {
                    continue;
                }
                let special = c.gpa < prog.admissions_bar;
                if special && (tier != Eligibility::Qualifier || !c.grows_out || prog.special_admits == 0) {
                    continue;
                }
                if !ours
                    && mins.iter().any(|m| {
                        m.pos == c.pos
                            && match m.measure {
                                super::scouting::own::Measure::Height => c.listed.0 < m.value,
                                _ => c.listed.1 < m.value,
                            }
                    })
                {
                    continue;
                }
                let g = super::coach::Group::of(c.pos) as usize;
                let (he, _) = hc_eye.unwrap_or((10.0, pseed));
                let (pe, pk) = eyes.get(g).copied().flatten().or(hc_eye).unwrap_or((10.0, pseed));
                let noise_sd = clamp(3.5 - 0.15 * (0.6 * pe + 0.4 * he - 10.0), 1.0, 6.0);
                let read = |k: u64| {
                    noise_sd * (0.6 * key_normal(pk ^ 0xE7E0 ^ k, c.id) + 0.4 * key_normal(pseed.wrapping_add(k), c.id))
                        / 0.7211
                };
                // Staffs offer on film more than on a service's projection,
                // and value a player who suits what they run (S6; worth up
                // to about a third of a standard deviation, proposed, not
                // confirmed).
                let mut eval = 0.6 * (c.film + c.wide * read(1)) + 0.4 * (c.proj + read(2));
                eval += FIT_EVAL * all_mean_sd.1 * super::systems::fit_to(&c.fit, c.pos, &prog.installed);
                // Traits that can't be coached (D3): a staff that has seen
                // him test (its own camp, a visit, his tape's numbers) pays
                // for size, length, speed and burst, as real staffs do
                // first. A staff with a better eye for athletes sees it
                // truer; it is why a 2-star freak draws an SEC offer before
                // his stars catch up.
                let gift_seen = (c.gift + 0.25 * key_normal(pk ^ 0x61F7, c.id)).max(0.0);
                eval += STAFF_GIFT * all_mean_sd.1 * gift_seen * clamp(0.6 + 0.04 * (pe - 10.0), 0.3, 1.0);
                // Staffs read the services too (lane 10 follow-up): a
                // ranking is a second opinion that draws more coaches to his
                // film, and a class's ranking is how a staff and its
                // collective are judged, so a four-star is a take where a
                // three-star of the same read isn't. Specialists are
                // recruited on their camps' stars already.
                if !c.pos.is_specialist() {
                    eval += STAR_EVAL * all_mean_sd.1 * (c.stars as f64 - 2.0).max(0.0);
                }
                // S7: the gamble on recorded speed alone, bigger with two signs.
                if burner {
                    eval += BURNER_EVAL * all_mean_sd.1 * if signs >= 2 { 1.5 } else { 1.0 };
                }
                // A borderline student can lose his eligibility in college. Strong
                // academic support takes that risk more easily, and so does a
                // coach who has to win now.
                let risk = clamp((2.6 - c.gpa) / 0.6, 0.0, 1.0);
                eval -= 4.0 * risk * (1.0 - academic_support(prog)) * (1.0 - 0.6 * pressure);
                // A staff honors its own commitments at every stage, signing
                // days included: five film standard deviations keep a
                // committed recruit above anyone new, so he loses his place
                // only when the spots at his position run out.
                if ours {
                    eval += 5.0 * all_mean_sd.1;
                }
                seen.push((c.k, eval, special, c.pos));
            }
            seen.sort_by(|a, b| b.1.total_cmp(&a.1));
            // Junior college leagues cap out-of-state players (Mississippi and
            // Kansas both do), so one JUCO can't sign every national blue-chip.
            if prog.division == Division::JUCO {
                let mut out_of_state = 0;
                seen.retain(|&(k, _, _, _)| {
                    let p = &w.people[pool[k].person];
                    if p.home_state == prog.state || p.cal.commit.is_some_and(|x| x.0 == prog.id) {
                        return true;
                    }
                    out_of_state += 1;
                    out_of_state <= JUCO_OUT_OF_STATE_OFFERS
                });
            }
            let mut b: Vec<Board> = vec![Vec::new(); POSITION_COUNT];
            for (k, eval, special, pos) in seen {
                let i = pos.index();
                if b[i].len() < 40 * (plan.scholarship[i] + plan.walk_on[i] + 1) {
                    b[i].push((k, eval, special));
                }
            }
            b
        })
        .collect();

    dbg_lap("lists");
    // Your board replaces your staff's (lane 10): your offers, commitments
    // first, each in your order. Where you leave the coordinator to fill
    // a position, his board follows yours there. A recruit the NCAA would
    // not qualify can't sign; the school's own bar takes a special admit.
    let mut coordinator: Vec<usize> = Vec::new();
    let mut own_at = [0usize; POSITION_COUNT];
    let mut priority: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut promise: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();
    if let Some(m) = &mine {
        let j = m.program;
        // Sandbox mode's roster limits (a test switch): your offers sign
        // past your class plan, the yearly signee cap and the roster limit.
        let lift_roster = w.sandbox(super::sandbox::Switch::RosterLimits);
        let k_of: std::collections::HashMap<usize, usize> =
            pool.iter().enumerate().map(|(k, r)| (r.person, k)).collect();
        let mut mb: Vec<Board> = vec![Vec::new(); POSITION_COUNT];
        let mut order = m.offers.clone();
        order.sort_by_key(|o| !w.people[o.person].cal.commit.is_some_and(|c| c.0 == j));
        for (n, o) in order.iter().enumerate() {
            let Some(&k) = k_of.get(&o.person) else { continue };
            if pool[k].juco_mover && programs[j].division == Division::JUCO {
                continue;
            }
            let p = &w.people[o.person];
            let elig = eligibility_gpa(pool[k].gpa, p.cal.test_score, sign_year);
            if ncaa_tier(programs[j].division, elig) == Eligibility::NonQualifier {
                continue;
            }
            let special = pool[k].gpa < programs[j].admissions_bar;
            mb[p.player.position.index()].push((k, 1.0e6 - n as f64, special));
            if o.priority {
                priority.insert(k);
            }
            if let Some(x) = o.nil {
                promise.insert(k, x);
            }
        }
        for i in 0..POSITION_COUNT {
            own_at[i] = mb[i].len();
            let need = plans[j].scholarship[i];
            if m.fill && own_at[i] < need {
                let theirs: Vec<(usize, f64, bool)> =
                    boards[j][i].iter().copied().filter(|e| !mb[i].iter().any(|x| x.0 == e.0)).collect();
                // Your coordinator's offers name no money: only you do (Cain
                // 2026-11-08), so each carries your number where you set one
                // (as he will be paid it) and is the scholarship alone where
                // you didn't.
                for e in &theirs {
                    let x = fo::college::money_from(w, j, pool[e.0].person).unwrap_or(0.0);
                    promise.entry(e.0).or_insert(x);
                }
                coordinator.extend(theirs.iter().map(|e| e.0));
                mb[i].extend(theirs);
            } else {
                // The class plan stands at each position (your staff's
                // roster math): more offers out than places is how
                // recruiting works, and the first to say yes take them. A
                // commitment you hold is honored past it.
                let held = mb[i]
                    .iter()
                    .filter(|e| w.people[pool[e.0].person].cal.commit.is_some_and(|c| c.0 == j && !c.2))
                    .count();
                plans[j].scholarship[i] = if lift_roster { own_at[i] } else { own_at[i].min(need.max(held)) };
            }
        }
        boards[j] = mb;
        if lift_roster {
            plans[j].slots = plans[j].slots.max(pool.len());
        }
    }
    let mj = mine.as_ref().map(|m| m.program);
    let fill = mine.as_ref().is_some_and(|m| m.fill);

    // --- Deferred acceptance: programs offer down their boards; each recruit holds the best offer.
    // Each staff offers to the best recruit left at a position its plan still
    // has open. A clearly better recruit still gets an offer at a full
    // position (fix list 1.3): one about a quarter of a film standard
    // deviation better than the best left at an open spot at a top program,
    // a full one at the bottom (proposed, not confirmed), and at most two
    // over the plan at a spot. Specialists follow the plan exactly.
    #[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
    struct Held {
        program: usize,
        /// His utility for it without the position room, which can change
        /// as others commit there (11.2).
        base: f64,
        special: bool,
        walk_on: bool,
        pos: usize,
    }
    const OVER_PLAN: usize = 2;
    // A Division I staff always finds a place for a 5-star it wants, over
    // a full class too (proposed, not confirmed: two places). Trial 9: once
    // the classes were full, the 5-stars who waited for February had no
    // Division I offer left, and from 2027 about half of every class (239
    // in all) signed with one junior college.
    const OVER_CLASS: usize = 2;
    let elite = |j: usize, k: usize| {
        w.people[pool[k].person].stars == 5 && matches!(programs[j].division, Division::FBS | Division::FCS)
    };
    let pn = programs.len();
    // The hunts (SK67): each recruit's hunters (AI staffs, and your
    // program for your own hunt). A hunter offers him first at his spot
    // and holds him a scholarship there (the others at his spot share one
    // fewer), gives him the head coach's own time and, an AI one, pays him
    // as a buying program would.
    let your_hunt = super::hunt::yours(w).filter(|h| h.seat == super::hunt::Seat::College && h.team < pn);
    let hunters: Vec<Vec<usize>> = {
        let ai = super::hunt::ai::college_hunters(w);
        let mut by_k: Vec<Vec<usize>> = vec![Vec::new(); pool.len()];
        if !ai.is_empty() || your_hunt.is_some() {
            for (k, r) in pool.iter().enumerate() {
                let id = w.people[r.person].player.id;
                if let Some(v) = ai.get(&id) {
                    by_k[k].extend(v.iter().copied().filter(|&j| j < pn && Some(j) != mj));
                }
                if let Some(h) = your_hunt
                    && h.id == id
                {
                    by_k[k].push(h.team);
                }
            }
        }
        by_k
    };
    let mut hunting = vec![false; pn];
    for (k, list) in hunters.iter().enumerate() {
        for &j in list {
            hunting[j] = true;
            let p = &w.people[pool[k].person];
            let i = p.player.position.index();
            let b = &mut boards[j][i];
            match b.iter().position(|e| e.0 == k) {
                Some(0) => {}
                Some(r) => {
                    let e = b.remove(r);
                    b.insert(0, e);
                }
                // Your board is your offers: the hunt doesn't offer for you.
                None if Some(j) != mj => {
                    let elig = eligibility_gpa(pool[k].gpa, p.cal.test_score, sign_year);
                    if ncaa_tier(programs[j].division, elig) != Eligibility::NonQualifier {
                        let special = pool[k].gpa < programs[j].admissions_bar;
                        let top = b.first().map_or(0.0, |e| e.1) + 1.0;
                        b.insert(0, (k, top, special));
                    }
                }
                None => {}
            }
            if Some(j) != mj || your_hunt.is_some_and(|h| h.hold) {
                plans[j].scholarship[i] = plans[j].scholarship[i].saturating_sub(1);
            }
        }
    }
    let hunted = |j: usize, k: usize| hunters[k].contains(&j);
    let mut cursor = vec![[0usize; POSITION_COUNT]; pn];
    let mut held: Vec<Option<Held>> = vec![None; pool.len()];
    let mut taken = vec![0usize; pn];
    let mut taken_walk_on = vec![0usize; pn];
    let mut taken_at = vec![[[0usize; 2]; POSITION_COUNT]; pn];
    let mut special_taken = vec![0u32; pn];
    let mut offers: Vec<Vec<usize>> = vec![Vec::new(); pool.len()];
    let film_sd = all_mean_sd.1;
    // NIL offers per player (note 13): his position's market value and how
    // good he is, scaled by what the collective can pay. A program on the rise
    // with money to spend reaches this year (the new-era reach).
    let reach: Vec<bool> = programs
        .iter()
        .map(|prog| {
            let rising = prog.head_coach.is_some_and(|c| w.coaches[c].recent.last().is_some_and(|&r| r > 0.25));
            let rich = prog.collective.budget / nil_scale(prog.division) > 1.15 * prog.prestige_rel;
            rising && rich && key_uniform(seed ^ 404, prog.id as u64) < 0.6
        })
        .collect();
    // Who buys blue-chips past the going rate: a reach program whose money
    // ranks well above its standing (minor bug 61). The established powers
    // pay the going rate; nothing caps a recruit's money but a school's own.
    let buying: Vec<bool> = programs
        .iter()
        .enumerate()
        .map(|(j, prog)| {
            let peers = programs.iter().filter(|q| q.division == prog.division);
            let (mut money, mut standing) = (0, 0);
            for q in peers {
                money += (q.collective.budget > prog.collective.budget) as usize;
                standing += (q.prestige > prog.prestige) as usize;
            }
            reach[j] && money + BUY_RANKS <= standing
        })
        .collect();
    let go_free = nil_go_free(w);
    let offer_for = |j: usize, k: usize, rank: f64| -> f64 {
        let prog = &programs[j];
        // Your promise, where you made one (lane 10).
        if Some(j) == mj
            && let Some(&x) = promise.get(&k)
        {
            return x + super::ncaa::booster_cash(prog, pool[k].person);
        }
        let p = &w.people[pool[k].person];
        let talent = offer_star_nil(recruiting_stars(p)) * (1.3 - 0.6 * rank);
        // Your collective's split puts more or less into the class (L9).
        let q = super::legacy::college::split_q(w, j, 2);
        let pay = (prog.collective.budget / 10.0e6).powf(0.8) * q;
        let cap = prog.collective.budget * 0.6 * if reach[j] { 0.35 } else { 0.25 } * q;
        // A school pays the going rate for his position (11.8). A buying
        // program pays past it for the blue-chips at the top of its board, as
        // far as its money goes (minor bug 61). Before NIL a rogue booster's
        // cash is the only money (11.6).
        let full = position_nil(p.player.position) * talent * pay * if reach[j] { 1.6 } else { 1.0 };
        let usual = full.min(cap).min(super::ncaa::position_nil_max(p.player.position));
        let money = if buying[j] && rank < 0.3 && recruiting_stars(p) >= 4 {
            usual.max(full.min(prog.collective.budget * BUY_SHARE))
        } else if hunted(j, k) {
            usual.max(full.min(prog.collective.budget * super::hunt::HUNT_SHARE))
        } else {
            usual
        };
        // From July 2025 the collective's part goes through NIL Go (B18).
        (cleared(prog, w.today, money, nil_go_top_recruit(p), go_free == Some(j))
            + super::ncaa::booster_cash(prog, pool[k].person))
            * rich(j)
    };
    // The next offer a staff makes: (position, walk-on spot).
    let next_offer = |j: usize,
                      cursor: &[[usize; POSITION_COUNT]],
                      taken: &[usize],
                      taken_walk_on: &[usize],
                      taken_at: &[[[usize; 2]; POSITION_COUNT]]|
     -> Option<(usize, bool)> {
        let plan = &plans[j];
        let head = |i: usize| boards[j][i].get(cursor[j][i]).map(|e| e.1);
        let sch_room = taken[j] < plan.slots;
        let mut best: Option<(usize, f64, bool)> = None;
        // The best recruit left at a regular position still open: the bar a full position must clear.
        let mut open_bar: Option<f64> = None;
        for pos in Position::ALL {
            let i = pos.index();
            let Some(e) = head(i) else { continue };
            let star = boards[j][i].get(cursor[j][i]).is_some_and(|x| elite(j, x.0) || hunted(j, x.0))
                && taken[j] < plan.slots + OVER_CLASS
                && taken_at[j][i][0] < plan.scholarship[i] + OVER_PLAN;
            let sch_open = (sch_room && taken_at[j][i][0] < plan.scholarship[i]) || star;
            // Under the House limit, a walk-on place at a regular position
            // opens only in the final period, once the portal has had its turn.
            let late = final_like || !plan.whole_roster || pos.is_specialist();
            let walk_on_open = late && plan.walk_on_open(i, taken_walk_on[j], taken_at[j][i]);
            if !sch_open && !walk_on_open {
                continue;
            }
            if sch_open && !pos.is_specialist() {
                open_bar = Some(open_bar.map_or(e, |b: f64| b.max(e)));
            }
            if best.is_none_or(|b| e > b.1) {
                best = Some((i, e, !sch_open));
            }
        }
        // Talent beats need (fix list 1.3).
        if let Some(bar) = open_bar {
            let margin = film_sd * (1.0 - 0.75 * programs[j].prestige_rel);
            for pos in Position::ALL.into_iter().filter(|p| !p.is_specialist()) {
                let i = pos.index();
                let Some(e) = head(i) else { continue };
                let full = taken_at[j][i][0] >= plan.scholarship[i];
                if full
                    && taken_at[j][i][0] < plan.scholarship[i] + OVER_PLAN
                    && e >= bar + margin
                    && best.is_none_or(|b| e > b.1)
                {
                    best = Some((i, e, false));
                }
            }
        }
        best.map(|b| (b.0, b.2))
    };
    // Each recruit's fit to every system, read once: what a school's systems
    // would show him off in (S7) weighs in every offer.
    let pool_fits: Vec<[f64; 11]> = pool
        .par_iter()
        .map(|r| {
            let p = &w.people[r.person];
            super::systems::fits(&p.player, p.player.position, &norms)
        })
        .collect();
    // How much home still pulls each recruit (minor bug 89).
    let home_w: Vec<f64> = pool.iter().map(|r| home_weight(programs, &w.people[r.person], year)).collect();
    // --- Relationships built over the year (dynasty item 11.18) -------------------------
    // Staffs recruit the top of each position board all year: the position
    // coach calls and visits every target, the head coach gives his own time
    // to the few at the very top. A recruit takes official visits to the five
    // programs he likes best among those recruiting him (the NCAA's limit),
    // and unofficial ones to any within 250 miles. What a staff builds
    // depends on how long it has known him (better-seen kids are found
    // earlier), how much of the head coach's time he got, whether he saw the
    // campus, and the coaches' recruiting and relationship skills.
    let feel =
        |j: usize, k: usize| FEEL_SD * key_normal(seed ^ 77 ^ j as u64, w.people[pool[k].person].player.id as u64);
    let chemistry =
        |j: usize, k: usize| 0.03 * key_normal(seed ^ 91 ^ j as u64, w.people[pool[k].person].player.id as u64);
    // His own high school coach on a staff (C17: Gus Malzahn left
    // Springdale for Arkansas and Mitch Mustain followed, 2006; proposed,
    // not confirmed: 0.05 relationship points).
    let hs_coach_at: std::collections::HashMap<usize, Vec<usize>> = {
        let mut m: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
        for c in w.coaches.iter() {
            if let Some(super::coach::Job { employer: super::coach::Employer::College(j), .. }) = c.job
                && let Some(s) = c.history.last()
                && let super::coach::Employer::HighSchool(h) = s.employer
                && s.role == super::coach::Role::HeadCoach
                && s.to >= w.year - 1
            {
                m.entry(h).or_default().push(j);
            }
        }
        m
    };
    // Per recruit: (program, relationship), and the money of the schools
    // recruiting him, the field a clear top offer is read against (F3).
    let (relationships, fields): (Vec<Vec<(usize, f64)>>, Vec<TopTwo>) = {
        // (program, head coach's own target, board rank share) per recruit.
        let mut targets: Vec<Vec<(usize, bool, f64)>> = vec![Vec::new(); pool.len()];
        for j in 0..pn {
            for i in 0..POSITION_COUNT {
                let plan = plans[j].scholarship[i] + plans[j].walk_on[i];
                let board = &boards[j][i];
                // Your staff recruits everyone you offered; the head coach
                // gives his own time to the targets you named, or to the top
                // of each spot when you named none (lane 10).
                let depth = if Some(j) == mj {
                    own_at[i] + if fill && own_at[i] < plan { RECRUITED_PER_SPOT * plan + RECRUITED_EXTRA } else { 0 }
                } else {
                    RECRUITED_PER_SPOT * plan + RECRUITED_EXTRA
                };
                for (r, &(k, _, _)) in board.iter().take(depth).enumerate() {
                    let share = r as f64 / board.len().max(1) as f64;
                    let own = if Some(j) == mj && !priority.is_empty() {
                        priority.contains(&k)
                    } else {
                        r <= plans[j].scholarship[i]
                    };
                    targets[k].push((j, own, share));
                }
            }
        }
        // Your hunt is yours to recruit before any offer (SK67).
        if let Some(h) = your_hunt
            && let Some(k) = pool.iter().position(|r| w.people[r.person].player.id == h.id)
            && !targets[k].iter().any(|t| t.0 == h.team)
        {
            targets[k].push((h.team, true, 0.0));
        }
        targets
            .par_iter()
            .enumerate()
            .map(|(k, list)| {
                let mut field = TopTwo::new();
                if list.is_empty() {
                    return (Vec::new(), field);
                }
                let p = &w.people[pool[k].person];
                let wants = p.family.decider_wants;
                let money: Vec<f64> = list.iter().map(|&(j, _, share)| offer_for(j, k, share)).collect();
                for (&(j, ..), &m) in list.iter().zip(&money) {
                    field.add(j, m);
                }
                let lead = recruit_top_weight(p, year);
                // The five he likes best before signing day get his official visits.
                let mut liked: Vec<(f64, usize)> = list
                    .iter()
                    .zip(&money)
                    .map(|(&(j, _, share), &m)| {
                        let u = utility_fit(
                            &pool_fits[k],
                            schools,
                            p,
                            &wants,
                            &programs[j],
                            year,
                            m,
                            1.0 - share,
                            home_w[k],
                        );
                        (u + feel(j, k) + top_money(lead, m, field.other(j)), j)
                    })
                    .collect();
                liked.sort_by(|a, b| b.0.total_cmp(&a.0));
                // Official visits before he decides (T5): a summer committer
                // could take none before juniors were allowed them (2018).
                // An April committer has had no time for them; before 2018
                // only a fall committer had.
                let officials = match p.cal.commit_timing {
                    Some(CommitTiming::Summer(0)) => 0,
                    Some(CommitTiming::Summer(1..=3)) if !junior_officials => 0,
                    Some(CommitTiming::Summer(1)) => 2,
                    _ => OFFICIAL_VISITS,
                };
                // Official visits he really took on their dates (T5.4); the
                // rest of his allowance goes to the schools he likes best.
                let mut official: Vec<usize> =
                    p.cal.visits.iter().filter(|v| v.2 == super::Visit::Official).map(|v| v.0).collect();
                // One place a school: sandbox mode's contact rules can give
                // your program more than one official.
                official.sort_unstable();
                official.dedup();
                for x in liked.iter() {
                    if official.len() >= officials {
                        break;
                    }
                    // Your program hosts only the visits you set (lane 10).
                    if Some(x.1) == vj {
                        continue;
                    }
                    if !official.contains(&x.1) {
                        official.push(x.1);
                    }
                }
                let known = clamp(0.35 + 0.65 * pool[k].visibility, 0.35, 1.0);
                let built = list
                    .iter()
                    .map(|&(j, own, _)| {
                        let prog = &programs[j];
                        // Dated contact builds the bond: local kids see a staff
                        // more and for less (T5.3).
                        let contact = p.cal.visits.iter().filter(|v| v.0 == j).count().min(5) as f64;
                        let visited =
                            official.contains(&j) || contact > 0.0 || geo::miles(p.home_state, prog.state) <= 250.0;
                        let group = super::coach::Group::of(p.player.position);
                        let hc = prog.head_coach.map(|c| *w.coaches[c].ability());
                        // C8: a famous name pulls recruits too.
                        let hc_skill = 0.6 * hc.map_or(9.0, |a| a.recruiting)
                            + 0.4 * hc.map_or(9.0, |a| a.relationships)
                            + prog.head_coach.map_or(0.0, |c| w.coaches[c].name_pull(w.dates.season));
                        let pc_skill =
                            prog.position_coaches[group as usize].map_or(9.0, |c| w.coaches[c].ability().recruiting);
                        // Most of a staff's recruiting time goes to its own state
                        // (fix list 2.5; 1.4 against 0.85 is proposed, not confirmed).
                        let effort = if p.home_state == prog.state { 1.4 } else { 0.85 };
                        // A hunter's head coach is on him (SK67): calls,
                        // home visits, his games; his other own targets
                        // get a little less of him.
                        let hunt = hunted(j, k);
                        let visited = visited || hunt;
                        let hc_time = if hunt {
                            super::hunt::HUNT_TIME
                        } else if own {
                            1.0 - if hunting[j] { super::hunt::HUNT_COST } else { 0.0 }
                        } else {
                            0.35
                        } * if visited { 1.0 } else { 0.4 }
                            * effort;
                        let pc_time = if visited { 1.0 } else { 0.75 } * effort;
                        // A staff member who knows his high school coach starts with trust.
                        let tie = [prog.head_coach, prog.oc, prog.dc]
                            .into_iter()
                            .chain(prog.position_coaches)
                            .flatten()
                            .filter_map(|c| w.coaches[c].ties.get(&p.hs).copied())
                            .fold(0.0, f64::max);
                        // A staff babysits its commitments: calls, visits, drop-ins (T5.3).
                        let babysat = if p.cal.commit.is_some_and(|c| c.0 == j) { 0.02 } else { 0.0 };
                        // An early offer is a long relationship; the first
                        // school to offer is remembered (part B 34).
                        let early_offer = match p.cal.early_offers.iter().position(|o| o.0 == j) {
                            Some(0) => 0.02,
                            Some(_) => 0.01,
                            None => 0.0,
                        };
                        let rel = known
                            * (0.025 * hc_time
                                + 0.006 * (hc_skill - 10.0) * hc_time
                                + 0.003 * (pc_skill - 10.0) * pc_time)
                            + 0.03 * tie
                            + babysat
                            + early_offer
                            + if hs_coach_at.get(&p.hs).is_some_and(|v| v.contains(&j)) { 0.05 } else { 0.0 }
                            + 0.004 * contact
                            + super::folk::dialogue::home_talk(p, j)
                            + chemistry(j, k) * if visited { 1.0 } else { 0.5 };
                        (j, rel)
                    })
                    .collect();
                (built, field)
            })
            .unzip()
    };
    // A late offer from deep on a board comes with no relationship behind it.
    let relationship =
        |j: usize, k: usize| relationships[k].iter().find(|e| e.0 == j).map_or_else(|| 0.5 * chemistry(j, k), |e| e.1);
    dbg_lap("relationships");

    let mut view = RosterView::new(programs, &w.people, &std::collections::HashSet::new());
    // Signed recruits not yet on campus hold their places (an NLI binds).
    for (who, p) in w.people.iter().enumerate() {
        if let Some(sg) = p.cal.signed {
            let (j, i) = (sg.program, p.player.position.index());
            if sg.walk_on {
                taken_walk_on[j] += 1;
            } else {
                taken[j] += 1;
            }
            taken_at[j][i][sg.walk_on as usize] += 1;
            view.join(j, p.player.position, who, p.player.overall() + 4.0, 0);
        }
    }
    // Your former players' pull (update 2, L-guys).
    let leans = super::legacy::guys::leans(w);
    // What a commitment and friends add to a school for him (T5.1, T5.3).
    let pull = |j: usize, k: usize| -> f64 {
        let p = &w.people[pool[k].person];
        let mut u = 0.0;
        if p.cal.commit.is_some_and(|c| c.0 == j) {
            u += stickiness(p, &programs[j], year);
            // Until the early period a commitment holds its place: he
            // weighs a switch only when he can sign (T5.1).
            if summer {
                u += 10.0;
            }
        }
        if friends.get(&p.hs).is_some_and(|v| v.contains(&j)) {
            u += 0.04;
        }
        if let Some(l) = leans.as_ref().filter(|l| l.prog == j) {
            u += l.of(p.player.id, p.hs);
        }
        // A school that holds his position's name (L8).
        u + super::legacy::college::u_lean(w, j, p.player.position, p.stars)
    };
    let mut held_offer = vec![0.0f64; pool.len()];
    // His fit at the school he holds, kept until that room changes: (program, room version, fit).
    let mut held_fit: Vec<Option<(usize, u32, f64)>> = vec![None; pool.len()];
    // Each recruit's rating, read once: nobody develops while offers go out.
    let overall_of: Vec<f64> = pool.par_iter().map(|r| w.people[r.person].player.overall()).collect();
    // What the matching reads of each recruit, read once on all cores: his
    // spot, the wants a room is weighed by and how much he cares about
    // playing early (trial 8 finding 10: reading these off each person, and
    // his ties to each school, was most of the matching's time).
    struct Seat {
        pos: Position,
        wants: super::background::Wants,
        early: f64,
        /// What a clear top offer is worth to him (`top_money_weight`).
        lead: f64,
    }
    let seats: Vec<Seat> = pool
        .par_iter()
        .map(|r| {
            let p = &w.people[r.person];
            Seat {
                pos: p.player.position,
                wants: p.family.decider_wants,
                early: early_weight(recruiting_stars(p)),
                lead: recruit_top_weight(p, year),
            }
        })
        .collect();
    // What doesn't change while offers go out, per board entry: the
    // scholarship offer; the school's appeal with it and as a walk-on place,
    // each with his ties (the relationship the staff built over the year,
    // 11.18, his own feel for the school, 11.1, and the pull of a
    // commitment) and less the walk-on cost; and the ties alone. Each is
    // read when a staff's offers first get there; once staffs are walking
    // deep down their boards (places nobody takes), the rest of every board
    // is read at once on all cores. The offers themselves go out in order,
    // so the results are the same.
    #[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
    struct Fixed {
        offer: f64,
        scholarship: f64,
        walk_on: f64,
        ties: f64,
    }
    let fixed_at = |j: usize, i: usize, r: usize| -> Fixed {
        let board = &boards[j][i];
        let k = board[r].0;
        let rank = r as f64 / board.len().max(1) as f64;
        let p = &w.people[pool[k].person];
        let wants = p.family.decider_wants;
        let prog = &programs[j];
        let offer = offer_for(j, k, rank);
        let ties = relationship(j, k) + feel(j, k) + pull(j, k);
        Fixed {
            offer,
            // A walk-on is last in line for reps, wherever he sits on the board.
            scholarship: utility_fit(&pool_fits[k], schools, p, &wants, prog, year, offer, 1.0 - rank, home_w[k])
                + ties,
            walk_on: utility_fit(&pool_fits[k], schools, p, &wants, prog, year, 0.0, 0.0, home_w[k])
                + (ties - walk_on_cost(p, prog, year + 1)),
            ties,
        }
    };
    let mut fixed: Vec<Vec<Vec<Fixed>>> = vec![vec![Vec::new(); POSITION_COUNT]; pn];
    let mut read_one_by_one = 0;
    let mut active = true;
    while active {
        active = false;
        for j in 0..pn {
            while let Some((i, walk_on)) = next_offer(j, &cursor, &taken, &taken_walk_on, &taken_at) {
                let board = &boards[j][i];
                let r = cursor[j][i];
                let (k, _, special) = board[r];
                let rank = r as f64 / board.len().max(1) as f64;
                cursor[j][i] += 1;
                active = true;
                if special && special_taken[j] >= programs[j].special_admits {
                    continue;
                }
                if r >= fixed[j][i].len() {
                    if read_one_by_one < READ_ALL_AFTER {
                        while fixed[j][i].len() <= r {
                            let e = fixed_at(j, i, fixed[j][i].len());
                            fixed[j][i].push(e);
                            read_one_by_one += 1;
                        }
                    } else {
                        fixed.par_iter_mut().enumerate().for_each(|(j, by_pos)| {
                            for (i, v) in by_pos.iter_mut().enumerate() {
                                let done = v.len();
                                v.extend((done..boards[j][i].len()).map(|r| fixed_at(j, i, r)));
                            }
                        });
                    }
                }
                let fx = fixed[j][i][r];
                let who = pool[k].person;
                let seat = &seats[k];
                let prog = &programs[j];
                // A walk-on spot carries no money and costs the family tuition (fix list 1.6).
                let mut offer = if walk_on { 0.0 } else { fx.offer };
                // His room there as it stands today, commitments included (11.2),
                // weighed more by a better recruit (11.3).
                let me = overall_of[k] + 6.0;
                let fit = view.recruit_fit(j, who, seat.pos, &seat.wants, me, seat.early, sign_year, &w.people);
                let base_u = if walk_on { fx.walk_on } else { fx.scholarship };
                let mut u = base_u + fit;
                // What the school he holds is worth to him now: a recruit at his
                // position may have committed there since (11.2).
                let held_u = held[k].map(|h| {
                    let v = view.ver[h.program][seat.pos.index()];
                    let f = match held_fit[k] {
                        Some((j0, v0, f)) if j0 == h.program && v0 == v => f,
                        _ => {
                            let f = view.recruit_fit(
                                h.program,
                                who,
                                seat.pos,
                                &seat.wants,
                                me,
                                seat.early,
                                sign_year,
                                &w.people,
                            );
                            held_fit[k] = Some((h.program, v, f));
                            f
                        }
                    };
                    // Against a scholarship here, the walk-on place he holds
                    // costs him the scholarship he would turn down.
                    h.base + f - if h.walk_on && !walk_on { scholarship_given_up(seat.pos, prog) } else { 0.0 }
                });
                // The money that tops every other offer counts once more (F3):
                // each of the two against the best money of the other schools
                // recruiting him and against each other's, read fresh at every
                // comparison (a raise can take the lead from the one he holds).
                let held_at = held[k].map(|h| (h.program, held_offer[k]));
                let leads = |mine: f64| {
                    let rival = held_at.map_or(0.0, |h| h.1);
                    (
                        top_money(seat.lead, mine, fields[k].other(j).max(rival)),
                        held_at.map_or(0.0, |h| top_money(seat.lead, h.1, fields[k].other(h.0).max(mine))),
                    )
                };
                let (lead, held_lead) = leads(offer);
                u += lead;
                let mut held_u = held_u.map(|hu| hu + held_lead);
                // Bidding (note 13): a school that wants him near the top of its
                // board raises its offer to beat the one he holds, within its money
                // and, unless it's buying a blue-chip, the going rate.
                if let Some(hu) = held_u
                    && hu >= u
                    && (rank < 0.3 || hunted(j, k))
                    && !walk_on
                    && !(Some(j) == mj && promise.contains_key(&k))
                {
                    let p = &w.people[who];
                    let wants = p.family.decider_wants;
                    let cap = prog.collective.budget
                        * 0.6
                        * if reach[j] { 0.35 } else { 0.25 }
                        * super::legacy::college::split_q(w, j, 2);
                    let top = if hunted(j, k) && Some(j) != mj {
                        prog.collective.budget * super::hunt::HUNT_SHARE.max(if buying[j] { BUY_SHARE } else { 0.0 })
                    } else if buying[j] && recruiting_stars(p) >= 4 {
                        prog.collective.budget * BUY_SHARE
                    } else {
                        super::ncaa::position_nil_max(seat.pos)
                    };
                    // The raise's collective part goes through NIL Go too (B18).
                    let raised = cleared(
                        prog,
                        w.today,
                        (offer * 1.5).min(cap).min(top.max(offer)),
                        nil_go_top_recruit(p),
                        go_free == Some(j),
                    );
                    let (raised_lead, held_then) = leads(raised);
                    let ru = utility_fit(&pool_fits[k], schools, p, &wants, prog, year, raised, 1.0 - rank, home_w[k])
                        + fx.ties
                        + fit
                        + raised_lead;
                    let hu = hu - held_lead + held_then;
                    if ru > hu {
                        offer = raised;
                        u = ru;
                        held_u = Some(hu);
                    }
                }
                // A walk-on place set against the scholarship he holds costs him that scholarship.
                let given_up = match held[k] {
                    Some(h) if walk_on && !h.walk_on => scholarship_given_up(seat.pos, &programs[h.program]),
                    _ => 0.0,
                };
                // Staffs don't waste an offer on a kid already holding a far better one.
                if held_u.is_some_and(|hu| hu > u - given_up + 0.15) {
                    continue;
                }
                offers[k].push(j);
                if held_u.is_some_and(|hu| hu >= u - given_up) {
                    continue;
                }
                let (pos, overall) = (seat.pos, overall_of[k]);
                if let Some(h) = held[k] {
                    // He trades up: his old program's spot opens again.
                    let c = h.program;
                    view.leave(c, pos, who);
                    if h.walk_on {
                        taken_walk_on[c] -= 1;
                    } else {
                        taken[c] -= 1;
                    }
                    taken_at[c][h.pos][h.walk_on as usize] -= 1;
                    if h.special {
                        special_taken[c] -= 1;
                    }
                }
                // What he holds is kept without the top offer's lift, which
                // is read again against each later offer.
                let lead = leads(offer).0;
                held[k] = Some(Held { program: j, base: u - fit - lead, special, walk_on, pos: i });
                // His commitment is news: the next recruit at his position sees
                // him in this room (11.2). He counts as a recruit two years on,
                // the way each recruit judges himself.
                view.join(j, pos, who, overall + 4.0, 0);
                held_offer[k] = offer;
                if walk_on {
                    taken_walk_on[j] += 1;
                } else {
                    taken[j] += 1;
                }
                taken_at[j][i][walk_on as usize] += 1;
                special_taken[j] += special as u32;
            }
        }
    }
    // A recruit whose commitment fell through on signing day (his school's
    // spots at his position ran out) still signs where there is room, as
    // real decommitted recruits nearly always do: of the programs that have
    // him on their boards and a place left, the one he likes best, with no
    // NIL money this late. A scholarship needs a slot in the class and his
    // position no more than two over the plan (a specialist follows the
    // plan exactly). Otherwise he walks on, at a walk-on place the plan
    // keeps at his position or at any place his position still has on the
    // roster: a scholarship limit leaves walk-ons outside it, so he needs
    // only room at his position (a real squad's depth there), while the
    // House era's roster limit needs a place under it too. By February the
    // matching has used the slots, and the plan keeps walk-on places for
    // specialists only, so those alone placed almost no one. The best
    // recruits choose first (proposed, not confirmed).
    let mut scrambled = vec![false; pool.len()];
    if stage == SignStage::Final {
        let mut dropped: Vec<usize> =
            (0..pool.len()).filter(|&k| held[k].is_none() && w.people[pool[k].person].cal.commit.is_some()).collect();
        // Per dropped recruit: (program, special admit, share down its board).
        let mut listed: Vec<Vec<(usize, bool, f64)>> = vec![Vec::new(); pool.len()];
        let mut is_dropped = vec![false; pool.len()];
        for &k in &dropped {
            is_dropped[k] = true;
        }
        if !dropped.is_empty() {
            for (j, by_pos) in boards.iter().enumerate() {
                for board in by_pos {
                    for (r, &(k, _, special)) in board.iter().enumerate() {
                        if is_dropped[k] {
                            listed[k].push((j, special, r as f64 / board.len() as f64));
                        }
                    }
                }
            }
        }
        dropped.sort_by(|&a, &b| pool[b].film.total_cmp(&pool[a].film).then(a.cmp(&b)));
        for k in dropped {
            let who = pool[k].person;
            let p = &w.people[who];
            let wants = p.family.decider_wants;
            let pos = p.player.position;
            let i = pos.index();
            let mut options: Vec<(f64, usize, bool, bool)> = Vec::new();
            for &(j, special, share) in &listed[k] {
                let plan = &plans[j];
                if special && special_taken[j] >= programs[j].special_admits {
                    continue;
                }
                let scholarship = if pos.is_specialist() {
                    taken[j] < plan.slots && taken_at[j][i][0] < plan.scholarship[i]
                } else {
                    let over = if elite(j, k) { OVER_CLASS } else { 0 };
                    taken[j] < plan.slots + over && taken_at[j][i][0] < plan.scholarship[i] + OVER_PLAN
                };
                let walk_on = !scholarship;
                if walk_on && !plan.late_walk_on(i, [taken[j], taken_walk_on[j]], taken_at[j][i]) {
                    continue;
                }
                let standing = if walk_on { 0.0 } else { 1.0 - share };
                let u = utility_fit(&pool_fits[k], schools, p, &wants, &programs[j], year, 0.0, standing, home_w[k])
                    + relationship(j, k)
                    + feel(j, k)
                    - if walk_on { walk_on_cost(p, &programs[j], year + 1) } else { 0.0 };
                options.push((u, j, walk_on, special));
            }
            // A walk-on place costs him the best scholarship he turns down for it.
            let scholarship = options.iter().filter(|o| !o.2).max_by(|a, b| a.0.total_cmp(&b.0)).map(|o| o.1);
            let mut best: Option<(f64, f64, usize, bool, bool)> = None;
            for &(u, j, walk_on, special) in &options {
                let given_up = match scholarship {
                    Some(s) if walk_on => scholarship_given_up(pos, &programs[s]),
                    _ => 0.0,
                };
                if best.is_none_or(|b| u - given_up > b.0) {
                    best = Some((u - given_up, u, j, walk_on, special));
                }
            }
            let Some((_, u, j, walk_on, special)) = best else { continue };
            held[k] = Some(Held { program: j, base: u, special, walk_on, pos: i });
            offers[k].push(j);
            scrambled[k] = true;
            view.join(j, pos, who, p.player.overall() + 4.0, 0);
            if walk_on {
                taken_walk_on[j] += 1;
            } else {
                taken[j] += 1;
            }
            taken_at[j][i][walk_on as usize] += 1;
            special_taken[j] += special as u32;
        }
    }
    // Preferred walk-ons from home (trial 8 finding 5). Walk-on offers go
    // down the same boards as scholarships, from the top, and the top of a
    // Division I board holds recruits with scholarships, who since trial 7's
    // fix 4 turn a walk-on place down. The boards hold only the recruits a
    // staff sees (rated ones, the top quarter of the class on film, and
    // its own commitments), so they ran out before reaching the local
    // unrated kids who make up most real walk-ons, and the places stayed
    // empty (FBS rosters at 82 of 105). So in the final period each
    // Division I staff with planned walk-on places still open offers them
    // to the best unsigned unrated and 2-star seniors from its own state,
    // by film, who meet the NCAA floor and its own admissions bar, until
    // its plan is met. A kid with nothing else takes it, as one with no
    // offer takes any; the walk-on cost is unchanged. Programs pick in
    // order of prestige, the order real kids weigh walk-on offers in
    // (proposed, not confirmed: Division I only, home state only).
    // Trial 9 finding 4: only kids holding nothing were asked, and by then
    // most local seniors held a Division II, III, NAIA or junior college
    // offer, so FBS rosters stayed at 80 (real near 105). A kid holding a
    // small-college place now weighs the walk-on place against it, the
    // scholarship he would give up included, as in the main signing, and
    // takes whichever he values more (the common preferred walk-on story).
    // The late signing days do the same for the places that open (lane B).
    if final_like {
        let division_one = |j: usize| matches!(programs[j].division, Division::FBS | Division::FCS);
        let mut by_state: Vec<Vec<usize>> = vec![Vec::new(); w.states.len()];
        for (k, r) in pool.iter().enumerate() {
            let p = &w.people[r.person];
            if held[k].is_none_or(|h| !division_one(h.program)) && !r.juco_mover && recruiting_stars(p) <= 2 {
                by_state[p.home_state].push(k);
            }
        }
        for list in by_state.iter_mut() {
            list.sort_by(|&a, &b| pool[b].film.total_cmp(&pool[a].film).then(a.cmp(&b)));
        }
        let mut order: Vec<usize> =
            (0..pn).filter(|&j| matches!(programs[j].division, Division::FBS | Division::FCS)).collect();
        order.sort_by(|&a, &b| programs[b].prestige.total_cmp(&programs[a].prestige).then(a.cmp(&b)));
        for j in order {
            let (prog, plan) = (&programs[j], &plans[j]);
            for &k in &by_state[prog.state] {
                if taken_walk_on[j] >= plan.walk_on_room {
                    break;
                }
                if held[k].is_some_and(|h| division_one(h.program)) {
                    continue;
                }
                let who = pool[k].person;
                let p = &w.people[who];
                let (pos, i) = (p.player.position, p.player.position.index());
                let elig = eligibility_gpa(pool[k].gpa, p.cal.test_score, sign_year);
                if ncaa_tier(prog.division, elig) == Eligibility::NonQualifier
                    || pool[k].gpa < prog.admissions_bar
                    || !plan.walk_on_open(i, taken_walk_on[j], taken_at[j][i])
                {
                    continue;
                }
                let wants = p.family.decider_wants;
                let u = utility_fit(&pool_fits[k], schools, p, &wants, prog, year, 0.0, 0.0, home_w[k])
                    + relationship(j, k)
                    + feel(j, k)
                    - walk_on_cost(p, prog, year + 1);
                if let Some(h) = held[k] {
                    let given_up = if h.walk_on { 0.0 } else { scholarship_given_up(pos, &programs[h.program]) };
                    // What he holds, with its money's lead if it tops his list (F3).
                    let lead = top_money(seats[k].lead, held_offer[k], fields[k].other(h.program));
                    if u - given_up <= h.base + lead {
                        continue;
                    }
                    // He takes the walk-on place: his small-college place opens.
                    let c = h.program;
                    view.leave(c, pos, who);
                    if h.walk_on {
                        taken_walk_on[c] -= 1;
                    } else {
                        taken[c] -= 1;
                    }
                    taken_at[c][h.pos][h.walk_on as usize] -= 1;
                    if h.special {
                        special_taken[c] -= 1;
                    }
                }
                held[k] = Some(Held { program: j, base: u, special: false, walk_on: true, pos: i });
                offers[k].push(j);
                view.join(j, pos, who, p.player.overall() + 4.0, 0);
                taken_walk_on[j] += 1;
                taken_at[j][i][1] += 1;
            }
        }
    }

    dbg_lap("deferred acceptance");
    // Who some staff had on its board on signing day (lane B): the ones
    // nobody signed keep looking.
    let on_board: Vec<bool> = if stage == SignStage::Final {
        let mut v = vec![false; pool.len()];
        for b in boards.iter().flatten().flatten() {
            v[b.0] = true;
        }
        v
    } else {
        Vec::new()
    };
    let mut late_pool = 0u32;
    // The offers your coordinator made where you left him the spot (lane 10).
    let coordinator_offers: Vec<u32> = match mj {
        Some(j) => coordinator
            .iter()
            .filter(|&&k| offers[k].contains(&j))
            .map(|&k| w.people[pool[k].person].player.id)
            .collect(),
        None => Vec::new(),
    };
    if std::env::var("AFM_DEBUG").is_ok() {
        let mut by_div = [0usize; DIVISION_COUNT];
        for h in held.iter().flatten() {
            by_div[programs[h.program].division as usize] += 1;
        }
        let want: Vec<usize> = (0..4)
            .map(|d| {
                programs.iter().enumerate().filter(|(_, p)| p.division as usize == d).map(|(j, _)| plans[j].slots).sum()
            })
            .collect();
        let listed: usize = boards.iter().flatten().map(|l| l.len()).sum();
        eprintln!(
            "signing {sign_year}: pool {} interest {} listed {listed} signed {:?} slots {:?}",
            pool.len(),
            interest.len(),
            by_div,
            want
        );
    }
    // --- Outcomes ---------------------------------------------------------------------
    // Only recruits whose time has come decide at this stage (T5.2): summer
    // committers commit; at the early period (or, before it, as January
    // enrollees) those who are ready sign with FBS and FCS programs; in
    // February everyone left decides.
    let esp_era = w.dates.early_signing.is_some() || stage == SignStage::Early;
    let mut decided: Vec<(usize, bool)> = Vec::new();
    for (k, r) in pool.iter().enumerate() {
        let p = &w.people[r.person];
        let timing = p.cal.commit_timing.unwrap_or(CommitTiming::Late);
        let division = held[k].map(|h| w.programs[h.program].division);
        let four_year = matches!(division, Some(Division::FBS | Division::FCS));
        let now = match stage {
            SignStage::Summer(wave) => {
                // Those who committed in an earlier wave hold to it until the
                // early period. Junior colleges recruit only at the end, from
                // what is left (part D 9): nobody commits to one in summer.
                timing == CommitTiming::Summer(wave)
                    && p.cal.commit.is_none()
                    && !r.juco_mover
                    && held[k].is_some_and(|h| w.programs[h.program].division != Division::JUCO)
            }
            SignStage::Early => {
                timing <= CommitTiming::Early
                    && four_year
                    && !waits_for_february(p, held[k].map(|h| &w.programs[h.program]), r.gpa, seed, year)
            }
            SignStage::JanuaryEnrollees => {
                timing <= CommitTiming::Early
                    && four_year
                    && key_uniform(seed ^ 0xEE, p.player.id as u64)
                        < early_enrollee_chance(sign_year, recruiting_stars(p), r.gpa, division.unwrap_or(Division::D2))
            }
            SignStage::Final | SignStage::Late(_) => true,
        };
        if !now {
            continue;
        }
        let early_enrollee = match stage {
            SignStage::JanuaryEnrollees => true,
            SignStage::Early => {
                r.juco_mover
                    || key_uniform(seed ^ 0xEE, p.player.id as u64)
                        < early_enrollee_chance(sign_year, recruiting_stars(p), r.gpa, division.unwrap_or(Division::D2))
            }
            _ => false,
        };
        decided.push((k, early_enrollee));
    }
    if std::env::var("AFM_DEBUG").is_ok() {
        let mut by_div = [0usize; DIVISION_COUNT];
        let mut held_div = [0usize; DIVISION_COUNT];
        let mut timing = [0usize; 3];
        for (k, _) in &decided {
            if let Some(h) = held[*k] {
                by_div[w.programs[h.program].division as usize] += 1;
            }
        }
        for (k, r) in pool.iter().enumerate() {
            if let Some(h) = held[k] {
                held_div[w.programs[h.program].division as usize] += 1;
            }
            match w.people[r.person].cal.commit_timing {
                Some(CommitTiming::Summer(_)) => timing[0] += 1,
                Some(CommitTiming::Early) => timing[1] += 1,
                _ => timing[2] += 1,
            }
        }
        eprintln!(
            "stage {stage:?} {sign_year}: decided by div {by_div:?}, held by div {held_div:?}, timing {timing:?}"
        );
    }
    // Who offered: the visits of the coming weeks go to these (T5).
    if !final_like {
        for (k, r) in pool.iter().enumerate() {
            let mut list = offers[k].clone();
            // The schools that offered him early still have him (part B 34).
            for &(j, _) in &w.people[r.person].cal.early_offers {
                list.push(j);
            }
            let mut seen = std::collections::HashSet::new();
            list.retain(|j| seen.insert(*j));
            list.truncate(8);
            w.people[r.person].cal.offers = list;
        }
    }
    let month = today.month() as usize - 1;
    for (k, early_enrollee) in decided {
        let r = pool[k];
        let i = r.person;
        if let SignStage::Summer(wave) = stage {
            // A verbal commitment: news, not a signature (T5.1). The mind is
            // made up at the wave; the recruit announces on a day of his own
            // before the next wave (July 4 is a favorite after June's
            // official visits; proposed, not confirmed).
            let Held { program: j, walk_on, .. } = held[k].unwrap();
            let id = w.people[i].player.id;
            // Until the next wave: 40 days after junior days, 45 in May, to
            // late August after June, 58 in the fall, 30 in November.
            let span = [40.0, 45.0, 68.0, 58.0, 30.0][wave.min(4) as usize];
            let u = key_uniform(seed ^ 0xA77, id as u64);
            let day = if wave == 2 && u < 0.08 {
                Date::ymd(today.year(), 7, 4)
            } else {
                today.add(1 + (key_uniform(seed ^ 0xA78, id as u64) * span) as i32)
            };
            w.announcements.push((day, i, id, j, walk_on));
            continue;
        }
        let from_hs = w.school_name(w.people[i].hs).to_string();
        let committed = w.people[i].cal.commit.map(|c| c.0);
        // A commitment to a walk-on place never held a scholarship.
        let committed_on_scholarship = w.people[i].cal.commit.is_some_and(|c| !c.2);
        match held[k] {
            Some(Held { program: j, special, walk_on, .. }) => {
                let prog_name = w.program_name(j).to_string();
                let division = w.programs[j].division;
                // The offers he had, biggest names first: this stage's, the
                // ones still standing from before and the early ones (trial 7
                // finding 15: a recruit long committed drew no new offers at
                // signing, so his card listed none).
                let earlier = &w.people[i].cal;
                let mut offer_list: Vec<usize> = offers[k]
                    .iter()
                    .copied()
                    .chain(earlier.offers.iter().copied())
                    .chain(earlier.early_offers.iter().map(|o| o.0))
                    .collect();
                offer_list.sort_unstable();
                offer_list.dedup();
                offer_list.sort_by(|&a, &b| {
                    (w.programs[b].prestige + 0.3 * level_value(w.programs[b].division))
                        .total_cmp(&(w.programs[a].prestige + 0.3 * level_value(w.programs[a].division)))
                        .then(a.cmp(&b))
                });
                let offer_names: Vec<String> = offer_list.iter().map(|&o| w.program_name(o).to_string()).collect();
                // Was he steered? Compare with where he himself would have gone among his offers.
                let p = &w.people[i];
                let steered = p.family.decider != Decider::Himself && {
                    let own = p.family.own_wants;
                    let mine = utility(&norms, schools, p, &own, &w.programs[j], year, 1e5, 0.5, home_w[k]);
                    // His own favorite among the programs that wanted him.
                    offers[k].iter().any(|&jj| {
                        jj != j
                            && utility(&norms, schools, p, &own, &w.programs[jj], year, 1e5, 0.5, home_w[k])
                                > mine + 0.06
                    })
                };
                if w.recording && !r.juco_mover && !walk_on && division == Division::FBS {
                    w.fbs_signees_by_state[w.people[i].home_state] += 1;
                    if w.people[i].stars == 5 && !w.people[i].player.position.is_specialist() {
                        w.recruiting_log.five_stars.push((sign_year, j));
                    }
                }
                if late_day {
                    // A late signing (lane B): its own count, walk-ons included.
                    w.log.signed_after[division as usize][r.juco_mover as usize] += 1;
                } else if stage == SignStage::Final {
                    w.log.signed_late += 1;
                } else {
                    w.log.signed_early += 1;
                }
                // Walk-ons sign no letter or scholarship agreement.
                if !walk_on && !late_day {
                    w.log.signed_by_division[division as usize][(stage == SignStage::Final) as usize] += 1;
                }
                if committed.is_none() && !late_day {
                    w.log.commits += 1;
                    w.log.commits_by_month[month] += 1;
                }
                if scrambled[k] && walk_on && committed == Some(j) && committed_on_scholarship {
                    w.people[i].note(format!(
                        "{prog_name} ran out of scholarships for him on signing day; he walked on there instead."
                    ));
                }
                if let Some(c) = committed
                    && c != j
                {
                    w.log.flips += 1;
                    w.log.blue_broken += (recruiting_stars(&w.people[i]) >= 4) as u32;
                    let from = w.program_name(c).to_string();
                    let age = w.people[i].row_age();
                    let text = if scrambled[k] {
                        format!(
                            "His commitment to {from} fell through on signing day at {age}; {prog_name} had room for him."
                        )
                    } else {
                        format!("Flipped his commitment from {from} to {prog_name} at {age}.")
                    };
                    w.people[i].note(text);
                    if w.people[i].stars >= 4 {
                        let name = &w.people[i].name;
                        let text = if scrambled[k] {
                            format!("{name} signs with {prog_name} after his commitment to {from} fell through.")
                        } else {
                            format!("{name} flips from {from} to {prog_name}.")
                        };
                        let id = w.people[i].player.id;
                        let name = w.people[i].name.clone();
                        w.calendar.post(
                            Event::new(today, CalLevel::Recruiting, Kind::Commitment, Loudness::Medium, text)
                                .program(j)
                                .person(id, &name)
                                .school_link(j, &prog_name),
                        );
                    }
                }
                // His reasons as he chose: before the signature's own hold
                // is on him (minor bug 118: read after it, the +10 that
                // marks a signed recruit as settled sat in "his ties").
                let why = w.logging().then(|| super::wlog::why::choice(w, i, j, "signed", walk_on));
                let p = &mut w.people[i];
                p.walk_on = walk_on;
                let step = match (r.juco_mover, division) {
                    (true, Division::JUCO) => Step::Sideways,
                    (true, _) => Step::Up,
                    (false, Division::FBS) => Step::HsToFbs,
                    (false, Division::FCS) => Step::HsToFcs,
                    (false, _) => Step::HsToSmallCollege,
                };
                p.pending_dip = Some(World::adjustment(p, step));
                let age = p.row_age();
                let pos = p.player.position;
                let services = match (p.stars, pos.is_specialist()) {
                    (0, false) => "an unranked recruit".to_string(),
                    (0, true) => format!("an unranked {}", pos.name()),
                    (s, false) => format!("a {s}-star"),
                    (s, true) => format!("a {s}-star {}", pos.name()),
                };
                // A specialist's camp stars are the kicking and snapping camps'
                // own (fix list 1.5), named only when they differ from the services'.
                let stars = match p.camp_stars {
                    c if c > 0 && c != p.stars && pos.is_specialist() => {
                        format!("{services} (a {c}-star by the specialist camps)")
                    }
                    _ => services,
                };
                let late_when = format!(" in {}, after signing day", month_name(today.month()));
                let when = match stage {
                    SignStage::Early => " in the early signing period",
                    SignStage::JanuaryEnrollees => " as a January enrollee",
                    SignStage::Late(_) => late_when.as_str(),
                    _ if esp_era => " on February signing day",
                    _ => "",
                };
                if r.juco_mover {
                    p.note(format!(
                        "At {age} his JUCO film earned a move up to {prog_name} ({}){when}.",
                        division.name()
                    ));
                } else {
                    let gpa = p.bg.gpa;
                    p.gpa_at_signing = Some(gpa);
                    if walk_on {
                        p.note(format!(
                            "Walked on at {prog_name} ({}) at {age} as {stars} out of {from_hs}: a roster spot without a scholarship (core GPA {gpa:.2}).",
                            division.name()
                        ));
                    } else {
                        p.note(format!(
                            "Signed with {prog_name} ({}) at {age} as {stars} out of {from_hs}{when} (core GPA {gpa:.2}).",
                            division.name()
                        ));
                    }
                    let nil = held_offer[k];
                    if nil >= 10_000.0 && super::ncaa::nil_era(year) {
                        if w.recording {
                            w.programs[j].era.offers.push((year, p.player.position, p.stars, nil));
                        }
                        let amount = if nil >= 1.0e6 {
                            format!("${:.1}M", nil / 1.0e6)
                        } else {
                            format!("${:.0}K", nil / 1.0e3)
                        };
                        let why = if reach[j] { " in a big reach by a rising program" } else { "" };
                        p.note(format!("His NIL deal there: about {amount} a year{why}."));
                    }
                    if offer_names.len() > 1 {
                        let n = offer_names.len();
                        let top: Vec<&str> = offer_names.iter().take(4).map(String::as_str).collect();
                        p.note(format!("He had {n} offers, the biggest from {}.", top.join(", ")));
                    }
                    p.college_seasons = 0;
                    p.redshirt_used = false;
                    match ncaa_tier(division, eligibility_gpa(gpa, p.cal.test_score, sign_year)) {
                        Eligibility::AcademicRedshirt if division != Division::JUCO => {
                            p.academic_redshirt = true;
                            p.note("He signed as an academic redshirt: on scholarship and practicing, but he can't play in his first year.");
                        }
                        _ => {}
                    }
                    if division == Division::JUCO && gpa < 2.0 {
                        p.note(format!(
                            "His core GPA ({gpa:.2}) was under the NCAA's 2.0 floor, so Division I and II schools couldn't take him."
                        ));
                    }
                    if special {
                        p.note(format!(
                            "{prog_name} used one of its few special admissions on him: his grades were below the school's bar, but trending up."
                        ));
                    }
                }
                if division == Division::JUCO {
                    p.juco = true;
                }
                if steered {
                    p.steered = true;
                    // Named for the school: a JUCO player who moves up signs
                    // again (minor bug 116: the line repeated word for word).
                    let whose = match p.family.decider {
                        Decider::Parent => "a parent's choice",
                        Decider::Grandparent => "his grandparents' choice",
                        Decider::Handler => "the choice of a handler close to the family",
                        Decider::Himself => "someone else's choice",
                    };
                    p.note_again(format!("{prog_name} was more {whose} than his own."), || {
                        format!("{prog_name} was again more {whose} than his own.")
                    });
                }
                let flipped = p.cal.commit.is_some_and(|c| c.0 != j);
                let committed = p.cal.commit.filter(|c| c.0 == j).map(|c| c.1);
                p.cal.signed =
                    Some(super::Signed { program: j, walk_on, date: today, early_enrollee, flipped, committed });
                p.cal.commit = None;
                p.cal.late = false;
                // His high school coach gets the credit for sending him up.
                let (hs, group) = (p.hs, super::coach::Group::of(p.player.position));
                let up = match w.programs[j].division {
                    Division::FBS => 1.0,
                    Division::FCS => 0.5,
                    _ => 0.25,
                };
                if w.recording
                    && !r.juco_mover
                    && let Some(c) = w.high_schools[hs].coach
                {
                    w.coaches[c].units[group as usize].moved_up += up;
                }
                if let Some(why) = why {
                    w.log_why(why);
                }
                let (stars_n, name) = (w.people[i].stars, w.people[i].name.clone());
                if stars_n >= 4 && !r.juco_mover {
                    let text = format!("{stars_n}-star {} {name} signs with {prog_name}.", w.people[i].player.position);
                    let loud = if stars_n == 5 { Loudness::Medium } else { Loudness::Small };
                    let id = w.people[i].player.id;
                    w.calendar.post(
                        Event::new(today, CalLevel::Recruiting, Kind::Signing, loud, text)
                            .program(j)
                            .person(id, &name)
                            .school_link(j, &prog_name),
                    );
                }
            }
            None => {
                // A late day that isn't the last: he keeps looking (lane B).
                if late_day && !last_day {
                    continue;
                }
                if committed.is_some() {
                    w.log.decommits += 1;
                    w.log.blue_broken += (recruiting_stars(&w.people[i]) >= 4) as u32;
                }
                let fell_through = committed.map(|c| w.program_name(c).to_string());
                // Unsigned on signing day with a staff that had him on its
                // board or offered: he keeps looking until summer
                // enrollment (lane B). A junior college player with a year
                // left there stays as before.
                let finishing = !r.juco_mover || sign_year - w.people[i].class_of >= 2;
                let offered = !offers[k].is_empty()
                    || !w.people[i].cal.offers.is_empty()
                    || !w.people[i].cal.early_offers.is_empty();
                if stage == SignStage::Final && finishing && keeps_looking(on_board[k], offered) {
                    let p = &mut w.people[i];
                    p.cal.commit = None;
                    p.cal.late = true;
                    if let Some(school) = &fell_through {
                        let age = p.row_age();
                        p.note(format!(
                            "His commitment to {school} fell through on signing day at {age}, and he kept looking for a place."
                        ));
                    }
                    late_pool += 1;
                    continue;
                }
                let p = &mut w.people[i];
                p.cal.commit = None;
                p.cal.late = false;
                if r.juco_mover {
                    if sign_year - p.class_of >= 2 {
                        p.stage = Stage::Done;
                        p.note("Never earned a move up from JUCO and left football.");
                    }
                    continue;
                }
                if p.stage == Stage::PrepSchool {
                    p.stage = Stage::Done;
                    p.note("A prep school year didn't bring an offer; he left football.");
                    continue;
                }
                // A kid with college film but bad grades can spend a year at prep school.
                let good = r.film >= all_mean_sd.0 + 2.2 * all_mean_sd.1;
                if good && p.bg.gpa < 2.3 && p.rng.chance(0.25 + 0.4 * p.bg.money) {
                    p.stage = Stage::PrepSchool;
                    p.bg.gpa = clamp(p.bg.gpa + p.rng.range(0.1, 0.6), 0.0, 4.0);
                    let age = p.row_age();
                    p.note(match &fell_through {
                        Some(school) => format!(
                            "His commitment to {school} fell through; at {age} he went to prep school for a year to fix his grades."
                        ),
                        None => format!("At {age} he went to prep school for a year to fix his grades."),
                    });
                } else {
                    p.stage = Stage::Done;
                    // A ranked recruit who never signs gets a reason (trial 1
                    // flag 5), and so does one whose commitment fell through.
                    if fell_through.is_some() || recruiting_stars(p) >= 3 {
                        let gpa = p.bg.gpa;
                        let why = if gpa < 2.0 {
                            format!(
                                "With a core GPA of {gpa:.2}, under the NCAA's 2.0 floor, no four-year school could take him, and no junior college place came through; he stopped playing."
                            )
                        } else if let Some(school) = &fell_through {
                            format!(
                                "His commitment to {school} fell through, and no other program had room for him; he stopped playing after high school."
                            )
                        } else if offers[k].is_empty() && p.cal.offers.is_empty() {
                            // (`cal.offers` holds the offers of the stage before.)
                            "No program offered him, and he stopped playing after high school.".to_string()
                        } else {
                            "None of his offers worked out, and he stopped playing after high school.".to_string()
                        };
                        p.note(why);
                    }
                }
            }
        }
    }
    if stage == SignStage::Final || last_day {
        // Seniors nobody signed simply finish high school; on signing day
        // the ones still looking keep going until summer enrollment (lane B).
        for p in w.people.iter_mut() {
            if p.stage == Stage::HighSchool
                && p.class_of <= sign_year
                && p.cal.signed.is_none()
                && (last_day || !p.cal.late)
            {
                p.stage = Stage::Done;
            }
            if last_day {
                p.cal.late = false;
            }
        }
    }
    if stage == SignStage::Final {
        w.log.late_pool += late_pool;
    }
    if last_day {
        w.late_signing = None;
    }
    let _ = n;
    if let Some(j) = mj {
        fo::college::after_stage(w, j, &coordinator_offers, final_like);
    }
}

/// Signees arrive on campus (T3): January enrollees for the spring semester
/// and spring practice, everyone else for summer school in June.
pub(crate) fn enroll(w: &mut World, early: bool) {
    let mut n = 0;
    for i in 0..w.people.len() {
        let Some(s) = w.people[i].cal.signed else { continue };
        if early && !s.early_enrollee {
            continue;
        }
        let name = w.program_name(s.program).to_string();
        let p = &mut w.people[i];
        if p.stage == Stage::College {
            // A junior college player moving up keeps two seasons at most behind him.
            p.college_seasons = p.college_seasons.min(2);
        }
        p.stage = Stage::College;
        p.college = Some(s.program);
        p.path.push(s.program);
        p.on_notice = false;
        p.cal.signed = None;
        if early {
            let age = p.row_age();
            p.note(format!("Enrolled early at {name} in January at {age}, in time for spring practice."));
        }
        n += 1;
    }
    if early {
        w.log.early_enrollees += n;
    }
    w.rebuild_rosters();
    // Your signees start on what they signed for (every NIL dollar is yours).
    fo::college::fill_deals(w);
    // An early enrollee's collective pays him from January, on the split
    // July 1 will make (minor bug 100: he held nothing until July); yours
    // on the deal he signed.
    if early {
        let mine = fo::program(w);
        let medians = nil_medians(w);
        let none = std::collections::HashSet::new();
        for j in 0..w.programs.len() {
            let fresh: Vec<usize> = w.programs[j]
                .roster
                .iter()
                .copied()
                .filter(|&i| {
                    w.people[i].nil_income == 0.0
                        && w.people[i].player.history.last().is_none_or(|h| !h.level.is_college())
                })
                .collect();
            if fresh.is_empty() || w.programs[j].collective.budget <= 0.0 {
                continue;
            }
            let div = w.programs[j].division;
            let pool = nil_pool(w, j, &none, &medians);
            let budget = w.programs[j].collective.budget * 0.6 * super::legacy::college::split_q(w, j, 0);
            let today = w.today;
            for i in fresh {
                let v = nil_value(&w.people[i], div, &medians, super::ncaa::two_pots(today));
                // The collective's part through NIL Go (B18).
                // Sandbox mode's NIL Go switch: your collective's deals
                // clear in full.
                let free = nil_go_free(w) == Some(j);
                let money = match fo::college::deal_of(w, w.people[i].player.id).filter(|_| mine == Some(j)) {
                    Some((s, c)) if super::ncaa::two_pots(today) => {
                        s + if free { c } else { nil_go(today, c, nil_go_top(&w.people[i], v)) }
                    }
                    Some((s, _)) => s,
                    // Yours with no deal: the game names no money for you
                    // (Cain 2026-11-08), as July 1 pays him nothing.
                    None if mine == Some(j) => 0.0,
                    None => {
                        cleared(&w.programs[j], today, budget * v / pool.max(1e-9), nil_go_top(&w.people[i], v), free)
                    }
                };
                let p = &mut w.people[i];
                if money > 250_000.0 {
                    p.cal.payday = true;
                }
                p.nil_income = money + p.brand as f64;
            }
        }
    }
}

// --- The transfer portal (T3) ---------------------------------------------------------

/// Why a player leaves (T3).
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
enum Why {
    PlayingTime,
    CoachingChange,
    Steered,
    MoveUp,
    Dismissed,
    Money,
    SpringDepth,
    Graduate,
    /// His school dropped football: free to play at once anywhere (NCAA waiver).
    ProgramDropped,
    /// His coach cut him (minor bug 60): he moves at the next window.
    Cut,
}

impl Why {
    fn reason(self) -> &'static str {
        match self {
            Why::PlayingTime => "playing time",
            Why::CoachingChange => "coaching change",
            Why::Steered => "steered",
            Why::MoveUp => "move up",
            Why::Dismissed => "dismissed",
            Why::Money => "money",
            Why::SpringDepth => "spring depth chart",
            Why::Graduate => "graduate transfer",
            Why::ProgramDropped => "program dropped",
            Why::Cut => "cut",
        }
    }
    fn phrase(self) -> &'static str {
        match self {
            Why::PlayingTime => "short of snaps",
            Why::CoachingChange => "after a coaching change",
            Why::Steered => "free at last to choose for himself",
            Why::MoveUp => "after standing out a level below",
            Why::Dismissed => "after his dismissal",
            Why::Money => "for a bigger NIL deal",
            Why::SpringDepth => "after losing the spring depth chart battle",
            Why::Graduate => "as a graduate transfer",
            Why::ProgramDropped => "after his school dropped football",
            Why::Cut => "after he was cut",
        }
    }
    fn from_reason(r: &str) -> Why {
        match r {
            "coaching change" => Why::CoachingChange,
            "steered" => Why::Steered,
            "move up" => Why::MoveUp,
            "dismissed" => Why::Dismissed,
            "money" => Why::Money,
            "spring depth chart" => Why::SpringDepth,
            "graduate transfer" => Why::Graduate,
            "program dropped" => Why::ProgramDropped,
            "cut" => Why::Cut,
            _ => Why::PlayingTime,
        }
    }
}

/// The market a window opens: every program's room without this window's
/// entrants, updated as each one lands (11.2, 11.16, fix list 1.10).
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct PortalMarket {
    /// The offseason it belongs to (the season just played) and the season the entrants play next.
    label: i32,
    pub(crate) season: i32,
    serious: std::collections::HashSet<usize>,
    medians: [[f64; POSITION_COUNT]; DIVISION_COUNT],
    floors: [[f64; POSITION_COUNT]; DIVISION_COUNT],
    open: Vec<[usize; POSITION_COUNT]>,
    backups_best: Vec<[f64; POSITION_COUNT]>,
    depth: Vec<Vec<Vec<f64>>>,
    adds: Vec<usize>,
    /// Each program's NIL payroll weight without this window's entrants
    /// (`nil_pool`), so an entrant's figure follows his market there.
    #[serde(default)]
    nil_pool: Vec<f64>,
    view: RosterView,
    /// Entrants from teams still alive for the title, on hold until their
    /// season is over (T3).
    held: Vec<Held>,
    /// Open starting spots the cycle's transfers filled who didn't start
    /// where they came from: a fresh read of the rosters doesn't count
    /// them as returning starters, so they stay filled (minor bug 337).
    #[serde(default)]
    filled: Vec<[usize; POSITION_COUNT]>,
}

impl PortalMarket {
    /// The open starting spots less those the cycle's transfers filled.
    fn keep_filled(&mut self) {
        for (o, f) in self.open.iter_mut().zip(&self.filled) {
            for k in 0..POSITION_COUNT {
                o[k] = o[k].saturating_sub(f[k]);
            }
        }
    }

    /// A program's NIL payroll weight (none kept in an older file's market).
    fn pool(&self, j: usize) -> f64 {
        self.nil_pool.get(j).copied().unwrap_or(0.0)
    }
}

/// The role a transfer would have with `ahead` players better than him at
/// a position with `slots` starting places.
fn role_at(ahead: usize, slots: usize) -> PlayingTime {
    if ahead < slots {
        PlayingTime::Starter
    } else if ahead < 2 * slots {
        PlayingTime::Rotation
    } else {
        PlayingTime::Backup
    }
}

/// A playoff team's entrant on hold (T3): the day the window would have
/// given him, his extra days after the last game (0-4), and the days he
/// takes to decide once in.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
struct Held {
    person: usize,
    program: usize,
    window_day: Date,
    extra: i32,
    decide_in: i32,
    why: Why,
    /// Your own player's bar (`Intent`): he talks to you once he's out.
    #[serde(default)]
    bar: Option<f64>,
}

/// The entry dates of a held entrant until he is released: no decision is
/// due before then.
pub(crate) const HELD: Date = Date(i32::MAX / 2);

/// One of your players who wants to leave (your program, as head coach):
/// he asks to see you first, and on his day he enters only if his lean is
/// still above the bar his draw set (`bar`), so the talk can change his mind.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Intent {
    id: u32,
    program: usize,
    /// The day of his talk with you, and the day he decides.
    talk: Date,
    enters: Date,
    decide_in: i32,
    why: Why,
    bar: f64,
}

impl Intent {
    pub(crate) fn is(&self, id: u32) -> bool {
        self.id == id
    }
}

/// His lean as it stood on `d`: the product of its dated factors (the fall
/// camp base comes first), or none before fall camp.
fn lean_on(p: &Person, d: Date) -> f64 {
    let mut seen = p.cal.lean_events.iter().filter(|e| e.0 <= d).peekable();
    if seen.peek().is_none() {
        return 0.0;
    }
    seen.map(|e| e.2).product()
}

/// Why your player means to enter the portal, as the word his talk's
/// opening checks (`reason=snaps`), while his talk with you is pending.
pub(crate) fn intent_reason(w: &World, id: u32) -> Option<&'static str> {
    w.portal_intents.iter().find(|t| t.is(id)).map(|t| match t.why {
        Why::PlayingTime | Why::SpringDepth => "snaps",
        Why::CoachingChange => "coach",
        Why::Steered => "choice",
        Why::MoveUp => "up",
        Why::Money => "money",
        Why::Graduate => "grad",
        Why::Dismissed | Why::Cut | Why::ProgramDropped => "other",
    })
}

/// Your program, when you are its head coach.
fn talks_first(w: &World) -> Option<usize> {
    fo::program(w)
}

/// Your players who want to leave ask to see you before they enter (Cain
/// 2026-10-28: "players should contact me"). Up to six talks a day, the
/// best first; one already on your calendar counts. Each decides the day
/// after his talk at the earliest. Proposed, not confirmed: six a day.
fn ask_first(w: &mut World, mut list: Vec<(usize, Date, i32, Why, f64)>) {
    list.sort_by_key(|x| (x.1, x.0));
    let today = w.today;
    for (k, (i, entered_on, decide_in, why, bar)) in list.into_iter().enumerate() {
        let id = w.people[i].player.id;
        let Some(program) = w.people[i].college else { continue };
        let booked = w
            .folk
            .scenes
            .iter()
            .filter(|s| !s.done && s.kind == super::folk::scene::Kind::OneOnOne && s.a == id as u64 && s.date >= today)
            .map(|s| s.date)
            .min();
        let talk = match booked {
            Some(d) => d,
            None => {
                let d = today.add(k as i32 / 6);
                super::folk::talks::portal_ask(w, i, d);
                d
            }
        };
        let enters = entered_on.max(talk.add(1));
        w.portal_intents.push(Intent { id, program, talk, enters, decide_in, why, bar });
    }
}

/// Your players' decisions on their day (after their talk with you): in,
/// or staying.
pub(crate) fn intents_day(w: &mut World) {
    let today = w.today;
    let (due, rest): (Vec<Intent>, Vec<Intent>) =
        std::mem::take(&mut w.portal_intents).into_iter().partition(|t| t.enters <= today);
    w.portal_intents = rest;
    for t in due {
        let Some(i) = super::folk::person(w, t.id) else { continue };
        let p = &w.people[i];
        if p.stage != Stage::College || p.college != Some(t.program) || p.cal.portal.is_some() {
            continue;
        }
        let link = super::folk::player_link(w, i);
        if p.cal.lean > t.bar {
            if let Some(e) = enter_portal(w, i, t.program, today, today.add(t.decide_in), t.why) {
                w.calendar.post(e);
            }
            w.portal_pending.push(i);
            w.log.portal_entries += 1;
            match w.log.portal_by_day.iter_mut().find(|x| x.0 == today) {
                Some(x) => x.1 += 1,
                None => w.log.portal_by_day.push((today, 1)),
            }
            if let Some(cs) = w.college.as_mut() {
                cs.out.insert(t.id, "entered the transfer portal");
            }
            super::folk::post_mine(
                w,
                today,
                Loudness::Small,
                false,
                "people: portal",
                &format!("{link} entered the transfer portal."),
            );
        } else {
            w.people[i].cal.lean_events.push((today, "decided to stay after talking it over", 1.0));
            super::folk::post_mine(
                w,
                today,
                Loudness::Small,
                false,
                "people: portal",
                &format!("{link} decided to stay."),
            );
        }
    }
}

/// A player's portal entry is set (T3): his NIL deal ends, and a starter's
/// or blue-chip's entry is news on his day.
fn enter_portal(w: &mut World, i: usize, c: usize, entered: Date, decide_by: Date, why: Why) -> Option<Event> {
    if w.logging() {
        let p = &w.people[i];
        w.log_why(super::wlog::Why::PortalEntry {
            id: p.player.id,
            program: Some(c),
            why: why.reason(),
            lean: p.cal.lean,
            events: p.cal.lean_events.clone(),
            role: p.last_role,
            held: false,
        });
    }
    // His NIL deals end when he enters (T3): his pay, and the deals your desk
    // counts against your budget while he shops (minor bug 375).
    let id = w.people[i].player.id;
    fo::college::drop_deals(w, id);
    let p = &mut w.people[i];
    p.cal.portal = Some(super::PortalEntry { entered, decide_by, why: why.reason(), held: false, looking: false });
    if p.nil_income > 0.0 {
        p.nil_income = 0.0;
        let age = p.row_age();
        p.note_again(format!("Lost his NIL deal when he entered the portal at {age}."), || {
            format!("Lost his new NIL deal when he entered the portal again at {age}.")
        });
    }
    if p.stars < 4 && p.last_role != Some(PlayingTime::Starter) {
        return None;
    }
    let text = format!("{} ({}, {}) enters the transfer portal.", p.name, p.player.position, w.programs[c].real_name);
    let loud =
        if p.stars >= 4 && p.last_role == Some(PlayingTime::Starter) { Loudness::Medium } else { Loudness::Small };
    Some(
        Event::new(entered, CalLevel::College, Kind::Transfer, loud, text)
            .program(c)
            .person(p.player.id, &p.name)
            .school_link(c, &w.programs[c].real_name),
    )
}

/// Adds one to a day's count (part B 20).
fn bump_day(days: &mut Vec<(Date, u32)>, date: Date) {
    match days.iter_mut().find(|x| x.0 == date) {
        Some(x) => x.1 += 1,
        None => days.push((date, 1)),
    }
}

/// Held playoff entrants whose season is over (a loss or the title) enter
/// within five days of that last game, or on their window day if later (T3,
/// the 2024-25 rule).
fn release_held(w: &mut World, market: &mut PortalMarket) {
    if market.held.is_empty() {
        return;
    }
    let mut out: Vec<(Held, Date)> = Vec::new();
    let mut still = Vec::new();
    let field = w.rules.cfp_teams as usize;
    for h in std::mem::take(&mut market.held) {
        let alive = w.college.as_ref().is_some_and(|cs| super::college::still_contending(cs, h.program, field));
        if alive {
            still.push(h);
            continue;
        }
        let last = w
            .college
            .as_ref()
            .and_then(|cs| {
                cs.games
                    .iter()
                    .filter(|g| {
                        g.involves(h.program) && g.played.is_some() && g.kind != super::schedule::GameKind::Regular
                    })
                    .map(|g| g.date)
                    .max()
            })
            .unwrap_or(w.today);
        out.push((h, last));
    }
    market.held = still;
    let mine = talks_first(w);
    let mut ask = Vec::new();
    for (h, last) in out {
        let entered = h.window_day.max(last.add(1 + h.extra));
        if let Some(bar) = h.bar.filter(|_| mine == Some(h.program)) {
            // The hold is over but he has not entered: like a player on a team
            // outside the playoff he has no entry until his talk with you.
            // His held entry would make `intents_day` skip him, and he would
            // read "In the portal" until fall camp (minor bug 372).
            w.people[h.person].cal.portal = None;
            ask.push((h.person, entered, h.decide_in, h.why, bar));
            continue;
        }
        if let Some(e) = enter_portal(w, h.person, h.program, entered, entered.add(h.decide_in), h.why) {
            w.calendar.post(e);
        }
        if let Some(e) = w.people[h.person].cal.portal.as_mut() {
            e.held = true;
        }
        w.log.portal_entries += 1;
        match w.log.portal_by_day.iter_mut().find(|x| x.0 == entered) {
            Some(x) => x.1 += 1,
            None => w.log.portal_by_day.push((entered, 1)),
        }
    }
    ask_first(w, ask);
}

/// How much of the yearly transfer flow an era had (T11 and part B 10;
/// proposed, not confirmed): before the portal a transfer needed his
/// school's release and sat a year, so about half as many moved; the first
/// portal years about four in five. The one-time free transfer and NIL
/// (2021) set the 2022 flow (NCAA: 2,918 FBS entrants); revenue sharing and
/// roster limits grew it to about 4,200 (2024-25). From the 2025 season
/// the single January window takes the players who would have waited for
/// spring.
fn era_factor(label: i32, rule: crate::calendar::rules::TransferRule) -> f64 {
    use crate::calendar::rules::TransferRule;
    match rule {
        TransferRule::Permission => 0.5,
        TransferRule::OpenPortal if label < 2020 => 0.8,
        TransferRule::OpenPortal if label < 2021 => 0.9,
        _ if label <= 2022 => 1.0,
        _ if label == 2023 => 1.2,
        TransferRule::January => 1.45 * 1.3,
        _ => 1.45,
    }
}

/// How close a player must be to a head coach to follow him to his new
/// college head job: about a player he recruited with a season under him,
/// or three seasons under him (`folk::takeover::tie_to_old`; minor bug 86,
/// proposed, not confirmed).
const FOLLOW_TIE: f32 = 0.4;
/// How much his head coach's call to come along moves a tied player's lean:
/// times 1 + this × his tie × his place there (a starter 1, a rotation
/// player 0.5) (minor bug 86, proposed, not confirmed).
const FOLLOW_LEAN: f64 = 3.0;
/// The lean event of a head coach's call to come along.
const FOLLOW: &str = "his old head coach wants him at his new school";

/// A head coach leaves program `a` for the head job at program `b` (minor
/// bug 86): the players he is close to who would start or rotate at `b`
/// lean toward following him, and `b` wants them whatever its board says
/// (the pull of a placement with a friend, C16, at the head coach's
/// scale). Real: Germie Bernard followed Kalen DeBoer from Washington to
/// Alabama (2024); Princewill Umanmielen and Devin Harper followed Lane
/// Kiffin from Ole Miss to LSU (2026); most moves take a few players.
pub(crate) fn follow_coach(w: &mut World, a: usize, b: usize, coach: usize) {
    let label = if w.today < w.dates.fall_camp { w.year - 1 } else { w.year };
    let (div_a, div_b) = (w.programs[a].division, w.programs[b].division);
    // Players follow a coach up or across, not down a level: a coach who
    // takes an FCS job takes nobody from his FBS roster. A fired coach never
    // gets here (`fill_opening` asks only a coach who still held his job).
    if div_b == Division::JUCO || div_b as usize > div_a as usize {
        return;
    }
    let perf_of = |p: &Person| p.last_performance.unwrap_or(p.player.overall());
    // The new school's players who return, best first, by position.
    let mut depth: [Vec<f64>; POSITION_COUNT] = Default::default();
    for p in w.people.iter() {
        if p.stage == Stage::College
            && p.college == Some(b)
            && p.cal.portal.is_none()
            && p.cal.declare_plan != Some(true)
            && !super::college::no_season_left(p, div_b, label)
        {
            depth[p.player.position.index()].push(perf_of(p));
        }
    }
    let today = w.today;
    let mut follows: Vec<(usize, f32, f64)> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        if p.stage != Stage::College
            || p.college != Some(a)
            || (p.cal.lean <= 0.0 && p.cal.portal.is_none())
            || p.cal.declare_plan == Some(true)
            || super::college::no_season_left(p, div_a, label)
        {
            continue;
        }
        let tie = super::folk::takeover::tie_to_old(w, i, Some(coach));
        if tie < FOLLOW_TIE {
            continue;
        }
        let pos = p.player.position;
        let perf = perf_of(p) - competition_discount(div_a) + competition_discount(div_b);
        let ahead = depth[pos.index()].iter().filter(|&&v| v >= perf).count();
        let place = match role_at(ahead, crate::league::depth_slots(pos).starters) {
            PlayingTime::Starter => 1.0,
            PlayingTime::Rotation => 0.5,
            _ => continue,
        };
        follows.push((i, tie, place));
    }
    for (i, tie, place) in follows {
        let p = &mut w.people[i];
        if p.cal.portal.is_none() {
            super::college::lean_push(p, today, FOLLOW, 1.0 + FOLLOW_LEAN * tie as f64 * place);
        }
        let id = p.player.id;
        w.coach_follows.retain(|f| f.0 != id);
        w.coach_follows.push((id, b, 0.08 + 0.08 * tie));
    }
}

/// A transfer window opens (T3): who enters, on which day, and when each
/// one decides. Stars and standouts enter on the first days; fringe players
/// trickle in. A head coach talks some of his rotation players into staying
/// first. The spring window takes players who lost the spring depth battle.
pub(crate) fn portal_open(w: &mut World, which: super::clock::PortalWindow, close: Date) {
    use super::clock::PortalWindow;
    let today = w.today;
    // The offseason it belongs to: the winter window follows this season; the
    // spring and graduate windows come in the next football year, and so
    // does a coaching-change window from a hire after the year turned (your
    // February takeover). It belonged to the coming season, so every player
    // drew afresh after the January window and juniors counted as out of
    // eligibility.
    let label = match which {
        PortalWindow::Spring | PortalWindow::Graduate => w.year - 1,
        PortalWindow::CoachChange if today < w.dates.fall_camp => w.year - 1,
        _ => w.year,
    };
    let rule = w.changes.rules(label).transfer;
    let era = era_factor(label, rule);
    let seed = w.options.seed as u64 ^ ((label as u64) << 24) ^ if which == PortalWindow::Spring { 0x5A } else { 0xA5 };
    let n = w.people.len();
    w.rebuild_rosters();
    // Position medians of starters' play by division, the bar to move up.
    let mut starters: [[Vec<f64>; POSITION_COUNT]; DIVISION_COUNT] = Default::default();
    for p in w.people.iter() {
        if p.stage == Stage::College
            && p.last_role == Some(PlayingTime::Starter)
            && let (Some(c), Some(perf)) = (p.college, p.last_performance)
        {
            starters[w.programs[c].division as usize][p.player.position.index()].push(perf);
        }
    }
    let mut medians = [[99.0; POSITION_COUNT]; DIVISION_COUNT];
    // The weakest quarter of a level's starters: a program at the bottom of
    // a level still takes an entrant who would play for it.
    let mut floors = [[99.0; POSITION_COUNT]; DIVISION_COUNT];
    for (d, list) in starters.iter_mut().enumerate() {
        for (k, v) in list.iter_mut().enumerate() {
            v.sort_by(|a, b| a.total_cmp(b));
            if !v.is_empty() {
                medians[d][k] = v[v.len() / 2];
                floors[d][k] = v[v.len() / 4];
            }
        }
    }
    let median = |d: Division, pos: Position| medians[d as usize][pos.index()];
    // A head coach meets the players he wants to keep before the window (T3).
    if which == PortalWindow::Winter {
        let sway: Vec<f64> = w
            .programs
            .iter()
            .map(|p| {
                p.head_coach
                    .map_or(1.0, |c| clamp(1.0 - 0.03 * (w.coaches[c].ability().relationships - 10.0), 0.75, 1.25))
            })
            .collect();
        // Your own program (lane 10): the players you sat down with get
        // your word, whatever their role; the rotation players you didn't
        // meet get none.
        let talks = fo::college::stay_talks(w);
        for p in w.people.iter_mut() {
            let met = talks.as_ref().and_then(|t| (p.college == Some(t.0)).then(|| t.1.get(&p.player.id).copied()));
            // With your staff on the portal, the players you didn't meet get
            // its word, as at any program (SC6).
            let met = if talks.as_ref().is_some_and(|t| t.2) && met == Some(None) { None } else { met };
            if p.stage == Stage::College
                && p.cal.lean > 0.0
                && (p.last_role == Some(PlayingTime::Rotation) || met.flatten().is_some())
                && let Some(c) = p.college
            {
                let f = match met {
                    Some(Some(f)) => f,
                    Some(None) => 1.0,
                    None => sway[c],
                };
                let why = if f < 1.0 {
                    "his head coach asked him to stay"
                } else {
                    "his head coach didn't fight to keep him"
                };
                p.cal.lean *= f;
                p.cal.lean_events.push((today, why, f));
            }
        }
    }
    let mut entrants: Vec<(usize, Why)> = Vec::new();
    // The lean each lean-driven entrant must still be above on his day
    // (`Intent`): his draw over the window's rate.
    let mut bars: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();
    let mut moved_up_far = 0u32;
    let mut steered_stays: Vec<usize> = Vec::new();
    // A coach-change window is only for the players whose head coach left.
    let new_coach: Vec<bool> = w.programs.iter().map(|p| p.hc_changed == w.year).collect();
    // The winter window this coaching-change window follows, when it came
    // first in the same cycle (your February takeover, or a head coach who
    // left after it: minor bug 88).
    let late = (which == PortalWindow::CoachChange)
        .then(|| super::clock::portal_windows(label, rule).into_iter().find(|x| x.2 == PortalWindow::Winter))
        .flatten()
        .map(|x| x.0)
        .filter(|&open| open < today);
    // Spring (part D 20): who was passed at his spot by a newcomer, a winter
    // transfer or a January enrollee, with the spring depth chart.
    let mut newcomer_best = vec![[f64::NEG_INFINITY; POSITION_COUNT]; w.programs.len()];
    if which == PortalWindow::Spring {
        for p in w.people.iter() {
            if p.stage == Stage::College
                && let Some(c) = p.college
                && ((p.class_of == w.year && p.path.len() == 1) || p.pending_dip.is_some())
            {
                let k = p.player.position.index();
                newcomer_best[c][k] = newcomer_best[c][k].max(p.player.overall());
            }
        }
    }
    for i in 0..n {
        let p = &w.people[i];
        if p.stage != Stage::College || p.cal.portal.is_some() {
            continue;
        }
        let Some(c) = p.college else { continue };
        // Your own hire opens a window for your program alone (takeover
        // T-B): only that program's players, whatever the year's coach list
        // says.
        // A late window (minor bug 88) draws the programs whose head coach
        // left after the cycle's window opened.
        if which == PortalWindow::CoachChange
            && (if w.window_programs.is_empty() {
                w.folk.window_only.is_some_and(|x| x != c) || (!new_coach[c] && w.folk.window_only != Some(c))
            } else {
                !w.window_programs.contains(&c)
            } || !p.cal.lean_events.iter().any(|e| e.1 == super::college::COACH_GONE))
        {
            continue;
        }
        let div = w.programs[c].division;
        // Cut from the team (minor bug 60): he enters the first window
        // after the cut, unless his eligibility is done.
        if p.cut_here().is_some() {
            if !super::college::no_season_left(p, div, label) {
                entrants.push((i, Why::Cut));
            }
            continue;
        }
        // His draw for this window. It must be uniform: it was a squeezed
        // normal (most draws near 0.5), so an 18% lean came true about 3% of
        // the time and the portal ran at about a tenth of its real size
        // (part B 10).
        let u = key_uniform(seed, p.player.id as u64);
        // A star his school sold a head start on (C15) enters this winter;
        // a player placed with a friend a level down (C16), the next window.
        if matches!(which, PortalWindow::Winter | PortalWindow::Spring) && div != Division::JUCO {
            let id = p.player.id;
            if which == PortalWindow::Winter
                && super::folk::backchannel::sold(w, id, label)
                && !super::college::no_season_left(p, div, label)
            {
                entrants.push((i, Why::MoveUp));
                continue;
            }
            if super::folk::backchannel::placed(w, id) {
                entrants.push((i, Why::PlayingTime));
                continue;
            }
        }
        if which == PortalWindow::Graduate {
            // A graduate with a season left (part B 36): free to play at
            // once from 2006, so many backups and rotation players move for
            // a last year of playing time, and a few standouts move up.
            // Proposed, not confirmed: his lean, plus a small base for a
            // starter below FBS.
            let left = p.college_seasons < super::eligibility::seasons_allowed(label + 1, div)
                && label < super::eligibility::clock_last(p);
            if !p.cal.may_graduate || !left || div == Division::JUCO || p.cal.declare_plan == Some(true) {
                continue;
            }
            let free = if label + 1 >= 2006 { 1.0 } else { 0.3 };
            let starter_up = if p.last_role == Some(PlayingTime::Starter) && div != Division::FBS { 0.08 } else { 0.0 };
            if u < (1.2 * p.cal.lean + starter_up) * free * era {
                entrants.push((i, Why::Graduate));
                bars.insert(i, ((u / (free * era) - starter_up) / 1.2).max(-1.0));
            }
            continue;
        }
        if which == PortalWindow::Spring {
            // After spring ball: a backup who lost the battle, most often to
            // a newcomer who passed him (part D 20: the real spring window
            // draws about a quarter to a third of a cycle's entrants;
            // proposed, not confirmed: the winter rate when passed, half of
            // it otherwise).
            let passed = newcomer_best[c][p.player.position.index()] > p.player.overall();
            let f = if passed { 1.0 } else { 0.5 };
            if div != Division::JUCO
                && p.college_seasons < 4
                && matches!(p.last_role, Some(PlayingTime::Backup | PlayingTime::Rotation))
                && u < f * p.cal.lean * era
            {
                entrants.push((i, Why::SpringDepth));
                bars.insert(i, u / (f * era));
            }
            continue;
        }
        // A player whose eligibility ends with this season has nowhere to
        // play next year: the season in play counts before January 2
        // settles it (lane 10: seniors were entering with no year left).
        let done = super::college::no_season_left(p, w.programs[c].division, label);
        // His school drops football after this season (college history):
        // everyone with a season left is free to move, and nearly all do.
        if which == PortalWindow::Winter && !w.programs[c].plays(label + 1) && !done && p.cal.declare_plan != Some(true)
        {
            entrants.push((i, Why::ProgramDropped));
            continue;
        }
        if !done && p.player.history.last().is_some_and(|h| h.events.has(crate::player::DevelopmentEvent::Dismissed)) {
            entrants.push((i, Why::Dismissed));
            continue;
        }
        if done || div == Division::JUCO || p.cal.declare_plan == Some(true) {
            continue;
        }
        let pos = p.player.position;
        let perf = p.last_performance.unwrap_or(0.0);
        // The levels a standout can move up to, highest first (A3): an FCS
        // star to FBS; a Division II star straight to FBS (Ali Marpet and
        // Shaquille Leonard went to the NFL from D2, Jaxon Smith-Njigba's
        // kind of jump rarer) or to FCS; NAIA and Division III standouts to
        // FCS or Division II (Division III to Division I needs no waiver).
        let ups: &[Division] = match div {
            Division::FCS => &[Division::FBS],
            Division::D2 => &[Division::FBS, Division::FCS],
            Division::NAIA | Division::D3 => &[Division::FCS, Division::D2],
            _ => &[],
        };
        // Standouts below FBS get noticed; richer programs quietly reach out first (never punished).
        if let Some(&up) =
            ups.iter().find(|&&up| perf - competition_discount(div) + competition_discount(up) >= median(up, pos) + 1.0)
        {
            let tampered = w.programs[c].collective.budget < 2.0e6;
            let chance = PIPELINE.transfer_chance * if tampered { 1.6 } else { 1.0 } * era;
            if u < chance {
                entrants.push((i, Why::MoveUp));
                if up as usize + 1 < div as usize || matches!(div, Division::NAIA | Division::D3) {
                    moved_up_far += 1;
                }
                continue;
            }
        }
        // A kid steered to the wrong school decides once, after his first year, whether to leave.
        if p.steered && p.player.age >= 18 {
            if u < 0.45 * era {
                entrants.push((i, Why::Steered));
                continue;
            }
            steered_stays.push(i);
        }
        // The lean built over the season (T3).
        // A coaching-change window after this cycle's winter window (your
        // February takeover, a head coach who left after it) draws only from
        // the players who stayed through it: the share the change adds, over
        // those who stayed, on a fresh draw of its own (the winter window's
        // draw already said he stayed; a career's own seed changes it too).
        let (go, bar) = match late {
            Some(open) => {
                let stayed = (lean_on(p, open) * era).min(1.0);
                let now = (p.cal.lean * era).min(1.0);
                let u = key_uniform(seed ^ 0x1A7E, p.player.id as u64);
                if stayed >= 1.0 {
                    (false, 0.0)
                } else {
                    (u < (now - stayed) / (1.0 - stayed), (u * (1.0 - stayed) + stayed) / era)
                }
            }
            None => (u < p.cal.lean * era, u / era),
        };
        if go {
            let changed = p.cal.lean_events.iter().any(|e| e.1 == super::college::COACH_GONE);
            entrants.push((i, if changed { Why::CoachingChange } else { Why::PlayingTime }));
            bars.insert(i, bar);
            continue;
        }
        // A greedy starter at a poor program can be bought away.
        if p.last_role == Some(PlayingTime::Starter)
            && super::ncaa::nil_era(label)
            && w.programs[c].collective.budget < 1.0e6
            && div == Division::FBS
            && u < 0.06 * p.family.own_wants.money * 4.0
        {
            entrants.push((i, Why::Money));
        }
    }
    w.log.moved_up_far += moved_up_far;
    // Friends leaving (T3): a teammate from his high school going makes it easier to go.
    let mut gone_from: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    for &(i, _) in &entrants {
        let p = &w.people[i];
        gone_from.insert((p.college.unwrap_or(0), p.hs));
    }
    let entered: std::collections::HashSet<usize> = entrants.iter().map(|e| e.0).collect();
    if which == PortalWindow::Winter {
        for i in 0..n {
            let p = &w.people[i];
            if p.stage != Stage::College
                || entered.contains(&i)
                || p.cal.lean <= 0.0
                || p.cal.declare_plan == Some(true)
                || p.college.is_some_and(|c| super::college::no_season_left(p, w.programs[c].division, label))
            {
                continue;
            }
            let Some(c) = p.college else { continue };
            if !gone_from.contains(&(c, p.hs)) {
                continue;
            }
            let u = key_uniform(seed ^ 0x7EA, p.player.id as u64);
            if u < p.cal.lean * era * 1.15 {
                entrants.push((i, Why::PlayingTime));
                bars.insert(i, u / (era * 1.15));
                w.people[i].cal.lean_events.push((today, "a high school teammate left", 1.15));
            }
        }
    }
    for i in steered_stays {
        w.people[i].steered = false;
    }
    // The AI's schools sell head starts on their stars moving up (C15).
    if which == PortalWindow::Winter {
        let movers: Vec<usize> = entrants.iter().filter(|e| e.1 == Why::MoveUp).map(|e| e.0).collect();
        super::folk::backchannel::ai_head_starts(w, &movers, label);
    }
    if which == PortalWindow::Graduate {
        w.log.graduate_transfers += entrants.len() as u32;
    }
    if entrants.is_empty() {
        return;
    }
    // Dates: stars and standouts enter on the first days and decide fast.
    let len = today.days_to(close).max(0) as f64;
    // This window's entrants, and an earlier window's still looking (minor
    // bug 337): both are leaving their programs.
    let mut leaving: std::collections::HashSet<usize> = entrants.iter().map(|e| e.0).collect();
    leaving.extend(w.people.iter().enumerate().filter(|(_, p)| p.cal.portal.is_some()).map(|(i, _)| i));
    let mut by_day: std::collections::BTreeMap<Date, u32> = std::collections::BTreeMap::new();
    let mut serious = std::collections::HashSet::new();
    let mut events = Vec::new();
    // Teams still alive for the title (T3, the 2024-25 rule): their
    // entrants play the run out and are held until the season is over (a
    // loss or the title), then enter within five days of that last game.
    // Bowl teams' entrants sit the bowl out.
    let field = w.rules.cfp_teams as usize;
    let alive: Vec<bool> = match w.college.as_ref() {
        Some(cs) => (0..w.programs.len()).map(|t| super::college::still_contending(cs, t, field)).collect(),
        None => vec![false; w.programs.len()],
    };
    // Before the windows (part D 21) a player left once his own season was
    // over: each team's entrants follow its last game, bowl included.
    use crate::calendar::rules::TransferRule;
    let no_windows =
        matches!(rule, TransferRule::Permission | TransferRule::OpenPortal) && which == PortalWindow::Winter;
    let last_game: Vec<Date> = match (no_windows, w.college.as_ref()) {
        (true, Some(cs)) => {
            let mut v = vec![today; w.programs.len()];
            for g in &cs.games {
                for t in [g.home, g.away] {
                    v[t] = v[t].max(g.date);
                }
            }
            v
        }
        _ => vec![today; w.programs.len()],
    };
    let mut held: Vec<Held> = Vec::new();
    let mut cs_out: Vec<u32> = Vec::new();
    let mine = talks_first(w);
    let mut ask: Vec<(usize, Date, i32, Why, f64)> = Vec::new();
    for &(i, why) in &entrants {
        let c = w.people[i].college.unwrap();
        let div = w.programs[c].division;
        w.log.portal_by_division[div as usize] += 1;
        let p = &w.people[i];
        let perf = p.last_performance.unwrap_or(p.player.overall());
        let q = clamp((perf - median(div, p.player.position)) / 10.0 + 0.4 + 0.1 * p.stars as f64, 0.0, 1.0);
        let id = p.player.id as u64;
        let entered_on = if matches!(why, Why::Dismissed | Why::Cut) {
            today
        } else if no_windows {
            // Within about three weeks of his last game, sooner for the best.
            let after = last_game[c].max(today);
            after.add(1 + (20.0 * (1.0 - q).powi(2) * key_uniform(seed ^ 0xE7, id)).floor() as i32)
        } else {
            today.add((len * (1.0 - q).powi(2) * key_uniform(seed ^ 0xE7, id)).floor() as i32)
        };
        let decide_in = 3 + (22.0 * (1.0 - q) * key_uniform(seed ^ 0xD7, id)).round() as i32;
        if why == Why::Dismissed && w.people[i].rng.chance(PIPELINE.college_trouble.serious_share) {
            serious.insert(i);
        }
        let bar = bars.get(&i).copied().filter(|_| mine == Some(c));
        if alive[c]
            && !matches!(why, Why::Dismissed | Why::Cut)
            && !matches!(which, PortalWindow::Spring | PortalWindow::Graduate)
        {
            let extra = (5.0 * key_uniform(seed ^ 0xE8, id)).floor() as i32;
            held.push(Held { person: i, program: c, window_day: entered_on, extra, decide_in, why, bar });
            // On hold: no day yet, and no decision due until he is released.
            w.people[i].cal.portal = Some(super::PortalEntry {
                entered: HELD,
                decide_by: HELD,
                why: why.reason(),
                held: true,
                looking: false,
            });
            continue;
        }
        // Your own player asks to see you first and decides on his day.
        if let Some(bar) = bar {
            ask.push((i, entered_on, decide_in, why, bar));
            continue;
        }
        cs_out.push(id as u32);
        *by_day.entry(entered_on).or_default() += 1;
        events.extend(enter_portal(w, i, c, entered_on, entered_on.add(decide_in), why));
    }
    for e in events {
        w.calendar.post(e);
    }
    w.log.portal_entries += (entrants.len() - held.len() - ask.len()) as u32;
    w.log.portal_by_day.extend(by_day);
    // Players still to play a bowl or playoff game sit it out.
    if let Some(cs) = w.college.as_mut() {
        for id in cs_out {
            cs.out.insert(id, "entered the transfer portal");
        }
    }
    let text = match which {
        PortalWindow::Winter => format!("The transfer window opens until {}.", close.pretty()),
        PortalWindow::Spring => format!("The spring transfer window opens until {}.", close.pretty()),
        PortalWindow::Graduate => format!("{} graduates look for a last season elsewhere.", entrants.len()),
        PortalWindow::CoachChange => {
            format!("Players at programs with a new head coach may enter the portal until {}.", close.pretty())
        }
    };
    let loud = if which == PortalWindow::CoachChange { Loudness::Medium } else { Loudness::Big };
    w.calendar.post(Event::new(today, CalLevel::College, Kind::Window, loud, text).rule("NCAA: transfer window"));
    // The market every entrant shops in.
    let pn = w.programs.len();
    let MarketRooms { open, backups_best, depth, view, nil_pool } = market_rooms(w, &leaving);
    let mut market = PortalMarket {
        label,
        season: label + 1,
        serious,
        medians,
        floors,
        open,
        backups_best,
        depth,
        adds: vec![0; pn],
        nil_pool,
        view,
        held,
        filled: vec![[0; POSITION_COUNT]; pn],
    };
    // Entrants from an earlier window still deciding keep their place in
    // line, and playoff entrants stay on hold.
    if let Some(old) = w.portal_market.take() {
        market.adds = old.adds;
        market.held.extend(old.held);
        // An earlier entrant still shopping keeps his record, and the
        // spots the cycle's transfers filled stay filled (minor bug 337).
        market.serious.extend(old.serious);
        if old.filled.len() == pn {
            market.filled = old.filled;
            market.keep_filled();
        }
    }
    w.portal_market = Some(market);
    let asked: std::collections::HashSet<usize> = ask.iter().map(|x| x.0).collect();
    w.portal_pending.extend(entrants.iter().map(|e| e.0).filter(|i| !asked.contains(i)));
    ask_first(w, ask);
}

/// Every program's room in the portal market, without the players leaving
/// (`leaving`): its open starting spots, its best backups, its depth by
/// play and its roster view and NIL pool.
struct MarketRooms {
    open: Vec<[usize; POSITION_COUNT]>,
    backups_best: Vec<[f64; POSITION_COUNT]>,
    depth: Vec<Vec<Vec<f64>>>,
    view: RosterView,
    nil_pool: Vec<f64>,
}

fn market_rooms(w: &World, leaving: &std::collections::HashSet<usize>) -> MarketRooms {
    let pn = w.programs.len();
    let perf_of = |p: &Person| p.last_performance.unwrap_or(p.player.overall());
    let mut open = vec![[0usize; POSITION_COUNT]; pn];
    let mut backups_best = vec![[f64::NEG_INFINITY; POSITION_COUNT]; pn];
    let mut depth: Vec<Vec<Vec<f64>>> = vec![vec![Vec::new(); POSITION_COUNT]; pn];
    for (j, prog) in w.programs.iter().enumerate() {
        let mut returning = [0usize; POSITION_COUNT];
        for &i in prog.roster.iter().filter(|i| !leaving.contains(i)) {
            let p = &w.people[i];
            let k = p.player.position.index();
            depth[j][k].push(perf_of(p));
            if p.last_role == Some(PlayingTime::Starter) {
                returning[k] += 1;
            } else {
                backups_best[j][k] = backups_best[j][k].max(perf_of(p));
            }
        }
        for pos in Position::ALL {
            open[j][pos.index()] = crate::league::depth_slots(pos).starters.saturating_sub(returning[pos.index()]);
            depth[j][pos.index()].sort_by(|a, b| b.total_cmp(a));
        }
    }
    let view = RosterView::new(&w.programs, &w.people, leaving);
    let nil_medians = nil_medians(w);
    let nil_pool: Vec<f64> = (0..pn).map(|j| nil_pool(w, j, leaving, &nil_medians)).collect();
    MarketRooms { open, backups_best, depth, view, nil_pool }
}

/// The portal market read again from today's rosters (minor bug 337):
/// entrants shop until fall camp, and through the spring the rooms change
/// (signing day's book, players who leave, the freshmen who enroll in
/// June). The landings so far are on the rosters; the cycle's count of
/// adds per program stays.
pub(crate) fn refresh_market(w: &mut World) {
    if w.portal_market.is_none() || w.portal_pending.is_empty() {
        return;
    }
    w.rebuild_rosters();
    let leaving: std::collections::HashSet<usize> =
        w.people.iter().enumerate().filter(|(_, p)| p.cal.portal.is_some()).map(|(i, _)| i).collect();
    let rooms = market_rooms(w, &leaving);
    if let Some(m) = w.portal_market.as_mut() {
        m.open = rooms.open;
        m.backups_best = rooms.backups_best;
        m.depth = rooms.depth;
        m.view = rooms.view;
        m.nil_pool = rooms.nil_pool;
        m.keep_filled();
    }
}

/// The people list is compacted on signing day (`World::compact`): the
/// entrants still shopping and the market's held players follow their new
/// places; the market's rooms are read again after (`refresh_market`).
pub(crate) fn remap_people(w: &mut World, map: &[Option<usize>]) {
    let at = |i: usize| map.get(i).copied().flatten();
    w.portal_pending = w.portal_pending.iter().filter_map(|&i| at(i)).collect();
    if let Some(m) = w.portal_market.as_mut() {
        m.held.retain_mut(|h| at(h.person).map(|i| h.person = i).is_some());
        m.serious = m.serious.iter().filter_map(|&i| at(i)).collect();
    }
}

/// Fall camp (minor bug 337): an entrant still without a school decides
/// now, as he would have on his own day: his old school may take him back,
/// or he leaves football.
pub(crate) fn portal_camp(w: &mut World) {
    if !w.portal_pending.is_empty() {
        portal_day(w, true);
    }
}

/// How a portal entrant weighs playing on two levels down or more: he goes
/// with chance (love of the game / 20) to this power, about 0.3 on average
/// (minor bug 99, proposed, not confirmed).
const STEP_DOWN_POW: f64 = 2.0;

/// The footprint a Division II, NAIA or Division III staff reads an entrant
/// from in the portal (minor bug 246): his home inside about four fifths of
/// its region. Proposed, not confirmed.
const SMALL_PORTAL_REACH: f64 = 0.2;

/// One program that wants an entrant, as he weighs it: (program, his
/// utility there, the money it would pay him, the role he'd have, his play
/// over the level's starters at his spot).
type Suitor = (usize, f64, f64, PlayingTime, f64);

/// What a portal decision reads besides the entrant himself: the day's
/// rosters, your offers and the head starts behind him.
struct Shop<'a> {
    norms: &'a super::systems::SystemNorms,
    pwant: Option<&'a fo::college::PortalWants>,
    bonds: &'a std::collections::HashMap<(u32, usize), f64>,
    counted: &'a [usize],
    adds: &'a [usize],
    season: i32,
}

/// Who wants an entrant (11.16) and what each program means to him: a
/// program where he would make the two-deep next season (proposed, not
/// confirmed); one with an open starting spot reaches out to anyone who
/// beats the backups it has left (fix list 1.10). Nobody else calls. His
/// decision day reads it (`portal_day`), and so does your staff's read of
/// his race while he shops (`portal_race`), by the same code.
#[allow(clippy::too_many_arguments)]
fn suitors(
    w: &World,
    mk: &PortalMarket,
    s: &Shop,
    i: usize,
    c: usize,
    why: Why,
    serious: bool,
    followed: Option<usize>,
) -> Vec<Suitor> {
    let pn = w.programs.len();
    let p0 = &w.people[i];
    let from = w.programs[c].division;
    let pos = p0.player.position;
    let k = pos.index();
    let perf = p0.last_performance.unwrap_or(p0.player.overall());
    let (medians, floors) = (mk.medians, mk.floors);
    let median = |d: Division| medians[d as usize][k];
    let floor = |d: Division| floors[d as usize][k];
    let slots = crate::league::depth_slots(pos).starters;
    let (counted, today_adds, bonds, pwant, norms, season) = (s.counted, s.adds, s.bonds, s.pwant, s.norms, s.season);
    // Sandbox mode (test switches): your offers land past your roster
    // limit, and NIL Go clears your collective's deals in full.
    let lift = w.sandbox(super::sandbox::Switch::RosterLimits);
    let go_free = nil_go_free(w);
    // Your program: every NIL dollar there is yours (Cain 2026-11-08).
    let yours = fo::college::paying(w);
    // A transfer weighs money more than the recruit he was (F3).
    let money = TransferMoney::of(w, mk, i);
    let wr: &World = w;
    (0..pn)
        .into_par_iter()
        .filter_map(|j| {
            let d = wr.programs[j].division;
            let bond = bonds.get(&(wr.people[i].player.id, j)).copied().unwrap_or(0.0);
            // Your offer (lane 10): your program has room for him if its
            // roster does, by the same level rules, and he weighs it by
            // the same wants, with your NIL money and the ties you built.
            if let Some(m) =
                pwant.filter(|m| m.program == j && (!m.staff || m.offers.contains_key(&wr.people[i].player.id)))
            {
                let o = m.offers.get(&wr.people[i].player.id)?;
                let limit = super::ncaa::roster_limits(&wr.programs[j], season).0;
                if j == c || (!lift && counted[j] + today_adds[j] >= limit) {
                    return None;
                }
                // His old head coach's new school takes him at any level (minor bug 86).
                let allowed = (followed == Some(j) && why != Why::Dismissed)
                    || match why {
                        Why::Dismissed => d as usize > from as usize || (d == from && !serious),
                        Why::Cut => d as usize >= from as usize,
                        Why::MoveUp => (d as usize) < from as usize,
                        Why::ProgramDropped => true,
                        _ => d as usize >= from as usize || d == Division::FBS,
                    };
                if !allowed {
                    return None;
                }
                let p = &wr.people[i];
                let (role, nil, perf, edge) = place_at(wr, mk, i, from, j, Some((o.nil, o.coll)));
                let mut u =
                    utility(norms, &wr.high_schools, p, &p.family.own_wants, &wr.programs[j], wr.year, nil, 0.6, 1.0)
                        + mk.view.fit(
                            j,
                            i,
                            p.player.position,
                            &p.family.own_wants,
                            perf,
                            3,
                            0.0,
                            early_weight(p.stars),
                        )
                        + o.bond
                        + bond
                        + money.extra(nil);
                // What you promised him, as far as he believes it, and your
                // staff's call when it works the portal and you didn't call.
                if let Some(kind) = o.promise {
                    u += super::folk::dialogue::promise_pull(wr, i, kind, role, perf, starter_bar(mk, j, k, slots)).0;
                }
                if m.staff && !o.called {
                    u += super::folk::dialogue::staff_call(wr, i, j, role, nil);
                }
                return Some((j, u, nil, role, edge));
            }
            // His play read at that program's level (part D 15): D2 play
            // is discounted against FBS and FCS the way FCS play is
            // against FBS.
            let perf = perf - competition_discount(from) + competition_discount(d);
            let ahead = mk.depth[j][k].iter().take_while(|&&v| v >= perf).count();
            let limit = super::ncaa::roster_limits(&wr.programs[j], season).0;
            // A program with room on its roster also takes an entrant as
            // its third man at the spot (part B 9).
            let room = counted[j] + today_adds[j] + 5 <= limit;
            let deep = if room { 3 } else { 2 };
            let wanted = ahead < deep * slots || (mk.open[j][k] > 0 && perf > mk.backups_best[j][k]);
            if (!wanted && bond == 0.0) || j == c || mk.adds[j] >= 10 || counted[j] + today_adds[j] >= limit {
                return None;
            }
            // A Division II, NAIA or Division III staff works the portal
            // in its own region (minor bug 246): an entrant from inside its
            // footprint, one playing in its state, or one it has ties to.
            // Every program that had a place for him called before, so a
            // transfer down drew offers from the whole country and 19% of
            // Division III's transfers came from 1,000 miles or more.
            // Proposed, not confirmed.
            let small = matches!(d, Division::D2 | Division::NAIA | Division::D3);
            let prog = &wr.programs[j];
            let near =
                prog.footprint[wr.people[i].home_state] >= SMALL_PORTAL_REACH || prog.state == wr.programs[c].state;
            if small && !near && bond == 0.0 {
                return None;
            }
            // A dismissed player starts over a level down or at JUCO; a
            // cut one at his level or below.
            // His old head coach's new school takes him at any level (minor bug 86).
            let allowed = (followed == Some(j) && why != Why::Dismissed)
                || match why {
                    Why::Dismissed => d as usize > from as usize || (d == from && !serious && perf >= median(d)),
                    Why::Cut => d as usize >= from as usize,
                    Why::MoveUp => (d as usize) < from as usize,
                    Why::ProgramDropped => true,
                    _ => d as usize >= from as usize || d == Division::FBS,
                };
            // The bar is the weakest quarter of the level's starters, not
            // the middle (part B 10: real FBS entrants land at FBS about
            // half the time, mostly as depth at a program a notch down;
            // the median bar sent four in five down a level).
            if !allowed || (bond == 0.0 && perf + 2.0 < floor(d) - if d as usize > from as usize { 3.0 } else { 0.0 }) {
                return None;
            }
            let p = &wr.people[i];
            let role = role_at(ahead, slots);
            let (school, coll) =
                portal_nil(&wr.programs[j], mk.pool(j), p, role, perf - median(d), wr.today, go_free == Some(j));
            // Your staff working the portal for you names no money: only
            // you do (Cain 2026-11-08, "I want the only offering control"),
            // so a man it brings in weighs the place alone.
            let nil = if yours == Some(j) {
                0.0
            } else {
                (school + coll) * super::legacy::college::split_q(wr, j, 1) * rich(j)
            };
            // Its staff calls him within hours of his name going in (portal
            // recruiting): his position coach's pitch, as yours makes it.
            let u = utility(norms, &wr.high_schools, p, &p.family.own_wants, &wr.programs[j], wr.year, nil, 0.6, 1.0)
                + mk.view.fit(j, i, p.player.position, &p.family.own_wants, perf, 3, 0.0, early_weight(p.stars))
                + bond
                + money.extra(nil)
                + super::folk::dialogue::staff_call(wr, i, j, role, nil);
            Some((j, u, nil, role, perf - median(d)))
        })
        .collect()
}

/// What a transfer weighs money by over the recruit he was (F3, Cain
/// 2026-11-12: "yes especially when coming from < FBS"). A recruit is
/// choosing where to grow up; a transfer knows what he is and how long he
/// has left. About twice on average for an FBS entrant, by his situation:
/// more with his last season ahead and no pro future, less for a real pro
/// prospect (the path to the league is his money), more for a man coming
/// from below FBS, whose first real money this is. Real: 62% of 377 FBS
/// players told On3 that NIL would factor into a transfer, where recruits
/// rank it sixth of seven reasons (sources in
/// `docs/reference/how-it-works/recruiting.md`). The sizes are proposed,
/// not confirmed, around the agreed average.
const TRANSFER_MONEY: f64 = 1.75;
/// With one season left and no pro future (half of it with two left).
const TRANSFER_LAST: f64 = 0.75;
/// A real pro prospect: his play six points or more over the FBS
/// starters at his spot (none at one point over).
const TRANSFER_PRO: f64 = 0.5;
/// Coming from below FBS.
const TRANSFER_UP: f64 = 0.4;

/// A transfer's money beyond `utility`'s own term for it.
struct TransferMoney {
    /// What he weighs money by over the recruit he was.
    more: f64,
    /// His own money weight ([`money_weights`]).
    weight: f64,
}

impl TransferMoney {
    fn of(w: &World, mk: &PortalMarket, i: usize) -> TransferMoney {
        let p = &w.people[i];
        let crisis = p.bg.story.iter().rev().take(1).any(|s| s.contains("death") || s.contains("lost a job"));
        TransferMoney { more: transfer_money(w, mk, i), weight: money_weights(p, &p.family.own_wants, crisis).0 }
    }

    /// `utility`'s money term again, `more` - 1 times.
    fn extra(&self, nil: f64) -> f64 {
        (self.more - 1.0) * self.weight * 1.5 * (nil.max(0.0) / 1.0e6).powf(0.6)
    }

    /// The money that tops every other program's counts once more with a
    /// transfer too ([`top_money`]), by his heavier weight.
    fn top(&self, ranked: &mut [Suitor]) {
        let mut field = TopTwo::new();
        for r in ranked.iter() {
            field.add(r.0, r.2);
        }
        let weight = top_money_weight(self.weight, self.more);
        for r in ranked.iter_mut() {
            r.1 += top_money(weight, r.2, field.other(r.0));
        }
    }
}

/// [`TRANSFER_MONEY`] for one entrant: his seasons left, his play read
/// against the FBS starters at his spot, the level he comes from.
fn transfer_money(w: &World, mk: &PortalMarket, i: usize) -> f64 {
    if before_f3() {
        return 1.0;
    }
    let p = &w.people[i];
    let from = p.college.map_or(Division::FBS, |c| w.programs[c].division);
    let median = mk.medians[Division::FBS as usize][p.player.position.index()];
    let play = p.last_performance.unwrap_or(p.player.overall()) - competition_discount(from);
    // The season just played counts on January 2 (`settle_eligibility`): a
    // December entrant has it behind him already.
    let used = p.college_seasons + p.cal.count_pending as u32;
    let clock = (super::eligibility::clock_last(p) - mk.season + 1).max(0) as u32;
    let left = super::eligibility::seasons_allowed(mk.season, from).saturating_sub(used).min(clock);
    transfer_more(left, median.is_finite().then_some(play - median), from == Division::FBS)
}

/// [`transfer_money`] from his situation: the seasons he has `left`, his
/// play `over` the FBS starters at his spot (None: no read), and whether
/// he comes from FBS.
pub(crate) fn transfer_more(left: u32, over: Option<f64>, fbs: bool) -> f64 {
    let last = match left {
        0 | 1 => 1.0,
        2 => 0.5,
        _ => 0.0,
    };
    let pro = over.map_or(0.0, |x| clamp((x - 1.0) / 5.0, 0.0, 1.0));
    TRANSFER_MONEY + TRANSFER_LAST * last * (1.0 - pro) - TRANSFER_PRO * pro + if fbs { 0.0 } else { TRANSFER_UP }
}

/// The place an entrant would have at program `j`: his role there, the
/// money it pays him (`yours`: your offer's number, None the usual; no
/// offer of yours, the program's usual), his play read at its level and
/// over its starters at his spot.
fn place_at(
    w: &World,
    mk: &PortalMarket,
    i: usize,
    from: Division,
    j: usize,
    yours: Option<(Option<f64>, Option<f64>)>,
) -> (PlayingTime, f64, f64, f64) {
    let p = &w.people[i];
    let pos = p.player.position;
    let k = pos.index();
    let d = w.programs[j].division;
    let perf = p.last_performance.unwrap_or(p.player.overall()) - competition_discount(from) + competition_discount(d);
    let ahead = mk.depth[j][k].iter().take_while(|&&v| v >= perf).count();
    let role = role_at(ahead, crate::league::depth_slots(pos).starters);
    let q = super::legacy::college::split_q(w, j, 1);
    let edge = perf - mk.medians[d as usize][k];
    let (school, coll) = portal_nil(&w.programs[j], mk.pool(j), p, role, edge, w.today, nil_go_free(w) == Some(j));
    // Your numbers are all his money from you (every NIL dollar is yours,
    // Cain 2026-11-07): in two pots your revenue share and your collective
    // deal as NIL Go clears it at the place he'd have (B18); before, one
    // number. An offer without a number is the place alone.
    let nil = match yours {
        Some((None, None)) => 0.0,
        Some((x, c)) if super::ncaa::two_pots(w.today) => {
            let v = nil_weight(Some(role), pos, recruiting_stars(p), edge, true);
            // Sandbox mode's NIL Go switch: your deal clears in full.
            x.unwrap_or(0.0) + nil_go_at(w, j, c.unwrap_or(0.0), nil_go_top(p, v))
        }
        Some((x, c)) => x.unwrap_or(0.0) + c.unwrap_or(0.0),
        None => (school + coll) * q,
    };
    (role, nil, perf, edge)
}

/// The weakest starter he would have to beat at program `j` (None: a
/// starting place is open).
fn starter_bar(mk: &PortalMarket, j: usize, k: usize, slots: usize) -> Option<f64> {
    mk.depth[j][k].get(slots.max(1) - 1).copied()
}

/// An entrant's place at program `j` while he shops (portal recruiting):
/// his role, the money, his play read at its level and the starter he'd
/// have to beat. None outside a portal window.
pub(crate) fn portal_place(w: &World, i: usize, j: usize) -> Option<(PlayingTime, f64, f64, Option<f64>)> {
    let mk = w.portal_market.as_ref()?;
    let c = w.people[i].college?;
    let id = w.people[i].player.id;
    let yours = (fo::program(w) == Some(j)).then(|| {
        w.front_office
            .college
            .portal
            .iter()
            .find(|t| t.id == id && !t.settled)
            .map_or((None, None), |t| (t.nil, t.nil_coll))
    });
    let (role, nil, perf, _) = place_at(w, mk, i, w.programs[c].division, j, yours);
    let pos = w.people[i].player.position;
    Some((role, nil, perf, starter_bar(mk, j, pos.index(), crate::league::depth_slots(pos).starters)))
}

/// Whether an entrant would play on two levels down or more (minor bug
/// 99): only for the love of the game.
fn plays_far_down(w: &World, label: i32, i: usize) -> bool {
    let seed = w.options.seed as u64 ^ ((label as u64) << 24) ^ 0xFA11;
    let love = (w.people[i].player.personality.love_of_game / 20.0).clamp(0.0, 1.0);
    key_uniform(seed ^ 0x57E9, w.people[i].player.id as u64) < love.powf(STEP_DOWN_POW)
}

/// The days his finalists host him on campus (portal recruiting): up to
/// three, spread over the days of his window that allow a visit, one
/// every other such day at most. Real transfers take two to four visits of
/// 24 to 48 hours in the days after they enter, none in a dead period.
/// Proposed, not confirmed.
fn visit_days(w: &World, e: &super::PortalEntry) -> Vec<Date> {
    let mut open = Vec::new();
    let mut d = e.entered.add(1);
    while d <= e.decide_by {
        if crate::calendar::recruiting_period(&w.dates, d).unofficial_visits() {
            open.push(d);
        }
        d = d.add(1);
    }
    let k = (open.len() / 2).min(3);
    (0..k).map(|n| open[((n + 1) * open.len()) / (k + 1) - 1]).collect()
}

/// His finalists' campus visits up to `upto`: the programs he likes best
/// (besides yours when you work him yourself: yours are the visits you
/// set) host him, each adding the visit's pull, how the place felt to him
/// and the head coach's sit-down at its end.
fn finalist_visits(
    w: &World,
    i: usize,
    e: &super::PortalEntry,
    ranked: &mut [Suitor],
    upto: Date,
    skip: Option<usize>,
) {
    let days = visit_days(w, e);
    let mut order: Vec<usize> = (0..ranked.len()).filter(|&k| Some(ranked[k].0) != skip).collect();
    order.sort_by(|&a, &b| ranked[b].1.total_cmp(&ranked[a].1).then(ranked[a].0.cmp(&ranked[b].0)));
    for (n, &k) in order.iter().take(days.len()).enumerate() {
        if days[n] > upto {
            break;
        }
        let (j, _, nil, role, _) = ranked[k];
        ranked[k].1 += fo::portal::VISIT_PULL
            + fo::portal::feel(w, i, j, e.entered)
            + super::folk::dialogue::staff_campus(w, i, j, role, nil);
    }
}

/// The program you work an entrant for yourself (your own visits, not
/// your staff's), and the one he told yes to before his date.
fn yours_in(pwant: Option<&fo::college::PortalWants>, id: u32) -> (Option<usize>, Option<usize>) {
    let Some(m) = pwant else { return (None, None) };
    let Some(o) = m.offers.get(&id) else { return (None, None) };
    ((!m.staff || o.visited).then_some(m.program), o.committed.then_some(m.program))
}

/// Your staff's read of an entrant's race while he shops (portal
/// recruiting): every program that wants him, with what it means to him
/// today, best first. The same code his decision day runs (`suitors`),
/// with the calls made and the campus visits he has taken so far.
pub(crate) fn portal_race(w: &World, i: usize) -> Vec<(usize, f64)> {
    portal_ranked(w, i, None).into_iter().map(|r| (r.0, r.1)).collect()
}

/// [`portal_race`] with each program's money and the role he'd have. `swap`
/// (trial tooling only, `debug::portal_race_with`): one program's money
/// replaced by another figure, weighed as its own would be (his utility
/// there, its staff's call and, if it is a finalist, the sit-down that ends
/// his visit).
fn portal_ranked(w: &World, i: usize, swap: Option<(usize, f64)>) -> Vec<Suitor> {
    let Some(mk) = w.portal_market.as_ref() else { return Vec::new() };
    let p = &w.people[i];
    let (Some(e), Some(c)) = (p.cal.portal, p.college) else { return Vec::new() };
    if e.on_hold() || e.entered > w.today {
        return Vec::new();
    }
    let id = p.player.id;
    let why = Why::from_reason(e.why);
    let followed = w.coach_follows.iter().find(|f| f.0 == id).map(|f| f.1);
    let pwant = fo::college::portal_wants(w);
    let mut bonds = super::folk::backchannel::bonds(w);
    for &(pid, j, b) in &w.coach_follows {
        let x = bonds.entry((pid, j)).or_insert(0.0);
        *x = x.max(b as f64);
    }
    let counted = counted_rosters(w, mk.season);
    // The places your other yeses hold tonight are taken (they sign first).
    let mut adds = vec![0usize; w.programs.len()];
    if let Some(m) = pwant.as_ref() {
        adds[m.program] = m
            .offers
            .iter()
            .filter(|&(&o, x)| {
                x.committed && o != id && fo::index_of(w, o).is_some_and(|k| w.people[k].cal.portal.is_some())
            })
            .count();
    }
    let shop = Shop {
        norms: &w.system_norms,
        pwant: pwant.as_ref(),
        bonds: &bonds,
        counted: &counted,
        adds: &adds,
        season: mk.season,
    };
    let mut ranked = suitors(w, mk, &shop, i, c, why, mk.serious.contains(&i), followed);
    let money = TransferMoney::of(w, mk, i);
    let from = w.programs[c].division as usize;
    let (skip, said_yes) = yours_in(pwant.as_ref(), id);
    if why != Why::Dismissed && !plays_far_down(w, mk.label, i) {
        ranked.retain(|&(j, ..)| (w.programs[j].division as usize) < from + 2 || Some(j) == said_yes);
    }
    if let Some((j, nil)) = swap
        && let Some(r) = ranked.iter_mut().find(|r| r.0 == j)
    {
        let (was, role) = (r.2, r.3);
        let at = |x: f64| {
            utility(shop.norms, &w.high_schools, p, &p.family.own_wants, &w.programs[j], w.year, x, 0.6, 1.0)
                + money.extra(x)
                + super::folk::dialogue::staff_call(w, i, j, role, x)
        };
        r.1 += at(nil) - at(was);
        r.2 = nil;
    }
    money.top(&mut ranked);
    finalist_visits(w, i, &e, &mut ranked, w.today, skip);
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    ranked
}

/// A transfer's own weights on the six reasons, as [`reason_weights`] gives
/// a recruit's: in the portal he decides for himself, home keeps its whole
/// weight, and money counts by what a transfer weighs it over the recruit
/// he was ([`transfer_money`]; the usual figure outside a window).
pub(crate) fn transfer_weights(w: &World, i: usize) -> [f64; 6] {
    let p = &w.people[i];
    let wants = p.family.own_wants;
    let crisis = p.bg.story.iter().rev().take(1).any(|s| s.contains("death") || s.contains("lost a job"));
    let (money_w, path_w) = money_weights(p, &wants, crisis);
    let more = match w.portal_market.as_ref() {
        Some(mk) => transfer_money(w, mk, i),
        None if before_f3() => 1.0,
        None => TRANSFER_MONEY,
    };
    [
        wants.close_to_home + if crisis { 0.15 } else { 0.0 },
        wants.playing_time + path_w,
        money_w * more,
        wants.development + path_w,
        wants.prestige,
        wants.academics,
    ]
}

/// Trial tooling (`debug`): what a transfer weighs money by over the
/// recruit he was ([`transfer_money`]), None outside a portal window.
#[cfg(feature = "debug")]
pub(crate) fn transfer_money_of(w: &World, i: usize) -> Option<f64> {
    w.portal_market.as_ref().map(|mk| transfer_money(w, mk, i))
}

/// Trial tooling (`debug`): [`portal_ranked`] as (program, what it means to
/// him, its money, the role he'd have there).
#[cfg(feature = "debug")]
pub(crate) fn portal_race_with(w: &World, i: usize, swap: Option<(usize, f64)>) -> Vec<(usize, f64, f64, PlayingTime)> {
    portal_ranked(w, i, swap).into_iter().map(|r| (r.0, r.1, r.2, r.3)).collect()
}

/// Entrants whose day has come decide (T3, T9): each shops among programs
/// that want him (11.16), best first; his first choice can fall through
/// while he waits. No school: his old one may take him back (portal regret),
/// or he leaves football. `force` decides everyone still waiting.
pub(crate) fn portal_day(w: &mut World, force: bool) {
    let norms = w.system_norms;
    let today = w.today;
    let Some(mut market) = w.portal_market.take() else {
        w.portal_pending.clear();
        return;
    };
    release_held(w, &mut market);
    let mut due: Vec<usize> = Vec::new();
    let mut waiting: Vec<usize> = Vec::new();
    for &i in &w.portal_pending {
        match w.people[i].cal.portal {
            Some(e) if force || e.decide_by <= today => due.push(i),
            Some(_) => waiting.push(i),
            None => {}
        }
    }
    if due.is_empty() {
        w.portal_market = Some(market);
        return;
    }
    let perf_of = |p: &Person| p.last_performance.unwrap_or(p.player.overall());
    // Your program wants only the entrants you offered (lane 10).
    let pwant = fo::college::portal_wants(w);
    // Those who told you yes sign first: the places they were promised
    // are theirs before anyone else's decision takes one.
    let yes = |i: usize| yours_in(pwant.as_ref(), w.people[i].player.id).1.is_some();
    due.sort_by(|&a, &b| {
        yes(b).cmp(&yes(a)).then(perf_of(&w.people[b]).total_cmp(&perf_of(&w.people[a]))).then(a.cmp(&b))
    });
    let pn = w.programs.len();
    let season = market.season;
    let one_time = w.changes.rules(season).one_time_transfer;
    // The rosters change only after the day's decisions, so each program's
    // count under its limit is read once. Earlier days' landings are on the
    // rosters already, so only the day's own add to it (the market's `adds`
    // keeps the whole cycle's, for its cap of ten).
    let counted = counted_rosters(w, season);
    let mut today_adds = vec![0usize; pn];
    let seed = w.options.seed as u64 ^ ((market.label as u64) << 24) ^ 0xFA11;
    // The day's landings (person, program), applied to the rosters in
    // place: a full rebuild every portal day costs more than the decisions.
    let mut landings: Vec<(usize, usize)> = Vec::new();
    // Head starts and placements (C15, C16): the buyer or the friend wants
    // him; so does a head coach who took a new job and wants him along
    // (minor bug 86).
    let mut bonds = super::folk::backchannel::bonds(w);
    for &(id, j, b) in &w.coach_follows {
        let e = bonds.entry((id, j)).or_insert(0.0);
        *e = e.max(b as f64);
    }
    for i in due {
        let Some(entry) = w.people[i].cal.portal else { continue };
        let why = Why::from_reason(entry.why);
        // Someone who left college meanwhile is out of the market.
        let Some(c) = w.people[i].college.filter(|_| w.people[i].stage == Stage::College) else {
            w.people[i].cal.portal = None;
            super::folk::backchannel::settled(w, w.people[i].player.id);
            continue;
        };
        let from = w.programs[c].division;
        let pos = w.people[i].player.position;
        let role = w.people[i].last_role;
        let label = market.label;
        let id = w.people[i].player.id;
        let today = w.today;
        let record = |w: &mut World, to: Option<usize>, fell_through: bool, returned: bool| {
            if w.recording {
                w.recruiting_log.portal.push(super::PortalRecord {
                    year: label,
                    role,
                    why: why.reason(),
                    from,
                    // A player who went back to his old school found no new one.
                    to: if returned { None } else { to.map(|j| w.programs[j].division) },
                    id,
                    entered: entry.entered,
                    decided: today,
                    from_program: c,
                    to_program: to,
                    held: entry.held,
                    fell_through,
                    returned,
                });
            }
        };
        let perf = perf_of(&w.people[i]);
        let serious = market.serious.contains(&i);
        w.people[i].cal.portal = None;
        // A placement is read on this, his decision day, then spent (minor
        // bug 325: the Monday cleanup dropped it at entry, before the day).
        super::folk::backchannel::settled(w, id);
        // His old head coach's new school (minor bug 86), asked until he decides.
        let followed = w.coach_follows.iter().find(|f| f.0 == id).map(|f| f.1);
        // His move is made: last season's reasons to leave are spent, so a
        // later window this offseason (spring, or a coaching change where he
        // landed) doesn't draw him again.
        w.people[i].cal.lean = 0.0;
        if serious && !entry.looking && w.people[i].rng.chance(PIPELINE.college_trouble.serious_quit_share) {
            let p = &mut w.people[i];
            p.stage = Stage::Done;
            p.note("After a serious incident no program would take him, and he never played again.");
            record(w, None, false, false);
            continue;
        }
        let k = pos.index();
        let medians = market.medians;
        let median = |d: Division| medians[d as usize][k];
        // Who wants him (11.16), and what each means to him.
        let mut ranked = {
            let shop = Shop {
                norms: &norms,
                pwant: pwant.as_ref(),
                bonds: &bonds,
                counted: &counted,
                adds: &today_adds,
                season,
            };
            suitors(w, &market, &shop, i, c, why, serious, followed)
        };
        // Two levels down or more (an FBS player at Division II or below, an
        // FCS player at the NAIA or below) he plays on only for the love of
        // the game; most finish their degree instead. A dismissed player
        // restarts wherever he can (minor bug 99: about one FBS entrant in
        // five landed below FCS, real about 2% in Hudl's 2025 count).
        // He told you yes before his date (your offer cleared his bar): he
        // signs with you, whatever the rest would have done.
        let (skip, said_yes) = yours_in(pwant.as_ref(), id);
        if why != Why::Dismissed && !plays_far_down(w, label, i) {
            ranked.retain(|&(j, ..)| (w.programs[j].division as usize) < from as usize + 2 || Some(j) == said_yes);
        }
        // The money that tops every other program's counts once more (F3).
        TransferMoney::of(w, &market, i).top(&mut ranked);
        // His finalists hosted him on campus while he shopped.
        finalist_visits(w, i, &entry, &mut ranked, today, skip);
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let promised = said_yes.and_then(|j| ranked.iter().find(|r| r.0 == j).copied());
        // His first choice can fall through while he waits (T9): another
        // entrant takes the spot, or the staff moves on (about 1% a day).
        // A looker's wait counts from his last weekly look, not his entry.
        let since = if entry.looking { entry.decide_by.add(-7) } else { entry.entered };
        let waited = since.days_to(today).max(1) as f64;
        let fell = promised.is_none()
            && ranked.len() >= 2
            && key_uniform(seed, w.people[i].player.id as u64) > 0.99f64.powf(waited);
        let pick = if promised.is_some() {
            promised
        } else if fell {
            ranked.get(1).copied()
        } else {
            ranked.first().copied()
        };
        let age = w.people[i].row_age();
        let from_name = w.program_name(c).to_string();
        let what = why.phrase();
        let Some((j, _, nil, role, edge)) = pick else {
            // Portal regret (T3): his old school may take him back the day
            // he first finds nothing. At fall camp the same word stands: only
            // one turned away then for want of room is asked back (a second
            // draw doubled the returns, 7% of entrants to 16% on seed 74).
            let back = (!entry.looking || force)
                && !matches!(why, Why::Dismissed | Why::Cut)
                && counted[c] < super::ncaa::roster_limits(&w.programs[c], season).0
                && key_uniform(seed ^ 0xBAC, w.people[i].player.id as u64)
                    < if perf >= median(from) - 2.0 { 0.4 } else { 0.15 };
            // No deadline to pick a school (minor bug 337, real since 2025):
            // he stays in the portal and shops again each week as rooms
            // open, until fall camp.
            if !back && !force {
                let p = &mut w.people[i];
                if !entry.looking {
                    p.note("Found no school in his first days in the portal and kept looking.");
                }
                p.cal.portal = Some(super::PortalEntry { decide_by: today.add(7), looking: true, ..entry });
                waiting.push(i);
                continue;
            }
            w.coach_follows.retain(|f| f.0 != id);
            if force {
                w.log.portal_unplaced_at_camp += 1;
            }
            let p = &mut w.people[i];
            if back {
                p.note_again(
                    format!("Entered the portal at {age} {what}, found no new school and returned to {from_name}."),
                    || format!("Entered the portal again at {age} {what}, and again returned to {from_name}."),
                );
                w.log.portal_returned += 1;
            } else {
                p.stage = Stage::Done;
                p.note(format!("Entered the portal at {age} {what}, found no school and left football."));
            }
            record(w, if back { Some(c) } else { None }, false, back);
            continue;
        };
        if fell {
            let first = w.program_name(ranked[0].0).to_string();
            w.people[i].note(format!("His first choice, {first}, fell through while he waited."));
        }
        record(w, Some(j), fell, false);
        w.coach_follows.retain(|f| f.0 != id);
        w.log.portal_landed += 1;
        if today > crate::calendar::YearDates::new(label).signing_day {
            w.log.portal_landed_after += 1;
        }
        market.adds[j] += 1;
        today_adds[j] += 1;
        if market.open[j][k] > 0 && perf > market.backups_best[j][k] {
            market.open[j][k] -= 1;
            if w.people[i].last_role != Some(PlayingTime::Starter) {
                market.filled.resize(pn, [0; POSITION_COUNT]);
                market.filled[j][k] += 1;
            }
        }
        // He is in that room now, for the entrants still shopping (11.2).
        let at = market.depth[j][k].partition_point(|&v| v >= perf);
        market.depth[j][k].insert(at, perf);
        let (overall, seasons) = (w.people[i].player.overall(), w.people[i].college_seasons);
        market.view.join(j, pos, i, overall, seasons);
        if w.recording {
            w.people[i].transfer_from = w.people[i].last_role;
        }
        let to = w.programs[j].division;
        let to_name = w.program_name(j).to_string();
        let step = if (to as usize) < from as usize {
            Step::Up
        } else if to == from {
            Step::Sideways
        } else {
            Step::Down
        };
        let sits = why != Why::ProgramDropped
            && transfer_sits(
                today,
                one_time,
                to,
                from,
                w.people[i].cal.graduated,
                four_year_transfers(w, &w.people[i]),
            );
        let (stars, starter, name) =
            (w.people[i].stars, w.people[i].last_role == Some(PlayingTime::Starter), w.people[i].name.clone());
        landings.push((i, j));
        let p = &mut w.people[i];
        p.pending_dip = Some(World::adjustment(p, step));
        p.college = Some(j);
        p.path.push(j);
        p.transfers += 1;
        p.steered = false;
        p.on_notice = false;
        // He left the school chosen for him, free to choose for himself, and
        // goes on choosing for himself (minor bug 116: a later JUCO move up
        // read as the family's choice again).
        if why == Why::Steered {
            p.family.decider = Decider::Himself;
            p.family.decider_wants = p.family.own_wants;
        }
        if to == Division::JUCO {
            p.juco = true;
        }
        p.note(format!("Entered the portal at {age} {what} and moved from {from_name} to {to_name} ({}).", to.name()));
        if followed == Some(j) {
            p.note_again("He followed his old head coach there.".to_string(), || {
                "Again he followed his old head coach there.".to_string()
            });
        }
        // His new collective's deal starts when he lands, on the place he
        // has there, and its July split keeps that place until he plays
        // (minor bug 100: he held nothing until July).
        if nil > 250_000.0 {
            p.cal.payday = true;
        }
        p.nil_income = nil + p.brand as f64;
        p.cal.nil_role = Some((role, edge as f32));
        if sits {
            p.cal.transfer_sit = true;
            p.note(format!("Under the transfer rules of the day he had to sit out a season at {to_name}."));
            w.log.sit_out_transfers += 1;
        }
        if stars >= 4 || starter {
            let text = format!("{name} transfers from {from_name} to {to_name}.");
            w.calendar.post(Event::new(today, CalLevel::College, Kind::Transfer, Loudness::Small, text).program(j));
        }
    }
    w.portal_pending = waiting;
    if w.portal_pending.is_empty() {
        w.rebuild_rosters();
    } else {
        w.portal_market = Some(market);
        for (i, j) in landings {
            let roster = &mut w.programs[j].roster;
            let at = roster.partition_point(|&x| x < i);
            roster.insert(at, i);
        }
        // Anyone no longer on a college roster (gone, declared, moved) drops
        // off: each program's own list, so on all cores.
        let people = &w.people;
        w.programs.par_iter_mut().enumerate().for_each(|(j, prog)| {
            prog.roster.retain(|&i| people[i].stage == Stage::College && people[i].college == Some(j));
        });
    }
}

/// A player's stars as the NIL market weighs them on a roster (`nil_weight`:
/// payday's split and NIL Go's range).
fn star_nil(stars: u8) -> f64 {
    match stars {
        5 => 2.5,
        4 => 1.0,
        3 => 0.3,
        2 => 0.08,
        _ => 0.03,
    }
}

/// The recruit market's read of his stars (`offer_for`, `usual_nil_offer`).
/// Minor bug 261: it was `star_nil`, and Power 4 four-stars signed for
/// $300-410K a year and three-stars $170-180K, against real 2026 classes of
/// about $85-253K a recruit at the top Power 4 programs (On3 class tables via
/// SI: Oregon $253K, Maryland $190K, Texas $178K, Miami $155K), five-star
/// quarterbacks at $750K-1M and Power 4 quarterback signees at $100-300K.
/// Now set so a top class of three five-stars, fifteen four-stars and seven
/// three-stars averages about $175K: four-stars about $185K, three-stars
/// about $60K (proposed, not confirmed). At 0.45, 0.1 and 0.035 a 30-year
/// world at 0.1 (seed 72) signed Power 4 four-stars at $289-299K,
/// three-stars at $77-79K and two-stars at $26-28K, five-stars at
/// $443-517K and its one five-star quarterback at $798K. Four-stars sit
/// under two caps (the position's going rate and NIL Go's range on the
/// collective's part), so above about 0.2 their money falls more slowly
/// than the factor: played on from saved worlds (seeds 70 and 72, classes of
/// 2026 and 2027) they signed at $227-254K at 0.28, $182-194K at 0.18 and
/// $139-149K at 0.13, with three-stars at $60-66K, two-stars at $20-23K and
/// five-stars at $460-615K. Payday's split keeps `star_nil`.
fn offer_star_nil(stars: u8) -> f64 {
    match stars {
        5 => 2.5,
        4 => 0.18,
        3 => 0.075,
        2 => 0.025,
        _ => 0.01,
    }
}

/// Collectives pay starters (minor bug 100: a flat split paid every P4
/// backup $10-30K, so 20% of Power 4 players earned under $10K against a
/// real 66.5%). A rotation player is worth this share of a starter in his
/// spot, a backup next to nothing; a newcomer with no college season is
/// paid on his stars, as the market read him; play against the level's
/// starters moves it about 3% a point (all proposed, not confirmed; minor
/// bug 335 took the play step from 5%, which alone spread Power 4 starters'
/// pay 2.2x from the 25th to the 90th percentile, against a real 1.7-2.2x
/// for all of it: The NIL Standard's 2026 starters, interior line
/// $424-787K, corners $476-802K, backs $458K-1M).
const NIL_ROTATION: f64 = 0.25;
const NIL_DEPTH: f64 = 0.02;
const NIL_FRESHMAN: f64 = 0.2;
const NIL_PLAY: f64 = 0.03;

/// A rotation player's and a newcomer's weight in the collective's split in
/// two pots (minor bug 354): from July 2025 the school's revenue share pays
/// them their base (`share_weight`, rotation 0.6, newcomer 0.4), so the
/// collective's money goes to the starters it means to keep. Fitted with
/// `nil_fit` on the shipped seed 41 world's July 2026 payday (`nil_check
/// portal`): at 0.15 and 0.12 Power 4 starters' medians ran 15-20% under
/// The NIL Standard's 2026 medians, at 0.10 about 10% (proposed, not
/// confirmed).
const NIL_ROTATION_TWO: f64 = 0.10;
const NIL_FRESHMAN_TWO: f64 = 0.10;

/// How much his recruiting stars still price a player once he has played a
/// college season (minor bug 335): the market reads his college play, so
/// his stars count as `star_nil`^0.2 (a 3-star 0.79 of a 4-star, a 2-star
/// 0.60), where the square root before (0.55 and 0.28) gave Power 4
/// starters a 3.6x spread from their stars alone and paid the top tenth of
/// starting interior linemen $2.4M+ against a real $787K (The NIL
/// Standard's 2026 valuations of Power 4 starters: P25 $424K, median
/// $525K, P90 $787K; the College Sports Commission's 2025-26 pay, Power 4
/// offensive linemen paid at all: P90 $726K). Fitted to those figures.
const NIL_STARS_PLAYED: f64 = 0.2;

/// An interior lineman's market against a tackle's (minor bug 335): Power 4
/// starters' 2026 medians $525K and $907K (The NIL Standard); ESPN's 2025
/// and 2026 surveys put guards and centers at $200-700K and tackles at
/// $600K-1.3M. The going rates (`position_nil`) read one line range.
const NIL_IOL: f64 = 0.58;

/// A defensive tackle's market against an edge rusher's (minor bug 335):
/// tackles' typical 2026 starter $500K-1M (ESPN's survey, The NIL
/// Standard), their average $500-700K (247Sports), against an edge
/// rusher's median $929K (The NIL Standard): 0.65-0.81, the middle 0.73.
const NIL_DT: f64 = 0.73;

/// A player's weight in his collective's payroll (what `nil_payouts` splits
/// a budget by on 1 July): his role there (None: no college season yet),
/// his position's going rate, his stars and how he played against the
/// level's starters at his spot (`edge`, points).
pub(crate) fn nil_weight(role: Option<PlayingTime>, pos: Position, stars: u8, edge: f64, two: bool) -> f64 {
    let line = match pos {
        Position::IOL => NIL_IOL,
        Position::DT => NIL_DT,
        _ => 1.0,
    };
    let market = if two { nil_fit(pos) } else { 1.0 } * line * position_nil(pos) / position_nil(Position::WR);
    let w = match role {
        None => {
            return if two { NIL_FRESHMAN_TWO } else { NIL_FRESHMAN } * star_nil(stars) * market;
        }
        Some(PlayingTime::Starter) => 1.0,
        Some(PlayingTime::Rotation) if two => NIL_ROTATION_TWO,
        Some(PlayingTime::Rotation) => NIL_ROTATION,
        _ => NIL_DEPTH,
    };
    w * star_nil(stars).powf(NIL_STARS_PLAYED) * market * (NIL_PLAY * edge.clamp(-15.0, 15.0)).exp()
}

/// The collective's market by position in two pots (minor bug 354), on top
/// of the going rates (`position_nil`, CBS's 2024 ranges). From July 2025
/// the school's revenue share spreads by the College Sports Commission's
/// medians (bug 336), so the collective's part is what lifts a starter to
/// his whole market, and that gap is widest at quarterback and receiver.
/// Fitted with `NIL_ROTATION_TWO` on the shipped seed 41 world's July 2026
/// payday (`nil_check portal`): Power 4 starters with a college season,
/// brand deals included, then run about 10% under The NIL Standard's 2026
/// medians at every position it gives (QB $1.79M against $2M, WR $651K
/// against $726K, OT $827K against $907K, EDGE $830K against $929K, RB
/// $493K against $527K, CB $523K against $572K, IOL $470K against $525K;
/// before, QB $926K, WR $397K, OT $594K). Tight ends, linebackers and
/// specialists keep 1.0, with no Standard figure in hand; safeties take the
/// corners' (all proposed, not confirmed).
fn nil_fit(pos: Position) -> f64 {
    match pos {
        Position::QB => 3.2,
        Position::WR => 2.9,
        Position::OT => 1.9,
        Position::IOL => 0.92,
        Position::EDGE | Position::DT => 1.67,
        Position::RB => 1.32,
        Position::CB | Position::S => 1.92,
        _ => 1.0,
    }
}

/// What NIL Go's range adds for each follower he has (B18): the same $1.20
/// the public NIL value counts (proposed, not confirmed).
const NIL_GO_FOLLOWER: f64 = 1.2;

/// The top of NIL Go's range for a collective's deal with him (B18). NIL Go
/// clears a deal when its pay sits in the range players like him are paid
/// (Deloitte compares his position, his play, his standing and his
/// following). Here: his position's going rate for a four-star starter at
/// the level's median (`position_nil_max`), moved by his payroll weight
/// against that starter (role, stars, play), plus his following (proposed,
/// not confirmed). The top moves with the collective's market at his
/// position (`nil_fit`, minor bug 354), or NIL Go would hold back what the
/// split now means for a starting quarterback. `weight` is his
/// `nil_weight`.
pub(crate) fn nil_go_top(p: &Person, weight: f64) -> f64 {
    let pos = p.player.position;
    let starter = nil_weight(Some(PlayingTime::Starter), pos, 4, 0.0, true);
    super::ncaa::position_nil_max(pos) * nil_fit(pos) * weight / starter + NIL_GO_FOLLOWER * p.followers as f64
}

/// A recruit's NIL Go top before his first college season.
pub(crate) fn nil_go_top_recruit(p: &Person) -> f64 {
    nil_go_top(p, nil_weight(None, p.player.position, recruiting_stars(p), 0.0, true))
}

/// A collective's deal through NIL Go (B18): one it reviews is cleared up
/// to the top of his range; past it the deal isn't cleared, and the
/// collective resubmits it at the top (proposed, not confirmed: rejected
/// money stays with the collective that year).
pub(crate) fn nil_go(today: Date, deal: f64, top: f64) -> f64 {
    if super::ncaa::nil_go_reviews(today, deal, deal) { deal.min(top.max(600.0)) } else { deal }
}

/// A program's money for one player split into its two pots (B18): the
/// school's revenue share, paid as the school likes, and the collective's
/// deal as NIL Go clears it. The split follows the program's money: its
/// school pot over its whole budget. Before July 2025 it is all the
/// collective's, and nothing reviews it.
/// `free`: sandbox mode's NIL Go switch on your program (`nil_go_free`),
/// where NIL Go clears the collective's part in full.
pub(crate) fn pots(prog: &Program, today: Date, money: f64, top: f64, free: bool) -> (f64, f64) {
    let total = prog.collective.budget;
    let s = if total > 0.0 { (super::ncaa::school_pot(prog, today) / total).clamp(0.0, 1.0) } else { 0.0 };
    let school = money * s;
    (school, if free { money - school } else { nil_go(today, money - school, top) })
}

/// The money a player gets from a program's usual offer once NIL Go has
/// read the collective's part (B18).
pub(crate) fn cleared(prog: &Program, today: Date, money: f64, top: f64, free: bool) -> f64 {
    let (s, c) = pots(prog, today, money, top, free);
    s + c
}

/// Sandbox mode's NIL Go switch (a test switch): your program, whose
/// collective's deals NIL Go clears in full; None otherwise. Every other
/// program's deals go through NIL Go as always.
pub(crate) fn nil_go_free(w: &World) -> Option<usize> {
    if w.sandbox(super::sandbox::Switch::NilGo) { fo::program(w) } else { None }
}

/// NIL Go on one of program `j`'s collective deals (`nil_go`), or the whole
/// deal for your program under sandbox mode's NIL Go switch.
pub(crate) fn nil_go_at(w: &World, j: usize, deal: f64, top: f64) -> f64 {
    if nil_go_free(w) == Some(j) { deal } else { nil_go(w.today, deal, top) }
}

/// Starters' median play by division and position, the bar a player's NIL
/// is read against (NaN where a level has no starter there).
pub(crate) fn nil_medians(w: &World) -> [[f64; POSITION_COUNT]; DIVISION_COUNT] {
    let mut v: [[Vec<f64>; POSITION_COUNT]; DIVISION_COUNT] = Default::default();
    for p in w.people.iter() {
        if p.stage == Stage::College
            && p.last_role == Some(PlayingTime::Starter)
            && let (Some(c), Some(perf)) = (p.college, p.last_performance)
        {
            v[w.programs[c].division as usize][p.player.position.index()].push(perf);
        }
    }
    let mut m = [[f64::NAN; POSITION_COUNT]; DIVISION_COUNT];
    for (d, list) in v.iter_mut().enumerate() {
        for (k, x) in list.iter_mut().enumerate() {
            x.sort_by(|a, b| a.total_cmp(b));
            if !x.is_empty() {
                m[d][k] = x[x.len() / 2];
            }
        }
    }
    m
}

/// A college player's weight in his program's payroll today.
pub(crate) fn nil_value(
    p: &Person,
    div: Division,
    medians: &[[f64; POSITION_COUNT]; DIVISION_COUNT],
    two: bool,
) -> f64 {
    let pos = p.player.position;
    let stars = recruiting_stars(p);
    // An early or summer enrollee's latest row is still his high school's;
    // a mission or service year (unsigned) doesn't count.
    let last = p.player.history.iter().rev().find(|h| h.playing_time != PlayingTime::Unsigned);
    if last.is_none_or(|h| !h.level.is_college()) {
        return nil_weight(None, pos, stars, 0.0, two);
    }
    // A transfer is paid on the place he landed in until he plays there.
    if let Some((role, edge)) = p.cal.nil_role {
        return nil_weight(Some(role), pos, stars, edge as f64, two);
    }
    let m = medians[div as usize][pos.index()];
    let edge = match p.last_performance {
        Some(x) if m.is_finite() => x - m,
        _ => 0.0,
    };
    nil_weight(p.last_role, pos, stars, edge, two)
}

/// A player's weight in his school's revenue share (minor bug 336). Real
/// schools spread the share over most of the roster, more to starters and
/// by position, where the collectives pay their starters; before, the
/// share followed the collective's split (a backup 0.02 of a starter), so
/// on the shipped seed 41 world's July 2026 payday Power 4 offensive
/// linemen paid at all took a median $26K against the College Sports
/// Commission's $155K for 2025-26 ($133K of it revenue share), and every
/// position's paid median ran $9-41K against real revenue-share medians of
/// $93-230K. By position, the College Sports Commission's 2025-26
/// revenue-share medians against a receiver's $112.5K: quarterbacks
/// $230K, defensive linemen $150K, offensive linemen $133K, backs $108.5K,
/// tight ends $93K; linebackers and defensive backs read as receivers and
/// specialists at 0.4 (not in the data). By role: a starter 1, a rotation
/// player and a backup less, a newcomer as a backup (all proposed, not
/// confirmed).
pub(crate) fn share_weight(role: Option<PlayingTime>, pos: Position) -> f64 {
    let at = match pos {
        Position::QB => 2.04,
        Position::EDGE | Position::DT => 1.33,
        Position::OT | Position::IOL => 1.18,
        Position::RB => 0.96,
        Position::TE => 0.83,
        Position::WR | Position::LB | Position::CB | Position::S => 1.0,
        Position::K | Position::P | Position::LS => 0.4,
    };
    let r = match role {
        None => SHARE_DEPTH,
        Some(PlayingTime::Starter) => 1.0,
        Some(PlayingTime::Rotation) => SHARE_ROTATION,
        _ => SHARE_DEPTH,
    };
    at * r
}
/// A rotation player's and a backup's revenue share against a starter's at
/// his position (minor bug 336). Proposed, not confirmed.
const SHARE_ROTATION: f64 = 0.6;
const SHARE_DEPTH: f64 = 0.4;

/// A college player's weight in his school's revenue share today (minor
/// bug 336), read on the same role as his collective weight (`nil_value`).
pub(crate) fn share_value(p: &Person) -> f64 {
    let pos = p.player.position;
    let last = p.player.history.iter().rev().find(|h| h.playing_time != PlayingTime::Unsigned);
    if last.is_none_or(|h| !h.level.is_college()) {
        return share_weight(None, pos);
    }
    if let Some((role, _)) = p.cal.nil_role {
        return share_weight(Some(role), pos);
    }
    share_weight(p.last_role, pos)
}

/// A July split's two parts for one player in two pots (minor bug 336):
/// the school's revenue share by `share_value` and the collective's money
/// by `nil_value`, each over its own roster total. `money` is the
/// program's year of money and `s` the school's share of it. The
/// collective pays its roster `NIL_ROSTER` of its part before two pots and
/// keeps the rest for the year's recruits and transfers, and all of it in
/// two pots (`two`, minor bug 354); the school pays its whole revenue
/// share to the roster, as real schools do (before, it paid 0.6 of it too,
/// and on the shipped seed 41 world's July 2026 payday every Power 4
/// position's median ran about 0.64 of the College Sports Commission's
/// 2025-26 revenue-share medians once the split was spread).
pub(crate) fn july_split(money: f64, s: f64, share: (f64, f64), coll: (f64, f64), two: bool) -> (f64, f64) {
    let roster = money * if two { NIL_ROSTER_TWO } else { NIL_ROSTER };
    (money * s * share.0 / share.1.max(1e-9), roster * (1.0 - s) * coll.0 / coll.1.max(1e-9))
}
/// The share of a collective's money its July split pays the roster before
/// two pots (bug 100's fit to Opendorse's 2021-25 NIL earnings).
pub(crate) const NIL_ROSTER: f64 = 0.6;
/// The same in two pots (minor bug 354): the whole collective part. At 0.6
/// Power 4 starters with a college season ran 20-55% under The NIL
/// Standard's 2026 medians on the shipped seed 41 world (QB $926K against
/// $2M), with the school's share already spread as real; the year's
/// signees and transfers are on the July roster, and the collective's
/// offers to them read its budget apart (proposed, not confirmed).
pub(crate) const NIL_ROSTER_TWO: f64 = 1.0;

/// A program's whole payroll weight (its roster by `nil_value`).
fn nil_pool(
    w: &World,
    j: usize,
    leaving: &std::collections::HashSet<usize>,
    medians: &[[f64; POSITION_COUNT]; DIVISION_COUNT],
) -> f64 {
    let div = w.programs[j].division;
    let two = super::ncaa::two_pots(w.today);
    w.programs[j]
        .roster
        .iter()
        .filter(|i| !leaving.contains(i))
        .map(|&i| nil_value(&w.people[i], div, medians, two))
        .sum()
}

/// What a program's collective offers a transfer (11.16; final build: NIL
/// follows his market). The yearly budget it pays its roster, split as it
/// splits it on 1 July: by the role he would have there, his position and
/// his stars, and moved by how he played against the level's starters at
/// his position (`nil_weight`). A starting quarterback who played well
/// draws seven figures at a big program; a backup lineman a few thousand.
fn portal_nil(
    prog: &Program,
    pool: f64,
    p: &Person,
    role: PlayingTime,
    edge: f64,
    today: Date,
    free: bool,
) -> (f64, f64) {
    let v = nil_weight(Some(role), p.player.position, recruiting_stars(p), edge, super::ncaa::two_pots(today));
    let pool = if pool > 0.0 { pool } else { 12.0 };
    // In two pots from July 2025, the collective's part through NIL Go (B18).
    pots(prog, today, prog.collective.budget * 0.6 * v / (pool + v), nil_go_top(p, v), free)
}

/// The collective's usual portal money for an entrant at program `j`, the
/// read his first answer to your offer measures it against (N1): his role
/// taken as the one he has now, his play read at that level. In its two
/// pots (B18): (the school's revenue share, the collective's cleared deal).
pub(crate) fn portal_usual(w: &World, j: usize, i: usize) -> (f64, f64) {
    let p = &w.people[i];
    if w.programs[j].collective.budget <= 0.0 {
        return (0.0, 0.0);
    }
    let medians = nil_medians(w);
    let pool = nil_pool(w, j, &std::collections::HashSet::new(), &medians);
    let div = w.programs[j].division;
    let from = p.college.map_or(div, |c| w.programs[c].division);
    let m = medians[div as usize][p.player.position.index()];
    let edge = match p.last_performance {
        Some(x) if m.is_finite() => x - competition_discount(from) + competition_discount(div) - m,
        _ => 0.0,
    };
    let role = p.last_role.unwrap_or(PlayingTime::Rotation);
    let q = super::legacy::college::split_q(w, j, 1);
    let (school, coll) = portal_nil(&w.programs[j], pool, p, role, edge, w.today, nil_go_free(w) == Some(j));
    (school * q, coll * q)
}

/// The top of NIL Go's range for an entrant at program `j` (B18), read the
/// way `portal_usual` reads him: his role as the one he has now, his play
/// at that level.
pub(crate) fn portal_top(w: &World, j: usize, i: usize) -> f64 {
    let p = &w.people[i];
    let medians = nil_medians(w);
    let div = w.programs[j].division;
    let from = p.college.map_or(div, |c| w.programs[c].division);
    let m = medians[div as usize][p.player.position.index()];
    let edge = match p.last_performance {
        Some(x) if m.is_finite() => x - competition_discount(from) + competition_discount(div) - m,
        _ => 0.0,
    };
    let role = p.last_role.unwrap_or(PlayingTime::Rotation);
    nil_go_top(p, nil_weight(Some(role), p.player.position, recruiting_stars(p), edge, true))
}

/// July 1, the new money year (T3, 11.5): each collective (and from 2025
/// each school's revenue share) pays its roster; big money can backfire.
pub(crate) fn nil_payouts(w: &mut World) {
    let pn = w.programs.len();
    w.rebuild_rosters();
    // Your deals: every NIL dollar of yours is yours to hand out (lane 10;
    // Cain 2026-11-07), and every arrival has his (`fill_deals`).
    fo::college::fill_deals(w);
    let deals = fo::college::roster_deals(w);
    let today = w.today;
    let mut short: Vec<(u32, f64, f64)> = Vec::new();
    // Your collective's deals NIL Go held back: (players, dollars).
    let mut held = (0usize, 0.0f64);
    let medians = nil_medians(w);
    let go_free = nil_go_free(w);
    for j in 0..pn {
        let year_money = w.programs[j].collective.budget * super::legacy::college::split_q(w, j, 0);
        let roster = w.programs[j].roster.clone();
        if roster.is_empty() {
            continue;
        }
        let div = w.programs[j].division;
        let value = |p: &Person| nil_value(p, div, &medians, super::ncaa::two_pots(today));
        let total: f64 = roster.iter().map(|&i| value(&w.people[i])).sum();
        let shares: f64 = roster.iter().map(|&i| share_value(&w.people[i])).sum();
        // From July 2025 the school's revenue share and the collective pay
        // apart, and NIL Go clears the collective's deals (B18): each split
        // is (school, collective as cleared, what NIL Go held back). The
        // school spreads its share over the roster (minor bug 336).
        let two = super::ncaa::two_pots(today);
        let s = {
            let prog = &w.programs[j];
            let b = prog.collective.budget;
            if b > 0.0 { (super::ncaa::school_pot(prog, today) / b).clamp(0.0, 1.0) } else { 0.0 }
        };
        let split = |p: &Person| {
            let (school, meant) = july_split(year_money, s, (share_value(p), shares), (value(p), total), two);
            let coll = if go_free == Some(j) { meant } else { nil_go(today, meant, nil_go_top(p, value(p))) };
            (school, coll, meant - coll)
        };
        let mut paid: Vec<(f64, usize)> = match deals.as_ref().filter(|d| d.0 == j) {
            Some(d) => {
                // A player paid far under what the collective's usual split
                // gives a man in his role hears it from his family and his
                // agent: his lean toward the portal grows (proposed, not
                // confirmed: under half his share, a share of $25K or more).
                let mut v = Vec::with_capacity(roster.len());
                for &i in &roster {
                    let p = &mut w.people[i];
                    let (school, coll, _) = split(p);
                    // Your going rate's revenue share is 0.6 of the AI's
                    // (`fo::college::splits`): the rest is for your offers.
                    let fair = school * if two { NIL_ROSTER } else { 1.0 } + coll;
                    // Your revenue share and your collective deal, which NIL
                    // Go clears to the top of his range (B18); before two
                    // pots one deal. A player with no deal gets nothing.
                    let id = p.player.id;
                    let written = if two { d.2.get(&id).copied().unwrap_or(0.0) } else { 0.0 };
                    // Sandbox mode's NIL Go switch: your deals clear in full.
                    let paid = if !two {
                        0.0
                    } else if go_free == Some(j) {
                        written
                    } else {
                        nil_go(today, written, nil_go_top(p, value(p)))
                    };
                    if written - paid > 0.5 {
                        held.0 += 1;
                        held.1 += written - paid;
                    }
                    let money = d.1.get(&id).copied().unwrap_or(0.0) + paid;
                    if fair >= 25_000.0 && money < 0.5 * fair && p.college_seasons < 4 {
                        p.cal.lean = clamp(p.cal.lean * 1.35 + 0.01, 0.0, 0.95);
                        p.cal.lean_events.push((today, "his NIL deal fell short of his worth", 1.35));
                        short.push((p.player.id, money, fair));
                    }
                    v.push((money, i));
                }
                v
            }
            None => roster
                .iter()
                .map(|&i| {
                    let (school, coll, _) = split(&w.people[i]);
                    (school + coll, i)
                })
                .collect(),
        };
        paid.sort_by(|a, b| b.0.total_cmp(&a.0));
        let team_median = {
            let mut v: Vec<f64> = roster.iter().filter_map(|&i| w.people[i].last_performance).collect();
            v.sort_by(|a, b| a.total_cmp(b));
            if v.is_empty() { 50.0 } else { v[v.len() / 2] }
        };
        let mut jealousy = 0.0;
        for (k, &(money, i)) in paid.iter().enumerate() {
            let p = &mut w.people[i];
            // A new big deal is a payday (fix list 4.5). Proposed, not
            // confirmed: $250,000, where the top earners' backlash starts.
            if money > 250_000.0 && p.nil_income <= 250_000.0 {
                p.cal.payday = true;
            }
            // Collectives pay most of it (about 82%); brand deals, which
            // follow fame and Aura (F15), come on top.
            let money = money + p.brand as f64;
            p.nil_income = money;
            // The top earners on a team: payouts can bring jealousy, complacency or pressure.
            if k < 3 && money > 250_000.0 {
                let perf = p.last_performance.unwrap_or(team_median);
                if perf < team_median {
                    p.bg.stability = clamp(p.bg.stability - 1.0, 1.0, 20.0);
                    jealousy += 0.2;
                    let age = p.row_age();
                    p.note(format!(
                        "At {age}, paid among the most on his team, he struggled and took heavy criticism."
                    ));
                } else if p.rng.chance(0.08) {
                    p.player.personality.work_ethic = clamp(p.player.personality.work_ethic - 1.0, 1.0, 20.0);
                    let age = p.row_age();
                    p.note(format!("At {age} a big NIL payday made him a little complacent."));
                }
            }
        }
        if jealousy > 0.0
            && let Some(hc) = w.programs[j].head_coach
        {
            w.coaches[hc].reputation = clamp(w.coaches[hc].reputation - 0.3 * jealousy, 1.0, 20.0);
        }
    }
    if deals.is_some() {
        fo::college::after_payouts(w, &short, held);
    }
}

// --- Reads for your desk (lane 10) ---------------------------------------------------------

/// The collective's usual NIL offer to a recruit from program `j`: the
/// money the AI's signing day starts from (`offer_for` at the middle of a
/// board, without a rising program's reach or a booster's cash).
pub(crate) fn usual_nil_offer(w: &World, j: usize, i: usize) -> f64 {
    let (school, collective) = usual_nil_parts(w, j, i);
    (school + collective) * rich(j)
}

/// The collective's usual NIL offer to a recruit from program `j` in its two
/// pots (B18): (the school's revenue share, the collective's deal as NIL Go
/// clears it). Before July 2025 it is all the collective's.
pub(crate) fn usual_nil_parts(w: &World, j: usize, i: usize) -> (f64, f64) {
    let prog = &w.programs[j];
    let p = &w.people[i];
    if prog.collective.budget <= 0.0 {
        return (0.0, 0.0);
    }
    let talent = offer_star_nil(recruiting_stars(p));
    let q = super::legacy::college::split_q(w, j, 2);
    let pay = (prog.collective.budget / 10.0e6).powf(0.8) * q;
    let money = (position_nil(p.player.position) * talent * pay)
        .min(prog.collective.budget * 0.6 * 0.25 * q)
        .min(super::ncaa::position_nil_max(p.player.position));
    pots(prog, w.today, money, nil_go_top_recruit(p), nil_go_free(w) == Some(j))
}

/// The NIL money a recruit weighs from program `j` today: from your own
/// program what you promise (with the board yours, nothing but the
/// scholarship until you name a number; `money_from`), from any other its
/// usual offer.
pub(crate) fn offer_money(w: &World, j: usize, i: usize) -> f64 {
    fo::college::money_from(w, j, i).unwrap_or_else(|| usual_nil_offer(w, j, i))
}

/// The day his class's first commitment wave signs (its junior spring,
/// the first of the summer waves): offers before it are early ones.
pub(crate) fn first_wave(class_of: i32) -> Date {
    let (m, d) = super::clock::COMMIT_WAVES[0];
    Date::ymd(class_of - 1, m, d)
}

/// What a school means to a recruit today (lane 10): the same appeal his
/// signing day weighs (his decider's wants, the school, its money, his own
/// feel for it), with what each staff has done for him so far: the visits
/// he took, a commitment's hold. `nil` is the money on the table (None: the
/// money the school offers him today, `offer_money`). It leaves out the
/// position room and the relationship a staff builds over the year, which
/// signing day adds. Money that tops every other offer on his list counts
/// once more ([`top_money`], F3).
pub(crate) fn appeal(w: &World, i: usize, j: usize, nil: Option<f64>) -> f64 {
    let nil = nil.unwrap_or_else(|| offer_money(w, j, i));
    appeal_in(w, i, j, nil, &money_field(w, i, None))
}

/// [`appeal`] with the money on his list read already ([`money_field`]).
fn appeal_in(w: &World, i: usize, j: usize, nil: f64, field: &TopTwo) -> f64 {
    let p = &w.people[i];
    let prog = &w.programs[j];
    let wants = p.family.decider_wants;
    let mut u = utility(
        &w.system_norms,
        &w.high_schools,
        p,
        &wants,
        prog,
        w.year,
        nil,
        0.6,
        home_weight(&w.programs, p, w.year),
    );
    let seed = w.options.seed as u64 ^ (p.class_of as u64) << 20;
    u += FEEL_SD * key_normal(seed ^ 77 ^ j as u64, p.player.id as u64);
    let contact = p.cal.visits.iter().filter(|v| v.0 == j).count().min(5) as f64;
    let official = p.cal.visits.iter().any(|v| v.0 == j && v.2 == super::Visit::Official);
    u += 0.004 * contact + if official { 0.02 } else { 0.0 };
    // What the head coach said in the living room (lane 14, D7).
    u += super::folk::dialogue::home_talk(p, j);
    if let Some(k) = p.cal.early_offers.iter().position(|o| o.0 == j) {
        u += if k == 0 { 0.02 } else { 0.01 };
    }
    if p.cal.commit.is_some_and(|c| c.0 == j) {
        u += stickiness(p, prog, w.year);
    }
    if p.cal.signed.is_some_and(|s| s.program == j) {
        u += 10.0;
    }
    u + super::legacy::guys::recruit_lean(w, p.player.id, p.hs, j)
        + super::legacy::college::u_lean(w, j, p.player.position, p.stars)
        + top_money(recruit_top_weight(p, w.year), nil, field.other(j))
}

/// What a school means to a recruit, reason by reason (board 8): six of
/// [`utility_fit`]'s terms as he weighs them, each in appeal points with the
/// school's score on it (0 to 1): close to home, playing time, NIL money,
/// coaching and development, winning and the name, academics. The seventh
/// is the rest of [`appeal`]: his ties and feel (visits, an early offer, a
/// commitment, the team he grew up with, a brother there, his school's
/// pipeline, the program's stories, his pull to stay home, his own feel).
/// NIL money carries what a clear top offer counts once more
/// ([`top_money`]). The seven add up to `appeal` exactly.
pub(crate) fn reason_parts(w: &World, i: usize, j: usize, nil: Option<f64>) -> [(f64, f64); 7] {
    let p = &w.people[i];
    let prog = &w.programs[j];
    let nil = nil.unwrap_or_else(|| offer_money(w, j, i));
    let wants = p.family.decider_wants;
    let g = |c: GradeCat| prog.grades[c as usize] / 100.0;
    let close = closeness(home_miles(&w.high_schools, p, prog));
    let crisis = p.bg.story.iter().rev().take(1).any(|s| s.contains("death") || s.contains("lost a job"));
    let close_w = wants.close_to_home + if crisis { 0.15 } else { 0.0 };
    let prestige = (g(GradeCat::TitleChances)
        + g(GradeCat::StadiumAtmosphere)
        + g(GradeCat::Tradition)
        + g(GradeCat::BrandExposure))
        / 4.0;
    let money = (nil.max(0.0) / 1.0e6).powf(0.6);
    let (money_w, path_w) = money_weights(p, &wants, crisis);
    // `appeal` reads him at 0.6 of the way down a board.
    let playing = g(GradeCat::PlayingTime) * (0.6 + 0.4 * 0.6);
    let academics = g(GradeCat::Academics) * (0.5 + 0.5 * clamp((p.bg.gpa - 2.5) / 1.5, 0.0, 1.0));
    let development = (g(GradeCat::ProPipeline) + g(GradeCat::CoachPrestige)) / 2.0;
    let brand = prog.prestige_rel.min(NAME_TIE);
    let lv = recruit_level(prog, w.year);
    let home = home_weight(&w.programs, p, w.year);
    let mut name = 1.6 * lv * (0.5 + wants.prestige + wants.development)
        + (0.25 + wants.prestige) * brand * (0.4 + 0.6 * lv)
        + 0.1 * g(GradeCat::Facilities)
        + wants.prestige * prestige;
    if p.stars == 5 {
        name += FIVE_STAR_PULL * brand.powi(3) * lv;
    }
    // The money that tops his list counts once more (F3), under money.
    let lead = top_money(recruit_top_weight(p, w.year), nil, money_field(w, i, None).other(j));
    let six = [
        (home * close_w * close, close),
        ((wants.playing_time + path_w) * playing, playing),
        (money_w * money * 1.5 + lead, money.min(1.0)),
        ((wants.development + path_w) * development, development),
        (name, (prestige + brand) / 2.0),
        (wants.academics * academics, g(GradeCat::Academics)),
    ];
    let rest = appeal(w, i, j, Some(nil)) - six.iter().map(|x| x.0).sum::<f64>();
    [six[0], six[1], six[2], six[3], six[4], six[5], (rest, clamp(0.5 + rest / 0.4, 0.0, 1.0))]
}

/// How much each of the six reasons counts with whoever decides for him,
/// as [`reason_parts`] weighs them (close to home, playing time, money,
/// development, winning and the name, academics).
pub(crate) fn reason_weights(w: &World, i: usize) -> [f64; 6] {
    let p = &w.people[i];
    let wants = p.family.decider_wants;
    let crisis = p.bg.story.iter().rev().take(1).any(|s| s.contains("death") || s.contains("lost a job"));
    let (money_w, path_w) = money_weights(p, &wants, crisis);
    [
        home_weight(&w.programs, p, w.year) * (wants.close_to_home + if crisis { 0.15 } else { 0.0 }),
        wants.playing_time + path_w,
        money_w,
        wants.development + path_w,
        wants.prestige,
        wants.academics,
    ]
}

/// Money's weight for him (his family's need, against his drive) and the
/// path to the pros the ambitious kid weighs instead, as `utility_fit`.
fn money_weights(p: &Person, wants: &super::background::Wants, crisis: bool) -> (f64, f64) {
    let need = clamp(0.25 + 0.75 * (1.0 - p.bg.money) + if crisis { 0.2 } else { 0.0 }, 0.1, 1.2);
    let drive = clamp((p.family.ambition - 10.0) / 10.0, -1.0, 1.0);
    (wants.money * need * (1.0 - 0.35 * drive), 0.35 * drive.max(0.0) * wants.money)
}

// --- The clear top offer (playtest item F3, Cain 2026-11-12) --------------------------------

/// What the money that tops every other offer counts once more, per point
/// of his money weight and of ln(his money over the next offer)
/// ([`top_money`]). The plain money term reads the dollars (`utility_fit`),
/// so double the field's money moved a school 0.023 appeal points on the
/// shipped seed 41 world, a third of his plain feel for a place, and an SEC
/// program at double the portal's best money was outside a transfer's top
/// ten 29% of the time. Real recruits rarely put money first and small
/// gaps lose to fit, but 1.5 to 2 times the next offer usually wins
/// (Rashada flipped at 1.46 times, Underwood at about 1.75 to 2; sources in
/// `docs/reference/how-it-works/recruiting.md`). Cain agreed 2026-11-12.
const TOP_MONEY: f64 = 0.6;

/// The ratio counts up to three times the next offer: nobody buys a class
/// at ten times the field (Cain agreed 2026-11-12).
const TOP_MONEY_CAP: f64 = 3.0;

/// The next offer is read as at least this much: a multiple of pocket
/// money is no multiple, and a lone offer needs real money behind it to
/// stand out (proposed, not confirmed).
const TOP_MONEY_FLOOR: f64 = 10_000.0;

/// The hungrier, the steeper (Cain 2026-11-12: "a money hungry player
/// should be closer to 75%" at double the money): the lift grows with his
/// money weight over the usual recruit's (`HUNGRY_AT`) to this power, so
/// the recruits who put money first come near three in four and the rest
/// stay where they were agreed.
const HUNGRY_POW: f64 = 0.5;
const HUNGRY_AT: f64 = 0.13;

/// Trial tooling (`debug`): the game as it was before the clear top offer
/// and the transfer's heavier money, so one build measures both on the
/// same players (`nil_pull`, `AFM_NIL_PULL=off`). The game has no switch.
#[cfg(feature = "debug")]
pub(crate) static BEFORE_F3: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn before_f3() -> bool {
    #[cfg(feature = "debug")]
    {
        BEFORE_F3.load(std::sync::atomic::Ordering::Relaxed)
    }
    #[cfg(not(feature = "debug"))]
    {
        false
    }
}

/// Trial tooling only: the programs whose money is multiplied, and by what
/// (a school that offers double, measured through signing day and the
/// portal by `nil_pull`). The game never builds it.
#[cfg(feature = "debug")]
pub(crate) static RICH: std::sync::RwLock<(Vec<usize>, f64)> = std::sync::RwLock::new((Vec::new(), 1.0));
/// Whether [`RICH`] names anyone: the only read a trial without it pays.
#[cfg(feature = "debug")]
pub(crate) static RICH_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// What program `j`'s money is multiplied by in a trial (1 in the game).
fn rich(j: usize) -> f64 {
    #[cfg(feature = "debug")]
    {
        if !RICH_ON.load(std::sync::atomic::Ordering::Relaxed) {
            return 1.0;
        }
        let r = RICH.read().expect("the trial's list");
        if r.0.contains(&j) { r.1 } else { 1.0 }
    }
    #[cfg(not(feature = "debug"))]
    {
        let _ = j;
        1.0
    }
}

/// What a clear top offer is worth to him per point of ln(ratio), from his
/// own money weight ([`money_weights`]: his wants, his family's need, his
/// ambition); `more` is what a transfer weighs money by over the recruit
/// he was ([`transfer_money`]; 1 for a recruit).
pub(crate) fn top_money_weight(money_w: f64, more: f64) -> f64 {
    TOP_MONEY * money_w * more * (money_w / HUNGRY_AT).max(0.0).powf(HUNGRY_POW)
}

/// The lift of money that tops every other offer: `weight`
/// ([`top_money_weight`]) × ln(his money over the best other money), the
/// ratio read up to [`TOP_MONEY_CAP`]. Nothing for money that tops no one.
pub(crate) fn top_money(weight: f64, mine: f64, other: f64) -> f64 {
    let other = other.max(TOP_MONEY_FLOOR);
    if mine <= other || before_f3() { 0.0 } else { weight * (mine / other).min(TOP_MONEY_CAP).ln() }
}

/// The two best moneys on a player's list, each program counted once.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TopTwo {
    best: f64,
    at: usize,
    second: f64,
}

impl TopTwo {
    fn new() -> TopTwo {
        TopTwo { best: 0.0, at: usize::MAX, second: 0.0 }
    }

    fn add(&mut self, at: usize, money: f64) {
        if at == self.at {
            self.best = self.best.max(money);
        } else if money > self.best {
            (self.second, self.best, self.at) = (self.best, money, at);
        } else if money > self.second {
            self.second = money;
        }
    }

    /// The best money on the list besides program `at`'s own.
    fn other(&self, at: usize) -> f64 {
        if at == self.at { self.second } else { self.best }
    }
}

/// The money on a recruit's list today: every school with an offer out to
/// him (his offers, the early ones, his commitment, yours once you
/// offered), each at the money he would weigh from it ([`offer_money`]).
/// `with` sets one school's money, on his list or coming onto it.
pub(crate) fn money_field(w: &World, i: usize, with: Option<(usize, f64)>) -> TopTwo {
    let p = &w.people[i];
    let mine = fo::program(w).filter(|_| fo::college::offered_here(w, p.player.id));
    let mut field = TopTwo::new();
    let schools = p.cal.offers.iter().copied().chain(p.cal.early_offers.iter().map(|o| o.0));
    for j in schools.chain(p.cal.commit.map(|c| c.0)).chain(mine).chain(with.map(|x| x.0)) {
        match with {
            Some((x, money)) if x == j => field.add(j, money),
            _ => field.add(j, offer_money(w, j, i)),
        }
    }
    field
}

/// What a recruit's clear top offer is worth to him ([`top_money_weight`]),
/// in the NIL era (`ncaa::nil_era`). A rogue booster's cash before it is
/// no public offer anyone can top or be topped by, and keeps the weight it
/// had (proposed, not confirmed).
fn recruit_top_weight(p: &Person, year: i32) -> f64 {
    if !super::ncaa::nil_era(year) {
        return 0.0;
    }
    let crisis = p.bg.story.iter().rev().take(1).any(|s| s.contains("death") || s.contains("lost a job"));
    top_money_weight(money_weights(p, &p.family.decider_wants, crisis).0, 1.0)
}

/// [`appeal`] for several schools at once, his list's money read once:
/// `with` sets one school's money, and every other school is weighed
/// against it too (the top offer it may no longer be).
pub(crate) fn appeals(w: &World, i: usize, schools: &[usize], with: Option<(usize, f64)>) -> Vec<f64> {
    let field = money_field(w, i, with);
    schools
        .iter()
        .map(|&j| {
            let nil = match with {
                Some((x, money)) if x == j => money,
                _ => offer_money(w, j, i),
            };
            appeal_in(w, i, j, nil, &field)
        })
        .collect()
}

/// How hard a recruit holds to the school he committed to, in appeal points.
pub(crate) fn commitment_hold(w: &World, i: usize) -> f64 {
    let p = &w.people[i];
    p.cal.commit.map_or(0.0, |c| stickiness(p, &w.programs[c.0], w.year))
}

/// The class a program signs next: the seniors until February's signing
/// day, which can fall after the Super Bowl in the next football year.
pub(crate) fn next_class(w: &World) -> i32 {
    match &w.prev_dates {
        Some(p) if w.today <= p.signing_day => w.year,
        _ => w.year + 1,
    }
}

/// Your staff's class plan (lane 10): see `front_office::college::ClassOutlook`.
use super::front_office::college::ClassOutlook;

pub(crate) fn class_outlook(w: &World, j: usize) -> ClassOutlook {
    let sign_year = next_class(w);
    let stage = match (w.dates.early_signing, &w.prev_dates) {
        _ if sign_year == w.year => SignStage::Final,
        (Some(e), _) if w.today > e => SignStage::Final,
        (Some(_), _) if w.today.month() == 12 => SignStage::Early,
        _ if w.today.month() >= 12 || w.today.month() <= 2 => SignStage::Final,
        _ => SignStage::Summer(0),
    };
    let read = RosterRead { stage };
    let prog = &w.programs[j];
    let signed: Vec<usize> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.stage != Stage::College && p.cal.signed.is_some_and(|s| s.program == j))
        .map(|(i, _)| i)
        .collect();
    let plan = class_plan(prog, &w.people, &signed, sign_year, w.options.seed as u64, w.draft_bars, read);
    let (limit, signees) = super::ncaa::roster_limits(prog, sign_year);
    ClassOutlook {
        sign_year,
        need: plan.scholarship,
        slots: plan.slots,
        room: plan.room,
        limit,
        signees,
        whole_roster: plan.whole_roster,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minor bug 372: a playoff team's player of yours who means to enter the
    /// portal has no entry once his team is out, as one on a team outside the
    /// playoff has none before his talk. His held entry made `intents_day`
    /// skip him, so after his talk he neither entered nor stayed and read "In
    /// the portal" until fall camp.
    #[test]
    fn a_playoff_teams_entrant_of_yours_decides_after_his_talk() {
        let mut w = crate::tests::legacy::seat("college");
        let mine = w.seat.and_then(|s| s.program).unwrap();
        w.add_class_aged(19);
        let year = w.year;
        let room: Vec<usize> =
            (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(2).collect();
        for &i in &room {
            let q = &mut w.people[i];
            q.stage = Stage::College;
            q.college = Some(mine);
            q.class_of = year - 1;
            q.college_seasons = 1;
            q.last_role = Some(crate::player::PlayingTime::Backup);
            q.cal.lean = 0.9;
            w.programs[mine].roster.push(i);
        }
        // The window opens (which makes its market): both ask to see you.
        let close = w.today.add(15);
        portal_open(&mut w, super::super::clock::PortalWindow::Winter, close);
        assert_eq!(w.portal_intents.len(), 2, "both ask to see you");
        // The second one's team is in the playoff: he is on hold.
        let (b, id) = (room[1], w.people[room[1]].player.id);
        w.portal_intents.retain(|t| !t.is(id));
        w.people[b].cal.portal = Some(crate::world::PortalEntry {
            entered: HELD,
            decide_by: HELD,
            why: Why::PlayingTime.reason(),
            held: true,
            looking: false,
        });
        assert!(w.people[b].cal.portal.is_some_and(|e| e.on_hold()));
        let today = w.today;
        w.portal_market.as_mut().unwrap().held.push(Held {
            person: b,
            program: mine,
            window_day: today,
            extra: 0,
            decide_in: 4,
            why: Why::PlayingTime,
            bar: Some(0.3),
        });
        // His team is out (the test world has no playoff): he is released and
        // asks to see you, with no entry until then.
        portal_day(&mut w, false);
        assert!(w.people[b].cal.portal.is_none(), "he has no entry before his talk");
        assert!(w.portal_intents.iter().any(|t| t.is(id)), "he asked to see you");
        // His talk didn't change his mind: on his day he is in the portal
        // with his own day, no longer on hold.
        w.today = w.today.add(30);
        intents_day(&mut w);
        assert!(w.portal_intents.is_empty(), "both decided");
        let e = w.people[b].cal.portal.expect("he entered after his talk");
        assert!(!e.on_hold() && e.entered <= w.today, "he is in the portal on his day, not on hold");
    }

    /// Minor bug 255: a recruit reads a school's distance from his own town
    /// to its campus. A Jacksonville kid has Georgia Southern, about 150
    /// miles off, nearer than Miami, about 330, where reading every school
    /// in his own state as 60 miles away had Miami at 0.96 and Georgia
    /// Southern at 0.52.
    #[test]
    fn closeness_reads_his_town() {
        let w = World::empty(super::super::WorldOptions { scale: 0.001, history_years: 0, ..Default::default() });
        let prog = |n: &str| w.programs.iter().find(|p| p.real_name == n).expect(n);
        let fl = geo::state_index("FL").unwrap();
        let jax = geo::place(fl, "Jacksonville");
        assert!(jax != (geo::STATE_CENTERS[fl].1, geo::STATE_CENTERS[fl].2), "the towns table has Jacksonville");
        let (gs, miami, uf) = (
            campus_miles(jax, prog("Georgia Southern")),
            campus_miles(jax, prog("Miami (FL)")),
            campus_miles(jax, prog("Florida")),
        );
        assert!((90.0..160.0).contains(&gs), "Georgia Southern {gs:.0} miles");
        assert!((280.0..380.0).contains(&miami), "Miami {miami:.0} miles");
        assert!(uf < gs, "Florida {uf:.0} miles");
        assert!(closeness(gs) > closeness(miami) + 0.3);
        // A school or town the tables lack reads his state's center.
        let center = home_place(&[], 0, fl);
        assert_eq!(center, (geo::STATE_CENTERS[fl].1, geo::STATE_CENTERS[fl].2));
    }

    /// Minor bug 256: the academies and the strict academic schools read the
    /// whole country for students they can admit, by their bar; a school
    /// without one reads only its region, its brand and the overflow states.
    #[test]
    fn academic_names_and_academies_read_the_country() {
        use super::super::program::{ACADEMIC_NATIONAL, academic_reach, rebuild_footprint};
        let mut w = World::empty(super::super::WorldOptions { scale: 0.001, history_years: 0, ..Default::default() });
        let at = |w: &World, n: &str| w.programs.iter().position(|p| p.real_name == n).expect(n);
        for (n, want) in
            [("Army", 1.0), ("Navy", 1.0), ("Air Force", 1.0), ("Stanford", 1.0), ("Duke", 0.75), ("Iowa", 0.0)]
        {
            let r = academic_reach(&w.programs[at(&w, n)]);
            assert!((r - want).abs() < 1e-9, "{n}: {r}");
        }
        let hi = geo::state_index("HI").unwrap();
        let (army, iowa) = (at(&w, "Army"), at(&w, "Iowa"));
        let flat = [0.0; 51];
        for j in [army, iowa] {
            let year = w.year;
            rebuild_footprint(&mut w.programs[j], &flat, year);
        }
        assert!(w.programs[army].footprint[hi] >= ACADEMIC_NATIONAL - 1e-9, "Army in Hawaii");
        assert!(w.programs[iowa].footprint[hi] < ACADEMIC_NATIONAL, "Iowa in Hawaii");
    }

    /// T1.5: each pick lands in its own round's night, counted on the
    /// board's real rounds: round 1 Thursday from 8 PM to before midnight,
    /// pick 33 opening Friday at 7 PM and round 3 done before midnight,
    /// Saturday from noon to the early evening.
    #[test]
    fn draft_picks_keep_to_their_nights() {
        let base: Vec<usize> = (0..32).collect();
        for year in [2005, 2015, 2025, 2026] {
            let board = super::super::franchise::draft_board(year, &base, 1.0, 7, &[]);
            let y = crate::calendar::YearDates::new(year);
            let t = pick_stamps(year, &board, 1.0, y.draft);
            for (k, &(_, r)) in board.iter().enumerate() {
                let day = (r > 1) as usize + (r > 3) as usize;
                assert!(t[k] >= y.draft[day], "{year} pick {} in round {r} at {}", k + 1, t[k]);
                assert_eq!(t[k].date, y.draft[day].date, "{year} pick {} in round {r} at {}", k + 1, t[k]);
                if k > 0 {
                    assert!(t[k] >= t[k - 1] || r != board[k - 1].1, "{year} pick {} runs backwards", k + 1);
                }
            }
            if year >= 2010 {
                assert_eq!(t[32], y.draft[1], "{year} pick 33");
                let end1 = t[31].minute;
                assert!((22 * 60 + 15..23 * 60 + 45).contains(&end1), "{year} round 1 ends {}", t[31]);
                let last = t[board.len() - 1].minute;
                assert!((17 * 60..20 * 60).contains(&last), "{year} the last pick at {}", t[board.len() - 1]);
            }
        }
    }

    /// The services' quotas count only the regular positions; a kicker,
    /// punter or long snapper ranked among them gets 3 stars at most.
    #[test]
    fn services_cap_specialists_at_three_stars() {
        let specialist = [false, true, false, true, false, false, false];
        let quotas = [(5, 1), (4, 2), (3, 1)];
        assert_eq!(service_stars(&specialist, &quotas), vec![5, 3, 4, 3, 4, 3, 0]);
        assert_eq!(service_stars(&[true, false], &[(5, 1)]), vec![3, 5]);
    }

    /// F17: a recruit whose commitment fell through on signing day can walk
    /// on wherever his position has room, even when the plan's walk-on
    /// places and the class's slots are used. Under a whole-roster limit
    /// (the House era) he needs a place under it too.
    #[test]
    fn late_walk_on_needs_room_at_his_position() {
        let (wr, k) = (Position::WR.index(), Position::K.index());
        let mut plan = ClassPlan { walk_on_room: 1, room: 20, ..Default::default() };
        plan.walk_on[k] = 1;
        plan.open_at[wr] = 2;
        // The class filled every place under the limit and the specialist walk-on place.
        let full = [19, 1];
        assert!(plan.late_walk_on(wr, full, [1, 0]), "room at his position");
        assert!(!plan.late_walk_on(wr, full, [2, 0]), "his position is full");
        plan.whole_roster = true;
        assert!(!plan.late_walk_on(wr, full, [1, 0]), "no place under a whole-roster limit");
        assert!(plan.late_walk_on(wr, [17, 1], [1, 0]), "a place left under it");
        // The plan's own walk-on places still count as before.
        assert!(plan.late_walk_on(k, [19, 0], [0, 0]));
        assert!(!plan.late_walk_on(k, full, [0, 1]));
    }

    /// Official visits (T5): the class signing next, from September 1 of the
    /// senior year; from 2018 also April 1 to June 30 of the junior year.
    #[test]
    fn official_visits_follow_the_ncaa_calendar() {
        let ok = official_visit_allowed;
        // Before 2018: seniors only.
        assert!(!ok(2016, Date::ymd(2016, 5, 13), 2017, false), "junior spring");
        assert!(!ok(2016, Date::ymd(2016, 8, 19), 2017, false), "summer before the senior year");
        assert!(ok(2016, Date::ymd(2016, 9, 2), 2017, false), "senior fall");
        assert!(ok(2016, Date::ymd(2017, 1, 13), 2017, false), "senior January");
        assert!(!ok(2016, Date::ymd(2016, 10, 7), 2018, false), "a junior in the fall");
        // From 2018: the junior spring too.
        assert!(ok(2018, Date::ymd(2018, 5, 11), 2019, true), "junior spring");
        assert!(!ok(2018, Date::ymd(2018, 3, 23), 2019, true), "before April 1");
        assert!(!ok(2018, Date::ymd(2018, 8, 17), 2019, true), "summer before the senior year");
        assert!(ok(2018, Date::ymd(2018, 10, 5), 2019, true), "senior fall");
        assert!(!ok(2018, Date::ymd(2018, 10, 5), 2020, true), "a junior in the fall");
    }
}
