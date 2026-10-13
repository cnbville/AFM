//! Moving players through the world: high school transfers and powerhouse
//! pulls (topic 2), exposure and recruiting (topics 4 and 5), JUCO and prep
//! school (topic 6, 1.6), the draft, the transfer portal, tampering and NIL
//! (topic 8).

use rayon::prelude::*;

use super::background::Decider;
use super::data::{CollegeRow, Division};
use super::program::{GradeCat, Program, record_impact};
use super::season::{SeasonResults, key_normal, key_uniform};
use super::{Alumnus, Person, Stage, World, division_level, geo};
use crate::career::{PIPELINE, Step, position_value};
use crate::player::{Level, PlayingTime};
use crate::positions::{POSITION_COUNT, Position};
use crate::rng::clamp;
use crate::scouting::{ScoutOptions, scout_player};

// --- Data hooks -------------------------------------------------------------------

/// Junior college programs. Placeholder names until the JUCO data pass adds
/// the real programs (topic 6): counts follow where JUCO football is played.
pub(crate) fn juco_rows() -> Vec<CollegeRow> {
    const JUCO_STATES: [(&str, usize); 12] = [
        ("CA", 20),
        ("MS", 14),
        ("KS", 9),
        ("TX", 6),
        ("MN", 6),
        ("IA", 4),
        ("NY", 4),
        ("AZ", 2),
        ("OK", 1),
        ("ND", 1),
        ("MI", 1),
        ("UT", 1),
    ];
    let mut rows = Vec::new();
    for (state, n) in JUCO_STATES {
        for k in 0..n {
            rows.push(CollegeRow {
                name: format!("{state} Junior College {}", k + 1),
                division: Division::JUCO,
                conference: format!("{state} JUCO"),
                city: state.to_string(),
                state: state.to_string(),
                enrollment: 5000,
                national_titles: 0,
                last_5yr_win_pct: Some(0.5),
                stadium_capacity: Some(5000),
                academic_tier: super::data::AcademicTier::Other,
            });
        }
    }
    rows
}

/// Whether a state lets high school players earn NIL money. Filled by the
/// per-state research pass (follow-up answer 5).
pub(crate) fn hs_nil_allowed(code: &str) -> bool {
    super::data::hs_nil(code)
}

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
    let n = w.people.len();
    let mut by_state: Vec<Vec<usize>> = vec![Vec::new(); 51];
    for i in 0..n {
        let p = &w.people[i];
        if p.stage == Stage::HighSchool && (15..=17).contains(&p.player.age) {
            by_state[p.home_state].push(i);
        }
    }
    let mut moves: Vec<(usize, usize, bool)> = Vec::new();
    for s in 0..51 {
        let list = &by_state[s];
        if list.is_empty() {
            continue;
        }
        let mut ovr: Vec<f64> = list.iter().map(|&i| w.people[i].player.overall()).collect();
        ovr.sort_by(|a, b| b.total_cmp(a));
        let bar = ovr[(ovr.len() as f64 * 0.03) as usize];
        let powerhouses: Vec<usize> =
            w.states[s].schools.iter().copied().filter(|&h| w.high_schools[h].is_powerhouse()).collect();
        let nil_state = w.states[s].hs_nil;
        for &i in list {
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
                let nat: Vec<usize> =
                    w.high_schools.iter().filter(|h| h.national && h.state != s).map(|h| h.id).collect();
                if nat.is_empty() { None } else { Some(nat[w.rng.int(0, nat.len() as i32 - 1) as usize]) }
            } else if !powerhouses.is_empty() {
                Some(powerhouses[w.rng.int(0, powerhouses.len() as i32 - 1) as usize])
            } else {
                None
            };
            let Some(t) = target else { continue };
            if t == p.hs || cur.is_powerhouse() && !escape {
                continue;
            }
            let dest = &w.high_schools[t];
            let gain = (dest.prestige - cur.prestige).max(0.0);
            let mut pull = 0.3 * gain;
            if nil_state || (national && w.states[dest.state].hs_nil) {
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
            if dest.private && !national {
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
            if escape {
                pull += 0.015;
            }
            if w.rng.chance(clamp(pull, 0.0, 0.4)) {
                moves.push((i, t, national || escape && w.rng.chance(0.5)));
            }
        }
    }
    for (i, t, family_moved) in moves {
        let from = w.people[i].hs;
        let from_name = w.school_name(from).to_string();
        let to_name = w.school_name(t).to_string();
        let st = w.high_schools[t].state;
        let rule = w.states[st].transfer;
        let p = &mut w.people[i];
        let first = p.transfers == 0;
        p.sit_out = if family_moved || (rule.free_once && first) { 0.0 } else { rule.sit_out };
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
    // Crowded powerhouse depth charts push some kids out again for playing time.
    for i in 0..n {
        let p = &w.people[i];
        if p.stage != Stage::HighSchool
            || !(16..=17).contains(&p.player.age)
            || p.last_role != Some(PlayingTime::Backup)
        {
            continue;
        }
        let cur = &w.high_schools[p.hs];
        if !cur.is_powerhouse() || !w.rng.chance(0.12) {
            continue;
        }
        let state = cur.state;
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
    // Reclassifying: a few repeat a year or finish early to time their college entry.
    for p in w.people.iter_mut() {
        if p.stage != Stage::HighSchool || p.player.age != 16 {
            continue;
        }
        let r = p.rng.next();
        if r < 0.004 && p.player.overall() > 52.0 {
            p.class_of -= 1;
            p.note("Reclassified to graduate a year early.");
        } else if r > 0.995 {
            p.class_of += 1;
            p.note("Reclassified to repeat a year of high school.");
        }
    }
    let _ = year;
}

// --- College exits: awards, the draft ------------------------------------------------

fn competition_discount(d: Division) -> f64 {
    match d {
        Division::FBS => 0.0,
        Division::FCS => 1.0,
        Division::D2 | Division::JUCO => 5.0,
    }
}

/// Stars and awards feed pipelines; then the draft. Only impact grows a
/// pipeline: stat leaders, award winners and picks in rounds 1-3 (topic 4).
pub(crate) fn college_exits(w: &mut World, _r: &SeasonResults) {
    let year = w.year;
    let draft_year = year + 1;
    // --- Impact seasons ---
    for division in [Division::FBS, Division::FCS, Division::D2] {
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
        let leaders = (perf.len() as f64 * 0.02).ceil() as usize;
        let awards = (perf.len() as f64 * 0.003).ceil() as usize;
        for (k, &(_, i)) in perf.iter().take(leaders).enumerate() {
            let (c, s) = (w.people[i].college.unwrap(), w.people[i].home_state);
            let weight = if k < awards { 1.0 } else { 0.5 };
            if w.recording {
                record_impact(&mut w.programs[c], s, weight, year);
            }
            let age = w.people[i].player.age - 1;
            let what = if k < awards { "won national honors" } else { "was among his level's top performers" };
            let school = w.program_name(c).to_string();
            w.people[i].note(format!("At {age} he {what} at {school}."));
        }
    }

    // --- Declaring ---
    let mut entrants: Vec<(usize, f64)> = Vec::new();
    let seed = w.options.seed as u64;
    let n = w.people.len();
    for i in 0..n {
        let p = &w.people[i];
        if p.stage != Stage::College {
            continue;
        }
        let Some(c) = p.college else { continue };
        let division = w.programs[c].division;
        if division == Division::JUCO {
            continue;
        }
        let years = draft_year - p.class_of;
        if years < 3 {
            continue;
        }
        let level = division_level(division);
        let p = &mut w.people[i];
        let rep = scout_player(&p.player, ScoutOptions { accuracy: 13.0, level }, &mut p.rng);
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
        let score =
            rep.overall + (0.25 + 0.2 * youth) * (rep.potential - rep.overall) - competition_discount(division) - flags
                + 0.5 * position_value(p.player.position)
                - if matches!(p.player.position, Position::K | Position::P) { 10.0 } else { 0.0 };
        let exhausted = p.college_seasons >= 4 || years >= 5;
        entrants.push((i, if exhausted { score } else { f64::NAN }));
        if !exhausted {
            // Underclassmen declare when the projection is good; a few gamble.
            entrants.last_mut().unwrap().1 = -score;
        }
    }
    // Underclassmen: projected high picks go; the rest return.
    let mut board: Vec<f64> = entrants.iter().map(|&(_, s)| s.abs()).collect();
    board.sort_by(|a, b| b.total_cmp(a));
    let slots = (PIPELINE.draft_picks * w.options.scale).round().max(1.0) as usize;
    let round2 = board.get((slots * 64 / 257).min(board.len().saturating_sub(1))).copied().unwrap_or(f64::MAX);
    let round4 = board.get((slots * 138 / 257).min(board.len().saturating_sub(1))).copied().unwrap_or(f64::MAX);
    let mut declared: Vec<(usize, f64)> = Vec::new();
    for &(i, s) in &entrants {
        if s >= 0.0 {
            declared.push((i, s));
            continue;
        }
        let score = -s;
        let p = &mut w.people[i];
        let chance = if score >= round2 {
            0.85
        } else if score >= round4 {
            0.35 + 0.02 * (p.family.decider_wants.money * 10.0)
        } else {
            0.04
        };
        if p.rng.chance(chance) {
            declared.push((i, score));
        }
    }
    // The board: each team's front office sees it a little differently.
    for d in declared.iter_mut() {
        d.1 += 2.0 * key_normal(seed ^ draft_year as u64, w.people[d.0].player.id as u64);
    }
    declared.sort_by(|a, b| b.1.total_cmp(&a.1));
    let udfa = (slots as f64 * PIPELINE.udfa_ratio).round() as usize;
    for (k, &(i, _)) in declared.iter().enumerate() {
        let c = w.people[i].college.unwrap();
        let state = w.people[i].home_state;
        let pick = if k < slots { Some(k as u32 + 1) } else { None };
        if k >= slots + udfa {
            let p = &mut w.people[i];
            p.stage = Stage::Done;
            p.note("Went undrafted and unsigned, and his football career ended.");
            continue;
        }
        let school = w.program_name(c).to_string();
        let round = pick.map(|pk| crate::career::draft_round(pk, slots as u32));
        if let Some(r) = round {
            w.programs[c].draft_picks.push((draft_year, r));
            if r <= 3 && w.recording {
                record_impact(&mut w.programs[c], state, 1.0, year);
            }
        }
        let p = &mut w.people[i];
        p.stage = Stage::Pro;
        let age = p.player.age;
        match (pick, round) {
            (Some(pk), Some(r)) => p.note(format!("Drafted in round {r} (pick {pk}) out of {school} at {age}.")),
            _ => p.note(format!("Signed as an undrafted free agent out of {school} at {age}.")),
        }
        if w.recording {
            let a = Alumnus::from_person(p, draft_year, pick, false);
            w.alumni.push(a);
        }
    }
    // Exhausted players who didn't make a board leave football.
    for p in w.people.iter_mut() {
        if p.stage == Stage::College && (draft_year - p.class_of >= 5 || p.college_seasons >= 4) {
            p.stage = Stage::Done;
            p.note("His college eligibility ran out and his playing days ended.");
        }
    }
}

// --- Recruiting ---------------------------------------------------------------------

#[derive(Clone, Copy)]
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
}

/// NCAA initial-eligibility floors and special admissions.
fn academically_eligible(prog: &Program, gpa: f64) -> bool {
    match prog.division {
        Division::JUCO => true,
        Division::D2 => gpa >= 2.2,
        _ => {
            let floor = if prog.conference == "Ivy" {
                3.5
            } else if matches!(prog.real_name.as_str(), "Air Force" | "Army" | "Navy") {
                3.0
            } else {
                2.3
            };
            gpa >= floor
        }
    }
}

fn roster_cap(d: Division) -> usize {
    match d {
        // Roster limits since the 2025 House settlement: 105 in FBS.
        Division::FBS => 105,
        Division::FCS => 100,
        Division::D2 => 110,
        Division::JUCO => 80,
    }
}

fn level_value(d: Division) -> f64 {
    match d {
        Division::FBS => 1.0,
        Division::FCS => 0.55,
        Division::D2 => 0.3,
        Division::JUCO => 0.35,
    }
}

/// How much a school appeals to a recruit, from the wants of whoever decides (topic 5).
fn utility(p: &Person, wants: &super::background::Wants, prog: &Program, nil_offer: f64, eval_rank: f64) -> f64 {
    let g = |c: GradeCat| prog.grades[c as usize] / 100.0;
    let miles = geo::miles(p.home_state, prog.state);
    let close = clamp(1.0 - miles / 1500.0, 0.0, 1.0);
    let crisis = p.bg.story.iter().rev().take(1).any(|s| s.contains("death") || s.contains("lost a job"));
    let close_w = wants.close_to_home + if crisis { 0.15 } else { 0.0 };
    let prestige = (g(GradeCat::TitleChances)
        + g(GradeCat::StadiumAtmosphere)
        + g(GradeCat::Tradition)
        + g(GradeCat::BrandExposure))
        / 4.0;
    let money = clamp((nil_offer.max(1.0)).log10() / 6.5, 0.0, 1.0);
    let playing = g(GradeCat::PlayingTime) * (0.6 + 0.4 * eval_rank);
    let academics = g(GradeCat::Academics) * (0.5 + 0.5 * clamp((p.bg.gpa - 2.5) / 1.5, 0.0, 1.0));
    let development = (g(GradeCat::ProPipeline) + g(GradeCat::CoachPrestige)) / 2.0;
    // Every recruit wants the biggest name he can get at a level; how much
    // more depends on what he (or his decider) wants.
    let mut u = 1.6 * level_value(prog.division) * (0.5 + wants.prestige + wants.development)
        + (0.25 + wants.prestige) * prog.prestige_rel * (0.4 + 0.6 * level_value(prog.division))
        + 0.1 * g(GradeCat::Facilities)
        + wants.prestige * prestige
        + wants.money * money * 1.5
        + close_w * close
        + wants.playing_time * playing
        + wants.academics * academics
        + wants.development * development;
    if p.family.older_brother_at == Some(prog.id) {
        u += if p.family.follows_brother { 0.15 } else { -0.05 };
    }
    u
}

/// Signing day: HS seniors, prep-school kids and JUCO players looking to move up.
pub(crate) fn signing_day(w: &mut World) {
    let year = w.year;
    let sign_year = year + 1;
    let seed = w.options.seed as u64 ^ (sign_year as u64) << 20;
    let n = w.people.len();
    let mut pool: Vec<Recruit> = Vec::new();
    // Film aligned by position: each position's top looks the same.
    let mut pos_stats: [(f64, f64, f64); POSITION_COUNT] = [(0.0, 0.0, 0.0); POSITION_COUNT];
    let mut candidates: Vec<usize> = Vec::new();
    for i in 0..n {
        let p = &w.people[i];
        let senior = p.stage == Stage::HighSchool && p.class_of <= sign_year;
        let prep = p.stage == Stage::PrepSchool;
        let juco = p.stage == Stage::College
            && p.college.is_some_and(|c| w.programs[c].division == Division::JUCO)
            && p.college_seasons >= 1;
        if senior || prep || juco {
            candidates.push(i);
            if senior {
                let f = p.last_performance.unwrap_or(p.player.overall());
                let s = &mut pos_stats[p.player.position.index()];
                s.0 += 1.0;
                s.1 += f;
                s.2 += f * f;
            }
        }
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
    let culture: Vec<f64> = w.states.iter().map(|s| s.culture).collect();
    let schools = &w.high_schools;
    let coaches = &w.coaches;
    let programs = &w.programs;
    let recruits: Vec<Recruit> = candidates
        .par_iter()
        .map(|&i| {
            // (Visibility and film read only; the person's own stream isn't touched here.)
            let p = &w.people[i];
            let raw = p.last_performance.unwrap_or(p.player.overall());
            let (m, sd) = pos_mean_sd[p.player.position.index()];
            let film = all_mean_sd.0 + all_mean_sd.1 * (raw - m) / sd;
            let juco_mover = p.stage == Stage::College;
            let hs = &schools[p.hs];
            let school_exposure = if juco_mover {
                0.7
            } else if p.stage == Stage::PrepSchool {
                0.6
            } else {
                hs.prestige
            };
            let network = if juco_mover {
                p.college.and_then(|c| programs[c].head_coach).map_or(0.5, |c| coaches[c].ability().network / 20.0)
            } else {
                hs.coach.map_or(0.4, |c| coaches[c].ability().network / 20.0)
            };
            let camps = 0.3 + 0.7 * p.bg.money;
            let tape = clamp((film - all_mean_sd.0) / (3.0 * all_mean_sd.1) + 0.3, 0.0, 1.0);
            let traffic = 0.1 * (culture[p.home_state] + 0.6);
            let viral_odds =
                if p.family.parent_style == super::background::ParentStyle::SocialMedia { 0.06 } else { 0.02 };
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
            let visibility = clamp(
                0.05 + 0.3 * school_exposure + 0.2 * network + 0.15 * camps + 0.25 * tape + traffic + viral + bloodline,
                0.0,
                1.0,
            );
            Recruit { person: i, film, projection: 0.0, visibility, gpa: p.bg.gpa, juco_mover, viral: went_viral }
        })
        .collect();
    pool.extend(recruits);
    // Older brothers already playing college football, from the same high school (topic 5).
    let mut alumni_of: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for p in w.people.iter() {
        if p.stage == Stage::College
            && let Some(c) = p.college
        {
            alumni_of.entry(p.hs).or_default().push(c);
        }
    }
    for r in pool.iter() {
        if r.juco_mover {
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
    // Services' upside guess: one shared projection.
    for r in pool.iter_mut() {
        let p = &mut w.people[r.person];
        let rep = scout_player(&p.player, ScoutOptions { accuracy: 12.0, level: Level::HS }, &mut p.rng);
        r.projection = rep.potential;
        p.visibility = r.visibility;
        p.bg.exposure_network = r.visibility;
        if r.viral {
            let age = p.player.age;
            p.note(format!("At {age} a highlight clip went viral and got him a first look from recruiters."));
        }
    }

    // --- Star ratings: the services only rank kids they have seen -------------------------
    let seniors: Vec<usize> = (0..pool.len()).filter(|&k| !pool[k].juco_mover).collect();
    let mut by_star: Vec<usize> = seniors.clone();
    let star_score = |r: &Recruit| 0.25 * r.film + 0.75 * r.projection - 12.0 * (1.0 - r.visibility).powi(2);
    by_star.sort_by(|&a, &b| star_score(&pool[b]).total_cmp(&star_score(&pool[a])));
    let class = 250_000.0 * w.options.scale;
    let mut cursor = 0;
    for (stars, share) in PIPELINE.star_shares {
        let count = (share * class).round() as usize;
        for &k in by_star.iter().skip(cursor).take(count) {
            let i = pool[k].person;
            if w.people[i].player.position.is_specialist() && stars >= 4 {
                continue;
            }
            w.people[i].stars = stars;
        }
        cursor += count;
    }

    if w.recording {
        let mut counts = [0u32; 6];
        for &k in &seniors {
            counts[w.people[pool[k].person].stars as usize] += 1;
        }
        w.signing_classes.push((sign_year, counts));
    }

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
    // A compact copy of what staffs look at, so the scan stays in cache.
    struct Cand {
        k: usize,
        state: usize,
        stars: u8,
        vis: f64,
        gpa: f64,
        film: f64,
        proj: f64,
        id: u64,
        specialist: bool,
        juco_mover: bool,
    }
    let cands: Vec<Cand> = interest
        .iter()
        .map(|&k| {
            let r = &pool[k];
            let p = &w.people[r.person];
            Cand {
                k,
                state: p.home_state,
                stars: p.stars,
                vis: r.visibility,
                gpa: r.gpa,
                film: r.film,
                proj: r.projection,
                id: p.player.id as u64,
                specialist: p.player.position.is_specialist(),
                juco_mover: r.juco_mover,
            }
        })
        .collect();
    let programs = &w.programs;
    let coaches = &w.coaches;
    let lists: Vec<Vec<(usize, f64)>> = programs
        .par_iter()
        .map(|prog| {
            let rec_skill = prog.head_coach.map_or(10.0, |c| coaches[c].ability().recruiting);
            let noise_sd = clamp(3.5 - 0.15 * (rec_skill - 10.0), 1.0, 6.0);
            let pseed = seed ^ (prog.id as u64).wrapping_mul(0x1000_0001);
            let mut seen: Vec<(usize, f64)> = Vec::new();
            for c in &cands {
                // A JUCO player moves up to a four-year school or nowhere.
                if c.juco_mover && prog.division == Division::JUCO {
                    continue;
                }
                // Staffs don't waste offers on blue-chips far above their level.
                if (prog.division == Division::D2 && c.stars >= 4) || (prog.division == Division::FCS && c.stars >= 5) {
                    continue;
                }
                // A blue-chip is seen by everyone; others only inside a footprint.
                let bonus = if c.stars >= 4 { 1.0 } else { 0.15 * c.stars as f64 };
                let see = prog.footprint[c.state] * c.vis * 1.6 + bonus;
                if see < 1.0 && key_uniform(pseed, c.id) > see {
                    continue;
                }
                if !academically_eligible(prog, c.gpa) {
                    continue;
                }
                let read = |k: u64| noise_sd * key_normal(pseed.wrapping_add(k), c.id);
                // Staffs offer on film more than on a service's projection.
                let eval = 0.6 * (c.film + read(1)) + 0.4 * (c.proj + read(2)) - if c.specialist { 8.0 } else { 0.0 };
                seen.push((c.k, eval));
            }
            seen.sort_by(|a, b| b.1.total_cmp(&a.1));
            seen.truncate(prog.class_size * 40);
            seen
        })
        .collect();

    dbg_lap("lists");
    // Slots: a class, bent by roster room.
    let slots: Vec<usize> = programs
        .iter()
        .map(|prog| {
            let room = roster_cap(prog.division).saturating_sub(prog.roster.len());
            (room as f64).min(prog.class_size as f64 * 1.4) as usize
        })
        .collect();

    // --- Deferred acceptance: programs offer down their boards; each recruit holds the best offer.
    let pn = programs.len();
    let mut next = vec![0usize; pn];
    let mut held: Vec<Option<(usize, f64)>> = vec![None; pool.len()];
    let mut taken = vec![0usize; pn];
    let mut offers: Vec<Vec<usize>> = vec![Vec::new(); pool.len()];
    let budget_rank = |prog: &Program, k: f64| prog.collective.budget * 0.6 / prog.class_size as f64 * (2.5 - 2.0 * k);
    let mut active = true;
    while active {
        active = false;
        for j in 0..pn {
            while taken[j] < slots[j] && next[j] < lists[j].len() {
                let (k, _) = lists[j][next[j]];
                let rank = next[j] as f64 / lists[j].len().max(1) as f64;
                next[j] += 1;
                active = true;
                offers[k].push(j);
                let p = &w.people[pool[k].person];
                let wants = p.family.decider_wants;
                let u = utility(p, &wants, &programs[j], budget_rank(&programs[j], rank), 1.0 - rank)
                    + 0.04 * key_normal(seed ^ 77 ^ j as u64, p.player.id as u64);
                match held[k] {
                    Some((_, cu)) if cu >= u => {}
                    Some((cur, _)) => {
                        taken[cur] -= 1;
                        held[k] = Some((j, u));
                        taken[j] += 1;
                    }
                    None => {
                        held[k] = Some((j, u));
                        taken[j] += 1;
                    }
                }
            }
        }
    }

    dbg_lap("deferred acceptance");
    if std::env::var("AFM_DEBUG").is_ok() {
        let mut by_div = [0usize; 4];
        for h in held.iter().flatten() {
            by_div[programs[h.0].division as usize] += 1;
        }
        let want: Vec<usize> = (0..4)
            .map(|d| programs.iter().enumerate().filter(|(_, p)| p.division as usize == d).map(|(j, _)| slots[j]).sum())
            .collect();
        let listed: usize = lists.iter().map(|l| l.len()).sum();
        eprintln!(
            "signing {sign_year}: pool {} interest {} listed {listed} signed {:?} slots {:?}",
            pool.len(),
            interest.len(),
            by_div,
            want
        );
    }
    // --- Outcomes ---------------------------------------------------------------------
    for (k, r) in pool.iter().enumerate() {
        let i = r.person;
        let from_hs = w.school_name(w.people[i].hs).to_string();
        match held[k] {
            Some((j, _)) => {
                let prog_name = w.program_name(j).to_string();
                let division = w.programs[j].division;
                // Was he steered? Compare with where he himself would have gone among his offers.
                let p = &w.people[i];
                let steered = p.family.decider != Decider::Himself && {
                    let own = p.family.own_wants;
                    let mine = utility(p, &own, &w.programs[j], 1e5, 0.5);
                    // His own favorite among the programs that wanted him.
                    offers[k].iter().any(|&jj| jj != j && utility(p, &own, &w.programs[jj], 1e5, 0.5) > mine + 0.06)
                };
                if w.recording && !r.juco_mover && division == Division::FBS {
                    w.fbs_signees_by_state[w.people[i].home_state] += 1;
                }
                let p = &mut w.people[i];
                let step = match (r.juco_mover, division) {
                    (true, Division::JUCO) => Step::Sideways,
                    (true, _) => Step::Up,
                    (false, Division::FBS) => Step::HsToFbs,
                    (false, Division::FCS) => Step::HsToFcs,
                    (false, _) => Step::HsToSmallCollege,
                };
                p.pending_dip = Some(World::adjustment(p, step));
                let age = p.player.age;
                let stars = if p.stars > 0 { format!("a {}-star", p.stars) } else { "an unranked recruit".to_string() };
                if r.juco_mover {
                    p.note(format!("At {age} his JUCO film earned a move up to {prog_name} ({}).", division.name()));
                    p.college_seasons = p.college_seasons.min(2);
                } else {
                    p.note(format!(
                        "Signed with {prog_name} ({}) at {age} as {stars} out of {from_hs}.",
                        division.name()
                    ));
                    p.college_seasons = 0;
                    p.redshirt_used = false;
                }
                if division == Division::JUCO {
                    p.juco = true;
                }
                if steered {
                    p.steered = true;
                    let who = match p.family.decider {
                        Decider::Parent => "a parent",
                        Decider::Grandparent => "his grandparents",
                        Decider::Handler => "a handler close to the family",
                        Decider::Himself => "himself",
                    };
                    p.note(format!("The choice was {who}'s more than his own."));
                }
                p.stage = Stage::College;
                p.college = Some(j);
                p.path.push(j);
                p.on_notice = false;
            }
            None => {
                let p = &mut w.people[i];
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
                    let age = p.player.age;
                    p.note(format!("At {age} he went to prep school for a year to fix his grades."));
                } else {
                    p.stage = Stage::Done;
                }
            }
        }
    }
    // Seniors not in the pool's interest list simply finish high school.
    for p in w.people.iter_mut() {
        if p.stage == Stage::HighSchool && p.class_of <= sign_year {
            p.stage = Stage::Done;
        }
    }
    let _ = n;
}

// --- The transfer portal --------------------------------------------------------------

/// Players short of snaps, after a coaching change, steered kids now free to
/// choose, standouts moving up (often after quiet tampering), and dismissed
/// players starting over. NIL money is then shared out.
pub(crate) fn portal(w: &mut World, _r: &SeasonResults) {
    let year = w.year;
    let seed = w.options.seed as u64 ^ ((year as u64) << 24) ^ 0xA5;
    let n = w.people.len();
    // Position medians of starters' play by division, the bar to move up.
    let mut starters: [[Vec<f64>; POSITION_COUNT]; 4] = Default::default();
    for p in w.people.iter() {
        if p.stage == Stage::College
            && p.last_role == Some(PlayingTime::Starter)
            && let (Some(c), Some(perf)) = (p.college, p.last_performance)
        {
            starters[w.programs[c].division as usize][p.player.position.index()].push(perf);
        }
    }
    for d in starters.iter_mut() {
        for v in d.iter_mut() {
            v.sort_by(|a, b| a.total_cmp(b));
        }
    }
    let median = |d: Division, pos: Position| -> f64 {
        let v = &starters[d as usize][pos.index()];
        if v.is_empty() { 99.0 } else { v[v.len() / 2] }
    };
    let new_staff: Vec<bool> = w
        .programs
        .iter()
        .map(|p| p.head_coach.is_some_and(|c| w.coaches[c].job.is_some_and(|j| j.since == year)))
        .collect();

    #[derive(Clone, Copy, PartialEq)]
    enum Why {
        PlayingTime,
        CoachingChange,
        Steered,
        MoveUp,
        Dismissed,
        Money,
    }
    let mut entrants: Vec<(usize, Why)> = Vec::new();
    let mut steered_stays: Vec<usize> = Vec::new();
    for i in 0..n {
        let p = &w.people[i];
        if p.stage != Stage::College {
            continue;
        }
        let Some(c) = p.college else { continue };
        let div = w.programs[c].division;
        if p.player.history.last().is_some_and(|h| h.events.has(crate::player::DevelopmentEvent::Dismissed)) {
            entrants.push((i, Why::Dismissed));
            continue;
        }
        if p.college_seasons >= 4 || div == Division::JUCO {
            continue;
        }
        let pos = p.player.position;
        let perf = p.last_performance.unwrap_or(0.0);
        let up = match div {
            Division::FCS => Some(Division::FBS),
            Division::D2 => Some(Division::FCS),
            _ => None,
        };
        let u = 0.5 + 0.5 * (key_normal(seed, p.player.id as u64) / 3.0).clamp(-1.0, 1.0);
        // Standouts below FBS get noticed; richer programs quietly reach out first (never punished).
        if let Some(up) = up
            && perf >= median(up, pos) + 1.0
        {
            let tampered = w.programs[c].collective.budget < 2.0e6;
            let chance = PIPELINE.transfer_chance * if tampered { 1.6 } else { 1.0 };
            if u < chance {
                entrants.push((i, Why::MoveUp));
                continue;
            }
        }
        // A kid steered to the wrong school decides once, after his first year, whether to leave.
        if p.steered && p.player.age >= 18 {
            if u < 0.45 {
                entrants.push((i, Why::Steered));
                continue;
            }
            steered_stays.push(i);
        }
        let base = match p.last_role {
            Some(PlayingTime::Backup) => PIPELINE.portal_entry.backup,
            Some(PlayingTime::Rotation) => PIPELINE.portal_entry.rotation,
            _ => 0.0,
        };
        let changed = new_staff[c];
        let chance = base
            * if changed { PIPELINE.portal_entry.coaching_change } else { 1.0 }
            * (1.0 + (p.player.personality.competitiveness - 10.0) * 0.04);
        if u < chance {
            entrants.push((i, if changed { Why::CoachingChange } else { Why::PlayingTime }));
            continue;
        }
        // A greedy starter at a poor program can be bought away.
        if p.last_role == Some(PlayingTime::Starter)
            && w.programs[c].collective.budget < 1.0e6
            && div == Division::FBS
            && u < 0.06 * p.family.own_wants.money * 4.0
        {
            entrants.push((i, Why::Money));
        }
    }

    for i in steered_stays {
        w.people[i].steered = false;
    }
    // Each entrant shops among programs that would take him.
    w.rebuild_rosters();
    let mut adds = vec![0usize; w.programs.len()];
    let pn = w.programs.len();
    for (i, why) in entrants {
        let c = w.people[i].college.unwrap();
        let from = w.programs[c].division;
        let pos = w.people[i].player.position;
        let perf = w.people[i].last_performance.unwrap_or(w.people[i].player.overall());
        let serious = why == Why::Dismissed && w.people[i].rng.chance(PIPELINE.college_trouble.serious_share);
        if serious && w.people[i].rng.chance(PIPELINE.college_trouble.serious_quit_share) {
            let p = &mut w.people[i];
            p.stage = Stage::Done;
            p.note("After a serious incident no program would take him, and he never played again.");
            continue;
        }
        let mut best: Option<(usize, f64)> = None;
        for _ in 0..40 {
            let j = w.rng.int(0, pn as i32 - 1) as usize;
            if j == c || adds[j] >= 10 || w.programs[j].roster.len() + adds[j] >= roster_cap(w.programs[j].division) {
                continue;
            }
            let d = w.programs[j].division;
            // A dismissed player starts over a level down or at JUCO.
            let allowed = match why {
                Why::Dismissed => d as usize > from as usize || (d == from && !serious && perf >= median(d, pos)),
                Why::MoveUp => (d as usize) < from as usize,
                _ => d as usize >= from as usize || d == Division::FBS,
            };
            if !allowed || perf + 2.0 < median(d, pos) - if d as usize > from as usize { 3.0 } else { 0.0 } {
                continue;
            }
            let p = &w.people[i];
            let u = utility(p, &p.family.own_wants, &w.programs[j], w.programs[j].collective.budget / 40.0, 0.6);
            if best.is_none_or(|(_, bu)| u > bu) {
                best = Some((j, u));
            }
        }
        let age = w.people[i].player.age - 1;
        let from_name = w.program_name(c).to_string();
        let what = match why {
            Why::PlayingTime => "short of snaps",
            Why::CoachingChange => "after a coaching change",
            Why::Steered => "free at last to choose for himself",
            Why::MoveUp => "after standing out a level below",
            Why::Dismissed => "after his dismissal",
            Why::Money => "for a bigger NIL deal",
        };
        let Some((j, _)) = best else {
            let p = &mut w.people[i];
            p.stage = Stage::Done;
            p.note(format!("Entered the portal at {age} {what}, found no school and left football."));
            continue;
        };
        adds[j] += 1;
        let to = w.programs[j].division;
        let to_name = w.program_name(j).to_string();
        let step = if (to as usize) < from as usize {
            Step::Up
        } else if to == from {
            Step::Sideways
        } else {
            Step::Down
        };
        let p = &mut w.people[i];
        p.pending_dip = Some(World::adjustment(p, step));
        p.college = Some(j);
        p.path.push(j);
        p.transfers += 1;
        p.steered = false;
        p.on_notice = false;
        if to == Division::JUCO {
            p.juco = true;
        }
        p.note(format!("Entered the portal at {age} {what} and moved from {from_name} to {to_name} ({}).", to.name()));
    }

    // --- NIL: each collective pays its roster; big money can backfire ----------------------
    w.rebuild_rosters();
    for j in 0..pn {
        let budget = w.programs[j].collective.budget * 0.6;
        let roster = w.programs[j].roster.clone();
        if roster.is_empty() {
            continue;
        }
        let value = |p: &Person| {
            let role = match p.last_role {
                Some(PlayingTime::Starter) => 3.0,
                Some(PlayingTime::Rotation) => 1.2,
                _ => 0.3,
            };
            let pos = 1.0 + 0.25 * position_value(p.player.position).max(-2.0);
            role * pos * (1.0 + 0.1 * p.stars as f64)
        };
        let total: f64 = roster.iter().map(|&i| value(&w.people[i])).sum();
        let mut paid: Vec<(f64, usize)> =
            roster.iter().map(|&i| (budget * value(&w.people[i]) / total.max(1e-9), i)).collect();
        paid.sort_by(|a, b| b.0.total_cmp(&a.0));
        let team_median = {
            let mut v: Vec<f64> = roster.iter().filter_map(|&i| w.people[i].last_performance).collect();
            v.sort_by(|a, b| a.total_cmp(b));
            if v.is_empty() { 50.0 } else { v[v.len() / 2] }
        };
        let mut jealousy = 0.0;
        for (k, &(money, i)) in paid.iter().enumerate() {
            let p = &mut w.people[i];
            p.nil_income = money;
            // The top earners on a team: payouts can bring jealousy, complacency or pressure.
            if k < 3 && money > 250_000.0 {
                let perf = p.last_performance.unwrap_or(team_median);
                if perf < team_median {
                    p.bg.stability = clamp(p.bg.stability - 1.0, 1.0, 20.0);
                    jealousy += 0.2;
                    let age = p.player.age;
                    p.note(format!(
                        "At {age}, paid among the most on his team, he struggled and took heavy criticism."
                    ));
                } else if p.rng.chance(0.08) {
                    p.player.personality.work_ethic = clamp(p.player.personality.work_ethic - 1.0, 1.0, 20.0);
                    let age = p.player.age;
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
    let _ = Level::HS;
}
