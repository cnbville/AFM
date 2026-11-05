//! High school NIL in the world (NIL update N3's check): builds a world on
//! the stand-in and prints, each year from 2021, how many high schoolers
//! earn NIL money, by state rule, stars and size, then the top earners with
//! why, the best-known who earn nothing and why not.
//! Usage: hs_nil_check build <seed> <history years> <scale> <start year> <out>
//!        hs_nil_check report <world file>

use afm_engine::world::data::{HsNilScope, hs_nil_rule};
use afm_engine::world::fame;
use afm_engine::world::{Stage, World, WorldOptions};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("build") => {
            let n = |k: usize| args[k].parse::<f64>().expect("a number");
            let opts = WorldOptions {
                seed: n(2) as u32,
                history_years: n(3) as u32,
                scale: n(4),
                start_year: n(5) as i32,
                keep_game_log: false,
                engine_from: Some(2100),
                ..Default::default()
            };
            let t = std::time::Instant::now();
            let w = World::new_recorded(opts, None, None, |w, _| {
                eprintln!("year {} at {:.0}s", w.year, t.elapsed().as_secs_f64());
                if w.year >= 2021 {
                    report(w, false);
                }
                true
            })
            .expect("never stopped");
            report(&w, true);
            afm_engine::save::write_world(&w, Path::new(&args[6]), "hs_nil_check", "hs_nil_check", None)
                .expect("write");
        }
        Some("report") => {
            let w = afm_engine::save::read_world(Path::new(&args[2])).expect("read the world");
            report(&w, true);
        }
        _ => eprintln!("usage: hs_nil_check build <seed> <history> <scale> <start> <out> | report <world>"),
    }
}

fn report(w: &World, full: bool) {
    let scale = w.options.scale;
    let hs: Vec<usize> =
        (0..w.people.len()).filter(|&i| matches!(w.people[i].stage, Stage::HighSchool | Stage::PrepSchool)).collect();
    let state_of = |i: usize| w.high_schools.get(w.people[i].hs).map(|h| h.state);
    let open_states = w.states.iter().filter(|s| s.hs_nil).count();
    let mut open = 0usize;
    let mut paid: Vec<(usize, f64)> = Vec::new();
    for &i in &hs {
        if fame::hs_nil_open(w, &w.people[i]) {
            open += 1;
        }
        let m = w.people[i].brand as f64;
        if m > 0.0 {
            paid.push((i, m));
        }
    }
    paid.sort_by(|a, b| b.1.total_cmp(&a.1));
    let over = |x: f64| paid.iter().filter(|p| p.1 >= x).count();
    let total: f64 = paid.iter().map(|p| p.1).sum();
    println!(
        "{} {:?}: {} high schoolers, {} under a rule that lets them earn ({} states open); deals {} (national {:.0}), $1K+ {} ({:.0}), $10K+ {} ({:.0}), $100K+ {} ({:.0}); total ${:.2}M (national ${:.1}M)",
        w.year,
        w.today,
        hs.len(),
        open,
        open_states,
        paid.len(),
        paid.len() as f64 / scale,
        over(1e3),
        over(1e3) as f64 / scale,
        over(1e4),
        over(1e4) as f64 / scale,
        over(1e5),
        over(1e5) as f64 / scale,
        total / 1e6,
        total / 1e6 / scale
    );
    if !full {
        return;
    }
    // By stars: how many there are, how many can earn, how many earn, and their money.
    println!("\nBy stars (high schoolers now): all / under an open rule / earning / median and top of those earning");
    for stars in (0..=5).rev() {
        let all: Vec<usize> = hs.iter().copied().filter(|&i| w.people[i].stars == stars).collect();
        let can = all.iter().filter(|&&i| fame::hs_nil_open(w, &w.people[i])).count();
        let mut m: Vec<f64> = all.iter().map(|&i| w.people[i].brand as f64).filter(|&x| x > 0.0).collect();
        m.sort_by(|a, b| a.total_cmp(b));
        let med = m.get(m.len() / 2).copied().unwrap_or(0.0);
        let top = m.last().copied().unwrap_or(0.0);
        println!("  {stars}*: {} / {} / {} / ${:.0} and ${:.0}", all.len(), can, m.len(), med, top);
    }
    // By state: deals and money, the biggest first.
    let mut by_state: Vec<(usize, usize, f64)> = (0..w.states.len()).map(|s| (s, 0, 0.0)).collect();
    for &(i, m) in &paid {
        if let Some(s) = state_of(i) {
            by_state[s].1 += 1;
            by_state[s].2 += m;
        }
    }
    by_state.sort_by(|a, b| b.2.total_cmp(&a.2));
    println!("\nStates by money (deals, $):");
    for &(s, n, m) in by_state.iter().take(12) {
        let st = &w.states[s];
        println!("  {} {}: {} deals, ${:.0}", st.code, if st.hs_nil { "open" } else { "closed" }, n, m);
    }
    // The top earners, with why.
    println!(
        "\nTop 15 earners: stars pos name, school (state), age; fame (stage stars, level rating, role, position) x Aura -> money, kinds"
    );
    for &(i, m) in paid.iter().take(15) {
        line(w, i, m);
    }
    // The best-known who earn nothing, and why not.
    let mut famous: Vec<(usize, f64)> =
        hs.iter().map(|&i| (i, fame::fame(w, i))).filter(|&(i, f)| f >= 0.25 && w.people[i].brand <= 0.0).collect();
    famous.sort_by(|a, b| b.1.total_cmp(&a.1));
    println!("\nBest-known high schoolers with no deals (fame >= 0.25): {}", famous.len());
    let mut why = std::collections::BTreeMap::<&str, usize>::new();
    for &(i, _) in &famous {
        *why.entry(why_not(w, i)).or_default() += 1;
    }
    for (k, v) in &why {
        println!("  {k}: {v}");
    }
    for &(i, _) in famous.iter().take(8) {
        line(w, i, 0.0);
        println!("      why none: {}", why_not(w, i));
    }
    let paydays = hs.iter().filter(|&&i| w.people[i].brand > 250_000.0).count();
    println!("\nHigh schoolers over $250K a year (a payday for the development step): {paydays}");
}

fn line(w: &World, i: usize, money: f64) {
    let p = &w.people[i];
    let f = fame::fame(w, i);
    let lr = p.player.history.last().and_then(|h| h.level_rating).map_or(-1, |r| r as i32);
    let st = w.high_schools.get(p.hs).and_then(|h| w.states.get(h.state)).map_or("", |s| s.code.as_str());
    println!(
        "  {}* {} {}, {} ({st}), {}; fame {:.3} (lr {lr}, role {:?}, all-state {}), Aura {} -> ${:.0} {}",
        p.stars,
        p.player.position.name(),
        p.name,
        w.school_name(p.hs),
        p.age_on(w.today),
        f,
        p.last_role,
        p.cal.all_state,
        p.player.personality.traits.aura,
        money,
        fame::hs_nil_kinds(money)
    );
}

fn why_not(w: &World, i: usize) -> &'static str {
    let p = &w.people[i];
    let Some(state) = w.high_schools.get(p.hs).and_then(|h| w.states.get(h.state)) else { return "no school" };
    let rule = hs_nil_rule(&state.code);
    if fame::hs_nil_open(w, p) {
        // Open, so either the family turned deals down or the money is under the smallest deal.
        let f = fame::fame(w, i);
        if fame::hs_nil_offered(p, f) > 0.0 { "his family turns deals down" } else { "too little known for a deal" }
    } else if state.hs_nil {
        "under the state's age"
    } else {
        match rule.scope {
            HsNilScope::SignedInState => "not signed with an in-state college (AR, MO)",
            HsNilScope::CollegeAfterHs => "Texas: only college deals, paid after high school",
            HsNilScope::All => "his state's rule starts later",
            HsNilScope::Barred => "his state bars it",
        }
    }
}
