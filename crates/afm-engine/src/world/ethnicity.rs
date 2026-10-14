//! Ethnicity (trial 1 note 2). It never changes a rating, development or any
//! hidden trait. It shapes names (note 3) and, for coaches, the hiring bias in
//! the AI's decisions, which the player sees through the news and a yearly
//! diversity report.
//!
//! Players get an ethnicity that matches who really plays at each level: the
//! state's child population, how often each group plays high school football,
//! and the real shares in FBS (NCAA 2023: 48.9% Black). Because those shares
//! differ by level, the draw leans on how far the player will go; his ratings
//! never depend on it.

use super::geo;
use crate::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Ethnicity {
    White,
    Black,
    Hispanic,
    /// Samoan, Tongan, Hawaiian and other Pacific Islanders.
    Polynesian,
    Asian,
}

pub const ETHNICITY_COUNT: usize = 5;

impl Ethnicity {
    pub const ALL: [Ethnicity; ETHNICITY_COUNT] =
        [Ethnicity::White, Ethnicity::Black, Ethnicity::Hispanic, Ethnicity::Polynesian, Ethnicity::Asian];

    pub fn name(self) -> &'static str {
        match self {
            Ethnicity::White => "white",
            Ethnicity::Black => "Black",
            Ethnicity::Hispanic => "Hispanic",
            Ethnicity::Polynesian => "Polynesian",
            Ethnicity::Asian => "Asian",
        }
    }
}

/// Child population shares per state: Black, Hispanic, Asian, Pacific Islander
/// (white is the rest). Approximate, from Census child-population figures as
/// Claude knows them; no Census download was reachable (proposed, not confirmed).
const STATE_SHARES: [(&str, f64, f64, f64, f64); 51] = [
    ("AL", 0.31, 0.09, 0.015, 0.001),
    ("AK", 0.03, 0.13, 0.07, 0.025),
    ("AZ", 0.05, 0.45, 0.035, 0.003),
    ("AR", 0.18, 0.14, 0.02, 0.006),
    ("CA", 0.05, 0.52, 0.13, 0.005),
    ("CO", 0.05, 0.34, 0.035, 0.002),
    ("CT", 0.12, 0.25, 0.05, 0.001),
    ("DE", 0.27, 0.18, 0.04, 0.001),
    ("DC", 0.55, 0.17, 0.04, 0.001),
    ("FL", 0.20, 0.32, 0.03, 0.001),
    ("GA", 0.34, 0.15, 0.04, 0.001),
    ("HI", 0.02, 0.20, 0.30, 0.25),
    ("ID", 0.01, 0.19, 0.015, 0.003),
    ("IL", 0.15, 0.27, 0.05, 0.001),
    ("IN", 0.11, 0.12, 0.025, 0.001),
    ("IA", 0.05, 0.12, 0.025, 0.002),
    ("KS", 0.07, 0.19, 0.03, 0.001),
    ("KY", 0.09, 0.08, 0.015, 0.001),
    ("LA", 0.37, 0.09, 0.02, 0.001),
    ("ME", 0.02, 0.03, 0.015, 0.001),
    ("MD", 0.31, 0.17, 0.06, 0.001),
    ("MA", 0.08, 0.20, 0.07, 0.001),
    ("MI", 0.17, 0.10, 0.035, 0.001),
    ("MN", 0.10, 0.11, 0.07, 0.001),
    ("MS", 0.43, 0.05, 0.01, 0.001),
    ("MO", 0.14, 0.07, 0.02, 0.003),
    ("MT", 0.01, 0.07, 0.01, 0.001),
    ("NE", 0.06, 0.20, 0.03, 0.001),
    ("NV", 0.10, 0.41, 0.08, 0.010),
    ("NH", 0.02, 0.08, 0.03, 0.001),
    ("NJ", 0.14, 0.28, 0.10, 0.001),
    ("NM", 0.02, 0.61, 0.015, 0.001),
    ("NY", 0.16, 0.26, 0.09, 0.001),
    ("NC", 0.22, 0.17, 0.035, 0.001),
    ("ND", 0.04, 0.09, 0.015, 0.001),
    ("OH", 0.15, 0.08, 0.025, 0.001),
    ("OK", 0.08, 0.17, 0.025, 0.002),
    ("OR", 0.03, 0.23, 0.05, 0.008),
    ("PA", 0.12, 0.13, 0.04, 0.001),
    ("RI", 0.07, 0.27, 0.03, 0.001),
    ("SC", 0.30, 0.11, 0.02, 0.001),
    ("SD", 0.03, 0.13, 0.015, 0.001),
    ("TN", 0.19, 0.12, 0.02, 0.001),
    ("TX", 0.12, 0.50, 0.05, 0.0015),
    ("UT", 0.015, 0.19, 0.025, 0.020),
    ("VT", 0.02, 0.03, 0.02, 0.001),
    ("VA", 0.21, 0.16, 0.07, 0.001),
    ("WA", 0.05, 0.23, 0.10, 0.012),
    ("WV", 0.04, 0.02, 0.01, 0.001),
    ("WI", 0.09, 0.13, 0.04, 0.001),
    ("WY", 0.01, 0.14, 0.01, 0.001),
];

/// How often boys in each group play high school football, relative to white
/// boys (proposed, not confirmed: no national source splits participation by race).
fn participation(e: Ethnicity) -> f64 {
    match e {
        Ethnicity::White => 1.0,
        Ethnicity::Black => 1.5,
        Ethnicity::Hispanic => 0.65,
        Ethnicity::Polynesian => 2.5,
        Ethnicity::Asian => 0.3,
    }
}

/// How the group's share changes with how far a player goes. Calibrated so
/// FBS rosters land near the real mix (NCAA 2023: 48.9% Black; 2007-08:
/// 46.6% white, 0.9% Pacific Islander, which has grown since).
fn level_lean(e: Ethnicity) -> f64 {
    match e {
        Ethnicity::White => 0.0,
        Ethnicity::Black => 0.95,
        Ethnicity::Hispanic => -0.6,
        Ethnicity::Polynesian => 0.95,
        Ethnicity::Asian => -0.6,
    }
}

fn shares(state: usize) -> [f64; ETHNICITY_COUNT] {
    let code = geo::STATE_CENTERS[state].0;
    let row = STATE_SHARES.iter().find(|r| r.0 == code).copied().unwrap_or(("", 0.12, 0.2, 0.04, 0.002));
    let (_, b, h, a, pi) = row;
    [(1.0 - b - h - a - pi).max(0.02), b, h, pi, a]
}

/// A high school player's ethnicity. `talent_z` is his talent draw; the lean
/// only reflects who reaches each level, and nothing about him is changed by it.
pub fn player_ethnicity(state: usize, talent_z: f64, rng: &mut Rng) -> Ethnicity {
    let s = shares(state);
    let up = (talent_z - 1.0).max(0.0);
    let entries: Vec<(Ethnicity, f64)> = Ethnicity::ALL
        .iter()
        .enumerate()
        .map(|(k, &e)| (e, s[k] * participation(e) * (level_lean(e) * up).exp()))
        .collect();
    rng.weighted(&entries)
}

/// Where a new coach comes from. Coaches are mostly former players, so the
/// pool entering coaching is diverse; who gets hired is where the gap opens
/// (see `hiring_bias`). `head_job_start` covers the head coaches already in
/// place when the world begins (1990): about 3% of major-college head coaches
/// were Black then. Shares are proposed, not confirmed.
pub fn coach_ethnicity(kind: CoachPool, rng: &mut Rng) -> Ethnicity {
    let w: [f64; ETHNICITY_COUNT] = match kind {
        CoachPool::Entering => [0.6, 0.34, 0.035, 0.015, 0.01],
        CoachPool::HighSchool => [0.78, 0.16, 0.04, 0.01, 0.01],
        CoachPool::CollegeHead1990 => [0.96, 0.03, 0.005, 0.004, 0.001],
        CoachPool::CollegeAssistant1990 => [0.88, 0.1, 0.01, 0.007, 0.003],
    };
    let entries: Vec<(Ethnicity, f64)> = Ethnicity::ALL.iter().copied().zip(w).collect();
    rng.weighted(&entries)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoachPool {
    Entering,
    HighSchool,
    CollegeHead1990,
    CollegeAssistant1990,
}

/// The hidden "fit" bias an AI search committee carries against a coach of
/// color, in hiring-score points (trial 1 note 2). It is strongest in the
/// history, eases from 2005 and keeps fading slowly after the save starts. Bigger jobs carry more of
/// it, so coaches of color more often get the harder jobs. Calibrated toward
/// TIDES counts: 7 Black FBS head coaches of 120 in 2009, 16 of 134 in 2024
/// (proposed, not confirmed). The player's own hires never carry it.
pub fn hiring_bias(e: Ethnicity, year: i32, prestige: f64) -> f64 {
    if e == Ethnicity::White {
        return 0.0;
    }
    // Flat through the early 2000s, then easing as the Rooney Rule era and
    // pressure on hiring grow; it keeps fading slowly after the save starts.
    let eased = 4.2 - 0.08 * (year.min(2026) - 2005).max(0) as f64;
    let base = (eased - 0.05 * (year - 2026).max(0) as f64).max(0.3);
    base * (0.5 + prestige.clamp(0.0, 1.0))
}
