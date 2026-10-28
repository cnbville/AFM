//! Why college dynasties form: reads a built world and prints, against real,
//! how often the strongest team wins the title, how big and how lasting the
//! leader's edge is, where its talent comes from (stars signed and on the
//! roster, its home state's share, draft picks, the portal), and the coaches
//! behind the titles with their hidden ratings.
//! Usage: dynasty_why <world file> [first year]

use afm_engine::world::World;
use afm_engine::world::coach::{Employer, Role};
use afm_engine::world::data::{Division, HISTORY_LAST};
use afm_engine::world::program::Program;
use std::collections::{BTreeMap, HashMap};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let w: World = afm_engine::save::read_world(std::path::Path::new(&args[1])).expect("read the world");
    let first: i32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1996);
    let fbs_in = |p: &Program, y: i32| {
        (if y <= HISTORY_LAST { p.real_status(y).map_or(p.division, |s| s.0) } else { p.division }) == Division::FBS
    };
    let conf_in = |p: &Program, y: i32| {
        if y <= HISTORY_LAST { p.real_status(y).map_or(p.conference.clone(), |s| s.1) } else { p.conference.clone() }
    };
    let years: Vec<i32> =
        (first..=w.year).filter(|y| w.programs.iter().any(|p| p.seasons.iter().any(|s| s.year == *y))).collect();
    let name = |p: usize| w.programs[p].real_name.as_str();

    // Talent by program and season: every player (in college now or gone) by
    // the seasons his college staffs list.
    let mut five: HashMap<(i32, usize), u32> = HashMap::new();
    let mut four: HashMap<(i32, usize), u32> = HashMap::new();
    let mut signed5: HashMap<(i32, usize), u32> = HashMap::new();
    let mut signed4: HashMap<(i32, usize), u32> = HashMap::new();
    let mut state5: HashMap<(i32, usize), (u32, u32)> = HashMap::new(); // (class, home state) -> (signed FBS, kept by the state's top taker)
    let mut by_state: HashMap<(usize, usize), u32> = HashMap::new(); // (state, program) 4-5 stars signed over the years
    let mut state_total: HashMap<usize, u32> = HashMap::new();
    let mut add_player =
        |stars: u8, home: usize, class_of: i32, staff: &[(i32, usize, Option<usize>, Option<usize>)], spec: bool| {
            if spec || stars < 4 {
                return;
            }
            let mut seen_first = false;
            for &(y, p, _, _) in staff {
                if !fbs_in(&w.programs[p], y) {
                    continue;
                }
                if stars == 5 {
                    *five.entry((y, p)).or_default() += 1;
                } else {
                    *four.entry((y, p)).or_default() += 1;
                }
                if !seen_first {
                    seen_first = true;
                    // His first FBS program in his first college year counts as his signing (JUCO movers included).
                    if y <= class_of + 1 {
                        if stars == 5 {
                            *signed5.entry((class_of, p)).or_default() += 1;
                        } else {
                            *signed4.entry((class_of, p)).or_default() += 1;
                        }
                        if class_of >= first {
                            *by_state.entry((home, p)).or_default() += 1;
                            *state_total.entry(home).or_default() += 1;
                            state5.entry((class_of, home)).or_default().0 += 1;
                        }
                    }
                }
            }
        };
    for a in &w.alumni {
        add_player(a.stars, a.home_state, a.class_of, &a.college_staff, a.position.is_specialist());
    }
    // A drafted or signed pro is already among the alumni: count him once.
    for p in w.people.iter().filter(|p| p.stage != afm_engine::world::Stage::Pro) {
        add_player(p.stars, p.home_state, p.class_of, &p.college_staff, p.player.position.is_specialist());
    }
    // Draft picks by the last FBS school, by draft year.
    let mut picks: HashMap<(i32, usize), u32> = HashMap::new();
    let mut picks_r1: HashMap<(i32, usize), u32> = HashMap::new();
    for a in &w.alumni {
        let Some(pk) = a.pick else { continue };
        let Some(&(_, p, _, _)) = a.college_staff.iter().rev().find(|e| fbs_in(&w.programs[e.1], e.0)) else {
            continue;
        };
        *picks.entry((a.year, p)).or_default() += 1;
        if pk <= 32 {
            *picks_r1.entry((a.year, p)).or_default() += 1;
        }
    }
    // Portal moves by season, FBS to FBS.
    let mut port_in: HashMap<(i32, usize), u32> = HashMap::new();
    let mut port_out: HashMap<(i32, usize), u32> = HashMap::new();
    for r in &w.recruiting_log.portal {
        *port_out.entry((r.year, r.from_program)).or_default() += 1;
        if let Some(t) = r.to_program {
            *port_in.entry((r.year, t)).or_default() += 1;
        }
    }

    // Each season's FBS table by strength.
    struct Row {
        p: usize,
        strength: f64,
        margin: f64,
        wins: u32,
        losses: u32,
        title: bool,
        coach: Option<usize>,
    }
    let mut table: BTreeMap<i32, Vec<Row>> = BTreeMap::new();
    for p in &w.programs {
        for s in &p.seasons {
            if s.year < first || !fbs_in(p, s.year) {
                continue;
            }
            let g = s.totals.games.max(1) as f64;
            table.entry(s.year).or_default().push(Row {
                p: p.id,
                strength: s.strength,
                margin: (s.totals.points_for - s.totals.points_against) as f64 / g,
                wins: s.wins,
                losses: s.losses,
                title: s.national_title,
                coach: s.coach,
            });
        }
    }
    for rows in table.values_mut() {
        rows.sort_by(|a, b| b.strength.total_cmp(&a.strength));
    }

    // 1. Titles.
    let champs: BTreeMap<i32, usize> =
        table.iter().filter_map(|(y, rows)| rows.iter().find(|r| r.title).map(|r| (*y, r.p))).collect();
    let mut count: BTreeMap<usize, u32> = BTreeMap::new();
    for &c in champs.values() {
        *count.entry(c).or_default() += 1;
    }
    let mut count: Vec<(usize, u32)> = count.into_iter().collect();
    count.sort_by_key(|x| std::cmp::Reverse(x.1));
    let cy: Vec<i32> = champs.keys().copied().collect();
    let mut most = (0, 0, 0);
    for (k, &y0) in cy.iter().enumerate() {
        let win: Vec<usize> = cy[k..].iter().filter(|&&y| y < y0 + 15).map(|y| champs[y]).collect();
        for &j in &win {
            let n = win.iter().filter(|&&x| x == j).count();
            if n > most.0 {
                most = (n, j, y0);
            }
        }
    }
    println!("world {} seed {}, {} seasons {}-{}", w.today, w.options.seed, years.len(), first, w.year);
    println!(
        "titles: {}",
        count.iter().take(8).map(|(p, n)| format!("{} {n}", name(*p))).collect::<Vec<_>>().join(", ")
    );
    println!("most in any 15 years: {} ({} from {}); real 6", most.0, name(most.1), most.2);
    let sec = champs.iter().filter(|(_, p)| w.programs[**p].conference == "SEC").count();
    println!("SEC (today's members) {} of {}; real 15 of 30 (1996-2025)", sec, champs.len());

    // 2. Does the strongest team win? Champion's strength rank and the gaps.
    let mut rank_of_champ = vec![];
    let mut gap12 = vec![];
    let mut gap110 = vec![];
    let mut top_margin = vec![];
    let mut champ_margin = vec![];
    for (y, rows) in &table {
        if rows.len() < 12 {
            continue;
        }
        if let Some(k) = rows.iter().position(|r| r.title) {
            rank_of_champ.push(k + 1);
            champ_margin.push(rows[k].margin);
        }
        gap12.push(rows[0].strength - rows[1].strength);
        gap110.push(rows[0].strength - rows[9].strength);
        let mut m: Vec<f64> = rows.iter().map(|r| r.margin).collect();
        m.sort_by(|a, b| b.total_cmp(a));
        top_margin.push(m[0]);
        let _ = y;
    }
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len().max(1) as f64;
    let share = |f: &dyn Fn(usize) -> bool| {
        rank_of_champ.iter().filter(|&&r| f(r)).count() as f64 / rank_of_champ.len().max(1) as f64
    };
    println!(
        "champion's strength rank: #1 {:.0}%, top 3 {:.0}%, top 5 {:.0}%, mean {:.1}",
        100.0 * share(&|r| r == 1),
        100.0 * share(&|r| r <= 3),
        100.0 * share(&|r| r <= 5),
        mean(&rank_of_champ.iter().map(|&r| r as f64).collect::<Vec<_>>())
    );
    println!(
        "strength gap #1-#2 {:.2}, #1-#10 {:.2} rating points; best margin a game {:.1}, champion's {:.1}",
        mean(&gap12),
        mean(&gap110),
        mean(&top_margin),
        mean(&champ_margin)
    );
    // Persistence: year-to-year correlation of strength rank among the top 25,
    // and seasons at #1 by program.
    let mut at1: BTreeMap<usize, u32> = BTreeMap::new();
    let mut top3: BTreeMap<usize, u32> = BTreeMap::new();
    for rows in table.values() {
        *at1.entry(rows[0].p).or_default() += 1;
        for r in rows.iter().take(3) {
            *top3.entry(r.p).or_default() += 1;
        }
    }
    let mut at1: Vec<(usize, u32)> = at1.into_iter().collect();
    at1.sort_by_key(|x| std::cmp::Reverse(x.1));
    let mut top3v: Vec<(usize, u32)> = top3.into_iter().collect();
    top3v.sort_by_key(|x| std::cmp::Reverse(x.1));
    println!(
        "seasons as the strongest: {}",
        at1.iter().take(6).map(|(p, n)| format!("{} {n}", name(*p))).collect::<Vec<_>>().join(", ")
    );
    println!(
        "seasons in the top 3: {}",
        top3v.iter().take(6).map(|(p, n)| format!("{} {n}", name(*p))).collect::<Vec<_>>().join(", ")
    );
    let ys: Vec<i32> = table.keys().copied().collect();
    let mut keep = vec![];
    for k in 1..ys.len() {
        let prev: HashMap<usize, usize> = table[&ys[k - 1]].iter().enumerate().map(|(i, r)| (r.p, i)).collect();
        let now = &table[&ys[k]];
        let top10: Vec<usize> = table[&ys[k - 1]].iter().take(10).map(|r| r.p).collect();
        let stay = now.iter().take(10).filter(|r| top10.contains(&r.p)).count();
        keep.push(stay as f64);
        let _ = prev;
    }
    println!("top 10 by strength still top 10 a year later: {:.1} of 10", mean(&keep));

    // 3. Talent concentration by class: 5-stars and 4-stars signed (first FBS school).
    println!(
        "\nclass  5* signed: top school (n, share), top 3 share, schools with 1+ | 4*: top school n, top 10 share | 5* on the leader's roster, leader"
    );
    let classes: Vec<i32> = (first..=w.year).collect();
    for &c in &classes {
        let mut s5: Vec<(usize, u32)> =
            signed5.iter().filter(|((y, _), _)| *y == c).map(|((_, p), n)| (*p, *n)).collect();
        let mut s4: Vec<(usize, u32)> =
            signed4.iter().filter(|((y, _), _)| *y == c).map(|((_, p), n)| (*p, *n)).collect();
        if s5.is_empty() {
            continue;
        }
        s5.sort_by_key(|x| std::cmp::Reverse(x.1));
        s4.sort_by_key(|x| std::cmp::Reverse(x.1));
        let t5: u32 = s5.iter().map(|x| x.1).sum();
        let t4: u32 = s4.iter().map(|x| x.1).sum();
        let top3: u32 = s5.iter().take(3).map(|x| x.1).sum();
        let top10_4: u32 = s4.iter().take(10).map(|x| x.1).sum();
        let lead = table.get(&(c + 2)).map(|r| r[0].p);
        let r5 = lead.map_or(0, |p| five.get(&(c + 2, p)).copied().unwrap_or(0));
        if (c - first) % 3 == 0 || c >= w.year - 4 {
            println!(
                "{c}  {} {} ({:.0}%), {:.0}%, {} | {} {}, {:.0}% | {} {}",
                name(s5[0].0),
                s5[0].1,
                100.0 * s5[0].1 as f64 / t5 as f64,
                100.0 * top3 as f64 / t5 as f64,
                s5.len(),
                name(s4.first().map_or(0, |x| x.0)),
                s4.first().map_or(0, |x| x.1),
                100.0 * top10_4 as f64 / t4.max(1) as f64,
                r5,
                lead.map_or("-", name)
            );
        }
    }
    // Home states: the top taker's share of each big state's 4-5 stars.
    let mut states: Vec<(usize, u32)> = state_total.iter().map(|(s, n)| (*s, *n)).collect();
    states.sort_by_key(|x| std::cmp::Reverse(x.1));
    println!("\nhome state's 4-5 stars ({}-): top taker and share, second", first);
    for (s, n) in states.iter().take(8) {
        let mut takers: Vec<(usize, u32)> =
            by_state.iter().filter(|((st, _), _)| st == s).map(|((_, p), k)| (*p, *k)).collect();
        takers.sort_by_key(|x| std::cmp::Reverse(x.1));
        println!(
            "{:<16} {:>5}: {} {:.0}%, {} {:.0}%",
            w.states[*s].code,
            n,
            name(takers[0].0),
            100.0 * takers[0].1 as f64 / *n as f64,
            takers.get(1).map_or("-", |t| name(t.0)),
            100.0 * takers.get(1).map_or(0, |t| t.1) as f64 / *n as f64
        );
    }
    let _ = state5;

    // 4. Dynasty programs, season by season.
    for &(p, n) in count.iter().take(2) {
        println!(
            "\n{} ({} titles): year W-L title | strength rank, gap to next | margin | 5* 4* on roster, 5* 4* signed | picks (R1) | portal in/out | coach (year in job)",
            name(p),
            n
        );
        for (y, rows) in &table {
            let Some(k) = rows.iter().position(|r| r.p == p) else { continue };
            let r = &rows[k];
            let next = if k == 0 { rows[1].strength } else { rows[0].strength };
            let coach = r.coach.map(|c| &w.coaches[c]);
            let since = coach.and_then(|c| {
                let mut from = None;
                for s in &c.history {
                    if s.employer == Employer::College(p) && s.role == Role::HeadCoach && s.from <= *y && *y <= s.to {
                        from = Some(s.from);
                    }
                }
                from.or(c
                    .job
                    .as_ref()
                    .filter(|j| j.employer == Employer::College(p) && j.role == Role::HeadCoach)
                    .map(|j| j.since))
            });
            println!(
                "{y} {:>2}-{:<2} {} | {:>3} {:+.2} | {:+5.1} | {:>2} {:>2}, {:>2} {:>2} | {:>2} ({}) | {:>2}/{:<2} | {} ({})",
                r.wins,
                r.losses,
                if r.title { "T" } else { " " },
                k + 1,
                r.strength - next,
                r.margin,
                five.get(&(*y, p)).unwrap_or(&0),
                four.get(&(*y, p)).unwrap_or(&0),
                signed5.get(&(*y, p)).unwrap_or(&0),
                signed4.get(&(*y, p)).unwrap_or(&0),
                picks.get(&(*y + 1, p)).unwrap_or(&0),
                picks_r1.get(&(*y + 1, p)).unwrap_or(&0),
                port_in.get(&(*y, p)).unwrap_or(&0),
                port_out.get(&(*y, p)).unwrap_or(&0),
                coach.map_or("-", |c| c.name.as_str()),
                since.map_or(0, |s| y - s + 1)
            );
        }
    }

    // 5. The coaches behind the titles, against every FBS head coach now.
    let hc_now: Vec<usize> = w
        .coaches
        .iter()
        .filter(|c| matches!(c.job, Some(ref j) if j.role == Role::HeadCoach && matches!(j.employer, Employer::College(p) if w.programs[p].division == Division::FBS)))
        .map(|c| c.id)
        .collect();
    let pct = |v: f64, f: &dyn Fn(usize) -> f64| {
        let below = hc_now.iter().filter(|&&c| f(c) < v).count();
        100.0 * below as f64 / hc_now.len().max(1) as f64
    };
    let ab = |c: usize| afm_engine::debug::coach_ability(&w.coaches[c]);
    let teach = |c: usize| ab(c).teaching.iter().sum::<f64>() / ab(c).teaching.len() as f64;
    let mut by_coach: BTreeMap<(usize, usize), (u32, i32, i32)> = BTreeMap::new();
    for (y, rows) in &table {
        if let Some(r) = rows.iter().find(|r| r.title)
            && let Some(c) = r.coach
        {
            let e = by_coach.entry((c, r.p)).or_insert((0, *y, *y));
            e.0 += 1;
            e.2 = *y;
        }
    }
    let mut by_coach: Vec<_> = by_coach.into_iter().collect();
    by_coach.sort_by_key(|x| std::cmp::Reverse(x.1.0));
    println!(
        "\ntitle coaches: titles at program (first-last) | hidden recruiting, game mgmt, scheme, teaching, head value (percentile among FBS head coaches now) | age, now"
    );
    for ((c, p), (n, a, b)) in by_coach.iter().take(8) {
        let co = &w.coaches[*c];
        let x = ab(*c);
        let now = match &co.job {
            Some(j) => match j.employer {
                Employer::College(q) => format!("{} {}", w.programs[q].real_name, j.role.name()),
                Employer::Nfl(t) => format!("NFL {} {}", w.nfl_team_name(t), j.role.name()),
                _ => j.role.name().to_string(),
            },
            None => {
                if co.retired {
                    "retired".into()
                } else {
                    "no job".into()
                }
            }
        };
        let ended = co
            .history
            .iter()
            .find(|s| s.employer == Employer::College(*p) && s.role == Role::HeadCoach)
            .map_or(String::new(), |s| format!(", left {} ({})", s.to, s.ended));
        println!(
            "{} {} {} ({a}-{b}) | rec {:.1} ({:.0}), gm {:.1} ({:.0}), sch {:.1} ({:.0}), teach {:.1} ({:.0}), hv {:.1} ({:.0}) | {}, {}{}",
            co.name,
            name(*p),
            n,
            x.recruiting,
            pct(x.recruiting, &|c| ab(c).recruiting),
            x.game_management,
            pct(x.game_management, &|c| ab(c).game_management),
            x.scheme,
            pct(x.scheme, &|c| ab(c).scheme),
            teach(*c),
            pct(teach(*c), &teach),
            x.head_coach_value(),
            pct(x.head_coach_value(), &|c| ab(c).head_coach_value()),
            co.age,
            now,
            ended
        );
    }
    // 6. Like real: roster blue chips (5* = 3, 4* = 1, last four signing
    // classes), the #1 and top 3 by season, and the champion's rank.
    let mut bc_rank: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
    for &y in table.keys() {
        let mut pts: HashMap<usize, u32> = HashMap::new();
        for c in y - 3..=y {
            for (&(cy, p), &n) in &signed5 {
                if cy == c {
                    *pts.entry(p).or_default() += 3 * n;
                }
            }
            for (&(cy, p), &n) in &signed4 {
                if cy == c {
                    *pts.entry(p).or_default() += n;
                }
            }
        }
        let mut v: Vec<(usize, u32)> = pts.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        bc_rank.insert(y, v.into_iter().map(|x| x.0).collect());
    }
    let ys: Vec<i32> = bc_rank.keys().copied().filter(|y| *y >= first + 3).collect();
    let mut one: BTreeMap<usize, u32> = BTreeMap::new();
    let mut three: BTreeMap<usize, u32> = BTreeMap::new();
    let mut crank = vec![];
    for y in &ys {
        let v = &bc_rank[y];
        *one.entry(v[0]).or_default() += 1;
        for &p in v.iter().take(3) {
            *three.entry(p).or_default() += 1;
        }
        if let Some(c) = champs.get(y) {
            crank.push(v.iter().position(|p| p == c).map_or(99, |k| k + 1));
        }
    }
    let top = |m: &BTreeMap<usize, u32>| {
        let mut v: Vec<(usize, u32)> = m.iter().map(|(a, b)| (*a, *b)).collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.1));
        v.iter().take(6).map(|(p, n)| format!("{} {n}", name(*p))).collect::<Vec<_>>().join(", ")
    };
    println!(
        "\nroster blue chips like real ({}-{}; real 2005-2025: #1 Alabama 7, Georgia 7, USC 6; top 3 Alabama 15, USC 9, Florida 8, Georgia 8)",
        ys[0],
        ys[ys.len() - 1]
    );
    println!("#1: {}", top(&one));
    println!("top 3: {}", top(&three));
    println!(
        "champion's blue-chip rank {:?}: #1 {}, top 3 {}, top 5 {} of {} (real 5, 10, 14 of 20)",
        crank,
        crank.iter().filter(|&&r| r == 1).count(),
        crank.iter().filter(|&&r| r <= 3).count(),
        crank.iter().filter(|&&r| r <= 5).count(),
        crank.len()
    );

    // 7. How much the head coach moves a program, against the program itself:
    // within each program, the class haul (blue-chip points) and the season's
    // strength and margin against the head coach's hidden ratings.
    let mut hx: Vec<(usize, f64, f64, f64, f64, f64)> = vec![]; // program, rec, hv, haul, strength, margin
    // Class y is signed by the coach of season y (the carousel comes
    // before signing day); the last season's class has no rows yet.
    for (y, rows) in table.iter().filter(|(y, _)| **y < w.year) {
        for r in rows {
            let Some(c) = r.coach else { continue };
            let x = ab(c);
            let haul =
                3 * signed5.get(&(*y, r.p)).copied().unwrap_or(0) + signed4.get(&(*y, r.p)).copied().unwrap_or(0);
            hx.push((r.p, x.recruiting, x.head_coach_value(), haul as f64, r.strength, r.margin));
        }
    }
    let mut means: HashMap<usize, (f64, f64, f64, f64, f64, f64)> = HashMap::new();
    for h in &hx {
        let e = means.entry(h.0).or_default();
        e.0 += h.1;
        e.1 += h.2;
        e.2 += h.3;
        e.3 += h.4;
        e.4 += h.5;
        e.5 += 1.0;
    }
    let slope = |xi: usize, yi: usize, only_big: bool| {
        let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
        for h in &hx {
            let m = means[&h.0];
            if only_big && m.2 / m.5 < 4.0 {
                continue;
            }
            let xs = [h.1 - m.0 / m.5, h.2 - m.1 / m.5];
            let ysv = [h.3 - m.2 / m.5, h.4 - m.3 / m.5, h.5 - m.4 / m.5];
            sxy += xs[xi] * ysv[yi];
            sxx += xs[xi] * xs[xi];
            syy += ysv[yi] * ysv[yi];
        }
        (sxy / sxx, sxy / (sxx * syy).sqrt())
    };
    // Share of each measure's spread that sits with the program.
    let between = |yi: usize| {
        let all: Vec<f64> = hx.iter().map(|h| [h.3, h.4, h.5][yi]).collect();
        let mu = mean(&all);
        let tot: f64 = all.iter().map(|v| (v - mu).powi(2)).sum();
        let within: f64 = hx
            .iter()
            .map(|h| {
                let m = means[&h.0];
                ([h.3, h.4, h.5][yi] - [m.2, m.3, m.4][yi] / m.5).powi(2)
            })
            .sum();
        1.0 - within / tot
    };
    let (a, ra) = slope(0, 0, false);
    let (b, rb) = slope(0, 0, true);
    let (c1, rc) = slope(1, 1, false);
    let (d, rd) = slope(1, 2, false);
    let (e, re) = slope(1, 2, true);
    println!("\nhead coach against the program (within each program, FBS {}-{}):", first, w.year);
    println!(
        "program's share of the spread: class haul {:.0}%, strength {:.0}%, margin {:.0}%",
        100.0 * between(0),
        100.0 * between(1),
        100.0 * between(2)
    );
    println!(
        "class haul per recruiting point {:+.2} (r {:.2}); at programs with 4+ a class {:+.2} (r {:.2})",
        a, ra, b, rb
    );
    println!(
        "strength per head-value point {:+.2} (r {:.2}); margin a game {:+.2} (r {:.2}); at the big programs {:+.2} (r {:.2})",
        c1, rc, d, rd, e, re
    );
    // Titles by the coach's standing now, only for coaches still working
    // and under 59 (ratings drift with age: learning to 58, decline after).
    let mut tiers = [0u32; 4];
    for (y, rows) in &table {
        let _ = y;
        if let Some(r) = rows.iter().find(|r| r.title)
            && let Some(c) = r.coach
            && w.coaches[c].job.is_some()
            && w.coaches[c].age < 59
        {
            let q = pct(ab(c).head_coach_value(), &|c| ab(c).head_coach_value());
            tiers[if q >= 90.0 {
                0
            } else if q >= 75.0 {
                1
            } else if q >= 50.0 {
                2
            } else {
                3
            }] += 1;
        }
    }
    println!(
        "titles by the coach's head value now (working coaches under 59): top 10% {}, 75-90% {}, 50-75% {}, bottom half {}",
        tiers[0], tiers[1], tiers[2], tiers[3]
    );
    // The carousel: do the best coaches hold the best jobs?
    let mut cp: Vec<(f64, f64)> = hc_now
        .iter()
        .filter_map(|&c| match w.coaches[c].job.as_ref()?.employer {
            Employer::College(p) => Some((ab(c).head_coach_value(), w.programs[p].prestige)),
            _ => None,
        })
        .collect();
    let n = cp.len() as f64;
    let (mx, my) = (cp.iter().map(|x| x.0).sum::<f64>() / n, cp.iter().map(|x| x.1).sum::<f64>() / n);
    let cov: f64 = cp.iter().map(|x| (x.0 - mx) * (x.1 - my)).sum();
    let vx: f64 = cp.iter().map(|x| (x.0 - mx).powi(2)).sum();
    let vy: f64 = cp.iter().map(|x| (x.1 - my).powi(2)).sum();
    cp.sort_by(|a, b| b.1.total_cmp(&a.1));
    let top15: Vec<f64> = cp.iter().take(15).map(|x| pct(x.0, &|c| ab(c).head_coach_value())).collect();
    println!(
        "head value against prestige, FBS head coaches now: r {:.2}; the 15 biggest names' coaches by percentile {:?}",
        cov / (vx * vy).sqrt(),
        top15.iter().map(|x| *x as i32).collect::<Vec<_>>()
    );
    // How long weak coaches last at big programs: stints at the 15 highest-prestige programs.
    let mut big: Vec<usize> = (0..w.programs.len()).filter(|&p| w.programs[p].division == Division::FBS).collect();
    big.sort_by(|a, b| w.programs[*b].prestige.total_cmp(&w.programs[*a].prestige));
    big.truncate(15);
    let mut lens = [(0.0f64, 0.0f64); 2];
    for co in &w.coaches {
        let q = pct(ab(co.id).head_coach_value(), &|c| ab(c).head_coach_value());
        for s in &co.history {
            if let Employer::College(p) = s.employer
                && s.role == Role::HeadCoach
                && big.contains(&p)
                && s.from >= first
            {
                let k = if q < 50.0 { 1 } else { 0 };
                lens[k].0 += (s.to - s.from + 1) as f64;
                lens[k].1 += 1.0;
            }
        }
    }
    println!(
        "finished head coach stints at the 15 biggest names since {first}: top half {:.1} seasons ({} stints), bottom half {:.1} ({})",
        lens[0].0 / lens[0].1.max(1.0),
        lens[0].1,
        lens[1].0 / lens[1].1.max(1.0),
        lens[1].1
    );
    // 8. Like real: at the 20 biggest programs (by blue chips signed), the
    // share of the spread in class hauls (share of the class's blue-chip
    // points) and in margin a game that sits with the program, then with the
    // program and its head coach, and the noise floor with the coach labels
    // shuffled within each program (real 2003-2025: haul 31% / 61% / 41%,
    // margin 23% / 54% / 36%).
    let mut tot_pts: HashMap<i32, f64> = HashMap::new();
    let mut prog_pts: HashMap<usize, f64> = HashMap::new();
    for (&(c, p), &n) in signed5.iter() {
        *tot_pts.entry(c).or_default() += 3.0 * n as f64;
        *prog_pts.entry(p).or_default() += 3.0 * n as f64;
    }
    for (&(c, p), &n) in signed4.iter() {
        *tot_pts.entry(c).or_default() += n as f64;
        *prog_pts.entry(p).or_default() += n as f64;
    }
    let mut big20: Vec<(usize, f64)> = prog_pts.into_iter().collect();
    big20.sort_by(|a, b| b.1.total_cmp(&a.1));
    let big20: Vec<usize> = big20.into_iter().take(20).map(|x| x.0).collect();
    let mut hauls: Vec<(usize, usize, f64)> = vec![]; // program, coach, value
    let mut margins: Vec<(usize, usize, f64)> = vec![];
    for (y, rows) in &table {
        for r in rows.iter().filter(|r| big20.contains(&r.p)) {
            let Some(c) = r.coach else { continue };
            margins.push((r.p, c, r.margin));
            if *y >= first + 3 && *y < w.year {
                let h = 3.0 * signed5.get(&(*y, r.p)).copied().unwrap_or(0) as f64
                    + signed4.get(&(*y, r.p)).copied().unwrap_or(0) as f64;
                hauls.push((r.p, c, 1000.0 * h / tot_pts.get(y).copied().unwrap_or(1.0)));
            }
        }
    }
    let r2 = |v: &[(usize, usize, f64)], by_coach: bool| {
        let mu = v.iter().map(|x| x.2).sum::<f64>() / v.len() as f64;
        let tot: f64 = v.iter().map(|x| (x.2 - mu).powi(2)).sum();
        let mut g: HashMap<(usize, usize), (f64, f64)> = HashMap::new();
        for x in v {
            let e = g.entry((x.0, if by_coach { x.1 } else { 0 })).or_default();
            e.0 += x.2;
            e.1 += 1.0;
        }
        let within: f64 = v
            .iter()
            .map(|x| {
                let e = g[&(x.0, if by_coach { x.1 } else { 0 })];
                (x.2 - e.0 / e.1).powi(2)
            })
            .sum();
        1.0 - within / tot
    };
    let floor = |v: &[(usize, usize, f64)]| {
        let mut seed: u64 = 12345;
        let mut sum = 0.0;
        for _ in 0..200 {
            let mut q = v.to_vec();
            for &p in &big20 {
                let idx: Vec<usize> = (0..q.len()).filter(|&i| q[i].0 == p).collect();
                let mut labels: Vec<usize> = idx.iter().map(|&i| q[i].1).collect();
                for k in (1..labels.len()).rev() {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                    let j = (seed >> 33) as usize % (k + 1);
                    labels.swap(k, j);
                }
                for (n, &i) in idx.iter().enumerate() {
                    q[i].1 = labels[n];
                }
            }
            sum += r2(&q, true);
        }
        sum / 200.0
    };
    let stints: std::collections::HashSet<(usize, usize)> = margins.iter().map(|x| (x.0, x.1)).collect();
    println!(
        "\nthe 20 biggest programs, like real: class haul program {:.0}% / with its head coach {:.0}% / noise floor {:.0}% (real 31/61/41); margin {:.0}% / {:.0}% / {:.0}% (real 23/54/36); {} coach stints (real 88)",
        100.0 * r2(&hauls, false),
        100.0 * r2(&hauls, true),
        100.0 * floor(&hauls),
        100.0 * r2(&margins, false),
        100.0 * r2(&margins, true),
        100.0 * floor(&margins),
        stints.len()
    );
    let (hp, hc, hf) = (r2(&hauls, false), r2(&hauls, true), floor(&hauls));
    let (mp, mc, mf) = (r2(&margins, false), r2(&margins, true), floor(&margins));
    println!(
        "the head coach's share of the spread within a program (with coach - floor) / (1 - program): class haul {:.0}% (real 29%), margin {:.0}% (real 23%)",
        100.0 * (hc - hf) / (1.0 - hp),
        100.0 * (mc - mf) / (1.0 - mp)
    );
    // 9. The biggest names now: prestige and its parts, the collective, and
    // the head coach's standing.
    let mut names: Vec<usize> = (0..w.programs.len()).filter(|&p| w.programs[p].division == Division::FBS).collect();
    names.sort_by(|a, b| w.programs[*b].prestige.total_cmp(&w.programs[*a].prestige));
    println!(
        "\nbiggest names now: prestige (rel) | tradition recent facilities fans | boosters $M budget $M | blue-chip points last 4 classes | coach head value pct"
    );
    for &p in names.iter().take(15) {
        let g = &w.programs[p];
        let bc: u32 = (w.year - 3..=w.year)
            .map(|c| 3 * signed5.get(&(c, p)).copied().unwrap_or(0) + signed4.get(&(c, p)).copied().unwrap_or(0))
            .sum();
        let hv = g.head_coach.map_or(-1.0, |c| pct(ab(c).head_coach_value(), &|c| ab(c).head_coach_value()));
        println!(
            "{:<16} {:.2} ({:.2}) | {:.2} {:.2} {:.2} {:.2} | {:.1} {:.1} | {} | {:.0}",
            g.real_name,
            g.prestige,
            g.prestige_rel,
            g.tradition,
            g.recent,
            g.facilities,
            g.fans,
            g.collective.boosters() / 1e6,
            g.collective.budget / 1e6,
            bc,
            hv
        );
    }
    // 10. Who builds the staff: the assistants' quality against the head
    // coach's and against the program's prestige (FBS now).
    let corr = |v: &[(f64, f64)]| {
        let n = v.len() as f64;
        let (mx, my) = (v.iter().map(|x| x.0).sum::<f64>() / n, v.iter().map(|x| x.1).sum::<f64>() / n);
        let c: f64 = v.iter().map(|x| (x.0 - mx) * (x.1 - my)).sum();
        let vx: f64 = v.iter().map(|x| (x.0 - mx).powi(2)).sum();
        let vy: f64 = v.iter().map(|x| (x.1 - my).powi(2)).sum();
        c / (vx * vy).sqrt()
    };
    let mut by_hc = vec![];
    let mut by_pr = vec![];
    let mut hc_pr = vec![];
    for g in w.programs.iter().filter(|g| g.division == Division::FBS) {
        let Some(h) = g.head_coach else { continue };
        let staff: Vec<f64> = [g.oc, g.dc]
            .into_iter()
            .flatten()
            .map(|c| 0.5 * ab(c).scheme + 0.5 * ab(c).teaching.iter().sum::<f64>() / ab(c).teaching.len() as f64)
            .chain(
                g.position_coaches
                    .iter()
                    .flatten()
                    .map(|&c| ab(c).teaching.iter().sum::<f64>() / ab(c).teaching.len() as f64),
            )
            .collect();
        if staff.is_empty() {
            continue;
        }
        let q = staff.iter().sum::<f64>() / staff.len() as f64;
        by_hc.push((ab(h).head_coach_value(), q));
        by_pr.push((g.prestige, q));
        hc_pr.push((ab(h).head_coach_value(), g.prestige));
    }
    println!(
        "\nassistants' quality (coordinators' scheme and teaching, position coaches' teaching) against the head coach's head value r {:.2}, against prestige r {:.2}; head value against prestige r {:.2} ({} FBS staffs)",
        corr(&by_hc),
        corr(&by_pr),
        corr(&hc_pr),
        by_hc.len()
    );
    // 11. Why blue chips pick the top 3 (trial tooling, 2026-10-28): for the
    // last two classes, each 4- or 5-star at one of the three programs with
    // the most blue chips over the last four classes, his appeal there
    // against the best of programs 4-12 (one that offered him when his
    // offers are kept), term by term; then the reverse for blue chips at
    // programs 4-12 against the best of the top 3. The process is left out:
    // visits, the staff's bond, his room, a commitment.
    let mut bc_rank: Vec<(usize, u32)> = (0..w.programs.len())
        .filter(|&p| w.programs[p].division == Division::FBS)
        .map(|p| {
            let pts = (w.year - 4..w.year)
                .map(|c| 3 * signed5.get(&(c, p)).copied().unwrap_or(0) + signed4.get(&(c, p)).copied().unwrap_or(0))
                .sum();
            (p, pts)
        })
        .collect();
    bc_rank.sort_by_key(|x| std::cmp::Reverse(x.1));
    let top3: Vec<usize> = bc_rank.iter().take(3).map(|x| x.0).collect();
    let next: Vec<usize> = bc_rank.iter().skip(3).take(9).map(|x| x.0).collect();
    const TERMS: [&str; 13] = [
        "level",
        "brand",
        "5-star pull",
        "prestige grades",
        "facilities",
        "development",
        "money",
        "playing time",
        "close",
        "home ties",
        "academics",
        "rest",
        "feel",
    ];
    let mut worst_check = 0.0f64;
    let mut gaps = |at: &[usize], against: &[usize], stars: Option<u8>| {
        let (mut sum, mut n, mut ahead, mut offered) = ([0.0f64; 13], 0u32, 0u32, 0u32);
        let (mut split, mut in_state, mut fan) = ([0.0f64; 4], 0u32, 0u32);
        for (i, p) in w.people.iter().enumerate() {
            if p.stars < 4 || stars.is_some_and(|s| p.stars != s) || p.player.position.is_specialist() {
                continue;
            }
            if p.class_of < w.year - 1 || p.class_of > w.year {
                continue;
            }
            let Some(j) = p.cal.signed.map(|s| s.program).or(p.college) else { continue };
            if !at.contains(&j) {
                continue;
            }
            let kept: Vec<usize> = against.iter().copied().filter(|x| p.cal.offers.contains(x)).collect();
            let pool = if kept.is_empty() { against.to_vec() } else { kept.clone() };
            let terms = |x: usize| afm_engine::debug::recruit_terms(&w, i, x);
            let total = |t: &[f64; 18]| t[..13].iter().sum::<f64>();
            let mine = terms(j);
            let Some(alt) = pool.iter().map(|&x| terms(x)).max_by(|a, b| total(a).total_cmp(&total(b))) else {
                continue;
            };
            for t in [&mine, &alt] {
                worst_check = worst_check.max((t[..12].iter().sum::<f64>() - t[13]).abs());
            }
            for g in 0..13 {
                sum[g] += mine[g] - alt[g];
            }
            for g in 0..4 {
                split[g] += mine[14 + g] - alt[14 + g];
            }
            in_state += (p.home_state == w.programs[j].state) as u32;
            fan += (p.childhood_team == Some(j)) as u32;
            n += 1;
            ahead += (total(&mine) > total(&alt)) as u32;
            offered += !kept.is_empty() as u32;
        }
        let nf = n.max(1) as f64;
        let parts = (0..13).map(|g| format!("{} {:+.3}", TERMS[g], sum[g] / nf)).collect::<Vec<_>>().join(", ");
        format!(
            "n {n} (offers kept for {offered}), ahead on appeal {:.0}%; mean gap: {parts}; total {:+.3}; home ties split: pipeline {:+.3}, stay home {:+.3}, childhood team {:+.3}, brother {:+.3}; from his school's state {:.0}%, grew up a fan of it {:.0}%",
            100.0 * ahead as f64 / nf,
            sum.iter().sum::<f64>() / nf,
            split[0] / nf,
            split[1] / nf,
            split[2] / nf,
            split[3] / nf,
            100.0 * in_state as f64 / nf,
            100.0 * fan as f64 / nf
        )
    };
    let names = |v: &[usize]| v.iter().map(|&p| name(p)).collect::<Vec<_>>().join(", ");
    println!(
        "\nwhy blue chips pick the top 3 (classes {}-{}; top 3 by blue chips over the last four classes: {}; programs 4-12: {}; the process is left out: visits, bond, room, commitment)",
        w.year - 1,
        w.year,
        names(&top3),
        names(&next)
    );
    println!("at the top 3, against the best of 4-12: {}", gaps(&top3, &next, None));
    println!("  5-stars only: {}", gaps(&top3, &next, Some(5)));
    println!("at 4-12, against the best of the top 3: {}", gaps(&next, &top3, None));
    println!("  (the terms add up to the appeal within {worst_check:.1e})");
    // Draft: the most picks from one school in a draft, by era.
    let mut most_picks: BTreeMap<i32, (usize, u32)> = BTreeMap::new();
    for (&(y, p), &n) in &picks {
        let e = most_picks.entry(y).or_insert((p, 0));
        if n > e.1 {
            *e = (p, n);
        }
    }
    println!(
        "\nmost picks from one school (draft year: school n): {}",
        most_picks
            .iter()
            .filter(|(y, _)| **y > first + 3)
            .map(|(y, (p, n))| format!("{y} {} {n}", name(*p)))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = conf_in;
}
