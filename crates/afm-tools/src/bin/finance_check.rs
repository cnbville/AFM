//! Money check (later extras topic 1): builds a world and prints the books
//! against real targets, year by year: NFL revenue, attendance, sellouts
//! and blackouts, club values; college department totals by tier,
//! attendance and realignment; high school budgets; and the betting lines'
//! accuracy and the upsets they call.
//! Usage: finance_check [seed] [history_years] [future_years] [scale]

use afm_engine::world::data::Division;
use afm_engine::world::finance::{college, hs, nfl};
use afm_engine::world::{World, WorldOptions};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let future: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
    let scale: f64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.1);
    let engine_from = std::env::var("AFM_ENGINE_FROM").ok().and_then(|v| v.parse().ok());
    let opts =
        WorldOptions { seed, history_years: years, keep_game_log: false, scale, engine_from, ..Default::default() };
    let mut w = World::new_watched(opts, |w, secs| println!("season {} built in {secs:.0}s", w.year - 1));
    for _ in 0..future {
        w.advance_year();
        println!("season {} played", w.year - 1);
    }
    nfl_report(&w);
    college_report(&w);
    hs_report(&w);
    lines_report(&w);
    pay_report(&w);
}

/// Coach pay by conference against the department's earned money (realism
/// audit item 9 on the seed 321 world): the mean of each rung's pay a year
/// now, in today's dollars.
fn pay_report(w: &World) {
    use afm_engine::world::coach::{Employer, Role};
    use afm_engine::world::folk::hiring::pay_now;
    println!(
        "\n== Coach pay by conference (real 2024 head coaches, USA Today, partly from memory: SEC 7.5, Big Ten 6.9, Big 12 5.0, ACC 5.0, American 2.3, Mountain West 1.6, Sun Belt 1.0, Conference USA 0.9, MAC 0.8 millions)"
    );
    let idx = afm_engine::world::finance::data::college_index(w.year);
    let mut rows: std::collections::BTreeMap<String, Vec<[f64; 6]>> = Default::default();
    for (p, prog) in w.programs.iter().enumerate() {
        if !prog.plays(w.year) {
            continue;
        }
        let d = &w.finance.college[p];
        let earned = college::earned(&d.budget, d) / idx;
        let mut r = [earned, 0.0, 0.0, 0.0, 0.0, college::Dept::costs(&d.budget) / idx];
        let (mut nc, mut np) = (0.0, 0.0);
        for (c, co) in w.coaches.iter().enumerate() {
            let Some(j) = co.job else { continue };
            if j.employer != Employer::College(p) {
                continue;
            }
            match j.role {
                Role::HeadCoach => r[1] = pay_now(w, c),
                Role::OffensiveCoordinator | Role::DefensiveCoordinator => {
                    r[2] += pay_now(w, c);
                    nc += 1.0;
                }
                Role::PositionCoach(_) => {
                    r[3] += pay_now(w, c);
                    np += 1.0;
                }
            }
        }
        r[2] /= f64::max(nc, 1.0);
        r[3] /= f64::max(np, 1.0);
        r[4] = prog.prestige;
        let key =
            if prog.division == Division::FBS { prog.conference.clone() } else { format!("({:?})", prog.division) };
        rows.entry(key).or_default().push(r);
    }
    // Each level's earned money (today's dollars): where its jobs sit.
    for dv in [Division::FBS, Division::FCS, Division::D2, Division::NAIA, Division::D3, Division::JUCO] {
        let mut v: Vec<f64> = (0..w.programs.len())
            .filter(|&p| w.programs[p].division == dv && w.programs[p].plays(w.year))
            .map(|p| college::earned(&w.finance.college[p].budget, &w.finance.college[p]) / idx)
            .collect();
        if v.is_empty() {
            continue;
        }
        v.sort_by(f64::total_cmp);
        let q = |f: f64| m2(v[((v.len() as f64 - 1.0) * f) as usize]);
        println!(
            "{dv:?} earned: n {} | min {} | p10 {} | median {} | p90 {} | max {}",
            v.len(),
            q(0.0),
            q(0.1),
            q(0.5),
            q(0.9),
            q(1.0)
        );
    }
    println!(
        "conference            n  budget  earned/school  head coach  min-max          coordinator  position  prestige"
    );
    for (k, v) in &rows {
        let n = v.len() as f64;
        let avg = |i: usize| v.iter().map(|r| r[i]).sum::<f64>() / n;
        let lo = v.iter().map(|r| r[1]).fold(f64::MAX, f64::min);
        let hi = v.iter().map(|r| r[1]).fold(0.0, f64::max);
        println!(
            "{k:<20} {:>3}  {:>6}  {:>13}  {:>10}  {:>6}-{:<8}  {:>11}  {:>8}  {:>8.2}",
            v.len(),
            m2(avg(5)),
            m2(avg(0)),
            m2(avg(1)),
            m2(lo),
            m2(hi),
            m2(avg(2)),
            m2(avg(3)),
            avg(4)
        );
    }
}

fn m(x: f64) -> String {
    format!("{:.0}M", x / 1.0e6)
}

fn m2(x: f64) -> String {
    format!("{:.2}M", x / 1.0e6)
}

fn nfl_report(w: &World) {
    println!(
        "\n== NFL (real 2024: revenue about $23B, $720M a club; 2010 about $8.5B; attendance about 69,000; Forbes 2024 average value $5.9B)"
    );
    let mut by_year: std::collections::BTreeMap<i32, Vec<(f64, f64, &nfl::Season)>> = Default::default();
    for c in &w.finance.nfl {
        for (b, s) in c.years.iter().zip(&c.seasons) {
            by_year.entry(b.year).or_default().push((nfl::Club::revenue(b), nfl::Club::costs(b), s));
        }
    }
    println!("year clubs  revenue/club (min-max)   costs/club  attendance  sellouts  blackouts  value/club  renew");
    for (y, rows) in &by_year {
        let n = rows.len() as f64;
        let rev: Vec<f64> = rows.iter().map(|r| r.0).collect();
        let games: f64 = rows.iter().map(|r| r.2.home_games as f64).sum();
        let att: f64 = rows.iter().map(|r| r.2.attendance as f64).sum::<f64>() / games.max(1.0);
        let sell: f64 = rows.iter().map(|r| r.2.sellouts as f64).sum::<f64>() / games.max(1.0);
        let black: u32 = rows.iter().map(|r| r.2.blackouts as u32).sum();
        let value = rows.iter().map(|r| r.2.value as f64).sum::<f64>() / n;
        let renew = rows.iter().map(|r| r.2.renewal as f64).sum::<f64>() / n;
        println!(
            "{y} {:>5}  {:>6} ({}-{})   {:>8}  {:>9.0}  {:>7.0}%  {:>9}  {:>9}  {:>4.0}%",
            rows.len(),
            m(rev.iter().sum::<f64>() / n),
            m(rev.iter().cloned().fold(f64::MAX, f64::min)),
            m(rev.iter().cloned().fold(0.0, f64::max)),
            m(rows.iter().map(|r| r.1).sum::<f64>() / n),
            att,
            100.0 * sell,
            black,
            m(value),
            100.0 * renew
        );
    }
    // The last year's lines, the league's average club.
    if let Some(last) = by_year.keys().last() {
        let books: Vec<_> = w.finance.nfl.iter().filter_map(|c| c.years.iter().find(|b| b.year == *last)).collect();
        println!("{last} lines, the average club:");
        for (k, line) in nfl::LINES.iter().enumerate() {
            let v = books.iter().map(|b| b.l[k] as f64).sum::<f64>() / books.len().max(1) as f64;
            println!("  {:<34} {:>8}", line.name, m(v));
        }
    }
    let moves: Vec<_> = w.finance.nfl.iter().enumerate().filter_map(|(t, c)| c.city.as_ref().map(|x| (t, x))).collect();
    println!("relocated clubs: {moves:?}");
    let p = &w.finance.projects;
    let nfl_p = p.iter().filter(|x| matches!(x.owner, afm_engine::world::finance::Owner::Nfl(_))).count();
    let col_p = p.iter().filter(|x| matches!(x.owner, afm_engine::world::finance::Owner::College(_))).count();
    println!("projects: NFL {nfl_p}, college {col_p}; report cards {}", w.finance.cards.len());
}

/// A school's year: revenue, university support, crowd a game, guarantees in.
type Row = (f64, f64, f64, f64);

fn college_report(w: &World) {
    println!(
        "\n== College (real 2023: SEC schools about $150-250M, Big Ten similar; G5 $40-70M with big university support; FCS $15-35M; FBS attendance about 41,000 a game)"
    );
    let mut by_tier: std::collections::BTreeMap<(i32, String), Vec<Row>> = Default::default();
    for (p, d) in w.finance.college.iter().enumerate() {
        let prog = &w.programs[p];
        for (b, s) in d.years.iter().zip(&d.seasons) {
            let tier = match college::tier(w, p, b.year) {
                0 => "power",
                1 => "group of five",
                _ if prog.division == Division::FCS => "FCS",
                _ => "smaller",
            };
            let games = s.home_games.max(1) as f64;
            by_tier.entry((b.year, tier.to_string())).or_default().push((
                college::Dept::revenue(b),
                b.l[college::SUPPORT] as f64,
                s.attendance as f64 / games,
                b.l[college::GUARANTEES_IN] as f64,
            ));
        }
    }
    println!("year tier            n   revenue/school  university support  attendance  guarantees in");
    for ((y, t), rows) in &by_tier {
        let n = rows.len() as f64;
        let avg = |f: &dyn Fn(&Row) -> f64| rows.iter().map(f).sum::<f64>() / n;
        println!(
            "{y} {t:<14} {:>4}  {:>14}  {:>18}  {:>10.0}  {:>13}",
            rows.len(),
            m(avg(&|r| r.0)),
            m(avg(&|r| r.1)),
            avg(&|r| r.2),
            m(avg(&|r| r.3))
        );
    }
    let moves = w.calendar.all().filter(|e| e.rule == college::MOVE_RULE).count();
    println!("conference moves posted on 1 July this year: {moves}");
    for e in w.calendar.all().filter(|e| e.rule == college::MOVE_RULE).take(8) {
        println!("  {}", e.text);
    }
    for d in &w.finance.deals {
        println!("  deal: {} {} a school to {}", d.conference, m(d.per_school as f64), d.until);
    }
}

fn hs_report(w: &World) {
    println!(
        "\n== High school (real: a typical public program spends $25-60K beyond stipends; big Texas programs $150K+)"
    );
    let mut rows: Vec<(f64, f64, u32, u16)> = w
        .finance
        .hs
        .iter()
        .map(|s| {
            let b = &s.budget;
            (b.sum(0..hs::INCOME), b.sum(hs::INCOME..hs::N) - b.l[hs::STIPENDS] as f64, s.crowd, s.fee)
        })
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let n = rows.len();
    if n == 0 {
        return;
    }
    for q in [0.1, 0.5, 0.9, 0.99] {
        let r = rows[((n as f64 * q) as usize).min(n - 1)];
        println!(
            "  p{:.0}: income ${:.0}K, spending beyond stipends ${:.0}K, crowd {}, fee ${}",
            q * 100.0,
            r.0 / 1e3,
            r.1 / 1e3,
            r.2,
            r.3
        );
    }
    let fees = rows.iter().filter(|r| r.3 > 0).count();
    println!("  schools charging a fee: {:.0}%", 100.0 * fees as f64 / n as f64);
}

fn lines_report(w: &World) {
    println!(
        "\n== Lines (real: NFL closing lines miss the margin by about 10.5 points on average, the favorite wins about 66%)"
    );
    let [n, c] = afm_engine::world::finance::lines::accuracy(w);
    println!("  NFL {} games: mean miss {:.1}, favorites won {:.0}%", n.0, n.1, 100.0 * n.2);
    println!(
        "  college {} games: mean miss {:.1} (real about 12.5), favorites won {:.0}% (real about 75%)",
        c.0,
        c.1,
        100.0 * c.2
    );
    let mut ats: Vec<(&usize, &[u16; 3])> = w.finance.ats.iter().collect();
    ats.sort_by_key(|x| std::cmp::Reverse(x.1[0] + x.1[1]));
    for (coach, r) in ats.iter().take(5) {
        println!("  coach {coach}: {}-{}-{} against the spread", r[0], r[1], r[2]);
    }
    let upsets = w.calendar.all().filter(|e| e.rule == "betting: the line").count();
    println!("  upsets by the line in the news this year: {upsets}");
}
