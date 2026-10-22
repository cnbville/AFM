//! Jersey numbers for the game views: every player on a team gets one,
//! inside his level's real numbering rules, never two alike on a team, and
//! the same one every time you watch (his id picks it).
//!
//! The rules: the NFL since 2021 (quarterbacks, kickers and punters 1-19;
//! backs and receivers 1-49 and 80-89; the line 50-79; linemen on defense
//! 50-79 and 90-99; linebackers 1-59 and 90-99; defensive backs 1-49), and
//! 0 from 2023. Before 2021 the NFL's tighter 1973 rules. College and high
//! school (NCAA and NFHS rules): any number 0-99, but the five interior
//! linemen of the offense must wear 50-79, so linemen do.

use afm_engine::positions::Position;

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

/// Numbers for one team's players (id and position), stars first so the
/// best players keep the numbers they would pick.
pub fn assign(players: &[(u32, Position)], nfl: bool, year: i32) -> Vec<(u32, u8)> {
    let mut taken = [false; 100];
    let mut out = Vec::with_capacity(players.len());
    for &(id, pos) in players {
        let rs = ranges(pos, nfl, year);
        let zero_ok = !nfl || year >= 2023;
        // His favourite range first (by the real shares), then the others.
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
        let n = options.iter().copied().find(|&n| !taken[n as usize]).or_else(|| (1..100u8).find(|&n| !taken[n as usize]));
        let n = n.unwrap_or(99);
        taken[n as usize] = true;
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
}
