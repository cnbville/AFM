//! Quarterbacks at the top of the NFL draft (realism audit item 4): builds a
//! world with its history on the stand-in and prints, for every recorded
//! draft, the first pick's position, the quarterbacks in the top 10 and the
//! first quarterback taken, then the shares against real 2000-2025 (73.1%
//! of first picks, 28.5% of the top 5, 19.1% of the top 10, 3.0 in round one, `draft_positions_2000_2025.csv`).
//! `draft_qb_check [seed] [history_years] [scale]`; defaults 321, 30, 1.0.
use afm_engine::player::Level;
use afm_engine::positions::Position;
use afm_engine::world::{World, WorldOptions};
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(321);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
    let scale: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let opts = WorldOptions {
        seed,
        history_years: years,
        scale,
        engine_from: Some(2100),
        hs_engine_years: None,
        ..Default::default()
    };
    let w = World::new_watched(opts, |w, secs| eprintln!("season {}: {secs:.0}s", w.year - 1));
    let mut by_year: BTreeMap<i32, Vec<(u32, Position)>> = BTreeMap::new();
    for a in &w.alumni {
        if let Some(pk) = a.pick {
            by_year.entry(a.year).or_default().push((pk, a.position));
        }
    }
    let (mut drafts, mut first_qb, mut top5_qb, mut top10, mut top10_qb, mut round1_qb, mut no_r1_qb) =
        (0, 0, 0, 0, 0, 0, 0);
    println!("year  first  QBs in top 10  first QB  QBs in round 1");
    for (y, mut picks) in by_year {
        picks.sort_by_key(|x| x.0);
        if picks.first().is_none_or(|x| x.0 != 1) {
            continue;
        }
        let r1 = picks.iter().filter(|x| x.0 <= 32 && x.1 == Position::QB).count();
        let t10 = picks.iter().filter(|x| x.0 <= 10).count();
        let t10q = picks.iter().filter(|x| x.0 <= 10 && x.1 == Position::QB).count();
        let fq = picks.iter().find(|x| x.1 == Position::QB).map_or("-".to_string(), |x| x.0.to_string());
        println!("{y}  {:>5}  {t10q:>13}  {fq:>8}  {r1:>14}", picks[0].1.to_string());
        drafts += 1;
        first_qb += (picks[0].1 == Position::QB) as u32;
        top10 += t10;
        top5_qb += picks.iter().filter(|x| x.0 <= 5 && x.1 == Position::QB).count();
        top10_qb += t10q;
        round1_qb += r1;
        no_r1_qb += (r1 == 0) as u32;
    }
    let d = drafts.max(1) as f64;
    println!(
        "\n{drafts} drafts: first pick a QB {:.1}% (real 73.1%), top-5 picks QBs {:.1}% (real 28.5%), top-10 picks QBs {:.1}% (real 19.1%), QBs in round 1 {:.2} a draft (real 3.0), drafts with no round-1 QB {no_r1_qb}",
        100.0 * first_qb as f64 / d,
        100.0 * top5_qb as f64 / (5.0 * d),
        100.0 * top10_qb as f64 / top10.max(1) as f64,
        round1_qb as f64 / d,
    );
    // How each position grows in its first four NFL seasons (the board's
    // projection reads the same records), from every pro with four.
    let mut grow: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for p in &w.people {
        let nfl: Vec<_> = p.player.history.iter().filter(|h| h.level == Level::NFL).collect();
        if nfl.len() >= 4 && nfl[0].age <= 23 {
            grow.entry(nfl[0].position.to_string()).or_default().push(nfl[3].overall - nfl[0].overall_start);
        }
    }
    println!("\nGrowth over the first four NFL seasons (rookies 23 or younger): position, men, mean");
    for (pos, v) in &grow {
        println!("{pos:>5} {:>6} {:>6.2}", v.len(), v.iter().sum::<f64>() / v.len().max(1) as f64);
    }
    let young_qb = w
        .nfl
        .iter()
        .filter(|t| {
            let best = t.depth[Position::QB.index()].iter().max_by(|a, b| a.rating.total_cmp(&b.rating));
            best.is_some_and(|m| m.rating >= 82.0 && m.age <= 30)
        })
        .count();
    println!("clubs whose best quarterback is 82+ and 30 or younger: {young_qb} of {}", w.nfl.len());
}
