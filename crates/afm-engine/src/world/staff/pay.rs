//! The coaching market (realism audit item 6, Cain 2026-11-15): a coach's
//! pay is bid up by who else wants him and by his results, not set by the
//! job alone.
//!
//! - The job's line (`salary`) is now only what that employer usually pays
//!   for the job: its budget line.
//! - What it can pay at most (`cap`) is its line stretched: a college's
//!   line reads what its department earns (realism audit item 9 on the
//!   seed 321 world), an NFL owner's line his money.
//! - His market read (`refresh`, `read`) is what the market says he is
//!   worth on his rung: the men of one level and rung ranked by how hot
//!   they are, the jobs ranked by their lines, the k-th hottest man worth
//!   the k-th richest job's line, the top names more for the bidding.
//! - A search's offer (`offer`) beats what he makes and his read; his
//!   school can counter (`auction`): both go up until one can't, and the
//!   price lands just past the loser's top. He leaves at it or stays with
//!   the raise.
//! - A kept head coach after a good year, and an assistant whose deal runs
//!   out, re-sign at their read (`renewed`).
//!
//! Every size here is proposed, not confirmed.

use super::{hotness, salary};
use crate::world::World;
use crate::world::coach::{Contract, Employer, Role};
use crate::world::data::Division;

/// A college stretches its line this far for one man, by rung (head coach,
/// coordinator, position coach): Kirby Smart's $13.3M (2024) against the
/// SEC's $7.5M average and a MAC school's top $1.26M against its $0.85M
/// (USA Today), the best coordinators' $3M, the best position coaches'
/// $1M+. Before realism audit item 9 it was a share of a budget read by
/// prestige (7%, 1.6%, 0.6%), so a Group of 5 school could pay $4-6M.
const COLLEGE_STRETCH: [f64; 3] = [1.4, 1.5, 1.5];
/// An NFL owner stretches his line this far for the man he wants (Andy
/// Reid, reported over $20M).
const NFL_STRETCH: f64 = 1.6;
/// The top names of a rung go for more than the k-th line: they are bid on
/// by several jobs at once. Rank 0 gets 1 + this, fading by half a rank.
const TOP_PREMIUM: f64 = 0.25;
/// A search's first offer: the first name on its list gets this times the
/// line, each name further down 0.03 less, never under 0.85.
const FIRST_WANT: f64 = 1.15;
/// A man in a job wants this much more to move across or down; a promotion
/// pays its own way.
const MOVE_PREMIUM: f64 = 1.15;
const PROMOTION_PREMIUM: f64 = 1.05;
/// A man out of work has no leverage.
const OUT_OF_WORK: f64 = 0.9;
/// His school goes up to this much over his read (or his pay) to keep him,
/// the suitor up to this much over its first offer.
const KEEP_STRETCH: f64 = 1.25;
const SUITOR_STRETCH: f64 = 1.3;
/// The chance his school counters when it wants to keep him.
const COUNTER_CHANCE: f64 = 0.7;

/// The market group of a job: (level, rung), or none for a high school and
/// the other leagues, which pay their own fixed ways.
fn group(w: &World, e: Employer, role: Role) -> Option<(u8, u8)> {
    let rung = match role {
        Role::HeadCoach => 0,
        Role::OffensiveCoordinator | Role::DefensiveCoordinator => 1,
        Role::PositionCoach(_) => 2,
    };
    let level = match e {
        Employer::College(p) => match w.programs[p].division {
            Division::FBS => 0,
            Division::FCS => 1,
            _ => 2,
        },
        Employer::Nfl(_) => 3,
        _ => return None,
    };
    Some((level, rung))
}

/// The most an employer can pay one man in a job a year.
pub(crate) fn cap(w: &World, e: Employer, role: Role) -> f64 {
    let line = salary(w, e, role);
    let rung = match role {
        Role::HeadCoach => 0,
        Role::OffensiveCoordinator | Role::DefensiveCoordinator => 1,
        Role::PositionCoach(_) => 2,
    };
    match e {
        Employer::College(_) => line * COLLEGE_STRETCH[rung],
        Employer::Nfl(_) => line * NFL_STRETCH,
        _ => line,
    }
}

/// Every college and NFL coach's market read on his rung, refreshed before
/// each carousel and each signing day (kept on the save, so a loaded world
/// plays the same).
pub(crate) fn refresh(w: &mut World) {
    let mut men: std::collections::HashMap<(u8, u8), Vec<(usize, f64)>> = Default::default();
    let mut lines: std::collections::HashMap<(u8, u8), Vec<f64>> = Default::default();
    for p in 0..w.programs.len() {
        if !w.programs[p].plays(w.year) && !w.programs[p].plays(w.year + 1) {
            continue;
        }
        let e = Employer::College(p);
        for role in super::college_roles() {
            let Some(g) = group(w, e, role) else { continue };
            lines.entry(g).or_default().push(salary(w, e, role));
        }
    }
    for t in 0..w.nfl.len() {
        if !crate::world::franchise::active_in(w, t, w.year) && !crate::world::franchise::active_in(w, t, w.year + 1) {
            continue;
        }
        let e = Employer::Nfl(t);
        for role in super::nfl_roles() {
            lines.entry((3, group(w, e, role).map_or(0, |g| g.1))).or_default().push(salary(w, e, role));
        }
    }
    for c in 0..w.coaches.len() {
        let Some(job) = w.coaches[c].job else { continue };
        let Some(g) = group(w, job.employer, job.role) else { continue };
        men.entry(g).or_default().push((c, hotness(w, c)));
    }
    let mut read: Vec<(u32, f32)> = Vec::new();
    for (g, mut list) in men {
        let mut ls = lines.remove(&g).unwrap_or_default();
        if ls.is_empty() {
            continue;
        }
        ls.sort_by(|a, b| b.total_cmp(a));
        list.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        for (k, (c, _)) in list.into_iter().enumerate() {
            let line = ls[k.min(ls.len() - 1)];
            let premium = 1.0 + TOP_PREMIUM * (-(k as f64) / 2.0).exp();
            read.push((c as u32, (line * premium) as f32));
        }
    }
    read.sort_by_key(|x| x.0);
    w.folk.staff.market = read;
    ensure_nfl_contracts(w);
}

/// His market read on his rung, if he has one.
pub(crate) fn read(w: &World, c: usize) -> Option<f64> {
    let m = &w.folk.staff.market;
    m.binary_search_by_key(&(c as u32), |x| x.0).ok().map(|k| m[k].1 as f64)
}

/// NFL head coaches sign deals too (four or five years, all of it owed on
/// a firing): a world from before this change gives each one now.
fn ensure_nfl_contracts(w: &mut World) {
    let year = w.year;
    for t in 0..w.nfl.len() {
        let Some(c) = w.nfl[t].head_coach else { continue };
        if w.coaches[c].contract.is_some() {
            continue;
        }
        let since = w.coaches[c].job.map_or(year, |j| j.since);
        let line = salary(w, Employer::Nfl(t), Role::HeadCoach);
        let salary = line * (0.8 + 0.4 * w.coaches[c].reputation / 20.0);
        let mut through = since + 4;
        if through < year {
            through = year + (crate::world::folk::roll(w, 0x000C_0AC4, c as u64) * 3.0) as i32;
        }
        w.coaches[c].contract = Some(Contract { through, salary, buyout_share: 1.0 });
    }
}

/// A new NFL head coach's deal: five years, fully owed.
pub(crate) fn nfl_contract(w: &World, c: usize, t: usize, since: i32) -> Contract {
    let line = salary(w, Employer::Nfl(t), Role::HeadCoach);
    Contract { through: since + 4, salary: line * (0.8 + 0.4 * w.coaches[c].reputation / 20.0), buyout_share: 1.0 }
}

fn is_promotion(from: Role, to: Role) -> bool {
    let r = |x: Role| match x {
        Role::HeadCoach => 2,
        Role::OffensiveCoordinator | Role::DefensiveCoordinator => 1,
        Role::PositionCoach(_) => 0,
    };
    r(to) > r(from)
}

/// A search's first offer to the man at `rank` on its list.
pub(crate) fn offer(w: &World, c: usize, e: Employer, role: Role, rank: usize) -> f64 {
    let line = salary(w, e, role);
    let top = cap(w, e, role);
    let want = (FIRST_WANT - 0.03 * rank as f64).max(0.85);
    let co = &w.coaches[c];
    let Some(job) = co.job else {
        return (line * want * OUT_OF_WORK).min(top);
    };
    let now = crate::world::folk::hiring::pay_now(w, c);
    let promotion = is_promotion(job.role, role);
    let mut x = line * want;
    x = x.max(now * if promotion { PROMOTION_PREMIUM } else { MOVE_PREMIUM });
    // His read prices him on his own rung and level only.
    if !promotion && group(w, job.employer, job.role) == group(w, e, role) {
        x = x.max(read(w, c).unwrap_or(0.0));
    }
    x.min(top.max(line))
}

/// The bidding when his school hears he means to go: it counters up to
/// what it would pay to keep him, the suitor answers up to its own top,
/// 5% at a time. Returns (the suitor's last offer, what he'd make if he
/// stays), or none when his school doesn't counter.
pub(crate) fn auction(w: &World, c: usize, offer: f64, e: Employer, role: Role) -> Option<(f64, f64)> {
    let co = &w.coaches[c];
    let job = co.job?;
    if !matches!(job.employer, Employer::College(_) | Employer::Nfl(_)) {
        return None;
    }
    // Your own staff is yours to keep or let go; the world doesn't counter
    // for you.
    let you = crate::world::front_office::market::me(w);
    let boss = match job.employer {
        Employer::College(p) => w.programs[p].head_coach,
        Employer::Nfl(t) => w.nfl[t].head_coach,
        _ => None,
    };
    if you.is_some() && (you == boss || you == Some(c)) {
        return None;
    }
    // It wants him back: his results at or over what was expected, or a
    // name above the middle of the profession.
    let good = co.recent.last().is_some_and(|&r| r >= 0.0) || co.reputation >= 11.0;
    if !good || crate::world::folk::roll(w, 0xB1D5, (c as u64) << 20 ^ w.today.0 as u64) >= COUNTER_CHANCE {
        return None;
    }
    let keep = keep_top(w, c);
    if keep <= offer {
        return None;
    }
    let suitor = (offer * SUITOR_STRETCH).min(cap(w, e, role).max(offer));
    Some(if suitor > keep { (suitor.min(keep * 1.05), keep) } else { (suitor, keep.min(suitor * 1.05)) })
}

/// The most his school would pay to keep him: his read (or his pay, if
/// more) and a quarter on top, within what it can pay.
pub(crate) fn keep_top(w: &World, c: usize) -> f64 {
    let now = crate::world::folk::hiring::pay_now(w, c);
    let Some(job) = w.coaches[c].job else { return now };
    (now.max(read(w, c).unwrap_or(0.0)) * KEEP_STRETCH).min(cap(w, job.employer, job.role).max(now))
}

/// Sets a man's pay a year (a head coach's contract, an assistant's deal),
/// and his deal runs at least through next season.
pub(crate) fn set(w: &mut World, c: usize, pay: f64) {
    let year = w.year;
    if let Some(k) = w.coaches[c].contract.as_mut() {
        k.salary = pay.round();
        return;
    }
    match w.folk.staff.deals.iter_mut().find(|d| d.coach == c) {
        Some(d) => {
            d.salary = pay.round();
            d.through = d.through.max(year + 1);
        }
        None => {
            crate::world::folk::staffing::sign(w, c);
            if let Some(d) = w.folk.staff.deals.iter_mut().find(|d| d.coach == c) {
                d.salary = pay.round();
            }
        }
    }
}

/// A kept man's new pay when his deal is redone: his read when the market
/// says more, else a small raise; never a cut, never past what his
/// employer can pay.
pub(crate) fn renewed(w: &World, c: usize, now: f64, raise: f64) -> f64 {
    let Some(job) = w.coaches[c].job else { return now };
    let top = cap(w, job.employer, job.role).max(now);
    (now * raise).max(read(w, c).unwrap_or(0.0)).min(top).max(now)
}

/// The hot names whose read is well over their pay: their schools extend
/// them before someone else comes calling (Smart's 2022 and 2024 deals).
pub(crate) fn underpaid(w: &World, c: usize, now: f64) -> bool {
    read(w, c).is_some_and(|r| r > now * 1.2)
}

/// He stayed for his school's counter: the raise, a year more on his deal,
/// and, for a head coach or a coordinator at the FBS or in the NFL, the news.
pub(crate) fn kept_by_counter(w: &mut World, c: usize, stay: f64, o: super::Opening) {
    let now = crate::world::folk::hiring::pay_now(w, c);
    let Some(job) = w.coaches[c].job else { return };
    if stay <= now * 1.01 {
        return;
    }
    set(w, c, stay);
    let year = w.year;
    if let Some(k) = w.coaches[c].contract.as_mut() {
        k.through = k.through.max(year + 2);
    } else if let Some(d) = w.folk.staff.deals.iter_mut().find(|d| d.coach == c) {
        d.through = d.through.max(year + 1);
    }
    let big = match job.employer {
        Employer::College(p) => w.programs[p].division == Division::FBS,
        Employer::Nfl(_) => true,
        _ => false,
    };
    if !w.recording || !big || matches!(job.role, Role::PositionCoach(_)) {
        return;
    }
    let here = super::employer_title(w, job.employer);
    let there = super::employer_title(w, o.employer);
    let text = format!(
        "{here} {} {} {} with a raise and a longer deal after {there} came calling about its {} job.",
        if matches!(job.employer, Employer::Nfl(_)) { "keep" } else { "keeps" },
        job.role.name(),
        w.coaches[c].name,
        o.role.name()
    );
    match job.employer {
        Employer::College(p) => super::college_news(w, p, &[c], crate::world::news::NewsKind::CoachExtended, text),
        Employer::Nfl(t) => {
            let item = super::story(w, crate::world::news::NewsKind::CoachExtended, None, Some(t), &[c], text);
            w.news.push(item);
        }
        _ => {}
    }
}

/// A kept NFL head coach after the season: a winning year with a year or
/// less left, or a hot name the market prices well over his pay, gets
/// three more years at his read; an expiring deal after a poor year that
/// didn't cost him the job runs two more at the same pay.
pub(crate) fn nfl_kept(w: &mut World, c: usize, last_share: f64) {
    let year = w.year;
    let Some(k) = w.coaches[c].contract else { return };
    let good = last_share >= 0.6 || w.coaches[c].titles > 0;
    if good && (k.through - year <= 1 || underpaid(w, c, k.salary)) {
        let salary = renewed(w, c, k.salary, 1.05);
        w.coaches[c].contract = Some(Contract { through: k.through.max(year + 3), salary, ..k });
    } else if k.through < year {
        w.coaches[c].contract = Some(Contract { through: year + 2, ..k });
    }
}
