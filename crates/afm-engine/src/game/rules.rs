//! The real rules of each level (topic 5). Only the rules differ by level:
//! how good the play looks comes from the players.

use crate::attributes::Attr;
use crate::player::Level;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Overtime {
    /// NFL regular season: one 10-minute period, both teams get the ball
    /// (2025 rule), ties allowed.
    NflPeriod,
    /// College: each team gets a possession from the 25; from the second
    /// overtime two-point tries are required, from the third a two-point
    /// shootout.
    College,
    /// High school (NFHS "Kansas plan"): possessions from the 10.
    HighSchool,
    /// The old NFL rule, kept by the leagues that played under it: a
    /// 15-minute period, the first score wins, a tie in the regular season
    /// if nobody scores (`books.rs`).
    SuddenDeath,
    /// CFL: each team gets a possession from the other's 35; from the
    /// `two_from` season on a touchdown must be followed by a two-point
    /// convert; at most two rounds in the regular season, then a tie.
    Cfl { two_from: i32 },
    /// A shootout of two-point plays (the XFL of 2020 and 2023, the USFL of
    /// 2022-23, the UFL): `rounds` alternating tries from `from` yards,
    /// then round by round until one team makes it and the other doesn't.
    Shootout { rounds: u8, from: u8 },
    /// Arena football: a 15-minute period in which each team gets one
    /// possession; tied after that, the next score wins.
    Arena,
    /// AAF (2019): one round of possessions from the 10, a two-point try
    /// after a touchdown, then a tie in the regular season.
    Aaf,
    /// XFL 2001: each team gets four downs from the 20 and only a
    /// touchdown counts.
    Xfl2001,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rules {
    pub level: Level,
    /// Seconds per quarter.
    pub quarter: f64,
    /// Distance of each hash from the middle of the field, yards.
    pub hash: f64,
    /// Feet a receiver must get down in bounds.
    pub feet_inbounds: u8,
    /// Kickoff spot (own yard line), touchback spot for kickoffs, and whether
    /// the NFL's 2025 kickoff format applies (landing zone, setup lines).
    pub kickoff_from: f64,
    pub kickoff_touchback: f64,
    pub nfl_kickoff: bool,
    /// Touchback spot on punts and other kicks.
    pub touchback: f64,
    /// Line of scrimmage for an extra point kick and a two-point try (yards from goal).
    pub xp_from: f64,
    pub two_point_from: f64,
    pub two_minute_warning: bool,
    /// The clock stops briefly to set the chains after a first down: always in
    /// high school, in the last two minutes of each half in college.
    pub first_down_stop: FirstDownStop,
    pub overtime: Overtime,
    /// Running clock in blowouts (high school): lead and earliest quarter.
    pub mercy: Option<(i32, u8)>,
    /// Timeouts per half.
    pub timeouts: u8,
    /// Play clock, seconds.
    pub play_clock: f64,
    /// A ball carrier who goes to the ground untouched is down (college and
    /// high school). In the NFL he can get up and keep running.
    pub down_on_fall: bool,
    /// The other leagues' rule books (X7, `books.rs`): the field, players a
    /// side, downs, kicking and scoring. The American levels keep the
    /// default, which changes nothing.
    #[serde(default)]
    pub code: super::books::Code,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FirstDownStop {
    Never,
    LastTwoMinutes,
    Always,
}

impl Rules {
    pub fn for_level(level: Level) -> Rules {
        match level {
            Level::NFL => Rules {
                level,
                quarter: 900.0,
                hash: 3.083,
                feet_inbounds: 2,
                kickoff_from: 35.0,
                kickoff_touchback: 35.0,
                nfl_kickoff: true,
                touchback: 20.0,
                xp_from: 15.0,
                two_point_from: 2.0,
                two_minute_warning: true,
                first_down_stop: FirstDownStop::Never,
                overtime: Overtime::NflPeriod,
                mercy: None,
                timeouts: 3,
                play_clock: 40.0,
                down_on_fall: false,
                code: Default::default(),
            },
            Level::HS => Rules {
                level,
                quarter: 720.0,
                hash: 8.889,
                feet_inbounds: 1,
                kickoff_from: 40.0,
                kickoff_touchback: 20.0,
                nfl_kickoff: false,
                touchback: 20.0,
                xp_from: 3.0,
                two_point_from: 3.0,
                two_minute_warning: false,
                first_down_stop: FirstDownStop::Always,
                overtime: Overtime::HighSchool,
                mercy: Some((35, 3)),
                timeouts: 3,
                play_clock: 40.0,
                down_on_fall: true,
                code: Default::default(),
            },
            // FBS, FCS, D2 and below play NCAA rules.
            _ => Rules {
                level,
                quarter: 900.0,
                hash: 6.667,
                feet_inbounds: 1,
                kickoff_from: 35.0,
                kickoff_touchback: 25.0,
                nfl_kickoff: false,
                touchback: 20.0,
                xp_from: 3.0,
                two_point_from: 3.0,
                two_minute_warning: false,
                first_down_stop: FirstDownStop::LastTwoMinutes,
                overtime: Overtime::College,
                mercy: None,
                timeouts: 3,
                play_clock: 40.0,
                down_on_fall: true,
                code: Default::default(),
            },
        }
    }

    /// States that play high school football under NCAA rules (Texas,
    /// Massachusetts): college hashes and clock, still high school length.
    pub fn high_school_on_college_rules() -> Rules {
        Rules { hash: 6.667, ..Rules::for_level(Level::HS) }
    }
}

/// What a typical dressed player at a level has in a trait-like rating
/// (discipline, toughness, motor, composure, concentration, competitiveness,
/// physicality). The engine reads these against the level, the way the old
/// hidden 1-20 traits were the same at every level: officials call holding to
/// the level's standard, so a college line and an NFL line draw about as many
/// flags (real: about 6 penalties a team game at both). Measured on the
/// synthetic rosters (`level_strength`, with the NFL's world-like position
/// offsets: at 64 the NFL drew 4.5 penalties a team game); proposed, not
/// confirmed.
pub fn trait_norm(level: Level, a: Attr) -> f64 {
    let base = match level {
        Level::NFL => 67.0,
        Level::FBS => 48.0,
        Level::FCS => 42.0,
        Level::SmallCollege => 37.0,
        Level::HS => 29.0,
    };
    let offset = match a {
        Attr::Toughness => 3.0,
        Attr::Motor => 1.5,
        Attr::Composure => 1.0,
        Attr::Concentration => 4.5,
        Attr::Competitiveness => 5.5,
        Attr::Physicality => -0.5,
        _ => 0.0,
    };
    base + offset
}

/// Rating points a step on the 1-20 trait scale: the spread of these ratings
/// within a level (sd about 9) over the old traits' spread (sd 3.5).
pub const TRAIT_STEP: f64 = 2.6;
