//! P5-P7: your game decisions graded the next morning (fourth downs and
//! timeouts, read by the analytics bots of each era, the critics and your
//! boss), your club's schedule release (prime-time games, short weeks,
//! flexes) and the NFL abroad (a home game given up, the trip, the clubs'
//! international marketing rights from 2022).

use std::sync::LazyLock;

use crate::calendar::{Date, Level, Loudness};
use crate::game::{GameStats, PlayType};
use crate::world::World;
use crate::world::nfl_year::Window;

use super::lines::Fill;
use super::{GameCtx, TeamKey, news, post, team_fill};

const IMR_CSV: &str = include_str!("../../../../../data/real-world/intl_marketing_rights.csv");

/// (club, country, first season) of the NFL's international marketing
/// rights (the Global Markets Program, from 2022).
static IMR: LazyLock<Vec<(String, String, i32)>> = LazyLock::new(|| {
    IMR_CSV
        .lines()
        .skip(1)
        .filter_map(|l| {
            let f = super::numbers::csv_fields(l);
            let year = f.get(2)?.parse().ok()?;
            Some((f.first()?.clone(), f.get(1)?.clone(), year))
        })
        .collect()
});

/// A club's international marketing countries in a season.
pub fn marketing_rights(club: &str, year: i32) -> Vec<String> {
    IMR.iter().filter(|x| x.0 == club && x.2 <= year).map(|x| x.1.clone()).collect()
}

/// What a fourth down became.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Choice {
    Go,
    Punt,
    FieldGoal,
}

impl Choice {
    pub fn word(self) -> &'static str {
        match self {
            Choice::Go => "go for it",
            Choice::Punt => "punt",
            Choice::FieldGoal => "kick the field goal",
        }
    }
}

/// One of your fourth downs, as the bot read it.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Call {
    pub quarter: u8,
    /// Seconds left in the quarter.
    pub clock: u16,
    pub togo: u8,
    /// Yards from the goal line.
    pub yardline: u8,
    /// Your lead (negative behind).
    pub lead: i8,
    pub chose: Choice,
    pub bot: Choice,
    /// Win probability given up against the bot's call, in percentage
    /// points (0 when you made its call).
    pub cost: f32,
    /// What your call gained on the next best one, in the same points.
    #[serde(default)]
    pub edge: f32,
    /// What happened ("converted", "stopped short", "good from 44").
    pub result: String,
}

/// One game's morning-after report.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub date: Date,
    pub team: TeamKey,
    pub opp: TeamKey,
    /// Your points, theirs.
    pub score: [i32; 2],
    /// The bot that read it (empty before the bots: the columnists judged
    /// by the result).
    pub bot: String,
    pub calls: Vec<Call>,
    /// Timeouts you still held when the game ended behind.
    pub timeouts_left: u8,
    /// The lines the morning brought: the bot's, the critics', your boss's.
    pub lines: Vec<String>,
    /// Harsh 0, fair 1, praise 2.
    pub tone: u8,
}

/// Your club's schedule as released, to catch a flex.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ScheduleStory {
    pub year: i32,
    pub team: u16,
    pub prime: u8,
    pub short: u8,
    pub abroad: Option<String>,
    /// Each game's (date, opponent, window name) at release, updated by
    /// flexes.
    pub games: Vec<(Date, u16, String)>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct NflStories {
    /// Each club's running season: points for, points against, sacks,
    /// takeaways, games (for unit names).
    pub totals: Vec<[f64; 5]>,
    pub totals_year: i32,
    /// Your morning-after reports, newest last (two seasons or so).
    pub reports: Vec<Report>,
    pub schedule: Option<ScheduleStory>,
}

impl NflStories {
    /// How often you went for it when the bot said go, this season.
    pub fn go_rate(&self, year: i32) -> Option<f64> {
        let calls: Vec<&Call> = self
            .reports
            .iter()
            .filter(|r| r.date.year() == year || r.date.year() == year + 1 && r.date.month() <= 2)
            .flat_map(|r| &r.calls)
            .filter(|c| c.bot == Choice::Go)
            .collect();
        if calls.len() < 5 {
            return None;
        }
        Some(calls.iter().filter(|c| c.chose == Choice::Go).count() as f64 / calls.len() as f64)
    }
}

/// The bot that grades fourth downs in a season, by level (from memory, to
/// verify): Advanced NFL Stats' calculator from 2007, the New York Times'
/// 4th Down Bot from 2014, EdjSports' Critical Call reports from 2018 and
/// the fourth-down bots on social media from 2020; in college the bots
/// arrive about 2018. Before them, the columnists judged by the result.
pub fn bot(level: Level, year: i32, key: u64) -> Option<&'static str> {
    match level {
        Level::Nfl => Some(match year {
            ..=2006 => return None,
            2007..=2013 => "the Advanced NFL Stats calculator",
            2014..=2017 => "The New York Times' 4th Down Bot",
            2018..=2019 => "EdjSports' Critical Call report",
            _ => {
                if key.is_multiple_of(2) {
                    "EdjSports' Critical Call report"
                } else {
                    "the fourth-down bots"
                }
            }
        }),
        Level::College if year >= 2018 => Some("the college fourth-down bots"),
        _ => None,
    }
}

/// A league kicker's chance from a distance, by level and era (NFL 2015-25
/// about 97% inside 30, 92% from 40, 70% from 55; college a few yards
/// shorter).
fn league_make(dist: f64, level: Level, year: i32) -> f64 {
    let mid = match level {
        Level::Nfl => 58.0 + ((year - 2000) as f64 * 0.12).clamp(-1.5, 3.0),
        _ => 52.0,
    };
    1.0 / (1.0 + ((dist - mid) / 6.0).exp())
}

/// The bot's read of one fourth down: its call and each choice's expected
/// points.
pub fn read(
    yardline: f64,
    togo: f64,
    lead: i32,
    quarter: u8,
    game_left: f64,
    level: Level,
    year: i32,
) -> (Choice, [f64; 3]) {
    let make = league_make(yardline + 17.0, level, year);
    let (_, go, punt, fg) = crate::game::calling::fourth_ev(yardline, togo, make, false);
    let late = quarter >= 4 && game_left < 300.0;
    let best = if late && lead < -3 {
        Choice::Go
    } else if late && lead < 0 && make > 0.6 && lead >= -3 {
        Choice::FieldGoal
    } else if go >= punt.max(fg) {
        Choice::Go
    } else if fg > punt && make > 0.3 {
        Choice::FieldGoal
    } else {
        Choice::Punt
    };
    (best, [go, punt, fg])
}

fn choice_of(kind: PlayType) -> Option<Choice> {
    match kind {
        PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble => Some(Choice::Go),
        PlayType::Punt => Some(Choice::Punt),
        PlayType::FieldGoal => Some(Choice::FieldGoal),
        _ => None,
    }
}

/// Grades your fourth downs and timeouts from one game and posts the
/// morning's lines the next day.
pub(crate) fn grade(w: &mut World, ctx: &GameCtx, s: &GameStats) {
    let Some(me) = super::seat_team(w) else { return };
    let Some(side) = ctx.teams.iter().position(|&t| t == me) else { return };
    if ctx.level == Level::HighSchool {
        return;
    }
    let year = w.today.year();
    let qlen = if ctx.level == Level::HighSchool { 720.0 } else { 900.0 };
    let opp = ctx.teams[1 - side];
    let key = w.today.0 as u64 ^ side as u64;
    let bot = bot(ctx.level, year, key);
    let mut calls = Vec::new();
    for r in s.plays.iter().filter(|r| r.offense == side && r.down == 4 && !r.nullified && r.quarter <= 4) {
        let Some(chose) = choice_of(r.kind) else { continue };
        // Garbage time and a kneel-out aren't decisions anyone grades.
        if r.garbage[side] > 0 {
            continue;
        }
        let lead = r.score[side] - r.score[1 - side];
        let left = (4 - r.quarter as i32).max(0) as f64 * qlen + r.clock;
        let i = |c: Choice| match c {
            Choice::Go => 0,
            Choice::Punt => 1,
            Choice::FieldGoal => 2,
        };
        // The engine's own win probabilities for the snap (go, punt, kick,
        // with his kicker's range) when the play carries them; otherwise the
        // bot's expected points against how much a point is worth here.
        let (best, cost, edge) = match r.fourth {
            Some(v) => {
                let wp = [v[0], v[1], if v[2] > 0.0 { v[2] } else { -1.0 }];
                let best = [Choice::Go, Choice::Punt, Choice::FieldGoal]
                    .into_iter()
                    .max_by(|a, b| wp[i(*a)].total_cmp(&wp[i(*b)]))
                    .unwrap_or(Choice::Punt);
                let mine = wp[i(chose)].max(0.0);
                let second = [Choice::Go, Choice::Punt, Choice::FieldGoal]
                    .into_iter()
                    .filter(|c| *c != chose)
                    .map(|c| wp[i(c)])
                    .fold(-1.0f32, f32::max);
                (best, (wp[i(best)] - mine).max(0.0) * 100.0, (mine - second.max(0.0)).max(0.0) * 100.0)
            }
            None => {
                let (best, ev) = read(r.yardline_100, r.togo, lead, r.quarter, left, ctx.level, year);
                let wp = |d: i32| crate::game::tables::win_probability(d, left, r.yardline_100);
                let lever = ((wp(lead + 4) - wp(lead - 4)) / 8.0).max(0.004);
                let cost = ((ev[i(best)] - ev[i(chose)]).max(0.0) * lever * 100.0) as f32;
                let second = (0..3).filter(|&k| k != i(chose)).map(|k| ev[k]).fold(f64::MIN, f64::max);
                (best, cost, ((ev[i(chose)] - second).max(0.0) * lever * 100.0) as f32)
            }
        };
        let cost = if chose == best { 0.0 } else { cost };
        let result = match chose {
            Choice::Go if r.touchdown => "touchdown".to_string(),
            Choice::Go if r.first_down => "converted".to_string(),
            Choice::Go if r.turnover => "turned it over".to_string(),
            Choice::Go => "stopped short".to_string(),
            Choice::Punt => format!("punted {:.0}", r.kick_distance),
            Choice::FieldGoal => {
                format!("{} from {:.0}", if r.kick_good { "good" } else { "no good" }, r.yardline_100 + 17.0)
            }
        };
        calls.push(Call {
            quarter: r.quarter,
            clock: r.clock as u16,
            togo: r.togo.round().clamp(0.0, 99.0) as u8,
            yardline: r.yardline_100.round().clamp(0.0, 99.0) as u8,
            lead: lead.clamp(-99, 99) as i8,
            chose,
            bot: best,
            cost,
            edge,
            result,
        });
    }
    let score = [s.score[side], s.score[1 - side]];
    let lost = score[0] < score[1];
    let timeouts_left = if lost && score[1] - score[0] <= 8 { s.timeouts_left[1][side] } else { 0 };
    let seed = w.options.seed;
    let coach = w.career.as_ref().map(|c| c.you.name.clone()).unwrap_or_default();
    let base = team_fill(w, me, Fill::new()).set("coach", coach.clone()).set("opp", super::short_name(w, opp));
    let mut lines = Vec::new();
    // The bot's verdict on the costliest call, or its nod to a good one.
    let worst = calls.iter().max_by(|a, b| a.cost.total_cmp(&b.cost)).cloned();
    let bold = calls.iter().find(|c| c.chose == Choice::Go && c.bot == Choice::Go && c.lead <= 7);
    if let Some(b) = bot {
        let f = base.clone().set("bot", b.to_string());
        match &worst {
            Some(c) if c.cost >= 1.5 => {
                let key = match (c.chose, c.bot) {
                    (_, Choice::Go) if c.chose == Choice::Punt => "grade.punt_wrong",
                    (_, Choice::Go) => "grade.kick_wrong",
                    _ => "grade.go_wrong",
                };
                let f = f.set("wp", format!("{:.1} points of win probability", c.cost));
                if let Some(t) = w.stories.line(seed, key, &f, year) {
                    lines.push(t);
                }
            }
            _ => {
                if let Some(c) = bold {
                    let gain = c.edge.max(0.1);
                    let f = f.set("wp", format!("{gain:.1} points of win probability"));
                    if let Some(t) = w.stories.line(seed, "grade.agree", &f, year) {
                        lines.push(t);
                    }
                }
            }
        }
    }
    if timeouts_left >= 1
        && let Some(t) = w.stories.line(seed, "grade.timeout", &base, year)
    {
        lines.push(t);
    }
    // The critics: by the numbers once there are numbers, by the result
    // before. A failed gamble in a loss is the classic bad morning.
    let gamble_failed = calls.iter().any(|c| c.chose == Choice::Go && c.result == "stopped short");
    let gamble_won =
        calls.iter().any(|c| c.chose == Choice::Go && c.result != "stopped short" && c.result != "turned it over");
    let total: f32 = calls.iter().map(|c| c.cost).sum();
    let tone = if bot.is_some() {
        if lost && (total >= 3.0 || timeouts_left >= 1) {
            0
        } else if !lost && (bold.is_some() || total < 0.5) && !calls.is_empty() {
            2
        } else {
            1
        }
    } else if lost && (gamble_failed || timeouts_left >= 1) {
        0
    } else if !lost && gamble_won {
        2
    } else {
        1
    };
    let worth = !lines.is_empty() || tone != 1 || ctx.big;
    if worth {
        let key = ["grade.critic.harsh", "grade.critic.fair", "grade.critic.praise"][tone as usize];
        if let Some(t) = w.stories.line(seed, key, &base, year) {
            lines.push(t);
        }
    }
    let boss = tone == 0 && (total >= 4.0 || ctx.big && total >= 2.0);
    let mut boss_line = None;
    if boss {
        boss_line = w.stories.line(seed, "grade.boss", &base, year);
    }
    let tomorrow = w.today.add(1);
    for t in &lines {
        let e = news(w, ctx.level, if tone == 0 { Loudness::Medium } else { Loudness::Small }, Some(me), t.clone());
        w.stories.later(tomorrow, e);
    }
    if let Some(t) = &boss_line {
        let e = news(w, ctx.level, Loudness::Medium, Some(me), t.clone());
        w.stories.later(tomorrow, e);
        lines.push(t.clone());
    }
    if calls.is_empty() && lines.is_empty() {
        return;
    }
    let r = Report {
        date: tomorrow,
        team: me,
        opp,
        score,
        bot: bot.unwrap_or("").to_string(),
        calls,
        timeouts_left,
        lines,
        tone,
    };
    let st = &mut w.stories.nfl;
    st.reports.push(r);
    if st.reports.len() > 60 {
        st.reports.remove(0);
    }
}

/// After every NFL game: the clubs' running totals.
pub(crate) fn after_game(w: &mut World, ctx: &GameCtx, s: &GameStats) {
    let year = w.year;
    let n = w.nfl.len();
    let st = &mut w.stories.nfl;
    if st.totals_year != year || st.totals.len() != n {
        st.totals = vec![[0.0; 5]; n];
        st.totals_year = year;
    }
    if ctx.postseason {
        return;
    }
    for side in 0..2 {
        let TeamKey::Nfl(t) = ctx.teams[side] else { continue };
        let Some(v) = st.totals.get_mut(t as usize) else { continue };
        v[0] += s.score[side] as f64;
        v[1] += s.score[1 - side] as f64;
        v[2] += s.team[1 - side].sacks as f64;
        v[3] += s.team[1 - side].turnovers as f64;
        v[4] += 1.0;
    }
}

/// Your club's games this season as (date, opponent, home, window, site).
fn my_games(w: &World, t: usize) -> Vec<(Date, usize, bool, Window, Option<String>)> {
    let Some(s) = w.nfl_season.as_ref() else { return Vec::new() };
    let mut out = Vec::new();
    for (k, week) in s.weeks.iter().enumerate() {
        for (g, &(h, a)) in week.iter().enumerate() {
            if h != t && a != t {
                continue;
            }
            let Some(sl) = s.slots.get(k).and_then(|x| x.get(g)) else { continue };
            out.push((sl.at.date, if h == t { a } else { h }, h == t, sl.window, sl.site.clone()));
        }
    }
    out.sort_by_key(|x| x.0);
    out
}

fn release(w: &mut World, t: usize) {
    let games = my_games(w, t);
    if games.is_empty() {
        return;
    }
    let year = w.year;
    let seed = w.options.seed;
    let me = TeamKey::Nfl(t as u16);
    let prime = games.iter().filter(|g| g.3.prime()).count();
    let short = games.windows(2).filter(|p| p[0].0.days_to(p[1].0) < 6).count();
    let f = team_fill(w, me, Fill::new()).set("n", prime.to_string()).set("short", short.to_string());
    let key = if prime == 0 { "sched.snub" } else { "sched.release" };
    if let Some(text) = w.stories.line(seed, key, &f, year) {
        let e = news(w, Level::Nfl, Loudness::Medium, Some(me), text);
        post(w, e);
    }
    let abroad = games.iter().find(|g| g.4.is_some()).cloned();
    if let Some((_, opp, home, _, Some(site))) = &abroad {
        let f = team_fill(w, me, Fill::new())
            .set("opp", super::short_name(w, TeamKey::Nfl(*opp as u16)))
            .set("site", site.clone());
        if let Some(text) = w.stories.line(seed, "sched.intl", &f, year) {
            let e = news(w, Level::Nfl, Loudness::Medium, Some(me), text);
            post(w, e);
        }
        if *home && let Some(text) = w.stories.line(seed, "sched.home_given", &f, year) {
            let e = news(w, Level::Nfl, Loudness::Small, Some(me), text);
            post(w, e);
        }
    }
    w.stories.nfl.schedule = Some(ScheduleStory {
        year,
        team: t as u16,
        prime: prime as u8,
        short: short as u8,
        abroad: abroad.and_then(|g| g.4),
        games: games.iter().map(|g| (g.0, g.1 as u16, g.3.name().to_string())).collect(),
    });
}

/// In season: a flex that moved your game, the short weeks, the trip abroad.
fn season_day(w: &mut World, d: Date, t: usize) {
    let games = my_games(w, t);
    let Some(story) = w.stories.nfl.schedule.clone() else { return };
    if story.year != w.year || story.team as usize != t {
        return;
    }
    let year = w.year;
    let seed = w.options.seed;
    let me = TeamKey::Nfl(t as u16);
    // A flex: the same opponent on a new day or window.
    for g in &games {
        let Some(old) = story.games.iter().find(|x| x.1 as usize == g.1) else { continue };
        if old.0 == g.0 && old.2 == g.3.name() {
            continue;
        }
        if g.0 < d {
            continue;
        }
        let f = team_fill(w, me, Fill::new())
            .set("opp", super::short_name(w, TeamKey::Nfl(g.1 as u16)))
            .set("window", g.3.name().to_string());
        let key = if g.3.prime() { "sched.flex_in" } else { "sched.flex_out" };
        if let Some(text) = w.stories.line(seed, key, &f, year) {
            let e = news(w, Level::Nfl, Loudness::Medium, Some(me), text);
            post(w, e);
        }
    }
    if let Some(s) = w.stories.nfl.schedule.as_mut() {
        s.games = games.iter().map(|g| (g.0, g.1 as u16, g.3.name().to_string())).collect();
    }
    // The day after a game: a short week ahead, or the trip abroad this week.
    let Some(k) = games.iter().position(|g| g.0.add(1) == d) else { return };
    let Some(next) = games.get(k + 1) else { return };
    let days = games[k].0.days_to(next.0);
    let opp = super::short_name(w, TeamKey::Nfl(next.1 as u16));
    if days < 6 {
        let f = team_fill(w, me, Fill::new()).set("opp", opp.clone()).set("days", days.to_string());
        if let Some(text) = w.stories.line(seed, "sched.short_week", &f, year) {
            let e = news(w, Level::Nfl, Loudness::Small, Some(me), text);
            post(w, e);
        }
    }
    if let Some(site) = &next.4 {
        let f = team_fill(w, me, Fill::new()).set("opp", opp).set("site", site.clone());
        if let Some(text) = w.stories.line(seed, "sched.travel", &f, year) {
            let e = news(w, Level::Nfl, Loudness::Small, Some(me), text);
            post(w, e);
        }
    }
}

/// The year's new international marketing rights, on a February day: your
/// club's in its own line, the rest in one.
fn marketing(w: &mut World, d: Date) {
    let year = d.year();
    let new: Vec<(String, String)> = IMR.iter().filter(|x| x.2 == year).map(|x| (x.0.clone(), x.1.clone())).collect();
    if new.is_empty() {
        return;
    }
    let mine = super::seat_team(w).and_then(|t| match t {
        TeamKey::Nfl(n) => Some(n),
        _ => None,
    });
    let seed = w.options.seed;
    let mut rest = Vec::new();
    for (club, country) in new {
        let t = w.nfl.iter().position(|x| x.name == club);
        if let (Some(t), Some(m)) = (t, mine)
            && t == m as usize
        {
            let me = TeamKey::Nfl(m);
            let f = team_fill(w, me, Fill::new()).set("country", country.clone());
            if let Some(text) = w.stories.line(seed, "sched.imr", &f, year) {
                let e = news(w, Level::Nfl, Loudness::Medium, Some(me), text);
                post(w, e);
            }
            continue;
        }
        if t.is_some() {
            rest.push(format!("the {club} ({country})"));
        }
    }
    if !rest.is_empty() {
        let text = format!(
            "The NFL hands out international marketing rights for {year}: {}. Each club can sell, play and hold events in its countries.",
            rest.join(", ")
        );
        let e = news(w, Level::Nfl, Loudness::Small, None, text);
        post(w, e);
    }
}

pub(crate) fn day(w: &mut World, d: Date) {
    if w.nfl.is_empty() {
        return;
    }
    if d.month() == 2 && d.day() == 20 {
        marketing(w, d);
    }
    let Some(TeamKey::Nfl(t)) = super::seat_team(w) else { return };
    let t = t as usize;
    if d == w.dates.schedule_release {
        release(w, t);
    } else if matches!(d.month(), 9..=12 | 1) {
        season_day(w, d, t);
    }
}
