//! Section 4, topic 2: the NFL year on its real dates. The offseason's dated
//! steps (the combine, the tag window, the new league year, the league
//! meeting, pro days, OTAs, camp, cutdowns), the regular season built by the
//! real opponent formula and played week by week, Black Monday, the playoffs
//! under each era's field and the Super Bowl. Around them: the training
//! blocks the CBA allows (topic 7), the contract and money dates other
//! sections will fill (A1-A8, A17), awards and Hall of Fame days (A15),
//! owner meetings (A44), betting releases (A45) and whisper windows (A46).
//!
//! NFL games are a stand-in until the league's rosters are full players
//! (section 5): each game is won from the two rosters' starters, the staffs
//! and home field, sized so a season's win shares spread like the real
//! league's (about 0.19, 0.12 of it luck).

use super::coach::TitleKind;
use super::news::{Channel, NewsItem, NewsKind};
use super::pro::games::{self as pro_games, Fixture, Side};
use super::season::{key_normal, key_uniform};
use super::{World, franchise, recruiting, staff};
use crate::calendar::{Date, Event, Kind, Level, Link, Loudness, Moment, Weekday, YearDates};

/// Win odds per standard deviation of roster strength (a logit), per point
/// of staff quality over 2.5, and home field (about 55% at home). Proposed,
/// not confirmed: they keep the old yearly stand-in's spread.
const PER_SD: f64 = 0.52;
const PER_STAFF: f64 = 0.24;
const HOME: f64 = 0.2;
/// About one game in 300 ends tied.
const TIE: f64 = 0.003;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct NflSeason {
    pub year: i32,
    /// Conference (0 AFC, 1 NFC) and division (0-7, AFC East first) per team.
    pub conf: Vec<usize>,
    pub div: Vec<usize>,
    /// Whether each franchise plays this season.
    pub active: Vec<bool>,
    /// Each week's games, (home, away).
    pub weeks: Vec<Vec<(usize, usize)>>,
    pub wins: Vec<u32>,
    pub losses: Vec<u32>,
    pub ties: Vec<u32>,
    pub div_wins: Vec<u32>,
    /// Strength in standard deviations and staff quality, read at kickoff.
    pub z: Vec<f64>,
    pub staff: Vec<f64>,
    pub expect: Vec<f64>,
    pub started: bool,
    pub booked: bool,
    /// Playoff seeds per conference, best first, still alive.
    pub alive: [Vec<usize>; 2],
    /// Seed of each playoff team.
    pub seed: Vec<Option<usize>>,
    /// The round a playoff team went out in (1 wild card .. 4 Super Bowl), or
    /// 5 for the champion.
    pub out_round: Vec<u32>,
    pub champion: Option<usize>,
    pub runner_up: Option<usize>,
    /// This April's draft order (from last season), and the order this season sets.
    pub last_order: Option<Vec<usize>>,
    pub final_order: Option<Vec<usize>>,
    /// Each team's division finish (0 first) last season, for the formula.
    pub prev_place: Vec<usize>,
    /// Dated per-team days: pro days per program (A12), ticket renewals (A17).
    pub pro_days: Vec<(usize, Date)>,
    pub ticket_days: Vec<(usize, Date)>,
    /// Each team's bye Sunday.
    pub byes: Vec<Option<Date>>,
    /// Games judged huge by the stakes test, per team.
    pub huge: Vec<u32>,
    /// Each game's place on the calendar, in the same order as `weeks`.
    pub slots: Vec<Vec<Slot>>,
    /// The playoff round's games still to play: (round, conference, higher
    /// seed, lower seed, kickoff), and the winners so far.
    pub round_games: Vec<(u32, usize, usize, usize, Moment)>,
    pub round_next: [Vec<usize>; 2],
    /// Flexes so far in weeks 5-10 (two at most a season from 2014).
    pub early_flexes: u32,
    /// Thursday night flexes this season (two at most, from 2023).
    #[serde(default)]
    pub tnf_flexes: u32,
}

/// One NFL team's season in the book.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NflTeamSeason {
    pub year: i32,
    pub team: usize,
    pub wins: u32,
    pub losses: u32,
    pub ties: u32,
    /// The head coach who finished the regular season.
    pub coach: Option<usize>,
    /// 0 out of the playoffs; the round it went out in, 1 wild card to 4
    /// the Super Bowl; 5 the champion.
    pub round: u32,
}

/// A game's place on the calendar (T1.2, T2): the kickoff in Eastern time,
/// its TV window, and a neutral site abroad.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Slot {
    pub at: Moment,
    pub window: Window,
    pub site: Option<String>,
    pub played: bool,
    /// Home and away points, for a game played on the engine (the stand-in
    /// draws only a winner).
    #[serde(default)]
    pub score: Option<[u32; 2]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum Window {
    /// Sunday at 1pm or in the late afternoon.
    Sunday,
    Opener,
    ThursdayNight,
    SundayNight,
    MondayNight,
    Saturday,
    Thanksgiving,
    BlackFriday,
    Christmas,
    International,
    /// Any other day the real schedule uses (a Wednesday opener, a Friday in Brazil).
    Other,
}

impl Window {
    pub fn name(self) -> &'static str {
        match self {
            Window::Sunday => "Sunday afternoon",
            Window::Opener => "the season opener",
            Window::ThursdayNight => "Thursday night",
            Window::SundayNight => "Sunday night",
            Window::MondayNight => "Monday night",
            Window::Saturday => "Saturday",
            Window::Thanksgiving => "Thanksgiving",
            Window::BlackFriday => "Black Friday",
            Window::Christmas => "Christmas",
            Window::International => "an international game",
            Window::Other => "a special date",
        }
    }

    /// Prime time: what national audiences and Pro Bowl voters see (A15).
    pub fn prime(self) -> bool {
        matches!(
            self,
            Window::Opener
                | Window::ThursdayNight
                | Window::SundayNight
                | Window::MondayNight
                | Window::Thanksgiving
                | Window::Christmas
        )
    }
}

/// No conference or division: a franchise that doesn't play this season.
const NONE: usize = 99;

/// Conference and division of each team that season, from the real
/// alignment of the year (see `franchise`).
fn alignment(w: &World) -> (Vec<usize>, Vec<usize>) {
    let n = w.nfl.len();
    let mut conf = vec![NONE; n];
    let mut div = vec![NONE; n];
    for t in 0..n {
        if let Some((c, d)) = franchise::alignment(t, w.year) {
            conf[t] = c;
            div[t] = d;
        }
    }
    (conf, div)
}

/// The team's row in the 2026 team table (market, home state), by franchise.
fn team_row(t: usize) -> Option<&'static super::data::NflTeamRow> {
    let name = crate::league::NFL_TEAM_NAMES.get(t)?;
    super::data::NFL_TEAMS.iter().find(|r| r.name == *name)
}

/// A new football year: last season's order and places carry over.
pub(crate) fn new_season(w: &World) -> NflSeason {
    let n = w.nfl.len();
    let (conf, div) = alignment(w);
    let (last_order, prev_place) = match w.nfl_season.as_ref() {
        Some(s) if s.booked => (s.final_order.clone(), places(s)),
        _ => (None, strength_places(w, &div)),
    };
    let year = w.year;
    let seed = w.options.seed as u64 ^ (year as u64) << 18 ^ 0x4E46;
    let y = &w.dates;
    // Pro days (A12): each FBS program and the FCS programs that produce
    // pros, on its own day in March and early April.
    let mut pro_days = Vec::new();
    let span = y.pro_days.0.days_to(y.pro_days.1).max(1);
    for p in w.programs.iter().filter(|p| p.division <= super::data::Division::FCS) {
        let k = (key_uniform(seed ^ 0x5044, p.id as u64) * (span + 1) as f64) as i32;
        let mut d = y.pro_days.0.add(k.min(span));
        if d.weekday() == Weekday::Sun {
            d = d.add(1);
        }
        pro_days.push((p.id, d));
    }
    let active: Vec<bool> = div.iter().map(|&d| d != NONE).collect();
    let ticket_days = (0..n)
        .filter(|&t| active[t])
        .map(|t| {
            let k = (key_uniform(seed ^ 0x544B, t as u64) * 60.0) as i32;
            (t, Date::ymd(year, 3, 1).add(k))
        })
        .collect();
    NflSeason {
        year,
        conf,
        div,
        active,
        wins: vec![0; n],
        losses: vec![0; n],
        ties: vec![0; n],
        div_wins: vec![0; n],
        seed: vec![None; n],
        out_round: vec![0; n],
        last_order,
        prev_place,
        pro_days,
        ticket_days,
        byes: vec![None; n],
        huge: vec![0; n],
        ..Default::default()
    }
}

/// Division finishes from a finished season.
fn places(s: &NflSeason) -> Vec<usize> {
    let n = s.wins.len();
    let mut place = vec![0; n];
    for d in 0..8 {
        let mut members: Vec<usize> = (0..n).filter(|&t| s.div[t] == d).collect();
        members.sort_by(|&a, &b| record_key(s, b).total_cmp(&record_key(s, a)).then(a.cmp(&b)));
        for (k, &t) in members.iter().enumerate() {
            place[t] = k;
        }
    }
    place
}

/// Division finishes by roster strength, before the first season.
fn strength_places(w: &World, div: &[usize]) -> Vec<usize> {
    let n = w.nfl.len();
    let mut place = vec![0; n];
    for d in 0..8 {
        let mut members: Vec<usize> = (0..n).filter(|&t| div[t] == d).collect();
        members.sort_by(|&a, &b| w.nfl[b].strength().total_cmp(&w.nfl[a].strength()).then(a.cmp(&b)));
        for (k, &t) in members.iter().enumerate() {
            place[t] = k;
        }
    }
    place
}

fn record_key(s: &NflSeason, t: usize) -> f64 {
    let g = (s.wins[t] + s.losses[t] + s.ties[t]).max(1) as f64;
    (s.wins[t] as f64 + 0.5 * s.ties[t] as f64) / g
        + 0.001 * s.div_wins[t] as f64
        + 0.0001 * s.z.get(t).copied().unwrap_or(0.0)
}

/// The season's games by the real opponent formula (2002 on): six division
/// games, four against a division of the same conference (a three-year
/// rotation), four against a division of the other conference (four
/// years), two against the same-place teams of the other two divisions in
/// the conference, and from 2021 a 17th game against a same-place team of
/// another division of the other conference. The 32-team alignment is used
/// for every year (proposed, not confirmed).
fn opponent_formula(s: &NflSeason, season: i32, games: u32) -> Vec<(usize, usize)> {
    let n = s.div.len();
    let members = |d: usize| -> Vec<usize> {
        let mut m: Vec<usize> = (0..n).filter(|&t| s.div[t] == d).collect();
        m.sort_by_key(|&t| (s.prev_place[t], t));
        m
    };
    let mut out = Vec::new();
    let par = season.rem_euclid(2) as usize;
    // Division: home and away.
    for d in 0..8 {
        let m = members(d);
        for i in 0..m.len() {
            for j in 0..m.len() {
                if i != j {
                    out.push((m[i], m[j]));
                }
            }
        }
    }
    // Four against four: home for half of them.
    let four = |a: usize, b: usize, out: &mut Vec<(usize, usize)>| {
        let (ma, mb) = (members(a), members(b));
        for (i, &x) in ma.iter().enumerate() {
            for (j, &y) in mb.iter().enumerate() {
                if (i + j + par).is_multiple_of(2) { out.push((x, y)) } else { out.push((y, x)) }
            }
        }
    };
    let rot3 = season.rem_euclid(3) as usize;
    let pairs3: [(usize, usize); 2] = match rot3 {
        0 => [(0, 1), (2, 3)],
        1 => [(0, 2), (1, 3)],
        _ => [(0, 3), (1, 2)],
    };
    for c in 0..2 {
        for &(a, b) in &pairs3 {
            four(c * 4 + a, c * 4 + b, &mut out);
        }
    }
    let rot4 = season.rem_euclid(4) as usize;
    for a in 0..4 {
        four(a, 4 + (a + rot4) % 4, &mut out);
    }
    // Same place, the two other divisions of the conference.
    for c in 0..2 {
        for a in 0..4 {
            let partner = pairs3.iter().find_map(|&(x, y)| {
                if x == a {
                    Some(y)
                } else if y == a {
                    Some(x)
                } else {
                    None
                }
            });
            for b in 0..4 {
                if b == a || Some(b) == partner || b < a {
                    continue;
                }
                let (ma, mb) = (members(c * 4 + a), members(c * 4 + b));
                for p in 0..ma.len().min(mb.len()) {
                    if (p + par + a + b).is_multiple_of(2) {
                        out.push((ma[p], mb[p]))
                    } else {
                        out.push((mb[p], ma[p]))
                    }
                }
            }
        }
    }
    // The 17th game (2021 on): AFC hosts in odd years.
    if games >= 17 {
        for a in 0..4 {
            let b = 4 + (a + rot4 + 2) % 4;
            let (ma, mb) = (members(a), members(b));
            for p in 0..ma.len().min(mb.len()) {
                if par == 1 { out.push((ma[p], mb[p])) } else { out.push((mb[p], ma[p])) }
            }
        }
    }
    // An 18th game (T11 change 1): another same-place game across
    // conferences, against a division not yet played, the NFC hosting when
    // the AFC hosts the 17th (proposed, not confirmed).
    if games >= 18 {
        for a in 0..4 {
            let b = 4 + (a + rot4 + 1) % 4;
            let (ma, mb) = (members(a), members(b));
            for p in 0..ma.len().min(mb.len()) {
                if par == 1 { out.push((mb[p], ma[p])) } else { out.push((ma[p], mb[p])) }
            }
        }
    }
    out
}

/// Which pairs of teams a phase of the old formula may match.
type Pairing = fn(&NflSeason, usize, usize) -> bool;

/// The season's games before the 2002 realignment, with three divisions a
/// conference of four to six teams (proposed, not confirmed: the real
/// formula's shape, not its exact rotation): home and away against the
/// division, four games against the other conference (a division in
/// rotation, same-place teams first), and the rest against the
/// conference's other divisions, same-place teams first. Home and away
/// even out over the season.
fn old_formula(s: &NflSeason, season: i32, games: u32, seed: u64) -> Vec<(usize, usize)> {
    let n = s.div.len();
    let teams: Vec<usize> = (0..n).filter(|&t| s.active[t]).collect();
    let size = |d: usize| teams.iter().filter(|&&t| s.div[t] == d).count() as i32;
    let mut out = Vec::new();
    for &a in &teams {
        for &b in &teams {
            if a != b && s.div[a] == s.div[b] {
                out.push((a, b));
            }
        }
    }
    let mut met = vec![vec![false; n]; n];
    for &(a, b) in &out {
        met[a][b] = true;
    }
    let mut need: Vec<i32> =
        (0..n).map(|t| if s.active[t] { (games as i32 - 2 * (size(s.div[t]) - 1)).max(0) } else { 0 }).collect();
    // East, Central and West as 0, 1 and 2 within each conference.
    let slot = |d: usize| match d % 4 {
        0 => 0,
        1 => 1,
        _ => 2,
    };
    let rot = season.rem_euclid(3) as usize;
    let cost = |a: usize, b: usize| -> f64 {
        let place = (s.prev_place[a] as f64 - s.prev_place[b] as f64).abs();
        let jitter = key_uniform(seed ^ 0x0F0F, (a.min(b) as u64) << 8 ^ a.max(b) as u64);
        let other = if s.conf[a] == s.conf[b] {
            0.0
        } else {
            let (x, y) =
                if s.conf[a] == 0 { (slot(s.div[a]), slot(s.div[b])) } else { (slot(s.div[b]), slot(s.div[a])) };
            if (x + rot) % 3 == y { 0.0 } else { 1.0 }
        };
        other + 0.3 * place + 0.5 * jitter
    };
    let mut edges: Vec<(usize, usize)> = Vec::new();
    // Four against the other conference, the rest inside it, then anyone
    // still short plays whoever is left.
    let phases: [(Pairing, bool); 3] =
        [(|s, a, b| s.conf[a] != s.conf[b], true), (|s, a, b| s.conf[a] == s.conf[b], false), (|_, _, _| true, false)];
    for (allowed, capped) in phases {
        let mut q: Vec<i32> = need.iter().map(|&x| if capped { x.min(4) } else { x }).collect();
        let before = q.clone();
        quota_match(&mut q, &mut met, &mut edges, |a, b| allowed(s, a, b), &cost);
        for t in 0..n {
            need[t] -= before[t] - q[t];
        }
    }
    // Home and away: the side with fewer home games so far hosts, then
    // lopsided teams swap games until no swap helps.
    let mut bal = vec![0i32; n];
    let mut sided: Vec<(usize, usize)> = Vec::with_capacity(edges.len());
    for &(a, b) in &edges {
        let flip = key_uniform(seed ^ 0x4A, (a as u64) << 8 ^ b as u64) < 0.5;
        let (h, v) = if bal[a] < bal[b] || (bal[a] == bal[b] && flip) { (a, b) } else { (b, a) };
        bal[h] += 1;
        bal[v] -= 1;
        sided.push((h, v));
    }
    for _ in 0..50 {
        let mut changed = false;
        for g in sided.iter_mut() {
            let (h, v) = *g;
            if bal[h] - bal[v] > 2 {
                *g = (v, h);
                bal[h] -= 2;
                bal[v] += 2;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    out.extend(sided);
    out
}

/// Pairs teams up to each one's quota of games: the team with the most
/// games still to place takes its cheapest allowed partner. When none is
/// left, it takes the place of one side of a pairing already made and the
/// other side goes to a team still short; when even that fails, the rest
/// of its quota passes to the next phase.
fn quota_match(
    q: &mut [i32],
    met: &mut [Vec<bool>],
    edges: &mut Vec<(usize, usize)>,
    allowed: impl Fn(usize, usize) -> bool,
    cost: &impl Fn(usize, usize) -> f64,
) {
    let n = q.len();
    let start = edges.len();
    let link = |met: &mut [Vec<bool>], a: usize, b: usize, on: bool| {
        met[a][b] = on;
        met[b][a] = on;
    };
    while let Some(x) = (0..n).filter(|&t| q[t] > 0).max_by_key(|&t| (q[t], n - t)) {
        let y = (0..n)
            .filter(|&y| y != x && q[y] > 0 && allowed(x, y) && !met[x][y])
            .min_by(|&a, &b| cost(x, a).total_cmp(&cost(x, b)));
        if let Some(y) = y {
            link(met, x, y, true);
            edges.push((x, y));
            q[x] -= 1;
            q[y] -= 1;
            continue;
        }
        let mut fixed = false;
        'swap: for e in start..edges.len() {
            let (a, b) = edges[e];
            if a == x || b == x {
                continue;
            }
            for (u, v) in [(a, b), (b, a)] {
                if !allowed(x, u) || met[x][u] {
                    continue;
                }
                let z = (0..n)
                    .find(|&z| z != u && z != v && q[z] > 0 && (z != x || q[x] >= 2) && allowed(z, v) && !met[z][v]);
                if let Some(z) = z {
                    link(met, a, b, false);
                    edges.swap_remove(e);
                    link(met, x, u, true);
                    link(met, z, v, true);
                    edges.push((x, u));
                    edges.push((z, v));
                    q[x] -= 1;
                    q[z] -= 1;
                    fixed = true;
                    break 'swap;
                }
            }
        }
        if !fixed {
            q[x] = 0;
        }
    }
}

/// Spreads a season with no bye weeks (before 1990): every team plays every
/// week, so each week is a perfect matching of the games left, found by a
/// keyed search that starts over when the last weeks can't be paired.
fn round_weeks(games: &[(usize, usize)], n: usize, n_weeks: usize, seed: u64) -> Option<Vec<Vec<(usize, usize)>>> {
    for attempt in 0..300u64 {
        let mut left = games.to_vec();
        let mut weeks = Vec::with_capacity(n_weeks);
        for k in 0..n_weeks {
            let Some(mut pick) = perfect_week(&left, n, seed ^ attempt << 24 ^ (k as u64) << 8) else { break };
            pick.sort_unstable_by(|a, b| b.cmp(a));
            weeks.push(pick.iter().map(|&g| left.swap_remove(g)).collect());
        }
        if weeks.len() == n_weeks && left.is_empty() {
            return Some(weeks);
        }
    }
    None
}

/// One week in which every team with games left plays one of them, or None.
fn perfect_week(left: &[(usize, usize)], n: usize, key: u64) -> Option<Vec<usize>> {
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (g, &(a, b)) in left.iter().enumerate() {
        adj[a].push(g);
        adj[b].push(g);
    }
    for (t, list) in adj.iter_mut().enumerate() {
        list.sort_by(|&x, &y| key_uniform(key ^ t as u64, x as u64).total_cmp(&key_uniform(key ^ t as u64, y as u64)));
    }
    let mut used: Vec<bool> = adj.iter().map(|a| a.is_empty()).collect();
    let mut pick = Vec::new();
    let mut budget = 5000u32;
    fn other(g: (usize, usize), t: usize) -> usize {
        if g.0 == t { g.1 } else { g.0 }
    }
    fn go(
        left: &[(usize, usize)],
        adj: &[Vec<usize>],
        used: &mut [bool],
        pick: &mut Vec<usize>,
        budget: &mut u32,
    ) -> bool {
        // The open team with the fewest open games goes first.
        let mut best: Option<(usize, usize)> = None;
        for t in 0..used.len() {
            if used[t] {
                continue;
            }
            let c = adj[t].iter().filter(|&&g| !used[other(left[g], t)]).count();
            if best.is_none_or(|b| c < b.1) {
                best = Some((t, c));
            }
        }
        let Some((t, c)) = best else { return true };
        if c == 0 || *budget == 0 {
            return false;
        }
        *budget -= 1;
        for &g in &adj[t] {
            let o = other(left[g], t);
            if used[o] {
                continue;
            }
            used[t] = true;
            used[o] = true;
            pick.push(g);
            if go(left, adj, used, pick, budget) {
                return true;
            }
            pick.pop();
            used[t] = false;
            used[o] = false;
        }
        false
    }
    go(left, &adj, &mut used, &mut pick, &mut budget).then_some(pick)
}

/// Spreads the games over the weeks, one game a team a week, with byes
/// between weeks 4 and 14 (a keyed search; any byes are allowed if it fails).
fn assign_weeks(games: &[(usize, usize)], n: usize, n_weeks: usize, seed: u64) -> Vec<Vec<(usize, usize)>> {
    // With an odd number of teams someone sits every week, so byes can't
    // all fall in weeks 4-14.
    let mut playing = vec![false; n];
    for &(a, b) in games {
        playing[a] = true;
        playing[b] = true;
    }
    let even = playing.iter().filter(|&&p| p).count() % 2 == 0;
    for attempt in 0..400u64 {
        let strict = attempt < 300 && even;
        let mut left: Vec<(usize, usize)> = games.to_vec();
        let mut weeks: Vec<Vec<(usize, usize)>> = Vec::with_capacity(n_weeks);
        let mut ok = true;
        for k in 0..n_weeks {
            let weeks_left = n_weeks - k;
            let mut remaining = vec![0usize; n];
            for &(a, b) in &left {
                remaining[a] += 1;
                remaining[b] += 1;
            }
            let bye_ok = !strict || (3..=13).contains(&k);
            let mut best: Option<Vec<(usize, usize)>> = None;
            for tries in 0..60u64 {
                let key = seed ^ attempt << 32 ^ (k as u64) << 16 ^ tries;
                let mut order: Vec<usize> = (0..left.len()).collect();
                // Teams that must play every remaining week go first.
                order.sort_by(|&x, &y| {
                    let ux = remaining[left[x].0].max(remaining[left[x].1]) as f64 + key_uniform(key, x as u64);
                    let uy = remaining[left[y].0].max(remaining[left[y].1]) as f64 + key_uniform(key, y as u64);
                    uy.total_cmp(&ux)
                });
                let mut used = vec![false; n];
                let mut pick = Vec::new();
                for &g in &order {
                    let (a, b) = left[g];
                    if !used[a] && !used[b] {
                        used[a] = true;
                        used[b] = true;
                        pick.push(g);
                    }
                }
                // Everyone with as many games as weeks left must play; outside the
                // bye weeks, everyone with games left plays.
                let fine = (0..n).all(|t| used[t] || remaining[t] == 0 || (remaining[t] < weeks_left && bye_ok));
                if fine {
                    best = Some(pick.iter().map(|&g| left[g]).collect());
                    break;
                }
            }
            let Some(week) = best else {
                ok = false;
                break;
            };
            left.retain(|g| !week.contains(g));
            weeks.push(week);
        }
        if ok && left.is_empty() {
            return weeks;
        }
    }
    // Fallback: greedy, any leftover games played in the last week.
    let mut left: Vec<(usize, usize)> = games.to_vec();
    let mut weeks = Vec::new();
    for _ in 0..n_weeks {
        let mut used = vec![false; n];
        let mut week = Vec::new();
        left.retain(|&(a, b)| {
            if !used[a] && !used[b] {
                used[a] = true;
                used[b] = true;
                week.push((a, b));
                false
            } else {
                true
            }
        });
        weeks.push(week);
    }
    if let Some(last) = weeks.last_mut() {
        last.extend(left);
    }
    weeks
}

/// The NFL day.
pub(crate) fn day(w: &mut World, d: Date) {
    if w.nfl.is_empty() || w.nfl_season.is_none() {
        return;
    }
    let y = w.dates.clone();
    dated_events(w, d, &y);
    if d == y.combine_invites {
        recruiting::combine_invites(w);
    }
    if d == y.combine_end {
        recruiting::combine(w);
    }
    if d == y.annual_meeting.0 {
        league_meeting(w, d);
    }
    if d == y.schedule_release {
        build_schedule(w, d);
    }
    // The season kicks off on the opener's own day, before its game.
    let first =
        w.nfl_season.as_ref().and_then(|s| s.slots.iter().flatten().map(|x| x.at.date).min()).unwrap_or(y.nfl_kickoff);
    if d == first {
        kickoff(w, d);
    }
    flex(w, d, &y);
    super::clock::timed("nfl games", || play_day(w, d));
    if d == y.black_monday {
        black_monday(w, d);
    }
    if d == y.wild_card {
        start_round(w, 1, d);
    }
    if d == y.divisional {
        start_round(w, 2, d);
    }
    if d == y.conference_title_games {
        start_round(w, 3, d);
    }
    play_round_games(w, d);
    // NFL hires after the conference title games: weeks of interviews, and
    // a playoff team's coaches can't take a job until it is out (T9).
    if d == y.conference_title_games.add(1) {
        staff::carousel(w, staff::Scope::NflHires);
    }
    if d == y.super_bowl {
        play_round(w, 4, d);
        // The Super Bowl teams' coordinators are free to go now.
        staff::carousel(w, staff::Scope::NflHires);
    }
    // The pros' own day: the league year, free agency, camp, cutdowns,
    // the street and the season's end (section 5).
    super::clock::timed("nfl people", || super::pro::day(w, d));
}

fn post(w: &mut World, d: Date, kind: Kind, loud: Loudness, rule: &'static str, text: impl Into<String>) {
    w.calendar.post(Event::new(d, Level::Nfl, kind, loud, text).rule(rule));
}

fn post_at(w: &mut World, m: Moment, kind: Kind, loud: Loudness, rule: &'static str, text: impl Into<String>) {
    w.calendar.post(Event::new(m.date, Level::Nfl, kind, loud, text).at(m).rule(rule));
}

/// The year's dated steps that are a date, a window or a slot other sections
/// fill (T2, T7, A1-A8, A11-A17, A44-A46).
fn dated_events(w: &mut World, d: Date, y: &YearDates) {
    let season = w.year;
    if d == y.start {
        post(
            w,
            d,
            Kind::Window,
            Loudness::Small,
            "CBA: offseason",
            "Players are on their own until the offseason program; rehab can happen at the facility.",
        );
        post(w, d, Kind::Window, Loudness::Small, "CBA: reserve/future", "Reserve/future contracts can be signed.");
        // The cap began in 1994; 2010 was the one uncapped year after it.
        let cap = w.rules.salary_cap;
        let text = match cap {
            Some(c) => Some(format!("The {season} salary cap is set at ${:.1} million.", c / 1.0e6)),
            None if season >= 1994 => Some(format!("The {season} league year has no salary cap.")),
            None => None,
        };
        if let Some(text) = text {
            post(w, d.add(10), Kind::Date, Loudness::Medium, "CBA: salary cap", text);
        }
    }
    if d == y.tag_window_open {
        post(w, d, Kind::Window, Loudness::Medium, "CBA: tag window", "The franchise and transition tag window opens.");
    }
    if d == y.tag_deadline.date {
        post_at(w, y.tag_deadline, Kind::Deadline, Loudness::Big, "CBA: tag deadline", "Franchise tag deadline.");
    }
    if d == y.combine_start {
        let text = if crate::calendar::anchors::cancelled("combine workouts", y.season) {
            "The scouting combine opens in Indianapolis for medical checks only: the workouts move to the pro days."
        } else {
            "The scouting combine opens in Indianapolis."
        };
        post(w, d, Kind::Date, Loudness::Medium, "NFL: combine", text);
        // A11, A46: agents and teams meet; free agency whispers.
        post(
            w,
            d,
            Kind::Whisper,
            Loudness::Small,
            "",
            "Combine week: agents and teams meet in Indianapolis, and free agency whispers start.",
        );
    }
    if let Some(open) = y.tampering_open
        && d == open.date
    {
        let days = if season <= 2015 { "three-day" } else { "two-day" };
        post_at(
            w,
            open,
            Kind::Window,
            Loudness::Big,
            "CBA: negotiating window",
            format!("The {days} negotiating window opens: teams may talk to other teams' free agents."),
        );
    }
    if d == y.league_year.date {
        let lock = w.rules.lockout_days;
        match lock {
            Some(days) => post(
                w,
                d,
                Kind::Date,
                Loudness::Huge,
                "CBA: lockout",
                format!("The league year is frozen by a lockout for {days} days."),
            ),
            None => post_at(
                w,
                y.league_year,
                Kind::Date,
                Loudness::Big,
                "CBA: new league year",
                "The new league year opens at 4pm ET: free agency and trades begin.",
            ),
        }
    }
    if d == y.roster_bonus_window.0 {
        post(
            w,
            d,
            Kind::Slot,
            Loudness::Small,
            "contracts: roster bonuses",
            "Roster bonuses come due over the next days: cuts cluster before them.",
        );
    }
    if d == y.comp_picks {
        post(w, d, Kind::Date, Loudness::Small, "CBA: compensatory picks", "Compensatory picks are awarded.");
    }
    if d == y.pro_days.0 {
        post(w, d, Kind::Window, Loudness::Small, "NFL: pro days", "Pro day season opens.");
        post(
            w,
            d,
            Kind::Window,
            Loudness::Small,
            "NFL: top-30 visits",
            "Teams may host up to 30 prospects on official visits, through the deadline.",
        );
    }
    let todays: Vec<usize> = w.nfl_season.as_ref().unwrap().pro_days.iter().filter(|x| x.1 == d).map(|x| x.0).collect();
    for p in todays {
        super::pro::scouts::pro_day(w, p, d);
        let text = format!("{} pro day.", w.program_name(p));
        w.calendar
            .post(Event::new(d, Level::College, Kind::Date, Loudness::Small, text).program(p).rule("NFL: pro day"));
    }
    let tickets: Vec<usize> =
        w.nfl_season.as_ref().unwrap().ticket_days.iter().filter(|x| x.1 == d).map(|x| x.0).collect();
    for t in tickets {
        let text = format!("{} season ticket renewal deadline.", w.nfl[t].name);
        w.calendar
            .post(Event::new(d, Level::Nfl, Kind::Slot, Loudness::Small, text).team(t).rule("team: tickets (A17)"));
    }
    if d == Date::ymd(season, 4, 1) {
        post(
            w,
            d,
            Kind::Whisper,
            Loudness::Small,
            "",
            "Draft smokescreen season: teams leak false interest until the draft.",
        );
    }
    if d == y.top30_deadline {
        super::pro::scouts::spring(w);
        post(w, d, Kind::Deadline, Loudness::Small, "NFL: top-30 visits", "Deadline for top-30 visits.");
    }
    if d == y.offseason_new_coach {
        post(
            w,
            d,
            Kind::Window,
            Loudness::Small,
            "CBA: phase 1",
            "Teams with a new head coach may start their offseason program (phase 1, strength and conditioning) and add a voluntary minicamp.",
        );
    }
    if d == y.offseason_returning {
        post(
            w,
            d,
            Kind::Window,
            Loudness::Small,
            "CBA: phase 1",
            "Offseason programs open: two weeks of strength and conditioning only (phase 1).",
        );
    }
    if d == y.offseason_returning.add(14) {
        post(
            w,
            d,
            Kind::Window,
            Loudness::Small,
            "CBA: phase 2",
            "Phase 2: three weeks of individual drills, no offense against defense.",
        );
    }
    if d == y.rfa_deadline {
        post(
            w,
            d,
            Kind::Deadline,
            Loudness::Small,
            "CBA: RFA offer sheets",
            "Restricted free agents' offer sheet deadline.",
        );
    }
    if d == y.fifth_year_option {
        post(
            w,
            d,
            Kind::Deadline,
            Loudness::Small,
            "CBA: fifth-year option",
            "Fifth-year option deadline for first-round picks.",
        );
    }
    if d == y.rookie_minicamp {
        post(w, d, Kind::Practice, Loudness::Small, "CBA: rookie minicamp", "Rookie minicamps.");
    }
    if d == y.schedule_release.add(1) {
        win_totals(w, d);
    }
    if d == y.otas.0 {
        post(
            w,
            d,
            Kind::Window,
            Loudness::Small,
            "CBA: phase 3 (OTAs)",
            "OTAs open (phase 3): 7-on-7 and 11-on-11 without live contact; workout bonuses count attendance.",
        );
    }
    if d == y.rookie_program {
        post(
            w,
            d,
            Kind::Window,
            Loudness::Small,
            "NFL: rookie program",
            "The league's rookie program takes rookies away from their teams for its days.",
        );
    }
    if d == y.june_1 {
        post(
            w,
            d,
            Kind::Date,
            Loudness::Small,
            "CBA: June 1",
            "June 1: a release now spreads dead money over two years.",
        );
    }
    if d == y.minicamp.0 {
        post(w, d, Kind::Practice, Loudness::Small, "CBA: mandatory minicamp", "Mandatory minicamps (three days).");
    }
    if d == y.ring_ceremony {
        let champ = w.nfl_season.as_ref().and_then(|_| w.prev_champion());
        if let Some(t) = champ {
            let text = format!("The {} receive their Super Bowl rings.", w.nfl[t].name);
            w.calendar.post(Event::new(d, Level::Nfl, Kind::Ceremony, Loudness::Small, text).team(t));
        }
    }
    if d == y.summer_break.0 {
        post(w, d, Kind::Window, Loudness::Small, "CBA: summer break", "Summer break: players train on their own.");
        post(
            w,
            d,
            Kind::Slot,
            Loudness::Small,
            "",
            "Offseason weddings, youth camps, charity events and quarterback-led throwing sessions fill the summer.",
        );
    }
    if d == y.tag_long_term_deadline.date {
        post_at(
            w,
            y.tag_long_term_deadline,
            Kind::Deadline,
            Loudness::Big,
            "CBA: tagged players",
            "Deadline for long-term deals with tagged players.",
        );
    }
    if d == y.veterans_report.add(-3) {
        post(w, d, Kind::Slot, Loudness::Small, "owners (A44)", "Preseason owner meetings: each team's goals are set.");
    }
    if d == y.rookies_report {
        post(w, d, Kind::Practice, Loudness::Small, "CBA: training camp", "Rookies report to training camp.");
    }
    if d == y.veterans_report {
        post(
            w,
            d,
            Kind::Practice,
            Loudness::Medium,
            "CBA: training camp",
            "Veterans report: a player who stays away is holding out (fines of about $50,000 a day); one who reports but won't practice is holding in.",
        );
    }
    if d == y.veterans_report.add(5) {
        post(w, d, Kind::Practice, Loudness::Small, "CBA: acclimation", "Five days without pads are done: pads go on.");
    }
    if d == y.hof_game {
        let text = if crate::calendar::anchors::cancelled("Hall of Fame Game", y.season) {
            "The Hall of Fame Game is called off."
        } else {
            "The Hall of Fame Game opens the preseason."
        };
        post(w, d, Kind::Game, Loudness::Medium, "", text);
    }
    if d == y.hof_enshrinement {
        let text = if crate::calendar::anchors::cancelled("Hall of Fame enshrinement", y.season) {
            "No enshrinement in Canton this year: the class goes in next summer."
        } else {
            "Hall of Fame enshrinement in Canton."
        };
        post(w, d, Kind::Ceremony, Loudness::Medium, "", text);
    }
    for (k, p) in y.preseason.iter().enumerate() {
        if d == *p {
            post(w, d, Kind::Game, Loudness::Small, "", format!("Preseason week {}.", k + 1));
        }
    }
    if d == y.cutdown.date {
        post_at(
            w,
            y.cutdown,
            Kind::Deadline,
            Loudness::Huge,
            "CBA: roster cutdown",
            format!("Cutdown day: rosters go from 90 to {}.", w.rules.nfl_roster),
        );
    }
    if d == y.waiver_claims.date {
        post_at(
            w,
            y.waiver_claims,
            Kind::Deadline,
            Loudness::Big,
            "CBA: waivers",
            "Waiver claims on cut players are awarded.",
        );
    }
    if d == y.practice_squads {
        post(
            w,
            d,
            Kind::Date,
            Loudness::Medium,
            "CBA: practice squads",
            format!("Practice squads are signed ({} spots).", w.rules.nfl_practice_squad),
        );
    }
    if d == y.trade_deadline.date.add(-14) {
        post(w, d, Kind::Whisper, Loudness::Small, "", "Trade rumors pick up ahead of the deadline.");
    }
    if d == y.trade_deadline.date {
        post_at(
            w,
            y.trade_deadline,
            Kind::Deadline,
            Loudness::Huge,
            "CBA: trade deadline",
            "The trade deadline, 4pm ET.",
        );
    }
    if d == y.pro_bowl_voting.0 {
        post(w, d, Kind::Window, Loudness::Small, "", "Pro Bowl voting opens.");
    }
    if let Some(&last) = y.nfl_weeks.last()
        && d == last
    {
        post(
            w,
            d,
            Kind::Deadline,
            Loudness::Small,
            "contracts: incentives",
            "The last regular-season games: every incentive is judged on the season's final numbers.",
        );
        post(
            w,
            d,
            Kind::Slot,
            Loudness::Small,
            "owners (A44)",
            "End-of-season reviews with the owners, the day before Black Monday.",
        );
    }
    if d == y.all_pro {
        post(w, d, Kind::Award, Loudness::Medium, "", "The All-Pro teams are named.");
    }
    if d == y.hof_finalists {
        post(w, d, Kind::Award, Loudness::Small, "", "Hall of Fame finalists are named.");
    }
    // The Pro Bowl: before the Super Bowl from the 2009 season; to 2008 it
    // came the Sunday after, inside the next football year. The Pro Bowl
    // Games format from the 2022 season.
    if Some(d) == y.pro_bowl_games || Some(d) == y.pro_bowl_prev {
        let text = if Some(d) == y.pro_bowl_games && season >= 2022 { "The Pro Bowl Games." } else { "The Pro Bowl." };
        post(w, d, Kind::Game, Loudness::Small, "", text);
    }
    if d == y.nfl_honors {
        post(
            w,
            d,
            Kind::Award,
            Loudness::Big,
            "",
            "NFL Honors: MVP and the season's awards; the Hall of Fame class is announced.",
        );
    }
    let byes: Vec<usize> =
        (0..w.nfl.len()).filter(|&t| w.nfl_season.as_ref().unwrap().byes[t].is_some_and(|b| b.add(-5) == d)).collect();
    if !byes.is_empty() {
        let names: Vec<&str> = byes.iter().map(|&t| w.nfl[t].name).collect();
        let text =
            format!("Bye week (four days off by the CBA, self-scouting, owner check-ins): {}.", names.join(", "));
        post(w, d, Kind::Slot, Loudness::Small, "CBA: bye week", text);
    }
}

/// The league meeting in late March (topic 11): from 2027 the agreed NFL
/// changes are proposed and voted on; a relocation request is heard.
fn league_meeting(w: &mut World, d: Date) {
    let season = w.year;
    post(
        w,
        d,
        Kind::Date,
        Loudness::Medium,
        "NFL: annual meeting",
        "The league's annual meeting: rules proposals are voted on.",
    );
    let mut rng = w.rng.fork();
    for text in w.changes.league_meeting(season, &mut rng) {
        w.log.rule_changes.push(text.clone());
        post(w, d, Kind::RuleChange, Loudness::Big, "owners' vote", text);
    }
    if season >= 2027 && rng.chance(crate::calendar::rules::CHANGE_ODDS.relocation_request) {
        relocate(w, d, &mut rng);
    }
}

/// A team asks to move (topic 11, change 5): the smallest market with the
/// least patient owner asks; 24 of 32 owners must agree. Proposed, not
/// confirmed: the cities without a team that real bids have named.
fn relocate(w: &mut World, d: Date, rng: &mut crate::rng::Rng) {
    const CITIES: [&str; 6] = ["San Antonio", "St. Louis", "San Diego", "Portland", "Salt Lake City", "Oakland"];
    let t = (0..w.nfl.len())
        .filter(|&t| franchise::active(t, w.year))
        .max_by(|&a, &b| {
            let ka = team_row(a).map_or(16, |r| r.market_rank) as f64 - w.nfl[a].owner;
            let kb = team_row(b).map_or(16, |r| r.market_rank) as f64 - w.nfl[b].owner;
            ka.total_cmp(&kb)
        })
        .unwrap_or(0);
    let yes = (0..32).filter(|_| rng.chance(0.7)).count() as u32;
    let old = w.nfl[t].name;
    let city = CITIES[rng.int(0, CITIES.len() as i32 - 1) as usize];
    if yes >= 24 {
        let nickname = old.rsplit(' ').next().unwrap_or("Team");
        let name: &'static str = Box::leak(format!("{city} {nickname}").into_boxed_str());
        w.nfl[t].name = name;
        let text = format!(
            "Owners approve the {old}' move to {city}, {yes}-{}: they play next season as the {name}.",
            32 - yes
        );
        w.changes.applied.push(crate::calendar::rules::AppliedChange {
            change: crate::calendar::Change::Relocation,
            season: w.year,
            detail: format!("{old} to {city}"),
        });
        w.log.rule_changes.push(text.clone());
        post(w, d, Kind::RuleChange, Loudness::Huge, "owners' vote", text);
    } else {
        let text = format!("The {old}' request to move to {city} fails, {yes} votes of the 24 needed.");
        post(w, d, Kind::RuleChange, Loudness::Medium, "owners' vote", text);
    }
}

/// Schedule release in May: the season's games by the formula.
fn build_schedule(w: &mut World, d: Date) {
    let season = w.year;
    let games = w.rules.nfl_games;
    let n_weeks = w.dates.nfl_weeks.len();
    let n = w.nfl.len();
    let seed = w.options.seed as u64 ^ (season as u64) << 22 ^ 0x5343;
    let real = real_schedule(w, season, n_weeks);
    let s = w.nfl_season.as_mut().unwrap();
    s.year = season;
    let from_real = real.is_some();
    match real {
        Some((weeks, slots)) => {
            s.weeks = weeks;
            s.slots = slots;
        }
        None => {
            // 1987's schedule was drawn for 16 games before the strike
            // cancelled week 3 (never made up).
            let drawn = games.max(16);
            let list = if franchise::eight_divisions(season) {
                opponent_formula(s, season, drawn)
            } else {
                old_formula(s, season, drawn, seed)
            };
            // Before 1990 there were no byes: every team played every week.
            s.weeks = if n_weeks <= drawn as usize {
                round_weeks(&list, n, n_weeks, seed).unwrap_or_else(|| assign_weeks(&list, n, n_weeks, seed))
            } else {
                assign_weeks(&list, n, n_weeks, seed)
            };
            if season == 1987 && s.weeks.len() > 2 {
                s.weeks[2].clear();
            }
        }
    }
    if !from_real {
        holiday_hosts(w, season);
        formula_slots(w, season, seed);
    }
    let sundays = w.dates.nfl_weeks.clone();
    let s = w.nfl_season.as_mut().unwrap();
    for (k, week) in s.weeks.iter().enumerate() {
        let mut playing = vec![false; n];
        for &(a, b) in week {
            playing[a] = true;
            playing[b] = true;
        }
        for t in 0..n {
            if s.active[t] && !playing[t] && s.byes[t].is_none() {
                s.byes[t] = sundays.get(k).copied();
            }
        }
    }
    let abroad = s.slots.iter().flatten().filter(|x| x.site.is_some()).count();
    let prime = s.slots.iter().flatten().filter(|x| x.window.prime()).count();
    let how = if from_real { "the real 2026 schedule" } else { "the opponent formula" };
    let abroad = if abroad > 0 { format!(", {abroad} abroad") } else { String::new() };
    post(
        w,
        d,
        Kind::Date,
        Loudness::Medium,
        "NFL: schedule release",
        format!(
            "The {season} schedule is released ({how}): {games} games over {n_weeks} weeks, {prime} in prime time{abroad}."
        ),
    );
}

// --- Game slots (T1.2, T2) ------------------------------------------------------------

/// The Bills' home away from home, 2008-2013.
const TORONTO: &str = "Rogers Centre, Toronto";

const SCHEDULE_2026: &str = include_str!("../../../../data/real-world/nfl_schedule_2026.csv");

/// A season's games by week (home, away) and each game's slot.
type WeekSlots = (Vec<Vec<(usize, usize)>>, Vec<Vec<Slot>>);

/// Year one plays the real 2026 schedule (T2.2): every game on its real day
/// and kickoff, abroad where it really is. Any other year, or a league that
/// no longer matches it (a relocation), uses the formula.
fn real_schedule(w: &World, season: i32, n_weeks: usize) -> Option<WeekSlots> {
    if season != 2026 {
        return None;
    }
    let idx = |name: &str| w.nfl.iter().position(|t| t.name == name);
    let y = &w.dates;
    let mut weeks = vec![Vec::new(); n_weeks];
    let mut slots = vec![Vec::new(); n_weeks];
    for line in SCHEDULE_2026.lines().skip(1).filter(|l| !l.trim().is_empty()) {
        let f: Vec<&str> = line.split(',').collect();
        if f.len() < 6 {
            return None;
        }
        let k = f[0].parse::<usize>().ok()?.checked_sub(1)?;
        let mut ymd = f[1].split('-').map(|x| x.parse::<i32>().ok());
        let date = Date::ymd(ymd.next()??, ymd.next()?? as u32, ymd.next()?? as u32);
        let mut hm = f[2].split(':').map(|x| x.parse::<u16>().ok());
        let at = Moment::at(date, hm.next()??, hm.next()??);
        let (away, home) = (idx(f[3])?, idx(f[4])?);
        let site = (!f[5].trim().is_empty()).then(|| f[5].trim().to_string());
        let window = if site.is_some() {
            Window::International
        } else if date == y.thanksgiving {
            Window::Thanksgiving
        } else if date == y.christmas {
            Window::Christmas
        } else {
            match (date.weekday(), at.minute >= 19 * 60) {
                (Weekday::Sun, true) => Window::SundayNight,
                (Weekday::Sun, false) => Window::Sunday,
                (Weekday::Mon, _) => Window::MondayNight,
                (Weekday::Thu, _) if k == 0 => Window::Opener,
                (Weekday::Thu, _) => Window::ThursdayNight,
                (Weekday::Sat, _) => Window::Saturday,
                (Weekday::Fri, _) if date == y.thanksgiving.add(1) => Window::BlackFriday,
                (Weekday::Wed, _) if k == 0 => Window::Opener,
                _ => Window::Other,
            }
        };
        weeks.get_mut(k)?.push((home, away));
        slots[k].push(Slot { at, window, site, played: false, score: None });
    }
    (weeks.iter().map(|v| v.len()).sum::<usize>() > 0).then_some((weeks, slots))
}

/// Detroit and Dallas host on Thanksgiving: a formula week where both are at
/// home is swapped into Thanksgiving week (inside the bye weeks, so byes stay
/// where they were allowed).
fn holiday_hosts(w: &mut World, season: i32) {
    let y = w.dates.clone();
    let det = w.nfl.iter().position(|t| t.name == "Detroit Lions");
    let dal = w.nfl.iter().position(|t| t.name == "Dallas Cowboys");
    let s = w.nfl_season.as_mut().unwrap();
    let Some(tk) = y.nfl_weeks.iter().position(|&sun| sun.add(-3) == y.thanksgiving) else { return };
    let hosts = |week: &Vec<(usize, usize)>, t: Option<usize>| t.is_none_or(|t| week.iter().any(|g| g.0 == t));
    if hosts(&s.weeks[tk], det) && hosts(&s.weeks[tk], dal) {
        return;
    }
    let pick = (3..=13usize)
        .filter(|&j| j != tk && j < s.weeks.len())
        .find(|&j| hosts(&s.weeks[j], det) && hosts(&s.weeks[j], dal))
        .or_else(|| (3..=13usize).filter(|&j| j != tk && j < s.weeks.len()).find(|&j| hosts(&s.weeks[j], det)));
    if let Some(j) = pick
        && (3..=13).contains(&tk)
    {
        s.weeks.swap(tk, j);
    }
    let _ = season;
}

/// International games a season (the real counts to 2025, from memory, to
/// verify; then nine a year as in 2026, proposed, not confirmed): Europe,
/// the Bills' Toronto series (2008-2013), Mexico City, Brazil, Australia.
fn international_sites(season: i32) -> Vec<(&'static str, Weekday, u16, u16)> {
    const LONDON: [&str; 2] = [TOTTENHAM, WEMBLEY];
    const TOTTENHAM: &str = "Tottenham Hotspur Stadium, London";
    const WEMBLEY: &str = "Wembley Stadium, London";
    let europe = match season {
        2007..=2012 => 1,
        2013 => 2,
        2014..=2016 => 3,
        2017 => 4,
        2018 => 3,
        2019 => 4,
        2020 => 0,
        2021 => 2,
        2022 => 4,
        2023 => 5,
        2024 => 4,
        2025 => 6,
        s if s > 2025 => 6,
        _ => 0,
    };
    let mut v = Vec::new();
    for k in 0..europe {
        // London: Wembley alone to 2015, one game a year at Twickenham in
        // 2016-2017, Tottenham's stadium from 2019 (both 2021 games there).
        let site = match (k, season) {
            (3, 2022..) => "Allianz Arena, Munich",
            (4, 2023..) => "Deutsche Bank Park, Frankfurt",
            (5, 2025..) => "Santiago Bernabeu, Madrid",
            (2, 2016..=2017) => "Twickenham Stadium, London",
            (_, ..=2018) => WEMBLEY,
            (_, 2021) => TOTTENHAM,
            _ => LONDON[k % 2],
        };
        v.push((site, Weekday::Sun, 9, 30));
    }
    // The Bills host one game a year in Toronto (2008-2013).
    if (2008..=2013).contains(&season) {
        v.push((TORONTO, Weekday::Sun, 16, 5));
    }
    if matches!(season, 2005 | 2016 | 2017 | 2019 | 2022) || season >= 2026 {
        v.push(("Estadio Azteca, Mexico City", Weekday::Sun, 20, 20));
    }
    if season >= 2024 {
        v.push(("Sao Paulo", Weekday::Fri, 20, 15));
    }
    if season >= 2026 {
        v.push(("Melbourne Cricket Ground", Weekday::Thu, 20, 35));
    }
    v
}

/// Every game's day and kickoff for a formula year (T1.2, T2), from the
/// modern pattern by era (proposed, not confirmed): a Thursday opener at the
/// champion's home (2002 on), Thursday night games (late season 2006-2011,
/// every week from 2012), Sunday night and Monday night, Thanksgiving in
/// Detroit and Dallas with a night game from 2006 and a Black Friday game
/// from 2023, Christmas Day games from 2016 (on a Monday Christmas the night
/// game is Monday Night Football), a Sunday Christmas's slate moved to
/// Christmas Eve, Saturday games once the college regular season is over,
/// international games, western teams hosting in the late afternoon.
fn formula_slots(w: &mut World, season: i32, seed: u64) {
    let y = w.dates.clone();
    let n = w.nfl.len();
    let appeal: Vec<f64> = (0..n)
        .map(|t| {
            let market = team_row(t).map_or(16, |r| r.market_rank) as f64;
            let place = w.nfl_season.as_ref().unwrap().prev_place.get(t).copied().unwrap_or(2) as f64;
            0.6 * (3.0 - place) + 1.5 * (1.0 - (market - 1.0) / 31.0)
        })
        .collect();
    let west: Vec<bool> = (0..n)
        .map(|t| {
            let state = franchise::era(t, season.min(franchise::PRESENT)).map(|e| e.state);
            let state = state.or_else(|| team_row(t).map(|r| r.state.as_str()));
            state.is_some_and(|st| matches!(st, "CA" | "WA" | "AZ" | "NV" | "CO"))
        })
        .collect();
    let champ = w.nfl_champions.last().map(|c| c.1);
    let det = w.nfl.iter().position(|t| t.name == "Detroit Lions");
    let dal = w.nfl.iter().position(|t| t.name == "Dallas Cowboys");
    let buf = w.nfl.iter().position(|t| t.name == "Buffalo Bills");
    let sites = international_sites(season);
    let s = w.nfl_season.as_mut().unwrap();
    let n_weeks = s.weeks.len();
    let last = n_weeks.saturating_sub(1);
    // Which weeks host the international games: one a week from week 4 (Brazil in week 1).
    let mut abroad_week: Vec<Option<usize>> = vec![None; n_weeks];
    let mut used_abroad = vec![false; n];
    let toronto = sites.iter().any(|s| s.0 == TORONTO);
    // The Toronto game's week and opponent, placed before the London games
    // so an earlier London game cannot take its opponent abroad first.
    let mut toronto_game: Option<(usize, usize)> = None;
    let mut euro_week = 3;
    for (i, site) in sites.iter().enumerate() {
        let k = match site.1 {
            Weekday::Fri | Weekday::Thu => 0,
            _ if site.0.contains("Mexico") => 10.min(last),
            // Toronto: a Bills home game, the one nearest week 13 (the real
            // series ran from week 8 to week 15; Claude's pick, proposed,
            // not confirmed).
            _ if site.0 == TORONTO => {
                let found = [12, 13, 11, 14, 10, 9, 8, 7]
                    .into_iter()
                    .filter(|&k| k <= last && abroad_week[k].is_none())
                    .find(|&k| s.weeks[k].iter().any(|g| Some(g.0) == buf));
                if let Some(k) = found
                    && let Some(&(home, away)) = s.weeks[k].iter().find(|g| Some(g.0) == buf)
                {
                    toronto_game = Some((k, away));
                    used_abroad[home] = true;
                    used_abroad[away] = true;
                }
                found.unwrap_or(12.min(last))
            }
            _ => {
                let k = euro_week;
                euro_week += 1;
                k.min(last)
            }
        };
        if abroad_week[k].is_none() {
            abroad_week[k] = Some(i);
        }
    }
    let mut slots: Vec<Vec<Slot>> = Vec::with_capacity(n_weeks);
    // Standalone prime-time games per team (part D 36, N61): at most five
    // on the schedule as released (real cap, from memory, to verify).
    let mut prime = vec![0u32; s.wins.len()];
    for k in 0..n_weeks {
        let sun = y.nfl_weeks[k];
        let games = s.weeks[k].clone();
        let div_game = |g: &(usize, usize)| s.div[g.0] == s.div[g.1];
        let score = |g: &(usize, usize)| {
            appeal[g.0] + appeal[g.1] + 0.3 * key_uniform(seed ^ k as u64, (g.0 * 64 + g.1) as u64)
        };
        let mut taken = vec![false; games.len()];
        let mut slot: Vec<Option<Slot>> = vec![None; games.len()];
        let mut set = |taken: &mut Vec<bool>, g: usize, at: Moment, window: Window, site: Option<String>| {
            taken[g] = true;
            slot[g] = Some(Slot { at, window, site, played: false, score: None });
        };
        // The best game still open, by appeal; `pred` limits the choice.
        let best = |taken: &Vec<bool>, pred: &dyn Fn(&(usize, usize)) -> bool| -> Option<usize> {
            (0..games.len())
                .filter(|&g| !taken[g] && pred(&games[g]))
                .max_by(|&a, &b| score(&games[a]).total_cmp(&score(&games[b])).then(b.cmp(&a)))
        };
        let open = |g: &(usize, usize)| prime[g.0] < PRIME_CAP && prime[g.1] < PRIME_CAP;
        // International games first: a team goes abroad once a year. The
        // Bills host in Toronto, division rivals included.
        if let Some(i) = abroad_week[k] {
            let (site, day, h, m) = sites[i];
            let pick = (0..games.len())
                .filter(|&g| {
                    let (home, away) = games[g];
                    if site == TORONTO {
                        // Its two teams were marked abroad when it was placed.
                        Some(home) == buf && toronto_game == Some((k, away))
                    } else {
                        // In the Toronto years the Bills keep their trip abroad for it.
                        let bills = toronto && (Some(home) == buf || Some(away) == buf);
                        !used_abroad[home] && !used_abroad[away] && !div_game(&games[g]) && !bills
                    }
                })
                .min_by(|&a, &b| {
                    key_uniform(seed ^ 0xAB ^ k as u64, a as u64)
                        .total_cmp(&key_uniform(seed ^ 0xAB ^ k as u64, b as u64))
                });
            if let Some(g) = pick {
                used_abroad[games[g].0] = true;
                used_abroad[games[g].1] = true;
                let date = match day {
                    Weekday::Thu => sun.add(-3),
                    Weekday::Fri => sun.add(-2),
                    _ => sun,
                };
                set(&mut taken, g, Moment::at(date, h, m), Window::International, Some(site.to_string()));
            }
        }
        // The opener: the champion at home on Thursday (2002 on).
        if k == 0 && season >= 2002 {
            let g = best(&taken, &|g| Some(g.0) == champ).or_else(|| best(&taken, &|_| true));
            if let Some(g) = g {
                set(&mut taken, g, Moment::at(sun.add(-3), 20, 20), Window::Opener, None);
            }
        }
        // Thanksgiving: Detroit, Dallas, a night game (2006 on), Black Friday (2023 on).
        if sun.add(-3) == y.thanksgiving {
            let tg = y.thanksgiving;
            if let Some(g) = best(&taken, &|g| Some(g.0) == det) {
                set(&mut taken, g, Moment::at(tg, 12, 30), Window::Thanksgiving, None);
            }
            if let Some(g) = best(&taken, &|g| Some(g.0) == dal) {
                set(&mut taken, g, Moment::at(tg, 16, 30), Window::Thanksgiving, None);
            }
            if season >= 2006
                && let Some(g) = best(&taken, &|_| true)
            {
                set(&mut taken, g, Moment::at(tg, 20, 20), Window::Thanksgiving, None);
            }
            if season >= 2023
                && let Some(g) = best(&taken, &|_| true)
            {
                set(&mut taken, g, Moment::at(tg.add(1), 15, 0), Window::BlackFriday, None);
            }
        } else if k > 0 && k < last {
            // Thursday night (on a Thursday Christmas, the Christmas games
            // are the week's Thursday games).
            let tnf = (season >= 2012 || (2006..=2011).contains(&season) && k >= 9) && y.christmas != sun.add(-3);
            if tnf
                && let Some(g) = best(&taken, &|g| s.div[g.0] == s.div[g.1] && open(g))
                    .or_else(|| best(&taken, &open))
                    .or_else(|| best(&taken, &|_| true))
            {
                set(&mut taken, g, Moment::at(sun.add(-3), 20, 15), Window::ThursdayNight, None);
            }
        }
        // Christmas Day games on a weekday of the week (2016 on). On a
        // Monday Christmas the night game is Monday Night Football.
        let xmas = y.christmas;
        let n_xmas = christmas_games(season);
        let weekday_xmas = n_xmas > 0 && xmas != sun && xmas >= sun.add(-4) && xmas <= sun.add(1);
        if weekday_xmas {
            let all = [(13, 0), (16, 30), (20, 15)];
            for &(h, m) in &all[3 - n_xmas.min(3)..] {
                if let Some(g) = best(&taken, &|_| true) {
                    set(&mut taken, g, Moment::at(xmas, h, m), Window::Christmas, None);
                }
            }
        }
        // A Sunday Christmas: the week's Sunday games move to Saturday,
        // Christmas Eve, and only the Christmas windows stay on the day: the
        // night game, a late-afternoon game (not in 2011) and a 1pm game from
        // 2020 (2022: 11 games on Dec 24, 3 on Dec 25; from memory, to verify).
        let snf = if season >= 2006 { (20, 20) } else { (20, 15) };
        let sunday_xmas = xmas == sun;
        if sunday_xmas {
            let all = [(13, 0), (16, 30), snf];
            for &(h, m) in &all[3 - n_xmas.min(3)..] {
                if let Some(g) = best(&taken, &|_| true) {
                    set(&mut taken, g, Moment::at(xmas, h, m), Window::Christmas, None);
                }
            }
        }
        // Saturdays once the college regular season is over (2006 on); a
        // Sunday Christmas's whole slate plays on Saturday afternoon instead.
        if season >= 2006 && sun.add(-1) > y.army_navy && xmas != sun.add(-1) && !sunday_xmas {
            for &(h, m) in &[(16, 30), (20, 15)] {
                let g = if k == last { best(&taken, &|g| s.div[g.0] == s.div[g.1]) } else { best(&taken, &|_| true) };
                if let Some(g) = g {
                    set(&mut taken, g, Moment::at(sun.add(-1), h, m), Window::Saturday, None);
                }
            }
        }
        // Sunday night, then Monday night (none in the last week; on a
        // Christmas Sunday or Monday, the Christmas night game is it).
        if !sunday_xmas && let Some(g) = best(&taken, &open).or_else(|| best(&taken, &|_| true)) {
            set(&mut taken, g, Moment::at(sun, snf.0, snf.1), Window::SundayNight, None);
        }
        if k < last && !(weekday_xmas && xmas == sun.add(1)) {
            let mnf = if k == 0 && (2006..=2021).contains(&season) { 2 } else { 1 };
            for j in 0..mnf {
                if let Some(g) = best(&taken, &open).or_else(|| best(&taken, &|_| true)) {
                    let (h, m) = if mnf == 2 && j == 1 {
                        (22, 20)
                    } else if mnf == 2 {
                        (19, 10)
                    } else {
                        (20, 15)
                    };
                    set(&mut taken, g, Moment::at(sun.add(1), h, m), Window::MondayNight, None);
                }
            }
        }
        // Sunday afternoon (Christmas Eve's afternoon on a Sunday Christmas):
        // western hosts and a few others late.
        let (day, window) = if sunday_xmas { (sun.add(-1), Window::Saturday) } else { (sun, Window::Sunday) };
        let mut late = 0;
        for g in 0..games.len() {
            if taken[g] {
                continue;
            }
            let lateish = west[games[g].0] || late < 2 && key_uniform(seed ^ 0x1A7E ^ k as u64, g as u64) < 0.3;
            let at = if lateish {
                late += 1;
                if late % 2 == 0 { Moment::at(day, 16, 5) } else { Moment::at(day, 16, 25) }
            } else {
                Moment::at(day, 13, 0)
            };
            set(&mut taken, g, at, window, None);
        }
        slots.push(slot.into_iter().map(|x| x.unwrap()).collect());
        if season >= 2006 {
            for (g, sl) in slots.last().unwrap().iter().enumerate() {
                if matches!(sl.window, Window::SundayNight | Window::MondayNight | Window::ThursdayNight) {
                    prime[games[g].0] += 1;
                    prime[games[g].1] += 1;
                }
            }
        }
    }
    s.slots = slots;
}

fn capitalize(text: &str) -> String {
    let mut c = text.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// The most standalone prime-time games a team is scheduled for.
const PRIME_CAP: u32 = 5;

/// Christmas Day games by season (part D 30, N35): the real number each
/// year (2016-2025 from the schedules; earlier years from memory, to
/// verify), three a year after 2025 (proposed, not confirmed).
fn christmas_games(season: i32) -> usize {
    match season {
        1989 | 1995 | 2000 | 2009 | 2010 | 2011 | 2020 => 1,
        2005 | 2006 | 2016 | 2017 | 2021 | 2024 => 2,
        2022 | 2023 | 2025 => 3,
        s if s > 2025 => 3,
        _ => 0,
    }
}

/// Flexible scheduling (T2): from 2006 a better Sunday afternoon game
/// takes a Sunday night slot from week 11, 12 days ahead (6 for the last
/// week); from 2014 from week 5, with at most two flexes in weeks 5-10.
/// From 2023 Monday night (weeks 12-17, 12 days ahead) and Thursday night
/// (weeks 13-17, 28 days ahead, two a season) flex too (from memory, to
/// verify).
fn flex(w: &mut World, d: Date, y: &YearDates) {
    let season = w.year;
    if season < 2006 {
        return;
    }
    flex_window(w, d, y, Window::SundayNight);
    if season >= 2023 {
        flex_window(w, d, y, Window::MondayNight);
        flex_window(w, d, y, Window::ThursdayNight);
    }
}

fn flex_window(w: &mut World, d: Date, y: &YearDates, window: Window) {
    let season = w.year;
    let Some(s) = w.nfl_season.as_ref() else { return };
    if s.slots.is_empty() {
        return;
    }
    let last = s.weeks.len().saturating_sub(1);
    let (from, to) = match window {
        Window::SundayNight => (if season >= 2014 { 4 } else { 10 }, last),
        Window::MondayNight => (11, last.saturating_sub(1)),
        _ => (12, last.saturating_sub(1)),
    };
    let notice = |k: usize| match window {
        Window::ThursdayNight => -28,
        Window::SundayNight if k == last => -6,
        _ => -12,
    };
    let Some(k) = (from..=to).find(|&k| y.nfl_weeks.get(k).is_some_and(|&sun| d == sun.add(notice(k)))) else {
        return;
    };
    let early = window == Window::SundayNight && (4..=9).contains(&k);
    if early && s.early_flexes >= 2 || window == Window::ThursdayNight && s.tnf_flexes >= 2 {
        return;
    }
    let wins = |t: usize| s.wins[t] as f64 + 0.5 * s.ties[t] as f64;
    let value = |g: &(usize, usize)| wins(g.0) + wins(g.1) - 0.5 * (wins(g.0) - wins(g.1)).abs();
    let games = &s.weeks[k];
    let Some(night) = s.slots[k].iter().position(|x| x.window == window) else { return };
    // A team that played on Sunday night or Monday night the week before
    // isn't moved to a Thursday.
    let rested = |t: usize| {
        window != Window::ThursdayNight
            || k == 0
            || s.weeks[k - 1].iter().zip(&s.slots[k - 1]).all(|(g, sl)| {
                (g.0 != t && g.1 != t) || !matches!(sl.window, Window::SundayNight | Window::MondayNight)
            })
    };
    let Some(best) = (0..games.len())
        .filter(|&g| {
            s.slots[k][g].window == Window::Sunday
                && s.slots[k][g].site.is_none()
                && rested(games[g].0)
                && rested(games[g].1)
        })
        .max_by(|&a, &b| value(&games[a]).total_cmp(&value(&games[b])).then(b.cmp(&a)))
    else {
        return;
    };
    // Only a night game that lost its stakes is flexed out (part D 35,
    // N60: real about one to four a season): a team in it is out of the
    // race (well under .500 this late).
    let played = (s.wins[games[night].0] + s.losses[games[night].0] + s.ties[games[night].0]).max(1) as f64;
    let dead = |t: usize| wins(t) / played < 0.4;
    if !(dead(games[night].0) || dead(games[night].1)) || value(&games[best]) < value(&games[night]) + 2.0 {
        return;
    }
    // A flex can't push a team past six standalone prime-time games.
    let prime = |t: usize| {
        s.slots
            .iter()
            .zip(&s.weeks)
            .flat_map(|(sl, wk)| sl.iter().zip(wk))
            .filter(|(sl, g)| {
                (g.0 == t || g.1 == t)
                    && matches!(sl.window, Window::SundayNight | Window::MondayNight | Window::ThursdayNight)
            })
            .count()
    };
    if prime(games[best].0) >= 6 || prime(games[best].1) >= 6 {
        return;
    }
    let (h, a) = games[best];
    let (oh, oa) = games[night];
    let s = w.nfl_season.as_mut().unwrap();
    let night_at = s.slots[k][night].at;
    let best_at = s.slots[k][best].at;
    s.slots[k][best].at = night_at;
    s.slots[k][best].window = window;
    s.slots[k][night].at = best_at;
    s.slots[k][night].window = Window::Sunday;
    if early {
        s.early_flexes += 1;
    }
    if window == Window::ThursdayNight {
        s.tnf_flexes += 1;
    }
    let text = format!(
        "Flexed into {}, week {}: the {} at the {}; the {} at the {} moves to Sunday afternoon.",
        window.name(),
        k + 1,
        w.nfl[a].name,
        w.nfl[h].name,
        w.nfl[oa].name,
        w.nfl[oh].name
    );
    post(w, d, Kind::Date, Loudness::Medium, "NFL: flexible scheduling", text);
}

/// Win totals after the schedule release (A45): the market's read of each
/// roster.
fn win_totals(w: &mut World, d: Date) {
    let games = w.rules.nfl_games as f64;
    let teams: Vec<usize> = (0..w.nfl.len()).filter(|&t| franchise::active(t, w.year)).collect();
    let s: Vec<(usize, f64)> = teams.iter().map(|&t| (t, w.nfl[t].strength())).collect();
    let m = s.len().max(1) as f64;
    let mean = s.iter().map(|x| x.1).sum::<f64>() / m;
    let sd = (s.iter().map(|x| (x.1 - mean).powi(2)).sum::<f64>() / m).sqrt().max(0.5);
    // A line sits on the half win nearest what the roster should win.
    let mut v: Vec<(f64, usize)> =
        s.iter().map(|&(t, x)| (((0.5 + 0.13 * (x - mean) / sd) * games).floor() + 0.5, t)).collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    let line = |x: &(f64, usize)| format!("{} {:.1}", w.nfl[x.1].name, x.0);
    let top: Vec<String> = v.iter().take(3).map(line).collect();
    let bottom: Vec<String> = v.iter().rev().take(3).map(line).collect();
    let text = format!("Preseason win totals are posted. Highest: {}. Lowest: {}.", top.join(", "), bottom.join(", "));
    post(w, d, Kind::News, Loudness::Small, "betting (A45)", text);
}

/// The opener's day: each roster and staff is read, and the season kicks off
/// with the first game.
fn kickoff(w: &mut World, d: Date) {
    let n = w.nfl.len();
    if w.nfl_season.as_ref().unwrap().weeks.is_empty() {
        build_schedule(w, d);
    }
    let active = w.nfl_season.as_ref().unwrap().active.clone();
    let strength: Vec<f64> = w.nfl.iter().map(|t| t.strength()).collect();
    let on: Vec<f64> = (0..n).filter(|&t| active[t]).map(|t| strength[t]).collect();
    let m = on.len().max(1) as f64;
    let mean = on.iter().sum::<f64>() / m;
    let sd = (on.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / m).sqrt().max(0.5);
    let z: Vec<f64> = (0..n).map(|t| if active[t] { (strength[t] - mean) / sd } else { 0.0 }).collect();
    let staff: Vec<f64> = w.nfl.iter().map(|t| (t.coaching - 13.0) / 2.5).collect();
    let s = w.nfl_season.as_mut().unwrap();
    s.z = z;
    s.staff = staff;
    s.expect = vec![0.0; n];
    s.started = true;
    let at = s.slots.iter().flatten().map(|x| x.at).filter(|m| m.date == d).min().unwrap_or(Moment::at(d, 20, 20));
    post_at(w, at, Kind::Date, Loudness::Big, "", "The NFL season kicks off.");
}

fn win_prob(s: &NflSeason, home: usize, away: usize, field: f64) -> f64 {
    let x = PER_SD * (s.z[home] - s.z[away]) + PER_STAFF * (s.staff[home] - s.staff[away]) + field;
    1.0 / (1.0 + (-x).exp())
}

/// The day's games (T1.2): each is played on its own day and kickoff. A
/// week's stakes are read before its first game; its lines open after its
/// last.
fn play_day(w: &mut World, d: Date) {
    let Some(s) = w.nfl_season.as_ref() else { return };
    if !s.started {
        return;
    }
    let mut todays: Vec<(usize, usize)> = Vec::new();
    for (k, week) in s.slots.iter().enumerate() {
        for (g, x) in week.iter().enumerate() {
            if x.at.date == d && !x.played {
                todays.push((k, g));
            }
        }
    }
    if todays.is_empty() {
        return;
    }
    let mut weeks: Vec<usize> = todays.iter().map(|x| x.0).collect();
    weeks.dedup();
    for &k in &weeks {
        let s = w.nfl_season.as_ref().unwrap();
        if s.slots[k].iter().all(|x| !x.played) {
            stakes(w, k, d);
            if k == 0 {
                post(
                    w,
                    d,
                    Kind::Slot,
                    Loudness::Small,
                    "contracts: per-game bonuses (A7)",
                    "Per-game roster bonuses are earned each game day by players on the active list.",
                );
            }
        }
    }
    let seed_of = |k: usize| w.options.seed as u64 ^ (w.year as u64) << 24 ^ (k as u64) << 10 ^ 0x4E47;
    let seeds: Vec<u64> = todays.iter().map(|&(k, _)| seed_of(k)).collect();
    // On the match engine (section 5): each game is played out.
    let outcomes = {
        let s = w.nfl_season.as_ref().unwrap();
        let fixtures: Vec<Fixture> = todays
            .iter()
            .zip(&seeds)
            .map(|(&(k, g), &seed)| {
                let (h, a) = s.weeks[k][g];
                let x = &s.slots[k][g];
                let loud = if x.window.prime() { 0.04 } else { 0.0 };
                Fixture {
                    home: h,
                    away: a,
                    at: x.at,
                    neutral: x.site.is_some(),
                    loud,
                    seed: seed ^ (h as u64) << 8 ^ a as u64,
                }
            })
            .collect();
        engine_games(w, &fixtures)
    };
    let mut played: Vec<([usize; 2], [f64; 2])> = Vec::new();
    let mut shown: Vec<(usize, usize, Slot)> = Vec::new();
    let s = w.nfl_season.as_mut().unwrap();
    for (n, (&(k, g), seed)) in todays.iter().zip(seeds).enumerate() {
        let (h, a) = s.weeks[k][g];
        let field = if s.slots[k][g].site.is_some() { 0.0 } else { HOME };
        let key = (h as u64) << 8 ^ a as u64;
        let p = win_prob(s, h, a, field);
        let pe = 1.0 / (1.0 + (-(PER_SD * (s.z[h] - s.z[a]) + field)).exp());
        s.expect[h] += pe;
        s.expect[a] += 1.0 - pe;
        // The engine's score, or the stand-in's draw.
        let (u, p, tie) = match &outcomes[n] {
            Some((r, _)) => {
                let [x, y] = r.stats.score;
                s.slots[k][g].score = Some([x.max(0) as u32, y.max(0) as u32]);
                played.push(([h, a], [s.z[a], s.z[h]]));
                (if x > y { 0.0 } else { 1.0 }, 0.5, x == y)
            }
            None => (key_uniform(seed, key), p, key_uniform(seed ^ 0x7E, key) < TIE),
        };
        let same_div = s.div[h] == s.div[a];
        if tie {
            s.ties[h] += 1;
            s.ties[a] += 1;
        } else {
            let (wn, l) = if u < p { (h, a) } else { (a, h) };
            s.wins[wn] += 1;
            s.losses[l] += 1;
            if same_div {
                s.div_wins[wn] += 1;
            }
            let x = &s.slots[k][g];
            if x.window.prime() || x.site.is_some() {
                shown.push((wn, l, x.clone()));
            }
        }
        s.slots[k][g].played = true;
    }
    let mut it = played.into_iter();
    for o in outcomes.iter().flatten() {
        if let Some((teams, z)) = it.next() {
            pro_games::record(w, teams, &o.0, &o.1, z);
        }
    }
    w.log.nfl_games += todays.len() as u32;
    // The national windows and the games abroad make the news.
    for (wn, l, x) in shown {
        let place = x.site.as_ref().map_or(String::new(), |site| format!(" at {site}"));
        let text =
            format!("{}{place}: the {} beat the {}.", capitalize(x.window.name()), w.nfl[wn].name, w.nfl[l].name);
        w.calendar.post(
            Event::new(x.at.date, Level::Nfl, Kind::Game, Loudness::Small, text)
                .team(wn)
                .at(x.at)
                .club_link(wn, w.nfl[wn].name)
                .club_link(l, w.nfl[l].name),
        );
    }
    // A45: next week's lines open after the week's last game.
    for k in weeks {
        let s = w.nfl_season.as_ref().unwrap();
        if s.slots[k].iter().all(|x| x.played) && k + 1 < s.weeks.len() {
            post(w, d, Kind::News, Loudness::Small, "betting (A45)", format!("Week {} lines open.", k + 2));
        }
    }
}

/// NFL games on the match engine once the world plays its games there
/// (`WorldOptions::engine_from`); None for each game on the stand-in.
fn engine_games(w: &World, games: &[Fixture]) -> Vec<Option<(crate::game::GameResult, [Side; 2])>> {
    let on = w.options.engine_from.is_none_or(|from| w.year >= from) && !w.clubs.is_empty();
    if !on {
        return games.iter().map(|_| None).collect();
    }
    pro_games::play(w, games)
}

/// The stakes test for NFL games (topic 8): in the last four weeks a game
/// is huge for a team when its result can clinch a playoff place or the
/// division or end its hopes (part D 27), or when its head coach's job
/// rides on the last two games (part B 27).
fn stakes(w: &mut World, k: usize, d: Date) {
    let s = w.nfl_season.as_ref().unwrap();
    let n_weeks = s.weeks.len();
    if k + 4 < n_weeks {
        return;
    }
    let per_conf = (w.rules.nfl_playoff_teams / 2) as usize;
    let n_teams = s.wins.len();
    let wins = |t: usize| s.wins[t] as f64 + 0.5 * s.ties[t] as f64;
    // Games each team has left, this week's included.
    let mut left = vec![0.0f64; n_teams];
    for week in &s.weeks[k..] {
        for &(h, a) in week {
            left[h] += 1.0;
            left[a] += 1.0;
        }
    }
    let best = |t: usize| wins(t) + left[t];
    // The `n`-th highest of a list (1 = highest).
    let nth = |mut v: Vec<f64>, n: usize| {
        v.sort_by(|a, b| b.total_cmp(a));
        v.get(n.saturating_sub(1)).copied().unwrap_or(f64::NEG_INFINITY)
    };
    // A game is a team's stake when its result can settle the team's
    // season (part D 27): a win clinches a playoff place or the division,
    // or a loss ends its hopes of either (wins only; tiebreakers left out).
    let settles = |t: usize| {
        let others: Vec<usize> = (0..n_teams).filter(|&o| o != t && s.conf[o] == s.conf[t]).collect();
        let clinch_spot = wins(t) + 1.0 > nth(others.iter().map(|&o| best(o)).collect(), per_conf);
        let alive = best(t) >= nth(others.iter().map(|&o| wins(o)).collect(), per_conf);
        let ends_spot = alive && best(t) - 1.0 < nth(others.iter().map(|&o| wins(o)).collect(), per_conf);
        let rivals: Vec<usize> = others.iter().copied().filter(|&o| s.div[o] == s.div[t]).collect();
        let div_alive = best(t) >= rivals.iter().map(|&o| wins(o)).fold(0.0, f64::max);
        let clinch_div = div_alive && wins(t) + 1.0 > rivals.iter().map(|&o| best(o)).fold(0.0, f64::max);
        let ends_div = div_alive && best(t) - 1.0 < rivals.iter().map(|&o| wins(o)).fold(0.0, f64::max);
        let spot_open = alive && wins(t) <= nth(others.iter().map(|&o| best(o)).collect(), per_conf);
        (spot_open && (clinch_spot || ends_spot)) || clinch_div || ends_div
    };
    // A head coach on the hot seat (part B 27): in his third season or
    // later, a losing year after a losing one; his last two games decide it.
    let hot = |t: usize| {
        let games = s.wins[t] + s.losses[t] + s.ties[t];
        let pct = wins(t) / games.max(1) as f64;
        let last = w.nfl[t].win_share.last().copied().unwrap_or(0.5);
        let tenure = w.nfl[t].head_coach.and_then(|c| w.coaches[c].job).map_or(0, |j| w.year - j.since);
        k + 2 >= n_weeks && tenure >= 2 && games >= 10 && pct < 0.4 && last < 0.45
    };
    let mut huge: Vec<(usize, usize, [bool; 2], Moment)> = Vec::new();
    for (g, &(h, a)) in s.weeks[k].iter().enumerate() {
        let side = [settles(h) || hot(h), settles(a) || hot(a)];
        if side[0] || side[1] {
            let at = s.slots.get(k).and_then(|v| v.get(g)).map_or(Moment::at(d, 13, 0), |x| x.at);
            huge.push((h, a, side, at));
        }
    }
    for (h, a, side, at) in huge {
        let s = w.nfl_season.as_mut().unwrap();
        s.huge[h] += side[0] as u32;
        s.huge[a] += side[1] as u32;
        // The league's news is loudest when both sides play for their
        // season; a one-sided stake is that team's huge game, not the week's.
        let (text, loud, team) = if side[0] && side[1] {
            (
                format!("A game with the season on it: the {} host the {}.", w.nfl[h].name, w.nfl[a].name),
                Loudness::Huge,
                h,
            )
        } else {
            let (t, o, verb) = if side[0] { (h, a, "host") } else { (a, h, "visit") };
            (
                format!("The {} play for their season when they {} the {}.", w.nfl[t].name, verb, w.nfl[o].name),
                Loudness::Big,
                t,
            )
        };
        w.calendar.post(
            Event::new(at.date, Level::Nfl, Kind::Game, loud, text)
                .team(team)
                .at(at)
                .regular()
                .club_link(h, w.nfl[h].name)
                .club_link(a, w.nfl[a].name),
        );
    }
}

/// Black Monday: the regular season's book, the playoff field, and the
/// firings (topic 2). Hires wait until after the conference title games.
fn black_monday(w: &mut World, d: Date) {
    let year = w.year;
    let n = w.nfl.len();
    let Some(s) = w.nfl_season.as_ref() else { return };
    if !s.started || s.booked {
        return;
    }
    let s = s.clone();
    // The season's book, under the coach who finished it.
    for t in (0..n).filter(|&t| s.active[t]) {
        w.nfl_seasons.push(NflTeamSeason {
            year: s.year,
            team: t,
            wins: s.wins[t],
            losses: s.losses[t],
            ties: s.ties[t],
            coach: w.nfl[t].head_coach,
            round: 0,
        });
    }
    for t in (0..n).filter(|&t| s.active[t]) {
        let games = (s.wins[t] + s.losses[t] + s.ties[t]).max(1);
        let share = (s.wins[t] as f64 + 0.5 * s.ties[t] as f64) / games as f64;
        let vs = share - s.expect[t] / games as f64;
        let team = &mut w.nfl[t];
        team.win_share.push(share);
        if team.win_share.len() > 3 {
            team.win_share.remove(0);
        }
        let (hc, oc, dc) = (team.head_coach, team.oc, team.dc);
        if let Some(h) = hc {
            let coach = &mut w.coaches[h];
            coach.wins += s.wins[t];
            coach.losses += s.losses[t];
            coach.season_reputation(vs, 0.0, true);
        }
        // A coordinator is judged on his unit: the team's year and his side's own form.
        for (c, off) in [(oc, true), (dc, false)] {
            let Some(c) = c else { continue };
            let unit = 0.03 * (staff::side_quality(&w.coaches[c], off) - 13.5) / 2.0
                + 0.04 * key_normal(w.options.seed as u64 ^ year as u64 ^ 0x554E, c as u64);
            w.coaches[c].season_reputation(0.5 * vs + unit, 0.0, false);
        }
    }
    // The playoff field: division winners first, then wild cards, by record.
    let per_conf = (w.rules.nfl_playoff_teams / 2) as usize;
    let mut alive: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
    let mut seed = vec![None; n];
    for c in 0..2 {
        let mut winners: Vec<usize> = (0..4)
            .filter_map(|dv| {
                (0..n)
                    .filter(|&t| s.div[t] == c * 4 + dv)
                    .max_by(|&a, &b| record_key(&s, a).total_cmp(&record_key(&s, b)))
            })
            .collect();
        winners.sort_by(|&a, &b| record_key(&s, b).total_cmp(&record_key(&s, a)));
        let mut rest: Vec<usize> = (0..n).filter(|&t| s.conf[t] == c && !winners.contains(&t)).collect();
        rest.sort_by(|&a, &b| record_key(&s, b).total_cmp(&record_key(&s, a)));
        let field: Vec<usize> = winners.into_iter().chain(rest).take(per_conf).collect();
        for (k, &t) in field.iter().enumerate() {
            seed[t] = Some(k + 1);
        }
        alive[c] = field;
    }
    let s = w.nfl_season.as_mut().unwrap();
    s.alive = alive;
    s.seed = seed;
    s.booked = true;
    w.log.nfl_huge_by_team = s.huge.clone();
    super::pro::honors::season_honors(w, d);
    super::pro::black_monday(w);
    post(
        w,
        d,
        Kind::Coaching,
        Loudness::Big,
        "",
        "Black Monday: the day after the regular season, when head coaches are fired.",
    );
    staff::carousel(w, staff::Scope::NflFirings);
}

/// A playoff round: 1 wild card, 2 divisional, 3 conference title games, 4
/// the Super Bowl. The higher seed hosts; the Super Bowl is neutral.
fn play_round(w: &mut World, round: u32, d: Date) {
    let Some(s) = w.nfl_season.as_ref() else { return };
    if !s.booked || round != 4 {
        return;
    }
    let seed = w.options.seed as u64 ^ (w.year as u64) << 24 ^ (round as u64) << 4 ^ 0x504F;
    let mut s = s.clone();
    let (a, b) = (s.alive[0].first().copied(), s.alive[1].first().copied());
    let (Some(a), Some(b)) = (a, b) else { return };
    let p = win_prob(&s, a, b, 0.0);
    let at = Moment::at(d, 18, 30);
    let game = engine_games(w, &[Fixture { home: a, away: b, at, neutral: true, loud: 0.0, seed: seed ^ 0x5B }]);
    let a_wins = match game.first().and_then(|g| g.as_ref()) {
        Some((r, sides)) => {
            pro_games::record(w, [a, b], r, sides, [s.z[b], s.z[a]]);
            let [x, y] = r.stats.score;
            let won = if x == y { key_uniform(seed, (a as u64) << 8 ^ b as u64) < p } else { x > y };
            let (side, champ) = if won { (0, a) } else { (1, b) };
            super::pro::honors::super_bowl_mvp(w, r, &sides[side], side, champ, d);
            won
        }
        None => key_uniform(seed, (a as u64) << 8 ^ b as u64) < p,
    };
    let (champ, loser) = if a_wins { (a, b) } else { (b, a) };
    s.out_round[champ] = 5;
    s.out_round[loser] = 4;
    s.champion = Some(champ);
    s.runner_up = Some(loser);
    s.final_order = Some(order_from(&s));
    for r in w.nfl_seasons.iter_mut().rev().take_while(|r| r.year == s.year) {
        r.round = s.out_round.get(r.team).copied().unwrap_or(0);
    }
    *w.nfl_season.as_mut().unwrap() = s;
    w.log.nfl_games += 1;
    super_bowl_book(w, champ, loser, d);
}

/// A playoff round's games and their days (T1.2): wild card weekend runs
/// Saturday to Monday (six games from 2020), the divisional round Saturday
/// and Sunday, the conference title games on Sunday. The top seeds rest
/// when the field isn't a power of two.
fn start_round(w: &mut World, round: u32, d: Date) {
    let Some(s) = w.nfl_season.as_mut() else { return };
    if !s.booked {
        return;
    }
    let mut pairs: Vec<(usize, usize, usize)> = Vec::new();
    for c in 0..2 {
        let field = s.alive[c].clone();
        let n = field.len();
        let byes = if round == 1 { n.next_power_of_two() - n } else { 0 };
        let byes = if round == 1 && n == 8 { 0 } else { byes.min(n) };
        let (stay, play) = field.split_at(byes);
        s.round_next[c] = stay.to_vec();
        let m = play.len();
        for k in 0..m / 2 {
            pairs.push((c, play[k], play[m - 1 - k]));
        }
    }
    // Kickoffs in order: Saturday afternoon and night, Sunday, Monday night.
    let times: Vec<Moment> = match (round, pairs.len()) {
        (3, _) => vec![Moment::at(d, 15, 0), Moment::at(d, 18, 30)],
        (2, _) => vec![
            Moment::at(d, 16, 30),
            Moment::at(d, 20, 15),
            Moment::at(d.add(1), 15, 0),
            Moment::at(d.add(1), 18, 30),
        ],
        (_, 6) => vec![
            Moment::at(d, 16, 30),
            Moment::at(d, 20, 15),
            Moment::at(d.add(1), 13, 0),
            Moment::at(d.add(1), 16, 30),
            Moment::at(d.add(1), 20, 15),
            Moment::at(d.add(2), 20, 15),
        ],
        (_, n) => (0..n)
            .map(|k| match k {
                0 => Moment::at(d, 16, 30),
                1 => Moment::at(d, 20, 15),
                2 => Moment::at(d.add(1), 13, 0),
                3 => Moment::at(d.add(1), 16, 30),
                4 => Moment::at(d.add(1), 20, 15),
                5 => Moment::at(d.add(2), 20, 15),
                6 => Moment::at(d, 13, 0),
                _ => Moment::at(d.add(2), 16, 30),
            })
            .collect(),
    };
    s.round_games =
        pairs.iter().enumerate().map(|(k, &(c, hi, lo))| (round, c, hi, lo, times[k.min(times.len() - 1)])).collect();
    let names = ["", "Wild card weekend begins", "The divisional round begins", "The conference title games begin"];
    let text = format!("{}.", names[round as usize]);
    post(w, d, Kind::Game, Loudness::Huge, "", text);
}

/// The day's playoff games; when a round is done, the winners are reseeded.
fn play_round_games(w: &mut World, d: Date) {
    let Some(s) = w.nfl_season.as_ref() else { return };
    if s.round_games.iter().all(|g| g.4.date != d) {
        return;
    }
    let mut s = s.clone();
    let mut results = Vec::new();
    let mut left = Vec::new();
    let today: Vec<_> = s.round_games.iter().filter(|g| g.4.date == d).copied().collect();
    let fixtures: Vec<Fixture> = today
        .iter()
        .map(|&(round, c, hi, lo, at)| {
            let seed = w.options.seed as u64 ^ (w.year as u64) << 24 ^ (round as u64) << 4 ^ 0x504F ^ c as u64;
            Fixture { home: hi, away: lo, at, neutral: false, loud: 0.1, seed: seed ^ (hi as u64) << 8 ^ lo as u64 }
        })
        .collect();
    let outcomes = engine_games(w, &fixtures);
    let mut engine: Vec<(usize, usize, bool)> = Vec::new();
    for (g, o) in today.iter().zip(&outcomes) {
        if let Some((r, sides)) = o {
            pro_games::record(w, [g.2, g.3], r, sides, [s.z[g.3], s.z[g.2]]);
            let [x, y] = r.stats.score;
            if x != y {
                engine.push((g.2, g.3, x > y));
            }
        }
    }
    for g in std::mem::take(&mut s.round_games) {
        let (round, c, hi, lo, at) = g;
        if at.date != d {
            left.push(g);
            continue;
        }
        let seed = w.options.seed as u64 ^ (w.year as u64) << 24 ^ (round as u64) << 4 ^ 0x504F;
        let p = win_prob(&s, hi, lo, HOME);
        let hi_wins = match engine.iter().find(|e| e.0 == hi && e.1 == lo) {
            Some(e) => e.2,
            None => key_uniform(seed ^ c as u64, (hi as u64) << 8 ^ lo as u64) < p,
        };
        let (wn, l) = if hi_wins { (hi, lo) } else { (lo, hi) };
        s.out_round[l] = round;
        s.round_next[c].push(wn);
        results.push((round, wn, l, at));
    }
    s.round_games = left;
    if s.round_games.is_empty() {
        for c in 0..2 {
            // Reseed: the best remaining seed plays the lowest.
            let mut next = std::mem::take(&mut s.round_next[c]);
            next.sort_by_key(|&t| s.seed[t].unwrap_or(99));
            s.alive[c] = next;
        }
    }
    w.log.nfl_games += results.len() as u32;
    *w.nfl_season.as_mut().unwrap() = s;
    let names = ["", "Wild card", "Divisional round", "Conference title game"];
    for (round, wn, l, at) in results {
        let text = format!("{}: the {} beat the {}.", names[round as usize], w.nfl[wn].name, w.nfl[l].name);
        w.calendar.post(
            Event::new(at.date, Level::Nfl, Kind::Game, Loudness::Huge, text)
                .team(wn)
                .at(at)
                .club_link(wn, w.nfl[wn].name)
                .club_link(l, w.nfl[l].name),
        );
    }
}

/// The draft order the season sets: teams out of the playoffs by record,
/// worst first, then playoff teams by the round they went out in, the Super
/// Bowl loser next to last and the champion last.
fn order_from(s: &NflSeason) -> Vec<usize> {
    let n = s.wins.len();
    let mut v: Vec<usize> = (0..n).filter(|&t| s.active[t]).collect();
    v.sort_by(|&a, &b| {
        let ra = (s.out_round[a], record_key(s, a));
        let rb = (s.out_round[b], record_key(s, b));
        ra.0.cmp(&rb.0).then(ra.1.total_cmp(&rb.1))
    });
    v
}

/// The Super Bowl's book: titles, reputations and the news.
fn super_bowl_book(w: &mut World, champ: usize, loser: usize, d: Date) {
    let year = w.year;
    for (t, title) in [(champ, TitleKind::National), (loser, TitleKind::Conference)] {
        let (hc, oc, dc) = (w.nfl[t].head_coach, w.nfl[t].oc, w.nfl[t].dc);
        if let Some(h) = hc {
            let coach = &mut w.coaches[h];
            coach.titles += 1;
            coach.career_titles += 1;
            coach.title_reputation(title, true, 1.0);
        }
        for c in [oc, dc].into_iter().flatten() {
            w.coaches[c].title_reputation(title, false, 1.0);
        }
    }
    w.nfl_champions.push((year, champ));
    let name = w.nfl[champ].name;
    let text = format!("The {name} win the Super Bowl over the {}.", w.nfl[loser].name);
    w.calendar.post(
        Event::new(d, Level::Nfl, Kind::Game, Loudness::Huge, text)
            .team(champ)
            .memorable()
            .club_link(champ, name)
            .club_link(loser, w.nfl[loser].name),
    );
    if w.recording {
        let head = w.nfl[champ].head_coach;
        let coach = head.map_or(String::new(), |h| format!(" under head coach {}", w.coaches[h].name));
        let mut item = NewsItem {
            date: Some(d),
            year,
            kind: NewsKind::NationalTitle,
            channel: Channel::News,
            program: None,
            text: format!("The {name} win the Super Bowl{coach}."),
            refs: Vec::new(),
        }
        .link(Link::Club(champ as u32), name);
        if let Some(h) = head {
            item = item.link(Link::Coach(h as u32), &w.coaches[h].name.clone());
        }
        w.news.push(item);
    }
}

/// The draft order for this April, from last season (T2); None before the
/// first season has been played on the clock.
pub(crate) fn draft_order(w: &World) -> Option<Vec<usize>> {
    w.nfl_season.as_ref().and_then(|s| s.last_order.clone())
}

impl World {
    /// Last season's Super Bowl champion, if the clock has played one.
    pub(crate) fn prev_champion(&self) -> Option<usize> {
        self.nfl_champions.last().filter(|c| c.0 == self.year - 1).map(|c| c.1)
    }
}

/// The training blocks of the year at every level (topic 7): when each kind
/// of training can happen. How training works is section 6b.
pub(crate) fn training_blocks(w: &mut World, d: Date, y: &YearDates) {
    let blocks: [(Date, Level, &str, &str); 9] = [
        (
            y.spring_semester,
            Level::College,
            "NCAA: winter conditioning",
            "College winter conditioning: strength and speed work, no football.",
        ),
        (
            y.spring_practice.0,
            Level::College,
            "NCAA: 15 spring practices",
            "College spring practice: 15 sessions over about five weeks.",
        ),
        (
            y.summer_workouts.0,
            Level::College,
            "NCAA: 8 hours a week",
            "College summer: 8 hours a week of required workouts, with film.",
        ),
        (y.fall_camp, Level::College, "NCAA: 29 camp practices", "College fall camp: 29 practices before the opener."),
        (Date::ymd(y.season, 1, 15).max(y.start), Level::HighSchool, "", "High school offseason lifting and track."),
        (Date::ymd(y.season, 5, 1), Level::HighSchool, "", "High school spring ball, in the states that allow it."),
        (
            Date::ymd(y.season, 6, 15),
            Level::HighSchool,
            "",
            "High school summer: 7-on-7 tournaments, camps, and each state's dead period.",
        ),
        (Date::ymd(y.season, 7, 27), Level::HighSchool, "", "High school practice opens in the earliest states."),
        (
            y.cfb_weeks[0].add(-5),
            Level::College,
            "NCAA: 20-hour rule",
            "College in-season: 20 countable hours a week, one day off.",
        ),
    ];
    for (date, level, rule, text) in blocks {
        if d == date {
            w.calendar.post(Event::new(d, level, Kind::Window, Loudness::Small, text).rule(rule));
        }
    }
    if d == y.nfl_kickoff {
        post(
            w,
            d,
            Kind::Window,
            Loudness::Small,
            "CBA: in-season practice",
            "NFL season: practice Wednesday to Friday, 14 padded practices in all.",
        );
    }
}

/// A one-line playoff summary for trials: (seed, team) per conference.
pub(crate) fn playoff_field(s: &NflSeason) -> Vec<(usize, usize)> {
    let mut v: Vec<(usize, usize)> = s.seed.iter().enumerate().filter_map(|(t, k)| k.map(|k| (k, t))).collect();
    v.sort();
    v
}

/// Your club's game today, as the day builds its fixture (section 9): the
/// regular season, a playoff round or the Super Bowl.
pub(crate) fn seat_fixture(w: &World, t: usize) -> Option<Fixture> {
    let d = w.today;
    let s = w.nfl_season.as_ref()?;
    if !s.started {
        return None;
    }
    for (k, week) in s.slots.iter().enumerate() {
        for (g, x) in week.iter().enumerate() {
            let (h, a) = s.weeks[k][g];
            if x.at.date == d && !x.played && (h == t || a == t) {
                let seed = w.options.seed as u64 ^ (w.year as u64) << 24 ^ (k as u64) << 10 ^ 0x4E47;
                let loud = if x.window.prime() { 0.04 } else { 0.0 };
                return Some(Fixture {
                    home: h,
                    away: a,
                    at: x.at,
                    neutral: x.site.is_some(),
                    loud,
                    seed: seed ^ (h as u64) << 8 ^ a as u64,
                });
            }
        }
    }
    for &(round, c, hi, lo, at) in &s.round_games {
        if at.date == d && (hi == t || lo == t) {
            let seed = w.options.seed as u64 ^ (w.year as u64) << 24 ^ (round as u64) << 4 ^ 0x504F ^ c as u64;
            return Some(Fixture {
                home: hi,
                away: lo,
                at,
                neutral: false,
                loud: 0.1,
                seed: seed ^ (hi as u64) << 8 ^ lo as u64,
            });
        }
    }
    if d == w.dates.super_bowl && s.booked {
        let (a, b) = (s.alive[0].first().copied()?, s.alive[1].first().copied()?);
        if a == t || b == t {
            let seed = w.options.seed as u64 ^ (w.year as u64) << 24 ^ 4u64 << 4 ^ 0x504F;
            return Some(Fixture {
                home: a,
                away: b,
                at: Moment::at(d, 18, 30),
                neutral: true,
                loud: 0.0,
                seed: seed ^ 0x5B,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real 2026 schedule (T2.2): 272 games over 18 weeks, 17 for every
    /// team, all inside the season, Detroit and Dallas home on Thanksgiving,
    /// three games on Christmas, nine abroad.
    #[test]
    fn the_real_2026_schedule_reads_whole() {
        let y = crate::calendar::YearDates::new(2026);
        let names: Vec<&str> = super::super::data::NFL_TEAMS.iter().map(|r| r.name.as_str()).collect();
        let mut games = vec![0u32; names.len()];
        let (mut rows, mut abroad, mut christmas) = (0, 0, 0);
        let mut thanksgiving_hosts = Vec::new();
        for line in SCHEDULE_2026.lines().skip(1).filter(|l| !l.trim().is_empty()) {
            let f: Vec<&str> = line.split(',').collect();
            let week: usize = f[0].parse().unwrap();
            assert!((1..=18).contains(&week), "{line}");
            let mut ymd = f[1].split('-').map(|x| x.parse::<i32>().unwrap());
            let date = Date::ymd(ymd.next().unwrap(), ymd.next().unwrap() as u32, ymd.next().unwrap() as u32);
            assert!(date >= y.nfl_kickoff.add(-1) && date <= y.nfl_weeks[17].add(1), "{line}");
            for team in [f[3], f[4]] {
                let t = names.iter().position(|&n| n == team).unwrap_or_else(|| panic!("unknown team {team}"));
                games[t] += 1;
            }
            rows += 1;
            abroad += !f[5].trim().is_empty() as u32;
            christmas += (date == y.christmas) as u32;
            if date == y.thanksgiving {
                thanksgiving_hosts.push(f[4].to_string());
            }
        }
        assert_eq!(rows, 272);
        assert!(games.iter().all(|&g| g == 17), "{games:?}");
        assert_eq!(abroad, 9);
        assert_eq!(christmas, 3);
        assert!(thanksgiving_hosts.iter().any(|h| h == "Detroit Lions"));
        assert!(thanksgiving_hosts.iter().any(|h| h == "Dallas Cowboys"));
    }

    /// Nine games abroad a season from 2026, none in 2020, the Bills'
    /// Toronto games 2008-2013 and two London games in 2021.
    #[test]
    fn international_games_follow_the_eras() {
        assert_eq!(international_sites(2020).len(), 0);
        assert_eq!(international_sites(2025).len(), 7);
        assert_eq!(international_sites(2027).len(), 9);
        assert_eq!(international_sites(2008).len(), 2, "London and Toronto");
        assert_eq!(international_sites(2013).len(), 3);
        assert_eq!(international_sites(2014).len(), 3);
        assert_eq!(international_sites(2021).len(), 2);
        assert!(international_sites(2012).iter().all(|x| !x.0.contains("Tottenham")), "opened 2019");
    }

    /// A league with no players and its formula schedule for `season`.
    fn formula_season(season: i32) -> World {
        formula_season_seeded(season, crate::world::WorldOptions::default().seed)
    }

    fn formula_season_seeded(season: i32, seed: u32) -> World {
        let options = crate::world::WorldOptions {
            seed,
            scale: 0.002,
            history_years: 0,
            start_year: season + crate::world::SPIN_UP_YEARS as i32,
            names: crate::world::names::NameMode::Fictional,
            ..Default::default()
        };
        let mut w = World::empty(options);
        assert_eq!(w.year, season);
        w.nfl_season = Some(new_season(&w));
        let d = w.dates.schedule_release;
        build_schedule(&mut w, d);
        w
    }

    fn games_on(w: &World, d: Date) -> Vec<&Slot> {
        w.nfl_season.as_ref().unwrap().slots.iter().flatten().filter(|x| x.at.date == d).collect()
    }

    /// The Bills' Toronto game is a Bills home game, and it counts abroad.
    /// It is placed before the London games, so a London game never takes
    /// its opponent first (world 2028 missed it in 2 of 6 seasons).
    #[test]
    fn the_bills_host_in_toronto() {
        for season in 2008..=2013 {
            for seed in [2026, 2027, 2028] {
                let w = formula_season_seeded(season, seed);
                let s = w.nfl_season.as_ref().unwrap();
                let buf = w.nfl.iter().position(|t| t.name == "Buffalo Bills");
                let abroad: Vec<(usize, &Slot)> = s
                    .weeks
                    .iter()
                    .zip(&s.slots)
                    .flat_map(|(g, x)| g.iter().map(|g| g.0).zip(x))
                    .filter(|(_, x)| x.site.is_some())
                    .collect();
                assert_eq!(abroad.len(), international_sites(season).len(), "season {season}, seed {seed}");
                assert!(
                    abroad.iter().any(|(home, x)| x.site.as_deref() == Some(TORONTO) && Some(*home) == buf),
                    "season {season}, seed {seed}"
                );
            }
        }
    }

    /// A Sunday Christmas (real 2022: 11 games on Dec 24 and 3 on Dec 25)
    /// plays the slate on Christmas Eve and keeps only the Christmas windows.
    #[test]
    fn a_sunday_christmas_moves_the_slate_to_christmas_eve() {
        for (season, christmas) in [(2011, 1), (2016, 2), (2022, 3)] {
            let w = formula_season(season);
            let day = games_on(&w, Date::ymd(season, 12, 25));
            let eve = games_on(&w, Date::ymd(season, 12, 24));
            assert_eq!(day.len(), christmas, "{season}");
            assert!(day.iter().all(|x| x.window == Window::Christmas && x.at.minute >= 13 * 60));
            assert!(eve.len() >= 10, "{season}: {} games on Christmas Eve", eve.len());
        }
    }

    /// On a Monday Christmas the night game is Monday Night Football: one
    /// game at night, not two.
    #[test]
    fn a_monday_christmas_has_one_night_game() {
        for (season, christmas) in [(2017, 2), (2023, 3)] {
            let w = formula_season(season);
            let day = games_on(&w, Date::ymd(season, 12, 25));
            assert_eq!(day.len(), christmas, "{season}");
            assert_eq!(day.iter().filter(|x| x.at.minute >= 19 * 60).count(), 1, "{season}");
        }
    }

    /// From 2014 flex scheduling allows at most two flexes in weeks 5-10.
    #[test]
    fn flex_scheduling_keeps_the_early_season_limit() {
        let mut w = formula_season(2015);
        let y = w.dates.clone();
        let s = w.nfl_season.as_mut().unwrap();
        s.wins = (0..s.wins.len()).map(|t| (t * 7 % 11) as u32).collect();
        // Ten games in: the teams at three wins or fewer are out of the race,
        // so their night games can be flexed out (part D 35).
        s.losses = s.wins.iter().map(|&x| 10 - x).collect();
        let mut d = y.nfl_weeks[0];
        while d <= *y.nfl_weeks.last().unwrap() {
            flex(&mut w, d, &y);
            d = d.add(1);
        }
        let flexed = |k: usize| {
            w.calendar.on(y.nfl_weeks[k].add(-12)).iter().filter(|e| e.rule == "NFL: flexible scheduling").count()
        };
        let early: usize = (4..=9).map(flexed).sum();
        let late: usize = (10..y.nfl_weeks.len() - 1).map(flexed).sum();
        assert_eq!(early, 2, "the limit binds in this league");
        assert_eq!(w.nfl_season.as_ref().unwrap().early_flexes, 2);
        // Later weeks have no limit, but only a night game that lost its
        // stakes is flexed out (part D 35: real one to four a season).
        assert!(late >= 1, "later weeks have no limit ({late})");
    }

    /// The season kicks off on the opener's own day, not the day before.
    #[test]
    fn the_season_kicks_off_on_the_openers_day() {
        let mut w = formula_season(2019);
        let opener = w.dates.nfl_kickoff;
        for d in [opener.add(-2), opener.add(-1), opener] {
            day(&mut w, d);
        }
        let kicks = |d: Date| w.calendar.on(d).iter().any(|e| e.text == "The NFL season kicks off.");
        assert!(!kicks(opener.add(-1)) && kicks(opener));
    }
}
