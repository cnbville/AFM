//! Role check: does a college player's season role ("Starter" on his
//! career page) match the snaps and targets he really got? Builds a small
//! world with the history on the stand-in, then plays one season on the
//! engine day by day. At fall camp it keeps each college player's role
//! (the season row the development step writes) and, every game week, his
//! place on the game-day chart (overall at his spot plus practice
//! impression, `game::roster::depth_chart`'s order). After the season it
//! reads the stat book, and each row's role as the season left it.
//! Usage: role_check [seed] [history_years] [scale]

use std::collections::HashMap;

use afm_engine::game::roster::GamePlayer;
use afm_engine::league::depth_slots;
use afm_engine::player::{Level, PlayingTime};
use afm_engine::positions::Position;
use afm_engine::world::records::Stat;
use afm_engine::world::{Stage, World, WorldOptions};

#[derive(Default, Clone)]
struct Seen {
    program: usize,
    pos: Option<Position>,
    role: Option<PlayingTime>,
    /// His row's role once the season is over.
    end: Option<PlayingTime>,
    seasons: u32,
    /// Game-day weeks checked, and weeks in the starting group.
    weeks: u32,
    started: u32,
    /// Camp: his place on the game-day order at his position.
    rank_camp: usize,
}

fn record_season(p: &afm_engine::world::Person, k: usize, year_now: i32) -> i32 {
    let hist = &p.player.history;
    if p.born == afm_engine::calendar::born::UNKNOWN {
        return year_now - hist.len() as i32 + k as i32;
    }
    p.born.year() + hist[k].age as i32 + i32::from(p.born.month() > 8)
}

/// Each program's game-day order at each position, by overall at the
/// position plus practice impression, best first.
fn game_day(w: &World) -> HashMap<u32, usize> {
    let mut rank = HashMap::new();
    for prog in &w.programs {
        let mut by: HashMap<Position, Vec<(f64, u32)>> = HashMap::new();
        for &i in &prog.roster {
            let p = &w.people[i];
            if p.stage != Stage::College {
                continue;
            }
            let g = GamePlayer::from_player(&p.player, 0.9, 0.0);
            by.entry(p.player.position)
                .or_default()
                .push((g.overall_at(p.player.position) + g.impression, p.player.id));
        }
        for (_, mut v) in by {
            v.sort_by(|a, b| b.0.total_cmp(&a.0));
            for (k, (_, id)) in v.into_iter().enumerate() {
                rank.insert(id, k);
            }
        }
    }
    rank
}

fn role_name(r: Option<PlayingTime>) -> &'static str {
    match r {
        Some(PlayingTime::Starter) => "Starter",
        Some(PlayingTime::Rotation) => "Rotation",
        Some(PlayingTime::Backup) => "Backup",
        Some(PlayingTime::Redshirt) => "Redshirt",
        Some(_) => "Other",
        None => "-",
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(73);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let scale: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.2);
    let t = std::time::Instant::now();
    let start = 2026;
    let opts = WorldOptions {
        seed,
        history_years: years,
        keep_game_log: false,
        scale,
        engine_from: Some(start),
        ..Default::default()
    };
    let mut w = World::new(opts);
    println!("history built in {:.0}s; year {}", t.elapsed().as_secs_f64(), w.year);
    let year = w.year;
    let mut seen: HashMap<u32, Seen> = HashMap::new();
    let mut camp_done = false;
    let mut last_week = 0;
    while w.year == year {
        w.advance_day();
        let d = w.today;
        if !camp_done {
            // Camp: most college players carry this season's row.
            let (mut n, mut with) = (0, 0);
            for p in w.people.iter().filter(|p| p.stage == Stage::College) {
                n += 1;
                let h = &p.player.history;
                if !h.is_empty() && record_season(p, h.len() - 1, w.year) == year {
                    with += 1;
                }
            }
            if n > 0 && with * 10 >= n * 9 {
                camp_done = true;
                let rank = game_day(&w);
                for (pi, prog) in w.programs.iter().enumerate() {
                    for &i in &prog.roster {
                        let p = &w.people[i];
                        if p.stage != Stage::College {
                            continue;
                        }
                        let h = &p.player.history;
                        let Some(row) = h.last().filter(|_| record_season(p, h.len() - 1, w.year) == year) else {
                            continue;
                        };
                        if !matches!(row.level, Level::FBS | Level::FCS | Level::SmallCollege) {
                            continue;
                        }
                        seen.insert(
                            p.player.id,
                            Seen {
                                program: pi,
                                pos: Some(row.position),
                                role: Some(row.playing_time),
                                seasons: p.college_seasons,
                                rank_camp: rank.get(&p.player.id).copied().unwrap_or(99),
                                ..Default::default()
                            },
                        );
                    }
                }
                println!("camp on {d:?}: {} college players kept", seen.len());
            }
        } else if d.year() > year {
            // After the bowls: each row's role as the season left it.
            for (id, s) in seen.iter_mut() {
                let Ok(i) = w.people.binary_search_by_key(id, |p| p.player.id) else { continue };
                let p = &w.people[i];
                let h = &p.player.history;
                if let Some(k) = (0..h.len()).rev().find(|&k| record_season(p, k, w.year) == year) {
                    s.end = Some(h[k].playing_time);
                }
            }
        } else if (d.month() >= 9 && d.month() <= 11) && d.day_of_year() / 7 != last_week {
            last_week = d.day_of_year() / 7;
            let rank = game_day(&w);
            for (id, s) in seen.iter_mut() {
                let Some(pos) = s.pos else { continue };
                if let Some(&r) = rank.get(id) {
                    s.weeks += 1;
                    if r < depth_slots(pos).starters {
                        s.started += 1;
                    }
                }
            }
        }
    }
    println!("season played in {:.0}s", t.elapsed().as_secs_f64());

    // The stat book's line for the season.
    let line = |id: u32| w.stats.career(id).into_iter().find(|r| r.year as i32 == year).cloned();
    let fbs = |pi: usize| matches!(w.programs[pi].division, afm_engine::world::data::Division::FBS);

    for (title, at_end) in [("camp role", false), ("role on his row after the season", true)] {
        println!("\nFBS players by {title}: snaps a game (offense or defense) and games");
        println!(
            "{:<6} {:<9} {:>5} {:>8} {:>8} {:>8} {:>10}",
            "pos", "role", "n", "snaps/g", "<15/g", "games", "gd start%"
        );
        for pos in Position::ALL {
            for role in [PlayingTime::Starter, PlayingTime::Rotation, PlayingTime::Backup] {
                let mut v: Vec<(f64, f64, f64)> = Vec::new();
                for (&id, s) in &seen {
                    let r = if at_end { s.end } else { s.role };
                    if s.pos != Some(pos) || r != Some(role) || !fbs(s.program) {
                        continue;
                    }
                    let (snaps, games) = match line(id) {
                        Some(r) => (r.get(Stat::SnapsOff) + r.get(Stat::SnapsDef), r.games as f64),
                        None => (0.0, 0.0),
                    };
                    let gd = if s.weeks > 0 { s.started as f64 / s.weeks as f64 } else { 0.0 };
                    v.push((if games > 0.0 { snaps / games } else { 0.0 }, games, gd));
                }
                if v.is_empty() {
                    continue;
                }
                let n = v.len() as f64;
                let mut sg: Vec<f64> = v.iter().map(|x| x.0).collect();
                sg.sort_by(|a, b| a.total_cmp(b));
                println!(
                    "{:<6} {:<9} {:>5} {:>8.1} {:>7.0}% {:>8.1} {:>9.0}%",
                    pos.name(),
                    role_name(Some(role)),
                    v.len(),
                    sg[sg.len() / 2],
                    100.0 * v.iter().filter(|x| x.0 < 15.0).count() as f64 / n,
                    v.iter().map(|x| x.1).sum::<f64>() / n,
                    100.0 * v.iter().map(|x| x.2).sum::<f64>() / n,
                );
            }
        }
    }

    println!("\nFBS camp-starter receivers with 5 catches or fewer");
    println!(
        "{:<22} {:>3} {:>5} {:>5} {:>6} {:>6} {:>5} {:>5} {:>6} {:>6}  row after",
        "program", "yrs", "camp", "gd%", "games", "snaps", "rte", "tgt", "rec", "ovr"
    );
    let mut wr: Vec<(u32, &Seen)> = seen
        .iter()
        .filter(|(_, s)| s.pos == Some(Position::WR) && s.role == Some(PlayingTime::Starter) && fbs(s.program))
        .map(|(&id, s)| (id, s))
        .collect();
    wr.sort_by_key(|x| x.0);
    let total = wr.len();
    let mut low = 0;
    let mut tgts: Vec<f64> = Vec::new();
    for (id, s) in &wr {
        let r = line(*id);
        let g = |k: Stat| r.as_ref().map_or(0.0, |r| r.get(k));
        tgts.push(g(Stat::Targets));
        if g(Stat::Rec) > 5.0 {
            continue;
        }
        low += 1;
        let ovr = w.people.iter().find(|p| p.player.id == *id).map_or(f64::NAN, |p| p.player.overall());
        println!(
            "{:<22} {:>3} {:>5} {:>4.0}% {:>6} {:>6.0} {:>5.0} {:>5.0} {:>6.0} {:>6.1}  {}",
            w.programs[s.program].real_name.chars().take(22).collect::<String>(),
            s.seasons,
            s.rank_camp + 1,
            if s.weeks > 0 { 100.0 * s.started as f64 / s.weeks as f64 } else { 0.0 },
            r.as_ref().map_or(0, |r| r.games),
            g(Stat::SnapsOff),
            g(Stat::Routes),
            g(Stat::Targets),
            g(Stat::Rec),
            ovr,
            role_name(s.end),
        );
    }
    tgts.sort_by(|a, b| a.total_cmp(b));
    println!(
        "{low} of {total} FBS camp-starter receivers caught 5 or fewer; targets 10th/50th/90th: {:.0}/{:.0}/{:.0}",
        tgts.get(tgts.len() / 10).copied().unwrap_or(0.0),
        tgts.get(tgts.len() / 2).copied().unwrap_or(0.0),
        tgts.get(tgts.len() * 9 / 10).copied().unwrap_or(0.0)
    );
}
