//! V10, the game flow board: the win chance snap by snap from the
//! engine's model (`game::win_chance`), the drives on the field from your
//! side, the plays that swung it, the final and the team numbers side by
//! side. The game page's chart, its key plays and the live game day screen
//! read the same model through `Wp`.

use afm_engine::calendar::{Date, Level as CalLevel};
use afm_engine::game::stats::{GameStats, PlayRecord, PlayType};
use afm_engine::game::win_chance::{self, Params};
use afm_engine::world::World;
use afm_engine::world::season_page::{self, Held};
use afm_engine::world::seat::SeatGame;
use serde_json::{Value, json};

use super::{clock, quarter, r1};
use crate::Reply;
use crate::lane_gameday::page;
use crate::views;

/// The win chance for one game: its level's numbers and what the home
/// side was expected to win by (the closing line where the books hung
/// one; else the home field alone, nothing at a neutral site).
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

/// The game in the home side's season page (this season's or last), for
/// its line and site.
fn page_game(w: &World, level: CalLevel, teams: [usize; 2], date: Date) -> Option<season_page::PageGame> {
    [Held::This, Held::Last].into_iter().find_map(|held| {
        season_page::page(w, level, teams[0], held)?
            .games
            .into_iter()
            .find(|g| g.date == date && g.opp == teams[1] && g.home)
    })
}

impl Wp {
    /// A game on a date: the line from the home side's season page, else
    /// the home field (nothing at a neutral site).
    pub fn new(w: &World, level: CalLevel, teams: [usize; 2], date: Date, neutral: bool) -> Wp {
        let lv = super::game_level(w, level, teams);
        let p = Params::for_level(lv);
        let g = page_game(w, level, teams, date);
        let neutral = neutral || g.as_ref().is_some_and(|g| g.neutral);
        // The page's line is the home side's spread (negative when favoured).
        let home_spread =
            g.and_then(|g| g.line).map(|l| -l[0] as f64).unwrap_or_else(|| if neutral { 0.0 } else { home_field(lv) });
        Wp { p, home_spread }
    }

    /// A kept game: the line stored with it, else as `new` reads it (a
    /// game kept before the lines were).
    pub fn of_game(w: &World, g: &SeatGame) -> Wp {
        match g.spread {
            Some(x) => Wp { p: Params::for_level(super::game_level(w, g.level, g.teams)), home_spread: x as f64 },
            None => Wp::new(w, g.level, g.teams, g.date, false),
        }
    }

    /// The home side's chance before each play of the log (one for each
    /// record), following college and high school overtime's rounds.
    pub fn series(&self, plays: &[PlayRecord]) -> Vec<f64> {
        win_chance::home_series(&self.p, plays, self.home_spread)
    }

    /// Each record's place on the chart's time line, in seconds from the
    /// kick, with where each overtime period or round starts. Regulation
    /// runs on its clock. A timed overtime (the NFL's) follows on its own
    /// clock, a new period when the clock starts again; college and high
    /// school overtime has no clock, so each snap takes a fixed width and
    /// each round is marked.
    pub fn times(&self, plays: &[PlayRecord]) -> (Vec<f64>, Vec<(f64, String)>) {
        let q = self.p.quarter;
        let reg = 4.0 * q;
        let untimed = |r: &PlayRecord| r.clock >= UNTIMED;
        let len = if plays.iter().any(|r| r.quarter > 4 && !untimed(r) && r.clock > 600.5) { 900.0 } else { 600.0 };
        let label = |k: usize| if k <= 1 { "OT".to_string() } else { format!("{k}OT") };
        let mut out = Vec::with_capacity(plays.len());
        let mut marks: Vec<(f64, String)> = Vec::new();
        let (mut period, mut prev, mut snaps) = (0usize, f64::MAX, 0usize);
        let rounds = win_chance::ot_rounds(plays);
        for (r, &round) in plays.iter().zip(&rounds) {
            if r.quarter <= 4 {
                out.push((r.quarter.max(1) - 1) as f64 * q + (q - r.clock).clamp(0.0, q));
                continue;
            }
            if !untimed(r) {
                if period == 0 || r.clock > prev + 1.0 {
                    period += 1;
                    marks.push((reg + (period - 1) as f64 * len, label(period)));
                }
                prev = r.clock;
                out.push(reg + (period - 1) as f64 * len + (len - r.clock.min(len)));
                continue;
            }
            let x = reg + snaps as f64 * OT_SNAP;
            if round as usize > marks.len() {
                marks.push((x, label(round as usize)));
            }
            out.push(x);
            snaps += 1;
        }
        (out, marks)
    }
}

/// A clock this high is college or high school overtime's: no clock.
const UNTIMED: f64 = 1.0e6;

/// The chart's width for a snap of untimed overtime, in seconds.
const OT_SNAP: f64 = 20.0;

fn scrimmage(r: &PlayRecord) -> bool {
    matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble)
}

/// "an 18-yard", "an 8-yard", "an 11-yard", else "a".
fn a_an(yards: f64) -> &'static str {
    let n = yards.round() as i64;
    if n == 8 || n == 11 || n == 18 || (80..90).contains(&n) { "an" } else { "a" }
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
    // Our chance before each play, then the final.
    let home = m.series(&s.plays);
    let (xs, ot_marks) = m.times(&s.plays);
    let idx: Vec<usize> = (0..s.plays.len()).filter(|&i| s.plays[i].offense < 2).collect();
    let plays: Vec<&PlayRecord> = idx.iter().map(|&i| &s.plays[i]).collect();
    let ours = |i: usize| if us == 0 { home[i] } else { 1.0 - home[i] };
    let fin = match s.score[us].cmp(&s.score[them]) {
        std::cmp::Ordering::Greater => 1.0,
        std::cmp::Ordering::Less => 0.0,
        _ => 0.5,
    };
    let mut series: Vec<(u32, f64, f64, &PlayRecord)> =
        idx.iter().map(|&i| (s.plays[i].n, xs[i], ours(i), &s.plays[i])).collect();
    let end_t = series.last().map_or(4.0 * m.p.quarter, |x| {
        let ot = if x.3.quarter > 4 && x.3.clock >= UNTIMED { OT_SNAP } else { 0.0 };
        (x.1 + ot).max(4.0 * m.p.quarter)
    });
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
    // The low and high points of ours before the final whistle, with the
    // time left then.
    let low = series.iter().min_by(|a, b| a.2.total_cmp(&b.2)).copied();
    let high = series.iter().max_by(|a, b| a.2.total_cmp(&b.2)).copied();
    series.push((plays.last().map_or(0, |r| r.n + 1), end_t, fin, plays.last().copied().unwrap_or(&s.plays[0])));
    let point = |x: Option<(u32, f64, f64, &PlayRecord)>| {
        x.map(|(n, t, p, r)| {
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
        "ot": ot_marks.iter().map(|(t, l)| json!([r1(*t), l])).collect::<Vec<_>>(),
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
    let mut with_try: Option<&str> = None;
    if !tied {
        let winner = if won { us } else { them };
        let live: Vec<&PlayRecord> = s.plays.iter().filter(|r| r.offense < 2 && !r.nullified).collect();
        let mut last_level = live.iter().rposition(|r| r.score[winner] - r.score[1 - winner] <= 0);
        // Level before the try: the touchdown before it decided it, with
        // the try.
        if let Some(i) = last_level
            && matches!(live[i].kind, PlayType::ExtraPoint | PlayType::TwoPoint)
            && let Some(j) = live[..i].iter().rposition(|r| r.touchdown)
        {
            with_try = Some(if live[i].kind == PlayType::TwoPoint { " and the two" } else { " and the extra point" });
            last_level = Some(j);
        }
        decider = last_level.and_then(|i| {
            live[i..].iter().copied().find(|r| {
                (r.kind == PlayType::FieldGoal && r.kick_good || r.touchdown || r.safety || r.single)
                    && scorer(r) == Some(winner)
            })
        });
    }
    let how = decider.map(|r| {
        let what = if r.kind == PlayType::FieldGoal {
            format!("{} {}-yard kick", a_an(r.kick_distance), r.kick_distance.round())
        } else if r.safety {
            "a safety".into()
        } else if r.defense_td {
            "a defensive touchdown".into()
        } else if matches!(r.kind, PlayType::Kickoff | PlayType::Punt) && r.touchdown && !r.turnover {
            let what = if r.kind == PlayType::Kickoff { "kickoff" } else { "punt" };
            match r.returner {
                Some(id) => format!("{}'s {what} return", name(id)),
                None => format!("a {what} return"),
            }
        } else if r.kind == PlayType::Run || r.kind == PlayType::Scramble {
            format!("{}'s {}-yard run", name(r.carrier.unwrap_or(0)), r.yards.round())
        } else if r.kind == PlayType::Pass {
            format!("{} {}-yard throw to {}", a_an(r.yards), r.yards.round(), name(r.target.unwrap_or(0)))
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
        json!({ "what": format!("{what}{}", with_try.unwrap_or("")), "when": left, "n": r.n })
    });
    let record = record_after(w, g, us);
    json!({
        "won": won, "tied": tied,
        "word": if tied { "Tied" } else if won { "Won" } else { "Lost" },
        "score": [s.score[us], s.score[them]],
        "how": how,
        "record": record,
        "quarters": s.quarters.iter().map(|q| [q[us], q[them]]).collect::<Vec<_>>(),
    })
}

/// The record after a game: its season's games up to it, from the season
/// page that holds it (none once the world no longer holds that season).
fn record_after(w: &World, g: &SeatGame, us: usize) -> String {
    let (team, opp) = (g.teams[us], g.teams[1 - us]);
    [Held::This, Held::Last]
        .into_iter()
        .find_map(|held| {
            let p = season_page::page(w, g.level, team, held)?;
            p.games.iter().any(|x| x.date == g.date && x.opp == opp).then(|| {
                let (mut wn, mut l, mut t) = (0, 0, 0);
                for x in p.games.iter().filter(|x| x.date <= g.date && !x.cancelled) {
                    match (x.score, x.won) {
                        (Some([a, b]), _) if a > b => wn += 1,
                        (Some([a, b]), _) if a < b => l += 1,
                        (Some(_), _) => t += 1,
                        (None, Some(true)) => wn += 1,
                        (None, Some(false)) => l += 1,
                        _ => {}
                    }
                }
                if t > 0 { format!("{wn}-{l}-{t}") } else { format!("{wn}-{l}") }
            })
        })
        .unwrap_or_default()
}

/// The side that scored on a play.
fn scorer(r: &PlayRecord) -> Option<usize> {
    let o = r.offense.min(1);
    if r.safety || r.defense_td {
        Some(1 - o)
    } else if matches!(r.kind, PlayType::Kickoff | PlayType::Punt) && r.touchdown && !r.turnover {
        // A return: the kicking side is the offense on the record.
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
