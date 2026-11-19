//! Your AD's motives (GR35): what he wants from this class, read from the
//! real world, never a menu. His makeup (`folk::cast`: ego, patience,
//! honesty, tight with money), his president, his donors' wants (the
//! rival beaten, the old high school's kid, NIL for the collective), the
//! program (its level and its last classes), this year's class in his
//! state (a generational prospect, the state's #1, the hype kid by
//! followers, a famous football family) and the fans.
//!
//! He holds you to one or two of them; the rest stay wishes that count
//! for you only when you meet them. Every weight is the build's pick
//! (proposed, not confirmed). The AD layer design (draft PR #386) adds his
//! ambition, his president and his donors in full; until then these read
//! his built traits, the boosters' wants and the program.

use super::goals::{self, Aim};
use crate::world::World;
use crate::world::data::{AcademicTier, Division};
use crate::world::folk::Team;
use crate::world::folk::cast::{self, CastRef, Role, Want};

/// One thing your AD wants from the class.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Motive {
    pub aim: Aim,
    /// Why, in his words.
    pub why: String,
    /// Whose want it is: "him", "the president", "Coleman" (a donor), "the fans".
    pub who: String,
    /// How much it weighs with him.
    pub weight: f32,
}

/// The state's #1 is the best the state has had in ten years: his
/// services' read tops every #1 of the last ten classes (still here or
/// gone), and he's a five-star.
fn generational(w: &World, st: usize, class: i32) -> Option<usize> {
    let top = crate::world::legacy::college::state_top(w, st, class, 1);
    let &i = top.first()?;
    let p = &w.people[i];
    if p.stars < 5 {
        return None;
    }
    let me = p.cal.service_read.get().unwrap_or(0.0);
    for c in class - 10..class {
        if let Some(&j) = crate::world::legacy::college::state_top(w, st, c, 1).first()
            && w.people[j].cal.service_read.get().unwrap_or(0.0) >= me
        {
            return None;
        }
    }
    // A #1 gone from the world (drafted, left) keeps his stars only: a
    // five-star #1 in the last ten classes ties him.
    let older = w
        .alumni
        .iter()
        .filter(|a| a.home_state == st && a.class_of >= class - 10 && a.class_of < class && a.stars >= 5)
        .count();
    (older <= 1).then_some(i)
}

/// The class's biggest following among the kids in your state or on your
/// offer list (100,000 or more).
fn hype_kid(w: &World, prog: usize, class: i32) -> Option<usize> {
    let st = w.programs[prog].state;
    w.people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.class_of == class && p.followers >= 100_000)
        .filter(|(_, p)| {
            p.home_state == st || p.cal.offers.contains(&prog) || p.cal.early_offers.iter().any(|x| x.0 == prog)
        })
        .max_by(|a, b| a.1.followers.cmp(&b.1.followers).then(b.0.cmp(&a.0)))
        .map(|x| x.0)
}

/// A three-star or better from a football family in your state: his father
/// played in the NFL (G8 adds the famous names; the bloodline is built).
fn famous(w: &World, prog: usize, class: i32) -> Option<usize> {
    let st = w.programs[prog].state;
    w.people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.class_of == class && p.home_state == st && p.stars >= 3)
        .filter(|(_, p)| p.family.bloodline == crate::world::background::Bloodline::NflPlayer)
        .max_by(|a, b| a.1.stars.cmp(&b.1.stars).then(b.0.cmp(&a.0)))
        .map(|x| x.0)
}

/// The best prospect from a high school in a class (three stars or better).
fn best_from(w: &World, h: usize, class: i32) -> Option<usize> {
    w.people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.class_of == class && p.hs == h && p.stars >= 3)
        .max_by(|a, b| a.1.stars.cmp(&b.1.stars).then(a.1.cmp_read(b.1)).then(b.0.cmp(&a.0)))
        .map(|x| x.0)
}

trait ReadCmp {
    fn cmp_read(&self, o: &Self) -> std::cmp::Ordering;
}

impl ReadCmp for crate::world::Person {
    fn cmp_read(&self, o: &Self) -> std::cmp::Ordering {
        self.cal.service_read.get().unwrap_or(0.0).total_cmp(&o.cal.service_read.get().unwrap_or(0.0))
    }
}

/// The program is its state's flagship: the most prestige among the
/// state's FBS programs.
pub fn flagship(w: &World, prog: usize) -> bool {
    let st = w.programs[prog].state;
    w.programs
        .iter()
        .enumerate()
        .filter(|(_, q)| q.state == st && q.division == Division::FBS)
        .max_by(|a, b| a.1.prestige.total_cmp(&b.1.prestige).then(b.0.cmp(&a.0)))
        .is_some_and(|x| x.0 == prog)
}

fn aim(goal: u8, n: u32, of: u32, class: i32) -> Aim {
    Aim { goal, n, of, class }
}

/// His conference-class bar, the built boss goal's: top-3 for a winner
/// (over .600 the last three years), top-6 otherwise.
pub fn conference_bar(w: &World, prog: usize) -> u32 {
    let last = crate::world::folk::facts::seasons(w, Team::College(prog), 3);
    let wp = if last.is_empty() {
        0.5
    } else {
        last.iter().map(|s| s.wins as f64 / (s.wins + s.losses).max(1) as f64).sum::<f64>() / last.len() as f64
    };
    if wp > 0.6 { 3 } else { 6 }
}

/// Your AD's motives for a class, weightiest first.
pub fn motives(w: &World, prog: usize, class: i32) -> Vec<Motive> {
    let team = Team::College(prog);
    let b = crate::world::folk::boss::boss(w, team);
    let pres = cast::get(w, CastRef::new(team, Role::President, 0));
    let boosters = cast::all(w, team, Role::Booster);
    let pr = &w.programs[prog];
    let st = pr.state;
    let state = crate::world::folk::facts::state_name(crate::world::geo::STATE_CENTERS[st.min(50)].0);
    let power = pr.division == Division::FBS && pr.power_conference(w.year);
    let flag = flagship(w, prog);
    let mut v: Vec<Motive> = Vec::new();
    let name_of = |i: usize| w.people[i].name.clone();
    let school_of = |i: usize| w.high_schools.get(w.people[i].hs).map_or("his school".to_string(), |h| h.name.clone());

    // The program: his built recruiting goal by class score.
    let n = conference_bar(w, prog);
    v.push(Motive {
        aim: aim(2, n, 0, class),
        why: "The rankings are how the boosters judge a staff in February.".into(),
        who: "him".into(),
        weight: 1.0 + 0.03 * (b.patience as f32 - 10.0).min(0.0).abs(),
    });
    // An ego or an ambitious man wants the national rankings at a power program.
    if power && b.ego >= 11.0 {
        let n = if pr.prestige >= 0.85 {
            5
        } else if pr.prestige >= 0.7 {
            15
        } else {
            30
        };
        v.push(Motive {
            aim: aim(1, n, 0, class),
            why: format!("A top-{n} class nationally. People nationally read those lists."),
            who: "him".into(),
            weight: 0.6 + 0.05 * (b.ego as f32 - 10.0),
        });
    }
    // This year's class in his state.
    if let Some(i) = generational(w, st, class) {
        v.push(Motive {
            aim: aim(22, 0, w.people[i].player.id, class),
            why: format!(
                "Everybody in this state knows the kid from {}. Best player {state} has had in ten years. If he leaves, I hear about it at every lunch until I retire.",
                school_of(i)
            ),
            who: "him".into(),
            weight: 1.3 + if flag { 0.3 } else { 0.0 },
        });
    }
    if flag {
        let top = crate::world::legacy::college::state_top(w, st, class, 1);
        if top.first().is_some_and(|&i| w.people[i].stars >= 4) {
            v.push(Motive {
                aim: aim(11, 0, 0, class),
                why: format!(
                    "The president wants the state's best kept at home: {} is the state's #1.",
                    name_of(top[0])
                ),
                who: "the president".into(),
                weight: 0.8 + 0.02 * (pres.ego as f32 - 10.0),
            });
        }
        let n = if pr.prestige >= 0.8 { 7 } else { 5 };
        v.push(Motive {
            aim: aim(14, n, 0, class),
            why: format!("Keep {n} of the state's top 10 home. That's the job at the flagship."),
            who: "the president".into(),
            weight: 0.7,
        });
    }
    if let Some(i) = hype_kid(w, prog, class) {
        let fol = w.people[i].followers;
        let collective =
            boosters.iter().find(|c| c.booster.as_ref().is_some_and(|x| x.wants.contains(&Want::NilDeals)));
        v.push(Motive {
            aim: aim(23, 0, w.people[i].player.id, class),
            why: format!(
                "{} has {} followers. {} wants him; the stage is part of his business, and ours.",
                name_of(i),
                fmt_followers(fol),
                collective.map_or("The collective".to_string(), |c| c.first().to_string())
            ),
            who: collective.map_or("the collective".to_string(), |c| c.name.clone()),
            weight: 0.55 + if b.ego >= 12.0 { 0.2 } else { 0.0 } + (fol as f32 / 2_000_000.0).min(0.4),
        });
    }
    if let Some(i) = famous(w, prog, class) {
        v.push(Motive {
            aim: aim(24, 0, w.people[i].player.id, class),
            why: format!("{}'s father played in the league. The fans know the name.", name_of(i)),
            who: "the fans".into(),
            weight: 0.45,
        });
    }
    // His donors.
    for c in &boosters {
        let Some(info) = c.booster.as_ref() else { continue };
        if info.wants.contains(&Want::OldSchool)
            && let Some(h) = info.old_school
            && let Some(i) = best_from(w, h, class)
        {
            v.push(Motive {
                aim: aim(55, 0, w.people[i].player.id, class),
                why: format!(
                    "{} wants {} from {}, his old high school. {} gives too much to say no to lightly.",
                    c.name,
                    name_of(i),
                    school_of(i),
                    c.first()
                ),
                who: c.name.clone(),
                weight: 0.5 + if info.big { 0.25 } else { 0.0 },
            });
        }
        if info.wants.contains(&Want::BeatRival)
            && let Some(r) = goals::rival(w, prog)
            && !v.iter().any(|m| m.aim.goal == 3)
        {
            v.push(Motive {
                aim: aim(3, 0, r as u32, class),
                why: format!("Beat {}'s class. {} hears about the rival every Saturday.", w.program_name(r), c.first()),
                who: c.name.clone(),
                weight: 0.55 + if info.big { 0.2 } else { 0.0 },
            });
        }
    }
    // His makeup and his president.
    if b.honesty >= 13.0 {
        v.push(Motive {
            aim: aim(53, 0, 0, class),
            why: "A clean class. One arrest in the paper is a week of phone calls for me.".into(),
            who: "him".into(),
            weight: 0.4 + 0.03 * (b.honesty as f32 - 13.0),
        });
    }
    let academic = matches!(pr.academic_tier, AcademicTier::Aau | AcademicTier::R1);
    let short = goals::read(w, prog, &aim(51, 0, 0, class - 1), class - 1).share < 1.0;
    if academic || short {
        v.push(Motive {
            aim: aim(51, 0, 0, class),
            why: if short {
                "Kids who stay eligible. The president asked about the last ones.".into()
            } else {
                "Every one of them qualifies. The president reads the academic report himself.".into()
            },
            who: "the president".into(),
            weight: 0.35 + if short { 0.3 } else { 0.0 } + if academic { 0.1 } else { 0.0 },
        });
    }
    if let Some(area) = goals::campus_area(w, prog) {
        let schools = (0..w.high_schools.len()).filter(|&h| w.high_schools[h].ground_area == area).count();
        if schools >= 40 {
            v.push(Motive {
                aim: aim(17, 1, 0, class),
                why: format!(
                    "Get me somebody from {} the paper writes about.",
                    crate::world::ground::areas::area(area).name
                ),
                who: "him".into(),
                weight: 0.4,
            });
        }
    }
    if b.tight >= 13.0 && w.year >= 2021 {
        v.push(Motive {
            aim: aim(50, if power { 1_000_000 } else { 250_000 }, 0, class),
            why: "A class people follow sells tickets and sponsorships.".into(),
            who: "the fans".into(),
            weight: 0.35,
        });
    }
    v.sort_by(|a, b| b.weight.total_cmp(&a.weight).then(a.aim.goal.cmp(&b.aim.goal)));
    v.dedup_by(|a, b| a.aim.goal == b.aim.goal && a.aim.of == b.aim.of);
    v
}

/// How many of his motives he holds you to: two for a meddler or an
/// impatient man, one otherwise (GR35).
pub fn held(w: &World, prog: usize) -> usize {
    let b = crate::world::folk::boss::boss(w, Team::College(prog));
    if b.meddling >= 12.0 || b.patience <= 8.0 { 2 } else { 1 }
}

fn fmt_followers(n: u32) -> String {
    if n >= 1_000_000 { format!("{:.1} million", n as f64 / 1e6) } else { format!("{},{:03}", n / 1000, n % 1000) }
}
