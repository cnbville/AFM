//! The freak check (Cain 2026-11-08: "be careful to not cap the game
//! creating rare freaks"; `afm/ab-tests/research-edge.md`, "No caps on
//! freaks"): do undersized edges hold their real share of the top pressure
//! seasons, do the best seasons reach real's, and does a 6-0, 240 lb edge
//! with top Strength, Agility and Finesse Rush rush with the league's best?
//!
//! Usage: freak_check [seasons] [seed] [synthetic | drawn | world file]
//! `synthetic` (the default) plays the synthetic NFL league as it comes, its
//! edges' bodies from the real combine (the rosters every check uses).
//! `drawn` redraws every edge with the game's own generator at his overall
//! (athlete-first, natural edges at 14 kept whatever they grow into, so it
//! has men the world's coaches would move inside or off the ball). A world
//! file plays that world's 32 NFL clubs.
//! Each season is 17 games a team. Real (PFR, edges with 200+ defense
//! snaps, 2018-2025): of the top 10% of pressure seasons, under 250 lb 18%
//! (19% of all), arms under 33 in 32% (24%), 6-5 or taller 21% (29%); the
//! best sack seasons 22.5 (T.J. Watt 2021) and 23 (Myles Garrett 2025).

extern crate afm_tools;
use std::collections::HashMap;

use afm_engine::attributes::Attr;
use afm_engine::game::{Conditions, Venue, quick_game_in};
use afm_engine::player::Level;
use afm_engine::positions::{Position, overall_for};
use afm_tools::ab::{Arm, Knob, Part, Who, change_one, synthetic, teams_for, world_nfl};
use rayon::prelude::*;

/// One edge's season: his id, the season, his counts, and his body
/// (height, weight, arm).
type EdgeSeason = (u32, usize, Season, (f64, f64, f64));

#[derive(Default, Clone)]
struct Season {
    snaps: u32,
    rush: u32,
    pressures: u32,
    sacks: f64,
    /// Rush wins by move: speed, power, finesse.
    wins: [u32; 3],
}

impl Season {
    fn add(&mut self, x: &Season) {
        self.snaps += x.snaps;
        self.rush += x.rush;
        self.pressures += x.pressures;
        self.sacks += x.sacks;
        for k in 0..3 {
            self.wins[k] += x.wins[k];
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seasons: usize = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
    let seed: u32 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(76);
    let mode = a.get(3).cloned().unwrap_or_else(|| "synthetic".into());
    let world = (mode != "synthetic" && mode != "drawn").then(|| mode.clone());
    let level = Level::NFL;
    let base = match &world {
        Some(p) => world_nfl(p),
        None => synthetic(level, seed),
    };
    let mut teams = if mode == "drawn" {
        let arm = Arm {
            label: "drawn".into(),
            groups: vec![(
                Who::at(&[Position::EDGE]),
                vec![Knob::Recruit(Part::Height, f64::NEG_INFINITY, f64::INFINITY)],
            )],
            mine: false,
        };
        teams_for(&base, &arm, level)
    } else {
        base.iter().map(|s| s.team.clone()).collect::<Vec<_>>()
    };
    // The freak: team 0's best edge, 6-0 and 240 lb, with top Strength,
    // Agility and Finesse Rush (his other ratings and his arms his own).
    // He is built from the league's own man at that spot, so his wingspan,
    // hands and recovery are his (in `drawn` mode too, where the drawn
    // man's would otherwise sit beside another man's body).
    let src = &base[0].team.players;
    let k = (0..src.len())
        .filter(|&i| src[i].position == Position::EDGE)
        .max_by(|&x, &y| {
            let o = |i: usize| overall_for(Position::EDGE, &src[i].attrs);
            o(x).total_cmp(&o(y))
        })
        .expect("an edge");
    let person = {
        let mut p = base[0].people[k].clone();
        p.attributes = src[k].attrs;
        p
    };
    let knobs = [
        Knob::Set(Part::Height, 72.0),
        Knob::Set(Part::Weight, 240.0),
        Knob::Rating(Attr::Strength, 99.0),
        Knob::Rating(Attr::Agility, 99.0),
        Knob::Rating(Attr::FinesseRush, 99.0),
    ];
    let mut gp = src[k].clone();
    change_one(&mut gp, &person, &knobs, level);
    teams[0].players[k] = gp;
    let freak = teams[0].players[k].id;
    eprintln!(
        "freak_check: {seasons} seasons, seed {seed}, {}; the freak is id {freak} ({:.0} in, {:.0} lb, arm {:.2})",
        match mode.as_str() {
            "synthetic" => "synthetic league",
            "drawn" => "synthetic league, generator edges",
            _ => "world rosters",
        },
        teams[0].players[k].height,
        teams[0].players[k].weight,
        teams[0].players[k].arm
    );
    let n = teams.len();
    let body: HashMap<u32, (f64, f64, f64, Position)> =
        teams.iter().flat_map(|t| t.players.iter().map(|p| (p.id, (p.height, p.weight, p.arm, p.position)))).collect();
    // What each edge brings: Strength, Power Rush, Finesse Rush, Hand Use,
    // Acceleration, Agility, Flexibility, Speed, Tackling.
    let rat: HashMap<u32, [f64; 9]> = teams
        .iter()
        .flat_map(|t| t.players.iter())
        .map(|p| {
            let g = |x: Attr| p.attrs.get(x);
            (
                p.id,
                [
                    g(Attr::Strength),
                    g(Attr::PowerRush),
                    g(Attr::FinesseRush),
                    g(Attr::HandUse),
                    g(Attr::Acceleration),
                    g(Attr::Agility),
                    g(Attr::Flexibility),
                    g(Attr::Speed),
                    g(Attr::Tackling),
                ],
            )
        })
        .collect();
    let who = |id: u32, x: &Season| {
        let r = rat.get(&id).copied().unwrap_or_default();
        let w = x.wins.iter().sum::<u32>().max(1) as f64;
        format!(
            "Str {:.0} PR {:.0} FR {:.0} HU {:.0} Acc {:.0} Agi {:.0} Flex {:.0} Spd {:.0} Tk {:.0}; {} rush snaps, {:.1}% pressured; wins {:.0}/{:.0}/{:.0}% speed/power/finesse",
            r[0],
            r[1],
            r[2],
            r[3],
            r[4],
            r[5],
            r[6],
            r[7],
            r[8],
            x.rush,
            100.0 * x.pressures as f64 / x.rush.max(1) as f64,
            100.0 * x.wins[0] as f64 / w,
            100.0 * x.wins[1] as f64 / w,
            100.0 * x.wins[2] as f64 / w
        )
    };
    let venue =
        || Venue { cond: Conditions { crowd: 0.65, ..Conditions::default() }, wear: [0.0, 0.012], ..Venue::default() };
    let mut lines: Vec<(u32, usize, Season)> = Vec::new();
    for s in 0..seasons {
        // A round robin (the circle method), 17 of its 31 rounds, turned by
        // the season.
        let mut games = Vec::new();
        for w in 0..17 {
            let r = (w + 3 * s) % (n - 1);
            for i in 0..n / 2 {
                let x = if i == 0 { n - 1 } else { (r + i) % (n - 1) };
                let y = (r + n - 1 - i) % (n - 1);
                games.push(if (w + i) % 2 == 0 { (x, y) } else { (y, x) });
            }
        }
        let played: Vec<HashMap<u32, Season>> = games
            .par_iter()
            .enumerate()
            .map(|(g, &(h, aw))| {
                let r = quick_game_in(
                    &teams[h],
                    &teams[aw],
                    level,
                    seed as u64 * 1_000_003 + (s * 1000 + g) as u64,
                    Default::default(),
                    venue(),
                );
                let mut m = HashMap::new();
                for side in 0..2 {
                    for (&id, l) in &r.stats.players[side] {
                        let e: &mut Season = m.entry(id).or_default();
                        e.add(&Season {
                            snaps: l.snaps_def,
                            rush: l.rush_snaps,
                            pressures: l.pressures,
                            sacks: l.sacks,
                            wins: [l.speed_wins, l.power_wins, l.finesse_wins],
                        });
                    }
                }
                m
            })
            .collect();
        let mut tot: HashMap<u32, Season> = HashMap::new();
        for m in played {
            for (id, x) in m {
                tot.entry(id).or_default().add(&x);
            }
        }
        for (id, x) in tot {
            lines.push((id, s, x));
        }
    }
    let mut edge: Vec<EdgeSeason> = lines
        .into_iter()
        .filter_map(|(id, s, x)| {
            let &(h, w, arm, pos) = body.get(&id)?;
            (pos == Position::EDGE && x.snaps >= 200).then_some((id, s, x, (h, w, arm)))
        })
        .collect();
    let rate = |x: &Season| x.pressures as f64 / x.snaps.max(1) as f64;
    edge.sort_by(|p, q| rate(&q.2).total_cmp(&rate(&p.2)));
    let all = edge.len();
    let top = (all as f64 * 0.10).round() as usize;
    let share = |v: &[EdgeSeason], f: &dyn Fn(&(f64, f64, f64)) -> bool| {
        100.0 * v.iter().filter(|e| f(&e.3)).count() as f64 / v.len().max(1) as f64
    };
    let light = |b: &(f64, f64, f64)| b.1 < 250.0;
    let short = |b: &(f64, f64, f64)| b.2 < 33.0;
    let tall = |b: &(f64, f64, f64)| b.0 >= 77.0;
    let mean = |f: &dyn Fn(&Season) -> f64| edge.iter().map(|e| f(&e.2)).sum::<f64>() / all.max(1) as f64;
    println!("Edge seasons with 200+ defense snaps: {all} (top 10%: {top})");
    let bw: Vec<f64> = edge.iter().map(|e| e.3.1).collect();
    let mw = bw.iter().sum::<f64>() / all.max(1) as f64;
    let sw = (bw.iter().map(|w| (w - mw).powi(2)).sum::<f64>() / all.max(1) as f64).sqrt();
    println!(
        "  their weight {mw:.0} lb (sd {sw:.0}); the top 10% start at {:.2}% pressures a defense snap",
        100.0 * edge.get(top.saturating_sub(1)).map_or(0.0, |e| rate(&e.2))
    );
    println!(
        "  mean pressures a defense snap {:.2}%, sacks a defense snap {:.2}%, sacks a pressure {:.1}% (real 3.44%, 0.94%, 27.3%)",
        100.0 * mean(&|x| rate(x)),
        100.0 * mean(&|x| x.sacks / x.snaps.max(1) as f64),
        100.0 * edge.iter().map(|e| e.2.sacks).sum::<f64>()
            / edge.iter().map(|e| e.2.pressures as f64).sum::<f64>().max(1.0)
    );
    let t = &edge[..top];
    println!("                      top 10%   all   real top / all");
    println!("  under 250 lb        {:5.1}%  {:5.1}%   18% / 19%", share(t, &light), share(&edge, &light));
    println!("  arms under 33 in    {:5.1}%  {:5.1}%   32% / 24%", share(t, &short), share(&edge, &short));
    println!("  6-5 or taller       {:5.1}%  {:5.1}%   21% / 29%", share(t, &tall), share(&edge, &tall));
    let heavy = |b: &(f64, f64, f64)| b.1 >= 290.0;
    println!("  290 lb or more      {:5.1}%  {:5.1}%", share(t, &heavy), share(&edge, &heavy));
    let mut by_sacks = edge.clone();
    by_sacks.sort_by(|p, q| q.2.sacks.total_cmp(&p.2.sacks));
    println!("Best sack seasons (real best 22.5 and 23):");
    for e in by_sacks.iter().take(8) {
        println!(
            "  {:5.1} sacks, {:3} pressures, {:.1}% a snap  id {} season {} ({:.0} in, {:.0} lb, arm {:.2}){}",
            e.2.sacks,
            e.2.pressures,
            100.0 * rate(&e.2),
            e.0,
            e.1,
            e.3.0,
            e.3.1,
            e.3.2,
            if e.0 == freak { "  <- the freak" } else { "" }
        );
        println!("        {}", who(e.0, &e.2));
    }
    let mut sacks: Vec<f64> = edge.iter().map(|e| e.2.sacks).collect();
    sacks.sort_by(|a, b| b.total_cmp(a));
    let q = |p: f64| sacks[((sacks.len() as f64 * p) as usize).min(sacks.len() - 1)];
    println!(
        "  sacks a season at the top 1%, 5%, 10%, 50%: {:.1}, {:.1}, {:.1}, {:.1}",
        q(0.01),
        q(0.05),
        q(0.10),
        q(0.5)
    );
    let mine: Vec<&EdgeSeason> = edge.iter().filter(|e| e.0 == freak).collect();
    if mine.is_empty() {
        println!("The freak played under 200 snaps a season.");
    }
    for e in mine {
        let rank = edge.iter().position(|x| x.0 == e.0 && x.1 == e.1).unwrap_or(0);
        println!(
            "The freak, season {}: {:.1} sacks, {} pressures, {:.2}% a snap, rank {} of {} (top {:.1}%)",
            e.1,
            e.2.sacks,
            e.2.pressures,
            100.0 * rate(&e.2),
            rank + 1,
            all,
            100.0 * (rank + 1) as f64 / all as f64
        );
        println!("        {}", who(e.0, &e.2));
    }
}
