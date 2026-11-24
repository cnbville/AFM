//! The program check (lane P0 of `docs/design/program-fortunes.md`): how much
//! of an FBS program's strength lasts, read from the book's season totals and
//! set beside the real FBS 2001-2025 read the same way
//! (`data/real-world/program_persistence.json`, from
//! `data/real-world/scripts/program_persistence.py`).
//!
//! Page `program_check` (also `fortunes`): the correlation of a program's win
//! share, margin a game and roster strength with its own a few years on, the
//! long records, the eras (5-year blocks, rises and falls, the top 10), the
//! head coach's share of the 20 biggest programs' class hauls and margin, and
//! the pass marks the design set before any fix.
//!
//! The reading is on season totals only (the world keeps no past polls or
//! year-by-year SRS), the same way on both sides: FBS team seasons with 8 or
//! more games, the last 25 seasons by default (`from=`, `to=` change it).

use std::collections::HashMap;

use rusqlite::types::Value as S;
use serde_json::Value as J;

use crate::book::Book;
use crate::view::{Filters, Section, View};

/// The real side, written by `scripts/program_persistence.py`.
const REAL: &str = include_str!("../../../data/real-world/program_persistence.json");

/// Seasons read by default, and the lags set beside real.
const WINDOW: i32 = 25;
const LAGS: [i32; 7] = [1, 2, 5, 10, 15, 20, 24];
/// Programs with this many seasons in the window count as long-time.
const LONG_TIME: usize = 22;
/// Class hauls are read over the last 23 classes (real 2003-2025), for the
/// biggest 20 programs, and the coach labels are shuffled this many times.
const HAUL_YEARS: i32 = 23;
const BIG: usize = 20;
const SHUFFLES: usize = 200;

/// One FBS team season.
#[derive(Clone, Debug, PartialEq)]
pub struct Season {
    pub pid: i64,
    pub name: String,
    pub year: i32,
    pub w: u32,
    pub l: u32,
    pub strength: f64,
    /// Points for minus points against, a game; none when the season has no scores.
    pub margin: Option<f64>,
    pub coach: Option<i64>,
}

impl Season {
    fn win_share(&self) -> f64 {
        f64::from(self.w) / f64::from((self.w + self.l).max(1))
    }
}

// ---------------------------------------------------------------- statistics

/// Pearson correlation of paired values; none under 3 pairs or without spread.
pub fn corr(pairs: &[(f64, f64)]) -> Option<f64> {
    let n = pairs.len() as f64;
    if pairs.len() < 3 {
        return None;
    }
    let (mx, my) = (pairs.iter().map(|p| p.0).sum::<f64>() / n, pairs.iter().map(|p| p.1).sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (x, y) in pairs {
        sxy += (x - mx) * (y - my);
        sxx += (x - mx).powi(2);
        syy += (y - my).powi(2);
    }
    (sxx > 0.0 && syy > 0.0).then(|| sxy / (sxx * syy).sqrt())
}

/// A program's value `lag` seasons on, correlated with its value now, over
/// every program and year that has both.
pub fn lag_corr(seasons: &[Season], value: impl Fn(&Season) -> Option<f64>, lag: i32) -> Option<(f64, usize)> {
    let by: HashMap<(i64, i32), f64> = seasons.iter().filter_map(|s| value(s).map(|v| ((s.pid, s.year), v))).collect();
    let mut keys: Vec<&(i64, i32)> = by.keys().collect();
    keys.sort_unstable();
    let pairs: Vec<(f64, f64)> =
        keys.into_iter().filter_map(|k| by.get(&(k.0, k.1 + lag)).map(|&u| (by[k], u))).collect();
    corr(&pairs).map(|r| (r, pairs.len()))
}

/// Ranks (1 = highest value) with ties sharing their average place.
fn rank_desc(values: &[(i64, f64)]) -> HashMap<i64, f64> {
    let mut v = values.to_vec();
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut out = HashMap::new();
    let mut i = 0;
    while i < v.len() {
        let mut j = i;
        while j + 1 < v.len() && v[j + 1].1 == v[i].1 {
            j += 1;
        }
        let avg = (i + j) as f64 / 2.0 + 1.0;
        for x in &v[i..=j] {
            out.insert(x.0, avg);
        }
        i = j + 1;
    }
    out
}

/// Long records of the programs with `LONG_TIME` seasons or more.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Records {
    pub programs: usize,
    pub best: Option<(String, u32, u32, f64)>,
    pub worst: Option<(String, u32, u32, f64)>,
    pub at_300: usize,
    pub under_350: usize,
    pub over_750: usize,
    /// No winning season in all of them / two or fewer.
    pub none_winning: usize,
    pub two_or_fewer: usize,
    pub longest_drought: u32,
    pub drought_team: String,
    pub unbeaten_a_year: f64,
}

pub fn records(seasons: &[Season]) -> Records {
    let mut by: HashMap<i64, Vec<&Season>> = HashMap::new();
    for s in seasons {
        by.entry(s.pid).or_default().push(s);
    }
    let years: std::collections::BTreeSet<i32> = seasons.iter().map(|s| s.year).collect();
    let mut r = Records::default();
    let unbeaten = seasons.iter().filter(|s| s.l == 0 && s.w >= 10).count();
    r.unbeaten_a_year = if years.is_empty() { 0.0 } else { unbeaten as f64 / years.len() as f64 };
    let mut ids: Vec<i64> = by.keys().copied().collect();
    ids.sort_unstable();
    for rows in by.values_mut() {
        rows.sort_by_key(|s| s.year);
    }
    for rows in ids.iter().map(|i| &by[i]) {
        // The longest run without a winning season counts for every program.
        let mut run = 0u32;
        for s in rows.iter() {
            run = if s.w > s.l { 0 } else { run + 1 };
            if run > r.longest_drought {
                r.longest_drought = run;
                r.drought_team = s.name.clone();
            }
        }
    }
    for rows in ids.iter().map(|i| &by[i]).filter(|v| v.len() >= LONG_TIME) {
        r.programs += 1;
        let (w, l): (u32, u32) = rows.iter().fold((0, 0), |a, s| (a.0 + s.w, a.1 + s.l));
        let pct = f64::from(w) / f64::from((w + l).max(1));
        let name = rows[0].name.clone();
        if pct <= 0.300 {
            r.at_300 += 1;
        }
        if pct < 0.350 {
            r.under_350 += 1;
        }
        if pct > 0.750 {
            r.over_750 += 1;
        }
        let winning = rows.iter().filter(|s| s.w > s.l).count();
        if winning == 0 {
            r.none_winning += 1;
        }
        if winning <= 2 {
            r.two_or_fewer += 1;
        }
        if r.best.as_ref().is_none_or(|b| pct > b.3) {
            r.best = Some((name.clone(), w, l, pct));
        }
        if r.worst.as_ref().is_none_or(|b| pct < b.3) {
            r.worst = Some((name, w, l, pct));
        }
    }
    r
}

/// The 5-year blocks: how a block's win share carries to the next, how often
/// it moves by .150 or .250, and the programs in both the bottom and the top
/// quarter of some blocks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Blocks {
    pub next_r: Option<f64>,
    pub moved_150: f64,
    pub moved_250: f64,
    pub pairs: usize,
    pub both_quarters: usize,
    pub of: usize,
}

pub fn blocks(seasons: &[Season]) -> Blocks {
    let Some(first) = seasons.iter().map(|s| s.year).min() else { return Blocks::default() };
    // (program, block) -> (wins, losses, seasons)
    let mut b: HashMap<(i64, i32), (u32, u32, u32)> = HashMap::new();
    for s in seasons {
        let e = b.entry((s.pid, (s.year - first) / 5)).or_default();
        e.0 += s.w;
        e.1 += s.l;
        e.2 += 1;
    }
    let wp: HashMap<(i64, i32), f64> =
        b.iter().filter(|(_, v)| v.2 >= 4).map(|(&k, v)| (k, f64::from(v.0) / f64::from((v.0 + v.1).max(1)))).collect();
    let mut out = Blocks::default();
    let mut wkeys: Vec<&(i64, i32)> = wp.keys().collect();
    wkeys.sort_unstable();
    let pairs: Vec<(f64, f64)> =
        wkeys.into_iter().filter_map(|k| wp.get(&(k.0, k.1 + 1)).map(|&y| (wp[k], y))).collect();
    out.pairs = pairs.len();
    out.next_r = corr(&pairs);
    if !pairs.is_empty() {
        let n = pairs.len() as f64;
        out.moved_150 = pairs.iter().filter(|(x, y)| (y - x).abs() >= 0.15).count() as f64 / n;
        out.moved_250 = pairs.iter().filter(|(x, y)| (y - x).abs() >= 0.25).count() as f64 / n;
    }
    // Quarter of each block by percentile rank (ties share their place).
    let mut quarter: HashMap<(i64, i32), f64> = HashMap::new();
    let blocks: std::collections::BTreeSet<i32> = wp.keys().map(|k| k.1).collect();
    for k in blocks {
        let vals: Vec<(i64, f64)> = wp.iter().filter(|(kk, _)| kk.1 == k).map(|(kk, &v)| (kk.0, v)).collect();
        let n = vals.len() as f64;
        // Ascending percentile = 1 - (descending average rank - 1) / n.
        let desc = rank_desc(&vals);
        for (p, _) in &vals {
            quarter.insert((*p, k), (n + 1.0 - desc[p]) / n);
        }
    }
    let mut per: HashMap<i64, Vec<f64>> = HashMap::new();
    for (&(p, _), &q) in &quarter {
        per.entry(p).or_default().push(q);
    }
    // A program with 4 or more blocks counts (real: 118 of FBS).
    for v in per.values().filter(|v| v.len() >= 4) {
        out.of += 1;
        let (lo, hi) = (v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max));
        if lo <= 0.25 && hi >= 0.75 {
            out.both_quarters += 1;
        }
    }
    out
}

/// The first five seasons' top 10 by margin: how many are still top 10 and
/// top 25 in the last five, and the share moved 30 places or more.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Top {
    pub still_10: usize,
    pub still_25: usize,
    pub moved_30: f64,
    pub n: usize,
}

pub fn top_ten(seasons: &[Season]) -> Option<Top> {
    let years: Vec<i32> =
        seasons.iter().map(|s| s.year).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    if years.len() < 10 {
        return None;
    }
    let mut rk: HashMap<(i64, i32), f64> = HashMap::new();
    for &y in &years {
        let vals: Vec<(i64, f64)> =
            seasons.iter().filter(|s| s.year == y).filter_map(|s| s.margin.map(|m| (s.pid, m))).collect();
        for (p, r) in rank_desc(&vals) {
            rk.insert((p, y), r);
        }
    }
    let mean = |ys: &[i32]| -> HashMap<i64, f64> {
        let mut acc: HashMap<i64, (f64, u32)> = HashMap::new();
        for ((p, y), r) in &rk {
            if ys.contains(y) {
                let e = acc.entry(*p).or_default();
                e.0 += r;
                e.1 += 1;
            }
        }
        acc.into_iter().map(|(p, (s, n))| (p, s / f64::from(n))).collect()
    };
    let a = mean(&years[..5]);
    let z = mean(&years[years.len() - 5..]);
    let mut av: Vec<(i64, f64)> = a.iter().map(|(&p, &v)| (p, v)).collect();
    av.sort_by(|x, y| x.1.total_cmp(&y.1).then(x.0.cmp(&y.0)));
    let zr = {
        // Ranks ascending among programs with a last-five mean (1 = best).
        let negated: Vec<(i64, f64)> = z.iter().map(|(&p, &v)| (p, -v)).collect();
        rank_desc(&negated)
    };
    let top: Vec<i64> = av.iter().take(10).map(|x| x.0).collect();
    let both: Vec<f64> = a.iter().filter_map(|(p, x)| z.get(p).map(|y| (y - x).abs())).collect();
    Some(Top {
        still_10: top.iter().filter(|p| zr.get(p).is_some_and(|&r| r <= 10.0)).count(),
        still_25: top.iter().filter(|p| zr.get(p).is_some_and(|&r| r <= 25.0)).count(),
        moved_30: both.iter().filter(|d| **d >= 30.0).count() as f64 / both.len().max(1) as f64,
        n: both.len(),
    })
}

/// A program's value in a year, with its head coach.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub program: i64,
    pub year: i32,
    pub coach: Option<i64>,
    pub value: f64,
}

/// How the spread among the big programs' years splits: the program, the
/// program and the head coach, the same with the coach labels shuffled inside
/// each program (the noise floor), and the coach's share of what the program
/// leaves, above that floor: (coach R2 - floor) / (1 - program R2).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Share {
    pub n: usize,
    pub stints: usize,
    pub program: f64,
    pub with_coach: f64,
    pub floor: f64,
    pub coach: f64,
}

fn r2_of(cells: &[Cell], labels: &[(i64, Option<i64>)], total: f64) -> f64 {
    let mut m: HashMap<(i64, Option<i64>), (f64, f64)> = HashMap::new();
    for (c, l) in cells.iter().zip(labels) {
        let e = m.entry(*l).or_default();
        e.0 += c.value;
        e.1 += 1.0;
    }
    let sse: f64 = cells
        .iter()
        .zip(labels)
        .map(|(c, l)| {
            let e = m[l];
            (c.value - e.0 / e.1).powi(2)
        })
        .sum();
    1.0 - sse / total
}

pub fn share(cells: &[Cell], shuffles: usize) -> Option<Share> {
    if cells.len() < 10 {
        return None;
    }
    let mean = cells.iter().map(|c| c.value).sum::<f64>() / cells.len() as f64;
    let total: f64 = cells.iter().map(|c| (c.value - mean).powi(2)).sum();
    if total <= 0.0 {
        return None;
    }
    let prog: Vec<(i64, Option<i64>)> = cells.iter().map(|c| (c.program, None)).collect();
    let real: Vec<(i64, Option<i64>)> = cells.iter().map(|c| (c.program, c.coach)).collect();
    let (program, with_coach) = (r2_of(cells, &prog, total), r2_of(cells, &real, total));
    // The same split with each program's coach labels shuffled among its years.
    let mut by: HashMap<i64, Vec<usize>> = HashMap::new();
    for (i, c) in cells.iter().enumerate() {
        by.entry(c.program).or_default().push(i);
    }
    let mut keys: Vec<i64> = by.keys().copied().collect();
    keys.sort_unstable();
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut floor = 0.0;
    for _ in 0..shuffles {
        let mut labels = real.clone();
        for k in &keys {
            let idx = &by[k];
            let mut coaches: Vec<Option<i64>> = idx.iter().map(|&i| cells[i].coach).collect();
            for i in (1..coaches.len()).rev() {
                coaches.swap(i, (next() % (i as u64 + 1)) as usize);
            }
            for (&i, c) in idx.iter().zip(coaches) {
                labels[i].1 = c;
            }
        }
        floor += r2_of(cells, &labels, total);
    }
    let floor = if shuffles == 0 { 0.0 } else { floor / shuffles as f64 };
    let mut stints: Vec<(i64, Option<i64>)> = real;
    stints.sort_unstable();
    stints.dedup();
    Some(Share {
        n: cells.len(),
        stints: stints.len(),
        program,
        with_coach,
        floor,
        coach: if program < 1.0 { (with_coach - floor) / (1.0 - program) } else { 0.0 },
    })
}

/// The haul's cells for the `BIG` programs with the most blue-chip points,
/// each program-year's haul as points per 1,000 of the year's total.
pub fn big_cells(hauls: &[(i64, i32, f64)], coaches: &HashMap<(i64, i32), Option<i64>>) -> Vec<Cell> {
    let mut year_total: HashMap<i32, f64> = HashMap::new();
    let mut prog_total: HashMap<i64, f64> = HashMap::new();
    for &(p, y, v) in hauls {
        *year_total.entry(y).or_default() += v;
        *prog_total.entry(p).or_default() += v;
    }
    let mut order: Vec<(i64, f64)> = prog_total.into_iter().collect();
    order.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let big: std::collections::HashSet<i64> = order.iter().take(BIG).map(|x| x.0).collect();
    let mut out = Vec::new();
    for &(p, y, v) in hauls {
        if big.contains(&p)
            && year_total.get(&y).is_some_and(|t| *t > 0.0)
            && let Some(c) = coaches.get(&(p, y))
        {
            out.push(Cell { program: p, year: y, coach: *c, value: v / year_total[&y] * 1000.0 });
        }
    }
    out.sort_by_key(|c| (c.program, c.year));
    out
}

// ------------------------------------------------------------------- the page

fn rj(path: &[&str]) -> Option<f64> {
    let v: J = serde_json::from_str(REAL).ok()?;
    let mut cur = &v;
    for p in path {
        cur = cur.get(*p)?;
    }
    cur.as_f64()
}

fn rn(path: &[&str]) -> Option<String> {
    let v: J = serde_json::from_str(REAL).ok()?;
    let mut cur = &v;
    for p in path {
        cur = cur.get(*p)?;
    }
    cur.as_str().map(str::to_string)
}

fn f2(x: Option<f64>) -> S {
    x.map_or(S::Null, |x| S::Text(format!("{x:.2}")))
}

fn f3(x: Option<f64>) -> S {
    x.map_or(S::Text("not read".into()), |x| S::Text(format!("{x:.3}")))
}

fn pct(x: Option<f64>) -> S {
    x.map_or(S::Text("not read".into()), |x| S::Text(format!("{:.0}%", x * 100.0)))
}

fn t(s: impl Into<String>) -> S {
    S::Text(s.into())
}

/// The FBS seasons in `from..=to` with 8 or more games.
pub fn load(b: &Book, from: i32, to: i32) -> Result<Vec<Season>, String> {
    let sql = format!(
        "WITH ps AS MATERIALIZED (SELECT p._row AS prog, p.id AS pid, COALESCE(p.real_name, p.fictional_name) AS name, s.year AS year,
         s.wins AS w, s.losses AS l, s.coach AS coach, s.strength AS strength,
         s.totals__games AS g, s.totals__points_for AS pf, s.totals__points_against AS pa,
         COALESCE(h._2, p.division) AS division
         FROM programs__seasons s JOIN programs p ON p._row = s._parent
         LEFT JOIN programs__history h ON h._parent = s._parent AND s.year BETWEEN h._0 AND h._1)
         SELECT pid, name, year, w, l, strength, g, pf, pa, coach FROM ps
         WHERE division = 'FBS' AND year BETWEEN {from} AND {to} AND w + l >= 8 ORDER BY year, pid"
    );
    let (_, rows) = b.rows(&sql).map_err(|e| e.to_string())?;
    let num = |v: &S| match v {
        S::Integer(i) => Some(*i as f64),
        S::Real(f) => Some(*f),
        _ => None,
    };
    Ok(rows
        .iter()
        .map(|r| {
            let games = num(&r[6]).unwrap_or(0.0);
            let margin = (games > 0.0).then(|| (num(&r[7]).unwrap_or(0.0) - num(&r[8]).unwrap_or(0.0)) / games);
            Season {
                pid: num(&r[0]).unwrap_or(-1.0) as i64,
                name: match &r[1] {
                    S::Text(s) => s.clone(),
                    _ => String::new(),
                },
                year: num(&r[2]).unwrap_or(0.0) as i32,
                w: num(&r[3]).unwrap_or(0.0) as u32,
                l: num(&r[4]).unwrap_or(0.0) as u32,
                strength: num(&r[5]).unwrap_or(0.0),
                margin,
                coach: num(&r[9]).map(|c| c as i64),
            }
        })
        .collect())
}

/// Blue-chip points a program signed in each class: 3 for a 5-star and 1
/// for a 4-star, from the classes the book still holds (`class_of` is the
/// season the class first plays).
pub fn hauls(b: &Book, from: i32, to: i32) -> Result<Vec<(i64, i32, f64)>, String> {
    let sql = HAUL_SQL.replace("{from}", &from.to_string()).replace("{to}", &to.to_string());
    let (_, rows) = b.rows(&sql).map_err(|e| e.to_string())?;
    Ok(rows
        .iter()
        .filter_map(|r| match (&r[0], &r[1], &r[2]) {
            (S::Integer(p), S::Integer(y), S::Integer(v)) => Some((*p, *y as i32, *v as f64)),
            (S::Integer(p), S::Integer(y), S::Real(v)) => Some((*p, *y as i32, *v)),
            _ => None,
        })
        .collect())
}

/// Every 4- and 5-star by the program he first signed with and his class:
/// retired and drafted players are in `alumni`, those still in the game in
/// `people` (a drafted pro still playing is in both, so `people` skips him).
const HAUL_SQL: &str = "WITH h AS (
    SELECT CAST(json_extract(colleges, '$[0]') AS INTEGER) AS prog, class_of AS year, stars FROM alumni
    WHERE stars >= 4 AND class_of BETWEEN {from} AND {to}
    UNION ALL
    SELECT CAST(json_extract(path, '$[0]') AS INTEGER), class_of, stars FROM people
    WHERE stars >= 4 AND class_of BETWEEN {from} AND {to} AND path IS NOT NULL
      AND (player__id IS NULL OR player__id NOT IN (SELECT player__id FROM alumni WHERE player__id IS NOT NULL)))
    SELECT p.id, h.year, SUM(CASE WHEN h.stars >= 5 THEN 3 ELSE 1 END) FROM h JOIN programs p ON p._row = h.prog
    GROUP BY p.id, h.year";

/// Five-stars the recruiting log counted over the same classes, a check on
/// the read above.
const LOG_SQL: &str = "SELECT COUNT(*) FROM recruiting_log__five_stars WHERE _0 BETWEEN {from} AND {to}";

pub fn view(b: &Book, f: &Filters) -> View {
    let mut v = View {
        page: "program_check".into(),
        title: "Program check: the rise and fall of programs".into(),
        ..Default::default()
    };
    let last = b.num("SELECT MAX(year) FROM programs__seasons").unwrap_or(0.0) as i32;
    let to = f.year_to.map_or(last, |y| y as i32);
    let from = f.year_from.map_or(to - WINDOW + 1, |y| y as i32);
    let seasons = match load(b, from, to) {
        Ok(s) if !s.is_empty() => s,
        Ok(_) => {
            v.verdict.push(format!("No FBS seasons with 8 or more games in {from}-{to}."));
            return v;
        }
        Err(e) => {
            v.verdict.push(format!("This book has no program seasons to read ({e})."));
            return v;
        }
    };
    let years = seasons.iter().map(|s| s.year).collect::<std::collections::BTreeSet<_>>().len();
    v.verdict.push(format!(
        "{} FBS team seasons, {from}-{to} ({years} seasons). Read from season totals the same way as the real 2001-2025 line (docs/design/program-fortunes.md).",
        seasons.len()
    ));
    let mut marks: Vec<(String, String, String, Option<bool>)> = Vec::new();

    // 1. Persistence by years apart.
    let mut pers = Section {
        title: "A program against itself, years on (correlation)".into(),
        kind: "table".into(),
        ..Default::default()
    };
    pers.columns = ["years on", "win share", "real", "margin a game", "real", "roster strength", "real SRS", "pairs"]
        .map(String::from)
        .to_vec();
    let mut win_at = HashMap::new();
    let mut str_at = HashMap::new();
    for lag in LAGS {
        let l = lag.to_string();
        let win = lag_corr(&seasons, |s| Some(s.win_share()), lag);
        let mar = lag_corr(&seasons, |s| s.margin, lag);
        let st = lag_corr(&seasons, |s| Some(s.strength), lag);
        win_at.insert(lag, win.map(|x| x.0));
        str_at.insert(lag, st.map(|x| x.0));
        pers.rows.push(vec![
            S::Integer(lag.into()),
            f2(win.map(|x| x.0)),
            f2(rj(&["lags", &l, "win_share"])),
            f2(mar.map(|x| x.0)),
            f2(rj(&["lags", &l, "margin"])),
            f2(st.map(|x| x.0)),
            f2(rj(&["lags", &l, "srs"])),
            S::Integer(win.map_or(0, |x| x.1 as i64)),
        ]);
    }
    pers.notes.push("Real: the schedules' scores (sportsdataverse, FBS, 8+ games, 2001-2025). The world's roster strength and the real SRS are different measures, so read how far each falls, not the level. The real 24-year line has only 116 pairs.".into());
    v.sections.push(pers);
    marks.push((
        "win share 10 years on".into(),
        "under 0.27".into(),
        f3(win_at[&10]).to_string_cell(),
        win_at[&10].map(|x| x < 0.27),
    ));
    marks.push((
        "win share 20 years on".into(),
        "under 0.25".into(),
        f3(win_at[&20]).to_string_cell(),
        win_at[&20].map(|x| x < 0.25),
    ));
    marks.push((
        "roster strength 10 years on".into(),
        "under 0.60".into(),
        f3(str_at[&10]).to_string_cell(),
        str_at[&10].map(|x| x < 0.60),
    ));

    // 2. Long records.
    let r = records(&seasons);
    let rec = |name: &str| rj(&["records", name]);
    let rec_line = |a: &Option<(String, u32, u32, f64)>| {
        a.as_ref().map_or("none".to_string(), |(n, w, l, p)| format!("{n} {p:.3} ({w}-{l})"))
    };
    let real_rec = |key: &str| {
        let v: J = serde_json::from_str(REAL).unwrap_or_default();
        v["records"][key][0]
            .as_array()
            .map(|a| format!("{} {:.3} ({}-{})", a[0].as_str().unwrap_or(""), a[3].as_f64().unwrap_or(0.0), a[1], a[2]))
            .unwrap_or_default()
    };
    let mut lr = Section {
        title: format!("Long records (programs with {LONG_TIME}+ FBS seasons in the window)"),
        kind: "table".into(),
        ..Default::default()
    };
    lr.columns = ["measure", "world", "real"].map(String::from).to_vec();
    let n = |x: usize| t(x.to_string());
    let rn_ = |k: &str| t(rec(k).map_or("?".into(), |x| format!("{x:.0}")));
    lr.rows = vec![
        vec![t("programs counted"), n(r.programs), rn_("programs_22_plus")],
        vec![t("best record"), t(rec_line(&r.best)), t(real_rec("best"))],
        vec![t("worst record"), t(rec_line(&r.worst)), t(real_rec("worst"))],
        vec![t("at .300 or under"), n(r.at_300), rn_("at_300_or_under")],
        vec![t("under .350"), n(r.under_350), rn_("under_350")],
        vec![t("over .750"), n(r.over_750), rn_("over_750")],
        vec![t("no winning season in the window"), n(r.none_winning), rn_("no_winning_season")],
        vec![t("two or fewer winning seasons"), n(r.two_or_fewer), rn_("two_or_fewer_winning")],
        vec![
            t("longest run without a winning season"),
            t(format!("{} ({})", r.longest_drought, r.drought_team)),
            t(format!(
                "{} ({})",
                rec("longest_drought").map_or("?".into(), |x| format!("{x:.0}")),
                rn(&["records", "longest_drought_team"]).unwrap_or_default()
            )),
        ],
        vec![
            t("unbeaten seasons (10+ wins, no loss) a year"),
            t(format!("{:.2}", r.unbeaten_a_year)),
            f2(rec("unbeaten_a_year")),
        ],
        vec![
            t("longest winning streak"),
            t("needs every game; not in season totals"),
            t(format!("{} (USC)", rec("longest_win_streak").map_or("?".into(), |x| format!("{x:.0}")))),
        ],
    ];
    v.sections.push(lr);
    if r.programs > 0 {
        marks.push((
            format!("best {}-season record", years.min(25)),
            "under .870".into(),
            r.best.as_ref().map_or("none".into(), |b| format!("{:.3}", b.3)),
            r.best.as_ref().map(|b| b.3 < 0.870),
        ));
        let low = seasons_under(&seasons, 0.330);
        marks.push(("long-time programs at .330 or under".into(), "at most 2".into(), low.to_string(), Some(low <= 2)));
        marks.push((
            "programs with no winning season in 20 straight".into(),
            "none".into(),
            r.longest_drought.to_string(),
            Some(r.longest_drought < 20),
        ));
    }
    marks.push((
        "unbeaten seasons a year".into(),
        "0.6 to 1.0".into(),
        format!("{:.2}", r.unbeaten_a_year),
        Some((0.6..=1.0).contains(&r.unbeaten_a_year)),
    ));

    // 3. Eras.
    let bl = blocks(&seasons);
    let top = top_ten(&seasons);
    let mut er =
        Section { title: "Eras: how much programs rise and fall".into(), kind: "table".into(), ..Default::default() };
    er.columns = ["measure", "world", "real"].map(String::from).to_vec();
    er.rows = vec![
        vec![
            t("a 5-year block's win share against the next block's (r)"),
            f2(bl.next_r),
            f2(rj(&["blocks", "next_block_r"])),
        ],
        vec![t("blocks that moved .150 or more"), pct(Some(bl.moved_150)), pct(rj(&["blocks", "moved_150_plus"]))],
        vec![t("blocks that moved .250 or more"), pct(Some(bl.moved_250)), pct(rj(&["blocks", "moved_250_plus"]))],
        vec![
            t("programs in both the bottom and top quarter of some 5-year blocks"),
            t(format!("{} of {}", bl.both_quarters, bl.of)),
            t(format!(
                "{} of {}",
                rj(&["blocks", "bottom_and_top_quarter"]).map_or("?".into(), |x| format!("{x:.0}")),
                rj(&["blocks", "of"]).map_or("?".into(), |x| format!("{x:.0}"))
            )),
        ],
        vec![
            t("first five years' top 10 (by margin) still top 10 in the last five"),
            t(top.as_ref().map_or("needs 10 seasons".into(), |x| x.still_10.to_string())),
            t(rj(&["top10_first_to_last_five_years", "margin", "still_top10"])
                .map_or("?".into(), |x| format!("{x:.0}"))),
        ],
        vec![
            t("... still top 25"),
            t(top.as_ref().map_or("needs 10 seasons".into(), |x| x.still_25.to_string())),
            t(rj(&["top10_first_to_last_five_years", "margin", "still_top25"])
                .map_or("?".into(), |x| format!("{x:.0}"))),
        ],
        vec![
            t("programs whose 5-year margin rank moved 30+ places"),
            pct(top.as_ref().map(|x| x.moved_30)),
            pct(rj(&["top10_first_to_last_five_years", "margin", "moved_30_places"])),
        ],
    ];
    er.notes.push("The real top-10 line here is by raw margin a game, as the world's is; by a schedule-adjusted SRS it is 4 still top 10 and 9 still top 25 (22% moved 30 places).".into());
    v.sections.push(er);
    marks.push((
        "programs in both bottom and top quarter of some 5-year blocks".into(),
        "15 or more".into(),
        bl.both_quarters.to_string(),
        Some(bl.both_quarters >= 15),
    ));

    // 4. The head coach.
    let hfrom = (to - HAUL_YEARS + 1).max(from);
    let mut hc = Section {
        title: format!("The head coach at the {BIG} biggest programs, {hfrom}-{to}"),
        kind: "table".into(),
        ..Default::default()
    };
    hc.columns = ["measure", "world", "real"].map(String::from).to_vec();
    let coaches: HashMap<(i64, i32), Option<i64>> = seasons.iter().map(|s| ((s.pid, s.year), s.coach)).collect();
    let haul_share = hauls(b, hfrom, to).ok().and_then(|h| {
        let cells = big_cells(&h, &coaches);
        share(&cells, SHUFFLES)
    });
    let margin_cells: Vec<Cell> = {
        let haul_programs: Option<Vec<Cell>> = hauls(b, hfrom, to).ok().map(|h| big_cells(&h, &coaches));
        let big: std::collections::HashSet<i64> = haul_programs.unwrap_or_default().iter().map(|c| c.program).collect();
        seasons
            .iter()
            .filter(|s| s.year >= hfrom && big.contains(&s.pid))
            .filter_map(|s| s.margin.map(|m| Cell { program: s.pid, year: s.year, coach: s.coach, value: m }))
            .collect()
    };
    let margin_share = share(&margin_cells, SHUFFLES);
    let line = |name: &str, w: Option<&Share>, key: &str| {
        let g = |f: fn(&Share) -> f64| pct(w.map(f));
        vec![
            vec![
                t(format!("{name}: the program's share")),
                g(|s| s.program),
                pct(rj(&["coach_share", key, "program"])),
            ],
            vec![
                t(format!("{name}: program and head coach")),
                g(|s| s.with_coach),
                pct(rj(&["coach_share", key, "program_and_coach"])),
            ],
            vec![
                t(format!("{name}: coach labels shuffled (floor)")),
                g(|s| s.floor),
                pct(rj(&["coach_share", key, "noise_floor"])),
            ],
            vec![
                t(format!("{name}: the head coach's share of the rest")),
                g(|s| s.coach),
                pct(rj(&["coach_share", key, "coach_share_of_rest"])),
            ],
        ]
    };
    hc.rows.extend(line("class hauls", haul_share.as_ref(), "haul"));
    hc.rows.extend(line("margin a game", margin_share.as_ref(), "margin"));
    let five = |sql: &str| b.num(&sql.replace("{from}", &hfrom.to_string()).replace("{to}", &to.to_string()));
    let in_read = five(
        "SELECT SUM(CASE WHEN stars >= 5 THEN 1 ELSE 0 END) FROM (SELECT stars FROM alumni WHERE class_of BETWEEN {from} AND {to} UNION ALL SELECT stars FROM people WHERE class_of BETWEEN {from} AND {to} AND path IS NOT NULL AND (player__id IS NULL OR player__id NOT IN (SELECT player__id FROM alumni WHERE player__id IS NOT NULL)))",
    );
    if let (Some(a), Some(l)) = (in_read, five(LOG_SQL)) {
        hc.notes.push(format!(
            "Check on the read: {a:.0} five-stars in these classes in alumni and people, {l:.0} in the recruiting log."
        ));
    }
    hc.notes.push(format!(
        "Hauls are blue-chip points (3 a 5-star, 1 a 4-star) per 1,000 of the year's total, from the classes the book holds{}. {}",
        if haul_share.is_none() { "; none were read from this book" } else { "" },
        haul_share.as_ref().map_or(String::new(), |s| format!("{} class-years, {} coach stints.", s.n, s.stints))
    ));
    hc.notes.push("The coach's share is (program and coach, minus the floor) over (1 minus the program's share), as in the diagnosis (afm/dynasty-layers/diagnosis.md). Part of the real 29% is drift, not coaches.".into());
    v.sections.push(hc);
    marks.push((
        "head coach's share of class swings".into(),
        "20 to 35%".into(),
        haul_share.as_ref().map_or("not read".into(), |s| format!("{:.0}%", s.coach * 100.0)),
        haul_share.as_ref().map(|s| (0.20..=0.35).contains(&s.coach)),
    ));

    // 5. Pass marks.
    let mut pm = Section {
        title: "Pass marks (set before any fix; one book is one seed, the mark is on every seed)".into(),
        kind: "table".into(),
        ..Default::default()
    };
    pm.columns = ["measure", "pass mark", "world", "result"].map(String::from).to_vec();
    let (mut pass, mut counted) = (0, 0);
    for (name, mark, val, ok) in &marks {
        if let Some(ok) = ok {
            counted += 1;
            pass += usize::from(*ok);
        }
        pm.rows.push(vec![
            t(name),
            t(mark),
            t(val),
            t(match ok {
                Some(true) => "pass",
                Some(false) => "FAIL",
                None => "not read (needs more seasons or data)",
            }),
        ]);
    }
    pm.notes.push("Also set before: win share 10 years on under 0.27 and 20 years on under 0.25 on every seed, strength 10 years on under 0.60, 9 to 10 champions in 15 years stays.".into());
    v.sections.push(pm);
    v.verdict.push(format!("Pass marks met: {pass} of {counted} read."));
    v.next.push("program_check from=2001 to=2025: another window".into());
    v.next.push("table programs__seasons: the season totals read".into());
    v
}

/// Long-time programs (LONG_TIME+ seasons) whose record is `max` or under.
fn seasons_under(seasons: &[Season], max: f64) -> usize {
    let mut by: HashMap<i64, (u32, u32, usize)> = HashMap::new();
    for s in seasons {
        let e = by.entry(s.pid).or_default();
        e.0 += s.w;
        e.1 += s.l;
        e.2 += 1;
    }
    by.values().filter(|v| v.2 >= LONG_TIME && f64::from(v.0) / f64::from((v.0 + v.1).max(1)) <= max).count()
}

trait Cellable {
    fn to_string_cell(&self) -> String;
}

impl Cellable for S {
    fn to_string_cell(&self) -> String {
        crate::text::cell(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn season(pid: i64, year: i32, w: u32, l: u32, margin: f64, coach: i64) -> Season {
        Season {
            pid,
            name: format!("P{pid}"),
            year,
            w,
            l,
            strength: f64::from(w),
            margin: Some(margin),
            coach: Some(coach),
        }
    }

    /// Programs that never change correlate 1 with themselves at every lag;
    /// programs with no memory correlate near 0.
    #[test]
    fn frozen_programs_correlate_fully_and_random_ones_do_not() {
        let mut frozen = Vec::new();
        let mut shuffled = Vec::new();
        let mut x = 12345u64;
        for p in 0..40 {
            for y in 0..25 {
                frozen.push(season(p, 2001 + y, (p % 12) as u32, 12 - (p % 12) as u32, 0.0, 0));
                x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let w = ((x >> 33) % 12) as u32;
                shuffled.push(season(p, 2001 + y, w, 12 - w, 0.0, 0));
            }
        }
        let f = lag_corr(&frozen, |s| Some(s.win_share()), 10).unwrap();
        assert!((f.0 - 1.0).abs() < 1e-9 && f.1 == 40 * 15, "{f:?}");
        let r = lag_corr(&shuffled, |s| Some(s.win_share()), 10).unwrap();
        assert!(r.0.abs() < 0.15, "{r:?}");
        assert!(lag_corr(&frozen, |s| Some(s.win_share()), 40).is_none());
    }

    #[test]
    fn long_records_count_droughts_and_the_bottom() {
        let mut v = Vec::new();
        for y in 0..24 {
            v.push(season(1, 2001 + y, 11, 1, 10.0, 0)); // .917
            v.push(season(2, 2001 + y, 3, 9, -10.0, 0)); // .250, never winning
            v.push(season(3, 2001 + y, 6 + (y % 2) as u32 * 2, 6 - (y % 2) as u32 * 2, 0.0, 0));
        }
        // A short-lived program doesn't count as long-time.
        v.push(season(4, 2001, 12, 0, 20.0, 0));
        let r = records(&v);
        assert_eq!(r.programs, 3);
        assert_eq!(r.at_300, 1);
        assert_eq!(r.none_winning, 1);
        assert_eq!(r.longest_drought, 24);
        assert_eq!(r.drought_team, "P2");
        assert_eq!(r.best.as_ref().unwrap().0, "P1");
        assert_eq!(r.worst.as_ref().unwrap().0, "P2");
        assert!((r.unbeaten_a_year - 1.0 / 24.0).abs() < 1e-9);
    }

    /// Eras: a program that is bottom in the early blocks and top in the later
    /// ones is counted; a steady one is not.
    #[test]
    fn blocks_find_risers() {
        let mut v = Vec::new();
        for p in 0..8i64 {
            for y in 0..25 {
                let w = if p == 0 {
                    if y < 10 { 2 } else { 11 }
                } else {
                    3 + (p as u32 % 7) // steady, spread
                };
                v.push(season(p, 2001 + y, w, 12 - w, 0.0, 0));
            }
        }
        let b = blocks(&v);
        assert_eq!(b.of, 8);
        assert_eq!(b.both_quarters, 1);
        assert!(b.moved_250 > 0.0 && b.next_r.unwrap() > 0.5);
    }

    #[test]
    fn top_ten_persistence() {
        let mut v = Vec::new();
        for p in 0..30i64 {
            for y in 0..25 {
                v.push(season(p, 2001 + y, 6, 6, 30.0 - p as f64, 0)); // rank never changes
            }
        }
        let t = top_ten(&v).unwrap();
        assert_eq!((t.still_10, t.still_25, t.n), (10, 10, 30));
        assert_eq!(t.moved_30, 0.0);
        let short: Vec<Season> = v.iter().filter(|s| s.year < 2010).cloned().collect();
        assert!(top_ten(&short).is_none());
    }

    /// Hauls that follow the program give the coach no share; hauls that follow
    /// the coach beat the shuffled floor.
    #[test]
    fn the_coach_share_reads_stints_above_the_floor() {
        let mut by_program = Vec::new();
        let mut by_coach = Vec::new();
        for p in 0..20i64 {
            for y in 0..24 {
                let coach = i64::from(y / 6);
                let base = 50.0 - p as f64;
                by_program.push(Cell {
                    program: p,
                    year: 2000 + y,
                    coach: Some(coach),
                    value: base + ((y * 7 + p as i32 * 3) % 5) as f64 * 0.01,
                });
                let bump = if coach % 2 == 0 { 15.0 } else { -15.0 };
                by_coach.push(Cell { program: p, year: 2000 + y, coach: Some(coach), value: base + bump });
            }
        }
        let a = share(&by_program, 50).unwrap();
        assert!(a.program > 0.9 && a.coach < 0.1, "{a:?}");
        let b = share(&by_coach, 50).unwrap();
        assert!(b.coach > 0.5 && b.with_coach > b.floor, "{b:?}");
        assert!(share(&by_coach[..5], 5).is_none());
    }

    #[test]
    fn the_biggest_programs_are_picked_by_blue_chip_points() {
        let mut h = Vec::new();
        let mut c = HashMap::new();
        for p in 0..30i64 {
            for y in 2003..2026 {
                h.push((p, y, (30 - p) as f64));
                c.insert((p, y), Some(p));
            }
        }
        let cells = big_cells(&h, &c);
        let progs: std::collections::HashSet<i64> = cells.iter().map(|x| x.program).collect();
        assert_eq!(progs.len(), BIG);
        assert!(progs.contains(&0) && !progs.contains(&25));
    }

    /// The real side parses and holds every figure the page prints.
    #[test]
    fn the_real_line_is_complete() {
        for lag in LAGS {
            for k in ["srs", "win_share", "margin"] {
                assert!(rj(&["lags", &lag.to_string(), k]).is_some(), "lag {lag} {k}");
            }
        }
        for k in ["at_300_or_under", "under_350", "unbeaten_a_year", "longest_drought"] {
            assert!(rj(&["records", k]).is_some(), "{k}");
        }
        assert_eq!(rj(&["coach_share", "haul", "coach_share_of_rest"]), Some(0.29));
        assert!(rj(&["top10_first_to_last_five_years", "margin", "still_top10"]).is_some());
        assert!(rj(&["blocks", "bottom_and_top_quarter"]).is_some());
    }

    /// The page runs on a book with the engine's own table and column names:
    /// 30 programs for 12 seasons, big programs signing blue chips, coaches
    /// who change every 4 years.
    #[test]
    fn the_page_reads_a_book() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE programs(_row INTEGER, id INTEGER, real_name TEXT, fictional_name TEXT, division TEXT);
             CREATE TABLE programs__seasons(_parent INTEGER, year INTEGER, wins INTEGER, losses INTEGER, coach INTEGER, strength REAL,
               totals__games INTEGER, totals__points_for INTEGER, totals__points_against INTEGER);
             CREATE TABLE programs__history(_parent INTEGER, _0 INTEGER, _1 INTEGER, _2 TEXT, _3 TEXT);
             CREATE TABLE alumni(colleges TEXT, stars INTEGER, class_of INTEGER, player__id INTEGER);
             CREATE TABLE people(path TEXT, stars INTEGER, class_of INTEGER, player__id INTEGER);
             CREATE TABLE recruiting_log__five_stars(_0 INTEGER, _1 INTEGER);",
        )
        .unwrap();
        for p in 0..30 {
            c.execute(
                "INSERT INTO programs VALUES (?1, ?1, ?2, NULL, 'FBS')",
                rusqlite::params![p, format!("Prog {p}")],
            )
            .unwrap();
            for y in 0..12 {
                let w = 4 + (p % 8) + (y % 3);
                c.execute(
                    "INSERT INTO programs__seasons VALUES (?1, ?2, ?3, ?4, ?5, ?6, 12, ?7, 200)",
                    rusqlite::params![p, 2014 + y, w, 12 - w, p * 10 + y / 4, 50.0 + f64::from(w), 150 + w * 10],
                )
                .unwrap();
                // Blue chips follow the program (big for low numbers).
                for k in 0..(30 - p) / 6 {
                    let (table, col) = if y % 2 == 0 { ("alumni", "colleges") } else { ("people", "path") };
                    c.execute(
                        &format!("INSERT INTO {table} ({col}, stars, class_of, player__id) VALUES (?1, ?2, ?3, NULL)"),
                        rusqlite::params![format!("[{p}, 3]"), 4 + (k % 2), 2014 + y],
                    )
                    .unwrap();
                }
            }
        }
        let b = Book::with(c).unwrap();
        let text = b.text("program_check", &Filters::default());
        for want in [
            "360 FBS team seasons",
            "years on",
            "Long records",
            "Eras",
            "head coach",
            "Pass marks",
            "class hauls: the head coach's share",
        ] {
            assert!(text.contains(want), "{want}: {text}");
        }
        assert!(!text.contains("none were read"), "{text}");
        assert_eq!(b.text("fortunes", &Filters::default()), text);
        // A window with no seasons says so instead of failing.
        let none = b.text("program_check", &Filters::parse(&["from=1900".into(), "to=1901".into()]));
        assert!(none.contains("No FBS seasons"), "{none}");
    }
}
