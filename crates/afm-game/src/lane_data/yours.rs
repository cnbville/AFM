//! Your numbers (Data overhaul item 41, board 45) and the next opponent's
//! report (item 33, which section 9's advance scouting reads):
//! - `your_numbers`: your team's rank in what decides games at your level
//!   (eight numbers, each with five seasons of percentiles), the verdict in
//!   plain words, what drives it (each line opens its page), this week's
//!   biggest movers and alerts on your own players;
//! - `next_opponent` (`kind`, `id` to read another team than the next
//!   one): its numbers beside yours, its tendencies, its best players by
//!   the analytics, the matchups that tilt and its last games.
//!
//! Numbers only, as every Data page: the squads and calls tallies and the
//! players' analytics, never a hidden rating.

use std::collections::HashMap;

use afm_engine::calendar::Date;
use afm_engine::world::World;
use afm_engine::world::analytics as an;
use afm_engine::world::marks::Tier;
use afm_engine::world::tally::squads::{self, Season as Sq};
use afm_engine::world::tally::teams::{add_adv, sub_adv};
use serde_json::{Value, json};

use super::teams::{self, Group, STATS, TStat, rank_of, stat_by, text};
use super::tier_key;
use crate::views::{self, TeamRef};

/// A tile: the number's key, its side (0 the offense, 1 the defense), its
/// heading and its words in a sentence.
type Tile = (&'static str, usize, &'static str, &'static str);

/// What decides games at each level (board 45; proposed, not confirmed:
/// the eight per level). Charted college reads the efficiency numbers
/// Bill Connelly's five factors name; the NFL the ones its analysts lead
/// with; box scores the plain ones.
const COLLEGE: [Tile; 8] = [
    ("TmEpa", 0, "Offense · EPA a play", "EPA a play"),
    ("TmSucc", 0, "Offense · success rate", "success rate"),
    ("TmExpl", 0, "Offense · explosive plays", "explosive plays"),
    ("TmPtsTrip", 0, "Points a scoring chance", "points a scoring chance"),
    ("TmEpa", 1, "Defense · EPA a play", "EPA a play allowed"),
    ("TmHavoc", 1, "Defense · havoc rate", "havoc"),
    ("TmToMargin", 0, "Turnover margin a game", "turnover margin"),
    ("TmStart", 0, "Field position · average start", "field position"),
];
const NFL: [Tile; 8] = [
    ("TmEpa", 0, "Offense · EPA a play", "EPA a play"),
    ("TmSucc", 0, "Offense · success rate", "success rate"),
    ("TmPassEpa", 0, "Offense · EPA a dropback", "EPA a dropback"),
    ("TmRz", 0, "Red zone touchdown rate", "red zone touchdowns"),
    ("TmEpa", 1, "Defense · EPA a play", "EPA a play allowed"),
    ("TmPress", 1, "Defense · pressure rate", "pressure rate"),
    ("TmToMargin", 0, "Turnover margin a game", "turnover margin"),
    ("TmThird", 1, "Defense · third downs", "third downs allowed"),
];
const BOX: [Tile; 8] = [
    ("TmPtsG", 0, "Points a game", "points a game"),
    ("TmYpp", 0, "Yards a play", "yards a play"),
    ("TmYpc", 0, "Rushing yards a carry", "yards a carry"),
    ("TmRz", 0, "Red zone touchdown rate", "red zone touchdowns"),
    ("TmPtsG", 1, "Points allowed a game", "points allowed"),
    ("TmYpp", 1, "Yards a play allowed", "yards a play allowed"),
    ("TmToMargin", 0, "Turnover margin a game", "turnover margin"),
    ("TmPenYdsG", 0, "Penalty yards a game", "penalty yards"),
];

/// The tiles at a level and charting depth; a number your charting
/// doesn't reach gives way to the nearest it does.
fn tiles(tier: Tier, depth: u8) -> Vec<Tile> {
    let base: &[Tile] = match (tier, depth) {
        (_, 0) | (Tier::Hs, _) => &BOX,
        (Tier::Nfl, _) => &NFL,
        (Tier::College, _) => &COLLEGE,
    };
    base.iter()
        .map(|&t| match stat_by(t.0) {
            Some(s) if s.depth > depth => ("TmHavoc", t.1, "Defense · havoc rate", "havoc"),
            _ => t,
        })
        .collect()
}

/// One number of a team's, read against its group.
#[derive(Clone, Copy)]
struct Read {
    v: f64,
    rank: Option<usize>,
    n: usize,
    /// 100 best in the group, 0 worst.
    pct: Option<f64>,
    avg: Option<f64>,
}

fn read(st: &TStat, side: usize, me: &Sq, g: &Group) -> Option<Read> {
    let v = (st.f)(me, side, &g.lv)?;
    let (rank, avg) = rank_of(st, side, g, v);
    let vals: Vec<f64> = g.rows.iter().filter_map(|s| (st.f)(s, side, &g.lv)).collect();
    let n = vals.len();
    // The share of the group below him, ties counted half, so a number
    // the whole group shares reads as the middle, not the top.
    let pct = rank.filter(|_| n > 1).map(|r| {
        let ties = vals.iter().filter(|&&x| (x - v).abs() <= 1e-9).count().max(1);
        let worse = n.saturating_sub(r + ties - 1);
        100.0 * (worse as f64 + 0.5 * (ties - 1) as f64) / (n - 1) as f64
    });
    Some(Read { v, rank, n, pct, avg })
}

fn ord(n: usize) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

fn read_json(st: &TStat, r: &Read) -> Value {
    json!({
        "text": text(st.fmt, r.v),
        "rank": r.rank,
        "n": r.n,
        "pct": r.pct.map(|p| p.round() as i64),
        "avg": r.avg.map(|a| text(st.fmt, a)),
    })
}

/// A team's squad row in its group that season, with the group.
fn team_group(w: &World, tier: Tier, team: usize, year: i32) -> Option<(Group, Sq)> {
    let g = teams::group(w, tier, team, year);
    let me = g.rows.iter().find(|s| s.team == team as u32).cloned()?;
    Some((g, me))
}

/// Where a team's Data numbers link: the situations page for the downs
/// and the scoring chances, the 4th down page, else its Data tab.
fn go_for(key: &str) -> &'static str {
    match key {
        "TmThird" | "TmFourth" | "TmRz" | "TmPtsTrip" | "TmTripsG" | "TmThreeOut" | "TmEarlyPass" | "TmProe" => {
            "situations"
        }
        _ => "team_data",
    }
}

/// The heading of a number on a side.
fn side_label(st: &TStat, side: usize) -> String {
    match st.unit {
        0 if side == 1 => format!("Defense · {}", st.label),
        0 => format!("Offense · {}", st.label),
        3 => format!("Special teams · {}", st.label),
        _ => st.label.to_string(),
    }
}

/// Every ranked number shown at a depth, with its sides.
fn ranked(depth: u8) -> Vec<(&'static TStat, usize)> {
    STATS
        .iter()
        .filter(|s| s.depth <= depth && s.up.is_some())
        .flat_map(|s| {
            let sides: &[usize] = if s.unit == 0 { &[0, 1] } else { &[0] };
            sides.iter().map(move |&d| (s, d))
        })
        .collect()
}

// ---------------------------------------------------------------------
// `your_numbers`.

pub fn your_numbers(w: &World, _args: &Value) -> Value {
    let Some(t) = views::mine(w) else { return json!({ "error": "You have no team yet." }) };
    let brief = views::team_brief(w, t);
    let Some((tier, team)) = teams::tier_of(t) else {
        return json!({ "team": brief, "error": "Another league's clubs keep no team data." });
    };
    let (years, first) = teams::team_years(w, tier, team);
    let Some(year) = first else {
        return json!({ "team": brief, "level": tier_key(tier), "error": "No game of yours has been counted yet." });
    };
    let depth = teams::depth_for(w, tier, team, year);
    let d = depth.unwrap_or(0);
    let tl = tiles(tier, d);
    let Some((g, me)) = team_group(w, tier, team, year) else {
        return json!({ "team": brief, "level": tier_key(tier), "error": "No game of yours has been counted yet." });
    };
    let reads: Vec<Option<Read>> = tl.iter().map(|t| stat_by(t.0).and_then(|s| read(s, t.1, &me, &g))).collect();

    // Five seasons, oldest first, ending with this one.
    let mut five: Vec<i32> = years.iter().copied().filter(|&y| y <= year).take(5).collect();
    five.reverse();
    let five_pcts: Vec<Vec<Option<i64>>> = five
        .iter()
        .map(|&y| {
            let pcts = |g: &Group, me: &Sq| -> Vec<Option<i64>> {
                tl.iter()
                    .map(|t| {
                        stat_by(t.0).and_then(|s| read(s, t.1, me, g)).and_then(|r| r.pct).map(|p| p.round() as i64)
                    })
                    .collect()
            };
            if y == year {
                pcts(&g, &me)
            } else {
                team_group(w, tier, team, y).map_or(vec![None; tl.len()], |(g, me)| pcts(&g, &me))
            }
        })
        .collect();

    let tiles_json: Vec<Value> = tl
        .iter()
        .zip(&reads)
        .enumerate()
        .map(|(k, (t, r))| {
            let st = stat_by(t.0);
            let mut v = json!({
                "label": t.2,
                "key": t.0,
                "side": t.1,
                "five": five_pcts.iter().map(|p| p[k]).collect::<Vec<_>>(),
            });
            if let (Some(st), Some(r)) = (st, r) {
                let r = read_json(st, r);
                for k in ["text", "rank", "n", "pct", "avg"] {
                    v[k] = r[k].clone();
                }
            }
            v
        })
        .collect();

    json!({
        "team": brief,
        "level": tier_key(tier),
        "year": year,
        "group": g.name,
        "n": g.rows.len(),
        "depth": depth,
        "charting": depth.map(afm_engine::world::analytics::depth_name),
        "source": source_line(tier, depth),
        "verdict": verdict(&tl, &reads, &g.name),
        "tiles": tiles_json,
        "five": {
            "years": five,
            "off": five_pcts.iter().map(|p| p[0]).collect::<Vec<_>>(),
            "def": five_pcts.iter().map(|p| p[4]).collect::<Vec<_>>(),
            "off_label": tl[0].2,
            "def_label": tl[4].2,
        },
        "drivers": drivers(w, tier, team, year, d, &tl, &me, &g),
        "movers": movers(w, tier, team, year, d, &g),
        "alerts": alerts(w, tier, team, year),
    })
}

fn source_line(tier: Tier, depth: Option<u8>) -> &'static str {
    match (tier, depth) {
        (Tier::Nfl, _) => "From the league's player tracking on every snap and your own charting staff.",
        (Tier::College, Some(2)) => "From your staff's charting of every game in your division, with the film.",
        (Tier::College, Some(_)) => "From your staff's charting of every game in your division.",
        (Tier::College, None) => "From the box scores: your staff doesn't chart this division.",
        (Tier::Hs, _) => "From the box scores the schools report.",
    }
}

/// The verdict in plain words: which side carries the team and where it
/// leaks (proposed, not confirmed: a side carries when its numbers sit 12
/// percentile points above the other's; a leak is a number in the bottom
/// third).
fn verdict(tl: &[Tile], reads: &[Option<Read>], group: &str) -> String {
    let at = |k: usize| reads.get(k).copied().flatten();
    let mean = |ks: &[usize]| {
        let v: Vec<f64> = ks.iter().filter_map(|&k| at(k).and_then(|r| r.pct)).collect();
        (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
    };
    let off_k = [0, 1, 2, 3];
    let def_k = [4, 5];
    let (Some(off), Some(def)) = (mean(&off_k), mean(&def_k)) else {
        return "Too few games counted to say yet.".into();
    };
    let best_two = |ks: &[usize]| -> String {
        let mut v: Vec<(usize, Read)> = ks.iter().filter_map(|&k| at(k).map(|r| (k, r))).collect();
        v.sort_by(|a, b| b.1.pct.unwrap_or(0.0).total_cmp(&a.1.pct.unwrap_or(0.0)));
        let words: Vec<String> =
            v.iter().take(2).filter_map(|(k, r)| r.rank.map(|n| format!("{} in {}", ord(n), tl[*k].3))).collect();
        words.join(" and ")
    };
    let mut s = if def - off >= 12.0 {
        format!("The defense wins your games: {}.", best_two(&def_k))
    } else if off - def >= 12.0 {
        format!("The offense carries you: {}.", best_two(&off_k))
    } else {
        format!(
            "Neither side carries you alone: the offense sits around the {} percentile of {group}, the defense around the {}.",
            ord(off.round() as usize),
            ord(def.round() as usize)
        )
    };
    let worst =
        (0..tl.len()).filter_map(|k| at(k).and_then(|r| Some((k, r, r.pct?)))).min_by(|a, b| a.2.total_cmp(&b.2));
    match worst {
        Some((k, r, p)) if p < 33.0 => {
            if let Some(rank) = r.rank {
                s.push_str(&format!(" The leak is {}: {} of {}.", tl[k].3, ord(rank), r.n));
            }
        }
        Some(_) => s.push_str(&format!(" None of these numbers sits in the bottom third of {group}.")),
        None => {}
    }
    s
}

/// What drives it: the two numbers of yours that stand out most each way
/// beyond the tiles, and your 4th down calls against the analysts' model.
#[allow(clippy::too_many_arguments)]
fn drivers(w: &World, tier: Tier, team: usize, year: i32, d: u8, tl: &[Tile], me: &Sq, g: &Group) -> Vec<Value> {
    let mut all: Vec<(&TStat, usize, Read)> = ranked(d)
        .into_iter()
        .filter(|(s, side)| !tl.iter().any(|t| t.0 == s.key && t.1 == *side))
        .filter_map(|(s, side)| read(s, side, me, g).filter(|r| r.pct.is_some() && r.n >= 3).map(|r| (s, side, r)))
        .collect();
    all.sort_by(|a, b| b.2.pct.unwrap_or(0.0).total_cmp(&a.2.pct.unwrap_or(0.0)));
    let line = |(s, side, r): &(&TStat, usize, Read), good: bool| {
        json!({
            "h": side_label(s, *side),
            "key": s.key,
            "t": format!(
                "{}, {} of {}; the {} average is {}",
                text(s.fmt, r.v),
                ord(r.rank.unwrap_or(0)),
                r.n,
                g.name,
                r.avg.map_or("-".into(), |a| text(s.fmt, a))
            ),
            "good": good,
            "go": go_for(s.key),
        })
    };
    let mut out: Vec<Value> = all.iter().take(2).map(|x| line(x, true)).collect();
    let n = all.len();
    out.extend(all.iter().skip(n.saturating_sub(2).max(2)).rev().map(|x| line(x, false)));
    // The 4th downs, where the calls tally keeps your games.
    if let Some(c) = w.tally_call(teams::level_of(tier), year, team as u32).filter(|c| c.sum.weighed > 0) {
        let s = &c.sum;
        out.push(json!({
            "h": "4th downs",
            "t": format!(
                "You went for it {} of the {} time{} the model called a clear go; {} of your {} calls were its best or within a point of it, and together they gave up {:.2} wins in win chance",
                s.went_said_go,
                s.said_go,
                if s.said_go == 1 { "" } else { "s" },
                s.agreed,
                s.weighed,
                s.lost
            ),
            "good": s.agreed * 4 >= s.weighed * 3,
            "go": "decisions",
        }));
    }
    out
}

/// One team's games this week, summed.
#[derive(Default)]
struct Week {
    games: u32,
    wins: u32,
    losses: u32,
    pf: u32,
    pa: u32,
    boxes: [Vec<f32>; 2],
    adv: afm_engine::world::tally::games::Adv,
    adv_against: afm_engine::world::tally::games::Adv,
}

/// A team's season before this week's games.
fn before(s: &Sq, wk: Option<&Week>) -> Sq {
    let mut b = s.clone();
    let Some(wk) = wk else { return b };
    b.games = b.games.saturating_sub(wk.games);
    b.wins = b.wins.saturating_sub(wk.wins);
    b.losses = b.losses.saturating_sub(wk.losses);
    b.points_for = b.points_for.saturating_sub(wk.pf);
    b.points_against = b.points_against.saturating_sub(wk.pa);
    for k in 0..2 {
        for (x, y) in b.boxes[k].iter_mut().zip(&wk.boxes[k]) {
            *x -= y;
        }
    }
    sub_adv(&mut b.adv, &wk.adv);
    sub_adv(&mut b.adv_against, &wk.adv_against);
    b
}

/// This week's biggest movers: your numbers whose rank in the group moved
/// most with the last seven days' games (college and the NFL, the season
/// being played; the numbers the box and the advanced box hold).
fn movers(w: &World, tier: Tier, team: usize, year: i32, d: u8, g: &Group) -> Vec<Value> {
    if tier == Tier::Hs || year != w.tally_open_season() {
        return Vec::new();
    }
    let since = Date(w.today.0 - 7);
    let mut weeks: HashMap<u32, Week> = HashMap::new();
    for row in w.tally_games_since(teams::level_of(tier), since) {
        if row.season != year {
            continue;
        }
        for side in 0..2 {
            let wk = weeks.entry(row.teams[side]).or_default();
            wk.games += 1;
            let (a, b) = (row.score[side], row.score[1 - side]);
            wk.wins += (a > b) as u32;
            wk.losses += (a < b) as u32;
            wk.pf += a.max(0) as u32;
            wk.pa += b.max(0) as u32;
            squads::add_vec(&mut wk.boxes[0], &squads::box_values(&row.boxes[side]));
            squads::add_vec(&mut wk.boxes[1], &squads::box_values(&row.boxes[1 - side]));
            add_adv(&mut wk.adv, &row.adv[side]);
            add_adv(&mut wk.adv_against, &row.adv[1 - side]);
        }
    }
    if !weeks.contains_key(&(team as u32)) {
        return Vec::new();
    }
    let rows: Vec<Sq> = g.rows.iter().map(|s| before(s, weeks.get(&s.team))).filter(|s| s.games > 0).collect();
    let lv = teams::level_numbers(&rows);
    let old = Group { rows, lv, name: g.name.clone() };
    let Some(me_now) = g.rows.iter().find(|s| s.team == team as u32) else { return Vec::new() };
    let Some(me_old) = old.rows.iter().find(|s| s.team == team as u32) else { return Vec::new() };
    // The kicking, the turnover luck and the calls by situation aren't in a
    // game's record: those numbers stay out.
    let skip = ["TmXToMargin", "TmToLuck", "TmProe", "TmEarlyPass"];
    let mut v: Vec<(i64, Value)> = ranked(d)
        .into_iter()
        .filter(|(s, _)| s.unit != 3 && !skip.contains(&s.key))
        .filter_map(|(s, side)| {
            let now = read(s, side, me_now, g)?;
            let was = read(s, side, me_old, &old)?;
            let (rn, rw) = (now.rank? as i64, was.rank? as i64);
            (rn != rw).then(|| {
                (
                    (rw - rn).abs(),
                    json!({
                        "label": side_label(s, side),
                        "key": s.key,
                        "from": rw,
                        "to": rn,
                        "n": now.n,
                        "was": text(s.fmt, was.v),
                        "now": text(s.fmt, now.v),
                        "up": rn < rw,
                    }),
                )
            })
        })
        .collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.0));
    v.into_iter().take(6).map(|x| x.1).collect()
}

/// The analytics table's arguments for a team's players at its level.
fn table_args(w: &World, tier: Tier, team: usize, year: i32, group: usize) -> Value {
    let mut a = json!({ "level": tier_key(tier), "group": an::GROUPS[group], "team": team, "year": year, "limit": 60 });
    match tier {
        Tier::College => a["division"] = json!(super::season_place(w, team, year).0.name()),
        Tier::Hs => a["state"] = json!(w.high_schools[team].state),
        Tier::Nfl => {}
    }
    a
}

/// A team's qualified players in a group with each one's percentiles,
/// and the columns.
struct Players {
    cols: Vec<Value>,
    rows: Vec<Value>,
}

fn players(w: &World, tier: Tier, team: usize, year: i32, group: usize) -> Players {
    let v = super::analytics::table(w, &table_args(w, tier, team, year, group));
    let cols = v.get("columns").and_then(Value::as_array).cloned().unwrap_or_default();
    let rows = v.get("rows").and_then(Value::as_array).cloned().unwrap_or_default();
    Players { cols, rows }
}

/// A group for the matchups: its qualified men, or, where fewer than two
/// qualify (early in a season, or a line that rotates), the men with half
/// the table's minimum (proposed, not confirmed).
fn group_men(w: &World, tier: Tier, team: usize, year: i32, group: usize) -> Players {
    let mut args = table_args(w, tier, team, year, group);
    let v = super::analytics::table(w, &args);
    let rows = v.get("rows").and_then(Value::as_array).cloned().unwrap_or_default();
    let min = v.get("min").and_then(Value::as_f64).unwrap_or(0.0);
    let v = if rows.len() < 2 && min > 0.0 {
        args["min"] = json!((min / 2.0).round());
        super::analytics::table(w, &args)
    } else {
        v
    };
    let cols = v.get("columns").and_then(Value::as_array).cloned().unwrap_or_default();
    let rows = v.get("rows").and_then(Value::as_array).cloned().unwrap_or_default();
    Players { cols, rows }
}

/// A player's numbers that rank, as (column, percentile), best first.
fn ranked_cells(p: &Players, row: &Value) -> Vec<(usize, i64)> {
    let pct = row.get("pct").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut v: Vec<(usize, i64)> = pct
        .iter()
        .enumerate()
        .filter(|(k, _)| p.cols.get(*k).and_then(|c| c.get("better")).and_then(Value::as_str) != Some("none"))
        .filter_map(|(k, x)| x.as_i64().map(|x| (k, x)))
        .collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    v
}

fn cell_words(p: &Players, row: &Value, k: usize, pct: i64) -> Value {
    let col = p.cols.get(k).cloned().unwrap_or(Value::Null);
    let raw = row.get("raw").or(row.get("cells")).and_then(|c| c.get(k)).cloned().unwrap_or(Value::Null);
    json!({ "label": col.get("label"), "key": col.get("key"), "value": raw, "pct": pct })
}

fn player_head(row: &Value) -> Value {
    json!({
        "id": row.get("id"),
        "name": row.get("name"),
        "linked": row.get("linked"),
        "pos": row.get("pos"),
        "games": row.get("games"),
    })
}

/// Alerts on your own players (proposed, not confirmed: a qualified man at
/// the 90th percentile or above in a number that ranks stands out, at the
/// 10th or below he is a worry; the strongest of each man's, at most
/// twelve, the kickers, punters and returners left out).
fn alerts(w: &World, tier: Tier, team: usize, year: i32) -> Vec<Value> {
    let mut out: Vec<(i64, Value)> = Vec::new();
    for group in 0..an::GROUPS.len() {
        if matches!(an::GROUPS[group], "K" | "P" | "RET") {
            continue;
        }
        let p = players(w, tier, team, year, group);
        for row in &p.rows {
            let cells = ranked_cells(&p, row);
            let (Some(top), Some(low)) = (cells.first().copied(), cells.last().copied()) else { continue };
            let pick = if top.1 >= 90 && (100 - top.1) <= low.1 {
                Some((top, true))
            } else if low.1 <= 10 {
                Some((low, false))
            } else if top.1 >= 90 {
                Some((top, true))
            } else {
                None
            };
            let Some(((k, pct), up)) = pick else { continue };
            let strength = if up { pct } else { 100 - pct };
            let mut v = player_head(row);
            v["group"] = json!(an::GROUPS[group]);
            v["up"] = json!(up);
            v["cell"] = cell_words(&p, row, k, pct);
            out.push((strength, v));
        }
    }
    out.sort_by_key(|x| std::cmp::Reverse(x.0));
    out.into_iter().take(12).map(|x| x.1).collect()
}

// ---------------------------------------------------------------------
// `next_opponent`.

/// Matchups that tilt: your group against theirs (proposed, not
/// confirmed: a group's number is its qualified men's mean percentile
/// over the numbers that rank; a gap of 15 or more tilts).
const MATCHUPS: [(&str, &str, &str); 6] = [
    ("EDGE", "OL", "Your edge rushers against their line"),
    ("DL", "OL", "Your interior line against their line"),
    ("CB", "WR", "Your corners against their receivers"),
    ("OL", "EDGE", "Your line against their edge rushers"),
    ("OL", "DL", "Your line against their interior line"),
    ("WR", "CB", "Your receivers against their corners"),
];

fn group_index(g: &str) -> usize {
    an::GROUPS.iter().position(|x| *x == g).unwrap_or(0)
}

/// A group's mean percentile over its men's ranking numbers.
fn group_score(p: &Players) -> Option<f64> {
    let per: Vec<f64> = p
        .rows
        .iter()
        .filter_map(|r| {
            let c = ranked_cells(p, r);
            (!c.is_empty()).then(|| c.iter().map(|x| x.1 as f64).sum::<f64>() / c.len() as f64)
        })
        .collect();
    (!per.is_empty()).then(|| per.iter().sum::<f64>() / per.len() as f64)
}

/// Your next opponent, and the game's facts.
fn next_opp(w: &World, me: TeamRef) -> Option<(TeamRef, Value)> {
    let g = views::next_game(w, me)?;
    let t = TeamRef::from_args(w, &g["opp"]["ref"]).ok()?;
    Some((t, g))
}

/// Their last games with the scores (college and the NFL).
fn last_games(w: &World, t: TeamRef) -> Vec<Value> {
    let mut v: Vec<(i32, Value)> = match t {
        TeamRef::College(i) => w
            .college_fixtures(Some(i))
            .into_iter()
            .filter_map(|g| {
                let s = g.score?;
                let home = g.home == i;
                let opp = if home { g.away } else { g.home };
                let (a, b) = if home { (s[0], s[1]) } else { (s[1], s[0]) };
                Some((
                    g.date.0,
                    json!({ "date": views::date_text(g.date), "opp": views::team_brief(w, TeamRef::College(opp)), "home": home, "score": [a, b] }),
                ))
            })
            .collect(),
        TeamRef::Nfl(i) => w
            .nfl_fixtures(Some(i))
            .into_iter()
            .filter(|g| g.played)
            .filter_map(|g| {
                let s = g.score?;
                let home = g.home == i;
                let opp = if home { g.away } else { g.home };
                let (a, b) = if home { (s[0], s[1]) } else { (s[1], s[0]) };
                Some((
                    g.when.map_or(g.week as i32, |d| d.0),
                    json!({ "date": g.when.map(views::date_text), "week": g.week, "opp": views::team_brief(w, TeamRef::Nfl(opp)), "home": home, "score": [a, b] }),
                ))
            })
            .collect(),
        _ => Vec::new(),
    };
    v.sort_by_key(|x| std::cmp::Reverse(x.0));
    v.into_iter().take(4).map(|x| x.1).collect()
}

pub fn next_opponent(w: &World, args: &Value) -> Value {
    let Some(mine) = views::mine(w) else { return json!({ "error": "You have no team yet." }) };
    let Some((tier, team)) = teams::tier_of(mine) else {
        return json!({ "team": views::team_brief(w, mine), "error": "Another league's clubs keep no team data." });
    };
    let (opp, game) = if args.get("kind").is_some() {
        match TeamRef::from_args(w, args) {
            Ok(t) => (t, Value::Null),
            Err(e) => return json!({ "error": e }),
        }
    } else {
        match next_opp(w, mine) {
            Some(x) => x,
            None => return json!({ "team": views::team_brief(w, mine), "error": "No game left on your schedule." }),
        }
    };
    let Some((otier, oteam)) = teams::tier_of(opp).filter(|x| x.0 == tier) else {
        return json!({ "team": views::team_brief(w, mine), "opp": views::team_brief(w, opp), "error": "No numbers kept for that team." });
    };
    let (years, first) = teams::team_years(w, otier, oteam);
    let year = first.or(years.first().copied()).unwrap_or(w.year);
    let depth = teams::depth_for(w, tier, oteam, year);
    let d = depth.unwrap_or(0);
    let mut out = json!({
        "team": views::team_brief(w, mine),
        "opp": views::team_brief(w, opp),
        "game": game,
        "level": tier_key(tier),
        "year": year,
        "depth": depth,
        "charting": depth.map(afm_engine::world::analytics::depth_name),
        "source": source_line(tier, depth),
        "last": last_games(w, opp),
    });
    let Some((og, them)) = team_group(w, tier, oteam, year) else {
        out["error"] = json!("No game of theirs has been counted this season.");
        return out;
    };
    out["group"] = json!(og.name);
    // Yours, against your own group that season.
    let yours = team_group(w, tier, team, year);
    let both: Vec<&TStat> = STATS.iter().filter(|s| s.unit == 0 && s.depth <= d).collect();
    let side_rows = |att: (&Sq, &Group), def: (&Sq, &Group)| -> Vec<Value> {
        both.iter()
            .map(|s| {
                let a = read(s, 0, att.0, att.1);
                let b = read(s, 1, def.0, def.1);
                // The edge: who sits higher in his own group.
                let edge = match (a.and_then(|r| r.pct), b.and_then(|r| r.pct)) {
                    (Some(x), Some(y)) if x - y >= 25.0 => "off",
                    (Some(x), Some(y)) if y - x >= 25.0 => "def",
                    _ => "",
                };
                json!({
                    "label": s.label,
                    "key": s.key,
                    "off": a.map(|r| read_json(s, &r)),
                    "def": b.map(|r| read_json(s, &r)),
                    "edge": edge,
                })
            })
            .collect()
    };
    if let Some((yg, me)) = &yours {
        out["sides"] = json!([
            { "name": "When you have the ball", "heads": ["Your offense", "Their defense"], "rows": side_rows((me, yg), (&them, &og)) },
            { "name": "When they have the ball", "heads": ["Their offense", "Your defense"], "rows": side_rows((&them, &og), (me, yg)) },
        ]);
    }
    // Their whole-team and special teams numbers.
    let team_side: Vec<Value> = STATS
        .iter()
        .filter(|s| s.unit != 0 && s.depth <= d)
        .filter_map(|s| {
            read(s, 0, &them, &og).map(|r| json!({ "label": side_label(s, 0), "key": s.key, "v": read_json(s, &r) }))
        })
        .collect();
    out["team_side"] = json!(team_side);
    if d >= 1 && tier != Tier::Hs {
        out["tendencies"] = teams::tendencies(w, tier, oteam, year, &them, &og, d);
    }
    // Their best men by the analytics, two a group.
    let shown: Vec<usize> = (0..an::GROUPS.len()).filter(|&gi| !matches!(an::GROUPS[gi], "K" | "P" | "RET")).collect();
    let mut cache: HashMap<(usize, usize), Players> = HashMap::new();
    for &gi in &shown {
        cache.insert((oteam, gi), group_men(w, tier, oteam, year, gi));
    }
    for (mg, _, _) in MATCHUPS {
        let gi = group_index(mg);
        cache.entry((team, gi)).or_insert_with(|| group_men(w, tier, team, year, gi));
    }
    let get = |t: usize, gi: usize| -> &Players { &cache[&(t, gi)] };
    let mut best = Vec::new();
    for &gi in &shown {
        let p = get(oteam, gi);
        let mut rows: Vec<(f64, Value)> = p
            .rows
            .iter()
            .filter_map(|r| {
                let c = ranked_cells(p, r);
                let mean = (!c.is_empty()).then(|| c.iter().map(|x| x.1 as f64).sum::<f64>() / c.len() as f64)?;
                let mut v = player_head(r);
                v["mean"] = json!(mean.round() as i64);
                v["best"] = json!(c.iter().take(2).map(|&(k, pct)| cell_words(p, r, k, pct)).collect::<Vec<_>>());
                Some((mean, v))
            })
            .collect();
        rows.sort_by(|a, b| b.0.total_cmp(&a.0));
        if !rows.is_empty() {
            best.push(
                json!({ "group": an::GROUPS[gi], "rows": rows.into_iter().take(2).map(|x| x.1).collect::<Vec<_>>() }),
            );
        }
    }
    out["players"] = json!(best);
    // The matchups that tilt.
    let mut tilt = Vec::new();
    for (mg, tg, label) in MATCHUPS {
        let a = group_score(get(team, group_index(mg)));
        let b = group_score(get(oteam, group_index(tg)));
        let (Some(a), Some(b)) = (a, b) else { continue };
        let way = if a - b >= 15.0 {
            "yours"
        } else if b - a >= 15.0 {
            "theirs"
        } else {
            "even"
        };
        tilt.push(json!({
            "label": label,
            "mine": mg,
            "theirs": tg,
            "a": a.round() as i64,
            "b": b.round() as i64,
            "way": way,
            "text": format!(
                "Your {mg} group sits at the {} percentile on average, their {tg} group at the {}: {}.",
                ord(a.round() as usize),
                ord(b.round() as usize),
                match way {
                    "yours" => "it tilts your way",
                    "theirs" => "it tilts their way",
                    _ => "an even fight",
                }
            ),
        }));
    }
    out["matchups"] = json!(tilt);
    out
}
