//! V10, the game flow board: the win chance snap by snap from the
//! engine's model (`game::win_chance`), the drives on the field from your
//! side, the plays that swung it, the final and the team numbers side by
//! side. The game page's chart, its key plays and the live game day screen
//! read the same model through `Wp`.

use afm_engine::calendar::{Date, Level as CalLevel};
use afm_engine::game::stats::{GameStats, PlayRecord, PlayType};
use afm_engine::game::win_chance::{self, Params};
use afm_engine::world::World;
use afm_engine::world::seat::SeatGame;
use serde_json::{Value, json};

use super::{clock, quarter, r1};
use crate::Reply;
use crate::lane_gameday::page;
use crate::views;

/// The win chance for one game: its level's numbers and what the home
/// side was expected to win by (the closing line where the books hung
/// one; else the home field alone).
#[derive(Clone, Copy, Debug)]
pub struct Wp {
    pub p: Params,
    pub home_spread: f64,
}

/// What a home field is worth without a line, in points (real: about 1.5
/// to 2 in today's NFL, about 2.5 in college; high school taken as
/// college's; proposed, not confirmed).
fn home_field(level: afm_engine::player::Level) -> f64 {
    match level {
        afm_engine::player::Level::NFL => 1.7,
        _ => 2.5,
    }
}

/// The home side's expected margin from the closing line, when there was
/// one (the line is the home spread, negative when favoured).
fn line_margin(w: &World, level: CalLevel, teams: [usize; 2], date: Date) -> Option<f64> {
    let [h, a] = teams;
    let line = match level {
        CalLevel::College => w
            .college_fixtures(Some(h))
            .into_iter()
            .find(|f| f.date == date && f.home == h && f.away == a)
            .and_then(|f| f.line),
        CalLevel::Nfl => w
            .nfl_fixtures(Some(h))
            .into_iter()
            .find(|f| f.when == Some(date) && f.home == h && f.away == a)
            .and_then(|f| f.line),
        _ => None,
    };
    line.map(|l| -l[0] as f64)
}

impl Wp {
    pub fn new(w: &World, level: CalLevel, teams: [usize; 2], date: Date) -> Wp {
        let lv = super::game_level(w, level, teams);
        let p = Params::for_level(lv);
        let home_spread = line_margin(w, level, teams, date).unwrap_or_else(|| home_field(lv));
        Wp { p, home_spread }
    }

    pub fn of_game(w: &World, g: &SeatGame) -> Wp {
        Wp::new(w, g.level, g.teams, g.date)
    }

    /// The home side's chance before a play.
    pub fn home(&self, r: &PlayRecord) -> f64 {
        win_chance::home_before(&self.p, r, self.home_spread)
    }

    /// Seconds of regulation played at a snap (overtime runs on past it).
    pub fn elapsed(&self, r: &PlayRecord) -> f64 {
        let q = self.p.quarter;
        if r.quarter > 4 {
            4.0 * q + (r.quarter - 5) as f64 * 600.0 + (600.0 - r.clock.min(600.0))
        } else {
            (r.quarter.max(1) - 1) as f64 * q + (q - r.clock).max(0.0)
        }
    }
}

fn scrimmage(r: &PlayRecord) -> bool {
    matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble)
}

/// The board for one of your games.
pub fn board(w: &World, k: usize) -> Reply {
    let g = w.seat_games().get(k).ok_or("That game is not kept.")?;
    let s = &g.stats;
    let m = Wp::of_game(w, g);
    let mine = w.seat_team().and_then(|(_, t)| g.teams.iter().position(|&x| x == t));
    // The side the board is told from: yours, else the home side.
    let us = mine.unwrap_or(0);
    let them = 1 - us;
    let refs = [super::team_ref(g.level, g.teams[0]), super::team_ref(g.level, g.teams[1])];
    let names = super::game_names(w, g, &refs);
    let name = |id: u32| names.get(&id).map_or(format!("No. {id}"), |x| x.0.clone());
    let plays: Vec<&PlayRecord> = s.plays.iter().filter(|r| r.offense < 2).collect();
    // Our chance before each play, then the final.
    let ours = |r: &PlayRecord| {
        let h = m.home(r);
        if us == 0 { h } else { 1.0 - h }
    };
    let fin = match s.score[us].cmp(&s.score[them]) {
        std::cmp::Ordering::Greater => 1.0,
        std::cmp::Ordering::Less => 0.0,
        _ => 0.5,
    };
    let mut series: Vec<(u32, f64, f64, &PlayRecord)> =
        plays.iter().map(|r| (r.n, m.elapsed(r), ours(r), *r)).collect();
    let end_t = series.last().map_or(4.0 * m.p.quarter, |x| x.1.max(4.0 * m.p.quarter));
    // Each play's swing: our chance after it (the next snap's) less ours
    // before it.
    let mut swings: Vec<(f64, usize)> = Vec::new();
    for i in 0..series.len() {
        let after = series.get(i + 1).map_or(fin, |x| x.2);
        let d = after - series[i].2;
        if !series[i].3.nullified {
            swings.push((d, i));
        }
    }
    swings.sort_by(|a, b| b.0.abs().total_cmp(&a.0.abs()));
    let mut picked: Vec<usize> = swings.iter().filter(|x| x.0.abs() >= 0.04).take(5).map(|x| x.1).collect();
    picked.sort();
    let letters = ["A", "B", "C", "D", "E"];
    let dr = kneels_as_one(page::drives_json(s, &g.names));
    let drive_of = |n: u32| {
        dr.iter().position(|(_, d)| {
            d["first"].as_u64().is_some_and(|f| f as u32 <= n) && d["last"].as_u64().is_some_and(|l| l as u32 >= n)
        })
    };
    let swing_rows: Vec<Value> = picked
        .iter()
        .enumerate()
        .map(|(j, &i)| {
            let (n, t, before, r) = series[i];
            let after = series.get(i + 1).map_or(fin, |x| x.2);
            json!({
                "letter": letters[j],
                "n": n,
                "t": r1(t),
                "before": r1(100.0 * before),
                "after": r1(100.0 * after),
                "delta": (100.0 * (after - before)).round(),
                "ours": r.offense == us,
                "q": quarter(r.quarter),
                "clock": clock(r.clock),
                "text": views::play_text(r, &name),
                "drive": drive_of(n),
                "who": page::who(r, &names).into_iter().map(|(id, n)| json!({ "id": id, "name": n })).collect::<Vec<_>>(),
            })
        })
        .collect();
    series.push((plays.last().map_or(0, |r| r.n + 1), end_t, fin, plays.last().copied().unwrap_or(&s.plays[0])));
    // The low and high points of ours, with the time left then.
    let low = series.iter().min_by(|a, b| a.2.total_cmp(&b.2));
    let high = series.iter().max_by(|a, b| a.2.total_cmp(&b.2));
    let point = |x: Option<&(u32, f64, f64, &PlayRecord)>| {
        x.map(|&(n, t, p, r)| {
            json!({ "n": n, "t": r1(t), "p": (100.0 * p).round(), "q": quarter(r.quarter), "qn": r.quarter, "clock": clock(r.clock) })
        })
    };
    let drives: Vec<Value> = dr
        .iter()
        .enumerate()
        .map(|(j, (i, d))| {
            let team = d["team"].as_u64().unwrap_or(0) as usize;
            let (sx, ex) = (d["start_x"].as_f64().unwrap_or(0.0), d["end_x"].as_f64().unwrap_or(0.0));
            // Our goal on the left: our drives run right, theirs left.
            let (a, b) = if team == us { (sx, ex) } else { (100.0 - sx, 100.0 - ex) };
            let result = d["result"].as_str().unwrap_or("");
            let letter = swing_rows.iter().find(|x| x["drive"].as_u64() == Some(j as u64)).map(|x| x["letter"].clone());
            json!({
                "i": i,
                "ours": team == us,
                "q": d["q"], "clock": d["clock"],
                "from": r1(a), "to": r1(b),
                "result": result,
                "bad": matches!(result, "Interception" | "Fumble" | "Pick six" | "Returned for a TD" | "Safety" | "Downs" | "Missed FG" | "Blocked FG"),
                "points": d["points"],
                "plays": d["plays"], "yards": d["yards"], "time": d["time"],
                "first": d["first"], "last": d["last"],
                "swing": letter,
            })
        })
        .collect();
    let wp: Vec<Value> = series.iter().map(|&(n, t, p, _)| json!([n, r1(t), (1000.0 * p).round() / 1000.0])).collect();
    Ok(json!({
        "index": k,
        "date": views::date_text(g.date),
        "level": super::game_level(w, g.level, g.teams).name(),
        "teams": [views::team_brief(w, refs[0]), views::team_brief(w, refs[1])],
        "names": g.names,
        "us": us,
        "mine": mine,
        "score": s.score,
        "quarters": s.quarters,
        "overtime": s.overtime,
        "length": 4.0 * m.p.quarter,
        "end": r1(end_t),
        "qlen": m.p.quarter,
        "spread": r1(if us == 0 { m.home_spread } else { -m.home_spread }),
        "wp": wp,
        "low": point(low),
        "high": point(high),
        "swings": swing_rows,
        "drives": drives,
        "final": final_card(w, g, us, &name),
        "stats": team_stats(s, us),
    }))
}

/// The drive list with each drive's place in the game page's own list.
/// That list starts a new drive at each kneel of a kneel-out; the board
/// shows the kneel-out as the one drive it was.
fn kneels_as_one(dr: Vec<Value>) -> Vec<(usize, Value)> {
    let secs = |v: &Value| {
        let t = v.as_str().unwrap_or("0:00");
        let (m, s) = t.split_once(':').unwrap_or(("0", t));
        m.parse::<f64>().unwrap_or(0.0) * 60.0 + s.parse::<f64>().unwrap_or(0.0)
    };
    let mut out: Vec<(usize, Value)> = Vec::new();
    for (i, d) in dr.into_iter().enumerate() {
        if let Some((_, p)) = out.last_mut()
            && p["team"] == d["team"]
            && p["result"] == "End of game"
            && d["result"] == "End of game"
            && p["q"] == d["q"]
        {
            p["last"] = d["last"].clone();
            p["end_x"] = d["end_x"].clone();
            p["plays"] = json!(p["plays"].as_u64().unwrap_or(0) + d["plays"].as_u64().unwrap_or(0));
            p["yards"] = json!((p["end_x"].as_f64().unwrap_or(0.0) - p["start_x"].as_f64().unwrap_or(0.0)).round());
            // From its first snap to the last kneel's end, the run-off too.
            let run = secs(&p["clock"]) - secs(&d["clock"]) + secs(&d["time"]);
            p["time"] = json!(clock(run.min(secs(&p["clock"]))));
            continue;
        }
        out.push((i, d));
    }
    out
}

/// How it ended: the score that decided it, when, and the record after.
fn final_card(w: &World, g: &SeatGame, us: usize, name: &dyn Fn(u32) -> String) -> Value {
    let s = &g.stats;
    let them = 1 - us;
    let won = s.score[us] > s.score[them];
    let tied = s.score[us] == s.score[them];
    // The deciding score: the first score by the winner after the last
    // moment it was level or behind.
    let mut decider: Option<&PlayRecord> = None;
    if !tied {
        let winner = if won { us } else { them };
        let live: Vec<&PlayRecord> = s.plays.iter().filter(|r| r.offense < 2 && !r.nullified).collect();
        let last_level = live.iter().rposition(|r| r.score[winner] - r.score[1 - winner] <= 0);
        decider = last_level.and_then(|i| {
            live[i..].iter().copied().find(|r| {
                (r.kind == PlayType::FieldGoal && r.kick_good || r.touchdown || r.safety || r.single)
                    && scorer(r) == Some(winner)
            })
        });
    }
    let how = decider.map(|r| {
        let what = if r.kind == PlayType::FieldGoal {
            format!("a {}-yard kick", r.kick_distance.round())
        } else if r.safety {
            "a safety".into()
        } else if r.defense_td {
            "a defensive touchdown".into()
        } else if r.kind == PlayType::Run || r.kind == PlayType::Scramble {
            format!("{}'s {}-yard run", name(r.carrier.unwrap_or(0)), r.yards.round())
        } else if r.kind == PlayType::Pass {
            format!("a {}-yard throw to {}", r.yards.round(), name(r.target.unwrap_or(0)))
        } else {
            "a touchdown".into()
        };
        let left = if r.quarter > 4 {
            "in overtime".to_string()
        } else if r.quarter == 4 {
            format!("with {} left", clock(r.clock))
        } else {
            format!("in the {}", ["first", "second", "third"][(r.quarter.max(1) - 1) as usize])
        };
        json!({ "what": what, "when": left, "n": r.n })
    });
    let record = match g.level {
        CalLevel::Nfl => {
            let (wn, l, t) = w.nfl_record(g.teams[us]);
            if t > 0 { format!("{wn}-{l}-{t}") } else { format!("{wn}-{l}") }
        }
        CalLevel::College => {
            let r = w.college_record(g.teams[us]);
            format!("{}-{}", r.wins, r.losses)
        }
        _ => String::new(),
    };
    json!({
        "won": won, "tied": tied,
        "word": if tied { "Tied" } else if won { "Won" } else { "Lost" },
        "score": [s.score[us], s.score[them]],
        "how": how,
        "record": record,
        "quarters": s.quarters.iter().map(|q| [q[us], q[them]]).collect::<Vec<_>>(),
    })
}

/// The side that scored on a play.
fn scorer(r: &PlayRecord) -> Option<usize> {
    let o = r.offense.min(1);
    if r.safety || r.defense_td {
        Some(1 - o)
    } else if r.touchdown || r.kick_good || r.single {
        Some(o)
    } else {
        None
    }
}

/// The team numbers, ours then theirs, with our share of each for the
/// split bar (a lower-is-better number gives the bar the other way).
fn team_stats(s: &GameStats, us: usize) -> Vec<Value> {
    let them = 1 - us;
    let top = views::possession(s);
    let side = |k: usize| {
        let own: Vec<&PlayRecord> = s.plays.iter().filter(|r| r.offense == k && !r.nullified && scrimmage(r)).collect();
        let yards: f64 = own.iter().map(|r| r.yards).sum();
        let explosive = own
            .iter()
            .filter(|r| (r.kind == PlayType::Run && r.yards >= 10.0) || (r.kind == PlayType::Pass && r.yards >= 20.0))
            .count();
        let sacks = s
            .plays
            .iter()
            .filter(|r| r.kind == PlayType::Sack && r.offense != k && r.offense < 2 && !r.nullified)
            .count();
        let t = &s.team[k];
        (yards, own.len(), t.third_conv, t.third_att, explosive, t.turnovers, top[k], sacks)
    };
    let (a, b) = (side(us), side(them));
    let share = |x: f64, y: f64| if x + y > 0.0 { r1(100.0 * x / (x + y)) } else { 50.0 };
    let ypp = |s: &(f64, usize, u32, u32, usize, u32, f64, usize)| if s.1 > 0 { s.0 / s.1 as f64 } else { 0.0 };
    let rate = |c: u32, n: u32| if n > 0 { c as f64 / n as f64 } else { 0.0 };
    vec![
        json!({ "k": "Total yards", "a": a.0.round(), "b": b.0.round(), "share": share(a.0.max(0.0), b.0.max(0.0)) }),
        json!({ "k": "Yards a play", "a": r1(ypp(&a)), "b": r1(ypp(&b)), "share": share(ypp(&a).max(0.0), ypp(&b).max(0.0)) }),
        json!({ "k": "Third downs", "a": format!("{}/{}", a.2, a.3), "b": format!("{}/{}", b.2, b.3), "share": share(rate(a.2, a.3), rate(b.2, b.3)) }),
        json!({ "k": "Explosive plays", "a": a.4, "b": b.4, "share": share(a.4 as f64, b.4 as f64) }),
        // Fewer is better: the bar leans to the side that gave it away less.
        json!({ "k": "Turnovers", "a": a.5, "b": b.5, "share": share(b.5 as f64, a.5 as f64), "low": true }),
        json!({ "k": "Possession", "a": clock(a.6), "b": clock(b.6), "share": share(a.6, b.6) }),
        json!({ "k": "Sacks", "a": a.7, "b": b.7, "share": share(a.7 as f64, b.7 as f64) }),
    ]
}
