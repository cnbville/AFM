//! Development check (section 3, topic 10): play many games between
//! synthetic teams at a level and compare the averages to real football.
//! Usage: match_check [nfl|fbs|hs] [games] [seed] [pbp]
//! With `pbp`, prints the first game's play-by-play with its whys.

use afm_engine::game::quick_game;
use afm_engine::game::stats::PlayType;
use afm_engine::game::synthetic::synthetic_league;
use afm_engine::player::Level;
use rayon::prelude::*;
use serde_json::Value;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let level = match args.get(1).map(|s| s.as_str()) {
        Some("fbs") => Level::FBS,
        Some("hs") => Level::HS,
        _ => Level::NFL,
    };
    let games: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(200);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
    let pbp = args.get(4).is_some_and(|s| s == "pbp");
    let teams = synthetic_league(level, 32, seed);
    // AFM_TRACE=1 prints every agent's state (use with one game).
    if std::env::var("AFM_TRACE").is_ok() {
        afm_engine::game::sim::TRACE.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    let t0 = std::time::Instant::now();
    let results: Vec<_> = (0..games)
        .into_par_iter()
        .map(|g| {
            let h = (g * 7) % 32;
            let a = (g * 13 + 5) % 32;
            let a = if a == h { (a + 1) % 32 } else { a };
            quick_game(&teams[h], &teams[a], level, seed as u64 * 1_000_003 + g as u64)
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
        for (g, r) in results.iter().enumerate() {
            let h = (g * 7) % 32;
            let a = (g * 13 + 5) % 32;
            let a = if a == h { (a + 1) % 32 } else { a };
            for (k, ti) in [h, a].into_iter().enumerate() {
                let lines = &r.stats.players[k];
                let team = &teams[ti];
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
            println!("  lead back's share of RB carries  {:.2}  (0.55-0.65)", carry[0] / carry[1].max(1.0));
        }
    }
    if pbp {
        for p in &results[0].stats.plays {
            println!(
                "Q{} {:>5.0} {} {}&{:.0} at {:.0} | {:?} {} | {} | {:+.1} yds air {:.0} ttt {:.1} EPA {:+.2} | {}",
                p.quarter,
                p.clock,
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
    let mut run_b = [0.0f64; 6];
    let mut cov_runs = std::collections::BTreeMap::<String, [f64; 3]>::new();
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
            let dropback = matches!(p.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble);
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
            if p.interception && p.touchdown {
                *m.entry("pick_six").or_default() += 1.0;
            }
        }
        for p in &s.plays {
            if p.pressured && matches!(p.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble) {
                *m.entry("pr_n").or_default() += 1.0;
                *m.entry("pr_sack").or_default() += (p.kind == PlayType::Sack) as u8 as f64;
                *m.entry("pr_comp").or_default() += p.complete as u8 as f64;
            }
            match p.kind {
                PlayType::Pass => {
                    if p.complete {
                        *m.entry("pass_td").or_default() += p.touchdown as u8 as f64;
                    }
                    if p.yards >= 20.0 {
                        *m.entry("explosive_pass").or_default() += 1.0;
                    }
                    *m.entry("air_yards").or_default() += p.air_yards;
                    *m.entry("ttt").or_default() += p.time_to_throw;
                    *m.entry("drops").or_default() += p.drop as u8 as f64;
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
                    let cov = if p.coverage_man { 1 } else { 0 };
                    by_cov[cov][0] += 1.0;
                    by_cov[cov][1] += p.complete as u8 as f64;
                    by_cov[cov][2] += p.yards;
                }
                PlayType::Run => {
                    *m.entry("rush_td").or_default() += p.touchdown as u8 as f64;
                    if p.yards >= 10.0 {
                        *m.entry("explosive_run").or_default() += 1.0;
                    }
                    *m.entry("ybc").or_default() += p.before_contact;
                    if !p.call.contains("kneel") {
                        let y = p.yards;
                        let k = [0.5, 3.5, 9.5, 19.5, 39.5].iter().filter(|&&b| y >= b).count();
                        run_b[k] += 1.0;
                        let cv = p.defense_call.split(',').next().unwrap_or("").to_string();
                        let e = cov_runs.entry(cv).or_insert([0.0; 3]);
                        e[0] += 1.0;
                        e[1] += y;
                        e[2] += (k == 5) as u8 as f64;
                        if k == 5 && std::env::var("AFM_LONG").is_ok() {
                            println!("LONG {:.0} | {} | {} | box {} | {}", y, p.call, p.defense_call, p.box_count, p.why);
                        }
                        let bx = if p.box_count <= 6 { 0 } else if p.box_count == 7 { 1 } else { 2 };
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
    row("plays", g("plays") / n, f("plays"));
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
    row("third downs", g("third") / n, f("third"));
    row("fourth down tries", g("fourth_go") / n, f("fourth_go"));
    row("pass TD", g("pass_td") / n, f("pass_td"));
    row("rush TD", g("rush_td") / n, f("rush_td"));
    row("scrambles", g("scrambles") / n, None);
    row("punts", g("punts") / n, None);
    // Real drive numbers: NFL 2021-2024 averages (Pro Football Reference drive stats).
    row("pass rate 1st down neutral", g("neutral_pass") / g("neutral_plays").max(1.0), Some(0.50));
    row("clock per run play (all)", g("run_gap") / g("run_gap_n").max(1.0), None);
    row("clock per run play (Q1/Q3)", g("run_gap_q13") / g("run_gap_q13_n").max(1.0), Some(38.0));
    row("red zone trips", g("rz") / n, Some(3.2));
    row("red zone TD rate", g("rz_td") / g("rz").max(1.0), Some(0.56));
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
    row("kickoff return avg", g("kr_yds") / g("kr").max(1.0), Some(26.5));
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
    row("XP %", g("xp_made") / g("xp_att"), rr["xp_pct"].as_f64());
    row("punt gross", g("punt_gross") / g("punts"), rr["punt_gross"].as_f64());
    row("punt inside 20", g("punt_in20") / g("punts"), rr["punt_inside20_rate"].as_f64());
    row("kickoff touchback", g("ko_tb") / g("kickoffs"), rr["kickoff_touchback_rate"].as_f64());
    row("pressure rate", g("pressures") / g("dropbacks"), None);
    row("time to throw", g("ttt") / g("att"), None);
    row("sacks per pressure", g("pr_sack") / g("pr_n"), None);
    row("completions per pressure", g("pr_comp") / g("pr_n"), None);
    row("air yards per att", g("air_yards") / g("att"), None);
    row("drops per att", g("drops") / g("att"), None);
    row("throw aways per att", g("throw_aways") / g("att"), None);
    row("yards before contact", g("ybc") / g("carries"), None);
    row("missed tackles per run", g("missed_tackles") / g("carries"), None);
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
    // Designed runs against nflverse 2021-25 (data/real-world/position_value.json).
    let pv: Value = serde_json::from_str(include_str!("../../../../data/real-world/position_value.json")).unwrap();
    let runs = &pv["designed_runs"];
    let rn: f64 = run_b.iter().sum();
    for (k, name) in ["<=0", "1-3", "4-9", "10-19", "20-39", "40+"].iter().enumerate() {
        row(&format!("runs {name} yards"), run_b[k] / rn.max(1.0), runs["share"][name].as_f64());
    }
    for (k, name) in ["<=6", "7", ">=8"].iter().enumerate() {
        row(&format!("ypc, {name} in the box"), by_box[k][1] / by_box[k][0].max(1.0), runs["by_box"][name]["ypc"].as_f64());
        row(&format!("share of runs, {name} in box"), by_box[k][0] / rn.max(1.0), None);
    }
    if std::env::var("AFM_LONG").is_ok() {
        for (k, v) in &cov_runs {
            println!("runs vs {k:<12} share {:.3} ypc {:.2} 40+ {:.3}", v[0] / rn, v[1] / v[0], v[2] / v[0]);
        }
    }
    let fgr = &real["field_goal_pct_by_distance"];
    for (k, name) in ["under30", "30-39", "40-49", "50+"].iter().enumerate() {
        row(&format!("FG {name} (n={:.0})", fg[k][0]), fg[k][1] / fg[k][0].max(1.0), fgr[name]["pct"].as_f64());
    }
}
