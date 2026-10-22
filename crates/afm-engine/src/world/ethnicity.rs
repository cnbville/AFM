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
//!
//! Asian Americans are three groups (faces item F4, so a face matches its
//! name): East Asian (Chinese, Korean, Japanese), Southeast Asian
//! (Vietnamese, Filipino, Hmong, Cambodian, Mien) and South Asian. Polynesian
//! is its own group. The split rides on the same draw as before: the old
//! Asian share is cut into three pieces at the end of the list, so one
//! uniform number still picks every other group exactly as it did, and every
//! seed's White, Black, Hispanic and Polynesian players stay who they were.

use super::geo;
use crate::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Ethnicity {
    White,
    Black,
    Hispanic,
    /// Samoan, Tongan, Hawaiian and other Pacific Islanders.
    Polynesian,
    /// Chinese, Korean and Japanese families. A world written before the
    /// split stored "Asian", which loads here and is split again by surname
    /// in `World::faces_after_load`.
    #[serde(alias = "Asian")]
    EastAsian,
    /// Vietnamese, Filipino, Hmong, Cambodian and Mien families.
    SoutheastAsian,
    /// Indian, Pakistani, Bangladeshi, Nepali and Sri Lankan families.
    SouthAsian,
}

pub const ETHNICITY_COUNT: usize = 7;

/// The three Asian groups' shares of Asian Americans (Census 2023, from the
/// faces research `asian-faces.md`): East 38.5%, Southeast 35.1%, South 26.4%.
pub const ASIAN_SPLIT: [f64; 3] = [0.385, 0.351, 0.264];

impl Ethnicity {
    pub const ALL: [Ethnicity; ETHNICITY_COUNT] = [
        Ethnicity::White,
        Ethnicity::Black,
        Ethnicity::Hispanic,
        Ethnicity::Polynesian,
        Ethnicity::EastAsian,
        Ethnicity::SoutheastAsian,
        Ethnicity::SouthAsian,
    ];
    pub const ASIAN: [Ethnicity; 3] = [Ethnicity::EastAsian, Ethnicity::SoutheastAsian, Ethnicity::SouthAsian];

    pub fn name(self) -> &'static str {
        match self {
            Ethnicity::White => "white",
            Ethnicity::Black => "Black",
            Ethnicity::Hispanic => "Hispanic",
            Ethnicity::Polynesian => "Polynesian",
            Ethnicity::EastAsian => "East Asian",
            Ethnicity::SoutheastAsian => "Southeast Asian",
            Ethnicity::SouthAsian => "South Asian",
        }
    }

    /// The key the screens and the face renderer use.
    pub fn key(self) -> &'static str {
        match self {
            Ethnicity::White => "white",
            Ethnicity::Black => "black",
            Ethnicity::Hispanic => "hispanic",
            Ethnicity::Polynesian => "polynesian",
            Ethnicity::EastAsian => "east_asian",
            Ethnicity::SoutheastAsian => "southeast_asian",
            Ethnicity::SouthAsian => "south_asian",
        }
    }

    pub fn from_key(k: &str) -> Option<Ethnicity> {
        Some(match k {
            "white" => Ethnicity::White,
            "black" => Ethnicity::Black,
            "hispanic" => Ethnicity::Hispanic,
            "polynesian" => Ethnicity::Polynesian,
            // The setup page's single Asian choice before the split.
            "east_asian" | "asian" => Ethnicity::EastAsian,
            "southeast_asian" => Ethnicity::SoutheastAsian,
            "south_asian" => Ethnicity::SouthAsian,
            _ => return None,
        })
    }

    pub fn is_asian(self) -> bool {
        matches!(self, Ethnicity::EastAsian | Ethnicity::SoutheastAsian | Ethnicity::SouthAsian)
    }
}

/// Each Asian group's share of a state's Asian children (East, Southeast,
/// South). Where a state's mix is far from the nation's: Japanese and
/// Filipino Hawaii, Filipino Nevada and Alaska, Hmong Minnesota and
/// Wisconsin, Vietnamese and Indian Texas, Indian New Jersey (Census ACS
/// shares as Claude knows them, rounded; proposed, not confirmed).
fn asian_split(code: &str) -> [f64; 3] {
    match code {
        "HI" => [0.42, 0.55, 0.03],
        "AK" => [0.30, 0.65, 0.05],
        "NV" => [0.30, 0.58, 0.12],
        "CA" => [0.37, 0.42, 0.21],
        "WA" | "OR" => [0.40, 0.38, 0.22],
        "TX" => [0.27, 0.37, 0.36],
        "NJ" => [0.30, 0.17, 0.53],
        "NY" => [0.45, 0.10, 0.45],
        "MN" => [0.22, 0.55, 0.23],
        "WI" => [0.18, 0.62, 0.20],
        "IL" | "MD" | "GA" => [0.35, 0.25, 0.40],
        "VA" => [0.30, 0.35, 0.35],
        "PA" | "MA" | "CT" => [0.45, 0.18, 0.37],
        _ => ASIAN_SPLIT,
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
        Ethnicity::EastAsian | Ethnicity::SoutheastAsian | Ethnicity::SouthAsian => 0.3,
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
        Ethnicity::EastAsian | Ethnicity::SoutheastAsian | Ethnicity::SouthAsian => -0.6,
    }
}

fn shares(state: usize) -> [f64; ETHNICITY_COUNT] {
    let code = geo::STATE_CENTERS[state].0;
    let row = STATE_SHARES.iter().find(|r| r.0 == code).copied().unwrap_or(("", 0.12, 0.2, 0.04, 0.002));
    let (_, b, h, a, pi) = row;
    let [e, se, so] = asian_split(code);
    [(1.0 - b - h - a - pi).max(0.02), b, h, pi, a * e, a * se, a * so]
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
pub fn coach_ethnicity(kind: CoachPool, year: i32, rng: &mut Rng) -> Ethnicity {
    // Young coaches entering the college ranks (trial 4 topic 9, traced
    // 2026-10-17): FBS assistants below coordinator were 34% Black and 5%
    // other minorities in 2012, 44% and 9% in 2022 (NCAA demographics, via
    // AP). Entrants run from about 27% of color by 1990 to 50% by 2015
    // (proposed, not confirmed); of them 82% Black, 8% Hispanic, 6%
    // Polynesian, 4% Asian.
    let entering = |year: i32| {
        let color = 0.27 + 0.23 * ((year - 1990) as f64 / 25.0).clamp(0.0, 1.0);
        [1.0 - color, 0.82 * color, 0.08 * color, 0.06 * color, 0.04 * color]
    };
    let w: [f64; 5] = match kind {
        CoachPool::Entering => entering(year),
        CoachPool::HighSchool => [0.78, 0.16, 0.04, 0.01, 0.01],
        CoachPool::CollegeHead1990 => [0.96, 0.03, 0.005, 0.004, 0.001],
        // The staffs already in place when the world starts (proposed, not confirmed).
        CoachPool::CollegeAssistant1990 => [0.8, 0.17, 0.015, 0.01, 0.005],
    };
    rng.weighted(&with_asian_split(w))
}

/// Weights over White, Black, Hispanic, Polynesian and Asian, with the Asian
/// weight cut into its three groups at the end (the national split): one
/// draw picks every other group exactly as the five-group list did.
pub fn with_asian_split(w: [f64; 5]) -> [(Ethnicity, f64); ETHNICITY_COUNT] {
    let [e, se, so] = ASIAN_SPLIT;
    [
        (Ethnicity::White, w[0]),
        (Ethnicity::Black, w[1]),
        (Ethnicity::Hispanic, w[2]),
        (Ethnicity::Polynesian, w[3]),
        (Ethnicity::EastAsian, w[4] * e),
        (Ethnicity::SoutheastAsian, w[4] * se),
        (Ethnicity::SouthAsian, w[4] * so),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    // pressure on hiring grow. Real progress stalled after the mid-2010s
    // (about 13 Black FBS head coaches in 2016, 16 in 2024), so the ease stops in
    // 2016; it fades slowly again after the save starts.
    let eased = 4.2 - 0.08 * (year.clamp(2005, 2016) - 2005) as f64;
    let base = (eased - 0.05 * (year - 2026).max(0) as f64).max(0.3);
    base * (0.5 + prestige.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The split keeps every seed's White, Black, Hispanic and Polynesian
    /// players: the old five-group draw, with Asian as one, picks the same.
    #[test]
    fn the_asian_split_leaves_every_other_draw_unchanged() {
        for state in 0..51 {
            let s = shares(state);
            for seed in 0..400u32 {
                let z = (seed % 7) as f64 * 0.5;
                let mut a = Rng::new(seed ^ ((state as u32) << 20));
                let mut b = a.clone();
                let new = player_ethnicity(state, z, &mut a);
                let up = (z - 1.0).max(0.0);
                let w = |k: usize, e: Ethnicity| s[k] * participation(e) * (level_lean(e) * up).exp();
                let old: Vec<(usize, f64)> = vec![
                    (0, w(0, Ethnicity::White)),
                    (1, w(1, Ethnicity::Black)),
                    (2, w(2, Ethnicity::Hispanic)),
                    (3, w(3, Ethnicity::Polynesian)),
                    (4, w(4, Ethnicity::EastAsian) + w(5, Ethnicity::SoutheastAsian) + w(6, Ethnicity::SouthAsian)),
                ];
                let was = b.weighted(&old);
                assert_eq!(was == 4, new.is_asian(), "state {state} seed {seed}");
                if was < 4 {
                    assert_eq!(new as usize, was);
                }
                assert_eq!(a.next_u32(), b.next_u32());
            }
        }
    }
}
