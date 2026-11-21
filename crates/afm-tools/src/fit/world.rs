//! The world check for lane SF11 (`fit_check world`): a small world on the
//! stand-in games (the check reads installs, fit and recruiting, not the
//! games), then how its FBS programs install and recruit by the measured
//! fit: the systems they run by year, what their own rosters are worth in
//! what they run against the other systems of its kind (points a game),
//! each class's fit to its own school's systems against the average FBS
//! school's, and a man's fit by position (both on the readers' scale,
//! `systems::fits`: tenths of a rating point at his job).

use afm_engine::positions::Position;
use afm_engine::world::data::Division;
use afm_engine::world::systems::{self, Kind};
use afm_engine::world::{Stage, World, WorldOptions};

const KINDS: [(Kind, &str); 3] = [(Kind::Offense, "offense"), (Kind::Front, "front"), (Kind::Shell, "coverage")];

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 }
}

fn sd(v: &[f64]) -> f64 {
    let m = mean(v);
    if v.len() < 2 { 0.0 } else { (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (v.len() - 1) as f64).sqrt() }
}

fn pct(v: &mut [f64], q: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    v[((v.len() - 1) as f64 * q).round() as usize]
}

fn slots(inst: &systems::Installed) -> [usize; 3] {
    [inst.off, 5 + inst.front, 7 + inst.shell]
}

pub fn run(scale: f64, years: u32, seed: u32) {
    let t = std::time::Instant::now();
    let w = World::new(WorldOptions {
        seed,
        scale,
        history_years: years,
        engine_from: Some(2100),
        keep_game_log: false,
        ..Default::default()
    });
    println!(
        "world: scale {scale}, {years} years of history, seed {seed}, stand-in games; built in {:.0} s; now {}",
        t.elapsed().as_secs_f64(),
        w.year
    );

    println!(
        "\ninstalls by year (FBS programs: switches; offense WC AR WZ PG SO; front 4-3 3-4; coverage C3 C4 man C2)"
    );
    for l in &w.log_history {
        let m = l.fbs_systems;
        println!(
            "  {}  switches {:>3}   offense {:>3} {:>3} {:>3} {:>3} {:>3}   front {:>3} {:>3}   coverage {:>3} {:>3} {:>3} {:>3}",
            l.year, l.system_switches, m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8], m[9], m[10]
        );
    }

    let fbs: Vec<usize> = w
        .programs
        .iter()
        .filter(|p| p.division == Division::FBS && p.plays(w.year) && p.roster.len() >= 30)
        .map(|p| p.id)
        .collect();

    // What each roster is worth in what it runs, against the other systems
    // of the kind (points a game).
    println!(
        "\nfit by school ({} FBS programs, {}): its roster's worth in each system, points a game",
        fbs.len(),
        w.year
    );
    println!(
        "  kind       installed over the kind's mean (mean, sd)   the best over the mean (mean)   running the best   best minus installed (mean, p90)"
    );
    for (kind, name) in KINDS {
        let r = kind.range();
        let (mut inst_over, mut best_over, mut gap) = (Vec::new(), Vec::new(), Vec::new());
        let mut at_best = 0;
        for &p in &fbs {
            let prog = &w.programs[p];
            let ids: Vec<usize> =
                prog.roster.iter().copied().filter(|&i| w.people[i].stage == Stage::College).collect();
            let v = systems::roster_fit(&w, &ids);
            let m = r.clone().map(|s| v[s]).sum::<f64>() / r.len() as f64;
            let s = slots(&prog.installed)[kind as usize];
            let best = r.clone().map(|s| v[s]).fold(f64::MIN, f64::max);
            inst_over.push(v[s] - m);
            best_over.push(best - m);
            gap.push(best - v[s]);
            if v[s] >= best - 1e-9 {
                at_best += 1;
            }
        }
        let n = fbs.len().max(1) as f64;
        println!(
            "  {name:<9}  {:>+6.2} {:>5.2}                                {:>+6.2}                            {:>4.0}%              {:>5.2} {:>5.2}",
            mean(&inst_over),
            sd(&inst_over),
            mean(&best_over),
            100.0 * at_best as f64 / n,
            mean(&gap),
            pct(&mut gap, 0.9)
        );
    }

    // Each class's fit to its own school's systems against the average FBS
    // school's (the schools weighted alike).
    let installs: Vec<systems::Installed> = fbs.iter().map(|&p| w.programs[p].installed).collect();
    let mut by_class: Vec<(Vec<f64>, Vec<f64>)> = vec![(Vec::new(), Vec::new()); 6];
    let mut by_stars: Vec<(Vec<f64>, Vec<f64>)> = vec![(Vec::new(), Vec::new()); 6];
    let mut by_pos: Vec<Vec<f64>> = vec![Vec::new(); Position::ALL.len()];
    let newest = fbs
        .iter()
        .flat_map(|&p| w.programs[p].roster.iter())
        .filter(|&&i| w.people[i].stage == Stage::College)
        .map(|&i| w.people[i].class_of)
        .max()
        .unwrap_or(w.year);
    for &p in &fbs {
        let prog = &w.programs[p];
        for &i in &prog.roster {
            let person = &w.people[i];
            if person.stage != Stage::College {
                continue;
            }
            let pos = person.player.position;
            if !matches!(
                pos,
                Position::QB
                    | Position::RB
                    | Position::WR
                    | Position::TE
                    | Position::OT
                    | Position::IOL
                    | Position::DT
                    | Position::EDGE
                    | Position::LB
                    | Position::CB
                    | Position::S
            ) {
                continue;
            }
            let f = systems::fits_of(&w, i);
            let own = systems::fit_to(&f, pos, &prog.installed);
            let avg = mean(&installs.iter().map(|inst| systems::fit_to(&f, pos, inst)).collect::<Vec<_>>());
            let c = ((newest - person.class_of).max(0) as usize).min(5);
            by_class[c].0.push(own);
            by_class[c].1.push(avg);
            if person.class_of == newest {
                let s = (person.stars as usize).min(5);
                by_stars[s].0.push(own);
                by_stars[s].1.push(avg);
            }
            by_pos[pos.index()].push(own);
        }
    }
    println!(
        "\nfit by class (FBS college players, the readers' scale: tenths of a rating point at his job): his fit to his own school's systems against the average FBS school's"
    );
    println!("  class          players   own     average   own minus average");
    for (c, (own, avg)) in by_class.iter().enumerate() {
        if own.is_empty() {
            continue;
        }
        let name = ["newest", "2nd year", "3rd year", "4th year", "5th year", "6th+"][c];
        println!(
            "  {name:<12} {:>8}   {:>+6.3}  {:>+6.3}    {:>+6.3}",
            own.len(),
            mean(own),
            mean(avg),
            mean(own) - mean(avg)
        );
    }
    println!("\nrecruiting (the newest class by stars): own school against the average FBS school");
    println!("  stars   signees   own     average   own minus average");
    for s in (0..6).rev() {
        let (own, avg) = &by_stars[s];
        if own.is_empty() {
            continue;
        }
        println!(
            "  {s:<6} {:>8}   {:>+6.3}  {:>+6.3}    {:>+6.3}",
            own.len(),
            mean(own),
            mean(avg),
            mean(own) - mean(avg)
        );
    }
    println!("\na man's fit to his own school's systems by position (FBS college players, the readers' scale)");
    println!("  position   players   sd      p10     p90");
    for pos in Position::ALL {
        let v = &mut by_pos[pos.index()];
        if v.is_empty() {
            continue;
        }
        let s = sd(v);
        println!(
            "  {:<9} {:>8}   {:>5.3}  {:>+6.3}  {:>+6.3}",
            format!("{pos:?}"),
            v.len(),
            s,
            pct(v, 0.1),
            pct(v, 0.9)
        );
    }
}
