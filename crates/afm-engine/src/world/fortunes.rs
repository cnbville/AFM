//! What a recruit reads of a program's coaches (program fortunes lane P1,
//! `docs/design/program-fortunes.md` items 2-4, Cain 2026-11-24): the head
//! coach's own record in his school's pitch, how fast each coach's
//! recruiting builds a bond, and a transfer's bond with the staff pitching
//! him. Item 1, the name from lived results, is `program::lived`. Every
//! size here is proposed, not confirmed: the measuring pass after lanes P1
//! and P2 sizes them against the real table.

use super::World;
use super::coach::{Coach, Employer, Group, Role};

/// Program fortunes item 2: a draft from before the head coach came reads
/// this share of his own picks from his last head-coaching seasons
/// elsewhere, the rest the program's.
pub const COACH_PIPELINE: f64 = 0.5;
/// Item 2: the title chances grade reads the head coach's wins above his
/// rosters over his last three head-coaching seasons at this weight (a
/// season one win-share above reads like a win share one higher).
pub const HEAD_RECORD: f64 = 2.0;
/// Item 2: head-coaching seasons his record above his rosters reads.
pub const HEAD_RECORD_SEASONS: usize = 3;
/// Item 3: the bond a head coach builds over a recruit's year on him at a
/// recruiting skill of 8, a recruit's own target he has visited. With
/// [`BOND_BASE`] that is 0.136 at 15.5, a typical FBS head coach as a
/// recruit feels him, the same as the lane's first sizes gave there (0.06
/// at 8 on a base of 3), 0.43 at 18 and 0.03 at 12. Sized on the measuring
/// pass (2026-11-25, `docs/design/program-fortunes.md`): proposed, not
/// confirmed.
pub const HC_BOND: f64 = 0.0043;
/// Item 3: the same for his position coach.
pub const PC_BOND: f64 = 0.0022;
/// Item 3: how much faster a coach ten points better builds a bond: each
/// point of recruiting is worth 58% more. On a base of 3 a head coach one
/// spread better than the typical one (2.1 points) built 26% more bond,
/// while where the recruit lives and whether he visited moved the same
/// bond three times over, so classes didn't follow the coach at any size
/// of the bond. Sized on the measuring pass: proposed, not confirmed.
pub const BOND_BASE: f64 = 100.0;
/// Item 4: a transfer's few weeks in the portal, as a share of a
/// recruit's year: the head coach's calls (a recruit who isn't his own
/// target gets 0.35 of him) and the position coach's (0.75, a recruit who
/// hasn't visited).
pub const PORTAL_HC_TIME: f64 = 0.35;
pub const PORTAL_PC_TIME: f64 = 0.75;
/// Item 4: what a head coach who recruited him out of high school built
/// then, as a share of a full year on an own target (about half his
/// offers are his own targets).
pub const OLD_HC_TIME: f64 = 0.6;
/// Item 4: an old recruiting bond keeps this share of itself each year.
pub const OLD_TIE_FADE: f64 = 0.85;

/// How fast a coach's recruiting builds a bond (item 3, Cain 2026-11-24):
/// 1 at a skill of 8, [`BOND_BASE`] times that at 18, so 1.58 times a
/// point: 6.3 at 12, 31.6 at 15.5, 100 at 18.
pub fn bond_rate(skill: f64) -> f64 {
    BOND_BASE.powf((skill - 8.0) / 10.0)
}

/// The college seasons a stint covers: from the season after its label
/// (an offseason hire's) to the one it ended in.
fn seasons_of(from: i32, to: i32) -> std::ops::RangeInclusive<i32> {
    (from + 1).min(to)..=to
}

/// His role at college program `x` in `season`, if he coached there then.
fn held_at(c: &Coach, x: usize, season: i32) -> Option<Role> {
    if let Some(j) = c.job.filter(|j| j.employer == Employer::College(x) && j.first_season() <= season) {
        return Some(j.role);
    }
    c.history
        .iter()
        .find(|s| s.employer == Employer::College(x) && seasons_of(s.from, s.to).contains(&season))
        .map(|s| s.role)
}

/// His finished college head-coaching seasons at programs other than
/// `here`, most recent first, with the program.
pub fn head_seasons_elsewhere(c: &Coach, here: usize) -> Vec<(usize, i32)> {
    let mut out = Vec::new();
    for s in c.history.iter().rev().filter(|s| s.role == Role::HeadCoach) {
        let Employer::College(x) = s.employer else { continue };
        if x != here {
            out.extend(seasons_of(s.from, s.to).rev().map(|y| (x, y)));
        }
    }
    out
}

/// A pick's weight in the pro pipeline grade: 3 a first-rounder, 2 a
/// second or third, 1 later.
pub fn pick_weight(round: u32) -> f64 {
    match round {
        1 => 3.0,
        2 | 3 => 2.0,
        _ => 1.0,
    }
}

/// A program's round-weighted picks in one draft.
fn picks_in(w: &World, x: usize, draft: i32) -> f64 {
    w.programs[x].draft_picks.iter().filter(|p| p.0 == draft).map(|p| pick_weight(p.1)).sum()
}

/// The pro pipeline grade's read (item 2): the round-weighted picks of the
/// last four drafts, `year`'s the latest. A draft his own seasons here fed
/// is the program's. One from before he came reads half the program's
/// picks and half his own from his last head-coaching seasons elsewhere,
/// the latest draft with his latest season, while he has any: Saban's first
/// Alabama classes read his LSU picks beside Shula's. A first-time head
/// coach brings none, so his school reads its own until he builds them.
pub fn pipeline(w: &World, j: usize, year: i32) -> f64 {
    let p = &w.programs[j];
    let hc = p.head_coach.map(|c| &w.coaches[c]);
    let first = hc
        .and_then(|c| c.job)
        .filter(|job| job.employer == Employer::College(j))
        .map_or(i32::MAX, |job| job.first_season());
    let mut elsewhere = hc.map(|c| head_seasons_elsewhere(c, j)).unwrap_or_default().into_iter();
    (year - 3..=year)
        .rev()
        .map(|draft| {
            let here = picks_in(w, j, draft);
            if draft > first {
                return here;
            }
            match elsewhere.next() {
                Some((x, season)) => (1.0 - COACH_PIPELINE) * here + COACH_PIPELINE * picks_in(w, x, season + 1),
                None => here,
            }
        })
        .sum()
}

/// His record above his rosters (item 2): the mean of his last three
/// college head-coaching seasons' wins above what the roster should have
/// won, as a share of the season (`TeamResult::vs_expectation`), through
/// `year`. 0 for a man who has never been a college head coach: a
/// coordinator's first job adds nothing until he wins. His seasons here
/// replace the ones elsewhere as they come, so his past counts most in his
/// first seasons.
pub fn head_record(w: &World, c: usize, year: i32) -> f64 {
    let coach = &w.coaches[c];
    let here = coach
        .job
        .filter(|j| j.role == Role::HeadCoach && matches!(j.employer, Employer::College(_)))
        .map_or(0, |j| (year - j.first_season() + 1).max(0) as usize);
    let before: usize = coach
        .history
        .iter()
        .filter(|s| s.role == Role::HeadCoach && matches!(s.employer, Employer::College(_)))
        .map(|s| seasons_of(s.from, s.to).count())
        .sum();
    let n = (here + before).min(HEAD_RECORD_SEASONS).min(coach.recent.len());
    if n == 0 {
        return 0.0;
    }
    coach.recent.iter().rev().take(n).sum::<f64>() / n as f64
}

/// A head coach's recruiting skill as a recruit feels it: 0.6 recruiting
/// and 0.4 relationships (the name and the voice come on top where a
/// recruit hears them).
pub fn head_skill(w: &World, c: usize) -> f64 {
    let a = w.coaches[c].ability();
    0.6 * a.recruiting + 0.4 * a.relationships
}

/// A transfer's bond with program `j`'s staff (item 4, Cain 2026-11-24),
/// built the way a recruit's is (item 3): the portal's own contact, the
/// head coach's and his position coach's at each one's rate; and the
/// bond either of them built recruiting him out of high school at a school
/// that offered him and that he didn't sign with, fading 15% a year. The
/// first part is the staff's calls; the second what he remembers.
pub fn transfer_bond(w: &World, i: usize, j: usize) -> (f64, f64) {
    let p = &w.people[i];
    let prog = &w.programs[j];
    let group = Group::of(p.player.position);
    let hc = prog.head_coach;
    let pc = prog.position_coaches[group as usize];
    let pc_skill = |c: usize| w.coaches[c].ability().recruiting;
    let calls = PORTAL_HC_TIME * HC_BOND * bond_rate(hc.map_or(9.0, |c| head_skill(w, c)))
        + PORTAL_PC_TIME * PC_BOND * bond_rate(pc.map_or(9.0, pc_skill));
    // His recruiting fall: the season before his class enrolled.
    let recruited = p.class_of - 1;
    let signed = p.cal.signed.map(|s| s.program);
    let offered = || {
        p.cal.offers.iter().copied().chain(p.cal.early_offers.iter().map(|o| o.0)).filter(move |&x| Some(x) != signed)
    };
    let recruited_him = |c: usize, role: Role| offered().any(|x| held_at(&w.coaches[c], x, recruited) == Some(role));
    let fade = OLD_TIE_FADE.powi((w.year - recruited).max(0));
    let old = fade
        * (hc
            .filter(|&c| recruited_him(c, Role::HeadCoach))
            .map_or(0.0, |c| OLD_HC_TIME * HC_BOND * bond_rate(head_skill(w, c)))
            + pc.filter(|&c| recruited_him(c, Role::PositionCoach(group)))
                .map_or(0.0, |c| PC_BOND * bond_rate(pc_skill(c))));
    (calls, old)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_point_of_recruiting_builds_the_bond_faster() {
        assert!((bond_rate(8.0) - 1.0).abs() < 1e-12);
        assert!((bond_rate(18.0) / bond_rate(8.0) - BOND_BASE).abs() < 1e-9);
        assert!((bond_rate(13.0) / bond_rate(12.0) - BOND_BASE.powf(0.1)).abs() < 1e-9);
        // A typical FBS head coach (15.5) builds what the lane's first
        // sizes gave him: 0.06 at 8 on a base of 3.
        let before = 0.06 * 3f64.powf(0.75);
        assert!((HC_BOND * bond_rate(15.5) - before).abs() < 0.002);
        // A head coach one spread better (2.1 points) is worth more on an
        // own target than a recruit's own feel for a school (SD 0.07).
        assert!(HC_BOND * (bond_rate(17.6) - bond_rate(15.5)) > 0.07);
    }

    #[test]
    fn a_stint_covers_the_seasons_after_its_label() {
        assert_eq!(seasons_of(2006, 2010), 2007..=2010);
        // A mid-season hire who left the same season.
        assert_eq!(seasons_of(2010, 2010), 2010..=2010);
    }
}
