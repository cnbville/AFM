//! Deep data (update 3, lane L20): the record store behind the Data
//! section's stat history (items 15-22). The stat book (`records.rs`) keeps
//! the college lists; on top of it this keeps, at every level:
//! - single-game marks (item 16): the best games in each division,
//!   conference, the NFL and each state (`records::Book::Game`), and each
//!   team's own;
//! - the NFL's lists: seasons, careers and games (`records::Scope::Nfl`);
//! - each team's own book (item 17): career, season and game marks in the
//!   record stats for every college program, NFL club and high school;
//! - rate records with the official minimums (item 18);
//! - the NFL's and each state's per-game averages among a season's
//!   regulars, for the era column (item 19; college uses the stat book's);
//! - each year's leader and runner-up in the NFL and each state (item 15;
//!   college's come from the stat book, which keeps every season);
//! - the records that fell this season and last, with the old holder and
//!   the day (item 21): the level books' and the college programs' and NFL
//!   clubs' own (a high school's own are not told: there are thousands);
//! - every NFL regular season's analytics numbers (item 8), so "seasons
//!   like his" reaches men who have retired;
//! - the names of record holders who have left the world, so a book keeps
//!   them after `World::compact`.
//!
//! NFL marks are the regular season's, as the league keeps them; college
//! counts its bowls and playoff games (the NCAA has since 2002) and high
//! school its playoffs (as the state associations do).
//!
//! Nothing here feeds the simulation: the store reads games and seasons and
//! writes only its own lists, so every seed plays exactly as before.

use std::collections::{BTreeMap, HashMap, HashSet};

use super::analytics;
use super::hs_year::{HS_STAT_COUNT, HsLine, HsStat};
use super::records::{Book, CONFERENCE_TOP, DIVISION_TOP, GameRef, RECORD_STATS, RecordEntry, STAT_COUNT, Scope, Stat};
use super::{Stage, World};
use crate::calendar::Date;
use crate::game::stats::{GameStats, StatLine};
use crate::positions::Position;

/// How many record stats there are (`records::RECORD_STATS`).
pub const N: usize = RECORD_STATS.len();

/// A line's record stats, in `RECORD_STATS` order.
pub type Vals = [f32; N];

/// The rate records (item 18), in `Book::RateSeason` and `RateCareer` order.
/// College's first is the NCAA's passing efficiency.
pub const RATES: [&str; 5] =
    ["passer rating", "completion percentage", "yards a carry", "yards a catch", "field goal percentage"];

/// The three levels with team books.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Tier {
    College,
    Nfl,
    Hs,
}

impl Tier {
    fn ix(self) -> usize {
        self as usize
    }
    /// How deep each team's lists go: a college program's and an NFL
    /// club's best ten, a high school's best three (memory: there are
    /// thousands of schools).
    pub fn team_top(self) -> usize {
        match self {
            Tier::Hs => 3,
            _ => 10,
        }
    }
}

pub const CAREER: u8 = 0;
pub const SEASON: u8 = 1;
pub const GAME: u8 = 2;

/// One mark in a team's book, kept small (there are a million or more).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(from = "MarkRow", into = "MarkRow")]
pub struct Mark {
    pub value: f32,
    pub id: u32,
    /// A game: the opponent's index (16 bits: there are over 14,000 high
    /// schools) and the two scores (8 bits each; `game_parts`). A career:
    /// its first season (low 16 bits) and its games (high 16 bits; zero in
    /// an older file, which kept them in `games` only).
    pub aux: u32,
    /// A game's date (`Date.0`).
    pub date: i32,
    /// The season it was set (a career's last).
    pub year: i16,
    /// Bits 0-3 the stat, 4-5 the kind (career, season, game), 6 a home
    /// game, 7 a career still being played.
    pub list: u8,
    /// Games played, up to 255 (a career's full count is `game_count`).
    pub games: u8,
}

type MarkRow = (f32, u32, u32, i32, i16, u8, u8);

impl From<MarkRow> for Mark {
    fn from(r: MarkRow) -> Mark {
        Mark { value: r.0, id: r.1, aux: r.2, date: r.3, year: r.4, list: r.5, games: r.6 }
    }
}

impl From<Mark> for MarkRow {
    fn from(m: Mark) -> MarkRow {
        (m.value, m.id, m.aux, m.date, m.year, m.list, m.games)
    }
}

impl Mark {
    pub fn kind(&self) -> u8 {
        (self.list >> 4) & 3
    }
    pub fn stat(&self) -> usize {
        (self.list & 15) as usize
    }
    pub fn home(&self) -> bool {
        self.list & 64 != 0
    }
    pub fn live(&self) -> bool {
        self.list & 128 != 0
    }
    fn key(&self) -> u8 {
        self.list & 63
    }
    /// A game's opponent and score (his team's points, then theirs).
    pub fn game_parts(&self) -> (usize, [u16; 2]) {
        ((self.aux & 0xFFFF) as usize, [((self.aux >> 16) & 0xFF) as u16, ((self.aux >> 24) & 0xFF) as u16])
    }
    pub fn first_year(&self) -> i32 {
        if self.kind() == CAREER { (self.aux & 0xFFFF) as i32 } else { self.year as i32 }
    }
    /// Games played: a career's full count (a long NFL career at one club
    /// passes 255), a season's, or one for a game.
    pub fn game_count(&self) -> u32 {
        let full = self.aux >> 16;
        if self.kind() == CAREER && full > 0 { full } else { self.games as u32 }
    }
}

/// A career mark's `aux`: its first season and its games.
pub(crate) fn career_aux(first: i32, games: u32) -> u32 {
    (first.clamp(0, 0xFFFF) as u32) | (games.min(0xFFFF) << 16)
}

fn pack_game(opp: usize, score: [u16; 2]) -> u32 {
    (opp.min(0xFFFF) as u32) | ((score[0].min(255) as u32) << 16) | ((score[1].min(255) as u32) << 24)
}

pub(crate) fn list_key(kind: u8, stat: usize) -> u8 {
    (kind << 4) | stat as u8
}

/// A team's marks for one list (kind and stat), best first.
pub fn team_list(book: &[Mark], kind: u8, stat: usize) -> &[Mark] {
    let (lo, hi) = range(book, list_key(kind, stat));
    &book[lo..hi]
}

fn range(book: &[Mark], key: u8) -> (usize, usize) {
    let lo = book.partition_point(|m| m.key() < key);
    let hi = lo + book[lo..].partition_point(|m| m.key() <= key);
    (lo, hi)
}

/// Puts a mark on a team's list if it makes the best `top`; a career mark
/// replaces his earlier one. Returns the holder it moved off first place.
pub(crate) fn place_mark(book: &mut Vec<Mark>, m: Mark, top: usize) -> Option<Mark> {
    let key = m.key();
    let (lo, hi) = range(book, key);
    let first = (lo < hi).then(|| book[lo]);
    let mut hi = hi;
    if m.kind() == CAREER
        && let Some(i) = book[lo..hi].iter().position(|x| x.id == m.id)
    {
        book.remove(lo + i);
        hi -= 1;
    }
    let pos = lo + book[lo..hi].partition_point(|x| x.value >= m.value);
    if pos - lo >= top {
        return None;
    }
    // A full list drops its last mark first, so the book never grows past
    // what it holds (a loaded book's Vec has no spare room: memory).
    if hi - lo >= top {
        book.remove(hi - 1);
    } else {
        book.reserve_exact(1);
    }
    book.insert(pos, m);
    first.filter(|f| pos == lo && f.id != m.id && f.value < m.value)
}

/// A level book's list placement: returns the entry it moved off first
/// place, if it took first place from someone else.
fn place_entry(list: &mut Vec<RecordEntry>, e: RecordEntry, top: usize, career: bool) -> Option<RecordEntry> {
    let first = list.first().cloned();
    if career {
        list.retain(|x| x.id != e.id);
    }
    let pos = list.partition_point(|x| x.value >= e.value);
    if pos >= top {
        return None;
    }
    let (id, value) = (e.id, e.value);
    list.insert(pos, e);
    list.truncate(top);
    first.filter(|f| pos == 0 && f.id != id && f.value < value)
}

/// Each year's leader and runner-up at one level in one stat.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Leader {
    pub scope: Scope,
    pub stat: u8,
    pub year: i32,
    pub first: RecordEntry,
    pub second: Option<RecordEntry>,
}

/// A record that fell (item 21).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Fell {
    pub year: i32,
    pub date: Date,
    /// A level's book, or a team's (`team`).
    pub scope: Option<Scope>,
    pub team: Option<(Tier, u32)>,
    pub book: Book,
    pub id: u32,
    pub name: String,
    /// His team (a program, club or school by the level).
    pub program: usize,
    pub value: f64,
    pub old_id: u32,
    pub old_name: String,
    pub old_value: f64,
    pub old_year: i32,
}

impl Fell {
    pub fn tier(&self) -> Tier {
        match (&self.scope, self.team) {
            (_, Some((t, _))) => t,
            (Some(Scope::Nfl), _) => Tier::Nfl,
            (Some(Scope::State(_)), _) => Tier::Hs,
            _ => Tier::College,
        }
    }
}

/// One NFL regular season's analytics numbers (item 8): the charted
/// card's values for his group (`analytics::charted`), unset ones NaN.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProRow {
    pub id: u32,
    pub year: i16,
    pub team: u8,
    pub pos: Position,
    pub games: u8,
    /// The group's volume count (`analytics::volume`).
    pub vol: f32,
    pub vals: [f32; analytics::CARD_MAX],
}

/// Names of record holders who have left the world, sorted by id.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Names {
    ids: Vec<u32>,
    ends: Vec<u32>,
    text: String,
}

impl Names {
    pub fn get(&self, id: u32) -> Option<&str> {
        let k = self.ids.binary_search(&id).ok()?;
        let start = if k == 0 { 0 } else { self.ends[k - 1] as usize };
        self.text.get(start..self.ends[k] as usize)
    }
    pub fn len(&self) -> usize {
        self.ids.len()
    }
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
    /// Keeps the names still wanted and adds the new ones.
    fn rebuild(&mut self, keep: &HashSet<u32>, add: Vec<(u32, String)>) {
        let mut all: BTreeMap<u32, String> = BTreeMap::new();
        for (k, &id) in self.ids.iter().enumerate() {
            if keep.contains(&id) {
                let start = if k == 0 { 0 } else { self.ends[k - 1] as usize };
                all.insert(id, self.text[start..self.ends[k] as usize].to_string());
            }
        }
        all.extend(add);
        let mut n = Names::default();
        for (id, name) in all {
            n.text.push_str(&name);
            n.ids.push(id);
            n.ends.push(n.text.len() as u32);
        }
        *self = n;
    }
}

/// The store (see the module notes).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Marks {
    /// Each team's own book, by index: college programs, NFL clubs and
    /// high schools. Sorted by list (kind and stat), best first.
    pub college: Vec<Vec<Mark>>,
    pub nfl: Vec<Vec<Mark>>,
    pub hs: Vec<Vec<Mark>>,
    pub leaders: Vec<Leader>,
    /// Records that fell this season and last.
    pub fell: Vec<Fell>,
    /// (year, level, position) to each record stat's per-game average
    /// among the season's regulars: the NFL is level 1000, a state's high
    /// schools 2000 and up.
    pub norms: HashMap<(i32, u16, Position), Vals>,
    pub pro_rows: Vec<ProRow>,
    /// Each NFL player's regular-season games and totals with each club.
    pub(crate) club_careers: BTreeMap<(u32, u8), (u16, Vals)>,
    pub names: Names,
    /// Records already told as fallen this season: (list, player).
    pub(crate) told: HashSet<(u64, u32)>,
    /// Each list's best mark told this season, and who has it: a record
    /// already broken this season falls again only to someone past him.
    #[serde(default)]
    pub(crate) best_told: HashMap<u64, (f64, u32)>,
    /// Each NFL player's regular season so far with each club, from the
    /// games (a man traded in the season has a line with each club):
    /// games and totals.
    #[serde(default)]
    pub(crate) club_season: BTreeMap<(u32, u8), (u16, Vals)>,
    /// Each NFL player's first season with each club (a stint split off
    /// by a trade leaves no season row with that club).
    #[serde(default)]
    pub(crate) club_first: BTreeMap<(u32, u8), i16>,
    /// Each level's first season in the books (a fallen record counts
    /// once the books have five seasons behind them).
    pub first: [i32; 3],
}

const NFL_LEVEL: u16 = 1000;
const HS_LEVEL: u16 = 2000;

/// The era column's number: what he did against what the average regular
/// at his position would have done in the same games (100 = average).
pub fn era(value: f64, expected: f64) -> f32 {
    if expected > 0.0 { (100.0 * value / expected) as f32 } else { 0.0 }
}

impl Marks {
    pub fn team_book(&self, tier: Tier, team: usize) -> &[Mark] {
        let v = match tier {
            Tier::College => &self.college,
            Tier::Nfl => &self.nfl,
            Tier::Hs => &self.hs,
        };
        v.get(team).map_or(&[], |b| b.as_slice())
    }
    fn book_mut(&mut self, tier: Tier, team: usize) -> &mut Vec<Mark> {
        let v = match tier {
            Tier::College => &mut self.college,
            Tier::Nfl => &mut self.nfl,
            Tier::Hs => &mut self.hs,
        };
        if v.len() <= team {
            v.resize_with(team + 1, Vec::new);
        }
        &mut v[team]
    }
    /// A level's per-game norm for a stat: the NFL's or a state's.
    pub fn norm(&self, year: i32, level: u16, pos: Position, stat: usize) -> f64 {
        self.norms.get(&(year, level, pos)).map_or(0.0, |m| m[stat] as f64)
    }
    fn established(&self, tier: Tier, year: i32) -> bool {
        let f = self.first[tier.ix()];
        f != 0 && year >= f + 5
    }
}

/// The NFL's norm level and a state's.
pub fn nfl_level() -> u16 {
    NFL_LEVEL
}
pub fn state_level(state: usize) -> u16 {
    HS_LEVEL + state as u16
}

/// A line's record stats.
pub fn vals(l: &StatLine) -> Vals {
    let all = super::records::from_line(l);
    RECORD_STATS.map(|s| all[s as usize])
}

/// A season's counts (the stat book's array) as record stats.
pub fn vals_of(all: &[f32; STAT_COUNT]) -> Vals {
    RECORD_STATS.map(|s| all[s as usize])
}

/// A high school line's record stats (no return touchdowns are kept).
pub fn hs_vals(s: &[i32; HS_STAT_COUNT]) -> Vals {
    let mut v = [0f32; N];
    for &(k, j) in super::hs_year::HS_BOOKS.iter() {
        let x = s[k as usize] as f32;
        v[j] = if matches!(k, HsStat::Tfl | HsStat::Sacks) { x / 2.0 } else { x };
    }
    v
}

/// The counts a rate is worked out from: attempts, completions, passing
/// yards, touchdown passes, interceptions, carries, rushing yards, catches,
/// receiving yards, field goal tries, field goals.
pub type Parts = [f64; 11];

pub fn parts_of(get: &dyn Fn(Stat) -> f64) -> Parts {
    use Stat as S;
    [
        get(S::Att),
        get(S::Cmp),
        get(S::PassYds),
        get(S::PassTd),
        get(S::Int),
        get(S::Rush),
        get(S::RushYds),
        get(S::Rec),
        get(S::RecYds),
        get(S::FgAtt),
        get(S::FgMade),
    ]
}

pub fn hs_parts(s: &[i32; HS_STAT_COUNT]) -> Parts {
    let g = |k: HsStat| s[k as usize] as f64;
    [
        g(HsStat::Att),
        g(HsStat::Cmp),
        g(HsStat::PassYds),
        g(HsStat::PassTd),
        g(HsStat::Int),
        g(HsStat::Rush),
        g(HsStat::RushYds),
        g(HsStat::Rec),
        g(HsStat::RecYds),
        g(HsStat::FgAtt),
        g(HsStat::FgMade),
    ]
}

fn add_parts(a: &mut Parts, b: &Parts) {
    for (x, y) in a.iter_mut().zip(b) {
        *x += y;
    }
}

/// A rate's value from its counts. College's passer rating is the NCAA's
/// passing efficiency.
pub fn rate(k: usize, p: &Parts, tier: Tier) -> Option<f64> {
    let r = |n: f64, d: f64| (d > 0.0).then(|| n / d);
    match k {
        0 if tier == Tier::College => analytics::passing_efficiency(p[0], p[1], p[2], p[3], p[4]),
        0 => analytics::passer_rating(p[0], p[1], p[2], p[3], p[4]),
        1 => r(100.0 * p[1], p[0]),
        2 => r(p[6], p[5]),
        3 => r(p[8], p[7]),
        _ => r(100.0 * p[10], p[9]),
    }
}

/// Whether a season qualifies for a rate record: the NFL's official
/// minimums a team game (14 attempts, 6.25 carries, 1.875 catches, one
/// field goal try). College and high school use the NCAA's 15 attempts a
/// game for passing and the NFL's per-game standards for the rest
/// (proposed, not confirmed: the NCAA book's own minimums couldn't be read
/// for this build).
pub fn season_qualifies(k: usize, p: &Parts, tier: Tier, team_games: u32) -> bool {
    let g = team_games.max(1) as f64;
    let per = if tier == Tier::Nfl { 14.0 } else { 15.0 };
    match k {
        0 | 1 => p[0] >= per * g,
        2 => p[5] >= 6.25 * g,
        3 => p[7] >= 1.875 * g,
        _ => p[9] >= g,
    }
}

/// Whether a career qualifies: the NFL's official 1,500 attempts, 750
/// carries, 200 catches and 100 field goals; college the NCAA's 325
/// completions for passing and 300 carries, with 100 catches and 40 field
/// goal tries; high school half the college marks (both proposed, not
/// confirmed where the NCAA's aren't known).
pub fn career_qualifies(k: usize, p: &Parts, tier: Tier) -> bool {
    let s = if tier == Tier::Hs { 0.5 } else { 1.0 };
    match (tier, k) {
        (Tier::Nfl, 0 | 1) => p[0] >= 1500.0,
        (Tier::Nfl, 2) => p[5] >= 750.0,
        (Tier::Nfl, 3) => p[7] >= 200.0,
        (Tier::Nfl, _) => p[10] >= 100.0,
        (_, 0 | 1) => p[1] >= 325.0 * s,
        (_, 2) => p[5] >= 300.0 * s,
        (_, 3) => p[7] >= 100.0 * s,
        (_, _) => p[9] >= 40.0 * s,
    }
}

/// A stable number for a list, for the "already told" set.
fn list_code(scope: Option<&Scope>, team: Option<(Tier, u32)>, book: Book) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |b: &[u8]| {
        for &x in b {
            h ^= x as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    match scope {
        Some(Scope::Division(d)) => eat(&[1, *d as u8]),
        Some(Scope::Conference(d, c)) => {
            eat(&[2, *d as u8]);
            eat(c.as_bytes());
        }
        Some(Scope::State(s)) => eat(&[3, (*s % 256) as u8, (*s / 256) as u8]),
        Some(Scope::Nfl) => eat(&[4]),
        None => {}
    }
    if let Some((t, i)) = team {
        eat(&[5, t as u8]);
        eat(&i.to_le_bytes());
    }
    let (k, j) = match book {
        Book::Season(j) => (1u8, j),
        Book::Career(j) => (2, j),
        Book::Game(j) => (3, j),
        Book::RateSeason(j) => (4, j),
        Book::RateCareer(j) => (5, j),
        Book::Streak(j) => (6, j),
        Book::WinStreak => (7, 0),
    };
    eat(&[k, j as u8]);
    h
}

fn person(w: &World, id: u32) -> Option<usize> {
    w.people.binary_search_by_key(&id, |p| p.player.id).ok()
}

/// A record holder's name: from the people still in the world, the store's
/// own names, or the stat book's honors.
pub fn name_of(w: &World, id: u32) -> Option<String> {
    if let Some(k) = person(w, id) {
        return Some(w.people[k].name.clone());
    }
    if let Some(n) = w.marks.names.get(id) {
        return Some(n.to_string());
    }
    w.stats.honors.get(&id).map(|h| h.name.clone())
}

/// One player's line in a game, for the game marks.
struct GameLine {
    id: u32,
    name: String,
    pos: Option<Position>,
    v: Vals,
}

/// What a game's side carries for the marks.
struct GameSide {
    tier: Tier,
    team: usize,
    opp: usize,
    home: bool,
    score: [u16; 2],
    /// The level books this side's games enter, with their depth.
    scopes: Vec<(Scope, usize)>,
}

/// A game's marks for one side: each line into the level's single-game
/// lists and the team's own.
fn game_marks(w: &mut World, year: i32, date: Date, side: &GameSide, lines: &[GameLine]) {
    // Each list's bar: anything over it gets a look.
    let cuts: Vec<Vals> = side
        .scopes
        .iter()
        .map(|(sc, top)| {
            let mut c = [0f32; N];
            for (j, x) in c.iter_mut().enumerate() {
                if let Some(l) = w.stats.books.get(&(sc.clone(), Book::Game(j)))
                    && l.len() >= *top
                {
                    *x = l.last().map_or(0.0, |e| e.value as f32);
                }
            }
            c
        })
        .collect();
    let established = w.marks.established(side.tier, year);
    let top = side.tier.team_top();
    for l in lines {
        for j in 0..N {
            let v = l.v[j];
            if v <= 0.0 {
                continue;
            }
            for (k, (sc, depth)) in side.scopes.iter().enumerate() {
                if v <= cuts[k][j] {
                    continue;
                }
                let e = RecordEntry {
                    value: v as f64,
                    id: l.id,
                    name: l.name.clone(),
                    program: side.team,
                    year,
                    from: year,
                    live: false,
                    games: 1,
                    pos: l.pos,
                    era: 0.0,
                    game: Some(GameRef { date, opp: side.opp, home: side.home, score: side.score }),
                };
                let list = w.stats.books.entry((sc.clone(), Book::Game(j))).or_default();
                if let Some(old) = place_entry(list, e, *depth, false)
                    && established
                {
                    tell(w, year, date, Some(sc.clone()), None, Book::Game(j), l, side.team, v as f64, &old);
                }
            }
            let m = Mark {
                value: v,
                id: l.id,
                aux: pack_game(side.opp, side.score),
                date: date.0,
                year: year as i16,
                list: list_key(GAME, j) | if side.home { 64 } else { 0 },
                games: 1,
            };
            let book = w.marks.book_mut(side.tier, side.team);
            // A high school's own records falling are told only at the
            // state level (thousands of schools: memory).
            if let Some(old) = place_mark(book, m, top)
                && established
                && side.tier != Tier::Hs
            {
                let old = entry_of(w, &old);
                tell(
                    w,
                    year,
                    date,
                    None,
                    Some((side.tier, side.team as u32)),
                    Book::Game(j),
                    l,
                    side.team,
                    v as f64,
                    &old,
                );
            }
        }
    }
}

/// A team mark as a book entry, with its holder's name.
fn entry_of(w: &World, m: &Mark) -> RecordEntry {
    RecordEntry {
        value: m.value as f64,
        id: m.id,
        name: name_of(w, m.id).unwrap_or_else(|| "a former player".into()),
        year: m.year as i32,
        from: m.first_year(),
        ..Default::default()
    }
}

/// A record fell: kept for the stat history's "fell this season".
#[allow(clippy::too_many_arguments)]
fn tell(
    w: &mut World,
    year: i32,
    date: Date,
    scope: Option<Scope>,
    team: Option<(Tier, u32)>,
    book: Book,
    l: &GameLine,
    program: usize,
    value: f64,
    old: &RecordEntry,
) {
    let code = list_code(scope.as_ref(), team, book);
    // Broken already this season: he has to pass the new mark, and its
    // holder is the one he takes it from.
    let mut old = old.clone();
    if let Some(&(best, by)) = w.marks.best_told.get(&code) {
        if by == l.id || value <= best {
            if by == l.id && value > best {
                w.marks.best_told.insert(code, (value, by));
            }
            return;
        }
        // The holder's own total may have passed the told mark since.
        if best > old.value {
            old = RecordEntry {
                value: best,
                id: by,
                name: name_of(w, by).unwrap_or_else(|| "a former player".into()),
                year,
                from: year,
                ..Default::default()
            };
        }
    }
    w.marks.best_told.insert(code, (value, l.id));
    if !w.marks.told.insert((code, l.id)) {
        return;
    }
    w.marks.fell.push(Fell {
        year,
        date,
        scope,
        team,
        book,
        id: l.id,
        name: l.name.clone(),
        program,
        value,
        old_id: old.id,
        old_name: old.name.clone(),
        old_value: old.value,
        old_year: old.year,
    });
}

/// Running season and career totals checked after a game against each
/// list's holder (season and career records fall on the day, item 21).
struct Running {
    line: GameLine,
    /// The season so far (at the level and with this team), his career at
    /// the level and at the team.
    season: Vals,
    team_season: Vals,
    career: Vals,
    team_career: Vals,
}

/// A career holder's season so far where it adds to his record: the
/// level's career in `scope`, or his career with `team`.
fn holder_run(w: &World, tier: Tier, id: u32, scope: Option<&Scope>, team: Option<usize>) -> Vals {
    let zero = [0f32; N];
    match tier {
        Tier::College => {
            let Some(b) = w.college.as_ref().and_then(|cs| cs.book.get(&id)) else { return zero };
            let counts = match (team, scope) {
                (Some(t), _) => b.program == t,
                (None, Some(Scope::Division(d))) => w.programs.get(b.program).is_some_and(|p| p.division == *d),
                _ => false,
            };
            if counts { vals(&b.line) } else { zero }
        }
        Tier::Nfl => match team {
            Some(t) => w.marks.club_season.get(&(id, t as u8)).map_or(zero, |x| x.1),
            None => person(w, id)
                .and_then(|k| w.people[k].pro.as_ref()?.seasons.iter().rev().find(|s| s.year == w.year))
                .map_or(zero, |s| vals(&s.stats)),
        },
        Tier::Hs => {
            let Some((school, acc)) = person(w, id).and_then(|k| super::hs_year::running(w, k)) else { return zero };
            let counts = match (team, scope) {
                (Some(t), _) => school == t,
                (None, Some(Scope::State(st))) => w.high_schools.get(school).is_some_and(|h| h.state == *st),
                _ => false,
            };
            if counts { hs_vals(&acc) } else { zero }
        }
    }
}

fn check_running(w: &mut World, year: i32, date: Date, side: &GameSide, runs: &[Running]) {
    if !w.marks.established(side.tier, year) {
        return;
    }
    // The holders' marks: each level list's first, then the team's.
    type Holders = [Option<RecordEntry>; N];
    let level: Vec<(Scope, Holders, Holders)> =
        side.scopes
            .iter()
            .map(|(sc, _)| {
                let first = |b: Book| w.stats.books.get(&(sc.clone(), b)).and_then(|l| l.first().cloned());
                (
                    sc.clone(),
                    std::array::from_fn(|j| first(Book::Season(j))),
                    // Conference books keep seasons only.
                    std::array::from_fn(|j| {
                        if matches!(sc, Scope::Conference(..)) { None } else { first(Book::Career(j)) }
                    }),
                )
            })
            .collect();
    let book = w.marks.team_book(side.tier, side.team).to_vec();
    let team_first = |kind: u8, j: usize| team_list(&book, kind, j).first().copied();
    for r in runs {
        for j in 0..N {
            for (sc, season, career) in &level {
                for (have, holder, b) in
                    [(r.season[j], &season[j], Book::Season(j)), (r.career[j], &career[j], Book::Career(j))]
                {
                    if let Some(h) = holder
                        && h.id != r.line.id
                        && have as f64 > h.value
                    {
                        // A holder still playing has added to his career.
                        let mut old = h.clone();
                        if matches!(b, Book::Career(_)) {
                            old.value += holder_run(w, side.tier, h.id, Some(sc), None)[j] as f64;
                        }
                        if have as f64 > old.value {
                            tell(w, year, date, Some(sc.clone()), None, b, &r.line, side.team, have as f64, &old);
                        }
                    }
                }
            }
            if side.tier == Tier::Hs {
                continue;
            }
            for (have, kind, b) in
                [(r.team_season[j], SEASON, Book::Season(j)), (r.team_career[j], CAREER, Book::Career(j))]
            {
                if let Some(h) = team_first(kind, j)
                    && h.id != r.line.id
                    && have > h.value
                {
                    let mut old = entry_of(w, &h);
                    if kind == CAREER {
                        old.value += holder_run(w, side.tier, h.id, None, Some(side.team))[j] as f64;
                    }
                    if have as f64 <= old.value {
                        continue;
                    }
                    tell(
                        w,
                        year,
                        date,
                        None,
                        Some((side.tier, side.team as u32)),
                        b,
                        &r.line,
                        side.team,
                        have as f64,
                        &old,
                    );
                }
            }
        }
    }
}

fn add(a: &mut Vals, b: &Vals) {
    for (x, y) in a.iter_mut().zip(b) {
        *x += y;
    }
}

fn college_scopes(w: &World, prog: usize) -> Vec<(Scope, usize)> {
    let p = &w.programs[prog];
    let mut v = vec![(Scope::Division(p.division), DIVISION_TOP)];
    if !p.conference.is_empty() {
        v.push((Scope::Conference(p.division, p.conference.clone()), CONFERENCE_TOP));
    }
    v
}

fn score_of(s: &GameStats, side: usize) -> [u16; 2] {
    [s.score[side].max(0) as u16, s.score[1 - side].max(0) as u16]
}

/// A college game on the engine: its marks, and the season and career
/// records it broke. Bowls and playoff games count. At a neutral site
/// neither side was at home (both read "vs").
pub(crate) fn college_game(w: &mut World, year: i32, teams: [usize; 2], s: &GameStats, neutral: bool) {
    if !w.recording {
        return;
    }
    let date = w.today;
    if w.marks.first[Tier::College.ix()] == 0 {
        w.marks.first[Tier::College.ix()] = year;
    }
    for side in 0..2 {
        let prog = teams[side];
        let gs = GameSide {
            tier: Tier::College,
            team: prog,
            opp: teams[1 - side],
            home: side == 0 || neutral,
            score: score_of(s, side),
            scopes: college_scopes(w, prog),
        };
        let div = w.programs[prog].division;
        let mut ids: Vec<u32> = s.players[side].keys().copied().collect();
        ids.sort_unstable();
        let mut lines = Vec::new();
        let mut runs = Vec::new();
        for id in ids {
            let l = &s.players[side][&id];
            let v = vals(l);
            if v.iter().all(|&x| x <= 0.0) {
                continue;
            }
            let k = person(w, id);
            let line = GameLine {
                id,
                name: k.map_or_else(|| "a player".into(), |k| w.people[k].name.clone()),
                pos: k.map(|k| w.people[k].player.position),
                v,
            };
            // The season so far is on the season's book (this game in it).
            if let Some(b) = w.college.as_ref().and_then(|cs| cs.book.get(&id)) {
                let season = vals(&b.line);
                let mut career = season;
                let mut team_career = season;
                for c in w.stats.career(id) {
                    let cv = vals_of(&c.s.to_array());
                    if c.division == div {
                        add(&mut career, &cv);
                    }
                    if c.program as usize == prog {
                        add(&mut team_career, &cv);
                    }
                }
                runs.push(Running {
                    line: GameLine { id, name: line.name.clone(), pos: line.pos, v },
                    season,
                    team_season: season,
                    career,
                    team_career,
                });
            }
            lines.push(line);
        }
        game_marks(w, year, date, &gs, &lines);
        check_running(w, year, date, &gs, &runs);
    }
}

/// An NFL regular-season game: its marks and the records it broke.
pub(crate) fn nfl_game(w: &mut World, teams: [usize; 2], s: &GameStats, neutral: bool) {
    if !w.recording || w.nfl_season.as_ref().is_some_and(|x| x.booked) {
        return;
    }
    let year = w.year;
    let date = w.today;
    if w.marks.first[Tier::Nfl.ix()] == 0 {
        w.marks.first[Tier::Nfl.ix()] = year;
    }
    for side in 0..2 {
        let club = teams[side];
        let gs = GameSide {
            tier: Tier::Nfl,
            team: club,
            opp: teams[1 - side],
            home: side == 0 || neutral,
            score: score_of(s, side),
            scopes: vec![(Scope::Nfl, DIVISION_TOP)],
        };
        let mut ids: Vec<u32> = s.players[side].keys().copied().collect();
        ids.sort_unstable();
        let mut lines = Vec::new();
        let mut runs = Vec::new();
        for id in ids {
            let l = &s.players[side][&id];
            let v = vals(l);
            let Some(k) = person(w, id) else { continue };
            // His season with this club, from its games (a man traded in
            // the season has a line with each club).
            let team_season = {
                let e = w.marks.club_season.entry((id, club as u8)).or_insert((0, [0.0; N]));
                if l.snaps_off + l.snaps_def + l.snaps_st > 0 {
                    e.0 = e.0.saturating_add(1);
                }
                add(&mut e.1, &v);
                e.1
            };
            if v.iter().all(|&x| x <= 0.0) {
                continue;
            }
            let p = &w.people[k];
            let line = GameLine { id, name: p.name.clone(), pos: Some(p.player.position), v };
            if let Some(season) = p.pro.as_ref().and_then(|c| c.seasons.iter().rev().find(|x| x.year == year)) {
                let season = vals(&season.stats);
                let mut career = season;
                if let Some(c) = w.nfl_book.careers.get(&id) {
                    add(&mut career, &vals(c));
                }
                let mut team_career = team_season;
                if let Some((_, c)) = w.marks.club_careers.get(&(id, club as u8)) {
                    add(&mut team_career, c);
                }
                runs.push(Running {
                    line: GameLine { id, name: line.name.clone(), pos: line.pos, v },
                    season,
                    team_season,
                    career,
                    team_career,
                });
            }
            lines.push(line);
        }
        game_marks(w, year, date, &gs, &lines);
        check_running(w, year, date, &gs, &runs);
    }
}

/// A high school game on the engine: its marks and the records it broke.
/// `lines`: each side's (person index, counts, played). Playoff games count;
/// a final at a neutral site has neither side at home.
pub(crate) fn hs_game(
    w: &mut World,
    teams: [usize; 2],
    score: [u32; 2],
    lines: &[Vec<(usize, [i32; HS_STAT_COUNT], bool)>; 2],
    neutral: bool,
) {
    if !w.recording {
        return;
    }
    let year = w.year;
    let date = w.today;
    if w.marks.first[Tier::Hs.ix()] == 0 {
        w.marks.first[Tier::Hs.ix()] = year;
    }
    for side in 0..2 {
        let school = teams[side];
        let state = w.high_schools[school].state;
        let gs = GameSide {
            tier: Tier::Hs,
            team: school,
            opp: teams[1 - side],
            home: side == 0 || neutral,
            score: [score[side].min(u16::MAX as u32) as u16, score[1 - side].min(u16::MAX as u32) as u16],
            scopes: vec![(Scope::State(state), DIVISION_TOP)],
        };
        let mut out = Vec::new();
        let mut runs = Vec::new();
        for &(i, c, _) in &lines[side] {
            let v = hs_vals(&c);
            if v.iter().all(|&x| x <= 0.0) {
                continue;
            }
            let p = &w.people[i];
            let id = p.player.id;
            let line = GameLine { id, name: p.name.clone(), pos: Some(p.player.position), v };
            if let Some((_, acc)) = super::hs_year::running(w, i) {
                let season = hs_vals(&acc);
                let mut career = season;
                let mut team_career = season;
                for l in &p.hs_lines {
                    let lv = hs_vals(&l.s.map(|x| x as i32));
                    if w.high_schools.get(l.school as usize).is_some_and(|h| h.state == state) {
                        add(&mut career, &lv);
                    }
                    if l.school as usize == school {
                        add(&mut team_career, &lv);
                    }
                }
                runs.push(Running {
                    line: GameLine { id, name: line.name.clone(), pos: line.pos, v },
                    season,
                    team_season: season,
                    career,
                    team_career,
                });
            }
            out.push(line);
        }
        game_marks(w, year, date, &gs, &out);
        check_running(w, year, date, &gs, &runs);
    }
}

/// The season's closing lists for one level book: its seasons by stat,
/// its leaders and its season rate records.
struct SeasonRow {
    id: u32,
    name: String,
    pos: Position,
    team: usize,
    games: u32,
    v: Vals,
    /// The era column's expected values for the season.
    expect: Vals,
    parts: Parts,
    team_games: u32,
}

fn leaders_of(w: &mut World, scope: &Scope, year: i32, rows: &[SeasonRow]) {
    for j in 0..N {
        let mut best: Vec<&SeasonRow> = rows.iter().filter(|r| r.v[j] > 0.0).collect();
        best.sort_by(|a, b| b.v[j].total_cmp(&a.v[j]).then(a.id.cmp(&b.id)));
        let entry = |r: &SeasonRow| RecordEntry {
            value: r.v[j] as f64,
            id: r.id,
            name: r.name.clone(),
            program: r.team,
            year,
            from: year,
            games: r.games.min(u16::MAX as u32) as u16,
            pos: Some(r.pos),
            era: era(r.v[j] as f64, r.expect[j] as f64),
            ..Default::default()
        };
        if let Some(first) = best.first() {
            w.marks.leaders.retain(|l| !(l.scope == *scope && l.stat == j as u8 && l.year == year));
            w.marks.leaders.push(Leader {
                scope: scope.clone(),
                stat: j as u8,
                year,
                first: entry(first),
                second: best.get(1).map(|r| entry(r)),
            });
        }
    }
}

/// Season rate records for a level book.
fn season_rates<'a>(w: &mut World, scope: &Scope, tier: Tier, year: i32, rows: impl Iterator<Item = &'a SeasonRow>) {
    for r in rows {
        for k in 0..RATES.len() {
            if !season_qualifies(k, &r.parts, tier, r.team_games) {
                continue;
            }
            let Some(v) = rate(k, &r.parts, tier) else { continue };
            let e = RecordEntry {
                value: v,
                id: r.id,
                name: r.name.clone(),
                program: r.team,
                year,
                from: year,
                games: r.games.min(u16::MAX as u32) as u16,
                pos: Some(r.pos),
                ..Default::default()
            };
            let list = w.stats.books.entry((scope.clone(), Book::RateSeason(k))).or_default();
            place_entry(list, e, DIVISION_TOP, false);
        }
    }
}

/// A career's rate records (the career's counts at the level).
#[allow(clippy::too_many_arguments)]
fn career_rates(
    w: &mut World,
    scope: &Scope,
    tier: Tier,
    row: &SeasonRow,
    parts: &Parts,
    from: i32,
    live: bool,
    games: u32,
) {
    for k in 0..RATES.len() {
        if !career_qualifies(k, parts, tier) {
            continue;
        }
        let Some(v) = rate(k, parts, tier) else { continue };
        let e = RecordEntry {
            value: v,
            id: row.id,
            name: row.name.clone(),
            program: row.team,
            year: w.year.max(from),
            from,
            live,
            games: games.min(u16::MAX as u32) as u16,
            pos: Some(row.pos),
            ..Default::default()
        };
        let list = w.stats.books.entry((scope.clone(), Book::RateCareer(k))).or_default();
        place_entry(list, e, DIVISION_TOP, true);
    }
}

/// The college season's close (after the stat book's own books): each
/// program's book, the division leaders and the rate records.
pub(crate) fn college_close(w: &mut World, year: i32, start: usize) {
    if !w.recording {
        return;
    }
    let n = w.stats.seasons.len();
    if w.marks.first[Tier::College.ix()] == 0 && start < n {
        w.marks.first[Tier::College.ix()] = year;
    }
    // The season's rows, and which of them each division and conference
    // book takes (a conference's rate records too).
    let mut rows: Vec<SeasonRow> = Vec::new();
    let mut by_scope: BTreeMap<Scope, Vec<usize>> = BTreeMap::new();
    let top = Tier::College.team_top();
    for k in start..n {
        let r = w.stats.seasons[k].clone();
        let prog = r.program as usize;
        let Some(name) = name_of(w, r.id) else { continue };
        let k_person = person(w, r.id);
        let still_here =
            k_person.is_some_and(|i| w.people[i].stage == Stage::College && w.people[i].college == Some(prog));
        let still_level = k_person.is_some_and(|i| w.people[i].stage == Stage::College);
        let v = vals_of(&r.s.to_array());
        let get = |s: Stat| r.get(s);
        let parts = parts_of(&get);
        let mut expect = [0f32; N];
        for (j, &s) in RECORD_STATS.iter().enumerate() {
            expect[j] = (super::records::norm_of(w, &r, s) * r.games as f64) as f32;
        }
        // His career at the program and at the level.
        let career: Vec<super::records::SeasonRecord> = w.stats.career(r.id).into_iter().cloned().collect();
        let mut at_prog = [0f32; N];
        let mut first_at = year;
        let mut games_at = 0u32;
        let mut div_parts = [0f64; 11];
        let mut div_games = 0u32;
        let mut div_from = year;
        for c in &career {
            if c.program as usize == prog {
                add(&mut at_prog, &vals_of(&c.s.to_array()));
                first_at = first_at.min(c.year as i32);
                games_at += c.games as u32;
            }
            if c.division == r.division {
                add_parts(&mut div_parts, &parts_of(&|s| c.get(s)));
                div_games += c.games as u32;
                div_from = div_from.min(c.year as i32);
            }
        }
        let book = w.marks.book_mut(Tier::College, prog);
        for j in 0..N {
            if v[j] > 0.0 {
                let m = Mark {
                    value: v[j],
                    id: r.id,
                    aux: 0,
                    date: 0,
                    year: year as i16,
                    list: list_key(SEASON, j),
                    games: r.games,
                };
                place_mark(book, m, top);
            }
            if at_prog[j] > 0.0 {
                let m = Mark {
                    value: at_prog[j],
                    id: r.id,
                    aux: career_aux(first_at, games_at),
                    date: 0,
                    year: year as i16,
                    list: list_key(CAREER, j) | if still_here { 128 } else { 0 },
                    games: games_at.min(255) as u8,
                };
                place_mark(book, m, top);
            }
        }
        // The team's games with its bowl and playoff games, as his counts
        // have them (the program's season still holds the regular season's).
        let team_games = w.college.as_ref().and_then(|cs| cs.results.get(prog)).map_or(12, |t| t.games());
        let row =
            SeasonRow { id: r.id, name, pos: r.pos, team: prog, games: r.games as u32, v, expect, parts, team_games };
        career_rates(
            w,
            &Scope::Division(r.division),
            Tier::College,
            &row,
            &div_parts,
            div_from,
            still_level,
            div_games,
        );
        by_scope.entry(Scope::Division(r.division)).or_default().push(rows.len());
        let conf = &w.programs[prog].conference;
        if !conf.is_empty() {
            by_scope.entry(Scope::Conference(r.division, conf.clone())).or_default().push(rows.len());
        }
        rows.push(row);
    }
    for (sc, ks) in &by_scope {
        // College's leaders come from the stat book, which keeps every season.
        season_rates(w, sc, Tier::College, year, ks.iter().map(|&k| &rows[k]));
        if matches!(sc, Scope::Division(_)) {
            refresh_live_lists(w, sc);
        }
    }
    refresh_live(w, Tier::College);
    end_season(w, Tier::College, year);
}

/// Whether he plays in a level book's scope now: in the NFL, at a program
/// in the division, or at a school in the state.
fn active_in(w: &World, id: u32, scope: &Scope) -> bool {
    person(w, id).is_some_and(|k| {
        let p = &w.people[k];
        match scope {
            Scope::Nfl => p.stage == Stage::Pro,
            Scope::Division(d) | Scope::Conference(d, _) => {
                p.stage == Stage::College && p.college.and_then(|c| w.programs.get(c)).is_some_and(|x| x.division == *d)
            }
            Scope::State(s) => p.stage == Stage::HighSchool && w.high_schools.get(p.hs).is_some_and(|h| h.state == *s),
        }
    })
}

/// Clears the "still being played" mark on a level book's careers that
/// are over: he retired, finished school or left the level.
fn refresh_live_lists(w: &mut World, scope: &Scope) {
    for b in (0..N).map(Book::Career).chain((0..RATES.len()).map(Book::RateCareer)) {
        let key = (scope.clone(), b);
        let Some(list) = w.stats.books.get(&key) else { continue };
        let over: Vec<usize> =
            list.iter().enumerate().filter(|(_, e)| e.live && !active_in(w, e.id, scope)).map(|(i, _)| i).collect();
        if let Some(list) = w.stats.books.get_mut(&key) {
            for i in over {
                list[i].live = false;
            }
        }
    }
}

/// Clears the "still playing" mark on careers that are over.
fn refresh_live(w: &mut World, tier: Tier) {
    let teams = match tier {
        Tier::College => w.marks.college.len(),
        Tier::Nfl => w.marks.nfl.len(),
        Tier::Hs => w.marks.hs.len(),
    };
    for t in 0..teams {
        let ids: Vec<(usize, u32)> =
            w.marks.team_book(tier, t).iter().enumerate().filter(|(_, m)| m.live()).map(|(i, m)| (i, m.id)).collect();
        for (i, id) in ids {
            let here = person(w, id).is_some_and(|k| {
                let p = &w.people[k];
                match tier {
                    Tier::College => p.stage == Stage::College && p.college == Some(t),
                    Tier::Nfl => p.stage == Stage::Pro && p.pro.as_ref().is_some_and(|c| c.team == Some(t)),
                    Tier::Hs => p.stage == Stage::HighSchool && p.hs == t,
                }
            });
            if !here {
                w.marks.book_mut(tier, t)[i].list &= 127;
            }
        }
    }
}

/// A level's season is in: last season's fallen records stay, older ones go.
fn end_season(w: &mut World, tier: Tier, year: i32) {
    w.marks.fell.retain(|f| f.tier() != tier || f.year >= year);
    // The "already told" set is for one season at a time, but the levels'
    // seasons overlap: keep what this season's other levels told.
    retell(w, |f| f.tier() != tier && f.year >= year);
}

/// The "already told" set and each list's best told mark, from the fallen
/// records that still count this season.
fn retell(w: &mut World, keep: impl Fn(&Fell) -> bool) {
    // A breaker who kept adding holds more than his fallen record says.
    let old = std::mem::take(&mut w.marks.best_told);
    let mut told = HashSet::new();
    let mut best: HashMap<u64, (f64, u32)> = HashMap::new();
    for f in w.marks.fell.iter().filter(|f| keep(f)) {
        let code = list_code(f.scope.as_ref(), f.team, f.book);
        told.insert((code, f.id));
        let start = old.get(&code).copied().filter(|o| o.0 > f.value).unwrap_or((f.value, f.id));
        let b = best.entry(code).or_insert(start);
        if f.value > b.0 {
            *b = (f.value, f.id);
        }
    }
    w.marks.told = told;
    w.marks.best_told = best;
}

/// Per-game averages of the record stats among a season's regulars, by
/// position (the era column's 100).
fn norms_of(rows: &[(Position, u32, u32, Vals)], min_games: u32) -> HashMap<Position, Vals> {
    let mut sums: HashMap<Position, (Vals, f32)> = HashMap::new();
    for &(pos, games, snaps, v) in rows {
        if games < min_games || snaps < 15 * games {
            continue;
        }
        let e = sums.entry(pos).or_insert(([0.0; N], 0.0));
        for j in 0..N {
            e.0[j] += v[j] / games as f32;
        }
        e.1 += 1.0;
    }
    sums.into_iter().map(|(p, (s, n))| (p, s.map(|x| x / n))).collect()
}

/// The NFL's regular season is over (after the league's own record book):
/// the season's averages, the NFL's lists, each club's book, the leaders,
/// the rate records and the analytics rows.
pub(crate) fn nfl_close(w: &mut World) {
    if !w.recording {
        return;
    }
    let year = w.year;
    // The season's length from the rule book (an 18-game season once voted).
    let team_games = w.changes.rules(year).nfl_games;
    struct Line {
        k: usize,
        club: usize,
        games: u32,
        snaps: u32,
        all: [f32; STAT_COUNT],
    }
    let lines: Vec<Line> = w
        .people
        .iter()
        .enumerate()
        .filter_map(|(k, p)| {
            let s = p.pro.as_ref()?.seasons.iter().rev().find(|s| s.year == year && s.games > 0)?;
            Some(Line {
                k,
                club: s.team?,
                games: s.games,
                snaps: s.snaps[0] + s.snaps[1] + s.snaps[2],
                all: super::records::from_line(&s.stats),
            })
        })
        .collect();
    // A season on the stand-in has no lines: the books begin with the
    // first season on the engine.
    if w.marks.first[Tier::Nfl.ix()] == 0 && !lines.is_empty() {
        w.marks.first[Tier::Nfl.ix()] = year;
    }
    let store_from = w.marks.first[Tier::Nfl.ix()];
    let norm_rows: Vec<(Position, u32, u32, Vals)> =
        lines.iter().map(|l| (w.people[l.k].player.position, l.games, l.snaps, vals_of(&l.all))).collect();
    for (pos, m) in norms_of(&norm_rows, 8) {
        w.marks.norms.insert((year, NFL_LEVEL, pos), m);
    }
    let top = Tier::Nfl.team_top();
    // Each man's earlier seasons in the store, by id.
    let mut earlier: HashMap<u32, Vec<usize>> = HashMap::new();
    for (k, r) in w.marks.pro_rows.iter().enumerate() {
        earlier.entry(r.id).or_default().push(k);
    }
    let mut rows = Vec::new();
    for l in &lines {
        let p = &w.people[l.k];
        let (id, name, pos) = (p.player.id, p.name.clone(), p.player.position);
        let live = p.stage == Stage::Pro;
        let v = vals_of(&l.all);
        let get = |s: Stat| l.all[s as usize] as f64;
        let parts = parts_of(&get);
        let mut expect = [0f32; N];
        for (j, x) in expect.iter_mut().enumerate() {
            *x = (w.marks.norm(year, NFL_LEVEL, pos, j) * l.games as f64) as f32;
        }
        // The NFL book's career line already holds this season.
        let career = w.nfl_book.careers.get(&id).cloned().unwrap_or_default();
        let cv = vals(&career);
        let cparts = parts_of(&|s| super::records::from_line(&career)[s as usize] as f64);
        let prior: Vec<&ProRow> =
            earlier.get(&id).map_or(Vec::new(), |v| v.iter().map(|&k| &w.marks.pro_rows[k]).collect());
        // His career games from the book's career line, like its totals:
        // regular-season games, those before the store began too.
        let games = if career.games > 0 { career.games } else { l.games };
        let mut from = year;
        let mut cexpect = expect;
        let mut known = w.marks.norms.contains_key(&(year, NFL_LEVEL, pos));
        for r in &prior {
            from = from.min(r.year as i32);
            known &= w.marks.norms.contains_key(&(r.year as i32, NFL_LEVEL, r.pos));
            for (j, x) in cexpect.iter_mut().enumerate() {
                *x += (w.marks.norm(r.year as i32, NFL_LEVEL, r.pos, j) * r.games as f64) as f32;
            }
        }
        // Seasons before the store began: his first season on record, and
        // no norms for them, so the era column is unknown.
        if let Some(c) = p.pro.as_ref() {
            if let Some(f) = c.seasons.first() {
                from = from.min(f.year);
            }
            known &= !c.seasons.iter().any(|s| s.year < store_from && s.games > 0);
        }
        if !known {
            cexpect = [0.0; N];
        }
        for j in 0..N {
            if v[j] > 0.0 {
                let e = RecordEntry {
                    value: v[j] as f64,
                    id,
                    name: name.clone(),
                    program: l.club,
                    year,
                    from: year,
                    games: l.games as u16,
                    pos: Some(pos),
                    era: era(v[j] as f64, expect[j] as f64),
                    ..Default::default()
                };
                let list = w.stats.books.entry((Scope::Nfl, Book::Season(j))).or_default();
                place_entry(list, e, DIVISION_TOP, false);
            }
            if cv[j] > 0.0 {
                let e = RecordEntry {
                    value: cv[j] as f64,
                    id,
                    name: name.clone(),
                    program: l.club,
                    year,
                    from,
                    live,
                    games: games.min(u16::MAX as u32) as u16,
                    pos: Some(pos),
                    era: era(cv[j] as f64, cexpect[j] as f64),
                    ..Default::default()
                };
                let list = w.stats.books.entry((Scope::Nfl, Book::Career(j))).or_default();
                place_entry(list, e, DIVISION_TOP, true);
            }
        }
        // Each club's season from its games (a man traded in the season
        // has a line with each); games the store didn't see (a file loaded
        // in the season) go to his last club.
        let mut clubs: Vec<(usize, u16, Vals)> =
            w.marks.club_season.range((id, 0)..=(id, u8::MAX)).map(|(&(_, c), &(g, cv))| (c as usize, g, cv)).collect();
        let seen: u32 = clubs.iter().map(|c| c.1 as u32).sum();
        if seen < l.games {
            let mut rest = v;
            for c in &clubs {
                for (x, y) in rest.iter_mut().zip(&c.2) {
                    *x = (*x - y).max(0.0);
                }
            }
            let missing = (l.games - seen).min(u16::MAX as u32) as u16;
            match clubs.iter_mut().find(|c| c.0 == l.club) {
                Some(c) => {
                    c.1 = c.1.saturating_add(missing);
                    add(&mut c.2, &rest);
                }
                None => clubs.push((l.club, missing, rest)),
            }
        }
        // Each club's book: his season there, and his career there.
        let club_from: Vec<i32> = clubs
            .iter()
            .map(|c| {
                let rows = prior.iter().filter(|r| r.team as usize == c.0).map(|r| r.year as i32).min();
                let kept = w.marks.club_first.get(&(id, c.0 as u8)).map(|&y| y as i32);
                rows.into_iter().chain(kept).min().unwrap_or(year)
            })
            .collect();
        for ((club, club_games, sv), club_from) in clubs.into_iter().zip(club_from) {
            w.marks.club_first.insert((id, club as u8), club_from as i16);
            let cc = {
                let e = w.marks.club_careers.entry((id, club as u8)).or_insert((0, [0.0; N]));
                e.0 = e.0.saturating_add(club_games);
                add(&mut e.1, &sv);
                *e
            };
            let here = live && p.pro.as_ref().is_some_and(|c| c.team == Some(club));
            let book = w.marks.book_mut(Tier::Nfl, club);
            for j in 0..N {
                if sv[j] > 0.0 {
                    let m = Mark {
                        value: sv[j],
                        id,
                        aux: 0,
                        date: 0,
                        year: year as i16,
                        list: list_key(SEASON, j),
                        games: club_games.min(255) as u8,
                    };
                    place_mark(book, m, top);
                }
                if cc.1[j] > 0.0 {
                    let m = Mark {
                        value: cc.1[j],
                        id,
                        aux: career_aux(club_from, cc.0 as u32),
                        date: 0,
                        year: year as i16,
                        list: list_key(CAREER, j) | if here { 128 } else { 0 },
                        games: cc.0.min(255) as u8,
                    };
                    place_mark(book, m, top);
                }
            }
        }
        let row = SeasonRow { id, name, pos, team: l.club, games: l.games, v, expect, parts, team_games };
        career_rates(w, &Scope::Nfl, Tier::Nfl, &row, &cparts, from, live, games);
        rows.push(row);
    }
    leaders_of(w, &Scope::Nfl, year, &rows);
    season_rates(w, &Scope::Nfl, Tier::Nfl, year, rows.iter());
    // The analytics rows (item 8).
    for l in &lines {
        let p = &w.people[l.k];
        let Some(g) = analytics::group_of(p.player.position) else { continue };
        let get = |s: Stat| l.all[s as usize] as f64;
        let mut vals = [f32::NAN; analytics::CARD_MAX];
        for (x, cm) in vals.iter_mut().zip(analytics::charted(g)) {
            if let Some(v) = cm.value(&get, l.games as f64) {
                *x = v as f32;
            }
        }
        w.marks.pro_rows.push(ProRow {
            id: p.player.id,
            year: year as i16,
            team: l.club as u8,
            pos: p.player.position,
            games: l.games.min(255) as u8,
            vol: get(analytics::volume(g).0) as f32,
            vals,
        });
    }
    // The season's club lines are in the books.
    w.marks.club_season.clear();
    refresh_live_lists(w, &Scope::Nfl);
    refresh_live(w, Tier::Nfl);
    end_season(w, Tier::Nfl, year);
}

/// A state's season averages, before its lines go on the state's books.
/// `lines`: (person index, the season's line).
pub(crate) fn hs_norms(w: &mut World, state: usize, lines: &[(usize, HsLine)]) {
    if !w.recording {
        return;
    }
    let year = w.year;
    let rows: Vec<(Position, u32, u32, Vals)> = lines
        .iter()
        .map(|(_, l)| {
            let s = l.s.map(|x| x as i32);
            let snaps = (s[HsStat::SnapsOff as usize] + s[HsStat::SnapsDef as usize]).max(0) as u32;
            (l.pos, l.games as u32, snaps, hs_vals(&s))
        })
        .collect();
    for (pos, m) in norms_of(&rows, 5) {
        w.marks.norms.insert((year, state_level(state), pos), m);
    }
}

/// The era column's expected values for a high school line.
pub(crate) fn hs_expect(w: &World, state: usize, l: &HsLine) -> Vals {
    hs_expect_known(w, state, l).unwrap_or([0.0; N])
}

/// The same, or none when the state has no norm for his position that
/// season (one lookup for all the stats).
pub(crate) fn hs_expect_known(w: &World, state: usize, l: &HsLine) -> Option<Vals> {
    let m = w.marks.norms.get(&(l.year as i32, state_level(state), l.pos))?;
    Some(m.map(|x| (x as f64 * l.games as f64) as f32))
}

/// A state's season is in: each school's book, the state's leaders and its
/// rate records. `lines`: (person index, the season's line).
pub(crate) fn hs_close(w: &mut World, state: usize, lines: &[(usize, HsLine)]) {
    if !w.recording {
        return;
    }
    let year = w.year;
    // A season on the stand-in files no lines: the books begin with the
    // first season on the engine.
    if w.marks.first[Tier::Hs.ix()] == 0 && !lines.is_empty() {
        w.marks.first[Tier::Hs.ix()] = year;
    }
    let top = Tier::Hs.team_top();
    let scope = Scope::State(state);
    let mut rows = Vec::new();
    for (i, l) in lines {
        let p = &w.people[*i];
        let (id, name, live) = (p.player.id, p.name.clone(), p.stage == Stage::HighSchool);
        let school = l.school as usize;
        let s = l.s.map(|x| x as i32);
        let v = hs_vals(&s);
        let expect = hs_expect(w, state, l);
        let team_games = w.hs_season.as_ref().and_then(|h| h.results.get(school)).map_or(10, |t| t.games());
        // His career at the school and in the state.
        let mut at = [0f32; N];
        let mut first_at = year;
        let mut games_at = 0u32;
        let mut st_parts = [0f64; 11];
        let mut st_games = 0u32;
        let mut st_from = year;
        for c in &p.hs_lines {
            let cs = c.s.map(|x| x as i32);
            if c.school as usize == school {
                add(&mut at, &hs_vals(&cs));
                first_at = first_at.min(c.year as i32);
                games_at += c.games as u32;
            }
            if w.high_schools.get(c.school as usize).is_some_and(|h| h.state == state) {
                add_parts(&mut st_parts, &hs_parts(&cs));
                st_games += c.games as u32;
                st_from = st_from.min(c.year as i32);
            }
        }
        let here = live && p.hs == school;
        let book = w.marks.book_mut(Tier::Hs, school);
        for j in 0..N {
            if v[j] > 0.0 {
                let m = Mark {
                    value: v[j],
                    id,
                    aux: 0,
                    date: 0,
                    year: year as i16,
                    list: list_key(SEASON, j),
                    games: l.games,
                };
                place_mark(book, m, top);
            }
            if at[j] > 0.0 {
                let m = Mark {
                    value: at[j],
                    id,
                    aux: career_aux(first_at, games_at),
                    date: 0,
                    year: year as i16,
                    list: list_key(CAREER, j) | if here { 128 } else { 0 },
                    games: games_at.min(255) as u8,
                };
                place_mark(book, m, top);
            }
        }
        let row = SeasonRow {
            id,
            name,
            pos: l.pos,
            team: school,
            games: l.games as u32,
            v,
            expect,
            parts: hs_parts(&s),
            team_games,
        };
        career_rates(w, &scope, Tier::Hs, &row, &st_parts, st_from, live, st_games);
        rows.push(row);
    }
    leaders_of(w, &scope, year, &rows);
    season_rates(w, &scope, Tier::Hs, year, rows.iter());
    refresh_live_lists(w, &scope);
    refresh_live_schools(w, state);
}

/// Clears the "still playing" mark on a state's school careers that are over.
fn refresh_live_schools(w: &mut World, state: usize) {
    let schools: Vec<usize> =
        (0..w.marks.hs.len()).filter(|&h| w.high_schools.get(h).is_some_and(|x| x.state == state)).collect();
    for t in schools {
        let ids: Vec<(usize, u32)> =
            w.marks.hs[t].iter().enumerate().filter(|(_, m)| m.live()).map(|(i, m)| (i, m.id)).collect();
        for (i, id) in ids {
            let here = person(w, id).is_some_and(|k| w.people[k].stage == Stage::HighSchool && w.people[k].hs == t);
            if !here {
                w.marks.hs[t][i].list &= 127;
            }
        }
    }
}

/// High school seasons end state by state; the fallen records of the last
/// season go when the next one starts.
pub(crate) fn hs_new_season(w: &mut World) {
    let year = w.year;
    w.marks.fell.retain(|f| f.tier() != Tier::Hs || f.year >= year - 1);
    // What this season's college and NFL already told stays told.
    retell(w, |f| f.year >= year && f.tier() != Tier::Hs);
}

/// Before the people who left are dropped (`World::compact`): the names
/// the team books and analytics rows still need. `stays`: whether each
/// person stays.
pub(crate) fn keep_names(w: &mut World, stays: &[Option<usize>]) {
    let mut wanted: HashSet<u32> = HashSet::new();
    for book in w.marks.college.iter().chain(&w.marks.nfl).chain(&w.marks.hs) {
        wanted.extend(book.iter().map(|m| m.id));
    }
    wanted.extend(w.marks.pro_rows.iter().map(|r| r.id));
    let add: Vec<(u32, String)> = w
        .people
        .iter()
        .zip(stays)
        .filter(|(p, s)| s.is_none() && wanted.contains(&p.player.id))
        .map(|(p, _)| (p.player.id, p.name.clone()))
        .collect();
    // Club careers of men who left football are done.
    let gone: HashSet<u32> =
        w.people.iter().zip(stays).filter(|(_, s)| s.is_none()).map(|(p, _)| p.player.id).collect();
    w.marks.club_careers.retain(|k, _| !gone.contains(&k.0));
    w.marks.club_first.retain(|k, _| !gone.contains(&k.0));
    // A name stays wanted only while no living person has it.
    let living: HashSet<u32> =
        w.people.iter().zip(stays).filter(|(_, s)| s.is_some()).map(|(p, _)| p.player.id).collect();
    let keep: HashSet<u32> = wanted.difference(&living).copied().collect();
    w.marks.names.rebuild(&keep, add);
}

/// One player's season so far (the Data pages' season in progress), as the
/// stat book's counts.
#[derive(Clone, Debug)]
pub struct NowLine {
    pub id: u32,
    pub team: usize,
    pub pos: Position,
    pub games: u32,
    pub all: [f32; STAT_COUNT],
}

/// A high school line's counts in the stat book's order (the box score's
/// numbers; the rest stay zero).
pub fn hs_all(s: &[i32; HS_STAT_COUNT]) -> [f32; STAT_COUNT] {
    let mut a = [0f32; STAT_COUNT];
    let pairs = [
        (HsStat::SnapsOff, Stat::SnapsOff),
        (HsStat::SnapsDef, Stat::SnapsDef),
        (HsStat::Att, Stat::Att),
        (HsStat::Cmp, Stat::Cmp),
        (HsStat::PassYds, Stat::PassYds),
        (HsStat::PassTd, Stat::PassTd),
        (HsStat::Int, Stat::Int),
        (HsStat::Rush, Stat::Rush),
        (HsStat::RushYds, Stat::RushYds),
        (HsStat::RushTd, Stat::RushTd),
        (HsStat::Rec, Stat::Rec),
        (HsStat::RecYds, Stat::RecYds),
        (HsStat::RecTd, Stat::RecTd),
        (HsStat::Tackles, Stat::Tackles),
        (HsStat::Tfl, Stat::Tfl),
        (HsStat::Sacks, Stat::Sacks),
        (HsStat::Ints, Stat::Ints),
        (HsStat::Pbus, Stat::Pbus),
        (HsStat::FgMade, Stat::FgMade),
        (HsStat::FgAtt, Stat::FgAtt),
        (HsStat::XpMade, Stat::XpMade),
    ];
    for (h, k) in pairs {
        let x = s[h as usize] as f32;
        a[k as usize] = if matches!(h, HsStat::Tfl | HsStat::Sacks) { x / 2.0 } else { x };
    }
    a
}

/// The season in progress at a level: college's lines so far, the NFL's
/// regular season (none once it is over: its lines are in the books), a
/// state's high school lines (all states when `state` is none).
pub fn season_now(w: &World, tier: Tier, state: Option<usize>) -> Vec<NowLine> {
    let mut out = Vec::new();
    match tier {
        Tier::College => {
            if let Some(cs) = w.college.as_ref() {
                for (&id, b) in &cs.book {
                    let Some(pos) = b.pos.or_else(|| person(w, id).map(|k| w.people[k].player.position)) else {
                        continue;
                    };
                    out.push(NowLine {
                        id,
                        team: b.program,
                        pos,
                        games: b.games,
                        all: super::records::from_line(&b.line),
                    });
                }
            }
        }
        Tier::Nfl => {
            if w.nfl_season.as_ref().is_none_or(|s| !s.started || s.booked) {
                return out;
            }
            let year = w.year;
            for p in &w.people {
                let Some(s) = p.pro.as_ref().and_then(|c| c.seasons.iter().rev().find(|s| s.year == year)) else {
                    continue;
                };
                let (Some(team), true) = (s.team, s.games > 0) else { continue };
                out.push(NowLine {
                    id: p.player.id,
                    team,
                    pos: p.player.position,
                    games: s.games,
                    all: super::records::from_line(&s.stats),
                });
            }
        }
        Tier::Hs => {
            for (i, school, pos, games, s) in super::hs_year::running_in(w, state) {
                if games == 0 {
                    continue;
                }
                let Some(p) = w.people.get(i) else { continue };
                out.push(NowLine { id: p.player.id, team: school, pos, games: games as u32, all: hs_all(&s) });
            }
        }
    }
    out.sort_by_key(|l| l.id);
    out
}

/// Someone closing in on a record (item 20).
#[derive(Clone, Debug)]
pub struct Chase {
    pub book: Book,
    pub id: u32,
    pub name: String,
    pub team: usize,
    /// What he has now, his season's pace (a season in progress), and his
    /// games so far.
    pub have: f64,
    pub pace: Option<f64>,
    pub games: u32,
    pub record: f64,
    pub holder: String,
    pub holder_year: i32,
}

/// The games in a regular season: what a season's pace is read against
/// (the NFL's from the world's rule book: 18 once voted).
pub fn season_games(w: &World, tier: Tier, year: i32) -> u32 {
    match tier {
        Tier::Nfl => w.changes.rules(year).nfl_games,
        Tier::College => 12,
        Tier::Hs => 10,
    }
}

/// Who is chasing a level's records now: this season's paces against the
/// season records (at least three games in, on pace for nine tenths of the
/// mark), and careers still being played within a quarter of the career
/// record (proposed, not confirmed: both bars).
pub fn chasing(w: &World, scope: &Scope) -> Vec<Chase> {
    let (tier, state) = match scope {
        Scope::Nfl => (Tier::Nfl, None),
        Scope::State(s) => (Tier::Hs, Some(*s)),
        _ => (Tier::College, None),
    };
    let now: Vec<NowLine> = season_now(w, tier, state)
        .into_iter()
        .filter(|l| match scope {
            Scope::Division(d) => w.programs.get(l.team).is_some_and(|p| p.division == *d),
            Scope::Conference(d, c) => w.programs.get(l.team).is_some_and(|p| p.division == *d && p.conference == *c),
            _ => true,
        })
        .collect();
    let total = season_games(w, tier, w.year) as f64;
    let mut out = Vec::new();
    for (j, &stat) in RECORD_STATS.iter().enumerate() {
        let k = stat as usize;
        if let Some(rec) = w.stats.books.get(&(scope.clone(), Book::Season(j))).and_then(|l| l.first()) {
            let mut best: Vec<(f64, &NowLine)> = now
                .iter()
                .filter(|l| l.games >= 3 && l.all[k] > 0.0)
                .map(|l| (l.all[k] as f64 / l.games as f64 * total.max(l.games as f64), l))
                .filter(|(pace, _)| *pace >= 0.9 * rec.value)
                .collect();
            best.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.id.cmp(&b.1.id)));
            for (pace, l) in best.into_iter().take(2) {
                out.push(Chase {
                    book: Book::Season(j),
                    id: l.id,
                    name: name_of(w, l.id).unwrap_or_default(),
                    team: l.team,
                    have: l.all[k] as f64,
                    pace: Some(pace),
                    games: l.games,
                    record: rec.value,
                    holder: rec.name.clone(),
                    holder_year: rec.year,
                });
            }
        }
        if matches!(scope, Scope::Conference(..)) {
            continue;
        }
        let Some(list) = w.stats.books.get(&(scope.clone(), Book::Career(j))) else { continue };
        let Some(rec) = list.first() else { continue };
        // Careers still being played at the level now (a man who retired or
        // finished school keeps his flag until the level's next close).
        for e in list.iter().skip(1).filter(|e| e.live && active_in(w, e.id, scope)) {
            let run = now.binary_search_by_key(&e.id, |l| l.id).ok().map_or(0.0, |i| now[i].all[k] as f64);
            let have = e.value + run;
            if have >= 0.75 * rec.value && rec.id != e.id {
                out.push(Chase {
                    book: Book::Career(j),
                    id: e.id,
                    name: e.name.clone(),
                    team: e.program,
                    have,
                    pace: None,
                    games: e.games as u32,
                    record: rec.value,
                    holder: rec.name.clone(),
                    holder_year: rec.year,
                });
            }
        }
    }
    let close = |c: &Chase| c.pace.unwrap_or(c.have) / c.record.max(1e-9);
    out.sort_by(|a, b| close(b).total_cmp(&close(a)).then(a.id.cmp(&b.id)));
    out.truncate(12);
    out
}
