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
//! seed. A heavy back's row is his yards after contact: since minor bug 279
//! his weight no longer costs him the first tackler's whiff, so his yards
//! before contact fall only 0.04 a 10 lb (z 1.4 here, real 0.12). A heavy
//! defensive tackle's row is his run blocks shed: his power-rush wins rise
//! as clearly as before (+0.63 and +0.73 pts high against low on main at
//! 4,000 games on seeds 7 and 86, +0.69 and +0.67 after the edge lane), but
//! at 150 games that is only z 2.5 on main and 1.9 after it, so a reshuffle
//! of the games flips the check; his run sheds rise 6 points (z 12 here).

use afm_engine::player::Level;
use afm_tools::ab::run::direction;

/// (test, stat, expected sign of high minus low).
const LIST: &[(&str, &str, f64)] = &[
    ("qb_speed", "qb_scr", 1.0),
    ("rb_weight", "rb_yaco", 1.0),
    ("ot_feet", "blk_lost_speed", -1.0),
    ("iol_weight", "blk_run_won", 1.0),
    ("dt_weight", "dl_run_shed", 1.0),
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
