//! The public evaluators (topics 2, 4, 10, 18): recruiting services and
//! draft analysts, each a desk with its own reads, lean and track record,
//! under made-up names (the real services are trademarks; Cain, 2026-10-21).
//! The services rank recruits from their era's start (the composite from
//! 2012); the analysts publish big boards from the fall and mock drafts
//! from the bowls to the draft, weekly from January, and the media scores
//! the mocks the morning after.
//!
//! A desk's read of a player is the public grade plus the desk's lasting
//! lean on him (keyed by desk and player, so it never moves without a new
//! look) and the week's noise, tilted by its bias. Only the latest board
//! and mock of each desk and the weekly consensus top 50 are kept.

use std::collections::HashMap;

use crate::calendar::{Date, Weekday};
use crate::rng::{Rng, clamp};
use crate::world::season::key_normal;
use crate::world::{Stage, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DeskKind {
    /// A recruiting service: stars and ranks for high school players.
    Service,
    /// A draft analyst: big boards and mocks.
    Analyst,
}

/// What a desk leans toward (its bias, by a little).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Lean {
    /// Big programs' camps and players (services) or Power conference
    /// players (analysts).
    BigPrograms,
    /// Players who go viral, testing freaks.
    Viral,
    /// Production and the stat sheet.
    Production,
    /// Film and technique.
    Tape,
    /// Size and length.
    Size,
}

impl Lean {
    pub fn words(self) -> &'static str {
        match self {
            Lean::BigPrograms => "leans toward the big programs",
            Lean::Viral => "loves the testing freaks and the viral clips",
            Lean::Production => "trusts production",
            Lean::Tape => "a tape grinder",
            Lean::Size => "loves size and length",
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Desk {
    pub name: String,
    /// The analyst's or the desk's outlet.
    pub outlet: String,
    pub kind: DeskKind,
    pub from: i32,
    pub to: Option<i32>,
    pub lean: Lean,
    /// Hidden skill, 1-20: how small its misread.
    pub skill: u8,
    pub key: u64,
    /// Its mock drafts' record: (drafts scored, exact hits, within three).
    #[serde(default)]
    pub record: (u16, u16, u16),
    /// Its big boards' record, the last ten drafts: (draft, how many of its
    /// final top 100 went in the first 100 picks).
    #[serde(default)]
    pub board_hits: Vec<(i32, u8)>,
}

impl Desk {
    pub fn active(&self, year: i32) -> bool {
        year >= self.from && self.to.is_none_or(|t| year <= t)
    }
}

/// This draft season's public word.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Season {
    /// The draft these boards are for.
    pub year: i32,
    /// Each desk's latest big board: desk, date, player ids best first (100).
    pub boards: Vec<(u8, Date, Vec<u32>)>,
    /// Each desk's latest mock: desk, date, player ids by pick (round 1;
    /// two rounds in the last week).
    pub mocks: Vec<(u8, Date, Vec<u32>)>,
    /// The weekly consensus top 50: date, ids.
    pub weekly: Vec<(Date, Vec<u32>)>,
    /// Each prospect's best spot in this season's mocks (player id, pick).
    pub best_mock: HashMap<u32, u16>,
    /// Last draft's mocks scored: desk, exact hits, within three, of how many.
    pub scored: Vec<(u8, u16, u16, u16)>,
}

/// The desks by era (made-up names; real counterparts: Rivals 1998,
/// Scout.com 1998-2017, ESPN 2006, 247Sports 2010 with its composite from
/// 2012, On3 2021; Mel Kiper since 1978, Mike Mayock 2000s-2017, Dane
/// Brugler from about 2010, a film analyst of the 2010s).
pub(crate) fn staff_desks(w: &mut World) {
    let seed = w.options.seed;
    let mut rng = Rng::new(seed ^ 0xDE5C);
    let services: [(&str, &str, i32, Option<i32>, Lean); 5] = [
        ("The Prep Report", "Prep Report Network", 1998, None, Lean::BigPrograms),
        ("ScoutLine", "ScoutLine", 1998, Some(2017), Lean::Production),
        ("Sports Network 300", "Sports Network", 2006, None, Lean::Size),
        ("Gridiron 360", "Gridiron 360", 2010, None, Lean::Viral),
        ("On Field", "On Field", 2021, None, Lean::Tape),
    ];
    let mut desks = Vec::new();
    for (name, outlet, from, to, lean) in services {
        desks.push(Desk {
            name: name.into(),
            outlet: outlet.into(),
            kind: DeskKind::Service,
            from,
            to,
            lean,
            skill: clamp(rng.normal(12.0, 2.0), 7.0, 17.0).round() as u8,
            key: rng.next_u32() as u64 | 0x5E00_0000_0000,
            record: (0, 0, 0),
            board_hits: Vec::new(),
        });
    }
    let analysts: [(&str, i32, Option<i32>, Lean, i32); 4] = [
        ("Sports Network", 1978, None, Lean::Size, 1950),
        ("League Network", 2004, Some(2017), Lean::Tape, 1962),
        ("The Athletic Desk", 2010, None, Lean::Tape, 1978),
        ("Film Room Daily", 2014, None, Lean::Production, 1986),
    ];
    for (outlet, from, to, lean, born) in analysts {
        let eth = crate::world::ethnicity::Ethnicity::White;
        let name = crate::world::names::person_name(eth, born, &mut rng);
        desks.push(Desk {
            name,
            outlet: outlet.into(),
            kind: DeskKind::Analyst,
            from,
            to,
            lean,
            skill: clamp(rng.normal(12.0, 2.0), 8.0, 17.0).round() as u8,
            key: rng.next_u32() as u64 | 0xA1_0000_0000,
            record: (0, 0, 0),
            board_hits: Vec::new(),
        });
    }
    w.scouting.desks = desks;
}

/// The draft the public is working toward.
pub fn draft_year(w: &World) -> i32 {
    if w.today <= w.dates.draft[0].date { w.year } else { w.year + 1 }
}

/// A desk's read of a draft prospect on the aligned draft scale: the
/// public grade plus its lasting lean and the week's noise, tilted by its
/// bias by a little.
pub fn desk_grade(w: &World, desk: &Desk, i: usize, week: i32) -> Option<f64> {
    let p = &w.people[i];
    let r = p.cal.draft_read?;
    let id = p.player.id as u64;
    let sd = 1.4 - 0.04 * desk.skill as f64;
    let lean = key_normal(desk.key ^ 0x1EA4, id);
    let noise = key_normal(desk.key ^ 0x0153 ^ (week as u64) << 30, id);
    let mut g = r.score + 3.5 * sd * (0.8 * lean + 0.35 * noise);
    let big = p
        .college
        .is_some_and(|c| w.programs[c].division == crate::world::data::Division::FBS && w.programs[c].prestige >= 60.0);
    let size = crate::body::combine_size(p.player.position);
    let z_size = clamp((p.player.body.weight - size.2) / size.3.max(1.0), -1.5, 1.5);
    g += match desk.lean {
        Lean::BigPrograms if big => 1.0,
        Lean::Size => 0.8 * z_size,
        Lean::Viral => 0.6 * clamp((p.player.attributes.get(crate::attributes::Attr::Speed) - 75.0) / 8.0, -1.5, 1.5),
        Lean::Production => 0.6 * clamp(p.last_performance.unwrap_or(0.0), -1.5, 1.5),
        _ => 0.0,
    };
    // The tape's shared leans (SK2, SK4): the desks watch the same games
    // and read the same numbers; by the spring's all-star games and the
    // combine the best have been seen against good players.
    let f = super::footage::footage(w, i);
    let spring = matches!(w.today.parts().1, 2..=4);
    g += super::footage::scout_tilt(&f, desk.skill as f64, spring, false)
        - super::footage::scout_tilt(&f, 12.0, false, false);
    Some(g)
}

/// The prospects of the coming draft the public ranks: declared players,
/// and before January the seniors with a read.
pub fn prospects(w: &World) -> Vec<usize> {
    let dy = draft_year(w);
    (0..w.people.len())
        .filter(|&i| {
            let p = &w.people[i];
            p.cal.draft_read.is_some()
                && (p.stage == Stage::Declared
                    || (p.stage == Stage::College && dy - p.class_of >= 4 && w.today.parts().1 >= 8))
        })
        .collect()
}

/// The latest consensus top 50 for the coming draft.
pub fn consensus(w: &World) -> Option<Vec<u32>> {
    let s = &w.scouting.public;
    if s.year != draft_year(w) {
        return None;
    }
    s.weekly.last().map(|x| x.1.clone())
}

/// A desk's big board: the top 100 by its read.
pub fn big_board(w: &World, desk: &Desk, pool: &[usize], week: i32) -> Vec<u32> {
    let mut v: Vec<(f64, u32)> =
        pool.iter().filter_map(|&i| desk_grade(w, desk, i, week).map(|g| (g, w.people[i].player.id))).collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    v.truncate(100);
    v.into_iter().map(|x| x.1).collect()
}

/// A desk's mock: each pick in order goes to the best player on its board
/// weighed by the club's need as the desk reads it.
pub fn mock(w: &World, desk: &Desk, pool: &[usize], week: i32, picks: usize) -> Vec<u32> {
    // While the season runs, the standings so far: never last April's order.
    let order = super::super::nfl_year::mock_order(w).unwrap_or_default();
    if order.is_empty() {
        return Vec::new();
    }
    let mut cands: Vec<(f64, usize)> =
        pool.iter().filter_map(|&i| desk_grade(w, desk, i, week).map(|g| (g, i))).collect();
    cands.sort_by(|a, b| b.0.total_cmp(&a.0));
    cands.truncate(picks * 3);
    let mut out = Vec::new();
    for k in 0..picks {
        let t = order[k % order.len()];
        let Some((idx, _)) = cands
            .iter()
            .enumerate()
            .map(|(j, &(g, i))| {
                let need =
                    if t < w.nfl.len() { crate::career::need(&w.nfl[t], w.people[i].player.position) } else { 0.0 };
                (j, g + 0.8 * need)
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
        else {
            break;
        };
        let (_, i) = cands.remove(idx);
        out.push(w.people[i].player.id);
    }
    out
}

/// The day's public work: big boards on the first of the month from
/// September and weekly from January, mocks weekly from mid-December, the
/// consensus kept weekly with its risers and fallers in the news.
pub(crate) fn day(w: &mut World, d: Date) {
    let dy = draft_year(w);
    if w.scouting.public.year != dy {
        let scored = std::mem::take(&mut w.scouting.public.scored);
        w.scouting.public = Season { year: dy, scored, ..Default::default() };
    }
    let (y, m, day) = d.parts();
    // From September to the spring's draft. The football year's own dates
    // hold the draft before its fall, so the draft these boards are for is
    // `dy`: in the spring only until that draft is held.
    let in_season = match m {
        9..=12 => true,
        1..=4 => dy == y,
        _ => false,
    };
    if !in_season {
        return;
    }
    let weekly = matches!(m, 1..=4) || (m == 12 && day >= 15);
    let board_day = (day == 1 && matches!(m, 9..=12)) || (weekly && d.weekday() == Weekday::Tue);
    let mock_day = weekly && d.weekday() == Weekday::Thu;
    if !board_day && !mock_day {
        return;
    }
    let pool = prospects(w);
    if pool.is_empty() {
        return;
    }
    let week = d.0 / 7;
    let desks: Vec<(u8, Desk)> = w
        .scouting
        .desks
        .iter()
        .enumerate()
        .filter(|(_, x)| x.kind == DeskKind::Analyst && x.active(dy))
        .map(|(k, x)| (k as u8, x.clone()))
        .collect();
    if board_day {
        let mut tally: HashMap<u32, f64> = HashMap::new();
        for (k, desk) in &desks {
            let b = big_board(w, desk, &pool, week);
            for (r, id) in b.iter().enumerate() {
                *tally.entry(*id).or_default() += 100.0 - r as f64;
            }
            let s = &mut w.scouting.public;
            s.boards.retain(|x| x.0 != *k);
            s.boards.push((*k, d, b));
        }
        let mut cons: Vec<(f64, u32)> = tally.into_iter().map(|(id, v)| (v, id)).collect();
        cons.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let top: Vec<u32> = cons.into_iter().take(50).map(|x| x.1).collect();
        risers_and_fallers(w, d, &top);
        w.scouting.public.weekly.push((d, top));
        if w.scouting.public.weekly.len() > 30 {
            w.scouting.public.weekly.remove(0);
        }
    }
    if mock_day {
        let draft_day =
            if dy == w.year { w.dates.draft[0].date } else { crate::calendar::YearDates::new(dy).draft[0].date };
        let last_week = d.add(7) > draft_day;
        let teams = crate::world::franchise::active_teams(dy).len().max(1);
        for (k, desk) in &desks {
            let n = if last_week { 2 * teams } else { teams };
            let mk = mock(w, desk, &pool, week, n);
            let s = &mut w.scouting.public;
            for (pick, id) in mk.iter().enumerate() {
                let e = s.best_mock.entry(*id).or_insert(u16::MAX);
                *e = (*e).min(pick as u16 + 1);
            }
            s.mocks.retain(|x| x.0 != *k);
            s.mocks.push((*k, d, mk));
        }
    }
}

/// The weekly consensus against last week's: a big riser or faller in the
/// top 50 makes the news (the media season, topic 10).
fn risers_and_fallers(w: &mut World, d: Date, top: &[u32]) {
    let Some((_, last)) = w.scouting.public.weekly.last().cloned() else { return };
    let mut moves: Vec<(i32, u32)> = Vec::new();
    for (r, id) in top.iter().enumerate() {
        let before = last.iter().position(|x| x == id).map_or(60, |k| k as i32);
        moves.push((before - r as i32, *id));
    }
    moves.sort_by_key(|m| -m.0);
    if let Some(&(up, id)) = moves.first()
        && up >= 10
        && let Some(i) = super::place(w, id)
    {
        let p = &w.people[i];
        let school = p.college.map_or(String::new(), |c| w.program_name(c).to_string());
        let rank = top.iter().position(|x| *x == id).unwrap_or(0) + 1;
        let text = format!(
            "Draft stock: {} ({}, {school}) climbs {up} places on the public boards to No. {rank}.",
            p.name, p.player.position
        );
        let mut e = crate::calendar::Event::new(
            d,
            crate::calendar::Level::Nfl,
            crate::calendar::Kind::News,
            crate::calendar::Loudness::Small,
            text,
        );
        if let Some(c) = p.college {
            e = e.program(c);
        }
        w.calendar.post(e);
    }
}

/// The morning after: each desk's last mock scored against the picks
/// (exact player at the pick, and within three picks), its record kept.
pub(crate) fn score_mocks(w: &mut World, board: &[(usize, u32)]) {
    let _ = board;
    let picks: HashMap<u32, u32> = w
        .people
        .iter()
        .filter_map(|p| {
            let c = p.pro.as_ref()?;
            let (y, _, pick, _) = c.draft?;
            (y == w.year).then_some((p.player.id, pick))
        })
        .collect();
    let mut scored = Vec::new();
    for (k, _, mk) in w.scouting.public.mocks.clone() {
        let n = mk.len() as u16;
        let mut exact = 0u16;
        let mut near = 0u16;
        for (j, id) in mk.iter().enumerate() {
            if let Some(&pk) = picks.get(id) {
                let d = (pk as i64 - (j as i64 + 1)).abs();
                if d == 0 {
                    exact += 1;
                }
                if d <= 3 {
                    near += 1;
                }
            }
        }
        if let Some(desk) = w.scouting.desks.get_mut(k as usize) {
            desk.record.0 += 1;
            desk.record.1 += exact;
            desk.record.2 += near;
        }
        scored.push((k, exact, near, n));
    }
    w.scouting.public.scored = scored;
    // Each big board's final top 100 against the first 100 picks.
    let year = w.year;
    for (k, _, ids) in w.scouting.public.boards.clone() {
        let Some(hits) = board_hit_rate(&ids, &picks) else { continue };
        if let Some(desk) = w.scouting.desks.get_mut(k as usize) {
            desk.board_hits.retain(|x| x.0 != year);
            desk.board_hits.push((year, hits));
            if desk.board_hits.len() > 10 {
                desk.board_hits.remove(0);
            }
        }
    }
}

/// How many of a board's top 100 went in the first 100 picks, per 100. A
/// small world's draft has fewer picks (`franchise::draft_board`), so its
/// top N is scored against the first N picks, N the last pick up to 100;
/// None when no pick was made.
pub(crate) fn board_hit_rate(ids: &[u32], picks: &HashMap<u32, u32>) -> Option<u8> {
    let n = picks.values().copied().filter(|pk| (1..=100).contains(pk)).max()? as usize;
    let hits = ids.iter().take(n).filter(|id| picks.get(id).is_some_and(|&pk| pk as usize <= n)).count();
    Some((100.0 * hits as f64 / n as f64).round().min(100.0) as u8)
}

/// A service's read of a recruit (topic 14): its star rating and national
/// rank from the shared film read and its own lean; the composite averages
/// the services (from 2012).
pub fn service_read(w: &World, desk: &Desk, i: usize) -> (u8, f64) {
    let p = &w.people[i];
    let id = p.player.id as u64;
    let base = super::super::recruiting::film_raw(p);
    let sd = 1.4 - 0.04 * desk.skill as f64;
    let mut g = base + 3.0 * sd * key_normal(desk.key ^ 0x5E41, id);
    let big_camp = p.camp_stars >= 4;
    g += match desk.lean {
        Lean::BigPrograms if big_camp => 1.2,
        Lean::Viral if p.player.attributes.get(crate::attributes::Attr::Speed) >= 80.0 => 1.0,
        Lean::Size => {
            0.5 * clamp((p.player.body.weight - crate::body::combine_size(p.player.position).2) / 20.0, -1.0, 1.0)
        }
        _ => 0.0,
    };
    // Stars stay within one of the shared read (services agree on most kids).
    let s = p.stars as f64 + clamp((g - base) / 4.0, -0.6, 0.6);
    (s.round().clamp(2.0, 5.0) as u8, g)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_board_is_scored_against_the_picks_the_draft_had() {
        // A full draft: 100 of the first 100 picks, 70 of them from the board.
        let mut picks: HashMap<u32, u32> = (0..224u32).map(|k| (1000 + k, k + 1)).collect();
        let board: Vec<u32> = (0..70u32).map(|k| 1000 + k).chain((0..30u32).map(|k| 5000 + k)).collect();
        assert_eq!(board_hit_rate(&board, &picks), Some(70));
        // A small world's 13 picks: its top 13 against them, as a share.
        picks.retain(|_, pk| *pk <= 13);
        let small: Vec<u32> = (0..10u32).map(|k| 1000 + k).chain((0..90u32).map(|k| 5000 + k)).collect();
        assert_eq!(board_hit_rate(&small, &picks), Some(77));
        // Only the top 13 are read: a pick lower on the board doesn't count.
        let late: Vec<u32> = (0..13u32).map(|k| 5000 + k).chain((0..13u32).map(|k| 1000 + k)).collect();
        assert_eq!(board_hit_rate(&late, &picks), Some(0));
        assert_eq!(board_hit_rate(&board, &HashMap::new()), None);
    }
}
