//! Staffs, the coaching carousel, and the yearly update of every program's
//! prestige, grades and NIL collective (topics 3, 7 and 8).

use std::collections::{BTreeMap, HashSet};

use super::coach::{
    Coach, CoachTier, Employer, Group, HiringNeeds, Job, Role, Stint, TitleKind, candidate_score, generate_coach,
    hiring_needs,
};
use super::data::{AcademicTier, Division};
use super::ethnicity::{CoachPool, Ethnicity, coach_ethnicity, hiring_bias};
use super::news::{self, Channel, NewsItem, NewsKind};
use super::program::{self, GRADE_COUNT, GradeCat, NilTrend};
use super::season::SeasonResults;
use super::{Stage, World, names};
use crate::player::PlayingTime;
use crate::rng::{Rng, clamp};

fn tier_of(d: Division) -> CoachTier {
    match d {
        Division::FBS => CoachTier::Fbs,
        Division::FCS => CoachTier::Fcs,
        Division::D2 | Division::JUCO => CoachTier::SmallCollege,
    }
}

fn new_coach(w: &mut World, tier: CoachTier, age: u32, pool: CoachPool) -> usize {
    let id = w.coaches.len();
    let ethnicity = coach_ethnicity(pool, &mut w.rng);
    let name = names::person_name(ethnicity, w.year - age as i32, &mut w.rng);
    let mut c = generate_coach(id, name, ethnicity, tier, age, &mut w.rng);
    // Sportsmanship from a keyed draw, so the world's own stream is untouched
    // (spread like the other personality traits; proposed, not confirmed).
    c.sportsmanship =
        clamp(10.0 + 3.5 * super::season::key_normal(w.options.seed as u64 ^ 0x5B0C, id as u64), 1.0, 20.0);
    w.coaches.push(c);
    id
}

/// The state a job is in (an NFL team has none here).
fn employer_state(w: &World, e: Employer) -> Option<usize> {
    match e {
        Employer::HighSchool(s) => Some(w.high_schools[s].state),
        Employer::College(p) => Some(w.programs[p].state),
        Employer::Nfl(_) => None,
    }
}

/// The NFL staff jobs the world tracks: head coach and the two coordinators.
fn nfl_roles() -> [Role; 3] {
    [Role::HeadCoach, Role::OffensiveCoordinator, Role::DefensiveCoordinator]
}

/// The coach in an NFL staff slot.
fn nfl_slot(team: &crate::league::Team, role: Role) -> Option<usize> {
    match role {
        Role::HeadCoach => team.head_coach,
        Role::OffensiveCoordinator => team.oc,
        Role::DefensiveCoordinator => team.dc,
        Role::PositionCoach(_) => None,
    }
}

fn hire(w: &mut World, coach: usize, employer: Employer, role: Role) {
    let year = w.year;
    if w.coaches[coach].home_state.is_none() {
        // His family roots (11.13): most coaches start out near home
        // (proposed, not confirmed: half); the rest grew up wherever the
        // country's players do.
        let home = match employer_state(w, employer) {
            Some(s) if w.rng.chance(0.5) => s,
            _ => {
                let shares: Vec<(usize, f64)> = w.states.iter().enumerate().map(|(i, s)| (i, s.class_share)).collect();
                w.rng.weighted(&shares)
            }
        };
        w.coaches[coach].home_state = Some(home);
    }
    w.coaches[coach].job = Some(Job { employer, role, since: year });
    w.coaches[coach].wins = 0;
    w.coaches[coach].losses = 0;
    w.coaches[coach].titles = 0;
    match employer {
        Employer::HighSchool(s) => w.high_schools[s].coach = Some(coach),
        Employer::College(p) => {
            let prog = &mut w.programs[p];
            match role {
                Role::HeadCoach => prog.head_coach = Some(coach),
                Role::OffensiveCoordinator => prog.oc = Some(coach),
                Role::DefensiveCoordinator => prog.dc = Some(coach),
                Role::PositionCoach(g) => prog.position_coaches[g as usize] = Some(coach),
            }
        }
        Employer::Nfl(t) => {
            let team = &mut w.nfl[t];
            match role {
                Role::HeadCoach => team.head_coach = Some(coach),
                Role::OffensiveCoordinator => team.oc = Some(coach),
                Role::DefensiveCoordinator => team.dc = Some(coach),
                Role::PositionCoach(_) => {}
            }
        }
    }
    // An assistant hired under a head coach joins his tree.
    let boss = match employer {
        Employer::College(p) => w.programs[p].head_coach,
        Employer::Nfl(t) => w.nfl[t].head_coach,
        Employer::HighSchool(_) => None,
    };
    if role != Role::HeadCoach
        && let Some(hc) = boss
        && w.coaches[coach].mentor.is_none()
    {
        w.coaches[coach].mentor = Some(hc);
    }
}

/// Ends a coach's current job and records the stint.
fn leave(w: &mut World, coach: usize, why: &'static str) {
    let year = w.year;
    let Some(job) = w.coaches[coach].job.take() else { return };
    let c = &mut w.coaches[coach];
    c.history.push(Stint {
        employer: job.employer,
        role: job.role,
        from: job.since,
        to: year,
        wins: c.wins,
        losses: c.losses,
        titles: c.titles,
        ended: why,
    });
    match job.employer {
        Employer::HighSchool(s) => {
            if w.high_schools[s].coach == Some(coach) {
                w.high_schools[s].coach = None;
            }
        }
        Employer::College(p) => {
            let prog = &mut w.programs[p];
            for slot in
                [&mut prog.head_coach, &mut prog.oc, &mut prog.dc].into_iter().chain(prog.position_coaches.iter_mut())
            {
                if *slot == Some(coach) {
                    *slot = None;
                }
            }
        }
        Employer::Nfl(t) => {
            let team = &mut w.nfl[t];
            for slot in [&mut team.head_coach, &mut team.oc, &mut team.dc] {
                if *slot == Some(coach) {
                    *slot = None;
                }
            }
        }
    }
}

/// Every college staff job: head coach, coordinators and the position coaches.
pub(crate) fn college_roles() -> impl Iterator<Item = Role> {
    [Role::HeadCoach, Role::OffensiveCoordinator, Role::DefensiveCoordinator]
        .into_iter()
        .chain(Group::ALL.into_iter().map(Role::PositionCoach))
}

/// The coach in a college staff slot.
pub(crate) fn slot(prog: &program::Program, role: Role) -> Option<usize> {
    match role {
        Role::HeadCoach => prog.head_coach,
        Role::OffensiveCoordinator => prog.oc,
        Role::DefensiveCoordinator => prog.dc,
        Role::PositionCoach(g) => prog.position_coaches[g as usize],
    }
}

/// Fills every job when the world is created.
pub(crate) fn staff_everyone(w: &mut World) {
    for p in 0..w.programs.len() {
        let tier = tier_of(w.programs[p].division);
        for role in college_roles() {
            let (lo, pool) = match role {
                Role::HeadCoach => (38, CoachPool::CollegeHead1990),
                Role::PositionCoach(_) => (26, CoachPool::CollegeAssistant1990),
                _ => (30, CoachPool::CollegeAssistant1990),
            };
            let age = w.rng.int(lo, 62) as u32;
            let c = new_coach(w, tier, age, pool);
            if let Role::PositionCoach(_) = role {
                // Position coaches sit a rung below coordinators.
                let coach = &mut w.coaches[c];
                coach.reputation = clamp(coach.reputation - 1.5, 1.0, 20.0);
            }
            hire(w, c, Employer::College(p), role);
            w.coaches[c].job.as_mut().unwrap().since = w.year - w.rng.int(0, 6);
        }
    }
    for s in 0..w.high_schools.len() {
        let age = w.rng.int(28, 62) as u32;
        let c = new_coach(w, CoachTier::HighSchool, age, CoachPool::HighSchool);
        // Powerhouses already have strong staffs.
        if w.high_schools[s].prestige > 0.75 {
            for t in w.coaches[c].ability.teaching.iter_mut() {
                *t = clamp(*t + 3.0, 1.0, 20.0);
            }
            w.coaches[c].reputation = clamp(w.coaches[c].reputation + 4.0, 1.0, 20.0);
        }
        hire(w, c, Employer::HighSchool(s), Role::HeadCoach);
    }
    // NFL staffs: a head coach and two coordinators per team (11.12).
    for t in 0..w.nfl.len() {
        for role in nfl_roles() {
            let (lo, pool) = match role {
                Role::HeadCoach => (38, CoachPool::CollegeHead1990),
                _ => (32, CoachPool::CollegeAssistant1990),
            };
            let age = w.rng.int(lo, 64) as u32;
            let c = new_coach(w, CoachTier::Nfl, age, pool);
            hire(w, c, Employer::Nfl(t), role);
            w.coaches[c].job.as_mut().unwrap().since = w.year - w.rng.int(0, 6);
        }
    }
    set_nfl_coaching(w);
}

/// How well a coach runs one side of the ball, 1-20: his teaching there and his scheme.
fn side_quality(c: &Coach, offense: bool) -> f64 {
    let a = c.ability();
    let groups: Vec<f64> = Group::ALL
        .iter()
        .filter(|g| g.offense() == offense && **g != Group::Specialists)
        .map(|&g| a.teaching[g as usize])
        .collect();
    0.7 * groups.iter().sum::<f64>() / groups.len() as f64 + 0.3 * a.scheme
}

/// The pro league's staff rating for each world team, 1-20, from its named
/// coaches (11.12): the head coach counts half, each coordinator a quarter on
/// his side of the ball (proposed, not confirmed).
fn set_nfl_coaching(w: &mut World) {
    let year = w.year;
    for t in 0..w.nfl.len() {
        let team = &w.nfl[t];
        let Some(h) = team.head_coach else { continue };
        let side = |c: Option<usize>, off: bool| c.map_or(11.0, |c| side_quality(&w.coaches[c], off));
        let q =
            0.5 * w.coaches[h].ability().head_coach_value() + 0.25 * side(team.oc, true) + 0.25 * side(team.dc, false);
        let since = w.coaches[h].job.map_or(year, |j| j.since);
        w.nfl[t].coaching = clamp(q, 1.0, 20.0);
        w.nfl[t].staff_years = (year - since).max(0) as u32;
    }
}

/// One NFL season for the coaches (11.12). The league's games aren't played,
/// so each team's record comes from its roster, its staff and luck. Real NFL
/// win shares spread about 0.19 a season, about 0.12 of it luck (proposed,
/// not confirmed: 0.13 per roster sd, 0.06 per staff sd). The playoff
/// bracket isn't played either: better teams are likelier to come through it.
fn nfl_season(w: &mut World) {
    let year = w.year;
    let n = w.nfl.len();
    if n == 0 {
        return;
    }
    let strength: Vec<f64> = w.nfl.iter().map(|t| t.strength()).collect();
    let mean = strength.iter().sum::<f64>() / n as f64;
    let sd = (strength.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / n as f64).sqrt().max(0.5);
    let games = if year >= 2021 { 17 } else { 16 };
    let mut expect = vec![0.0; n];
    let mut raw = vec![0.0; n];
    for t in 0..n {
        expect[t] = 0.5 + 0.13 * (strength[t] - mean) / sd;
        let staff = (w.nfl[t].coaching - 13.0) / 2.5;
        raw[t] = expect[t] + 0.06 * staff + w.rng.normal(0.0, 0.12);
    }
    // Every game has a winner: the league's win shares average one half.
    let shift = 0.5 - raw.iter().sum::<f64>() / n as f64;
    let wins: Vec<u32> = raw.iter().map(|r| (clamp(r + shift, 0.0, 1.0) * games as f64).round() as u32).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| wins[b].cmp(&wins[a]).then(a.cmp(&b)));
    let spots = if year >= 2020 { 14 } else { 12 };
    let seeds: Vec<(usize, f64)> =
        order.iter().take(spots).map(|&t| (t, (8.0 * (wins[t] as f64 / games as f64 - 0.5)).exp())).collect();
    let champ = w.rng.weighted(&seeds);
    let rest: Vec<(usize, f64)> = seeds.iter().copied().filter(|s| s.0 != champ).collect();
    let runner_up = w.rng.weighted(&rest);
    for t in 0..n {
        let share = wins[t] as f64 / games as f64;
        let vs = share - expect[t];
        let title = if t == champ {
            TitleKind::National
        } else if t == runner_up {
            TitleKind::Conference
        } else {
            TitleKind::None
        };
        let team = &mut w.nfl[t];
        team.win_share.push(share);
        if team.win_share.len() > 3 {
            team.win_share.remove(0);
        }
        let (hc, oc, dc) = (team.head_coach, team.oc, team.dc);
        if let Some(h) = hc {
            let coach = &mut w.coaches[h];
            coach.wins += wins[t];
            coach.losses += games - wins[t];
            if title != TitleKind::None {
                coach.titles += 1;
                coach.career_titles += 1;
            }
            coach.update_reputation(vs, title, 0.0, true, 1.0);
        }
        // A coordinator is judged on his unit: the team's year and his side's own form.
        for (c, off) in [(oc, true), (dc, false)] {
            let Some(c) = c else { continue };
            let unit = 0.03 * (side_quality(&w.coaches[c], off) - 13.5) / 2.0 + w.rng.normal(0.0, 0.04);
            w.coaches[c].update_reputation(0.5 * vs + unit, title, 0.0, false, 1.0);
        }
        if title == TitleKind::National && w.recording {
            let name = w.nfl[t].name;
            let coach = hc.map_or(String::new(), |h| format!(" under head coach {}", w.coaches[h].name));
            w.news.push(NewsItem {
                year,
                kind: NewsKind::NationalTitle,
                channel: Channel::News,
                program: None,
                text: format!("The {name} win the Super Bowl{coach}."),
            });
        }
    }
}

/// Records the season: titles, records, news, and every coach's reputation.
pub(crate) fn after_season(w: &mut World, r: &SeasonResults) {
    let year = w.year;
    // The spin-up only fills rosters; history starts when recording does.
    if !w.recording {
        return;
    }
    for s in 0..w.high_schools.len() {
        let t = &r.hs[s];
        if t.games() == 0 {
            continue;
        }
        let school = &mut w.high_schools[s];
        school.seasons.push(super::high_school::HsSeason {
            year,
            wins: t.wins,
            losses: t.losses,
            state_title: t.state_title,
            coach: school.coach,
        });
        if t.state_title {
            school.state_titles += 1;
        }
        if let Some(c) = school.coach {
            let coach = &mut w.coaches[c];
            coach.wins += t.wins;
            coach.losses += t.losses;
            if t.state_title {
                coach.titles += 1;
                coach.career_titles += 1;
            }
            let locker = clamp(t.incidents as f64 / t.roster.max(1) as f64 * 8.0, 0.0, 1.0);
            let stage = w.high_schools[s].prestige;
            coach.update_reputation(t.vs_expectation(), t.best_title(), locker, true, stage);
        }
    }
    for p in 0..w.programs.len() {
        let t = &r.college[p];
        if t.games() == 0 {
            continue;
        }
        let prog = &mut w.programs[p];
        prog.seasons.push(program::ProgramSeason {
            year,
            wins: t.wins,
            losses: t.losses,
            conference_title: t.conference_title,
            national_title: t.national_title,
            coach: prog.head_coach,
            strength: t.strength,
            totals: t.totals,
        });
        if t.conference_title {
            prog.conference_titles += 1;
        }
        if t.national_title {
            prog.titles += 1;
        }
        if w.recording && t.national_title {
            let name = prog.name(w.options.names).to_string();
            w.news.push(NewsItem {
                year,
                kind: NewsKind::NationalTitle,
                channel: Channel::News,
                program: Some(p),
                text: format!("{name} win the {} national title.", prog.division.name()),
            });
        }
        let locker = clamp(t.incidents as f64 / t.roster.max(1) as f64 * 8.0, 0.0, 1.0);
        let stage = prog.prestige_rel;
        let staff: Vec<(Option<usize>, bool)> = [(prog.head_coach, true), (prog.oc, false), (prog.dc, false)]
            .into_iter()
            .chain(prog.position_coaches.iter().map(|&c| (c, false)))
            .collect();
        for (c, head) in staff {
            let Some(c) = c else { continue };
            let coach = &mut w.coaches[c];
            if head {
                coach.wins += t.wins;
                coach.losses += t.losses;
                if t.best_title() != TitleKind::None {
                    coach.titles += 1;
                    coach.career_titles += 1;
                }
            }
            coach.update_reputation(t.vs_expectation(), t.best_title(), locker, head, stage);
        }
    }
    nfl_season(w);
}

#[derive(Clone, Copy, Debug)]
struct Opening {
    employer: Employer,
    role: Role,
    prestige: f64,
}

fn employer_prestige(w: &World, e: Employer) -> f64 {
    match e {
        Employer::HighSchool(s) => w.high_schools[s].prestige * 0.35,
        Employer::College(p) => {
            let base = match w.programs[p].division {
                Division::FBS => 0.55,
                Division::FCS => 0.35,
                Division::D2 => 0.25,
                Division::JUCO => 0.2,
            };
            base + 0.45 * w.programs[p].prestige
        }
        Employer::Nfl(_) => 1.0,
    }
}

/// Rung bonus of a role on the coaching ladder, for comparing jobs.
fn rung(role: Role) -> f64 {
    match role {
        Role::HeadCoach => 0.1,
        Role::OffensiveCoordinator | Role::DefensiveCoordinator => 0.05,
        Role::PositionCoach(_) => 0.0,
    }
}

/// A loyal coach who won here (note 1): five or more seasons, no history of
/// leaving for other jobs (or a loyal nature), and a title in his last eight
/// seasons here or a long winning record.
fn earned_patience(w: &World, c: usize, p: usize, tenure: i32) -> bool {
    let coach = &w.coaches[c];
    if tenure < 5 {
        return false;
    }
    let chased = coach.history.iter().filter(|s| s.ended == "left for another job").count();
    let loyal = chased == 0 || (chased == 1 && coach.personality.loyalty >= 13.0);
    let recent: Vec<_> = w.programs[p].seasons.iter().rev().take(8).filter(|s| s.coach == Some(c)).collect();
    let title = recent.iter().any(|s| s.conference_title || s.national_title);
    let games = coach.wins + coach.losses;
    let winning = games >= 50 && coach.wins as f64 / games as f64 >= 0.7;
    loyal && (title || winning)
}

/// How good the coach's excuse is for the bad stretch, 0-1: the roster lost
/// many players to the draft, or the depth chart is young. (Injuries to a star
/// quarterback count once section 3 models injuries.)
fn excuse(w: &World, p: usize) -> f64 {
    let year = w.year;
    let picks = w.programs[p].draft_picks.iter().filter(|(y, _)| *y == year).count() as f64;
    let (mut young, mut starters) = (0.0, 0.0);
    for &i in &w.programs[p].roster {
        let person = &w.people[i];
        if person.last_role == Some(PlayingTime::Starter) {
            starters += 1.0;
            if person.college_seasons <= 2 {
                young += 1.0;
            }
        }
    }
    let young_share = if starters > 0.0 { young / starters } else { 0.0 };
    clamp(((picks - 3.0) / 5.0).max((young_share - 0.35) / 0.3), 0.0, 1.0)
}

fn college_news(w: &mut World, p: usize, kind: NewsKind, text: String) {
    if w.recording {
        w.news.push(NewsItem { year: w.year, kind, channel: Channel::News, program: Some(p), text });
    }
}

/// A high school coach so dominant that colleges notice (note 5): a state
/// title in one of his last two seasons and almost no losses. Only his own
/// seasons count, never his predecessor's.
fn hs_buzz(w: &World, c: usize) -> bool {
    let coach = &w.coaches[c];
    let Some(Job { employer: Employer::HighSchool(s), .. }) = coach.job else { return false };
    let seasons: Vec<_> = w.high_schools[s].seasons.iter().rev().take(2).filter(|x| x.coach == Some(c)).collect();
    let title = seasons.iter().any(|x| x.state_title);
    let (wins, games) = seasons.iter().fold((0, 0), |a, x| (a.0 + x.wins, a.1 + x.wins + x.losses));
    title && games > 0 && wins as f64 / games as f64 >= 0.9
}

/// What a job pays a year, in today's dollars (11.13). Claude's reading of
/// 2024-25 pay (USA Today's college salary database, NFL reporting), proposed,
/// not confirmed: each level's range runs from its smallest program to its
/// biggest, by prestige. NFL teams pay by owner, and the NFL paid about twice
/// what college did in the 1990s, until top college pay caught up by the
/// mid-2010s.
fn salary(w: &World, e: Employer, role: Role) -> f64 {
    let pick = |r: [(f64, f64); 3]| match role {
        Role::HeadCoach => r[0],
        Role::OffensiveCoordinator | Role::DefensiveCoordinator => r[1],
        Role::PositionCoach(_) => r[2],
    };
    let ((lo, hi), x, era) = match e {
        Employer::HighSchool(s) => ((0.05e6, 0.16e6), w.high_schools[s].prestige, 1.0),
        Employer::College(p) => {
            let range = match w.programs[p].division {
                Division::FBS => [(0.6e6, 12.0e6), (0.25e6, 2.5e6), (0.15e6, 1.0e6)],
                Division::FCS => [(0.15e6, 0.7e6), (0.08e6, 0.25e6), (0.05e6, 0.12e6)],
                Division::D2 => [(0.07e6, 0.2e6), (0.045e6, 0.09e6), (0.035e6, 0.06e6)],
                Division::JUCO => [(0.05e6, 0.12e6), (0.04e6, 0.07e6), (0.03e6, 0.05e6)],
            };
            (pick(range), w.programs[p].prestige, 1.0)
        }
        Employer::Nfl(t) => {
            let owner = (w.nfl[t].owner - 1.0) / 19.0;
            let era = clamp(1.0 + 0.035 * (2024 - w.year) as f64, 1.0, 2.0);
            (pick([(5.0e6, 14.0e6), (1.2e6, 4.0e6), (0.5e6, 1.2e6)]), owner, era)
        }
    };
    lo * (hi / lo).powf(clamp(x, 0.0, 1.0)) * era
}

/// How hot a name is (11.11): what a search committee and the press can see.
/// His reputation with recency bias, the size of his stage, titles in his
/// last three seasons there, and a program rising under him (proposed, not
/// confirmed: +3 for the biggest stage, +2 a conference title, +4 a national
/// title, +1 per 0.01 of prestige gained in three years, -1 to +3).
fn hotness(w: &World, c: usize) -> f64 {
    let coach = &w.coaches[c];
    let mut h = coach.hiring_buzz();
    let Some(job) = coach.job else { return h };
    h += 3.0 * employer_prestige(w, job.employer);
    match job.employer {
        Employer::College(p) => {
            let prog = &w.programs[p];
            for s in prog.seasons.iter().rev().take(3).filter(|s| s.year > job.since) {
                if s.national_title {
                    h += 4.0;
                } else if s.conference_title {
                    h += 2.0;
                }
            }
            let hist = &prog.prestige_history;
            if hist.len() >= 4 && hist[hist.len() - 4].0 >= job.since {
                let rise = hist[hist.len() - 1].1 - hist[hist.len() - 4].1;
                h += clamp(100.0 * rise, -1.0, 3.0);
            }
        }
        Employer::Nfl(t) => {
            // A coordinator on a winning team, or one who reached the Super Bowl.
            h += 6.0 * (w.nfl[t].win_share.last().copied().unwrap_or(0.5) - 0.5);
            if coach.titles > 0 {
                h += 2.0;
            }
        }
        Employer::HighSchool(_) => {}
    }
    h
}

/// The `k` hottest names on a list, hottest first.
fn hottest(w: &World, list: &[usize], k: usize) -> Vec<usize> {
    if k == 0 {
        return Vec::new();
    }
    let mut ranked: Vec<(f64, usize)> = list.iter().map(|&c| (hotness(w, c), c)).collect();
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    ranked.into_iter().take(k).map(|x| x.1).collect()
}

/// The roster a head coach would leave (11.13), 0-1: 1 when a young core of
/// starters is coming back, 0 when the starters are mostly leaving.
fn young_core(w: &World, p: usize) -> f64 {
    let (mut young, mut starters) = (0.0, 0.0);
    for &i in &w.programs[p].roster {
        let person = &w.people[i];
        if person.last_role == Some(PlayingTime::Starter) {
            starters += 1.0;
            if person.college_seasons <= 2 {
                young += 1.0;
            }
        }
    }
    if starters == 0.0 { 0.0 } else { clamp((young / starters - 0.3) / 0.4, 0.0, 1.0) }
}

/// A short name for an employer in a story line.
fn employer_title(w: &World, e: Employer) -> String {
    match e {
        Employer::HighSchool(h) => format!("{} High School", w.high_schools[h].name),
        Employer::College(p) => w.programs[p].name(w.options.names).to_string(),
        Employer::Nfl(t) => format!("the {}", w.nfl[t].name),
    }
}

/// Whether he has coached in the NFL (or, with `nfl` false, in college).
fn has_coached(c: &Coach, nfl: bool) -> bool {
    let at = |e: Employer| if nfl { matches!(e, Employer::Nfl(_)) } else { matches!(e, Employer::College(_)) };
    c.history.iter().any(|s| at(s.employer)) || c.job.is_some_and(|j| at(j.employer))
}

/// A coach weighs an offer (11.13): the job itself, the money, the NFL pull,
/// the roster he would leave, his age, his roots and his seat. Each part is
/// noteworthy and none decides alone. The move has to clear what it costs
/// to uproot (0.03, the old bar), then his ambition and loyalty decide.
/// Returns why he takes it, in plain words, or None.
///
/// Sizes (proposed, not confirmed), in prestige points: doubling his pay is
/// worth 0.035; a first head job 0.12 (0.18 for the most ambitious); the NFL
/// +0.04 for the most ambitious, -0.04 for the least, and a gifted recruiter
/// gives up his edge (-0.02); a young core coming back -0.03; -0.003 a year
/// of age past 52 and -0.0015 a season settled (to 12); going home +0.03,
/// leaving home -0.02; a hot seat up to +0.03.
fn weigh_offer(w: &mut World, c: usize, o: Opening) -> Option<String> {
    let year = w.year;
    let coach = &w.coaches[c];
    let Some(job) = coach.job else {
        // Out of work: he takes it.
        return Some(coach.history.last().map_or(String::new(), |s| {
            let how = if s.ended.starts_with("fired") { "fired" } else { "let go" };
            format!("out of work since he was {how}")
        }));
    };
    let a = (coach.personality.ambition - 10.0) / 10.0;
    let mut parts: Vec<(f64, String)> = Vec::new();
    // The job: a bigger program, or a step up the ladder.
    let been_head = coach
        .history
        .iter()
        .map(|s| (s.employer, s.role))
        .chain(std::iter::once((job.employer, job.role)))
        .any(|(e, r)| r == Role::HeadCoach && !matches!(e, Employer::HighSchool(_)));
    let first_head = o.role == Role::HeadCoach && !been_head && !matches!(o.employer, Employer::HighSchool(_));
    let gain = o.prestige + rung(o.role) - employer_prestige(w, job.employer) - rung(job.role);
    let to_nfl = matches!(o.employer, Employer::Nfl(_));
    let from_nfl = matches!(job.employer, Employer::Nfl(_));
    let label = if to_nfl && !from_nfl {
        "the step up to the NFL".to_string()
    } else if o.prestige > employer_prestige(w, job.employer) + 0.02 {
        "a bigger program".to_string()
    } else if rung(o.role) > rung(job.role) && !first_head {
        format!("a step up to {}", o.role.name())
    } else {
        String::new()
    };
    parts.push((gain, label));
    if first_head {
        parts.push((0.12 * (1.0 + 0.5 * a), "his first head job".to_string()));
    }
    // The money: a winner has had raises to keep him (10% a title, to three).
    let pay_now = salary(w, job.employer, job.role) * (1.0 + 0.1 * coach.titles.min(3) as f64);
    let pay = salary(w, o.employer, o.role);
    parts.push((0.05 * (pay / pay_now).ln(), "a big raise".to_string()));
    // The NFL: the top of the profession for the ambitious. A gifted
    // recruiter gives up his edge there; an NFL man who hates recruiting stays.
    let recruiter = (coach.ability().recruiting - 10.0) / 10.0;
    if to_nfl && !from_nfl {
        parts.push((0.04 * a - 0.02 * recruiter, "the NFL, where he always wanted to coach".to_string()));
    } else if from_nfl && !to_nfl {
        parts.push((0.02 * recruiter, "a return to the college game".to_string()));
    }
    // The roster he leaves: a young core coming back keeps a head coach.
    if job.role == Role::HeadCoach
        && let Employer::College(p) = job.employer
    {
        parts.push((-0.03 * young_core(w, p), String::new()));
    }
    // An older coach and a settled family move less.
    let tenure = (year - job.since).clamp(0, 12) as f64;
    parts.push((-0.003 * (coach.age as f64 - 52.0).max(0.0) - 0.0015 * tenure, String::new()));
    // Family roots: going home pulls; leaving home costs.
    let (here, there) = (employer_state(w, job.employer), employer_state(w, o.employer));
    if let Some(home) = coach.home_state {
        if there == Some(home) && here != Some(home) {
            parts.push((0.03, format!("a chance to go home to {}", w.states[home].code)));
        } else if here == Some(home) && there != Some(home) {
            parts.push((-0.02, String::new()));
        }
    }
    // A seat getting hot: a way out before he is pushed.
    if job.role == Role::HeadCoach {
        let n = coach.recent.len();
        let last = if n >= 1 { coach.recent[n - 1] } else { 0.0 };
        let before = if n >= 2 { coach.recent[n - 2] } else { 0.0 };
        let heat = clamp(-(1.5 * last + 0.7 * before) / 0.3, 0.0, 1.0);
        parts.push((0.03 * heat, "a way out before his seat got hot".to_string()));
    }
    let pull: f64 = parts.iter().map(|p| p.0).sum();
    if pull <= 0.03 {
        return None;
    }
    let p = &coach.personality;
    let chance = clamp(0.35 + 0.04 * (p.ambition - 10.0) - 0.03 * (p.loyalty - 10.0) + 1.5 * pull, 0.05, 0.95);
    if !w.rng.chance(chance) {
        return None;
    }
    // Why, in plain words: the two biggest pulls.
    parts.sort_by(|x, y| y.0.total_cmp(&x.0));
    let why: Vec<String> = parts.into_iter().filter(|x| x.0 > 0.01 && !x.1.is_empty()).take(2).map(|x| x.1).collect();
    Some(if why.is_empty() { "a fresh start".to_string() } else { why.join(" and ") })
}

/// A coach under 60 who steps away (11.15): his health, likelier with age,
/// or burnout, likelier the longer he has carried a high-pressure job and the
/// more intense he is (proposed, not confirmed: health 0.08% a year at 40,
/// 0.55% at 59; burnout 0.8% a year after ten seasons in the biggest head
/// jobs, a coordinator carrying 40% of that pressure).
fn early_exit(w: &mut World, c: usize, job: Job) -> Option<&'static str> {
    let coach = &w.coaches[c];
    let age = coach.age as f64;
    let health = 0.0008 + 0.00025 * (age - 40.0).max(0.0);
    let pressure = employer_prestige(w, job.employer) * if job.role == Role::HeadCoach { 1.0 } else { 0.4 };
    let intensity = 1.0 + 0.04 * (coach.personality.temper - 10.0);
    let tenure = (w.year - job.since).clamp(0, 15) as f64;
    let burnout = 0.008 * pressure * pressure * (tenure / 10.0) * intensity;
    let roll = w.rng.next();
    if roll < health {
        Some("retired for his health")
    } else if roll < health + burnout {
        Some("stepped away, burned out")
    } else {
        None
    }
}

fn seasons_text(n: i32) -> String {
    if n == 1 { "1 season".to_string() } else { format!("{n} seasons") }
}

/// The story line when a head coach retires or steps away.
fn retirement_news(w: &mut World, c: usize, job: Job, why: &str) {
    if !w.recording || job.role != Role::HeadCoach {
        return;
    }
    let year = w.year;
    let (name, age, seasons) = (w.coaches[c].name.clone(), w.coaches[c].age, seasons_text(year - job.since));
    let at = employer_title(w, job.employer);
    let text = match why {
        "retired for his health" => {
            format!("Head coach {name} steps down at {at} at {age} for health reasons, after {seasons}.")
        }
        "stepped away, burned out" => {
            format!("Head coach {name} steps away from coaching at {age}, worn out after {seasons} at {at}.")
        }
        _ => format!("Head coach {name} retires at {age} after {seasons} at {at}."),
    };
    match job.employer {
        Employer::College(p) => {
            w.programs[p].story.push((year, format!("Head coach {name} {why} at {age}.")));
            college_news(w, p, NewsKind::CoachLeft, text);
        }
        Employer::Nfl(_) => {
            w.news.push(NewsItem { year, kind: NewsKind::CoachLeft, channel: Channel::News, program: None, text })
        }
        Employer::HighSchool(_) => {}
    }
}

/// The story line when a head coach leaves for another job, with why.
fn departure_news(w: &mut World, c: usize, job: Job, o: Opening, why: &str) {
    if !w.recording || job.role != Role::HeadCoach {
        return;
    }
    let year = w.year;
    let name = w.coaches[c].name.clone();
    let dest = employer_title(w, o.employer);
    let role = if o.role == Role::HeadCoach { String::new() } else { format!(" as {}", o.role.name()) };
    let seasons = seasons_text(year - job.since);
    let at = employer_title(w, job.employer);
    let text = format!("Head coach {name} leaves {at} for {dest}{role} after {seasons} ({why}).");
    match job.employer {
        Employer::College(p) => {
            w.programs[p].story.push((year, format!("Head coach {name} left for {dest}{role} ({why}).")));
            college_news(w, p, NewsKind::CoachLeft, text);
        }
        Employer::Nfl(_) => {
            w.news.push(NewsItem { year, kind: NewsKind::CoachLeft, channel: Channel::News, program: None, text })
        }
        Employer::HighSchool(_) => {}
    }
}

/// NFL owners judge wins (11.12): a losing season or two costs most head
/// coaches their job by the third year, a coach who beat what his roster
/// should do gets some slack, and a patient owner waits longer. A coach who
/// keeps his job often fires a coordinator after a losing year. Proposed, not
/// confirmed; sized for about 6-8 openings a year (real 2015-2025 about 7).
fn nfl_firing(w: &mut World, c: usize, t: usize, tenure: i32, openings: &mut Vec<Opening>) {
    let ws = &w.nfl[t].win_share;
    let n = ws.len();
    let last = if n >= 1 { ws[n - 1] } else { 0.5 };
    let before = if n >= 2 { ws[n - 2] } else { 0.5 };
    let vs = w.coaches[c].recent.last().copied().unwrap_or(0.0);
    let badness = 1.6 * (0.5 - last) + 0.6 * (0.5 - before) - 0.5 * vs;
    let patience = clamp(1.0 - 0.03 * (w.nfl[t].owner - 10.0), 0.6, 1.4);
    let p = clamp((badness - 0.1) * 3.2 * patience, 0.0, 0.9)
        * match tenure {
            0 => 0.0,
            1 => 0.3,
            2 => 0.7,
            _ => 1.0,
        };
    if w.rng.chance(p) {
        leave(w, c, "fired");
        w.coaches[c].recent.clear();
        openings.push(Opening { employer: Employer::Nfl(t), role: Role::HeadCoach, prestige: 1.0 });
        if w.recording {
            let games = if w.year >= 2021 { 17.0 } else { 16.0 };
            let wins = (last * games).round();
            let text = format!(
                "The {} fire head coach {} after {}, the last one {wins}-{}.",
                w.nfl[t].name,
                w.coaches[c].name,
                seasons_text(tenure),
                games - wins
            );
            w.news.push(NewsItem {
                year: w.year,
                kind: NewsKind::CoachFired,
                channel: Channel::News,
                program: None,
                text,
            });
        }
        // A new head coach brings his own coordinators (proposed: each kept 30% of the time).
        for role in [Role::OffensiveCoordinator, Role::DefensiveCoordinator] {
            if let Some(a) = nfl_slot(&w.nfl[t], role)
                && !w.rng.chance(0.3)
            {
                leave(w, a, "let go by the new staff");
                openings.push(Opening { employer: Employer::Nfl(t), role, prestige: 1.0 });
            }
        }
    } else {
        for role in [Role::OffensiveCoordinator, Role::DefensiveCoordinator] {
            let p = clamp((0.42 - last) * 1.5, 0.0, 0.35);
            if let Some(a) = nfl_slot(&w.nfl[t], role)
                && w.rng.chance(p)
            {
                leave(w, a, "fired");
                openings.push(Opening { employer: Employer::Nfl(t), role, prestige: 1.0 });
            }
        }
    }
}

/// The coaching carousel: firings (with recency bias), "prove it" extensions,
/// retirements, and hiring in order of prestige, where every move opens
/// another job. Coaches climb a ladder: high school, position coach,
/// coordinator, head coach (note 5).
pub(crate) fn carousel(w: &mut World) {
    let year = w.year;
    let mut openings: Vec<Opening> = Vec::new();
    // --- Retirements, firings and extensions -------------------------------------
    for c in 0..w.coaches.len() {
        let Some(job) = w.coaches[c].job else { continue };
        let age = w.coaches[c].age;
        let retire = age >= 60 && w.rng.chance(clamp(0.04 + 0.03 * (age as f64 - 60.0), 0.0, 1.0));
        // Before 60, his health or burnout (11.15).
        let early = if age < 60 { early_exit(w, c, job) } else { None };
        if retire || early.is_some() {
            let why = early.unwrap_or("retired");
            leave(w, c, why);
            w.coaches[c].retired = true;
            let prestige = employer_prestige(w, job.employer);
            openings.push(Opening { employer: job.employer, role: job.role, prestige });
            retirement_news(w, c, job, why);
            continue;
        }
        if job.role != Role::HeadCoach {
            continue;
        }
        let tenure = year - job.since;
        if let Employer::Nfl(t) = job.employer {
            nfl_firing(w, c, t, tenure, &mut openings);
            continue;
        }
        let coach = &w.coaches[c];
        let n = coach.recent.len();
        let last = if n >= 1 { coach.recent[n - 1] } else { 0.0 };
        let before = if n >= 2 { coach.recent[n - 2] } else { 0.0 };
        // Recency bias: one bad stretch outweighs a long good record.
        let mut badness = -(1.5 * last + 0.7 * before);
        // Losing is losing: boosters don't grade on the roster he inherited.
        if let Employer::College(p) = job.employer
            && tenure >= 3
        {
            let s = &w.programs[p].seasons;
            let recent: Vec<_> = s.iter().rev().take(2).collect();
            let games: u32 = recent.iter().map(|s| s.wins + s.losses).sum();
            if games > 0 {
                let wp = recent.iter().map(|s| s.wins).sum::<u32>() as f64 / games as f64;
                badness += 0.8 * (0.4 - wp).max(0.0);
            }
        }
        let (prestige, hs) = match job.employer {
            Employer::HighSchool(s) => (w.high_schools[s].prestige, true),
            Employer::College(p) => (w.programs[p].prestige, false),
            Employer::Nfl(_) => (0.5, false),
        };
        let mut p_fire = clamp((badness - 0.06) * (2.0 + 4.0 * prestige), 0.0, 0.85);
        // Most get three or four seasons; a few are gone after one bad stretch.
        p_fire *= match tenure {
            0 | 1 => 0.1,
            2 => 0.4,
            3 => 0.75,
            _ => 1.0,
        };
        if hs {
            p_fire *= 0.5;
        }
        // A "prove it" year: a third bad season ends it; a good one clears it.
        let mut fire = false;
        let mut why = "fired";
        match w.coaches[c].prove_it {
            Some(granted) if granted == year - 1 => {
                // A third bad season: a losing record, or well under what the roster should have won.
                if let Employer::College(p) = job.employer
                    && let Some(s) = w.programs[p].seasons.last()
                {
                    let wp = s.wins as f64 / (s.wins + s.losses).max(1) as f64;
                    if wp < 0.5 || (last < -0.08 && wp < 0.65) {
                        fire = true;
                        why = "fired after his prove-it year";
                    }
                }
                w.coaches[c].prove_it = None;
            }
            Some(_) => w.coaches[c].prove_it = None,
            None => {}
        }
        if !fire && w.rng.chance(p_fire) {
            fire = true;
            if let Employer::College(p) = job.employer
                && earned_patience(w, c, p, tenure)
                && w.rng.chance(0.6 + 0.35 * excuse(w, p))
            {
                // Loyal, proven coaches get an extension instead (note 1).
                fire = false;
                w.coaches[c].prove_it = Some(year);
                let name = w.programs[p].name(w.options.names).to_string();
                let coach_name = w.coaches[c].name.clone();
                college_news(
                    w,
                    p,
                    NewsKind::CoachExtended,
                    format!(
                        "{name} give head coach {coach_name} a contract extension and a vote of confidence after two tough seasons."
                    ),
                );
            }
        }
        if fire {
            leave(w, c, why);
            w.coaches[c].recent.clear();
            let prestige = employer_prestige(w, job.employer);
            openings.push(Opening { employer: job.employer, role: Role::HeadCoach, prestige });
            if !hs && let Employer::College(p) = job.employer {
                let name = w.programs[p].name(w.options.names).to_string();
                let coach_name = w.coaches[c].name.clone();
                let text = if why == "fired" {
                    format!("{name} fire head coach {coach_name} after {tenure} seasons.")
                } else {
                    format!("{name} fire head coach {coach_name} after his prove-it season ({tenure} seasons in all).")
                };
                college_news(w, p, NewsKind::CoachFired, text);
            }
            // A new head coach usually brings his own coordinators and many of his own position coaches.
            if let Employer::College(p) = job.employer {
                for role in college_roles().skip(1) {
                    let keep = if matches!(role, Role::PositionCoach(_)) { 0.5 } else { 0.4 };
                    if let Some(a) = slot(&w.programs[p], role)
                        && !w.rng.chance(keep)
                    {
                        leave(w, a, "let go by the new staff");
                        let prestige = employer_prestige(w, job.employer);
                        openings.push(Opening { employer: job.employer, role, prestige });
                    }
                }
            }
        }
    }
    // Empty seats from earlier years too.
    for p in 0..w.programs.len() {
        let pr = employer_prestige(w, Employer::College(p));
        for role in college_roles() {
            if slot(&w.programs[p], role).is_none()
                && !openings.iter().any(|o| o.employer == Employer::College(p) && o.role == role)
            {
                openings.push(Opening { employer: Employer::College(p), role, prestige: pr });
            }
        }
    }
    for t in 0..w.nfl.len() {
        for role in nfl_roles() {
            if nfl_slot(&w.nfl[t], role).is_none()
                && !openings.iter().any(|o| o.employer == Employer::Nfl(t) && o.role == role)
            {
                openings.push(Opening { employer: Employer::Nfl(t), role, prestige: 1.0 });
            }
        }
    }
    for s in 0..w.high_schools.len() {
        if w.high_schools[s].coach.is_none() && !openings.iter().any(|o| o.employer == Employer::HighSchool(s)) {
            let pr = employer_prestige(w, Employer::HighSchool(s));
            openings.push(Opening { employer: Employer::HighSchool(s), role: Role::HeadCoach, prestige: pr });
        }
    }

    // --- Hiring, best jobs first ---------------------------------------------------
    let buzz: Vec<usize> = w.high_schools.iter().filter_map(|s| s.coach).filter(|&c| hs_buzz(w, c)).collect();
    let mut moved: HashSet<usize> = HashSet::new();
    let mut guard = 0;
    while !openings.is_empty() && guard < 200_000 {
        guard += 1;
        openings.sort_by(|a, b| (a.prestige + rung(a.role)).total_cmp(&(b.prestige + rung(b.role))));
        let o = openings.pop().unwrap();
        let candidates = candidate_pool(w, o, &moved, &buzz);
        let (needs_prestige, is_hs) = match o.employer {
            Employer::HighSchool(s) => (w.high_schools[s].prestige, true),
            Employer::College(p) => (w.programs[p].prestige, false),
            Employer::Nfl(_) => (1.0, false),
        };
        let nfl = matches!(o.employer, Employer::Nfl(_));
        // An NFL team doesn't recruit and is its own exposure: it pays for teaching.
        let needs = if nfl {
            HiringNeeds { network: 0.0, teaching: 0.6, recruiting: 0.0 }
        } else {
            hiring_needs(needs_prestige, is_hs)
        };
        let hiring_hc = match o.employer {
            Employer::College(p) if o.role != Role::HeadCoach => w.programs[p].head_coach,
            Employer::Nfl(t) if o.role != Role::HeadCoach => w.nfl[t].head_coach,
            _ => None,
        };
        // Nobody sees a coach's teaching directly (note 15B). A school reads
        // it from film and calls, sharper with a well-connected staff, and
        // adds what it can see: how his players grew and moved up.
        let net = match hiring_hc {
            Some(h) => w.coaches[h].ability.network,
            None => 6.0 + 10.0 * needs_prestige,
        };
        let read_sd = clamp(3.5 - 0.15 * (net - 10.0), 1.5, 5.0);
        let mut scored: Vec<(usize, f64)> = candidates
            .into_iter()
            .map(|c| {
                let mut s = candidate_score(&w.coaches[c], needs, &mut w.rng);
                let coach = &w.coaches[c];
                let a = *coach.ability();
                let seen = |g: Group, rng: &mut Rng| {
                    let u = coach.units[g as usize];
                    a.teaching[g as usize] - 10.0
                        + rng.normal(0.0, read_sd)
                        + 2.5 * u.growth_read()
                        + 0.8 * (1.0 + u.moved_up).ln()
                };
                match o.role {
                    // A coordinator hire is judged on his side of the ball.
                    Role::OffensiveCoordinator | Role::DefensiveCoordinator => {
                        let off = o.role == Role::OffensiveCoordinator;
                        let side: f64 = Group::ALL
                            .iter()
                            .filter(|g| g.offense() == off && **g != Group::Specialists)
                            .map(|&g| seen(g, &mut w.rng))
                            .sum::<f64>()
                            / 4.0;
                        s += 0.5 * side;
                        if coach.came_up.offense() != off {
                            s -= 1.0;
                        }
                    }
                    // A position coach is judged on teaching his group, and on
                    // whether he came up coaching it or his record there stands out.
                    Role::PositionCoach(g) => {
                        s += 0.8 * seen(g, &mut w.rng);
                        let u = coach.units[g as usize];
                        if coach.came_up == g {
                            s += 0.6;
                        } else if coach.came_up.offense() != g.offense() && u.growth_read() < 0.3 && u.moved_up < 2.0 {
                            s -= 1.5;
                        }
                    }
                    Role::HeadCoach => {
                        // Committees rarely hand a first head job to a man near
                        // retirement, and prefer coaches in their prime.
                        let age = coach.age as f64;
                        let been_head = coach.history.iter().any(|h| h.role == Role::HeadCoach)
                            || coach.job.is_some_and(|j| j.role == Role::HeadCoach);
                        s -= 0.5 * (age - 58.0).max(0.0);
                        if !been_head {
                            s -= 0.8 * (age - 54.0).max(0.0);
                        }
                    }
                }
                // Each world doubts the other's record (11.11, 11.12): NFL owners
                // doubt that college success carries over (a different game, no
                // recruiting edge); a college search doubts a man with no
                // recruiting ties (proposed, not confirmed: 4 and 2 points for a
                // head job, half for a coordinator job).
                let head = if o.role == Role::HeadCoach { 1.0 } else { 0.5 };
                if nfl && !has_coached(coach, true) {
                    s -= 4.0 * head;
                } else if !nfl && !is_hs && !has_coached(coach, false) && !matches!(o.role, Role::PositionCoach(_)) {
                    s -= 2.0 * head;
                }
                // Head coaches hire from their own tree (note 2: how the gap travels).
                if hiring_hc.is_some() && w.coaches[c].mentor == hiring_hc {
                    s += 1.5;
                }
                if !is_hs {
                    s -= hiring_bias(w.coaches[c].ethnicity, year, needs_prestige);
                }
                (c, s)
            })
            .collect();
        scored.sort_by(|a, b| b.1.total_cmp(&a.1));
        let mut hired = None;
        let mut why = String::new();
        // A search works down its list: when the first names say no, the
        // school calls the next ones (proposed, not confirmed: 20 calls).
        // Each one weighs the offer his own way (11.13).
        for &(c, _) in scored.iter().take(20) {
            if let Some(reason) = weigh_offer(w, c, o) {
                hired = Some(c);
                why = reason;
                break;
            }
        }
        // Nobody outside said yes to a head job: the school or team promotes
        // its own coordinator, the better-regarded one (an interim who stays).
        if hired.is_none() && o.role == Role::HeadCoach && !is_hs {
            let own = match o.employer {
                Employer::College(p) => [w.programs[p].oc, w.programs[p].dc],
                Employer::Nfl(t) => [w.nfl[t].oc, w.nfl[t].dc],
                Employer::HighSchool(_) => [None, None],
            };
            hired = own
                .into_iter()
                .flatten()
                .filter(|c| !moved.contains(c))
                .max_by(|&a, &b| w.coaches[a].reputation.total_cmp(&w.coaches[b].reputation));
            why = "the names outside said no".to_string();
        }
        let c = match hired {
            Some(c) => {
                if let Some(job) = w.coaches[c].job {
                    leave(w, c, "left for another job");
                    let prestige = employer_prestige(w, job.employer);
                    openings.push(Opening { employer: job.employer, role: job.role, prestige });
                    if job.employer != o.employer {
                        departure_news(w, c, job, o, &why);
                    }
                }
                c
            }
            None => {
                // Nobody suitable said yes: promote a young assistant.
                why = if o.role == Role::HeadCoach {
                    "a long-time assistant's first head job, after the names on the list said no".to_string()
                } else {
                    String::new()
                };
                let tier = match o.employer {
                    Employer::HighSchool(_) => CoachTier::HighSchool,
                    Employer::College(p) => tier_of(w.programs[p].division),
                    Employer::Nfl(_) => CoachTier::Nfl,
                };
                // A head job goes to a long-time assistant; assistant jobs go younger.
                let (lo, hi) = match (o.role, o.employer) {
                    (Role::HeadCoach, Employer::College(_)) => (36, 52),
                    (Role::HeadCoach, Employer::Nfl(_)) => (40, 55),
                    (_, Employer::Nfl(_)) => (34, 50),
                    (Role::PositionCoach(_), _) => (25, 36),
                    _ => (28, 40),
                };
                let age = w.rng.int(lo, hi) as u32;
                // The same committee bias decides which long-time assistant gets a head job.
                let pool = if is_hs {
                    CoachPool::HighSchool
                } else if o.role == Role::HeadCoach
                    && w.rng.chance(1.0 - (-hiring_bias(Ethnicity::Black, year, needs_prestige) / 3.0).exp())
                {
                    CoachPool::CollegeHead1990
                } else {
                    CoachPool::Entering
                };
                let c = new_coach(w, tier, age, pool);
                let drop = if matches!(o.role, Role::PositionCoach(_)) { 3.0 } else { 2.0 };
                w.coaches[c].reputation = clamp(w.coaches[c].reputation - drop, 1.0, 20.0);
                c
            }
        };
        moved.insert(c);
        let from_job = w.coaches[c].history.last().map(|s| (s.employer, s.role));
        hire(w, c, o.employer, o.role);
        // Where he came from, and why he came (11.13).
        let from = from_job.map_or("a new coach".to_string(), |(e, _)| match e {
            e if e == o.employer => "promoted from his own staff".to_string(),
            Employer::College(q) => format!("from {}", w.programs[q].name(w.options.names)),
            Employer::HighSchool(h) => format!("from {} High School football", w.high_schools[h].name),
            Employer::Nfl(u) => format!("from the {}", w.nfl[u].name),
        });
        let from = if why.is_empty() { from } else { format!("{from}: {why}") };
        if w.recording
            && let Employer::Nfl(t) = o.employer
            && o.role == Role::HeadCoach
        {
            let text = format!("The {} hire {} as head coach ({from}).", w.nfl[t].name, w.coaches[c].name);
            w.news.push(NewsItem { year, kind: NewsKind::CoachHired, channel: Channel::News, program: None, text });
        }
        if w.recording
            && let Employer::College(p) = o.employer
            && (o.role == Role::HeadCoach || matches!(from_job, Some((Employer::HighSchool(_), _))))
        {
            let name = w.programs[p].name(w.options.names).to_string();
            let coach = &w.coaches[c];
            if o.role == Role::HeadCoach {
                let line = format!("Hired head coach {} ({from}).", coach.name);
                w.programs[p].story.push((year, line));
            }
            let coach = &w.coaches[c];
            let text = if o.role == Role::HeadCoach {
                format!("{name} hire {} ({from}).", coach.name)
            } else {
                format!("{name} hire {} as {} ({from}).", coach.name, o.role.name())
            };
            w.news.push(NewsItem { year, kind: NewsKind::CoachHired, channel: Channel::News, program: Some(p), text });
        }
    }

    // --- Everyone ages a year; assistants learn from their head coach --------------
    for c in 0..w.coaches.len() {
        if w.coaches[c].retired {
            continue;
        }
        let mentor = match w.coaches[c].job {
            Some(Job { employer: Employer::College(p), role, .. }) if role != Role::HeadCoach => {
                w.programs[p].head_coach.map(|h| *w.coaches[h].ability())
            }
            Some(Job { employer: Employer::Nfl(t), role, .. }) if role != Role::HeadCoach => {
                w.nfl[t].head_coach.map(|h| *w.coaches[h].ability())
            }
            _ => None,
        };
        let mut rng = w.rng.fork();
        w.coaches[c].age_one_year(mentor.as_ref(), &mut rng);
        // Schools remember recent work best.
        for u in w.coaches[c].units.iter_mut() {
            u.fade(0.85);
        }
        // Recruiting ties fade unless impact players keep them alive.
        w.coaches[c].ties.retain(|_, t| {
            *t *= 0.9;
            *t > 0.02
        });
        if w.coaches[c].job.is_none() && w.coaches[c].age > 66 {
            w.coaches[c].retired = true;
        }
    }
    set_nfl_coaching(w);
    if w.recording {
        diversity_report(w);
    }
}

/// The yearly TIDES-style count of FBS head coaches by ethnicity (note 2).
fn diversity_report(w: &mut World) {
    let mut counts = [0u32; super::ethnicity::ETHNICITY_COUNT];
    let mut total = 0;
    for p in w.programs.iter().filter(|p| p.division == Division::FBS) {
        if let Some(c) = p.head_coach {
            counts[w.coaches[c].ethnicity as usize] += 1;
            total += 1;
        }
    }
    let parts: Vec<String> = Ethnicity::ALL
        .iter()
        .filter(|&&e| e != Ethnicity::White && counts[e as usize] > 0)
        .map(|&e| format!("{} {}", counts[e as usize], e.name()))
        .collect();
    let of_color = total - counts[Ethnicity::White as usize];
    let text = format!(
        "Diversity report: {of_color} of {total} FBS head coaches are coaches of color ({}).",
        if parts.is_empty() { "none".to_string() } else { parts.join(", ") }
    );
    w.news.push(NewsItem {
        year: w.year,
        kind: NewsKind::DiversityReport,
        channel: Channel::News,
        program: None,
        text,
    });
}

/// Who a search looks at for an opening.
fn candidate_pool(w: &mut World, o: Opening, moved: &HashSet<usize>, buzz: &[usize]) -> Vec<usize> {
    let mut pool: Vec<usize> = Vec::new();
    let sample = |w: &mut World, list: &[usize], k: usize, pool: &mut Vec<usize>| {
        if list.is_empty() {
            return;
        }
        for _ in 0..k {
            pool.push(list[w.rng.int(0, list.len() as i32 - 1) as usize]);
        }
    };
    match o.employer {
        Employer::College(p) => {
            let div = w.programs[p].division;
            let mut list: Vec<usize> = Vec::new();
            for q in 0..w.programs.len() {
                if q == p {
                    continue;
                }
                let qd = w.programs[q].division;
                let same = qd == div;
                // One level down: FCS head coaches move up to FBS (Cignetti at JMU).
                let below = (qd as usize) == div as usize + 1;
                let above = (qd as usize) + 1 == div as usize;
                let prog = &w.programs[q];
                for role in college_roles() {
                    let Some(c) = slot(prog, role) else { continue };
                    let fits = match (o.role, role) {
                        (Role::HeadCoach, Role::PositionCoach(_)) => false,
                        // A coordinator a level down rarely jumps straight to a
                        // head job; head coaches there do (note 5).
                        (Role::HeadCoach, Role::HeadCoach) => same || below,
                        (Role::HeadCoach, _) => same,
                        // Coordinators come from coordinators, head coaches a level
                        // down, and position coaches on that side of the ball.
                        (Role::OffensiveCoordinator | Role::DefensiveCoordinator, Role::HeadCoach) => below,
                        (Role::OffensiveCoordinator | Role::DefensiveCoordinator, _) => {
                            (same || below || above) && role.offense() == o.role.offense()
                        }
                        // Position coaches move sideways, up from a level down, or
                        // down from a level up into a coordinator job's shadow.
                        (Role::PositionCoach(g), Role::PositionCoach(h)) => g == h && (same || below || above),
                        (Role::PositionCoach(g), Role::OffensiveCoordinator | Role::DefensiveCoordinator) => {
                            below && role.offense() == Some(g.offense())
                        }
                        (Role::PositionCoach(_), Role::HeadCoach) => false,
                    };
                    if fits && !moved.contains(&c) {
                        list.push(c);
                    }
                }
            }
            // The short list (11.11): for a head or coordinator job, the
            // hottest names that fit come first (proposed, not confirmed: 15
            // for a head job, 10 for a coordinator job). The random draw
            // still brings lesser-known names.
            let short = match o.role {
                Role::HeadCoach => 15,
                Role::OffensiveCoordinator | Role::DefensiveCoordinator => 10,
                Role::PositionCoach(_) => 0,
            };
            pool.extend(hottest(w, &list, short));
            sample(w, &list, 40, &mut pool);
            // A big head job also calls the hottest NFL coordinators (proposed: 3).
            if o.role == Role::HeadCoach && div == Division::FBS {
                let nfl: Vec<usize> =
                    w.nfl.iter().flat_map(|t| [t.oc, t.dc]).flatten().filter(|c| !moved.contains(c)).collect();
                pool.extend(hottest(w, &nfl, 3));
            }
            // High school coaches: the top ones get small-college head jobs; a
            // dominant one can jump to a college staff at any level (Gus Malzahn).
            let hs_head = o.role == Role::HeadCoach && matches!(div, Division::D2 | Division::JUCO);
            if hs_head {
                let hs: Vec<usize> = w
                    .high_schools
                    .iter()
                    .filter(|s| s.prestige > 0.7)
                    .filter_map(|s| s.coach)
                    .filter(|c| !moved.contains(c))
                    .collect();
                sample(w, &hs, 6, &mut pool);
            }
            let free_buzz: Vec<usize> = buzz.iter().copied().filter(|c| !moved.contains(c)).collect();
            if o.role != Role::HeadCoach || div != Division::FBS {
                sample(w, &free_buzz, 3, &mut pool);
            }
            // Position coaches also come from strong high school staffs and ex-pros.
            if let Role::PositionCoach(g) = o.role {
                let state = w.programs[p].state;
                let hs: Vec<usize> = w.states[state]
                    .schools
                    .iter()
                    .filter_map(|&h| w.high_schools[h].coach)
                    .filter(|&c| {
                        // Seen as a candidate if he came up on that side of the
                        // ball or his players at that spot stood out (note 15B).
                        let coach = &w.coaches[c];
                        let u = coach.units[g as usize];
                        !moved.contains(&c)
                            && ((coach.came_up.offense() == g.offense() && u.growth_read() > 0.15) || u.moved_up >= 1.5)
                    })
                    .collect();
                sample(w, &hs, 4, &mut pool);
                if w.rng.chance(0.12) {
                    let tier = tier_of(div);
                    let age = w.rng.int(27, 38) as u32;
                    let c = new_coach(w, tier, age, CoachPool::Entering);
                    let coach = &mut w.coaches[c];
                    coach.ex_pro = true;
                    coach.ability.network = clamp(coach.ability.network + w.rng.range(3.0, 6.0), 1.0, 20.0);
                    coach.reputation = clamp(coach.reputation - 1.0, 1.0, 20.0);
                    pool.push(c);
                }
            }
        }
        Employer::HighSchool(s) => {
            let state = w.high_schools[s].state;
            let list: Vec<usize> = w.states[state]
                .schools
                .iter()
                .filter(|&&h| h != s)
                .filter_map(|&h| w.high_schools[h].coach)
                .filter(|c| !moved.contains(c))
                .collect();
            sample(w, &list, 20, &mut pool);
            // An ex-pro looking for his first job: knowledge and network, less teaching (1.5).
            if w.rng.chance(0.25) {
                let age = w.rng.int(30, 45) as u32;
                let c = new_coach(w, CoachTier::HighSchool, age, CoachPool::Entering);
                let coach = &mut w.coaches[c];
                coach.ex_pro = true;
                coach.ability.network = clamp(coach.ability.network + w.rng.range(4.0, 8.0), 1.0, 20.0);
                coach.personality.ambition = clamp(coach.personality.ambition + 3.0, 1.0, 20.0);
                coach.reputation = clamp(coach.reputation + 2.0, 1.0, 20.0);
                pool.push(c);
            }
        }
        Employer::Nfl(t) => {
            // NFL coordinators: any of them for a head job, the same side of the
            // ball elsewhere for a coordinator job (11.12).
            let head = o.role == Role::HeadCoach;
            let side = o.role.offense();
            for (u, team) in w.nfl.iter().enumerate() {
                for (r, c) in [(Role::OffensiveCoordinator, team.oc), (Role::DefensiveCoordinator, team.dc)] {
                    if let Some(c) = c
                        && (head || (u != t && r.offense() == side))
                        && !moved.contains(&c)
                    {
                        pool.push(c);
                    }
                }
            }
            // Out-of-work NFL coaches: former head coaches, and coordinators on that side.
            for c in w.coaches.iter().filter(|c| c.job.is_none() && !c.retired && c.age < 66) {
                let fits = c.history.iter().any(|s| {
                    matches!(s.employer, Employer::Nfl(_))
                        && (s.role == Role::HeadCoach || (!head && s.role.offense() == side))
                });
                if fits && !moved.contains(&c.id) {
                    pool.push(c.id);
                }
            }
            // The hottest college names: FBS head coaches for a head job, FBS
            // coordinators on that side for a coordinator job (proposed: 8 and 5).
            let college: Vec<usize> = w
                .programs
                .iter()
                .filter(|p| p.division == Division::FBS)
                .filter_map(|p| match o.role {
                    Role::OffensiveCoordinator => p.oc,
                    Role::DefensiveCoordinator => p.dc,
                    _ => p.head_coach,
                })
                .filter(|c| !moved.contains(c))
                .collect();
            pool.extend(hottest(w, &college, if head { 8 } else { 5 }));
        }
    }
    // Unemployed coaches still looking. A former college head coach doesn't take a high school job.
    let hs_job = matches!(o.employer, Employer::HighSchool(_));
    let free: Vec<usize> = w
        .coaches
        .iter()
        .filter(|c| c.job.is_none() && !c.retired && c.age < 66 && !c.history.is_empty())
        .filter(|c| {
            !hs_job
                || !c.history.iter().any(|s| {
                    s.role == Role::HeadCoach
                        && matches!(s.employer, Employer::College(p) if matches!(w.programs[p].division, Division::FBS | Division::FCS))
                })
        })
        .map(|c| c.id)
        .filter(|c| !moved.contains(c))
        .take(400)
        .collect();
    sample(w, &free, 8, &mut pool);
    pool.sort_unstable();
    pool.dedup();
    pool
}

/// Prestige, grades, NIL and footprints after the season (topic 7, 8, 4).
pub(crate) fn update_programs(w: &mut World, r: &SeasonResults) {
    let year = w.year;
    // Conference strength: the average prestige of the members.
    let mut conf: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for p in w.programs.iter() {
        let e = conf.entry(p.conference.clone()).or_insert((0.0, 0.0));
        e.0 += p.prestige;
        e.1 += 1.0;
    }
    let conf_strength = |name: &str| conf.get(name).map_or(0.3, |(s, n)| s / n);

    // Raw values per grade, then graded within each division.
    let n = w.programs.len();
    let mut raw = vec![[0.0f64; GRADE_COUNT]; n];
    // Playing time: starters about to leave open spots; a weak roster opens more.
    let mut leaving = vec![(0.0f64, 0.0f64); n];
    for p in w.people.iter() {
        if p.stage == Stage::College
            && let Some(c) = p.college
            && p.last_role == Some(PlayingTime::Starter)
        {
            leaving[c].1 += 1.0;
            if p.college_seasons >= 3 {
                leaving[c].0 += 1.0;
            }
        }
    }
    for i in 0..n {
        let t = &r.college[i];
        let win = if t.games() > 0 { t.win_pct() } else { 0.5 };
        let hc_rep = w.programs[i].head_coach.map_or(8.0, |c| w.coaches[c].reputation);
        let mut rng = w.rng.fork();
        let (recording, year) = (w.recording, w.year);
        let p = &mut w.programs[i];
        let scale = match p.division {
            Division::FBS => 1.0,
            Division::FCS => 0.45,
            Division::D2 => 0.25,
            Division::JUCO => 0.15,
        };
        let season_score = scale
            * (0.2
                + 0.8 * win
                + if t.conference_title { 0.15 } else { 0.0 }
                + if t.national_title { 0.3 } else { 0.0 });
        if w.recording {
            p.recent = 0.6 * p.recent + 0.4 * clamp(season_score, 0.0, 1.0);
            // Tradition is built by titles; recent form only seeps in slowly (note 15C).
            p.tradition = clamp(
                0.995 * p.tradition
                    + 0.005 * p.recent
                    + if t.national_title { 0.03 } else { 0.0 }
                    + if t.conference_title { 0.004 } else { 0.0 },
                0.0,
                1.0,
            );
            let fan_target =
                clamp(0.4 * p.tradition + 0.4 * p.recent + 0.2 * (p.stadium as f64 / 110_000.0).sqrt(), 0.0, 1.0);
            p.fans = 0.93 * p.fans + 0.07 * fan_target;
            let nil_share = clamp((p.collective.budget / 25.0e6).sqrt(), 0.0, 1.0);
            let fac_target = clamp(scale * (0.3 + 0.4 * p.fans) + 0.3 * nil_share, 0.0, 1.0);
            // Facilities move in steps (note 15C): buildings age a little each
            // year, and a project lands when the money and the will line up.
            p.facilities = (p.facilities - 0.006).max(0.02);
            let gap = fac_target - p.facilities;
            let booster = p.collective.big_booster.is_some();
            let chance = clamp(0.03 + 0.6 * gap.max(0.0) + if booster { 0.05 } else { 0.0 }, 0.0, 0.4);
            if gap > 0.02 && rng.chance(chance) {
                let step = clamp(gap + rng.range(0.0, 0.08), 0.04, 0.2);
                p.facilities = clamp(p.facilities + step, 0.02, 1.0);
                let cause = if booster && rng.chance(0.6) {
                    "paid for by a big booster"
                } else if win >= 0.7 {
                    "after a winning run filled the budget"
                } else {
                    "a school budget decision"
                };
                if recording {
                    p.story.push((year, format!("New football facilities ({cause}).")));
                }
            }
        }
        p.prestige = program::prestige_of(p, hc_rep, conf_strength(&p.conference));
    }
    for division in [Division::FBS, Division::FCS, Division::D2, Division::JUCO] {
        let top = w.programs.iter().filter(|p| p.division == division).map(|p| p.prestige).fold(0.01, f64::max);
        for p in w.programs.iter_mut().filter(|p| p.division == division) {
            p.prestige_rel = p.prestige / top;
        }
    }
    for i in 0..n {
        let t = &r.college[i];
        let win = if t.games() > 0 { t.win_pct() } else { 0.5 };
        let hc_rep = w.programs[i].head_coach.map_or(8.0, |c| w.coaches[c].reputation);
        let p = &mut w.programs[i];
        let picks: f64 = p
            .draft_picks
            .iter()
            .filter(|(y, _)| year - y < 4)
            .map(|&(_, round)| match round {
                1 => 3.0,
                2 | 3 => 2.0,
                _ => 1.0,
            })
            .sum();
        let (leave, starters) = leaving[i];
        let academics = match p.academic_tier {
            AcademicTier::Aau => 0.9,
            AcademicTier::R1 => 0.7,
            AcademicTier::R2 => 0.5,
            AcademicTier::Other => 0.3,
        } + if p.conference == "Ivy" { 0.4 } else { 0.0 };
        let titles_recent: f64 = p.seasons.iter().rev().take(2).filter(|s| s.conference_title).count() as f64;
        raw[i][GradeCat::Academics as usize] = academics;
        raw[i][GradeCat::ProPipeline as usize] = picks;
        raw[i][GradeCat::PlayingTime as usize] = leave / starters.max(1.0) - 0.01 * t.strength;
        raw[i][GradeCat::Tradition as usize] = p.tradition;
        raw[i][GradeCat::TitleChances as usize] =
            0.02 * t.strength + (0.6 * titles_recent + win) * conf_strength(&p.conference);
        raw[i][GradeCat::CoachPrestige as usize] = hc_rep;
        raw[i][GradeCat::Facilities as usize] = p.facilities;
        raw[i][GradeCat::Nil as usize] = p.collective.budget;
        raw[i][GradeCat::BrandExposure as usize] =
            0.6 * p.fans + 0.2 * p.tradition + if p.power_four() { 0.2 } else { 0.0 };
        raw[i][GradeCat::ConferenceStrength as usize] = conf_strength(&p.conference);
        raw[i][GradeCat::StadiumAtmosphere as usize] =
            (p.stadium as f64).sqrt() * (0.5 + p.fans) * (0.7 + 0.3 * p.recent);
        raw[i][GradeCat::CampusLife as usize] = (p.enrollment as f64).ln() + 0.3 * academics;
    }
    for division in [Division::FBS, Division::FCS, Division::D2, Division::JUCO] {
        let ids: Vec<usize> = (0..n).filter(|&i| w.programs[i].division == division).collect();
        for cat in GradeCat::ALL {
            let mut sorted: Vec<(f64, usize)> = ids.iter().map(|&i| (raw[i][cat as usize], i)).collect();
            sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
            let m = sorted.len().max(2) - 1;
            for (rank, &(_, i)) in sorted.iter().enumerate() {
                let target = 55.0 + 44.0 * rank as f64 / m as f64;
                let p = &mut w.programs[i];
                let k = cat as usize;
                p.last_grades[k] = p.grades[k];
                p.grades[k] =
                    if w.recording { program::move_grade(p.grades[k], target, cat.speed(), false) } else { target };
            }
        }
    }

    // --- NIL collectives and their warnings -------------------------------------------
    for i in 0..n {
        let win = if r.college[i].games() > 0 { r.college[i].win_pct() } else { 0.5 };
        let before = w.programs[i].collective.budget;
        let mut rng: Rng = w.rng.fork();
        let level = program::update_collective(&mut w.programs[i], win, &mut rng);
        let p = &w.programs[i];
        if w.recording && level > 0 && p.division != Division::JUCO {
            let change = (p.collective.budget / before.max(1.0) - 1.0) * 100.0;
            let name = p.name(w.options.names).to_string();
            let booster_left = p.collective.trend == NilTrend::BoosterLeft;
            w.news.extend(news::nil_warnings(year, i, &name, level, booster_left, change));
        }
    }
    for p in w.programs.iter_mut() {
        program::fade_pipelines(p, year);
        program::rebuild_footprint(p, &w.overflow);
        if w.recording {
            p.grade_history.push((year, p.grades));
            p.prestige_history.push((year, p.prestige));
        }
    }
}

/// What schools can see of each coach's teaching (note 15B): this year's
/// growth of every player he coached, against all players at the same level
/// and age. A high school head coach teaches every group; in college the
/// position coach counts most, then the coordinator, then the head coach.
/// One player's year: his group, his (level, age) cohort, his growth and the
/// coaches credited with their weights.
type TeachingRow = (Group, (u8, u32), f64, [(Option<usize>, f64); 3]);

pub(crate) fn credit_teaching(w: &mut World) {
    if !w.recording {
        return;
    }
    let mut rows: Vec<TeachingRow> = Vec::new();
    let mut norm: BTreeMap<(u8, u32), (f64, f64)> = BTreeMap::new();
    for p in &w.people {
        let Some(h) = p.player.history.last() else { continue };
        if h.age + 1 != p.player.age || h.performance.is_none() {
            continue;
        }
        let group = Group::of(h.position);
        let staff = match p.stage {
            Stage::HighSchool => [(w.high_schools[p.hs].coach, 1.0), (None, 0.0), (None, 0.0)],
            Stage::College => {
                let prog = &w.programs[p.college.unwrap_or(0)];
                let coord = if group.offense() { prog.oc } else { prog.dc };
                [(prog.position_coaches[group as usize], 1.0), (coord, 0.5), (prog.head_coach, 0.25)]
            }
            _ => continue,
        };
        let key = (h.level as u8, h.age);
        let delta = h.overall - h.overall_start;
        let e = norm.entry(key).or_default();
        e.0 += delta;
        e.1 += 1.0;
        rows.push((group, key, delta, staff));
    }
    for (group, key, delta, staff) in rows {
        let (sum, n) = norm[&key];
        let residual = delta - sum / n;
        for (c, wt) in staff {
            if let Some(c) = c {
                let u = &mut w.coaches[c].units[group as usize];
                u.growth += wt * residual;
                u.seasons += wt;
            }
        }
    }
}
