//! Running the tests: arms, the shared baseline, bands, verdicts against
//! real numbers, and the reports.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::time::Instant;

use afm_engine::attributes::Attr;
use afm_engine::game::GameTeam;
use afm_engine::player::Level;
use afm_engine::positions::Position;
use rayon::prelude::*;
use serde_json::{Value, json};

use super::table::{Kind, Test, group_name};
use super::{Arm, C, Cx, Knob, Part, Squad, Stat, Unit, Who, focus_of, one_game, stats, teams_for};

/// Blocks for the jackknife bands.
const BLOCKS: usize = 50;
const MT: [&str; 7] =
    ["mt_pts_for", "mt_pts_against", "mt_win", "mt_epa_off", "mt_epa_def", "mt_expl_for", "mt_expl_against"];

// ------------------------------------------------------------------ real numbers

/// `data/real-world/ab_real.json` (from `data/real-world/scripts/ab_real.py`).
pub struct Real(pub Value);

#[derive(Clone, Copy, Debug)]
pub struct Band {
    pub b: f64,
    pub lo: f64,
    pub hi: f64,
}

impl Band {
    fn of(v: &Value) -> Option<Band> {
        Some(Band { b: v["b"].as_f64()?, lo: v["lo"].as_f64()?, hi: v["hi"].as_f64()? })
    }
    fn clear(&self) -> bool {
        self.lo > 0.0 || self.hi < 0.0
    }
}

impl Real {
    pub fn load() -> Real {
        let path = std::env::var("AFM_AB_REAL")
            .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/real-world/ab_real.json").into());
        let v = std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
        Real(v)
    }
    /// (raw, held) slopes.
    pub fn slope(&self, key: &str) -> Option<(Band, Band)> {
        let s = &self.0["slopes"][key];
        Some((Band::of(&s["raw"])?, Band::of(&s["held"])?))
    }
    /// (p5, p95, the true p5-p95 width).
    pub fn spread(&self, key: &str) -> Option<(f64, f64, f64)> {
        let s = &self.0["spreads"][key];
        // CPOE comes in percentage points; every other share is a fraction.
        let k = if key.ends_with("cpoe") { 0.01 } else { 1.0 };
        let (p5, p95) = (k * s["p5"].as_f64()?, k * s["p95"].as_f64()?);
        let w = s["sd_true"].as_f64().filter(|x| *x > 0.0).map_or(p95 - p5, |sd| 3.29 * k * sd);
        Some((p5, p95, w))
    }
}

// ------------------------------------------------------------------ rows and bands

/// One arm's numbers: for each game, each stat's sums.
pub type Rows = Vec<Vec<C>>;

fn add(a: &mut C, b: &C) {
    for k in 0..5 {
        a[k] += b[k];
    }
}
fn sub(a: &C, b: &C) -> C {
    let mut o = *a;
    for k in 0..5 {
        o[k] -= b[k];
    }
    o
}

fn sums(rows: &Rows, k: usize, g: usize) -> (C, Vec<C>) {
    let mut t = [0.0; 5];
    let mut b = vec![[0.0; 5]; BLOCKS];
    for (i, r) in rows.iter().take(g).enumerate() {
        add(&mut t, &r[k]);
        add(&mut b[i * BLOCKS / g], &r[k]);
    }
    (t, b)
}

/// A difference between two arms with its 95% band, paired over the same
/// games.
#[derive(Clone, Copy, Debug, Default)]
pub struct Diff {
    pub a: f64,
    pub b: f64,
    pub d: f64,
    pub se: f64,
}

impl Diff {
    pub fn lo(&self) -> f64 {
        self.d - 1.96 * self.se
    }
    pub fn hi(&self) -> f64 {
        self.d + 1.96 * self.se
    }
    pub fn clear(&self) -> bool {
        self.d.is_finite() && (self.lo() > 0.0 || self.hi() < 0.0)
    }
}

pub fn diff(a: &Rows, b: &Rows, k: usize, value: fn(&C) -> f64) -> Diff {
    let g = a.len().min(b.len());
    if g < BLOCKS {
        return Diff { a: f64::NAN, b: f64::NAN, d: f64::NAN, se: f64::NAN };
    }
    let (ta, ba) = sums(a, k, g);
    let (tb, bb) = sums(b, k, g);
    let (va, vb) = (value(&ta), value(&tb));
    let ds: Vec<f64> = (0..BLOCKS).map(|i| value(&sub(&ta, &ba[i])) - value(&sub(&tb, &bb[i]))).collect();
    let ok: Vec<f64> = ds.iter().copied().filter(|x| x.is_finite()).collect();
    let n = ok.len() as f64;
    let m = ok.iter().sum::<f64>() / n.max(1.0);
    let var = (n - 1.0) / n.max(1.0) * ok.iter().map(|x| (x - m) * (x - m)).sum::<f64>();
    Diff { a: va, b: vb, d: va - vb, se: var.sqrt() }
}

fn value_of(rows: &Rows, k: usize, value: fn(&C) -> f64) -> f64 {
    let mut t = [0.0; 5];
    for r in rows {
        add(&mut t, &r[k]);
    }
    value(&t)
}

// ------------------------------------------------------------------ playing

/// What one arm run measures: a focus group and its stats.
pub struct Lens {
    pub focus: HashSet<u32>,
    pub stats: Vec<usize>,
}

/// Play games `from..to` for an arm, measuring each lens.
#[allow(clippy::too_many_arguments)]
pub fn play(
    teams: &[GameTeam],
    mine: bool,
    level: Level,
    seed: u32,
    from: usize,
    to: usize,
    lenses: &[Lens],
    st: &[Stat],
    pos: &HashMap<u32, Position>,
) -> Vec<Rows> {
    let body: HashMap<u32, [f64; 2]> =
        teams.iter().flat_map(|t| t.players.iter().map(|p| (p.id, [p.weight, p.forty()]))).collect();
    let per: Vec<Vec<Vec<C>>> = (from..to)
        .into_par_iter()
        .map(|g| {
            let (res, h, _) = one_game(teams, level, g, seed, mine);
            let side = mine.then_some(if h == 0 { 0 } else { 1 });
            lenses
                .iter()
                .map(|ln| {
                    let cx = Cx { focus: &ln.focus, pos, mine: side, body: &body };
                    ln.stats
                        .iter()
                        .map(|&k| {
                            let s = &st[k];
                            let mut c = [0.0; 5];
                            if let Some(f) = s.play {
                                for r in &res.stats.plays {
                                    add(&mut c, &f(r, &cx));
                                }
                            }
                            if let Some(f) = s.game {
                                add(&mut c, &f(&res, &cx));
                            }
                            c
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    let mut out: Vec<Rows> = (0..lenses.len()).map(|_| Vec::with_capacity(to - from)).collect();
    for g in per {
        for (k, v) in g.into_iter().enumerate() {
            out[k].push(v);
        }
    }
    out
}

fn idx(st: &[Stat], key: &str) -> usize {
    st.iter().position(|s| s.key == key).unwrap_or_else(|| panic!("no stat {key}"))
}

// ------------------------------------------------------------------ arms

fn knobs_for(kind: &Kind, end: usize) -> Vec<Knob> {
    let r = [-15.0, 0.0, 15.0][end];
    let tv = [4.0, 10.5, 16.0][end];
    match kind {
        Kind::Body { part, v, .. } => vec![Knob::Set(*part, v[end])],
        Kind::Rating(attrs) => attrs.iter().map(|a| Knob::Rating(*a, r)).collect(),
        Kind::Tend(t) => vec![Knob::Tend(*t, tv)],
        Kind::Consistency => vec![Knob::Consistency(tv)],
        Kind::RatingTend(a, _) => vec![Knob::Rating(*a, r)],
        Kind::Grid { part, v, .. } => vec![Knob::Set(*part, v[end])],
        Kind::Link(l) => vec![Knob::Link(*l, [0.0, 0.5, 1.0][end])],
    }
}

/// The groups' ranges for the as recruited arms: past one spread from the
/// typical value on each side (the arms are 1.5 spreads out).
fn recruit_ranges(v: [f64; 3]) -> [(f64, f64); 2] {
    let side = |x: f64| {
        let b = v[1] + (x - v[1]) / 1.5;
        if x < v[1] { (f64::NEG_INFINITY, b) } else { (b, f64::INFINITY) }
    };
    [side(v[0]), side(v[2])]
}

/// Every arm a test plays (the shared baseline is played once for all).
pub fn arms(t: &Test, with_mine: bool) -> Vec<Arm> {
    let mk = |label: &str, groups: Vec<(Who, Vec<Knob>)>, mine: bool| Arm { label: label.into(), groups, mine };
    let w = || t.who.clone();
    let mine = t.mine;
    let mut v = Vec::new();
    match &t.kind {
        Kind::Body { part, v: vals, recruit, cross } => {
            for (k, l) in ["lo", "typ", "hi"].iter().enumerate() {
                v.push(mk(l, vec![(w(), knobs_for(&t.kind, k))], mine));
            }
            if *recruit {
                let rr = recruit_ranges(*vals);
                v.push(mk("rec_lo", vec![(w(), vec![Knob::Recruit(*part, rr[0].0, rr[0].1)])], mine));
                v.push(mk("rec_hi", vec![(w(), vec![Knob::Recruit(*part, rr[1].0, rr[1].1)])], mine));
            }
            if let Some(td) = cross {
                for (k, l) in [(0, "lo"), (2, "hi")] {
                    for tv in [4.0, 16.0] {
                        let mut kn = knobs_for(&t.kind, k);
                        kn.push(Knob::Tend(*td, tv));
                        v.push(mk(&format!("{l}_t{tv}"), vec![(w(), kn)], mine));
                    }
                }
            }
        }
        Kind::RatingTend(_, td) => {
            for (k, l) in [(0, "lo"), (2, "hi")] {
                for tv in [4.0, 16.0] {
                    let mut kn = knobs_for(&t.kind, k);
                    kn.push(Knob::Tend(*td, tv));
                    v.push(mk(&format!("{l}_t{tv}"), vec![(w(), kn)], mine));
                }
            }
        }
        Kind::Grid { part, v: vals, other, opart, ov } => {
            for (i, &vi) in vals.iter().enumerate() {
                for (j, &oj) in ov.iter().enumerate() {
                    v.push(mk(
                        &format!("g{i}{j}"),
                        vec![(w(), vec![Knob::Set(*part, vi)]), (other.clone(), vec![Knob::Set(*opart, oj)])],
                        false,
                    ));
                }
            }
        }
        Kind::Link(_) => {
            v.push(mk("lo", vec![(w(), knobs_for(&t.kind, 0))], mine));
            v.push(mk("hi", vec![(w(), knobs_for(&t.kind, 2))], mine));
        }
        _ => {
            v.push(mk("lo", vec![(w(), knobs_for(&t.kind, 0))], mine));
            v.push(mk("hi", vec![(w(), knobs_for(&t.kind, 2))], mine));
        }
    }
    if with_mine && t.headline && !t.mine {
        v.push(mk("mine_lo", vec![(w(), knobs_for(&t.kind, 0))], true));
        v.push(mk("mine_hi", vec![(w(), knobs_for(&t.kind, 2))], true));
    }
    v
}

/// Tests whose typical arm is the untouched league.
pub fn uses_base(t: &Test) -> bool {
    matches!(t.kind, Kind::Rating(_) | Kind::Tend(_) | Kind::Consistency)
}

fn part_of(t: &Test) -> Option<Part> {
    match t.kind {
        Kind::Body { part, .. } | Kind::Grid { part, .. } => Some(part),
        _ => None,
    }
}

/// The mean of a body number over the focus group in an arm's teams.
fn mean_part(base: &[Squad], teams: &[GameTeam], focus: &HashSet<u32>, part: Part) -> f64 {
    let (mut s, mut n) = (0.0f64, 0.0f64);
    for (sq, t) in base.iter().zip(teams) {
        for (k, gp) in t.players.iter().enumerate() {
            if focus.contains(&gp.id) {
                s += part.of(gp, &sq.people[k]);
                n += 1.0;
            }
        }
    }
    s / n.max(1.0)
}

fn mean_attrs(teams: &[GameTeam], focus: &HashSet<u32>) -> Vec<f64> {
    let mut s = vec![0.0; Attr::ALL.len()];
    let mut n = 0.0;
    for t in teams {
        for gp in t.players.iter().filter(|g| focus.contains(&g.id)) {
            for (k, a) in Attr::ALL.into_iter().enumerate() {
                s[k] += gp.a(a);
            }
            n += 1.0;
        }
    }
    s.iter().map(|x| x / f64::max(n, 1.0)).collect()
}

// ------------------------------------------------------------------ one test

pub struct ArmRun {
    pub label: String,
    pub rows: Rows,
    pub x: Option<f64>,
    pub attrs: Vec<f64>,
}

pub struct TestRun {
    pub test: Test,
    pub arms: Vec<ArmRun>,
    /// Stat keys in each arm's rows (the chain, then the my team stats for
    /// the my team arms).
    pub keys: Vec<&'static str>,
    pub mine_keys: Vec<&'static str>,
    pub world: Vec<ArmRun>,
    pub games: usize,
    pub secs: f64,
}

pub struct Ctx<'a> {
    pub level: Level,
    pub seed: u32,
    pub base: &'a [Squad],
    pub st: &'a [Stat],
    pub pos: HashMap<u32, Position>,
}

impl<'a> Ctx<'a> {
    pub fn new(level: Level, seed: u32, base: &'a [Squad], st: &'a [Stat]) -> Ctx<'a> {
        let pos = base.iter().flat_map(|s| s.team.players.iter().map(|p| (p.id, p.position))).collect();
        Ctx { level, seed, base, st, pos }
    }

    fn lens(&self, t: &Test, mine: bool, extra: &[&'static str]) -> Lens {
        let mut stats: Vec<usize> = t.chain.iter().map(|k| idx(self.st, k)).collect();
        for k in extra {
            stats.push(idx(self.st, k));
        }
        Lens { focus: focus_of(self.base, &t.who, mine), stats }
    }

    pub fn run_arm(&self, t: &Test, arm: &Arm, from: usize, to: usize) -> ArmRun {
        let teams = teams_for(self.base, arm, self.level);
        let extra: &[&str] = if arm.mine { &MT } else { &[] };
        let ln = self.lens(t, arm.mine, extra);
        let mut rows =
            play(&teams, arm.mine, self.level, self.seed, from, to, std::slice::from_ref(&ln), self.st, &self.pos);
        let x = part_of(t).map(|p| mean_part(self.base, &teams, &ln.focus, p));
        let attrs = if arm.label.starts_with("rec") { mean_attrs(&teams, &ln.focus) } else { Vec::new() };
        ArmRun { label: arm.label.clone(), rows: rows.pop().unwrap(), x, attrs }
    }
}

/// The shared untouched league for every test that reads it, in one pass.
pub fn base_rows(cx: &Ctx, tests: &[&Test], mine: bool, games: usize) -> HashMap<&'static str, Rows> {
    let list: Vec<&&Test> = tests.iter().filter(|t| uses_base(t) && t.mine == mine).collect();
    if list.is_empty() {
        return HashMap::new();
    }
    let teams: Vec<GameTeam> = cx.base.iter().map(|s| s.team.clone()).collect();
    let extra: &[&str] = if mine { &MT } else { &[] };
    let lenses: Vec<Lens> = list.iter().map(|t| cx.lens(t, mine, extra)).collect();
    let t0 = Instant::now();
    let rows = play(&teams, mine, cx.level, cx.seed, 0, games, &lenses, cx.st, &cx.pos);
    eprintln!(
        "  baseline{}: {} games for {} tests [{:.0} s]",
        if mine { " (my team)" } else { "" },
        games,
        list.len(),
        t0.elapsed().as_secs_f64()
    );
    list.iter().map(|t| t.id).zip(rows).collect()
}

/// The headline stat with a real slope, and its key.
fn grow_key(t: &Test, real: &Real) -> Option<(usize, &'static str)> {
    t.chain.iter().enumerate().find_map(|(k, s)| {
        t.real.iter().find(|r| r.0 == *s && r.1.contains("__") && real.slope(r.1).is_some()).map(|r| (k, r.1))
    })
}

fn slope_unit(t: &Test) -> f64 {
    part_of(t).map_or(1.0, |p| p.unit().1)
}

pub fn run_test(cx: &Ctx, t: &Test, games: usize, max: usize, real: &Real, world: Option<&[Squad]>) -> TestRun {
    let t0 = Instant::now();
    let arms_list = arms(t, true);
    let mut runs: Vec<ArmRun> = arms_list.par_iter().map(|a| cx.run_arm(t, a, 0, games)).collect();
    let mut g = games;
    // Grow until the headline's band is narrower than half the real effect.
    // Real numbers are the NFL's, so only NFL arms grow.
    if let (Some((k, key)), Kind::Body { .. }, Level::NFL) = (grow_key(t, real), &t.kind, cx.level) {
        let held = real.slope(key).unwrap().1;
        loop {
            let (lo, hi) = (find(&runs, "lo"), find(&runs, "hi"));
            let d = diff(&hi.rows, &lo.rows, k, cx.st[idx(cx.st, t.chain[k])].value);
            let dx = (hi.x.unwrap() - lo.x.unwrap()) / slope_unit(t);
            let width = 2.0 * 1.96 * d.se / dx.abs();
            if g >= max || !width.is_finite() || width < 0.5 * held.b.abs() {
                break;
            }
            let next = (2 * g).min(max);
            eprintln!(
                "  {}: band {:.4} against half the real {:.4}; {} more games",
                t.id,
                width,
                0.5 * held.b.abs(),
                next - g
            );
            for (arm, run) in arms_list.iter().zip(runs.iter_mut()) {
                if arm.mine {
                    continue;
                }
                let more = cx.run_arm(t, arm, g, next);
                run.rows.extend(more.rows);
            }
            g = next;
        }
    }
    let mut world_runs = Vec::new();
    if let Some(w) = world.filter(|_| t.headline && !t.mine) {
        let wcx = Ctx::new(cx.level, cx.seed, w, cx.st);
        let mut list: Vec<Arm> =
            arms(t, false).into_iter().filter(|a| ["lo", "typ", "hi"].contains(&a.label.as_str())).collect();
        if uses_base(t) {
            list.push(Arm { label: "typ".into(), groups: Vec::new(), mine: false });
        }
        world_runs = list.par_iter().map(|a| wcx.run_arm(t, a, 0, games)).collect();
    }
    let keys = t.chain.clone();
    let mut mine_keys = t.chain.clone();
    mine_keys.extend(MT);
    let secs = t0.elapsed().as_secs_f64();
    eprintln!("  {} ({}): {} arms, {} games [{:.0} s]", t.id, t.name, runs.len(), g, secs);
    TestRun { test: t.clone(), arms: runs, keys, mine_keys, world: world_runs, games: g, secs }
}

fn find<'r>(runs: &'r [ArmRun], label: &str) -> &'r ArmRun {
    runs.iter().find(|r| r.label == label).unwrap_or_else(|| panic!("no arm {label}"))
}
fn get<'r>(runs: &'r [ArmRun], label: &str) -> Option<&'r ArmRun> {
    runs.iter().find(|r| r.label == label)
}

// ------------------------------------------------------------------ verdicts

fn show(v: f64, u: Unit) -> String {
    if !v.is_finite() {
        return "–".into();
    }
    match u {
        Unit::Pct if v.abs() < 0.02 => format!("{:.2}%", 100.0 * v),
        Unit::Pct => format!("{:.1}%", 100.0 * v),
        Unit::Num if v.abs() < 1.0 => format!("{v:.3}"),
        Unit::Num => format!("{v:.2}"),
    }
}

fn show_d(d: &Diff, u: Unit) -> String {
    if !d.d.is_finite() {
        return "–".into();
    }
    let (m, unit) = if u == Unit::Pct { (100.0, " pts") } else { (1.0, "") };
    let p = if (d.d * m).abs() < 0.1 || (d.se * m) < 0.05 { 3 } else { 2 };
    format!(
        "{:+.p$}{unit} [{:+.p$}, {:+.p$}]{}",
        d.d * m,
        d.lo() * m,
        d.hi() * m,
        if d.clear() { "" } else { " (no clear effect)" },
    )
}

#[derive(Clone, Debug)]
pub struct Verdict {
    pub text: String,
    pub miss: bool,
}

fn v(text: String, miss: bool) -> Verdict {
    Verdict { text, miss }
}

/// The engine's slope against a real one (per unit).
fn slope_verdict(e: &Diff, dx: f64, r: Band, real_kind: &str) -> Verdict {
    let (b, lo, hi) = (e.d / dx, e.lo() / dx, e.hi() / dx);
    let (lo, hi) = (lo.min(hi), lo.max(hi));
    let eng = format!("engine {b:+.4} [{lo:+.4}, {hi:+.4}]");
    let rl = format!("real {real_kind} {:+.4} [{:+.4}, {:+.4}]", r.b, r.lo, r.hi);
    let engine_clear = lo > 0.0 || hi < 0.0;
    let overlap = lo <= r.hi && hi >= r.lo;
    if !engine_clear && r.clear() {
        return v(format!("MISS: no clear effect where real shows one ({eng}; {rl})"), true);
    }
    if !engine_clear {
        return v(format!("both flat ({eng}; {rl})"), false);
    }
    if (r.lo..=r.hi).contains(&b) {
        return v(format!("in the real band ({eng}; {rl})"), false);
    }
    if overlap {
        return v(format!("at the edge of the real band ({eng}; {rl})"), false);
    }
    if r.clear() && b.signum() != r.b.signum() {
        return v(format!("MISS: the wrong way ({eng}; {rl})"), true);
    }
    let k = if r.b.abs() > 1e-9 { b / r.b } else { f64::INFINITY };
    if !r.clear() {
        return v(format!("MISS: an edge real doesn't show ({eng}; {rl})"), true);
    }
    v(format!("MISS: {} ({:.1}× real; {eng}; {rl})", if k.abs() > 1.0 { "too big" } else { "too small" }, k), true)
}

/// A rating's swing against how far real starters spread (proposed, not
/// confirmed: off past the whole spread, or under a tenth of it).
fn spread_verdict(e: &Diff, real: (f64, f64, f64), u: Unit) -> Verdict {
    let (p5, p95, w) = real;
    let share = e.d.abs() / w;
    let r =
        format!("real starters p5 {} to p95 {}; the swing is {:.2} of that spread", show(p5, u), show(p95, u), share);
    if !e.clear() {
        return v(format!("MISS: barely moves it, no clear effect ({r})"), true);
    }
    if share > 1.0 {
        return v(format!("MISS: moves it more than real players differ ({r})"), true);
    }
    if share < 0.1 {
        return v(format!("MISS: barely moves it ({r})"), true);
    }
    v(format!("in range ({r})"), false)
}

fn sign_word(d: &Diff, want: f64) -> Option<Verdict> {
    if want == 0.0 || !d.d.is_finite() {
        return None;
    }
    Some(if !d.clear() {
        v("no clear effect (expected one)".into(), true)
    } else if d.d.signum() == want {
        v("points the right way".into(), false)
    } else {
        v("MISS: points the wrong way".into(), true)
    })
}

// ------------------------------------------------------------------ the report

pub struct Out {
    pub md: String,
    pub json: Value,
    pub misses: Vec<Value>,
}

fn arm_value(label: &str, x: Option<f64>, part: Option<Part>) -> String {
    match (x, part) {
        (Some(x), Some(p)) => format!("{label} ({})", p.show(x)),
        _ => label.into(),
    }
}

pub fn report(cx: &Ctx, tr: &TestRun, base: Option<&Rows>, real: &Real) -> Out {
    let t = &tr.test;
    let st = cx.st;
    let mut md = String::new();
    let mut js = serde_json::Map::new();
    let mut misses = Vec::new();
    let part = part_of(t);
    let _ = writeln!(md, "\n### {} (`{}`)\n", t.name, t.id);
    if !t.note.is_empty() {
        let _ = writeln!(md, "Step 0: {}.\n", t.note);
    }
    let unit = part.map(|p| p.unit());
    let stat = |k: &str| &st[idx(st, k)];
    let lo_hi = match &t.kind {
        Kind::Grid { .. } | Kind::RatingTend(..) => None,
        _ => Some((find(&tr.arms, "lo"), find(&tr.arms, "hi"))),
    };
    let typ: Option<&Rows> = get(&tr.arms, "typ").map(|a| &a.rows).or(base);
    let mut why = Vec::new();
    let mut flat: Vec<&str> = Vec::new();
    if let Some((lo, hi)) = lo_hi {
        let dx = match (lo.x, hi.x, unit) {
            (Some(a), Some(b), Some((_, u))) => (b - a) / u,
            _ => 1.0,
        };
        let names = match &t.kind {
            Kind::Link(_) => ("off", "full"),
            Kind::Tend(_) | Kind::Consistency => ("at 4", "at 16"),
            Kind::Rating(_) => ("−15", "+15"),
            _ => ("low", "high"),
        };
        let _ = writeln!(
            md,
            "Arms: {} / {} / {}; {} games an arm.\n",
            arm_value(names.0, lo.x, part),
            arm_value("typical", get(&tr.arms, "typ").and_then(|a| a.x), part),
            arm_value(names.1, hi.x, part),
            tr.games,
        );
        let _ = writeln!(
            md,
            "| Stat | {} | Typical | {} | {} − {} (95% band) | Check against real |",
            names.0, names.1, names.1, names.0
        );
        let _ = writeln!(md, "|---|---|---|---|---|---|");
        let mut rows_js = Vec::new();
        for (k, key) in tr.keys.iter().enumerate() {
            let s = stat(key);
            let d = diff(&hi.rows, &lo.rows, k, s.value);
            let ty = typ.map_or(f64::NAN, |r| value_of(r, k, s.value));
            let mut checks: Vec<Verdict> = Vec::new();
            for (_, rk) in t.real.iter().filter(|r| r.0 == *key) {
                if rk.contains("__") {
                    match real.slope(rk) {
                        Some((_, held)) => checks.push(slope_verdict(&d, dx, held, "held equal")),
                        None => checks.push(v(format!("no real number ({rk} not in ab_real.json)"), false)),
                    }
                } else {
                    match real.spread(rk) {
                        Some(sp) => checks.push(spread_verdict(&d, sp, s.unit)),
                        None => checks.push(v(format!("no real number ({rk} not in ab_real.json)"), false)),
                    }
                }
            }
            if k == 0
                && let Some(sv) = sign_word(&d, t.sign)
                && checks.iter().all(|c| !c.miss)
            {
                checks.push(sv);
            }
            let check = if checks.is_empty() {
                "–".to_string()
            } else {
                checks.iter().map(|c| c.text.clone()).collect::<Vec<_>>().join("; ")
            };
            for c in checks.iter().filter(|c| c.miss) {
                misses.push(json!({"test": t.id, "name": t.name, "stat": key, "label": s.label, "d": d.d, "lo": d.lo(), "hi": d.hi(), "check": c.text, "arms": format!("{} vs {}", names.1, names.0)}));
            }
            let _ = writeln!(
                md,
                "| {} | {} | {} | {} | {} | {} |",
                s.label,
                show(d.b, s.unit),
                show(ty, s.unit),
                show(d.a, s.unit),
                show_d(&d, s.unit),
                check
            );
            if d.clear() {
                let (m, u) = if s.unit == Unit::Pct { (100.0, " pts") } else { (1.0, "") };
                why.push(format!("{} {:+.2}{u}", s.label, d.d * m));
            } else if d.d.is_finite() {
                flat.push(s.label);
            }
            rows_js.push(json!({"stat": key, "lo": d.b, "typ": ty, "hi": d.a, "d": d.d, "band": [d.lo(), d.hi()], "checks": checks.iter().map(|c| c.text.clone()).collect::<Vec<_>>()}));
        }
        js.insert("rows".into(), Value::Array(rows_js));
        let _ = writeln!(
            md,
            "\nWhy ({} against {}): {}{}.",
            names.1,
            names.0,
            if why.is_empty() { "nothing changes clearly".to_string() } else { why.join(", ") },
            if flat.is_empty() || why.is_empty() {
                String::new()
            } else {
                format!("; no clear change in {}", flat.join(", "))
            },
        );
    }
    // Crossed with a habit.
    if matches!(t.kind, Kind::RatingTend(..) | Kind::Body { cross: Some(_), .. }) {
        let td = match &t.kind {
            Kind::RatingTend(_, td) | Kind::Body { cross: Some(td), .. } => *td,
            _ => unreachable!(),
        };
        let _ = writeln!(md, "\nCrossed with the {:?} habit (4 and 16):\n", td);
        let _ = writeln!(
            md,
            "| Stat | low, habit 4 | low, habit 16 | high, habit 4 | high, habit 16 | high − low at 4 | high − low at 16 |"
        );
        let _ = writeln!(md, "|---|---|---|---|---|---|---|");
        for (k, key) in tr.keys.iter().enumerate() {
            let s = stat(key);
            let a = |l: &str| find(&tr.arms, l);
            let d4 = diff(&a("hi_t4").rows, &a("lo_t4").rows, k, s.value);
            let d16 = diff(&a("hi_t16").rows, &a("lo_t16").rows, k, s.value);
            let _ = writeln!(
                md,
                "| {} | {} | {} | {} | {} | {} | {} |",
                s.label,
                show(d4.b, s.unit),
                show(d16.b, s.unit),
                show(d4.a, s.unit),
                show(d16.a, s.unit),
                show_d(&d4, s.unit),
                show_d(&d16, s.unit)
            );
        }
    }
    // As recruited.
    if let (Some(rl), Some(rh)) = (get(&tr.arms, "rec_lo"), get(&tr.arms, "rec_hi")) {
        let dx = match (rl.x, rh.x, unit) {
            (Some(a), Some(b), Some((_, u))) => (b - a) / u,
            _ => 1.0,
        };
        let _ = writeln!(
            md,
            "\nAs recruited: the generator's players at the position, matched on overall, {} against {}:\n",
            arm_value("low group", rl.x, part),
            arm_value("high group", rh.x, part)
        );
        let _ =
            writeln!(md, "| Stat | Low group | High group | High − low (95% band) | Check against real (raw slope) |");
        let _ = writeln!(md, "|---|---|---|---|---|");
        let mut rj = Vec::new();
        for (k, key) in tr.keys.iter().enumerate() {
            let s = stat(key);
            let d = diff(&rh.rows, &rl.rows, k, s.value);
            let mut checks = Vec::new();
            for (_, rk) in t.real.iter().filter(|r| r.0 == *key && r.1.contains("__")) {
                if let Some((raw, _)) = real.slope(rk) {
                    checks.push(slope_verdict(&d, dx, raw, "raw"));
                }
            }
            for c in checks.iter().filter(|c| c.miss) {
                misses.push(json!({"test": t.id, "name": t.name, "stat": key, "label": s.label, "d": d.d, "lo": d.lo(), "hi": d.hi(), "check": c.text, "arms": "as recruited, high group vs low group"}));
            }
            let check = if checks.is_empty() {
                "–".into()
            } else {
                checks.iter().map(|c| c.text.clone()).collect::<Vec<_>>().join("; ")
            };
            let _ = writeln!(
                md,
                "| {} | {} | {} | {} | {} |",
                s.label,
                show(d.b, s.unit),
                show(d.a, s.unit),
                show_d(&d, s.unit),
                check
            );
            rj.push(json!({"stat": key, "lo": d.b, "hi": d.a, "d": d.d, "band": [d.lo(), d.hi()]}));
        }
        js.insert("recruited".into(), Value::Array(rj));
        // What each group brings with it.
        // Only ratings his position's overall counts.
        let p0 = t.who.pos[0];
        let mut gaps: Vec<(f64, Attr)> = Attr::ALL
            .into_iter()
            .enumerate()
            .filter(|(_, a)| afm_engine::positions::weight(p0, *a) > 0.0)
            .map(|(k, a)| (rh.attrs[k] - rl.attrs[k], a))
            .collect();
        gaps.sort_by(|a, b| b.0.abs().total_cmp(&a.0.abs()));
        let brings: Vec<String> = gaps.iter().take(8).map(|(d, a)| format!("{a:?} {d:+.1}")).collect();
        let _ = writeln!(
            md,
            "\nWhat each group brings at the same overall (the high group's {:?} ratings against the low group's, biggest gaps): {}.",
            p0,
            brings.join(", ")
        );
        js.insert("brings".into(), json!(gaps.iter().take(8).map(|(d, a)| (format!("{a:?}"), *d)).collect::<Vec<_>>()));
    }
    // Matchup grid.
    if let Kind::Grid { v: mv, ov, opart, other, part: p, .. } = &t.kind {
        let _ = writeln!(
            md,
            "Arms: his {:?} {} / {} / {} against the {:?} {:?} {} / {} / {}; {} games a cell.\n",
            p,
            p.show(mv[0]),
            p.show(mv[1]),
            p.show(mv[2]),
            other.pos,
            opart,
            opart.show(ov[0]),
            opart.show(ov[1]),
            opart.show(ov[2]),
            tr.games
        );
        let mut gj = Vec::new();
        for (k, key) in tr.keys.iter().enumerate() {
            let s = stat(key);
            let _ = writeln!(md, "{} (rows: his {:?}; columns: the {:?} {:?}):\n", s.label, p, other.pos, opart);
            let _ = writeln!(md, "| | {} | {} | {} |", opart.show(ov[0]), opart.show(ov[1]), opart.show(ov[2]));
            let _ = writeln!(md, "|---|---|---|---|");
            let mut cells = Vec::new();
            for (i, &mi) in mv.iter().enumerate() {
                let vals: Vec<f64> =
                    (0..3).map(|j| value_of(&find(&tr.arms, &format!("g{i}{j}")).rows, k, s.value)).collect();
                cells.push(vals.clone());
                let _ = writeln!(
                    md,
                    "| {} | {} | {} | {} |",
                    p.show(mi),
                    show(vals[0], s.unit),
                    show(vals[1], s.unit),
                    show(vals[2], s.unit)
                );
            }
            let a = |l: &str| &find(&tr.arms, l).rows;
            let mine = diff(a("g21"), a("g01"), k, s.value);
            let theirs = diff(a("g12"), a("g10"), k, s.value);
            let _ = writeln!(
                md,
                "\nHis size (against a typical man): {}. The other man's size (against a typical one): {}.\n",
                show_d(&mine, s.unit),
                show_d(&theirs, s.unit)
            );
            gj.push(json!({"stat": key, "cells": cells, "his": [mine.d, mine.lo(), mine.hi()], "theirs": [theirs.d, theirs.lo(), theirs.hi()]}));
        }
        js.insert("grid".into(), Value::Array(gj));
    }
    // My team view.
    if let (Some(ml), Some(mh)) = (get(&tr.arms, "mine_lo"), get(&tr.arms, "mine_hi")) {
        let _ = writeln!(
            md,
            "\nMy team view: one team's players at the position low against high, every game against the rest:\n"
        );
        let _ = writeln!(md, "| Stat | Low | High | High − low (95% band) |");
        let _ = writeln!(md, "|---|---|---|---|");
        let mut mj = Vec::new();
        for (k, key) in tr.mine_keys.iter().enumerate() {
            let s = stat(key);
            let d = diff(&mh.rows, &ml.rows, k, s.value);
            let _ = writeln!(
                md,
                "| {} | {} | {} | {} |",
                s.label,
                show(d.b, s.unit),
                show(d.a, s.unit),
                show_d(&d, s.unit)
            );
            mj.push(json!({"stat": key, "lo": d.b, "hi": d.a, "d": d.d, "band": [d.lo(), d.hi()]}));
        }
        js.insert("mine".into(), Value::Array(mj));
    }
    // World rosters.
    if !tr.world.is_empty() {
        let (lo, hi) = (find(&tr.world, "lo"), find(&tr.world, "hi"));
        let _ = writeln!(md, "\nOn the released seed-41 world's 2026 NFL rosters (pure arms):\n");
        let _ = writeln!(md, "| Stat | Low | Typical | High | High − low (95% band) |");
        let _ = writeln!(md, "|---|---|---|---|---|");
        let typ = find(&tr.world, "typ");
        let mut wj = Vec::new();
        for (k, key) in tr.keys.iter().enumerate() {
            let s = stat(key);
            let d = diff(&hi.rows, &lo.rows, k, s.value);
            let _ = writeln!(
                md,
                "| {} | {} | {} | {} | {} |",
                s.label,
                show(d.b, s.unit),
                show(value_of(&typ.rows, k, s.value), s.unit),
                show(d.a, s.unit),
                show_d(&d, s.unit)
            );
            wj.push(json!({"stat": key, "lo": d.b, "hi": d.a, "d": d.d, "band": [d.lo(), d.hi()]}));
        }
        js.insert("world".into(), Value::Array(wj));
    }
    js.insert("id".into(), json!(t.id));
    js.insert("name".into(), json!(t.name));
    js.insert("games".into(), json!(tr.games));
    js.insert("note".into(), json!(t.note));
    js.insert("secs".into(), json!(tr.secs));
    js.insert("arms".into(), json!(tr.arms.iter().map(|a| json!({"label": a.label, "x": a.x})).collect::<Vec<_>>()));
    Out { md, json: Value::Object(js), misses }
}

/// Every group's file for a level: the tests, then their misses.
pub fn group_md(level: Level, group: &str, outs: &[Out], seed: u32) -> String {
    let mut md = String::new();
    let _ = writeln!(md, "# A/B tests: {} ({:?})\n", group_name(group), level);
    let _ = writeln!(
        md,
        "`ab_check {group} {} <games> {seed}`: same engine and code paths as the game; 32 synthetic teams at the level's spread, every game on the same seed in every arm. Bands are 95%, from a block jackknife over the games (paired). \"Typical\" is the untouched league for ratings and habits, and everyone at the middle value for a body number. Exact arm values are Claude's picks, proposed, not confirmed.",
        format!("{level:?}").to_lowercase()
    );
    let n: usize = outs.iter().map(|o| o.misses.len()).sum();
    let _ = writeln!(md, "\nMisses in this group: {n}.");
    for o in outs {
        md.push_str(&o.md);
    }
    md
}

// ------------------------------------------------------------------ CI

/// The direction test (quick CI): each position's headline, low against
/// high, on a fixed seed. Returns (test, stat, high − low, its band, the
/// expected sign).
pub fn direction(
    level: Level,
    games: usize,
    seed: u32,
    list: &[(&str, &str, f64)],
) -> Vec<(String, String, Diff, f64)> {
    let base = super::synthetic(level, seed);
    let st = stats();
    let cx = Ctx::new(level, seed, &base, &st);
    let tests = super::table::all();
    list.iter()
        .map(|&(id, key, want)| {
            let t = tests.iter().find(|t| t.id == id).unwrap_or_else(|| panic!("no test {id}"));
            let mut t = t.clone();
            t.chain = vec![
                tests
                    .iter()
                    .find(|x| x.id == id)
                    .unwrap()
                    .chain
                    .iter()
                    .copied()
                    .find(|k| *k == key)
                    .unwrap_or_else(|| panic!("{key} not in {id}")),
            ];
            let arms = arms(&t, false);
            let lo = cx.run_arm(&t, arms.iter().find(|a| a.label == "lo").unwrap(), 0, games);
            let hi = cx.run_arm(&t, arms.iter().find(|a| a.label == "hi").unwrap(), 0, games);
            let s = &st[idx(&st, key)];
            (id.to_string(), key.to_string(), diff(&hi.rows, &lo.rows, 0, s.value), want)
        })
        .collect()
}
