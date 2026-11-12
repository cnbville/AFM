//! Praise in a one-on-one (playtest bug F2, Cain 2026-11-12: "saying 'I see
//! that hard work you're putting in' constantly drops mood and trust. It
//! hasn't lifted spirits once").

use super::legacy::you;
use crate::world::folk::Team;
use crate::world::folk::scene;
use crate::world::folk::talks::{self, Subject};
use crate::world::seat::Job;
use crate::world::{World, WorldOptions};

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
    both_down: u32,
    trust_up: u32,
    mood_up: u32,
    good: u32,
    bad: u32,
}

impl Cell {
    fn add(&mut self, dt: f32, dm: f32) {
        self.n += 1;
        self.dt += dt as f64;
        self.dm += dm as f64;
        // The cuts `went_words` shows: a little less trust, his mood dips.
        let (td, md) = (dt < -0.015, dm < -0.02);
        self.trust_down += td as u32;
        self.mood_down += md as u32;
        self.both_down += (td && md) as u32;
        self.trust_up += (dt > 0.015) as u32;
        self.mood_up += (dm > 0.025) as u32;
        self.good += (dt + dm > 0.06) as u32;
        self.bad += (dt + dm < -0.03) as u32;
    }
    fn row(&self, name: &str) -> String {
        let n = self.n.max(1) as f64;
        let pc = |x: u32| 100.0 * x as f64 / n;
        format!(
            "{name:<34} n {:>5}  trust {:+.3}  mood {:+.3}  trust up {:>3.0}% down {:>3.0}%  mood up {:>3.0}% down {:>3.0}%  both down {:>3.0}%  reply good {:>3.0}% bad {:>3.0}%",
            self.n,
            self.dt / n,
            self.dm / n,
            pc(self.trust_up),
            pc(self.trust_down),
            pc(self.mood_up),
            pc(self.mood_down),
            pc(self.both_down),
            pc(self.good),
            pc(self.bad),
        )
    }
}

/// The measure: every player on your roster hears the praise line in a
/// one-on-one, six talks each, on a Jan 1 job at each level. Prints what a
/// coach sees. Builds a world, so it runs by hand:
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
        let names = ["all", "earned", "not earned", "on his worry", "off his worry", "calm tone", "firm tone", "fiery tone"];
        let mut cells = [Cell::default(); 8];
        let (mut starters, mut over62, mut earned_n) = (0, 0, 0);
        let (mut trust, mut mood) = (0.0f64, 0.0f64);
        for &i in &roster {
            let id = w.people[i].player.id;
            let p = &w.people[i];
            let starter = p.last_role == Some(crate::player::PlayingTime::Starter);
            let good = p.last_performance.unwrap_or(0.0) > 62.0;
            let earned = talks::praise_earned(&w, i);
            starters += starter as u32;
            over62 += good as u32;
            earned_n += earned as u32;
            trust += p.trust as f64;
            mood += p.mood as f64;
            let (real, _) = talks::concern(&w, i);
            for k in 0..6u32 {
                // Three talks on his role (the page's first subject), three
                // on what is really on his mind.
                let subject = if k < 3 { Subject::Role } else { real };
                let on = subject == real;
                let (t0, m0, lean) = (w.people[i].trust, w.people[i].mood, w.people[i].cal.lean);
                w.folk.mine.hours = 0.0;
                let sid = talks::start_talk(&mut w, id, subject).unwrap();
                let s = scene::open(&mut w, sid).unwrap();
                let Some(code) = s.picks.iter().find(|x| x.mv == "Praise").map(|x| x.code) else { continue };
                // The line's own tone (warm), then the three others.
                let tone = [1, 1, 1, 0, 2, 3][k as usize % 6];
                let code = code - code % 10 + tone;
                let s = scene::answer(&mut w, sid, code).unwrap();
                let (dt, dm) = (s.f[0], s.f[1]);
                if tone == 1 {
                    cells[0].add(dt, dm);
                    cells[if earned { 1 } else { 2 }].add(dt, dm);
                    cells[if on { 3 } else { 4 }].add(dt, dm);
                } else {
                    cells[5 + [0, 0, 1, 2][tone as usize] as usize].add(dt, dm);
                }
                // Each talk starts from the same man.
                w.folk.scenes.retain(|x| x.id != sid);
                let p = &mut w.people[i];
                p.trust = t0;
                p.mood = m0;
                p.cal.lean = lean;
            }
        }
        let n = roster.len().max(1) as f64;
        println!(
            "\n{level}: {} players, {} starters last season, {} played above 62, {} count as earned; trust {:.2}, mood {:+.2}",
            roster.len(),
            starters,
            over62,
            earned_n,
            trust / n,
            mood / n
        );
        for (c, name) in cells.iter().zip(names) {
            println!("  {}", c.row(name));
        }
    }
}
