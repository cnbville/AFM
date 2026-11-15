//! Terms on the season (live talks v3, lane N2; spec section 8.4): the
//! same `usage::Usage` term the promises use, judged on a whole season, for
//! the deals that pay or move on one. An incentive in a pro contract, an
//! NIL clause, a conditional pick, a staff contract's goal and a boss's
//! goal all name a term here; lanes N5b and N5c make them in talks.
//!
//! - **A season term** is a `Usage` with `season` set: a stat line number
//!   (yards, touchdowns, sacks, picks...), his starts or games, an honor
//!   (the Pro Bowl, All-Pro, All-America, an award), his team's wins, a
//!   postseason, a title or a rank (scoring or total offense or defense),
//!   or any play-log measure added up over the season (his snap share, his
//!   targets...).
//! - **Read from the world's record:** a pro's season (`ProSeason`), your
//!   own games (`World::seat_games`, box scores and plays), a program's
//!   seasons, the open season's team rows (`tallies.squads`), the honors
//!   book. Nothing is read that the world doesn't keep: a term it can't
//!   read says nothing (`None`), and its deal lapses.
//! - **Outcomes are allowed here:** an incentive or a clause pays on what
//!   he does (real contracts do), unlike a promise, which holds you only to
//!   what you control.
//! - **Likely or not:** a term he met last season is likely to be earned
//!   (the NFL's rule for incentives); `likely` gives the chance a side
//!   weighs when it values a deal.
//!
//! Every size and pick here is Claude's (proposed, not confirmed).

use super::Team;
use super::usage::{Count, Usage, What};
use crate::calendar::Date;
use crate::game::stats::StatLine;
use crate::world::World;

/// A season stat line number a term can name: its key, its words, and its
/// value from a line.
pub struct Line {
    pub key: &'static str,
    pub words: &'static str,
    pub get: fn(&StatLine) -> f64,
    /// A rate (per attempt) rather than a count.
    pub rate: bool,
}

/// The stat line numbers (index + 1 is a term's `kind`).
pub const LINES: &[Line] = &[
    Line { key: "pass_yards", words: "passing yards", get: |l| l.pass_yds, rate: false },
    Line { key: "pass_tds", words: "touchdown passes", get: |l| l.pass_td as f64, rate: false },
    Line { key: "completions", words: "completions", get: |l| l.cmp as f64, rate: false },
    Line {
        key: "completion_pct",
        words: "completion percentage",
        get: |l| if l.att == 0 { 0.0 } else { l.cmp as f64 / l.att as f64 },
        rate: true,
    },
    Line { key: "rush_yards", words: "rushing yards", get: |l| l.rush_yds, rate: false },
    Line { key: "rush_tds", words: "rushing touchdowns", get: |l| l.rush_td as f64, rate: false },
    Line { key: "catches", words: "catches", get: |l| l.rec as f64, rate: false },
    Line { key: "rec_yards", words: "receiving yards", get: |l| l.rec_yds, rate: false },
    Line { key: "rec_tds", words: "touchdown catches", get: |l| l.rec_td as f64, rate: false },
    Line { key: "total_tds", words: "touchdowns", get: |l| (l.rush_td + l.rec_td) as f64, rate: false },
    Line { key: "scrimmage_yards", words: "yards from scrimmage", get: |l| l.rush_yds + l.rec_yds, rate: false },
    Line { key: "sacks", words: "sacks", get: |l| l.sacks, rate: false },
    Line { key: "tackles", words: "tackles", get: |l| (l.tackles + l.assists) as f64, rate: false },
    Line { key: "tfl", words: "tackles for loss", get: |l| l.tfl, rate: false },
    Line { key: "pressures", words: "pressures", get: |l| l.pressures as f64, rate: false },
    Line { key: "picks", words: "interceptions", get: |l| l.ints as f64, rate: false },
    Line { key: "pbus", words: "passes broken up", get: |l| l.pbus as f64, rate: false },
    Line { key: "forced_fumbles", words: "forced fumbles", get: |l| l.forced_fumbles as f64, rate: false },
    Line { key: "sacks_allowed", words: "sacks allowed", get: |l| l.sacks_allowed, rate: false },
    Line { key: "fg_made", words: "field goals made", get: |l| l.fg_made.iter().sum::<u32>() as f64, rate: false },
    Line {
        key: "fg_pct",
        words: "field goal percentage",
        get: |l| {
            let att: u32 = l.fg_att.iter().sum();
            if att == 0 { 0.0 } else { l.fg_made.iter().sum::<u32>() as f64 / att as f64 }
        },
        rate: true,
    },
    Line {
        key: "punt_net",
        words: "net punting average",
        get: |l| if l.punts == 0 { 0.0 } else { l.punt_net / l.punts as f64 },
        rate: true,
    },
    Line { key: "return_yards", words: "kick return yards", get: |l| l.kr_yds, rate: false },
];

/// The honors (index + 1 is the kind): the Pro Bowl, All-Pro (first or
/// second team), first-team All-Pro, first-team All-America, a named award.
pub const HONORS: &[(&str, &str)] = &[
    ("pro_bowl", "the Pro Bowl"),
    ("all_pro", "All-Pro"),
    ("all_pro_first", "first-team All-Pro"),
    ("all_america", "first-team All-America"),
    ("award", "a national award"),
];

/// The ranks (index + 1 is the kind), in his team's division.
pub const RANKS: &[(&str, &str)] = &[
    ("scoring_offense", "in scoring offense"),
    ("scoring_defense", "in scoring defense"),
    ("total_offense", "in total offense"),
    ("total_defense", "in total defense"),
];

/// The season-only keys a reader can name, with what they count.
pub const SEASON_KEYS: &[(&str, What)] = &[
    ("starts", What::Starts),
    ("games", What::Games),
    ("wins", What::Wins),
    ("postseason", What::Postseason),
    ("title", What::Title),
];

/// A season term from a reader's labels (lane N2): a stat line key, an
/// honor, a rank, a season key or any play-log measure (`usage::read`'s
/// keys, read over the season), with the number you said and at most.
/// `pos` is the man's position for his own terms, None for a team's.
pub fn read(pos: Option<crate::positions::Position>, key: &str, number: Option<f32>, most: bool) -> Option<Usage> {
    let base = Usage { season: true, most, ..Usage::default() };
    if let Some(k) = LINES.iter().position(|l| l.key == key) {
        pos?;
        let rate = LINES[k].rate;
        let mut n = number?;
        if rate && n > 1.0 {
            n /= 100.0;
        }
        return Some(Usage { what: What::Stat, kind: k as u8 + 1, n: n.max(0.0), share: rate, ..base });
    }
    if let Some(k) = HONORS.iter().position(|h| h.0 == key) {
        pos?;
        return Some(Usage { what: What::Honor, kind: k as u8 + 1, n: 1.0, most: false, ..base });
    }
    if let Some(k) = RANKS.iter().position(|r| r.0 == key) {
        let n = number.unwrap_or(25.0).round().clamp(1.0, 130.0);
        return Some(Usage { what: What::Rank, kind: k as u8 + 1, n, most: true, ..base });
    }
    if let Some(&(_, what)) = SEASON_KEYS.iter().find(|x| x.0 == key) {
        let player = matches!(what, What::Starts | What::Games);
        if player != pos.is_some() {
            return None;
        }
        let n = match what {
            What::Postseason | What::Title => 1.0,
            _ => number?.max(0.0),
        };
        return Some(Usage { what, n, ..base });
    }
    // A play-log measure over the season.
    let u = super::usage::read(pos?, key, number, None, most, "any", 0, 0, 0, 0, 0)?;
    if u.shown_only() || u.never() {
        return None;
    }
    Some(Usage { season: true, ..u })
}

/// Whose season a term reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Who {
    /// A player, by id.
    Player(u32),
    /// A team (a staff goal, a boss goal).
    Team(Team),
}

/// The football season a day falls in: January and February belong to the
/// season before (bowls, the playoffs, the Super Bowl).
pub fn season_of(d: Date) -> i32 {
    if d.month() <= 2 { d.year() - 1 } else { d.year() }
}

/// His season from your own games: his line summed, games played, games
/// started (half the most snaps on his side), and the play-log games with
/// his side, for a measure read off the plays. None when he played none.
/// His summed line, starts, games and the (game, side) pairs he played.
type Yours = (StatLine, u32, u32, Vec<(usize, usize)>);

fn yours(w: &World, id: u32, year: i32) -> Option<Yours> {
    let mut sum = StatLine::default();
    let (mut games, mut starts) = (0u32, 0u32);
    let mut plays = Vec::new();
    for (k, g) in w.seat_games.iter().enumerate().filter(|(_, g)| season_of(g.date) == year) {
        let Some(side) = (0..2).find(|&s| g.stats.players[s].contains_key(&id)) else { continue };
        let l = &g.stats.players[side][&id];
        let off = l.snaps_off >= l.snaps_def;
        let snaps = |x: &StatLine| if off { x.snaps_off } else { x.snaps_def };
        let most = g.stats.players[side].values().map(snaps).max().unwrap_or(0);
        games += 1;
        if most > 0 && snaps(l) as f64 >= super::seen::START_SHARE * most as f64 {
            starts += 1;
        }
        sum.add(l);
        if !g.stats.plays.is_empty() {
            plays.push((k, side));
        }
    }
    (games > 0).then_some((sum, games, starts, plays))
}

/// A pro's season, regular season only (as incentives count it).
fn pro(w: &World, id: u32, year: i32) -> Option<&crate::world::pro::ProSeason> {
    let i = super::person(w, id)?;
    w.people[i].pro.as_ref()?.seasons.iter().find(|s| s.year == year)
}

/// A pro's snap share on his side: his snaps over the most any teammate
/// played that season (a team's center or quarterback is on for all of
/// them).
fn pro_share(w: &World, s: &crate::world::pro::ProSeason, year: i32) -> Option<f64> {
    let t = s.team?;
    let k = if s.snaps[0] >= s.snaps[1] { 0 } else { 1 };
    let most = w
        .people
        .iter()
        .filter_map(|p| p.pro.as_ref())
        .filter_map(|c| c.seasons.iter().find(|x| x.year == year && x.team == Some(t)))
        .map(|x| x.snaps[k])
        .max()?;
    (most > 0).then(|| s.snaps[k] as f64 / most as f64)
}

/// What the season showed: a count, a share or a rank. None when the world
/// keeps nothing it can read it from.
pub fn value(w: &World, who: Who, u: &Usage, year: i32) -> Option<f64> {
    match who {
        Who::Team(team) => team_value(w, team, u, year),
        Who::Player(id) => player_value(w, id, u, year),
    }
}

fn player_value(w: &World, id: u32, u: &Usage, year: i32) -> Option<f64> {
    let p = pro(w, id, year);
    let mine = || yours(w, id, year);
    match u.what {
        What::Stat => {
            let line = LINES.get(u.kind.checked_sub(1)? as usize)?;
            let l = match p {
                Some(s) => s.stats.clone(),
                None => mine()?.0,
            };
            Some((line.get)(&l))
        }
        What::Starts => p.map(|s| s.starts as f64).or_else(|| mine().map(|m| m.2 as f64)),
        What::Games => p.map(|s| s.games as f64).or_else(|| mine().map(|m| m.1 as f64)),
        What::Honor => {
            let honors = w.stats.honors.get(&id);
            Some(match u.kind {
                1 => p.is_some_and(|s| s.pro_bowl),
                2 => p.is_some_and(|s| s.all_pro > 0),
                3 => p.is_some_and(|s| s.all_pro == 1),
                4 => honors.is_some_and(|h| h.all_america.iter().any(|x| x.0 == year)),
                _ => {
                    p.is_some_and(|s| !s.awards.is_empty())
                        || honors.is_some_and(|h| h.awards.iter().any(|x| x.0 == year))
                }
            } as u8 as f64)
        }
        What::Wins | What::Postseason | What::Title | What::Rank => {
            let i = super::person(w, id)?;
            let team = team_of(w, i, year)?;
            team_value(w, team, u, year)
        }
        _ => {
            // A play-log measure over the season: your games' plays, pooled;
            // a pro on another club, from his season's line.
            if let Some((_, _, _, games)) = mine().filter(|m| !m.3.is_empty()) {
                let mut c = Count::default();
                for (k, side) in games {
                    let x = u.count(&w.seat_games[k].stats.plays, side, id);
                    c.n += x.n;
                    c.of += x.of;
                }
                return if u.share { (c.of > 0.0).then(|| c.n / c.of) } else { Some(c.n) };
            }
            let s = p?;
            let l = &s.stats;
            match u.what {
                What::Snaps if u.share => pro_share(w, s, year),
                What::Carries => Some(l.rush as f64),
                What::Targets => Some(l.targets as f64),
                What::Touches => Some((l.rush + l.targets) as f64),
                What::Throws => Some(l.att as f64),
                What::FieldGoals => Some(l.fg_att.iter().sum::<u32>() as f64),
                What::Returns => Some(l.kr as f64),
                _ => None,
            }
        }
    }
}

/// The team a man played for that season (a pro's club, else his
/// program or school today).
fn team_of(w: &World, i: usize, year: i32) -> Option<Team> {
    let p = &w.people[i];
    if let Some(t) = p.pro.as_ref().and_then(|c| c.seasons.iter().find(|s| s.year == year)).and_then(|s| s.team) {
        return Some(Team::Nfl(t));
    }
    super::claims::team_of(w, i)
}

fn team_value(w: &World, team: Team, u: &Usage, year: i32) -> Option<f64> {
    use super::boss::{GoalKind, goal_met};
    match u.what {
        What::Wins => super::facts::seasons(w, team, 12)
            .into_iter()
            .find(|s| s.year == year)
            .map(|s| s.wins as f64)
            .or_else(|| open_row(w, team, year).map(|r| r.wins as f64)),
        What::Postseason => Some(goal_met(w, team, GoalKind::Postseason, year) as u8 as f64),
        What::Title => Some(goal_met(w, team, GoalKind::Title, year) as u8 as f64),
        What::Rank => rank(w, team, u.kind, year).map(|r| r as f64),
        _ => None,
    }
}

/// The open season's row for a team (its games so far), when it is this
/// season.
fn open_row(w: &World, team: Team, year: i32) -> Option<&crate::world::tally::squads::Season> {
    if w.year != year {
        return None;
    }
    let (level, id) = match team {
        Team::College(p) => (crate::world::tally::Level::College, p as u32),
        Team::Nfl(t) => (crate::world::tally::Level::Nfl, t as u32),
        Team::High(h) => (crate::world::tally::Level::Hs, h as u32),
        Team::Other(_) => return None,
    };
    w.tallies.squads.seasons.get(&crate::world::tally::squads::key(level, id)).filter(|s| s.games > 0)
}

/// A team's rank in its division that season (1 best): points scored or
/// allowed a game, yards or yards allowed a game. A program's seasons keep
/// their points and engine totals; the other levels read the open
/// season's team rows. None when neither has it.
pub fn rank(w: &World, team: Team, kind: u8, year: i32) -> Option<u32> {
    // (points for, against, yards, yards allowed) a game.
    let per = |pf: f64, pa: f64, y: f64, ya: f64, g: f64| [pf / g, pa / g, y / g, ya / g];
    let rows: Vec<(Team, [f64; 4])> = match team {
        Team::College(me) => {
            let div = w.programs[me].seasons.iter().find(|s| s.year == year).and_then(|s| s.division)?;
            w.programs
                .iter()
                .enumerate()
                .filter_map(|(k, p)| {
                    let s = p.seasons.iter().find(|s| s.year == year && s.division == Some(div))?;
                    let g = (s.wins + s.losses) as f64;
                    let t = &s.totals;
                    let tg = (t.games as f64).max(1.0);
                    (g > 0.0).then(|| {
                        let mut v = per(s.points[0] as f64, s.points[1] as f64, 0.0, 0.0, g);
                        v[2] = t.yards / tg;
                        v[3] = t.yards_allowed / tg;
                        (Team::College(k), v)
                    })
                })
                .collect()
        }
        Team::Other(_) => return None,
        _ => {
            if w.year != year {
                return None;
            }
            let lv = match team {
                Team::Nfl(_) => crate::world::tally::Level::Nfl,
                _ => crate::world::tally::Level::Hs,
            } as u8;
            w.tallies
                .squads
                .seasons
                .values()
                .filter(|s| s.level == lv && s.games > 0)
                .map(|s| {
                    let t = match team {
                        Team::Nfl(_) => Team::Nfl(s.team as usize),
                        _ => Team::High(s.team as usize),
                    };
                    let g = s.games as f64;
                    (
                        t,
                        per(
                            s.points_for as f64,
                            s.points_against as f64,
                            s.box_of(0, "yards"),
                            s.box_of(1, "yards"),
                            g,
                        ),
                    )
                })
                .collect()
        }
    };
    let k = kind.checked_sub(1)? as usize;
    if k > 3 {
        return None;
    }
    let mine = rows.iter().find(|r| r.0 == team)?.1[k];
    // Offense: more is better; defense (allowed): less is better.
    let better = |v: f64| if k.is_multiple_of(2) { v > mine + 1e-9 } else { v < mine - 1e-9 };
    Some(1 + rows.iter().filter(|r| better(r.1[k])).count() as u32)
}

/// Whether the season met the term: Some(met), None when it can't be read.
pub fn met(w: &World, who: Who, u: &Usage, year: i32) -> Option<bool> {
    let v = value(w, who, u, year)?;
    let bar = u.n as f64;
    Some(if u.most { v <= bar + 1e-9 } else { v + 1e-9 >= bar })
}

/// The chance a side weighs that the term is met this season (0.05-0.95):
/// met last season, likely (the NFL's "likely to be earned"); otherwise
/// by how close last season came. With no last season to read, even.
pub fn likely(w: &World, who: Who, u: &Usage, year: i32) -> f64 {
    let Some(v) = value(w, who, u, year - 1) else { return 0.5 };
    let bar = u.n as f64;
    if bar <= 0.0 {
        return if u.most { 0.5 } else { 0.95 };
    }
    let r = if u.most { if v <= 0.0 { 2.0 } else { bar / v } } else { v / bar };
    (0.75 * r.powi(2)).clamp(0.05, 0.95)
}

/// Likely to be earned (the NFL's rule): met last season.
pub fn ltbe(w: &World, who: Who, u: &Usage, year: i32) -> bool {
    met(w, who, u, year - 1) == Some(true)
}

/// A season term in words.
pub fn words(u: &Usage) -> String {
    let n = |x: f32| if (x - x.round()).abs() < 0.05 { format!("{}", x.round() as i32) } else { format!("{x:.1}") };
    let how = if u.most { "at most" } else { "at least" };
    match u.what {
        What::Stat => {
            let Some(l) = u.kind.checked_sub(1).and_then(|k| LINES.get(k as usize)) else { return String::new() };
            if l.rate {
                format!("{how} {}% {} this season", (u.n * 100.0).round() as i32, l.words)
            } else {
                format!("{how} {} {} this season", n(u.n), l.words)
            }
        }
        What::Starts => format!("{how} {} starts this season", n(u.n)),
        What::Games => format!("{how} {} games played this season", n(u.n)),
        What::Honor => {
            let h = u.kind.checked_sub(1).and_then(|k| HONORS.get(k as usize)).map_or("an honor", |h| h.1);
            format!("makes {h} this season")
        }
        What::Wins => format!("{how} {} wins this season", n(u.n)),
        What::Postseason => "the postseason this season".into(),
        What::Title => "a title this season".into(),
        What::Rank => {
            let r = u.kind.checked_sub(1).and_then(|k| RANKS.get(k as usize)).map_or("", |r| r.1);
            format!("top {} {r} this season", n(u.n))
        }
        _ => format!("{} over the season", u.words()),
    }
}

// ---------------------------------------------------------------------------
// Clauses: the deals that pay on a season term, settled after the season.

/// What a clause belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum On {
    /// An NIL deal's clause (your collective pays it to your player).
    Nil,
    /// A staff contract's goal (a bonus for your coach).
    Staff,
}

/// A clause: a term on one season and what it pays when met.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Clause {
    pub on: On,
    pub who: Who,
    /// The coach it pays (a staff goal), by index.
    #[serde(default)]
    pub coach: Option<usize>,
    pub term: Usage,
    pub season: i32,
    /// Dollars when met.
    pub pays: f64,
    pub made: Date,
    /// Settled: met or not (None while open, or when nothing could be read).
    #[serde(default)]
    pub met: Option<bool>,
    #[serde(default)]
    pub settled: bool,
}

/// Adds a clause (lanes N5b and N5c call this when a deal is agreed).
pub fn add(w: &mut World, c: Clause) {
    w.folk.clauses.push(c);
}

/// The clauses settle on Super Bowl day, every game of their season
/// played and the season's team rows still open: met, the money is paid
/// and noted; not met, noted. A clause the world can't read lapses.
pub(crate) fn day(w: &mut World, d: Date) {
    if w.folk.clauses.is_empty() || d < w.dates.super_bowl {
        return;
    }
    let year = w.year;
    for k in 0..w.folk.clauses.len() {
        let c = w.folk.clauses[k].clone();
        if c.settled || c.season > year {
            continue;
        }
        let met = met(w, c.who, &c.term, c.season);
        w.folk.clauses[k].met = met;
        w.folk.clauses[k].settled = true;
        let who = match (c.who, c.coach) {
            (_, Some(ci)) if ci < w.coaches.len() => w.coaches[ci].name.clone(),
            (Who::Player(id), _) => super::person(w, id).map_or(String::new(), |i| w.people[i].name.clone()),
            (Who::Team(t), _) => t.name(w),
        };
        let what = match c.on {
            On::Nil => "NIL clause",
            On::Staff => "contract goal",
        };
        let text = match met {
            Some(true) => {
                if c.on == On::Nil
                    && let Who::Player(id) = c.who
                    && let Some(i) = super::person(w, id)
                {
                    w.people[i].nil_income += c.pays;
                }
                format!("{who} met the {what} ({}): it pays.", words(&c.term))
            }
            Some(false) => format!("{who} missed the {what} ({}).", words(&c.term)),
            None => format!("The {what} for {who} ({}) couldn't be judged, so it lapses.", words(&c.term)),
        };
        super::note(w, "talk", "Your deals".into(), text, None);
    }
}
