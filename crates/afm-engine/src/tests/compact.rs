//! Memory compaction (final build lane 1): the compact stores read back
//! every value bit for bit and write the same bytes as the plain forms, so
//! results and world files don't move.

use crate::player::OptF64;
use crate::world::records::{STAT_COUNT, StatSet};

#[derive(serde::Serialize, serde::Deserialize)]
struct Plain {
    #[serde(with = "serde_big_array::BigArray")]
    s: [f32; STAT_COUNT],
    o: Option<f64>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Compact {
    s: StatSet,
    o: OptF64,
}

#[test]
fn a_stat_row_keeps_every_value_and_the_file_shape() {
    let mut a = [0f32; STAT_COUNT];
    a[0] = 41.0;
    a[3] = -0.0;
    a[7] = 312.5;
    a[STAT_COUNT - 1] = 1e-9;
    let s = StatSet::from(a);
    for k in 0..STAT_COUNT {
        assert_eq!(s.get(k).to_bits(), a[k].to_bits(), "stat {k}");
    }
    assert_eq!(StatSet::default().to_array(), [0f32; STAT_COUNT]);
    // Written exactly as the plain array, and read back from it.
    for o in [None, Some(0.0), Some(-3.25), Some(87.123456789)] {
        let plain = rmp_serde::to_vec_named(&Plain { s: a, o }).unwrap();
        let compact = rmp_serde::to_vec_named(&Compact { s: s.clone(), o: o.into() }).unwrap();
        assert_eq!(plain, compact);
        let back: Compact = rmp_serde::from_slice(&plain).unwrap();
        assert_eq!(back.s.to_array().map(f32::to_bits), a.map(f32::to_bits));
        assert_eq!(back.o.get().map(f64::to_bits), o.map(f64::to_bits));
    }
}

/// A world file from before the world reader's stats (65 a row) still
/// loads: the stats it lacks read as zero.
#[test]
fn an_older_shorter_stat_row_still_reads() {
    #[derive(serde::Serialize)]
    struct Old {
        s: Vec<f32>,
    }
    #[derive(serde::Deserialize)]
    struct New {
        s: StatSet,
    }
    let old: Vec<f32> = (0..65).map(|k| k as f32 + 0.5).collect();
    let bytes = rmp_serde::to_vec_named(&Old { s: old.clone() }).unwrap();
    let back: New = rmp_serde::from_slice(&bytes).unwrap();
    for k in 0..STAT_COUNT {
        assert_eq!(back.s.get(k), old.get(k).copied().unwrap_or(0.0), "stat {k}");
    }
}

#[test]
fn season_rows_grow_by_one_not_by_doubling() {
    use crate::development::{SeasonContext, develop_season};
    use crate::player::{Level, PlayingTime};
    let mut rng = crate::rng::Rng::new(7);
    let mut p = crate::player::generate_prospect(&mut rng, Default::default());
    for _ in 0..5 {
        develop_season(&mut p, &SeasonContext::new(Level::HS, 10.0, PlayingTime::Starter), &mut rng);
        assert_eq!(p.history.capacity(), p.history.len());
    }
}

/// The game log builds its rows lean (`wlog/lean.rs`): one pass that keeps
/// only the fields that are not zero, false or empty, rounded the log's
/// way. It has to hold the same as the full value compacted afterwards.
#[test]
fn lean_rows_match_the_full_value_compacted() {
    use crate::game::drives::drives;
    use crate::game::quick_game;
    use crate::game::synthetic::synthetic_league;
    use crate::player::Level;
    use crate::world::wlog::games::compact;
    use crate::world::wlog::lean::lean;
    // Lean drops every empty field as it goes; only a variant keeps its
    // name over an empty value (`{"Challenge": 0}`: the home side
    // challenged), which compacting afterwards lost.
    fn no_empty(v: &serde_json::Value, what: &str) {
        match v {
            serde_json::Value::Object(m) => {
                for (k, x) in m {
                    let variant = k.starts_with(|c: char| c.is_ascii_uppercase());
                    assert!(variant || !crate::world::wlog::games::empty(x), "{what} kept {k}: {x}");
                    no_empty(x, what);
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| no_empty(x, what)),
            _ => {}
        }
    }
    let same = |what: &str, full: serde_json::Value, lean: serde_json::Value| {
        no_empty(&lean, what);
        let (mut a, mut b) = (full, lean);
        compact(&mut a);
        compact(&mut b);
        assert_eq!(a, b, "{what}");
    };
    for level in [Level::HS, Level::FBS, Level::NFL] {
        let teams = synthetic_league(level, 4, 11);
        for g in 0..3u64 {
            let r = quick_game(&teams[g as usize], &teams[3 - g as usize], level, 9_100 + g);
            let s = &r.stats;
            for side in 0..2 {
                same("team box", serde_json::to_value(&s.team[side]).unwrap(), lean(&s.team[side]));
                for (id, l) in &s.players[side] {
                    same(&format!("player {id}"), serde_json::to_value(l).unwrap(), lean(l));
                }
            }
            for p in &s.plays {
                same(&format!("{level:?} snap {}", p.n), serde_json::to_value(p).unwrap(), lean(p));
            }
            same("drives", serde_json::to_value(drives(s)).unwrap(), lean(&drives(s)));
        }
    }
}
