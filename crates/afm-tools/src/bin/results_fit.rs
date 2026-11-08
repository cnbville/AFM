//! Fits growth from results' table (`crates/afm-engine/data/results_fit.csv`,
//! read by `afm_engine::results::table`) on the match engine's own plays at
//! each level and position, as `xcomp_fit` fits expected completions: for
//! every skill read, the result a man gets on a trial from all he is (each
//! of his ratings, his body, his hidden game traits), how much a stronger
//! opponent takes away and the play's own context (a kick's distance); a
//! trial's spread; the trials in a full starter's season; what the model
//! leaves (what follows the man to another team, and his teammates and
//! scheme); and the skill's spread among the level's regulars.
//!
//! Each game moves every man's ratings at random by `SHIFT`, so each
//! rating's own slope shows apart from the rest of his game (synthetic
//! leagues have too few men, and their skills move together). The second
//! half of the games moves every man to another team, so what follows the
//! man shows apart from his teammates. A read whose result doesn't move
//! with its rating (t under 4) is left off there.
//! Refit after engine changes that move the reads:
//! `cargo run --release --bin results_fit -- 2000 [seed] [csv]`

use std::collections::HashMap;
use std::fmt::Write as _;

use afm_engine::attributes::ATTRIBUTE_COUNT;
use afm_engine::game::flow::quick_game_in;
use afm_engine::game::synthetic::synthetic_league;
use afm_engine::game::{GamePlayer, GameTeam};
use afm_engine::player::Level;
use afm_engine::positions::Position;
use afm_engine::results::{self, Fit, OWN, Read};
use afm_engine::rng::Rng;
use rayon::prelude::*;

/// A man's ratings, then his own (body and hidden traits), as one game had them.
const NV: usize = ATTRIBUTE_COUNT + OWN;

/// One trial: player id, result, opponent strength, context, his vector
/// for the game, and whether every man had moved team.
#[derive(Clone, Copy)]
struct Row {
    id: u32,
    y: f32,
    z: f32,
    cx: f32,
    v: u32,
    late: bool,
}

/// How far each man's ratings move at random each game (rating points,
/// one standard deviation).
const SHIFT: f64 = 8.0;

/// The league with every man moved to another team in the same place
/// (position and rank there), so what follows a man whatever his team
/// shows apart from his teammates.
fn shuffled(teams: &[GameTeam], seed: u32) -> Vec<GameTeam> {
    let mut slots: HashMap<(usize, usize), Vec<(usize, usize)>> = HashMap::new();
    for (ti, t) in teams.iter().enumerate() {
        let mut seen: HashMap<usize, usize> = HashMap::new();
        for (pi, p) in t.players.iter().enumerate() {
            let k = seen.entry(p.position.index()).or_default();
            slots.entry((p.position.index(), *k)).or_default().push((ti, pi));
            *k += 1;
        }
    }
    let mut out = teams.to_vec();
    let mut rng = Rng::new(seed ^ 0x5A0F_F1E5);
    let mut keys: Vec<_> = slots.keys().copied().collect();
    keys.sort_unstable();
    for k in keys {
        let v = &slots[&k];
        let mut order: Vec<usize> = (0..v.len()).collect();
        for i in (1..order.len()).rev() {
            let j = rng.int(0, i as i32) as usize;
            order.swap(i, j);
        }
        for (dst, &src) in v.iter().zip(&order) {
            out[dst.0].players[dst.1] = teams[v[src].0].players[v[src].1].clone();
        }
    }
    out
}

/// Reads whose result needs the play's context in the model.
fn has_context(r: Read) -> bool {
    matches!(r, Read::Picks | Read::Separation | Read::Release | Read::ShortKick | Read::LongKick | Read::Placement)
}

fn vector(p: &GamePlayer) -> [f32; NV] {
    let mut v = [0.0f32; NV];
    for (k, x) in v.iter_mut().take(ATTRIBUTE_COUNT).enumerate() {
        *x = p.attrs.at(k) as f32;
    }
    for (k, x) in results::own(p).iter().enumerate() {
        v[ATTRIBUTE_COUNT + k] = *x as f32;
    }
    v
}

/// The features of a trial: his vector, the opponent, the context.
fn feats(r: &Row, vecs: &[[f32; NV]], ctx: bool, out: &mut Vec<f64>) {
    out.clear();
    out.extend(vecs[r.v as usize].iter().map(|&x| x as f64));
    out.push(r.z as f64);
    if ctx {
        out.push(r.cx as f64);
    }
}

/// Least squares with an intercept on the centred features. Returns the
/// intercept (uncentred), the weights, the residual spread and the
/// weights' standard errors.
fn ols(rows: &[Row], vecs: &[[f32; NV]], ctx: bool) -> (f64, Vec<f64>, f64, Vec<f64>) {
    let k = NV + 1 + ctx as usize;
    let mut f = Vec::with_capacity(k);
    let mut m = vec![0.0; k];
    let mut my = 0.0;
    for r in rows {
        feats(r, vecs, ctx, &mut f);
        for i in 0..k {
            m[i] += f[i];
        }
        my += r.y as f64;
    }
    let n = rows.len() as f64;
    m.iter_mut().for_each(|x| *x /= n);
    my /= n;
    let (xtx, xty) = rows
        .par_chunks(4096)
        .map(|ch| {
            let mut xtx = vec![0.0; k * k];
            let mut xty = vec![0.0; k];
            let mut f = Vec::with_capacity(k);
            for r in ch {
                feats(r, vecs, ctx, &mut f);
                for i in 0..k {
                    f[i] -= m[i];
                }
                let y = r.y as f64 - my;
                for i in 0..k {
                    xty[i] += f[i] * y;
                    let fi = f[i];
                    let row = &mut xtx[i * k..i * k + k];
                    for j in 0..k {
                        row[j] += fi * f[j];
                    }
                }
            }
            (xtx, xty)
        })
        .reduce(
            || (vec![0.0; k * k], vec![0.0; k]),
            |mut a, b| {
                a.0.iter_mut().zip(&b.0).for_each(|(x, y)| *x += y);
                a.1.iter_mut().zip(&b.1).for_each(|(x, y)| *x += y);
                a
            },
        );
    let inv = invert((0..k).map(|i| xtx[i * k..i * k + k].to_vec()).collect());
    let w: Vec<f64> = (0..k).map(|i| (0..k).map(|j| inv[i][j] * xty[j]).sum()).collect();
    let a = my - (0..k).map(|i| w[i] * m[i]).sum::<f64>();
    let sse: f64 = rows
        .par_iter()
        .map_init(Vec::new, |f, r| {
            feats(r, vecs, ctx, f);
            let p: f64 = a + (0..k).map(|i| w[i] * f[i]).sum::<f64>();
            (r.y as f64 - p).powi(2)
        })
        .sum();
    let s2 = sse / (n - k as f64 - 1.0).max(1.0);
    let se: Vec<f64> = (0..k).map(|i| (s2 * inv[i][i]).max(0.0).sqrt()).collect();
    (a, w, s2.sqrt(), se)
}

/// Gauss-Jordan inverse.
#[allow(clippy::needless_range_loop)]
fn invert(mut a: Vec<Vec<f64>>) -> Vec<Vec<f64>> {
    let n = a.len();
    let mut inv: Vec<Vec<f64>> = (0..n).map(|i| (0..n).map(|j| (i == j) as u8 as f64).collect()).collect();
    for c in 0..n {
        let p = (c..n).max_by(|&x, &y| a[x][c].abs().total_cmp(&a[y][c].abs())).unwrap();
        a.swap(c, p);
        inv.swap(c, p);
        let d = a[c][c];
        if d.abs() < 1e-12 {
            continue;
        }
        for j in 0..n {
            a[c][j] /= d;
            inv[c][j] /= d;
        }
        for r in 0..n {
            if r != c {
                let f = a[r][c];
                if f == 0.0 {
                    continue;
                }
                for j in 0..n {
                    a[r][j] -= f * a[c][j];
                    inv[r][j] -= f * inv[c][j];
                }
            }
        }
    }
    inv
}

fn mean(v: impl Iterator<Item = f64>) -> f64 {
    let (s, n) = v.fold((0.0, 0usize), |(s, n), x| (s + x, n + 1));
    if n == 0 { 0.0 } else { s / n as f64 }
}

type Game = (Vec<[f32; NV]>, HashMap<(usize, usize), Vec<Row>>, Vec<(u32, u32)>);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2000);
    let seed: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let out = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/../afm-engine/data/results_fit.csv").to_string());
    let mut csv = String::new();
    let _ = write!(csv, "level,read,pos,a,b,d,e,c0,sd,season,tau,tp,sk");
    for k in 0..NV {
        let _ = write!(csv, ",w{k}");
    }
    csv.push('\n');
    let mut team_rows = Vec::new();
    for (li, level) in [Level::HS, Level::FBS, Level::FCS, Level::SmallCollege, Level::NFL].into_iter().enumerate() {
        let teams = synthetic_league(level, 32, seed);
        let st: Vec<f64> = teams.iter().map(results::strength).collect();
        let sm = mean(st.iter().copied());
        let ss = (mean(st.iter().map(|s| (s - sm).powi(2)))).sqrt();
        team_rows.push((sm, ss));
        // The second half of the games: every man on another team.
        let moved = shuffled(&teams, seed);
        let st2: Vec<f64> = moved.iter().map(results::strength).collect();
        let orig: HashMap<u32, &GamePlayer> = teams.iter().flat_map(|t| t.players.iter().map(|p| (p.id, p))).collect();
        let per: Vec<Game> = (0..games)
            .into_par_iter()
            .map(|g| {
                let k = g / 2;
                let h = (k * 7) % 32;
                let a = (k * 13 + 5) % 32;
                let a = if a == h { (a + 1) % 32 } else { a };
                let (h, a) = if g.is_multiple_of(2) { (h, a) } else { (a, h) };
                let s = seed as u64 * 1_000_003 + g as u64 + 77;
                let late = g >= games / 2;
                let (lg, stl) = if late { (&moved, &st2) } else { (&teams, &st) };
                // Every man's ratings moved at random for this game (as the
                // A/B tests move one rating), so each one's slope shows.
                let mut rng = Rng::new((s as u32) ^ 0x5EED_0F17);
                let mut sides = [lg[h].clone(), lg[a].clone()];
                for t in sides.iter_mut() {
                    for p in t.players.iter_mut() {
                        for k in 0..ATTRIBUTE_COUNT {
                            let v = (p.attrs.at(k) + rng.normal(0.0, SHIFT)).clamp(1.0, 99.0);
                            p.attrs.set_at(k, v);
                        }
                    }
                }
                let r = quick_game_in(&sides[0], &sides[1], level, s, Default::default(), Default::default());
                let z = [(stl[h] - sm) / ss, (stl[a] - sm) / ss];
                let mut vecs = Vec::new();
                let mut who = HashMap::new();
                for (side, t) in sides.iter().enumerate() {
                    for p in &t.players {
                        who.insert(p.id, (side, p, vecs.len() as u32));
                        vecs.push(vector(p));
                    }
                }
                let mut rows: HashMap<(usize, usize), Vec<Row>> = HashMap::new();
                let mut buf = Vec::new();
                for p in &r.stats.plays {
                    buf.clear();
                    results::trials(p, level, &mut buf);
                    for t in &buf {
                        let Some(&(side, gp, v)) = who.get(&t.id) else { continue };
                        if !t.read.reads(gp.position) {
                            continue;
                        }
                        rows.entry((t.read.index(), gp.position.index())).or_default().push(Row {
                            id: t.id,
                            y: t.y,
                            z: z[1 - side] as f32,
                            cx: t.cx,
                            v,
                            late,
                        });
                    }
                }
                let snaps: Vec<(u32, u32)> = r
                    .stats
                    .players
                    .iter()
                    .flat_map(|m| m.iter().map(|(&id, l)| (id, l.snaps_off + l.snaps_def)))
                    .collect();
                (vecs, rows, snaps)
            })
            .collect();
        let mut vecs: Vec<[f32; NV]> = Vec::new();
        let mut rows: HashMap<(usize, usize), Vec<Row>> = HashMap::new();
        let mut snaps: HashMap<u32, (u32, u32)> = HashMap::new();
        for (vs, rs, sn) in per {
            let off = vecs.len() as u32;
            vecs.extend(vs);
            for (k, r) in rs {
                rows.entry(k).or_default().extend(r.into_iter().map(|mut x| {
                    x.v += off;
                    x
                }));
            }
            for (id, s) in sn {
                let e = snaps.entry(id).or_default();
                e.0 += s;
                e.1 += 1;
            }
        }
        let season_snaps = results::season_snaps(level);
        let season_games = afm_engine::development::season_games(level) as f64;
        println!("\n{level:?}: {games} games; team strength {sm:.2} (sd {ss:.2})");
        println!(
            "  {:<22} {:<4} {:>7} {:>8} {:>9} {:>6} {:>7} {:>7} {:>7} {:>7} {:>5} {:>6} {:>4}",
            "read", "pos", "trials", "mean", "b/pt", "t(b)", "sd", "season", "tau pt", "his pt", "sk", "keeps", "on"
        );
        for read in Read::ALL {
            for pos in Position::ALL {
                if !read.reads(pos) {
                    continue;
                }
                let Some(rs) = rows.get(&(read.index(), pos.index())) else { continue };
                if rs.len() < 1000 {
                    println!("  {:<22} {:<4} {:>7} too few", read.name(), pos.name(), rs.len());
                    continue;
                }
                let ctx = has_context(read);
                let (a, w, sd, se) = ols(rs, &vecs, ctx);
                let own = read.attr().index();
                let b = w[own];
                let t = if se[own] > 0.0 { b / se[own] } else { 0.0 };
                let d = w[NV];
                let e = if ctx { w[NV + 1] } else { 0.0 };
                let c0 = mean(rs.iter().map(|r| r.cx as f64));
                // Volume: trials per snap of the men who played most of
                // their team's snaps, times a starter's season (kickers
                // and punters: trials per game they kicked in). Their
                // skill as it stood, for its spread among regulars.
                let mut by: HashMap<u32, usize> = HashMap::new();
                for r in rs {
                    *by.entry(r.id).or_default() += 1;
                }
                let specialist = matches!(read, Read::ShortKick | Read::LongKick | Read::Placement);
                let (mut tn, mut sn) = (0.0, 0.0);
                let mut skill = Vec::new();
                for (id, n) in &by {
                    let (s, g) = snaps.get(id).copied().unwrap_or((0, 0));
                    let regular = if specialist {
                        g as f64 >= 0.25 * games as f64 / 16.0
                    } else {
                        g > 0 && s as f64 / g as f64 >= 0.6 * season_snaps / season_games
                    };
                    if !regular {
                        continue;
                    }
                    tn += *n as f64;
                    sn += if specialist { g as f64 } else { s as f64 };
                    skill.push(orig[id].a(read.attr()));
                }
                let season =
                    if sn > 0.0 { tn / sn * if specialist { season_games } else { season_snaps } } else { 0.0 };
                let sk = if skill.len() >= 2 {
                    let m = mean(skill.iter().copied());
                    (skill.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (skill.len() - 1) as f64).sqrt()
                } else {
                    0.0
                };
                // What the model leaves in a man's mean result, beyond the
                // trials' own noise, in each half; what follows him to
                // another team is what his two halves share.
                let mut res: HashMap<(u32, bool), (f64, f64)> = HashMap::new();
                let mut f = Vec::new();
                for r in rs {
                    feats(r, &vecs, ctx, &mut f);
                    let p = a + (0..f.len()).map(|i| w[i] * f[i]).sum::<f64>();
                    let q = res.entry((r.id, r.late)).or_default();
                    q.0 += r.y as f64 - p;
                    q.1 += 1.0;
                }
                let (mut tau2, mut m) = (0.0, 0.0);
                for (s, n) in res.values() {
                    if *n >= 20.0 {
                        tau2 += (s / n).powi(2) - sd * sd / n;
                        m += 1.0;
                    }
                }
                let tau = if m > 0.0 { (tau2 / m).max(0.0).sqrt() } else { 0.0 };
                let (mut cv, mut mc) = (0.0, 0.0);
                for (&(id, late), &(s1, n1)) in &res {
                    if late || n1 < 20.0 {
                        continue;
                    }
                    if let Some(&(s2, n2)) = res.get(&(id, true))
                        && n2 >= 20.0
                    {
                        cv += (s1 / n1) * (s2 / n2);
                        mc += 1.0;
                    }
                }
                let tp = if mc > 0.0 { (cv / mc).max(0.0).sqrt().min(tau) } else { 0.0 };
                let on = b > 0.0 && t >= 4.0 && season > 0.0 && sk > 0.0;
                let mut fit = Fit { a, b, d, e, c0, sd, season, tau, tp, sk, ..Fit::OFF };
                for (k, &v) in w.iter().enumerate().take(NV) {
                    fit.set_w(k, v);
                }
                println!(
                    "  {:<22} {:<4} {:>7} {:>8.4} {:>9.5} {:>6.1} {:>7.3} {:>7.1} {:>7.2} {:>7.2} {:>5.1} {:>6.2} {:>4}",
                    read.name(),
                    pos.name(),
                    rs.len(),
                    mean(rs.iter().map(|r| r.y as f64)),
                    b,
                    t,
                    sd,
                    season,
                    if b > 0.0 { tau / b } else { 0.0 },
                    if b > 0.0 { tp / b } else { 0.0 },
                    sk,
                    if on { fit.weight(season) } else { 0.0 },
                    if on { "yes" } else { "off" }
                );
                if on {
                    let _ = write!(
                        csv,
                        "{li},{read:?},{},{a:.6},{b:.6},{d:.6},{e:.6},{c0:.3},{sd:.5},{season:.1},{tau:.6},{tp:.6},{sk:.3}",
                        pos.name()
                    );
                    for x in w.iter().take(NV) {
                        let _ = write!(csv, ",{x:.4e}");
                    }
                    csv.push('\n');
                }
            }
        }
    }
    std::fs::write(&out, &csv).expect("write the table");
    eprintln!("wrote {out}");
    println!("\n// --- results/table.rs TEAM ---");
    println!("pub const TEAM: [(f64, f64); 5] = [");
    for (m, s) in team_rows {
        println!("    ({m:.3}, {s:.3}),");
    }
    println!("];");
}
