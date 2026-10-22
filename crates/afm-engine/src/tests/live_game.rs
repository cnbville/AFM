//! Game day control (section 9): a live game you hand to your staff
//! plays exactly as the one-loop game; your calls and the officials work.

use crate::game::OffScheme;
use crate::game::live::{self, Ask, Input, LiveGame, Params, Settings, StopMode, TossChoice};
use crate::game::synthetic::synthetic_team;
use crate::game::{GameResult, GameSetup, play_game};
use crate::player::Level;

fn teams(level: Level, seed: u32) -> (crate::game::GameTeam, crate::game::GameTeam) {
    (
        synthetic_team("Home", level, 0.55, OffScheme::WestCoast, seed),
        synthetic_team("Away", level, 0.5, OffScheme::SpreadOption, seed.wrapping_add(77)),
    )
}

fn same(a: &GameResult, b: &GameResult) {
    assert_eq!(a.stats.score, b.stats.score);
    assert_eq!(a.stats.plays.len(), b.stats.plays.len());
    for (x, y) in a.stats.plays.iter().zip(&b.stats.plays) {
        assert_eq!(x.why, y.why);
        assert_eq!(x.yards, y.yards);
        assert_eq!(x.clock, y.clock);
    }
    assert_eq!(a.injuries.len(), b.injuries.len());
}

#[test]
fn a_delegated_live_game_matches_the_one_loop_game() {
    for (level, seed) in [(Level::NFL, 11u64), (Level::FBS, 12), (Level::HS, 13), (Level::NFL, 14)] {
        let (home, away) = teams(level, seed as u32);
        let setup = GameSetup::new(&home, &away, level, seed);
        let whole = play_game(&setup);
        for team in 0..2 {
            let settings = Settings { stop: StopMode::EverySnap, ..Settings::default() };
            let mut live = LiveGame::new(home.clone(), away.clone(), Params::of(&setup), team, settings);
            let mut stops = 0;
            let mut step = live.run();
            while step.pending.is_some() {
                stops += 1;
                assert!(stops < 2000, "the game never ends");
                step = live.answer(Input::Staff);
            }
            same(&whole, &step.result);
            assert!(stops > 40, "every snap of yours stops: {stops}");
            // The answers alone rebuild the game (a save mid-game).
            let mut again = live.clone();
            again.ctl.settings.stop = StopMode::Off;
            same(&whole, &again.run().result);
        }
    }
}

#[test]
fn your_calls_and_choices_change_the_game() {
    let level = Level::NFL;
    let (home, away) = teams(level, 21);
    let setup = GameSetup::new(&home, &away, level, 21);
    let whole = play_game(&setup);
    let settings = Settings { stop: StopMode::EverySnap, ..Settings::default() };
    let mut live = LiveGame::new(home.clone(), away.clone(), Params::of(&setup), 0, settings);
    let mut step = live.run();
    let mut kinds = Vec::new();
    let mut n = 0;
    while let Some(p) = step.pending.clone() {
        n += 1;
        kinds.push(p.ask);
        let input = match p.ask {
            Ask::Toss => Input::Toss(TossChoice::Receive),
            // Always the second choice from the box: the play caller's
            // next call.
            Ask::Snap if p.staff_fourth.is_none() && !p.off_options.is_empty() => {
                Input::Call { fourth: None, offense: Some(p.off_options[0]), defense: None }
            }
            Ask::Defense if !p.def_options.is_empty() => {
                Input::Call { fourth: None, offense: None, defense: Some(p.def_options[0]) }
            }
            _ => Input::Staff,
        };
        step = live.answer(input);
        assert!(n < 2000);
    }
    let r = step.result;
    let mine = r.stats.plays.iter().filter(|p| p.yours != 0).count();
    assert!(mine > 30, "your calls are on the record: {mine}");
    assert!(r.stats.plays.iter().any(|p| p.yours != 0 && !p.staff_call.is_empty()));
    assert_ne!(
        r.stats.plays.iter().map(|p| p.why.clone()).collect::<Vec<_>>(),
        whole.stats.plays.iter().map(|p| p.why.clone()).collect::<Vec<_>>()
    );
    assert!(kinds.contains(&Ask::Snap) && kinds.contains(&Ask::Defense) && kinds.contains(&Ask::Halftime));
    // The same answers give the same game again.
    let mut again = live.clone();
    again.ctl.settings.stop = StopMode::Off;
    same(&r, &again.run().result);
}

#[test]
fn a_timeout_stops_the_clock_and_the_snap_asks_again() {
    let level = Level::FBS;
    let (home, away) = teams(level, 31);
    let setup = GameSetup::new(&home, &away, level, 31);
    let settings = Settings { stop: StopMode::EverySnap, ..Settings::default() };
    let params = Params::of(&setup);
    let mut live = LiveGame::new(home.clone(), away.clone(), params, 1, settings);
    let mut step = live.run();
    // Play to the first snap of yours with the clock running.
    let mut guard = 0;
    while let Some(p) = &step.pending {
        guard += 1;
        assert!(guard < 500);
        if matches!(p.ask, Ask::Snap | Ask::Defense) && p.view.running && p.view.timeouts[1] == 3 {
            break;
        }
        step = live.answer(Input::Staff);
    }
    let before = step.pending.clone().expect("a running-clock snap");
    step = live.answer(Input::Timeout);
    let after = step.pending.clone().expect("the same snap asks again");
    assert_eq!(after.view.plays, before.view.plays);
    assert_eq!(after.view.timeouts[1], 2);
    assert!(!after.view.running);
    assert!(step.result.log.iter().any(|l| l.2.contains("Your timeout")));
}

/// The officials over many games: close calls, wrong rulings, reviews and
/// challenges at about real NFL rates.
#[test]
fn officials_make_mistakes_and_replay_fixes_some() {
    let level = Level::NFL;
    let (mut close, mut wrong, mut challenges, mut won, mut booth, mut booth_rev, mut left_wrong) =
        (0, 0, 0, 0, 0, 0, 0);
    let games = 120;
    for g in 0..games {
        let (home, away) = teams(level, 400 + g);
        let r = play_game(&GameSetup::new(&home, &away, level, 900 + g as u64));
        for p in &r.stats.plays {
            if let Some(o) = &p.officiating {
                close += 1;
                wrong += o.wrong as u32;
                match o.reviewer {
                    live::Reviewer::Challenge(_) => {
                        challenges += 1;
                        won += o.overturned as u32;
                    }
                    live::Reviewer::Booth => {
                        booth += 1;
                        booth_rev += o.overturned as u32;
                    }
                    _ => {}
                }
                if o.wrong && !o.overturned {
                    left_wrong += 1;
                }
            }
        }
    }
    let per = |x: u32| x as f64 / games as f64;
    eprintln!(
        "per game: noted {:.2}, wrong {:.2}, challenges {:.2} (won {:.0}%), booth {:.2} (reversed {:.0}%), wrong left {:.2}",
        per(close),
        per(wrong),
        per(challenges),
        100.0 * won as f64 / challenges.max(1) as f64,
        per(booth),
        100.0 * booth_rev as f64 / booth.max(1) as f64,
        per(left_wrong)
    );
    assert!((0.5..1.4).contains(&per(wrong)), "wrong rulings a game");
    assert!((0.2..0.9).contains(&per(challenges)), "challenges a game");
    let rate = won as f64 / challenges.max(1) as f64;
    assert!((0.2..0.65).contains(&rate), "challenges won");
}

#[test]
fn the_toss_winner_chooses_and_high_school_has_no_replay() {
    let level = Level::HS;
    let (home, away) = teams(level, 41);
    let r = play_game(&GameSetup::new(&home, &away, level, 41));
    assert!(r.toss.is_some());
    assert!(r.stats.plays.iter().filter_map(|p| p.officiating.as_ref()).all(|o| o.reviewer == live::Reviewer::None));
    assert_eq!(live::Replay::of(Level::NFL, 1998).challenges, 0);
    assert_eq!(live::Replay::of(Level::NFL, 2010).challenges, 2);
    assert!(live::Replay::of(Level::NFL, 2024).assist);
    assert!(live::Replay::of(Level::FBS, 2020).booth_all);
}
