//! Your draft board (topic 9): your director builds a first board from the
//! club's files, with his reasons; you move players, set tiers, take
//! players off the board and mark priority free agents. Kept by player id
//! for the draft it was set for.

use std::collections::HashMap;

use crate::world::World;

/// The tiers as the war room names them (screens topic 8, confirmed by
/// Cain on 2026-10-21): tier, name, what it means.
pub const TIERS: [(&str, &str); 8] = [
    ("Tier 1", "Top-ten grades"),
    ("Tier 2", "Round 1 grades"),
    ("Tier 3", "Late round 1 to round 2"),
    ("Tier 4", "Round 2 grades"),
    ("Tier 5", "Round 2 to round 3"),
    ("Tier 6", "Round 3 grades"),
    ("Tier 7", "Day 3 · rounds 4-5"),
    ("Tier 8", "Day 3 · rounds 6-7"),
];

/// Your board as you set it.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SeatBoard {
    /// The draft this board is for.
    #[serde(default)]
    pub year: i32,
    /// Your order (player ids, best first). Players not in it follow in
    /// your director's order.
    #[serde(default)]
    pub order: Vec<u32>,
    /// Tiers you set by hand (player id, tier 1-8).
    #[serde(default)]
    pub tiers: Vec<(u32, u8)>,
    /// Your do-not-draft list with the reason in your own words.
    #[serde(default)]
    pub dnd: Vec<(u32, String)>,
    /// Priority free agents: player id, bonus and guaranteed salary
    /// offered (dollars).
    #[serde(default)]
    pub pfa: Vec<(u32, u32, u32)>,
    /// Players you put back on the board after your director took them off.
    #[serde(default)]
    pub restored: Vec<u32>,
}

impl SeatBoard {
    /// A board left over from last year's draft starts fresh.
    pub fn for_year(&mut self, year: i32) {
        if self.year != year {
            *self = SeatBoard { year, ..Default::default() };
        }
    }

    pub fn tier_of(&self, id: u32) -> Option<u8> {
        self.tiers.iter().find(|x| x.0 == id).map(|x| x.1)
    }

    pub fn is_dnd(&self, id: u32) -> bool {
        self.dnd.iter().any(|x| x.0 == id)
    }
}

/// The tier a grade lands in, on the aligned draft scale, by where the
/// grade would go in a normal year (the class's own grades decide the
/// lines: the top ten, the first round, and so on).
pub fn tier_of_rank(rank: usize, teams: usize) -> u8 {
    let r = rank as f64 / teams.max(1) as f64;
    if r < 10.0 / 32.0 {
        1
    } else if r < 0.75 {
        2
    } else if r < 1.4 {
        3
    } else if r < 2.0 {
        4
    } else if r < 2.6 {
        5
    } else if r < 3.1 {
        6
    } else if r < 5.0 {
        7
    } else {
        8
    }
}

/// Words for a round grade from a tier.
pub fn round_words(tier: u8) -> &'static str {
    match tier {
        1 => "Top-ten grade",
        2 => "Round 1 grade",
        3 => "Late 1st-2nd grade",
        4 => "Round 2 grade",
        5 => "Round 2-3 grade",
        6 => "Round 3 grade",
        7 => "Round 4-5 grade",
        _ => "Round 6-7 grade",
    }
}

/// The out-on-a-limb rule (confirmed by Cain on 2026-10-21): we have a
/// player far from the public rank, at least 12 places or a third of our
/// rank, whichever is more. 1 higher than the public, -1 lower, 0 not.
pub fn limb(ours: usize, public: Option<usize>) -> i8 {
    let Some(p) = public else { return 0 };
    let r = ours + 1;
    let d = p as i64 + 1 - r as i64;
    if d.unsigned_abs() as f64 >= (12.0f64).max((r as f64 * 0.35).round()) { if d > 0 { 1 } else { -1 } } else { 0 }
}

/// Your board in order: (player id, your club's grade on the aligned scale)
/// for every prospect the club has a file on or the public ranks, your own
/// moves first, then the director's order.
pub fn ordered(w: &World, grades: &HashMap<u32, f64>) -> Vec<u32> {
    let b = &w.scouting.board;
    let mut dir: Vec<(u32, f64)> = grades.iter().map(|(&k, &v)| (k, v)).collect();
    dir.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut out: Vec<u32> = b.order.iter().copied().filter(|id| grades.contains_key(id)).collect();
    for (id, _) in dir {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// Your club's grade of every prospect in the coming draft (player id to
/// the aligned grade): the live draft's boards while it runs, otherwise
/// your club's board built now from its files the way draft morning builds
/// it (the declared class, plus the seniors in the fall).
pub fn seat_grades(w: &World) -> HashMap<u32, f64> {
    if let Some(l) = &w.scouting.live {
        return super::live::grades(w, l);
    }
    let Some(t) = w.seat.and_then(|s| s.nfl_team) else { return HashMap::new() };
    if t >= w.clubs.len() {
        return HashMap::new();
    }
    let pool = super::public::prospects(w);
    let raw: Vec<(crate::positions::Position, f64)> = pool
        .iter()
        .filter_map(|&i| w.people[i].cal.draft_read.map(|r| (w.people[i].player.position, r.grade)))
        .collect();
    if raw.is_empty() {
        return HashMap::new();
    }
    let align = crate::level_rating::PositionAligner::for_draft(
        &raw,
        crate::world::recruiting::full_draft(w, coming_slots(w).len()),
    );
    let mut aligned: Vec<(usize, f64)> = pool
        .iter()
        .filter_map(|&i| w.people[i].cal.draft_read.map(|r| (i, align.align(w.people[i].player.position, r.grade))))
        .collect();
    aligned.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let b = crate::world::pro::scouts::board_for(w, t, &aligned, &align);
    aligned.iter().map(|&(i, g)| (w.people[i].player.id, b.grade.get(&i).copied().unwrap_or(g))).collect()
}

/// The public's rank of each prospect (player id to rank from 0): the
/// latest consensus board, else the services' grade order.
pub fn public_ranks(w: &World) -> HashMap<u32, usize> {
    if let Some(l) = &w.scouting.live {
        return l.run.public_rank.iter().map(|(&i, &r)| (w.people[i].player.id, r)).collect();
    }
    let top = super::public::consensus(w).unwrap_or_default();
    let mut v: Vec<(f64, u32)> = super::public::prospects(w)
        .into_iter()
        .filter_map(|i| w.people[i].cal.draft_read.map(|r| (r.score, w.people[i].player.id)))
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut order = top.clone();
    order.extend(v.into_iter().map(|x| x.1).filter(|id| !top.contains(id)));
    order.into_iter().enumerate().map(|(k, id)| (id, k)).collect()
}

/// The coming draft's slots as they stand today: (club, round) by pick,
/// the way draft morning sets them (last season's order, traded picks to
/// their owners, comp picks), sized to the world. Before the season ends
/// the order is the clubs' current one.
pub fn coming_slots(w: &World) -> Vec<(usize, u32)> {
    if let Some(l) = &w.scouting.live {
        return l.run.board.clone();
    }
    let year = super::public::draft_year(w);
    let seed = w.options.seed as u64;
    let order = if year == w.year { crate::world::nfl_year_order(w) } else { None };
    let base: Vec<usize> = order
        .unwrap_or_else(|| crate::world::franchise::active_teams_in(w, year))
        .into_iter()
        .filter(|&t| crate::world::franchise::active_in(w, t, year))
        .collect();
    let mut board = crate::world::franchise::draft_board(
        year,
        &base,
        w.options.scale,
        seed,
        &crate::world::franchise::first_draft_in(w, year),
    );
    crate::world::pro::trades::board_owners(w, year, &mut board, &base);
    board
}
