//! The CFL's scouting year and its drafts (SK77, agreed by Cain on
//! 2026-11-14): each club's personnel department scouts the coming draft
//! class all year, the CFL Scouting Bureau ranks the class three times, the
//! invitational and national combines measure it in March, the national
//! draft runs after the NFL's, and the Global Draft the day after.
//!
//! The class is made of people from September (real: clubs scout them all
//! year): Canadians at U Sports schools, made here as people in the
//! `Stage::CflPool` stage, and Canadians at American colleges, a share of
//! the world's own college players (Canadian citizens by birth or through a
//! parent, their hometowns their own). Global prospects (from 2019, then
//! from 2021) are made the winter before their draft.
//!
//! Clubs pick from their own file's reads (`scouts::read_src`), not the
//! truth. The NFL takes the NCAA Canadians it wants first: one an NFL club
//! signed slides, and the club that drafts him holds his rights. A U Sports
//! pick with a year of eligibility left may go back to school, his rights
//! held. Your club's room holds the clock on its turn, as the NFL's war room
//! does.
//!
//! Real: 8 rounds from 2016 (7 from 2013, 6 from 1997 to 2012), the
//! Bureau's top 20 in September, January and April, an invitational combine
//! and the national CFL Combine in March, 74 picks in 2024 (47 from U Sports,
//! 27 from NCAA and NAIA schools), the Global Draft's 2 rounds mostly
//! kickers and punters. Sizes, shares and weights here are Claude's,
//! proposed, not confirmed.

use std::collections::{HashMap, HashSet};

use super::Status;
use super::other::{self, CFL, Pick, U_SPORTS};
use super::scouts::{self, DC, EVENT, HC, Look, LookKind, OC, Role, Src};
use crate::calendar::{Date, Kind, Loudness, Weekday};
use crate::positions::Position;
use crate::rng::{Rng, clamp};
use crate::world::season::{key_normal, key_uniform};
use crate::world::{Stage, World};

/// The share of college players who are Canadian citizens (by birth or a
/// parent): about 400 of the NCAA's 70,000 (proposed, not confirmed).
pub const NCAA_SHARE: f64 = 0.006;

/// A global prospect's school: none.
pub const GLOBAL: u8 = 255;

/// The U Sports players made for each class (proposed, not confirmed: the
/// draft takes about 47 of them).
const USPORTS_CLASS: usize = 130;

/// Global prospects in a pool (real 2026: 91 eligible).
const GLOBAL_POOL: usize = 80;

/// The scouts' regions: the four U Sports conferences, the NCAA, abroad.
pub const REGIONS: [&str; 6] = ["Canada West", "OUA", "RSEQ", "AUS", "American colleges", "Abroad"];

/// Countries global prospects come from, with their weight (from the real
/// global drafts: Germany, Mexico and Australia's punters lead).
pub const COUNTRIES: [(&str, f64); 18] = [
    ("Germany", 14.0),
    ("Mexico", 12.0),
    ("Australia", 10.0),
    ("England", 7.0),
    ("France", 7.0),
    ("Japan", 6.0),
    ("Austria", 5.0),
    ("Italy", 4.0),
    ("Finland", 4.0),
    ("Sweden", 4.0),
    ("Nigeria", 4.0),
    ("Denmark", 3.0),
    ("Brazil", 3.0),
    ("Netherlands", 3.0),
    ("Switzerland", 3.0),
    ("Ireland", 3.0),
    ("Norway", 2.0),
    ("Spain", 2.0),
];

/// The Bureau's three editions.
pub const EDITIONS: [&str; 3] = ["Fall", "Winter", "Spring"];

/// A prospect in the CFL's pool from outside the world's colleges.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Prospect {
    /// The draft he's in.
    pub draft: i32,
    /// His U Sports school (`U_SPORTS`), or `GLOBAL`.
    pub school: u8,
    /// A global prospect's country (`COUNTRIES`).
    pub country: u8,
    /// His year of U Sports eligibility in his draft season (4 or 5); 0
    /// abroad.
    pub year: u8,
    /// His last U Sports season, once played.
    #[serde(default)]
    pub line: Option<other::OtherSeason>,
}

impl Prospect {
    /// Where he plays, in words.
    pub fn from(&self) -> String {
        if self.school == GLOBAL {
            COUNTRIES[self.country as usize % COUNTRIES.len()].0.to_string()
        } else {
            U_SPORTS[self.school as usize % U_SPORTS.len()].to_string()
        }
    }
    pub fn global(&self) -> bool {
        self.school == GLOBAL
    }
}

/// Where a CFL prospect plays, in words: "Laval (RSEQ)" or "Germany".
pub fn pool_words(w: &World, id: u32) -> Option<String> {
    let x = w.others.pool.get(&id)?;
    Some(if x.global() { x.from() } else { format!("{} ({})", x.from(), conference(x.school)) })
}

/// A U Sports school's conference.
pub fn conference(school: u8) -> &'static str {
    REGIONS[region_of_school(school) as usize]
}

fn region_of_school(school: u8) -> u8 {
    match school {
        3..=8 => 0,
        2 | 9..=17 => 1,
        0 | 1 | 18..=21 => 2,
        22 => 3,
        GLOBAL => 5,
        _ => 4,
    }
}

/// One edition of the CFL Scouting Bureau's top 20.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Bureau {
    pub draft: i32,
    pub edition: u8,
    pub date: Date,
    pub ids: Vec<u32>,
}

/// A club's offer for your pick while you're on the clock.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Offer {
    pub from: u16,
    /// Their slot (index into the order) and a later slot of theirs.
    pub their: u16,
    pub extra: Option<u16>,
}

/// A CFL draft with your club's room open.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Room {
    pub year: i32,
    pub global: bool,
    pub rounds: u16,
    /// The franchise on each slot, every round.
    pub order: Vec<u16>,
    /// The picks made: slot, player id.
    pub picked: Vec<(u16, u32)>,
    pub team: u16,
    pub offers: Vec<Offer>,
    /// Trades made in the room, in words.
    pub trades: Vec<String>,
}

impl Room {
    pub fn next(&self) -> usize {
        self.picked.len()
    }
    pub fn on_clock(&self) -> bool {
        self.order.get(self.next()).is_some_and(|&f| f == self.team)
    }
}

/// Whether a player is a Canadian at an American college (keyed, so it
/// never changes).
pub fn canadian_ncaa(seed: u32, id: u32) -> bool {
    key_uniform(seed as u64 ^ 0xCA7A_0001, id as u64) < NCAA_SHARE
}

/// The draft's rounds by year (real: 7 from 1993, 6 from 1997, 7 from
/// 2013, 8 from 2016).
pub fn rounds(year: i32, global: bool) -> u16 {
    if global {
        // 2019: the Mexican and European drafts (about four rounds of
        // picks between them), then two rounds.
        return if year == 2019 { 4 } else { 2 };
    }
    match year {
        ..=1996 => 7,
        1997..=2012 => 6,
        2013..=2015 => 7,
        _ => 8,
    }
}

/// Whether a Global Draft is held (real: 2019, then from 2021).
pub fn global_draft(year: i32) -> bool {
    year == 2019 || year >= 2021
}

/// The draft a day works toward: next spring's from September.
fn coming(d: Date) -> i32 {
    let (y, m, _) = d.parts();
    if m >= 6 { y + 1 } else { y }
}

/// Your club, when you coach in the CFL.
pub fn seat_club(w: &World) -> Option<usize> {
    let s = w.seat?;
    let fr = s.other.filter(|_| s.level == crate::calendar::Level::Other)?;
    (other::league_of(fr) == CFL).then_some(fr)
}

/// The CFL's day (called by `other::day` while the CFL runs in full).
pub(crate) fn day(w: &mut World, d: Date, camp: Date) {
    let (year, m, dd) = d.parts();
    let next = coming(d);
    // The U Sports season (late August to the Vanier Cup in late
    // November): the class is made on opening weekend; the clubs' scouts
    // are at games on Saturdays.
    // (A world or a save that missed the day makes it on the next one, up to
    // April.)
    if (m, dd) == (8, 25) || ((m >= 9 || m <= 4) && !made(w, next)) {
        staffs(w, next);
        make_class(w, next);
    }
    if m == 12 && dd == 1 && year == 2018 {
        make_global(w, 2019);
    }
    if m == 2 && dd == 1 && global_draft(year) && year != 2019 {
        make_global(w, year);
    }
    if d.weekday() == Weekday::Sat && ((m, dd) >= (8, 30) && (m, dd) <= (11, 22)) {
        fall_week(w, d, next);
    }
    if (m, dd) == (11, 23) {
        season_lines(w, next);
    }
    if d.weekday() == Weekday::Wed && (m == 12 || m <= 2) {
        winter_week(w, d, next);
    }
    if (m, dd) == (9, 15) {
        bureau(w, d, next, 0);
    }
    if (m, dd) == (1, 14) {
        bureau(w, d, year, 1);
    }
    if (m, dd) == (4, 15) {
        bureau(w, d, year, 2);
    }
    if (m, dd) == (3, 6) {
        combine(w, d, year, true);
    }
    if (m, dd) == (3, 20) {
        combine(w, d, year, false);
    }
    if (m, dd) == (3, 28) {
        pro_days(w, d, year);
    }
    if d.weekday() == Weekday::Wed && m == 4 && dd <= 22 {
        spring_week(w, d, year);
    }
    // The drafts (a save from before the day moved holds it late, to May 1).
    let day = other::draft_day(year, camp);
    if global_draft(year) && year == 2019 && (m, dd) == (1, 14) && !drafted(w, year, true) {
        draft(w, d, true);
    }
    if d >= day && d <= Date::ymd(year, 5, 1) && !drafted(w, year, false) && w.others.room.is_none() {
        draft(w, d, false);
    }
    if year != 2019
        && global_draft(year)
        && d > day
        && d <= Date::ymd(year, 5, 2)
        && drafted(w, year, false)
        && !drafted(w, year, true)
        && w.others.room.is_none()
    {
        draft(w, d, true);
    }
    if (m, dd) == (5, 20) {
        after_drafts(w, year);
    }
}

/// The national draft's day in a year (the Global Draft is the next day;
/// 2019's was in January).
pub fn draft_date(year: i32) -> Date {
    let camp = other::format(CFL, year).map_or(Date::ymd(year, 5, 15), |f| f.start.add(-21));
    other::draft_day(year, camp)
}

/// The draft your scouts work toward today (the spring's until May).
pub fn coming_draft(w: &World) -> i32 {
    w.others.room.as_ref().map_or(coming(w.today), |r| r.year)
}

fn made(w: &World, draft: i32) -> bool {
    w.others.pool.values().any(|p| p.draft == draft && !p.global())
}

fn drafted(w: &World, year: i32, global: bool) -> bool {
    w.others.draft.iter().any(|p| p.year == year && p.global == global)
}

// --- The departments ------------------------------------------------------------------

/// Every CFL club's personnel department (proposed, not confirmed: real
/// CFL departments are small): a director of player personnel, a director
/// of Canadian scouting and two scouts, one on the U Sports West and the
/// Atlantic, one on Canadians at American colleges. A man leaves after the
/// draft about one year in eight.
fn staffs(w: &mut World, draft: i32) {
    let year = draft - 1;
    let teams = other::teams_in(CFL, draft);
    for fr in teams {
        let seed = w.options.seed ^ 0x5C0F ^ (year as u32) << 8 ^ (fr as u32 * 131);
        let mut rng = Rng::new(seed);
        let t = &mut w.others.teams[fr];
        if t.scouts.is_empty() {
            for role in [Role::Director, Role::National, Role::Area, Role::Area] {
                let s = scouts::hire(&mut rng, year, role, 0.3);
                t.scouts.push(s);
            }
        }
        // The two scouts: West and Atlantic, then the NCAA.
        let mut k = 0;
        for s in t.scouts.iter_mut() {
            s.region = match s.role {
                Role::Director => 5,
                Role::National => 1,
                Role::Area => {
                    k += 1;
                    if k == 1 { 0 } else { 4 }
                }
            };
        }
    }
}

fn turnover(w: &mut World, year: i32) {
    for fr in other::teams_in(CFL, year) {
        let seed = w.options.seed ^ 0x7E40 ^ (year as u32) << 8 ^ (fr as u32 * 37);
        let mut rng = Rng::new(seed);
        let n = w.others.teams[fr].scouts.len();
        for k in 0..n {
            if rng.chance(0.12) {
                let role = w.others.teams[fr].scouts[k].role;
                let region = w.others.teams[fr].scouts[k].region;
                let mut s = scouts::hire(&mut rng, year, role, 0.3);
                s.region = region;
                w.others.teams[fr].scouts[k] = s;
            }
        }
    }
}

/// The scouts' regions they cover.
pub fn covers(region: u8) -> Vec<u8> {
    match region {
        0 => vec![0, 3],
        1 => vec![1, 2],
        r => vec![r],
    }
}

// --- The class ------------------------------------------------------------------------

/// Makes next spring's U Sports class (opening weekend): 130 Canadians in
/// their fourth or fifth year (four in ten in their fourth: a pick may go
/// back to school), best near a CFL backup's level now.
fn make_class(w: &mut World, draft: i32) {
    if made(w, draft) {
        return;
    }
    use Position::*;
    let mix = [
        (IOL, 12.0),
        (OT, 8.0),
        (DT, 7.0),
        (EDGE, 6.0),
        (LB, 15.0),
        (S, 10.0),
        (CB, 9.0),
        (WR, 14.0),
        (RB, 8.0),
        (TE, 2.0),
        (K, 2.0),
        (P, 2.0),
        (QB, 2.0),
        (LS, 1.0),
    ];
    let mut rng = Rng::new(w.options.seed ^ 0xC1A5 ^ draft as u32);
    let mut made = Vec::new();
    for r in 0..USPORTS_CLASS {
        let pos = rng.weighted(&mix);
        let rating = clamp(56.5 - 0.11 * r as f64 + rng.normal(0.0, 2.2), 36.0, 61.0);
        let fourth = rng.chance(0.4);
        let age = if fourth { rng.int(20, 22) } else { rng.int(21, 23) } as u32;
        let school = rng.int(0, U_SPORTS.len() as i32 - 2) as u8;
        let seed = rng.next_u32();
        let i = pool_person(w, pos, rating, age, seed);
        let id = w.people[i].player.id;
        let year = if fourth { 4 } else { 5 };
        w.others.pool.insert(id, Prospect { draft, school, country: 0, year, line: None });
        let p = &mut w.people[i];
        let age_now = p.row_age();
        p.note(format!(
            "At {} ({}) in his year {year} of U Sports eligibility, at {age_now}: in the {draft} CFL draft class.",
            U_SPORTS[school as usize],
            conference(school)
        ));
        made.push(id);
    }
}

/// A person for the CFL's pool: built like the market's veterans, with no
/// college or pro career in the world.
fn pool_person(w: &mut World, pos: Position, rating: f64, age: u32, seed: u32) -> usize {
    let mut p = super::veteran(w, pos, rating, age, seed);
    p.notes.clear();
    p.college = None;
    p.path.clear();
    p.pro = None;
    p.stage = Stage::CflPool;
    w.people.push(p);
    w.people.len() - 1
}

/// The global pool (1 February, or 1 December 2018 for the Mexican and
/// European drafts of 2019): kickers and punters most of all, a few
/// linemen, linebackers and receivers, each from his country's own league
/// or its universities.
fn make_global(w: &mut World, draft: i32) {
    if w.others.pool.values().any(|p| p.draft == draft && p.global()) {
        return;
    }
    use Position::*;
    let mut rng = Rng::new(w.options.seed ^ 0x610B ^ draft as u32);
    let weights: Vec<(u8, f64)> = COUNTRIES
        .iter()
        .enumerate()
        .filter(|(_, c)| draft != 2019 || !matches!(c.0, "Australia" | "Japan" | "Nigeria" | "Brazil"))
        .map(|(k, c)| (k as u8, if draft == 2019 && c.0 == "Mexico" { 40.0 } else { c.1 }))
        .collect();
    for _ in 0..GLOBAL_POOL {
        let country = rng.weighted(&weights);
        let name = COUNTRIES[country as usize].0;
        let pos = if name == "Australia" {
            rng.weighted(&[(P, 8.0), (K, 1.0), (DT, 0.5)])
        } else {
            rng.weighted(&[
                (K, 1.5),
                (P, 1.5),
                (WR, 1.4),
                (DT, 1.0),
                (EDGE, 0.8),
                (LB, 1.2),
                (IOL, 1.0),
                (OT, 0.6),
                (RB, 0.6),
                (S, 0.6),
                (CB, 0.5),
                (LS, 0.2),
            ])
        };
        let rating = if pos.kicks() {
            clamp(rng.normal(50.0, 5.0), 38.0, 62.0)
        } else {
            clamp(rng.normal(43.0, 4.5), 32.0, 56.0)
        };
        let age = rng.int(21, 27) as u32;
        let seed = rng.next_u32();
        let i = pool_person(w, pos, rating, age, seed);
        let eth = match name {
            "Mexico" | "Brazil" | "Spain" => crate::world::ethnicity::Ethnicity::Hispanic,
            "Japan" => crate::world::ethnicity::Ethnicity::EastAsian,
            "Nigeria" => crate::world::ethnicity::Ethnicity::Black,
            _ => crate::world::ethnicity::Ethnicity::White,
        };
        let born = w.year - age as i32;
        let p = &mut w.people[i];
        p.ethnicity = eth;
        p.name = crate::world::names::person_name(eth, born, &mut rng);
        let id = p.player.id;
        let age_now = p.row_age();
        p.note(format!("A {name} prospect for the {draft} CFL Global Draft, at {age_now}."));
        w.others.pool.insert(id, Prospect { draft, school: GLOBAL, country, year: 0, line: None });
    }
}

/// The CFL draft class: U Sports players in the pool for this draft (not
/// yet drafted) and the NCAA's Canadians in their last college season or
/// just out of it.
pub fn class(w: &World, draft: i32, global: bool) -> Vec<usize> {
    let taken: HashSet<u32> = w.others.draft.iter().map(|p| p.id).collect();
    let seed = w.options.seed;
    w.people
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            let id = p.player.id;
            if taken.contains(&id) {
                return false;
            }
            if let Some(x) = w.others.pool.get(&id) {
                return p.stage == Stage::CflPool && x.draft == draft && x.global() == global;
            }
            !global && ncaa_eligible(w, p, draft, seed)
        })
        .map(|(i, _)| i)
        .collect()
}

/// An NCAA Canadian in the class: his last college season this fall, or
/// declared, or a rookie pro this spring (the NFL's draft and rush come
/// first).
fn ncaa_eligible(w: &World, p: &crate::world::Person, draft: i32, seed: u32) -> bool {
    if p.college.is_none() || !canadian_ncaa(seed, p.player.id) {
        return false;
    }
    match p.stage {
        Stage::College => {
            let div = crate::world::eligibility::division_of(p, &w.programs);
            crate::world::eligibility::exhausted_after(p, div, draft - 1)
        }
        Stage::Declared => p.class_of <= draft - 3,
        Stage::Pro => p.pro.as_ref().is_some_and(|c| c.first_year == draft && c.seasons.is_empty()),
        _ => false,
    }
}

/// The public's read of a prospect (who the scouts go and see first): the
/// truth with a lasting misread, wider below the FBS.
pub fn consensus(w: &World, i: usize) -> f64 {
    let p = &w.people[i];
    let vis = if p.stage == Stage::CflPool {
        1.5
    } else {
        match p.college.map(|c| w.programs[c].division) {
            Some(crate::world::data::Division::FBS) => 1.0,
            Some(crate::world::data::Division::FCS) => 1.2,
            _ => 1.5,
        }
    };
    p.player.overall() + 4.0 * vis * key_normal(w.options.seed as u64 ^ 0xB0EA, p.player.id as u64)
}

/// Where a prospect plays, in the scouts' regions.
pub fn region(w: &World, i: usize) -> u8 {
    let id = w.people[i].player.id;
    w.others.pool.get(&id).map_or(4, |x| region_of_school(x.school))
}

/// A club's own working list: the public read with the club's lean.
fn club_list(w: &World, fr: usize, class: &[usize]) -> Vec<usize> {
    let mut v: Vec<(f64, usize)> = class
        .iter()
        .map(|&i| {
            let id = w.people[i].player.id as u64;
            (consensus(w, i) + 3.0 * key_normal(0xC1B0 ^ (fr as u64) << 20, id), i)
        })
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter().map(|x| x.1).collect()
}

fn file(w: &mut World, fr: usize, i: usize, who: u8, kind: LookKind, date: Date) {
    let id = w.people[i].player.id;
    let f = w.others.teams[fr].files.entry(id).or_default();
    if !f.iter().any(|l| l.who == who && l.kind == kind && l.date == date) {
        f.push(Look { who, kind, date });
    }
}

fn clubs(w: &World, draft: i32) -> Vec<usize> {
    other::teams_in(CFL, draft).into_iter().filter(|&fr| !w.others.teams[fr].scouts.is_empty()).collect()
}

/// A Saturday in the fall: each scout at a game in his regions, about three
/// prospects a week (the school call: film, practice, the coaches).
fn fall_week(w: &mut World, d: Date, draft: i32) {
    let class = class(w, draft, false);
    if class.is_empty() {
        return;
    }
    let week = (d.0 / 7) as usize;
    for fr in clubs(w, draft) {
        let list = club_list(w, fr, &class);
        let n = w.others.teams[fr].scouts.len();
        for k in 0..n {
            let s = &w.others.teams[fr].scouts[k];
            let (role, regions) = (s.role, covers(s.region));
            let mine: Vec<usize> = if role == Role::Director {
                // The director cross-checks the top of the list from October.
                if d.parts().1 < 10 {
                    continue;
                }
                list.iter().copied().take(30).collect()
            } else {
                list.iter().copied().filter(|&i| regions.contains(&region(w, i))).take(60).collect()
            };
            if mine.is_empty() {
                continue;
            }
            let per = if role == Role::Director { 2 } else { 3 };
            let at = (week * per) % mine.len();
            for &i in mine.iter().cycle().skip(at).take(per.min(mine.len())) {
                let kind = if role == Role::Director { LookKind::Director } else { LookKind::School };
                file(w, fr, i, k as u8, kind, d);
            }
        }
    }
}

/// A Wednesday from December to February: film study on the list.
fn winter_week(w: &mut World, d: Date, draft: i32) {
    let class = class(w, draft, false);
    if class.is_empty() {
        return;
    }
    let week = (d.0 / 7) as usize;
    for fr in clubs(w, draft) {
        let list = club_list(w, fr, &class);
        let n = w.others.teams[fr].scouts.len();
        for k in 0..n {
            let s = &w.others.teams[fr].scouts[k];
            let (role, regions) = (s.role, covers(s.region));
            let mine: Vec<usize> = match role {
                Role::Director => list.iter().copied().take(40).collect(),
                Role::National => list.iter().copied().take(80).collect(),
                Role::Area => list.iter().copied().filter(|&i| regions.contains(&region(w, i))).take(50).collect(),
            };
            if mine.is_empty() {
                continue;
            }
            let per = if role == Role::Area { 3 } else { 4 };
            let at = (week * per) % mine.len();
            let kind = if role == Role::Director { LookKind::Director } else { LookKind::Film };
            for &i in mine.iter().cycle().skip(at).take(per.min(mine.len())) {
                file(w, fr, i, k as u8, kind, d);
            }
        }
    }
}

/// The combines (March): the invitational first, for prospects who aren't
/// household names, then the national combine for the top of the class and
/// the invitational's best. Every club gets the same numbers; each club's
/// director interviews fifteen at the national combine. NCAA Canadians
/// test at their own pro days or the NFL's combine instead.
pub fn invited(w: &World, draft: i32, invitational: bool) -> Vec<usize> {
    let mut v: Vec<(f64, usize)> = class(w, draft, false)
        .into_iter()
        .filter(|&i| w.people[i].stage == Stage::CflPool)
        .map(|i| (consensus(w, i), i))
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let (top, inv) = (45usize, 40usize);
    let invitational_list: Vec<usize> = v.iter().skip(top).take(inv).map(|x| x.1).collect();
    if invitational {
        return invitational_list;
    }
    // The invitational's best nine by their testing join the top.
    let mut best: Vec<(f64, usize)> = invitational_list
        .iter()
        .map(|&i| {
            let a = &w.people[i].player.attributes;
            use crate::attributes::Attr;
            (a.get(Attr::Speed) + a.get(Attr::Acceleration) + a.get(Attr::Strength), i)
        })
        .collect();
    best.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.iter().take(top).map(|x| x.1).chain(best.into_iter().take(9).map(|x| x.1)).collect()
}

fn combine(w: &mut World, d: Date, draft: i32, invitational: bool) {
    let list = invited(w, draft, invitational);
    let cl = clubs(w, draft);
    for &fr in &cl {
        for &i in &list {
            file(w, fr, i, EVENT, LookKind::Combine, d);
        }
    }
    if invitational {
        // The global combine: every global prospect measured.
        let global = class(w, draft, true);
        for &fr in &cl {
            for &i in &global {
                file(w, fr, i, EVENT, LookKind::Combine, d);
            }
        }
        return;
    }
    // Interviews: each director sits down with fifteen from his list.
    for &fr in &cl {
        let mine: Vec<usize> = club_list(w, fr, &list).into_iter().take(15).collect();
        if let Some(dir) = w.others.teams[fr].scouts.iter().position(|s| s.role == Role::Director) {
            for i in mine {
                file(w, fr, i, dir as u8, LookKind::Interview, d);
            }
        }
    }
}

/// NCAA Canadians' testing: the NFL combine's numbers for those who went,
/// else their own school's pro day.
fn pro_days(w: &mut World, d: Date, draft: i32) {
    let ncaa: Vec<usize> =
        class(w, draft, false).into_iter().filter(|&i| w.people[i].stage != Stage::CflPool).collect();
    let cl = clubs(w, draft);
    for i in ncaa {
        let id = w.people[i].player.id;
        let at_combine = w
            .clubs
            .iter()
            .any(|c| c.files.get(&id).is_some_and(|f| f.iter().any(|l| l.who == EVENT && l.kind == LookKind::Combine)));
        let kind = if at_combine { LookKind::Combine } else { LookKind::ProDay };
        for &fr in &cl {
            file(w, fr, i, EVENT, kind, d);
        }
    }
}

/// April: the head coach studies the top twenty's film, his coordinators
/// their side's top twenty-five.
fn spring_week(w: &mut World, d: Date, draft: i32) {
    let class = class(w, draft, false);
    if class.is_empty() {
        return;
    }
    let week = (d.0 / 7) as usize;
    for fr in clubs(w, draft) {
        let list = club_list(w, fr, &class);
        let top: Vec<usize> = list.iter().copied().take(20).collect();
        let at = (week * 6) % top.len().max(1);
        for &i in top.iter().cycle().skip(at).take(6.min(top.len())) {
            file(w, fr, i, HC, LookKind::Coach, d);
        }
        for (who, off) in [(OC, true), (DC, false)] {
            let side: Vec<usize> = list
                .iter()
                .copied()
                .filter(|&i| {
                    crate::world::coach::Group::of(w.people[i].player.position).offense() == off
                        && !w.people[i].player.position.kicks()
                })
                .take(25)
                .collect();
            let at = (week * 6) % side.len().max(1);
            for &i in side.iter().cycle().skip(at).take(6.min(side.len())) {
                file(w, fr, i, who, LookKind::Coach, d);
            }
        }
    }
}

/// The U Sports season's lines (after the Vanier Cup): an eight-game season
/// from his level and role (a stand-in, as the world keeps no U Sports
/// games; proposed, not confirmed).
fn season_lines(w: &mut World, draft: i32) {
    let ids: Vec<u32> = w
        .others
        .pool
        .iter()
        .filter(|(_, x)| x.draft == draft && !x.global() && x.line.is_none())
        .map(|(&id, _)| id)
        .collect();
    for id in ids {
        let Some(i) = crate::world::scouting::place(w, id) else { continue };
        let p = &w.people[i];
        let pos = p.player.position;
        let q = clamp((p.player.overall() - 38.0) / 20.0, 0.1, 1.3);
        let k = |s: u64| key_uniform(0x5EA5 ^ s, id as u64);
        let g = 8u16;
        let mut l = other::OtherSeason { year: draft - 1, league: u8::MAX, games: g, starts: g, ..Default::default() };
        use Position as P;
        match pos {
            P::QB => {
                l.att = (190.0 + 90.0 * k(1)) as u16;
                l.cmp = (l.att as f64 * (0.55 + 0.12 * q)) as u16;
                l.pass_yds = (l.att as f64 * (6.5 + 2.5 * q)) as f32;
                l.pass_td = (8.0 + 10.0 * q * (0.8 + 0.4 * k(2))) as u16;
                l.int = (3.0 + 6.0 * k(3)) as u16;
                l.rush = (30.0 + 30.0 * k(4)) as u16;
                l.rush_yds = l.rush as f32 * (3.0 + 2.0 * q as f32);
            }
            P::RB => {
                l.rush = (70.0 + 70.0 * q * (0.7 + 0.6 * k(1))) as u16;
                l.rush_yds = l.rush as f32 * (4.5 + 2.0 * q as f32);
                l.rush_td = (2.0 + 8.0 * q * k(2)) as u16;
                l.rec = (8.0 + 15.0 * k(3)) as u16;
                l.rec_yds = l.rec as f32 * 9.0;
            }
            P::WR | P::TE => {
                l.rec = (15.0 + 35.0 * q * (0.6 + 0.8 * k(1))) as u16;
                l.rec_yds = l.rec as f32 * (11.0 + 5.0 * q as f32);
                l.rec_td = (1.0 + 7.0 * q * k(2)) as u16;
            }
            P::EDGE | P::DT | P::LB => {
                l.tackles = (18.0 + 30.0 * q * (0.6 + 0.8 * k(1))) as u16;
                l.sacks = ((if pos == P::LB { 1.0 } else { 3.0 }) * q * (0.5 + 1.5 * k(2))) as f32;
            }
            P::CB | P::S => {
                l.tackles = (15.0 + 25.0 * q * (0.6 + 0.8 * k(1))) as u16;
                l.ints = (3.0 * q * k(2)) as u16;
            }
            P::K => {
                l.fg_att = (10.0 + 8.0 * k(1)) as u16;
                l.fg = (l.fg_att as f64 * (0.6 + 0.25 * q)) as u16;
            }
            _ => {}
        }
        if let Some(x) = w.others.pool.get_mut(&id) {
            x.line = Some(l);
        }
    }
}

// --- Reads, the Bureau and boards --------------------------------------------------

/// Each club's grade of the class from its own file: its scouts', head
/// coach's and coordinators' leans, the combine's numbers (`read_src`);
/// a man it never filed reads off the public's word, a few points down.
pub(crate) fn board(w: &World, fr: usize, class: &[usize], ctx: &scouts::Ctx) -> HashMap<usize, f64> {
    let files = &w.others.teams[fr].files;
    class
        .iter()
        .map(|&i| {
            let id = w.people[i].player.id;
            let g = match files.get(&id) {
                Some(f) if !f.is_empty() => scouts::read_src(w, Src::Other(fr), i, f, ctx).grade,
                _ => consensus(w, i) - 3.0 + 2.0 * key_normal(0xB0A2 ^ (fr as u64) << 20, id as u64),
            };
            (i, g - slide(w, i) - specialist(w, i))
        })
        .collect()
}

/// What a national draft takes off a kicker, punter or snapper (proposed,
/// not confirmed): real clubs take them from the third round on, as one
/// spot on the roster; the Global Draft is for them, so nothing there.
fn specialist(w: &World, i: usize) -> f64 {
    let p = &w.people[i];
    if w.others.pool.get(&p.player.id).is_some_and(|x| x.global()) {
        return 0.0;
    }
    match p.player.position {
        Position::K | Position::P => 5.0,
        Position::LS => 7.0,
        _ => 0.0,
    }
}

/// What a club takes off a prospect it may not get soon (proposed, not
/// confirmed): an NCAA Canadian an NFL club signed slides about two
/// rounds; a fourth-year U Sports player who may go back to school a
/// little.
pub fn slide(w: &World, i: usize) -> f64 {
    let p = &w.people[i];
    if p.stage == Stage::Pro && p.pro.as_ref().is_some_and(|c| nfl_bound(c.status)) {
        return 6.0;
    }
    match w.others.pool.get(&p.player.id) {
        Some(x) if x.year == 4 => 0.5,
        _ => 0.0,
    }
}

fn nfl_bound(s: Status) -> bool {
    matches!(s, Status::Roster | Status::PracticeSquad | Status::Reserve)
}

/// The Bureau's edition: the top 20 by the clubs' reads together (the
/// Bureau is the clubs' own scouts).
fn bureau(w: &mut World, d: Date, draft: i32, edition: u8) {
    if w.others.bureau.iter().any(|b| b.draft == draft && b.edition == edition) {
        return;
    }
    let class = class(w, draft, false);
    if class.is_empty() {
        return;
    }
    let ids = bureau_order(w, &class);
    w.others.bureau.push(Bureau { draft, edition, date: d, ids: ids.into_iter().take(20).collect() });
    // Keep the last five years.
    w.others.bureau.retain(|b| b.draft > draft - 5);
    if w.recording {
        let top = w.others.bureau.last().and_then(|b| b.ids.first().copied());
        if let Some(i) = top.and_then(|id| crate::world::scouting::place(w, id)) {
            let text = format!(
                "The CFL Scouting Bureau's {} edition for the {draft} draft: {} is No. 1.",
                EDITIONS[edition as usize].to_lowercase(),
                w.people[i].name
            );
            super::market::post(w, d, None, Some(i), Loudness::Small, Kind::News, text);
        }
    }
}

fn bureau_order(w: &World, class: &[usize]) -> Vec<u32> {
    let ctx = scouts::Ctx::new(w, class);
    let cl = clubs(w, w.year + 1).into_iter().chain(clubs(w, w.year)).collect::<HashSet<_>>();
    let boards: Vec<HashMap<usize, f64>> = cl.iter().map(|&fr| board(w, fr, class, &ctx)).collect();
    let mut v: Vec<(f64, u32)> = class
        .iter()
        .map(|&i| {
            let g = if boards.is_empty() {
                consensus(w, i)
            } else {
                boards.iter().map(|b| b.get(&i).copied().unwrap_or(0.0)).sum::<f64>() / boards.len() as f64
            };
            (g, w.people[i].player.id)
        })
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.into_iter().map(|x| x.1).collect()
}

// --- The drafts ---------------------------------------------------------------------

/// The order: worst record last season first, every round.
pub fn order(w: &World, year: i32, global: bool) -> Vec<u16> {
    let teams = other::teams_in(CFL, year);
    let last = w.others.seasons.iter().rev().find(|s| s.league == CFL && s.year < year);
    let mut by: Vec<usize> = teams.clone();
    by.sort_by(|&a, &b| {
        let r = |fr: usize| last.and_then(|s| s.table.iter().find(|r| r.fr as usize == fr)).map_or(0.5, |r| r.pct());
        r(a).total_cmp(&r(b)).then(a.cmp(&b))
    });
    let n = rounds(year, global);
    (0..n).flat_map(|_| by.iter().map(|&f| f as u16)).collect()
}

/// A draft opens: with your club in it, your room holds the clock at your
/// turn; else the clubs pick it through.
fn draft(w: &mut World, d: Date, global: bool) {
    let year = d.year();
    report(w, year, global);
    let class = class(w, year, global);
    let order = order(w, year, global);
    if order.is_empty() || class.is_empty() {
        return;
    }
    let mine = seat_club(w);
    let room = Room {
        year,
        global,
        rounds: rounds(year, global),
        order,
        picked: Vec::new(),
        team: mine.map_or(u16::MAX, |f| f as u16),
        offers: Vec::new(),
        trades: Vec::new(),
    };
    w.others.room = Some(room);
    advance(w);
    if mine.is_none() {
        close(w, d);
    }
}

/// Whether your club's room holds the clock (your turn in a draft under
/// way).
pub fn holds(w: &World) -> bool {
    w.others.room.as_ref().is_some_and(|r| r.on_clock())
}

/// The AI clubs pick to your turn (or to the end).
pub fn advance(w: &mut World) {
    loop {
        let Some(r) = w.others.room.as_ref() else { return };
        let k = r.next();
        if k >= r.order.len() {
            break;
        }
        if r.on_clock() {
            offers(w);
            return;
        }
        let fr = r.order[k] as usize;
        let (year, global) = (r.year, r.global);
        let Some(i) = ai_choice(w, fr, year, global) else { break };
        take(w, k, fr, i);
    }
    let d = w.today;
    close(w, d);
}

/// The best left on a club's board, a little toward a need (each man at
/// the position on its roster counts against him).
fn ai_choice(w: &World, fr: usize, year: i32, global: bool) -> Option<usize> {
    let left = left(w, year, global);
    if left.is_empty() {
        return None;
    }
    let ctx = scouts::Ctx::new(w, &left);
    let b = board(w, fr, &left, &ctx);
    let have = |pos: Position| {
        w.others.teams[fr].roster.iter().filter(|&&i| w.people[i as usize].player.position == pos).count() as f64
    };
    let mut v: Vec<(f64, usize)> = left.iter().map(|&i| (b.get(&i).copied().unwrap_or(0.0), i)).collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.truncate(6);
    v.into_iter()
        .map(|(g, i)| {
            let need = if global { 0.0 } else { 0.8 * have(w.people[i].player.position) };
            (g - need, i)
        })
        .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
        .map(|x| x.1)
}

/// Your club's board (SK77): the coming draft's class still on the board,
/// best first, graded from your club's file the way its staff will pick
/// (your own looks as head coach are copied into it).
pub fn seat_board(w: &World) -> Vec<(usize, f64)> {
    let Some(fr) = seat_club(w) else { return Vec::new() };
    let (year, global) = match &w.others.room {
        Some(r) => (r.year, r.global),
        None => (coming(w.today), false),
    };
    let pool = left(w, year, global);
    if pool.is_empty() {
        return Vec::new();
    }
    let ctx = scouts::Ctx::new(w, &pool);
    let b = board(w, fr, &pool, &ctx);
    let mut v: Vec<(usize, f64)> = b.into_iter().collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// The round words for a place on a board: "Round 1", "Rounds 2-3",
/// "Rounds 4-5", "Rounds 6-8", "Free agent".
pub fn round_words(rank: usize, teams: usize, rounds: u16) -> &'static str {
    let r = rank / teams.max(1) + 1;
    match r {
        1 => "Round 1",
        2 | 3 => "Rounds 2-3",
        4 | 5 => "Rounds 4-5",
        _ if r <= rounds as usize => "Late rounds",
        _ => "Free agent",
    }
}

/// The class still on the board.
pub fn left(w: &World, year: i32, global: bool) -> Vec<usize> {
    let picked: HashSet<u32> =
        w.others.room.as_ref().map_or(HashSet::new(), |r| r.picked.iter().map(|x| x.1).collect());
    // An NCAA Canadian still on a college roster on draft day isn't
    // draftable yet.
    class(w, year, global)
        .into_iter()
        .filter(|&i| !picked.contains(&w.people[i].player.id) && w.people[i].stage != Stage::College)
        .collect()
}

/// Your pick: a player still on the board.
pub fn pick(w: &mut World, id: u32) -> Result<(), String> {
    let Some(r) = w.others.room.as_ref() else { return Err("No CFL draft is under way.".into()) };
    if !r.on_clock() {
        return Err("You're not on the clock.".into());
    }
    let (year, global, k, fr) = (r.year, r.global, r.next(), r.team as usize);
    let i = crate::world::scouting::place(w, id).ok_or("He isn't in this draft.")?;
    if !left(w, year, global).contains(&i) {
        return Err("He isn't on the board.".into());
    }
    take(w, k, fr, i);
    if let Some(r) = w.others.room.as_mut() {
        r.offers.clear();
    }
    advance(w);
    Ok(())
}

/// Your staff's pick: the top of your club's board.
pub fn staff_choice(w: &World) -> Option<usize> {
    let r = w.others.room.as_ref()?;
    ai_choice(w, r.team as usize, r.year, r.global)
}

/// Hands your pick to your staff.
pub fn auto_pick(w: &mut World) -> Result<(), String> {
    let i = staff_choice(w).ok_or("Nobody is left on the board.")?;
    let id = w.people[i].player.id;
    pick(w, id)
}

/// Takes a club's offer for your pick: your slot is theirs, theirs and
/// their later one are yours.
pub fn accept(w: &mut World, k: usize) -> Result<(), String> {
    let year = w.year;
    let Some(r) = w.others.room.as_mut() else { return Err("No CFL draft is under way.".into()) };
    let o = r.offers.get(k).cloned().ok_or("That offer is gone.")?;
    let now = r.next();
    let mine = r.team;
    r.order[now] = o.from;
    r.order[o.their as usize] = mine;
    if let Some(x) = o.extra {
        r.order[x as usize] = mine;
    }
    let n = r.order.len() / r.rounds.max(1) as usize;
    let words = |s: usize| format!("pick {} (round {})", s + 1, s / n.max(1) + 1);
    let text = format!(
        "You traded {} to the {} for {}{}.",
        words(now),
        other::name(o.from as usize, year),
        words(o.their as usize),
        o.extra.map_or(String::new(), |x| format!(" and {}", words(x as usize)))
    );
    r.trades.push(text);
    r.offers.clear();
    advance(w);
    Ok(())
}

/// Says no to the calls on your pick.
pub fn decline(w: &mut World) {
    if let Some(r) = w.others.room.as_mut() {
        r.offers.clear();
    }
}

/// Asks a caller for more (proposed, not confirmed): two in five add a
/// later pick of theirs; otherwise they hang up.
pub fn ask_more(w: &mut World, k: usize) -> Result<String, String> {
    let seed = w.options.seed as u64;
    let year = w.year;
    let Some(r) = w.others.room.as_mut() else { return Err("No CFL draft is under way.".into()) };
    let o = r.offers.get(k).cloned().ok_or("That offer is gone.")?;
    let name = other::name(o.from as usize, year);
    let key = (r.year as u64) << 20 ^ (r.next() as u64) << 4 ^ k as u64;
    let n = r.order.len() / r.rounds.max(1) as usize;
    if o.extra.is_some() {
        return Ok(format!("{name} say that's as far as they go."));
    }
    let more = (o.their as usize + n..r.order.len()).find(|&s| r.order[s] == o.from);
    if key_uniform(seed ^ 0x7EAF, key) < 0.4
        && let Some(m) = more
    {
        r.offers[k].extra = Some(m as u16);
        Ok(format!("{name} add pick {} (round {}).", m + 1, m / n.max(1) + 1))
    } else {
        r.offers.remove(k);
        Ok(format!("{name} pass. The offer is off the table."))
    }
}

/// Your staff's mock of the picks before your next one (SK77): for each
/// man on your board, how many times in ten he is still there, each club
/// in between taking the top of its own board (a little noise on each
/// read, and its needs). Empty when you have no later pick.
pub fn there_odds(w: &World) -> (Option<usize>, HashMap<u32, u8>) {
    let Some(r) = w.others.room.as_ref() else { return (None, HashMap::new()) };
    let start = r.next() + usize::from(r.on_clock());
    let Some(mine) = (start..r.order.len()).find(|&s| r.order[s] == r.team) else { return (None, HashMap::new()) };
    let pool = left(w, r.year, r.global);
    if pool.is_empty() {
        return (Some(mine), HashMap::new());
    }
    let ctx = scouts::Ctx::new(w, &pool);
    let between: Vec<usize> = (r.next()..mine).filter(|&s| r.order[s] != r.team).collect();
    let mut boards: HashMap<u16, HashMap<usize, f64>> = HashMap::new();
    for &s in &between {
        let fr = r.order[s];
        boards.entry(fr).or_insert_with(|| {
            let b = board(w, fr as usize, &pool, &ctx);
            b.into_iter()
                .map(|(i, g)| {
                    let have = w.others.teams[fr as usize]
                        .roster
                        .iter()
                        .filter(|&&j| w.people[j as usize].player.position == w.people[i].player.position)
                        .count() as f64;
                    (i, g - if r.global { 0.0 } else { 0.8 * have })
                })
                .collect()
        });
    }
    let mut there: HashMap<u32, u8> = HashMap::new();
    for run in 0..10u64 {
        let mut gone: HashSet<usize> = HashSet::new();
        for &s in &between {
            let b = &boards[&r.order[s]];
            let key = run << 24 ^ s as u64;
            let best = b
                .iter()
                .filter(|(i, _)| !gone.contains(i))
                .map(|(&i, &g)| (g + 2.0 * key_normal(0x7E40 ^ key, w.people[i].player.id as u64), i))
                .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)));
            if let Some((_, i)) = best {
                gone.insert(i);
            }
        }
        for &i in &pool {
            if !gone.contains(&i) {
                *there.entry(w.people[i].player.id).or_default() += 1;
            }
        }
    }
    (Some(mine), there)
}

/// Calls while you're on the clock (rounds 1 to 3, proposed, not
/// confirmed): a club picking soon after you wants your staff's top man
/// and offers its own pick and a later one of the same worth (about a pick
/// two rounds on), one call in three.
fn offers(w: &mut World) {
    let Some(r) = w.others.room.as_ref() else { return };
    if !r.offers.is_empty() {
        return;
    }
    let k = r.next();
    let n = r.order.len() / r.rounds.max(1) as usize;
    if n == 0 || k / n >= 3 || r.global {
        return;
    }
    let key = (r.year as u64) << 20 ^ k as u64;
    if key_uniform(w.options.seed as u64 ^ 0x7EAD, key) > 0.33 {
        return;
    }
    let me = r.team;
    let after: Vec<usize> = (k + 1..(k + 6).min(r.order.len())).filter(|&s| r.order[s] != me).collect();
    let Some(&their) = after.get((key_uniform(0x7EAE, key) * after.len() as f64) as usize % after.len().max(1)) else {
        return;
    };
    let from = r.order[their];
    let extra = (their + 2 * n..r.order.len()).find(|&s| r.order[s] == from);
    let offer = Offer { from, their: their as u16, extra: extra.map(|x| x as u16) };
    if let Some(r) = w.others.room.as_mut() {
        r.offers.push(offer);
    }
}

/// A club takes a player.
fn take(w: &mut World, k: usize, fr: usize, i: usize) {
    let Some(r) = w.others.room.as_ref() else { return };
    let (year, global) = (r.year, r.global);
    let n = r.order.len() / r.rounds.max(1) as usize;
    let round = (k / n.max(1) + 1) as u16;
    let pick = (k + 1) as u16;
    let id = w.people[i].player.id;
    if let Some(r) = w.others.room.as_mut() {
        r.picked.push((k as u16, id));
    }
    let club = other::name(fr, year);
    let what = if global { "CFL Global Draft" } else { "CFL draft" };
    let x = w.others.pool.get(&id).cloned();
    let from = match &x {
        Some(x) => x.from(),
        None => w.people[i].college.map_or("an American college".into(), |c| w.program_name(c).to_string()),
    };
    let school = x.as_ref().map_or((U_SPORTS.len() - 1) as u8, |x| x.school);
    {
        let p = &mut w.people[i];
        let age = p.row_age();
        p.note(format!(
            "Drafted by the {club} in round {round} of the {year} {what} (pick {pick}) out of {from}, at {age}."
        ));
    }
    let name = w.people[i].name.clone();
    let pos = w.people[i].player.position;
    w.others.draft.push(Pick { year, pick, team: fr as u16, id, name, pos, school, global, from });
    match x {
        Some(x) if x.year == 4 && back_to_school(w, id, round) => {
            w.others.rights.insert(id, (fr as u16, year));
            w.people[i].note(format!("Went back to {} for his last year; the {club} hold his rights.", x.from()));
        }
        Some(x) => join(w, i, fr, (!x.global()).then_some(x.school)),
        None => {
            w.others.canadians.insert(id);
            let p = &w.people[i];
            let nfl = p.stage == Stage::Pro && p.pro.as_ref().is_some_and(|c| nfl_bound(c.status));
            if nfl {
                w.others.rights.insert(id, (fr as u16, year));
                w.people[i].note(format!("The {club} hold his CFL rights while he's in the NFL."));
            } else {
                join(w, i, fr, None);
            }
        }
    }
}

/// A fourth-year pick goes back to school (proposed, not confirmed: about
/// one in three, fewer of the early picks, keyed).
fn back_to_school(w: &World, id: u32, round: u16) -> bool {
    key_uniform(w.options.seed as u64 ^ 0xBAC5, id as u64) < 0.2 + 0.03 * round as f64
}

/// A pick reports: a pro on the club's roster, a Canadian under the ratio.
fn join(w: &mut World, i: usize, fr: usize, school: Option<u8>) {
    let year = w.year;
    let id = w.people[i].player.id;
    {
        let p = &mut w.people[i];
        if p.stage != Stage::Pro || p.pro.is_none() {
            p.stage = Stage::Pro;
            p.pro = Some(Box::new(super::ProCareer::new(None, Status::FreeAgent, year)));
        }
    }
    if let Some(s) = school {
        w.others.nationals.insert(id, s);
    }
    w.others.pool.remove(&id);
    other::sign(w, i, CFL, fr);
    if !w.others.teams[fr].roster.contains(&(i as u32)) {
        w.others.teams[fr].roster.push(i as u32);
    }
}

/// Last spring's picks who went back to school report with the new class.
fn report(w: &mut World, year: i32, global: bool) {
    if global {
        return;
    }
    let back: Vec<(u32, u16)> = w
        .others
        .rights
        .iter()
        .filter(|(id, (_, y))| *y < year && w.others.pool.contains_key(id))
        .map(|(&id, &(fr, _))| (id, fr))
        .collect();
    let mut back = back;
    back.sort_unstable();
    for (id, fr) in back {
        let Some(i) = crate::world::scouting::place(w, id) else { continue };
        if w.people[i].stage != Stage::CflPool {
            continue;
        }
        let school = w.others.pool.get(&id).map(|x| x.school);
        w.others.rights.remove(&id);
        join(w, i, fr as usize, school);
    }
}

/// The draft is done: the news, your room closes.
fn close(w: &mut World, d: Date) {
    let Some(r) = w.others.room.take() else { return };
    if w.recording {
        let what = if r.global { "Global Draft" } else { "draft" };
        let text = format!("The {} CFL {what}: {} picks over {} rounds.", r.year, r.picked.len(), r.rounds);
        super::market::post(w, d, None, None, Loudness::Small, Kind::News, text);
    }
}

/// Finishes your room as your staff would (a day passed with it open).
pub fn finish(w: &mut World) {
    for _ in 0..200 {
        if !holds(w) {
            break;
        }
        if auto_pick(w).is_err() {
            let d = w.today;
            close(w, d);
            break;
        }
    }
}

/// After the drafts (late May): the undrafted. A U Sports player good
/// enough to try goes to the street as a free agent (a Canadian under the
/// ratio); the rest, and every undrafted global prospect, leave football.
/// The files are cleared and the departments turn over.
fn after_drafts(w: &mut World, year: i32) {
    let mine: Vec<(u32, bool, u8)> = w
        .others
        .pool
        .iter()
        .filter(|(id, x)| x.draft <= year && !w.others.rights.contains_key(id))
        .map(|(&id, x)| (id, x.global(), x.school))
        .collect();
    let mut mine = mine;
    mine.sort_unstable();
    for (id, global, school) in mine {
        w.others.pool.remove(&id);
        let Some(i) = crate::world::scouting::place(w, id) else { continue };
        let p = &mut w.people[i];
        if p.stage != Stage::CflPool {
            continue;
        }
        let age = p.row_age();
        if !global && p.player.overall() >= 44.0 {
            p.stage = Stage::Pro;
            p.pro = Some(Box::new(super::ProCareer::new(None, Status::FreeAgent, year)));
            p.note(format!("Undrafted, a free agent at {age}."));
            w.others.nationals.insert(id, school);
        } else {
            p.stage = Stage::Done;
            p.note(format!("Undrafted; left football at {age}."));
        }
    }
    for fr in other::teams_in(CFL, year) {
        w.others.teams[fr].files.clear();
    }
    turnover(w, year);
}
