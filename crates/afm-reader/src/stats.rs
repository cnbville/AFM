//! Distributions: count, mean, spread and percentiles of a set of numbers,
//! and the percentile a value has within them.

#[derive(Clone, Debug, Default)]
pub struct Dist {
    pub n: usize,
    pub mean: f64,
    pub sd: f64,
    pub min: f64,
    pub max: f64,
    /// p1, p5, p10, p25, p50, p75, p90, p95, p99
    pub p: [f64; 9],
}

pub const PCTS: [f64; 9] = [0.01, 0.05, 0.10, 0.25, 0.50, 0.75, 0.90, 0.95, 0.99];
pub const PCT_NAMES: [&str; 9] = ["p1", "p5", "p10", "p25", "p50", "p75", "p90", "p95", "p99"];

pub fn dist(xs: &mut [f64]) -> Dist {
    let n = xs.len();
    if n == 0 {
        return Dist::default();
    }
    xs.sort_by(|a, b| a.total_cmp(b));
    let mean = xs.iter().sum::<f64>() / n as f64;
    let var = if n > 1 { xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1) as f64 } else { 0.0 };
    let mut p = [0.0; 9];
    for (i, q) in PCTS.iter().enumerate() {
        p[i] = quantile(xs, *q);
    }
    Dist { n, mean, sd: var.sqrt(), min: xs[0], max: xs[n - 1], p }
}

/// A sorted list's quantile, linear between points.
pub fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let pos = q * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f64)
}

/// The share of a sorted list below `x` (ties count half), 0 to 1.
pub fn rank(sorted: &[f64], x: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let below = sorted.partition_point(|v| *v < x);
    let upto = sorted.partition_point(|v| *v <= x);
    (below as f64 + (upto - below) as f64 / 2.0) / sorted.len() as f64
}

/// Equal-width bins between min and max: (low, high, count).
pub fn histogram(sorted: &[f64], bins: usize) -> Vec<(f64, f64, usize)> {
    if sorted.is_empty() {
        return Vec::new();
    }
    let (lo, hi) = (sorted[0], sorted[sorted.len() - 1]);
    if hi <= lo {
        return vec![(lo, hi, sorted.len())];
    }
    let w = (hi - lo) / bins as f64;
    let mut out: Vec<(f64, f64, usize)> = (0..bins).map(|i| (lo + w * i as f64, lo + w * (i + 1) as f64, 0)).collect();
    for x in sorted {
        let i = (((x - lo) / w) as usize).min(bins - 1);
        out[i].2 += 1;
    }
    out
}
