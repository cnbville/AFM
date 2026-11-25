//! Your coach's ratings, measured (coach ratings lane RE0,
//! `docs/design/coach-ratings-earned.md`, section 12): what each rating is
//! worth today, in both ways of playing, and how many coaches in a world
//! meet each archetype's feat. Every size in the design stays proposed
//! until this tool has measured it. Results: `docs/coach-check.md`.
//!
//! Usage:
//!   coach_check game [games] [seed] [world]
//!     Game day, seat by seat: your career at a team in the middle of its
//!     level, played on to 10 October with your staff running things, against the sixteen teams nearest
//!     it. Your coach at 12 in every rating, then one rating at 17 (a tier
//!     4 ability is 5 points), the same games on the same seeds. "Sim": your coach calls and decides. "By hand": every
//!     call on both sides comes from the hand, a caller of the level's
//!     typical skill, the same in both arms; the rest is your coach's. A
//!     rating that leaves your team as the engine dresses it unchanged is
//!     printed "dead" and plays no games.
//!   coach_check hand [games] [seed] [world]
//!     A real live game that stops on every snap: the same hand's calls
//!     are fed to two coaches who differ in one rating. Identical games
//!     mean the rating reads nothing at all. Games that part prove less:
//!     the staff's unused call still draws from the game's random stream,
//!     so the sizes by hand come from `game`, not from here.
//!   coach_check year <world> [days]
//!     Away from game day. One world played on twice from the same
//!     morning, games on the stand-in: as it is, and with one rating 5 points higher for every
//!     fourth head coach at each level. Growth, signings, the portal and
//!     wins, each team against itself in the other run; the head coaches
//!     nobody touched give the noise.
//!   coach_check world <world>
//!     The feats: how many head coaches and coordinators meet each
//!     archetype's feat on the world's own history, and in which season
//!     of a head coach's career he first does. Tiers are not built yet,
//!     so these are the most each archetype could be held.
//! `<world>` is a world file, or `new [scale] [history_years] [seed]`
//! (defaults 0.02, 1, 331; games on the stand-in). `game` and `hand` build
//! that small world themselves for its rosters and staffs.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use afm_engine::game::live::{Ask, Input, LiveGame, Params, PlanChange, Settings, StopMode};
use afm_engine::game::roster::Caller;
use afm_engine::game::rules::coach_norm;
use afm_engine::game::{GameSetup, GameTeam, quick_game};
use afm_engine::player::Level;
use afm_engine::world::coach::{Ability, Employer, GROUP_COUNT, Role, Yours};
use afm_engine::world::data::Division;
use afm_engine::world::folk::bonds::Step;
use afm_engine::world::power;
use afm_engine::world::seat::{Job, You};
use afm_engine::world::tally::Level as TLevel;
use afm_engine::world::{Stage, World, WorldOptions};
use rayon::prelude::*;

/// Your coach's start: 12 in everything (a road's base is 11 to 14).
const BASE: f64 = 12.0;
/// A tier 4 ability: 5 points of its rating in its own situation.
const LIFT: f64 = 5.0;

const RATINGS: [&str; 12] = [
    "play calling",
    "adaptation",
    "game management",
    "scheme",
    "motivation",
    "discipline",
    "relationships",
    "teaching",
    "recruiting",
    "evaluation",
    "network",
    "negotiating",
];

/// Adds `by` to one rating.
fn lift(a: &mut Ability, y: Option<&mut Yours>, rating: &str, by: f64) {
    let up = |v: &mut f64| *v = (*v + by).clamp(1.0, 20.0);
    match rating {
        "scheme" => up(&mut a.scheme),
        "game management" => up(&mut a.game_management),
        "motivation" => up(&mut a.motivation),
        "discipline" => up(&mut a.discipline),
        "relationships" => up(&mut a.relationships),
        "recruiting" => up(&mut a.recruiting),
        "network" => up(&mut a.network),
        "teaching" => a.teaching.iter_mut().for_each(up),
        _ => {
            if let Some(y) = y {
                match rating {
                    "play calling" => up(&mut y.play_calling),
                    "adaptation" => up(&mut y.adaptation),
                    "evaluation" => up(&mut y.evaluation),
                    "negotiating" => up(&mut y.negotiating),
                    _ => {}
                }
            }
        }
    }
}

fn you() -> You {
    let teaching = vec![BASE; GROUP_COUNT];
    serde_json::from_value(serde_json::json!({
        "name": "Test Coach", "age": 45, "ethnicity": "White", "home_state": 0, "ex_pro": false, "reputation": 9.0,
        "teaching": teaching, "scheme": BASE, "game_management": BASE,
        "motivation": BASE, "discipline": BASE, "recruiting": BASE, "network": BASE, "relationships": BASE,
        "style": { "risk": 10.0, "freedom": 10.0, "physical": 10.0, "ball_hunting": 10.0, "mould": 10.0 },
        "yours": { "play_calling": BASE, "adaptation": BASE, "evaluation": BASE, "negotiating": BASE }
    }))
    .unwrap()
}

/// The world for a part: a file, or one built here and kept in the temp
/// folder for the next run.
fn world_of(args: &[String]) -> (World, std::path::PathBuf) {
    let first = args.first().map(String::as_str).unwrap_or("new");
    if first != "new" {
        let p = Path::new(first).to_path_buf();
        return (afm_engine::save::read_world(&p).expect("a world file"), p);
    }
    let scale: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.02);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(331);
    let file = std::env::temp_dir().join(format!("coach_check_{seed}_{years}_{scale}.afmworld"));
    if !file.exists() {
        let t = std::time::Instant::now();
        let w = World::new(WorldOptions {
            seed,
            scale,
            history_years: years,
            engine_from: Some(2100),
            hs_engine_years: None,
            stop_jan1: true,
            ..Default::default()
        });
        afm_engine::save::write_world(&w, &file, "coach_check", "now", None).expect("write the world");
        eprintln!("built a world: scale {scale}, {years} years, seed {seed}, {:.0} s", t.elapsed().as_secs_f64());
    }
    (afm_engine::save::read_world(&file).expect("the world just written"), file)
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 }
}

fn sd(v: &[f64]) -> f64 {
    if v.len() < 2 {
        return 0.0;
    }
    let m = mean(v);
    (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64).sqrt()
}

// ------------------------------------------------------------ game day

struct Seat {
    name: &'static str,
    level: &'static str,
    calls: (bool, bool),
}

const SEATS: [Seat; 6] = [
    Seat { name: "High school head coach (he calls both sides)", level: "hs", calls: (true, true) },
    Seat { name: "College head coach, his coordinators call", level: "fbs", calls: (false, false) },
    Seat { name: "College head coach who calls his offense", level: "fbs", calls: (true, false) },
    Seat { name: "College head coach who calls his defense", level: "fbs", calls: (false, true) },
    Seat { name: "NFL head coach, his coordinators call", level: "nfl", calls: (false, false) },
    Seat { name: "NFL head coach who calls his offense", level: "nfl", calls: (true, false) },
];

/// The job for a seat's level: a team in the middle of its level, so the
/// games are close enough for a coach to matter.
fn job_at(w: &World, level: &str) -> Job {
    match level {
        "nfl" => {
            let mut v: Vec<usize> =
                (0..w.clubs.len()).filter(|&t| afm_engine::world::franchise::active_in(w, t, w.year)).collect();
            v.sort_by_key(|&t| w.clubs[t].roster.len());
            Job::Nfl(v[v.len() / 2])
        }
        "fbs" => {
            let mut v: Vec<usize> = (0..w.programs.len())
                .filter(|&p| w.programs[p].division == Division::FBS && w.programs[p].head_coach.is_some())
                .collect();
            v.sort_by(|&a, &b| w.programs[a].prestige.total_cmp(&w.programs[b].prestige));
            Job::College(v[v.len() / 2])
        }
        _ => {
            let mut v: Vec<usize> = (0..w.high_schools.len()).filter(|&h| w.high_schools[h].coach.is_some()).collect();
            v.sort_by(|&a, &b| w.high_schools[a].prestige.total_cmp(&w.high_schools[b].prestige));
            Job::HighSchool(v[v.len() * 3 / 4])
        }
    }
}

/// The world for a level's seats: your career at a team in the middle of
/// its level, played on to the tenth of October (on the first morning an
/// interim still has the team), and the sixteen teams nearest yours in
/// standing as the opponents.
fn seat_world(file: &Path, level: &str) -> (World, Vec<GameTeam>) {
    // A career can end before October (the world fires you as it fires
    // anyone: an NFL club can let a first-year head coach go after four
    // games). Then the same morning is played again as another career.
    let (w, job) = (7..15u32)
        .find_map(|career| {
            let mut w = afm_engine::save::read_world(file).expect("the world");
            w.options.engine_from = Some(2100);
            w.set_tallies_off(true);
            let job = job_at(&w, level);
            let me = w.start_career(&you(), job, career);
            // On to 10 October with your staff running things (the sandbox
            // jump), so the roster is kept as an AI staff keeps it.
            let day = afm_engine::calendar::Date::ymd(w.today.year(), 10, 10);
            while w.today.days_to(day) > 0 {
                let next = w.today.add(1);
                w.jump_to(next);
                if w.coaches[me].job.is_none() {
                    let why = w.news.iter().rev().take(40).find(|n| n.text.contains(&w.coaches[me].name));
                    eprintln!(
                        "{level}: career {career} lost the job on {} ({}); playing another",
                        w.today.pretty(),
                        why.map_or("no news names him", |n| n.text.as_str())
                    );
                    return None;
                }
            }
            Some((w, job))
        })
        .unwrap_or_else(|| panic!("{level}: eight careers in a row lost the job before 10 October"));
    let near = |mine: usize, mut v: Vec<usize>, key: &dyn Fn(usize) -> f64| -> Vec<usize> {
        v.sort_by(|&a, &b| key(a).total_cmp(&key(b)));
        let at = v.iter().position(|&x| x == mine).unwrap_or(v.len() / 2);
        let lo = at.saturating_sub(8).min(v.len().saturating_sub(17));
        v.into_iter().skip(lo).take(17).filter(|&x| x != mine).collect()
    };
    let opp: Vec<GameTeam> = match job {
        Job::College(p) => {
            let all = (0..w.programs.len()).filter(|&q| w.programs[q].division == Division::FBS).collect();
            near(p, all, &|q| w.programs[q].prestige).into_iter().filter_map(|q| w.debug_game_team(q)).collect()
        }
        Job::HighSchool(h) => {
            let all = (0..w.high_schools.len()).filter(|&q| w.high_schools[q].coach.is_some() || q == h).collect();
            near(h, all, &|q| w.high_schools[q].prestige).into_iter().filter_map(|q| w.debug_hs_team(q)).collect()
        }
        Job::Nfl(t) => (0..w.clubs.len())
            .filter(|&q| q != t && afm_engine::world::franchise::active_in(&w, q, w.year))
            .filter_map(|q| w.debug_nfl_team(q).map(|x| x.0))
            .collect(),
        _ => Vec::new(),
    };
    assert!(opp.len() >= 4, "{level}: only {} opponents dress", opp.len());
    (w, opp)
}

/// Your team as the engine dresses it today, with your coach at the base
/// and one rating lifted (None: the base), calling the sides the seat says.
fn dressed(w: &mut World, seat: &Seat, rating: Option<&str>) -> (GameTeam, Level) {
    let c = w.career.as_ref().expect("a career").coach;
    let base = you();
    afm_engine::debug::set_coach(w, c, |a, y| {
        a.teaching = base.teaching;
        a.scheme = BASE;
        a.game_management = BASE;
        a.motivation = BASE;
        a.discipline = BASE;
        a.recruiting = BASE;
        a.network = BASE;
        a.relationships = BASE;
        let mut mine = base.yours;
        mine.calls_offense = seat.calls.0;
        mine.calls_defense = seat.calls.1;
        if let Some(y) = y {
            *y = mine;
        }
    });
    if let Some(r) = rating {
        afm_engine::debug::set_coach(w, c, |a, y| lift(a, y, r, LIFT));
    }
    w.seat_game_team().unwrap_or_else(|| {
        // Why not, as far as the tool can see.
        let club = w.seat.and_then(|s| s.nfl_team).map(|t| {
            use afm_engine::world::pro::Status;
            let on = |f: &dyn Fn(&afm_engine::world::pro::ProCareer) -> bool| {
                w.clubs[t].roster.iter().filter(|&&i| w.people[i].pro.as_deref().is_some_and(f)).count()
            };
            format!(
                "club {t}: {} on its list, {} on the roster, {} of them healthy, {} on the practice squad, {} on reserve",
                w.clubs[t].roster.len(),
                on(&|c| c.status == Status::Roster),
                on(&|c| c.status == Status::Roster && c.out_weeks == 0),
                on(&|c| c.status == Status::PracticeSquad),
                on(&|c| c.status == Status::Reserve)
            )
        });
        panic!(
            "{}: your team does not dress on {} (seat {:?}; your job {:?}; {})",
            seat.name,
            w.today.pretty(),
            w.seat.map(|s| (s.level, s.nfl_team, s.program)),
            w.coaches[c].job.map(|j| (j.employer, j.role)),
            club.unwrap_or_default()
        )
    })
}

/// The hand: a caller of the level's typical skill, on both sides.
fn by_hand(t: &GameTeam, level: Level) -> GameTeam {
    let n = coach_norm(level);
    let mut t = t.clone();
    t.staff.offense = Caller::new(n, n, 10.0);
    t.staff.defense = Caller::new(n, n, 10.0);
    t
}

fn same(a: &GameTeam, b: &GameTeam) -> bool {
    serde_json::to_string(a).ok() == serde_json::to_string(b).ok()
}

fn parts(a: &serde_json::Value, b: &serde_json::Value, path: &str, out: &mut BTreeMap<String, usize>) {
    use serde_json::Value as V;
    match (a, b) {
        (V::Object(x), V::Object(y)) => {
            for (k, v) in x {
                let at = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
                match y.get(k) {
                    Some(w) => parts(v, w, &at, out),
                    None => *out.entry(at).or_default() += 1,
                }
            }
        }
        (V::Array(x), V::Array(y)) if x.len() == y.len() => {
            for (v, w) in x.iter().zip(y) {
                parts(v, w, &format!("{path}[]"), out);
            }
        }
        _ if a != b => *out.entry(path.to_string()).or_default() += 1,
        _ => {}
    }
}

/// The why of a row: which parts of your dressed team the lift changed
/// (a field of every player counts once, with how many men it changed).
fn reach(a: &GameTeam, b: &GameTeam) -> String {
    let (Ok(x), Ok(y)) = (serde_json::to_value(a), serde_json::to_value(b)) else { return String::new() };
    let mut out = BTreeMap::new();
    parts(&x, &y, "", &mut out);
    let n = out.len();
    let mut v: Vec<String> =
        out.into_iter().take(8).map(|(k, c)| if c > 1 { format!("{k} (x{c})") } else { k }).collect();
    if n > 8 {
        v.push(format!("and {} more", n - 8));
    }
    v.join(", ")
}

/// One game's line for team `me`: points for, against, win (a tie a half).
fn line(home: &GameTeam, away: &GameTeam, level: Level, seed: u64, mine_home: bool) -> [f64; 3] {
    let r = quick_game(home, away, level, seed);
    let (f, a) = if mine_home { (r.stats.score[0], r.stats.score[1]) } else { (r.stats.score[1], r.stats.score[0]) };
    [
        f as f64,
        a as f64,
        if f > a {
            1.0
        } else if f == a {
            0.5
        } else {
            0.0
        },
    ]
}

fn play(me: &GameTeam, opp: &[GameTeam], level: Level, games: usize, seed: u32) -> Vec<[f64; 3]> {
    (0..games)
        .into_par_iter()
        .map(|g| {
            let o = &opp[g % opp.len()];
            let s = seed as u64 * 1_000_003 + g as u64;
            if g % 2 == 0 { line(me, o, level, s, true) } else { line(o, me, level, s, false) }
        })
        .collect()
}

/// The difference of two arms on the same games, in blocks: (mean, its
/// standard error) for points for, against and wins.
fn diff(a: &[[f64; 3]], b: &[[f64; 3]]) -> [(f64, f64); 3] {
    let blocks = if a.len() >= 400 { 40 } else { 8 };
    let per = a.len().div_ceil(blocks);
    let mut out = [(0.0, 0.0); 3];
    for (k, o) in out.iter_mut().enumerate() {
        let d: Vec<f64> = a
            .chunks(per)
            .zip(b.chunks(per))
            .map(|(x, y)| {
                mean(&x.iter().map(|l| l[k]).collect::<Vec<_>>()) - mean(&y.iter().map(|l| l[k]).collect::<Vec<_>>())
            })
            .collect();
        *o = (mean(&d), sd(&d) / (d.len() as f64).sqrt());
    }
    out
}

fn game(games: usize, seed: u32, file: &Path) {
    println!("# Game day: one rating at {} against {BASE}, {games} games an arm, seed {seed}", BASE + LIFT);
    println!("Each cell: the lift's worth a game, with its standard error in brackets. Wins are a share of games.");
    let mut held: Option<(&str, World, Vec<GameTeam>)> = None;
    // `AFM_SEATS=nfl` (or `fbs`, `hs`, or a list) plays only those levels' seats.
    let only = std::env::var("AFM_SEATS").unwrap_or_default();
    for seat in SEATS.iter().filter(|s| only.is_empty() || only.split(',').any(|x| x == s.level)) {
        if held.as_ref().is_none_or(|h| h.0 != seat.level) {
            let (w, opp) = seat_world(file, seat.level);
            held = Some((seat.level, w, opp));
        }
        let (_, w, opp) = held.as_mut().unwrap();
        let (base, level) = dressed(w, seat, None);
        let hand = by_hand(&base, level);
        let base_sim = play(&base, opp, level, games, seed);
        let base_hand = play(&hand, opp, level, games, seed);
        let col = |v: &[[f64; 3]], k: usize| mean(&v.iter().map(|l| l[k]).collect::<Vec<_>>());
        println!("\n## {}", seat.name);
        println!(
            "your team against the sixteen teams nearest it ({}): sim {:.1}-{:.1}, {:.0}% wins; by hand {:.1}-{:.1}, {:.0}% wins",
            w.today.pretty(),
            col(&base_sim, 0),
            col(&base_sim, 1),
            100.0 * col(&base_sim, 2),
            col(&base_hand, 0),
            col(&base_hand, 1),
            100.0 * col(&base_hand, 2)
        );
        println!("{:<16} {:<5} {:>16} {:>16} {:>16}", "rating", "way", "points for", "points against", "wins");
        for r in RATINGS {
            let (t, _) = dressed(w, seat, Some(r));
            for (way, mine, base_team, base_lines) in
                [("sim", t.clone(), &base, &base_sim), ("hand", by_hand(&t, level), &hand, &base_hand)]
            {
                if same(&mine, base_team) {
                    println!("{r:<16} {way:<5} dead: your team dresses the same");
                    continue;
                }
                let d = diff(&play(&mine, opp, level, games, seed), base_lines);
                let cell = |x: (f64, f64)| format!("{:+.2} ({:.2})", x.0, x.1);
                let wins = format!("{:+.3} ({:.3})", d[2].0, d[2].1);
                println!(
                    "{r:<16} {way:<5} {:>16} {:>16} {:>16}   reaches: {}",
                    cell(d[0]),
                    cell(d[1]),
                    wins,
                    reach(base_team, &mine)
                );
            }
        }
    }
}

// ------------------------------------------------------------ by hand, live

/// Plays a live game that stops on every snap. With `feed` None every stop
/// takes the staff's choice and the answers are kept; with a feed, the
/// answers given are that list, in order. Returns the score, the plays and
/// the answers.
fn live(
    home: &GameTeam,
    away: &GameTeam,
    level: Level,
    seed: u64,
    feed: Option<&[Input]>,
) -> ([i32; 2], Vec<String>, Vec<Input>) {
    let setup = GameSetup::new(home, away, level, seed);
    let settings = Settings { stop: StopMode::EverySnap, ..Default::default() };
    let mut g = LiveGame::new(home.clone(), away.clone(), Params::of(&setup), 0, settings);
    let mut said = Vec::new();
    let mut step = g.run();
    while let Some(p) = step.pending.clone() {
        let input = match feed.and_then(|f| f.get(said.len())) {
            Some(i) => i.clone(),
            None if feed.is_some() => Input::Staff,
            None => match p.ask {
                Ask::Snap | Ask::Defense => {
                    Input::Call { fourth: p.staff_fourth, offense: p.staff_off, defense: p.staff_def }
                }
                Ask::Pat => Input::Pat { two: p.staff_two, offense: p.staff_off },
                Ask::Onside => Input::Onside(p.staff_onside),
                Ask::Toss => p.staff_toss.map_or(Input::Staff, Input::Toss),
                Ask::Challenge => Input::Challenge(p.staff_challenge),
                Ask::Halftime => Input::Halftime(PlanChange::default()),
            },
        };
        said.push(input.clone());
        g.ctl.answer(input);
        g.ctl.record_from = step.view.plays + 1;
        step = g.run();
    }
    let plays = step.result.stats.plays.iter().map(|p| serde_json::to_string(p).unwrap_or_default()).collect();
    (step.result.stats.score, plays, said)
}

fn hand(games: usize, seed: u32, file: &Path) {
    println!("# By hand, on a live game that stops on every snap ({games} games a rating, seed {seed})");
    println!("Coach A is at {BASE} in everything and his staff's choice is taken at every stop. Coach B has one");
    println!("rating at {} and is fed coach A's answers, so the hand makes the same calls for both.", BASE + LIFT);
    println!("\"Same\" means every play of the two games is the same: the rating read nothing at all. Where");
    println!("they part, the rating was read somewhere, but not always for a reason: the staff's own call is");
    println!("still worked out on every snap before the hand's replaces it, and it draws from the game's");
    println!("random stream, so a different caller can shift every later draw. After that play coach A's");
    println!("answers no longer fit coach B's game, so only the play they part on is printed, not a score.");
    for seat in [&SEATS[2], &SEATS[0]] {
        let (mut w, opp) = seat_world(file, seat.level);
        let (a, level) = dressed(&mut w, seat, None);
        println!("\n## {}", seat.name);
        for r in
            ["play calling", "adaptation", "game management", "scheme", "motivation", "discipline", "relationships"]
        {
            let (b, _) = dressed(&mut w, seat, Some(r));
            if same(&a, &b) {
                println!("{r:<16} dead: your team dresses the same");
                continue;
            }
            let rows: Vec<(usize, usize)> = (0..games)
                .into_par_iter()
                .map(|g| {
                    let s = seed as u64 * 7_919 + g as u64;
                    let o = &opp[g % opp.len()];
                    let (sa, pa, said) = live(&a, o, level, s, None);
                    let (sb, pb, _) = live(&b, o, level, s, Some(&said));
                    let first = pa.iter().zip(&pb).position(|(x, y)| x != y);
                    let part = match first {
                        Some(i) => i + 1,
                        None if pa.len() != pb.len() || sa != sb => pa.len().min(pb.len()) + 1,
                        None => 0,
                    };
                    (part, pa.len())
                })
                .collect();
            let n = rows.iter().filter(|x| x.0 == 0).count();
            let mut at: Vec<usize> = rows.iter().filter(|x| x.0 > 0).map(|x| x.0).collect();
            at.sort_unstable();
            let plays = mean(&rows.iter().map(|x| x.1 as f64).collect::<Vec<_>>());
            match at.get(at.len() / 2) {
                Some(m) => println!(
                    "{r:<16} same in {n} of {games} games; the others part on play {m} (the median) of about {plays:.0}; reaches: {}",
                    reach(&a, &b)
                ),
                None => println!("{r:<16} same in {n} of {games} games; reaches: {}", reach(&a, &b)),
            }
        }
    }
}

// ------------------------------------------------------------ a played year

/// What a team's year came to.
#[derive(Default, Clone)]
struct Outcome {
    /// Mean overall gained by the players it had on the first morning.
    growth: f64,
    kept: f64,
    /// High school signees in the run: how many, and their stars summed.
    signed: f64,
    stars: f64,
    /// Players who left for another college.
    left: f64,
    /// A high school's players signed by colleges in the run.
    sent: f64,
    wins: f64,
    margin: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Team {
    Hs(usize),
    College(usize),
    Nfl(usize),
}

/// Who each head coach's rating is lifted in, by team: every fourth team
/// at each level takes one of three ratings, the fourth is left alone.
fn arm(t: Team) -> Option<&'static str> {
    let (k, set): (usize, [&str; 3]) = match t {
        Team::College(p) => (p, ["teaching", "recruiting", "relationships"]),
        Team::Hs(h) => (h, ["teaching", "network", "relationships"]),
        Team::Nfl(n) => (n, ["teaching", "motivation", "discipline"]),
    };
    (k % 4 < 3).then(|| set[k % 4])
}

fn head_of(w: &World, t: Team) -> Option<usize> {
    match t {
        Team::College(p) => w.programs[p].head_coach,
        Team::Hs(h) => w.high_schools[h].coach,
        Team::Nfl(n) => w.nfl[n].head_coach,
    }
}

fn team_of(w: &World, i: usize) -> Option<Team> {
    let p = &w.people[i];
    match p.stage {
        Stage::HighSchool => Some(Team::Hs(p.hs)),
        Stage::College => p.college.map(Team::College),
        Stage::Pro => p.pro.as_ref().and_then(|x| x.team).map(Team::Nfl),
        _ => None,
    }
}

fn teams(w: &World) -> Vec<Team> {
    let mut v: Vec<Team> = (0..w.programs.len())
        .filter(|&p| w.programs[p].division == Division::FBS && w.programs[p].head_coach.is_some())
        .map(Team::College)
        .collect();
    v.extend((0..w.high_schools.len()).filter(|&h| w.high_schools[h].coach.is_some()).map(Team::Hs));
    v.extend((0..w.nfl.len()).filter(|&n| w.nfl[n].head_coach.is_some()).map(Team::Nfl));
    v
}

/// Plays the world on from the file and reads each team's year.
fn run_year(file: &Path, days: u32, treated: bool) -> BTreeMap<Team, Outcome> {
    let mut w = afm_engine::save::read_world(file).expect("the world");
    // Games on the stand-in, and no counting for the Data section, in both runs.
    w.options.engine_from = Some(2100);
    w.set_tallies_off(true);
    let all = teams(&w);
    if treated {
        for &t in &all {
            if let (Some(r), Some(c)) = (arm(t), head_of(&w, t)) {
                afm_engine::debug::set_coach(&mut w, c, |a, y| lift(a, y, r, LIFT));
            }
        }
    }
    // Who was where on the first morning.
    let mut first: HashMap<u32, (Team, f64, bool)> = HashMap::new();
    for i in 0..w.people.len() {
        if let Some(t) = team_of(&w, i) {
            let p = &w.people[i];
            first.insert(p.player.id, (t, p.player.overall(), p.cal.signed.is_some()));
        }
    }
    let year = w.year;
    for _ in 0..days {
        w.advance_day();
    }
    let mut out: BTreeMap<Team, Outcome> = all.iter().map(|&t| (t, Outcome::default())).collect();
    for i in 0..w.people.len() {
        let p = &w.people[i];
        let Some(&(t, was, signed)) = first.get(&p.player.id) else { continue };
        let now = team_of(&w, i);
        if let Some(o) = out.get_mut(&t) {
            if now == Some(t) {
                o.growth += p.player.overall() - was;
                o.kept += 1.0;
            } else if matches!((t, now), (Team::College(_), Some(Team::College(_)))) {
                o.left += 1.0;
            }
        }
        // A high school player of the first morning, not signed then, who
        // has signed with a college since or enrolled at one (no walk-ons).
        if let (Team::Hs(h), false) = (t, signed) {
            let to = match p.cal.signed {
                Some(s) => (!s.walk_on).then_some(s.program),
                None => p.college.filter(|_| p.stage == Stage::College && !p.walk_on),
            };
            if let Some(prog) = to {
                if let Some(o) = out.get_mut(&Team::College(prog)) {
                    o.signed += 1.0;
                    o.stars += p.stars as f64;
                }
                if let Some(o) = out.get_mut(&Team::Hs(h)) {
                    o.sent += 1.0;
                }
            }
        }
    }
    for (t, o) in out.iter_mut() {
        if o.kept > 0.0 {
            o.growth /= o.kept;
        }
        let (wn, l, pf, pa) = match *t {
            Team::College(p) => w.programs[p]
                .seasons
                .iter()
                .rev()
                .find(|s| s.year >= year)
                .map_or((0, 0, 0, 0), |s| (s.wins, s.losses, s.points[0], s.points[1])),
            Team::Hs(h) => w.high_schools[h]
                .seasons
                .iter()
                .rev()
                .find(|s| s.year >= year)
                .map_or((0, 0, 0, 0), |s| (s.wins, s.losses, s.points[0], s.points[1])),
            // An NFL club's book keeps wins and losses, no points.
            Team::Nfl(n) => w
                .nfl_seasons
                .iter()
                .rev()
                .find(|s| s.team == n && s.year >= year)
                .map_or((0, 0, 0, 0), |s| (s.wins, s.losses + s.ties, 0, 0)),
        };
        let g = (wn + l).max(1) as f64;
        o.wins = wn as f64 / g;
        o.margin = (pf as f64 - pa as f64) / g;
    }
    out
}

fn year(args: &[String]) {
    let (w, file) = world_of(args);
    let days: u32 = args.iter().rev().find_map(|s| s.strip_prefix("days=").and_then(|d| d.parse().ok())).unwrap_or(400);
    println!(
        "# A played year: one rating {LIFT} points higher for every fourth head coach, {days} days from {}",
        w.today.pretty()
    );
    drop(w);
    let t = std::time::Instant::now();
    let base = run_year(&file, days, false);
    let lifted = run_year(&file, days, true);
    println!(
        "two runs in {:.0} s. Each cell: the lifted teams' change against themselves in the plain run,",
        t.elapsed().as_secs_f64()
    );
    println!("then the untouched teams' change the same way (the noise), with its standard error.");
    type Measure = (&'static str, fn(&Outcome) -> f64);
    let measures: [Measure; 7] = [
        ("growth, overall points a player", |o| o.growth),
        ("high school signees", |o| o.signed),
        ("signees' stars, summed", |o| o.stars),
        ("players lost to the portal", |o| o.left),
        ("players signed by colleges", |o| o.sent),
        ("win share", |o| o.wins),
        ("points margin a game", |o| o.margin),
    ];
    for (label, pick) in [("College (FBS)", 0u8), ("High school", 1), ("NFL", 2)] {
        let of = |t: &Team| match t {
            Team::College(_) => 0u8,
            Team::Hs(_) => 1,
            Team::Nfl(_) => 2,
        };
        println!("\n## {label}");
        let mut groups: BTreeMap<&str, Vec<Team>> = BTreeMap::new();
        for t in base.keys().filter(|t| of(t) == pick) {
            groups.entry(arm(*t).unwrap_or("untouched")).or_default().push(*t);
        }
        for (name, f) in measures {
            let d = |g: &[Team]| -> Vec<f64> { g.iter().map(|t| f(&lifted[t]) - f(&base[t])).collect() };
            let noise = d(groups.get("untouched").map_or(&[][..], |v| v));
            if noise.iter().all(|x| *x == 0.0) && groups.values().all(|g| d(g).iter().all(|x| *x == 0.0)) {
                continue;
            }
            println!("{name}");
            for (r, g) in &groups {
                let v = d(g);
                let base_mean = mean(&g.iter().map(|t| f(&base[t])).collect::<Vec<_>>());
                println!(
                    "  {r:<14} {:>5} teams  plain {base_mean:>8.3}  change {:>+8.3} ({:.3})",
                    g.len(),
                    mean(&v),
                    sd(&v) / (v.len().max(1) as f64).sqrt()
                );
            }
        }
    }
}

// ------------------------------------------------------------ the feats

/// A coach's seasons in a job: (season, employer, role).
fn seasons_of(w: &World) -> Vec<Vec<(i32, Employer, Role)>> {
    w.coaches
        .iter()
        .map(|c| {
            let mut v = Vec::new();
            for s in &c.history {
                for y in s.from..=s.to {
                    v.push((y, s.employer, s.role));
                }
            }
            if let Some(j) = c.job {
                let from = v.last().map_or(j.since, |l| (l.0 + 1).max(j.since));
                for y in from..w.year {
                    v.push((y, j.employer, j.role));
                }
            }
            v.sort_by_key(|x| x.0);
            v.dedup_by_key(|x| x.0);
            v
        })
        .collect()
}

fn tier(e: Employer, w: &World) -> Option<&'static str> {
    match e {
        Employer::HighSchool(_) => Some("High school"),
        Employer::College(p) => match w.programs[p].division {
            Division::FBS => Some("FBS"),
            Division::FCS => Some("FCS"),
            _ => None,
        },
        Employer::Nfl(_) => Some("NFL"),
        Employer::Other(_) => None,
    }
}

fn feats(args: &[String]) {
    let (mut w, file) = world_of(args);
    // The world's tallies sit beside it, as the game keeps them.
    let data = file.with_extension(afm_engine::world::tally::EXT);
    if data.is_dir() {
        w.attach_tallies(&data).expect("the tallies folder");
    }
    let jobs = seasons_of(&w);
    let years: Vec<i32> = {
        let mut y: Vec<i32> = w.programs.iter().flat_map(|p| p.seasons.iter().map(|s| s.year)).collect();
        y.sort_unstable();
        y.dedup();
        y
    };
    println!("# The feats on this world's history (seasons: {}, the last {})", years.len(), w.year - 1);
    println!("Tiers are not built, so a share here is the most an archetype could be held at.");

    // Unit ranks by season: (level, team, season) -> (offense's place, defense's place), 0 best to 1.
    let mut place: HashMap<(Employer, i32), (f64, f64)> = HashMap::new();
    for &y in &years {
        for (level, group) in [(TLevel::College, 0u8), (TLevel::Nfl, 1), (TLevel::Hs, 2)] {
            let r = power::season_ratings(&w, level, y);
            // A division for a college, a state for a high school, the league.
            let mut pools: HashMap<u32, Vec<u32>> = HashMap::new();
            for &t in r.keys() {
                let pool = match group {
                    0 => w.programs.get(t as usize).map_or(99, |p| p.division as u32),
                    2 => w.high_schools.get(t as usize).map_or(0, |h| h.state as u32),
                    _ => 0,
                };
                pools.entry(pool).or_default().push(t);
            }
            for v in pools.values() {
                let n = v.len().max(2) as f64 - 1.0;
                let rank = |f: fn(&power::Rating) -> f64| -> HashMap<u32, f64> {
                    let mut s = v.clone();
                    s.sort_by(|a, b| f(&r[b]).total_cmp(&f(&r[a])).then(a.cmp(b)));
                    s.iter().enumerate().map(|(k, t)| (*t, k as f64 / n)).collect()
                };
                let (off, def) = (rank(|x| x.off), rank(|x| x.def));
                for &t in v {
                    let e = match group {
                        0 => Employer::College(t as usize),
                        1 => Employer::Nfl(t as usize),
                        _ => Employer::HighSchool(t as usize),
                    };
                    place.insert((e, y), (off[&t], def[&t]));
                }
            }
        }
    }

    // Postseason wins and wins as a big underdog, by team and season, from
    // the engine's games (none in a world on the stand-in).
    let mut post: HashMap<(Employer, i32), u32> = HashMap::new();
    let mut upsets: HashMap<(Employer, i32), u32> = HashMap::new();
    let mut tallied = 0usize;
    for &y in &years {
        let all = power::tallied_games(&w, y);
        for (level, mk) in [
            (TLevel::College as u8, Employer::College as fn(usize) -> Employer),
            (TLevel::Nfl as u8, Employer::Nfl as fn(usize) -> Employer),
        ] {
            let g: Vec<power::SGame> = all.iter().filter(|x| x.0 == level).map(|x| x.2).collect();
            if g.is_empty() {
                continue;
            }
            tallied += g.len();
            let fit = power::solve(&g, &HashMap::new(), 1.0);
            for x in &g {
                let Some(won) = x.won else { continue };
                let home = power::win_chance(fit.margin(x.teams[0], x.teams[1], x.neutral), 14.0);
                let chance = if won == 0 { home } else { 1.0 - home };
                let e = mk(x.teams[won as usize] as usize);
                if x.post {
                    *post.entry((e, y)).or_default() += 1;
                }
                if chance < 0.35 {
                    *upsets.entry((e, y)).or_default() += 1;
                }
            }
        }
    }
    for (h, s) in w.high_schools.iter().enumerate() {
        for x in &s.seasons {
            if x.playoff_wins > 0 {
                *post.entry((Employer::HighSchool(h), x.year)).or_default() += x.playoff_wins as u32;
            }
        }
    }

    // Titles by team and season.
    let mut title: HashSet<(Employer, i32)> = HashSet::new();
    for (p, prog) in w.programs.iter().enumerate() {
        for s in &prog.seasons {
            if s.conference_title || s.national_title {
                title.insert((Employer::College(p), s.year));
            }
        }
    }
    for (h, s) in w.high_schools.iter().enumerate() {
        for x in s.seasons.iter().filter(|x| x.state_title) {
            title.insert((Employer::HighSchool(h), x.year));
        }
    }
    // The NFL's book: the champion, and (where no engine games counted the
    // club's postseason wins) the rounds it got through, a bye counted as one.
    let mut super_bowl: HashSet<(Employer, i32)> = HashSet::new();
    let nfl_post = post.keys().any(|k| matches!(k.0, Employer::Nfl(_)));
    for s in &w.nfl_seasons {
        let e = Employer::Nfl(s.team);
        // A conference title is reaching the Super Bowl; winning it counts as two.
        if s.round >= 4 {
            title.insert((e, s.year));
        }
        if s.round == 5 {
            super_bowl.insert((e, s.year));
        }
        if !nfl_post && s.round > 1 {
            *post.entry((e, s.year)).or_default() += s.round - 1;
        }
    }

    // Classes: each college class's score by program and year, and its
    // place in the conference; the head coach of the class's first season.
    let mut class: HashMap<(usize, i32), (Vec<f64>, Option<usize>)> = HashMap::new();
    // Draft picks who signed as three-stars or lower, by that head coach.
    let mut raw_picks: HashMap<usize, Vec<i32>> = HashMap::new();
    let mut signee = |staff: &[(i32, usize, Option<usize>, Option<usize>)], stars: u8, drafted: Option<i32>| {
        let Some(&(y, p, hc, _)) = staff.first() else { return };
        let e = class.entry((p, y)).or_default();
        e.0.push((stars as f64).powi(2));
        e.1 = e.1.or(hc);
        if let (Some(c), Some(d), true) = (hc, drafted, stars <= 3) {
            raw_picks.entry(c).or_default().push(d);
        }
    };
    for a in &w.alumni {
        signee(&a.college_staff, a.stars, a.pick.map(|_| a.year));
    }
    for p in &w.people {
        let drafted = p.pro.as_ref().and_then(|x| x.draft).map(|d| d.0);
        signee(&p.college_staff, p.stars, drafted);
    }
    let mut top2: HashMap<usize, Vec<i32>> = HashMap::new();
    {
        // A conference's classes in a year: (score, the head coach).
        type Classes = Vec<(f64, Option<usize>)>;
        let mut by: HashMap<(String, i32), Classes> = HashMap::new();
        for ((p, y), (v, hc)) in &class {
            let mut v = v.clone();
            v.sort_by(|a, b| b.total_cmp(a));
            let score: f64 = v.iter().take(25).sum();
            let prog = &w.programs[*p];
            let conf = prog.seasons.iter().find(|s| s.year == *y).and_then(|s| s.conference.clone());
            by.entry((conf.unwrap_or_else(|| prog.conference.clone()), *y)).or_default().push((score, *hc));
        }
        for ((_, y), v) in by.iter_mut() {
            v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            for c in v.iter().take(2).filter_map(|x| x.1) {
                top2.entry(c).or_default().push(*y);
            }
        }
    }

    // Bonds at the top stage, by coach.
    let mut family: HashMap<usize, u32> = HashMap::new();
    for v in w.folk.bonds.by.values() {
        for b in v.iter().filter(|b| Step::of(b) == Step::Family) {
            *family.entry(b.coach as usize).or_default() += 1;
        }
    }

    // Each coach's first season meeting each feat (None: never), read over
    // his head coach seasons (and, for the unit feat, his coordinator ones).
    const FEATS: [&str; 7] = [
        "Mastermind (a top-tenth unit, or two in the top quarter)",
        "Trail Boss (two classes first or second in the conference)",
        "Standard Setter (five players at the bond's top stage)",
        "Sideline General (five wins under 35% in two seasons)",
        "Pro Factory (three picks signed at three stars or lower)",
        "Builder (five postseason wins as head coach)",
        "Institution (two titles as head coach)",
    ];
    // (the head-coach season number it came in, 1 = his first)
    let met: Vec<[Option<u32>; 7]> = (0..w.coaches.len())
        .map(|c| {
            let mut m = [None; 7];
            let (mut quarter, mut posts, mut titles, mut hc_n) = (0u32, 0u32, 0u32, 0u32);
            let mut last_upsets = 0u32;
            for &(y, e, role) in &jobs[c] {
                let head = role == Role::HeadCoach;
                if head {
                    hc_n += 1;
                }
                let n = hc_n.max(1);
                // The unit he runs: his side as a coordinator, the better of the two as head coach.
                let unit = place.get(&(e, y)).and_then(|&(o, d)| match role {
                    Role::HeadCoach => Some(o.min(d)),
                    Role::OffensiveCoordinator => Some(o),
                    Role::DefensiveCoordinator => Some(d),
                    Role::PositionCoach(_) => None,
                });
                if let Some(u) = unit {
                    if u <= 0.25 {
                        quarter += 1;
                    }
                    if (u <= 0.10 || quarter >= 2) && m[0].is_none() {
                        m[0] = Some(n);
                    }
                }
                if !head {
                    continue;
                }
                if let Employer::College(_) = e {
                    if top2.get(&c).is_some_and(|v| v.iter().filter(|z| **z <= y).count() >= 2) && m[1].is_none() {
                        m[1] = Some(n);
                    }
                    if raw_picks.get(&c).is_some_and(|v| v.iter().filter(|d| **d <= y + 1).count() >= 3)
                        && m[4].is_none()
                    {
                        m[4] = Some(n);
                    }
                }
                let ups = upsets.get(&(e, y)).copied().unwrap_or(0);
                if ups + last_upsets >= 5 && m[3].is_none() {
                    m[3] = Some(n);
                }
                last_upsets = ups;
                posts += post.get(&(e, y)).copied().unwrap_or(0);
                if posts >= 5 && m[5].is_none() {
                    m[5] = Some(n);
                }
                if title.contains(&(e, y)) {
                    titles += 1;
                }
                if super_bowl.contains(&(e, y)) {
                    titles += 1;
                }
                if titles >= 2 && m[6].is_none() {
                    m[6] = Some(n);
                }
            }
            if family.get(&c).copied().unwrap_or(0) >= 5 {
                m[2] = Some(hc_n.max(1));
            }
            m
        })
        .collect();

    println!(
        "read from: {} engine games with a postseason flag and a win chance{}; {} classes; {} bonds at the top stage",
        tallied,
        if tallied == 0 { " (none: Sideline General and the colleges' Builder can't be read here)" } else { "" },
        class.len(),
        family.values().sum::<u32>()
    );
    println!("not read: System Builder (who signed each starter is not kept by season); the high school and NFL");
    println!("forms of Trail Boss and Pro Factory. An NFL conference title is one title and a Super Bowl two;");
    println!("a college title is a conference or national one, a high school title a state one.");
    for level in ["High school", "FBS", "FCS", "NFL"] {
        let now = |role: fn(Role) -> bool| -> Vec<usize> {
            (0..w.coaches.len())
                .filter(|&c| w.coaches[c].job.is_some_and(|j| role(j.role) && tier(j.employer, &w) == Some(level)))
                .collect()
        };
        let heads = now(|r| r == Role::HeadCoach);
        let coords = now(|r| matches!(r, Role::OffensiveCoordinator | Role::DefensiveCoordinator));
        if heads.is_empty() {
            continue;
        }
        println!("\n## {level}: {} head coaches, {} coordinators today", heads.len(), coords.len());
        println!("{:<60} {:>7} {:>7} {:>9} {:>20}", "feat", "heads", "coords", "median", "by season 2 / 4 / 6");
        for (k, name) in FEATS.iter().enumerate() {
            let share =
                |v: &[usize]| 100.0 * v.iter().filter(|&&c| met[c][k].is_some()).count() as f64 / v.len().max(1) as f64;
            let mut when: Vec<u32> = heads.iter().filter_map(|&c| met[c][k]).collect();
            when.sort_unstable();
            let by = |n: u32| 100.0 * when.iter().filter(|&&x| x <= n).count() as f64 / heads.len() as f64;
            let median = when.get(when.len() / 2).map_or("-".to_string(), |m| format!("{m}"));
            let coord = if k == 0 { format!("{:.1}%", share(&coords)) } else { "-".to_string() };
            println!(
                "{name:<60} {:>6.1}% {coord:>7} {median:>9} {:>20}",
                share(&heads),
                format!("{:.0}% / {:.0}% / {:.0}%", by(2), by(4), by(6))
            );
        }
        // Builder needs any two advanced archetypes; the most that could hold it.
        let two = heads.iter().filter(|&&c| (0..5).filter(|&k| met[c][k].is_some()).count() >= 2).count();
        let builder = heads
            .iter()
            .filter(|&&c| met[c][5].is_some() && (0..5).filter(|&k| met[c][k].is_some()).count() >= 2)
            .count();
        let inst = heads
            .iter()
            .filter(|&&c| {
                met[c][5].is_some() && met[c][6].is_some() && (0..5).filter(|&k| met[c][k].is_some()).count() >= 2
            })
            .count();
        let seasons = mean(
            &heads
                .iter()
                .map(|&c| jobs[c].iter().filter(|x| x.2 == Role::HeadCoach).count() as f64)
                .collect::<Vec<_>>(),
        );
        println!(
            "two or more of the first five: {two}; with Builder's feat too: {builder}; with Institution's as well: {inst}. Head coaches' mean seasons as a head coach: {seasons:.1}"
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let n = |k: usize, d: usize| args.get(k).and_then(|s| s.parse().ok()).unwrap_or(d);
    match args.first().map(String::as_str) {
        Some("game") => {
            let (_, file) = world_of(args.get(3..).unwrap_or(&[]));
            game(n(1, 400), n(2, 331) as u32, &file);
        }
        Some("hand") => {
            let (_, file) = world_of(args.get(3..).unwrap_or(&[]));
            hand(n(1, 8), n(2, 331) as u32, &file);
        }
        Some("year") => year(&args[1..]),
        Some("world") => feats(&args[1..]),
        _ => eprintln!(
            "usage: coach_check game [games] [seed] [world] | hand [games] [seed] [world] | year <world> [days=N] | world <world>"
        ),
    }
}
