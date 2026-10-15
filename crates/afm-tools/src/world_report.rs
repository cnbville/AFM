//! Headline checks for a built world.

use std::collections::BTreeMap;

use afm_engine::world::World;
use afm_engine::world::data::{self, Division};
use afm_engine::world::program::{GradeCat, letter};

pub fn print(w: &World) {
    let picks: Vec<_> = w.alumni.iter().filter(|a| a.pick.is_some()).collect();
    let mut by_div: BTreeMap<&str, usize> = BTreeMap::new();
    for a in &picks {
        let d = w.programs[*a.colleges.last().unwrap()].division;
        *by_div.entry(d.name()).or_default() += 1;
    }
    println!("draft picks: {} total; by division: {:?}", picks.len(), by_div);
    let juco_picks = picks.iter().filter(|a| a.juco).count();
    println!("picks with a JUCO stop: {juco_picks}");
    // Star draft rates over complete classes.
    let mut star_total = [0u32; 6];
    let mut drafted = [0u32; 6];
    let last_draft = w.year;
    for &(y, c) in &w.signing_classes {
        if y + 5 > last_draft {
            continue;
        }
        for s in 0..6 {
            star_total[s] += c[s];
        }
        for a in &picks {
            if a.year >= y + 3
                && a.year <= y + 5
                && a.notes.iter().any(|n| n.contains(&format!("as a {}-star", a.stars)))
            {
                // counted below
            }
        }
    }
    for a in &picks {
        // Class year: his first signing note's year isn't stored; approximate via draft year - 4.
        let y = a.year - 4;
        if w.signing_classes.iter().any(|&(sy, _)| sy == y && sy + 5 <= last_draft) {
            drafted[a.stars as usize] += 1;
        }
    }
    for s in (2..=5).rev() {
        if star_total[s] > 0 {
            println!(
                "{s}-star draft rate (approx): {:.1}% of {}",
                100.0 * drafted[s] as f64 / star_total[s] as f64,
                star_total[s]
            );
        }
    }
    // Prestige top 15.
    let mut fbs: Vec<_> = w.programs.iter().filter(|p| p.division == Division::FBS).collect();
    fbs.sort_by(|a, b| b.prestige.total_cmp(&a.prestige));
    println!("top FBS prestige:");
    for p in fbs.iter().take(15) {
        let g = |c: GradeCat| letter(p.grades[c as usize]);
        println!(
            "  {:<22} {:.2}  titles {:>2} conf {:>2}  NIL ${:.1}M  title chances {} coach {} pipeline {}",
            p.real_name,
            p.prestige,
            p.titles,
            p.conference_titles,
            p.collective.budget / 1e6,
            g(GradeCat::TitleChances),
            g(GradeCat::CoachPrestige),
            g(GradeCat::ProPipeline)
        );
    }
    let champs: Vec<String> = w
        .programs
        .iter()
        .filter(|p| p.division == Division::FBS)
        .flat_map(|p| p.seasons.iter().filter(|s| s.national_title).map(move |s| (s.year, p.real_name.clone())))
        .collect::<BTreeMap<_, _>>()
        .into_iter()
        .map(|(y, n)| format!("{y} {n}"))
        .collect();
    println!("FBS champions: {}", champs.join(", "));
    let total: f64 = data::STATES.iter().map(|s| s.fbs_signees_per_year).sum();
    let _ = total;
    let nil_warn =
        w.news.iter().filter(|n| matches!(n.kind, afm_engine::world::news::NewsKind::NilWarning { .. })).count();
    println!("news items: {}, NIL warnings: {}", w.news.len(), nil_warn);
}

/// Strength and talent by program group, to check talent concentrates where it should.
pub fn talent(w: &World) {
    use afm_engine::world::Stage;
    let mut stats: BTreeMap<String, (f64, f64, u32, u32)> = BTreeMap::new();
    let mut per_prog: Vec<(f64, u32, u32)> = vec![(0.0, 0, 0); w.programs.len()];
    for p in &w.people {
        if p.stage != Stage::College {
            continue;
        }
        let c = p.college.unwrap();
        per_prog[c].0 += p.player.overall();
        per_prog[c].1 += 1;
        if p.stars >= 4 && !p.player.position.is_specialist() {
            per_prog[c].2 += 1;
        }
    }
    for (i, prog) in w.programs.iter().enumerate() {
        let group = match prog.division {
            Division::FBS if prog.power_four() => "FBS P4".to_string(),
            d => d.name().to_string(),
        };
        let e = stats.entry(group).or_default();
        e.0 += per_prog[i].0;
        e.2 += per_prog[i].1;
        e.3 += per_prog[i].2;
    }
    for (g, (sum, _, n, blue)) in &stats {
        println!("{g:<8} players {n:>6}  avg overall {:.1}  4-5 stars {blue}", sum / (*n).max(1) as f64);
    }
    let mut strongest: Vec<(f64, &str, f64)> = w
        .programs
        .iter()
        .filter_map(|p| p.seasons.last().map(|s| (s.strength, p.real_name.as_str(), p.prestige)))
        .collect();
    strongest.sort_by(|a, b| b.0.total_cmp(&a.0));
    let top: Vec<String> = strongest.iter().take(15).map(|(s, n, pr)| format!("{n} {s:.1} ({pr:.2})")).collect();
    println!("strongest last season: {}", top.join(", "));
}

/// How blue-chips ended up at JUCO.
pub fn juco_blue_chips(w: &World) {
    use afm_engine::world::Stage;
    let mut n = 0;
    for p in &w.people {
        if p.stage == Stage::College
            && p.stars >= 4
            && !p.player.position.is_specialist()
            && p.college.is_some_and(|c| w.programs[c].division == Division::JUCO)
        {
            n += 1;
            if n <= 6 {
                println!(
                    "JUCO {}-star gpa {:.2}: {}",
                    p.stars,
                    afm_engine::debug::person_background(p).0.gpa,
                    p.notes.join(" | ")
                );
            }
        }
    }
}
