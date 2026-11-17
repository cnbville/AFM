//! A coach's season review (Cain 2026-11-17, "Give coaches real depth when
//! asked why"): a coordinator asked how the year went walks you through it
//! from the world's season tallies, a season played before you took the
//! job included; a man hired after the season says it isn't his; a push
//! gets the rest; your boss knows last season's record before a game is
//! played.

use super::legacy::seat;
use crate::game::stats::GameStats;
use crate::world::World;
use crate::world::coach::Role;
use crate::world::folk::claims::rooms::{self, Form};
use crate::world::folk::persona::knows::{Knows, Topic};
use crate::world::folk::persona::people::People;
use crate::world::folk::persona::review::{self, FIX, HEADER};
use crate::world::folk::persona::{Them, makeup};
use crate::world::folk::{Team, facts};
use crate::world::program::ProgramSeason;
use crate::world::tally::{self, games};

/// A college game in the world's season tallies (never in your seat's
/// kept games): home and away, the score, days ago. Its boxes carry the
/// points and a plain line of yards and downs, so every number reads.
pub(crate) fn tally(w: &mut World, home: usize, away: usize, score: [i32; 2], days_ago: i32) {
    w.recording = true;
    if w.tallies.season == 0 {
        w.tallies.season = w.year;
    }
    let mut g = GameStats { score, ..Default::default() };
    for side in 0..2 {
        let t = &mut g.team[side];
        let pts = score[side].max(0) as f64;
        t.points = score[side];
        t.plays = 65;
        t.yards = 150.0 + 9.0 * pts;
        t.rush_yards = 60.0 + 3.0 * pts;
        t.pass_yards = t.yards - t.rush_yards;
        t.carries = 30;
        t.att = 32;
        t.third_att = 13;
        t.third_conv = 3 + (pts / 6.0) as u32;
        t.red_zone_trips = 4;
        t.red_zone_tds = (pts / 10.0).min(4.0) as u32;
        t.explosive = 2 + (pts / 8.0) as u32;
        t.turnovers = if pts < 14.0 { 3 } else { 1 };
        t.sacks = if pts < 14.0 { 4 } else { 1 };
    }
    let f = games::Facts {
        level: tally::Level::College,
        season: w.tally_open_season(),
        date: w.today.add(-days_ago),
        kind: "regular",
        teams: [home as u32, away as u32],
        group: "FBS".into(),
        neutral: false,
        conference: true,
        weather: Default::default(),
        ranked: [false; 2],
        spread: None,
        lv: None,
    };
    let part = games::part(f, &g, false, &|_, _| None);
    w.tally_game(part);
}

/// Your program's conference that season and its other members.
fn conference(w: &World) -> (usize, Vec<usize>) {
    let mine = w.seat.and_then(|s| s.program).unwrap();
    let conf = w.programs[mine].conference.clone();
    let others: Vec<usize> = (0..w.programs.len()).filter(|&k| k != mine && w.programs[k].conference == conf).collect();
    (mine, others)
}

/// A season on file before you came: your defense gives up `allowed` a game
/// over 12 games, the conference's other teams play each other evenly.
fn season(w: &mut World, allowed: i32) {
    let (mine, others) = conference(w);
    assert!(others.len() >= 6, "the conference has teams to rank against: {}", others.len());
    for k in 0..12 {
        let opp = others[k % others.len()];
        tally(w, mine, opp, [20, allowed], 120 - 7 * k as i32);
    }
    for (k, pair) in others.windows(2).enumerate() {
        for r in 0..6 {
            tally(w, pair[0], pair[1], [24, 24 + (r % 3) * 3 - 3], 118 - 7 * (k + r as usize) as i32);
        }
    }
    // The season is over: its row is in the program's seasons.
    let won = if allowed < 20 { 12 } else { 0 };
    w.programs[mine].seasons.push(ProgramSeason {
        year: w.year,
        wins: won,
        losses: 12 - won,
        conference_title: false,
        national_title: false,
        coach: None,
        strength: 0.0,
        totals: Default::default(),
        conference_won: None,
        conference: None,
        division: None,
        points: [240, 12 * allowed.max(0) as u32],
    });
}

/// Your defensive coordinator, his first season `from` seasons after this
/// one (negative: before).
fn coordinator(w: &mut World, from: i32) -> usize {
    let first = w.year + from;
    let team = Team::seat(w).unwrap();
    let (c, _) = facts::assistants(w, team).into_iter().next().expect("the program has a staff");
    let job = w.coaches[c].job.as_mut().unwrap();
    job.role = Role::DefensiveCoordinator;
    job.first = first;
    c
}

#[test]
fn a_coordinator_knows_a_season_played_before_you_came() {
    let mut w = seat("college");
    assert!(w.seat_games.is_empty(), "no game under you");
    season(&mut w, 38);
    let c = coordinator(&mut w, -2);
    // His unit's year comes from the world's records.
    let u = rooms::unit(&w, c).unwrap();
    assert_eq!(u.form, Some(Form::Poor), "{u:?}");
    assert!(u.year_words().is_some_and(|y| y.contains("gave up 38 a game")), "{u:?}");
    assert_eq!(u.years, 3, "two seasons and this one: {u:?}");
    // Asked about his unit, he walks through the season with numbers.
    let known = People.knows(&w, Them::Coach(c), Topic::Football, None);
    let words: Vec<&str> = known.iter().map(|k| k.words.as_str()).collect();
    assert!(words[0].starts_with(HEADER) && words[0].contains("he coached the defense"), "{words:?}");
    assert!(words.iter().any(|x| x.contains("gave up 38.0 points a game") && x.contains(" of ")), "{words:?}");
    assert!(words.last().unwrap().starts_with(FIX), "{words:?}");
    assert!(!words.iter().any(|x| x.contains("general terms")), "{words:?}");
}

#[test]
fn a_coordinator_hired_after_the_season_says_it_isnt_his() {
    let mut w = seat("college");
    season(&mut w, 38);
    let c = coordinator(&mut w, 1);
    let u = rooms::unit(&w, c).unwrap();
    assert_eq!(u.years, 0, "he hasn't coached a game here: {u:?}");
    assert!(u.form.is_none() && u.points.is_none(), "a season he didn't coach isn't his form: {u:?}");
    let known = People.knows(&w, Them::Coach(c), Topic::Football, None);
    assert!(known[0].words.contains("before he got here") && known[0].words.contains("isn't his season"), "{known:?}");
    // His case says so too.
    let case = crate::world::folk::persona::talk::stakes::case_facts(&w, c);
    assert!(case.iter().any(|x| x.contains("hasn't coached a game here yet")), "{case:?}");
    assert!(!case.iter().any(|x| x.contains("one season")), "{case:?}");
}

#[test]
fn a_sharper_coach_sees_more_of_the_season() {
    let mut w = seat("college");
    season(&mut w, 38);
    let c = coordinator(&mut w, 0);
    let mut m = makeup::makeup(&w, Them::Coach(c));
    m.iq = 4.0;
    let dull = review::review(&w, c, &m).unwrap();
    m.iq = 19.0;
    let sharp = review::review(&w, c, &m).unwrap();
    assert!(dull.points.len() <= review::REACH_MIN, "{dull:?}");
    assert!(sharp.points.len() > dull.points.len(), "{sharp:?} against {dull:?}");
    // The heaviest first: the same first point either way.
    assert_eq!(dull.points.first(), sharp.points.first());
}

#[test]
fn the_first_answer_gives_the_heaviest_and_a_push_the_rest() {
    let mut w = seat("college");
    season(&mut w, 38);
    let c = coordinator(&mut w, 0);
    let known = People.knows(&w, Them::Coach(c), Topic::Football, None);
    let points = known.len() - 2;
    let page = crate::world::folk::persona::talk::review_page;
    let (first, n) = page(&known, true, 0).unwrap();
    assert_eq!(n, points.min(review::FIRST_POINTS));
    assert_eq!(first.len(), 2 + n, "{first:?}");
    assert!(first[0].starts_with(HEADER) && first.last().unwrap().starts_with(FIX), "{first:?}");
    // Held back: the header and one point; a push then starts at the second.
    let (held, one) = page(&known, false, 0).unwrap();
    assert_eq!((held.len(), one), (2, 1), "{held:?}");
    if points > 1 {
        let (next, _) = page(&known, true, 1).unwrap();
        assert_eq!(next[0], known[2].words, "the push gives the second point: {next:?}");
    }
    // After the first three, a push gives what he hadn't said, or nothing.
    match page(&known, true, n) {
        Some((more, _)) => assert!(more.iter().all(|x| !first.contains(x)), "{more:?}"),
        None => assert_eq!(n, points, "nothing left only when all were given"),
    }
}

#[test]
fn your_boss_knows_last_season_before_a_game_is_played() {
    let w = seat("college");
    let team = Team::seat(&w).unwrap();
    let Some(last) = facts::seasons(&w, team, 1).last().cloned() else { return };
    let ad = crate::world::folk::persona::office::cast_them(
        &w,
        crate::world::folk::cast::CastRef::new(team, crate::world::folk::cast::Role::AthleticDirector, 0),
    );
    let view = People.knows(&w, ad, Topic::Football, None);
    assert!(view.iter().any(|k| k.words.contains(&format!("{}-{}", last.wins, last.losses))), "{view:?}");
}
