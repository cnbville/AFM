//! The schedule page (update 2, lane 6): the NFL rotation it shows is the
//! one the schedule formula draws; a high school district holds for its
//! two-year cycle with the sites swapped; no team plays past its season's
//! games, a conference move and your own booking included; an NFL playoff
//! game's round comes from the league's dates; the season you coached is
//! kept when you change jobs; an older file's empty last season isn't held.

use crate::calendar::{Date, Level, YearDates};
use crate::game::stats::GameStats;
use crate::world::coach::{Employer, Role};
use crate::world::data::Division;
use crate::world::deals;
use crate::world::hs_year::{cycle_start, state_schedule};
use crate::world::nfl_year::{NflSeason, opponent_formula};
use crate::world::schedule::{self, GameKind, games_target};
use crate::world::season_page::{self as sp, Held, nfl_rotation};
use crate::world::seat::SeatGame;
use crate::world::{World, WorldOptions};

#[test]
fn the_nfl_rotation_matches_the_schedule_formula() {
    let s = NflSeason {
        div: (0..32).map(|t| t / 4).collect(),
        prev_place: (0..32).map(|t| t % 4).collect(),
        ..Default::default()
    };
    for year in 2024..2036 {
        let games = opponent_formula(&s, year, 17);
        for t in 0..32 {
            let d = t / 4;
            let r = nfl_rotation(year, d);
            let mut divs: Vec<usize> = games
                .iter()
                .filter_map(|&(h, a)| {
                    if h == t {
                        Some(a / 4)
                    } else if a == t {
                        Some(h / 4)
                    } else {
                        None
                    }
                })
                .collect();
            divs.sort();
            let mut want = vec![d; 6];
            want.extend([r.conference; 4]);
            want.extend([r.cross; 4]);
            want.extend(r.same_place);
            want.push(r.seventeenth);
            want.sort();
            assert_eq!(divs, want, "{year}: team {t} of division {d}");
            // The 17th game's site: the AFC hosts in odd years.
            let home = games.iter().any(|&(h, a)| h == t && a / 4 == r.seventeenth);
            assert_eq!(home, r.seventeenth_home, "{year}: team {t}'s 17th game site");
        }
    }
    // The cycles: the same conference division every three years, the
    // other conference's every four.
    assert_eq!(nfl_rotation(2026, 0).conference, nfl_rotation(2029, 0).conference);
    assert_ne!(nfl_rotation(2026, 0).conference, nfl_rotation(2027, 0).conference);
    assert_eq!(nfl_rotation(2026, 5).cross, nfl_rotation(2030, 5).cross);
    assert_ne!(nfl_rotation(2026, 5).cross, nfl_rotation(2027, 5).cross);
}

#[test]
fn a_district_holds_for_two_years_and_swaps_its_sites() {
    assert_eq!(cycle_start(2026), 2026);
    assert_eq!(cycle_start(2027), 2026);
    assert_eq!(cycle_start(2028), 2028);
    let ids: Vec<usize> = (0..60).collect();
    let classes: Vec<(u32, &[usize])> = vec![(0, &ids[..30]), (1, &ids[30..])];
    let area = |id: usize| id / 15;
    let power = |id: usize| (0.2 + (id % 7) as f64 * 0.05, false);
    let (first, d1) = state_schedule(&classes, &area, &power, 10, 3, (99, false), None);
    let (second, d2) = state_schedule(&classes, &area, &power, 10, 4, (99, true), None);
    assert_eq!(d1, d2, "the districts hold for the cycle");
    let district_of = |x: usize| d1.iter().position(|d| d.contains(&x));
    let mut n = 0;
    for (wk, games) in first.iter().enumerate() {
        for &(h, a) in games {
            if district_of(h) == district_of(a) {
                assert!(second[wk].contains(&(a, h)), "week {wk}: {h} hosted {a}, so {a} hosts next year");
                n += 1;
            }
        }
    }
    assert!(n > 40, "district games checked: {n}");
    // A new cycle realigns.
    let (_, d3) = state_schedule(&classes, &area, &power, 10, 5, (100, false), None);
    assert_ne!(d1, d3, "a new cycle draws new districts");
}

/// The second year plays the first year's districts even when its season
/// has a week fewer and the private powerhouses' standing moved (Illinois
/// and Wisconsin's 8- and 9-week years), and still swaps every site.
#[test]
fn the_second_year_keeps_its_districts_when_the_weeks_change() {
    let ids: Vec<usize> = (0..60).collect();
    let classes: Vec<(u32, &[usize])> = vec![(0, &ids[..30]), (1, &ids[30..])];
    let area = |id: usize| id / 15;
    let before = |id: usize| (if id < 5 { 0.9 } else { 0.3 }, id < 5);
    let after = |id: usize| (if (2..7).contains(&id) { 0.9 } else { 0.3 }, id < 7);
    let (first, d1) = state_schedule(&classes, &area, &before, 9, 3, (99, false), None);
    let (second, d2) = state_schedule(&classes, &area, &after, 8, 4, (99, true), Some(&d1));
    assert_eq!(d1, d2, "the districts hold through the cycle");
    let district_of = |x: usize| d1.iter().position(|d| d.contains(&x));
    let pairs = |weeks: &Vec<Vec<(usize, usize)>>| -> Vec<(usize, usize)> {
        weeks.iter().flatten().copied().filter(|&(h, a)| district_of(h) == district_of(a)).collect()
    };
    let (one, two) = (pairs(&first), pairs(&second));
    assert!(one.len() > 40, "district games: {}", one.len());
    for &(h, a) in &one {
        assert!(two.contains(&(a, h)), "{h} hosted {a}, so {a} hosts in the second year");
    }
}

/// The leagues set their schedules before the contracts go in: the books
/// cost no team a league game, and nearly every contract is played. Then
/// an independent that booked its dates as one joins a league (BYU's 2023
/// move): nobody plays past the season's games, it plays its new league's
/// schedule, and next February's books drop the newest contracts it can no
/// longer play.
#[test]
fn a_conference_move_never_gives_a_team_more_games_than_its_season() {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, ..Default::default() });
    let season = w.year + 1;
    deals::top_up(&mut w);
    let fbs = |w: &World, p: usize| w.programs[p].division == Division::FBS && w.programs[p].plays(season);
    // League games short across FBS, and contracts not played.
    let short = |w: &World| -> (u32, usize) {
        let games = schedule::build(w, season, &YearDates::new(season));
        let mut conf = vec![0u32; w.programs.len()];
        for g in games.iter().filter(|g| g.kind == GameKind::Regular && g.conference) {
            conf[g.home] += 1;
            conf[g.away] += 1;
        }
        let missing = (0..w.programs.len())
            .filter(|&p| fbs(w, p))
            .map(|p| deals::conference_games(w, p, season).saturating_sub(conf[p]))
            .sum();
        let played: std::collections::HashSet<(usize, usize)> = games.iter().map(|g| (g.home, g.away)).collect();
        (missing, deals::for_season(w, season).filter(|d| !played.contains(&(d.home, d.away))).count())
    };
    let contracts = deals::for_season(&w, season).count();
    let (missing, unplayed) = short(&w);
    let kept = std::mem::take(&mut w.deals);
    let (missing_without, _) = short(&w);
    w.deals = kept;
    assert_eq!(missing, missing_without, "the contracts cost league games");
    assert!(contracts > 100 && unplayed * 50 <= contracts, "{unplayed} of {contracts} contracts not played");
    let ind = (0..w.programs.len())
        .filter(|&p| fbs(&w, p) && w.programs[p].conference.contains("Independent"))
        .max_by_key(|&p| deals::booked(&w, p, season))
        .expect("an FBS independent");
    let booked_before = deals::booked(&w, ind, season);
    let league = (0..w.programs.len())
        .filter(|&p| fbs(&w, p) && !w.programs[p].conference.contains("Independent"))
        .max_by_key(|&p| deals::conference_games(&w, p, season))
        .unwrap();
    w.programs[ind].conference = w.programs[league].conference.clone();
    let league_games = deals::conference_games(&w, ind, season);
    assert!(
        booked_before + league_games > games_target(Division::FBS, season),
        "the test needs an overbooked independent: {booked_before} contracts, {league_games} league games"
    );
    let games = schedule::build(&w, season, &YearDates::new(season));
    let mut count = vec![0u32; w.programs.len()];
    let mut conf = vec![0u32; w.programs.len()];
    for g in games.iter().filter(|g| g.kind == GameKind::Regular) {
        for t in [g.home, g.away] {
            count[t] += 1;
            conf[t] += g.conference as u32;
        }
    }
    for (p, prog) in w.programs.iter().enumerate().filter(|(_, x)| x.plays(season)) {
        let target = games_target(prog.division, season);
        assert!(count[p] <= target, "{} plays {} games, {target} allowed", prog.real_name, count[p]);
    }
    let name = w.programs[ind].real_name.clone();
    assert!(
        conf[ind] >= league_games,
        "{name} plays {} of its {league_games} league games beside its contracts",
        conf[ind]
    );
    // Next February: the contracts past the new league's open dates go.
    deals::top_up(&mut w);
    for y in w.year + 1..=w.year + deals::HORIZON + 1 {
        assert!(deals::booked(&w, ind, y) <= deals::slots(&w, ind, y), "{name} {y}");
    }
}

/// The athletic directors leave your date four years out for the March
/// call, and the call comes only with a date really open.
#[test]
fn the_booking_letter_comes_only_with_a_date_open() {
    use crate::world::legacy::{ACT_BOOKING, college};
    let mut w = super::legacy::seat("college");
    let (_, p) = w.seat_team().unwrap();
    let target = w.year + 4;
    deals::top_up(&mut w);
    assert!(
        deals::booked(&w, p, target) < deals::slots(&w, p, target),
        "the books hold a date for your call: {} of {}",
        deals::booked(&w, p, target),
        deals::slots(&w, p, target)
    );
    let calls = |w: &World| w.legacy.pending.iter().filter(|x| x.1 == ACT_BOOKING).count();
    let march = Date::ymd(w.year, 3, 10);
    // Booked full by hand: no letter.
    let before = w.deals.len();
    let mut q = 0;
    while deals::booked(&w, p, target) < deals::slots(&w, p, target) {
        if q != p && w.programs[q].division == Division::FBS && !deals::met(&w, p, q, target) {
            w.deals.push(deals::neutral(p, q, target, w.year, 0.5));
        }
        q += 1;
    }
    college::day(&mut w, march);
    assert_eq!(calls(&w), 0, "no letter with every date booked");
    // With the date open again, the athletic director calls.
    w.deals.truncate(before);
    college::day(&mut w, march);
    assert_eq!(calls(&w), 1, "the March call with a date open");
}

/// An NFL playoff game's round is read from the league's dates: a Monday
/// wild card game after a Saturday finale is still the wild card round,
/// and the conference title games are never the Super Bowl.
#[test]
fn an_nfl_playoff_games_round_comes_from_the_leagues_dates() {
    for season in 1996..=2045 {
        let y = YearDates::new(season);
        for d in 0..3 {
            assert_eq!(sp::playoff_round(&y, y.wild_card.add(d)), 1, "{season} wild card +{d}");
        }
        for d in 0..2 {
            assert_eq!(sp::playoff_round(&y, y.divisional.add(d)), 2, "{season} divisional +{d}");
        }
        assert_eq!(sp::playoff_round(&y, y.conference_title_games), 3, "{season} title games");
        assert_eq!(sp::playoff_round(&y, y.super_bowl), 4, "{season} Super Bowl");
        let (stadium, town, state, _) = sp::super_bowl_site(season);
        assert!(!stadium.is_empty() && !town.is_empty() && state.len() == 2, "{season}");
    }
}

/// Taking another job keeps the season you coached so far; the new team's
/// season, which you never coached, isn't kept as yours.
#[test]
fn a_job_change_keeps_the_season_you_coached() {
    use crate::world::front_office::market;
    let mut w = super::legacy::seat("college");
    w.college = Some(crate::world::college::new_season(&w));
    let (level, p) = w.seat_team().unwrap();
    let page = sp::page(&w, level, p, Held::This).expect("this season's page");
    let g = page.games.iter().find(|g| !g.bowl && !g.playoff).expect("a game");
    let teams = if g.home { [p, g.opp] } else { [g.opp, p] };
    w.seat_games.push(SeatGame {
        date: g.date,
        level,
        teams,
        names: Default::default(),
        stats: GameStats::default(),
        replay: Vec::new(),
        marks: Vec::new(),
        gone: Vec::new(),
        spread: None,
    });
    let other = (0..w.programs.len()).find(|&q| q != p && w.programs[q].division == Division::FBS).unwrap();
    market::hired(&mut w, Employer::College(other), Role::HeadCoach);
    let kept = sp::kept(&w, level, p);
    assert_eq!(kept.len(), 1, "the season you coached is kept");
    assert_eq!(kept[0].year, page.year);
    // Fired from the new job before coaching a game there: nothing more.
    market::left(&mut w, "fired");
    assert!(sp::kept(&w, level, other).is_empty(), "a season you never coached isn't yours");
    assert_eq!(sp::kept(&w, level, p).len(), 1);
}

/// A world or save from before last season's games were kept holds its
/// last college season with no games: the page isn't held, so its book
/// line shows; with the games kept, it is.
#[test]
fn an_older_files_empty_last_season_is_not_held() {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, ..Default::default() });
    let p = w.programs.iter().position(|x| x.division == Division::FBS).unwrap();
    let mut cs = crate::world::college::new_season(&w);
    cs.year -= 1;
    let games = std::mem::take(&mut cs.games);
    assert!(!games.is_empty());
    let last = cs.year;
    w.college_prev = Some(cs);
    assert!(sp::page(&w, Level::College, p, Held::Last).is_none(), "no games, no page");
    assert_eq!(sp::held_years(&w, Level::College).1, None, "no games, not held");
    w.college_prev.as_mut().unwrap().games = games;
    assert!(sp::page(&w, Level::College, p, Held::Last).is_some());
    assert_eq!(sp::held_years(&w, Level::College).1, Some(last));
}
