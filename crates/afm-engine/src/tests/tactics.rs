//! Plays and tactics: the play designer's rules and the new plays running
//! on the engine.

use crate::game::OffScheme;
use crate::game::designer::{Design, Issue, legal, practice, review, save};
use crate::game::flow::trace_snap_opt;
use crate::game::playbook::*;
use crate::game::roster::Caller;
use crate::game::synthetic::synthetic_team;
use crate::player::Level;
use crate::rng::Rng;

fn smash_like() -> Design {
    Design {
        name: "test smash".into(),
        formation: Formation::plain(Personnel::P11, Backfield::Shotgun, 1.0),
        routes: vec![(ORole::Z, Route::Hitch(5.0)), (ORole::Slot, Route::Corner(12.0)), (ORole::X, Route::Slant)],
        reads: vec![ORole::Slot, ORole::Z, ORole::X],
        protection: Protection::Standard,
        drop: Drop::Quick,
        play_action: false,
    }
}

/// A lineman can't run a route, and a read must run one (topic 8.2).
#[test]
fn designer_flags_illegal_plays() {
    let ok = smash_like();
    assert!(legal(&ok, Level::NFL).is_empty(), "{:?}", legal(&ok, Level::NFL));
    let mut bad = smash_like();
    bad.routes.push((ORole::LT, Route::Flat));
    bad.reads.push(ORole::RB);
    let issues = legal(&bad, Level::FBS);
    assert!(issues.contains(&Issue::Ineligible(ORole::LT)), "{issues:?}");
    assert!(issues.contains(&Issue::ReadBlocks(ORole::RB)), "{issues:?}");
    assert!(save(&bad, Level::FBS).is_err());
}

/// A sharp coordinator sees two routes finishing on the same spot; the
/// saved play runs on the engine like any other.
#[test]
fn designer_reviews_and_practices() {
    let mut crowded = smash_like();
    crowded.routes = vec![(ORole::Z, Route::Hitch(6.0)), (ORole::Slot, Route::Hitch(6.0))];
    crowded.formation.set = Set::Bunch;
    let sharp = Caller::new(20.0, 20.0, 10.0);
    let notes = review(&crowded, &sharp, &mut Rng::new(3));
    assert!(notes.iter().any(|n| n.contains("on top of each other")), "{notes:?}");
    let k = save(&smash_like(), Level::NFL).expect("legal");
    assert!(k >= PASS_CONCEPTS.len());
    let a = synthetic_team("A", Level::NFL, 0.0, OffScheme::WestCoast, 11);
    let b = synthetic_team("B", Level::NFL, 0.0, OffScheme::WestCoast, 22);
    let look = [DefCall::new(DefPersonnel::Nickel, Coverage::Cover3, Pressure::Four)];
    let read = practice(k, smash_like().formation, &a, &b, &look, 40, Level::NFL, 5);
    assert!(read.completions > 0 && read.yards > 0.0, "{read:?}");
}

/// Every new run and the RPO gains ground on average against a plain
/// front, and the option and jet plays hand the ball to someone.
#[test]
fn new_plays_run() {
    let a = synthetic_team("A", Level::FBS, 0.0, OffScheme::SpreadOption, 11);
    let b = synthetic_team("B", Level::FBS, 0.0, OffScheme::WestCoast, 22);
    let d = DefCall::new(DefPersonnel::Base, Coverage::Cover3, Pressure::Four);
    for (rc, gun, motion, dir) in [
        (RunConcept::Duo, false, Motion::None, 1.0),
        (RunConcept::Trap, false, Motion::None, 1.0),
        (RunConcept::QbPower, true, Motion::None, 1.0),
        (RunConcept::SpeedOption, true, Motion::None, 1.0),
        (RunConcept::TripleOption, true, Motion::None, 1.0),
        (RunConcept::JetSweep, false, Motion::Jet, -1.0),
    ] {
        let mut f =
            Formation::plain(Personnel::P11, if gun { Backfield::Shotgun } else { Backfield::UnderCenter }, 1.0);
        f.motion = motion;
        let call = OffCall::new(f, PlayKind::Run(rc), dir, false);
        let mut yards = 0.0;
        // 300 snaps: a run's yards spread about six yards, so 60 left
        // the average a yard either way (Duo, a double-team run the
        // engine blocks one on one, gains about 1.6 a carry here).
        for s in 0..300 {
            let r = trace_snap_opt(&a, &b, Level::FBS, call, d, 35.0, 100 + s, false);
            yards += r.yards;
        }
        assert!(yards / 300.0 > 1.0, "{rc:?} averaged {:.2}", yards / 300.0);
    }
    let f = Formation::plain(Personnel::P11, Backfield::Shotgun, 1.0);
    let call = OffCall::new(f, PlayKind::Rpo(RunConcept::InsideZone, concept("RPO glance")), 1.0, false);
    let thrown =
        (0..80).filter(|&s| trace_snap_opt(&a, &b, Level::FBS, call, d, 35.0, 300 + s, false).air_yards != 0.0).count();
    assert!(thrown > 0 && thrown < 80, "RPO thrown {thrown} of 80");
}
