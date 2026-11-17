//! The top of the ratings at the draft (physical study items 1-3, 17, 18 and
//! 21): for drafted men, each attribute's ceilings at 95+, at 99 or past it
//! (a draft's worth), the best, and the overall and potential tops. Run it
//! with the wall in and without to see where men stack on 99 and where the
//! thin top bites.
//!
//! `top_check [class size] [classes] [seed]`

use afm_engine::attributes::Attr;
use afm_engine::career::{CohortOptions, simulate_cohort};
use afm_engine::player::Player;
use afm_engine::positions::weight;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let size: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(250_000);
    let classes: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(8);
    let seed = args.get(3).cloned().unwrap_or_else(|| "tops".into());
    let mut drafted: Vec<Player> = Vec::new();
    let mut pool = String::from("pos,drafted,weight,speed,accel,overall,grade\n");
    for c in 0..classes {
        let class = simulate_cohort(CohortOptions::new(size, format!("{seed}-{c}").as_str()));
        // `AFM_TOP_POOL=file.csv`: every man who finished in college, drafted
        // or not, at the draft.
        if std::env::var("AFM_TOP_POOL").is_ok() {
            for r in class.records.iter().filter(|r| r.college.is_some()) {
                let p = r.at_draft.as_deref().unwrap_or(&r.player);
                pool += &format!(
                    "{:?},{},{:.1},{:.2},{:.2},{:.2},{:.2}\n",
                    p.position,
                    r.draft_pick.is_some() as u8,
                    afm_engine::body::body_now(p).weight,
                    p.attributes.get(Attr::Speed),
                    p.attributes.get(Attr::Acceleration),
                    p.overall(),
                    r.draft_grade.unwrap_or(0.0)
                );
            }
        }
        drafted.extend(
            class.records.into_iter().filter(|r| r.draft_pick.is_some()).map(|r| r.at_draft.map_or(r.player, |b| *b)),
        );
    }
    if let Ok(path) = std::env::var("AFM_TOP_POOL") {
        std::fs::write(path, pool).expect("pool");
    }
    let drafts = classes.max(1) as f64;
    println!("top_check: {classes} classes of {size}, {} drafted; a draft's worth of ceilings", drafted.len());
    println!(
        "{:<22} {:>6} {:>6} {:>6} {:>7}   key at his position: {:>6} {:>6} {:>6} {:>7}",
        "attribute", "95+", "99+", "100+", "best", "95+", "99+", "100+", "best"
    );
    for a in Attr::ALL {
        let all: Vec<f64> = drafted.iter().map(|p| afm_engine::debug::ceilings(p).get(a)).collect();
        let key: Vec<f64> = drafted
            .iter()
            .filter(|p| weight(p.position, a) > 0.0)
            .map(|p| afm_engine::debug::ceilings(p).get(a))
            .collect();
        let n = |v: &[f64], at: f64| v.iter().filter(|&&x| x >= at - 1e-6).count() as f64 / drafts;
        let best = |v: &[f64]| v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        println!(
            "{:<22} {:>6.2} {:>6.2} {:>6.2} {:>7.1}   {:>27.2} {:>6.2} {:>6.2} {:>7.1}",
            a.label(),
            n(&all, 95.0),
            n(&all, 99.0),
            n(&all, 100.0),
            best(&all),
            n(&key, 95.0),
            n(&key, 99.0),
            n(&key, 100.0),
            if key.is_empty() { 0.0 } else { best(&key) }
        );
    }
    // `AFM_TOP_DUMP=file.csv`: each drafted man's position, weight, his
    // speed, acceleration and agility now and their ceilings, and his
    // combine forty.
    if let Ok(path) = std::env::var("AFM_TOP_DUMP") {
        let mut rng = afm_engine::rng::Rng::new(0x70_9C4E);
        let mut out = String::from(
            "pos,weight,speed,accel,agility,c_speed,c_accel,c_agility,forty,strength,c_strength,bench,age,vertical,broad,jumping,lower,c_jumping,c_lower\n",
        );
        for p in &drafted {
            let c = afm_engine::debug::ceilings(p);
            let w = afm_engine::body::body_now(p).weight;
            let t = afm_engine::body::combine(p, &mut rng);
            out += &format!(
                "{:?},{w:.1},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{},{},{},{},{:.2},{:.2},{:.2},{:.2}\n",
                p.position,
                p.attributes.get(Attr::Speed),
                p.attributes.get(Attr::Acceleration),
                p.attributes.get(Attr::Agility),
                c.get(Attr::Speed),
                c.get(Attr::Acceleration),
                c.get(Attr::Agility),
                t.forty,
                p.attributes.get(Attr::Strength),
                c.get(Attr::Strength),
                t.bench,
                p.age,
                t.vertical,
                t.broad,
                p.attributes.get(Attr::Jumping),
                p.attributes.get(Attr::LowerBody),
                c.get(Attr::Jumping),
                c.get(Attr::LowerBody)
            );
        }
        std::fs::write(path, out).expect("dump");
    }
    let pots: Vec<f64> = drafted
        .iter()
        .map(|p| afm_engine::positions::overall_for(p.position, afm_engine::debug::ceilings(p)))
        .collect();
    let ovr: Vec<f64> = drafted.iter().map(|p| p.overall()).collect();
    // The 94 rule's check (physical study item 21): how even an elite's key
    // ceilings are, by potential band (the spread of his key ceilings,
    // weighted by how much his position reads each).
    for (lo, hi) in [(0.0, 75.0), (75.0, 85.0), (85.0, 90.0), (90.0, 999.0)] {
        let sds: Vec<f64> = drafted
            .iter()
            .zip(&pots)
            .filter(|(_, pot)| **pot >= lo && **pot < hi)
            .map(|(p, _)| {
                let c = afm_engine::debug::ceilings(p);
                let ks: Vec<(f64, f64)> = Attr::ALL
                    .iter()
                    .filter(|&&a| weight(p.position, a) > 0.0)
                    .map(|&a| (weight(p.position, a), c.get(a)))
                    .collect();
                let w: f64 = ks.iter().map(|k| k.0).sum();
                let m = ks.iter().map(|k| k.0 * k.1).sum::<f64>() / w;
                (ks.iter().map(|k| k.0 * (k.1 - m).powi(2)).sum::<f64>() / w).sqrt()
            })
            .collect();
        let mean = sds.iter().sum::<f64>() / sds.len().max(1) as f64;
        println!("key ceilings' spread in a man, potential {lo}-{hi}: {mean:.2} ({} men)", sds.len());
    }
    for (name, v) in [("potential", &pots), ("overall at the draft", &ovr)] {
        let n = |at: f64| v.iter().filter(|&&x| x >= at).count() as f64 / drafts;
        let best = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        println!(
            "{name}: 90+ {:.2}, 95+ {:.2}, 99+ {:.2}, 100+ {:.2} a draft; best {best:.1}",
            n(90.0),
            n(95.0),
            n(99.0),
            n(100.0)
        );
    }
}
