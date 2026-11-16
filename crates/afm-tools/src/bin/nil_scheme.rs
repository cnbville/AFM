//! The new NIL scheme's fit (thread "Build the new NIL scheme", lane A):
//! reads a saved world and prints every figure the scheme is fitted to, by
//! conference and level, against the real 2026 anchors
//! (`/mnt/project-files/afm/nil-scheme/spec.md`).
//! Usage: nil_scheme pots <world file>: the money pots by conference now.
//!        nil_scheme run <world file> <until year> <until month>: plays the
//!        world on and prints each July's payday, December's keep-him
//!        offers, the window's transfers, each class's signed offers and
//!        NIL Go's denials.

use afm_engine::player::PlayingTime;
use afm_engine::positions::Position;
use afm_engine::world::data::Division;
use afm_engine::world::{Person, Stage, World};
use std::collections::HashMap;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("pots") => {
            let w = afm_engine::save::read_world(Path::new(&args[2])).expect("read the world");
            eprintln!("read, {:?}", w.today);
            pots(&w);
        }
        Some("run") => {
            let mut w = afm_engine::save::read_world(Path::new(&args[2])).expect("read the world");
            w.options.engine_from = Some(2100);
            let until: i32 = args[3].parse().expect("a year");
            let month: u32 = args[4].parse().expect("a month");
            eprintln!("read, {:?}", w.today);
            pots(&w);
            let mut seen: HashMap<u32, (u32, f64)> = w
                .people
                .iter()
                .filter(|p| p.stage == Stage::College)
                .map(|p| (p.player.id, (p.transfers, p.nil_income)))
                .collect();
            let mut landed: Vec<Landed> = Vec::new();
            let mut classes = std::collections::HashSet::new();
            while w.today.year() < until || w.today.month() < month {
                w.advance_day();
                let (m, d) = (w.today.month(), w.today.day());
                if m == 7 && d == 2 {
                    pots(&w);
                    payday(&w);
                    denials(&w);
                }
                // Inside the January window (January 4-13, 2027): the
                // keep-him offers come the day it opens.
                if m == 1 && d == 12 {
                    keeps(&w);
                }
                if m == 3 && d == 1 && classes.insert(w.today.year()) {
                    offers(&w, w.today.year());
                    transfers(&landed);
                    denials(&w);
                }
                for p in w.people.iter().filter(|p| p.stage == Stage::College) {
                    let e = seen.entry(p.player.id).or_insert((p.transfers, p.nil_income));
                    if p.transfers > e.0 {
                        let deal = p.cal.nil_deal.filter(|d| d.kind == afm_engine::world::nil_bid::DealKind::Transfer);
                        let amount = deal.map_or((p.nil_income - p.brand as f64).max(0.0), |d| d.amount);
                        let j = p.college.unwrap_or(0);
                        let g = level(&w, j);
                        let role = p.cal.nil_role.map(|r| r.0);
                        let what = format!(
                            "{} {}★ to {} ({:?}, budget {})",
                            p.player.position.name(),
                            p.stars,
                            w.program_name(j),
                            w.programs[j].division,
                            crate::m(w.programs[j].collective.budget)
                        );
                        landed.push(Landed { amount, g, role, what });
                    }
                    *e = (p.transfers, p.nil_income);
                }
            }
        }
        _ => eprintln!("usage: nil_scheme pots <world> | run <world> <until year> <until month>"),
    }
}

/// The groups the anchors name: each P4 conference, the Group of 5, FCS.
fn conf(w: &World, j: usize) -> Option<&'static str> {
    let p = &w.programs[j];
    match p.division {
        Division::FBS if p.real_name == "Notre Dame" => Some("ND"),
        Division::FBS if p.power_four() => match p.conference.as_str() {
            "SEC" => Some("SEC"),
            "Big Ten" => Some("B1G"),
            "ACC" => Some("ACC"),
            _ => Some("B12"),
        },
        Division::FBS => Some("G5"),
        Division::FCS => Some("FCS"),
        _ => None,
    }
}

fn level(w: &World, j: usize) -> Option<&'static str> {
    match conf(w, j)? {
        "G5" => Some("G5"),
        "FCS" => Some("FCS"),
        _ => Some("P4"),
    }
}

fn m(x: f64) -> String {
    if x.abs() >= 1e6 { format!("${:.2}M", x / 1e6) } else { format!("${:.0}K", x / 1e3) }
}

fn q(v: &mut [f64], at: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f64 * at).round() as usize]
}

fn spread(v: &mut [f64]) -> String {
    if v.is_empty() {
        return "none".into();
    }
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    format!(
        "n {} p10 {} median {} mean {} p90 {} max {}",
        v.len(),
        m(q(v, 0.1)),
        m(q(v, 0.5)),
        m(mean),
        m(q(v, 0.9)),
        m(q(v, 1.0))
    )
}

const CONFS: [&str; 7] = ["SEC", "B1G", "ACC", "B12", "ND", "G5", "FCS"];

/// Each group's pots: the third-party pot (budget less the school's share),
/// the school's revenue share and the whole budget.
fn pots(w: &World) {
    println!("== money pots on {:?} ==", w.today);
    for c in CONFS {
        let js: Vec<usize> = (0..w.programs.len()).filter(|&j| conf(w, j) == Some(c)).collect();
        let mut third: Vec<f64> =
            js.iter().map(|&j| (w.programs[j].collective.budget - w.programs[j].collective.school).max(0.0)).collect();
        let mut school: Vec<f64> = js.iter().map(|&j| w.programs[j].collective.school).collect();
        let mut all: Vec<f64> = js.iter().map(|&j| w.programs[j].collective.budget).collect();
        let mut fans: Vec<f64> = js.iter().map(|&j| w.programs[j].fans).collect();
        println!("{c} ({}): third-party {}", js.len(), spread(&mut third));
        println!("   school {}", spread(&mut school));
        println!("   budget {}", spread(&mut all));
        println!("   fans p10 {:.2} median {:.2} p90 {:.2}", q(&mut fans, 0.1), q(&mut fans, 0.5), q(&mut fans, 0.9));
    }
    let mut rich: Vec<usize> = (0..w.programs.len()).filter(|&j| level(w, j) == Some("P4")).collect();
    rich.sort_by(|&a, &b| w.programs[b].collective.budget.total_cmp(&w.programs[a].collective.budget));
    let top: Vec<String> = rich
        .iter()
        .take(10)
        .map(|&j| format!("{} {}", w.program_name(j), m(w.programs[j].collective.budget)))
        .collect();
    println!("  richest: {}", top.join(", "));
    let low: Vec<String> = rich
        .iter()
        .rev()
        .take(5)
        .map(|&j| format!("{} {}", w.program_name(j), m(w.programs[j].collective.budget)))
        .collect();
    println!("  poorest P4: {}", low.join(", "));
}

fn money(p: &Person) -> f64 {
    (p.nil_income - p.brand as f64).max(0.0)
}

fn played(p: &Person) -> bool {
    p.player
        .history
        .iter()
        .rev()
        .find(|h| h.playing_time != PlayingTime::Unsigned)
        .is_some_and(|h| h.level.is_college())
}

/// July's payday: rosters by conference, the share of each roster paid,
/// paid medians by position group (the CSC's), starters' medians (The NIL
/// Standard's), the biggest deals and the budget's split by role.
fn payday(w: &World) {
    use Position as P;
    println!("== July {} payday ==", w.today.year());
    for c in CONFS {
        let js: Vec<usize> = (0..w.programs.len()).filter(|&j| conf(w, j) == Some(c)).collect();
        let mut rosters: Vec<f64> =
            js.iter().map(|&j| w.programs[j].roster.iter().map(|&i| money(&w.people[i])).sum::<f64>()).collect();
        let (n, paid) = js.iter().fold((0usize, 0usize), |a, &j| {
            let r = &w.programs[j].roster;
            (a.0 + r.len(), a.1 + r.iter().filter(|&&i| money(&w.people[i]) > 0.0).count())
        });
        println!(
            "{c}: rosters {}; {:.0}% of players paid",
            spread(&mut rosters),
            100.0 * paid as f64 / n.max(1) as f64
        );
    }
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
    for g in ["P4", "G5", "FCS"] {
        let on = |p: &Person| p.stage == Stage::College && p.college.is_some_and(|j| level(w, j) == Some(g));
        println!(
            "-- {g}: paid players' medians (real P4 revenue share: QB 230K, DL 150K, OL 133K, DB 115K, WR/LB 112.5K, RB 108.5K, TE 93K; non-P4 QB 31K, others 14-18K) --"
        );
        let mut line = Vec::new();
        for (name, ps) in groups {
            let mut v: Vec<f64> = w
                .people
                .iter()
                .filter(|p| on(p) && ps.contains(&p.player.position))
                .map(money)
                .filter(|&x| x > 0.0)
                .collect();
            line.push(format!("{name} {}", m(q(&mut v, 0.5))));
        }
        println!("  {}", line.join(", "));
        // The CSC's own measure: revenue share alone, among the players
        // the school pays at all.
        let (mut n, mut got) = (0usize, 0usize);
        let mut line = Vec::new();
        for (name, ps) in groups {
            let mut v: Vec<f64> = w
                .people
                .iter()
                .filter(|p| on(p) && ps.contains(&p.player.position))
                .inspect(|p| {
                    n += 1;
                    got += (p.cal.nil_school > 0.0) as usize;
                })
                .map(|p| p.cal.nil_school as f64)
                .filter(|&x| x > 0.0)
                .collect();
            line.push(format!("{name} {}", m(q(&mut v, 0.5))));
        }
        println!("  revenue share alone ({:.0}% paid any): {}", 100.0 * got as f64 / n.max(1) as f64, line.join(", "));
        if g == "P4" {
            share_grid(w, &groups);
        }
        println!(
            "-- {g}: starters with a college season, brand included (real P4: QB 2M, EDGE 929K, OT 907K, WR 726K, CB 572K, RB 527K, IOL 525K) --"
        );
        let mut line = Vec::new();
        for pos in P::ALL {
            let mut v: Vec<f64> = w
                .people
                .iter()
                .filter(|p| on(p) && p.player.position == pos && p.last_role == Some(PlayingTime::Starter) && played(p))
                .map(|p| p.nil_income.max(0.0))
                .collect();
            line.push(format!("{} {} (p90 {})", pos.name(), m(q(&mut v, 0.5)), m(q(&mut v, 0.9))));
        }
        println!("  {}", line.join(", "));
        // The budget's split by role (the scheme's check: starters ~62%,
        // rotation ~11%, recruits and backups ~27%).
        let (mut st, mut rot, mut rest) = (0.0, 0.0, 0.0);
        for p in w.people.iter().filter(|p| on(p)) {
            let x = money(p);
            match (played(p), p.last_role) {
                (true, Some(PlayingTime::Starter)) => st += x,
                (true, Some(PlayingTime::Rotation)) => rot += x,
                _ => rest += x,
            }
        }
        let t = (st + rot + rest).max(1.0);
        println!(
            "  split: starters {:.0}%, rotation {:.0}%, recruits and backups {:.0}%",
            100.0 * st / t,
            100.0 * rot / t,
            100.0 * rest / t
        );
        // A freshman backup (real ~$25K at P4).
        let mut fr: Vec<f64> = w.people.iter().filter(|p| on(p) && !played(p)).map(money).collect();
        println!("  newcomers {}", spread(&mut fr));
    }
    let mut top: Vec<(f64, usize)> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.stage == Stage::College)
        .map(|(i, p)| (p.nil_income, i))
        .collect();
    top.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("-- the 15 biggest (brand included; real stars ~$4-7.5M) --");
    for &(x, i) in top.iter().take(15) {
        let p = &w.people[i];
        let j = p.college.unwrap_or(0);
        println!(
            "  {} (brand {}) {} {}★ {:?} perf {:.1} at {} (budget {})",
            m(x),
            m(p.brand as f64),
            p.player.position.name(),
            p.stars,
            p.last_role,
            p.last_performance.unwrap_or(0.0),
            w.program_name(j),
            m(w.programs[j].collective.budget)
        );
    }
}

/// The school's revenue-share role weights tried on today's P4 payday
/// (rotation, backup or newcomer against a starter): the paid players'
/// revenue-share medians and the starters' whole pay each would give.
fn share_grid(w: &World, groups: &[(&str, &[Position]); 9]) {
    use Position as P;
    let at = |pos: P| match pos {
        P::QB => 2.04,
        P::EDGE | P::DT => 1.33,
        P::OT | P::IOL => 1.18,
        P::RB => 0.96,
        P::TE => 0.83,
        P::WR | P::LB | P::CB | P::S => 1.0,
        P::K | P::P | P::LS => 0.4,
    };
    let role = |p: &Person| -> u8 {
        if !played(p) {
            return 2;
        }
        match p.cal.nil_role.map(|r| r.0).or(p.last_role) {
            Some(PlayingTime::Starter) => 0,
            Some(PlayingTime::Rotation) => 1,
            _ => 2,
        }
    };
    for (rot, dep) in [(0.6, 0.4), (0.45, 0.25), (0.35, 0.15), (0.3, 0.1), (0.25, 0.07)] {
        let mut share: HashMap<usize, f64> = HashMap::new();
        for j in (0..w.programs.len()).filter(|&j| level(w, j) == Some("P4")) {
            let r = &w.programs[j].roster;
            let pot: f64 = r.iter().map(|&i| w.people[i].cal.nil_school as f64).sum();
            let wt = |i: usize| {
                let p = &w.people[i];
                if p.cal.nil_school <= 0.0 {
                    return 0.0;
                }
                at(p.player.position) * [1.0, rot, dep][role(p) as usize]
            };
            let tot: f64 = r.iter().map(|&i| wt(i)).sum();
            for &i in r {
                share.insert(i, if tot > 0.0 { pot * wt(i) / tot } else { 0.0 });
            }
        }
        let mut line = Vec::new();
        for (name, ps) in groups.iter() {
            let mut v: Vec<f64> = share
                .iter()
                .filter(|(i, x)| **x > 0.0 && ps.contains(&w.people[**i].player.position))
                .map(|(_, x)| *x)
                .collect();
            line.push(format!("{name} {}", m(q(&mut v, 0.5))));
        }
        let mut st = Vec::new();
        for pos in [P::QB, P::EDGE, P::OT, P::WR, P::CB, P::RB, P::IOL] {
            let mut v: Vec<f64> = share
                .iter()
                .filter(|(i, _)| {
                    let p = &w.people[**i];
                    p.player.position == pos && p.last_role == Some(PlayingTime::Starter) && played(p)
                })
                .map(|(i, x)| {
                    let p = &w.people[*i];
                    p.nil_income.max(0.0) - p.cal.nil_school as f64 + x
                })
                .collect();
            st.push(format!("{} {}", pos.name(), m(q(&mut v, 0.5))));
        }
        println!("  grid rot {rot} dep {dep}: shares {}; starters {}", line.join(", "), st.join(", "));
    }
}

/// NIL Go's year so far: deals reviewed and denied, and the dollars.
fn denials(w: &World) {
    let g = &w.log;
    println!(
        "== NIL Go: {} of {} reviewed deals denied ({:.1}%), {} of {} ({:.0}%); moved to revenue share {} (real May-June 2026: 7.9% of deals, 23% of dollars) ==",
        g.nil_go_denied,
        g.nil_go_reviewed,
        100.0 * g.nil_go_denied as f64 / g.nil_go_reviewed.max(1) as f64,
        m(g.nil_go_denied_money),
        m(g.nil_go_reviewed_money),
        100.0 * g.nil_go_denied_money / g.nil_go_reviewed_money.max(1.0),
        m(g.nil_go_moved)
    );
}

/// December's keep-him offers (item 9), read on January 2.
fn keeps(w: &World) {
    let mut v: Vec<f64> = Vec::new();
    let mut by: HashMap<&'static str, Vec<f64>> = HashMap::new();
    for p in w.people.iter().filter(|p| p.stage == Stage::College) {
        if let Some((k, _)) = p.cal.nil_keep {
            v.push(k);
            if let Some(g) = p.college.and_then(|j| level(w, j)) {
                by.entry(g).or_default().push(k);
            }
        }
    }
    println!("== keep-him offers before the {} window: {} ==", w.today.year(), spread(&mut v));
    for g in ["P4", "G5", "FCS"] {
        println!("  {g}: {}", spread(by.entry(g).or_default()));
    }
}

/// One landing, read the day he landed (people move in the store later).
struct Landed {
    amount: f64,
    g: Option<&'static str>,
    role: Option<PlayingTime>,
    what: String,
}

/// The window's transfers: their landing deals by level and role.
fn transfers(landed: &[Landed]) {
    println!("== transfers' landing deals: {} ==", landed.len());
    for g in ["P4", "G5", "FCS"] {
        for role in [PlayingTime::Starter, PlayingTime::Rotation] {
            let mut v: Vec<f64> =
                landed.iter().filter(|x| x.g == Some(g) && x.role == Some(role)).map(|x| x.amount).collect();
            println!("  {g} {}: {}", role.name(), spread(&mut v));
        }
        let mut v: Vec<f64> = landed.iter().filter(|x| x.g == Some(g)).map(|x| x.amount).collect();
        println!("  {g} all: {}", spread(&mut v));
    }
    let mut top: Vec<&Landed> = landed.iter().collect();
    top.sort_by(|a, b| b.amount.total_cmp(&a.amount));
    for x in top.iter().take(10) {
        println!("  {} {}", m(x.amount), x.what);
    }
}

/// Each class's signed offers: by stars, the top classes, the No. 1 QB and
/// four-star receivers at the top programs.
fn offers(w: &World, year: i32) {
    println!(
        "== signed NIL offers, class of {year} (real: 5★ $750K-1M, No. 1 QB at a $50M school ~$2.4M, 4★ WR $100-250K at top programs, top class $6-9M) =="
    );
    for g in ["P4", "G5", "FCS"] {
        let mut by: [Vec<f64>; 6] = Default::default();
        for j in (0..w.programs.len()).filter(|&j| level(w, j) == Some(g)) {
            for o in w.programs[j].era.offers.iter().filter(|o| o.0 == year - 1) {
                by[o.2.min(5) as usize].push(o.3);
            }
        }
        let line: Vec<String> = (2..=5).rev().map(|s| format!("{s}★ {}", spread(&mut by[s]))).collect();
        println!("{g}: {}", line.join("\n    "));
    }
    let mut rich: Vec<usize> = (0..w.programs.len()).filter(|&j| level(w, j) == Some("P4")).collect();
    rich.sort_by(|&a, &b| w.programs[b].collective.budget.total_cmp(&w.programs[a].collective.budget));
    let class = |j: usize| -> Vec<(Position, u8, f64)> {
        w.programs[j].era.offers.iter().filter(|o| o.0 == year - 1).map(|o| (o.1, o.2, o.3)).collect()
    };
    let mut totals: Vec<(f64, usize)> = rich.iter().map(|&j| (class(j).iter().map(|x| x.2).sum(), j)).collect();
    totals.sort_by(|a, b| b.0.total_cmp(&a.0));
    let t: Vec<String> = totals
        .iter()
        .take(8)
        .map(|&(x, j)| {
            format!(
                "{} {} ({} signed, mean {})",
                w.program_name(j),
                m(x),
                class(j).len(),
                m(x / class(j).len().max(1) as f64)
            )
        })
        .collect();
    println!("  top classes: {}", t.join("; "));
    let mut wr4: Vec<f64> =
        rich.iter().take(15).flat_map(|&j| class(j)).filter(|x| x.0 == Position::WR && x.1 == 4).map(|x| x.2).collect();
    println!("  4★ WRs at the 15 richest: {}", spread(&mut wr4));
    let mut qbs: Vec<(f64, usize)> = Vec::new();
    for &j in &rich {
        for x in class(j).iter().filter(|x| x.0 == Position::QB && x.1 == 5) {
            qbs.push((x.2, j));
        }
    }
    qbs.sort_by(|a, b| b.0.total_cmp(&a.0));
    let t: Vec<String> = qbs
        .iter()
        .take(5)
        .map(|&(x, j)| format!("{} at {} ({})", m(x), w.program_name(j), m(w.programs[j].collective.budget)))
        .collect();
    println!("  five-star QBs: {}", t.join("; "));
}
