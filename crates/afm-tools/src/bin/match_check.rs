//! Development check (section 3, topic 10): play many games between
//! synthetic teams at a level and compare the averages to real football.
//! Usage: match_check [nfl|fbs|hs] [games] [seed] [pbp]
//! With `pbp`, prints the first game's play-by-play with its whys.

extern crate afm_tools;
use afm_engine::game::stats::PlayType;
use afm_engine::game::synthetic::synthetic_league;
use afm_engine::game::{Conditions, Venue, quick_game_in};
use afm_engine::player::Level;
use rayon::prelude::*;
use serde_json::Value;

/// The engine's routes pooled with the real route types they match.
const ROUTE_GROUPS: [(&str, &[&str]); 12] = [
    ("go / fade / seam", &["GO"]),
    ("post", &["POST"]),
    ("corner", &["CORNER"]),
    ("wheel", &["WHEEL"]),
    ("dig / in / cross", &["IN/DIG", "IN", "CROSS"]),
    ("out (10+ air)", &["DEEP OUT"]),
    ("out (under 10)", &["OUT", "QUICK OUT"]),
    ("slant", &["SLANT"]),
    ("hitch / curl / sit", &["HITCH", "HITCH/CURL"]),
    ("drag", &["SHALLOW CROSS/DRAG"]),
    ("flat / swing", &["FLAT", "SWING"]),
    ("screen", &["SCREEN"]),
];

fn route_group(route: &str, air: f64) -> Option<usize> {
    Some(match route {
        "go" | "fade" | "seam" => 0,
        "post" => 1,
        "corner" => 2,
        "wheel" => 3,
        "dig" => 4,
        "out" | "comeback" if air >= 10.0 => 5,
        "out" | "comeback" => 6,
        "slant" => 7,
        "hitch" | "curl" | "sit" => 8,
        "drag" => 9,
        "flat" | "swing" | "check-release" => 10,
        "screen" | "bubble" => 11,
        _ => return None,
    })
}

/// The teams of game `g`: each matchup is played twice, once at each end,
/// so team strength cancels out of the home margin.
fn pair_of(g: usize) -> (usize, usize) {
    let k = g / 2;
    let h = (k * 7) % 32;
    let a = (k * 13 + 5) % 32;
    let a = if a == h { (a + 1) % 32 } else { a };
    if g.is_multiple_of(2) { (h, a) } else { (a, h) }
}

/// AFM_ROSTERS=file.json (from `tools/rosters/export_rosters.py`): a world
/// trial's own rosters instead of synthetic teams, playing conference
/// games, home and away. Returns the teams and the schedule.
fn world_rosters(path: &str, level: Level, seed: u32) -> (Vec<afm_engine::game::GameTeam>, Vec<(usize, usize)>) {
    use afm_engine::attributes::{Attr, Attributes};
    use afm_engine::game::OffScheme;
    use afm_engine::game::synthetic::{RatedPlayer, team_from_ratings};
    use afm_engine::positions::Position;
    use afm_engine::tendencies::{TEND_COUNT, Tend};
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("rosters file")).expect("rosters json");
    let schemes =
        [OffScheme::SpreadOption, OffScheme::AirRaid, OffScheme::WestCoast, OffScheme::PowerGap, OffScheme::WideZone];
    let mut teams = Vec::new();
    let mut conf = Vec::new();
    for (k, t) in v["teams"].as_array().unwrap().iter().enumerate() {
        let list: Vec<RatedPlayer> = t["players"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|p| {
                let pos = Position::ALL.into_iter().find(|x| format!("{x:?}") == p["pos"].as_str().unwrap_or(""))?;
                let mut attrs = Attributes::default();
                for a in Attr::ALL {
                    attrs.set(a, p["attrs"][format!("{a:?}")].as_f64().unwrap_or(40.0));
                }
                let mut tend = [10.5; TEND_COUNT];
                for (i, td) in Tend::ALL.into_iter().enumerate() {
                    tend[i] = p["tend"][td.key()].as_f64().unwrap_or(10.5);
                }
                Some(RatedPlayer { position: pos, attrs, tend })
            })
            .collect();
        let s = seed.wrapping_add(k as u32 * 7919);
        teams.push(team_from_ratings(t["name"].as_str().unwrap_or("?"), level, &list, schemes[k % schemes.len()], s));
        conf.push(t["conf"].as_str().unwrap_or("").to_string());
    }
    let mut sched = Vec::new();
    for h in 0..teams.len() {
        for a in 0..teams.len() {
            if h != a && conf[h] == conf[a] && !conf[h].is_empty() && !conf[h].starts_with("Indep") {
                sched.push((h, a));
            }
        }
    }
    // Interleave so a short run still spreads over every conference.
    let mut r = afm_engine::rng::Rng::new(seed ^ 0x5EED);
    for i in (1..sched.len()).rev() {
        let j = (r.next() * (i + 1) as f64) as usize;
        sched.swap(i, j.min(i));
    }
    (teams, sched)
}

fn main() {
    // AFM_QBSTATS=1: how the quarterbacks chose their throws (debug).
    let qb_stats = std::env::var("AFM_QBSTATS").is_ok();
    let args: Vec<String> = std::env::args().collect();
    let level = match args.get(1).map(|s| s.as_str()) {
        Some("fbs") => Level::FBS,
        Some("hs") => Level::HS,
        _ => Level::NFL,
    };
    let games: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(200);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
    let pbp = args.get(4).is_some_and(|s| s == "pbp");
    let (mut teams, sched) = match std::env::var("AFM_ROSTERS") {
        Ok(path) => world_rosters(&path, level, seed),
        Err(_) => (synthetic_league(level, 32, seed), Vec::new()),
    };
    let pair = |g: usize| if sched.is_empty() { pair_of(g) } else { sched[g % sched.len()] };
    // AFM_DUMP=path writes the teams as a rosters file (the AFM_ROSTERS
    // format) and stops: with tools/rosters/export_rosters.py output it lets
    // a check swap one position group between test and world rosters.
    if let Ok(out) = std::env::var("AFM_DUMP") {
        use afm_engine::attributes::Attr;
        use afm_engine::tendencies::Tend;
        let ts: Vec<Value> = teams
            .iter()
            .map(|t| {
                let ps: Vec<Value> = t
                    .players
                    .iter()
                    .map(|p| {
                        let attrs: serde_json::Map<String, Value> =
                            Attr::ALL.into_iter().map(|a| (format!("{a:?}"), p.attrs.get(a).into())).collect();
                        let tend: serde_json::Map<String, Value> =
                            Tend::ALL.into_iter().map(|td| (td.key().to_string(), p.tend.get(td).into())).collect();
                        serde_json::json!({"pos": format!("{:?}", p.position), "attrs": attrs, "tend": tend})
                    })
                    .collect();
                serde_json::json!({"name": t.name, "conf": "", "players": ps})
            })
            .collect();
        std::fs::write(&out, serde_json::json!({"teams": ts}).to_string()).expect("write the rosters file");
        return;
    }
    // AFM_STAR=RB:15 makes each team's best player at a position that many
    // points better at everything, to see how a star shares the load.
    // A position ending in * (CB*:5) moves every player there.
    // Several at once: AFM_STAR="CB*:-4.7;LB*:5.1" (each spec as above).
    let stars: Vec<String> =
        std::env::var("AFM_STAR").map(|v| v.split(';').map(String::from).collect()).unwrap_or_default();
    for v in stars.iter().filter(|v| v.contains("*:")) {
        let Some((name, add)) = v.split_once(':') else { continue };
        let (Ok(add), Some(name)) = (add.parse::<f64>(), name.strip_suffix('*')) else { continue };
        for name in name.split(',') {
            let pos = afm_engine::positions::Position::ALL.into_iter().find(|p| format!("{p:?}") == name);
            for t in teams.iter_mut() {
                for p in t.players.iter_mut().filter(|p| Some(p.position) == pos) {
                    for a in afm_engine::attributes::Attr::ALL {
                        p.attrs.set(a, (p.attrs.get(a) + add).clamp(1.0, 99.0));
                    }
                    for r in p.spot_r.iter_mut() {
                        *r += add;
                    }
                }
            }
        }
    }
    if let Ok(v) = std::env::var("AFM_STAR")
        && !v.contains('*')
        && let Some((name, add)) = v.split_once(':')
        && let Ok(add) = add.parse::<f64>()
    {
        let pos = afm_engine::positions::Position::ALL.into_iter().find(|p| format!("{p:?}") == name);
        for t in teams.iter_mut() {
            let best = (0..t.players.len()).filter(|&i| Some(t.players[i].position) == pos).max_by(|&a, &b| {
                t.players[a].overall_at(pos.unwrap()).total_cmp(&t.players[b].overall_at(pos.unwrap()))
            });
            if let Some(i) = best {
                let p = &mut t.players[i];
                for a in afm_engine::attributes::Attr::ALL {
                    p.attrs.set(a, (p.attrs.get(a) + add).min(99.0));
                }
                for r in p.spot_r.iter_mut() {
                    *r += add;
                }
            }
        }
    }
    // AFM_TRACE=1 prints every agent's state (use with one game).
    if std::env::var("AFM_TRACE").is_ok() {
        afm_engine::game::sim::TRACE.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    let t0 = std::time::Instant::now();
    let results: Vec<_> = (0..games)
        .into_par_iter()
        .map(|g| {
            let (h, a) = pair(g);
            // An average road trip: the crowd of an average FBS conference
            // or NFL game (a 50,000-70,000 stadium) and about 500 miles and
            // half a time zone of travel for the visitors.
            let venue = Venue {
                cond: Conditions { crowd: 0.65, ..Conditions::default() },
                wear: [0.0, 0.012],
                ..Venue::default()
            };
            quick_game_in(&teams[h], &teams[a], level, seed as u64 * 1_000_003 + g as u64, Default::default(), venue)
        })
        .collect();
    let secs = t0.elapsed().as_secs_f64();
    // Snap shares (topic: depth and rotation): starters' share of their
    // side's snaps by position, and the lead back's share of the backs'
    // carries, against real bands (proposed, not confirmed; see
    // data/real-world/README.md).
    {
        use afm_engine::positions::Position as P;
        let groups: [(P, usize, &str); 11] = [
            (P::QB, 1, "0.97-1.00"),
            (P::RB, 1, "0.55-0.70"),
            (P::WR, 3, "0.75-0.90"),
            (P::TE, 1, "0.70-0.85"),
            (P::OT, 2, "0.95-1.00"),
            (P::IOL, 3, "0.95-1.00"),
            (P::EDGE, 2, "0.65-0.85"),
            (P::DT, 2, "0.55-0.75"),
            (P::LB, 2, "0.80-0.98"),
            (P::CB, 2, "0.90-1.00"),
            (P::S, 2, "0.90-1.00"),
        ];
        let mut share = vec![0.0f64; groups.len()];
        let mut carry = [0.0f64; 2];
        let mut nt = 0.0;
        // Sacks and tackles for loss by player over all a team's games.
        let mut rush: Vec<std::collections::HashMap<u32, (f64, f64)>> = vec![Default::default(); teams.len()];
        for (g, r) in results.iter().enumerate() {
            let (h, a) = pair(g);
            for (k, ti) in [h, a].into_iter().enumerate() {
                let lines = &r.stats.players[k];
                let team = &teams[ti];
                for (id, l) in lines.iter() {
                    let e = rush[ti].entry(*id).or_default();
                    e.0 += l.sacks;
                    e.1 += l.tfl;
                }
                let off = lines.values().map(|l| l.snaps_off).max().unwrap_or(1).max(1) as f64;
                let def = lines.values().map(|l| l.snaps_def).max().unwrap_or(1).max(1) as f64;
                nt += 1.0;
                for (gi, &(pos, n, _)) in groups.iter().enumerate() {
                    let defense = matches!(pos, P::EDGE | P::DT | P::LB | P::CB | P::S);
                    let mut v: Vec<f64> = team
                        .players
                        .iter()
                        .filter(|p| p.position == pos)
                        .filter_map(|p| lines.get(&p.id))
                        .map(|l| if defense { l.snaps_def } else { l.snaps_off } as f64)
                        .collect();
                    v.sort_by(|x, y| y.total_cmp(x));
                    let top: f64 = v.iter().take(n).sum();
                    share[gi] += top / (n as f64 * if defense { def } else { off });
                }
                let mut c: Vec<f64> = team
                    .players
                    .iter()
                    .filter(|p| p.position == P::RB)
                    .filter_map(|p| lines.get(&p.id))
                    .map(|l| l.rush as f64)
                    .collect();
                c.sort_by(|x, y| y.total_cmp(x));
                carry[0] += c.first().copied().unwrap_or(0.0);
                carry[1] += c.iter().sum::<f64>();
            }
        }
        if std::env::var("AFM_SNAPS").is_ok() {
            println!("starters' share of snaps (real band)");
            for (gi, &(pos, n, real)) in groups.iter().enumerate() {
                println!("  {:?} x{n}  {:.2}  ({real})", pos, share[gi] / nt);
            }
            println!(
                "  lead back's share of RB carries  {:.2}  (real NFL game 0.70, p10 0.50, p90 0.92)",
                carry[0] / carry[1].max(1.0)
            );
            let top = |f: &dyn Fn(&(f64, f64)) -> f64| {
                let v: Vec<f64> = rush
                    .iter()
                    .filter(|m| !m.is_empty())
                    .map(|m| {
                        let all: f64 = m.values().map(f).sum();
                        m.values().map(f).fold(0.0, f64::max) / all.max(1.0)
                    })
                    .collect();
                v.iter().sum::<f64>() / v.len().max(1) as f64
            };
            println!("  top man's share of team sacks  {:.2}  (about 0.25-0.35)", top(&|x| x.0));
            println!("  top man's share of team TFL    {:.2}  (about 0.20-0.30)", top(&|x| x.1));
        }
    }
    if pbp {
        for p in &results[0].stats.plays {
            println!(
                "Q{} {:>5.0} {:?} {} {}&{:.0} at {:.0} | {:?} {} | {} | {:+.1} yds air {:.0} ttt {:.1} EPA {:+.2} | {}",
                p.quarter,
                p.clock,
                p.score,
                p.offense,
                p.down,
                p.togo,
                p.yardline_100,
                p.kind,
                p.call,
                p.defense_call,
                p.yards,
                p.air_yards,
                p.time_to_throw,
                p.epa,
                p.why
            );
        }
        println!("Final {:?}", results[0].stats.score);
    }
    let n = (games * 2) as f64;
    let mut m = std::collections::BTreeMap::<&str, f64>::new();
    let mut fg = [[0.0f64; 2]; 4];
    let mut ay = [[0.0f64; 6]; 4];
    let mut by_cov = [[0.0f64; 3]; 2];
    let pos_of: std::collections::HashMap<u32, afm_engine::positions::Position> =
        teams.iter().flat_map(|t| t.players.iter().map(|p| (p.id, p.position))).collect();
    let mut tgt_pos = [0.0f64; 4];
    // First tackles on runs by line, linebackers and backs: [runs, yards].
    let mut first_tackle = [[0.0f64; 2]; 3];
    // Seconds between snaps: [running, stopped] x [normal, late, two-minute] x [seconds, plays].
    let mut between = [[[0.0f64; 2]; 3]; 2];
    let mut emergencies = 0.0;
    // Targets by route group: [targets, catches, yards].
    let mut by_route = [[0.0f64; 5]; ROUTE_GROUPS.len()];
    let mut run_b = [0.0f64; 6];
    let mut cov_runs = std::collections::BTreeMap::<String, [f64; 5]>::new();
    // Passing by the coverage called: [dropbacks, dropback EPA, sacks,
    // targets, target EPA, completions].
    let mut cov_pass = std::collections::BTreeMap::<String, [f64; 6]>::new();
    let mut cov_route = std::collections::BTreeMap::<(String, usize), [f64; 3]>::new();
    let mut by_box = [[0.0f64; 2]; 3];
    let mut pts = Vec::new();
    let mut close = 0.0;
    let mut injuries = 0.0;
    for r in &results {
        let s = &r.stats;
        injuries += r.injuries.iter().filter(|i| i.1.weeks >= 1.0).count() as f64;
        pts.push(s.score[0] as f64);
        pts.push(s.score[1] as f64);
        if (s.score[0] - s.score[1]).abs() <= 8 {
            close += 1.0;
        }
        for t in &s.team {
            *m.entry("points").or_default() += t.points as f64;
            *m.entry("plays").or_default() += t.plays as f64;
            *m.entry("att").or_default() += t.att as f64;
            *m.entry("cmp").or_default() += t.cmp as f64;
            *m.entry("pass_yards").or_default() += t.pass_yards;
            *m.entry("carries").or_default() += t.carries as f64;
            *m.entry("rush_yards").or_default() += t.rush_yards;
            *m.entry("sacks").or_default() += t.sacks as f64;
            *m.entry("int").or_default() += t.ints as f64;
            *m.entry("fumbles_lost").or_default() += t.fumbles_lost as f64;
            *m.entry("penalties").or_default() += t.penalties as f64;
            *m.entry("penalty_yards").or_default() += t.penalty_yds;
            *m.entry("third").or_default() += t.third_att as f64;
            *m.entry("third_conv").or_default() += t.third_conv as f64;
            *m.entry("fourth_go").or_default() += t.fourth_att as f64;
            *m.entry("fourth_conv").or_default() += t.fourth_conv as f64;
            *m.entry("dropbacks").or_default() += t.dropbacks as f64;
            *m.entry("punts").or_default() += t.punts as f64;
            *m.entry("explosive").or_default() += t.explosive as f64;
            *m.entry("pressures").or_default() += t.pressures as f64;
            *m.entry("first_downs").or_default() += t.first_downs as f64;
        }
        // Game clock used from one snap to the next, after a play that kept it running.
        for w in s.plays.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            if a.quarter == b.quarter
                && matches!(a.kind, PlayType::Run)
                && !a.first_down
                && !a.touchdown
                && !a.turnover
                && !a.out_of_bounds
                && a.penalties.is_empty()
                && matches!(b.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble)
            {
                *m.entry("run_gap_n").or_default() += 1.0;
                *m.entry("run_gap").or_default() += a.clock - b.clock;
                if a.quarter == 1 || a.quarter == 3 {
                    *m.entry("run_gap_q13_n").or_default() += 1.0;
                    *m.entry("run_gap_q13").or_default() += a.clock - b.clock;
                }
            }
        }
        // Drives: a new one starts when the offense changes.
        let mut last: Option<usize> = None;
        let (mut in_rz, mut rz_td) = (false, false);
        for p in &s.plays {
            if matches!(
                p.kind,
                PlayType::Run
                    | PlayType::Pass
                    | PlayType::Sack
                    | PlayType::Scramble
                    | PlayType::Punt
                    | PlayType::FieldGoal
            ) {
                if last != Some(p.offense) {
                    if in_rz {
                        *m.entry("rz").or_default() += 1.0;
                        *m.entry("rz_td").or_default() += rz_td as u8 as f64;
                    }
                    in_rz = false;
                    rz_td = false;
                    *m.entry("drives").or_default() += 1.0;
                    *m.entry("drive_start").or_default() += p.yardline_100;
                    if p.yardline_100 <= 50.0 {
                        *m.entry("short_fields").or_default() += 1.0;
                    }
                }
                last = Some(p.offense);
                if p.yardline_100 <= 20.0 {
                    in_rz = true;
                }
                if p.touchdown && !p.turnover && in_rz {
                    rz_td = true;
                }
            }
            emergencies += p.emergency as u8 as f64;
            let dropback = matches!(p.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble);
            if dropback {
                let cv = p.defense_call.split(',').next().unwrap_or("").to_string();
                let e = cov_pass.entry(cv).or_insert([0.0; 6]);
                e[0] += 1.0;
                e[1] += p.epa;
                e[2] += (p.kind == PlayType::Sack) as u8 as f64;
                if p.kind == PlayType::Pass {
                    e[3] += 1.0;
                    e[4] += p.epa;
                    e[5] += p.complete as u8 as f64;
                }
            }
            if (dropback || p.kind == PlayType::Run)
                && p.down == 1
                && p.quarter <= 3
                && (p.score[0] - p.score[1]).abs() <= 8
            {
                *m.entry("neutral_plays").or_default() += 1.0;
                *m.entry("neutral_pass").or_default() += dropback as u8 as f64;
            }
            if p.kind == PlayType::Run {
                let b = if p.yards <= 0.0 {
                    "run<=0"
                } else if p.yards < 4.0 {
                    "run1-3"
                } else if p.yards < 7.0 {
                    "run4-6"
                } else if p.yards < 10.0 {
                    "run7-9"
                } else if p.yards < 20.0 {
                    "run10-19"
                } else if p.yards < 40.0 {
                    "run20-39"
                } else {
                    "run40+"
                };
                *m.entry(b).or_default() += 1.0;
                *m.entry("runs_all").or_default() += 1.0;
            }
            if p.kind == PlayType::FieldGoal {
                *m.entry("fg_att").or_default() += 1.0;
            }
            if p.touchdown && matches!(p.kind, PlayType::Run | PlayType::Pass | PlayType::Scramble) && p.yards >= 40.0 {
                *m.entry("long_td").or_default() += 1.0;
                *m.entry(if p.kind == PlayType::Run { "long_td_run" } else { "long_td_pass" }).or_default() += 1.0;
            }
            if p.kind == PlayType::Kickoff && p.returner.is_some() && p.return_yards != 0.0 {
                *m.entry("kr").or_default() += 1.0;
                *m.entry("kr_yds").or_default() += p.return_yards;
                *m.entry("kr40").or_default() += (p.return_yards >= 40.0) as u8 as f64;
            }
            if p.kind == PlayType::Punt && p.returner.is_some() && p.return_yards != 0.0 {
                *m.entry("pr").or_default() += 1.0;
                *m.entry("pr_yds").or_default() += p.return_yards;
                *m.entry("pr20").or_default() += (p.return_yards >= 20.0) as u8 as f64;
            }
            if matches!(p.kind, PlayType::Kickoff | PlayType::Punt) && p.touchdown {
                *m.entry(if p.kind == PlayType::Kickoff { "return_td_ko" } else { "return_td_punt" }).or_default() +=
                    1.0;
            }
            if p.interception && p.defense_td {
                *m.entry("pick_six").or_default() += 1.0;
            }
        }
        // Seconds of game clock from a run or pass to the next snap, by
        // whether the play stopped the clock, as the nflverse script counts
        // them (`data/real-world/scripts/nfl_situations.py`).
        for pair in s.plays.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            let gap = a.clock - b.clock;
            if a.quarter != b.quarter
                || !matches!(a.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble)
                || !(0.0..=60.0).contains(&gap)
            {
                continue;
            }
            let incomplete = a.kind == PlayType::Pass && !a.complete && !a.interception;
            let stopped = incomplete || a.out_of_bounds || a.touchdown || a.penalties.iter().any(|x| x.4);
            let half_left = a.clock + if a.quarter % 2 == 1 { 900.0 } else { 0.0 };
            let tb = if half_left <= 120.0 {
                2
            } else if a.quarter == 4 && half_left <= 420.0 {
                1
            } else {
                0
            };
            let e = &mut between[stopped as usize][tb];
            e[0] += gap;
            e[1] += 1.0;
        }
        // Plays ending out of bounds in normal time (fix list S6.2), as
        // nflverse's out_of_bounds flag counts them.
        for p in &s.plays {
            let half_left = p.clock + if p.quarter % 2 == 1 { 900.0 } else { 0.0 };
            let late = half_left <= 120.0 || (p.quarter == 4 && half_left <= 300.0);
            if late
                || p.nullified
                || !matches!(p.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble)
            {
                continue;
            }
            let oob = p.out_of_bounds as u8 as f64;
            *m.entry("oob_n").or_default() += 1.0;
            *m.entry("oob").or_default() += oob;
            if p.kind == PlayType::Pass && p.complete {
                *m.entry("oob_cmp_n").or_default() += 1.0;
                *m.entry("oob_cmp").or_default() += oob;
            }
            if p.kind == PlayType::Run {
                *m.entry("oob_run_n").or_default() += 1.0;
                *m.entry("oob_run").or_default() += oob;
            }
        }
        for p in &s.plays {
            for &(kind, _, _, _, accepted) in &p.penalties {
                if accepted {
                    *m.entry(kind.name()).or_default() += 1.0;
                }
            }
            if matches!(p.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble) {
                // A rusher who beat his block within 2.5 s, as ESPN's win rate counts it.
                *m.entry("rw_n").or_default() += 1.0;
                *m.entry("rw_any").or_default() += !p.rush_wins.is_empty() as u8 as f64;
            }
            if p.pressured && matches!(p.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble) {
                *m.entry("pr_n").or_default() += 1.0;
                *m.entry("pr_sack").or_default() += (p.kind == PlayType::Sack) as u8 as f64;
                *m.entry("pr_comp").or_default() += p.complete as u8 as f64;
            }
            match p.kind {
                PlayType::Pass => {
                    if p.complete {
                        *m.entry("pass_td").or_default() += p.touchdown as u8 as f64;
                        let band = if p.yardline_100 <= 5.0 {
                            "ptd_5"
                        } else if p.yardline_100 <= 20.0 {
                            "ptd_20"
                        } else {
                            "ptd_far"
                        };
                        *m.entry(band).or_default() += p.touchdown as u8 as f64;
                    }
                    if p.yardline_100 <= 20.0 && !p.throw_away {
                        *m.entry("rz_att").or_default() += 1.0;
                        *m.entry("rz_comp").or_default() += p.complete as u8 as f64;
                        *m.entry("rz_ptd").or_default() += (p.complete && p.touchdown) as u8 as f64;
                    }
                    if p.yards >= 20.0 {
                        *m.entry("explosive_pass").or_default() += 1.0;
                    }
                    *m.entry("air_yards").or_default() += p.air_yards;
                    *m.entry("ttt").or_default() += p.time_to_throw;
                    *m.entry("ttt_u16").or_default() += (p.time_to_throw < 1.6) as u8 as f64;
                    *m.entry("ttt_u25").or_default() += (p.time_to_throw < 2.5) as u8 as f64;
                    *m.entry("ttt_o39").or_default() += (p.time_to_throw > 3.94) as u8 as f64;
                    *m.entry("drops").or_default() += p.drop as u8 as f64;
                    if !p.complete && !p.interception && !p.drop && !p.throw_away {
                        *m.entry(if p.pbu.is_some() { "inc_pbu" } else { "inc_off" }).or_default() += 1.0;
                    }
                    *m.entry("throw_aways").or_default() += p.throw_away as u8 as f64;
                    if !p.throw_away {
                        // Real completion by air-yards band (nflverse 2021-2024).
                        let b = if p.air_yards < 0.0 {
                            0
                        } else if p.air_yards < 10.0 {
                            1
                        } else if p.air_yards < 20.0 {
                            2
                        } else {
                            3
                        };
                        ay[b][0] += 1.0;
                        ay[b][1] += p.complete as u8 as f64;
                        ay[b][2] += p.interception as u8 as f64;
                        ay[b][4] += p.separation;
                        ay[b][5] += (p.separation < 1.0) as u8 as f64;
                        if p.complete {
                            ay[b][3] += p.yac;
                        }
                        if p.complete {
                            *m.entry("yac").or_default() += p.yac;
                        }
                    }
                    if !p.throw_away
                        && let Some(g) = route_group(p.route, p.air_yards)
                    {
                        by_route[g][0] += 1.0;
                        by_route[g][1] += p.complete as u8 as f64;
                        by_route[g][2] += p.yards;
                        by_route[g][3] += p.air_yards;
                        by_route[g][4] += p.time_to_throw;
                        let cv = p.defense_call.split(',').next().unwrap_or("").to_string();
                        let e = cov_route.entry((cv, g)).or_insert([0.0; 3]);
                        e[0] += 1.0;
                        e[1] += p.complete as u8 as f64;
                        e[2] += p.yards;
                    }
                    let cov = if p.coverage_man { 1 } else { 0 };
                    by_cov[cov][0] += 1.0;
                    by_cov[cov][1] += p.complete as u8 as f64;
                    by_cov[cov][2] += p.yards;
                    if let Some(t) = p.target {
                        use afm_engine::positions::Position as P;
                        let k = match pos_of.get(&t) {
                            Some(P::WR) => 0,
                            Some(P::TE) => 1,
                            Some(P::RB) => 2,
                            _ => 3,
                        };
                        tgt_pos[k] += 1.0;
                    }
                }
                PlayType::Run => {
                    *m.entry("rush_td").or_default() += p.touchdown as u8 as f64;
                    if p.yards >= 10.0 {
                        *m.entry("explosive_run").or_default() += 1.0;
                    }
                    *m.entry("ybc").or_default() += p.before_contact;
                    // Who made the first tackle (not on touchdowns): line,
                    // linebacker or back, and the gain on those runs.
                    if !p.touchdown {
                        use afm_engine::positions::Position as P;
                        let k = match p.tacklers.first().and_then(|t| pos_of.get(t)) {
                            Some(P::EDGE | P::DT) => Some(0),
                            Some(P::LB) => Some(1),
                            Some(P::CB | P::S) => Some(2),
                            _ => None,
                        };
                        if let Some(k) = k {
                            first_tackle[k][0] += 1.0;
                            first_tackle[k][1] += p.yards;
                        }
                    }
                    if !p.call.contains("kneel") {
                        let y = p.yards;
                        let k = [0.5, 3.5, 9.5, 19.5, 39.5].iter().filter(|&&b| y >= b).count();
                        run_b[k] += 1.0;
                        let cv = p.defense_call.split(',').next().unwrap_or("").to_string();
                        let e = cov_runs.entry(cv).or_insert([0.0; 5]);
                        e[0] += 1.0;
                        e[1] += y;
                        e[2] += (k == 5) as u8 as f64;
                        e[3] += p.before_contact;
                        e[4] += p.box_count as f64;
                        if k == 5 && std::env::var("AFM_LONG").is_ok() {
                            println!(
                                "LONG {:.0} | {} | {} | box {} | {}",
                                y, p.call, p.defense_call, p.box_count, p.why
                            );
                        }
                        let bx = if p.box_count <= 6 {
                            0
                        } else if p.box_count == 7 {
                            1
                        } else {
                            2
                        };
                        by_box[bx][0] += 1.0;
                        by_box[bx][1] += y;
                    }
                    *m.entry("missed_tackles").or_default() += p.missed_tackles.len() as f64;
                }
                PlayType::Scramble => *m.entry("scrambles").or_default() += 1.0,
                PlayType::FieldGoal => {
                    let b = afm_engine::game::stats::fg_band(p.kick_distance);
                    fg[b][0] += 1.0;
                    fg[b][1] += p.kick_good as u8 as f64;
                }
                PlayType::ExtraPoint => {
                    *m.entry("xp_att").or_default() += 1.0;
                    *m.entry("xp_made").or_default() += p.kick_good as u8 as f64;
                }
                PlayType::Punt => {
                    *m.entry("punt_gross").or_default() += p.kick_distance;
                    *m.entry("punt_in20").or_default() += p.inside20 as u8 as f64;
                }
                PlayType::Kickoff => {
                    *m.entry("kickoffs").or_default() += 1.0;
                    *m.entry("ko_tb").or_default() += p.touchback as u8 as f64;
                }
                _ => {}
            }
            if matches!(p.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble) {
                *m.entry("epa").or_default() += p.epa;
                *m.entry("success").or_default() += p.success() as u8 as f64;
                // cfbfastR's success: half the distance on 1st down, 70% on
                // 2nd, all of it on 3rd and 4th.
                let need = match p.down {
                    1 => 0.5,
                    2 => 0.7,
                    _ => 1.0,
                };
                let cfb_ok = !p.turnover && (p.touchdown || p.first_down || p.yards >= need * p.togo);
                *m.entry("success_cfb").or_default() += cfb_ok as u8 as f64;
                let k = if p.kind == PlayType::Run { "run" } else { "db" };
                *m.entry(if k == "run" { "run_n" } else { "db_n" }).or_default() += 1.0;
                *m.entry(if k == "run" { "run_epa" } else { "db_epa" }).or_default() += p.epa;
                *m.entry(if k == "run" { "run_succ" } else { "db_succ" }).or_default() += p.success() as u8 as f64;
            }
        }
    }
    let g = |k: &str| m.get(k).copied().unwrap_or(0.0);
    let mean = pts.iter().sum::<f64>() / pts.len() as f64;
    let sd = (pts.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / pts.len() as f64).sqrt();
    let real: Value = serde_json::from_str(include_str!("../../../../data/real-world/nfl_game_targets.json")).unwrap();
    let pt = &real["per_team_game"];
    let rr = &real["rates"];
    let row = |name: &str, ours: f64, theirs: Option<f64>| match theirs {
        Some(t) => println!("{:<28} {:>9.3} {:>9.3}  {:+6.1}%", name, ours, t, (ours / t - 1.0) * 100.0),
        None => println!("{:<28} {:>9.3}", name, ours),
    };
    println!("{} games at {:?} in {:.1}s ({:.1} ms/game)", games, level, secs, secs * 1000.0 / games as f64);
    println!("{:<28} {:>9} {:>9}", "per team game", "engine", "NFL");
    let f = |k: &str| pt[k].as_f64();
    row("points", mean, real["points_per_team_game"].as_f64());
    row("points sd", sd, real["points_sd"].as_f64());
    row("games decided by <= 8", close / games as f64, real["share_games_decided_by_8_or_less"].as_f64());
    // Home field (trial 4 topic 3): the same matchups at both ends, crowd
    // 0.65. Real: NFL 2021-25 2.1 points (nfldata), FBS conference games
    // 2.4, FCS 2.0 (CFBD 2014-25).
    {
        let m: Vec<f64> = results.iter().map(|r| (r.stats.team[0].points - r.stats.team[1].points) as f64).collect();
        let hm = m.iter().sum::<f64>() / m.len() as f64;
        let se = (m.iter().map(|x| (x - hm).powi(2)).sum::<f64>() / m.len() as f64).sqrt() / (m.len() as f64).sqrt();
        let wins =
            m.iter().filter(|&&x| x > 0.0).count() as f64 / m.iter().filter(|&&x| x != 0.0).count().max(1) as f64;
        let target = if level == Level::NFL { 2.1 } else { 2.4 };
        println!("{:<28} {:>9.3} {:>9.3}  (se {:.2}, home wins {:.3})", "home margin", hm, target, se, wins);
    }
    row("plays", g("plays") / n, f("plays"));
    {
        let sit: Value = serde_json::from_str(include_str!("../../../../data/real-world/nfl_situations.json")).unwrap();
        for (si, sname) in ["running", "stopped"].iter().enumerate() {
            for (ti, tname) in ["normal", "late", "two_minute"].iter().enumerate() {
                let e = between[si][ti];
                let real = sit["seconds_between_snaps"][format!("{sname}_{tname}")].as_f64();
                row(&format!("snap to snap, {sname} {tname}"), e[0] / e[1].max(1.0), real);
            }
        }
        let all = |si: usize| between[si].iter().map(|e| e[1]).sum::<f64>();
        row("share of plays stopping clock", all(1) / (all(0) + all(1)), None);
        // nflverse 2023, regular season, outside the last two minutes of a
        // half and the last five of the game.
        let real = |v: f64| if level == Level::NFL { Some(v) } else { None };
        row("plays ending out of bounds", g("oob") / g("oob_n").max(1.0), real(0.107));
        row("  completions out of bounds", g("oob_cmp") / g("oob_cmp_n").max(1.0), real(0.207));
        row("  designed runs out of bounds", g("oob_run") / g("oob_run_n").max(1.0), real(0.055));
    }
    row("pass attempts", g("att") / n, f("att"));
    row("completions", g("cmp") / n, f("cmp"));
    row("pass yards (net)", g("pass_yards") / n, Some(f("pass_yards").unwrap() - f("sack_yards").unwrap()));
    row("carries", g("carries") / n, f("carries"));
    row("rush yards", g("rush_yards") / n, f("rush_yards"));
    row("sacks", g("sacks") / n, f("sacks"));
    row("interceptions", g("int") / n, f("int"));
    row("fumbles lost", g("fumbles_lost") / n, f("fumbles_lost"));
    row("penalties", g("penalties") / n, f("penalties"));
    row("penalty yards", g("penalty_yards") / n, f("penalty_yards"));
    // Accepted fouls by kind per team game: Claude's reading of the 2023
    // season totals (nflpenalties.com), for diagnosis only. Kinds the engine
    // folds together: offensive holding with offensive illegal hands;
    // defensive holding with illegal contact and defensive illegal hands;
    // unnecessary roughness with horse collars and taunting; illegal
    // formation with shifts, motion, ineligible downfield and too many men;
    // offside with neutral zone and encroachment.
    for (k, real) in [
        ("offensive holding", 1.26),
        ("false start", 1.1),
        ("pass interference", 0.42),
        ("defensive holding", 0.65),
        ("unnecessary roughness", 0.47),
        ("roughing the passer", 0.15),
        ("illegal block in the back", 0.28),
        ("offside", 0.52),
        ("delay of game", 0.26),
        ("illegal formation", 0.6),
        ("face mask", 0.18),
        ("offensive pass interference", 0.17),
        ("intentional grounding", 0.07),
    ] {
        row(&format!("  {k}"), g(k) / n, Some(real));
    }
    // The officials and the toss, per game (both teams): the targets in
    // docs/match-engine.md, "Live games, officials and the toss" (NFL 2024:
    // 133 coach's challenges in 285 games, 50 won; booth reviews about 0.7
    // a game, a little over half reversed; winners defer about 82%).
    {
        use afm_engine::game::live::{Reviewer, TossChoice};
        let per = games.max(1) as f64;
        let calls: Vec<&afm_engine::game::live::Officiating> =
            results.iter().flat_map(|r| r.stats.plays.iter().filter_map(|p| p.officiating.as_ref())).collect();
        let count =
            |f: &dyn Fn(&afm_engine::game::live::Officiating) -> bool| calls.iter().filter(|o| f(o)).count() as f64;
        let chal = count(&|o| matches!(o.reviewer, Reviewer::Challenge(_)));
        let booth = count(&|o| matches!(o.reviewer, Reviewer::Booth | Reviewer::Assist));
        let real_chal = matches!(level, Level::NFL).then_some(133.0 / 285.0);
        row("close calls a game", calls.len() as f64 / per, None);
        row("  wrong on the field", count(&|o| o.wrong) / per, matches!(level, Level::NFL).then_some(0.8));
        row("  wrong and left standing", count(&|o| o.wrong && !o.overturned) / per, None);
        row("coach's challenges a game", chal / per, real_chal);
        row(
            "  won",
            count(&|o| matches!(o.reviewer, Reviewer::Challenge(_)) && o.overturned) / chal.max(1.0),
            matches!(level, Level::NFL).then_some(50.0 / 133.0),
        );
        row("booth reviews a game", booth / per, matches!(level, Level::NFL).then_some(0.7));
        row(
            "  reversed",
            count(&|o| matches!(o.reviewer, Reviewer::Booth | Reviewer::Assist) && o.overturned) / booth.max(1.0),
            matches!(level, Level::NFL).then_some(0.55),
        );
        if std::env::var("AFM_REFS").is_ok() {
            let mut m: std::collections::BTreeMap<String, [u32; 6]> = Default::default();
            for o in &calls {
                let e = m.entry(format!("{:?}", o.call)).or_default();
                e[0] += 1;
                e[1] += o.wrong as u32;
                let k = match o.reviewer {
                    Reviewer::None => 2,
                    Reviewer::Booth | Reviewer::Assist => 3,
                    Reviewer::Challenge(_) => 4,
                };
                e[k] += 1;
                e[5] += (o.overturned) as u32;
            }
            for (k, e) in m {
                eprintln!(
                    "{k:12} close {:.3} wrong {:.3} none {:.3} booth {:.3} chal {:.3} over {:.3}",
                    e[0] as f64 / per,
                    e[1] as f64 / per,
                    e[2] as f64 / per,
                    e[3] as f64 / per,
                    e[4] as f64 / per,
                    e[5] as f64 / per
                );
            }
        }
        let tosses: Vec<TossChoice> = results.iter().filter_map(|r| r.toss.map(|t| t.1)).collect();
        let defer = tosses.iter().filter(|&&c| c == TossChoice::Defer).count() as f64 / tosses.len().max(1) as f64;
        row(
            "toss winners who defer",
            defer,
            Some(match level {
                Level::NFL => 0.82,
                Level::HS => 0.60,
                _ => 0.78,
            }),
        );
    }
    row("third downs", g("third") / n, f("third"));
    row("fourth down tries", g("fourth_go") / n, f("fourth_go"));
    row("pass TD", g("pass_td") / n, f("pass_td"));
    row("  from inside the 5", g("ptd_5") / n, None);
    row("  from the 6 to the 20", g("ptd_20") / n, None);
    row("  from outside the 20", g("ptd_far") / n, None);
    // Red zone passing, Claude's reading of recent NFL seasons: about 53%
    // completed and a touchdown on about 19% of attempts.
    row("red zone completion", g("rz_comp") / g("rz_att"), Some(0.53));
    row("red zone TD per attempt", g("rz_ptd") / g("rz_att"), Some(0.19));
    row("rush TD", g("rush_td") / n, f("rush_td"));
    row("scrambles", g("scrambles") / n, None);
    row("punts", g("punts") / n, None);
    // Real drive numbers: NFL 2021-2024 averages (Pro Football Reference drive stats).
    row("pass rate 1st down neutral", g("neutral_pass") / g("neutral_plays").max(1.0), Some(0.50));
    row("clock per run play (all)", g("run_gap") / g("run_gap_n").max(1.0), None);
    row("clock per run play (Q1/Q3)", g("run_gap_q13") / g("run_gap_q13_n").max(1.0), Some(38.0));
    // engine_targets.json (nflverse 2021-25, a trip is a drive with a play
    // at or inside the 20).
    row("red zone trips", g("rz") / n, Some(3.98));
    row("red zone TD rate", g("rz_td") / g("rz").max(1.0), Some(0.598));
    row("drives", g("drives") / n, Some(11.0));
    row("drive start (to goal)", g("drive_start") / g("drives").max(1.0), Some(70.5));
    row("drives starting in opp half", g("short_fields") / g("drives").max(1.0), None);
    row("points per drive", g("points") / g("drives").max(1.0), Some(2.0));
    row("FG attempts", g("fg_att") / n, Some(1.85));
    row("TDs of 40+ yards", g("long_td") / n, None);
    row("  of them runs", g("long_td_run") / n, None);
    // Real run distribution: nflverse 2021-2024 designed runs (approximate).
    for (k, real) in [
        ("run<=0", 0.19),
        ("run1-3", 0.33),
        ("run4-6", 0.235),
        ("run7-9", 0.10),
        ("run10-19", 0.115),
        ("run20-39", 0.022),
        ("run40+", 0.006),
    ] {
        row(k, g(k) / g("runs_all").max(1.0), Some(real));
    }
    row("kickoff returns", g("kr") / n, None);
    // 2025 dynamic kickoff (touchback to the 35): 74.6% returned, 25.9 a
    // return (engine_targets.json, red_zone_and_returns); the 2021-25 mix
    // of rules is no target for one rule book.
    row("kickoff return avg", g("kr_yds") / g("kr").max(1.0), Some(25.9));
    row("kickoff returns 40+", g("kr40") / g("kr").max(1.0), Some(0.05));
    row("punt returns", g("pr") / n, None);
    row("punt return avg", g("pr_yds") / g("pr").max(1.0), Some(9.5));
    row("punt returns 20+", g("pr20") / g("pr").max(1.0), Some(0.08));
    row("kickoff return TDs", g("return_td_ko") / n, Some(0.012));
    row("punt return TDs", g("return_td_punt") / n, Some(0.012));
    row("pick sixes", g("pick_six") / n, Some(0.05));
    row("injuries (1+ week)", injuries / n, None);
    println!("{:<28} {:>9} {:>9}", "rates", "engine", "NFL");
    row("completion %", g("cmp") / g("att"), rr["completion_pct"].as_f64());
    row("sack rate", g("sacks") / g("dropbacks"), rr["sack_rate"].as_f64());
    row("yards per attempt", (g("pass_yards") + 0.0) / g("att"), rr["yards_per_attempt"].as_f64());
    row("int rate", g("int") / g("att"), rr["int_rate"].as_f64());
    row("yards per carry", g("rush_yards") / g("carries"), rr["yards_per_carry"].as_f64());
    row("pass rate", g("dropbacks") / g("plays"), rr["pass_rate"].as_f64());
    row("third down conv", g("third_conv") / g("third"), rr["third_down_conversion"].as_f64());
    row("fourth down conv", g("fourth_conv") / g("fourth_go"), rr["fourth_down_conversion"].as_f64());
    row("explosive run rate", g("explosive_run") / g("carries"), rr["explosive_run_rate"].as_f64());
    row("explosive pass rate", g("explosive_pass") / g("att"), rr["explosive_pass_rate"].as_f64());
    row("success rate", g("success") / g("plays"), rr["success_rate"].as_f64());
    row("EPA per play", g("epa") / g("plays"), rr["epa_per_play"].as_f64());
    // engine_targets.json: dropbacks success 0.454, EPA 0.029; designed runs
    // about 0.41 and -0.05 (box-count rows weighted; runs here include QB
    // keepers).
    row("  dropback success", g("db_succ") / g("db_n"), Some(0.454));
    row("  dropback EPA", g("db_epa") / g("db_n"), Some(0.029));
    row("  run success", g("run_succ") / g("run_n"), Some(0.41));
    row("  run EPA", g("run_epa") / g("run_n"), Some(-0.05));
    row("XP %", g("xp_made") / g("xp_att"), rr["xp_pct"].as_f64());
    row("punt gross", g("punt_gross") / g("punts"), rr["punt_gross"].as_f64());
    row("punt inside 20", g("punt_in20") / g("punts"), rr["punt_inside20_rate"].as_f64());
    row("kickoff touchback", g("ko_tb") / g("kickoffs"), Some(0.22));
    // FTN 2021-25 (engine_targets.json, dropbacks by pressure): pressured on
    // 28.3% of dropbacks, 48.9% completed then; scrambles 4.9% of dropbacks.
    // Time to throw (engine_targets.json): mean 2.71 s, p10 1.6, p50 2.5,
    // p90 3.94; a pressured dropback ends in a sack 13.9% of the time.
    row("pressure rate", g("pressures") / g("dropbacks"), Some(0.283));
    row("scramble rate", g("scrambles") / g("dropbacks"), Some(0.049));
    row("time to throw", g("ttt") / g("att"), Some(2.71));
    row("  thrown under 1.6 s", g("ttt_u16") / g("att"), Some(0.1));
    row("  thrown under 2.5 s", g("ttt_u25") / g("att"), Some(0.5));
    row("  thrown after 3.94 s", g("ttt_o39") / g("att"), Some(0.1));
    row("sacks per pressure", g("pr_sack") / g("pr_n"), Some(0.139));
    row("dropbacks with a rush win", g("rw_any") / g("rw_n"), None);
    row("completions per pressure", g("pr_comp") / g("pr_n"), Some(0.489));
    row("air yards per att", g("air_yards") / g("att"), None);
    row("drops per att", g("drops") / g("att"), None);
    row("broken up per att", g("inc_pbu") / g("att"), None);
    row("off target per att", g("inc_off") / g("att"), None);
    row("throw aways per att", g("throw_aways") / g("att"), None);
    row("yards before contact", g("ybc") / g("carries"), None);
    row("missed tackles per run", g("missed_tackles") / g("carries"), None);
    // Real: nflverse play-by-play 2023-24, backs' designed runs, the first
    // tackler's position (solo, then shared tackle), touchdowns left out.
    let ft: f64 = first_tackle.iter().map(|r| r[0]).sum();
    for (k, (name, share, yds)) in
        [("line", 0.330, 2.44), ("linebackers", 0.371, 3.35), ("backs", 0.296, 6.86)].into_iter().enumerate()
    {
        row(&format!("first tackle on runs: {name}"), first_tackle[k][0] / ft, Some(share));
        row("  gain when they make it", first_tackle[k][1] / first_tackle[k][0].max(1.0), Some(yds));
    }
    row("YAC per completion", g("yac") / g("cmp"), Some(5.2));
    let real_ay = [(0.176, 0.788), (0.501, 0.709), (0.207, 0.545), (0.115, 0.360)];
    let tot: f64 = ay.iter().map(|b| b[0]).sum();
    for (k, name) in ["air < 0", "air 0-9", "air 10-19", "air 20+"].iter().enumerate() {
        row(&format!("{name} share"), ay[k][0] / tot.max(1.0), Some(real_ay[k].0));
        row(&format!("{name} completion"), ay[k][1] / ay[k][0].max(1.0), Some(real_ay[k].1));
        row(&format!("{name} int rate"), ay[k][2] / ay[k][0].max(1.0), None);
        row(&format!("{name} YAC"), ay[k][3] / ay[k][1].max(1.0), None);
        row(&format!("{name} separation"), ay[k][4] / ay[k][0].max(1.0), None);
        row(&format!("{name} tight window"), ay[k][5] / ay[k][0].max(1.0), None);
    }
    for (k, name) in ["zone", "man"].iter().enumerate() {
        row(&format!("{name}: att share / cmp / ypa"), by_cov[k][0] / (by_cov[0][0] + by_cov[1][0]).max(1.0), None);
        row("", by_cov[k][1] / by_cov[k][0].max(1.0), None);
        row("", by_cov[k][2] / by_cov[k][0].max(1.0), None);
    }
    // Targets by route against NFL 2022-25 (data/real-world/engine_targets.json),
    // with the real route types pooled into the engine's route groups.
    let et: Value = serde_json::from_str(include_str!("../../../../data/real-world/engine_targets.json")).unwrap();
    let tr = &et["targets_by_route"];
    let rt: f64 = by_route.iter().map(|b| b[0]).sum();
    let real_all: f64 = ROUTE_GROUPS.iter().flat_map(|g| g.1.iter()).map(|k| tr[*k]["n"].as_f64().unwrap_or(0.0)).sum();
    println!("targets by route            share  real   catch  real    ypt   real    air    ttt");
    for (k, (name, real_keys)) in ROUTE_GROUPS.iter().enumerate() {
        let (mut n, mut c, mut y) = (0.0, 0.0, 0.0);
        for key in real_keys.iter() {
            let r = &tr[*key];
            let rn = r["n"].as_f64().unwrap_or(0.0);
            n += rn;
            c += rn * r["catch"].as_f64().unwrap_or(0.0);
            y += rn * r["ypt"].as_f64().unwrap_or(0.0);
        }
        let b = by_route[k];
        println!(
            "  {name:<24} {:>6.3} {:>6.3} {:>6.3} {:>6.3} {:>6.2} {:>6.2} {:>6.1} {:>6.2}",
            b[0] / rt.max(1.0),
            n / real_all.max(1.0),
            b[1] / b[0].max(1.0),
            c / n.max(1.0),
            b[2] / b[0].max(1.0),
            y / n.max(1.0),
            b[3] / b[0].max(1.0),
            b[4] / b[0].max(1.0)
        );
    }
    // Designed runs against nflverse 2021-25 (data/real-world/position_value.json).
    let pv: Value = serde_json::from_str(include_str!("../../../../data/real-world/position_value.json")).unwrap();
    let runs = &pv["designed_runs"];
    let rn: f64 = run_b.iter().sum();
    for (k, name) in ["<=0", "1-3", "4-9", "10-19", "20-39", "40+"].iter().enumerate() {
        row(&format!("runs {name} yards"), run_b[k] / rn.max(1.0), runs["share"][name].as_f64());
    }
    for (k, name) in ["<=6", "7", ">=8"].iter().enumerate() {
        row(
            &format!("ypc, {name} in the box"),
            by_box[k][1] / by_box[k][0].max(1.0),
            runs["by_box"][name]["ypc"].as_f64(),
        );
        row(&format!("share of runs, {name} in box"), by_box[k][0] / rn.max(1.0), None);
    }
    let tt: f64 = tgt_pos.iter().sum::<f64>().max(1.0);
    row("emergency snaps per game", emergencies / games as f64, None);
    row("targets to WR", tgt_pos[0] / tt, Some(0.593));
    row("targets to TE", tgt_pos[1] / tt, Some(0.219));
    row("targets to RB", tgt_pos[2] / tt, Some(0.181));
    row("targets to others", tgt_pos[3] / tt, None);
    // Real EPA per target by coverage (NFL 2022-25, FTN charting pooled
    // over routes, `targets_by_route_and_coverage`): no coverage gives up
    // much more than another.
    println!("passing by coverage          share  db EPA  sack   tgt EPA  real   catch");
    let real_tgt = |k: &str| match k {
        "Cover 0" => Some(0.171),
        "Cover 1" => Some(0.179),
        "Cover 2" => Some(0.149),
        "Cover 3" => Some(0.190),
        "Cover 4" => Some(0.158),
        "2-man" => Some(0.134),
        "Cover 6" => Some(0.179),
        _ => None,
    };
    let dbs: f64 = cov_pass.values().map(|v| v[0]).sum::<f64>().max(1.0);
    for (k, v) in &cov_pass {
        println!(
            "  {:<26} {:.3}  {:+.3}  {:.3}  {:+.3}  {}  {:.3}",
            k,
            v[0] / dbs,
            v[1] / v[0].max(1.0),
            v[2] / v[0].max(1.0),
            v[4] / v[3].max(1.0),
            real_tgt(k).map_or("  -   ".to_string(), |r| format!("{r:+.3}")),
            v[5] / v[3].max(1.0)
        );
    }
    if std::env::var("AFM_COVROUTE").is_ok() {
        let tc = &et["targets_by_route_and_coverage"];
        for (cov, key) in [("Cover 1", "COVER_1"), ("Cover 2", "COVER_2"), ("Cover 3", "COVER_3"), ("2-man", "2_MAN")] {
            let tot: f64 =
                (0..ROUTE_GROUPS.len()).map(|g| cov_route.get(&(cov.to_string(), g)).map_or(0.0, |v| v[0])).sum();
            let mut rtot = 0.0;
            for (_, keys) in ROUTE_GROUPS.iter() {
                for k in keys.iter() {
                    rtot += tc[*k][key]["n"].as_f64().unwrap_or(0.0);
                }
            }
            println!("{cov}: route share real catch real ypt real");
            for (g, (name, keys)) in ROUTE_GROUPS.iter().enumerate() {
                let (mut n, mut c, mut y) = (0.0, 0.0, 0.0);
                for k in keys.iter() {
                    let r = &tc[*k][key];
                    let rn = r["n"].as_f64().unwrap_or(0.0);
                    n += rn;
                    c += rn * r["catch"].as_f64().unwrap_or(0.0);
                    y += rn * r["ypt"].as_f64().unwrap_or(0.0);
                }
                let v = cov_route.get(&(cov.to_string(), g)).copied().unwrap_or([0.0; 3]);
                println!(
                    "  {name:<22} {:.3} {:.3}  {:.3} {:.3}  {:5.2} {:5.2}",
                    v[0] / tot.max(1.0),
                    n / rtot.max(1.0),
                    v[1] / v[0].max(1.0),
                    c / n.max(1.0),
                    v[2] / v[0].max(1.0),
                    y / n.max(1.0)
                );
            }
        }
    }
    if std::env::var("AFM_LONG").is_ok() {
        for (k, v) in &cov_runs {
            println!(
                "runs vs {k:<12} share {:.3} ypc {:.2} 40+ {:.3} before contact {:.2} box {:.2}",
                v[0] / rn,
                v[1] / v[0],
                v[2] / v[0],
                v[3] / v[0],
                v[4] / v[0]
            );
        }
    }
    let fgr = &real["field_goal_pct_by_distance"];
    for (k, name) in ["under30", "30-39", "40-49", "50+"].iter().enumerate() {
        row(&format!("FG {name} (n={:.0})", fg[k][0]), fg[k][1] / fg[k][0].max(1.0), fgr[name]["pct"].as_f64());
    }
    // College levels: the real cfbfastR figures (2022-24) for the main rows.
    if matches!(level, Level::FBS | Level::FCS) {
        let cr: Value =
            serde_json::from_str(include_str!("../../../../data/real-world/college_game_targets.json")).unwrap();
        let d = &cr["divisions"][if level == Level::FBS { "fbs" } else { "fcs" }];
        let (cp, cq) = (&d["per_team_game"], &d["rates"]);
        println!("{:<28} {:>9} {:>9}", "college (cfbfastR)", "engine", "real");
        row("points", mean, cp["final_score"].as_f64());
        row("plays", g("plays") / n, cp["plays"].as_f64());
        row("pass attempts", g("att") / n, cp["att"].as_f64());
        row("carries", g("carries") / n, cp["carries"].as_f64());
        row("rush yards", g("rush_yards") / n, cp["rush_yards"].as_f64());
        row("pass TD", g("pass_td") / n, cp["pass_td"].as_f64());
        row("rush TD", g("rush_td") / n, cp["rush_td"].as_f64());
        row("sacks", g("sacks") / n, cp["sacks"].as_f64());
        row("interceptions", g("int") / n, cp["int"].as_f64());
        row("completion %", g("cmp") / g("att"), cq["completion_pct"].as_f64());
        row("yards per attempt", g("pass_yards") / g("att"), cq["yards_per_attempt"].as_f64());
        row("yards per carry", g("rush_yards") / g("carries"), cq["yards_per_carry"].as_f64());
        row("pass rate", g("dropbacks") / g("plays"), cq["pass_rate"].as_f64());
        row("third down conv", g("third_conv") / g("third"), cq["third_down_conversion"].as_f64());
        row("explosive run rate", g("explosive_run") / g("carries"), cq["explosive_run_rate"].as_f64());
        row("success rate (cfbfastR's)", g("success_cfb") / g("plays"), cq["success_rate"].as_f64());
    }
    if qb_stats {
        let c: Vec<u64> =
            afm_engine::game::scrimmage::QB_WHY.iter().map(|x| x.load(std::sync::atomic::Ordering::Relaxed)).collect();
        let total = c.iter().sum::<u64>().max(1) as f64;
        let names =
            ["read 1", "read 2", "read 3", "read 4+", "any open late", "checkdown", "forced", "throwaway", "scramble"];
        for (n, v) in names.iter().zip(&c) {
            println!("  QB choice {n:<14} {:.3}", *v as f64 / total);
        }
        let cc: Vec<u64> = afm_engine::game::scrimmage::QB_WHY_COV
            .iter()
            .map(|x| x.load(std::sync::atomic::Ordering::Relaxed))
            .collect();
        for (ci, cn) in ["C0", "C1", "C2", "T2", "C3", "C4", "2M", "C6"].iter().enumerate() {
            let row = &cc[ci * 9..ci * 9 + 9];
            let t = row.iter().sum::<u64>().max(1) as f64;
            let s: Vec<String> = row.iter().map(|v| format!("{:.3}", *v as f64 / t)).collect();
            println!("  QB choice {cn}: {}", s.join(" "));
        }
        let sd: Vec<u64> = afm_engine::game::scrimmage::THROW_SD
            .iter()
            .map(|x| x.load(std::sync::atomic::Ordering::Relaxed))
            .collect();
        for (k, n) in ["under 10", "10-20", "over 20"].iter().enumerate() {
            println!("  throw miss sd {n:<9} {:.2} yd", sd[2 * k] as f64 / 1000.0 / sd[2 * k + 1].max(1) as f64);
        }
        let throws = (sd[1] + sd[3] + sd[5]).max(1) as f64;
        let batted = afm_engine::game::scrimmage::BATTED.load(std::sync::atomic::Ordering::Relaxed);
        println!("  batted at the line {:.1}% of throws", 100.0 * batted as f64 / throws);
    }
}
