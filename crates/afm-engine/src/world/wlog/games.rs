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
use super::lean::{full, lean, num};
use crate::game::drives::drives;
use crate::game::{GameResult, GameStats, GameTeam, PlayRecord, PlayType};
use crate::world::game_log::{GameSitu, Situ, bump};

/// A game's facts the caller knows (level, season, date, kind, teams,
/// site, weather...), as a JSON object.
pub type Head = Map<String, Value>;

/// One team's box: the engine's team totals and what the player lines
/// and plays add (touchdowns by kind, sack yards, kicks and returns),
/// every field kept, zeros too.
fn team_box(s: &GameStats, side: usize) -> Value {
    let mut b = match full(&s.team[side]) {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    let mut add: BTreeMap<&'static str, f64> = BTreeMap::new();
    let mut sum = |k: &'static str, v: f64| *add.entry(k).or_default() += v;
    for l in s.players[side].values() {
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
    for r in s.plays.iter().filter(|r| !r.nullified && r.offense < 2) {
        if r.kind == PlayType::TwoPoint && r.offense == side {
            sum("two_point", 1.0);
            sum("two_point_good", (r.touchdown && !r.turnover) as u32 as f64);
        }
        if r.defense_td && r.offense != side {
            sum("def_td", 1.0);
        }
        if r.blocked && r.offense != side {
            sum("kicks_blocked", 1.0);
        }
        if r.emergency && r.offense == side {
            sum("emergency_snaps", 1.0);
        }
    }
    for (k, v) in add {
        b.insert(k.into(), num(v));
    }
    Value::Object(b)
}

/// Every scoring play: what it was, the try after it and the score after.
/// A defensive touchdown (a pick six or a fumble return) counts, and so does
/// a two-point play that follows no score: the college and other leagues'
/// overtime rounds from the third on are two-point plays only.
fn scoring(s: &GameStats) -> Vec<Value> {
    let mut out = Vec::new();
    // The tries already listed with the score they followed.
    let mut taken = Vec::new();
    for (i, r) in s.plays.iter().enumerate() {
        if r.nullified || r.offense > 1 || matches!(r.kind, PlayType::ExtraPoint | PlayType::NoPlay) {
            continue;
        }
        let rest = &s.plays[i + 1..];
        if r.kind == PlayType::TwoPoint {
            if taken.contains(&i) {
                continue;
            }
            // An overtime round's try: its own entry when it scored.
            let after = rest.iter().find(|x| x.kind != PlayType::NoPlay).map_or(s.score, |x| x.score);
            if after == r.score {
                continue;
            }
            let team = if after[0] - r.score[0] >= after[1] - r.score[1] { 0 } else { 1 };
            out.push(json!({
                "n": r.n, "q": r.quarter, "clock": r.clock, "team": team, "what": "two point", "kind": format!("{:?}", r.kind),
                "yards": r.yards, "passer": r.passer, "carrier": r.carrier, "target": r.target, "kicker": r.kicker,
                "returner": r.returner, "interceptor": r.interceptor, "defense_td": r.defense_td, "try": Value::Null,
                "after": after,
            }));
            continue;
        }
        let what = if r.touchdown || r.defense_td {
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
        // Only a touchdown has a try: the first one that stood after it,
        // past flags (a field goal before an overtime round's two-point
        // play keeps none).
        let tried = rest
            .iter()
            .enumerate()
            .take_while(|(_, x)| matches!(x.kind, PlayType::ExtraPoint | PlayType::TwoPoint | PlayType::NoPlay))
            .find(|(_, x)| matches!(x.kind, PlayType::ExtraPoint | PlayType::TwoPoint) && !x.nullified)
            .filter(|_| what == "touchdown");
        let from = tried.map_or(i, |(k, _)| i + 1 + k);
        if from > i {
            taken.push(from);
        }
        let tried = tried.map(|(_, x)| x);
        // The score at the next snap after it (and its try): a no-play and
        // a try wiped out by a flag don't count.
        let after = s.plays[from + 1..]
            .iter()
            .find(|x| {
                x.kind != PlayType::NoPlay
                    && !(x.nullified && matches!(x.kind, PlayType::ExtraPoint | PlayType::TwoPoint))
            })
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
                    let mut v = lean(l);
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

/// Everything a played game's row holds beyond its head, made on the core
/// that played it: the score by quarter, the toss, both boxes, every
/// player's line, the drives, the scoring, injuries, penalties, the
/// officials' calls, timeouts and the moments between plays.
pub fn game_body(r: &GameResult, teams: Option<[&GameTeam; 2]>) -> Head {
    let s = &r.stats;
    let mut b = Map::new();
    b.insert("score".into(), json!(s.score));
    b.insert("quarters".into(), json!(s.quarters));
    b.insert("ot".into(), json!(s.overtime));
    b.insert("snaps".into(), json!(s.plays.len()));
    b.insert("toss".into(), r.toss.map_or(Value::Null, |(t, c)| json!({ "winner": t, "choice": format!("{c:?}") })));
    b.insert("scoring".into(), Value::Array(scoring(s)));
    b.insert("injuries".into(), Value::Array(injuries(s, r)));
    b.insert("penalties".into(), Value::Array(penalties(s)));
    b.insert("officiating".into(), officiating(s, r));
    b.insert("timeouts_left".into(), json!(s.timeouts_left));
    b.insert(
        "between".into(),
        Value::Array(r.log.iter().map(|(n, t, what)| json!({ "after": n, "team": t, "what": what })).collect()),
    );
    b.insert(
        "decisions".into(),
        Value::Array(r.decisions.iter().map(|(n, t, what)| json!({ "n": n, "team": t, "what": what })).collect()),
    );
    b.insert(
        "trust".into(),
        Value::Array(r.trust.iter().map(|(t, id, d)| json!({ "team": t, "player": id, "change": d })).collect()),
    );
    let mut v = Value::Object(b);
    compact(&mut v);
    let Value::Object(mut b) = v else { unreachable!("an object stays an object") };
    // The bulk, lean as it is made.
    b.insert("box".into(), Value::Array(vec![team_box(s, 0), team_box(s, 1)]));
    b.insert("players".into(), Value::Array(player_lines(s, teams).into_iter().map(Value::Array).collect()));
    let d = lean(&drives(s));
    if !empty(&d) {
        b.insert("drives".into(), d);
    }
    b
}

/// A game's whole row: its head, then its body (`game_body`).
pub fn game_row(head: Head, body: Head) -> Value {
    let mut v = Value::Object(head);
    compact(&mut v);
    if let Value::Object(m) = &mut v {
        m.extend(body);
    }
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
                let mut v = lean(*r);
                if let Some(m) = v.as_object_mut() {
                    for (k, x) in [("ep_after", r.ep_before + r.epa), ("wp_after", wp_after(s, &plays, i))] {
                        let x = num(x);
                        if !empty(&x) {
                            m.insert(k.into(), x);
                        }
                    }
                }
                v
            })
            .collect(),
    )
}

/// A game's snaps as their record (`plays-<level>`): the keys that name
/// the game, taken from its head or row (level, league, season, date and
/// teams), and every snap (`plays`).
pub fn snaps_record(name: &Head, snaps: Value) -> Value {
    let mut rec = Map::new();
    for k in ["level", "league", "season", "date", "teams"] {
        if let Some(v) = name.get(k) {
            rec.insert(k.into(), v.clone());
        }
    }
    rec.insert("plays".into(), snaps);
    Value::Object(rec)
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
        if let (Some(snaps), Some(name)) = (snaps, row.as_object()) {
            self.put(plays, &snaps_record(name, snaps));
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

    /// Writes the situation totals held and drops them: one record per
    /// key, `v` its eight totals, under its season, level and group
    /// (`situations`) or team (`team-situations`), in order.
    pub(crate) fn write_totals(&mut self) {
        let t = std::mem::take(&mut self.totals);
        let v = |x: Situ| -> Vec<f64> { x.iter().map(|x| (x * 100.0).round() / 100.0).collect() };
        let sorted = |m: HashMap<String, Situ>| -> BTreeMap<String, Situ> { m.into_iter().collect() };
        let mut league: Vec<_> = t.league.into_iter().collect();
        league.sort_by(|a, b| a.0.cmp(&b.0));
        for ((season, level, group), m) in league {
            for (key, x) in sorted(m) {
                let rec = json!({ "season": season, "level": level, "group": group, "key": key, "v": v(x) });
                self.put("situations", &rec);
            }
        }
        let mut team: Vec<_> = t.team.into_iter().collect();
        team.sort_by(|a, b| a.0.cmp(&b.0));
        for ((season, level, team), m) in team {
            for (key, x) in sorted(m) {
                let rec = json!({ "season": season, "level": level, "team": team, "key": key, "v": v(x) });
                self.put("team-situations", &rec);
            }
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

pub(crate) fn empty(v: &serde_json::Value) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn play(n: u32, offense: usize, kind: PlayType, score: [i32; 2]) -> PlayRecord {
        let mut r = PlayRecord::new(n, 4, 100.0, offense, kind);
        r.score = score;
        r
    }

    /// Minor bug 133: a pick six and the overtime two-point rounds are scores.
    #[test]
    fn the_scoring_list_keeps_pick_sixes_and_two_point_rounds() {
        let mut s = GameStats::default();
        // A pick six by the away side, then its kick.
        let mut six = play(1, 0, PlayType::Pass, [42, 35]);
        six.turnover = true;
        six.defense_td = true;
        s.plays.push(six);
        let mut kick = play(2, 1, PlayType::ExtraPoint, [42, 41]);
        kick.kick_good = true;
        s.plays.push(kick);
        // Third overtime: each side tries for two from the 3; home makes it.
        let mut home = play(3, 0, PlayType::TwoPoint, [42, 42]);
        home.touchdown = true;
        s.plays.push(home);
        s.plays.push(play(4, 1, PlayType::TwoPoint, [44, 42]));
        s.score = [44, 42];
        let list = scoring(&s);
        assert_eq!(list.len(), 2, "{list:?}");
        assert_eq!(list[0]["what"], "touchdown");
        assert_eq!(list[0]["team"], 1);
        assert_eq!(list[0]["try"]["good"], true);
        assert_eq!(list[0]["after"], json!([42, 42]));
        assert_eq!(list[1]["what"], "two point");
        assert_eq!(list[1]["team"], 0);
        assert_eq!(list[1]["after"], json!([44, 42]));
    }
}
