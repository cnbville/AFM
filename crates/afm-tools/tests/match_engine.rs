//! Development checks for the match engine (section 3, topic 10): games
//! between synthetic teams should land near real football from 2021-25
//! (nflverse and cfbfastR play-by-play, in `data/real-world/`). These are a
//! safeguard for engine changes only; the game never nudges results toward
//! them.
//!
//! The bands are wide on purpose: they catch an engine change that breaks
//! the game, not normal tuning. Where the engine is known to sit off the
//! real figure (see docs/match-engine.md), the band says so.

use afm_engine::game::quick_game;
use afm_engine::game::synthetic::synthetic_league;
use afm_engine::player::Level;
use rayon::prelude::*;

struct Totals {
    team_games: f64,
    points: f64,
    att: f64,
    cmp: f64,
    pass_yds: f64,
    int: f64,
    sacks: f64,
    carries: f64,
    rush_yds: f64,
    third: f64,
    third_conv: f64,
    best_rush_game: f64,
    best_pass_game: f64,
}

fn play(level: Level, games: usize, seed: u32) -> Totals {
    let teams = synthetic_league(level, 32, seed);
    let results: Vec<_> = (0..games)
        .into_par_iter()
        .map(|g| {
            let h = (g * 7) % 32;
            let a = (g * 13 + 5) % 32;
            let a = if a == h { (a + 1) % 32 } else { a };
            quick_game(&teams[h], &teams[a], level, seed as u64 * 1_000_003 + g as u64)
        })
        .collect();
    let mut t = Totals {
        team_games: 0.0,
        points: 0.0,
        att: 0.0,
        cmp: 0.0,
        pass_yds: 0.0,
        int: 0.0,
        sacks: 0.0,
        carries: 0.0,
        rush_yds: 0.0,
        third: 0.0,
        third_conv: 0.0,
        best_rush_game: 0.0,
        best_pass_game: 0.0,
    };
    for r in &results {
        for side in 0..2 {
            let ts = &r.stats.team[side];
            t.team_games += 1.0;
            t.points += ts.points as f64;
            t.sacks += ts.sacks as f64;
            t.third += ts.third_att as f64;
            t.third_conv += ts.third_conv as f64;
            for l in r.stats.players[side].values() {
                t.att += l.att as f64;
                t.cmp += l.cmp as f64;
                t.pass_yds += l.pass_yds;
                t.int += l.int as f64;
                t.carries += l.rush as f64;
                t.rush_yds += l.rush_yds;
                t.best_rush_game = t.best_rush_game.max(l.rush_yds);
                t.best_pass_game = t.best_pass_game.max(l.pass_yds);
            }
        }
    }
    t
}

fn within(name: &str, value: f64, lo: f64, hi: f64, real: f64) {
    println!("{name:<24} {value:>8.3}   band {lo:.3}-{hi:.3}   real {real:.3}");
    assert!(value >= lo && value <= hi, "{name} = {value:.3}, outside {lo:.3}-{hi:.3} (real {real:.3})");
}

#[test]
fn nfl_games_look_like_the_nfl() {
    let t = play(Level::NFL, 160, 3);
    // Narrowed from 18-30 with the attribute retune (2026-10-18), back to
    // 30 at the trial 8 fixes (2026-10-19): the agreed band is 20-25 for
    // the world, and these test rosters score about 26 (passing runs long:
    // 7.9 yards an attempt and touchdown passes well above real;
    // docs/match-engine.md, "Trial 8 fixes").
    within("points per team game", t.points / t.team_games, 20.0, 30.0, 22.5);
    within("completion %", t.cmp / t.att, 0.59, 0.71, 0.645);
    // Upper edge raised from 8.2 after the physics fixes (2026-10-14), when
    // NFL passing ran 8.2-8.6 a throw; since the attribute retune it runs
    // about 7.0-7.3.
    within("yards per attempt", t.pass_yds / t.att, 6.2, 8.8, 7.05);
    within("int rate", t.int / t.att, 0.008, 0.035, 0.023);
    // Known gap: these test rosters run short of the real 4.5 (about 3.4).
    // Runs are tuned on the world's rosters, which ran 0.7 yards a carry
    // above these in the NFL at trial 8 and 1.2 above them in FBS.
    within("yards per carry", t.rush_yds / t.carries.max(1.0), 3.0, 5.2, 4.5);
    within("sacks per team game", t.sacks / t.team_games, 0.9, 3.5, 2.41);
    within("third down conversion", t.third_conv / t.third, 0.32, 0.47, 0.398);
    // Outliers must happen (topic 10): over a few hundred team games
    // somebody has a big day.
    assert!(t.best_rush_game >= 120.0, "best rushing day only {:.0}", t.best_rush_game);
    assert!(t.best_pass_game >= 330.0, "best passing day only {:.0}", t.best_pass_game);
}

#[test]
fn fbs_games_look_like_fbs() {
    let t = play(Level::FBS, 160, 5);
    // Lower edge from 21 with the trial 8 fixes (2026-10-19): the clock
    // starts again on the ready signal after penalties and the chains, so
    // these slow test rosters (below) run about 63 plays and 21 points; the
    // world's FBS rosters 63 plays and 24.5 points. With throws at their
    // real depth and test staffs like the world's (minor bug 135,
    // 2026-10-31) these rosters scored about 18, and with the scrimmage
    // tackle reach (minor bug 162) about 21.
    within("points per team game", t.points / t.team_games, 18.0, 36.0, 27.0);
    // Known gap: a few points under the real 60.6%.
    within("completion %", t.cmp / t.att, 0.52, 0.67, 0.606);
    within("yards per attempt", t.pass_yds / t.att, 6.2, 8.6, 7.29);
    // These test rosters are slower than the world's (starting backs 4.78
    // in the forty, the world's 4.57) and run about 1.2 yards a carry below
    // them: about 3.9 here, 5.1 on trial 8's FBS rosters (real 5.12).
    within("yards per carry", t.rush_yds / t.carries.max(1.0), 3.3, 6.2, 5.12);
    // About 0.33 here since the trial 8 fixes (the world's FBS rosters 0.38).
    within("third down conversion", t.third_conv / t.third, 0.29, 0.47, 0.403);
}
