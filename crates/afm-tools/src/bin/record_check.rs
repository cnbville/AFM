//! Record deals on a real world's league (realism audit item 4): plays a
//! world file on with games on the stand-in, the same code paths as the
//! game, and reads every new veteran deal and extension against the
//! position's record, as shares of the cap of the year each was signed.
//! Prints each new record with how far it jumped, then each position's top
//! deal against its top five at the end, and the read gaps at the top.
//! Usage: record_check <world file> <days>

use afm_engine::positions::Position;
use afm_engine::world::pro::{market, money};
use std::collections::HashSet;
use std::path::Path;

fn share(k: &money::Contract) -> f64 {
    k.apy() / money::price_cap(k.signed)
}

fn is_deal(k: &money::Contract) -> bool {
    matches!(k.kind, money::Kind::Veteran | money::Kind::Extension)
}

/// Every deal in force at a position: (share of its year's cap, person).
fn deals(w: &afm_engine::world::World, pos: Position) -> Vec<(f64, usize)> {
    let mut v: Vec<(f64, usize)> = (0..w.people.len())
        .filter(|&i| w.people[i].player.position == pos)
        .filter_map(|i| {
            let c = w.people[i].pro.as_ref()?;
            c.team?;
            let k = c.contract.as_ref().filter(|k| is_deal(k))?;
            Some((share(k), i))
        })
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    v
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut w = afm_engine::save::read_world(Path::new(&args[1])).expect("read the world");
    w.options.engine_from = Some(2100);
    let days: u32 = args[2].parse().expect("days");
    eprintln!("start {:?}", w.today);
    let mut record: Vec<f64> = Position::ALL.iter().map(|&p| deals(&w, p).first().map_or(0.0, |x| x.0)).collect();
    let mut seen: HashSet<(u32, i32, i64)> = HashSet::new();
    for p in &w.people {
        if let Some(k) = p.pro.as_ref().and_then(|c| c.contract.as_ref()) {
            seen.insert((p.player.id, k.signed, (k.apy() / 1e3) as i64));
        }
    }
    let mut jumps: Vec<(String, f64)> = Vec::new();
    let mut fa: Vec<f64> = Vec::new();
    for _ in 0..days {
        w.advance_day();
        for i in 0..w.people.len() {
            let p = &w.people[i];
            let Some(k) = p.pro.as_ref().and_then(|c| c.contract.as_ref()) else { continue };
            if !is_deal(k) || !seen.insert((p.player.id, k.signed, (k.apy() / 1e3) as i64)) {
                continue;
            }
            let pos = p.player.position;
            let s = share(k);
            let top5 = market::top5(&w, w.year, pos);
            if k.kind == money::Kind::Veteran {
                fa.push(s / top5);
            }
            let r = record[pos.index()];
            if s > r && s > top5 {
                println!(
                    "RECORD {} {pos} {} age {} read {:.1}: {:.2}% of the cap, {:.3}x the top five, {:.3}x the old record ({:.2}%), {:?} ${:.1}M",
                    w.today.0,
                    p.name,
                    p.player.age,
                    market::consensus(&w, i),
                    100.0 * s,
                    s / top5,
                    s / r.max(1e-9),
                    100.0 * r,
                    k.kind,
                    k.apy() / 1e6
                );
                jumps.push((pos.to_string(), s / r.max(1e-9)));
                record[pos.index()] = s;
            }
        }
    }
    eprintln!("end {:?}", w.today);
    println!("\nPOSITION  top/top5share  top5avg/top5share  top/top5avg  read#1-mean(#2-6)");
    for &pos in Position::ALL.iter() {
        let v = deals(&w, pos);
        if v.len() < 5 {
            continue;
        }
        let t5 = market::top5(&w, w.year, pos);
        let avg = v.iter().take(5).map(|x| x.0).sum::<f64>() / 5.0;
        let l = &w.market.ladder[pos.index()];
        let gap = if l.len() >= 6 { l[0] - l[1..6].iter().sum::<f64>() / 5.0 } else { 0.0 };
        println!("{pos:<4} {:>8.3} {:>8.3} {:>8.3} {:>8.2}", v[0].0 / t5, avg / t5, v[0].0 / avg, gap);
    }
    let over = fa.iter().filter(|&&x| x > 1.08).count();
    fa.sort_by(|a, b| a.total_cmp(b));
    let mean = fa.iter().sum::<f64>() / fa.len().max(1) as f64;
    println!(
        "\nnew veteran deals {}: mean {:.4} of the top five's share, median {:.4}, past 1.08x {}",
        fa.len(),
        mean,
        fa.get(fa.len() / 2).copied().unwrap_or(0.0),
        over
    );
    for y in w.market.years.iter().filter(|y| y.year >= 2026) {
        println!(
            "market year {}: signed_new {} swayed {} extensions {} rookie extensions {} bids {}",
            y.year, y.signed_new, y.swayed, y.extensions, y.rookie_extensions, y.bids
        );
    }
    // The markets after the real years (set at each tag window).
    let real = market::shares(&w, money::REAL_TO);
    for (y, v) in &w.market.shares {
        let moved: Vec<String> = Position::ALL
            .iter()
            .map(|p| format!("{p} {:+.1}%", 100.0 * (v[p.index()] / real[p.index()] - 1.0)))
            .collect();
        println!("shares {y} against 2026: {}", moved.join(", "));
    }
    if let Some(y) = w.market.years.iter().find(|y| y.year == money::REAL_TO) {
        let h: Vec<String> = Position::ALL
            .iter()
            .filter_map(|p| y.heat.get(p.index()).filter(|h| h[1] > 0.0).map(|h| format!("{p} {:.3}", h[0] / h[1])))
            .collect();
        println!("heat {}: {}", y.year, h.join(", "));
    }
    let mut j: Vec<f64> = jumps.iter().map(|x| x.1).collect();
    j.sort_by(|a, b| a.total_cmp(b));
    println!(
        "records {}: jump median {:.3}, max {:.3}",
        j.len(),
        j.get(j.len() / 2).copied().unwrap_or(0.0),
        j.last().copied().unwrap_or(0.0)
    );
}
