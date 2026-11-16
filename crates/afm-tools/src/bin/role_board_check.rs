//! Role grading on the NFL boards (`world::pro::roles`): plays a saved world
//! to its draft on the stand-in and prints, for each position with jobs,
//! what kind of player each scheme took (his true lean toward one job over
//! the other) and how many picks came from below the public top 300.
//! `role_board_check <world file> [boards]`: with `boards`, the clubs'
//! boards on draft morning instead: how far each club's grade over the
//! public grade follows a prospect's true lean to one job, by scheme.
use afm_engine::game::roster::{Front, Shell};
use afm_engine::positions::Position;
use afm_engine::world::pro::roles::{Job, lift, three_wide};
use afm_engine::world::systems::Installed;
use afm_engine::world::{Stage, World};
use std::collections::HashMap;
use std::path::Path;

struct Pick {
    pos: Position,
    pick: u32,
    inst: Installed,
    lean: f64,
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() { f64::NAN } else { v.iter().sum::<f64>() / v.len() as f64 }
}

fn corr(x: &[f64], y: &[f64]) -> f64 {
    let (mx, my) = (mean(x), mean(y));
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        sxy += (a - mx) * (b - my);
        sxx += (a - mx) * (a - mx);
        syy += (b - my) * (b - my);
    }
    sxy / (sxx * syy).sqrt().max(1e-12)
}

/// His true lean toward the first job over the second, in rating points.
fn job_pair(pos: Position) -> Option<(Job, Job)> {
    match pos {
        Position::WR => Some((Job::SlotWr, Job::OutsideWr)),
        Position::CB => Some((Job::Nickel, Job::OutsideCb)),
        Position::S => Some((Job::FreeSafety, Job::BoxSafety)),
        Position::EDGE => Some((Job::StandUpRusher, Job::HandDownEnd)),
        Position::DT => Some((Job::FiveTech, Job::ThreeTech)),
        Position::LB => Some((Job::Sam, Job::Will)),
        _ => None,
    }
}

fn lean_of(w: &World, i: usize) -> Option<(Position, f64)> {
    let p = &w.people[i];
    let pos = p.cal.pro_position.unwrap_or(p.player.position);
    let (a, b) = job_pair(pos)?;
    let (h, wt) = (p.player.body.height, p.player.body.weight);
    Some((pos, lift(pos, a, &p.player.attributes, wt, h) - lift(pos, b, &p.player.attributes, wt, h)))
}

type Split = (&'static str, fn(&Installed) -> bool, &'static str, &'static str);

fn splits() -> Vec<(Position, Split)> {
    vec![
        (Position::EDGE, ("the 3-4 rusher", |i| i.front() == Front::ThreeFour, "3-4", "4-3")),
        (Position::DT, ("the 3-4 end", |i| i.front() == Front::ThreeFour, "3-4", "4-3")),
        (Position::LB, ("the Sam", |i| i.front() == Front::ThreeFour, "3-4", "4-3")),
        (
            Position::S,
            ("the deep safety", |i| matches!(i.shell(), Shell::Cover3 | Shell::ManPressure), "single-high", "two-high"),
        ),
        (Position::WR, ("the slot", |i| three_wide(i.offense()) >= 0.9, "3-wide", "other")),
        (Position::CB, ("the nickel", |_| true, "all", "none")),
    ]
}

/// Each club's board on draft morning: the correlation, over the top 300
/// public prospects it filed at a position, of its grade over the public
/// grade with his true lean, averaged over the clubs on each side.
fn boards(w: &mut World) {
    let boards = afm_engine::debug::draft_boards(w);
    let mut v: Vec<(f64, usize)> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.stage == Stage::Declared)
        .filter_map(|(i, p)| p.cal.draft_read.map(|r| (r.score, i)))
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let top: Vec<usize> = v.iter().take(300).map(|x| x.1).collect();
    // The public grade: the mean board grade over the clubs.
    let public: HashMap<usize, f64> = top
        .iter()
        .map(|&i| (i, mean(&boards.iter().filter_map(|b| b.grade.get(&i).copied()).collect::<Vec<_>>())))
        .collect();
    println!(
        "{} draft morning, {} boards, public top 300; filed {} in all, graded for a job {}",
        w.year,
        boards.len(),
        boards.iter().map(|b| b.filed).sum::<usize>(),
        boards.iter().map(|b| b.role.len()).sum::<usize>()
    );
    for (pos, (name, yes, a, b)) in splits() {
        let men: Vec<(usize, f64)> =
            top.iter().filter_map(|&i| lean_of(w, i).filter(|x| x.0 == pos).map(|x| (i, x.1))).collect();
        let (mut ya, mut nb) = (Vec::new(), Vec::new());
        for (t, bd) in boards.iter().enumerate() {
            let (mut x, mut y) = (Vec::new(), Vec::new());
            for &(i, l) in &men {
                if let Some(g) = bd.grade.get(&i) {
                    x.push(l);
                    y.push(g - public[&i]);
                }
            }
            if x.len() < 5 {
                continue;
            }
            let c = corr(&x, &y);
            if yes(&w.clubs[t].installed) { ya.push(c) } else { nb.push(c) }
        }
        println!(
            "{:<5} board over public vs lean to {name}: {a} clubs r {:+.2} (n {}), {b} clubs r {:+.2} (n {}), {} men",
            pos.name(),
            mean(&ya),
            ya.len(),
            mean(&nb),
            nb.len(),
            men.len()
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut w = afm_engine::save::read_world(Path::new(&args[1])).expect("read the world");
    w.options.engine_from = Some(2100);
    eprintln!("today {:?}, draft set up {}", w.today, w.draft_run.is_some());
    if afm_engine::debug::redo_draft_setup(&mut w) {
        eprintln!("the saved draft was set up again on today's code");
    }
    if args.get(2).map(String::as_str) == Some("boards") {
        while w.today != w.dates.draft[0].date {
            w.advance_day();
        }
        boards(&mut w);
        return;
    }
    let mut inst: Vec<Installed> = Vec::new();
    let mut year = 0;
    // The public order the morning of round one (the services' score).
    let mut public: HashMap<u32, usize> = HashMap::new();
    for _ in 0..500 {
        if w.today == w.dates.draft[0].date && inst.is_empty() {
            inst = w.clubs.iter().map(|c| c.installed).collect();
            year = w.year;
            let mut v: Vec<(f64, u32)> = w
                .people
                .iter()
                .filter(|p| p.stage == Stage::Declared)
                .filter_map(|p| p.cal.draft_read.map(|r| (r.score, p.player.id)))
                .collect();
            v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            public = v.into_iter().enumerate().map(|(k, x)| (x.1, k)).collect();
        }
        w.advance_day();
        if !inst.is_empty() && w.today > w.dates.udfa_rush.date {
            break;
        }
    }
    assert!(!inst.is_empty(), "never reached the draft");
    let mut picks: Vec<Pick> = Vec::new();
    let (mut all, mut deep) = (0, 0);
    for (i, p) in w.people.iter().enumerate() {
        let Some((y, _, pick, t)) = p.pro.as_ref().and_then(|c| c.draft) else { continue };
        if y != year {
            continue;
        }
        all += 1;
        deep += usize::from(public.get(&p.player.id).is_some_and(|&k| k >= 300));
        if let Some((pos, lean)) = lean_of(&w, i) {
            picks.push(Pick { pos, pick, inst: inst[t], lean });
        }
    }
    println!("{year} draft: {all} picks, {deep} from below the public top 300; {} at positions with jobs", picks.len());
    for (pos, (name, yes, a, b)) in splits() {
        let v: Vec<&Pick> = picks.iter().filter(|x| x.pos == pos).collect();
        let ya: Vec<f64> = v.iter().filter(|x| yes(&x.inst)).map(|x| x.lean).collect();
        let nb: Vec<f64> = v.iter().filter(|x| !yes(&x.inst)).map(|x| x.lean).collect();
        println!(
            "{:<5} lean to {name}: {a} clubs took {:+.2} (n {}), {b} clubs {:+.2} (n {})",
            pos.name(),
            mean(&ya),
            ya.len(),
            mean(&nb),
            nb.len(),
        );
    }
    // Where the part-time types went: the quarter leaning most to the
    // part-time job against the rest.
    for (pos, what) in [(Position::CB, "nickel"), (Position::WR, "slot"), (Position::LB, "Sam")] {
        let mut v: Vec<&Pick> = picks.iter().filter(|x| x.pos == pos).collect();
        v.sort_by(|a, b| b.lean.total_cmp(&a.lean));
        let q = v.len() / 4;
        let top: Vec<f64> = v[..q].iter().map(|x| x.pick as f64).collect();
        let rest: Vec<f64> = v[q..].iter().map(|x| x.pick as f64).collect();
        println!(
            "{:<5} most {what}-leaning quarter went at pick {:.0} on average, the rest {:.0} (n {})",
            pos.name(),
            mean(&top),
            mean(&rest),
            v.len()
        );
    }
}
