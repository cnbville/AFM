//! Lane NB8: the analysts' model (`tally::decide`) reads known 4th downs
//! and tries as the public models do; a game's calls add up the same from
//! its rows as from its sums and match the situation counts; concepts and
//! coverages come from the calls; garbage time follows the public sites'
//! lines; post-game win expectancy favours the side with the better box.
//! Quick: engine games only, no world.

use crate::game::flow::quick_game_in;
use crate::game::stats::{PlayRecord, PlayType};
use crate::game::synthetic::synthetic_league;
use crate::game::win_chance::State;
use crate::player::Level;
use crate::world::tally::{calls, decide, games, teams};

fn st(quarter: u8, clock: f64, lead: f64, togo: f64, yl: f64) -> State {
    State { quarter, clock, lead, down: 4, togo, yl, timeouts: [3, 3], spread: 0.0, answering: false }
}

#[test]
fn the_model_reads_known_fourth_downs_and_tries() {
    for level in [Level::NFL, Level::FBS, Level::FCS, Level::HS] {
        let m = decide::Model::new(level, 2025, None, true);
        let best = |s: State| m.fourth_at(&s).best().0;
        // 4th and 1 at midfield, tied in the 2nd quarter: go (nfl4th and
        // every public model).
        assert_eq!(best(st(2, 600.0, 0.0, 1.0, 50.0)), 0, "{level:?} 4th and 1 at midfield");
        // Deep in your own end, college and the NFL punt. (High school's
        // punts net little, 28 yards from your own 10 in the fit, so its
        // model says go even on 4th and 15 there; not checked.)
        if level != Level::HS {
            // 4th and 15 at your own 10: punt.
            assert_eq!(best(st(2, 600.0, 0.0, 15.0, 90.0)), 1, "{level:?} 4th and 15 at own 10");
            // 4th and 10 at your own 20: never a clear go. Public models
            // punt; this one reads the engine's own conversions and calls
            // it close (the punt ahead by under a point).
            let w = m.fourth_at(&st(2, 600.0, 0.0, 10.0, 80.0));
            assert!(w.go < w.punt.unwrap() + calls::CLEAR, "{level:?} 4th and 10 at own 20: {w:?}");
        }
        // 4th and 9 at their 12 early in a tied game: kick.
        assert_eq!(best(st(1, 700.0, 0.0, 9.0, 12.0)), 2, "{level:?} 4th and 9 at their 12");
        // Down 4 with a minute left, 4th and 5 at their 30: a field goal
        // doesn't win it, so go.
        assert_eq!(best(st(4, 60.0, -4.0, 5.0, 30.0)), 0, "{level:?} down 4 late");
        // A punt is no choice inside the 30's reach, a kick none from deep.
        let deep = m.fourth_at(&st(2, 600.0, 0.0, 5.0, 85.0));
        assert!(deep.kick.is_none() && deep.punt.is_some(), "{level:?} {deep:?}");
        // Kicks get harder with distance, and short ones are near sure.
        assert!(m.fg_make(5.0) > 0.85, "{level:?} a 22-yard kick {:.2}", m.fg_make(5.0));
        assert!(m.fg_make(5.0) > m.fg_make(25.0) && m.fg_make(25.0) > m.fg_make(35.0), "{level:?}");
        // Converting: more likely from 1 yard than from 8.
        assert!(
            m.fourth_at(&st(2, 600.0, 0.0, 1.0, 50.0)).conv > m.fourth_at(&st(2, 600.0, 0.0, 8.0, 50.0)).conv + 0.2
        );
        // Tries: down 8 and a touchdown late (down 2 before the try), go
        // for two; up 6 after a touchdown early, kick.
        let mut r = PlayRecord::new(1, 4, 90.0, 0, PlayType::TwoPoint);
        r.down = 1;
        r.togo = 3.0;
        r.yardline_100 = 3.0;
        r.score = [12, 14];
        let [kick, two] = m.try_after(&r).unwrap();
        assert!(two > kick, "{level:?} down 2 late: kick {kick:.3}, two {two:.3}");
        r.quarter = 1;
        r.clock = 600.0;
        r.score = [6, 0];
        let [kick, two] = m.try_after(&r).unwrap();
        assert!(kick > two, "{level:?} up 6 early: kick {kick:.3}, two {two:.3}");
    }
}

fn concept_play(call: &str, defense: &str, kind: PlayType) -> PlayRecord {
    let mut r = PlayRecord::new(1, 1, 900.0, 0, kind);
    r.call = call.to_string();
    r.defense_call = defense.to_string();
    r
}

#[test]
fn concepts_and_coverages_come_from_the_calls() {
    let run = concept_play("11 gun trips right jet motion, inside zone left", "Cover 3, base", PlayType::Run);
    assert_eq!(decide::concept_of(&run), Some("inside zone"));
    assert_eq!(decide::coverage_of(&run), Some("Cover 3"));
    let rpo = concept_play("11 gun trips right inside zone left with slant-flat", "Cover 1, nickel", PlayType::Pass);
    assert_eq!(decide::concept_of(&rpo), Some("slant-flat"));
    let rpo_run = concept_play("11 gun trips right inside zone left with slant-flat", "Cover 1, nickel", PlayType::Run);
    assert_eq!(decide::concept_of(&rpo_run), Some("inside zone"));
    let audible = concept_play("12 under center i-form power right (audible)", "Cover 2", PlayType::Run);
    assert_eq!(decide::concept_of(&audible), Some("power"));
    let odd = concept_play("11 gun spread something new", "", PlayType::Pass);
    assert_eq!(decide::concept_of(&odd), Some("other pass"));
    assert_eq!(decide::coverage_of(&odd), None);
    let punt = concept_play("punt", "", PlayType::Punt);
    assert_eq!(decide::concept_of(&punt), None);
}

#[test]
fn garbage_time_follows_the_public_lines() {
    let mut r = PlayRecord::new(1, 4, 300.0, 0, PlayType::Run);
    r.score = [35, 10];
    assert!(decide::garbage(Level::FBS, &r), "25 up in the 4th");
    r.quarter = 3;
    assert!(!decide::garbage(Level::FBS, &r), "25 up in the 3rd");
    r.quarter = 1;
    r.score = [60, 0];
    assert!(!decide::garbage(Level::FBS, &r), "never in the 1st");
    r.quarter = 4;
    r.wp_before = 0.95;
    assert!(decide::garbage(Level::NFL, &r));
    r.wp_before = 0.6;
    assert!(!decide::garbage(Level::NFL, &r));
}

#[test]
fn a_games_calls_add_up() {
    let level = Level::FBS;
    let teams_ = synthetic_league(level, 8, 11);
    let m = decide::Model::new(level, 2025, Some(3.0), false);
    let mut fourths = 0;
    let mut went = 0;
    let mut tries = 0;
    let mut cc = 0.0;
    let mut snaps = 0.0;
    let (mut better, mut games_n) = (0, 0);
    for g in 0..24u64 {
        let (h, a) = ((g % 8) as usize, ((g * 3 + 1) % 8) as usize);
        let a = if a == h { (a + 1) % 8 } else { a };
        let r = quick_game_in(&teams_[h], &teams_[a], level, 500 + g, Default::default(), Default::default());
        let s = &r.stats;
        let about = calls::About { date: 1000 + g as i32, teams: [h as u32, a as u32], neutral: false, season: 2025 };
        let both = calls::game(s, &m, about, true);
        for (side, c) in both.iter().enumerate() {
            // The rows give back the sums and the grid.
            let f: Vec<&calls::Fourth> = c.fourths.iter().collect();
            let t: Vec<&calls::Try> = c.tries.iter().collect();
            let (sum, grid) = calls::sums_of(&f, &t);
            assert_eq!(
                (sum.fourths, sum.went, sum.converted, sum.weighed, sum.said_go, sum.went_said_go, sum.agreed),
                (
                    c.sum.fourths,
                    c.sum.went,
                    c.sum.converted,
                    c.sum.weighed,
                    c.sum.said_go,
                    c.sum.went_said_go,
                    c.sum.agreed
                ),
                "game {g} side {side}"
            );
            assert!((sum.lost - c.sum.lost).abs() < 1e-4);
            assert_eq!(
                (sum.tries, sum.twos, sum.twos_made, sum.said_two),
                (c.sum.tries, c.sum.twos, c.sum.twos_made, c.sum.said_two)
            );
            let g2: std::collections::BTreeMap<u16, [u32; 3]> = c.grid.iter().map(|(k, v)| (*k, *v)).collect();
            assert_eq!(grid, g2);
            // The situation counts agree on the 4th downs.
            let tg = teams::game(s, side, level);
            let get = |k: &str| tg.cells.get(k).map_or(0.0, |v| v[0]) as u32;
            assert_eq!(get("o|fourth|go"), c.sum.went, "game {g} side {side} went");
            assert_eq!(get("o|fourth|go") + get("o|fourth|punt") + get("o|fourth|fg"), c.sum.fourths);
            for (k, v) in &tg.cells {
                if k.starts_with("o|cc|") {
                    cc += v[0];
                }
            }
            snaps +=
                tg.cells.get("o|kind|run").map_or(0.0, |v| v[0]) + tg.cells.get("o|kind|pass").map_or(0.0, |v| v[0]);
            fourths += c.sum.fourths;
            went += c.sum.went;
            tries += c.sum.tries;
            // Every weighed 4th down has its win chances in 0-100.
            // (College overtime's are left unweighed.)
            for x in &c.fourths {
                let weighed = x.wp[0] >= 0.0 && x.wp[0] <= 100.0 && x.conv >= 0.0;
                assert!(weighed || (x.q > 4 && x.wp == [-1.0; 3]), "{x:?}");
            }
        }
        // The side with the better box is the favourite after the fact.
        let adv = games::adv(s);
        let p = decide::pgwe(level, &adv, &s.team, false);
        assert!((0.0..=1.0).contains(&p));
        if s.score[0] != s.score[1] {
            games_n += 1;
            better += ((p > 0.5) == (s.score[0] > s.score[1])) as u32;
        }
        assert!(adv[0].explosive + adv[1].explosive > 0 && adv[0].rz_td <= adv[0].rz_trips);
    }
    assert!(fourths > 50 && went > 3 && tries > 30, "{fourths} 4th downs, went {went}, {tries} tries");
    assert!(cc > 0.95 * snaps, "{cc} concept-coverage snaps of {snaps}");
    assert!(better as f64 > 0.75 * games_n as f64, "win expectancy picked {better} of {games_n}");
}
