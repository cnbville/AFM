//! What a rating means at a player's level (update 2, RR1): every range on
//! screen is read against the world's own players at his level and
//! position this season. A team's two-deep is its best players at the
//! position by their rating there (`game::roster::starters` a side: three
//! receivers, two corners, one quarterback); "a typical starter" is the
//! middle starter in the pool, "a typical backup" the middle backup, and
//! elite starts where the best tenth of the starters does.
//!
//! High school is read by state classification and graduating class (a
//! junior against the juniors in Texas 6A), widened to the whole state and
//! then the nation when a pool has too few starters to say anything;
//! college by FBS, FCS and the smaller colleges; the NFL as one league.
//!
//! These are true values of the world's players, read in bulk: a mark
//! tells what a rating means, never what any one player has. Sizes are
//! Claude's picks (proposed, not confirmed): 12 starters at least before a
//! pool is used, the 90th percentile of starters as the elite line, and
//! about 1,500 schools sampled for a national high school pool.

use super::data::Division;
use super::{Stage, World};
use crate::attributes::Attr;
use crate::game::roster::starters;
use crate::player::Player;
use crate::positions::{Position, overall_for};

/// The fewest starters a pool needs before its marks are used.
const MIN_STARTERS: usize = 12;
/// About this many schools are read for a national high school pool.
const NATIONAL_SCHOOLS: usize = 1500;

/// The players a rating is read against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pool {
    /// High school: a state (None for the nation), a classification within
    /// it (0 = its biggest; None for every class) and a graduating class
    /// (0: every class, a whole varsity's starters).
    Hs {
        state: Option<usize>,
        class: Option<u32>,
        grade: i32,
    },
    /// College: 0 FBS, 1 FCS, 2 the smaller colleges (Division II, III,
    /// the NAIA and junior colleges).
    College(u8),
    Nfl,
}

/// What is read: his overall at his position, one attribute, or his
/// overall at another spot (the "where else he could play" reads).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rated {
    Overall,
    Attr(Attr),
    At(Position),
}

/// A pool's marks for one rating at one position, on the 1-99 scale.
#[derive(Clone, Debug)]
pub struct Marks {
    /// The pool used (wider than asked when the narrow one was too small).
    pub pool: Pool,
    pub pos: Position,
    /// The middle backup, the middle starter and where the best tenth of
    /// the starters begins.
    pub backup: f64,
    pub starter: f64,
    pub elite: f64,
    /// Every starter's value, low to high.
    pub starters: Vec<f32>,
    pub backups: usize,
    pub teams: usize,
}

impl Marks {
    /// The share of the pool's starters below a value, 0-1.
    pub fn below(&self, v: f64) -> f64 {
        if self.starters.is_empty() {
            return 0.0;
        }
        let n = self.starters.partition_point(|&x| (x as f64) < v);
        n as f64 / self.starters.len() as f64
    }
}

/// The pool a person's ratings are read against, by where he plays now.
pub fn pool_of(w: &World, k: usize) -> Option<Pool> {
    let p = w.people.get(k)?;
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => {
            let s = w.high_schools.get(p.hs)?;
            Some(Pool::Hs { state: Some(s.state), class: Some(s.classification), grade: p.class_of })
        }
        Stage::College => Some(Pool::College(tier(w.programs.get(p.college?)?.division))),
        Stage::Pro | Stage::Declared => Some(Pool::Nfl),
        _ => None,
    }
}

/// A college division's tier: FBS, FCS or the smaller colleges.
pub fn tier(d: Division) -> u8 {
    match d {
        Division::FBS => 0,
        Division::FCS => 1,
        _ => 2,
    }
}

fn value(p: &Player, pos: Position, rated: Rated) -> f64 {
    match rated {
        Rated::Attr(a) => p.attributes.get(a),
        Rated::Overall | Rated::At(_) => overall_for(pos, &p.attributes),
    }
}

/// One team's two-deep at a position, by the rating there: its starters'
/// and backups' values, counting only those `keep` lets in.
fn two_deep(
    w: &World,
    roster: &[usize],
    stage: Stage,
    pos: Position,
    rated: Rated,
    keep: &dyn Fn(usize) -> bool,
    out: &mut (Vec<f32>, Vec<f32>),
) {
    let mut at: Vec<(f64, usize)> = roster
        .iter()
        .filter_map(|&i| w.people.get(i).map(|p| (i, p)))
        .filter(|(_, p)| p.stage == stage && p.player.position == pos)
        .map(|(i, p)| (overall_for(pos, &p.player.attributes), i))
        .collect();
    at.sort_by(|a, b| b.0.total_cmp(&a.0));
    let n = starters(pos);
    for (rank, &(_, i)) in at.iter().enumerate().take(2 * n) {
        if keep(i) {
            let v = value(&w.people[i].player, pos, rated) as f32;
            if rank < n { out.0.push(v) } else { out.1.push(v) }
        }
    }
}

/// The marks for a rating at a position in a pool. A high school pool too
/// small to say anything widens to the state, then the nation; None when
/// even that has no starters.
pub fn marks(w: &World, pool: Pool, pos: Position, rated: Rated) -> Option<Marks> {
    let pos = match rated {
        Rated::At(p) => p,
        _ => pos,
    };
    let mut tries = vec![pool];
    if let Pool::Hs { state, class, grade } = pool {
        if class.is_some() {
            tries.push(Pool::Hs { state, class: None, grade });
        }
        if state.is_some() {
            tries.push(Pool::Hs { state: None, class: None, grade });
        }
    }
    let last = tries.len() - 1;
    for (k, &p) in tries.iter().enumerate() {
        let (mut st, bk, teams) = collect(w, p, pos, rated);
        if st.len() >= MIN_STARTERS || (k == last && !st.is_empty()) {
            st.sort_by(|a, b| a.total_cmp(b));
            let mut b = bk;
            b.sort_by(|a, b| a.total_cmp(b));
            let at = |v: &[f32], q: f64| v[((v.len() - 1) as f64 * q).round() as usize] as f64;
            let starter = at(&st, 0.5);
            return Some(Marks {
                pool: p,
                pos,
                backup: if b.is_empty() { starter } else { at(&b, 0.5) },
                starter,
                elite: at(&st, 0.9),
                backups: b.len(),
                starters: st,
                teams,
            });
        }
    }
    None
}

fn collect(w: &World, pool: Pool, pos: Position, rated: Rated) -> (Vec<f32>, Vec<f32>, usize) {
    let mut out = (Vec::new(), Vec::new());
    let mut teams = 0;
    match pool {
        Pool::Nfl => {
            for t in 0..w.clubs.len() {
                if super::franchise::active(t, w.year) {
                    two_deep(w, &w.clubs[t].roster, Stage::Pro, pos, rated, &|_| true, &mut out);
                    teams += 1;
                }
            }
        }
        Pool::College(t) => {
            for prog in &w.programs {
                if tier(prog.division) == t && prog.plays(w.year) {
                    two_deep(w, &prog.roster, Stage::College, pos, rated, &|_| true, &mut out);
                    teams += 1;
                }
            }
        }
        Pool::Hs { state, class, grade } => {
            let keep = |i: usize| grade == 0 || w.people[i].class_of == grade;
            let schools: Vec<usize> = match state {
                Some(s) => w.states[s].schools.clone(),
                None => {
                    let step = (w.high_schools.len() / NATIONAL_SCHOOLS).max(1);
                    (0..w.high_schools.len()).step_by(step).collect()
                }
            };
            for h in schools {
                let s = &w.high_schools[h];
                if class.is_none_or(|c| s.classification == c) {
                    two_deep(w, &s.roster, Stage::HighSchool, pos, rated, &keep, &mut out);
                    teams += 1;
                }
            }
        }
    }
    (out.0, out.1, teams)
}
