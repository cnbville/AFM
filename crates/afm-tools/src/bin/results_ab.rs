//! The A/B test for growth from results (items 14, 15 and 17 of
//! `afm/game-reps/proposal.md`): synthetic players from 14 to 30 through
//! their seasons in each arm, the same men on the same seeds. Each season
//! draws his results from the fitted odds (`results::table::FIT`: what his
//! ratings give on average, the trials his snaps bring, a trial's spread,
//! the season's teammates and scheme), runs `develop_season` with the year
//! held back and the results step after every game, as the world does.
//! Reports growth by role and level, how a season's stat carries into the
//! next against real, the stat spread against real, rating levels by stage
//! against arm A, breakouts and slumps, and the share pulled down.
//! `cargo run --release --bin results_ab -- 20000 [seed] [out.md]`

use std::collections::HashMap;
use std::fmt::Write as _;

use afm_engine::development::{SeasonContext, develop_season, season_games};
use afm_engine::player::{DevelopmentEvent as Ev, Level, Player, PlayingTime, ProspectOptions, generate_prospect};
use afm_engine::positions::Position;
use afm_engine::results::{self, Dice, Ledger, Man, Mode, Read};
use afm_engine::rng::Rng;
use afm_engine::training::{FAMILIES, close_year, cover, land};
use rayon::prelude::*;

/// The arms: A today, B-E items 14 and 15, then per-snap rates.
fn arms() -> Vec<(&'static str, Option<Mode>)> {
    let m =
        |dice, pull_down, scale| Some(Mode { on: true, dice, pull_down, practice: false, scale, ..Mode::default() });
    vec![
        ("A today", None),
        ("B on top, pull down", m(Dice::Kept, true, 1.0)),
        ("C replace, pull down", m(Dice::Replaced, true, 1.0)),
        ("D replace, up only", m(Dice::Replaced, false, 1.0)),
        ("E on top, up only", m(Dice::Kept, false, 1.0)),
        ("B at half rate", m(Dice::Kept, true, 0.5)),
        ("C at half rate", m(Dice::Replaced, true, 0.5)),
    ]
}

#[derive(Clone)]
struct Career {
    p: Player,
    /// 0 high school, 1 college, 2 NFL, 3 out of football.
    stage: u8,
    top: bool,
    last: Option<(Level, f64)>,
    redshirted: bool,
    /// Each read's teammates-and-scheme draw last season (standard units)
    /// and the stage it was drawn at.
    context: Vec<(u8, f64)>,
    context_stage: u8,
    /// His results ledger, rolled each season (his past kept).
    ledger: Ledger,
}

/// How much of a season's teammates and scheme carries into the next at
/// the same stage (rosters turn over about a quarter a year; proposed,
/// not confirmed).
const CONTEXT_CARRY: f64 = 0.6;

/// One season of one man.
#[derive(Clone, Default)]
struct Row {
    id: u32,
    age: u32,
    level: u8,
    role: u8,
    qb: bool,
    top: bool,
    start: f32,
    end: f32,
    results: f32,
    best: f32,
    best_read: u8,
    /// The skill results moved most this season (rating points, signed).
    skill: f32,
    /// Games where one skill moved 1, 1.5 and 2 points or more up.
    big: [u16; 3],
    games: u16,
    events: u32,
    /// Per read: trials and the summed result (the stat).
    stats: Vec<(u8, f32, f32)>,
}

const EVS: [Ev; 7] =
    [Ev::Breakout, Ev::Slump, Ev::Stagnation, Ev::LostSeason, Ev::BigGame, Ev::BigSeason, Ev::RoughSeason];

fn level_of(stage: u8) -> Level {
    match stage {
        0 => Level::HS,
        1 => Level::FBS,
        _ => Level::NFL,
    }
}

/// Role shares (starter, rotation, backup; the rest redshirt) by stage and age.
fn shares(stage: u8, age: u32) -> [f64; 3] {
    match (stage, age) {
        (0, 14) => [0.10, 0.30, 0.60],
        (0, 15) => [0.25, 0.35, 0.40],
        (0, 16) => [0.45, 0.30, 0.25],
        (0, _) => [0.60, 0.25, 0.15],
        (1, 18) => [0.12, 0.20, 0.33],
        (1, 19) => [0.35, 0.30, 0.35],
        (1, 20) => [0.50, 0.30, 0.20],
        (1, _) => [0.65, 0.20, 0.15],
        (_, 22) => [0.30, 0.30, 0.40],
        (_, 23) => [0.45, 0.30, 0.25],
        _ => [0.55, 0.25, 0.20],
    }
}

fn poisson(rng: &mut Rng, l: f64) -> u32 {
    if l <= 0.0 {
        return 0;
    }
    if l > 30.0 {
        return rng.normal(l, l.sqrt()).round().max(0.0) as u32;
    }
    let (lim, mut k, mut p) = ((-l).exp(), 0u32, 1.0);
    loop {
        p *= rng.next();
        if p <= lim {
            return k;
        }
        k += 1;
    }
}

fn key(parts: &[u64]) -> u32 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &x in parts {
        h ^= x;
        h = h.wrapping_mul(0x100_0000_01b3);
        h ^= h >> 29;
    }
    (h ^ (h >> 32)) as u32
}

/// Ranks the men still playing at each stage and gives each a role.
fn roles(cs: &[Career], age: u32) -> Vec<Option<PlayingTime>> {
    let mut out = vec![None; cs.len()];
    let mut groups: HashMap<(u8, bool, Position), Vec<usize>> = HashMap::new();
    for (i, c) in cs.iter().enumerate() {
        if c.stage < 3 {
            groups.entry((c.stage, c.top, c.p.position)).or_default().push(i);
        }
    }
    for ((stage, _, _), mut ix) in groups {
        ix.sort_by(|&a, &b| cs[b].p.overall().total_cmp(&cs[a].p.overall()).then(a.cmp(&b)));
        let s = shares(stage, age);
        let n = ix.len() as f64;
        for (k, &i) in ix.iter().enumerate() {
            let q = k as f64 / n;
            out[i] = Some(if q < s[0] {
                PlayingTime::Starter
            } else if q < s[0] + s[1] {
                PlayingTime::Rotation
            } else if q < s[0] + s[1] + s[2] || stage != 1 || age != 18 || cs[i].redshirted {
                PlayingTime::Backup
            } else {
                PlayingTime::Redshirt
            });
        }
    }
    out
}

/// Moves men between stages at 18 (college: the top 55%, the top tenth
/// of them at top programs) and 22 (the NFL: the top quarter), and cuts
/// the NFL's bottom eighth each year from 24.
fn advance(cs: &mut [Career], age: u32) {
    let mut by: HashMap<Position, Vec<usize>> = HashMap::new();
    let from = match age {
        18 => 0,
        22 => 1,
        a if a >= 24 => 2,
        _ => return,
    };
    for (i, c) in cs.iter().enumerate() {
        if c.stage == from {
            by.entry(c.p.position).or_default().push(i);
        }
    }
    for (_, mut ix) in by {
        ix.sort_by(|&a, &b| cs[b].p.overall().total_cmp(&cs[a].p.overall()).then(a.cmp(&b)));
        let n = ix.len() as f64;
        for (k, &i) in ix.iter().enumerate() {
            let q = k as f64 / n;
            let c = &mut cs[i];
            match from {
                0 => {
                    c.stage = if q < 0.55 { 1 } else { 3 };
                    c.top = q < 0.055;
                }
                1 => {
                    c.stage = if q < 0.25 { 2 } else { 3 };
                    c.top = false;
                }
                _ => {
                    if q >= 0.875 {
                        c.stage = 3;
                    }
                }
            }
        }
    }
}

/// One season of one man in one arm.
fn season(c: &mut Career, role: PlayingTime, age: u32, seed: u32, mode: Option<Mode>) -> Row {
    let level = level_of(c.stage);
    let p = &mut c.p;
    let id = p.id;
    let mut ctx = SeasonContext::new(
        level,
        match (c.stage, c.top) {
            (0, _) => 9.0,
            (1, true) => 16.0,
            (1, false) => 13.0,
            _ => 14.0,
        },
        role,
    );
    ctx.snaps = c.last.filter(|l| l.0 == level).map(|l| l.1);
    ctx.defer = true;
    ctx.results = mode;
    let start = p.overall();
    let mut rng = Rng::new(key(&[seed as u64, id as u64, age as u64, 1]));
    develop_season(p, &ctx, &mut rng);
    let g = season_games(level);
    let mut draw = Rng::new(key(&[seed as u64, id as u64, age as u64, 2]));
    let share = match role {
        PlayingTime::Starter => draw.range(0.75, 1.0),
        PlayingTime::Rotation => draw.range(0.3, 0.6),
        PlayingTime::Backup => draw.range(0.03, 0.15),
        _ => 0.0,
    };
    let games = if role == PlayingTime::Redshirt { 0 } else { g };
    // What his ratings don't explain, one read at a time: his own part
    // (the same all career) and his teammates' and scheme's (one draw a
    // season, part of it carried from last season at the same stage).
    let same = c.context_stage == c.stage;
    let mut context = Vec::new();
    let reads: Vec<(Read, f64)> = Read::ALL
        .iter()
        .filter(|r| r.reads(p.position) && results::fit(level, **r, p.position).on())
        .map(|&r| {
            let fresh = draw.normal(0.0, 1.0);
            let prev = c.context.iter().find(|x| x.0 == r as u8).filter(|_| same).map(|x| x.1);
            let u = match prev {
                Some(v) => CONTEXT_CARRY * v + (1.0 - CONTEXT_CARRY * CONTEXT_CARRY).sqrt() * fresh,
                None => fresh,
            };
            context.push((r as u8, u));
            // What follows him to any team (his body, his other skills'
            // mix): one draw for his career; the rest is the team's.
            let f = results::fit(level, r, p.position);
            let own = Rng::new(key(&[seed as u64, id as u64, r as u64, 7])).normal(0.0, 1.0);
            let team = (f.tau * f.tau - f.tp * f.tp).max(0.0).sqrt();
            (r, own * f.tp + u * team)
        })
        .collect();
    c.ledger.roll(age as i32);
    let ledger = &mut c.ledger;
    let mut stats: Vec<(u8, f32, f32)> = reads.iter().map(|r| (r.0 as u8, 0.0, 0.0)).collect();
    let spec = p.position.is_specialist();
    let snaps_game = (share * results::season_snaps(level) / g as f64).round() as u32;
    let blocks = 13;
    let game_blocks: Vec<usize> = (0..games as usize).map(|k| 2 + k * 4 / games.max(1) as usize).collect();
    let mut gi = 0;
    let mut big = [0u16; 3];
    // His body and hidden traits for the season, as a fit reads them.
    let own = results::own(&afm_engine::game::GamePlayer::from_player(p, 1.0, 0.0));
    for b in 0..blocks {
        let base = [1.0 / blocks as f64; FAMILIES];
        let s = cover(&mut p.train, &base);
        land(p, &s, None, key(&[id as u64, age as u64, b as u64]) as u64);
        while gi < game_blocks.len() && game_blocks[gi] == b {
            let mut rr = Rng::new(key(&[seed as u64, id as u64, age as u64, 3, gi as u64]));
            let z = rr.normal(0.0, 1.0);
            let w = results::opp_weight(z);
            let mut man = Man { snaps: snaps_game, opp: z as f32, ..Man::default() };
            for (k, &(r, tau)) in reads.iter().enumerate() {
                let f = results::fit(level, r, p.position);
                let l = f.season / g as f64 * if spec { (share / 0.75).min(1.0) } else { share };
                let n = poisson(&mut rr, l);
                let e = rr.normal(0.0, 1.0);
                if n == 0 {
                    continue;
                }
                let nf = n as f64;
                let ex = f.expected(&p.attributes, &own, z, f.c0);
                let noise = nf.sqrt() * f.sd * e;
                stats[k].1 += n as f32;
                stats[k].2 += (nf * (ex + tau) + noise) as f32;
                let s = (nf * tau + noise) / f.b;
                man.reads.push((r, (nf * w) as f32, (s * w) as f32));
            }
            if mode.is_some_and(|m| m.on) {
                let mv = results::apply(p, ledger, &man, level, mode.as_ref().unwrap());
                if let Some((_, d)) = mv.read {
                    for (k, t) in [1.0, 1.5, 2.0].iter().enumerate() {
                        if d >= *t {
                            big[k] += 1;
                        }
                    }
                }
                results::reps(p, ledger, snaps_game, level, z, None);
            }
            gi += 1;
        }
    }
    if mode.is_some_and(|m| m.on) {
        results::close(p, ledger);
    }
    let mut events = 0u32;
    if let Some(h) = p.history.last() {
        for (k, e) in EVS.iter().enumerate() {
            if h.events.has(*e) {
                events |= 1 << k;
            }
        }
    }
    close_year(p, None);
    c.last = Some((level, share));
    c.context = context;
    c.context_stage = c.stage;
    if role == PlayingTime::Redshirt {
        c.redshirted = true;
    }
    Row {
        id,
        age,
        level: c.stage,
        role: role as u8,
        qb: c.p.position == Position::QB,
        top: c.top,
        start: start as f32,
        end: c.p.overall() as f32,
        results: ledger.total,
        best: ledger.best,
        best_read: ledger.best_read,
        big,
        skill: ledger.reads.iter().map(|e| e.applied).fold(0.0f32, |m, v| if v.abs() > m.abs() { v } else { m }),
        games: ledger.games,
        events,
        stats,
    }
}

struct Run {
    rows: Vec<Row>,
    /// Overall by stage marker: college freshmen, seniors, the draft class.
    stage: [Vec<f32>; 3],
}

fn run(base: &[Player], seed: u32, mode: Option<Mode>) -> Run {
    let mut cs: Vec<Career> = base
        .iter()
        .map(|p| Career {
            p: p.clone(),
            stage: 0,
            top: false,
            last: None,
            redshirted: false,
            context: Vec::new(),
            context_stage: 9,
            ledger: Ledger::default(),
        })
        .collect();
    let mut rows = Vec::new();
    let mut stage: [Vec<f32>; 3] = Default::default();
    for age in 14..30 {
        advance(&mut cs, age);
        match age {
            18 => stage[0] = cs.iter().filter(|c| c.stage == 1).map(|c| c.p.overall() as f32).collect(),
            21 => stage[1] = cs.iter().filter(|c| c.stage == 1).map(|c| c.p.overall() as f32).collect(),
            22 => stage[2] = cs.iter().filter(|c| c.stage == 2).map(|c| c.p.overall() as f32).collect(),
            _ => {}
        }
        let rl = roles(&cs, age);
        let out: Vec<Option<Row>> =
            cs.par_iter_mut().zip(rl.par_iter()).map(|(c, r)| r.map(|r| season(c, r, age, seed, mode))).collect();
        rows.extend(out.into_iter().flatten());
    }
    Run { rows, stage }
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 }
}

fn sd(v: &[f64]) -> f64 {
    let m = mean(v);
    if v.len() < 2 { 0.0 } else { (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64).sqrt() }
}

fn corr(a: &[f64], b: &[f64]) -> f64 {
    let (ma, mb) = (mean(a), mean(b));
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        sab += (x - ma) * (y - mb);
        saa += (x - ma).powi(2);
        sbb += (y - mb).powi(2);
    }
    if saa <= 0.0 || sbb <= 0.0 { 0.0 } else { sab / (saa * sbb).sqrt() }
}

fn pct(v: &[f64], q: f64) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    if s.is_empty() { 0.0 } else { s[((s.len() - 1) as f64 * q).round() as usize] }
}

const ROLES: [&str; 6] = ["starter", "rotation", "backup", "redshirt", "practice squad", "unsigned"];
const LEVELS: [&str; 3] = ["high school", "college", "NFL"];

fn reason(r: &Row) -> String {
    let mut w = Vec::new();
    let names = ["a breakout (dice)", "a slump (dice)", "stagnation (dice)", "a lost season", "", "", ""];
    for (k, n) in names.iter().enumerate() {
        if !n.is_empty() && r.events & (1 << k) != 0 {
            w.push(n.to_string());
        }
    }
    if r.results.abs() >= 0.5 {
        let best = Read::from_index(r.best_read as usize).name();
        w.push(format!("results {:+.1} over {} games (biggest game {:+.1}, {best})", r.results, r.games, r.best));
    }
    if w.is_empty() { "the yearly step alone".to_string() } else { w.join("; ") }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(20000);
    let seed: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let out = args.get(3).cloned();
    let real: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/real-world/results_real.json"))
            .expect("results_real.json"),
    )
    .expect("json");
    let t0 = std::time::Instant::now();
    let mut rng = Rng::new(format!("results-ab-{seed}"));
    let base: Vec<Player> = (0..n)
        .map(|k| {
            let z = rng.normal(0.9, 0.8);
            generate_prospect(
                &mut rng,
                ProspectOptions { id: Some(k as u32 + 1), talent_z: Some(z), ..Default::default() },
            )
        })
        .collect();
    let mut rep = String::new();
    let _ = writeln!(rep, "# Growth from results: the A/B on {n} synthetic players (seed {seed})\n");
    let arms = arms();
    let runs: Vec<Run> = arms.iter().map(|(_, m)| run(&base, seed, *m)).collect();
    let _ = writeln!(rep, "Ran in {:.0} s.\n", t0.elapsed().as_secs_f64());

    // 1. Growth per season by level and role.
    let _ = writeln!(rep, "## 1. Growth per season (overall points, checkpoint to checkpoint)\n");
    let mut head = "| level | role |".to_string();
    for (a, _) in &arms {
        let _ = write!(head, " {a} |");
    }
    let _ = writeln!(rep, "{head}\n|{}", "---|".repeat(arms.len() + 2));
    for lv in 0..3u8 {
        for rl in 0..4u8 {
            let cells: Vec<Vec<f64>> = runs
                .iter()
                .map(|r| {
                    r.rows.iter().filter(|x| x.level == lv && x.role == rl).map(|x| (x.end - x.start) as f64).collect()
                })
                .collect();
            if cells[0].len() < 30 {
                continue;
            }
            let mut line = format!("| {} | {} (n {}) |", LEVELS[lv as usize], ROLES[rl as usize], cells[0].len());
            for c in &cells {
                let _ = write!(line, " {:+.2} |", mean(c));
            }
            let _ = writeln!(rep, "{line}");
        }
    }
    for (label, top) in [("redshirt QBs, top programs", true), ("redshirt QBs, the rest", false)] {
        let mut line = format!("| college | {label} |");
        for r in &runs {
            let v: Vec<f64> = r
                .rows
                .iter()
                .filter(|x| x.level == 1 && x.role == 3 && x.qb && x.top == top)
                .map(|x| (x.end - x.start) as f64)
                .collect();
            let _ = write!(line, " {:+.2} (n {}) |", mean(&v), v.len());
        }
        let _ = writeln!(rep, "{line}");
    }

    // 2 and 3. The stat a season shows, next season, and its spread (NFL
    // starters with real qualifying volume).
    let _ = writeln!(rep, "\n## 2. How a season's stat carries into the next (NFL starters), against real\n");
    let _ = writeln!(rep, "Year-to-year r per read. Real: nflverse 2018-25 among qualifiers.\n");
    let mut head = "| read | real r |".to_string();
    for (a, _) in &arms {
        let _ = write!(head, " {a} |");
    }
    let _ = writeln!(rep, "{head}\n|{}", "---|".repeat(arms.len() + 2));
    let mut spread = String::new();
    let mut shead = "| read | real sd |".to_string();
    for (a, _) in &arms {
        let _ = write!(shead, " {a} |");
    }
    let _ = writeln!(spread, "{shead}\n|{}", "---|".repeat(arms.len() + 2));
    for read in Read::ALL {
        let rv = &real["reads"][format!("{read:?}").as_str()];
        let qual = rv["qualify"].as_f64().unwrap_or(40.0);
        let mut line = format!("| {} | {} |", read.name(), rv["r"].as_f64().map_or("-".into(), |x| format!("{x:+.2}")));
        let mut sl = format!("| {} | {} |", read.name(), rv["sd"].as_f64().map_or("-".into(), |x| format!("{x:.4}")));
        let mut any = false;
        for r in &runs {
            let mut by: HashMap<(u32, u32), f64> = HashMap::new();
            let mut all = Vec::new();
            for x in r.rows.iter().filter(|x| x.level == 2 && x.role == 0) {
                if let Some(s) = x.stats.iter().find(|s| s.0 == read as u8)
                    && s.1 as f64 >= qual
                {
                    let v = s.2 as f64 / s.1 as f64;
                    by.insert((x.id, x.age), v);
                    all.push(v);
                }
            }
            let (mut a, mut b) = (Vec::new(), Vec::new());
            for (&(id, age), &v) in &by {
                if let Some(&w) = by.get(&(id, age + 1)) {
                    a.push(v);
                    b.push(w);
                }
            }
            if a.len() >= 30 {
                any = true;
                let _ = write!(line, " {:+.2} ({}) |", corr(&a, &b), a.len());
                let _ = write!(sl, " {:.4} |", sd(&all));
            } else {
                let _ = write!(line, " - |");
                let _ = write!(sl, " - |");
            }
        }
        if any {
            let _ = writeln!(rep, "{line}");
            let _ = writeln!(spread, "{sl}");
        }
    }
    let _ = writeln!(rep, "\n## 3. The spread of each stat across NFL starter seasons, against real\n\n{spread}");

    // 4. Rating levels by stage.
    let _ = writeln!(rep, "## 4. Rating levels by stage (overall: mean, 90th percentile, 99th)\n");
    let mut head = "| stage |".to_string();
    for (a, _) in &arms {
        let _ = write!(head, " {a} |");
    }
    let _ = writeln!(rep, "{head}\n|{}", "---|".repeat(arms.len() + 1));
    let fmt = |v: &[f64]| format!("{:.1} / {:.1} / {:.1}", mean(v), pct(v, 0.9), pct(v, 0.99));
    for (k, label) in ["college freshmen (18)", "college seniors (21)", "the draft class (22)"].iter().enumerate() {
        let mut line = format!("| {label} |");
        for r in &runs {
            let v: Vec<f64> = r.stage[k].iter().map(|&x| x as f64).collect();
            let _ = write!(line, " {} |", fmt(&v));
        }
        let _ = writeln!(rep, "{line}");
    }
    for (label, lo, hi) in [("NFL starters 23-25", 23, 25), ("NFL starters 26-29", 26, 29)] {
        let mut line = format!("| {label} |");
        for r in &runs {
            let v: Vec<f64> = r
                .rows
                .iter()
                .filter(|x| x.level == 2 && x.role == 0 && x.age >= lo && x.age <= hi)
                .map(|x| x.end as f64)
                .collect();
            let _ = write!(line, " {} |", fmt(&v));
        }
        let _ = writeln!(rep, "{line}");
    }

    // 8. Breakouts and slumps; the share pulled down.
    let _ = writeln!(rep, "\n## 8. Breakouts and slumps per player-season, and the share pulled down\n");
    let mut head = "| | ".to_string();
    for (a, _) in &arms {
        let _ = write!(head, " {a} |");
    }
    let _ = writeln!(rep, "{head}\n|{}", "---|".repeat(arms.len() + 1));
    let played = |x: &&Row| x.games > 0 || x.role <= 2;
    type Line<'a> = (&'a str, Box<dyn Fn(&Row) -> bool>);
    let lines: [Line; 13] = [
        ("dice breakout", Box::new(|x: &Row| x.events & 1 != 0)),
        ("dice slump", Box::new(|x: &Row| x.events & 2 != 0)),
        ("season up 5+", Box::new(|x: &Row| x.end - x.start >= 5.0)),
        ("season down 3+", Box::new(|x: &Row| x.end - x.start <= -3.0)),
        ("a named big game (a skill up BIG_GAME in a game)", Box::new(|x: &Row| x.events & 16 != 0)),
        ("a named big season (a skill up BIG_SEASON)", Box::new(|x: &Row| x.events & 32 != 0)),
        ("a named rough season (a skill down BIG_SEASON)", Box::new(|x: &Row| x.events & 64 != 0)),
        ("pulled down by results (any)", Box::new(|x: &Row| x.results < 0.0)),
        ("pulled down 1+", Box::new(|x: &Row| x.results <= -1.0)),
        ("lifted 1+", Box::new(|x: &Row| x.results >= 1.0)),
        ("a skill moved 2+ up by results", Box::new(|x: &Row| x.skill >= 2.0)),
        ("a skill moved 4+ up by results", Box::new(|x: &Row| x.skill >= 4.0)),
        ("a skill moved 2+ down by results", Box::new(|x: &Row| x.skill <= -2.0)),
    ];
    for (label, f) in &lines {
        let mut line = format!("| {label} |");
        for r in &runs {
            let v: Vec<&Row> = r.rows.iter().filter(played).collect();
            let k = v.iter().filter(|x| f(x)).count();
            let _ = write!(line, " {:.1}% |", 100.0 * k as f64 / v.len().max(1) as f64);
        }
        let _ = writeln!(rep, "{line}");
    }
    for (label, bit) in [
        ("starter seasons: a named big game", 16),
        ("starter seasons: a named big season", 32),
        ("starter seasons: a named rough season", 64),
    ] {
        let mut line = format!("| {label} |");
        for r in &runs {
            let v: Vec<&Row> = r.rows.iter().filter(|x| x.role == 0 && x.games > 0).collect();
            let k = v.iter().filter(|x| x.events & bit != 0).count();
            let _ = write!(line, " {:.1}% |", 100.0 * k as f64 / v.len().max(1) as f64);
        }
        let _ = writeln!(rep, "{line}");
    }
    for (k, t) in ["1", "1.5", "2"].iter().enumerate() {
        let mut line = format!("| games where one skill moved {t}+ up (per player-season) |");
        for r in &runs {
            let v: Vec<&Row> = r.rows.iter().filter(played).collect();
            let g: f64 = v.iter().map(|x| x.big[k] as f64).sum();
            let _ = write!(line, " {:.3} |", g / v.len().max(1) as f64);
        }
        let _ = writeln!(rep, "{line}");
    }
    // Where the named moments' lines fall: the share of starter seasons
    // past each size, to set BIG_GAME and BIG_SEASON on.
    for (label, q) in [
        ("starter seasons: a game's top skill move, 86th percentile", 0),
        ("starter seasons: the season's top skill move, 90th percentile", 1),
        ("starter seasons: the season's top skill move, 10th percentile", 2),
    ] {
        let mut line = format!("| {label} |");
        for r in &runs {
            let mut v: Vec<f64> = r
                .rows
                .iter()
                .filter(|x| x.role == 0 && x.games > 0)
                .map(|x| if q == 0 { x.best.max(0.0) as f64 } else { x.skill as f64 })
                .collect();
            v.sort_by(|a, b| a.total_cmp(b));
            let at = if v.is_empty() { 0.0 } else { v[((v.len() - 1) as f64 * [0.86, 0.9, 0.1][q]) as usize] };
            let _ = write!(line, " {at:.2} |");
        }
        let _ = writeln!(rep, "{line}");
    }
    let mut line = "| results per season, sd (overall points) |".to_string();
    for r in &runs {
        let v: Vec<f64> = r.rows.iter().filter(played).map(|x| x.results as f64).collect();
        let _ = write!(line, " {:.2} |", sd(&v));
    }
    let _ = writeln!(rep, "{line}");
    let _ = writeln!(rep, "\n### Biggest risers and fallers in a season, per arm\n");
    for ((a, _), r) in arms.iter().zip(&runs) {
        let mut v: Vec<&Row> = r.rows.iter().filter(|x| x.level >= 1).collect();
        v.sort_by(|x, y| (y.end - y.start).total_cmp(&(x.end - x.start)));
        let _ = writeln!(rep, "**{a}**\n");
        for x in v.iter().take(3).chain(v.iter().rev().take(3)) {
            let pos = base[x.id as usize - 1].position;
            let _ = writeln!(
                rep,
                "- player {} ({}, {} {}, {}): {:.1} to {:.1} ({:+.1}): {}",
                x.id,
                pos.name(),
                LEVELS[x.level as usize],
                ROLES[x.role as usize],
                x.age,
                x.start,
                x.end,
                x.end - x.start,
                reason(x)
            );
        }
        let _ = writeln!(rep);
    }
    let _ = writeln!(rep, "### Biggest moves from results alone, per arm with results\n");
    for ((a, m), r) in arms.iter().zip(&runs) {
        if m.is_none() {
            continue;
        }
        let mut v: Vec<&Row> = r.rows.iter().filter(|x| x.games > 0).collect();
        v.sort_by(|x, y| y.skill.total_cmp(&x.skill));
        let _ = writeln!(rep, "**{a}**\n");
        for x in v.iter().take(4).chain(v.iter().rev().take(2)) {
            let pos = base[x.id as usize - 1].position;
            let best = Read::from_index(x.best_read as usize).name();
            let _ = writeln!(
                rep,
                "- player {} ({}, {} {}, {}): a skill {:+.1} from results, overall {:+.1} from results ({:+.1} in all), biggest game {:+.1} ({best})",
                x.id,
                pos.name(),
                LEVELS[x.level as usize],
                ROLES[x.role as usize],
                x.age,
                x.skill,
                x.results,
                x.end - x.start,
                x.best,
            );
        }
        let _ = writeln!(rep);
    }
    print!("{rep}");
    if let Some(path) = out {
        std::fs::write(&path, &rep).expect("write report");
        eprintln!("wrote {path}");
    }
}
