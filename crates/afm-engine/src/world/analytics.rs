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
use super::tally::depth::{self as dp, Dc};
use crate::positions::Position;

/// The position groups the page's chips pick (board 46), and the
/// returners as their own group (Data overhaul item 10): a returner is
/// anyone who returned kicks or punts, whatever his position.
pub const GROUPS: [&str; 13] = ["QB", "RB", "WR", "TE", "OL", "DL", "EDGE", "LB", "CB", "S", "K", "P", "RET"];

/// The returners' group.
pub const RET: usize = 12;

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
    // ---- Data overhaul items 4-13 (lane NB4): read from the depth counts
    // of the players tally (`tally::depth`), so seasons since the tallies
    // began. ----
    // Passing.
    DbSucc,
    Anya,
    SackYds,
    Ttt,
    Aggr,
    Cay,
    DeepRate,
    DeepCmp,
    DeepCpoe,
    DeepYpa,
    TaRate,
    PaRate,
    PaYpa,
    NoPaYpa,
    PaEpaDb,
    BlitzRate,
    BlitzEpaDb,
    BlitzYpa,
    CleanEpaDb,
    PrsEpaDb,
    CleanYpa,
    PrsYpa,
    PrsCmp,
    ScrRate,
    ScrYds,
    ScrYpc,
    // Running.
    Ybc,
    Ryoe,
    RyoeTot,
    RyoeRate,
    Stuff,
    Breakaway,
    Elusive,
    RFirst,
    REpa,
    Box8Rate,
    Box8Ypc,
    // Catching.
    RAdot,
    TgtShare,
    AirShare,
    Separation,
    ContRate,
    Yacoe,
    FdRoute,
    EpaTgt,
    TgtRating,
    SlotRate,
    WideRate,
    InlineRate,
    BackRate,
    // Blocking.
    Pbe,
    PbWr,
    PrsAllowedN,
    SacksAllowedN,
    HitsAllowedN,
    HurriesAllowedN,
    PenN,
    BustN,
    TackleRate,
    GuardRate,
    CenterRate,
    // The pass rush and the run.
    Prp,
    PrsN,
    HitsN,
    HurriesN,
    GetOff,
    Ttp,
    Unblocked,
    SpeedShare,
    PowerShare,
    FinesseShare,
    RunStop,
    BlitzUse,
    BlitzPrs,
    // Coverage.
    CmpA,
    RatingA,
    YdsTgtA,
    SepA,
    FInc,
    TdA,
    EpaTgtA,
    YacA,
    AdotA,
    BoxRate,
    DSlotRate,
    DWideRate,
    DeepSpot,
    LineRate,
    // Kicking, punting and returns.
    Fg30s,
    Fg40s,
    Fg50Pct,
    ClutchPct,
    XpPct,
    KoTbRate,
    KoHang,
    KoRetAvg,
    Gross,
    In20Rate,
    PuntTbRate,
    PuntHang,
    FairRate,
    PuntRetAvg,
    KrAvg,
    KrLong,
    KrTd,
    PrAvg,
    PrLong,
    PrTd,
    FairCatches,
    Returns,
    RetAvg,
    RetTd,
    // Facet grades (item 11), `game::stats::FACETS` in turn.
    GrPass,
    GrRush,
    GrRecv,
    GrPb,
    GrRb,
    GrPr,
    GrRd,
    GrCov,
    GrTkl,
    GrSt,
    // By zone (item 28): completion % by depth (behind the line, 0-9,
    // 10-19, 20+) and EPA a throw by third (left, middle, right), for the
    // passer (`Q`), the target (`R`) and the man covering (`D`).
    ZqBehind,
    ZqShort,
    ZqMid,
    ZqDeep,
    ZqLeft,
    ZqMiddle,
    ZqRight,
    ZrBehind,
    ZrShort,
    ZrMid,
    ZrDeep,
    ZrLeft,
    ZrMiddle,
    ZrRight,
    ZdBehind,
    ZdShort,
    ZdMid,
    ZdDeep,
    ZdLeft,
    ZdMiddle,
    ZdRight,
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
            Explosive => ("10+%", "Runs of 10 yards or more", Pct, true),
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
            DbSucc => ("Succ%", "Success rate a dropback", Pct, true),
            Anya => ("ANY/A", "Adjusted net yards an attempt", Two, true),
            SackYds => ("Sk yds", "Yards lost to sacks", Whole, false),
            Ttt => ("TTT", "Time to throw", Two, false),
            Aggr => ("Aggr%", "Aggressiveness: throws into tight windows", Pct, true),
            Cay => ("CAY", "Completed air yards a completion", One, true),
            DeepRate => ("Deep%", "Throws 20+ yards downfield", Pct, true),
            DeepCmp => ("Deep Cmp%", "Completion % on throws 20+ yards", Pct, true),
            DeepCpoe => ("Deep CPOE", "Completion % over expected, 20+ yards", Pct, true),
            DeepYpa => ("Deep Y/A", "Yards an attempt, 20+ yards", One, true),
            TaRate => ("TA%", "Throwaway rate", Pct, false),
            PaRate => ("PA%", "Play-action share of dropbacks", Pct, true),
            PaYpa => ("PA Y/A", "Yards an attempt off play action", One, true),
            NoPaYpa => ("No PA Y/A", "Yards an attempt without play action", One, true),
            PaEpaDb => ("PA EPA", "EPA a dropback off play action", Signed, true),
            BlitzRate => ("Blitzed%", "Dropbacks against the blitz", Pct, false),
            BlitzEpaDb => ("vs blitz", "EPA a dropback against the blitz", Signed, true),
            BlitzYpa => ("Blitz Y/A", "Yards an attempt against the blitz", One, true),
            CleanEpaDb => ("Clean EPA", "EPA a dropback from a clean pocket", Signed, true),
            PrsEpaDb => ("Prs EPA", "EPA a dropback under pressure", Signed, true),
            CleanYpa => ("Clean Y/A", "Yards an attempt from a clean pocket", One, true),
            PrsYpa => ("Prs Y/A", "Yards an attempt under pressure", One, true),
            PrsCmp => ("Prs Cmp%", "Completion % under pressure", Pct, true),
            ScrRate => ("Scr%", "Scrambles a dropback", Pct, true),
            ScrYds => ("Scr yds", "Scramble yards", Whole, true),
            ScrYpc => ("Y/Scr", "Yards a scramble", One, true),
            Ybc => ("YBC/C", "Yards before contact a carry", Two, true),
            Ryoe => ("RYOE/C", "Rush yards over expected a carry", Two, true),
            RyoeTot => ("RYOE", "Rush yards over expected", Whole, true),
            RyoeRate => ("ROE%", "Carries beating the expected yards", Pct, true),
            Stuff => ("Stuff%", "Carries stopped at or behind the line", Pct, false),
            Breakaway => ("BA%", "Breakaway: share of yards from runs of 15+", Pct, true),
            Elusive => ("Elusive", "Elusive rating", One, true),
            RFirst => ("1D%", "First downs a carry", Pct, true),
            REpa => ("EPA/C", "EPA a carry", Signed, true),
            Box8Rate => ("8+ box%", "Carries against eight or more in the box", Pct, true),
            Box8Ypc => ("8+ Y/C", "Yards a carry against eight or more in the box", One, true),
            RAdot => ("aDOT", "Average depth of his targets", One, true),
            TgtShare => ("Tgt%", "Share of the team's targets", Pct, true),
            AirShare => ("Air%", "Share of the team's air yards", Pct, true),
            Separation => ("Sep/T", "Separation when targeted", Two, true),
            ContRate => ("Cont%", "Contested catch rate", Pct, true),
            Yacoe => ("YACOE", "Yards after the catch over expected a catch", Two, true),
            FdRoute => ("1D/RR", "First downs a route run", Two, true),
            EpaTgt => ("EPA/T", "EPA a target", Signed, true),
            TgtRating => ("Rtg/T", "Passer rating when targeted", One, true),
            SlotRate => ("Slot%", "Snaps in the slot", Pct, true),
            WideRate => ("Wide%", "Snaps split wide", Pct, true),
            InlineRate => ("Inline%", "Snaps in line as a tight end", Pct, true),
            BackRate => ("Bkf%", "Snaps in the backfield", Pct, true),
            Pbe => ("PBE", "Pass-blocking efficiency", One, true),
            PbWr => ("PBWR", "Pass-block win rate", Pct, true),
            PrsAllowedN => ("Prs", "Pressures allowed", Whole, false),
            SacksAllowedN => ("Sk", "Sacks allowed", Whole, false),
            HitsAllowedN => ("Hits", "Hits allowed", Whole, false),
            HurriesAllowedN => ("Hur", "Hurries allowed", Whole, false),
            PenN => ("Pen", "Penalties", Whole, false),
            BustN => ("Busts", "Blown assignments", Whole, false),
            TackleRate => ("T%", "Snaps at tackle", Pct, true),
            GuardRate => ("G%", "Snaps at guard", Pct, true),
            CenterRate => ("C%", "Snaps at center", Pct, true),
            Prp => ("PRP", "Pass-rush productivity", One, true),
            PrsN => ("Prs", "Pressures", Whole, true),
            HitsN => ("Hits", "Quarterback hits", Whole, true),
            HurriesN => ("Hur", "Hurries", Whole, true),
            GetOff => ("Get-off", "Get-off: seconds to his first contact", Two, false),
            Ttp => ("TTP", "Time to pressure", Two, false),
            Unblocked => ("Unblk%", "Pressures unblocked", Pct, false),
            SpeedShare => ("Speed%", "Rush wins with speed", Pct, true),
            PowerShare => ("Power%", "Rush wins with power", Pct, true),
            FinesseShare => ("Fin%", "Rush wins with a move", Pct, true),
            RunStop => ("RStop%", "Run stops a run snap", Pct, true),
            BlitzUse => ("Blitz%", "Blitzes a passing snap", Pct, true),
            BlitzPrs => ("Prs/Blz", "Pressure rate on his blitzes", Pct, true),
            CmpA => ("Cmp% A", "Completion % allowed", Pct, false),
            RatingA => ("Rtg A", "Passer rating allowed", One, false),
            YdsTgtA => ("Yds/T A", "Yards allowed a target", One, false),
            SepA => ("Sep A", "Separation allowed", Two, false),
            FInc => ("FInc%", "Forced incompletion rate", Pct, true),
            TdA => ("TD A", "Touchdowns allowed", Whole, false),
            EpaTgtA => ("EPA/T A", "EPA allowed a target", Signed, false),
            YacA => ("YAC A", "Yards after the catch allowed a catch", One, false),
            AdotA => ("aDOT A", "Average depth of the throws at him", One, true),
            BoxRate => ("Box%", "Snaps in the box", Pct, true),
            DSlotRate => ("Slot%", "Snaps over the slot", Pct, true),
            DWideRate => ("Wide%", "Snaps out wide", Pct, true),
            DeepSpot => ("Deep%", "Snaps deep", Pct, true),
            LineRate => ("Line%", "Snaps on the line", Pct, true),
            Fg30s => ("FG 30s", "Field goal % from 30-39", Pct, true),
            Fg40s => ("FG 40s", "Field goal % from 40-49", Pct, true),
            Fg50Pct => ("FG 50+", "Field goal % from 50 or more", Pct, true),
            ClutchPct => ("Clutch", "Field goal % in the clutch", Pct, true),
            XpPct => ("XP%", "Extra point percentage", Pct, true),
            KoTbRate => ("TB%", "Kickoff touchback rate", Pct, true),
            KoHang => ("KO hang", "Kickoff hang time", Two, true),
            KoRetAvg => ("Ret A", "Return yards allowed a kickoff return", One, false),
            Gross => ("Gross", "Gross punting average", One, true),
            In20Rate => ("In20%", "Punts inside the 20", Pct, true),
            PuntTbRate => ("TB%", "Punt touchback rate", Pct, false),
            PuntHang => ("Hang", "Punt hang time", Two, true),
            FairRate => ("FC%", "Punts fair caught", Pct, true),
            PuntRetAvg => ("Ret A", "Return yards allowed a punt return", One, false),
            KrAvg => ("KR avg", "Kick return average", One, true),
            KrLong => ("KR long", "Longest kick return", Whole, true),
            KrTd => ("KR TD", "Kick return touchdowns", Whole, true),
            PrAvg => ("PR avg", "Punt return average", One, true),
            PrLong => ("PR long", "Longest punt return", Whole, true),
            PrTd => ("PR TD", "Punt return touchdowns", Whole, true),
            FairCatches => ("FC", "Fair catches", Whole, true),
            Returns => ("Ret", "Kick and punt returns", Whole, true),
            RetAvg => ("Avg", "Yards a return", One, true),
            RetTd => ("TD", "Return touchdowns", Whole, true),
            GrPass => ("Pass gr", "Passing grade", One, true),
            GrRush => ("Run gr", "Running grade", One, true),
            GrRecv => ("Recv gr", "Receiving grade", One, true),
            GrPb => ("PB gr", "Pass-blocking grade", One, true),
            GrRb => ("RB gr", "Run-blocking grade", One, true),
            GrPr => ("PR gr", "Pass-rush grade", One, true),
            GrRd => ("RD gr", "Run-defense grade", One, true),
            GrCov => ("Cov gr", "Coverage grade", One, true),
            GrTkl => ("Tkl gr", "Tackling grade", One, true),
            GrSt => ("ST gr", "Special teams grade", One, true),
            ZqBehind | ZrBehind => ("Behind", "Completion % behind the line", Pct, true),
            ZqShort | ZrShort => ("0-9", "Completion % 0-9 yards downfield", Pct, true),
            ZqMid | ZrMid => ("10-19", "Completion % 10-19 yards downfield", Pct, true),
            ZqDeep | ZrDeep => ("20+", "Completion % 20+ yards downfield", Pct, true),
            ZqLeft | ZrLeft => ("Left", "EPA a throw to the left third", Signed, true),
            ZqMiddle | ZrMiddle => ("Middle", "EPA a throw to the middle third", Signed, true),
            ZqRight | ZrRight => ("Right", "EPA a throw to the right third", Signed, true),
            ZdBehind => ("Behind", "Completion % allowed behind the line", Pct, false),
            ZdShort => ("0-9", "Completion % allowed 0-9 yards downfield", Pct, false),
            ZdMid => ("10-19", "Completion % allowed 10-19 yards downfield", Pct, false),
            ZdDeep => ("20+", "Completion % allowed 20+ yards downfield", Pct, false),
            ZdLeft => ("Left", "EPA allowed a throw to the left third", Signed, false),
            ZdMiddle => ("Middle", "EPA allowed a throw to the middle third", Signed, false),
            ZdRight => ("Right", "EPA allowed a throw to the right third", Signed, false),
        }
    }

    /// A number that describes how he plays, not how well (time to throw,
    /// where he lines up, how often he blitzes): its shade is plain
    /// percentile, high is more, and it sorts the same.
    pub fn neutral(self) -> bool {
        use Cm::*;
        matches!(
            self,
            Ttt | Aggr
                | Cay
                | DeepRate
                | TaRate
                | PaRate
                | BlitzRate
                | ScrRate
                | Box8Rate
                | RAdot
                | SlotRate
                | WideRate
                | InlineRate
                | BackRate
                | TackleRate
                | GuardRate
                | CenterRate
                | Unblocked
                | SpeedShare
                | PowerShare
                | FinesseShare
                | BlitzUse
                | AdotA
                | BoxRate
                | DSlotRate
                | DWideRate
                | DeepSpot
                | LineRate
                | FairCatches
                | Adot
                | PressRate
        )
    }

    /// Whether the number reads the players tally's depth counts (the
    /// seasons since the tallies began), not the stat book.
    pub fn deep(self) -> bool {
        (self as u16) > (Cm::Grade as u16) && !matches!(self, Cm::Returns | Cm::RetAvg | Cm::RetTd)
    }

    /// Its value from a season's depth counts (`deep` numbers only): `d`
    /// reads a code of `tally::depth`.
    pub fn value_d(self, d: &dyn Fn(u16) -> f64) -> Option<f64> {
        use Cm::*;
        let r = |n: f64, den: f64| if den > 0.0 { Some(n / den) } else { None };
        let pct = |n: f64, den: f64| r(n, den).map(|x| x * 100.0);
        let c = |k: Dc| d(k as u16);
        let align_o = |spots: &[u8]| {
            let all: f64 = (0..dp::orole::N as u16).map(|k| d(dp::ALIGN_O + k)).sum();
            pct(spots.iter().map(|&k| d(dp::ALIGN_O + k as u16)).sum(), all)
        };
        let all_d = || -> f64 { (0..dp::dslot::N as u16).map(|k| d(dp::ALIGN_D + k)).sum() };
        let align_d = |spots: &[u8]| pct(spots.iter().map(|&k| d(dp::ALIGN_D + k as u16)).sum(), all_d());
        let fg = |band: u16| pct(d(dp::FG + 2 * band + 1), d(dp::FG + 2 * band));
        let facet = |f: u16| {
            let n = d(dp::FACET + 2 * f + 1);
            (n >= 1.0).then(|| {
                crate::game::stats::StatLine {
                    grade_sum: d(dp::FACET + 2 * f),
                    graded: n.round() as u32,
                    ..Default::default()
                }
                .grade()
            })
        };
        // A role's throw grid: completion % down a depth row, EPA a throw
        // across a third.
        let row = |role: usize, row: usize| {
            let (mut a, mut m) = (0.0, 0.0);
            for t in 0..3 {
                a += d(dp::throw_code(role, row, t, 0));
                m += d(dp::throw_code(role, row, t, 1));
            }
            pct(m, a)
        };
        let third = |role: usize, t: usize| {
            let (mut a, mut e) = (0.0, 0.0);
            for w in 0..4 {
                a += d(dp::throw_code(role, w, t, 0));
                e += d(dp::throw_code(role, w, t, 3));
            }
            r(e, a)
        };
        use dp::dslot as ds;
        use dp::orole as or;
        match self {
            DbSucc => pct(c(Dc::PassSucc) + c(Dc::ScrSucc), c(Dc::Db)),
            Anya => r(
                c(Dc::PassYds) + 20.0 * c(Dc::PassTd) - 45.0 * c(Dc::Int) - c(Dc::SackYds),
                c(Dc::Att) + c(Dc::Sacked),
            ),
            SackYds => (c(Dc::Db) > 0.0).then(|| c(Dc::SackYds)),
            Ttt => r(c(Dc::Ttt), c(Dc::Att)),
            Aggr => pct(c(Dc::Tight), c(Dc::Att)),
            Cay => r(c(Dc::CmpAir), c(Dc::Cmp)),
            DeepRate => pct(c(Dc::DeepAtt), c(Dc::Att)),
            DeepCmp => pct(c(Dc::DeepCmp), c(Dc::DeepAtt)),
            DeepCpoe => pct(c(Dc::DeepCmp) - c(Dc::DeepXCmp), c(Dc::DeepAtt)),
            DeepYpa => r(c(Dc::DeepYds), c(Dc::DeepAtt)),
            TaRate => pct(c(Dc::ThrowAway), c(Dc::Db)),
            PaRate => pct(c(Dc::PaDb), c(Dc::Db)),
            PaYpa => r(c(Dc::PaYds), c(Dc::PaAtt)),
            NoPaYpa => r(c(Dc::PassYds) - c(Dc::PaYds), c(Dc::Att) - c(Dc::PaAtt)),
            PaEpaDb => r(c(Dc::PaEpa), c(Dc::PaDb)),
            BlitzRate => pct(c(Dc::BlzDb), c(Dc::Db)),
            BlitzEpaDb => r(c(Dc::BlzEpa), c(Dc::BlzDb)),
            BlitzYpa => r(c(Dc::BlzYds), c(Dc::BlzAtt)),
            CleanEpaDb => r(c(Dc::PassEpa) + c(Dc::ScrEpa) - c(Dc::PrsEpa), c(Dc::Db) - c(Dc::PrsDb)),
            PrsEpaDb => r(c(Dc::PrsEpa), c(Dc::PrsDb)),
            CleanYpa => r(c(Dc::PassYds) - c(Dc::PrsYds), c(Dc::Att) - c(Dc::PrsAtt)),
            PrsYpa => r(c(Dc::PrsYds), c(Dc::PrsAtt)),
            PrsCmp => pct(c(Dc::PrsCmp), c(Dc::PrsAtt)),
            ScrRate => pct(c(Dc::Scr), c(Dc::Db)),
            ScrYds => (c(Dc::Db) + c(Dc::Scr) > 0.0).then(|| c(Dc::ScrYds)),
            ScrYpc => r(c(Dc::ScrYds), c(Dc::Scr)),
            Ybc => r(c(Dc::RYbc), c(Dc::RAtt)),
            Ryoe => r(c(Dc::RYds) - c(Dc::RXYds), c(Dc::RAtt)),
            RyoeTot => (c(Dc::RAtt) > 0.0).then(|| c(Dc::RYds) - c(Dc::RXYds)),
            RyoeRate => pct(c(Dc::RBeat), c(Dc::RAtt)),
            Stuff => pct(c(Dc::RStuff), c(Dc::RAtt)),
            Breakaway => (c(Dc::RYds) > 0.0).then(|| c(Dc::RBreakYds) / c(Dc::RYds) * 100.0),
            // PFF's elusive rating: missed tackles forced a touch, times
            // yards after contact a carry, times 100.
            Elusive => r(c(Dc::RMtf) + c(Dc::RecMtf), c(Dc::RAtt) + c(Dc::Rec))
                .and_then(|m| r(c(Dc::RYaco), c(Dc::RAtt)).map(|y| m * y * 100.0)),
            RFirst => pct(c(Dc::RFirst), c(Dc::RAtt)),
            REpa => r(c(Dc::REpa), c(Dc::RAtt)),
            Box8Rate => pct(c(Dc::RBox8), c(Dc::RAtt)),
            Box8Ypc => r(c(Dc::RBox8Yds), c(Dc::RBox8)),
            RAdot => r(c(Dc::TgtAir), c(Dc::Tgt)),
            TgtShare => pct(c(Dc::Tgt), c(Dc::TeamTgt)),
            AirShare => pct(c(Dc::TgtAir), c(Dc::TeamAir)),
            Separation => r(c(Dc::Sep), c(Dc::Tgt)),
            ContRate => pct(c(Dc::ContRec), c(Dc::ContTgt)),
            Yacoe => r(c(Dc::Yac) - c(Dc::XYac), c(Dc::Rec)),
            FdRoute => r(c(Dc::RecFirst), c(Dc::Routes)),
            EpaTgt => r(c(Dc::RecEpa), c(Dc::Tgt)),
            TgtRating => passer_rating(c(Dc::Tgt), c(Dc::Rec), c(Dc::RecYds), c(Dc::RecTd), c(Dc::TgtInt)),
            SlotRate => align_o(&[or::SLOT, or::SLOT2, or::SLOT3]),
            WideRate => align_o(&[or::X, or::Z]),
            InlineRate => align_o(&[or::TE, or::TE2, or::TE3]),
            BackRate => align_o(&[or::RB, or::FB]),
            // PFF's pass-blocking efficiency: a hurry or a hit counts three
            // quarters of a sack.
            Pbe => r(c(Dc::SacksAllowed) + 0.75 * (c(Dc::HitsAllowed) + c(Dc::HurriesAllowed)), c(Dc::PbSnaps))
                .map(|x| 100.0 - 100.0 * x),
            PbWr => pct(c(Dc::PbWins), c(Dc::PbSnaps)),
            PrsAllowedN => (c(Dc::PbSnaps) > 0.0).then(|| c(Dc::PrsAllowed)),
            SacksAllowedN => (c(Dc::PbSnaps) > 0.0).then(|| c(Dc::SacksAllowed)),
            HitsAllowedN => (c(Dc::PbSnaps) > 0.0).then(|| c(Dc::HitsAllowed)),
            HurriesAllowedN => (c(Dc::PbSnaps) > 0.0).then(|| c(Dc::HurriesAllowed)),
            PenN => Some(c(Dc::Pens)),
            BustN => Some(c(Dc::Busts)),
            TackleRate => align_o(&[or::LT, or::RT]),
            GuardRate => align_o(&[or::LG, or::RG]),
            CenterRate => align_o(&[or::C]),
            // PFF's pass-rush productivity, the same weights.
            Prp => r(c(Dc::Sacks) + 0.75 * (c(Dc::Hits) + c(Dc::Hurries)), c(Dc::PrSnaps)).map(|x| 100.0 * x),
            PrsN => (c(Dc::PrSnaps) > 0.0).then(|| c(Dc::Prs)),
            HitsN => (c(Dc::PrSnaps) > 0.0).then(|| c(Dc::Hits)),
            HurriesN => (c(Dc::PrSnaps) > 0.0).then(|| c(Dc::Hurries)),
            GetOff => r(c(Dc::GetOff), c(Dc::GetOffN)),
            Ttp => r(c(Dc::Ttp), c(Dc::TtpN)),
            Unblocked => pct(c(Dc::PrsFree), c(Dc::Prs)),
            SpeedShare | PowerShare | FinesseShare => {
                let k = match self {
                    SpeedShare => Dc::WinSpeed,
                    PowerShare => Dc::WinPower,
                    _ => Dc::WinFinesse,
                };
                pct(c(k), c(Dc::WinSpeed) + c(Dc::WinPower) + c(Dc::WinFinesse))
            }
            RunStop => pct(c(Dc::RunStops), c(Dc::DefRunSnaps)),
            BlitzUse => pct(c(Dc::Blitz), c(Dc::DefDbSnaps)),
            BlitzPrs => pct(c(Dc::BlitzPrs), c(Dc::Blitz)),
            CmpA => pct(c(Dc::CmpA), c(Dc::TgtA)),
            RatingA => passer_rating(c(Dc::TgtA), c(Dc::CmpA), c(Dc::YdsA), c(Dc::TdA), c(Dc::IntA)),
            YdsTgtA => r(c(Dc::YdsA), c(Dc::TgtA)),
            SepA => r(c(Dc::SepA), c(Dc::TgtA)),
            FInc => pct(c(Dc::FIncA), c(Dc::TgtA)),
            TdA => (c(Dc::CovSnaps) > 0.0).then(|| c(Dc::TdA)),
            EpaTgtA => r(c(Dc::EpaA), c(Dc::TgtA)),
            YacA => r(c(Dc::YacA), c(Dc::CmpA)),
            AdotA => r(c(Dc::AirA), c(Dc::TgtA)),
            // In the box: the linebackers' spots and a safety rolled down.
            BoxRate => pct(
                [ds::SAM, ds::MIKE, ds::WILL].iter().map(|&k| d(dp::ALIGN_D + k as u16)).sum::<f64>() + c(Dc::InBox),
                all_d(),
            ),
            DSlotRate => align_d(&[ds::NCB, ds::DCB]),
            DWideRate => align_d(&[ds::CBL, ds::CBR]),
            DeepSpot => pct(d(dp::ALIGN_D + ds::FS as u16) + d(dp::ALIGN_D + ds::SS as u16) - c(Dc::InBox), all_d()),
            LineRate => align_d(&[ds::LE, ds::RE, ds::DTL, ds::DTR, ds::NT]),
            Fg30s => fg(1),
            Fg40s => fg(2),
            Fg50Pct => fg(3),
            ClutchPct => pct(c(Dc::ClutchMade), c(Dc::ClutchAtt)),
            XpPct => pct(c(Dc::XpMade), c(Dc::XpAtt)),
            KoTbRate => pct(c(Dc::KoTb), c(Dc::Ko)),
            KoHang => r(c(Dc::KoHang), c(Dc::Ko)),
            KoRetAvg => r(c(Dc::KoRetYds), c(Dc::KoRet)),
            Gross => r(c(Dc::PuntYds), c(Dc::Punts)),
            In20Rate => pct(c(Dc::PuntIn20), c(Dc::Punts)),
            PuntTbRate => pct(c(Dc::PuntTb), c(Dc::Punts)),
            PuntHang => r(c(Dc::PuntHang), c(Dc::Punts)),
            FairRate => pct(c(Dc::PuntFair), c(Dc::Punts)),
            PuntRetAvg => r(c(Dc::PuntRetYds), c(Dc::PuntRet)),
            KrAvg => r(c(Dc::KrYds), c(Dc::Kr)),
            KrLong => (c(Dc::Kr) > 0.0).then(|| c(Dc::KrLong)),
            KrTd => (c(Dc::Kr) > 0.0).then(|| c(Dc::KrTd)),
            PrAvg => r(c(Dc::PrYds), c(Dc::Pr)),
            PrLong => (c(Dc::Pr) > 0.0).then(|| c(Dc::PrLong)),
            PrTd => (c(Dc::Pr) > 0.0).then(|| c(Dc::PrTd)),
            FairCatches => (c(Dc::Pr) + c(Dc::FairCatches) > 0.0).then(|| c(Dc::FairCatches)),
            GrPass => facet(0),
            GrRush => facet(1),
            GrRecv => facet(2),
            GrPb => facet(3),
            GrRb => facet(4),
            GrPr => facet(5),
            GrRd => facet(6),
            GrCov => facet(7),
            GrTkl => facet(8),
            GrSt => facet(9),
            ZqBehind | ZrBehind | ZdBehind => row(self.zone_role(), 0),
            ZqShort | ZrShort | ZdShort => row(self.zone_role(), 1),
            ZqMid | ZrMid | ZdMid => row(self.zone_role(), 2),
            ZqDeep | ZrDeep | ZdDeep => row(self.zone_role(), 3),
            ZqLeft | ZrLeft | ZdLeft => third(self.zone_role(), 0),
            ZqMiddle | ZrMiddle | ZdMiddle => third(self.zone_role(), 1),
            ZqRight | ZrRight | ZdRight => third(self.zone_role(), 2),
            _ => None,
        }
    }

    /// A zone number's role in the throw grid: 0 the passer, 1 the target,
    /// 2 the man covering.
    fn zone_role(self) -> usize {
        use Cm::*;
        match self {
            ZrBehind | ZrShort | ZrMid | ZrDeep | ZrLeft | ZrMiddle | ZrRight => 1,
            ZdBehind | ZdShort | ZdMid | ZdDeep | ZdLeft | ZdMiddle | ZdRight => 2,
            _ => 0,
        }
    }

    /// Its value from the stat book or the depth counts, whichever it reads.
    pub fn value_any(self, get: &dyn Fn(Stat) -> f64, d: &dyn Fn(u16) -> f64, games: f64) -> Option<f64> {
        if self.deep() { self.value_d(d) } else { self.value(get, games) }
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
            // A scramble is a dropback, so its EPA counts (minor bug 183, as
            // nflfastR counts it).
            EpaDb => r(g(S::PassEpa) + g(S::ScrambleEpa), g(S::Dropbacks)),
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
            // Summed on catches, so a catch's (minor fix, lane NB4).
            Sep => r(g(S::SepSum), g(S::Rec)),
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
            // The game's 0-100 grade, as the game pages show it.
            Grade => (g(S::Graded) >= 1.0).then(|| {
                crate::game::stats::StatLine {
                    grade_sum: g(S::GradeSum),
                    graded: g(S::Graded).round() as u32,
                    ..Default::default()
                }
                .grade()
            }),
            Returns => Some(g(S::Returns)),
            RetAvg => r(g(S::ReturnYds), g(S::Returns)),
            RetTd => (g(S::Returns) > 0.0).then(|| g(S::ReturnTd)),
            _ => None,
        }
    }
}

/// What an opponent-adjusted number (Data overhaul item 13) weighs a
/// player's opponents by: a rate of the opponent's season, as the teams'
/// game boxes give it. A man on offense is read against the defenses he
/// faced (what offenses did against them), a man on defense against the
/// offenses he faced (what they did against everyone).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OppRate {
    /// EPA a dropback.
    PassEpa,
    /// Successful dropbacks, a share.
    PassSucc,
    /// Net passing yards an attempt.
    Ypa,
    /// Completions an attempt.
    Cmp,
    /// Sacks a dropback.
    Sack,
    /// Pressures a dropback.
    Prs,
    /// EPA a designed run.
    RunEpa,
    /// Successful designed runs, a share.
    RunSucc,
    /// Designed runs stuffed (no gain or a loss), a share.
    Stuff,
    /// Rushing yards a carry.
    Ypc,
}

/// How a number is opponent-adjusted (item 13): the opponent rate it reads
/// and how. `Some((rate, None))` scales it by the level's rate over his
/// opponents' (yardage rates); `Some((rate, Some(k)))` takes `k` times his
/// opponents' rate over the level's off it (EPA and shares; `k` turns a
/// share into percent and carries the sign). The level's rate is the
/// average his field faced, so the field's average stays put. Each game
/// weighs by his snaps in it; an opponent's season leaves out its games
/// against his team. Proposed, not confirmed.
pub fn opp_adjust(cm: Cm) -> Option<(OppRate, Option<f64>)> {
    use Cm::*;
    use OppRate as O;
    Some(match cm {
        EpaDb | EpaTgt | EpaTgtA => (O::PassEpa, Some(1.0)),
        DbSucc => (O::PassSucc, Some(100.0)),
        Ypa | Anya | Yprr | YdsTgtA | YdsCov => (O::Ypa, None),
        CmpPct | CatchRate | CmpA => (O::Cmp, Some(100.0)),
        SackRate | SackAllowed | SackPerRush => (O::Sack, Some(100.0)),
        PressRate | PressAllowed | Pressure => (O::Prs, Some(100.0)),
        // A hurry or a hit is three quarters of a sack in PBE and PRP.
        Pbe => (O::Prs, Some(-75.0)),
        Prp => (O::Prs, Some(75.0)),
        REpa => (O::RunEpa, Some(1.0)),
        Success => (O::RunSucc, Some(100.0)),
        // A stop is a run that failed: an offense that succeeds more leaves
        // fewer.
        Stop | RunStop => (O::RunSucc, Some(-100.0)),
        Stuff => (O::Stuff, Some(100.0)),
        Ypc => (O::Ypc, None),
        _ => return None,
    })
}

/// The spots the filter by exact spot picks (Data overhaul item 14), on
/// offense and on defense.
pub const SPOTS_O: [&str; 8] = ["quarterback", "wide", "slot", "in line", "backfield", "tackle", "guard", "center"];
pub const SPOTS_D: [&str; 7] =
    ["edge", "interior line", "off-ball linebacker", "outside corner", "slot corner", "deep safety", "box safety"];

/// His snaps at one spot of `SPOTS_O` (`offense`) or `SPOTS_D`, from the
/// depth counts (`d` reads a code). A safety rolled into the box counts as a
/// box safety, else a deep one.
pub fn spot_snaps(d: &dyn Fn(u16) -> f64, offense: bool, k: usize) -> f64 {
    use dp::dslot as ds;
    use dp::orole as or;
    let sum = |base: u16, s: &[u8]| s.iter().map(|&x| d(base + x as u16)).sum::<f64>();
    if offense {
        let s: &[u8] = match k {
            0 => &[or::QB],
            1 => &[or::X, or::Z],
            2 => &[or::SLOT, or::SLOT2, or::SLOT3],
            3 => &[or::TE, or::TE2, or::TE3],
            4 => &[or::RB, or::FB],
            5 => &[or::LT, or::RT],
            6 => &[or::LG, or::RG],
            _ => &[or::C],
        };
        sum(dp::ALIGN_O, s)
    } else {
        let box_s = d(Dc::InBox as u16);
        match k {
            0 => sum(dp::ALIGN_D, &[ds::LE, ds::RE]),
            1 => sum(dp::ALIGN_D, &[ds::DTL, ds::DTR, ds::NT]),
            2 => sum(dp::ALIGN_D, &[ds::SAM, ds::MIKE, ds::WILL]),
            3 => sum(dp::ALIGN_D, &[ds::CBL, ds::CBR]),
            4 => sum(dp::ALIGN_D, &[ds::NCB, ds::DCB]),
            5 => (sum(dp::ALIGN_D, &[ds::FS, ds::SS]) - box_s).max(0.0),
            _ => box_s,
        }
    }
}

/// The spot he played most, as an index into `SPOTS_O` or `SPOTS_D`.
pub fn spot_of(d: &dyn Fn(u16) -> f64, offense: bool) -> Option<usize> {
    let n = if offense { SPOTS_O.len() } else { SPOTS_D.len() };
    (0..n).map(|k| (k, spot_snaps(d, offense, k))).filter(|x| x.1 > 0.0).max_by(|a, b| a.1.total_cmp(&b.1)).map(|x| x.0)
}

/// Whether a group plays offense (its opponents' defenses adjust it).
pub fn offense_group(group: usize) -> bool {
    group <= 4
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
pub const fn charted(group: usize) -> &'static [Cm] {
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
        11 => &[NetPunt, Punts],
        _ => &[Returns, RetAvg, RetTd, KrAvg, KrLong, KrTd, PrAvg, PrLong, PrTd, FairCatches, GrSt],
    }
}

/// The views of a group's charted card (Data overhaul items 4-11 and 28):
/// the overview first (`charted`), then the position's depth cards, its
/// facet grades and, for passers, receivers and the men covering, the
/// throw map by zone. Each has no more than `CARD_MAX` numbers.
pub fn views(group: usize) -> &'static [(&'static str, &'static [Cm])] {
    use Cm::*;
    const ZQ: &[Cm] = &[ZqBehind, ZqShort, ZqMid, ZqDeep, ZqLeft, ZqMiddle, ZqRight];
    const ZR: &[Cm] = &[ZrBehind, ZrShort, ZrMid, ZrDeep, ZrLeft, ZrMiddle, ZrRight];
    const ZD: &[Cm] = &[ZdBehind, ZdShort, ZdMid, ZdDeep, ZdLeft, ZdMiddle, ZdRight];
    const COV: &[Cm] = &[CmpA, RatingA, YdsTgtA, EpaTgtA, SepA, FInc, YacA, AdotA, TdA, YdsCov, TgtCov, BallsPlayed];
    const SPOT_D: &[Cm] = &[BoxRate, LineRate, DSlotRate, DWideRate, DeepSpot];
    match group {
        0 => {
            const {
                &[
                    ("Overview", charted(0)),
                    (
                        "Accuracy and depth",
                        &[Cpoe, CmpPct, Aggr, Adot, Cay, Ttt, DeepRate, DeepCmp, DeepCpoe, DeepYpa, TaRate, Anya],
                    ),
                    (
                        "Pressure and blitz",
                        &[
                            PressRate, SackRate, PressSack, SackYds, CleanEpaDb, PrsEpaDb, CleanYpa, PrsYpa, PrsCmp,
                            BlitzRate, BlitzEpaDb, BlitzYpa,
                        ],
                    ),
                    (
                        "Play action and scrambles",
                        &[PaRate, PaEpaDb, PaYpa, NoPaYpa, ScrRate, ScrYds, ScrYpc, DbSucc, EpaDb, Anya],
                    ),
                    ("Grades", &[Grade, GrPass, GrRush]),
                    ("By zone", ZQ),
                ]
            }
        }
        1 => {
            const {
                &[
                    ("Overview", charted(1)),
                    (
                        "Running",
                        &[Ypc, Ybc, YacoC, Ryoe, RyoeTot, RyoeRate, Stuff, Breakaway, Elusive, RFirst, REpa, Success],
                    ),
                    ("Against the box", &[Box8Rate, Box8Ypc, Ryoe, Ypc, Stuff]),
                    (
                        "Receiving and usage",
                        &[TgtShare, Yprr, CatchRate, Drop, YacR, Yacoe, EpaTgt, RAdot, BackRate, SlotRate, WideRate],
                    ),
                    ("Blocking", &[Pbe, PbWr, PrsAllowedN, SacksAllowedN, PenN, BustN]),
                    ("Grades", &[Grade, GrRush, GrRecv, GrPb, GrSt]),
                ]
            }
        }
        2 | 3 => {
            const WR: &[(&str, &[Cm])] = &[
                ("Overview", charted(2)),
                (
                    "Routes and separation",
                    &[
                        Separation, Sep, ContRate, RAdot, TgtShare, AirShare, FdRoute, EpaTgt, TgtRating, Yacoe, Drop,
                        Yprr,
                    ],
                ),
                ("Where he lines up", &[WideRate, SlotRate, InlineRate, BackRate]),
                ("Blocking", &[Pbe, PbWr, PrsAllowedN, SacksAllowedN, RbWin, PenN, BustN]),
                ("Grades", &[Grade, GrRecv, GrRb, GrPb, GrRush, GrSt]),
                ("By zone", ZR),
            ];
            WR
        }
        4 => {
            const {
                &[
                    ("Overview", charted(4)),
                    (
                        "Pass protection",
                        &[
                            Pbe,
                            PbWr,
                            PressAllowed,
                            SackAllowed,
                            PrsAllowedN,
                            SacksAllowedN,
                            HitsAllowedN,
                            HurriesAllowedN,
                        ],
                    ),
                    ("Discipline and spots", &[PenN, BustN, TackleRate, GuardRate, CenterRate]),
                    ("Grades", &[Grade, GrPb, GrRb]),
                ]
            }
        }
        5 | 6 => {
            const DL: &[(&str, &[Cm])] = &[
                ("Overview", charted(5)),
                (
                    "Pass rush",
                    &[Prp, PrWin, Pressure, PrsN, Sacks, HitsN, HurriesN, SackPerRush, GetOff, Ttp, Unblocked],
                ),
                ("Rush moves", &[SpeedShare, PowerShare, FinesseShare, PrWin, GetOff]),
                ("Run defense", &[RunStop, Stop, Tfl, Miss, TklG, LineRate, BoxRate]),
                ("Grades", &[Grade, GrPr, GrRd, GrTkl, GrCov, GrSt]),
            ];
            DL
        }
        7 => {
            const {
                &[
                    ("Overview", charted(7)),
                    ("Coverage", COV),
                    ("Rush and blitz", &[BlitzUse, BlitzPrs, Prp, PrsN, Sacks, Ttp, GetOff]),
                    ("Run defense", &[RunStop, Stop, Tfl, Miss, TklG]),
                    ("Where he lines up", SPOT_D),
                    ("Grades", &[Grade, GrRd, GrCov, GrPr, GrTkl, GrSt]),
                    ("By zone", ZD),
                ]
            }
        }
        8 | 9 => {
            const DB: &[(&str, &[Cm])] = &[
                ("Overview", charted(8)),
                ("Coverage", COV),
                ("Run support and blitz", &[RunStop, Stop, Miss, TklG, BlitzUse, BlitzPrs, PrsN]),
                ("Where he lines up", SPOT_D),
                ("Grades", &[Grade, GrCov, GrRd, GrTkl, GrPr, GrSt]),
                ("By zone", ZD),
            ];
            DB
        }
        10 => {
            const {
                &[
                    ("Overview", charted(10)),
                    ("Kicking", &[Fg30s, Fg40s, Fg50Pct, ClutchPct, XpPct, KoTbRate, KoHang, KoRetAvg, GrSt]),
                ]
            }
        }
        11 => {
            const {
                &[
                    ("Overview", charted(11)),
                    ("Punting", &[Gross, NetPunt, In20Rate, PuntTbRate, PuntHang, FairRate, PuntRetAvg, GrSt]),
                ]
            }
        }
        _ => const { &[("Overview", charted(12))] },
    }
}

/// How deep charting must go for a number (Data overhaul item 2): 1 the
/// play by play (EPA, success, explosive plays and what a box score gives),
/// 2 film charting (pressures, coverage, expected completions, missed
/// tackles, grades), 3 tracking-style detail (separation and the trench
/// win rates, ESPN's from the NFL's tracking). Proposed, not confirmed.
pub fn depth(cm: Cm) -> u8 {
    use Cm::*;
    match cm {
        Sep | RbWin | PrWin => 3,
        // Tracking: the clock on throws, contact and hang times, separation,
        // the expected yards models and the trench win rates.
        Ttt | Aggr | Separation | SepA | GetOff | Ttp | KoHang | PuntHang | Ryoe | RyoeTot | RyoeRate | Yacoe
        | PbWr | SpeedShare | PowerShare | FinesseShare => 3,
        // What the play by play gives: air yards and where the ball went,
        // carries, EPA, kicks and returns.
        Anya | SackYds | Cay | DeepRate | DeepCmp | DeepYpa | ScrRate | ScrYds | ScrYpc | DbSucc | Stuff
        | Breakaway | RFirst | REpa | RAdot | TgtShare | AirShare | EpaTgt | TgtRating | PenN | Fg30s | Fg40s
        | Fg50Pct | ClutchPct | XpPct | KoTbRate | KoRetAvg | Gross | In20Rate | PuntTbRate | FairRate | PuntRetAvg
        | KrAvg | KrLong | KrTd | PrAvg | PrLong | PrTd | FairCatches | Returns | RetAvg | RetTd | ZqBehind
        | ZqShort | ZqMid | ZqDeep | ZqLeft | ZqMiddle | ZqRight | ZrBehind | ZrShort | ZrMid | ZrDeep | ZrLeft
        | ZrMiddle | ZrRight => 1,
        _ if cm.deep() => 2,
        Cpoe | Adot | Btt | Twp | PressRate | PressSack | YacoC | Mtf | Yprr | TgtRoute | Drop | YacR | Pressure
        | SackPerRush | PressAllowed | SackAllowed | Stop | Miss | YdsCov | TgtCov | BallsPlayed | Grade => 2,
        _ => 1,
    }
}

/// What an analytics head of a 1-20 rating gets a staff that charts its own
/// level (Data overhaul item 2): how deep the charting goes (`depth`) and
/// how many days after a game its charting arrives. A rating of 15 or more
/// charts to tracking-style detail, 8 or more the film; 17 or more has it
/// the next morning, 12 two days, 7 three, less four. Proposed, not
/// confirmed. The NFL's league tracking is 3 deep and a day late for
/// every seat.
pub fn charting(rating: u8) -> (u8, u8) {
    let r = if rating == 0 { 10 } else { rating };
    let depth = match r {
        15.. => 3,
        8.. => 2,
        _ => 1,
    };
    let days = match r {
        17.. => 1,
        12.. => 2,
        7.. => 3,
        _ => 4,
    };
    (depth, days)
}

/// What each depth is called.
pub fn depth_name(d: u8) -> &'static str {
    match d {
        3 => "tracking-style charting",
        2 => "film charting",
        _ => "play-by-play charting",
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
        12 => &[Returns, RetAvg, RetTd],
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
        11 => (Stat::Punts, "punts"),
        _ => (Stat::Returns, "returns"),
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
        12 => (Stat::Returns, "returns"),
        _ => (Stat::Tackles, "tackles"),
    }
}

/// The default qualifier for a season: about two thirds of a starter's
/// volume at the level (the page's minimum can be changed). `nfl` for the
/// NFL's 17 games, else a college or high school season's 12.
pub fn default_min(group: usize, charted: bool, nfl: bool) -> f64 {
    let m = if charted {
        [200.0, 80.0, 180.0, 150.0, 200.0, 150.0, 150.0, 300.0, 200.0, 200.0, 8.0, 20.0, 15.0][group]
    } else {
        [80.0, 50.0, 15.0, 15.0, 0.0, 15.0, 15.0, 25.0, 15.0, 15.0, 5.0, 10.0, 10.0][group]
    };
    if nfl { m * 1.35 } else { m }
}
