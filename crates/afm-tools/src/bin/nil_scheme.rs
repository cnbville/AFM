//! The new NIL scheme's fit (thread "Build the new NIL scheme", lane A):
//! reads a saved world and prints every figure the scheme is fitted to, by
//! conference and level, against the real 2026 anchors
//! (`/mnt/project-files/afm/nil-scheme/spec.md`).
//! Usage: nil_scheme pots <world file>: the money pots by conference now.
//!        nil_scheme run <world file> <until year> <until month>: plays the
//!        world on and prints each July's payday, December's keep-him
//!        offers, the window's transfers, each class's signed offers and
//!        NIL Go's denials.
//!        nil_scheme market <world file> <until year>: plays the world on
//!        to July 2 of that year and, at each July's payday, compares
//!        every AI college player's pay with his market price the day
//!        before (thread "Pay inherited players their market price") and
//!        with the roster split your desk would show at his school.
//!        Both modes play every game on the match engine, as the game does;
//!        `quick` as the last argument plays them on the stand-in instead,
//!        which spreads play wider than the engine, so every price read
//!        after the first season runs high (quick checks only).

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
            quick(&mut w, &args);
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
            let mut window: Option<Window> = None;
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
                    signees(&w);
                    transfers(&landed);
                    denials(&w);
                    multi(&w);
                }
                // Multi-year deals and the portal (item 12): who holds one
                // as the winter window opens, and who left by February.
                if m == 1 && d == 1 {
                    window = Some(under_contract(&w));
                }
                if m == 2
                    && d == 1
                    && let Some(win) = window.take()
                {
                    left_on_contract(&w, &win);
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
        Some("market") => {
            let mut w = afm_engine::save::read_world(Path::new(&args[2])).expect("read the world");
            quick(&mut w, &args);
            let until: i32 = args[3].parse().expect("a year");
            eprintln!("read, {:?}", w.today);
            let mut before: HashMap<u32, Priced> = HashMap::new();
            let mut split: HashMap<u32, (usize, f64)> = HashMap::new();
            let mut kinds: HashMap<u32, (Option<afm_engine::world::nil_bid::DealKind>, Option<PlayingTime>)> =
                HashMap::new();
            while w.today.year() < until || w.today.month() < 7 || w.today.day() < 2 {
                w.advance_day();
                let (m, d) = (w.today.month(), w.today.day());
                if m == 6 && d == 30 {
                    before = priced(&w);
                    split = roster_splits(&w);
                    kinds = deal_kinds(&w);
                    eprintln!("{:?}: {} players priced", w.today, before.len());
                }
                if m == 7 && d == 2 && !before.is_empty() {
                    payday(&w);
                    by_deal(&w, &kinds);
                    against_market(&w, &before);
                    scales(&w, &before);
                    against_split(&w, &split);
                }
            }
        }
        _ => eprintln!(
            "usage: nil_scheme pots <world> | run <world> <until year> <until month> | market <world> <until year>"
        ),
    }
}

/// Games on the stand-in only with `quick` as the last argument (`GAPS.md`:
/// the stand-in spreads play wider than the engine).
fn quick(w: &mut World, args: &[String]) {
    if args.last().is_some_and(|a| a == "quick") {
        w.options.engine_from = Some(2100);
        eprintln!("games on the stand-in (quick)");
    }
}

/// The roster split your desk would show at every AI FBS and FCS school
/// the day before July 1 (`front_office::college::splits_for`), for each
/// player without a deal for the year: (school, his split in both pots).
fn roster_splits(w: &World) -> HashMap<u32, (usize, f64)> {
    let year = w.today.year();
    let mut out = HashMap::new();
    for j in (0..w.programs.len()).filter(|&j| level(w, j).is_some() && w.programs[j].collective.budget > 0.0) {
        let at: HashMap<u32, usize> = w.programs[j].roster.iter().map(|&i| (w.people[i].player.id, i)).collect();
        for (id, s, c, _) in afm_engine::world::front_office::college::splits_for(w, j, year) {
            let Some(&i) = at.get(&id) else { continue };
            if w.people[i].cal.nil_deal.is_some_and(|d| d.pays(year)) {
                continue;
            }
            out.insert(id, (j, s + c));
        }
    }
    out
}

/// July's pay against the roster split read the day before, for everyone
/// without a deal still at the same school: the desk's going rate should
/// be what an AI school pays (before 2026-11-18 it read 0.6 of it).
fn against_split(w: &World, split: &HashMap<u32, (usize, f64)>) {
    println!("== July {}: AI pay against the desk's roster split (the day before) ==", w.today.year());
    for g in ["P4", "G5", "FCS"] {
        for (name, want) in [("starters", 0), ("rotation", 1), ("the rest", 2)] {
            let mut v: Vec<f64> = Vec::new();
            for p in w.people.iter().filter(|p| p.stage == Stage::College) {
                let Some(&(j, x)) = split.get(&p.player.id) else { continue };
                if p.college != Some(j) || level(w, j) != Some(g) || x < 1_000.0 {
                    continue;
                }
                let r = match p.last_role {
                    Some(PlayingTime::Starter) => 0,
                    Some(PlayingTime::Rotation) => 1,
                    _ => 2,
                };
                if r == want {
                    v.push(p.cal.nil_paid as f64 / x);
                }
            }
            if v.is_empty() {
                continue;
            }
            let n = v.len();
            println!(
                "  {g} {name}: n {n}  pay/split p10 {:.2} p25 {:.2} median {:.2} p75 {:.2} p90 {:.2}",
                q(&mut v, 0.1),
                q(&mut v, 0.25),
                q(&mut v, 0.5),
                q(&mut v, 0.75),
                q(&mut v, 0.9)
            );
        }
    }
}

/// The class that just signed, every signee (paid or not), by level and
/// stars: the share with a deal, and the deals' spread (real: Group of 5
/// signees are paid five figures when paid at all, Bleacher Report on The
/// Athletic's reporting, 2026).
fn signees(w: &World) {
    use afm_engine::world::nil_bid::DealKind;
    println!("== the class just signed: share with a deal and its spread, by level and stars ==");
    let mut by: HashMap<(&'static str, u8), (usize, Vec<f64>)> = HashMap::new();
    // Paid signees per program (a school pays only recruits its July paid
    // set takes, Cain 2026-11-19).
    let mut per: HashMap<usize, (usize, usize)> = HashMap::new();
    for p in w.people.iter() {
        let j = match (p.cal.signed, p.stage) {
            (Some(s), _) if !s.walk_on => s.program,
            (None, Stage::College) if !p.walk_on && !played(p) && p.college_seasons == 0 => {
                let Some(j) = p.college else { continue };
                j
            }
            _ => continue,
        };
        let Some(g) = level(w, j) else { continue };
        let e = by.entry((g, p.stars.clamp(1, 5))).or_default();
        e.0 += 1;
        let c = per.entry(j).or_default();
        c.0 += 1;
        if let Some(d) = p.cal.nil_deal.filter(|d| d.kind == DealKind::Signed && d.amount > 0.0) {
            e.1.push(d.amount);
            c.1 += 1;
        }
    }
    for g in ["P4", "G5", "FCS"] {
        let mut v: Vec<f64> = per.iter().filter(|(j, _)| level(w, **j) == Some(g)).map(|(_, c)| c.1 as f64).collect();
        let class: usize = per.iter().filter(|(j, _)| level(w, **j) == Some(g)).map(|(_, c)| c.0).sum();
        println!(
            "  {g}: paid signees a program {} (of {:.1} signed)",
            spread_n(&mut v),
            class as f64 / v.len().max(1) as f64
        );
    }
    for g in ["P4", "G5", "FCS"] {
        for stars in (2..=5).rev() {
            let Some((n, v)) = by.get_mut(&(g, stars)) else { continue };
            let k = v.len();
            println!(
                "  {g} {stars}★: {n} signed, {:.0}% with a deal: {}",
                100.0 * k as f64 / (*n).max(1) as f64,
                spread(v)
            );
        }
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

/// [`spread`] for counts.
fn spread_n(v: &mut [f64]) -> String {
    if v.is_empty() {
        return "none".into();
    }
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    format!(
        "n {} p10 {:.0} median {:.0} mean {:.1} p90 {:.0} max {:.0}",
        v.len(),
        q(v, 0.1),
        q(v, 0.5),
        mean,
        q(v, 0.9),
        q(v, 1.0)
    )
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
        let (low, high) = js.iter().fold((0usize, 0usize), |a, &j| {
            let r = &w.programs[j].roster;
            (
                a.0 + r.iter().filter(|&&i| money(&w.people[i]) < 10_000.0).count(),
                a.1 + r.iter().filter(|&&i| money(&w.people[i]) >= 100_000.0).count(),
            )
        });
        println!(
            "{c}: rosters {}; {:.0}% of players paid, {:.0}% under $10K, {:.0}% at $100K or more",
            spread(&mut rosters),
            100.0 * paid as f64 / n.max(1) as f64,
            100.0 * low as f64 / n.max(1) as f64,
            100.0 * high as f64 / n.max(1) as f64
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
        // The spread inside the paid set (thread "Research real NIL split by
        // sport", item 3): the CSC's real tail among the paid is a 10th
        // percentile of $17.5-25K at every position, an offensive line 90th
        // percentile of $512K, a quarterback's 75th of $675K and 90th of $1.611M.
        println!("  revenue share among the paid, p10 / p25 / p50 / p75 / p90:");
        for (name, ps) in groups {
            let mut v: Vec<f64> = w
                .people
                .iter()
                .filter(|p| on(p) && ps.contains(&p.player.position))
                .map(|p| p.cal.nil_school as f64)
                .filter(|&x| x > 0.0)
                .collect();
            println!(
                "    {name} (n {}): {} / {} / {} / {} / {}",
                v.len(),
                m(q(&mut v, 0.1)),
                m(q(&mut v, 0.25)),
                m(q(&mut v, 0.5)),
                m(q(&mut v, 0.75)),
                m(q(&mut v, 0.9))
            );
        }
        // Whole-roster shares, everything he is paid (revenue share, third
        // party, brand): under $10K and $100K or more, against the CSC-based
        // 25-30% and 35-45% for a Power 4 roster.
        {
            let all: Vec<f64> = w.people.iter().filter(|p| on(p)).map(money).collect();
            let k = all.len().max(1) as f64;
            println!(
                "  whole roster: {:.0}% under $10K, {:.0}% paid anything, {:.0}% at $100K or more",
                100.0 * all.iter().filter(|&&x| x < 10_000.0).count() as f64 / k,
                100.0 * all.iter().filter(|&&x| x > 0.0).count() as f64 / k,
                100.0 * all.iter().filter(|&&x| x >= 100_000.0).count() as f64 / k
            );
        }
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
            p.last_performance.get().unwrap_or(0.0),
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

/// Multi-year deals (the new NIL scheme, item 12): the class just signed,
/// the transfers who landed on one, and everyone holding one. Real: about
/// 5% of recruits sign multi-year, mostly blue-chips; most players sign one
/// year at a time (On3).
fn multi(w: &World) {
    use afm_engine::world::nil_bid::DealKind;
    let g = &w.log;
    println!(
        "== multi-year deals (real: ~5% of recruits, mostly blue-chips; most players one year at a time) ==\n  this year's log: {} recruits on one (by stars 5★ {} 4★ {} 3★ {} other {}), {} in all over their terms; {} cut to one year by the bound; {} transfers landed on one; {} buyouts paid, {}",
        g.nil_multi,
        g.nil_multi_stars[5],
        g.nil_multi_stars[4],
        g.nil_multi_stars[3],
        g.nil_multi_stars[0] + g.nil_multi_stars[1] + g.nil_multi_stars[2],
        m(g.nil_multi_money),
        g.nil_multi_bound,
        g.nil_multi_transfers,
        g.nil_buyouts,
        m(g.nil_buyout_money)
    );
    // The class waiting to enroll: signees with a deal, by level and stars.
    for lv in ["P4", "G5", "FCS"] {
        let mut n = [0u32; 6];
        let mut multi = [0u32; 6];
        let mut per: Vec<f64> = Vec::new();
        let mut one: Vec<f64> = Vec::new();
        let mut terms = [0u32; 5];
        for p in w.people.iter().filter(|p| p.stage == Stage::HighSchool) {
            let Some(s) = p.cal.signed.as_ref() else { continue };
            if level(w, s.program) != Some(lv) || s.walk_on {
                continue;
            }
            let k = p.stars.min(5) as usize;
            n[k] += 1;
            match p.cal.nil_deal.filter(|d| d.kind == DealKind::Signed) {
                Some(d) if d.contract() => {
                    multi[k] += 1;
                    per.push(d.amount);
                    terms[d.years_left().clamp(0, 4) as usize] += 1;
                }
                Some(d) if k >= 4 => one.push(d.amount),
                _ => {}
            }
        }
        let all: u32 = n.iter().sum();
        let on: u32 = multi.iter().sum();
        println!(
            "  {lv} signees: {on} of {all} on a multi-year deal ({:.1}%); 5★ {}/{} 4★ {}/{} 3★ {}/{}; terms 2y {} 3y {} 4y {}; a year on one {}; blue-chips on one year {}",
            100.0 * on as f64 / all.max(1) as f64,
            multi[5],
            n[5],
            multi[4],
            n[4],
            multi[3],
            n[3],
            terms[2],
            terms[3],
            terms[4],
            spread(&mut per),
            spread(&mut one)
        );
    }
    let mut held = 0;
    let mut money = 0.0;
    for p in w.people.iter().filter(|p| p.stage == Stage::College) {
        if let Some(d) = p.cal.nil_deal.filter(|d| d.contract()) {
            held += 1;
            money += d.amount;
        }
    }
    println!("  college players holding one: {held}, {} a year", m(money));
}

/// Who holds a multi-year deal as the winter window opens, and the
/// players like them who don't, at FBS: by group (a transfer who landed
/// last year, a blue-chip in his first season, everyone else) and his role
/// last season.
struct Window {
    rows: Vec<(u32, u32, bool, usize, usize)>,
}

const GROUPS: [&str; 3] = ["landed a year ago", "4-5★ first season", "the rest"];
const ROLES: [&str; 3] = ["starter", "rotation", "backup"];

fn under_contract(w: &World) -> Window {
    let mut rows = Vec::new();
    for p in w.people.iter().filter(|p| p.stage == Stage::College) {
        let Some(j) = p.college else { continue };
        if w.programs[j].division != Division::FBS {
            continue;
        }
        let on = p.cal.nil_deal.is_some_and(|d| d.contract() && d.years_left() >= 1);
        let group =
            if p.transfers > 0 && p.path.last() == Some(&j) && p.college_seasons >= 1 && p.cal.nil_role.is_none() {
                // Landed before last season and played it there.
                if p.path.len() >= 2 { 0 } else { 2 }
            } else if p.stars >= 4 && p.college_seasons <= 1 && p.transfers == 0 {
                1
            } else {
                2
            };
        let role = match p.last_role {
            Some(PlayingTime::Starter) => 0,
            Some(PlayingTime::Rotation) => 1,
            _ => 2,
        };
        rows.push((p.player.id, p.transfers, on, group, role));
    }
    Window { rows }
}

fn left_on_contract(w: &World, win: &Window) {
    let at: HashMap<u32, &Person> = w.people.iter().map(|p| (p.player.id, p)).collect();
    let mut n = [[[0u32; 2]; 3]; 3];
    let mut gone = [[[0u32; 2]; 3]; 3];
    for &(id, t, on, g, r) in &win.rows {
        n[g][r][on as usize] += 1;
        if at.get(&id).is_some_and(|p| p.cal.portal.is_some() || p.transfers > t) {
            gone[g][r][on as usize] += 1;
        }
    }
    println!(
        "== the winter window (item 12), FBS players who entered the portal by Feb 1, on a multi-year deal against not; buyouts paid this year {} ({}) ==",
        w.log.nil_buyouts,
        m(w.log.nil_buyout_money)
    );
    for g in 0..3 {
        let cells: Vec<String> = (0..3)
            .map(|r| {
                let pct = |k: usize| 100.0 * gone[g][r][k] as f64 / n[g][r][k].max(1) as f64;
                format!(
                    "{} {}/{} ({:.1}%) vs {}/{} ({:.1}%)",
                    ROLES[r],
                    gone[g][r][1],
                    n[g][r][1],
                    pct(1),
                    gone[g][r][0],
                    n[g][r][0],
                    pct(0)
                )
            })
            .collect();
        println!("  {}: {}", GROUPS[g], cells.join("; "));
    }
}

/// One player's market price the day before July's payday, with what the
/// pay is read against: his program, role, play over his level's starters
/// at his spot and whether he holds a deal for the year (a keep-him offer,
/// or one he signed).
struct Priced {
    j: usize,
    price: f64,
    role: Option<PlayingTime>,
    edge: f64,
    keep: bool,
    signed: bool,
    pos: Position,
}

fn priced(w: &World) -> HashMap<u32, Priced> {
    use afm_engine::world::nil_bid::DealKind;
    let year = afm_engine::world::nil_bid::deal_year(w.today);
    afm_engine::world::nil_bid::market_all(w)
        .into_iter()
        .map(|(i, price, edge)| {
            let p = &w.people[i];
            let j = p.college.expect("a college player");
            let deal = p.cal.nil_deal.filter(|d| d.pays(year));
            (
                p.player.id,
                Priced {
                    j,
                    price,
                    role: p.last_role,
                    edge: if edge.is_finite() { edge } else { 0.0 },
                    keep: deal.is_some_and(|d| d.kind == DealKind::Keep),
                    signed: deal.is_some_and(|d| d.kind != DealKind::Keep),
                    pos: p.player.position,
                },
            )
        })
        .collect()
}

/// July's pay against the market price, for everyone still at the same
/// school: the median of pay over price, and the shares under 0.5 and 0.8
/// of it and over 1.25, by level and role, by a starter's play, by deal
/// and by position group for starters.
fn against_market(w: &World, before: &HashMap<u32, Priced>) {
    println!("== July {}: AI pay against market price (the day before) ==", w.today.year());
    // (level, row label, pay / price)
    let mut rows: Vec<(&'static str, String, f64, f64)> = Vec::new();
    for p in w.people.iter().filter(|p| p.stage == Stage::College) {
        let Some(b) = before.get(&p.player.id) else { continue };
        if p.college != Some(b.j) || b.price <= 0.0 {
            continue;
        }
        let Some(g) = level(w, b.j) else { continue };
        let pay = if p.cal.nil_paid > 0.0 { p.cal.nil_paid as f64 } else { money(p) };
        let r = pay / b.price;
        let role = match b.role {
            Some(PlayingTime::Starter) => "starter",
            Some(PlayingTime::Rotation) => "rotation",
            _ => "backup",
        };
        rows.push((g, format!("1 {role}"), r, b.price));
        if role == "starter" {
            let band = match b.edge {
                x if x < 0.0 => "under the starters' median",
                x if x < 5.0 => "0-5 over",
                x if x < 10.0 => "5-10 over",
                _ => "10+ over",
            };
            rows.push((g, format!("2 starter, play {band}"), r, b.price));
            let deal = if b.keep {
                "keep-him offer"
            } else if b.signed {
                "signed deal"
            } else {
                "no deal (split)"
            };
            rows.push((g, format!("3 starter, {deal}"), r, b.price));
            let group = match b.pos {
                Position::QB => "QB",
                Position::RB => "RB",
                Position::WR => "WR",
                Position::TE => "TE",
                Position::OT | Position::IOL => "OL",
                Position::EDGE | Position::DT => "DL",
                Position::LB => "LB",
                Position::CB | Position::S => "DB",
                _ => "ST",
            };
            rows.push((g, format!("4 starter {group}"), r, b.price));
        } else {
            let band = if b.edge >= 0.0 { "playing over the starters' median" } else { "under it" };
            rows.push((g, format!("2 {role}, {band}"), r, b.price));
        }
    }
    for g in ["P4", "G5", "FCS"] {
        println!("-- {g} --");
        let mut labels: Vec<&String> = rows.iter().filter(|r| r.0 == g).map(|r| &r.1).collect();
        labels.sort();
        labels.dedup();
        for l in labels {
            let mut v: Vec<f64> = rows.iter().filter(|r| r.0 == g && &r.1 == l).map(|r| r.2).collect();
            let mut price: Vec<f64> = rows.iter().filter(|r| r.0 == g && &r.1 == l).map(|r| r.3).collect();
            let n = v.len() as f64;
            let share = |f: &dyn Fn(f64) -> bool| 100.0 * v.iter().filter(|&&x| f(x)).count() as f64 / n;
            let (u5, u8, o) = (share(&|x| x < 0.5), share(&|x| x < 0.8), share(&|x| x > 1.25));
            println!(
                "{:<44} n {:>5}  market median {:>8}  pay/market p25 {:.2} median {:.2} p75 {:.2}  under 0.5 {:>4.0}%  under 0.8 {:>4.0}%  over 1.25 {:>4.0}%",
                &l[2..],
                v.len(),
                m(q(&mut price, 0.5)),
                q(&mut v, 0.25),
                q(&mut v, 0.5),
                q(&mut v, 0.75),
                u5,
                u8,
                o
            );
        }
    }
}

/// The two scales side by side against the real anchors: for everyone
/// priced the day before and still at the same school, July's pay and the
/// market price by level, role and position (starters against The NIL
/// Standard's P4 medians, the rest by position group).
fn scales(w: &World, before: &HashMap<u32, Priced>) {
    use Position as P;
    println!(
        "== July {}: pay and market price by position (real P4 starters: QB 2M, EDGE 929K, OT 907K, WR 726K, CB 572K, RB 527K, IOL 525K) ==",
        w.today.year()
    );
    for g in ["P4", "G5", "FCS"] {
        for (role, want) in
            [("starters", PlayingTime::Starter), ("rotation", PlayingTime::Rotation), ("backups", PlayingTime::Backup)]
        {
            let mut line = Vec::new();
            for pos in P::ALL {
                let (mut pay, mut price) = (Vec::new(), Vec::new());
                for p in w.people.iter().filter(|p| p.stage == Stage::College && p.player.position == pos) {
                    let Some(b) = before.get(&p.player.id) else { continue };
                    if p.college != Some(b.j)
                        || level(w, b.j) != Some(g)
                        || !matches!(
                            (b.role, want),
                            (Some(PlayingTime::Starter), PlayingTime::Starter)
                                | (Some(PlayingTime::Rotation), PlayingTime::Rotation)
                        ) && !(want == PlayingTime::Backup
                            && !matches!(b.role, Some(PlayingTime::Starter | PlayingTime::Rotation)))
                    {
                        continue;
                    }
                    pay.push(if p.cal.nil_paid > 0.0 { p.cal.nil_paid as f64 } else { money(p) });
                    price.push(b.price);
                }
                if pay.is_empty() {
                    continue;
                }
                line.push(format!(
                    "{} n{} pay {} market {}",
                    pos.name(),
                    pay.len(),
                    m(q(&mut pay, 0.5)),
                    m(q(&mut price, 0.5))
                ));
            }
            println!("-- {g} {role}: {}", line.join(" | "));
        }
    }
}

/// June 30's deal kind and role of every Power 4 player (the payday clears
/// the deals), for `by_deal`.
fn deal_kinds(w: &World) -> HashMap<u32, (Option<afm_engine::world::nil_bid::DealKind>, Option<PlayingTime>)> {
    w.people
        .iter()
        .filter(|p| p.stage == Stage::College && p.college.is_some_and(|j| level(w, j) == Some("P4")))
        .map(|p| (p.player.id, (p.cal.nil_deal.map(|d| d.kind), p.last_role)))
        .collect()
}

/// Power 4 revenue share among the paid, by the kind of deal he held the day
/// before July's payday and his role: where the spread is lost (thread
/// "Research real NIL split by sport").
fn by_deal(w: &World, kinds: &HashMap<u32, (Option<afm_engine::world::nil_bid::DealKind>, Option<PlayingTime>)>) {
    println!("-- P4 revenue share among the paid, by June 30 deal and role: n, p10 / p50 / p90 --");
    let mut rows: HashMap<String, Vec<f64>> = HashMap::new();
    for p in w.people.iter().filter(|p| p.stage == Stage::College) {
        let Some(&(k, _)) = kinds.get(&p.player.id) else { continue };
        // The role the split read on July 1 (a season's roll moves it).
        let role = p.last_role;
        let x = p.cal.nil_school as f64;
        let deal = match k {
            None => "no deal".to_string(),
            Some(d) => format!("{d:?}"),
        };
        let name = |r: Option<PlayingTime>| match r {
            Some(PlayingTime::Starter) => "starter",
            Some(PlayingTime::Rotation) => "rotation",
            Some(_) => "backup",
            None => "none",
        };
        let r = format!("nil_role {} / last_role {}", name(p.cal.nil_role.map(|x| x.0)), name(role));
        rows.entry(format!("{deal} / {r} / {}", if x > 0.0 { "paid" } else { "no school pay" })).or_default().push(x);
    }
    let mut keys: Vec<_> = rows.keys().cloned().collect();
    keys.sort();
    for k in keys {
        let v = rows.get_mut(&k).unwrap();
        println!("  {k}: {} players; {} / {} / {}", v.len(), m(q(v, 0.1)), m(q(v, 0.5)), m(q(v, 0.9)));
    }
}
