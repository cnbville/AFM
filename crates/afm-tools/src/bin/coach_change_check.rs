//! Who leaves after a head coaching change. Builds a world, then measures
//! (1) the AI's changes in the history: portal entrants per program in the
//! cycle of the change, by the old staff's last three seasons and why the
//! coach left, against programs that kept their coach; and (2) your own
//! takeover in a new save: you take a losing FBS program on the start
//! morning, the days run on, and each entrant from your roster is listed
//! with the reason the game gives.
//! Usage: coach_change_check [history_years] [seed] [programs]

use std::collections::HashMap;

use afm_engine::world::coach::{Employer, Role};
use afm_engine::world::data::Division;
use afm_engine::world::seat::{Job, You};
use afm_engine::world::{PortalRecord, World, WorldOptions};

fn you() -> You {
    let teaching = vec![12.0; afm_engine::world::coach::GROUP_COUNT];
    serde_json::from_value(serde_json::json!({
        "name": "Test Coach", "age": 45, "ethnicity": "White", "home_state": 0, "ex_pro": false, "reputation": 60.0,
        "teaching": teaching, "scheme": 12.0, "game_management": 12.0,
        "motivation": 12.0, "discipline": 12.0, "recruiting": 12.0, "network": 12.0, "relationships": 12.0,
        "style": { "risk": 10.0, "freedom": 10.0, "physical": 10.0, "ball_hunting": 10.0, "mould": 10.0 },
        "yours": { "play_calling": 12.0, "adaptation": 12.0, "evaluation": 12.0, "negotiating": 12.0 }
    }))
    .unwrap()
}

/// A program's record over the three seasons up to `year`.
fn last3(w: &World, p: usize, year: i32) -> (u32, u32) {
    w.programs[p]
        .seasons
        .iter()
        .filter(|s| s.year > year - 3 && s.year <= year)
        .fold((0, 0), |a, s| (a.0 + s.wins, a.1 + s.losses))
}

fn records(w: &World) -> Vec<&PortalRecord> {
    w.recruiting_log.portal.iter().collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let years: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(6);
    let seed: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let n_seats: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(6);
    let t = std::time::Instant::now();
    // The world is kept on disk: read back fresh for each takeover, and
    // reused by a later run.
    let file = std::env::temp_dir().join(format!("coach_change_{seed}_{years}.afmworld"));
    let w = if file.exists() {
        afm_engine::save::read_world(&file).expect("read")
    } else {
        let w = World::new(WorldOptions {
            seed,
            history_years: years,
            engine_from: Some(2100),
            hs_engine_years: None,
            ..Default::default()
        });
        afm_engine::save::write_world(&w, &file, "check", "now", None).expect("write");
        w
    };
    println!("built in {:.0}s; today {} (world year {})", t.elapsed().as_secs_f64(), w.today.pretty(), w.year);

    // (1) The AI's changes: entrants per FBS program and cycle.
    let mut entrants: HashMap<(usize, i32), (u32, u32)> = HashMap::new();
    for r in records(&w) {
        let e = entrants.entry((r.from_program, r.year)).or_default();
        e.0 += 1;
        if r.why == "coaching change" {
            e.1 += 1;
        }
    }
    let mut changes: HashMap<(usize, i32), &'static str> = HashMap::new();
    // A head coach who left for another college head job: (old program, cycle) to the new one.
    let mut moved: HashMap<(usize, i32), usize> = HashMap::new();
    for c in &w.coaches {
        for (k, s) in c.history.iter().enumerate() {
            if let (Employer::College(p), Role::HeadCoach) = (s.employer, s.role) {
                changes.insert((p, s.to), s.ended);
                let next = c.history.get(k + 1).map(|n| (n.employer, n.role)).or(c.job.map(|j| (j.employer, j.role)));
                if let Some((Employer::College(b), Role::HeadCoach)) = next
                    && b != p
                {
                    moved.insert((p, s.to), b);
                }
            }
        }
    }
    let first = w.year - years as i32 + 2;
    // Buckets: (label, programs, entrants, labelled "coaching change").
    let mut rows: Vec<(String, u32, u32, u32)> = Vec::new();
    let mut add = |label: String, e: (u32, u32)| {
        if let Some(r) = rows.iter_mut().find(|r| r.0 == label) {
            r.1 += 1;
            r.2 += e.0;
            r.3 += e.1;
        } else {
            rows.push((label, 1, e.0, e.1));
        }
    };
    for (p, prog) in w.programs.iter().enumerate() {
        if prog.division != Division::FBS {
            continue;
        }
        for year in first..w.year {
            if !prog.seasons.iter().any(|s| s.year == year) {
                continue;
            }
            let e = entrants.get(&(p, year)).copied().unwrap_or((0, 0));
            let (wn, l) = last3(&w, p, year);
            let pct = wn as f64 / (wn + l).max(1) as f64;
            let band = if pct < 0.35 {
                "last 3 under .350"
            } else if pct < 0.55 {
                "last 3 .350-.549"
            } else {
                "last 3 .550+"
            };
            match changes.get(&(p, year)) {
                Some(why) => {
                    let kind = if why.starts_with("fired") {
                        "fired"
                    } else if moved.contains_key(&(p, year)) {
                        "left for a head job"
                    } else {
                        "left (other job, retired...)"
                    };
                    add(format!("change, {kind}, {band}"), e);
                    add("change, all".into(), e);
                }
                None => {
                    add(format!("no change, {band}"), e);
                    add("no change, all".into(), e);
                }
            }
        }
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    println!("\nFBS portal entrants per program and cycle, {first}-{}", w.year - 1);
    println!("{:<44} {:>6} {:>9} {:>14}", "group", "cycles", "entrants", "of them 'coach'");
    for (label, n, e, c) in &rows {
        println!("{label:<44} {n:>6} {:>9.1} {:>14.1}", *e as f64 / *n as f64, *c as f64 / *n as f64);
    }
    // Followers (bug 86): players who left the old program for the coach's new one that cycle.
    let follows = |a: usize, b: usize, year: i32| {
        records(&w).iter().filter(|r| r.from_program == a && r.to_program == Some(b) && r.year == year).count()
    };
    let in_window: Vec<(&(usize, i32), &usize)> =
        moved.iter().filter(|((_, y), _)| *y >= first && *y < w.year).collect();
    let total: usize = in_window.iter().map(|((a, y), b)| follows(*a, **b, *y)).sum();
    let with: usize = in_window.iter().filter(|((a, y), b)| follows(*a, **b, *y) > 0).count();
    println!(
        "college head coaches who moved to another college head job: {}, followers {total} ({:.2} a move, {with} moves with one or more)",
        in_window.len(),
        total as f64 / in_window.len().max(1) as f64
    );
    let fbs: Vec<_> = in_window.iter().filter(|(_, b)| w.programs[**b].division == Division::FBS).collect();
    let total: usize = fbs.iter().map(|((a, y), b)| follows(*a, **b, *y)).sum();
    println!(
        "  to an FBS head job: {} moves, followers {total} ({:.2} a move)",
        fbs.len(),
        total as f64 / fbs.len().max(1) as f64
    );

    // (2) Your own takeover at a losing FBS program on the start morning.
    let last = w.year - 1;
    let mut losing: Vec<(usize, u32, u32)> = (0..w.programs.len())
        .filter(|&p| w.programs[p].division == Division::FBS && w.programs[p].head_coach.is_some())
        .map(|p| {
            let (wn, l) = last3(&w, p, last);
            (p, wn, l)
        })
        .filter(|&(_, wn, l)| wn + l >= 30 && (wn as f64) < 0.36 * (wn + l) as f64)
        .collect();
    losing.sort_by_key(|x| x.0);
    let pick: Vec<_> = losing.iter().step_by((losing.len() / n_seats.max(1)).max(1)).take(n_seats).collect();
    println!("\nYour takeover on {} at {} losing FBS programs (last three seasons)", w.today.pretty(), pick.len());
    // One world in memory at a time.
    drop(w);
    let mut total = 0;
    let mut whys: HashMap<String, u32> = HashMap::new();
    for &&(p, wn, l) in &pick {
        let mut seen: HashMap<u32, &'static str> = HashMap::new();
        let mut whys_role: HashMap<u32, String> = HashMap::new();
        let mut s = afm_engine::save::read_world(&file).expect("read");
        let d0 = s.today;
        let roster = s.people.iter().filter(|x| x.college == Some(p)).count();
        let jan = records(&s).iter().filter(|r| r.from_program == p && r.year == last).count();
        let leaning: Vec<f64> = s.people.iter().filter(|x| x.college == Some(p)).map(|x| x.cal.lean).collect();
        s.start_career(&you(), Job::College(p), 7);
        // A live save keeps no portal records: everyone from your roster
        // with an entry dated after your hire, read each day.
        for _ in 0..45 {
            s.advance_day();
            for x in s.people.iter().filter(|x| x.college == Some(p)) {
                if let Some(e) = x.cal.portal.as_ref().filter(|e| e.entered >= d0) {
                    seen.insert(x.player.id, e.why);
                    whys_role.insert(x.player.id, format!("{} ({:?})", e.why, x.last_role));
                }
            }
        }
        let n = seen.len();
        total += n;
        let mut by: HashMap<&str, u32> = HashMap::new();
        for (id, why) in &seen {
            *by.entry(why).or_default() += 1;
            *whys.entry(whys_role[id].clone()).or_default() += 1;
        }
        let with_lean = leaning.iter().filter(|&&x| x > 0.0).count();
        let mean = leaning.iter().sum::<f64>() / with_lean.max(1) as f64;
        println!(
            "{:<28} {wn:>2}-{l:<2} roster {roster:>3}, January window {jan:>2}, after your hire {n:>2} {by:?} ({with_lean} with a lean, mean {mean:.2})",
            s.programs[p].real_name
        );
    }
    println!("mean after your hire: {:.1}", total as f64 / pick.len().max(1) as f64);
    let mut v: Vec<_> = whys.into_iter().collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    println!("reasons by role: {v:?}");
}
