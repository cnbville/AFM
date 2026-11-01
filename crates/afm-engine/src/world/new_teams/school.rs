//! New high schools where the young population grows (SK68). A district
//! that runs out of room builds a school, redraws the lines and splits a
//! big school's zone. The new school opens with the zone's freshmen and
//! sophomores, the zone's juniors choose (many stay to finish where they
//! started), and the seniors stay. Its first varsity schedule is played by
//! kids, with no district to join until the next realignment.

use crate::calendar::{Event, Kind, Level, Loudness};
use crate::rng::Rng;
use crate::world::high_school::HighSchool;
use crate::world::season::key_uniform;
use crate::world::{Stage, World};

/// Each state's total population growth, 2010 to 2020, in percent (Census
/// apportionment counts), in `geo::STATE_CENTERS` order.
const GROWTH: [f64; 51] = [
    5.1, 3.3, 11.9, 3.3, 6.1, 14.8, 0.9, 10.2, 14.6, 14.6, 10.6, 7.0, 17.3, -0.1, 4.7, 4.7, 3.0, 3.8, 2.7, 2.6, 7.0,
    7.4, 2.0, 7.6, -0.2, 2.8, 9.6, 7.4, 15.0, 4.6, 5.7, 2.8, 4.2, 9.5, 15.8, 2.3, 5.5, 10.6, 2.4, 4.3, 10.7, 8.9, 8.9,
    15.9, 18.4, 2.8, 7.9, 14.6, -3.2, 3.6, 2.3,
];

/// The nation grew 7.4% in 2010-20 while its under-18 population fell
/// 1.4% (Census): a state's school-age growth runs about 8.8 points under
/// its total.
const YOUNG_GAP: f64 = 8.8;

/// The share of a state's school-age growth that new buildings take
/// (proposed, not confirmed: all of it. The growth lands in suburbs whose
/// schools are full, while the room that shrinking towns free up is too
/// far away to use).
const NEW_BUILDING_SHARE: f64 = 1.0;

/// The share of the old zone the new school takes (proposed, not
/// confirmed: a new suburban school opens at about two fifths of its
/// parent's size, as Texas and Utah districts redraw).
pub const ZONE_SHARE: f64 = 0.4;

/// A zoned junior's chance to move, before playing time and your pitch
/// (proposed, not confirmed: many districts let rising juniors and
/// seniors finish at their old school).
pub const JUNIOR_MOVE: f64 = 0.5;

/// A school too small to split.
const MIN_PARENT: u32 = 1200;

/// A state's yearly school-age growth, as a share (0 where it shrinks).
pub fn young_growth(s: usize) -> f64 {
    ((GROWTH.get(s).copied().unwrap_or(0.0) - YOUNG_GAP) / 1000.0).max(0.0)
}

/// The year's new schools, state by state: a Poisson draw on the state's
/// public schools times the growth new buildings take. Each opens this
/// August.
pub(crate) fn year(w: &mut World) {
    let year = w.year;
    let mut rng = Rng::new(w.options.seed ^ 0x5C40_0E11 ^ (year as u32).wrapping_mul(2_246_822_519));
    for s in 0..w.states.len() {
        let g = young_growth(s);
        if g <= 0.0 {
            continue;
        }
        let publics = w.states[s].schools.iter().filter(|&&h| !w.high_schools[h].private).count();
        let mean = NEW_BUILDING_SHARE * publics as f64 * g;
        let n = super::college::poisson(&mut rng, mean);
        for _ in 0..n {
            let Some(parent) = pick_parent(w, s, &mut rng) else { break };
            open(w, parent, year, &mut rng);
        }
    }
}

/// The school a district splits: a big public school, the bigger the
/// likelier (crowding grows with size).
fn pick_parent(w: &World, s: usize, rng: &mut Rng) -> Option<usize> {
    let pool: Vec<(usize, f64)> = w.states[s]
        .schools
        .iter()
        .filter(|&&h| {
            let x = &w.high_schools[h];
            !x.private && x.enrollment >= MIN_PARENT && w.new_teams.school(h).is_none_or(|n| n.opens < w.year)
        })
        .map(|&h| (h, (w.high_schools[h].enrollment as f64).powi(2)))
        .collect();
    (!pool.is_empty()).then(|| rng.weighted(&pool))
}

/// Opens a school in `parent`'s town that August, with part of its zone.
/// Returns the new school's index.
pub fn open(w: &mut World, parent: usize, opens: i32, rng: &mut Rng) -> usize {
    let p = w.high_schools[parent].clone();
    let id = w.high_schools.len();
    let in_town = w.states[p.state].schools.iter().filter(|&&h| w.high_schools[h].town == p.town).count();
    let taken = |w: &World, n: &str| w.states[p.state].schools.iter().any(|&h| w.high_schools[h].name == n);
    let mut name = crate::world::names::high_school_name(&p.town, in_town, false, rng);
    let mut k = in_town;
    while taken(w, &name) && k < in_town + 60 {
        k += 1;
        name = crate::world::names::high_school_name(&p.town, k, false, rng);
    }
    let fictional_name = crate::world::names::high_school_name(&p.town, k % 7 + 1, false, rng);
    let enrollment = (p.enrollment as f64 * ZONE_SHARE) as u32;
    // A new building with a new weight room, and no name yet (proposed,
    // not confirmed).
    let resources = (0.55 * p.resources + 0.3).clamp(0.3, 0.95);
    let prestige = (0.12 + rng.range(0.0, 0.08)).min(p.prestige);
    // The nearest classification by size among the state's schools until
    // the next realignment.
    let classification = w.states[p.state]
        .schools
        .iter()
        .map(|&h| &w.high_schools[h])
        .filter(|x| !x.private)
        .min_by_key(|x| x.enrollment.abs_diff(enrollment))
        .map_or(p.classification, |x| x.classification);
    w.high_schools.push(HighSchool {
        id,
        state: p.state,
        town: p.town.clone(),
        county: p.county.clone(),
        area: p.area,
        name: name.clone(),
        fictional_name,
        real: false,
        private: false,
        enrollment,
        classification,
        resources,
        area_money: p.area_money,
        area_safety: p.area_safety,
        prestige,
        national: false,
        coach: None,
        oc: None,
        dc: None,
        roster: Vec::new(),
        seasons: Vec::new(),
        state_titles: 0,
    });
    w.high_schools[parent].enrollment = p.enrollment - enrollment;
    w.states[p.state].schools.push(id);
    crate::world::finance::hs::add_school(w, id);
    w.new_teams.schools.push(super::NewSchool {
        school: id,
        announced: opens - 1,
        opens,
        parents: vec![parent],
        zoned: false,
        moved: 0,
        stayed: 0,
        pitched: Vec::new(),
    });
    let text = format!(
        "{name} opens in August: the district redrew the lines, and part of {}'s zone goes to the new school. Its freshmen and sophomores move; juniors choose; seniors stay.",
        p.name
    );
    let d = w.today;
    w.calendar.post(
        Event::new(d, Level::HighSchool, Kind::News, Loudness::Medium, text)
            .hs_link(id, &name)
            .hs_link(parent, &p.name),
    );
    id
}

/// Whether a player lives in the new school's zone (a keyed draw: the
/// same answer every time it's asked).
pub fn in_zone(w: &World, school: usize, person_id: u32) -> bool {
    key_uniform(w.options.seed as u64 ^ 0x2_0E5E ^ (school as u64) << 24, person_id as u64) < ZONE_SHARE
}

/// What a zoned player does: `None` for a senior (he stays), else his
/// chance to move.
pub fn move_chance(w: &World, new: usize, i: usize) -> Option<f64> {
    let p = &w.people[i];
    let year = w.year;
    match p.class_of - year {
        // Freshmen and sophomores (the class comes in on the first day).
        3.. => Some(1.0),
        2 => {
            // A junior who sits behind two or more at his position goes where
            // he'd play; a starter stays (proposed, not confirmed).
            let pos = p.player.position;
            let ovr = p.player.overall();
            let ahead = w.high_schools[p.hs]
                .roster
                .iter()
                .filter(|&&j| j != i && w.people[j].player.position == pos && w.people[j].player.overall() > ovr)
                .count();
            let play = match ahead {
                0 => -0.15,
                1 => 0.0,
                _ => 0.15,
            };
            let pitched = w.new_teams.school(new).is_some_and(|n| n.pitched.contains(&p.player.id));
            Some((JUNIOR_MOVE + play + if pitched { 0.15 } else { 0.0 }).clamp(0.05, 0.95))
        }
        _ => None,
    }
}

/// The zone's players before the first day: (person index, his chance to
/// move; 0 for a senior who stays). For the new school's staff.
pub fn zone(w: &World, new: usize) -> Vec<(usize, f64)> {
    let Some(n) = w.new_teams.school(new) else { return Vec::new() };
    let mut out = Vec::new();
    for &parent in &n.parents {
        for &i in &w.high_schools[parent].roster {
            let p = &w.people[i];
            if p.stage != Stage::HighSchool || !in_zone(w, new, p.player.id) {
                continue;
            }
            out.push((i, move_chance(w, new, i).unwrap_or(0.0)));
        }
    }
    out
}

/// The first school day: the zone's families move to the new school (the
/// class that comes in today was drawn to it already). Whether anyone
/// moved.
pub(crate) fn class_day(w: &mut World) -> bool {
    let year = w.year;
    let open: Vec<usize> =
        w.new_teams.schools.iter().filter(|n| n.opens == year && !n.zoned).map(|n| n.school).collect();
    let any = !open.is_empty();
    for new in open {
        let zoned = zone(w, new);
        let (mut moved, mut stayed) = (0u32, 0u32);
        let name = w.high_schools[new].name.clone();
        for (i, chance) in zoned {
            let p = &w.people[i];
            if p.class_of - year >= 4 {
                continue;
            }
            let u = key_uniform(w.options.seed as u64 ^ 0x2_0E5F ^ (new as u64) << 24, p.player.id as u64);
            let old = w.high_schools[p.hs].name.clone();
            if u < chance {
                moved += 1;
                let p = &mut w.people[i];
                p.hs = new;
                p.note(format!("{year}: zoned to the new {name}; moved from {old}"));
            } else if chance > 0.0 {
                stayed += 1;
                w.people[i].note(format!("{year}: zoned to the new {name}; chose to stay at {old}"));
            }
        }
        if let Some(n) = w.new_teams.schools.iter_mut().find(|n| n.school == new) {
            n.zoned = true;
            n.moved = moved;
            n.stayed = stayed;
        }
    }
    any
}
