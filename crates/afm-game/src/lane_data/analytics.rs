//! Player analytics (items 7 and 8, board 46): the percentile table by
//! position group and a player's card with the past seasons closest to his.
//!
//! A season's numbers are the group's card (`world::analytics`): the
//! charted card where your seat charts the level, the box score card where
//! it doesn't. Percentiles are among that season's qualifiers at the level
//! (a college division, the NFL, a state's high schools), so the seasons
//! like his compare men against their own seasons' fields, whatever the
//! era.

use afm_engine::positions::Position;
use afm_engine::world::analytics::{self as an, Cm, Fmt};
use afm_engine::world::data::Division;
use afm_engine::world::marks::{self, Tier};
use afm_engine::world::records::{STAT_COUNT, Stat};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use super::{charted, division_arg, state_arg, team_json, tier_arg, tier_key};
use crate::views;

/// One player's season in a pool.
struct Row {
    id: u32,
    team: usize,
    pos: Position,
    games: u32,
    year: i32,
    vol: f64,
    vals: Vec<Option<f64>>,
}

/// Where a pool is drawn from.
#[derive(Clone, Copy)]
struct Place {
    tier: Tier,
    division: Option<Division>,
    state: Option<usize>,
}

fn place_of(w: &World, args: &Value) -> Place {
    let tier = tier_arg(w, args);
    Place {
        tier,
        division: (tier == Tier::College).then(|| division_arg(w, args)),
        state: (tier == Tier::Hs).then(|| state_arg(w, args)),
    }
}

fn group_arg(args: &Value) -> usize {
    match args.get("group") {
        Some(Value::String(g)) => an::GROUPS.iter().position(|x| x.eq_ignore_ascii_case(g)).unwrap_or(0),
        Some(Value::Number(n)) => (n.as_u64().unwrap_or(0) as usize).min(an::GROUPS.len() - 1),
        _ => 0,
    }
}

fn card_of(w: &World, p: Place, group: usize) -> (&'static [Cm], Stat, &'static str, bool) {
    let ch = charted(w, p.tier, p.division);
    let card = if ch { an::charted(group) } else { an::boxed(group) };
    let (vol, word) = if ch { an::volume(group) } else { an::box_volume(group) };
    (card, vol, word, ch)
}

#[allow(clippy::too_many_arguments)]
fn row_from(
    id: u32,
    team: usize,
    pos: Position,
    games: u32,
    year: i32,
    all: &dyn Fn(Stat) -> f64,
    card: &[Cm],
    vol: Stat,
) -> Row {
    Row { id, team, pos, games, year, vol: all(vol), vals: card.iter().map(|cm| cm.value(all, games as f64)).collect() }
}

/// The seasons a level keeps, newest first, and the one in progress.
fn years(w: &World, p: Place) -> Vec<i32> {
    let mut v: Vec<i32> = match p.tier {
        Tier::College => w.stats.year_index.iter().map(|x| x.0).collect(),
        Tier::Nfl => {
            let mut y: Vec<i32> = w.marks.pro_rows.iter().map(|r| r.year as i32).collect();
            if y.is_empty() {
                y = w
                    .people
                    .iter()
                    .filter_map(|x| x.pro.as_ref())
                    .flat_map(|c| c.seasons.iter().map(|s| s.year))
                    .collect();
            }
            y
        }
        Tier::Hs => {
            let mut y: Vec<i32> = w.people.iter().flat_map(|x| x.hs_lines.iter().map(|l| l.year as i32)).collect();
            y.sort_unstable();
            y.dedup();
            y
        }
    };
    if !marks::season_now(w, p.tier, p.state).is_empty() {
        v.push(w.year);
    }
    v.sort_unstable();
    v.dedup();
    v.reverse();
    v
}

/// A season's pool at a place for a group.
fn pool(w: &World, p: Place, year: i32, group: usize) -> Vec<Row> {
    let (card, vol, _, ch) = card_of(w, p, group);
    let in_group = |pos: Position| an::group_of(pos) == Some(group);
    let now = marks::season_now(w, p.tier, p.state);
    let mut out = Vec::new();
    if year == w.year && !now.is_empty() {
        for l in now.iter().filter(|l| in_group(l.pos)) {
            if p.tier == Tier::College && w.programs.get(l.team).map(|x| x.division) != p.division {
                continue;
            }
            let get = |s: Stat| l.all[s as usize] as f64;
            out.push(row_from(l.id, l.team, l.pos, l.games, year, &get, card, vol));
        }
        return out;
    }
    match p.tier {
        Tier::College => {
            for r in w.stats.year(year).iter().filter(|r| Some(r.division) == p.division && in_group(r.pos)) {
                let get = |s: Stat| r.get(s);
                out.push(row_from(r.id, r.program as usize, r.pos, r.games as u32, year, &get, card, vol));
            }
        }
        Tier::Nfl => {
            let kept: Vec<&marks::ProRow> =
                w.marks.pro_rows.iter().filter(|r| r.year as i32 == year && in_group(r.pos)).collect();
            if !kept.is_empty() && ch {
                for r in kept {
                    out.push(Row {
                        id: r.id,
                        team: r.team as usize,
                        pos: r.pos,
                        games: r.games as u32,
                        year,
                        vol: r.vol as f64,
                        vals: (0..card.len())
                            .map(|k| r.vals.get(k).copied().filter(|v| v.is_finite()).map(|v| v as f64))
                            .collect(),
                    });
                }
            } else {
                // An older world file: the players' own season lines.
                for x in &w.people {
                    let Some(s) = x.pro.as_ref().and_then(|c| c.seasons.iter().find(|s| s.year == year)) else {
                        continue;
                    };
                    let (Some(team), true) = (s.team, s.games > 0 && in_group(x.player.position)) else { continue };
                    let all = afm_engine::world::records::from_line(&s.stats);
                    let get = |k: Stat| all[k as usize] as f64;
                    out.push(row_from(x.player.id, team, x.player.position, s.games, year, &get, card, vol));
                }
            }
        }
        Tier::Hs => out = hs_pools(w, p, group, Some(year)).remove(&year).unwrap_or_default(),
    }
    out
}

/// A state's high school seasons for a group, by year, in one pass over
/// the players (`only`: one season).
fn hs_pools(w: &World, p: Place, group: usize, only: Option<i32>) -> std::collections::BTreeMap<i32, Vec<Row>> {
    let (card, vol, _, _) = card_of(w, p, group);
    let mut out: std::collections::BTreeMap<i32, Vec<Row>> = Default::default();
    for x in &w.people {
        for l in &x.hs_lines {
            let year = l.year as i32;
            if only.is_some_and(|o| o != year) || an::group_of(l.pos) != Some(group) {
                continue;
            }
            if w.high_schools.get(l.school as usize).map(|h| h.state) != p.state {
                continue;
            }
            let all: [f32; STAT_COUNT] = marks::hs_all(&l.s.map(|v| v as i32));
            let get = |k: Stat| all[k as usize] as f64;
            out.entry(year).or_default().push(row_from(
                x.player.id,
                l.school as usize,
                l.pos,
                l.games as u32,
                year,
                &get,
                card,
                vol,
            ));
        }
    }
    out
}

/// Every past season's pool at a place (the season in progress left out).
fn past_pools(w: &World, p: Place, group: usize, ys: &[i32]) -> Vec<(i32, Vec<Row>)> {
    if p.tier == Tier::Hs {
        return hs_pools(w, p, group, None).into_iter().filter(|(y, _)| *y != w.year).collect();
    }
    ys.iter().filter(|&&y| y != w.year).map(|&y| (y, pool(w, p, y, group))).collect()
}

/// Each metric's percentile (0-100, 100 the best) among the qualifiers.
fn percentiles(rows: &[Row], qual: &[bool], card: &[Cm]) -> Vec<Vec<Option<f64>>> {
    let mut out = vec![vec![None; card.len()]; rows.len()];
    for (k, cm) in card.iter().enumerate() {
        let better = cm.about().3;
        let mut peers: Vec<f64> = rows.iter().zip(qual).filter(|(_, q)| **q).filter_map(|(r, _)| r.vals[k]).collect();
        peers.sort_by(f64::total_cmp);
        let n = peers.len();
        if n < 2 {
            continue;
        }
        for (i, r) in rows.iter().enumerate() {
            let Some(v) = r.vals[k] else { continue };
            let below = peers.partition_point(|&x| x < v) as f64;
            let same = (peers.partition_point(|&x| x <= v) as f64 - below).max(0.0);
            let mut pct = 100.0 * (below + 0.5 * same) / n as f64;
            if !better {
                pct = 100.0 - pct;
            }
            out[i][k] = Some(pct.clamp(0.0, 100.0));
        }
    }
    out
}

fn fmt_cell(cm: Cm, v: f64) -> String {
    match cm.about().2 {
        Fmt::Whole => format!("{v:.0}"),
        Fmt::One => format!("{v:.1}"),
        Fmt::Two => format!("{v:.2}"),
        Fmt::Pct => format!("{v:.1}%"),
        Fmt::Signed => format!("{v:+.2}"),
    }
}

fn name(w: &World, id: u32) -> String {
    marks::name_of(w, id).unwrap_or_else(|| "a former player".into())
}

fn columns(card: &[Cm]) -> Vec<Value> {
    card.iter()
        .map(|cm| {
            let (short, long, _, better) = cm.about();
            json!({ "label": short, "long": long, "better": if better { "high" } else { "low" } })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn base(
    w: &World,
    p: Place,
    group: usize,
    year: i32,
    years: &[i32],
    min: f64,
    word: &str,
    ch: bool,
) -> serde_json::Map<String, Value> {
    let mut m = serde_json::Map::new();
    m.insert("level".into(), json!(tier_key(p.tier)));
    m.insert("division".into(), json!(p.division.map(|d| d.name())));
    m.insert("state".into(), json!(p.state.and_then(|s| w.states.get(s)).map(|s| s.code.as_str())));
    m.insert("group".into(), json!(an::GROUPS[group]));
    m.insert("groups".into(), json!(an::GROUPS));
    m.insert("year".into(), json!(year));
    m.insert("years".into(), json!(years));
    m.insert("min".into(), json!(min));
    m.insert("volume".into(), json!(word));
    m.insert("charted".into(), json!(ch));
    m.insert("divisions".into(), json!(Division::ALL.iter().map(|d| d.name()).collect::<Vec<_>>()));
    m.insert("source".into(), super::sources(w)["line"].clone());
    m
}

/// The minimum a season needs to qualify: the page's, or two thirds of a
/// starter's volume at the level, cut down while a season is young.
fn min_for(w: &World, p: Place, group: usize, year: i32, ch: bool, args: &Value) -> f64 {
    if let Some(m) = args.get("min").and_then(Value::as_f64) {
        return m.max(0.0);
    }
    let full = an::default_min(group, ch, p.tier == Tier::Nfl);
    if year != w.year {
        return full;
    }
    // A season in progress: its share played so far, by the most games
    // anyone in the pool has.
    let played = marks::season_now(w, p.tier, p.state).iter().map(|l| l.games).max().unwrap_or(0) as f64;
    let total = marks::season_games(p.tier, year) as f64;
    if played > 0.0 { (full * (played / total).min(1.0)).round() } else { full }
}

pub fn table(w: &World, args: &Value) -> Value {
    let p = place_of(w, args);
    let group = group_arg(args);
    let (card, _, word, ch) = card_of(w, p, group);
    let ys = years(w, p);
    let year = args
        .get("year")
        .and_then(Value::as_i64)
        .map(|y| y as i32)
        .filter(|y| ys.contains(y))
        .or(ys.first().copied())
        .unwrap_or(w.year);
    let min = min_for(w, p, group, year, ch, args);
    let rows = pool(w, p, year, group);
    let qual: Vec<bool> = rows.iter().map(|r| r.vol >= min).collect();
    let pcts = percentiles(&rows, &qual, card);
    let sort = args.get("sort").and_then(Value::as_u64).map(|k| k as usize).filter(|&k| k < card.len()).unwrap_or(0);
    let dir = args.get("dir").and_then(Value::as_i64).unwrap_or(-1);
    let mut order: Vec<usize> = (0..rows.len()).filter(|&i| qual[i]).collect();
    // Sorted by percentile, so "best first" is the same for every metric.
    order.sort_by(|&a, &b| {
        let (x, y) = (pcts[a][sort].unwrap_or(-1.0), pcts[b][sort].unwrap_or(-1.0));
        let o = y.total_cmp(&x).then(rows[a].id.cmp(&rows[b].id));
        if dir > 0 { o.reverse() } else { o }
    });
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(150).min(500) as usize;
    let out: Vec<Value> = order
        .iter()
        .take(limit)
        .enumerate()
        .map(|(n, &i)| {
            let r = &rows[i];
            json!({
                "rank": n + 1,
                "id": r.id,
                "name": name(w, r.id),
                "linked": views::person_index(w, r.id).is_some(),
                "pos": r.pos.name(),
                "team": team_json(w, p.tier, r.team),
                "games": r.games,
                "vol": r.vol.round() as i64,
                "cells": r.vals.iter().zip(card).map(|(v, cm)| v.map(|v| fmt_cell(*cm, v))).collect::<Vec<_>>(),
                "pct": pcts[i].iter().map(|x| x.map(|x| x.round() as i64)).collect::<Vec<_>>(),
            })
        })
        .collect();
    let mut m = base(w, p, group, year, &ys, min, word, ch);
    m.insert("columns".into(), json!(columns(card)));
    m.insert("sort".into(), json!(sort));
    m.insert("total".into(), json!(order.len()));
    m.insert("pool".into(), json!(rows.len()));
    m.insert("rows".into(), json!(out));
    Value::Object(m)
}

/// Where a player is now, for "seasons like his".
fn now_text(w: &World, id: u32) -> String {
    let Some(k) = views::person_index(w, id) else { return "Out of football".into() };
    let x = &w.people[k];
    match x.stage {
        Stage::HighSchool => format!("High school · {}", w.school_name(x.hs)),
        Stage::College | Stage::PrepSchool | Stage::Away => {
            x.college.map_or_else(|| "College".to_string(), |c| format!("College · {}", w.program_name(c)))
        }
        Stage::Declared => "In the draft pool".into(),
        Stage::Pro => x
            .pro
            .as_ref()
            .and_then(|c| c.team)
            .map_or_else(|| "Pro football".to_string(), |t| format!("NFL · {}", w.nfl_team_name(t))),
        Stage::Done => "Out of football".into(),
    }
}

/// A player's card (item 7) and the past seasons closest to his (item 8).
pub fn card(w: &World, args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_u64).map(|x| x as u32) else {
        return json!({ "error": "No player." });
    };
    let Some(k) = views::person_index(w, id) else { return json!({ "error": "No such player." }) };
    let x = &w.people[k];
    let Some(group) = an::group_of(x.player.position) else { return json!({ "error": "No card for long snappers." }) };
    // His level: the one asked for, else where he plays now.
    let tier = match args.get("level").and_then(Value::as_str) {
        Some(_) => tier_arg(w, args),
        None => match x.stage {
            Stage::Pro => Tier::Nfl,
            Stage::HighSchool => Tier::Hs,
            _ => Tier::College,
        },
    };
    let p = Place {
        tier,
        division: (tier == Tier::College)
            .then(|| {
                x.college.map(|c| w.programs[c].division).or_else(|| w.stats.career(id).last().map(|r| r.division))
            })
            .flatten(),
        state: (tier == Tier::Hs).then(|| w.high_schools[x.hs].state),
    };
    let (card, _, word, ch) = card_of(w, p, group);
    // His seasons at the level, newest first.
    let now = marks::season_now(w, tier, p.state);
    let mut mine: Vec<i32> = match tier {
        Tier::College => {
            w.stats.career(id).iter().filter(|r| Some(r.division) == p.division).map(|r| r.year as i32).collect()
        }
        Tier::Nfl => {
            x.pro.as_ref().map_or(Vec::new(), |c| c.seasons.iter().filter(|s| s.games > 0).map(|s| s.year).collect())
        }
        Tier::Hs => x.hs_lines.iter().map(|l| l.year as i32).collect(),
    };
    if now.iter().any(|l| l.id == id) {
        mine.push(w.year);
    }
    mine.sort_unstable();
    mine.dedup();
    mine.reverse();
    let year = args
        .get("year")
        .and_then(Value::as_i64)
        .map(|y| y as i32)
        .filter(|y| mine.contains(y))
        .or(mine.first().copied());
    let Some(year) = year else {
        let mut m = base(w, p, group, w.year, &mine, 0.0, word, ch);
        m.insert("id".into(), json!(id));
        m.insert("name".into(), json!(x.name));
        m.insert("empty".into(), json!("No season at this level yet."));
        return Value::Object(m);
    };
    let min = min_for(w, p, group, year, ch, &Value::Null);
    let rows = pool(w, p, year, group);
    let qual: Vec<bool> = rows.iter().map(|r| r.vol >= min).collect();
    let pcts = percentiles(&rows, &qual, card);
    let Some(me) = rows.iter().position(|r| r.id == id) else {
        return json!({ "error": "No line that season." });
    };
    let r = &rows[me];
    let metrics: Vec<Value> = card
        .iter()
        .enumerate()
        .map(|(k, cm)| {
            let (short, long, _, better) = cm.about();
            json!({
                "label": short, "long": long, "better": if better { "high" } else { "low" },
                "value": r.vals[k].map(|v| fmt_cell(*cm, v)),
                "pct": pcts[me][k].map(|x| x.round() as i64),
            })
        })
        .collect();
    // Seasons like his: every other season's field at the level, each read
    // against its own field (proposed, not confirmed: the closeness is the
    // mean gap in percentiles over the numbers both seasons have, at least
    // half the card).
    let my_pct = &pcts[me];
    let mut like: Vec<(f64, u32, i32, usize, u32)> = Vec::new();
    let ys = years(w, p);
    for (y, rs) in past_pools(w, p, group, &ys) {
        if y == year {
            continue;
        }
        let q: Vec<bool> = rs.iter().map(|r| r.vol >= an::default_min(group, ch, tier == Tier::Nfl)).collect();
        let ps = percentiles(&rs, &q, card);
        for (i, o) in rs.iter().enumerate() {
            if !q[i] || o.id == id {
                continue;
            }
            let pairs: Vec<f64> =
                my_pct.iter().zip(&ps[i]).filter_map(|(a, b)| Some((a.as_ref()? - b.as_ref()?).abs())).collect();
            if pairs.len() * 2 < card.len().max(1) {
                continue;
            }
            let gap = pairs.iter().sum::<f64>() / pairs.len() as f64;
            like.push((gap, o.id, o.year, o.team, o.games));
        }
    }
    like.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    like.dedup_by_key(|l| l.1);
    let like: Vec<Value> = like
        .iter()
        .take(5)
        .map(|&(gap, oid, y, team, games)| {
            json!({
                "id": oid,
                "name": name(w, oid),
                "linked": views::person_index(w, oid).is_some(),
                "year": y,
                "team": team_json(w, tier, team),
                "games": games,
                "close": (100.0 - gap).round() as i64,
                "now": now_text(w, oid),
            })
        })
        .collect();
    let mut m = base(w, p, group, year, &ys, min, word, ch);
    m.insert("id".into(), json!(id));
    m.insert("name".into(), json!(x.name));
    m.insert("pos".into(), json!(r.pos.name()));
    m.insert("team".into(), team_json(w, tier, r.team));
    m.insert("games".into(), json!(r.games));
    m.insert("vol".into(), json!(r.vol.round() as i64));
    m.insert("qualified".into(), json!(qual[me]));
    m.insert("field".into(), json!(qual.iter().filter(|q| **q).count()));
    m.insert("seasons".into(), json!(mine));
    m.insert("metrics".into(), json!(metrics));
    m.insert("like".into(), json!(like));
    Value::Object(m)
}
