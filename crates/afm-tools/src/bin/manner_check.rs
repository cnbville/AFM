//! How AI coaches talk, measured on one small world (coach voice,
//! `docs/design/coach-voice.md`, Cain's answer 2: AI coaches on the same
//! rules changes worlds). Builds a world, then reads: the words for every
//! AI head coach's manner by level; how each college player takes his head
//! coach's (the share by how he took it, and the season's trust change);
//! the staff offers' pull from it; the recruits' read of it in head-coach
//! points; and the firings for cause in the history against the real rate.
//! Usage: manner_check [scale] [history_years] [seed]

use std::collections::BTreeMap;

use afm_engine::world::coach::Employer;
use afm_engine::world::data::Division;
use afm_engine::world::folk::persona::manner::{self, Setting, TookAs};
use afm_engine::world::{Stage, World, WorldOptions};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scale: f64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.05);
    let years: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let seed: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(181);
    let t = std::time::Instant::now();
    let w =
        World::new(WorldOptions { seed, scale, history_years: years, engine_from: Some(2100), ..Default::default() });
    println!(
        "world: scale {scale}, {years} years of history, seed {seed}, built in {:.0} s",
        t.elapsed().as_secs_f64()
    );

    // 1. The words for each AI head coach's manner, by level.
    for (level, heads) in [
        (
            "FBS",
            w.programs.iter().filter(|p| p.division == Division::FBS).filter_map(|p| p.head_coach).collect::<Vec<_>>(),
        ),
        ("NFL", w.nfl.iter().filter_map(|t| t.head_coach).collect()),
        ("High school", w.high_schools.iter().filter_map(|h| h.coach).take(2000).collect()),
    ] {
        let mut words: BTreeMap<String, usize> = BTreeMap::new();
        for &c in &heads {
            let m = manner::ai(&w, c);
            *words.entry(manner::prototypes(&m, None).0.name().to_string()).or_default() += 1;
        }
        let n = heads.len().max(1) as f64;
        let list: Vec<String> = words.iter().map(|(k, v)| format!("{k} {:.0}%", 100.0 * *v as f64 / n)).collect();
        println!("{level} head coaches ({}): {}", heads.len(), list.join(", "));
    }

    // 2. How each college player takes his head coach's manner.
    let mut by: BTreeMap<String, usize> = BTreeMap::new();
    let (mut n, mut sum, mut lo, mut hi) = (0usize, 0.0f64, 0.0f64, 0.0f64);
    let mut fits: Vec<f64> = Vec::new();
    for p in w.programs.iter().filter(|p| p.division == Division::FBS) {
        let Some(c) = p.head_coach else { continue };
        let m = manner::ai(&w, c);
        for &i in &p.roster {
            let pp = &w.people[i];
            if pp.stage != Stage::College {
                continue;
            }
            let took = manner::land(&m, &manner::player_wants(pp), pp.trust, Some(m.heat), &Setting::default());
            *by.entry(format!("{:?}", took.as_)).or_default() += 1;
            let f = manner::ai_fit(&w, Some(c), i);
            fits.push(f);
            sum += f;
            lo = lo.min(f);
            hi = hi.max(f);
            n += 1;
        }
    }
    fits.sort_by(|a, b| a.total_cmp(b));
    let q = |x: f64| fits.get(((fits.len() as f64 - 1.0) * x) as usize).copied().unwrap_or(0.0);
    let list: Vec<String> =
        by.iter().map(|(k, v)| format!("{k} {:.1}%", 100.0 * *v as f64 / n.max(1) as f64)).collect();
    println!("FBS players ({n}) by how they take their head coach: {}", list.join(", "));
    println!(
        "  a season's trust change: mean {:+.4}, 10th {:+.4}, median {:+.4}, 90th {:+.4}, range {:+.4} to {:+.4} (the camp read's fair-role lift is +0.06)",
        sum / n.max(1) as f64,
        q(0.1),
        q(0.5),
        q(0.9),
        lo,
        hi
    );
    let abused = by.get(&format!("{:?}", TookAs::Hurt)).copied().unwrap_or(0)
        + by.get(&format!("{:?}", TookAs::ShutDown)).copied().unwrap_or(0)
        + by.get(&format!("{:?}", TookAs::PushesBack)).copied().unwrap_or(0);
    println!("  taking it as abuse: {:.1}%", 100.0 * abused as f64 / n.max(1) as f64);

    // 3. The staff offers' pull.
    let mut pulls: Vec<f64> = (0..w.coaches.len()).filter_map(|c| manner::offer_pull(&w, c).map(|x| x.0)).collect();
    pulls.sort_by(|a, b| a.total_cmp(b));
    let staff =
        w.coaches.iter().filter(|c| c.job.is_some_and(|j| j.role != afm_engine::world::coach::Role::HeadCoach)).count();
    let out = pulls.iter().filter(|&&x| x > 0.0).count();
    println!(
        "Assistants ({staff}): {} with a pull from how their head coach talks; {out} pushed toward the door (up to {:+.3}), {} kept (down to {:+.3}); a hot seat is up to +0.03",
        pulls.len(),
        pulls.last().copied().unwrap_or(0.0),
        pulls.len() - out,
        pulls.first().copied().unwrap_or(0.0)
    );

    // 4. Recruits' read of the head coach, in head-coach points.
    let heads: Vec<usize> =
        w.programs.iter().filter(|p| p.division == Division::FBS).filter_map(|p| p.head_coach).collect();
    let mut rf: Vec<f64> = Vec::new();
    for (k, p) in w.people.iter().filter(|p| p.stage == Stage::HighSchool).take(4000).enumerate() {
        if let Some(&c) = heads.get(k % heads.len().max(1)) {
            rf.push(manner::recruit_fit(&w, c, p));
        }
    }
    rf.sort_by(|a, b| a.total_cmp(b));
    let rq = |x: f64| rf.get(((rf.len() as f64 - 1.0) * x) as usize).copied().unwrap_or(0.0);
    println!(
        "Recruits ({}) on an FBS head coach's manner, in head-coach points (skill 1-20): 10th {:+.2}, median {:+.2}, 90th {:+.2}",
        rf.len(),
        rq(0.1),
        rq(0.5),
        rq(0.9)
    );

    // 5. Firings for cause in the history.
    let mut cause: BTreeMap<i32, Vec<String>> = BTreeMap::new();
    for c in &w.coaches {
        for s in c.history.iter().filter(|s| s.ended == "fired for cause") {
            let at = match s.employer {
                Employer::College(p) => format!("{} ({:?})", w.programs[p].real_name, w.programs[p].division),
                _ => "elsewhere".into(),
            };
            cause.entry(s.to).or_default().push(format!("{} at {at}", c.name));
        }
    }
    let total: usize = cause.values().map(Vec::len).sum();
    println!(
        "Fired for cause in {years} years: {total} ({:.2} a year; real: a handful of FBS coaches in twenty years, Kansas 2009, Illinois 2015, Maryland 2018)",
        total as f64 / years.max(1) as f64
    );
    for (y, list) in &cause {
        println!("  {y}: {}", list.join("; "));
    }
    println!("done in {:.0} s", t.elapsed().as_secs_f64());
}
