//! The scheme duel test (section 12, `docs/design/game-planning.md`, item 16
//! and section 14, lane GP0): the gate every game-planning lane passes.
//!
//! A sharp play caller plays a poor one on the same games. Both callers are
//! the offensive and the defensive coordinator of their team; every call
//! skill (play calling, reading, situational, adaptation, creativity) sits at
//! the same rating, `AFM_DUEL_SHARP` (17) against `AFM_DUEL_POOR` (5). The
//! rosters come from the level's synthetic team generator, the sharp
//! caller's at quality 0 and the poor caller's at quality `delta` better,
//! for each delta in `AFM_DUEL_DELTAS`. Three things come out of it:
//!
//! 1. The coaching edge: the sharp team's margin with the callers set, less
//!    its margin with both callers average, on the same games (paired, so the
//!    rosters cancel). The real band is 3 to 7 points a game.
//! 2. The upsets: how often the roster favorite wins with average callers
//!    (ratings only) and with the duel, by the roster spread, against real
//!    favorites by spread (`data/real-world/sequencing.json`, item 5) and the
//!    real margin spread around the line (12.5 points, NFL).
//! 3. A lane's own share: `AFM_DUEL_OFF` names mechanisms to switch off
//!    (`each` plays one arm for every known mechanism). An engine lane reads
//!    the variable `AFM_GP_OFF` (a comma list of mechanism names, read once
//!    per game, never cached across games) and skips its mechanism when its
//!    name is listed; this tool sets it around the arm's games and clears
//!    it after. The share is the edge with the mechanism on less the edge
//!    with it off. An arm that plays exactly like the full duel says so:
//!    the engine does not read that switch yet (all of them, today).
//!
//! Measurement only: no engine change. Every arm plays the same games on the
//! same seeds, the sharp team at home on even games.
//!
//! `cargo run --release --bin duel_check -- <nfl|fbs|hs> [games] [seed] [out.md]`
//! `games` is the games for each delta in each arm (default 1,000).

use std::fmt::Write as _;

use afm_engine::game::roster::{Caller, GameTeam};
use afm_engine::game::synthetic::{synthetic_league, synthetic_team};
use afm_engine::game::{Conditions, Venue, quick_game_in};
use afm_engine::player::Level;
use rayon::prelude::*;

/// The mechanisms later lanes add, in lane order (`docs/design/game-planning.md`,
/// section 15). `AFM_GP_OFF` carries these names.
const MECHANISMS: &[(&str, &str)] = &[
    ("direction", "GP1 direction memory"),
    ("keys", "GP1 defender keys and false steps"),
    ("series", "GP2 series links"),
    ("selfscout", "GP2 self-scouting and chosen breakers"),
    ("read_defenders", "GP3 read and conflict defenders"),
    ("gap_blocking", "GP3 gap-scheme blocking"),
    ("disguise", "GP4 disguise and rotation"),
    ("match_rules", "GP4 match rules"),
    ("pressure", "GP5 pressure and protection"),
    ("tempo", "GP5 time before the snap"),
    ("situations", "GP6 situations"),
    ("adjustments", "GP6 adjustments and the change budget"),
    ("calibration", "GP7 calibration"),
];

/// The variable an engine lane reads (comma list of mechanism names).
const OFF_VAR: &str = "AFM_GP_OFF";

/// Real 2024-25 favorites by spread (`data/real-world/sequencing.json`,
/// `5_upsets_by_spread`), as the data file has it.
const REAL: &str = include_str!("../../../../data/real-world/sequencing.json");

fn level_of(s: &str) -> Level {
    match s {
        "fbs" => Level::FBS,
        "fcs" => Level::FCS,
        "hs" => Level::HS,
        _ => Level::NFL,
    }
}

fn env_f(name: &str, default: f64) -> f64 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// A caller whose every call skill sits at one rating; style stays middle.
fn caller(r: f64) -> Caller {
    Caller { skill: r, adaptation: r, aggressiveness: 10.0, reading: r, situational: r, creativity: r, nerve: 0.0 }
}

#[derive(Clone, Copy, PartialEq)]
enum Arm {
    /// Both coordinators average: the ratings alone.
    Control,
    /// Team A's sharp, team B's poor.
    Duel,
}

struct Cfg {
    level: Level,
    games: usize,
    seed: u32,
    sharp: f64,
    poor: f64,
    deltas: Vec<f64>,
    /// The head coach's game management goes with the callers too.
    head: bool,
}

fn game_seed(seed: u32, g: usize, d: usize) -> u64 {
    // The seed does not depend on the arm, so every arm plays the same
    // game; it does depend on the delta so each delta is its own sample.
    seed as u64 * 1_000_003 + (d as u64) * 7_000_001 + g as u64
}

/// One game's margin for team A (the sharp caller's team) in an arm.
fn margin(cfg: &Cfg, league: &[GameTeam], g: usize, d: usize, arm: Arm) -> f64 {
    let n = league.len();
    let sa = league[g % n].staff.off_scheme;
    let sb = league[(g / n + g + 1) % n].staff.off_scheme;
    let ra = cfg.seed.wrapping_add(g as u32 * 2 + 1);
    let rb = cfg.seed.wrapping_add(g as u32 * 2 + 2);
    let mut a = synthetic_team("Sharp", cfg.level, 0.0, sa, ra);
    let mut b = synthetic_team("Poor", cfg.level, cfg.deltas[d], sb, rb);
    if arm == Arm::Duel {
        for (t, r) in [(&mut a, cfg.sharp), (&mut b, cfg.poor)] {
            t.staff.offense = caller(r);
            t.staff.defense = caller(r);
            if cfg.head {
                t.staff.game_management = r;
            }
        }
    }
    let home_a = g.is_multiple_of(2);
    let venue =
        Venue { cond: Conditions { crowd: 0.65, ..Conditions::default() }, wear: [0.0, 0.012], ..Venue::default() };
    let (h, w) = if home_a { (&a, &b) } else { (&b, &a) };
    let r = quick_game_in(h, w, cfg.level, game_seed(cfg.seed, g, d), Default::default(), venue);
    let (sh, sw) = (r.stats.score[0] as f64, r.stats.score[1] as f64);
    if home_a { sh - sw } else { sw - sh }
}

/// An arm over every delta: `[delta][game]` margins for team A.
fn phase(cfg: &Cfg, league: &[GameTeam], arm: Arm) -> Vec<Vec<f64>> {
    (0..cfg.deltas.len())
        .map(|d| (0..cfg.games).into_par_iter().map(|g| margin(cfg, league, g, d, arm)).collect())
        .collect()
}

/// A phase with mechanisms switched off through `AFM_GP_OFF`.
fn phase_off(cfg: &Cfg, league: &[GameTeam], off: &str) -> Vec<Vec<f64>> {
    // Set before and cleared after the games, with no other thread running.
    unsafe { std::env::set_var(OFF_VAR, off) };
    let m = phase(cfg, league, Arm::Duel);
    unsafe { std::env::remove_var(OFF_VAR) };
    m
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len().max(1) as f64
}

fn sd(v: &[f64]) -> f64 {
    let m = mean(v);
    (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (v.len().max(2) - 1) as f64).sqrt()
}

/// Wins for A by the margin, a tie half.
fn wins(v: &[f64]) -> f64 {
    v.iter()
        .map(|&x| {
            if x > 0.0 {
                1.0
            } else if x == 0.0 {
                0.5
            } else {
                0.0
            }
        })
        .sum::<f64>()
        / v.len().max(1) as f64
}

/// The paired difference's mean and its standard error.
fn paired(a: &[f64], b: &[f64]) -> (f64, f64) {
    let d: Vec<f64> = a.iter().zip(b).map(|(x, y)| x - y).collect();
    (mean(&d), sd(&d) / (d.len() as f64).sqrt())
}

/// The real favorite's win rate for a spread (the bin it falls in) and the
/// bin's label.
fn real_bin(spread: f64) -> Option<(String, f64, f64)> {
    let v: serde_json::Value = serde_json::from_str(REAL).ok()?;
    let bins = v.get("5_upsets_by_spread")?.get("bins")?.as_object()?;
    for (k, b) in bins {
        // "(3.1, 6.6]"
        let t: Vec<f64> =
            k.trim_matches(|c| "()[] ".contains(c)).split(',').filter_map(|x| x.trim().parse().ok()).collect();
        if t.len() == 2 && spread > t[0] && spread <= t[1] {
            return Some((k.clone(), b.get("favorite wins")?.as_f64()?, b.get("games")?.as_f64()?));
        }
    }
    None
}

fn real_sd() -> Option<f64> {
    let v: serde_json::Value = serde_json::from_str(REAL).ok()?;
    v.get("5_upsets_by_spread")?.get("margin around the line sd")?.as_f64()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let level_name = args.get(1).map(String::as_str).unwrap_or("nfl");
    let level = level_of(level_name);
    let games: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(701);
    let out = args.get(4).cloned();
    let deltas: Vec<f64> = std::env::var("AFM_DUEL_DELTAS")
        .ok()
        .map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect())
        .unwrap_or_else(|| vec![0.0, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0]);
    let cfg = Cfg {
        level,
        games,
        seed,
        sharp: env_f("AFM_DUEL_SHARP", 17.0),
        poor: env_f("AFM_DUEL_POOR", 5.0),
        deltas,
        head: std::env::var("AFM_DUEL_HEAD").is_ok_and(|v| v == "1"),
    };
    let league = synthetic_league(level, 32, seed);
    let t0 = std::time::Instant::now();

    let control = phase(&cfg, &league, Arm::Control);
    let duel = phase(&cfg, &league, Arm::Duel);
    // The same games again, to prove the arm repeats itself.
    let check = games.min(40);
    let again: Vec<f64> = (0..check).map(|g| margin(&cfg, &league, g, 0, Arm::Duel)).collect();
    let repeats = again.iter().zip(&duel[0]).all(|(a, b)| a == b);

    let off_spec = std::env::var("AFM_DUEL_OFF").unwrap_or_default();
    let mut off_arms: Vec<(String, String)> = Vec::new();
    if off_spec == "each" {
        for (k, what) in MECHANISMS {
            off_arms.push((k.to_string(), what.to_string()));
        }
    } else if !off_spec.is_empty() {
        off_arms.push((off_spec.clone(), off_spec.clone()));
    }
    let off: Vec<Vec<Vec<f64>>> = off_arms.iter().map(|(k, _)| phase_off(&cfg, &league, k)).collect();

    let mut s = String::new();
    let lv = level_name.to_uppercase();
    writeln!(s, "## Scheme duel test, {lv}, seed {seed}").unwrap();
    writeln!(
        s,
        "\nSharp caller {:.0} against poor {:.0} (every call skill, both coordinators{}), {games} games a delta in each arm, the sharp team at home on even games. The poor caller's roster is `delta` rating points a player better than the sharp one's. Played in {:.0} s.\n",
        cfg.sharp,
        cfg.poor,
        if cfg.head { " and the head coach's game management" } else { "" },
        t0.elapsed().as_secs_f64()
    )
    .unwrap();
    writeln!(
        s,
        "The same game plays the same every time: {}.\n",
        if repeats {
            format!("yes ({check} games replayed, margins identical)")
        } else {
            "NO, the margins differ".into()
        }
    )
    .unwrap();

    // 1. The edge.
    writeln!(s, "### 1. The coaching edge\n").unwrap();
    writeln!(
        s,
        "Margin is the sharp team's points less the poor team's. Edge is the duel's margin less the average-callers margin on the same games (± one standard error). Real band: 3 to 7 points.\n"
    )
    .unwrap();
    writeln!(
        s,
        "| delta | roster spread (control margin) | duel margin | edge | win % control | win % duel | in 3-7? |"
    )
    .unwrap();
    writeln!(s, "|---|---|---|---|---|---|---|").unwrap();
    let mut edges = Vec::new();
    for d in 0..cfg.deltas.len() {
        let (e, se) = paired(&duel[d], &control[d]);
        edges.push(e);
        let inside = if (3.0..=7.0).contains(&e) {
            "yes"
        } else if e < 3.0 {
            "below"
        } else {
            "above"
        };
        writeln!(
            s,
            "| +{:.1} | {:+.2} | {:+.2} | {:+.2} ± {:.2} | {:.1} | {:.1} | {} |",
            cfg.deltas[d],
            mean(&control[d]),
            mean(&duel[d]),
            e,
            se,
            100.0 * wins(&control[d]),
            100.0 * wins(&duel[d]),
            inside
        )
        .unwrap();
    }
    let gap = cfg.sharp - cfg.poor;
    let all_e = mean(&edges);
    writeln!(
        s,
        "\nMean edge {:+.2} points for a gap of {gap:.0} rating points ({:+.2} a point of gap). Win % is the sharp team's, a tie half.\n",
        all_e,
        all_e / gap
    )
    .unwrap();

    // 2. The upsets.
    writeln!(s, "### 2. Upsets against real\n").unwrap();
    let rsd = real_sd().unwrap_or(12.48);
    let sd0 = sd(&control[0]);
    writeln!(
        s,
        "Margin spread around the roster's own mean with average callers: {:.2} points at delta +0 ({:.2} pooled over every delta). Real NFL margins spread {rsd:.2} points around the line (2024-25). Real figures are the NFL's; the other levels have none, so they read as measures only.\n",
        sd0,
        {
            let mut v = Vec::new();
            for c in &control {
                let m = mean(c);
                v.extend(c.iter().map(|x| x - m));
            }
            sd(&v)
        }
    )
    .unwrap();
    writeln!(s, "The favorite is the better roster (the control margin's side). Win % is the favorite's.\n").unwrap();
    writeln!(s, "| delta | favorite | spread | real bin | real fav win % | control fav win % | duel fav win % |")
        .unwrap();
    writeln!(s, "|---|---|---|---|---|---|---|").unwrap();
    for d in 1..cfg.deltas.len() {
        let m = mean(&control[d]);
        // A's roster is the weaker one at delta > 0: the poor caller's team is the favorite.
        let fav_is_a = m > 0.0;
        let fw = |v: &[f64]| if fav_is_a { 100.0 * wins(v) } else { 100.0 - 100.0 * wins(v) };
        let (bin, real) = match real_bin(m.abs()) {
            Some((b, r, _)) => (b, format!("{:.1}", 100.0 * r)),
            None => ("outside the bins".into(), "-".into()),
        };
        writeln!(
            s,
            "| +{:.1} | {} | {:.2} | {} | {} | {:.1} | {:.1} |",
            cfg.deltas[d],
            if fav_is_a { "sharp" } else { "poor caller's" },
            m.abs(),
            bin,
            real,
            fw(&control[d]),
            fw(&duel[d])
        )
        .unwrap();
    }
    writeln!(
        s,
        "\nReading it: the control column is what the ratings alone give, so it should sit near the real column. The duel column is what a {gap:.0}-point caller gap adds on top; the real column already holds every real coaching gap, a smaller one on average, so the duel's favorite should win a little less than real (the sharp coach beats a better roster sometimes, never always).\n"
    )
    .unwrap();

    // 3. Mechanism switches.
    writeln!(s, "### 3. Each lane's own share\n").unwrap();
    if off_arms.is_empty() {
        writeln!(
            s,
            "No mechanism arm played (set `AFM_DUEL_OFF=each` or a comma list). The known switches, in lane order, each read by its lane from `{OFF_VAR}`:\n"
        )
        .unwrap();
        for (k, what) in MECHANISMS {
            writeln!(s, "- `{k}`: {what}").unwrap();
        }
        writeln!(s).unwrap();
    } else {
        writeln!(
            s,
            "Edge with the mechanism on, less the edge with it off (`{OFF_VAR}` set to the name), over every delta's games. \"Not read\" means every game played exactly as the full duel did, so the engine does not read that name yet.\n"
        )
        .unwrap();
        writeln!(s, "| mechanism | what | edge off | share of the edge | engine reads it |").unwrap();
        writeln!(s, "|---|---|---|---|---|").unwrap();
        for ((k, what), m) in off_arms.iter().zip(&off) {
            let same = m.iter().zip(&duel).all(|(a, b)| a == b);
            let e_off: Vec<f64> = (0..cfg.deltas.len()).map(|d| paired(&m[d], &control[d]).0).collect();
            let eo = mean(&e_off);
            let share = all_e - eo;
            writeln!(s, "| `{k}` | {what} | {:+.2} | {:+.2} | {} |", eo, share, if same { "not read" } else { "yes" })
                .unwrap();
        }
        writeln!(s).unwrap();
    }

    println!("{s}");
    if let Some(p) = out {
        std::fs::write(&p, &s).expect("write the report");
        eprintln!("wrote {p}");
    }
}
