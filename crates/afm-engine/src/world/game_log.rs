//! Trial tooling: every college game's team totals and each player's season
//! totals, kept only when `WorldOptions::keep_game_log` is on. It only
//! watches the games, so a world with it on plays out exactly the same.
//! The game's own career stats record (section 3) is still to be designed.

use std::collections::HashMap;

use super::data::Division;
use crate::game::{GameStats, PlayRecord, PlayType, TeamStats};

/// A situation's totals: plays, yards, EPA, successes (EPA above 0), first
/// downs, touchdowns, turnovers, and one extra count (completions on passes,
/// sacks on dropbacks, makes on kicks, touchbacks on kickoffs).
pub type Situ = [f64; 8];

/// One college game, as the box score's team totals.
#[derive(Clone, Debug)]
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
    pub overtime: bool,
}

/// A player's season at one program: the headline counting stats.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerSeason {
    pub program: usize,
    /// His listed position at the end of the season.
    pub pos: Option<crate::positions::Position>,
    pub games: u32,
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
}

#[derive(Clone, Debug, Default)]
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

/// Every situation key one play counts under (league-wide).
fn play_keys(p: &PlayRecord) -> Vec<String> {
    let kind = kind_name(p.kind);
    let mut keys = vec![format!("kind|{kind}")];
    let scrimmage = matches!(p.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble);
    let field = field_bucket(p.yardline_100);
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
    }
    if scrimmage {
        let margin = p.score[p.offense] - p.score[1 - p.offense];
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
        keys.push(format!("call|{kind}|{}", p.call));
        keys.push(format!("defense|{}|{kind}", p.defense_call));
        keys.push(format!("personnel|{}|{kind}", p.personnel));
        keys.push(format!("box|{}|{kind}", p.box_count));
        keys.push(format!("blitz|{}|{kind}", p.blitz));
        keys.push(format!("coverage|{}|{kind}", if p.coverage_man { "man" } else { "zone" }));
        if p.after_turnover {
            keys.push(format!("after turnover|{kind}"));
        }
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
    for (foul, _, _, _, accepted) in &p.penalties {
        keys.push(format!("penalty|{foul:?}|{}", if *accepted { "accepted" } else { "declined" }));
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

fn bump(m: &mut Situ, v: &Situ) {
    for k in 0..8 {
        m[k] += v[k];
    }
}

/// One game's situation totals, built outside the log's lock.
pub type SituPart = (HashMap<String, Situ>, HashMap<(usize, String), Situ>);

/// Sums one game's plays by situation (league keys, and FBS team keys).
pub fn situations(division: Division, teams: [usize; 2], g: &GameStats) -> SituPart {
    let mut league: HashMap<String, Situ> = HashMap::new();
    let mut team: HashMap<(usize, String), Situ> = HashMap::new();
    for p in &g.plays {
        let v = play_values(p);
        for k in play_keys(p) {
            bump(league.entry(k).or_default(), &v);
        }
        if division == Division::FBS {
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
            for (side, tag) in [(p.offense, "o"), (1 - p.offense, "d")] {
                for k in &tk {
                    bump(team.entry((teams[side], format!("{tag}|{k}"))).or_default(), &v);
                }
            }
        }
    }
    (league, team)
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
            }
        }
        let (league, team) = part;
        for (k, v) in league {
            bump(self.situations.entry((year, division, k)).or_default(), &v);
        }
        for ((prog, k), v) in team {
            bump(self.team_situations.entry((year, prog, k)).or_default(), &v);
        }
        self.games.push(GameSummary {
            year,
            division,
            teams,
            score: g.score,
            stats: g.team.clone(),
            kr,
            pr,
            overtime: g.overtime,
        });
    }
}
