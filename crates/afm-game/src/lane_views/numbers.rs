//! Jersey numbers for the game views. A team's men wear the numbers its
//! roster pages show (`afm_engine::world::stories::numbers`: the era's
//! rules, retired numbers out); a man the team no longer lists (gone since
//! the game) gets one here, inside his level's real numbering rules, from
//! the numbers left, and the same one every time you watch (his id picks
//! it). Never two alike on one side of the ball; a big college or high
//! school roster doubles up across it, as real ones do.
//!
//! The rules: the NFL since 2021 (quarterbacks, kickers and punters 1-19;
//! backs and receivers 1-49 and 80-89; the line 50-79; linemen on defense
//! 50-79 and 90-99; linebackers 1-59 and 90-99; defensive backs 1-49), and
//! 0 from 2023. Before 2021 the NFL's tighter 1973 rules. College and high
//! school (NCAA and NFHS rules): any number 0-99, but the five interior
//! linemen of the offense must wear 50-79, so linemen do.

use std::collections::{HashMap, HashSet};

use afm_engine::calendar::Level as CalLevel;
use afm_engine::positions::Position;
use afm_engine::world::World;
use afm_engine::world::stories::{TeamKey, numbers};

/// Where a player's number most often comes from: ranges with weights, in
/// the order real teams hand them out.
fn ranges(pos: Position, nfl: bool, year: i32) -> &'static [(u8, u8, u32)] {
    use Position::*;
    if nfl && year < 2021 {
        return match pos {
            QB | K | P => &[(1, 19, 1)],
            RB => &[(20, 49, 1)],
            WR => &[(10, 19, 1), (80, 89, 3)],
            TE => &[(80, 89, 3), (40, 49, 1)],
            OT | IOL => &[(60, 79, 3), (50, 59, 1)],
            EDGE | DT => &[(90, 99, 3), (60, 79, 2)],
            LB => &[(50, 59, 3), (90, 99, 1)],
            CB | S => &[(20, 49, 1)],
            LS => &[(40, 49, 1), (90, 99, 1)],
        };
    }
    match pos {
        QB => &[(1, 19, 6), (0, 0, 1)],
        K | P => &[(1, 19, 4), (90, 99, 1)],
        RB => &[(20, 39, 5), (0, 9, 2)],
        WR => &[(1, 19, 6), (80, 89, 2)],
        TE => &[(80, 89, 5), (40, 49, 1)],
        OT | IOL => &[(60, 79, 4), (50, 59, 2)],
        EDGE => &[(90, 99, 4), (40, 59, 2), (1, 19, 1)],
        DT => &[(90, 99, 5), (50, 79, 2)],
        LB => &[(40, 59, 4), (1, 39, 2), (90, 99, 1)],
        CB => &[(20, 39, 4), (1, 19, 3), (0, 0, 1)],
        S => &[(20, 39, 4), (1, 19, 2)],
        LS => &[(40, 49, 2), (60, 69, 1), (90, 99, 1)],
    }
}

fn mix(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^ (x >> 33)
}

/// Which side of the ball a position plays (1 for the defense): the two
/// men who share a number on a big roster are never on the field together.
pub fn defense(pos: Position) -> usize {
    matches!(pos, Position::EDGE | Position::DT | Position::LB | Position::CB | Position::S) as usize
}

/// A team's numbers in the film: the ones it wears on its roster pages
/// now, and for anyone it no longer lists, one by the rules below from
/// the numbers left. A man who played on the other side of the ball in
/// this game (`crossed`: a corner at receiver, an emergency) and shares
/// his number with a man there wears one nobody in the game wears, as a
/// real team hands him another jersey: two men who share a number are
/// never on the field for one snap.
pub fn for_team(
    w: &World,
    cal: CalLevel,
    team: usize,
    players: &[(u32, Position)],
    year: i32,
    crossed: &HashSet<u32>,
) -> HashMap<u32, u8> {
    let key = match cal {
        // Another league's club keeps no numbers between seasons: each
        // man picks his own, the pro way.
        CalLevel::Other => return assign_with(players, true, year, [[false; 100]; 2]).into_iter().collect(),
        CalLevel::Nfl => TeamKey::Nfl(team as u16),
        CalLevel::HighSchool => TeamKey::Hs(team as u32),
        _ => TeamKey::College(team as u32),
    };
    let worn = numbers::numbers(w, key);
    let mut out: HashMap<u32, u8> = players.iter().filter_map(|(id, _)| worn.get(id).map(|&n| (*id, n))).collect();
    let mut taken = [[false; 100]; 2];
    for &n in &w.stories.traditions.retired_numbers(key) {
        taken[0][n as usize] = true;
        taken[1][n as usize] = true;
    }
    for &(id, pos) in players {
        if let Some(&n) = out.get(&id) {
            taken[defense(pos)][n as usize] = true;
        }
    }
    let rest: Vec<(u32, Position)> = players.iter().filter(|(id, _)| !out.contains_key(id)).copied().collect();
    let nfl = cal == CalLevel::Nfl;
    out.extend(assign_with(&rest, nfl, year, taken));
    own_numbers(&mut out, players, nfl, year, taken, crossed);
    out
}

/// A man in `crossed` whose number a man on the other side of the ball
/// wears in this game takes one free on both sides (`for_team`); `taken`
/// holds the retired numbers.
fn own_numbers(
    out: &mut HashMap<u32, u8>,
    players: &[(u32, Position)],
    nfl: bool,
    year: i32,
    mut taken: [[bool; 100]; 2],
    crossed: &HashSet<u32>,
) {
    for &(id, pos) in players {
        taken[defense(pos)][out[&id] as usize] = true;
    }
    for &(id, pos) in players.iter().filter(|(id, _)| crossed.contains(id)) {
        let side = defense(pos);
        if !taken[1 - side][out[&id] as usize] {
            continue;
        }
        let free = |n: u8| !taken[0][n as usize] && !taken[1][n as usize];
        if let Some(n) =
            options(id, pos, nfl, year).into_iter().find(|&n| free(n)).or_else(|| (1..100u8).find(|&n| free(n)))
        {
            taken[side][n as usize] = true;
            out.insert(id, n);
        }
    }
}

/// Numbers for one team's players (id and position), stars first so the
/// best players keep the numbers they would pick.
pub fn assign(players: &[(u32, Position)], nfl: bool, year: i32) -> Vec<(u32, u8)> {
    assign_with(players, nfl, year, [[false; 100]; 2])
}

/// The numbers a man would wear, in the order he would pick them: his
/// favourite range first (by the real shares), then the others.
fn options(id: u32, pos: Position, nfl: bool, year: i32) -> Vec<u8> {
    let rs = ranges(pos, nfl, year);
    let zero_ok = !nfl || year >= 2023;
    let total: u32 = rs.iter().map(|r| r.2).sum();
    let mut pick = (mix(id as u64 ^ 0x4A45_5253) % total as u64) as u32;
    let fav = rs
        .iter()
        .position(|r| {
            if pick < r.2 {
                true
            } else {
                pick -= r.2;
                false
            }
        })
        .unwrap_or(0);
    let order = std::iter::once(&rs[fav]).chain(rs.iter().enumerate().filter(|(k, _)| *k != fav).map(|(_, r)| r));
    let mut options: Vec<u8> = Vec::new();
    for &(lo, hi, _) in order {
        let span = (hi - lo) as u64 + 1;
        let start = mix(id as u64 ^ 0x4E55_4D42) % span;
        for k in 0..span {
            let n = lo + ((start + k) % span) as u8;
            if n == 0 && !zero_ok {
                continue;
            }
            options.push(n);
        }
    }
    options
}

/// `assign` around the numbers already worn on each side of the ball. A
/// number free on both sides comes first; past that a college or high
/// school man shares one with a man on the other side of the ball.
fn assign_with(players: &[(u32, Position)], nfl: bool, year: i32, mut taken: [[bool; 100]; 2]) -> Vec<(u32, u8)> {
    let mut out = Vec::with_capacity(players.len());
    for &(id, pos) in players {
        let side = defense(pos);
        let options = options(id, pos, nfl, year);
        let free = |n: u8, t: &[[bool; 100]; 2]| !t[0][n as usize] && !t[1][n as usize];
        let ours = |n: u8, t: &[[bool; 100]; 2]| !t[side][n as usize];
        let n = options
            .iter()
            .copied()
            .find(|&n| free(n, &taken))
            .or_else(|| (1..100u8).find(|&n| free(n, &taken)))
            .or_else(|| (!nfl).then(|| options.iter().copied().find(|&n| ours(n, &taken))).flatten())
            .or_else(|| (0..100u8).find(|&n| ours(n, &taken)));
        let n = n.unwrap_or(99);
        taken[side][n as usize] = true;
        out.push((id, n));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_follow_the_rules_and_never_repeat() {
        let mut players = Vec::new();
        for (k, pos) in Position::ALL.iter().enumerate() {
            for j in 0..6 {
                players.push((1000 + (k * 10 + j) as u32, *pos));
            }
        }
        let got = assign(&players, true, 2026);
        let mut seen = std::collections::HashSet::new();
        for (&(id, pos), &(id2, n)) in players.iter().zip(&got) {
            assert_eq!(id, id2);
            assert!(seen.insert(n), "{n} twice");
            if matches!(pos, Position::OT | Position::IOL) {
                assert!((50..=79).contains(&n), "lineman wears {n}");
            }
            if pos == Position::QB {
                assert!(n <= 19, "quarterback wears {n}");
            }
        }
        assert_eq!(got, assign(&players, true, 2026), "the same numbers every time");
    }

    #[test]
    fn a_man_who_crosses_the_ball_wears_a_number_of_his_own() {
        // A college game's dressed men, with a corner who shares his number
        // with a receiver (a roster bigger than its numbers doubles up).
        let mut players = Vec::new();
        for (k, pos) in Position::ALL.iter().enumerate() {
            for j in 0..5 {
                players.push((3000 + (k * 10 + j) as u32, *pos));
            }
        }
        let mut out: HashMap<u32, u8> = assign(&players, false, 2026).into_iter().collect();
        let first = |pos: Position| players.iter().find(|x| x.1 == pos).unwrap().0;
        let (corner, receiver) = (first(Position::CB), first(Position::WR));
        out.insert(corner, out[&receiver]);
        let shares =
            |out: &HashMap<u32, u8>, id: u32| players.iter().filter(|(o, _)| *o != id && out[o] == out[&id]).count();
        assert_eq!(shares(&out, corner), 1);
        // He plays receiver in this game: he gets a number nobody wears.
        let before = out.clone();
        let crossed: HashSet<u32> = [corner].into_iter().collect();
        own_numbers(&mut out, &players, false, 2026, [[false; 100]; 2], &crossed);
        assert_eq!(shares(&out, corner), 0, "the corner at receiver still shares #{}", out[&corner]);
        assert!(players.iter().all(|(id, _)| *id == corner || out[id] == before[id]), "only he changes");
        // A man who crossed but shares with nobody keeps his number.
        let mut again = before.clone();
        let unworn = (1..100u8).find(|n| !before.values().any(|m| m == n)).unwrap();
        again.insert(corner, unworn);
        let kept = again.clone();
        own_numbers(&mut again, &players, false, 2026, [[false; 100]; 2], &crossed);
        assert_eq!(again, kept);
    }

    #[test]
    fn a_big_college_roster_doubles_up_across_the_ball() {
        let mut players = Vec::new();
        for (k, pos) in Position::ALL.iter().enumerate() {
            for j in 0..9 {
                players.push((2000 + (k * 10 + j) as u32, *pos));
            }
        }
        assert!(players.len() > 100);
        let got = assign(&players, false, 2026);
        let mut seen = std::collections::HashSet::new();
        for (&(_, pos), &(_, n)) in players.iter().zip(&got) {
            assert!(seen.insert((defense(pos), n)), "{n} twice on one side of the ball");
        }
    }
}
