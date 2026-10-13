//! One season at every level: depth charts, development, and the games.
//!
//! The games are a stand-in until the match engine (section 3): a game is won
//! by comparing the two teams' starters as they played that year (form,
//! slumps and breakouts included), plus home field and luck.

use rayon::prelude::*;

use super::background;
use super::coach::{Group, TitleKind};
use super::data::Division;
use super::{Stage, World, division_level};
use crate::athlete::Eye;
use crate::development::{SeasonContext, Trouble, develop_season};
use crate::league::depth_slots;
use crate::player::{Level, PlayingTime};
use crate::positions::{POSITION_COUNT, Position};
use crate::rng::clamp;

#[derive(Clone, Debug, Default)]
pub(crate) struct TeamResult {
    pub wins: u32,
    pub losses: u32,
    pub conf_wins: u32,
    pub conf_losses: u32,
    /// Wins the roster should have won, from preseason strength.
    pub expected: f64,
    pub strength: f64,
    pub preseason: f64,
    pub conference_title: bool,
    pub national_title: bool,
    pub state_title: bool,
    /// Off-field incidents on the roster this season.
    pub incidents: u32,
    pub roster: u32,
}

impl TeamResult {
    pub fn games(&self) -> u32 {
        self.wins + self.losses
    }
    pub fn win_pct(&self) -> f64 {
        if self.games() == 0 { 0.5 } else { self.wins as f64 / self.games() as f64 }
    }
    /// Wins above expectation as a share of the season.
    pub fn vs_expectation(&self) -> f64 {
        if self.games() == 0 { 0.0 } else { (self.wins as f64 - self.expected) / self.games() as f64 }
    }
    pub fn best_title(&self) -> TitleKind {
        if self.national_title {
            TitleKind::National
        } else if self.state_title {
            TitleKind::State
        } else if self.conference_title {
            TitleKind::Conference
        } else {
            TitleKind::None
        }
    }
}

pub(crate) struct SeasonResults {
    pub hs: Vec<TeamResult>,
    pub college: Vec<TeamResult>,
}

/// A cheap, order-independent uniform draw in [0, 1) from a key.
#[inline]
pub(crate) fn key_uniform(a: u64, b: u64) -> f64 {
    let mut x = a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_mul(0xD1B5_4A32_D192_ED03);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    ((x ^ (x >> 31)) >> 11) as f64 / (1u64 << 53) as f64
}

/// A cheap, order-independent normal draw from a key, so parallel work stays reproducible.
pub(crate) fn key_normal(a: u64, b: u64) -> f64 {
    let mut z = a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_add(0x632B_E59B_D9B4_E019);
    let mut next = || {
        z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut x = z;
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((x ^ (x >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    };
    let u1 = next().max(1e-12);
    let u2 = next();
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

/// Roles on one team's depth chart, from how the staff sees each player.
fn depth_chart(
    entries: &mut [(usize, Position, f64)],
    college: bool,
    redshirt_free: &dyn Fn(usize) -> bool,
) -> Vec<(usize, PlayingTime)> {
    let mut out = Vec::with_capacity(entries.len());
    let mut by_pos: [Vec<(usize, f64)>; POSITION_COUNT] = Default::default();
    for &(i, pos, seen) in entries.iter() {
        by_pos[pos.index()].push((i, seen));
    }
    for pos in Position::ALL {
        let list = &mut by_pos[pos.index()];
        list.sort_by(|a, b| b.1.total_cmp(&a.1));
        let slots = depth_slots(pos);
        for (k, &(i, _)) in list.iter().enumerate() {
            let role = if k < slots.starters {
                PlayingTime::Starter
            } else if k < slots.starters + slots.rotation.max(1) {
                PlayingTime::Rotation
            } else if college && redshirt_free(i) {
                PlayingTime::Redshirt
            } else {
                PlayingTime::Backup
            };
            out.push((i, role));
        }
    }
    out
}

pub(crate) fn play_seasons(w: &mut World) -> SeasonResults {
    let year = w.year as u64;
    let seed = w.options.seed as u64;
    let n = w.people.len();
    let mut roles = vec![PlayingTime::Backup; n];

    // --- Where everyone plays (topic 5.2) ---------------------------------------
    // Before each season, staffs take a fresh look: a boy who grew, a new
    // college staff, an athlete signed without a set position.
    let eyes: Vec<Option<(Eye, f64)>> = w
        .people
        .iter()
        .map(|p| match p.stage {
            Stage::HighSchool if p.player.age > 14 => Some((w.hs_eye(p.hs), 0.03)),
            Stage::College => p.college.map(|c| {
                let prog = &w.programs[c];
                let skill = prog.head_coach.map_or(10.0, |h| w.coaches[h].ability().head_coach_value());
                (Eye::coach(skill).staff(0x434F_0000 ^ c as u32), 0.015)
            }),
            _ => None,
        })
        .collect();
    w.people.par_iter_mut().zip(eyes.par_iter()).for_each(|(p, eye)| {
        let Some((eye, margin)) = *eye else { return };
        if let Some(from) = crate::athlete::reconsider(&mut p.player, eye, margin) {
            let (age, to) = (p.player.age, p.player.position);
            let by = if p.stage == Stage::College { "his college staff" } else { "his high school coach" };
            p.note(format!("Moved from {from} to {to} by {by} at {age}."));
        }
    });

    // --- Depth charts ---------------------------------------------------------
    let people = &w.people;
    let hs_roles: Vec<Vec<(usize, PlayingTime)>> = w
        .high_schools
        .par_iter()
        .map(|s| {
            let mut entries: Vec<(usize, Position, f64)> = s
                .roster
                .iter()
                .map(|&i| {
                    let p = &people[i];
                    let seen = p.player.overall() + 1.5 * key_normal(seed ^ year, p.player.id as u64);
                    (i, p.player.position, seen)
                })
                .collect();
            let mut chart = depth_chart(&mut entries, false, &|_| false);
            // A transfer sitting out under his state's rule plays no varsity games.
            for (i, role) in chart.iter_mut() {
                let p = &people[*i];
                if p.sit_out > 0.0 && key_normal(seed ^ year ^ 7, p.player.id as u64).abs() < 1.6 * p.sit_out {
                    *role = PlayingTime::Redshirt;
                }
            }
            chart
        })
        .collect();
    let programs = &w.programs;
    let college_roles: Vec<Vec<(usize, PlayingTime)>> = programs
        .par_iter()
        .map(|prog| {
            let mut entries: Vec<(usize, Position, f64)> = prog
                .roster
                .iter()
                .map(|&i| {
                    let p = &people[i];
                    // Young players must beat the upperclassmen ahead of them.
                    let blocked = match p.college_seasons {
                        0 => 4.0 + 6.0 * prog.prestige,
                        1 => 2.0 + 3.0 * prog.prestige,
                        _ => 0.0,
                    };
                    let seen = p.player.overall() - 12.0 * p.pending_dip.unwrap_or(0.0) - blocked
                        + 1.5 * key_normal(seed ^ year, p.player.id as u64);
                    (i, p.player.position, seen)
                })
                .collect();
            depth_chart(&mut entries, true, &|i| {
                let p = &people[i];
                !p.redshirt_used && p.college_seasons <= 1
            })
        })
        .collect();
    for (i, role) in hs_roles.into_iter().flatten().chain(college_roles.into_iter().flatten()) {
        roles[i] = role;
    }

    // --- Development -----------------------------------------------------------
    let (people, schools, programs, coaches) = (&mut w.people, &w.high_schools, &w.programs, &w.coaches);
    people.par_iter_mut().zip(roles.par_iter()).for_each(|(p, &role)| {
        p.incident = false;
        let group = Group::of(p.player.position);
        let (level, coaching, team_situation, base_trouble) = match p.stage {
            Stage::HighSchool => {
                let s = &schools[p.hs];
                let teach = s.coach.map_or(7.0, |c| coaches[c].teaching(group));
                // Bigger, richer staffs add position coaches beside the head coach.
                (Level::HS, 0.6 * teach + 0.4 * (4.0 + 12.0 * s.resources), (s.resources - 0.5) * 0.1, 0.012)
            }
            Stage::College => {
                let prog = &programs[p.college.unwrap_or(0)];
                p.college_staff.push((year as i32, prog.id, prog.head_coach, prog.position_coaches[group as usize]));
                let hc = prog.head_coach.map_or(9.0, |c| coaches[c].teaching(group));
                let coord = if group.offense() { prog.oc } else { prog.dc };
                let co = coord.map_or(hc, |c| coaches[c].teaching(group));
                // His position coach teaches him most days (note 5).
                let pc = prog.position_coaches[group as usize].map_or(co, |c| coaches[c].teaching(group));
                (
                    division_level(prog.division),
                    0.2 * hc + 0.35 * co + 0.3 * pc + 0.15 * (6.0 + 12.0 * prog.facilities),
                    (prog.facilities - 0.5) * 0.15,
                    crate::career::PIPELINE.college_trouble.chance,
                )
            }
            Stage::PrepSchool => (Level::HS, 12.0, 0.05, 0.01),
            _ => return,
        };
        let role = if p.stage == Stage::PrepSchool { PlayingTime::Rotation } else { role };
        let pers = &p.player.personality;
        let trouble_p = base_trouble
            * clamp(1.0 + (10.0 - pers.work_ethic) * 0.12 + (10.0 - pers.coachability) * 0.06, 0.25, 3.0)
            * background::trouble_multiplier(&p.player, &p.bg);
        let mut trouble = None;
        if p.rng.chance(trouble_p) {
            p.incident = true;
            trouble = Some(
                if p.stage == Stage::College
                    && (p.on_notice || p.rng.chance(crate::career::PIPELINE.college_trouble.dismissal_share))
                {
                    Trouble::Dismissed
                } else {
                    Trouble::Suspended
                },
            );
        }
        let role = if trouble.is_some() { PlayingTime::Backup } else { role };
        // Academics (note 6): an academic redshirt sits his first year; a
        // borderline student can lose his eligibility, less often with strong
        // academic support. Neither uses a season of eligibility.
        let mut academic_sit = false;
        if p.stage == Stage::College && trouble.is_none() {
            let prog = &programs[p.college.unwrap_or(0)];
            let support = clamp(0.3 + 0.7 * prog.facilities, 0.0, 1.0);
            if p.academic_redshirt {
                academic_sit = true;
                p.academic_redshirt = false;
            } else if prog.division != Division::JUCO {
                let risk = 0.06 * clamp((2.5 - p.bg.gpa) / 0.5, 0.0, 1.0) * (1.3 - support);
                if p.rng.chance(risk) {
                    academic_sit = true;
                    let age = p.player.age;
                    p.note(format!("Academically ineligible for the season at {age} (GPA {:.2}).", p.bg.gpa));
                }
            }
            let effort = 0.012 * (p.player.personality.work_ethic - 10.0) + 0.06 * (support - 0.5);
            p.bg.gpa = clamp(p.bg.gpa + effort + p.rng.normal(0.0, 0.1), 0.0, 4.0);
        }
        let role = if academic_sit { PlayingTime::Redshirt } else { role };
        let ctx = SeasonContext {
            trouble,
            adjustment: p.pending_dip.take().unwrap_or(0.0),
            situation: clamp(team_situation + background::situation_push(&p.player, &p.bg), -1.0, 1.0),
            ..SeasonContext::new(level, clamp(coaching, 1.0, 20.0), role)
        };
        develop_season(&mut p.player, &ctx, &mut p.rng);
        p.last_role = Some(role);
        p.last_performance = p.player.history.last().and_then(|h| h.performance);
        p.sit_out = 0.0;
        // JUCO classes: transferable credits and grades build back, or don't.
        if p.stage == Stage::College
            && p.juco
            && programs[p.college.unwrap_or(0)].division == super::data::Division::JUCO
        {
            let effort = (p.player.personality.work_ethic - 10.0) * 0.03 + (p.bg.stability - 10.0) * 0.02;
            p.bg.gpa = clamp(p.bg.gpa + p.rng.range(-0.1, 0.45) + effort, 0.0, 4.0);
        }
        // An academic sit is a year on the clock without a season of competition.
        if p.stage == Stage::College && !academic_sit {
            if role == PlayingTime::Redshirt {
                p.redshirt_used = true;
            } else if trouble.is_none() {
                p.college_seasons += 1;
            }
        }
        match trouble {
            Some(Trouble::Suspended) => {
                p.on_notice = true;
                let age = p.player.age - 1;
                p.note(format!("Suspended for the season at {age}."));
            }
            Some(Trouble::Dismissed) => {
                let age = p.player.age - 1;
                p.note(format!("Dismissed from his team at {age}."));
            }
            None => {}
        }
        if p.rng.chance(background::quit_chance(&p.player, &p.bg)) {
            let age = p.player.age - 1;
            p.note(format!("Walked away from football at {age}."));
            p.stage = Stage::Done;
        }
    });

    attach_level_ratings(w);

    // --- Team strength -----------------------------------------------------------
    let team_strength = |roster: &[usize], people: &[super::Person]| -> (f64, f64, u32) {
        let mut sum = 0.0;
        let mut pre = 0.0;
        let mut n = 0.0;
        let mut incidents = 0;
        for &i in roster {
            let p = &people[i];
            if p.incident {
                incidents += 1;
            }
            if p.last_role != Some(PlayingTime::Starter) || p.player.position.is_specialist() {
                continue;
            }
            let h = p.player.history.last();
            pre += h.map_or(p.player.overall(), |h| h.overall_start);
            sum += p.last_performance.unwrap_or_else(|| h.map_or(p.player.overall(), |h| h.overall_start));
            n += 1.0;
        }
        if n == 0.0 { (30.0, 30.0, incidents) } else { (sum / n, pre / n, incidents) }
    };
    let people = &w.people;
    let mut hs: Vec<super::season::TeamResult> = w
        .high_schools
        .par_iter()
        .map(|s| {
            let (strength, preseason, incidents) = team_strength(&s.roster, people);
            TeamResult { strength, preseason, incidents, roster: s.roster.len() as u32, ..Default::default() }
        })
        .collect();
    let mut college: Vec<TeamResult> = w
        .programs
        .par_iter()
        .map(|p| {
            let (strength, preseason, incidents) = team_strength(&p.roster, people);
            // Game management turns talent into wins.
            let gm = p.head_coach.map_or(0.0, |c| (w.coaches[c].ability().game_management - 10.0) * 0.12);
            TeamResult {
                strength: strength + gm,
                preseason: preseason + gm,
                incidents,
                roster: p.roster.len() as u32,
                ..Default::default()
            }
        })
        .collect();

    // --- High school games: by state and classification ----------------------------
    let mut groups: std::collections::HashMap<(usize, u32), Vec<usize>> = std::collections::HashMap::new();
    for s in w.high_schools.iter() {
        if !s.roster.is_empty() {
            groups.entry((s.state, s.classification)).or_default().push(s.id);
        }
    }
    let mut keys: Vec<(usize, u32)> = groups.keys().copied().collect();
    keys.sort();
    for key in keys {
        let teams = &groups[&key];
        play_rounds(teams, 10, 3.5, &mut hs, &mut w.rng, false);
        let champ = playoff(teams, 8, 3.5, &mut hs, &mut w.rng);
        if let Some(c) = champ {
            hs[c].state_title = true;
        }
    }

    // --- College games: conference games, then non-conference, by division --------
    for division in [Division::FBS, Division::FCS, Division::D2, Division::JUCO] {
        let members: Vec<usize> =
            w.programs.iter().filter(|p| p.division == division && !p.roster.is_empty()).map(|p| p.id).collect();
        if members.is_empty() {
            continue;
        }
        if std::env::var("AFM_DEBUG").is_ok() {
            let xs: Vec<f64> = members.iter().map(|&i| college[i].strength).collect();
            let m = xs.iter().sum::<f64>() / xs.len() as f64;
            let sd = (xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / xs.len() as f64).sqrt();
            let mut v = xs.clone();
            v.sort_by(|a, b| b.total_cmp(a));
            eprintln!("STRENGTH {:?} {} mean {m:.1} sd {sd:.2} top5 {:?}", division, w.year, &v[..5.min(v.len())]);
        }
        let mut confs: std::collections::BTreeMap<String, Vec<usize>> = std::collections::BTreeMap::new();
        for &i in &members {
            confs.entry(w.programs[i].conference.clone()).or_default().push(i);
        }
        for (name, teams) in confs.iter() {
            if name.contains("Independent") || teams.len() < 2 {
                continue;
            }
            let rounds = (teams.len() - 1).min(8);
            play_rounds(teams, rounds, COLLEGE_SPREAD, &mut college, &mut w.rng, true);
            // Conference champion: best conference record, then strength.
            let champ = *teams
                .iter()
                .max_by(|&&a, &&b| {
                    let ra = college[a].conf_wins as f64 - college[a].conf_losses as f64 + 0.01 * college[a].strength;
                    let rb = college[b].conf_wins as f64 - college[b].conf_losses as f64 + 0.01 * college[b].strength;
                    ra.total_cmp(&rb)
                })
                .unwrap();
            college[champ].conference_title = true;
        }
        // Non-conference games fill every schedule, independents included.
        let target: u32 = if division == Division::JUCO { 10 } else { 12 };
        for _ in 0..12 {
            let needy: Vec<usize> = members.iter().copied().filter(|&i| college[i].games() < target).collect();
            if needy.len() < 2 {
                break;
            }
            play_rounds(&needy, 1, COLLEGE_SPREAD, &mut college, &mut w.rng, false);
        }
        let field = match division {
            Division::FBS => 12,
            Division::FCS | Division::D2 => 16,
            Division::JUCO => 4,
        };
        // Conference champions get in first, then the best of the rest.
        // The SWAC and MEAC skip the FCS playoffs (Celebration Bowl instead).
        let mut ranked: Vec<usize> = members
            .iter()
            .copied()
            .filter(|&i| {
                !matches!(w.programs[i].conference.as_str(), "SWAC" | "MEAC" | "Mid-Eastern_Athletic_Conference")
            })
            .collect();
        ranked.sort_by(|&a, &b| {
            let ka = college[a].win_pct()
                + if college[a].conference_title { 0.15 } else { 0.0 }
                + 0.01 * college[a].strength;
            let kb = college[b].win_pct()
                + if college[b].conference_title { 0.15 } else { 0.0 }
                + 0.01 * college[b].strength;
            kb.total_cmp(&ka)
        });
        if let Some(c) = playoff(&ranked, field, COLLEGE_SPREAD, &mut college, &mut w.rng) {
            college[c].national_title = true;
        }
    }
    SeasonResults { hs, college }
}

/// How much a gap in starters' average rating decides a college game. At 3.5,
/// a team 5 points better wins about 4 in 5 (proposed, not confirmed; the
/// match engine in section 3 replaces this stand-in).
const COLLEGE_SPREAD: f64 = 3.5;

fn win_chance(a: f64, b: f64, spread: f64, home: f64) -> f64 {
    1.0 / (1.0 + (-(a - b + home) / spread).exp())
}

/// Plays `rounds` rounds of random pairings within a group.
fn play_rounds(
    teams: &[usize],
    rounds: usize,
    spread: f64,
    results: &mut [TeamResult],
    rng: &mut crate::rng::Rng,
    conference: bool,
) {
    if teams.len() < 2 {
        return;
    }
    let mut order: Vec<usize> = teams.to_vec();
    for r in 0..rounds {
        // Rotate a shuffled order so pairings change every round.
        for k in (1..order.len()).rev() {
            let j = rng.int(0, k as i32) as usize;
            order.swap(k, j);
        }
        let mut k = 0;
        while k + 1 < order.len() {
            let (a, b) = if r % 2 == 0 { (order[k], order[k + 1]) } else { (order[k + 1], order[k]) };
            let p = win_chance(results[a].strength, results[b].strength, spread, 0.8);
            let pe = win_chance(results[a].preseason, results[b].preseason, spread, 0.8);
            results[a].expected += pe;
            results[b].expected += 1.0 - pe;
            let a_wins = rng.chance(p);
            let (w, l) = if a_wins { (a, b) } else { (b, a) };
            results[w].wins += 1;
            results[l].losses += 1;
            if conference {
                results[w].conf_wins += 1;
                results[l].conf_losses += 1;
            }
            k += 2;
        }
    }
}

/// Single elimination among the top `field` teams of `ranked` (already in seed
/// order or ranked here by record). Returns the champion.
fn playoff(
    teams: &[usize],
    field: usize,
    spread: f64,
    results: &mut [TeamResult],
    rng: &mut crate::rng::Rng,
) -> Option<usize> {
    let mut seeds: Vec<usize> = teams.to_vec();
    seeds.sort_by(|&a, &b| {
        let ka =
            results[a].win_pct() + if results[a].conference_title { 0.15 } else { 0.0 } + 0.01 * results[a].strength;
        let kb =
            results[b].win_pct() + if results[b].conference_title { 0.15 } else { 0.0 } + 0.01 * results[b].strength;
        kb.total_cmp(&ka)
    });
    seeds.truncate(field.max(1));
    while seeds.len() > 1 {
        let n = seeds.len();
        let mut next = Vec::with_capacity(n.div_ceil(2));
        // Top seeds get byes when the field isn't a power of two.
        let pow = n.next_power_of_two();
        let byes = pow - n;
        next.extend_from_slice(&seeds[..byes]);
        let rest = &seeds[byes..];
        let m = rest.len();
        for k in 0..m / 2 {
            let (a, b) = (rest[k], rest[m - 1 - k]);
            let p = win_chance(results[a].strength, results[b].strength, spread, 0.4);
            let (w, l) = if rng.chance(p) { (a, b) } else { (b, a) };
            results[w].wins += 1;
            results[l].losses += 1;
            next.push(w);
        }
        seeds = next;
    }
    seeds.first().copied()
}

/// Level ratings for the season just played (note 8): "89 among seniors".
/// High schoolers are ranked against everyone their age, college players on
/// the FBS scale (FCS and small-college tape discounted), as in `level_rating`.
fn attach_level_ratings(w: &mut World) {
    use crate::level_rating::{
        LevelGroup, Pool, PositionAligner, build_level_scale, level_group, level_rating, sets_level_scale,
    };
    use std::collections::HashMap;
    type Scales = HashMap<LevelGroup, (crate::level_rating::LevelScale, PositionAligner)>;
    let build = |at_end: bool| -> Scales {
        let mut groups: HashMap<LevelGroup, Vec<(Position, f64)>> = HashMap::new();
        for p in w.people.iter() {
            let Some(s) = p.player.history.last() else { continue };
            if s.age + 1 != p.player.age || !sets_level_scale(s.level) {
                continue;
            }
            if let Some(key) = level_group(s.level, s.age) {
                groups
                    .entry(key)
                    .or_default()
                    .push((p.player.position, if at_end { s.overall } else { s.overall_start }));
            }
        }
        groups
            .into_par_iter()
            .map(|(key, samples)| {
                let align = PositionAligner::new(&samples);
                let aligned: Vec<f64> = samples.iter().map(|&(pos, x)| align.align(pos, x)).collect();
                let pool = if key == LevelGroup::College { Pool::College } else { Pool::HS };
                (key, (build_level_scale(&aligned, pool, w.options.scale), align))
            })
            .collect()
    };
    let start = build(false);
    let end = build(true);
    w.people.par_iter_mut().for_each(|p| {
        let pos = p.player.position;
        let age_now = p.player.age;
        let Some(s) = p.player.history.last_mut() else { return };
        if s.age + 1 != age_now {
            return;
        }
        let discount = match s.level {
            Level::FCS => 1.0,
            Level::SmallCollege => 5.0,
            _ => 0.0,
        };
        let key = level_group(s.level, s.age);
        let level = s.level;
        let rate = |scales: &Scales, ovr: f64| {
            let g = key.and_then(|k| scales.get(&k));
            let x = g.map_or(ovr, |(_, a)| a.align(pos, ovr - discount));
            level_rating(x, level, g.map(|g| &g.0))
        };
        s.level_rating = Some(rate(&start, s.overall_start));
        s.level_rating_end = Some(rate(&end, s.overall));
    });
}
