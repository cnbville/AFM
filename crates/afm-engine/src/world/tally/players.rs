//! The players tally (`players/<season>.tally`): each college and NFL
//! player's season, by id: games, starts, snaps, the games he played (keys
//! into the season's games file, for his game-by-game lines and the home,
//! road, opponent and weather splits) and his numbers split by down,
//! distance, field zone, quarter, the last two minutes of a half and the
//! score (item 15; Sports Reference and PFF split this way).

use std::collections::{HashMap, HashSet};

use crate::game::stats::{GameStats, PlayRecord, PlayType};
use crate::positions::Position;

use super::Level;

/// The splits, by code. Append only: files keep them by code. Codes 0-19
/// come from each snap; 20 on from the game (where it was played, the
/// weather, the opponent's ranking at kickoff); an NFL opponent's split is
/// `OPP` plus the club's index (`split_name`).
pub const SPLITS: [&str; 29] = [
    "1st down",
    "2nd down",
    "3rd down",
    "4th down",
    "short (1-3 to go)",
    "medium (4-7 to go)",
    "long (8+ to go)",
    "backed up (own 1-20)",
    "open field",
    "red zone (opp 20-1)",
    "goal to go",
    "1st quarter",
    "2nd quarter",
    "3rd quarter",
    "4th quarter",
    "overtime",
    "last two minutes of a half",
    "ahead by 9+",
    "within 8",
    "behind by 9+",
    "home",
    "road",
    "neutral site",
    "rain or snow",
    "wind 15+ mph",
    "cold (40 F or below)",
    "indoors",
    "against a ranked team",
    "against an unranked team",
];

/// An NFL opponent's split code: this plus the club's index.
pub const OPP: u8 = 100;

/// A split's name (an NFL opponent's by its index).
pub fn split_name(code: u8) -> String {
    match SPLITS.get(code as usize) {
        Some(n) => n.to_string(),
        None if code >= OPP => format!("against club {}", code - OPP),
        None => format!("split {code}"),
    }
}

/// The game's own split codes for one side: where it was played, the
/// weather and the opponent (college: ranked in the latest poll at
/// kickoff; the NFL: the opponent's club).
pub fn game_codes(
    level: Level,
    side: usize,
    neutral: bool,
    w: &super::games::Weather,
    ranked: bool,
    opp: u32,
) -> Vec<u8> {
    let mut c = vec![if neutral {
        22
    } else if side == 0 {
        20
    } else {
        21
    }];
    if w.indoor {
        c.push(26);
    } else {
        if w.rain || w.snow {
            c.push(23);
        }
        if w.wind >= 15.0 {
            c.push(24);
        }
        if w.temp <= 40.0 {
            c.push(25);
        }
    }
    match level {
        Level::College => c.push(if ranked { 27 } else { 28 }),
        Level::Nfl if opp < (u8::MAX - OPP) as u32 => c.push(OPP + opp as u8),
        _ => {}
    }
    c
}

/// The passer's numbers in a split. Append only.
pub const PASS: [&str; 11] =
    ["dropbacks", "att", "cmp", "yds", "td", "int", "sacked", "epa", "success", "first", "scramble_epa"];
/// The ball carrier's (designed runs and scrambles).
pub const RUSH: [&str; 6] = ["att", "yds", "td", "epa", "success", "first"];
/// The target's.
pub const RECV: [&str; 6] = ["targets", "rec", "yds", "td", "epa", "first"];
/// A defender's: tackles (each man in on one), missed tackles, pressures,
/// sacks (half for a shared one), targets, catches and yards allowed in
/// coverage, passes broken up, interceptions.
pub const DEF: [&str; 9] = ["tackles", "missed", "pressures", "sacks", "targets", "cmp", "yds", "pbu", "int"];

/// One player's splits: per role, the codes he has numbers in.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Splits {
    pub pass: Vec<(u8, [f32; 11])>,
    pub rush: Vec<(u8, [f32; 6])>,
    pub recv: Vec<(u8, [f32; 6])>,
    pub def: Vec<(u8, [f32; 9])>,
}

fn cell<const N: usize>(v: &mut Vec<(u8, [f32; N])>, code: u8) -> &mut [f32; N] {
    let k = match v.iter().position(|x| x.0 == code) {
        Some(k) => k,
        None => {
            v.push((code, [0.0; N]));
            v.len() - 1
        }
    };
    &mut v[k].1
}

fn add<const N: usize>(into: &mut Vec<(u8, [f32; N])>, from: &[(u8, [f32; N])]) {
    for (code, x) in from {
        let c = cell(into, *code);
        for (a, b) in c.iter_mut().zip(x) {
            *a += b;
        }
    }
}

impl Splits {
    pub fn add(&mut self, o: &Splits) {
        add(&mut self.pass, &o.pass);
        add(&mut self.rush, &o.rush);
        add(&mut self.recv, &o.recv);
        add(&mut self.def, &o.def);
    }
    fn sort(&mut self) {
        self.pass.sort_by_key(|x| x.0);
        self.rush.sort_by_key(|x| x.0);
        self.recv.sort_by_key(|x| x.0);
        self.def.sort_by_key(|x| x.0);
    }
}

/// The split codes a snap counts under, for the side `view` (the score is
/// read from its side).
pub fn codes(p: &PlayRecord, view: usize) -> Vec<u8> {
    let mut c = Vec::with_capacity(6);
    if (1..=4).contains(&p.down) {
        c.push(p.down - 1);
        c.push(match p.togo {
            t if t <= 3.0 => 4,
            t if t <= 7.0 => 5,
            _ => 6,
        });
    }
    c.push(if p.togo >= p.yardline_100 - 0.01 && p.yardline_100 <= 10.0 {
        10
    } else if p.yardline_100 <= 20.0 {
        9
    } else if p.yardline_100 >= 80.0 {
        7
    } else {
        8
    });
    c.push(if p.quarter >= 5 { 15 } else { 10 + p.quarter.max(1) });
    if matches!(p.quarter, 2 | 4) && p.clock <= 120.0 {
        c.push(16);
    }
    let v = view.min(1);
    let margin = p.score[v] - p.score[1 - v];
    c.push(match margin {
        m if m >= 9 => 17,
        m if m <= -9 => 19,
        _ => 18,
    });
    c
}

/// One player's game, built on the core that played it.
#[derive(Clone, Debug, Default)]
pub struct Game {
    pub id: u32,
    pub side: u8,
    pub pos: Option<Position>,
    pub played: bool,
    pub started: bool,
    pub snaps: [u32; 3],
    pub splits: Splits,
    /// His depth counts (`depth`), sparse by code.
    pub depth: super::depth::Depth,
}

/// Every player's game: who played and started, his splits (each snap's
/// own, then the game's `extra` codes for each side) and his depth counts.
pub fn game(
    g: &GameStats,
    start: &[HashSet<u32>; 2],
    pos: &dyn Fn(usize, u32) -> Option<Position>,
    extra: &[Vec<u8>; 2],
) -> Vec<Game> {
    let mut m: HashMap<(usize, u32), Splits> = HashMap::new();
    let mut deep = super::depth::game(g);
    for p in g.plays.iter().filter(|p| !p.nullified) {
        let o = p.offense.min(1);
        let d = 1 - o;
        let (mut oc, mut dc) = (codes(p, o), codes(p, d));
        oc.extend_from_slice(&extra[o]);
        dc.extend_from_slice(&extra[d]);
        let mut each = |side: usize, id: u32, f: &mut dyn FnMut(&mut Splits, u8)| {
            let s = m.entry((side, id)).or_default();
            for &c in if side == o { &oc } else { &dc } {
                f(s, c);
            }
        };
        let dropback = matches!(p.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble);
        if dropback && let Some(qb) = p.passer {
            each(o, qb, &mut |s, c| {
                let x = cell(&mut s.pass, c);
                x[0] += 1.0;
                match p.kind {
                    PlayType::Pass => {
                        x[1] += 1.0;
                        if p.complete {
                            x[2] += 1.0;
                            x[3] += p.yards as f32;
                            x[4] += p.touchdown as u32 as f32;
                        }
                        x[5] += p.interception as u32 as f32;
                        x[7] += p.epa as f32;
                        x[8] += p.success() as u32 as f32;
                        x[9] += p.first_down as u32 as f32;
                    }
                    PlayType::Sack => {
                        x[6] += 1.0;
                        x[7] += p.epa as f32;
                    }
                    _ => x[10] += p.epa as f32,
                }
            });
        }
        if matches!(p.kind, PlayType::Run | PlayType::Scramble)
            && let Some(c) = p.carrier
        {
            each(o, c, &mut |s, code| {
                let x = cell(&mut s.rush, code);
                x[0] += 1.0;
                x[1] += p.yards as f32;
                x[2] += p.touchdown as u32 as f32;
                x[3] += p.epa as f32;
                x[4] += p.success() as u32 as f32;
                x[5] += p.first_down as u32 as f32;
            });
        }
        if p.kind == PlayType::Pass
            && let Some(t) = p.target
        {
            each(o, t, &mut |s, code| {
                let x = cell(&mut s.recv, code);
                x[0] += 1.0;
                if p.complete {
                    x[1] += 1.0;
                    x[2] += p.yards as f32;
                    x[3] += p.touchdown as u32 as f32;
                    x[5] += p.first_down as u32 as f32;
                }
                x[4] += p.epa as f32;
            });
        }
        if !matches!(p.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble) {
            continue;
        }
        let def = |s: &mut Splits, c: u8, k: usize, v: f32| cell(&mut s.def, c)[k] += v;
        for &id in &p.tacklers {
            each(p.side_of(id).unwrap_or(d), id, &mut |s, c| def(s, c, 0, 1.0));
        }
        for &(id, _) in &p.missed_tackles {
            each(p.side_of(id).unwrap_or(d), id, &mut |s, c| def(s, c, 1, 1.0));
        }
        let mut seen = Vec::new();
        for &id in &p.pressures {
            if !seen.contains(&id) {
                seen.push(id);
                each(d, id, &mut |s, c| def(s, c, 2, 1.0));
            }
        }
        let share = if p.sackers.len() > 1 { 0.5 } else { 1.0 };
        for &id in p.sackers.iter().take(2) {
            each(d, id, &mut |s, c| def(s, c, 3, share));
        }
        if let Some(cb) = p.defender {
            each(d, cb, &mut |s, c| {
                let x = cell(&mut s.def, c);
                x[4] += 1.0;
                if p.complete {
                    x[5] += 1.0;
                    x[6] += p.yards as f32;
                }
            });
        }
        if let Some(id) = p.pbu {
            each(d, id, &mut |s, c| def(s, c, 7, 1.0));
        }
        if let Some(id) = p.interceptor {
            each(d, id, &mut |s, c| def(s, c, 8, 1.0));
        }
    }
    let mut out = Vec::new();
    for side in 0..2 {
        for (&id, l) in &g.players[side] {
            let snaps = [l.snaps_off, l.snaps_def, l.snaps_st];
            let mut splits = m.remove(&(side, id)).unwrap_or_default();
            splits.sort();
            out.push(Game {
                id,
                side: side as u8,
                pos: pos(side, id),
                played: snaps.iter().sum::<u32>() > 0,
                started: start[side].contains(&id),
                snaps,
                splits,
                depth: deep.remove(&(side, id)).unwrap_or_default(),
            });
        }
    }
    out
}

/// A player's season.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Season {
    pub id: u32,
    /// 0 college, 1 NFL.
    pub level: u8,
    /// His last team this season (program id or NFL team index).
    pub team: u32,
    pub pos: Option<Position>,
    pub games: u16,
    pub starts: u16,
    /// Offense, defense, special teams.
    pub snaps: [u32; 3],
    /// Keys of the games he dressed for in the season's games file.
    pub played: Vec<u64>,
    pub splits: Splits,
    /// His depth counts (Data overhaul items 4-11 and 28, minor bug 44):
    /// `depth::Dc` and the grids, sparse by code.
    pub depth: super::depth::Depth,
    /// His offense and defense snaps in each game of `played`, in turn
    /// (the opponent-adjusted numbers weigh each game by them).
    pub game_snaps: Vec<[u16; 2]>,
}

/// The open season's players, by level and id.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Open {
    pub seasons: HashMap<u32, Season>,
    /// NFL seasons are their own rows: (id) for a man who played both in
    /// one football year (rare: a college player can't).
    #[serde(default)]
    pub nfl: HashMap<u32, Season>,
    /// The last few days' games' depth counts (date, level, id, counts),
    /// so a page can leave out the games your charting hasn't reached yet
    /// (item 2's delay); older ones drop off as days pass.
    #[serde(default)]
    pub recent: Vec<(i32, u8, u32, super::depth::Depth)>,
    /// The oldest date in `recent` (None: not known yet), so a game only
    /// walks `recent` when a day has dropped off (world speed lane).
    #[serde(skip)]
    oldest: Option<i32>,
}

/// How many days of games `Open::recent` keeps: the longest charting
/// delay (`world::analytics::charting`).
pub const RECENT_DAYS: i32 = 4;

impl Open {
    pub fn add(&mut self, level: Level, date: crate::calendar::Date, team: u32, key: u64, g: Game) {
        if !g.depth.is_empty() {
            let cut = date.0 - RECENT_DAYS;
            if self.oldest.is_none_or(|o| o <= cut) {
                self.recent.retain(|r| r.0 > cut);
                self.oldest = self.recent.iter().map(|r| r.0).min();
            }
            self.recent.push((date.0, level as u8, g.id, g.depth.clone()));
            self.oldest = Some(self.oldest.map_or(date.0, |o| o.min(date.0)));
        }
        let m = if level == Level::Nfl { &mut self.nfl } else { &mut self.seasons };
        let s = m.entry(g.id).or_insert_with(|| Season { id: g.id, level: level as u8, ..Default::default() });
        s.team = team;
        if g.pos.is_some() {
            s.pos = g.pos;
        }
        if g.played {
            s.games += 1;
            s.starts += g.started as u16;
        }
        for k in 0..3 {
            s.snaps[k] += g.snaps[k];
        }
        s.played.push(key);
        s.game_snaps.push([g.snaps[0].min(u16::MAX as u32) as u16, g.snaps[1].min(u16::MAX as u32) as u16]);
        s.splits.add(&g.splits);
        super::depth::add_game(&mut s.depth, &g.depth);
    }

    /// A player's depth counts from games on or after `since` (the ones
    /// your charting hasn't reached yet).
    pub fn recent_since(&self, level: Level, since: crate::calendar::Date) -> HashMap<u32, super::depth::Depth> {
        let mut out: HashMap<u32, super::depth::Depth> = HashMap::new();
        for (date, l, id, d) in &self.recent {
            if *l == level as u8 && *date >= since.0 {
                super::depth::add(out.entry(*id).or_default(), d);
            }
        }
        out
    }

    /// Every season for the file, by key (`key`).
    pub fn rows(&self) -> Vec<(u64, &Season)> {
        let mut v: Vec<(u64, &Season)> = self
            .seasons
            .values()
            .map(|s| (key(Level::College, s.id), s))
            .chain(self.nfl.values().map(|s| (key(Level::Nfl, s.id), s)))
            .collect();
        v.sort_by_key(|x| x.0);
        v
    }

    pub fn get(&self, level: Level, id: u32) -> Option<&Season> {
        if level == Level::Nfl { self.nfl.get(&id) } else { self.seasons.get(&id) }
    }
}

/// A player's season's key in the file: the level, then his id.
pub fn key(level: Level, id: u32) -> u64 {
    (level as u64) << 32 | id as u64
}
