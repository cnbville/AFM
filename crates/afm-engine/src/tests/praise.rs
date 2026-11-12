//! Praise in a one-on-one (playtest bug F2, Cain 2026-11-12: "saying 'I see
//! that hard work you're putting in' constantly drops mood and trust. It
//! hasn't lifted spirits once"). The line is about his work, so his work
//! earns it: a hard worker hears it whatever the depth chart says, a
//! player who coasts spots it as empty, and a starter's place counts from
//! your chart today as well as from last season.

use super::legacy::{seat, you};
use crate::player::{PlayingTime, Trait};
use crate::world::folk::Team;
use crate::world::folk::scene;
use crate::world::folk::talks::{self, Subject};
use crate::world::seat::Job;
use crate::world::{Stage, World, WorldOptions};

/// The praise line said once in a fresh one-on-one, in a tone (0 calm,
/// 1 warm as the line is written, 2 firm, 3 fiery): (trust, mood) as the
/// talk screen shows them. He is the same man afterwards, so every talk
/// starts level; only the day he's having differs.
fn praise(w: &mut World, i: usize, subject: Subject, tone: u32) -> (f32, f32) {
    let id = w.people[i].player.id;
    let (trust, mood, lean) = (w.people[i].trust, w.people[i].mood, w.people[i].cal.lean);
    w.folk.mine.hours = 0.0;
    let sid = talks::start_talk(w, id, subject).unwrap();
    let s = scene::open(w, sid).unwrap();
    let code = s.picks.iter().find(|x| x.mv == "Praise").expect("praise is on the table").code;
    let s = scene::answer(w, sid, code - code % 10 + tone).unwrap();
    w.folk.scenes.retain(|x| x.id != sid);
    let p = &mut w.people[i];
    p.trust = trust;
    p.mood = mood;
    p.cal.lean = lean;
    (s.f[0], s.f[1])
}

/// Your college program with three level-headed sophomore backups on it,
/// alike in everything but what a test sets.
fn three() -> (World, Vec<usize>) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let v: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(3).collect();
    for &i in &v {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.last_role = Some(PlayingTime::Backup);
        q.trust = 0.5;
        q.mood = 0.0;
        q.mood_why = 0;
        q.bg.stability = 10.0;
        q.player.personality.competitiveness = 10.0;
        q.player.personality.work_ethic = 10.0;
        for t in Trait::ALL {
            q.player.personality.traits.set(t, 10.0);
        }
        w.programs[p].roster.push(i);
    }
    (w, v)
}

fn work(w: &mut World, i: usize, ethic: f64, professionalism: f64) {
    let me = &mut w.people[i].player.personality;
    me.work_ethic = ethic;
    me.traits.set(Trait::Professionalism, professionalism);
}

/// A hundred and twenty talks: (mean trust, mean mood, talks where either
/// fell).
fn many(w: &mut World, i: usize) -> (f32, f32, u32) {
    let v: Vec<(f32, f32)> = (0..120).map(|_| praise(w, i, Subject::Role, 1)).collect();
    let n = v.len() as f32;
    (
        v.iter().map(|x| x.0).sum::<f32>() / n,
        v.iter().map(|x| x.1).sum::<f32>() / n,
        v.iter().filter(|x| x.0 < 0.0 || x.1 < 0.0).count() as u32,
    )
}

#[test]
fn praise_lands_by_the_work_he_puts_in() {
    let (mut w, v) = three();
    let (hard, plain, coasts) = (v[0], v[1], v[2]);
    work(&mut w, hard, 17.0, 15.0);
    work(&mut w, plain, 10.0, 10.0);
    work(&mut w, coasts, 3.0, 5.0);
    let (th, tp, tc) = (talks::praise_truth(&w, hard), talks::praise_truth(&w, plain), talks::praise_truth(&w, coasts));
    assert!(th > 0.95 && (tp - 0.5).abs() < 1e-6 && tc < 0.01, "how true it rings: {th} {tp} {tc}");

    // A backup who works: none of them started last season, and before the
    // fix all three heard it as empty.
    let (dt, dm, fell) = many(&mut w, hard);
    assert!(dt > 0.03 && dm > 0.06, "a hard worker hears it: trust {dt:+.3}, mood {dm:+.3}");
    assert!(fell <= 12, "and it nearly always lifts him: fell in {fell} of 120");
    // An everyday worker: a small lift, nothing lost.
    let (pt, pm, _) = many(&mut w, plain);
    assert!(pt > 0.0 && pm > 0.03, "an everyday worker: trust {pt:+.3}, mood {pm:+.3}");
    assert!(pt < dt && pm < dm, "less than the hard worker");
    // A player who coasts spots it: no trust gained, the smallest lift.
    let (ct, cm, _) = many(&mut w, coasts);
    assert!(ct < 0.01 && ct < pt && cm < pm, "empty praise is spotted: trust {ct:+.3}, mood {cm:+.3}");
    // Said on what is really on his mind, it costs trust with him.
    w.people[coasts].mood_why = crate::world::people::Why::Role as u8;
    let (ct, _, _) = many(&mut w, coasts);
    assert!(ct < 0.0, "on his worry, empty praise costs trust: {ct:+.3}");
}

#[test]
fn a_starters_place_counts_from_your_chart_today() {
    let (mut w, v) = three();
    let (a, b) = (v[0], v[1]);
    let before = talks::praise_truth(&w, a);
    // First at his position on your depth chart, a backup last season.
    let id = w.people[a].player.id;
    let pos = w.people[a].player.position;
    w.seat_depth.push((pos, vec![id]));
    let now = talks::praise_truth(&w, a);
    assert!((now - before - 0.25).abs() < 1e-6, "your starter today: {before} to {now}");
    // The same as a starter of last season.
    w.people[b].last_role = Some(PlayingTime::Starter);
    assert_eq!(talks::praise_truth(&w, b), now);
    // A place on the chart is proof, never the whole of it: a starter who
    // coasts still hears less than a backup who works.
    work(&mut w, a, 4.0, 6.0);
    work(&mut w, v[2], 16.0, 14.0);
    assert!(talks::praise_truth(&w, a) < talks::praise_truth(&w, v[2]));
}

// ---------------------------------------------------------------------------
// The measure, on real rosters.

/// One small Jan 1 world, built once and written to a file.
fn fresh() -> World {
    static FILE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    let file = FILE.get_or_init(|| {
        let w = World::new(WorldOptions {
            seed: 76,
            scale: 0.02,
            history_years: 1,
            engine_from: Some(2100),
            stop_jan1: true,
            ..Default::default()
        });
        let f = std::env::temp_dir().join(format!("afm-praise-{}.afmworld", std::process::id()));
        crate::save::write_world(&w, &f, "test", "", None).unwrap();
        f
    });
    let mut w = crate::save::read_world(file).unwrap();
    w.after_load();
    w
}

#[derive(Default, Clone, Copy)]
struct Cell {
    n: u32,
    dt: f64,
    dm: f64,
    trust_down: u32,
    mood_down: u32,
    trust_up: u32,
    mood_up: u32,
    good: u32,
    bad: u32,
}

impl Cell {
    fn add(&mut self, (dt, dm): (f32, f32)) {
        self.n += 1;
        self.dt += dt as f64;
        self.dm += dm as f64;
        // The cuts the talk screen's words use (`went_words`): a little
        // less trust, his mood dips; and the cuts for his reply.
        self.trust_down += (dt < -0.015) as u32;
        self.mood_down += (dm < -0.02) as u32;
        self.trust_up += (dt > 0.015) as u32;
        self.mood_up += (dm > 0.025) as u32;
        self.good += (dt + dm > 0.06) as u32;
        self.bad += (dt + dm < -0.03) as u32;
    }
    fn row(&self, name: &str) -> String {
        let n = self.n.max(1) as f64;
        let pc = |x: u32| 100.0 * x as f64 / n;
        format!(
            "{name:<28} n {:>4}  trust {:+.3}  mood {:+.3}  trust up {:>3.0}% down {:>3.0}%  mood up {:>3.0}% down {:>3.0}%  his reply good {:>3.0}% bad {:>3.0}%",
            self.n,
            self.dt / n,
            self.dm / n,
            pc(self.trust_up),
            pc(self.trust_down),
            pc(self.mood_up),
            pc(self.mood_down),
            pc(self.good),
            pc(self.bad),
        )
    }
}

/// The measure behind the fix: every player on your roster hears the
/// praise line in a one-on-one, six talks each (three warm on his role,
/// then calm, firm and fiery on what is really on his mind), on a Jan 1 job
/// at each level, and it prints what the talk screen would show. It builds
/// a world, so it runs by hand:
/// `cargo test -p afm-engine --lib praise_measured -- --ignored --nocapture`.
#[test]
#[ignore]
fn praise_measured() {
    for level in ["college", "nfl", "hs"] {
        let mut w = fresh();
        let job = match level {
            "nfl" => Job::Nfl(
                (0..w.clubs.len())
                    .filter(|&t| crate::world::franchise::active_in(&w, t, w.year))
                    .max_by_key(|&t| w.clubs[t].roster.len())
                    .unwrap(),
            ),
            "college" => Job::College(
                (0..w.programs.len())
                    .filter(|&p| w.programs[p].division == crate::world::data::Division::FBS)
                    .max_by_key(|&p| w.programs[p].roster.len())
                    .unwrap(),
            ),
            _ => Job::HighSchool((0..w.high_schools.len()).max_by_key(|&h| w.high_schools[h].roster.len()).unwrap()),
        };
        w.start_career(&you(), job, 7);
        let roster = Team::seat(&w).unwrap().roster(&w);
        let names = [
            "all, warm as written",
            "started last season",
            "did not start",
            "  of them: hard workers",
            "  of them: everyday",
            "  of them: coasting",
            "calm tone",
            "firm tone",
            "fiery tone",
        ];
        let mut cells = [Cell::default(); 9];
        let mut starters = 0;
        for &i in &roster {
            let starter = w.people[i].last_role == Some(PlayingTime::Starter);
            starters += starter as u32;
            let truth = talks::praise_truth(&w, i);
            let (real, _) = talks::concern(&w, i);
            for _ in 0..3 {
                let x = praise(&mut w, i, Subject::Role, 1);
                cells[0].add(x);
                cells[if starter { 1 } else { 2 }].add(x);
                if !starter {
                    cells[if truth >= 0.7 {
                        3
                    } else if truth >= 0.3 {
                        4
                    } else {
                        5
                    }]
                    .add(x);
                }
            }
            for (k, tone) in [0, 2, 3].into_iter().enumerate() {
                cells[6 + k].add(praise(&mut w, i, real, tone));
            }
        }
        println!("\n{level}: {} players, {starters} started last season", roster.len());
        for (c, name) in cells.iter().zip(names) {
            println!("  {}", c.row(name));
        }
    }
}
