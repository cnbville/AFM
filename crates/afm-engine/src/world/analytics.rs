//! Player analytics (update 3, lane L20, items 7 and 8): the numbers each
//! position group is judged by on the Data section's analytics page, worked
//! out from a season's counts (`records::Stat`). Two families: the charted
//! one (the college stat book's and the NFL's advanced counts: EPA, expected
//! completions, pressures, separation, wins in the trenches, coverage) and
//! the box score one (what any box score gives: high school, and the levels
//! your seat doesn't chart).
//!
//! The card's rates need volume to mean anything; each group's qualifier is
//! the group's own volume count (dropbacks, carries, routes, pass-block
//! snaps, pass-rush snaps, defensive snaps, coverage snaps, kicks, punts).

use super::records::Stat;
use crate::positions::Position;

/// The position groups the page's chips pick (board 46).
pub const GROUPS: [&str; 12] = ["QB", "RB", "WR", "TE", "OL", "DL", "EDGE", "LB", "CB", "S", "K", "P"];

/// A position's group, by `GROUPS` index (the long snapper has none).
pub fn group_of(pos: Position) -> Option<usize> {
    use Position as P;
    Some(match pos {
        P::QB => 0,
        P::RB => 1,
        P::WR => 2,
        P::TE => 3,
        P::OT | P::IOL => 4,
        P::DT => 5,
        P::EDGE => 6,
        P::LB => 7,
        P::CB => 8,
        P::S => 9,
        P::K => 10,
        P::P => 11,
        P::LS => return None,
    })
}

/// The most numbers a card carries.
pub const CARD_MAX: usize = 12;

/// One number on a card or a column in the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cm {
    // Passing.
    EpaDb,
    Cpoe,
    Adot,
    Ypa,
    Rating,
    PressSack,
    Btt,
    Twp,
    TdPct,
    IntPct,
    SackRate,
    PressRate,
    CmpPct,
    PassYdsG,
    PassTd,
    Int,
    // Running.
    Ypc,
    YacoC,
    Mtf,
    Explosive,
    Success,
    FumRate,
    RushYdsG,
    RushTd,
    Carries,
    // Catching.
    Yprr,
    Sep,
    CatchRate,
    Drop,
    YacR,
    Ypr,
    TgtRoute,
    RecG,
    RecYdsG,
    RecTd,
    // Blocking.
    PressAllowed,
    SackAllowed,
    RbWin,
    // Rushing the passer and stopping the run.
    PrWin,
    Pressure,
    SackPerRush,
    Stop,
    Tfl,
    Miss,
    TklG,
    Sacks,
    // Coverage.
    YdsCov,
    TgtCov,
    BallsPlayed,
    Ints,
    Pbus,
    // Kicking.
    FgPct,
    Fg50,
    FgLong,
    FgMade,
    XpMade,
    NetPunt,
    Punts,
    Grade,
}

/// How a number is written: whole, one decimal, two decimals, a percent
/// with one decimal, or signed with two decimals (EPA).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fmt {
    Whole,
    One,
    Two,
    Pct,
    Signed,
}

impl Cm {
    /// Short label, long label, how it is written, whether more is better.
    pub fn about(self) -> (&'static str, &'static str, Fmt, bool) {
        use Cm::*;
        use Fmt::*;
        match self {
            EpaDb => ("EPA/db", "EPA a dropback", Signed, true),
            Cpoe => ("CPOE", "Completion % over expected", Pct, true),
            Adot => ("aDOT", "Average depth of target", One, true),
            Ypa => ("Y/A", "Yards an attempt", One, true),
            Rating => ("Rating", "Passer rating", One, true),
            PressSack => ("P2S", "Pressure to sack rate", Pct, false),
            Btt => ("BTT%", "Big-time throw rate", Pct, true),
            Twp => ("TWP%", "Turnover-worthy play rate", Pct, false),
            TdPct => ("TD%", "Touchdown rate", Pct, true),
            IntPct => ("INT%", "Interception rate", Pct, false),
            SackRate => ("Sack%", "Sacked on dropbacks", Pct, false),
            PressRate => ("Pressured", "Pressured on dropbacks", Pct, false),
            CmpPct => ("Cmp%", "Completion percentage", Pct, true),
            PassYdsG => ("Yds/G", "Passing yards a game", One, true),
            PassTd => ("TD", "Touchdown passes", Whole, true),
            Int => ("INT", "Interceptions thrown", Whole, false),
            Ypc => ("Y/C", "Yards a carry", One, true),
            YacoC => ("YACo/C", "Yards after contact a carry", Two, true),
            Mtf => ("MTF/T", "Missed tackles forced a touch", Two, true),
            Explosive => ("15+%", "Runs of 15 yards or more", Pct, true),
            Success => ("Success", "Run success rate", Pct, true),
            FumRate => ("Fum/T", "Fumbles lost a 100 touches", Two, false),
            RushYdsG => ("Yds/G", "Rushing yards a game", One, true),
            RushTd => ("TD", "Rushing touchdowns", Whole, true),
            Carries => ("Car", "Carries", Whole, true),
            Yprr => ("YPRR", "Yards a route run", Two, true),
            Sep => ("Sep", "Separation at the catch point", One, true),
            CatchRate => ("Catch%", "Catch rate", Pct, true),
            Drop => ("Drop%", "Drop rate", Pct, false),
            YacR => ("YAC/R", "Yards after the catch a catch", One, true),
            Ypr => ("Y/R", "Yards a catch", One, true),
            TgtRoute => ("Tgt/Rt", "Targets a route run", Pct, true),
            RecG => ("Rec/G", "Catches a game", One, true),
            RecYdsG => ("Yds/G", "Receiving yards a game", One, true),
            RecTd => ("TD", "Touchdown catches", Whole, true),
            PressAllowed => ("Press%", "Pressures allowed a pass block", Pct, false),
            SackAllowed => ("Sack%", "Sacks allowed a pass block", Pct, false),
            RbWin => ("RBW%", "Run block win rate", Pct, true),
            PrWin => ("PRW%", "Pass rush win rate", Pct, true),
            Pressure => ("Press%", "Pressure rate", Pct, true),
            SackPerRush => ("Sack%", "Sacks a pass rush", Pct, true),
            Stop => ("Stop%", "Run stops a snap", Pct, true),
            Tfl => ("TFL", "Tackles for loss", One, true),
            Miss => ("Miss%", "Missed tackle rate", Pct, false),
            TklG => ("Tkl/G", "Tackles a game", One, true),
            Sacks => ("Sacks", "Sacks", One, true),
            YdsCov => ("Yds/Cov", "Yards allowed a coverage snap", Two, false),
            TgtCov => ("Tgt/Cov", "Targets a coverage snap", Pct, false),
            BallsPlayed => ("PD%", "Passes defended a target", Pct, true),
            Ints => ("INT", "Interceptions", Whole, true),
            Pbus => ("PBU", "Passes broken up", Whole, true),
            FgPct => ("FG%", "Field goal percentage", Pct, true),
            Fg50 => ("50+", "Field goals of 50 or more", Whole, true),
            FgLong => ("Long", "Longest field goal", Whole, true),
            FgMade => ("FG", "Field goals", Whole, true),
            XpMade => ("XP", "Extra points", Whole, true),
            NetPunt => ("Net", "Net punting average", One, true),
            Punts => ("Punts", "Punts", Whole, true),
            Grade => ("Grade", "Grade from the charting", One, true),
        }
    }

    /// Its value from a season's counts, or none when the season gives it
    /// nothing to divide by. `get` reads a count; `games` the games played.
    pub fn value(self, get: &dyn Fn(Stat) -> f64, games: f64) -> Option<f64> {
        use Cm::*;
        use Stat as S;
        let r = |n: f64, d: f64| if d > 0.0 { Some(n / d) } else { None };
        let pct = |n: f64, d: f64| r(n, d).map(|x| x * 100.0);
        let g = |k: Stat| get(k);
        let per_game = |n: f64| r(n, games);
        match self {
            EpaDb => r(g(S::PassEpa), g(S::Dropbacks)),
            Cpoe => pct(g(S::Cmp) - g(S::XCmp), g(S::Att)),
            Adot => r(g(S::AirYds), g(S::Att)),
            Ypa => r(g(S::PassYds), g(S::Att)),
            Rating => passer_rating(g(S::Att), g(S::Cmp), g(S::PassYds), g(S::PassTd), g(S::Int)),
            PressSack => pct(g(S::Sacked), g(S::Pressured)),
            Btt => pct(g(S::BigTime), g(S::Dropbacks)),
            Twp => pct(g(S::TurnoverWorthy), g(S::Dropbacks)),
            TdPct => pct(g(S::PassTd), g(S::Att)),
            IntPct => pct(g(S::Int), g(S::Att)),
            SackRate => pct(g(S::Sacked), g(S::Dropbacks)),
            PressRate => pct(g(S::Pressured), g(S::Dropbacks)),
            CmpPct => pct(g(S::Cmp), g(S::Att)),
            PassYdsG => per_game(g(S::PassYds)),
            PassTd => Some(g(S::PassTd)),
            Int => Some(g(S::Int)),
            Ypc => r(g(S::RushYds), g(S::Rush)),
            YacoC => r(g(S::Yaco), g(S::Rush)),
            Mtf => r(g(S::ForcedMissed), g(S::Rush) + g(S::Rec)),
            Explosive => pct(g(S::ExplosiveRuns), g(S::Rush)),
            Success => pct(g(S::RushSuccess), g(S::Rush)),
            FumRate => r(100.0 * g(S::FumblesLost), g(S::Rush) + g(S::Rec)),
            RushYdsG => per_game(g(S::RushYds)),
            RushTd => Some(g(S::RushTd)),
            Carries => Some(g(S::Rush)),
            Yprr => r(g(S::RecYds), g(S::Routes)),
            Sep => r(g(S::SepSum), g(S::Targets)),
            CatchRate => pct(g(S::Rec), g(S::Targets)),
            Drop => pct(g(S::Drops), g(S::Targets)),
            YacR => r(g(S::Yac), g(S::Rec)),
            Ypr => r(g(S::RecYds), g(S::Rec)),
            TgtRoute => pct(g(S::Targets), g(S::Routes)),
            RecG => per_game(g(S::Rec)),
            RecYdsG => per_game(g(S::RecYds)),
            RecTd => Some(g(S::RecTd)),
            PressAllowed => pct(g(S::PressuresAllowed), g(S::PbSnaps)),
            SackAllowed => pct(g(S::SacksAllowed), g(S::PbSnaps)),
            RbWin => pct(g(S::RbWins), g(S::RbSnaps)),
            PrWin => pct(g(S::RushWins), g(S::RushSnaps)),
            Pressure => pct(g(S::Pressures), g(S::RushSnaps)),
            SackPerRush => pct(g(S::Sacks), g(S::RushSnaps)),
            Stop => pct(g(S::Stops), g(S::SnapsDef)),
            Tfl => Some(g(S::Tfl)),
            Miss => pct(g(S::Missed), g(S::Tackles) + g(S::Missed)),
            TklG => per_game(g(S::Tackles)),
            Sacks => Some(g(S::Sacks)),
            YdsCov => r(g(S::YdsAllowed), g(S::CovSnaps)),
            TgtCov => pct(g(S::TgtAllowed), g(S::CovSnaps)),
            BallsPlayed => pct(g(S::Ints) + g(S::Pbus), g(S::TgtAllowed)),
            Ints => Some(g(S::Ints)),
            Pbus => Some(g(S::Pbus)),
            FgPct => pct(g(S::FgMade), g(S::FgAtt)),
            Fg50 => Some(g(S::Fg50Made)),
            FgLong => (g(S::FgLong) > 0.0).then(|| g(S::FgLong)),
            FgMade => Some(g(S::FgMade)),
            XpMade => Some(g(S::XpMade)),
            NetPunt => r(g(S::PuntNet), g(S::Punts)),
            Punts => Some(g(S::Punts)),
            Grade => r(g(S::GradeSum), g(S::Graded)),
        }
    }
}

/// The NFL's passer rating (also the high school one): four parts, each
/// held between 0 and 2.375.
pub fn passer_rating(att: f64, cmp: f64, yds: f64, td: f64, int: f64) -> Option<f64> {
    if att <= 0.0 {
        return None;
    }
    let c = |x: f64| x.clamp(0.0, 2.375);
    let p1 = c((cmp / att - 0.3) * 5.0);
    let p2 = c((yds / att - 3.0) * 0.25);
    let p3 = c(td / att * 20.0);
    let p4 = c(2.375 - int / att * 25.0);
    Some((p1 + p2 + p3 + p4) / 6.0 * 100.0)
}

/// The NCAA's passing efficiency: (8.4 yards + 330 touchdowns + 100
/// completions - 200 interceptions) a pass attempt, with no caps.
pub fn passing_efficiency(att: f64, cmp: f64, yds: f64, td: f64, int: f64) -> Option<f64> {
    (att > 0.0).then(|| (8.4 * yds + 330.0 * td + 100.0 * cmp - 200.0 * int) / att)
}

/// The charted card for a group: what PFF-style charting and the league's
/// tracking give (college divisions you chart, the NFL).
pub fn charted(group: usize) -> &'static [Cm] {
    use Cm::*;
    match group {
        0 => &[EpaDb, Cpoe, Adot, Ypa, Rating, Btt, Twp, PressRate, PressSack, SackRate, TdPct, Grade],
        1 => &[Ypc, YacoC, Mtf, Explosive, Success, FumRate, RushYdsG, Yprr, CatchRate, YacR, RushTd, Grade],
        2 | 3 => &[Yprr, Sep, TgtRoute, CatchRate, Drop, YacR, Ypr, RecYdsG, RecG, RecTd, Grade],
        4 => &[PressAllowed, SackAllowed, RbWin, Grade],
        5 | 6 => &[PrWin, Pressure, SackPerRush, Sacks, Stop, Tfl, Miss, TklG, Grade],
        7 => &[Stop, Miss, TklG, Tfl, Pressure, Sacks, YdsCov, TgtCov, BallsPlayed, Grade],
        8 | 9 => &[YdsCov, TgtCov, BallsPlayed, Ints, Pbus, Miss, Stop, TklG, Grade],
        10 => &[FgPct, FgMade, Fg50, FgLong, XpMade],
        _ => &[NetPunt, Punts],
    }
}

/// The box score card for a group: what any box score gives.
pub fn boxed(group: usize) -> &'static [Cm] {
    use Cm::*;
    match group {
        0 => &[CmpPct, Ypa, Rating, PassYdsG, PassTd, Int, TdPct, IntPct],
        1 => &[Carries, Ypc, RushYdsG, RushTd, RecG, Ypr],
        2 | 3 => &[RecG, RecYdsG, Ypr, RecTd],
        4 => &[],
        5..=9 => &[TklG, Tfl, Sacks, Ints, Pbus],
        10 => &[FgPct, FgMade, XpMade],
        _ => &[],
    }
}

/// The count a group qualifies by on the charted card, with its label.
pub fn volume(group: usize) -> (Stat, &'static str) {
    match group {
        0 => (Stat::Dropbacks, "dropbacks"),
        1 => (Stat::Rush, "carries"),
        2 | 3 => (Stat::Routes, "routes"),
        4 => (Stat::PbSnaps, "pass-block snaps"),
        5 | 6 => (Stat::RushSnaps, "pass rushes"),
        7 => (Stat::SnapsDef, "defensive snaps"),
        8 | 9 => (Stat::CovSnaps, "coverage snaps"),
        10 => (Stat::FgAtt, "field goal tries"),
        _ => (Stat::Punts, "punts"),
    }
}

/// The count a group qualifies by on the box score card.
pub fn box_volume(group: usize) -> (Stat, &'static str) {
    match group {
        0 => (Stat::Att, "attempts"),
        1 => (Stat::Rush, "carries"),
        2 | 3 => (Stat::Rec, "catches"),
        10 => (Stat::FgAtt, "field goal tries"),
        11 => (Stat::Punts, "punts"),
        _ => (Stat::Tackles, "tackles"),
    }
}

/// The default qualifier for a season: about two thirds of a starter's
/// volume at the level (the page's minimum can be changed). `nfl` for the
/// NFL's 17 games, else a college or high school season's 12.
pub fn default_min(group: usize, charted: bool, nfl: bool) -> f64 {
    let m = if charted {
        [200.0, 80.0, 180.0, 150.0, 200.0, 150.0, 150.0, 300.0, 200.0, 200.0, 8.0, 20.0][group]
    } else {
        [80.0, 50.0, 15.0, 15.0, 0.0, 15.0, 15.0, 25.0, 15.0, 15.0, 5.0, 10.0][group]
    };
    if nfl { m * 1.35 } else { m }
}
