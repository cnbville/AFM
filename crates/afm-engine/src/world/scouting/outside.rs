//! Athletes outside football (scouting update SK65): kids who play other
//! sports or come from other countries, for a college or an NFL club to
//! find, with real conversion rates: most don't make it.
//!
//! - **NFL.** College basketball forwards (Antonio Gates played basketball
//!   at Kent State and went undrafted in 2003), rugby players (Jordan
//!   Mailata came from rugby league in 2018, through the International
//!   Player Pathway from 2017), sprinters, Australian punters and European
//!   kickers. Each spring after the draft and through the summer your pro
//!   staff hears of a handful; one you sign joins your practice squad.
//! - **College.** Australian punters through ProKick Australia (from 2007),
//!   soccer kickers, and basketball players who never played football, for
//!   a scholarship from November to August; one you sign joins your roster
//!   as a freshman.
//! - **High school.** The athletes in your own building (SK22).
//!
//! Each is drawn by key from your seat and the year, never stored until you
//! sign him. The athlete he truly is stays hidden: you see your staff's
//! read of him as a football player, wide, since nobody has seen him play
//! the game. Proposed, not confirmed: about six a year at a club, four at
//! a college; a converted athlete's football rating at an NFL club centred
//! near 46 (a punter or kicker 52), a ProKick punter near 52 on the college
//! scale, both with a long tail up.

use super::readers::{self, Staff};
use crate::positions::Position;
use crate::rng::Rng;
use crate::world::World;
use crate::world::ethnicity::Ethnicity;

/// Where an athlete comes from: the sport, the country and the spots.
struct Source {
    sport: &'static str,
    /// The families he comes from, by share.
    eth: &'static [(Ethnicity, f64)],
    from: &'static str,
    spots: &'static [Position],
    /// His football rating's centre and spread, and his age range.
    centre: f64,
    spread: f64,
    ages: (u32, u32),
    /// The first year this road was open.
    since: i32,
}

const NFL: [Source; 6] = [
    Source {
        eth: &[(Ethnicity::Black, 0.75), (Ethnicity::White, 0.25)],
        sport: "college basketball",
        from: "the United States",
        spots: &[Position::TE, Position::EDGE],
        centre: 45.0,
        spread: 7.0,
        ages: (22, 24),
        since: 1960,
    },
    Source {
        eth: &[(Ethnicity::White, 0.55), (Ethnicity::Polynesian, 0.45)],
        sport: "rugby league",
        from: "Australia",
        spots: &[Position::OT, Position::EDGE, Position::RB],
        centre: 45.0,
        spread: 6.0,
        ages: (21, 25),
        since: 1990,
    },
    Source {
        eth: &[(Ethnicity::White, 0.8), (Ethnicity::Black, 0.2)],
        sport: "rugby union",
        from: "England",
        spots: &[Position::RB, Position::EDGE, Position::LB],
        centre: 44.0,
        spread: 6.0,
        ages: (21, 25),
        since: 1990,
    },
    Source {
        eth: &[(Ethnicity::Black, 0.85), (Ethnicity::White, 0.15)],
        sport: "track (sprints)",
        from: "the United States",
        spots: &[Position::WR, Position::CB],
        centre: 44.0,
        spread: 6.0,
        ages: (22, 25),
        since: 1960,
    },
    Source {
        eth: &[(Ethnicity::White, 1.0)],
        sport: "Australian rules football",
        from: "Australia",
        spots: &[Position::P],
        centre: 52.0,
        spread: 6.0,
        ages: (24, 29),
        since: 1985,
    },
    Source {
        eth: &[(Ethnicity::White, 0.9), (Ethnicity::Black, 0.1)],
        sport: "soccer",
        from: "Europe",
        spots: &[Position::K],
        centre: 50.0,
        spread: 6.0,
        ages: (21, 26),
        since: 1960,
    },
];

const COLLEGE: [Source; 3] = [
    Source {
        eth: &[(Ethnicity::White, 1.0)],
        sport: "Australian rules football, through ProKick Australia",
        from: "Australia",
        spots: &[Position::P],
        centre: 52.0,
        spread: 6.0,
        ages: (20, 24),
        since: 2007,
    },
    Source {
        eth: &[(Ethnicity::White, 0.55), (Ethnicity::Hispanic, 0.45)],
        sport: "soccer",
        from: "the United States",
        spots: &[Position::K],
        centre: 46.0,
        spread: 6.0,
        ages: (18, 19),
        since: 1960,
    },
    Source {
        eth: &[(Ethnicity::Black, 0.75), (Ethnicity::White, 0.25)],
        sport: "basketball",
        from: "the United States",
        spots: &[Position::TE, Position::EDGE],
        centre: 40.0,
        spread: 7.0,
        ages: (18, 19),
        since: 1960,
    },
];

/// One athlete your staff heard of.
#[derive(Clone, Debug)]
pub struct Outsider {
    pub key: u64,
    pub name: String,
    pub sport: &'static str,
    pub from: &'static str,
    pub pos: Position,
    pub age: u32,
    /// The football player he truly is (hidden; screens show `read`).
    pub(crate) truth: f64,
    /// Your staff's read, wide: nobody has seen him play football.
    pub read: (u8, u8),
    pub spotted_by: String,
}

/// Your seat and the window: NFL May 1 to August 31, college November 1 to
/// August 31.
fn window(w: &World) -> Option<(Staff, &'static [Source])> {
    let staff = readers::seat_staff(w)?;
    let m = w.today.parts().1;
    match staff {
        Staff::Club(_) if (5..=8).contains(&m) => Some((staff, &NFL)),
        Staff::Program(_) if m >= 11 || m <= 8 => Some((staff, &COLLEGE)),
        _ => None,
    }
}

/// The athletes your staff has heard of this year.
pub fn list(w: &World) -> Vec<Outsider> {
    let Some((staff, roads)) = window(w) else { return Vec::new() };
    let year = w.dates.season;
    let n = if matches!(staff, Staff::Club(_)) { 6 } else { 4 };
    let eye = readers::you(w).map_or(10.0, |c| w.coaches[c].evaluation());
    let base = readers::staff_key(staff) ^ (year as u64) << 24 ^ 0x0075_1DE5;
    let mut rng = Rng::new((base ^ base >> 32) as u32 ^ w.options.seed);
    let open: Vec<&Source> = roads.iter().filter(|r| year >= r.since).collect();
    let mut v = Vec::new();
    for k in 0..n {
        let r = open[rng.int(0, open.len() as i32 - 1) as usize];
        let pos = r.spots[rng.int(0, r.spots.len() as i32 - 1) as usize];
        let age = rng.int(r.ages.0 as i32, r.ages.1 as i32) as u32;
        // A long tail up: most never make it, a few become players.
        let z = rng.normal(0.0, 1.0);
        let truth = (r.centre + r.spread * if z > 1.2 { 1.2 + 1.8 * (z - 1.2) } else { z }).clamp(30.0, 80.0);
        let miss = 6.0 * super::file::sharp(eye) * rng.normal(0.0, 1.0);
        let half = 6.0 + 4.0 * super::file::sharp(eye);
        let c = truth + miss;
        let read = (((c - half).floor()).clamp(1.0, 99.0) as u8, ((c + half).ceil()).clamp(1.0, 99.0) as u8);
        let key = base ^ (k as u64) << 8 ^ rng.next_u32() as u64;
        if w.scouting.outside.contains(&key) {
            continue;
        }
        let eth = r.eth[rng.weighted(&r.eth.iter().enumerate().map(|(k, e)| (k, e.1)).collect::<Vec<_>>())].0;
        let name = crate::world::names::person_name(eth, year - age as i32, &mut rng);
        let spotted_by = match staff {
            Staff::Club(_) => "Your pro personnel staff".to_string(),
            _ => if pos == Position::P || pos == Position::K {
                "Your special teams coordinator"
            } else {
                "Your recruiting staff"
            }
            .to_string(),
        };
        v.push(Outsider { key, name, sport: r.sport, from: r.from, pos, age, truth, read, spotted_by });
    }
    v
}

/// You sign one: a club's practice squad, or your college roster as a
/// freshman.
pub fn sign(w: &mut World, key: u64) -> Result<String, String> {
    let a = list(w)
        .into_iter()
        .find(|a| a.key == key)
        .ok_or("He's gone: another staff got to him, or the window closed.")?;
    let staff = readers::seat_staff(w).ok_or("Start a career first.")?;
    let seed = (key ^ key >> 32) as u32;
    let (name, sport, from, pos) = (a.name.clone(), a.sport, a.from, a.pos);
    let i = match staff {
        Staff::Club(t) => crate::world::pro::leagues::outside_signing(w, t, pos, a.truth, a.age, seed),
        Staff::Program(j) => crate::world::pro::leagues::outside_freshman(w, j, pos, a.truth, a.age, seed),
        Staff::School(_) => return Err("At a high school, talk to the athletes in your building.".into()),
    };
    let p = &mut w.people[i];
    p.name = name.clone();
    let age = p.player.age;
    p.note(format!("At {age} he came to football from {sport} ({from})."));
    w.scouting.outside.push(key);
    Ok(match staff {
        Staff::Club(_) => {
            format!("{name} signs to your practice squad: a {} from {sport}. Most never make it; some do.", pos.name())
        }
        _ => format!("{name} signs with you and joins your roster as a freshman {}: from {sport}.", pos.name()),
    })
}
