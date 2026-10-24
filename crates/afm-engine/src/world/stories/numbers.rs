//! Names and numbers the stories need: mascots and team colors
//! (`data/real-world/mascots.csv`), high school mascots, state names, and
//! jersey numbers by each level's and era's rules.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::positions::Position;

use super::TeamKey;
use crate::world::World;

const MASCOTS_CSV: &str = include_str!("../../../../../data/real-world/mascots.csv");

/// Team name to (mascot, color word).
static MASCOTS: LazyLock<HashMap<String, (String, String)>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    for line in MASCOTS_CSV.lines().skip(1) {
        let f = csv_fields(line);
        if f.len() >= 3 && !f[0].is_empty() {
            m.insert(f[0].clone(), (f[1].clone(), f[2].clone()));
        }
    }
    m
});

/// A CSV line's fields, quotes handled.
pub(crate) fn csv_fields(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut cur).trim().to_string()),
            _ => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

pub fn mascot_of(team: &str) -> Option<String> {
    MASCOTS.get(team).map(|m| m.0.clone()).filter(|m| !m.is_empty())
}

pub fn color_of(team: &str) -> Option<String> {
    MASCOTS.get(team).map(|m| m.1.clone()).filter(|m| !m.is_empty())
}

/// The mascots American high schools use most (MaxPreps' counts put Eagles,
/// Tigers, Panthers, Bulldogs, Wildcats and Lions on top; Claude's list).
const HS_MASCOTS: &[&str] = &[
    "Eagles",
    "Tigers",
    "Panthers",
    "Bulldogs",
    "Wildcats",
    "Lions",
    "Warriors",
    "Knights",
    "Falcons",
    "Cougars",
    "Mustangs",
    "Hornets",
    "Trojans",
    "Rams",
    "Bears",
    "Vikings",
    "Spartans",
    "Raiders",
    "Patriots",
    "Chargers",
    "Hawks",
    "Bobcats",
    "Pirates",
    "Indians",
    "Cardinals",
    "Wolverines",
    "Titans",
    "Blue Devils",
    "Rebels",
    "Jaguars",
    "Owls",
    "Golden Eagles",
    "Rockets",
    "Generals",
    "Huskies",
    "Comets",
    "Broncos",
    "Lancers",
    "Bearcats",
    "Saints",
    "Cavaliers",
    "Red Devils",
    "Wolves",
    "Royals",
    "Yellow Jackets",
    "Buccaneers",
    "Mavericks",
    "Gators",
    "Pioneers",
    "Hurricanes",
    "Bluejays",
    "Rangers",
    "Colts",
    "Crusaders",
    "Statesmen",
    "Greyhounds",
    "Bengals",
    "Longhorns",
    "Thunderbirds",
    "Railroaders",
    "Miners",
    "Blackhawks",
    "Wildcats",
    "Red Raiders",
    "Shamrocks",
    "Golden Bears",
];

/// The mascot of high school `h` (generated schools get one of the common
/// ones, fixed by the school).
pub fn hs_mascot(h: u32) -> String {
    let k = (h.wrapping_mul(2_654_435_761) >> 7) as usize % HS_MASCOTS.len();
    HS_MASCOTS[k].to_string()
}

pub fn state_name(code: &str) -> String {
    const NAMES: &[(&str, &str)] = &[
        ("AL", "Alabama"),
        ("AK", "Alaska"),
        ("AZ", "Arizona"),
        ("AR", "Arkansas"),
        ("CA", "California"),
        ("CO", "Colorado"),
        ("CT", "Connecticut"),
        ("DE", "Delaware"),
        ("DC", "D.C."),
        ("FL", "Florida"),
        ("GA", "Georgia"),
        ("HI", "Hawaii"),
        ("ID", "Idaho"),
        ("IL", "Illinois"),
        ("IN", "Indiana"),
        ("IA", "Iowa"),
        ("KS", "Kansas"),
        ("KY", "Kentucky"),
        ("LA", "Louisiana"),
        ("ME", "Maine"),
        ("MD", "Maryland"),
        ("MA", "Massachusetts"),
        ("MI", "Michigan"),
        ("MN", "Minnesota"),
        ("MS", "Mississippi"),
        ("MO", "Missouri"),
        ("MT", "Montana"),
        ("NE", "Nebraska"),
        ("NV", "Nevada"),
        ("NH", "New Hampshire"),
        ("NJ", "New Jersey"),
        ("NM", "New Mexico"),
        ("NY", "New York"),
        ("NC", "North Carolina"),
        ("ND", "North Dakota"),
        ("OH", "Ohio"),
        ("OK", "Oklahoma"),
        ("OR", "Oregon"),
        ("PA", "Pennsylvania"),
        ("RI", "Rhode Island"),
        ("SC", "South Carolina"),
        ("SD", "South Dakota"),
        ("TN", "Tennessee"),
        ("TX", "Texas"),
        ("UT", "Utah"),
        ("VT", "Vermont"),
        ("VA", "Virginia"),
        ("WA", "Washington"),
        ("WV", "West Virginia"),
        ("WI", "Wisconsin"),
        ("WY", "Wyoming"),
    ];
    NAMES.iter().find(|x| x.0 == code).map_or_else(|| code.to_string(), |x| x.1.to_string())
}

/// The numbers a position may wear, by level and season, most worn first.
/// NFL (rule 5, from memory, to verify): before 2004 receivers wore 80-89;
/// 2004 opened 10-19 to receivers; 2015 opened 40-49 to linebackers; 2021
/// opened 1-49 to backs, receivers, tight ends, linebackers and defensive
/// backs; 2024 allowed 0. College and high school: interior linemen must
/// wear 50-79 to be ineligible receivers, the rest follows habit; college
/// allowed 0 from 2020.
pub fn allowed(pos: Position, level: crate::calendar::Level, year: i32) -> Vec<u8> {
    use crate::calendar::Level as L;
    let r = |a: u8, b: u8| (a..=b).collect::<Vec<u8>>();
    let cat = |parts: &[Vec<u8>]| parts.concat();
    let zero = match level {
        L::Nfl => year >= 2024,
        L::College => year >= 2020,
        _ => true,
    };
    let z = |v: Vec<u8>| if zero { [vec![0], v].concat() } else { v };
    match level {
        L::Nfl => {
            let open = year >= 2021;
            match pos {
                Position::QB | Position::K | Position::P => z(r(1, 19)),
                Position::LS => cat(&[r(40, 59), r(90, 99)]),
                Position::RB | Position::S | Position::CB => {
                    if open {
                        z(cat(&[r(20, 49), r(1, 19)]))
                    } else {
                        r(20, 49)
                    }
                }
                Position::WR => {
                    if open {
                        z(cat(&[r(10, 19), r(80, 89), r(1, 9), r(20, 49)]))
                    } else if year >= 2004 {
                        cat(&[r(80, 89), r(10, 19)])
                    } else {
                        r(80, 89)
                    }
                }
                Position::TE => {
                    if open {
                        cat(&[r(80, 89), r(40, 49), r(1, 39)])
                    } else {
                        cat(&[r(80, 89), r(40, 49)])
                    }
                }
                Position::OT | Position::IOL => cat(&[r(60, 79), r(50, 59)]),
                Position::DT => cat(&[r(90, 99), r(60, 79), r(50, 59)]),
                Position::EDGE => {
                    if open {
                        cat(&[r(90, 99), r(50, 59), r(1, 49)])
                    } else {
                        cat(&[r(90, 99), r(50, 59)])
                    }
                }
                Position::LB => {
                    if open {
                        z(cat(&[r(50, 59), r(40, 49), r(1, 39), r(90, 99)]))
                    } else if year >= 2015 {
                        cat(&[r(50, 59), r(40, 49), r(90, 99)])
                    } else {
                        cat(&[r(50, 59), r(90, 99)])
                    }
                }
            }
        }
        _ => match pos {
            Position::QB => z(r(1, 19)),
            Position::K | Position::P => cat(&[r(1, 49), r(90, 99)]),
            Position::LS => cat(&[r(40, 69), r(90, 99)]),
            Position::RB => z(cat(&[r(1, 9), r(20, 39), r(10, 19)])),
            Position::WR => z(cat(&[r(1, 19), r(80, 89)])),
            Position::TE => cat(&[r(80, 89), r(1, 49)]),
            Position::OT | Position::IOL => r(50, 79),
            Position::DT => cat(&[r(90, 99), r(50, 69)]),
            Position::EDGE => cat(&[r(90, 99), r(1, 59)]),
            Position::LB => z(cat(&[r(1, 59), r(90, 99)])),
            Position::CB | Position::S => z(r(1, 49)),
        },
    }
}

/// Everyone on a team now, as (person index, player id), oldest first.
pub(crate) fn roster(w: &World, t: TeamKey) -> Vec<(usize, u32)> {
    let idx: Vec<usize> = match t {
        TeamKey::College(p) => w.programs.get(p as usize).map_or_else(Vec::new, |x| x.roster.clone()),
        TeamKey::Hs(h) => w.high_schools.get(h as usize).map_or_else(Vec::new, |x| x.roster.clone()),
        TeamKey::Nfl(n) => w.clubs.get(n as usize).map_or_else(Vec::new, |c| c.roster.clone()),
        TeamKey::Other(f) => {
            w.others.teams.get(f as usize).map_or_else(Vec::new, |t| t.roster.iter().map(|&i| i as usize).collect())
        }
    };
    // A player who walked away stays on the list until the roster is
    // rebuilt; he is not on the team.
    let mut v: Vec<(usize, u32, u32)> = idx
        .into_iter()
        .filter_map(|i| {
            w.people.get(i).filter(|p| p.stage != crate::world::Stage::Done).map(|p| (i, p.player.id, p.player.age))
        })
        .collect();
    v.sort_by_key(|x| (std::cmp::Reverse(x.2), x.1));
    v.into_iter().map(|x| (x.0, x.1)).collect()
}

fn mix(a: u32, b: u32) -> u32 {
    let mut h = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^ (h >> 12)
}

/// Each player's jersey number on a team: veterans choose first, each
/// starting from a number of his own (a big ego reaches for a single digit)
/// and taking the next free one his position allows. Retired numbers are
/// out of circulation. A college or high school roster bigger than its
/// numbers doubles up the way real ones do: an offensive and a defensive
/// player share a number, never two on the field at once.
pub fn numbers(w: &World, t: TeamKey) -> HashMap<u32, u8> {
    let level = t.level();
    let year = w.year;
    let retired: Vec<u8> = w.stories.traditions.retired_numbers(t);
    let mut taken = [false; 100];
    let mut side_taken = [[false; 100]; 2];
    for &n in &retired {
        taken[n as usize] = true;
        side_taken[0][n as usize] = true;
        side_taken[1][n as usize] = true;
    }
    let mut out = HashMap::new();
    for (i, id) in roster(w, t) {
        let p = &w.people[i].player;
        let list = allowed(p.position, level, year);
        if list.is_empty() {
            continue;
        }
        let ego = p.personality.traits.ego;
        let h = mix(id, 0x4A45);
        // A showman wants a low number where his position allows it.
        let start = if ego >= 15 && list.iter().any(|&n| n <= 9) {
            list.iter().position(|&n| n <= 9).unwrap_or(0) + (h as usize % 3)
        } else {
            h as usize % list.len()
        };
        let side =
            matches!(p.position, Position::EDGE | Position::DT | Position::LB | Position::CB | Position::S) as usize;
        let order = || (0..list.len()).map(|k| list[(start + k) % list.len()]);
        let pick = order().find(|&n| !taken[n as usize]).or_else(|| {
            (level != crate::calendar::Level::Nfl).then(|| order().find(|&n| !side_taken[side][n as usize])).flatten()
        });
        if let Some(n) = pick {
            taken[n as usize] = true;
            side_taken[side][n as usize] = true;
            out.insert(id, n);
        }
    }
    out
}

/// One player's number on his team now.
pub fn number_of(w: &World, t: TeamKey, id: u32) -> Option<u8> {
    numbers(w, t).get(&id).copied()
}

/// A player's team now, as a key.
pub fn team_of(w: &World, i: usize) -> Option<TeamKey> {
    let p = w.people.get(i)?;
    match p.stage {
        crate::world::Stage::Pro => p.pro.as_ref()?.team.map(|t| TeamKey::Nfl(t as u16)),
        crate::world::Stage::College => p.college.map(|c| TeamKey::College(c as u32)),
        crate::world::Stage::HighSchool => Some(TeamKey::Hs(p.hs as u32)),
        _ => None,
    }
}
