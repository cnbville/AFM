//! How much NIL money moves a recruit's and a transfer's choice (playtest
//! item F3, Cain 2026-11-12: an SEC school offering double the field's
//! money wasn't in a player's first ten). For every recruit of the class
//! signing next and every picked school on his list, the school's money is
//! set to 0, 1, 1.5, 2 and 3 times the best money any other school on his
//! list offers, and every school's place on his list is read again by the
//! same appeal his race and his signing day weigh (`recruiting::appeals`).
//! The same for portal entrants in an open window
//! (`recruiting::portal_race`). Results by what he weighs: money first of
//! his six reasons, among his first three, or lower.
//!
//! Usage: nil_pull play <world> <out dir> [recruit date] [portal date] [last date]
//!          plays the world on (games on the stand-in) and saves it on each
//!          of the first two dates (`recruits.afmworld`, `portal.afmworld`;
//!          an out dir of "-" saves nothing), so a change is measured on
//!          the very same recruits and entrants. On the last date it prints
//!          where they went: who signed with the most money on his list,
//!          the blue-chips by conference, where the entrants landed.
//!        nil_pull recruits <world> [conference]
//!        nil_pull portal <world> [conference] [most entrants]
//! Dates are yyyy-mm-dd. The conference (default SEC) names the schools
//! whose money is moved; "power" takes every power-conference school.
//! `AFM_NIL_PULL=off` runs the game as it was before the fix (no lift for
//! the clear top offer, a transfer weighing money as a recruit does), for
//! a before and after on one build.
//! `AFM_RICH=<times>[:conference]` (with `play`) multiplies the money of
//! four schools all cycle long, the conference's fifth to eighth by
//! prestige (default SEC), and prints what they signed and who came
//! through the portal since the start: a school that offers double, end to
//! end. `1` moves nothing and prints the same lines, for the base.

use afm_engine::PlayingTime;
use afm_engine::calendar::Date;
use afm_engine::debug;
use afm_engine::world::front_office::college as desk;
use afm_engine::world::{Stage, World};
use std::path::Path;

const TIMES: [f64; 5] = [0.0, 1.0, 1.5, 2.0, 3.0];

fn date(s: &str) -> Date {
    let v: Vec<i32> = s.split('-').map(|x| x.parse().expect("yyyy-mm-dd")).collect();
    Date::ymd(v[0], v[1] as u32, v[2] as u32)
}

fn read(path: &str) -> World {
    let t = std::time::Instant::now();
    let w = afm_engine::save::read_world(Path::new(path)).expect("read the world");
    eprintln!("read {path} in {:.0}s: {}", t.elapsed().as_secs_f64(), w.today.pretty());
    w
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if std::env::var("AFM_NIL_PULL").is_ok_and(|s| s == "off") {
        debug::before_f3(true);
        println!("(the game as it was before the fix)");
    }
    match args.get(1).map(String::as_str) {
        Some("play") => play(&args),
        Some("recruits") => recruits(&read(&args[2]), args.get(3).map_or("SEC", String::as_str)),
        Some("portal") => portal(
            &read(&args[2]),
            args.get(3).map_or("SEC", String::as_str),
            args.get(4).and_then(|s| s.parse().ok()).unwrap_or(usize::MAX),
        ),
        _ => eprintln!(
            "usage: nil_pull play <world> <out dir> | recruits <world> [conference] | portal <world> [conference]"
        ),
    }
}

/// The schools with an offer out to a recruit: his list.
fn list_of(p: &afm_engine::world::Person) -> Vec<usize> {
    let mut schools: Vec<usize> = p.cal.offers.iter().copied().chain(p.cal.early_offers.iter().map(|o| o.0)).collect();
    if let Some(c) = p.cal.commit {
        schools.push(c.0);
    }
    schools.sort_unstable();
    schools.dedup();
    schools
}

/// The place of the best money in `money` when it is $1,000 or more, and
/// how many times the next it is.
fn richest(money: &[f64]) -> Option<(usize, f64)> {
    let at = (0..money.len()).max_by(|&a, &b| money[a].total_cmp(&money[b]))?;
    let next = money.iter().enumerate().filter(|x| x.0 != at).map(|x| *x.1).fold(0.0, f64::max);
    (money[at] >= 1_000.0).then(|| (at, money[at] / next.max(1_000.0)))
}

/// A recruit on the first date: who led his race, who offered the most.
struct Pick {
    id: u32,
    stars: u8,
    leader: usize,
    /// The school with the most money on his list, and how many times the next offer.
    rich: Option<(usize, f64)>,
    commit: Option<usize>,
    money_rank: usize,
    /// The moved schools on his list.
    moved: Vec<usize>,
}

/// An entrant on the second date: his first choice, who paid the most.
struct Shopper {
    id: u32,
    from: usize,
    division: afm_engine::world::data::Division,
    first: usize,
    rich: Option<(usize, f64)>,
    money_rank: usize,
}

/// The schools whose money `AFM_RICH` moves: the conference's fifth to
/// eighth by prestige, and by how many times.
fn moved_schools(w: &World) -> (Vec<usize>, f64) {
    let Ok(spec) = std::env::var("AFM_RICH") else { return (Vec::new(), 1.0) };
    let mut parts = spec.split(':');
    let times: f64 = parts.next().and_then(|x| x.parse().ok()).expect("AFM_RICH=<times>[:conference]");
    let conference = parts.next().unwrap_or("SEC");
    let mut schools: Vec<usize> = (0..w.programs.len()).filter(|&j| w.programs[j].conference == conference).collect();
    schools.sort_by(|&a, &b| w.programs[b].prestige.total_cmp(&w.programs[a].prestige).then(a.cmp(&b)));
    let schools: Vec<usize> = schools.into_iter().skip(4).take(4).collect();
    (schools, times)
}

fn play(args: &[String]) {
    let mut w = read(&args[2]);
    w.options.engine_from = Some(2100);
    let out = Path::new(&args[3]);
    let save = args[3] != "-";
    if save {
        std::fs::create_dir_all(out).expect("the folder");
    }
    let season = w.today.year();
    let first = args.get(4).map_or(Date::ymd(season, 11, 15), |s| date(s));
    let second = args.get(5).map_or(Date::ymd(season + 1, 1, 8), |s| date(s));
    let last = args.get(6).map_or(Date::ymd(season + 1, 2, 20), |s| date(s));
    let (moved, times) = moved_schools(&w);
    if times != 1.0 {
        debug::rich(moved.clone(), times);
    }
    // Where every college player is today: a man somewhere else on the last
    // date came through the portal.
    let start: std::collections::HashMap<u32, usize> =
        w.people.iter().filter(|p| p.stage == Stage::College).filter_map(|p| Some((p.player.id, p.college?))).collect();
    let t = std::time::Instant::now();
    let to = |w: &mut World, day: Date, name: &str| {
        while w.today < day {
            w.advance_day();
        }
        eprintln!("{} at {:.0}s", w.today.pretty(), t.elapsed().as_secs_f64());
        if save && !name.is_empty() {
            let file = out.join(format!("{name}.afmworld"));
            afm_engine::save::write_world(w, &file, "nil_pull", "nil_pull", None).expect("write");
        }
    };
    to(&mut w, first, "recruits");
    let class = desk::next_class(&w);
    let picks: Vec<Pick> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.stage == Stage::HighSchool && p.class_of == class && p.cal.signed.is_none())
        .filter_map(|(i, p)| {
            let schools = list_of(p);
            if schools.len() < 2 {
                return None;
            }
            let appeals = debug::recruit_appeals(&w, i, &schools, None);
            let money: Vec<f64> = schools.iter().map(|&j| debug::recruit_money(&w, j, i)).collect();
            let leader = (0..schools.len()).max_by(|&a, &b| appeals[a].total_cmp(&appeals[b]))?;
            Some(Pick {
                id: p.player.id,
                stars: p.stars,
                leader: schools[leader],
                rich: richest(&money).map(|(n, x)| (schools[n], x)),
                commit: p.cal.commit.map(|c| c.0),
                money_rank: money_place(&desk::reason_weights(&w, i)),
                moved: schools.iter().copied().filter(|j| moved.contains(j)).collect(),
            })
        })
        .collect();
    to(&mut w, second, "portal");
    let shoppers: Vec<Shopper> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.stage == Stage::College && p.cal.portal.is_some_and(|e| !e.held && e.entered <= w.today))
        .filter_map(|(i, p)| {
            let race = debug::portal_race_with(&w, i, None);
            let from = p.college?;
            let money: Vec<f64> = race.iter().map(|r| r.2).collect();
            Some(Shopper {
                id: p.player.id,
                from,
                division: w.programs[from].division,
                first: race.first()?.0,
                rich: richest(&money).map(|(n, x)| (race[n].0, x)),
                money_rank: money_place(&debug::transfer_weights(&w, i)),
            })
        })
        .collect();
    to(&mut w, last, "");
    outcome(&w, class, &picks, &shoppers);
    if !moved.is_empty() {
        moved_outcome(&w, class, &picks, &start, &moved, times);
    }
}

/// What the moved schools signed and who came through the portal since
/// the start (`start`: where every college player was then).
fn moved_outcome(
    w: &World,
    class: i32,
    picks: &[Pick],
    start: &std::collections::HashMap<u32, usize>,
    moved: &[usize],
    times: f64,
) {
    use afm_engine::world::data::Division;
    let at: std::collections::HashMap<u32, usize> =
        w.people.iter().enumerate().map(|(i, p)| (p.player.id, i)).collect();
    let went = |id: u32| at.get(&id).and_then(|&i| w.people[i].cal.signed.map(|s| s.program).or(w.people[i].college));
    // Every FBS class of the year by the services' score, best first.
    let score = |j: usize| w.programs[j].class_scores.iter().rev().find(|c| c.0 == class).map_or(0.0, |c| c.1);
    let mut ranked: Vec<usize> = (0..w.programs.len()).filter(|&j| w.programs[j].division == Division::FBS).collect();
    ranked.sort_by(|&a, &b| score(b).total_cmp(&score(a)).then(a.cmp(&b)));
    println!("\n## The four schools whose money was multiplied by {times}");
    println!(
        "| school | signed | blue-chips | 3-stars | class rank in FBS | open recruits with it on their list on the first date | signed there | transfers in | from FBS | their overall |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|");
    let mut sum = [0usize; 7];
    let mut all = Vec::new();
    for &j in moved {
        let class_of: Vec<&afm_engine::world::Person> = w
            .people
            .iter()
            .filter(|p| p.class_of == class && p.cal.signed.map(|s| s.program).or(p.college) == Some(j))
            .collect();
        let open: Vec<&Pick> = picks.iter().filter(|p| p.commit.is_none() && p.moved.contains(&j)).collect();
        let came: Vec<(&afm_engine::world::Person, usize)> = w
            .people
            .iter()
            .filter(|p| p.stage == Stage::College && p.college == Some(j))
            .filter_map(|p| start.get(&p.player.id).filter(|&&c| c != j).map(|&c| (p, c)))
            .collect();
        let overall: Vec<f64> = came.iter().map(|x| x.0.player.overall()).collect();
        let row = [
            class_of.len(),
            class_of.iter().filter(|p| p.stars >= 4).count(),
            class_of.iter().filter(|p| p.stars == 3).count(),
            open.len(),
            open.iter().filter(|p| went(p.id) == Some(j)).count(),
            came.len(),
            came.iter().filter(|x| w.programs[x.1].division == Division::FBS).count(),
        ];
        for (a, b) in sum.iter_mut().zip(row) {
            *a += b;
        }
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.1} |",
            w.program_name(j),
            row[0],
            row[1],
            row[2],
            ranked.iter().position(|&x| x == j).map_or(0, |n| n + 1),
            row[3],
            row[4],
            row[5],
            row[6],
            mean(overall.iter().copied())
        );
        all.extend(overall);
    }
    println!(
        "| all four | {} | {} | {} | | {} | {} | {} | {} | {:.1} |",
        sum[0],
        sum[1],
        sum[2],
        sum[3],
        sum[4],
        sum[5],
        sum[6],
        mean(all.iter().copied())
    );
}

/// Where the recruits and entrants of the two dates went by the last one.
fn outcome(w: &World, class: i32, picks: &[Pick], shoppers: &[Shopper]) {
    use afm_engine::world::data::Division;
    let at: std::collections::HashMap<u32, usize> =
        w.people.iter().enumerate().map(|(i, p)| (p.player.id, i)).collect();
    let pct = |n: usize, of: usize| 100.0 * n as f64 / of.max(1) as f64;
    println!("# Where they went by {}", w.today.pretty());
    println!("\n## The class of {class}: recruits with two or more schools after them on the first date");
    println!("| who | recruits | signed | with the most money | with his leader then | flipped from his commitment |");
    println!("|---|---|---|---|---|---|");
    let row = |name: &str, f: &dyn Fn(&Pick) -> bool| {
        let v: Vec<&Pick> = picks.iter().filter(|p| f(p)).collect();
        let signed: Vec<(&Pick, usize)> = v
            .iter()
            .filter_map(|p| {
                let x = &w.people[*at.get(&p.id)?];
                Some((*p, x.cal.signed.map(|s| s.program).or(x.college)?))
            })
            .collect();
        let rich = signed.iter().filter(|(p, s)| p.rich.is_some_and(|r| r.0 == *s)).count();
        let had_rich = signed.iter().filter(|(p, _)| p.rich.is_some()).count();
        let led = signed.iter().filter(|(p, s)| p.leader == *s).count();
        let held = signed.iter().filter(|(p, _)| p.commit.is_some()).count();
        let flipped = signed.iter().filter(|(p, s)| p.commit.is_some_and(|c| c != *s)).count();
        println!(
            "| {name} | {} | {} | {:.0}% of {had_rich} | {:.0}% | {:.0}% of {held} |",
            v.len(),
            signed.len(),
            pct(rich, had_rich),
            pct(led, signed.len()),
            pct(flipped, held)
        );
    };
    row("everyone", &|_| true);
    row("5-stars", &|p| p.stars == 5);
    row("4-stars", &|p| p.stars == 4);
    row("3-stars", &|p| p.stars == 3);
    row("money is his first reason", &|p| p.money_rank == 0);
    row("money is his second or third", &|p| (1..=2).contains(&p.money_rank));
    row("money is fourth or lower", &|p| p.money_rank >= 3);
    row("the most money is 1.5 times the next or more", &|p| p.rich.is_some_and(|r| r.1 >= 1.5));
    row("the most money is under 1.2 times the next", &|p| p.rich.is_some_and(|r| r.1 < 1.2));
    // The blue-chips by conference, and the biggest classes of them.
    let mut by_conf: std::collections::BTreeMap<String, usize> = Default::default();
    let mut by_prog: std::collections::HashMap<usize, usize> = Default::default();
    let mut blue = 0usize;
    for p in w.people.iter().filter(|p| p.class_of == class && p.stars >= 4) {
        if let Some(j) = p.cal.signed.map(|s| s.program).or(p.college) {
            blue += 1;
            *by_conf.entry(w.programs[j].conference.clone()).or_default() += 1;
            *by_prog.entry(j).or_default() += 1;
        }
    }
    let mut conf: Vec<(String, usize)> = by_conf.into_iter().collect();
    conf.sort_by_key(|c| std::cmp::Reverse(c.1));
    println!(
        "\nBlue-chips (4 and 5 stars) signed: {blue}. By conference: {}.",
        conf.iter().take(8).map(|c| format!("{} {}", c.0, c.1)).collect::<Vec<_>>().join(", ")
    );
    let mut prog: Vec<(usize, usize)> = by_prog.into_iter().collect();
    prog.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    println!(
        "The ten biggest blue-chip classes hold {:.0}% of them: {}.",
        pct(prog.iter().take(10).map(|x| x.1).sum(), blue),
        prog.iter().take(10).map(|x| format!("{} {}", w.program_name(x.0), x.1)).collect::<Vec<_>>().join(", ")
    );
    // The ten biggest collectives' share of the blue-chips.
    let mut rich: Vec<usize> = (0..w.programs.len()).collect();
    rich.sort_by(|&a, &b| w.programs[b].collective.budget.total_cmp(&w.programs[a].collective.budget));
    let top: std::collections::HashSet<usize> = rich.into_iter().take(10).collect();
    println!(
        "The ten biggest collectives signed {:.0}% of them.",
        pct(prog.iter().filter(|x| top.contains(&x.0)).map(|x| x.1).sum(), blue)
    );

    println!("\n## The portal: entrants shopping on the second date");
    println!(
        "| who | entrants | landed | at the most money | at his first choice then | at FBS | at FCS | lower | back or still looking | left football |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|");
    let row = |name: &str, f: &dyn Fn(&Shopper) -> bool| {
        let v: Vec<&Shopper> = shoppers.iter().filter(|s| f(s)).collect();
        let (mut landed, mut rich, mut had_rich, mut first, mut stay, mut gone) = (0, 0, 0, 0, 0, 0);
        let mut level = [0usize; 3];
        for s in &v {
            let Some(x) = at.get(&s.id).map(|&i| &w.people[i]) else {
                gone += 1;
                continue;
            };
            match x.college {
                Some(j) if j != s.from && x.stage == Stage::College => {
                    landed += 1;
                    had_rich += s.rich.is_some() as usize;
                    rich += s.rich.is_some_and(|r| r.0 == j) as usize;
                    first += (s.first == j) as usize;
                    level[match w.programs[j].division {
                        Division::FBS => 0,
                        Division::FCS => 1,
                        _ => 2,
                    }] += 1;
                }
                _ if x.stage == Stage::College => stay += 1,
                _ => gone += 1,
            }
        }
        println!(
            "| {name} | {} | {landed} | {:.0}% of {had_rich} | {:.0}% | {:.0}% | {:.0}% | {:.0}% | {stay} | {gone} |",
            v.len(),
            pct(rich, had_rich),
            pct(first, landed),
            pct(level[0], landed),
            pct(level[1], landed),
            pct(level[2], landed),
        );
    };
    row("everyone", &|_| true);
    row("from FBS", &|s| s.division == Division::FBS);
    row("from FCS", &|s| s.division == Division::FCS);
    row("from below FCS", &|s| !matches!(s.division, Division::FBS | Division::FCS));
    row("money among his first three reasons", &|s| s.money_rank <= 2);
    row("money fourth or lower", &|s| s.money_rank >= 3);
    row("the most money is 1.5 times the next or more", &|s| s.rich.is_some_and(|r| r.1 >= 1.5));
}

/// Whether a program is one of the schools whose money the check moves.
fn picked(w: &World, j: usize, conference: &str) -> bool {
    let p = &w.programs[j];
    if conference == "power" { p.power_conference(w.year) } else { p.conference == conference }
}

/// A school's place among `rows` (1: first) and its appeal less the best
/// other school's.
fn place(appeals: &[f64], at: usize) -> (usize, f64) {
    let others = || appeals.iter().enumerate().filter(move |x| x.0 != at).map(|x| *x.1);
    (1 + others().filter(|&a| a > appeals[at]).count(), appeals[at] - others().fold(f64::NEG_INFINITY, f64::max))
}

/// One (player, school) pair: what he weighs, and the school's place on his
/// list at each multiple of the field's best money.
struct Pair {
    /// Money's place among his six reasons (0: first).
    money_rank: usize,
    /// Money's own weight in his choice.
    money_w: f64,
    stars: u8,
    committed: bool,
    /// The role a transfer would have there.
    role: Option<PlayingTime>,
    list: usize,
    /// The field's best money, and the school's own today.
    field: f64,
    own: f64,
    /// The school's place today, at its own money.
    place: usize,
    /// Its place at each of `TIMES`.
    at: [usize; 5],
    /// Its appeal less the best other school's, at each of `TIMES`.
    gap: [f64; 5],
    /// The spread (standard deviation) of the list's appeals.
    spread: f64,
}

fn money_place(weights: &[f64; 6]) -> usize {
    weights.iter().filter(|&&x| x > weights[2]).count()
}

fn sd(v: &[f64]) -> f64 {
    let n = v.len().max(1) as f64;
    let m = v.iter().sum::<f64>() / n;
    (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / n).sqrt()
}

fn recruits(w: &World, conference: &str) {
    let class = desk::next_class(w);
    println!("# Recruits of the class of {class} on {} ({conference} schools' money moved)", w.today.pretty());
    let mut pairs: Vec<Pair> = Vec::new();
    // A school not on his list yet, coming in with an offer (three-stars up).
    let mut fresh: Vec<Pair> = Vec::new();
    // NIL Go's top for a collective's deal with him over the field's best money, by stars.
    let mut clears: [Vec<f64>; 6] = Default::default();
    let (mut lists, mut sizes) = (0usize, 0usize);
    // Uncommitted recruits whose leader is the school with the most money
    // on his list: (recruits, leaders), everyone and the money-first.
    let mut led = [[0usize; 2]; 2];
    for (i, p) in w.people.iter().enumerate() {
        if p.stage != Stage::HighSchool || p.class_of != class || p.cal.signed.is_some() {
            continue;
        }
        let schools = list_of(p);
        if schools.len() < 5 {
            continue;
        }
        lists += 1;
        sizes += schools.len();
        // (appeal, money) of every school on his list today.
        let money: Vec<f64> = schools.iter().map(|&j| debug::recruit_money(w, j, i)).collect();
        let appeals = debug::recruit_appeals(w, i, &schools, None);
        let weights = desk::reason_weights(w, i);
        let spread = sd(&appeals);
        if p.cal.commit.is_none()
            && let Some((rich, _)) = richest(&money)
        {
            let leader = (0..schools.len()).max_by(|&a, &b| appeals[a].total_cmp(&appeals[b]));
            for row in led.iter_mut().take(1 + (money_place(&weights) == 0) as usize) {
                row[0] += 1;
                row[1] += (leader == Some(rich)) as usize;
            }
        }
        // One school's money moved: `n` its place on his list (the list's
        // length for a new school), `own` its money today.
        let moved = |j: usize, n: usize, own: f64, to: &mut Vec<Pair>| {
            let mut list = schools.clone();
            if n == list.len() {
                list.push(j);
            }
            let field = money.iter().enumerate().filter(|x| x.0 != n).map(|x| *x.1).fold(0.0, f64::max);
            if field < 1_000.0 {
                return;
            }
            // Every school on his list is read again against the money moved.
            let today = place(&debug::recruit_appeals(w, i, &list, Some((j, own))), n).0;
            let mut at = [0; 5];
            let mut gap = [0.0; 5];
            for (k, t) in TIMES.iter().enumerate() {
                (at[k], gap[k]) = place(&debug::recruit_appeals(w, i, &list, Some((j, t * field))), n);
            }
            to.push(Pair {
                money_rank: money_place(&weights),
                money_w: weights[2],
                stars: p.stars,
                committed: p.cal.commit.is_some(),
                role: None,
                list: list.len(),
                field,
                own,
                place: today,
                at,
                gap,
                spread,
            });
        };
        for (n, &j) in schools.iter().enumerate() {
            if picked(w, j, conference) && p.cal.commit.is_none_or(|c| c.0 != j) {
                moved(j, n, money[n], &mut pairs);
            }
        }
        if p.stars >= 3 {
            for j in (0..w.programs.len()).filter(|&j| picked(w, j, conference) && !schools.contains(&j)) {
                moved(j, schools.len(), debug::recruit_money(w, j, i), &mut fresh);
            }
            let field = money.iter().copied().fold(0.0, f64::max);
            if field >= 1_000.0 {
                clears[p.stars.min(5) as usize].push(debug::recruit_nil_go_top(w, i) / field);
            }
        }
    }
    println!(
        "{lists} recruits with five or more schools after them (a list of {:.1} on average); {} pairs with a {conference} school",
        sizes as f64 / lists.max(1) as f64,
        pairs.len()
    );
    for (name, v) in [("uncommitted recruits", led[0]), ("uncommitted recruits who put money first", led[1])] {
        println!(
            "The school with the most money leads {:.0}% of {} {name}' lists",
            100.0 * v[1] as f64 / v[0].max(1) as f64,
            v[0]
        );
    }
    for s in [5usize, 4, 3] {
        println!(
            "NIL Go's top for a collective's deal with a {s}-star, over the best money on his list: median {:.2} ({} recruits)",
            median(&mut clears[s]),
            clears[s].len()
        );
    }
    report(&pairs);
    println!(
        "\n# A new offer: {conference} schools not on his list yet, three-stars and better ({} pairs)",
        fresh.len()
    );
    report(&fresh);
}

fn portal(w: &World, conference: &str, most: usize) {
    use afm_engine::world::data::Division;
    println!("# Portal entrants on {} ({conference} schools' money moved)", w.today.pretty());
    let entrants: Vec<usize> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.stage == Stage::College && p.cal.portal.is_some_and(|e| !e.held && e.entered <= w.today))
        .map(|(i, _)| i)
        .collect();
    let mut pairs: Vec<Pair> = Vec::new();
    let (mut lists, mut sizes, mut seen) = (0usize, 0usize, 0usize);
    // Entrants with a picked program after them: money first, and among
    // the first three, of his six reasons.
    let mut cares = [0usize; 2];
    // What a transfer weighs money by over the recruit he was: from FBS,
    // from below it, and those with a picked program after them.
    let mut more: [Vec<f64>; 3] = Default::default();
    // Entrants whose first choice today pays the most: (entrants, first choices).
    let mut led = [0usize; 2];
    for &i in &entrants {
        let base = debug::portal_race_with(w, i, None);
        if base.len() < 3 {
            continue;
        }
        lists += 1;
        sizes += base.len();
        let p = &w.people[i];
        let m = debug::transfer_money(w, i).unwrap_or(1.0);
        let fbs = p.college.is_some_and(|c| w.programs[c].division == Division::FBS);
        more[!fbs as usize].push(m);
        if let Some((rich, _)) = richest(&base.iter().map(|b| b.2).collect::<Vec<_>>()) {
            led[0] += 1;
            led[1] += (rich == 0) as usize;
        }
        if !base.iter().any(|b| picked(w, b.0, conference)) {
            continue;
        }
        seen += 1;
        if seen > most {
            break;
        }
        more[2].push(m);
        let weights = debug::transfer_weights(w, i);
        cares[0] += (money_place(&weights) == 0) as usize;
        cares[1] += (money_place(&weights) <= 2) as usize;
        let spread = sd(&base.iter().map(|b| b.1).collect::<Vec<_>>());
        let appeals = |race: &[(usize, f64, f64, PlayingTime)]| race.iter().map(|r| r.1).collect::<Vec<_>>();
        let today = appeals(&base);
        for (n, &(j, _, own, role)) in base.iter().enumerate() {
            if !picked(w, j, conference) {
                continue;
            }
            let field = base.iter().filter(|b| b.0 != j).map(|b| b.2).fold(0.0, f64::max);
            if field < 1_000.0 {
                continue;
            }
            let mut at = [0; 5];
            let mut gap = [0.0; 5];
            for (k, t) in TIMES.iter().enumerate() {
                let race = debug::portal_race_with(w, i, Some((j, t * field)));
                let at_j = race.iter().position(|r| r.0 == j).expect("still after him");
                (at[k], gap[k]) = place(&appeals(&race), at_j);
            }
            pairs.push(Pair {
                money_rank: money_place(&weights),
                money_w: weights[2],
                stars: p.stars,
                committed: false,
                role: Some(role),
                list: base.len(),
                field,
                own,
                place: place(&today, n).0,
                at,
                gap,
                spread,
            });
        }
    }
    println!(
        "{} entrants shopping, {lists} with three or more programs after them (a list of {:.1} on average); {} pairs with a {conference} program",
        entrants.len(),
        sizes as f64 / lists.max(1) as f64,
        pairs.len()
    );
    let pct = |n: usize, of: usize| 100.0 * n as f64 / of.max(1) as f64;
    println!(
        "Of those with a {conference} program after them, money is the first reason for {:.0}% and among the first three for {:.0}%",
        pct(cares[0], seen.min(most)),
        pct(cares[1], seen.min(most)),
    );
    println!(
        "The program that pays the most is the first choice today of {:.0}% of {} entrants",
        pct(led[1], led[0]),
        led[0]
    );
    let names =
        ["from FBS".to_string(), "from below FBS".to_string(), format!("with a {conference} program after them")];
    for (name, v) in names.iter().zip(more.iter_mut()) {
        if v.is_empty() {
            continue;
        }
        let n = v.len();
        let mean = v.iter().sum::<f64>() / n as f64;
        v.sort_by(|a, b| a.total_cmp(b));
        println!(
            "What a transfer weighs money by over the recruit he was, {name}: {mean:.2} on average ({n} entrants; a tenth under {:.2}, half under {:.2}, a tenth over {:.2})",
            v[n / 10],
            v[n / 2],
            v[n - 1 - n / 10]
        );
    }
    report(&pairs);
    println!("\n# By the role he'd have there");
    let all: Vec<&Pair> = pairs.iter().collect();
    for (name, role) in [("start", PlayingTime::Starter), ("be in the rotation", PlayingTime::Rotation)] {
        let v: Vec<&Pair> = all.iter().copied().filter(|p| p.role == Some(role)).collect();
        table(&format!("He would {name}"), &v);
        table(
            &format!("He would {name}, and money is among his first three reasons"),
            &v.iter().copied().filter(|p| p.money_rank <= 2).collect::<Vec<_>>(),
        );
    }
    table(
        "He would be a backup",
        &all.iter()
            .copied()
            .filter(|p| !matches!(p.role, Some(PlayingTime::Starter | PlayingTime::Rotation)))
            .collect::<Vec<_>>(),
    );
}

fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

fn mean(v: impl Iterator<Item = f64>) -> f64 {
    let (mut s, mut n) = (0.0, 0usize);
    for x in v {
        s += x;
        n += 1;
    }
    if n == 0 { f64::NAN } else { s / n as f64 }
}

fn table(name: &str, v: &[&Pair]) {
    if v.is_empty() {
        println!("\n## {name}: none");
        return;
    }
    let n = v.len();
    println!(
        "\n## {name}: {n} pairs; list {:.1}; the field's best money ${:.0}K (median), the school's own ${:.0}K; money's weight {:.3}; the list's spread {:.3}",
        mean(v.iter().map(|p| p.list as f64)),
        median(&mut v.iter().map(|p| p.field / 1e3).collect::<Vec<_>>()),
        median(&mut v.iter().map(|p| p.own / 1e3).collect::<Vec<_>>()),
        mean(v.iter().map(|p| p.money_w)),
        mean(v.iter().map(|p| p.spread)),
    );
    println!("| money | median place | mean place | first | top 3 | top 5 | top 10 | behind the leader |");
    println!("|---|---|---|---|---|---|---|---|");
    let share = |f: &dyn Fn(&Pair) -> bool| 100.0 * v.iter().filter(|p| f(p)).count() as f64 / n as f64;
    let row = |label: &str, at: &dyn Fn(&Pair) -> usize, gap: Option<usize>| {
        println!(
            "| {label} | {:.0} | {:.1} | {:.0}% | {:.0}% | {:.0}% | {:.0}% | {} |",
            median(&mut v.iter().map(|p| at(p) as f64).collect::<Vec<_>>()),
            mean(v.iter().map(|p| at(p) as f64)),
            share(&|p| at(p) == 1),
            share(&|p| at(p) <= 3),
            share(&|p| at(p) <= 5),
            share(&|p| at(p) <= 10),
            gap.map_or("".into(), |k| format!("{:+.3}", mean(v.iter().map(|p| p.gap[k])))),
        );
    };
    row("its own today", &|p| p.place, None);
    for (k, t) in TIMES.iter().enumerate() {
        row(&format!("{t}x the field"), &|p| p.at[k], Some(k));
    }
}

fn report(pairs: &[Pair]) {
    let all: Vec<&Pair> = pairs.iter().collect();
    table("Everyone", &all);
    let open: Vec<&Pair> = pairs.iter().filter(|p| !p.committed).collect();
    if open.len() != all.len() {
        table("Not committed anywhere", &open);
        table("Committed to another school", &pairs.iter().filter(|p| p.committed).collect::<Vec<_>>());
    }
    println!("\n# By what he weighs (not committed anywhere)");
    table("Money is his first reason", &open.iter().copied().filter(|p| p.money_rank == 0).collect::<Vec<_>>());
    table(
        "Money is his second or third reason",
        &open.iter().copied().filter(|p| (1..=2).contains(&p.money_rank)).collect::<Vec<_>>(),
    );
    table("Money is fourth or lower", &open.iter().copied().filter(|p| p.money_rank >= 3).collect::<Vec<_>>());
    println!("\n# By stars (not committed anywhere)");
    for s in [5u8, 4, 3] {
        table(&format!("{s}-stars"), &open.iter().copied().filter(|p| p.stars == s).collect::<Vec<_>>());
    }
    table("2-stars and lower", &open.iter().copied().filter(|p| p.stars <= 2).collect::<Vec<_>>());
    // Cain's case: a player with many schools after him.
    table("Lists of 15 schools or more", &open.iter().copied().filter(|p| p.list >= 15).collect::<Vec<_>>());
}
