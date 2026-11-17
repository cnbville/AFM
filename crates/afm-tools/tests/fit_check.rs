//! The scheme-fit check (lane SF0, `docs/design/scheme-fit.md`) on a tiny
//! run: it plays and reads back, a stopped run picks up where it was, an
//! arm changes only its own man, the same arm plays the same games every
//! time, a drawn man plays inside his band, every claim behind the repeat
//! table names a real arm and measure, and the tool's copy of the world's
//! fit weights matches `systems::fits`.
//! Sizes are the tool's job (`cargo run --release --bin fit_check`), not
//! this test's.

use afm_engine::attributes::Attr;
use afm_engine::player::{Level, ProspectOptions, generate_prospect};
use afm_engine::positions::{POSITION_COUNT, Position, overall_for};
use afm_engine::rng::Rng;
use afm_engine::world::systems::{SystemNorms, fits};
use afm_tools::ab::Part;
use afm_tools::fit::{self, ROLES, Setup, read};

fn arm<'a>(all: &'a [fit::Arm], id: &str) -> &'a fit::Arm {
    all.iter().find(|a| a.id == id).unwrap_or_else(|| panic!("no arm {id}"))
}

#[test]
fn a_tiny_run_plays_reads_and_picks_up() {
    let dir = std::env::temp_dir().join(format!("fit_check_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let all = fit::arms(Level::FBS);
    let words: Vec<String> =
        ["FS-wc-ZoneCoverage+15", "RB1-pg-heavy1", "CBS-wc-man-built"].iter().map(|s| s.to_string()).collect();
    let list = fit::pick(&all, &words);
    let ids: Vec<&str> = list.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, ["base-wc", "base-pg", "FS-wc-ZoneCoverage+15", "CBS-wc-man-built", "RB1-pg-heavy1"]);
    let setup = Setup { level: Level::FBS, games: 40, seed: 151 };
    fit::run(&setup, &dir, &list).expect("the run");
    let run = read::Run::load(&dir).expect("read it back");
    assert_eq!(run.arms.len(), list.len());
    let files = read::report(&run).expect("the report");
    for f in ["FS.md", "CBS.md", "RB1.md", "effects.csv", "_bases.md", "_compact.md", "_items.md", "_recruited.md"] {
        assert!(files.iter().any(|x| x == f), "{f} not written: {files:?}");
        assert!(std::fs::metadata(dir.join("report").join(f)).is_ok_and(|m| m.len() > 0), "{f} empty");
    }
    let eff = read::effects(&run);
    let picks = eff.get("FS-wc-ZoneCoverage+15", "INT/season").expect("picks a season");
    assert!(picks.base > 0.0 && picks.se > 0.0 && picks.diff.is_finite(), "{picks:?}");
    // The as-recruited draw found a man who plays inside his band.
    let drawn = run.arm("RB1-pg-heavy1").expect("the draw");
    let m = &drawn.men[0];
    assert!(m.drawn && (225.0..=231.0).contains(&m.weight), "{m:?}");
    // A stopped run picks up where it was: a kept arm isn't played again.
    std::fs::remove_file(dir.join("RB1-pg-heavy1.f32")).unwrap();
    let before = std::fs::metadata(dir.join("base-wc.f32")).unwrap().modified().unwrap();
    fit::run(&setup, &dir, &list).expect("picks up");
    assert_eq!(std::fs::metadata(dir.join("base-wc.f32")).unwrap().modified().unwrap(), before);
    let again = read::Run::load(&dir).expect("read it again");
    assert_eq!(again.arms.len(), list.len());
    assert_eq!(again.arm("RB1-pg-heavy1").unwrap().x, drawn.x, "the replayed arm plays the same games");
    // A folder holding another run's settings is refused, and so is a game
    // count that would give the blocks uneven games.
    assert!(fit::run(&Setup { seed: 152, ..setup }, &dir, &list).is_err());
    assert!(fit::run(&Setup { games: 100, ..setup }, &dir.join("uneven"), &list).is_err());
    assert!(!dir.join("uneven").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_arm_changes_its_man_and_replays_the_same_games() {
    let (base, _) = fit::league(Level::FBS, 151);
    let b = fit::base_team(&base[0], Level::FBS);
    let all = fit::arms(Level::FBS);
    let (typ, _) = fit::arm_team(&base, &b, arm(&all, "base-wc"), Level::FBS);
    let a = arm(&all, "FS-wc-ZoneCoverage+15");
    let (t, men) = fit::arm_team(&base, &b, a, Level::FBS);
    let fs = b.at[fit::role("FS")];
    assert_eq!(men.len(), 1);
    assert_eq!(men[0].role, "FS");
    for (k, (p, q)) in typ.players.iter().zip(&t.players).enumerate() {
        for x in Attr::ALL {
            let want =
                if k == fs && x == Attr::ZoneCoverage { (p.attrs.get(x) + 15.0).min(99.0) } else { p.attrs.get(x) };
            // Ratings are kept as f32.
            assert!((q.attrs.get(x) - want).abs() < 1e-4, "player {k} {x:?}: {} against {want}", q.attrs.get(x));
        }
    }
    let (x1, _) = fit::run_arm(&base, &b, a, Level::FBS, 40, 151);
    let (x2, _) = fit::run_arm(&base, &b, a, Level::FBS, 40, 151);
    assert_eq!(x1.len(), fit::NB * fit::width());
    assert!(x1 == x2, "the same arm played different games");
}

#[test]
fn a_drawn_man_plays_inside_his_band() {
    for (level, seed) in [(Level::FBS, 151), (Level::NFL, 161)] {
        let (base, _) = fit::league(level, seed);
        let b = fit::base_team(&base[0], level);
        let all = fit::arms(level);
        let mut drawn = 0;
        for bd in fit::bands(level) {
            let typical = {
                let p = &b.team.players[b.at[fit::role(bd.role)]];
                overall_for(p.position, &p.attrs)
            };
            for (side, subs) in &bd.sides {
                for (i, &(lo, hi)) in subs.iter().enumerate() {
                    let id = format!("{}-{}-{side}{}", bd.role, bd.ctx.name(), i + 1);
                    let (_, men) = fit::arm_team(&base, &b, arm(&all, &id), level);
                    let m = &men[0];
                    if !m.drawn {
                        continue;
                    }
                    drawn += 1;
                    let v = if bd.part == Part::Forty { m.forty } else { m.weight };
                    assert!((lo..=hi).contains(&v), "{level:?} {id}: plays at {v:.2}, band {lo}-{hi}");
                    assert!(
                        (m.overall - typical).abs() <= 3.0,
                        "{level:?} {id}: overall {:.1}, typical {typical:.1}",
                        m.overall
                    );
                }
            }
        }
        assert!(drawn >= 40, "{level:?}: only {drawn} of 48 draws found a man");
    }
}

#[test]
fn every_claim_names_an_arm_and_a_measure() {
    for level in [Level::FBS, Level::NFL, Level::HS] {
        let all = fit::arms(level);
        let mut seen = std::collections::HashSet::new();
        for a in &all {
            assert!(seen.insert(a.id.clone()), "{} twice", a.id);
        }
        let season = if level == Level::NFL { 17.0 } else { 12.0 };
        for (label, id, metric, _) in read::CLAIMS {
            let a = arm(&all, id);
            let role = if a.who.len() == 1 { ROLES[a.who[0]] } else { "CB1" };
            assert!(
                read::metrics_for(role, season).iter().any(|m| m.name == *metric),
                "{label}: {role} has no measure {metric}"
            );
        }
        for id in fit::REPEAT {
            arm(&all, id);
        }
        for (_, id, _, _) in read::CLAIMS {
            assert!(fit::REPEAT.contains(id), "{id} is behind the repeat table but not in the repeat set");
        }
        let repeat = fit::pick(&all, &["repeat".to_string()]);
        assert_eq!(repeat.iter().filter(|a| !a.is_base()).count(), fit::REPEAT.len());
    }
}

#[test]
fn the_fit_weights_match_the_world() {
    let zero: SystemNorms = [[0.0; 11]; POSITION_COUNT];
    let mut rng = Rng::new(9u32);
    for (i, &pos) in Position::ALL.iter().enumerate() {
        for j in 0..20u32 {
            let p = generate_prospect(
                &mut rng,
                ProspectOptions {
                    id: Some(1_000 * i as u32 + j),
                    position: Some(pos),
                    age: Some(21),
                    talent_z: Some(0.5 + 0.1 * j as f64),
                },
            );
            let world = fits(&p, pos, &zero);
            let score = |slot: usize| {
                let mix: f64 = read::fit_weights(pos, slot).iter().map(|&(a, w)| p.attributes.get(a) * w).sum::<f64>();
                let heavy = ((p.body.weight - 250.0) / 1.2).clamp(20.0, 99.0);
                mix + if pos == Position::DT && slot == 6 { read::HEAVY_WEIGHT * heavy } else { 0.0 }
            };
            let mut ours = [0.0; 11];
            for r in [0..5, 5..7, 7..11] {
                if r.clone().all(|s| read::fit_weights(pos, s).is_empty()) {
                    continue;
                }
                let s: Vec<f64> = r.clone().map(score).collect();
                let mean = s.iter().sum::<f64>() / s.len() as f64;
                for (k, slot) in r.enumerate() {
                    ours[slot] = ((s[k] - mean) / 10.0).clamp(-1.5, 1.5);
                }
            }
            for slot in 0..11 {
                assert!(
                    (ours[slot] - world[slot]).abs() < 1e-9,
                    "{pos:?} slot {slot}: {} against {}",
                    ours[slot],
                    world[slot]
                );
            }
        }
    }
}
