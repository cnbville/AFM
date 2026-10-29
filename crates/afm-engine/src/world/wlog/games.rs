//! The game log (world reader lane R1): every game at every level, as
//! kinds of the world log. A game's row (`games-<level>`) is its facts
//! (kind, date, week, site, kickoff, the line, weather, the crowd, the
//! toss), the score by quarter, both team boxes, every player's line and
//! snaps, the drives, the scoring plays, the injuries, the penalties and
//! the officials' calls. College, NFL and other-league engine games also
//! write every snap (`plays-<level>`), and every level sums its plays by
//! situation for the season (`situations`, `team-situations`). A game is
//! named by its level, date and teams (the league too in the other
//! leagues). Nothing here changes a game.

use std::collections::{BTreeMap, HashMap};

use serde_json::{Map, Value, json};

use super::WorldLog;
use crate::game::drives::drives;
use crate::game::{GameResult, GameStats, GameTeam, PlayRecord, PlayType};
use crate::world::game_log::{GameSitu, Situ, bump};

/// A game's facts the caller knows (level, season, date, kind, teams,
/// site, weather...), as a JSON object.
pub type Head = Map<String, Value>;

/// One team's box: the engine's team totals and what the player lines
/// and plays add (touchdowns by kind, sack yards, kicks and returns).
fn team_box(s: &GameStats, side: usize) -> Value {
    let mut b = serde_json::to_value(&s.team[side]).unwrap_or_else(|_| json!({}));
    let lines = s.players[side].values();
    let mut add = Map::new();
    let mut sum = |k: &str, v: f64| {
        let e = add.entry(k.to_string()).or_insert(json!(0.0));
        *e = json!(e.as_f64().unwrap_or(0.0) + v);
    };
    for l in lines {
        sum("pass_td", l.pass_td as f64);
        sum("rush_td", l.rush_td as f64);
        sum("rec_td", l.rec_td as f64);
        sum("sack_yds", l.sack_yds);
        sum("kr", l.kr as f64);
        sum("kr_yds", l.kr_yds);
        sum("kr_td", l.kr_td as f64);
        sum("pr", l.pr as f64);
        sum("pr_yds", l.pr_yds);
        sum("pr_td", l.pr_td as f64);
        sum("xp_att", l.xp_att as f64);
        sum("xp_made", l.xp_made as f64);
        sum("kickoffs", l.kickoffs as f64);
        sum("ko_tb", l.ko_tb as f64);
        sum("punt_yds", l.punt_yds);
        sum("punt_net", l.punt_net);
        sum("punt_in20", l.punt_in20 as f64);
        sum("punt_tb", l.punt_tb as f64);
        sum("qb_hits", l.qb_hits as f64);
        sum("tfl", l.tfl);
        sum("ints", l.ints as f64);
        sum("pbus", l.pbus as f64);
        sum("forced_fumbles", l.forced_fumbles as f64);
        sum("fumble_rec", l.fumble_rec as f64);
        for k in 0..4 {
            sum(["fg_att_u30", "fg_att_30s", "fg_att_40s", "fg_att_50"][k], l.fg_att[k] as f64);
            sum(["fg_made_u30", "fg_made_30s", "fg_made_40s", "fg_made_50"][k], l.fg_made[k] as f64);
        }
    }
    let (mut two, mut two_good, mut def_td, mut blocked_by, mut emergency) = (0, 0, 0, 0, 0);
    for r in s.plays.iter().filter(|r| !r.nullified && r.offense < 2) {
        if r.kind == PlayType::TwoPoint && r.offense == side {
            two += 1;
            two_good += (r.touchdown && !r.turnover) as u32;
        }
        if r.defense_td && r.offense != side {
            def_td += 1;
        }
        if r.blocked && r.offense != side {
            blocked_by += 1;
        }
        if r.emergency && r.offense == side {
            emergency += 1;
        }
    }
    if let Some(m) = b.as_object_mut() {
        m.extend(add);
        m.insert("two_point".into(), json!(two));
        m.insert("two_point_good".into(), json!(two_good));
        m.insert("def_td".into(), json!(def_td));
        m.insert("kicks_blocked".into(), json!(blocked_by));
        m.insert("emergency_snaps".into(), json!(emergency));
    }
    b
}

/// Every scoring play: what it was, the try after it and the score after.
fn scoring(s: &GameStats) -> Vec<Value> {
    let mut out = Vec::new();
    for (i, r) in s.plays.iter().enumerate() {
        if r.nullified
            || r.offense > 1
            || matches!(r.kind, PlayType::ExtraPoint | PlayType::TwoPoint | PlayType::NoPlay)
        {
            continue;
        }
        let what = if r.touchdown {
            "touchdown"
        } else if r.kind == PlayType::FieldGoal && r.kick_good {
            "field goal"
        } else if r.safety {
            "safety"
        } else if r.single {
            "single"
        } else {
            continue;
        };
        let rest = &s.plays[i + 1..];
        let tried =
            rest.iter().take(3).find(|x| matches!(x.kind, PlayType::ExtraPoint | PlayType::TwoPoint) && !x.nullified);
        let after = rest
            .iter()
            .find(|x| !matches!(x.kind, PlayType::ExtraPoint | PlayType::TwoPoint | PlayType::NoPlay))
            .map_or(s.score, |x| x.score);
        let team = if after[0] - r.score[0] >= after[1] - r.score[1] { 0 } else { 1 };
        let tried = tried.map(|x| {
            let good = match x.kind {
                PlayType::ExtraPoint => x.kick_good,
                _ => x.touchdown && !x.turnover,
            };
            json!({ "n": x.n, "kind": if x.kind == PlayType::ExtraPoint { "kick" } else { "two point" }, "good": good })
        });
        out.push(json!({
            "n": r.n, "q": r.quarter, "clock": r.clock, "team": team, "what": what, "kind": format!("{:?}", r.kind),
            "yards": if r.kind == PlayType::FieldGoal { r.kick_distance } else { r.yards },
            "passer": r.passer, "carrier": r.carrier, "target": r.target, "kicker": r.kicker, "returner": r.returner,
            "interceptor": r.interceptor, "defense_td": r.defense_td, "try": tried, "after": after,
        }));
    }
    out
}

/// The officials' game: every close call and how it ended, and the counts.
fn officiating(s: &GameStats, r: &GameResult) -> Value {
    use crate::game::live::Reviewer;
    let mut calls = Vec::new();
    let (mut close, mut wrong, mut booth, mut assist, mut overturned, mut stood_wrong) = (0, 0, 0, 0, 0, 0);
    let mut challenges = [0u32; 2];
    let mut won = [0u32; 2];
    for p in &s.plays {
        let Some(o) = &p.officiating else { continue };
        close += 1;
        wrong += o.wrong as u32;
        overturned += o.overturned as u32;
        stood_wrong += (o.wrong && !o.overturned) as u32;
        match o.reviewer {
            Reviewer::Booth => booth += 1,
            Reviewer::Assist => assist += 1,
            Reviewer::Challenge(t) if t < 2 => {
                challenges[t] += 1;
                won[t] += o.overturned as u32;
            }
            _ => {}
        }
        calls.push(json!({
            "n": p.n, "call": format!("{:?}", o.call), "wrong": o.wrong, "reviewer": format!("{:?}", o.reviewer),
            "overturned": o.overturned, "against": o.against, "note": o.note,
        }));
    }
    json!({
        "close_calls": close, "wrong": wrong, "booth_reviews": booth, "assists": assist,
        "challenges": challenges, "challenges_won": won, "overturned": overturned, "wrong_left_standing": stood_wrong,
        "calls": calls, "crew": r.crew,
    })
}

/// Every penalty: the play, the team, the player, the kind, the yards and
/// whether it was accepted.
fn penalties(s: &GameStats) -> Vec<Value> {
    let mut out = Vec::new();
    for p in &s.plays {
        for (foul, team, id, yards, accepted) in &p.penalties {
            out.push(json!({
                "n": p.n, "q": p.quarter, "team": team, "player": id, "kind": foul.name(), "yards": yards,
                "accepted": accepted, "no_play": p.kind == PlayType::NoPlay,
            }));
        }
    }
    out
}

/// Every injury in the game: who, on which snap, the kind, the weeks he
/// will miss and whether he played on.
fn injuries(s: &GameStats, r: &GameResult) -> Vec<Value> {
    r.injuries
        .iter()
        .map(|(team, inj)| {
            let at = s.plays.iter().find(|p| p.injuries.iter().any(|x| x.id == inj.id));
            json!({
                "team": team, "player": inj.id, "kind": format!("{:?}", inj.kind), "weeks": inj.weeks,
                "plays_on": inj.plays_on, "n": at.map(|p| p.n), "q": at.map(|p| p.quarter),
            })
        })
        .collect()
}

/// Each side's players: the game line with his id and the position he was
/// listed at.
fn player_lines(s: &GameStats, teams: Option<[&GameTeam; 2]>) -> Vec<Vec<Value>> {
    (0..2)
        .map(|side| {
            let pos: std::collections::HashMap<u32, &'static str> =
                teams.map(|t| t[side].players.iter().map(|p| (p.id, p.position.name())).collect()).unwrap_or_default();
            s.players[side]
                .iter()
                .map(|(&id, l)| {
                    let mut v = serde_json::to_value(l).unwrap_or_else(|_| json!({}));
                    if let Some(m) = v.as_object_mut() {
                        m.insert("id".into(), json!(id));
                        if let Some(p) = pos.get(&id) {
                            m.insert("pos".into(), json!(p));
                        }
                    }
                    v
                })
                .collect()
        })
        .collect()
}

/// A game's whole row: the head, then everything the result holds.
pub fn game_row(mut head: Head, r: &GameResult, teams: Option<[&GameTeam; 2]>) -> Value {
    let s = &r.stats;
    head.insert("score".into(), json!(s.score));
    head.insert("quarters".into(), json!(s.quarters));
    head.insert("ot".into(), json!(s.overtime));
    head.insert("snaps".into(), json!(s.plays.len()));
    head.insert("toss".into(), r.toss.map_or(Value::Null, |(t, c)| json!({ "winner": t, "choice": format!("{c:?}") })));
    head.insert("box".into(), Value::Array(vec![team_box(s, 0), team_box(s, 1)]));
    head.insert("players".into(), Value::Array(player_lines(s, teams).into_iter().map(Value::Array).collect()));
    head.insert("drives".into(), serde_json::to_value(drives(s)).unwrap_or(Value::Null));
    head.insert("scoring".into(), Value::Array(scoring(s)));
    head.insert("injuries".into(), Value::Array(injuries(s, r)));
    head.insert("penalties".into(), Value::Array(penalties(s)));
    head.insert("officiating".into(), officiating(s, r));
    head.insert("timeouts_left".into(), json!(s.timeouts_left));
    head.insert(
        "between".into(),
        Value::Array(r.log.iter().map(|(n, t, what)| json!({ "after": n, "team": t, "what": what })).collect()),
    );
    head.insert(
        "decisions".into(),
        Value::Array(r.decisions.iter().map(|(n, t, what)| json!({ "n": n, "team": t, "what": what })).collect()),
    );
    head.insert(
        "trust".into(),
        Value::Array(r.trust.iter().map(|(t, id, d)| json!({ "team": t, "player": id, "change": d })).collect()),
    );
    let mut v = Value::Object(head);
    compact(&mut v);
    v
}

/// A stand-in game's row: the head and the winner (no box score exists).
pub fn standin_row(mut head: Head, home_won: bool, score: Option<[u32; 2]>) -> Value {
    head.insert("standin".into(), json!(true));
    head.insert("home_won".into(), json!(home_won));
    if let Some(sc) = score {
        head.insert("score".into(), json!(sc));
    }
    let mut v = Value::Object(head);
    compact(&mut v);
    v
}

/// The win chance the side with the ball had after a snap: the next snap's
/// chance from its side, or the final.
fn wp_after(s: &GameStats, plays: &[&PlayRecord], i: usize) -> f64 {
    let r = plays[i];
    match plays.get(i + 1) {
        Some(n) if n.offense == r.offense => n.wp_before,
        Some(n) => 1.0 - n.wp_before,
        None => {
            let [a, b] = s.score;
            let mine = if r.offense == 0 { a - b } else { b - a };
            match mine.cmp(&0) {
                std::cmp::Ordering::Greater => 1.0,
                std::cmp::Ordering::Less => 0.0,
                std::cmp::Ordering::Equal => 0.5,
            }
        }
    }
}

/// A game's plays as one JSON array, snap by snap: every field of the
/// play record (the call and its why, the defense, every block, rush win,
/// tackle and miss, the result and its why), with the expected points and
/// the offense's win chance before and after.
pub fn plays(s: &GameStats) -> Value {
    let plays: Vec<&PlayRecord> = s.plays.iter().filter(|r| r.offense < 2).collect();
    Value::Array(
        plays
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let mut v = serde_json::to_value(r).unwrap_or_else(|_| json!({}));
                if let Some(m) = v.as_object_mut() {
                    m.insert("ep_after".into(), json!(round(r.ep_before + r.epa)));
                    m.insert("wp_after".into(), json!(round(wp_after(s, &plays, i))));
                }
                compact(&mut v);
                v
            })
            .collect(),
    )
}

/// The college game kind's words.
pub fn college_kind(k: crate::world::schedule::GameKind) -> (&'static str, Option<u8>) {
    use crate::world::schedule::GameKind;
    match k {
        GameKind::Regular => ("regular", None),
        GameKind::ConferenceTitle => ("conference title", None),
        GameKind::Bowl(_) => ("bowl", None),
        GameKind::Celebration => ("bowl", None),
        GameKind::Playoff(r) => ("playoff", Some(r)),
        GameKind::Title => ("title", None),
    }
}

/// High school situation keys summed for the whole country (group `all`),
/// not by state: the play names, thousands of calls and fronts in every
/// state.
const HS_NATIONAL: &[&str] = &["call|", "defense|"];

/// The situation totals not written yet, by (season, level, group) and
/// by (season, level, team), each a map of keys. Written at the year's
/// close, at a save and when the log closes, then dropped: a season's
/// totals are the sum of its records.
#[derive(Default)]
pub(crate) struct Totals {
    league: HashMap<(i32, &'static str, String), HashMap<String, Situ>>,
    team: HashMap<(i32, &'static str, usize), HashMap<String, Situ>>,
}

impl WorldLog {
    /// Writes a game's row (kind `games-<level>`) and, for an engine game,
    /// its snaps (`plays-<level>`) under the row's level, season, date and
    /// teams.
    pub(crate) fn game(&self, games: &'static str, plays: &'static str, row: &Value, snaps: Option<Value>) {
        self.put(games, row);
        if let Some(snaps) = snaps {
            let mut rec = Map::new();
            for k in ["level", "league", "season", "date", "teams"] {
                if let Some(v) = row.get(k) {
                    rec.insert(k.into(), v.clone());
                }
            }
            rec.insert("plays".into(), snaps);
            self.put(plays, &Value::Object(rec));
        }
    }

    /// Adds one game's situation totals: the league keys under its level
    /// and group (a division, a state, a league), and each side's keys
    /// under its team. High school keeps no team keys (14,000 schools), and
    /// its play names go under the group `all` (`HS_NATIONAL`).
    pub(crate) fn add_situations(
        &mut self,
        season: i32,
        level: &'static str,
        group: &str,
        teams: [usize; 2],
        part: GameSitu,
    ) {
        let t = &mut self.totals;
        let hs = level == "hs";
        let mut national = Vec::new();
        let m = t.league.entry((season, level, group.to_string())).or_default();
        for (k, v) in part.league {
            if hs && HS_NATIONAL.iter().any(|p| k.starts_with(p)) {
                national.push((k, v));
            } else {
                bump(m.entry(k).or_default(), &v);
            }
        }
        if !national.is_empty() {
            let m = t.league.entry((season, level, "all".to_string())).or_default();
            for (k, v) in national {
                bump(m.entry(k).or_default(), &v);
            }
        }
        if hs {
            return;
        }
        for (side, keys) in part.team.into_iter().enumerate() {
            let m = t.team.entry((season, level, teams[side])).or_default();
            for (k, v) in keys {
                bump(m.entry(k).or_default(), &v);
            }
        }
    }

    /// Writes the situation totals held (one record per group and per
    /// team, keys in order) and drops them.
    pub(crate) fn write_totals(&mut self) {
        let t = std::mem::take(&mut self.totals);
        let keys = |m: HashMap<String, Situ>| -> BTreeMap<String, Vec<f64>> {
            m.into_iter().map(|(k, v)| (k, v.iter().map(|x| (x * 100.0).round() / 100.0).collect())).collect()
        };
        let mut league: Vec<_> = t.league.into_iter().collect();
        league.sort_by(|a, b| a.0.cmp(&b.0));
        for ((season, level, group), m) in league {
            self.put("situations", &json!({ "season": season, "level": level, "group": group, "keys": keys(m) }));
        }
        let mut team: Vec<_> = t.team.into_iter().collect();
        team.sort_by(|a, b| a.0.cmp(&b.0));
        for ((season, level, team), m) in team {
            self.put("team-situations", &json!({ "season": season, "level": level, "team": team, "keys": keys(m) }));
        }
    }
}

/// Rounds a decimal the log's way: one place from 10 up, three below; a
/// whole number stays whole.
pub fn round(x: f64) -> f64 {
    if x.fract() == 0.0 || !x.is_finite() {
        x
    } else if x.abs() >= 10.0 {
        (x * 10.0).round() / 10.0
    } else {
        (x * 1000.0).round() / 1000.0
    }
}

/// Makes a row compact: drops every field that is null, false, zero, ""
/// or empty (an array keeps its places), rounds decimals and writes whole
/// decimals as whole numbers.
pub fn compact(v: &mut serde_json::Value) {
    use serde_json::Value;
    match v {
        Value::Object(m) => {
            m.retain(|_, x| {
                compact(x);
                !empty(x)
            });
        }
        Value::Array(a) => a.iter_mut().for_each(compact),
        Value::Number(n) => {
            if let Some(f) = n.as_f64()
                && !n.is_i64()
                && !n.is_u64()
            {
                let r = round(f);
                *v = if r.fract() == 0.0 && r.abs() < 9.0e15 {
                    Value::from(r as i64)
                } else {
                    serde_json::Number::from_f64(r).map_or(Value::Null, Value::Number)
                };
            }
        }
        _ => {}
    }
}

fn empty(v: &serde_json::Value) -> bool {
    use serde_json::Value;
    match v {
        Value::Null => true,
        Value::Bool(b) => !b,
        Value::Number(n) => n.as_f64() == Some(0.0),
        Value::String(s) => s.is_empty(),
        Value::Array(a) => a.is_empty(),
        Value::Object(m) => m.is_empty(),
    }
}
