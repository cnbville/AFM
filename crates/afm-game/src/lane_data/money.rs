//! Money (Data overhaul item 40, board 50), read from the money tally
//! (`world::tally::money`; the season being played from the world as it
//! stands):
//! - an NFL club: the cap by position group with each group's results,
//!   the cap used, cash a win, dead money, the most money for the least
//!   back, cash against wins over three seasons, and the best and worst
//!   value for the cap (OverTheCap's way);
//! - a college program: NIL and the school's revenue share by position
//!   group (the two pots, B18), the collective's budget, paid a win, money
//!   against wins, and the best and worst value for the money;
//! - a high school: its budget line by line against the books so far, each
//!   line ranked in its state's class, the fee to play and the crowd.
//!
//! `money_data` (`level`, `year`, `kind` and `id` for another team).
//! Another program's money is what its reported deals add up to (the
//! tally's rows), marked estimated.

use std::collections::HashMap;

use afm_engine::positions::Position;
use afm_engine::world::World;
use afm_engine::world::finance::hs as fhs;
use afm_engine::world::marks::Tier;
use afm_engine::world::tally::Level;
use afm_engine::world::tally::money::{Pay, Row};
use afm_engine::world::tally::squads::Season as Sq;
use serde_json::{Value, json};

use super::teams::{self, Group, stat_by, text};
use super::tier_key;
use crate::views::{self, TeamRef};

const GROUPS: [&str; 11] = ["QB", "RB", "WR", "TE", "OL", "IDL", "EDGE", "LB", "CB", "S", "K/P"];

fn group_of(p: Position) -> usize {
    match p {
        Position::QB => 0,
        Position::RB => 1,
        Position::WR => 2,
        Position::TE => 3,
        Position::OT | Position::IOL => 4,
        Position::DT => 5,
        Position::EDGE => 6,
        Position::LB => 7,
        Position::CB => 8,
        Position::S => 9,
        _ => 10,
    }
}

/// Each group's result: the team number that best shows what it does
/// (proposed, not confirmed: the list), its side, and whether your
/// charting reads it.
const RESULTS: [(&str, usize); 11] = [
    ("TmPassEpa", 0),
    ("TmRunSucc", 0),
    ("TmPassSucc", 0),
    ("TmExpl", 0),
    ("TmSackRate", 0),
    ("TmStuff", 1),
    ("TmPress", 1),
    ("TmRunEpa", 1),
    ("TmPassEpa", 1),
    ("TmExpl", 1),
    ("TmFgPct", 0),
];

/// The box score's stand-in where your charting doesn't reach.
fn box_result(k: usize) -> (&'static str, usize) {
    match k {
        0 => ("TmPassG", 0),
        1 => ("TmYpc", 0),
        2 | 3 => ("TmPassG", 0),
        4 => ("TmSackRate", 0),
        5 => ("TmYpc", 1),
        6 => ("TmSackRate", 1),
        7 => ("TmRushG", 1),
        8 | 9 => ("TmPassG", 1),
        _ => ("TmFgPct", 0),
    }
}

fn money_text(v: f64) -> String {
    if v.abs() >= 1e6 {
        format!("${:.1}M", v / 1e6)
    } else if v.abs() >= 1e3 {
        format!("${:.0}K", v / 1e3)
    } else {
        format!("${v:.0}")
    }
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    v.sort_by(f64::total_cmp);
    (!v.is_empty()).then(|| v[v.len() / 2])
}

/// A player's money: an NFL cap hit; at college the school's share and the
/// collective's deals (brand deals are his own, left out).
fn paid(level: Level, p: &Pay) -> f64 {
    match level {
        Level::Nfl => p.a as f64,
        _ => (p.a + p.b) as f64,
    }
}

/// An NFL club's cash for a league year: the books' total once the year is
/// paid out (at its close), else what its contracts pay that year.
fn nfl_cash(r: &Row) -> f64 {
    let c = r.val("cash");
    if c > 0.0 { c } else { r.pay.iter().map(|p| p.b as f64).sum() }
}

fn rank_desc(vals: &[(u32, f64)], team: u32) -> Option<usize> {
    let mine = vals.iter().find(|x| x.0 == team)?.1;
    Some(1 + vals.iter().filter(|x| x.1 > mine + 1e-9).count())
}

pub fn money_data(w: &World, args: &Value) -> Value {
    let t = match TeamRef::from_args(w, args) {
        Ok(t) => t,
        Err(e) => return json!({ "error": e }),
    };
    let Some((tier, team)) = teams::tier_of(t) else {
        return json!({ "team": views::team_brief(w, t), "error": "Another league's clubs keep no money data here." });
    };
    let level = teams::level_of(tier);
    let mut years: Vec<i32> = w.tally_seasons();
    years.sort_unstable_by(|a, b| b.cmp(a));
    let year =
        args.get("year").and_then(Value::as_i64).map(|y| y as i32).filter(|y| years.contains(y)).unwrap_or_else(|| {
            // The season being played, or the last one until its first game.
            let open = w.tally_open_season();
            let played = w.tally_squad(level, open, team as u32).is_some_and(|s| s.games > 0);
            if played || !years.contains(&(open - 1)) { open } else { open - 1 }
        });
    let mut out = json!({
        "team": views::team_brief(w, t),
        "level": tier_key(tier),
        "year": year,
        "years": years,
        "mine": views::is_mine(w, t),
    });
    let rows = w.tally_money_level(level, year);
    let Some(me) = rows.iter().find(|r| r.team == team as u32).cloned() else {
        out["error"] = json!("No money kept for this team that season.");
        return out;
    };
    let extra = match tier {
        Tier::Hs => hs_money(w, team, year, &rows, &me),
        _ => pro_college(w, tier, team, year, &rows, &me),
    };
    if let (Some(o), Value::Object(m)) = (out.as_object_mut(), extra) {
        o.extend(m);
    }
    out
}

/// The teams a club or program is ranked among: the NFL, or its
/// conference that season (its division for an independent).
fn peers(w: &World, tier: Tier, team: usize, year: i32, rows: &[Row]) -> (Vec<u32>, String) {
    match tier {
        Tier::College => {
            let (d, conf) = super::season_place(w, team, year);
            let conf = conf.to_string();
            let in_conf = |p: u32| super::season_place(w, p as usize, year).1 == conf;
            let in_div = |p: u32| super::season_place(w, p as usize, year).0 == d;
            // An independent (or a conference too small to rank in) is
            // ranked in its division.
            let ids: Vec<u32> = rows.iter().map(|r| r.team).filter(|&p| in_conf(p)).collect();
            if conf.is_empty() || conf.contains("Independent") || ids.len() < 6 {
                (rows.iter().map(|r| r.team).filter(|&p| in_div(p)).collect(), d.name().to_string())
            } else {
                (ids, conf)
            }
        }
        _ => (rows.iter().map(|r| r.team).collect(), "the NFL".into()),
    }
}

#[allow(clippy::too_many_arguments)]
fn pro_college(w: &World, tier: Tier, team: usize, year: i32, rows: &[Row], me: &Row) -> Value {
    let level = teams::level_of(tier);
    let (ids, among) = peers(w, tier, team, year, rows);
    let peer_rows: Vec<&Row> = rows.iter().filter(|r| ids.contains(&r.team)).collect();
    // Money by group, each team.
    let by_group = |r: &Row| -> [f64; 11] {
        let mut g = [0.0; 11];
        for p in &r.pay {
            if let Some(pos) = p.pos {
                g[group_of(pos)] += paid(level, p);
            }
        }
        g
    };
    let groups_all: Vec<(u32, [f64; 11])> = peer_rows.iter().map(|r| (r.team, by_group(r))).collect();
    let mine_g = by_group(me);
    // Results: the peers' squads that season.
    let g = Group { rows: teams::rows_for(w, level, year, &ids), lv: Default::default(), name: among.clone() };
    let g = Group { lv: teams::level_numbers(&g.rows), ..g };
    let my_sq: Option<&Sq> = g.rows.iter().find(|s| s.team == team as u32);
    let depth = teams::depth_for(w, tier, team, year).unwrap_or(0);
    let names = |id: u32| afm_engine::world::marks::name_of(w, id).unwrap_or_default();
    let mut gap_best: Option<(i64, usize, usize, usize)> = None;
    let group_rows: Vec<Value> = (0..GROUPS.len())
        .map(|k| {
            let vals: Vec<(u32, f64)> = groups_all.iter().map(|x| (x.0, x.1[k])).collect();
            let money_rank = rank_desc(&vals, team as u32);
            let (key, side) = match stat_by(RESULTS[k].0) {
                Some(s) if s.depth <= depth => RESULTS[k],
                _ => box_result(k),
            };
            let st = stat_by(key);
            let res = st.zip(my_sq).and_then(|(st, me)| {
                let v = (st.f)(me, side, &g.lv)?;
                let (rank, _) = teams::rank_of(st, side, &g, v);
                Some((format!("{} {}", st.label, text(st.fmt, v)), rank))
            });
            if let (Some(m), Some((_, Some(r)))) = (money_rank, &res) {
                let gap = *r as i64 - m as i64;
                if gap_best.is_none_or(|b| gap > b.0) {
                    gap_best = Some((gap, k, m, *r));
                }
            }
            let biggest = me
                .pay
                .iter()
                .filter(|p| p.pos.map(group_of) == Some(k))
                .max_by(|a, b| paid(level, a).total_cmp(&paid(level, b)));
            let (school, collective) = me
                .pay
                .iter()
                .filter(|p| p.pos.map(group_of) == Some(k))
                .fold((0.0, 0.0), |a, p| (a.0 + p.a as f64, a.1 + p.b as f64));
            json!({
                "group": GROUPS[k],
                "money": money_text(mine_g[k]),
                "raw": mine_g[k],
                "money_rank": money_rank,
                "result": res.as_ref().map(|r| r.0.clone()),
                "result_key": key,
                "result_rank": res.and_then(|r| r.1),
                "biggest": biggest.map(|p| json!({ "id": p.id, "name": names(p.id), "money": money_text(paid(level, p)) })),
                "pots": (level == Level::College).then(|| json!({ "school": money_text(school), "collective": money_text(collective) })),
            })
        })
        .collect();
    // Tiles.
    let wins = |t: u32| g.rows.iter().find(|s| s.team == t).map(|s| s.wins);
    let my_wins = wins(team as u32).unwrap_or(0);
    let total_paid: f64 = mine_g.iter().sum();
    // A season in progress pays its year's money over the games so far.
    let season_games = afm_engine::world::marks::season_games(w, tier, year).max(1) as f64;
    let per_win = |r: &Row| -> Option<f64> {
        let s = g.rows.iter().find(|s| s.team == r.team)?;
        let money = if level == Level::Nfl { nfl_cash(r) } else { r.val("paid") };
        let share = (s.games as f64 / season_games).min(1.0);
        (s.wins > 0).then(|| money * share / s.wins as f64)
    };
    let med_per_win = median(peer_rows.iter().filter_map(|r| per_win(r)).collect());
    let most = gap_best.filter(|b| b.0 > 0).map(|(_, k, m, r)| {
        json!({ "group": GROUPS[k], "text": format!("{} in money, {} in {}", ord(m), ord(r), "results") })
    });
    let tiles = if level == Level::Nfl {
        let cap = me.val("cap") + me.val("carryover");
        let dead = me.val("dead");
        let med_dead = median(peer_rows.iter().map(|r| r.val("dead") / r.val("cap").max(1.0)).collect());
        json!([
            { "label": "Cap used", "value": money_text(me.val("charges")), "sub": format!("of {}; {} to spare", money_text(cap), money_text(cap - me.val("charges"))) },
            { "label": "Cash a win", "value": per_win(me).map(money_text), "sub": format!("{my_wins} wins; the NFL's middle {}", med_per_win.map_or("-".into(), money_text)) },
            { "label": "Most money, least back", "value": most.as_ref().map(|m| m["group"].clone()), "sub": most.as_ref().map(|m| m["text"].clone()) },
            if dead >= 0.0 {
                json!({ "label": "Dead money", "value": money_text(dead), "sub": format!("{:.1}% of the cap; the NFL's middle {}", 100.0 * dead / me.val("cap").max(1.0), med_dead.map_or("-".into(), |x| format!("{:.1}%", 100.0 * x))) })
            } else {
                // Unearned incentives credited back outweigh the charges.
                json!({ "label": "Dead money", "value": "None", "sub": format!("a {} credit from unearned incentives", money_text(-dead)) })
            },
        ])
    } else {
        json!([
            { "label": "Collective budget", "value": money_text(me.val("budget")), "sub": format!("revenue share {}, boosters {}", money_text(me.val("school")), money_text(me.val("boosters"))) },
            { "label": "Paid to players", "value": money_text(total_paid), "sub": format!("{:.0}% of the budget, {} players", 100.0 * total_paid / me.val("budget").max(1.0), me.pay.len()) },
            { "label": "Paid a win", "value": per_win(me).map(money_text), "sub": format!("{my_wins} wins; {}'s middle {}, estimated", among, med_per_win.map_or("-".into(), money_text)) },
            { "label": "Most money, least back", "value": most.as_ref().map(|m| m["group"].clone()), "sub": most.as_ref().map(|m| m["text"].clone()) },
        ])
    };
    json!({
        "among": among,
        "of": ids.len(),
        "total": money_text(if level == Level::Nfl { me.val("cap") } else { me.val("budget") }),
        "groups": group_rows,
        "tiles": tiles,
        "scatter": scatter(w, tier, team, year, &ids),
        "value": value(w, level, year, me),
    })
}

fn ord(n: usize) -> String {
    let s = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

/// Money against wins over the last three seasons: each peer a dot (its
/// money a year, its win share).
fn scatter(w: &World, tier: Tier, team: usize, year: i32, ids: &[u32]) -> Value {
    let level = teams::level_of(tier);
    let mut sums: HashMap<u32, (f64, u32, u32, u32)> = HashMap::new();
    let ys: Vec<i32> = w.tally_seasons().into_iter().filter(|&y| y <= year && y > year - 3).collect();
    for &y in &ys {
        let rows = w.tally_money_level(level, y);
        let sq = teams::rows_for(w, level, y, ids);
        for r in rows.iter().filter(|r| ids.contains(&r.team)) {
            let Some(s) = sq.iter().find(|s| s.team == r.team) else { continue };
            let money = if level == Level::Nfl { nfl_cash(r) } else { r.val("budget") };
            let e = sums.entry(r.team).or_default();
            e.0 += money;
            e.1 += 1;
            e.2 += s.wins;
            e.3 += s.games;
        }
    }
    let dots: Vec<Value> = sums
        .iter()
        .filter(|(_, v)| v.1 > 0 && v.3 > 0)
        .map(|(&t, v)| {
            json!({
                "team": super::team_json(w, tier, t as usize),
                "money": v.0 / v.1 as f64,
                "share": v.2 as f64 / v.3 as f64,
                "mine": t == team as u32,
            })
        })
        .collect();
    // The fit's correlation, for the caption.
    let n = dots.len() as f64;
    let xs: Vec<f64> = dots.iter().filter_map(|d| d["money"].as_f64()).collect();
    let ys_: Vec<f64> = dots.iter().filter_map(|d| d["share"].as_f64()).collect();
    let r = (n >= 3.0).then(|| {
        let mx = xs.iter().sum::<f64>() / n;
        let my = ys_.iter().sum::<f64>() / n;
        let sxy: f64 = xs.iter().zip(&ys_).map(|(x, y)| (x - mx) * (y - my)).sum();
        let sxx: f64 = xs.iter().map(|x| (x - mx).powi(2)).sum();
        let syy: f64 = ys_.iter().map(|y| (y - my).powi(2)).sum();
        if sxx > 0.0 && syy > 0.0 { sxy / (sxx * syy).sqrt() } else { 0.0 }
    });
    json!({ "years": ys, "dots": dots, "r": r.map(|r| (r * 100.0).round() / 100.0) })
}

/// The best and worst value for the money (proposed, not confirmed: the
/// snaps a man played for each $1M of an NFL cap hit, or each $100K of
/// college money, among men paid at least a league minimum's worth; the
/// game keeps no approximate value).
fn value(w: &World, level: Level, year: i32, me: &Row) -> Value {
    let unit = if level == Level::Nfl { 1e6 } else { 1e5 };
    let floor = if level == Level::Nfl { 0.75e6 } else { 1e4 };
    let mut v: Vec<(f64, Value)> = me
        .pay
        .iter()
        .filter(|p| paid(level, p) >= floor)
        .filter_map(|p| {
            let s = w.tally_player(level, year, p.id)?;
            let snaps: u32 = s.snaps.iter().sum();
            let per = snaps as f64 / (paid(level, p) / unit);
            Some((
                per,
                json!({
                    "id": p.id,
                    "name": afm_engine::world::marks::name_of(w, p.id),
                    "linked": views::person_index(w, p.id).is_some(),
                    "pos": p.pos.map(|x| x.name()),
                    "money": money_text(paid(level, p)),
                    "games": s.games,
                    "starts": s.starts,
                    "snaps": snaps,
                    "per": format!("{per:.0}"),
                }),
            ))
        })
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    let n = v.len();
    let best: Vec<Value> = v.iter().take(4).map(|x| x.1.clone()).collect();
    let worst: Vec<Value> = v.iter().skip(n.saturating_sub(3).max(4)).rev().map(|x| x.1.clone()).collect();
    json!({ "per": if level == Level::Nfl { "snaps a $1M" } else { "snaps a $100K" }, "best": best, "worst": worst })
}

/// A high school's budget, line by line.
fn hs_money(w: &World, team: usize, year: i32, rows: &[Row], me: &Row) -> Value {
    let h = &w.high_schools[team];
    let peers: Vec<&Row> = rows
        .iter()
        .filter(|r| {
            w.high_schools
                .get(r.team as usize)
                .is_some_and(|x| x.state == h.state && x.classification == h.classification)
        })
        .collect();
    let lines: Vec<Value> = fhs::LINES
        .iter()
        .enumerate()
        .map(|(k, l)| {
            let vals: Vec<(u32, f64)> = peers.iter().map(|r| (r.team, r.budget_line(k))).collect();
            json!({
                "name": l.name,
                "income": l.income,
                "budget": me.budget_line(k),
                "books": me.books_line(k),
                "rank": (me.budget_line(k) > 0.0).then(|| rank_desc(&vals, team as u32)).flatten(),
            })
        })
        .collect();
    let sum = |income: bool, books: bool| -> f64 {
        (0..fhs::N)
            .filter(|&k| fhs::LINES[k].income == income)
            .map(|k| if books { me.books_line(k) } else { me.budget_line(k) })
            .sum()
    };
    let spent_all: Vec<f64> =
        peers.iter().map(|r| (0..fhs::N).filter(|&k| !fhs::LINES[k].income).map(|k| r.budget_line(k)).sum()).collect();
    let roster = views::roster_people(w, TeamRef::HighSchool(team)).len().max(1);
    json!({
        "among": teams::group_name(w, Tier::Hs, team, year),
        "of": peers.len(),
        "lines": lines,
        "fee": me.val("fee"),
        "crowd": me.val("crowd"),
        "tiles": [
            { "label": "Brought in", "value": money_text(sum(true, true)), "sub": format!("budget {}", money_text(sum(true, false))) },
            { "label": "Spent", "value": money_text(sum(false, true)), "sub": format!("budget {}; the class's middle {}", money_text(sum(false, false)), median(spent_all).map_or("-".into(), money_text)) },
            { "label": "A player", "value": money_text(sum(false, false) / roster as f64), "sub": format!("{roster} players") },
            { "label": "Fee to play", "value": if me.val("fee") > 0.0 { money_text(me.val("fee")) } else { "None".into() }, "sub": format!("a home crowd of about {}", me.val("crowd").round()) },
        ],
    })
}
