//! How many flaws and tools players carry by stage of career (individual
//! practice topic 3: a typical freshman 3 to 5 flaws; a drafted player 1
//! to 3 flaws and 1 or 2 tools; a veteran mostly tools): the same players
//! on the same dice through high school, college and the pros, by position
//! and by staff quality, with the average size of what they hold.
//!
//! Usage: craft_check [players a position] [seed] [coaching 1-20]

use afm_engine::craft::Craft;
use afm_engine::development::{SeasonContext, develop_season};
use afm_engine::player::{ProspectOptions, generate_prospect};
use afm_engine::{Level, PlayingTime, Position, Rng};

fn level_at(age: u32) -> Level {
    if age < 18 {
        Level::HS
    } else if age < 22 {
        Level::FBS
    } else {
        Level::NFL
    }
}

#[derive(Default, Clone, Copy)]
struct Tally {
    n: usize,
    flaws: f64,
    tools: f64,
    flaw_sev: f64,
    tool_sev: f64,
    none: usize,
}

impl Tally {
    fn add(&mut self, p: &afm_engine::Player) {
        self.n += 1;
        let (f, t) = (p.craft.flaws(), p.craft.tools());
        self.flaws += f as f64;
        self.tools += t as f64;
        if f == 0 {
            self.none += 1;
        }
        for (c, it) in p.craft.iter() {
            if c.is_flaw() {
                self.flaw_sev += it.sev();
            } else {
                self.tool_sev += it.sev();
            }
        }
    }
    fn row(&self) -> String {
        let n = self.n.max(1) as f64;
        format!(
            "{:>4.2} flaws (size {:.2}, none {:>4.1}%) {:>4.2} tools (size {:.2})",
            self.flaws / n,
            self.flaw_sev / self.flaws.max(1.0),
            100.0 * self.none as f64 / n,
            self.tools / n,
            self.tool_sev / self.tools.max(1.0)
        )
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(400);
    let seed: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(97);
    let coaching: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(11.0);
    let marks = [14u32, 18, 22, 26, 30];
    let mut all = vec![Tally::default(); marks.len()];
    // The better third at 22 by overall (the drafted, roughly).
    let mut top22 = Tally::default();
    let mut freq = vec![0usize; afm_engine::craft::CRAFT_COUNT];
    let mut freq_n = 0usize;
    println!("staff {coaching}, {n} a position, seed {seed}");
    println!("position | at 14 | at 18 (freshman) | at 22 | at 26 | at 30");
    for (pi, &pos) in Position::ALL.iter().enumerate() {
        let mut rows = vec![Tally::default(); marks.len()];
        let mut rng = Rng::new(seed.wrapping_add(pi as u32 * 7919));
        let mut at22: Vec<(f64, afm_engine::Player)> = Vec::new();
        for i in 0..n {
            let z = 1.2 + 0.6 * rng.normal(0.0, 1.0);
            let mut p = generate_prospect(
                &mut rng,
                ProspectOptions {
                    id: Some(800_000 + (pi * n + i) as u32),
                    position: Some(pos),
                    age: Some(14),
                    talent_z: Some(z),
                },
            );
            let mut r = Rng::new(seed.wrapping_mul(31).wrapping_add((pi * n + i) as u32));
            for (k, &m) in marks.iter().enumerate() {
                while p.age < m {
                    let ctx = SeasonContext::new(level_at(p.age), coaching, PlayingTime::Starter);
                    develop_season(&mut p, &ctx, &mut r);
                }
                rows[k].add(&p);
                all[k].add(&p);
                if m == 18 {
                    freq_n += 1;
                    for (c, _) in p.craft.iter() {
                        freq[c.index()] += 1;
                    }
                }
                if m == 22 {
                    at22.push((p.overall(), p.clone()));
                }
            }
        }
        at22.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (_, p) in at22.iter().take(n / 3) {
            top22.add(p);
        }
        println!(
            "{:>3} | {}",
            pos.name(),
            rows.iter()
                .map(|t| format!("{:.1}f {:.1}t", t.flaws / t.n as f64, t.tools / t.n as f64))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
    println!();
    for (k, &m) in marks.iter().enumerate() {
        println!("all at {m}: {}", all[k].row());
    }
    println!("better third at 22 (the drafted, roughly): {}", top22.row());
    println!("\nheld at 18, share of players at the position who carry it:");
    for c in Craft::ALL {
        let at: usize = Position::ALL.iter().filter(|p| c.fits(**p)).count();
        let share = freq[c.index()] as f64 / (n * at.max(1)) as f64;
        println!("  {:<34} {:>5.1}%", c.name(), 100.0 * share);
    }
    let _ = freq_n;
}
