//! A coach's season review (Cain 2026-11-17, thread "Give coaches real
//! depth when asked why", items 1-6, and "i want coaches to be aware of
//! team stats and player stats too"): a coach of yours asked how the year
//! went, or why it went wrong, walks you through it from what the world
//! really recorded, never "in general terms".
//!
//! Everything is read from the season tallies (`world::tally`): the team's
//! squad row (both sides' box scores), its games (scores by quarter and the
//! win chance before each snap) and its players' depth counts. These hold
//! every season the world played, so a coach who was here before you took
//! the job knows last season (the games kept for your seat, `seat_games`,
//! start the day you arrive).
//!
//! What goes in, each a point with its weight:
//! - his side's numbers ranked in the conference (the NFL: the league):
//!   the worst and the best stand out;
//! - the games that got away: a lead into the fourth or a game in hand
//!   lost, a one-score loss, a blowout;
//! - his side's starters at their own spot, ranked against the
//!   conference's starters there: who struggled, who stood out;
//! - starters who missed games;
//! - in a season under way, his side's last three games against the rest.
//!
//! He sees as many points as his football knowledge reaches (`reach`), the
//! heaviest first. A coach hired after the season says so: it isn't his to
//! own, but he has studied its tape. Every size here is Claude's, proposed,
//! not confirmed.

use std::collections::{HashMap, HashSet};

use super::makeup::Makeup;
use crate::positions::Position;
use crate::world::World;
use crate::world::coach::{Group, Role};
use crate::world::folk::{Team, facts};
use crate::world::tally::depth::{self, Dc};
use crate::world::tally::squads::{self, Season};
use crate::world::tally::{Level, file, games::GameRow, players};

/// Games a season needs before it says anything (`claims::rooms`).
pub const MIN_GAMES: u32 = 3;
/// Points he gives on the first ask (a push gets the rest).
pub const FIRST_POINTS: usize = 3;
/// The fewest and most points his eye reaches.
pub const REACH_MIN: usize = 2;
pub const REACH_MAX: usize = 6;
/// A number stands out when its place in the conference is this far from
/// the middle (0 to 0.5 on a scale where 0 is the best and 1 the worst).
pub const STANDS_OUT: f64 = 0.2;
/// Fewest peers for a rank to mean anything.
pub const MIN_PEERS: usize = 6;
/// A starter's place among his spot's starters that reads as struggling
/// (worst third) or standing out (best sixth).
pub const STRUGGLED: f64 = 0.67;
pub const STOOD_OUT: f64 = 0.17;
/// Games a starter must miss to count.
pub const MISSED: u32 = 2;
/// A loss by this much or less is one score; by this much or more a blowout.
pub const ONE_SCORE: i32 = 8;
pub const BLOWOUT: i32 = 24;
/// A win chance this high at any snap makes a loss one that got away.
pub const IN_HAND: u8 = 80;
/// Games of a season under way before the last three say anything.
pub const TREND_GAMES: usize = 6;
/// Points a game his side's last three must move to count as a trend.
pub const TREND_POINTS: f64 = 5.0;

/// The words a review's first line starts with (the voice's rules name
/// it: a review takes more sentences).
pub const HEADER: &str = "Season review";
/// The words its last line starts with.
pub const FIX: &str = "What he'd fix first";

/// A coach's season review: the header, then the points he sees, heaviest
/// first, then what he'd fix first when something stood out badly.
#[derive(Clone, Debug, PartialEq)]
pub struct Review {
    pub season: i32,
    /// He coached the season under review.
    pub coached: bool,
    pub header: String,
    pub points: Vec<String>,
    pub fix: Option<String>,
}

impl Review {
    /// Every line, in order.
    pub fn lines(&self) -> Vec<String> {
        let mut v = vec![self.header.clone()];
        v.extend(self.points.iter().cloned());
        v.extend(self.fix.iter().cloned());
        v
    }
}

/// What the review is of: a coordinator's side or a position coach's room.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Of {
    Offense,
    Defense,
    Room(Group),
}

impl Of {
    fn from_role(role: Role) -> Option<Of> {
        match role {
            Role::OffensiveCoordinator => Some(Of::Offense),
            Role::DefensiveCoordinator => Some(Of::Defense),
            Role::PositionCoach(g) if g != Group::Specialists => Some(Of::Room(g)),
            _ => None,
        }
    }

    fn offense(self) -> bool {
        match self {
            Of::Offense => true,
            Of::Defense => false,
            Of::Room(g) => g.offense(),
        }
    }

    /// The side's starters: 0 offense, 1 defense.
    fn unit(self) -> usize {
        if self.offense() { 0 } else { 1 }
    }

    fn holds(self, pos: Position) -> bool {
        match self {
            Of::Room(g) => Group::of(pos) == g,
            _ => !pos.is_specialist() && Group::of(pos).offense() == self.offense(),
        }
    }

    fn words(self) -> &'static str {
        match self {
            Of::Offense => "the offense",
            Of::Defense => "the defense",
            Of::Room(g) => match g {
                Group::QB => "the quarterbacks",
                Group::RB => "the running backs",
                Group::Receivers => "the receivers",
                Group::OL => "the offensive line",
                Group::DL => "the defensive line",
                Group::LB => "the linebackers",
                Group::DB => "the secondary",
                Group::Specialists => "the specialists",
            },
        }
    }
}

/// The tally level of a team, and its id there.
pub fn level_of(team: Team) -> Option<(Level, u32)> {
    match team {
        Team::College(p) => Some((Level::College, p as u32)),
        Team::Nfl(t) => Some((Level::Nfl, t as u32)),
        Team::High(h) => Some((Level::Hs, h as u32)),
        Team::Other(_) => None,
    }
}

/// The team's latest season with games enough to say something: the one
/// under way when it has `MIN_GAMES`, else the last one. Read from the
/// world's tallies, so a season played before you took the job counts.
pub fn season_of(w: &World, team: Team) -> Option<(i32, Season)> {
    let (level, id) = level_of(team)?;
    let open = w.tally_open_season();
    [open, open - 1]
        .into_iter()
        .find_map(|y| w.tally_squad(level, y, id).filter(|s| s.games >= MIN_GAMES).map(|s| (y, s)))
}

/// The latest season the team has played any game of: the one under way
/// once it has a game, else the last finished one.
pub fn latest_played(w: &World, team: Team) -> i32 {
    let open = w.tally_open_season();
    let under_way = level_of(team).and_then(|(l, id)| w.tally_squad(l, open, id)).is_some_and(|s| s.games > 0)
        || facts::record_now(w, team) != (0, 0);
    if under_way {
        return open;
    }
    facts::seasons(w, team, 1).last().map_or(open - 1, |s| s.year)
}

/// How many points his eye reaches, by his football knowledge (players'
/// football talk reads the same way, `football::reach`).
pub fn reach(m: &Makeup) -> usize {
    (2.0 + (m.iq - 4.0).max(0.0) / 3.0).clamp(REACH_MIN as f64, REACH_MAX as f64) as usize
}

/// The coach's review, or None for a man with no side or room (a head
/// coach, a special teams coach) or no season on file.
pub fn review(w: &World, c: usize, m: &Makeup) -> Option<Review> {
    let job = w.coaches.get(c)?.job?;
    let of = Of::from_role(job.role)?;
    let team = Team::from_employer(job.employer);
    let (season, mine) = season_of(w, team)?;
    let (wins, losses) = record_of(w, team, season, &mine);
    let coached = job.first_season() <= season;
    let under_way = in_season(w, team, season);
    let peers = peers(w, team, season);
    let mut pts: Vec<(f64, String)> = Vec::new();
    let mut worst: Option<(f64, &'static str)> = None;
    for mt in metrics(of) {
        let Some(v) = (mt.val)(&mine) else { continue };
        let line = format!("{} {}", of_side(of), (mt.words)(v));
        match &peers {
            Some((label, rows)) => {
                let vals: Vec<f64> = rows.iter().filter_map(|r| (mt.val)(r)).collect();
                let Some(s) = place(v, &vals, mt.high_good) else { continue };
                if (s - 0.5).abs() < STANDS_OUT {
                    continue;
                }
                let n = vals.len();
                let r = 1 + vals.iter().filter(|&&x| if mt.high_good { x > v + 1e-9 } else { x < v - 1e-9 }).count();
                pts.push(((s - 0.5).abs() * 2.0, format!("{line}, {} of {n} {label}", ordinal(r))));
                if s > 0.5 && worst.is_none_or(|x| s > x.0) {
                    worst = Some((s, mt.short));
                }
            }
            None => pts.push((0.3, line)),
        }
    }
    let rows = side_games(w, team, season, &mine);
    if !matches!(of, Of::Room(_)) {
        pts.extend(got_away(w, team, &rows));
    }
    if under_way {
        pts.extend(trend(team, &rows, of));
    }
    pts.extend(starters(w, team, season, &mine, of, peers.as_ref().map(|p| p.0.as_str())));
    pts.sort_by(|a, b| b.0.total_cmp(&a.0));
    pts.truncate(reach(m));
    let side = of.words();
    let header = if under_way {
        format!("{HEADER} so far, the {season} season: the team is {wins}-{losses}; he runs {side}")
    } else if coached {
        format!("{HEADER}, the {season} season: the team went {wins}-{losses}; he coached {side} all season")
    } else {
        format!(
            "{HEADER}, the {season} season: the team went {wins}-{losses} before he got here. He was hired after it, so it isn't his season to own, but he has been through its tape since he arrived"
        )
    };
    let fix = worst.map(|(_, short)| format!("{FIX}: {short}"));
    Some(Review { season, coached, header, points: pts.into_iter().map(|p| p.1).collect(), fix })
}

/// The team's win-loss line for the season in the review's header: the
/// same record the boss and the press read (`facts::seasons`, every game
/// of the season at any engine or stand-in level, ties and all), so the
/// coach and the AD never give two records for one season. The tally
/// holds only games the engine played (a program too thin to field a full
/// squad plays the stand-in), so it keeps the stats and is the record only
/// for a season still under way, which has no finished row yet.
fn record_of(w: &World, team: Team, season: i32, mine: &Season) -> (u32, u32) {
    facts::seasons(w, team, 3)
        .into_iter()
        .find(|s| s.year == season)
        .map_or((mine.wins, mine.losses), |s| (s.wins, s.losses))
}

/// The season is still being played (its finished row isn't in the
/// team's seasons yet): the review is of the year so far.
fn in_season(w: &World, team: Team, season: i32) -> bool {
    season == w.tally_open_season() && !facts::seasons(w, team, 3).iter().any(|s| s.year == season)
}

fn of_side(of: Of) -> &'static str {
    match of {
        Of::Offense | Of::Room(Group::QB | Group::RB | Group::Receivers | Group::OL) => "the offense",
        _ => "the defense",
    }
}

/// Its place among the values, 0 the best to 1 the worst. None with too
/// few to rank against.
fn place(v: f64, vals: &[f64], high_good: bool) -> Option<f64> {
    if vals.len() < MIN_PEERS {
        return None;
    }
    let better = vals.iter().filter(|&&x| if high_good { x > v + 1e-9 } else { x < v - 1e-9 }).count();
    Some(better as f64 / (vals.len() - 1).max(1) as f64)
}

pub fn ordinal(n: usize) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// The teams it is ranked against that season, with how to say it ("in
/// the Big 12", "in the league"). College: its conference that season
/// (an independent, its division); the NFL: the league; high school: none.
fn peers(w: &World, team: Team, season: i32) -> Option<(String, Vec<Season>)> {
    let (level, _) = level_of(team)?;
    let (label, ids): (String, Vec<u32>) = match team {
        Team::College(p) => {
            let conf_of = |k: usize| {
                let pr = &w.programs[k];
                pr.seasons
                    .iter()
                    .find(|s| s.year == season)
                    .and_then(|s| s.conference.clone())
                    .unwrap_or_else(|| pr.conference.clone())
            };
            let conf = conf_of(p);
            let same: Vec<u32> = (0..w.programs.len()).filter(|&k| conf_of(k) == conf).map(|k| k as u32).collect();
            if same.len() >= MIN_PEERS && !conf.to_lowercase().contains("independent") {
                (format!("in the {conf}"), same)
            } else {
                let div = w.programs[p].division;
                let all: Vec<u32> =
                    (0..w.programs.len()).filter(|&k| w.programs[k].division == div).map(|k| k as u32).collect();
                (format!("in {}", div.name()), all)
            }
        }
        Team::Nfl(_) => ("in the league".into(), (0..w.clubs.len() as u32).collect()),
        _ => return None,
    };
    let keys: Vec<u64> = ids.iter().map(|&id| squads::key(level, id)).collect();
    let mut rows = Vec::new();
    if season == w.tally_open_season() {
        rows.extend(keys.iter().filter_map(|k| w.tallies.squads.seasons.get(k).cloned()));
    } else {
        let _ = w.tally_get("squads", season, &keys, |_, r| {
            if let Some(s) = file::decode::<Season>(r) {
                rows.push(s)
            }
        });
    }
    rows.retain(|s| s.games >= MIN_GAMES);
    (rows.len() >= MIN_PEERS).then_some((label, rows))
}

struct Metric {
    val: fn(&Season) -> Option<f64>,
    high_good: bool,
    words: fn(f64) -> String,
    /// What he'd work on, in a few words.
    short: &'static str,
}

fn metric(val: fn(&Season) -> Option<f64>, high_good: bool, words: fn(f64) -> String, short: &'static str) -> Metric {
    Metric { val, high_good, words, short }
}

fn g(s: &Season) -> f64 {
    s.games.max(1) as f64
}

fn ratio(n: f64, d: f64, min: f64) -> Option<f64> {
    (d >= min).then(|| n / d)
}

/// The numbers he's judged on, by side and room.
fn metrics(of: Of) -> Vec<Metric> {
    let defense = |keep: &[&str]| -> Vec<Metric> {
        let all = vec![
            (
                "points",
                metric(
                    |s: &Season| Some(s.points_against as f64 / g(s)),
                    false,
                    |v| format!("gave up {v:.1} points a game"),
                    "keeping points off the board",
                ),
            ),
            (
                "yards",
                metric(
                    |s: &Season| Some(s.box_of(1, "yards") / g(s)),
                    false,
                    |v| format!("gave up {v:.0} yards a game"),
                    "the yards we give up",
                ),
            ),
            (
                "run",
                metric(
                    |s: &Season| ratio(s.box_of(1, "rush_yards"), s.box_of(1, "carries"), 50.0),
                    false,
                    |v| format!("gave up {v:.1} yards a carry"),
                    "stopping the run",
                ),
            ),
            (
                "pass",
                metric(
                    |s: &Season| ratio(s.box_of(1, "pass_yards"), s.box_of(1, "att"), 50.0),
                    false,
                    |v| format!("gave up {v:.1} yards a pass attempt"),
                    "the pass defense",
                ),
            ),
            (
                "third",
                metric(
                    |s: &Season| ratio(s.box_of(1, "third_conv"), s.box_of(1, "third_att"), 20.0).map(|x| 100.0 * x),
                    false,
                    |v| format!("let opponents convert {v:.0}% of their third downs"),
                    "getting off the field on third down",
                ),
            ),
            (
                "red",
                metric(
                    |s: &Season| {
                        ratio(s.box_of(1, "red_zone_tds"), s.box_of(1, "red_zone_trips"), 8.0).map(|x| 100.0 * x)
                    },
                    false,
                    |v| format!("gave up a touchdown on {v:.0}% of red zone trips"),
                    "red zone defense",
                ),
            ),
            (
                "big",
                metric(
                    |s: &Season| Some(s.box_of(1, "explosive") / g(s)),
                    false,
                    |v| format!("gave up {v:.1} explosive plays a game"),
                    "the big plays we give up",
                ),
            ),
            (
                "takeaways",
                metric(
                    |s: &Season| Some(s.box_of(1, "turnovers") / g(s)),
                    true,
                    |v| format!("took the ball away {v:.1} times a game"),
                    "taking the ball away",
                ),
            ),
            (
                "sacks",
                metric(
                    |s: &Season| Some(s.box_of(1, "sacks") / g(s)),
                    true,
                    |v| format!("got {v:.1} sacks a game"),
                    "the pass rush",
                ),
            ),
        ];
        all.into_iter().filter(|x| keep.is_empty() || keep.contains(&x.0)).map(|x| x.1).collect()
    };
    let offense = |keep: &[&str]| -> Vec<Metric> {
        let all = vec![
            (
                "points",
                metric(
                    |s: &Season| Some(s.points_for as f64 / g(s)),
                    true,
                    |v| format!("scored {v:.1} points a game"),
                    "scoring points",
                ),
            ),
            (
                "yards",
                metric(
                    |s: &Season| Some(s.box_of(0, "yards") / g(s)),
                    true,
                    |v| format!("gained {v:.0} yards a game"),
                    "moving the ball",
                ),
            ),
            (
                "run",
                metric(
                    |s: &Season| ratio(s.box_of(0, "rush_yards"), s.box_of(0, "carries"), 50.0),
                    true,
                    |v| format!("ran for {v:.1} yards a carry"),
                    "the run game",
                ),
            ),
            (
                "pass",
                metric(
                    |s: &Season| ratio(s.box_of(0, "pass_yards"), s.box_of(0, "att"), 50.0),
                    true,
                    |v| format!("threw for {v:.1} yards a pass attempt"),
                    "the passing game",
                ),
            ),
            (
                "third",
                metric(
                    |s: &Season| ratio(s.box_of(0, "third_conv"), s.box_of(0, "third_att"), 20.0).map(|x| 100.0 * x),
                    true,
                    |v| format!("converted {v:.0}% of its third downs"),
                    "third down",
                ),
            ),
            (
                "red",
                metric(
                    |s: &Season| {
                        ratio(s.box_of(0, "red_zone_tds"), s.box_of(0, "red_zone_trips"), 8.0).map(|x| 100.0 * x)
                    },
                    true,
                    |v| format!("scored a touchdown on {v:.0}% of red zone trips"),
                    "finishing in the red zone",
                ),
            ),
            (
                "big",
                metric(
                    |s: &Season| Some(s.box_of(0, "explosive") / g(s)),
                    true,
                    |v| format!("made {v:.1} explosive plays a game"),
                    "making big plays",
                ),
            ),
            (
                "giveaways",
                metric(
                    |s: &Season| Some(s.box_of(0, "turnovers") / g(s)),
                    false,
                    |v| format!("turned it over {v:.1} times a game"),
                    "protecting the ball",
                ),
            ),
            (
                "sacks",
                metric(
                    |s: &Season| Some(s.box_of(0, "sacks") / g(s)),
                    false,
                    |v| format!("gave up {v:.1} sacks a game"),
                    "pass protection",
                ),
            ),
        ];
        all.into_iter().filter(|x| keep.is_empty() || keep.contains(&x.0)).map(|x| x.1).collect()
    };
    match of {
        Of::Offense => offense(&[]),
        Of::Defense => defense(&[]),
        Of::Room(Group::QB) => offense(&["pass", "third", "giveaways"]),
        Of::Room(Group::RB) => offense(&["run", "big", "giveaways"]),
        Of::Room(Group::Receivers) => offense(&["pass", "big", "third"]),
        Of::Room(Group::OL) => offense(&["run", "sacks"]),
        Of::Room(Group::DL) => defense(&["run", "sacks"]),
        Of::Room(Group::LB) => defense(&["run", "third"]),
        Of::Room(Group::DB) => defense(&["pass", "big", "takeaways"]),
        Of::Room(Group::Specialists) => Vec::new(),
    }
}

/// The team's starters' season rows (the side's, or the room's), by id.
fn rows_of(w: &World, level: Level, season: i32, ids: &[u32]) -> HashMap<u32, players::Season> {
    let mut out = HashMap::new();
    if season == w.tally_open_season() {
        for &id in ids {
            if let Some(s) = w.tallies.players.get(level, id) {
                out.insert(id, s.into_owned());
            }
        }
        return out;
    }
    let keys: Vec<u64> = ids.iter().map(|&id| players::key(level, id)).collect();
    let _ = w.tally_get("players", season, &keys, |_, r| {
        if let Some(s) = file::decode::<players::Season>(r) {
            out.insert(s.id, s);
        }
    });
    out
}

/// The team's games that season, oldest first: the keys its starters
/// dressed for, read from the season's games file.
fn side_games(w: &World, team: Team, season: i32, mine: &Season) -> Vec<GameRow> {
    let Some((level, _)) = level_of(team) else { return Vec::new() };
    if level == Level::Hs {
        return Vec::new();
    }
    let mut ids: Vec<(u32, u8)> = mine.starters.iter().map(|s| (s.0, s.1.max(s.2))).collect();
    ids.sort_by_key(|a| std::cmp::Reverse(a.1));
    ids.truncate(12);
    let ids: Vec<u32> = ids.into_iter().map(|x| x.0).collect();
    let rows = rows_of(w, level, season, &ids);
    let keys: Vec<u64> =
        rows.values().flat_map(|r| r.played.iter().copied()).collect::<HashSet<u64>>().into_iter().collect();
    let mut out = Vec::new();
    w.tally_games(season, Some(&keys), |_, g| out.push(g));
    out.retain(|g| side_in(team, g).is_some());
    out.sort_by_key(|g| g.date);
    out
}

/// Its side in a game, 0 home or 1 away.
fn side_in(team: Team, g: &GameRow) -> Option<usize> {
    let (level, id) = level_of(team)?;
    if g.level != level as u8 {
        return None;
    }
    g.teams.iter().position(|&t| t == id)
}

fn opponent(w: &World, team: Team, g: &GameRow, side: usize) -> String {
    let o = g.teams[1 - side] as usize;
    match team {
        Team::College(_) => Team::College(o).name(w),
        Team::Nfl(_) => Team::Nfl(o).name(w),
        Team::High(_) => Team::High(o).name(w),
        Team::Other(_) => String::new(),
    }
}

/// The games that got away: a lead into the fourth or a game in hand
/// lost, a one-score loss, a blowout. Three at most.
fn got_away(w: &World, team: Team, games: &[GameRow]) -> Vec<(f64, String)> {
    let mut v: Vec<(f64, String)> = Vec::new();
    for g in games {
        let Some(i) = side_in(team, g) else { continue };
        let (us, them) = (g.score[i], g.score[1 - i]);
        if us >= them {
            continue;
        }
        let opp = opponent(w, team, g, i);
        let three: [i32; 2] = g.quarters.iter().take(3).fold([0, 0], |a, q| [a[0] + q[0], a[1] + q[1]]);
        let led = g.quarters.len() >= 4 && three[i] > three[1 - i];
        let in_hand = g.wp.iter().any(|&p| if i == 0 { p >= IN_HAND } else { 100 - p.min(100) >= IN_HAND });
        let margin = them - us;
        if led {
            v.push((
                0.85,
                format!(
                    "lost {us}-{them} to {opp} after leading {}-{} going into the fourth quarter",
                    three[i],
                    three[1 - i]
                ),
            ));
        } else if in_hand {
            v.push((0.8, format!("lost {us}-{them} to {opp}, a game they had in hand")));
        } else if margin <= ONE_SCORE {
            v.push((0.6, format!("lost {us}-{them} to {opp}, a one-score game")));
        } else if margin >= BLOWOUT {
            v.push((0.55, format!("lost {us}-{them} to {opp}")));
        }
    }
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    v.truncate(3);
    v
}

/// A season under way: his side's points over the last three games
/// against the games before.
fn trend(team: Team, games: &[GameRow], of: Of) -> Vec<(f64, String)> {
    if games.len() < TREND_GAMES {
        return Vec::new();
    }
    let pts = |g: &GameRow| -> Option<f64> {
        let i = side_in(team, g)?;
        Some(if of.offense() { g.score[i] } else { g.score[1 - i] } as f64)
    };
    let all: Vec<f64> = games.iter().filter_map(pts).collect();
    let (before, last) = all.split_at(all.len() - 3);
    let a = before.iter().sum::<f64>() / before.len().max(1) as f64;
    let b = last.iter().sum::<f64>() / 3.0;
    if (b - a).abs() < TREND_POINTS {
        return Vec::new();
    }
    let verb = if of.offense() { "scored" } else { "gave up" };
    let better = if of.offense() { b > a } else { b < a };
    vec![(
        0.65,
        format!(
            "over the last three games {} {verb} {b:.1} points a game, {} the {a:.1} before that",
            of_side(of),
            if better { "better than" } else { "worse than" }
        ),
    )]
}

/// A starter's one telling number at his spot: (value, more is better,
/// the number in words). None with too small a sample.
pub fn key_rate(d: &depth::Depth, pos: Position) -> Option<(f64, bool, String)> {
    let g = |c: Dc| depth::get(d, c as u16);
    match pos {
        Position::QB => {
            let att = g(Dc::Att);
            (att >= 60.0)
                .then(|| (g(Dc::Int) / att, false, format!("{} interceptions in {} attempts", g(Dc::Int), att)))
        }
        Position::RB => {
            let c = g(Dc::RAtt);
            (c >= 40.0)
                .then(|| (g(Dc::RMtf) / c, true, format!("{} missed tackles forced on {} carries", g(Dc::RMtf), c)))
        }
        Position::WR | Position::TE => {
            let t = g(Dc::Tgt);
            (t >= 20.0).then(|| (g(Dc::Drop) / t, false, format!("{} drops on {} targets", g(Dc::Drop), t)))
        }
        Position::OT | Position::IOL => {
            let pb = g(Dc::PbSnaps);
            (pb >= 100.0).then(|| {
                (
                    g(Dc::PrsAllowed) / pb,
                    false,
                    format!("{} pressures given up in {} pass-block snaps", g(Dc::PrsAllowed), pb),
                )
            })
        }
        Position::EDGE | Position::DT => {
            let pr = g(Dc::PrSnaps);
            (pr >= 60.0).then(|| (g(Dc::Prs) / pr, true, format!("{} pressures in {} pass rushes", g(Dc::Prs), pr)))
        }
        Position::LB | Position::S => {
            let a = g(Dc::Tkl) + g(Dc::Miss);
            (a >= 20.0)
                .then(|| (g(Dc::Miss) / a, false, format!("{} missed tackles to {} made", g(Dc::Miss), g(Dc::Tkl))))
        }
        Position::CB => {
            let t = g(Dc::TgtA);
            (t >= 15.0)
                .then(|| (g(Dc::CmpA) / t, false, format!("{} of {} targets against him caught", g(Dc::CmpA), t)))
        }
        _ => None,
    }
}

fn spot_words(pos: Position) -> &'static str {
    match pos {
        Position::QB => "quarterbacks",
        Position::RB => "running backs",
        Position::WR => "receivers",
        Position::TE => "tight ends",
        Position::OT => "tackles",
        Position::IOL => "interior linemen",
        Position::EDGE => "edge rushers",
        Position::DT => "defensive tackles",
        Position::LB => "linebackers",
        Position::CB => "corners",
        Position::S => "safeties",
        _ => "specialists",
    }
}

/// The conference's starters at a spot that season, with their rates.
fn spot_rates(w: &World, team: Team, season: i32, pos: Position) -> Vec<(u32, f64)> {
    let Some((level, _)) = level_of(team) else { return Vec::new() };
    let unit = if Group::of(pos).offense() { 0 } else { 1 };
    let Some((_, rows)) = peers(w, team, season) else { return Vec::new() };
    let ids: Vec<u32> = rows.iter().flat_map(|r| r.regulars(unit)).collect();
    rows_of(w, level, season, &ids)
        .into_values()
        .filter(|s| s.pos == Some(pos))
        .filter_map(|s| key_rate(&s.depth, pos).map(|r| (s.id, r.0)))
        .collect()
}

/// A starter's telling number and its place among his spot's starters
/// ("2037: 41 of 58 targets against him caught, the 3rd-worst rate of 28
/// starting corners in the Big 12"): for a coach of his asked about him
/// (Cain 2026-11-17: coaches know player stats too). None when he wasn't a
/// starter or there's too little to rank.
pub fn player_place(w: &World, i: usize) -> Option<String> {
    let p = w.people.get(i)?;
    let team = super::makeup::team_of(w, i)?;
    let (season, _) = season_of(w, team)?;
    let (level, _) = level_of(team)?;
    let id = p.player.id;
    let (_, team_id) = level_of(team)?;
    let row = rows_of(w, level, season, &[id]).remove(&id).filter(|r| r.team == team_id)?;
    let pos = row.pos?;
    let (v, high_good, said) = key_rate(&row.depth, pos)?;
    let rates = spot_rates(w, team, season, pos);
    let vals: Vec<f64> = rates.iter().map(|r| r.1).collect();
    let label = peers(w, team, season)?.0;
    Some(format!("{season}: {said}, {}", placed(v, &vals, high_good, pos, &label)?))
}

/// "the 3rd-worst rate of 28 starting corners in the Big 12", or the best.
fn placed(v: f64, vals: &[f64], high_good: bool, pos: Position, label: &str) -> Option<String> {
    let s = place(v, vals, high_good)?;
    let n = vals.len();
    let better = vals.iter().filter(|&&x| if high_good { x > v + 1e-9 } else { x < v - 1e-9 }).count();
    let words = if s > 0.5 {
        let worse = vals.iter().filter(|&&x| if high_good { x < v - 1e-9 } else { x > v + 1e-9 }).count();
        format!("the {}-worst rate", ordinal(worse + 1))
    } else {
        format!("the {}-best rate", ordinal(better + 1))
    };
    Some(format!("{words} of {n} starting {} {label}", spot_words(pos)))
}

/// His side's (or room's) starters at their own spot: who struggled and who
/// stood out against the conference's starters there (two and one at
/// most), and the starters who missed games (two at most).
fn starters(w: &World, team: Team, season: i32, mine: &Season, of: Of, label: Option<&str>) -> Vec<(f64, String)> {
    let Some((level, _)) = level_of(team) else { return Vec::new() };
    let unit = of.unit();
    let ids: Vec<u32> = mine.starters.iter().filter(|s| if unit == 0 { s.1 } else { s.2 } >= 2).map(|s| s.0).collect();
    let rows = rows_of(w, level, season, &ids);
    let roster: HashSet<usize> = team.roster(w).into_iter().collect();
    let name = |id: u32| -> Option<(String, bool)> {
        let i = crate::world::folk::person(w, id)?;
        Some((w.people[i].name.clone(), roster.contains(&i)))
    };
    let mut ours: Vec<&players::Season> = rows.values().filter(|r| r.pos.is_some_and(|p| of.holds(p))).collect();
    ours.sort_by_key(|r| r.id);
    let mut struggled = Vec::new();
    let mut stood = Vec::new();
    if let Some(label) = label {
        let mut by_spot: HashMap<Position, Vec<f64>> = HashMap::new();
        for r in &ours {
            let pos = r.pos.unwrap();
            if squads::regular(if unit == 0 { start_of(mine, r.id).0 } else { start_of(mine, r.id).1 }, mine.games)
                && !by_spot.contains_key(&pos)
            {
                by_spot.insert(pos, spot_rates(w, team, season, pos).into_iter().map(|x| x.1).collect());
            }
        }
        for r in &ours {
            let pos = r.pos.unwrap();
            let Some(vals) = by_spot.get(&pos) else { continue };
            let Some((v, high_good, said)) = key_rate(&r.depth, pos) else { continue };
            let Some(s) = place(v, vals, high_good) else { continue };
            let Some((who, here)) = name(r.id) else { continue };
            let Some(at) = placed(v, vals, high_good, pos, label) else { continue };
            let gone = if here { "" } else { "; he's gone now" };
            let line = format!("{who}, a starting {}: {said}, {at}{gone}", pos.name());
            if s >= STRUGGLED {
                struggled.push((0.4 + 0.5 * s, line));
            } else if s <= STOOD_OUT {
                stood.push((0.35 + 0.4 * (1.0 - s), line));
            }
        }
    }
    struggled.sort_by(|a, b| b.0.total_cmp(&a.0));
    struggled.truncate(2);
    stood.sort_by(|a, b| b.0.total_cmp(&a.0));
    stood.truncate(1);
    let mut missed = Vec::new();
    for r in &ours {
        let out = mine.games.saturating_sub(r.games as u32);
        if out < MISSED {
            continue;
        }
        let Some((who, _)) = name(r.id) else { continue };
        let i = crate::world::folk::person(w, r.id).unwrap();
        let hurt = w.people[i].player.history.iter().rev().take(2).any(|h| h.games_missed > 0);
        let how = if hurt { " hurt" } else { "" };
        missed.push((
            (0.45 + 0.04 * out as f64).min(0.8),
            format!("{who}, a starting {}, missed {out} of the {} games{how}", r.pos.unwrap().name(), mine.games),
        ));
    }
    missed.sort_by(|a, b| b.0.total_cmp(&a.0));
    missed.truncate(2);
    struggled.into_iter().chain(stood).chain(missed).collect()
}

/// A man's starts on offense and defense that season.
fn start_of(s: &Season, id: u32) -> (u8, u8) {
    s.starters.iter().find(|x| x.0 == id).map_or((0, 0), |x| (x.1, x.2))
}
