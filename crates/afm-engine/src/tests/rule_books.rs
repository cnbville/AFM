//! Each level's rule book for fouls and kicks (engine gaps lane EG5,
//! items 23-25, `game/fouls.rs`): the spec's examples, by book and era.

use crate::game::Rules;
use crate::game::fouls::{self, Fouls, Play};
use crate::game::sim::{Foul, FoulKind};
use crate::player::Level;

fn foul(kind: FoulKind, side: usize, at: f64) -> Foul {
    Foul { kind, side, player: 1, at }
}

/// A pass play from `los` (the offense's frame) that fell incomplete.
fn incomplete(los: f64) -> Play {
    Play { los, end: los, goal: 100.0, run: false, complete: false, turnover: false, crossed: true }
}

#[test]
fn penalty_yards_and_first_downs_by_book_and_era() {
    use FoulKind as K;
    let p = |b: Fouls, y: i32, k: FoulKind, side: usize, crossed: bool| {
        let x = fouls::penalty(b, y, k, side, crossed);
        (x.yards, x.first_down)
    };
    // Defensive holding: the NFL's 5 and a first down; college's 10, a
    // first down on every call from 2022 and before only on a pass that
    // crossed the line; high school's 10 and none.
    assert_eq!(p(Fouls::Nfl, 2025, K::DefensiveHolding, 1, false), (5.0, true));
    assert_eq!(p(Fouls::Ncaa, 2023, K::DefensiveHolding, 1, false), (10.0, true));
    assert_eq!(p(Fouls::Ncaa, 2019, K::DefensiveHolding, 1, false), (10.0, false));
    assert_eq!(p(Fouls::Ncaa, 2019, K::DefensiveHolding, 1, true), (10.0, true));
    assert_eq!(p(Fouls::Nfhs, 2025, K::DefensiveHolding, 1, true), (10.0, false));
    // Offensive pass interference: 10 in the NFL, 15 below it.
    assert_eq!(p(Fouls::Nfl, 2025, K::OffensivePassInterference, 0, true).0, 10.0);
    assert_eq!(p(Fouls::Ncaa, 2025, K::OffensivePassInterference, 0, true).0, 15.0);
    assert_eq!(p(Fouls::Nfhs, 2025, K::OffensivePassInterference, 0, true).0, 15.0);
    // NFHS pass interference: a first down only through 2012.
    assert!(p(Fouls::Nfhs, 2012, K::PassInterference, 1, true).1);
    assert!(!p(Fouls::Nfhs, 2013, K::PassInterference, 1, true).1);
    // NFHS: a personal foul is 15 yards without the first down; roughing
    // the passer keeps it.
    assert_eq!(p(Fouls::Nfhs, 2025, K::UnnecessaryRoughness, 1, false), (15.0, false));
    assert_eq!(p(Fouls::Nfhs, 2025, K::RoughingPasser, 1, false), (15.0, true));
    assert_eq!(p(Fouls::Ncaa, 2025, K::FaceMask, 1, false), (15.0, true));
    // Illegal use of hands: the offense's 10; the defense's the NFL's 5 and
    // a first down, holding's 10 elsewhere.
    assert_eq!(p(Fouls::Nfl, 2025, K::IllegalUseOfHands, 0, false), (10.0, false));
    assert_eq!(p(Fouls::Nfl, 2025, K::IllegalUseOfHands, 1, false), (5.0, true));
    assert_eq!(p(Fouls::Ncaa, 2025, K::IllegalUseOfHands, 1, false), (10.0, true));
    // Twelve men: 5 yards, illegal participation's 15 under NFHS.
    assert_eq!(p(Fouls::Ncaa, 2025, K::TooManyMen, 1, false).0, 5.0);
    assert_eq!(p(Fouls::Nfhs, 2025, K::TooManyMen, 1, false).0, 15.0);
    // Grounding costs the down at every level.
    assert!(fouls::penalty(Fouls::Ncaa, 2025, K::IntentionalGrounding, 0, false).loss_of_down);
    // Unsportsmanlike conduct: 15 yards; the defense's a first down in the
    // NFL and college, not under NFHS rules.
    assert_eq!(p(Fouls::Nfl, 2025, K::UnsportsmanlikeConduct, 1, false), (15.0, true));
    assert_eq!(p(Fouls::Ncaa, 2025, K::UnsportsmanlikeConduct, 1, false), (15.0, true));
    assert_eq!(p(Fouls::Nfhs, 2025, K::UnsportsmanlikeConduct, 1, false), (15.0, false));
    assert_eq!(p(Fouls::Nfl, 2025, K::UnsportsmanlikeConduct, 0, false), (15.0, false));
}

#[test]
fn the_spec_examples() {
    let nfl = Rules::for_level(Level::NFL);
    let hs = Rules::for_level(Level::HS);
    // A high school offense in an NFHS state on 3rd and 20 draws pass
    // interference: 15 from the previous spot and no first down (3rd and 5).
    let m = fouls::defense(&hs, &foul(FoulKind::PassInterference, 1, 62.0), &incomplete(30.0));
    assert_eq!((m.spot, m.first_down), (45.0, false));
    // Roughing the passer on an incompletion from the 20: the 10, half the
    // distance (the old table gave the 5).
    let m = fouls::defense(&nfl, &foul(FoulKind::RoughingPasser, 1, 70.0), &incomplete(80.0));
    assert_eq!(m.spot, 90.0);
    // Defensive holding from the 6: the 3 (half the distance), not the 1.
    let m = fouls::defense(&nfl, &foul(FoulKind::DefensiveHolding, 1, 99.0), &incomplete(94.0));
    assert_eq!(m.spot, 97.0);
    // A hold in the end zone, the line at the 6: a safety, not the 3.
    let m = fouls::offense(&nfl, &foul(FoulKind::OffensiveHolding, 0, -1.0), &incomplete(6.0));
    assert!(m.safety);
    // NFL pass interference in the end zone: the 1.
    let m = fouls::defense(&nfl, &foul(FoulKind::PassInterference, 1, 104.0), &incomplete(80.0));
    assert_eq!(m.spot, 99.0);
    // College: 15 from the previous spot at most, the 2 in the end zone.
    let fbs = Rules::for_level(Level::FBS);
    let m = fouls::defense(&fbs, &foul(FoulKind::PassInterference, 1, 70.0), &incomplete(30.0));
    assert_eq!(m.spot, 45.0);
    let m = fouls::defense(&fbs, &foul(FoulKind::PassInterference, 1, 105.0), &incomplete(90.0));
    assert_eq!(m.spot, 98.0);
}

#[test]
fn fouls_during_a_run_and_grounding() {
    let nfl = Rules::for_level(Level::NFL);
    let fbs = Rules::for_year(Level::FBS, 2024, false);
    let hs = Rules::for_year(Level::HS, 2024, false);
    // A defensive hold on a run that ended 8 yards past the line: marked
    // from the end of the run, the yards standing.
    let run = Play { los: 40.0, end: 48.0, goal: 100.0, run: true, complete: false, turnover: false, crossed: false };
    let m = fouls::defense(&fbs, &foul(FoulKind::DefensiveHolding, 1, 45.0), &run);
    assert!(m.stands && m.spot == 58.0);
    // A hold by the offense 4 yards past the line on that run: from its
    // spot; one behind the line from the previous spot (NFHS before 2023
    // from its spot).
    let m = fouls::offense(&nfl, &foul(FoulKind::OffensiveHolding, 0, 44.0), &run);
    assert_eq!(m.spot, 34.0);
    let m = fouls::offense(&nfl, &foul(FoulKind::OffensiveHolding, 0, 37.0), &run);
    assert_eq!(m.spot, 30.0);
    let old_hs = Rules::for_year(Level::HS, 2021, false);
    let m = fouls::offense(&old_hs, &foul(FoulKind::OffensiveHolding, 0, 37.0), &run);
    assert_eq!(m.spot, 27.0);
    // Grounding from the 40, thrown from the 33: the NFL's 10 from the line
    // (the 30), college's spot of the pass (the 33), high school's 5 from
    // it (the 28); thrown from the 27, the NFL takes the spot of the pass.
    let g = |r: &Rules, at: f64| fouls::offense(r, &foul(FoulKind::IntentionalGrounding, 0, at), &incomplete(40.0));
    assert_eq!(g(&nfl, 33.0).spot, 30.0);
    assert_eq!(g(&fbs, 33.0).spot, 33.0);
    assert_eq!(g(&hs, 33.0).spot, 28.0);
    assert_eq!(g(&nfl, 27.0).spot, 27.0);
    assert!(g(&fbs, 33.0).loss_of_down);
    // From his own end zone: a safety.
    assert!(g(&nfl, -2.0).safety);
}

#[test]
fn kick_rules_by_level() {
    let nfl = Rules::for_level(Level::NFL);
    let fbs = Rules::for_year(Level::FBS, 2024, false);
    let hs = Rules::for_year(Level::HS, 2024, false);
    // A miss from the other side's 30 (the line at 70, the hold at 63):
    // the NFL's spot of the kick (37), college's line (30), high school's
    // touchback (20); a short high school one that came down at the 10 is
    // the defense's ball there.
    assert_eq!(fouls::missed_fg_spot(&nfl, 70.0, 63.0, None, 100.0), 37.0);
    assert_eq!(fouls::missed_fg_spot(&fbs, 70.0, 63.0, None, 100.0), 30.0);
    assert_eq!(fouls::missed_fg_spot(&hs, 70.0, 63.0, None, 100.0), 20.0);
    assert_eq!(fouls::missed_fg_spot(&hs, 70.0, 63.0, Some(90.0), 100.0), 10.0);
    // Inside the 20 the miss comes out to the 20.
    assert_eq!(fouls::missed_fg_spot(&fbs, 88.0, 81.0, None, 100.0), 20.0);
    // A kickoff out of bounds: the NFL's 40; college's 35 to 2007, 40 in
    // 2008-11, then 30 beyond the kick (the 35 from the 35, the 50 from a
    // free kick at the 20); high school 25 beyond it (the 35 from the 40).
    assert_eq!(fouls::kick_out_spot(&nfl, 35.0, 100.0), 40.0);
    assert_eq!(fouls::kick_out_spot(&Rules::for_year(Level::FBS, 2005, false), 35.0, 100.0), 35.0);
    assert_eq!(fouls::kick_out_spot(&Rules::for_year(Level::FBS, 2010, false), 30.0, 100.0), 40.0);
    assert_eq!(fouls::kick_out_spot(&fbs, 35.0, 100.0), 35.0);
    assert_eq!(fouls::kick_out_spot(&fbs, 20.0, 100.0), 50.0);
    assert_eq!(fouls::kick_out_spot(&hs, 40.0, 100.0), 35.0);
}

#[test]
fn each_states_book() {
    // Texas plays NCAA rules every season, Massachusetts through 2018.
    assert_eq!(Rules::high_school("TX", 2005).foul_book(), Fouls::Ncaa);
    assert_eq!(Rules::high_school("TX", 2025).foul_book(), Fouls::Ncaa);
    assert_eq!(Rules::high_school("MA", 2018).foul_book(), Fouls::Ncaa);
    assert_eq!(Rules::high_school("MA", 2019).foul_book(), Fouls::Nfhs);
    assert_eq!(Rules::high_school("OH", 2025).foul_book(), Fouls::Nfhs);
    assert_eq!(Rules::high_school("TX", 2025).level, Level::HS);
    assert_eq!(Rules::for_level(Level::FCS).foul_book(), Fouls::Ncaa);
    // Linemen downfield on a pass: the NFL's one yard, the NCAA's three
    // (Texas high schools too), the NFHS's two.
    assert_eq!(fouls::downfield(Fouls::Nfl), 1.0);
    assert_eq!(fouls::downfield(Rules::high_school("TX", 2025).foul_book()), 3.0);
    assert_eq!(fouls::downfield(Rules::high_school("OH", 2025).foul_book()), 2.0);
    // Live-ball fouls at the snap: offside runs in the NFL and college, not
    // under NFHS rules; a false start never does.
    assert!(fouls::live_at_snap(Fouls::Nfl, FoulKind::Offside));
    assert!(!fouls::live_at_snap(Fouls::Nfhs, FoulKind::Offside));
    assert!(!fouls::live_at_snap(Fouls::Ncaa, FoulKind::FalseStart));
    assert!(fouls::live_at_snap(Fouls::Nfhs, FoulKind::IllegalFormation));
}

/// The fouls before a kick (EG5): a false start, a delay or a jump before a
/// punt, field goal or try is a dead ball booked under the kick's own call,
/// and the same side plays the down (or the try) again; a hard count on
/// fourth and short ends in one foul. Real NFL 2021-25: about 0.014 of kick
/// snaps (nflverse).
#[test]
fn fouls_before_kicks_are_dead_balls_and_replay_the_down() {
    use crate::game::stats::{KICK_BACK_UP, PlayType, kick_no_play};
    use FoulKind as K;
    let level = Level::NFL;
    let teams = crate::game::synthetic::synthetic_league(level, 2, 7);
    let (mut kicks, mut fouls, mut counts) = (0usize, 0usize, 0usize);
    for seed in 0..120 {
        let r = crate::game::quick_game(&teams[0], &teams[1], level, seed);
        let plays = &r.stats.plays;
        for (k, p) in plays.iter().enumerate() {
            if matches!(p.kind, PlayType::Punt | PlayType::FieldGoal | PlayType::ExtraPoint) {
                kicks += 1;
            }
            let count = p.kind == PlayType::NoPlay && p.call == "hard count on fourth down";
            if !(count || kick_no_play(p) && p.call != KICK_BACK_UP) {
                continue;
            }
            if count {
                counts += 1;
            } else {
                fouls += 1;
            }
            assert_eq!(p.penalties.len(), 1, "one foul: {}", p.why);
            let kind = p.penalties[0].0;
            assert!(
                matches!(kind, K::FalseStart | K::DelayOfGame | K::NeutralZone | K::Offside),
                "a dead-ball foul: {}",
                p.why
            );
            if let Some(next) = plays.get(k + 1) {
                assert_eq!(next.offense, p.offense, "the same side plays again: {} / {}", p.why, next.why);
            }
        }
    }
    let share = fouls as f64 / kicks.max(1) as f64;
    assert!((0.005..0.04).contains(&share), "fouls before a kick, a kick snap: {share:.4} ({fouls} of {kicks})");
    assert!(counts < 60, "hard counts on fourth and short: {counts} in 120 games");
}
