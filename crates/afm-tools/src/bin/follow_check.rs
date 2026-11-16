//! Ground lane G8's check on a real world (GR42-GR46): reads a world file
//! (history on the match engine, so the NFL's honors are there), plays it
//! on for a number of days with games on the stand-in, and prints the
//! famous names, the class's biggest followings, the big and huge
//! commitment news, the brand lifts and the storms.
//! Usage: follow_check <world file> <days>

use std::collections::BTreeMap;
use std::path::Path;

use afm_engine::world::{Stage, World};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let t = std::time::Instant::now();
    let mut w = afm_engine::save::read_world(Path::new(&args[1])).expect("read the world");
    eprintln!("read in {:.0} s, {:?}", t.elapsed().as_secs_f64(), w.today);
    w.options.engine_from = Some(2100);
    let days: u32 = args[2].parse().expect("days");
    for _ in 0..days {
        w.advance_day();
    }
    eprintln!("played to {:?} in {:.0} s", w.today, t.elapsed().as_secs_f64());
    following(&w);
}
/// Ground lane G8: famous names, the class's biggest followings, the
/// commitments' big and huge news, brand lifts and storms (GR42-GR46).
fn following(w: &World) {
    use afm_engine::calendar::events::{Kind, Loudness};
    use afm_engine::world::storylines::K;
    println!("\n## A following and famous names (Ground lane G8)");
    let f = &w.following;
    let sons = f.names.iter().filter(|n| !n.nephew).count();
    println!("Famous names: {} ({sons} sons, {} nephews).", f.names.len(), f.names.len() - sons);
    let mut by_year: BTreeMap<i32, u32> = BTreeMap::new();
    for n in &f.names {
        *by_year.entry(n.year).or_default() += 1;
    }
    println!("  joining by year: {by_year:?}");
    let mut tiers: BTreeMap<String, u32> = BTreeMap::new();
    for n in &f.names {
        *tiers.entry(format!("{:.2}", n.fame)).or_default() += 1;
    }
    println!("  by the man's fame: {tiers:?}");
    let live: Vec<_> =
        f.names.iter().filter_map(|n| w.people.iter().find(|p| p.player.id == n.id).map(|p| (n, p))).collect();
    let stars: Vec<u32> = (0..=5).map(|s| live.iter().filter(|(_, p)| p.stars == s).count() as u32).collect();
    println!("  still in the world: {}, by stars 0-5: {stars:?}", live.len());
    let mut best: Vec<_> = live.iter().collect();
    best.sort_by_key(|(_, p)| std::cmp::Reverse(p.followers));
    for (n, p) in best.iter().take(8) {
        let at = p.college.map(|c| w.program_name(c).to_string()).unwrap_or_else(|| format!("{:?}", p.stage));
        println!(
            "  {} ({}, {}-star {}, class of {}): {} followers, {}; {}",
            p.name,
            if n.nephew { "nephew" } else { "son" },
            p.stars,
            p.player.position.name(),
            p.class_of,
            p.followers,
            n.line(),
            at
        );
    }
    println!("Class biggest followings now (class: biggest, tenth):");
    for c in &f.classes {
        println!("  {}: {}, {}", c.class_of, c.top, c.tenth);
    }
    let commits: Vec<_> = w.calendar.history.iter().filter(|e| e.kind == Kind::Commitment).collect();
    let huge = commits.iter().filter(|e| e.loud == Loudness::Huge).count();
    println!("Big and huge commitment news kept: {} ({huge} huge). Last five:", commits.len());
    for e in commits.iter().rev().take(5) {
        println!("  {}: {}", e.when.world_date().pretty(), e.text);
    }
    let mut lifted: Vec<_> = w.programs.iter().filter(|p| p.brand_lift > 0.0).collect();
    lifted.sort_by(|a, b| b.brand_lift.total_cmp(&a.brand_lift));
    println!("Programs with a brand lift now: {}; biggest:", lifted.len());
    for p in lifted.iter().take(5) {
        println!("  {}: +{:.2}", w.program_name(p.id), p.brand_lift);
    }
    let storm = K::SocialStorm as u16;
    let mut storms: BTreeMap<String, u32> = BTreeMap::new();
    for s in w.storylines.open.iter().chain(&w.storylines.ended).filter(|s| s.kind == storm) {
        *storms.entry(format!("{:?}", s.level)).or_default() += 1;
    }
    println!("Social media storms kept, by level: {storms:?}");
    let big: Vec<u32> = [Stage::HighSchool, Stage::College]
        .iter()
        .map(|&st| w.people.iter().filter(|p| p.stage == st && p.followers >= 500_000).count() as u32)
        .collect();
    println!("Players with 500,000 followers now: high school {}, college {}.", big[0], big[1]);
}
