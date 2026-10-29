//! Trial tooling: every college game's team totals and each player's season
//! totals, kept only when `WorldOptions::keep_game_log` is on. It only
//! watches the games, so a world with it on plays out exactly the same.
//! The situation keys here also feed the world reader's game log on disk
//! (`wlog/games.rs`), which keeps every game at every level.

use std::collections::HashMap;

use super::data::Division;
use crate::game::{GameStats, PlayRecord, PlayType, TeamStats};

/// A situation's totals: plays, yards, EPA, successes (EPA above 0), first
/// downs, touchdowns, turnovers, and one extra count (completions on passes,
/// sacks on dropbacks, makes on kicks, touchbacks on kickoffs).
pub type Situ = [f64; 8];

/// One college game, as the box score's team totals.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GameSummary {
    pub year: i32,
    pub division: Division,
    /// Program ids, home first.
    pub teams: [usize; 2],
    pub score: [i32; 2],
    pub stats: [TeamStats; 2],
    /// Kick and punt returns (count, yards), from the player lines.
    pub kr: [(u32, f64); 2],
    pub pr: [(u32, f64); 2],
    /// Extra point kicks, two-point tries, and punts with their gross yards,
    /// from the play log (fix list topic 1 checks).
    pub xp: [u32; 2],
    pub two_point: [u32; 2],
    pub punt_yds: [(u32, f64); 2],
    pub overtime: bool,
    /// Why the game mattered (its tags, part D 40), how loud it was, and
    /// snaps a man from another position group had to play (fix list 3.6).
    #[serde(deserialize_with = "crate::save::leaked")]
    pub tags: Vec<&'static str>,
    #[serde(deserialize_with = "crate::save::leaked")]
    pub loud: crate::save::Text,
    pub emergency_snaps: u32,
    /// A game between programs of different divisions (an FBS buy game).
    pub cross: bool,
    /// What kind of game (regular, conference title, bowl, playoff, title,
    /// preseason...), its date, week and a neutral site (world reader R1).
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub week: u32,
    #[serde(default)]
    pub neutral: bool,
    /// Points by quarter (overtime periods after the fourth), home then away.
    #[serde(default)]
    pub quarters: Vec<[i32; 2]>,
}

/// A player's season at one program: the headline counting stats.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PlayerSeason {
    pub program: usize,
    /// His listed position at the end of the season.
    pub pos: Option<crate::positions::Position>,
    pub games: u32,
    /// Offensive and defensive snaps.
    pub snaps: u32,
    pub att: u32,
    pub cmp: u32,
    pub pass_yds: f32,
    pub pass_td: u32,
    pub int_thrown: u32,
    pub sacked: u32,
    pub rush: u32,
    pub rush_yds: f32,
    pub rush_td: u32,
    pub rec: u32,
    pub rec_yds: f32,
    pub rec_td: u32,
    pub tackles: u32,
    pub tfl: f32,
    pub sacks: f32,
    pub ints: u32,
    pub pbus: u32,
    pub pressures: u32,
    pub kr: u32,
    pub kr_yds: f32,
    pub fg_made: u32,
    pub fg_att: u32,
    pub grade_sum: f32,
    pub graded: u32,
    /// Field goals by distance (under 30, 30s, 40s, 50+) and extra points,
    /// kicks blocked, and his penalties (world reader R1).
    #[serde(default)]
    pub fg_att_band: [u32; 4],
    #[serde(default)]
    pub fg_made_band: [u32; 4],
    #[serde(default)]
    pub xp_att: u32,
    #[serde(default)]
    pub xp_made: u32,
    #[serde(default)]
    pub penalties: u32,
    #[serde(default)]
    pub penalty_yds: f32,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GameLog {
    pub games: Vec<GameSummary>,
    /// (season, player id) to his season totals.
    pub players: HashMap<(i32, u32), PlayerSeason>,
    /// Player id to name, position when first seen, stars and class.
    pub names: HashMap<u32, (String, crate::positions::Position, u8, i32)>,
    /// (season, FBS program) to its starters' rating sum and count by position.
    /// (season, FBS program) to [rotation players' average, their count,
    /// starters' average college seasons, kicker, OC calling, DC calling].
    pub extras: HashMap<(i32, usize), [f64; 6]>,
    pub lineups:
        HashMap<(i32, usize), ([f64; crate::positions::POSITION_COUNT], [u32; crate::positions::POSITION_COUNT])>,
    /// (season, division, situation key) to every play's totals in it: down
    /// and distance, field zone, game state, fourth-down decisions, pass
    /// depth, time to throw, calls, fronts, penalties, kicks. Keys are
    /// readable text, such as "dd|3|4-6|pass".
    pub situations: HashMap<(i32, Division, String), Situ>,
    /// (season, FBS program, key) to the same totals for one team: "o|" keys
    /// when it has the ball, "d|" keys when it defends.
    pub team_situations: HashMap<(i32, usize, String), Situ>,
    /// NFL games on the engine (section 5): the same box score, team
    /// indices into `World::nfl`; `division` is unused.
    pub nfl_games: Vec<GameSummary>,
    /// (season, "NFL" or "HS", key): the NFL's and high school's situation
    /// totals, the same keys as `situations`.
    #[serde(default)]
    pub level_situations: HashMap<(i32, String, String), Situ>,
}

fn togo_bucket(t: f64) -> &'static str {
    match t {
        t if t <= 1.0 => "1",
        t if t <= 3.0 => "2-3",
        t if t <= 6.0 => "4-6",
        t if t <= 10.0 => "7-10",
        _ => "11+",
    }
}

fn field_bucket(y: f64) -> &'static str {
    match y {
        y if y <= 5.0 => "goal (1-5)",
        y if y <= 20.0 => "red zone (6-20)",
        y if y <= 40.0 => "opp 21-40",
        y if y <= 60.0 => "midfield",
        y if y <= 80.0 => "own 21-40",
        y if y <= 95.0 => "own 6-20",
        _ => "backed up (own 1-5)",
    }
}

fn kind_name(k: PlayType) -> &'static str {
    match k {
        PlayType::Run => "run",
        PlayType::Pass | PlayType::Sack | PlayType::Scramble => "pass",
        PlayType::Kneel => "kneel",
        PlayType::Spike => "spike",
        PlayType::Punt => "punt",
        PlayType::FieldGoal => "fg",
        PlayType::ExtraPoint => "xp",
        PlayType::TwoPoint => "two point",
        PlayType::Kickoff => "kickoff",
        PlayType::NoPlay => "no play",
    }
}

/// Score margin buckets of the real NFL tables (`nfl_situations.json`):
/// -15 is 15 or more behind, -8 is 8 to 14 behind, -1 is 1 to 7 behind,
/// 0 tied, 7 is 1 to 7 ahead, 14 is 8 to 14 ahead, 15 is 15 or more.
fn real_score_bucket(m: i32) -> i32 {
    match m {
        m if m <= -15 => -15,
        m if m <= -8 => -8,
        m if m <= -1 => -1,
        0 => 0,
        m if m <= 7 => 7,
        m if m <= 14 => 14,
        _ => 15,
    }
}

/// Yards to go buckets of the real NFL tables: 1, 2-3, 4-6, 7-10, 11+.
fn real_togo_bucket(t: f64) -> u32 {
    match t {
        t if t <= 1.0 => 1,
        t if t <= 3.0 => 3,
        t if t <= 6.0 => 6,
        t if t <= 10.0 => 10,
        _ => 15,
    }
}

/// The real tables' time of game: the last two minutes of a half, the
/// fourth quarter's last seven minutes, or normal.
fn real_time(p: &PlayRecord) -> &'static str {
    if matches!(p.quarter, 2 | 4) && p.clock <= 120.0 {
        "two_minute"
    } else if p.quarter == 4 && p.clock <= 420.0 {
        "late"
    } else {
        "normal"
    }
}

/// `garbage_time.json`'s buckets for the score after three quarters.
fn q4_bucket(lead: i32) -> &'static str {
    match lead {
        l if l >= 28 => "lead 28+",
        l if l >= 21 => "lead 21-27",
        l if l >= 15 => "lead 15-20",
        l if l >= 1 => "lead 1-14",
        0 => "tied",
        l if l >= -14 => "trail 1-14",
        l if l >= -20 => "trail 15-20",
        l if l >= -27 => "trail 21-27",
        _ => "trail 28+",
    }
}

/// Where a designed run crossed the line, as the real play by play names
/// it (left or right, then end, tackle or guard; middle): from how far
/// across from the snap the carrier crossed, the offense's left positive
/// (`TendNotes::cross`). Gap widths about the line's real splits: inside a
/// yard is the middle, inside 2.5 the guard, inside 4 the tackle
/// (proposed, not confirmed).
fn run_gap(p: &PlayRecord) -> Option<(&'static str, &'static str)> {
    let (y, _) = p.tend.cross?;
    let side = if y > 0.0 { "left" } else { "right" };
    Some(match y.abs() {
        a if a < 1.0 => ("middle", "middle"),
        a if a < 2.5 => (side, "guard"),
        a if a < 4.0 => (side, "tackle"),
        _ => (side, "end"),
    })
}

/// The call's look, read off the call text ("11 gun trips jet motion,
/// inside zone left"): the backfield, the motion, and whether it was a
/// run-pass option.
fn call_look(call: &str) -> (&'static str, &'static str, bool) {
    let backfield = if call.contains(" gun ") {
        "gun"
    } else if call.contains(" pistol ") {
        "pistol"
    } else if call.contains(" under center ") {
        "under center"
    } else {
        "other"
    };
    let motion = ["jet motion, ", "orbit motion, ", "motion across, ", "shift, "]
        .into_iter()
        .find(|m| call.contains(m))
        .map_or("none", |m| m.trim_end_matches(", "));
    (backfield, motion, call.contains(" with "))
}

/// Context one play's keys need from the whole game.
#[derive(Clone, Copy, Default)]
struct PlayCtx {
    /// Each side's lead (home, away) after three quarters, once the fourth
    /// has started.
    q4_lead: Option<[i32; 2]>,
}

/// Every situation key one play counts under (league-wide).
fn play_keys(p: &PlayRecord, ctx: PlayCtx) -> Vec<String> {
    let kind = kind_name(p.kind);
    let mut keys = vec![format!("kind|{kind}")];
    let scrimmage = matches!(p.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble);
    let field = field_bucket(p.yardline_100);
    let margin = p.score[p.offense.min(1)] - p.score[1 - p.offense.min(1)];
    if (1..=4).contains(&p.down) && !matches!(p.kind, PlayType::Kickoff | PlayType::ExtraPoint | PlayType::TwoPoint) {
        keys.push(format!("dd|{}|{}|{kind}", p.down, togo_bucket(p.togo)));
        keys.push(format!("field|{field}|{kind}"));
    }
    if p.down == 4
        && matches!(
            p.kind,
            PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble | PlayType::Punt | PlayType::FieldGoal
        )
    {
        let decision = if scrimmage { "go" } else { kind };
        keys.push(format!("fourth|{field}|{}|{decision}", togo_bucket(p.togo)));
        // The real table's own buckets: to go, the yard line by tens and
        // the score.
        keys.push(format!(
            "real fourth|{}|{}|{}|{decision}",
            real_togo_bucket(p.togo),
            ((p.yardline_100 / 10.0).floor() * 10.0).clamp(0.0, 90.0) as u32,
            real_score_bucket(margin)
        ));
    }
    if scrimmage {
        let state = match margin {
            m if m <= -17 => "down 17+",
            m if m <= -9 => "down 9-16",
            m if m <= -1 => "down 1-8",
            0 => "tied",
            m if m <= 8 => "up 1-8",
            m if m <= 16 => "up 9-16",
            _ => "up 17+",
        };
        let q = if p.quarter > 4 { "OT".to_string() } else { format!("Q{}", p.quarter) };
        keys.push(format!("state|{q}|{state}|{kind}"));
        if p.quarter >= 4 && p.clock <= 120.0 {
            keys.push(format!("two minute|{state}|{kind}"));
        }
        if (1..=4).contains(&p.down) {
            keys.push(format!(
                "real pass rate|{}|{}|{}|{}|{kind}",
                p.down,
                real_togo_bucket(p.togo),
                real_score_bucket(margin),
                real_time(p)
            ));
        }
        if p.quarter == 4
            && let Some(lead) = ctx.q4_lead
        {
            keys.push(format!("q4|{}|{kind}", q4_bucket(lead[p.offense.min(1)])));
        }
        keys.push(format!("call|{kind}|{}", p.call));
        keys.push(format!("defense|{}|{kind}", p.defense_call));
        keys.push(format!("personnel|{}|{kind}", p.personnel));
        keys.push(format!("box|{}|{kind}", p.box_count));
        keys.push(format!("blitz|{}|{kind}", p.blitz));
        keys.push(format!("coverage|{}|{kind}", if p.coverage_man { "man" } else { "zone" }));
        let (backfield, motion, rpo) = call_look(&p.call);
        keys.push(format!("backfield|{backfield}|{kind}"));
        keys.push(format!("motion|{motion}|{kind}"));
        if rpo {
            keys.push(format!("rpo|{kind}"));
        }
        keys.push(format!("tempo|{}|{kind}", if p.hurry { "no huddle" } else { "huddle" }));
        let g = p.garbage[p.offense.min(1)];
        let gd = p.garbage[1 - p.offense.min(1)];
        keys.push(format!("garbage|{g}|{gd}|{kind}"));
        if p.after_turnover {
            keys.push(format!("after turnover|{kind}"));
        }
        if p.out_of_bounds {
            keys.push(format!("out of bounds|{kind}"));
        }
    }
    if p.kind == PlayType::Run
        && let Some((side, gap)) = run_gap(p)
    {
        keys.push(format!("run gap|{side}|{gap}"));
    }
    if kind == "pass" {
        keys.push(format!("play action|{}", p.play_action));
        keys.push(format!("pressured|{}", p.pressured));
        let ttt = match p.time_to_throw {
            t if t <= 0.0 => "none",
            t if t < 2.0 => "<2.0s",
            t if t < 2.5 => "2.0-2.5s",
            t if t < 3.0 => "2.5-3.0s",
            t if t < 3.5 => "3.0-3.5s",
            _ => "3.5s+",
        };
        keys.push(format!("time to throw|{ttt}"));
        if p.kind == PlayType::Pass && !p.throw_away {
            let air = match p.air_yards {
                a if a < 0.0 => "behind line",
                a if a < 10.0 => "0-9",
                a if a < 20.0 => "10-19",
                _ => "20+",
            };
            keys.push(format!("depth|{air}"));
            // The real table's air-yards buckets (-5, 0, 10, 20).
            let real = match p.air_yards {
                a if a < 0.0 => -5,
                a if a < 10.0 => 0,
                a if a < 20.0 => 10,
                _ => 20,
            };
            keys.push(format!("real air yards|{real}"));
        }
        if p.target.is_some() && !p.route.is_empty() {
            keys.push(format!("route|{}", p.route));
            keys.push(format!("route coverage|{}|{}", p.route, if p.coverage_man { "man" } else { "zone" }));
        }
        if p.drop {
            keys.push("drop".to_string());
        }
        if p.throw_away {
            keys.push("throw away".to_string());
        }
    }
    if matches!(p.kind, PlayType::FieldGoal | PlayType::Punt | PlayType::Kickoff) {
        let d = (p.kick_distance / 10.0).floor() * 10.0;
        keys.push(format!("kick|{kind}|{d:.0}-{:.0}", d + 9.0));
    }
    if matches!(p.kind, PlayType::Kickoff | PlayType::Punt) && p.return_yards != 0.0 {
        keys.push(format!("return|{kind}"));
    }
    if matches!(p.kind, PlayType::FieldGoal | PlayType::ExtraPoint | PlayType::Punt | PlayType::Kickoff) {
        for (on, what) in [
            (p.blocked, "blocked"),
            (p.tend.muffed, "muffed"),
            (p.bad_snap, "bad snap"),
            (p.fair_catch, "fair catch"),
            (p.touchback, "touchback"),
            (p.inside20, "inside 20"),
        ] {
            if on {
                keys.push(format!("kick result|{kind}|{what}"));
            }
        }
    }
    for (foul, team, _, _, accepted) in &p.penalties {
        let state = if *accepted { "accepted" } else { "declined" };
        keys.push(format!("penalty|{foul:?}|{state}"));
        keys.push(format!("penalty site|{foul:?}|{}", if *team == 0 { "home" } else { "away" }));
    }
    if let Some(o) = &p.officiating {
        keys.push(format!(
            "officiating|{:?}|{}|{}|{}",
            o.call,
            if o.wrong { "wrong" } else { "right" },
            match o.reviewer {
                crate::game::live::Reviewer::None => "not reviewed",
                crate::game::live::Reviewer::Booth => "booth",
                crate::game::live::Reviewer::Assist => "assist",
                crate::game::live::Reviewer::Challenge(_) => "challenge",
            },
            if o.overturned { "overturned" } else { "stood" }
        ));
    }
    if !p.injuries.is_empty() {
        keys.push(format!("injury|{kind}"));
    }
    if !p.missed_tackles.is_empty() {
        keys.push(format!("missed tackles|{kind}"));
    }
    keys
}

/// The totals one play adds.
fn play_values(p: &PlayRecord) -> Situ {
    let extra = match p.kind {
        PlayType::Pass => p.complete as u32 as f64,
        PlayType::Sack => 1.0,
        PlayType::FieldGoal | PlayType::ExtraPoint => p.kick_good as u32 as f64,
        PlayType::Kickoff | PlayType::Punt => p.touchback as u32 as f64,
        PlayType::Run => p.missed_tackles.len() as f64,
        _ => 0.0,
    };
    let yards = match p.kind {
        PlayType::Punt | PlayType::Kickoff => p.kick_distance,
        PlayType::FieldGoal | PlayType::ExtraPoint => p.kick_distance,
        _ => p.yards,
    };
    [
        1.0,
        yards,
        p.epa,
        (p.epa > 0.0) as u32 as f64,
        p.first_down as u32 as f64,
        p.touchdown as u32 as f64,
        p.turnover as u32 as f64,
        extra,
    ]
}

pub fn bump(m: &mut Situ, v: &Situ) {
    for k in 0..8 {
        m[k] += v[k];
    }
}

/// One game's situation totals, built outside the log's lock.
pub type SituPart = (HashMap<String, Situ>, HashMap<(usize, String), Situ>);

/// One game's situation totals: the league keys, and each side's own short
/// list (home, away), "o|" keys with the ball and "d|" keys defending.
pub struct GameSitu {
    pub league: HashMap<String, Situ>,
    pub team: [HashMap<String, Situ>; 2],
}

/// Sums one game's plays by situation, at any level.
pub fn game_situations(g: &GameStats) -> GameSitu {
    let mut league: HashMap<String, Situ> = HashMap::new();
    let mut team: [HashMap<String, Situ>; 2] = Default::default();
    let mut ctx = PlayCtx::default();
    for p in &g.plays {
        if ctx.q4_lead.is_none() && p.quarter >= 4 {
            ctx.q4_lead = Some([p.score[0] - p.score[1], p.score[1] - p.score[0]]);
        }
        let v = play_values(p);
        for k in play_keys(p, ctx) {
            bump(league.entry(k).or_default(), &v);
        }
        // A team's own profile: a short list of keys, both sides of the ball.
        let kind = kind_name(p.kind);
        let mut tk = vec![format!("kind|{kind}")];
        if p.down == 3 {
            tk.push(format!("third|{}|{kind}", togo_bucket(p.togo)));
        }
        if p.down == 4 {
            let decision = if matches!(p.kind, PlayType::Punt | PlayType::FieldGoal) { kind } else { "go" };
            tk.push(format!("fourth|{decision}"));
        }
        if p.yardline_100 <= 20.0 {
            tk.push(format!("red zone|{kind}"));
        }
        if kind == "pass" {
            tk.push(format!("pressured|{}", p.pressured));
            tk.push(format!("blitz|{}", p.blitz));
        }
        let o = p.offense.min(1);
        for (side, tag) in [(o, "o"), (1 - o, "d")] {
            for k in &tk {
                bump(team[side].entry(format!("{tag}|{k}")).or_default(), &v);
            }
        }
    }
    GameSitu { league, team }
}

/// Sums one game's plays by situation (league keys, and FBS team keys).
pub fn situations(division: Division, teams: [usize; 2], g: &GameStats) -> SituPart {
    let GameSitu { league, team } = game_situations(g);
    let mut by_team: HashMap<(usize, String), Situ> = HashMap::new();
    if division == Division::FBS {
        for (side, m) in team.into_iter().enumerate() {
            for (k, v) in m {
                by_team.insert((teams[side], k), v);
            }
        }
    }
    (league, by_team)
}

impl GameLog {
    pub fn add(&mut self, year: i32, division: Division, teams: [usize; 2], g: &GameStats, part: SituPart) {
        let mut kr = [(0, 0.0); 2];
        let mut pr = [(0, 0.0); 2];
        for side in 0..2 {
            for (&id, l) in &g.players[side] {
                kr[side].0 += l.kr;
                kr[side].1 += l.kr_yds;
                pr[side].0 += l.pr;
                pr[side].1 += l.pr_yds;
                let s = self.players.entry((year, id)).or_default();
                s.program = teams[side];
                s.games += 1;
                s.snaps += l.snaps_off + l.snaps_def;
                s.att += l.att;
                s.cmp += l.cmp;
                s.pass_yds += l.pass_yds as f32;
                s.pass_td += l.pass_td;
                s.int_thrown += l.int;
                s.sacked += l.sacked;
                s.rush += l.rush;
                s.rush_yds += l.rush_yds as f32;
                s.rush_td += l.rush_td;
                s.rec += l.rec;
                s.rec_yds += l.rec_yds as f32;
                s.rec_td += l.rec_td;
                s.tackles += l.tackles + l.assists;
                s.tfl += l.tfl as f32;
                s.sacks += l.sacks as f32;
                s.ints += l.ints;
                s.pbus += l.pbus;
                s.pressures += l.pressures;
                s.kr += l.kr;
                s.kr_yds += l.kr_yds as f32;
                s.fg_made += l.fg_made.iter().sum::<u32>();
                s.fg_att += l.fg_att.iter().sum::<u32>();
                s.grade_sum += l.grade_sum as f32;
                s.graded += l.graded;
                for b in 0..4 {
                    s.fg_att_band[b] += l.fg_att[b];
                    s.fg_made_band[b] += l.fg_made[b];
                }
                s.xp_att += l.xp_att;
                s.xp_made += l.xp_made;
                s.penalties += l.penalties;
                s.penalty_yds += l.penalty_yds as f32;
            }
        }
        let (league, team) = part;
        for (k, v) in league {
            bump(self.situations.entry((year, division, k)).or_default(), &v);
        }
        for ((prog, k), v) in team {
            bump(self.team_situations.entry((year, prog, k)).or_default(), &v);
        }
        let summary = summarize(year, division, teams, g, kr, pr);
        self.games.push(summary);
    }

    /// A level's situation totals (the NFL's, high school's).
    pub fn add_level(&mut self, year: i32, level: &str, g: &GameStats) {
        for (k, v) in game_situations(g).league {
            bump(self.level_situations.entry((year, level.to_string(), k)).or_default(), &v);
        }
    }

    /// An NFL game's box score (the players' seasons live on their pro careers).
    pub fn add_nfl(&mut self, year: i32, teams: [usize; 2], g: &GameStats) {
        self.add_level(year, "NFL", g);
        let mut kr = [(0, 0.0); 2];
        let mut pr = [(0, 0.0); 2];
        for side in 0..2 {
            for l in g.players[side].values() {
                kr[side].0 += l.kr;
                kr[side].1 += l.kr_yds;
                pr[side].0 += l.pr;
                pr[side].1 += l.pr_yds;
            }
        }
        let summary = summarize(year, Division::FBS, teams, g, kr, pr);
        self.nfl_games.push(summary);
    }
}

fn summarize(
    year: i32,
    division: Division,
    teams: [usize; 2],
    g: &GameStats,
    kr: [(u32, f64); 2],
    pr: [(u32, f64); 2],
) -> GameSummary {
    let mut xp = [0; 2];
    let mut two_point = [0; 2];
    let mut punt_yds = [(0, 0.0); 2];
    for r in &g.plays {
        match r.kind {
            PlayType::ExtraPoint => xp[r.offense] += 1,
            PlayType::TwoPoint => two_point[r.offense] += 1,
            PlayType::Punt if !r.blocked => {
                punt_yds[r.offense].0 += 1;
                punt_yds[r.offense].1 += r.kick_distance;
            }
            _ => {}
        }
    }
    GameSummary {
        year,
        division,
        teams,
        score: g.score,
        stats: g.team.clone(),
        kr,
        pr,
        xp,
        two_point,
        punt_yds,
        overtime: g.overtime,
        tags: Vec::new(),
        loud: "",
        emergency_snaps: g.plays.iter().filter(|p| p.emergency).count() as u32,
        cross: false,
        kind: String::new(),
        date: String::new(),
        week: 0,
        neutral: false,
        quarters: g.quarters.clone(),
    }
}
