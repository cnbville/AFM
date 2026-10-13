//! Staffs, the coaching carousel, and the yearly update of every program's
//! prestige, grades and NIL collective (topics 3, 7 and 8).

use std::collections::{BTreeMap, HashSet};

use super::coach::{
    CoachTier, Employer, Group, Job, Role, Stint, TitleKind, candidate_score, generate_coach, hiring_needs,
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
    let c = generate_coach(id, name, ethnicity, tier, age, &mut w.rng);
    w.coaches.push(c);
    id
}

fn hire(w: &mut World, coach: usize, employer: Employer, role: Role) {
    let year = w.year;
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
        Employer::Nfl(_) => {}
    }
    // An assistant hired under a head coach joins his tree.
    if role != Role::HeadCoach
        && let Employer::College(p) = employer
        && let Some(hc) = w.programs[p].head_coach
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
        Employer::Nfl(_) => {}
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
/// title in one of his last two seasons and almost no losses.
fn hs_buzz(w: &World, c: usize) -> bool {
    let coach = &w.coaches[c];
    let Some(Job { employer: Employer::HighSchool(s), .. }) = coach.job else { return false };
    let seasons: Vec<_> = w.high_schools[s].seasons.iter().rev().take(2).collect();
    let title = seasons.iter().any(|x| x.state_title);
    let (wins, games) = seasons.iter().fold((0, 0), |a, x| (a.0 + x.wins, a.1 + x.wins + x.losses));
    title && games > 0 && wins as f64 / games as f64 >= 0.9
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
        if retire {
            leave(w, c, "retired");
            w.coaches[c].retired = true;
            let prestige = employer_prestige(w, job.employer);
            openings.push(Opening { employer: job.employer, role: job.role, prestige });
            continue;
        }
        if job.role != Role::HeadCoach {
            continue;
        }
        let tenure = year - job.since;
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
        let needs = hiring_needs(needs_prestige, is_hs);
        let hiring_hc = match o.employer {
            Employer::College(p) if o.role != Role::HeadCoach => w.programs[p].head_coach,
            _ => None,
        };
        let mut scored: Vec<(usize, f64)> = candidates
            .into_iter()
            .map(|c| {
                let mut s = candidate_score(&w.coaches[c], needs, &mut w.rng);
                let a = w.coaches[c].ability();
                match o.role {
                    // A coordinator hire is judged on his side of the ball.
                    Role::OffensiveCoordinator | Role::DefensiveCoordinator => {
                        let side: f64 = Group::ALL
                            .iter()
                            .filter(|g| g.offense() == (o.role == Role::OffensiveCoordinator))
                            .map(|&g| a.teaching[g as usize])
                            .sum::<f64>()
                            / 4.0;
                        s += 0.5 * (side - 10.0);
                    }
                    // A position coach is judged on teaching his group.
                    Role::PositionCoach(g) => s += 0.8 * (a.teaching[g as usize] - 10.0),
                    Role::HeadCoach => {}
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
        for &(c, _) in scored.iter().take(6) {
            let coach = &w.coaches[c];
            let accepts = match coach.job {
                None => true,
                Some(job) => {
                    let current = employer_prestige(w, job.employer) + rung(job.role);
                    let offered = o.prestige + rung(o.role);
                    let gain = offered - current;
                    gain > 0.03
                        && w.rng.chance(clamp(
                            0.35 + 0.04 * (coach.personality.ambition - 10.0)
                                - 0.03 * (coach.personality.loyalty - 10.0)
                                + 1.5 * gain,
                            0.05,
                            0.95,
                        ))
                }
            };
            if accepts {
                hired = Some(c);
                break;
            }
        }
        let c = match hired {
            Some(c) => {
                if let Some(job) = w.coaches[c].job {
                    leave(w, c, "left for another job");
                    let prestige = employer_prestige(w, job.employer);
                    openings.push(Opening { employer: job.employer, role: job.role, prestige });
                }
                c
            }
            None => {
                // Nobody suitable said yes: promote a young assistant.
                let tier = match o.employer {
                    Employer::HighSchool(_) => CoachTier::HighSchool,
                    Employer::College(p) => tier_of(w.programs[p].division),
                    Employer::Nfl(_) => CoachTier::Nfl,
                };
                // A head job goes to a long-time assistant; assistant jobs go younger.
                let (lo, hi) = match (o.role, o.employer) {
                    (Role::HeadCoach, Employer::College(_)) => (36, 52),
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
        if w.recording
            && let Employer::College(p) = o.employer
            && (o.role == Role::HeadCoach || matches!(from_job, Some((Employer::HighSchool(_), _))))
        {
            let name = w.programs[p].name(w.options.names).to_string();
            let coach = &w.coaches[c];
            let from = from_job.map_or("a new coach".to_string(), |(e, _)| match e {
                Employer::College(q) => format!("from {}", w.programs[q].name(w.options.names)),
                Employer::HighSchool(h) => format!("from {} High School football", w.high_schools[h].name),
                Employer::Nfl(_) => "from the NFL".to_string(),
            });
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
            _ => None,
        };
        let mut rng = w.rng.fork();
        w.coaches[c].age_one_year(mentor.as_ref(), &mut rng);
        // Recruiting ties fade unless impact players keep them alive.
        w.coaches[c].ties.retain(|_, t| {
            *t *= 0.9;
            *t > 0.02
        });
        if w.coaches[c].job.is_none() && w.coaches[c].age > 66 {
            w.coaches[c].retired = true;
        }
    }
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
                        (Role::HeadCoach, _) => same || below,
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
            sample(w, &list, 40, &mut pool);
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
                    .filter(|&c| !moved.contains(&c) && w.coaches[c].teaching(g) >= 13.0)
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
        Employer::Nfl(_) => {}
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
            p.tradition =
                clamp(0.985 * p.tradition + 0.015 * p.recent + if t.national_title { 0.03 } else { 0.0 }, 0.0, 1.0);
            let fan_target =
                clamp(0.4 * p.tradition + 0.4 * p.recent + 0.2 * (p.stadium as f64 / 110_000.0).sqrt(), 0.0, 1.0);
            p.fans = 0.93 * p.fans + 0.07 * fan_target;
            let nil_share = clamp((p.collective.budget / 25.0e6).sqrt(), 0.0, 1.0);
            let fac_target = clamp(scale * (0.3 + 0.4 * p.fans) + 0.3 * nil_share, 0.0, 1.0);
            p.facilities = 0.95 * p.facilities + 0.05 * fac_target;
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
