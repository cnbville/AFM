//! Ethnicity (trial 1 note 2). It never changes a rating, development or any
//! hidden trait. It shapes names (note 3) and, for coaches, the hiring bias in
//! the AI's decisions, which the player sees through the news and a yearly
//! diversity report.
//!
//! Players get an ethnicity that matches who really plays at each level: the
//! state's child population, how often each group plays high school football,
//! and the real shares in FBS (NCAA 2023: 48.9% Black). Because those shares
//! differ by level, the draw leans on how far the player will go; his ratings
//! never depend on it. It also reads the position his body fits when he is
//! made (A20, A25): real rosters stack groups by position (FBS running
//! backs 77% Black, kickers 88% white), and his ratings are built first.
//!
//! Asian Americans are three groups (faces item F4, so a face matches its
//! name): East Asian (Chinese, Korean, Japanese), Southeast Asian
//! (Vietnamese, Filipino, Hmong, Cambodian, Mien) and South Asian. Polynesian
//! is its own group. The split rides on the same draw as before: the old
//! Asian share is cut into three pieces at the end of the list, so one
//! uniform number still picks every other group exactly as it did, and every
//! seed's White, Black, Hispanic and Polynesian players stay who they were.

use super::geo;
use crate::positions::Position;
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
/// boys (proposed, not confirmed: no national source splits participation by
/// race). Re-fit with `level_lean` (A23) so each level's mix on final rosters
/// lands on the real one: FBS 48.9% Black (NCAA 2023) and 2.7% Hispanic
/// (TIDES 2018-19), the NFL 53.5% Black (TIDES 2023).
fn participation(e: Ethnicity) -> f64 {
    match e {
        Ethnicity::White => 1.0,
        Ethnicity::Black => PART_BLACK,
        Ethnicity::Hispanic => PART_HISPANIC,
        Ethnicity::Polynesian => 2.5,
        Ethnicity::EastAsian | Ethnicity::SoutheastAsian | Ethnicity::SouthAsian => 0.3,
    }
}

/// How the group's share changes with how far a player goes (A23, see
/// `participation`; 2007-08 FBS: 0.9% Pacific Islander, which has grown
/// since). It grows with his talent draw over `LEAN_RAMP` and is full where
/// FBS players start: real rosters change a lot from high school to FBS and
/// little from FBS to the NFL (48.9% to 53.5% Black), which a lean that kept
/// growing with talent could not give.
fn level_lean(e: Ethnicity) -> f64 {
    match e {
        Ethnicity::White => 0.0,
        Ethnicity::Black => LEAN_BLACK,
        Ethnicity::Hispanic => LEAN_HISPANIC,
        Ethnicity::Polynesian => 0.95,
        Ethnicity::EastAsian | Ethnicity::SoutheastAsian | Ethnicity::SouthAsian => -0.6,
    }
}

const PART_BLACK: f64 = 1.5;
const LEAN_BLACK: f64 = 1.91;
const PART_HISPANIC: f64 = 0.65;
const LEAN_HISPANIC: f64 = -1.88;
/// The talent draws over which the lean grows from none to full (an
/// average boy's draw is 0; FBS players' median is about 1.9).
const LEAN_RAMP: (f64, f64) = (0.5, 1.4);
/// The talent draws where the position odds are `ODDS_LOW`, `ODDS_MID` and
/// `ODDS_HIGH`, moving smoothly between them (players' median draw is about
/// 1.4 at FCS, 1.9 at FBS and 2.9 in the NFL).
const ODDS_AT: [f64; 3] = [1.0, 2.0, 3.0];

/// A boy with a long snapper's knack draws with the snapper's odds (A25):
/// nobody's body is built as a snapper; the camps find the best snappers
/// among the other positions as seniors (`recruiting.rs`, from his long
/// snapping, among boys no one recruits at another spot), and real
/// snappers are 94% white. His knack is his long snapping ceiling over his
/// talent, a little more for more talent; a boy talented enough to be
/// recruited as a regular (`SNAPPER_TOP`) is left out. At this mark it takes
/// in 92% of the boys the camps later make snappers, and about 4% of all
/// boys.
pub const SNAPPER_KNACK: f64 = 1.25;
/// The talent draw from which a boy is recruited as a regular, not as a
/// snapper (95% of the world's snappers are below it).
const SNAPPER_TOP: f64 = 2.6;

/// Whether a boy has a snapper's knack when he is made (see `SNAPPER_KNACK`).
pub fn snapper_knack(long_snapping_ceiling: f64, talent: f64, talent_z: f64) -> bool {
    !regular_talent(talent_z) && long_snapping_ceiling / talent.max(1.0) * (0.1 * talent_z).exp() >= SNAPPER_KNACK
}

/// Whether a boy is talented enough to be recruited as a regular
/// (`SNAPPER_TOP`): he draws his own position's odds, so the camps never
/// make him a snapper either. Before, the camps made an unrated boy his
/// school's snapper whatever his talent, and the most gifted snappers are
/// the ones the NFL keeps: the test world's NFL long snappers read 4 of 33
/// Black (real none), all four linebackers, edge rushers or safeties as
/// boys. After, 0 of 29; FBS snappers 0.4% Black and FCS 1.8% (real 3 and
/// 6; 1.5 and 4.1 before).
pub fn regular_talent(talent_z: f64) -> bool {
    talent_z >= SNAPPER_TOP
}

/// How much likelier a group is at a position than its share of the boys
/// drawn there (A20). Columns: white, Black, Hispanic, Asian or Pacific
/// Islander. Rows: QB, RB, WR, TE, OL, DL, LB, DB, K, P, LS. Measured on
/// 2025 rosters with race estimated from names (`afm/race-check/report.md`:
/// the FBS table), then fitted on the world's final rosters (A25, A24) so
/// every position at FBS, FCS and the NFL lands on its real mix at once,
/// with the overall FBS and NFL mixes held (`ODDS_AT`). Real stacking grows
/// with the level at receiver, linebacker and the line (receivers 52% Black
/// at FCS, 64% at FBS, 77% in the NFL, against 45%, 49% and 54% overall),
/// so one table can't fit every level. The high table's Black column sits
/// lower than the middle one at the skill spots, which keeps the NFL near
/// 53.5% Black while FBS is at 48.9%. Boys who stay in high school read the
/// low table (A21). The Asian or Pacific Islander column is the measured
/// one at every level. The linebackers' Black column is the fit × 0.87:
/// once FBS stopped losing talent to the portal (minor bug 131) FBS
/// linebackers read 50-53% Black (real 48%), the NFL's 65% (real 60%) and
/// FCS's 45% (real 42%), on the test world and a 12-year world alike.
/// The tight ends' is the fit × 0.88 for the same reason: the test
/// world's FBS tight ends read 21.5% Black and 74.7% white (real 18 and
/// 80), its NFL tight ends 26.4% Black (real 23), and a 12-year world's
/// FBS tight ends 18.8% Black, up from 15.1%.
const ODDS_LOW: [[f64; 4]; 11] = [
    [1.40, 0.268, 0.740, 0.740],
    [0.626, 1.65, 0.377, 0.580],
    [0.919, 0.698, 0.488, 0.530],
    [1.64, 0.193, 0.136, 0.630],
    [1.26, 0.369, 1.76, 1.53],
    [0.479, 0.685, 0.324, 1.84],
    [0.923, 0.492, 0.343, 1.11],
    [0.238, 1.55, 0.308, 0.580],
    [6.66, 0.0086, 9.04, 1.42],
    [2.02, 0.0091, 0.333, 0.110],
    [6.06, 0.0079, 0.682, 0.580],
];
const ODDS_MID: [[f64; 4]; 11] = [
    [1.96, 0.202, 0.684, 0.740],
    [0.517, 1.74, 0.408, 0.580],
    [0.644, 0.878, 0.491, 0.530],
    [1.85, 0.156, 0.135, 0.630],
    [1.46, 0.325, 1.61, 1.53],
    [0.367, 0.631, 0.330, 1.84],
    [0.887, 0.480, 0.330, 1.11],
    [0.289, 1.22, 0.288, 0.580],
    [6.32, 0.0086, 9.92, 1.42],
    [2.03, 0.0090, 0.395, 0.110],
    [6.04, 0.0078, 0.698, 0.580],
];
const ODDS_HIGH: [[f64; 4]; 11] = [
    [2.05, 0.202, 0.638, 0.740],
    [0.763, 1.03, 0.438, 0.580],
    [0.553, 0.885, 0.499, 0.530],
    [1.85, 0.139, 0.133, 0.630],
    [1.98, 0.226, 1.60, 1.53],
    [0.358, 0.416, 0.342, 1.84],
    [0.872, 0.445, 0.324, 1.11],
    [0.369, 0.879, 0.279, 0.580],
    [7.52, 0.0084, 8.96, 1.42],
    [2.08, 0.0088, 0.465, 0.110],
    [5.74, 0.0080, 0.716, 0.580],
];

/// Where a talent draw sits on a ramp, 0 to 1.
fn ramp(z: f64, (from, to): (f64, f64)) -> f64 {
    ((z - from) / (to - from)).clamp(0.0, 1.0)
}

/// A group's odds at a position for a boy of this talent. Polynesian
/// players read the Asian and Pacific Islander column (more linemen); the
/// three Asian American groups are too few on rosters to measure and get
/// none (A22).
fn position_odds(pos: Position, e: Ethnicity, talent_z: f64) -> f64 {
    let row = match pos {
        Position::QB => 0,
        Position::RB => 1,
        Position::WR => 2,
        Position::TE => 3,
        Position::OT | Position::IOL => 4,
        Position::EDGE | Position::DT => 5,
        Position::LB => 6,
        Position::CB | Position::S => 7,
        Position::K => 8,
        Position::P => 9,
        Position::LS => 10,
    };
    let col = match e {
        Ethnicity::White => 0,
        Ethnicity::Black => 1,
        Ethnicity::Hispanic => 2,
        Ethnicity::Polynesian => 3,
        Ethnicity::EastAsian | Ethnicity::SoutheastAsian | Ethnicity::SouthAsian => return 1.0,
    };
    let (from, to, r) = if talent_z <= ODDS_AT[1] {
        (&ODDS_LOW, &ODDS_MID, ramp(talent_z, (ODDS_AT[0], ODDS_AT[1])))
    } else {
        (&ODDS_MID, &ODDS_HIGH, ramp(talent_z, (ODDS_AT[1], ODDS_AT[2])))
    };
    from[row][col].powf(1.0 - r) * to[row][col].powf(r)
}

fn shares(state: usize) -> [f64; ETHNICITY_COUNT] {
    let code = geo::STATE_CENTERS[state].0;
    let row = STATE_SHARES.iter().find(|r| r.0 == code).copied().unwrap_or(("", 0.12, 0.2, 0.04, 0.002));
    let (_, b, h, a, pi) = row;
    let [e, se, so] = asian_split(code);
    [(1.0 - b - h - a - pi).max(0.02), b, h, pi, a * e, a * se, a * so]
}

/// A high school player's ethnicity. `talent_z` is his talent draw; the lean
/// only reflects who reaches each level, and nothing about him is changed by
/// it. `position` is the one his body fits as he is made (`athlete::create`,
/// A25), or None for an athlete from another sport. A later move keeps his
/// ethnicity: a kid built like a back who ends up a linebacker carries back
/// odds, which is how most real anomalies happen.
pub fn player_ethnicity(state: usize, talent_z: f64, position: Option<Position>, rng: &mut Rng) -> Ethnicity {
    let s = shares(state);
    let up = ramp(talent_z, LEAN_RAMP);
    let entries: Vec<(Ethnicity, f64)> = Ethnicity::ALL
        .iter()
        .enumerate()
        .map(|(k, &e)| {
            let odds = position.map_or(1.0, |p| position_odds(p, e, talent_z));
            (e, s[k] * participation(e) * (level_lean(e) * up).exp() * odds)
        })
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

    /// A20: the draw reads his position's odds. In Georgia an average boy
    /// built as a back is far likelier Black than the state's players as a
    /// whole, a quarterback or tight end far less, a kicker almost never;
    /// with no position the draw is the plain one.
    #[test]
    fn the_draw_follows_each_positions_odds() {
        let ga = geo::STATE_CENTERS.iter().position(|c| c.0 == "GA").expect("Georgia");
        let black = |pos: Option<Position>| {
            let mut rng = Rng::new(11);
            let n = 20_000;
            (0..n).filter(|_| player_ethnicity(ga, 0.0, pos, &mut rng) == Ethnicity::Black).count() as f64 / n as f64
        };
        let any = black(None);
        let (rb, cb, qb, te, k) = (
            black(Some(Position::RB)),
            black(Some(Position::CB)),
            black(Some(Position::QB)),
            black(Some(Position::TE)),
            black(Some(Position::K)),
        );
        assert!(rb > any + 0.15 && cb > any + 0.15, "backs {rb:.2}, corners {cb:.2}, all {any:.2}");
        assert!(qb < any - 0.15 && te < any - 0.15, "quarterbacks {qb:.2}, tight ends {te:.2}, all {any:.2}");
        assert!(k < 0.05, "kickers {k:.2}");
    }

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
                let new = player_ethnicity(state, z, None, &mut a);
                let up = ramp(z, LEAN_RAMP);
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
