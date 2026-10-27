//! Your staff's mock of the picks before yours: the public's grades, each
//! club's needs as anyone can read them off its depth chart, and the noise
//! of a draft nobody can call, run a few hundred times. It never looks at
//! another club's own board. What comes out is a chance, not a promise.

use std::collections::HashMap;

use afm_engine::positions::Position;
use afm_engine::world::DraftRun;
use afm_engine::world::World;

/// How many drafts the staff runs.
pub const RUNS: usize = 400;
/// The spread of the public's read of a player from one run to the next
/// (grade points), and of one club's taste for him on top.
const PLAYER_SD: f64 = 2.0;
const CLUB_SD: f64 = 2.2;

pub struct Mock {
    /// The candidates (people indices), best public grade first.
    pub cands: Vec<usize>,
    /// For each checkpoint slot (board index), the share of runs in which
    /// each candidate was still there when that slot came up.
    pub there: Vec<(usize, Vec<f64>)>,
    /// For each checkpoint, the share of runs in which at least one of the
    /// watched players was still there.
    pub any: Vec<f64>,
    /// For each slot from now on, how often each candidate went there.
    pub went: Vec<(usize, HashMap<usize, u32>)>,
}

/// A small fixed random stream: the same room gives the same mock.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
        ((self.next() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    fn normal(&mut self) -> f64 {
        let (a, b) = (self.unit(), self.unit());
        (-2.0 * a.ln()).sqrt() * (std::f64::consts::TAU * b).cos()
    }
}

/// A club's need at a position as the public reads it (its depth chart),
/// for any club in the league.
pub fn need(w: &World, club: usize, pos: Position) -> f64 {
    w.nfl.get(club).map_or(0.0, |t| afm_engine::career::need(t, pos))
}

/// The positions a club drafts for.
pub const DRAFT_POS: [Position; 11] = {
    use Position::*;
    [QB, RB, WR, TE, OT, IOL, EDGE, DT, LB, CB, S]
};

/// Runs the staff's mock from the pick on the clock up to the last
/// checkpoint. `me`: your club (its own slots go to the best player left:
/// what you'd do there is yours to decide). `watch`: players whose joint
/// chance (any one of them there) the room wants.
pub fn run(w: &World, run: &DraftRun, me: usize, checks: &[usize], watch: &[usize], seed: u64) -> Mock {
    let from = (run.next as usize).saturating_sub(1);
    let upto = checks.iter().copied().max().unwrap_or(from).min(run.board.len());
    let n = (upto.saturating_sub(from) * 2 + 40).min(run.pool.len());
    let cands: Vec<usize> = run.pool.iter().take(n).map(|x| x.0).collect();
    let grade: Vec<f64> = run.pool.iter().take(n).map(|x| x.1).collect();
    let pos: Vec<Position> = cands.iter().map(|&i| w.people[i].player.position).collect();
    // Each club's need per candidate in each round it picks, read once.
    let mut needs: HashMap<(usize, u32), Vec<f64>> = HashMap::new();
    for k in from..upto {
        let (club, round) = run.board[k];
        let rf = if round <= 2 { 0.5 } else { 1.0 };
        needs.entry((club, round)).or_insert_with(|| {
            pos.iter()
                .map(|&p| {
                    if club == me {
                        return 0.0;
                    }
                    match p {
                        // A kicker or a snapper goes only where he's needed, and late.
                        Position::K | Position::P | Position::LS => {
                            if round >= 4 && need(w, club, p) > 0.0 {
                                0.0
                            } else {
                                -20.0
                            }
                        }
                        _ => rf * need(w, club, p),
                    }
                })
                .collect()
        });
    }
    let mut there: Vec<(usize, Vec<f64>)> = checks.iter().map(|&c| (c, vec![0.0; n])).collect();
    let watched: Vec<usize> = (0..n).filter(|&j| watch.contains(&cands[j])).collect();
    let watch_below = watch.iter().any(|i| !cands.contains(i));
    let mut any = vec![0.0; checks.len()];
    let mut went: Vec<(usize, HashMap<usize, u32>)> = (from..upto).map(|k| (k, HashMap::new())).collect();
    let mut rng = Rng(seed ^ ((run.year as u64) << 32) ^ run.next as u64);
    let mut taken = vec![false; n];
    let mut base = vec![0.0; n];
    for _ in 0..RUNS {
        taken.iter_mut().for_each(|t| *t = false);
        for (b, g) in base.iter_mut().zip(&grade) {
            *b = g + PLAYER_SD * rng.normal();
        }
        for k in from..=upto {
            for (q, (c, left)) in there.iter_mut().enumerate() {
                if *c == k {
                    for (j, t) in taken.iter().enumerate() {
                        if !t {
                            left[j] += 1.0;
                        }
                    }
                    if watch_below || watched.iter().any(|&j| !taken[j]) {
                        any[q] += 1.0;
                    }
                }
            }
            if k >= upto {
                break;
            }
            let nd = &needs[&run.board[k]];
            let mut best: Option<(f64, usize)> = None;
            for j in 0..n {
                if taken[j] {
                    continue;
                }
                let v = base[j] + nd[j] + CLUB_SD * rng.normal();
                if best.is_none_or(|b| v > b.0) {
                    best = Some((v, j));
                }
            }
            let Some((_, j)) = best else { break };
            taken[j] = true;
            *went[k - from].1.entry(cands[j]).or_default() += 1;
        }
    }
    for (_, left) in there.iter_mut() {
        left.iter_mut().for_each(|x| *x /= RUNS as f64);
    }
    any.iter_mut().for_each(|x| *x /= RUNS as f64);
    Mock { cands, there, any, went }
}

impl Mock {
    /// The share of runs a player was still there at a checkpoint (players
    /// below the candidates are there for sure).
    pub fn there_at(&self, check: usize, i: usize) -> f64 {
        let Some((_, left)) = self.there.iter().find(|x| x.0 == check) else { return 1.0 };
        self.cands.iter().position(|&c| c == i).map_or(1.0, |j| left[j])
    }
    /// The share of runs with a watched player there at a checkpoint.
    pub fn any_at(&self, check: usize) -> f64 {
        self.there.iter().position(|x| x.0 == check).map_or(1.0, |q| self.any[q])
    }
    /// The staff's most likely pick at a slot and how often it came up.
    pub fn likely(&self, slot: usize) -> Option<(usize, f64)> {
        let (_, m) = self.went.iter().find(|x| x.0 == slot)?;
        let (&i, &c) = m.iter().max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))?;
        Some((i, c as f64 / RUNS as f64))
    }
}

#[cfg(test)]
mod tests {
    use super::Rng;

    #[test]
    fn the_stream_is_fixed_and_normal() {
        let mut a = Rng(7);
        let mut b = Rng(7);
        assert_eq!(a.next(), b.next());
        let mut r = Rng(11);
        let v: Vec<f64> = (0..20_000).map(|_| r.normal()).collect();
        let mean = v.iter().sum::<f64>() / v.len() as f64;
        let sd = (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / v.len() as f64).sqrt();
        assert!(mean.abs() < 0.03, "mean {mean}");
        assert!((sd - 1.0).abs() < 0.03, "sd {sd}");
    }
}
