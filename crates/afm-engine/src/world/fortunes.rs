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
/// [`BOND_BASE`] that is 0.55 at 15.5, a typical FBS head coach as a
/// recruit feels him (4 times what the lane's first sizes gave him: 0.136
/// on a base of 100), 1.8 at 19 and 0.17 at 12. Sized with lane P3
/// (2026-11-26, `docs/design/program-fortunes.md`, question 11): the
/// stronger arm the measuring pass set aside for its Group of 5 title
/// runs, now that a school's level holds a great recruiter's bond in check
/// (see [`held_bond`]). Proposed, not confirmed.
pub const HC_BOND: f64 = 0.0425;
/// Item 3: the same for his position coach.
pub const PC_BOND: f64 = 0.0217;
/// Item 3: how much faster a coach ten points better builds a bond: each
/// point of recruiting is worth 40% more (a base of 30 over ten points).
/// On a base of 3 a head coach one spread better than the typical one (2.1
/// points) built 26% more bond, while where the recruit lives and whether
/// he visited moved the same bond three times over, so classes didn't
/// follow the coach at any size of the bond. Sized on the measuring pass
/// and lane P3: proposed, not confirmed.
pub const BOND_BASE: f64 = 30.0;
/// The recruiting skill of a typical FBS head coach as a recruit feels
/// him (15.5, SD 2.1): the bond a coach builds up to it is the same
/// against any school, the part above it is what a school's level holds in
/// check ([`held_bond`]).
pub const TYPICAL_SKILL: f64 = 15.5;
/// Lane P3 (question 11): how far above a school's standing (its level and
/// name, `recruiting::bond_standing`, 0 to 1) the best school in a recruit's
/// field has to stand for a coach's bond above the typical one to count for
/// nothing. A great recruiter at a Group of 5 school keeps all of it
/// against the Group of 5 and the weaker Power 4 schools; against a
/// national name (about 0.17 above the best Group of 5 school) it is
/// mostly gone, and it comes back as the school's name grows with its wins.
/// Proposed, not confirmed.
pub const LEVEL_STEP: f64 = 0.25;
/// Item 4: a transfer's bond keeps the sizes of the measuring pass (a
/// base of 100 at the same typical bond, `HC_BOND` 0.0043 and `PC_BOND`
/// 0.0022 at a skill of 8): the portal has no field of schools to hold a
/// great recruiter's bond against by level yet (`GAPS.md`), so it stays
/// where the pass measured it.
const PORTAL_BASE: f64 = 100.0;
const PORTAL_HC_BOND: f64 = 0.0043;
const PORTAL_PC_BOND: f64 = 0.0022;

/// A transfer's bond rate: [`bond_rate`] at the portal's own base.
fn portal_rate(skill: f64) -> f64 {
    PORTAL_BASE.powf((skill - 8.0) / 10.0)
}

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
/// 1 at a skill of 8, [`BOND_BASE`] times that at 18, so 1.4 times a
/// point: 3.0 at 12, 12.8 at 15.5, 30 at 18.
pub fn bond_rate(skill: f64) -> f64 {
    BOND_BASE.powf((skill - 8.0) / 10.0)
}

/// The share of a coach's bond above the typical one that a recruit still
/// weighs when the best school in his field stands `gap` above the
/// coach's school (lane P3, question 11, Cain 2026-11-26): all of it at a
/// gap of 0 or less (against schools at his own level and below), none at
/// [`LEVEL_STEP`] or more.
pub fn held_bond(gap: f64) -> f64 {
    (1.0 - gap / LEVEL_STEP).clamp(0.0, 1.0)
}

/// [`bond_rate`] with the part above a typical coach's rate cut to `held`
/// ([`held_bond`]): a coach at or below the typical skill builds the same
/// bond against any school, so the typical world is unchanged.
pub fn bond_rate_held(skill: f64, held: f64) -> f64 {
    let typical = bond_rate(TYPICAL_SKILL);
    let rate = bond_rate(skill);
    if rate > typical { typical + (rate - typical) * held } else { rate }
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
    let calls = PORTAL_HC_TIME * PORTAL_HC_BOND * portal_rate(hc.map_or(9.0, |c| head_skill(w, c)))
        + PORTAL_PC_TIME * PORTAL_PC_BOND * portal_rate(pc.map_or(9.0, pc_skill));
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
            .map_or(0.0, |c| OLD_HC_TIME * PORTAL_HC_BOND * portal_rate(head_skill(w, c)))
            + pc.filter(|&c| recruited_him(c, Role::PositionCoach(group)))
                .map_or(0.0, |c| PORTAL_PC_BOND * portal_rate(pc_skill(c))));
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
        // A typical FBS head coach (15.5) builds 0.55 on an own target he
        // has visited, 4 times the sizes of the measuring pass (lane P3).
        assert!((HC_BOND * bond_rate(15.5) - 0.55).abs() < 0.01);
        // A head coach one spread better (2.1 points) is worth more on an
        // own target than a recruit's own feel for a school (SD 0.07).
        assert!(HC_BOND * (bond_rate(17.6) - bond_rate(15.5)) > 0.07);
    }

    #[test]
    fn a_schools_level_holds_a_great_recruiters_bond_in_check() {
        // Against his own level and below he keeps all of it, against a
        // school a step above him none of the part above the typical coach.
        assert_eq!(held_bond(-0.3), 1.0);
        assert_eq!(held_bond(0.0), 1.0);
        assert!((held_bond(LEVEL_STEP / 2.0) - 0.5).abs() < 1e-12);
        assert_eq!(held_bond(LEVEL_STEP), 0.0);
        assert_eq!(held_bond(1.0), 0.0);
        let typical = bond_rate(TYPICAL_SKILL);
        assert!((bond_rate_held(19.0, 1.0) - bond_rate(19.0)).abs() < 1e-9);
        assert!((bond_rate_held(19.0, 0.0) - typical).abs() < 1e-9);
        assert!(bond_rate_held(19.0, 0.5) > typical && bond_rate_held(19.0, 0.5) < bond_rate(19.0));
        // A typical or poor recruiter builds the same bond against anyone.
        assert!((bond_rate_held(TYPICAL_SKILL, 0.0) - typical).abs() < 1e-12);
        assert!((bond_rate_held(12.0, 0.0) - bond_rate(12.0)).abs() < 1e-12);
    }

    #[test]
    fn a_stint_covers_the_seasons_after_its_label() {
        assert_eq!(seasons_of(2006, 2010), 2007..=2010);
        // A mid-season hire who left the same season.
        assert_eq!(seasons_of(2010, 2010), 2010..=2010);
    }
}
