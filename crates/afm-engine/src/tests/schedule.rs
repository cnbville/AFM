//! The schedule page's seasons ahead (update 2, lane 6): the NFL rotation it
//! shows is the one the schedule formula draws, and a high school district
//! holds for its two-year cycle with the sites swapped.

use crate::world::hs_year::{cycle_start, state_schedule};
use crate::world::nfl_year::{NflSeason, opponent_formula};
use crate::world::season_page::nfl_rotation;

#[test]
fn the_nfl_rotation_matches_the_schedule_formula() {
    let s = NflSeason {
        div: (0..32).map(|t| t / 4).collect(),
        prev_place: (0..32).map(|t| t % 4).collect(),
        ..Default::default()
    };
    for year in 2024..2036 {
        let games = opponent_formula(&s, year, 17);
        for t in 0..32 {
            let d = t / 4;
            let r = nfl_rotation(year, d);
            let mut divs: Vec<usize> = games
                .iter()
                .filter_map(|&(h, a)| {
                    if h == t {
                        Some(a / 4)
                    } else if a == t {
                        Some(h / 4)
                    } else {
                        None
                    }
                })
                .collect();
            divs.sort();
            let mut want = vec![d; 6];
            want.extend([r.conference; 4]);
            want.extend([r.cross; 4]);
            want.extend(r.same_place);
            want.push(r.seventeenth);
            want.sort();
            assert_eq!(divs, want, "{year}: team {t} of division {d}");
            // The 17th game's site: the AFC hosts in odd years.
            let home = games.iter().any(|&(h, a)| h == t && a / 4 == r.seventeenth);
            assert_eq!(home, r.seventeenth_home, "{year}: team {t}'s 17th game site");
        }
    }
    // The cycles: the same conference division every three years, the
    // other conference's every four.
    assert_eq!(nfl_rotation(2026, 0).conference, nfl_rotation(2029, 0).conference);
    assert_ne!(nfl_rotation(2026, 0).conference, nfl_rotation(2027, 0).conference);
    assert_eq!(nfl_rotation(2026, 5).cross, nfl_rotation(2030, 5).cross);
    assert_ne!(nfl_rotation(2026, 5).cross, nfl_rotation(2027, 5).cross);
}

#[test]
fn a_district_holds_for_two_years_and_swaps_its_sites() {
    assert_eq!(cycle_start(2026), 2026);
    assert_eq!(cycle_start(2027), 2026);
    assert_eq!(cycle_start(2028), 2028);
    let ids: Vec<usize> = (0..60).collect();
    let classes: Vec<(u32, &[usize])> = vec![(0, &ids[..30]), (1, &ids[30..])];
    let area = |id: usize| id / 15;
    let power = |id: usize| (0.2 + (id % 7) as f64 * 0.05, false);
    let (first, d1) = state_schedule(&classes, &area, &power, 10, 3, (99, false));
    let (second, d2) = state_schedule(&classes, &area, &power, 10, 4, (99, true));
    assert_eq!(d1, d2, "the districts hold for the cycle");
    let district_of = |x: usize| d1.iter().position(|d| d.contains(&x));
    let mut n = 0;
    for (wk, games) in first.iter().enumerate() {
        for &(h, a) in games {
            if district_of(h) == district_of(a) {
                assert!(second[wk].contains(&(a, h)), "week {wk}: {h} hosted {a}, so {a} hosts next year");
                n += 1;
            }
        }
    }
    assert!(n > 40, "district games checked: {n}");
    // A new cycle realigns.
    let (_, d3) = state_schedule(&classes, &area, &power, 10, 5, (100, false));
    assert_ne!(d1, d3, "a new cycle draws new districts");
}
