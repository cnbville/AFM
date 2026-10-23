//! Betting lines (creative addition C8): a point spread and an over/under
//! for every NFL game and every FBS and FCS game, set by the market from
//! what the public can see (each team's strength as the world reads it,
//! home field, the season's scoring). Upsets are judged by the line, and
//! each head coach keeps a record against the spread. Information only:
//! nothing in the world reads a line.

use super::{draw_normal, probit, story};
use crate::calendar::{Level, Loudness};
use crate::world::World;
use crate::world::data::Division;

/// One score's spread in points: how the world's own win chances turn
/// into a points margin (a game's margin spreads about 13.5 points
/// around the line in the NFL and 16 in college, by the 2010-2024 lines).
const NFL_SD: f64 = 13.5;
const COLLEGE_SD: f64 = 16.0;

/// What the market has learned of a team from its games against the line:
/// the points it beat the line by, summed, and the games. A team the
/// numbers misread keeps covering (or missing) until the line moves with
/// it; last season's lesson counts half, and nothing older.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Form {
    pub year: i32,
    pub sum: f32,
    pub n: f32,
}

pub(crate) fn form_key(level: Level, t: usize) -> u32 {
    (if level == Level::Nfl { 0 } else { 1 << 20 }) | t as u32
}

/// The points a team's line moves for what the market has learned
/// (shrunk toward nothing over its first games).
fn form(w: &World, level: Level, t: usize) -> f64 {
    let Some(f) = w.finance.form.get(&form_key(level, t)) else { return 0.0 };
    let (sum, n) = match w.year - f.year {
        0 => (f.sum, f.n),
        1 => (0.5 * f.sum, 0.5 * f.n),
        _ => return 0.0,
    };
    sum as f64 / (n as f64 + 4.0)
}

/// A finished game teaches the market: each side's margin against the line.
pub(crate) fn learn(w: &mut World, level: Level, teams: [usize; 2], line: [f32; 2], score: [u32; 2]) {
    let cover = score[0] as f32 - score[1] as f32 + line[0];
    let year = w.year;
    for (t, x) in [(teams[0], cover), (teams[1], -cover)] {
        let f = w.finance.form.entry(form_key(level, t)).or_default();
        if f.year != year {
            let keep = if f.year == year - 1 { 0.5 } else { 0.0 };
            *f = Form { year, sum: f.sum * keep, n: f.n * keep };
        }
        f.sum += x;
        f.n += 1.0;
    }
}

fn half(x: f64) -> f32 {
    ((x * 2.0).round() / 2.0) as f32
}

/// An NFL line: (home spread, total) from the win chance and both
/// teams' scoring so far, with a little market noise.
pub(crate) fn nfl(w: &World, p_home: f64, home: usize, away: usize) -> [f32; 2] {
    let margin = NFL_SD * probit(p_home) + form(w, Level::Nfl, home) - form(w, Level::Nfl, away);
    let noise = 0.6 * draw_normal(w, 0x11E5, (home as u64) << 8 ^ away as u64, w.year);
    let spread = half(-(margin + noise));
    let (pf_h, pa_h, n_h) = nfl_points(w, home);
    let (pf_a, pa_a, n_a) = nfl_points(w, away);
    let avg = 22.0;
    let shrink = |x: f64, n: f64| (x * n + avg * 4.0) / (n + 4.0);
    let total = shrink(pf_h, n_h) + shrink(pa_a, n_a) + shrink(pf_a, n_a) + shrink(pa_h, n_h);
    [spread, half(total / 2.0 + 0.8 * draw_normal(w, 0x11E6, (home as u64) << 8 ^ away as u64, w.year))]
}

/// A team's points for and against a game this NFL season, and its games.
fn nfl_points(w: &World, t: usize) -> (f64, f64, f64) {
    let Some(s) = w.nfl_season.as_ref() else { return (22.0, 22.0, 0.0) };
    let (mut pf, mut pa, mut n) = (0.0, 0.0, 0.0);
    for (k, week) in s.weeks.iter().enumerate() {
        for (g, &(h, a)) in week.iter().enumerate() {
            let Some(sc) = s.slots.get(k).and_then(|x| x.get(g)).and_then(|x| x.score) else { continue };
            if h == t {
                pf += sc[0] as f64;
                pa += sc[1] as f64;
                n += 1.0;
            } else if a == t {
                pf += sc[1] as f64;
                pa += sc[0] as f64;
                n += 1.0;
            }
        }
    }
    if n == 0.0 { (22.0, 22.0, 0.0) } else { (pf / n, pa / n, n) }
}

/// A college line, for FBS and FCS games only (the books don't hang lines
/// on Division II and below).
pub(crate) fn college(w: &World, home: usize, away: usize, neutral: bool) -> Option<[f32; 2]> {
    let cs = w.college.as_ref()?;
    let (dh, da) = (w.programs[home].division, w.programs[away].division);
    let d1 = |d: Division| matches!(d, Division::FBS | Division::FCS);
    if !d1(dh) || !d1(da) {
        return None;
    }
    let (rh, ra) = (cs.results.get(home)?, cs.results.get(away)?);
    let adv = if neutral { 0.0 } else { 0.8 };
    let p = crate::world::season::win_chance(rh.strength, ra.strength, w.college_spread.max(0.5), adv);
    let margin = COLLEGE_SD * probit(p) + form(w, Level::College, home) - form(w, Level::College, away);
    let key = (home as u64) << 16 ^ away as u64;
    let spread = half(-(margin + 0.8 * draw_normal(w, 0xC11E, key, w.year)));
    let avg = if dh == Division::FBS || da == Division::FBS { 27.5 } else { 26.0 };
    let per = |t: &crate::world::season::TeamResult| {
        let g = t.totals.games as f64;
        let pf = if g > 0.0 { t.totals.points_for as f64 / g } else { avg };
        let pa = if g > 0.0 { t.totals.points_against as f64 / g } else { avg };
        let s = |x: f64| (x * g + avg * 3.0) / (g + 3.0);
        (s(pf), s(pa))
    };
    let (fh, ah) = per(rh);
    let (fa, aa) = per(ra);
    let total = (fh + aa + fa + ah) / 2.0 + draw_normal(w, 0xC11F, key, w.year);
    Some([spread, half(total)])
}

/// A finished game against its line: the coaches' records against the
/// spread, and an upset judged by the line. `names` are home then away.
pub(crate) fn settle(
    w: &mut World,
    level: Level,
    line: [f32; 2],
    score: [u32; 2],
    coaches: [Option<usize>; 2],
    names: [String; 2],
    about: (Option<usize>, Option<usize>),
) {
    let margin = score[0] as f64 - score[1] as f64;
    let cover = margin + line[0] as f64;
    let res = if cover > 0.0 {
        [0, 1]
    } else if cover < 0.0 {
        [1, 0]
    } else {
        [2, 2]
    };
    for (side, c) in coaches.iter().enumerate() {
        if let Some(c) = c {
            w.finance.ats.entry(*c).or_default()[res[side]] += 1;
        }
    }
    // An upset by the line: the underdog wins outright.
    let spread = line[0] as f64;
    let dog_home = spread > 0.0;
    let dog_won = (dog_home && margin > 0.0) || (!dog_home && margin < 0.0);
    let big = if level == Level::Nfl { 7.0 } else { 14.0 };
    if dog_won && spread.abs() >= big {
        let (dog, fav) = if dog_home { (&names[0], &names[1]) } else { (&names[1], &names[0]) };
        let (ds, fs) = if dog_home { (score[0], score[1]) } else { (score[1], score[0]) };
        let huge = if level == Level::Nfl { 10.0 } else { 20.0 };
        let loud = if spread.abs() >= huge { Loudness::Big } else { Loudness::Medium };
        let text = format!("Upset: {dog}, {:.1}-point underdogs, beat {fav} {ds}-{fs}.", spread.abs());
        let mut e = story(w, level, loud, text).rule("betting: the line");
        if let Some(t) = about.0 {
            e = e.team(t);
        }
        if let Some(p) = about.1 {
            e = e.program(p);
        }
        super::post(w, e);
    }
}

/// A line in words: "Ohio State -7.5, O/U 52.5".
pub fn words(line: [f32; 2], fav_home: &str, fav_away: &str) -> String {
    let s = line[0];
    if s == 0.0 {
        return format!("Pick'em, O/U {:.1}", line[1]);
    }
    let (name, pts) = if s < 0.0 { (fav_home, -s) } else { (fav_away, s) };
    format!("{name} -{pts:.1}, O/U {:.1}", line[1])
}

/// A coach's record against the spread: covers, misses, pushes.
pub fn ats(w: &World, coach: usize) -> [u16; 3] {
    w.finance.ats.get(&coach).copied().unwrap_or_default()
}

/// This season's lines against the results so far, for the checks: games,
/// the mean miss in points, and the share the favorite won, NFL then college.
pub fn accuracy(w: &World) -> [(u32, f64, f64); 2] {
    let mut out = [(0u32, 0.0f64, 0.0f64); 2];
    let mut add = |k: usize, line: [f32; 2], score: [u32; 2]| {
        let margin = score[0] as f64 - score[1] as f64;
        out[k].0 += 1;
        out[k].1 += (margin + line[0] as f64).abs();
        if line[0] != 0.0 && (line[0] < 0.0) == (margin > 0.0) {
            out[k].2 += 1.0;
        }
    };
    if let Some(s) = w.nfl_season.as_ref() {
        for x in s.slots.iter().flatten() {
            if let (Some(l), Some(sc)) = (x.line, x.score) {
                add(0, l, sc);
            }
        }
    }
    if let Some(cs) = w.college.as_ref() {
        for g in &cs.games {
            if let (Some(l), Some(p)) = (g.line, g.played)
                && !p.cancelled
            {
                add(1, l, p.score);
            }
        }
    }
    for o in &mut out {
        if o.0 > 0 {
            o.1 /= o.0 as f64;
            o.2 /= o.0 as f64;
        }
    }
    out
}
