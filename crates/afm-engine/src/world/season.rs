//! One season at every level: depth charts, development, team strength and
//! what the match engine needs. The games themselves run on the calendar
//! (`college.rs`, `hs_year.rs`). High school games are a stand-in: a game is
//! won by comparing the two teams' starters as they played that year (form,
//! slumps and breakouts included), plus home field and luck.

use rayon::prelude::*;

use super::background;
use super::coach::{Group, TitleKind};
use super::data::Division;
use super::{Stage, World, division_level};
use crate::athlete::Eye;
use crate::attributes::Attr;
use crate::development::{SeasonContext, Trouble, develop_season};
use crate::game::playbook::{FAMILY_COUNT, Family};
use crate::game::{Caller, GamePlayer, GameTeam, Staff as GameStaff};
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
    pub totals: super::program::SeasonTotals,
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

/// A college travel squad at each position (F14): the two-deep, a third
/// quarterback, and the extra backs, receivers, linemen, linebackers and
/// defensive backs who cover kicks and step in when a starter is hurt; one
/// kicker, punter and long snapper. 76 players, scaled to the program's own
/// road limit (`travel_limit`); at home the whole roster dresses. Proposed,
/// not confirmed: the counts, set so about
/// 40-50% of first-year FBS players redshirt and 600-850 true freshmen a
/// year play five games or more (trial 6 measured about 870 before 2018,
/// when nobody was held out).
pub(crate) fn travel_slots(pos: Position, limit: usize) -> usize {
    if pos.is_specialist() {
        return 1;
    }
    let base = travel_base(pos) as f64;
    // Scaled to the program's own limit; never fewer than three quarterbacks.
    let n = (base * limit as f64 / TRAVEL_BASE_TOTAL).round() as usize;
    if pos == Position::QB { n.max(3) } else { n.max(1) }
}

/// The counts above add up to this many players.
const TRAVEL_BASE_TOTAL: f64 = 76.0;

/// A program's road travel squad by conference and era (part C 8): the
/// Power conferences 70, the Big Ten 74 from 2018 (NBC Sports, 2018), the
/// rest of FBS 70, FCS 64, Division II 55, Division III and the NAIA 55,
/// junior colleges 50. Only the Big Ten change is checked; the rest from
/// memory, to verify (proposed, not confirmed).
pub(crate) fn travel_limit(prog: &super::program::Program, season: i32) -> usize {
    match prog.division {
        Division::FBS if prog.conference == "Big Ten" && season >= 2018 => 74,
        Division::FBS => 70,
        Division::FCS => 64,
        Division::D2 | Division::D3 | Division::NAIA => 55,
        Division::JUCO => 50,
    }
}

fn travel_base(pos: Position) -> usize {
    match pos {
        Position::QB => 3,
        Position::RB => 5,
        Position::WR => 10,
        Position::TE => 4,
        Position::OT => 6,
        Position::IOL => 9,
        Position::EDGE => 7,
        Position::DT => 6,
        Position::LB => 8,
        Position::CB => 8,
        Position::S => 7,
        Position::K | Position::P | Position::LS => 1,
    }
}

/// The two-deep at a position: a starter and his backup at each spot, and
/// one kicker, punter and long snapper.
pub(crate) fn two_deep(pos: Position) -> usize {
    if pos.is_specialist() { 1 } else { 2 * depth_slots(pos).starters }
}

/// Roles on one team's depth chart, from how the staff sees each player.
/// In college the redshirt role goes only to a player with his redshirt
/// year left who falls below the travel squad (F14): a freshman in it plays
/// his season in every era. A player sitting the season out (`sits`: an
/// academic or transfer sit) takes no travel place.
pub(crate) fn depth_chart(
    entries: &mut [(usize, Position, f64)],
    travel: Option<usize>,
    redshirt_free: &dyn Fn(usize) -> bool,
    sits: &dyn Fn(usize) -> bool,
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
        // Travel places taken by the players ranked above him.
        let mut travelling = 0;
        for (k, &(i, _)) in list.iter().enumerate() {
            let role = if k < slots.starters {
                PlayingTime::Starter
            } else if k < slots.starters + slots.rotation.max(1) {
                PlayingTime::Rotation
            } else if travel.is_some_and(|t| travelling >= travel_slots(pos, t)) && redshirt_free(i) {
                PlayingTime::Redshirt
            } else {
                PlayingTime::Backup
            };
            if !sits(i) {
                travelling += 1;
            }
            out.push((i, role));
        }
    }
    out
}

/// Staffs take a fresh look before each season (topic 5.2): a boy who grew,
/// a new college staff, an athlete signed without a set position.
pub(crate) fn reconsider_positions(w: &mut World, who: &[bool]) {
    let r: &World = w;
    let moves: Vec<(usize, Position)> = r
        .people
        .par_iter()
        .enumerate()
        .filter_map(|(i, p)| {
            if !who[i] {
                return None;
            }
            let (eye, margin) = match p.stage {
                Stage::HighSchool if p.player.age > 14 => (r.hs_eye(p.hs), 0.03),
                Stage::College => {
                    let c = p.college?;
                    let prog = &r.programs[c];
                    let skill = prog.head_coach.map_or(10.0, |h| r.coaches[h].ability().head_coach_value());
                    (Eye::coach(skill).staff(0x434F_0000 ^ c as u32), 0.015)
                }
                _ => return None,
            };
            crate::athlete::better_position(&p.player, eye, margin).map(|to| (i, to))
        })
        .collect();
    if moves.is_empty() {
        return;
    }
    // A staff never empties a room to fill another (trial 7 finding 12: a
    // team whose quarterbacks were all moved to defense played one of them
    // at quarterback anyway, listed at linebacker): a player moves only
    // while his old room keeps its two-deep. Each team's rooms as they
    // stand, high schools after the colleges.
    let schools_from = w.programs.len();
    let team = |p: &super::Person| match p.stage {
        Stage::College => p.college,
        Stage::HighSchool => Some(schools_from + p.hs),
        _ => None,
    };
    let mut counted = vec![false; schools_from + w.high_schools.len()];
    for &(i, _) in &moves {
        if let Some(t) = team(&w.people[i]) {
            counted[t] = true;
        }
    }
    let mut rooms = vec![[0u32; POSITION_COUNT]; counted.len()];
    for p in w.people.iter() {
        if let Some(t) = team(p)
            && counted[t]
        {
            rooms[t][p.player.position.index()] += 1;
        }
    }
    let mut made: Vec<(usize, Position)> = Vec::with_capacity(moves.len());
    for (i, to) in moves {
        let from = w.people[i].player.position;
        let Some(t) = team(&w.people[i]) else { continue };
        let room = &mut rooms[t];
        if room[from.index()] as usize <= two_deep(from) {
            continue;
        }
        room[from.index()] -= 1;
        room[to.index()] += 1;
        made.push((i, to));
    }
    let persons = super::pick_mut(&mut w.people, made.iter().map(|m| m.0));
    persons.into_par_iter().zip(made.par_iter()).for_each(|(p, &(_, to))| {
        let from = p.player.position;
        crate::athlete::set_position(&mut p.player, to);
        let age = p.player.age;
        let by = if p.stage == Stage::College { "his college staff" } else { "his high school coach" };
        p.note(format!("Moved from {from} to {to} by {by} at {age}."));
    });
}

/// Depth charts at a set of high schools. A transfer sitting out under his
/// state's rule plays no varsity games.
pub(crate) fn hs_roles(w: &World, schools: &[usize]) -> Vec<(usize, PlayingTime)> {
    let year = w.year as u64;
    let seed = w.options.seed as u64;
    let people = &w.people;
    schools
        .par_iter()
        .map(|&h| {
            let s = &w.high_schools[h];
            let mut entries: Vec<(usize, Position, f64)> = s
                .roster
                .iter()
                .map(|&i| {
                    let p = &people[i];
                    let seen = p.player.overall() + 1.5 * key_normal(seed ^ year, p.player.id as u64);
                    (i, p.player.position, seen)
                })
                .collect();
            let mut chart = depth_chart(&mut entries, None, &|_| false, &|_| false);
            for (i, role) in chart.iter_mut() {
                let p = &people[*i];
                if p.sit_out > 0.0 && key_normal(seed ^ year ^ 7, p.player.id as u64).abs() < 1.6 * p.sit_out {
                    *role = PlayingTime::Redshirt;
                }
            }
            chart
        })
        .flatten()
        .collect()
}

/// Depth charts at every college program. Young players must beat the
/// upperclassmen ahead of them.
pub(crate) fn college_roles(w: &World) -> Vec<(usize, PlayingTime)> {
    let year = w.year as u64;
    let seed = w.options.seed as u64;
    let people = &w.people;
    w.programs
        .par_iter()
        .map(|prog| {
            let mut entries: Vec<(usize, Position, f64)> = prog
                .roster
                .iter()
                .map(|&i| {
                    let p = &people[i];
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
            depth_chart(
                &mut entries,
                Some(travel_limit(prog, year as i32)),
                &|i| {
                    let p = &people[i];
                    !p.redshirt_used && p.college_seasons <= 1
                },
                &|i| {
                    let p = &people[i];
                    p.academic_redshirt || p.cal.spring_ineligible || p.cal.transfer_sit
                },
            )
        })
        .flatten()
        .collect()
}

/// The season's development step (one checkpoint a year, `development.rs`)
/// for the people given, with their roles. High school players develop when
/// their state's season starts, college and prep players at fall camp.
pub(crate) fn develop(w: &mut World, roles: &[(usize, PlayingTime)]) {
    let year = w.year as u64;
    let n = w.people.len();
    let mut role_of: Vec<Option<PlayingTime>> = vec![None; n];
    for &(i, r) in roles {
        role_of[i] = Some(r);
    }
    // A transfer's first depth chart at his new school (trial records, 11.16).
    for (i, p) in w.people.iter_mut().enumerate() {
        if let Some(role) = role_of[i]
            && p.stage == Stage::College
            && let Some(from) = p.transfer_from.take()
            && let Some(c) = p.college
        {
            w.recruiting_log.transfer_seasons.push((w.year, from, role, w.programs[c].division));
        }
    }
    let (people, schools, programs, coaches) = (&mut w.people, &w.high_schools, &w.programs, &w.coaches);
    people.par_iter_mut().zip(role_of.par_iter()).for_each(|(p, role)| {
        let Some(role) = *role else { return };
        p.incident = false;
        p.cal.season_reset();
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
        // borderline student can lose his eligibility (checked when the
        // spring term ended, A23), less often with strong academic support.
        // Neither uses a season of eligibility.
        let mut academic_sit = false;
        if p.stage == Stage::College && trouble.is_none() {
            let prog = &programs[p.college.unwrap_or(0)];
            let support = clamp(0.3 + 0.7 * prog.facilities, 0.0, 1.0);
            if p.academic_redshirt {
                academic_sit = true;
                p.academic_redshirt = false;
            } else if p.cal.spring_ineligible {
                academic_sit = true;
                p.cal.spring_ineligible = false;
            }
            // A transfer sitting out his year under the rules of the day: a
            // year on the clock, practice only, no season of competition.
            if p.cal.transfer_sit {
                academic_sit = true;
                p.cal.transfer_sit = false;
                p.cal.sitting_transfer = true;
            }
            let effort = 0.012 * (p.player.personality.work_ethic - 10.0) + 0.06 * (support - 0.5);
            p.bg.gpa = clamp(p.bg.gpa + effort + p.rng.normal(0.0, 0.1), 0.0, 4.0);
        }
        let role = if academic_sit { PlayingTime::Redshirt } else { role };
        let ctx = SeasonContext {
            trouble,
            adjustment: p.pending_dip.take().unwrap_or(0.0),
            situation: clamp(team_situation + background::situation_push(&p.player, &p.bg), -1.0, 1.0),
            payday: std::mem::take(&mut p.cal.payday),
            ..SeasonContext::new(level, clamp(coaching, 1.0, 20.0), role)
        };
        develop_season(&mut p.player, &ctx, &mut p.rng);
        if let Some(gain) = p.cal.spring_gain_ahead.take() {
            crate::development::open_row_with_spring_step(&mut p.player, gain);
        }
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
        // An academic sit is a year on the clock without a season of
        // competition. A redshirt is decided now; whether the season counts
        // is settled when it ends, by the games he played (the 2018 rule).
        p.cal.academic_sit = academic_sit;
        p.cal.trouble = trouble.is_some();
        if p.stage == Stage::College && !academic_sit {
            if role == PlayingTime::Redshirt {
                p.redshirt_used = true;
            } else if trouble.is_none() {
                p.cal.count_pending = true;
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
}

/// A team's strength from its starters as they play this season (form,
/// slumps and breakouts included), its preseason strength, and its
/// off-field incidents.
pub(crate) fn team_strength(roster: &[usize], people: &[super::Person]) -> (f64, f64, u32) {
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
        // The rating he plays the season at: an early enrollee's row opens
        // with his first college spring step (F2), taken before the season.
        let camp =
            p.player.history.last().map_or(p.player.overall(), |h| h.overall_start + h.spring_open.unwrap_or(0.0));
        pre += camp;
        sum += p.last_performance.unwrap_or(camp);
        n += 1.0;
    }
    if n == 0.0 { (30.0, 30.0, incidents) } else { (sum / n, pre / n, incidents) }
}

/// Every program's season record so far, with its strength (game
/// management turns talent into wins).
pub(crate) fn college_results(w: &World) -> Vec<TeamResult> {
    let people = &w.people;
    w.programs
        .par_iter()
        .map(|p| {
            let (strength, preseason, incidents) = team_strength(&p.roster, people);
            let gm = p.head_coach.map_or(0.0, |c| (w.coaches[c].ability().game_management - 10.0) * 0.12);
            TeamResult {
                strength: strength + gm,
                preseason: preseason + gm,
                incidents,
                roster: p.roster.len() as u32,
                ..Default::default()
            }
        })
        .collect()
}

/// Trial tooling: each FBS starting lineup by position, rated as `strength`
/// rates it, with depth, experience, kicker and the coordinators' calling.
pub(crate) fn record_lineups(w: &mut World) {
    if !(w.options.keep_game_log && w.recording) {
        return;
    }
    for p in w.programs.iter().filter(|p| p.division == Division::FBS) {
        let mut sum = [0.0; POSITION_COUNT];
        let mut n = [0u32; POSITION_COUNT];
        for &i in &p.roster {
            let q = &w.people[i];
            if q.last_role != Some(PlayingTime::Starter) {
                continue;
            }
            let h = q.player.history.last();
            let v = q.last_performance.unwrap_or_else(|| h.map_or(q.player.overall(), |h| h.overall_start));
            sum[q.player.position.index()] += v;
            n[q.player.position.index()] += 1;
        }
        w.game_log.lineups.insert((w.year, p.id), (sum, n));
        let mut rot = (0.0_f64, 0.0_f64);
        let mut exp = (0.0_f64, 0.0_f64);
        let mut kick = 0.0_f64;
        for &i in &p.roster {
            let q = &w.people[i];
            let h = q.player.history.last();
            let v = q.last_performance.unwrap_or_else(|| h.map_or(q.player.overall(), |h| h.overall_start));
            match q.last_role {
                Some(PlayingTime::Rotation) => rot = (rot.0 + v, rot.1 + 1.0),
                Some(PlayingTime::Starter) => {
                    exp = (exp.0 + q.college_seasons as f64 + q.cal.count_pending as u32 as f64, exp.1 + 1.0);
                    if q.player.position == Position::K {
                        kick = v;
                    }
                }
                _ => {}
            }
        }
        let call = |c: Option<usize>| {
            c.or(p.head_coach).map_or(0.0, |c| {
                let a = w.coaches[c].ability();
                0.6 * a.scheme + 0.4 * a.game_management
            })
        };
        w.game_log.extras.insert(
            (w.year, p.id),
            [rot.0 / rot.1.max(1.0), rot.1, exp.0 / exp.1.max(1.0), kick, call(p.oc), call(p.dc)],
        );
    }
}

/// How much a gap in starters' average rating decides a college game. At 3.5,
/// a team 5 points better wins about 4 in 5 (proposed, not confirmed; the
/// match engine in section 3 replaces this stand-in).
pub(crate) const COLLEGE_SPREAD: f64 = 3.5;

pub(crate) fn win_chance(a: f64, b: f64, spread: f64, home: f64) -> f64 {
    1.0 / (1.0 + (-(a - b + home) / spread).exp())
}

/// Level ratings for the season just played (note 8): "89 among seniors".
/// High schoolers are ranked against everyone their age, college players on
/// the FBS scale (FCS and small-college tape discounted), as in `level_rating`.
pub(crate) fn attach_level_ratings(w: &mut World) {
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

/// About the snaps of a full college season as a starter on one side of the
/// ball: experience counts in these, so a backup's garbage-time snaps add up
/// slowly (proposed, not confirmed).
pub(crate) const SEASON_SNAPS: u32 = 750;

/// A college team for the match engine: its players and its staff's calling.
/// The systems are the ones the staff installed this spring (S3); a new
/// playbook is known only as far as it shares the old one (S5).
pub(crate) fn game_team(w: &World, prog: &super::program::Program) -> Option<GameTeam> {
    let inst = prog.installed;
    // What the playbook leans on (plays and tactics topic 4): families the
    // team runs a lot get the reps, the rest are known less well. Its own
    // tape says which; with none yet every family counts the same.
    let fam = prog.film.families();
    let fam_top = |off: bool| {
        let r = if off { 0..Family::Front as usize } else { Family::Front as usize..FAMILY_COUNT };
        fam[r].iter().copied().fold(0.0, f64::max)
    };
    let (top_off, top_def) = (fam_top(true), fam_top(false));
    let has_film = prog.film.off_snaps >= 40.0;
    let players: Vec<GamePlayer> = prog
        .roster
        .iter()
        .map(|&i| &w.people[i])
        .filter(|p| p.stage == Stage::College)
        .map(|p| {
            // This season counts as one (settled when it ends).
            let seasons = (p.college_seasons + p.cal.count_pending as u32) as f64;
            let experience = p.college_snaps as f64 / SEASON_SNAPS as f64;
            let side = if super::systems::offense(p.player.position) { inst.off_known } else { inst.def_known };
            let know = clamp(0.78 + 0.05 * seasons, 0.0, 0.95) * (0.8 + 0.2 * side);
            let mut g = GamePlayer::from_player(&p.player, know, experience);
            // Per family: reps by how much the playbook uses it, and the
            // understanding a sharp player builds faster (topic 4.2, 4.4;
            // proposed, not confirmed).
            let iq = 0.5 * (p.player.attributes[Attr::FootballIq] + p.player.attributes[Attr::PlayRecognition]);
            let mind = clamp(0.92 + 0.08 * (iq - 60.0) / 25.0, 0.82, 1.04);
            for f in Family::ALL {
                let (use_, top) = if f.offense() { (fam[f as usize], top_off) } else { (fam[f as usize], top_def) };
                let reps = if has_film && top > 0.0 { 0.78 + 0.22 * clamp(use_ / top, 0.0, 1.0).sqrt() } else { 0.92 };
                g.know[f as usize] = clamp(know * reps * mind, 0.0, 1.0) as f32;
            }
            g
        })
        .collect();
    if players.len() < 30 {
        return None;
    }
    // Line chemistry (topic 4.9): the share of the five most experienced
    // linemen who started up front here last season (proposed, not
    // confirmed).
    let mut line: Vec<&super::Person> = prog
        .roster
        .iter()
        .map(|&i| &w.people[i])
        .filter(|p| p.stage == Stage::College && matches!(p.player.position, Position::OT | Position::IOL))
        .collect();
    line.sort_by_key(|p| std::cmp::Reverse(p.college_snaps));
    let back = line
        .iter()
        .take(5)
        .filter(|p| p.last_snaps >= 350 && p.college_staff.last().is_some_and(|s| s.1 == prog.id && s.0 == w.year - 1))
        .count();
    let line_chem = 0.25 + 0.15 * back as f64;
    let ab = |c: Option<usize>| c.map(|c| *w.coaches[c].ability());
    let hc = ab(prog.head_coach);
    // A caller runs a system he knows less well worse (S4): his calling
    // skill scales with what he knows of the installed system.
    let caller = |c: Option<usize>, slot: usize| {
        let who = c.or(prog.head_coach);
        let a = ab(c).or(hc);
        let temper = who.map_or(10.0, |c| w.coaches[c].personality.ambition);
        let know = who.map_or(0.8, |c| w.coaches[c].systems.know[slot]);
        // His four calling skills (plays and tactics topic 6.2): reading
        // leans on scheme, situational on game management, creativity is
        // scheme with more of his own spread; nerve from ambition (proposed,
        // not confirmed).
        let id = who.unwrap_or(usize::MAX) as u64;
        let spread = |k: u64, sd: f64| if who.is_some() { sd * key_normal(k, id) } else { 0.0 };
        let fit = 0.7 + 0.3 * know;
        let reading =
            a.map_or(9.0, |a| clamp(0.7 * a.scheme + 0.3 * a.game_management + spread(0x5EAD, 2.0), 1.0, 20.0));
        let situational =
            a.map_or(9.0, |a| clamp(0.7 * a.game_management + 0.3 * a.scheme + spread(0x517A, 2.0), 1.0, 20.0));
        let creativity = a.map_or(9.0, |a| clamp(a.scheme + spread(0xC4EA, 3.0), 1.0, 20.0));
        let nerve = clamp(0.04 * (temper - 10.0) + spread(0x4E4E, 0.4), -1.0, 1.0);
        Caller {
            skill: a.map_or(9.0, |a| 0.6 * a.scheme + 0.4 * a.game_management) * fit,
            adaptation: a.map_or(9.0, |a| a.game_management),
            aggressiveness: temper,
            reading: reading * fit,
            situational,
            creativity: creativity * fit,
            nerve,
        }
    };
    // Film study (topic 5.1): the coordinators' eye and the quality-control
    // staff a budget buys (proposed, not confirmed).
    let eye = |c: Option<usize>| ab(c.or(prog.head_coach)).map_or(9.0, |a| a.scheme);
    let film_study = clamp(0.35 * (eye(prog.oc) + eye(prog.dc)) + 7.0 * prog.facilities, 1.0, 20.0);
    let dc = prog.dc.or(prog.head_coach);
    // The install (T6.2): the staff sizes the weekly plan to what its
    // players can absorb; a poor staff misjudges it, an aggressive one puts
    // in more (proposed, not confirmed).
    let aware = players.iter().map(|p| p.a(Attr::FootballIq)).sum::<f64>() / players.len().max(1) as f64;
    let install = {
        let oc = caller(prog.oc, inst.off);
        let who = prog.oc.or(prog.head_coach).unwrap_or(usize::MAX) as u64;
        let miss = 0.15 * (1.0 - oc.skill / 20.0).max(0.0) * key_normal(0x1257 ^ w.year as u64, who);
        clamp(crate::game::roster::absorb(aware) + miss + 0.01 * (oc.aggressiveness - 10.0), 0.35, 1.0)
    };
    let mut team = GameTeam::new(
        prog.real_name.clone(),
        players,
        GameStaff {
            offense: caller(prog.oc, inst.off),
            defense: caller(prog.dc, 7 + inst.shell),
            game_management: hc.map_or(9.0, |a| a.game_management),
            strictness: hc.map_or(10.0, |a| a.discipline),
            off_scheme: inst.offense(),
            front: inst.front(),
            shell: inst.shell(),
            playbook: 1.0,
            sportsmanship: prog.head_coach.map_or(10.0, |c| w.coaches[c].sportsmanship),
            // His own habit, kept for his career (S8; spread proposed, not
            // confirmed).
            nickel: dc.map_or(0.88, |c| clamp(0.88 + 0.08 * key_normal(0x41C8, c as u64), 0.55, 0.99)),
            year: w.year,
            film_study,
            install,
        },
    );
    team.film = prog.film.clone();
    team.line_chem = line_chem;
    team.novelty = super::trends::team_novelty(w, prog.division, &prog.film);
    Some(team)
}

/// What a head coach brings into a game besides his team (dynasty item 3):
/// the programs that fired him, and whether his job is on the line.
#[derive(Clone, Debug, Default)]
pub(crate) struct CoachContext {
    pub fired_by: Vec<usize>,
    pub hot_seat: bool,
}

pub(crate) fn coach_context(w: &World, prog: &super::program::Program) -> CoachContext {
    use super::coach::Employer;
    let Some(c) = prog.head_coach.map(|c| &w.coaches[c]) else { return CoachContext::default() };
    let fired_by = c
        .history
        .iter()
        .filter(|s| s.ended.starts_with("fired"))
        .filter_map(|s| if let Employer::College(p) = s.employer { Some(p) } else { None })
        .collect();
    // The carousel's own measure of a bad stretch (`staff.rs`), or a
    // prove-it year.
    let n = c.recent.len();
    let last = if n >= 1 { c.recent[n - 1] } else { 0.0 };
    let before = if n >= 2 { c.recent[n - 2] } else { 0.0 };
    let tenure = c.job.as_ref().map_or(0, |j| w.year - j.since);
    let hot_seat = c.prove_it.is_some() || (tenure >= 2 && -(1.5 * last + 0.7 * before) > 0.06);
    CoachContext { fired_by, hot_seat }
}

/// The situation pushes for one side of one game (proposed, not confirmed):
/// a contender in the playoff race, a grudge against the program that fired
/// him, a job on the line, a winning season in reach.
pub(crate) fn game_context(
    coaches: &[CoachContext],
    division: Division,
    results: &[TeamResult],
    me: usize,
    opp: usize,
) -> crate::game::garbage::Context {
    let r = &results[me];
    let c = coaches.get(me).cloned().unwrap_or_default();
    crate::game::garbage::Context {
        race: division == Division::FBS && r.games() >= 4 && r.losses <= 1,
        grudge: c.fired_by.contains(&opp),
        hot_seat: c.hot_seat,
        bowl_push: r.games() >= 7 && (4..=5).contains(&r.wins),
    }
}
