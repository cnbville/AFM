//! Dynasty check (world trial 3, item 1): what in a team turns into wins on
//! the match engine. Two equal synthetic FBS teams play many games; then one
//! side gets the same total rating boost spread in different ways (all
//! starters, QB only, the lines, receivers, the secondary), or a better
//! play caller. Prints win share and average margin for each.
//! Usage: dynasty_check [games] [seed], or dynasty_check value [games] [seed]
//! for points per rating point at every position.

use afm_engine::attributes::Attr;
use afm_engine::game::roster::depth_chart;
use afm_engine::game::synthetic::synthetic_team;
use afm_engine::game::{GameTeam, OffScheme, TeamStats, quick_game};
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

/// Raises the attributes of the depth-chart starters at `positions` by `by`:
/// every attribute but stamina (stamina changes how often he rotates out, so
/// it measures snaps, not skill), or only the ones named in `AFM_ATTRS`.
fn boost(t: &GameTeam, positions: &[Position], by: f64) -> GameTeam {
    let margin = match level() {
        Level::HS => 4.0,
        Level::NFL => 99.0,
        _ => 14.0,
    };
    let depth = depth_chart(t, margin);
    let only: Vec<String> =
        std::env::var("AFM_ATTRS").map(|v| v.split(',').map(String::from).collect()).unwrap_or_default();
    let attrs: Vec<Attr> = Attr::ALL
        .iter()
        .copied()
        .filter(|&a| if only.is_empty() { a != Attr::Stamina } else { only.iter().any(|o| *o == format!("{a:?}")) })
        .collect();
    let mut t = t.clone();
    for &(pos, n) in STARTERS.iter().filter(|(p, _)| positions.contains(p)) {
        for &i in depth.list(pos).iter().take(n) {
            for &a in &attrs {
                t.players[i].attrs[a] = (t.players[i].attrs[a] + by).min(99.0);
            }
        }
    }
    t
}

fn starters(positions: &[Position]) -> f64 {
    STARTERS.iter().filter(|(p, _)| positions.contains(p)).map(|s| s.1 as f64).sum()
}

/// Win share and average margin for `a` against `b`, home and away in turn.
fn level() -> Level {
    match std::env::var("AFM_LEVEL").as_deref() {
        Ok("nfl") => Level::NFL,
        Ok("hs") => Level::HS,
        _ => Level::FBS,
    }
}

fn play(a: &GameTeam, b: &GameTeam, games: usize, seed: u64) -> (f64, f64) {
    let r: Vec<(f64, f64, [TeamStats; 2])> = (0..games)
        .into_par_iter()
        .map(|g| {
            let s = seed * 1_000_003 + g as u64;
            let (m, st) = if g % 2 == 0 {
                let r = quick_game(a, b, level(), s);
                ((r.stats.score[0] - r.stats.score[1]) as f64, r.stats.team)
            } else {
                let r = quick_game(b, a, level(), s);
                let [x, y] = r.stats.team;
                ((r.stats.score[1] - r.stats.score[0]) as f64, [y, x])
            };
            (
                if m > 0.0 {
                    1.0
                } else if m == 0.0 {
                    0.5
                } else {
                    0.0
                },
                m,
                st,
            )
        })
        .collect();
    let n = r.len() as f64;
    if std::env::var("AFM_DETAIL").is_ok() {
        // How the edge shows up: each side's offense, boosted side first.
        for side in 0..2 {
            let sum = |f: &dyn Fn(&TeamStats) -> f64| r.iter().map(|x| f(&x.2[side])).sum::<f64>();
            let car = sum(&|t| t.carries as f64);
            let db = sum(&|t| t.dropbacks as f64);
            eprintln!(
                "    {} offense: ypc {:.2}  yds/dropback {:.2}  sack {:.3}  pressure {:.3}  int {:.4}  explosive/play {:.3}  pts {:.1}",
                if side == 0 { "boosted" } else { "base   " },
                sum(&|t| t.rush_yards) / car,
                sum(&|t| t.pass_yards) / db,
                sum(&|t| t.sacks as f64) / db,
                sum(&|t| t.pressures as f64) / db,
                sum(&|t| t.ints as f64) / db,
                sum(&|t| t.explosive as f64) / sum(&|t| t.plays as f64),
                sum(&|t| t.points as f64) / n
            );
        }
    }
    (r.iter().map(|x| x.0).sum::<f64>() / n, r.iter().map(|x| x.1).sum::<f64>() / n)
}

/// Position values: the same team plays itself, one side with every starter
/// at one position raised by `by` on every attribute. Prints points of
/// margin per rating point per player, against the real-world band.
fn values(games: usize, seed: u32, by: f64) {
    let base = synthetic_team("A", level(), 0.0, OffScheme::WestCoast, seed);
    // Real value per player per rating point, relative to the QB (see
    // data/real-world/position_value.md).
    let groups: [(&str, Vec<Position>, &str); 11] = [
        ("QB", vec![Position::QB], "1.00"),
        ("OT", vec![Position::OT], "0.15-0.30"),
        ("IOL", vec![Position::IOL], "0.08-0.18"),
        ("WR", vec![Position::WR], "0.15-0.30"),
        ("TE", vec![Position::TE], "0.07-0.18"),
        ("RB", vec![Position::RB], "0.05-0.15"),
        ("EDGE", vec![Position::EDGE], "0.15-0.30"),
        ("DT", vec![Position::DT], "0.10-0.25"),
        ("LB", vec![Position::LB], "0.05-0.15"),
        ("CB", vec![Position::CB], "0.12-0.25"),
        ("S", vec![Position::S], "0.08-0.18"),
    ];
    println!("{:?}, {games} games per line: a team against itself, one side's starters raised {by:+}\n", level());
    println!("| Position | starters | margin | points per rating point per player | share of QB | real share |");
    println!("|---|---|---|---|---|---|");
    let mut qb = f64::NAN;
    let only: Vec<String> =
        std::env::var("AFM_ONLY").map(|v| v.split(',').map(String::from).collect()).unwrap_or_default();
    for (label, ps, real) in groups {
        if !only.is_empty() && !only.iter().any(|o| o == label) && label != "QB" {
            continue;
        }
        let n = starters(&ps);
        let (_, m) = play(&boost(&base, &ps, by), &base, games, seed as u64);
        let per = m / (by * n);
        if label == "QB" {
            qb = per;
        }
        println!("| {label} | {n} | {m:+.2} | {per:+.3} | {:.2} | {real} |", per / qb);
    }
    if !only.is_empty() && !only.iter().any(|o| o == "coach") {
        return;
    }
    let mut t = base.clone();
    for c in [&mut t.staff.offense, &mut t.staff.defense] {
        c.skill = 18.0;
        c.adaptation = 18.0;
    }
    t.staff.game_management = 18.0;
    let mut u = base.clone();
    for c in [&mut u.staff.offense, &mut u.staff.defense] {
        c.skill = 4.0;
        c.adaptation = 4.0;
    }
    u.staff.game_management = 4.0;
    let (w, m) = play(&t, &u, games, seed as u64);
    println!("\nPlay callers and game management 18 against 4, same players: wins {:.1}%, margin {m:+.2}", 100.0 * w);
}

/// Scheme check: the same players, only the offensive scheme, coverage shell
/// or front changed on one side. A scheme should not be worth points by itself.
fn schemes(games: usize, seed: u32) {
    use afm_engine::game::{Front, Shell};
    let base = synthetic_team("A", level(), 0.0, OffScheme::WestCoast, seed);
    println!(
        "{:?}, {games} games per line, same players; base: {:?}, {:?}, {:?}\n",
        level(),
        base.staff.off_scheme,
        base.staff.shell,
        base.staff.front
    );
    let offs = [OffScheme::WestCoast, OffScheme::AirRaid, OffScheme::WideZone, OffScheme::PowerGap, OffScheme::SpreadOption];
    let shells = [Shell::Cover3, Shell::Quarters, Shell::ManPressure, Shell::Cover2];
    // AFM_MIX: judge each scheme against every scheme on the other side, not
    // only the base one. Both teams share the other unit, so the margin is
    // this offense against that shell minus West Coast against it (and this
    // shell against that offense minus Cover 2 against it).
    if std::env::var("AFM_MIX").is_ok() {
        for s in offs {
            let mut ms = Vec::new();
            for sh in shells {
                let (mut a, mut b) = (base.clone(), base.clone());
                a.staff.off_scheme = s;
                a.staff.shell = sh;
                b.staff.shell = sh;
                ms.push(play(&a, &b, games, seed as u64).1);
            }
            let avg = ms.iter().sum::<f64>() / ms.len() as f64;
            println!("offense {s:?}: margin {avg:+.2} (vs C3 {:+.2}, C4 {:+.2}, man {:+.2}, C2 {:+.2})", ms[0], ms[1], ms[2], ms[3]);
        }
        for sh in shells {
            let mut ms = Vec::new();
            for s in offs {
                let (mut a, mut b) = (base.clone(), base.clone());
                a.staff.off_scheme = s;
                b.staff.off_scheme = s;
                a.staff.shell = sh;
                ms.push(play(&a, &b, games, seed as u64).1);
            }
            let avg = ms.iter().sum::<f64>() / ms.len() as f64;
            println!(
                "shell {sh:?}: margin {avg:+.2} (vs WC {:+.2}, AR {:+.2}, WZ {:+.2}, PG {:+.2}, SO {:+.2})",
                ms[0], ms[1], ms[2], ms[3], ms[4]
            );
        }
        return;
    }
    for s in offs {
        let mut t = base.clone();
        t.staff.off_scheme = s;
        let (w, m) = play(&t, &base, games, seed as u64);
        println!("offense {s:?}: wins {:.1}%, margin {m:+.2}", 100.0 * w);
    }
    for sh in shells {
        let mut t = base.clone();
        t.staff.shell = sh;
        let (w, m) = play(&t, &base, games, seed as u64);
        println!("shell {sh:?}: wins {:.1}%, margin {m:+.2}", 100.0 * w);
    }
    for f in [Front::FourThree, Front::ThreeFour] {
        let mut t = base.clone();
        t.staff.front = f;
        let (w, m) = play(&t, &base, games, seed as u64);
        println!("front {f:?}: wins {:.1}%, margin {m:+.2}", 100.0 * w);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("value") {
        let games = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2000);
        let seed = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
        values(games, seed, 10.0);
        return;
    }
    if args.get(1).map(String::as_str) == Some("schemes") {
        let games = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2000);
        let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
        schemes(games, seed);
        return;
    }
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
