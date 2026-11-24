//! Development check (section 3, topic 10): play many games between
//! synthetic teams at a level and compare the averages to real football.
//! Usage: match_check [nfl|fbs|hs] [games] [seed] [pbp]
//! With `pbp`, prints the first game's play-by-play with its whys.
//! AFM_AB_WORLD=<world file> plays a world's 32 NFL clubs instead, or at FBS
//! its FBS programs on its own season's schedule (AFM_CONF=1: conference
//! games; AFM_TEAMS=<file> keeps the teams, a world file takes a minute to
//! read). On any teams: AFM_STARTERS=<points> prints the starters against
//! the level's typical starter, AFM_TYPICAL=<spec> moves groups to him, and
//! AFM_SEASONS=<games> reads the games as seasons (thousand-yard backs and
//! receivers, the leaders, the Heisman voters' read).

extern crate afm_tools;
use afm_engine::game::stats::PlayType;
use afm_engine::game::synthetic::synthetic_league;
use afm_engine::game::{Conditions, Venue, quick_game_in};
use afm_engine::player::Level;
use rayon::prelude::*;
use serde_json::Value;

/// The engine's routes pooled with the real route types they match: the
/// route lists of 2021-2025 (both charting lists, for the coverage table)
/// and of 2023-2025 alone (one list, NGS, for the route table).
type RouteGroup = (&'static str, &'static [&'static str], &'static [&'static str]);
const ROUTE_GROUPS: [RouteGroup; 12] = [
    ("go / fade / seam", &["GO"], &["GO"]),
    ("post", &["POST"], &["POST"]),
    ("corner", &["CORNER"], &["CORNER"]),
    ("wheel", &["WHEEL"], &["WHEEL"]),
    ("dig / in / cross", &["IN/DIG", "IN", "CROSS"], &["IN/DIG"]),
    ("out (10+ air)", &["DEEP OUT"], &["DEEP OUT"]),
    // NGS charts the flat as a quick out (2.7 air yards).
    ("flat / quick out / swing", &["OUT", "QUICK OUT", "FLAT", "SWING"], &["QUICK OUT", "SWING"]),
    ("slant", &["SLANT"], &["SLANT"]),
    ("hitch / curl / sit", &["HITCH", "HITCH/CURL"], &["HITCH/CURL"]),
    ("drag", &["SHALLOW CROSS/DRAG"], &["SHALLOW CROSS/DRAG"]),
    ("screen", &["SCREEN"], &["SCREEN"]),
    ("angle", &["ANGLE", "TEXAS/ANGLE"], &["TEXAS/ANGLE"]),
];

fn route_group(route: &str, air: f64) -> Option<usize> {
    Some(match route {
        "go" | "fade" | "seam" => 0,
        "post" => 1,
        "corner" => 2,
        "wheel" => 3,
        "dig" => 4,
        "out" | "comeback" if air >= 10.0 => 5,
        "out" | "comeback" | "flat" | "swing" | "check-release" => 6,
        "slant" => 7,
        "hitch" | "curl" | "sit" => 8,
        "drag" => 9,
        "screen" | "bubble" => 10,
        "angle" => 11,
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
    let sched = conference_games(&conf, seed);
    (teams, sched)
}

/// Every conference game, home and away, in a shuffled order (so a short
/// run still spreads over every conference).
fn conference_games(conf: &[String], seed: u32) -> Vec<(usize, usize)> {
    let mut sched = Vec::new();
    for h in 0..conf.len() {
        for a in 0..conf.len() {
            if h != a && conf[h] == conf[a] && !conf[h].is_empty() && !conf[h].starts_with("Indep") {
                sched.push((h, a));
            }
        }
    }
    let mut r = afm_engine::rng::Rng::new(seed ^ 0x5EED);
    for i in (1..sched.len()).rev() {
        let j = (r.next() * (i + 1) as f64) as usize;
        sched.swap(i, j.min(i));
    }
    sched
}

/// AFM_AB_WORLD=<world file> at FBS: the world's FBS programs as its
/// season's games played them, on its own schedule's games between two of
/// them (AFM_CONF=1: conference games, home and away, instead).
/// AFM_TEAMS=<file> keeps them: read from it when it exists, else written
/// to it after the world is read (a world file takes a minute to read).
fn world_fbs(path: &str, seed: u32) -> (Vec<afm_engine::game::GameTeam>, Vec<(usize, usize)>) {
    let keep = std::env::var("AFM_TEAMS").ok();
    let cached: Option<afm_tools::ab::CollegeTeams> =
        keep.as_ref().and_then(|k| std::fs::read(k).ok()).map(|b| serde_json::from_slice(&b).expect("the teams file"));
    let (teams, conf, games) = cached.unwrap_or_else(|| {
        let t = afm_tools::ab::world_college(path, "FBS");
        if let Some(k) = &keep {
            std::fs::write(k, serde_json::to_vec(&t).unwrap()).expect("write the teams file");
        }
        t
    });
    eprintln!("{} FBS programs from the world, {} games between them", teams.len(), games.len());
    let sched =
        if std::env::var("AFM_CONF").is_ok() || games.is_empty() { conference_games(&conf, seed) } else { games };
    (teams, sched)
}

/// The NFL's fouls on scrimmage plays, per team game, 2021-25 regular
/// seasons (nflverse play-by-play, every penalty in the play text, 2,718
/// team games; EG5, `docs/design/engine-gaps.md`): (the engine's kind,
/// accepted, flagged with the declined and offsetting ones, yards an
/// accepted call). Kinds the engine folds: unnecessary roughness with
/// horse collars and taunting; illegal formation with shifts, motion,
/// offensive offside and the offense's too many men; neutral zone
/// infraction with encroachment; illegal use of hands both sides (the
/// defense 0.086, the offense 0.024); face masks both sides. Offside is
/// the live-ball foul alone. The two kickoff fouls are the 2025 rule
/// book's, on kicks (the repo's 2025 counts; no landing zone before 2024).
const REAL_FOULS: &[(&str, f64, f64, f64)] = &[
    ("offensive holding", 0.893, 1.114, 9.8),
    ("false start", 1.131, 1.132, 4.9),
    ("pass interference", 0.497, 0.561, 15.8),
    ("defensive holding", 0.322, 0.452, 4.7),
    ("illegal contact", 0.121, 0.175, 4.9),
    ("illegal use of hands", 0.110, 0.154, 6.0),
    ("offside", 0.253, 0.377, 4.8),
    ("neutral zone infraction", 0.226, 0.229, 4.8),
    ("delay of game", 0.295, 0.310, 4.9),
    ("unnecessary roughness", 0.271, 0.308, 13.1),
    ("roughing the passer", 0.181, 0.190, 13.4),
    ("ineligible man downfield", 0.106, 0.175, 4.9),
    ("offensive pass interference", 0.128, 0.159, 9.7),
    ("illegal formation", 0.224, 0.353, 4.9),
    ("face mask", 0.124, 0.134, 13.3),
    ("too many men on the field", 0.068, 0.083, 4.5),
    ("intentional grounding", 0.077, 0.082, 11.7),
    ("illegal block in the back", 0.031, 0.038, 9.9),
    ("unsportsmanlike conduct", 0.025, 0.032, 13.3),
    ("kickoff out of bounds", 0.070, 0.070, 0.0),
    ("kickoff short of the landing zone", 0.079, 0.079, 0.0),
];

/// Accepted fouls on scrimmage plays a team game and their yards (the same
/// count as `REAL_FOULS`).
const REAL_SCRIMMAGE_FOULS: (f64, f64) = (5.244, 42.6);

/// Team games in `games` games.
fn n_games(games: usize) -> f64 {
    2.0 * games as f64
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
    // Another league's rule book (X7): `cfl`, `arena`, `nfle`, `xfl2001`,
    // `xfl`, `aaf`, `usfl1983`, `usfl`, `ufl2009`, `ufl`, with NFL-level
    // players; AFM_BOOK_YEAR picks the season (else the book's sample).
    let book = args.get(1).and_then(|k| afm_engine::game::books::Book::from_key(k)).map(|b| {
        let y = std::env::var("AFM_BOOK_YEAR").ok().and_then(|y| y.parse().ok()).unwrap_or(b.sample_year());
        (b, y)
    });
    let games: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(200);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
    let pbp = args.get(4).is_some_and(|s| s == "pbp");
    // AFM_AB_WORLD=<world file> at the NFL: the world's 32 clubs as they
    // would dress today, on the same schedule as the synthetic league (as
    // `ab_check` and `fit_check` read it).
    let (mut teams, sched) = match (std::env::var("AFM_ROSTERS"), std::env::var("AFM_AB_WORLD")) {
        (Ok(path), _) => world_rosters(&path, level, seed),
        (_, Ok(path)) if level == Level::FBS => world_fbs(&path, seed),
        (_, Ok(path)) if level == Level::NFL => {
            let w = afm_tools::ab::world_nfl(&path);
            assert_eq!(w.len(), 32, "a world with 32 NFL clubs");
            (w.into_iter().map(|s| s.team).collect(), Vec::new())
        }
        _ => (synthetic_league(level, 32, seed), Vec::new()),
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
    // AFM_STARTERS=1.5: the teams' starters (each depth chart's) against the
    // level's typical starter, every rating that many points or more apart,
    // and their weight (audit item 1: are a world's rosters the test
    // league's?).
    if let Some(gap) = std::env::var("AFM_STARTERS").ok().map(|v| v.parse::<f64>().unwrap_or(1.5)) {
        use afm_engine::attributes::Attr;
        use afm_engine::game::roster::{depth_chart, starters};
        use afm_engine::game::synthetic::{typical_body, typical_starter};
        let charts: Vec<_> = teams.iter().map(|t| depth_chart(t, 99.0)).collect();
        for pos in afm_engine::positions::Position::ALL {
            let men: Vec<_> = teams
                .iter()
                .zip(&charts)
                .flat_map(|(t, d)| d.list(pos).iter().take(starters(pos)).map(move |&i| &t.players[i]))
                .collect();
            if men.is_empty() {
                continue;
            }
            let n = men.len() as f64;
            let wt = men.iter().map(|p| p.weight).sum::<f64>() / n;
            let ovr = men.iter().map(|p| afm_engine::positions::overall_for(pos, &p.attrs)).sum::<f64>() / n;
            let mut typ = afm_engine::attributes::Attributes::default();
            for a in Attr::ALL {
                typ.set(a, typical_starter(level, pos, a));
            }
            let mut line = format!(
                "starters {pos:?} n {} overall {ovr:.1} ({:.1}) weight {wt:.0} ({:.0})",
                men.len(),
                afm_engine::positions::overall_for(pos, &typ),
                typical_body(level, pos).1
            );
            for a in Attr::ALL {
                let m = men.iter().map(|p| p.a(a)).sum::<f64>() / n;
                let t = typical_starter(level, pos, a);
                if (m - t).abs() >= gap {
                    line += &format!(" | {a:?} {m:.1} ({:+.1})", m - t);
                }
            }
            println!("{line}");
        }
    }
    // AFM_TYPICAL="OT,IOL;RB:Speed,Acceleration;LB:weight": every player at
    // those positions moved by the gap between the teams' starters there and
    // the level's typical starter, in those ratings (all of them when none
    // is named; `weight` his weight), so his group's starters average the
    // typical starter's (which part of a world's rosters plays off the test
    // league's?).
    if let Ok(spec) = std::env::var("AFM_TYPICAL") {
        use afm_engine::attributes::Attr;
        use afm_engine::game::roster::{depth_chart, starters};
        use afm_engine::game::synthetic::{typical_body, typical_starter};
        let charts: Vec<_> = teams.iter().map(|t| depth_chart(t, 99.0)).collect();
        for part in spec.split(';').filter(|p| !p.is_empty()) {
            let (who, what) = part.split_once(':').unwrap_or((part, ""));
            let names: Vec<&str> = what.split(',').filter(|w| !w.is_empty()).collect();
            let all = afm_engine::positions::Position::ALL;
            for pos in all.into_iter().filter(|p| who == "ALL" || who.split(',').any(|n| format!("{p:?}") == n)) {
                let men: Vec<_> = teams
                    .iter()
                    .zip(&charts)
                    .flat_map(|(t, d)| d.list(pos).iter().take(starters(pos)).map(move |&i| &t.players[i]))
                    .collect();
                if men.is_empty() {
                    continue;
                }
                let n = men.len() as f64;
                let attrs: Vec<Attr> = Attr::ALL
                    .into_iter()
                    .filter(|a| names.is_empty() || names.contains(&format!("{a:?}").as_str()))
                    .collect();
                let gap: Vec<f64> = attrs
                    .iter()
                    .map(|&a| men.iter().map(|p| p.a(a)).sum::<f64>() / n - typical_starter(level, pos, a))
                    .collect();
                let wgap = if names.is_empty() || names.contains(&"weight") {
                    men.iter().map(|p| p.weight).sum::<f64>() / n - typical_body(level, pos).1
                } else {
                    0.0
                };
                for t in teams.iter_mut() {
                    for p in t.players.iter_mut().filter(|p| p.position == pos) {
                        for (&a, g) in attrs.iter().zip(&gap) {
                            p.attrs.set(a, (p.attrs.get(a) - g).clamp(1.0, 99.0));
                        }
                        p.weight -= wgap;
                        p.spot_r = afm_engine::spots::spot_ratings(&p.attrs, p.weight, p.height);
                    }
                }
                eprintln!("typical {pos:?}: {} ratings moved, weight {:+.1}", attrs.len(), -wgap);
            }
        }
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
    // AFM_BURNER=12 makes each team's second receiver that many points
    // faster (speed and burst only), the speed update's S10 proof: does a
    // burner raise his teammates' short catches and yards after the catch?
    // Run it beside AFM_BURNER=0 on the same seed.
    let mut burners: std::collections::HashSet<u32> = std::collections::HashSet::new();
    if let Some(add) = std::env::var("AFM_BURNER").ok().and_then(|v| v.parse::<f64>().ok()) {
        use afm_engine::positions::Position;
        for t in teams.iter_mut() {
            let mut wrs: Vec<usize> = (0..t.players.len()).filter(|&i| t.players[i].position == Position::WR).collect();
            wrs.sort_by(|&a, &b| {
                t.players[b].overall_at(Position::WR).total_cmp(&t.players[a].overall_at(Position::WR))
            });
            if let Some(&i) = wrs.get(1) {
                let p = &mut t.players[i];
                for a in [afm_engine::attributes::Attr::Speed, afm_engine::attributes::Attr::Acceleration] {
                    p.attrs.set(a, (p.attrs.get(a) + add).min(99.0));
                }
                burners.insert(p.id);
            }
        }
    }
    // AFM_TRACE=1 prints every agent's state (use with one game).
    if std::env::var("AFM_TRACE").is_ok() {
        afm_engine::game::sim::TRACE.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    // College and NFL coordinators draft a call sheet at kickoff (the
    // game's rule). AFM_SHEET=every has high school staffs draft too,
    // AFM_SHEET=none plays with no sheets, AFM_SHEET=home with the home
    // team's only; the home-side block below sets each against the others.
    let sheet = std::env::var("AFM_SHEET").ok();
    let mode = match sheet.as_deref() {
        Some("none") => 0,
        Some("home") => 2,
        Some("every") => 1,
        _ => 3,
    };
    afm_engine::game::sheet::KICKOFF.store(mode, std::sync::atomic::Ordering::Relaxed);
    let mode_name = ["none", "every team", "home only", "college and NFL teams"][mode as usize];
    // What a kickoff draft costs: one team's sheet from the other's film,
    // on one core, under the rule book the games are played on.
    {
        use afm_engine::world::plans::{DraftIn, film_tells, kickoff_card};
        let rules = match book {
            Some((b, y)) => afm_engine::game::Rules::for_book(b, y),
            None => afm_engine::game::Rules::for_level(level),
        };
        let n = 64;
        let t = std::time::Instant::now();
        let mut boxes = 0;
        let (mut plays, mut dboxes, mut dplays) = (0, 0, 0);
        for k in 0..n {
            let (h, a) = pair(k);
            let inp = DraftIn {
                film: teams[a].film.clone(),
                level,
                two_from: rules.two_point_from,
                qb_run: 0.3,
                seed: seed ^ k as u32,
                favs: Vec::new(),
                tells: film_tells(&teams[h].film),
                designs: Vec::new(),
                players: rules.code.players,
            };
            let card = kickoff_card(&teams[h], &inp);
            boxes += card.sheet.len();
            plays += card.sheet.iter().map(|b| b.1.len()).sum::<usize>();
            dboxes += card.dsheet.len();
            dplays += card.dsheet.iter().map(|b| b.1.len()).sum::<usize>();
        }
        let ms = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
        println!(
            "kickoff sheet: {ms:.2} ms to draft on one core ({:.1} offense boxes a sheet, {:.1} plays a box; {:.1} defense boxes, {:.1} calls a box)",
            boxes as f64 / n as f64,
            plays as f64 / boxes.max(1) as f64,
            dboxes as f64 / n as f64,
            dplays as f64 / dboxes.max(1) as f64
        );
    }
    // AFM_FAKE=1 gives every team the fake punt "if the look shows",
    // AFM_FAKE=2 "called" (the first chance); the fakes are counted below.
    if let Some(f) = std::env::var("AFM_FAKE").ok().and_then(|v| v.parse::<u8>().ok()) {
        for t in teams.iter_mut() {
            t.prep.teams.fake = f;
        }
    }
    let t0 = std::time::Instant::now();
    let results: Vec<_> = (0..games)
        .into_par_iter()
        .map(|g| {
            let (h, a) = pair(g);
            let home = &teams[h];
            // An average road trip: the crowd of an average FBS conference
            // or NFL game (a 50,000-70,000 stadium) and about 500 miles and
            // half a time zone of travel for the visitors.
            // AFM_YEAR sets the season's rules (else the default venue's);
            // AFM_WIND a steady wind in mph and AFM_RAIN=1 rain, for the
            // weather splits.
            let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<f64>().ok());
            let mut venue = Venue {
                cond: Conditions { crowd: 0.65, ..Conditions::default() },
                wear: [0.0, 0.012],
                ..Venue::default()
            };
            if let Some(y) = env("AFM_YEAR") {
                venue.year = y as i32;
            }
            if env("AFM_RAIN").is_some() {
                venue.cond.rain = true;
            }
            if let Some(w) = env("AFM_WIND") {
                venue.cond.wind = w;
                venue.cond.wind_along = ((g as f64 * 0.618_034) % 1.0) * 2.0 - 1.0;
            }
            // AFM_TEMP the kickoff temperature in F (engine gaps item 14's
            // cold run; the default is 60).
            if let Some(t) = env("AFM_TEMP") {
                venue.cond.temperature = t;
            }
            let seed = seed as u64 * 1_000_003 + g as u64;
            match book {
                Some((b, y)) => afm_engine::game::quick_game_book(&teams[h], &teams[a], b, y, seed, venue, false),
                None => quick_game_in(home, &teams[a], level, seed, Default::default(), venue),
            }
        })
        .collect();
    let secs = t0.elapsed().as_secs_f64();
    // AFM_SEASONS=13: each team's games in seasons of that many, in order,
    // and the season lines they give, a year being one season of every
    // team (audit items 1, 5 and 8: the world's thousand-yard backs and
    // receivers, its rushing leader).
    if let Some(len) = std::env::var("AFM_SEASONS").ok().and_then(|v| v.parse::<usize>().ok()) {
        use afm_engine::positions::Position as P;
        use std::collections::HashMap;
        let pos: HashMap<u32, P> = teams.iter().flat_map(|t| t.players.iter().map(|p| (p.id, p.position))).collect();
        // Each team's games this season, its seasons done and its lines:
        // [rushing yards, carries, receiving yards, catches, passing yards,
        // passing touchdowns, interceptions thrown, rushing and receiving
        // touchdowns, tackles, sacks, tackles for loss, interceptions].
        type Lines = HashMap<u32, [f64; 13]>;
        let mut cur: HashMap<usize, (usize, usize, Lines)> = HashMap::new();
        let mut done: Vec<(usize, Lines)> = Vec::new();
        for (g, r) in results.iter().enumerate() {
            let (h, a) = pair(g);
            for (k, ti) in [h, a].into_iter().enumerate() {
                let e = cur.entry(ti).or_default();
                for (id, l) in r.stats.players[k].iter() {
                    let v = e.2.entry(*id).or_default();
                    v[0] += l.rush_yds;
                    v[1] += l.rush as f64;
                    v[2] += l.rec_yds;
                    v[3] += l.rec as f64;
                    v[4] += l.pass_yds;
                    v[5] += l.pass_td as f64;
                    v[6] += l.int as f64;
                    v[7] += l.rush_td as f64;
                    v[8] += l.rec_td as f64;
                    v[9] += (l.tackles + l.assists) as f64;
                    v[10] += l.sacks;
                    v[11] += l.tfl;
                    v[12] += l.ints as f64;
                }
                e.0 += 1;
                if e.0 == len {
                    done.push((e.1, std::mem::take(&mut e.2)));
                    e.0 = 0;
                    e.1 += 1;
                }
            }
        }
        let years = done.iter().map(|d| d.0 + 1).max().unwrap_or(0);
        // Only years every team finished.
        let full: Vec<usize> =
            (0..years).filter(|&y| done.iter().filter(|d| d.0 == y).count() == teams.len()).collect();
        let lines = |y: usize| done.iter().filter(move |d| d.0 == y).flat_map(|d| d.1.iter());
        let ny = full.len().max(1) as f64;
        let count = |f: &dyn Fn(&u32, &[f64; 13]) -> bool| {
            full.iter().map(|&y| lines(y).filter(|(id, v)| f(id, v)).count()).sum::<usize>() as f64 / ny
        };
        let is = |id: &u32, p: P| pos.get(id) == Some(&p);
        let lead_rush = full.iter().map(|&y| lines(y).map(|(_, v)| v[0]).fold(0.0, f64::max)).sum::<f64>() / ny;
        let lead_rec = full.iter().map(|&y| lines(y).map(|(_, v)| v[2]).fold(0.0, f64::max)).sum::<f64>() / ny;
        let pct = |mut v: Vec<f64>, q: f64| {
            v.sort_by(|a, b| a.total_cmp(b));
            if v.is_empty() { 0.0 } else { v[((v.len() - 1) as f64 * q).round() as usize] }
        };
        let backs: Vec<f64> = full
            .iter()
            .flat_map(|&y| lines(y).filter(|(id, v)| is(id, P::RB) && v[1] >= 100.0).map(|(_, v)| v[0] / v[1]))
            .collect();
        let recv: Vec<f64> = full
            .iter()
            .flat_map(|&y| {
                lines(y).filter(|(id, v)| (is(id, P::WR) || is(id, P::TE)) && v[3] >= 25.0).map(|(_, v)| v[2])
            })
            .collect();
        println!(
            "seasons of {len} games: {} years of {} teams | 1,000-yard rushers a year {:.1} (backs {:.1}), 2,000-yard {:.2}, the rushing leader {:.0} | backs with 100+ carries: median {:.2} a carry, 10th percentile {:.2} | 1,000-yard receivers a year {:.1}; with 25+ catches 90th percentile {:.0}, 99th {:.0}; the receiving leader {:.0}",
            full.len(),
            teams.len(),
            count(&|_, v| v[0] >= 1000.0),
            count(&|id, v| is(id, P::RB) && v[0] >= 1000.0),
            count(&|_, v| v[0] >= 2000.0),
            lead_rush,
            pct(backs.clone(), 0.5),
            pct(backs, 0.1),
            count(&|_, v| v[2] >= 1000.0),
            pct(recv.clone(), 0.9),
            pct(recv, 0.99),
            lead_rec,
        );
        // The Heisman voters' read of each line alone (item 5; no team,
        // hype or region): production as `world::college::production` reads
        // it, each kind's against its own top ten, weighed as
        // `heisman_kind` weighs it; the kinds of each year's top read and
        // top five.
        let kind = |p: P| match p {
            P::QB => Some((0, 1.0)),
            P::RB => Some((1, 0.84)),
            P::WR | P::TE => Some((2, 0.78)),
            P::K | P::P | P::LS => None,
            _ => Some((3, 0.75)),
        };
        let (mut top, mut five) = ([0.0f64; 4], [0.0f64; 4]);
        for &y in &full {
            let prods: Vec<(usize, f64, f64)> = lines(y)
                .filter_map(|(id, v)| {
                    let (k, wt) = kind(*pos.get(id)?)?;
                    let scrim = (v[0] + v[2]) / 10.0 + 6.0 * (v[7] + v[8]);
                    let pass = v[4] / 20.0 + 4.0 * v[5] - 3.0 * v[6];
                    let def = v[9] + 6.0 * v[10] + 4.0 * v[11] + 10.0 * v[12];
                    Some((k, wt, if k == 3 { 0.6 * def } else { pass + scrim }))
                })
                .collect();
            let bar: Vec<f64> = (0..4)
                .map(|k| {
                    let mut v: Vec<f64> = prods.iter().filter(|x| x.0 == k).map(|x| x.2).collect();
                    v.sort_by(|a, b| b.total_cmp(a));
                    let t = &v[..v.len().min(10)];
                    (t.iter().sum::<f64>() / t.len().max(1) as f64).max(1.0)
                })
                .collect();
            let mut reads: Vec<(usize, f64)> = prods.iter().map(|x| (x.0, x.1 * x.2 / bar[x.0])).collect();
            reads.sort_by(|a, b| b.1.total_cmp(&a.1));
            for (i, r) in reads.iter().take(5).enumerate() {
                five[r.0] += 1.0 / (5.0 * ny);
                if i == 0 {
                    top[r.0] += 1.0 / ny;
                }
            }
        }
        println!(
            "the Heisman voters' read alone: the top read QB {:.0}%, back {:.0}%, receiver {:.0}%, defender {:.0}% | the top five QB {:.0}%, back {:.0}%, receiver {:.0}%, defender {:.0}%",
            100.0 * top[0],
            100.0 * top[1],
            100.0 * top[2],
            100.0 * top[3],
            100.0 * five[0],
            100.0 * five[1],
            100.0 * five[2],
            100.0 * five[3],
        );
    }
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
        // The line's men by rank (item 29): each rank's share of his side's
        // snaps, ranks 1-4, and the men on 10% or more.
        let mut ranks = [[0.0f64; 5]; 2];
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
                    if let Some(r) = match pos {
                        P::DT => Some(0),
                        P::EDGE => Some(1),
                        _ => None,
                    } {
                        for (k, x) in v.iter().take(4).enumerate() {
                            ranks[r][k] += x / def;
                        }
                        ranks[r][4] += v.iter().filter(|&&x| x / def >= 0.1).count() as f64;
                    }
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
            // Real: nflverse snap counts 2021-25, a team game, PFR labels
            // (engine_targets.json, snap_shares): interior is DT and NT, so
            // a 3-4 end listed DE counts with the edges.
            for (r, name, real) in
                [(0, "DT", "0.68 0.50 0.31 0.16, 3.47 on 10%+"), (1, "EDGE", "0.67 0.45 0.26 0.13, 3.21 on 10%+")]
            {
                let x = ranks[r].map(|v| v / nt);
                println!(
                    "  {name} by rank  {:.2} {:.2} {:.2} {:.2}, {:.2} on 10%+  (real {real})",
                    x[0], x[1], x[2], x[3], x[4]
                );
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
        // AFM_PBP_GAMES=n prints the first n games' play-by-play, each under
        // its own "Game" line (the scheme sequencing research reads them).
        let shown: usize = std::env::var("AFM_PBP_GAMES").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
        for (g, r) in results.iter().take(shown.max(1)).enumerate() {
            if shown > 1 {
                println!("Game {g}");
            }
            for p in &r.stats.plays {
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
            println!("Final {:?}", r.stats.score);
        }
    }
    let n = (games * 2) as f64;
    let mut m = std::collections::BTreeMap::<&str, f64>::new();
    let mut fg = [[0.0f64; 2]; 4];
    let mut ay = [[0.0f64; 6]; 4];
    // The burner proof: [his teammates' catches under 10 air yards, their
    // yards after those catches, his own targets, his catches, his yards,
    // two-high snaps, coverage snaps].
    let mut bp = [0.0f64; 7];
    let mut by_cov = [[0.0f64; 3]; 2];
    let pos_of: std::collections::HashMap<u32, afm_engine::positions::Position> =
        teams.iter().flat_map(|t| t.players.iter().map(|p| (p.id, p.position))).collect();
    let mut tgt_pos = [0.0f64; 4];
    // First tackles on runs by line, linebackers and backs: [runs, yards].
    let mut first_tackle = [[0.0f64; 2]; 3];
    // Seconds between snaps: [running, stopped] x [normal, late, two-minute] x [seconds, plays].
    let mut between = [[[0.0f64; 2]; 3]; 2];
    // Stopped-clock snaps by why the clock stopped: [incomplete, out of
    // bounds, penalty, touchdown] x [normal, late, two-minute] x [seconds, plays].
    let mut stop_why = [[[0.0f64; 2]; 3]; 4];
    // Designed runs by where they crossed the line (outside the tackles,
    // the real "end" gap, against inside) and by the call: [runs, yards].
    let mut run_dir = [[0.0f64; 2]; 4];
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
        // Fourth-quarter passes and runs by the offense's lead after three
        // quarters, as the world log keys them (`garbage_time.json`).
        if let Some(q4) =
            s.plays.iter().find(|p| p.quarter >= 4).map(|p| [p.score[0] - p.score[1], p.score[1] - p.score[0]])
        {
            for p in s.plays.iter().filter(|p| p.quarter == 4) {
                let pass = matches!(p.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble);
                if !pass && p.kind != PlayType::Run {
                    continue;
                }
                let (n, k) = match q4[p.offense.min(1)] {
                    l if l >= 28 => ("q4_lead28", "q4_lead28_pass"),
                    l if (1..=14).contains(&l) => ("q4_lead14", "q4_lead14_pass"),
                    _ => continue,
                };
                *m.entry(n).or_default() += 1.0;
                *m.entry(k).or_default() += pass as u8 as f64;
            }
        }
        // Home and road offenses (the home side is team 0).
        for p in s.plays.iter().filter(|p| !p.nullified) {
            let home = p.offense == 0;
            let key = |h: &'static str, r: &'static str| if home { h } else { r };
            match p.kind {
                PlayType::Pass => {
                    *m.entry(key("ha_att_h", "ha_att_r")).or_default() += 1.0;
                    *m.entry(key("ha_cmp_h", "ha_cmp_r")).or_default() += p.complete as u8 as f64;
                    *m.entry(key("ha_pyds_h", "ha_pyds_r")).or_default() += if p.complete { p.yards } else { 0.0 };
                }
                PlayType::Sack => *m.entry(key("ha_sack_h", "ha_sack_r")).or_default() += 1.0,
                PlayType::Run => {
                    *m.entry(key("ha_car_h", "ha_car_r")).or_default() += 1.0;
                    *m.entry(key("ha_ryds_h", "ha_ryds_r")).or_default() += p.yards;
                }
                _ => {}
            }
        }
        // Fourth downs: the real go rate at the engine's own fourth downs,
        // so a gap in tries can be told apart from a gap in the downs faced.
        for p in s.plays.iter().filter(|p| p.down == 4 && !p.nullified) {
            let go = matches!(p.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble);
            if !go && !matches!(p.kind, PlayType::Punt | PlayType::FieldGoal) {
                continue;
            }
            let o = p.offense.min(1);
            let diff = p.score[o] - p.score[1 - o];
            *m.entry("fourth_n").or_default() += 1.0;
            *m.entry("fourth_n_go").or_default() += go as u8 as f64;
            *m.entry("fourth_table").or_default() +=
                afm_engine::game::tables::real_fourth_go_rate(p.togo, p.yardline_100, diff, level != Level::NFL);
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
            if stopped {
                let why = if a.touchdown {
                    3
                } else if a.penalties.iter().any(|x| x.4) {
                    2
                } else if incomplete {
                    0
                } else {
                    1
                };
                let e = &mut stop_why[why][tb];
                e[0] += gap;
                e[1] += 1.0;
            }
        }
        if !burners.is_empty() {
            for p in s.plays.iter().filter(|p| p.kind == PlayType::Pass && !p.nullified && !p.throw_away) {
                let his = p.target.is_some_and(|t| burners.contains(&t));
                if his {
                    bp[2] += 1.0;
                    bp[3] += p.complete as u8 as f64;
                    bp[4] += if p.complete { p.yards } else { 0.0 };
                } else if p.complete && p.air_yards < 10.0 {
                    bp[0] += 1.0;
                    bp[1] += p.yac;
                }
                let d = p.defense_call.to_lowercase();
                bp[5] += ["cover 2", "tampa", "cover 4", "cover 6", "2-man"].iter().any(|c| d.contains(c)) as u8 as f64;
                bp[6] += 1.0;
            }
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
            let kick = matches!(
                p.kind,
                PlayType::Kickoff | PlayType::Punt | PlayType::FieldGoal | PlayType::ExtraPoint | PlayType::TwoPoint
            );
            for &(kind, _, _, yds, accepted) in &p.penalties {
                if accepted {
                    *m.entry(kind.name()).or_default() += 1.0;
                    if !kick {
                        *m.entry("pen_scrim").or_default() += 1.0;
                        *m.entry("pen_scrim_yds").or_default() += yds.abs();
                    }
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
            // Play action (scheme-fit item 3), as FTN charts it: dropbacks
            // with screens and RPOs left out, the boot on its own.
            if !p.nullified
                && matches!(p.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble)
                && !p.call.contains("screen")
                && !p.call.contains("RPO")
            {
                let keys: [&'static str; 9] = if p.play_action {
                    ["pa_db", "pa_epa", "pa_sack", "pa_prs", "pa_scr", "pa_att", "pa_ttt", "pa_pyds", "pa_air"]
                } else {
                    ["npa_db", "npa_epa", "npa_sack", "npa_prs", "npa_scr", "npa_att", "npa_ttt", "npa_pyds", "npa_air"]
                };
                let att = p.kind == PlayType::Pass && !p.throw_away;
                let vals = [
                    1.0,
                    p.epa,
                    (p.kind == PlayType::Sack) as u8 as f64,
                    p.pressured as u8 as f64,
                    (p.kind == PlayType::Scramble) as u8 as f64,
                    att as u8 as f64,
                    if att { p.time_to_throw } else { 0.0 },
                    if att && p.complete { p.yards } else { 0.0 },
                    if att { p.air_yards } else { 0.0 },
                ];
                for (k, x) in keys.into_iter().zip(vals) {
                    *m.entry(k).or_default() += x;
                }
                // By where the quarterback lines up (real: under center
                // 14.1% of dropbacks with 79.8% play action, the pistol 3.2%
                // with 40.7%, the shotgun 81.4% with 10.4%).
                let (bk, bpa): (&'static str, &'static str) = if p.call.contains(" under center ") {
                    ("db_uc", "pa_uc")
                } else if p.call.contains(" pistol ") {
                    ("db_pi", "pa_pi")
                } else {
                    ("db_gun", "pa_gun")
                };
                *m.entry(bk).or_default() += 1.0;
                *m.entry(bpa).or_default() += p.play_action as u8 as f64;
                // By down (real 1st 36.6%, 2nd 20.8%, 3rd and 4th 4.2%).
                let (dk, dpa): (&'static str, &'static str) = match p.down {
                    1 => ("db_d1", "pa_d1"),
                    2 => ("db_d2", "pa_d2"),
                    _ => ("db_d3", "pa_d3"),
                };
                *m.entry(dk).or_default() += 1.0;
                *m.entry(dpa).or_default() += p.play_action as u8 as f64;
                // Throw depth on play action (real 2024: behind the line
                // 14.8%, 0-9 44.8%, 10-19 26.5%, 20+ 13.9%).
                if p.play_action && att {
                    let b = if p.air_yards < 0.0 {
                        "pa_ay0"
                    } else if p.air_yards < 10.0 {
                        "pa_ay1"
                    } else if p.air_yards < 20.0 {
                        "pa_ay2"
                    } else {
                        "pa_ay3"
                    };
                    *m.entry(b).or_default() += 1.0;
                }
                // Each play action concept on its own.
                for (name, k) in [
                    ("play-action shot", ["pas_db", "pas_epa", "pas_sack", "pas_prs", "pas_scr"]),
                    ("play-action dig", ["pad_db", "pad_epa", "pad_sack", "pad_prs", "pad_scr"]),
                    ("play-action boot", ["pab_db", "pab_epa", "pab_sack", "pab_prs", "pab_scr"]),
                ] {
                    if p.play_action && p.call.contains(name) {
                        let v = [
                            1.0,
                            p.epa,
                            (p.kind == PlayType::Sack) as u8 as f64,
                            p.pressured as u8 as f64,
                            (p.kind == PlayType::Scramble) as u8 as f64,
                        ];
                        for (k, x) in k.into_iter().zip(v) {
                            *m.entry(k).or_default() += x;
                        }
                    }
                }
                if p.call.contains("boot") {
                    *m.entry("boot_db").or_default() += 1.0;
                    *m.entry("boot_sack").or_default() += (p.kind == PlayType::Sack) as u8 as f64;
                    *m.entry("boot_epa").or_default() += p.epa;
                }
            }
            // Where the quarterback lines up for each kind of play (FTN,
            // NFL 2022-25: runs and play action 58% under center, straight
            // drops 4%, screens 19%), and screens off a fake (27%).
            if !p.nullified && !p.call.contains("RPO") {
                let kind = match p.kind {
                    PlayType::Run => Some(0),
                    PlayType::Pass | PlayType::Sack | PlayType::Scramble if p.call.contains("screen") => Some(3),
                    PlayType::Pass | PlayType::Sack | PlayType::Scramble => Some(if p.play_action { 1 } else { 2 }),
                    _ => None,
                };
                if let Some(k) = kind {
                    const SPOT: [[&str; 4]; 4] = [
                        ["sp_run", "sp_run_u", "sp_run_p", "sp_run_s"],
                        ["sp_pa", "sp_pa_u", "sp_pa_p", "sp_pa_s"],
                        ["sp_drop", "sp_drop_u", "sp_drop_p", "sp_drop_s"],
                        ["sp_scr", "sp_scr_u", "sp_scr_p", "sp_scr_s"],
                    ];
                    let at = if p.call.contains(" under center ") {
                        1
                    } else if p.call.contains(" pistol ") {
                        2
                    } else {
                        3
                    };
                    *m.entry(SPOT[k][0]).or_default() += 1.0;
                    *m.entry(SPOT[k][at]).or_default() += 1.0;
                    if k == 3 {
                        *m.entry("sp_scr_pa").or_default() += p.play_action as u8 as f64;
                    }
                }
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
                            // How far after the catch, rounded to the yard
                            // as real spots are (lane SF9b).
                            let y = p.yac.round();
                            let k = if y <= 0.0 {
                                "yac_0"
                            } else if y < 5.0 {
                                "yac_1"
                            } else if y < 10.0 {
                                "yac_5"
                            } else if y < 20.0 {
                                "yac_10"
                            } else {
                                "yac_20"
                            };
                            *m.entry(k).or_default() += 1.0;
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
                        let gap = afm_engine::world::tally::expect::run_gap(&p.call);
                        // A run stopped behind the line counts where it was
                        // called, as a charter logs it (nflverse's run gaps
                        // cover every designed run, losses too).
                        let out = match p.tend.cross {
                            Some((dy, _)) => dy.abs() > 4.0,
                            None => matches!(gap, 0 | 4),
                        };
                        let e = &mut run_dir[out as usize];
                        e[0] += 1.0;
                        e[1] += y;
                        let e = &mut run_dir[2 + matches!(gap, 0 | 4) as usize];
                        e[0] += 1.0;
                        e[1] += y;
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
                    if !p.blocked {
                        *m.entry("punt_kicked").or_default() += 1.0;
                        let k = if p.fair_catch {
                            "punt_fc"
                        } else if p.touchback {
                            "punt_tb"
                        } else if p.returner.is_some() {
                            "punt_ret"
                        } else {
                            "punt_downed"
                        };
                        *m.entry(k).or_default() += 1.0;
                    }
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
    if let Some((b, y)) = book {
        println!("rule book {} {y}: real points a team game {:.1}", b.key(), b.real_points().unwrap_or(0.0));
        row("points (league)", mean, b.real_points());
        let singles = results.iter().flat_map(|r| r.stats.plays.iter()).filter(|p| p.single).count();
        row("singles (rouge)", singles as f64 / n_games(games), None);
    }
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
    {
        let mode = mode_name;
        // The home side against the road side, with sheets as `AFM_SHEET`
        // sets them: the offense [points, pass yards, rush yards, snaps,
        // EPA, runs, sacks taken, interceptions thrown] and the defense
        // [points allowed, snaps, EPA allowed, man, blitz, dropbacks faced,
        // pressures, sacks, interceptions].
        let (mut o, mut d) = ([0.0f64; 8], [0.0f64; 9]);
        for r in &results {
            let s = &r.stats;
            let (ht, at) = (&s.team[0], &s.team[1]);
            o[0] += ht.points as f64;
            o[1] += ht.pass_yards;
            o[2] += ht.rush_yards;
            o[6] += ht.sacks as f64;
            o[7] += ht.ints as f64;
            d[0] += at.points as f64;
            d[5] += at.dropbacks as f64;
            d[6] += at.pressures as f64;
            d[7] += at.sacks as f64;
            d[8] += at.ints as f64;
            for p in s
                .plays
                .iter()
                .filter(|p| matches!(p.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble))
            {
                if p.offense == 0 {
                    o[3] += 1.0;
                    o[4] += p.epa;
                    o[5] += (p.kind == PlayType::Run) as u8 as f64;
                } else {
                    d[1] += 1.0;
                    d[2] += p.epa;
                    d[3] += p.coverage_man as u8 as f64;
                    d[4] += p.blitz as u8 as f64;
                }
            }
        }
        let gm = games as f64;
        println!(
            "home side, sheet {mode}: offense points {:.2}, pass yards {:.1}, rush yards {:.1}, run share {:.3}, EPA a snap {:.3}, sacks taken {:.2}, interceptions {:.3}",
            o[0] / gm,
            o[1] / gm,
            o[2] / gm,
            o[5] / o[3].max(1.0),
            o[4] / o[3].max(1.0),
            o[6] / gm,
            o[7] / gm
        );
        println!(
            "home side, sheet {mode}: defense points allowed {:.2}, man share {:.3}, blitz share {:.3}, pressure rate {:.3}, sacks {:.2}, interceptions {:.3}, EPA a snap allowed {:.3}",
            d[0] / gm,
            d[3] / d[1].max(1.0),
            d[4] / d[1].max(1.0),
            d[6] / d[5].max(1.0),
            d[7] / gm,
            d[8] / gm,
            d[2] / d[1].max(1.0)
        );
        // Fake punts (AFM_FAKE): how many, with the return team's look or
        // without it, and how many made the line to gain.
        let mut fk = [[0.0f64; 3]; 2];
        for r in &results {
            for p in r.stats.plays.iter().filter(|p| p.call_why.starts_with("a fake punt") && p.kind == PlayType::Run) {
                let k = p.look as usize;
                fk[k][0] += 1.0;
                fk[k][1] += (p.first_down || p.touchdown) as u8 as f64;
                fk[k][2] += p.yards;
            }
        }
        let f: Vec<u64> =
            afm_engine::game::sheet::FOUND.iter().map(|x| x.load(std::sync::atomic::Ordering::Relaxed)).collect();
        let share = |a: &[u64]| {
            let n = a.iter().sum::<u64>().max(1) as f64;
            a.iter().map(|&x| format!("{:.3}", x as f64 / n)).collect::<Vec<_>>().join(" / ")
        };
        println!("box calls: offense own / second thought / closest {}, defense {}", share(&f[..3]), share(&f[3..]));
        for (k, name) in [(1, "with the look"), (0, "without it")] {
            if fk[k][0] > 0.0 {
                println!(
                    "fake punts {name}: {:.0}, converted {:.3}, yards {:.2}",
                    fk[k][0],
                    fk[k][1] / fk[k][0],
                    fk[k][2] / fk[k][0]
                );
            }
        }
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
        // By why it stopped (nflverse 2021-25, the same rows as
        // nfl_situations.py: penalty first, then incomplete, then out of
        // bounds; touchdowns before all).
        let real_why = [[5.0, 5.0, 4.8], [33.8, 15.9, 6.3], [21.5, 12.6, 8.3], [5.9, 5.85, 5.2]];
        for (wi, wname) in ["incomplete", "out of bounds", "penalty", "touchdown"].iter().enumerate() {
            for (ti, tname) in ["normal", "late", "two_minute"].iter().enumerate() {
                let e = stop_why[wi][ti];
                row(&format!("  stopped {tname}, {wname}"), e[0] / e[1].max(1.0), Some(real_why[wi][ti]));
                row(&format!("  share of stopped {tname}, {wname}"), e[1] / between[1][ti][1].max(1.0), None);
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
    // The engine logs no foul on a punt or kickoff return yet (bug 403), so
    // the kinds read against nflverse's own on scrimmage plays (2021-25,
    // `REAL_FOULS`); the kickoff fouls against the 2025 rule book's kicks.
    let nfl = level == Level::NFL;
    row("  on scrimmage plays", g("pen_scrim") / n, nfl.then_some(REAL_SCRIMMAGE_FOULS.0));
    row("  their yards", g("pen_scrim_yds") / n, nfl.then_some(REAL_SCRIMMAGE_FOULS.1));
    for &(k, real, _, _) in REAL_FOULS {
        row(&format!("  {k}"), g(k) / n, nfl.then_some(real));
    }
    // Targeting (coach fire): college and high school only, the NFL calls
    // those hits unnecessary roughness. FBS 0.031-0.043 a team game in
    // 2022-2025 (cfbfastR play-by-play); no high school count.
    row("  targeting", g("targeting") / n, matches!(level, Level::FBS).then_some(0.037));
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
    // Home against road offenses, real nflverse 2021-2025 regular season
    // without neutral sites: the road offense loses most of its edge
    // through the air (home completions 65.2%, road 63.8%; 7.15 and 6.95
    // yards an attempt; sacks 6.63% and 6.87% of dropbacks; 4.52 and 4.48
    // yards a carry).
    for (home, real) in [(true, [0.6523, 7.151, 0.0663, 4.521]), (false, [0.6383, 6.951, 0.0687, 4.484])] {
        let k = |h: &str, r: &str| g(if home { h } else { r });
        let name = if home { "home" } else { "road" };
        let (att, sacks, car) = (k("ha_att_h", "ha_att_r"), k("ha_sack_h", "ha_sack_r"), k("ha_car_h", "ha_car_r"));
        row(&format!("{name} offense completion"), k("ha_cmp_h", "ha_cmp_r") / att, Some(real[0]));
        row(&format!("{name} offense yards per att"), k("ha_pyds_h", "ha_pyds_r") / att, Some(real[1]));
        row(&format!("{name} offense sack rate"), sacks / (att + sacks), Some(real[2]));
        row(&format!("{name} offense yards per carry"), k("ha_ryds_h", "ha_ryds_r") / car, Some(real[3]));
    }
    row("fourth down tries", g("fourth_go") / n, f("fourth_go"));
    row("  fourth downs faced", g("fourth_n") / n, None);
    row("  go share (real table)", g("fourth_n_go") / g("fourth_n"), Some(g("fourth_table") / g("fourth_n")));
    row("pass TD", g("pass_td") / n, f("pass_td"));
    row("  from inside the 5", g("ptd_5") / n, None);
    row("  from the 6 to the 20", g("ptd_20") / n, None);
    row("  from outside the 20", g("ptd_far") / n, None);
    // Red zone passing without throwaways (lane EG6, nflverse play-by-play
    // and FTN charting, regular seasons 2022-25): 60.4% completed and a
    // touchdown on 24.6% of throws (with throwaways 56.7% and 23.2%; the
    // audit's 53% and 19% were an older reading).
    row("red zone completion", g("rz_comp") / g("rz_att"), Some(0.604));
    row("red zone TD per attempt", g("rz_ptd") / g("rz_att"), Some(0.246));
    row("rush TD", g("rush_td") / n, f("rush_td"));
    row("scrambles", g("scrambles") / n, None);
    row("punts", g("punts") / n, None);
    // Real drive numbers: NFL 2021-2024 averages (Pro Football Reference drive stats).
    row("pass rate 1st down neutral", g("neutral_pass") / g("neutral_plays").max(1.0), Some(0.50));
    let gt: Value = serde_json::from_str(include_str!("../../../../data/real-world/garbage_time.json")).unwrap();
    let gl = &gt[if level == Level::NFL { "nfl" } else { "fbs" }]["lead"];
    row(
        "Q4 pass rate, up 1-14 after Q3",
        g("q4_lead14_pass") / g("q4_lead14").max(1.0),
        gl["1-14"]["q4_pass_rate"].as_f64(),
    );
    row(
        "Q4 pass rate, up 28+ after Q3",
        g("q4_lead28_pass") / g("q4_lead28").max(1.0),
        gl["28+"]["q4_pass_rate"].as_f64(),
    );
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
    // Gross, before sacks, as real yards an attempt are (the team's pass
    // yards are net of sacks).
    let ypa = (g("ha_pyds_h") + g("ha_pyds_r")) / (g("ha_att_h") + g("ha_att_r")).max(1.0);
    row("yards per attempt", ypa, rr["yards_per_attempt"].as_f64());
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
    // How unblocked punts end (nflverse 2021-2025 regular seasons, 10,469
    // punts): fair caught, returned (muffs too), touchbacks, and downed or
    // out of bounds (0.121 and 0.101).
    for (k, name, real) in [
        ("punt_fc", "  punts fair caught", 0.2745),
        ("punt_ret", "  punts returned", 0.4302),
        ("punt_tb", "  punts for a touchback", 0.0729),
        ("punt_downed", "  punts downed or out", 0.2223),
    ] {
        row(name, g(k) / g("punt_kicked").max(1.0), Some(real));
    }
    row("kickoff touchback", g("ko_tb") / g("kickoffs"), Some(0.22));
    // Pressure as PFR counts it (ab_real.json means, 2018-25): the passer
    // pressured on 22.2% of dropbacks (times pressured over play-by-play
    // dropbacks), 28.3% of those a sack. The engine's flag follows PFR's
    // since minor bug 284 (the nearest rusher inside 3.2 yards after 0.6 s).
    // FTN's charting flag is wider and leaves some sacks out (28.3% of
    // dropbacks, 13.9% of them sacks, 48.9% completed), so it is not the
    // same count. Scrambles 4.9% of dropbacks (FTN 2021-25). Time to throw
    // (engine_targets.json): mean 2.71 s, p10 1.6, p50 2.5, p90 3.94.
    row("pressure rate", g("pressures") / g("dropbacks"), Some(0.222));
    row("scramble rate", g("scrambles") / g("dropbacks"), Some(0.049));
    row("time to throw", g("ttt") / g("att"), Some(2.71));
    row("  thrown under 1.6 s", g("ttt_u16") / g("att"), Some(0.1));
    row("  thrown under 2.5 s", g("ttt_u25") / g("att"), Some(0.5));
    row("  thrown after 3.94 s", g("ttt_o39") / g("att"), Some(0.1));
    row("sacks per pressure", g("pr_sack") / g("pr_n"), Some(0.283));
    // Play action (scheme-fit item 3; real 2024 NFL with FTN charting,
    // screens and RPOs left out, `afm/scheme-fit/full-pass.md`): 21.2% of
    // dropbacks, +0.149 EPA a dropback against +0.031, sacked 6.6% against
    // 7.3%, boots and rollouts 5.7% (play action out of the pocket, 36% of
    // play action, flushed passers too), 3.15 s to throw against 2.66, 8.60
    // yards an attempt against 6.93. The
    // pressure flag counts as PFR's does, a narrower one than the
    // participation data's (36.9% against 30.9%), so only its rise is real.
    let (pa, npa) = (g("pa_db").max(1.0), g("npa_db").max(1.0));
    row("play action share", pa / (pa + npa), Some(0.212));
    let dbs = pa + npa;
    row("  dropbacks under center", g("db_uc") / dbs, Some(0.141));
    row("    play action there", g("pa_uc") / g("db_uc").max(1.0), Some(0.798));
    row("  dropbacks from the pistol", g("db_pi") / dbs, Some(0.032));
    row("    play action there", g("pa_pi") / g("db_pi").max(1.0), Some(0.407));
    row("  dropbacks from the shotgun", g("db_gun") / dbs, Some(0.814));
    row("    play action there", g("pa_gun") / g("db_gun").max(1.0), Some(0.104));
    row("  on 1st down", g("pa_d1") / g("db_d1").max(1.0), Some(0.366));
    row("  on 2nd down", g("pa_d2") / g("db_d2").max(1.0), Some(0.208));
    row("  on 3rd and 4th down", g("pa_d3") / g("db_d3").max(1.0), Some(0.042));
    row("  EPA a dropback, play action", g("pa_epa") / pa, Some(0.149));
    row("  EPA a dropback, no play action", g("npa_epa") / npa, Some(0.031));
    row("  sack rate, play action", g("pa_sack") / pa, Some(0.066));
    row("  sack rate, no play action", g("npa_sack") / npa, Some(0.073));
    row("  sack rate, boots", g("boot_sack") / g("boot_db").max(1.0), Some(0.057));
    row("  boots, share of play action", g("boot_db") / pa, None);
    for (name, k) in [("shot", "pas"), ("dig", "pad"), ("boot", "pab")] {
        let n = g(&format!("{k}_db")).max(1.0);
        println!(
            "  {name:5} share of play action {:.3}, EPA {:+.3}, sacked {:.3}, pressured {:.3}, scrambles {:.3}",
            n / pa,
            g(&format!("{k}_epa")) / n,
            g(&format!("{k}_sack")) / n,
            g(&format!("{k}_prs")) / n,
            g(&format!("{k}_scr")) / n
        );
    }
    row("  pressure, play action / none", (g("pa_prs") / pa) / (g("npa_prs") / npa).max(1e-9), Some(1.19));
    row("  time to throw, play action", g("pa_ttt") / g("pa_att").max(1.0), Some(3.15));
    row("  time to throw, no play action", g("npa_ttt") / g("npa_att").max(1.0), Some(2.66));
    row("  yards an attempt, play action", g("pa_pyds") / g("pa_att").max(1.0), Some(8.60));
    row("  yards an attempt, no play action", g("npa_pyds") / g("npa_att").max(1.0), Some(6.93));
    row("  air yards an attempt, play action", g("pa_air") / g("pa_att").max(1.0), Some(9.2));
    row("  air yards an attempt, no play action", g("npa_air") / g("npa_att").max(1.0), Some(8.9));
    let pat = g("pa_att").max(1.0);
    row("  play action throws behind the line", g("pa_ay0") / pat, Some(0.148));
    row("  play action throws 0-9 air yards", g("pa_ay1") / pat, Some(0.448));
    row("  play action throws 10-19 air yards", g("pa_ay2") / pat, Some(0.265));
    row("  play action throws 20+ air yards", g("pa_ay3") / pat, Some(0.139));
    row("  scrambles, play action", g("pa_scr") / pa, Some(0.070));
    row("  scrambles, no play action", g("npa_scr") / npa, Some(0.054));
    // Where the quarterback lines up (FTN, NFL 2022-25).
    let all_snaps = ["sp_run", "sp_pa", "sp_drop", "sp_scr"].iter().map(|k| g(k)).sum::<f64>().max(1.0);
    let uc_all = ["sp_run_u", "sp_pa_u", "sp_drop_u", "sp_scr_u"].iter().map(|k| g(k)).sum::<f64>();
    row("snaps under center", uc_all / all_snaps, Some(0.319));
    for (name, k, real) in [
        ("runs", "sp_run", [0.580, 0.101, 0.314]),
        ("play action", "sp_pa", [0.581, 0.048, 0.368]),
        ("straight drops", "sp_drop", [0.040, 0.014, 0.941]),
        ("screens", "sp_scr", [0.187, 0.030, 0.781]),
    ] {
        let n = g(k).max(1.0);
        for (j, at) in ["under center", "pistol", "shotgun"].iter().enumerate() {
            row(&format!("  {name}, {at}"), g(&format!("{k}_{}", ["u", "p", "s"][j])) / n, Some(real[j]));
        }
    }
    row("  screens off a fake", g("sp_scr_pa") / g("sp_scr").max(1.0), Some(0.273));
    row("dropbacks with a rush win", g("rw_any") / g("rw_n"), None);
    row("completions per pressure", g("pr_comp") / g("pr_n"), None);
    row("air yards per att", g("air_yards") / g("att"), None);
    row("drops per att", g("drops") / g("att"), None);
    row("broken up per att", g("inc_pbu") / g("att"), None);
    row("off target per att", g("inc_off") / g("att"), None);
    row("throw aways per att", g("throw_aways") / g("att"), None);
    row("yards before contact", g("ybc") / g("carries"), None);
    row("missed tackles per run", g("missed_tackles") / g("carries"), None);
    // Real: nflverse play-by-play 2021-24, backs' designed runs, the first
    // tackler (solo, then shared tackle), touchdowns left out, grouped by
    // the role PFF lists for him (interior and edge are the line, off-ball
    // linebackers the linebackers). The roster position put the 3-4 edge
    // rushers with the linebackers (line 0.330, linebackers 0.371), but
    // the engine's EDGE is a lineman (bug 164).
    let ft: f64 = first_tackle.iter().map(|r| r[0]).sum();
    for (k, (name, share, yds)) in
        [("line", 0.431, 2.50), ("linebackers", 0.278, 3.69), ("backs", 0.291, 6.96)].into_iter().enumerate()
    {
        row(&format!("first tackle on runs: {name}"), first_tackle[k][0] / ft, Some(share));
        row("  gain when they make it", first_tackle[k][1] / first_tackle[k][0].max(1.0), Some(yds));
    }
    row("YAC per completion", g("yac") / g("cmp"), Some(5.2));
    // Real 2024 regular season (nflverse, completions with YAC recorded).
    for (k, name, real) in [
        ("yac_0", "YAC 0 or less", 0.247),
        ("yac_1", "YAC 1-4", 0.343),
        ("yac_5", "YAC 5-9", 0.229),
        ("yac_10", "YAC 10-19", 0.143),
        ("yac_20", "YAC 20+", 0.039),
    ] {
        row(name, g(k) / g("cmp").max(1.0), Some(real));
    }
    row("YAC 20+ of 10+", g("yac_20") / (g("yac_10") + g("yac_20")).max(1.0), Some(0.214));
    let real_ay = [(0.176, 0.788), (0.501, 0.709), (0.207, 0.545), (0.115, 0.360)];
    let tot: f64 = ay.iter().map(|b| b[0]).sum();
    for (k, name) in ["air < 0", "air 0-9", "air 10-19", "air 20+"].iter().enumerate() {
        row(&format!("{name} share"), ay[k][0] / tot.max(1.0), Some(real_ay[k].0));
        row(&format!("{name} completion"), ay[k][1] / ay[k][0].max(1.0), Some(real_ay[k].1));
        row(&format!("{name} int rate"), ay[k][2] / ay[k][0].max(1.0), None);
        row(&format!("{name} YAC"), ay[k][3] / ay[k][1].max(1.0), Some([9.49, 4.09, 3.67, 5.72][k]));
        row(&format!("{name} separation"), ay[k][4] / ay[k][0].max(1.0), None);
        row(&format!("{name} tight window"), ay[k][5] / ay[k][0].max(1.0), None);
    }
    for (k, name) in ["zone", "man"].iter().enumerate() {
        row(&format!("{name}: att share / cmp / ypa"), by_cov[k][0] / (by_cov[0][0] + by_cov[1][0]).max(1.0), None);
        row("", by_cov[k][1] / by_cov[k][0].max(1.0), None);
        row("", by_cov[k][2] / by_cov[k][0].max(1.0), None);
    }
    // Targets by route against NFL 2023-25 (data/real-world/nfl_route_depth.json,
    // one route list), with the real route types pooled into the engine's
    // route groups.
    let rd: Value = serde_json::from_str(include_str!("../../../../data/real-world/nfl_route_depth.json")).unwrap();
    let tr = &rd["by_route"];
    let rt: f64 = by_route.iter().map(|b| b[0]).sum();
    let real_all = rd["all"]["n"].as_f64().unwrap_or(1.0);
    println!("targets by route                share  real  catch  real   ypt  real   air  real   ttt  real");
    for (k, (name, _, real_keys)) in ROUTE_GROUPS.iter().enumerate() {
        let mut v = [0.0; 5];
        for key in real_keys.iter() {
            let r = &tr[*key];
            let rn = r["n"].as_f64().unwrap_or(0.0);
            v[0] += rn;
            for (j, f) in ["catch", "ypt", "air", "ttt"].iter().enumerate() {
                v[j + 1] += rn * r[*f].as_f64().unwrap_or(0.0);
            }
        }
        let b = by_route[k];
        let n = v[0].max(1.0);
        println!(
            "  {name:<28} {:.3} {:.3}  {:.3} {:.3} {:5.2} {:5.2} {:5.1} {:5.1}  {:.2}  {:.2}",
            b[0] / rt.max(1.0),
            v[0] / real_all,
            b[1] / b[0].max(1.0),
            v[1] / n,
            b[2] / b[0].max(1.0),
            v[2] / n,
            b[3] / b[0].max(1.0),
            v[3] / n,
            b[4] / b[0].max(1.0),
            v[4] / n
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
    // Where designed runs crossed the line (or, stopped behind it, where
    // they were called) against nflverse 2021-25's run
    // gaps (end 5.06 a carry, 23% of designed runs; guard, tackle and
    // middle 4.05), and by the call (outside zone, toss, sweep, option).
    if games > 0 {
        let rd = |k: usize| run_dir[k][1] / run_dir[k][0].max(1.0);
        row("ypc, inside the tackles", rd(0), Some(4.05));
        row("ypc, outside the tackles", rd(1), Some(5.06));
        row("share outside the tackles", run_dir[1][0] / (run_dir[0][0] + run_dir[1][0]).max(1.0), Some(0.231));
        row("ypc, inside calls", rd(2), None);
        row("ypc, outside calls", rd(3), None);
        row("share of outside calls", run_dir[3][0] / (run_dir[2][0] + run_dir[3][0]).max(1.0), None);
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
        let et: Value = serde_json::from_str(include_str!("../../../../data/real-world/engine_targets.json")).unwrap();
        let tc = &et["targets_by_route_and_coverage"];
        for (cov, key) in [("Cover 1", "COVER_1"), ("Cover 2", "COVER_2"), ("Cover 3", "COVER_3"), ("2-man", "2_MAN")] {
            let tot: f64 =
                (0..ROUTE_GROUPS.len()).map(|g| cov_route.get(&(cov.to_string(), g)).map_or(0.0, |v| v[0])).sum();
            let mut rtot = 0.0;
            for (_, keys, _) in ROUTE_GROUPS.iter() {
                for k in keys.iter() {
                    rtot += tc[*k][key]["n"].as_f64().unwrap_or(0.0);
                }
            }
            println!("{cov}: route share real catch real ypt real");
            for (g, (name, keys, _)) in ROUTE_GROUPS.iter().enumerate() {
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
        row("yards per attempt", ypa, cq["yards_per_attempt"].as_f64());
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
    if std::env::var("AFM_RUNDIAG").is_ok() {
        let pairs: Vec<(usize, usize)> = (0..games).map(pair).collect();
        run_diag(&results, &pos_of, &pairs, &teams);
        sf9_diag(&results, &pos_of);
        // Who starts the tackle on designed runs, by his block at the hit
        // (scheme-fit item 7, `sim::FIRST_TACKLE_BLOCK`).
        let h: Vec<f64> = afm_engine::game::sim::FIRST_TACKLE_BLOCK
            .iter()
            .map(|a| a.load(std::sync::atomic::Ordering::Relaxed) as f64)
            .collect();
        let n: f64 = h.iter().sum::<f64>().max(1.0);
        println!(
            "  the hit that starts the tackle on designed runs, by the tackler's block then: free {:.3}, on a block he isn't winning (w <= 0) {:.3}, winning it at w up to 0.3 {:.3}, 0.3-0.6 {:.3}, past 0.6 {:.3}",
            h[0] / n,
            h[1] / n,
            h[2] / n,
            h[3] / n,
            h[4] / n
        );
    }
    if std::env::var("AFM_RECDIAG").is_ok() {
        rec_diag(&results, &teams);
    }
    if std::env::var("AFM_PICKS").is_ok() {
        pick_diag(&results, &pos_of);
    }
    if std::env::var("AFM_GAPS").is_ok() {
        gaps_counts(&results, &pos_of, level);
        foul_counts(&results, level);
        red_zone_counts(&results, level);
        off_script_counts(&results);
    }
    if !burners.is_empty() {
        println!(
            "\nBurner proof (AFM_BURNER): per team game, teammates' catches under 10 air yards {:.2}, their yards after the catch {:.2} a catch; the burner {:.2} targets, {:.2} catches, {:.1} yards; two-high on {:.1}% of passes",
            bp[0] / n,
            bp[1] / bp[0].max(1.0),
            bp[2] / n,
            bp[3] / n,
            bp[4] / n,
            100.0 * bp[5] / bp[6].max(1.0)
        );
    }
}

/// AFM_GAPS=1: the engine gaps audit's counts (`docs/design/engine-gaps.md`),
/// so every EG lane measures its items the same way before and after:
/// points by quarter and late in each half, margins and overtime, injuries
/// by position, field goals by distance with blocks, punts and kickoffs with
/// their returns, tries, safeties, fumbles by kind, the RPO share,
/// quarterback runs, pass interference yards, penalties by kind and the call
/// on a half's last snap. Per team game unless named.
fn gaps_counts(
    results: &[afm_engine::game::GameResult],
    pos_of: &std::collections::HashMap<u32, afm_engine::positions::Position>,
    level: Level,
) {
    use afm_engine::game::sim::FoulKind;
    use afm_engine::game::stats::PlayType as PT;
    use afm_engine::positions::Position;
    use std::collections::BTreeMap;
    let tg = results.len() as f64 * 2.0;
    let is_qb = |id: Option<u32>| id.and_then(|c| pos_of.get(&c)) == Some(&Position::QB);
    let mut q = [0.0f64; 5];
    // Injuries by position: [all, a week or more, plays on hurt].
    let mut inj: BTreeMap<String, [f64; 3]> = BTreeMap::new();
    // Field goals by distance: [tries, made, blocked].
    let mut fg: BTreeMap<&str, [f64; 3]> = BTreeMap::new();
    // Punts: [punts, blocked, returned, return yards, fair catches].
    let mut punt = [0.0f64; 5];
    // Kickoffs: [kickoffs, returned, return yards].
    let mut ko = [0.0f64; 3];
    // Tries: [extra points, good, blocked, two-point tries, good].
    let mut tries = [0.0f64; 5];
    // Scrimmage: [plays, RPOs, runs, QB designed runs, scrambles, kneels].
    let mut scrim = [0.0f64; 6];
    // Safeties, fumbles, fumbles lost.
    let mut fum = [0.0f64; 3];
    // Penalties by kind: [flagged, accepted].
    let mut pens: BTreeMap<String, [f64; 2]> = BTreeMap::new();
    // Fumbles by kind: [fumbles, lost].
    let mut fcat: BTreeMap<String, [f64; 2]> = BTreeMap::new();
    // Fumbles by nflverse's kinds and precedence (engine gaps items 4-7; the
    // spec's `real_fumbles` reading): [fumbles, lost, out of bounds, the
    // defense's recoveries, advanced, 10+ yards, returned for a score, the
    // fumbler's own recoveries, a teammate's].
    let mut fnv: BTreeMap<&str, [f64; 9]> = BTreeMap::new();
    // What each kind is a share of: [sacks, designed runs, catches, snaps
    // under center, in the gun, fumbled snaps under center, in the gun].
    let mut per = [0.0f64; 7];
    // Plays and fumbles by the man's position: "sack by EDGE" and the like.
    let mut bypos: BTreeMap<String, [f64; 2]> = BTreeMap::new();
    // Accepted pass interference: [yards, flags].
    let mut dpi = [0.0f64; 2];
    // The half's last snap: each call's count, then [snaps, touchdowns].
    let mut last: BTreeMap<String, f64> = BTreeMap::new();
    let mut last_n = [0.0f64; 2];
    // Laterals (item 16): passes thrown with 5 s or less left in the
    // fourth quarter from 40+ yards out, tied or down 1-8, and those with
    // a lateral.
    let mut lat = [0.0f64; 2];
    // Hail Marys (item 15) by yards out (under 40, 40s, 50s, 60s, 70+):
    // [snaps, touchdowns, interceptions], for the win chance's credit.
    let mut hail = [[0.0f64; 3]; 5];
    // Points in the last 2:00 of the second and fourth quarters.
    let mut late = [0.0f64; 2];
    // Timeouts in the last 4:00 of the second and fourth quarters (engine
    // gaps item 17): [Q2 defense, Q2 offense, Q4 defense, Q4 offense], and
    // the games with a defensive one in the second quarter's.
    let mut tos = [0.0f64; 4];
    let mut to_games = 0.0f64;
    // Out of bounds (item 18): [designed runs, out, catches, out] by the
    // side ahead in the last 5:00, then in the first three quarters.
    let mut oob = [[0.0f64; 4]; 2];
    // AI fakes (item 21): [fake punts, converted, fake field goals,
    // converted]; snaps at 0:00 (item 22, untimed downs).
    let mut fakes = [0.0f64; 4];
    let mut untimed = 0.0f64;
    let mut ot_games = 0.0f64;
    let mut margins: Vec<i32> = Vec::new();
    // The kicking game (lane EG3): kickoff returns of 40+, punt returns of
    // 20+, kickoff and punt return touchdowns; blocked kicks of every kind;
    // blocked punts kept by the punting team (with a first down), returned
    // for a touchdown and safeties; blocked field goals advanced and
    // returned for a touchdown; field goals of 50+ tried and made.
    let mut kk = [0.0f64; 14];
    // 55+ tries in a half's last 30 seconds and the rest; fourth downs from
    // their 36-44 outside a half's last two minutes: field goal, punt, go.
    let mut long = [0.0f64; 2];
    let mut f4 = [[0.0f64; 3]; 3];
    let mut rets: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
    // Accepted fouls on kickoffs and punts by team and kind (bug 403).
    let mut kfoul: BTreeMap<String, f64> = BTreeMap::new();
    for r in results {
        if r.stats.overtime {
            ot_games += 1.0;
        }
        margins.push((r.stats.score[0] - r.stats.score[1]).abs());
        let mut q2_def_to = false;
        for w in r.stats.plays.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            let (Some(ta), Some(tb)) = (a.timeouts, b.timeouts) else { continue };
            if a.quarter != b.quarter || !(b.quarter == 2 || b.quarter == 4) || b.clock > 240.0 {
                continue;
            }
            for k in 0..2 {
                if tb[k] < ta[k] {
                    let used = (ta[k] - tb[k]) as f64;
                    let def = k != b.offense;
                    let i = if b.quarter == 2 { 0 } else { 2 } + if def { 0 } else { 1 };
                    tos[i] += used;
                    q2_def_to |= def && b.quarter == 2;
                }
            }
        }
        if q2_def_to {
            to_games += 1.0;
        }
        for (qi, qs) in r.stats.quarters.iter().enumerate() {
            q[qi.min(4)] += (qs[0] + qs[1]) as f64;
        }
        for (_, i) in &r.injuries {
            let pos = pos_of.get(&i.id).map_or_else(|| "?".to_string(), |p| format!("{p:?}"));
            let e = inj.entry(pos).or_default();
            e[0] += 1.0;
            if i.weeks >= 1.0 {
                e[1] += 1.0;
            }
            if i.plays_on {
                e[2] += 1.0;
            }
        }
        let ps = &r.stats.plays;
        for (k, p) in ps.iter().enumerate() {
            if matches!(p.kind, PT::Kickoff | PT::Punt) {
                let ko = p.kind == PT::Kickoff;
                let returned = p.returner.is_some() && !p.touchback && !p.fair_catch && !p.blocked;
                if returned {
                    rets[usize::from(!ko)].push(p.return_yards);
                }
                if returned && p.return_yards >= if ko { 40.0 } else { 20.0 } {
                    kk[if ko { 0 } else { 1 }] += 1.0;
                }
                if returned && p.touchdown && !p.turnover {
                    kk[if ko { 2 } else { 3 }] += 1.0;
                }
                let kicking = p.kicker.and_then(|id| p.side_of(id)).unwrap_or(p.offense);
                for f in p.penalties.iter().filter(|f| f.4) {
                    let by = if f.1 == kicking { "kicking" } else { "receiving" };
                    *kfoul
                        .entry(format!("{} {by} {}", if ko { "kickoff" } else { "punt" }, f.0.name()))
                        .or_default() += 1.0;
                }
            }
            if p.blocked && matches!(p.kind, PT::FieldGoal | PT::Punt | PT::ExtraPoint) {
                kk[4] += 1.0;
                let next = ps[k + 1..].iter().find(|n| {
                    matches!(
                        n.kind,
                        PT::Run | PT::Pass | PT::Sack | PT::Scramble | PT::Kneel | PT::Spike | PT::Punt | PT::FieldGoal
                    )
                });
                if p.kind == PT::Punt {
                    kk[5] += 1.0;
                    if !p.touchdown
                        && !p.safety
                        && next.is_some_and(|n| n.offense == p.offense && n.quarter == p.quarter)
                    {
                        kk[6] += 1.0;
                        if next.is_some_and(|n| n.down == 1) {
                            kk[7] += 1.0;
                        }
                    }
                    if p.touchdown {
                        kk[8] += 1.0;
                    }
                    if p.safety {
                        kk[9] += 1.0;
                    }
                } else if p.kind == PT::FieldGoal {
                    kk[10] += 1.0;
                    if p.return_yards > 0.0 {
                        kk[11] += 1.0;
                    }
                    if p.touchdown {
                        kk[12] += 1.0;
                    }
                }
            }
            let half_left = if p.quarter == 2 || p.quarter == 4 { p.clock } else { 999.0 };
            if p.kind == PT::FieldGoal && p.kick_distance >= 55.0 {
                long[usize::from(half_left > 30.0)] += 1.0;
            }
            if p.down == 4 && half_left > 120.0 && (36.0..45.0).contains(&p.yardline_100) && p.quarter <= 4 {
                let b = ((p.yardline_100 - 36.0) / 3.0) as usize;
                let k = match p.kind {
                    PT::FieldGoal => Some(0),
                    PT::Punt => Some(1),
                    PT::Run | PT::Pass | PT::Sack | PT::Scramble => Some(2),
                    _ => None,
                };
                if let Some(k) = k {
                    f4[b.min(2)][k] += 1.0;
                }
            }
            if p.kind == PT::FieldGoal && p.kick_distance >= 50.0 && p.kick_good {
                kk[13] += 1.0;
            }
            for f in &p.penalties {
                let e = pens.entry(f.0.name().to_string()).or_default();
                e[0] += 1.0;
                if f.4 {
                    e[1] += 1.0;
                    if f.0 == FoulKind::PassInterference {
                        dpi[0] += f.3;
                        dpi[1] += 1.0;
                    }
                }
            }
            if p.safety {
                fum[0] += 1.0;
            }
            if p.fumbled.is_some() {
                fum[1] += 1.0;
                if p.fumble_lost {
                    fum[2] += 1.0;
                }
                // How it came out (the engine's own kinds), and who had it.
                use afm_engine::game::sim::FumbleHow;
                let note = p.fumble.unwrap_or_default();
                let fpos = p.fumbled.and_then(|f| pos_of.get(&f)).map_or("?".to_string(), |x| format!("{x:?}"));
                let cat =
                    format!("{:?}{}", note.how, if matches!(p.kind, PT::Kickoff | PT::Punt) { " (kick)" } else { "" });
                let e = fcat.entry(format!("{cat:<16} by {fpos}")).or_default();
                e[0] += 1.0;
                if p.fumble_lost {
                    e[1] += 1.0;
                }
                // nflverse's order: a kick, then an aborted snap, a sack and
                // a scramble over it, then a designed run, a catch and a
                // pass without one. A snap he picked up and threw from is a
                // pass's.
                let nv = match p.kind {
                    PT::Kickoff | PT::Punt => "kick or punt play",
                    _ if note.how == FumbleHow::Snap && p.kind != PT::Pass => "aborted snap",
                    PT::Sack => "sack",
                    PT::Scramble => "scramble",
                    PT::Run => "designed run",
                    PT::Pass if p.complete => "after a catch",
                    PT::Pass => "pass, no catch",
                    _ => "other",
                };
                let e = fnv.entry(nv).or_default();
                e[0] += 1.0;
                if p.fumble_lost {
                    e[1] += 1.0;
                }
                if note.out {
                    e[2] += 1.0;
                }
                // The defense of the play came up with it (on an
                // interception the offense's man is the one who recovers).
                let d = 1 - p.offense;
                if p.recovered.is_some_and(|id| p.side_of(id) == Some(d)) {
                    e[3] += 1.0;
                    if note.ran >= 0.5 {
                        e[4] += 1.0;
                    }
                    if note.ran >= 10.0 {
                        e[5] += 1.0;
                    }
                    if p.defense_td && !p.interception {
                        e[6] += 1.0;
                    }
                } else if p.recovered.is_some() && p.recovered == p.fumbled {
                    e[7] += 1.0;
                } else if p.recovered.is_some() {
                    e[8] += 1.0;
                }
            }
            if matches!(p.kind, PT::Run | PT::Pass | PT::Sack | PT::Scramble) {
                let uc = p.call.contains("under center");
                per[3 + (!uc) as usize] += 1.0;
                if p.fumble.is_some_and(|f| f.how == afm_engine::game::sim::FumbleHow::Snap) {
                    per[5 + (!uc) as usize] += 1.0;
                }
                match p.kind {
                    PT::Sack => per[0] += 1.0,
                    PT::Run => per[1] += 1.0,
                    PT::Pass if p.complete => per[2] += 1.0,
                    _ => {}
                }
                // Each kind's rate by the man's position (engine gaps items 6
                // and 7): the sacker, the designed run's carrier, the catcher.
                let snap = p.fumble.is_some_and(|f| f.how == afm_engine::game::sim::FumbleHow::Snap);
                let key = match p.kind {
                    PT::Sack => p.sackers.first().map(|s| ("sack by", *s)),
                    PT::Run if !snap => p.carrier.map(|c| ("designed run by", c)),
                    PT::Pass if p.complete => p.target.map(|c| ("catch by", c)),
                    _ => None,
                };
                if let Some((k, id)) = key {
                    let pos = pos_of.get(&id).map_or("?".to_string(), |x| format!("{x:?}"));
                    let e = bypos.entry(format!("{k} {pos}")).or_default();
                    e[0] += 1.0;
                    if p.fumbled.is_some() && !snap {
                        e[1] += 1.0;
                    }
                }
            }
            match p.kind {
                PT::Run | PT::Pass | PT::Sack | PT::Scramble => {
                    scrim[0] += 1.0;
                    if p.call.contains("RPO") {
                        scrim[1] += 1.0;
                    }
                    if p.kind == PT::Scramble {
                        scrim[4] += 1.0;
                    }
                    if p.kind == PT::Run {
                        scrim[2] += 1.0;
                        if is_qb(p.carrier) && !p.call.contains("kneel") {
                            scrim[3] += 1.0;
                        }
                    }
                }
                PT::Kneel => scrim[5] += 1.0,
                PT::Punt => {
                    punt[0] += 1.0;
                    if p.blocked {
                        punt[1] += 1.0;
                    }
                    if p.fair_catch {
                        punt[4] += 1.0;
                    } else if p.returner.is_some() && !p.touchback && !p.blocked {
                        punt[2] += 1.0;
                        punt[3] += p.return_yards;
                    }
                }
                PT::Kickoff => {
                    ko[0] += 1.0;
                    if p.returner.is_some() && !p.touchback && !p.fair_catch {
                        ko[1] += 1.0;
                        ko[2] += p.return_yards;
                    }
                }
                PT::FieldGoal => {
                    let d = p.kick_distance;
                    let b = match d {
                        d if d < 30.0 => "a <30",
                        d if d < 40.0 => "b 30-39",
                        d if d < 50.0 => "c 40-49",
                        d if d < 55.0 => "d 50-54",
                        _ => "e 55+",
                    };
                    let e = fg.entry(b).or_default();
                    e[0] += 1.0;
                    if p.kick_good {
                        e[1] += 1.0;
                    }
                    if p.blocked {
                        e[2] += 1.0;
                    }
                }
                PT::ExtraPoint => {
                    tries[0] += 1.0;
                    if p.kick_good {
                        tries[1] += 1.0;
                    }
                    if p.blocked {
                        tries[2] += 1.0;
                    }
                }
                PT::TwoPoint => {
                    tries[3] += 1.0;
                    if p.touchdown || p.kick_good {
                        tries[4] += 1.0;
                    }
                }
                _ => {}
            }
            // Out of bounds by the side ahead late, against the first three
            // quarters: designed runs (no kneels or sneaks) and catches.
            if !p.nullified && matches!(p.kind, PT::Run | PT::Pass) && !p.scramble {
                let ahead_late = p.quarter == 4 && p.clock <= 300.0 && p.score[p.offense] > p.score[1 - p.offense];
                let w = if ahead_late {
                    Some(0)
                } else if p.quarter <= 3 {
                    Some(1)
                } else {
                    None
                };
                if let Some(w) = w {
                    let run = p.kind == PT::Run && !p.call.contains("kneel") && !p.call.contains("sneak");
                    let catch = p.kind == PT::Pass && p.complete;
                    let k = if run {
                        Some(0)
                    } else if catch {
                        Some(2)
                    } else {
                        None
                    };
                    if let Some(k) = k {
                        oob[w][k] += 1.0;
                        if p.out_of_bounds {
                            oob[w][k + 1] += 1.0;
                        }
                    }
                }
            }
            if p.call_why.starts_with("a fake punt") && matches!(p.kind, PT::Run | PT::Pass) {
                fakes[0] += 1.0;
                fakes[1] += (p.first_down || p.touchdown) as u8 as f64;
            }
            if p.call_why.starts_with("a Hail Mary") {
                let b = ((p.yardline_100 - 30.0) / 10.0).clamp(0.0, 4.0) as usize;
                hail[b][0] += 1.0;
                hail[b][1] += p.touchdown as u8 as f64;
                hail[b][2] += p.interception as u8 as f64;
            }
            if p.call_why.starts_with("a fake field goal") && matches!(p.kind, PT::Run | PT::Pass) {
                fakes[2] += 1.0;
                fakes[3] += (p.first_down || p.touchdown) as u8 as f64;
            }
            if p.clock <= 0.0 && p.quarter <= 4 && !matches!(p.kind, PT::ExtraPoint | PT::TwoPoint | PT::NoPlay) {
                untimed += 1.0;
            }
            // The half's last snap from scrimmage or a kick, 5 s or less:
            // every second-quarter one, and the fourth quarter's when tied or
            // down 1-8.
            let ends_half = (p.quarter == 2 || p.quarter == 4) && p.clock <= 5.0;
            if ends_half && !matches!(p.kind, PT::NoPlay | PT::ExtraPoint | PT::TwoPoint | PT::Kickoff) {
                let lead = p.score[p.offense] - p.score[1 - p.offense];
                if p.quarter == 2 || (-8..=0).contains(&lead) {
                    let deep = if p.kind == PT::Pass && p.air_yards >= 35.0 { " 35+ air" } else { "" };
                    *last.entry(format!("Q{} {:?}{deep}", p.quarter, p.kind)).or_default() += 1.0;
                    last_n[0] += 1.0;
                    if p.touchdown {
                        last_n[1] += 1.0;
                    }
                    if p.quarter == 4 && p.kind == PT::Pass && p.yardline_100 >= 40.0 {
                        lat[0] += 1.0;
                        lat[1] += (p.laterals > 0) as u8 as f64;
                    }
                }
            }
            if let Some(nx) = ps.get(k + 1) {
                let pts = ((nx.score[0] + nx.score[1]) - (p.score[0] + p.score[1])) as f64;
                if p.clock <= 120.0 && p.quarter == 2 {
                    late[0] += pts;
                }
                if p.clock <= 120.0 && p.quarter == 4 {
                    late[1] += pts;
                }
            }
        }
    }
    let share =
        |f: &dyn Fn(i32) -> bool| margins.iter().filter(|&&x| f(x)).count() as f64 / margins.len().max(1) as f64;
    println!("\n=== Engine gaps counts (AFM_GAPS) {tg} team games ===");
    println!(
        "points a team game by quarter: Q1 {:.2} Q2 {:.2} Q3 {:.2} Q4 {:.2} OT {:.2}",
        q[0] / tg,
        q[1] / tg,
        q[2] / tg,
        q[3] / tg,
        q[4] / tg
    );
    println!("points a team game in the last 2:00 of Q2 {:.2}, of Q4 {:.2}", late[0] / tg, late[1] / tg);
    println!("overtime games {:.3}", ot_games / (tg / 2.0));
    println!(
        "margin <=3 {:.3}, <=7 {:.3}, 14+ {:.3}, 21+ {:.3}",
        share(&|x| x <= 3),
        share(&|x| x <= 7),
        share(&|x| x >= 14),
        share(&|x| x >= 21)
    );
    println!("injuries a team game by position (all / 1+ week / plays on):");
    let mut tot = [0.0f64; 3];
    for e in inj.values() {
        for (t, v) in tot.iter_mut().zip(e) {
            *t += v;
        }
    }
    for (k, e) in &inj {
        println!(
            "  {k:<5} {:.3} {:.3} {:.3}  share of 1+wk {:.3}",
            e[0] / tg,
            e[1] / tg,
            e[2] / tg,
            e[1] / tot[1].max(1.0)
        );
    }
    println!("  all   {:.3} {:.3} {:.3}", tot[0] / tg, tot[1] / tg, tot[2] / tg);
    injury_counts(results, pos_of);
    println!("field goals by distance (att a team game, make, blocked share):");
    for (k, e) in &fg {
        println!("  {k:<8} {:.3}  {:.3}  {:.4}", e[0] / tg, e[1] / e[0].max(1.0), e[2] / e[0].max(1.0));
    }
    let fga: f64 = fg.values().map(|e| e[0]).sum();
    let fgb: f64 = fg.values().map(|e| e[2]).sum();
    println!("  all FG blocked {:.4} of tries", fgb / fga.max(1.0));
    println!(
        "punts {:.2} a team game, blocked {:.4}; punt returns {:.3} a punt avg {:.2}, fair catch {:.3} a punt",
        punt[0] / tg,
        punt[1] / punt[0].max(1.0),
        punt[2] / punt[0].max(1.0),
        punt[3] / punt[2].max(1.0),
        punt[4] / punt[0].max(1.0)
    );
    println!(
        "kickoffs {:.2} a team game, returned {:.3} avg {:.2}",
        ko[0] / tg,
        ko[1] / ko[0].max(1.0),
        ko[2] / ko[1].max(1.0)
    );
    let fg50: f64 = fg.iter().filter(|(k, _)| k.starts_with('d') || k.starts_with('e')).map(|(_, e)| e[0]).sum();
    println!(
        "  50+ {:.3} a team game, make {:.3} (real NFL 2021-25 outdoor: calm and dry 0.386 made 0.657, wind 15+ 0.335 made 0.780, 32 F or colder 0.286 made 0.594)",
        fg50 / tg,
        kk[13] / fg50.max(1.0)
    );
    println!(
        "returns: kickoff 40+ {:.3} of returns (real 0.050), punt 20+ {:.3} (real 0.080); return TDs a team game kickoff {:.4} punt {:.4} (real all 0.024)",
        kk[0] / ko[1].max(1.0),
        kk[1] / punt[2].max(1.0),
        kk[2] / tg,
        kk[3] / tg
    );
    println!(
        "  55+ tries a team game: in a half's last 30 seconds {:.3}, the rest {:.3} (real NFL 2024-25 0.060 and 0.125)",
        long[0] / tg,
        long[1] / tg
    );
    for (b, row) in f4.iter().enumerate() {
        let t = row.iter().sum::<f64>().max(1.0);
        println!(
            "  fourth downs from their {}-{} outside a half's last two minutes: field goal {:.2}, punt {:.2}, go {:.2} of {:.0} (real NFL 2024-25: {})",
            36 + 3 * b,
            38 + 3 * b,
            row[0] / t,
            row[1] / t,
            row[2] / t,
            t,
            ["0.56 0.05 0.39", "0.24 0.33 0.43", "0.02 0.60 0.37"][b]
        );
    }
    for (n, v) in rets.iter_mut().enumerate() {
        v.sort_by(f64::total_cmp);
        let q = |f: f64| v.get(((v.len() as f64 - 1.0) * f).round().max(0.0) as usize).copied().unwrap_or(0.0);
        let share = |y: f64| v.iter().filter(|&&r| r >= y).count() as f64 / v.len().max(1) as f64;
        let marks: [f64; 3] = if n == 0 { [30.0, 40.0, 60.0] } else { [10.0, 20.0, 40.0] };
        println!(
            "  {} return yards: p10 {:.0} p25 {:.0} p50 {:.0} p75 {:.0} p90 {:.0} p95 {:.0} p99 {:.0}; {:.0}+ {:.3} {:.0}+ {:.3} {:.0}+ {:.3}",
            if n == 0 { "kickoff" } else { "punt" },
            q(0.1),
            q(0.25),
            q(0.5),
            q(0.75),
            q(0.9),
            q(0.95),
            q(0.99),
            marks[0],
            share(marks[0]),
            marks[1],
            share(marks[1]),
            marks[2],
            share(marks[2])
        );
    }
    println!(
        "blocked kicks {:.4} a team game (real 0.072); blocked punts {:.0}: kept by the punting team {:.3} (with a first down {:.3}), returned for TDs {:.3} (real 17 of 51), safeties {:.3} (3 of 51); blocked FGs {:.0}: advanced {:.3}, returned for TDs {:.3} (real 8 of 101)",
        kk[4] / tg,
        kk[5],
        kk[6] / kk[5].max(1.0),
        kk[7] / kk[5].max(1.0),
        kk[8] / kk[5].max(1.0),
        kk[9] / kk[5].max(1.0),
        kk[10],
        kk[11] / kk[10].max(1.0),
        kk[12] / kk[10].max(1.0)
    );
    let kf: f64 = kfoul.values().sum();
    println!(
        "accepted fouls on kicks {:.3} a team game (real NFL 2025 0.86: kickoffs 0.54, punts 0.32), by kind:",
        kf / tg
    );
    for (k, v) in &kfoul {
        println!("  {k:<48} {:.4}", v / tg);
    }
    {
        use afm_engine::attributes::Attr;
        use afm_engine::game::synthetic::typical_starter as ts;
        let k = Position::K;
        println!(
            "the typical starting kicker: kick power {:.1}, short accuracy {:.1}, long accuracy {:.1}, trajectory {:.1}, get off {:.1}",
            ts(level, k, Attr::KickPower),
            ts(level, k, Attr::ShortKickAccuracy),
            ts(level, k, Attr::LongKickAccuracy),
            ts(level, k, Attr::KickTrajectory),
            ts(level, k, Attr::GetOff)
        );
    }
    println!(
        "XP {:.2} a team game make {:.3} blocked {:.4}; two-point {:.3} a team game, good {:.3}",
        tries[0] / tg,
        tries[1] / tries[0].max(1.0),
        tries[2] / tries[0].max(1.0),
        tries[3] / tg,
        tries[4] / tries[3].max(1.0)
    );
    println!(
        "safeties {:.4} a team game; fumbles {:.3} a team game, lost {:.3} of them",
        fum[0] / tg,
        fum[1] / tg,
        fum[2] / fum[1].max(1.0)
    );
    println!(
        "RPO share of scrimmage plays {:.3}; QB designed runs {:.2} a team game ({:.3} of runs); scrambles {:.2}; kneels {:.2}",
        scrim[1] / scrim[0].max(1.0),
        scrim[3] / tg,
        scrim[3] / scrim[2].max(1.0),
        scrim[4] / tg,
        scrim[5] / tg
    );
    println!("fumbles by how the ball came out and who had it:");
    for (k, e) in &fcat {
        println!("  {k:<28} {:.4} a team game, lost {:.3}", e[0] / tg, e[1] / e[0].max(1.0));
    }
    // Real, nflverse 2021-25 (`real_eg2.py`): sack 0.298 lost 0.510 out
    // 0.038 advanced 0.327 10+ 0.147 scores 46; after a catch 0.244 0.500
    // 0.190 0.313 0.166 17; designed run 0.197 0.576 0.058 0.241 0.122 18;
    // aborted snap 0.185 0.247 0.026 0.134 0.067 5; pass, no catch 0.036
    // 0.082 0.072; scramble 0.024 0.469 0.281. All: 0.983 a team game,
    // lost 0.455, out of bounds 0.085, advanced 0.280, 10+ 0.136, return
    // touchdowns 0.032 a team game.
    // Who came up with it, real (`real_eg2e.py`; the defense, the fumbler
    // himself, a teammate, out of bounds): designed run 0.564 0.185 0.193
    // 0.058; after a catch 0.488 0.156 0.166 0.190; sack 0.503 0.153
    // 0.305 0.038; aborted snap 0.237 0.374 0.364 0.026; all from
    // scrimmage 0.448 0.214 0.253 0.085.
    println!(
        "fumbles by nflverse's kinds (a team game, lost, out of bounds; the defense's recoveries advanced, 10+, scores; who came up with it: the defense, himself, a teammate):"
    );
    let mut all = [0.0f64; 9];
    for (k, e) in &fnv {
        for (a, v) in all.iter_mut().zip(e) {
            *a += v;
        }
        println!(
            "  {k:<18} {:.3}  lost {:.3}  out {:.3}  adv {:.3}  10+ {:.3}  TDs {:<4}  who {:.3} {:.3} {:.3}",
            e[0] / tg,
            e[1] / e[0].max(1.0),
            e[2] / e[0].max(1.0),
            e[4] / e[3].max(1.0),
            e[5] / e[3].max(1.0),
            e[6],
            e[3] / e[0].max(1.0),
            e[7] / e[0].max(1.0),
            e[8] / e[0].max(1.0)
        );
    }
    println!(
        "  all                {:.3}  lost {:.3}  out {:.3}  adv {:.3}  10+ {:.3}  return TDs {:.4} a team game  who {:.3} {:.3} {:.3}",
        all[0] / tg,
        all[1] / all[0].max(1.0),
        all[2] / all[0].max(1.0),
        all[4] / all[3].max(1.0),
        all[5] / all[3].max(1.0),
        all[6] / tg,
        all[3] / all[0].max(1.0),
        all[7] / all[0].max(1.0),
        all[8] / all[0].max(1.0)
    );
    let rate = |k: &str, n: f64| fnv.get(k).map_or(0.0, |e| e[0]) / n.max(1.0);
    // Real: 12.4% of sacks, 0.81% of designed runs, 1.13% of catches;
    // snaps on the ground 0.467% under center, 0.227% in the gun.
    println!(
        "fumble rates: a sack {:.4}, a designed run {:.4}, a catch {:.4}; fumbled snaps under center {:.5}, in the gun {:.5}",
        rate("sack", per[0]),
        rate("designed run", per[1]),
        rate("after a catch", per[2]),
        per[5] / per[3].max(1.0),
        per[6] / per[4].max(1.0)
    );
    // Real (nflverse 2021-25): sacks fumble by the sacker's spot DE 12.6%,
    // OLB 15.7%, DT 7.7%, NT 9.3%, ILB 7.2%, CB 10.5%, S 12.0%; designed
    // runs by the carrier RB 0.70%, QB 2.12%, WR 1.34%; catches WR 1.09%,
    // TE 1.23%, RB 1.14%.
    println!("fumble rates by position (plays, rate):");
    for (k, e) in &bypos {
        if e[0] >= 50.0 {
            println!("  {k:<24} {:>7.0}  {:.4}", e[0], e[1] / e[0]);
        }
    }
    println!("DPI accepted avg yards {:.2}", dpi[0] / dpi[1].max(1.0));
    println!("penalties a team game (flagged / accepted):");
    let mut pv: Vec<_> = pens.iter().collect();
    pv.sort_by(|a, b| b.1[1].total_cmp(&a.1[1]));
    for (k, e) in pv {
        println!("  {k:<32} {:.3} {:.3}", e[0] / tg, e[1] / tg);
    }
    println!(
        "last snap of a half (Q2 any, Q4 down 0-8): {} snaps, TD {:.3}",
        last_n[0],
        last_n[1] / last_n[0].max(1.0)
    );
    for (b, h) in hail.iter().enumerate() {
        println!(
            "  Hail Marys {}: {} snaps, TD {:.3}, INT {:.3}",
            ["under 40 out", "40-49 out", "50-59 out", "60-69 out", "70+ out"][b],
            h[0],
            h[1] / h[0].max(1.0),
            h[2] / h[0].max(1.0)
        );
    }
    println!("  Q4 passes from 40+ out with a lateral {:.3} of {} (real 0.22 of 74)", lat[1] / lat[0].max(1.0), lat[0]);
    for (k, v) in &last {
        println!("  {k:<28} {:.3}", v / last_n[0].max(1.0));
    }
    let g = results.len() as f64;
    println!(
        "timeouts a game, last 4:00: Q2 defense {:.3} offense {:.3}, Q4 defense {:.3} offense {:.3}; games with a Q2 defensive one {:.3} (real 1.346 1.544 1.834 0.773; 0.748)",
        tos[0] / g,
        tos[1] / g,
        tos[2] / g,
        tos[3] / g,
        to_games / g
    );
    let sh = |a: f64, b: f64| a / b.max(1.0);
    println!(
        "out of bounds, ahead in the last 5:00: designed runs {:.3}, catches {:.3}; first three quarters {:.3}, {:.3} (real 0.023 0.147; 0.058 0.213)",
        sh(oob[0][1], oob[0][0]),
        sh(oob[0][3], oob[0][2]),
        sh(oob[1][1], oob[1][0]),
        sh(oob[1][3], oob[1][2])
    );
    println!(
        "fake punts {:.4} a team game, converted {:.3}; fake field goals {:.4}, converted {:.3}; untimed snaps at 0:00 one in {:.0} games (real 0.0361 0.551; 0.0044 0.083; one in 68)",
        fakes[0] / tg,
        sh(fakes[1], fakes[0]),
        fakes[2] / tg,
        sh(fakes[3], fakes[2]),
        g / untimed.max(1.0)
    );
}

/// AFM_GAPS=1, lane EG5 (each level's rule book and the flags): every foul
/// kind flagged, accepted and declined with its yards and first downs
/// (against `REAL_FOULS` at the NFL), safeties on a foul in the end zone,
/// pass interference by depth against every target there, ineligible men
/// downfield by the call, the RPO throw's time, kickoffs dead without a
/// return, free plays and the pre-snap fouls by down. Per team game unless
/// named.
fn foul_counts(results: &[afm_engine::game::GameResult], level: Level) {
    use afm_engine::game::stats::PlayType as PT;
    use std::collections::BTreeMap;
    let tg = results.len() as f64 * 2.0;
    let nfl = level == Level::NFL;
    // By kind: [flagged, accepted, declined, accepted yards, first downs].
    let mut kinds: BTreeMap<&'static str, [f64; 5]> = BTreeMap::new();
    // Air yards bands: [targets, PI flagged, PI accepted, PI accepted yards].
    const BANDS: [(f64, &str); 6] =
        [(5.0, "1-5"), (10.0, "6-10"), (15.0, "11-15"), (20.0, "16-20"), (30.0, "21-30"), (99.0, "31+")];
    // Real NFL DPI accepted a target in each band (nflverse 2021-25: the
    // DPI's yards against every target's air yards).
    const REAL_PI: [f64; 6] = [0.0077, 0.0170, 0.0284, 0.0270, 0.0379, 0.0432];
    let mut bands = [[0.0f64; 4]; 6];
    // Ineligible men downfield: [on RPO calls, on other calls]; RPO throws:
    // [throws, time to throw sum], and the times.
    let mut inel = [0.0f64; 4];
    let mut inel_calls: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    let mut rpo_t: Vec<f64> = Vec::new();
    // Kickoffs: [kickoffs, dead at the spot without a return, booked fouls].
    let mut ko = [0.0f64; 3];
    // Free plays (a live-ball offside and the play ran): [plays, accepted].
    let mut free = [0.0f64; 2];
    // Safeties on an accepted foul by the offense.
    let mut foul_safety = 0.0f64;
    // Snaps by down and the pre-snap fouls flagged on them:
    // [snaps, false starts, delays, offside + neutral zone].
    let mut by_down = [[0.0f64; 4]; 4];
    let mut backups = 0.0;
    // Kick snaps (punts, field goals, tries) and the fouls before them:
    // [snaps, false starts, delays, jumps]; the hard count on fourth and
    // short: [tried, drew the defense, own false start, delay].
    let mut kick_snaps = [[0.0f64; 4]; 3];
    let mut gambits = [0.0f64; 4];
    // The defense's hit fouls by where they came, flagged: [unnecessary
    // roughness, targeting] at the catch point (an incomplete pass), on a
    // scramble (a slide or the tackle) and on any other play (the tackle).
    let mut hits = [[0.0f64; 2]; 3];
    let ff = |r: &afm_engine::game::stats::PlayRecord, k: &str| r.penalties.iter().any(|p| p.0.name() == k);
    for g in results {
        for p in &g.stats.plays {
            for &(kind, team, _, yds, acc) in &p.penalties {
                let e = kinds.entry(kind.name()).or_default();
                e[0] += 1.0;
                let hit = match kind.name() {
                    "unnecessary roughness" => Some(0),
                    "targeting" => Some(1),
                    _ => None,
                };
                if let Some(h) = hit
                    && team != p.offense
                {
                    let w = if p.kind == PT::Pass && !p.complete {
                        0
                    } else if p.kind == PT::Scramble {
                        1
                    } else {
                        2
                    };
                    hits[w][h] += 1.0;
                }
                if acc {
                    e[1] += 1.0;
                    e[3] += yds.abs();
                    if p.first_down {
                        e[4] += 1.0;
                    }
                    if p.safety && team == p.offense {
                        foul_safety += 1.0;
                    }
                } else {
                    e[2] += 1.0;
                }
            }
            let scrim =
                matches!(p.kind, PT::Run | PT::Pass | PT::Sack | PT::Scramble | PT::NoPlay | PT::Kneel | PT::Spike);
            {
                use afm_engine::game::stats::{KICK_FG, KICK_PUNT, KICK_TRY};
                let unit = match p.kind {
                    PT::Punt => Some(0),
                    PT::FieldGoal => Some(1),
                    PT::ExtraPoint => Some(2),
                    PT::NoPlay if p.call == KICK_PUNT => Some(0),
                    PT::NoPlay if p.call == KICK_FG => Some(1),
                    PT::NoPlay if p.call == KICK_TRY => Some(2),
                    _ => None,
                };
                if let Some(u) = unit {
                    let k = &mut kick_snaps[u];
                    k[0] += 1.0;
                    k[1] += ff(p, "false start") as u8 as f64;
                    k[2] += ff(p, "delay of game") as u8 as f64;
                    k[3] += (ff(p, "offside") || ff(p, "neutral zone infraction")) as u8 as f64;
                }
                if p.kind == PT::NoPlay && p.call == "hard count on fourth down" {
                    gambits[0] += 1.0;
                    gambits[1] += p.penalties.iter().any(|f| f.1 != p.offense) as u8 as f64;
                    gambits[2] += ff(p, "false start") as u8 as f64;
                    gambits[3] += ff(p, "delay of game") as u8 as f64;
                }
            }
            if scrim && (1..=4).contains(&p.down) && p.call != afm_engine::game::stats::KICK_TRY {
                let d = &mut by_down[p.down as usize - 1];
                d[0] += 1.0;
                d[1] += ff(p, "false start") as u8 as f64;
                d[2] += ff(p, "delay of game") as u8 as f64;
                d[3] += (ff(p, "offside") || ff(p, "neutral zone infraction")) as u8 as f64;
                if p.call == "delay of game to back up the punt" {
                    backups += 1.0;
                }
            }
            if p.kind == PT::Pass && !p.throw_away && p.passer.is_some() && p.air_yards > 0.0 {
                let b = BANDS.iter().position(|b| p.air_yards <= b.0).unwrap_or(5);
                bands[b][0] += 1.0;
                for &(kind, _, _, yds, acc) in &p.penalties {
                    if kind.name() == "pass interference" {
                        bands[b][1] += 1.0;
                        if acc {
                            bands[b][2] += 1.0;
                            bands[b][3] += yds;
                        }
                    }
                }
            }
            if ff(p, "ineligible man downfield") {
                if !p.call.contains("RPO") {
                    *inel_calls.entry(p.call.clone()).or_default() += 1.0;
                }
                inel[if p.call.contains("RPO") {
                    0
                } else if p.call.contains("screen") {
                    2
                } else if p.play_action {
                    3
                } else {
                    1
                }] += 1.0;
            }
            if p.kind == PT::Pass && p.call.contains("RPO") && p.passer.is_some() && !p.throw_away {
                rpo_t.push(p.time_to_throw);
            }
            if p.kind == PT::Kickoff && p.call == "kickoff" {
                ko[0] += 1.0;
                if p.returner.is_none() && !p.touchback && !p.fair_catch && !p.turnover {
                    ko[1] += 1.0;
                }
                if p.penalties.iter().any(|x| x.0.name().starts_with("kickoff")) {
                    ko[2] += 1.0;
                }
            }
            if ff(p, "offside") && p.kind != PT::NoPlay {
                free[0] += 1.0;
                if p.penalties.iter().any(|x| x.0.name() == "offside" && x.4) {
                    free[1] += 1.0;
                }
            }
        }
    }
    println!("\n=== Rule books and flags (EG5) {tg} team games ===");
    println!(
        "  {:<34} {:>7} {:>7} {:>7} {:>6} {:>6}   real NFL scrimmage: flagged accepted yards",
        "per team game", "flagged", "accepted", "declined", "yds", "1st dn"
    );
    let mut list: Vec<_> = kinds.iter().collect();
    list.sort_by(|a, b| b.1[1].total_cmp(&a.1[1]));
    for (k, e) in list {
        let real = REAL_FOULS.iter().find(|r| r.0 == *k).filter(|_| nfl);
        println!(
            "  {k:<34} {:>7.3} {:>7.3} {:>7.3} {:>6.1} {:>6.2}   {}",
            e[0] / tg,
            e[1] / tg,
            e[2] / tg,
            e[3] / e[1].max(1.0),
            e[4] / e[1].max(1.0),
            real.map_or(String::new(), |r| format!("{:.3} {:.3} {:.1}", r.2, r.1, r.3))
        );
    }
    println!("  safeties on an accepted foul by the offense {:.4} a team game (real NFL 0.0044)", foul_safety / tg);
    println!(
        "  the defense's hit fouls flagged (unnecessary roughness, targeting): at the catch point {:.3} {:.3}, on a scramble {:.3} {:.3}, on any other play (the tackle) {:.3} {:.3}",
        hits[0][0] / tg,
        hits[0][1] / tg,
        hits[1][0] / tg,
        hits[1][1] / tg,
        hits[2][0] / tg,
        hits[2][1] / tg
    );
    println!(
        "  pass interference by air yards (targets a team game, PI flagged and accepted a target, accepted yards; real accepted a target):"
    );
    for (b, (_, name)) in bands.iter().zip(BANDS) {
        println!(
            "    {name:<6} {:>6.2}  {:.4} {:.4} {:>5.1}   {}",
            b[0] / tg,
            b[1] / b[0].max(1.0),
            b[2] / b[0].max(1.0),
            b[3] / b[2].max(1.0),
            if nfl {
                format!("{:.4}", REAL_PI[BANDS.iter().position(|x| x.1 == name).unwrap()])
            } else {
                String::new()
            }
        );
    }
    rpo_t.sort_by(f64::total_cmp);
    let q = |f: f64| rpo_t.get(((rpo_t.len() as f64 - 1.0) * f).round().max(0.0) as usize).copied().unwrap_or(0.0);
    println!(
        "  ineligible men downfield flagged: on RPO calls {:.3}, screens {:.3}, play action {:.3}, other calls {:.3} a team game; RPO throws {:.2} a team game, time to throw p25 {:.2} p50 {:.2} p75 {:.2} s",
        inel[0] / tg,
        inel[2] / tg,
        inel[3] / tg,
        inel[1] / tg,
        rpo_t.len() as f64 / tg,
        q(0.25),
        q(0.5),
        q(0.75)
    );
    let mut calls: Vec<(String, f64)> = inel_calls.into_iter().collect();
    calls.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    println!(
        "    on other calls, the most: {}",
        calls.iter().take(8).map(|(c, n)| format!("{c} {:.3}", n / tg)).collect::<Vec<_>>().join(", ")
    );
    println!(
        "  kickoffs {:.2} a team game: dead without a return or touchback {:.3} a team game, kickoff fouls booked {:.3}",
        ko[0] / tg,
        ko[1] / tg,
        ko[2] / tg
    );
    println!(
        "  free plays (live-ball offside, the play ran) {:.3} a team game, the offside taken {:.3} of them",
        free[0] / tg,
        free[1] / free[0].max(1.0)
    );
    let aw: Vec<f64> =
        afm_engine::game::scrimmage::AWAY.iter().map(|x| x.load(std::sync::atomic::Ordering::Relaxed) as f64).collect();
    let ie: Vec<f64> = afm_engine::game::scrimmage::INELIGIBLE
        .iter()
        .map(|x| x.load(std::sync::atomic::Ordering::Relaxed) as f64)
        .collect();
    println!(
        "    linemen past the limit at a throw past the line a team game: run block {:.3}, pass block on a man {:.3}, \
         looking for work {:.3}, on a block begun past it {:.3}, escort {:.3}, other {:.3}; on dropbacks {:.3}: passer within 2.5 of the line {:.3}, \
         after 3.5 s {:.3}, scramble drill {:.3}",
        ie[0] / tg,
        ie[1] / tg,
        ie[2] / tg,
        ie[3] / tg,
        ie[4] / tg,
        ie[5] / tg,
        ie[6] / tg,
        ie[7] / tg,
        ie[8] / tg,
        ie[9] / tg
    );
    println!(
        "    of them on dropbacks: in a pass block {:.3} (a helper {:.3}), in a run-style block {:.3}, on a finished block {:.3}, \
         in no block {:.3}, over a yard past the limit {:.3}; on RPOs: in a block {:.3}, in none {:.3}, over a yard past {:.3}; \
         on dropbacks every lineman past the limit on a block begun inside it {:.3}, pass blocks driving the man back {:.3}",
        ie[10] / tg,
        ie[13] / tg,
        ie[11] / tg,
        ie[12] / tg,
        ie[14] / tg,
        ie[18] / tg,
        ie[15] / tg,
        ie[16] / tg,
        ie[17] / tg,
        ie[19] / tg,
        ie[20] / tg
    );
    println!(
        "    dropback pass blocks begun past the limit: begun within a yard past it {:.3}, farther {:.3}; on a man not rushing {:.3}; \
         begun under 0.3 s before the throw {:.3}; on a man off the line {:.3}; the lineman 3+ yards past the line {:.3}; the passer near the line {:.3}",
        ie[21] / tg,
        ie[22] / tg,
        ie[23] / tg,
        ie[24] / tg,
        ie[25] / tg,
        ie[26] / tg,
        ie[27] / tg
    );
    let flicks = aw[2].max(1.0);
    println!(
        "  throwaways a team game: in the pocket {:.3}, outside it {:.3} (real NFL 0.549 and 0.829, FTN 2022-24); \
         outside with a rusher on him (a flick) {:.3}, of them over 20, 25, 30, 35 yards {:.3} {:.3} {:.3} {:.3}, short of the line {:.3}; \
         in the pocket nowhere near a receiver {:.3} of them; grounding flagged {:.3} a team game (real 0.080, 35% from outside the pocket)",
        aw[0] / tg,
        aw[1] / tg,
        aw[2] / tg,
        aw[4] / flicks,
        aw[5] / flicks,
        aw[6] / flicks,
        aw[7] / flicks,
        aw[3] / tg,
        aw[8] / aw[0].max(1.0),
        aw[9] / tg
    );
    // Real NFL per snap from scrimmage by down (nflverse 2021-25, no plays
    // included): false starts, delays, defensive offside and neutral zone.
    const REAL_DOWN: [[f64; 3]; 4] =
        [[0.0132, 0.0014, 0.0069], [0.0125, 0.0022, 0.0075], [0.0232, 0.0053, 0.0133], [0.0620, 0.0816, 0.0391]];
    println!("  pre-snap fouls a snap by down (false start, delay, offside or neutral zone; real NFL):");
    for (k, d) in by_down.iter().enumerate() {
        let r = REAL_DOWN[k];
        println!(
            "    down {} {:>6.2} snaps  {:.4} {:.4} {:.4}   {:.4} {:.4} {:.4}",
            k + 1,
            d[0] / tg,
            d[1] / d[0].max(1.0),
            d[2] / d[0].max(1.0),
            d[3] / d[0].max(1.0),
            r[0],
            r[1],
            r[2]
        );
    }
    println!(
        "    delays on fourth down {:.3} a team game, of them backing up the punt {:.3} (real NFL 0.142 and 0.066, FBS 0.148 and 0.052; nflverse 2021-25, cfbfastR 2023-24)",
        by_down[3][2] / tg,
        backups / tg
    );
    // Real NFL 2021-25 a kick snap, from the formation on each no-play
    // (nflverse): false starts, delays not backing up a punt, jumps.
    const REAL_KICK: [[f64; 3]; 3] = [[0.0128, 0.0055, 0.0015], [0.0039, 0.0043, 0.0026], [0.0036, 0.0036, 0.0015]];
    println!("  fouls before a kick, a kick snap (false start, delay, jump; real NFL 2021-25):");
    for (u, name) in ["punts", "field goals", "tries"].iter().enumerate() {
        let (k, r) = (kick_snaps[u], REAL_KICK[u]);
        println!(
            "    {name:<12} {:>5.2} a team game  {:.4} {:.4} {:.4}   {:.4} {:.4} {:.4}",
            k[0] / tg,
            k[1] / k[0].max(1.0),
            k[2] / k[0].max(1.0),
            k[3] / k[0].max(1.0),
            r[0],
            r[1],
            r[2]
        );
    }
    println!(
        "  hard counts on fourth and short before a kick {:.3} a team game: drew the defense {:.3}, own false start {:.3}, delay {:.3} (real NFL delays 0.024 a team game)",
        gambits[0] / tg,
        gambits[1] / tg,
        gambits[2] / tg,
        gambits[3] / tg
    );
}

/// AFM_GAPS=1, lane EG7 (the quarterback off script): dropbacks in and out
/// of the pocket, as FTN charts it (`PlayRecord::out_of_pocket`: outside the
/// tackle box at the throw or the sack, on the scramble drill, or
/// scrambling), beside real NFL regular seasons 2023-25 (nflverse
/// play-by-play with FTN charting, time to throw from Next Gen Stats
/// participation; `afm/engine-gaps/tools/real_off_script.py.txt`). Real
/// pressure is Next Gen Stats' flag (30% of dropbacks), wider than the
/// engine's (PFR's, 22%), so its rows compare in shape only. The real
/// column is the NFL's at every level.
fn off_script_counts(results: &[afm_engine::game::GameResult]) {
    use afm_engine::game::stats::PlayType as PT;
    let tg = results.len() as f64 * 2.0;
    // Real: share of dropbacks, sack, scramble, throws, throwaways of
    // throws, completion of aimed throws, yards an attempt, air yards of
    // aimed throws, yards a dropback, EPA a dropback; time to throw mean,
    // p10, p50, p90 and the share after 3.94 s.
    const ROWS: [(&str, [f64; 15]); 5] = [
        ("all", [1.0, 0.066, 0.052, 0.882, 0.041, 0.674, 7.05, 7.80, 6.18, 0.035, 2.65, 1.6, 2.5, 3.9, 0.096]),
        (
            "in the pocket",
            [0.812, 0.061, 0.005, 0.933, 0.019, 0.682, 7.18, 7.57, 6.31, 0.039, 2.40, 1.5, 2.3, 3.4, 0.029],
        ),
        (
            "out of the pocket",
            [0.188, 0.087, 0.256, 0.658, 0.175, 0.610, 6.26, 9.47, 5.61, 0.016, 4.18, 2.5, 4.0, 6.1, 0.505],
        ),
        (
            "pressured, in",
            [0.193, 0.257, 0.011, 0.732, 0.062, 0.548, 6.01, 8.31, 2.66, -0.435, 2.75, 1.8, 2.6, 3.7, 0.071],
        ),
        (
            "pressured, out",
            [0.107, 0.152, 0.194, 0.654, 0.262, 0.524, 4.95, 9.83, 3.90, -0.227, 4.58, 2.8, 4.4, 6.5, 0.626],
        ),
    ];
    #[derive(Default)]
    struct Acc {
        db: f64,
        sack: f64,
        scr: f64,
        att: f64,
        away: f64,
        aim: f64,
        comp_aim: f64,
        yds: f64,
        air: f64,
        ydb: f64,
        epa: f64,
        boot: f64,
        ttt: Vec<f64>,
    }
    let mut acc: Vec<Acc> = (0..ROWS.len()).map(|_| Acc::default()).collect();
    for r in results {
        for p in &r.stats.plays {
            if p.nullified || !matches!(p.kind, PT::Pass | PT::Sack | PT::Scramble) {
                continue;
            }
            let out = p.out_of_pocket;
            let mut rows = vec![0, if out { 2 } else { 1 }];
            if p.pressured {
                rows.push(if out { 4 } else { 3 });
            }
            for k in rows {
                let a = &mut acc[k];
                a.db += 1.0;
                a.sack += (p.kind == PT::Sack) as u8 as f64;
                a.scr += (p.kind == PT::Scramble) as u8 as f64;
                a.ydb += p.yards;
                a.epa += p.epa;
                a.boot += p.call.contains("boot") as u8 as f64;
                if p.kind == PT::Pass {
                    a.att += 1.0;
                    a.away += p.throw_away as u8 as f64;
                    a.yds += if p.complete { p.yards } else { 0.0 };
                    if !p.throw_away {
                        a.aim += 1.0;
                        a.comp_aim += p.complete as u8 as f64;
                        a.air += p.air_yards;
                    }
                    a.ttt.push(p.time_to_throw);
                }
            }
        }
    }
    // Sacks: when they come (seconds after the snap) and the passers who
    // slipped a rusher's try (a missed tackle on him) on a throw or a sack.
    let mut sacks: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
    let mut slipped = [0.0f64; 2];
    for r in results {
        for p in &r.stats.plays {
            if p.nullified || !matches!(p.kind, PT::Pass | PT::Sack) {
                continue;
            }
            let k = p.out_of_pocket as usize;
            if p.kind == PT::Sack {
                sacks[k].push(p.time_to_throw);
            }
            if p.passer.is_some_and(|q| p.missed_tackles.iter().any(|&(_, by)| by == q)) {
                slipped[k] += 1.0;
            }
        }
    }
    // When the pressure comes (Next Gen Stats: on average 2.9 s after the
    // snap; a quick one inside 2.5 s).
    let mut prs: Vec<f64> = Vec::new();
    for r in results {
        for p in &r.stats.plays {
            if !p.nullified && p.pressured && matches!(p.kind, PT::Pass | PT::Sack | PT::Scramble) {
                prs.push(p.pressure_at as f64);
            }
        }
    }
    prs.sort_by(f64::total_cmp);
    let pq = |x: f64| prs.get(((prs.len().max(1) - 1) as f64 * x) as usize).copied().unwrap_or(0.0);
    println!(
        "  pressure at {:.2} s (real 2.9), p10 {:.2}, p50 {:.2}, p90 {:.2}; inside 2.5 s {:.3}",
        prs.iter().sum::<f64>() / prs.len().max(1) as f64,
        pq(0.1),
        pq(0.5),
        pq(0.9),
        prs.iter().filter(|&&x| x <= 2.5).count() as f64 / prs.len().max(1) as f64
    );
    for (k, name) in ["in the pocket", "out of the pocket"].iter().enumerate() {
        let v = &mut sacks[k];
        v.sort_by(f64::total_cmp);
        let q = |x: f64| v.get(((v.len().max(1) - 1) as f64 * x) as usize).copied().unwrap_or(0.0);
        println!(
            "  sacks {name}: {:.3} a team game, at {:.2} s (p10 {:.2}, p50 {:.2}, p90 {:.2}); passers who slipped a rusher on a throw or sack {:.3} a team game",
            v.len() as f64 / tg,
            v.iter().sum::<f64>() / v.len().max(1) as f64,
            q(0.1),
            q(0.5),
            q(0.9),
            slipped[k] / tg
        );
    }
    let all = acc[0].db.max(1.0);
    println!(
        "\nThe quarterback off script (AFM_GAPS, lane EG7), engine (real NFL 2023-25): {:.2} dropbacks a team game ({:.2}), out of the pocket {:.2} ({:.2})",
        acc[0].db / tg,
        37.27,
        acc[2].db / tg,
        7.00
    );
    println!(
        "  {:>18} {:>13} {:>13} {:>13} {:>13} {:>13} {:>13} {:>11} {:>11} {:>11} {:>13}",
        "", "share", "sack", "scramble", "throws", "throwaways", "comp aimed", "Y/A", "air aimed", "yds/db", "EPA/db"
    );
    for (k, (name, real)) in ROWS.iter().enumerate() {
        let a = &mut acc[k];
        let n = a.db.max(1.0);
        let v = [
            a.db / all,
            a.sack / n,
            a.scr / n,
            a.att / n,
            a.away / a.att.max(1.0),
            a.comp_aim / a.aim.max(1.0),
            a.yds / a.att.max(1.0),
            a.air / a.aim.max(1.0),
            a.ydb / n,
            a.epa / n,
        ];
        let mut line = format!("  {name:>18}");
        for (j, x) in v.iter().enumerate() {
            let w = if (6..9).contains(&j) { 5 } else { 6 };
            line += &format!(" {:>w$.3} ({:>w$.3})", x, real[j], w = w);
        }
        println!("{line}");
        a.ttt.sort_by(f64::total_cmp);
        let q = |x: f64| a.ttt.get(((a.ttt.len().max(1) - 1) as f64 * x) as usize).copied().unwrap_or(0.0);
        let mean = a.ttt.iter().sum::<f64>() / a.ttt.len().max(1) as f64;
        let late = a.ttt.iter().filter(|&&x| x > 3.94).count() as f64 / a.ttt.len().max(1) as f64;
        println!(
            "  {:>18} time to throw {:.2} ({:.2}), p10 {:.2} ({:.2}), p50 {:.2} ({:.2}), p90 {:.2} ({:.2}), after 3.94 s {:.3} ({:.3}); boots {:.3} of these dropbacks",
            "",
            mean,
            real[10],
            q(0.1),
            real[11],
            q(0.5),
            real[12],
            q(0.9),
            real[13],
            late,
            real[14],
            a.boot / n
        );
    }
}

/// AFM_GAPS=1, lane EG6 (the red zone): throws, dropbacks and runs by how
/// far the snap is from the goal line, beside real NFL regular seasons
/// (nflverse play-by-play 2021-25; throws without throwaways and spikes,
/// 2022-25 with FTN's throwaway flag; coverage, rushers, time to throw and
/// pressure from Next Gen Stats participation 2023-25). The real column is
/// the NFL's at every level.
fn red_zone_counts(results: &[afm_engine::game::GameResult], level: Level) {
    use afm_engine::game::stats::PlayType as PT;
    let tg = results.len() as f64 * 2.0;
    const BANDS: [(f64, &str); 5] = [(5.0, "1-5"), (10.0, "6-10"), (15.0, "11-15"), (20.0, "16-20"), (100.0, "21+")];
    // Real per band: throws a team game, completion, TD a throw, INT a
    // throw, air yards, share thrown at or past the goal line.
    const REAL_T: [[f64; 6]; 5] = [
        [0.865, 0.561, 0.480, 0.0255, 2.13, 0.725],
        [0.961, 0.564, 0.295, 0.0239, 4.41, 0.463],
        [1.111, 0.614, 0.165, 0.0232, 5.64, 0.292],
        [1.174, 0.659, 0.111, 0.0266, 6.45, 0.195],
        [27.399, 0.686, 0.015, 0.0233, 8.25, 0.015],
    ];
    // Real per band, dropbacks: sack rate, scramble rate, throwaway share
    // of throws, man coverage, Cover 0, rushers, time to throw, pressure.
    const REAL_D: [[f64; 8]; 5] = [
        [0.0515, 0.0460, 0.0784, 0.751, 0.402, 4.72, 2.64, 0.335],
        [0.0662, 0.0547, 0.0756, 0.554, 0.142, 4.27, 2.70, 0.333],
        [0.0596, 0.0571, 0.0541, 0.473, 0.073, 4.26, 2.61, 0.298],
        [0.0582, 0.0562, 0.0481, 0.435, 0.041, 4.24, 2.52, 0.301],
        [0.0654, 0.0485, 0.0364, 0.379, 0.019, 4.26, 2.66, 0.298],
    ];
    // Real per band, designed runs: yards a carry, TD a carry, and the
    // dropback share of snaps.
    // On first down too (2021-25), and Cover 0's rushers and sacks (2023-25).
    const REAL_R: [[f64; 6]; 5] = [
        [1.06, 0.410, 0.451, 0.311, 5.26, 0.042],
        [2.85, 0.117, 0.561, 0.384, 5.15, 0.069],
        [3.58, 0.056, 0.565, 0.436, 5.22, 0.050],
        [3.96, 0.035, 0.583, 0.455, 5.49, 0.011],
        [4.62, 0.006, 0.618, 0.519, 5.60, 0.076],
    ];
    // Per band: throws, completions, TDs, INTs, air yards, end zone throws,
    // throwaways, dropbacks, sacks, scrambles, man, Cover 0, rushers (and
    // dropbacks with a count), time to throw (and throws with one),
    // pressured, runs, run yards, run TDs, contested throws and their
    // catches, separation, drops, breakups, Cover 0's rushers (and dropbacks
    // with a count), sacks and dropbacks, first-down dropbacks and snaps.
    let mut b = [[0.0f64; 31]; 5];
    // Inside the 20: [man, zone] x [inside the 10, 11-20] x [throws, comp, TD];
    // end zone throws and short of it: [throws, comp, TD].
    let mut mz = [[0.0f64; 3]; 6];
    let mut ez = [[0.0f64; 3]; 2];
    // Targets by the target's route inside the 10 and from 11 to 20:
    // throws, completions, TDs, air yards.
    let mut routes: [std::collections::BTreeMap<&str, [f64; 4]>; 3] = Default::default();
    // Goal-line snaps: third and fourth and goal from the 1 and from the 2
    // (garbage time and overtime out, as `decide_fit` reads them), and
    // two-point tries: [snaps, scores, dropbacks, their scores, Cover 0 and
    // man of the dropbacks, runs against Cover 0, their scores, runs
    // against man, their scores].
    let mut gl = [[0.0f64; 10]; 3];
    for r in results {
        for p in &r.stats.plays {
            if p.nullified {
                continue;
            }
            let row = if p.kind == PT::TwoPoint {
                Some(2)
            } else if matches!(p.kind, PT::Run | PT::Pass | PT::Sack | PT::Scramble)
                && matches!(p.down, 3 | 4)
                && p.quarter <= 4
                && p.garbage == [0, 0]
                && p.togo >= p.yardline_100 - 0.01
                && p.togo <= 2.0
            {
                Some(usize::from(p.togo > 1.0))
            } else {
                None
            };
            if let Some(row) = row {
                let g = &mut gl[row];
                let one = |v: bool| f64::from(u8::from(v));
                let score = p.touchdown && !p.defense_td && !p.turnover;
                let dropback = p.passer.is_some() || !p.sackers.is_empty() || p.scramble;
                let zero = p.defense_call.split(',').next() == Some("Cover 0");
                g[0] += 1.0;
                g[1] += one(score);
                if dropback {
                    g[2] += 1.0;
                    g[3] += one(score);
                    g[4] += one(zero);
                    g[5] += one(p.coverage_man);
                } else if zero {
                    g[6] += 1.0;
                    g[7] += one(score);
                } else if p.coverage_man {
                    g[8] += 1.0;
                    g[9] += one(score);
                }
            }
            let k = BANDS.iter().position(|x| p.yardline_100 <= x.0).unwrap_or(4);
            let x = &mut b[k];
            let one = |v: bool| f64::from(u8::from(v));
            match p.kind {
                PT::Pass | PT::Sack | PT::Scramble => {
                    x[7] += 1.0;
                    x[8] += one(p.kind == PT::Sack);
                    x[9] += one(p.kind == PT::Scramble);
                    x[10] += one(p.coverage_man);
                    let zero = p.defense_call.split(',').next() == Some("Cover 0");
                    x[11] += one(zero);
                    if zero {
                        x[27] += one(p.kind == PT::Sack);
                        x[28] += 1.0;
                        if p.rushed != 0 {
                            x[25] += f64::from(p.rushed.count_ones());
                            x[26] += 1.0;
                        }
                    }
                    if p.down == 1 {
                        x[29] += 1.0;
                        x[30] += 1.0;
                    }
                    if p.rushed != 0 {
                        x[12] += f64::from(p.rushed.count_ones());
                        x[13] += 1.0;
                    }
                    x[16] += one(p.pressured);
                    if p.kind == PT::Pass {
                        x[6] += one(p.throw_away);
                        if p.time_to_throw > 0.0 {
                            x[14] += p.time_to_throw;
                            x[15] += 1.0;
                        }
                        if !p.throw_away {
                            x[0] += 1.0;
                            x[1] += one(p.complete);
                            x[2] += one(p.complete && p.touchdown);
                            x[3] += one(p.interception);
                            // Contested (under a yard of separation at the
                            // throw's end) and its catches; separation summed.
                            let tight = p.separation < 1.0;
                            x[20] += one(tight);
                            x[21] += one(tight && p.complete);
                            x[22] += p.separation;
                            x[23] += one(p.drop);
                            x[24] += one(p.pbu.is_some());
                            // nflverse measures air yards to the goal line at most.
                            x[4] += p.air_yards.min(p.yardline_100);
                            let at_goal = p.air_yards >= p.yardline_100;
                            x[5] += one(at_goal);
                            if p.yardline_100 > 20.0 {
                                let m = &mut mz[4 + usize::from(!p.coverage_man)];
                                m[0] += 1.0;
                                m[1] += one(p.complete);
                                m[2] += one(p.complete && p.touchdown);
                            } else {
                                let m = &mut mz[usize::from(!p.coverage_man) * 2 + usize::from(p.yardline_100 > 10.0)];
                                let e = &mut ez[usize::from(!at_goal)];
                                for t in [m, e] {
                                    t[0] += 1.0;
                                    t[1] += one(p.complete);
                                    t[2] += one(p.complete && p.touchdown);
                                }
                                let name: &str = p.route;
                                let name = if name == "out" && p.air_yards >= 8.0 { "out 8+" } else { name };
                                let ks: &[usize] = if p.yardline_100 <= 10.0 && p.coverage_man {
                                    &[0, 2]
                                } else if p.yardline_100 <= 10.0 {
                                    &[0]
                                } else {
                                    &[1]
                                };
                                for &k in ks {
                                    let t = routes[k].entry(name).or_default();
                                    t[0] += 1.0;
                                    t[1] += one(p.complete);
                                    t[2] += one(p.complete && p.touchdown);
                                    t[3] += p.air_yards.min(p.yardline_100);
                                }
                            }
                        }
                    }
                }
                PT::Run => {
                    x[17] += 1.0;
                    x[30] += one(p.down == 1);
                    x[18] += p.yards;
                    x[19] += one(p.touchdown);
                }
                _ => {}
            }
        }
    }
    let d = |a: f64, b: f64| a / b.max(1.0);
    println!(
        "\nThe red zone (AFM_GAPS, lane EG6) by yards to the goal, engine (real NFL 2021-25){}",
        if level == Level::NFL { "" } else { "; the real column is the NFL's" }
    );
    println!(
        "  throws (no throwaways): a team game, completion, TD a throw, INT a throw, air yards, at or past the goal line"
    );
    for (k, (_, name)) in BANDS.iter().enumerate() {
        let (x, r) = (&b[k], &REAL_T[k]);
        println!(
            "    {name:>5}: {:.3} ({:.3})  {:.3} ({:.3})  {:.3} ({:.3})  {:.4} ({:.4})  {:.2} ({:.2})  {:.3} ({:.3})",
            x[0] / tg,
            r[0],
            d(x[1], x[0]),
            r[1],
            d(x[2], x[0]),
            r[2],
            d(x[3], x[0]),
            r[3],
            d(x[4], x[0]),
            r[4],
            d(x[5], x[0]),
            r[5]
        );
    }
    // Real contested share of throws (FTN charting 2022-25): 22.7%, 20.0%,
    // 17.1%, 15.7%, 13.4%.
    const REAL_C: [f64; 5] = [0.227, 0.200, 0.171, 0.157, 0.134];
    println!(
        "  throws under a yard of separation (FTN's contested share), their catch rate, the rest's, mean separation, drops, breakups"
    );
    for (k, (_, name)) in BANDS.iter().enumerate() {
        let x = &b[k];
        println!(
            "    {name:>5}: {:.3} ({:.3})  {:.3}  {:.3}  {:.2}  {:.3}  {:.3}",
            d(x[20], x[0]),
            REAL_C[k],
            d(x[21], x[20]),
            d(x[1] - x[21], x[0] - x[20]),
            d(x[22], x[0]),
            d(x[23], x[0]),
            d(x[24], x[0])
        );
    }
    println!("  dropbacks: sacks, scrambles, throwaways of throws, man, Cover 0, rushers, time to throw, pressured");
    for (k, (_, name)) in BANDS.iter().enumerate() {
        let (x, r) = (&b[k], &REAL_D[k]);
        println!(
            "    {name:>5}: {:.4} ({:.4})  {:.4} ({:.4})  {:.4} ({:.4})  {:.3} ({:.3})  {:.3} ({:.3})  {:.2} ({:.2})  {:.2} ({:.2})  {:.3} ({:.3})",
            d(x[8], x[7]),
            r[0],
            d(x[9], x[7]),
            r[1],
            d(x[6], x[0] + x[6]),
            r[2],
            d(x[10], x[7]),
            r[3],
            d(x[11], x[7]),
            r[4],
            d(x[12], x[13]),
            r[5],
            d(x[14], x[15]),
            r[6],
            d(x[16], x[7]),
            r[7]
        );
    }
    println!(
        "  designed runs: yards a carry, TD a carry; dropbacks of snaps, on first down; Cover 0's rushers and sacks"
    );
    for (k, (_, name)) in BANDS.iter().enumerate() {
        let (x, r) = (&b[k], &REAL_R[k]);
        println!(
            "    {name:>5}: {:.2} ({:.2})  {:.3} ({:.3})  {:.3} ({:.3})  {:.3} ({:.3})  {:.2} ({:.2})  {:.3} ({:.3})",
            d(x[18], x[17]),
            r[0],
            d(x[19], x[17]),
            r[1],
            d(x[7], x[7] + x[17]),
            r[2],
            d(x[29], x[30]),
            r[3],
            d(x[25], x[26]),
            r[4],
            d(x[27], x[28]),
            r[5]
        );
    }
    // Real inside the 20 (no throwaways, 2023-25 for coverage, 2022-25 for
    // the end zone): [man, zone] x [inside the 10, 11-20] completion and TD
    // a throw; at or past the goal line and short of it.
    const REAL_MZ: [[f64; 2]; 6] =
        [[0.540, 0.409], [0.602, 0.160], [0.613, 0.347], [0.682, 0.124], [0.619, 0.020], [0.729, 0.013]];
    const REAL_EZ: [[f64; 2]; 2] = [[0.418, 0.418], [0.725, 0.134]];
    println!("  by coverage, completion and TD a throw:");
    for (k, name) in ["man, inside the 10", "man, 11-20", "zone, inside the 10", "zone, 11-20", "man, 21+", "zone, 21+"]
        .iter()
        .enumerate()
    {
        let (x, r) = (&mz[k], &REAL_MZ[k]);
        println!(
            "    {name:>19}: {:.3} ({:.3})  {:.3} ({:.3})  n {:.0}",
            d(x[1], x[0]),
            r[0],
            d(x[2], x[0]),
            r[1],
            x[0]
        );
    }
    // Real targets by route (NGS, 2023-25, no throwaways): inside the 10
    // quick out 23.7% (65.6% caught, 40.0% TDs), corner 10.9% (41.5%,
    // 41.2%), drag 10.5% (62.4%, 37.0%), slant 9.8% (58.3%, 45.2%), hitch
    // and curl 8.5% (58.6%, 37.8%), in and dig 7.4% (49.8%, 44.7%), go 6.8%
    // (31.5%, 31.5%), screen 6.1% (85.0%, 22.8%), deep out 5.9% (42.5%,
    // 40.8%), post 5.1% (50.7%, 50.0%), swing 2.3%; from 11 to 20 quick out
    // 18.2% (78.5%), screen 13.9% (90.2%), hitch and curl 11.7% (73.1%), go
    // 10.1% (32.5%), drag 7.5% (71.7%), slant 6.6% (68.0%), dig 6.3%
    // (54.9%), corner 6.2% (37.9%), post 6.0% (48.0%), deep out 4.7%
    // (40.1%), swing 4.1% (86.4%).
    for (k, name) in ["inside the 10", "11-20", "inside the 10 in man"].iter().enumerate() {
        let n: f64 = routes[k].values().map(|x| x[0]).sum();
        let mut v: Vec<_> = routes[k].iter().collect();
        v.sort_by(|a, b| b.1[0].total_cmp(&a.1[0]));
        let line: Vec<String> = v
            .iter()
            .take(12)
            .map(|(r, x)| {
                format!(
                    "{r} {:.1}% ({:.1}%, {:.1}%, air {:.1})",
                    100.0 * x[0] / n,
                    100.0 * d(x[1], x[0]),
                    100.0 * d(x[2], x[0]),
                    d(x[3], x[0])
                )
            })
            .collect();
        println!("  targets by route {name} (share, caught, TDs): {}", line.join("; "));
    }
    let all = ez[0][0] + ez[1][0];
    for (k, name) in ["at the goal line or past", "short of the goal line"].iter().enumerate() {
        let (x, r) = (&ez[k], &REAL_EZ[k]);
        println!(
            "    {name:>24}: {:.3} ({:.3})  {:.3} ({:.3})  {:.3} of red zone throws",
            d(x[1], x[0]),
            r[0],
            d(x[2], x[0]),
            r[1],
            d(x[0], all)
        );
    }
    // Real (2021-25, garbage time and overtime out for the downs): scored,
    // dropbacks of snaps, their scores, the runs' scores; Cover 0 and man of
    // the dropbacks (2023-25; from the 1 and the 2 together).
    const REAL_GL: [[f64; 6]; 3] = [
        [0.556, 0.347, 0.533, 0.567, 0.500, 0.795],
        [0.445, 0.744, 0.418, 0.525, 0.500, 0.795],
        [0.475, 0.716, 0.440, 0.564, 0.348, 0.678],
    ];
    println!(
        "  goal line: scored, dropbacks of snaps, their scores, the runs' scores; Cover 0 and man of the dropbacks; runs against Cover 0 and against man (share of runs, scored)"
    );
    for (k, name) in
        ["3rd/4th and goal from the 1", "3rd/4th and goal from the 2", "two-point tries"].iter().enumerate()
    {
        let (x, r) = (&gl[k], &REAL_GL[k]);
        let runs = x[0] - x[2];
        println!(
            "    {name:>27}: n {:.0}  {:.3} ({:.3})  {:.3} ({:.3})  {:.3} ({:.3})  {:.3} ({:.3})  {:.3} ({:.3})  {:.3} ({:.3})  v Cover 0 {:.3} {:.3}  v man {:.3} {:.3}",
            x[0],
            d(x[1], x[0]),
            r[0],
            d(x[2], x[0]),
            r[1],
            d(x[3], x[2]),
            r[2],
            d(x[1] - x[3], runs),
            r[3],
            d(x[4], x[2]),
            r[4],
            d(x[5], x[2]),
            r[5],
            d(x[6], runs),
            d(x[7], x[6]),
            d(x[8], runs),
            d(x[9], x[8])
        );
    }
}

/// AFM_RUNDIAG=1, scheme-fit lane SF9: the backs' designed runs by how far
/// they go and who ends them (item 9), and short yardage (item 8), each
/// beside real 2024 (nflverse play-by-play and participation, regular
/// season, designed runs without a flag, the first listed tackler; engine
/// yards rounded to the nearest yard, as spots are).
fn sf9_diag(
    results: &[afm_engine::game::GameResult],
    pos_of: &std::collections::HashMap<u32, afm_engine::positions::Position>,
) {
    use afm_engine::positions::Position as P;
    let mut backs: Vec<&afm_engine::game::stats::PlayRecord> = Vec::new();
    let mut qbs: Vec<&afm_engine::game::stats::PlayRecord> = Vec::new();
    for r in results {
        for p in r.stats.plays.iter() {
            if p.nullified
                || p.kind != PlayType::Run
                || p.scramble
                || p.call.contains("kneel")
                || !p.penalties.is_empty()
            {
                continue;
            }
            match p.carrier.and_then(|c| pos_of.get(&c)) {
                Some(P::RB) => backs.push(p),
                Some(P::QB) => qbs.push(p),
                _ => {}
            }
        }
    }
    let n = backs.len().max(1) as f64;
    let gained = |p: &afm_engine::game::stats::PlayRecord| p.yards.round();
    let ks = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 12.0, 15.0, 20.0, 25.0, 30.0, 40.0];
    let real = [0.820, 0.703, 0.579, 0.458, 0.349, 0.269, 0.165, 0.104, 0.076, 0.044, 0.0226, 0.0144, 0.0096, 0.0056];
    let cells: Vec<String> = ks
        .iter()
        .zip(real)
        .map(|(&k, r)| format!("{k}:{:.4}({r})", backs.iter().filter(|p| gained(p) >= k).count() as f64 / n))
        .collect();
    println!("  SF9 backs' designed runs {:.0}, gaining k or more (real 2024): {}", n, cells.join(" "));
    let ge = |k: f64| backs.iter().filter(|p| gained(p) >= k).count() as f64;
    println!(
        "  SF9 20+ of 10+ {:.3} (real 0.218), 40+ of 20+ {:.3} (real 0.246)",
        ge(20.0) / ge(10.0).max(1.0),
        ge(40.0) / ge(20.0).max(1.0)
    );
    // Wide of the tackles (crossed more than 4 yards from the ball, as the
    // run rows above) and inside, against nflverse 2024's end gap and the
    // others (guard, tackle, middle).
    for (wide, name, real) in
        [(true, "wide", "ypc 4.72 10+ 0.135 20+ 0.0369"), (false, "inside", "ypc 4.26 10+ 0.096 20+ 0.0193")]
    {
        let out = |p: &afm_engine::game::stats::PlayRecord| match p.tend.cross {
            Some((dy, _)) => dy.abs() > 4.0,
            None => matches!(afm_engine::world::tally::expect::run_gap(&p.call), 0 | 4),
        };
        let v: Vec<_> = backs.iter().filter(|p| out(p) == wide).collect();
        let m = v.len().max(1) as f64;
        println!(
            "  SF9 {name}: {} runs, ypc {:.2} 10+ {:.3} 20+ {:.4} | real {real}",
            v.len(),
            v.iter().map(|p| p.yards).sum::<f64>() / m,
            v.iter().filter(|p| gained(p) >= 10.0).count() as f64 / m,
            v.iter().filter(|p| gained(p) >= 20.0).count() as f64 / m
        );
        let cells: Vec<String> = [1.0, 3.0, 5.0, 8.0, 10.0, 15.0, 20.0, 30.0]
            .iter()
            .map(|&k| format!("{k}:{:.3}", v.iter().filter(|p| gained(p) >= k).count() as f64 / m))
            .collect();
        println!("  SF9 {name} gaining k or more: {}", cells.join(" "));
    }
    // Who ends them: the first tackler's group, out of bounds with nobody on
    // him, a touchdown (real 2024 by run length below).
    let group = |p: &afm_engine::game::stats::PlayRecord| -> &'static str {
        if p.touchdown {
            return "td";
        }
        match p.tacklers.first().and_then(|t| pos_of.get(t)) {
            Some(P::DT) => "DT",
            Some(P::EDGE) => "EDGE",
            Some(P::LB) => "LB",
            Some(P::CB) => "CB",
            Some(P::S) => "S",
            Some(_) => "other",
            None if p.out_of_bounds => "oob",
            None => "none",
        }
    };
    let bins: [(f64, f64, &str, &str); 5] = [
        (-99.0, 1.0, "0-", "DT .223 EDGE .339 LB .233 CB .093 S .104 td .002"),
        (1.0, 5.0, "1-4", "DT .231 EDGE .286 LB .239 CB .082 S .117 td .041"),
        (5.0, 10.0, "5-9", "DT .134 EDGE .199 LB .262 CB .151 S .233 td .021"),
        (10.0, 20.0, "10-19", "DT .034 EDGE .080 LB .158 CB .224 S .468 td .036"),
        (20.0, 999.0, "20+", "DT .000 EDGE .057 LB .072 CB .242 S .398 td .223"),
    ];
    for (lo, hi, name, real) in bins {
        let v: Vec<_> = backs.iter().filter(|p| (lo..hi).contains(&gained(p))).collect();
        let m = v.len().max(1) as f64;
        let mut c = std::collections::BTreeMap::<&str, f64>::new();
        for p in &v {
            *c.entry(group(p)).or_default() += 1.0;
        }
        let oob = v.iter().filter(|p| p.out_of_bounds).count() as f64 / m;
        let cells: Vec<String> = ["DT", "EDGE", "LB", "CB", "S", "oob", "td", "none"]
            .iter()
            .map(|k| format!("{k} {:.3}", c.get(k).copied().unwrap_or(0.0) / m))
            .collect();
        println!(
            "  SF9 {name:>5} {:.4} of runs, out of bounds {oob:.3}, ended by {} | real {real}",
            v.len() as f64 / n,
            cells.join(" ")
        );
    }
    println!("  SF9 real 10-19 out of bounds 0.124, 20+ 0.288");
    // Missed tackles by the missing man's group, a run, and as a share of
    // his group's tries (its misses over its misses and first tackles).
    for (lo, hi, name) in [(-99.0, 5.0, "0-4"), (5.0, 10.0, "5-9"), (10.0, 999.0, "10+"), (-99.0, 999.0, "all")] {
        let v: Vec<_> = backs.iter().filter(|p| (lo..hi).contains(&gained(p))).collect();
        let m = v.len().max(1) as f64;
        let mut miss = std::collections::BTreeMap::<&str, f64>::new();
        let mut tk = std::collections::BTreeMap::<&str, f64>::new();
        for p in &v {
            for (d, _) in &p.missed_tackles {
                let g = match pos_of.get(d) {
                    Some(P::DT) => "DT",
                    Some(P::EDGE) => "EDGE",
                    Some(P::LB) => "LB",
                    Some(P::CB) => "CB",
                    Some(P::S) => "S",
                    _ => "other",
                };
                *miss.entry(g).or_default() += 1.0;
            }
            *tk.entry(group(p)).or_default() += 1.0;
        }
        let cells: Vec<String> = ["DT", "EDGE", "LB", "CB", "S"]
            .iter()
            .map(|k| {
                let a = miss.get(k).copied().unwrap_or(0.0);
                let b = tk.get(k).copied().unwrap_or(0.0);
                format!("{k} {:.3}/{:.2}", a / m, a / (a + b).max(1.0))
            })
            .collect();
        println!("  SF9 missed tackles on {name:>4} runs, a run / of the group's tries: {}", cells.join(" "));
    }
    // Short yardage: third and fourth down, two or less to go, the distance
    // rounded to the yard as real spots are (the audit's `fit_check` rows
    // read 2.0 or less).
    let conv = |v: &[&&afm_engine::game::stats::PlayRecord]| {
        v.iter().filter(|p| p.first_down || p.touchdown).count() as f64 / v.len().max(1) as f64
    };
    let sy: Vec<_> = backs.iter().filter(|p| p.down >= 3 && p.togo < 2.5).collect();
    println!("  SF9 short yardage, backs: {} runs, converted {:.3} (real 0.669)", sy.len(), conv(&sy));
    for (tg, real_c, real_h) in [
        (1.0, 0.691, "-1-:.101 0:.210 1:.122 2:.180 3:.112 4-5:.108 6+:.167"),
        (2.0, 0.608, "-1-:.078 0:.093 1:.222 2:.077 3:.124 4-5:.175 6+:.232"),
    ] {
        let v: Vec<_> = sy.iter().copied().filter(|p| p.togo.round().max(1.0) == tg).collect();
        let m = v.len().max(1) as f64;
        let h = |lo: f64, hi: f64| v.iter().filter(|p| (lo..hi).contains(&gained(p))).count() as f64 / m;
        println!(
            "  SF9   to go {tg}: {} runs, converted {:.3} (real {real_c}) | gain -1-:{:.3} 0:{:.3} 1:{:.3} 2:{:.3} 3:{:.3} 4-5:{:.3} 6+:{:.3} | real {real_h}",
            v.len(),
            conv(&v),
            h(-99.0, 0.0),
            h(0.0, 1.0),
            h(1.0, 2.0),
            h(2.0, 3.0),
            h(3.0, 4.0),
            h(4.0, 6.0),
            h(6.0, 999.0)
        );
    }
    let cells: Vec<String> =
        [(0u8, 5u8, ".043/.719"), (6, 6, ".369/.682"), (7, 7, ".375/.694"), (8, 8, ".144/.620"), (9, 99, ".069/.537")]
            .iter()
            .map(|&(lo, hi, real)| {
                let v: Vec<_> = sy.iter().copied().filter(|p| (lo..=hi).contains(&p.box_count)).collect();
                format!(
                    "{}:{:.3}/{:.3}({real})",
                    if hi == 99 { 9 } else { hi },
                    v.len() as f64 / sy.len().max(1) as f64,
                    conv(&v)
                )
            })
            .collect();
    println!("  SF9   by the box, share/converted: {}", cells.join(" "));
    let cells: Vec<String> = [
        (50.0, 100.0, "own half", 0.683),
        (20.0, 50.0, "their 50-20", 0.690),
        (4.0, 20.0, "their 20-4", 0.611),
        (0.0, 4.0, "inside 3", 0.644),
    ]
    .iter()
    .map(|&(lo, hi, name, real)| {
        let v: Vec<_> = sy.iter().copied().filter(|p| (lo..hi).contains(&p.yardline_100)).collect();
        format!("{name} {:.3}/{}({real})", conv(&v), v.len())
    })
    .collect();
    println!("  SF9   by the field, converted/runs: {}", cells.join(" "));
    // Who ends them, short yardage against every other run (real 2024).
    for (name, short, real) in [
        ("short", true, "CB .116@5.32 DT .168@2.10 EDGE .224@1.79 LB .245@2.17 S .181@6.08 td .064"),
        ("other", false, "CB .116@6.51 DT .186@2.48 EDGE .254@2.78 LB .233@3.77 S .178@7.29 td .031"),
    ] {
        let v: Vec<_> = backs.iter().filter(|p| (p.down >= 3 && p.togo < 2.5) == short).collect();
        let m = v.len().max(1) as f64;
        let cells: Vec<String> = ["CB", "DT", "EDGE", "LB", "S", "oob", "td"]
            .iter()
            .map(|k| {
                let w: Vec<f64> = v.iter().filter(|p| group(p) == *k).map(|p| gained(p)).collect();
                format!("{k} {:.3}@{:.2}", w.len() as f64 / m, w.iter().sum::<f64>() / w.len().max(1) as f64)
            })
            .collect();
        println!("  SF9   ended by, {name}: {} | real {real}", cells.join(" "));
    }
    let ybc = |v: &[&&afm_engine::game::stats::PlayRecord]| {
        v.iter().map(|p| p.before_contact).sum::<f64>() / v.len().max(1) as f64
    };
    let all: Vec<_> = backs.iter().collect();
    println!("  SF9   yards before contact: short yardage {:.2}, all runs {:.2}", ybc(&sy), ybc(&all));
    let qsy: Vec<_> = qbs.iter().filter(|p| p.down >= 3 && p.togo < 2.5).collect();
    let sneaks: Vec<_> = qsy.iter().copied().filter(|p| p.call.contains("sneak")).collect();
    let other: Vec<_> = qsy.iter().copied().filter(|p| !p.call.contains("sneak")).collect();
    println!(
        "  SF9 short yardage, quarterbacks: {} runs, converted {:.3} (real 0.827); sneaks {} at {:.3} (real 0.852), other {} at {:.3} (real 0.757)",
        qsy.len(),
        conv(&qsy),
        sneaks.len(),
        conv(&sneaks),
        other.len(),
        conv(&other)
    );
    // Wide runs (as above) by length: share, out of bounds and who ends
    // them, against nflverse 2024's end-gap runs.
    let wide_of = |p: &afm_engine::game::stats::PlayRecord| match p.tend.cross {
        Some((dy, _)) => dy.abs() > 4.0,
        None => matches!(afm_engine::world::tally::expect::run_gap(&p.call), 0 | 4),
    };
    let wide: Vec<_> = backs.iter().filter(|p| wide_of(p)).collect();
    let wn = wide.len().max(1) as f64;
    for (lo, hi, name, real) in [
        (-99.0, 1.0, "0-", "share .232 oob .073 DT .143 EDGE .308 LB .277 CB .161 S .112"),
        (1.0, 5.0, "1-4", "share .380 oob .128 DT .120 EDGE .246 LB .272 CB .175 S .145"),
        (5.0, 10.0, "5-9", "share .254 oob .194 DT .073 EDGE .162 LB .249 CB .220 S .270"),
        (10.0, 999.0, "10+", "share .135 oob .355 DT .031 EDGE .057 LB .138 CB .230 S .459"),
    ] {
        let v: Vec<_> = wide.iter().filter(|p| (lo..hi).contains(&gained(p))).collect();
        let m = v.len().max(1) as f64;
        let mut c = std::collections::BTreeMap::<&str, f64>::new();
        for p in &v {
            *c.entry(group(p)).or_default() += 1.0;
        }
        let cells: Vec<String> = ["DT", "EDGE", "LB", "CB", "S"]
            .iter()
            .map(|g| format!("{g} {:.3}", c.get(g).copied().unwrap_or(0.0) / m))
            .collect();
        println!(
            "  SF9 wide {name:>4}: share {:.3} oob {:.3} {} | real {real}",
            v.len() as f64 / wn,
            v.iter().filter(|p| p.out_of_bounds).count() as f64 / m,
            cells.join(" ")
        );
    }
}

/// AFM_RUNDIAG=1: where backs' designed runs are stopped and by whom
/// (minor bug 164).
fn run_diag(
    results: &[afm_engine::game::GameResult],
    pos_of: &std::collections::HashMap<u32, afm_engine::positions::Position>,
    pairs: &[(usize, usize)],
    teams: &[afm_engine::game::GameTeam],
) {
    // Each team's line on runs (scheme-fit item 7): its five best blockers'
    // Run Blocking, and its four best linemen's Block Shedding.
    use afm_engine::attributes::Attr;
    let top = |t: &afm_engine::game::GameTeam, ps: &[afm_engine::positions::Position], a: Attr, n: usize| {
        let mut v: Vec<f64> = t.players.iter().filter(|p| ps.contains(&p.position)).map(|p| p.attrs.get(a)).collect();
        v.sort_by(|x, y| y.total_cmp(x));
        v.truncate(n);
        v.iter().sum::<f64>() / v.len().max(1) as f64
    };
    {
        use afm_engine::positions::Position as Q;
        let ol: Vec<f64> = teams.iter().map(|t| top(t, &[Q::OT, Q::IOL], Attr::RunBlocking, 5)).collect();
        let dl: Vec<f64> = teams.iter().map(|t| top(t, &[Q::EDGE, Q::DT], Attr::BlockShedding, 4)).collect();
        // Per team game: the backs' yards before contact a carry against
        // the line's edge (its blockers' Run Blocking less the other side's
        // shedders' Block Shedding), weighted by carries.
        let (mut sw, mut sx, mut sy, mut sxx, mut sxy) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
        for (gi, r) in results.iter().enumerate() {
            let (h, a) = pairs[gi];
            for off in 0..2usize {
                let (me, them) = if off == 0 { (h, a) } else { (a, h) };
                let (mut n, mut y) = (0.0, 0.0);
                for p in r.stats.plays.iter().filter(|p| !p.nullified && p.offense == off && p.kind == PlayType::Run) {
                    if p.call.contains("kneel") || p.call.contains("sneak") {
                        continue;
                    }
                    if !matches!(p.carrier.and_then(|c| pos_of.get(&c)), Some(Q::RB)) {
                        continue;
                    }
                    n += 1.0;
                    y += p.before_contact;
                }
                if n == 0.0 {
                    continue;
                }
                let x = ol[me] - dl[them];
                let yy = y / n;
                sw += n;
                sx += n * x;
                sy += n * yy;
                sxx += n * x * x;
                sxy += n * x * yy;
            }
        }
        let mx = sx / sw.max(1.0);
        let my = sy / sw.max(1.0);
        let slope = (sxy / sw.max(1.0) - mx * my) / (sxx / sw.max(1.0) - mx * mx).max(1e-9);
        let sd = |v: &[f64]| {
            let m = v.iter().sum::<f64>() / v.len().max(1) as f64;
            (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (v.len().max(2) - 1) as f64).sqrt()
        };
        println!(
            "  the line on runs: before contact {slope:.4} a point of the line's edge (its five blockers' Run Blocking less the other four linemen's Block Shedding); teams' blockers sd {:.2}, shedders sd {:.2}: the line's spread {:.3} a carry on offense, {:.3} on defense",
            sd(&ol),
            sd(&dl),
            slope * sd(&ol),
            slope * sd(&dl)
        );
    }
    // Each team's backs' designed runs in 17-game seasons: [runs, yards, games].
    let mut team_season = std::collections::HashMap::<usize, [f64; 3]>::new();
    let mut team_ypc: Vec<f64> = Vec::new();
    use afm_engine::positions::Position as P;
    // [runs, yards, before contact, contact behind the line, stuffs]
    let mut g = [[0.0f64; 6]; 4];
    let mut why = std::collections::BTreeMap::<String, f64>::new();
    let mut sheds = [0.0f64; 3];
    let mut held_to_end = std::collections::BTreeMap::<&str, f64>::new();
    // [neutral, to go 2 or less, inside the 5, other, all] x box 5..9 x [runs, yards, stuffs]
    let mut box_sit = [[[0.0f64; 3]; 5]; 5];
    // Each back's carries in blocks of 204 (a real 120+ carry season's mean):
    // the spread of yards a carry across blocks, all and within one back.
    let mut per_back = std::collections::HashMap::<u32, (f64, f64, f64)>::new();
    let mut blocks: Vec<(u32, f64)> = Vec::new();
    // The same blocks' yards before contact a carry (scheme-fit item 7):
    // real backs' seasons have a true spread of 0.36 (PFR 2018-25), mostly
    // the line's work.
    let mut ybc_blocks: Vec<(u32, f64)> = Vec::new();
    // First tackles: [all, by a man on a block he never shed, by one who shed]
    let mut ftk_blk = [0.0f64; 3];
    // Broken tackles a carry in blocks of 182 carries (the real qualifying
    // season's mean, 100+ carries; minor bug 334): real sd 0.0301, season to
    // season r 0.209 (nflverse 2018-25, `results_real.json`, Broken).
    let mut bt_back = std::collections::HashMap::<u32, (f64, f64)>::new();
    let mut bt_blocks: Vec<(u32, f64)> = Vec::new();
    let mut shed_by = std::collections::BTreeMap::<String, f64>::new();
    let mut contact_at = std::collections::BTreeMap::<String, [f64; 10]>::new();
    // Box 5..9 x first tackler group x [runs, yards, before contact]
    let mut box_tk = [[[0.0f64; 3]; 4]; 5];
    let mut n = 0.0;
    let mut hist = [0.0f64; 16];
    for (gi, r) in results.iter().enumerate() {
        let (h, a) = pairs[gi];
        for side in [h, a] {
            let e = team_season.entry(side).or_default();
            e[2] += 1.0;
            if e[2] > 17.0 {
                team_ypc.push(e[1] / e[0].max(1.0));
                *e = [0.0, 0.0, 1.0];
            }
        }
        for p in r.stats.plays.iter().filter(|p| !p.nullified) {
            if p.kind != PlayType::Run || p.call.contains("kneel") || p.call.contains("sneak") {
                continue;
            }
            if !matches!(p.carrier.and_then(|c| pos_of.get(&c)), Some(P::RB)) {
                continue;
            }
            n += 1.0;
            let team = if p.offense == 0 { h } else { a };
            let e = team_season.entry(team).or_default();
            e[0] += 1.0;
            e[1] += p.yards;
            let bx = (p.box_count as usize).clamp(5, 9) - 5;
            let diff = (p.score[0] - p.score[1]).abs();
            let neutral = p.down <= 2
                && p.togo >= 5.0
                && (20.0..=80.0).contains(&p.yardline_100)
                && (diff <= 16 && p.quarter < 4 || diff <= 8);
            let sit = if neutral {
                0
            } else if p.togo <= 2.0 {
                1
            } else if p.yardline_100 <= 5.0 {
                2
            } else {
                3
            };
            for k in [sit, 4] {
                box_sit[k][bx][0] += 1.0;
                box_sit[k][bx][1] += p.yards;
                box_sit[k][bx][2] += (p.yards < 0.5) as u8 as f64;
            }
            if let Some(c) = p.carrier {
                let e = bt_back.entry(c).or_default();
                e.0 += 1.0;
                e.1 += p.missed_tackles.iter().filter(|m| m.1 == c).count() as f64;
                if e.0 >= 182.0 {
                    bt_blocks.push((c, e.1 / e.0));
                    *e = (0.0, 0.0);
                }
                let e = per_back.entry(c).or_default();
                e.0 += 1.0;
                e.1 += p.yards;
                e.2 += p.before_contact;
                if e.0 >= 204.0 {
                    blocks.push((c, e.1 / e.0));
                    ybc_blocks.push((c, e.2 / e.0));
                    *e = (0.0, 0.0, 0.0);
                }
            }
            // The first tackler's run blocks on record (scheme-fit item 7,
            // the audit's count): one he never shed (still on it at the
            // whistle, put down, or picked up by a blocker after his hit),
            // one he shed, or none. Who was on a block at the hit itself is
            // `sim::FIRST_TACKLE_BLOCK`.
            if let Some(&d) = p.tacklers.first() {
                let held = p.blocks.iter().any(|b| b.1 == d && !b.2 && b.3);
                let shed = p.blocks.iter().any(|b| b.1 == d && !b.2 && !b.3);
                ftk_blk[0] += 1.0;
                ftk_blk[1] += f64::from(u8::from(held));
                ftk_blk[2] += f64::from(u8::from(shed && !held));
            }
            let k = match p.tacklers.first().and_then(|t| pos_of.get(t)) {
                Some(P::EDGE | P::DT) => 0,
                Some(P::LB) => 1,
                Some(P::CB | P::S) => 2,
                _ => 3,
            };
            box_tk[bx][k][0] += 1.0;
            box_tk[bx][k][1] += p.yards;
            box_tk[bx][k][2] += p.before_contact;
            let e = &mut g[k];
            e[0] += 1.0;
            e[1] += p.yards;
            e[2] += p.before_contact;
            e[3] += (p.before_contact < 0.0) as u8 as f64;
            e[4] += (p.yards < 0.5) as u8 as f64;
            e[5] += p.missed_tackles.len() as f64;
            let b = ((p.yards + 3.5).floor().max(0.0) as usize).min(15);
            hist[b] += 1.0;
            if p.yards < 0.5 {
                let w = if p.why.contains("came free") {
                    "came free"
                } else if p.why.contains(" shed ") {
                    "shed"
                } else {
                    "other"
                };
                let lab =
                    p.tacklers.first().and_then(|t| pos_of.get(t)).map(|x| format!("{x:?}")).unwrap_or("-".into());
                *why.entry(format!("{w} {lab}")).or_default() += 1.0;
            }
            for bl in p.blocks.iter().filter(|b| !b.2) {
                if !bl.3 {
                    let tb = if bl.4 < 1.2 {
                        0
                    } else if bl.4 < 2.5 {
                        1
                    } else {
                        2
                    };
                    sheds[tb] += 1.0;
                    let grp = |id: u32| match pos_of.get(&id) {
                        Some(P::EDGE | P::DT) => "DL",
                        Some(P::LB) => "LB",
                        Some(P::CB | P::S) => "DB",
                        Some(P::OT | P::IOL) => "OL",
                        Some(P::TE) => "TE",
                        Some(P::WR) => "WR",
                        _ => "RB",
                    };
                    *shed_by
                        .entry(format!("{}>{} {}", grp(bl.1), grp(bl.0), ["<1.2", "1.2-2.5", "2.5+"][tb]))
                        .or_default() += 1.0;
                }
            }
            for bl in p.blocks.iter().filter(|b| !b.2 && b.3) {
                let g = match pos_of.get(&bl.1) {
                    Some(P::EDGE | P::DT) => "DL",
                    Some(P::LB) => "LB",
                    Some(P::CB | P::S) => "DB",
                    _ => "other",
                };
                *held_to_end.entry(g).or_default() += 1.0;
            }
            let lab = p.tacklers.first().and_then(|t| pos_of.get(t)).map(|x| format!("{x:?}")).unwrap_or("-".into());
            let bcb = ((p.before_contact + 3.0).floor().clamp(0.0, 9.0)) as usize;
            contact_at.entry(lab).or_insert([0.0; 10])[bcb] += 1.0;
        }
    }
    println!("\nRun diagnostic (backs' designed runs, {n:.0})");
    for (bx, row) in box_tk.iter().enumerate() {
        let tot: f64 = row.iter().map(|e| e[0]).sum::<f64>().max(1.0);
        let cells: Vec<String> = row
            .iter()
            .map(|e| format!("{:.2}@{:.1}/{:.1}", e[0] / tot, e[1] / e[0].max(1.0), e[2] / e[0].max(1.0)))
            .collect();
        println!(
            "  box {} tackler share@ypc/before contact: line {} lb {} db {} none {}",
            bx + 5,
            cells[0],
            cells[1],
            cells[2],
            cells[3]
        );
    }
    for (k, name) in ["line", "linebackers", "backs", "none/other"].iter().enumerate() {
        let e = g[k];
        println!(
            "  {name:<12} share {:.3} ypc {:.2} before contact {:.2} contact behind the line {:.3} stuffed {:.3} missed {:.2}",
            e[0] / n,
            e[1] / e[0].max(1.0),
            e[2] / e[0].max(1.0),
            e[3] / e[0].max(1.0),
            e[4] / e[0].max(1.0),
            e[5] / e[0].max(1.0)
        );
    }
    let tot: f64 = g.iter().map(|e| e[0]).sum();
    println!(
        "  all: ypc {:.2} before contact {:.2} contact behind {:.3}",
        g.iter().map(|e| e[1]).sum::<f64>() / tot,
        g.iter().map(|e| e[2]).sum::<f64>() / tot,
        g.iter().map(|e| e[3]).sum::<f64>() / tot
    );
    println!(
        "  run sheds a run: under 1.2 s {:.2}, 1.2-2.5 s {:.2}, later {:.2}",
        sheds[0] / n,
        sheds[1] / n,
        sheds[2] / n
    );
    let h: Vec<String> = hist.iter().enumerate().map(|(i, v)| format!("{}:{:.3}", i as i32 - 3, v / n)).collect();
    println!("  yards (floor, -3..12+): {}", h.join(" "));
    for (k, v) in &held_to_end {
        println!("  blocked to the whistle or pancaked: {k} {:.2} a run", v / n);
    }
    for (k, v) in shed_by.iter().filter(|x| *x.1 / n > 0.02) {
        println!("  shed {k:<16} {:.3} a run", v / n);
    }
    for (k, v) in &contact_at {
        let h: Vec<String> = v.iter().map(|x| format!("{:.3}", x / n)).collect();
        println!("  first contact (yards -3..6+) by first tackler {k:<5} {}", h.join(" "));
    }
    // Real (RB designed runs 2021-25, engine_targets.json runs_by_box):
    // neutral 5-9 in the box 5.12 4.85 4.68 4.39 3.89; all 5.27 4.61 4.18 3.69 2.30;
    // to go 2 or less 3.27 a carry, inside the 5 1.09.
    for (k, name) in ["neutral", "to go <=2", "inside 5", "other", "all"].iter().enumerate() {
        let tot: f64 = box_sit[k].iter().map(|b| b[0]).sum();
        let y: f64 = box_sit[k].iter().map(|b| b[1]).sum();
        let cells: Vec<String> = box_sit[k]
            .iter()
            .enumerate()
            .map(|(b, v)| {
                format!("{}:{:.2}/{:.2}/{:.2}", b + 5, v[1] / v[0].max(1.0), v[2] / v[0].max(1.0), v[0] / tot.max(1.0))
            })
            .collect();
        println!(
            "  box {name:<10} n {:.3} ypc {:.2} | box: ypc/stuffed/share {}",
            tot / n,
            y / tot.max(1.0),
            cells.join(" ")
        );
    }
    let sd = |v: &[f64]| {
        let m = v.iter().sum::<f64>() / v.len().max(1) as f64;
        (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (v.len().max(2) - 1) as f64).sqrt()
    };
    let all: Vec<f64> = blocks.iter().map(|b| b.1).collect();
    let mut within = 0.0;
    let mut wn = 0.0;
    let mut ids: Vec<u32> = blocks.iter().map(|b| b.0).collect();
    ids.sort();
    ids.dedup();
    for id in &ids {
        let v: Vec<f64> = blocks.iter().filter(|b| b.0 == *id).map(|b| b.1).collect();
        if v.len() >= 2 {
            within += sd(&v).powi(2) * (v.len() - 1) as f64;
            wn += (v.len() - 1) as f64;
        }
    }
    // Consecutive blocks of one back (real season to season 0.29, 120+ carries).
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for id in &ids {
        let v: Vec<f64> = blocks.iter().filter(|b| b.0 == *id).map(|b| b.1).collect();
        for w in v.windows(2) {
            xs.push(w[0]);
            ys.push(w[1]);
        }
    }
    let corr = {
        let mx = xs.iter().sum::<f64>() / xs.len().max(1) as f64;
        let my = ys.iter().sum::<f64>() / ys.len().max(1) as f64;
        let c: f64 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum();
        let vx: f64 = xs.iter().map(|x| (x - mx) * (x - mx)).sum();
        let vy: f64 = ys.iter().map(|y| (y - my) * (y - my)).sum();
        c / (vx * vy).sqrt().max(1e-9)
    };
    // A spread's lasting part from consecutive blocks of one back: the
    // covariance of block k and k+1 (noise doesn't carry over).
    let lasting = |bl: &[(u32, f64)]| {
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        for id in &ids {
            let v: Vec<f64> = bl.iter().filter(|b| b.0 == *id).map(|b| b.1).collect();
            for w in v.windows(2) {
                xs.push(w[0]);
                ys.push(w[1]);
            }
        }
        let k = xs.len().max(2) as f64;
        let mx = xs.iter().sum::<f64>() / k;
        let my = ys.iter().sum::<f64>() / k;
        let c: f64 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum::<f64>() / (k - 1.0);
        let vx: f64 = xs.iter().map(|x| (x - mx) * (x - mx)).sum::<f64>() / (k - 1.0);
        let vy: f64 = ys.iter().map(|y| (y - my) * (y - my)).sum::<f64>() / (k - 1.0);
        (c.max(0.0).sqrt(), c / (vx * vy).sqrt().max(1e-9), xs.len())
    };
    let ybc: Vec<f64> = ybc_blocks.iter().map(|b| b.1).collect();
    let (ybc_true, ybc_r, ybc_n) = lasting(&ybc_blocks);
    let (ypc_true, _, _) = lasting(&blocks);
    println!(
        "  backs' 204-carry blocks, before contact: mean {:.2}, sd {:.3}, consecutive r {ybc_r:.3} ({ybc_n} pairs), lasting spread {ybc_true:.3} (real true spread 0.36); yards a carry lasting spread {ypc_true:.3}",
        ybc.iter().sum::<f64>() / ybc.len().max(1) as f64,
        sd(&ybc)
    );
    println!(
        "  first tackles on backs' runs, his blocks on record: one he never shed (on it at the whistle, put down, or picked up after the hit) {:.3}, one he shed {:.3}",
        ftk_blk[1] / ftk_blk[0].max(1.0),
        ftk_blk[2] / ftk_blk[0].max(1.0)
    );
    println!("  team seasons (17 games): {} , backs' ypc sd {:.3} (real 0.448)", team_ypc.len(), sd(&team_ypc));
    println!("  backs' consecutive blocks: {} pairs, correlation {corr:.3} (real season to season 0.29)", xs.len());
    println!(
        "  backs' 204-carry blocks: {} from {} backs, ypc sd {:.3} (real 120+ carry seasons 0.573), within one back {:.3} (real noise about 0.42)",
        all.len(),
        ids.len(),
        sd(&all),
        (within / wn.max(1.0)).sqrt()
    );
    {
        let v: Vec<f64> = bt_blocks.iter().map(|b| b.1).collect();
        let m = v.iter().sum::<f64>() / v.len().max(1) as f64;
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        let mut bids: Vec<u32> = bt_blocks.iter().map(|b| b.0).collect();
        bids.sort();
        bids.dedup();
        for id in &bids {
            let v: Vec<f64> = bt_blocks.iter().filter(|b| b.0 == *id).map(|b| b.1).collect();
            for w in v.windows(2) {
                xs.push(w[0]);
                ys.push(w[1]);
            }
        }
        let mx = xs.iter().sum::<f64>() / xs.len().max(1) as f64;
        let my = ys.iter().sum::<f64>() / ys.len().max(1) as f64;
        let c: f64 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum();
        let vx: f64 = xs.iter().map(|x| (x - mx) * (x - mx)).sum();
        let vy: f64 = ys.iter().map(|y| (y - my) * (y - my)).sum();
        println!(
            "  backs' 182-carry blocks: {} from {} backs, broken tackles a carry {:.4}, sd {:.4} (real 0.0301; chance alone {:.4}), consecutive r {:.3} (real 0.209)",
            v.len(),
            bids.len(),
            m,
            sd(&v),
            (m / 182.0).sqrt(),
            c / (vx * vy).sqrt().max(1e-9)
        );
    }
    let mut w: Vec<_> = why.into_iter().collect();
    w.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (k, v) in w.iter().take(12) {
        println!("  stuff: {k:<16} {:.3} of runs", v / n);
    }
}

/// Engine gaps lane EG1's injury counts (`AFM_GAPS=1`): each position's
/// share of the week-plus injuries against real (nflverse injury reports,
/// episodes a player-season by the men a team plays at each position), what
/// did it, the body parts against real, and the sideline (back in the same
/// game, concussions and their checks). Per team game unless named.
fn injury_counts(
    results: &[afm_engine::game::GameResult],
    pos_of: &std::collections::HashMap<u32, afm_engine::positions::Position>,
) {
    use afm_engine::game::sim::InjuryKind as K;
    use afm_engine::positions::Position as P;
    use std::collections::BTreeMap;
    let tg = results.len() as f64 * 2.0;
    let dev: Value =
        serde_json::from_str(include_str!("../../../../data/real-world/development_injuries.json")).unwrap();
    // Real: episodes a player-season times the men a team plays four games
    // or more at each position (Claude's count of a typical NFL team).
    let eps = &dev["injury_episodes_per_player_season"];
    let men = [
        ("QB", "QB", 1.6),
        ("RB", "RB", 3.3),
        ("WR", "WR", 5.5),
        ("TE", "TE", 3.3),
        ("OL", "OL", 8.0),
        ("DT", "DL", 5.0),
        ("EDGE", "EDGE", 4.0),
        ("LB", "LB", 5.0),
        ("CB", "CB", 5.2),
        ("S", "S", 4.0),
        ("K", "K", 1.0),
        ("P", "P", 1.0),
    ];
    let real: Vec<(&str, f64)> = men.iter().map(|&(g, k, n)| (g, eps[k].as_f64().unwrap_or(0.0) * n)).collect();
    let real_total: f64 = real.iter().map(|x| x.1).sum();
    let group = |p: Option<&P>| match p {
        Some(P::OT | P::IOL) => "OL",
        Some(P::LS) => "K",
        Some(p) => p.name(),
        None => "?",
    };
    let mut by_group: BTreeMap<&str, f64> = BTreeMap::new();
    let mut by_cause: BTreeMap<&str, [f64; 2]> = BTreeMap::new();
    // Week-plus ones by position and what did it.
    let mut grid: BTreeMap<(&str, &str), f64> = BTreeMap::new();
    let mut by_part: BTreeMap<&str, f64> = BTreeMap::new();
    // Sideline: [hurt, played on, went off and came back, out for the game],
    // plays sat by those who came back, and the same for week-plus ones.
    let mut side = [0.0f64; 4];
    let mut side_wk = [0.0f64; 4];
    let mut back_plays = Vec::new();
    // Concussions: [all, played on (nobody caught it), back in, out].
    let mut conc = [0.0f64; 4];
    let mut checks = [0.0f64; 3];
    let mut check_plays = Vec::new();
    let mut nc = [0.0f64; 2];
    let mut wk_total = 0.0f64;
    for r in results {
        for (_, i) in &r.injuries {
            let wk = i.weeks >= 1.0;
            let e = by_cause.entry(i.cause.name()).or_default();
            e[0] += 1.0;
            let k = if i.plays_on {
                1
            } else if i.back.is_some() {
                2
            } else {
                3
            };
            side[0] += 1.0;
            side[k] += 1.0;
            if let Some(b) = i.back {
                back_plays.push(b as f64);
            }
            if i.cause.non_contact() {
                nc[0] += 1.0;
            }
            if i.kind == K::Concussion {
                conc[0] += 1.0;
                conc[k] += 1.0;
            }
            if wk {
                e[1] += 1.0;
                wk_total += 1.0;
                side_wk[0] += 1.0;
                side_wk[k] += 1.0;
                *by_group.entry(group(pos_of.get(&i.id))).or_default() += 1.0;
                *grid.entry((group(pos_of.get(&i.id)), i.cause.name())).or_default() += 1.0;
                if i.cause.non_contact() {
                    nc[1] += 1.0;
                }
                let part = match i.kind {
                    K::Knee | K::Acl => "Knee",
                    K::AnkleSprain | K::HighAnkleSprain => "Ankle",
                    K::Hamstring => "Hamstring",
                    K::Concussion => "Concussion",
                    K::Shoulder => "Shoulder",
                    K::Foot => "Foot",
                    K::Groin => "Groin",
                    K::Calf => "Calf",
                    K::Back => "Back",
                    K::Neck => "Neck",
                    K::Hip => "Hip",
                    K::Quad => "Quadricep",
                    K::Achilles => "(Achilles)",
                    _ => "(bruise, ribs, hand, bone)",
                };
                *by_part.entry(part).or_default() += 1.0;
            }
        }
        for c in &r.checks {
            checks[0] += 1.0;
            if let Some(b) = c.2 {
                checks[1] += 1.0;
                check_plays.push(b as f64);
            } else {
                checks[2] += 1.0;
            }
        }
    }
    println!("injuries of a week or more by position (share; real share from nflverse episodes):");
    for (g, rv) in &real {
        let v = by_group.get(g).copied().unwrap_or(0.0);
        println!("  {g:<5} {:.3} a team game  share {:.3}  real {:.3}", v / tg, v / wk_total.max(1.0), rv / real_total);
    }
    let causes: Vec<&str> = by_cause.keys().copied().collect();
    println!("injuries of a week or more a team game by position and what did it:");
    println!("  {:<5} {}", "", causes.iter().map(|c| format!("{c:>9}")).collect::<String>());
    for (g, _) in &real {
        let row: String =
            causes.iter().map(|c| format!("{:>9.3}", grid.get(&(*g, *c)).copied().unwrap_or(0.0) / tg)).collect();
        println!("  {g:<5} {row}");
    }
    println!("injuries by what did it (all / 1+ week a team game):");
    for (c, e) in &by_cause {
        println!("  {c:<9} {:.3} {:.3}", e[0] / tg, e[1] / tg);
    }
    println!(
        "  nobody touching him: {:.3} of all, {:.3} of 1+ week (the mix's non-contact share: 0.246)",
        nc[0] / side[0].max(1.0),
        nc[1] / wk_total.max(1.0)
    );
    println!("injuries of a week or more by body part (share; real nflverse):");
    let parts = &dev["injury_episodes_by_body_part"];
    for (p, v) in &by_part {
        let rv = parts[*p]["share"].as_f64();
        println!(
            "  {p:<26} {:.3}  real {}",
            v / wk_total.max(1.0),
            rv.map_or_else(|| "-".to_string(), |x| format!("{x:.3}"))
        );
    }
    let mean = |v: &[f64]| if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 };
    println!(
        "the sideline: of all injuries {:.3} played on, {:.3} went off and came back (after {:.1} plays on average), {:.3} out for the game; 1+ week {:.3} / {:.3} / {:.3}",
        side[1] / side[0].max(1.0),
        side[2] / side[0].max(1.0),
        mean(&back_plays),
        side[3] / side[0].max(1.0),
        side_wk[1] / side_wk[0].max(1.0),
        side_wk[2] / side_wk[0].max(1.0),
        side_wk[3] / side_wk[0].max(1.0)
    );
    println!(
        "men back in the same game {:.3} a team game; concussions {:.3} a game ({:.3} nobody caught, {:.3} back in, {:.3} out); cleared concussion checks {:.3} a game ({:.3} back in after {:.1} plays, {:.3} sat)",
        side[2] / tg,
        conc[0] / (tg / 2.0),
        conc[1] / conc[0].max(1.0),
        conc[2] / conc[0].max(1.0),
        conc[3] / conc[0].max(1.0),
        checks[0] / (tg / 2.0),
        checks[1] / checks[0].max(1.0),
        mean(&check_plays),
        checks[2] / checks[0].max(1.0)
    );
}

/// A rate's spread between men with `min` or more tries, the binomial noise
/// of each man's count taken out: (men, pooled rate, true sd). Each row is
/// (hits, tries).
fn true_sd(rows: &[(f64, f64)], min: f64) -> (usize, f64, f64) {
    let r: Vec<&(f64, f64)> = rows.iter().filter(|x| x.1 >= min).collect();
    let n = r.len();
    if n < 3 {
        return (n, 0.0, 0.0);
    }
    let pb = r.iter().map(|x| x.0).sum::<f64>() / r.iter().map(|x| x.1).sum::<f64>();
    let p: Vec<f64> = r.iter().map(|x| x.0 / x.1).collect();
    let m = p.iter().sum::<f64>() / n as f64;
    let var = p.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1) as f64;
    let noise = r.iter().map(|x| pb * (1.0 - pb) / x.1).sum::<f64>() / n as f64;
    (n, pb, (var - noise).max(0.0).sqrt())
}

/// The slope of y on x and their correlation, over (x, y) rows.
fn slope(rows: &[(f64, f64)]) -> (f64, f64) {
    let n = rows.len().max(1) as f64;
    let (mx, my) = (rows.iter().map(|r| r.0).sum::<f64>() / n, rows.iter().map(|r| r.1).sum::<f64>() / n);
    let sxy: f64 = rows.iter().map(|r| (r.0 - mx) * (r.1 - my)).sum();
    let sxx: f64 = rows.iter().map(|r| (r.0 - mx).powi(2)).sum();
    let syy: f64 = rows.iter().map(|r| (r.1 - my).powi(2)).sum();
    (sxy / sxx.max(1e-9), sxy / (sxx * syy).sqrt().max(1e-9))
}

/// AFM_RECDIAG=1: the spread between receivers (scheme-fit lane SF4, items
/// 6 and 25). Every man with 40 or more targets (12 or more contested: under
/// a yard of separation at the catch, picks left out), with the binomial
/// noise of his counts taken out, beside the real NFL's receiver seasons
/// (FTN charting 2022-24 joined to nflverse play by play and rosters, the
/// same cuts), the slope of drops on Catching, and the forty's slopes of his
/// aDOT and deep targets across men with 40 or more targets, so read it at
/// the NFL.
fn rec_diag(results: &[afm_engine::game::GameResult], teams: &[afm_engine::game::GameTeam]) {
    use afm_engine::attributes::Attr;
    use afm_engine::positions::Position as P;
    let who: std::collections::HashMap<u32, (P, f64, f64, f64, f64)> = teams
        .iter()
        .flat_map(|t| {
            t.players.iter().map(|p| {
                (p.id, (p.position, p.height, p.attrs.get(Attr::Speed), p.forty(), p.attrs.get(Attr::Catching)))
            })
        })
        .collect();
    // Per man: [targets, catches, drops, contested, contested catches, and
    // over the throws not picked (a pick keeps no separation) separation,
    // its square and the throws; then air yards and targets 20+ yards
    // downfield].
    let mut by = std::collections::HashMap::<u32, [f64; 10]>::new();
    for r in results {
        for p in &r.stats.plays {
            if p.nullified || p.kind != PlayType::Pass || p.throw_away {
                continue;
            }
            let Some(t) = p.target else { continue };
            let e = by.entry(t).or_default();
            e[0] += 1.0;
            e[1] += f64::from(u8::from(p.complete));
            e[2] += f64::from(u8::from(p.drop));
            if p.separation < 1.0 && !p.interception {
                e[3] += 1.0;
                e[4] += f64::from(u8::from(p.complete));
            }
            if !p.interception {
                e[5] += p.separation;
                e[6] += p.separation * p.separation;
                e[7] += 1.0;
            }
            e[8] += p.air_yards;
            e[9] += f64::from(u8::from(p.air_yards >= 20.0));
        }
    }
    println!(
        "\nReceivers (AFM_RECDIAG): men with 40+ targets (12+ contested), each man's binomial noise taken out; real: NFL receiver seasons, FTN charting 2022-24 with nflverse"
    );
    // Real: contested share, contested catch rate, drops a target, and the
    // true sd of the catch rate, contested catch rate (too few real backs
    // reach 12 contested) and drops a target; the height slopes (an inch)
    // of the contested catch rate and share.
    // The forty's slopes (a 0.1 s) of aDOT and of the share of targets 20+
    // yards downfield (`data/real-world/ab_real.json`, nflverse 2018-25 with
    // combine forties, 40+ targets; too few backs ran a forty for it).
    let real = [
        (P::WR, [0.170, 0.462, 0.0345, 0.0567, 0.0888, 0.0054], Some([0.0072, 0.0107]), Some([-0.79, -0.0285])),
        (P::TE, [0.137, 0.511, 0.0369, 0.0330, 0.1135, 0.0101], Some([-0.0157, 0.0128]), Some([-0.31, f64::NAN])),
        (P::RB, [0.054, 0.513, 0.0487, 0.0358, f64::NAN, 0.0112], None, None),
    ];
    for (pos, r, ht, fr) in real {
        let men: Vec<(u32, [f64; 10])> =
            by.iter().filter(|(id, _)| who.get(id).map(|w| w.0) == Some(pos)).map(|(&id, &v)| (id, v)).collect();
        let sum = |k: usize| men.iter().map(|m| m.1[k]).sum::<f64>();
        let rows = |a: usize, b: usize| men.iter().map(|m| (m.1[a], m.1[b])).collect::<Vec<_>>();
        let (n, _, sd_cat) = true_sd(&rows(1, 0), 40.0);
        let (_, _, sd_drop) = true_sd(&rows(2, 0), 40.0);
        let (nc, _, sd_cont) = true_sd(&rows(4, 3), 12.0);
        println!(
            "  {pos:?}: {n} men; contested {:.1}% of targets (real {:.1}), caught {:.1}% (real {:.1}); dropped {:.2}% of targets (real {:.2})",
            100.0 * sum(3) / sum(0).max(1.0),
            100.0 * r[0],
            100.0 * sum(4) / sum(3).max(1.0),
            100.0 * r[1],
            100.0 * sum(2) / sum(0).max(1.0),
            100.0 * r[2]
        );
        println!(
            "      true sd between men: catch rate {:.1} pts (real {:.1}), contested catch rate {:.1} ({nc} men, real {}), drops a target {:.2} (real {:.2})",
            100.0 * sd_cat,
            100.0 * r[3],
            100.0 * sd_cont,
            if r[4].is_nan() { "too few".to_string() } else { format!("{:.1}", 100.0 * r[4]) },
            100.0 * sd_drop,
            100.0 * r[5]
        );
        let ht_of = |id: u32| who.get(&id).map(|w| w.1).unwrap_or(0.0);
        let cc: Vec<(f64, f64)> =
            men.iter().filter(|m| m.1[3] >= 12.0).map(|m| (ht_of(m.0), m.1[4] / m.1[3])).collect();
        let share: Vec<(f64, f64)> =
            men.iter().filter(|m| m.1[0] >= 40.0).map(|m| (ht_of(m.0), m.1[3] / m.1[0])).collect();
        let sep: Vec<(f64, f64)> = men
            .iter()
            .filter(|m| m.1[7] >= 40.0)
            .map(|m| (who.get(&m.0).map(|w| w.2).unwrap_or(0.0), m.1[5] / m.1[7]))
            .collect();
        // Separation's spread between men: the variance of their means less
        // the mean of each man's own variance over his throws.
        let (sep_m, sep_noise) = {
            let k = sep.len().max(2) as f64;
            let m = sep.iter().map(|x| x.1).sum::<f64>() / k;
            let var = sep.iter().map(|x| (x.1 - m).powi(2)).sum::<f64>() / (k - 1.0);
            let noise = men
                .iter()
                .filter(|m| m.1[7] >= 40.0)
                .map(|m| (m.1[6] / m.1[7] - (m.1[5] / m.1[7]).powi(2)) / m.1[7])
                .sum::<f64>()
                / k;
            (m, (var - noise).max(0.0).sqrt())
        };
        let real_ht = |k: usize| ht.map(|h| format!(" (real {:+.2})", 100.0 * h[k])).unwrap_or_default();
        println!(
            "      an inch of height: contested catch rate {:+.2} pts{}, contested share {:+.2}{}",
            100.0 * slope(&cc).0,
            real_ht(0),
            100.0 * slope(&share).0,
            real_ht(1),
        );
        println!(
            "      separation at the catch {sep_m:.2} yd, true sd {sep_noise:.2} (real WR 0.42, Next Gen Stats 2018-25), {:+.3} yd a 10 points of Speed (correlation {:+.2}, real about none)",
            10.0 * slope(&sep).0,
            slope(&sep).1
        );
        // Drops on Catching across men with 40 or more targets: the slope,
        // the spread of Catching between them, and the drops' spread that
        // Catching alone gives (slope × that spread), beside the true sd above.
        let hands: Vec<(f64, f64)> = men
            .iter()
            .filter(|m| m.1[0] >= 40.0)
            .map(|m| (who.get(&m.0).map(|w| w.4).unwrap_or(0.0), m.1[2] / m.1[0]))
            .collect();
        let sd_hands = {
            let k = hands.len().max(2) as f64;
            let m = hands.iter().map(|x| x.0).sum::<f64>() / k;
            (hands.iter().map(|x| (x.0 - m).powi(2)).sum::<f64>() / (k - 1.0)).sqrt()
        };
        println!(
            "      a 10 points of Catching: drops a target {:+.2} pts; Catching spreads {:.1} points between them, so it alone spreads drops {:.2}",
            1000.0 * slope(&hands).0,
            sd_hands,
            100.0 * slope(&hands).0.abs() * sd_hands
        );
        if let Some(fr) = fr {
            let forty_of = |id: u32| who.get(&id).map(|w| w.3).unwrap_or(0.0);
            let fast: Vec<&(u32, [f64; 10])> = men.iter().filter(|m| m.1[0] >= 40.0).collect();
            let adot: Vec<(f64, f64)> = fast.iter().map(|m| (forty_of(m.0), m.1[8] / m.1[0])).collect();
            let deep: Vec<(f64, f64)> = fast.iter().map(|m| (forty_of(m.0), m.1[9] / m.1[0])).collect();
            let real_deep = if fr[1].is_nan() { String::new() } else { format!(" (real {:+.2})", 100.0 * fr[1]) };
            println!(
                "      a 0.1 s slower forty: aDOT {:+.2} yd (real {:+.2}), targets 20+ yards {:+.2} pts{real_deep}",
                0.1 * slope(&adot).0,
                fr[0],
                10.0 * slope(&deep).0,
            );
        }
    }
}

/// AFM_PICKS=1: where picks and breakups come from (scheme-fit lane SF5,
/// items 15 to 17; the audit's table in `afm/scheme-fit/full-pass.md`,
/// "Where picks come from"), over every targeted throw in the league: throws
/// and picks by air yards, picks by the interceptor's position (all, behind
/// man coverage, on throws of 30+ air yards), the linemen's picks made while
/// in a block, and breakups a 100 targeted throws by position with picks left
/// out. Real: the 2024 NFL with FTN coverage charting (`docs/design/scheme-fit.md`).
fn pick_diag(
    results: &[afm_engine::game::GameResult],
    pos_of: &std::collections::HashMap<u32, afm_engine::positions::Position>,
) {
    use afm_engine::positions::Position as P;
    // 0 the line (tackles and edges), 1 linebackers, 2 corners, 3 safeties, 4 anyone else.
    let grp = |id: Option<u32>| match id.and_then(|i| pos_of.get(&i)) {
        Some(P::DT | P::EDGE) => 0,
        Some(P::LB) => 1,
        Some(P::CB) => 2,
        Some(P::S) => 3,
        _ => 4,
    };
    // Upper end of each band, its name, and the real share of throws, pick
    // rate, completion rate (%) and breakups a 100 throws by anyone (2024
    // NFL play-by-play, regular season: incompletions with a pass defensed).
    const BANDS: [(f64, &str, f64, f64, f64, f64); 5] = [
        (0.0, "behind the line", 18.2, 0.80, 78.7, 7.38),
        (10.0, "0-9", 49.7, 1.22, 71.7, 8.21),
        (20.0, "10-19", 20.7, 3.39, 55.0, 13.60),
        (30.0, "20-29", 7.1, 6.06, 38.3, 15.91),
        (f64::INFINITY, "30+", 4.3, 6.72, 28.9, 18.84),
    ];
    // Per band: throws, picks, completions, breakups by the line, by the back seven.
    let mut band = [[0.0f64; 5]; 5];
    let mut int_by = [[0.0f64; 5]; 3];
    // Picks by band and the interceptor's group (scheme-fit lane SF6).
    let mut int_band = [[0.0f64; 5]; 5];
    let mut pbu_by = [0.0f64; 5];
    let (mut tgt, mut dl_blk, mut drops) = (0.0f64, 0.0f64, 0.0f64);
    for r in results {
        for p in &r.stats.plays {
            if p.nullified || p.kind != PlayType::Pass || p.throw_away || p.target.is_none() {
                continue;
            }
            tgt += 1.0;
            let b = BANDS.iter().position(|x| p.air_yards < x.0).unwrap_or(4);
            band[b][0] += 1.0;
            band[b][2] += f64::from(u8::from(p.complete));
            drops += f64::from(u8::from(p.drop));
            if p.interception {
                band[b][1] += 1.0;
                let g = grp(p.interceptor);
                int_by[0][g] += 1.0;
                int_band[b][g] += 1.0;
                if p.coverage_man {
                    int_by[1][g] += 1.0;
                }
                if b == 4 {
                    int_by[2][g] += 1.0;
                }
                if g == 0 && p.interceptor.is_some_and(|id| p.blocks.iter().any(|x| x.1 == id)) {
                    dl_blk += 1.0;
                }
            } else if p.pbu.is_some() && !p.complete {
                let g = grp(p.pbu);
                pbu_by[g] += 1.0;
                band[b][if g == 0 { 3 } else { 4 }] += 1.0;
            }
        }
    }
    let picks: f64 = int_by[0].iter().sum();
    println!(
        "\nWhere picks and breakups come from (AFM_PICKS): {tgt:.0} targeted throws, picked {:.2}% (real 2.3%)",
        100.0 * picks / tgt.max(1.0)
    );
    for (k, (_, name, real_share, real_int, real_cmp, real_pbu)) in BANDS.iter().enumerate() {
        let n = band[k][0].max(1.0);
        println!(
            "  {name:<16} share {:5.1}% (real {real_share:4.1}%)  picked {:5.2}% (real {real_int:4.2}%)  complete {:5.1}% (real {real_cmp:4.1}%)  broken up {:5.2} (real {real_pbu:5.2}; the line {:4.2}, the back seven {:5.2})",
            100.0 * band[k][0] / tgt.max(1.0),
            100.0 * band[k][1] / n,
            100.0 * band[k][2] / n,
            100.0 * (band[k][3] + band[k][4]) / n,
            100.0 * band[k][3] / n,
            100.0 * band[k][4] / n
        );
    }
    let share = |row: &[f64; 5], g: usize| 100.0 * row[g] / row.iter().sum::<f64>().max(1.0);
    for (k, (name, real)) in [
        ("all picks", "line 4% (+5% 3-4 OLBs), safeties 39%"),
        ("behind man", "safeties 43%"),
        ("on 30+ throws", "safeties 56-61%"),
    ]
    .iter()
    .enumerate()
    {
        println!(
            "  {name:<14} ({:.0}): line {:4.1}%, linebackers {:4.1}%, corners {:4.1}%, safeties {:4.1}%  (real {real})",
            int_by[k].iter().sum::<f64>(),
            share(&int_by[k], 0),
            share(&int_by[k], 1),
            share(&int_by[k], 2),
            share(&int_by[k], 3)
        );
    }
    for (k, (_, name, ..)) in BANDS.iter().enumerate() {
        println!(
            "  picks on {name:<16} ({:.0}): line {:4.1}%, linebackers {:4.1}%, corners {:4.1}%, safeties {:4.1}%",
            int_band[k].iter().sum::<f64>(),
            share(&int_band[k], 0),
            share(&int_band[k], 1),
            share(&int_band[k], 2),
            share(&int_band[k], 3)
        );
    }
    println!(
        "  the line's picks made while in a block: {dl_blk:.0} of {:.0} (the audit's league: 175 of 195)",
        int_by[0][0]
    );
    let per = |x: f64| 100.0 * x / tgt.max(1.0);
    println!(
        "  breakups a 100 targeted throws, picks left out: the line {:.2} (real 2.0), linebackers {:.2}, corners {:.2}, safeties {:.2}; the back seven {:.2} (real 8.2); drops {:.2}",
        per(pbu_by[0]),
        per(pbu_by[1]),
        per(pbu_by[2]),
        per(pbu_by[3]),
        per(pbu_by[1] + pbu_by[2] + pbu_by[3]),
        per(drops)
    );
}
