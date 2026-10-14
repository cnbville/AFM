//! What beats what on the field: every pass and run concept against every
//! defensive look, played on the match engine between two average NFL
//! teams. The result is the football knowledge coordinators call from
//! (`crates/afm-engine/data/call_values.csv`): a sharp one trusts it, a
//! poor one barely uses it. Rerun after engine changes:
//! `cargo run --release --bin call_values -- 400 > crates/afm-engine/data/call_values.csv`
//! Each value is mean yards on the snap (capped at 40) minus 45 for a turnover.

use afm_engine::game::OffScheme;
use afm_engine::game::flow::trace_snap_opt;
use afm_engine::game::playbook::*;
use afm_engine::game::synthetic::synthetic_team;
use afm_engine::player::Level;
use rayon::prelude::*;

fn main() {
    let n: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(400);
    let a = synthetic_team("A", Level::NFL, 0.0, OffScheme::WestCoast, 11);
    let b = synthetic_team("B", Level::NFL, 0.0, OffScheme::WestCoast, 22);
    let covs = [
        Coverage::Cover0,
        Coverage::Cover1,
        Coverage::Cover2,
        Coverage::Tampa2,
        Coverage::Cover3,
        Coverage::Cover4,
        Coverage::Cover2Man,
        Coverage::Cover6,
    ];
    // Each coverage with four rushers and with a linebacker blitz (Cover 0
    // always sends the double-A pressure).
    let mut looks = Vec::new();
    for &c in &covs {
        if c == Coverage::Cover0 {
            looks.push((c, Pressure::DoubleA));
        } else {
            looks.push((c, Pressure::Four));
            looks.push((c, Pressure::LbBlitz));
        }
    }
    let runs = [
        RunConcept::InsideZone,
        RunConcept::OutsideZone,
        RunConcept::Power,
        RunConcept::Counter,
        RunConcept::Draw,
        RunConcept::ZoneRead,
        RunConcept::Toss,
        RunConcept::BuckSweep,
    ];
    let mut plays: Vec<(String, PlayKind)> =
        PASS_CONCEPTS.iter().enumerate().map(|(i, p)| (p.name.to_string(), PlayKind::Pass(i))).collect();
    plays.extend(runs.iter().map(|&r| (format!("{r:?}"), PlayKind::Run(r))));
    // AFM_CELL=concept:Coverage prints what happens on one cell (debug).
    if let Ok(cell) = std::env::var("AFM_CELL") {
        let (pn, cn) = cell.split_once(':').unwrap();
        let i = PASS_CONCEPTS.iter().position(|p| p.name == pn).unwrap();
        let cov = *covs.iter().find(|c| format!("{c:?}") == cn).unwrap();
        let call = OffCall {
            formation: Formation {
                personnel: Personnel::P11,
                backfield: Backfield::Shotgun,
                trips: false,
                strength: 1.0,
            },
            kind: PlayKind::Pass(i),
            dir: 1.0,
            tempo: false,
        };
        let dcall =
            DefCall { personnel: DefPersonnel::Nickel, coverage: cov, pressure: Pressure::Four, disguise: false };
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
    let rows: Vec<String> = plays
        .par_iter()
        .map(|(name, kind)| {
            let gun = matches!(kind, PlayKind::Pass(_) | PlayKind::Run(RunConcept::ZoneRead));
            let call = OffCall {
                formation: Formation {
                    personnel: Personnel::P11,
                    backfield: if gun { Backfield::Shotgun } else { Backfield::UnderCenter },
                    trips: false,
                    strength: 1.0,
                },
                kind: *kind,
                dir: 1.0,
                tempo: false,
            };
            let cells: Vec<String> = looks
                .iter()
                .map(|&(coverage, pressure)| {
                    let dcall = DefCall { personnel: DefPersonnel::Nickel, coverage, pressure, disguise: false };
                    let mut v = 0.0;
                    for s in 0..n {
                        let r = trace_snap_opt(&a, &b, Level::NFL, call, dcall, 35.0, 1 + s, false);
                        v += r.yards.min(40.0) - if r.turnover { 45.0 } else { 0.0 };
                    }
                    format!("{:.2}", v / n as f64)
                })
                .collect();
            format!("{name},{}", cells.join(","))
        })
        .collect();
    let head: Vec<String> = looks.iter().map(|(c, p)| format!("{c:?}/{p:?}")).collect();
    println!("concept,{}", head.join(","));
    for r in rows {
        println!("{r}");
    }
}
