//! A/B tests on attribute effects (`afm/ab-tests/plan.md`): does a body or
//! a rating change the stats the way it does in real football?
//!
//! Usage: ab_check <what> [nfl|fbs|hs] [games] [seed]
//! what: a test id (`cb_height`), a group (qb rb wr te ot iol dt edge lb cb
//! s st all links), `headline` (each position's headline row), `every`
//! (every test), or `summary` (merge a level's group files into its
//! summary).
//!
//! AFM_AB_OUT: the output folder (default `ab-out`); each level writes
//! `<out>/<level>/<group>.md` and `.json`. AFM_AB_WORLD: a world file whose
//! NFL rosters also play each headline row (NFL only). AFM_AB_MAX: the most
//! games an arm grows to (default 16,000). AFM_AB_REAL: the real numbers
//! (default `data/real-world/ab_real.json`).

extern crate afm_tools;
use std::collections::BTreeMap;
use std::time::Instant;

use afm_engine::player::Level;
use afm_tools::ab::run::{Ctx, Real, base_rows, group_md, report, run_test};
use afm_tools::ab::table::{self, GROUPS, Test};
use afm_tools::ab::{stats, synthetic, world_nfl};
use serde_json::{Value, json};

fn level_of(s: &str) -> Level {
    match s {
        "fbs" => Level::FBS,
        "fcs" => Level::FCS,
        "hs" => Level::HS,
        _ => Level::NFL,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let what = args.get(1).cloned().unwrap_or_else(|| "headline".into());
    let lv = args.get(2).cloned().unwrap_or_else(|| "nfl".into());
    let level = level_of(&lv);
    let games: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(4000);
    let seed: u32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1);
    let out = std::env::var("AFM_AB_OUT").unwrap_or_else(|_| "ab-out".into());
    let max: usize = std::env::var("AFM_AB_MAX").ok().and_then(|s| s.parse().ok()).unwrap_or(16_000);
    let dir = format!("{out}/{lv}");
    std::fs::create_dir_all(&dir).expect("make the output folder");
    if what == "summary" {
        summary(&dir, level);
        return;
    }
    let all = table::all();
    let chosen: Vec<&Test> = all
        .iter()
        .filter(|t| match what.as_str() {
            "every" => true,
            "headline" => t.headline,
            w if GROUPS.contains(&w) => t.group == w,
            w => t.id == w,
        })
        .collect();
    assert!(!chosen.is_empty(), "nothing matches {what}");
    let t0 = Instant::now();
    // The real numbers are the NFL's: college and high school check that
    // each effect points the same way and how its size moves.
    let real = if level == Level::NFL { Real::load() } else { Real(Value::Null) };
    if level == Level::NFL && real.0.is_null() {
        eprintln!("no real numbers found (data/real-world/ab_real.json): checks against real are skipped");
    }
    let base = synthetic(level, seed);
    let world = match (std::env::var("AFM_AB_WORLD"), level) {
        (Ok(p), Level::NFL) => {
            let w0 = Instant::now();
            let w = world_nfl(&p);
            eprintln!("world rosters: {} NFL clubs [{:.0} s]", w.len(), w0.elapsed().as_secs_f64());
            Some(w)
        }
        _ => None,
    };
    let st = stats();
    let cx = Ctx::new(level, seed, &base, &st);
    eprintln!("ab_check {what} {lv}: {} tests, {games} games an arm, seed {seed}", chosen.len());
    let base_league = base_rows(&cx, &chosen, false, games);
    let base_mine = base_rows(&cx, &chosen, true, games);
    // One group at a time, each written as soon as it is done.
    for g in GROUPS {
        let list: Vec<&&Test> = chosen.iter().filter(|t| t.group == g).collect();
        if list.is_empty() {
            continue;
        }
        let mut outs = Vec::new();
        for t in list {
            let tr = run_test(&cx, t, games, max, &real, world.as_deref());
            let b = if t.mine { base_mine.get(t.id) } else { base_league.get(t.id) };
            outs.push(report(&cx, &tr, b, &real));
        }
        // Keep what an earlier run of other tests in this group wrote.
        let path = format!("{dir}/{g}.json");
        let mut kept: BTreeMap<String, Value> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .and_then(|v| v["tests"].as_object().cloned())
            .map(|m| m.into_iter().collect())
            .unwrap_or_default();
        let mut kept_md: BTreeMap<String, String> = std::fs::read_to_string(format!("{dir}/{g}.parts.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let mut kept_miss: BTreeMap<String, Value> = std::fs::read_to_string(format!("{dir}/{g}.misses.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        for o in &outs {
            let id = o.json["id"].as_str().unwrap().to_string();
            kept.insert(id.clone(), o.json.clone());
            kept_md.insert(id.clone(), o.md.clone());
            kept_miss.insert(id, Value::Array(o.misses.clone()));
        }
        // Report order follows the table.
        let order: Vec<&str> = all.iter().filter(|t| t.group == g).map(|t| t.id).collect();
        let merged: Vec<afm_tools::ab::run::Out> = order
            .iter()
            .filter_map(|id| {
                Some(afm_tools::ab::run::Out {
                    md: kept_md.get(*id)?.clone(),
                    json: kept.get(*id)?.clone(),
                    misses: kept_miss.get(*id).and_then(|v| v.as_array().cloned()).unwrap_or_default(),
                })
            })
            .collect();
        std::fs::write(&path, json!({"level": lv, "seed": seed, "tests": kept}).to_string()).unwrap();
        std::fs::write(format!("{dir}/{g}.parts.json"), serde_json::to_string(&kept_md).unwrap()).unwrap();
        std::fs::write(format!("{dir}/{g}.misses.json"), serde_json::to_string(&kept_miss).unwrap()).unwrap();
        std::fs::write(format!("{dir}/{g}.md"), group_md(level, g, &merged, seed)).unwrap();
        eprintln!("wrote {dir}/{g}.md [{:.0} s so far]", t0.elapsed().as_secs_f64());
    }
    summary(&dir, level);
    eprintln!("done in {:.0} s", t0.elapsed().as_secs_f64());
}

/// Every miss of a level in one list.
fn summary(dir: &str, level: Level) {
    let mut md = format!("# A/B tests: every miss against real ({level:?})\n\n");
    let mut n = 0;
    for g in GROUPS {
        let Some(m) = std::fs::read_to_string(format!("{dir}/{g}.misses.json"))
            .ok()
            .and_then(|s| serde_json::from_str::<BTreeMap<String, Value>>(&s).ok())
        else {
            continue;
        };
        let rows: Vec<&Value> = m.values().filter_map(|v| v.as_array()).flatten().collect();
        if rows.is_empty() {
            continue;
        }
        md.push_str(&format!("\n## {}\n\n", table::group_name(g)));
        for r in rows {
            n += 1;
            md.push_str(&format!(
                "- {} (`{}`), {} ({}): {}\n",
                r["name"].as_str().unwrap_or(""),
                r["test"].as_str().unwrap_or(""),
                r["label"].as_str().unwrap_or(""),
                r["arms"].as_str().unwrap_or(""),
                r["check"].as_str().unwrap_or("")
            ));
        }
    }
    md.push_str(&format!("\n{n} misses in all.\n"));
    std::fs::write(format!("{dir}/summary.md"), md).unwrap();
}
