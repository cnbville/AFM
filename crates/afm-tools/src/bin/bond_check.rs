//! Coach and player bonds measured on one small world
//! (`docs/design/coach-bonds.md`, question 6: AI coaches on the same rules
//! changes worlds). Builds a world with games on the stand-in, then prints
//! the world's own numbers (the portal, who followed a coach, trouble,
//! mood, growth by class) that a world before bonds prints the same way,
//! and the bonds themselves: stages by seat and class, the kinds at the
//! top, the moments, fit and need, and their size in memory.
//! Usage: bond_check [scale] [history_years] [seed] (defaults 0.25, 4, 261)

use std::collections::BTreeMap;

use afm_engine::world::{Stage, World, WorldOptions};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scale: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.25);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(261);
    let t = std::time::Instant::now();
    let w =
        World::new(WorldOptions { seed, scale, history_years: years, engine_from: Some(2100), ..Default::default() });
    println!(
        "world: scale {scale}, {years} years of history, seed {seed}, built in {:.0} s",
        t.elapsed().as_secs_f64()
    );
    world_numbers(&w, years);
    bonds(&w);
}

/// The world's own numbers, the same before and after bonds.
fn world_numbers(w: &World, years: u32) {
    println!("\n## The world by year");
    println!("year  portal  landed  FBS-portal  trouble  graduates  declared");
    for log in w.log_history.iter().rev().take(years as usize).rev() {
        println!(
            "{}  {:>6}  {:>6}  {:>10}  {:>7}  {:>9}  {:>8}",
            log.year,
            log.portal_entries,
            log.portal_landed,
            log.portal_by_division[0],
            log.college_trouble,
            log.graduates,
            log.declared
        );
    }
    let notes = w.people.iter().flat_map(|p| p.notes.iter()).chain(w.alumni.iter().flat_map(|a| a.notes.iter()));
    let (mut head, mut pos) = (0, 0);
    for n in notes {
        if n.contains("followed his old head coach") {
            head += 1;
        } else if n.contains("followed his old position coach") {
            pos += 1;
        }
    }
    println!("followed a head coach: {head}, followed a position coach: {pos}");
    let college: Vec<_> = w.people.iter().filter(|p| p.stage == Stage::College).collect();
    let mood = college.iter().map(|p| p.mood as f64).sum::<f64>() / college.len().max(1) as f64;
    let moved = college.iter().filter(|p| p.transfers > 0).count();
    println!(
        "college players {}: mean mood {mood:.3}, transferred at least once {moved} ({:.1}%)",
        college.len(),
        100.0 * moved as f64 / college.len().max(1) as f64
    );
    println!("mean overall by college seasons:");
    for s in 0..5u32 {
        let v: Vec<f64> =
            college.iter().filter(|p| p.college_seasons.min(4) == s).map(|p| p.player.overall()).collect();
        let m = v.iter().sum::<f64>() / v.len().max(1) as f64;
        println!("  {s}{}: {:>5} players, {m:.2}", if s == 4 { "+" } else { "" }, v.len());
    }
}

// BONDS-START
fn bonds(w: &World) {
    use afm_engine::world::folk::bonds::{self, Kind, Seat, Step};
    let b = &w.folk.bonds;
    let rows: usize = b.by.values().map(Vec::len).sum();
    let moments: usize = b.moments.values().map(Vec::len).sum();
    println!("\n## Bonds");
    println!(
        "{} people with bonds, {rows} bonds, {moments} moments; about {:.1} MB",
        b.by.len(),
        (rows * std::mem::size_of::<bonds::Bond>() + moments * std::mem::size_of::<bonds::Moment>()) as f64 / 1e6
    );
    let mut by_stage: BTreeMap<String, usize> = BTreeMap::new();
    for p in &w.people {
        if b.by.contains_key(&p.player.id) {
            *by_stage.entry(format!("{:?}", p.stage)).or_default() += 1;
        }
    }
    println!("people with bonds by stage: {by_stage:?}");
    let steps = [Step::JustMet, Step::KnowsYou, Step::TrustsYou, Step::InHisCorner, Step::Family];
    let line = |label: &str, v: &[Step]| {
        let n = v.len().max(1) as f64;
        let parts: Vec<String> =
            steps.iter().map(|s| format!("{:>5.1}", 100.0 * v.iter().filter(|x| *x == s).count() as f64 / n)).collect();
        println!("{label:<22} {:>5}  {}", v.len(), parts.join(" "));
    };
    println!("\n### Stages today, % (Just met, Knows you, Trusts you, In his corner, Family)");
    for (seat, name) in
        [(Seat::Head, "head coach"), (Seat::Position, "position coach"), (Seat::Coordinator, "coordinator")]
    {
        for s in 0..5u32 {
            let mut v = Vec::new();
            let mut close = 0.0f64;
            for i in 0..w.people.len() {
                let p = &w.people[i];
                if p.stage != Stage::College || p.college_seasons.min(4) != s {
                    continue;
                }
                for (c, st) in bonds::coaches_of(w, i) {
                    if st == seat
                        && let Some(x) = b.get(p.player.id, c)
                    {
                        v.push(Step::of(x));
                        close += x.close as f64;
                    }
                }
            }
            let label = format!("college {name} {s}{}", if s == 4 { "+" } else { "" });
            line(&label, &v);
            {
                println!("{:<22} mean closeness {:.3}", "", close / v.len().max(1) as f64);
            }
        }
    }
    for (seat, name) in [(Seat::Head, "NFL head coach"), (Seat::Coordinator, "NFL coordinator")] {
        let mut v = Vec::new();
        for i in 0..w.people.len() {
            let p = &w.people[i];
            if p.stage != Stage::Pro {
                continue;
            }
            for (c, st) in bonds::coaches_of(w, i) {
                if st == seat
                    && let Some(x) = b.get(p.player.id, c)
                {
                    v.push(Step::of(x));
                }
            }
        }
        line(name, &v);
    }
    let mut hs = Vec::new();
    for i in 0..w.people.len() {
        let p = &w.people[i];
        if p.stage == Stage::HighSchool {
            for (c, st) in bonds::coaches_of(w, i) {
                if st == Seat::HsHead
                    && let Some(x) = b.get(p.player.id, c)
                {
                    hs.push(Step::of(x));
                }
            }
        }
    }
    line("high school head coach", &hs);

    // The top stages' kinds, and who is closest.
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut top: Vec<(f32, usize, usize)> = Vec::new();
    for i in 0..w.people.len() {
        let p = &w.people[i];
        let Some(list) = b.by.get(&p.player.id) else { continue };
        for x in list {
            let c = x.coach as usize;
            if Step::of(x) >= Step::InHisCorner {
                let k: Kind = bonds::kind(w, i, c);
                *kinds.entry(format!("{:?} {:?}", Step::of(x), k)).or_default() += 1;
            }
            top.push((x.close, i, c));
        }
    }
    println!("\nIn his corner and Family by kind: {kinds:?}");
    top.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("closest pairs:");
    for &(close, i, c) in top.iter().take(8) {
        let p = &w.people[i];
        let x = b.get(p.player.id, c).unwrap();
        let co = &w.coaches[c];
        println!(
            "  {:.2} trust {:.2} fit {:.2} {:?} {} ({}, {:?}, need {:.2}) with {} (age {}), {} weeks, hard {}",
            close,
            x.trust,
            x.fit,
            Step::of(x),
            p.name,
            p.player.age,
            p.stage,
            bonds::need(p).0,
            co.name,
            co.age,
            x.weeks,
            x.hard()
        );
    }

    // Moments.
    let mut whats: BTreeMap<String, (usize, i64)> = BTreeMap::new();
    for ms in b.moments.values() {
        for m in ms {
            let e = whats.entry(format!("{:?}", m.what)).or_default();
            e.0 += 1;
            e.1 += m.by as i64;
        }
    }
    println!("\nmoments (count, mean move in hundredths):");
    for (k, (n, s)) in &whats {
        println!("  {k:<14} {n:>6}  {:+.1}", *s as f64 / *n as f64);
    }
    println!(
        "waiting {}, lost {}, burned {}, passed over {}, talked {}",
        b.waiting.len(),
        b.lost.len(),
        b.burned.len(),
        b.passed.len(),
        b.talked.len()
    );

    // Fit and need.
    let fits: Vec<f64> = b.by.values().flatten().map(|x| x.fit as f64).collect();
    let m = fits.iter().sum::<f64>() / fits.len().max(1) as f64;
    let sd = (fits.iter().map(|f| (f - m).powi(2)).sum::<f64>() / fits.len().max(1) as f64).sqrt();
    let mut needs = [0usize; 4];
    for p in w.people.iter().filter(|p| p.stage == Stage::College) {
        let n = bonds::need(p).0;
        needs[if n < 1.0 {
            0
        } else if n < 1.2 {
            1
        } else if n < 1.35 {
            2
        } else {
            3
        }] += 1;
    }
    println!("fit mean {m:.3} sd {sd:.3}; college need under 1.0 / 1.0-1.2 / 1.2-1.35 / 1.35+: {needs:?}");
}
// BONDS-END
