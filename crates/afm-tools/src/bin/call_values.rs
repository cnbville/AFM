//! What beats what on the field: every pass and run concept against every
//! defensive look, played on the match engine between two average NFL
//! teams. The result is the football knowledge coordinators call from
//! (`crates/afm-engine/data/call_values.csv`): a sharp one trusts it, a
//! poor one barely uses it. Rerun after engine changes:
//! `cargo run --release --bin call_values -- 400 > crates/afm-engine/data/call_values.csv`
//! Each value is mean yards on the snap (capped at 40) minus 45 for a turnover.

use afm_engine::game::flow::trace_snap_opt;
use afm_engine::game::playbook::*;
use afm_engine::game::synthetic::synthetic_team;
use afm_engine::game::{Front, OffScheme};
use afm_engine::player::Level;
use rayon::prelude::*;

fn main() {
    let n: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(400);
    let a = synthetic_team("A", Level::NFL, 0.0, OffScheme::WestCoast, 11);
    let mut b = synthetic_team("B", Level::NFL, 0.0, OffScheme::WestCoast, 22);
    // The odd and tite fronts are a 3-4 staff's (a nose and two ends), the
    // rest a 4-3 staff's: the synthetic defense drew a 3-4, so the over
    // front lined up as an odd one.
    b.staff.front = Front::FourThree;
    let mut b34 = b.clone();
    b34.staff.front = Front::ThreeFour;
    // Each coverage with four rushers and with a linebacker blitz (Cover 0
    // always sends the double-A pressure).
    let mut looks = Vec::new();
    for c in Coverage::ALL {
        if c == Coverage::Cover0 {
            looks.push((c, Pressure::DoubleA));
        } else {
            looks.push((c, Pressure::Four));
            looks.push((c, Pressure::LbBlitz));
        }
    }
    // Runs against each front (plays and tactics topic 1), from Cover 3
    // with four rushers.
    let fronts = [FrontLook::Over, FrontLook::Under, FrontLook::Wide, FrontLook::Odd, FrontLook::Tite, FrontLook::Bear];
    let runs: Vec<RunConcept> = RUN_CONCEPTS.iter().copied().filter(|&r| r != RunConcept::Sneak).collect();
    let plays: Vec<(String, PlayKind)> =
        PASS_CONCEPTS.iter().enumerate().map(|(i, p)| (p.name.to_string(), PlayKind::Pass(i))).collect();
    // AFM_CELL=concept:Coverage prints what happens on one cell (debug).
    if let Ok(cell) = std::env::var("AFM_CELL") {
        let (pn, cn) = cell.split_once(':').unwrap();
        let i = PASS_CONCEPTS.iter().position(|p| p.name == pn).unwrap();
        let cov = *Coverage::ALL.iter().find(|c| format!("{c:?}") == cn).unwrap();
        let call =
            OffCall::new(Formation::plain(Personnel::P11, Backfield::Shotgun, 1.0), PlayKind::Pass(i), 1.0, false);
        let dcall = DefCall::new(DefPersonnel::Nickel, cov, Pressure::Four);
        let mut hist = std::collections::BTreeMap::new();
        let (mut comp, mut air, mut yac, mut sep, mut ttt) = (0.0, 0.0, 0.0, 0.0, 0.0);
        // Completions and attempts by time to throw (half-second buckets).
        let mut by_t = [(0u32, 0u32, 0.0f64); 16];
        for s in 0..n {
            let r = trace_snap_opt(&a, &b, Level::NFL, call, dcall, 35.0, 1 + s, false);
            *hist.entry(format!("{:?}{}", r.kind, if r.interception { "-INT" } else { "" })).or_insert(0) += 1;
            if r.complete {
                comp += 1.0;
                air += r.air_yards;
                yac += r.yac;
            }
            sep += r.separation;
            ttt += r.time_to_throw;
            let b = ((r.time_to_throw / 0.5) as usize).min(15);
            by_t[b].1 += 1;
            by_t[b].0 += r.complete as u32;
            by_t[b].2 += r.yards;
        }
        let nf = n as f64;
        for (b, (c, n, y)) in by_t.iter().enumerate() {
            if *n > 0 {
                println!(
                    "  ttt {:.1}-{:.1}s: {n} att, {c} comp, {:.1} yds/att",
                    b as f64 * 0.5,
                    b as f64 * 0.5 + 0.5,
                    y / *n as f64
                );
            }
        }
        println!(
            "{hist:?} comp {:.3} air/comp {:.1} yac/comp {:.1} sep {:.2} ttt {:.2}",
            comp / nf,
            air / comp,
            yac / comp,
            sep / nf,
            ttt / nf
        );
        return;
    }
    let blank_fronts = vec![String::new(); fronts.len()];
    let mut rows: Vec<String> = plays
        .par_iter()
        .map(|(name, kind)| {
            let call = OffCall::new(Formation::plain(Personnel::P11, Backfield::Shotgun, 1.0), *kind, 1.0, false);
            let cells: Vec<String> = looks
                .iter()
                .map(|&(coverage, pressure)| {
                    let dcall = DefCall::new(DefPersonnel::Nickel, coverage, pressure);
                    let mut v = 0.0;
                    for s in 0..n {
                        let r = trace_snap_opt(&a, &b, Level::NFL, call, dcall, 35.0, 1 + s, false);
                        v += r.yards.min(40.0) - if r.turnover { 45.0 } else { 0.0 };
                    }
                    format!("{:.2}", v / n as f64)
                })
                .collect();
            format!("{name},{},{}", cells.join(","), blank_fronts.join(","))
        })
        .collect();
    let blank_looks = vec![String::new(); looks.len()];
    rows.extend(
        runs.par_iter()
            .map(|&rc| {
                // Each run from the look it is run from: gun for the reads and
                // QB runs, a fullback for the iso, jet motion for the sweep.
                let gun = matches!(
                    rc,
                    RunConcept::ZoneRead | RunConcept::QbPower | RunConcept::SpeedOption | RunConcept::TripleOption
                );
                let pers = if rc == RunConcept::Iso { Personnel::P21 } else { Personnel::P11 };
                let mut f = Formation::plain(pers, if gun { Backfield::Shotgun } else { Backfield::UnderCenter }, 1.0);
                let mut dir = 1.0;
                if rc == RunConcept::JetSweep {
                    f.motion = Motion::Jet;
                    dir = -1.0;
                }
                let call = OffCall::new(f, PlayKind::Run(rc), dir, false);
                let cells: Vec<String> = fronts
                    .iter()
                    .map(|&front| {
                        let mut dcall = DefCall::new(DefPersonnel::Base, Coverage::Cover3, Pressure::Four);
                        dcall.front = front;
                        let d = if matches!(front, FrontLook::Odd | FrontLook::Tite) { &b34 } else { &b };
                        let mut v = 0.0;
                        for s in 0..n {
                            let r = trace_snap_opt(&a, d, Level::NFL, call, dcall, 35.0, 1 + s, false);
                            v += r.yards.min(40.0) - if r.turnover { 45.0 } else { 0.0 };
                        }
                        format!("{:.2}", v / n as f64)
                    })
                    .collect();
                format!("{rc:?},{},{}", blank_looks.join(","), cells.join(","))
            })
            .collect::<Vec<_>>(),
    );
    let mut head: Vec<String> = looks.iter().map(|(c, p)| format!("{c:?}/{p:?}")).collect();
    head.extend(fronts.iter().map(|f| format!("Front:{f:?}")));
    println!("concept,{}", head.join(","));
    for r in rows {
        println!("{r}");
    }
}
