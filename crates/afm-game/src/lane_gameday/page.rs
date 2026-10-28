//! The game page (section 9 topics 10-12, section 10's game pages): the
//! scoreboard with win probability, key plays, the advanced numbers, the
//! drive chart, every snap with your call beside your staff's, the charts
//! and the officials; and the live game's halftime and final screens.

use std::collections::HashMap;

use afm_engine::game::GameResult;
use afm_engine::game::live::Reviewer;
use afm_engine::game::stats::{GameStats, PlayRecord, PlayType};
use afm_engine::world::World;
use serde_json::{Value, json};

use super::film::Names;
use super::{clock_text, down_text, quarter_text, spot, success, team_ref};
use crate::lane_boards::flow::Wp;
use crate::views;

fn scrimmage(r: &PlayRecord) -> bool {
    matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble) && !r.nullified
}

fn dropback(r: &PlayRecord) -> bool {
    matches!(r.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble) && !r.nullified
}

/// "#4021" in a staff line to the player's name.
pub fn with_names(text: &str, names: &Names) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find('#') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        match digits.parse::<u32>().ok().and_then(|id| names.get(&id)) {
            Some(n) => out.push_str(&n.0),
            None => {
                out.push('#');
                out.push_str(&digits);
            }
        }
        rest = &after[digits.len()..];
    }
    out.push_str(rest);
    out
}

/// Seconds left in the game at a play.
fn left(r: &PlayRecord, qlen: f64) -> f64 {
    if r.quarter > 4 { r.clock.min(300.0) } else { (4 - r.quarter.min(4)) as f64 * qlen + r.clock }
}

/// The home team's win chance through the game (the game boards' model,
/// `lane_boards::flow::Wp`: the level's numbers and the line): before
/// each play, then the final.
pub fn wp_series(s: &GameStats, m: &Wp) -> Vec<(u32, f64)> {
    let home = m.series(&s.plays);
    let mut v: Vec<(u32, f64)> =
        s.plays.iter().zip(home).filter(|(r, _)| r.offense < 2).map(|(r, h)| (r.n, h)).collect();
    let fin = match s.score[0].cmp(&s.score[1]) {
        std::cmp::Ordering::Greater => 1.0,
        std::cmp::Ordering::Less => 0.0,
        _ => 0.5,
    };
    if let Some(last) = s.plays.last() {
        v.push((last.n + 1, fin));
    }
    v
}

/// Each team's totals for the scoreboard and the box.
pub fn totals(s: &GameStats, names: &[String; 2]) -> Value {
    let top = views::possession(s);
    let sides: Vec<Value> = (0..2)
        .map(|k| {
            let t = &s.team[k];
            json!({
                "name": names[k],
                "points": t.points,
                "yards": t.yards.round(),
                "pass": t.pass_yards.round(),
                "rush": t.rush_yards.round(),
                "first_downs": t.first_downs,
                "third": format!("{}/{}", t.third_conv, t.third_att),
                "fourth": format!("{}/{}", t.fourth_conv, t.fourth_att),
                "turnovers": t.turnovers,
                "penalties": format!("{}-{}", t.penalties, t.penalty_yds.round()),
                "top": clock_text(top[k]),
                "plays": t.plays,
                "sacks": s.plays.iter().filter(|r| r.kind == PlayType::Sack && r.offense != k && !r.nullified).count(),
            })
        })
        .collect();
    json!(sides)
}

/// The advanced numbers by team (section 10: the analytics view).
fn advanced(s: &GameStats, names: &[String; 2]) -> Value {
    let sides: Vec<Value> = (0..2)
        .map(|k| {
            let own: Vec<&PlayRecord> = s.plays.iter().filter(|r| r.offense == k && scrimmage(r)).collect();
            let n = own.len().max(1) as f64;
            let epa: f64 = own.iter().map(|r| r.epa).sum();
            let succ = own.iter().filter(|r| success(r)).count() as f64;
            let runs: Vec<&&PlayRecord> = own.iter().filter(|r| r.kind == PlayType::Run).collect();
            let drops: Vec<&&PlayRecord> = own.iter().filter(|r| dropback(r)).collect();
            let passes: Vec<&&PlayRecord> = own.iter().filter(|r| r.kind == PlayType::Pass && !r.throw_away).collect();
            let early: Vec<&&PlayRecord> = own.iter().filter(|r| r.down <= 2).collect();
            let early_pass = early.iter().filter(|r| dropback(r)).count() as f64 / early.len().max(1) as f64;
            let explosive = own
                .iter()
                .filter(|r| {
                    (r.kind == PlayType::Run && r.yards >= 10.0) || (r.kind == PlayType::Pass && r.yards >= 20.0)
                })
                .count();
            let rz: Vec<&PlayRecord> =
                s.plays.iter().filter(|r| r.offense == k && scrimmage(r) && r.yardline_100 <= 20.0).collect();
            // Red-zone trips: drives that reached the 20.
            let mut trips = 0;
            let mut tds = 0;
            let mut in_trip = false;
            for r in s.plays.iter().filter(|r| r.offense < 2) {
                if r.offense != k
                    || !matches!(
                        r.kind,
                        PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble | PlayType::FieldGoal
                    )
                {
                    if r.offense != k {
                        in_trip = false;
                    }
                    continue;
                }
                if r.yardline_100 <= 20.0 && !in_trip {
                    in_trip = true;
                    trips += 1;
                }
                if in_trip && r.touchdown && !r.defense_td {
                    tds += 1;
                    in_trip = false;
                }
            }
            let _ = rz;
            let pressured = drops.iter().filter(|r| r.pressured || r.kind == PlayType::Sack).count() as f64;
            let air: f64 = passes.iter().map(|r| r.air_yards).sum::<f64>() / passes.len().max(1) as f64;
            let yac: f64 = passes.iter().filter(|r| r.complete).map(|r| r.yac).sum::<f64>()
                / passes.iter().filter(|r| r.complete).count().max(1) as f64;
            let ttt: f64 = passes.iter().map(|r| r.time_to_throw).sum::<f64>() / passes.len().max(1) as f64;
            let ryds: f64 = runs.iter().map(|r| r.yards).sum::<f64>() / runs.len().max(1) as f64;
            let bc: f64 = runs.iter().map(|r| r.before_contact).sum::<f64>() / runs.len().max(1) as f64;
            let cmp = passes.iter().filter(|r| r.complete).count() as f64 / passes.len().max(1) as f64;
            let starts: Vec<f64> = drives(s).iter().filter(|d| d.team == k).map(|d| 100.0 - d.start).collect();
            let start = starts.iter().sum::<f64>() / starts.len().max(1) as f64;
            json!({
                "name": names[k],
                "plays": own.len(),
                "epa_play": epa / n,
                "epa": epa,
                "success": 100.0 * succ / n,
                "explosive": explosive,
                "early_pass": 100.0 * early_pass,
                "rz": format!("{tds}/{trips}"),
                "pressure": 100.0 * pressured / drops.len().max(1) as f64,
                "adot": air,
                "yac": yac,
                "ttt": ttt,
                "ypc": ryds,
                "before_contact": bc,
                "cmp": 100.0 * cmp,
                "start": start,
                "run_epa": runs.iter().map(|r| r.epa).sum::<f64>() / runs.len().max(1) as f64,
                "pass_epa": drops.iter().map(|r| r.epa).sum::<f64>() / drops.len().max(1) as f64,
            })
        })
        .collect();
    json!(sides)
}

/// A drive: who had it, where it started (yards from its own goal line),
/// the plays, yards, time and how it ended.
struct Drive {
    team: usize,
    q: u8,
    clock: f64,
    start: f64,
    end: f64,
    plays: u32,
    secs: f64,
    result: &'static str,
    points: i32,
    first: u32,
    last: u32,
}

fn drives(s: &GameStats) -> Vec<Drive> {
    let mut out: Vec<Drive> = Vec::new();
    let mut cur: Option<Drive> = None;
    let qlen = s.plays.iter().map(|r| r.clock).fold(0.0, f64::max).max(720.0);
    let at = |r: &PlayRecord| left(r, qlen);
    let close = |d: &mut Option<Drive>, out: &mut Vec<Drive>| {
        if let Some(x) = d.take() {
            out.push(x);
        }
    };
    for r in s.plays.iter().filter(|r| r.offense < 2) {
        let snap = matches!(
            r.kind,
            PlayType::Run
                | PlayType::Pass
                | PlayType::Sack
                | PlayType::Scramble
                | PlayType::Punt
                | PlayType::FieldGoal
                | PlayType::Kneel
                | PlayType::Spike
                | PlayType::NoPlay
        );
        if !snap {
            continue;
        }
        if cur.as_ref().is_some_and(|d| d.team != r.offense || (r.quarter == 3 && d.q <= 2)) {
            let d = cur.as_mut().unwrap();
            // A side that knelt and still gave the ball back (the clock
            // didn't run out) turned it over on downs.
            if d.result.is_empty() || (d.result == "End of game" && d.team != r.offense) {
                d.result = if d.q <= 2 && r.quarter >= 3 { "End of half" } else { "Downs" };
            }
            close(&mut cur, &mut out);
        }
        let d = cur.get_or_insert(Drive {
            team: r.offense,
            q: r.quarter,
            clock: r.clock,
            start: 100.0 - r.yardline_100,
            end: 100.0 - r.yardline_100,
            plays: 0,
            secs: 0.0,
            result: "",
            points: 0,
            first: r.n,
            last: r.n,
        });
        if d.result == "End of game" {
            // A kneel with time still on the clock: the same drive goes on.
            d.result = "";
        } else if !d.result.is_empty() {
            // The drive already ended (a score or a kick) but the same team
            // has the ball again: a new drive.
            let team = d.team;
            close(&mut cur, &mut out);
            cur = Some(Drive {
                team,
                q: r.quarter,
                clock: r.clock,
                start: 100.0 - r.yardline_100,
                end: 100.0 - r.yardline_100,
                plays: 0,
                secs: 0.0,
                result: "",
                points: 0,
                first: r.n,
                last: r.n,
            });
        }
        let d = cur.as_mut().unwrap();
        if r.kind != PlayType::NoPlay {
            d.plays += 1;
        }
        d.last = r.n;
        d.end = 100.0 - r.yardline_100 + if scrimmage(r) && !r.turnover { r.yards } else { 0.0 };
        let start_left = left(&PlayRecord { quarter: d.q, clock: d.clock, ..r.clone() }, qlen);
        // The last play's seconds, up to the clock it had (a snap at 0:02
        // that runs eight seconds ends the quarter at 0:00).
        d.secs = (start_left - at(r) + r.duration.max(0.0).min(r.clock.max(0.0))).max(0.0);
        if r.kind == PlayType::Punt && r.touchdown && !r.turnover {
            // Returned: the punting side's drive gave up the points.
            d.result = "Returned for a TD";
            d.points = -6;
        } else if r.touchdown && !r.defense_td {
            d.result = "Touchdown";
            d.points = 6;
        } else if r.defense_td {
            d.result = if r.interception { "Pick six" } else { "Returned for a TD" };
            d.points = -6;
        } else if r.kind == PlayType::FieldGoal {
            d.result = if r.kick_good {
                "Field goal"
            } else if r.blocked {
                "Blocked FG"
            } else {
                "Missed FG"
            };
            d.points = if r.kick_good { 3 } else { 0 };
        } else if r.kind == PlayType::Punt {
            d.result = "Punt";
        } else if r.safety {
            d.result = "Safety";
            d.points = -2;
        } else if r.interception {
            d.result = "Interception";
        } else if r.fumble_lost {
            d.result = "Fumble";
        } else if r.turnover {
            d.result = "Downs";
        } else if r.kind == PlayType::Kneel && r.quarter >= 4 {
            d.result = "End of game";
        }
    }
    if let Some(mut d) = cur.take() {
        if d.result.is_empty() {
            d.result = "End of game";
        }
        out.push(d);
    }
    out
}

/// Every drive of a game as the drive chart shows it (the game flow
/// board reads them too).
pub(crate) fn drives_json(s: &GameStats, names: &[String; 2]) -> Vec<Value> {
    drives(s).iter().map(|d| drive_json(d, names)).collect()
}

fn drive_json(d: &Drive, names: &[String; 2]) -> Value {
    json!({
        "team": d.team,
        "name": names[d.team],
        "q": quarter_text(d.q, d.q > 4),
        "clock": clock_text(d.clock),
        "start": spot(100.0 - d.start),
        "start_x": d.start,
        "end_x": d.end.clamp(0.0, 100.0),
        "plays": d.plays,
        "yards": (d.end - d.start).round(),
        "time": clock_text(d.secs),
        "result": d.result,
        "points": d.points,
        "first": d.first,
        "last": d.last,
    })
}

/// Every score in order, as a box score lists them: what it was, the
/// play and its try, the drive it ended and the score after it.
fn scoring_summary(s: &GameStats, names: &Names, dr: &[Drive]) -> Vec<Value> {
    let name = |id: u32| names.get(&id).map_or(format!("No. {id}"), |x| x.0.clone());
    let mut out = Vec::new();
    for (i, r) in s.plays.iter().enumerate() {
        if r.nullified || matches!(r.kind, PlayType::ExtraPoint | PlayType::TwoPoint | PlayType::NoPlay) {
            continue;
        }
        let what = if r.touchdown {
            "Touchdown"
        } else if r.kind == PlayType::FieldGoal && r.kick_good {
            "Field goal"
        } else if r.safety {
            "Safety"
        } else {
            continue;
        };
        let rest = &s.plays[i + 1..];
        // The try right after a touchdown.
        let tried =
            rest.iter().take(3).find(|x| matches!(x.kind, PlayType::ExtraPoint | PlayType::TwoPoint) && !x.nullified);
        let after = rest
            .iter()
            .find(|x| !matches!(x.kind, PlayType::ExtraPoint | PlayType::TwoPoint | PlayType::NoPlay))
            .map_or(s.score, |x| x.score);
        let team = if after[0] - r.score[0] >= after[1] - r.score[1] { 0 } else { 1 };
        let drive = dr.iter().find(|d| d.team == team && d.first <= r.n && r.n <= d.last && d.plays > 0);
        let mut linked = who(r, names);
        if let Some(x) = tried {
            linked.extend(who(x, names));
        }
        out.push(json!({
            "n": r.n,
            "q": quarter_text(r.quarter, r.quarter > 4),
            "clock": clock_text(r.clock),
            "side": team,
            "what": what,
            "text": views::play_text(r, &name),
            "try": tried.map(|x| views::play_text(x, &name)),
            "who": linked,
            "drive": drive.map(|d| format!("{} {}, {} yards, {}", d.plays, if d.plays == 1 { "play" } else { "plays" }, (d.end - d.start).round(), clock_text(d.secs))),
            "score": after,
        }));
    }
    out
}

fn play_line(r: &PlayRecord, names: &Names, sides: &[String; 2], wp: f64, swing: f64) -> Value {
    let name = |id: u32| names.get(&id).map_or(format!("No. {id}"), |x| x.0.clone());
    let downless = matches!(r.kind, PlayType::Kickoff | PlayType::ExtraPoint | PlayType::TwoPoint);
    json!({
        "n": r.n,
        "q": quarter_text(r.quarter, r.quarter > 4),
        "clock": clock_text(r.clock),
        "off": sides[r.offense.min(1)],
        "team": r.offense.min(1),
        "dd": if downless { String::new() } else { format!("{} at {}", down_text(r.down, r.togo, r.yardline_100), spot(r.yardline_100)) },
        "text": views::play_text(r, &name),
        "call": r.call,
        "defense": r.defense_call,
        "why": r.call_why,
        "yours": r.yours,
        "staff_call": r.staff_call,
        "off_sheet": r.off_sheet,
        "epa": r.epa,
        "success": scrimmage(r) && success(r),
        "yards": r.yards,
        "score": r.score,
        "wp_home": wp,
        "swing": swing,
        "officiating": r.officiating.as_ref().map(|o| json!({ "note": with_names(&o.note, names), "wrong": o.wrong, "reviewed": o.reviewer != Reviewer::None, "overturned": o.overturned })),
        "kind": format!("{:?}", r.kind),
        "x": 100.0 - r.yardline_100,
        "td": r.touchdown,
        "turnover": r.turnover,
        "who": who(r, names),
        "look": look(r),
    })
}

/// Everyone a play names, for its links: (id, name).
pub(crate) fn who(r: &PlayRecord, names: &Names) -> Vec<(u32, String)> {
    let mut ids: Vec<u32> = [
        r.passer,
        r.target,
        r.carrier,
        r.defender,
        r.pbu,
        r.interceptor,
        r.forced_fumble,
        r.fumbled,
        r.kicker,
        r.returner,
    ]
    .into_iter()
    .flatten()
    .collect();
    ids.extend(r.tacklers.iter().chain(&r.sackers).copied());
    ids.extend(r.injuries.iter().map(|i| i.id));
    let mut out: Vec<(u32, String)> = Vec::new();
    for id in ids {
        if !out.iter().any(|x| x.0 == id)
            && let Some(n) = names.get(&id)
        {
            out.push((id, n.0.clone()));
        }
    }
    out
}

/// The pre-snap look the booth saw: personnel and the box.
fn look(r: &PlayRecord) -> String {
    if r.personnel.is_empty()
        || !matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble | PlayType::TwoPoint)
    {
        return String::new();
    }
    let mut t = format!("{} personnel", r.personnel);
    if r.box_count > 0 {
        t.push_str(&format!(", {} in the box", r.box_count));
    }
    if r.blitz {
        t.push_str(", blitz");
    }
    t
}

/// The win-probability change of each play (home), by play number.
fn swings(s: &GameStats, m: &Wp) -> HashMap<u32, (f64, f64)> {
    let series = wp_series(s, m);
    let mut m = HashMap::new();
    for w in series.windows(2) {
        m.insert(w[0].0, (w[0].1, w[1].1 - w[0].1));
    }
    m
}

fn key_plays(s: &GameStats, names: &Names, sides: &[String; 2], m: &Wp, mine: Option<usize>, n: usize) -> Vec<Value> {
    let sw = swings(s, m);
    let mut v: Vec<(&PlayRecord, f64, f64)> = s
        .plays
        .iter()
        .filter(|r| r.offense < 2 && r.kind != PlayType::NoPlay)
        .map(|r| {
            let (wp, d) = sw.get(&r.n).copied().unwrap_or((0.5, 0.0));
            (r, wp, d)
        })
        .filter(|(r, _, d)| {
            d.abs() >= 0.06
                || r.touchdown
                || r.turnover
                || r.safety
                || (r.kind == PlayType::FieldGoal)
                || r.yards >= 25.0
                || r.officiating.as_ref().is_some_and(|o| o.reviewer != Reviewer::None)
                || mine.is_some_and(|m| {
                    r.yours != 0 && r.offense < 2 && (r.offense == m) == (r.yours & 1 != 0) && d.abs() >= 0.04
                })
        })
        .collect();
    v.sort_by(|a, b| b.2.abs().total_cmp(&a.2.abs()));
    v.truncate(n);
    v.sort_by_key(|x| x.0.n);
    v.into_iter().map(|(r, wp, d)| play_line(r, names, sides, wp, d)).collect()
}

/// Your calls against your staff's in a game (topic 11): your snaps and
/// theirs, EPA a play and success rate.
fn calls(s: &GameStats, mine: usize) -> Value {
    let mut y = (0u32, 0.0f64, 0u32, 0u32);
    let mut st = (0u32, 0.0f64, 0u32);
    for r in s.plays.iter().filter(|r| scrimmage(r)) {
        let ours = r.offense == mine;
        let yours = (ours && r.yours & 1 != 0) || (!ours && r.yours & 2 != 0);
        let epa = if ours { r.epa } else { -r.epa };
        let ok = success(r) == ours;
        if yours {
            y.0 += 1;
            y.1 += epa;
            y.2 += ok as u32;
            y.3 += r.off_sheet as u32;
        } else {
            st.0 += 1;
            st.1 += epa;
            st.2 += ok as u32;
        }
    }
    let row = |n: u32, e: f64, k: u32| json!({ "snaps": n, "epa": if n > 0 { e / n as f64 } else { 0.0 }, "success": if n > 0 { 100.0 * k as f64 / n as f64 } else { 0.0 } });
    json!({ "yours": row(y.0, y.1, y.2), "off_sheet": y.3, "staff": row(st.0, st.1, st.2) })
}

fn officiating(s: &GameStats, names: &[String; 2], people: &Names) -> Value {
    let rows: Vec<Value> = s
        .plays
        .iter()
        .filter_map(|r| {
            let o = r.officiating.as_ref()?;
            Some(json!({
                "n": r.n,
                "q": quarter_text(r.quarter, r.quarter > 4),
                "clock": clock_text(r.clock),
                "call": o.call.name(),
                "note": with_names(&o.note, people),
                "wrong": o.wrong,
                "who": match o.reviewer { Reviewer::None => "nobody".to_string(), Reviewer::Booth => "the booth".into(), Reviewer::Assist => "replay assist".into(), Reviewer::Challenge(t) => format!("{} challenge", names[t.min(1)]) },
                "overturned": o.overturned,
                "against": names[o.against.min(1)],
            }))
        })
        .collect();
    let wrong = rows.iter().filter(|r| r["wrong"] == true).count();
    let fixed = rows.iter().filter(|r| r["wrong"] == true && r["overturned"] == true).count();
    // Every flag thrown, with the player it was on.
    let flags: Vec<Value> = s
        .plays
        .iter()
        .flat_map(|r| {
            r.penalties.iter().map(move |(f, t, id, y, accepted)| {
                json!({
                    "n": r.n,
                    "q": quarter_text(r.quarter, r.quarter > 4),
                    "clock": clock_text(r.clock),
                    "foul": f.name(),
                    "side": (*t).min(1),
                    "team": names[(*t).min(1)],
                    "id": id,
                    "name": people.get(id).map_or(String::new(), |x| x.0.clone()),
                    "pos": people.get(id).map_or(String::new(), |x| x.1.clone()),
                    "yards": y.round(),
                    "accepted": accepted,
                    "pre_snap": r.kind == PlayType::NoPlay,
                })
            })
        })
        .collect();
    json!({ "rows": rows, "close": rows.len(), "wrong": wrong, "fixed": fixed, "flags": flags })
}

/// Each team's EPA by quarter, for the charts.
fn epa_by_quarter(s: &GameStats) -> Value {
    let qs = s.plays.iter().map(|r| r.quarter).max().unwrap_or(4).max(4) as usize;
    let mut v = vec![[0.0f64; 2]; qs];
    for r in s.plays.iter().filter(|r| scrimmage(r)) {
        v[(r.quarter.max(1) - 1) as usize][r.offense.min(1)] += r.epa;
    }
    json!(v)
}

/// Top performers: game balls (topic 11).
fn stars(s: &GameStats, names: &Names, sides: &[String; 2]) -> Vec<Value> {
    let mut v: Vec<(f64, Value)> = Vec::new();
    let name = |id: u32| names.get(&id).map_or(format!("No. {id}"), |x| x.0.clone());
    for (k, side) in s.players.iter().enumerate() {
        for (&id, l) in side {
            let mut score = 0.0;
            let mut line = Vec::new();
            if l.att > 0 {
                score += l.pass_yds / 25.0 + 4.0 * l.pass_td as f64 - 3.0 * l.int as f64;
                line.push(format!(
                    "{}/{}, {:.0} yds, {} TD{}",
                    l.cmp,
                    l.att,
                    l.pass_yds,
                    l.pass_td,
                    if l.int > 0 { format!(", {} INT", l.int) } else { String::new() }
                ));
            }
            if l.rush > 0 {
                score += l.rush_yds / 10.0 + 6.0 * l.rush_td as f64;
                if l.rush >= 5 || l.rush_td > 0 {
                    line.push(format!(
                        "{} {}, {:.0} yds{}",
                        l.rush,
                        if l.rush == 1 { "carry" } else { "carries" },
                        l.rush_yds,
                        if l.rush_td > 0 { format!(", {} TD", l.rush_td) } else { String::new() }
                    ));
                }
            }
            if l.rec > 0 {
                score += l.rec_yds / 10.0 + 6.0 * l.rec_td as f64;
                if l.rec >= 3 || l.rec_td > 0 {
                    line.push(format!(
                        "{} {}, {:.0} yds{}",
                        l.rec,
                        if l.rec == 1 { "catch" } else { "catches" },
                        l.rec_yds,
                        if l.rec_td > 0 { format!(", {} TD", l.rec_td) } else { String::new() }
                    ));
                }
            }
            let tk = l.tackles + l.assists;
            if tk > 0 || l.sacks > 0.0 || l.ints > 0 {
                score += tk as f64 * 0.8 + l.sacks * 4.0 + l.ints as f64 * 5.0 + l.tfl * 1.5 + l.pbus as f64;
                if tk >= 6 || l.sacks > 0.0 || l.ints > 0 {
                    let mut d = Vec::new();
                    if tk > 0 {
                        d.push(format!("{tk} {}", if tk == 1 { "tackle" } else { "tackles" }));
                    }
                    if l.sacks > 0.0 {
                        d.push(format!("{} {}", l.sacks, if l.sacks == 1.0 { "sack" } else { "sacks" }));
                    }
                    if l.ints > 0 {
                        d.push(format!("{} INT", l.ints));
                    }
                    line.push(d.join(", "));
                }
            }
            if !line.is_empty() {
                v.push((score, json!({ "id": id, "name": name(id), "pos": names.get(&id).map(|x| x.1.clone()), "team": sides[k], "side": k, "line": line.join("; ") })));
            }
        }
    }
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    v.into_iter().take(6).map(|x| x.1).collect()
}

/// A whole game of yours (topic 11): the base box score and play by play,
/// with the scoreboard's win chance, key plays, the advanced numbers, the
/// drive chart, every snap, the charts and the officials.
pub fn game_page(w: &World, k: usize) -> Result<Value, String> {
    let mut v = views::game(w, k)?;
    let g = w.seat_games().get(k).ok_or("That game is not kept.")?;
    let s = &g.stats;
    let refs = [team_ref(g.level, g.teams[0]), team_ref(g.level, g.teams[1])];
    let names = crate::lane_boards::game_names(w, g, &refs);
    let mine = w.seat_team().and_then(|(_, t)| g.teams.iter().position(|&x| x == t));
    let m = Wp::of_game(w, g);
    let sw = swings(s, &m);
    let snaps: Vec<Value> = s
        .plays
        .iter()
        .filter(|r| r.offense < 2)
        .map(|r| {
            let (wp, d) = sw.get(&r.n).copied().unwrap_or((0.5, 0.0));
            play_line(r, &names, &g.names, wp, d)
        })
        .collect();
    let dr = drives(s);
    v["home"] = views::team_brief(w, refs[0]);
    v["away"] = views::team_brief(w, refs[1]);
    v["level"] = json!(match g.level {
        afm_engine::calendar::Level::Nfl => "NFL",
        afm_engine::calendar::Level::HighSchool => "High school",
        afm_engine::calendar::Level::Other => {
            afm_engine::world::pro::other::league_short(afm_engine::world::pro::other::league_of(g.teams[0]))
        }
        _ => "College",
    });
    v["mine"] = json!(mine);
    v["wp"] = json!(wp_series(s, &m));
    v["key_plays"] = json!(key_plays(s, &names, &g.names, &m, mine, 14));
    v["advanced"] = advanced(s, &g.names);
    v["drives"] = json!(dr.iter().map(|d| drive_json(d, &g.names)).collect::<Vec<_>>());
    v["scoring"] = json!(scoring_summary(s, &names, &dr));
    v["snaps"] = json!(snaps);
    v["epa_q"] = epa_by_quarter(s);
    v["officials"] = officiating(s, &g.names, &names);
    v["stars"] = json!(stars(s, &names, &g.names));
    v["calls"] = mine.map_or(Value::Null, |m| calls(s, m));
    v["totals"] = totals(s, &g.names);
    v["box"] = box_plus(&v, s);
    v["snap_counts"] = snap_counts(s, &names);
    v["passes"] = json!(passes(s, &names));
    v["gaps"] = json!([gaps(s, 0), gaps(s, 1)]);
    v["halves"] = halves(s, &g.names);
    v["fourths"] = json!(fourths(s, &g.names));
    v["index"] = json!(k);
    Ok(v)
}

/// The box score's extra columns (screens topic 10): passing clean and
/// under pressure with the depth of target, yards before contact, air
/// yards, yards after the catch and separation, pressures, the kickers
/// and the returners.
fn box_plus(v: &Value, s: &GameStats) -> Value {
    let live: Vec<&PlayRecord> = s.plays.iter().filter(|r| !r.nullified && r.offense < 2).collect();
    let mut sides = Vec::new();
    for k in 0..2 {
        let own: Vec<&&PlayRecord> = live.iter().filter(|r| r.offense == k).collect();
        let opp: Vec<&&PlayRecord> = live.iter().filter(|r| r.offense != k).collect();
        let rows = |key: &str| v["sides"][k][key].as_array().cloned().unwrap_or_default();
        let id_of = |row: &Value| row["id"].as_u64().unwrap_or(0) as u32;
        let passing: Vec<Value> = rows("passing")
            .into_iter()
            .map(|mut row| {
                let id = id_of(&row);
                let thr: Vec<&&&PlayRecord> =
                    own.iter().filter(|r| r.kind == PlayType::Pass && r.passer == Some(id)).collect();
                let split = |p: bool| {
                    let v: Vec<&&&&PlayRecord> = thr.iter().filter(|r| r.pressured == p).collect();
                    format!("{}/{}", v.iter().filter(|r| r.complete).count(), v.len())
                };
                let aimed: Vec<f64> = thr.iter().filter(|r| !r.throw_away).map(|r| r.air_yards).collect();
                row["clean"] = json!(split(false));
                row["pressured"] = json!(split(true));
                row["adot"] =
                    if aimed.is_empty() { Value::Null } else { json!(aimed.iter().sum::<f64>() / aimed.len() as f64) };
                row
            })
            .collect();
        let rushing: Vec<Value> = rows("rushing")
            .into_iter()
            .map(|mut row| {
                let id = id_of(&row);
                let runs: Vec<f64> = own
                    .iter()
                    .filter(|r| r.kind == PlayType::Run && r.carrier == Some(id))
                    .map(|r| r.before_contact)
                    .collect();
                row["bc"] =
                    if runs.is_empty() { Value::Null } else { json!(runs.iter().sum::<f64>() / runs.len() as f64) };
                row
            })
            .collect();
        let receiving: Vec<Value> = rows("receiving")
            .into_iter()
            .map(|mut row| {
                let id = id_of(&row);
                let tg: Vec<&&&PlayRecord> =
                    own.iter().filter(|r| r.kind == PlayType::Pass && r.target == Some(id)).collect();
                let caught: Vec<&&&&PlayRecord> = tg.iter().filter(|r| r.complete).collect();
                row["air"] = json!(caught.iter().map(|r| r.air_yards).sum::<f64>());
                row["yac"] = json!(caught.iter().map(|r| r.yac).sum::<f64>());
                row["sep"] = if caught.is_empty() {
                    Value::Null
                } else {
                    json!(caught.iter().map(|r| r.separation).sum::<f64>() / caught.len() as f64)
                };
                row
            })
            .collect();
        let defense: Vec<Value> = rows("defense")
            .into_iter()
            .map(|mut row| {
                let id = id_of(&row);
                row["pres"] =
                    json!(opp.iter().filter(|r| r.pressures.contains(&id) || r.sackers.contains(&id)).count());
                row
            })
            .collect();
        // Kickers and punters.
        let mut kick: Vec<(u32, [u32; 6], f64, f64)> = Vec::new();
        for r in own.iter().filter(|r| matches!(r.kind, PlayType::FieldGoal | PlayType::ExtraPoint | PlayType::Punt)) {
            let Some(id) = r.kicker else { continue };
            let i = match kick.iter().position(|x| x.0 == id) {
                Some(i) => i,
                None => {
                    kick.push((id, [0; 6], 0.0, 0.0));
                    kick.len() - 1
                }
            };
            let e = &mut kick[i];
            match r.kind {
                PlayType::FieldGoal => {
                    e.1[0] += r.kick_good as u32;
                    e.1[1] += 1;
                    if r.kick_good {
                        e.3 = e.3.max(r.kick_distance);
                    }
                }
                PlayType::ExtraPoint => {
                    e.1[2] += r.kick_good as u32;
                    e.1[3] += 1;
                }
                _ => {
                    e.1[4] += 1;
                    e.1[5] += r.inside20 as u32;
                    e.2 += r.kick_distance;
                }
            }
        }
        let names_of = |id: u32| -> String {
            ["passing", "rushing", "receiving", "defense"]
                .iter()
                .flat_map(|k2| rows(k2))
                .find(|row| id_of(row) == id)
                .and_then(|row| row["name"].as_str().map(str::to_string))
                .unwrap_or_default()
        };
        let kicking: Vec<Value> = kick
            .iter()
            .map(|(id, c, dist, long)| {
                json!({ "id": id, "name": names_of(*id), "fg": format!("{}/{}", c[0], c[1]), "long": if *long > 0.0 { json!(long.round()) } else { Value::Null }, "xp": format!("{}/{}", c[2], c[3]), "punts": c[4], "avg": if c[4] > 0 { json!(dist / c[4] as f64) } else { Value::Null }, "in20": c[5] })
            })
            .collect();
        // Returners: the other team's kicks.
        let mut ret: Vec<(u32, [u32; 2], [f64; 2], f64)> = Vec::new();
        for r in opp.iter().filter(|r| matches!(r.kind, PlayType::Kickoff | PlayType::Punt)) {
            let Some(id) = r.returner else { continue };
            if r.touchback || r.fair_catch {
                continue;
            }
            let i = match ret.iter().position(|x| x.0 == id) {
                Some(i) => i,
                None => {
                    ret.push((id, [0; 2], [0.0; 2], 0.0));
                    ret.len() - 1
                }
            };
            let k2 = (r.kind == PlayType::Punt) as usize;
            ret[i].1[k2] += 1;
            ret[i].2[k2] += r.return_yards;
            ret[i].3 = ret[i].3.max(r.return_yards);
        }
        let returns: Vec<Value> = ret
            .iter()
            .map(|(id, n, y, long)| json!({ "id": id, "name": names_of(*id), "kr": n[0], "kr_yds": y[0].round(), "pr": n[1], "pr_yds": y[1].round(), "long": long.round() }))
            .collect();
        sides.push(json!({ "passing": passing, "rushing": rushing, "receiving": receiving, "defense": defense, "kicking": kicking, "returns": returns }));
    }
    json!(sides)
}

/// Snaps by personnel and package, and every player's snaps (the
/// rotations), from who was on the field.
fn snap_counts(s: &GameStats, names: &Names) -> Value {
    let pos = |id: u32| names.get(&id).map_or("", |x| x.1.as_str());
    let plays: Vec<&PlayRecord> =
        s.plays.iter().filter(|r| r.offense < 2 && !r.nullified && r.kind != PlayType::NoPlay).collect();
    let offense_snap = |r: &PlayRecord| {
        matches!(
            r.kind,
            PlayType::Run
                | PlayType::Pass
                | PlayType::Sack
                | PlayType::Scramble
                | PlayType::Kneel
                | PlayType::Spike
                | PlayType::TwoPoint
        )
    };
    let package = |r: &PlayRecord| -> &'static str {
        let d = &r.on_field[1];
        let dbs = d.iter().filter(|&&id| matches!(pos(id), "CB" | "S")).count();
        let big = d.iter().filter(|&&id| matches!(pos(id), "DT" | "EDGE" | "LB")).count();
        if big >= 8 {
            "Goal line"
        } else if dbs <= 4 {
            "Base"
        } else if dbs == 5 {
            "Nickel"
        } else if dbs == 6 {
            "Dime"
        } else {
            "Quarter"
        }
    };
    let mut personnel = Vec::new();
    let mut packages = Vec::new();
    let mut players = Vec::new();
    for k in 0..2 {
        // Called plays only: tries, kneels and spikes aren't a choice of
        // personnel.
        let own: Vec<&&PlayRecord> = plays
            .iter()
            .filter(|r| {
                r.offense == k
                    && offense_snap(r)
                    && !matches!(r.kind, PlayType::TwoPoint | PlayType::Kneel | PlayType::Spike)
            })
            .collect();
        let total = own.len().max(1) as f64;
        let mut by: Vec<(String, Vec<&&&PlayRecord>)> = Vec::new();
        for r in &own {
            let key = if r.personnel.is_empty() { "Other".to_string() } else { r.personnel.clone() };
            match by.iter_mut().find(|x| x.0 == key) {
                Some(x) => x.1.push(r),
                None => by.push((key, vec![r])),
            }
        }
        by.sort_by_key(|b| std::cmp::Reverse(b.1.len()));
        let row = |name: String, v: &[&&&PlayRecord]| {
            let n = v.len().max(1) as f64;
            json!({
                "name": name,
                "n": v.len(),
                "share": (100.0 * v.len() as f64 / total).round(),
                "run": (100.0 * v.iter().filter(|r| r.kind == PlayType::Run).count() as f64 / n).round(),
                "pass": (100.0 * v.iter().filter(|r| dropback(r)).count() as f64 / n).round(),
                "ypp": v.iter().map(|r| r.yards).sum::<f64>() / n,
                "epa": v.iter().map(|r| r.epa).sum::<f64>() / n,
            })
        };
        personnel.push(by.iter().map(|(p, v)| row(format!("{p} personnel"), v)).collect::<Vec<_>>());
        let mut pk: Vec<(&str, Vec<&&&PlayRecord>)> = Vec::new();
        for r in own.iter().filter(|r| !r.on_field[1].is_empty()) {
            let key = package(r);
            match pk.iter_mut().find(|x| x.0 == key) {
                Some(x) => x.1.push(r),
                None => pk.push((key, vec![r])),
            }
        }
        pk.sort_by_key(|b| std::cmp::Reverse(b.1.len()));
        packages.push(
            pk.iter()
                .map(|(p, v)| {
                    let n = v.len().max(1) as f64;
                    json!({ "name": p, "n": v.len(), "share": (100.0 * v.len() as f64 / total).round(), "ypp": v.iter().map(|r| r.yards).sum::<f64>() / n, "epa": v.iter().map(|r| r.epa).sum::<f64>() / n })
                })
                .collect::<Vec<_>>(),
        );
        // The rotations.
        let mut count: HashMap<(u32, &'static str), u32> = HashMap::new();
        let mut units = [0u32; 3];
        for r in &plays {
            let (unit, side) = match (offense_snap(r), r.offense == k) {
                (true, true) => ("OFF", 0),
                (true, false) => ("DEF", 1),
                (false, true) => ("ST", 0),
                (false, false) => ("ST", 1),
            };
            let ui = match unit {
                "OFF" => 0,
                "DEF" => 1,
                _ => 2,
            };
            units[ui] += 1;
            for &id in &r.on_field[side] {
                *count.entry((id, unit)).or_default() += 1;
            }
        }
        let mut list: Vec<Value> = count
            .iter()
            .filter(|((id, _), _)| names.contains_key(id))
            .map(|((id, unit), n)| {
                let ui = match *unit {
                    "OFF" => 0,
                    "DEF" => 1,
                    _ => 2,
                };
                let (nm, p) = names.get(id).cloned().unwrap_or_default();
                json!({ "id": id, "name": nm, "pos": p, "unit": unit, "snaps": n, "share": (100.0 * *n as f64 / units[ui].max(1) as f64).round() })
            })
            .collect();
        list.sort_by(|a, b| b["snaps"].as_u64().cmp(&a["snaps"].as_u64()));
        players.push(list);
    }
    json!({ "personnel": personnel, "packages": packages, "players": players })
}

/// Every aimed throw for the pass chart: its depth and how far right of
/// the snap it arrived (from the passer's view), and how it ended.
fn passes(s: &GameStats, names: &Names) -> Vec<Value> {
    let name = |id: Option<u32>| id.and_then(|i| names.get(&i)).map_or(String::new(), |x| x.0.clone());
    s.plays
        .iter()
        .filter(|r| r.kind == PlayType::Pass && !r.nullified && !r.throw_away && r.offense < 2)
        .filter_map(|r| {
            let (snap, at) = r.throw_y?;
            Some(json!({
                "team": r.offense,
                "air": r.air_yards,
                "y": snap - at,
                "complete": r.complete,
                "td": r.touchdown && !r.defense_td,
                "int": r.interception,
                "text": format!("{} to {}: {}", name(r.passer), name(r.target), if r.interception { "intercepted".to_string() } else if r.complete { format!("{:.0} {}", r.yards, if r.yards.round() == 1.0 { "yard" } else { "yards" }) } else { "incomplete".to_string() }),
            }))
        })
        .collect()
}

/// Designed runs by gap: left end, left inside, middle, right inside,
/// right end (from the offense's view).
fn gaps(s: &GameStats, k: usize) -> Vec<Value> {
    const NAMES: [&str; 5] = ["Left end", "Left inside", "Middle", "Right inside", "Right end"];
    let mut bins = [(0u32, 0.0f64); 5];
    for r in s.plays.iter().filter(|r| r.kind == PlayType::Run && r.offense == k && !r.nullified) {
        let c = r.call.to_lowercase();
        if c.contains(" with ") {
            continue;
        }
        let left = c.ends_with("left");
        let right = c.ends_with("right");
        let outside = ["outside zone", "toss", "sweep", "option"].iter().any(|x| c.contains(x));
        let middle = ["draw", "sneak"].iter().any(|x| c.contains(x)) || !(left || right);
        let i = if middle {
            2
        } else if outside {
            if left { 0 } else { 4 }
        } else if left {
            1
        } else {
            3
        };
        bins[i].0 += 1;
        bins[i].1 += r.yards;
    }
    bins.iter()
        .enumerate()
        .map(|(i, (n, y))| json!({ "name": NAMES[i], "n": n, "ypc": if *n > 0 { y / *n as f64 } else { 0.0 } }))
        .collect()
}

/// The advanced numbers by half (overtime its own).
fn halves(s: &GameStats, names: &[String; 2]) -> Value {
    let ot = s.plays.iter().any(|r| r.quarter > 4);
    let parts: Vec<(u8, u8)> = if ot { vec![(1, 2), (3, 4), (5, 9)] } else { vec![(1, 2), (3, 4)] };
    json!(
        parts
            .iter()
            .map(|&(a, b)| {
                (0..2)
                    .map(|k| {
                        let own: Vec<&PlayRecord> = s.plays.iter().filter(|r| r.offense == k && scrimmage(r) && (a..=b).contains(&r.quarter)).collect();
                        let n = own.len().max(1) as f64;
                        let drops: Vec<&&PlayRecord> = own.iter().filter(|r| dropback(r)).collect();
                        json!({
                            "name": names[k],
                            "plays": own.len(),
                            "epa_play": own.iter().map(|r| r.epa).sum::<f64>() / n,
                            "success": 100.0 * own.iter().filter(|r| success(r)).count() as f64 / n,
                            "explosive": own.iter().filter(|r| (r.kind == PlayType::Run && r.yards >= 10.0) || (r.kind == PlayType::Pass && r.yards >= 20.0)).count(),
                            "pressure": 100.0 * drops.iter().filter(|r| r.pressured || r.kind == PlayType::Sack).count() as f64 / drops.len().max(1) as f64,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    )
}

/// Every 4th down against the model (screens topic 10): what each team
/// did, the true win chance of each choice and what the choice cost.
fn fourths(s: &GameStats, names: &[String; 2]) -> Vec<Value> {
    s.plays
        .iter()
        .filter(|r| r.offense < 2)
        .filter_map(|r| {
            let f = r.fourth?;
            let (did, chosen) = match r.kind {
                PlayType::Punt => ("Punted".to_string(), f[1]),
                PlayType::FieldGoal => (
                    format!(
                        "Kicked the field goal: {}",
                        if r.kick_good {
                            "good"
                        } else if r.blocked {
                            "blocked"
                        } else {
                            "no good"
                        }
                    ),
                    f[2],
                ),
                PlayType::Kneel => ("Knelt".to_string(), f[0]),
                _ => (
                    format!("Went for it: {}", if r.first_down || r.touchdown { "converted" } else { "stopped" }),
                    f[0],
                ),
            };
            let opts = [("go", f[0]), ("punt", f[1]), ("kick", f[2])];
            let best =
                opts.iter().copied().filter(|x| x.1 > 0.0).max_by(|a, b| a.1.total_cmp(&b.1)).unwrap_or(("go", f[0]));
            let cost = (best.1 - chosen).max(0.0);
            let verdict = if cost < 0.01 {
                format!("Agreed: {} was best", best.0)
            } else {
                format!("Cost {:.1} points of win chance: {} was best", 100.0 * cost, best.0)
            };
            Some(json!({
                "n": r.n,
                "q": quarter_text(r.quarter, r.quarter > 4),
                "clock": clock_text(r.clock),
                "team": names[r.offense],
                "dd": format!("{} at {}", down_text(4, r.togo, r.yardline_100), spot(r.yardline_100)),
                "did": did,
                "go": f[0],
                "punt": f[1],
                "fg": if f[2] > 0.0 { json!(f[2]) } else { Value::Null },
                "convert": f[3],
                "cost": cost,
                "verdict": verdict,
            }))
        })
        .collect()
}

/// Halftime (topic 9): the first half by your staff and each
/// coordinator's adjustments, with reasons, as one click each.
pub fn halftime(r: &GameResult, mine: usize, names: &Names, sides: &[String; 2]) -> Value {
    let s = &r.stats;
    let them = 1 - mine;
    let first: Vec<&PlayRecord> = s.plays.iter().filter(|p| p.quarter <= 2 && scrimmage(p)).collect();
    let name = |id: u32| names.get(&id).map_or(format!("No. {id}"), |x| x.0.clone());
    let pos = |id: u32| names.get(&id).map_or(String::new(), |x| x.1.clone());
    let unit = |team: usize| {
        let p: Vec<&&PlayRecord> = first.iter().filter(|x| x.offense == team).collect();
        let runs: Vec<&&&PlayRecord> = p.iter().filter(|x| x.kind == PlayType::Run).collect();
        let drops: Vec<&&&PlayRecord> = p.iter().filter(|x| dropback(x)).collect();
        let e = |v: &[&&&PlayRecord]| v.iter().map(|x| x.epa).sum::<f64>() / v.len().max(1) as f64;
        json!({
            "plays": p.len(),
            "yards": p.iter().map(|x| x.yards).sum::<f64>().round(),
            "run": { "n": runs.len(), "ypc": runs.iter().map(|x| x.yards).sum::<f64>() / runs.len().max(1) as f64, "epa": e(&runs) },
            "pass": { "n": drops.len(), "epa": e(&drops), "sacks": drops.iter().filter(|x| x.kind == PlayType::Sack).count() },
            "success": 100.0 * p.iter().filter(|x| success(x)).count() as f64 / p.len().max(1) as f64,
        })
    };
    // What works: your best and worst calls by play, and what hurt you.
    let mut by_play: HashMap<String, (u32, f64)> = HashMap::new();
    for p in first.iter().filter(|x| x.offense == mine) {
        let name = p.call.splitn(4, ' ').last().unwrap_or("").to_string();
        let e = by_play.entry(name).or_default();
        e.0 += 1;
        e.1 += p.epa;
    }
    let mut bp: Vec<(String, (u32, f64))> = by_play.into_iter().filter(|x| x.1.0 >= 2).collect();
    bp.sort_by(|a, b| (b.1.1 / b.1.0 as f64).total_cmp(&(a.1.1 / a.1.0 as f64)));
    let works: Vec<String> = bp
        .iter()
        .take(2)
        .filter(|x| x.1.1 > 0.0)
        .map(|(p, (n, e))| format!("{p}: {n} calls, {:+.2} EPA a play", e / *n as f64))
        .collect();
    let fails: Vec<String> = bp
        .iter()
        .rev()
        .take(2)
        .filter(|x| x.1.1 < 0.0)
        .map(|(p, (n, e))| format!("{p}: {n} calls, {:+.2} EPA a play", e / *n as f64))
        .collect();
    // Coordinators' adjustments, each with its reason and the plan change.
    let mut adjust: Vec<Value> = Vec::new();
    let mut tgt: HashMap<u32, (u32, f64)> = HashMap::new();
    let mut cov: HashMap<u32, (u32, u32, f64)> = HashMap::new();
    for p in first.iter().filter(|x| x.kind == PlayType::Pass) {
        if p.offense == them
            && let Some(t) = p.target
        {
            let e = tgt.entry(t).or_default();
            e.0 += 1;
            e.1 += if p.complete { p.yards } else { 0.0 };
        }
        if p.offense == mine
            && let Some(d) = p.defender
        {
            let e = cov.entry(d).or_default();
            e.0 += 1;
            e.1 += p.complete as u32;
            e.2 += if p.complete { p.yards } else { 0.0 };
        }
    }
    let their_yds: f64 = tgt.values().map(|x| x.1).sum();
    if let Some((&id, &(n, y))) = tgt.iter().max_by(|a, b| a.1.1.total_cmp(&b.1.1))
        && y >= 50.0
        && y >= 0.4 * their_yds
    {
        adjust.push(json!({
                "who": "Your defensive coordinator",
                "text": format!("Bracket {} ({}): {n} target{}, {y:.0} of their {their_yds:.0} receiving yards.", name(id), pos(id), if n == 1 { "" } else { "s" }),
                "change": { "bracket": id },
            }));
    }
    if let Some((&id, &(n, c, y))) = cov.iter().filter(|x| pos(*x.0) == "CB").max_by(|a, b| a.1.2.total_cmp(&b.1.2))
        && c >= 3
        && y >= 40.0
    {
        adjust.push(json!({
            "who": "Your offensive coordinator",
            "text": format!("Go after {}: {c} catches on {n} throws at him for {y:.0} yards.", name(id)),
            "change": { "target_cb": id },
        }));
    }
    let qb_runs: Vec<&&PlayRecord> = first
        .iter()
        .filter(|x| {
            x.offense == them
                && (x.kind == PlayType::Scramble
                    || (x.kind == PlayType::Run && x.carrier.is_some() && x.carrier == x.passer))
        })
        .collect();
    let qb_yds: f64 = qb_runs.iter().map(|x| x.yards).sum();
    if qb_runs.len() >= 3 && qb_yds >= 25.0 {
        adjust.push(json!({
            "who": "Your defensive coordinator",
            "text": format!("Spy the quarterback: {} runs for {qb_yds:.0} yards in the half.", qb_runs.len()),
            "change": { "spy": true },
        }));
    }
    let big_ret = s
        .plays
        .iter()
        .filter(|x| x.quarter <= 2 && x.kind == PlayType::Kickoff && x.offense == mine && x.return_yards >= 35.0)
        .count();
    if big_ret >= 1 {
        adjust.push(json!({
            "who": "Your special teams coordinator",
            "text": format!("Kick away from their returner: {big_ret} return{} of 35 yards or more.", if big_ret == 1 { "" } else { "s" }),
            "change": { "kick_away": true },
        }));
    }
    let hurt: Vec<String> = r
        .injuries
        .iter()
        .filter(|(side, _)| *side == mine)
        .map(|(_, i)| {
            format!(
                "{} ({}): {}, about {:.0} week{}{}",
                name(i.id),
                pos(i.id),
                i.kind.name(),
                i.weeks,
                if i.weeks.round() == 1.0 { "" } else { "s" },
                if i.plays_on { ", playing through it" } else { ", out for the game" }
            )
        })
        .collect();
    json!({
        "score": s.score,
        "us": unit(mine),
        "them": unit(them),
        "works": works,
        "fails": fails,
        "adjust": adjust,
        "hurt": hurt,
        "names": sides,
    })
}

/// The final whistle (topic 11): the result, your calls against your
/// staff's, the plays that decided it, game balls, injuries and the
/// officials.
pub fn after_game(w: &World, r: &GameResult, mine: usize, names: &Names, sides: &[String; 2], m: &Wp) -> Value {
    let s = &r.stats;
    let name = |id: u32| names.get(&id).map_or(format!("No. {id}"), |x| x.0.clone());
    let hurt: Vec<String> = r
        .injuries
        .iter()
        .filter(|(side, _)| *side == mine)
        .map(|(_, i)| {
            format!(
                "{}: {}, about {:.0} week{}",
                name(i.id),
                i.kind.name(),
                i.weeks,
                if i.weeks.round() == 1.0 { "" } else { "s" }
            )
        })
        .collect();
    let won = s.score[mine] > s.score[1 - mine];
    let tied = s.score[0] == s.score[1];
    let _ = w;
    json!({
        "won": won,
        "tied": tied,
        "score": s.score,
        "calls": calls(s, mine),
        "key_plays": key_plays(s, names, sides, m, Some(mine), 6),
        "stars": stars(s, names, sides),
        "hurt": hurt,
        "officials": officiating(s, sides, names),
        "advanced": advanced(s, sides),
    })
}
