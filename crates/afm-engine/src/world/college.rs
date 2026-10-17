//! Section 4: the college season on the calendar (topic 3, A18-A28). Fall
//! camp sets depth charts and the year's development; games are played on
//! their Saturdays on the match engine, in the weather of the day and the
//! place (A34, A37, A38); polls come out on their days (A18); conference
//! title games, selection day (A19, A20), bowls, bowl practices (A21) and
//! the playoff run by each era's format (topic 11); FCS, D2 and JUCO run
//! their own playoffs. Opt-outs (topic 3.2) and the four-game redshirt rule
//! (2018 on) are decided on their days.
//!
//! Rankings and award picks are placeholders built from results and stats
//! until the voter mechanics are designed (proposed, not confirmed).

use std::collections::{BTreeMap, HashMap, HashSet};

use rayon::prelude::*;

use super::coach::TitleKind;
use super::data::{DIVISION_COUNT, Division};
use super::news::{Channel, NewsItem, NewsKind};
use super::schedule::{self, Game, GameKind, Played, Site, Tag};
use super::season::{self, COLLEGE_SPREAD, CoachContext, TeamResult, key_normal, key_uniform, win_chance};
use super::{Stage, World, division_level, geo};
use crate::calendar::data::{BOWLS, CELEBRATION_BOWL, INDOOR_PROGRAMS, RIVALRIES, et_offset};
use crate::calendar::weather::{self, Severe};
use crate::calendar::{CfbPostseason, Date, Event, Kind, Level as CalLevel, Loudness};
use crate::game::{Conditions, GameTeam, Venue, quick_game_in};
use crate::player::PlayingTime;
use crate::rng::clamp;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollKind {
    CoachesPreseason,
    ApPreseason,
    /// The weekly Coaches Poll, out on Sunday before the AP poll.
    Coaches,
    Ap,
    /// The BCS standings (1998-2013).
    Bcs,
    /// The playoff committee's ranking (2014 on).
    Cfp,
    /// The final poll after the bowls.
    Final,
}

impl PollKind {
    pub fn name(self) -> &'static str {
        match self {
            PollKind::CoachesPreseason => "preseason Coaches Poll",
            PollKind::ApPreseason => "preseason AP poll",
            PollKind::Coaches => "Coaches Poll",
            PollKind::Ap => "AP poll",
            PollKind::Bcs => "BCS standings",
            PollKind::Cfp => "CFP rankings",
            PollKind::Final => "final AP poll",
        }
    }
}

/// A ranking release: the top 25 FBS teams, best first.
#[derive(Clone, Debug)]
pub struct Poll {
    pub date: Date,
    pub kind: PollKind,
    pub top: Vec<usize>,
}

/// A player's season stat line, kept for the award picks.
#[derive(Clone, Copy, Debug, Default)]
pub struct Line {
    pub program: usize,
    pub games: u32,
    pub pass_yds: f64,
    pub pass_td: u32,
    pub int_thrown: u32,
    pub rush_yds: f64,
    pub rush_td: u32,
    pub rec: u32,
    pub rec_yds: f64,
    pub rec_td: u32,
    pub tackles: u32,
    pub tfl: f64,
    pub sacks: f64,
    pub ints: u32,
    pub fg_made: u32,
    /// Passes broken up.
    pub pbus: u32,
    /// The season's play as film graders see it: the sum of the per-play
    /// grades (the game's PFF-style grade, -2 to +2 a play).
    pub grade_sum: f64,
    /// Field goals tried and made by distance band (under 30, 30-39, 40-49, 50+).
    pub fg_att_band: [u32; 4],
    pub fg_made_band: [u32; 4],
}

/// A single-elimination bracket in standard seeding order.
#[derive(Clone, Debug)]
pub(crate) struct Bracket {
    pub division: Division,
    /// Teams by bracket slot; None is a bye.
    pub slots: Vec<Option<usize>>,
    /// Each round's dates, games alternating between them.
    pub dates: Vec<Vec<Date>>,
    /// Each round's game names (the bowls hosting playoff games), if any.
    pub names: Vec<Vec<&'static str>>,
    /// Rounds from this one on are at neutral sites.
    pub neutral_from: usize,
    pub round: usize,
    pub pending: Vec<usize>,
    pub seed: HashMap<usize, usize>,
    pub champion: Option<usize>,
}

/// The standard seed order of a bracket of `n` (a power of two): 1 v n, ...
fn seed_order(n: usize) -> Vec<usize> {
    let mut v = vec![1usize];
    while v.len() < n {
        let m = v.len() * 2;
        v = v.iter().flat_map(|&s| [s, m + 1 - s]).collect();
    }
    v
}

impl Bracket {
    fn new(division: Division, seeds: &[usize], dates: Vec<Vec<Date>>, neutral_from: usize) -> Bracket {
        let size = seeds.len().next_power_of_two().max(2);
        let slots = seed_order(size).into_iter().map(|s| seeds.get(s - 1).copied()).collect();
        let seed = seeds.iter().enumerate().map(|(k, &t)| (t, k + 1)).collect();
        Bracket {
            division,
            slots,
            dates,
            names: Vec::new(),
            neutral_from,
            round: 0,
            pending: Vec::new(),
            seed,
            champion: None,
        }
    }
}

pub(crate) struct CollegeSeason {
    pub year: i32,
    pub games: Vec<Game>,
    pub results: Vec<TeamResult>,
    pub teams: Vec<Option<GameTeam>>,
    pub coaches: Vec<CoachContext>,
    pub banned: Vec<bool>,
    pub on_engine: bool,
    pub started: bool,
    /// Offensive and defensive snaps, and games played (any snap), by player id.
    pub snaps: HashMap<u32, (u32, u32)>,
    pub lines: HashMap<u32, Line>,
    /// Players who sit out games from now on, and why.
    pub out: HashMap<u32, &'static str>,
    pub polls: Vec<Poll>,
    pub brackets: Vec<Bracket>,
    /// National champions by division (FBS, FCS, D2, JUCO).
    pub champions: [Option<usize>; DIVISION_COUNT],
    pub finished: bool,
    /// Regular-season wins and losses already credited to head coaches.
    pub credited: Vec<(u32, u32)>,
    /// Award winners: (award, player id, program).
    pub awards: Vec<(&'static str, u32, usize)>,
    /// Teams in a bowl or playoff game.
    pub bowl_teams: HashSet<usize>,
    /// Each rivalry's record so far (A28), by index into `RIVALRIES`,
    /// carried from season to season.
    pub series: Vec<Series>,
    /// The rivalry between two programs (lower id first), and the program
    /// listed first in it.
    pub rivalry_of: HashMap<(usize, usize), (usize, usize)>,
    /// The Commander-in-Chief's Trophy's holder, carried from season to season.
    pub academy_holder: Option<usize>,
}

/// A rivalry's record over the world's history (A28): wins by its first
/// and second listed programs, the first season counted, and the trophy's
/// holder (the last rivalry game's winner).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Series {
    pub wins: [u32; 2],
    pub ties: u32,
    pub since: Option<i32>,
    pub holder: Option<usize>,
    /// The real all-time record and trophy holder are loaded (part B 29, 30).
    pub real: bool,
}

/// A new season: its schedule is built at the start of the football year
/// (real schedules are set long before; schedule release is an event).
pub(crate) fn new_season(w: &World) -> CollegeSeason {
    let np = w.programs.len();
    let by_name: HashMap<&str, usize> = w.programs.iter().map(|p| (p.real_name.as_str(), p.id)).collect();
    let rivalry_of = RIVALRIES
        .iter()
        .enumerate()
        .filter_map(|(k, r)| {
            let (a, b) = (*by_name.get(r.a)?, *by_name.get(r.b)?);
            Some(((a.min(b), a.max(b)), (k, a)))
        })
        .collect();
    let mut series =
        w.college_prev.as_ref().map_or_else(|| vec![Series::default(); RIVALRIES.len()], |c| c.series.clone());
    // The real all-time records and trophy holders come in when the history
    // starts being kept (part B 29, 30): through 1995 for a 1996 start,
    // through 1983 for a world that keeps it from 1984. Games in the hidden
    // warm-up years before don't count.
    if w.recording {
        for (k, r) in RIVALRIES.iter().enumerate() {
            let s = &mut series[k];
            if s.real {
                continue;
            }
            let Some(h) = super::data::rivalry_past(r.a, r.b) else { continue };
            let (wins, ties, holder) = if w.year <= 1984 { h.thru_1983 } else { h.thru_1995 };
            let side = |i: usize| by_name.get(if i == 0 { r.a } else { r.b }).copied();
            *s = Series { wins, ties, since: Some(h.first), holder: holder.and_then(side), real: true };
        }
    }
    CollegeSeason {
        year: w.year,
        games: schedule::build(w, w.year, &w.dates),
        results: vec![TeamResult::default(); np],
        teams: Vec::new(),
        coaches: Vec::new(),
        banned: w.programs.iter().map(|p| p.era.banned(w.year)).collect(),
        on_engine: w.options.engine_from.is_none_or(|from| w.year >= from),
        started: false,
        snaps: HashMap::new(),
        lines: HashMap::new(),
        out: HashMap::new(),
        polls: Vec::new(),
        brackets: Vec::new(),
        champions: [None; DIVISION_COUNT],
        finished: false,
        credited: vec![(0, 0); np],
        awards: Vec::new(),
        bowl_teams: HashSet::new(),
        series,
        rivalry_of,
        academy_holder: w.college_prev.as_ref().and_then(|c| c.academy_holder),
    }
}

/// A team's projected strength before the season: its best players at each
/// starting spot (what preseason polls and media days read).
pub(crate) fn projected_strength(w: &World, prog: usize) -> f64 {
    let p = &w.programs[prog];
    let mut by_pos: HashMap<crate::positions::Position, Vec<f64>> = HashMap::new();
    for &i in &p.roster {
        let q = &w.people[i];
        if q.stage == Stage::College && !q.player.position.is_specialist() {
            by_pos.entry(q.player.position).or_default().push(q.player.overall());
        }
    }
    let (mut sum, mut n) = (0.0, 0.0);
    for (pos, mut v) in by_pos {
        v.sort_by(|a, b| b.total_cmp(a));
        for x in v.into_iter().take(crate::league::depth_slots(pos).starters) {
            sum += x;
            n += 1.0;
        }
    }
    if n == 0.0 { 30.0 } else { sum / n }
}

/// Fall camp: staffs set depth charts and the year's development lands
/// (topic 7: one checkpoint a year until training, section 6b, splits it).
pub(crate) fn start(w: &mut World) {
    if w.college.as_ref().is_none_or(|c| c.started) {
        return;
    }
    for prog in w.programs.iter_mut() {
        prog.sideline = None;
    }
    let who: Vec<bool> = w.people.iter().map(|p| p.stage == Stage::College).collect();
    season::reconsider_positions(w, &who);
    let mut roles = season::college_roles(w);
    for (i, p) in w.people.iter().enumerate() {
        if p.stage == Stage::PrepSchool {
            roles.push((i, PlayingTime::Rotation));
        }
    }
    season::develop(w, &roles);
    season::record_lineups(w);
    let results = season::college_results(w);
    let on_engine = w.college.as_ref().unwrap().on_engine;
    let teams: Vec<Option<GameTeam>> =
        if on_engine { w.programs.par_iter().map(|p| season::game_team(w, p)).collect() } else { Vec::new() };
    let coaches: Vec<CoachContext> = w.programs.iter().map(|p| season::coach_context(w, p)).collect();
    #[cfg(feature = "debug")]
    crate::debug::capture_teams(w.year, &teams, &results.iter().map(|r| r.strength).collect::<Vec<_>>());
    // Academic sits and suspended or dismissed players sit out. A redshirt
    // stays on the game roster: the staff plays him when he is needed, and
    // the games he plays decide whether he keeps the year (settled January
    // 2 by the era's rule).
    let mut out = HashMap::new();
    for p in w.people.iter() {
        if p.stage != Stage::College {
            continue;
        }
        if p.cal.sitting_transfer {
            out.insert(p.player.id, "sitting out a transfer year");
        } else if p.cal.academic_sit {
            out.insert(p.player.id, "academically ineligible");
        } else if p.cal.trouble {
            out.insert(p.player.id, "suspended");
        }
    }
    let c = w.college.as_mut().unwrap();
    c.results = results;
    c.teams = teams;
    c.coaches = coaches;
    c.out = out;
    c.started = true;
}

/// Weather for a game: from the climate of the home town (or the neutral
/// site) at local kickoff, or indoors.
fn conditions(w: &World, g: &Game, k: usize) -> (Venue, Played, Option<Severe>) {
    let home = &w.programs[g.home];
    let (state, indoor) = match g.site {
        Some(s) => (s.state, s.indoor),
        None => (home.state, INDOOR_PROGRAMS.contains(&home.real_name.as_str())),
    };
    let code = w.states[state].code.as_str();
    let local = (g.kickoff.0 as i32 - et_offset(code)).clamp(9, 23) as u32;
    let seed = w.options.seed as u64;
    let mut wx = if indoor { weather::indoors() } else { weather::weather_at(seed, state, code, g.date, local) };
    // A landfall reaches only the coastal plain (part B 41).
    let city = match g.site {
        Some(s) => s.town,
        None => home.city.as_str(),
    };
    if wx.severe == Some(Severe::Hurricane) && !weather::hurricane_reaches(code, city) {
        wx.severe = None;
    }
    let along = (key_uniform(seed ^ 0xA10, (g.date.0 as u64) << 16 ^ k as u64) * 2.0 - 1.0).clamp(-1.0, 1.0);
    let altitude = match g.site {
        Some(s) => geo::town_altitude(s.town),
        None => geo::college_altitude(&home.real_name),
    };
    // The crowd (trial 4 topic 3): a big stadium is louder, a night game,
    // a rivalry and a game that matters to the home side louder still.
    // Real home edges are about 2 points at every level, so small crowds
    // still count (proposed, not confirmed).
    let crowd = clamp(
        0.3 + 0.45 * (home.stadium as f64 / 100_000.0).sqrt()
            + if local >= 18 { 0.08 } else { 0.0 }
            + if g.rivalry.is_some() { 0.05 } else { 0.0 }
            + if g.huge_for[0] { 0.05 } else { 0.0 },
        0.1,
        1.0,
    );
    let cond = Conditions {
        temperature: wx.temperature,
        wind: wx.wind,
        wind_along: along,
        rain: wx.rain,
        snow: wx.snow,
        altitude,
        crowd,
        ..Conditions::default()
    };
    let neutral = g.site.is_some();
    let teams = [g.home, g.away];
    let wear = teams.map(|t| if t == g.home && !neutral { 0.0 } else { road_wear(w, t, state, g.kickoff.0, altitude) });
    let cold_unused = teams.map(|t| {
        if indoor {
            return 0.0;
        }
        let p = &w.programs[t];
        let usual = if INDOOR_PROGRAMS.contains(&p.real_name.as_str()) {
            72.0
        } else {
            weather::usual_high(&w.states[p.state].code, g.date)
        };
        clamp((usual - wx.temperature - 15.0) / 25.0, 0.0, 1.0) * clamp((45.0 - wx.temperature) / 15.0, 0.0, 1.0)
    });
    let venue = Venue { cond, neutral, wear, cold_unused };
    let mut played = Played {
        temperature: wx.temperature,
        wind: wx.wind,
        rain: wx.rain,
        snow: wx.snow,
        indoor,
        ..Played::default()
    };
    if wx.lightning && !indoor {
        played.delay = weather::lightning_delay(seed, g.date.0 as u64 * 1009 + k as u64);
    }
    (venue, played, wx.severe)
}

/// A travelling team's road wear (trial 4 topic 3), extra fatigue per play
/// as a share: the distance, the time zones, a kickoff early or late on its
/// body clock and thin air it is not used to. Fitted with the crowd to the
/// real home margin (proposed, not confirmed).
fn road_wear(w: &World, team: usize, state: usize, kickoff_et: u16, altitude: f64) -> f64 {
    let p = &w.programs[team];
    let miles = geo::miles(p.state, state).min(2500.0);
    let (here, home) = (et_offset(&w.states[state].code), et_offset(&w.states[p.state].code));
    let body = kickoff_et as i32 - home;
    let early = (11 - body).max(0) as f64;
    let late = (body - 22).max(0) as f64;
    let thin = clamp((altitude - geo::college_altitude(&p.real_name).max(3000.0)) / 4000.0, 0.0, 1.0);
    0.015 * miles / 1000.0 + 0.008 * (here - home).abs() as f64 + 0.01 * early + 0.008 * late + 0.08 * thin
}

/// A redshirt from fall camp: the staff means him to sit this season.
fn camp_redshirt(p: &super::Person) -> bool {
    p.stage == Stage::College
        && p.last_role == Some(PlayingTime::Redshirt)
        && p.redshirt_used
        && !p.cal.count_pending
        && !p.cal.academic_sit
}

/// The camp redshirts a staff keeps off the field today (F14). Before 2018
/// any game cost a redshirt his year, so a staff did not play one; from
/// 2018 it plays him in up to four games and stops there (settle_eligibility).
/// A redshirt whose year is already gone, or who still has games under the
/// four-game rule, costs nothing today and is never held. When absences
/// leave his position short of its two-deep even with those, the best of
/// the rest play, only as many as fill it, whatever it costs (pulling the
/// redshirt).
pub(crate) fn held_redshirts(w: &World, cs: &CollegeSeason) -> HashSet<u32> {
    let four = w.rules.four_game_redshirt;
    let mut held = HashSet::new();
    for prog in &w.programs {
        let mut able = [0usize; crate::positions::POSITION_COUNT];
        let mut planned: Vec<(crate::positions::Position, f64, u32)> = Vec::new();
        for &i in &prog.roster {
            let p = &w.people[i];
            if p.stage != Stage::College || cs.out.contains_key(&p.player.id) {
                continue;
            }
            if camp_redshirt(p) {
                planned.push((p.player.position, p.player.overall(), p.player.id));
            } else {
                able[p.player.position.index()] += 1;
            }
        }
        if planned.is_empty() {
            continue;
        }
        // First those who cost nothing today: the year already counts (any
        // game before 2018, five or more from 2018), or games are left
        // under the four-game rule.
        let mut rest = Vec::new();
        for (pos, ov, id) in planned {
            let games = cs.snaps.get(&id).map_or(0, |s| s.1);
            let gone = games > 0 && (!four || games > 4);
            if gone || (four && games < 4) {
                able[pos.index()] += 1;
            } else {
                rest.push((pos, ov, id));
            }
        }
        // Then the best of the rest, only while the position is short.
        rest.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.2.cmp(&b.2)));
        for (pos, _, id) in rest {
            if able[pos.index()] < season::two_deep(pos) {
                able[pos.index()] += 1;
            } else {
                held.insert(id);
            }
        }
    }
    held
}

/// The day's college games.
pub(crate) fn play_day(w: &mut World, date: Date) {
    let Some(cs) = w.college.as_ref() else { return };
    if !cs.started {
        return;
    }
    let mut idx: Vec<usize> =
        (0..cs.games.len()).filter(|&k| cs.games[k].date == date && cs.games[k].played.is_none()).collect();
    if idx.is_empty() {
        return;
    }
    // Severe weather (A38): a hurricane moves a regular-season game to a
    // later open Saturday, or cancels it when there is none. Snow is played in.
    let mut moved = Vec::new();
    for &k in &idx {
        let g = &w.college.as_ref().unwrap().games[k];
        let (_, _, severe) = conditions(w, g, k);
        if severe == Some(Severe::Hurricane) && g.kind == GameKind::Regular && g.site.is_none_or(|s| !s.indoor) {
            moved.push(k);
        }
    }
    for k in moved {
        let cs = w.college.as_mut().unwrap();
        let (a, b) = (cs.games[k].home, cs.games[k].away);
        let busy: HashSet<Date> = cs.games.iter().filter(|g| g.involves(a) || g.involves(b)).map(|g| g.date).collect();
        let later =
            w.dates.cfb_weeks.iter().copied().find(|&d| d > date && !busy.contains(&d) && d <= w.dates.fbs_regular_end);
        let name = format!("{} at {}", w.programs[b].real_name, w.programs[a].real_name);
        let cs = w.college.as_mut().unwrap();
        match later {
            Some(d) => {
                cs.games[k].date = d;
                if let Some(p) = cs.games[k].played.as_mut() {
                    p.moved = true;
                }
                cs.games[k].tags.retain(|_| true);
                let text = format!("{name} is moved to {} because of a hurricane.", d.pretty());
                w.calendar.post(Event::new(date, CalLevel::College, Kind::Weather, Loudness::Medium, text).program(a));
                w.log.weather_moves += 1;
            }
            None => {
                cs.games[k].played = Some(Played { cancelled: true, moved: true, ..Played::default() });
                let text = format!("{name} is cancelled because of a hurricane: a no contest.");
                w.calendar.post(Event::new(date, CalLevel::College, Kind::Weather, Loudness::Medium, text).program(a));
                w.log.weather_cancels += 1;
            }
        }
        idx.retain(|&x| x != k);
    }
    // A program with no players yet (the spin-up before history) plays no games.
    let empty: Vec<usize> = idx
        .iter()
        .copied()
        .filter(|&k| {
            let g = &w.college.as_ref().unwrap().games[k];
            w.programs[g.home].roster.len() < 22 || w.programs[g.away].roster.len() < 22
        })
        .collect();
    for &k in &empty {
        w.college.as_mut().unwrap().games[k].played = Some(Played { cancelled: true, ..Played::default() });
    }
    idx.retain(|k| !empty.contains(k));
    if idx.is_empty() {
        advance_brackets(w);
        return;
    }
    stakes_for_day(w, &idx);
    let cs = w.college.as_ref().unwrap();
    let capped = if cs.on_engine { held_redshirts(w, cs) } else { HashSet::new() };
    let seed = w.options.seed as u64 ^ ((cs.year as u64) << 32) ^ ((date.0 as u64) << 8);
    struct Job {
        k: usize,
        venue: Venue,
        played: Played,
        neutral: bool,
        teams: Option<(GameTeam, GameTeam)>,
        context: [crate::game::garbage::Context; 2],
    }
    let jobs: Vec<Job> = idx
        .iter()
        .map(|&k| {
            let g = &cs.games[k];
            let (venue, played, _) = conditions(w, g, k);
            let strip = |t: &GameTeam| {
                let mut t = t.clone();
                t.players.retain(|p| !cs.out.contains_key(&p.id) && !capped.contains(&p.id));
                t
            };
            let teams = if cs.on_engine {
                match (cs.teams.get(g.home).and_then(|t| t.as_ref()), cs.teams.get(g.away).and_then(|t| t.as_ref())) {
                    (Some(a), Some(b)) => Some((strip(a), strip(b))),
                    _ => None,
                }
            } else {
                None
            };
            let context = [(g.home, g.away), (g.away, g.home)]
                .map(|(me, opp)| season::game_context(&cs.coaches, g.division, &cs.results, me, opp));
            Job { k, venue, played, neutral: g.site.is_some(), teams, context }
        })
        .collect();
    let levels: Vec<crate::player::Level> = jobs
        .iter()
        .map(|j| {
            let g = &cs.games[j.k];
            // A cross-division game is played under the higher level's rules.
            let d = w.programs[g.home].division.min(w.programs[g.away].division);
            division_level(d)
        })
        .collect();
    let outcomes: Vec<Option<crate::game::GameResult>> = jobs
        .par_iter()
        .zip(levels.par_iter())
        .map(|(j, &level)| {
            let (a, b) = j.teams.as_ref()?;
            if a.players.len() < 30 || b.players.len() < 30 {
                return None;
            }
            let s = seed.wrapping_mul(1_000_003).wrapping_add(j.k as u64);
            Some(quick_game_in(a, b, level, s, j.context, j.venue))
        })
        .collect();
    let recording_log = w.options.keep_game_log && w.recording;
    let year = cs.year;
    for (j, r) in jobs.into_iter().zip(outcomes) {
        let k = j.k;
        let mut played = j.played;
        let (home, away, division, kind, conference) = {
            let g = &w.college.as_ref().unwrap().games[k];
            (g.home, g.away, g.division, g.kind, g.conference)
        };
        let home_won = match &r {
            Some(r) => {
                let [ha, hb] = r.stats.score;
                played.score = [ha.max(0) as u32, hb.max(0) as u32];
                if ha == hb { (seed ^ k as u64) & 1 == 0 } else { ha > hb }
            }
            None => {
                let cs = w.college.as_ref().unwrap();
                let adv = if j.neutral { 0.0 } else { 0.8 };
                let p = win_chance(cs.results[home].strength, cs.results[away].strength, COLLEGE_SPREAD, adv);
                let won = w.rng.chance(p);
                // A stand-in score, for polls and records only.
                let margin = (3.0 + 14.0 * w.rng.next()).round() as u32;
                let loser = (10.0 + 14.0 * w.rng.next()).round() as u32;
                played.score = if won { [loser + margin, loser] } else { [loser, loser + margin] };
                won
            }
        };
        played.home_won = home_won;
        let cs = w.college.as_mut().unwrap();
        // Records.
        if kind == GameKind::Regular {
            let adv = if j.neutral { 0.0 } else { 0.8 };
            let pe = win_chance(cs.results[home].preseason, cs.results[away].preseason, w.college_spread, adv);
            cs.results[home].expected += pe;
            cs.results[away].expected += 1.0 - pe;
        }
        let (wi, li) = if home_won { (home, away) } else { (away, home) };
        cs.results[wi].wins += 1;
        cs.results[li].losses += 1;
        if conference {
            cs.results[wi].conf_wins += 1;
            cs.results[li].conf_losses += 1;
        }
        if kind == GameKind::ConferenceTitle {
            cs.results[wi].conference_title = true;
        }
        if let Some(r) = &r {
            let [sa, sb] = &r.stats.team;
            cs.results[home].totals.add(sa, sb);
            cs.results[away].totals.add(sb, sa);
            for (side, prog) in [(0usize, home), (1usize, away)] {
                for (&id, l) in &r.stats.players[side] {
                    let e = cs.snaps.entry(id).or_default();
                    e.0 += l.snaps_off + l.snaps_def;
                    if l.snaps_off + l.snaps_def + l.snaps_st > 0 {
                        e.1 += 1;
                    }
                    if matches!(w.programs[prog].division, Division::FBS | Division::FCS) {
                        let s = cs.lines.entry(id).or_default();
                        s.program = prog;
                        s.games += 1;
                        s.pass_yds += l.pass_yds;
                        s.pass_td += l.pass_td;
                        s.int_thrown += l.int;
                        s.rush_yds += l.rush_yds;
                        s.rush_td += l.rush_td;
                        s.rec += l.rec;
                        s.rec_yds += l.rec_yds;
                        s.rec_td += l.rec_td;
                        s.tackles += l.tackles + l.assists;
                        s.tfl += l.tfl;
                        s.sacks += l.sacks;
                        s.ints += l.ints;
                        s.fg_made += l.fg_made.iter().sum::<u32>();
                        s.pbus += l.pbus;
                        s.grade_sum += l.grade_sum;
                        for b in 0..4 {
                            s.fg_att_band[b] += l.fg_att[b];
                            s.fg_made_band[b] += l.fg_made[b];
                        }
                    }
                }
            }
            if recording_log {
                let d = w.programs[home].division;
                let part = super::game_log::situations(d, [home, away], &r.stats);
                w.game_log.add(year, d, [home, away], &r.stats, part);
                if w.programs[home].division != w.programs[away].division
                    && let Some(last) = w.game_log.games.last_mut()
                {
                    last.cross = true;
                }
            }
        }
        cs.games[k].played = Some(played);
        crown_title_game(cs, k);
        if played.delay > 0 {
            w.log.lightning_delays += 1;
        }
        w.log.games += 1;
        if played.snow {
            w.log.snow_games += 1;
        }
        if played.rain {
            w.log.rain_games += 1;
        }
        if !played.indoor && played.temperature < 32.0 {
            w.log.freezing_games += 1;
        }
        let _ = division;
        result_event(w, k);
    }
    advance_brackets(w);
}

/// Keeps each rivalry's record (A28): every meeting of the two programs in
/// the world's history counts, and the trophy goes with the rivalry game.
/// Returns the trophy's holder before this game.
fn record_series(w: &mut World, k: usize) -> Option<usize> {
    let recording = w.recording;
    let cs = w.college.as_mut()?;
    let g = &cs.games[k];
    let p = g.played.filter(|p| !p.cancelled)?;
    let &(r, first) = cs.rivalry_of.get(&(g.home.min(g.away), g.home.max(g.away)))?;
    let winner = if p.home_won { g.home } else { g.away };
    let trophy_game = g.rivalry == Some(r);
    let year = cs.year;
    let s = &mut cs.series[r];
    let before = s.holder;
    if recording {
        s.wins[(winner != first) as usize] += 1;
        s.since.get_or_insert(year);
        if trophy_game {
            s.holder = Some(winner);
        }
    }
    before
}

/// The Commander-in-Chief's Trophy (A28): a game between two service
/// academies is a leg of it, and the Army-Navy game settles it. The academy
/// that won both its games takes it; when all three split 1-1 (or a leg was
/// not played) the holder keeps it. Returns what to add to the result's
/// line, and a sentence after it.
fn academy_trophy(w: &mut World, k: usize) -> (String, String) {
    use crate::calendar::data::{COMMANDER_IN_CHIEF, RivalrySlot};
    let (trophy, names) = COMMANDER_IN_CHIEF;
    let recording = w.recording;
    let Some(cs) = w.college.as_mut() else { return Default::default() };
    let leg = |g: &Game| {
        g.kind == GameKind::Regular
            && g.rivalry.is_some_and(|r| names.contains(&RIVALRIES[r].a) && names.contains(&RIVALRIES[r].b))
    };
    let g = &cs.games[k];
    if !leg(g) || g.played.is_none_or(|p| p.cancelled) {
        return Default::default();
    }
    if g.rivalry.is_some_and(|r| RIVALRIES[r].slot != RivalrySlot::ArmyNavy) {
        return (format!(", a leg of {trophy}"), String::new());
    }
    // Wins in the season's legs: with one game each way, only one academy
    // can win two.
    let mut wins: BTreeMap<usize, u32> = BTreeMap::new();
    for g in cs.games.iter().filter(|g| leg(g)) {
        if let Some(p) = g.played.filter(|p| !p.cancelled) {
            *wins.entry(if p.home_won { g.home } else { g.away }).or_default() += 1;
        }
    }
    let before = cs.academy_holder;
    let outright = wins.iter().find(|(_, n)| **n >= 2).map(|(&t, _)| t);
    if recording && outright.is_some() {
        cs.academy_holder = outright;
    }
    let name = |t: usize| w.programs[t].real_name.as_str();
    let mut t = trophy.to_string();
    t[..1].make_ascii_uppercase();
    let after = match (outright, before) {
        (Some(o), Some(h)) if o == h => format!(" {} keeps {trophy}, 2-0 against the other academies.", name(o)),
        (Some(o), Some(_)) => format!(" {} claims {trophy}, 2-0 against the other academies.", name(o)),
        (Some(o), None) => format!(" {} wins {trophy}, 2-0 against the other academies.", name(o)),
        (None, Some(h)) => format!(" The academies split their games, so {} keeps {trophy}.", name(h)),
        (None, None) => format!(" The academies split their games; {t} goes unclaimed."),
    };
    (String::new(), after)
}

/// A playoff game's round, from the rounds left in its bracket.
fn playoff_round(cs: &CollegeSeason, k: usize) -> Option<&'static str> {
    let b = cs.brackets.iter().find(|b| b.pending.contains(&k))?;
    let left = b.slots.len().trailing_zeros() as usize;
    Some(match left {
        2 => "semifinal",
        3 => "quarterfinal",
        l if l >= b.dates.len() => "first round",
        _ => "second round",
    })
}

/// Posts a game's result: loud for marked and big games, a news line otherwise.
fn result_event(w: &mut World, k: usize) {
    let holder = record_series(w, k);
    let (leg, academy) = academy_trophy(w, k);
    let cs = w.college.as_ref().unwrap();
    let g = &cs.games[k];
    let Some(p) = g.played else { return };
    let huge = g.huge_for.contains(&true);
    if g.loud == Loudness::Small && g.rivalry.is_none() && g.kind == GameKind::Regular && !huge {
        return;
    }
    let (wi, li, ws, ls) =
        if p.home_won { (g.home, g.away, p.score[0], p.score[1]) } else { (g.away, g.home, p.score[1], p.score[0]) };
    // Playoff games name their round; a title game below the FBS names its
    // title; a BCS title game in a host bowl says what was at stake.
    let level = if g.division == Division::FBS { "CFP" } else { g.division.name() };
    let what = match (g.kind, g.name) {
        (GameKind::Playoff(_), name) => match (name, playoff_round(cs, k).unwrap_or("first round")) {
            (Some(n), round) => format!(" in the {n}, {} {round}", crate::text::with_article(level)),
            (None, round @ ("first round" | "second round")) => format!(" in the {level} {round}"),
            (None, round) => format!(" in the {level} {round}s"),
        },
        (GameKind::Title, Some(n)) if BOWLS.iter().any(|b| b.name == n) => {
            format!(" in the {n} for the national title")
        }
        (GameKind::Title, None) => format!(" for the {} title", g.division.name()),
        (_, Some(n)) => format!(" in the {n}"),
        (GameKind::ConferenceTitle, _) => format!(" for the {} title", w.programs[g.home].conference),
        _ => match g.rivalry {
            Some(r) => {
                let rv = &RIVALRIES[r];
                let name = rv.name_in(cs.year);
                match rv.trophy {
                    // The holder keeps the trophy; it is claimed when it
                    // changes hands; the first meeting on record wins it.
                    Some(t) => {
                        let t = if t == name { "the trophy" } else { t };
                        let how = match holder {
                            Some(h) if h == wi => "to keep",
                            Some(_) => "to claim",
                            None => "to win",
                        };
                        format!(" in {name} {how} {t}{leg}")
                    }
                    None => format!(" in {name}{leg}"),
                }
            }
            None => String::new(),
        },
    };
    // The series record after this game, once the two have met twice.
    let series = cs.rivalry_of.get(&(g.home.min(g.away), g.home.max(g.away))).map_or(String::new(), |&(r, first)| {
        let s = &cs.series[r];
        let (a, b) = (s.wins[0], s.wins[1]);
        let since = s.since.unwrap_or(cs.year);
        // A real record is all-time, ties included (part B 29).
        let ties = if s.ties > 0 { format!("-{}", s.ties) } else { String::new() };
        let span = if s.real { String::new() } else { format!(" since {since}") };
        let all = if s.real { "all-time " } else { "" };
        if a + b < 2 {
            String::new()
        } else if a == b {
            format!(" The {all}series is tied {a}-{b}{ties}{span}.")
        } else {
            let other = if first == g.home { g.away } else { g.home };
            let leader = if a > b { first } else { other };
            format!(" {} leads the {all}series {}-{}{ties}{span}.", w.programs[leader].real_name, a.max(b), a.min(b))
        }
    });
    let weather = if p.snow {
        " in the snow"
    } else if p.rain {
        " in the rain"
    } else if !p.indoor && p.temperature < 25.0 {
        " in the cold"
    } else {
        ""
    };
    let text = format!(
        "{} beat {} {ws}-{ls}{weather}{what}.{series}{academy}",
        w.programs[wi].real_name, w.programs[li].real_name
    );
    let loud = g.loud.max(if g.kind != GameKind::Regular { Loudness::Medium } else { Loudness::Small });
    // A game huge for one of its teams (a rivalry-week game, a sixth win for
    // a bowl, a coach's job) posts as big at least, so Continue stops on it.
    let loud = if huge { loud.max(Loudness::Big) } else { loud };
    // Below the FBS a game is huge for its two teams, but national news
    // carries it as big at most (proposed, not confirmed).
    let loud = if g.division == Division::FBS { loud } else { loud.min(Loudness::Big) };
    // Why it is loud (part B 22).
    let text = if loud >= Loudness::Big && !g.why.is_empty() {
        let label = if loud == Loudness::Huge { "Huge" } else { "Big" };
        format!("{text} ({label}: {}.)", g.why.join(", "))
    } else {
        text
    };
    let date = g.date;
    let title = g.kind == GameKind::Title && g.division == Division::FBS;
    let mut e = Event::new(date, CalLevel::College, Kind::Game, loud, text).program(wi);
    if title {
        e = e.memorable();
    }
    w.calendar.post(e);
}

// --- Rankings (A18) ------------------------------------------------------------------

/// A ranking score for every team in a division (proposed, not confirmed:
/// the voters' own minds come later). Wins matter most, then a margin- and
/// schedule-adjusted rating, with the preseason view fading over the year
/// and a conference title counted once won.
pub(crate) fn ranking(w: &World, division: Division) -> Vec<(usize, f64)> {
    let Some(cs) = w.college.as_ref() else { return Vec::new() };
    let members: Vec<usize> = w
        .programs
        .iter()
        .filter(|p| p.division == division && !p.roster.is_empty() && p.plays(w.year))
        .map(|p| p.id)
        .collect();
    let set: HashSet<usize> = members.iter().copied().collect();
    let mut games: HashMap<usize, Vec<(usize, f64)>> = HashMap::new();
    for g in cs.games.iter() {
        let Some(p) = g.played.filter(|p| !p.cancelled) else { continue };
        let m = (p.score[0] as f64 - p.score[1] as f64).clamp(-24.0, 24.0);
        games.entry(g.home).or_default().push((g.away, m));
        games.entry(g.away).or_default().push((g.home, -m));
    }
    let mut r: HashMap<usize, f64> = members.iter().map(|&t| (t, 0.0)).collect();
    for _ in 0..20 {
        let mut next = HashMap::new();
        for &t in &members {
            let list = games.get(&t);
            let v = match list {
                Some(l) if !l.is_empty() => {
                    l.iter().map(|&(o, m)| m + if set.contains(&o) { r[&o] } else { -18.0 }).sum::<f64>()
                        / l.len() as f64
                }
                _ => 0.0,
            };
            next.insert(t, v);
        }
        let mean = next.values().sum::<f64>() / next.len().max(1) as f64;
        for v in next.values_mut() {
            *v -= mean;
        }
        r = next;
    }
    let pre: Vec<f64> = members.iter().map(|&t| projected_or(cs, t)).collect();
    let pm = pre.iter().sum::<f64>() / pre.len().max(1) as f64;
    let ps = (pre.iter().map(|x| (x - pm).powi(2)).sum::<f64>() / pre.len().max(1) as f64).sqrt().max(0.5);
    let mut out: Vec<(usize, f64)> = members
        .iter()
        .zip(pre)
        .map(|(&t, pv)| {
            let res = &cs.results[t];
            let n = res.games() as f64;
            let fade = 0.85f64.powf(n);
            let s = 3.0 * res.win_pct() * (1.0 - fade)
                + 0.05 * r[&t]
                + 0.6 * ((pv - pm) / ps) * fade
                + if res.conference_title { 0.25 } else { 0.0 };
            (t, s)
        })
        .collect();
    out.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    out
}

fn projected_or(cs: &CollegeSeason, t: usize) -> f64 {
    let r = &cs.results[t];
    if r.preseason > 0.0 { r.preseason } else { 50.0 }
}

/// Releases a poll (A18).
pub(crate) fn release_poll(w: &mut World, date: Date, kind: PollKind) {
    if w.college.is_none() {
        return;
    }
    let top: Vec<usize> = match kind {
        PollKind::CoachesPreseason | PollKind::ApPreseason => {
            let mut v: Vec<(usize, f64)> = w
                .programs
                .iter()
                .filter(|p| p.division == Division::FBS && !p.roster.is_empty())
                .map(|p| (p.id, projected_strength(w, p.id) + 2.0 * p.prestige))
                .collect();
            v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            v.into_iter().take(25).map(|x| x.0).collect()
        }
        PollKind::Coaches => {
            // Placeholder voters (proposed, not confirmed): the same read of
            // results as the AP voters, with their own small differences.
            let seed = w.options.seed as u64 ^ (date.0 as u64) << 20 ^ 0xC0AC;
            let mut v = ranking(w, Division::FBS);
            let sd = 0.04 * v.first().map_or(1.0, |x| x.1 - v.get(25).map_or(x.1 - 1.0, |y| y.1)).abs().max(0.1);
            for x in v.iter_mut() {
                x.1 += sd * key_normal(seed, x.0 as u64);
            }
            v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            v.into_iter().take(25).map(|x| x.0).collect()
        }
        _ => ranking(w, Division::FBS).into_iter().take(25).map(|x| x.0).collect(),
    };
    let mut top = top;
    // When a title game decided it (the BCS and playoff eras), the national
    // champion is first in the final poll.
    if kind == PollKind::Final
        && w.rules.cfb_postseason != CfbPostseason::PollsOnly
        && let Some(c) = w.college.as_ref().and_then(|cs| cs.champions[0])
    {
        top.retain(|&t| t != c);
        top.insert(0, c);
        // The title game's loser is second (part D 25), as voters rank it.
        let runner_up = w.college.as_ref().and_then(|cs| {
            cs.games
                .iter()
                .find(|g| g.kind == GameKind::Title && g.division == Division::FBS && g.involves(c))
                .and_then(|g| g.played.as_ref().map(|_| if g.home == c { g.away } else { g.home }))
        });
        if let Some(r) = runner_up {
            top.retain(|&t| t != r);
            top.insert(1, r);
        }
        top.truncate(25);
    }
    if top.is_empty() {
        return;
    }
    let loud = match kind {
        PollKind::Cfp if date == w.dates.selection_day => Loudness::Big,
        PollKind::Final => Loudness::Medium,
        PollKind::ApPreseason | PollKind::Cfp | PollKind::Bcs => Loudness::Medium,
        _ => Loudness::Small,
    };
    let text = format!(
        "{}: 1. {}, 2. {}, 3. {}, 4. {}.",
        kind.name(),
        w.programs[top[0]].real_name,
        w.programs[top.get(1).copied().unwrap_or(top[0])].real_name,
        w.programs[top.get(2).copied().unwrap_or(top[0])].real_name,
        w.programs[top.get(3).copied().unwrap_or(top[0])].real_name
    );
    w.calendar.post(Event::new(date, CalLevel::College, Kind::Poll, loud, text));
    w.college.as_mut().unwrap().polls.push(Poll { date, kind, top });
}

/// The latest poll's rank of a team (1-25).
pub(crate) fn rank_of(cs: &CollegeSeason, t: usize) -> Option<usize> {
    cs.polls.last().and_then(|p| p.top.iter().position(|&x| x == t)).map(|r| r + 1)
}

// --- Stakes (topic 8.1, A40, A41) --------------------------------------------------------

/// Tags each of the day's games and sets how loud it is. A game is huge only
/// when it really matters: two ranked teams with a playoff place at stake, a
/// rivalry between contenders, a coach's job on the line in a big game.
/// One game's stakes for the day: the game, its tags, how loud, which side
/// is on the line and why.
type StakeUpdate = (usize, Vec<Tag>, Loudness, [bool; 2], Vec<&'static str>);

fn stakes_for_day(w: &mut World, idx: &[usize]) {
    let Some(cs) = w.college.as_ref() else { return };
    let mut updates: Vec<StakeUpdate> = Vec::new();
    let mut notes: Vec<(usize, String)> = Vec::new();
    // Each team's own race (T8.1): the fewest conference losses in its
    // conference, and the regular-season games it has left.
    let mut conf_best: HashMap<&str, u32> = HashMap::new();
    for (t, prog) in w.programs.iter().enumerate() {
        if prog.division == Division::FBS && !prog.roster.is_empty() && prog.plays(w.year) {
            let e = conf_best.entry(prog.conference.as_str()).or_insert(u32::MAX);
            *e = (*e).min(cs.results[t].conf_losses);
        }
    }
    let mut left = vec![0u32; w.programs.len()];
    for g in cs.games.iter().filter(|g| g.kind == GameKind::Regular && g.played.is_none()) {
        left[g.home] += 1;
        left[g.away] += 1;
    }
    for &k in idx {
        let g = &cs.games[k];
        let mut tags = g.tags.clone();
        let ranked = |t: usize| rank_of(cs, t);
        if g.division == Division::FBS {
            if let (Some(_), Some(_)) = (ranked(g.home), ranked(g.away))
                && !tags.contains(&Tag::Ranked)
            {
                tags.push(Tag::Ranked);
            }
            // A playoff place: a contender late in the year, or any playoff game.
            let contender = |t: usize| {
                let r = &cs.results[t];
                r.games() >= 7 && r.losses <= 2 && ranked(t).is_some_and(|x| x <= 15)
            };
            if (g.kind == GameKind::ConferenceTitle && (contender(g.home) || contender(g.away)))
                || matches!(g.kind, GameKind::Playoff(_) | GameKind::Title)
                || (g.kind == GameKind::Regular && contender(g.home) && contender(g.away))
            {
                tags.push(Tag::PlayoffSpot);
            }
        }
        for t in [g.home, g.away] {
            if cs.coaches.get(t).is_some_and(|c| c.hot_seat)
                && cs.results[t].games() >= 6
                && !tags.contains(&Tag::HotSeat)
            {
                tags.push(Tag::HotSeat);
            }
            let opp = g.opponent(t);
            if cs.coaches.get(t).is_some_and(|c| c.fired_by.contains(&opp)) && !tags.contains(&Tag::CoachReturn) {
                tags.push(Tag::CoachReturn);
            }
        }
        // Players facing a program they left, or playing near home.
        let (mut revenge, mut hometown) = (false, false);
        for t in [g.home, g.away] {
            let opp = g.opponent(t);
            let site_state = g.site.map_or(w.programs[g.home].state, |s| s.state);
            for &i in &w.programs[t].roster {
                let p = &w.people[i];
                if p.last_role != Some(PlayingTime::Starter) {
                    continue;
                }
                if p.path.len() >= 2 && p.path[..p.path.len() - 1].contains(&opp) {
                    revenge = true;
                }
                if p.home_state == site_state && t != g.home && w.programs[t].state != site_state {
                    hometown = true;
                }
            }
        }
        // A school single-season record within reach in this game (A40): one
        // more of his usual games would break it.
        if matches!(g.division, Division::FBS | Division::FCS) {
            for t in [g.home, g.away] {
                for &i in &w.programs[t].roster {
                    let p = &w.people[i];
                    let Some(l) = cs.lines.get(&p.player.id).filter(|l| l.program == t && l.games >= 6) else {
                        continue;
                    };
                    for (k, rec) in w.programs[t].season_records.iter().enumerate() {
                        let Some(rec) = rec else { continue };
                        let now = record_value(l, k);
                        let per_game = now / l.games as f64;
                        if now < rec.value && now + per_game >= rec.value {
                            if !tags.contains(&Tag::Milestone) {
                                tags.push(Tag::Milestone);
                            }
                            let need = rec.value - now;
                            let stat = super::program::RECORD_STATS[k].0;
                            notes.push((
                                t,
                                format!(
                                    "{} needs {need:.0} {stat} to break {}'s single-season record ({:.0}, {} in {}).",
                                    p.name, w.programs[t].real_name, rec.value, rec.holder, rec.year
                                ),
                            ));
                        }
                    }
                }
            }
        }
        if revenge && !tags.contains(&Tag::Revenge) {
            tags.push(Tag::Revenge);
        }
        if hometown && !tags.contains(&Tag::Hometown) {
            tags.push(Tag::Hometown);
        }
        // Loudness: the stakes test.
        let real = tags.iter().filter(|t| matches!(t, Tag::PlayoffSpot | Tag::Ranked)).count()
            + (tags.contains(&Tag::Rivalry) && (ranked(g.home).is_some() || ranked(g.away).is_some())) as usize
            + (tags.contains(&Tag::HotSeat) && tags.contains(&Tag::Rivalry)) as usize;
        // Story hooks (a player facing his old team, a coach's return, a
        // player at home, a record within reach) only colour the text: alone
        // they never lift a result into the inbox (N57, Cain: "No.").
        let story = |t: &Tag| matches!(t, Tag::Revenge | Tag::CoachReturn | Tag::Hometown | Tag::Milestone);
        let tagged = tags.iter().any(|t| !story(t));
        let stakes = real as f64;
        // The reasons, in plain words, for the result's text (part B 22).
        let mut why: Vec<&'static str> = match g.kind {
            GameKind::Title => vec!["the national title"],
            GameKind::Playoff(_) => vec!["a playoff game"],
            GameKind::ConferenceTitle => vec!["a conference title"],
            GameKind::Bowl(_) | GameKind::Celebration => vec!["a bowl game"],
            GameKind::Regular => Vec::new(),
        };
        for (t, reason) in [
            (Tag::Ranked, "both ranked"),
            (Tag::PlayoffSpot, "a playoff place at stake"),
            (Tag::Rivalry, "a rivalry"),
            (Tag::HotSeat, "a coach's job on the line"),
            (Tag::Revenge, "a player facing his old team"),
            (Tag::CoachReturn, "a coach facing his old team"),
            (Tag::Hometown, "a player at home"),
            (Tag::Milestone, "a record within reach"),
        ] {
            if tags.contains(&t) {
                why.push(reason);
            }
        }
        // Bowls by the bowl (part D 28): a major bowl is big, huge with a
        // stake; any other bowl medium, big with a stake.
        let major = g.name.is_some_and(|n| BOWLS.iter().any(|b| b.name == n && b.major));
        let loud = match g.kind {
            GameKind::Title => Loudness::Huge,
            GameKind::Playoff(_) if g.division == Division::FBS => Loudness::Huge,
            GameKind::Bowl(_) if major && stakes >= 1.0 => Loudness::Huge,
            GameKind::Bowl(_) if major => Loudness::Big,
            GameKind::Bowl(_) | GameKind::Celebration if stakes >= 1.0 => Loudness::Big,
            GameKind::Bowl(_) | GameKind::Celebration => Loudness::Medium,
            _ if stakes >= 2.0 => Loudness::Huge,
            _ if stakes >= 1.0 => Loudness::Big,
            _ if tagged => Loudness::Medium,
            _ => Loudness::Small,
        };
        // Each side's own stakes (T8.1, proposed, not confirmed): the
        // rivalry-week (or Army-Navy) game against its rival; any other rivalry with one more
        // stake; otherwise two of a ranked meeting, a playoff place, a
        // conference race between leaders, a sixth win for a bowl in the last
        // game, a coach on the hot seat late in the year. Postseason games
        // are huge for both.
        let rivalry_week = g.rivalry.is_some_and(|r| {
            use crate::calendar::data::RivalrySlot;
            matches!(crate::calendar::data::RIVALRIES[r].slot, RivalrySlot::RivalryWeek | RivalrySlot::ArmyNavy)
        });
        // Below the FBS only the rivalry-week game is read (the other stakes
        // are FBS races): Harvard-Yale is huge for both, as The Game is.
        let mut huge_for = [rivalry_week; 2];
        if g.division == Division::FBS {
            for (side, t) in [g.home, g.away].into_iter().enumerate() {
                let opp = g.opponent(t);
                let r = &cs.results[t];
                let leader = |x: usize| {
                    conf_best
                        .get(w.programs[x].conference.as_str())
                        .is_some_and(|&b| cs.results[x].conf_losses <= b + 1)
                };
                let contender = r.games() >= 7 && r.losses <= 2 && ranked(t).is_some_and(|x| x <= 15);
                let race = g.conference && r.conf_wins + r.conf_losses >= 4 && leader(t) && leader(opp);
                let bowl = g.kind == GameKind::Regular && r.wins == 5 && left[t] == 1;
                let job =
                    tags.contains(&Tag::HotSeat) && cs.coaches.get(t).is_some_and(|c| c.hot_seat) && r.games() >= 8;
                let n = tags.contains(&Tag::Ranked) as u32 + contender as u32 + race as u32 + bowl as u32 + job as u32;
                let rivalry = tags.contains(&Tag::Rivalry);
                // Rivalry week alone is not enough (part D 26): it needs one
                // more real stake; a sixth win for a bowl in the last game
                // is huge on its own (part B 26).
                huge_for[side] =
                    rivalry && n >= 1 || n >= 2 || bowl || g.kind != GameKind::Regular && loud == Loudness::Huge;
                if huge_for[side] || loud >= Loudness::Big {
                    for (on, reason) in [
                        (race, "a conference race between leaders"),
                        (bowl, "a sixth win for a bowl"),
                        (contender && !tags.contains(&Tag::PlayoffSpot), "playoff hopes"),
                    ] {
                        if on && !why.contains(&reason) {
                            why.push(reason);
                        }
                    }
                }
            }
        }
        updates.push((k, tags, loud, huge_for, why));
    }
    let cs = w.college.as_mut().unwrap();
    for (k, tags, loud, huge_for, why) in updates {
        cs.games[k].tags = tags;
        cs.games[k].loud = loud;
        cs.games[k].huge_for = huge_for;
        cs.games[k].why = why;
    }
    let d = w.today;
    for (t, text) in notes {
        w.calendar.post(Event::new(d, CalLevel::College, Kind::News, Loudness::Small, text).program(t));
    }
}

/// A season line's value for a school record (A40), in `RECORD_STATS` order.
fn record_value(l: &Line, k: usize) -> f64 {
    match k {
        0 => l.pass_yds,
        1 => l.rush_yds,
        2 => l.rec_yds,
        _ => l.sacks,
    }
}

/// The season's lines set new school records (A40).
fn update_records(w: &mut World, year: i32) {
    let Some(cs) = w.college.as_ref() else { return };
    let mut broken: Vec<(usize, usize, f64, u32)> = Vec::new();
    for (&id, l) in &cs.lines {
        for (k, &(_, floor)) in super::program::RECORD_STATS.iter().enumerate() {
            let v = record_value(l, k);
            let best = w.programs[l.program].season_records[k].as_ref().map_or(floor, |r| r.value.max(floor));
            if v > best {
                broken.push((l.program, k, v, id));
            }
        }
    }
    // The best line per school and stat sets the record.
    broken.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)).then(b.2.total_cmp(&a.2)).then(a.3.cmp(&b.3)));
    broken.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
    let names: HashMap<u32, String> = w
        .people
        .iter()
        .filter(|p| broken.iter().any(|b| b.3 == p.player.id))
        .map(|p| (p.player.id, p.name.clone()))
        .collect();
    let d = w.today;
    for (prog, k, v, id) in broken {
        let holder = names.get(&id).cloned().unwrap_or_else(|| "a former player".into());
        let had = w.programs[prog].season_records[k].is_some();
        if had && w.recording {
            let stat = super::program::RECORD_STATS[k].0;
            let text = format!(
                "{holder} finishes with {v:.0} {stat}, {} single-season record.",
                crate::text::with_article(&w.programs[prog].real_name)
            );
            w.calendar.post(Event::new(d, CalLevel::College, Kind::News, Loudness::Small, text).program(prog));
        }
        w.programs[prog].season_records[k] = Some(super::program::SeasonRecord { value: v, holder, year });
    }
}

// --- The end of the regular season ------------------------------------------------------

/// Conference standings: (team, conference win share, overall win share).
fn conference_order(cs: &CollegeSeason, members: &[usize], rank: &HashMap<usize, usize>) -> Vec<usize> {
    let mut v: Vec<usize> = members.iter().copied().filter(|&t| !cs.banned[t]).collect();
    v.sort_by(|&a, &b| {
        let ka = cs.results[a].conf_wins as f64 - cs.results[a].conf_losses as f64;
        let kb = cs.results[b].conf_wins as f64 - cs.results[b].conf_losses as f64;
        kb.total_cmp(&ka)
            .then(cs.results[b].win_pct().total_cmp(&cs.results[a].win_pct()))
            .then(rank.get(&a).unwrap_or(&999).cmp(rank.get(&b).unwrap_or(&999)))
    });
    v
}

/// A conference title game's site (2025 sites): most at a neutral dome or
/// stadium, the rest at the higher seed.
fn ccg_site(conf: &str) -> Option<Site> {
    let s = |code: &str, town: &'static str, indoor: bool| {
        Some(Site { state: geo::state_index(code).unwrap_or(0), town, indoor })
    };
    match conf {
        "SEC" => s("GA", "Atlanta", true),
        "Big Ten" => s("IN", "Indianapolis", true),
        "ACC" => s("NC", "Charlotte", false),
        "Big 12" => s("TX", "Arlington", true),
        "Pac-12" => s("NV", "Las Vegas", true),
        "MAC" => s("MI", "Detroit", true),
        _ => None,
    }
}

/// The Sunday after a division's last regular-season Saturday: conference
/// title games are set (FBS, and the SWAC), conference titles without one
/// are decided, and the FCS, D2 and JUCO playoff fields are picked.
pub(crate) fn regular_season_end(w: &mut World, division: Division) {
    let Some(cs) = w.college.as_ref() else { return };
    if !cs.started {
        return;
    }
    let rank: HashMap<usize, usize> =
        ranking(w, division).into_iter().enumerate().map(|(k, (t, _))| (t, k + 1)).collect();
    let mut confs: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for p in w.programs.iter().filter(|p| p.division == division && !p.roster.is_empty() && p.plays(w.year)) {
        if !p.conference.contains("Independent") {
            confs.entry(p.conference.clone()).or_default().push(p.id);
        }
    }
    let season = cs.year;
    let ccg_date = w.dates.conference_championships;
    let mut new_games = Vec::new();
    let mut titles = Vec::new();
    for (name, members) in &confs {
        let order = conference_order(cs, members, &rank);
        let (_, has_ccg) = crate::calendar::rules::conference_rules(name, season);
        let has_ccg = has_ccg && matches!(division, Division::FBS | Division::FCS) && order.len() >= 4;
        if has_ccg {
            let (a, b) = (order[0], order[1]);
            let site = ccg_site(name);
            let mut g = Game::new(ccg_date, a, b, division, GameKind::ConferenceTitle);
            g.site = site;
            g.conference = false;
            let code = w.states[w.programs[a].state].code.clone();
            g.kickoff = schedule::kickoff(&code, key_uniform(w.options.seed as u64 ^ 0xCC6, a as u64), true);
            g.name = None;
            new_games.push(g);
        } else if let Some(&t) = order.first() {
            titles.push(t);
        }
    }
    let cs = w.college.as_mut().unwrap();
    for t in titles {
        cs.results[t].conference_title = true;
    }
    cs.games.extend(new_games);
    if division != Division::FBS {
        pick_division_playoff(w, division);
    }
}

/// The FCS, D2 and JUCO playoff fields: conference champions first (the
/// SWAC and MEAC skip the FCS playoffs for the Celebration Bowl), then the
/// best of the rest.
fn pick_division_playoff(w: &mut World, division: Division) {
    let rules = w.rules.clone();
    let order: Vec<usize> = ranking(w, division).into_iter().map(|x| x.0).collect();
    let cs = w.college.as_ref().unwrap();
    let field = match division {
        Division::FCS => rules.fcs_playoff as usize,
        Division::D2 => rules.d2_playoff as usize,
        Division::JUCO => {
            if cs.year >= 2021 {
                4
            } else {
                2
            }
        }
        // The real fields (from memory, to verify): Division III 16 teams
        // from 1985, 28 from 1999, 32 from 2005; the NAIA 16 from 1987
        // and 20 from 2022.
        Division::D3 => match cs.year {
            ..=1984 => 8,
            1985..=1998 => 16,
            1999..=2004 => 28,
            _ => 32,
        },
        Division::NAIA => match cs.year {
            ..=1986 => 8,
            1987..=2021 => 16,
            _ => 20,
        },
        Division::FBS => return,
    };
    let excluded = |t: usize| {
        cs.banned[t]
            || (division == Division::FCS && matches!(w.programs[t].conference.as_str(), "SWAC" | "MEAC" | "Ivy"))
    };
    let mut seeds: Vec<usize> = Vec::new();
    if division == Division::FCS {
        for &t in &order {
            if cs.results[t].conference_title && !excluded(t) && seeds.len() < field {
                seeds.push(t);
            }
        }
    }
    for &t in &order {
        if seeds.len() >= field {
            break;
        }
        if !excluded(t) && !seeds.contains(&t) {
            seeds.push(t);
        }
    }
    let pos: HashMap<usize, usize> = order.iter().enumerate().map(|(k, &t)| (t, k)).collect();
    seeds.sort_by_key(|t| pos[t]);
    let y = &w.dates;
    let rounds_needed = seeds.len().next_power_of_two().trailing_zeros() as usize;
    let all: Vec<Date> = match division {
        Division::FCS => y.fcs_rounds.clone(),
        // Division III and the NAIA play their rounds on Division II's weekends.
        Division::D2 | Division::NAIA | Division::D3 => y.d2_rounds.clone(),
        _ => vec![y.juco_title.add(-7), y.juco_title],
    };
    let dates: Vec<Vec<Date>> = all[all.len().saturating_sub(rounds_needed)..].iter().map(|&d| vec![d]).collect();
    if seeds.len() < 2 {
        return;
    }
    let neutral_from = dates.len() - 1;
    let text = format!(
        "The {} playoff field is set: {} teams, top seed {}.",
        division.name(),
        seeds.len(),
        w.programs[seeds[0]].real_name
    );
    let date = w.today;
    w.calendar.post(Event::new(date, CalLevel::College, Kind::Date, Loudness::Small, text));
    let b = Bracket::new(division, &seeds, dates, neutral_from);
    w.college.as_mut().unwrap().brackets.push(b);
    advance_brackets(w);
}

/// A title game outside a bracket (the BCS era's No. 1 against No. 2)
/// crowns its winner when it is played; a bracket crowns its own champion
/// as it advances.
fn crown_title_game(cs: &mut CollegeSeason, k: usize) {
    let g = &cs.games[k];
    if g.kind != GameKind::Title || cs.brackets.iter().any(|b| b.pending.contains(&k)) {
        return;
    }
    let Some(home_won) = g.home_won() else { return };
    let (winner, division) = (if home_won { g.home } else { g.away }, g.division);
    cs.champions[division as usize] = Some(winner);
    cs.results[winner].national_title = true;
}

/// Creates each bracket's next round once the last one is played.
pub(crate) fn advance_brackets(w: &mut World) {
    let Some(cs) = w.college.as_mut() else { return };
    let mut new_games: Vec<(usize, Game)> = Vec::new();
    let mut crowned: Vec<(Division, usize)> = Vec::new();
    for (bi, b) in cs.brackets.iter_mut().enumerate() {
        if b.champion.is_some() {
            continue;
        }
        // Wait for the round in play.
        if !b.pending.is_empty() {
            if b.pending.iter().any(|&k| cs.games.get(k).is_none_or(|g| g.played.is_none())) {
                continue;
            }
            let mut next = Vec::with_capacity(b.slots.len() / 2);
            let mut gi = 0;
            for pair in b.slots.chunks(2) {
                match (pair[0], pair[1]) {
                    (Some(_), Some(_)) => {
                        let g = &cs.games[b.pending[gi]];
                        gi += 1;
                        let won = g.home_won().unwrap_or(true);
                        next.push(Some(if won { g.home } else { g.away }));
                    }
                    (Some(a), None) | (None, Some(a)) => next.push(Some(a)),
                    (None, None) => next.push(None),
                }
            }
            b.slots = next;
            b.pending.clear();
            b.round += 1;
        }
        if b.slots.len() == 1 {
            if let Some(c) = b.slots[0] {
                b.champion = Some(c);
                crowned.push((b.division, c));
            }
            continue;
        }
        // Rounds the bracket skipped (all byes) don't use a date.
        let left = b.slots.len().trailing_zeros() as usize;
        let total = b.dates.len();
        let date_round = total.saturating_sub(left);
        let dates = b.dates.get(date_round).cloned().unwrap_or_else(|| vec![w.today.add(7)]);
        let names = b.names.get(date_round).cloned().unwrap_or_default();
        let neutral = date_round >= b.neutral_from;
        let final_round = b.slots.len() == 2;
        let mut gi = 0;
        let chunks: Vec<(Option<usize>, Option<usize>)> = b.slots.chunks(2).map(|c| (c[0], c[1])).collect();
        for (x, y) in chunks {
            let (Some(x), Some(y)) = (x, y) else { continue };
            // The higher seed hosts until the neutral rounds.
            let (home, away) =
                if b.seed.get(&x).unwrap_or(&99) <= b.seed.get(&y).unwrap_or(&99) { (x, y) } else { (y, x) };
            let date = dates[gi % dates.len()];
            let kind = if final_round { GameKind::Title } else { GameKind::Playoff((date_round + 1) as u8) };
            let mut g = Game::new(date, home, away, b.division, kind);
            g.kickoff = if final_round { (20, 0) } else { (12 + 4 * (gi as u16 % 3), 0) };
            if neutral {
                g.name = names.get(gi).copied();
                g.site = Some(neutral_site(g.name, b.division, cs.year));
            }
            new_games.push((bi, g));
            gi += 1;
        }
        let _ = left;
    }
    for (bi, g) in new_games {
        cs.games.push(g);
        let k = cs.games.len() - 1;
        cs.brackets[bi].pending.push(k);
        cs.bowl_teams.insert(cs.games[k].home);
        cs.bowl_teams.insert(cs.games[k].away);
    }
    for (d, c) in crowned {
        cs.champions[d as usize] = Some(c);
        cs.results[c].national_title = true;
    }
}

/// Where a neutral playoff game is played.
fn neutral_site(name: Option<&'static str>, division: Division, season: i32) -> Site {
    if let Some(n) = name
        && let Some(b) = BOWLS.iter().find(|b| b.name == n)
    {
        return Site { state: geo::state_index(b.state).unwrap_or(0), town: b.town, indoor: b.indoor };
    }
    let s = |code: &str, town: &'static str, indoor: bool| Site {
        state: geo::state_index(code).unwrap_or(0),
        town,
        indoor,
    };
    match division {
        // Title game sites rotate; recent and coming ones.
        Division::FBS => match season.rem_euclid(6) {
            0 => s("TX", "Houston", true),
            1 => s("GA", "Atlanta", true),
            2 => s("FL", "Miami Gardens", false),
            3 => s("CA", "Santa Clara", false),
            4 => s("NV", "Las Vegas", true),
            _ => s("LA", "New Orleans", true),
        },
        Division::FCS => {
            if season >= 2026 {
                s("TN", "Nashville", false)
            } else {
                s("TX", "Frisco", false)
            }
        }
        Division::D2 => s("TX", "McKinney", false),
        Division::JUCO => s("TX", "Wichita Falls", false),
        // The Stagg Bowl (from memory, to verify): Phenix City 1985-89,
        // Bradenton 1990-92, Salem 1993-2017 and from 2023, Shenandoah
        // 2018-19, Canton 2021-22.
        Division::D3 => match season {
            ..=1989 => s("AL", "Phenix City", false),
            1990..=1992 => s("FL", "Bradenton", false),
            2018 | 2019 => s("TX", "Shenandoah", false),
            2021 | 2022 => s("OH", "Canton", false),
            _ => s("VA", "Salem", false),
        },
        // The NAIA final (from memory, to verify): Rome, Georgia 2008-13,
        // Daytona Beach from 2014, Savannah, Tennessee before.
        Division::NAIA => match season {
            ..=2007 => s("TN", "Savannah", false),
            2008..=2013 => s("GA", "Rome", false),
            _ => s("FL", "Daytona Beach", false),
        },
    }
}

// --- Selection day (A19, A20) -------------------------------------------------------------

/// The New Year's Six bowls hosting the semifinals in a playoff season (a
/// three-year rotation).
fn semifinal_bowls(season: i32) -> [&'static str; 2] {
    match (season - 2014).rem_euclid(3) {
        0 => ["Rose Bowl", "Sugar Bowl"],
        1 => ["Orange Bowl", "Cotton Bowl"],
        _ => ["Fiesta Bowl", "Peach Bowl"],
    }
}

/// Selection day, the Sunday after the conference title games: the playoff
/// or the BCS pairing by era, every bowl under its tie-ins, and bowl
/// practice starts.
pub(crate) fn selection_day(w: &mut World) {
    let Some(cs) = w.college.as_ref() else { return };
    if !cs.started {
        return;
    }
    let season = cs.year;
    let rules = w.rules.clone();
    let y = w.dates.clone();
    let ranked: Vec<usize> = ranking(w, Division::FBS).into_iter().map(|x| x.0).collect();
    let pos: HashMap<usize, usize> = ranked.iter().enumerate().map(|(k, &t)| (t, k)).collect();
    let cs = w.college.as_ref().unwrap();
    let eligible = |t: usize| !cs.banned[t];
    let mut taken: HashSet<usize> = HashSet::new();
    let mut games: Vec<Game> = Vec::new();
    let mut brackets: Vec<Bracket> = Vec::new();
    let fbs = Division::FBS;
    let bowl_idx = |name: &str| BOWLS.iter().position(|b| b.name == name);
    let jan1 = Date::ymd(season + 1, 1, 1);
    let mut used_bowls: HashSet<&'static str> = HashSet::new();
    let site_of = |name: &str| {
        let b = &BOWLS[bowl_idx(name).unwrap()];
        Site { state: geo::state_index(b.state).unwrap_or(0), town: b.town, indoor: b.indoor }
    };
    let mk_bowl = |name: &'static str, a: usize, b: usize, date: Date| -> Game {
        let (home, away) = if pos.get(&a).unwrap_or(&999) <= pos.get(&b).unwrap_or(&999) { (a, b) } else { (b, a) };
        let mut g = Game::new(date, home, away, fbs, GameKind::Bowl(bowl_idx(name).unwrap()));
        g.name = Some(name);
        g.site = Some(site_of(name));
        g.kickoff = (16, 0);
        g
    };
    // Best eligible team left from a conference ("any" = anyone).
    let pick = |conf: &str, taken: &HashSet<usize>, min_wins: u32| -> Option<usize> {
        ranked.iter().copied().find(|&t| {
            eligible(t)
                && !taken.contains(&t)
                && cs.results[t].wins >= min_wins
                && (conf == "any" || w.programs[t].conference == conf)
        })
    };
    match rules.cfb_postseason {
        CfbPostseason::PollsOnly => {
            // The Bowl Alliance: #1 against #2 unless one is bound to the Rose.
            let rose_bound = |t: usize| matches!(w.programs[t].conference.as_str(), "Big Ten" | "Pac-12");
            if let (Some(&a), Some(&b)) = (ranked.first(), ranked.get(1))
                && !rose_bound(a)
                && !rose_bound(b)
            {
                games.push(mk_bowl("Fiesta Bowl", a, b, Date::ymd(season + 1, 1, 2)));
                taken.insert(a);
                taken.insert(b);
                used_bowls.insert("Fiesta Bowl");
            }
        }
        CfbPostseason::Bcs => {
            // The BCS title game: #1 against #2.
            if let (Some(&a), Some(&b)) = (ranked.first(), ranked.get(1)) {
                let host = if season <= 2005 {
                    Some(["Fiesta Bowl", "Sugar Bowl", "Orange Bowl", "Rose Bowl"][((season - 1998) % 4) as usize])
                } else {
                    None
                };
                let mut g = Game::new(y.title_game, a, b, fbs, GameKind::Title);
                g.site = Some(match host {
                    Some(h) => site_of(h),
                    None => neutral_site(None, fbs, season),
                });
                g.name = Some(host.unwrap_or("BCS National Championship"));
                g.kickoff = (20, 0);
                if let Some(h) = host {
                    used_bowls.insert(h);
                }
                games.push(g);
                taken.insert(a);
                taken.insert(b);
            }
        }
        CfbPostseason::FourTeam | CfbPostseason::TwelveTeam => {
            let n = rules.cfp_teams.max(4) as usize;
            let pool: Vec<usize> = if rules.breakaway {
                ranked
                    .iter()
                    .copied()
                    .filter(|&t| eligible(t) && matches!(w.programs[t].conference.as_str(), "SEC" | "Big Ten"))
                    .collect()
            } else {
                ranked.iter().copied().filter(|&t| eligible(t)).collect()
            };
            let mut field: Vec<usize> = Vec::new();
            if n >= 12 {
                // The five best-ranked conference champions get in.
                let champs: Vec<usize> =
                    pool.iter().copied().filter(|&t| cs.results[t].conference_title).take(5).collect();
                field.extend(&champs);
                for &t in &pool {
                    if field.len() >= n {
                        break;
                    }
                    if !field.contains(&t) {
                        field.push(t);
                    }
                }
                field.sort_by_key(|t| pos[t]);
                // 2024: the four best champions got the byes.
                if season == 2024 {
                    let mut top: Vec<usize> = champs.iter().copied().take(4).collect();
                    top.sort_by_key(|t| pos[t]);
                    let rest: Vec<usize> = field.iter().copied().filter(|t| !top.contains(t)).collect();
                    field = top.into_iter().chain(rest).collect();
                }
            } else {
                field = pool.into_iter().take(n).collect();
            }
            // A bracket needs two teams (tiny test worlds can have fewer).
            if field.len() >= 2 {
                let semis = semifinal_bowls(season);
                let all6 = ["Rose Bowl", "Sugar Bowl", "Orange Bowl", "Cotton Bowl", "Fiesta Bowl", "Peach Bowl"];
                let qf: Vec<&'static str> = all6.iter().copied().filter(|b| !semis.contains(b)).collect();
                let mut dates: Vec<Vec<Date>> = Vec::new();
                let mut names: Vec<Vec<&'static str>> = Vec::new();
                let size = field.len().next_power_of_two();
                let rounds = size.trailing_zeros() as usize;
                for r in 0..rounds {
                    let left = rounds - r;
                    match left {
                        1 => {
                            dates.push(vec![y.title_game]);
                            names.push(vec!["CFP National Championship"]);
                        }
                        2 => {
                            dates.push(vec![y.cfp_semifinals.0, y.cfp_semifinals.1]);
                            names.push(semis.to_vec());
                        }
                        3 => {
                            dates.push(vec![y.cfp_quarterfinals.0, y.cfp_quarterfinals.1]);
                            names.push(qf.clone());
                        }
                        _ => {
                            dates.push(vec![y.cfp_first_round.0, y.cfp_first_round.1]);
                            names.push(Vec::new());
                        }
                    }
                }
                let neutral_from = rounds.saturating_sub(3).min(rounds - 1);
                let mut b = Bracket::new(fbs, &field, dates, if size <= 4 { 0 } else { neutral_from });
                b.names = names;
                for &t in &field {
                    taken.insert(t);
                }
                for n in semis.iter().chain(if size > 4 { qf.iter() } else { [].iter() }) {
                    used_bowls.insert(n);
                }
                brackets.push(b);
            }
        }
    }
    // The other bowls under their tie-ins (best eligible team left from each
    // tie-in conference; teams with five wins fill the last places).
    let bowls: Vec<usize> = (0..BOWLS.len()).filter(|&k| BOWLS[k].first <= season).collect();
    for pass_wins in [6u32, 5] {
        for &k in &bowls {
            let bw = &BOWLS[k];
            if used_bowls.contains(bw.name) {
                continue;
            }
            let a = pick(bw.ties[0], &taken, pass_wins)
                .or_else(|| (bw.ties[0] != "any").then(|| pick("any", &taken, pass_wins)).flatten());
            let Some(a) = a else { continue };
            taken.insert(a);
            let b = pick(bw.ties[1], &taken, pass_wins)
                .filter(|&b| w.programs[b].conference != w.programs[a].conference)
                .or_else(|| {
                    ranked.iter().copied().find(|&t| {
                        eligible(t)
                            && !taken.contains(&t)
                            && cs.results[t].wins >= pass_wins
                            && w.programs[t].conference != w.programs[a].conference
                    })
                });
            let Some(b) = b else {
                taken.remove(&a);
                continue;
            };
            taken.insert(b);
            let date = match bw.day {
                Some(d) => y.bowl_season.0.add(d),
                None => {
                    if bw.name == "Rose Bowl" || bw.name == "Sugar Bowl" {
                        jan1
                    } else {
                        jan1.add(-1)
                    }
                }
            };
            games.push(mk_bowl(bw.name, a, b, date));
            used_bowls.insert(bw.name);
        }
    }
    // The Celebration Bowl: SWAC and MEAC champions (FCS, since 2015).
    if season >= CELEBRATION_BOWL.2 {
        let fcs_champ = |conf: &str| {
            w.programs
                .iter()
                .filter(|p| p.division == Division::FCS && p.conference == conf)
                .map(|p| p.id)
                .find(|&t| cs.results[t].conference_title)
        };
        if let (Some(a), Some(b)) = (fcs_champ("SWAC"), fcs_champ("MEAC")) {
            let mut g = Game::new(y.bowl_season.0, a, b, Division::FCS, GameKind::Celebration);
            g.name = Some(CELEBRATION_BOWL.0);
            g.site = Some(Site { state: geo::state_index("GA").unwrap_or(0), town: CELEBRATION_BOWL.1, indoor: true });
            g.kickoff = (12, 0);
            games.push(g);
        }
    }
    let n_bowls = games.iter().filter(|g| matches!(g.kind, GameKind::Bowl(_))).count();
    let field_text = brackets.first().map_or(String::new(), |b| {
        let mut top: Vec<(usize, usize)> = b.seed.iter().filter(|(_, s)| **s <= 4).map(|(&t, &s)| (s, t)).collect();
        top.sort();
        let teams: Vec<&str> = top.iter().map(|&(_, t)| w.programs[t].real_name.as_str()).collect();
        format!(" The playoff field is set ({} teams; top seeds {}).", b.seed.len(), teams.join(", "))
    });
    // Nothing to announce while the world spins up and rosters fill.
    if n_bowls > 0 || !brackets.is_empty() {
        let text = format!("Selection day: {n_bowls} bowls filled.{field_text}");
        w.calendar.post(Event::new(w.today, CalLevel::College, Kind::Date, Loudness::Big, text));
    }
    let cs = w.college.as_mut().unwrap();
    for g in games {
        cs.bowl_teams.insert(g.home);
        cs.bowl_teams.insert(g.away);
        cs.games.push(g);
    }
    cs.brackets.extend(brackets);
    advance_brackets(w);
    opt_outs(w);
}

/// Whether this season is a player's last in college: the season in play
/// counts (unless January 2 settles otherwise), so four seasons (five under
/// the five-in-five change) or five years since high school end it.
pub(crate) fn last_season(p: &super::Person, season: i32, five_in_five: bool) -> bool {
    let limit = if five_in_five { 5 } else { 4 };
    p.college_seasons + p.cal.count_pending as u32 >= limit || season + 1 - p.class_of >= 5
}

/// Bowl opt-outs (topic 3.2): players with high draft grades skip minor
/// bowls, rarely playoff games, from their draft read, personality and what
/// is at stake. Only players headed to the draft opt out: a senior in his
/// last season, or an underclassman whose plan is to declare, who declares
/// with it. Proposed, not confirmed: the chances below; opt-outs began
/// around 2016 and became common from 2018.
fn opt_outs(w: &mut World) {
    let Some(cs) = w.college.as_ref() else { return };
    let season = cs.year;
    let five = w.changes.rules(season + 1).five_in_five;
    // A yearly ramp (part D 19): a handful before 2016 (Christian
    // McCaffrey and Leonard Fournette made it news that year), then rising
    // every year into the 2020s (proposed, not confirmed: the shares of the
    // full rate below).
    let era = match season {
        ..=2015 => 0.03,
        2016 => 0.15,
        2017 => 0.3,
        2018 => 0.45,
        2019 => 0.6,
        2020 => 0.85,
        2021 => 0.9,
        2022 => 0.95,
        _ => 1.0,
    };
    // Each bowl team's first postseason game, and its day.
    let mut first: HashMap<usize, (GameKind, bool, Date)> = HashMap::new();
    let mut later: Vec<&super::schedule::Game> =
        cs.games.iter().filter(|g| g.played.is_none() && g.kind != GameKind::Regular).collect();
    later.sort_by_key(|g| g.date);
    for g in later {
        let major = g.name.is_some_and(|n| BOWLS.iter().any(|b| b.name == n && b.major));
        for t in [g.home, g.away] {
            first.entry(t).or_insert((g.kind, major, g.date));
        }
    }
    let seed = w.options.seed as u64 ^ ((season as u64) << 20) ^ 0x0F70;
    let mut outs: Vec<(u32, usize, String, bool, Date)> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        if p.stage != Stage::College {
            continue;
        }
        let Some(c) = p.college else { continue };
        let Some(&(kind, major, game_day)) = first.get(&c) else { continue };
        let Some(read) = p.cal.draft_read else { continue };
        let senior = last_season(p, season, five);
        if !senior && p.cal.declare_plan != Some(true) {
            continue;
        }
        let base = match (kind, read.round) {
            (GameKind::Playoff(_) | GameKind::Title, 1) => 0.03,
            (GameKind::Playoff(_) | GameKind::Title, _) => 0.005,
            (_, 1) if major => 0.45,
            (_, 1) => 0.75,
            (_, 2 | 3) if major => 0.15,
            (_, 2 | 3) => 0.35,
            _ => 0.0,
        };
        if base == 0.0 {
            continue;
        }
        let pers = &p.player.personality;
        let lean = 1.0 - 0.05 * (pers.love_of_game - 10.0) - 0.03 * (pers.competitiveness - 10.0)
            + 0.4 * (p.family.decider_wants.money - 0.25);
        let chance = clamp(base * era * lean, 0.0, 0.95);
        if key_uniform(seed, p.player.id as u64) < chance {
            let game = match kind {
                GameKind::Playoff(_) | GameKind::Title => "the playoff",
                _ => "his bowl game",
            };
            let why = if senior { "to prepare for the draft" } else { "to enter the NFL draft" };
            // His own day to say so (part D 16): from the day after the
            // bowl draw to three days before the game, most in the first weeks.
            let span = (w.today.days_to(game_day) - 3).max(0) as f64;
            let u = key_uniform(seed ^ 0xDA7E, p.player.id as u64);
            let day = w.today.add(1 + (span * u.powf(1.5)).floor() as i32);
            outs.push((p.player.id, i, format!("Opted out of {game} at {} {why}.", p.row_age()), senior, day));
        }
    }
    for (id, i, note, senior, date) in outs {
        let c = w.people[i].college.unwrap_or(0);
        let name = w.people[i].name.clone();
        w.people[i].note(note);
        w.people[i].cal.opted_out = true;
        w.people[i].cal.opt_out_on = Some(date);
        w.log.opt_outs += 1;
        let loud =
            if w.people[i].cal.draft_read.is_some_and(|r| r.round == 1) { Loudness::Medium } else { Loudness::Small };
        let why = if senior { "to prepare for the draft" } else { "and will enter the NFL draft" };
        let text = format!("{name} of {} opts out of the postseason {why}.", w.programs[c].real_name);
        w.calendar.post(Event::new(date, CalLevel::College, Kind::News, loud, text).program(c));
        w.college.as_mut().unwrap().out.insert(id, "opted out");
    }
}

/// Bowl practices (A21): the days between selection day and a team's first
/// bowl or playoff game give young players extra practices, up to 15.
/// Applied when that game is played; finals week takes some of them (A22).
pub(crate) fn bowl_practices(w: &mut World, date: Date) {
    let Some(cs) = w.college.as_ref() else { return };
    let mut teams: Vec<(usize, u32)> = Vec::new();
    let sel = w.dates.selection_day;
    for g in cs.games.iter().filter(|g| g.date == date && g.kind != GameKind::Regular) {
        for t in [g.home, g.away] {
            // Only a team's first postseason game.
            let earlier = cs.games.iter().any(|h| {
                h.involves(t) && h.kind != GameKind::Regular && h.kind != GameKind::ConferenceTitle && h.date < date
            });
            if earlier || w.programs[t].division != Division::FBS {
                continue;
            }
            let days = sel.days_to(date).max(0) as u32;
            // About five practices a week, finals week costing two.
            let practices = (days * 5 / 7).saturating_sub(2).min(15);
            teams.push((t, practices));
        }
    }
    if teams.is_empty() {
        return;
    }
    let coaching: HashMap<usize, f64> = teams
        .iter()
        .map(|&(t, _)| {
            let prog = &w.programs[t];
            let hc = prog.head_coach.map_or(9.0, |c| w.coaches[c].ability().head_coach_value());
            (t, hc)
        })
        .collect();
    let practice: HashMap<usize, u32> = teams.into_iter().collect();
    let gains: f64 = w
        .people
        .par_iter_mut()
        .filter(|p| p.stage == Stage::College && p.college.is_some_and(|c| practice.contains_key(&c)))
        .map(|p| {
            let c = p.college.unwrap();
            crate::development::extra_practices(&mut p.player, practice[&c], coaching[&c], &mut p.rng)
        })
        .sum();
    w.log.bowl_practice_gain += gains;
}

// --- Awards and honors (A25) ---------------------------------------------------------------

/// A season line's production as voters read it: a passer's yards and
/// touchdowns accounted for, less interceptions; a runner's or receiver's
/// yards from scrimmage and touchdowns; a defender's tackles, sacks, stops
/// and interceptions.
fn production(l: &Line, pos: crate::positions::Position) -> f64 {
    use crate::positions::Position::*;
    let scrimmage = (l.rush_yds + l.rec_yds) / 10.0 + 6.0 * (l.rush_td + l.rec_td) as f64;
    let passing = l.pass_yds / 20.0 + 4.0 * l.pass_td as f64 - 3.0 * l.int_thrown as f64;
    let defense = 1.0 * l.tackles as f64 + 6.0 * l.sacks + 4.0 * l.tfl + 10.0 * l.ints as f64;
    match pos {
        QB | RB | WR | TE => passing + scrimmage,
        _ => 0.6 * defense,
    }
}

/// The Heisman voters' kinds of candidate, with how they weigh each
/// (placeholder voters, Claude's pick, proposed, not confirmed): the trophy
/// goes to a quarterback about three times in four, to a back or a receiver
/// now and then and almost never to a defender (2000-2024, from memory, to
/// verify). Kickers and snappers are not candidates.
fn heisman_kind(pos: crate::positions::Position) -> Option<(usize, f64)> {
    use crate::positions::Position::*;
    match pos {
        QB => Some((0, 1.0)),
        RB => Some((1, 0.84)),
        WR | TE => Some((2, 0.78)),
        K | P | LS => None,
        _ => Some((3, 0.75)),
    }
}

/// What the position award voters count in a season line (A25): a passer's,
/// runner's or receiver's production as the Heisman voters read it; a
/// kicker's field goals, long makes worth more and short misses costing
/// more; a defender's tackles, sacks, stops behind the line, interceptions
/// and breakups (a defensive back's tackles count little). Blockers have no
/// box-score line. Claude's weights, proposed, not confirmed.
pub(crate) fn award_stats(l: &Line, pos: crate::positions::Position) -> f64 {
    use crate::positions::Position::*;
    let (tackles, sacks, tfl, ints, pbus) = (l.tackles as f64, l.sacks, l.tfl, l.ints as f64, l.pbus as f64);
    match pos {
        QB | RB | WR | TE => production(l, pos),
        K | P | LS => {
            const MADE: [f64; 4] = [1.0, 1.0, 1.25, 1.75];
            const MISSED: [f64; 4] = [2.0, 1.5, 1.0, 0.5];
            (0..4)
                .map(|b| {
                    let made = l.fg_made_band[b].min(l.fg_att_band[b]) as f64;
                    MADE[b] * made - MISSED[b] * (l.fg_att_band[b] as f64 - made)
                })
                .sum()
        }
        OT | IOL => 0.0,
        LB => tackles + 6.0 * sacks + 4.0 * tfl + 10.0 * ints + 3.0 * pbus,
        CB | S => 0.5 * tackles + 6.0 * sacks + 4.0 * tfl + 10.0 * ints + 4.0 * pbus,
        _ => tackles + 6.0 * sacks + 4.0 * tfl + 10.0 * ints + 3.0 * pbus,
    }
}

/// How much of a position award rests on film (the season's per-play
/// grades) rather than stats: none for passers, runners, receivers and
/// kickers, all of it for blockers, half for defenders, whose stats tell
/// part of the story. Claude's pick, proposed, not confirmed.
pub(crate) fn award_film_weight(pos: crate::positions::Position) -> f64 {
    use crate::positions::Position::*;
    match pos {
        QB | RB | WR | TE | K | P | LS => 0.0,
        OT | IOL => 1.0,
        _ => 0.5,
    }
}

/// A position award's winner (A25), judged like the Heisman: the season's
/// stats and film against the mean of the season's ten best at his position,
/// over a full season (six games or more) in FBS for the team he is still
/// on. His team's ranking and a former winner's higher bar count as for the
/// Heisman: real winners nearly all play for ranked teams, a defender's big
/// numbers on a weak team do not win it, and back-to-back winners are rare
/// (the Biletnikoff: Michael Crabtree 2007-08, Justin Blackmon 2010-11; from
/// memory, to verify). Claude's weights, proposed, not confirmed.
fn show_pick(
    w: &World,
    cs: &CollegeSeason,
    rank: &HashMap<usize, usize>,
    by_id: &HashMap<u32, usize>,
    award: &str,
    positions: &[crate::positions::Position],
) -> Option<usize> {
    let candidates: Vec<(u32, usize, usize, f64, f64, usize)> = cs
        .lines
        .iter()
        .filter(|(_, l)| w.programs[l.program].division == Division::FBS && l.games >= 6)
        .filter_map(|(&id, l)| {
            let i = *by_id.get(&id)?;
            // Still on that team: a player gone mid-season is no candidate.
            let p = &w.people[i];
            if p.stage != Stage::College || p.college != Some(l.program) {
                return None;
            }
            let pos = p.player.position;
            let k = positions.iter().position(|&q| q == pos)?;
            Some((id, i, k, award_stats(l, pos), l.grade_sum, l.program))
        })
        .collect();
    // Each position's bar: the mean of its ten best, by stats and by film.
    let top_ten = |mut v: Vec<f64>| {
        v.sort_by(|a, b| b.total_cmp(a));
        let top = &v[..v.len().min(10)];
        (top.iter().sum::<f64>() / top.len().max(1) as f64).max(1.0)
    };
    let bars: Vec<(f64, f64)> = (0..positions.len())
        .map(|k| {
            let stats = candidates.iter().filter(|c| c.2 == k).map(|c| c.3).collect();
            let film = candidates.iter().filter(|c| c.2 == k).map(|c| c.4).collect();
            (top_ten(stats), top_ten(film))
        })
        .collect();
    let won = format!("Won the {award}");
    candidates
        .iter()
        .map(|&(id, i, k, stats, film, prog)| {
            let fw = award_film_weight(positions[k]);
            let read = (1.0 - fw) * stats / bars[k].0 + fw * film / bars[k].1;
            let team = rank.get(&prog).map_or(0.8, |&r| 1.3 - 0.02 * r.min(25) as f64);
            let former = w.people[i].notes.iter().any(|n| n.starts_with(&won));
            (read * team * if former { 0.75 } else { 1.0 }, id, i)
        })
        .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
        .map(|(_, _, i)| i)
}

/// The Heisman finalists, the ceremony, the awards show and All-America
/// teams on their days (A25). Placeholder picks: season stats and team
/// success (proposed, not confirmed).
pub(crate) fn awards(w: &mut World, which: &'static str) {
    let Some(cs) = w.college.as_ref() else { return };
    if !cs.started {
        return;
    }
    let rank: HashMap<usize, usize> =
        ranking(w, Division::FBS).into_iter().enumerate().map(|(k, (t, _))| (t, k + 1)).collect();
    let by_id: HashMap<u32, usize> = w.people.iter().enumerate().map(|(i, p)| (p.player.id, i)).collect();
    // The Heisman race: each candidate's production against the mean of the
    // season's ten best of his kind, weighted by how voters treat his kind
    // and by his team's ranking. A full season (six games or more) makes a
    // candidate, and a former winner is held to a higher bar (part B 15:
    // 0.7): nobody has won twice since Archie Griffin (1974-75). Claude's
    // picks, proposed, not confirmed.
    let candidates: Vec<(u32, usize, usize, f64, f64, usize)> = cs
        .lines
        .iter()
        .filter(|(_, l)| w.programs[l.program].division == Division::FBS && l.games >= 6)
        .filter_map(|(&id, l)| {
            let i = *by_id.get(&id)?;
            let pos = w.people[i].player.position;
            let (kind, weight) = heisman_kind(pos)?;
            Some((id, i, kind, weight, production(l, pos), l.program))
        })
        .collect();
    let mut best: [Vec<f64>; 4] = Default::default();
    for c in &candidates {
        best[c.2].push(c.4);
    }
    let bar: Vec<f64> = best
        .iter_mut()
        .map(|v| {
            v.sort_by(|a, b| b.total_cmp(a));
            let top = &v[..v.len().min(10)];
            (top.iter().sum::<f64>() / top.len().max(1) as f64).max(1.0)
        })
        .collect();
    let mut scored: Vec<(f64, u32, usize)> = candidates
        .iter()
        .map(|&(id, i, kind, weight, prod, prog)| {
            let team = rank.get(&prog).map_or(0.8, |&r| 1.3 - 0.02 * r.min(25) as f64);
            let former = w.people[i].notes.iter().any(|n| n.starts_with("Won the Heisman Trophy"));
            let s = weight * prod / bar[kind] * team * if former { 0.7 } else { 1.0 };
            (s, id, i)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    // Teammates split the vote (part D 24): each teammate below a school's
    // best candidate loses a fifth more (proposed, not confirmed).
    let mut seen: HashMap<usize, i32> = HashMap::new();
    for e in scored.iter_mut() {
        let prog = w.people[e.2].college.unwrap_or(usize::MAX);
        let k = seen.entry(prog).or_insert(0);
        e.0 *= 0.8f64.powi(*k);
        *k += 1;
    }
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let date = w.today;
    match which {
        "finalists" => {
            let names: Vec<String> = scored
                .iter()
                .take(4)
                .map(|&(_, _, i)| {
                    let p = &w.people[i];
                    format!("{} ({}, {})", p.name, p.player.position, w.program_name(p.college.unwrap_or(0)))
                })
                .collect();
            for &(_, _, i) in scored.iter().take(4) {
                let age = w.people[i].row_age();
                w.people[i].note(format!("A Heisman Trophy finalist at {age}."));
            }
            if !names.is_empty() {
                let text = format!("Heisman finalists: {}.", names.join(", "));
                w.calendar.post(Event::new(date, CalLevel::College, Kind::Award, Loudness::Medium, text));
            }
        }
        "heisman" => {
            if let Some(&(_, id, i)) = scored.first() {
                let c = w.people[i].college.unwrap_or(0);
                let text = format!("{} of {} wins the Heisman Trophy.", w.people[i].name, w.program_name(c));
                let age = w.people[i].row_age();
                w.people[i].note(format!("Won the Heisman Trophy at {age}."));
                w.calendar
                    .post(Event::new(date, CalLevel::College, Kind::Award, Loudness::Big, text).program(c).memorable());
                w.college.as_mut().unwrap().awards.push(("Heisman Trophy", id, c));
            }
        }
        "show" => {
            use crate::positions::Position::*;
            let list: [(&'static str, &[crate::positions::Position]); 8] = [
                ("Davey O'Brien Award", &[QB]),
                ("Doak Walker Award", &[RB]),
                ("Biletnikoff Award", &[WR]),
                ("John Mackey Award", &[TE]),
                ("Outland Trophy", &[OT, IOL, DT]),
                ("Butkus Award", &[LB]),
                ("Jim Thorpe Award", &[CB, S]),
                ("Lou Groza Award", &[K]),
            ];
            let mut picks = Vec::new();
            for (award, positions) in list {
                let best = if cs.on_engine {
                    show_pick(w, cs, &rank, &by_id, award, positions)
                } else {
                    // The stand-in keeps no stats: the season's play from
                    // the development step.
                    w.people
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| {
                            p.stage == Stage::College
                                && positions.contains(&p.player.position)
                                && p.last_role == Some(PlayingTime::Starter)
                                && p.college.is_some_and(|c| w.programs[c].division == Division::FBS)
                        })
                        .max_by(|a, b| {
                            a.1.last_performance.unwrap_or(0.0).total_cmp(&b.1.last_performance.unwrap_or(0.0))
                        })
                        .map(|(i, _)| i)
                };
                if let Some(i) = best {
                    picks.push((award, i));
                }
            }
            let mut lines = Vec::new();
            for (award, i) in picks {
                let c = w.people[i].college.unwrap_or(0);
                let id = w.people[i].player.id;
                let age = w.people[i].row_age();
                w.people[i].note(format!("Won the {award} at {age}."));
                lines.push(format!("{} {} ({})", award, w.people[i].name, w.program_name(c)));
                w.college.as_mut().unwrap().awards.push((award, id, c));
            }
            if !lines.is_empty() {
                let text = format!("College football awards: {}.", lines.join("; "));
                w.calendar.post(Event::new(date, CalLevel::College, Kind::Award, Loudness::Medium, text));
            }
        }
        _ => {}
    }
}

// --- The season's end ----------------------------------------------------------------------

/// The regular season's book at the Sunday after rivalry week (section 4:
/// coaches are judged and fired before the title games). Pushes each
/// program's season (updated as its postseason ends) and moves reputations.
pub(crate) fn record_regular_season(w: &mut World) {
    let Some(cs) = w.college.as_ref() else { return };
    if !cs.started || !w.recording {
        return;
    }
    let year = cs.year;
    // The athletic directors read results against the engine's own win
    // curve (trial 4 topic 4): expected wins are worked out again with the
    // spread this season's games show.
    refit_expectations(w);
    let cs = w.college.as_ref().unwrap();
    let results: Vec<TeamResult> = cs.results.clone();
    let credited = cs.credited.clone();
    for p in 0..w.programs.len() {
        let t = &results[p];
        if t.games() == 0 {
            continue;
        }
        let prog = &mut w.programs[p];
        // The coach on the sideline: an interim after an in-season firing.
        let head = prog.sideline.or(prog.head_coach);
        prog.seasons.push(super::program::ProgramSeason {
            year,
            wins: t.wins,
            losses: t.losses,
            conference_title: t.conference_title,
            national_title: false,
            coach: head,
            strength: t.strength,
            totals: t.totals,
        });
        let locker = clamp(t.incidents as f64 / t.roster.max(1) as f64 * 8.0, 0.0, 1.0);
        let staff = staff_of(prog, head);
        let (cw, cl) = credited[p];
        for (c, is_head) in staff {
            if is_head {
                super::staff::credit_head(w, c, p, t.wins.saturating_sub(cw), t.losses.saturating_sub(cl), 0);
            }
            w.coaches[c].season_reputation(t.vs_expectation(), locker, is_head);
        }
        w.college.as_mut().unwrap().credited[p] = (t.wins, t.losses);
    }
}

/// A program's staff for credit, the coach on the sideline first (as head
/// coach), each coach once.
fn staff_of(prog: &super::program::Program, head: Option<usize>) -> Vec<(usize, bool)> {
    let mut staff: Vec<(usize, bool)> = head.map(|h| (h, true)).into_iter().collect();
    for c in [prog.oc, prog.dc].into_iter().chain(prog.position_coaches.iter().copied()).flatten() {
        if !staff.iter().any(|s| s.0 == c) {
            staff.push((c, false));
        }
    }
    staff
}

/// Fits this season's win curve to the games played (one spread for every
/// level, home field as before) and works each team's expected wins out
/// again with it, so a roster's expectation matches what the engine does.
/// The fitted spread carries into next season's running expectation.
fn refit_expectations(w: &mut World) {
    let Some(cs) = w.college.as_ref() else { return };
    let games: Vec<(usize, usize, f64, bool)> = cs
        .games
        .iter()
        .filter(|g| g.kind == GameKind::Regular)
        .filter_map(|g| {
            g.played.as_ref().map(|pl| (g.home, g.away, if g.site.is_some() { 0.0 } else { 0.8 }, pl.home_won))
        })
        .collect();
    if games.len() < 200 {
        return;
    }
    let pre: Vec<f64> = cs.results.iter().map(|r| r.preseason).collect();
    let loglik = |spread: f64| -> f64 {
        games
            .iter()
            .map(|&(h, a, adv, won)| {
                let p = win_chance(pre[h], pre[a], spread, adv).clamp(1e-6, 1.0 - 1e-6);
                if won { p.ln() } else { (1.0 - p).ln() }
            })
            .sum()
    };
    // Golden-section search on 1-20 rating points.
    let (mut lo, mut hi) = (1.0f64, 20.0f64);
    let g = 0.618_033_988_75;
    for _ in 0..40 {
        let a = hi - g * (hi - lo);
        let b = lo + g * (hi - lo);
        if loglik(a) > loglik(b) {
            hi = b;
        } else {
            lo = a;
        }
    }
    let spread = 0.5 * (lo + hi);
    let mut expected = vec![0.0; pre.len()];
    for &(h, a, adv, _) in &games {
        let pe = win_chance(pre[h], pre[a], spread, adv);
        expected[h] += pe;
        expected[a] += 1.0 - pe;
    }
    w.college_spread = spread;
    let cs = w.college.as_mut().unwrap();
    for (r, e) in cs.results.iter_mut().zip(expected) {
        r.expected = e;
    }
}

/// The season ends with the last title game: champions, the postseason's
/// wins and losses, titles and their reputation, the final poll.
pub(crate) fn finish(w: &mut World) {
    let Some(cs) = w.college.as_ref() else { return };
    if cs.finished || !cs.started {
        return;
    }
    // In the polls-only era the final poll picks the champion.
    if w.rules.cfb_postseason == CfbPostseason::PollsOnly {
        let top = ranking(w, Division::FBS).first().map(|x| x.0);
        if let Some(t) = top {
            let cs = w.college.as_mut().unwrap();
            cs.results[t].national_title = true;
            cs.champions[0] = Some(t);
        }
    }
    let date = w.today;
    release_poll(w, date, PollKind::Final);
    let year = w.college.as_ref().unwrap().year;
    update_records(w, year);
    let cs = w.college.as_ref().unwrap();
    let results = cs.results.clone();
    let credited = cs.credited.clone();
    let champions = cs.champions;
    if w.recording {
        for p in 0..w.programs.len() {
            let t = &results[p];
            if t.games() == 0 {
                continue;
            }
            let (cw, cl) = credited[p];
            let prog = &mut w.programs[p];
            if let Some(s) = prog.seasons.last_mut().filter(|s| s.year == year) {
                s.wins = t.wins;
                s.losses = t.losses;
                s.conference_title = t.conference_title;
                s.national_title = t.national_title;
                s.strength = t.strength;
                s.totals = t.totals;
            }
            if t.conference_title {
                prog.conference_titles += 1;
            }
            if t.national_title {
                prog.titles += 1;
            }
            let stage = prog.prestige_rel;
            // The coach on the sideline gets the postseason (part B 45): a
            // retiring coach who stayed, or the interim.
            let head = prog.sideline.or(prog.head_coach);
            let staff = staff_of(prog, head);
            for (c, is_head) in staff {
                if is_head {
                    let title = (t.best_title() != TitleKind::None) as u32;
                    super::staff::credit_head(w, c, p, t.wins.saturating_sub(cw), t.losses.saturating_sub(cl), title);
                }
                w.coaches[c].title_reputation(t.best_title(), is_head, stage);
            }
        }
        for prog in w.programs.iter_mut() {
            prog.sideline = None;
        }
        for (d, champ) in champions.iter().enumerate() {
            let Some(c) = *champ else { continue };
            let division = Division::ALL[d];
            let name = w.programs[c].name(w.options.names).to_string();
            w.news.push(NewsItem {
                date: None,
                year,
                kind: NewsKind::NationalTitle,
                channel: Channel::News,
                program: Some(c),
                text: format!("{name} wins the {} national title.", division.name()),
            });
        }
    }
    if let Some(c) = champions[0] {
        let text = format!("{} wins the national title.", w.programs[c].real_name);
        w.calendar.post(Event::new(date, CalLevel::College, Kind::Game, Loudness::Huge, text).program(c));
    }
    // Snaps become game experience.
    let on_engine = w.college.as_ref().unwrap().on_engine;
    let snaps = std::mem::take(&mut w.college.as_mut().unwrap().snaps);
    for p in w.people.iter_mut() {
        if !matches!(p.stage, Stage::College | Stage::Declared) {
            continue;
        }
        let n = if on_engine {
            snaps.get(&p.player.id).map_or(0, |s| s.0)
        } else {
            match p.last_role {
                Some(PlayingTime::Starter) => season::SEASON_SNAPS,
                Some(PlayingTime::Rotation) => season::SEASON_SNAPS * 2 / 5,
                Some(PlayingTime::Backup) => season::SEASON_SNAPS / 12,
                _ => 0,
            }
        };
        p.last_snaps = n;
        p.college_snaps += n;
    }
    w.college.as_mut().unwrap().snaps = snaps;
    if w.options.keep_game_log && w.recording {
        for p in w.people.iter().filter(|p| matches!(p.stage, Stage::College | Stage::Declared)) {
            if let Some(s) = w.game_log.players.get_mut(&(year, p.player.id)) {
                s.pos = Some(p.player.position);
            }
            w.game_log
                .names
                .entry(p.player.id)
                .or_insert_with(|| (p.name.clone(), p.player.position, p.stars, p.class_of));
        }
    }
    // The stakes test's count: huge games per FBS team (topic 8: most
    // seasons 1 to 3, some none).
    let mut huge: HashMap<usize, u32> = HashMap::new();
    for g in
        w.college.as_ref().unwrap().games.iter().filter(|g| g.division == Division::FBS && g.kind == GameKind::Regular)
    {
        for (side, t) in [g.home, g.away].into_iter().enumerate() {
            if g.huge_for[side] || g.loud == Loudness::Huge {
                *huge.entry(t).or_default() += 1;
            }
        }
    }
    let mut v: Vec<(usize, u32)> = huge.into_iter().collect();
    v.sort();
    w.log.huge_by_team = v;
    w.college.as_mut().unwrap().finished = true;
}

/// Whether this season counts against a player's eligibility, settled when
/// the portal opens (January 2): a redshirt can play four games and keep the
/// year from 2018 on; a redshirt trick keeps it by stopping at four. Before
/// 2018 any game a redshirt played cost him the year. A season without a
/// game is never a season of competition: a player who never got in uses
/// his redshirt year if he still has it, in every era.
pub(crate) fn settle_eligibility(w: &mut World) {
    let Some(cs) = w.college.as_ref() else { return };
    let four = w.rules.four_game_redshirt;
    let on_engine = cs.on_engine;
    let games: HashMap<u32, u32> = cs.snaps.iter().map(|(&id, s)| (id, s.1)).collect();
    let team_games: Vec<u32> = cs.results.iter().map(|r| r.games()).collect();
    // The team he played his games for: a December transfer has moved on by now.
    let prog_of: HashMap<u32, usize> = cs.lines.iter().map(|(&id, l)| (id, l.program)).collect();
    let seed = w.options.seed as u64 ^ ((cs.year as u64) << 24) ^ 0x5E77;
    let mut kept = 0u32;
    for p in w.people.iter_mut() {
        if !matches!(p.stage, Stage::College | Stage::Declared) {
            continue;
        }
        let played = games.get(&p.player.id).copied().unwrap_or(0);
        // A redshirt from fall camp who got into games: past four (any game
        // before 2018) the season counts after all.
        if !p.cal.count_pending {
            let camp_redshirt = p.last_role == Some(PlayingTime::Redshirt) && !p.cal.academic_sit && p.redshirt_used;
            if !camp_redshirt || played == 0 {
                continue;
            }
            let age = p.row_age();
            let games = if played == 1 { "game" } else { "games" };
            let burned = !four || played > 4;
            if burned {
                p.redshirt_used = false;
                p.college_seasons += 1;
                p.note(format!("Played in {played} {games} at {age}, which cost him his redshirt year."));
            } else {
                kept += 1;
                p.note(format!(
                    "Played in {played} {games} at {age} and kept his redshirt year under the four-game rule."
                ));
            }
            let team = prog_of
                .get(&p.player.id)
                .copied()
                .or(p.college)
                .and_then(|c| team_games.get(c).copied())
                .unwrap_or(played);
            played_as_redshirt(p, played, team, burned, seed);
            continue;
        }
        p.cal.count_pending = false;
        if on_engine && !p.redshirt_used && (played == 0 || four && played <= 4) {
            p.redshirt_used = true;
            if four {
                kept += 1;
            }
            if p.cal.redshirt_trick {
                let age = p.row_age();
                p.note(format!("Stopped playing after four games at {age} to keep his redshirt year."));
            }
        } else {
            p.college_seasons += 1;
        }
    }
    w.log.four_game_redshirts += kept;
}

/// A camp redshirt who got into games (F1): his season row counts them.
/// His growth stays the year his staff planned at camp (practice and the
/// weight room); the row shows his play over the games he got, the form of
/// his year and the games an injury cost him, as the yearly step reads a
/// backup's season. When the year counts against his eligibility it was a
/// backup's season, and his row and role say so. A redshirt kept under the
/// four-game rule (2018 on, four games or fewer) gets the same play, form
/// and games missed, but his row and role stay a redshirt's: a year of four
/// games or fewer is a redshirt year in real football, and the role feeds
/// the staff's roster read and the portal's role records (proposed, not
/// confirmed: F1's text said the row says backup).
fn played_as_redshirt(p: &mut super::Person, played: u32, team_games: u32, burned: bool, seed: u64) {
    use crate::player::DevelopmentEvent as Ev;
    let id = p.player.id as u64;
    let Some(h) = p.player.history.last_mut() else { return };
    // The rating he played the season at: an early enrollee's row opens
    // with his first college spring step (F2), taken before the season.
    let camp = h.overall_start + h.spring_open.unwrap_or(0.0);
    // His play, as the yearly step reads a backup's season (development.rs):
    // his rating through the year, a first-year dip and the year's form. A
    // lost season (drawn at camp) costs 3-7 points of play and no other
    // form follows it; otherwise a slump costs 4-9 (6%) and a hot year adds
    // 3-7 (a backup's breakout chance, 2.1%; not in a year a major injury
    // mostly took). Each is named on the row, so a page gives the reason.
    let dip = h.adjustment.unwrap_or(0.0);
    let mut play = 2.0 * key_normal(seed, id) - 7.0 * dip;
    let form = key_uniform(seed ^ 0xF0, id);
    let size = key_uniform(seed ^ 0xF1, id);
    if h.events.has(Ev::LostSeason) {
        play -= 3.0 + 4.0 * size;
    } else if form < 0.06 {
        play -= 4.0 + 5.0 * size;
        h.events.add(Ev::Slump);
    } else if form > 1.0 - 0.035 * 0.6 && !h.events.has(Ev::MajorInjury) {
        play += 3.0 + 4.0 * size;
        // A big year from a player whose rating still fell is not a breakout.
        if h.overall >= camp {
            h.events.add(Ev::Breakout);
        }
    }
    if burned {
        h.playing_time = PlayingTime::Backup;
        p.last_role = Some(PlayingTime::Backup);
    }
    let perf = clamp((camp + h.overall) / 2.0 + play, crate::attributes::MIN_RATING, crate::attributes::MAX_RATING);
    h.performance = Some(perf);
    p.last_performance = Some(perf);
    // An injury this year cost him games he could have played (the yearly
    // step's ranges), at most the games his team played without him.
    if h.games_missed == 0 {
        let u = key_uniform(seed ^ 0x1A, id);
        let games = crate::development::season_games(h.level) as f64;
        let missed = if h.events.has(Ev::MajorInjury) {
            let least = (games * 0.5).round();
            least + (games - least + 1.0) * u
        } else if h.events.has(Ev::ModerateInjury) {
            2.0 + 5.0 * u
        } else if h.events.has(Ev::MinorInjury) {
            3.0 * u
        } else {
            0.0
        };
        h.games_missed = (missed.floor() as u32).min(team_games.saturating_sub(played));
    }
}

/// The lean to leave at which a benched veteran takes the redshirt trick: a
/// backup clearly better than his division's typical starter at his
/// position, with some competitive edge (a fall camp lean of 0.18 times 1.7
/// or more; Claude's pick, proposed, not confirmed). Real: Kelly Bryant,
/// Clemson 2018, and a few dozen a year since (from memory, to verify).
const REDSHIRT_TRICK_LEAN: f64 = 0.3;

/// The redshirt trick (topic 3): a player who knows he is leaving stops
/// playing after four games to keep the year (2018 on). Checked the Sunday
/// after the fourth week of games.
pub(crate) fn redshirt_trick(w: &mut World) {
    let Some(cs) = w.college.as_ref() else { return };
    if !w.rules.four_game_redshirt || !cs.on_engine {
        return;
    }
    let mut stop = Vec::new();
    for p in w.people.iter() {
        if p.stage != Stage::College || p.redshirt_used || !p.cal.count_pending {
            continue;
        }
        // Benched veterans with a year to save and a real lean to leave.
        if p.college_seasons >= 2
            && p.cal.lean > REDSHIRT_TRICK_LEAN
            && matches!(p.last_role, Some(PlayingTime::Backup | PlayingTime::Rotation))
            && cs.snaps.get(&p.player.id).is_some_and(|s| s.1 <= 4)
        {
            stop.push(p.player.id);
        }
    }
    let ids: HashSet<u32> = stop.iter().copied().collect();
    let date = w.today;
    for p in w.people.iter_mut().filter(|p| ids.contains(&p.player.id)) {
        p.cal.redshirt_trick = true;
        // He sits to leave: he enters the portal when it opens.
        let to_sure = (1.0 / p.cal.lean.max(0.01)).max(1.0);
        lean_push(p, date, "sat out the season to transfer", to_sure);
    }
    w.log.redshirt_tricks += stop.len() as u32;
    let cs = w.college.as_mut().unwrap();
    for id in stop {
        cs.out.insert(id, "redshirt trick");
    }
}

/// Whether a team still has a game to play: one on the schedule, or a place
/// in an undecided bracket (a bye, or a round not yet drawn).
pub(crate) fn still_playing(cs: &CollegeSeason, t: usize) -> bool {
    cs.games.iter().any(|g| g.played.is_none() && g.involves(t))
        || cs.brackets.iter().any(|b| b.champion.is_none() && b.slots.contains(&Some(t)))
}

/// Whether an FBS team is still alive for the national title: a playoff or
/// title game to play, or a place in the undecided playoff bracket.
pub(crate) fn playoff_alive(cs: &CollegeSeason, t: usize) -> bool {
    cs.games.iter().any(|g| {
        g.played.is_none()
            && g.involves(t)
            && g.division == Division::FBS
            && matches!(g.kind, GameKind::Playoff(_) | GameKind::Title)
    }) || cs.brackets.iter().any(|b| b.division == Division::FBS && b.champion.is_none() && b.slots.contains(&Some(t)))
}

/// Whether a team may still play for the title (part D 23): alive in the
/// playoff, or, before the field is picked, with a conference title game
/// still to play or ranked close enough to the field to make it.
pub(crate) fn still_contending(cs: &CollegeSeason, t: usize, field: usize) -> bool {
    if playoff_alive(cs, t) {
        return true;
    }
    if field == 0 || cs.brackets.iter().any(|b| b.division == Division::FBS) {
        return false;
    }
    let title_game =
        cs.games.iter().any(|g| g.played.is_none() && g.involves(t) && g.kind == GameKind::ConferenceTitle);
    let ranked = cs.polls.last().is_some_and(|p| p.top.iter().take(field + 6).any(|&x| x == t));
    title_game || ranked
}

/// Whether every college game scheduled so far has been played and every
/// bracket is decided.
pub(crate) fn all_played(w: &World) -> bool {
    let Some(cs) = w.college.as_ref() else { return true };
    cs.games.iter().all(|g| g.played.is_some()) && cs.brackets.iter().all(|b| b.champion.is_some())
}

// --- The all-season lean (T3) ------------------------------------------------------------

/// The days the lean toward the portal is read and moved (T3: it runs from
/// what happens, not once a year).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LeanCheck {
    /// Fall camp: his place on the depth chart, how far he is from home.
    FallCamp,
    /// Midseason: the snaps he has had.
    MidSeason,
    /// The Sunday after rivalry week: a coaching change at his school.
    CoachingChanges,
    /// Selection day: a bowl (and its practices) or none.
    Bowls,
    /// Early signing day: a blue-chip signed at his position.
    EarlySigning,
}

fn lean_push(p: &mut super::Person, date: Date, why: &'static str, factor: f64) {
    p.cal.lean *= factor;
    p.cal.lean_events.push((date, why, factor));
}

/// Reads and moves each college player's lean toward leaving (T3). The base
/// is the yearly entry chance by role (`PIPELINE.portal_entry`), how buried
/// he is and his competitiveness; dated events move it up or down from there.
/// Proposed, not confirmed: each push below.
pub(crate) fn lean_checkpoint(w: &mut World, check: LeanCheck) {
    use crate::career::PIPELINE;
    let date = w.today;
    let year = w.year;
    let seed = w.options.seed as u64 ^ ((year as u64) << 24) ^ 0x1EA4;
    match check {
        LeanCheck::FallCamp => {
            // Starters' level at each position and division: the bar a backup measures himself against.
            let mut perf: [[Vec<f64>; crate::positions::POSITION_COUNT]; DIVISION_COUNT] = Default::default();
            for p in w.people.iter() {
                if p.stage == Stage::College
                    && p.last_role == Some(PlayingTime::Starter)
                    && let Some(c) = p.college
                {
                    perf[w.programs[c].division as usize][p.player.position.index()].push(p.player.overall());
                }
            }
            let med: Vec<Vec<f64>> = perf
                .iter_mut()
                .map(|d| {
                    d.iter_mut()
                        .map(|v| {
                            v.sort_by(|a, b| a.total_cmp(b));
                            if v.is_empty() { 99.0 } else { v[v.len() / 2] }
                        })
                        .collect()
                })
                .collect();
            let programs = &w.programs;
            w.people.par_iter_mut().for_each(|p| {
                if p.stage != Stage::College {
                    return;
                }
                let Some(c) = p.college else { return };
                let div = programs[c].division;
                if div == Division::JUCO {
                    return;
                }
                // A starter has a small lean of his own (part B 8), about a
                // third of a rotation player's, so a coaching change, a
                // missed bowl or a blue-chip signed at his spot can move him.
                let base = match p.last_role {
                    Some(PlayingTime::Starter) => PIPELINE.portal_entry.rotation / 3.0,
                    Some(PlayingTime::Backup) => PIPELINE.portal_entry.backup,
                    Some(PlayingTime::Rotation) => PIPELINE.portal_entry.rotation,
                    Some(PlayingTime::Redshirt) if !p.cal.academic_sit => {
                        super::recruiting::REDSHIRT_PORTAL * super::recruiting::early_weight(p.stars)
                    }
                    _ => 0.0,
                };
                if base == 0.0 {
                    return;
                }
                // A starter measures himself against the level above.
                let up = if p.last_role == Some(PlayingTime::Starter) {
                    match div {
                        Division::FBS | Division::FCS => Division::FBS,
                        _ => Division::FCS,
                    }
                } else {
                    div
                };
                let m = med[up as usize][p.player.position.index()];
                let buried = clamp(1.0 + 0.12 * (p.player.overall() - m), 0.6, 2.0);
                let comp = 1.0 + (p.player.personality.competitiveness - 10.0) * 0.04;
                p.cal.lean = base;
                let why = match p.last_role {
                    Some(PlayingTime::Starter) => "a starter at fall camp",
                    Some(PlayingTime::Backup) => "a backup at fall camp",
                    Some(PlayingTime::Rotation) => "in the rotation at fall camp",
                    _ => "redshirting this year",
                };
                p.cal.lean_events.push((date, why, base));
                lean_push(
                    p,
                    date,
                    if buried >= 1.0 { "good enough to start elsewhere" } else { "well down the depth chart" },
                    buried,
                );
                lean_push(p, date, "his competitiveness", comp);
                // Far from home with a family that wanted him close: homesickness.
                let miles = geo::miles(p.home_state, programs[c].state);
                if miles > 900.0 && p.family.own_wants.close_to_home > 0.25 {
                    lean_push(p, date, "homesick, far from home", 1.12);
                }
            });
        }
        LeanCheck::MidSeason => {
            let Some(cs) = w.college.as_ref() else { return };
            let games: HashMap<u32, u32> = cs.snaps.iter().map(|(&id, s)| (id, s.1)).collect();
            let on_engine = cs.on_engine;
            let mut whispers = Vec::new();
            for (i, p) in w.people.iter_mut().enumerate() {
                if p.stage != Stage::College || p.cal.lean <= 0.0 {
                    continue;
                }
                if on_engine {
                    let g = games.get(&p.player.id).copied().unwrap_or(0);
                    if g >= 4 {
                        lean_push(p, date, "earning snaps by midseason", 0.85);
                    } else if g == 0 && p.last_role != Some(PlayingTime::Redshirt) {
                        lean_push(p, date, "no snaps by midseason", 1.15);
                    }
                }
                if p.cal.lean >= 0.3 && !p.cal.whispered && (p.stars >= 3 || p.transfers > 0) {
                    p.cal.whispered = true;
                    whispers.push(i);
                }
            }
            for i in whispers {
                let c = w.people[i].college.unwrap_or(0);
                let text = format!(
                    "Whispers say {} ({}) could look elsewhere after the season.",
                    w.people[i].name, w.programs[c].real_name
                );
                w.calendar.post(Event::new(date, CalLevel::College, Kind::Whisper, Loudness::Small, text).program(c));
                w.log.whispers += 1;
            }
        }
        LeanCheck::CoachingChanges => {
            let changed: Vec<bool> = w.programs.iter().map(|p| p.hc_changed == year).collect();
            let factor = PIPELINE.portal_entry.coaching_change;
            for p in w.people.iter_mut() {
                if p.stage == Stage::College
                    && p.cal.lean > 0.0
                    && let Some(c) = p.college
                    && changed[c]
                {
                    lean_push(p, date, "his head coach is gone", factor);
                }
            }
        }
        LeanCheck::Bowls => {
            let Some(cs) = w.college.as_ref() else { return };
            let bowl = cs.bowl_teams.clone();
            for p in w.people.iter_mut() {
                if p.stage != Stage::College || p.cal.lean <= 0.0 {
                    continue;
                }
                let Some(c) = p.college else { continue };
                if w.programs[c].division != Division::FBS {
                    continue;
                }
                if bowl.contains(&c) {
                    lean_push(p, date, "bowl practices and a postseason game", 0.97);
                } else {
                    lean_push(p, date, "no bowl game", 1.08);
                }
            }
        }
        LeanCheck::EarlySigning => {
            // Blue-chips who signed today, by program and position.
            let mut signed: HashSet<(usize, crate::positions::Position)> = HashSet::new();
            for p in w.people.iter() {
                if p.stars >= 4
                    && let Some(s) = p.cal.signed
                    && s.date == date
                {
                    signed.insert((s.program, p.player.position));
                }
            }
            for p in w.people.iter_mut() {
                if p.stage == Stage::College
                    && p.cal.lean > 0.0
                    && let Some(c) = p.college
                    && signed.contains(&(c, p.player.position))
                {
                    lean_push(p, date, "a blue-chip signed at his position", 1.2);
                }
            }
        }
    }
    let _ = seed;
}

// --- The academic calendar (A22-A24) -------------------------------------------------------

/// Finals week (A22): grades for the fall term. A player with poor grades
/// falls behind (his lean to leave grows a little); bowl practices lose two
/// days to exams (in `bowl_practices`).
pub(crate) fn finals_week(w: &mut World) {
    let date = w.today;
    let programs = &w.programs;
    w.people.par_iter_mut().for_each(|p| {
        if p.stage != Stage::College {
            return;
        }
        let Some(c) = p.college else { return };
        let support = clamp(0.3 + 0.7 * programs[c].facilities, 0.0, 1.0);
        let effort = 0.006 * (p.player.personality.work_ethic - 10.0) + 0.03 * (support - 0.5);
        p.bg.gpa = clamp(p.bg.gpa + effort + p.rng.normal(0.0, 0.06), 0.0, 4.0);
        if p.bg.gpa < 2.2 && p.cal.lean > 0.0 {
            p.cal.lean *= 1.1;
            p.cal.lean_events.push((date, "behind in class after finals", 1.1));
        }
    });
}

/// The spring term ends (A23): the NCAA's progress check. A player short of
/// the grades sits the fall, less often with strong academic support. Junior
/// college players are judged by their own rules.
pub(crate) fn spring_term_check(w: &mut World) {
    let programs = &w.programs;
    let flagged: u32 = w
        .people
        .par_iter_mut()
        .map(|p| {
            // The spring term's grades replace the fall term's verdict.
            p.cal.fall_ineligible = false;
            if p.stage != Stage::College {
                return 0;
            }
            let Some(c) = p.college else { return 0 };
            let prog = &programs[c];
            if prog.division == Division::JUCO {
                return 0;
            }
            let support = clamp(0.3 + 0.7 * prog.facilities, 0.0, 1.0);
            let risk = 0.06 * clamp((2.5 - p.bg.gpa) / 0.5, 0.0, 1.0) * (1.3 - support);
            if p.rng.chance(risk) {
                p.cal.spring_ineligible = true;
                let age = p.row_age();
                p.note(format!(
                    "Academically ineligible for the fall at {age} after the spring term (GPA {:.2}).",
                    p.bg.gpa
                ));
                1
            } else {
                0
            }
        })
        .sum();
    w.log.spring_ineligible += flagged;
}

/// The fall term ends (A23): the spring check's rule, on the fall term's
/// grades after finals week. A player short of the grades misses his team's
/// bowl or playoff games and spring practice; the spring term's grades
/// decide the fall. Junior college players are judged by their own rules.
fn fall_term_check(w: &mut World) {
    let season = w.year;
    let five = w.changes.rules(season + 1).five_in_five;
    let seed = w.options.seed as u64 ^ ((season as u64) << 24) ^ 0xFA11_7E2A;
    let playing: Vec<bool> = match w.college.as_ref() {
        Some(cs) => (0..w.programs.len()).map(|t| still_playing(cs, t)).collect(),
        None => vec![false; w.programs.len()],
    };
    let programs = &w.programs;
    let flagged: Vec<(u32, bool)> = w
        .people
        .par_iter_mut()
        .filter_map(|p| {
            if p.stage != Stage::College {
                return None;
            }
            let c = p.college?;
            let prog = &programs[c];
            if prog.division == Division::JUCO {
                return None;
            }
            let support = clamp(0.3 + 0.7 * prog.facilities, 0.0, 1.0);
            let risk = 0.06 * clamp((2.5 - p.bg.gpa) / 0.5, 0.0, 1.0) * (1.3 - support);
            if key_uniform(seed, p.player.id as u64) >= risk {
                return None;
            }
            p.cal.fall_ineligible = true;
            let (age, gpa) = (p.row_age(), p.bg.gpa);
            let what = match (playing[c], !last_season(p, season, five)) {
                (true, true) => "for the postseason and spring practice ",
                (true, false) => "for the postseason ",
                (false, true) => "for spring practice ",
                (false, false) => "",
            };
            p.note(format!("Academically ineligible {what}at {age} after the fall term (GPA {gpa:.2})."));
            Some((p.player.id, playing[c]))
        })
        .collect();
    w.log.fall_ineligible += flagged.len() as u32;
    if let Some(cs) = w.college.as_mut() {
        for (id, _) in flagged.into_iter().filter(|f| f.1) {
            cs.out.insert(id, "academically ineligible");
        }
    }
}

/// The week after finals: the fall term's eligibility check, then December
/// graduation (A24): seniors with three years done and their degree in hand
/// can transfer as graduates and play at once, under every era's rules.
pub(crate) fn december_graduation(w: &mut World) {
    fall_term_check(w);
    let year = w.year;
    let five = w.changes.rules(year + 1).five_in_five;
    let seed = w.options.seed as u64 ^ (year as u64) << 20 ^ 0xDEC6;
    for p in w.people.iter_mut() {
        // December is for the early finishers (part B 36): a good student
        // in his fourth fall, or a fifth-year senior finishing his degree.
        // Proposed, not confirmed: a third of the first, half of the second.
        let years = year + 1 - p.class_of;
        let early = years == 4 && p.bg.gpa >= 3.0 && key_uniform(seed, p.player.id as u64) < 0.35;
        let fifth = years >= 5 && p.bg.gpa >= 2.3 && key_uniform(seed ^ 5, p.player.id as u64) < 0.5;
        if p.stage == Stage::College && (early || fifth) && p.college.is_some() && !p.cal.graduated {
            p.cal.graduated = true;
            let age = p.row_age();
            // The year to use anywhere only when he has a season left.
            if last_season(p, year, five) {
                p.note(format!("Graduated in December at {age}."));
            } else {
                p.note(format!("Graduated in December at {age} with a year of eligibility to use anywhere."));
            }
        }
    }
}

/// Spring commencement in May (part B 36), where most players graduate:
/// at the end of a fourth or fifth year, by his grades. A graduate with a
/// season left may then move as a graduate transfer (`portal_open`).
/// Proposed, not confirmed: 45-90% of fourth-year players by GPA, 85% of
/// fifth-year players with a 2.0 or better.
pub(crate) fn may_graduation(w: &mut World) {
    let year = w.year;
    let seed = w.options.seed as u64 ^ (year as u64) << 20 ^ 0x3A76;
    let mut n = 0;
    for p in w.people.iter_mut() {
        if p.stage != Stage::College || p.cal.graduated || p.college.is_none() {
            continue;
        }
        let years = year - p.class_of;
        let chance = match years {
            ..=3 => 0.0,
            4 => clamp(0.45 + 0.25 * (p.bg.gpa - 2.0), 0.0, 0.9) * (p.bg.gpa >= 2.0) as u8 as f64,
            _ => 0.85 * (p.bg.gpa >= 2.0) as u8 as f64,
        };
        if key_uniform(seed, p.player.id as u64) >= chance {
            continue;
        }
        p.cal.graduated = true;
        p.cal.may_graduate = true;
        let age = p.row_age();
        p.note(format!("Graduated in May at {age}."));
        n += 1;
    }
    w.log.graduates += n;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldOptions;

    /// A world at the end of a regular season: no games left on the schedule.
    fn world() -> World {
        let mut w = World::empty(WorldOptions { scale: 0.01, ..Default::default() });
        let mut cs = new_season(&w);
        cs.games.clear();
        w.college = Some(cs);
        w
    }

    fn id(w: &World, name: &str) -> usize {
        w.programs.iter().find(|p| p.real_name == name).unwrap().id
    }

    fn played(home_won: bool) -> Option<Played> {
        let score = if home_won { [24, 17] } else { [17, 24] };
        Some(Played { score, home_won, temperature: 60.0, ..Played::default() })
    }

    #[test]
    fn a_bcs_title_game_crowns_its_winner_who_leads_the_final_poll() {
        let mut w = world();
        w.rules = w.changes.rules(2005);
        assert_eq!(w.rules.cfb_postseason, CfbPostseason::Bcs);
        let (texas, usc, osu) = (id(&w, "Texas"), id(&w, "USC"), id(&w, "Ohio State"));
        // The poll reads results only; any roster puts a program on the board.
        for p in w.programs.iter_mut().filter(|p| p.division == Division::FBS) {
            p.roster = vec![0];
        }
        let cs = w.college.as_mut().unwrap();
        cs.started = true;
        let mut g = Game::new(Date::ymd(2006, 1, 4), texas, usc, Division::FBS, GameKind::Title);
        g.played = played(true);
        cs.games.push(g);
        crown_title_game(cs, cs.games.len() - 1);
        assert_eq!(cs.champions[0], Some(texas));
        assert!(cs.results[texas].national_title);
        // A better record elsewhere does not top the champion in the final poll.
        cs.results[osu].wins = 13;
        cs.results[texas].wins = 10;
        cs.results[texas].losses = 3;
        release_poll(&mut w, Date::ymd(2006, 1, 5), PollKind::Final);
        let poll = w.college.as_ref().unwrap().polls.last().unwrap();
        assert_eq!(poll.top[0], texas);
        assert!(poll.top.contains(&osu));
    }

    #[test]
    fn rivalry_results_carry_the_trophy_and_the_series() {
        let mut w = world();
        w.recording = true;
        w.college.as_mut().unwrap().year = 2026;
        let (bama, auburn) = (id(&w, "Alabama"), id(&w, "Auburn"));
        let r = RIVALRIES.iter().position(|r| r.a == "Alabama" && r.b == "Auburn").unwrap();
        let mut texts = Vec::new();
        for (day, home, away, home_won) in
            [(1, bama, auburn, true), (2, auburn, bama, true), (3, bama, auburn, false), (4, auburn, bama, true)]
        {
            let date = Date::ymd(2026, 11, day);
            let cs = w.college.as_mut().unwrap();
            let mut g = Game::new(date, home, away, Division::FBS, GameKind::Regular);
            g.rivalry = Some(r);
            g.played = played(home_won);
            cs.games.push(g);
            let k = cs.games.len() - 1;
            result_event(&mut w, k);
            texts.push(w.calendar.on(date).last().unwrap().text.clone());
        }
        assert_eq!(texts[0], "Alabama beat Auburn 24-17 in the Iron Bowl to win the Foy-ODK Trophy.");
        assert_eq!(
            texts[1],
            "Auburn beat Alabama 24-17 in the Iron Bowl to claim the Foy-ODK Trophy. The series is tied 1-1 since 2026."
        );
        assert_eq!(
            texts[2],
            "Auburn beat Alabama 24-17 in the Iron Bowl to keep the Foy-ODK Trophy. Auburn leads the series 2-1 since 2026."
        );
        assert!(
            texts[3].ends_with("to keep the Foy-ODK Trophy. Auburn leads the series 3-1 since 2026."),
            "{}",
            texts[3]
        );
        // The Oregon game by era (the Civil War through 2019).
        let oregon = RIVALRIES.iter().find(|r| r.a == "Oregon" && r.b == "Oregon State").unwrap();
        assert_eq!(oregon.name_in(2019), "the Civil War");
        assert_eq!(oregon.name_in(2020), "the Oregon-Oregon State game");
    }

    /// Rivalries are played in their real years, and the history opens
    /// with each one's real all-time record and trophy holder (part B 28-30).
    #[test]
    fn rivalries_keep_their_real_years_records_and_holders() {
        let past = |a: &str, b: &str| super::super::data::rivalry_past(a, b).unwrap();
        let lone_star = past("Texas", "Texas A&M");
        assert!(lone_star.played(2011) && !lone_star.played(2012) && !lone_star.played(2023) && lone_star.played(2024));
        let heroes = past("Iowa", "Nebraska");
        assert!(!heroes.played(1990) && heroes.played(2011));
        let bedlam = past("Oklahoma", "Oklahoma State");
        assert!(bedlam.played(2023) && !bedlam.played(2024) && !bedlam.played(2030));
        assert!(past("Alabama", "Auburn").played(2030));

        let mut w = World::empty(WorldOptions { scale: 0.01, ..Default::default() });
        w.recording = true;
        w.year = 1996;
        let cs = new_season(&w);
        let r = RIVALRIES.iter().position(|r| r.a == "Alabama" && r.b == "Auburn").unwrap();
        let s = cs.series[r];
        assert_eq!((s.wins, s.ties, s.holder), ([34, 25], 1, Some(id(&w, "Auburn"))));
        assert!(s.real);
    }

    /// The Commander-in-Chief's Trophy is one three-way trophy: the
    /// Air Force games are legs of it, the Army-Navy game settles it, and
    /// the holder keeps it when the academies split their games.
    #[test]
    fn the_academies_play_for_one_commander_in_chiefs_trophy() {
        let mut w = world();
        w.recording = true;
        let (army, navy, af) = (id(&w, "Army"), id(&w, "Navy"), id(&w, "Air Force"));
        let row = |a: &str, b: &str| RIVALRIES.iter().position(|r| r.a == a && r.b == b).unwrap();
        let (na, aa, an) = (row("Navy", "Air Force"), row("Army", "Air Force"), row("Army", "Navy"));
        let mut texts = Vec::new();
        // 2026: Navy beats both. 2027: each academy goes 1-1.
        for (year, games) in [
            (2026, [(navy, af, true), (army, af, true), (army, navy, false)]),
            (2027, [(af, navy, true), (army, af, true), (army, navy, false)]),
        ] {
            let cs = w.college.as_mut().unwrap();
            cs.year = year;
            cs.games.clear();
            for (day, (&(home, away, home_won), r)) in games.iter().zip([na, aa, an]).enumerate() {
                let date = Date::ymd(year, 10 + day as u32, 9);
                let cs = w.college.as_mut().unwrap();
                let mut g = Game::new(date, home, away, Division::FBS, GameKind::Regular);
                g.rivalry = Some(r);
                g.played = played(home_won);
                cs.games.push(g);
                let k = cs.games.len() - 1;
                result_event(&mut w, k);
                texts.push(w.calendar.on(date).last().unwrap().text.clone());
            }
        }
        assert_eq!(
            texts[0],
            "Navy beat Air Force 24-17 in the Navy-Air Force game, a leg of the Commander-in-Chief's Trophy."
        );
        assert!(!texts[1].contains("claim") && !texts[1].contains("keep"), "{}", texts[1]);
        assert!(
            texts[2].ends_with("Navy wins the Commander-in-Chief's Trophy, 2-0 against the other academies."),
            "{}",
            texts[2]
        );
        assert!(
            texts[5].ends_with("The academies split their games, so Navy keeps the Commander-in-Chief's Trophy."),
            "{}",
            texts[5]
        );
        assert_eq!(w.college.as_ref().unwrap().academy_holder, Some(navy));
    }

    /// The awards show reads stats for passers, runners, receivers and
    /// kickers, film for blockers: a kicker who makes his long ones beats
    /// one with more makes who misses short ones.
    #[test]
    fn award_reads_weigh_kicks_by_distance() {
        use crate::positions::Position::*;
        let accurate = Line { fg_att_band: [4, 6, 8, 4], fg_made_band: [4, 6, 7, 3], ..Line::default() };
        let volume = Line { fg_att_band: [10, 10, 8, 4], fg_made_band: [8, 8, 6, 2], ..Line::default() };
        assert!(award_stats(&accurate, K) > award_stats(&volume, K));
        assert_eq!(award_film_weight(OT), 1.0);
        assert_eq!(award_film_weight(QB), 0.0);
        assert_eq!(award_film_weight(LB), 0.5);
    }

    /// A rivalry-week game is huge for both its teams below the FBS too,
    /// so its result posts as big.
    #[test]
    fn a_rivalry_week_game_below_the_fbs_is_huge_for_both() {
        let mut w = world();
        let (harvard, yale) = (id(&w, "Harvard"), id(&w, "Yale"));
        let r = RIVALRIES.iter().position(|r| r.a == "Harvard" && r.b == "Yale").unwrap();
        let date = Date::ymd(2026, 11, 21);
        let cs = w.college.as_mut().unwrap();
        let mut g = Game::new(date, harvard, yale, Division::FCS, GameKind::Regular);
        g.rivalry = Some(r);
        g.tags = vec![Tag::Rivalry];
        cs.games.push(g);
        let k = cs.games.len() - 1;
        stakes_for_day(&mut w, &[k]);
        let cs = w.college.as_mut().unwrap();
        assert_eq!(cs.games[k].huge_for, [true, true]);
        cs.games[k].played = played(true);
        result_event(&mut w, k);
        assert_eq!(w.calendar.on(date).last().unwrap().loud, Loudness::Big);
    }

    /// A game huge for one of its teams posts its result as big at least,
    /// whatever the nation makes of it.
    #[test]
    fn a_teams_huge_game_posts_big() {
        let mut w = world();
        let (a, b) = (id(&w, "Kansas"), id(&w, "Iowa State"));
        let date = Date::ymd(2026, 11, 21);
        let cs = w.college.as_mut().unwrap();
        let mut g = Game::new(date, a, b, Division::FBS, GameKind::Regular);
        g.tags = vec![Tag::HotSeat];
        g.loud = Loudness::Medium;
        g.huge_for = [false, true];
        g.played = played(true);
        cs.games.push(g);
        let k = cs.games.len() - 1;
        result_event(&mut w, k);
        assert_eq!(w.calendar.on(date).last().unwrap().loud, Loudness::Big);
    }

    #[test]
    fn playoff_results_name_their_round_and_teams_play_until_they_lose() {
        let mut w = world();
        let fcs: Vec<usize> = w.programs.iter().filter(|p| p.division == Division::FCS).map(|p| p.id).take(8).collect();
        let dates: Vec<Vec<Date>> =
            [Date::ymd(2026, 12, 5), Date::ymd(2026, 12, 12), Date::ymd(2027, 1, 9)].iter().map(|&d| vec![d]).collect();
        let cs = w.college.as_mut().unwrap();
        cs.brackets.push(Bracket::new(Division::FCS, &fcs, dates, 2));
        advance_brackets(&mut w);
        let mut texts = Vec::new();
        while w.college.as_ref().unwrap().brackets[0].champion.is_none() {
            let pending = w.college.as_ref().unwrap().brackets[0].pending.clone();
            for &k in &pending {
                w.college.as_mut().unwrap().games[k].played = played(true);
                result_event(&mut w, k);
            }
            let k = pending[0];
            let date = w.college.as_ref().unwrap().games[k].date;
            texts.push(w.calendar.on(date).last().unwrap().text.clone());
            // The winners play on; the losers are done once the round advances.
            let (home, away) = (w.college.as_ref().unwrap().games[k].home, w.college.as_ref().unwrap().games[k].away);
            assert!(still_playing(w.college.as_ref().unwrap(), away));
            advance_brackets(&mut w);
            let cs = w.college.as_ref().unwrap();
            assert!(!still_playing(cs, away));
            assert_eq!(still_playing(cs, home), cs.brackets[0].champion.is_none());
        }
        assert!(texts[0].ends_with(" in the FCS quarterfinals."), "{}", texts[0]);
        assert!(texts[1].ends_with(" in the FCS semifinals."), "{}", texts[1]);
        assert!(texts[2].ends_with(" for the FCS title."), "{}", texts[2]);
        // Only the FBS playoff holds portal entrants.
        let cs = w.college.as_ref().unwrap();
        assert!(fcs.iter().all(|&t| !playoff_alive(cs, t)));
    }
}
