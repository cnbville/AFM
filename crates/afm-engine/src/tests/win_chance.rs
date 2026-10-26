//! The game boards' records (update 2, lane 9): the win chance is sane at
//! known states, rises with the score and the field and is calibrated on
//! each level's engine games; the line chart is kept on your games only
//! and changes nothing in them; the film keeps each throw; your games keep
//! the names of players who leave; saves written before lane 9 load.

use std::cmp::Ordering;

use crate::game::synthetic::synthetic_league;
use crate::game::win_chance::{Params, State, chance, home_before, home_series};
use crate::game::{GameResult, GameSetup, PlayRecord, PlayType, play_game};
use crate::player::Level;
use crate::world::charts::{Charts, Tally, depth_bin};
use crate::world::tape::{Snap, Throw};

/// The most the side behind may gain in one second (see
/// `win_chance_runs_down_with_the_clock`).
const RISE: f64 = 0.012;

const LEVELS: [Level; 5] = [Level::NFL, Level::FBS, Level::FCS, Level::SmallCollege, Level::HS];

fn st(quarter: u8, clock: f64, lead: f64, down: u8, togo: f64, yl: f64) -> State {
    State { quarter, clock, lead, down, togo, yl, timeouts: [3, 3], spread: 0.0, answering: false }
}

#[test]
fn win_chance_reads_known_states() {
    for level in LEVELS {
        let p = Params::for_level(level);
        let q = p.quarter;
        // The opening snap from the 25, no line: about a coin flip, the ball
        // a small edge.
        let open = chance(&p, &st(1, q, 0.0, 1, 10.0, 75.0));
        assert!((0.49..0.58).contains(&open), "{level:?} opening snap {open:.3}");
        // A pregame favourite by 7 starts well ahead, not certain.
        let fav = chance(&p, &State { spread: 7.0, ..st(1, q, 0.0, 1, 10.0, 75.0) });
        assert!(fav > open + 0.1 && fav < 0.85, "{level:?} favourite by 7 {fav:.3}");
        // Up 7 with the ball, a minute left, the other side out of timeouts:
        // he kneels it out.
        let kneel = chance(&p, &State { timeouts: [3, 0], ..st(4, 60.0, 7.0, 1, 10.0, 70.0) });
        assert!(kneel > 0.98, "{level:?} kneel-out {kneel:.3}");
        // Down 14 with two minutes left: almost lost.
        let late = chance(&p, &st(4, 120.0, -14.0, 1, 10.0, 75.0));
        assert!(late < 0.04, "{level:?} down 14 with 2:00 left {late:.3}");
        // Down 3, first and goal at the 10 with 20 seconds and three
        // timeouts: time for three or four snaps at the end zone, and a
        // short kick to tie and go to overtime if they fail.
        let close = chance(&p, &st(4, 20.0, -3.0, 1, 10.0, 10.0));
        assert!((0.5..0.88).contains(&close), "{level:?} down 3 at the 10, 0:20 {close:.3}");
        // Up 10 with the ball to start the second half: well ahead.
        let half = chance(&p, &st(3, q, 10.0, 1, 10.0, 75.0));
        assert!((0.7..0.95).contains(&half), "{level:?} up 10 at the half {half:.3}");
    }
}

#[test]
fn win_chance_rises_with_the_score_and_the_field() {
    for level in LEVELS {
        let p = Params::for_level(level);
        for (qtr, clock) in [(1, p.quarter), (2, 300.0), (2, 30.0), (3, 600.0), (4, 400.0), (4, 90.0), (4, 15.0)] {
            let mut last = 0.0;
            for lead in -24..=24 {
                let c = chance(&p, &st(qtr, clock, lead as f64, 1, 10.0, 60.0));
                assert!(
                    c + 1e-9 >= last,
                    "{level:?} Q{qtr} {clock}s: lead {lead} gives {c:.4}, one less gave {last:.4}"
                );
                last = c;
            }
            for lead in [-7.0, -3.0, 0.0, 3.0] {
                let mut last = 0.0;
                for yl in (1..=99).rev() {
                    let yl = yl as f64;
                    let c = chance(&p, &st(qtr, clock, lead, 1, yl.min(10.0), yl));
                    // The expected points table is real data and has
                    // small kinks: never down by a point.
                    assert!(
                        c + 0.01 >= last,
                        "{level:?} Q{qtr} {clock}s lead {lead}: the {yl} gives {c:.4}, a yard back gave {last:.4}"
                    );
                    last = last.max(c);
                }
            }
        }
    }
}

/// The last second (R12): a kick only from range, as often as it is good;
/// past the longest kick, nothing but a Hail Mary, and no cliff a yard
/// apart.
#[test]
fn win_chance_late_kicks_need_range() {
    for level in LEVELS {
        let p = Params::for_level(level);
        let at = |yl: f64| chance(&p, &State { timeouts: [0, 0], ..st(4, 1.0, -2.0, 1, 10.0, yl) });
        let mid = at(50.0);
        assert!(mid < 0.08, "{level:?} down 2 at the 50 with 0:01 {mid:.3}");
        for yl in 20..70 {
            let (yl, c) = (yl as f64, at(yl as f64));
            let kick = p.fg_make(yl);
            if kick > 0.02 {
                assert!((c - kick).abs() < 0.05, "{level:?} 0:01 at the {yl}: {c:.3}, the kick makes {kick:.3}");
            } else {
                // Past the longest kick: only a throw at the end zone.
                let shot = 0.55 * (-yl / 14.0).exp();
                assert!(c < shot + 0.01, "{level:?} 0:01 at the {yl}, out of range: {c:.3}, the throw {shot:.3}");
                assert!((c - at(yl + 1.0)).abs() < 0.03, "{level:?} 0:01: a cliff past the {yl}");
            }
        }
    }
}

/// The fourth quarter for the side behind (R13): no step at any moment
/// (the old one came at 5:00). Closer in, the chance may rise a little as
/// the clock runs, as in real play: the drive can't use all the time, so
/// the other side gets less to answer. Never by more than about a point a
/// second. From its own 25 the side behind can use six minutes itself, so
/// 5:00 is never better than 6:00.
#[test]
fn win_chance_runs_down_with_the_clock() {
    for level in LEVELS {
        let p = Params::for_level(level);
        let q = p.quarter as i32;
        for lead in [-17.0, -14.0, -10.0, -8.0, -7.0, -6.0, -5.0, -3.0, -2.0, -1.0] {
            for (yl, to) in [(75.0, [3, 3]), (50.0, [2, 3]), (30.0, [1, 2]), (75.0, [0, 3])] {
                let at = |t: f64| chance(&p, &State { timeouts: to, ..st(4, t, lead, 1, 10.0, yl) });
                let mut prev: Option<f64> = None;
                for t in (15..=q).rev() {
                    let c = at(t as f64);
                    if let Some(b) = prev {
                        assert!(c - b <= RISE, "{level:?} down {lead} at the {yl} {to:?}: {b:.4} to {c:.4} at {t}s");
                        assert!((c - b).abs() <= 0.02, "{level:?} down {lead} at the {yl} {to:?}: a step at {t}s");
                    }
                    prev = Some(c);
                }
                if yl >= 75.0 {
                    let (a, b) = (at(360.0), at(300.0));
                    assert!(b <= a + 0.005, "{level:?} down {lead} at the {yl} {to:?}: 6:00 {a:.3}, 5:00 {b:.3}");
                }
            }
        }
    }
}

/// An overtime snap for the log tests, from a real record.
fn ot_play(base: &PlayRecord, kind: PlayType, offense: usize, score: [i32; 2], yl: f64) -> PlayRecord {
    let mut r = base.clone();
    r.kind = kind;
    r.offense = offense;
    r.quarter = 5;
    r.clock = 1.0e9;
    r.score = score;
    r.yardline_100 = yl;
    r.down = 1;
    r.togo = yl.min(10.0);
    r.turnover = false;
    r.touchdown = false;
    r.kick_good = false;
    r.nullified = false;
    r.timeouts = Some([1, 1]);
    r
}

/// College and high school overtime (R14): the side that answers knows
/// what it needs, and a try hands the other side the ball at the spot.
#[test]
fn win_chance_reads_overtime_possessions() {
    for level in [Level::FBS, Level::FCS, Level::HS] {
        let p = Params::for_level(level);
        let spot = p.ot_spot.unwrap();
        let ot = |lead: f64, answering: bool| {
            chance(&p, &State { answering, ..st(5, 1.0e9, lead, 1, 10.0_f64.min(spot), spot) })
        };
        let first = ot(0.0, false);
        assert!((0.4..0.6).contains(&first), "{level:?} the first possession, level: {first:.3}");
        let answer = ot(0.0, true);
        assert!(answer > 0.8, "{level:?} answering level: any score wins, {answer:.3}");
        assert!(ot(-7.0, true) < ot(-3.0, true) && ot(-3.0, true) < answer, "{level:?} answering behind");
        // A log: the home side kicks a field goal, the away side answers
        // with one; round two the away side goes first, scores a
        // touchdown and a two, and the home side answers.
        let teams = synthetic_league(level, 4, 78);
        let r = play_game(&GameSetup::new(&teams[0], &teams[1], level, 33_001));
        let base = r.stats.plays.iter().find(|x| x.kind == PlayType::Run).expect("a run").clone();
        let mut fg = ot_play(&base, PlayType::FieldGoal, 0, [0, 0], spot - 4.0);
        fg.kick_good = true;
        let mut fg2 = ot_play(&base, PlayType::FieldGoal, 1, [3, 0], spot - 4.0);
        fg2.kick_good = true;
        let mut td = ot_play(&base, PlayType::Run, 1, [3, 3], 4.0);
        td.touchdown = true;
        let mut two = ot_play(&base, PlayType::TwoPoint, 1, [3, 9], 3.0);
        two.touchdown = true;
        let log = vec![
            ot_play(&base, PlayType::Run, 0, [0, 0], spot),
            fg,
            ot_play(&base, PlayType::Run, 1, [3, 0], spot),
            fg2,
            ot_play(&base, PlayType::Run, 1, [3, 3], spot),
            td,
            two,
            ot_play(&base, PlayType::Run, 0, [3, 11], spot),
        ];
        let w = home_series(&p, &log, 0.0);
        assert!((0.4..0.6).contains(&w[0]), "{level:?} home's first snap {:.3}", w[0]);
        // Away answers a field goal: it needs one to go on, a touchdown to
        // win (likelier from high school's 10 than college's 25).
        let lo = if spot < 20.0 { 0.08 } else { 0.15 };
        assert!(w[2] > lo && w[2] < 0.6, "{level:?} away answering 3 down: home {:.3}", w[2]);
        // Round two, level, away first again: no one is answering yet.
        assert!((0.35..0.65).contains(&w[4]), "{level:?} round two's first snap: home {:.3}", w[4]);
        // The two, then home's first snap from the spot: before it, home
        // still had a chance (a miss leaves it a touchdown to win); after
        // the made two, home answering 8 down is nearly lost.
        assert!(w[6] > 0.15 && w[6] < 0.5, "{level:?} the two read home {:.3}", w[6]);
        assert!(w[7] < 0.1, "{level:?} home answering 8 down {:.3}", w[7]);
        // A miss on the first possession: the answer needs any score.
        let mut miss = ot_play(&base, PlayType::FieldGoal, 0, [0, 0], spot - 4.0);
        miss.kick_good = false;
        let log =
            vec![ot_play(&base, PlayType::Run, 0, [0, 0], spot), miss, ot_play(&base, PlayType::Run, 1, [0, 0], spot)];
        let w = home_series(&p, &log, 0.0);
        assert!(w[2] < 0.2, "{level:?} away answering a miss: home {:.3}", w[2]);
        // The first side's touchdown and kick: the kick read about where
        // the answer's first snap is, not as a kickoff from deep.
        let mut td = ot_play(&base, PlayType::Run, 0, [0, 0], 3.0);
        td.touchdown = true;
        let mut xp = ot_play(&base, PlayType::ExtraPoint, 0, [6, 0], 3.0);
        xp.kick_good = true;
        let log = vec![td, xp, ot_play(&base, PlayType::Run, 1, [7, 0], spot)];
        let w = home_series(&p, &log, 0.0);
        assert!((w[1] - w[2]).abs() < 0.08, "{level:?} the kick read {:.3}, the answer's first snap {:.3}", w[1], w[2]);
    }
}

/// Engine games between unequal teams, with no line (the score so far
/// hints which side is better) and the home field as the boards read it:
/// how close the chance before each snap is to who won. Each pairing plays
/// both ways, so the sample's home sides are no stronger than its away
/// sides. High school's bins are shown, not held: this synthetic league's
/// margins (about 36 points from even) are far wider than real high
/// school's (about 25), so a lead there decides more than it does in life.
#[test]
fn win_chance_is_calibrated_on_engine_games() {
    for (level, seed) in [(Level::NFL, 5u32), (Level::FBS, 6), (Level::HS, 7)] {
        let teams = synthetic_league(level, 12, seed);
        let p = Params::for_level(level).without_line();
        let home = if level == Level::NFL { 1.7 } else { 2.5 };
        let (mut brier, mut n) = (0.0f64, 0u32);
        let mut bins = [(0.0f64, 0.0f64, 0u32); 5];
        for g in 0..60u64 {
            let k = g / 2;
            let (h, a) = ((k * 7) as usize % 12, (k * 5 + 1) as usize % 12);
            let a = if a == h { (a + 1) % 12 } else { a };
            let (h, a) = if g % 2 == 0 { (h, a) } else { (a, h) };
            let r = play_game(&GameSetup::new(&teams[h], &teams[a], level, 900 + g * 31));
            let won = match r.stats.score[0].cmp(&r.stats.score[1]) {
                Ordering::Greater => 1.0,
                Ordering::Less => 0.0,
                Ordering::Equal => 0.5,
            };
            for pl in r.stats.plays.iter().filter(|x| x.offense <= 1) {
                let x = home_before(&p, pl, home);
                assert!((0.0..=1.0).contains(&x), "{level:?}: {x}");
                brier += (x - won).powi(2);
                n += 1;
                let b = ((x * 5.0) as usize).min(4);
                bins[b].0 += x;
                bins[b].1 += won;
                bins[b].2 += 1;
            }
        }
        let brier = brier / n as f64;
        println!("{level:?}: Brier {brier:.3} over {n} snaps");
        assert!(brier < 0.19, "{level:?}: Brier score {brier:.3}");
        for (b, &(sx, sw, c)) in bins.iter().enumerate() {
            if c < 300 {
                continue;
            }
            let (said, came) = (sx / c as f64, sw / c as f64);
            println!("  bin {b}: said {said:.2}, won {came:.2} over {c} snaps");
            if level != Level::HS {
                assert!((said - came).abs() < 0.15, "{level:?} bin {b}: said {said:.2}, won {came:.2} over {c} snaps");
            }
        }
    }
}

/// What a game is, line chart aside.
fn shape(r: &GameResult) -> Vec<(u32, String, i64, [i32; 2])> {
    r.stats.plays.iter().map(|p| (p.n, format!("{:?}", p.kind), (p.yards * 100.0).round() as i64, p.score)).collect()
}

#[test]
fn only_kept_games_chart_the_line_and_charting_changes_nothing() {
    for level in [Level::NFL, Level::HS] {
        let teams = synthetic_league(level, 4, 41);
        for g in 0..3u64 {
            let mut s = GameSetup::new(&teams[g as usize], &teams[(g as usize + 1) % 4], level, 5_100 + g);
            let off = play_game(&s);
            assert!(off.stats.plays.iter().all(|p| p.line.is_none()), "{level:?}: a game nobody keeps charts nothing");
            s.keep = true;
            let on = play_game(&s);
            assert_eq!(off.stats.score, on.stats.score, "{level:?} game {g}: charting changed the score");
            assert!(shape(&off) == shape(&on), "{level:?} game {g}: charting changed the plays");
            assert!(on.stats.plays.iter().all(|p| p.timeouts.is_some()), "every snap keeps the timeouts");
            let drops: Vec<&PlayRecord> = on
                .stats
                .plays
                .iter()
                .filter(|p| matches!(p.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble) && !p.nullified)
                .collect();
            let charted = drops
                .iter()
                .filter(|p| {
                    p.line.as_ref().is_some_and(|c| !c.reps.is_empty() && !c.rush.is_empty() && c.blockers >= 5)
                })
                .count();
            assert!(charted * 10 >= drops.len() * 9, "{level:?}: {charted} of {} dropbacks charted", drops.len());
            let runs = on.stats.plays.iter().filter(|p| p.kind == PlayType::Run && !p.nullified);
            assert!(runs.clone().all(|p| p.line.is_some()), "runs keep their blocks");
            // The reps name the men on the field.
            let ids: std::collections::HashSet<u32> =
                teams.iter().flat_map(|t| t.players.iter().map(|p| p.id)).collect();
            for c in on.stats.plays.iter().filter_map(|p| p.line.as_ref()) {
                for r in &c.reps {
                    assert!(ids.contains(&r.blocker) && ids.contains(&r.defender), "a rep names two players");
                    assert!(r.mv <= 2 && r.bpos <= 7);
                }
                assert!(c.rush.iter().all(|r| r.gap <= 4 && ids.contains(&r.id)));
            }
        }
    }
}

#[test]
fn the_film_keeps_each_throw_and_the_tally_counts_them() {
    let teams = synthetic_league(Level::FBS, 4, 52);
    let r = play_game(&GameSetup::new(&teams[0], &teams[1], Level::FBS, 7_001));
    let mut passes = 0;
    for p in &r.stats.plays {
        let t = Throw::of(p);
        assert_eq!(t.is_some(), p.kind == PlayType::Pass, "{:?}", p.kind);
        if let Some(t) = t {
            passes += 1;
            assert!(t.from <= 107 && t.to <= 107);
            assert_eq!(t.air as f64, p.air_yards.round().clamp(-20.0, 99.0));
            assert_eq!(t.away, p.throw_away);
        }
    }
    assert!(passes > 30, "{passes} passes");
    // The level's tally: both sides, throws at a man by depth.
    let mut t = Tally::default();
    t.add_game(&r.stats);
    assert_eq!(t.games, 2);
    let at_man =
        r.stats.plays.iter().filter(|p| p.kind == PlayType::Pass && !p.throw_away && !p.nullified && p.offense <= 1);
    assert_eq!(t.throws as usize, at_man.clone().count());
    assert_eq!(t.depth.iter().sum::<u32>(), t.throws);
    for p in at_man {
        assert!(depth_bin(p.air_yards) < 7);
    }
    assert!(t.dropbacks >= t.throws && t.pressured <= t.dropbacks && t.sacks <= t.dropbacks);
    // A season's charts keep the level's numbers by year.
    let mut c = Charts::default();
    c.add(2026, 1, &t);
    c.add(2026, 1, &t);
    assert_eq!(c.get(2026, 1).map(|x| x.games), Some(4));
    assert!(c.get(2025, 1).is_none());
}

/// A save written before lane 9 has no timeouts, line charts, throws or
/// charts: each reads back empty.
#[test]
fn saves_from_before_the_boards_load() {
    let teams = synthetic_league(Level::NFL, 4, 61);
    let mut s = GameSetup::new(&teams[0], &teams[1], Level::NFL, 8_001);
    s.keep = true;
    let r = play_game(&s);
    let rec = r.stats.plays.iter().find(|p| p.kind == PlayType::Pass && p.line.is_some()).expect("a charted pass");
    let mut v = serde_json::to_value(rec).unwrap();
    let m = v.as_object_mut().unwrap();
    assert!(m.remove("timeouts").is_some() && m.remove("line").is_some());
    let old = rmp_serde::to_vec_named(&v).unwrap();
    let back: PlayRecord = rmp_serde::from_slice(&old).expect("an old play reads");
    assert!(back.timeouts.is_none() && back.line.is_none());
    assert_eq!(back.yards, rec.yards);
    // Old film: no throws.
    let tape = crate::world::tape::Tape::of(
        crate::calendar::Date::ymd(2026, 9, 5),
        crate::calendar::Level::Nfl,
        [0, 1],
        ["Home".into(), "Away".into()],
        &r.stats,
        &[Default::default(), Default::default()],
    );
    let snap = tape.snaps.iter().find(|x| x.throw.is_some()).expect("a throw on film");
    let mut v = serde_json::to_value(snap).unwrap();
    v.as_object_mut().unwrap().remove("throw");
    let back: Snap = rmp_serde::from_slice(&rmp_serde::to_vec_named(&v).unwrap()).expect("an old snap reads");
    assert!(back.throw.is_none());
    // A world without charts reads them empty.
    let empty = rmp_serde::to_vec_named(&serde_json::json!({})).unwrap();
    let c: Charts = rmp_serde::from_slice(&empty).expect("empty charts read");
    assert!(c.get(2026, 1).is_none() && c.state_polls.is_empty());
}

#[test]
fn your_games_keep_the_names_of_players_who_leave() {
    use crate::world::seat::SeatGame;
    use crate::world::{World, WorldOptions, seat_names};
    let mut w = World::empty(WorldOptions { scale: 0.001, ..Default::default() });
    w.add_class_aged(17);
    let (a, b) = (w.people[0].player.id, w.people[1].player.id);
    let mut stats = crate::game::GameStats::default();
    stats.line(0, a).games = 1;
    stats.line(1, b).games = 1;
    let g = SeatGame {
        date: crate::calendar::Date::ymd(2026, 9, 5),
        level: crate::calendar::Level::HighSchool,
        teams: [0, 1],
        names: ["Home".into(), "Away".into()],
        stats,
        replay: Vec::new(),
        marks: Vec::new(),
        gone: Vec::new(),
        spread: None,
    };
    // A game kept before the names were: it reads with none kept (an
    // empty box score, as JSON keys a box score's ids as text).
    let mut v = serde_json::to_value(SeatGame { stats: Default::default(), ..g.clone() }).unwrap();
    v.as_object_mut().unwrap().remove("gone");
    let old: SeatGame = rmp_serde::from_slice(&rmp_serde::to_vec_named(&v).unwrap()).expect("an old game reads");
    assert!(old.gone.is_empty());
    w.seat_games.push(g);
    // The first man leaves football, the second stays.
    let name = w.people[0].name.clone();
    let mut map: Vec<Option<usize>> = (0..w.people.len()).map(Some).collect();
    map[0] = None;
    seat_names::keep(&mut w, &map);
    seat_names::keep(&mut w, &map);
    let g = &w.seat_games[0];
    assert_eq!(g.gone.len(), 1, "kept once, and only the man who left");
    assert_eq!(seat_names::gone(g, a).map(|x| x.name.as_str()), Some(name.as_str()));
    assert!(seat_names::gone(g, b).is_none());
}
