//! Firings during the season beyond the college head coach (realism audit
//! items 7 and 8, Cain 2026-11-15: "all staff can be fired during season").
//!
//! - College: on the same Sundays as the head coaches' (weeks 4-10), a head
//!   coach whose season is going wrong fires the coordinator of the side
//!   that is failing, or a weak position coach; on the hot seat he fires
//!   more, to save his own job (the Bears' Shane Waldron, November 2024).
//! - NFL: the Mondays after week 4 through the next-to-last week, an owner fires his head coach when
//!   the season has collapsed (about two a season since 2008: Saleh, Allen
//!   and Eberflus in 2024); a coordinator takes the games as the interim,
//!   and after the season the owner keeps him or opens the job. A kept head
//!   coach whose season is sliding fires a coordinator.
//! - The NFL head coach's hot seat comes first (`Staffs::seats`): the
//!   owner reads the record with the benefit of the doubt he gives a man
//!   he hired (most in his first season), the seat goes hot in public
//!   when that read is bad enough, and only a seat at least a week old can
//!   end in a firing. A coach kept on Black Monday after a year that nearly
//!   cost him the job carries the seat into the next season. Your own
//!   coach is on the same rules, with the owner's warning in your inbox.
//! - The fired man's chair stays empty for the rest of the season: the head
//!   coach (or the interim) calls that side, and the carousel fills it.
//!
//! - High school: rare, and only at schools that expect to win (a head
//!   coach at a powerhouse whose season has collapsed, or a coordinator);
//!   a fired head coach's better-regarded coordinator takes the team.
//!
//! Every size here is proposed, not confirmed.

use std::collections::HashMap;

use super::{World, college_news, hire, leave, nfl_slot, set_nfl_coaching, slot, story};
use crate::rng::clamp;
use crate::world::coach::{Employer, Group, Role};
use crate::world::data::Division;
use crate::world::folk::staffing::Seat;
use crate::world::front_office::market;
use crate::world::news::NewsKind;
use crate::world::nfl_year::NflSeason;

/// A college coordinator's chance on one Sunday at the base: a season
/// 0.15 under what was expected, the blame split evenly.
const COLLEGE_COORD: f64 = 0.007;
/// A college position coach's, before his standing on the staff.
const COLLEGE_POSITION: f64 = 0.001;
/// An NFL head coach's chance on one Monday from a hot seat, at the base
/// heat (the owner's read 0.2 over the line).
const NFL_HEAD: f64 = 0.035;
/// The benefit of the doubt an owner gives his head coach, in games of
/// .500 football added to the record he reads, by the coach's season in
/// the job (first, second, later): he hired the man, so a season has to go
/// wrong for a long time before it reads as the coach (2008-24: 3 of 36
/// in-season firings came in a first season, none before game 11; 6 in a
/// second; 27 later). An owner in a hurry gives less.
const BENEFIT: [f64; 3] = [12.0, 8.0, 3.0];
/// The read where talk of the head coach's job starts: the seat is hot.
const SEAT_HOT: f64 = 0.25;
/// The read where it stops again.
const SEAT_COOL: f64 = 0.12;
/// The Black Monday chance a kept head coach survived from which he
/// starts the next season on the hot seat.
const SEAT_CARRY: f64 = 0.2;
/// Days a seat is public before the owner can act on it.
const SEAT_NOTICE: i32 = 7;
/// An NFL coordinator's.
const NFL_COORD: f64 = 0.003;

fn key(w: &World, a: u64, b: u64) -> f64 {
    crate::world::folk::roll(w, 0x03D5_EA50 ^ a ^ (w.today.0 as u64) << 24, b)
}

/// Fires an assistant during the season, with the news for a coordinator.
fn fire_assistant(w: &mut World, e: Employer, role: Role, a: usize, record: (u32, u32)) {
    if market::spared(w, a, "fired during the season") {
        return;
    }
    leave(w, a, "fired during the season");
    if !w.recording || matches!(role, Role::PositionCoach(_)) {
        return;
    }
    let name = w.coaches[a].name.clone();
    let (wins, losses) = record;
    let side = if role == Role::OffensiveCoordinator { "offense" } else { "defense" };
    match e {
        Employer::College(p) => {
            let school = w.programs[p].name(w.options.names).to_string();
            let text = format!(
                "{school} fires {} {name} {} games into the season ({wins}-{losses}). The head coach takes over the {side}.",
                role.name(),
                wins + losses
            );
            college_news(w, p, &[a], NewsKind::CoachFired, text);
        }
        Employer::Nfl(t) => {
            let text = format!(
                "The {} fire {} {name} {} games into the season ({wins}-{losses}). The head coach takes over the {side}.",
                w.nfl[t].name,
                role.name(),
                wins + losses
            );
            let item = story(w, NewsKind::CoachFired, None, Some(t), &[a], text);
            w.news.push(item);
        }
        _ => {}
    }
}

/// College coordinators and position coaches, after the head coaches'
/// Sunday (`gone`: programs whose head coach just went; their interim keeps
/// the staff).
pub(super) fn college_assistants(w: &mut World, gone: &[usize]) {
    let Some(cs) = w.college.as_ref() else { return };
    let year = w.year;
    let era = if year < 2010 { 0.4 } else { 1.0 };
    // Points a game, scored and allowed, by division: which side failed.
    let mut mean: HashMap<Division, (f64, f64, f64)> = HashMap::new();
    for (p, t) in cs.results.iter().enumerate() {
        if t.games() >= 4 && p < w.programs.len() {
            let m = mean.entry(w.programs[p].division).or_default();
            m.0 += t.points[0] as f64 / t.games() as f64;
            m.1 += t.points[1] as f64 / t.games() as f64;
            m.2 += 1.0;
        }
    }
    let mut picks: Vec<(usize, Role, usize, f64)> = Vec::new();
    for p in 0..w.programs.len() {
        let prog = &w.programs[p];
        let Some(t) = cs.results.get(p) else { continue };
        if gone.contains(&p) || prog.sideline.is_some() || t.games() < 4 {
            continue;
        }
        // Your staff is yours to fire.
        let Some(hc) = prog.head_coach else { continue };
        if market::is_you(w, hc) {
            continue;
        }
        let bad = -t.vs_expectation() + 0.5 * (0.4 - t.win_pct()).max(0.0);
        if bad < 0.1 {
            continue;
        }
        let level = match prog.division {
            Division::FBS => 1.0,
            Division::FCS => 0.4,
            _ => 0.1,
        };
        let coach = &w.coaches[hc];
        let seat = if coach.prove_it.is_some() || coach.recent.last().is_some_and(|&r| r < -0.05) { 1.8 } else { 1.0 };
        let sev = (bad / 0.15).min(2.5);
        let (m_pf, m_pa, n) = mean.get(&prog.division).copied().unwrap_or((1.0, 1.0, 1.0));
        let (m_pf, m_pa) = ((m_pf / n).max(1.0), (m_pa / n).max(1.0));
        let pf = t.points[0] as f64 / t.games() as f64;
        let pa = t.points[1] as f64 / t.games() as f64;
        let off = clamp(0.5 + ((m_pf - pf) / m_pf - (pa - m_pa) / m_pa), 0.1, 0.9);
        for (role, blame) in [(Role::OffensiveCoordinator, off), (Role::DefensiveCoordinator, 1.0 - off)] {
            let Some(a) = slot(prog, role) else { continue };
            let first = if w.coaches[a].job.is_some_and(|j| j.season_no(year) == 1) { 0.4 } else { 1.0 };
            picks.push((p, role, a, COLLEGE_COORD * era * level * sev * 2.0 * blame * seat * first));
        }
        let rooms: Vec<(Group, usize)> =
            Group::ALL.into_iter().filter_map(|g| prog.position_coaches[g as usize].map(|a| (g, a))).collect();
        if rooms.is_empty() {
            continue;
        }
        let teach = |a: usize, g: Group| w.coaches[a].ability().teaching[g as usize];
        let avg = rooms.iter().map(|&(g, a)| teach(a, g)).sum::<f64>() / rooms.len() as f64;
        for (g, a) in rooms {
            let weak = clamp((-0.15 * (teach(a, g) - avg)).exp(), 0.5, 2.0);
            picks.push((p, Role::PositionCoach(g), a, COLLEGE_POSITION * era * level * sev * weak * seat));
        }
    }
    for (p, role, a, chance) in picks {
        let code = match role {
            Role::OffensiveCoordinator => 1,
            Role::DefensiveCoordinator => 2,
            Role::PositionCoach(g) => 3 + g as u64,
            Role::HeadCoach => 0,
        };
        if key(w, 0xC011, (p as u64) << 8 | code) >= chance || slot(&w.programs[p], role) != Some(a) {
            continue;
        }
        let record = w.college.as_ref().map_or((0, 0), |cs| (cs.results[p].wins, cs.results[p].losses));
        fire_assistant(w, Employer::College(p), role, a, record);
    }
}

/// A high school head coach's chance on one Monday at the base badness,
/// at the biggest-prestige school; a coordinator's.
const HS_HEAD: f64 = 0.002;
const HS_COORD: f64 = 0.001;

/// High school Mondays in the fall, from the fifth game on.
pub(super) fn hs_week(w: &mut World) {
    if !w.recording {
        return;
    }
    let Some(hs) = w.hs_season.as_ref() else { return };
    let year = w.year;
    let mut heads: Vec<(usize, usize)> = Vec::new();
    let mut coords: Vec<(usize, Role, usize)> = Vec::new();
    for (s, t) in hs.results.iter().enumerate() {
        let Some(school) = w.high_schools.get(s) else { continue };
        let Some(hc) = school.coach else { continue };
        if t.games() < 4 || market::is_you(w, hc) {
            continue;
        }
        let bad = -t.vs_expectation() + 0.5 * (0.4 - t.win_pct()).max(0.0);
        if bad < 0.15 {
            continue;
        }
        let sev = (bad / 0.15).min(2.5);
        let stakes = school.prestige.clamp(0.0, 1.0).powi(2);
        // In his third season or later.
        let nth = w.coaches[hc].job.map_or(1, |j| j.season_no(year));
        if nth >= 3 && key(w, 0x4511, s as u64) < HS_HEAD * sev * stakes {
            heads.push((s, hc));
            continue;
        }
        for (role, a) in [(Role::OffensiveCoordinator, school.oc), (Role::DefensiveCoordinator, school.dc)] {
            let Some(a) = a else { continue };
            let code = role.offense().map_or(0, |o| o as u64);
            if key(w, 0x4512 ^ code, s as u64) < HS_COORD * sev * stakes {
                coords.push((s, role, a));
            }
        }
    }
    for (s, hc) in heads {
        if market::spared(w, hc, "fired during the season") || w.high_schools[s].coach != Some(hc) {
            continue;
        }
        // A school with no coordinator to hand the team to waits for the
        // season's end.
        let school = &w.high_schools[s];
        let Some(i) = [school.oc, school.dc]
            .into_iter()
            .flatten()
            .filter(|&c| !market::is_you(w, c))
            .max_by(|&a, &b| w.coaches[a].reputation.total_cmp(&w.coaches[b].reputation))
        else {
            continue;
        };
        leave(w, hc, "fired during the season");
        leave(w, i, "named interim head coach");
        hire(w, i, Employer::HighSchool(s), Role::HeadCoach);
    }
    for (s, role, a) in coords {
        let school = &w.high_schools[s];
        let now = if role == Role::OffensiveCoordinator { school.oc } else { school.dc };
        if now == Some(a) && !market::spared(w, a, "fired during the season") {
            leave(w, a, "fired during the season");
        }
    }
}

/// How an owner reads his head coach during the season (`seat_read`).
#[derive(Clone, Copy, Debug)]
pub struct SeatRead {
    pub team: usize,
    pub coach: usize,
    /// His season in the job, 1 in his first.
    pub nth: i32,
    pub wins: u32,
    pub losses: u32,
    pub ties: u32,
    /// The wins his roster should have by now.
    pub expect: f64,
    /// The club's win share last season.
    pub last: f64,
    /// The owner's hurry, 0.6 (patient) to 1.4.
    pub hurry: f64,
    /// The owner's read of him: the record with the benefit of the doubt,
    /// against what the roster should have won, and his own last season.
    /// The seat goes hot at `SEAT_HOT`.
    pub bad: f64,
    /// The same read on the bare record: how the season looks to the
    /// head coach judging his coordinators.
    pub raw: f64,
    /// On the hot seat since.
    pub hot: Option<crate::calendar::Date>,
}

/// The owner's read of club `t`'s head coach on the games played so far.
/// The record counts with `BENEFIT` games of .500 football beside it, so
/// four games say little about a man in his first season and a lot about
/// one in his fourth; last season counts only when it was his, and a first
/// season softer.
fn seat_read(w: &World, s: &NflSeason, t: usize) -> Option<SeatRead> {
    let hc = w.nfl[t].head_coach?;
    let games = s.wins[t] + s.losses[t] + s.ties[t];
    if games == 0 {
        return None;
    }
    let won = s.wins[t] as f64 + 0.5 * s.ties[t] as f64;
    let expect = s.expect[t];
    let nth = w.coaches[hc].job.map_or(1, |j| j.season_no(s.year));
    let hurry = clamp(1.0 - 0.03 * (w.nfl[t].owner - 10.0), 0.6, 1.4);
    let benefit = BENEFIT[(nth - 1).clamp(0, 2) as usize] / hurry;
    let n = games as f64 + benefit;
    let held = (won + 0.5 * benefit) / n;
    let last = w.nfl[t].win_share.last().copied().unwrap_or(0.5);
    // Last season counts when it was his, and a first season as the owner
    // read it then: with the benefit of the doubt beside its games.
    let his_last = match nth {
        ..=1 => 0.5,
        2 => {
            let played = if s.year > 2021 { 17.0 } else { 16.0 };
            0.5 + (last - 0.5) * played / (played + BENEFIT[0] / hurry)
        }
        _ => last,
    };
    let bad = 1.2 * (0.5 - held) - 0.8 * (won - expect) / n + 0.4 * (0.5 - his_last);
    let share = won / games as f64;
    let raw = 1.2 * (0.5 - share) - 0.8 * (share - expect / games as f64) + 0.4 * (0.5 - last);
    let hot = w.folk.staff.seat(t, hc).map(|x| x.since);
    Some(SeatRead {
        team: t,
        coach: hc,
        nth,
        wins: s.wins[t],
        losses: s.losses[t],
        ties: s.ties[t],
        expect,
        last,
        hurry,
        bad,
        raw,
        hot,
    })
}

impl World {
    /// Every NFL owner's read of his head coach today, for the measuring
    /// tool (`seat_check`). The read is the owner's own: no screen shows it.
    #[doc(hidden)]
    pub fn nfl_seat_reads(&self) -> Vec<SeatRead> {
        let Some(s) = self.nfl_season.as_ref().filter(|s| s.started && !s.booked) else { return Vec::new() };
        (0..self.nfl.len())
            .filter(|&t| s.active.get(t).copied().unwrap_or(false))
            .filter(|&t| !self.folk.staff.interims.iter().any(|x| x.0 as usize == t))
            .filter_map(|t| seat_read(self, s, t))
            .collect()
    }
}

/// The NFL's Monday after week 4 through the next-to-last week.
pub(super) fn nfl_week(w: &mut World) {
    if !w.recording {
        return;
    }
    if !w.nfl_season.as_ref().is_some_and(|s| s.started && !s.booked) {
        return;
    }
    // Interims gone since drop off the list, and seats whose coach left.
    let heads: Vec<Option<usize>> = w.nfl.iter().map(|t| t.head_coach).collect();
    w.folk.staff.interims.retain(|x| heads.get(x.0 as usize).copied().flatten() == Some(x.1 as usize));
    w.folk.staff.seats.retain(|x| heads.get(x.team as usize).copied().flatten() == Some(x.coach as usize));
    let today = w.today;
    let Some(s) = w.nfl_season.as_ref() else { return };
    let season = s.year;
    let era = if season >= 2008 { 1.0 } else { 0.35 };
    let mut heads_out: Vec<(usize, usize, (u32, u32))> = Vec::new();
    let mut warmed: Vec<SeatRead> = Vec::new();
    let mut cooled: Vec<SeatRead> = Vec::new();
    let mut coords_out: Vec<(usize, Role, usize, (u32, u32))> = Vec::new();
    for t in 0..w.nfl.len() {
        if !s.active.get(t).copied().unwrap_or(false) {
            continue;
        }
        if w.folk.staff.interims.iter().any(|x| x.0 as usize == t) {
            continue;
        }
        let Some(r) = seat_read(w, s, t) else { continue };
        let hc = r.coach;
        let games = r.wins + r.losses + r.ties;
        if games < 4 {
            continue;
        }
        let record = (r.wins, r.losses);
        // The head coach. The seat comes first: it goes hot in public when
        // the owner's read is bad enough, and he acts only on a seat at
        // least a week old (Reich in 2023 went in his first season, after
        // eleven games; most go in their third or later).
        match r.hot {
            Some(since) => {
                let heat = clamp((r.bad - 0.15) / 0.2, 0.0, 2.0);
                let chance = NFL_HEAD * era * r.hurry * heat;
                if since.days_to(today) >= SEAT_NOTICE && key(w, 0x4EAD, t as u64) < chance {
                    heads_out.push((t, hc, record));
                    continue;
                }
                if r.bad < SEAT_COOL {
                    cooled.push(r);
                }
            }
            None if r.bad >= SEAT_HOT => warmed.push(r),
            None => {}
        }
        // His coordinators: yours are yours to fire.
        let share = (r.wins as f64 + 0.5 * r.ties as f64) / games as f64;
        if market::is_you(w, hc) || share > 0.45 || r.raw < 0.1 {
            continue;
        }
        let q = |c: Option<usize>, off: bool| c.map_or(10.0, |c| super::side_quality(&w.coaches[c], off));
        let off = clamp(0.5 + 0.08 * (q(w.nfl[t].dc, false) - q(w.nfl[t].oc, true)), 0.2, 0.8);
        // A head coach on the hot seat fires more, to save his own job.
        let seat = if r.hot.is_some() { 1.8 } else { 1.0 };
        let sev = (r.raw / 0.15).min(2.5);
        for (role, blame) in [(Role::OffensiveCoordinator, off), (Role::DefensiveCoordinator, 1.0 - off)] {
            let Some(a) = nfl_slot(&w.nfl[t], role) else { continue };
            let first = if w.coaches[a].job.is_some_and(|j| j.season_no(season) == 1) { 0.4 } else { 1.0 };
            let chance = NFL_COORD * era * sev * 2.0 * blame * seat * first;
            if key(w, 0xC00D ^ role.offense().map_or(0, |o| o as u64), t as u64) < chance {
                coords_out.push((t, role, a, record));
            }
        }
    }
    for (t, hc, record) in heads_out {
        if !market::spared(w, hc, "fired during the season") {
            fire_nfl_head(w, t, hc, record);
        }
    }
    for r in warmed {
        seat_hot(w, r);
    }
    for r in cooled {
        seat_cools(w, r);
    }
    for (t, role, a, record) in coords_out {
        if nfl_slot(&w.nfl[t], role) == Some(a) {
            fire_assistant(w, Employer::Nfl(t), role, a, record);
        }
    }
    set_nfl_coaching(w);
}

/// "3-8" or "3-8-1".
fn record_text(r: &SeatRead) -> String {
    if r.ties > 0 { format!("{}-{}-{}", r.wins, r.losses, r.ties) } else { format!("{}-{}", r.wins, r.losses) }
}

/// The seat goes hot: the talk about his job is public from today, and the
/// owner can act from next Monday. You hear it from the owner himself.
fn seat_hot(w: &mut World, r: SeatRead) {
    let today = w.today;
    w.folk.staff.seats.push(Seat { team: r.team as u32, coach: r.coach as u32, since: today, carried: false });
    let team = w.nfl[r.team].name;
    let record = record_text(&r);
    let nth = super::ordinal(r.nth.max(1) as u32);
    if w.recording {
        let text = format!(
            "The {team} are {record}, and head coach {} is on the hot seat in his {nth} season: the owner is said to be weighing a change.",
            w.coaches[r.coach].name
        );
        let item = story(w, NewsKind::CoachSeat, None, Some(r.team), &[r.coach], text);
        w.news.push(item);
    }
    if market::is_you(w, r.coach) {
        market::seat_word(
            w,
            format!(
                "The {team}'s owner called you up to his suite: {record} is not what he hired you for, and he said the results have to change. Your seat is hot, and the papers have it. He can make a change any Monday from next week on."
            ),
        );
    }
}

/// The team steadied: the talk dies down.
fn seat_cools(w: &mut World, r: SeatRead) {
    w.folk.staff.seats.retain(|x| x.team as usize != r.team);
    let team = w.nfl[r.team].name;
    let record = record_text(&r);
    if w.recording {
        let text = format!(
            "The {team} have steadied at {record}: the talk about head coach {}'s job has quieted.",
            w.coaches[r.coach].name
        );
        let item = story(w, NewsKind::CoachSeat, None, Some(r.team), &[r.coach], text);
        w.news.push(item);
    }
    if market::is_you(w, r.coach) {
        market::seat_word(
            w,
            format!(
                "The {team} are {record} and the talk about your job has quieted. The owner is off your back for now."
            ),
        );
    }
}

/// Black Monday for a head coach the owner kept (`nfl_firing`): `p` is the
/// chance of a firing he just survived. From `SEAT_CARRY` he starts the
/// next season on the hot seat, in public from today, so the owner can act
/// from its first Monday; under it the owner has backed him and any seat
/// from this season is over.
pub(super) fn carry_seat(w: &mut World, c: usize, t: usize, p: f64) {
    let was = w.folk.staff.seat(t, c).is_some();
    w.folk.staff.seats.retain(|x| x.team as usize != t);
    let team = w.nfl[t].name;
    if p < SEAT_CARRY {
        if was && market::is_you(w, c) {
            market::seat_word(
                w,
                format!(
                    "The {team}'s owner is keeping you, and he has told the building so. The talk about your job is over."
                ),
            );
        }
        return;
    }
    let today = w.today;
    w.folk.staff.seats.push(Seat { team: t as u32, coach: c as u32, since: today, carried: true });
    let next = w.coaches[c].job.map_or(2, |j| j.season_no(w.year) + 1);
    let nth = super::ordinal(next.max(1) as u32);
    if w.recording {
        let text = format!(
            "The {team} keep head coach {} for a {nth} season, but the owner says the results have to change: he starts it on the hot seat.",
            w.coaches[c].name
        );
        let item = story(w, NewsKind::CoachSeat, None, Some(t), &[c], text);
        w.news.push(item);
    }
    if market::is_you(w, c) {
        market::seat_word(
            w,
            format!(
                "The {team}'s owner is keeping you for a {nth} season, but it was close, and he told you the results have to change. You start it on the hot seat: he can make a change during the season, from the Monday after week 4."
            ),
        );
    }
}

/// The club's seat is over: its head coach is out.
pub(super) fn clear_seat(w: &mut World, t: usize) {
    w.folk.staff.seats.retain(|x| x.team as usize != t);
}

/// The owner fires his head coach in season: the better-regarded
/// coordinator takes the games as interim head coach (never you: your
/// own chair stays yours).
fn fire_nfl_head(w: &mut World, t: usize, hc: usize, record: (u32, u32)) {
    let season = w.nfl_season.as_ref().map_or(w.year, |s| s.year);
    let nth = w.coaches[hc].job.map_or(1, |j| j.season_no(season));
    clear_seat(w, t);
    leave(w, hc, "fired during the season");
    w.coaches[hc].recent.clear();
    let pick = [w.nfl[t].oc, w.nfl[t].dc]
        .into_iter()
        .flatten()
        .filter(|&c| !market::is_you(w, c))
        .max_by(|&a, &b| w.coaches[a].reputation.total_cmp(&w.coaches[b].reputation));
    let mut interim_words = String::new();
    if let Some(i) = pick {
        let role = w.coaches[i].job.map(|j| j.role);
        let now = crate::world::folk::hiring::pay_now(w, i);
        leave(w, i, "named interim head coach");
        hire(w, i, Employer::Nfl(t), Role::HeadCoach);
        // Through this season only, a little over his coordinator's pay.
        if let Some(k) = w.coaches[i].contract.as_mut() {
            k.through = season;
            k.salary = (now * 1.15).round();
        }
        w.folk.staff.interims.push((t as u32, i as u32, record.0, record.1));
        interim_words = format!(
            " {}, the {}, is the interim head coach.",
            w.coaches[i].name,
            role.map_or("assistant", |r| r.name())
        );
    }
    if w.recording {
        let (wins, losses) = record;
        let text = format!(
            "The {} fire head coach {} {} games into his {} season ({wins}-{losses}).{interim_words}",
            w.nfl[t].name,
            w.coaches[hc].name,
            wins + losses,
            super::ordinal(nth.max(1) as u32),
        );
        let item = story(w, NewsKind::CoachFired, None, Some(t), &[hc], text);
        w.news.push(item);
    }
}

/// Black Monday for an interim head coach: the owner keeps him (about a
/// third of real interims since 2010 kept the job: Jason Garrett,
/// Antonio Pierce; most didn't: Jeff Saturday, Rich Bisaccia), more often
/// the better his record. True when `c` was an interim (the caller's
/// review is done).
pub(super) fn interim_review(w: &mut World, c: usize, t: usize, openings: &mut Vec<super::Opening>) -> bool {
    let Some(k) = w.folk.staff.interims.iter().position(|x| x.0 as usize == t && x.1 as usize == c) else {
        return false;
    };
    let (_, _, w0, l0) = w.folk.staff.interims.remove(k);
    let (wins, losses) =
        w.nfl_season.as_ref().filter(|s| t < s.wins.len()).map_or((w0, l0), |s| (s.wins[t], s.losses[t]));
    let games = (wins + losses).saturating_sub(w0 + l0).max(1);
    let share = wins.saturating_sub(w0) as f64 / games as f64;
    let keep = clamp(0.1 + 1.2 * (share - 0.35), 0.05, 0.6);
    let year = w.year;
    if key(w, 0x1E7E, (t as u64) << 20 | c as u64) < keep {
        let since = w.coaches[c].job.map_or(year, |j| j.since);
        w.coaches[c].contract = Some(super::pay::nfl_contract(w, c, t, since.max(year)));
        if w.recording {
            let text =
                format!("The {} remove the interim tag: {} is their head coach.", w.nfl[t].name, w.coaches[c].name);
            let item = story(w, NewsKind::CoachHired, None, Some(t), &[c], text);
            w.news.push(item);
        }
        return true;
    }
    if market::spared(w, c, "not kept on after his interim spell") {
        return true;
    }
    leave(w, c, "not kept on after his interim spell");
    openings.push(super::Opening { employer: Employer::Nfl(t), role: Role::HeadCoach, prestige: 1.0 });
    // The new head coach brings his own coordinators (each kept 30% of
    // the time, as after a firing).
    for role in [Role::OffensiveCoordinator, Role::DefensiveCoordinator] {
        if let Some(a) = nfl_slot(&w.nfl[t], role)
            && key(w, 0x1E7F ^ role.offense().map_or(0, |o| o as u64), t as u64) >= 0.3
            && !market::spared(w, a, "let go by the new staff")
        {
            leave(w, a, "let go by the new staff");
            openings.push(super::Opening { employer: Employer::Nfl(t), role, prestige: 1.0 });
        }
    }
    if w.recording {
        let text = format!("The {} move on from interim head coach {}.", w.nfl[t].name, w.coaches[c].name);
        let item = story(w, NewsKind::CoachFired, None, Some(t), &[c], text);
        w.news.push(item);
    }
    true
}
