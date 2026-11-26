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
    /// College before 2021 (update 3, F26): possessions from the 25, two-
    /// point tries required from overtime `two_from`, a two-point shootout
    /// from `shootout_from` (1996-2018: from the 3rd, none; 2019-20: from
    /// the 3rd, the 5th).
    CollegeEra { two_from: u8, shootout_from: u8 },
    /// The NFL's modified sudden death (2012-2024 regular season, 2010-21
    /// playoffs): a touchdown on the first possession wins, a field goal
    /// gives the other side a possession, then the next score wins.
    /// Periods of `minutes` (15 to 2016, then 10), 15 in the playoffs.
    NflModified { minutes: u8 },
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
    /// A fair catch of a kickoff inside the 25 is a touchback (minor bug
    /// B47): college from 2018, the NFL in 2023 only.
    #[serde(default)]
    pub fair_catch_touchback: bool,
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
    /// The targeting rule of the level and season (coach fire).
    #[serde(default)]
    pub targeting: Targeting,
    /// The concussion rules of the level and season (engine gaps item 3).
    #[serde(default)]
    pub concussion: Concussion,
    /// Whose foul rules the game is played under (engine gaps lane EG5,
    /// `fouls.rs`): `None` is the level's own; a high school in a state
    /// that plays college rules takes the NCAA's (`Rules::high_school`).
    #[serde(default)]
    pub fouls: Option<super::fouls::Fouls>,
    /// The season the rule book is read in (0: today's).
    #[serde(default)]
    pub year: i32,
    /// The book's fumble rules where they are not the level's own (engine
    /// gaps item 4): a state on college rules, the CFL, a high school
    /// season before 2025. None reads as the level's (`fumble_rule`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fumbles: Option<FumbleRule>,
    /// The book's rules at 0:00 where they are not the level's today
    /// (engine gaps item 22): a college season to 2022 or before 2011.
    /// None reads as the level's (`period_end`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period_end: Option<PeriodEnd>,
}

/// A foul on a period's last timed down and the 10-second runoff (engine
/// gaps item 22). The NFL: the defense's accepted foul (a double foul has
/// one) gives an untimed down at a half's end, the offense's don't; in a
/// half's last minute the offense's foul that stops a running clock costs
/// it 10 seconds, the defense's choice. College: either team's accepted
/// live-ball foul extends the period (the defense turns down the extension
/// on the offense's loss-of-down foul), at every quarter's end to 2022 and
/// a half's from 2023; the runoff from 2011, for either team's foul, the
/// other team's choice. High school (NFHS): either team's at every
/// quarter's end, except a loss-of-down or unsportsmanlike foul; no
/// runoff. A runoff is saved by the fouling team's timeout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PeriodEnd {
    /// The offense's fouls extend a period too, not only the defense's.
    pub either: bool,
    /// At every quarter's end, not only a half's.
    pub every_quarter: bool,
    /// The 10-second runoff: None, the offense's fouls only, or either's.
    pub runoff: u8,
    /// Unsportsmanlike fouls never extend it (NFHS).
    pub no_unsporting: bool,
}

impl PeriodEnd {
    pub const NFL: PeriodEnd = PeriodEnd { either: false, every_quarter: false, runoff: 1, no_unsporting: false };
    pub const NCAA: PeriodEnd = PeriodEnd { either: true, every_quarter: false, runoff: 2, no_unsporting: false };
    pub const NFHS: PeriodEnd = PeriodEnd { either: true, every_quarter: true, runoff: 0, no_unsporting: true };

    /// Whether this accepted foul gives an untimed down when the clock runs
    /// out on its play: `defense` the fouling team was on defense, `half`
    /// the period's end is a half's (or the game's).
    pub fn extends(&self, kind: super::sim::FoulKind, defense: bool, half: bool) -> bool {
        use super::sim::FoulKind;
        (half || self.every_quarter)
            && (defense || self.either && kind != FoulKind::IntentionalGrounding)
            && !(self.no_unsporting && kind == FoulKind::UnsportsmanlikeConduct)
    }
}

/// The concussion rules of a level and season (engine gaps item 3). The
/// NFL barred a same-day return for any player with concussion symptoms in
/// December 2009, put athletic trainers in the booth as spotters in 2011
/// and independent neurotrauma consultants on the sidelines in 2013. The
/// NCAA required that a student diagnosed with a concussion not return the
/// same day from 2010; the NFHS rule from 2010-11 removes a player showing
/// signs, and by 2014 every state's law kept a player removed for a
/// suspected concussion out for the day (Arizona and South Carolina let a
/// cleared one back). Before those years the doctors could send him back
/// in: in the NFL in 1996-2001 15.2% of concussed players went straight
/// back and 34.3% rested and went back in the same game (Pellman et al.
/// 2005).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Concussion {
    /// A diagnosed concussion sits the rest of the day.
    pub no_same_day: bool,
    /// The share of concussions the sideline catches: spotters in the NFL
    /// (0.92 from 2011), trainers in college (0.8), and some high schools
    /// have nobody (0.6). Before the rules about 0.6 of that: reported NFL
    /// concussions were 0.41 a game in 1996-2001 (Pellman et al. 2003)
    /// against 0.45-0.74 in 2012-17. Proposed, not confirmed: 0.85 in the
    /// NFL's 2009-10 and every size before the rules.
    pub caught: f64,
    /// Men pulled for a check on his signs who turn out clear, for each
    /// concussion caught. The NFL ran 538 in-game concussion evaluations in
    /// 2025 for 168 concussions in games, about 3.2 a concussion; 3.4 clear
    /// for each one caught gives that from the spotters on, as a
    /// concussion nobody caught or one from the trench draws no check.
    /// Proposed, not confirmed: 1.0 in 2009-10, 1.5 in college and 0.75 in
    /// high school (trainers at fewer games), none before the rules.
    pub checks: f64,
    /// A man pulled for signs sits the rest of the game even when he
    /// checks out (high school, the state laws).
    pub removal_sits: bool,
    /// A concussion's time out against today's protocol: NFL players lost
    /// a mean 1.9 days a concussion in 1996-2001 and 4.7 in 2002-07 (Casson
    /// et al. 2010), against about nine days today. Proposed, not
    /// confirmed: 0.2 before 2002 and 0.5 until the level's rule, at every
    /// level.
    pub length: f64,
}

impl Default for Concussion {
    fn default() -> Concussion {
        Concussion::for_year(Level::NFL, 0)
    }
}

impl Concussion {
    /// A level's concussion rules in a season; year 0 reads as today's.
    pub fn for_year(level: Level, year: i32) -> Concussion {
        let year = if year == 0 { 9999 } else { year };
        let (rule_from, checks, caught) = match level {
            Level::NFL if year >= 2011 => (2009, 3.4, 0.92),
            Level::NFL => (2009, 1.0, 0.85),
            Level::HS => (2010, 0.75, 0.6),
            _ => (2010, 1.5, 0.8),
        };
        if year >= rule_from {
            Concussion { no_same_day: true, caught, checks, removal_sits: level == Level::HS, length: 1.0 }
        } else {
            let caught = 0.6 * if level == Level::NFL { 0.92 } else { caught };
            let length = if year < 2002 { 0.2 } else { 0.5 };
            Concussion { no_same_day: false, caught, checks: 0.0, removal_sits: false, length }
        }
    }

    /// Before the rules, the share of caught concussions that went back in
    /// the same game, and of those the share that went straight back (the
    /// NFL in 1996-2001: 15.2% straight back and 34.3% after a rest,
    /// Pellman et al. 2005).
    pub const BACK_BEFORE: (f64, f64) = (0.495, 0.31);
}

/// Who may advance a fumble and where one out of bounds is spotted (engine
/// gaps item 4). The defense may always advance a fumble at every level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FumbleRule {
    /// On fourth down and on a try only the fumbler may advance his own
    /// team's fumble: a teammate's recovery is dead, back at the spot of
    /// the fumble if he got it ahead of it (the NFL since 1979, the NCAA).
    pub fourth_down: bool,
    /// The same after the two-minute warning of either half (the NFL).
    pub late: bool,
    /// A fumble that goes forward and out of bounds between the goal lines
    /// comes back to the spot of the fumble (the NFL, the NCAA, the NFHS
    /// from 2025); a muffed snap is spotted where it went out.
    pub forward_back: bool,
    /// A ball out of bounds belongs to the team that touched it last in
    /// the field of play, not the team that last had it (the CFL).
    pub last_touch: bool,
}

impl FumbleRule {
    pub const NFL: FumbleRule = FumbleRule { fourth_down: true, late: true, forward_back: true, last_touch: false };
    pub const NCAA: FumbleRule = FumbleRule { fourth_down: true, late: false, forward_back: true, last_touch: false };
    /// The NFHS: anyone may advance a fumble (`forward_back` from 2025).
    pub const NFHS: FumbleRule = FumbleRule { fourth_down: false, late: false, forward_back: true, last_touch: false };
    /// The CFL: anyone may advance a fumble; out of bounds it is the team's
    /// that touched it last.
    pub const CFL: FumbleRule = FumbleRule { fourth_down: false, late: false, forward_back: false, last_touch: true };
}

/// The targeting rule (coach fire, item 4): a forcible hit to the head or
/// neck of a defenseless man, or one led with the crown of the helmet.
/// College: a 15-yard personal foul from 2008, with an ejection from 2013
/// (in the second half he also sits the next game's first half). High
/// school (NFHS): a 15-yard personal foul from 2013, disqualification only
/// when flagrant. The NFL has no targeting foul: the same hits are
/// unnecessary roughness, ejected when flagrant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Targeting {
    /// No targeting foul: such a hit is unnecessary roughness.
    #[default]
    None,
    /// A 15-yard targeting foul; disqualified only when flagrant.
    Foul,
    /// A 15-yard targeting foul and an ejection.
    Eject,
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
                fair_catch_touchback: false,
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
                targeting: Targeting::None,
                concussion: Concussion::for_year(level, 0),
                fouls: None,
                year: 0,
                fumbles: None,
                period_end: None,
            },
            Level::HS => Rules {
                level,
                quarter: 720.0,
                hash: 8.889,
                feet_inbounds: 1,
                kickoff_from: 40.0,
                kickoff_touchback: 20.0,
                nfl_kickoff: false,
                fair_catch_touchback: false,
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
                // NFHS goalposts: 23 feet 4 inches between the uprights, the
                // NCAA's and the NFL's 18 feet 6 inches (engine gaps item 11).
                code: super::books::Code {
                    field: super::geom::Field { posts: 3.889, ..super::geom::Field::AMERICAN },
                    ..Default::default()
                },
                targeting: Targeting::Foul,
                concussion: Concussion::for_year(level, 0),
                fouls: None,
                year: 0,
                fumbles: None,
                period_end: None,
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
                fair_catch_touchback: true,
                touchback: 20.0,
                xp_from: 3.0,
                two_point_from: 3.0,
                // The two-minute timeout in each half since 2024 (NCAA,
                // approved 19 April 2024).
                two_minute_warning: true,
                first_down_stop: FirstDownStop::LastTwoMinutes,
                overtime: Overtime::College,
                mercy: None,
                timeouts: 3,
                play_clock: 40.0,
                down_on_fall: true,
                code: Default::default(),
                targeting: Targeting::Eject,
                concussion: Concussion::for_year(level, 0),
                fouls: None,
                year: 0,
                fumbles: None,
                period_end: None,
            },
        }
    }

    /// A level's book in a season (update 3, F25): the kickoff, the extra
    /// point and overtime as they were that year (real); later seasons keep
    /// today's book. Year 0 reads as today's.
    pub fn for_year(level: Level, year: i32, playoff: bool) -> Rules {
        let mut r = Rules::for_level(level);
        if year == 0 {
            return r;
        }
        r.concussion = Concussion::for_year(level, year);
        r.year = year;
        match level {
            Level::NFL => {
                // Kickoffs from the 30 to 2010, then the 35; the touchback
                // at the 20 to 2015, the 25 to 2023, the 30 in 2024 and the
                // 35 from 2025, the new kickoff format from 2024.
                r.kickoff_from = if year <= 2010 { 30.0 } else { 35.0 };
                r.kickoff_touchback = match year {
                    ..=2015 => 20.0,
                    2016..=2023 => 25.0,
                    2024 => 30.0,
                    _ => 35.0,
                };
                r.nfl_kickoff = year >= 2024;
                r.fair_catch_touchback = year == 2023;
                // The extra point from the 2 to 2014, the 15 from 2015.
                r.xp_from = if year <= 2014 { 2.0 } else { 15.0 };
                r.overtime = match (playoff, year) {
                    (false, ..=2011) | (true, ..=2009) => Overtime::SuddenDeath,
                    (false, 2012..=2016) | (true, 2010..=2021) => Overtime::NflModified { minutes: 15 },
                    (false, 2017..=2024) => Overtime::NflModified { minutes: 10 },
                    _ => Overtime::NflPeriod,
                };
            }
            Level::HS => {
                // NFHS targeting from 2013.
                if year < 2013 {
                    r.targeting = Targeting::None;
                }
                // A forward fumble out of bounds came back to the spot of
                // the fumble only from 2025 (NFHS rule 4-3-1 exception).
                if year < 2025 {
                    r.fumbles = Some(FumbleRule { forward_back: false, ..FumbleRule::NFHS });
                }
            }
            _ => {
                // Kickoffs from the 35, the 30 in 2007-2011, the 35 again
                // from 2012 with the touchback at the 25 (the 20 before);
                // a fair catch inside the 25 a touchback from 2018 (minor
                // bugs B46, B47).
                r.kickoff_from = match year {
                    2007..=2011 => 30.0,
                    _ => 35.0,
                };
                r.kickoff_touchback = if year <= 2011 { 20.0 } else { 25.0 };
                r.fair_catch_touchback = year >= 2018;
                // The two-minute timeout from 2024; the clock stopped on
                // every first down to 2022, only in each half's last two
                // minutes from 2023 (NCAA rule changes, April 2024 and 2023).
                r.two_minute_warning = year >= 2024;
                r.first_down_stop = if year <= 2022 { FirstDownStop::Always } else { FirstDownStop::LastTwoMinutes };
                // Targeting a 15-yard foul from 2008, with an ejection from
                // 2013 (NCAA rule changes, 2008 and 2013).
                r.targeting = match year {
                    ..=2007 => Targeting::None,
                    2008..=2012 => Targeting::Foul,
                    _ => Targeting::Eject,
                };
                // A foul on a quarter's last play extended every quarter to
                // 2022, only a half's from 2023; the 10-second runoff from
                // 2011 (NCAA rule changes, 2011 and 2023).
                if year <= 2022 {
                    r.period_end = Some(PeriodEnd {
                        every_quarter: true,
                        runoff: if year >= 2011 { 2 } else { 0 },
                        ..PeriodEnd::NCAA
                    });
                }
                r.overtime = match year {
                    ..=2018 => Overtime::CollegeEra { two_from: 3, shootout_from: 99 },
                    2019..=2020 => Overtime::CollegeEra { two_from: 3, shootout_from: 5 },
                    _ => Overtime::College,
                };
            }
        }
        r
    }

    /// College overtime's rounds: two-point tries required from the first,
    /// a two-point shootout from the second (today: the 2nd and the 3rd).
    pub fn college_ot(&self) -> Option<(u8, u8)> {
        match self.overtime {
            Overtime::College => Some((2, 3)),
            Overtime::CollegeEra { two_from, shootout_from } => Some((two_from, shootout_from)),
            _ => None,
        }
    }

    /// A high school game in a state in a season (engine gaps lane EG5,
    /// item 23). Texas plays NCAA rules (with the UIL's own changes) in
    /// every season and Massachusetts did through 2018 (NFHS from 2019):
    /// their games take the NCAA's foul, kick and fumble rules (`fouls.rs`,
    /// engine gaps item 4) and its rules at 0:00 (item 22). Both
    /// keep college hash marks, the fields they were laid out for; every
    /// other state plays the NFHS book. Year 0 reads as today's.
    pub fn high_school(state: &str, year: i32) -> Rules {
        let mut r = Rules::for_year(Level::HS, year, false);
        if matches!(state, "TX" | "MA") {
            r.hash = 6.667;
        }
        let college = match state {
            "TX" => true,
            "MA" => year != 0 && year <= 2018,
            _ => false,
        };
        if college {
            r.fouls = Some(super::fouls::Fouls::Ncaa);
            r.fumbles = Some(FumbleRule::NCAA);
            // The NCAA's rules at 0:00 of that season (year 0 reads as
            // today's).
            r.period_end = Some(Rules::for_year(Level::FBS, year, false).period_end());
        }
        r
    }

    /// Whose foul rules the game is played under: the NFL's (the other
    /// leagues too, `books.rs`), the NCAA's, or the NFHS's.
    pub fn foul_book(&self) -> super::fouls::Fouls {
        self.fouls.unwrap_or(super::fouls::Fouls::of_level(self.level))
    }

    /// The fumble rules in play (engine gaps item 4): the book's own where
    /// it has them, otherwise the level's.
    /// The rules at 0:00 in play (engine gaps item 22): the book's own
    /// where it has them, otherwise the level's today.
    pub fn period_end(&self) -> PeriodEnd {
        self.period_end.unwrap_or(match self.level {
            Level::NFL => PeriodEnd::NFL,
            Level::HS => PeriodEnd::NFHS,
            _ => PeriodEnd::NCAA,
        })
    }

    pub fn fumble_rule(&self) -> FumbleRule {
        self.fumbles.unwrap_or(match self.level {
            Level::NFL => FumbleRule::NFL,
            Level::HS => FumbleRule::NFHS,
            _ => FumbleRule::NCAA,
        })
    }
}

/// Whether the level's scorers have a line for a helper on a tackle
/// (realism audit item 3, the front seven's share). The NFL's book writes a
/// shared tackle two ways: "(A; B)", an assist each, or "(A, B)", a tackle
/// for A and an assist for B (38.5% of shared tackles on runs and 38.9% on
/// catches, nflverse 2023-24). On a loss it is almost always the second
/// (394 of the 406 shared run losses of 2023), and A has the whole tackle
/// for loss. The NCAA's and the high school books have one form: a stop is
/// one man's solo or two men's assists, and a shared tackle for loss is
/// half each. So the man a runner falls into in his block
/// (`Sim::pile_credit`) is credited where the book can name him without
/// taking the tackle from the man who made it. College and high school
/// keep their count as it is: their tackle fights run longer than the
/// pros' (the fight reads a tackler against a fixed mark, `GAPS.md`),
/// which gives FBS the real share of shared tackles (30.5%, real 30.1%) by
/// another road. Proposed, not confirmed.
pub fn helper_form(level: Level) -> bool {
    level == Level::NFL
}

/// The typical head coach's game management (1-20) at each level, as the
/// world's coaches come out of hiring (a 15-season world at a quarter's
/// scale, seed 2026: NFL 15.1, FBS 14.2, FCS 12.8, D2 to JUCO 12.1, high
/// school 9.2). The real rates a coach's calls are set to (NFL toss
/// winners deferring 82%, challenges won 38%, fourth-down tries) are what
/// the level's own coaches do, so a coach is read against this: the NFL's
/// were read against 10 and, at about 15, went to the replay booth's clamp
/// (minor bug 140). Proposed, not confirmed.
pub fn coach_norm(level: Level) -> f64 {
    match level {
        Level::NFL => 15.0,
        Level::FBS => 14.0,
        Level::FCS => 13.0,
        Level::SmallCollege => 12.0,
        Level::HS => 9.0,
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
