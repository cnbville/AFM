//! Every day: what happens on a day. The coaching carousel (a firing, the
//! search, the interim, the names in the rumors, the hire), the recruits
//! (a visit, a commitment, a flip, a signing, a late rise), the portal, the
//! pros' holdouts, trade requests, suspensions, tags and trades, the courts,
//! a death in the family, the season's off-field trouble as it comes out,
//! and the dated days: the draft, signing days, award nights, the Hall of
//! Fame vote, Black Monday, the Super Bowl, free agency, your own offers.
//!
//! Thresholds are Claude's picks (proposed, not confirmed).

use super::weekly::{class_scores, employer, note, opener, roll, team_of_employer};
use super::{
    F_HOLDOUT, F_INCIDENT, F_OPTED, F_PORTAL, F_SUSPENDED, F_TAGGED, F_WANTS_OUT, K, Part, Who, cname, coach_spot,
    college, end, end_once, in_season, mine, on, open_of, pid, player_spot, pname, s_coach, s_player, s_season,
    team_of, team_spot, tell, tname,
};
use crate::calendar::{Date, Level};
use crate::player::PlayingTime;
use crate::positions::Position;
use crate::world::coach::{Employer, Role};
use crate::world::data::Division;
use crate::world::folk::courts::Step;
use crate::world::pro::market::Hold;
use crate::world::pro::money::Kind as Deal;
use crate::world::stories::TeamKey;
use crate::world::storylines::{Family, Spot, close, fizzle, open_id, pi, subj, tcode};
use crate::world::{Stage, Visit, World};

/// Once a day.
pub(crate) fn day(w: &mut World, d: Date) {
    let teams = super::weekly::teams(w);
    carousel(w, d, &teams);
    recruits(w, d);
    portal(w, d);
    players(w, d, &teams);
    trades(w, d);
    courts(w, d);
    life(w, d);
    incidents(w, d);
    dated(w, d);
    closings(w, d);
    offers(w, d);
}

/// Open stories of a kind about a team (its `team`).
fn about(w: &World, kind: K, t: TeamKey) -> Vec<u32> {
    let k = kind.id();
    w.storylines.open.iter().filter(|s| s.kind == k && s.phase.open() && s.team == Some(t)).map(|s| s.id).collect()
}

/// A college player the press follows: a starter, a four- or five-star, a
/// high draft read.
fn notable_college(w: &World, i: usize) -> bool {
    let p = &w.people[i];
    p.last_role == Some(PlayingTime::Starter) || p.stars >= 4 || p.cal.draft_read.is_some_and(|r| r.round <= 3)
}

/// A pro the press follows: a starter, a Pro Bowler of the last three
/// seasons, a recent first-round pick, a well-graded player.
fn notable_pro(w: &World, i: usize) -> bool {
    let p = &w.people[i];
    let Some(c) = p.pro.as_ref() else { return false };
    let (id, year) = (p.player.id, w.year);
    let starter = c.seasons.iter().rev().find(|s| s.year >= year - 1).is_some_and(|s| s.starts >= 8);
    let first = c.draft.is_some_and(|(dy, r, _, _)| r == 1 && year - dy <= 3);
    starter || first || c.last_grade.unwrap_or(0.0) >= 78.0 || pro_bowler(w, id, 3)
}

/// A Pro Bowl in the last `years` seasons.
fn pro_bowler(w: &World, id: u32, years: i32) -> bool {
    let year = w.year;
    w.nfl_book.pro_bowls.iter().any(|x| x.1 == id && x.0 >= year - years)
}

/// The next signing day (the early period from 2017).
fn next_signing(w: &World, d: Date) -> Date {
    let mut v = vec![w.dates.signing_day];
    v.extend(w.dates.early_signing);
    if let Some(p) = w.prev_dates.as_ref() {
        v.push(p.signing_day);
        v.extend(p.early_signing);
    }
    v.into_iter().filter(|&x| x >= d).min().unwrap_or(w.dates.signing_day)
}

// --- The carousel ------------------------------------------------------------

/// A team's head coach and coordinators (index + 1, 0 for none).
fn staff_now(w: &World, t: TeamKey) -> [u32; 3] {
    let f = |c: Option<usize>| c.map_or(0, |c| c as u32 + 1);
    match t {
        TeamKey::College(p) => w.programs.get(p as usize).map_or([0; 3], |g| [f(g.head_coach), f(g.oc), f(g.dc)]),
        TeamKey::Nfl(n) => w.nfl.get(n as usize).map_or([0; 3], |g| [f(g.head_coach), f(g.oc), f(g.dc)]),
        TeamKey::Hs(h) => w.high_schools.get(h as usize).map_or([0; 3], |s| [f(s.coach), 0, 0]),
        TeamKey::Other(_) => [0; 3],
    }
}

/// The staff against the last look: who left, who came.
fn carousel(w: &mut World, d: Date, teams: &[TeamKey]) {
    let first = w.storylines.watch.heads.is_empty();
    let mut changes = Vec::new();
    for &t in teams {
        let now = staff_now(w, t);
        if let Some(old) = w.storylines.watch.heads.insert(tcode(t), now)
            && old != now
            && !first
        {
            changes.push((t, old, now));
        }
    }
    for (t, old, now) in changes {
        if old[0] != now[0] {
            if old[0] != 0 {
                head_left(w, t, old[0] as usize - 1, d);
            }
            if now[0] != 0 {
                head_hired(w, t, now[0] as usize - 1);
            }
        }
        for k in 1..3 {
            if old[k] == now[k] {
                continue;
            }
            if old[k] != 0 {
                coordinator_left(w, t, old[k] as usize - 1, d);
            }
            if now[k] != 0 {
                coordinator_hired(w, t, now[k] as usize - 1);
            }
        }
    }
}

/// The stint a coach just ended at a team, if it is his last.
fn last_stint(w: &World, c: usize, t: TeamKey) -> Option<crate::world::coach::Stint> {
    w.coaches[c].history.last().filter(|s| team_of_employer(s.employer) == t).cloned()
}

fn head_left(w: &mut World, t: TeamKey, c: usize, d: Date) {
    let year = w.year;
    let stint = last_stint(w, c, t);
    let why = stint.as_ref().map_or("", |s| s.ended);
    let fired = why.starts_with("fired") || why.starts_with("not retained");
    let done = why.contains("retired") || why.starts_with("walked away") || why.starts_with("stepped away");
    let moved = why.starts_with("left for another job");
    let sc = s_coach(c);
    if fired {
        end(w, K::HotSeat, sc, "fired", None);
        end(w, K::VoteOfConfidence, sc, "fired_anyway", None);
        end(w, K::BuyoutWatch, sc, "paid_it", None);
        end(w, K::Rebuild, sc, "stalled", None);
        end(w, K::Graveyard, sc, "another_gone", None);
        end(w, K::NewCoach, sc, "rough_start", None);
    } else {
        end(w, K::HotSeat, sc, "resigned", None);
        end(w, K::VoteOfConfidence, sc, "meant_it", None);
    }
    if moved {
        end(w, K::NflWantsCoach, sc, "left", None);
        end(w, K::BigJobWantsCoach, sc, "left", None);
        end(w, K::ExtensionTalks, sc, "left", None);
        end(w, K::OldSchoolCalls, sc, "went_back", None);
        // Gone after a season or less.
        if let Some(s) = &stint
            && s.to - s.from <= 1
            && on(w, t)
        {
            let n = (s.to - s.from).max(1) as u32;
            let text =
                format!("{} leaves {} after {}.", cname(w, c), tname(w, t), crate::world::news::count(n, "season"));
            let s = coach_spot(w, K::QuickExit, c, t).fact(text, 4.0).public();
            tell(w, s);
        }
    }
    if done {
        end(w, K::CoachFarewell, sc, "went_out", None);
    }
    if !on(w, t) {
        return;
    }
    let due = search_due(w, t, d);
    let s = team_spot(w, K::CoachingSearch, t, s_season(t, year)).due(due).public();
    tell(w, s);
    // An interim coaches the games left.
    if let TeamKey::College(p) = t
        && let Some(i) = w.programs[p as usize].sideline
        && i != c
        && in_season(w, Level::College)
    {
        let s = coach_spot(w, K::InterimAudition, i, t).due(due).public();
        tell(w, s);
    }
    candidates(w, t, due);
    wobbles(w, t, d);
}

/// When a search decides: a college search's own day, about three weeks
/// for an NFL job, a month for a high school.
fn search_due(w: &World, t: TeamKey, d: Date) -> Date {
    match t {
        TeamKey::College(p) => w.head_search_due(p as usize).unwrap_or(d.add(21)),
        TeamKey::Nfl(_) => d.add(24),
        _ => d.add(30),
    }
}

fn head_hired(w: &mut World, t: TeamKey, c: usize) {
    let year = w.year;
    let prev = w.coaches[c].history.last().cloned();
    let here_before = prev.as_ref().is_some_and(|s| team_of_employer(s.employer) == t && s.to >= year - 1);
    let text = format!("{} names {} its head coach.", tname(w, t), cname(w, c));
    for id in about(w, K::CoachingSearch, t) {
        close(w, id, if here_before { "promoted" } else { "hired" }, Some(text.clone()));
    }
    for id in about(w, K::InterimAudition, t) {
        let got = w.storylines.get(id).and_then(|s| s.lead_coach()) == Some(c);
        close(w, id, if got { "got_job" } else { "passed_over" }, None);
    }
    for id in about(w, K::QuickExit, t) {
        close(w, id, "replaced", None);
    }
    // The other names in this job's rumors stay where they are.
    for kind in [K::NflWantsCoach, K::BigJobWantsCoach, K::OldSchoolCalls, K::CoordinatorHeadJob] {
        let k = kind.id();
        let ids: Vec<u32> = w
            .storylines
            .open
            .iter()
            .filter(|s| s.kind == k && s.phase.open() && s.other == Some(t) && s.lead_coach() != Some(c))
            .map(|s| s.id)
            .collect();
        for id in ids {
            close(w, id, "stayed", None);
        }
    }
    if prev.as_ref().is_some_and(|s| matches!(s.role, Role::OffensiveCoordinator | Role::DefensiveCoordinator)) {
        end(w, K::CoordinatorHeadJob, s_coach(c), "got_job", None);
    }
    hs_jump(w, t, c, prev.as_ref());
}

/// A high school head coach hired onto a college staff.
/// The story's team is the college; the school he left is the other side.
fn hs_jump(w: &mut World, t: TeamKey, c: usize, prev: Option<&crate::world::coach::Stint>) {
    let Some(Employer::HighSchool(h)) = prev.filter(|s| s.role == Role::HeadCoach).map(|s| s.employer) else {
        return;
    };
    if !matches!(t, TeamKey::College(_)) || !on(w, t) {
        return;
    }
    let s = coach_spot(w, K::HsCoachJump, c, t).other(TeamKey::Hs(h as u32)).due(Date::ymd(w.year + 1, 12, 1)).public();
    tell(w, s);
}

fn coordinator_left(w: &mut World, t: TeamKey, c: usize, d: Date) {
    let year = w.year;
    let why = last_stint(w, c, t).map_or("", |s| s.ended);
    if why.starts_with("fired") || why.starts_with("let go") {
        end(w, K::CoordinatorHeat, s_coach(c), "fired", None);
    }
    if !why.starts_with("left for another job") || !on(w, t) {
        return;
    }
    // Two of the top three gone to other jobs in one cycle: a raid.
    let e = employer(t);
    let gone = w
        .coaches
        .iter()
        .filter(|co| {
            co.history.last().is_some_and(|s| {
                s.employer == e
                    && s.to == year
                    && s.ended.starts_with("left for another job")
                    && matches!(s.role, Role::HeadCoach | Role::OffensiveCoordinator | Role::DefensiveCoordinator)
            })
        })
        .count();
    if gone >= 2 {
        let text = format!("{} has lost {} of its top coaches to other jobs.", tname(w, t), gone);
        let s = team_spot(w, K::StaffRaid, t, s_season(t, year)).due(d.add(45)).fact(text, 3.0).public();
        tell(w, s);
    }
}

fn coordinator_hired(w: &mut World, t: TeamKey, c: usize) {
    let prev = w.coaches[c].history.last().cloned();
    hs_jump(w, t, c, prev.as_ref());
}

/// The names in the rumors when a head job opens: a college star for an
/// NFL job, the hot coordinators, a winner at a smaller program for a big
/// job, a former coach for his old school. They start as talk.
fn candidates(w: &mut World, t: TeamKey, due: Date) {
    let year = w.year;
    let job = employer(t);
    let worth = crate::world::staff::employer_prestige(w, job);
    let big = match t {
        TeamKey::College(p) => {
            let g = &w.programs[p as usize];
            g.division == Division::FBS && g.prestige >= 0.6
        }
        TeamKey::Hs(h) => w.high_schools[h as usize].national,
        _ => false,
    };
    let state = match t {
        TeamKey::Hs(h) => Some(w.high_schools[h as usize].state),
        _ => None,
    };
    let mut heads: Vec<(f64, usize, TeamKey, f64)> = Vec::new();
    let mut coords: Vec<(f64, usize, TeamKey, f64)> = Vec::new();
    let mut old: Vec<(f64, usize, TeamKey, f64)> = Vec::new();
    for (c, co) in w.coaches.iter().enumerate() {
        if co.retired {
            continue;
        }
        let Some(j) = co.job else { continue };
        let at = team_of_employer(j.employer);
        if at == t || year - j.since < 1 {
            continue;
        }
        let here = crate::world::staff::employer_prestige(w, j.employer);
        let score = co.reputation + 3.0 * co.recent.iter().rev().take(2).sum::<f64>();
        let row = (score, c, at, here);
        match j.role {
            Role::HeadCoach => {
                if co.history.iter().any(|s| s.employer == job) && here <= worth + 0.05 && co.reputation >= 10.0 {
                    old.push(row);
                }
                heads.push(row);
            }
            Role::OffensiveCoordinator | Role::DefensiveCoordinator => coords.push(row),
            Role::PositionCoach(_) => {}
        }
    }
    let by = |v: &mut Vec<(f64, usize, TeamKey, f64)>| v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    by(&mut heads);
    by(&mut coords);
    by(&mut old);
    let mut picks: Vec<(K, (f64, usize, TeamKey, f64))> = Vec::new();
    match t {
        TeamKey::Nfl(_) => {
            let star = heads.iter().find(|x| {
                matches!(x.2, TeamKey::College(p) if w.programs[p as usize].division == Division::FBS)
                    && w.coaches[x.1].reputation >= 15.0
                    && year - w.coaches[x.1].job.map_or(year, |j| j.since) >= 2
            });
            if let Some(&x) = star
                && roll(w, 0x4F1C, x.1 as u64) < 0.4
            {
                picks.push((K::NflWantsCoach, x));
            }
            for &x in
                coords.iter().filter(|x| matches!(x.2, TeamKey::Nfl(_)) && w.coaches[x.1].reputation >= 12.0).take(2)
            {
                picks.push((K::CoordinatorHeadJob, x));
            }
        }
        TeamKey::College(_) if big => {
            for &x in heads
                .iter()
                .filter(|x| {
                    matches!(x.2, TeamKey::College(_)) && x.3 < worth - 0.05 && w.coaches[x.1].reputation >= 12.0
                })
                .take(2)
            {
                picks.push((K::BigJobWantsCoach, x));
            }
            if let Some(&x) = coords.iter().find(|x| w.coaches[x.1].reputation >= 12.0 && x.3 >= worth - 0.1) {
                picks.push((K::CoordinatorHeadJob, x));
            }
        }
        TeamKey::Hs(_) if big => {
            let local = heads.iter().find(|x| {
                matches!(x.2, TeamKey::Hs(h) if Some(w.high_schools[h as usize].state) == state)
                    && x.3 < worth
                    && w.coaches[x.1].reputation >= 10.0
            });
            if let Some(&x) = local {
                picks.push((K::BigJobWantsCoach, x));
            }
        }
        _ => {}
    }
    if let Some(&x) = old.first() {
        picks.push((K::OldSchoolCalls, x));
    }
    for (kind, (_, c, at, here)) in picks {
        if !on(w, at) && !matches!(at, TeamKey::Nfl(_)) {
            continue;
        }
        let s = Spot::new(kind, s_coach(c))
            .team(at)
            .other(t)
            .lead(Who::Coach(c as u32))
            .with(Who::Team(t), Part::Suitor)
            .with(Who::Boss(at), Part::Boss)
            .truth((worth - here + 0.1) as f32 * 2.0)
            .due(due);
        tell(w, s);
    }
}

/// Committed recruits of a program that just lost its head coach.
fn wobbles(w: &mut World, t: TeamKey, d: Date) {
    let TeamKey::College(p) = t else { return };
    let due = next_signing(w, d);
    let list: Vec<usize> = w
        .storylines
        .watch
        .recruits
        .iter()
        .filter(|(_, r)| r[0] == p + 1 && r[1] == 0 && r[2] >= 4)
        .filter_map(|(&id, _)| pi(w, id))
        .collect();
    for i in list {
        let s = recruit_spot(w, K::CommitWobble, i).team(t).due(due);
        tell(w, s);
    }
}

// --- Recruits --------------------------------------------------------------------

/// A recruit's story: him and the parent who decides with him.
fn recruit_spot(w: &World, kind: K, i: usize) -> Spot {
    let id = pid(w, i);
    Spot::new(kind, s_player(id)).lead(Who::Player(id)).with(Who::Parent(id), Part::Parent)
}

/// What the daily look compares: his commitment (program + 1), signed or
/// not, his stars, his high school + 1.
fn snapshot(p: &crate::world::Person) -> [u32; 4] {
    [p.cal.commit.map_or(0, |c| c.0 as u32 + 1), p.cal.signed.is_some() as u32, p.stars as u32, p.hs as u32 + 1]
}

/// The recruits the world follows, picked again each Monday: the coming
/// class's four- and five-stars, the next classes' five-stars; and the
/// stories a recruit starts with.
fn follow_recruits(w: &mut World, d: Date) {
    let class = crate::world::recruiting::next_class(w);
    let mut picked = std::collections::BTreeMap::new();
    let mut tops = Vec::new();
    let mut prodigies = Vec::new();
    let mut legacies = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        if p.stage != Stage::HighSchool || p.player.position.is_specialist() || p.class_of < class {
            continue;
        }
        let senior = p.class_of == class;
        if !(senior && p.stars >= 4) && !(p.class_of <= class + 2 && p.stars >= 5) {
            continue;
        }
        let id = p.player.id;
        picked.insert(id, w.storylines.watch.recruits.get(&id).copied().unwrap_or_else(|| snapshot(p)));
        let open = p.cal.commit.is_none() && p.cal.signed.is_none();
        if senior && open && p.stars >= 5 {
            tops.push(i);
        }
        if !senior && open && p.cal.early_offers.len() >= 8 {
            prodigies.push(i);
        }
        if senior
            && open
            && let Some(b) = p.family.older_brother_at
            && w.programs.get(b).is_some_and(|g| g.division == Division::FBS)
        {
            legacies.push((i, b));
        }
    }
    w.storylines.watch.recruits = picked;
    w.storylines.watch.recruits_on = Some(d);
    // His recruitment is a story until he decides: signing day keeps it
    // alive through a quiet month (it came back with the same opening).
    for i in tops {
        let s =
            recruit_spot(w, K::TopRecruit, i).team(TeamKey::Hs(w.people[i].hs as u32)).due(next_signing(w, d)).public();
        tell(w, s);
    }
    for i in prodigies {
        let n = w.people[i].cal.early_offers.len();
        let text = format!("{} already holds {} offers.", pname(w, i), n);
        let s = recruit_spot(w, K::Prodigy, i).team(TeamKey::Hs(w.people[i].hs as u32)).fact(text, 2.0).public();
        tell(w, s);
    }
    for (i, b) in legacies {
        let t = college(b);
        let s = recruit_spot(w, K::LegacyRecruit, i).team(t).with(Who::Team(t), Part::Suitor).due(next_signing(w, d));
        tell(w, s);
    }
}

fn recruits(w: &mut World, d: Date) {
    let monday = d.weekday() == crate::calendar::Weekday::Mon || w.storylines.watch.recruits_on.is_none();
    if monday {
        follow_recruits(w, d);
    }
    let list: Vec<(u32, [u32; 4])> = w.storylines.watch.recruits.iter().map(|(&k, &v)| (k, v)).collect();
    for (id, old) in list {
        let Some(i) = pi(w, id) else { continue };
        let now = snapshot(&w.people[i]);
        visits(w, i, d, now);
        if now != old {
            changed(w, i, old, now, d);
            w.storylines.watch.recruits.insert(id, now);
        }
    }
    // After the day's own look, so a signing seen today ends its story
    // the daily way first.
    if monday {
        settle_recruits(w);
    }
    signing_dues(w, d);
}

/// The recruiting stories the daily watch no longer sees through: a
/// recruit who signed after his class rolled over, whose stars fell, who
/// went to a prep school or left football. His signing or his college ends
/// them the way it went; a recruit gone from football lets them fade.
fn settle_recruits(w: &mut World) {
    let open: Vec<(u32, u16, Option<TeamKey>, u32)> = w
        .storylines
        .open
        .iter()
        .filter(|s| s.phase.open() && s.family() == Family::Recruiting && s.kind != K::HsTransfer.id())
        .filter_map(|s| Some((s.id, s.kind, s.team, s.lead_player()?)))
        .collect();
    let today = w.today;
    let mut told: Vec<u32> = Vec::new();
    for (id, kind, team, who) in open {
        let Some(i) = pi(w, who) else {
            fizzle(w, id, today);
            continue;
        };
        let p = &w.people[i];
        let (at, signed) = match (p.cal.signed, p.stage) {
            (Some(sg), _) => (Some(sg.program), true),
            (None, Stage::College) => (p.college, false),
            _ => (None, false),
        };
        let Some(c) = at else {
            if !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) {
                fizzle(w, id, today);
            }
            continue;
        };
        let t = college(c);
        let in_state = w.programs.get(c).is_some_and(|g| g.state == p.home_state);
        let up = w.programs.get(c).is_some_and(|g| g.division == Division::FBS && g.prestige >= 0.55);
        let won = team == Some(t);
        let ending = match K::ALL.get(kind as usize) {
            Some(K::TopRecruit | K::Prodigy) => "committed",
            Some(K::RecruitWar) => {
                if won {
                    "won"
                } else {
                    "lost"
                }
            }
            Some(K::HometownLeaving) => {
                if in_state {
                    "stayed"
                } else {
                    "left"
                }
            }
            Some(K::LegacyRecruit) => {
                if won {
                    "followed"
                } else {
                    "own_way"
                }
            }
            Some(K::CommitWobble) => {
                if won {
                    "held"
                } else {
                    "decommitted"
                }
            }
            Some(K::SigningFlip) => {
                if won {
                    "held"
                } else {
                    "flipped"
                }
            }
            Some(K::LateRiser) => {
                if up {
                    "signed_up"
                } else {
                    "signed"
                }
            }
            _ => continue,
        };
        // His stories end together: the first tells where he went, the
        // others in their own words.
        let fact = (!told.contains(&who)).then(|| {
            if signed {
                format!("{} signs with {}.", pname(w, i), tname(w, t))
            } else {
                format!("{} enrolls at {}.", pname(w, i), tname(w, t))
            }
        });
        told.push(who);
        close(w, id, ending, fact);
    }
}

/// The program a commitment code names.
fn prog(x: u32) -> Option<TeamKey> {
    (x > 0).then(|| TeamKey::College(x - 1))
}

fn changed(w: &mut World, i: usize, old: [u32; 4], now: [u32; 4], d: Date) {
    let name = pname(w, i);
    // A new high school.
    if old[3] != now[3] && old[3] != 0 {
        let (from, to) = (TeamKey::Hs(old[3] - 1), TeamKey::Hs(now[3] - 1));
        let due = opener(w, to).unwrap_or(Date::ymd(d.year(), 8, 25).max(d.add(30)));
        let text = format!("{name} transfers from {} to {}.", tname(w, from), tname(w, to));
        let s = recruit_spot(w, K::HsTransfer, i).team(to).other(from).due(due).fact(text, 3.0).public();
        tell(w, s);
    }
    // A late rise in his senior year.
    let senior_fall = d.month() >= 9 || d.month() <= 2;
    let class = crate::world::recruiting::next_class(w);
    if now[2] >= 4 && old[2] <= 3 && now[1] == 0 && senior_fall && w.people[i].class_of == class {
        let s = recruit_spot(w, K::LateRiser, i)
            .nums([old[2] as i32, now[2] as i32, 0, 0])
            .due(next_signing(w, d))
            .public();
        tell(w, s);
    }
    if old[1] == 0 && now[1] == 1 {
        signed(w, i, prog(old[0]));
        return;
    }
    if old[0] == now[0] || now[1] == 1 {
        return;
    }
    match (prog(old[0]), prog(now[0])) {
        (None, Some(t)) => {
            let text = format!("{name} commits to {}.", tname(w, t));
            decided(w, i, t, Some(text));
        }
        (Some(a), None) => {
            let sp = s_player(pid(w, i));
            let mut fact = Some(format!("{name} decommits from {}.", tname(w, a)));
            end_once(w, K::CommitWobble, sp, "decommitted", &mut fact);
            end_once(w, K::SigningFlip, sp, "flipped", &mut fact);
            if w.people[i].stars >= 4 {
                // On the market again: the decommitment, or in other words
                // when a story already told it.
                let text = fact.unwrap_or_else(|| format!("{name} is back on the market."));
                let s =
                    recruit_spot(w, K::TopRecruit, i).team(TeamKey::Hs(w.people[i].hs as u32)).fact(text, 4.0).public();
                tell(w, s);
            }
        }
        (Some(a), Some(b)) => {
            let sp = s_player(pid(w, i));
            let text = format!("{name} flips from {} to {}.", tname(w, a), tname(w, b));
            let mut fact = Some(text.clone());
            end_once(w, K::CommitWobble, sp, "decommitted", &mut fact);
            match open_id(w, K::SigningFlip, sp) {
                Some(id) => close(w, id, "flipped", fact.take()),
                None if fact.is_some() && w.people[i].stars >= 4 && d.days_to(next_signing(w, d)) <= 30 => {
                    // A flip nobody saw coming: told as it happens.
                    let s = recruit_spot(w, K::SigningFlip, i)
                        .team(a)
                        .other(b)
                        .with(Who::Team(b), Part::Suitor)
                        .fact(text.clone(), 6.0)
                        .public();
                    if let Some(id) = tell(w, s) {
                        fact = None;
                        close(w, id, "flipped", None);
                    }
                }
                None => {}
            }
            decided(w, i, b, fact);
        }
        (None, None) => {}
    }
}

/// A recruit made up his mind: the stories about where he would go end, the
/// first with the fact (`fact`, None when a story already told it) and the
/// others in their own words.
fn decided(w: &mut World, i: usize, t: TeamKey, mut fact: Option<String>) {
    let sp = s_player(pid(w, i));
    end_once(w, K::TopRecruit, sp, "committed", &mut fact);
    end_once(w, K::Prodigy, sp, "committed", &mut fact);
    let home = w.people[i].home_state;
    let in_state = match t {
        TeamKey::College(p) => w.programs.get(p as usize).is_some_and(|g| g.state == home),
        _ => false,
    };
    for (kind, yes, no) in [
        (K::RecruitWar, "won", "lost"),
        (K::HometownLeaving, "stayed", "left"),
        (K::LegacyRecruit, "followed", "own_way"),
    ] {
        let Some(id) = open_id(w, kind, sp) else { continue };
        let (team, other) = w.storylines.get(id).map_or((None, None), |s| (s.team, s.other));
        let good = match kind {
            K::HometownLeaving => in_state,
            _ => team == Some(t),
        };
        // A war lost to a third school: neither side's line fits.
        let third = kind == K::RecruitWar && !good && other != Some(t);
        let text = match (fact.take(), third, team, other) {
            (Some(f), _, _, _) => Some(f),
            (None, true, Some(a), Some(b)) => Some(format!(
                "Neither {} nor {} gets {}: he picks {}.",
                tname(w, a),
                tname(w, b),
                pname(w, i),
                tname(w, t)
            )),
            _ => None,
        };
        close(w, id, if good { yes } else { no }, text);
    }
}

fn signed(w: &mut World, i: usize, was: Option<TeamKey>) {
    let Some(sg) = w.people[i].cal.signed else { return };
    let t = college(sg.program);
    let sp = s_player(pid(w, i));
    let held = was == Some(t);
    let mut fact = Some(format!("{} signs with {}.", pname(w, i), tname(w, t)));
    end_once(w, K::SigningFlip, sp, if held { "held" } else { "flipped" }, &mut fact);
    end_once(w, K::CommitWobble, sp, if held { "held" } else { "decommitted" }, &mut fact);
    let up = w.programs.get(sg.program).is_some_and(|g| g.division == Division::FBS && g.prestige >= 0.55);
    end_once(w, K::LateRiser, sp, if up { "signed_up" } else { "signed" }, &mut fact);
    decided(w, i, t, fact);
}

/// Official visits: a fact for his stories, a war between two powers, the
/// hometown school's worry, a committed recruit looking around.
fn visits(w: &mut World, i: usize, d: Date, now: [u32; 4]) {
    let today: Vec<usize> =
        w.people[i].cal.visits.iter().filter(|v| v.1 == d && v.2 == Visit::Official).map(|v| v.0).collect();
    if today.is_empty() {
        return;
    }
    let sp = s_player(pid(w, i));
    let stars = w.people[i].stars;
    for j in today {
        let t = college(j);
        let text = format!("{} takes an official visit to {}.", pname(w, i), tname(w, t));
        for kind in [K::TopRecruit, K::RecruitWar, K::HometownLeaving, K::LegacyRecruit, K::Prodigy, K::SigningFlip] {
            note(w, kind, sp, text.clone());
        }
        if stars < 4 || now[1] == 1 {
            continue;
        }
        if now[0] == 0 {
            // A war: his second official visit to a power.
            let mut powers: Vec<usize> = Vec::new();
            for v in &w.people[i].cal.visits {
                let power = w.programs.get(v.0).is_some_and(|g| g.division == Division::FBS && g.prestige >= 0.6);
                if v.2 == Visit::Official && v.1 <= d && power && !powers.contains(&v.0) {
                    powers.push(v.0);
                }
            }
            if powers.len() >= 2 && powers.last() == Some(&j) && open_id(w, K::RecruitWar, sp).is_none() {
                let a = college(powers[0]);
                let s = recruit_spot(w, K::RecruitWar, i)
                    .team(a)
                    .other(t)
                    .with(Who::Team(a), Part::Suitor)
                    .with(Who::Team(t), Part::Suitor)
                    .due(next_signing(w, d))
                    .public();
                tell(w, s);
            }
            // The state school that offered him watches him visit away.
            let home = w.people[i].home_state;
            let away = w.programs.get(j).is_some_and(|g| g.state != home);
            if away && let Some(f) = flagship(w, home) {
                let p = &w.people[i];
                let offered = p.cal.offers.contains(&f) || p.cal.early_offers.iter().any(|x| x.0 == f);
                if offered {
                    let h = college(f);
                    let s = recruit_spot(w, K::HometownLeaving, i)
                        .team(h)
                        .other(t)
                        .with(Who::Team(h), Part::Suitor)
                        .due(next_signing(w, d));
                    tell(w, s);
                }
            }
        } else if now[0] != j as u32 + 1 && d.days_to(next_signing(w, d)) <= 45 {
            let a = TeamKey::College(now[0] - 1);
            let s = recruit_spot(w, K::SigningFlip, i)
                .team(a)
                .other(t)
                .with(Who::Team(t), Part::Suitor)
                .due(next_signing(w, d));
            tell(w, s);
        }
    }
}

/// A state's flagship: its most prestigious FBS program.
fn flagship(w: &World, state: usize) -> Option<usize> {
    w.programs
        .iter()
        .filter(|g| g.state == state && g.division == Division::FBS)
        .max_by(|a, b| a.prestige.total_cmp(&b.prestige).then(b.id.cmp(&a.id)))
        .map(|g| g.id)
}

/// Signing day passed without a signature: the wobble held or not.
fn signing_dues(w: &mut World, d: Date) {
    for kind in [K::CommitWobble, K::SigningFlip, K::LateRiser] {
        let k = kind.id();
        let due: Vec<(u32, Option<u32>, Option<TeamKey>)> = w
            .storylines
            .open
            .iter()
            .filter(|s| s.kind == k && s.phase.open() && s.due.is_some_and(|x| x.add(3) < d))
            .map(|s| (s.id, s.lead_player(), s.team))
            .collect();
        for (id, lead, team) in due {
            let commit = lead.and_then(|x| pi(w, x)).and_then(|i| w.people[i].cal.commit).map(|c| college(c.0));
            let ending = match kind {
                K::LateRiser => "signed",
                K::SigningFlip => {
                    if commit == team {
                        "held"
                    } else {
                        "flipped"
                    }
                }
                _ => {
                    if commit == team {
                        "held"
                    } else {
                        "decommitted"
                    }
                }
            };
            close(w, id, ending, None);
        }
    }
}

// --- The portal --------------------------------------------------------------------

fn portal(w: &mut World, d: Date) {
    let year = w.year;
    if w.storylines.watch.exodus_year != year {
        w.storylines.watch.exodus.clear();
        w.storylines.watch.exodus_year = year;
    }
    let mut fresh: Vec<usize> = Vec::new();
    for &i in &w.portal_pending {
        let Some(p) = w.people.get(i) else { continue };
        let Some(e) = p.cal.portal else { continue };
        let f = w.storylines.watch.flags.get(&p.player.id).copied().unwrap_or(0);
        if e.entered <= d && f & F_PORTAL == 0 {
            fresh.push(i);
        }
    }
    for i in fresh {
        let id = pid(w, i);
        *w.storylines.watch.flags.entry(id).or_default() |= F_PORTAL;
        let Some(c) = w.people[i].college else { continue };
        let t = college(c);
        if !on(w, t) {
            continue;
        }
        let starter = w.people[i].last_role == Some(PlayingTime::Starter);
        let decide = w.people[i].cal.portal.map_or(d.add(30), |e| e.decide_by);
        if notable_college(w, i) {
            let text = format!("{} enters the transfer portal.", pname(w, i));
            let s = player_spot(w, K::PortalDecision, i).due(decide).fact(text, 3.0).public();
            tell(w, s);
        }
        let n = {
            let e = w.storylines.watch.exodus.entry(c as u32).or_default();
            e[0] += 1;
            if starter {
                e[1] += 1;
            }
            *e
        };
        let line = if w.programs[c].division == Division::FBS { 18 } else { 15 };
        if n[0] >= line || n[1] >= 5 {
            let mut s = team_spot(w, K::PortalExodus, t, s_season(t, year))
                .nums([n[0] as i32, n[1] as i32, 0, 0])
                .due(decide.add(14))
                .public();
            if n[0] == line || n[0] % 5 == 0 || (starter && n[1] >= 5) {
                let text = format!(
                    "{} has lost {} to the portal this offseason, {} of them starters.",
                    tname(w, t),
                    crate::world::news::count(n[0], "player"),
                    n[1]
                );
                s = s.fact(text, 2.0);
            }
            tell(w, s);
        }
    }
    // Decided: landed, back home, or nowhere.
    let flagged: Vec<u32> =
        w.storylines.watch.flags.iter().filter(|(_, f)| **f & F_PORTAL != 0).map(|(&id, _)| id).collect();
    for id in flagged {
        let i = pi(w, id);
        if i.is_some_and(|i| w.people[i].cal.portal.is_some()) {
            continue;
        }
        if let Some(f) = w.storylines.watch.flags.get_mut(&id) {
            *f &= !F_PORTAL;
            if *f == 0 {
                w.storylines.watch.flags.remove(&id);
            }
        }
        let Some(sid) = open_id(w, K::PortalDecision, s_player(id)) else { continue };
        let from = w.storylines.get(sid).and_then(|s| s.team);
        let (ending, text) = match i {
            Some(i) if w.people[i].stage == Stage::College => {
                let now = w.people[i].college.map(college);
                if now == from {
                    let text = format!("{} withdraws from the portal and stays.", pname(w, i));
                    ("returned", Some(text))
                } else {
                    let to = now.map_or_else(String::new, |t| tname(w, t));
                    ("landed", Some(format!("{} transfers to {to}.", pname(w, i))))
                }
            }
            _ => ("unsigned", None),
        };
        close(w, sid, ending, text);
    }
}

// --- Players: holdouts, trade requests, suspensions, tags, opt-outs, trouble -------

/// A player's story flags now (the portal's is kept by `portal`).
fn flags_now(w: &World, i: usize, d: Date) -> u8 {
    let p = &w.people[i];
    let mut f = 0;
    if let Some(c) = p.pro.as_ref() {
        if c.hold != Hold::None {
            f |= F_HOLDOUT;
        }
        if c.wants_out {
            f |= F_WANTS_OUT;
        }
        if c.suspended > 0 {
            f |= F_SUSPENDED;
        }
        if c.contract
            .as_ref()
            .is_some_and(|k| matches!(k.kind, Deal::Franchise | Deal::Transition) && k.signed == w.year)
        {
            f |= F_TAGGED;
        }
    }
    if p.cal.opted_out && p.cal.opt_out_on.is_none_or(|x| x <= d) {
        f |= F_OPTED;
    }
    if p.incident {
        f |= F_INCIDENT;
    }
    f
}

fn players(w: &mut World, d: Date, teams: &[TeamKey]) {
    let mut look: Vec<usize> = Vec::new();
    for c in &w.clubs {
        look.extend(c.roster.iter().copied());
    }
    for &t in teams {
        match t {
            TeamKey::College(p) => look.extend(w.programs[p as usize].roster.iter().copied()),
            TeamKey::Hs(h) => look.extend(w.high_schools[h as usize].roster.iter().copied()),
            _ => {}
        }
    }
    look.sort_unstable();
    look.dedup();
    let mut seen: Vec<u32> = Vec::with_capacity(look.len());
    let mut rises: Vec<(usize, u8)> = Vec::new();
    let mut falls: Vec<(u32, u8)> = Vec::new();
    for &i in &look {
        let id = pid(w, i);
        seen.push(id);
        let now = flags_now(w, i, d);
        let stored = w.storylines.watch.flags.get(&id).copied().unwrap_or(0);
        let old = stored & !F_PORTAL;
        if now == old {
            continue;
        }
        if now & !old != 0 {
            rises.push((i, now & !old));
        }
        if old & !now != 0 {
            falls.push((id, old & !now));
        }
        let new = (stored & F_PORTAL) | now;
        if new == 0 {
            w.storylines.watch.flags.remove(&id);
        } else {
            w.storylines.watch.flags.insert(id, new);
        }
    }
    // Gone from every roster: their flags fall too.
    seen.sort_unstable();
    let stale: Vec<(u32, u8)> = w
        .storylines
        .watch
        .flags
        .iter()
        .filter(|(id, f)| **f & !F_PORTAL != 0 && seen.binary_search(id).is_err())
        .map(|(&id, &f)| (id, f))
        .collect();
    for (id, f) in stale {
        falls.push((id, f & !F_PORTAL));
        if f & F_PORTAL == 0 {
            w.storylines.watch.flags.remove(&id);
        } else {
            w.storylines.watch.flags.insert(id, F_PORTAL);
        }
    }
    for (i, f) in rises {
        rose(w, i, f, d);
    }
    for (id, f) in falls {
        fell(w, id, f);
    }
}

/// The day a suspended pro plays again.
fn return_day(w: &World, games: u32, d: Date) -> Date {
    let from = if d >= w.dates.nfl_kickoff && d <= w.dates.super_bowl { d } else { w.dates.nfl_kickoff };
    from.add(7 * games as i32)
}

fn rose(w: &mut World, i: usize, f: u8, d: Date) {
    let name = pname(w, i);
    let Some(t) = team_of(w, i) else { return };
    let id = pid(w, i);
    if f & F_HOLDOUT != 0 && notable_pro(w, i) {
        let out = w.people[i].pro.as_ref().is_some_and(|c| c.hold == Hold::Out);
        let how = if out { "is holding out" } else { "is holding in at camp" };
        let s = player_spot(w, K::Holdout, i)
            .due(w.dates.nfl_kickoff.max(d.add(14)))
            .fact(format!("{name} {how}, wanting a new deal."), 4.0)
            .public();
        tell(w, s);
    }
    if f & F_WANTS_OUT != 0 && notable_pro(w, i) {
        let deadline = w.dates.trade_deadline.date;
        let due = if d < deadline { deadline } else { Date::ymd(w.year + 1, 3, 15) };
        let s = player_spot(w, K::TradeRequest, i)
            .due(due)
            .fact(format!("{name} asks {} for a trade.", tname(w, t)), 4.0)
            .public();
        tell(w, s);
    }
    if f & F_SUSPENDED != 0 && notable_pro(w, i) {
        let games = w.people[i].pro.as_ref().map_or(0, |c| c.suspended);
        let appeal =
            w.folk.courts.open.iter().find(|c| c.id == id && matches!(c.step, Step::Appeal { .. })).map(|c| c.next);
        let s = match appeal {
            Some(next) => player_spot(w, K::LeagueSuspension, i)
                .nums([games as i32, 0, 0, 0])
                .due(next)
                .fact(
                    format!("The league suspends {name} {}; he appeals.", crate::world::news::count(games, "game")),
                    4.0,
                )
                .public(),
            None => player_spot(w, K::SuspensionReturn, i)
                .nums([games as i32, 0, 0, 0])
                .due(return_day(w, games, d))
                .fact(format!("The league suspends {name} {}.", crate::world::news::count(games, "game")), 3.0)
                .public(),
        };
        tell(w, s);
    }
    if f & F_TAGGED != 0 {
        let text = format!("{} puts the franchise tag on {name}.", tname(w, t));
        end(w, K::ContractYear, s_player(id), "tagged", Some(text));
        let deadline = w.dates.tag_long_term_deadline.date;
        if notable_pro(w, i) && d < deadline {
            let s = player_spot(w, K::TagStandoff, i).due(deadline).public();
            tell(w, s);
        }
    }
    if f & F_OPTED != 0 && notable_college(w, i) && on(w, t) {
        let text = format!("{name} opts out of the bowl game to get ready for the draft.");
        let s = player_spot(w, K::OptOut, i).fact(text, 3.0).public();
        tell(w, s);
    }
    if f & F_INCIDENT != 0 && on(w, t) && !matches!(t, TeamKey::Nfl(_)) && notable_college(w, i) {
        // The news comes out over camp and the first weeks.
        let until = opener(w, t).unwrap_or(d.add(21)).add(14);
        let span = d.days_to(until).max(1);
        let when = d.add((roll(w, 0x1C1D, id as u64) * span as f64) as i32);
        w.storylines.watch.incidents.push((when, id));
    }
}

fn fell(w: &mut World, id: u32, f: u8) {
    let i = pi(w, id);
    let sp = s_player(id);
    let now_team = i.and_then(|i| team_of(w, i));
    let deal = |w: &World, team: Option<TeamKey>| {
        i.and_then(|i| w.people[i].pro.as_ref()).and_then(|c| c.contract.as_ref()).is_some_and(|k| {
            k.signed == w.year
                && Some(TeamKey::Nfl(k.team as u16)) == team
                && matches!(k.kind, Deal::Extension | Deal::Veteran)
        })
    };
    if f & F_HOLDOUT != 0
        && let Some(sid) = open_id(w, K::Holdout, sp)
    {
        let team = w.storylines.get(sid).and_then(|s| s.team);
        let ending = if now_team != team {
            "traded"
        } else if deal(w, team) {
            "paid"
        } else {
            "reported"
        };
        close(w, sid, ending, None);
    }
    if f & F_WANTS_OUT != 0
        && let Some(sid) = open_id(w, K::TradeRequest, sp)
    {
        let team = w.storylines.get(sid).and_then(|s| s.team);
        let ending = match now_team {
            None => "released",
            t if t != team => "traded",
            _ => "made_up",
        };
        close(w, sid, ending, None);
    }
    if f & F_TAGGED != 0
        && let Some(sid) = open_id(w, K::TagStandoff, sp)
    {
        let team = w.storylines.get(sid).and_then(|s| s.team);
        if deal(w, team) {
            close(w, sid, "long_deal", None);
        }
    }
}

// --- Trades ----------------------------------------------------------------------

fn trades(w: &mut World, d: Date) {
    let today: Vec<(u32, usize, usize)> = w
        .market
        .picks
        .trades
        .iter()
        .rev()
        .take_while(|t| t.date >= d)
        .filter(|t| t.date == d)
        .map(|t| (t.player, t.buyer, t.seller))
        .collect();
    for (id, buyer, seller) in today {
        let sp = s_player(id);
        end(w, K::TradeRequest, sp, "traded", None);
        end(w, K::Holdout, sp, "traded", None);
        let Some(i) = pi(w, id) else { continue };
        let star =
            pro_bowler(w, id, 3) || w.people[i].pro.as_ref().is_some_and(|c| c.last_grade.unwrap_or(0.0) >= 80.0);
        if !star {
            continue;
        }
        let (to, from) = (TeamKey::Nfl(buyer as u16), TeamKey::Nfl(seller as u16));
        let text = format!("{} trade {} to {}.", tname(w, from), pname(w, i), tname(w, to));
        let mut s = Spot::new(K::StarTraded, sp).team(to).other(from).lead(Who::Player(id)).fact(text, 5.0).public();
        if let Some(c) = super::head(w, to) {
            s = s.with(Who::Coach(c as u32), Part::Decider);
        }
        tell(w, s);
    }
}

// --- The courts ------------------------------------------------------------------

/// How a closed case came out, from its words.
fn outcome(text: &str) -> Option<&'static str> {
    if text.contains("upheld") {
        Some("upheld")
    } else if text.contains("cut") {
        Some("reduced")
    } else if text.contains("declined") || text.contains("dropped") || text.contains("acquitted") {
        Some("cleared")
    } else if text.contains("pleaded") || text.contains("convicted") {
        Some("convicted")
    } else {
        None
    }
}

fn courts(w: &mut World, d: Date) {
    // New arrests in the courts: every pro's, your own players'.
    let fresh: Vec<(u32, String)> = w
        .folk
        .courts
        .open
        .iter()
        .filter(|c| c.opened == d && c.step == Step::Arrest)
        .map(|c| (c.id, c.what.clone()))
        .collect();
    for (id, what) in fresh {
        let Some(i) = pi(w, id) else { continue };
        let Some(t) = team_of(w, i) else { continue };
        if !matches!(t, TeamKey::Nfl(_)) && !mine(w, t) {
            continue;
        }
        // A second arrest is told as one.
        let again = w
            .storylines
            .open
            .iter()
            .chain(&w.storylines.ended)
            .any(|s| s.kind == K::Arrest.id() && s.lead_player() == Some(id));
        let text = format!("{} is arrested{} ({what}).", pname(w, i), if again { " again" } else { "" });
        let s = player_spot(w, K::Arrest, i).nums([0, 0, 0, 1]).word(what).fact(text, 5.0).public();
        tell(w, s);
    }
    // Cases decided.
    for kind in [K::Arrest, K::LeagueSuspension] {
        let k = kind.id();
        let list: Vec<(u32, u32)> = w
            .storylines
            .open
            .iter()
            .filter(|s| s.kind == k && s.phase.open() && (kind == K::LeagueSuspension || s.nums[3] != 0))
            .filter_map(|s| s.lead_player().map(|p| (s.id, p)))
            .collect();
        for (sid, id) in list {
            let appeal = |c: &crate::world::folk::courts::Case| matches!(c.step, Step::Appeal { .. });
            let pending = w.folk.courts.open.iter().any(|c| c.id == id && (kind == K::LeagueSuspension) == appeal(c));
            if pending {
                continue;
            }
            let Some(c) = w.folk.courts.closed.iter().rev().find(|c| c.id == id) else { continue };
            let text = c.outcome.clone().unwrap_or_default();
            let ending = match (kind, outcome(&text)) {
                (K::LeagueSuspension, Some(e @ ("upheld" | "reduced"))) => e,
                (K::Arrest, Some(e @ ("cleared" | "convicted"))) => e,
                _ => continue,
            };
            close(w, sid, ending, (!text.is_empty()).then_some(text));
        }
    }
}

// --- Life ------------------------------------------------------------------------

/// A death in the family of a player the press follows.
fn life(w: &mut World, d: Date) {
    let deaths: Vec<u32> = w
        .folk
        .life
        .events
        .iter()
        .filter(|e| e.date == d && e.event == crate::world::folk::life::Event::Death)
        .map(|e| e.id)
        .collect();
    for id in deaths {
        let Some(i) = pi(w, id) else { continue };
        let Some(t) = team_of(w, i) else { continue };
        if !on(w, t) || !(mine(w, t) || notable_pro(w, i)) {
            continue;
        }
        let another = if super::ran(w, K::FamilyLoss, s_player(id), None) { "another" } else { "a" };
        let text = format!("{} is away from {} after {another} death in his family.", pname(w, i), tname(w, t));
        let s = player_spot(w, K::FamilyLoss, i).due(d.add(7)).fact(text, 3.0).public();
        tell(w, s);
    }
}

/// The season's off-field trouble, as it comes out: an arrest, a team
/// rule broken, grades. Whether he was dismissed or suspended is already
/// decided; the story's answer comes about ten days later.
fn incidents(w: &mut World, d: Date) {
    if w.storylines.watch.incidents.is_empty() {
        return;
    }
    let due: Vec<u32> = w.storylines.watch.incidents.iter().filter(|x| x.0 <= d).map(|x| x.1).collect();
    w.storylines.watch.incidents.retain(|x| x.0 > d);
    for id in due {
        let Some(i) = pi(w, id) else { continue };
        let p = &w.people[i];
        let what = p
            .notes
            .iter()
            .rev()
            .find_map(|n| n.strip_prefix("Off-field trouble at ").and_then(|r| r.split(": ").nth(1)))
            .map(|s| s.trim_end_matches('.').to_string());
        let Some(what) = what else { continue };
        let dismissed = p.notes.iter().rev().take(4).any(|n| n.starts_with("Dismissed from his team"));
        let kind = if what.contains("academic") || what.contains("grades") {
            K::Academics
        } else if what.contains("team rules")
            || what.contains("drug test")
            || what.contains("at school")
            || what.contains("benefits")
        {
            K::TeamRules
        } else {
            K::Arrest
        };
        let name = pname(w, i);
        let text = match kind {
            K::Academics => format!("{name} is ruled ineligible over {what}."),
            _ => format!("{name} is suspended indefinitely after {what}."),
        };
        let s = player_spot(w, kind, i)
            .nums([0, 0, if dismissed { 1 } else { 2 }, 0])
            .word(what)
            .due(d.add(10))
            .fact(text, 4.0)
            .public();
        tell(w, s);
    }
}

// --- The dated days --------------------------------------------------------------

fn dated(w: &mut World, d: Date) {
    let year = w.year;
    if d == w.dates.heisman {
        heisman_night(w);
    }
    if d == w.dates.nfl_honors {
        mvp_night(w);
    }
    if d == w.dates.hof_finalists {
        hall_finalists(w);
    }
    if d == w.dates.super_bowl.add(-1) {
        hall_class(w);
    }
    if d == w.dates.black_monday && w.storylines.once(38, year) {
        black_monday(w);
    }
    if d == w.dates.super_bowl && w.storylines.once(37, year) {
        super::weekly::season_close(w, Level::Nfl);
    }
    for k in 0..3 {
        if d == w.dates.draft[k].date {
            draft_day(w, k, d);
        }
    }
    let tampering = w.dates.tampering_open.map_or(w.dates.league_year.date.add(-2), |m| m.date);
    if d == tampering {
        free_agents(w);
    }
    class_races(w, d);
}

/// The Heisman: the favourite won it or not.
fn heisman_night(w: &mut World) {
    let year = w.year;
    let Some(id) = open_id(w, K::AwardRace, subj(0xA11A, year as u64)) else { return };
    let winner = w
        .college
        .as_ref()
        .and_then(|c| c.heisman_vote.iter().max_by_key(|r| (r.points, std::cmp::Reverse(r.id))))
        .map(|r| r.id);
    let Some(win) = winner else { return };
    let lead = w.storylines.get(id).and_then(|s| s.lead_player());
    let name = pi(w, win).map_or_else(String::new, |i| pname(w, i));
    close(w, id, if lead == Some(win) { "won" } else { "lost" }, Some(format!("{name} wins the Heisman Trophy.")));
}

fn mvp_night(w: &mut World) {
    let year = w.year;
    let Some(id) = open_id(w, K::AwardRace, subj(0xA11B, year as u64)) else { return };
    let Some((win, name)) = w.nfl_book.awards.iter().find(|a| a.0 == year && a.1 == "MVP").map(|a| (a.2, a.3.clone()))
    else {
        return;
    };
    let lead = w.storylines.get(id).and_then(|s| s.lead_player());
    close(w, id, if lead == Some(win) { "won" } else { "lost" }, Some(format!("{name} is the league's MVP.")));
}

/// The Hall of Fame's finalists: the strongest cases on the ballot.
fn hall_finalists(w: &mut World) {
    let year = w.year;
    let mut v: Vec<(f64, u32, String)> = w
        .nfl_book
        .candidates
        .iter()
        .filter(|c| year - c.retired >= 6 && year - c.retired <= 25)
        .map(|c| (c.score + 0.6 * (year - c.retired - 6) as f64, c.id, c.name.clone()))
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let due = w.dates.super_bowl.add(-1);
    for (score, id, name) in v.into_iter().take(7) {
        if score < 16.0 {
            break;
        }
        let team = w.nfl_book.pro_bowls.iter().rev().find(|x| x.1 == id).map(|x| x.2);
        let mut s = Spot::new(K::HallCall, s_player(id)).lead(Who::Player(id)).word(name).due(due).public();
        if let Some(n) = team {
            s = s.team(TeamKey::Nfl(n as u16));
        }
        tell(w, s);
    }
}

fn hall_class(w: &mut World) {
    let year = w.year;
    for (id, subject) in open_of(w, K::HallCall) {
        let pid = subject as u32;
        let inducted = w.nfl_book.hall.iter().any(|h| h.0 == year && h.1 == pid);
        close(w, id, if inducted { "inducted" } else { "waits" }, None);
    }
}

/// The regular season's end: who made the playoffs, who won the division.
fn black_monday(w: &mut World) {
    let Some(s) = w.nfl_season.as_ref() else { return };
    let (seed, div) = (s.seed.clone(), s.div.clone());
    let winner = |dv: usize| {
        (0..seed.len()).filter(|&t| div.get(t) == Some(&dv)).filter_map(|t| seed[t].map(|x| (x, t))).min().map(|x| x.1)
    };
    for (id, _) in open_of(w, K::PlayoffPush) {
        let Some(TeamKey::Nfl(n)) = w.storylines.get(id).and_then(|s| s.team) else { continue };
        let made = seed.get(n as usize).copied().flatten().is_some();
        close(w, id, if made { "in" } else { "out" }, None);
    }
    for (id, subject) in open_of(w, K::ConferenceRace) {
        let Some(TeamKey::Nfl(n)) = w.storylines.get(id).and_then(|s| s.team) else { continue };
        let dv = ((subject >> 32) as usize).wrapping_sub(0xD1F0);
        close(w, id, if winner(dv) == Some(n as usize) { "won" } else { "lost" }, None);
    }
}

/// A draft day: the first pick, the slides, and the stories a pick answers.
fn draft_day(w: &mut World, k: usize, d: Date) {
    let year = d.year();
    let pick = |w: &World, id: u32| -> Option<u32> {
        let i = pi(w, id)?;
        let (dy, r, _, _) = w.people[i].pro.as_ref()?.draft?;
        (dy == year).then_some(r)
    };
    if k == 0 {
        for (id, _) in open_of(w, K::TopPickDebate) {
            let Some(s) = w.storylines.get(id) else { continue };
            let first = |x: Option<u32>| {
                x.and_then(|id| pi(w, id))
                    .and_then(|i| w.people[i].pro.as_ref())
                    .and_then(|c| c.draft)
                    .is_some_and(|x| x.0 == year && x.2 == 1)
            };
            let lead = first(s.lead_player());
            close(w, id, if lead { "lead_first" } else { "rival_first" }, None);
        }
        // The best prospects still waiting after the first round.
        let top = crate::world::scouting::public::consensus(w).unwrap_or_default();
        let mut slides = Vec::new();
        for (r, &id) in top.iter().enumerate().take(15) {
            if pick(w, id).is_none()
                && let Some(i) = pi(w, id)
                && w.people[i].stage == Stage::Declared
            {
                slides.push((r + 1, i));
            }
        }
        for (rank, i) in slides {
            let text =
                format!("{}, No. {rank} on the consensus board, is still there after the first round.", pname(w, i));
            let mut s = Spot::new(K::DraftSlide, s_player(pid(w, i)))
                .lead(Who::Player(pid(w, i)))
                .due(w.dates.draft[1].date)
                .fact(text, 5.0)
                .public();
            if let Some(c) = w.people[i].college {
                s = s.team(college(c));
            }
            tell(w, s);
        }
    }
    // Rounds done by the end of today.
    let done = [1, 3, 7][k];
    let bound: [(K, u32, &str, &str); 7] = [
        (K::StockRise, 2, "drafted_high", "cooled"),
        (K::CombineRiser, 2, "drafted_high", "cooled"),
        (K::StockFall, 2, "recovered", "fell"),
        (K::RedFlag, 4, "drafted", "fell"),
        (K::EarlyEntry, 7, "drafted", "undrafted"),
        (K::SmallSchoolStar, 7, "drafted", "overlooked"),
        (K::DraftSlide, 7, "picked", "picked"),
    ];
    for (kind, best, good, bad) in bound {
        let k = kind.id();
        let list: Vec<(u32, u32)> = w
            .storylines
            .open
            .iter()
            .filter(|s| s.kind == k && s.phase.open())
            .filter_map(|s| s.lead_player().map(|p| (s.id, p)))
            .collect();
        for (id, p) in list {
            match pick(w, p) {
                Some(r) => close(w, id, if r <= best { good } else { bad }, None),
                None if done >= best => close(w, id, bad, None),
                None => {}
            }
        }
    }
    // The clubs that needed a quarterback.
    for (id, _) in open_of(w, K::QbNeed) {
        let Some(TeamKey::Nfl(n)) = w.storylines.get(id).and_then(|s| s.team) else { continue };
        let n = n as usize;
        let found = w.clubs.get(n).is_some_and(|c| {
            c.roster.iter().any(|&i| {
                w.people[i].player.position == Position::QB
                    && w.people[i]
                        .pro
                        .as_ref()
                        .and_then(|x| x.draft)
                        .is_some_and(|x| x.0 == year && x.1 <= 3 && x.3 == n)
            })
        });
        if found {
            close(w, id, "found_one", None);
        } else if done >= 3 {
            close(w, id, "passed", None);
        }
    }
}

/// The best free agents to be, as the market opens to talks.
fn free_agents(w: &mut World) {
    let year = w.year;
    let mut v: Vec<(f64, usize)> = Vec::new();
    for club in &w.clubs {
        for &i in &club.roster {
            let p = &w.people[i];
            let Some(c) = p.pro.as_ref() else { continue };
            let expiring = c.contract.as_ref().is_some_and(|k| {
                k.last_year() == year - 1 && !matches!(k.kind, Deal::Franchise | Deal::Transition | Deal::Rookie)
            });
            if !expiring || p.player.age > 31 {
                continue;
            }
            let grade = c.last_grade.unwrap_or(0.0);
            if grade >= 82.0 || pro_bowler(w, p.player.id, 2) {
                v.push((grade, i));
            }
        }
    }
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let due = w.dates.league_year.date;
    for (_, i) in v.into_iter().take(10) {
        let s = player_spot(w, K::FreeAgentStar, i).due(due).public();
        tell(w, s);
    }
}

/// The class race on its signing day: first or second.
fn class_races(w: &mut World, d: Date) {
    let due: Vec<(u32, Option<TeamKey>)> = w
        .storylines
        .open
        .iter()
        .filter(|s| s.kind == K::ClassRace.id() && s.phase.open() && s.due.is_some_and(|x| x <= d))
        .map(|s| (s.id, s.team))
        .collect();
    if due.is_empty() {
        return;
    }
    let top = class_scores(w).first().map(|x| college(x.1));
    for (id, team) in due {
        close(w, id, if team.is_some() && top == team { "won" } else { "second" }, None);
    }
}

// --- What a day answers -------------------------------------------------------------

fn closings(w: &mut World, d: Date) {
    contract_years(w, d);
    // A free agent signed.
    for (id, subject) in open_of(w, K::FreeAgentStar) {
        let Some(i) = pi(w, subject as u32) else { continue };
        let Some(n) = w.people[i]
            .pro
            .as_ref()
            .filter(|c| c.contract.as_ref().is_some_and(|k| k.signed == w.year))
            .and_then(|c| c.team)
        else {
            continue;
        };
        let to = TeamKey::Nfl(n as u16);
        let stayed = w.storylines.get(id).and_then(|s| s.team) == Some(to);
        let text = format!("{} signs with {}.", pname(w, i), tname(w, to));
        close(w, id, if stayed { "stayed" } else { "signed" }, Some(text));
    }
    // An old star decided.
    for (id, subject) in open_of(w, K::RetireDecision) {
        let due = w.storylines.get(id).and_then(|s| s.due).unwrap_or(d);
        let i = pi(w, subject as u32);
        let retired = i.is_none_or(|i| w.people[i].stage == Stage::Done);
        let playing = i.is_some_and(|i| w.people[i].pro.as_ref().is_some_and(|c| c.team.is_some()));
        if retired && d > w.dates.start {
            close(w, id, "retired", None);
        } else if d >= due && playing {
            close(w, id, "returned", None);
        } else if d >= due.add(120) {
            close(w, id, "retired", None);
        }
    }
    // He turned pro.
    for (id, subject) in open_of(w, K::DeclareDecision) {
        let Some(i) = pi(w, subject as u32) else { continue };
        if w.people[i].stage == Stage::Declared {
            let text = format!("{} declares for the draft.", pname(w, i));
            close(w, id, "declared", Some(text));
        }
    }
    // The portal closed on a program's exodus.
    for (id, _) in open_of(w, K::PortalExodus) {
        if w.storylines.get(id).and_then(|s| s.due).is_some_and(|x| x < d) {
            close(w, id, "window_closed", None);
        }
    }
    // A raided staff filled again.
    for (id, _) in open_of(w, K::StaffRaid) {
        let Some(s) = w.storylines.get(id) else { continue };
        let (Some(t), due) = (s.team, s.due) else { continue };
        let now = staff_now(w, t);
        let filled = now[0] != 0 && now[1] != 0 && now[2] != 0;
        if filled {
            let e = employer(t);
            let inside = now[1..].iter().all(|&c| {
                let co = &w.coaches[c as usize - 1];
                co.history.last().is_some_and(|s| s.employer == e && s.to >= w.year - 1)
            });
            close(w, id, if inside { "promoted" } else { "replaced" }, None);
        } else if due.is_some_and(|x| x.add(30) < d) {
            close(w, id, "replaced", None);
        }
    }
}

/// A contract year answered: an extension, the tag, or gone in free agency.
fn contract_years(w: &mut World, d: Date) {
    for (id, subject) in open_of(w, K::ContractYear) {
        let Some(s) = w.storylines.get(id) else { continue };
        let (season, team) = (s.season, s.team);
        let Some(i) = pi(w, subject as u32) else { continue };
        let Some(c) = w.people[i].pro.as_ref() else { continue };
        let k = c.contract.as_ref();
        let here = k.is_some_and(|k| Some(TeamKey::Nfl(k.team as u16)) == team);
        let tagged = here && k.is_some_and(|k| matches!(k.kind, Deal::Franchise | Deal::Transition));
        let extended = here && !tagged && k.is_some_and(|k| k.signed >= season && k.last_year() > season);
        let free = d >= w.dates.league_year.date && w.year > season;
        let name = pname(w, i);
        if tagged {
            close(w, id, "tagged", None);
        } else if extended {
            let text = format!("{name} signs an extension with {}.", team.map_or_else(String::new, |t| tname(w, t)));
            close(w, id, "extended", Some(text));
        } else if free && !here {
            close(w, id, "walked", None);
        }
    }
}

// --- Your offers --------------------------------------------------------------------

/// Another job wants you: the rumor, and how it ends.
fn offers(w: &mut World, d: Date) {
    let Some(c) = crate::world::stories::seat_coach(w) else { return };
    let mine_now = crate::world::stories::seat_team(w);
    let list: Vec<(Employer, Date, Date)> = w
        .career
        .as_ref()
        .map_or(Vec::new(), |k| k.market.offers.iter().map(|o| (o.employer, o.made, o.until)).collect());
    for &(e, made, until) in &list {
        let t = team_of_employer(e);
        let code = tcode(t);
        if w.storylines.watch.offers.iter().any(|x| x.0 == code && x.1 == made) {
            continue;
        }
        w.storylines.watch.offers.push((code, made));
        let mut s = Spot::new(K::JobRumors, s_coach(c))
            .lead(Who::Coach(c as u32))
            .other(t)
            .with(Who::Team(t), Part::Suitor)
            .word(tname(w, t))
            .due(until);
        if let Some(m) = mine_now {
            s = s.team(m).with(Who::Boss(m), Part::Boss);
        }
        tell(w, s);
    }
    w.storylines.watch.offers.retain(|x| x.1.days_to(d) < 400);
    for (id, _) in open_of(w, K::JobRumors) {
        let Some(s) = w.storylines.get(id) else { continue };
        let suitors: Vec<TeamKey> = s
            .cast
            .iter()
            .filter(|m| m.part == Part::Suitor)
            .filter_map(|m| match m.who {
                Who::Team(t) => Some(t),
                _ => None,
            })
            .collect();
        if mine_now.is_some_and(|m| suitors.contains(&m)) {
            close(w, id, "took_it", None);
        } else if !list.iter().any(|x| suitors.contains(&team_of_employer(x.0))) {
            close(w, id, "stayed", None);
        }
    }
}
