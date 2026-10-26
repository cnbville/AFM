//! The recruiting services through a class's high school years (update 2,
//! lane 7; minor bugs 12 and 15, FR5, FR6, FR9).
//!
//! Real services rank a class from its sophomore year and update it each
//! winter, spring, summer and fall; a class's first list is short and
//! fills as the services see more of it. The signing stages rate the class
//! signing next (from its junior spring); the updates here rank the younger
//! classes on the same quotas, scaled by how far the services reach by then
//! (`recruiting::rate_underclass`). Each rating keeps the class's national
//! order and the one before, so a page shows rank movement since the last
//! update, and each service's own stars and ranks are read from the shared
//! order with its own lasting lean (the composite averages them, from 2012).
//!
//! The insiders (one a service, 247Sports' Crystal Ball and On3's RPM in
//! life) log predictions on the open recruitments that matter, with a
//! confidence, and are judged when the kid commits; kids cut their lists to
//! announced top schools as their decision nears.
//!
//! Memory: a class's order and the one before (a few thousand ids each),
//! the insiders' open picks and their yearly records, and the announced
//! lists of the kids still deciding.

use std::collections::HashMap;

use rayon::prelude::*;

use super::public::{Desk, DeskKind, Lean};
use crate::calendar::{Date, Weekday};
use crate::rng::{Rng, clamp};
use crate::world::season::{key_normal, key_uniform};
use crate::world::{CommitTiming, Stage, World};

/// The composite (averaging the services) exists from 2012.
pub const COMPOSITE_FROM: i32 = 2012;

/// Insiders' logged predictions start with 247Sports' Crystal Ball (2011).
pub const INSIDERS_FROM: i32 = 2011;

/// What the services keep (stored on `Scouting::rankings`; every field fills
/// in empty on an older file).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Rankings {
    /// Each ranked class's national order at the latest rating and the one
    /// before.
    #[serde(default)]
    pub classes: Vec<ClassRanks>,
    /// The insiders' open predictions.
    #[serde(default)]
    pub picks: Vec<Pick>,
    /// Each insider's judged picks by year: (desk, year, judged, right).
    #[serde(default)]
    pub records: Vec<(u8, i32, u16, u16)>,
    /// Announced top schools of the kids still deciding.
    #[serde(default)]
    pub tops: Vec<TopList>,
}

/// One class's national order (player ids, best first) on the day it came
/// out, and the order before it.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ClassRanks {
    pub class_of: i32,
    pub date: Date,
    pub ids: Vec<u32>,
    #[serde(default)]
    pub prev_date: Option<Date>,
    #[serde(default)]
    pub prev: Vec<u32>,
}

/// An insider's prediction: the school, how sure (percent) and when.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Pick {
    pub id: u32,
    pub desk: u8,
    pub program: u16,
    pub conf: u8,
    pub date: Date,
}

/// A kid's announced top schools (his own order) and the day he said them.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TopList {
    pub id: u32,
    pub date: Date,
    pub schools: Vec<u16>,
}

/// The services' four updates of a year for the classes not yet signing
/// next: after the season (winter), after the spring evaluation period,
/// after June's camps and showcases, and in mid-season. Proposed, not
/// confirmed: 21 January, 12 May, 14 July and 20 October.
pub fn update_days(year: i32) -> [(Date, &'static str); 4] {
    [
        (Date::ymd(year, 1, 21), "winter"),
        (Date::ymd(year, 5, 12), "spring"),
        (Date::ymd(year, 7, 14), "summer"),
        (Date::ymd(year, 10, 20), "fall"),
    ]
}

/// The update on or before a day, with its season's word.
pub fn last_update(d: Date) -> (Date, &'static str) {
    let mut best = update_days(d.year() - 1)[3];
    for u in update_days(d.year()) {
        if u.0 <= d {
            best = u;
        }
    }
    best
}

/// The next update after a day.
pub fn next_update(d: Date) -> (Date, &'static str) {
    update_days(d.year()).into_iter().find(|u| u.0 > d).unwrap_or(update_days(d.year() + 1)[0])
}

/// The spring a school year ends in: a day from August on belongs to the
/// year that ends next spring.
pub fn school_year_end(d: Date) -> i32 {
    d.year() + i32::from(d.month() >= 8)
}

/// How far down each star tier the services' lists reach for a class this
/// early (5, 4, 3 and 2 stars, as shares of the class's full quotas): a
/// sophomore list is short and cautious at the top and fills through the
/// year; a junior class carries most of its 4-stars and about two thirds of
/// its 3-stars by the winter. Proposed, not confirmed (247Sports' and On3's
/// first lists for a class run to a few hundred names).
pub fn reach(class_of: i32, d: Date) -> [f64; 4] {
    match class_of - school_year_end(d) {
        ..=1 => [1.0, 1.0, 0.7, 0.25],
        // A sophomore class after its spring and summer.
        2 if (5..=7).contains(&d.month()) => [1.0, 0.9, 0.5, 0.1],
        _ => [0.6, 0.6, 0.25, 0.0],
    }
}

/// The daily work: an update day ranks the younger classes; each Tuesday
/// the insiders judge, predict and the kids near a decision cut their lists.
pub(crate) fn day(w: &mut World, d: Date) {
    if update_days(d.year()).iter().any(|u| u.0 == d) {
        underclass_update(w, d);
    }
    if d.weekday() == Weekday::Tue {
        insiders_week(w, d);
    }
}

/// The classes a services' update ranks on a day: from the one after the
/// class the signing stages rate (they take over at its first commitment
/// wave, 20 March of its junior year) down to this year's sophomores.
pub fn underclasses(w: &World, d: Date) -> std::ops::RangeInclusive<i32> {
    let season = w.dates.season;
    let rated = if d >= Date::ymd(season, 3, 20) { season + 1 } else { season };
    (rated + 1)..=(school_year_end(d) + 2)
}

/// The classes not in a signing stage's pool, sophomores up: their stars
/// and order for this update.
fn underclass_update(w: &mut World, d: Date) {
    for c in underclasses(w, d) {
        crate::world::recruiting::rate_underclass(w, c, d, reach(c, d));
        record(w, c, d);
    }
}

/// A file from before the services ranked the younger classes: the latest
/// update is taken now, so its pages show the lists a world of today keeps.
pub fn catch_up(w: &mut World) {
    let d = w.today;
    if w.year < 2000 || w.people.is_empty() {
        return;
    }
    // The class the signing stages rated keeps its stars: only its order is
    // written down.
    let signing = *underclasses(w, d).start() - 1;
    if class_ranks(w, signing).is_none() {
        record(w, signing, d);
    }
    let missing: Vec<i32> = underclasses(w, d).filter(|&c| class_ranks(w, c).is_none()).collect();
    for c in missing {
        let at = last_update(d).0;
        crate::world::recruiting::rate_underclass(w, c, d, reach(c, at));
        record(w, c, d);
    }
}

/// Keeps a class's national order after a rating: stars first, then the
/// services' running read (the order every page shows until the next).
pub(crate) fn record(w: &mut World, class_of: i32, d: Date) {
    let mut list: Vec<(u8, f64, u32)> = w
        .people
        .iter()
        .filter(|p| {
            p.class_of == class_of
                && p.stars > 0
                && (matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)
                    || (p.stage == Stage::College && p.college_seasons == 0))
        })
        .map(|p| (p.stars, p.cal.service_read.unwrap_or(0.0), p.player.id))
        .collect();
    list.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.total_cmp(&a.1)).then(a.2.cmp(&b.2)));
    let ids: Vec<u32> = list.into_iter().map(|x| x.2).collect();
    let r = &mut w.scouting.rankings;
    match r.classes.iter_mut().find(|c| c.class_of == class_of) {
        Some(c) if c.date == d => c.ids = ids,
        Some(c) => {
            c.prev = std::mem::replace(&mut c.ids, ids);
            c.prev_date = Some(c.date);
            c.date = d;
        }
        None => r.classes.push(ClassRanks { class_of, date: d, ids, prev_date: None, prev: Vec::new() }),
    }
    // Classes that have signed and gone keep nothing here.
    let oldest = w.year - 1;
    w.scouting.rankings.classes.retain(|c| c.class_of >= oldest);
}

/// A class's latest order, if the services have ranked it.
pub fn class_ranks(w: &World, class_of: i32) -> Option<&ClassRanks> {
    w.scouting.rankings.classes.iter().find(|c| c.class_of == class_of)
}

/// The composite's rating scale (0.70-1.00, as 247Sports' composite): each
/// star band from its top to its bottom in national order. 5 stars from
/// .9834, 4 from .8900, 3 from .7970.
const BANDS: [(u8, f64, f64); 4] = [(5, 1.0, 0.9834), (4, 0.9833, 0.89), (3, 0.8899, 0.797), (2, 0.7969, 0.70)];

/// Each ranked kid of a class: his national rank and composite rating, read
/// off the class's order (stars as the list gives them, a specialist's
/// services' stars included).
pub fn composite(w: &World, c: &ClassRanks) -> HashMap<u32, (u32, f64)> {
    let stars: Vec<u8> = c.ids.iter().map(|&id| super::place(w, id).map_or(0, |i| w.people[i].stars)).collect();
    let mut out = HashMap::with_capacity(c.ids.len());
    for &(s, hi, lo) in &BANDS {
        let band: Vec<usize> = (0..c.ids.len()).filter(|&k| stars[k] == s).collect();
        let n = band.len().max(1) as f64;
        for (j, &k) in band.iter().enumerate() {
            let rating = hi - (hi - lo) * (j as f64 / n);
            out.insert(c.ids[k], (k as u32 + 1, rating));
        }
    }
    out
}

/// A class's rank before its latest rating, by id (rank movement).
pub fn previous(c: &ClassRanks) -> HashMap<u32, u32> {
    c.prev.iter().enumerate().map(|(k, &id)| (id, k as u32 + 1)).collect()
}

/// One service's board: (stars, national rank) by player id.
pub type ServiceBoard = HashMap<u32, (u8, u32)>;

/// Each service's own read of a ranked class: its stars and national rank
/// for every kid on its list, from the composite order with the service's
/// lasting lean on each kid, its bias, and a little of the update's noise.
/// The services rank as many kids at each star as the shared list does;
/// the shorter-listed ones stop sooner among the 2-stars.
pub fn service_boards(w: &World, c: &ClassRanks) -> Vec<(usize, ServiceBoard)> {
    let comp = composite(w, c);
    let year = c.date.year();
    let mut counts = [0usize; 6];
    for &id in &c.ids {
        if let Some(i) = super::place(w, id) {
            counts[w.people[i].stars.min(5) as usize] += 1;
        }
    }
    w.scouting
        .desks
        .iter()
        .enumerate()
        .filter(|(_, d)| d.kind == DeskKind::Service && d.active(year))
        .map(|(k, desk)| {
            let mut v: Vec<(f64, u32)> = c
                .ids
                .iter()
                .filter_map(|&id| {
                    let i = super::place(w, id)?;
                    let base = comp.get(&id)?.1;
                    Some((base + lean(w, desk, i, c.date), id))
                })
                .collect();
            v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            // How deep this service goes into the 2-stars.
            let depth = 0.6 + 0.4 * key_uniform(desk.key ^ 0xDE97, c.class_of as u64);
            let take = counts[5] + counts[4] + counts[3] + (counts[2] as f64 * depth).round() as usize;
            let mut out = HashMap::with_capacity(take);
            let mut at = 0;
            for s in [5u8, 4, 3, 2] {
                for _ in 0..counts[s as usize] {
                    if at >= v.len() || at >= take {
                        break;
                    }
                    out.insert(v[at].1, (s, at as u32 + 1));
                    at += 1;
                }
            }
            (k, out)
        })
        .collect()
}

/// A service's lean on a kid on the composite scale: lasting (the same
/// misread every update), a little of the update's own, and its bias.
fn lean(w: &World, desk: &Desk, i: usize, d: Date) -> f64 {
    let p = &w.people[i];
    let id = p.player.id as u64;
    let sd = 0.012 + 0.0015 * (20.0 - desk.skill as f64);
    let mut g = sd * (0.85 * key_normal(desk.key ^ 0x5E41, id) + 0.35 * key_normal(desk.key ^ d.0 as u64, id));
    let size = crate::body::combine_size(p.player.position);
    g += match desk.lean {
        Lean::BigPrograms if p.camp_stars >= 4 || p.cal.offers.len() + p.cal.early_offers.len() >= 15 => 0.006,
        Lean::Viral if p.player.attributes.get(crate::attributes::Attr::Speed) >= 80.0 => 0.006,
        Lean::Size => 0.004 * clamp((p.player.body.weight - size.2) / size.3.max(1.0), -1.5, 1.5),
        Lean::Production => {
            let games: u32 = p.hs_lines.iter().rev().take(1).map(|l| l.games as u32).sum();
            if games >= 10 { 0.004 } else { 0.0 }
        }
        _ => 0.0,
    };
    g
}

// --- The insiders (FR5) and announced top schools (FR6) ---------------------------------

/// The insider who logs a service's predictions, by name (made up, from the
/// desk's key, so it never changes).
pub fn insider_name(desk: &Desk) -> String {
    use crate::world::ethnicity::Ethnicity;
    let u = key_uniform(desk.key ^ 0x1A51, 3);
    let eth = if u < 0.7 {
        Ethnicity::White
    } else if u < 0.9 {
        Ethnicity::Black
    } else {
        Ethnicity::Hispanic
    };
    let born = 1975 + (key_uniform(desk.key ^ 0x1A52, 5) * 18.0) as i32;
    let mut rng = Rng::new((key_uniform(desk.key ^ 0x1A53, 7) * u32::MAX as f64) as u32);
    crate::world::names::person_name(eth, born, &mut rng)
}

/// An insider's record in a year: (picks judged, right).
pub fn insider_record(w: &World, desk: usize, year: i32) -> (u16, u16) {
    w.scouting.rankings.records.iter().find(|r| r.0 as usize == desk && r.1 == year).map_or((0, 0), |r| (r.2, r.3))
}

/// The day a recruit means to decide, from his own timing: a commitment
/// wave of his junior spring or senior year, the early period, or
/// February's signing day.
pub fn decision_day(w: &World, i: usize) -> Option<Date> {
    let p = &w.people[i];
    let season = p.class_of - 1;
    Some(match p.cal.commit_timing? {
        CommitTiming::Summer(k) => {
            let (m, day) = [(3, 20), (4, 30), (6, 15), (8, 31), (10, 31)][(k as usize).min(4)];
            Date::ymd(season, m, day)
        }
        CommitTiming::Early => Date::ymd(season, 12, 18),
        CommitTiming::Late => Date::nth_weekday(season + 1, 2, Weekday::Wed, 1),
    })
}

/// The insiders' week: picks judged on the kids who committed, new and
/// changed picks on the open recruitments that matter, and the top lists of
/// the kids whose decision nears.
fn insiders_week(w: &mut World, d: Date) {
    judge(w, d);
    let year = d.year();
    if year < INSIDERS_FROM {
        return;
    }
    let next = crate::world::recruiting::next_class(w);
    // The recruitments that matter: 4- and 5-stars and the 3-stars with a
    // real offer list, still open, in the two classes signing next.
    let kids: Vec<usize> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)
                && (p.class_of == next || p.class_of == next + 1)
                && p.cal.commit.is_none()
                && p.cal.signed.is_none()
                && !p.player.position.is_specialist()
                && (p.stars >= 4 || (p.stars == 3 && p.cal.offers.len() + p.cal.early_offers.len() >= 6))
        })
        .map(|(i, _)| i)
        .collect();
    if kids.is_empty() {
        return;
    }
    // Each kid's race as he feels it (read only, on all cores).
    let races: Vec<(usize, Vec<(usize, f64)>)> = {
        let wr: &World = w;
        kids.par_iter()
            .map(|&i| (i, crate::world::front_office::college::race_at(wr, i)))
            .filter(|(_, r)| r.len() >= 2)
            .collect()
    };
    let desks: Vec<(u8, Desk)> = w
        .scouting
        .desks
        .iter()
        .enumerate()
        .filter(|(_, x)| x.kind == DeskKind::Service && x.active(year) && x.from.max(INSIDERS_FROM) <= year)
        .map(|(k, x)| (k as u8, x.clone()))
        .collect();
    let week = (d.0 / 7) as u64;
    let mut picks: Vec<Pick> = Vec::new();
    let mut tops: Vec<TopList> = Vec::new();
    // Picks and lists his recruitment has left behind: a pick on a school
    // out of his top three, a list his new leader isn't on.
    let mut dead_picks: Vec<(u32, u8)> = Vec::new();
    let mut dead_tops: Vec<u32> = Vec::new();
    for (i, race) in &races {
        let p = &w.people[*i];
        let id = p.player.id;
        let soon = decision_day(w, *i).map(|x| d.days_to(x));
        for (k, desk) in &desks {
            // The insider's read: the kid's feel plus his own lasting misread
            // of each school and a little of the week's.
            let sd = 0.018 + 0.0025 * (20.0 - desk.skill as f64);
            let mut read: Vec<(f64, usize)> = race
                .iter()
                .map(|&(j, a)| {
                    let key = desk.key ^ (j as u64) << 24;
                    (a + sd * (0.8 * key_normal(key ^ 0xC8A1, id as u64) + 0.4 * key_normal(key ^ week, id as u64)), j)
                })
                .collect();
            read.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            let gap = read[0].0 - read[1].0;
            let near = soon.is_some_and(|s| (0..=75).contains(&s));
            let have = w.scouting.rankings.picks.iter().find(|x| x.id == id && x.desk == *k);
            // His stock moved and new offers came: a pick on a school that has
            // fallen out of his top three is moved like a fresh one, or taken
            // back when nobody leads yet.
            let stale = have.is_some_and(|h| !read.iter().take(3).any(|r| r.1 == h.program as usize));
            let wants = match have {
                Some(h) if !stale => h.program as usize != read[0].1 && gap >= 0.06,
                _ => gap >= 0.08 || (near && gap >= 0.025),
            };
            if wants {
                let conf = clamp(50.0 + gap * 550.0, 50.0, 95.0).round() as u8;
                picks.push(Pick { id, desk: *k, program: read[0].1 as u16, conf, date: d });
            } else if stale {
                dead_picks.push((id, *k));
            }
        }
        // His list: a top group once his decision is three months off, cut
        // to the last few in the final month.
        let n = race.len();
        let list = w.scouting.rankings.tops.iter().find(|t| t.id == id);
        // A list his recruitment has outgrown (the school he now likes best
        // isn't on it, or most of it has fallen away) is gone: he has
        // reopened, and names a new group when his decision nears again.
        let outgrown = list.is_some_and(|t| {
            let alive = t.schools.iter().filter(|&&s| race.iter().any(|r| r.0 == s as usize)).count();
            !t.schools.contains(&(race[0].0 as u16)) || alive * 2 < t.schools.len()
        });
        if outgrown {
            dead_tops.push(id);
        }
        let had = list.filter(|_| !outgrown).map(|t| t.schools.len());
        let cut = match (soon, had) {
            (Some(s), None) if (0..=90).contains(&s) && n >= 6 && (p.stars >= 4 || n >= 10) => Some(if n >= 14 {
                10
            } else if n >= 10 {
                8
            } else {
                5
            }),
            (Some(s), Some(h)) if (0..=30).contains(&s) && h >= 6 => Some(if h >= 8 { 5 } else { 3 }),
            _ => None,
        };
        if let Some(cut) = cut {
            let schools: Vec<u16> = race.iter().take(cut.min(n)).map(|r| r.0 as u16).collect();
            tops.push(TopList { id, date: d, schools });
        }
    }
    let r = &mut w.scouting.rankings;
    r.picks.retain(|p| !dead_picks.contains(&(p.id, p.desk)));
    r.tops.retain(|t| !dead_tops.contains(&t.id));
    for x in picks {
        r.picks.retain(|p| !(p.id == x.id && p.desk == x.desk));
        r.picks.push(x);
    }
    for t in tops {
        r.tops.retain(|x| x.id != t.id);
        r.tops.push(t);
    }
}

/// Picks on kids who committed are judged right or wrong in the insider's
/// record; picks and lists of kids no longer in high school go.
fn judge(w: &mut World, d: Date) {
    let year = d.year();
    let mut done: Vec<(u8, bool)> = Vec::new();
    let mut keep: Vec<Pick> = Vec::new();
    for x in std::mem::take(&mut w.scouting.rankings.picks) {
        let Some(i) = super::place(w, x.id) else { continue };
        let p = &w.people[i];
        let went = p.cal.commit.map(|c| c.0).or(p.cal.signed.map(|s| s.program));
        match went {
            Some(j) => done.push((x.desk, j == x.program as usize)),
            None if matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) => keep.push(x),
            None => {}
        }
    }
    let r = &mut w.scouting.rankings;
    r.picks = keep;
    for (k, right) in done {
        match r.records.iter_mut().find(|x| x.0 == k && x.1 == year) {
            Some(x) => {
                x.2 += 1;
                x.3 += u16::from(right);
            }
            None => r.records.push((k, year, 1, u16::from(right))),
        }
    }
    r.records.retain(|x| x.1 >= year - 6);
    let people = &w.people;
    w.scouting.rankings.tops.retain(|t| match people.binary_search_by_key(&t.id, |p| p.player.id) {
        Ok(i) => matches!(people[i].stage, Stage::HighSchool | Stage::PrepSchool),
        Err(_) => false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_updates_a_year_in_order() {
        let u = update_days(2026);
        assert!(u.windows(2).all(|p| p[0].0 < p[1].0));
        assert_eq!(last_update(Date::ymd(2026, 1, 20)), (Date::ymd(2025, 10, 20), "fall"));
        assert_eq!(last_update(Date::ymd(2026, 1, 21)).1, "winter");
        assert_eq!(last_update(Date::ymd(2026, 9, 1)).1, "summer");
        assert_eq!(next_update(Date::ymd(2026, 8, 1)), (Date::ymd(2026, 10, 20), "fall"));
        assert_eq!(next_update(Date::ymd(2026, 10, 20)), (Date::ymd(2027, 1, 21), "winter"));
    }

    #[test]
    fn a_school_year_turns_in_august() {
        assert_eq!(school_year_end(Date::ymd(2026, 7, 31)), 2026);
        assert_eq!(school_year_end(Date::ymd(2026, 8, 1)), 2027);
    }

    #[test]
    fn a_sophomore_list_starts_short_and_fills_by_the_junior_year() {
        // The class of 2029: sophomores in the fall of 2026, juniors in 2027-28.
        let fall = reach(2029, Date::ymd(2026, 10, 20));
        let summer = reach(2029, Date::ymd(2027, 7, 14));
        let junior = reach(2029, Date::ymd(2027, 10, 20));
        for t in 0..4 {
            assert!(fall[t] <= summer[t] && summer[t] <= junior[t], "tier {t}: {fall:?} {summer:?} {junior:?}");
        }
        assert_eq!(fall[3], 0.0, "no 2-stars on a first sophomore list");
        assert!(junior[1] >= 0.99, "a junior class carries its 4-stars");
    }

    #[test]
    fn the_composite_bands_meet_without_gaps() {
        assert_eq!(BANDS[0].0, 5);
        for p in BANDS.windows(2) {
            assert!(p[0].2 > p[1].1 && p[0].2 - p[1].1 < 0.0002, "{p:?}");
            assert_eq!(p[0].0, p[1].0 + 1);
        }
    }
}
