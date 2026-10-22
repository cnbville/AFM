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
