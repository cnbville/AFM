//! NIL money by program and era (lane NB9's check, B18): builds a world up
//! to the start of a year and saves it, or plays a saved world on and prints
//! each July's payday and each class's signed NIL offers by level.
//! It reads only what every build has, so the same file can run on main and
//! on a lane to compare them.
//! Usage: nil_check build <seed> <history years> <scale> <start year> <out>
//!        nil_check play <world file> <until year>
//!        nil_check portal <world file> <until year> <until month>: every
//!        transfer's NIL deal the day he lands (minor bug 335), by level,
//!        position and role, and each July's payday by position for starters.

use afm_engine::positions::Position;
use afm_engine::world::data::Division;
use afm_engine::world::{Stage, World, WorldOptions};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("build") => {
            // Saved after every recorded year as `<out>.part`, so a build cut
            // short (a machine restart) goes on from the last year.
            let n = |k: usize| args[k].parse::<f64>().expect("a number");
            let start_year = n(5) as i32;
            let out = Path::new(&args[6]);
            let part = out.with_extension("part");
            let save = |w: &World, to: &Path| {
                afm_engine::save::write_world(w, to, "nil_check", "nil_check", None).expect("write");
            };
            let t = std::time::Instant::now();
            let w = if part.exists() {
                let mut w = afm_engine::save::read_world(&part).expect("read the part");
                eprintln!("going on from {:?}", w.today);
                while w.year < start_year {
                    w.advance_year();
                    save(&w, &part);
                    eprintln!("year {} at {:.0}s", w.year, t.elapsed().as_secs_f64());
                }
                w
            } else {
                let opts = WorldOptions {
                    seed: n(2) as u32,
                    history_years: n(3) as u32,
                    scale: n(4),
                    start_year,
                    keep_game_log: false,
                    engine_from: Some(2100),
                    ..Default::default()
                };
                let first = start_year - opts.history_years as i32;
                World::new_recorded(opts, None, None, |w, _| {
                    if w.year > first {
                        save(w, &part);
                    }
                    eprintln!("year {} at {:.0}s", w.year, t.elapsed().as_secs_f64());
                    true
                })
                .expect("never stopped")
            };
            eprintln!("built in {:.0}s, {:?}", t.elapsed().as_secs_f64(), w.today);
            save(&w, out);
        }
        Some("play") => {
            let mut w = afm_engine::save::read_world(Path::new(&args[2])).expect("read the world");
            w.options.engine_from = Some(2100);
            let until: i32 = args[3].parse().expect("a year");
            eprintln!("read, {:?}", w.today);
            let mut classes_done = std::collections::HashSet::new();
            while w.today.year() < until || w.today.month() < 8 {
                w.advance_day();
                if w.today.month() == 7 && w.today.day() == 2 {
                    payday(&w);
                }
                // The class signed: its offers, once March comes.
                if w.today.month() == 3 && w.today.day() == 1 && classes_done.insert(w.today.year()) {
                    offers(&w, w.today.year());
                }
            }
        }
        Some("portal") => {
            let mut w = afm_engine::save::read_world(Path::new(&args[2])).expect("read the world");
            w.options.engine_from = Some(2100);
            let until: i32 = args[3].parse().expect("a year");
            let month: u32 = args[4].parse().expect("a month");
            eprintln!("read, {:?}", w.today);
            // The window before the save: transfers still paid on the place
            // they landed in (`nil_role` holds until fall camp).
            let before: Vec<Landing> = w
                .people
                .iter()
                .filter(|p| p.stage == Stage::College && p.cal.nil_role.is_some() && p.path.len() >= 2)
                .filter_map(|p| {
                    let (j, c) = (p.college?, p.path[p.path.len() - 2]);
                    let r = p.cal.nil_role?;
                    Some(Landing {
                        id: p.player.id,
                        from: w.programs[c].division.name(),
                        to: group(&w, j).unwrap_or("below"),
                        pos: p.player.position.name(),
                        role: r.0.name(),
                        edge: r.1,
                        stars: p.stars,
                        money: (p.nil_income - p.brand as f64).max(0.0),
                        name: format!("{} ({} to {})", p.name, w.program_name(c), w.program_name(j)),
                        budget: w.programs[j].collective.budget,
                    })
                })
                .collect();
            println!("## landed before the save ({:?})", w.today);
            transfers(&before);
            let mut seen: std::collections::HashMap<u32, (u32, Option<usize>)> = w
                .people
                .iter()
                .filter(|p| p.stage == Stage::College)
                .map(|p| (p.player.id, (p.transfers, p.college)))
                .collect();
            let mut landed: Vec<Landing> = Vec::new();
            while w.today.year() < until || w.today.month() < month {
                w.advance_day();
                if w.today.month() == 7 && w.today.day() == 2 {
                    payday(&w);
                    held(&w);
                    starters(&w);
                    july(&w, &before);
                }
                for p in w.people.iter().filter(|p| p.stage == Stage::College) {
                    let before = seen.get(&p.player.id).copied();
                    if let Some((t, Some(c))) = before
                        && p.transfers > t
                        && let Some(j) = p.college
                    {
                        let (role, edge) = p.cal.nil_role.map_or(("none", 0.0), |r| (r.0.name(), r.1));
                        landed.push(Landing {
                            id: p.player.id,
                            from: w.programs[c].division.name(),
                            to: group(&w, j).unwrap_or("below"),
                            pos: p.player.position.name(),
                            role,
                            edge,
                            stars: p.stars,
                            money: (p.nil_income - p.brand as f64).max(0.0),
                            name: format!("{} ({} to {})", p.name, w.program_name(c), w.program_name(j)),
                            budget: w.programs[j].collective.budget,
                        });
                    }
                    seen.insert(p.player.id, (p.transfers, p.college));
                }
            }
            println!("## landed after the save, to {:?}", w.today);
            transfers(&landed);
        }
        _ => eprintln!("usage: nil_check build <seed> <years> <scale> <start year> <out> | play <file> <until year>"),
    }
}

struct Landing {
    id: u32,
    from: &'static str,
    to: &'static str,
    pos: &'static str,
    role: &'static str,
    edge: f32,
    stars: u8,
    money: f64,
    name: String,
    budget: f64,
}

/// Count, median, 90th percentile and top of a list of dollars.
fn spread(v: &mut [f64]) -> String {
    v.sort_by(|a, b| a.total_cmp(b));
    if v.is_empty() {
        return "none".into();
    }
    let at = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
    format!("{} median {} p90 {} max {}", v.len(), m(at(0.5)), m(at(0.9)), m(at(1.0)))
}

fn transfers(landed: &[Landing]) {
    println!("== transfers' NIL the day they land: {} ==", landed.len());
    for to in ["P4", "G5", "FCS"] {
        for role in ["starter", "rotation", "backup"] {
            println!("-- to {to}, {role} --");
            for pos in ["QB", "RB", "WR", "TE", "OT", "IOL", "EDGE", "DT", "LB", "CB", "S"] {
                let mut all: Vec<f64> =
                    landed.iter().filter(|l| l.to == to && l.role == role && l.pos == pos).map(|l| l.money).collect();
                let mut up: Vec<f64> = landed
                    .iter()
                    .filter(|l| l.to == to && l.role == role && l.pos == pos && l.from != "FBS")
                    .map(|l| l.money)
                    .collect();
                if !all.is_empty() {
                    println!("  {pos}: all {}; from below FBS {}", spread(&mut all), spread(&mut up));
                }
            }
        }
    }
    let mut top: Vec<&Landing> = landed.iter().collect();
    top.sort_by(|a, b| b.money.total_cmp(&a.money));
    println!("-- the 25 biggest --");
    for l in top.iter().take(25) {
        println!(
            "  {} {} {}★ {} at {} edge {:+.1} budget {}: {}",
            m(l.money),
            l.pos,
            l.stars,
            l.role,
            l.to,
            l.edge,
            m(l.budget),
            l.name
        );
    }
    println!("-- the 15 biggest from below FBS --");
    for l in top.iter().filter(|l| l.from != "FBS").take(15) {
        println!(
            "  {} {} {}★ {} at {} (from {}) edge {:+.1} budget {}: {}",
            m(l.money),
            l.pos,
            l.stars,
            l.role,
            l.to,
            l.from,
            l.edge,
            m(l.budget),
            l.name
        );
    }
}

/// The collectives' July money NIL Go held back (B18; real, May-June 2026:
/// 23% of the money failed review): the payroll the split meant, less what
/// was paid, over the collectives' part of it.
fn held(w: &World) {
    let today = w.today;
    for g in ["P4", "G5", "FCS"] {
        let (mut meant, mut paid, mut coll) = (0.0, 0.0, 0.0);
        for j in (0..w.programs.len()).filter(|&j| group(w, j) == Some(g)) {
            let prog = &w.programs[j];
            let b = prog.collective.budget;
            let school = afm_engine::world::ncaa::school_pot(prog, today);
            meant += 0.6 * b;
            coll += 0.6 * (b - school).max(0.0);
            paid += w
                .people
                .iter()
                .filter(|p| p.stage == Stage::College && p.college == Some(j))
                .map(|p| (p.nil_income - p.brand as f64).max(0.0))
                .sum::<f64>();
        }
        println!(
            "{g}: NIL Go held {} of the collectives' {} ({:.0}%)",
            m(meant - paid),
            m(coll),
            100.0 * (meant - paid) / coll.max(1.0)
        );
    }
}

/// What the window's transfers make once July 1 splits the money again.
fn july(w: &World, landed: &[Landing]) {
    let by: std::collections::HashMap<u32, usize> =
        w.people.iter().enumerate().map(|(k, p)| (p.player.id, k)).collect();
    println!("== the window's transfers on July {} ==", w.today.year());
    for to in ["P4", "G5", "FCS"] {
        let mut land: Vec<f64> = Vec::new();
        let mut now: Vec<f64> = Vec::new();
        for l in landed.iter().filter(|l| l.to == to && l.role == "starter") {
            if let Some(&k) = by.get(&l.id) {
                let p = &w.people[k];
                if p.stage == Stage::College {
                    land.push(l.money);
                    now.push((p.nil_income - p.brand as f64).max(0.0));
                }
            }
        }
        println!("  {to} starters: landing {}; July {}", spread(&mut land), spread(&mut now));
    }
}

/// July's payday by position against the College Sports Commission's
/// 2025-26 figures (paid players only, revenue share plus NIL): every
/// paid player by position group, and the starters with a college season
/// (what a transfer's deal is read against).
fn starters(w: &World) {
    use afm_engine::positions::Position as P;
    let groups: [(&str, &[P]); 9] = [
        ("QB", &[P::QB]),
        ("RB", &[P::RB]),
        ("WR", &[P::WR]),
        ("TE", &[P::TE]),
        ("OL", &[P::OT, P::IOL]),
        ("DL", &[P::EDGE, P::DT]),
        ("LB", &[P::LB]),
        ("DB", &[P::CB, P::S]),
        ("ST", &[P::K, P::P, P::LS]),
    ];
    let money = |p: &afm_engine::world::Person| (p.nil_income - p.brand as f64).max(0.0);
    let played = |p: &afm_engine::world::Person| {
        p.player
            .history
            .iter()
            .rev()
            .find(|h| h.playing_time != afm_engine::player::PlayingTime::Unsigned)
            .is_some_and(|h| h.level.is_college())
    };
    println!("== July {} payday ==", w.today.year());
    for g in ["P4", "G5", "FCS"] {
        println!("-- {g}: every paid player (over $0) --");
        for (name, ps) in groups {
            let mut v: Vec<f64> = w
                .people
                .iter()
                .filter(|p| {
                    p.stage == Stage::College
                        && ps.contains(&p.player.position)
                        && p.college.is_some_and(|j| group(w, j) == Some(g))
                })
                .map(money)
                .filter(|&x| x > 0.0)
                .collect();
            let n = v.len();
            let under = v.iter().filter(|&&x| x < 10_000.0).count();
            println!("  {name}: {} ({:.0}% under $10K)", spread(&mut v), 100.0 * under as f64 / n.max(1) as f64);
        }
        // What spreads a starter's pay: his recruiting stars, his play
        // against the level's starters and his program's price per unit
        // of payroll weight (`nil_weight`'s parts, rebuilt here).
        println!("-- {g}: starters' pay parts (p10 / p25 / median / p75 / p90) --");
        let mut medians = std::collections::HashMap::new();
        for p in w.people.iter().filter(|p| {
            p.stage == Stage::College
                && p.last_role == Some(afm_engine::player::PlayingTime::Starter)
                && played(p)
                && p.college.is_some_and(|j| group(w, j) == Some(g))
        }) {
            if let Some(x) = p.last_performance {
                medians.entry(p.player.position).or_insert_with(Vec::new).push(x);
            }
        }
        for v in medians.values_mut() {
            v.sort_by(|a, b| a.total_cmp(b));
        }
        let q = |v: &mut Vec<f64>| {
            v.sort_by(|a, b| a.total_cmp(b));
            if v.is_empty() {
                return "none".to_string();
            }
            let at = |x: f64| v[((v.len() - 1) as f64 * x).round() as usize];
            format!("{:.2} / {:.2} / {:.2} / {:.2} / {:.2}", at(0.1), at(0.25), at(0.5), at(0.75), at(0.9))
        };
        for pos in [P::QB, P::OT, P::IOL, P::EDGE, P::DT, P::WR, P::CB] {
            let mid = medians.get(&pos).map_or(0.0, |v| v[v.len() / 2]);
            let (mut star, mut play, mut stars) = (Vec::new(), Vec::new(), Vec::new());
            for p in w.people.iter().filter(|p| {
                p.stage == Stage::College
                    && p.player.position == pos
                    && p.last_role == Some(afm_engine::player::PlayingTime::Starter)
                    && played(p)
                    && p.cal.nil_role.is_none()
                    && p.college.is_some_and(|j| group(w, j) == Some(g))
            }) {
                let f: f64 = match p.stars {
                    5 => 2.5,
                    4 => 1.0,
                    3 => 0.3,
                    2 => 0.08,
                    _ => 0.03,
                };
                star.push(f.sqrt());
                stars.push(p.stars as f64);
                let edge = p.last_performance.map_or(0.0, |x| x - mid).clamp(-15.0, 15.0);
                play.push(edge);
            }
            println!(
                "  {}: stars {}; star part {}; play edge {}",
                pos.name(),
                q(&mut stars),
                q(&mut star),
                q(&mut play)
            );
        }
        println!("-- {g}: starters with a college season --");
        for pos in P::ALL {
            let mut v: Vec<f64> = w
                .people
                .iter()
                .filter(|p| {
                    p.stage == Stage::College
                        && p.player.position == pos
                        && p.last_role == Some(afm_engine::player::PlayingTime::Starter)
                        && played(p)
                        && p.college.is_some_and(|j| group(w, j) == Some(g))
                })
                .map(money)
                .collect();
            v.sort_by(|a, b| a.total_cmp(b));
            let p25 = if v.is_empty() { 0.0 } else { v[(v.len() - 1) / 4] };
            println!("  {}: p25 {} {}", pos.name(), m(p25), spread(&mut v));
        }
    }
}

fn group(w: &World, j: usize) -> Option<&'static str> {
    let p = &w.programs[j];
    match p.division {
        Division::FBS if p.power_four() => Some("P4"),
        Division::FBS => Some("G5"),
        Division::FCS => Some("FCS"),
        _ => None,
    }
}

fn m(x: f64) -> String {
    if x >= 1e6 { format!("${:.2}M", x / 1e6) } else { format!("${:.0}K", x / 1e3) }
}

fn payday(w: &World) {
    let year = w.today.year();
    println!("== payday {year} (July 1) ==");
    for g in ["P4", "G5", "FCS"] {
        let progs: Vec<usize> = (0..w.programs.len()).filter(|&j| group(w, j) == Some(g)).collect();
        let mut totals: Vec<(f64, usize)> = Vec::new();
        let mut deals: Vec<(f64, usize)> = Vec::new();
        for &j in &progs {
            let mut t = 0.0;
            for p in w.people.iter().enumerate().filter(|(_, p)| p.stage == Stage::College && p.college == Some(j)) {
                let x = (p.1.nil_income - p.1.brand as f64).max(0.0);
                t += x;
                deals.push((x, p.0));
            }
            totals.push((t, j));
        }
        totals.sort_by(|a, b| b.0.total_cmp(&a.0));
        deals.sort_by(|a, b| b.0.total_cmp(&a.0));
        let budget: f64 = progs.iter().map(|&j| w.programs[j].collective.budget).sum();
        let school: f64 = progs.iter().map(|&j| afm_engine::world::ncaa::revenue_share(&w.programs[j], year)).sum();
        let paid: f64 = totals.iter().map(|t| t.0).sum();
        let n = deals.len().max(1) as f64;
        let under = deals.iter().filter(|d| d.0 < 10_000.0).count() as f64 / n;
        let million = deals.iter().filter(|d| d.0 >= 1.0e6).count();
        let half = deals.iter().filter(|d| d.0 >= 0.5e6).count();
        let med = totals.get(totals.len() / 2).map_or(0.0, |t| t.0);
        println!(
            "{g}: {} programs, budgets {} (revenue share {}), paid {} ({:.0}%), program median {}, players {} ({:.1}% under $10K, {half} at $500K+, {million} at $1M+)",
            progs.len(),
            m(budget),
            m(school),
            m(paid),
            100.0 * paid / budget.max(1.0),
            m(med),
            deals.len(),
            100.0 * under
        );
        let tops: Vec<String> = totals.iter().take(3).map(|t| format!("{} {}", w.program_name(t.1), m(t.0))).collect();
        println!("  top programs: {}", tops.join(", "));
        let top: Vec<String> = deals
            .iter()
            .take(5)
            .map(|d| {
                let p = &w.people[d.1];
                format!(
                    "{} {}★ {} {}",
                    p.player.position.name(),
                    p.stars,
                    w.program_name(p.college.unwrap_or(0)),
                    m(d.0)
                )
            })
            .collect();
        println!("  top deals: {}", top.join("; "));
    }
}

fn offers(w: &World, year: i32) {
    println!("== signed NIL offers, class of {year} ==");
    for g in ["P4", "G5", "FCS"] {
        let mut by: [Vec<f64>; 6] = Default::default();
        for j in (0..w.programs.len()).filter(|&j| group(w, j) == Some(g)) {
            // Kept by the season before: the class of 2026 signs in the 2025 season.
            for o in w.programs[j].era.offers.iter().filter(|o| o.0 == year - 1) {
                by[o.2.min(5) as usize].push(o.3);
            }
        }
        let line: Vec<String> = (2..=5)
            .rev()
            .map(|s| {
                let v = &mut by[s];
                v.sort_by(|a, b| a.total_cmp(b));
                let mean = v.iter().sum::<f64>() / (v.len().max(1) as f64);
                format!("{s}★ {} mean {} max {}", v.len(), m(mean), m(v.last().copied().unwrap_or(0.0)))
            })
            .collect();
        println!("{g}: {}", line.join("; "));
        // A class's mean signed offer at the ten richest programs of the
        // group (real 2026, top Power 4: about $85-253K a recruit, On3's
        // valuations via SI), and its quarterbacks (real P4 QB signees
        // $100-300K, five-star QBs $750K-1M).
        let mut rich: Vec<usize> = (0..w.programs.len()).filter(|&j| group(w, j) == Some(g)).collect();
        rich.sort_by(|&a, &b| w.programs[b].collective.budget.total_cmp(&w.programs[a].collective.budget));
        let class = |j: usize| -> Vec<(Position, u8, f64)> {
            w.programs[j].era.offers.iter().filter(|o| o.0 == year - 1).map(|o| (o.1, o.2, o.3)).collect()
        };
        let tops: Vec<String> = rich
            .iter()
            .take(10)
            .map(|&j| {
                let c = class(j);
                let mean = c.iter().map(|x| x.2).sum::<f64>() / c.len().max(1) as f64;
                format!("{} {} ({})", w.program_name(j), m(mean), c.len())
            })
            .collect();
        println!("  class means, ten richest: {}", tops.join(", "));
        let qbs: Vec<(u8, f64)> =
            rich.iter().flat_map(|&j| class(j)).filter(|x| x.0 == Position::QB).map(|x| (x.1, x.2)).collect();
        let mean = |v: Vec<f64>| v.iter().sum::<f64>() / v.len().max(1) as f64;
        let five: Vec<f64> = qbs.iter().filter(|x| x.0 >= 5).map(|x| x.1).collect();
        let rest: Vec<f64> = qbs.iter().filter(|x| x.0 < 5).map(|x| x.1).collect();
        println!(
            "  quarterbacks: five-stars {} mean {}, others {} mean {}",
            five.len(),
            m(mean(five.clone())),
            rest.len(),
            m(mean(rest.clone()))
        );
    }
}
