//! What each rating is worth in each scheme, job by job (scheme-fit item 24,
//! lane SF11): the measure behind the world's fit scores
//! (`crates/afm-engine/data/fit_values.csv`, read by `world::systems`).
//!
//! Team 0 of the league is the level's typical team (`base_team`), as in
//! every arm. On each game it runs one offense, one front and one coverage
//! family (all 40 combinations in turn), and every watched man on it has
//! each watched rating moved at random about his own, up to `SPREAD` points
//! either way, and his weight up to `WEIGHT_SPREAD` of it. Games come in
//! pairs: the same game again (seed, opponent, field, systems) with every
//! move turned the other way. A rating's worth in a system is then the
//! slope of the points team 0 scores (its offense's men, in that offense)
//! or keeps off the board (its defense's men, in that front or coverage
//! family) on the man's change, from one least-squares fit of the pairs'
//! half differences over the system's games: the opponent, the field and
//! the luck a pair shares drop out (a pair's points correlate about 0.3).
//! Every rating is moved on every game, so one run prices them all, each
//! with its noise.
//!
//! The fit scores read how a rating's worth differs between the systems
//! of a kind, so a system's own weight is pooled with the kind's unless
//! they differ past their noise (a chi-square test at 5%), and then pulled
//! toward the kind's by its noise (empirical Bayes, DerSimonian-Laird): a
//! rating no system pays differently reads the same in each, and one that
//! no system pays past its noise reads nothing.
//!
//! `fit_check values <fbs|nfl> [games] [seed] [out]` plays the games into
//! `<out>/rows.f32` (kept, so a second call fits again without playing),
//! then writes `<out>/fit_values.csv` (the table) and `<out>/raw.csv`
//! (every system's own slope and its noise, before pooling). Copy the
//! table to `crates/afm-engine/data/fit_values.csv`.

use std::path::Path;
use std::time::Instant;

use afm_engine::attributes::Attr;
use afm_engine::game::guide::Guidelines;
use afm_engine::game::roster::{Caller, GameTeam};
use afm_engine::game::{Conditions, Venue, quick_game_in};
use afm_engine::player::Level;
use afm_engine::world::systems::{FRONTS, OFFENSES, SHELLS};
use rayon::prelude::*;

use super::{Base, base_team, game_seed, league, role};
use crate::ab::{Knob, Part, Squad, change_one};

/// The men whose ratings move: one per job the world's fit reads (the
/// receivers' three jobs, both guards, both edges, both inside men and
/// both outside corners each become one job in the world).
pub const MOVED: &[&str] = &[
    "QB1", "RB1", "FB", "X", "Z", "SLOT", "TE1", "LT", "LG", "C", "RG", "RT", "EDGE1", "EDGE2", "DT1", "DT2", "SAM",
    "MIKE", "WILL", "CB1", "CB2", "NCB", "FS", "SS",
];

/// A feature: a rating's index, or his weight.
pub const WEIGHT: u8 = u8::MAX;

/// How far a game moves each rating (up to this many points either way,
/// about the spread of a level's starters and backups, as lane SF10's
/// roster table) and his weight (this share of it either way).
pub const SPREAD: f64 = 18.0;
pub const WEIGHT_SPREAD: f64 = 0.08;

pub fn feature_name(f: u8) -> String {
    if f == WEIGHT { "Weight".to_string() } else { format!("{:?}", Attr::ALL[f as usize]) }
}

/// The ratings watched at each job: the ones the engine reads for his work
/// there (lane SF10's list, with Football IQ for the busts and the calls'
/// communication), the ones the world's fit weighed before (so a rating
/// the old scores counted is priced too), and his weight.
pub fn watched(role: &str) -> Vec<u8> {
    use Attr as A;
    let v: &[Attr] = match role {
        "QB1" => &[
            A::ArmStrength,
            A::ShortAccuracy,
            A::MediumAccuracy,
            A::DeepAccuracy,
            A::Touch,
            A::ThrowOnMove,
            A::QuickRelease,
            A::Footwork,
            A::PocketPresence,
            A::Decisions,
            A::Anticipation,
            A::EyeManipulation,
            A::FootballIq,
            A::BallHandling,
            A::Speed,
            A::Acceleration,
            A::Agility,
            A::Elusiveness,
            A::BallCarrierVision,
        ],
        "RB1" => &[
            A::Speed,
            A::Acceleration,
            A::Agility,
            A::Elusiveness,
            A::BallCarrierVision,
            A::LegDrive,
            A::Strength,
            A::Balance,
            A::Traffic,
            A::BallSecurity,
            A::Catching,
            A::RouteRunning,
            A::PassBlocking,
        ],
        "FB" => &[A::RunBlocking, A::BlockingInSpace, A::Strength, A::HandUse, A::LegDrive, A::Catching],
        "X" | "Z" | "SLOT" => &[
            A::Speed,
            A::Acceleration,
            A::Agility,
            A::RouteRunning,
            A::Release,
            A::Catching,
            A::ContestedCatch,
            A::BallTracking,
            A::BodyControl,
            A::Elusiveness,
            A::BlockingInSpace,
            A::FootballIq,
            A::Strength,
        ],
        "TE1" => &[
            A::Speed,
            A::Agility,
            A::RouteRunning,
            A::Release,
            A::Catching,
            A::ContestedCatch,
            A::RunBlocking,
            A::PassBlocking,
            A::BlockingInSpace,
            A::HandUse,
            A::Anchor,
            A::Strength,
        ],
        "LT" | "LG" | "C" | "RG" | "RT" => &[
            A::PassBlocking,
            A::RunBlocking,
            A::HandUse,
            A::Anchor,
            A::Strength,
            A::Agility,
            A::Flexibility,
            A::Speed,
            A::BlockingInSpace,
        ],
        "EDGE1" | "EDGE2" => &[
            A::PowerRush,
            A::FinesseRush,
            A::BlockShedding,
            A::HandUse,
            A::Flexibility,
            A::Anchor,
            A::Speed,
            A::Acceleration,
            A::Strength,
            A::Pursuit,
            A::Tackling,
            A::ZoneCoverage,
        ],
        "DT1" | "DT2" => &[
            A::PowerRush,
            A::FinesseRush,
            A::BlockShedding,
            A::HandUse,
            A::Anchor,
            A::Strength,
            A::Acceleration,
            A::Agility,
            A::Anticipation,
            A::Pursuit,
            A::Tackling,
        ],
        "SAM" | "MIKE" | "WILL" => &[
            A::Tackling,
            A::Pursuit,
            A::PlayRecognition,
            A::ZoneCoverage,
            A::ManCoverage,
            A::Speed,
            A::BlockShedding,
            A::Traffic,
            A::Strength,
            A::Blitzing,
            A::Anticipation,
        ],
        "CB1" | "CB2" | "NCB" => &[
            A::ManCoverage,
            A::ZoneCoverage,
            A::Press,
            A::Speed,
            A::Acceleration,
            A::Agility,
            A::BallTracking,
            A::Anticipation,
            A::PlayRecognition,
            A::Tackling,
            A::Strength,
        ],
        "FS" | "SS" => &[
            A::ZoneCoverage,
            A::ManCoverage,
            A::Speed,
            A::FootballIq,
            A::Anticipation,
            A::BallTracking,
            A::Tackling,
            A::Pursuit,
            A::PlayRecognition,
            A::BlockShedding,
        ],
        _ => &[],
    };
    let mut v = v.to_vec();
    // Every man busts by what he knows of the call (`scrimmage`, busts):
    // Football IQ at every job. The calls (communication): the passer's,
    // the center's for the protection, and the defense's green dot among
    // its linebackers and safeties; and the center's shotgun snap.
    let mut extra = vec![A::FootballIq];
    if matches!(role, "QB1" | "C" | "SAM" | "MIKE" | "WILL" | "FS" | "SS") {
        extra.push(A::Communication);
    }
    if role == "C" {
        extra.push(A::ShotgunSnap);
    }
    for a in extra {
        if !v.contains(&a) {
            v.push(a);
        }
    }
    let mut f: Vec<u8> = v.iter().map(|&a| a.index() as u8).collect();
    f.push(WEIGHT);
    f
}

fn offense_role(r: &str) -> bool {
    MOVED.iter().position(|x| *x == r).is_some_and(|k| k < MOVED.iter().position(|x| *x == "EDGE1").unwrap())
}

/// The watched pairs (role, feature) on one side, in table order.
pub fn pairs(offense: bool) -> Vec<(&'static str, u8)> {
    MOVED
        .iter()
        .filter(|r| offense_role(r) == offense)
        .flat_map(|&r| watched(r).into_iter().map(move |f| (r, f)))
        .collect()
}

/// Game `g`'s systems for team 0 (slots into `OFFENSES`, `FRONTS`,
/// `SHELLS`): every combination once in each 40 games.
pub fn systems_of(g: usize) -> (usize, usize, usize) {
    (g % 5, (g / 5) % 2, (g / 10) % 4)
}

/// A small stream of its own for each game's moves (the game keeps the
/// engine's).
struct Draw(u64);

impl Draw {
    fn u(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Team 0 for game `g`: its systems, a typical staff (the league's middle
/// play callers, no guidelines, so each system plays its own mix), and
/// every watched man moved. Returns the team and each pair's change, the
/// offense's pairs first.
pub fn moved_team(base: &[Squad], b: &Base, level: Level, seed: u32, g: usize, sign: f64) -> (GameTeam, Vec<f64>) {
    let mut t = b.team.clone();
    let (o, f, s) = systems_of(g);
    t.staff.off_scheme = OFFENSES[o];
    t.staff.front = FRONTS[f];
    t.staff.shell = SHELLS[s];
    t.staff.guide = Guidelines::default();
    t.staff.offense = Caller::average();
    t.staff.defense = Caller::average();
    let mut r = Draw(game_seed(seed, g) ^ 0x5F11_7A1E_0000_0000);
    let mut dev = Vec::new();
    for offense in [true, false] {
        for &rname in MOVED.iter().filter(|r| offense_role(r) == offense) {
            let k = b.at[role(rname)];
            let feats = watched(rname);
            let before = t.players[k].clone();
            let knobs: Vec<Knob> = feats
                .iter()
                .map(|&f| {
                    if f == WEIGHT {
                        Knob::Set(Part::Weight, before.weight * (1.0 + sign * WEIGHT_SPREAD * (2.0 * r.u() - 1.0)))
                    } else {
                        Knob::Rating(Attr::ALL[f as usize], sign * SPREAD * (2.0 * r.u() - 1.0))
                    }
                })
                .collect();
            change_one(&mut t.players[k], &base[0].people[k], &knobs, level);
            let gp = &t.players[k];
            for &f in &feats {
                dev.push(if f == WEIGHT {
                    gp.weight - before.weight
                } else {
                    let a = Attr::ALL[f as usize];
                    gp.attrs.get(a) - before.attrs.get(a)
                });
            }
        }
    }
    (t, dev)
}

/// One game's row: team 0's points for and against, the opponent and
/// whether team 0 was at home, and every watched pair's change.
fn play(base: &[Squad], b: &Base, level: Level, seed: u32, g: usize) -> Vec<f32> {
    // Games come in pairs: the same game (seed, opponent, field, systems)
    // with every move turned the other way.
    let k = g / 2;
    let sign = if g.is_multiple_of(2) { 1.0 } else { -1.0 };
    let (t0, dev) = moved_team(base, b, level, seed, k, sign);
    let n = base.len();
    let opp = 1 + (k / 2) % (n - 1);
    let home = k.is_multiple_of(2);
    let (h, a) = if home { (&t0, &base[opp].team) } else { (&base[opp].team, &t0) };
    let venue =
        Venue { cond: Conditions { crowd: 0.65, ..Conditions::default() }, wear: [0.0, 0.012], ..Venue::default() };
    let res = quick_game_in(h, a, level, game_seed(seed, k), Default::default(), venue);
    let s = res.stats.score;
    let (pf, pa) = if home { (s[0], s[1]) } else { (s[1], s[0]) };
    let mut row = vec![pf as f32, pa as f32, opp as f32, home as u8 as f32];
    row.extend(dev.iter().map(|&x| x as f32));
    row
}

/// The leading columns of a row before the pairs.
const HEAD: usize = 4;

/// Plays the games into `out/rows.f32` (kept if there) and fits the table.
pub fn run(level: Level, games: usize, seed: u32, out: &Path) -> Result<(), String> {
    if games < 2 || !games.is_multiple_of(2) {
        return Err(format!("games come in pairs: an even count, not {games}"));
    }
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let (off, def) = (pairs(true), pairs(false));
    let width = HEAD + off.len() + def.len();
    let path = out.join("rows.f32");
    let schema = format!("{} games {} seed {} width {}", super::level_name(level), games, seed, width);
    let sp = out.join("schema.txt");
    let rows: Vec<f32> = if path.exists() {
        let old = std::fs::read_to_string(&sp).unwrap_or_default();
        if old.trim() != schema {
            return Err(format!("{} holds another run ({old}): use a new folder", out.display()));
        }
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        bytes.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect()
    } else {
        let (base, _) = league(level, seed);
        let b = base_team(&base[0], level);
        let t0 = Instant::now();
        eprintln!(
            "{} games, {} seed {seed}: {} offense and {} defense pairs",
            games,
            super::level_name(level),
            off.len(),
            def.len()
        );
        let done = std::sync::atomic::AtomicUsize::new(0);
        let rows: Vec<Vec<f32>> = (0..games)
            .into_par_iter()
            .map(|g| {
                let r = play(&base, &b, level, seed, g);
                let d = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                if d.is_multiple_of(5_000) {
                    let el = t0.elapsed().as_secs_f64();
                    eprintln!("{d} games [{:.0} s, eta {:.0} min]", el, el / d as f64 * (games - d) as f64 / 60.0);
                }
                r
            })
            .collect();
        eprintln!("played [{:.0} s]", t0.elapsed().as_secs_f64());
        let flat: Vec<f32> = rows.into_iter().flatten().collect();
        let bytes: Vec<u8> = flat.iter().flat_map(|x| x.to_le_bytes()).collect();
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        std::fs::write(&sp, &schema).map_err(|e| e.to_string())?;
        flat
    };
    let n = rows.len() / width;
    if n * width != rows.len() || n < 2 {
        return Err("rows.f32 is damaged".to_string());
    }
    let row = |g: usize| &rows[g * width..(g + 1) * width];
    // Each pair's half difference: the same game with the moves turned
    // the other way, so the opponent, the field and the game's own luck
    // they share drop out, and so does any curve in a rating's worth.
    let half = |k: usize, c: usize| (row(2 * k)[c] as f64 - row(2 * k + 1)[c] as f64) / 2.0;
    let pairs_n = n / 2;
    let mut table = vec!["system,role,feature,weight".to_string()];
    let mut raw = vec!["system,role,feature,slope,se,points,games".to_string()];
    // Each kind: its systems, which side's pairs, the system of a game.
    let kinds: [(&str, usize, bool); 3] = [("offense", 5, true), ("front", 2, false), ("shell", 4, false)];
    for (kind, count, offense) in kinds {
        let pp = if offense { &off } else { &def };
        let at = if offense { HEAD } else { HEAD + off.len() };
        let mut fits = Vec::new();
        for sys in 0..count {
            let games: Vec<usize> = (0..pairs_n)
                .filter(|&k| {
                    let (o, f, s) = systems_of(k);
                    match kind {
                        "offense" => o == sys,
                        "front" => f == sys,
                        _ => s == sys,
                    }
                })
                .collect();
            // Points scored for the offense's men, points kept off the
            // board for the defense's.
            let y: Vec<f64> = games.iter().map(|&k| if offense { half(k, 0) } else { -half(k, 1) }).collect();
            let x: Vec<Vec<f64>> = games.iter().map(|&k| (at..at + pp.len()).map(|c| half(k, c)).collect()).collect();
            let (b, se) = within_fit(&x, &y, &vec![0; games.len()]);
            let mean = games
                .iter()
                .map(
                    |&k| if offense { row(2 * k)[0] + row(2 * k + 1)[0] } else { -(row(2 * k)[1] + row(2 * k + 1)[1]) },
                )
                .map(|v| v as f64 / 2.0)
                .sum::<f64>()
                / games.len() as f64;
            for (j, &(r, f)) in pp.iter().enumerate() {
                raw.push(format!(
                    "{},{r},{},{:.6},{:.6},{:.3},{}",
                    sys_name(kind, sys),
                    feature_name(f),
                    b[j],
                    se[j],
                    mean,
                    games.len()
                ));
            }
            eprintln!("{} {}: {} pairs of games, {:.2} points a game", kind, sys_name(kind, sys), games.len(), mean);
            fits.push((b, se));
        }
        let post = pool(&fits);
        for (sys, w) in post.iter().enumerate() {
            for (j, &(r, f)) in pp.iter().enumerate() {
                table.push(format!("{},{r},{},{:.6}", sys_name(kind, sys), feature_name(f), w[j]));
            }
        }
    }
    std::fs::write(out.join("fit_values.csv"), table.join("\n") + "\n").map_err(|e| e.to_string())?;
    std::fs::write(out.join("raw.csv"), raw.join("\n") + "\n").map_err(|e| e.to_string())?;
    eprintln!("wrote {} and {}", out.join("fit_values.csv").display(), out.join("raw.csv").display());
    Ok(())
}

pub fn sys_name(kind: &str, s: usize) -> String {
    match kind {
        "offense" => format!("{:?}", OFFENSES[s]),
        "front" => format!("{:?}", FRONTS[s]),
        _ => format!("{:?}", SHELLS[s]),
    }
}

/// Least squares of `y` on `x` with each group's mean taken out of both
/// (the opponent and the field as fixed effects): the slopes and their
/// standard errors.
pub fn within_fit(x: &[Vec<f64>], y: &[f64], groups: &[usize]) -> (Vec<f64>, Vec<f64>) {
    let p = x[0].len();
    let n = y.len();
    let ng = groups.iter().copied().max().unwrap_or(0) + 1;
    let mut cnt = vec![0.0; ng];
    let mut my = vec![0.0; ng];
    let mut mx = vec![vec![0.0; p]; ng];
    for i in 0..n {
        let g = groups[i];
        cnt[g] += 1.0;
        my[g] += y[i];
        for (a, v) in mx[g].iter_mut().zip(&x[i]) {
            *a += v;
        }
    }
    for g in 0..ng {
        if cnt[g] > 0.0 {
            my[g] /= cnt[g];
            for v in mx[g].iter_mut() {
                *v /= cnt[g];
            }
        }
    }
    let used = cnt.iter().filter(|&&c| c > 0.0).count();
    let mut xtx = vec![0.0; p * p];
    let mut xty = vec![0.0; p];
    let mut yy = 0.0;
    let mut xr = vec![0.0; p];
    for i in 0..n {
        let g = groups[i];
        for k in 0..p {
            xr[k] = x[i][k] - mx[g][k];
        }
        let yr = y[i] - my[g];
        for a in 0..p {
            let xa = xr[a];
            for b2 in a..p {
                xtx[a * p + b2] += xa * xr[b2];
            }
            xty[a] += xa * yr;
        }
        yy += yr * yr;
    }
    for a in 0..p {
        for b2 in 0..a {
            xtx[a * p + b2] = xtx[b2 * p + a];
        }
    }
    let inv = invert(&xtx, p);
    let beta: Vec<f64> = (0..p).map(|a| (0..p).map(|c| inv[a * p + c] * xty[c]).sum()).collect();
    let fit: f64 = (0..p).map(|a| beta[a] * xty[a]).sum();
    let s2 = ((yy - fit) / (n - p - used) as f64).max(0.0);
    let se = (0..p).map(|a| (s2 * inv[a * p + a]).sqrt()).collect();
    (beta, se)
}

/// Gauss-Jordan inverse of a symmetric positive matrix.
fn invert(m: &[f64], p: usize) -> Vec<f64> {
    let mut a = m.to_vec();
    let mut inv = vec![0.0; p * p];
    for i in 0..p {
        inv[i * p + i] = 1.0;
    }
    for col in 0..p {
        let piv = (col..p).max_by(|&x, &y| a[x * p + col].abs().total_cmp(&a[y * p + col].abs())).unwrap();
        if piv != col {
            for k in 0..p {
                a.swap(col * p + k, piv * p + k);
                inv.swap(col * p + k, piv * p + k);
            }
        }
        let d = a[col * p + col];
        for k in 0..p {
            a[col * p + k] /= d;
            inv[col * p + k] /= d;
        }
        for row in 0..p {
            if row != col {
                let f = a[row * p + col];
                if f != 0.0 {
                    for k in 0..p {
                        a[row * p + k] -= f * a[col * p + k];
                        inv[row * p + k] -= f * inv[col * p + k];
                    }
                }
            }
        }
    }
    inv
}

/// How far past its noise a shared weight must stand to be kept (standard
/// errors, as lane SF10's roster table).
pub const KEEP: f64 = 2.5;

/// Each system's weights after pooling with its kind: for each pair, the
/// kind's systems share one weight unless they differ past their noise (a
/// chi-square test at 5%), kept only past `KEEP` of its noise; when they
/// differ, each system's own is pulled toward the kind's by its noise
/// (DerSimonian-Laird).
pub fn pool(fits: &[(Vec<f64>, Vec<f64>)]) -> Vec<Vec<f64>> {
    let pairs = fits[0].0.len();
    let k = fits.len();
    let mut out = vec![vec![0.0; pairs]; k];
    for j in 0..pairs {
        let b: Vec<f64> = fits.iter().map(|f| f.0[j]).collect();
        let v: Vec<f64> = fits.iter().map(|f| f.1[j].powi(2).max(1e-12)).collect();
        let w: Vec<f64> = v.iter().map(|x| 1.0 / x).collect();
        let sw: f64 = w.iter().sum();
        let m = b.iter().zip(&w).map(|(b, w)| b * w).sum::<f64>() / sw;
        let q: f64 = b.iter().zip(&w).map(|(b, w)| w * (b - m).powi(2)).sum();
        let df = (k - 1) as f64;
        let tau2 = if q > chi2_95(df) {
            let sw2: f64 = w.iter().map(|w| w * w).sum();
            ((q - df) / (sw - sw2 / sw)).max(0.0)
        } else {
            0.0
        };
        let ws: Vec<f64> = v.iter().map(|v| 1.0 / (v + tau2)).collect();
        let sws: f64 = ws.iter().sum();
        let ms = b.iter().zip(&ws).map(|(b, w)| b * w).sum::<f64>() / sws;
        if tau2 == 0.0 {
            // One weight for the kind, kept only as far as it stands past
            // `KEEP` of its noise (a soft threshold): a rating no system
            // pays reads nothing.
            let se = (1.0 / sws).sqrt();
            let kept = ms.signum() * (ms.abs() - KEEP * se).max(0.0);
            for row in out.iter_mut() {
                row[j] = kept;
            }
            continue;
        }
        for s in 0..k {
            let pull = tau2 / (tau2 + v[s]);
            out[s][j] = ms + pull * (b[s] - ms);
        }
    }
    out
}

/// The chi-square's 95th percentile at `df` degrees of freedom
/// (Wilson-Hilferty).
fn chi2_95(df: f64) -> f64 {
    let z = 1.6449;
    let a = 2.0 / (9.0 * df);
    df * (1.0 - a + z * a.sqrt()).powi(3)
}
