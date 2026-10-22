//! Section 3, topic 8: the career stats record. Every college season of
//! every player at every division is kept forever as one line (8.6), built
//! from his game lines and tagged with his level, program and schedule
//! strength (8.4). On top of it: percentiles among his peers, each season's
//! norms for the era view, record books per division and conference with
//! streaks, the College Football Hall of Fame, coach buzz from unit stats
//! (world item 5) and the hidden-gem footprint flags (world item 9).
//!
//! High school games still run on the stand-in, which keeps no player
//! stats, so high school seasons are not in the book yet.

use std::collections::HashMap;

use super::data::Division;
use super::{Stage, World};
use crate::calendar::{Event, Kind, Level as CalLevel, Loudness};
use crate::game::stats::StatLine;
use crate::positions::Position;
use crate::rng::clamp;

macro_rules! stats {
    ($($name:ident),* $(,)?) => {
        /// The numbers kept for every season: the classic box score and the
        /// advanced stats the percentiles, footprints and records read.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        pub enum Stat { $($name),* }
        impl Stat {
            pub const ALL: &'static [Stat] = &[$(Stat::$name),*];
            pub fn key(self) -> &'static str {
                match self { $(Stat::$name => stringify!($name)),* }
            }
        }
    };
}

stats!(
    SnapsOff,
    SnapsDef,
    SnapsSt,
    Att,
    Cmp,
    PassYds,
    PassTd,
    Int,
    Sacked,
    Dropbacks,
    Pressured,
    AirYds,
    XCmp,
    PassEpa,
    BigTime,
    TurnoverWorthy,
    Rush,
    RushYds,
    RushTd,
    RushLong,
    Yaco,
    ForcedMissed,
    ExplosiveRuns,
    RushSuccess,
    FumblesLost,
    Targets,
    Rec,
    RecYds,
    RecTd,
    RecLong,
    Drops,
    Yac,
    SepSum,
    Routes,
    PbSnaps,
    PressuresAllowed,
    SacksAllowed,
    RbSnaps,
    RbWins,
    RushSnaps,
    RushWins,
    Sacks,
    Pressures,
    Tackles,
    Tfl,
    Stops,
    Missed,
    ForcedFumbles,
    CovSnaps,
    TgtAllowed,
    YdsAllowed,
    Ints,
    Pbus,
    FgAtt,
    FgMade,
    Fg50Made,
    FgLong,
    XpMade,
    Punts,
    PuntNet,
    Returns,
    ReturnYds,
    ReturnTd,
    GradeSum,
    Graded,
);

pub const STAT_COUNT: usize = Stat::ALL.len();

impl Stat {
    /// How a record book names it.
    pub fn name(self) -> &'static str {
        match self {
            Stat::PassYds => "passing yards",
            Stat::PassTd => "touchdown passes",
            Stat::RushYds => "rushing yards",
            Stat::RushTd => "rushing touchdowns",
            Stat::Rec => "receptions",
            Stat::RecYds => "receiving yards",
            Stat::RecTd => "touchdown catches",
            Stat::Tackles => "tackles",
            Stat::Sacks => "sacks",
            Stat::Ints => "interceptions",
            Stat::Tfl => "tackles for loss",
            Stat::FgMade => "field goals",
            Stat::ReturnTd => "return touchdowns",
            s => s.key(),
        }
    }
}

/// The stats with record books, single season and career.
pub const RECORD_STATS: [Stat; 13] = [
    Stat::PassYds,
    Stat::PassTd,
    Stat::RushYds,
    Stat::RushTd,
    Stat::Rec,
    Stat::RecYds,
    Stat::RecTd,
    Stat::Tackles,
    Stat::Sacks,
    Stat::Ints,
    Stat::Tfl,
    Stat::FgMade,
    Stat::ReturnTd,
];

/// A season's numbers from his summed game lines.
pub fn from_line(l: &StatLine) -> [f32; STAT_COUNT] {
    use Stat::*;
    let mut s = [0f32; STAT_COUNT];
    let mut set = |k: Stat, v: f64| s[k as usize] = v as f32;
    set(SnapsOff, l.snaps_off as f64);
    set(SnapsDef, l.snaps_def as f64);
    set(SnapsSt, l.snaps_st as f64);
    set(Att, l.att as f64);
    set(Cmp, l.cmp as f64);
    set(PassYds, l.pass_yds);
    set(PassTd, l.pass_td as f64);
    set(Int, l.int as f64);
    set(Sacked, l.sacked as f64);
    set(Dropbacks, l.dropbacks as f64);
    set(Pressured, l.pressured as f64);
    set(AirYds, l.air_yds_att);
    set(XCmp, l.x_cmp);
    set(PassEpa, l.pass_epa);
    set(BigTime, l.big_time as f64);
    set(TurnoverWorthy, l.turnover_worthy as f64);
    set(Rush, l.rush as f64);
    set(RushYds, l.rush_yds);
    set(RushTd, l.rush_td as f64);
    set(RushLong, l.rush_long);
    set(Yaco, l.yaco);
    set(ForcedMissed, l.forced_missed as f64);
    set(ExplosiveRuns, l.explosive_runs as f64);
    set(RushSuccess, l.rush_success as f64);
    set(FumblesLost, l.fumbles_lost as f64);
    set(Targets, l.targets as f64);
    set(Rec, l.rec as f64);
    set(RecYds, l.rec_yds);
    set(RecTd, l.rec_td as f64);
    set(RecLong, l.rec_long);
    set(Drops, l.drops as f64);
    set(Yac, l.yac);
    set(SepSum, l.sep_sum);
    set(Routes, l.routes as f64);
    set(PbSnaps, l.pb_snaps as f64);
    set(PressuresAllowed, l.pressures_allowed as f64);
    set(SacksAllowed, l.sacks_allowed);
    set(RbSnaps, l.rb_snaps as f64);
    set(RbWins, l.rb_wins as f64);
    set(RushSnaps, l.rush_snaps as f64);
    set(RushWins, l.rush_wins as f64);
    set(Sacks, l.sacks);
    set(Pressures, l.pressures as f64);
    set(Tackles, (l.tackles + l.assists) as f64);
    set(Tfl, l.tfl);
    set(Stops, l.stops as f64);
    set(Missed, l.missed as f64);
    set(ForcedFumbles, l.forced_fumbles as f64);
    set(CovSnaps, l.cov_snaps as f64);
    set(TgtAllowed, l.tgt_allowed as f64);
    set(YdsAllowed, l.yds_allowed);
    set(Ints, l.ints as f64);
    set(Pbus, l.pbus as f64);
    set(FgAtt, l.fg_att.iter().sum::<u32>() as f64);
    set(FgMade, l.fg_made.iter().sum::<u32>() as f64);
    set(Fg50Made, l.fg_made[3] as f64);
    set(FgLong, l.fg_long);
    set(XpMade, l.xp_made as f64);
    set(Punts, l.punts as f64);
    set(PuntNet, l.punt_net);
    set(Returns, (l.kr + l.pr) as f64);
    set(ReturnYds, l.kr_yds + l.pr_yds);
    set(ReturnTd, (l.kr_td + l.pr_td) as f64);
    set(GradeSum, l.grade_sum);
    set(Graded, l.graded as f64);
    s
}

/// One player's season at one program.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SeasonRecord {
    pub id: u32,
    pub year: i16,
    pub division: Division,
    /// His listed position that season.
    pub pos: Position,
    pub program: u16,
    pub games: u8,
    /// Hidden-gem footprint flags (bit k: `FOOTPRINT[k]` in the top 1% of
    /// his division and position; bit 15: a testing outlier).
    pub footprint: u16,
    /// His opponents' average strength against his division's average, in
    /// rating points: what the opponent-adjusted view reads.
    pub sos: f32,
    #[serde(with = "serde_big_array::BigArray")]
    pub s: [f32; STAT_COUNT],
}

impl SeasonRecord {
    #[inline]
    pub fn get(&self, k: Stat) -> f64 {
        self.s[k as usize] as f64
    }
}

/// The rates percentiles and footprints read, each with the volume that
/// makes a season count (proposed, not confirmed: about a third of a
/// starter's season at that job).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Metric {
    PasserRating,
    EpaPerDropback,
    Cpoe,
    YardsPerAttempt,
    YardsPerCarry,
    YacoPerCarry,
    ForcedMissedPerTouch,
    ExplosiveRunRate,
    YardsPerRouteRun,
    SeparationPerTarget,
    DropRate,
    PressureAllowedRate,
    RunBlockWinRate,
    PassRushWinRate,
    PressureRate,
    StopRate,
    MissedTackleRate,
    YardsPerCoverageSnap,
    BallsPlayedRate,
    FgPct,
    NetPunt,
    GradePerPlay,
}

impl Metric {
    pub fn name(self) -> &'static str {
        use Metric::*;
        match self {
            PasserRating => "passer rating",
            EpaPerDropback => "EPA per dropback",
            Cpoe => "completion % over expected",
            YardsPerAttempt => "yards per attempt",
            YardsPerCarry => "yards per carry",
            YacoPerCarry => "yards after contact per carry",
            ForcedMissedPerTouch => "missed tackles forced per touch",
            ExplosiveRunRate => "explosive run rate",
            YardsPerRouteRun => "yards per route run",
            SeparationPerTarget => "separation at the catch point",
            DropRate => "drop rate",
            PressureAllowedRate => "pressures allowed per pass block",
            RunBlockWinRate => "run block win rate",
            PassRushWinRate => "pass rush win rate",
            PressureRate => "pressure rate",
            StopRate => "run stops per snap",
            MissedTackleRate => "missed tackle rate",
            YardsPerCoverageSnap => "yards allowed per coverage snap",
            BallsPlayedRate => "passes defended per target",
            FgPct => "field goal %",
            NetPunt => "net punting average",
            GradePerPlay => "grade per play",
        }
    }

    pub fn higher_better(self) -> bool {
        !matches!(
            self,
            Metric::DropRate | Metric::PressureAllowedRate | Metric::MissedTackleRate | Metric::YardsPerCoverageSnap
        )
    }

    /// The season's value, or None when it is too small a sample.
    pub fn value(self, r: &SeasonRecord) -> Option<f64> {
        use Metric::*;
        use Stat as S;
        let g = |k: Stat| r.get(k);
        let rate = |num: f64, den: f64, min: f64| if den >= min { Some(num / den) } else { None };
        match self {
            PasserRating => {
                let a = g(S::Att);
                if a < 100.0 {
                    return None;
                }
                let c = |x: f64| x.clamp(0.0, 2.375);
                let p1 = c((g(S::Cmp) / a - 0.3) * 5.0);
                let p2 = c((g(S::PassYds) / a - 3.0) * 0.25);
                let p3 = c(g(S::PassTd) / a * 20.0);
                let p4 = c(2.375 - g(S::Int) / a * 25.0);
                Some((p1 + p2 + p3 + p4) / 6.0 * 100.0)
            }
            EpaPerDropback => rate(g(S::PassEpa), g(S::Dropbacks), 100.0),
            Cpoe => rate(g(S::Cmp) - g(S::XCmp), g(S::Att), 100.0),
            YardsPerAttempt => rate(g(S::PassYds), g(S::Att), 100.0),
            YardsPerCarry => rate(g(S::RushYds), g(S::Rush), 60.0),
            YacoPerCarry => rate(g(S::Yaco), g(S::Rush), 60.0),
            ForcedMissedPerTouch => rate(g(S::ForcedMissed), g(S::Rush) + g(S::Rec), 60.0),
            ExplosiveRunRate => rate(g(S::ExplosiveRuns), g(S::Rush), 60.0),
            YardsPerRouteRun => rate(g(S::RecYds), g(S::Routes), 150.0),
            SeparationPerTarget => rate(g(S::SepSum), g(S::Targets), 30.0),
            DropRate => rate(g(S::Drops), g(S::Targets), 30.0),
            PressureAllowedRate => rate(g(S::PressuresAllowed), g(S::PbSnaps), 150.0),
            RunBlockWinRate => rate(g(S::RbWins), g(S::RbSnaps), 150.0),
            PassRushWinRate => rate(g(S::RushWins), g(S::RushSnaps), 120.0),
            PressureRate => rate(g(S::Pressures), g(S::RushSnaps), 120.0),
            StopRate => rate(g(S::Stops), g(S::SnapsDef), 250.0),
            MissedTackleRate => rate(g(S::Missed), g(S::Tackles) + g(S::Missed), 25.0),
            YardsPerCoverageSnap => rate(g(S::YdsAllowed), g(S::CovSnaps), 150.0),
            BallsPlayedRate => rate(g(S::Ints) + g(S::Pbus), g(S::TgtAllowed), 20.0),
            FgPct => rate(g(S::FgMade), g(S::FgAtt), 10.0),
            NetPunt => rate(g(S::PuntNet), g(S::Punts), 20.0),
            GradePerPlay => rate(g(S::GradeSum), g(S::Graded), 200.0),
        }
    }
}

/// The footprint metrics, by flag bit (world item 9: outliers relative to
/// level, such as yards after contact or separation).
pub const FOOTPRINT: [Metric; 15] = [
    Metric::EpaPerDropback,
    Metric::Cpoe,
    Metric::YacoPerCarry,
    Metric::ForcedMissedPerTouch,
    Metric::ExplosiveRunRate,
    Metric::SeparationPerTarget,
    Metric::YardsPerRouteRun,
    Metric::PressureAllowedRate,
    Metric::RunBlockWinRate,
    Metric::PassRushWinRate,
    Metric::PressureRate,
    Metric::StopRate,
    Metric::MissedTackleRate,
    Metric::YardsPerCoverageSnap,
    Metric::BallsPlayedRate,
];

/// The testing outlier's flag bit.
pub const TESTING: u16 = 1 << 15;

/// The footprint metrics a position is judged on.
pub fn footprint_metrics(pos: Position) -> &'static [Metric] {
    use Metric::*;
    use Position as P;
    match pos {
        P::QB => &[EpaPerDropback, Cpoe],
        P::RB => &[YacoPerCarry, ForcedMissedPerTouch, ExplosiveRunRate],
        P::WR | P::TE => &[SeparationPerTarget, YardsPerRouteRun],
        P::OT | P::IOL => &[PressureAllowedRate, RunBlockWinRate],
        P::EDGE | P::DT => &[PassRushWinRate, PressureRate],
        P::LB => &[StopRate, MissedTackleRate],
        P::CB | P::S => &[YardsPerCoverageSnap, BallsPlayedRate],
        _ => &[],
    }
}

/// The metrics a position's percentiles show first.
pub fn headline_metrics(pos: Position) -> &'static [Metric] {
    use Metric::*;
    use Position as P;
    match pos {
        P::QB => &[PasserRating, EpaPerDropback, Cpoe, YardsPerAttempt],
        P::RB => &[YardsPerCarry, YacoPerCarry, ForcedMissedPerTouch, ExplosiveRunRate],
        P::WR | P::TE => &[YardsPerRouteRun, SeparationPerTarget, DropRate],
        P::OT | P::IOL => &[PressureAllowedRate, RunBlockWinRate, GradePerPlay],
        P::EDGE | P::DT => &[PassRushWinRate, PressureRate, StopRate],
        P::LB => &[StopRate, MissedTackleRate, PressureRate],
        P::CB | P::S => &[YardsPerCoverageSnap, BallsPlayedRate, MissedTackleRate],
        P::K => &[FgPct],
        P::P => &[NetPunt],
        P::LS => &[GradePerPlay],
    }
}

/// Where a record book applies.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Scope {
    Division(Division),
    Conference(Division, String),
    /// A state's high schools: team records, and players' records from
    /// the seasons played on the match engine.
    State(usize),
}

/// Which list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Book {
    Season(usize),
    Career(usize),
    /// Consecutive games: 0 with a touchdown pass, 1 with 100 rushing
    /// yards, 2 with a catch.
    Streak(usize),
    /// A program's consecutive wins.
    WinStreak,
}

pub const STREAKS: [&str; 3] =
    ["straight games with a touchdown pass", "straight 100-yard rushing games", "straight games with a catch"];

impl Book {
    pub fn name(self) -> String {
        match self {
            Book::Season(k) => format!("single-season {}", RECORD_STATS[k].name()),
            Book::Career(k) => format!("career {}", RECORD_STATS[k].name()),
            Book::Streak(k) => STREAKS[k].to_string(),
            Book::WinStreak => "straight wins".to_string(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RecordEntry {
    pub value: f64,
    /// The player, or 0 for a program's streak.
    pub id: u32,
    pub name: String,
    pub program: usize,
    /// The season it was set (a career's or streak's last season).
    pub year: i32,
    /// A career's or streak's first season.
    pub from: i32,
    /// Still running (a streak, or a career still being played).
    pub live: bool,
}

/// What a player was honored with, kept for his record and the Hall of
/// Fame ballot.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Honors {
    pub name: String,
    pub pos: Option<Position>,
    /// First-team All-America seasons and the division.
    pub all_america: Vec<(i32, Division)>,
    pub all_conference: u32,
    #[serde(deserialize_with = "crate::save::leaked")]
    pub awards: Vec<(i32, &'static str)>,
    pub inducted: Option<i32>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HallEntry {
    pub year: i32,
    pub name: String,
    pub coach: bool,
    pub division: Division,
    pub programs: Vec<usize>,
    pub why: String,
    /// Who it is, for links (L2): a player's id, or a coach's index.
    #[serde(default)]
    pub who: Option<u32>,
}

/// A coach's unit numbers for the season (world item 5), and how the
/// buzz from them reads (offense or defense percentile in his division).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UnitBuzz {
    pub year: i32,
    pub coach: usize,
    pub program: usize,
    pub offense: bool,
    pub pct: f64,
    pub gain: f64,
    pub why: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct StatBook {
    /// Every player season, oldest first.
    pub seasons: Vec<SeasonRecord>,
    /// Player id to his seasons (indexes into `seasons`), oldest first.
    pub by_player: HashMap<u32, Vec<u32>>,
    /// Each season's first index in `seasons`.
    pub year_index: Vec<(i32, usize)>,
    /// (year, division, position) to each stat's per-game mean among the
    /// season's regulars: what the era-adjusted view divides by.
    #[serde(with = "crate::save::big_values")]
    pub norms: HashMap<(i32, Division, Position), [f32; STAT_COUNT]>,
    /// Record books: the best ten in a division, five in a conference.
    pub books: HashMap<(Scope, Book), Vec<RecordEntry>>,
    /// Records broken this season and earlier (year, division, book, holder, value).
    pub broken: Vec<(i32, Division, Book, String, f64)>,
    pub honors: HashMap<u32, Honors>,
    pub hall: Vec<HallEntry>,
    pub buzz: Vec<UnitBuzz>,
    /// Running streaks per player (games played in a row).
    pub(crate) streaks: HashMap<u32, [u16; 3]>,
    pub(crate) streak_from: HashMap<u32, [i32; 3]>,
    /// Streaks of five games or more ended this season, for the close:
    /// (division, kind, player, program, games, first season, last season).
    pub(crate) ended: Vec<(Division, usize, u32, usize, u32, i32, i32)>,
    /// Each program's running winning streak and the season it began.
    pub(crate) win_streak: Vec<(u32, i32)>,
    /// Each high school's.
    pub(crate) hs_streak: Vec<(u32, i32)>,
}

impl StatBook {
    /// A season's records.
    pub fn year(&self, year: i32) -> &[SeasonRecord] {
        let Some(k) = self.year_index.iter().position(|&(y, _)| y == year) else { return &[] };
        let start = self.year_index[k].1;
        let end = self.year_index.get(k + 1).map_or(self.seasons.len(), |x| x.1);
        &self.seasons[start..end]
    }

    /// A player's seasons, oldest first.
    pub fn career(&self, id: u32) -> Vec<&SeasonRecord> {
        self.by_player.get(&id).map(|v| v.iter().map(|&k| &self.seasons[k as usize]).collect()).unwrap_or_default()
    }

    /// His percentile (0-1, 1 the best) at a metric among his division's
    /// players at his position that season with enough volume.
    pub fn percentile(&self, r: &SeasonRecord, m: Metric) -> Option<f64> {
        let v = m.value(r)?;
        let peers: Vec<f64> = self
            .year(r.year as i32)
            .iter()
            .filter(|x| x.division == r.division && x.pos == r.pos)
            .filter_map(|x| m.value(x))
            .collect();
        Some(rank_pct(&peers, v, m.higher_better()))
    }

    /// The era-adjusted value (8.4): his number scaled by how the same
    /// stat ran per game for his division and position in the latest season
    /// against his own season.
    pub fn era_adjusted(&self, r: &SeasonRecord, k: Stat) -> f64 {
        let raw = r.get(k);
        let Some(&(last, _)) = self.year_index.last() else { return raw };
        let (Some(then), Some(now)) =
            (self.norms.get(&(r.year as i32, r.division, r.pos)), self.norms.get(&(last, r.division, r.pos)))
        else {
            return raw;
        };
        let (a, b) = (then[k as usize] as f64, now[k as usize] as f64);
        if a <= 0.0 || b <= 0.0 { raw } else { raw * b / a }
    }

    /// The opponent-adjusted value (8.4): a counting stat scaled by his
    /// schedule, 3% a rating point of opponent strength against his
    /// division's average (proposed, not confirmed).
    pub fn opponent_adjusted(r: &SeasonRecord, k: Stat) -> f64 {
        r.get(k) * clamp(1.0 + 0.03 * r.sos as f64, 0.7, 1.3)
    }

    /// His footprint flags in seasons from `since` on.
    pub fn footprint_since(&self, id: u32, since: i32) -> Vec<&SeasonRecord> {
        self.career(id).into_iter().filter(|r| r.year as i32 >= since && r.footprint != 0).collect()
    }
}

/// Share of `peers` he is better than (1 = the best).
fn rank_pct(peers: &[f64], v: f64, higher: bool) -> f64 {
    if peers.len() <= 1 {
        return 1.0;
    }
    let below = peers.iter().filter(|&&x| if higher { x < v } else { x > v }).count();
    below as f64 / (peers.len() - 1) as f64
}

/// What a footprint flag says, for notes.
pub fn footprint_text(r: &SeasonRecord) -> Vec<String> {
    let mut out = Vec::new();
    for (k, m) in FOOTPRINT.iter().enumerate() {
        if r.footprint & (1 << k) != 0 {
            out.push(format!("top 1% in {} among {} {}s ({})", m.name(), r.division.name(), r.pos, r.year));
        }
    }
    if r.footprint & TESTING != 0 {
        out.push(format!("a testing outlier among college {}s ({})", r.pos, r.year));
    }
    out
}

/// One in-season line for the book: his program, his listed position, his
/// summed game lines and his opponents' strength.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BookLine {
    pub program: usize,
    pub pos: Option<Position>,
    pub line: StatLine,
    pub games: u32,
    pub opp_strength: f64,
}

/// Sets the footprint flags of a season's lines (top 1% of his division
/// and position at a footprint metric, among at least 50 with enough
/// volume; proposed, not confirmed).
pub fn set_footprints(recs: &mut [SeasonRecord]) {
    let mut groups: HashMap<(Division, Position), Vec<usize>> = HashMap::new();
    for (k, r) in recs.iter().enumerate() {
        groups.entry((r.division, r.pos)).or_default().push(k);
    }
    for ((_, pos), idx) in groups {
        for &m in footprint_metrics(pos) {
            let bit = FOOTPRINT.iter().position(|&x| x == m).unwrap();
            let mut vals: Vec<(f64, usize)> = idx.iter().filter_map(|&k| m.value(&recs[k]).map(|v| (v, k))).collect();
            if vals.len() < 50 {
                continue;
            }
            if m.higher_better() {
                vals.sort_by(|a, b| b.0.total_cmp(&a.0));
            } else {
                vals.sort_by(|a, b| a.0.total_cmp(&b.0));
            }
            let top = ((vals.len() as f64 * 0.01).round() as usize).max(1);
            for &(_, k) in vals.iter().take(top) {
                recs[k].footprint |= 1 << bit;
            }
        }
    }
}

/// The season's records from the in-season book, footprints set.
pub(crate) fn season_records(w: &World, year: i32, book: &HashMap<u32, BookLine>) -> Vec<SeasonRecord> {
    let mut ids: Vec<&u32> = book.keys().collect();
    ids.sort();
    // Each division's average opponent strength.
    let mut div_sum = [(0.0, 0.0); super::data::DIVISION_COUNT];
    for l in ids.iter().map(|id| &book[*id]) {
        if l.games > 0 {
            let d = w.programs[l.program].division as usize;
            div_sum[d].0 += l.opp_strength / l.games as f64;
            div_sum[d].1 += 1.0;
        }
    }
    let mut recs: Vec<SeasonRecord> = ids
        .into_iter()
        .filter_map(|&id| {
            let l = &book[&id];
            let pos = l.pos?;
            if l.games == 0 {
                return None;
            }
            let division = w.programs[l.program].division;
            let (s, n) = div_sum[division as usize];
            let avg = if n > 0.0 { s / n } else { 0.0 };
            Some(SeasonRecord {
                id,
                year: year as i16,
                division,
                pos,
                program: l.program as u16,
                games: l.games.min(255) as u8,
                footprint: 0,
                sos: (l.opp_strength / l.games as f64 - avg) as f32,
                s: from_line(&l.line),
            })
        })
        .collect();
    set_footprints(&mut recs);
    recs
}

/// Testing outliers (world item 9): the top 0.2% of college players at
/// his position nationally in the forty, or in the jumps, on a preseason
/// testing day (Feldman's Freaks list; proposed, not confirmed).
fn set_testing(w: &World, recs: &mut [SeasonRecord]) {
    let by_id: HashMap<u32, usize> =
        w.people.iter().enumerate().filter(|(_, p)| p.stage == Stage::College).map(|(i, p)| (p.player.id, i)).collect();
    let seed = w.options.seed;
    let mut groups: HashMap<Position, Vec<(usize, f64, f64)>> = HashMap::new();
    for (k, r) in recs.iter().enumerate() {
        if r.pos.is_specialist() {
            continue;
        }
        let Some(&i) = by_id.get(&r.id) else { continue };
        let mut rng = crate::rng::Rng::new(seed.wrapping_mul(0x9E37).wrapping_add(r.id) ^ (r.year as u32) << 20);
        let c = crate::body::combine(&w.people[i].player, &mut rng);
        groups.entry(r.pos).or_default().push((k, c.forty, c.vertical + c.broad / 3.0));
    }
    for (_, mut v) in groups {
        let top = ((v.len() as f64 * 0.002).round() as usize).max(1);
        v.sort_by(|a, b| a.1.total_cmp(&b.1));
        for e in v.iter().take(top) {
            recs[e.0].footprint |= TESTING;
        }
        v.sort_by(|a, b| b.2.total_cmp(&a.2));
        for e in v.iter().take(top) {
            recs[e.0].footprint |= TESTING;
        }
    }
}

/// Footprint flags of the season still being played, by player id: what
/// pro scouts see in December.
pub(crate) fn current_footprints(w: &World) -> HashMap<u32, u16> {
    let Some(cs) = w.college.as_ref() else { return HashMap::new() };
    let recs = season_records(w, cs.year, &cs.book);
    recs.into_iter().filter(|r| r.footprint != 0).map(|r| (r.id, r.footprint)).collect()
}

/// The season is over: its lines go into the book for good, with norms,
/// record books, and the Hall of Fame vote that follows the season.
pub(crate) fn close_season(w: &mut World) {
    let Some(cs) = w.college.as_mut() else { return };
    if !cs.started || cs.book.is_empty() {
        return;
    }
    let year = cs.year;
    let book = std::mem::take(&mut cs.book);
    let mut recs = season_records(w, year, &book);
    set_testing(w, &mut recs);
    drop(book);
    norms(w, year, &recs);
    let start = w.stats.seasons.len();
    w.stats.year_index.push((year, start));
    for (k, r) in recs.into_iter().enumerate() {
        w.stats.by_player.entry(r.id).or_default().push((start + k) as u32);
        w.stats.seasons.push(r);
    }
    record_books(w, year, start);
    streak_books(w, year);
    if w.recording {
        hall_of_fame(w, year);
    }
}

/// Per-game means among the season's regulars (six games or more and 15
/// snaps a game), by division and position.
fn norms(w: &mut World, year: i32, recs: &[SeasonRecord]) {
    let mut sums: HashMap<(Division, Position), ([f64; STAT_COUNT], f64)> = HashMap::new();
    for r in recs {
        let snaps = r.get(Stat::SnapsOff) + r.get(Stat::SnapsDef) + r.get(Stat::SnapsSt);
        if r.games < 6 || snaps < 15.0 * r.games as f64 {
            continue;
        }
        let e = sums.entry((r.division, r.pos)).or_insert(([0.0; STAT_COUNT], 0.0));
        for k in 0..STAT_COUNT {
            e.0[k] += r.s[k] as f64 / r.games as f64;
        }
        e.1 += 1.0;
    }
    for ((d, pos), (s, n)) in sums {
        let mut m = [0f32; STAT_COUNT];
        for k in 0..STAT_COUNT {
            m[k] = (s[k] / n) as f32;
        }
        w.stats.norms.insert((year, d, pos), m);
    }
}

/// A player's name from the people still in the world, or the honors book.
fn name_of(w: &World, by_id: &HashMap<u32, usize>, id: u32) -> String {
    if let Some(&i) = by_id.get(&id) {
        return w.people[i].name.clone();
    }
    w.stats.honors.get(&id).map_or_else(|| "a former player".to_string(), |h| h.name.clone())
}

const DIVISION_TOP: usize = 10;
const CONFERENCE_TOP: usize = 5;

/// Puts an entry on a list if it makes the top `n`. Returns whether it
/// is now first, ahead of an earlier holder.
fn place(list: &mut Vec<RecordEntry>, e: RecordEntry, n: usize) -> bool {
    let was_best = list.first().map(|x| x.value);
    let pos = list.partition_point(|x| x.value >= e.value);
    if pos >= n {
        return false;
    }
    list.insert(pos, e);
    list.truncate(n);
    pos == 0 && was_best.is_some()
}

/// Puts an entry on a state's list (the best ten, like a division's).
pub(crate) fn place_entry(list: &mut Vec<RecordEntry>, e: RecordEntry) {
    place(list, e, DIVISION_TOP);
}

/// Puts a running streak or career on its list, in place of its own
/// earlier entry.
fn place_running(list: &mut Vec<RecordEntry>, e: RecordEntry, n: usize) {
    list.retain(|x| !(x.live && x.id == e.id && x.program == e.program && x.from == e.from));
    place(list, e, n);
}

/// The season's record books (8.7): single season and career per division
/// (a career counts his seasons in that division, as the NCAA's books do)
/// and single season per conference. A new division record is news once
/// the books have ten seasons behind them.
fn record_books(w: &mut World, year: i32, start: usize) {
    let by_id: HashMap<u32, usize> = w.people.iter().enumerate().map(|(i, p)| (p.player.id, i)).collect();
    let n = w.stats.seasons.len();
    let established = w.stats.year_index.len() > 10;
    let mut news: Vec<(Division, Book, String, f64, usize, u32)> = Vec::new();
    for k in start..n {
        let r = w.stats.seasons[k].clone();
        let prog = r.program as usize;
        let conf = w.programs[prog].conference.clone();
        let name = name_of(w, &by_id, r.id);
        let still = by_id.get(&r.id).is_some_and(|&i| w.people[i].stage == Stage::College);
        let (first, totals) = {
            let career: Vec<&SeasonRecord> =
                w.stats.career(r.id).into_iter().filter(|c| c.division == r.division).collect();
            let first = career.first().map_or(year, |s| s.year as i32);
            (first, RECORD_STATS.iter().map(|&s| career.iter().map(|c| c.get(s)).sum::<f64>()).collect::<Vec<f64>>())
        };
        for (j, &s) in RECORD_STATS.iter().enumerate() {
            let v = r.get(s);
            if v > 0.0 {
                let e = RecordEntry {
                    value: v,
                    id: r.id,
                    name: name.clone(),
                    program: prog,
                    year,
                    from: year,
                    live: false,
                };
                let list = w.stats.books.entry((Scope::Division(r.division), Book::Season(j))).or_default();
                if place(list, e.clone(), DIVISION_TOP) {
                    news.push((r.division, Book::Season(j), name.clone(), v, prog, r.id));
                }
                if !conf.is_empty() {
                    let key = (Scope::Conference(r.division, conf.clone()), Book::Season(j));
                    place(w.stats.books.entry(key).or_default(), e, CONFERENCE_TOP);
                }
            }
            if totals[j] > 0.0 {
                let e = RecordEntry {
                    value: totals[j],
                    id: r.id,
                    name: name.clone(),
                    program: prog,
                    year,
                    from: first,
                    live: still,
                };
                // A career entry follows him: the old one goes when it grows.
                let list = w.stats.books.entry((Scope::Division(r.division), Book::Career(j))).or_default();
                let held = list.first().is_some_and(|x| x.id == r.id);
                list.retain(|x| x.id != r.id);
                if place(list, e, DIVISION_TOP) && !held {
                    news.push((r.division, Book::Career(j), name.clone(), totals[j], prog, r.id));
                }
            }
        }
    }
    let date = w.today;
    for (d, b, name, v, prog, id) in news {
        w.stats.broken.push((year, d, b, name.clone(), v));
        if w.recording && established {
            let text =
                format!("{name} of {} sets the {} record for {}: {v:.0}.", w.program_name(prog), d.name(), b.name());
            let loud = if d == Division::FBS { Loudness::Big } else { Loudness::Medium };
            w.calendar.post(
                Event::new(date, CalLevel::College, Kind::News, loud, text)
                    .program(prog)
                    .person(id, &name)
                    .school_link(prog, w.program_name(prog)),
            );
        }
    }
}

/// A game's streaks (8.7): a player's run of games with a touchdown pass,
/// 100 rushing yards or a catch goes on while he keeps doing it and ends
/// the first game he plays without it. `players`: (id, program, played
/// the part, did it), sorted by id.
pub(crate) fn game_streaks(w: &mut World, year: i32, players: &[(u32, usize, [bool; 3], [bool; 3])]) {
    for &(id, prog, eligible, hit) in players {
        let division = w.programs[prog].division;
        let cur = w.stats.streaks.entry(id).or_insert([0; 3]);
        let from = w.stats.streak_from.entry(id).or_insert([year; 3]);
        for k in 0..3 {
            if !eligible[k] {
                continue;
            }
            if hit[k] {
                if cur[k] == 0 {
                    from[k] = year;
                }
                cur[k] += 1;
            } else if cur[k] > 0 {
                if cur[k] >= 5 {
                    w.stats.ended.push((division, k, id, prog, cur[k] as u32, from[k], year));
                }
                cur[k] = 0;
            }
        }
    }
}

/// At the season's close: ended streaks go on the books, running ones as
/// running, and a player whose college career is over ends his.
fn streak_books(w: &mut World, year: i32) {
    let by_id: HashMap<u32, usize> = w.people.iter().enumerate().map(|(i, p)| (p.player.id, i)).collect();
    let mut ended = std::mem::take(&mut w.stats.ended);
    let mut running: Vec<(Division, usize, u32, usize, u32, i32)> = Vec::new();
    let mut ids: Vec<u32> = w.stats.streaks.keys().copied().collect();
    ids.sort();
    // His last season's division and program.
    let last = |w: &World, id: u32| w.stats.career(id).last().map(|s| (s.division, s.program as usize));
    for id in ids {
        let still = by_id.get(&id).is_some_and(|&i| w.people[i].stage == Stage::College);
        let Some((d, prog)) = last(w, id) else { continue };
        let cur = w.stats.streaks[&id];
        let from = w.stats.streak_from[&id];
        for k in 0..3 {
            if cur[k] >= 5 {
                if still {
                    running.push((d, k, id, prog, cur[k] as u32, from[k]));
                } else {
                    ended.push((d, k, id, prog, cur[k] as u32, from[k], year));
                }
            }
        }
        if !still {
            w.stats.streaks.remove(&id);
            w.stats.streak_from.remove(&id);
        }
    }
    for (d, k, id, prog, len, from, y) in ended {
        let name = name_of(w, &by_id, id);
        let e = RecordEntry { value: len as f64, id, name, program: prog, year: y, from, live: false };
        place_running(w.stats.books.entry((Scope::Division(d), Book::Streak(k))).or_default(), e.clone(), DIVISION_TOP);
        // An ended streak takes the place of its running entry.
        let list = w.stats.books.get_mut(&(Scope::Division(d), Book::Streak(k))).unwrap();
        list.retain(|x| !(x.live && x.id == e.id && x.from == e.from));
    }
    for (d, k, id, prog, len, from) in running {
        let name = name_of(w, &by_id, id);
        let e = RecordEntry { value: len as f64, id, name, program: prog, year, from, live: true };
        place_running(w.stats.books.entry((Scope::Division(d), Book::Streak(k))).or_default(), e, DIVISION_TOP);
    }
    // High schools' running streaks.
    for school in 0..w.stats.hs_streak.len() {
        let (len, from) = w.stats.hs_streak[school];
        if len >= 15 {
            let h = &w.high_schools[school];
            let e = hs_entry(h, len, year, from, true);
            let list = w.stats.books.entry((Scope::State(h.state), Book::WinStreak)).or_default();
            list.retain(|x| !(x.live && x.id == e.id && x.from == from));
            place(list, e, DIVISION_TOP);
        }
    }
    // Programs' running winning streaks.
    for prog in 0..w.stats.win_streak.len() {
        let (len, from) = w.stats.win_streak[prog];
        if len >= 10 {
            let d = w.programs[prog].division;
            let e = RecordEntry {
                value: len as f64,
                id: 0,
                name: w.programs[prog].real_name.clone(),
                program: prog,
                year,
                from,
                live: true,
            };
            place_running(w.stats.books.entry((Scope::Division(d), Book::WinStreak)).or_default(), e, DIVISION_TOP);
        }
    }
}

/// A program's winning streak after a game.
pub(crate) fn program_streak(w: &mut World, year: i32, prog: usize, won: bool) {
    if w.stats.win_streak.len() < w.programs.len() {
        w.stats.win_streak.resize(w.programs.len(), (0, year));
    }
    let (len, from) = w.stats.win_streak[prog];
    if won {
        w.stats.win_streak[prog] = (len + 1, if len == 0 { year } else { from });
        return;
    }
    w.stats.win_streak[prog] = (0, year);
    if len >= 10 {
        let d = w.programs[prog].division;
        let e = RecordEntry {
            value: len as f64,
            id: 0,
            name: w.programs[prog].real_name.clone(),
            program: prog,
            year,
            from,
            live: false,
        };
        let list = w.stats.books.entry((Scope::Division(d), Book::WinStreak)).or_default();
        list.retain(|x| !(x.live && x.program == prog && x.from == from));
        place(list, e, DIVISION_TOP);
    }
}

/// A high school's winning streak after a game (the state's record book;
/// streaks of 15 or more are listed).
pub(crate) fn hs_result(w: &mut World, school: usize, won: bool) {
    let year = w.year;
    if w.stats.hs_streak.len() < w.high_schools.len() {
        w.stats.hs_streak.resize(w.high_schools.len(), (0, year));
    }
    let (len, from) = w.stats.hs_streak[school];
    if won {
        w.stats.hs_streak[school] = (len + 1, if len == 0 { year } else { from });
        return;
    }
    w.stats.hs_streak[school] = (0, year);
    if len >= 15 {
        let h = &w.high_schools[school];
        let e = hs_entry(h, len, year, from, false);
        let list = w.stats.books.entry((Scope::State(h.state), Book::WinStreak)).or_default();
        list.retain(|x| !(x.live && x.id == school as u32 && x.from == from));
        place(list, e, DIVISION_TOP);
    }
}

fn hs_entry(h: &super::high_school::HighSchool, len: u32, year: i32, from: i32, live: bool) -> RecordEntry {
    RecordEntry {
        value: len as f64,
        id: h.id as u32,
        name: format!("{} ({})", h.name, h.town),
        program: usize::MAX,
        year,
        from,
        live,
    }
}

/// Records an honor (the All-America and all-conference teams, the
/// season's awards).
pub(crate) fn honor(w: &mut World, i: usize, year: i32, all_america: Option<Division>, award: Option<&'static str>) {
    let p = &w.people[i];
    let h = w.stats.honors.entry(p.player.id).or_default();
    h.name = p.name.clone();
    h.pos = Some(p.player.position);
    match (all_america, award) {
        (Some(d), _) => h.all_america.push((year, d)),
        (None, Some(a)) => h.awards.push((year, a)),
        (None, None) => h.all_conference += 1,
    }
}

/// The College Football Hall of Fame vote, once a season is over (the
/// National Football Foundation's rules): a first-team All-American, ten
/// full seasons after his last college season and within 50. About a
/// dozen from the top level a year and four from the divisions below; two
/// coaches, retired three years, with 100 games as a head coach and a
/// .600 record (proposed, not confirmed: the ballot score, the counts).
fn hall_of_fame(w: &mut World, year: i32) {
    let scale = w.options.scale;
    let mut fbs: Vec<(f64, u32, Division, Vec<usize>, String)> = Vec::new();
    let mut lower: Vec<(f64, u32, Division, Vec<usize>, String)> = Vec::new();
    for (&id, h) in &w.stats.honors {
        if h.inducted.is_some() || h.all_america.is_empty() {
            continue;
        }
        let seasons = w.stats.career(id);
        let Some(last) = seasons.last() else { continue };
        let last_year = last.year as i32;
        if last_year > year - 10 || last_year < year - 50 {
            continue;
        }
        // His ballot: All-America seasons, the big awards, his career's film.
        let aa = h.all_america.len() as f64;
        let awards: f64 = h.awards.iter().map(|(_, a)| if *a == "Heisman Trophy" { 5.0 } else { 2.0 }).sum::<f64>();
        let graded: f64 = seasons.iter().map(|s| s.get(Stat::Graded)).sum();
        let grade =
            if graded > 0.0 { seasons.iter().map(|s| s.get(Stat::GradeSum)).sum::<f64>() / graded } else { 0.0 };
        let score = 3.0 * aa
            + awards
            + 0.4 * h.all_conference as f64
            + 8.0 * grade
            + 1.5 * super::season::key_normal(0x4A11 ^ year as u64, id as u64);
        let d = h.all_america.iter().map(|x| x.1).min().unwrap_or(Division::FBS);
        let mut programs: Vec<usize> = Vec::new();
        for s in &seasons {
            if !programs.contains(&(s.program as usize)) {
                programs.push(s.program as usize);
            }
        }
        let why = format!(
            "{} first-team All-America season{}{}, last played {last_year}",
            h.all_america.len(),
            if h.all_america.len() == 1 { "" } else { "s" },
            if h.awards.is_empty() {
                String::new()
            } else {
                format!(", {}", h.awards.iter().map(|a| format!("{} {}", a.1, a.0)).collect::<Vec<_>>().join(", "))
            }
        );
        if d == Division::FBS {
            fbs.push((score, id, d, programs, why))
        } else {
            lower.push((score, id, d, programs, why))
        }
    }
    let mut inducted: Vec<HallEntry> = Vec::new();
    for (mut list, n) in
        [(fbs, (12.0 * scale).round().max(1.0) as usize), (lower, (4.0 * scale).round().max(1.0) as usize)]
    {
        list.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        for (_, id, d, programs, why) in list.into_iter().take(n) {
            let h = w.stats.honors.get_mut(&id).unwrap();
            h.inducted = Some(year);
            inducted.push(HallEntry {
                year,
                name: h.name.clone(),
                coach: false,
                division: d,
                programs,
                why,
                who: Some(id),
            });
        }
    }
    // Coaches: retired three seasons, 100 games and a .600 record as a
    // college head coach.
    let inducted_coaches: Vec<String> = w.stats.hall.iter().filter(|e| e.coach).map(|e| e.name.clone()).collect();
    let mut coaches: Vec<(f64, usize, Division, Vec<usize>, String)> = Vec::new();
    for (c, coach) in w.coaches.iter().enumerate() {
        if !coach.retired || inducted_coaches.contains(&coach.name) {
            continue;
        }
        let mut wins = 0;
        let mut losses = 0;
        let mut last = 0;
        let mut titles = 0;
        let mut programs = Vec::new();
        let mut top = Division::JUCO;
        for s in &coach.history {
            if let (super::coach::Employer::College(p), super::coach::Role::HeadCoach) = (s.employer, s.role) {
                wins += s.wins;
                losses += s.losses;
                titles += s.titles;
                last = last.max(s.to);
                if !programs.contains(&p) {
                    programs.push(p);
                }
                top = top.min(w.programs[p].division);
            }
        }
        let games = wins + losses;
        if games < 100 || (wins as f64) < 0.6 * games as f64 || last > year - 3 || last < year - 50 {
            continue;
        }
        let score = wins as f64 / games as f64 * 10.0 + titles as f64 + games as f64 / 100.0;
        let why = format!("{wins}-{losses} as a college head coach, {}", super::news::count(titles, "title"));
        coaches.push((score, c, top, programs, why));
    }
    coaches.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let n_coach = (2.0 * scale).round().max(1.0) as usize;
    for (_, c, d, programs, why) in coaches.into_iter().take(n_coach) {
        inducted.push(HallEntry {
            year,
            name: w.coaches[c].name.clone(),
            coach: true,
            division: d,
            programs,
            why,
            who: Some(c as u32),
        });
    }
    if inducted.is_empty() {
        return;
    }
    let names: Vec<String> = inducted.iter().take(6).map(|e| e.name.clone()).collect();
    let text = format!(
        "The College Football Hall of Fame announces its class: {} inductees, among them {}.",
        inducted.len(),
        names.join(", ")
    );
    let mut e = Event::new(w.today, CalLevel::College, Kind::Award, Loudness::Medium, text);
    for h in inducted.iter().take(6) {
        if let Some(who) = h.who {
            let to = if h.coach { crate::calendar::Link::Coach(who) } else { crate::calendar::Link::Person(who) };
            e = e.link(to, &h.name);
        }
    }
    w.calendar.post(e);
    w.stats.hall.extend(inducted);
}

/// Coach buzz from unit stats (world item 5): an offense or defense near
/// the top of its division in EPA a play makes its coordinator a name, and
/// the best unit in a conference adds to it. The buzz fades over two to
/// three seasons (halves each year). Proposed, not confirmed: +3 for the
/// top 1% of the division, +2 top 5%, +1.25 top 10%, +0.5 top quarter,
/// -1 bottom tenth, +0.75 for the conference's best.
pub(crate) fn unit_buzz(w: &mut World, year: i32, results: &[super::season::TeamResult]) {
    for c in w.coaches.iter_mut() {
        c.stat_buzz *= 0.5;
    }
    // EPA a play by division, offense and defense.
    let mut by_div: HashMap<Division, Vec<(usize, f64, f64)>> = HashMap::new();
    for (p, t) in results.iter().enumerate() {
        let tot = &t.totals;
        if tot.plays < 200 || tot.plays_allowed < 200 {
            continue;
        }
        let off = tot.epa / tot.plays as f64;
        let def = tot.epa_allowed / tot.plays_allowed as f64;
        by_div.entry(w.programs[p].division).or_default().push((p, off, def));
    }
    let gain = |pct: f64| {
        if pct >= 0.99 {
            3.0
        } else if pct >= 0.95 {
            2.0
        } else if pct >= 0.9 {
            1.25
        } else if pct >= 0.75 {
            0.5
        } else if pct <= 0.1 {
            -1.0
        } else {
            0.0
        }
    };
    let mut out: Vec<UnitBuzz> = Vec::new();
    for (d, list) in by_div {
        let offs: Vec<f64> = list.iter().map(|x| x.1).collect();
        let defs: Vec<f64> = list.iter().map(|x| x.2).collect();
        // The conference's best units.
        let mut conf_best: HashMap<&str, (usize, f64, usize, f64)> = HashMap::new();
        for &(p, o, de) in &list {
            let e = conf_best.entry(w.programs[p].conference.as_str()).or_insert((p, o, p, de));
            if o > e.1 {
                e.0 = p;
                e.1 = o;
            }
            if de < e.3 {
                e.2 = p;
                e.3 = de;
            }
        }
        let conf_size: HashMap<&str, usize> = list.iter().fold(HashMap::new(), |mut m, x| {
            *m.entry(w.programs[x.0].conference.as_str()).or_insert(0) += 1;
            m
        });
        for &(p, o, de) in &list {
            let prog = &w.programs[p];
            let conf = prog.conference.as_str();
            let big_conf = !conf.is_empty() && conf_size.get(conf).copied().unwrap_or(0) >= 6;
            for (offense, value, peers, coord) in [(true, o, &offs, prog.oc), (false, de, &defs, prog.dc)] {
                let pct = rank_pct(peers, value, offense);
                let best = big_conf && conf_best.get(conf).is_some_and(|b| if offense { b.0 == p } else { b.2 == p });
                let g = gain(pct) + if best { 0.75 } else { 0.0 };
                if g == 0.0 {
                    continue;
                }
                let side = if offense { "offense" } else { "defense" };
                let why = format!(
                    "his {side} was {} in {} in EPA a play{}",
                    if pct >= 0.5 {
                        format!("in the top {:.0}%", (100.0 * (1.0 - pct)).max(1.0))
                    } else {
                        format!("in the bottom {:.0}%", (100.0 * pct).max(1.0))
                    },
                    d.name(),
                    if best { ", the best in his conference" } else { "" }
                );
                let head = prog.sideline.or(prog.head_coach);
                for (c, share) in [(coord, 1.0), (head, 0.5)] {
                    let Some(c) = c else { continue };
                    out.push(UnitBuzz { year, coach: c, program: p, offense, pct, gain: share * g, why: why.clone() });
                }
            }
        }
    }
    out.sort_by(|a, b| a.coach.cmp(&b.coach).then(a.program.cmp(&b.program)).then(b.offense.cmp(&a.offense)));
    for b in &out {
        w.coaches[b.coach].stat_buzz += b.gain;
    }
    if w.recording {
        w.stats.buzz.extend(out);
    }
}
