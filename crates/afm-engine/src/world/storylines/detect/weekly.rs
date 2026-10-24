//! Mondays: what builds over weeks. In camp, the quarterback battles and
//! the veterans losing their jobs; once a season as camp opens, the arcs a
//! season starts with (a title to defend, a dynasty, a comeback, a contract
//! year, a final season, a rebuild); in season, the seats, the
//! coordinators, the controversies, the races and the chases, the
//! breakouts, and the stories of the coming week's games; after each
//! level's season, the sweep that answers what is still open; and in the
//! offseason, the draft's and the signing calendar's arcs.
//!
//! Thresholds are Claude's picks (proposed, not confirmed).

use super::games::{hurt, last_starter};
use super::{
    K, Part, Who, at, career_wins, cname, coach_spot, head, in_season, mine, on, open_of, pid, player_spot, pname,
    s_coach, s_pair, s_player, s_season, s_team, started, team_of, team_spot, tell, tname,
};
use crate::calendar::{Date, Level};
use crate::player::PlayingTime;
use crate::positions::Position;
use crate::world::coach::{Employer, Role};
use crate::world::data::Division;
use crate::world::stories::TeamKey;
use crate::world::storylines::tell::record_now;
use crate::world::storylines::{Spot, close, fact, open_id, pi, subj, tcode};
use crate::world::{Stage, World};

/// Every team the world tells stories about this season: every FBS and
/// FCS program with a team, every NFL club playing, the high schools
/// people follow, and yours.
pub(crate) fn teams(w: &World) -> Vec<TeamKey> {
    let year = w.year;
    let mut v: Vec<TeamKey> = w
        .programs
        .iter()
        .filter(|p| matches!(p.division, Division::FBS | Division::FCS))
        .filter(|p| !p.dark.iter().any(|&(a, b)| year >= a && year <= b))
        .map(|p| TeamKey::College(p.id as u32))
        .collect();
    let active = |t: usize| w.nfl_season.as_ref().is_none_or(|s| s.active.get(t).copied().unwrap_or(false));
    v.extend((0..w.nfl.len()).filter(|&t| active(t)).map(|t| TeamKey::Nfl(t as u16)));
    v.extend(w.storylines.watch.hs.iter().map(|&h| TeamKey::Hs(h)));
    if let Some(t) = crate::world::stories::seat_team(w)
        && !v.contains(&t)
    {
        v.push(t);
    }
    v
}

/// The team's first game this season.
pub(crate) fn opener(w: &World, t: TeamKey) -> Option<Date> {
    match t {
        TeamKey::College(_) => w.dates.cfb_weeks.first().copied(),
        TeamKey::Nfl(_) => Some(w.dates.nfl_kickoff),
        TeamKey::Hs(h) => {
            let st = w.high_schools.get(h as usize)?.state;
            w.hs_season.as_ref()?.states.get(st)?.weeks.first().copied()
        }
        TeamKey::Other(_) => None,
    }
}

/// The team's last regular-season day.
fn regular_end(w: &World, t: TeamKey) -> Option<Date> {
    match t {
        TeamKey::College(p) => Some(if w.programs.get(p as usize)?.division == Division::FBS {
            w.dates.fbs_regular_end
        } else {
            w.dates.fcs_regular_end
        }),
        TeamKey::Nfl(_) => Some(w.dates.black_monday.add(-1)),
        TeamKey::Hs(h) => {
            let st = w.high_schools.get(h as usize)?.state;
            w.hs_season.as_ref()?.states.get(st)?.weeks.last().copied()
        }
        TeamKey::Other(_) => None,
    }
}

/// Camp: from the spring game (college), the week before veterans report
/// (NFL) or four weeks out (high school) to the first game.
fn in_camp(w: &World, t: TeamKey, d: Date) -> bool {
    let Some(o) = opener(w, t) else { return false };
    let from = match t {
        TeamKey::College(_) => w.dates.spring_game,
        TeamKey::Nfl(_) => w.dates.veterans_report.add(-7),
        TeamKey::Hs(_) => o.add(-28),
        TeamKey::Other(_) => return false,
    };
    d >= from && d < o
}

fn ovr(w: &World, i: usize) -> f64 {
    w.people[i].player.overall()
}

fn played(w: &World, t: TeamKey) -> u32 {
    let (a, b) = record_now(w, t);
    a + b
}

fn win_pct(w: &World, t: TeamKey) -> f64 {
    let (a, b) = record_now(w, t);
    if a + b == 0 { 0.5 } else { a as f64 / (a + b) as f64 }
}

/// A deterministic draw in [0, 1) for a weekly chance.
pub(crate) fn roll(w: &World, salt: u64, key: u64) -> f64 {
    crate::world::season::key_uniform(w.options.seed as u64 ^ salt ^ ((w.today.0 as u64) << 20), key)
}

/// A coach's years in his job.
fn tenure(w: &World, c: usize) -> i32 {
    w.coaches[c].job.map_or(0, |j| w.year - j.since)
}

/// The employer a team key names.
pub(crate) fn employer(t: TeamKey) -> Employer {
    match t {
        TeamKey::College(p) => Employer::College(p as usize),
        TeamKey::Nfl(n) => Employer::Nfl(n as usize),
        TeamKey::Hs(h) => Employer::HighSchool(h as usize),
        TeamKey::Other(f) => Employer::Other(f as usize),
    }
}

pub(crate) fn team_of_employer(e: Employer) -> TeamKey {
    match e {
        Employer::College(p) => TeamKey::College(p as u32),
        Employer::Nfl(n) => TeamKey::Nfl(n as u16),
        Employer::HighSchool(h) => TeamKey::Hs(h as u32),
        Employer::Other(f) => TeamKey::Other(f as u16),
    }
}

/// Whether a coach was a head coach at a team before.
fn coached_at(w: &World, c: usize, t: TeamKey) -> bool {
    let e = employer(t);
    w.coaches[c].history.iter().any(|s| s.employer == e && s.role == Role::HeadCoach)
}

/// A player's season line at his level, for the rises and the chases:
/// passing, rushing and receiving yards, sacks and interceptions.
fn season_nums(w: &World, i: usize) -> Option<[f64; 5]> {
    let p = &w.people[i];
    let id = p.player.id;
    match p.stage {
        Stage::College => {
            w.college.as_ref()?.lines.get(&id).map(|l| [l.pass_yds, l.rush_yds, l.rec_yds, l.sacks, l.ints as f64])
        }
        Stage::Pro => {
            let l = nfl_line(w, i)?;
            Some([l.pass_yds, l.rush_yds, l.rec_yds, l.sacks, l.ints as f64])
        }
        _ => None,
    }
}

fn nfl_line(w: &World, i: usize) -> Option<&crate::game::stats::StatLine> {
    let year = w.year;
    w.people[i].pro.as_ref()?.seasons.iter().rev().find(|s| s.year == year).map(|s| &s.stats)
}

const NUM_NAMES: [&str; 5] = ["passing yards", "rushing yards", "receiving yards", "sacks", "interceptions"];

/// The NFL stats the record book keeps (`pro::honors`).
pub(crate) const NFL_STATS: [&str; 10] = [
    "passing yards",
    "passing touchdowns",
    "rushing yards",
    "rushing touchdowns",
    "receptions",
    "receiving yards",
    "receiving touchdowns",
    "sacks",
    "interceptions",
    "field goals",
];

/// A chase's stat now: codes 0-3 a school's season records (passing,
/// rushing and receiving yards, sacks), 10-19 an NFL season mark, 20-29 an
/// NFL career mark.
pub(crate) fn record_value(w: &World, id: u32, code: i32) -> f64 {
    let Some(i) = pi(w, id) else { return 0.0 };
    match code {
        0..=3 => w.college.as_ref().and_then(|c| c.lines.get(&id)).map_or(0.0, |l| match code {
            0 => l.pass_yds,
            1 => l.rush_yds,
            2 => l.rec_yds,
            _ => l.sacks,
        }),
        10..=19 => {
            nfl_line(w, i).map_or(0.0, |l| crate::world::pro::honors::stat_of(l, NFL_STATS[(code - 10) as usize]))
        }
        20..=29 => {
            let stat = NFL_STATS[(code - 20) as usize];
            let past = w.nfl_book.careers.get(&id).map_or(0.0, |l| crate::world::pro::honors::stat_of(l, stat));
            let booked = w.nfl_season.as_ref().is_some_and(|s| s.booked);
            past + if booked {
                0.0
            } else {
                nfl_line(w, i).map_or(0.0, |l| crate::world::pro::honors::stat_of(l, stat))
            }
        }
        _ => 0.0,
    }
}

pub(crate) fn stat_name(code: i32) -> &'static str {
    match code {
        0..=3 => crate::world::program::RECORD_STATS[code as usize].0,
        10..=19 => NFL_STATS[(code - 10) as usize],
        20..=29 => NFL_STATS[(code - 20) as usize],
        _ => "",
    }
}

pub(crate) fn record_scope(code: i32) -> &'static str {
    match code {
        0..=3 => "school",
        10..=19 => "NFL single-season",
        _ => "NFL career",
    }
}

/// Mondays.
pub(crate) fn monday(w: &mut World, d: Date) {
    let teams = teams(w);
    for &t in &teams {
        if in_camp(w, t, d) {
            qb_battle(w, t);
            if matches!(t, TeamKey::Nfl(_)) {
                veteran_job(w, t, d);
            }
        }
    }
    season_start(w, d, &teams);
    season_looks(w, d, &teams);
    game_week(w, d);
    sweeps(w, d);
    due_checks(w, d);
    offseason(w, d);
}

// --- Camp ------------------------------------------------------------------

/// Two quarterbacks close in camp: the incumbent (last season's starter)
/// and the best of the rest.
fn qb_battle(w: &mut World, t: TeamKey) {
    let year = w.year;
    let subject = s_season(t, year);
    let qbs = at(w, t, Position::QB);
    if qbs.len() < 2 {
        return;
    }
    let inc = incumbent(w, t, &qbs);
    let Some(chal) =
        qbs.iter().copied().filter(|&i| i != inc).max_by(|&a, &b| ovr(w, a).total_cmp(&ovr(w, b)).then(b.cmp(&a)))
    else {
        return;
    };
    let gap = ovr(w, chal) - ovr(w, inc);
    let margin = if matches!(t, TeamKey::Nfl(_)) { -1.5 } else { -2.0 };
    if gap < margin && open_id(w, K::QbBattle, subject).is_none() {
        return;
    }
    let Some(due) = opener(w, t) else { return };
    let s = team_spot(w, K::QbBattle, t, subject)
        .lead(Who::Player(pid(w, inc)))
        .rival(Who::Player(pid(w, chal)))
        .due(due)
        .truth((gap / 3.0) as f32);
    tell(w, s);
}

/// Last season's starter: the quarterback who started the team's last
/// game, or the one who started last season, or the top of the depth chart.
fn incumbent(w: &World, t: TeamKey, qbs: &[usize]) -> usize {
    if let Some(id) = last_starter(w, t)
        && let Some(&i) = qbs.iter().find(|&&i| pid(w, i) == id)
    {
        return i;
    }
    if let Some(&i) = qbs.iter().find(|&&i| started(w, i) && w.people[i].college_seasons > 0) {
        return i;
    }
    qbs[0]
}

/// A veteran starter behind a young player on the camp depth chart (NFL).
fn veteran_job(w: &mut World, t: TeamKey, d: Date) {
    let TeamKey::Nfl(n) = t else { return };
    let n = n as usize;
    let year = w.year;
    let Some(club) = w.clubs.get(n) else { return };
    let mut found: Vec<(usize, usize)> = Vec::new();
    for pos in Position::ALL {
        if pos == Position::QB || pos.is_specialist() {
            continue;
        }
        let Some(depth) = club.depth.get(pos.index()) else { continue };
        let starters = crate::league::depth_slots(pos).starters;
        for &i in depth.iter().skip(starters).take(2) {
            let p = &w.people[i];
            let Some(c) = p.pro.as_ref() else { continue };
            let was = c
                .seasons
                .iter()
                .rev()
                .find(|s| s.year == year - 1)
                .is_some_and(|s| s.team == Some(n) && s.starts >= 10);
            if was
                && p.player.age >= 29
                && let Some(&r) = depth.iter().take(starters).find(|&&j| w.people[j].player.age <= 25)
            {
                found.push((i, r));
            }
        }
    }
    let due = w.dates.nfl_weeks.get(2).copied().unwrap_or(d.add(30));
    for (i, r) in found.into_iter().take(1) {
        let s = player_spot(w, K::VeteranJob, i).rival(Who::Player(pid(w, r))).due(due);
        tell(w, s);
    }
}

// --- Once a season, as camp opens --------------------------------------------

fn season_start(w: &mut World, d: Date, teams: &[TeamKey]) {
    let year = w.year;
    if d >= w.dates.fall_camp && w.storylines.once(21, year) {
        for &t in teams.iter().filter(|t| matches!(t, TeamKey::College(_))) {
            college_start(w, t);
            team_start(w, t);
        }
        final_seasons_college(w, teams);
        your_contract(w);
    }
    if d >= w.dates.veterans_report && w.storylines.once(23, year) {
        for &t in teams.iter().filter(|t| matches!(t, TeamKey::Nfl(_))) {
            nfl_start(w, t);
            team_start(w, t);
        }
        career_records(w);
    }
    let hs_ready = w.hs_season.as_ref().is_some_and(|s| d >= s.ratings_day);
    if hs_ready && w.storylines.once(22, year) {
        for &t in teams.iter().filter(|t| matches!(t, TeamKey::Hs(_))) {
            team_start(w, t);
        }
    }
}

/// The team's past seasons, most recent last: (season, wins, losses, a
/// title, its head coach).
fn past(w: &World, t: TeamKey) -> Vec<(i32, u32, u32, bool, Option<usize>)> {
    match t {
        TeamKey::College(p) => w.programs[p as usize]
            .seasons
            .iter()
            .map(|s| (s.year, s.wins, s.losses, s.national_title, s.coach))
            .collect(),
        TeamKey::Nfl(n) => w
            .nfl_seasons
            .iter()
            .filter(|s| s.team == n as usize)
            .map(|s| (s.year, s.wins, s.losses, s.round == 5, s.coach))
            .collect(),
        TeamKey::Hs(h) => w.high_schools[h as usize]
            .seasons
            .iter()
            .map(|s| (s.year, s.wins, s.losses, s.state_title, s.coach))
            .collect(),
        TeamKey::Other(_) => Vec::new(),
    }
}

/// A team's arcs as its season opens: a title to defend, a dynasty, a
/// decline, a rise, a rebuild, a graveyard job, the old coach's farewell.
fn team_start(w: &mut World, t: TeamKey) {
    if !on(w, t) {
        return;
    }
    let year = w.year;
    let past = past(w, t);
    let last = past.iter().rev().find(|s| s.0 == year - 1).copied();
    let recent: Vec<_> = past.iter().filter(|s| s.0 >= year - 6 && s.0 < year).collect();
    let titles = recent.iter().filter(|s| s.3).count();
    let name = tname(w, t);
    let opener = opener(w, t);
    let end = regular_end(w, t);
    if let Some(l) = last
        && l.3
    {
        let mut s = team_spot(w, K::TitleDefense, t, s_season(t, year))
            .fact(format!("{name} opens camp as the defending champion."), 4.0)
            .public();
        if let Some(e) = end {
            s = s.due(e);
        }
        tell(w, s);
    }
    let dynasty_at = match t {
        TeamKey::Nfl(_) => 2,
        TeamKey::Hs(_) => 4,
        _ => 3,
    };
    if titles >= dynasty_at {
        let s = team_spot(w, K::Dynasty, t, s_season(t, year))
            .nums([titles as i32, 0, 0, 0])
            .fact(format!("{name} has won {titles} titles in six seasons."), 5.0)
            .public();
        tell(w, s);
    }
    // A fallen power: three losing seasons in a row after better days.
    let losing = |s: &&(i32, u32, u32, bool, Option<usize>)| s.1 < s.2;
    let three: Vec<_> = past.iter().filter(|s| s.0 >= year - 3 && s.0 < year).collect();
    let was_power = match t {
        TeamKey::College(p) => {
            let g = &w.programs[p as usize];
            g.prestige_history.iter().any(|&(y, v)| y <= year - 5 && y >= year - 12 && v >= 0.72)
        }
        _ => past.iter().filter(|s| s.0 >= year - 12 && s.0 < year - 3).filter(|s| s.3).count() >= 1,
    };
    if three.len() == 3 && three.iter().all(losing) && was_power {
        let s = team_spot(w, K::Decline, t, s_season(t, year))
            .fact(format!("{name} has had three losing seasons in a row."), 3.0)
            .public();
        tell(w, s);
    }
    // A rising program: prestige up and a big last season.
    if let TeamKey::College(p) = t {
        let g = &w.programs[p as usize];
        let then = g.prestige_history.iter().find(|&&(y, _)| y == year - 4).map(|x| x.1);
        let (prestige, dark_next) = (g.prestige, g.dark.iter().any(|&(a, _)| a == year + 1));
        if let (Some(then), Some(l)) = (then, last)
            && prestige - then >= 0.1
            && l.1 >= 9
        {
            let s = team_spot(w, K::ProgramRise, t, s_season(t, year))
                .fact(format!("{name} comes off a {}-{} season, climbing.", l.1, l.2), 3.0)
                .public();
            tell(w, s);
        }
        // A school about to drop football.
        if dark_next {
            let s = team_spot(w, K::ChoppingBlock, t, s_team(t)).truth(1.0).due(w.dates.title_game).public();
            tell(w, s);
        }
    }
    // The head coach: a new one, an old one near the end.
    let Some(c) = head(w, t) else { return };
    let ten = tenure(w, c);
    if ten == 0 {
        if let Some(l) = last
            && l.1 <= if matches!(t, TeamKey::Nfl(_)) { 4 } else { 3 }
        {
            let s = team_spot(w, K::Rebuild, t, s_coach(c))
                .lead(Who::Coach(c as u32))
                .due(Date::ymd(year + 2, 12, 1))
                .fact(format!("{} takes over at {name} after a {}-{} season.", cname(w, c), l.1, l.2), 3.0)
                .public();
            tell(w, s);
        }
        let coaches: Vec<usize> = {
            let mut v: Vec<usize> = past.iter().filter(|s| s.0 >= year - 10).filter_map(|s| s.4).collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        if coaches.len() >= 4 {
            let s = team_spot(w, K::Graveyard, t, s_coach(c))
                .lead(Who::Coach(c as u32))
                .nums([coaches.len() as i32 + 1, 0, 0, 0])
                .due(Date::ymd(year + 2, 12, 1))
                .public();
            tell(w, s);
        }
        if crate::world::stories::seat_coach(w) == Some(c) {
            let mut s = Spot::new(K::NewCoach, s_coach(c))
                .team(t)
                .lead(Who::Coach(c as u32))
                .with(Who::Boss(t), Part::Boss)
                .public();
            if let Some(e) = end {
                s = s.due(e);
            }
            tell(w, s);
        }
    }
    let co = &w.coaches[c];
    let wins = career_wins(w, c);
    if co.age >= 70 || (co.age >= 66 && wins >= 150) {
        let mut s = coach_spot(w, K::CoachFarewell, c, t).nums([wins as i32, 0, 0, 0]).public();
        if let Some(o) = opener {
            s = s.due(o.add(150));
        }
        tell(w, s);
    }
}

/// College camp: position switches, busts, comebacks.
fn college_start(w: &mut World, t: TeamKey) {
    if !on(w, t) {
        return;
    }
    let year = w.year;
    let roster = super::roster(w, t);
    let end = regular_end(w, t);
    let mut switches = Vec::new();
    let mut busts = Vec::new();
    let mut comebacks = Vec::new();
    for &i in &roster {
        let p = &w.people[i];
        let h = &p.player.history;
        let n = h.len();
        if n >= 2 {
            let (a, b) = (&h[n - 2], &h[n - 1]);
            if a.position != b.position && a.playing_time == PlayingTime::Starter && b.position == p.player.position {
                switches.push((i, a.position));
            }
            if a.events.has(crate::player::DevelopmentEvent::MajorInjury)
                && h.iter().rev().skip(1).any(|r| r.playing_time == PlayingTime::Starter)
            {
                comebacks.push(i);
            }
        }
        if p.stars == 5 && p.college_seasons == 2 && p.last_role != Some(PlayingTime::Starter) {
            busts.push(i);
        }
    }
    for (i, from) in switches.into_iter().take(2) {
        let to = w.people[i].player.position;
        let mut s = player_spot(w, K::PositionSwitch, i)
            .word(to.name())
            .fact(format!("{} moves from {} to {} for {}.", pname(w, i), from.name(), to.name(), tname(w, t)), 2.0);
        if let Some(e) = end {
            s = s.due(e);
        }
        tell(w, s);
    }
    for i in busts.into_iter().take(1) {
        let mut s = player_spot(w, K::Bust, i).public();
        if let Some(e) = end {
            s = s.due(e);
        }
        tell(w, s);
    }
    for i in comebacks.into_iter().take(2) {
        let mut s = player_spot(w, K::InjuryComeback, i)
            .fact(format!("{} is back in camp after the injury that ended his season.", pname(w, i)), 3.0)
            .public();
        if let Some(e) = end {
            s = s.due(e);
        }
        tell(w, s);
    }
    let _ = year;
}

/// The best seniors' last seasons (the top 25 FBS starters by rating).
fn final_seasons_college(w: &mut World, teams: &[TeamKey]) {
    let mut v: Vec<(f64, usize, TeamKey)> = Vec::new();
    for &t in teams {
        let TeamKey::College(p) = t else { continue };
        if w.programs[p as usize].division != Division::FBS {
            continue;
        }
        for i in super::roster(w, t) {
            let q = &w.people[i];
            if q.college_seasons >= 3 && q.last_role == Some(PlayingTime::Starter) && !q.player.position.is_specialist()
            {
                v.push((ovr(w, i), i, t));
            }
        }
    }
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, i, t) in v.into_iter().take(25) {
        let mut s = player_spot(w, K::FinalSeason, i).public();
        if let Some(e) = regular_end(w, t) {
            s = s.due(e);
        }
        tell(w, s);
    }
}

/// Your contract in its last year.
fn your_contract(w: &mut World) {
    let Some(c) = crate::world::stories::seat_coach(w) else { return };
    let Some(t) = crate::world::stories::seat_team(w) else { return };
    let year = w.year;
    let Some(k) = w.coaches[c].contract.as_ref() else { return };
    if k.through > year {
        return;
    }
    let through = k.through;
    let s = Spot::new(K::YourContract, s_coach(c))
        .team(t)
        .lead(Who::Coach(c as u32))
        .with(Who::Boss(t), Part::Boss)
        .nums([0, 0, through, 0])
        .due(w.dates.signing_day)
        .public();
    tell(w, s);
}

/// NFL camp: contract years, busts, second chances, fallen stars,
/// comebacks, final seasons.
fn nfl_start(w: &mut World, t: TeamKey) {
    let TeamKey::Nfl(n) = t else { return };
    let n = n as usize;
    let year = w.year;
    let next_year = Date::ymd(year + 1, 3, 12);
    let end = regular_end(w, t);
    let roster = super::roster(w, t);
    let starters: Vec<usize> = {
        let mut v = Vec::new();
        if let Some(club) = w.clubs.get(n) {
            for pos in Position::ALL {
                if let Some(dc) = club.depth.get(pos.index()) {
                    v.extend(dc.iter().take(crate::league::depth_slots(pos).starters).copied());
                }
            }
        }
        v.sort_unstable();
        v
    };
    let pro_bowls = |w: &World, id: u32| w.nfl_book.pro_bowls.iter().filter(|x| x.1 == id).count();
    let mut picks: Vec<(K, usize)> = Vec::new();
    for &i in &roster {
        let p = &w.people[i];
        let Some(c) = p.pro.as_ref() else { continue };
        let id = p.player.id;
        let starting = starters.binary_search(&i).is_ok();
        let age = p.player.age;
        let last = c.seasons.iter().rev().find(|s| s.year == year - 1);
        let pb = pro_bowls(w, id);
        // His deal's last year, a good starter: will they pay him first?
        if starting
            && age <= 30
            && c.contract.as_ref().is_some_and(|k| k.last_year() == year && k.team == n)
            && (c.last_grade.unwrap_or(0.0) >= 76.0 || pb > 0)
        {
            picks.push((K::ContractYear, i));
        }
        // A first-round pick in year two or three, not starting.
        if let Some((dy, 1, _, _)) = c.draft
            && (year - dy == 1 || year - dy == 2)
            && !starting
        {
            picks.push((K::Bust, i));
        }
        // A first-round pick on his second team.
        if let Some((_, 1, _, dt)) = c.draft
            && dt != n
            && c.new_team
            && age <= 28
        {
            picks.push((K::SecondChance, i));
        }
        // A past star on a new team.
        if pb >= 2 && age >= 30 && c.new_team {
            picks.push((K::FallenStar, i));
        }
        // Back from a long injury.
        if last.is_some_and(|s| s.ir_weeks >= 8) && starting {
            picks.push((K::InjuryComeback, i));
        }
        // The old star's last go.
        if starting && (age >= 37 || (age >= 36 && pb >= 4)) {
            picks.push((K::FinalSeason, i));
        }
    }
    for (kind, i) in picks {
        let mut s = player_spot(w, kind, i);
        s = match kind {
            K::ContractYear => s.due(next_year),
            _ => match end {
                Some(e) => s.due(e).public(),
                None => s.public(),
            },
        };
        tell(w, s);
    }
}

/// Active players near a career record.
fn career_records(w: &mut World) {
    let records: Vec<(usize, f64, String)> = w
        .nfl_book
        .records
        .iter()
        .filter(|r| r.career)
        .filter_map(|r| NFL_STATS.iter().position(|s| *s == r.stat).map(|k| (k, r.value, r.holder.clone())))
        .collect();
    let mut found: Vec<(usize, i32, f64, String, f64)> = Vec::new();
    for club in &w.clubs {
        for &i in &club.roster {
            let id = w.people[i].player.id;
            let Some(l) = w.nfl_book.careers.get(&id) else { continue };
            for (k, rec, holder) in &records {
                let v = crate::world::pro::honors::stat_of(l, NFL_STATS[*k]);
                if v >= 0.9 * rec && v < *rec && w.people[i].name != *holder {
                    found.push((i, 20 + *k as i32, *rec, holder.clone(), v));
                }
            }
        }
    }
    for (i, code, rec, holder, v) in found {
        let s = player_spot(w, K::CareerRecord, i)
            .nums([v.round() as i32, rec.round() as i32, code, 0])
            .word(holder)
            .due(w.dates.black_monday)
            .public();
        tell(w, s);
    }
}

// --- In season ---------------------------------------------------------------

fn season_looks(w: &mut World, d: Date, teams: &[TeamKey]) {
    let col = in_season(w, Level::College);
    let nfl = in_season(w, Level::Nfl);
    let hs = in_season(w, Level::HighSchool);
    if !(col || nfl || hs) {
        return;
    }
    for &t in teams {
        let lvl = match t {
            TeamKey::College(_) => col,
            TeamKey::Nfl(_) => nfl,
            TeamKey::Hs(_) => hs,
            TeamKey::Other(_) => false,
        };
        if !lvl || !on(w, t) {
            continue;
        }
        let g = played(w, t);
        let (seat_at, coord_at) = match t {
            TeamKey::Nfl(_) => (6, 6),
            TeamKey::Hs(_) => (6, 99),
            _ => (5, 5),
        };
        if g >= 3 && !matches!(t, TeamKey::Hs(_)) {
            qb_controversy(w, t);
        }
        if g >= seat_at
            && let Some(c) = head(w, t)
        {
            seat_look(w, t, c, false);
        }
        if g >= coord_at {
            coordinator_heat(w, t);
        }
        if g >= 8 {
            extension_talks(w, t);
        }
        if (4..=7).contains(&g) {
            turnaround(w, t);
        }
        room_and_fans(w, t);
        if g >= 8
            && let TeamKey::College(p) = t
            && w.programs[p as usize].division == Division::FBS
        {
            bowl_chase(w, t);
        }
        milestone(w, t);
    }
    if col {
        college_rises(w);
        school_records(w);
        heisman(w, d);
        conference_races(w, d);
        cfp_push(w, d);
        drought(w, d, Level::College);
    }
    if nfl {
        let wk = w.dates.nfl_weeks.iter().filter(|&&x| x <= d).count();
        if wk >= 6 {
            nfl_rises(w);
        }
        if wk >= 8 {
            nfl_records(w);
        }
        if wk >= 10 {
            mvp(w);
        }
        if wk >= 12 {
            nfl_races(w);
            drought(w, d, Level::Nfl);
            retire_decisions(w);
        }
    }
    chances(w, d);
}

/// The hot seat (S2): the record against the boss's patience. `push`: a
/// losing streak lowers the bar.
pub(crate) fn seat_look(w: &mut World, t: TeamKey, c: usize, push: bool) {
    if !on(w, t) {
        return;
    }
    let year = w.year;
    let (wins, losses) = record_now(w, t);
    let games = wins + losses;
    let ten = tenure(w, c);
    let prove = w.coaches[c].prove_it.is_some();
    let (hot, truth) = match t {
        TeamKey::College(p) => {
            let ctx = w.college.as_ref().and_then(|cs| cs.coaches.get(p as usize)).is_some_and(|x| x.hot_seat);
            let vs = w.college.as_ref().and_then(|cs| cs.results.get(p as usize)).map_or(0.0, |r| r.vs_expectation());
            let hot = (ctx && losses > wins)
                || (ten >= 2 && losses >= wins + 3)
                || (push && ten >= 2 && losses >= wins + 2 && games >= 5)
                || (prove && losses >= wins);
            (hot, (-2.5 * vs + if ten >= 3 { 0.15 } else { 0.0 } + if prove { 0.3 } else { 0.0 }) as f32)
        }
        TeamKey::Nfl(_) => {
            let pct = if games == 0 { 0.5 } else { wins as f64 / games as f64 };
            let hot = (ten >= 2 && games >= 6 && pct < 0.36)
                || (ten >= 1 && games >= 8 && pct <= 0.25)
                || (push && ten >= 2 && pct < 0.42);
            (hot, ((0.45 - pct) * 3.0 + if ten >= 4 { 0.1 } else { 0.0 }) as f32)
        }
        TeamKey::Hs(_) => (ten >= 3 && losses >= wins + 4, 0.0),
        TeamKey::Other(_) => (false, 0.0),
    };
    if !hot {
        return;
    }
    let due = match t {
        TeamKey::Nfl(_) => w.dates.black_monday,
        TeamKey::Hs(_) => Date::ymd(year, 12, 1),
        _ => regular_end(w, t).unwrap_or(w.dates.fbs_regular_end).add(1),
    };
    let (through, buyout) =
        w.coaches[c].contract.as_ref().map_or((0, 0), |k| (k.through, (k.buyout(year) / 1e6).round() as i32));
    let s = coach_spot(w, K::HotSeat, c, t).due(due).truth(truth).nums([wins as i32, losses as i32, through, buyout]);
    let Some(id) = tell(w, s) else { return };
    let (public, heat) = w.storylines.get(id).map_or((false, 0.0), |s| (s.public, s.heat));
    // The buyout, big enough to be its own story.
    if public && buyout >= 10 && matches!(t, TeamKey::College(_)) && open_id(w, K::BuyoutWatch, s_coach(c)).is_none() {
        let s = coach_spot(w, K::BuyoutWatch, c, t).nums([buyout, 0, 0, 0]).due(due).public();
        tell(w, s);
    }
    // The boss says he's safe.
    if public
        && heat >= 45.0
        && open_id(w, K::VoteOfConfidence, s_coach(c)).is_none()
        && roll(w, 0x707E, c as u64) < 0.25
    {
        let s = coach_spot(w, K::VoteOfConfidence, c, t).due(due).public();
        tell(w, s);
    }
}

/// A unit at the bottom of its level: the coordinator's seat.
fn coordinator_heat(w: &mut World, t: TeamKey) {
    let Some((pf, pa, n, rank_f, rank_a)) = unit_ranks(w, t) else { return };
    let (oc, dc) = coordinators(w, t);
    let bottom = (n as f64 * 0.15).ceil() as usize;
    for (c, rank, off) in [(oc, rank_f, true), (dc, rank_a, false)] {
        let Some(c) = c else { continue };
        let subject = s_coach(c);
        let open = open_id(w, K::CoordinatorHeat, subject);
        if rank + bottom >= n {
            let what = if off {
                format!("{} scores {pf:.1} points a game, near the bottom of the league.", tname(w, t))
            } else {
                format!("{} allows {pa:.1} points a game, near the bottom of the league.", tname(w, t))
            };
            let s = coach_spot(w, K::CoordinatorHeat, c, t)
                .nums([rank as i32 + 1, n as i32, off as i32, 0])
                .fact(what, 2.0);
            tell(w, s);
        } else if let Some(id) = open
            && rank + (n as f64 * 0.3) as usize <= n
        {
            close(w, id, "turned_it", None);
        }
    }
}

/// A team's coordinators.
fn coordinators(w: &World, t: TeamKey) -> (Option<usize>, Option<usize>) {
    match t {
        TeamKey::College(p) => (w.programs[p as usize].oc, w.programs[p as usize].dc),
        TeamKey::Nfl(n) => (w.nfl[n as usize].oc, w.nfl[n as usize].dc),
        _ => (None, None),
    }
}

/// Points a game for and against, the level's team count, and the team's
/// rank on each (0 best).
fn unit_ranks(w: &World, t: TeamKey) -> Option<(f64, f64, usize, usize, usize)> {
    let per: Vec<(usize, f64, f64)> = match t {
        TeamKey::College(p) => {
            let c = w.college.as_ref()?;
            let div = w.programs[p as usize].division;
            c.results
                .iter()
                .enumerate()
                .filter(|(j, r)| w.programs[*j].division == div && r.totals.games >= 3)
                .map(|(j, r)| {
                    (
                        j,
                        r.totals.points_for as f64 / r.totals.games as f64,
                        r.totals.points_against as f64 / r.totals.games as f64,
                    )
                })
                .collect()
        }
        TeamKey::Nfl(_) => {
            let s = w.nfl_season.as_ref()?;
            let mut acc = vec![(0.0, 0.0, 0u32); s.wins.len()];
            for (k, week) in s.weeks.iter().enumerate() {
                for (j, &(h, a)) in week.iter().enumerate() {
                    if let Some(sc) = s.slots.get(k).and_then(|x| x.get(j)).and_then(|x| x.score) {
                        acc[h].0 += sc[0] as f64;
                        acc[h].1 += sc[1] as f64;
                        acc[h].2 += 1;
                        acc[a].0 += sc[1] as f64;
                        acc[a].1 += sc[0] as f64;
                        acc[a].2 += 1;
                    }
                }
            }
            acc.iter()
                .enumerate()
                .filter(|x| x.1.2 >= 3)
                .map(|(j, x)| (j, x.0 / x.2 as f64, x.1 / x.2 as f64))
                .collect()
        }
        _ => return None,
    };
    let me = match t {
        TeamKey::College(p) => p as usize,
        TeamKey::Nfl(n) => n as usize,
        _ => return None,
    };
    let mine = per.iter().find(|x| x.0 == me)?;
    let rank_f = per.iter().filter(|x| x.1 > mine.1).count();
    let rank_a = per.iter().filter(|x| x.2 < mine.2).count();
    Some((mine.1, mine.2, per.len(), rank_f, rank_a))
}

/// A winning coach near the end of his deal (college).
fn extension_talks(w: &mut World, t: TeamKey) {
    let TeamKey::College(_) = t else { return };
    let Some(c) = head(w, t) else { return };
    let year = w.year;
    let Some(k) = w.coaches[c].contract.as_ref() else { return };
    let through = k.through;
    let (wins, losses) = record_now(w, t);
    if through <= year + 1 && wins >= 7 && losses <= 2 {
        let s = coach_spot(w, K::ExtensionTalks, c, t).nums([wins as i32, losses as i32, through, 0]);
        tell(w, s);
    }
}

/// A losing team, a starter playing poorly, a backup close behind.
fn qb_controversy(w: &mut World, t: TeamKey) {
    let (wins, losses) = record_now(w, t);
    if wins >= losses {
        return;
    }
    let Some(id) = last_starter(w, t) else { return };
    let Some(i) = pi(w, id) else { return };
    let poor = match t {
        TeamKey::College(_) => w.college.as_ref().and_then(|c| c.lines.get(&id)).is_some_and(|l| {
            let att = (l.pass_yds / 7.0).max(1.0);
            l.int_thrown >= l.pass_td + 2 || (l.pass_yds / att < 6.0 && l.pass_td <= l.int_thrown + 1)
        }),
        TeamKey::Nfl(_) => {
            nfl_line(w, i).is_some_and(|l| l.att >= 60 && (l.pass_yds / l.att as f64 <= 6.2 || l.int > l.pass_td))
        }
        _ => false,
    };
    if !poor {
        return;
    }
    let qbs = at(w, t, Position::QB);
    let Some(b) = qbs.iter().copied().filter(|&j| j != i).max_by(|&a, &b| ovr(w, a).total_cmp(&ovr(w, b))) else {
        return;
    };
    let close_behind = ovr(w, b) >= ovr(w, i) - 3.0 || (w.people[b].stars >= 4 && w.people[b].college_seasons <= 1);
    if !close_behind || hurt(w, pid(w, b)) {
        return;
    }
    let s = player_spot(w, K::QbControversy, i).rival(Who::Player(pid(w, b))).nums([wins as i32, losses as i32, 0, 0]);
    tell(w, s);
}

/// Last season's bad team off to a good start.
fn turnaround(w: &mut World, t: TeamKey) {
    let year = w.year;
    let Some(last) = past(w, t).into_iter().rev().find(|s| s.0 == year - 1) else { return };
    let (wins, losses) = record_now(w, t);
    if last.1 <= 4 && last.2 >= 7 && wins >= 4 && losses <= 1 {
        let name = tname(w, t);
        let s = team_spot(w, K::Turnaround, t, s_season(t, year))
            .nums([wins as i32, losses as i32, last.1 as i32, last.2 as i32])
            .fact(format!("{name} is {wins}-{losses} a year after going {}-{}.", last.1, last.2), 3.0)
            .public();
        tell(w, s);
    }
}

/// A room turning (the team's mood) and fans losing patience.
fn room_and_fans(w: &mut World, t: TeamKey) {
    let year = w.year;
    let (mood, prestige) = match t {
        TeamKey::College(p) => (w.programs[p as usize].team_mood, w.programs[p as usize].prestige),
        TeamKey::Nfl(n) => (w.clubs.get(n as usize).map_or(0.0, |c| c.team_mood), 0.7),
        _ => return,
    };
    if let Some(c) = head(w, t) {
        let subject = s_coach(c);
        if mood < -0.4 {
            let s = coach_spot(w, K::RoomRift, c, t);
            tell(w, s);
        } else if mood > -0.1
            && let Some(id) = open_id(w, K::RoomRift, subject)
        {
            close(w, id, "healed", None);
        }
    }
    let run = w.storylines.watch.runs.get(&tcode(t)).copied().unwrap_or(0);
    if run <= -3 && prestige >= 0.55 {
        let s = team_spot(w, K::FanRevolt, t, s_season(t, year)).nums([-run as i32, 0, 0, 0]);
        tell(w, s);
    } else if run >= 3
        && let Some(id) = open_id(w, K::FanRevolt, s_season(t, year))
    {
        close(w, id, "calmed", None);
    }
}

/// Four or five wins late: the chase for six and a bowl.
fn bowl_chase(w: &mut World, t: TeamKey) {
    let year = w.year;
    let (wins, losses) = record_now(w, t);
    if (4..=5).contains(&wins) && losses <= 6 && losses >= 4 {
        let s = team_spot(w, K::BowlChase, t, s_season(t, year)).nums([wins as i32, losses as i32, 0, 0]).public();
        tell(w, s);
    }
}

/// A head coach close to a round number of wins.
fn milestone(w: &mut World, t: TeamKey) {
    let Some(c) = head(w, t) else { return };
    let wins = career_wins(w, c);
    let Some(m) = [50u32, 100, 150, 200, 250, 300, 350, 400].into_iter().find(|&m| m > wins) else { return };
    if m - wins > 2 || (m == 50 && !matches!(t, TeamKey::Hs(_))) {
        return;
    }
    let kind = if crate::world::stories::seat_coach(w) == Some(c) { K::YourMilestone } else { K::CoachMilestone };
    let s = coach_spot(w, kind, c, t).nums([m as i32, wins as i32, 0, 0]).public();
    tell(w, s);
}

/// The college season's rises: breakouts, freshmen, walk-ons, two-way
/// players, read against every FBS line.
fn college_rises(w: &mut World) {
    let Some(c) = w.college.as_ref() else { return };
    if c.results.iter().map(|r| r.games()).max().unwrap_or(0) < 4 {
        return;
    }
    // National ranks in each stat, FBS only.
    let mut cols: [Vec<(f64, u32)>; 5] = Default::default();
    for (&id, l) in &c.lines {
        if w.programs.get(l.program).is_none_or(|p| p.division != Division::FBS) {
            continue;
        }
        for (k, v) in [l.pass_yds, l.rush_yds, l.rec_yds, l.sacks, l.ints as f64].into_iter().enumerate() {
            if v > 0.0 {
                cols[k].push((v, id));
            }
        }
    }
    for col in cols.iter_mut() {
        col.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        col.truncate(60);
    }
    let rank = |id: u32| -> Option<(usize, usize)> {
        cols.iter().enumerate().filter_map(|(k, col)| col.iter().position(|x| x.1 == id).map(|r| (r, k))).min()
    };
    let mut ids: Vec<u32> = cols.iter().flat_map(|c| c.iter().map(|x| x.1)).collect();
    ids.sort_unstable();
    ids.dedup();
    let snaps = c.snaps.clone();
    let mut opens: Vec<(K, usize, usize, usize)> = Vec::new();
    for id in ids {
        let Some(i) = pi(w, id) else { continue };
        let Some((r, k)) = rank(id) else { continue };
        let p = &w.people[i];
        if p.stage != Stage::College || !team_of(w, i).is_some_and(|t| on(w, t)) {
            continue;
        }
        let was_starter = p.player.history.iter().rev().nth(1).is_some_and(|h| h.playing_time == PlayingTime::Starter);
        if r < 15 && !was_starter && p.college_seasons >= 1 {
            opens.push((K::Breakout, i, r, k));
        }
        if r < 40 && p.college_seasons == 0 {
            opens.push((K::FreshmanStar, i, r, k));
        }
        if r < 60 && p.walk_on {
            opens.push((K::WalkOnStar, i, r, k));
        }
    }
    // Both ways: real snaps on offense and defense.
    for (&id, &(off, def)) in &snaps {
        if off >= 150
            && def >= 150
            && let Some(i) = pi(w, id)
            && team_of(w, i).is_some_and(|t| on(w, t))
        {
            opens.push((K::TwoWay, i, 0, 0));
        }
    }
    // Update the ranks of open rises.
    for (id, _) in open_of(w, K::Breakout).into_iter().chain(open_of(w, K::FreshmanStar)) {
        let Some(lead) = w.storylines.get(id).and_then(|s| s.lead_player()) else { continue };
        let r = rank(lead).map_or(99, |x| x.0 as i32);
        if let Some(k) = w.storylines.idx(id) {
            w.storylines.open[k].nums[1] = r + 1;
        }
    }
    for (kind, i, r, k) in opens {
        let mut s = player_spot(w, kind, i).nums([0, r as i32 + 1, k as i32, 0]).public();
        if kind != K::TwoWay {
            let v = season_nums(w, i).map_or(0.0, |n| n[k]);
            s = s.fact(
                format!(
                    "{} has {} {} this season, No. {} in the country.",
                    pname(w, i),
                    crate::world::folk::facts::thousands(v.round() as u32),
                    NUM_NAMES[k],
                    r + 1
                ),
                3.0,
            );
        }
        if kind == K::WalkOnStar {
            s = s.due(Date::ymd(w.year + 1, 8, 10));
        }
        tell(w, s);
    }
}

/// A school's season record within reach.
fn school_records(w: &mut World) {
    let Some(c) = w.college.as_ref() else { return };
    let mut opens: Vec<(usize, i32, f64, f64, String)> = Vec::new();
    for (&id, l) in &c.lines {
        let Some(prog) = w.programs.get(l.program) else { continue };
        if !matches!(prog.division, Division::FBS | Division::FCS) || l.games < 5 {
            continue;
        }
        for k in 0..4 {
            let v = match k {
                0 => l.pass_yds,
                1 => l.rush_yds,
                2 => l.rec_yds,
                _ => l.sacks,
            };
            let floor = crate::world::program::RECORD_STATS[k].1;
            let (rec, holder) = prog.season_records[k]
                .as_ref()
                .map_or((floor, String::new()), |r| (r.value.max(floor), r.holder.clone()));
            let pace = v / l.games as f64 * 12.0;
            if pace >= rec && v >= 0.55 * rec && v < rec {
                let Some(i) = pi(w, id) else { continue };
                opens.push((i, k as i32, v, rec, holder));
            }
        }
    }
    for (i, code, v, rec, holder) in opens {
        let s = player_spot(w, K::RecordChase, i)
            .nums([v.round() as i32, rec.round() as i32, code, 0])
            .word(holder)
            .public();
        tell(w, s);
    }
}

/// The Heisman race: a clear leader by midseason.
fn heisman(w: &mut World, d: Date) {
    let year = w.year;
    if d >= w.dates.heisman {
        return;
    }
    let Some(c) = w.college.as_ref() else { return };
    if c.results.iter().map(|r| r.games()).max().unwrap_or(0) < 8 {
        return;
    }
    let mut v: Vec<(f64, u32)> = c
        .lines
        .iter()
        .filter(|(_, l)| w.programs.get(l.program).is_some_and(|p| p.division == Division::FBS))
        .map(|(&id, l)| {
            let wins = c.results.get(l.program).map_or(0, |r| r.wins) as f64;
            let qb = l.pass_yds / 20.0 + l.pass_td as f64 * 4.0 - l.int_thrown as f64 * 3.0;
            let ground = l.rush_yds / 10.0 + l.rush_td as f64 * 6.0 + l.rec_yds / 10.0 + l.rec_td as f64 * 6.0;
            (qb + ground + wins * 8.0, id)
        })
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let (Some(&(a, lead)), Some(&(b, rival))) = (v.first(), v.get(1)) else { return };
    if a < 1.15 * b {
        return;
    }
    let Some(i) = pi(w, lead) else { return };
    let mut s = Spot::new(K::AwardRace, subj(0xA11A, year as u64))
        .lead(Who::Player(lead))
        .rival(Who::Player(rival))
        .word("Heisman Trophy")
        .due(w.dates.heisman)
        .public();
    if let Some(t) = team_of(w, i) {
        s = s.team(t);
    }
    tell(w, s);
}

/// Conference races in the last four weeks.
fn conference_races(w: &mut World, d: Date) {
    let year = w.year;
    if d < w.dates.fbs_regular_end.add(-28) || d > w.dates.fbs_regular_end {
        return;
    }
    let Some(c) = w.college.as_ref() else { return };
    let mut by_conf: std::collections::BTreeMap<String, Vec<(u32, u32, usize)>> = Default::default();
    for (j, r) in c.results.iter().enumerate() {
        let p = &w.programs[j];
        if p.division != Division::FBS || p.conference.is_empty() || p.conference.contains("Independent") {
            continue;
        }
        by_conf.entry(p.conference.clone()).or_default().push((r.conf_losses, r.conf_wins, j));
    }
    let mut opens = Vec::new();
    for (conf, mut v) in by_conf {
        v.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)));
        if let (Some(a), Some(b)) = (v.first(), v.get(1))
            && a.0 <= 2
            && b.0 <= a.0 + 1
            && a.1 >= 3
        {
            opens.push((conf, a.2, b.2));
        }
    }
    for (conf, a, b) in opens {
        let (ta, tb) = (TeamKey::College(a as u32), TeamKey::College(b as u32));
        let code = conf.bytes().fold(0u64, |h, x| h.wrapping_mul(31).wrapping_add(x as u64));
        let s = team_spot(w, K::ConferenceRace, ta, subj(code & 0xFFFF_FFFF, year as u64))
            .other(tb)
            .rival(Who::Team(tb))
            .word(conf)
            .due(w.dates.conference_championships)
            .public();
        tell(w, s);
    }
}

/// The playoff bubble from the committee's ranking (2014 on).
fn cfp_push(w: &mut World, d: Date) {
    let year = w.year;
    if year < 2014 || d < w.dates.first_cfp_ranking || d > w.dates.selection_day {
        return;
    }
    let Some(poll) =
        w.college.as_ref().and_then(|c| c.polls.iter().rev().find(|p| p.kind == crate::world::college::PollKind::Cfp))
    else {
        return;
    };
    let field: usize = if year >= 2024 { 12 } else { 4 };
    let (from, to) = (field.saturating_sub(2), field + 3);
    let bubble: Vec<usize> = poll.top.iter().copied().skip(from).take(to - from).collect();
    for j in bubble {
        let t = TeamKey::College(j as u32);
        let s = team_spot(w, K::PlayoffPush, t, s_season(t, year)).due(w.dates.selection_day).public();
        tell(w, s);
    }
}

/// A long wait for a title, with a contender this year.
fn drought(w: &mut World, d: Date, level: Level) {
    let year = w.year;
    let mut cands: Vec<(TeamKey, i32)> = Vec::new();
    match level {
        Level::College => {
            if d.month() != 11 && d.month() != 12 {
                return;
            }
            let Some(poll) = w.college.as_ref().and_then(|c| c.polls.last()) else { return };
            for &j in poll.top.iter().take(4) {
                let g = &w.programs[j];
                let last = g.seasons.iter().rev().find(|s| s.national_title).map(|s| s.year);
                let since = last.map_or(year - (w.options.start_year - w.options.history_years as i32), |y| year - y);
                if g.titles == 0 || since >= 25 {
                    cands.push((TeamKey::College(j as u32), since));
                }
            }
        }
        Level::Nfl => {
            let Some(s) = w.nfl_season.as_ref() else { return };
            for t in 0..s.wins.len() {
                let games = s.wins[t] + s.losses[t];
                if games < 11 || s.wins[t] * 10 < games * 7 {
                    continue;
                }
                let last = w.nfl_champions.iter().rev().find(|x| x.1 == t).map(|x| x.0);
                let since = last.map_or(30, |y| year - y);
                if since >= 25 {
                    cands.push((TeamKey::Nfl(t as u16), since));
                }
            }
        }
        _ => return,
    }
    for (t, since) in cands {
        let s = team_spot(w, K::Drought, t, s_season(t, year)).nums([since, 0, 0, 0]).public();
        tell(w, s);
    }
}

/// NFL rises: breakouts and late-round gems, from the season's lines.
fn nfl_rises(w: &mut World) {
    let year = w.year;
    let mut cols: [Vec<(f64, usize)>; 5] = Default::default();
    for club in &w.clubs {
        for &i in &club.roster {
            if let Some(n) = season_nums(w, i) {
                for (k, v) in n.into_iter().enumerate() {
                    if v > 0.0 {
                        cols[k].push((v, i));
                    }
                }
            }
        }
    }
    for col in cols.iter_mut() {
        col.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        col.truncate(15);
    }
    let mut opens: Vec<(K, usize, usize, usize)> = Vec::new();
    for (k, col) in cols.iter().enumerate() {
        for (r, &(_, i)) in col.iter().enumerate() {
            let Some(c) = w.people[i].pro.as_ref() else { continue };
            let last = c.seasons.iter().rev().find(|s| s.year == year - 1);
            if r < 10 && last.is_none_or(|s| s.starts == 0) {
                opens.push((K::Breakout, i, r, k));
            }
            let late = c.draft.is_none_or(|x| x.1 >= 5);
            if late && year - c.first_year <= 2 {
                opens.push((K::LateRoundGem, i, r, k));
            }
        }
    }
    for (kind, i, r, k) in opens {
        let v = season_nums(w, i).map_or(0.0, |n| n[k]);
        let s = player_spot(w, kind, i)
            .nums([0, r as i32 + 1, k as i32, 0])
            .fact(
                format!(
                    "{} has {} {} this season, No. {} in the league.",
                    pname(w, i),
                    crate::world::folk::facts::thousands(v.round() as u32),
                    NUM_NAMES[k],
                    r + 1
                ),
                3.0,
            )
            .public();
        tell(w, s);
    }
}

/// NFL season records within reach.
fn nfl_records(w: &mut World) {
    let year = w.year;
    let games = if year >= 2021 { 17.0 } else { 16.0 };
    let records: Vec<(usize, f64, String)> = w
        .nfl_book
        .records
        .iter()
        .filter(|r| !r.career)
        .filter_map(|r| NFL_STATS.iter().position(|s| *s == r.stat).map(|k| (k, r.value, r.holder.clone())))
        .collect();
    let mut opens = Vec::new();
    for club in &w.clubs {
        for &i in &club.roster {
            let Some(l) = nfl_line(w, i) else { continue };
            if l.games < 6 {
                continue;
            }
            for (k, rec, holder) in &records {
                let v = crate::world::pro::honors::stat_of(l, NFL_STATS[*k]);
                if v / l.games as f64 * games >= *rec && v >= 0.6 * rec && v < *rec {
                    opens.push((i, 10 + *k as i32, v, *rec, holder.clone()));
                }
            }
        }
    }
    for (i, code, v, rec, holder) in opens {
        let s = player_spot(w, K::RecordChase, i)
            .nums([v.round() as i32, rec.round() as i32, code, 0])
            .word(holder)
            .public();
        tell(w, s);
    }
}

/// The MVP race: a quarterback clearly ahead.
fn mvp(w: &mut World) {
    let year = w.year;
    let mut v: Vec<(f64, usize)> = Vec::new();
    for (n, club) in w.clubs.iter().enumerate() {
        let wins = w.nfl_record(n).0 as f64;
        for &i in &club.roster {
            let Some(l) = nfl_line(w, i) else { continue };
            let score = l.pass_yds / 20.0 + l.pass_td as f64 * 4.0 - l.int as f64 * 3.0
                + l.rush_yds / 10.0
                + l.rush_td as f64 * 6.0
                + wins * 10.0;
            v.push((score, i));
        }
    }
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let (Some(&(a, i)), Some(&(b, j))) = (v.first(), v.get(1)) else { return };
    if a < 1.12 * b {
        return;
    }
    let mut s = Spot::new(K::AwardRace, subj(0xA11B, year as u64))
        .lead(Who::Player(pid(w, i)))
        .rival(Who::Player(pid(w, j)))
        .word("MVP")
        .due(w.dates.nfl_honors)
        .public();
    if let Some(t) = team_of(w, i) {
        s = s.team(t);
    }
    tell(w, s);
}

/// Division races and the wild-card bubble.
fn nfl_races(w: &mut World) {
    let year = w.year;
    let Some(s) = w.nfl_season.as_ref() else { return };
    let n = s.wins.len();
    let pct = |t: usize| {
        let g = s.wins[t] + s.losses[t] + s.ties[t];
        if g == 0 { 0.5 } else { (s.wins[t] as f64 + 0.5 * s.ties[t] as f64) / g as f64 }
    };
    let mut races = Vec::new();
    for div in 0..8 {
        let mut v: Vec<usize> =
            (0..n).filter(|&t| s.active.get(t).copied().unwrap_or(false) && s.div[t] == div).collect();
        v.sort_by(|&a, &b| pct(b).total_cmp(&pct(a)).then(a.cmp(&b)));
        if let (Some(&a), Some(&b)) = (v.first(), v.get(1))
            && s.wins[a] <= s.wins[b] + 1
        {
            races.push((div, a, b));
        }
    }
    let mut bubble = Vec::new();
    for conf in 0..2 {
        let mut v: Vec<usize> =
            (0..n).filter(|&t| s.active.get(t).copied().unwrap_or(false) && s.conf[t] == conf).collect();
        v.sort_by(|&a, &b| pct(b).total_cmp(&pct(a)).then(a.cmp(&b)));
        let line = if year >= 2020 { 7 } else { 6 };
        bubble.extend(v.into_iter().skip(line - 2).take(4));
    }
    for (div, a, b) in races {
        let (ta, tb) = (TeamKey::Nfl(a as u16), TeamKey::Nfl(b as u16));
        let s = team_spot(w, K::ConferenceRace, ta, subj(0xD1F0 + div as u64, year as u64))
            .other(tb)
            .rival(Who::Team(tb))
            .word(division_name(div, year))
            .due(w.dates.black_monday)
            .public();
        tell(w, s);
    }
    for t in bubble {
        let tk = TeamKey::Nfl(t as u16);
        let s = team_spot(w, K::PlayoffPush, tk, s_season(tk, year)).due(w.dates.black_monday).public();
        tell(w, s);
    }
}

/// An NFL division's name in its era.
pub(crate) fn division_name(div: usize, year: i32) -> String {
    let conf = if div < 4 { "AFC" } else { "NFC" };
    let part = if year >= 2002 {
        ["East", "North", "South", "West"][div % 4]
    } else {
        ["East", "Central", "West", "West"][div % 4]
    };
    format!("{conf} {part}")
}

/// The old stars near the end.
fn retire_decisions(w: &mut World) {
    let mut opens = Vec::new();
    for club in &w.clubs {
        for &i in &club.roster {
            let p = &w.people[i];
            let id = p.player.id;
            let pb = w.nfl_book.pro_bowls.iter().filter(|x| x.1 == id).count();
            if p.player.age >= 34 && pb >= 3 && nfl_line(w, i).is_some_and(|l| l.games >= 8) {
                opens.push(i);
            }
        }
    }
    let due = Date::ymd(w.year + 1, 3, 15);
    for i in opens {
        let s = player_spot(w, K::RetireDecision, i).due(due).public();
        tell(w, s);
    }
}

/// The weekly chances off the field: a social media storm, a second
/// opinion on an injury, a player at odds with his coach.
fn chances(w: &mut World, _d: Date) {
    let mut opens: Vec<(K, usize)> = Vec::new();
    for club in &w.clubs {
        for &i in &club.roster {
            let p = &w.people[i];
            let Some(c) = p.pro.as_ref() else { continue };
            let id = p.player.id as u64;
            if p.followers >= 500_000 {
                let temper = p.player.personality.traits.z(crate::player::Trait::Temper);
                if roll(w, 0x5051, id) < 0.0015 * (1.0 + temper.max(0.0)) {
                    opens.push((K::SocialStorm, i));
                }
            }
            if c.out_weeks >= 6 && roll(w, 0x5E0D, id) < 0.04 {
                opens.push((K::SecondOpinion, i));
            }
            if c.wants_out && c.trust < 0.3 && roll(w, 0xFE0D, id) < 0.2 {
                opens.push((K::PublicFeud, i));
            }
        }
    }
    let d = w.today;
    for (kind, i) in opens {
        let mut s = player_spot(w, kind, i);
        s = match kind {
            K::SocialStorm => s.due(d.add(14)).public(),
            K::PublicFeud => match team_of(w, i).and_then(|t| head(w, t)) {
                Some(c) => s.rival(Who::Coach(c as u32)),
                None => s,
            },
            _ => s,
        };
        tell(w, s);
    }
}

// --- The coming week's games -------------------------------------------------

/// One game in the coming week, as the pair stories read it.
struct Coming {
    date: Date,
    teams: [TeamKey; 2],
    rivalry: Option<usize>,
    postseason: bool,
    title: bool,
    senior_day: bool,
}

fn game_week(w: &mut World, d: Date) {
    let until = d.add(7);
    let mut games: Vec<Coming> = Vec::new();
    if in_season(w, Level::College)
        && let Some(c) = w.college.as_ref()
    {
        use crate::world::schedule::GameKind;
        for g in c.games.iter().filter(|g| g.played.is_none() && g.date >= d && g.date < until) {
            games.push(Coming {
                date: g.date,
                teams: [TeamKey::College(g.home as u32), TeamKey::College(g.away as u32)],
                rivalry: g.rivalry,
                postseason: g.kind != GameKind::Regular,
                title: matches!(g.kind, GameKind::Title | GameKind::ConferenceTitle),
                senior_day: g.senior_day,
            });
        }
    }
    if in_season(w, Level::Nfl)
        && let Some(s) = w.nfl_season.as_ref()
    {
        for (k, week) in s.weeks.iter().enumerate() {
            for (j, &(h, a)) in week.iter().enumerate() {
                let Some(slot) = s.slots.get(k).and_then(|x| x.get(j)) else { continue };
                if slot.played || slot.at.date < d || slot.at.date >= until {
                    continue;
                }
                let div = s.div.get(h) == s.div.get(a);
                games.push(Coming {
                    date: slot.at.date,
                    teams: [TeamKey::Nfl(h as u16), TeamKey::Nfl(a as u16)],
                    rivalry: div.then_some(usize::MAX),
                    postseason: false,
                    title: false,
                    senior_day: false,
                });
            }
        }
        for &(_, _, h, a, at) in &s.round_games {
            if at.date >= d && at.date < until {
                games.push(Coming {
                    date: at.date,
                    teams: [TeamKey::Nfl(h as u16), TeamKey::Nfl(a as u16)],
                    rivalry: None,
                    postseason: true,
                    title: at.date == w.dates.super_bowl,
                    senior_day: false,
                });
            }
        }
    }
    // Senior night at the high schools people follow.
    if in_season(w, Level::HighSchool)
        && let Some(hs) = w.hs_season.as_ref()
    {
        for &h in &w.storylines.watch.hs {
            if let Some(Some((_, senior))) = hs.marks.get(h as usize)
                && *senior >= d
                && *senior < until
            {
                games.push(Coming {
                    date: *senior,
                    teams: [TeamKey::Hs(h), TeamKey::Hs(h)],
                    rivalry: None,
                    postseason: false,
                    title: false,
                    senior_day: true,
                });
            }
        }
    }
    for g in games {
        if !(on(w, g.teams[0]) || on(w, g.teams[1])) {
            continue;
        }
        if g.senior_day {
            last_home_game(w, g.teams[0], g.date);
        }
        if g.teams[0] == g.teams[1] {
            continue;
        }
        for side in 0..2 {
            pair_stories(w, &g, side);
        }
    }
}

/// The stories a coming game opens for one side.
fn pair_stories(w: &mut World, g: &Coming, side: usize) {
    let (t, o) = (g.teams[side], g.teams[1 - side]);
    if !on(w, t) {
        return;
    }
    let pair = s_pair(t, o);
    let (name, oname) = (tname(w, t), tname(w, o));
    let due = g.date;
    let spot = |w: &World, kind: K| {
        let mut s =
            Spot::new(kind, subj(pair, kind.id() as u64 ^ (w.year as u64) << 16)).team(t).other(o).due(due).public();
        if let Some(c) = head(w, t) {
            s = s.with(Who::Coach(c as u32), Part::Decider);
        }
        s
    };
    // The rivalry's run.
    if g.rivalry.is_some()
        && let Some(&(holder, n)) = w.storylines.watch.rivals.get(&pair)
    {
        if holder == tcode(t) && n >= 4 {
            let s = spot(w, K::RivalryStreak)
                .with(Who::Team(t), Part::Lead)
                .nums([n as i32, 0, 0, 0])
                .fact(format!("{name} has beaten {oname} {n} straight times."), 3.0);
            tell(w, s);
        }
        if holder == tcode(o)
            && let Some(c) = head(w, t)
            && tenure(w, c) >= 3
            && n as i32 >= tenure(w, c)
        {
            let s = spot(w, K::CantBeatRival)
                .lead(Who::Coach(c as u32))
                .nums([n as i32, 0, 0, 0])
                .fact(format!("{} has never beaten {oname} as {name}'s head coach.", cname(w, c)), 3.0);
            tell(w, s);
        }
    }
    // A trophy on the line (listed college rivalries).
    if side == 0
        && let Some(r) = g.rivalry
        && let Some(trophy) = crate::calendar::data::RIVALRIES.get(r).and_then(|x| x.trophy)
    {
        let holder = w.college.as_ref().and_then(|c| c.series.get(r)).and_then(|s| s.holder);
        let (team, other) = match holder {
            Some(hd) if TeamKey::College(hd as u32) == o => (o, t),
            _ => (t, o),
        };
        let mut s = Spot::new(K::TrophyGame, subj(pair, 0x7E0 ^ (w.year as u64) << 16))
            .team(team)
            .other(other)
            .due(due)
            .word(trophy)
            .public();
        if let Some(c) = head(w, team) {
            s = s.with(Who::Coach(c as u32), Part::Decider);
        }
        tell(w, s);
    }
    // Revenge: a starter facing the team he left.
    if let Some(i) = revenge_player(w, t, o) {
        let id = pid(w, i);
        let s = Spot::new(K::RevengeGame, s_player(id))
            .team(t)
            .other(o)
            .lead(Who::Player(id))
            .due(due)
            .fact(format!("{} faces {oname}, the team he left.", pname(w, i)), 3.0)
            .public();
        tell(w, s);
    }
    // The coach's return.
    if let Some(c) = head(w, t)
        && coached_at(w, c, o)
    {
        let kind = if crate::world::stories::seat_coach(w) == Some(c) { K::OldTeam } else { K::CoachReturn };
        let s = Spot::new(kind, subj(s_coach(c), tcode(o) ^ (w.year as u64) << 24))
            .team(t)
            .other(o)
            .lead(Who::Coach(c as u32))
            .due(due)
            .fact(format!("{} returns to face {oname}, his old team.", cname(w, c)), 3.0)
            .public();
        tell(w, s);
    }
    // A rematch in the postseason, for the side that lost the first one.
    if g.postseason
        && let Some(&winner) = w.storylines.watch.met.get(&pair)
        && winner == tcode(o)
    {
        let s = spot(w, K::Rematch)
            .with(Who::Team(t), Part::Lead)
            .fact(format!("{name} gets another shot at {oname}."), 3.0);
        tell(w, s);
    }
    // Bulletin-board material: yours, or a rival week's loose talk.
    let yours = w
        .folk
        .bulletin
        .is_some_and(|(team, when)| crate::world::storylines::team_key(team) == o && when.days_to(w.today) <= 14)
        && mine(w, t);
    if yours && let Some(c) = head(w, t) {
        let s = spot(w, K::BulletinBoard).lead(Who::Coach(c as u32));
        tell(w, s);
    } else if g.rivalry.is_some() && roll(w, 0xB0AD, tcode(t) ^ pair) < 0.05 {
        let roster = super::roster(w, t);
        if let Some(&i) = roster.iter().max_by(|&&a, &&b| ovr(w, a).total_cmp(&ovr(w, b)).then(b.cmp(&a))) {
            let s = spot(w, K::BulletinBoard).lead(Who::Player(pid(w, i)));
            tell(w, s);
        }
    }
    // Your shot at the big one.
    if (g.title || g.postseason)
        && let Some(c) = head(w, t)
        && crate::world::stories::seat_coach(w) == Some(c)
        && w.coaches[c].career_titles == 0
        && g.title
    {
        let s = Spot::new(K::BigOne, s_coach(c)).team(t).other(o).lead(Who::Coach(c as u32)).due(due).public();
        tell(w, s);
    }
}

/// A starter facing the team he left: a college transfer, an NFL player
/// facing a team that cut or traded him.
fn revenge_player(w: &World, t: TeamKey, o: TeamKey) -> Option<usize> {
    let roster = super::roster(w, t);
    roster
        .into_iter()
        .filter(|&i| w.people[i].last_role == Some(PlayingTime::Starter) || matches!(t, TeamKey::Nfl(_)))
        .filter(|&i| {
            let p = &w.people[i];
            match (t, o) {
                (TeamKey::College(_), TeamKey::College(op)) => {
                    p.path.len() >= 2 && p.path[..p.path.len() - 1].contains(&(op as usize))
                }
                (TeamKey::Nfl(_), TeamKey::Nfl(op)) => p.pro.as_ref().is_some_and(|c| {
                    c.former == Some(op as usize)
                        && c.seasons.iter().rev().find(|s| s.year == w.year - 1).is_some_and(|s| s.starts >= 8)
                }),
                _ => false,
            }
        })
        .max_by(|&a, &b| ovr(w, a).total_cmp(&ovr(w, b)).then(b.cmp(&a)))
}

/// Senior day: the best senior's last home game.
fn last_home_game(w: &mut World, t: TeamKey, date: Date) {
    if !on(w, t) {
        return;
    }
    let year = w.year;
    let roster = super::roster(w, t);
    let senior = |w: &World, i: usize| {
        let p = &w.people[i];
        match t {
            TeamKey::College(_) => p.college_seasons >= 3,
            TeamKey::Hs(_) => p.class_of == year + 1,
            _ => false,
        }
    };
    let Some(&i) = roster
        .iter()
        .filter(|&&i| senior(w, i) && w.people[i].last_role == Some(PlayingTime::Starter))
        .max_by(|&&a, &&b| ovr(w, a).total_cmp(&ovr(w, b)).then(b.cmp(&a)))
    else {
        return;
    };
    let s = player_spot(w, K::LastHomeGame, i).due(date).public();
    tell(w, s);
}

// --- After each level's season ----------------------------------------------

fn sweeps(w: &mut World, d: Date) {
    let year = w.year;
    if d > w.dates.fbs_regular_end && w.storylines.once(31, year) {
        for (id, _) in open_of(w, K::BowlChase) {
            close(w, id, "fell_short", None);
        }
        // Races decided without a title game.
        for (id, _) in open_of(w, K::ConferenceRace) {
            let Some(s) = w.storylines.get(id).cloned() else { continue };
            if !matches!(s.team, Some(TeamKey::College(_))) {
                continue;
            }
            if let (Some(TeamKey::College(a)), Some(TeamKey::College(b))) = (s.team, s.other) {
                let r = |j: u32| {
                    w.college
                        .as_ref()
                        .and_then(|c| c.results.get(j as usize))
                        .map_or((0, 0), |r| (r.conf_wins, r.conf_losses))
                };
                let ((aw, al), (bw, bl)) = (r(a), r(b));
                if d > w.dates.conference_championships {
                    let won = al < bl || (al == bl && aw >= bw);
                    close(w, id, if won { "won" } else { "lost" }, None);
                }
            }
        }
    }
    if d > w.dates.selection_day && w.storylines.once(36, year) {
        let field: Vec<usize> = w
            .college
            .as_ref()
            .map(|c| {
                c.brackets
                    .iter()
                    .filter(|b| b.division == Division::FBS)
                    .flat_map(|b| b.slots.iter().flatten().copied())
                    .collect()
            })
            .unwrap_or_default();
        for (id, _) in open_of(w, K::PlayoffPush) {
            let Some(TeamKey::College(j)) = w.storylines.get(id).and_then(|s| s.team) else { continue };
            close(w, id, if field.contains(&(j as usize)) { "in" } else { "out" }, None);
        }
    }
    if d > w.dates.title_game && w.storylines.once(32, year) {
        season_close(w, Level::College);
    }
    let hs_done = w.hs_season.as_ref().and_then(|s| s.states.iter().map(|x| x.finals).max()).is_some_and(|f| d > f);
    if hs_done && w.storylines.once(35, year) {
        season_close(w, Level::HighSchool);
    }
}

/// Every open season-bound story at a level gets the season's answer.
pub(crate) fn season_close(w: &mut World, level: Level) {
    let year = w.year;
    let list: Vec<u32> = w
        .storylines
        .open
        .iter()
        .filter(|s| s.phase.open() && s.level == level && s.season <= year)
        .map(|s| s.id)
        .collect();
    for id in list {
        let Some(s) = w.storylines.get(id).cloned() else { continue };
        if let Some(e) = season_ending(w, &s) {
            close(w, id, e, None);
        }
    }
}

/// How a season-bound story ends when the season does.
fn season_ending(w: &World, s: &crate::world::storylines::Story) -> Option<&'static str> {
    let k = *K::ALL.get(s.kind as usize)?;
    let lead = s.lead_player();
    let li = lead.and_then(|id| pi(w, id));
    let team = s.team;
    let pct = team.map_or(0.5, |t| win_pct(w, t));
    let started_last = |t: Option<TeamKey>| t.zip(lead).is_some_and(|(t, l)| last_starter(w, t) == Some(l));
    let on_team = li.is_some_and(|i| team_of(w, i) == team);
    let starts = li
        .and_then(|i| w.people[i].pro.as_ref())
        .and_then(|c| c.seasons.iter().rev().find(|x| x.year == s.season))
        .map_or(0, |x| x.starts);
    let pro_bowl = lead.is_some_and(|id| w.nfl_book.pro_bowls.iter().any(|x| x.0 == s.season && x.1 == id));
    let title = team.is_some_and(|t| match t {
        TeamKey::Nfl(n) => w.nfl_champions.iter().any(|x| x.0 == s.season && x.1 == n as usize),
        TeamKey::College(p) => w
            .college
            .as_ref()
            .and_then(|c| c.results.get(p as usize))
            .is_some_and(|r| r.national_title || r.conference_title),
        TeamKey::Hs(h) => w.hs_season.as_ref().and_then(|x| x.results.get(h as usize)).is_some_and(|r| r.state_title),
        TeamKey::Other(_) => false,
    });
    Some(match k {
        K::QbBattle => "split",
        K::QbControversy => {
            if started_last(team) {
                "held_job"
            } else {
                "season_over"
            }
        }
        K::VeteranJob => {
            if on_team {
                "lost_job"
            } else {
                "released"
            }
        }
        K::TwoQb => {
            if pct >= 0.6 {
                "works"
            } else {
                "ends"
            }
        }
        K::KickerSlump | K::LosingStreak | K::CoordinatorHeat => "season_over",
        K::NextManUp => {
            if started_last(team) {
                "kept_job"
            } else {
                "faded"
            }
        }
        K::FreshmanStarter => {
            if !started_last(team) {
                "benched"
            } else if pct >= 0.5 {
                "arrived"
            } else {
                "struggled"
            }
        }
        K::PositionSwitch => {
            if li.is_some_and(|i| w.people[i].player.position.name() == s.word) {
                "stuck"
            } else {
                "moved_back"
            }
        }
        K::ExtensionTalks => {
            let now = s.lead_coach().and_then(|c| w.coaches[c].contract.as_ref()).map_or(0, |k| k.through);
            if now > s.nums[2] { "extended" } else { "stalled" }
        }
        K::Unbeaten => "perfect",
        K::RecordChase | K::CareerRecord => "fell_short",
        K::Drought => "goes_on",
        K::BowlChase => {
            if team.is_some_and(|t| record_now(w, t).0 >= 6) {
                "made_it"
            } else {
                "fell_short"
            }
        }
        K::TitleDefense => "dethroned",
        K::Dynasty => "ended",
        K::Breakout => {
            if s.nums[1] > 0 && s.nums[1] <= 30 {
                "for_real"
            } else {
                "cooled"
            }
        }
        K::FreshmanStar => {
            if s.nums[1] > 0 && s.nums[1] <= 40 {
                "star"
            } else {
                "cooled"
            }
        }
        K::Bust => {
            let started = li.is_some_and(|i| w.people[i].last_role == Some(PlayingTime::Starter)) || starts >= 8;
            if started { "turned_it" } else { "bust" }
        }
        K::TwoWay => {
            let both = lead
                .and_then(|id| w.college.as_ref().and_then(|c| c.snaps.get(&id)))
                .is_some_and(|&(o, d)| o >= 300 && d >= 300);
            if both { "did_it" } else { "one_side" }
        }
        K::ProgramRise => {
            if pct >= 0.7 || title {
                "arrived"
            } else {
                "fell_back"
            }
        }
        K::Turnaround => {
            if pct >= 0.6 {
                "real"
            } else {
                "faded"
            }
        }
        K::RoomRift => {
            let mood = match team {
                Some(TeamKey::College(p)) => w.programs[p as usize].team_mood,
                Some(TeamKey::Nfl(n)) => w.clubs.get(n as usize).map_or(0.0, |c| c.team_mood),
                _ => 0.0,
            };
            if mood >= -0.2 { "healed" } else { "broke" }
        }
        K::FanRevolt => "calmed",
        K::Decline => {
            if pct >= 0.65 {
                "revived"
            } else {
                "still_down"
            }
        }
        K::LateRoundGem => {
            if pro_bowl {
                "star"
            } else {
                "faded"
            }
        }
        K::SecondChance => {
            if starts >= 8 {
                "made_it"
            } else {
                "cut_again"
            }
        }
        K::FallenStar => {
            if starts >= 8 || pro_bowl {
                "found_it"
            } else {
                "done"
            }
        }
        K::InjuryComeback => {
            let back = li.is_some_and(|i| {
                let p = &w.people[i];
                let h = &p.player.history;
                p.last_role == Some(PlayingTime::Starter)
                    || starts >= 8
                    || h.last().is_some_and(|r| r.games_missed <= 3)
            });
            if back { "returned" } else { "not_same" }
        }
        K::StarTraded => {
            let playoffs = team.is_some_and(|t| match t {
                TeamKey::Nfl(n) => {
                    w.nfl_season.as_ref().is_some_and(|x| x.seed.get(n as usize).copied().flatten().is_some())
                }
                _ => false,
            });
            if playoffs || pro_bowl { "paid_off" } else { "flopped" }
        }
        K::FinalSeason => {
            if title {
                "went_out_on_top"
            } else {
                "went_out"
            }
        }
        K::NewCoach => {
            if pct >= 0.55 {
                "strong_start"
            } else {
                "rough_start"
            }
        }
        K::ConferenceRace => "lost",
        K::PlayoffPush => "out",
        K::TeamRules => {
            if on_team {
                "reinstated"
            } else {
                "dismissed"
            }
        }
        K::Academics => {
            if li.is_some_and(|i| w.people[i].cal.fall_ineligible || w.people[i].cal.academic_sit) {
                "out"
            } else {
                "cleared"
            }
        }
        K::Arrest => {
            if s.nums[3] != 0 {
                return None;
            }
            if on_team { "suspended" } else { "dismissed" }
        }
        _ => return None,
    })
}

/// Stories answered on their day: a seat that held, a suitor's search done,
/// a case decided, a project opened.
fn due_checks(w: &mut World, d: Date) {
    let year = w.year;
    let due: Vec<(u32, u16, Date)> = w
        .storylines
        .open
        .iter()
        .filter(|s| s.phase.open())
        .filter_map(|s| s.due.map(|x| (s.id, s.kind, x)))
        .filter(|x| x.2 <= d)
        .collect();
    for (id, kind, when) in due {
        let Some(s) = w.storylines.get(id).cloned() else { continue };
        let late = when.days_to(d);
        let coach_here = s
            .lead_coach()
            .is_some_and(|c| w.coaches[c].job.is_some_and(|j| s.team.is_some_and(|t| employer(t) == j.employer)));
        let li = s.lead_player().and_then(|x| pi(w, x));
        let ending: Option<&str> = match K::ALL[kind as usize] {
            K::HotSeat if late >= 14 && coach_here => {
                let now = s.lead_coach().and_then(|c| w.coaches[c].contract.as_ref()).map_or(0, |k| k.through);
                Some(if s.nums[2] > 0 && now > s.nums[2] { "extended" } else { "saved" })
            }
            K::VoteOfConfidence if late >= 14 && coach_here => Some("meant_it"),
            K::BuyoutWatch if late >= 14 && coach_here => Some("kept_him"),
            K::VeteranJob => {
                let t = s.team;
                let start = li.zip(t).is_some_and(|(i, t)| {
                    let TeamKey::Nfl(n) = t else { return false };
                    let pos = w.people[i].player.position;
                    w.clubs
                        .get(n as usize)
                        .and_then(|c| c.depth.get(pos.index()))
                        .is_some_and(|dc| dc.iter().take(crate::league::depth_slots(pos).starters).any(|&j| j == i))
                });
                Some(if start {
                    "won_back"
                } else if li.is_some_and(|i| team_of(w, i) == t) {
                    "lost_job"
                } else {
                    "released"
                })
            }
            K::NflWantsCoach | K::BigJobWantsCoach | K::OldSchoolCalls | K::CoordinatorHeadJob
                if late >= 7 && coach_here =>
            {
                Some("stayed")
            }
            K::InterimAudition if late >= 45 => Some(if coach_here { "got_job" } else { "passed_over" }),
            K::CoachFarewell if late >= 0 => {
                let gone = s.lead_coach().is_some_and(|c| w.coaches[c].retired || w.coaches[c].job.is_none());
                Some(if gone { "went_out" } else { "stayed_on" })
            }
            K::YourContract => {
                let c = s.lead_coach();
                let now = c.and_then(|c| w.coaches[c].contract.as_ref()).map_or(0, |k| k.through);
                if now > s.nums[2] && coach_here {
                    Some("extended")
                } else if !coach_here {
                    let fired =
                        c.and_then(|c| w.coaches[c].history.last()).is_some_and(|h| h.ended.starts_with("fired"));
                    Some(if fired { "let_go" } else { "left" })
                } else {
                    None
                }
            }
            K::HsCoachJump => Some(if coach_here { "made_it" } else { "came_back" }),
            K::Rebuild => {
                let won = s.team.is_some_and(|t| past(w, t).iter().any(|x| x.0 >= s.season && x.1 > x.2));
                Some(if won && coach_here { "built" } else { "stalled" })
            }
            K::Graveyard => Some(if coach_here { "broken" } else { "another_gone" }),
            K::ChoppingBlock if d.year() > when.year() || late >= 30 => {
                let dark = match s.team {
                    Some(TeamKey::College(p)) => {
                        w.programs[p as usize].dark.iter().any(|&(a, b)| year + 1 >= a && year + 1 <= b)
                    }
                    _ => false,
                };
                Some(if dark { "dropped" } else { "saved" })
            }
            K::FacilityPush => Some("built"),
            K::Realignment => Some("moved"),
            K::NcaaCase => Some(if s.nums[1] > 0 { "sanctioned" } else { "cleared" }),
            K::SocialStorm => {
                let pro = li.is_some_and(|i| {
                    w.people[i].player.personality.traits.z(crate::player::Trait::Professionalism) > 0.0
                });
                Some(if pro { "apologized" } else { "blew_over" })
            }
            K::SuspensionReturn if late >= 0 => {
                let sharp = li.and_then(|i| w.people[i].pro.as_ref()).is_some_and(|c| c.sharpness >= 0.9);
                Some(if sharp { "sharp" } else { "rusty" })
            }
            K::FamilyLoss => {
                let away = s.lead_player().is_some_and(|id| w.folk.out.iter().any(|x| x.0 == id && x.1 >= d));
                Some(if away { "stepped_away" } else { "played" })
            }
            K::DeclareDecision => {
                Some(if li.is_some_and(|i| w.people[i].stage == Stage::Declared) { "declared" } else { "returned" })
            }
            K::HsTransfer => Some(if li.is_some_and(|i| w.people[i].sit_out > 0.0) { "sits" } else { "eligible" }),
            K::WalkOnStar => Some(if li.is_some_and(|i| !w.people[i].walk_on) { "scholarship" } else { "faded" }),
            K::TagStandoff => {
                let tagged = li
                    .and_then(|i| w.people[i].pro.as_ref())
                    .and_then(|c| c.contract.as_ref())
                    .is_some_and(|k| k.kind == crate::world::pro::money::Kind::Franchise);
                Some(if tagged { "plays_on_tag" } else { "long_deal" })
            }
            K::LeagueSuspension if late >= 60 => Some("upheld"),
            K::Arrest if late >= 0 && s.nums[3] == 0 => {
                let here = li.is_some_and(|i| team_of(w, i) == s.team);
                Some(match s.nums[2] {
                    1 => "dismissed",
                    2 => "suspended",
                    _ if here => "suspended",
                    _ => "dismissed",
                })
            }
            K::TeamRules if late >= 0 && s.nums[2] == 1 => Some("dismissed"),
            K::Academics if late >= 0 && s.nums[2] != 0 => Some("out"),
            K::Arrest if late >= 200 => Some("cleared"),
            K::SecondOpinion => {
                let back = s.lead_player().is_some_and(|id| !hurt(w, id));
                let moved = li.is_some_and(|i| team_of(w, i) != s.team);
                if moved {
                    Some("split")
                } else if back {
                    Some("back")
                } else {
                    None
                }
            }
            K::PublicFeud if late >= 0 => {
                let moved = li.is_some_and(|i| team_of(w, i) != s.team);
                Some(if moved { "split" } else { "made_peace" })
            }
            K::NilDispute => {
                let left = li.is_some_and(|i| w.people[i].cal.portal.is_some() || team_of(w, i) != s.team);
                Some(if left { "left" } else { "settled" })
            }
            K::FreeAgentStar if late >= 45 => {
                let signed = li.and_then(|i| w.people[i].pro.as_ref()).and_then(|c| c.team);
                match signed {
                    Some(n) if s.team == Some(TeamKey::Nfl(n as u16)) => Some("stayed"),
                    Some(_) => Some("signed"),
                    None => None,
                }
            }
            _ => None,
        };
        if let Some(e) = ending {
            close(w, id, e, None);
        }
    }
}

// --- The offseason -----------------------------------------------------------

fn offseason(w: &mut World, d: Date) {
    let year = w.year;
    let y = w.dates.clone();
    // Turning pro: December to the deadline.
    if d.month() == 12 || (d < y.declare_deadline && d.month() <= 2 && d > y.title_game.add(-40)) {
        declarations(w, &y);
    }
    // The class race before each signing day.
    if d.month() == 12 || (d.month() <= 2 && d <= y.signing_day) {
        class_race(w, &y);
    }
    if d.month() >= 1 && d.month() <= 4 && d < y.draft[0].date && d > y.title_game {
        top_pick(w, &y);
    }
    if d > y.combine_end && d < y.draft[0].date && w.storylines.once(41, year) {
        combine_and_flags(w, &y);
    }
    if d > y.declare_deadline && d < y.draft[0].date && w.storylines.once(42, year) {
        early_entries(w, &y);
    }
    if d > y.league_year.date && d < y.draft[0].date && w.storylines.once(43, year) {
        qb_needs(w, &y);
    }
    if d.month() == 9 && d.day() >= 15 && w.storylines.once(44, year) {
        small_schools(w, &y);
    }
    stock(w, &y);
    program_news(w);
    if (d.month() == 3 || d.month() == 4) && year >= 2021 {
        nil_disputes(w, &y);
    }
}

/// Underclassmen with a first- or second-round read: will he turn pro?
fn declarations(w: &mut World, y: &crate::calendar::YearDates) {
    let mut opens = Vec::new();
    for p in &w.programs {
        if p.division != Division::FBS {
            continue;
        }
        for &i in &p.roster {
            let q = &w.people[i];
            if q.stage == Stage::College
                && (2..=3).contains(&q.college_seasons)
                && q.cal.draft_read.is_some_and(|r| r.round <= 2)
            {
                opens.push(i);
            }
        }
    }
    for i in opens {
        let s = player_spot(w, K::DeclareDecision, i).due(y.declare_deadline).public();
        tell(w, s);
    }
}

/// The best classes close at the top before a signing day.
fn class_race(w: &mut World, y: &crate::calendar::YearDates) {
    let year = w.year;
    let scores = class_scores(w);
    let (Some(&(a, ja)), Some(&(b, jb))) = (scores.first(), scores.get(1)) else { return };
    if a > 1.04 * b {
        return;
    }
    let (ta, tb) = (TeamKey::College(ja as u32), TeamKey::College(jb as u32));
    let due = match y.early_signing {
        Some(e) if w.today <= e => e,
        _ => y.signing_day,
    };
    let s = team_spot(w, K::ClassRace, ta, subj(0xC1A5, year as u64)).other(tb).rival(Who::Team(tb)).due(due).public();
    tell(w, s);
}

/// The FBS classes signing next, best first: each commitment's stars on
/// the services' points, the deeper players counting less (the 247 shape).
pub(crate) fn class_scores(w: &World) -> Vec<(f64, usize)> {
    let sign_year = crate::world::recruiting::next_class(w);
    let mut by: std::collections::BTreeMap<usize, Vec<f64>> = Default::default();
    for p in &w.people {
        if p.stage != Stage::HighSchool || p.class_of != sign_year {
            continue;
        }
        let Some((j, _, walk_on)) = p.cal.commit else { continue };
        if walk_on || w.programs.get(j).is_none_or(|g| g.division != Division::FBS) {
            continue;
        }
        by.entry(j).or_default().push(match p.stars {
            5 => 99.0,
            4 => 92.0,
            3 => 85.0,
            2 => 78.0,
            _ => 70.0,
        });
    }
    let mut scores: Vec<(f64, usize)> = by
        .into_iter()
        .map(|(j, mut v)| {
            v.sort_by(|a, b| b.total_cmp(a));
            (v.iter().enumerate().map(|(n, r)| r * (-((n as f64).powi(2)) / 200.0).exp()).sum(), j)
        })
        .collect();
    scores.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    scores
}

/// Two desks with different first picks.
fn top_pick(w: &mut World, y: &crate::calendar::YearDates) {
    let year = w.year;
    let firsts: Vec<u32> = w.scouting.public.boards.iter().filter_map(|b| b.2.first().copied()).collect();
    let mut tally: std::collections::BTreeMap<u32, u32> = Default::default();
    for id in &firsts {
        *tally.entry(*id).or_default() += 1;
    }
    if tally.len() < 2 {
        return;
    }
    let mut v: Vec<(u32, u32)> = tally.into_iter().map(|(id, n)| (n, id)).collect();
    v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let (a, b) = (v[0].1, v[1].1);
    let (Some(ia), Some(_)) = (pi(w, a), pi(w, b)) else { return };
    let mut s = Spot::new(K::TopPickDebate, subj(0x70B1, year as u64))
        .lead(Who::Player(a))
        .rival(Who::Player(b))
        .due(y.draft[0].date)
        .public();
    if let Some(t) = team_of(w, ia) {
        s = s.team(t);
    }
    tell(w, s);
}

/// After the combine: the risers and the red flags.
fn combine_and_flags(w: &mut World, y: &crate::calendar::YearDates) {
    let Some(top) = crate::world::scouting::public::consensus(w) else { return };
    let before =
        w.scouting.public.weekly.iter().rev().find(|x| x.0 < y.combine_start).map(|x| x.1.clone()).unwrap_or_default();
    let mut opens: Vec<(K, usize)> = Vec::new();
    for (r, id) in top.iter().enumerate() {
        let was = before.iter().position(|x| x == id).unwrap_or(60);
        let Some(i) = pi(w, *id) else { continue };
        if was >= r + 12 && w.people[i].cal.combine_invite {
            opens.push((K::CombineRiser, i));
        }
        let flagged = w.people[i].player.history.iter().any(|h| {
            h.events.has(crate::player::DevelopmentEvent::Dismissed)
                || h.events.has(crate::player::DevelopmentEvent::Suspended)
        }) || w.people[i].on_notice;
        if flagged && r < 40 {
            opens.push((K::RedFlag, i));
        }
    }
    for (kind, i) in opens {
        let s = player_spot(w, kind, i).due(y.draft[0].date).public();
        tell(w, s);
    }
}

/// Underclassmen who declared on a late read.
fn early_entries(w: &mut World, y: &crate::calendar::YearDates) {
    let mut opens = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        if p.stage != Stage::Declared || p.college_seasons > 3 {
            continue;
        }
        let starter = p.last_role == Some(PlayingTime::Starter);
        if starter
            && p.cal.draft_read.is_some_and(|r| r.round >= 4)
            && p.college.is_some_and(|c| w.programs[c].division == Division::FBS)
        {
            opens.push(i);
        }
    }
    for i in opens.into_iter().take(12) {
        let mut s =
            Spot::new(K::EarlyEntry, s_player(pid(w, i))).lead(Who::Player(pid(w, i))).due(y.draft[0].date).public();
        if let Some(c) = w.people[i].college {
            s = s.team(TeamKey::College(c as u32));
        }
        tell(w, s);
    }
}

/// Clubs picking high with no quarterback.
fn qb_needs(w: &mut World, y: &crate::calendar::YearDates) {
    let Some(order) = crate::world::nfl_year::draft_order(w) else { return };
    let mut opens = Vec::new();
    for &t in order.iter().take(12) {
        let tk = TeamKey::Nfl(t as u16);
        let qbs = at(w, tk, Position::QB);
        let weak = qbs.first().is_none_or(|&i| ovr(w, i) < 70.0 || w.people[i].player.age >= 36);
        if weak {
            opens.push(tk);
        }
    }
    let year = w.year;
    for t in opens {
        let s = team_spot(w, K::QbNeed, t, s_season(t, year)).due(y.draft[0].date).public();
        tell(w, s);
    }
}

/// Small-school players on the first big boards.
fn small_schools(w: &mut World, y: &crate::calendar::YearDates) {
    let mut ids: Vec<u32> = w.scouting.public.boards.iter().flat_map(|b| b.2.iter().take(100).copied()).collect();
    ids.sort_unstable();
    ids.dedup();
    let mut opens = Vec::new();
    for id in ids {
        let Some(i) = pi(w, id) else { continue };
        if w.people[i].college.is_some_and(|c| !matches!(w.programs[c].division, Division::FBS)) {
            opens.push(i);
        }
    }
    let due = Date::ymd(w.year + 1, 4, 25);
    let _ = y;
    for i in opens.into_iter().take(6) {
        let s = player_spot(w, K::SmallSchoolStar, i).due(due).public();
        tell(w, s);
    }
}

/// Draft stock: the week's big risers and fallers on the consensus board.
fn stock(w: &mut World, y: &crate::calendar::YearDates) {
    let weekly = &w.scouting.public.weekly;
    let Some((date, now)) = weekly.last().cloned() else { return };
    if w.storylines.watch.board == Some(date) || weekly.len() < 2 {
        return;
    }
    let before = weekly[weekly.len() - 2].1.clone();
    w.storylines.watch.board = Some(date);
    let mut opens: Vec<(K, usize)> = Vec::new();
    for (r, id) in now.iter().enumerate() {
        let was = before.iter().position(|x| x == id).unwrap_or(60);
        if r < 32
            && was >= r + 12
            && let Some(i) = pi(w, *id)
        {
            opens.push((K::StockRise, i));
        }
    }
    for (r, id) in before.iter().enumerate().take(15) {
        let at = now.iter().position(|x| x == id).unwrap_or(60);
        if at >= r + 20
            && let Some(i) = pi(w, *id)
        {
            opens.push((K::StockFall, i));
        }
    }
    let due = if w.today < y.draft[0].date { y.draft[0].date } else { Date::ymd(w.year + 1, 4, 25) };
    for (kind, i) in opens {
        let s = player_spot(w, kind, i).due(due).public();
        tell(w, s);
    }
}

/// Conference moves, NCAA cases and building projects, as they come.
fn program_news(w: &mut World) {
    let year = w.year;
    // Realignment announced.
    let moves: Vec<(usize, String)> = w.realign.clone();
    for (p, conf) in moves {
        if w.storylines.watch.realign.contains(&(p as u32)) {
            continue;
        }
        w.storylines.watch.realign.push(p as u32);
        let t = TeamKey::College(p as u32);
        if !on(w, t) {
            continue;
        }
        let s = team_spot(w, K::Realignment, t, s_team(t))
            .word(conf.clone())
            .due(Date::ymd(year + 1, 7, 1))
            .fact(format!("{} will join the {conf}.", tname(w, t)), 5.0)
            .public();
        tell(w, s);
    }
    if w.realign.is_empty() {
        w.storylines.watch.realign.clear();
    }
    // NCAA cases.
    let mut cases = Vec::new();
    for p in &w.programs {
        let n = p.era.cases.len() as u32;
        let seen = w.storylines.watch.ncaa.get(&(p.id as u32)).copied().unwrap_or(0);
        if n > seen {
            if let Some(c) = p.era.cases.last() {
                cases.push((p.id, c.scholarships + c.postseason_years));
            }
        }
    }
    for (p, weight) in cases {
        let n = w.programs[p].era.cases.len() as u32;
        w.storylines.watch.ncaa.insert(p as u32, n);
        let t = TeamKey::College(p as u32);
        if !on(w, t) {
            continue;
        }
        let s = team_spot(w, K::NcaaCase, t, s_season(t, year))
            .nums([0, weight as i32, 0, 0])
            .due(w.today.add(60))
            .public();
        tell(w, s);
    }
    // Building projects.
    let n = w.finance.projects.len() as u32;
    if n > w.storylines.watch.projects {
        let fresh: Vec<(TeamKey, i32, String)> = w.finance.projects[w.storylines.watch.projects as usize..]
            .iter()
            .map(|p| {
                let t = match p.owner {
                    crate::world::finance::Owner::Nfl(n) => TeamKey::Nfl(n as u16),
                    crate::world::finance::Owner::College(c) => TeamKey::College(c as u32),
                    crate::world::finance::Owner::Hs(h) => TeamKey::Hs(h as u32),
                };
                (t, p.opens, p.name.clone())
            })
            .collect();
        w.storylines.watch.projects = n;
        for (t, opens, name) in fresh {
            if !on(w, t) {
                continue;
            }
            let s = team_spot(w, K::FacilityPush, t, s_team(t)).word(name).due(Date::ymd(opens, 8, 1)).public();
            tell(w, s);
        }
    }
}

/// NIL-era stars paid well under what the market would give them.
fn nil_disputes(w: &mut World, y: &crate::calendar::YearDates) {
    let mut opens = Vec::new();
    for p in &w.programs {
        if p.division != Division::FBS {
            continue;
        }
        for &i in &p.roster {
            let q = &w.people[i];
            if q.stage == Stage::College
                && q.last_role == Some(PlayingTime::Starter)
                && q.stars >= 4
                && q.mood < -0.3
                && q.nil_income < 50_000.0
                && roll(w, 0x0111, q.player.id as u64) < 0.05
            {
                opens.push(i);
            }
        }
    }
    for i in opens.into_iter().take(4) {
        let s = player_spot(w, K::NilDispute, i).due(y.portal_close);
        tell(w, s);
    }
}

/// Facts the weekly looks share with the day's.
pub(crate) fn note(w: &mut World, kind: K, subject: u64, text: String) {
    if let Some(id) = open_id(w, kind, subject) {
        fact(w, id, text, 2.0);
    }
}
