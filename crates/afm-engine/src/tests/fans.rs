//! Fan voices (people topic 9, the fan lane): the town after a game, the
//! call-in show and the board read whole at every level; the live crowd
//! says what the kept game's page says; the mood follows the results
//! against the lines; a crowd says every version before it repeats one.

use crate::calendar::{Date, Level};
use crate::game::OffScheme;
use crate::game::stats::GameStats;
use crate::game::synthetic::synthetic_team;
use crate::game::{GameSetup, play_game};
use crate::world::World;
use crate::world::data::Division;
use crate::world::folk::Team;
use crate::world::folk::fans::{self, Mood};
use crate::world::folk::voice::{Chorus, Facts, Speaker};
use crate::world::seat::SeatGame;

/// Text a screen can show: no holes, choices or tags left, and nothing
/// that isn't a word. A name link (`[[p:id|Last]]`) is fine.
fn whole(s: &str) -> bool {
    let mut t = String::new();
    let mut rest = s;
    while let Some(a) = rest.find("[[") {
        t.push_str(&rest[..a]);
        let Some(b) = rest[a..].find("]]") else { return false };
        t.push('X');
        rest = &rest[a + b + 2..];
    }
    t.push_str(rest);
    !t.trim().is_empty()
        && !t.contains(['{', '}', '[', ']', '|', '~'])
        && !t.contains("  ")
        && !t.split(|c: char| !c.is_alphanumeric()).any(|w| ["NaN", "inf", "undefined", "null"].contains(&w))
}

/// Your seat at one level, today in late October, and an opponent.
fn seat(level: &str) -> (World, Team, usize, Level) {
    let mut w = super::legacy::seat(level);
    w.today = Date::ymd(2037, 10, 27);
    let team = Team::seat(&w).expect("a seat");
    let (_, i) = team.parts();
    let (opp, lv) = match team {
        Team::Nfl(_) => (
            (0..w.clubs.len()).find(|&t| t != i && crate::world::franchise::active_in(&w, t, w.year)).unwrap(),
            Level::Nfl,
        ),
        Team::College(_) => (
            (0..w.programs.len()).find(|&p| p != i && w.programs[p].division == Division::FBS).unwrap(),
            Level::College,
        ),
        _ => {
            let st = w.high_schools[i].state;
            ((0..w.high_schools.len()).find(|&h| h != i && w.high_schools[h].state == st).unwrap(), Level::HighSchool)
        }
    };
    (w, team, opp, lv)
}

/// A kept game `ago` days before today.
fn keep(w: &mut World, ago: i32, level: Level, teams: [usize; 2], stats: GameStats, spread: Option<f32>) {
    let date = w.today.add(-ago);
    w.seat_games.push(SeatGame {
        date,
        level,
        teams,
        names: ["Home".into(), "Away".into()],
        stats,
        replay: Vec::new(),
        marks: Vec::new(),
        gone: Vec::new(),
        spread,
        talks: Default::default(),
    });
}

/// Three of your games on the engine, a week apart, the last two days ago.
fn played(level: &str) -> (World, Team) {
    let (mut w, team, opp, lv) = seat(level);
    let (_, i) = team.parts();
    let pl = match lv {
        Level::Nfl => crate::player::Level::NFL,
        Level::HighSchool => crate::player::Level::HS,
        _ => crate::player::Level::FBS,
    };
    for g in 0..3u32 {
        let home = synthetic_team("Home", pl, 0.5, OffScheme::WestCoast, 300 + g);
        let away = synthetic_team("Away", pl, 0.5, OffScheme::SpreadOption, 700 + g);
        let r = play_game(&GameSetup::new(&home, &away, pl, 50 + g as u64));
        let teams = if g == 1 { [opp, i] } else { [i, opp] };
        keep(&mut w, 2 + 7 * (2 - g as i32), lv, teams, r.stats, Some(3.0));
    }
    (w, team)
}

#[test]
fn the_town_the_show_and_the_board_read_whole_at_every_level() {
    for level in ["hs", "college", "nfl"] {
        let (w, team) = played(level);
        let x = fans::week(&w, team);
        assert!(whole(&x.board) && whole(&x.station), "{level}: {} / {}", x.board, x.station);
        assert!(x.station.starts_with(['W', 'K']), "{level}: call letters {}", x.station);
        let (k, town) = x.town.as_ref().unwrap_or_else(|| panic!("{level}: the town after the last game"));
        assert_eq!(*k, 2, "{level}: the last game");
        assert!((3..=6).contains(&town.len()), "{level}: {} in town", town.len());
        for v in town {
            assert!(whole(&v.who) && whole(&v.place) && whole(&v.text), "{level}: {v:?}");
        }
        assert!(!x.shows.is_empty(), "{level}: a show on each game night");
        let calls: Vec<_> = x.shows.iter().flat_map(|s| &s.calls).collect();
        assert!(!calls.is_empty(), "{level}: callers");
        for s in &x.shows {
            assert!(whole(&s.name), "{level}: {}", s.name);
        }
        for c in &calls {
            assert!(whole(&c.caller) && whole(&c.intro) && whole(&c.text), "{level}: {c:?}");
        }
        assert!(!x.threads.is_empty(), "{level}: threads on the board");
        for t in &x.threads {
            assert!(whole(&t.title) && !t.posts.is_empty(), "{level}: {t:?}");
            for p in &t.posts {
                assert!(whole(&p.by) && whole(&p.text), "{level}: {p:?}");
            }
        }
        // Each kept game: the signs, the crowd and the town.
        for k in 0..3 {
            let v = fans::game(&w, k).unwrap_or_else(|| panic!("{level}: game {k}"));
            assert_eq!(v.team, team, "{level}: your fans' town");
            assert!(!v.signs.is_empty() && v.signs.iter().all(|s| whole(s)), "{level}: {:?}", v.signs);
            assert!(v.crowd.len() >= 3, "{level}: game {k} crowd {}", v.crowd.len());
            for c in &v.crowd {
                assert!(whole(&c.text), "{level}: {c:?}");
            }
            assert!(v.town.iter().all(|t| whole(&t.text)), "{level}: {:?}", v.town);
        }
        // The same screen opened twice says the same.
        let again = fans::week(&w, team);
        assert_eq!(again.town.as_ref().unwrap().1[0].text, town[0].text, "{level}");
    }
}

#[test]
fn the_live_crowd_says_what_the_kept_page_says() {
    for level in ["hs", "college", "nfl"] {
        let (w, team) = played(level);
        let k = 2;
        let g = &w.seat_games[k];
        let kept = fans::game(&w, k).unwrap();
        let (signs, crowd) = fans::live(&w, team, g.date, g.level, g.teams, &g.stats.plays, Some(g.stats.score));
        assert_eq!(signs, kept.signs, "{level}: the signs");
        let pairs = |c: &[fans::Cheer]| c.iter().map(|c| (c.n, c.text.clone())).collect::<Vec<_>>();
        assert_eq!(pairs(&crowd), pairs(&kept.crowd), "{level}: the crowd");
        // Half the game in: what the crowd said so far doesn't change.
        let half = g.stats.plays.len() / 2;
        let (_, part) = fans::live(&w, team, g.date, g.level, g.teams, &g.stats.plays[..half], None);
        assert!(!part.is_empty() && part.len() < crowd.len(), "{level}: {} of {}", part.len(), crowd.len());
        assert_eq!(pairs(&part), pairs(&crowd[..part.len()]), "{level}: a prefix");
    }
}

#[test]
fn the_mood_follows_the_results_against_the_lines() {
    for level in ["hs", "college", "nfl"] {
        // Four home losses as two-touchdown favorites.
        let (mut w, team, opp, lv) = seat(level);
        let (_, i) = team.parts();
        for g in 0..4 {
            let s = GameStats { score: [10, 31], ..Default::default() };
            keep(&mut w, 2 + 7 * (3 - g), lv, [i, opp], s, Some(14.0));
        }
        let low = fans::week(&w, team).mood;
        assert!(low <= Mood::Low, "{level}: {low:?} after four upset losses");
        assert!(fans::game(&w, 3).unwrap().mood_after <= Mood::Low, "{level}");
        // Four home wins as two-touchdown dogs.
        let (mut w, team, opp, lv) = seat(level);
        for g in 0..4 {
            let s = GameStats { score: [31, 10], ..Default::default() };
            keep(&mut w, 2 + 7 * (3 - g), lv, [i, opp], s, Some(-14.0));
        }
        let high = fans::week(&w, team).mood;
        assert_eq!(high, Mood::High, "{level}: after four upset wins");
        // The first game's crowd walked in with what they carried; the
        // last one's with the run behind it.
        assert!(fans::game(&w, 3).unwrap().mood_before >= Mood::Good, "{level}");
    }
}

#[test]
fn a_crowd_says_every_version_before_it_repeats_one() {
    let who = Speaker { key: 7, tags: vec!["crowd", "nfl", "social", "modern"] };
    let f = Facts::new();
    let mut ch = Chorus::new(99);
    let key = "fans.crowd.score";
    let n = ch.fresh(key, &who, &f);
    assert!(n >= 4, "{n} versions fit");
    let mut said = Vec::new();
    for k in 0..n {
        assert_eq!(ch.fresh(key, &who, &f), n - k);
        said.push(ch.part(key, &who, &f).unwrap());
    }
    assert_eq!(ch.fresh(key, &who, &f), 0);
    said.sort();
    said.dedup();
    assert_eq!(said.len(), n, "no repeat before every version: {said:?}");
    // Then the one said longest ago comes back, and the crowd keeps going.
    assert!(ch.part(key, &who, &f).is_some());
    // Another crowd's memory is its own.
    let other = Speaker { key: 8, ..who };
    assert_eq!(ch.fresh(key, &other, &f), n);
}
