//! The A/B direction test (quick CI; `afm/ab-tests/plan.md`, agreed by Cain
//! 2026-11-06): each position's headline row at the NFL, low against high,
//! on a fixed seed. It fails only if an effect's direction flips (fast
//! scramblers stop scrambling, heavy backs stop running through
//! contact), not on its size. Sizes against real numbers are `ab_check`'s
//! job (`cargo run --release --bin ab_check -- headline`).
//!
//! Each row's stat is a step of its chain whose effect stands well clear
//! of its noise at this many games (z above 4 on this seed; `ab_check`
//! measures the whole chain). A headline the engine shows no clear effect for (a
//! corner's height, a tackle's or an edge's arms, a tight end's height, a
//! safety's weight: see `afm/ab-tests/`) has no direction to keep and is
//! left out until it gets one. So is a receiver's height: its catch rate
//! points against real, and its contested catches are not clear on this
//! seed.

use afm_engine::player::Level;
use afm_tools::ab::run::direction;

/// (test, stat, expected sign of high minus low).
const LIST: &[(&str, &str, f64)] = &[
    ("qb_speed", "qb_scr", 1.0),
    ("rb_weight", "rb_ybc", -1.0),
    ("ot_feet", "blk_lost_speed", -1.0),
    ("iol_weight", "blk_run_won", 1.0),
    ("dt_weight", "dl_power_win", 1.0),
    ("edge_getoff", "dl_shed", 1.0),
    ("edge_speed", "dl_speed_win", 1.0),
    ("lb_speed", "tk_rate", 1.0),
    ("k_power", "k_50_share", 1.0),
];

const GAMES: usize = 150;

#[test]
fn headline_effects_keep_their_direction() {
    let rows = direction(Level::NFL, GAMES, 7, LIST);
    let mut bad = Vec::new();
    for (id, key, d, want) in &rows {
        let z = d.d / d.se;
        eprintln!("{id} {key}: high − low {:+.4} (z {z:+.1}), want {want:+}", d.d);
        if d.d.signum() != *want || z.abs() < 2.0 {
            bad.push(format!("{id} {key}: {:+.4} (z {z:+.1})", d.d));
        }
    }
    assert!(bad.is_empty(), "these effects flipped or vanished: {bad:?}");
}
