//! Where college players come from (minor bugs 255 and 257): a world's FBS
//! players in their home state by state, the FBS team mean in-state, and
//! chosen teams' rosters by distance from home (in state, other states
//! under 500 miles, 500-1,000, 1,000+), against 2025 rosters.
//! Usage: home_bands <world file> [first class]: with a class, only the
//! players of that class and later (the ones a variant recruited).
use afm_engine::world::data::Division;
use afm_engine::world::geo;
use std::collections::BTreeMap;
use std::path::Path;

/// 2025 FBS rosters (home state from the roster pages): players from each
/// state at an FBS school in that state, %.
const REAL_STAY: [(&str, f64); 27] = [
    ("AL", 46.0),
    ("AZ", 20.0),
    ("CA", 32.0),
    ("CO", 30.0),
    ("FL", 24.0),
    ("GA", 27.0),
    ("IA", 53.0),
    ("IL", 26.0),
    ("IN", 39.0),
    ("LA", 44.0),
    ("MD", 22.0),
    ("MI", 44.0),
    ("MO", 25.0),
    ("MS", 31.0),
    ("NC", 39.0),
    ("NJ", 18.0),
    ("NY", 35.0),
    ("OH", 49.0),
    ("OK", 46.0),
    ("PA", 27.0),
    ("SC", 39.0),
    ("TN", 33.0),
    ("TX", 47.0),
    ("UT", 66.0),
    ("VA", 40.0),
    ("WA", 27.0),
    ("WI", 23.0),
];

const TEAMS: [&str; 16] = [
    "Iowa",
    "Iowa State",
    "Wisconsin",
    "Nebraska",
    "Kansas State",
    "Minnesota",
    "Washington",
    "Washington State",
    "Florida",
    "Florida State",
    "Miami (FL)",
    "UCF",
    "Texas",
    "Ohio State",
    "Alabama",
    "Utah",
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let w = afm_engine::save::read_world(Path::new(&args[1])).expect("read the world");
    let since: i32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(i32::MIN);
    eprintln!("{} {:?}", args[1], w.today);
    let code = |s: usize| geo::STATE_CENTERS[s].0;
    let mut stay: BTreeMap<&str, (u32, u32)> = BTreeMap::new();
    let (mut all, mut ins, mut team_means) = (0u32, 0u32, Vec::new());
    for p in w.programs.iter().filter(|p| p.division == Division::FBS) {
        if p.roster.len() < 40 {
            continue;
        }
        let roster: Vec<usize> = p.roster.iter().copied().filter(|&i| w.people[i].class_of >= since).collect();
        if roster.is_empty() {
            continue;
        }
        let campus = geo::campus(p.state, &p.city);
        let mut b = [0u32; 4];
        let mut fl_tx = 0;
        for &i in &roster {
            let q = &w.people[i];
            let e = stay.entry(code(q.home_state)).or_default();
            e.0 += 1;
            all += 1;
            if q.home_state == p.state {
                e.1 += 1;
                ins += 1;
                b[0] += 1;
            } else {
                let home = match w.high_schools.get(q.hs) {
                    Some(h) => geo::place(h.state, &h.town),
                    None => {
                        let (_, lat, lon) = geo::STATE_CENTERS[q.home_state.min(50)];
                        (lat, lon)
                    }
                };
                let m = geo::miles_between(home, campus);
                b[if m < 500.0 {
                    1
                } else if m < 1000.0 {
                    2
                } else {
                    3
                }] += 1;
            }
            if matches!(code(q.home_state), "FL" | "TX") {
                fl_tx += 1;
            }
        }
        let n = roster.len() as f64;
        team_means.push(b[0] as f64 / n);
        if TEAMS.contains(&p.real_name.as_str()) {
            println!(
                "{:<17} n {:3}: in state {:3.0}, other <500 {:3.0}, 500-1000 {:3.0}, 1000+ {:3.0}; FL+TX {:3.0}",
                p.real_name,
                roster.len(),
                100.0 * b[0] as f64 / n,
                100.0 * b[1] as f64 / n,
                100.0 * b[2] as f64 / n,
                100.0 * b[3] as f64 / n,
                100.0 * fl_tx as f64 / n
            );
        }
    }
    println!(
        "FBS players in their home state {:.1}% (real 35.3), team mean {:.1}% (real 35.6)",
        100.0 * ins as f64 / all.max(1) as f64,
        100.0 * team_means.iter().sum::<f64>() / team_means.len().max(1) as f64
    );
    // Each state's share of all FBS players against its share of real FBS
    // signees (2022-26 classes, `data/real-world/talent_by_state.csv`): a
    // state that grows too few players fills its own schools more.
    let real_signees: BTreeMap<String, f64> = std::fs::read_to_string("data/real-world/talent_by_state.csv")
        .unwrap_or_default()
        .lines()
        .skip(1)
        .filter_map(|l| {
            let mut f = l.split(',');
            Some((f.next()?.to_string(), f.next()?.parse().ok()?))
        })
        .collect();
    let real_all: f64 = real_signees.values().sum();
    let mut err = 0.0;
    for (s, real) in REAL_STAY {
        let (n, k) = stay.get(s).copied().unwrap_or((0, 0));
        let x = 100.0 * k as f64 / n.max(1) as f64;
        err += (x - real).powi(2);
        let share = real_signees.get(s).map_or(f64::NAN, |v| 100.0 * v / real_all.max(1.0));
        println!(
            "from {s}: {n:5} FBS players ({:4.1}% of FBS, real {share:4.1}%), {x:3.0}% stay (real {real:.0})",
            100.0 * n as f64 / all.max(1) as f64
        );
    }
    println!("rms gap over the 27 states: {:.1} points", (err / REAL_STAY.len() as f64).sqrt());
}
