//! Real NFL tables (nflverse, 2021-2025): expected points, situational pass
//! rates, fourth-down go rates and win probability. Coordinators start from
//! how real teams call games and bend it by their own scheme and judgment;
//! EPA in the stats uses the expected points.

use std::collections::HashMap;
use std::sync::LazyLock;

const EP_CSV: &str = include_str!("../../../../data/real-world/nfl_expected_points.csv");
const PASS_CSV: &str = include_str!("../../../../data/real-world/nfl_pass_rate.csv");
const GO_CSV: &str = include_str!("../../../../data/real-world/nfl_fourth_go.csv");
// College calling (trial 4 topic 5): FBS-vs-FBS cfbfastR play-by-play,
// 2023-24, in the same buckets (`college_situations.py`). College teams pass
// half their snaps, the NFL 58%; high school uses the college habit too
// (no real table; proposed, not confirmed).
const COLLEGE_PASS_CSV: &str = include_str!("../../../../data/real-world/college_pass_rate.csv");
const COLLEGE_GO_CSV: &str = include_str!("../../../../data/real-world/college_fourth_go.csv");
const WP_CSV: &str = include_str!("../../../../data/real-world/nfl_win_prob.csv");

fn rows(csv: &str) -> impl Iterator<Item = Vec<&str>> {
    csv.lines().skip(1).filter(|l| !l.trim().is_empty()).map(|l| l.split(',').collect())
}

fn togo_bucket(t: f64) -> i32 {
    if t <= 1.0 {
        1
    } else if t <= 3.0 {
        3
    } else if t <= 6.0 {
        6
    } else if t <= 10.0 {
        10
    } else {
        15
    }
}

fn ep_togo_bucket(t: f64) -> i32 {
    if t <= 1.0 {
        1
    } else if t <= 2.0 {
        2
    } else if t <= 3.0 {
        3
    } else if t <= 6.0 {
        6
    } else if t <= 10.0 {
        10
    } else if t <= 15.0 {
        15
    } else {
        20
    }
}

fn score_bucket(s: i32) -> i32 {
    match s {
        i32::MIN..=-15 => -15,
        -14..=-8 => -8,
        -7..=-1 => -1,
        0 => 0,
        1..=7 => 7,
        8..=14 => 14,
        _ => 15,
    }
}

/// (down, togo bucket, yardline bucket) -> EP, plus a coarse fallback by
/// (down, yardline bucket).
struct Ep {
    cells: HashMap<(i32, i32, i32), f64>,
    by_field: HashMap<(i32, i32), (f64, f64)>,
}

static EP: LazyLock<Ep> = LazyLock::new(|| {
    let mut cells = HashMap::new();
    let mut by_field: HashMap<(i32, i32), (f64, f64)> = HashMap::new();
    for r in rows(EP_CSV) {
        let (d, t, y): (i32, i32, i32) = (r[0].parse().unwrap(), r[1].parse().unwrap(), r[2].parse().unwrap());
        let (ep, n): (f64, f64) = (r[3].parse().unwrap(), r[4].parse().unwrap());
        cells.insert((d, t, y), ep);
        let e = by_field.entry((d, y)).or_insert((0.0, 0.0));
        e.0 += ep * n;
        e.1 += n;
    }
    Ep { cells, by_field }
});

/// Expected points for the offense before a snap: down, yards to go, and
/// yards from the opponent's goal line.
pub fn expected_points(down: u8, togo: f64, yardline_100: f64) -> f64 {
    let d = down.clamp(1, 4) as i32;
    let y = ((yardline_100.clamp(1.0, 99.0) as i32) / 5) * 5;
    let t = ep_togo_bucket(togo);
    if let Some(&e) = EP.cells.get(&(d, t, y)) {
        return e;
    }
    // Rare cells: average over distances at that spot, else the nearest spot.
    for dy in [0, 5, -5, 10, -10, 15, -15] {
        if let Some(&(s, n)) = EP.by_field.get(&(d, y + dy)) {
            return s / n - 0.08 * (togo - 10.0).max(-9.0) * if d >= 3 { 1.5 } else { 1.0 };
        }
    }
    0.0
}

type PassTable = HashMap<(i32, i32, i32, u8), (f64, f64)>;

static PASS: LazyLock<PassTable> = LazyLock::new(|| pass_table(PASS_CSV));
static COLLEGE_PASS: LazyLock<PassTable> = LazyLock::new(|| pass_table(COLLEGE_PASS_CSV));

fn pass_table(csv: &str) -> PassTable {
    let mut m = HashMap::new();
    for r in rows(csv) {
        let time = match r[3] {
            "late" => 1,
            "two_minute" => 2,
            _ => 0,
        };
        m.insert(
            (r[0].parse().unwrap(), r[1].parse().unwrap(), r[2].parse().unwrap(), time),
            (r[4].parse().unwrap(), r[5].parse().unwrap()),
        );
    }
    m
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Clock {
    Normal,
    /// Fourth quarter, last seven minutes.
    Late,
    /// Last two minutes of a half.
    TwoMinute,
}

/// How often real offenses pass in this situation (downs 1-3): the NFL's
/// habit, or the college one below the NFL.
pub fn real_pass_rate(down: u8, togo: f64, score_diff: i32, clock: Clock, college: bool) -> f64 {
    let table = if college { &*COLLEGE_PASS } else { &*PASS };
    let d = down.clamp(1, 3) as i32;
    let t = togo_bucket(togo);
    let s = score_bucket(score_diff);
    let c = match clock {
        Clock::Normal => 0,
        Clock::Late => 1,
        Clock::TwoMinute => 2,
    };
    for (cc, ss) in [(c, s), (0, s), (c, 0), (0, 0)] {
        if let Some(&(rate, _)) = table.get(&(d, t, ss, cc)) {
            return rate;
        }
    }
    if college { 0.5 } else { 0.58 }
}

fn go_table(csv: &str) -> HashMap<(i32, i32, i32), f64> {
    rows(csv)
        .map(|r| ((r[0].parse().unwrap(), r[1].parse().unwrap(), r[2].parse().unwrap()), r[3].parse().unwrap()))
        .collect()
}

static GO: LazyLock<HashMap<(i32, i32, i32), f64>> = LazyLock::new(|| go_table(GO_CSV));
static COLLEGE_GO: LazyLock<HashMap<(i32, i32, i32), f64>> = LazyLock::new(|| go_table(COLLEGE_GO_CSV));

/// How often real teams go for it on fourth down here (NFL, or college
/// below the NFL).
pub fn real_fourth_go_rate(togo: f64, yardline_100: f64, score_diff: i32, college: bool) -> f64 {
    let table = if college { &*COLLEGE_GO } else { &*GO };
    let t = togo_bucket(togo);
    let y = ((yardline_100.clamp(1.0, 99.0) as i32) / 10).min(9) * 10;
    let s = score_bucket(score_diff);
    for ss in [s, 0, -1, 7] {
        if let Some(&r) = table.get(&(t, y, ss)) {
            return r;
        }
    }
    0.1
}

static WP: LazyLock<HashMap<(i32, i32, i32), f64>> = LazyLock::new(|| {
    rows(WP_CSV)
        .map(|r| ((r[0].parse().unwrap(), r[1].parse().unwrap(), r[2].parse().unwrap()), r[3].parse().unwrap()))
        .collect()
});

/// The offense's win probability from score, game seconds left and field
/// position (nflfastR's model, averaged). Used by coaches' clock and fourth
/// down judgment, and for the win-probability graph.
pub fn win_probability(score_diff: i32, game_seconds_left: f64, yardline_100: f64) -> f64 {
    let s = (score_diff.div_euclid(4) * 4).clamp(-28, 28);
    let g = ((game_seconds_left.clamp(0.0, 3599.0) as i32) / 300) * 300;
    let y = ((yardline_100.clamp(0.0, 99.0) as i32) / 25).min(3);
    for (ds, dg) in [(0, 0), (0, 300), (0, -300), (4, 0), (-4, 0), (0, 600)] {
        if let Some(&w) = WP.get(&(s + ds, g + dg, y)) {
            return w;
        }
    }
    // Outside the table: a logistic on the margin per remaining possession.
    let poss = (game_seconds_left / 330.0).max(0.4);
    1.0 / (1.0 + (-(score_diff as f64) / (4.5 * poss.sqrt())).exp())
}
