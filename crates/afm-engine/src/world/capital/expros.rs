//! Ex-pros who give back (Capital lane C1, CP6).
//!
//! A pro's rookie deal changes nothing. When he signs his second contract,
//! his first big deal after it, he may join his college's donors, with a
//! bond to each coach who coached him there (his head coach and position
//! coach, warmer for each season together). Every program gets its own
//! ex-pros on the same rules; you are one of the coaches when you coached
//! him. Sizes are Claude's (proposed, not confirmed).

use super::donors::{self, Bond, Donor, Money};
use crate::calendar::Date;
use crate::world::pro::money::{self, Kind};
use crate::world::{Stage, World};

/// A second contract that counts: at least this many times the league's
/// minimum for a first-year player, a year on average.
pub const BAR: f64 = 5.0;
/// The chance he joins: 0.3, plus 0.05 a season he played there (to 0.5),
/// less 0.15 when he started at another school.
pub const JOIN_BASE: f64 = 0.3;
pub const JOIN_SEASON: f64 = 0.05;
pub const JOIN_MOVED: f64 = 0.15;
/// What he could give: this share of his deal's new money.
pub const SHARE: f64 = 0.06;
/// His bond with a coach who coached him: 0.55 and 0.08 a season, to 0.9.
pub const COACHED_WARMTH: f32 = 0.55;
pub const COACHED_SEASON: f32 = 0.08;

/// A deal past the rookie one.
fn second(kind: Kind) -> bool {
    matches!(kind, Kind::Veteran | Kind::Extension | Kind::Franchise | Kind::Transition)
}

/// On 1 July: pros on their first big deal after the rookie one are looked
/// at once (`Capital.pros`).
pub(crate) fn year(w: &mut World) {
    let year = w.year;
    let bar = BAR * money::minimum(year, 0);
    let mut new: Vec<(usize, f64)> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        if p.stage != Stage::Pro {
            continue;
        }
        let Some(c) = p.pro.as_ref().and_then(|x| x.contract.as_ref()) else { continue };
        if !second(c.kind) || c.apy() < bar {
            continue;
        }
        if w.capital.pros.binary_search(&p.player.id).is_ok() {
            continue;
        }
        new.push((i, c.total()));
    }
    for (i, total) in new {
        let id = w.people[i].player.id;
        if let Err(k) = w.capital.pros.binary_search(&id) {
            w.capital.pros.insert(k, id);
        }
        if let Some(d) = ex_pro(w, i, total, year) {
            let mut d = d;
            d.id = w.capital.id();
            w.capital.donors.push(d);
        }
    }
}

/// The ex-pro donor for person `i`, if he gives (keyed: one look).
fn ex_pro(w: &World, i: usize, total: f64, year: i32) -> Option<Donor> {
    let p = &w.people[i];
    let pro = p.pro.as_ref()?;
    let alum = pro.alumnus.and_then(|a| w.alumni.get(a));
    let colleges: Vec<usize> = alum.map(|a| a.colleges.clone()).unwrap_or_else(|| p.college.into_iter().collect());
    let &school = colleges.last()?;
    if w.programs.get(school).is_none_or(|s| s.division == crate::world::data::Division::JUCO) {
        return None;
    }
    let staff: Vec<(i32, usize, Option<usize>, Option<usize>)> =
        alum.map(|a| a.college_staff.iter().copied().filter(|s| s.1 == school).collect()).unwrap_or_default();
    let seasons = staff.len().max(1) as f64;
    let moved = colleges.len() > 1;
    let chance = JOIN_BASE + (JOIN_SEASON * seasons).min(0.2) - if moved { JOIN_MOVED } else { 0.0 };
    if super::draw(w, 8, p.player.id as u64) >= chance {
        return None;
    }
    let mut d = donors::draw_donor(w, school, year, year, 0xE0_0000 ^ p.player.id as u64);
    d.name = p.name.clone();
    d.she = false;
    d.eth = p.ethnicity;
    d.born = p.born.parts().0;
    d.money = Money::New;
    d.alum = true;
    d.also = None;
    d.also_pull = 0.0;
    let team = pro.contract.as_ref().map_or(0, |c| c.team);
    d.business = format!("his contract with the {}", w.nfl.get(team).map_or("league", |t| t.name));
    d.capacity = total * SHARE;
    d.paper = d.capacity * 1.05;
    d.affinity = (0.5 + 0.06 * seasons as f32).min(0.85);
    d.pro = Some(p.player.id);
    let mut rng = crate::rng::Rng::new(w.options.seed ^ p.player.id.wrapping_mul(0x9E37_79B9));
    d.rooms = donors::pick_rooms(&mut rng, Money::New, true, true);
    // Ex-pros back NIL more than most (a fund for the players after him).
    d.nil_lean = if year >= 2021 { (d.nil_lean - 0.2).max(-1.0) } else { 1.0 };
    // A bond with each coach who coached him there.
    let mut bonds: Vec<Bond> = Vec::new();
    for &(_, _, head, pos) in &staff {
        for c in [head, pos].into_iter().flatten() {
            let c = c as u32;
            match bonds.iter_mut().find(|b| b.coach == c) {
                Some(b) => b.warmth = (b.warmth + COACHED_SEASON).min(0.9),
                None => bonds.push(Bond {
                    coach: c,
                    warmth: COACHED_WARMTH + COACHED_SEASON,
                    coached: true,
                    last: None::<Date>,
                    ..Default::default()
                }),
            }
        }
    }
    d.bonds = bonds;
    Some(d)
}
