//! The fitted table behind growth from results (`results`): what each
//! man produces on each read at each level and position, from the engine's
//! own plays with every man's ratings moved at random game by game and,
//! for the second half of the games, every man on another team
//! (`cargo run --release --bin results_fit -- 2000`, written to
//! `data/results_fit.csv`). A read left out is off there: its result
//! doesn't move with its rating in this engine (t under 4), so its results
//! move nothing. Refit after engine changes that move the reads.

use std::sync::LazyLock;

use super::{Fit, OWN, READS, Read};
use crate::attributes::ATTRIBUTE_COUNT;
use crate::positions::{POSITION_COUNT, Position};

const CSV: &str = include_str!("../../data/results_fit.csv");

/// Every level's, read's and position's fit (`index`).
pub static FIT: LazyLock<Vec<Fit>> = LazyLock::new(|| parse(CSV));

/// A fit's place in `FIT`: level (HS, FBS, FCS, small colleges, NFL), read, position.
pub fn index(level: usize, read: usize, pos: usize) -> usize {
    (level.min(4) * READS + read) * POSITION_COUNT + pos
}

fn parse(csv: &str) -> Vec<Fit> {
    let mut t = vec![Fit::OFF; 5 * READS * POSITION_COUNT];
    for line in csv.lines().skip(1).filter(|l| !l.trim().is_empty()) {
        let c: Vec<&str> = line.split(',').collect();
        let level: usize = c[0].parse().expect("results_fit.csv: level");
        let read = Read::ALL.iter().position(|r| format!("{r:?}") == c[1]).expect("results_fit.csv: read");
        let pos = Position::ALL.iter().position(|p| p.name() == c[2]).expect("results_fit.csv: position");
        let n: Vec<f64> = c[3..].iter().map(|x| x.parse().expect("results_fit.csv: number")).collect();
        assert_eq!(n.len(), 10 + ATTRIBUTE_COUNT + OWN, "results_fit.csv: columns");
        let mut f = Fit {
            a: n[0],
            b: n[1],
            d: n[2],
            e: n[3],
            c0: n[4],
            sd: n[5],
            season: n[6],
            tau: n[7],
            tp: n[8],
            sk: n[9],
            ..Fit::OFF
        };
        for (k, &v) in n[10..].iter().enumerate() {
            f.set_w(k, v);
        }
        t[index(level, read, pos)] = f;
    }
    t
}

/// Each level's team strength (`super::strength`): mean and spread.
pub const TEAM: [(f64, f64); 5] = [(37.335, 4.812), (60.714, 2.370), (53.286, 2.354), (48.536, 2.354), (81.651, 2.459)];
