//! Lane NB8 (Data overhaul item 24): the game page for any game the
//! tallies keep, `tally_game` (`year`, `key`): the box score and every
//! player's line for anyone; and where your staff charts the game, the
//! advanced box (EPA a play, success rate, explosiveness, havoc, finishing
//! drives, field position), the win-chance chart, post-game win
//! expectancy (what the box says the result should have been) and every
//! 4th down and try against the analysts' model.

use afm_engine::calendar::Date;
use afm_engine::game::stats::TeamStats;
use afm_engine::player::Level as PLevel;
use afm_engine::world::World;
use afm_engine::world::data::Division;
use afm_engine::world::marks::Tier;
use afm_engine::world::records::Stat;
use afm_engine::world::tally::{self, calls, decide, games};
use serde_json::{Value, json};

fn tier_of(level: u8) -> Tier {
    match level {
        1 => Tier::Nfl,
        2 => Tier::Hs,
        _ => Tier::College,
    }
}

fn clock(secs: f64) -> String {
    let s = secs.max(0.0).round() as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

/// A side's box: the team totals a box score prints.
pub(crate) fn team_box(t: &TeamStats) -> Value {
    let top = if t.clock_possession > 0.0 { t.clock_possession } else { t.time_of_possession };
    json!({
        "points": t.points,
        "yards": t.yards.round(),
        "pass": t.pass_yards.round(),
        "rush": t.rush_yards.round(),
        "first_downs": t.first_downs,
        "third": format!("{}/{}", t.third_conv, t.third_att),
        "fourth": format!("{}/{}", t.fourth_conv, t.fourth_att),
        "turnovers": t.turnovers,
        "penalties": format!("{}-{}", t.penalties, t.penalty_yds.round()),
        "top": clock(top),
        "plays": t.plays,
        "sacks": t.sacks,
        "cmp": format!("{}/{}", t.cmp, t.att),
        "carries": t.carries,
    })
}

/// A side's advanced box from its plays' sums (`a` its own, `b` the other
/// side's: havoc is the share of the other side's snaps this side's
/// defense made a play on).
pub(crate) fn adv_box(a: &games::Adv, b: &games::Adv, t: &TeamStats) -> Value {
    let plays = (a.runs + a.dropbacks) as f64;
    let five = decide::five(a);
    let per = |x: f64, n: f64| if n > 0.0 { x / n } else { 0.0 };
    let pts = |td: u32, fg: u32, n: u32| per(7.0 * td as f64 + 3.0 * fg as f64, n as f64);
    json!({
        "plays": plays,
        "epa": per((a.run_epa + a.pass_epa) as f64, plays),
        "epa_total": (a.run_epa + a.pass_epa) as f64,
        "run_epa": per(a.run_epa as f64, a.runs as f64),
        "pass_epa": per(a.pass_epa as f64, a.dropbacks as f64),
        "success": five[0],
        "run_success": 100.0 * per(a.run_success as f64, a.runs as f64),
        "pass_success": 100.0 * per(a.pass_success as f64, a.dropbacks as f64),
        "explosiveness": five[1],
        "explosive": a.explosive,
        "explosive_rate": 100.0 * per(a.explosive as f64, plays),
        "havoc": 100.0 * per(a.havoc as f64, (b.runs + b.dropbacks) as f64),
        "havoc_n": a.havoc,
        "start": five[2],
        "drives": a.drives,
        "trips40": a.trips40,
        "pts40": five[3],
        "rz": format!("{}/{}", a.rz_td, a.rz_trips),
        "rz_pts": pts(a.rz_td, a.rz_fg, a.rz_trips),
        "three_outs": a.three_outs,
        "line_yards": per(a.line_yards as f64, a.runs as f64),
        "stuffed": 100.0 * per(a.stuffed as f64, a.runs as f64),
        "turnovers": t.turnovers,
    })
}

/// What the box says each side's chance was, home and away (0-100).
pub(crate) fn win_expectancy(level: PLevel, a: &[games::Adv; 2], b: &[TeamStats; 2], neutral: bool) -> [f64; 2] {
    let h = 100.0 * decide::pgwe(level, a, b, neutral);
    [h, 100.0 - h]
}

fn player_lines(w: &World, g: &games::GameRow, side: u8) -> Value {
    let name = |id: u32| {
        crate::views::person_index(w, id)
            .map(|k| json!({ "id": id, "name": w.people[k].name }))
            .unwrap_or(json!({ "id": null, "name": format!("No. {id}") }))
    };
    let lines: Vec<&games::Line> = g.lines.iter().filter(|l| l.side == side).collect();
    let pick = |stat: Stat, cols: &[Stat], top: usize| -> Vec<Value> {
        let mut v: Vec<(f64, Value)> = lines
            .iter()
            .filter(|l| l.get(stat) > 0.0)
            .map(|l| {
                let all = l.all();
                let cells: Vec<String> = cols.iter().map(|&c| super::fmt_num(all[c as usize] as f64)).collect();
                (l.get(stat), json!({ "who": name(l.id), "pos": l.pos.map(|p| format!("{p:?}")), "cells": cells }))
            })
            .collect();
        v.sort_by(|a, b| b.0.total_cmp(&a.0));
        v.into_iter().take(top).map(|x| x.1).collect()
    };
    json!({
        "passing": pick(Stat::Att, &[Stat::Cmp, Stat::Att, Stat::PassYds, Stat::PassTd, Stat::Int, Stat::Sacked], 3),
        "rushing": pick(Stat::Rush, &[Stat::Rush, Stat::RushYds, Stat::RushTd], 6),
        "receiving": pick(Stat::Targets, &[Stat::Targets, Stat::Rec, Stat::RecYds, Stat::RecTd], 8),
        "defense": pick(Stat::Tackles, &[Stat::Tackles, Stat::Tfl, Stat::Sacks, Stat::Ints, Stat::Pbus], 10),
        "kicking": pick(Stat::FgAtt, &[Stat::FgMade, Stat::FgAtt, Stat::XpMade, Stat::FgLong], 2),
    })
}

/// Whether your staff charts a game: the NFL's tracking always; a college
/// game where either side is in a division you chart.
fn charted(w: &World, g: &games::GameRow) -> Option<super::Charting> {
    match tier_of(g.level) {
        Tier::Nfl => super::charting(w, Tier::Nfl, None),
        Tier::Hs => None,
        Tier::College => {
            let div = |t: u32| -> Option<Division> {
                ((t as usize) < w.programs.len()).then(|| super::season_place(w, t as usize, g.season).0)
            };
            g.teams.iter().find_map(|&t| super::charting(w, Tier::College, div(t)))
        }
    }
}

/// Item 24: any game's page.
pub fn tally_game(w: &World, args: &Value) -> Value {
    let year = args.get("year").and_then(Value::as_i64).map(|y| y as i32).unwrap_or(w.tally_open_season());
    let Some(key) = args.get("key").and_then(Value::as_u64) else { return json!({ "error": "No such game." }) };
    let Some(g) = w.tally_game_row(year, key) else { return json!({ "error": "That game isn't in the books." }) };
    let tier = tier_of(g.level);
    let team = |k: usize| super::team_json(w, tier, g.teams[k] as usize);
    let mut m = serde_json::Map::new();
    m.insert("year".into(), json!(year));
    m.insert("key".into(), json!(key));
    m.insert("level".into(), json!(super::tier_key(tier)));
    m.insert("group".into(), json!(g.group));
    m.insert("date".into(), json!(g.date().to_string()));
    m.insert("kind".into(), json!(g.kind));
    m.insert("neutral".into(), json!(g.neutral));
    m.insert("conference".into(), json!(g.conference));
    m.insert("weather".into(), json!(g.weather));
    m.insert("home".into(), team(0));
    m.insert("away".into(), team(1));
    m.insert("score".into(), json!(g.score));
    m.insert("quarters".into(), json!(g.quarters));
    m.insert("overtime".into(), json!(g.overtime));
    m.insert("box".into(), json!([team_box(&g.boxes[0]), team_box(&g.boxes[1])]));
    m.insert("players".into(), json!([player_lines(w, &g, 0), player_lines(w, &g, 1)]));
    // Your kept game, for the full page with every snap.
    let cal = match tier {
        Tier::Nfl => afm_engine::calendar::Level::Nfl,
        Tier::Hs => afm_engine::calendar::Level::HighSchool,
        Tier::College => afm_engine::calendar::Level::College,
    };
    let seat = w.seat_games().iter().position(|s| {
        s.level == cal && s.date.0 == g.date && s.teams[0] as u32 == g.teams[0] && s.teams[1] as u32 == g.teams[1]
    });
    m.insert("seat".into(), json!(seat));
    let ch = charted(w, &g);
    let open = year == w.tally_open_season();
    let wait = ch.as_ref().filter(|_| open).map(|c| Date(g.date + c.days as i32 - 1)).filter(|d| *d > w.today);
    let charted_now = ch.is_some() && wait.is_none() && g.adv.iter().any(|a| a.runs + a.dropbacks > 0);
    m.insert("charted".into(), json!(charted_now));
    m.insert("charting".into(), ch.as_ref().map_or(Value::Null, super::charting_json));
    if let Some(d) = wait {
        m.insert("wait".into(), json!(d.to_string()));
    }
    m.insert(
        "source".into(),
        json!(match (&ch, wait, tier) {
            (_, Some(d), _) => format!("Your staff's charting of this game comes in on {d}: the box score until then."),
            (None, _, Tier::Hs) => "High school is box scores: no one charts its snaps.".to_string(),
            (None, _, _) => "Your staff doesn't chart this game's division: the box score only.".to_string(),
            (Some(c), _, Tier::Nfl) =>
                format!("The NFL's league-wide tracking ({}).", afm_engine::world::analytics::depth_name(c.depth)),
            (Some(c), _, _) => format!("Your staff's {}.", afm_engine::world::analytics::depth_name(c.depth)),
        }),
    );
    if !charted_now {
        return Value::Object(m);
    }
    let lv = calls::model_level(
        match tier {
            Tier::Nfl => tally::Level::Nfl,
            Tier::Hs => tally::Level::Hs,
            Tier::College => tally::Level::College,
        },
        group_level(&g.group),
    );
    m.insert(
        "advanced".into(),
        json!([adv_box(&g.adv[0], &g.adv[1], &g.boxes[0]), adv_box(&g.adv[1], &g.adv[0], &g.boxes[1])]),
    );
    m.insert("pgwe".into(), json!(win_expectancy(lv, &g.adv, &g.boxes, g.neutral)));
    m.insert("wp".into(), json!(g.wp));
    // Both sides' 4th downs and tries in this game, from their seasons' rows.
    let tl = match tier {
        Tier::Nfl => tally::Level::Nfl,
        _ => tally::Level::College,
    };
    let mut fourths = Vec::new();
    let mut tries = Vec::new();
    for k in 0..2 {
        let Some(s) = w.tally_call(tl, year, g.teams[k]) else { continue };
        for f in s.fourths.iter().filter(|f| f.game == key) {
            let j = f.weigh().map(|x| calls::judge(&x, f.did));
            fourths.push((
                (f.q, std::cmp::Reverse(f.clock)),
                json!({
                    "side": k,
                    "clock": super::situ::clock_text(f.q, f.clock),
                    "lead": f.lead,
                    "togo": f.togo,
                    "yl": f.yl,
                    "did": f.did,
                    "wp": f.wp,
                    "conv": f.conv,
                    "best": j.map(|j| j.best),
                    "lost": j.map(|j| 100.0 * j.lost),
                    "res": f.res,
                    "yds": f.yds,
                }),
            ));
        }
        for t in s.tries.iter().filter(|t| t.game == key) {
            let j = t.judge();
            tries.push((
                (t.q, std::cmp::Reverse(t.clock)),
                json!({
                    "side": k,
                    "clock": super::situ::clock_text(t.q, t.clock),
                    "lead": t.lead,
                    "two": t.two,
                    "good": t.good,
                    "wp": t.wp,
                    "best_two": j.map(|x| x.0),
                    "lost": j.map(|x| 100.0 * x.2),
                }),
            ));
        }
    }
    fourths.sort_by_key(|x| x.0);
    tries.sort_by_key(|x| x.0);
    m.insert("fourths".into(), json!(fourths.into_iter().map(|x| x.1).collect::<Vec<_>>()));
    m.insert("tries".into(), json!(tries.into_iter().map(|x| x.1).collect::<Vec<_>>()));
    Value::Object(m)
}

/// The model's level of a college game from its group ("FBS", "FBS/FCS":
/// the higher one).
fn group_level(group: &str) -> Option<PLevel> {
    group.split('/').filter_map(Division::parse).min().map(afm_engine::world::division_level)
}
