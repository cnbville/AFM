//! The world reader's game log (lane R1): every game at every level,
//! written to the world log on disk (`wlog.rs`). A game's row is its
//! facts (kind, date, week, site, kickoff, the line, weather, the crowd,
//! the toss), the score by quarter, both team boxes, every player's line
//! and snaps, the drives, the scoring plays, the injuries, the penalties
//! and the officials' calls. College, NFL and other-league games also
//! write every snap to the plays stream. Nothing here changes a game.

use serde_json::{Map, Value, json};

use super::wlog::{compact, round};
use crate::game::drives::drives;
use crate::game::{GameResult, GameStats, GameTeam, PlayRecord, PlayType};

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
    head.insert("box".into(), json!([team_box(s, 0), team_box(s, 1)]));
    head.insert("players".into(), json!(player_lines(s, teams)));
    head.insert("drives".into(), serde_json::to_value(drives(s)).unwrap_or(Value::Null));
    head.insert("scoring".into(), json!(scoring(s)));
    head.insert("injuries".into(), json!(injuries(s, r)));
    head.insert("penalties".into(), json!(penalties(s)));
    head.insert("officiating".into(), officiating(s, r));
    head.insert("timeouts_left".into(), json!(s.timeouts_left));
    head.insert(
        "between".into(),
        json!(r.log.iter().map(|(n, t, what)| json!({ "after": n, "team": t, "what": what })).collect::<Vec<_>>()),
    );
    head.insert(
        "decisions".into(),
        json!(r.decisions.iter().map(|(n, t, what)| json!({ "n": n, "team": t, "what": what })).collect::<Vec<_>>()),
    );
    head.insert(
        "trust".into(),
        json!(r.trust.iter().map(|(t, id, d)| json!({ "team": t, "player": id, "change": d })).collect::<Vec<_>>()),
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
pub fn plays_json(s: &GameStats) -> Vec<u8> {
    let plays: Vec<&PlayRecord> = s.plays.iter().filter(|r| r.offense < 2).collect();
    let rows: Vec<Value> = plays
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
        .collect();
    serde_json::to_vec(&rows).unwrap_or_else(|_| b"[]".to_vec())
}

/// The plays stream's line for a game: `{"id":7,"plays":[...]}`.
pub fn plays_line(id: u32, plays: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(plays.len() + 24);
    out.extend_from_slice(format!("{{\"id\":{id},\"plays\":").as_bytes());
    out.extend_from_slice(plays);
    out.push(b'}');
    out
}

/// The college game kind's words.
pub fn college_kind(k: super::schedule::GameKind) -> (&'static str, Option<u8>) {
    use super::schedule::GameKind;
    match k {
        GameKind::Regular => ("regular", None),
        GameKind::ConferenceTitle => ("conference title", None),
        GameKind::Bowl(_) => ("bowl", None),
        GameKind::Celebration => ("bowl", None),
        GameKind::Playoff(r) => ("playoff", Some(r)),
        GameKind::Title => ("title", None),
    }
}
