//! The world's hunts (SK67): every team plays by the rules your seat
//! does.
//!
//! - College head coaches name one target a class on the services' summer
//!   update before its junior year (14 July): a kid within the program's
//!   reach (its stars, as the offer chain reads them), weighed by the
//!   services' read, home and footprint, and the coach's own taste. How
//!   likely a staff runs one follows the head coach's ambition and
//!   recruiting. The hunter's head coach gives the kid his own time, holds
//!   a scholarship at his spot and pays him as a buying program would
//!   (`recruiting::run_signing_stage`).
//! - NFL clubs name one target a draft on September 1: a club without a
//!   franchise quarterback leans to the class's best quarterback; a club
//!   reaches about six places past where it expects to pick. Its GM watches
//!   his Saturdays (a look in the club's file each week) and on draft day
//!   values him `HUNT_EDGE` points higher, so it takes him or trades up for
//!   him more readily.
//! - Private high school powers name one standout a year (15 January),
//!   before the summer's transfers: his move to them comes likelier
//!   (`recruiting::high_school_transfers`, 1.8 times the pull).
//! - Rivals hear of your interest: each Monday a chase the world knows
//!   about can draw one more program (a club, in the spring before the
//!   draft), as many as three (two at the NFL).
//!
//! Proposed, not confirmed: every rate here.

use std::collections::{HashMap, HashSet};

use super::{Hunt, Seat};
use crate::calendar::{Date, Event, Kind as CalKind, Level, Loudness, Weekday};
use crate::world::data::Division;
use crate::world::scouting as sc;
use crate::world::season::{key_normal, key_uniform};
use crate::world::{Stage, World};

pub const SCHOOL: u8 = 0;
pub const COLLEGE: u8 = 1;
pub const CLUB: u8 = 2;

/// Why a team hunts him: its own pick, your interest leaked, the whole
/// world's chase for a generational prospect.
pub const OWN: u8 = 0;
pub const PILED: u8 = 1;
pub const GEN: u8 = 2;

/// The prestige (relative) from which a college chases a generational
/// prospect whatever it planned.
pub const GEN_PRESTIGE: f64 = 0.6;

/// One team's hunt.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AiHunt {
    pub level: u8,
    pub team: u32,
    pub id: u32,
    /// The class (high school, college) or the draft (club).
    pub year: i32,
    pub since: Date,
    pub why: u8,
    /// When it ended and whether they got him.
    pub end: Option<(Date, bool)>,
}

impl AiHunt {
    pub fn open(&self) -> bool {
        self.end.is_none()
    }
}

pub(crate) fn day(w: &mut World, d: Date) {
    let (_, m, dd) = d.parts();
    if (m, dd) == (7, 14) {
        college(w, d);
    }
    if (m, dd) == (9, 1) {
        clubs(w, d);
    }
    if (m, dd) == (1, 15) {
        schools(w, d);
    }
    if d.weekday() == Weekday::Sat && (9..=12).contains(&m) {
        looks(w, d);
    }
    if d.weekday() == Weekday::Mon {
        settle(w, d);
    }
    if (m, dd) == (3, 1) {
        // Ended hunts stay six years for the record and the screens.
        let cut = d.add(-6 * 365);
        w.hunt.ai.retain(|h| h.end.is_none_or(|e| e.0 >= cut));
    }
}

/// The stars a program's hunt reaches for: its own level and one below
/// (a top program hunts five-stars and four-stars, the middle of the FBS
/// four- and three-stars).
fn reach(prestige_rel: f64) -> u8 {
    (2.0 + (3.0 * prestige_rel).ceil()).clamp(3.0, 5.0) as u8
}

fn seed(w: &World) -> u64 {
    w.options.seed as u64 ^ 0x4A11_0000_0000
}

fn your_team(w: &World, s: Seat) -> Option<usize> {
    super::seat(w).filter(|x| x.0 == s).map(|x| x.1)
}

/// 14 July: each college staff's target in the class going into its
/// junior year.
fn college(w: &mut World, d: Date) {
    let class_of = d.year() + 2;
    let seed = seed(w);
    let mut cands: Vec<(usize, f64)> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)
                && p.class_of == class_of
                && p.stars >= 3
                && p.cal.signed.is_none()
        })
        .map(|(i, p)| (i, p.cal.service_read.unwrap_or(0.0)))
        .collect();
    if cands.is_empty() {
        return;
    }
    cands.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    cands.truncate(800);
    let chased = super::generational::of_class(w, class_of).and_then(|id| sc::place(w, id));
    let mine = your_team(w, Seat::College);
    let busy: HashSet<u32> =
        w.hunt.ai.iter().filter(|h| h.level == COLLEGE && h.year == class_of && h.open()).map(|h| h.team).collect();
    let mut new: Vec<(usize, usize, u8)> = Vec::new();
    for (j, prog) in w.programs.iter().enumerate() {
        if !matches!(prog.division, Division::FBS | Division::FCS) || !prog.plays(d.year()) {
            continue;
        }
        if Some(j) == mine || busy.contains(&(j as u32)) {
            continue;
        }
        let Some(c) = prog.head_coach else { continue };
        let coach = &w.coaches[c];
        if let Some(g) = chased
            && prog.division == Division::FBS
            && prog.prestige_rel >= GEN_PRESTIGE
        {
            new.push((j, g, GEN));
            continue;
        }
        let p_hunt = (0.2 + 0.03 * (coach.personality.ambition - 10.0) + 0.02 * (coach.ability().recruiting - 10.0))
            .clamp(0.05, 0.6);
        if key_uniform(seed ^ (class_of as u64) << 32, j as u64) >= p_hunt {
            continue;
        }
        let reach = reach(prog.prestige_rel);
        let mut best: Option<(f64, usize)> = None;
        for &(i, read) in &cands {
            let p = &w.people[i];
            if p.stars > reach || p.stars + 1 < reach {
                continue;
            }
            let home = p.home_state.min(50);
            let s = read
                + if p.home_state == prog.state { 3.0 } else { 0.0 }
                + 2.0 * prog.footprint[home]
                + 2.0 * key_normal(seed ^ 0x7A57 ^ (c as u64) << 24, p.player.id as u64);
            if best.is_none_or(|b| s > b.0) {
                best = Some((s, i));
            }
        }
        if let Some((_, i)) = best {
            new.push((j, i, OWN));
        }
    }
    let mut chased_count = 0;
    for (j, i, why) in new {
        chased_count += (why == GEN) as usize;
        let id = w.people[i].player.id;
        w.hunt.ai.push(AiHunt { level: COLLEGE, team: j as u32, id, year: class_of, since: d, why, end: None });
    }
    if let Some(g) = chased
        && chased_count > 0
    {
        let p = &w.people[g];
        let (id, name) = (p.player.id, p.name.clone());
        let text = format!(
            "{chased_count} of the nation's power programs are after {name} already, two years before he can sign."
        );
        w.calendar.post(Event::new(d, Level::Recruiting, CalKind::Whisper, Loudness::Big, text).person(id, &name));
    }
}

/// September 1: each club's target for the next draft.
fn clubs(w: &mut World, d: Date) {
    let dy = sc::public::draft_year(w);
    let seed = seed(w);
    let mut cands: Vec<(usize, f64)> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.stage == Stage::College && dy - p.class_of >= 3 && p.pro.is_none())
        .filter_map(|(i, p)| p.cal.draft_read.map(|r| (i, r.score)))
        .collect();
    if cands.is_empty() {
        return;
    }
    cands.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    cands.truncate(64);
    let mine = your_team(w, Seat::Club);
    let mut order: Vec<(f64, usize)> = crate::world::franchise::active_teams_in(w, dy)
        .into_iter()
        .filter(|&t| t < w.clubs.len())
        .map(|t| (w.nfl[t].strength(), t))
        .collect();
    order.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let chased = super::generational::of_draft(w, dy).and_then(|id| sc::place(w, id));
    let busy: HashSet<u32> =
        w.hunt.ai.iter().filter(|h| h.level == CLUB && h.year == dy && h.open()).map(|h| h.team).collect();
    let mut new: Vec<(usize, usize, u8)> = Vec::new();
    for (k, &(_, t)) in order.iter().enumerate() {
        if Some(t) == mine || busy.contains(&(t as u32)) {
            continue;
        }
        if let Some(g) = chased
            && k < 5
        {
            new.push((t, g, GEN));
            continue;
        }
        let hole = crate::world::legacy::nfl::qb_rank(w, t).is_none_or(|r| !crate::world::legacy::nfl::franchise(r.0));
        let gm = &w.clubs[t].office.gm;
        let p_hunt = (0.25 + if hole { 0.35 } else { 0.0 } + 0.2 * gm.impatience.clamp(0.0, 1.0)).min(0.85);
        if key_uniform(seed ^ 0xC1B0 ^ (dy as u64) << 32, t as u64) >= p_hunt {
            continue;
        }
        let mut best: Option<(f64, usize)> = None;
        for &(i, score) in cands.iter().take(k + 7) {
            let p = &w.people[i];
            let pos = p.player.position;
            let s = score
                + if hole && pos == crate::positions::Position::QB { 4.0 } else { 0.0 }
                + 0.5 * crate::career::need(&w.nfl[t], pos)
                + 1.5 * key_normal(seed ^ 0x6E7A ^ (t as u64) << 24 ^ (dy as u64) << 40, p.player.id as u64);
            if best.is_none_or(|b| s > b.0) {
                best = Some((s, i));
            }
        }
        if let Some((_, i)) = best {
            new.push((t, i, OWN));
        }
    }
    for (t, i, why) in new {
        let id = w.people[i].player.id;
        w.hunt.ai.push(AiHunt { level: CLUB, team: t as u32, id, year: dy, since: d, why, end: None });
    }
}

/// 15 January: each private high school power's standout for the
/// summer's transfers.
fn schools(w: &mut World, d: Date) {
    let year = d.year();
    let seed = seed(w);
    let mine = your_team(w, Seat::School);
    let busy: HashSet<u32> =
        w.hunt.ai.iter().filter(|h| h.level == SCHOOL && h.year == year && h.open()).map(|h| h.team).collect();
    // Standouts by state: the top 3% of the freshmen and sophomores (two
    // seasons left at least), and the nation's best for the academies.
    let mut by_state: Vec<Vec<(usize, f64)>> = vec![Vec::new(); 51];
    for (i, p) in w.people.iter().enumerate() {
        if p.stage == Stage::HighSchool && p.class_of >= w.dates.season + 3 && p.class_of <= w.dates.season + 4 {
            by_state[p.home_state.min(50)].push((i, p.player.overall()));
        }
    }
    let mut nation: Vec<(usize, f64)> = Vec::new();
    for v in by_state.iter_mut() {
        if v.is_empty() {
            continue;
        }
        v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let keep = ((v.len() as f64 * 0.03).ceil() as usize).max(1);
        v.truncate(keep);
        nation.extend(v.iter().take(2).copied());
    }
    let mut new: Vec<(usize, usize)> = Vec::new();
    for (h, s) in w.high_schools.iter().enumerate() {
        if !(s.private && (s.is_powerhouse() || s.national)) || Some(h) == mine || busy.contains(&(h as u32)) {
            continue;
        }
        let amb = s.coach.map_or(10.0, |c| w.coaches[c].personality.ambition);
        if key_uniform(seed ^ 0x5C00 ^ (year as u64) << 32, h as u64) >= (0.35 + 0.03 * (amb - 10.0)).clamp(0.1, 0.7) {
            continue;
        }
        let pool: &[(usize, f64)] = if s.national { &nation } else { &by_state[s.state.min(50)] };
        let mut best: Option<(f64, usize)> = None;
        for &(i, ovr) in pool {
            let p = &w.people[i];
            let at = &w.high_schools[p.hs];
            if p.hs == h || at.is_powerhouse() || (!s.national && at.state != s.state) {
                continue;
            }
            let v = ovr
                + if at.county == s.county { 3.0 } else { 0.0 }
                + 2.0 * key_normal(seed ^ 0x5C01 ^ (h as u64) << 24, p.player.id as u64);
            if best.is_none_or(|b| v > b.0) {
                best = Some((v, i));
            }
        }
        if let Some((_, i)) = best {
            new.push((h, i));
        }
    }
    for (h, i) in new {
        let id = w.people[i].player.id;
        w.hunt.ai.push(AiHunt { level: SCHOOL, team: h as u32, id, year, since: d, why: OWN, end: None });
    }
}

/// Saturdays in the fall: a club's GM watches the man it hunts (a look in
/// its file each week).
fn looks(w: &mut World, d: Date) {
    let list: Vec<(usize, u32)> =
        w.hunt.ai.iter().filter(|h| h.level == CLUB && h.open()).map(|h| (h.team as usize, h.id)).collect();
    for (t, id) in list {
        if sc::place(w, id).is_some_and(|i| w.people[i].stage == Stage::College) {
            sc::dept::club_look(w, t, id, crate::world::pro::scouts::GM, crate::world::pro::scouts::LookKind::Gm, d);
        }
    }
}

/// Mondays: hunts end when he signs, is drafted, moves or is gone.
fn settle(w: &mut World, d: Date) {
    let draft_done = w.today > w.dates.draft[2].date;
    for k in 0..w.hunt.ai.len() {
        let h = w.hunt.ai[k].clone();
        if !h.open() {
            continue;
        }
        let Some(i) = sc::place(w, h.id) else {
            w.hunt.ai[k].end = Some((d, false));
            continue;
        };
        let p = &w.people[i];
        let team = h.team as usize;
        let end: Option<bool> = match h.level {
            COLLEGE => match p.cal.signed {
                Some(s) => Some(s.program == team),
                None if !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) => Some(p.college == Some(team)),
                None => None,
            },
            CLUB => match p.pro.as_ref() {
                Some(pro) => Some(pro.draft.map(|x| x.3).or(pro.team) == Some(team)),
                None if !matches!(p.stage, Stage::College | Stage::Declared) => Some(false),
                // Back to school for another year: the hunt is over.
                None if (w.year > h.year || (w.year == h.year && draft_done)) => Some(false),
                None => None,
            },
            _ => {
                if p.hs == team {
                    Some(true)
                } else if p.stage != Stage::HighSchool || d >= Date::ymd(h.year, 8, 31) {
                    Some(false)
                } else {
                    None
                }
            }
        };
        if let Some(got) = end {
            w.hunt.ai[k].end = Some((d, got));
        }
    }
}

/// Rivals who hear of your interest (Mondays): one more program (a club in
/// the draft's spring) may join the chase, by how much the world knows.
pub(crate) fn pile_on(w: &mut World, d: Date, h: &Hunt, heat: f64) {
    if heat < 0.2 {
        return;
    }
    let Some(i) = sc::place(w, h.id) else { return };
    let p = &w.people[i];
    let on: Vec<u32> = w.hunt.ai.iter().filter(|x| x.id == h.id && x.open()).map(|x| x.team).collect();
    let piled = w.hunt.ai.iter().filter(|x| x.id == h.id && x.open() && x.why == PILED).count();
    let u = key_uniform(seed(w) ^ 0x9113 ^ (d.0 as u64) << 20, h.id as u64);
    match h.seat {
        Seat::College => {
            if piled >= 3 || u >= 0.1 * heat {
                return;
            }
            // Programs at your level or above who'd chase a kid of his
            // stars; one of the five best of them hears first.
            let mine = w.programs[h.team].prestige_rel;
            let mut pool: Vec<usize> = (0..w.programs.len())
                .filter(|&j| {
                    let q = &w.programs[j];
                    j != h.team
                        && q.division == Division::FBS
                        && q.plays(d.year())
                        && q.head_coach.is_some()
                        && q.prestige_rel >= mine - 0.15
                        && p.stars + 1 >= reach(q.prestige_rel)
                        && !on.contains(&(j as u32))
                })
                .collect();
            if pool.is_empty() {
                return;
            }
            pool.sort_by(|&a, &b| w.programs[b].prestige.total_cmp(&w.programs[a].prestige).then(a.cmp(&b)));
            pool.truncate(5);
            let j = pool[((u / (0.1 * heat)) * pool.len() as f64).min(pool.len() as f64 - 1.0) as usize];
            let class_of = p.class_of;
            // It drops its own target in the class for him.
            for x in w
                .hunt
                .ai
                .iter_mut()
                .filter(|x| x.level == COLLEGE && x.team == j as u32 && x.year == class_of && x.open())
            {
                x.end = Some((d, false));
            }
            w.hunt.ai.push(AiHunt {
                level: COLLEGE,
                team: j as u32,
                id: h.id,
                year: class_of,
                since: d,
                why: PILED,
                end: None,
            });
            let text = format!("{} has joined the chase for {} ({}).", w.program_name(j), h.name, h.pos);
            w.calendar.post(
                Event::new(d, Level::Recruiting, CalKind::Whisper, Loudness::Small, text)
                    .program(j)
                    .person(h.id, &h.name),
            );
        }
        Seat::Club => {
            let m = d.parts().1;
            if !(3..=4).contains(&m) || piled >= 2 || u >= 0.15 * heat || p.stage != Stage::Declared {
                return;
            }
            let Some(order) = crate::world::nfl_year::draft_order(w) else { return };
            let mine = order.iter().position(|&t| t == h.team).unwrap_or(order.len());
            let Some(&t) = order.iter().take(mine).find(|&&t| {
                t < w.clubs.len()
                    && !on.contains(&(t as u32))
                    && crate::career::need(&w.nfl[t], p.player.position) > 0.0
            }) else {
                return;
            };
            let dy = sc::public::draft_year(w);
            for x in w.hunt.ai.iter_mut().filter(|x| x.level == CLUB && x.team == t as u32 && x.year == dy && x.open())
            {
                x.end = Some((d, false));
            }
            w.hunt.ai.push(AiHunt { level: CLUB, team: t as u32, id: h.id, year: dy, since: d, why: PILED, end: None });
            let text = format!("The {} are said to be hot on {} ({}) too.", w.nfl[t].name, h.name, h.pos);
            w.calendar
                .post(Event::new(d, Level::Nfl, CalKind::Whisper, Loudness::Small, text).team(t).person(h.id, &h.name));
        }
        Seat::School => {}
    }
}

/// The open hunts on a player.
pub fn on(w: &World, id: u32) -> Vec<&AiHunt> {
    w.hunt.ai.iter().filter(|h| h.id == id && h.open()).collect()
}

/// Whether the public knows of a team's hunt: a college's offer or a
/// visit, a club's leak, a chase that made the news (piled on, or the
/// generational prospect's); a private school's only once it's done.
pub fn known(w: &World, h: &AiHunt) -> bool {
    if h.why != OWN {
        return true;
    }
    let Some(i) = sc::place(w, h.id) else { return false };
    let p = &w.people[i];
    let t = h.team as usize;
    match h.level {
        COLLEGE => {
            p.cal.offers.contains(&t)
                || p.cal.early_offers.iter().any(|o| o.0 == t)
                || p.cal.visits.iter().any(|v| v.0 == t)
        }
        CLUB => w.scouting.predraft.leaks.iter().any(|l| l.team == t && l.player == h.id),
        _ => h.end.is_some(),
    }
}

/// Each recruit's AI college hunters: player id to programs.
pub(crate) fn college_hunters(w: &World) -> HashMap<u32, Vec<usize>> {
    let mut m: HashMap<u32, Vec<usize>> = HashMap::new();
    for h in w.hunt.ai.iter().filter(|h| h.level == COLLEGE && h.open()) {
        m.entry(h.id).or_default().push(h.team as usize);
    }
    m
}

/// The clubs' hunts for a draft: (club, player id).
pub(crate) fn club_hunts(w: &World, dy: i32) -> Vec<(usize, u32)> {
    w.hunt.ai.iter().filter(|h| h.level == CLUB && h.year == dy && h.open()).map(|h| (h.team as usize, h.id)).collect()
}
