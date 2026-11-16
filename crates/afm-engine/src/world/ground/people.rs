//! Who is a ground person, and his makeup (GR62, part B). At college the
//! people on your ground are high school head coaches and coordinators
//! (built coaches, `world/coach.rs`), trainers and 7v7 directors (former
//! players, `world/former.rs`) and each school's counselor (made per
//! school). Families, junior college and prep school coaches and other
//! college staffs join in lane G7.
//!
//! A person's makeup is read from what the world already has (a coach's
//! loyalty) and the world's seed, never from its random stream, so he is
//! the same every time. Nobody sees these numbers: you learn them from his
//! record and your staff's read.

use crate::world::World;
use crate::world::coach::{Employer, Role};

/// A person on someone's ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Who {
    /// A high school head coach or coordinator, by coach id.
    Coach(u32),
    /// A private trainer (`Life::Trainer`), by former player id.
    Trainer(u32),
    /// A 7v7 director and camp man (`Life::Camp`), by former player id.
    SevenOnSeven(u32),
    /// A school's counselor, by high school id.
    Counselor(u32),
}

/// Who holds a memory (GR29): a program (its ties stay when its men go)
/// or a coach (his own ties, which go with him).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Holder {
    Program(u32),
    Coach(u32),
}

/// What a person is like with staffs (hidden).
#[derive(Clone, Copy, Debug)]
pub struct Makeup {
    /// How warmly he takes visitors, 0.7-1.3 (by his nature).
    pub warmth: f32,
    /// Loyal (1) or self-serving (0).
    pub loyal: f32,
    /// How many years a big moment holds at full weight, 3-5.
    pub grudge: f32,
    /// His hour: when he writes or calls (local minutes after midnight).
    pub hour: u16,
}

fn salt(who: Who) -> [u64; 2] {
    match who {
        Who::Coach(c) => [1, c as u64],
        Who::Trainer(f) => [2, f as u64],
        Who::SevenOnSeven(f) => [3, f as u64],
        Who::Counselor(h) => [4, h as u64],
    }
}

/// His makeup.
pub fn makeup(w: &World, who: Who) -> Makeup {
    let s = salt(who);
    let u = |k: u64| super::unit(w.options.seed, &[0x6E0D, s[0], s[1], k]);
    // A coach's loyalty is built (1-20); everyone else's comes from the seed.
    let loyal = match who {
        Who::Coach(c) => w.coaches.get(c as usize).map_or(0.5, |c| ((c.personality.loyalty - 1.0) / 19.0) as f32),
        _ => u(1) as f32,
    };
    // Warmth leans on loyalty: a loyal man warms to the staff that shows up.
    let warmth = (0.7 + 0.6 * (0.6 * u(2) as f32 + 0.4 * loyal)).clamp(0.7, 1.3);
    let grudge = 3.0 + 2.0 * u(3) as f32;
    // A high school coach writes in his planning period after lunch, a
    // counselor in the school morning, trainers and 7v7 men in the evening.
    let hour = match who {
        Who::Coach(_) => 13 * 60 + (u(4) * 50.0) as u16,
        Who::Counselor(_) => 10 * 60 + (u(4) * 60.0) as u16,
        Who::Trainer(_) | Who::SevenOnSeven(_) => 19 * 60 + (u(4) * 90.0) as u16,
    };
    Makeup { warmth, loyal, grudge, hour }
}

/// The high school a coach works at now, and his role there.
pub fn coach_school(w: &World, c: usize) -> Option<(usize, Role)> {
    let j = w.coaches.get(c)?.job?;
    match j.employer {
        Employer::HighSchool(h) => Some((h, j.role)),
        _ => None,
    }
}

/// The school a person works at now.
pub fn school(w: &World, who: Who) -> Option<usize> {
    match who {
        Who::Coach(c) => coach_school(w, c as usize).map(|s| s.0),
        Who::Counselor(h) => Some(h as usize),
        Who::Trainer(_) | Who::SevenOnSeven(_) => None,
    }
}

/// A school's head coach, the man who holds its kids (GR27).
pub fn head_coach_of(w: &World, hs: usize) -> Option<Who> {
    w.high_schools.get(hs)?.coach.map(|c| Who::Coach(c as u32))
}

/// Everyone at a school a college staff deals with: the head coach, the
/// coordinators who sit in on visits, the counselor.
pub fn at_school(w: &World, hs: usize) -> Vec<Who> {
    let Some(h) = w.high_schools.get(hs) else { return Vec::new() };
    let mut v: Vec<Who> = [h.coach, h.oc, h.dc].into_iter().flatten().map(|c| Who::Coach(c as u32)).collect();
    v.push(Who::Counselor(hs as u32));
    v
}

/// His name (a counselor has none yet: lane G7 makes counselors people).
pub fn name(w: &World, who: Who) -> String {
    match who {
        Who::Coach(c) => w.coaches.get(c as usize).map_or_else(|| "A coach".into(), |c| c.name.clone()),
        Who::Trainer(f) | Who::SevenOnSeven(f) => {
            w.formers.list.iter().find(|x| x.id == f).map_or_else(|| "A trainer".into(), |x| x.name.clone())
        }
        Who::Counselor(h) => format!("The counselor at {}", w.school_name(h as usize)),
    }
}

/// What he is, in words ("head coach at Destrehan").
pub fn role(w: &World, who: Who) -> String {
    match who {
        Who::Coach(c) => match coach_school(w, c as usize) {
            Some((h, r)) => format!("{} at {}", r.name(), w.school_name(h)),
            None => "coach".into(),
        },
        Who::Trainer(_) => "private trainer".into(),
        Who::SevenOnSeven(_) => "7v7 director".into(),
        Who::Counselor(h) => format!("counselor at {}", w.school_name(h as usize)),
    }
}

/// A holder's people skill (GR23): 0.6 + 0.04 × his relationships (1-20);
/// a program on its own (its personnel staff) is an average man.
pub fn skill(w: &World, holder: Holder) -> f32 {
    match holder {
        Holder::Coach(c) => w.coaches.get(c as usize).map_or(1.0, |c| (0.6 + 0.04 * c.ability.relationships) as f32),
        Holder::Program(_) => 1.0,
    }
}
