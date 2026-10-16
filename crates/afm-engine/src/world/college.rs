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
use super::data::Division;
use super::news::{Channel, NewsItem, NewsKind};
use super::schedule::{self, Game, GameKind, Played, Site, Tag};
use super::season::{self, COLLEGE_SPREAD, CoachContext, TeamResult, key_normal, key_uniform, win_chance};
use super::{Stage, World, division_level, geo};
use crate::calendar::data::{BOWLS, CELEBRATION_BOWL, INDOOR_PROGRAMS, RIVALRIES, et_offset};
use crate::calendar::weather::{self, Severe};
use crate::calendar::{CfbPostseason, Date, Event, Kind, Level as CalLevel, Loudness};
use crate::game::{Conditions, GameTeam, quick_game_in};
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
    pub champions: [Option<usize>; 4],
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
}

/// A rivalry's record over the world's history (A28): wins by its first
/// and second listed programs, the first season counted, and the trophy's
/// holder (the last rivalry game's winner).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Series {
    pub wins: [u32; 2],
    pub since: Option<i32>,
    pub holder: Option<usize>,
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
    let series = w.college_prev.as_ref().map_or_else(|| vec![Series::default(); RIVALRIES.len()], |c| c.series.clone());
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
        champions: [None; 4],
        finished: false,
        credited: vec![(0, 0); np],
        awards: Vec::new(),
        bowl_teams: HashSet::new(),
        series,
        rivalry_of,
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
fn conditions(w: &World, g: &Game, k: usize) -> (Conditions, Played, Option<Severe>) {
    let home = &w.programs[g.home];
    let (state, indoor) = match g.site {
        Some(s) => (s.state, s.indoor),
        None => (home.state, INDOOR_PROGRAMS.contains(&home.real_name.as_str())),
    };
    let code = w.states[state].code.as_str();
    let local = (g.kickoff.0 as i32 - et_offset(code)).clamp(9, 23) as u32;
    let seed = w.options.seed as u64;
    let wx = if indoor { weather::indoors() } else { weather::weather_at(seed, state, code, g.date, local) };
    let along = (key_uniform(seed ^ 0xA10, (g.date.0 as u64) << 16 ^ k as u64) * 2.0 - 1.0).clamp(-1.0, 1.0);
    let cond = Conditions {
        temperature: wx.temperature,
        wind: wx.wind,
        wind_along: along,
        rain: wx.rain,
        snow: wx.snow,
        crowd: 0.5,
        ..Conditions::default()
    };
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
    (cond, played, wx.severe)
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
    // From 2018 a staff stops playing a fall-camp redshirt once he has four
    // games, so he keeps the year (settle_eligibility).
    let capped: HashSet<u32> = if w.rules.four_game_redshirt && cs.on_engine {
        w.people
            .iter()
            .filter(|p| matches!(p.stage, Stage::College) && !p.cal.count_pending && !p.cal.academic_sit)
            .filter(|p| p.last_role == Some(PlayingTime::Redshirt) && p.redshirt_used)
            .map(|p| p.player.id)
            .filter(|id| cs.snaps.get(id).is_some_and(|s| s.1 >= 4))
            .collect()
    } else {
        HashSet::new()
    };
    let seed = w.options.seed as u64 ^ ((cs.year as u64) << 32) ^ ((date.0 as u64) << 8);
    struct Job {
        k: usize,
        cond: Conditions,
        played: Played,
        neutral: bool,
        teams: Option<(GameTeam, GameTeam)>,
        context: [crate::game::garbage::Context; 2],
    }
    let jobs: Vec<Job> = idx
        .iter()
        .map(|&k| {
            let g = &cs.games[k];
            let (cond, played, _) = conditions(w, g, k);
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
            Job { k, cond, played, neutral: g.site.is_some(), teams, context }
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
            Some(quick_game_in(a, b, level, s, j.context, j.cond, j.neutral))
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
            let pe = win_chance(cs.results[home].preseason, cs.results[away].preseason, COLLEGE_SPREAD, adv);
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
    let cs = w.college.as_ref().unwrap();
    let g = &cs.games[k];
    let Some(p) = g.played else { return };
    if g.loud == Loudness::Small && g.rivalry.is_none() && g.kind == GameKind::Regular {
        return;
    }
    let (wi, li, ws, ls) =
        if p.home_won { (g.home, g.away, p.score[0], p.score[1]) } else { (g.away, g.home, p.score[1], p.score[0]) };
    // Playoff games name their round; a title game below the FBS names its
    // title; a BCS title game in a host bowl says what was at stake.
    let level = if g.division == Division::FBS { "CFP" } else { g.division.name() };
    let what = match (g.kind, g.name) {
        (GameKind::Playoff(_), name) => match (name, playoff_round(cs, k).unwrap_or("first round")) {
            (Some(n), round) => format!(" in the {n}, a {level} {round}"),
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
                        format!(" in {name} {how} {t}")
                    }
                    None => format!(" in {name}"),
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
        if a + b < 2 {
            String::new()
        } else if a == b {
            format!(" The series is tied {a}-{b} since {since}.")
        } else {
            let other = if first == g.home { g.away } else { g.home };
            let leader = if a > b { first } else { other };
            format!(" {} leads the series {}-{} since {since}.", w.programs[leader].real_name, a.max(b), a.min(b))
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
    let text =
        format!("{} beat {} {ws}-{ls}{weather}{what}.{series}", w.programs[wi].real_name, w.programs[li].real_name);
    let loud = g.loud.max(if g.kind != GameKind::Regular { Loudness::Medium } else { Loudness::Small });
    // Below the FBS a game is huge for its two teams, but national news
    // carries it as big at most (proposed, not confirmed).
    let loud = if g.division == Division::FBS { loud } else { loud.min(Loudness::Big) };
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
    let members: Vec<usize> =
        w.programs.iter().filter(|p| p.division == division && !p.roster.is_empty()).map(|p| p.id).collect();
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
fn stakes_for_day(w: &mut World, idx: &[usize]) {
    let Some(cs) = w.college.as_ref() else { return };
    let mut updates: Vec<(usize, Vec<Tag>, Loudness, [bool; 2])> = Vec::new();
    let mut notes: Vec<(usize, String)> = Vec::new();
    // Each team's own race (T8.1): the fewest conference losses in its
    // conference, and the regular-season games it has left.
    let mut conf_best: HashMap<&str, u32> = HashMap::new();
    for (t, prog) in w.programs.iter().enumerate() {
        if prog.division == Division::FBS && !prog.roster.is_empty() {
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
        // Loudness: the stakes test. Story tags (A40) count half a stake each,
        // at most one in all, so story alone never makes a game huge
        // (proposed, not confirmed).
        let real = tags.iter().filter(|t| matches!(t, Tag::PlayoffSpot | Tag::Ranked)).count()
            + (tags.contains(&Tag::Rivalry) && (ranked(g.home).is_some() || ranked(g.away).is_some())) as usize
            + (tags.contains(&Tag::HotSeat) && tags.contains(&Tag::Rivalry)) as usize;
        let story = tags
            .iter()
            .filter(|t| matches!(t, Tag::Revenge | Tag::CoachReturn | Tag::Hometown | Tag::Milestone))
            .count();
        let stakes = real as f64 + (0.5 * story as f64).min(1.0);
        let loud = match g.kind {
            GameKind::Title => Loudness::Huge,
            GameKind::Playoff(_) if g.division == Division::FBS => Loudness::Huge,
            _ if stakes >= 2.0 => Loudness::Huge,
            _ if stakes >= 1.0 => Loudness::Big,
            _ if !tags.is_empty() => Loudness::Medium,
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
        let mut huge_for = [false; 2];
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
                huge_for[side] = rivalry_week
                    || rivalry && n >= 1
                    || n >= 2
                    || g.kind != GameKind::Regular && loud == Loudness::Huge;
            }
        }
        updates.push((k, tags, loud, huge_for));
    }
    let cs = w.college.as_mut().unwrap();
    for (k, tags, loud, huge_for) in updates {
        cs.games[k].tags = tags;
        cs.games[k].loud = loud;
        cs.games[k].huge_for = huge_for;
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
            let text =
                format!("{holder} finishes with {v:.0} {stat}, a {} single-season record.", w.programs[prog].real_name);
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
    for p in w.programs.iter().filter(|p| p.division == division && !p.roster.is_empty()) {
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
        Division::D2 => y.d2_rounds.clone(),
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
    let era = match season {
        ..=2015 => 0.05,
        2016 | 2017 => 0.4,
        _ => 1.0,
    };
    // Each bowl team's first postseason game.
    let mut first: HashMap<usize, (GameKind, bool)> = HashMap::new();
    for g in cs.games.iter().filter(|g| g.played.is_none() && g.kind != GameKind::Regular) {
        let major = g.name.is_some_and(|n| BOWLS.iter().any(|b| b.name == n && b.major));
        for t in [g.home, g.away] {
            first.entry(t).or_insert((g.kind, major));
        }
    }
    let seed = w.options.seed as u64 ^ ((season as u64) << 20) ^ 0x0F70;
    let mut outs: Vec<(u32, usize, String, bool)> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        if p.stage != Stage::College {
            continue;
        }
        let Some(c) = p.college else { continue };
        let Some(&(kind, major)) = first.get(&c) else { continue };
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
            outs.push((p.player.id, i, format!("Opted out of {game} at {} {why}.", p.player.age), senior));
        }
    }
    let date = w.today;
    for (id, i, note, senior) in outs {
        let c = w.people[i].college.unwrap_or(0);
        let name = w.people[i].name.clone();
        w.people[i].note(note);
        w.people[i].cal.opted_out = true;
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
    // candidate, and a former winner is held to a higher bar: nobody has won
    // twice since Archie Griffin (1974-75, from memory, to verify). Claude's
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
            let s = weight * prod / bar[kind] * team * if former { 0.85 } else { 1.0 };
            (s, id, i)
        })
        .collect();
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
                let age = w.people[i].player.age;
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
                let age = w.people[i].player.age;
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
                // By the season's play (performance) for linemen and kickers, stats otherwise.
                let best = w
                    .people
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| {
                        p.stage == Stage::College
                            && positions.contains(&p.player.position)
                            && p.last_role == Some(PlayingTime::Starter)
                            && p.college.is_some_and(|c| w.programs[c].division == Division::FBS)
                    })
                    .max_by(|a, b| {
                        let va = a.1.last_performance.unwrap_or(0.0)
                            + cs.lines.get(&a.1.player.id).map_or(0.0, |l| 0.01 * production(l, a.1.player.position));
                        let vb = b.1.last_performance.unwrap_or(0.0)
                            + cs.lines.get(&b.1.player.id).map_or(0.0, |l| 0.01 * production(l, b.1.player.position));
                        va.total_cmp(&vb)
                    })
                    .map(|(i, _)| i);
                if let Some(i) = best {
                    picks.push((award, i));
                }
            }
            let mut lines = Vec::new();
            for (award, i) in picks {
                let c = w.people[i].college.unwrap_or(0);
                let id = w.people[i].player.id;
                let age = w.people[i].player.age;
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
    let results: Vec<TeamResult> = cs.results.clone();
    for p in 0..w.programs.len() {
        let t = &results[p];
        if t.games() == 0 {
            continue;
        }
        let prog = &mut w.programs[p];
        prog.seasons.push(super::program::ProgramSeason {
            year,
            wins: t.wins,
            losses: t.losses,
            conference_title: t.conference_title,
            national_title: false,
            coach: prog.head_coach,
            strength: t.strength,
            totals: t.totals,
        });
        let locker = clamp(t.incidents as f64 / t.roster.max(1) as f64 * 8.0, 0.0, 1.0);
        let staff: Vec<(Option<usize>, bool)> = [(prog.head_coach, true), (prog.oc, false), (prog.dc, false)]
            .into_iter()
            .chain(prog.position_coaches.iter().map(|&c| (c, false)))
            .collect();
        for (c, head) in staff {
            let Some(c) = c else { continue };
            let coach = &mut w.coaches[c];
            if head {
                coach.wins += t.wins;
                coach.losses += t.losses;
            }
            coach.season_reputation(t.vs_expectation(), locker, head);
        }
        w.college.as_mut().unwrap().credited[p] = (t.wins, t.losses);
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
            let staff: Vec<(Option<usize>, bool)> = [(prog.head_coach, true), (prog.oc, false), (prog.dc, false)]
                .into_iter()
                .chain(prog.position_coaches.iter().map(|&c| (c, false)))
                .collect();
            for (c, head) in staff {
                let Some(c) = c else { continue };
                let coach = &mut w.coaches[c];
                if head {
                    coach.wins += t.wins.saturating_sub(cw);
                    coach.losses += t.losses.saturating_sub(cl);
                    if t.best_title() != TitleKind::None {
                        coach.titles += 1;
                        coach.career_titles += 1;
                    }
                }
                coach.title_reputation(t.best_title(), head, stage);
            }
        }
        for (d, champ) in champions.iter().enumerate() {
            let Some(c) = *champ else { continue };
            let division = [Division::FBS, Division::FCS, Division::D2, Division::JUCO][d];
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
/// 2018 any game a redshirt played cost him the year.
pub(crate) fn settle_eligibility(w: &mut World) {
    let Some(cs) = w.college.as_ref() else { return };
    let four = w.rules.four_game_redshirt;
    let on_engine = cs.on_engine;
    let games: HashMap<u32, u32> = cs.snaps.iter().map(|(&id, s)| (id, s.1)).collect();
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
            let age = p.player.age;
            let games = if played == 1 { "game" } else { "games" };
            if !four || played > 4 {
                p.redshirt_used = false;
                p.college_seasons += 1;
                p.note(format!("Played in {played} {games} at {age}, which cost him his redshirt year."));
            } else {
                kept += 1;
                p.note(format!(
                    "Played in {played} {games} at {age} and kept his redshirt year under the four-game rule."
                ));
            }
            continue;
        }
        p.cal.count_pending = false;
        if four && on_engine && !p.redshirt_used && played <= 4 {
            p.redshirt_used = true;
            kept += 1;
            if p.cal.redshirt_trick {
                let age = p.player.age;
                p.note(format!("Stopped playing after four games at {age} to keep his redshirt year."));
            }
        } else {
            p.college_seasons += 1;
        }
    }
    w.log.four_game_redshirts += kept;
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
            let mut perf: [[Vec<f64>; crate::positions::POSITION_COUNT]; 4] = Default::default();
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
                let base = match p.last_role {
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
                let m = med[div as usize][p.player.position.index()];
                let buried = clamp(1.0 + 0.12 * (p.player.overall() - m), 0.6, 2.0);
                let comp = 1.0 + (p.player.personality.competitiveness - 10.0) * 0.04;
                p.cal.lean = base;
                let why = match p.last_role {
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
            let changed: Vec<bool> = w
                .programs
                .iter()
                .map(|p| p.head_coach.is_some_and(|c| w.coaches[c].job.is_some_and(|j| j.since == year)))
                .collect();
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
                let age = p.player.age;
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
            let (age, gpa) = (p.player.age, p.bg.gpa);
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
    for p in w.people.iter_mut() {
        if p.stage == Stage::College
            && year + 1 - p.class_of >= 4
            && p.bg.gpa >= 2.6
            && p.college.is_some()
            && !p.cal.graduated
        {
            p.cal.graduated = true;
            let age = p.player.age;
            // The year to use anywhere only when he has a season left.
            if last_season(p, year, five) {
                p.note(format!("Graduated in December at {age}."));
            } else {
                p.note(format!("Graduated in December at {age} with a year of eligibility to use anywhere."));
            }
        }
    }
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
