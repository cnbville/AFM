//! V5, a player's career arc: your staff's kept read at each season's end
//! (kept from the day your staff followed him; earlier seasons show what
//! the record keeps, not a read made up after the fact), today's read, and
//! the staff's guess at the road ahead from how players his age grow in
//! this world, with players who were rated like him at his age and what
//! became of them. A guess, not a promise: no ceiling is shown.

use afm_engine::player::{DevelopmentEvent as E, Level, PlayingTime};
use afm_engine::positions::Position;
use afm_engine::world::norms::{Pool, Rated};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};
use std::collections::HashSet;

use super::{any_read, first_last, growth_at, growth_table, kept, key_attrs, mid, pair_json, quantile, tier_of};
use crate::Reply;
use crate::reads;
use crate::views::{self, TeamRef};

fn level_word(l: Level) -> &'static str {
    match l {
        Level::HS => "High school",
        Level::NFL => "NFL",
        _ => "College",
    }
}

/// What a season held, in a few words.
fn what_happened(h: &afm_engine::player::SeasonRecord) -> String {
    let mut v: Vec<String> = vec![views::role_words(h.playing_time).to_string()];
    for e in h.events.iter() {
        let s = match e {
            E::GrowthSpurt => "a growth spurt",
            E::BodyStalled => "his body stalled",
            E::Breakout => "a breakout",
            E::Stagnation => "stalled",
            E::LostSeason => "a lost season",
            E::Suspended => "suspended",
            E::Dismissed => "dismissed",
            E::Slump => "a slump",
            E::MinorInjury => "a minor injury",
            E::ModerateInjury => "an injury",
            E::MajorInjury => "a major injury",
            E::FirstYear => "a first-year dip",
            E::SpringBreakout => "a spring breakout",
            E::LostBurst => "lost a step",
            E::WeightGain => "bad weight",
            E::Complacent => "lost his drive",
            E::PoorCoaching => "poor coaching",
        };
        v.push(s.to_string());
    }
    if h.games_missed > 0 {
        v.push(format!("missed {} game{}", h.games_missed, if h.games_missed == 1 { "" } else { "s" }));
    }
    let mut s = v.join(", ");
    if let Some(c) = s.get(..1) {
        s = c.to_uppercase() + &s[1..];
    }
    s
}

/// The highest step a player reached, from the public record.
fn reached(draft: bool, college: bool, pro: Option<&afm_engine::world::pro::ProCareer>) -> &'static str {
    match pro {
        Some(c) if c.seasons.iter().any(|s| s.starts >= 8) => "NFL starter",
        Some(c) if c.seasons.iter().any(|s| s.games > 0) => "Played in the NFL",
        _ if draft => "Drafted",
        _ if college => "College",
        _ => "High school",
    }
}

/// A past player's road in public words.
fn became(draft: Option<(i32, u32, u32)>, school: &str, pro: Option<&afm_engine::world::pro::ProCareer>) -> String {
    let nfl = match pro {
        Some(c) => {
            let n = c.seasons.iter().filter(|s| s.games > 0).count();
            let bowls = c.seasons.iter().filter(|s| s.pro_bowl).count();
            match (n, bowls) {
                (0, _) => "never played an NFL game".to_string(),
                (n, 0) => format!("{n} NFL season{}", if n == 1 { "" } else { "s" }),
                (n, b) => format!("{n} NFL seasons, {b} Pro Bowl{}", if b == 1 { "" } else { "s" }),
            }
        }
        None => String::new(),
    };
    let d = match draft {
        Some((y, r, p)) => format!("Drafted {y}, round {r} (pick {p})"),
        None if !school.is_empty() => format!("Undrafted out of {school}"),
        None => "Never played college football".to_string(),
    };
    if nfl.is_empty() { d } else { format!("{d} · {nfl}") }
}

/// A man rated like him at his age, from the world's records.
struct Alike {
    name: String,
    id: Option<u32>,
    /// His overall from that age on, as change from that age.
    deltas: Vec<f64>,
    gap: f64,
    year: i32,
    outcome: String,
    reached: &'static str,
}

/// Pros playing now, by name and class. The alumni list keeps a drafted
/// man from the day he left college; while he plays, his live record is
/// the fuller one, so the alumni copy is skipped.
pub(crate) fn live_pros(w: &World) -> HashSet<(&str, i32)> {
    w.people.iter().filter(|q| q.stage == Stage::Pro).map(|q| (q.name.as_str(), q.class_of)).collect()
}

/// Past players and pros at his position whose rating at his age and
/// level sat within 3 of your staff's read.
fn alikes(w: &World, pos: Position, age: u32, tier: u8, at: f64, me: u32) -> Vec<Alike> {
    let mut out: Vec<Alike> = Vec::new();
    let mut take = |hist: &[afm_engine::player::SeasonRecord],
                    name: &str,
                    id: Option<u32>,
                    year: i32,
                    outcome: String,
                    reached: &'static str| {
        let Some(k) = hist.iter().position(|h| h.age == age && tier_of(h.level) == tier) else { return };
        let base = hist[k].overall_start;
        let gap = (base - at).abs();
        if gap > 3.0 {
            return;
        }
        let deltas = hist[k..].iter().map(|h| h.overall - base).take(10).collect();
        out.push(Alike { name: name.to_string(), id, deltas, gap, year, outcome, reached });
    };
    let live = live_pros(w);
    for a in &w.alumni {
        let Some(pl) = a.player.as_ref() else { continue };
        if a.position != pos || live.contains(&(a.name.as_str(), a.class_of)) {
            continue;
        }
        let draft = a
            .pro
            .as_ref()
            .and_then(|c| c.draft)
            .map(|d| (d.0, d.1, d.2))
            .or(a.pick.map(|p| (a.year, (p - 1) / 32 + 1, p)));
        let school = a.colleges.last().map_or(String::new(), |&c| w.program_name(c).to_string());
        let outcome = became(draft, &school, a.pro.as_deref());
        let top = reached(draft.is_some(), !a.colleges.is_empty(), a.pro.as_deref());
        take(&pl.history, &a.name, None, a.year, outcome, top);
    }
    for q in w.people.iter().filter(|q| q.stage == Stage::Pro && q.player.position == pos && q.player.id != me) {
        let c = q.pro.as_deref();
        let draft = c.and_then(|c| c.draft).map(|d| (d.0, d.1, d.2));
        let school = q.college.map_or(String::new(), |c| w.program_name(c).to_string());
        let outcome = format!("{} · in the NFL now", became(draft, &school, c));
        let top = reached(draft.is_some(), q.college.is_some(), c);
        take(&q.player.history, &q.name, Some(q.player.id), w.year, outcome, top);
    }
    out.sort_by(|a, b| a.gap.total_cmp(&b.gap).then(b.year.cmp(&a.year)));
    out
}

/// The drafted share, the usual rounds, the starter share (when enough
/// left long enough ago) and how many men they come from.
type AfterCollege = (f64, Option<(u32, u32)>, Option<f64>, usize);

/// Of the men who signed with an NFL team out of college with a rating
/// near `at` at his position in the last 25 years: the share drafted,
/// their usual rounds, and the share who became NFL starters (a season of
/// 8 starts or more), counted over the men who left four seasons ago or
/// more, who had the time. The world keeps no record of a man who left
/// college for life, only of its blue-chips, so those are left out too:
/// counting them and not the rest would tilt the shares.
fn after_college(w: &World, pos: Position, at: f64) -> Option<AfterCollege> {
    let (mut n, mut drafted, mut settled, mut starters) = (0usize, 0usize, 0usize, 0usize);
    let mut rounds: Vec<f64> = Vec::new();
    let since = w.year - 25;
    let live = live_pros(w);
    let mut count = |left: i32, round: Option<f64>, started: bool| {
        n += 1;
        if let Some(r) = round {
            drafted += 1;
            rounds.push(r);
        }
        if left <= w.year - 4 {
            settled += 1;
            starters += usize::from(started);
        }
    };
    for a in
        w.alumni.iter().filter(|a| a.position == pos && !a.colleges.is_empty() && !a.left_football && a.year >= since)
    {
        if (a.overall - at).abs() > 3.0 || live.contains(&(a.name.as_str(), a.class_of)) {
            continue;
        }
        let round =
            a.pick.map(|p| a.pro.as_ref().and_then(|c| c.draft).map_or(((p - 1) / 32 + 1) as f64, |d| d.1 as f64));
        count(a.year, round, a.pro.as_ref().is_some_and(|c| c.seasons.iter().any(|s| s.starts >= 8)));
    }
    for q in w.people.iter().filter(|q| q.stage == Stage::Pro && q.player.position == pos) {
        let hist = &q.player.history;
        let Some(k) = hist.iter().rposition(|h| h.level.is_college()) else { continue };
        let left = record_season(w, q, k) + 1;
        if (hist[k].overall - at).abs() > 3.0 || left < since {
            continue;
        }
        let c = q.pro.as_ref();
        count(
            left,
            c.and_then(|c| c.draft).map(|d| d.1 as f64),
            c.is_some_and(|c| c.seasons.iter().any(|s| s.starts >= 8)),
        );
    }
    if n < 8 {
        return None;
    }
    let r = (rounds.len() >= 3)
        .then(|| (quantile(&mut rounds.clone(), 0.25).round() as u32, quantile(&mut rounds, 0.75).round() as u32));
    Some((drafted as f64 / n as f64, r, (settled >= 8).then(|| starters as f64 / settled as f64), n))
}

/// The season of the `k`th row of his record. A row is written at his
/// yearly checkpoint with the age he carries through that season, his age
/// on 31 August as it starts (`calendar::born`); a person from an older
/// world file without a birth date counts back from last season.
pub(crate) fn record_season(w: &World, p: &afm_engine::world::Person, k: usize) -> i32 {
    let hist = &p.player.history;
    if p.born == afm_engine::calendar::born::UNKNOWN {
        return w.year - hist.len() as i32 + k as i32;
    }
    p.born.year() + hist[k].age as i32 + i32::from(p.born.month() > 8)
}

pub(crate) fn view(w: &World, id: u32) -> Reply {
    let i = views::person_index(w, id).ok_or("That player is no longer in football.")?;
    let p = &w.people[i];
    let pos = p.player.position;
    let (ovr, own, eye) = any_read(w, i);
    let reader = views::reader_key(w);
    let attrs = reads::attributes(&p.player, own, eye, reader);
    let keys = key_attrs(pos);
    let hist = &p.player.history;
    let seasons_rows = crate::lane_core::pages::seasons(w, i);
    // The record's seasons, oldest first, with any kept read. This season's
    // row is the one his age points at once his checkpoint has run.
    let mut table: Vec<Value> = Vec::new();
    let mut points: Vec<Value> = Vec::new();
    let mut this_in_record = false;
    for (k, h) in hist.iter().enumerate() {
        let season = record_season(w, p, k);
        if season == w.year {
            this_in_record = true;
            continue;
        }
        let kr = kept(w, id, season);
        let stat = seasons_rows.iter().find(|s| s["year"].as_i64() == Some(season as i64));
        let team = match (h.level, h.team) {
            (Level::NFL, Some(t)) => Some(w.nfl_team_name(t)),
            _ => stat.and_then(|s| s["team"]["name"].as_str().map(String::from)).or_else(|| {
                // A season without games (a redshirt, an injury) still had a school.
                let c = p.college_staff.iter().find(|c| c.0 == season).filter(|_| h.level.is_college())?;
                Some(w.program_name(c.1).to_string())
            }),
        };
        table.push(json!({
            "season": season, "age": h.age, "level": level_word(h.level), "pos": h.position.name(),
            "team": team, "read": kr.map(|r| pair_json(r.ovr)),
            "keys": kr.map(|r| r.keys.iter().map(|x| pair_json([x[1], x[2]])).collect::<Vec<_>>()),
            "games": stat.and_then(|s| s["games"].as_u64()),
            "line": stat.and_then(|s| s["line"].as_str()),
            "what": what_happened(h),
            "level_rating": h.level_rating_end.or(h.level_rating),
        }));
        if let Some(r) = kr {
            points.push(json!({
                "season": season, "age": h.age, "level": level_word(h.level), "read": pair_json(r.ovr),
                "keys": r.keys.iter().map(|x| pair_json([x[1], x[2]])).collect::<Vec<_>>(), "now": false,
            }));
        }
    }
    // Today's read, this season's row.
    let level_now = views::level_of(w, p);
    let age = p.player.age;
    let key_now: Vec<Value> =
        keys.iter().map(|a| attrs.iter().find(|x| x.0 == *a).map_or(Value::Null, |x| x.1.json())).collect();
    let team_now = views::team_of(w, p).map(|t| views::team_name(w, t));
    let this_stat = seasons_rows.iter().find(|s| s["year"].as_i64() == Some(w.year as i64));
    if this_in_record || matches!(p.stage, Stage::HighSchool | Stage::PrepSchool | Stage::College | Stage::Pro) {
        table.push(json!({
            "season": w.year, "age": age, "level": level_word(level_now), "pos": pos.name(), "team": team_now,
            "read": ovr.json(), "keys": key_now, "now": true,
            "games": this_stat.and_then(|s| s["games"].as_u64()),
            "line": this_stat.and_then(|s| s["line"].as_str()),
            "what": "This season, so far",
        }));
    }
    points.push(json!({ "season": w.year, "age": age, "level": level_word(level_now), "read": ovr.json(), "keys": key_now, "now": true }));
    // The road ahead: each coming season's level and the growth there.
    let table_g = growth_table(w);
    let start = mid(ovr);
    let half = match ovr {
        reads::Read::Range(lo, hi) => (hi as f64 - lo as f64) / 2.0,
        reads::Read::NotSeen => 6.0,
    };
    let mut road: Vec<(i32, u32, u8)> = Vec::new();
    let tier_now = tier_of(level_now);
    let college_left = |p: &afm_engine::world::Person| -> i32 {
        let limit = if w.changes.rules(w.year).five_in_five { 5 } else { 4 };
        (limit - p.college_seasons as i32).min(p.class_of + 5 - w.year).max(0)
    };
    for n in 1..=8 {
        let season = w.year + n;
        let a = age + n as u32;
        let tier = match p.stage {
            Stage::HighSchool | Stage::PrepSchool => {
                if season < p.class_of {
                    0
                } else if season < p.class_of + 4 {
                    1
                } else {
                    2
                }
            }
            Stage::College => {
                if n < college_left(p) {
                    1
                } else {
                    2
                }
            }
            _ => tier_now,
        };
        road.push((season, a, tier));
    }
    let mut path: Vec<Value> = Vec::new();
    let mut peak: Option<(f64, u32, f64, f64)> = start.map(|m| (m, age, m - half, m + half));
    if let Some(m0) = start {
        let (mut p50, mut s_lo, mut s_hi, mut s_lo2, mut s_hi2) =
            (m0, half * half, half * half, half * half, half * half);
        for &(season, a, tier) in &road {
            let q = growth_at(&table_g, a - 1, tier);
            p50 = (p50 + q[2]).clamp(1.0, 99.0);
            s_lo += (q[2] - q[1]).powi(2);
            s_hi += (q[3] - q[2]).powi(2);
            s_lo2 += (q[2] - q[0]).powi(2);
            s_hi2 += (q[4] - q[2]).powi(2);
            let row = [p50 - s_lo2.sqrt(), p50 - s_lo.sqrt(), p50, p50 + s_hi.sqrt(), p50 + s_hi2.sqrt()]
                .map(|x| (x.clamp(1.0, 99.0) * 10.0).round() / 10.0);
            if peak.is_none_or(|pk| p50 > pk.0) {
                peak = Some((p50, a, row[1], row[3]));
            }
            path.push(json!({ "season": season, "age": a, "tier": tier, "level": (["High school", "College", "NFL"][tier as usize]), "q": row }));
        }
    }
    // The cells: likely peak, the draft, an NFL starter.
    let nfl_marks = crate::lane_ratings::marks_cached(w, Pool::Nfl, pos, Rated::Overall);
    // The years after college say "NFL" only when his middle path gets to
    // a typical NFL backup by then; most men's football ends with college.
    if let Some(r) = path.iter().position(|r| r["tier"] == 2)
        && path[r]["q"][2].as_f64().zip(nfl_marks.as_ref()).is_none_or(|(m, k)| m < k.backup)
        && p.stage != Stage::Pro
    {
        for row in &mut path[r..] {
            row["level"] = json!("After college");
        }
    }
    let (draft_cell, starter_cell, outcome_n) = match p.stage {
        Stage::Pro | Stage::Done => {
            let d = p.pro.as_ref().and_then(|c| c.draft);
            let dc = d.map_or("Undrafted".to_string(), |d| format!("Round {}, pick {} ({})", d.1, d.2, d.0));
            let starts: u32 = p.pro.as_ref().and_then(|c| c.seasons.last()).map_or(0, |s| s.starts);
            let sc = match p.last_role {
                Some(PlayingTime::Starter) => "Starting now".to_string(),
                _ if starts > 0 => format!("{starts} {} last season", afm_engine::world::folk::noun(starts, "start")),
                _ => "Not a starter yet".to_string(),
            };
            (json!({ "value": dc, "note": "His draft" }), json!({ "value": sc, "note": "His role" }), None)
        }
        _ => {
            // His read carried to his last college season.
            let at_end = path.iter().take_while(|r| r["tier"] != 2).last().and_then(|r| r["q"][2].as_f64()).or(start);
            match at_end.and_then(|x| after_college(w, pos, x)) {
                Some((share, rounds, starters, n)) => (
                    json!({
                        "value": format!("{:.0}%", share * 100.0),
                        "note": match rounds { Some((a, b)) if a == b => format!("drafted, usually in round {a}"), Some((a, b)) => format!("drafted, usually rounds {a}-{b}"), None => "drafted".to_string() },
                    }),
                    match starters {
                        Some(s) => json!({ "value": format!("{:.0}%", s * 100.0), "note": "became NFL starters" }),
                        None => json!({ "value": "–", "note": "too few had the time to tell" }),
                    },
                    Some(n),
                ),
                None => (
                    json!({ "value": "–", "note": "too few like him to read" }),
                    json!({ "value": "–", "note": "too few like him to read" }),
                    None,
                ),
            }
        }
    };
    // Players rated like him at his age.
    let near = start.map_or(Vec::new(), |m| alikes(w, pos, age, tier_now, m, id));
    let (_, many) = crate::lane_ratings::pos_words(pos);
    let ghosts: Vec<Value> = near
        .iter()
        .take(12)
        .map(|a| json!({ "name": a.name, "d": a.deltas.iter().map(|x| (x * 10.0).round() / 10.0).collect::<Vec<_>>() }))
        .collect();
    let named: Vec<Value> = near
        .iter()
        .take(5)
        .map(|a| json!({ "name": a.name, "id": a.id, "outcome": a.outcome, "grew": a.reached }))
        .collect();
    let level_marks = super::marks(w, views::team_of(w, p).unwrap_or(TeamRef::Nfl(0)), pos, Rated::Overall);
    let refs: Vec<Value> = [
        level_marks.as_ref().filter(|_| tier_now < 2).map(|m| json!({ "label": format!("Typical {} starter", level_word(level_now).to_lowercase()), "v": m.starter.round() })),
        nfl_marks.as_ref().map(|m| json!({ "label": "Typical NFL starter", "v": m.starter.round() })),
    ]
    .into_iter()
    .flatten()
    .collect();
    let (first, last) = first_last(&p.name);
    let footnote = format!(
        "From your staff's read today, his age and how {many} grow in this world{}. The staff's guess, not a promise.",
        match outcome_n {
            Some(n) => format!(", and {n} past {many} rated like him who signed with an NFL team out of college"),
            None => String::new(),
        }
    );
    Ok(json!({
        "id": id, "name": p.name, "first": first, "last": last, "pos": pos.name(), "own": own,
        "read": ovr.json(),
        "metrics": std::iter::once(json!({ "key": "ovr", "label": "Overall" }))
            .chain(keys.iter().map(|a| json!({ "key": a.key(), "label": a.label() })))
            .collect::<Vec<_>>(),
        "points": points,
        "table": table,
        "path": path,
        "peak": peak.map(|(v, a, lo, hi)| json!({ "v": v.round(), "age": a, "lo": lo.round(), "hi": hi.round() })),
        "draft": draft_cell, "starter": starter_cell,
        "nfl_starter": nfl_marks.map(|m| m.starter.round()),
        "refs": refs,
        "ghosts": ghosts, "alikes": named,
        "kept_note": if points.len() <= 1 { Some("Your staff keeps a read of every player it follows when each season ends. Seasons before show what the record keeps.") } else { None },
        "footnote": footnote,
    }))
}
