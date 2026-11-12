//! The freak check (Cain 2026-11-08: "be careful to not cap the game
//! creating rare freaks"; `afm/ab-tests/research-edge.md`, "No caps on
//! freaks"): do undersized edges hold their real share of the top pressure
//! seasons, do the best seasons reach real's and no further, and does a 6-0,
//! 240 lb edge with top Strength, Agility and Finesse Rush rush with the
//! league's best?
//!
//! Usage: freak_check [seasons] [seed | first-last] [synthetic | drawn | world file]
//! `synthetic` (the default) plays the synthetic NFL league as it comes, its
//! edges' bodies from the real combine (the rosters every check uses).
//! `drawn` redraws every edge with the game's own generator at his overall
//! (athlete-first, natural edges at 14 kept whatever they grow into, so it
//! has men the world's coaches would move inside or off the ball). A world
//! file plays that world's 32 NFL clubs. Each season is 17 games a team.
//! The rosters are built once a seed and replayed every season, so one
//! seed's top seasons are a few dozen men: a range of seeds (`86-95`) pools
//! them and gives each share a 95% band from resampling the men (edge lane
//! 2, minor bug 359: seeds 82 and 83 alone showed tall men at 5-12% of the
//! top, twelve seeds 22% of the top against 23% of all).
//! `AFM_FREAK_DUMP=file.csv` writes one row per edge season.
//!
//! Pressures are PFR's (hurries, knockdowns and sacks: every sack and hit
//! is his pressure), so are the shares of the top 10%. Real (PFR, nflverse
//! snap counts and play by play, edges with 200+ defense snaps, 2018-2025,
//! 983 seasons from PFR's season totals, so a traded man's stints add up):
//! of the top 10% by pressures a defense snap, under 250 lb 15% (18% of
//! all), arms under 33 in 41% (26%; the 679 with a measured arm), 6-5 or
//! taller 19% (28%; real heights are whole inches, so 6-5 is 76.5 in and
//! up), 290 lb or more none (0.6%); the real men average 260 lb (sd 12)
//! and 75.7 in; sacks a pressure 27.4%; a season's sacks at
//! the top 1%, 5%, 10% and 50% 17.5, 12.5, 10.5 and 4.0; 2 seasons of 20+
//! (T.J. Watt 2021, 22.5, and Myles Garrett 2025, 23); the top 1% of sacks
//! a defense snap 2.40%; the top 1% and 10% of sack seasons play 77% and
//! 74% of their team's defense snaps; 10.8% of sacks are shared.

extern crate afm_tools;
use std::collections::HashMap;

use afm_engine::Rng;
use afm_engine::attributes::Attr;
use afm_engine::game::stats::PlayType;
use afm_engine::game::{Conditions, Venue, quick_game_in};
use afm_engine::player::Level;
use afm_engine::positions::{Position, overall_for};
use afm_tools::ab::{Arm, Knob, Part, Who, change_one, synthetic, teams_for, world_nfl};
use rayon::prelude::*;

#[derive(Default, Clone)]
struct Season {
    snaps: u32,
    /// His team's defense snaps that season (every game, his or not).
    team: u32,
    rush: u32,
    /// The stat line's pressures: the men credited at the first moment.
    pressures: u32,
    hurries: u32,
    hits: u32,
    sacks: f64,
    /// Plays he is in on a sack: a shared sack is half a sack but a whole
    /// pressure to each man, as PFR counts it.
    sack_plays: u32,
    /// Rush wins by move: speed, power, finesse.
    wins: [u32; 3],
}

impl Season {
    fn add(&mut self, x: &Season) {
        self.snaps += x.snaps;
        self.team += x.team;
        self.rush += x.rush;
        self.pressures += x.pressures;
        self.hurries += x.hurries;
        self.hits += x.hits;
        self.sacks += x.sacks;
        self.sack_plays += x.sack_plays;
        for k in 0..3 {
            self.wins[k] += x.wins[k];
        }
    }
    /// PFR's pressures: hurries, knockdowns and the sacks he is in on.
    fn pfr(&self) -> f64 {
        self.hurries as f64 + self.hits as f64 + self.sack_plays as f64
    }
    fn rate(&self) -> f64 {
        self.pfr() / self.snaps.max(1) as f64
    }
}

/// One edge season: the seed, his id, the season, his counts, his body
/// (height, weight, arm), what he brings (Strength, Power Rush, Finesse
/// Rush, Hand Use, Acceleration, Agility, Flexibility, Speed, Tackling) and
/// whether he is the freak.
#[derive(Clone)]
struct Row {
    seed: u32,
    id: u32,
    season: usize,
    x: Season,
    body: (f64, f64, f64),
    rat: [f64; 9],
    freak: bool,
}

/// One seed's league: its edge seasons with 200+ defense snaps, its sacks
/// and the shared ones.
struct Run {
    rows: Vec<Row>,
    sack_plays: u32,
    shared: u32,
    /// Every edge's and tackle's season with 200+ defense snaps: his share
    /// of his team's (the line's rotation).
    line: Vec<(Position, f64)>,
}

/// One game: each man's counts, each side's team and defense snaps, its
/// sacks and the shared ones.
type Game = (HashMap<u32, Season>, [(usize, u32); 2], u32, u32);

fn league(seasons: usize, seed: u32, mode: &str) -> Run {
    let level = Level::NFL;
    let world = (mode != "synthetic" && mode != "drawn").then_some(mode);
    let base = match world {
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
        match mode {
            "synthetic" => "synthetic league",
            "drawn" => "synthetic league, generator edges",
            _ => "world rosters",
        },
        teams[0].players[k].height,
        teams[0].players[k].weight,
        teams[0].players[k].arm
    );
    let n = teams.len();
    let body: HashMap<u32, (f64, f64, f64, Position, usize)> = teams
        .iter()
        .enumerate()
        .flat_map(|(ti, t)| t.players.iter().map(move |p| (p.id, (p.height, p.weight, p.arm, p.position, ti))))
        .collect();
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
    let venue =
        || Venue { cond: Conditions { crowd: 0.65, ..Conditions::default() }, wear: [0.0, 0.012], ..Venue::default() };
    let mut rows = Vec::new();
    let mut line = Vec::new();
    let (mut sack_plays, mut shared) = (0, 0);
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
        let played: Vec<Game> = games
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
                for players in &r.stats.players {
                    for (&id, l) in players {
                        let e: &mut Season = m.entry(id).or_default();
                        e.add(&Season {
                            snaps: l.snaps_def,
                            team: 0,
                            rush: l.rush_snaps,
                            pressures: l.pressures,
                            hurries: l.hurries,
                            hits: l.qb_hits,
                            sacks: l.sacks,
                            sack_plays: 0,
                            wins: [l.speed_wins, l.power_wins, l.finesse_wins],
                        });
                    }
                }
                // Each side's defense snaps, counted as a man's are (every
                // scrimmage play, wiped ones too).
                let mut def = [0u32; 2];
                let (mut sk, mut sh) = (0, 0);
                for p in &r.stats.plays {
                    if matches!(p.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble) {
                        def[1 - p.offense] += 1;
                    }
                    if !p.sackers.is_empty() && !p.nullified {
                        sk += 1;
                        sh += (p.sackers.len() > 1) as u32;
                        // The stat line credits two sackers at most.
                        for &id in p.sackers.iter().take(2) {
                            m.entry(id).or_default().sack_plays += 1;
                        }
                    }
                }
                (m, [(h, def[0]), (aw, def[1])], sk, sh)
            })
            .collect();
        let mut tot: HashMap<u32, Season> = HashMap::new();
        let mut team = vec![0u32; n];
        for (m, d, sk, sh) in played {
            for (id, x) in m {
                tot.entry(id).or_default().add(&x);
            }
            for (t, k) in d {
                team[t] += k;
            }
            sack_plays += sk;
            shared += sh;
        }
        for (id, mut x) in tot {
            let Some(&(h, w, arm, pos, ti)) = body.get(&id) else { continue };
            if matches!(pos, Position::EDGE | Position::DT) && x.snaps >= 200 {
                line.push((pos, x.snaps as f64 / team[ti].max(1) as f64));
            }
            if pos != Position::EDGE || x.snaps < 200 {
                continue;
            }
            x.team = team[ti];
            rows.push(Row {
                seed,
                id,
                season: s,
                x,
                body: (h, w, arm),
                rat: rat.get(&id).copied().unwrap_or_default(),
                freak: id == freak,
            });
        }
    }
    rows.sort_by_key(|r| (r.season, r.id));
    Run { rows, sack_plays, shared, line }
}

fn who(r: &Row) -> String {
    let (x, q) = (&r.x, &r.rat);
    let w = x.wins.iter().sum::<u32>().max(1) as f64;
    format!(
        "Str {:.0} PR {:.0} FR {:.0} HU {:.0} Acc {:.0} Agi {:.0} Flex {:.0} Spd {:.0} Tk {:.0}; {:.0}% of team snaps; wins {:.0}/{:.0}/{:.0}% speed/power/finesse",
        q[0],
        q[1],
        q[2],
        q[3],
        q[4],
        q[5],
        q[6],
        q[7],
        q[8],
        100.0 * x.snaps as f64 / x.team.max(1) as f64,
        100.0 * x.wins[0] as f64 / w,
        100.0 * x.wins[1] as f64 / w,
        100.0 * x.wins[2] as f64 / w
    )
}

type Group = (&'static str, fn(&(f64, f64, f64)) -> bool, &'static str);
const GROUPS: [Group; 5] = [
    ("under 250 lb", |b| b.1 < 250.0, "15% / 18%"),
    ("arms under 33 in", |b| b.2 < 33.0, "41% / 26%"),
    ("6-5 or taller (76.5 in)", |b| b.0 >= 76.5, "19% / 28%"),
    ("77 in or taller", |b| b.0 >= 77.0, "19% / 28%"),
    ("290 lb or more", |b| b.1 >= 290.0, "0% / 0.6%"),
];

/// The bars on a set of natural men's seasons (the freak left out).
struct Bars {
    n: usize,
    /// Each group's share of the top 10% by PFR pressures a defense snap,
    /// and of all.
    groups: Vec<(f64, f64)>,
    /// A season's sacks at the top 1%, 5%, 10% and 50%.
    sacks: [f64; 4],
    /// Seasons of 20 or more sacks a 1,000.
    twenty: f64,
    /// The top 1% of sacks a defense snap.
    p99_rate: f64,
    /// The top 1% and 10% of sack seasons' share of their team's defense
    /// snaps.
    snaps: (f64, f64),
}

fn bars(v: &[&Row]) -> Bars {
    let n = v.len();
    let mut by_rate: Vec<&Row> = v.to_vec();
    by_rate.sort_by(|p, q| q.x.rate().total_cmp(&p.x.rate()));
    let top = ((n as f64 * 0.10).round() as usize).clamp(1, n.max(1));
    let share = |s: &[&Row], f: fn(&(f64, f64, f64)) -> bool| {
        100.0 * s.iter().filter(|r| f(&r.body)).count() as f64 / s.len().max(1) as f64
    };
    let groups = GROUPS.iter().map(|g| (share(&by_rate[..top.min(n)], g.1), share(&by_rate, g.1))).collect();
    let mut by_sacks: Vec<&Row> = v.to_vec();
    by_sacks.sort_by(|p, q| q.x.sacks.total_cmp(&p.x.sacks));
    let q = |p: f64| by_sacks.get(((n as f64 * p) as usize).min(n.saturating_sub(1))).map_or(0.0, |r| r.x.sacks);
    let twenty = 1000.0 * v.iter().filter(|r| r.x.sacks >= 20.0).count() as f64 / n.max(1) as f64;
    let mut rates: Vec<f64> = v.iter().map(|r| r.x.sacks / r.x.snaps.max(1) as f64).collect();
    rates.sort_by(|a, b| b.total_cmp(a));
    let p99_rate = rates.get(((n as f64 * 0.01) as usize).min(n.saturating_sub(1))).copied().unwrap_or(0.0);
    let snap_share = |k: usize| {
        let s = &by_sacks[..k.clamp(1, n.max(1)).min(n)];
        100.0 * s.iter().map(|r| r.x.snaps as f64 / r.x.team.max(1) as f64).sum::<f64>() / s.len().max(1) as f64
    };
    Bars {
        n,
        groups,
        sacks: [q(0.01), q(0.05), q(0.10), q(0.5)],
        twenty,
        p99_rate,
        snaps: (snap_share((n as f64 * 0.01).round() as usize), snap_share(top)),
    }
}

/// A 95% band from resampled values.
fn band(v: &[f64]) -> (f64, f64) {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    let at = |p: f64| s[((s.len() as f64 * p) as usize).min(s.len() - 1)];
    (at(0.025), at(0.975))
}

/// `same_men`: every seed replays the same men (a world file), so a man is
/// one unit in the resample however many seeds he plays.
fn report(rows: &[Row], sack_plays: u32, shared: u32, same_men: bool) {
    let natural: Vec<&Row> = rows.iter().filter(|r| !r.freak).collect();
    let all = natural.len();
    let sum = |f: &dyn Fn(&Season) -> f64| natural.iter().map(|r| f(&r.x)).sum::<f64>();
    let snaps = sum(&|x| x.snaps as f64);
    let w: Vec<f64> = natural.iter().map(|r| r.body.1).collect();
    let mw = w.iter().sum::<f64>() / all.max(1) as f64;
    let sw = (w.iter().map(|x| (x - mw).powi(2)).sum::<f64>() / all.max(1) as f64).sqrt();
    println!("Edge seasons with 200+ defense snaps, the freak left out: {all}; their weight {mw:.0} lb (sd {sw:.0})");
    println!(
        "  pressures a defense snap {:.2}% (PFR's count; the stat line's {:.2}%), sacks {:.2}%, sacks a pressure {:.1}% (the line's {:.1}%) (real 3.44%, 0.94%, 27.4%)",
        100.0 * sum(&|x| x.pfr()) / snaps,
        100.0 * sum(&|x| x.pressures as f64) / snaps,
        100.0 * sum(&|x| x.sacks) / snaps,
        100.0 * sum(&|x| x.sacks) / sum(&|x| x.pfr()).max(1.0),
        100.0 * sum(&|x| x.sacks) / sum(&|x| x.pressures as f64).max(1.0)
    );
    println!("  shared sacks {:.1}% of {} (real 10.8%)", 100.0 * shared as f64 / sack_plays.max(1) as f64, sack_plays);
    let b = bars(&natural);
    // Resample the men, each with all his seasons.
    let mut men: HashMap<(u32, u32), Vec<&Row>> = HashMap::new();
    for r in &natural {
        men.entry((if same_men { 0 } else { r.seed }, r.id)).or_default().push(r);
    }
    let mut keys: Vec<(u32, u32)> = men.keys().copied().collect();
    keys.sort();
    let mut rng = Rng::new(0xF4EA_u32);
    let boots: Vec<Bars> = (0..400)
        .map(|_| {
            let pick: Vec<&Row> = (0..keys.len())
                .flat_map(|_| men[&keys[rng.next_u32() as usize % keys.len()]].iter().copied())
                .collect();
            bars(&pick)
        })
        .collect();
    let bb = |f: &dyn Fn(&Bars) -> f64| band(&boots.iter().map(f).collect::<Vec<_>>());
    println!(
        "  {} men; the top 10% by PFR pressures a defense snap:  top 10%   all    top/all (95% band)   real top / all",
        keys.len()
    );
    for (k, g) in GROUPS.iter().enumerate() {
        let (lo, hi) = bb(&|x: &Bars| x.groups[k].0 / x.groups[k].1.max(1e-9));
        println!(
            "  {:26} {:5.1}%  {:5.1}%   {:4.2} ({:.2}-{:.2})   {}",
            g.0,
            b.groups[k].0,
            b.groups[k].1,
            b.groups[k].0 / b.groups[k].1.max(1e-9),
            lo,
            hi,
            g.2
        );
    }
    let (l1, h1) = bb(&|x: &Bars| x.sacks[0]);
    println!(
        "  sacks a season at the top 1%, 5%, 10%, 50%: {:.1} ({l1:.1}-{h1:.1}), {:.1}, {:.1}, {:.1} (real 17.5, 12.5, 10.5, 4.0)",
        b.sacks[0], b.sacks[1], b.sacks[2], b.sacks[3]
    );
    let (l2, h2) = bb(&|x: &Bars| x.twenty);
    println!("  seasons of 20+ sacks a 1,000: {:.1} ({l2:.1}-{h2:.1}) (real 2.0)", b.twenty);
    let (l3, h3) = bb(&|x: &Bars| 100.0 * x.p99_rate);
    println!("  sacks a defense snap at the top 1%: {:.2}% ({l3:.2}-{h3:.2}) (real 2.40%)", 100.0 * b.p99_rate);
    println!(
        "  the top 1% and 10% of sack seasons play {:.0}% and {:.0}% of their team's defense snaps (real 77% and 74%)",
        b.snaps.0, b.snaps.1
    );
    let mut by_sacks: Vec<&Row> = natural.clone();
    by_sacks.sort_by(|p, q| q.x.sacks.total_cmp(&p.x.sacks));
    println!("Best sack seasons (real best 22.5 and 23):");
    for r in by_sacks.iter().take(8) {
        println!(
            "  {:5.1} sacks, {:3.0} pressures, {:.1}% a snap  seed {} id {} season {} ({:.0} in, {:.0} lb, arm {:.2})",
            r.x.sacks,
            r.x.pfr(),
            100.0 * r.x.rate(),
            r.seed,
            r.id,
            r.season,
            r.body.0,
            r.body.1,
            r.body.2
        );
        println!("        {}", who(r));
    }
}

/// The freak's seasons among his seed's edges, by PFR pressures a defense
/// snap.
fn freak_lines(rows: &[Row]) {
    let mut by_rate: Vec<&Row> = rows.iter().collect();
    by_rate.sort_by(|p, q| q.x.rate().total_cmp(&p.x.rate()));
    let all = by_rate.len();
    let mine: Vec<(usize, &Row)> = by_rate.iter().enumerate().filter(|(_, r)| r.freak).map(|(k, r)| (k, *r)).collect();
    if mine.is_empty() {
        println!("The freak played under 200 snaps a season.");
        return;
    }
    let mut ranks: Vec<f64> = mine.iter().map(|(k, _)| 100.0 * (k + 1) as f64 / all as f64).collect();
    ranks.sort_by(|a, b| a.total_cmp(b));
    let mut sacks: Vec<f64> = mine.iter().map(|(_, r)| r.x.sacks).collect();
    sacks.sort_by(|a, b| a.total_cmp(b));
    println!(
        "The freak (seed {}): best season top {:.1}%, median top {:.1}%; {:.0}-{:.0} sacks a season (median {:.0}); {}",
        mine[0].1.seed,
        ranks[0],
        ranks[ranks.len() / 2],
        sacks[0],
        sacks[sacks.len() - 1],
        sacks[sacks.len() / 2],
        who(mine[0].1)
    );
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seasons: usize = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
    let spec = a.get(2).cloned().unwrap_or_else(|| "76".into());
    let seeds: Vec<u32> = match spec.split_once('-') {
        Some((x, y)) => (x.parse().expect("first seed")..=y.parse().expect("last seed")).collect(),
        None => vec![spec.parse().expect("seed")],
    };
    let mode = a.get(3).cloned().unwrap_or_else(|| "synthetic".into());
    let mut rows = Vec::new();
    let mut line = Vec::new();
    let (mut sack_plays, mut shared) = (0, 0);
    for &seed in &seeds {
        let r = league(seasons, seed, &mode);
        if seeds.len() > 1 {
            let nat: Vec<&Row> = r.rows.iter().filter(|x| !x.freak).collect();
            let b = bars(&nat);
            let best = nat.iter().map(|x| x.x.sacks).fold(0.0, f64::max);
            println!(
                "seed {seed}: {} seasons; 6-5+ top {:.1}% all {:.1}%; sacks top 1% {:.1}, best {:.1}; shared {:.1}%",
                b.n,
                b.groups[2].0,
                b.groups[2].1,
                b.sacks[0],
                best,
                100.0 * r.shared as f64 / r.sack_plays.max(1) as f64
            );
            freak_lines(&r.rows);
        }
        sack_plays += r.sack_plays;
        shared += r.shared;
        rows.extend(r.rows);
        line.extend(r.line);
    }
    if seeds.len() > 1 {
        println!("Pooled over seeds {spec}:");
    }
    report(&rows, sack_plays, shared, mode != "synthetic" && mode != "drawn");
    // The line's rotation: men with 200+ defense snaps a team season and
    // their share of the team's (real, nflverse snap counts 2018-2025:
    // edges 3.84 a team season, 47% at the median, 77% at the 90th; tackles
    // 4.35, 44%, 70%).
    let team_seasons = (seeds.len() * seasons * 32) as f64;
    for (pos, real) in [(Position::EDGE, "3.84, 47%, 77%"), (Position::DT, "4.35, 44%, 70%")] {
        let mut v: Vec<f64> = line.iter().filter(|x| x.0 == pos).map(|x| x.1).collect();
        v.sort_by(|a, b| a.total_cmp(b));
        let at = |p: f64| v.get(((v.len() as f64 * p) as usize).min(v.len().saturating_sub(1))).copied().unwrap_or(0.0);
        println!(
            "Line rotation, {pos:?}: {:.2} men with 200+ defense snaps a team season, {:.0}% of the team's at the median, {:.0}% at the 90th (real {real})",
            v.len() as f64 / team_seasons,
            100.0 * at(0.5),
            100.0 * at(0.9)
        );
    }
    if seeds.len() == 1 {
        freak_lines(&rows);
    }
    if let Ok(path) = std::env::var("AFM_FREAK_DUMP") {
        let mut out = String::from(
            "seed,id,season,freak,snaps,team_snaps,rush_snaps,pressures_line,hurries,hits,sacks,speed_wins,power_wins,finesse_wins,height,weight,arm,str,pr,fr,hu,acc,agi,flex,spd,tk\n",
        );
        for r in &rows {
            let x = &r.x;
            out += &format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.2},{:.1},{:.2},{}\n",
                r.seed,
                r.id,
                r.season,
                r.freak as u8,
                x.snaps,
                x.team,
                x.rush,
                x.pressures,
                x.hurries,
                x.hits,
                x.sacks,
                x.wins[0],
                x.wins[1],
                x.wins[2],
                r.body.0,
                r.body.1,
                r.body.2,
                r.rat.iter().map(|v| format!("{v:.0}")).collect::<Vec<_>>().join(",")
            );
        }
        std::fs::write(&path, out).expect("write the dump");
    }
}
