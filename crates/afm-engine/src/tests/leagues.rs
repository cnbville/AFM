//! The other leagues (later extras X7): their teams by year, their seasons,
//! their rule books on the engine and the CFL's ratio.

use std::collections::HashSet;

use crate::calendar::Date;
use crate::game::books::Book;
use crate::game::roster::depth_chart;
use crate::game::synthetic::synthetic_league;
use crate::game::{GamePlayer, Venue, quick_game_book};
use crate::player::Level;
use crate::positions::Position;
use crate::world::pro::other::{self as o, AAF, AFL1, CFL, NFLE, UFL09, UFL24, USFL22, XFL01, XFL20, XFL23};
use crate::world::{World, WorldOptions};

#[test]
fn each_league_fields_its_real_teams() {
    let n = |k: u8, y: i32| o::teams_in(k, y).len();
    // The CFL: nine from 1996 (Ottawa's Rough Riders folded after it), eight
    // in 1997-2001, the Renegades 2002-05, the Redblacks from 2014.
    assert_eq!((n(CFL, 1996), n(CFL, 1998), n(CFL, 2003), n(CFL, 2010), n(CFL, 2026)), (9, 8, 9, 8, 9));
    assert_eq!((n(NFLE, 2000), n(XFL01, 2001), n(AAF, 2019), n(XFL20, 2020)), (6, 8, 8, 8));
    assert_eq!((n(XFL23, 2023), n(USFL22, 2022), n(UFL24, 2024), n(UFL09, 2009)), (8, 8, 8, 4));
    // Arena football's biggest years had 19 teams.
    assert!((15..=19).contains(&n(AFL1, 2007)), "AFL 2007: {}", n(AFL1, 2007));
    // Every season the world plays has a format and an even schedule.
    for k in 0..o::LEAGUE_COUNT as u8 {
        for y in o::FULL_FROM..=2026 {
            if !o::full(k, y) {
                continue;
            }
            let f = o::format(k, y).unwrap();
            let teams = o::teams_in(k, y);
            let weeks = o::schedule(&teams, f.games, 7);
            for &t in &teams {
                let played = weeks.iter().flatten().filter(|g| g.0 as usize == t || g.1 as usize == t).count();
                assert_eq!(played, f.games as usize, "{} {y}: {} played {played}", o::league_short(k), o::name(t, y));
            }
            for wk in &weeks {
                let mut seen = HashSet::new();
                assert!(
                    wk.iter().all(|g| seen.insert(g.0) && seen.insert(g.1)),
                    "{} {y}: a team twice in a week",
                    o::league_short(k)
                );
            }
        }
    }
}

#[test]
fn the_rule_books_read_in_plain_words() {
    let has = |k: u8, y: i32, s: &str| o::rules_words(k, y).iter().any(|l| l.contains(s));
    assert!(has(CFL, 2024, "Three downs") && has(CFL, 2024, "12 a side") && has(CFL, 2024, "rouge"));
    assert!(has(AFL1, 2005, "No punting") && has(AFL1, 2005, "nets") && has(AFL1, 2005, "8 a side"));
    assert!(has(NFLE, 2000, "worth 4"));
    assert!(has(AAF, 2019, "No kickoffs"));
    assert!(has(XFL01, 2001, "No fair catch"));
}

/// The CFL and arena books on the engine score like their leagues (the
/// engine's NFL runs a little high on synthetic rosters, so the bands are
/// wide; `match_check cfl` and `match_check arena` measure them closely).
#[test]
fn the_cfl_and_arena_books_score_like_their_leagues() {
    let teams = synthetic_league(Level::NFL, 8, 21);
    for (book, year, lo, hi) in [(Book::Cfl, 2024, 18.0, 40.0), (Book::Arena, 2015, 36.0, 68.0)] {
        let games = 12;
        let mut pts = 0.0;
        for g in 0..games {
            let (h, a) = (2 * (g % 4), 2 * (g % 4) + 1);
            let r = quick_game_book(&teams[h], &teams[a], book, year, 4_400_017 + g as u64, Venue::default(), false);
            pts += (r.stats.score[0] + r.stats.score[1]) as f64;
        }
        let per = pts / (2 * games) as f64;
        assert!((lo..hi).contains(&per), "{book:?}: {per:.1} points a team game");
    }
}

#[test]
fn the_cfl_starts_seven_canadians() {
    use Position::*;
    let mut t = synthetic_league(Level::NFL, 2, 5).remove(0);
    // The Canadians are the two weakest at four spots: none would start.
    let mut nat: HashSet<u32> = HashSet::new();
    for pos in [IOL, S, LB, WR] {
        let mut v: Vec<&GamePlayer> = t.players.iter().filter(|p| p.position == pos).collect();
        v.sort_by(|a, b| a.overall_at(pos).total_cmp(&b.overall_at(pos)));
        nat.extend(v.iter().take(2).map(|p| p.id));
    }
    let starting = |t: &crate::game::GameTeam| {
        let d = depth_chart(t, 99.0);
        Position::ALL
            .iter()
            .map(|&pos| {
                d.at[pos.index()].iter().take(o::cfl_starts(pos)).filter(|&&i| nat.contains(&t.players[i].id)).count()
            })
            .sum::<usize>()
    };
    assert!(starting(&t) < 7);
    t.order = o::ratio_starters(&t.players, &nat, 7);
    assert!(starting(&t) >= 7, "{} Canadians start", starting(&t));
}

/// A spring league plays its season through to its title game on the
/// engine: every team's ten games, a champion, rosters filled from the
/// street and below it, the coaches' seats filled by the carousel.
#[test]
fn the_xfl_of_2001_plays_to_a_champion() {
    let mut w = World::empty(WorldOptions { scale: 0.001, ..Default::default() });
    w.year = 2001;
    let mut d = Date::ymd(2001, 1, 1);
    while d < Date::ymd(2001, 5, 1) {
        w.today = d;
        o::day(&mut w, d);
        d = d.add(1);
    }
    let s = w.others.seasons.iter().find(|s| s.league == XFL01 && s.year == 2001).expect("the XFL's season");
    assert_eq!(s.table.len(), 8);
    assert!(s.table.iter().all(|r| r.w + r.l + r.t == 10), "{:?}", s.table);
    assert!(s.champion.is_some() && s.runner_up.is_some());
    let fr = s.champion.unwrap() as usize;
    let roster = &w.others.teams[fr].roster;
    assert!(roster.is_empty(), "a spring team's players go back to the street at the season's end");
    // The players' lines are on their pages.
    let lines = w
        .people
        .iter()
        .filter_map(|p| p.pro.as_ref())
        .flat_map(|c| c.other.iter())
        .filter(|s| s.league == XFL01)
        .count();
    assert!(lines >= 8 * 30, "{lines} XFL season lines");
    // Arena football and NFL Europe were playing too that spring.
    assert!(w.others.live.iter().any(|l| l.league == AFL1) || w.others.seasons.iter().any(|s| s.league == AFL1));
    let seated =
        (0..o::FRANCHISES).filter(|&f| o::league_of(f) == XFL01).filter(|&f| o::head_coach(&w, f).is_some()).count();
    assert!(seated >= 6, "{seated} XFL head coaches");
}

#[test]
fn rosters_follow_the_people_when_the_world_compacts() {
    let mut x = o::Others::default();
    x.teams.resize(3, o::Team::default());
    x.teams[1].roster = vec![0, 2, 5];
    // Person 2 left football; 5 moved up to 4.
    let map = [Some(0), Some(1), None, Some(2), Some(3), Some(4)];
    x.remap(&map);
    assert_eq!(x.teams[1].roster, vec![0, 4]);
}

/// Your club in another league (later extras topic 5): the morning's game
/// is the one the day plays, your games are kept, and the film room keeps
/// your coming opponents' tapes.
#[test]
fn your_club_in_another_league_plays_its_season() {
    use crate::calendar::Level as Cal;
    use crate::world::Seat;
    let mut w = World::empty(WorldOptions { scale: 0.001, ..Default::default() });
    w.year = 2001;
    let fr = *o::teams_in(XFL01, 2001).first().unwrap();
    let mut d = Date::ymd(2001, 1, 1);
    let mut played = 0;
    while d < Date::ymd(2001, 5, 1) {
        w.today = d;
        if w.seat.is_none() && w.others.live.iter().any(|l| l.league == XFL01 && l.year == 2001) {
            w.seat = Some(Seat { level: Cal::Other, nfl_team: None, program: None, other: Some(fr) });
            assert_eq!(o::seat_fixtures(&w).len(), 10, "ten games ahead");
            assert!(!o::seat_opponents(&w).is_empty());
        }
        let today = w.seat_setup_today();
        let before = w.seat_games.len();
        let dry = today.as_ref().map(|s| crate::game::play_game(&s.params.setup(&s.home, &s.away)));
        o::day(&mut w, d);
        // Injuries heal a day at a time after the day's games, as the
        // world's clock runs it (`nfl_year::day`; this world has no NFL).
        let y = w.dates.clone();
        crate::world::pro::heal_day(&mut w, d, &y);
        if let (Some(s), Some(r)) = (today, dry) {
            assert_eq!(w.seat_games.len(), before + 1, "{d:?}: your game is kept");
            let g = w.seat_games.last().unwrap();
            assert_eq!(g.teams, s.teams);
            assert!(g.teams.contains(&fr));
            assert!(!g.replay.is_empty(), "{d:?}: your game keeps its film");
            let score = |t: &crate::game::GameStats| [t.team[0].points, t.team[1].points];
            assert_eq!(score(&g.stats), score(&r.stats), "{d:?}: the day played the morning's game");
            played += 1;
            assert!(w.gameday.tapes.iter().all(|t| t.level == Cal::Other), "only this league's film");
            // The club's schedule page shows every game it has played so far.
            if w.others.live.iter().any(|l| l.league == XFL01 && l.year == 2001) {
                let shown = o::season_games(&w, fr).iter().filter(|g| g.3.is_some()).count();
                assert_eq!(shown, played, "{d:?}: the schedule has every game played");
            }
        } else {
            assert_eq!(w.seat_games.len(), before, "{d:?}: a game with no morning");
        }
        d = d.add(1);
    }
    assert!(played >= 10, "{played} games for your club");
    assert_eq!(o::record_of(&w, fr).map(|(_, r)| r.w + r.l + r.t), Some(10));
}

#[test]
fn a_club_in_another_league_has_a_city_and_a_nickname() {
    let club = |n: &str| (0..o::FRANCHISES).find(|&f| o::name(f, 2026) == n).unwrap();
    let wpg = club("Winnipeg Blue Bombers");
    assert_eq!((o::city(wpg, 2026), o::nickname(wpg, 2026)), ("Winnipeg", "Blue Bombers"));
    let edm = club("Edmonton Elks");
    assert_eq!((o::city(edm, 2026), o::nickname(edm, 2026)), ("Edmonton", "Elks"));
    let stl = club("St. Louis Battlehawks");
    assert_eq!((o::city(stl, 2026), o::nickname(stl, 2026)), ("St. Louis", "Battlehawks"));
}
