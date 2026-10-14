//! Dynasty check (world trial 3, item 1): what in a team turns into wins on
//! the match engine. Two equal synthetic FBS teams play many games; then one
//! side gets the same total rating boost spread in different ways (all
//! starters, QB only, the lines, receivers, the secondary), or a better
//! play caller. Prints win share and average margin for each.
//! Usage: dynasty_check [games] [seed]

use afm_engine::attributes::Attr;
use afm_engine::game::synthetic::synthetic_team;
use afm_engine::game::{GameTeam, OffScheme, quick_game};
use afm_engine::player::Level;
use afm_engine::positions::Position;
use rayon::prelude::*;

/// Starters per position in an 11-on-11 base lineup.
const STARTERS: [(Position, usize); 11] = [
    (Position::QB, 1),
    (Position::RB, 1),
    (Position::WR, 3),
    (Position::TE, 1),
    (Position::OT, 2),
    (Position::IOL, 3),
    (Position::EDGE, 2),
    (Position::DT, 2),
    (Position::LB, 2),
    (Position::CB, 2),
    (Position::S, 2),
];

/// Raises every attribute of the starters at `positions` by `by`.
fn boost(t: &GameTeam, positions: &[Position], by: f64) -> GameTeam {
    let mut t = t.clone();
    for &(pos, n) in STARTERS.iter().filter(|(p, _)| positions.contains(p)) {
        for p in t.players.iter_mut().filter(|p| p.position == pos).take(n) {
            for a in Attr::ALL {
                p.attrs[a] = (p.attrs[a] + by).min(99.0);
            }
        }
    }
    t
}

fn starters(positions: &[Position]) -> f64 {
    STARTERS.iter().filter(|(p, _)| positions.contains(p)).map(|s| s.1 as f64).sum()
}

/// Win share and average margin for `a` against `b`, home and away in turn.
fn play(a: &GameTeam, b: &GameTeam, games: usize, seed: u64) -> (f64, f64) {
    let r: Vec<(f64, f64)> = (0..games)
        .into_par_iter()
        .map(|g| {
            let s = seed * 1_000_003 + g as u64;
            let (m, home) = if g % 2 == 0 {
                let r = quick_game(a, b, Level::FBS, s);
                ((r.stats.score[0] - r.stats.score[1]) as f64, true)
            } else {
                let r = quick_game(b, a, Level::FBS, s);
                ((r.stats.score[1] - r.stats.score[0]) as f64, false)
            };
            let _ = home;
            (
                if m > 0.0 {
                    1.0
                } else if m == 0.0 {
                    0.5
                } else {
                    0.0
                },
                m,
            )
        })
        .collect();
    let n = r.len() as f64;
    (r.iter().map(|x| x.0).sum::<f64>() / n, r.iter().map(|x| x.1).sum::<f64>() / n)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1000);
    let seed: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    // Same scheme on both sides so the only difference is the boost.
    let a = synthetic_team("A", Level::FBS, 0.0, OffScheme::WestCoast, seed);
    let b = synthetic_team("B", Level::FBS, 0.0, OffScheme::WestCoast, seed + 1);
    let all: Vec<Position> = STARTERS.iter().map(|s| s.0).collect();
    let total = starters(&all);
    let (w0, m0) = play(&a, &b, games, 1);
    println!("FBS, {games} games per line, team A against team B\n");
    println!("| A's change | lineup average | A wins | A's margin | margin vs no change |");
    println!("|---|---|---|---|---|");
    println!("| none | +0.0 | {:.1}% | {:+.1} | |", 100.0 * w0, m0);
    let row = |label: &str, t: &GameTeam, avg: f64| {
        let (w, m) = play(t, &b, games, 1);
        println!("| {label} | {avg:+.1} | {:.1}% | {m:+.1} | {:+.1} |", 100.0 * w, m - m0);
    };
    for by in [1.0, 3.0] {
        row(&format!("all starters +{by}"), &boost(&a, &all, by), by);
    }
    // The same total boost (lineup average +0.5) put in one place.
    let budget = 0.5 * total;
    for (label, ps) in [
        ("QB", vec![Position::QB]),
        ("offensive line", vec![Position::OT, Position::IOL]),
        ("defensive line", vec![Position::EDGE, Position::DT]),
        ("receivers and TE", vec![Position::WR, Position::TE]),
        ("RB", vec![Position::RB]),
        ("secondary", vec![Position::CB, Position::S]),
        ("linebackers", vec![Position::LB]),
    ] {
        let by = budget / starters(&ps);
        row(&format!("{label} +{by:.1} each"), &boost(&a, &ps, by), 0.5);
    }
    row("all starters +0.5", &boost(&a, &all, 0.5), 0.5);
    // Coaching: the same players, a far better or worse play caller.
    for (label, skill) in [("play callers and game management 18", 18.0), ("play callers and game management 4", 4.0)] {
        let mut t = a.clone();
        t.staff.offense.skill = skill;
        t.staff.defense.skill = skill;
        t.staff.offense.adaptation = skill;
        t.staff.defense.adaptation = skill;
        t.staff.game_management = skill;
        row(label, &t, 0.0);
    }
}
