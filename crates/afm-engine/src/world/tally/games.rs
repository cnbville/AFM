//! The games tally (`games/<season>.tally`): every engine game of the
//! season at every level, one record each in the order they were played.
//! College and NFL games carry both team boxes, the advanced numbers each
//! side's plays add up to, the win chance before every snap and every
//! player's line; high school games carry the facts and the boxes.
//!
//! Built on the core that played the game (`part`), so the world's own
//! thread only adds it.

use crate::calendar::Date;
use crate::game::stats::{GameStats, PlayRecord, PlayType, TeamStats};
use crate::positions::Position;
use crate::world::records::{STAT_COUNT, Stat, from_line};

use super::file::{Frames, encode};
use super::{Level, players, squads, teams};

/// What the world knows about a game before it is played.
#[derive(Clone, Debug, Default)]
pub struct Facts {
    pub level: Level,
    pub season: i32,
    pub date: Date,
    /// regular, conference title, bowl, playoff, title (college); regular,
    /// postseason (NFL); regular, playoff, state final (high school).
    pub kind: &'static str,
    pub teams: [u32; 2],
    /// The college division (both, "FBS/FCS", for a game across them),
    /// "NFL", or the state's code.
    pub group: String,
    pub neutral: bool,
    pub conference: bool,
    pub weather: Weather,
    /// Each side ranked in the latest college poll at kickoff (home,
    /// away), for the players' split against ranked teams.
    pub ranked: [bool; 2],
    /// What the home side was expected to win by (the closing line turned
    /// round), where the books hung one (lane NB8: the analysts' model).
    pub spread: Option<f32>,
    /// The level the game was played at, for the analysts' model (a
    /// college game across divisions reads the higher one).
    pub lv: Option<crate::player::Level>,
}

/// The weather a game was played in.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Weather {
    pub temp: f32,
    pub wind: f32,
    pub rain: bool,
    pub snow: bool,
    pub indoor: bool,
}

/// What one side's plays add up to beyond the box: the numbers
/// cfbfastR and Football Outsiders work out from play by play.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Adv {
    /// Designed runs (scrambles are dropbacks), their EPA and successes,
    /// the stuffed (no gain or a loss) and line yards (a loss at 120%, the
    /// first four yards whole, the next six at half, nothing past ten).
    pub runs: u32,
    pub run_epa: f32,
    pub run_success: u32,
    pub stuffed: u32,
    pub line_yards: f32,
    /// Dropbacks (throws, sacks, scrambles), their EPA and successes.
    pub dropbacks: u32,
    pub pass_epa: f32,
    pub pass_success: u32,
    /// Plays this side's defense made a tackle for loss (sacks too), forced
    /// a fumble, intercepted or broke up a pass: havoc.
    pub havoc: u32,
    /// Drives, those that reached the opponent's 40 (a scoring chance), and
    /// what they ended in.
    pub drives: u32,
    pub trips40: u32,
    pub trip_td: u32,
    pub trip_fg: u32,
    /// Three plays or fewer and a punt.
    pub three_outs: u32,
    /// Drive starts, summed in yards from its own goal.
    pub start_sum: f32,
    /// Lane NB8 (the advanced box): explosive plays (runs of 10 yards or
    /// more, dropbacks of 20 or more), the EPA of the successful plays
    /// (explosiveness, Bill Connelly's IsoPPP, is that over the successes)
    /// and the drives that reached the opponent's 20, with their
    /// touchdowns and field goals.
    pub explosive: u32,
    pub success_epa: f32,
    pub rz_trips: u32,
    pub rz_td: u32,
    pub rz_fg: u32,
}

/// One player's game.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Line {
    pub id: u32,
    /// 0 home, 1 away.
    pub side: u8,
    /// On the field for his side's first snap on offense or defense.
    pub started: bool,
    pub pos: Option<Position>,
    /// The stat book's numbers (`records::Stat`) that aren't zero: a bit
    /// each in `mask` (low, high), the values in hundredths for the ones
    /// with fractions and whole for counts (`Stat::fraction`).
    pub mask: [u64; 2],
    pub v: Vec<i32>,
}

impl Line {
    pub fn new(id: u32, side: u8, started: bool, pos: Option<Position>, all: &[f32; STAT_COUNT]) -> Line {
        let mut mask = [0u64; 2];
        let mut v = Vec::new();
        for (k, &x) in all.iter().enumerate() {
            let q = quantize(Stat::ALL[k], x);
            if q != 0 {
                mask[k / 64] |= 1 << (k % 64);
                v.push(q);
            }
        }
        Line { id, side, started, pos, mask, v }
    }

    /// Every stat's value, as the book keeps it.
    pub fn all(&self) -> [f32; STAT_COUNT] {
        let mut out = [0f32; STAT_COUNT];
        let mut it = self.v.iter();
        for (k, o) in out.iter_mut().enumerate() {
            if self.mask[k / 64] >> (k % 64) & 1 == 1 {
                let Some(&q) = it.next() else { break };
                *o = unquantize(Stat::ALL[k], q);
            }
        }
        out
    }

    pub fn get(&self, s: Stat) -> f64 {
        self.all()[s as usize] as f64
    }
}

/// A stat as a whole number on file: hundredths when it has fractions.
pub fn quantize(s: Stat, x: f32) -> i32 {
    if s.fraction() { (x as f64 * 100.0).round() as i32 } else { x.round() as i32 }
}

pub fn unquantize(s: Stat, q: i32) -> f32 {
    if s.fraction() { q as f32 / 100.0 } else { q as f32 }
}

/// A game's record.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct GameRow {
    /// 0 college, 1 NFL, 2 high school (`Level`).
    pub level: u8,
    pub season: i32,
    /// Days, as `calendar::Date`.
    pub date: i32,
    pub kind: String,
    /// Home, away: program ids (college), NFL team indices, high school ids.
    pub teams: [u32; 2],
    pub group: String,
    pub neutral: bool,
    pub conference: bool,
    pub score: [i32; 2],
    pub quarters: Vec<[i32; 2]>,
    pub overtime: bool,
    pub weather: Weather,
    pub boxes: [TeamStats; 2],
    pub adv: [Adv; 2],
    /// The home side's win chance before each snap, in percent (college and
    /// the NFL).
    pub wp: Vec<u8>,
    /// Every player's line (college and the NFL).
    pub lines: Vec<Line>,
}

impl GameRow {
    pub fn date(&self) -> Date {
        Date(self.date)
    }
}

/// One game's tallies, built on the core that played it.
pub struct Part {
    pub level: Level,
    pub date: Date,
    pub teams: [u32; 2],
    pub score: [i32; 2],
    pub adv: [Adv; 2],
    /// The game's record, packed.
    pub row: Vec<u8>,
    /// Every player's game (college and the NFL).
    pub players: Vec<players::Game>,
    /// Each side's situation counts (college and the NFL).
    pub sides: Option<[teams::Game; 2]>,
    /// Each side's 4th downs, tries, timeouts and challenges (every level;
    /// rows only in college and the NFL).
    pub calls: [super::calls::Game; 2],
    /// Each side's squad counts, every level (lane NB7).
    pub squads: [squads::Game; 2],
}

fn scrimmage(p: &PlayRecord) -> bool {
    matches!(p.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble) && !p.nullified
}

/// Each side's starters: on the field for its first snap with the ball
/// (offense) and its first snap without it (defense).
fn starters(g: &GameStats) -> [std::collections::HashSet<u32>; 2] {
    let mut out: [std::collections::HashSet<u32>; 2] = Default::default();
    let mut seen = [[false; 2]; 2];
    for p in g
        .plays
        .iter()
        .filter(|p| matches!(p.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble))
    {
        let o = p.offense.min(1);
        for (unit, side) in [(0usize, o), (1usize, 1 - o)] {
            if !seen[side][unit] {
                seen[side][unit] = true;
                out[side].extend(p.on_field[unit].iter().copied());
            }
        }
        if seen.iter().all(|s| s[0] && s[1]) {
            break;
        }
    }
    out
}

/// Each side's advanced numbers.
pub fn adv(g: &GameStats) -> [Adv; 2] {
    let mut a = [Adv::default(); 2];
    for p in g.plays.iter().filter(|p| scrimmage(p)) {
        let o = p.offense.min(1);
        let x = &mut a[o];
        match p.kind {
            PlayType::Run => {
                x.runs += 1;
                x.run_epa += p.epa as f32;
                x.run_success += p.success() as u32;
                x.stuffed += (p.yards <= 0.0) as u32;
                x.line_yards += line_yards(p.yards) as f32;
            }
            _ => {
                x.dropbacks += 1;
                x.pass_epa += p.epa as f32;
                x.pass_success += p.success() as u32;
            }
        }
        if p.success() {
            x.success_epa += p.epa as f32;
        }
        x.explosive += ((p.kind == PlayType::Run && p.yards >= 10.0) || p.yards >= 20.0) as u32;
        let tfl = p.kind == PlayType::Run && p.yards < 0.0 && !p.tacklers.is_empty();
        if tfl || !p.sackers.is_empty() || p.interception || p.forced_fumble.is_some() || p.pbu.is_some() {
            a[1 - o].havoc += 1;
        }
    }
    let by_n: std::collections::HashMap<u32, &PlayRecord> = g.plays.iter().map(|p| (p.n, p)).collect();
    for d in crate::game::drives::drives(g) {
        let x = &mut a[d.team.min(1)];
        x.drives += 1;
        x.start_sum += d.start as f32;
        let closest = (d.first..=d.last)
            .filter_map(|n| by_n.get(&n))
            .filter(|p| p.offense.min(1) == d.team.min(1))
            .map(|p| p.yardline_100)
            .fold(f64::MAX, f64::min);
        if closest <= 40.0 {
            x.trips40 += 1;
            x.trip_td += (d.result == "Touchdown") as u32;
            x.trip_fg += (d.result == "Field goal") as u32;
        }
        if d.plays <= 3 && d.result == "Punt" {
            x.three_outs += 1;
        }
        if d.red_zone {
            x.rz_trips += 1;
            x.rz_td += (d.result == "Touchdown") as u32;
            x.rz_fg += (d.result == "Field goal") as u32;
        }
    }
    a
}

/// Football Outsiders' line yards for a run.
pub fn line_yards(y: f64) -> f64 {
    if y < 0.0 { 1.2 * y } else { y.min(4.0) + 0.5 * (y.min(10.0) - 4.0).max(0.0) }
}

/// The home side's win chance before each scrimmage snap, from the game
/// page's model (`game::win_chance`, with the line where the books hung
/// one; lane NB8: it read the engine's own quick read before).
fn wp_chart(g: &GameStats, m: &super::decide::Model) -> Vec<u8> {
    let home = crate::game::win_chance::home_series(&m.p, &g.plays, m.spread);
    g.plays
        .iter()
        .zip(home)
        .filter(|(p, _)| scrimmage(p))
        .map(|(_, h)| (h.clamp(0.0, 1.0) * 100.0).round() as u8)
        .collect()
}

/// One game's tallies: its record, and (with `deep`, college and the NFL)
/// every player's lines and splits and both teams' situation counts.
/// `pos` names a player's position that day by side and id.
pub fn part(f: Facts, g: &GameStats, deep: bool, pos: &dyn Fn(usize, u32) -> Option<Position>) -> Part {
    let mut row = GameRow {
        level: f.level as u8,
        season: f.season,
        date: f.date.0,
        kind: f.kind.to_string(),
        teams: f.teams,
        group: f.group.clone(),
        neutral: f.neutral,
        conference: f.conference,
        score: g.score,
        quarters: g.quarters.clone(),
        overtime: g.overtime,
        weather: f.weather,
        boxes: g.team.clone(),
        adv: adv(g),
        ..Default::default()
    };
    let mut people = Vec::new();
    let mut sides = None;
    let lv = super::calls::model_level(f.level, f.lv);
    let model = super::decide::Model::new(lv, f.season, f.spread.map(|x| x as f64), f.neutral);
    if deep {
        row.wp = wp_chart(g, &model);
        let start = starters(g);
        for side in 0..2 {
            for (&id, l) in &g.players[side] {
                let p = pos(side, id);
                row.lines.push(Line::new(id, side as u8, start[side].contains(&id), p, &from_line(l)));
            }
        }
        // The game's own splits for each side: site, weather, opponent.
        let extra =
            [0, 1].map(|s| players::game_codes(f.level, s, f.neutral, &f.weather, f.ranked[1 - s], f.teams[1 - s]));
        people = players::game(g, &start, pos, &extra);
        sides = Some([teams::game(g, 0, lv), teams::game(g, 1, lv)]);
    }
    let about = super::calls::About { date: f.date.0, teams: f.teams, neutral: f.neutral, season: f.season };
    let calls = super::calls::game(g, &model, about, deep);
    let squads = [squads::game(g, 0, deep), squads::game(g, 1, deep)];
    Part {
        level: f.level,
        date: f.date,
        teams: f.teams,
        score: g.score,
        adv: row.adv,
        row: encode(&row),
        players: people,
        sides,
        calls,
        squads,
    }
}

/// The open season's games: their records in order, and the date and level
/// of each college and NFL game (what a delayed charting reads back).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Open {
    pub frames: Frames,
    /// (key, date, level) of the college and NFL games.
    #[serde(default)]
    pub dated: Vec<(u64, i32, u8)>,
    #[serde(default)]
    pub next: u64,
}

impl Open {
    pub fn add(&mut self, level: Level, date: Date, row: &[u8]) -> u64 {
        let key = self.next;
        self.next += 1;
        self.frames.push(key, row);
        if level != Level::Hs {
            self.dated.push((key, date.0, level as u8));
        }
        key
    }

    /// The college or NFL games played on or after `since`.
    pub fn since(&self, level: Level, since: Date) -> Vec<GameRow> {
        let keys: Vec<u64> = self.dated.iter().filter(|x| x.2 == level as u8 && x.1 >= since.0).map(|x| x.0).collect();
        let mut out = Vec::new();
        if keys.is_empty() {
            return out;
        }
        self.frames.get_many(&keys, |_, r| {
            if let Some(g) = super::file::decode::<GameRow>(r) {
                out.push(g);
            }
        });
        out
    }
}
