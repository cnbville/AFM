//! The other leagues in a small world (later extras X7): every season's
//! table, champion, scoring against the real league, the CFL's ratio, the
//! draft, generated players and the coaches' seats.
//! `leagues_check <start year> <history years> [scale] [seed]`; college
//! games use the stand-in for speed (the leagues' own games are on the
//! engine).

use afm_engine::world::pro::other as o;
use afm_engine::world::{World, WorldOptions};
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let start: i32 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(2002);
    let years: u32 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(6);
    let scale: f64 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.03);
    let seed: u32 = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(7);
    let t = Instant::now();
    let w = World::new(WorldOptions {
        seed,
        scale,
        history_years: years,
        start_year: start,
        engine_from: Some(2100),
        hs_engine_years: None,
        ..WorldOptions::default()
    });
    println!("world {start} with {years} years of history at scale {scale}: {:.0}s", t.elapsed().as_secs_f64());
    report(&w);
}

fn report(w: &World) {
    let x = &w.others;
    println!("\n## Seasons");
    println!("| League | Year | Teams | Games | Points a team game (real) | Champion | Final | Note |");
    println!("|---|---|---|---|---|---|---|---|");
    for s in &x.seasons {
        let games: Vec<_> = s.games.iter().filter(|g| g.round == 0).collect();
        let pts: f64 =
            games.iter().map(|g| (g.score[0] + g.score[1]) as f64).sum::<f64>() / (2.0 * games.len().max(1) as f64);
        let real = o::book(s.league).real_points().map_or(String::new(), |r| format!("{r:.1}"));
        println!(
            "| {} | {} | {} | {} | {:.1} ({real}) | {} | {} | {} |",
            o::league_short(s.league),
            s.year,
            s.table.len(),
            games.len(),
            pts,
            s.champion.map_or("-", |c| o::name(c as usize, s.year)),
            s.runner_up.map_or(String::new(), |r| format!(
                "{}-{} over {}",
                s.final_score[0],
                s.final_score[1],
                o::name(r as usize, s.year)
            )),
            s.note
        );
    }
    println!("\n## Live seasons");
    for l in &x.live {
        println!(
            "{} {}: week {}, round {}, {} games",
            o::league_short(l.league),
            l.year,
            l.week,
            l.round,
            l.games.len()
        );
    }
    println!("\n## Rosters now");
    let mut nat_total = (0usize, 0usize);
    for (fr, t) in x.teams.iter().enumerate() {
        if t.roster.is_empty() {
            continue;
        }
        let k = o::league_of(fr);
        let nat = t.roster.iter().filter(|&&i| x.national(w.people[i as usize].player.id)).count();
        if k == o::CFL {
            nat_total.0 += nat;
            nat_total.1 += t.roster.len();
        }
        let hc = t.head_coach.map_or("-".to_string(), |c| w.coaches[c].name.clone());
        let ovr: f64 =
            t.roster.iter().map(|&i| w.people[i as usize].player.overall()).sum::<f64>() / t.roster.len() as f64;
        println!(
            "{} ({}): {} players, {nat} Canadians, mean overall {ovr:.1}, HC {hc}, OC {}, DC {}, titles {}",
            o::name(fr, w.year),
            o::league_short(k),
            t.roster.len(),
            t.oc.is_some(),
            t.dc.is_some(),
            t.titles
        );
    }
    println!("CFL: {} Canadians on {} roster places", nat_total.0, nat_total.1);
    println!("\n## Draft: {} picks, {} nationals known", x.draft.len(), x.nationals.len());
    for p in x.draft.iter().rev().take(5) {
        println!(
            "  {} pick {}: {} ({}), {} to {}",
            p.year,
            p.pick,
            p.name,
            p.pos.name(),
            o::U_SPORTS[p.school as usize],
            o::name(p.team as usize, p.year)
        );
    }
    // Players with other-league seasons, and the best lines.
    let mut with = 0;
    let mut nfl_after = 0;
    let mut best: Vec<(f32, String)> = Vec::new();
    for p in &w.people {
        let Some(c) = &p.pro else { continue };
        if c.other.is_empty() {
            continue;
        }
        with += 1;
        let first = c.other.iter().map(|s| s.year).min().unwrap_or(0);
        if c.seasons.iter().any(|s| s.year > first && s.games > 0) {
            nfl_after += 1;
        }
        for s in &c.other {
            if p.player.position == afm_engine::positions::Position::QB {
                best.push((
                    s.pass_yds,
                    format!(
                        "{} {} {} ({}): {}",
                        s.year,
                        o::league_short(s.league),
                        p.name,
                        o::name(s.team as usize, s.year),
                        s.line(p.player.position)
                    ),
                ));
            }
        }
    }
    best.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("\n## Players: {with} with other-league seasons, {nfl_after} played in the NFL after");
    for (_, l) in best.iter().take(8) {
        println!("  {l}");
    }
    // Coaches who ever held a seat in another league.
    let held = w
        .coaches
        .iter()
        .filter(|c| c.history.iter().any(|s| matches!(s.employer, afm_engine::world::coach::Employer::Other(_))))
        .count();
    let now = w
        .coaches
        .iter()
        .filter(|c| c.job.is_some_and(|j| matches!(j.employer, afm_engine::world::coach::Employer::Other(_))))
        .count();
    let moved_up = w
        .coaches
        .iter()
        .filter(|c| {
            let other_to = c
                .history
                .iter()
                .filter(|s| matches!(s.employer, afm_engine::world::coach::Employer::Other(_)))
                .map(|s| s.to)
                .max();
            other_to.is_some_and(|y| {
                c.job.is_some_and(|j| {
                    matches!(
                        j.employer,
                        afm_engine::world::coach::Employer::Nfl(_) | afm_engine::world::coach::Employer::College(_)
                    )
                }) && c
                    .history
                    .iter()
                    .all(|s| s.to <= y || !matches!(s.employer, afm_engine::world::coach::Employer::Other(_)))
            })
        })
        .count();
    println!(
        "\n## Coaches: {now} in other-league seats now, {held} with an other-league stint behind them, {moved_up} of those now in the NFL or college"
    );
    println!("people {}", w.people.len());
}
