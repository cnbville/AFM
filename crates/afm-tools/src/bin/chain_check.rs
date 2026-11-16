//! The high school chain reaction's world check (Ground lane G11a): reads a
//! world file and prints how high school prestige moved over the history
//! (powerhouses gained and lost by state, the real top programs, the
//! biggest climbers and fallers with why), the June transfers and move-ins
//! by class, and where the blue chips played. A world from before the chain
//! prints its transfers and blue chips only, for the comparison.
//! Usage: chain_check <world file>

use afm_engine::world::high_school::chain_why;
use afm_engine::world::{Stage, World};
use std::collections::BTreeMap;
use std::path::Path;

fn main() {
    let path = std::env::args().nth(1).expect("a world file");
    let w = afm_engine::save::read_world(Path::new(&path)).expect("read the world");
    transfers(&w);
    blue_chips(&w);
    // AFM_STATE=GA: that state's schools by their old players' score.
    if let Ok(code) = std::env::var("AFM_STATE") {
        state_table(&w, &code);
        return;
    }
    if w.high_schools.iter().any(|h| !h.chain.years.is_empty()) {
        ladder(&w);
    } else {
        println!("\nNo chain in this world (built before it).");
    }
}

/// The June moves by class, from each player's notes.
fn transfers(w: &World) {
    println!("High school moves by class (players still on record), per 1,000 in the class:");
    println!("class  players  to a power  move-ins  buried out  all moves  per 1,000");
    let mut by: BTreeMap<i32, [u32; 5]> = BTreeMap::new();
    for p in &w.people {
        if !(2016..=2029).contains(&p.class_of) || matches!(p.stage, Stage::Done) {
            continue;
        }
        let e = by.entry(p.class_of).or_default();
        e[0] += 1;
        for n in &p.notes {
            if n.contains("he transferred from") {
                e[1] += 1;
                e[4] += 1;
                if n.contains("moving into its district") {
                    e[2] += 1;
                }
            } else if n.contains("buried on") {
                e[3] += 1;
                e[4] += 1;
            }
        }
    }
    for (c, e) in &by {
        println!(
            "{c}  {:>8}  {:>10}  {:>8}  {:>10}  {:>9}  {:>9.1}",
            e[0],
            e[1],
            e[2],
            e[3],
            e[4],
            1000.0 * e[4] as f64 / e[0].max(1) as f64
        );
    }
}

/// Where the 4- and 5-stars of the classes 2020-2026 finished high school.
fn blue_chips(w: &World) {
    let (mut all, mut power, mut real) = (0u32, 0u32, 0u32);
    let mut prestige = 0.0;
    for p in w.people.iter().filter(|p| (2020..=2026).contains(&p.class_of) && p.stars >= 4) {
        let h = &w.high_schools[p.hs];
        all += 1;
        power += h.is_powerhouse() as u32;
        real += h.real as u32;
        prestige += h.prestige;
    }
    println!(
        "\nBlue chips, classes 2020-2026: {all}; at today's powerhouses {:.1}%, at the states' real top-five schools {:.1}%, their schools' mean prestige {:.3}.",
        100.0 * power as f64 / all.max(1) as f64,
        100.0 * real as f64 / all.max(1) as f64,
        prestige / all.max(1) as f64
    );
    let powerhouses = w.high_schools.iter().filter(|h| h.is_powerhouse()).count();
    println!("Powerhouses today: {powerhouses} of {} schools.", w.high_schools.len());
}

/// Prestige over the history: the ladder by state, the real top schools,
/// the climbers and fallers.
fn ladder(w: &World) {
    let first = w.high_schools.iter().filter_map(|h| h.chain.years.first().map(|y| y.0)).min().unwrap();
    let last = w.high_schools.iter().filter_map(|h| h.chain.years.last().map(|y| y.0)).max().unwrap();
    println!("\nThe chain, {first}-{last}.");
    let at = |h: &afm_engine::world::high_school::HighSchool, y: i32| {
        h.chain.years.iter().find(|x| x.0 == y).map_or(h.chain.start, |x| x.1 as f64)
    };
    println!("Powerhouses held by a school that didn't start as one, and starters that lost it, every five years:");
    let mut y = first;
    while y <= last {
        let (mut held, mut new, mut lost) = (0, 0, 0);
        for h in &w.high_schools {
            let now = at(h, y) >= 0.75;
            let start = h.chain.start >= 0.75;
            held += now as u32;
            new += (now && !start) as u32;
            lost += (start && !now) as u32;
        }
        println!("  {y}: {held} powerhouses, {new} new, {lost} starters out");
        y += 5;
    }
    // The real top-five schools.
    let real: Vec<_> = w.high_schools.iter().filter(|h| h.real).collect();
    let kept = real.iter().filter(|h| h.is_powerhouse()).count();
    let mean_start = real.iter().map(|h| h.chain.start).sum::<f64>() / real.len().max(1) as f64;
    let mean_now = real.iter().map(|h| h.prestige).sum::<f64>() / real.len().max(1) as f64;
    println!(
        "Real top-five schools: {} ; still powerhouses {kept}; mean prestige {mean_start:.3} at the start, {mean_now:.3} now.",
        real.len()
    );
    // How far schools moved.
    let moved: Vec<f64> = w.high_schools.iter().map(|h| h.prestige - h.chain.start).collect();
    let mean_abs = moved.iter().map(|d| d.abs()).sum::<f64>() / moved.len().max(1) as f64;
    let up10 = moved.iter().filter(|&&d| d >= 0.1).count();
    let down10 = moved.iter().filter(|&&d| d <= -0.1).count();
    println!(
        "Moved since the start: mean {mean_abs:.3}; {up10} schools up 0.10 or more, {down10} down 0.10 or more, of {}.",
        moved.len()
    );
    // By state.
    println!("By state (powerhouses at the start / today / new):");
    let mut line = String::new();
    for (s, st) in w.states.iter().enumerate() {
        let schools: Vec<_> = w.high_schools.iter().filter(|h| h.state == s).collect();
        let start = schools.iter().filter(|h| h.chain.start >= 0.75).count();
        let now = schools.iter().filter(|h| h.is_powerhouse()).count();
        let new = schools.iter().filter(|h| h.is_powerhouse() && h.chain.start < 0.75).count();
        line.push_str(&format!("{} {start}/{now}/{new}  ", st.code));
        if line.len() > 90 {
            println!("  {line}");
            line.clear();
        }
    }
    if !line.is_empty() {
        println!("  {line}");
    }
    // Climbers and fallers.
    let mut order: Vec<usize> = (0..w.high_schools.len()).collect();
    order.sort_by(|&a, &b| moved[b].total_cmp(&moved[a]));
    let show = |h: usize| {
        let s = &w.high_schools[h];
        println!("  {} ({}): {}", w.school_name(h), w.states[s.state].code, chain_why(s, first));
    };
    println!("Biggest climbers:");
    order.iter().take(10).for_each(|&h| show(h));
    println!("Biggest fallers:");
    order.iter().rev().take(10).for_each(|&h| show(h));
    // How production and prestige line up today.
    let n = w.high_schools.len() as f64;
    let (mx, my) = (
        w.high_schools.iter().map(|h| h.chain.made as f64).sum::<f64>() / n,
        w.high_schools.iter().map(|h| h.prestige).sum::<f64>() / n,
    );
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for h in &w.high_schools {
        let (x, y) = (h.chain.made as f64 - mx, h.prestige - my);
        sxy += x * y;
        sxx += x * x;
        syy += y * y;
    }
    println!(
        "Old players' score against prestige today: correlation {:.2}; mean score {mx:.2}; top-decile schools' share of the score {:.0}%.",
        sxy / (sxx * syy).sqrt().max(1e-9),
        {
            let mut v: Vec<f64> = w.high_schools.iter().map(|h| h.chain.made as f64).collect();
            v.sort_by(|a, b| b.total_cmp(a));
            let top: f64 = v.iter().take(v.len() / 10).sum();
            100.0 * top / v.iter().sum::<f64>().max(1e-9)
        }
    );
    let entries: usize = w.high_schools.iter().map(|h| h.chain.years.len()).sum();
    println!("Chain store: {entries} yearly entries ({:.1} MB).", entries as f64 * 8.0 / 1e6);
}

/// One state's schools, best score first: score, counts, prestige now and
/// at the start, enrollment.
fn state_table(w: &World, code: &str) {
    let s = w.states.iter().position(|x| x.code == code).expect("a state");
    let mut score = vec![0.0f64; w.high_schools.len()];
    for p in &w.people {
        if let Some((v, _, _)) = afm_engine::world::high_school::alumni_value(w, p) {
            score[p.hs] += v;
        }
    }
    let schools = w.states[s].schools.clone();
    let moves = afm_engine::world::high_school::state_form(
        &schools.iter().map(|&h| (w.high_schools[h].prestige, score[h])).collect::<Vec<_>>(),
    );
    let mut rows: Vec<usize> = (0..schools.len()).collect();
    rows.sort_by(|&a, &b| w.high_schools[schools[b]].prestige.total_cmp(&w.high_schools[schools[a]].prestige));
    println!("school                          score  college  power  nfl  prestige  start  enroll  real  form now");
    for k in rows.into_iter().take(25) {
        let h = schools[k];
        let x = &w.high_schools[h];
        let c = &x.chain;
        println!(
            "{:<30} {:>6.1} {:>8} {:>6} {:>4} {:>9.2} {:>6.2} {:>7} {:>5} {:>+8.3}",
            w.school_name(h).chars().take(30).collect::<String>(),
            score[h],
            c.college,
            c.power,
            c.nfl,
            x.prestige,
            c.start,
            x.enrollment,
            x.real,
            moves[k].unwrap_or(0.0)
        );
    }
}
