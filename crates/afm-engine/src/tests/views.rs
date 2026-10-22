//! The frames the 2D and 3D views read (later extras topics 2 and 3): the
//! poses, actions and ball height are output only, and a kept game plays
//! again exactly from its seed.

use crate::game::pose::{Act, BallMode, NONE};
use crate::game::synthetic::synthetic_league;
use crate::game::{GameResult, GameSetup, PlayType, ReplaySetup, play_game};
use crate::player::Level;

fn j<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap()
}

/// The same game: play log, score, totals, every player's line (sorted, as
/// they sit in hash maps), injuries, trust and decisions.
fn same_game(a: &GameResult, b: &GameResult) -> bool {
    let lines = |r: &GameResult| {
        r.stats.players.iter().map(|m| m.iter().map(|(k, v)| (*k, j(v))).collect::<std::collections::BTreeMap<_, _>>()).collect::<Vec<_>>()
    };
    j(&a.stats.plays) == j(&b.stats.plays)
        && a.stats.score == b.stats.score
        && j(&a.stats.team) == j(&b.stats.team)
        && lines(a) == lines(b)
        && j(&a.injuries) == j(&b.injuries)
        && j(&a.trust) == j(&b.trust)
        && a.decisions == b.decisions
}

/// Recording frames (with every pose, action and the ball's height) never
/// changes a game: the same seed gives the same play log, injuries and
/// trust with frames on or off, at every level.
#[test]
fn frames_are_output_only() {
    for (level, seed) in [(Level::NFL, 11), (Level::FBS, 12), (Level::HS, 13)] {
        let teams = synthetic_league(level, 8, seed);
        for g in 0..6u64 {
            let (h, a) = ((g * 3) as usize % 8, (g * 5 + 1) as usize % 8);
            let a = if a == h { (a + 1) % 8 } else { a };
            let mut s = GameSetup::new(&teams[h], &teams[a], level, 4_400_001 + g * 7919);
            let off = play_game(&s);
            s.record = true;
            let on = play_game(&s);
            assert!(same_game(&off, &on), "{level:?} game {g}: recording frames changed the game");
            assert!(off.frames.is_empty() && !on.frames.is_empty());
        }
    }
}

/// A game kept for watching (your games) plays again snap for snap from
/// its packed setup, with or without frames.
#[test]
fn a_kept_game_replays_exactly() {
    let teams = synthetic_league(Level::FBS, 4, 21);
    let mut s = GameSetup::new(&teams[0], &teams[1], Level::FBS, 9_001);
    s.keep = true;
    s.kickoff = 19.5;
    let first = play_game(&s);
    let kept = first.replay.as_ref().expect("the setup is kept");
    let packed = kept.pack();
    assert!(packed.len() < 120_000, "packed setup is {} bytes", packed.len());
    let back = ReplaySetup::unpack(&packed).expect("unpacks");
    assert_eq!(back.kickoff, 19.5);
    let again = back.play(false);
    assert!(same_game(&first, &again), "the replay is a different game");
    let film = back.play(true);
    assert!(same_game(&first, &film), "the replay with frames is a different game");
}

/// The frames say what bodies do: every recorded snap has a pose for each
/// player, kick returns are kept too, a pass shows the throw and the catch, blocks name their man, and a deep ball rises well above the
/// players.
#[test]
fn frames_carry_poses_actions_and_ball_height() {
    let teams = synthetic_league(Level::NFL, 4, 31);
    let mut acts = [0usize; 18];
    let (mut deep_peak, mut kicks, mut snaps) = (0.0f32, 0, 0);
    for g in 0..3u64 {
        let mut s = GameSetup::new(&teams[g as usize % 4], &teams[(g as usize + 1) % 4], Level::NFL, 77 + g);
        s.record = true;
        let r = play_game(&s);
        for (n, frames) in &r.frames {
            let rec = r.stats.plays.iter().find(|p| p.n == *n).expect("frames belong to a logged play");
            if matches!(rec.kind, PlayType::Kickoff | PlayType::Punt) {
                kicks += 1;
            } else {
                snaps += 1;
                assert!(frames[0].poses.iter().all(|p| p.act == Act::Stance), "everyone is set at the snap");
            }
            for f in frames {
                assert_eq!(f.poses.len(), f.players.len());
                for p in &f.poses {
                    acts[p.act as usize] += 1;
                    if matches!(p.act, Act::Block | Act::Engaged | Act::Tackle) {
                        assert!(p.with != NONE && (p.with as usize) < f.players.len(), "{:?} names his man", p.act);
                    }
                    assert!(p.face.is_finite());
                }
                if f.ball_mode == BallMode::Held {
                    assert!((f.holder as usize) < f.players.len());
                }
                assert!(f.ball_z >= 0.0 && f.ball_z < 30.0);
            }
            if rec.kind == PlayType::Pass && rec.air_yards >= 30.0 {
                deep_peak = deep_peak.max(frames.iter().map(|f| f.ball_z).fold(0.0, f32::max));
            }
        }
    }
    assert!(snaps > 300 && kicks > 20, "{snaps} snaps and {kicks} kick returns recorded");
    for a in [Act::Run, Act::Block, Act::Engaged, Act::Rush, Act::Cover, Act::Pocket, Act::Throw, Act::Catch] {
        assert!(acts[a as usize] > 0, "no frame shows {}", a.name());
    }
    for a in [Act::Carry, Act::Tackle, Act::Fall, Act::Down] {
        assert!(acts[a as usize] > 0, "no frame shows {}", a.name());
    }
    // A 30-yard throw is in the air about 1.5 seconds: gravity lifts it
    // about 3 yards above the line from hand to hands.
    assert!(deep_peak > 4.0, "a deep ball peaks at {deep_peak:.1} yards");
}


#[test]
fn dbg_plays() {
    let teams = synthetic_league(Level::NFL, 4, 31);
    let mut s = GameSetup::new(&teams[0], &teams[1], Level::NFL, 77);
    s.record = true;
    let r = play_game(&s);
    let have: std::collections::HashSet<u32> = r.frames.iter().map(|f| f.0).collect();
    for p in r.stats.plays.iter().take(400) {
        let f = r.frames.iter().find(|f| f.0 == p.n);
        let h = f.map(|f| { let fr = &f.1[0]; let hold = fr.holder; (fr.players.len(), hold, fr.players.get(hold as usize).map(|x| x.0), fr.ball.x) });
        eprintln!("n{} q{} {:?} off{} yl{:.0} d{} call[{}] def[{}] pers[{}] route[{}] frames:{} {:?} kd{:.0} why[{}]", p.n, p.quarter, p.kind, p.offense, p.yardline_100, p.down, p.call, p.defense_call, p.personnel, p.route, have.contains(&p.n), h, p.kick_distance, p.why);
    }
    eprintln!("home ids {:?}", teams[0].players.iter().take(5).map(|p| p.id).collect::<Vec<_>>());
}
