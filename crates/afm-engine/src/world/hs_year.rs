//! Section 4, topic 4: each state's high school year on its own real
//! calendar. A state's season starts on its own Friday (shifted from the
//! 2026 dates with Labor Day), plays its regular season week by week, then
//! its playoffs and finals (shifted with Thanksgiving). Around it: grade
//! checks by each state's rule (A29), SAT and ACT dates (A30), homecoming and
//! senior night (A31), all-state teams (A32), weekly state polls (A33), heat
//! rules on practice days (A35, A36), forfeits and no contests (A39), spring
//! ball as a breakout stage (T4.1) and the January all-star games.
//!
//! High school games run on the match engine for the history's last
//! seasons and every season after it (section 8, choice 2; Cain: the last
//! 12, "for now"): each school's varsity that week against the other, under
//! the state's rules, with its head coach calling both sides. The players'
//! lines go on their record (`HsLine`) and the state's record books. Older
//! history seasons use the stand-in: a game is won by comparing the two
//! teams' starters as they play this year, plus home field and luck.

use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

use super::coach::TitleKind;
use super::high_school::HsSeason;
use super::recruiting::film_raw;
use super::season::{self, TeamResult, key_normal, key_uniform, win_chance};
use super::{Stage, World};
use crate::calendar::anchors::{labor_day, thanksgiving};
use crate::calendar::data::{ACT_DATES, GradeCheck, SAT_DATES, hs_calendar};
use crate::calendar::weather::{Severe, weather_at};
use crate::calendar::{Date, Event, Kind, Level, Loudness, Weekday};
use crate::game::{GamePlayer, GameTeam};
use crate::player::PlayingTime;
use crate::positions::{POSITION_COUNT, Position};
use crate::rng::clamp;

/// High school games are close to a coin flip between equal teams: the
/// stand-in's spread in rating points.
const HS_SPREAD: f64 = 3.5;

/// One state's season dates for a year.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct StateSeason {
    pub start: Date,
    pub finals: Date,
    /// Regular-season game days, by week (a bye week has no games).
    pub weeks: Vec<Date>,
    /// The playoff rounds, one a week from the state's first round to its
    /// final (part B 12).
    pub rounds: Vec<Date>,
    pub developed: bool,
    pub done: bool,
}

/// A classification inside a state: its schools play each other.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Group {
    state: usize,
    class: u32,
    schools: Vec<usize>,
    /// Each week's games: (home, away).
    schedule: Vec<Vec<(usize, usize)>>,
}

/// A high school player's counting stats, in the order `HsLine::s` keeps them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HsStat {
    SnapsOff,
    SnapsDef,
    Att,
    Cmp,
    PassYds,
    PassTd,
    Int,
    Rush,
    RushYds,
    RushTd,
    Rec,
    RecYds,
    RecTd,
    /// Solo and assisted.
    Tackles,
    /// Kept in halves.
    Tfl,
    /// Kept in halves.
    Sacks,
    Ints,
    Pbus,
    FgMade,
    FgAtt,
    XpMade,
}

pub const HS_STAT_COUNT: usize = 21;

/// The state record books a season's line can enter, with the college
/// book's list it shares (`records::RECORD_STATS`).
const HS_BOOKS: [(HsStat, usize); 12] = [
    (HsStat::PassYds, 0),
    (HsStat::PassTd, 1),
    (HsStat::RushYds, 2),
    (HsStat::RushTd, 3),
    (HsStat::Rec, 4),
    (HsStat::RecYds, 5),
    (HsStat::RecTd, 6),
    (HsStat::Tackles, 7),
    (HsStat::Sacks, 8),
    (HsStat::Ints, 9),
    (HsStat::Tfl, 10),
    (HsStat::FgMade, 11),
];

/// One high school season on his record (section 8, choice 2): what a
/// player page, a state record book and college scouting read. Compact,
/// since nearly every player in the world has some.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct HsLine {
    pub year: i16,
    pub school: u32,
    pub pos: Position,
    pub games: u8,
    pub s: [i16; HS_STAT_COUNT],
}

impl HsLine {
    pub fn get(&self, k: HsStat) -> f64 {
        let v = self.s[k as usize] as f64;
        if matches!(k, HsStat::Tfl | HsStat::Sacks) { v / 2.0 } else { v }
    }
}

/// A season's line while it is played.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct HsAcc {
    school: u32,
    pos: Position,
    games: u8,
    s: [i32; HS_STAT_COUNT],
}

pub(crate) fn hs_counts(l: &crate::game::stats::StatLine) -> [i32; HS_STAT_COUNT] {
    let mut s = [0; HS_STAT_COUNT];
    let mut set = |k: HsStat, v: f64| s[k as usize] = v.round() as i32;
    set(HsStat::SnapsOff, l.snaps_off as f64);
    set(HsStat::SnapsDef, l.snaps_def as f64);
    set(HsStat::Att, l.att as f64);
    set(HsStat::Cmp, l.cmp as f64);
    set(HsStat::PassYds, l.pass_yds);
    set(HsStat::PassTd, l.pass_td as f64);
    set(HsStat::Int, l.int as f64);
    set(HsStat::Rush, l.rush as f64);
    set(HsStat::RushYds, l.rush_yds);
    set(HsStat::RushTd, l.rush_td as f64);
    set(HsStat::Rec, l.rec as f64);
    set(HsStat::RecYds, l.rec_yds);
    set(HsStat::RecTd, l.rec_td as f64);
    set(HsStat::Tackles, (l.tackles + l.assists) as f64);
    set(HsStat::Tfl, 2.0 * l.tfl);
    set(HsStat::Sacks, 2.0 * l.sacks);
    set(HsStat::Ints, l.ints as f64);
    set(HsStat::Pbus, l.pbus as f64);
    set(HsStat::FgMade, l.fg_made.iter().sum::<u32>() as f64);
    set(HsStat::FgAtt, l.fg_att.iter().sum::<u32>() as f64);
    set(HsStat::XpMade, l.xp_made as f64);
    s
}

/// One engine game, kept small: the score, each side's team totals and
/// each player's counts (person index, counts, whether he played).
struct HsPlayed {
    /// The game's injuries, by its team (update 3, A1).
    injuries: Vec<(usize, crate::game::sim::Injury)>,
    score: [u32; 2],
    team: [crate::game::stats::TeamStats; 2],
    lines: [Vec<(usize, [i32; HS_STAT_COUNT], bool)>; 2],
    /// The whole game, kept only for your school's game page (with its
    /// setup for watching it back) and your coming opponents' film.
    #[allow(clippy::type_complexity)]
    full: Option<(
        crate::game::stats::GameStats,
        [crate::game::calling::Film; 2],
        Option<Box<crate::game::ReplaySetup>>,
        crate::game::talk::Record,
    )>,
    /// The game on the state's numbers for the game boards (lane 9).
    tally: super::charts::Tally,
    /// A final at a neutral site.
    neutral: bool,
    /// The world reader's game log: the game's row, packed where the game
    /// was played, when a log is open, and its situation totals (for it or
    /// the trial game log).
    log: Option<Vec<u8>>,
    situ: Option<super::game_log::GameSitu>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct HsSeasonState {
    pub states: Vec<StateSeason>,
    pub results: Vec<TeamResult>,
    groups: Vec<Group>,
    /// Strength lost each week (regular weeks then playoff rounds) to
    /// starters out for grades, by school.
    out: HashMap<usize, Vec<f64>>,
    /// Each school's homecoming and senior night (A31).
    pub marks: Vec<Option<(Date, Date)>>,
    /// The new class of 14-year-olds arrives (before the earliest season).
    pub class_day: Date,
    /// Level ratings and teaching credit, once every state has developed.
    pub ratings_day: Date,
    rated: bool,
    /// This season's games run on the match engine (section 8, choice 2).
    pub on_engine: bool,
    /// The weeks each player misses for grades, as bits (regular weeks then
    /// playoff rounds), by person.
    grade_weeks: HashMap<usize, u32>,
    /// The season's lines so far, by person.
    book: HashMap<usize, HsAcc>,
    /// Each school's district this season (U-d), by school id.
    #[serde(default)]
    district: Vec<u32>,
    /// The regular season's results so far, (winner, loser), for the
    /// states' playoff ratings (U-d).
    #[serde(default)]
    played: Vec<(u32, u32)>,
    /// The playoff brackets, by state as each state's field is set (U-d).
    #[serde(default)]
    pub(crate) brackets: Vec<super::hs_playoffs::Bracket>,
    /// The districts as drawn, school ids in their round-robin order
    /// (update 2, lane 6): the cycle's second year plays the same ones.
    #[serde(default)]
    district_lists: Vec<Vec<u32>>,
    /// The playoff games played so far: (round, host, visitor, winner), for
    /// every school's schedule page (update 2, lane 6).
    #[serde(default)]
    playoff_games: Vec<(u8, u32, u32, u32)>,
}

impl HsSeasonState {
    /// The regular season's results so far among the schools `keep` picks:
    /// (winner, loser) (the schedule page, update 2, lane 6).
    pub(crate) fn winners(&self, keep: &dyn Fn(usize) -> bool) -> HashSet<(u32, u32)> {
        self.played.iter().copied().filter(|&(a, _)| keep(a as usize)).collect()
    }

    /// A school's playoff games so far: (round, host, visitor, winner).
    pub(crate) fn playoff_games_of(&self, h: usize) -> Vec<(usize, usize, usize, usize)> {
        let t = h as u32;
        self.playoff_games
            .iter()
            .filter(|g| g.1 == t || g.2 == t)
            .map(|g| (g.0 as usize, g.1 as usize, g.2 as usize, g.3 as usize))
            .collect()
    }

    /// A district's standing as the playoffs read it (`hs_playoffs::build`):
    /// district record, then the state's own rating, then the school.
    pub(crate) fn district_order(&self, w: &World, state: usize, mut members: Vec<usize>) -> Vec<usize> {
        let f = super::hs_playoffs::format(&w.states[state].code);
        let opps = super::hs_playoffs::opponents(w.high_schools.len(), &self.played);
        let rating: HashMap<usize, f64> =
            members.iter().map(|&h| (h, super::hs_playoffs::rate(f.rating, h, &opps, &self.results))).collect();
        let rec = |h: usize| self.results.get(h).map_or(0, |r| r.conf_wins as i64 - r.conf_losses as i64);
        members.sort_by(|&a, &b| rec(b).cmp(&rec(a)).then(rating[&b].total_cmp(&rating[&a])).then(a.cmp(&b)));
        members
    }

    /// Every school's regular-season games in a state, (home, away), in one
    /// pass over its groups.
    pub(crate) fn fixtures_by_school(&self, state: usize) -> HashMap<usize, Vec<(usize, usize)>> {
        let n = self.states.get(state).map_or(0, |s| s.weeks.len());
        let mut out: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
        for g in self.groups.iter().filter(|g| g.state == state) {
            for week in g.schedule.iter().take(n) {
                for &(a, b) in week {
                    out.entry(a).or_default().push((a, b));
                    out.entry(b).or_default().push((a, b));
                }
            }
        }
        out
    }

    /// Each school's record going into each regular-season week of a state
    /// (the schedule page, update 2, lane 6): school to (wins, losses)
    /// before week k, and after the last week at the end.
    pub(crate) fn records_by_week(&self, state: usize) -> HashMap<usize, Vec<(u32, u32)>> {
        let won: HashSet<(u32, u32)> = self.played.iter().copied().collect();
        let n = self.states.get(state).map_or(0, |s| s.weeks.len());
        let mut out: HashMap<usize, Vec<(u32, u32)>> = HashMap::new();
        for g in self.groups.iter().filter(|g| g.state == state) {
            for &s in &g.schools {
                out.entry(s).or_insert_with(|| vec![(0, 0); n + 1]);
            }
        }
        for g in self.groups.iter().filter(|g| g.state == state) {
            for (k, week) in g.schedule.iter().enumerate().take(n) {
                for &(h, a) in week {
                    let (x, y) = (h as u32, a as u32);
                    let winner = if won.contains(&(x, y)) {
                        h
                    } else if won.contains(&(y, x)) {
                        a
                    } else {
                        continue;
                    };
                    let loser = if winner == h { a } else { h };
                    for (t, win) in [(winner, true), (loser, false)] {
                        let v = out.entry(t).or_insert_with(|| vec![(0, 0); n + 1]);
                        for r in v.iter_mut().skip(k + 1) {
                            if win {
                                r.0 += 1;
                            } else {
                                r.1 += 1;
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// A school's district this season, if it has one.
    pub(crate) fn district_of(&self, h: usize) -> Option<u32> {
        self.district.get(h).copied().filter(|&d| d != super::hs_playoffs::NONE)
    }
}

/// Whether a season's high school games run on the match engine: every
/// season of a live save (the start year on), and the history's last
/// `hs_engine_years`. `engine_from`, the quick-check knob, holds back every
/// level alike, live seasons included.
pub(crate) fn hs_on_engine(w: &World, year: i32) -> bool {
    let start = w.options.start_year;
    let from = w.options.hs_engine_years.map_or(start, |n| start - n as i32);
    year >= from && w.options.engine_from.is_none_or(|e| year >= e)
}

/// A 2026 date shifted to `year` by the anchor it follows (Labor Day for
/// season starts, Thanksgiving for playoffs and finals), so the weekday stays.
fn shifted(m: u32, d: u32, year: i32, anchor: fn(i32) -> Date) -> Date {
    anchor(year).add(anchor(2026).days_to(Date::ymd(2026, m, d)))
}

/// The year's dates for each state.
fn state_dates(w: &World, year: i32) -> Vec<StateSeason> {
    w.states
        .iter()
        .map(|st| {
            let c = hs_calendar(&st.code);
            let start = shifted(c.start.0, c.start.1, year, labor_day);
            let playoffs = shifted(c.playoffs.0, c.playoffs.1, year, thanksgiving);
            let finals = shifted(c.finals.0, c.finals.1, year, thanksgiving);
            let room = (start.days_to(playoffs) / 7).max(1) as usize;
            let games = (c.games as usize).min(room);
            // Bye weeks go in the middle of the season.
            let byes = room - games;
            let mut weeks = Vec::with_capacity(games);
            let mut k = 0;
            let mut skipped = 0;
            while weeks.len() < games {
                let mid = games / 2 + skipped;
                if skipped < byes && k == mid {
                    skipped += 1;
                } else {
                    weeks.push(start.add(7 * k as i32));
                }
                k += 1;
            }
            // One round a week (part B 12): the weeks between the first
            // round and the final, the final on its own day.
            let span = ((playoffs.days_to(finals) + 3) / 7).max(1);
            let mut rounds: Vec<Date> = (0..span).map(|k| playoffs.add(7 * k)).collect();
            rounds.push(finals);
            StateSeason { start, finals, weeks, rounds, developed: false, done: false }
        })
        .collect()
}

/// A new football year: every state's dates and its weekly schedules, built
/// in February from the rosters of the day.
pub(crate) fn new_season(w: &World) -> HsSeasonState {
    let year = w.year;
    let states = state_dates(w, year);
    let seed = w.options.seed as u64 ^ (year as u64) << 32 ^ 0x4853;
    let cycle_seed = w.options.seed as u64 ^ (cycle_start(year) as u64) << 32 ^ 0x4853;
    let mut by: std::collections::BTreeMap<(usize, u32), Vec<usize>> = std::collections::BTreeMap::new();
    for s in &w.high_schools {
        by.entry((s.state, s.classification)).or_default().push(s.id);
    }
    // Groups come out by state, then classification.
    let mut groups: Vec<Group> = by
        .into_iter()
        .map(|((state, class), schools)| Group {
            state,
            class,
            schools,
            schedule: vec![Vec::new(); states[state].weeks.len()],
        })
        .collect();
    let mut marks = vec![None; w.high_schools.len()];
    let mut district = vec![super::hs_playoffs::NONE; w.high_schools.len()];
    let mut n_districts = 0u32;
    // The cycle's second year plays the districts its first year drew.
    let kept: &[Vec<u32>] = match w.hs_season.as_ref() {
        Some(h) if year != cycle_start(year) => &h.district_lists,
        _ => &[],
    };
    let mut district_lists: Vec<Vec<u32>> = Vec::new();
    let mut first = 0;
    while first < groups.len() {
        let state = groups[first].state;
        let end = first + groups[first..].iter().take_while(|g| g.state == state).count();
        let n_weeks = states[state].weeks.len();
        let classes: Vec<(u32, &[usize])> =
            groups[first..end].iter().map(|g| (g.class, g.schools.as_slice())).collect();
        let area = |id: usize| w.high_schools[id].area;
        let power = |id: usize| {
            let h = &w.high_schools[id];
            (h.prestige, h.private && h.is_powerhouse())
        };
        // Districts hold for a two-year cycle (update 2, lane 6): the same
        // schools both years, home and away swapped in the second.
        let cycle = (cycle_seed ^ (state as u64) << 8, year != cycle_start(year));
        let in_state = |id: u32| w.high_schools.get(id as usize).is_some_and(|s| s.state == state);
        let fixed: Vec<Vec<usize>> = kept
            .iter()
            .filter(|d| d.first().is_some_and(|&x| in_state(x)))
            .map(|d| d.iter().map(|&x| x as usize).collect())
            .collect();
        let (weeks, districts) = state_schedule(
            &classes,
            &area,
            &power,
            n_weeks,
            seed ^ (state as u64) << 8,
            cycle,
            (!fixed.is_empty()).then_some(fixed.as_slice()),
        );
        for d in districts {
            for &h in &d {
                district[h] = n_districts;
            }
            n_districts += 1;
            district_lists.push(d.into_iter().map(|h| h as u32).collect());
        }
        // Each game is filed under the home team's classification.
        let mut home_days: Vec<(usize, Date)> = Vec::new();
        for (k, games) in weeks.into_iter().enumerate() {
            for (h, a) in games {
                let class = w.high_schools[h].classification;
                let g = first + groups[first..end].iter().position(|g| g.class == class).unwrap_or(0);
                groups[g].schedule[k].push((h, a));
                home_days.push((h, states[state].weeks[k]));
            }
        }
        // Homecoming: the home game nearest midseason; senior night: the last home game.
        home_days.sort_by_key(|&(h, d)| (h, d));
        let mid = states[state].weeks[n_weeks / 2];
        for days in home_days.chunk_by(|a, b| a.0 == b.0) {
            let hc = days.iter().map(|x| x.1).min_by_key(|d| d.days_to(mid).abs()).unwrap();
            let sn = days.last().unwrap().1;
            marks[days[0].0] = Some((hc, sn));
        }
        first = end;
    }
    let earliest = states.iter().map(|s| s.start).min().unwrap_or(w.dates.hs_school_start);
    let latest = states.iter().map(|s| s.start).max().unwrap_or(w.dates.hs_school_start);
    HsSeasonState {
        states,
        results: vec![TeamResult::default(); w.high_schools.len()],
        groups,
        out: HashMap::new(),
        marks,
        class_day: w.dates.hs_school_start.min(earliest.add(-10)),
        ratings_day: latest.add(1),
        rated: false,
        on_engine: hs_on_engine(w, year),
        grade_weeks: HashMap::new(),
        book: HashMap::new(),
        district,
        played: Vec::new(),
        brackets: Vec::new(),
        district_lists,
        playoff_games: Vec::new(),
    }
}

/// Shuffles a week's schedule may try before it settles for byes or extra
/// games outside the classification.
const SCHEDULE_TRIES: u64 = 8;

/// Prestige from which a program schedules up (A11): the states' named
/// powerhouses (0.8 and up) and the strongest generated programs (they top
/// out at 0.72; proposed, not confirmed).
const BIG_NAME: f64 = 0.6;

/// The first season of a district cycle: states realign every two years,
/// for the even year and the next (Texas's UIL each February of an even
/// year; update 2, lane 6, proposed, not confirmed for every state).
pub fn cycle_start(year: i32) -> i32 {
    year - year.rem_euclid(2)
}

/// About how many schools make a district (part B 40; real districts run
/// about 5 to 8; proposed, not confirmed: 7, fewer when the season is short).
const DISTRICT_SIZE: usize = 7;

/// Each week's games (home, away), by week.
type WeekPairs = Vec<Vec<(usize, usize)>>;

/// A state's regular season, week by week, as (home, away) games, from its
/// classifications (class number and schools) and each school's area (part
/// B 40). Each classification splits into districts of nearby schools
/// (sorted by area), which play a round robin in the season's last weeks.
/// The first weeks are non-district games, against a school not met yet
/// from the same area first, then a similar classification. A school with a
/// district bye plays another school with one. Two schools never meet twice
/// in a regular season; a playoff rematch is fine. Also returns the
/// districts, as school ids.
///
/// A11, the dynasty leveler: strong programs schedule up, as real ones do.
/// A state's private powerhouses play in one league of their own when there
/// are three or more (California's Trinity League, the Catholic leagues),
/// whatever their class. And a program with a big name (`power`: prestige,
/// and whether it is a private powerhouse) books its non-district games
/// against the biggest names in the state that are free, not the school
/// down the road, so the best teams meet every year.
///
/// `cycle`: the district cycle's own seed, so the districts hold for both
/// of its years, and whether this is its second year (home and away swap).
/// `fixed`: the districts the cycle's first year drew (school ids in their
/// round-robin order), played again in its second year whatever the weeks
/// or the programs' standing now; a school new since gets placed as usual.
pub(crate) fn state_schedule(
    classes: &[(u32, &[usize])],
    area: &dyn Fn(usize) -> usize,
    power: &dyn Fn(usize) -> (f64, bool),
    n_weeks: usize,
    seed: u64,
    cycle: (u64, bool),
    fixed: Option<&[Vec<usize>]>,
) -> (WeekPairs, Vec<Vec<usize>>) {
    const FREE: usize = usize::MAX;
    let ids: Vec<usize> = classes.iter().flat_map(|c| c.1.iter().copied()).collect();
    let class_of: Vec<u32> = classes.iter().flat_map(|c| std::iter::repeat_n(c.0, c.1.len())).collect();
    let area_of: Vec<usize> = ids.iter().map(|&id| area(id)).collect();
    let n = ids.len();
    // --- Districts: nearby schools of one classification ---------------------------
    let size = DISTRICT_SIZE.min(n_weeks.saturating_sub(2)).max(2);
    let mut district = vec![FREE; n];
    let mut districts: Vec<Vec<usize>> = Vec::new();
    let prestige: Vec<f64> = ids.iter().map(|&id| power(id).0).collect();
    // The cycle's districts as drawn in its first year.
    if let Some(lists) = fixed {
        let at: HashMap<usize, usize> = ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();
        for list in lists {
            let part: Vec<usize> = list.iter().filter_map(|id| at.get(id).copied()).collect();
            if part.is_empty() {
                continue;
            }
            for &i in &part {
                district[i] = districts.len();
            }
            districts.push(part);
        }
    }
    // The private powerhouses' league (A11).
    let mut league: Vec<usize> = (0..n).filter(|&i| power(ids[i]).1 && district[i] == FREE).collect();
    if fixed.is_none() && league.len() >= 3 {
        league.sort_by(|&a, &b| prestige[b].total_cmp(&prestige[a]).then(a.cmp(&b)));
        league.truncate(size.max(3));
        for &i in &league {
            district[i] = districts.len();
        }
        districts.push(league);
    }
    let mut at = 0;
    for c in classes {
        let mut idx: Vec<usize> = (at..at + c.1.len()).filter(|&i| district[i] == FREE).collect();
        at += c.1.len();
        if idx.is_empty() {
            continue;
        }
        idx.sort_by(|&a, &b| {
            area_of[a].cmp(&area_of[b]).then(
                key_uniform(cycle.0 ^ 0xD15, ids[a] as u64).total_cmp(&key_uniform(cycle.0 ^ 0xD15, ids[b] as u64)),
            )
        });
        let k = idx.len().div_ceil(size).max(1);
        for j in 0..k {
            let part: Vec<usize> = idx[j * idx.len() / k..(j + 1) * idx.len() / k].to_vec();
            for &i in &part {
                district[i] = districts.len();
            }
            districts.push(part);
        }
    }
    // Round robins by the circle method: a phantom school gives the byes.
    let rounds_of = |m: usize| if m.is_multiple_of(2) { m.saturating_sub(1) } else { m };
    let d_weeks = districts.iter().map(|d| rounds_of(d.len())).max().unwrap_or(0).min(n_weeks);
    let mut weeks: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n_weeks];
    let mut met: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut busy: Vec<Vec<bool>> = vec![vec![false; n]; n_weeks];
    for d in &districts {
        let mut ring: Vec<usize> = d.clone();
        if ring.len() % 2 == 1 {
            ring.push(FREE);
        }
        let m = ring.len();
        for r in 0..rounds_of(d.len()).min(d_weeks) {
            let wk = n_weeks - d_weeks + r;
            for x in 0..m / 2 {
                let (a, b) = (ring[x], ring[m - 1 - x]);
                if a != FREE && b != FREE {
                    weeks[wk].push((a, b));
                    met[a].push(b);
                    met[b].push(a);
                    busy[wk][a] = true;
                    busy[wk][b] = true;
                }
            }
            // Rotate all but the first.
            let last = ring.pop().unwrap();
            ring.insert(1, last);
        }
    }
    // --- Everything else: non-district weeks and district byes -------------------
    for k in 0..n_weeks {
        let mut best: (usize, Vec<(usize, usize)>) = (usize::MAX, Vec::new());
        for t in 0..SCHEDULE_TRIES {
            let key = seed ^ (k as u64) << 20 ^ t << 48;
            let mut order: Vec<usize> = (0..n).filter(|&a| !busy[k][a]).collect();
            let shuffle: Vec<f64> = ids.iter().map(|&id| key_uniform(key, id as u64)).collect();
            order.sort_by(|&a, &b| area_of[a].cmp(&area_of[b]).then(shuffle[a].total_cmp(&shuffle[b])));
            // The big names book first (A11).
            let (mut big, rest): (Vec<usize>, Vec<usize>) = order.iter().partition(|&&a| prestige[a] >= BIG_NAME);
            big.sort_by(|&a, &b| prestige[b].total_cmp(&prestige[a]).then(shuffle[a].total_cmp(&shuffle[b])));
            big.extend(rest);
            let order = big;
            let mut partner = vec![FREE; n];
            let mut pairs: Vec<(usize, usize)> = Vec::new();
            for (x, &a) in order.iter().enumerate() {
                if partner[a] != FREE {
                    continue;
                }
                // The same area first, then the closest classification;
                // the order is by area, so the nearby schools come next.
                let open = |b: &&usize| partner[**b] == FREE && !met[a].contains(*b) && district[**b] != district[a];
                let rank = |b: &&usize| ((area_of[**b] != area_of[a]) as u32, class_of[**b].abs_diff(class_of[a]));
                // A big name books the biggest free name in the state.
                let b = if prestige[a] >= BIG_NAME {
                    order[x + 1..]
                        .iter()
                        .filter(open)
                        .max_by(|&&p, &&q| prestige[p].total_cmp(&prestige[q]).then(shuffle[q].total_cmp(&shuffle[p])))
                } else {
                    let near = order[x + 1..].iter().take(200).filter(open).min_by_key(rank);
                    near.or_else(|| order[x + 1..].iter().filter(open).min_by_key(rank))
                };
                if let Some(&b) = b {
                    (partner[a], partner[b]) = (b, a);
                    pairs.push((a, b));
                }
            }
            let left = order.len() - 2 * pairs.len();
            if left < best.0 {
                best = (left, pairs);
            }
            if best.0 <= order.len() % 2 {
                break;
            }
        }
        for &(a, b) in &best.1 {
            met[a].push(b);
            met[b].push(a);
            weeks[k].push((a, b));
        }
    }
    // Home and away: a district game by its round (so the second year swaps
    // each pairing even when the season's weeks moved), the rest by week.
    let first_round = n_weeks - d_weeks;
    let weeks = weeks
        .into_iter()
        .enumerate()
        .map(|(k, pairs)| {
            pairs
                .into_iter()
                .map(|(a, b)| {
                    let same = district[a] != FREE && district[a] == district[b] && k >= first_round;
                    let even = if same { (k - first_round).is_multiple_of(2) } else { k.is_multiple_of(2) };
                    if even != cycle.1 { (ids[a], ids[b]) } else { (ids[b], ids[a]) }
                })
                .collect()
        })
        .collect();
    (weeks, districts.into_iter().map(|d| d.into_iter().map(|i| ids[i]).collect()).collect())
}

/// A school that opens after the year's season was drawn (SK68, a ground
/// zero start): the season is drawn again with it while no state has
/// started; later it sits out until next season.
pub(crate) fn add_school(w: &mut World, h: usize) {
    let Some(hs) = w.hs_season.as_ref() else { return };
    if hs.states.iter().all(|s| !s.developed && !s.done) && hs.played.is_empty() {
        w.hs_season = Some(new_season(w));
        return;
    }
    let hs = w.hs_season.as_mut().unwrap();
    hs.results.resize(h + 1, TeamResult::default());
    hs.marks.resize(h + 1, None);
    hs.district.resize(h + 1, super::hs_playoffs::NONE);
}

/// The high school day.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(hs) = w.hs_season.as_ref() else { return };
    if d == hs.class_day {
        super::clock::timed("hs class", || {
            w.add_class();
            w.rebuild_rosters();
            // New schools take their zones' players (SK68).
            if super::new_teams::school::class_day(w) {
                w.rebuild_rosters();
            }
        });
    }
    let n_states = w.states.len();
    for s in 0..n_states {
        let st = w.hs_season.as_ref().unwrap().states[s].clone();
        if st.done {
            continue;
        }
        if d == st.start.add(-1) && !st.developed {
            heat_days(w, s, st.start);
            super::clock::timed("hs start", || start_state(w, s));
            poll(w, s, d, "Preseason");
        }
        if let Some(k) = st.weeks.iter().position(|&g| g == d) {
            super::clock::timed("hs week", || play_week(w, s, k, d));
        }
        if st.weeks.iter().any(|&g| g.add(3) == d) {
            super::clock::timed("hs poll", || poll(w, s, d, "Weekly"));
        }
        if d == st.rounds[0] {
            seed_playoffs(w, s);
        }
        if let Some(r) = st.rounds.iter().position(|&x| x == d) {
            super::clock::timed("hs playoffs", || play_round(w, s, r, d));
        }
        if d == st.finals {
            super::clock::timed("hs finish", || finish_state(w, s, d));
        }
    }
    let hs = w.hs_season.as_ref().unwrap();
    if d == hs.ratings_day && !hs.rated {
        super::clock::timed("hs ratings", || {
            season::attach_level_ratings(w);
            super::staff::credit_teaching(w);
        });
        w.hs_season.as_mut().unwrap().rated = true;
    }
    // SAT and ACT Saturdays (A30).
    let (y, m, _) = d.parts();
    if d.weekday() == Weekday::Sat
        && SAT_DATES
            .iter()
            .chain(ACT_DATES.iter())
            .any(|&(mm, n)| mm == m && Date::nth_weekday(y, mm, Weekday::Sat, n) == d)
    {
        super::clock::timed("hs tests", || test_day(w, d));
    }
    // High school spring ball in the states that have it (T4.1): notices
    // early in May, the step by the end of it.
    if (m, d.day()) == (5, 1) {
        super::clock::timed("hs spring", || spring_notices_hs(w, d));
    }
    if d == Date::last_weekday(y, 5, Weekday::Fri) {
        super::clock::timed("hs spring", || spring_jumps_hs(w, d));
    }
    // State all-star games for the graduating seniors, late May.
    if d == Date::last_weekday(y, 5, Weekday::Sat) {
        super::clock::timed("hs allstars", || state_all_stars(w, d));
    }
}

/// Heat on August practice days (A35, A36): in the states with a heat rule,
/// a practice day with a heat index of 104 F or more at 3pm is cut or moved
/// indoors. Logged; the step on players waits for training (section 6b).
fn heat_days(w: &mut World, s: usize, start: Date) {
    let c = hs_calendar(&w.states[s].code);
    if !c.heat_rule {
        return;
    }
    let code = w.states[s].code.clone();
    let mut d = Date::ymd(start.year(), 8, 1).min(start.add(-21));
    let mut days = 0;
    while d < start {
        if d.weekday() != Weekday::Sun && weather_at(w.options.seed as u64, s, &code, d, 15).heat_index() >= 104.0 {
            days += 1;
        }
        d = d.add(1);
    }
    w.log.heat_limited_days += days;
}

/// A state's season starts: staffs set positions and depth charts, the
/// year's development lands, team strength is read, and grade checks are
/// scheduled for those who will miss games (A29).
fn start_state(w: &mut World, s: usize) {
    let schools: Vec<usize> = w.states[s].schools.clone();
    // The state's players, read from its schools' rosters (rebuilt on the
    // class day, before any state starts).
    let mut who = vec![false; w.people.len()];
    for &h in &schools {
        for &i in &w.high_schools[h].roster {
            let p = &w.people[i];
            who[i] = p.stage == Stage::HighSchool && p.school_age() > 14;
        }
    }
    super::clock::timed("hs reconsider", || season::reconsider_positions(w, &who));
    let mut roles = super::clock::timed("hs roles", || season::hs_roles(w, &schools));
    // Your JV and freshman players get their own squad's game reps (lane 10).
    super::front_office::school::squad_roles(w, &mut roles);
    super::clock::timed("hs mood", || super::people::hs_camp_mood(w, &schools, &roles));
    super::clock::timed("hs develop", || season::develop(w, &roles));
    let people = &w.people;
    let mut res: Vec<(usize, TeamResult)> = Vec::with_capacity(schools.len());
    for &h in &schools {
        let (strength, preseason, incidents) = season::team_strength(&w.high_schools[h].roster, people);
        res.push((
            h,
            TeamResult {
                strength,
                preseason,
                incidents,
                roster: w.high_schools[h].roster.len() as u32,
                ..Default::default()
            },
        ));
    }
    let st = w.hs_season.as_ref().unwrap().states[s].clone();
    let n_weeks = st.weeks.len() + st.rounds.len();
    let check = hs_calendar(&w.states[s].code).grades;
    let seed = w.options.seed as u64 ^ (w.year as u64) << 28 ^ 0x6772;
    let mut out: HashMap<usize, Vec<f64>> = HashMap::new();
    let mut grade_weeks: HashMap<usize, u32> = HashMap::new();
    let mut missed_players = 0;
    for &h in &schools {
        let starters = w.high_schools[h]
            .roster
            .iter()
            .filter(|&&i| {
                let p = &w.people[i];
                p.last_role == Some(PlayingTime::Starter) && !p.player.position.is_specialist()
            })
            .count()
            .max(1);
        for &i in &w.high_schools[h].roster.clone() {
            let p = &w.people[i];
            let miss = grade_outs(p, check, n_weeks, st.weeks.len(), seed);
            if miss == 0 {
                continue;
            }
            missed_players += 1;
            grade_weeks.insert(i, miss);
            let games = miss.count_ones();
            let starter = p.last_role == Some(PlayingTime::Starter) && !p.player.position.is_specialist();
            if starter {
                // A starter out: his backup plays, about 8 points worse.
                let v = out.entry(h).or_insert_with(|| vec![0.0; n_weeks]);
                for (k, x) in v.iter_mut().enumerate() {
                    if miss >> k & 1 == 1 {
                        *x += 8.0 / starters as f64;
                    }
                }
            }
            let p = &mut w.people[i];
            p.cal.hs_grade_out = games;
            if starter {
                let age = p.player.age - 1;
                let rule = match check {
                    GradeCheck::SixWeek => "the state's no pass, no play rule",
                    GradeCheck::Weekly => "weekly grade checks",
                    GradeCheck::NineWeek => "a failing grading period",
                    GradeCheck::Semester => "last semester's grades",
                };
                let g = if games == 1 { "1 game".to_string() } else { format!("{games} games") };
                p.note(format!("At {age} he was ruled ineligible for {g} under {rule}."));
            }
        }
    }
    w.log.hs_grade_outs += missed_players;
    let hs = w.hs_season.as_mut().unwrap();
    for (h, r) in res {
        hs.results[h] = r;
    }
    hs.out.extend(out);
    hs.grade_weeks.extend(grade_weeks);
    hs.states[s].developed = true;
}

/// Which weeks a player misses for grades (A29), as a bit per week (regular
/// weeks then playoff rounds). Failing a check grows below a 2.6 GPA and with
/// a poor work ethic (proposed, not confirmed: 30% a check at a 2.0 GPA).
fn grade_outs(p: &super::Person, check: GradeCheck, n_weeks: usize, regular: usize, seed: u64) -> u32 {
    let gpa = p.bg.gpa;
    let q = clamp(0.5 * (2.6 - gpa), 0.0, 0.6) * clamp(1.0 - 0.04 * (p.player.personality.work_ethic - 10.0), 0.6, 1.4);
    if q <= 0.0 {
        return 0;
    }
    let id = p.player.id as u64;
    let all = if n_weeks >= 32 { u32::MAX } else { (1u32 << n_weeks) - 1 };
    match check {
        // Last semester's grades: out for the season, unless summer school fixed them.
        GradeCheck::Semester => {
            if key_uniform(seed ^ 1, id) < 0.4 * q {
                all
            } else {
                0
            }
        }
        // Texas: report cards about every six weeks; a failing grade sits him three games.
        GradeCheck::SixWeek => {
            let mut m = 0;
            for (k, at) in [(1u64, 4usize), (2, 10)] {
                if at < n_weeks && key_uniform(seed ^ (10 + k), id) < q {
                    for j in at..(at + 3).min(n_weeks) {
                        m |= 1 << j;
                    }
                }
            }
            m
        }
        // A weekly check: a failing week sits the next game.
        GradeCheck::Weekly => {
            let mut m = 0;
            for j in 1..n_weeks {
                if key_uniform(seed ^ (100 + j as u64), id) < q / 5.0 {
                    m |= 1 << j;
                }
            }
            m
        }
        // A nine-week grading period: out for the rest of the fall.
        GradeCheck::NineWeek => {
            let at = (regular * 7 / 10).max(1);
            if key_uniform(seed ^ 3, id) < q { all & !((1u32 << at) - 1) } else { 0 }
        }
    }
}

fn adj(hs: &HsSeasonState, school: usize, week: usize) -> f64 {
    hs.out.get(&school).and_then(|v| v.get(week)).copied().unwrap_or(0.0)
}

/// One Friday of a state's regular season: every classification plays.
fn play_week(w: &mut World, s: usize, k: usize, d: Date) {
    let seed = w.options.seed as u64 ^ (w.year as u64) << 24 ^ (k as u64) << 8 ^ 0x6761;
    // Severe weather can cancel a state's Friday (A38): most are made up the
    // next day; some become no contests.
    let code = w.states[s].code.clone();
    let wx = weather_at(w.options.seed as u64, s, &code, d, 19);
    let storm = wx.severe == Some(Severe::Hurricane);
    // Your school's games always run on the engine, so you get their game pages.
    let seat = w.seat_school();
    let hs = w.hs_season.as_mut().unwrap();
    let on_engine = hs.on_engine;
    let mut games = 0;
    let mut forfeits = 0;
    let mut no_contests = 0;
    let (mut homecomings, mut senior_nights) = (0, 0);
    let mut outcomes: Vec<(usize, usize)> = Vec::new();
    let mut engine: Vec<(usize, usize, u64)> = Vec::new();
    // Games decided off the engine, for the world reader's game log: home,
    // away, whether the home side won, a forfeit.
    let mut standins: Vec<(usize, usize, bool, bool)> = Vec::new();
    for g in hs.groups.iter().filter(|g| g.state == s) {
        let Some(week) = g.schedule.get(k) else { continue };
        for &(h, a) in week {
            let key = (h as u64) << 20 ^ a as u64;
            if hs.results[h].roster == 0 || hs.results[a].roster == 0 {
                continue;
            }
            // A landfall reaches the coastal schools only (part B 41).
            let coastal = key_uniform(w.options.seed as u64 ^ 0xC0A5, h as u64)
                < crate::calendar::weather::hurricane_school_share(&code);
            if storm && coastal && key_uniform(seed ^ 0x51, key) < 0.3 || key_uniform(seed ^ 0x52, key) < 0.002 {
                no_contests += 1;
                continue;
            }
            games += 1;
            if let Some((hc, sn)) = hs.marks[h] {
                homecomings += (hc == d) as u32;
                senior_nights += (sn == d) as u32;
            }
            let pe = win_chance(hs.results[h].preseason, hs.results[a].preseason, HS_SPREAD, 0.8);
            hs.results[h].expected += pe;
            hs.results[a].expected += 1.0 - pe;
            // A forfeit (A39): a short roster can't field a team, or an
            // ineligible player is found; it counts as a loss.
            let short = |r: u32| if r < 22 { 0.02 } else { 0.0015 };
            let fh = key_uniform(seed ^ 0x53, key) < short(hs.results[h].roster);
            let fa = !fh && key_uniform(seed ^ 0x54, key) < short(hs.results[a].roster);
            let home_wins = if fh || fa {
                forfeits += 1;
                fa
            } else if on_engine || seat.is_some_and(|x| x == h || x == a) {
                engine.push((h, a, key));
                continue;
            } else {
                let p = win_chance(
                    hs.results[h].strength - adj(hs, h, k),
                    hs.results[a].strength - adj(hs, a, k),
                    HS_SPREAD,
                    0.8,
                );
                key_uniform(seed ^ 0x55, key) < p
            };
            standins.push((h, a, home_wins, fh || fa));
            let (wn, l) = if home_wins { (h, a) } else { (a, h) };
            hs.results[wn].wins += 1;
            hs.results[l].losses += 1;
            outcomes.push((wn, l));
        }
    }
    w.log.hs_games += games;
    w.log.hs_forfeits += forfeits;
    w.log.hs_no_contests += no_contests;
    w.log.hs_homecomings += homecomings;
    w.log.hs_senior_nights += senior_nights;
    if w.logging() {
        for (h, a, home_wins, forfeit) in standins {
            let head = hs_head(w, s, k, d, [h, a], false);
            log_hs_standin(w, head, home_wins, forfeit);
        }
    }
    let played = engine_games(w, s, k, &engine, (d, &wx), false, seed ^ 0x55);
    let hs = w.hs_season.as_mut().unwrap();
    let mut all: Vec<(usize, usize)> = outcomes.clone();
    for (&(h, a, _), home_wins) in engine.iter().zip(played) {
        all.push(if home_wins { (h, a) } else { (a, h) });
    }
    // The season so far, for the playoff ratings, and district records (U-d).
    for &(wn, l) in &all {
        hs.played.push((wn as u32, l as u32));
        let dist = |x: usize| hs.district.get(x).copied().unwrap_or(super::hs_playoffs::NONE);
        if dist(wn) != super::hs_playoffs::NONE && dist(wn) == dist(l) {
            hs.results[wn].conf_wins += 1;
            hs.results[l].conf_losses += 1;
        }
    }
    if w.recording {
        for (wn, l) in outcomes {
            super::records::hs_result(w, wn, true);
            super::records::hs_result(w, l, false);
        }
    }
}

/// Fewest players a school can dress for an engine game; with fewer it
/// plays the stand-in (11 a side, two ways: real small schools play with
/// about 20).
const HS_DRESS_MIN: usize = 18;

/// A school's game team this week (section 8, choice 2): every player on
/// the roster but those sitting a transfer or out for grades; freshmen only
/// at a school short of 30 older players (the rest play freshman ball).
/// The head coach calls both sides, in the systems he knows best (a high
/// school runs its coach's system and fits its kids to it). Knowledge and
/// experience grow with his years in the program (proposed, not confirmed).
/// Also returns each player's id and person index.
fn hs_team(w: &World, h: usize, week: usize) -> Option<(GameTeam, Vec<(u32, usize)>)> {
    let school = &w.high_schools[h];
    let ok = |i: usize| can_play(w, i, week);
    let older = school.roster.iter().filter(|&&i| ok(i) && w.people[i].school_age() > 14).count();
    // Your school dresses its varsity squad (lane 10: you or your staff
    // decide who plays varsity, JV and freshman ball).
    let squads = super::front_office::school::varsity_filter(w, h);
    let dressed: Vec<usize> = school
        .roster
        .iter()
        .copied()
        .filter(|&i| {
            ok(i)
                && match &squads {
                    Some(v) => v.contains(&i),
                    None => older < 30 || w.people[i].school_age() > 14,
                }
        })
        .collect();
    squad_team(w, h, dressed, true)
}

/// Whether a high schooler may play in week `week` (regular weeks, then
/// playoff rounds): not sitting a transfer, suspended, out for grades or
/// hurt in your practices. A player suspended for the season sits, as in
/// college (his status line says so).
pub(crate) fn can_play(w: &World, i: usize, week: usize) -> bool {
    let grades = w.hs_season.as_ref().map(|hs| &hs.grade_weeks);
    let p = &w.people[i];
    p.stage == Stage::HighSchool
        && p.last_role != Some(PlayingTime::Redshirt)
        && !p.cal.trouble
        && !super::folk::rules::sitting(w, p.player.id, w.today)
        && !w.training.hurt_out(p.player.id, w.today)
        && grades.is_none_or(|g| g.get(&i).is_none_or(|m| m >> week & 1 == 0))
}

/// A school's systems as its head coach would install them: what he knows
/// best, and at your school what your guidelines ask for (G2).
pub(crate) fn school_install(w: &World, h: usize, yours: bool) -> super::systems::Installed {
    use super::systems::Installed;
    let coach = w.high_schools[h].coach;
    let know = coach.map_or([0.5; 11], |c| w.coaches[c].systems.know);
    let best = |r: std::ops::Range<usize>| {
        let start = r.start;
        r.max_by(|&a, &b| know[a].total_cmp(&know[b])).map_or(0, |x| x - start)
    };
    let inst = Installed { off: best(0..5), front: best(5..7), shell: best(7..11), ..Installed::default() };
    if !yours {
        return inst;
    }
    let level = crate::calendar::Level::HighSchool;
    let guide = super::guide::team_guidelines(w, coach, coach, coach, level, true);
    super::guide::wanted_install(&guide, inst)
}

/// Your school's summer camp install (update 2, G2 and S5): from August
/// the season's systems are set by your guidelines, once a season; a
/// system changed from last season is known only as far as it shares the
/// old one's playbook. Before camp, last season's stand.
pub(crate) fn camp_install(w: &mut World) {
    use super::systems::{Kind, overlap};
    let Some(h) = w.seat.filter(|s| s.level == crate::calendar::Level::HighSchool).and_then(|s| s.program) else {
        return;
    };
    let season = super::plans::season_of(w.today);
    if w.today.month() < 8 || w.plans.hs_install.is_some_and(|x| x.0 == season) {
        return;
    }
    let mut new = school_install(w, h, true);
    let (off, def) = match w.plans.hs_install {
        Some((_, old)) => (
            overlap(Kind::Offense, old.off, new.off),
            overlap(Kind::Front, old.front, new.front).min(overlap(Kind::Shell, old.shell, new.shell)),
        ),
        None => (1.0, 1.0),
    };
    (new.off_known, new.def_known) = (off, def);
    w.plans.hs_install = Some((season, new));
}

/// A game team from the players given (a school's varsity, or your JV or
/// freshman squad): the same build for each, with the school's head coach
/// calling both sides. Your order is played only by your varsity.
pub(crate) fn squad_team(
    w: &World,
    h: usize,
    dressed: Vec<usize>,
    varsity: bool,
) -> Option<(GameTeam, Vec<(u32, usize)>)> {
    let school = &w.high_schools[h];
    if dressed.len() < HS_DRESS_MIN {
        return None;
    }
    let ids: Vec<(u32, usize)> = dressed.iter().map(|&i| (w.people[i].player.id, i)).collect();
    // Your varsity's systems as your summer camp installed them (update 2,
    // G2): a change from last season is known only as far as the systems
    // share their playbooks (S5) until a year on.
    let seat = varsity && w.seat_school() == Some(h);
    let season = super::plans::season_of(w.today);
    let stored = w.plans.hs_install.filter(|_| seat).map(|(s, mut inst)| {
        if s != season {
            (inst.off_known, inst.def_known) = (1.0, 1.0);
        }
        inst
    });
    // Your feeders that took up your system send freshmen who know its
    // terms (update 2, L14): the playbook learning starts further along.
    let feeders = if w.seat_school() == Some(h) && !w.legacy.hs.feeders.is_empty() {
        super::front_office::school::feeders(w, h)
    } else {
        Vec::new()
    };
    let (seat_mind, rel_mind) = (seat, season::coach_relationships(w, w.high_schools[h].coach));
    let players: Vec<GamePlayer> = dressed
        .iter()
        .map(|&i| {
            let p = &w.people[i].player;
            let years = p.age.saturating_sub(14) as f64;
            let head = if feeders.is_empty() { 0.0 } else { super::legacy::hs::head_start(w, &feeders, h, i) };
            let side =
                stored.map_or(1.0, |s| if super::systems::offense(p.position) { s.off_known } else { s.def_known });
            let know = clamp(0.72 + 0.04 * years + head, 0.0, 0.9) * (0.8 + 0.2 * side);
            let mut g = GamePlayer::from_player(p, know, 0.7 * years);
            (g.mood, g.belief) = season::game_mind(&w.people[i], seat_mind, rel_mind);
            g
        })
        .collect();
    // A suspended coach (SK42) is off the sideline: his staff coaches.
    let coach = if super::scouting::pitch::sits(w, h) { None } else { school.coach };
    // Your guidelines pick your varsity's systems (update 2, G2) at camp.
    // An AI school's head coach calls the side he came up on and his
    // coordinator the other (C37); with no coordinator he calls both.
    let off_head = coach.is_none_or(|c| w.coaches[c].came_up.offense());
    let (oc, dc) = if seat {
        (coach, coach)
    } else if off_head {
        (coach, school.dc.or(coach))
    } else {
        (school.oc.or(coach), coach)
    };
    let level = crate::calendar::Level::HighSchool;
    let guide = super::guide::team_guidelines(w, coach, oc, dc, level, seat);
    let inst = stored.unwrap_or_else(|| school_install(w, h, seat));
    let ab = coach.map(|c| *w.coaches[c].ability());
    let aware = players.iter().map(|p| p.a(crate::attributes::Attr::FootballIq)).sum::<f64>() / players.len() as f64;
    let offense = season::coach_caller(w, oc, inst.off);
    let install = clamp(crate::game::roster::absorb(aware) + 0.01 * (offense.aggressiveness - 10.0), 0.35, 1.0);
    let mut team = GameTeam::new(
        school.name.clone(),
        players,
        crate::game::Staff {
            offense,
            defense: season::coach_caller(w, dc, 7 + inst.shell),
            game_management: ab.map_or(9.0, |a| a.game_management),
            strictness: ab.map_or(10.0, |a| a.discipline),
            off_scheme: inst.offense(),
            front: inst.front(),
            shell: inst.shell(),
            playbook: 1.0,
            sportsmanship: coach.map_or(10.0, |c| w.coaches[c].sportsmanship),
            nickel: coach.map_or(0.88, |c| clamp(0.88 + 0.08 * key_normal(0x41C8, c as u64), 0.55, 0.99)),
            year: w.year,
            // His eye and what the school's budget buys (proposed, not confirmed).
            film_study: clamp(0.7 * ab.map_or(9.0, |a| a.scheme) + 7.0 * school.resources, 1.0, 20.0),
            install,
            style: coach.map_or_else(Default::default, |c| w.coaches[c].style()),
            guide: Default::default(),
            motivation: ab.map_or(10.0, |a| a.motivation),
            temper: coach.map_or(10.0, |c| w.coaches[c].personality.temper),
        },
    );
    super::guide::apply(w, &mut team, guide, coach, coach);
    team.give_orders(crate::player::Level::HS);
    if varsity && w.seat_school() == Some(h) {
        team.order = w.seat_order();
        crate::training::seat::sit_hurt(w, &mut team);
        crate::training::seat::prepare(w, &mut team);
    }
    // Line chemistry: upperclassmen up front have played together.
    let line_vets = dressed
        .iter()
        .filter(|&&i| {
            let p = &w.people[i];
            matches!(p.player.position, Position::OT | Position::IOL)
                && p.school_age() >= 17
                && p.last_role == Some(PlayingTime::Starter)
        })
        .count()
        .min(5);
    team.line_chem = 0.25 + 0.15 * line_vets as f64;
    super::chemistry::apply(w, &mut team, super::chemistry::Unit::HighSchool);
    Some((team, ids))
}

/// Your school's game team for its next game (the week after the last one
/// played; the first week before the season), for the depth chart screen.
/// A school's team for its film (update 3, lane 9): the team as it would
/// dress for a game this week, for a tape played on the engine.
pub(crate) fn film_team(w: &World, h: usize) -> Option<(GameTeam, Vec<(u32, usize)>)> {
    hs_team(w, h, this_week(w, h))
}

/// The school's games played so far this season.
fn this_week(w: &World, h: usize) -> usize {
    w.hs_season.as_ref().map_or(0, |hs| {
        let st = &hs.states[w.high_schools[h].state];
        st.weeks.iter().chain(&st.rounds).filter(|&&d| d < w.today).count()
    })
}

pub(crate) fn seat_team(w: &World, h: usize) -> Option<GameTeam> {
    hs_team(w, h, this_week(w, h)).map(|t| t.0)
}

/// Whether a high school's home field is artificial turf this season
/// (update 3, F24): richer and bigger schools turf theirs first (proposed,
/// not confirmed).
pub(crate) fn hs_turf(w: &World, h: usize) -> bool {
    let school = &w.high_schools[h];
    let means = 0.5 * school.resources + 0.5 * (school.enrollment as f64 / 2500.0).min(1.0);
    super::season::field_turf(w.options.seed as u64, (1 << 40) | h as u64, &super::season::TURF_HS, w.year, means)
}

/// A Friday night's conditions at a home school: the day's weather, a
/// crowd as loud as the school is big, and the wind along the field.
fn hs_conditions(
    w: &World,
    h: usize,
    wx: &crate::calendar::weather::Weather,
    seed: u64,
    key: u64,
) -> crate::game::Conditions {
    let school = &w.high_schools[h];
    // The crowd: a big school's Friday night is louder (proposed, not confirmed).
    let crowd = clamp(0.2 + 0.3 * (school.enrollment as f64 / 2500.0).sqrt() + 0.1 * school.prestige, 0.1, 0.7);
    let turf = hs_turf(w, h);
    crate::game::Conditions {
        temperature: wx.temperature,
        wind: wx.wind,
        wind_along: key_uniform(seed ^ 0xA10, key) * 2.0 - 1.0,
        rain: wx.rain,
        snow: wx.snow,
        crowd,
        turf,
        ..Default::default()
    }
}

/// The day a school brings into a game (dialogue D4): the crosstown rival
/// is the one game that means more; the playoffs the engine reads itself.
fn hs_day(w: &World, h: usize, opp: usize, dressed: &GameTeam) -> crate::game::talk::Day {
    use super::folk::Team;
    let rival = super::folk::facts::rival(w, Team::High(h)).is_some_and(|r| r.0 == Team::High(opp));
    super::folk::gametalk::day_for(w, Team::High(h), Team::High(opp), 0.0, rival, dressed)
}

/// Your school's game today as the day will play it (section 9): a
/// regular-season Friday or a playoff round.
pub(crate) fn seat_setup(w: &World, me: usize) -> Option<super::live_seat::SeatSetup> {
    let d = w.today;
    let hs = w.hs_season.as_ref()?;
    let s = w.high_schools.get(me)?.state;
    let st = &hs.states[s];
    let code = w.states[s].code.clone();
    let rules = if matches!(code.as_str(), "TX" | "MA") {
        crate::game::Rules::high_school_on_college_rules()
    } else {
        crate::game::Rules::for_level(crate::player::Level::HS)
    };
    // (home, away, week index, seed, key, final, playoff)
    let found = if let Some(k) = st.weeks.iter().position(|&g| g == d) {
        let seed = (w.options.seed as u64 ^ (w.year as u64) << 24 ^ (k as u64) << 8 ^ 0x6761) ^ 0x55;
        hs.groups.iter().filter(|g| g.state == s).find_map(|g| {
            let &(h, a) = g.schedule.get(k)?.iter().find(|&&(h, a)| h == me || a == me)?;
            Some((h, a, k, seed, (h as u64) << 20 ^ a as u64, false, false))
        })
    } else if let Some(r) = st.rounds.iter().position(|&x| x == d) {
        let seed = w.options.seed as u64 ^ (w.year as u64) << 24 ^ (r as u64) << 12 ^ 0x706F;
        let last = st.rounds.len() - 1;
        hs.brackets.iter().filter(|b| b.state as usize == s && !b.done() && b.round as usize == r).find_map(|b| {
            b.games()
                .into_iter()
                .map(|(a, h)| (a as usize, h as usize))
                .find(|&(a, b)| a == me || b == me)
                .map(|(a, b)| (a, b, st.weeks.len() + r, seed, (a as u64) << 20 ^ b as u64, r == last, true))
        })
    } else {
        None
    }?;
    let (h, a, week, seed, key, final_game, playoff) = found;
    // The home town's own climate (U-e); a neutral-site final, the state's.
    let wx = if final_game {
        weather_at(w.options.seed as u64, s, &code, d, 19)
    } else {
        crate::calendar::weather::weather_in(w.options.seed as u64, s, &code, &w.high_schools[h].town, d, 19)
    };
    let (mut home, _) = hs_team(w, h, week)?;
    let (mut away, _) = hs_team(w, a, week)?;
    home.day = hs_day(w, h, a, &home);
    away.day = hs_day(w, a, h, &away);
    let mut setup = crate::game::GameSetup::new(
        &home,
        &away,
        crate::player::Level::HS,
        seed.wrapping_mul(1_000_003).wrapping_add(key),
    );
    setup.rules = rules;
    setup.cond = hs_conditions(w, h, &wx, seed, key);
    setup.neutral = final_game;
    setup.playoff = playoff;
    setup.kickoff = 19.0;
    let params = crate::game::live::Params::of(&setup);
    Some(super::live_seat::SeatSetup {
        level: Level::HighSchool,
        teams: [h, a],
        names: [w.school_name(h).to_string(), w.school_name(a).to_string()],
        home,
        away,
        params,
    })
}

/// The world reader's game log: a high school game's facts (the state,
/// the week, regular or playoff, the teams).
fn hs_head(
    w: &World,
    s: usize,
    week: usize,
    day: Date,
    teams: [usize; 2],
    final_game: bool,
) -> serde_json::Map<String, serde_json::Value> {
    let regular = w.hs_season.as_ref().map_or(usize::MAX, |hs| hs.states[s].weeks.len());
    let mut head = serde_json::Map::new();
    head.insert("level".into(), "hs".into());
    head.insert("season".into(), w.year.into());
    head.insert("state".into(), w.states[s].code.as_str().into());
    head.insert("date".into(), day.to_string().into());
    head.insert("week".into(), (week + 1).into());
    let kind = if final_game {
        "state final"
    } else if week >= regular {
        "playoff"
    } else {
        "regular"
    };
    head.insert("kind".into(), kind.into());
    if week >= regular {
        head.insert("round".into(), (week - regular + 1).into());
    }
    head.insert("teams".into(), serde_json::json!(teams));
    head.insert("neutral".into(), final_game.into());
    head.insert("kickoff".into(), "19:00 local".into());
    head
}

/// Logs a high school game decided off the engine (the stand-in, or a
/// forfeit): the winner only.
fn log_hs_standin(w: &mut World, mut head: serde_json::Map<String, serde_json::Value>, home_won: bool, forfeit: bool) {
    if forfeit {
        head.insert("forfeit".into(), true.into());
    }
    let row = super::wlog::games::standin_row(head, home_won, None);
    if let Some(l) = w.wlog.as_deref() {
        l.game("games-hs", "", &row, None);
    }
}

/// Plays a state's engine games for one week (regular week `week`, or a
/// playoff round) and files the results and lines. Each game is played in
/// its home town's weather (U-e: the town's own climate normals, with the
/// state's shared swing for the day); a final at a neutral site plays in
/// the state's weather (`state_wx`). Returns whether each home team won. A
/// school too short to dress a team forfeits (both short: the home side),
/// as a real one does; no game falls back to the stand-in.
fn engine_games(
    w: &mut World,
    s: usize,
    week: usize,
    games: &[(usize, usize, u64)],
    (day, state_wx): (Date, &crate::calendar::weather::Weather),
    final_game: bool,
    seed: u64,
) -> Vec<bool> {
    if games.is_empty() {
        return Vec::new();
    }
    let rules = if matches!(w.states[s].code.as_str(), "TX" | "MA") {
        crate::game::Rules::high_school_on_college_rules()
    } else {
        crate::game::Rules::for_level(crate::player::Level::HS)
    };
    let seat = w.seat_school();
    let logging = w.logging();
    let trial_log = w.options.keep_game_log && w.recording;
    let world: &World = w;
    // Your game, played live on the game day screens (section 9).
    let live = world.live_result(Level::HighSchool);
    let watch = world.tape_watch(Level::HighSchool);
    // Each game draws from its own seeded stream (from its key), so results
    // are the same on any core count.
    let played: Vec<Result<HsPlayed, bool>> = games
        .par_iter()
        .map(|&(h, a, key)| {
            let ((mut home, hid), (mut away, aid)) = match (hs_team(world, h, week), hs_team(world, a, week)) {
                (Some(x), Some(y)) => (x, y),
                // A forfeit: the home side wins only if it could dress a team.
                (x, _) => return Err(x.is_some()),
            };
            let school = &world.high_schools[h];
            let code = world.states[s].code.as_str();
            let town_wx;
            let wx = if final_game {
                state_wx
            } else {
                town_wx =
                    crate::calendar::weather::weather_in(world.options.seed as u64, s, code, &school.town, day, 19);
                &town_wx
            };
            let cond = hs_conditions(world, h, wx, seed, key);
            home.day = hs_day(world, h, a, &home);
            away.day = hs_day(world, a, h, &away);
            let mut setup = crate::game::GameSetup::new(
                &home,
                &away,
                crate::player::Level::HS,
                seed.wrapping_mul(1_000_003).wrapping_add(key),
            );
            setup.rules = rules;
            setup.cond = cond;
            setup.year = world.year;
            setup.neutral = final_game;
            setup.playoff = week >= world.hs_season.as_ref().unwrap().states[s].weeks.len();
            // Your school's game keeps its setup for the film; Friday night
            // under the lights (7 pm).
            setup.keep = seat.is_some_and(|x| x == h || x == a);
            setup.kickoff = 19.0;
            let mut r = match &live {
                Some((t, r)) if *t == [h, a] => r.clone(),
                _ => crate::game::play_game(&setup),
            };
            let lines = [(0usize, &hid), (1usize, &aid)].map(|(side, ids)| {
                ids.iter()
                    .filter_map(|&(id, i)| {
                        let l = r.stats.players[side].get(&id)?;
                        let on = l.snaps_off + l.snaps_def + l.snaps_st > 0;
                        Some((i, hs_counts(l), on))
                    })
                    .collect()
            });
            let [ha, hb] = r.stats.score;
            // Your game, and your coming opponents' film.
            let kept =
                seat.is_some_and(|x| x == h || x == a) || watch.as_ref().is_some_and(|o| o.watches(h) || o.watches(a));
            let replay = r.replay.take();
            let full = kept.then(|| (r.stats.clone(), r.film.clone(), replay, r.talks.clone()));
            let mut tally = super::charts::Tally::default();
            tally.add_game(&r.stats);
            let log = logging.then(|| {
                let mut head = hs_head(world, s, week, day, [h, a], final_game);
                head.insert("weather".into(), serde_json::json!(wx));
                head.insert("conditions".into(), serde_json::json!(cond));
                let row = super::wlog::games::game_row(head, super::wlog::games::game_body(&r, Some([&home, &away])));
                super::wlog::pack(&row)
            });
            let log = log.flatten();
            let situ = (logging || trial_log).then(|| super::game_log::game_situations(&r.stats, false));
            let injuries = std::mem::take(&mut r.injuries);
            Ok(HsPlayed {
                injuries,
                score: [ha.max(0) as u32, hb.max(0) as u32],
                team: r.stats.team.clone(),
                lines,
                full,
                tally,
                neutral: final_game,
                log,
                situ,
            })
        })
        .collect();
    let mut out = Vec::with_capacity(games.len());
    for (&(h, a, key), g) in games.iter().zip(played) {
        let g = match g {
            Ok(g) => g,
            Err(home_wins) => {
                if w.logging() {
                    let head = hs_head(w, s, week, day, [h, a], final_game);
                    log_hs_standin(w, head, home_wins, true);
                }
                let (wn, l) = if home_wins { (h, a) } else { (a, h) };
                let hs = w.hs_season.as_mut().unwrap();
                hs.results[wn].wins += 1;
                hs.results[l].losses += 1;
                w.log.hs_forfeits += 1;
                if w.recording {
                    super::records::hs_result(w, wn, true);
                    super::records::hs_result(w, l, false);
                }
                out.push(home_wins);
                continue;
            }
        };
        let home_wins = if g.score[0] == g.score[1] { key & 1 == 0 } else { g.score[0] > g.score[1] };
        if let Some(situ) = g.situ {
            let year = w.year;
            if w.options.keep_game_log && w.recording {
                for (k, v) in &situ.league {
                    let e = w.game_log.level_situations.entry((year, "HS".to_string(), k.clone())).or_default();
                    super::game_log::bump(e, v);
                }
            }
            if let (Some(row), Some(l)) = (g.log, w.wlog.as_mut()) {
                l.add_situations(year, "hs", w.states[s].code.as_str(), [h, a], situ);
                l.put_packed("games-hs", row);
            }
        }
        let (wn, l) = if home_wins { (h, a) } else { (a, h) };
        let hs = w.hs_season.as_mut().unwrap();
        hs.results[wn].wins += 1;
        hs.results[l].losses += 1;
        hs.results[h].totals.add(&g.team[0], &g.team[1]);
        hs.results[a].totals.add(&g.team[1], &g.team[0]);
        for (side, school) in [(0usize, h), (1usize, a)] {
            for &(i, c, on) in &g.lines[side] {
                let pos = w.people[i].player.position;
                let e =
                    hs.book.entry(i).or_insert(HsAcc { school: school as u32, pos, games: 0, s: [0; HS_STAT_COUNT] });
                e.games = e.games.saturating_add(on as u8);
                for (x, y) in e.s.iter_mut().zip(c) {
                    *x += y;
                }
            }
        }
        // Injuries outlast the game (update 3, A1); high schools have no
        // medical staff.
        let side = [h, a].iter().position(|&x| seat == Some(x));
        crate::training::seat::game_injuries(w, day, &g.injuries, [0.0; 2], side);
        if let Some((full, film, replay, talks)) = g.full.as_ref() {
            w.keep_seat_game(Level::HighSchool, [h, a], full, replay.as_deref(), talks);
            w.seat_line(Level::HighSchool, [h, a], None, g.neutral);
            if let Some(o) = &watch {
                w.keep_tape(o, Level::HighSchool, [h, a], full, film);
            }
            super::stories::hs_game(w, [h, a], full);
        }
        // The storylines' look after the game (update 2, S2), for the
        // schools people follow.
        let teams = [crate::world::stories::TeamKey::Hs(h as u32), crate::world::stories::TeamKey::Hs(a as u32)];
        if w.recording && (super::storylines::detect::on(w, teams[0]) || super::storylines::detect::on(w, teams[1])) {
            let hs = w.hs_season.as_ref().unwrap();
            let playoff = week >= hs.states[s].weeks.len();
            let played = super::storylines::detect::games::Played {
                level: Level::HighSchool,
                teams,
                score: [g.score[0] as i32, g.score[1] as i32],
                stats: g.full.as_ref().map(|f| &f.0),
                postseason: playoff,
                senior_day: !playoff && hs.marks.get(h).copied().flatten().is_some_and(|m| m.1 == day),
                title: final_game,
                conf_title: false,
                rivalry: super::stories::traditions::hs_rivals(w, h, a),
                playoff,
            };
            super::storylines::detect::games::after(w, &played);
        }
        let year = w.year;
        w.charts.add(year, super::charts::HS + s as u16, &g.tally);
        w.log.hs_engine_games += 1;
        w.log.hs_totals.add(&g.team[0], &g.team[1]);
        w.log.hs_totals.add(&g.team[1], &g.team[0]);
        if w.recording {
            super::records::hs_result(w, wn, true);
            super::records::hs_result(w, l, false);
        }
        out.push(home_wins);
    }
    out
}

/// How a state's writers rate a school (A33; lane 9 review, proposed,
/// not confirmed): only what they can know, as real media polls do. This
/// season's record and scoring margin, and before and early in the season
/// the school's name (prestige), last season's record, playoff run and
/// title, and the starters coming back. That read fades as games are
/// played. Never the team's true strength.
fn poll_score(w: &World, r: &super::season::TeamResult, h: usize) -> f64 {
    let school = &w.high_schools[h];
    let last = school.seasons.iter().rev().find(|x| x.year < w.year);
    let last_pct = last.filter(|x| x.wins + x.losses > 0).map_or(0.5, |x| x.wins as f64 / (x.wins + x.losses) as f64);
    let run = last.map_or(0.0, |x| 0.5 * x.playoff_wins.min(4) as f64 + if x.state_title { 1.0 } else { 0.0 });
    let back = school
        .roster
        .iter()
        .filter(|&&i| w.people.get(i).is_some_and(|p| p.last_role == Some(crate::player::PlayingTime::Starter)))
        .count() as f64;
    let prior = (3.0 * (school.prestige - 0.5) + 2.0 * (last_pct - 0.5) + run + 0.1 * (back - 11.0)).clamp(-4.0, 4.0);
    let g = r.games() as f64;
    let t = &r.totals;
    let margin = if t.games > 0 { (t.points_for - t.points_against) as f64 / t.games as f64 } else { 0.0 };
    r.wins as f64 - r.losses as f64 + 0.1 * margin.clamp(-30.0, 30.0) + prior * 4.0 / (4.0 + g)
}

/// A state's poll (A33): its top five, on what the writers can know.
fn poll(w: &mut World, s: usize, d: Date, which: &str) {
    if !w.recording {
        return;
    }
    let hs = w.hs_season.as_ref().unwrap();
    let mut teams: Vec<(f64, usize)> = w.states[s]
        .schools
        .iter()
        .filter(|&&h| hs.results[h].roster > 0)
        .map(|&h| (poll_score(w, &hs.results[h], h), h))
        .collect();
    teams.sort_by(|a, b| b.0.total_cmp(&a.0));
    let top: Vec<String> = teams
        .iter()
        .take(5)
        .enumerate()
        .map(|(k, &(_, h))| {
            let r = &hs.results[h];
            if r.games() == 0 {
                format!("{}. {}", k + 1, w.school_name(h))
            } else {
                format!("{}. {} ({}-{})", k + 1, w.school_name(h), r.wins, r.losses)
            }
        })
        .collect();
    if top.is_empty() {
        return;
    }
    let text = format!("{which} {} high school poll: {}.", w.states[s].code, top.join(", "));
    let mut e = Event::new(d, Level::HighSchool, Kind::Poll, Loudness::Small, text);
    for &(_, h) in teams.iter().take(5) {
        e = e.hs_link(h, w.school_name(h));
    }
    w.calendar.post(e);
    super::charts::state_poll(w, s, d, which, &teams);
}

/// The playoff field (U-d): each bracket of the state's real format, from
/// its regular season (`hs_playoffs`).
fn seed_playoffs(w: &mut World, s: usize) {
    let hs = w.hs_season.as_ref().unwrap();
    let opps = super::hs_playoffs::opponents(w.high_schools.len(), &hs.played);
    let rounds = hs.states[s].rounds.len();
    let brackets = super::hs_playoffs::build(w, s, &hs.results, &hs.district, &opps, rounds);
    let hs = w.hs_season.as_mut().unwrap();
    hs.brackets.retain(|b| b.state as usize != s);
    for b in &brackets {
        if b.champion != super::hs_playoffs::NONE {
            hs.results[b.champion as usize].state_title = true;
        }
    }
    hs.brackets.extend(brackets);
    // Your school's playoff goals read whether it made the field.
    if let Some(me) = super::front_office::school::your_school(w)
        && w.high_schools[me].state == s
        && w.hs_alive(me)
    {
        w.front_office.school.in_playoffs = true;
    }
}

/// A playoff round: the better seed hosts until the final, which is on a
/// neutral field.
fn play_round(w: &mut World, s: usize, r: usize, d: Date) {
    let seed = w.options.seed as u64 ^ (w.year as u64) << 24 ^ (r as u64) << 12 ^ 0x706F;
    let hs = w.hs_season.as_ref().unwrap();
    let week = hs.states[s].weeks.len() + r;
    let last = hs.states[s].rounds.len() - 1;
    let playing = |b: &super::hs_playoffs::Bracket| b.state as usize == s && !b.done() && b.round as usize == r;
    // Each bracket playing this round: (bracket, home, away, key).
    let mut pairs: Vec<(usize, usize, usize, u64)> = Vec::new();
    for (bi, b) in hs.brackets.iter().enumerate().filter(|x| playing(x.1)) {
        for (a, h) in b.games() {
            let (a, h) = (a as usize, h as usize);
            pairs.push((bi, a, h, (a as u64) << 20 ^ h as u64));
        }
    }
    let code = w.states[s].code.clone();
    let wx = weather_at(w.options.seed as u64, s, &code, d, 19);
    let engine: Vec<(usize, usize, u64)> = pairs.iter().map(|&(_, a, b, key)| (a, b, key)).collect();
    let seat = w.seat_school();
    let played: Vec<Option<bool>> = if hs.on_engine {
        engine_games(w, s, week, &engine, (d, &wx), r == last, seed).into_iter().map(Some).collect()
    } else if let Some(k) = seat.and_then(|x| engine.iter().position(|&(a, b, _)| a == x || b == x)) {
        // Your school's playoff game runs on the engine even in a stand-in season.
        let mut v = vec![None; pairs.len()];
        v[k] = Some(engine_games(w, s, week, &engine[k..k + 1], (d, &wx), r == last, seed)[0]);
        v
    } else {
        vec![None; pairs.len()]
    };
    let off_engine: Vec<bool> = played.iter().map(|x| x.is_none()).collect();
    let hs = w.hs_season.as_mut().unwrap();
    let mut outcomes: Vec<(usize, usize)> = Vec::new();
    let mut by_bracket: HashMap<usize, Vec<(u32, u32)>> = HashMap::new();
    let mut kept: Vec<(u8, u32, u32, u32)> = Vec::new();
    for (&(bi, a, b, key), on) in pairs.iter().zip(played) {
        let a_wins = match on {
            Some(won) => won,
            None => {
                let home = if r == last { 0.0 } else { 0.4 };
                let p = win_chance(
                    hs.results[a].strength - adj(hs, a, week),
                    hs.results[b].strength - adj(hs, b, week),
                    HS_SPREAD,
                    home,
                );
                let won = key_uniform(seed, key) < p;
                let (wn, l) = if won { (a, b) } else { (b, a) };
                hs.results[wn].wins += 1;
                hs.results[l].losses += 1;
                outcomes.push((wn, l));
                won
            }
        };
        let (wn, l) = if a_wins { (a, b) } else { (b, a) };
        by_bracket.entry(bi).or_default().push((wn as u32, l as u32));
        kept.push((r as u8, a as u32, b as u32, wn as u32));
    }
    w.log.hs_games += pairs.len() as u32;
    // The world reader's game log: the games the stand-in decided (the
    // engine's games are logged as they are played).
    if w.logging() {
        for (&(_, a, b, wn), off) in kept.iter().zip(off_engine) {
            if off {
                let head = hs_head(w, s, week, d, [a as usize, b as usize], r == last);
                log_hs_standin(w, head, wn == a, false);
            }
        }
    }
    let hs = w.hs_season.as_mut().unwrap();
    hs.playoff_games.extend(kept);
    for bi in 0..hs.brackets.len() {
        if !playing(&hs.brackets[bi]) {
            continue;
        }
        let res = by_bracket.remove(&bi).unwrap_or_default();
        hs.brackets[bi].advance(&res);
        let c = hs.brackets[bi].champion;
        if c != super::hs_playoffs::NONE {
            hs.results[c as usize].state_title = true;
        }
    }
    if w.recording {
        for (wn, l) in outcomes {
            super::records::hs_result(w, wn, true);
            super::records::hs_result(w, l, false);
        }
    }
}

/// A state's season ends with its finals: the season's book, coaches'
/// records and reputations, all-state teams (A32) and the final poll.
fn finish_state(w: &mut World, s: usize, d: Date) {
    let year = w.year;
    {
        let hs = w.hs_season.as_mut().unwrap();
        hs.states[s].done = true;
        // A bracket still open on finals day (it should not be) crowns its
        // best seed left.
        for b in hs.brackets.iter_mut().filter(|b| b.state as usize == s && !b.done()) {
            if let Some(&c) = b.seeds.iter().find(|&&t| b.alive(t)) {
                b.champion = c;
                hs.results[c as usize].state_title = true;
            }
        }
    }
    poll(w, s, d, "Final");
    all_state(w, s, d);
    file_lines(w, s);
    if !w.recording {
        return;
    }
    let schools = w.states[s].schools.clone();
    // Each school's playoff run: wins and whether it reached the last four.
    let mut runs: HashMap<usize, (u8, bool)> = HashMap::new();
    for b in w.hs_season.as_ref().unwrap().brackets.iter().filter(|b| b.state as usize == s) {
        for (k, &t) in b.seeds.iter().enumerate() {
            runs.insert(t as usize, (b.wins[k], b.semi[k]));
        }
    }
    for h in schools {
        let t = w.hs_season.as_ref().unwrap().results[h].clone();
        if t.games() == 0 {
            continue;
        }
        let school = &mut w.high_schools[h];
        school.seasons.push(HsSeason {
            year,
            wins: t.wins,
            losses: t.losses,
            state_title: t.state_title,
            coach: school.coach,
            points: [t.totals.points_for.max(0) as u32, t.totals.points_against.max(0) as u32],
            playoff_wins: runs.get(&h).map_or(0, |r| r.0),
            semifinal: runs.get(&h).is_some_and(|r| r.1),
            unit: crate::world::high_school::HsUnit {
                games: t.totals.games.min(255) as u8,
                sacks: t.totals.sacks.min(65_535) as u16,
                sacks_allowed: t.totals.sacks_allowed.min(65_535) as u16,
                takeaways: t.totals.takeaways.min(65_535) as u16,
                turnovers: t.totals.turnovers.min(65_535) as u16,
                yards: t.totals.yards.max(0.0) as u32,
                yards_allowed: t.totals.yards_allowed.max(0.0) as u32,
            },
        });
        if t.state_title {
            school.state_titles += 1;
        }
        if let Some(c) = school.coach {
            let stage = w.high_schools[h].prestige;
            let coach = &mut w.coaches[c];
            coach.wins += t.wins;
            coach.losses += t.losses;
            if t.state_title {
                coach.titles += 1;
                coach.career_titles += 1;
            }
            let locker = clamp(t.incidents as f64 / t.roster.max(1) as f64 * 8.0, 0.0, 1.0);
            coach.season_reputation(t.vs_expectation(), locker, true);
            let title = if t.state_title { TitleKind::State } else { TitleKind::None };
            coach.title_reputation(title, true, stage);
        }
        // His coordinators' names move with the season too, at an
        // assistant's pace (C37).
        for c in [w.high_schools[h].oc, w.high_schools[h].dc].into_iter().flatten() {
            w.coaches[c].season_reputation(t.vs_expectation(), 0.0, false);
        }
    }
}

/// A state's season goes on its players' records: each line to his
/// record, and into the state's record books (single season and career
/// in the state, the best ten; section 8, choice 2).
fn file_lines(w: &mut World, s: usize) {
    use super::records::{Book, RecordEntry, Scope};
    let year = w.year;
    let hs = w.hs_season.as_mut().unwrap();
    let mut mine: Vec<(usize, HsAcc)> = Vec::new();
    let schools = &w.high_schools;
    hs.book.retain(|&i, e| {
        if schools[e.school as usize].state == s {
            mine.push((i, e.clone()));
            false
        } else {
            true
        }
    });
    mine.sort_by_key(|x| x.0);
    for (i, e) in mine {
        if e.games == 0 {
            continue;
        }
        let line = HsLine {
            year: year as i16,
            school: e.school,
            pos: e.pos,
            games: e.games,
            s: e.s.map(|v| v.clamp(i16::MIN as i32, i16::MAX as i32) as i16),
        };
        w.people[i].hs_lines.reserve_exact(1);
        w.people[i].hs_lines.push(line);
        if !w.recording {
            continue;
        }
        let p = &w.people[i];
        let name = format!("{} ({})", p.name, w.high_schools[e.school as usize].name);
        let id = p.player.id;
        let career: Vec<&HsLine> = p.hs_lines.iter().filter(|l| w.high_schools[l.school as usize].state == s).collect();
        let first = career.first().map_or(year, |l| l.year as i32);
        let totals: Vec<f64> = HS_BOOKS.iter().map(|&(k, _)| career.iter().map(|l| l.get(k)).sum()).collect();
        let still = p.stage == Stage::HighSchool;
        for (n, &(k, j)) in HS_BOOKS.iter().enumerate() {
            let v = line.get(k);
            if v <= 0.0 {
                continue;
            }
            let e =
                RecordEntry { value: v, id, name: name.clone(), program: usize::MAX, year, from: year, live: false };
            super::records::place_entry(w.stats.books.entry((Scope::State(s), Book::Season(j))).or_default(), e);
            let e = RecordEntry {
                value: totals[n],
                id,
                name: name.clone(),
                program: usize::MAX,
                year,
                from: first,
                live: still,
            };
            let list = w.stats.books.entry((Scope::State(s), Book::Career(j))).or_default();
            list.retain(|x| x.id != id);
            super::records::place_entry(list, e);
        }
    }
}

/// All-state teams (A32): in each classification, the best starter at each
/// spot on the first team, the next on the second.
fn all_state(w: &mut World, s: usize, d: Date) {
    let groups: Vec<(u32, Vec<usize>)> = w
        .hs_season
        .as_ref()
        .unwrap()
        .groups
        .iter()
        .filter(|g| g.state == s)
        .map(|g| (g.class, g.schools.clone()))
        .collect();
    let code = w.states[s].code.clone();
    let mut named = 0;
    for (class, schools) in groups {
        let mut by_pos: [Vec<(f64, usize)>; POSITION_COUNT] = Default::default();
        for &h in &schools {
            for &i in &w.high_schools[h].roster {
                let p = &w.people[i];
                if p.stage != Stage::HighSchool || p.last_role != Some(PlayingTime::Starter) {
                    continue;
                }
                // Games missed count against him: voters reward a full season.
                let perf = p.last_performance.unwrap_or(p.player.overall()) - 0.6 * p.cal.hs_grade_out as f64;
                by_pos[p.player.position.index()].push((perf, i));
            }
        }
        for pos in Position::ALL {
            let list = &mut by_pos[pos.index()];
            list.sort_by(|a, b| b.0.total_cmp(&a.0));
            // A first and a second team per classification, as many at each
            // spot as start; a smaller world names fewer, rounded by a keyed
            // draw so the count is right on average.
            let per_team = crate::league::depth_slots(pos).starters as f64 * w.options.scale;
            let draw = |t: u64| {
                let u = key_uniform(
                    w.options.seed as u64 ^ 0xA115 ^ (s as u64) << 24 ^ (class as u64) << 16 ^ t,
                    pos.index() as u64,
                );
                (per_team + u).floor() as usize
            };
            let (first, second) = (draw(1), draw(2));
            for (rank, &(_, i)) in list.iter().take(first + second).enumerate() {
                let team = if rank < first { 2 } else { 1 };
                let p = &mut w.people[i];
                p.cal.all_state = p.cal.all_state.max(team);
                let age = p.player.age - 1;
                let which = if team == 2 { "first-team" } else { "second-team" };
                let cl = super::hs_playoffs::format(&code)
                    .class_label(class)
                    .map_or_else(|| format!("class {}", class + 1), str::to_string);
                p.note(format!("Named {which} all-state ({code} {cl}) at {age}."));
                named += 1;
            }
        }
    }
    w.log.hs_all_state += named;
    if w.recording {
        let text = format!("{code} all-state teams are named.");
        w.calendar.post(Event::new(d, Level::HighSchool, Kind::Award, Loudness::Small, text));
    }
}

/// A test Saturday (A30): juniors and seniors take the SAT or ACT (on the
/// SAT scale), best score kept. Most take it in the spring of junior year
/// and retake in the fall; scores follow grades and work habits (proposed,
/// not confirmed: mean 1050 at a 2.9 GPA, 300 points per GPA point, 90 a
/// sitting).
fn test_day(w: &mut World, d: Date) {
    let (y, m, _) = d.parts();
    let seed = w.options.seed as u64 ^ (d.0 as u64) << 16 ^ 0x5341;
    let mut taken = 0;
    for p in w.people.iter_mut() {
        if p.stage != Stage::HighSchool {
            continue;
        }
        // Juniors in the spring, seniors in the fall; seniors still short retake in the spring.
        let junior_spring = m <= 6 && p.class_of == y + 1;
        let senior_fall = m >= 8 && p.class_of == y + 1;
        let senior_retake = m <= 6 && p.class_of == y && p.cal.test_score.is_some_and(|s| s < 1000);
        if !(junior_spring || senior_fall || senior_retake) {
            continue;
        }
        let id = p.player.id as u64;
        let chance = match p.cal.test_score {
            None => 0.3,
            Some(s) if s < 1000 => 0.25,
            Some(_) => 0.05,
        };
        if key_uniform(seed, id) >= chance {
            continue;
        }
        let raw = 1050.0
            + 300.0 * (p.bg.gpa - 2.9)
            + 20.0 * (p.player.personality.work_ethic - 10.0)
            + 90.0 * key_normal(seed ^ 1, id);
        let score = (clamp(raw, 400.0, 1600.0) / 10.0).round() as u32 * 10;
        p.cal.test_score = Some(p.cal.test_score.map_or(score, |s| s.max(score)));
        taken += 1;
    }
    if w.recording && taken > 0 {
        let which = if SAT_DATES.iter().any(|&(mm, n)| mm == m && Date::nth_weekday(y, mm, Weekday::Sat, n) == d) {
            "SAT"
        } else {
            "ACT"
        };
        let text = format!("{which} test day: {taken} high school players sit the test.");
        w.calendar.post(Event::new(d, Level::Recruiting, Kind::Academic, Loudness::Small, text));
    }
}

// --- Spring ball as a breakout stage (T4.1) ---------------------------------------------

/// How right the mix is for a spring step (T4.1): work ethic,
/// coachability, a good position coach, open reps on the depth chart, a body
/// still filling out and room under his hidden ceiling. About zero is average.
fn spring_mix(w: &World, i: usize, teaching: f64, open: bool) -> f64 {
    let p = &w.people[i];
    let pers = &p.player.personality;
    let pos = p.player.position;
    let mut room = 0.0;
    let mut n = 0.0;
    for key in crate::attributes::Attr::ALL {
        if crate::positions::weight(pos, key) == 0.0 {
            continue;
        }
        room += (p.player.ceilings.get(key) - p.player.attributes.get(key)).max(0.0);
        n += 1.0;
    }
    let room = room / f64::max(n, 1.0);
    let body = if p.player.age <= 19 { 0.3 } else { 0.0 };
    0.3 * (pers.work_ethic - 10.0) / 3.0
        + 0.2 * (pers.coachability - 10.0) / 3.0
        + 0.25 * (teaching - 10.0) / 3.0
        + 0.35 * (room - 8.0) / 4.0
        + if open { 0.35 } else { -0.2 }
        + body
}

/// The share of eligible players who make a real spring step: about the top
/// 3% (proposed, not confirmed).
const SPRING_SHARE: f64 = 0.03;
/// The share of notices that land (T4.1: some don't pan out): about half
/// (the build's pick, proposed, not confirmed). Only a noticed player can
/// step, so a staff tips about twice as many players as step.
const NOTICES_LAND: f64 = 0.5;

/// The age on his latest season row (`Person::row_age`).
fn row_age(p: &super::Person) -> u32 {
    p.row_age()
}

/// An early enrollee in his first college spring (F2): his latest row is
/// still his high school senior season, so a spring step there belongs to
/// his first college row, which opens with it at fall camp.
fn before_first_college_row(p: &super::Person) -> bool {
    p.stage == Stage::College && p.player.history.last().is_none_or(|h| !h.level.is_college())
}

/// The value the top `share` of `values` clear.
fn top_bar(values: &[f64], share: f64) -> f64 {
    if values.is_empty() {
        return f64::INFINITY;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| b.total_cmp(a));
    let k = ((share * v.len() as f64).round() as usize).clamp(1, v.len());
    v[k - 1]
}

/// Whether a player's spot is open: no more than one healthy player ahead of
/// him at his position on the roster.
fn open_reps(w: &World, roster: &[usize], i: usize) -> bool {
    let p = &w.people[i];
    let mine = p.player.overall();
    let ahead = roster
        .iter()
        .filter(|&&j| {
            j != i
                && w.people[j].player.position == p.player.position
                && matches!(w.people[j].stage, Stage::College | Stage::HighSchool)
                && w.people[j].player.overall() > mine
        })
        .count();
    ahead <= crate::league::depth_slots(p.player.position).starters
}

/// Spring steps a player has made: at most two in a career (part B 43, Cain's call).
fn spring_steps(p: &super::Person) -> usize {
    p.player.history.iter().filter(|r| r.spring_gain.is_some()).count() + p.cal.spring_gain_ahead.is_some() as usize
}

/// The spring candidates: (person, his position coach's teaching, open reps).
fn college_spring_candidates(w: &World) -> Vec<(usize, f64, bool)> {
    let year = w.year;
    // Read only, so worked out on all cores; the list keeps the people order.
    w.people
        .par_iter()
        .enumerate()
        .filter_map(|(i, p)| {
            // True freshmen (early enrollees), redshirt freshmen and sophomores.
            if p.stage != Stage::College || p.college_seasons > 1 || p.class_of < year - 1 || spring_steps(p) >= 2 {
                return None;
            }
            // Short of the grades after the fall term: no spring practice (A23).
            if p.cal.fall_ineligible {
                return None;
            }
            let c = p.college?;
            let prog = &w.programs[c];
            let group = super::coach::Group::of(p.player.position);
            let teaching =
                prog.position_coaches[group as usize].or(prog.head_coach).map_or(9.0, |c| w.coaches[c].teaching(group));
            Some((i, teaching, open_reps(w, &prog.roster, i)))
        })
        .collect()
}

fn hs_spring_candidates(w: &World) -> Vec<(usize, f64, bool)> {
    w.people
        .par_iter()
        .enumerate()
        .filter_map(|(i, p)| {
            if p.stage != Stage::HighSchool || !(15..=16).contains(&p.school_age()) || spring_steps(p) >= 2 {
                return None;
            }
            let school = &w.high_schools[p.hs];
            if !hs_calendar(&w.states[school.state].code).spring_ball {
                return None;
            }
            let group = super::coach::Group::of(p.player.position);
            let teaching = school.coach.map_or(7.0, |c| w.coaches[c].teaching(group));
            Some((i, teaching, open_reps(w, &school.roster, i)))
        })
        .collect()
}

/// Notices early in spring (T4.1): the staff reads the mix, sharper with a
/// better staff, so some notices don't pan out. A step never comes without
/// one.
fn notices(w: &mut World, cands: &[(usize, f64, bool)], d: Date, college: bool) {
    let seed = w.options.seed as u64 ^ (w.year as u64) << 20 ^ if college { 0x5350 } else { 0x5348 };
    let mut n = 0;
    let reads: Vec<f64> = cands
        .iter()
        .map(|&(i, teaching, open)| {
            let mix = spring_mix(w, i, teaching, open);
            let sd = clamp(0.45 - 0.03 * (teaching - 10.0), 0.2, 0.8);
            mix + sd * key_normal(seed, w.people[i].player.id as u64)
        })
        .collect();
    let bar = top_bar(&reads, SPRING_SHARE / NOTICES_LAND);
    for (&(i, _, _), &read) in cands.iter().zip(&reads) {
        if read < bar {
            continue;
        }
        n += 1;
        let p = &mut w.people[i];
        p.cal.spring_notice = true;
        let first = p.name.split_whitespace().next().unwrap_or("He").to_string();
        let what = match super::coach::Group::of(p.player.position) {
            super::coach::Group::OL | super::coach::Group::DL => {
                "keeps his weight up and keeps winning his one-on-ones"
            }
            super::coach::Group::QB => "keeps reading it this fast and keeps his feet clean",
            _ => "keeps stacking good days and keeps his footwork sharp",
        };
        let age = row_age(p);
        p.note(format!("Spring notice at {age}: if {first} {what}, he'll likely make a big step."));
        if college && p.stars >= 4 {
            let text = format!("Spring practice: {} is turning heads.", p.name);
            let prog = p.college;
            let mut e =
                Event::new(d, Level::College, Kind::Breakout, Loudness::Small, text).person(p.player.id, &p.name);
            if let Some(c) = prog {
                e = e.program(c);
            }
            w.calendar.post(e);
        }
    }
    w.log.spring_notices += n;
}

/// By the spring game the step lands, or the notice fizzles (T4.1). Only a
/// noticed player can step: the better half of the notices by the mix land.
/// The step closes 12-30% of the gap to his ceilings (proposed, not
/// confirmed) and goes on his latest season row, so the row's end runs
/// through it. An early enrollee's first college spring step opens his
/// first college row instead (F2).
fn jumps(w: &mut World, cands: &[(usize, f64, bool)], d: Date, college: bool) {
    let seed = w.options.seed as u64 ^ (w.year as u64) << 20 ^ if college { 0x5347 } else { 0x5346 };
    let (mut breakouts, mut fizzles) = (0, 0);
    let noticed: Vec<(usize, f64, bool)> =
        cands.iter().copied().filter(|&(i, _, _)| w.people[i].cal.spring_notice).collect();
    // The mix is the cause; a little luck in how it lands.
    let lands: Vec<f64> = noticed
        .iter()
        .map(|&(i, teaching, open)| {
            spring_mix(w, i, teaching, open) + 0.25 * key_normal(seed, w.people[i].player.id as u64)
        })
        .collect();
    let bar = top_bar(&lands, NOTICES_LAND);
    for (&(i, _, _), &land) in noticed.iter().zip(&lands) {
        let id = w.people[i].player.id as u64;
        let p = &mut w.people[i];
        p.cal.spring_notice = false;
        let mut gain = 0.0;
        if land >= bar {
            let share = 0.12 + 0.18 * key_uniform(seed ^ 1, id);
            let before = p.player.attributes;
            gain = crate::development::spring_jump(&mut p.player, share, &mut p.rng);
            // Under half a point is no step: he had no room left to make one.
            if gain < 0.5 {
                p.player.attributes = before;
            }
        }
        let age = row_age(p);
        if gain < 0.5 {
            fizzles += 1;
            p.note(format!("At {age} the spring step he was tipped for didn't come."));
            continue;
        }
        if college && before_first_college_row(p) {
            p.cal.spring_gain_ahead = Some(p.cal.spring_gain_ahead.unwrap_or(0.0) + gain);
        } else {
            crate::development::record_spring_step(&mut p.player, gain);
        }
        breakouts += 1;
        let r#where = if college { "spring practice" } else { "spring ball" };
        p.note(format!(
            "Broke out in {where} at {age}, as his coaches had hinted: +{gain:.1} overall by the spring game."
        ));
        if college && (p.stars >= 4 || gain >= 4.0) {
            let text = format!("{} breaks out in spring practice (+{gain:.1}).", p.name);
            let prog = p.college;
            let mut e =
                Event::new(d, Level::College, Kind::Breakout, Loudness::Medium, text).person(p.player.id, &p.name);
            if let Some(c) = prog {
                e = e.program(c);
            }
            w.calendar.post(e);
        }
    }
    w.log.spring_breakouts += breakouts;
    w.log.spring_fizzles += fizzles;
}

/// College spring practice opens: notices (T4.1).
pub(crate) fn spring_notices(w: &mut World) {
    let d = w.today;
    let cands = college_spring_candidates(w);
    notices(w, &cands, d, true);
}

/// The spring game: the college steps land.
pub(crate) fn spring_game(w: &mut World) {
    let d = w.today;
    let cands = college_spring_candidates(w);
    jumps(w, &cands, d, true);
    if w.recording {
        w.calendar.post(Event::new(
            d,
            Level::College,
            Kind::Practice,
            Loudness::Small,
            "Spring games around the country.",
        ));
    }
}

fn spring_notices_hs(w: &mut World, d: Date) {
    let cands = hs_spring_candidates(w);
    notices(w, &cands, d, false);
}

fn spring_jumps_hs(w: &mut World, d: Date) {
    let cands = hs_spring_candidates(w);
    jumps(w, &cands, d, false);
}

// --- All-star games --------------------------------------------------------------------

/// Places on a January high school all-star game's two rosters by
/// position (part D 6): about a real roster's shape, with one kicker,
/// punter and snapper a side.
const ALL_STAR_PLACES: [(Position, usize); 14] = [
    (Position::QB, 6),
    (Position::RB, 6),
    (Position::WR, 12),
    (Position::TE, 4),
    (Position::OT, 8),
    (Position::IOL, 8),
    (Position::EDGE, 8),
    (Position::DT, 8),
    (Position::LB, 10),
    (Position::CB, 10),
    (Position::S, 8),
    (Position::K, 2),
    (Position::P, 2),
    (Position::LS, 2),
];

/// A January high school all-star game names its rosters in the senior
/// fall (part B 35): the All-American Bowl in mid-October, the Polynesian
/// Bowl from what is left in November. Invitations follow the services'
/// ranking at each position (part D 5), the specialists the camps' stars;
/// the Polynesian Bowl looks first at players of Polynesian heritage.
pub(crate) fn all_star_rosters(w: &mut World, name: &'static str) {
    if w.year < all_star_first_season(name) {
        return;
    }
    let d = w.today;
    let class = w.year + 1;
    let seed = w.options.seed as u64 ^ (class as u64) << 16 ^ name.len() as u64;
    let poly = name.starts_with("Polynesian");
    let scale = w.options.scale;
    let played = super::recruiting::hs_played(w);
    let mut invited = 0u32;
    for (pos, places) in ALL_STAR_PLACES {
        let mut pool: Vec<(f64, usize)> = w
            .people
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                p.stage == Stage::HighSchool
                    && p.class_of == class
                    && p.player.position == pos
                    && p.cal.hs_all_star.is_none()
                    && (p.stars >= 3 || p.camp_stars >= 3)
            })
            .map(|(i, p)| {
                let stars = if pos.is_specialist() { p.camp_stars } else { p.stars } as f64;
                let read = p.cal.service_read.unwrap_or(p.player.overall());
                let mut score = 10.0 * stars
                    + 0.25 * film_raw(p, &played)
                    + 0.75 * read
                    + 0.3 * key_normal(seed, p.player.id as u64);
                if poly && p.ethnicity == super::ethnicity::Ethnicity::Polynesian {
                    score += 15.0;
                }
                (score, i)
            })
            .collect();
        pool.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let size = ((places as f64 * scale).round() as usize).max(1);
        for &(_, i) in pool.iter().take(size) {
            let p = &mut w.people[i];
            p.cal.hs_all_star = Some(name);
            let age = row_age(p);
            p.note(format!("Invited to the {name} at {age}, in his senior season."));
            invited += 1;
        }
    }
    w.log.all_star_invites += invited;
    if w.recording {
        let text = format!("The {name} names its rosters: {invited} seniors invited.");
        w.calendar.post(Event::new(d, Level::HighSchool, Kind::Signing, Loudness::Small, text));
    }
}

/// The game itself in January: everyone invited plays, early enrollees
/// included (part D 7: they report to campus right after it).
pub(crate) fn all_star_game(w: &mut World, name: &'static str) {
    if w.year < all_star_first_season(name) {
        return;
    }
    let d = w.today;
    let class = w.year + 1;
    let mut played = 0;
    for p in w.people.iter_mut() {
        if p.class_of == class
            && p.cal.hs_all_star == Some(name)
            && matches!(p.stage, Stage::HighSchool | Stage::College)
        {
            // The age on his senior season's row (the game is in its offseason).
            let age = row_age(p);
            p.note(format!("Played in the {name} at {age}."));
            played += 1;
        }
    }
    if w.recording {
        let text = format!("The {name} is played, {played} seniors on its rosters.");
        w.calendar.post(Event::new(d, Level::HighSchool, Kind::Game, Loudness::Medium, text));
    }
}

/// The first football season of a January high school all-star game (from
/// memory, to verify): the All-American Bowl's first game was Dec 30, 2000
/// (the 2000 season), the Polynesian Bowl's in January 2017 (the 2016
/// season).
pub(crate) fn all_star_first_season(name: &str) -> i32 {
    if name.starts_with("Polynesian") { 2016 } else { 2000 }
}

/// Each state's all-star game for its graduating seniors, late May.
fn state_all_stars(w: &mut World, d: Date) {
    let year = w.year;
    let mut states_played = std::collections::BTreeSet::new();
    for p in w.people.iter_mut() {
        if p.stage == Stage::HighSchool && p.class_of == year && p.cal.all_state >= 2 {
            states_played.insert(p.home_state);
            let age = row_age(p);
            p.note(format!("Played in his state's high school all-star game at {age}."));
        }
    }
    if w.recording && !states_played.is_empty() {
        let text = format!("{} states hold their high school all-star games.", states_played.len());
        w.calendar.post(Event::new(d, Level::HighSchool, Kind::Game, Loudness::Small, text));
    }
}

/// The year's high school dates in a state, for tests and the screens.
pub fn state_season_dates(code: &str, year: i32) -> (Date, Date, Date) {
    let c = hs_calendar(code);
    (
        shifted(c.start.0, c.start.1, year, labor_day),
        shifted(c.playoffs.0, c.playoffs.1, year, thanksgiving),
        shifted(c.finals.0, c.finals.1, year, thanksgiving),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldOptions;
    use std::collections::BTreeSet;

    #[test]
    fn high_school_regular_seasons_have_no_rematches() {
        let w = World::empty(WorldOptions { scale: 1.0, ..Default::default() });
        let hs = new_season(&w);
        let mut games = vec![0u32; w.high_schools.len()];
        for (s, st) in hs.states.iter().enumerate() {
            let mut met = BTreeSet::new();
            for k in 0..st.weeks.len() {
                let mut busy = BTreeSet::new();
                for g in hs.groups.iter().filter(|g| g.state == s) {
                    for &(h, a) in &g.schedule[k] {
                        assert!(met.insert((h.min(a), h.max(a))), "{} schools meet twice", w.states[s].code);
                        assert!(busy.insert(h) && busy.insert(a), "a school plays twice in one week");
                        games[h] += 1;
                        games[a] += 1;
                    }
                }
            }
        }
        // Byes come only from odd counts and the tiny states: under 1% of
        // school-weeks, and under 10% even in DC (25 schools in 4 classes).
        let byes = |state: Option<usize>| {
            let (mut slots, mut played) = (0, 0);
            for s in w.high_schools.iter().filter(|s| state.is_none_or(|x| s.state == x)) {
                slots += hs.states[s.state].weeks.len() as u32;
                played += games[s.id];
            }
            1.0 - played as f64 / slots as f64
        };
        assert!(byes(None) < 0.01, "bye share {:.3}", byes(None));
        let dc = w.states.iter().position(|s| s.code == "DC").unwrap();
        assert!(byes(Some(dc)) < 0.1, "DC bye share {:.3}", byes(Some(dc)));
        // The same seed gives the same schedule.
        let again = new_season(&w);
        assert!(hs.groups.iter().zip(&again.groups).all(|(a, b)| a.schedule == b.schedule));
        // Part B 40: most games are inside the classification (district
        // games and nearby non-district games of the same size).
        let (mut inside, mut all) = (0, 0);
        for g in &hs.groups {
            for week in &g.schedule {
                for &(h, a) in week {
                    all += 1;
                    inside += (w.high_schools[h].classification == w.high_schools[a].classification) as u32;
                }
            }
        }
        assert!(inside as f64 / all as f64 > 0.6, "inside share {inside}/{all}");
        // Florida plays in its seven classifications.
        let fl = w.states.iter().position(|s| s.code == "FL").unwrap();
        assert_eq!(hs.groups.iter().filter(|g| g.state == fl).count(), 7);
    }

    /// Every live season plays every high school game on the engine, even
    /// with the history's engine seasons turned off; only the quick-check
    /// knob holds it back.
    #[test]
    fn live_seasons_play_high_school_games_on_the_engine() {
        let mut w = World::empty(WorldOptions { scale: 0.02, hs_engine_years: None, ..Default::default() });
        assert!(hs_on_engine(&w, 2026) && hs_on_engine(&w, 2031));
        assert!(!hs_on_engine(&w, 2025), "history seasons follow hs_engine_years");
        w.options.hs_engine_years = Some(12);
        assert!(hs_on_engine(&w, 2014) && !hs_on_engine(&w, 2013));
        w.options.engine_from = Some(2100);
        assert!(!hs_on_engine(&w, 2026));
    }

    #[test]
    fn january_all_star_games_start_in_their_real_seasons() {
        assert_eq!(all_star_first_season("All-American Bowl"), 2000);
        assert_eq!(all_star_first_season("Polynesian Bowl"), 2016);
    }
}

/// A school in its classification's standings: (school, wins, losses).
pub type HsStanding = (usize, u32, u32);

/// A high school game on this season's schedule, for the screens.
#[derive(Clone, Debug)]
pub struct HsFixture {
    pub date: Date,
    pub home: usize,
    pub away: usize,
    /// A playoff game (rounds are filled in as the field is set).
    pub playoff: bool,
}

impl World {
    /// The schools still alive with a school in its playoff bracket.
    pub fn hs_playoff_field(&self, h: usize) -> Vec<usize> {
        let Some(hs) = self.hs_season.as_ref() else { return Vec::new() };
        hs.brackets
            .iter()
            .filter(|b| b.alive(h as u32))
            .flat_map(|b| b.seeds.iter().copied().filter(|&t| b.alive(t)))
            .map(|t| t as usize)
            .filter(|&x| x != h)
            .collect()
    }

    /// A school's regular season this year (empty before the schedule is
    /// made in the summer).
    pub fn hs_fixtures(&self, h: usize) -> Vec<HsFixture> {
        let Some(hs) = self.hs_season.as_ref() else { return Vec::new() };
        let Some(state) = self.high_schools.get(h).map(|s| s.state) else { return Vec::new() };
        let mut out = Vec::new();
        // A game is filed under the home team's classification, so a road
        // game at a school of another class is in that class's group.
        for g in hs.groups.iter().filter(|g| g.state == state) {
            let weeks = &hs.states[g.state].weeks;
            for (k, week) in g.schedule.iter().enumerate() {
                if let (Some(&(home, away)), Some(&date)) =
                    (week.iter().find(|&&(a, b)| a == h || b == h), weeks.get(k))
                {
                    out.push(HsFixture { date, home, away, playoff: false });
                }
            }
        }
        out.sort_by_key(|f| f.date);
        out
    }

    /// A school's next playoff game once its bracket sets it: (date, home,
    /// away), the better seed hosting.
    pub(crate) fn hs_playoff_next(&self, h: usize) -> Option<(Date, usize, usize)> {
        let hs = self.hs_season.as_ref()?;
        let state = self.high_schools.get(h)?.state;
        let st = hs.states.get(state)?;
        hs.brackets.iter().filter(|b| b.state as usize == state && !b.done()).find_map(|b| {
            let (home, away) = b.games().into_iter().find(|g| g.0 as usize == h || g.1 as usize == h)?;
            Some((*st.rounds.get(b.round as usize)?, home as usize, away as usize))
        })
    }

    /// A school's classification this season: its name and every school's
    /// (school, wins, losses), best first.
    pub fn hs_standings(&self, h: usize) -> Option<(String, Vec<HsStanding>)> {
        let hs = self.hs_season.as_ref()?;
        let g = hs.groups.iter().find(|g| g.schools.contains(&h))?;
        let mut rows: Vec<(usize, u32, u32)> = g
            .schools
            .iter()
            .filter(|&&s| hs.results.get(s).is_some_and(|r| r.roster > 0))
            .map(|&s| (s, hs.results[s].wins, hs.results[s].losses))
            .collect();
        rows.sort_by_key(|&(_, wn, l)| (std::cmp::Reverse(wn), l));
        let f = super::hs_playoffs::format(&self.states[g.state].code);
        let name = f.class_label(g.class).map_or_else(|| format!("class {}", g.class + 1), str::to_string);
        Some((format!("{} {name}", self.states[g.state].code), rows))
    }

    /// Whether a school is in its state's playoff field and still alive.
    pub fn hs_alive(&self, h: usize) -> bool {
        self.hs_season.as_ref().is_some_and(|hs| hs.brackets.iter().any(|b| b.alive(h as u32)))
    }

    /// The day this year's new class joins the high school rosters.
    pub fn hs_class_day(&self) -> Option<Date> {
        self.hs_season.as_ref().map(|hs| hs.class_day)
    }

    /// A school's state season this year: its regular-season Fridays, its
    /// playoff rounds and whether it is over.
    pub fn hs_weeks(&self, h: usize) -> Option<(Vec<Date>, Vec<Date>, bool)> {
        let hs = self.hs_season.as_ref()?;
        let st = hs.states.get(self.high_schools.get(h)?.state)?;
        Some((st.weeks.clone(), st.rounds.clone(), st.done))
    }

    /// A school's record this season, if its season has started. On the
    /// new football year's first day, before that day runs, the season held
    /// is still last year's, which is not this season's record.
    pub fn hs_record(&self, h: usize) -> Option<(u32, u32)> {
        let hs = self.hs_season.as_ref()?;
        let st = hs.states.get(self.high_schools.get(h)?.state)?;
        if st.start.year() < self.year {
            return None;
        }
        let r = hs.results.get(h)?;
        Some((r.wins, r.losses))
    }

    /// The schools whose players have engine lines in the season being
    /// played (their lines are filed at each state's finals).
    pub fn hs_schools_with_lines(&self) -> std::collections::BTreeSet<usize> {
        self.hs_season.as_ref().map_or_else(Default::default, |hs| {
            hs.book.values().filter(|e| e.games > 0).map(|e| e.school as usize).collect()
        })
    }
}

/// Whether a school won or shared its district this season: the best
/// district record among its district's schools (update 2, lane 2).
pub(crate) fn district_champion(w: &World, h: usize) -> bool {
    let Some(s) = w.hs_season.as_ref() else { return false };
    let Some(&d) = s.district.get(h) else { return false };
    if d == super::hs_playoffs::NONE {
        return false;
    }
    let mut rec: HashMap<u32, (u32, u32)> = HashMap::new();
    for &(win, lose) in &s.played {
        if s.district.get(win as usize) == Some(&d) && s.district.get(lose as usize) == Some(&d) {
            rec.entry(win).or_default().0 += 1;
            rec.entry(lose).or_default().1 += 1;
        }
    }
    let pct = |x: &(u32, u32)| x.0 as f64 / (x.0 + x.1).max(1) as f64;
    let Some(mine) = rec.get(&(h as u32)).map(pct) else { return false };
    rec.values().all(|x| pct(x) <= mine + 1e-9)
}
