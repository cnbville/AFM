//! Firings during the season beyond the college head coach (realism audit
//! items 7 and 8, Cain 2026-11-15: "all staff can be fired during season").
//!
//! - College: on the same Sundays as the head coaches' (weeks 4-10), a head
//!   coach whose season is going wrong fires the coordinator of the side
//!   that is failing, or a weak position coach; on the hot seat he fires
//!   more, to save his own job (the Bears' Shane Waldron, November 2024).
//! - NFL: the Mondays after weeks 4-15, an owner fires his head coach when
//!   the season has collapsed (about two a season since 2008: Saleh, Allen
//!   and Eberflus in 2024); a coordinator takes the games as the interim,
//!   and after the season the owner keeps him or opens the job. A kept head
//!   coach whose season is sliding fires a coordinator.
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
use crate::world::front_office::market;
use crate::world::news::NewsKind;

/// A college coordinator's chance on one Sunday at the base: a season
/// 0.15 under what was expected, the blame split evenly.
const COLLEGE_COORD: f64 = 0.007;
/// A college position coach's, before his standing on the staff.
const COLLEGE_POSITION: f64 = 0.001;
/// An NFL head coach's chance on one Monday when the season reads at its
/// base badness (0.35 over the line).
const NFL_HEAD: f64 = 0.025;
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
            let first = if w.coaches[a].job.is_some_and(|j| j.since >= year) { 0.4 } else { 1.0 };
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
        let tenure = year - w.coaches[hc].job.map_or(year, |j| j.since);
        if tenure >= 2 && key(w, 0x4511, s as u64) < HS_HEAD * sev * stakes {
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

/// The NFL's Monday after weeks 4-15.
pub(super) fn nfl_week(w: &mut World) {
    if !w.recording {
        return;
    }
    if !w.nfl_season.as_ref().is_some_and(|s| s.started && !s.booked) {
        return;
    }
    // Interims gone since drop off the list.
    let heads: Vec<Option<usize>> = w.nfl.iter().map(|t| t.head_coach).collect();
    w.folk.staff.interims.retain(|x| heads.get(x.0 as usize).copied().flatten() == Some(x.1 as usize));
    let Some(s) = w.nfl_season.as_ref() else { return };
    let season = s.year;
    let era = if season >= 2008 { 1.0 } else { 0.35 };
    let mut heads_out: Vec<(usize, usize, (u32, u32))> = Vec::new();
    let mut coords_out: Vec<(usize, Role, usize, (u32, u32))> = Vec::new();
    for t in 0..w.nfl.len() {
        if !s.active.get(t).copied().unwrap_or(false) {
            continue;
        }
        let Some(hc) = w.nfl[t].head_coach else { continue };
        if w.folk.staff.interims.iter().any(|x| x.0 as usize == t) {
            continue;
        }
        let games = s.wins[t] + s.losses[t] + s.ties[t];
        if games < 4 {
            continue;
        }
        let share = (s.wins[t] as f64 + 0.5 * s.ties[t] as f64) / games as f64;
        let vs = share - s.expect[t] / games as f64;
        let last = w.nfl[t].win_share.last().copied().unwrap_or(0.5);
        let bad = 1.2 * (0.5 - share) - 0.8 * vs + 0.4 * (0.5 - last);
        let tenure = season - w.coaches[hc].job.map_or(season, |j| j.since);
        let record = (s.wins[t], s.losses[t]);
        // The head coach: a season at or under 35%, read against what the
        // roster should have won and last year's (Reich in 2023 went in his
        // first season; most go in their third or later).
        if share <= 0.35 {
            let tf = match tenure {
                ..=0 => 0.15,
                1 => 0.5,
                _ => 1.0,
            };
            let patience = clamp(1.0 - 0.03 * (w.nfl[t].owner - 10.0), 0.6, 1.4);
            let chance = NFL_HEAD * era * patience * tf * clamp((bad - 0.15) / 0.2, 0.0, 2.0);
            if key(w, 0x4EAD, t as u64) < chance {
                heads_out.push((t, hc, record));
                continue;
            }
        }
        // His coordinators: yours are yours to fire.
        if market::is_you(w, hc) || share > 0.45 || bad < 0.1 {
            continue;
        }
        let q = |c: Option<usize>, off: bool| c.map_or(10.0, |c| super::side_quality(&w.coaches[c], off));
        let off = clamp(0.5 + 0.08 * (q(w.nfl[t].dc, false) - q(w.nfl[t].oc, true)), 0.2, 0.8);
        let seat = if tenure >= 2 && (last < 0.4 || bad > 0.3) { 1.8 } else { 1.0 };
        let sev = (bad / 0.15).min(2.5);
        for (role, blame) in [(Role::OffensiveCoordinator, off), (Role::DefensiveCoordinator, 1.0 - off)] {
            let Some(a) = nfl_slot(&w.nfl[t], role) else { continue };
            let first = if w.coaches[a].job.is_some_and(|j| j.since >= season) { 0.4 } else { 1.0 };
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
    for (t, role, a, record) in coords_out {
        if nfl_slot(&w.nfl[t], role) == Some(a) {
            fire_assistant(w, Employer::Nfl(t), role, a, record);
        }
    }
    set_nfl_coaching(w);
}

/// The owner fires his head coach in season: the better-regarded
/// coordinator takes the games as interim head coach (never you: your
/// own chair stays yours).
fn fire_nfl_head(w: &mut World, t: usize, hc: usize, record: (u32, u32)) {
    let season = w.nfl_season.as_ref().map_or(w.year, |s| s.year);
    let tenure = season - w.coaches[hc].job.map_or(season, |j| j.since);
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
            super::ordinal((tenure + 1).max(1) as u32),
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
