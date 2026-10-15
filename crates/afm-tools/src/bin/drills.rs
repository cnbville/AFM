//! Movement drills (dynasty item 1, X1): runs the 5-10-5 shuttle and the
//! 3-cone through the match engine's own movement (`Agent::steer`, one
//! 0.1 s tick at a time) and compares what +10 Acceleration or Agility does
//! there with the combine model (`body::combine`), the way the 40 was fit.
//! Usage: drills [weight]

use afm_engine::attributes::Attr;
use afm_engine::game::geom::{DT, V2, v2};
use afm_engine::game::sim::Agent;
use afm_engine::game::synthetic::synthetic_team;
use afm_engine::game::{GamePlayer, OffScheme};
use afm_engine::player::Level;
use afm_engine::positions::Position;

/// Runs through `points` in order. `stop[k]`: touch the line and come back
/// (shuttle), otherwise round the cone at speed.
fn run(gp: &GamePlayer, points: &[(V2, bool)]) -> f64 {
    let mut a = Agent::new(0, 0, gp, "X", v2(0.0, 0.0), 1.0, 0.0);
    let mut t = 0.0;
    let mut k = 0;
    let brake = a.brake();
    // Rounding a cone a yard wide takes v^2 = sideways grip x 1 yd.
    let v_turn = (a.accel * a.cut).sqrt();
    while k < points.len() && t < 30.0 {
        let (target, stop) = points[k];
        let d = target - a.pos;
        let dist = d.len();
        let (end_speed, near) = if stop { (0.0, 0.3) } else { (v_turn, 0.7) };
        let want = (end_speed * end_speed + 2.0 * brake * (dist - near).max(0.0)).sqrt().min(a.vmax);
        let before = a.pos;
        a.steer(d.norm().scale(want.max(0.5)));
        a.pos = a.pos + a.vel.scale(DT);
        let after = (target - a.pos).len();
        if after < near {
            // The part of this tick after he got there, by distance moved.
            let moved = (a.pos - before).len().max(1e-6);
            let over = ((near - after) / moved).clamp(0.0, 1.0);
            t += DT * (1.0 - over);
            k += 1;
        } else {
            t += DT;
        }
    }
    t
}

/// The engine moves in 0.1 s ticks, so one run lands on the tick grid by
/// chance: average over many runs with the cones a few inches apart.
const RUNS: usize = 400;

fn jitter(k: usize, j: usize) -> f64 {
    let x = ((k * 7919 + j * 104_729) % 1000) as f64 / 1000.0;
    1.0 + 0.06 * (x - 0.5)
}

fn shuttle(gp: &GamePlayer) -> f64 {
    (0..RUNS)
        .map(|k| {
            let (a, b) = (5.0 * jitter(k, 1), 5.0 * jitter(k, 2));
            run(gp, &[(v2(a, 0.0), true), (v2(-b, 0.0), true), (v2(0.0, 0.0), false)])
        })
        .sum::<f64>()
        / RUNS as f64
}

fn cone(gp: &GamePlayer) -> f64 {
    (0..RUNS)
        .map(|k| {
            let (a, b) = (5.0 * jitter(k, 3), 5.0 * jitter(k, 4));
            run(
                gp,
                &[
                    (v2(a, 0.0), true),
                    (v2(0.0, 0.0), true),
                    (v2(a + 0.3, -0.3), false),
                    (v2(a + 0.6, b), false),
                    (v2(a - 0.3, b + 0.5), false),
                    (v2(a + 0.3, 0.3), false),
                    (v2(0.0, 0.0), false),
                ],
            )
        })
        .sum::<f64>()
        / RUNS as f64
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let weight: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(200.0);
    let t = synthetic_team("D", Level::NFL, 0.0, OffScheme::WestCoast, 1);
    let mut base = t.players.iter().find(|p| p.position == Position::WR).unwrap().clone();
    base.weight = weight;
    for a in [Attr::Speed, Attr::Acceleration, Attr::Agility] {
        base.attrs[a] = 75.0;
    }
    let with = |acc: f64, agi: f64| {
        let mut p = base.clone();
        p.attrs[Attr::Acceleration] += acc;
        p.attrs[Attr::Agility] += agi;
        p
    };
    let (s0, c0) = (shuttle(&base), cone(&base));
    println!("weight {weight}: shuttle {s0:.2} s, 3-cone {c0:.2} s in the sim (75 speed, acceleration, agility)\n");
    println!("| change | sim shuttle | combine model | sim 3-cone | combine model |");
    println!("|---|---|---|---|---|");
    for (label, acc, agi) in [("+10 acceleration", 10.0, 0.0), ("+10 agility", 0.0, 10.0), ("+10 both", 10.0, 10.0)] {
        let p = with(acc, agi);
        let quick = 0.7 * agi + 0.3 * acc;
        println!(
            "| {label} | {:+.3} | {:+.3} | {:+.3} | {:+.3} |",
            shuttle(&p) - s0,
            -0.0045 * quick,
            cone(&p) - c0,
            -0.007 * quick
        );
    }
}
