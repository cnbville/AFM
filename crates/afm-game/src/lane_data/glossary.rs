//! Every stat explains itself (Cain, 2026-11-03: "when i hover over a
//! statname it explains the stat in plain words"; item 3 of the Data
//! overhaul). One plain-words entry per stat the game shows, with how the
//! game works it out and which way is better, and the table headers that
//! name it. The screens ask for the whole list once (`stat_glossary`) and
//! mark every header and name they find: by key where the page knows the
//! stat (Player analytics, Leaderboards, Stat history), else by the
//! header's text under its table's first header (a box score's "Yds"
//! under "Rushing" is rushing yards).
//!
//! Words follow real football's own definitions (PFF, Next Gen Stats, ESPN,
//! the NFL and NCAA stat books) and say where the game counts something its
//! own way.

use afm_engine::world::analytics::Cm;
use serde_json::{Value, json};

/// One stat: key, name, what it is in plain words, how the game works it
/// out (empty when the plain words already say it), and which way is
/// better (1 higher, -1 lower, 0 neither).
pub struct Term {
    pub key: &'static str,
    pub name: &'static str,
    pub plain: &'static str,
    pub how: &'static str,
    pub better: i8,
}

const fn t(key: &'static str, name: &'static str, plain: &'static str, how: &'static str, better: i8) -> Term {
    Term { key, name, plain, how, better }
}

pub const TERMS: &[Term] = &[
    // ---- Playing time ----
    t("Games", "Games", "Games he played in.", "", 0),
    t("Starts", "Games started", "Games he started.", "", 0),
    t("Snaps", "Snaps", "Plays he was on the field for.", "", 0),
    t(
        "Vol",
        "Volume",
        "How much he played in the one number his position group is judged on: dropbacks for a quarterback, carries for a back, routes for a receiver, blocking or rushing snaps up front, coverage snaps in the secondary, kicks and punts for specialists. A season needs at least the page's minimum to qualify.",
        "",
        0,
    ),
    // ---- Passing ----
    t("Cmp", "Completions", "Passes caught by his own receiver.", "", 1),
    t(
        "Att",
        "Pass attempts",
        "Every pass he threw, caught or not, throwaways included. Sacks and scrambles are not attempts.",
        "",
        0,
    ),
    t(
        "CmpAtt",
        "Completions and attempts",
        "Passes caught out of passes thrown: 22/31 is 22 caught from 31 thrown.",
        "",
        0,
    ),
    t("Pct", "Completion percentage", "The share of his passes that were caught.", "Completions ÷ attempts.", 1),
    t(
        "PassYds",
        "Passing yards",
        "Yards gained on his completed passes, the run after the catch included. Sacks don't come off a passer's own total.",
        "",
        1,
    ),
    t(
        "PassYdsG",
        "Passing yards a game",
        "His passing yards spread over the games he played.",
        "Passing yards ÷ games.",
        1,
    ),
    t(
        "Ypa",
        "Yards an attempt",
        "How far the offense moved for every pass he threw. Incompletions count as zero, so it rewards being accurate and throwing downfield at once.",
        "Passing yards ÷ attempts.",
        1,
    ),
    t("PassTd", "Touchdown passes", "Passes that ended in a touchdown.", "", 1),
    t("TdPct", "Touchdown rate", "How often a pass of his went for a touchdown.", "Touchdown passes ÷ attempts.", 1),
    t("Int", "Interceptions thrown", "His passes the defense caught.", "", -1),
    t("IntPct", "Interception rate", "How often a pass of his was picked off.", "Interceptions ÷ attempts.", -1),
    t("Sacked", "Times sacked", "Times he was tackled behind the line before he could throw.", "", -1),
    t(
        "SackRate",
        "Sack rate",
        "How often a dropback of his ended in a sack. Partly his line, partly him: holding the ball too long shows here.",
        "Sacks ÷ dropbacks.",
        -1,
    ),
    t(
        "PressRate",
        "Pressured rate",
        "How often a rusher got to him, or close enough to hurry him, before he threw. Mostly his line's doing, partly how long he holds the ball.",
        "Pressured dropbacks ÷ dropbacks.",
        -1,
    ),
    t(
        "PressSack",
        "Pressure to sack rate",
        "When he is pressured, how often it becomes a sack. Good passers slip the rush or get rid of the ball.",
        "Sacks ÷ pressured dropbacks.",
        -1,
    ),
    t(
        "Rating",
        "Passer rating",
        "The NFL's old one-number grade for a passer, from 0 to 158.3, built from completion percentage, yards an attempt, touchdown rate and interception rate. The game uses the NFL's formula at every level.",
        "The four parts, each scored 0 to 2.375, are added, divided by 6 and multiplied by 100.",
        1,
    ),
    t(
        "Efficiency",
        "Passing efficiency",
        "College football's passer rating, the NCAA's own formula. About 135 to 140 is an average FBS season lately, and 160 or more an excellent one.",
        "(8.4 × yards + 330 × touchdowns + 100 × completions − 200 × interceptions) ÷ attempts.",
        1,
    ),
    t(
        "EpaDb",
        "EPA a dropback",
        "Expected points added. Every snap starts with a value: how many points teams usually go on to score from that down, distance and spot. EPA is how much a play raised or lowered it. This is his average over every dropback. Above zero means his dropbacks moved his team closer to points.",
        "EPA on his throws, sacks and scrambles ÷ dropbacks.",
        1,
    ),
    t(
        "Cpoe",
        "Completion % over expected",
        "How many more of his passes were caught than an average passer would complete on the same throws: the same depth, the same tight or open receiver, the same pressure. +3 is three more completions every 100 throws. It shows accuracy that plain completion percentage hides.",
        "Completions minus expected completions, over attempts. Each level's own model, fitted on its own throws, gives the expected chance of each throw.",
        1,
    ),
    t(
        "Adot",
        "Average depth of target",
        "How far downfield his passes travel in the air before anyone catches them, on average. High is a deep passer, low a short, quick one: it describes his style more than his quality.",
        "Air yards ÷ attempts.",
        0,
    ),
    t(
        "AirYds",
        "Air yards",
        "How far his passes travelled past the line of scrimmage in the air, caught or not, before any run after the catch.",
        "",
        0,
    ),
    t(
        "BigTime",
        "Big-time throws",
        "His best throws: a completion 15 or more yards downfield into a tight window. PFF's idea; the game reads it off the throw itself.",
        "A completion with 15+ air yards to a receiver less than 1.5 yards clear of his man.",
        1,
    ),
    t(
        "Btt",
        "Big-time throw rate",
        "How often a dropback of his produced a big-time throw: a completion 15 or more yards downfield into a tight window.",
        "Big-time throws ÷ dropbacks.",
        1,
    ),
    t(
        "TurnoverWorthy",
        "Turnover-worthy plays",
        "Throws that were picked off, or should have been. PFF's idea.",
        "Interceptions, plus passes a defender got his hands on with the receiver less than 0.8 yards clear.",
        -1,
    ),
    t(
        "Twp",
        "Turnover-worthy play rate",
        "How often a dropback of his gave the defense a real chance at the ball.",
        "Turnover-worthy plays ÷ dropbacks.",
        -1,
    ),
    t("Clean", "Kept clean", "His completions and attempts on throws where no rusher got to him.", "", 1),
    t("Pressured", "Under pressure", "His completions and attempts on throws where a rusher got to him first.", "", 1),
    // ---- Running ----
    t("Rush", "Carries", "Running plays where he had the ball, a quarterback's scrambles included.", "", 0),
    t("RushYds", "Rushing yards", "Yards gained on his carries, less any lost.", "", 1),
    t(
        "RushYdsG",
        "Rushing yards a game",
        "His rushing yards spread over the games he played.",
        "Rushing yards ÷ games.",
        1,
    ),
    t("Ypc", "Yards a carry", "How far he got on an average carry.", "Rushing yards ÷ carries.", 1),
    t("RushTd", "Rushing touchdowns", "Carries that ended in the end zone.", "", 1),
    t("RushLong", "Longest run", "His longest carry.", "", 1),
    t(
        "Ybc",
        "Yards before contact",
        "How far he got before the first defender touched him, on an average carry. Mostly the blocking's doing.",
        "",
        1,
    ),
    t(
        "Yaco",
        "Yards after contact",
        "Yards he gained after the first defender touched him. Shows the back's own power and balance more than the blocking does.",
        "",
        1,
    ),
    t(
        "YacoC",
        "Yards after contact a carry",
        "How far he kept going after the first defender touched him, on an average carry. The back's own power and balance, more than his blocking.",
        "Yards after contact ÷ carries.",
        1,
    ),
    t("ForcedMissed", "Missed tackles forced", "Times a defender got to him and failed to bring him down.", "", 1),
    t(
        "Mtf",
        "Missed tackles forced a touch",
        "How often he made a tackler miss, for every time he had the ball.",
        "Missed tackles forced ÷ (carries + catches).",
        1,
    ),
    t("ExplosiveRuns", "Explosive runs", "Carries that went 10 yards or more.", "", 1),
    t(
        "Explosive",
        "Explosive run rate",
        "How often a carry of his went 10 yards or more.",
        "Runs of 10+ yards ÷ carries.",
        1,
    ),
    t(
        "Success",
        "Run success rate",
        "How often a carry left his team better placed to score than before the snap. Two yards on 3rd and 1 is a success; five yards on 3rd and 8 isn't.",
        "Carries with positive EPA ÷ carries (nflfastR's measure).",
        1,
    ),
    t("FumblesLost", "Fumbles lost", "Times he dropped the ball and the defense recovered it.", "", -1),
    t(
        "FumRate",
        "Fumbles lost a 100 touches",
        "How often he gave the ball away, for every 100 times he had it.",
        "100 × fumbles lost ÷ (carries + catches).",
        -1,
    ),
    // ---- Catching ----
    t("Targets", "Targets", "Passes thrown his way, caught or not.", "", 0),
    t("Rec", "Catches", "Passes he caught.", "", 1),
    t("RecG", "Catches a game", "His catches spread over the games he played.", "Catches ÷ games.", 1),
    t("RecYds", "Receiving yards", "Yards gained on his catches, the run after the catch included.", "", 1),
    t(
        "RecYdsG",
        "Receiving yards a game",
        "His receiving yards spread over the games he played.",
        "Receiving yards ÷ games.",
        1,
    ),
    t("Ypr", "Yards a catch", "How far the offense moved on an average catch of his.", "Receiving yards ÷ catches.", 1),
    t("RecTd", "Touchdown catches", "Catches that ended in a touchdown.", "", 1),
    t("RecLong", "Longest catch", "His longest catch, the run after it included.", "", 1),
    t("Drops", "Drops", "Catchable passes he let fall.", "", -1),
    t("Drop", "Drop rate", "How often a pass thrown his way was dropped.", "Drops ÷ targets.", -1),
    t(
        "CatchRate",
        "Catch rate",
        "The share of passes thrown his way that he caught. Depth matters: deep targets are caught less often.",
        "Catches ÷ targets.",
        1,
    ),
    t("Yac", "Yards after the catch", "Yards he ran with the ball after catching it.", "", 1),
    t(
        "YacR",
        "Yards after the catch a catch",
        "How far he ran after an average catch.",
        "Yards after the catch ÷ catches.",
        1,
    ),
    t(
        "TgtAir",
        "Air yards on targets",
        "How far downfield the passes thrown his way travelled in the air, before any run after the catch.",
        "",
        0,
    ),
    t(
        "Sep",
        "Separation",
        "How many yards clear of the nearest defender he was when the ball arrived, on average. Next Gen Stats' measure of getting open.",
        "Yards to the nearest defender at the catch point, averaged over his targets.",
        1,
    ),
    t(
        "Yprr",
        "Yards a route run",
        "Receiving yards for every route he ran, whether the ball came his way or not. The best single number for how much a receiver gets open and produces.",
        "Receiving yards ÷ routes run.",
        1,
    ),
    t(
        "TgtRoute",
        "Targets a route run",
        "How often the quarterback looked his way when he ran a route.",
        "Targets ÷ routes run.",
        1,
    ),
    // ---- Blocking ----
    t("PbSnaps", "Pass-block snaps", "Dropbacks where he blocked to protect the passer.", "", 0),
    t(
        "PressuresAllowed",
        "Pressures allowed",
        "Times the man he blocked got to the quarterback: a hurry, a hit or a sack.",
        "",
        -1,
    ),
    t(
        "PressAllowed",
        "Pressure allowed rate",
        "How often the man he blocked got to the quarterback, for every pass-block snap.",
        "Pressures allowed ÷ pass-block snaps.",
        -1,
    ),
    t("SacksAllowed", "Sacks allowed", "Sacks made by the man he was blocking.", "", -1),
    t(
        "SackAllowed",
        "Sack allowed rate",
        "How often the man he blocked sacked the quarterback, for every pass-block snap.",
        "Sacks allowed ÷ pass-block snaps.",
        -1,
    ),
    t(
        "RbWin",
        "Run-block win rate",
        "How often he beat the man he blocked on a running play.",
        "Run blocks won ÷ run blocks.",
        1,
    ),
    // ---- Rushing the passer, tackling ----
    t(
        "PrWin",
        "Pass-rush win rate",
        "How often he beat his blocker within 2.5 seconds of the snap: ESPN's measure, the time a quarterback usually needs.",
        "Rushes won inside 2.5 s ÷ pass-rush snaps.",
        1,
    ),
    t("Pressures", "Pressures", "Times he got to the quarterback: a hurry, a hit or a sack.", "", 1),
    t(
        "Pressure",
        "Pressure rate",
        "How often he got to the quarterback, for every pass rush.",
        "Pressures ÷ pass-rush snaps.",
        1,
    ),
    t(
        "Sacks",
        "Sacks",
        "Tackling the quarterback behind the line before he throws. A shared sack is half to each man.",
        "",
        1,
    ),
    t("SackPerRush", "Sack rate", "How often a pass rush of his ended in a sack.", "Sacks ÷ pass-rush snaps.", 1),
    t(
        "Tackles",
        "Tackles",
        "Plays where he brought the ball carrier down, alone or as the first man in; a second man in gets an assist.",
        "",
        1,
    ),
    t(
        "TklTotal",
        "Tackles",
        "Every tackle he was in on, alone or helping (solo tackles plus assists), as box scores count them.",
        "",
        1,
    ),
    t("TklG", "Tackles a game", "His tackles spread over the games he played.", "Tackles ÷ games.", 1),
    t(
        "Tfl",
        "Tackles for loss",
        "Tackles behind the line of scrimmage, sacks included. A shared one is half to each man.",
        "",
        1,
    ),
    t(
        "Stops",
        "Stops",
        "Tackles that ended a play as a failure for the offense: the play cost it expected points. PFF's measure.",
        "",
        1,
    ),
    t(
        "Stop",
        "Stop rate",
        "How often he made a stop, for every snap he played on defense.",
        "Stops ÷ defensive snaps.",
        1,
    ),
    t("Missed", "Missed tackles", "Times he got to the ball carrier and failed to bring him down.", "", -1),
    t(
        "Miss",
        "Missed tackle rate",
        "How often he missed when he had a chance to make a tackle.",
        "Missed tackles ÷ (tackles + missed tackles).",
        -1,
    ),
    t("ForcedFumbles", "Forced fumbles", "Times he knocked the ball loose.", "", 1),
    // ---- Coverage ----
    t("Ints", "Interceptions", "Passes he caught for his own team.", "", 1),
    t("Pbus", "Passes broken up", "Passes he knocked away or tipped without catching them.", "", 1),
    t(
        "Pd",
        "Passes defended",
        "Passes he stopped from being caught: interceptions plus passes broken up, as the NFL and NCAA count them.",
        "",
        1,
    ),
    t(
        "BallsPlayed",
        "Passes defended a target",
        "How often he got a hand on a pass thrown at the man he covered.",
        "(Interceptions + passes broken up) ÷ targets allowed.",
        1,
    ),
    t("TgtAllowed", "Targets allowed", "Passes thrown at the man he was covering.", "", -1),
    t("YdsAllowed", "Yards allowed in coverage", "Yards gained on passes thrown at the man he was covering.", "", -1),
    t(
        "TgtCov",
        "Targets a coverage snap",
        "How often a quarterback threw at the man he covered. Low can mean tight coverage, or that passers stay away from him.",
        "Targets allowed ÷ coverage snaps.",
        -1,
    ),
    t(
        "YdsCov",
        "Yards allowed a coverage snap",
        "Yards given up in coverage for every snap he dropped into it. Counts the snaps when nobody threw at him, so it rewards coverage that keeps the ball away.",
        "Yards allowed ÷ coverage snaps.",
        -1,
    ),
    // ---- Kicking and punting ----
    t("FgMade", "Field goals made", "Field goals he kicked through.", "", 1),
    t("FgAtt", "Field goal attempts", "Field goals he tried.", "", 0),
    t("FgLine", "Field goals", "Field goals made out of those tried: 2/3 is two of three.", "", 1),
    t(
        "FgPct",
        "Field goal percentage",
        "The share of his field goals that went through. Distance matters: a kicker sent out from 50 misses more.",
        "Field goals made ÷ tried.",
        1,
    ),
    t("Fg50Made", "Field goals of 50+", "Field goals of 50 yards or more that he made.", "", 1),
    t("FgLong", "Longest field goal", "His longest made field goal.", "", 1),
    t("XpMade", "Extra points", "Kicks after a touchdown that went through.", "", 1),
    t("XpLine", "Extra points", "Extra points made out of those tried.", "", 1),
    t("Punts", "Punts", "Times he punted.", "", 0),
    t(
        "PuntAvg",
        "Net punting average",
        "How far each punt moved the other team back once the return is taken off. A touchback counts 20 yards less, since the ball comes out to the 20.",
        "(Punt yards − return yards − 20 a touchback) ÷ punts.",
        1,
    ),
    t(
        "PuntGross",
        "Punting average",
        "How far his punts travelled, on average, before any return.",
        "Punt yards ÷ punts.",
        1,
    ),
    t("PuntIn20", "Inside the 20", "Punts that left the other team starting inside its own 20-yard line.", "", 1),
    // ---- Returns ----
    t("Returns", "Returns", "Kickoffs and punts he ran back.", "", 0),
    t("ReturnYds", "Return yards", "Yards he gained running back kickoffs and punts.", "", 1),
    t("ReturnTd", "Return touchdowns", "Kickoffs and punts he ran back for a touchdown.", "", 1),
    t("KickReturns", "Kick returns", "Kickoffs he ran back.", "", 0),
    t("PuntReturns", "Punt returns", "Punts he ran back.", "", 0),
    t("RetYdsBox", "Return yards", "Yards gained on the returns in the column to its left.", "", 1),
    t("RetLong", "Longest return", "His longest kickoff or punt return.", "", 1),
    // ---- The grade ----
    t(
        "Grade",
        "Grade",
        "The game's own 0-100 grade from charting every snap, the way PFF grades: each play earns him a small plus or minus for what he did (a sack, a pressure given up, a missed tackle, a big throw). A player whose plays add up to nothing good or bad sits at 60; it takes a lot of good snaps to climb toward 90. With few snaps the grade stays near 60.",
        "60 + 95 × his average play score, pulled toward 60 when he has played few snaps; between 20 and 99.9.",
        1,
    ),
    // ---- A team's game ----
    t("TeamYards", "Total yards", "Yards the offense gained on every play from scrimmage, passing and running.", "", 1),
    t("TeamPass", "Net passing yards", "Passing yards less the yards lost on sacks.", "", 1),
    t("TeamRush", "Rushing yards", "Yards gained on every running play, scrambles included.", "", 1),
    t("FirstDowns", "First downs", "Times the offense earned a new set of downs.", "", 1),
    t(
        "ThirdDowns",
        "Third downs",
        "Third downs turned into a first down or a touchdown, out of those tried: 5-12 is five of twelve.",
        "",
        1,
    ),
    t(
        "FourthDowns",
        "Fourth downs",
        "Fourth downs turned into a first down or a touchdown, out of those tried.",
        "",
        1,
    ),
    t("TeamSacks", "Sacks", "Sacks the defense made.", "", 1),
    t("Turnovers", "Turnovers", "Times it gave the ball away: interceptions thrown and fumbles lost.", "", -1),
    t(
        "Penalties",
        "Penalties",
        "Penalties called against it and the yards they cost: 6-45 is six for 45 yards.",
        "",
        -1,
    ),
    t("Possession", "Time of possession", "How long it had the ball.", "", 0),
    t("Plays", "Plays", "Offensive plays it ran: runs, passes, sacks and scrambles.", "", 0),
    // ---- Personnel and packages ----
    t("PkgSnaps", "Snaps", "Plays run with this grouping on the field.", "", 0),
    t("Share", "Share", "The share of all its snaps that used this grouping.", "", 0),
    t("PkgRun", "Runs", "Running plays from this grouping.", "", 0),
    t("PkgPass", "Passes", "Dropbacks from this grouping.", "", 0),
    t("YdsPlay", "Yards a play", "Yards gained on an average play from this grouping.", "", 1),
    t(
        "EpaPlay",
        "EPA a play",
        "Expected points added on an average play from this grouping: above zero helped the offense toward points.",
        "",
        1,
    ),
    // ---- A player's splits ----
    t(
        "SplitYds",
        "Yards a game",
        "His passing, rushing and receiving yards added up, spread over the games in this split.",
        "",
        1,
    ),
    t("SplitTd", "Touchdowns", "Touchdowns he threw, ran in or caught in the games in this split.", "", 1),
    t(
        "SplitTkl",
        "Tackles a game",
        "Every tackle he was in on, alone or helping, spread over the games in this split.",
        "",
        1,
    ),
    // ---- Measurables ----
    t(
        "Forty",
        "40-yard dash",
        "Seconds to run 40 yards from a standing start, the combine's test of straight-line speed.",
        "",
        -1,
    ),
    t("Bench", "Bench press", "Times he pressed 225 pounds, the combine's strength test.", "", 1),
    t("Vertical", "Vertical jump", "How high he jumped from a standstill, in inches.", "", 1),
    t("Broad", "Broad jump", "How far he jumped forward from a standstill. Shows lower-body explosion.", "", 1),
    t(
        "Cone",
        "3-cone drill",
        "Seconds to run around three cones in an L. Shows how sharply he changes direction.",
        "",
        -1,
    ),
    t(
        "Shuttle",
        "Short shuttle",
        "Seconds to run 5 yards, 10 back and 5 again (the 20-yard shuttle). Shows quickness in a short space.",
        "",
        -1,
    ),
    t(
        "Arm",
        "Arm length",
        "From shoulder to fingertip, in inches. Long arms keep blockers and receivers off him.",
        "",
        1,
    ),
    t(
        "Hand",
        "Hand size",
        "From thumb tip to little-finger tip with the hand spread, in inches. Big hands hold the ball better.",
        "",
        1,
    ),
];

/// Table headers that name a stat: (the table's first header in lower
/// case, or "" for any table; the header; the stat). A box score's "Yds"
/// means different stats under "Passing" and "Rushing"; headers under ""
/// mean one stat in any table.
pub const HEADS: &[(&str, &str, &str)] = &[
    ("", "G", "Games"),
    ("", "GS", "Starts"),
    ("", "Cmp", "Cmp"),
    ("", "Att", "Att"),
    ("", "C/A", "CmpAtt"),
    ("", "Car", "Rush"),
    ("", "Tgt", "Targets"),
    ("", "TFL", "Tfl"),
    ("", "PBU", "Pbus"),
    ("", "aDOT", "Adot"),
    ("", "CPOE", "Cpoe"),
    ("", "EPA/db", "EpaDb"),
    ("", "YPRR", "Yprr"),
    ("", "Y/A", "Ypa"),
    ("", "Y/C", "Ypc"),
    ("", "Y/R", "Ypr"),
    ("", "FG%", "FgPct"),
    ("", "40", "Forty"),
    ("", "Bench", "Bench"),
    ("", "Vertical", "Vertical"),
    ("", "Broad", "Broad"),
    ("", "3-cone", "Cone"),
    ("", "Shuttle", "Shuttle"),
    ("", "Arm", "Arm"),
    ("", "Hand", "Hand"),
    ("passing", "Yds", "PassYds"),
    ("passing", "TD", "PassTd"),
    ("passing", "Int", "Int"),
    ("passing", "Sk", "Sacked"),
    ("passing", "Clean", "Clean"),
    ("passing", "Pressured", "Pressured"),
    ("rushing", "Yds", "RushYds"),
    ("rushing", "TD", "RushTd"),
    ("rushing", "Long", "RushLong"),
    ("rushing", "Before contact", "Ybc"),
    ("receiving", "Rec", "Rec"),
    ("receiving", "Yds", "RecYds"),
    ("receiving", "TD", "RecTd"),
    ("receiving", "Air yds", "TgtAir"),
    ("receiving", "YAC", "Yac"),
    ("receiving", "Sep", "Sep"),
    ("defense", "Tkl", "TklTotal"),
    ("defense", "Sk", "Sacks"),
    ("defense", "Pres", "Pressures"),
    ("defense", "Int", "Ints"),
    ("defense", "PD", "Pd"),
    ("kicking", "FG", "FgLine"),
    ("kicking", "Long", "FgLong"),
    ("kicking", "XP", "XpLine"),
    ("kicking", "Punts", "Punts"),
    ("kicking", "Avg", "PuntGross"),
    ("kicking", "In 20", "PuntIn20"),
    ("returns", "KR", "KickReturns"),
    ("returns", "PR", "PuntReturns"),
    ("returns", "Yds", "RetYdsBox"),
    ("returns", "Long", "RetLong"),
    ("team", "Total yards", "TeamYards"),
    ("team", "Passing", "TeamPass"),
    ("team", "Rushing", "TeamRush"),
    ("team", "First downs", "FirstDowns"),
    ("team", "Third downs", "ThirdDowns"),
    ("team", "Fourth downs", "FourthDowns"),
    ("team", "Sacks by", "TeamSacks"),
    ("team", "Turnovers", "Turnovers"),
    ("team", "Penalties", "Penalties"),
    ("team", "Possession", "Possession"),
    ("team", "Plays", "Plays"),
    ("personnel", "Snaps", "PkgSnaps"),
    ("personnel", "Share", "Share"),
    ("personnel", "Run", "PkgRun"),
    ("personnel", "Pass", "PkgPass"),
    ("personnel", "Yds a play", "YdsPlay"),
    ("personnel", "EPA a play", "EpaPlay"),
    ("their package", "Snaps", "PkgSnaps"),
    ("their package", "Share", "Share"),
    ("their package", "Yds a play", "YdsPlay"),
    ("their package", "EPA a play", "EpaPlay"),
    ("split", "Yards a game", "SplitYds"),
    ("split", "TD", "SplitTd"),
    ("split", "Tackles a game", "SplitTkl"),
];

/// A Player analytics number's entry.
pub fn cm_key(cm: Cm) -> &'static str {
    use Cm::*;
    match cm {
        EpaDb => "EpaDb",
        Cpoe => "Cpoe",
        Adot => "Adot",
        Ypa => "Ypa",
        Rating => "Rating",
        PressSack => "PressSack",
        Btt => "Btt",
        Twp => "Twp",
        TdPct => "TdPct",
        IntPct => "IntPct",
        SackRate => "SackRate",
        PressRate => "PressRate",
        CmpPct => "Pct",
        PassYdsG => "PassYdsG",
        PassTd => "PassTd",
        Int => "Int",
        Ypc => "Ypc",
        YacoC => "YacoC",
        Mtf => "Mtf",
        Explosive => "Explosive",
        Success => "Success",
        FumRate => "FumRate",
        RushYdsG => "RushYdsG",
        RushTd => "RushTd",
        Carries => "Rush",
        Yprr => "Yprr",
        Sep => "Sep",
        CatchRate => "CatchRate",
        Drop => "Drop",
        YacR => "YacR",
        Ypr => "Ypr",
        TgtRoute => "TgtRoute",
        RecG => "RecG",
        RecYdsG => "RecYdsG",
        RecTd => "RecTd",
        PressAllowed => "PressAllowed",
        SackAllowed => "SackAllowed",
        RbWin => "RbWin",
        PrWin => "PrWin",
        Pressure => "Pressure",
        SackPerRush => "SackPerRush",
        Stop => "Stop",
        Tfl => "Tfl",
        Miss => "Miss",
        TklG => "TklG",
        Sacks => "Sacks",
        YdsCov => "YdsCov",
        TgtCov => "TgtCov",
        BallsPlayed => "BallsPlayed",
        Ints => "Ints",
        Pbus => "Pbus",
        FgPct => "FgPct",
        Fg50 => "Fg50Made",
        FgLong => "FgLong",
        FgMade => "FgMade",
        XpMade => "XpMade",
        NetPunt => "PuntAvg",
        Punts => "Punts",
        Grade => "Grade",
    }
}

/// The record books' rates (`marks::RATES`, in order); a college book's
/// first rate is passing efficiency.
pub fn rate_key(k: usize, college: bool) -> Option<&'static str> {
    match k {
        0 if college => Some("Efficiency"),
        0 => Some("Rating"),
        1 => Some("Pct"),
        2 => Some("Ypc"),
        3 => Some("Ypr"),
        4 => Some("FgPct"),
        _ => None,
    }
}

/// `stat_glossary`: every entry and every header that names one.
pub fn all() -> Value {
    json!({
        "terms": TERMS.iter().map(|t| json!({ "key": t.key, "name": t.name, "plain": t.plain, "how": t.how, "better": t.better })).collect::<Vec<_>>(),
        "heads": HEADS.iter().map(|h| [h.0, h.1, h.2]).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use afm_engine::world::analytics::{GROUPS, boxed, charted};
    use afm_engine::world::marks::RATES;
    use afm_engine::world::records::RECORD_STATS;
    use std::collections::HashSet;

    fn has(key: &str) -> bool {
        TERMS.iter().any(|t| t.key == key)
    }

    /// Every stat a page names has plain words, once, and every header
    /// points at one.
    #[test]
    fn every_stat_the_game_shows_explains_itself() {
        let mut keys = HashSet::new();
        for t in TERMS {
            assert!(keys.insert(t.key), "{} twice", t.key);
            assert!(!t.name.is_empty() && t.plain.len() > 10, "{} has no plain words", t.key);
            assert!(t.plain.ends_with('.') && (t.how.is_empty() || t.how.ends_with('.')), "{}: whole sentences", t.key);
            assert!((-1..=1).contains(&t.better));
        }
        let mut heads = HashSet::new();
        for (g, h, k) in HEADS {
            assert!(has(k), "header {g}|{h} points at no entry ({k})");
            assert!(heads.insert((g, h)), "header {g}|{h} twice");
            assert_eq!(*g, g.to_lowercase(), "groups are in lower case");
        }
        for g in 0..GROUPS.len() {
            for cm in charted(g).iter().chain(boxed(g)) {
                assert!(has(cm_key(*cm)), "{cm:?} has no entry");
            }
        }
        for c in crate::lane_core::tables::COLS {
            assert!(has(c.key), "leaderboard column {} has no entry", c.key);
        }
        for s in RECORD_STATS {
            assert!(has(s.key()), "record book {} has no entry", s.key());
        }
        for k in 0..RATES.len() {
            for college in [false, true] {
                assert!(rate_key(k, college).is_some_and(has), "rate {k} has no entry");
            }
        }
    }
}
