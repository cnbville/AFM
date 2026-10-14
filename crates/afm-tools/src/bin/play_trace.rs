//! Debug tool: trace one snap. Usage: play_trace [run|pass|N concept index] [seed]

use afm_engine::game::OffScheme;
use afm_engine::game::flow::{trace_snap, trace_snap_opt};
use afm_engine::game::playbook::*;
use afm_engine::game::synthetic::synthetic_team;
use afm_engine::player::Level;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let what = args.get(1).cloned().unwrap_or("run".into());
    let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    // AFM_LEVEL=fbs|hs plays the snap at that level (default NFL).
    let level = match std::env::var("AFM_LEVEL").as_deref() {
        Ok("fbs") => Level::FBS,
        Ok("hs") => Level::HS,
        _ => Level::NFL,
    };
    let mut a = synthetic_team("A", level, 0.0, OffScheme::WestCoast, 11);
    // AFM_BOOST=POS:points raises every attribute of that position's starter.
    if let Ok(v) = std::env::var("AFM_BOOST") {
        let parts: Vec<&str> = v.split(':').collect();
        let pos = parts[0];
        let by: f64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(10.0);
        let only: Vec<String> = parts.get(2).map(|s| s.split(',').map(String::from).collect()).unwrap_or_default();
        if let Some(p) = a.players.iter_mut().find(|p| format!("{:?}", p.position) == pos) {
            for at in afm_engine::attributes::Attr::ALL {
                if !only.is_empty() && !only.contains(&format!("{at:?}")) {
                    continue;
                }
                p.attrs[at] = (p.attrs[at] + by).min(99.0);
            }
        }
    }
    let b = synthetic_team("B", level, 0.0, OffScheme::WestCoast, 22);
    let kind = match what.as_str() {
        "run" => PlayKind::Run(RunConcept::InsideZone),
        "power" => PlayKind::Run(RunConcept::Power),
        "oz" => PlayKind::Run(RunConcept::OutsideZone),
        "toss" => PlayKind::Run(RunConcept::Toss),
        "counter" => PlayKind::Run(RunConcept::Counter),
        "draw" => PlayKind::Run(RunConcept::Draw),
        "sneak" => PlayKind::Run(RunConcept::Sneak),
        "pass" => PlayKind::Pass(4),
        s => PlayKind::Pass(s.parse().unwrap_or(0)),
    };
    // AFM_BASE=1: 12 personnel under center against a base defense.
    let base = std::env::var("AFM_BASE").is_ok();
    let (pers, bf) = if base { (Personnel::P12, Backfield::UnderCenter) } else { (Personnel::P11, Backfield::Shotgun) };
    let call = OffCall {
        formation: Formation { personnel: pers, backfield: bf, trips: false, strength: 1.0 },
        kind,
        dir: 1.0,
        tempo: false,
    };
    // AFM_COV picks the coverage (0 Cover 0 .. 7 Cover 6), default Cover 3.
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
    let cov = std::env::var("AFM_COV")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|k| covs[k % 8])
        .unwrap_or(Coverage::Cover3);
    let pressure = if cov == Coverage::Cover0 { Pressure::DoubleA } else { Pressure::Four };
    let dcall = DefCall {
        personnel: if base { DefPersonnel::Base } else { DefPersonnel::Nickel },
        coverage: cov,
        pressure,
        disguise: false,
    };
    // A third argument sweeps that many seeds without the trace.
    if let Some(n) = args.get(3).and_then(|s| s.parse::<u64>().ok()) {
        let mut ys = Vec::new();
        for s in seed..seed + n {
            let los = std::env::var("AFM_LOS").ok().and_then(|v| v.parse().ok()).unwrap_or(30.0);
            let r = trace_snap_opt(&a, &b, level, call, dcall, los, s, false);
            println!(
                "{s:>4} {:?} {:+.1} air {:.0} sep {:.1} cmp {} | {}",
                r.kind, r.yards, r.air_yards, r.separation, r.complete as u8, r.why
            );
            ys.push(r.yards);
        }
        ys.sort_by(|a, b| a.total_cmp(b));
        let share = |lo: f64, hi: f64| 100.0 * ys.iter().filter(|&&y| y >= lo && y < hi).count() as f64 / n as f64;
        println!(
            "<=0 {:.0}% 1-3 {:.0}% 4-9 {:.0}% 10-19 {:.0}% 20+ {:.1}% 40+ {:.1}%",
            share(-99.0, 0.5),
            share(0.5, 3.5),
            share(3.5, 9.5),
            share(9.5, 19.5),
            share(19.5, 200.0),
            share(39.5, 200.0)
        );
        println!("mean {:.2} median {:.1}", ys.iter().sum::<f64>() / n as f64, ys[ys.len() / 2]);
        return;
    }
    let los = std::env::var("AFM_LOS").ok().and_then(|v| v.parse().ok()).unwrap_or(30.0);
    let r = trace_snap(&a, &b, level, call, dcall, los, seed);
    println!("{:?} {:+.1} yds | {}", r.kind, r.yards, r.why);
}
