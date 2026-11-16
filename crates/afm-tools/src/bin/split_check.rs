//! The 10-yard split against real (physical study item 2, Cain 2026-11-15:
//! "make sure you look at 10-y splits, especially with big men"): drafted
//! men run 40 yards through the match engine's own movement (`Agent::steer`,
//! 0.1 s ticks), and their split at 10 yards is set beside the real
//! combine's by weight: the mean, the split's share of the forty, how far
//! men with the same forty split apart (real sd 0.04 s) and how much the
//! split moves with the forty (real big men 0.28 a second). Also the test
//! day's split (`body::split`) on one combine day each.
//!
//! Real: MockDraftable's electronic combine splits, drafted men 2005-2025
//! (1,997 men; the study's table).
//!
//! `split_check [class size] [classes] [seed]`

use afm_engine::body::combine;
use afm_engine::career::{CohortOptions, simulate_cohort};
use afm_engine::game::GamePlayer;
use afm_engine::game::geom::{DT, v2};
use afm_engine::game::sim::Agent;
use afm_engine::player::Player;
use afm_engine::rng::Rng;

/// Weight bands with the real split, its share of the forty.
const BANDS: [(f64, f64, f64, f64); 6] = [
    (0.0, 200.0, 1.55, 0.345),
    (200.0, 230.0, 1.58, 0.345),
    (230.0, 260.0, 1.62, 0.344),
    (260.0, 290.0, 1.66, 0.346),
    (290.0, 315.0, 1.77, 0.344),
    (315.0, 999.0, 1.81, 0.344),
];

/// His engine run: the time at 10 and at 40 yards.
fn run(gp: &GamePlayer) -> (f64, f64) {
    let mut a = Agent::new(0, 0, gp, "X", v2(0.0, 0.0), 1.0, 0.0);
    let (mut t, mut ten) = (0.0, f64::NAN);
    while t < 12.0 {
        let before = a.pos.x;
        a.steer(v2(a.vmax, 0.0));
        a.pos = a.pos + a.vel.scale(DT);
        let after = a.pos.x;
        if before < 10.0 && after >= 10.0 {
            ten = t + DT * (10.0 - before) / (after - before).max(1e-9);
        }
        if after >= 40.0 {
            return (ten, t + DT * (40.0 - before) / (after - before).max(1e-9));
        }
        t += DT;
    }
    (ten, f64::NAN)
}

/// Mean, sd around the line on x, and the line's slope.
fn fit(xy: &[(f64, f64)]) -> (f64, f64, f64) {
    let n = xy.len().max(1) as f64;
    let mx = xy.iter().map(|p| p.0).sum::<f64>() / n;
    let my = xy.iter().map(|p| p.1).sum::<f64>() / n;
    let sxx = xy.iter().map(|p| (p.0 - mx).powi(2)).sum::<f64>();
    let sxy = xy.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum::<f64>();
    let b = sxy / sxx.max(1e-12);
    let res = (xy.iter().map(|p| (p.1 - my - b * (p.0 - mx)).powi(2)).sum::<f64>() / n).sqrt();
    (my, res, b)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let size: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(250_000);
    let classes: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let seed = args.get(3).cloned().unwrap_or_else(|| "splits".into());
    let mut drafted: Vec<Player> = Vec::new();
    for c in 0..classes {
        let class = simulate_cohort(CohortOptions::new(size, format!("{seed}-{c}").as_str()));
        drafted.extend(
            class.records.into_iter().filter(|r| r.draft_pick.is_some()).map(|r| r.at_draft.map_or(r.player, |b| *b)),
        );
    }
    let mut rng = Rng::new(0x5_9117);
    // (weight, field forty, field split, test forty, test split)
    let rows: Vec<(f64, f64, f64, f64, f64)> = drafted
        .iter()
        .filter(|p| !p.position.is_specialist())
        .map(|p| {
            let gp = GamePlayer::from_player(p, 1.0, 3.0);
            let (ten, forty) = run(&gp);
            let c = combine(p, &mut rng);
            (gp.weight, forty, ten, c.forty, c.split)
        })
        .collect();
    println!("split_check: {classes} classes of {size}, {} drafted", rows.len());
    println!(
        "weight      n   field split (real)  /forty (real)  sd at a forty  slope   test split  /forty  sd at a forty  slope"
    );
    for (lo, hi, real, share) in BANDS {
        let g: Vec<_> = rows.iter().filter(|r| r.0 >= lo && r.0 < hi).collect();
        if g.len() < 10 {
            continue;
        }
        let field: Vec<(f64, f64)> = g.iter().map(|r| (r.1, r.2)).collect();
        let test: Vec<(f64, f64)> = g.iter().map(|r| (r.3, r.4)).collect();
        let (fm, fs, fb) = fit(&field);
        let (tm, ts, tb) = fit(&test);
        let ff = g.iter().map(|r| r.2 / r.1).sum::<f64>() / g.len() as f64;
        let tf = g.iter().map(|r| r.4 / r.3).sum::<f64>() / g.len() as f64;
        println!(
            "{:>3.0}-{:<3.0}  {:>4}   {:.3} ({:.2})       {:.3} ({:.3})    {:.3}          {:.2}    {:.3}       {:.3}   {:.3}          {:.2}",
            lo,
            hi.min(999.0),
            g.len(),
            fm,
            real,
            ff,
            share,
            fs,
            fb,
            tm,
            tf,
            ts,
            tb
        );
    }
    println!("real: same forty, split sd 0.04 s at every weight; big men's split moves 0.28 s a second of forty");
    // `AFM_SPLIT_DUMP=file.csv` writes each man's weight, the engine's forty
    // formula, speed, acceleration and the engine's split and forty.
    if let Ok(path) = std::env::var("AFM_SPLIT_DUMP") {
        let mut out = String::from("weight,forty_formula,speed,accel,field_split,field_forty\n");
        for p in drafted.iter().filter(|p| !p.position.is_specialist()) {
            let gp = GamePlayer::from_player(p, 1.0, 3.0);
            let (ten, forty) = run(&gp);
            out += &format!(
                "{:.1},{:.4},{:.2},{:.2},{:.4},{:.4}\n",
                gp.weight,
                gp.forty(),
                gp.attrs.get(afm_engine::attributes::Attr::Speed),
                gp.attrs.get(afm_engine::attributes::Attr::Acceleration),
                ten,
                forty
            );
        }
        std::fs::write(path, out).expect("dump");
    }
}
