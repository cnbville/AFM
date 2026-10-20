//! Development check for player tendencies (`docs/design/tendencies.md`,
//! "What the build must prove"). Match checks only, no world runs: a league
//! of synthetic teams plays one season (each team 17 games) per arm.
//!
//! 1. The spread is real: rosters drawn at the level's spread, three
//!    seasons, each tied stat's per-player spread against the real NFL
//!    starters' (data/real-world/tendency_spreads.json).
//! 2. Each end moves results: every player who owns a tendency at 4, then
//!    at 16, on the same seed; the tied stat and its cost.
//! 3. Fit matters: fast and slow backs who bounce.
//! 4. Each instruction moves its row, and obedience.
//! 5. No level drifts: league averages with tendencies at the level spread
//!    (compare with `match_check` on the commit before).
//!
//! Usage: tendency_check [nfl|fbs|hs] [games per arm] [seed] [part]
//! part: spread, ends, fit, orders, levels or all (default).

extern crate afm_tools;
use std::collections::HashMap;
use std::time::Instant;

use afm_engine::attributes::Attr;
use afm_engine::game::sim::FoulKind;
use afm_engine::game::stats::{PlayRecord, PlayType};
use afm_engine::game::synthetic::synthetic_league;
use afm_engine::game::{Conditions, GamePlayer, GameResult, GameTeam, Venue, quick_game_in};
use afm_engine::player::Level;
use afm_engine::positions::Position;
use afm_engine::tendencies::{Instruction, Orders, Tend};
use rayon::prelude::*;
use serde_json::Value;

fn pair(g: usize) -> (usize, usize) {
    let k = g / 2;
    let h = (k * 7) % 32;
    let a = (k * 13 + 5) % 32;
    let a = if a == h { (a + 1) % 32 } else { a };
    if g.is_multiple_of(2) { (h, a) } else { (a, h) }
}

fn play(teams: &[GameTeam], level: Level, games: usize, seed: u32) -> Vec<GameResult> {
    (0..games)
        .into_par_iter()
        .map(|g| {
            let (h, a) = pair(g);
            let venue = Venue {
                cond: Conditions { crowd: 0.65, ..Conditions::default() },
                wear: [0.0, 0.012],
                ..Venue::default()
            };
            quick_game_in(&teams[h], &teams[a], level, seed as u64 * 1_000_003 + g as u64, Default::default(), venue)
        })
        .collect()
}

/// What one player did over the season.
#[derive(Default, Clone, Debug)]
struct P {
    pos: Option<Position>,
    games: f64,
    dropbacks: f64,
    scrambles: f64,
    att: f64,
    tight: f64,
    air: f64,
    ttt: f64,
    runs: f64,
    outside: f64,
    oob: f64,
    ttl: f64,
    run_yds: f64,
    comb: f64,
    missed: f64,
    ff: f64,
    tfl: f64,
    tgt: f64,
    ints: f64,
    yds_allowed: f64,
    offside: f64,
    batted: f64,
}

/// League totals over the season.
#[derive(Default, Clone, Debug)]
struct L {
    team_games: f64,
    points: f64,
    runs: f64,
    run_yds: f64,
    stuffs: f64,
    big_runs: f64,
    att: f64,
    pass_yds: f64,
    ints: f64,
    big_cmp: f64,
    cmp: f64,
    dropbacks: f64,
    sacks: f64,
    pressures: f64,
    scrambles: f64,
    scramble_yds: f64,
    draws: f64,
    draw_yds: f64,
    pa_att: f64,
    pa_yds: f64,
    deep_kicks: f64,
    brought_out: f64,
    deep_start: f64,
    punts_caught: f64,
    fair: f64,
    inside10: f64,
    fielded10: f64,
    muffs: f64,
    pr_yds: f64,
    returned: f64,
    roughness: f64,
    offside: f64,
    fumbles: f64,
    reaches: f64,
    reach_got: f64,
    reach_fumbles: f64,
    slides: f64,
    qb_runs: f64,
    qb_hits: f64,
    stepped_out: f64,
    jumps: f64,
    jumps_right: f64,
    rush_wins: f64,
    jump_int: f64,
    jump_cmp: f64,
    jump_wrong_yds: f64,
    batted: f64,
    injuries: f64,
    tackles: f64,
    missed: f64,
    ff: f64,
    tfl: f64,
    yaco: f64,
    rb_runs: f64,
}

fn collect(teams: &[GameTeam], results: &[GameResult]) -> (HashMap<u32, P>, L) {
    let mut pos: HashMap<u32, Position> = HashMap::new();
    for t in teams {
        for p in &t.players {
            pos.insert(p.id, p.position);
        }
    }
    let mut ps: HashMap<u32, P> = HashMap::new();
    let mut l = L::default();
    for r in results {
        l.team_games += 2.0;
        l.points += (r.stats.score[0] + r.stats.score[1]) as f64;
        for side in &r.stats.players {
            for (id, s) in side {
                let p = ps.entry(*id).or_default();
                p.pos = pos.get(id).copied();
                p.games += s.games as f64;
                p.dropbacks += s.dropbacks as f64;
                p.scrambles += s.scrambles as f64;
                p.comb += (s.tackles + s.assists) as f64;
                p.missed += s.missed as f64;
                p.ff += s.forced_fumbles as f64;
                p.tfl += s.tfl;
                p.tgt += s.tgt_allowed as f64;
                p.ints += s.ints as f64;
                p.yds_allowed += s.yds_allowed;
                l.tackles += (s.tackles + s.assists) as f64;
                l.missed += s.missed as f64;
                l.ff += s.forced_fumbles as f64;
                l.tfl += s.tfl;
                l.fumbles += s.fumbles as f64;
                if pos.get(id) == Some(&Position::RB) {
                    l.yaco += s.yaco;
                    l.rb_runs += s.rush as f64;
                }
            }
        }
        for pr in &r.stats.plays {
            one_play(pr, &pos, &mut ps, &mut l);
        }
    }
    (ps, l)
}

fn one_play(r: &PlayRecord, pos: &HashMap<u32, Position>, ps: &mut HashMap<u32, P>, l: &mut L) {
    for &(k, _, id, _, _) in &r.penalties {
        if k == FoulKind::Offside {
            l.offside += 1.0;
            ps.entry(id).or_default().offside += 1.0;
        }
        if k == FoulKind::UnnecessaryRoughness {
            l.roughness += 1.0;
        }
    }
    l.injuries += r.injuries.len() as f64;
    if let Some((_, got, loose)) = r.tend.reach {
        l.reaches += 1.0;
        l.reach_got += got as u8 as f64;
        l.reach_fumbles += loose as u8 as f64;
    }
    if let Some((_, right)) = r.tend.jump {
        l.jumps += 1.0;
        l.jumps_right += right as u8 as f64;
        if right {
            l.jump_int += r.interception as u8 as f64;
            l.jump_cmp += r.complete as u8 as f64;
        } else {
            l.jump_wrong_yds += r.yards;
        }
    }
    if let Some(id) = r.tend.hands_up {
        l.batted += 1.0;
        ps.entry(id).or_default().batted += 1.0;
    }
    l.stepped_out += r.tend.stepped_out as u8 as f64;
    l.rush_wins += r.rush_wins.len() as f64;
    match r.kind {
        PlayType::Run => {
            l.runs += 1.0;
            l.run_yds += r.yards;
            l.stuffs += (r.yards <= 0.0) as u8 as f64;
            l.big_runs += (r.yards >= 20.0) as u8 as f64;
            if r.call.contains("draw") {
                l.draws += 1.0;
                l.draw_yds += r.yards;
            }
            if let (Some(c), Some((dy, t))) = (r.carrier, r.tend.cross)
                && pos.get(&c) == Some(&Position::RB)
            {
                let p = ps.entry(c).or_default();
                p.runs += 1.0;
                p.outside += (dy.abs() > OUTSIDE) as u8 as f64;
                p.oob += r.out_of_bounds as u8 as f64;
                p.ttl += t;
                p.run_yds += r.yards;
            }
        }
        PlayType::Pass => {
            l.att += 1.0;
            l.pass_yds += r.yards;
            l.ints += r.interception as u8 as f64;
            l.cmp += r.complete as u8 as f64;
            l.big_cmp += (r.complete && r.yards >= 20.0) as u8 as f64;
            l.dropbacks += 1.0;
            l.pressures += r.pressured as u8 as f64;
            l.qb_hits += r.hits.len() as f64;
            if r.play_action {
                l.pa_att += 1.0;
                l.pa_yds += r.yards;
            }
            if let Some(q) = r.passer
                && !r.throw_away
            {
                let p = ps.entry(q).or_default();
                p.att += 1.0;
                p.tight += (r.target.is_some() && r.separation < 1.0) as u8 as f64;
                p.air += r.air_yards;
                p.ttt += r.time_to_throw;
            }
        }
        PlayType::Sack => {
            l.dropbacks += 1.0;
            l.sacks += 1.0;
            l.pressures += 1.0;
        }
        PlayType::Scramble => {
            l.dropbacks += 1.0;
            l.scrambles += 1.0;
            l.scramble_yds += r.yards;
            l.qb_runs += 1.0;
            l.slides += r.tend.slid as u8 as f64;
            l.qb_hits += (!r.tend.slid && !r.out_of_bounds) as u8 as f64;
        }
        PlayType::Kickoff => {
            if let Some(out) = r.tend.deep_kick {
                l.deep_kicks += 1.0;
                l.brought_out += out as u8 as f64;
                l.deep_start += r.tend.start.unwrap_or(0.0);
            }
        }
        PlayType::Punt => {
            if let Some(f) = r.tend.fielded_inside_10 {
                l.inside10 += 1.0;
                l.fielded10 += f as u8 as f64;
            }
            l.muffs += r.tend.muffed as u8 as f64;
            if r.fair_catch {
                l.fair += 1.0;
                l.punts_caught += 1.0;
            } else if r.returner.is_some() && !r.tend.muffed {
                l.punts_caught += 1.0;
                l.returned += 1.0;
                l.pr_yds += r.return_yards;
            }
        }
        _ => {}
    }
}

/// Yards across the field from the snap beyond which a run crossed the line
/// outside the tackles (the real "end" gap: past the tackle, about four
/// yards from the ball).
const OUTSIDE: f64 = 4.0;

fn pct(v: &mut [f64], q: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let k = ((v.len() - 1) as f64 * q / 100.0).round() as usize;
    v[k]
}

fn owners(t: Tend) -> Vec<Position> {
    Position::ALL.into_iter().filter(|&p| t.owned_by(p)).collect()
}

fn with(base: &[GameTeam], f: &(dyn Fn(&mut GamePlayer) + Sync)) -> Vec<GameTeam> {
    let mut t = base.to_vec();
    for team in t.iter_mut() {
        for p in team.players.iter_mut() {
            f(p);
        }
    }
    t
}

fn set_tend(t: Tend, v: f64) -> impl Fn(&mut GamePlayer) + Sync {
    let own = owners(t);
    move |p: &mut GamePlayer| {
        if own.contains(&p.position) {
            p.tend.set(t, v);
        }
    }
}

struct Arm {
    l: L,
    ps: HashMap<u32, P>,
    secs: f64,
}

fn run(base: &[GameTeam], f: &(dyn Fn(&mut GamePlayer) + Sync), level: Level, games: usize, seed: u32) -> Arm {
    let teams = with(base, f);
    let t0 = Instant::now();
    let res = play(&teams, level, games, seed);
    let secs = t0.elapsed().as_secs_f64();
    let (ps, l) = collect(&teams, &res);
    Arm { l, ps, secs }
}

fn ratio(a: f64, b: f64) -> f64 {
    if b > 0.0 { a / b } else { f64::NAN }
}

/// Group stats over the players of some positions.
fn group(arm: &Arm, pos: &[Position], f: impl Fn(&P) -> (f64, f64)) -> f64 {
    let (mut n, mut d) = (0.0, 0.0);
    for p in arm.ps.values().filter(|p| p.pos.is_some_and(|x| pos.contains(&x))) {
        let (a, b) = f(p);
        n += a;
        d += b;
    }
    ratio(n, d)
}

/// The tied stat and its cost for a tendency, as printed numbers.
fn measures(t: Tend, a: &Arm) -> Vec<(&'static str, f64, &'static str)> {
    use Position as Ps;
    let l = &a.l;
    let tg = l.team_games.max(1.0);
    match t {
        Tend::Bounce => vec![
            ("carries outside the tackles", group(a, &[Ps::RB], |p| (p.outside, p.runs)), "%"),
            ("RB yards a designed carry", group(a, &[Ps::RB], |p| (p.run_yds, p.runs)), ""),
            ("stuffed (no gain or loss)", ratio(l.stuffs, l.runs), "%"),
        ],
        Tend::Contact => vec![
            ("RB carries ending out of bounds", group(a, &[Ps::RB], |p| (p.oob, p.runs)), "%"),
            ("QB scrambles ending in a slide", ratio(l.slides, l.qb_runs), "%"),
            ("RB yards after contact a carry", ratio(l.yaco, l.rb_runs), ""),
            ("injuries a team game", l.injuries / tg, ""),
        ],
        Tend::Reach => vec![
            ("reaches a team game", l.reaches / tg, ""),
            ("reaches that got there", ratio(l.reach_got, l.reaches), "%"),
            ("fumbles on a reach a team game", l.reach_fumbles / tg, ""),
        ],
        Tend::Patience => vec![
            ("RB time to the line, s", group(a, &[Ps::RB], |p| (p.ttl, p.runs)), ""),
            ("RB yards a designed carry", group(a, &[Ps::RB], |p| (p.run_yds, p.runs)), ""),
            ("stuffed", ratio(l.stuffs, l.runs), "%"),
            ("runs of 20+", ratio(l.big_runs, l.runs), "%"),
        ],
        Tend::Scramble => vec![
            ("QB scramble rate", group(a, &[Ps::QB], |p| (p.scrambles, p.dropbacks)), "%"),
            ("scramble yards a team game", l.scramble_yds / tg, ""),
            ("sacks a dropback", ratio(l.sacks, l.dropbacks), "%"),
        ],
        Tend::Gunslinger => vec![
            ("tight-window throws (under 1 yd)", group(a, &[Ps::QB], |p| (p.tight, p.att)), "%"),
            ("intended air yards", group(a, &[Ps::QB], |p| (p.air, p.att)), ""),
            ("interceptions an attempt", ratio(l.ints, l.att), "%"),
            ("yards an attempt", ratio(l.pass_yds, l.att), ""),
        ],
        Tend::Hold => vec![
            ("time to throw, s", group(a, &[Ps::QB], |p| (p.ttt, p.att)), ""),
            ("completions of 20+ an attempt", ratio(l.big_cmp, l.att), "%"),
            ("sacks a dropback", ratio(l.sacks, l.dropbacks), "%"),
        ],
        Tend::ReturnRisk => vec![
            ("end-zone kicks brought out", ratio(l.brought_out, l.deep_kicks), "%"),
            ("start after an end-zone kick", ratio(l.deep_start, l.deep_kicks), ""),
            ("punt fair-catch rate", ratio(l.fair, l.punts_caught), "%"),
            ("punts inside the 10 fielded", ratio(l.fielded10, l.inside10), "%"),
            ("muffs a team game", l.muffs / tg, ""),
        ],
        Tend::Upfield => vec![
            ("scramble yards allowed a team game", l.scramble_yds / tg, ""),
            ("sacks a dropback", ratio(l.sacks, l.dropbacks), "%"),
            ("pressures a dropback", ratio(l.pressures, l.dropbacks), "%"),
            ("rush wins a dropback", ratio(l.rush_wins, l.dropbacks), ""),
            ("draw yards a carry", ratio(l.draw_yds, l.draws), ""),
        ],
        Tend::SnapTiming => vec![
            ("DL offside flags per 17 games", 17.0 * group(a, &[Ps::EDGE, Ps::DT], |p| (p.offside, p.games)), ""),
            ("sacks a dropback", ratio(l.sacks, l.dropbacks), "%"),
            ("pressures a dropback", ratio(l.pressures, l.dropbacks), "%"),
        ],
        Tend::HandsUp => vec![
            ("batted passes per DL per 17 games", 17.0 * group(a, &[Ps::EDGE, Ps::DT], |p| (p.batted, p.games)), ""),
            ("sacks a dropback", ratio(l.sacks, l.dropbacks), "%"),
        ],
        Tend::Strip => vec![
            ("forced fumbles per 100 tackles", 100.0 * ratio(l.ff, l.tackles), ""),
            ("missed tackle rate", ratio(l.missed, l.missed + l.tackles), "%"),
            ("RB yards after contact a carry", ratio(l.yaco, l.rb_runs), ""),
        ],
        Tend::BigHit => vec![
            ("missed tackle rate", ratio(l.missed, l.missed + l.tackles), "%"),
            ("forced fumbles per 100 tackles", 100.0 * ratio(l.ff, l.tackles), ""),
            ("roughness flags a team game", l.roughness / tg, ""),
            ("injuries a team game", l.injuries / tg, ""),
        ],
        Tend::Downhill => vec![
            ("LB and S tackles for loss per 17 games", 17.0 * group(a, &[Ps::LB, Ps::S], |p| (p.tfl, p.games)), ""),
            ("play-action yards an attempt", ratio(l.pa_yds, l.pa_att), ""),
            ("yards a run", ratio(l.run_yds, l.runs), ""),
        ],
        Tend::JumpRoutes => vec![
            ("CB and S interceptions a target", group(a, &[Ps::CB, Ps::S], |p| (p.ints, p.tgt)), "%"),
            ("completions of 20+ an attempt", ratio(l.big_cmp, l.att), "%"),
            ("yards an attempt", ratio(l.pass_yds, l.att), ""),
            ("route jumps a team game", ratio(l.jumps, l.team_games), ""),
            ("jumps guessed right", ratio(l.jumps_right, l.jumps), "%"),
            ("picked off when he guessed right", ratio(l.jump_int, l.jumps_right), "%"),
            ("caught when he guessed right", ratio(l.jump_cmp, l.jumps_right), "%"),
            ("yards a throw when he guessed wrong", ratio(l.jump_wrong_yds, l.jumps - l.jumps_right), ""),
        ],
    }
}

fn show(v: f64, unit: &str) -> String {
    if unit == "%" { format!("{:.1}%", 100.0 * v) } else { format!("{v:.2}") }
}

fn real(spreads: &Value, key: &str) -> String {
    let s = &spreads["stats"][key];
    if s.is_null() {
        return "no public stat".into();
    }
    let f = |k: &str| s[k].as_f64().unwrap_or(f64::NAN);
    format!("real p5 {} / p50 {} / p95 {}", f("p5"), f("p50"), f("p95"))
}

fn real_key(t: Tend) -> &'static str {
    match t {
        Tend::Bounce => "outside_share",
        Tend::Contact => "run_oob_share",
        Tend::Reach => "",
        Tend::Patience => "time_to_los",
        Tend::Scramble => "scramble_rate",
        Tend::Gunslinger => "tight_window_pct",
        Tend::Hold => "time_to_throw",
        Tend::ReturnRisk => "endzone_return_share",
        Tend::Upfield => "",
        Tend::SnapTiming => "snap_flags_per_17",
        Tend::HandsUp => "batted_per_17",
        Tend::Strip => "forced_fumbles_per_100_tackles",
        Tend::BigHit => "missed_tackle_pct",
        Tend::Downhill => "tfl_per_17",
        Tend::JumpRoutes => "int_per_target",
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let level = match args.get(1).map(|s| s.as_str()) {
        Some("fbs") => Level::FBS,
        Some("fcs") => Level::FCS,
        Some("small") => Level::SmallCollege,
        Some("hs") => Level::HS,
        _ => Level::NFL,
    };
    let games: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(272);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
    let part = args.get(4).cloned().unwrap_or_else(|| "all".into());
    let only = args.get(5).cloned();
    let spreads: Value =
        serde_json::from_str(include_str!("../../../../data/real-world/tendency_spreads.json")).unwrap();
    let base = synthetic_league(level, 32, seed);
    let all = Instant::now();
    let keep = |_: &mut GamePlayer| {};

    if part == "all" || part == "spread" {
        println!("1. THE SPREAD IS REAL: rosters at the level's spread, per player season (3 seasons)");
        let t0 = Instant::now();
        let mut ps: Vec<P> = Vec::new();
        let mut teams_l: Vec<L> = Vec::new();
        for k in 0..3u32 {
            let b = synthetic_league(level, 32, seed + 100 * k);
            let teams = b.clone();
            let res = play(&teams, level, games, seed + 100 * k);
            let (p, _) = collect(&teams, &res);
            ps.extend(p.into_values());
            // Per team season, for the kick stats (the play log names no
            // returner on a touchback, as in the real data).
            for (ti, _) in teams.iter().enumerate() {
                let mine: Vec<GameResult> = res
                    .iter()
                    .enumerate()
                    .filter(|(g, _)| {
                        let (h, a) = pair(*g);
                        h == ti || a == ti
                    })
                    .map(|x| x.1.clone())
                    .collect();
                let mut l = L::default();
                let pos: HashMap<u32, Position> = HashMap::new();
                let mut scratch = HashMap::new();
                for (g, r) in res.iter().enumerate().filter(|(g, _)| {
                    let (h, a) = pair(*g);
                    h == ti || a == ti
                }) {
                    let (h, _) = pair(g);
                    let me = if h == ti { 0 } else { 1 };
                    for pr in &r.stats.plays {
                        // Kickoffs and punts this team received.
                        let received = match pr.kind {
                            PlayType::Kickoff => pr.offense != me,
                            PlayType::Punt => pr.offense != me,
                            _ => false,
                        };
                        if received {
                            one_play(pr, &pos, &mut scratch, &mut l);
                        }
                    }
                }
                let _ = mine;
                teams_l.push(l);
            }
        }
        let line = |name: &str, key: &str, mut v: Vec<f64>, scale: f64| {
            let n = v.len();
            let (a, b, c) = (pct(&mut v, 5.0) * scale, pct(&mut v, 50.0) * scale, pct(&mut v, 95.0) * scale);
            println!("  {name:<38} engine p5 {a:.3} / p50 {b:.3} / p95 {c:.3} (n {n}); {}", real(&spreads, key));
        };
        let by = |pos: &[Position], ok: &dyn Fn(&P) -> bool, f: &dyn Fn(&P) -> f64| -> Vec<f64> {
            ps.iter().filter(|p| p.pos.is_some_and(|x| pos.contains(&x)) && ok(p)).map(f).collect()
        };
        use Position as Ps;
        line(
            "scramble rate (QB, 250+ dropbacks)",
            "scramble_rate",
            by(&[Ps::QB], &|p| p.dropbacks >= 250.0, &|p| p.scrambles / p.dropbacks),
            1.0,
        );
        line(
            "tight-window share % (QB, 200+ att)",
            "tight_window_pct",
            by(&[Ps::QB], &|p| p.att >= 200.0, &|p| p.tight / p.att),
            100.0,
        );
        line(
            "intended air yards (QB)",
            "intended_air_yards",
            by(&[Ps::QB], &|p| p.att >= 200.0, &|p| p.air / p.att),
            1.0,
        );
        line("time to throw (QB)", "time_to_throw", by(&[Ps::QB], &|p| p.att >= 200.0, &|p| p.ttt / p.att), 1.0);
        line(
            "time to the line (RB, 100+ carries)",
            "time_to_los",
            by(&[Ps::RB], &|p| p.runs >= 100.0, &|p| p.ttl / p.runs),
            1.0,
        );
        line("outside share (RB)", "outside_share", by(&[Ps::RB], &|p| p.runs >= 100.0, &|p| p.outside / p.runs), 1.0);
        line(
            "out of bounds share (RB)",
            "run_oob_share",
            by(&[Ps::RB], &|p| p.runs >= 100.0, &|p| p.oob / p.runs),
            1.0,
        );
        let front = [Ps::EDGE, Ps::DT];
        line(
            "offside flags per 17 (DL, 10+ games)",
            "snap_flags_per_17",
            by(&front, &|p| p.games >= 10.0, &|p| 17.0 * p.offside / p.games),
            1.0,
        );
        line(
            "batted per 17 (DL, 10+ games)",
            "batted_per_17",
            by(&front, &|p| p.games >= 10.0, &|p| 17.0 * p.batted / p.games),
            1.0,
        );
        line(
            "forced fumbles per 100 tackles (40+)",
            "forced_fumbles_per_100_tackles",
            by(&Position::ALL, &|p| p.comb >= 40.0, &|p| 100.0 * p.ff / p.comb),
            1.0,
        );
        line(
            "missed tackle rate (40+ tackles)",
            "missed_tackle_pct",
            by(&Position::ALL, &|p| p.comb >= 40.0, &|p| p.missed / (p.missed + p.comb)),
            1.0,
        );
        line(
            "TFL per 17 (LB and S, 40+ tackles)",
            "tfl_per_17",
            by(&[Ps::LB, Ps::S], &|p| p.comb >= 40.0, &|p| 17.0 * p.tfl / p.games.max(1.0)),
            1.0,
        );
        line(
            "INT a target (CB and S, 40+ targets)",
            "int_per_target",
            by(&[Ps::CB, Ps::S], &|p| p.tgt >= 40.0, &|p| p.ints / p.tgt),
            1.0,
        );
        line(
            "yards a target allowed (CB and S)",
            "yards_per_target_allowed",
            by(&[Ps::CB, Ps::S], &|p| p.tgt >= 40.0, &|p| p.yds_allowed / p.tgt),
            1.0,
        );
        line(
            "end-zone kicks brought out (team)",
            "endzone_return_share",
            teams_l.iter().filter(|l| l.deep_kicks >= 10.0).map(|l| l.brought_out / l.deep_kicks).collect(),
            1.0,
        );
        line(
            "punt fair-catch rate (team)",
            "fair_catch_rate",
            teams_l.iter().filter(|l| l.punts_caught >= 15.0).map(|l| l.fair / l.punts_caught).collect(),
            1.0,
        );
        println!("  ({:.0} s for {} games)", t0.elapsed().as_secs_f64(), 3 * games);
    }

    if part == "all" || part == "ends" {
        println!("\n2. EACH END MOVES RESULTS: every player who owns it at 4, then at 16 (same seed)");
        for t in Tend::ALL {
            if only.as_deref().is_some_and(|o| o != t.key()) {
                continue;
            }
            let lo = run(&base, &set_tend(t, 4.0), level, games, seed);
            let hi = run(&base, &set_tend(t, 16.0), level, games, seed);
            println!("  {} ({:?}); {} [{:.0} s]", t.label(), owners(t), real(&spreads, real_key(t)), lo.secs + hi.secs);
            for ((name, a, u), (_, b, _)) in measures(t, &lo).into_iter().zip(measures(t, &hi)) {
                println!("      {name:<40} at 4: {:>8}   at 16: {:>8}", show(a, u), show(b, u));
            }
        }
    }

    if part == "all" || part == "fit" {
        println!("\n3. FIT MATTERS: backs who bounce at 16 or 4, made fast or slow (speed, burst, elusiveness +-12)");
        let t0 = Instant::now();
        for (name, add) in [("fast", 12.0), ("slow", -12.0)] {
            for b in [4.0, 16.0] {
                let f = move |p: &mut GamePlayer| {
                    if p.position == Position::RB {
                        p.tend.set(Tend::Bounce, b);
                        for a in [Attr::Speed, Attr::Acceleration, Attr::Elusiveness] {
                            p.attrs.set(a, (p.attrs.get(a) + add).clamp(1.0, 99.0));
                        }
                    }
                };
                let arm = run(&base, &f, level, games, seed);
                let ypc = group(&arm, &[Position::RB], |p| (p.run_yds, p.runs));
                let out = group(&arm, &[Position::RB], |p| (p.outside, p.runs));
                println!("  {name} backs, bounce {b:>2}: {ypc:.2} yards a designed carry, {:.1}% outside", 100.0 * out);
            }
        }
        println!("  ({:.0} s)", t0.elapsed().as_secs_f64());
    }

    if part == "all" || part == "orders" {
        println!("\n4. EACH INSTRUCTION MOVES ITS ROW: the habit at 16 (or 18), without and with the order");
        let rows: Vec<(Instruction, Tend, f64)> = vec![
            (Instruction::KneelDeeperThan(3.0), Tend::ReturnRisk, 17.0),
            (Instruction::FairCatchInsideTen, Tend::ReturnRisk, 17.0),
            (Instruction::Slide, Tend::Contact, 17.0),
            (Instruction::TakeShots, Tend::Gunslinger, 4.0),
            (Instruction::ProtectBall, Tend::Gunslinger, 17.0),
            (Instruction::HighAndTight, Tend::Reach, 17.0),
            (Instruction::KeepContain, Tend::Upfield, 17.0),
            (Instruction::NoSnapGuessing, Tend::SnapTiming, 17.0),
            (Instruction::WrapUp, Tend::BigHit, 17.0),
            (Instruction::StayHome, Tend::JumpRoutes, 17.0),
        ];
        for (ins, t, v) in rows {
            let own = owners(t);
            let qb_too = matches!(ins, Instruction::Slide);
            let give = move |p: &mut GamePlayer, order: bool, disc: Option<f64>| {
                let mine = own.contains(&p.position) && (!qb_too || p.position == Position::QB);
                if mine {
                    p.tend.set(t, v);
                    if matches!(ins, Instruction::TakeShots) {
                        p.tend.set(Tend::Hold, 4.0);
                    }
                    if matches!(ins, Instruction::ProtectBall) {
                        p.tend.set(Tend::Hold, 16.0);
                    }
                    if matches!(ins, Instruction::WrapUp) {
                        p.tend.set(Tend::Strip, 16.0);
                    }
                    if order {
                        p.orders = Orders(vec![ins]);
                    }
                    if let Some(d) = disc {
                        p.attrs.set(Attr::Discipline, d);
                    }
                }
            };
            let free = run(&base, &|p: &mut GamePlayer| give(p, false, None), level, games, seed);
            let told = run(&base, &|p: &mut GamePlayer| give(p, true, None), level, games, seed);
            // A low-discipline group: about two times in three.
            let norm = afm_engine::game::rules::trait_norm(level, Attr::Discipline);
            let low = norm - 2.6 * 7.0;
            let loose = run(&base, &|p: &mut GamePlayer| give(p, true, Some(low)), level, games, seed);
            let follow = |d: f64| afm_engine::tendencies::obey_chance(10.0 + (d - norm) / 2.6, 10.0, 0.5, 10.0);
            println!(
                "  {} ({:?} at {v}); followed {:.0}% by a typical player, {:.0}% at discipline 3 of 20 [{:.0} s]",
                ins.label(),
                t,
                100.0 * follow(norm),
                100.0 * follow(low),
                free.secs + told.secs + loose.secs
            );
            for (((name, a, u), (_, b, _)), (_, c, _)) in
                measures(t, &free).into_iter().zip(measures(t, &told)).zip(measures(t, &loose))
            {
                println!(
                    "      {name:<40} free: {:>8}  ordered: {:>8}  low discipline: {:>8}",
                    show(a, u),
                    show(b, u),
                    show(c, u)
                );
            }
        }
    }

    if part == "all" || part == "levels" {
        println!("\n5. NO LEVEL DRIFTS: league averages, every tendency at the middle, then at the level spread");
        let mid = |p: &mut GamePlayer| {
            for t in Tend::ALL {
                p.tend.set(t, afm_engine::tendencies::MID);
            }
        };
        for (name, a) in
            [("middle", run(&base, &mid, level, games, seed)), ("spread", run(&base, &keep, level, games, seed))]
        {
            let l = &a.l;
            let tg = l.team_games;
            println!(
                "  {name}: points a team game {:.1}; yards a run {:.2}; yards an attempt {:.2}; completion {:.1}%; INT an attempt {:.2}%; sacks a dropback {:.1}%; scrambles a dropback {:.1}%; fumbles a team game {:.2}; offside a team game {:.2} [{:.0} s]",
                l.points / tg,
                l.run_yds / l.runs,
                l.pass_yds / l.att,
                100.0 * l.cmp / l.att,
                100.0 * l.ints / l.att,
                100.0 * l.sacks / l.dropbacks,
                100.0 * l.scrambles / l.dropbacks,
                l.fumbles / tg,
                l.offside / tg,
                a.secs
            );
        }
    }
    if part == "centers" {
        // The mean of each tendency among the players who own it, as drawn.
        let owned: Vec<String> = Tend::ALL
            .into_iter()
            .map(|t| {
                let v: Vec<f64> = base
                    .iter()
                    .flat_map(|team| team.players.iter())
                    .filter(|p| t.owned_by(p.position))
                    .map(|p| p.tend.get(t))
                    .collect();
                format!("{:.2}", v.iter().sum::<f64>() / v.len() as f64 - afm_engine::tendencies::MID)
            })
            .collect();
        println!("centers {:?}: [{}]", level, owned.join(", "));
    }
    if part == "drift" {
        // Which tendency moves the league average: every one at the middle
        // but this one, which keeps the level spread.
        println!("\nDRIFT BY TENDENCY: all at the middle but one (at the level spread), against all at the middle");
        let mid = |p: &mut GamePlayer| {
            for t in Tend::ALL {
                p.tend.set(t, afm_engine::tendencies::MID);
            }
        };
        let row = |l: &L| {
            let tg = l.team_games;
            [
                l.points / tg,
                l.run_yds / l.runs,
                l.pass_yds / l.att,
                100.0 * l.ints / l.att,
                100.0 * l.sacks / l.dropbacks,
                100.0 * l.scrambles / l.dropbacks,
                l.fumbles / tg,
                l.offside / tg,
            ]
        };
        let m = row(&run(&base, &mid, level, games, seed).l);
        println!(
            "  middle: points {:.2} ypc {:.2} ypa {:.2} int% {:.2} sack% {:.2} scr% {:.2} fum {:.2} offside {:.2}",
            m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7]
        );
        let keepers: HashMap<u32, afm_engine::tendencies::Tendencies> =
            base.iter().flat_map(|t| t.players.iter().map(|p| (p.id, p.tend))).collect();
        // A comma list of keys runs one arm with those at the spread.
        let groups: Vec<Vec<Tend>> = match &only {
            Some(o) => vec![Tend::ALL.into_iter().filter(|t| o.split(',').any(|k| k == t.key())).collect()],
            None => Tend::ALL.into_iter().map(|t| vec![t]).collect(),
        };
        for g in groups {
            let f = |p: &mut GamePlayer| {
                let vs: Vec<f64> = g.iter().map(|&t| keepers[&p.id].get(t)).collect();
                for u in Tend::ALL {
                    p.tend.set(u, afm_engine::tendencies::MID);
                }
                for (&t, v) in g.iter().zip(vs) {
                    p.tend.set(t, v);
                }
            };
            let r = row(&run(&base, &f, level, games, seed).l);
            let t = g.iter().map(|t| t.key()).collect::<Vec<_>>().join("+");
            println!(
                "  {:<22} points {:+.2} ypc {:+.2} ypa {:+.2} int% {:+.2} sack% {:+.2} scr% {:+.2} fum {:+.2} offside {:+.2}",
                t,
                r[0] - m[0],
                r[1] - m[1],
                r[2] - m[2],
                r[3] - m[3],
                r[4] - m[4],
                r[5] - m[5],
                r[6] - m[6],
                r[7] - m[7]
            );
        }
    }
    println!("\nall parts: {:.0} s on {} threads", all.elapsed().as_secs_f64(), rayon::current_num_threads());
}
