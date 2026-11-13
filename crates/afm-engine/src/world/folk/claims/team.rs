//! Your team's own week and season (talk facts design, section 9: the
//! team talk, the press, your boss; section 5, "Out of season"): the last
//! game while it is fresh, a streak, the next game, the starters out, and
//! the season that ended. A game is named for a week, so "Monday after
//! the 24-17 loss" is never said in April.

use super::kinds::Kind;
use super::sheet::{self, Point, Ref};
use super::{Reader, same_kind, side_of, words};
use crate::calendar::Date;
use crate::player::PlayingTime;
use crate::world::folk::{Team, facts};

/// A margin this small is a close game, this big a rout. Proposed, not
/// confirmed.
pub const CLOSE_MARGIN: i32 = 8;
/// A margin this big is a rout. Proposed, not confirmed.
pub const BIG_MARGIN: i32 = 21;
/// A streak worth naming.
/// Proposed, not confirmed.
pub const STREAK_GAMES: usize = 3;
/// A game this far ahead still belongs to the season being played (a bye
/// and the wait for a bowl both fit inside it). Proposed, not confirmed.
pub const SEASON_GAP: i32 = 45;
/// The next game is on the table from this far out.
/// Proposed, not confirmed.
pub const NEXT_GAME_DAYS: i32 = 14;
/// A season is a good one from this share of games won, a bad one under
/// this. Proposed, not confirmed.
pub const GOOD_YEAR: f64 = 0.65;
/// Under this share of games won the season was a bad one. Proposed, not confirmed.
pub const BAD_YEAR: f64 = 0.4;

pub(super) fn points(r: &Reader, team: Team, out: &mut Vec<Point>) {
    let played = played(r, team);
    out.extend(last_game(r, &played));
    out.extend(streak(r, &played));
    out.extend(next_game(r, team, &played));
    out.extend(starters_out(r, team));
    out.extend(season_ended(r, team, &played, Kind::SeasonEnded));
}

/// One game a team has played.
pub(super) struct Played {
    pub day: Date,
    pub opp: Team,
    pub opp_name: String,
    pub us: i32,
    pub them: i32,
    pub home: bool,
    /// At a neutral site (a bowl, a playoff or title game).
    pub neutral: bool,
    /// A bowl or playoff game's name.
    pub name: Option<&'static str>,
}

impl Played {
    pub fn result(&self) -> &'static str {
        match self.us.cmp(&self.them) {
            std::cmp::Ordering::Greater => "win",
            std::cmp::Ordering::Less => "loss",
            std::cmp::Ordering::Equal => "tie",
        }
    }
}

/// A team's games played in the last year, newest first. Your own team's
/// come from the games kept whole; another college or club's from its
/// fixtures, this season's and the one before. Other high schools' games
/// keep no score.
pub(super) fn played(r: &Reader, team: Team) -> Vec<Played> {
    let w = r.w;
    let recent = |d: Date| (0..=365).contains(&d.days_to(r.today));
    let mut out: Vec<Played> = Vec::new();
    if Team::seat(w) == Some(team) {
        for g in w.seat_games.iter().rev().filter(|g| recent(g.date)) {
            let Some(side) = side_of(g, team) else { continue };
            out.push(Played {
                day: g.date,
                opp: same_kind(team, g.teams[1 - side]),
                opp_name: g.names[1 - side].clone(),
                us: g.stats.score[side],
                them: g.stats.score[1 - side],
                home: side == 0,
                neutral: false,
                name: None,
            });
        }
    }
    match team {
        Team::College(p) => {
            for prev in [false, true] {
                for f in w.college_fixtures_in(Some(p), prev) {
                    let Some(s) = f.score.filter(|_| !f.cancelled && recent(f.date)) else { continue };
                    let home = f.home == p;
                    let (us, them) = if home { (s[0], s[1]) } else { (s[1], s[0]) };
                    match out.iter_mut().find(|x| x.day == f.date) {
                        Some(x) => {
                            x.name = f.name;
                            x.neutral = f.neutral;
                        }
                        None => out.push(Played {
                            day: f.date,
                            opp: Team::College(if home { f.away } else { f.home }),
                            opp_name: w.program_name(if home { f.away } else { f.home }).to_string(),
                            us: us as i32,
                            them: them as i32,
                            home,
                            neutral: f.neutral,
                            name: f.name,
                        }),
                    }
                }
            }
        }
        Team::Nfl(t) if out.is_empty() => {
            for prev in [false, true] {
                for f in w.nfl_fixtures_in(Some(t), prev) {
                    let (Some(day), Some(s)) = (f.when, f.score) else { continue };
                    if !f.played || !recent(day) {
                        continue;
                    }
                    let home = f.home == t;
                    let (us, them) = if home { (s[0], s[1]) } else { (s[1], s[0]) };
                    let opp = if home { f.away } else { f.home };
                    out.push(Played {
                        day,
                        opp: Team::Nfl(opp),
                        opp_name: w.nfl_team_name(opp),
                        us: us as i32,
                        them: them as i32,
                        home,
                        neutral: false,
                        name: None,
                    });
                }
            }
        }
        _ => {}
    }
    out.sort_by_key(|x| std::cmp::Reverse(x.day));
    out
}

fn place(home: bool, neutral: bool) -> &'static str {
    if neutral {
        "neutral"
    } else if home {
        "home"
    } else {
        "away"
    }
}

/// Whether a college game on a day is at a neutral site.
fn neutral_on(r: &Reader, team: Team, day: Date) -> bool {
    match team {
        Team::College(p) => r.w.college_fixtures(Some(p)).iter().any(|f| f.date == day && f.neutral),
        _ => false,
    }
}

/// The first day of the season the last game played belongs to: back
/// from it, no gap over six weeks.
pub(super) fn season_start(played: &[Played]) -> Option<Date> {
    played.first()?;
    let mut n = 1;
    while n < played.len() && played[n].day.days_to(played[n - 1].day) <= SEASON_GAP {
        n += 1;
    }
    Some(played[n - 1].day)
}

/// A game on the schedule still to be played: a later day, or today's
/// game before it is played.
fn to_play(r: &Reader, day: Date, last: Option<Date>) -> bool {
    day > r.today || (day == r.today && last != Some(r.today))
}

fn margin(g: &Played) -> &'static str {
    let m = (g.us - g.them).abs();
    if m <= CLOSE_MARGIN {
        "close"
    } else if m >= BIG_MARGIN {
        "big"
    } else {
        "clear"
    }
}

/// A point about one played game: the opponent, the score, the day in
/// today's words.
pub(super) fn game_point(r: &Reader, kind: Kind, g: &Played) -> Point {
    let mut pt = Point::new(kind)
        .on(g.day)
        .text("opp", g.opp_name.clone())
        .text("score", format!("{}-{}", g.us, g.them))
        .text("when", words::when_words(g.day, r.today))
        .state("result", g.result())
        .state("margin", margin(g))
        .state("place", place(g.home, g.neutral))
        .refer(Ref::Team(g.opp));
    if let Some(n) = g.name {
        pt = pt.text("game", n);
    }
    pt
}

/// The last game, for a week.
fn last_game(r: &Reader, played: &[Played]) -> Option<Point> {
    let g = played.first()?;
    sheet::in_window(Kind::LastGame, g.day, r.today).then(|| game_point(r, Kind::LastGame, g))
}

/// Three or more won or lost in a row, while the last of them is fresh.
fn streak(r: &Reader, played: &[Played]) -> Option<Point> {
    let first = played.first()?;
    if !sheet::in_window(Kind::Streak, first.day, r.today) || first.result() == "tie" {
        return None;
    }
    // One season's run: a gap of more than six weeks ends it.
    let mut n = 1;
    while n < played.len()
        && played[n].result() == first.result()
        && played[n].day.days_to(played[n - 1].day) <= SEASON_GAP
    {
        n += 1;
    }
    (n >= STREAK_GAMES).then(|| {
        Point::new(Kind::Streak)
            .on(first.day)
            .count("games", n as i64)
            .text("run", words::counted(n as u32, "game"))
            .state("run", if first.result() == "win" { "winning" } else { "losing" })
    })
}

/// The next game, from two weeks out.
fn next_game(r: &Reader, team: Team, played: &[Played]) -> Option<Point> {
    let w = r.w;
    let last = played.first().map(|g| g.day);
    let (day, opp, home) = r.games(team).into_iter().find(|g| to_play(r, g.0, last))?;
    let left = r.today.days_to(day);
    if left > NEXT_GAME_DAYS {
        return None;
    }
    let rival = facts::rival(w, team).is_some_and(|x| x.0 == opp);
    Some(
        Point::new(Kind::NextGame)
            .due(day)
            .text("opp", opp.name(w))
            .text("when", words::when_words(day, r.today))
            .count("days", left as i64)
            .state("place", place(home, neutral_on(r, team, day)))
            .state("rival", if rival { "yes" } else { "no" })
            .refer(Ref::Team(opp)),
    )
}

/// Last season's starters who are out hurt today (your team: the
/// trainers' list).
fn starters_out(r: &Reader, team: Team) -> Option<Point> {
    let w = r.w;
    if Team::seat(w) != Some(team) {
        return None;
    }
    let out: Vec<usize> = team
        .roster(w)
        .into_iter()
        .filter(|&i| {
            let p = &w.people[i];
            p.last_role == Some(PlayingTime::Starter)
                && (w.training.hurt_on(p.player.id, r.today).is_some_and(|h| h.from <= r.today)
                    || p.pro.as_ref().is_some_and(|c| c.out_weeks > 0))
        })
        .collect();
    if out.is_empty() {
        return None;
    }
    let names: Vec<String> = out.iter().take(3).map(|&i| w.people[i].name.clone()).collect();
    let mut pt = Point::new(Kind::StartersOut)
        .count("out", out.len() as i64)
        .text("starters", words::counted(out.len() as u32, "starter"))
        .text("names", words::list(&names));
    for &i in out.iter().take(3) {
        pt = pt.refer(Ref::Player(w.people[i].player.id));
    }
    Some(pt)
}

/// Whether a team still has a game to play this season: one on the
/// schedule inside six weeks, a bowl or a bracket still to be drawn, or a
/// playoff run that is alive.
pub(super) fn still_playing(r: &Reader, team: Team, last: Date) -> bool {
    let w = r.w;
    if r.games(team).iter().any(|g| to_play(r, g.0, Some(last)) && r.today.days_to(g.0) <= SEASON_GAP) {
        return true;
    }
    match team {
        Team::College(p) => {
            // Bowls and brackets are drawn after the last regular game: a
            // team that can be picked waits for selection day.
            let y = &w.dates;
            let fall = y.cfb_weeks.first().is_some_and(|&d| last >= d);
            let rec = w.college_record_in(p, false);
            fall && r.today <= y.selection_day && rec.wins >= rec.losses
        }
        Team::Nfl(t) => w.nfl_season.as_ref().is_some_and(|s| {
            s.started
                && s.champion.is_none()
                && (!s.booked
                    || (s.seed.get(t).copied().flatten().is_some() && s.out_round.get(t).copied().unwrap_or(0) == 0))
        }),
        Team::High(h) => {
            let Some((_, rounds, done)) = w.hs_weeks(h) else { return false };
            !done && (w.hs_alive(h) || rounds.first().is_some_and(|&d| last < d && r.today < d))
        }
        Team::Other(_) => false,
    }
}

/// The season that ended in the last month: the record, the last game by
/// name and score, a good or a bad year. `kind` is what the sheet calls
/// it: a player's `SeasonEnded`, a recruit's `YourBowl`.
pub(super) fn season_ended(r: &Reader, team: Team, played: &[Played], kind: Kind) -> Option<Point> {
    let last = played.first()?;
    if !sheet::in_window(kind, last.day, r.today) || still_playing(r, team, last.day) {
        return None;
    }
    // The season's games: back from the last one, no gap over six weeks.
    let from = season_start(played)?;
    let season: Vec<&Played> = played.iter().take_while(|g| g.day >= from).collect();
    let wins = season.iter().filter(|g| g.us > g.them).count();
    let losses = season.iter().filter(|g| g.us < g.them).count();
    let share = wins as f64 / (wins + losses).max(1) as f64;
    let year = if share >= GOOD_YEAR {
        "good"
    } else if share < BAD_YEAR {
        "bad"
    } else {
        "even"
    };
    let mut pt = game_point(r, kind, last)
        .text("record", format!("{wins}-{losses}"))
        .state("year", year)
        .state("last", if last.name.is_some() { "named" } else { "plain" });
    // A title is in the team's book once the season is filed.
    if let Some(s) = facts::seasons(r.w, team, 1).first().filter(|s| s.year == last_season(last.day))
        && !s.note.is_empty()
    {
        pt = pt.text("finish", s.note.clone());
    }
    Some(pt)
}

/// The season a game belonged to: a game in January or early February is
/// the season before's.
fn last_season(day: Date) -> i32 {
    let (y, m, _) = day.parts();
    if m <= 2 { y - 1 } else { y }
}
