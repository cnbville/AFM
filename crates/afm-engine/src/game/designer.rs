//! Building your own plays (plays and tactics topic 8): a play made from
//! the library's parts (formation, motion, each eligible's route,
//! protection and the quarterback's progression), checked against each
//! level's real rules before it is saved (8.2), reviewed by the coordinator
//! at his own skill (8.4) and read in practice against the scout team, with
//! the noise a few practice snaps carry (8.6). The game never grades a
//! design: it plays only as well as its geometry (8.3). A saved design is a
//! numbered concept like any other, so it can be installed, called and
//! filmed (8.5, 8.7).

use super::flow::trace_snap_opt;
use super::playbook::*;
use super::roster::{Caller, GameTeam};
use crate::player::Level;
use crate::rng::Rng;

/// A play on the drawing board.
#[derive(Clone, Debug)]
pub struct Design {
    pub name: String,
    pub formation: Formation,
    /// Each player's route; a player not listed blocks (linemen) or runs
    /// his default for the protection (backs and tight ends).
    pub routes: Vec<(ORole, Route)>,
    pub reads: Vec<ORole>,
    pub protection: Protection,
    pub drop: Drop,
    pub play_action: bool,
}

/// Why a design is illegal at a level (8.2).
#[derive(Clone, Debug, PartialEq)]
pub enum Issue {
    /// Fewer than seven on the line of scrimmage.
    FewOnLine(usize),
    /// More than four in the backfield.
    ManyBacks(usize),
    /// A lineman, or a man covered up on the line, runs a route.
    Ineligible(ORole),
    /// A read that runs no route.
    ReadBlocks(ORole),
    /// A route given to someone not in the formation.
    NotOnField(ORole),
}

impl Issue {
    pub fn text(&self) -> String {
        match self {
            Issue::FewOnLine(n) => format!("only {n} on the line of scrimmage (seven needed)"),
            Issue::ManyBacks(n) => format!("{n} in the backfield (four at most)"),
            Issue::Ineligible(r) => format!("{} is not an eligible receiver here", super::scrimmage::orole_label(*r)),
            Issue::ReadBlocks(r) => format!("{} is a read but has no route", super::scrimmage::orole_label(*r)),
            Issue::NotOnField(r) => format!("{} is not in this personnel", super::scrimmage::orole_label(*r)),
        }
    }
}

/// Everyone on the field for the design's personnel, the quarterback and
/// the line included.
fn on_field(f: &Formation) -> Vec<ORole> {
    let mut v = vec![ORole::QB, ORole::LT, ORole::LG, ORole::C, ORole::RG, ORole::RT];
    v.extend_from_slice(f.personnel.roles());
    v
}

/// The real rules of formation and eligibility, the same at every level
/// for these parts (NFL, NCAA and NFHS all need seven on the line, at most
/// four backs, and only the end men of the line and the backs eligible).
pub fn legal(d: &Design, level: Level) -> Vec<Issue> {
    let _ = level;
    let mut out = Vec::new();
    let roles = on_field(&d.formation);
    let los = 30.0;
    let at: Vec<(ORole, f64, f64)> = roles
        .iter()
        .map(|&r| {
            let p = align(&d.formation, r, los, 26.65);
            (r, p.x, p.y)
        })
        .collect();
    // On the line: within a yard of the ball; a jet man is behind it.
    let on_line: Vec<&(ORole, f64, f64)> = at.iter().filter(|x| x.1 >= los - 1.0 && x.0 != ORole::QB).collect();
    if on_line.len() < 7 {
        out.push(Issue::FewOnLine(on_line.len()));
    }
    let backs = at.len() - on_line.len();
    if backs > 4 {
        out.push(Issue::ManyBacks(backs));
    }
    let (lo, hi) = on_line.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, x| (a.0.min(x.2), a.1.max(x.2)));
    for &(r, route) in &d.routes {
        if route.blocks() {
            continue;
        }
        let Some(&(_, x, y)) = at.iter().find(|a| a.0 == r) else {
            out.push(Issue::NotOnField(r));
            continue;
        };
        let lineman = r.is_line();
        let covered = x >= los - 1.0 && y > lo + 0.01 && y < hi - 0.01;
        if lineman || covered {
            out.push(Issue::Ineligible(r));
        }
    }
    for &r in &d.reads {
        let runs = d.routes.iter().any(|&(o, rt)| o == r && !rt.blocks());
        if !runs {
            out.push(Issue::ReadBlocks(r));
        }
    }
    out
}

/// What the coordinator says about a design (8.4): a sharp one catches its
/// flaws, a weak one misses them (each flaw seen with chance skill / 20).
pub fn review(d: &Design, coordinator: &Caller, rng: &mut Rng) -> Vec<String> {
    let mut flaws = Vec::new();
    let pts: Vec<(ORole, super::geom::V2)> = d
        .routes
        .iter()
        .filter(|(_, r)| !r.blocks())
        .map(|&(o, r)| {
            let start = align(&d.formation, o, 30.0, 26.65);
            let out = if start.y >= 26.65 { 1.0 } else { -1.0 };
            let (p, _) = r.points(start, 30.0, out);
            (o, p.last().copied().unwrap_or(start))
        })
        .collect();
    for (i, a) in pts.iter().enumerate() {
        for b in pts.iter().skip(i + 1) {
            if a.1.dist(b.1) < 3.0 {
                flaws.push(format!(
                    "{} and {} finish on top of each other: one defender covers both",
                    super::scrimmage::orole_label(a.0),
                    super::scrimmage::orole_label(b.0)
                ));
            }
        }
    }
    let hot = pts.iter().any(|(_, p)| p.x - 30.0 <= 6.0);
    let blockers = 5 + match d.protection {
        Protection::Standard => 0,
        Protection::Six => 1,
        Protection::Max => 2,
    };
    if !hot && blockers < 7 && d.drop != Drop::Screen {
        flaws.push("nothing short for a blitz: if they bring six, nobody is hot".to_string());
    }
    if pts.len() < 2 && d.drop != Drop::Screen {
        flaws.push("one route: a defense that takes it away leaves nothing".to_string());
    }
    if d.reads.is_empty() {
        flaws.push("no progression: the quarterback doesn't know where to look".to_string());
    }
    let sharp = (coordinator.reading / 20.0).clamp(0.05, 0.95);
    let seen: Vec<String> = flaws.into_iter().filter(|_| rng.chance(sharp)).collect();
    if seen.is_empty() { vec!["Looks good to me.".to_string()] } else { seen }
}

/// The concept a legal design becomes.
pub fn to_concept(d: &Design) -> PassConcept {
    let deepest = d
        .routes
        .iter()
        .filter(|(_, r)| !r.blocks())
        .map(|&(o, r)| {
            let start = align(&d.formation, o, 30.0, 26.65);
            let (p, _) = r.points(start, 30.0, 1.0);
            p.iter().map(|q| q.x - 30.0).fold(0.0, f64::max)
        })
        .fold(0.0, f64::max);
    let depth = match d.drop {
        Drop::Screen | Drop::Bubble => PassDepth::Screen,
        _ if deepest <= 7.0 => PassDepth::Quick,
        _ if deepest <= 16.0 => PassDepth::Medium,
        _ => PassDepth::Deep,
    };
    PassConcept {
        name: Box::leak(d.name.clone().into_boxed_str()),
        drop: d.drop,
        protection: d.protection,
        play_action: d.play_action,
        routes: Box::leak(d.routes.clone().into_boxed_slice()),
        reads: Box::leak(d.reads.clone().into_boxed_slice()),
        depth,
    }
}

/// Saves a legal design into the playbook: its concept number, or the
/// issues that stop it.
pub fn save(d: &Design, level: Level) -> Result<usize, Vec<Issue>> {
    let issues = legal(d, level);
    if issues.is_empty() { Ok(register_concept(to_concept(d))) } else { Err(issues) }
}

/// A practice read (8.6): the play run against the scout team's looks a
/// few times. The scout team's own players set how true the look is, and
/// a few snaps make a noisy read.
#[derive(Clone, Debug)]
pub struct PracticeRead {
    pub snaps: u32,
    pub yards: f64,
    pub completions: u32,
}

#[allow(clippy::too_many_arguments)]
pub fn practice(
    concept: usize,
    formation: Formation,
    offense: &GameTeam,
    scout: &GameTeam,
    looks: &[DefCall],
    snaps: u32,
    level: Level,
    seed: u64,
) -> PracticeRead {
    let call = OffCall::new(formation, PlayKind::Pass(concept), 1.0, false);
    let mut yards = 0.0;
    let mut completions = 0;
    for k in 0..snaps {
        let d = looks[k as usize % looks.len().max(1)];
        let r = trace_snap_opt(offense, scout, level, call, d, 35.0, seed.wrapping_add(k as u64), false);
        yards += r.yards;
        completions += r.complete as u32;
    }
    PracticeRead { snaps, yards: yards / snaps.max(1) as f64, completions }
}
