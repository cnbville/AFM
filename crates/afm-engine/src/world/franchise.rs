//! The NFL's real franchises by year (part B 39, section 5 topic 1): 28
//! teams to 1994, 30 from 1995 (Carolina and Jacksonville), the Browns'
//! move to Baltimore as the Ravens in 1996 with Cleveland dark until its
//! return in 1999 (31), the Texans in 2002 (32) and the realignment into
//! eight divisions that year. Every name, home city, division and roof on
//! its real dates. A franchise keeps its index in `World::nfl` (today's
//! alphabetical list) for its whole life; one that doesn't play a year has
//! no era that year.

/// Divisions: conference times four plus East 0, North or Central 1,
/// South 2, West 3. Before 2002 each conference had East, Central and West.
pub const AFC_EAST: u8 = 0;
pub const AFC_CENTRAL: u8 = 1;
pub const AFC_SOUTH: u8 = 2;
pub const AFC_WEST: u8 = 3;
pub const NFC_EAST: u8 = 4;
pub const NFC_CENTRAL: u8 = 5;
pub const NFC_SOUTH: u8 = 6;
pub const NFC_WEST: u8 = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Roof {
    Open,
    Dome,
    Retractable,
}

/// One stretch of a franchise's life with the same name, home, division
/// and stadium roof.
#[derive(Clone, Copy, Debug)]
pub struct Era {
    pub from: i32,
    pub to: i32,
    pub name: &'static str,
    pub city: &'static str,
    pub state: &'static str,
    pub div: u8,
    pub roof: Roof,
}

const fn e(
    from: i32,
    to: i32,
    name: &'static str,
    city: &'static str,
    state: &'static str,
    div: u8,
    roof: Roof,
) -> Era {
    Era { from, to, name, city, state, div, roof }
}

const FAR: i32 = 9999;
const OLD: i32 = 1900;
use Roof::*;

/// Each franchise's eras, in `league::NFL_TEAM_NAMES` order.
pub static ERAS: [&[Era]; 32] = [
    &[
        e(OLD, 1987, "St. Louis Cardinals", "St. Louis", "MO", NFC_EAST, Open),
        e(1988, 1993, "Phoenix Cardinals", "Tempe", "AZ", NFC_EAST, Open),
        e(1994, 2001, "Arizona Cardinals", "Tempe", "AZ", NFC_EAST, Open),
        e(2002, 2005, "Arizona Cardinals", "Tempe", "AZ", NFC_WEST, Open),
        e(2006, FAR, "Arizona Cardinals", "Glendale", "AZ", NFC_WEST, Retractable),
    ],
    &[
        e(OLD, 1991, "Atlanta Falcons", "Atlanta", "GA", NFC_WEST, Open),
        e(1992, 2001, "Atlanta Falcons", "Atlanta", "GA", NFC_WEST, Dome),
        e(2002, 2016, "Atlanta Falcons", "Atlanta", "GA", NFC_SOUTH, Dome),
        e(2017, FAR, "Atlanta Falcons", "Atlanta", "GA", NFC_SOUTH, Retractable),
    ],
    &[
        e(1996, 2001, "Baltimore Ravens", "Baltimore", "MD", AFC_CENTRAL, Open),
        e(2002, FAR, "Baltimore Ravens", "Baltimore", "MD", AFC_CENTRAL, Open),
    ],
    &[e(OLD, FAR, "Buffalo Bills", "Orchard Park", "NY", AFC_EAST, Open)],
    &[
        e(1995, 1995, "Carolina Panthers", "Clemson", "SC", NFC_WEST, Open),
        e(1996, 2001, "Carolina Panthers", "Charlotte", "NC", NFC_WEST, Open),
        e(2002, FAR, "Carolina Panthers", "Charlotte", "NC", NFC_SOUTH, Open),
    ],
    &[e(OLD, FAR, "Chicago Bears", "Chicago", "IL", NFC_CENTRAL, Open)],
    &[e(OLD, FAR, "Cincinnati Bengals", "Cincinnati", "OH", AFC_CENTRAL, Open)],
    &[
        e(OLD, 1995, "Cleveland Browns", "Cleveland", "OH", AFC_CENTRAL, Open),
        e(1999, FAR, "Cleveland Browns", "Cleveland", "OH", AFC_CENTRAL, Open),
    ],
    &[
        e(OLD, 2008, "Dallas Cowboys", "Irving", "TX", NFC_EAST, Open),
        e(2009, FAR, "Dallas Cowboys", "Arlington", "TX", NFC_EAST, Retractable),
    ],
    &[e(OLD, FAR, "Denver Broncos", "Denver", "CO", AFC_WEST, Open)],
    &[
        e(OLD, 2001, "Detroit Lions", "Pontiac", "MI", NFC_CENTRAL, Dome),
        e(2002, FAR, "Detroit Lions", "Detroit", "MI", NFC_CENTRAL, Dome),
    ],
    &[e(OLD, FAR, "Green Bay Packers", "Green Bay", "WI", NFC_CENTRAL, Open)],
    &[e(2002, FAR, "Houston Texans", "Houston", "TX", AFC_SOUTH, Retractable)],
    &[
        e(OLD, 1983, "Baltimore Colts", "Baltimore", "MD", AFC_EAST, Open),
        e(1984, 2001, "Indianapolis Colts", "Indianapolis", "IN", AFC_EAST, Dome),
        e(2002, 2007, "Indianapolis Colts", "Indianapolis", "IN", AFC_SOUTH, Dome),
        e(2008, FAR, "Indianapolis Colts", "Indianapolis", "IN", AFC_SOUTH, Retractable),
    ],
    &[
        e(1995, 2001, "Jacksonville Jaguars", "Jacksonville", "FL", AFC_CENTRAL, Open),
        e(2002, FAR, "Jacksonville Jaguars", "Jacksonville", "FL", AFC_SOUTH, Open),
    ],
    &[e(OLD, FAR, "Kansas City Chiefs", "Kansas City", "MO", AFC_WEST, Open)],
    &[
        e(OLD, 1994, "Los Angeles Raiders", "Los Angeles", "CA", AFC_WEST, Open),
        e(1995, 2019, "Oakland Raiders", "Oakland", "CA", AFC_WEST, Open),
        e(2020, FAR, "Las Vegas Raiders", "Paradise", "NV", AFC_WEST, Dome),
    ],
    &[
        e(OLD, 2016, "San Diego Chargers", "San Diego", "CA", AFC_WEST, Open),
        e(2017, 2019, "Los Angeles Chargers", "Carson", "CA", AFC_WEST, Open),
        e(2020, FAR, "Los Angeles Chargers", "Inglewood", "CA", AFC_WEST, Dome),
    ],
    &[
        e(OLD, 1994, "Los Angeles Rams", "Anaheim", "CA", NFC_WEST, Open),
        e(1995, 2015, "St. Louis Rams", "St. Louis", "MO", NFC_WEST, Dome),
        e(2016, 2019, "Los Angeles Rams", "Los Angeles", "CA", NFC_WEST, Open),
        e(2020, FAR, "Los Angeles Rams", "Inglewood", "CA", NFC_WEST, Dome),
    ],
    &[e(OLD, FAR, "Miami Dolphins", "Miami Gardens", "FL", AFC_EAST, Open)],
    &[
        e(OLD, 2013, "Minnesota Vikings", "Minneapolis", "MN", NFC_CENTRAL, Dome),
        e(2014, 2015, "Minnesota Vikings", "Minneapolis", "MN", NFC_CENTRAL, Open),
        e(2016, FAR, "Minnesota Vikings", "Minneapolis", "MN", NFC_CENTRAL, Dome),
    ],
    &[e(OLD, FAR, "New England Patriots", "Foxborough", "MA", AFC_EAST, Open)],
    &[
        e(OLD, 2001, "New Orleans Saints", "New Orleans", "LA", NFC_WEST, Dome),
        e(2002, FAR, "New Orleans Saints", "New Orleans", "LA", NFC_SOUTH, Dome),
    ],
    &[e(OLD, FAR, "New York Giants", "East Rutherford", "NJ", NFC_EAST, Open)],
    &[e(OLD, FAR, "New York Jets", "East Rutherford", "NJ", AFC_EAST, Open)],
    &[e(OLD, FAR, "Philadelphia Eagles", "Philadelphia", "PA", NFC_EAST, Open)],
    &[e(OLD, FAR, "Pittsburgh Steelers", "Pittsburgh", "PA", AFC_CENTRAL, Open)],
    &[
        e(OLD, 2013, "San Francisco 49ers", "San Francisco", "CA", NFC_WEST, Open),
        e(2014, FAR, "San Francisco 49ers", "Santa Clara", "CA", NFC_WEST, Open),
    ],
    &[
        e(OLD, 1999, "Seattle Seahawks", "Seattle", "WA", AFC_WEST, Dome),
        e(2000, 2001, "Seattle Seahawks", "Seattle", "WA", AFC_WEST, Open),
        e(2002, FAR, "Seattle Seahawks", "Seattle", "WA", NFC_WEST, Open),
    ],
    &[
        e(OLD, 2001, "Tampa Bay Buccaneers", "Tampa", "FL", NFC_CENTRAL, Open),
        e(2002, FAR, "Tampa Bay Buccaneers", "Tampa", "FL", NFC_SOUTH, Open),
    ],
    &[
        e(OLD, 1996, "Houston Oilers", "Houston", "TX", AFC_CENTRAL, Dome),
        e(1997, 1997, "Tennessee Oilers", "Memphis", "TN", AFC_CENTRAL, Open),
        e(1998, 1998, "Tennessee Oilers", "Nashville", "TN", AFC_CENTRAL, Open),
        e(1999, 2001, "Tennessee Titans", "Nashville", "TN", AFC_CENTRAL, Open),
        e(2002, FAR, "Tennessee Titans", "Nashville", "TN", AFC_SOUTH, Open),
    ],
    &[
        e(OLD, 1996, "Washington Redskins", "Washington", "DC", NFC_EAST, Open),
        e(1997, 2019, "Washington Redskins", "Landover", "MD", NFC_EAST, Open),
        e(2020, 2021, "Washington Football Team", "Landover", "MD", NFC_EAST, Open),
        e(2022, FAR, "Washington Commanders", "Landover", "MD", NFC_EAST, Open),
    ],
];

/// The franchise's era in a season, None when it didn't play that year.
pub fn era(team: usize, year: i32) -> Option<&'static Era> {
    ERAS.get(team)?.iter().find(|e| e.from <= year && year <= e.to)
}

/// Whether the franchise plays this season.
pub fn active(team: usize, year: i32) -> bool {
    era(team, year).is_some()
}

/// Its name that season (today's name when it didn't play).
pub fn name(team: usize, year: i32) -> &'static str {
    era(team, year).map_or(crate::league::NFL_TEAM_NAMES[team], |e| e.name)
}

/// Its first season in the league since `year` began counting (an
/// expansion team or the returning Browns): the season it joined.
pub fn joined(team: usize, year: i32) -> bool {
    active(team, year) && !active(team, year - 1)
}

/// The eight-division alignment starts in 2002.
pub fn eight_divisions(year: i32) -> bool {
    year >= 2002
}

/// The NFL's expansion drafts (real): the year, the new teams and how many
/// players each took from the other rosters' unprotected lists.
pub fn expansion_draft(year: i32) -> &'static [(usize, usize)] {
    match year {
        1995 => &[(4, 35), (14, 31)],
        1999 => &[(7, 37)],
        2002 => &[(12, 19)],
        _ => &[],
    }
}

/// The last season whose franchises follow real history. Later names and
/// homes change only through the world's own owners' votes.
pub const PRESENT: i32 = 2026;

/// Conference and division of a franchise in a season, None when it
/// doesn't play. After `PRESENT` the 2026 alignment holds.
pub fn alignment(team: usize, year: i32) -> Option<(usize, usize)> {
    era(team, year.min(PRESENT)).map(|e| (e.div as usize / 4, e.div as usize))
}

/// Rounds in the draft: 12 through 1992, 8 in 1993, 7 from 1994 (real).
pub fn draft_rounds(year: i32) -> u32 {
    match year {
        ..=1992 => 12,
        1993 => 8,
        _ => 7,
    }
}

/// Compensatory picks at the end of rounds 3-7, from 1994: the world's
/// 33 a year in a 32-team league, spread as the real 2021-25 drafts spread
/// them (round 3 six, round 4 four, round 5 six, round 6 eight, round 7
/// nine), scaled by the league's size.
fn comp_picks(year: i32, round: u32, teams: usize) -> usize {
    if year < 1994 {
        return 0;
    }
    let per = [0.0, 0.0, 6.0, 4.0, 6.0, 8.0, 9.0];
    let k = per.get(round as usize - 1).copied().unwrap_or(0.0);
    (k * teams as f64 / 32.0).round() as usize
}

/// One round's order. Expansion teams have their own places (real): in
/// 1995 the Panthers and Jaguars picked first and last in every round,
/// taking turns at the front (the Panthers won the toss for the first
/// pick); in 1999 the Browns and in 2002 the Texans picked first in every
/// round.
pub fn round_order(year: i32, round: u32, base: &[usize]) -> Vec<usize> {
    let (front, back): (Vec<usize>, Vec<usize>) = match year {
        1995 if round % 2 == 1 => (vec![4, 14], vec![14, 4]),
        1995 => (vec![14, 4], vec![4, 14]),
        1999 => (vec![7], vec![]),
        2002 => (vec![12], vec![]),
        _ => (vec![], vec![]),
    };
    let mid = base.iter().copied().filter(|t| !front.contains(t));
    front.iter().copied().chain(mid).chain(back).collect()
}

/// The whole draft, pick by pick: the team and the round. `base` is last
/// season's order of this year's teams. Compensatory picks go to keyed
/// teams until free agency awards them (proposed, not confirmed). A world
/// smaller than full size keeps every k-th pick, so its rounds keep their
/// real shares.
pub fn draft_board(year: i32, base: &[usize], scale: f64, seed: u64) -> Vec<(usize, u32)> {
    let teams = base.len().max(1);
    let mut full = Vec::new();
    for round in 1..=draft_rounds(year) {
        for t in round_order(year, round, base) {
            full.push((t, round));
        }
        for k in 0..comp_picks(year, round, teams) {
            let u = super::season::key_uniform(seed ^ year as u64 ^ 0xC0A1, (round as u64) << 8 ^ k as u64);
            full.push((base[(u * teams as f64) as usize % teams], round));
        }
    }
    if scale >= 1.0 {
        return full;
    }
    let n = ((full.len() as f64 * scale).round() as usize).max(1);
    (0..n).map(|i| full[((i as f64 / scale) as usize).min(full.len() - 1)]).collect()
}

/// The round a pick falls in, for a league of `teams` that year.
pub fn round_at(year: i32, pick: usize, teams: usize, scale: f64) -> u32 {
    let base: Vec<usize> = (0..teams).collect();
    let board = draft_board(year, &base, scale, 0);
    board.get(pick.max(1) - 1).map_or(draft_rounds(year) + 1, |x| x.1)
}

/// Franchises playing a season.
pub fn active_teams(year: i32) -> Vec<usize> {
    (0..32).filter(|&t| active(t, year)).collect()
}

const RAVENS: usize = 2;
const BROWNS: usize = 7;

/// A new football year (the day after the Super Bowl): names and homes by
/// era, the Browns' move to Baltimore (1996), a dark franchise's staff and
/// roster let go, the expansion drafts (1995, 1999, 2002), and last
/// season's draft order handed to this year's teams.
pub(crate) fn year_start(w: &mut super::World) {
    use crate::calendar::{Event, Kind, Level, Loudness};
    let year = w.year;
    let d = w.dates.start;
    let n = w.nfl.len().min(32);
    if year <= PRESENT {
        for t in 0..n {
            w.nfl[t].name = name(t, year);
        }
    }
    // 1996 (real): Art Modell takes the Browns' players, front office and
    // draft picks to Baltimore as the Ravens; Cleveland keeps the name,
    // colors and records and is promised a team by 1999.
    if year == 1996 && n == 32 {
        let depth = std::mem::take(&mut w.nfl[BROWNS].depth);
        w.nfl[RAVENS].depth = depth;
        super::pro::move_club(w, BROWNS, RAVENS, "the Browns moved to Baltimore as the Ravens");
        w.nfl[RAVENS].owner = w.nfl[BROWNS].owner;
        w.nfl[RAVENS].draft_skill = w.nfl[BROWNS].draft_skill;
        if let Some(s) = w.nfl_season.as_mut()
            && let Some(order) = s.last_order.as_mut()
        {
            for t in order.iter_mut().filter(|t| **t == BROWNS) {
                *t = RAVENS;
            }
        }
        let text = "The Browns' players, front office and draft picks move to Baltimore as the Ravens; \
                    Cleveland keeps the Browns' name, colors and records and is promised a team by 1999.";
        w.calendar.post(Event::new(d, Level::Nfl, Kind::News, Loudness::Huge, text).team(RAVENS).memorable());
    }
    // A franchise that doesn't play this season: no staff, no roster.
    for t in 0..n {
        if active(t, year) {
            continue;
        }
        let team = &w.nfl[t];
        for c in [team.head_coach, team.oc, team.dc].into_iter().flatten() {
            super::staff::leave(w, c, "franchise went dark");
        }
        w.nfl[t].depth = Default::default();
        for i in w.clubs.get(t).map(|c| c.roster.clone()).unwrap_or_default() {
            super::pro::release(w, i, d, "his franchise went dark");
        }
    }
    if let Some(s) = w.nfl_season.as_mut()
        && let Some(order) = s.last_order.as_mut()
    {
        order.retain(|&t| active(t, year));
    }
    expansion(w, year);
    super::pro::rebuild(w);
}

/// Empties the rosters of franchises that don't play this season (after
/// the background league's yearly refill).
pub(crate) fn clear_dark(w: &mut super::World) {
    let year = w.year;
    for t in 0..w.nfl.len().min(32) {
        if !active(t, year) {
            w.nfl[t].depth = Default::default();
        }
    }
}

/// The expansion draft (February, real): each old team leaves six players
/// open (1995; five in 1999 and 2002), the new teams take turns, and after
/// each pick the team that lost him pulls one more of its open players
/// (the 1995 rule; proposed, not confirmed, for 1999 and 2002). A team
/// leaves its veterans past 29 and its backups open, never its best 22
/// (proposed, not confirmed). The rest of the roster comes off the street.
fn expansion(w: &mut super::World, year: i32) {
    use crate::calendar::{Event, Kind, Level, Loudness};
    let list = expansion_draft(year);
    if list.is_empty() || w.nfl.len() < 32 {
        return;
    }
    let new: Vec<usize> = list.iter().map(|x| x.0).collect();
    let open_n = if year == 1995 { 6 } else { 5 };
    if w.clubs.len() < 32 {
        return;
    }
    // (team, person, the team's keep value)
    let mut pool: Vec<(usize, usize, f64)> = Vec::new();
    for t in (0..32).filter(|&t| active(t, year) && !new.contains(&t)) {
        let mut ranked: Vec<(usize, f64, u32)> = w.clubs[t]
            .roster
            .iter()
            .filter(|&&i| w.people[i].pro.as_ref().is_some_and(|c| c.status != super::pro::Status::PracticeSquad))
            .map(|&i| {
                let age = w.people[i].player.age;
                (i, super::pro::grade(w, t, i) - 1.5 * (age as f64 - 29.0).max(0.0), age)
            })
            .collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
        let mut tail: Vec<_> = ranked.into_iter().skip(22).collect();
        tail.sort_by(|a, b| b.2.cmp(&a.2).then(b.1.total_cmp(&a.1)));
        for &(i, keep, _) in tail.iter().take(open_n) {
            pool.push((t, i, keep));
        }
    }
    let mut picks: Vec<Vec<usize>> = vec![Vec::new(); new.len()];
    let mut turn = 0;
    while picks.iter().zip(list).any(|(p, x)| p.len() < x.1) && !pool.is_empty() {
        let k = turn % new.len();
        turn += 1;
        if picks[k].len() >= list[k].1 {
            continue;
        }
        // The new team's own view of each open player, a little for youth.
        let value = |e: &(usize, usize, f64)| {
            let age = w.people[e.1].player.age as f64;
            super::pro::grade(w, new[k], e.1) - 0.6 * (age - 28.0).max(0.0)
        };
        let Some(best) = (0..pool.len()).max_by(|&a, &b| value(&pool[a]).total_cmp(&value(&pool[b]))) else {
            break;
        };
        let (t, i, _) = pool.swap_remove(best);
        picks[k].push(i);
        // The team that lost him pulls its most valued open player.
        if let Some(pull) = (0..pool.len()).filter(|&e| pool[e].0 == t).max_by(|&a, &b| pool[a].2.total_cmp(&pool[b].2))
        {
            pool.swap_remove(pull);
        }
    }
    for (k, v) in picks.iter().enumerate() {
        for &i in v {
            super::pro::transfer(w, i, new[k], "taken in the expansion draft");
        }
    }
    let mut parts = Vec::new();
    for (k, &(t, _)) in list.iter().enumerate() {
        // A new owner and general manager (real: Richardson and Weaver, Polian,
        // Lerner and Clark, McNair and Casserly), with their own ratings.
        let mut rng = w.rng.fork();
        if t != RAVENS {
            w.nfl[t].owner = crate::league::new_staff_rating(&mut rng);
            w.nfl[t].draft_skill = crate::league::new_staff_rating(&mut rng);
        }
        w.nfl[t].win_share.clear();
        parts.push(format!("the {} take {}", w.nfl[t].name, picks[k].len()));
    }
    super::pro::rebuild(w);
    let text = format!(
        "Expansion draft: {} players from the other rosters; free agency, the draft and the street fill the rest.",
        parts.join(" and ")
    );
    let d = w.dates.start;
    w.calendar.post(Event::new(d, Level::Nfl, Kind::DraftChoice, Loudness::Big, text).team(new[0]));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drafts_have_their_real_rounds_and_size() {
        // Last season's teams that play this year.
        let base = |y: i32| -> Vec<usize> { active_teams(y - 1).into_iter().filter(|&t| active(t, y)).collect() };
        let size = |y: i32| draft_board(y, &base(y), 1.0, 1).len();
        assert_eq!(size(1990), 12 * 28);
        assert_eq!(size(1993), 8 * 28);
        assert_eq!(size(2024), 257);
        // 1995: 28 old teams and both new teams twice a round, plus 29 compensatory picks.
        assert_eq!(size(1995), 7 * 32 + 29);
        let b = draft_board(1999, &base(1999), 1.0, 1);
        assert_eq!(b[0].0, 7);
        assert_eq!(b.iter().find(|x| x.1 == 2).unwrap().0, 7);
        assert_eq!(round_at(2024, 32, 32, 1.0), 1);
        assert_eq!(round_at(2024, 33, 32, 1.0), 2);
        assert_eq!(round_at(2024, 257, 32, 1.0), 7);
    }

    #[test]
    fn the_league_grows_on_its_real_dates() {
        let count = |y: i32| (0..32).filter(|&t| active(t, y)).count();
        assert_eq!(count(1984), 28);
        assert_eq!(count(1994), 28);
        assert_eq!(count(1995), 30);
        assert_eq!(count(1996), 30);
        assert_eq!(count(1998), 30);
        assert_eq!(count(1999), 31);
        assert_eq!(count(2001), 31);
        assert_eq!(count(2002), 32);
        assert_eq!(count(2026), 32);
        assert_eq!(name(30, 1996), "Houston Oilers");
        assert_eq!(name(16, 2019), "Oakland Raiders");
        assert_eq!(name(31, 2021), "Washington Football Team");
        for y in [1990, 1999, 2002, 2026] {
            for d in 0..8u8 {
                let n = (0..32).filter(|&t| era(t, y).is_some_and(|e| e.div == d)).count();
                let empty_ok = !eight_divisions(y) && (d == AFC_SOUTH || d == NFC_SOUTH);
                assert!(empty_ok == (n == 0), "{y} division {d}: {n}");
            }
        }
        for t in 0..32 {
            assert_eq!(crate::league::NFL_TEAM_NAMES[t], name(t, 2026));
        }
    }
}
