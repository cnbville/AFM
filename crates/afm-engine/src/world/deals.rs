//! Non-conference games under contract (update 2, lane 6, SC13 and SC14).
//! Real FBS schools book their non-conference dates years ahead: a Power 4
//! home-and-home eight to twelve years out, a neutral-site opener three to
//! six, the home games they buy from a Group of 5 or an FCS school two to
//! six, and the last open date a year or two before. Each February the
//! world's athletic directors fill their open dates up to how full a season
//! that far off usually is, and the schedule builder plays the contracts in
//! their season (`schedule::build`). A one-off home game carries the
//! guarantee its contract names; a neutral-site game pays each side.
//!
//! Proposed, not confirmed: how full each season is by how far off it is
//! (`share`), how often each kind of game is booked, and the neutral sites'
//! payouts by year.

use super::World;
use super::data::Division;
use super::schedule::{Game, GameKind};
use super::season::key_uniform;
use crate::calendar::data::RIVALRIES;
use crate::calendar::rules::conference_rules;

/// How far ahead the books run.
pub const HORIZON: i32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DealKind {
    /// One leg of a home-and-home series.
    HomeAndHome,
    /// A home game the host buys: no return trip, a guarantee to the visitor.
    OneOff,
    /// A neutral-site game, each side paid by the game's organisers.
    Neutral,
}

/// A non-conference game under contract.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Deal {
    pub year: i32,
    /// The host (the designated home side at a neutral site).
    pub home: usize,
    pub away: usize,
    pub kind: DealKind,
    /// A home-and-home's leg (1 or 2); 0 for the other kinds.
    pub leg: u8,
    /// The year it was signed.
    pub signed: i32,
    /// Dollars: the guarantee the host pays (a one-off), each side's payout
    /// (a neutral site); 0 for a home-and-home.
    pub fee: f32,
    /// A neutral site, by index into `SITES`.
    pub site: Option<u8>,
}

/// A neutral site for an opening-weekend game.
pub struct Site {
    pub stadium: &'static str,
    pub town: &'static str,
    pub state: &'static str,
    pub indoor: bool,
    /// The first season it hosts kickoff games.
    pub first: i32,
}

/// The kickoff games' stadiums (2025): Atlanta from 2008 (the Georgia Dome
/// through 2016), Arlington from 2009, Houston from 2013, Charlotte from
/// 2015, Orlando from 2016, Las Vegas from 2021.
pub const SITES: &[Site] = &[
    Site { stadium: "Mercedes-Benz Stadium", town: "Atlanta", state: "GA", indoor: true, first: 2008 },
    Site { stadium: "AT&T Stadium", town: "Arlington", state: "TX", indoor: true, first: 2009 },
    Site { stadium: "NRG Stadium", town: "Houston", state: "TX", indoor: true, first: 2013 },
    Site { stadium: "Bank of America Stadium", town: "Charlotte", state: "NC", indoor: false, first: 2015 },
    Site { stadium: "Camping World Stadium", town: "Orlando", state: "FL", indoor: false, first: 2016 },
    Site { stadium: "Allegiant Stadium", town: "Las Vegas", state: "NV", indoor: true, first: 2021 },
];

impl Site {
    /// The stadium's name in a season (Atlanta's dome before 2017).
    pub fn stadium_in(&self, year: i32) -> &'static str {
        if self.town == "Atlanta" && year < 2017 { "Georgia Dome" } else { self.stadium }
    }
}

/// How full a season's non-conference dates are, by how many years off it
/// is (Power 4 schedules on the FBSchedules lists: next season nearly full,
/// half full six years out, a quarter ten years out).
fn share(h: i32) -> f64 {
    match h {
        ..=1 => 0.95,
        2 => 0.9,
        3 => 0.82,
        4 => 0.74,
        5 => 0.66,
        6 => 0.56,
        7 => 0.46,
        8 => 0.38,
        9 => 0.3,
        _ => 0.24,
    }
}

/// Each side's payout at a neutral-site kickoff game: about $1.6M in 2008,
/// $2.4M in 2014, $5M by 2024, then 3% a year.
pub fn neutral_payout(year: i32) -> f64 {
    let pts = [(1990, 0.5e6), (2008, 1.6e6), (2014, 2.4e6), (2024, 5.0e6)];
    if year >= 2024 {
        return 5.0e6 * 1.03f64.powi(year - 2024);
    }
    if year <= pts[0].0 {
        return pts[0].1;
    }
    let k = pts.iter().position(|p| p.0 >= year).unwrap_or(pts.len() - 1);
    let (a, b) = (pts[k - 1], pts[k]);
    a.1 + (b.1 - a.1) * (year - a.0) as f64 / (b.0 - a.0) as f64
}

fn independent(conf: &str) -> bool {
    conf.contains("Independent")
}

/// A program's conference games in a season, as the schedule builder plays
/// them: each FBS conference's own count, eight below FBS, one fewer than
/// the members when the conference is small.
pub fn conference_games(w: &World, p: usize, season: i32) -> u32 {
    let prog = &w.programs[p];
    if independent(&prog.conference) {
        return 0;
    }
    let n = w
        .programs
        .iter()
        .filter(|q| q.division == prog.division && q.conference == prog.conference && q.fields_games(season))
        .count() as u32;
    if n < 2 {
        return 0;
    }
    let g = if prog.division == Division::FBS { conference_rules(&prog.conference, season).0 } else { (n - 1).min(8) };
    g.min(n - 1)
}

/// Whether two programs' game is a listed rivalry played in its own slot.
fn rivals(w: &World, a: usize, b: usize) -> bool {
    let (x, y) = (w.programs[a].real_name.as_str(), w.programs[b].real_name.as_str());
    RIVALRIES.iter().any(|r| (r.a == x && r.b == y) || (r.a == y && r.b == x))
}

/// A program's rivalry games outside its conference in a season (Clemson
/// and South Carolina, Florida and Florida State): they take a date.
fn outside_rivalries(w: &World, p: usize, season: i32) -> u32 {
    let me = &w.programs[p];
    RIVALRIES
        .iter()
        .filter(|r| r.a == me.real_name || r.b == me.real_name)
        .filter(|r| {
            let other = if r.a == me.real_name { r.b } else { r.a };
            let Some(q) = w.programs.iter().find(|q| q.real_name == other) else { return false };
            q.division == me.division
                && q.fields_games(season)
                && (q.conference != me.conference || independent(&me.conference))
                && super::data::rivalry_past(r.a, r.b).is_none_or(|h| h.played(season))
        })
        .count() as u32
}

/// The non-conference dates a program has in a season.
pub fn slots(w: &World, p: usize, season: i32) -> u32 {
    let prog = &w.programs[p];
    super::schedule::games_target(prog.division, season)
        .saturating_sub(conference_games(w, p, season) + outside_rivalries(w, p, season))
}

/// Deals that play in a season.
pub fn for_season(w: &World, season: i32) -> impl Iterator<Item = &Deal> {
    w.deals.iter().filter(move |d| d.year == season)
}

/// A program's deals from a season on, soonest first.
pub fn of_program(w: &World, p: usize, from: i32) -> Vec<&Deal> {
    let mut v: Vec<&Deal> = w.deals.iter().filter(|d| d.year >= from && (d.home == p || d.away == p)).collect();
    v.sort_by_key(|d| (d.year, d.home != p));
    v
}

/// The deal behind a game, if one was booked.
pub fn find(w: &World, season: i32, home: usize, away: usize) -> Option<&Deal> {
    w.deals.iter().find(|d| d.year == season && d.home == home && d.away == away)
}

/// What a game paid (SC13), as the books and the schedule page both read it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Money {
    /// The host paid the visitor this guarantee.
    Guarantee(f64),
    /// The game's organisers paid each side this.
    Neutral(f64),
}

/// The money in a college game (`k`, its place in the season's list): its
/// contract's, else the guarantee a bigger program pays a smaller one it
/// hosts outside its conference (a game the schedule builder filled late).
pub fn money(w: &World, season: i32, g: &Game, k: usize) -> Option<Money> {
    use super::finance::college::tier;
    if g.kind != GameKind::Regular || g.conference {
        return None;
    }
    // A played game keeps what it paid, whatever the conferences are now.
    if let Some((kind, x)) = g.paid {
        return match kind {
            1 => Some(Money::Guarantee(x as f64)),
            2 => Some(Money::Neutral(x as f64)),
            _ => None,
        };
    }
    if let Some(d) = find(w, season, g.home, g.away) {
        return match d.kind {
            DealKind::OneOff => Some(Money::Guarantee(d.fee as f64)),
            DealKind::Neutral => Some(Money::Neutral(d.fee as f64)),
            DealKind::HomeAndHome => None,
        };
    }
    if g.site.is_some() || g.home >= w.programs.len() || g.away >= w.programs.len() {
        return None;
    }
    let (th, ta) = (tier(w, g.home, season), tier(w, g.away, season));
    (ta > th).then(|| {
        let draw = super::finance::draw(w, 0x6A4A, k as u64, season);
        Money::Guarantee(super::finance::data::guarantee(th, ta, season) * (0.8 + 0.4 * draw))
    })
}

/// The non-conference games a program has under contract in a season: the
/// world's and the ones you booked (a neutral-site booking of yours is a
/// contract too, counted once).
pub(crate) fn booked(w: &World, p: usize, season: i32) -> u32 {
    let own = w.deals.iter().filter(|d| d.year == season && (d.home == p || d.away == p)).count();
    let yours = w
        .legacy
        .college
        .booked
        .iter()
        .filter(|b| b.year == season && (b.program == p || b.opp == p) && !met(w, b.program, b.opp, season))
        .count();
    (own + yours) as u32
}

/// A date held for your own program: the season four years out stays open
/// for the athletic director's March call to you (`legacy::college`,
/// `booking_letter`) until you book it.
fn held(w: &World, p: usize, season: i32) -> u32 {
    let yours = matches!(w.seat_team(), Some((crate::calendar::Level::College, q)) if q == p);
    let asked = w.legacy.college.booked.iter().any(|b| b.program == p && b.year == season);
    (yours && season == w.year + 4 && !asked) as u32
}

/// Whether a program still has an open non-conference date in a season.
pub(crate) fn open(w: &World, p: usize, season: i32) -> bool {
    booked(w, p, season) + held(w, p, season) < slots(w, p, season)
}

/// Whether two programs have a contract in a season.
pub(crate) fn met(w: &World, a: usize, b: usize, season: i32) -> bool {
    w.deals.iter().any(|d| d.year == season && ((d.home == a && d.away == b) || (d.home == b && d.away == a)))
}

/// FBS games an FCS school has under contract in a season.
fn fbs_trips(w: &World, q: usize, season: i32) -> u32 {
    w.deals.iter().filter(|d| d.year == season && d.away == q).count() as u32
}

/// Fills every FBS program's open dates for the seasons ahead (each
/// February, once the new season's schedule is built), and lets the
/// contracts go once their season is a year gone (last season's page still
/// reads its fees).
pub(crate) fn top_up(w: &mut World) {
    let now = w.year;
    w.deals.retain(|d| d.year >= now - 1);
    trim(w);
    for y in now + 1..=now + HORIZON {
        let h = y - now;
        let mut order: Vec<usize> =
            w.programs.iter().filter(|p| p.division == Division::FBS && p.fields_games(y)).map(|p| p.id).collect();
        // Power programs set their schedules first; the rest fill around them.
        let key = |p: usize| key_uniform(w.options.seed as u64 ^ 0xDEA1 ^ (y as u64) << 24, p as u64);
        order.sort_by(|&a, &b| {
            super::finance::college::tier(w, a, y)
                .cmp(&super::finance::college::tier(w, b, y))
                .then(key(a).total_cmp(&key(b)))
        });
        for p in order {
            let room = slots(w, p, y).saturating_sub(held(w, p, y));
            let want = ((slots(w, p, y) as f64 * share(h)).round() as u32).min(room);
            let mut tries = 0;
            while booked(w, p, y) < want && tries < 6 {
                tries += 1;
                book_one(w, p, y, h, tries);
            }
        }
    }
}

/// A program whose conference changed since it signed (a move to a league
/// with more games, or an independent joining one) has fewer open dates:
/// the newest contracts past them are bought out, both legs of a
/// home-and-home, as real schools do when they realign.
fn trim(w: &mut World) {
    let now = w.year;
    for y in now + 1..=now + HORIZON + 1 {
        for p in 0..w.programs.len() {
            if w.programs[p].division != Division::FBS {
                continue;
            }
            while booked(w, p, y) > slots(w, p, y) {
                let Some(k) = (0..w.deals.len())
                    .filter(|&k| w.deals[k].year == y && (w.deals[k].home == p || w.deals[k].away == p))
                    .max_by_key(|&k| (w.deals[k].signed, k))
                else {
                    break;
                };
                let d = w.deals.remove(k);
                if d.kind == DealKind::HomeAndHome {
                    let other = if d.leg == 1 { d.year + 1 } else { d.year - 1 };
                    w.deals.retain(|x| {
                        !(x.kind == DealKind::HomeAndHome
                            && x.year == other
                            && ((x.home == d.home && x.away == d.away) || (x.home == d.away && x.away == d.home)))
                    });
                }
            }
        }
    }
}

/// Books one more non-conference game for a program in a season, if a
/// partner with an open date fits.
fn book_one(w: &mut World, p: usize, y: i32, h: i32, salt: u32) -> bool {
    use super::finance::college::tier;
    let seed = w.options.seed as u64 ^ 0xB00C ^ (y as u64) << 24 ^ (salt as u64) << 52;
    let roll = |what: u64| key_uniform(seed ^ what, p as u64);
    let me = tier(w, p, y);
    let has_fcs = w.deals.iter().any(|d| d.year == y && d.home == p && w.programs[d.away].division == Division::FCS);
    // The kinds a program of its tier books this far out, most likely first.
    let mut kinds: Vec<(DealKind, u8)> = Vec::new();
    if me == 0 {
        if !has_fcs && h <= 6 && roll(1) < 0.85 {
            kinds.push((DealKind::OneOff, 2));
        }
        if (2..=6).contains(&h) && y >= 2008 && roll(2) < 0.09 {
            kinds.push((DealKind::Neutral, 0));
        }
        if h <= 7 && roll(3) < 0.6 {
            kinds.push((DealKind::OneOff, 1));
        }
        if h >= 2 {
            kinds.push((DealKind::HomeAndHome, 0));
        }
        kinds.push((DealKind::OneOff, 1));
    } else {
        // A Group of 5 school: its FCS home game, a road game a Power 4
        // school buys, home-and-homes with its own kind.
        if !has_fcs && h <= 5 && roll(4) < 0.6 {
            kinds.push((DealKind::OneOff, 2));
        }
        if h <= 6 && roll(5) < 0.55 {
            kinds.push((DealKind::OneOff, ROAD));
        }
        kinds.push((DealKind::HomeAndHome, 1));
        kinds.push((DealKind::OneOff, ROAD));
    }
    for (kind, want) in kinds {
        if let Some(d) = partner(w, p, y, kind, want, seed) {
            let n = d.len();
            w.deals.extend(d);
            return n > 0;
        }
    }
    false
}

/// A one-off a program plays on the road at a Power 4 host that buys it.
const ROAD: u8 = 3;

/// The partner and the deal (two legs for a home-and-home). A one-off's
/// `want`: 2 an FCS visitor, 1 a Group of 5 visitor, `ROAD` a Power 4 host.
fn partner(w: &World, p: usize, y: i32, kind: DealKind, want: u8, seed: u64) -> Option<Vec<Deal>> {
    use super::finance::college::tier;
    let me = &w.programs[p];
    let free = |q: usize, s: i32| open(w, q, s);
    let mut cands: Vec<(f64, usize)> = w
        .programs
        .iter()
        .filter(|q| q.id != p && q.fields_games(y))
        .filter(|q| q.conference != me.conference || independent(&me.conference))
        .filter(|q| match kind {
            DealKind::OneOff if want == 2 => q.division == Division::FCS && fbs_trips(w, q.id, y) == 0,
            DealKind::OneOff if want == ROAD => q.division == Division::FBS && tier(w, q.id, y) == 0 && free(q.id, y),
            DealKind::OneOff => q.division == Division::FBS && tier(w, q.id, y) == 1 && free(q.id, y),
            DealKind::Neutral => q.division == Division::FBS && tier(w, q.id, y) == 0 && free(q.id, y),
            // Both legs need both programs' dates (the second a season on).
            DealKind::HomeAndHome => {
                q.division == Division::FBS
                    && tier(w, q.id, y) == want
                    && free(q.id, y)
                    && free(q.id, y + 1)
                    && free(p, y + 1)
                    && !met(w, p, q.id, y + 1)
            }
        })
        .filter(|q| !met(w, p, q.id, y) && !met(w, p, q.id, y - 1) && !rivals(w, p, q.id))
        .map(|q| {
            let miles = super::geo::miles(me.state, q.state);
            let noise = key_uniform(seed ^ 0x9A57, (q.id as u64) << 20 ^ p as u64);
            let score = match kind {
                // A neighbour for the buy games; the big names travel for a home-and-home.
                DealKind::OneOff => miles + 900.0 * noise,
                DealKind::Neutral => 1500.0 * noise + 800.0 * (me.prestige - q.prestige).abs(),
                DealKind::HomeAndHome => miles * 0.5 + 1600.0 * noise + 1200.0 * (me.prestige - q.prestige).abs(),
            };
            (score, q.id)
        })
        .collect();
    cands.sort_by(|a, b| a.0.total_cmp(&b.0));
    let &(_, q) = cands.first()?;
    // A contract fixes its sum when it is signed: the going rate that year.
    let signed = w.year;
    let noise = key_uniform(seed ^ 0xFEE, (q as u64) << 20 ^ p as u64);
    Some(match kind {
        DealKind::OneOff => {
            let (home, away) = if want == ROAD { (q, p) } else { (p, q) };
            let fee = super::finance::data::guarantee(tier(w, home, signed), tier(w, away, signed), signed)
                * (0.8 + 0.5 * noise);
            vec![Deal { year: y, home, away, kind, leg: 0, signed, fee: round_fee(fee), site: None }]
        }
        DealKind::Neutral => {
            vec![neutral(p, q, y, signed, noise)]
        }
        DealKind::HomeAndHome => {
            // Who hosts first: a coin flip.
            let (a, b) = if noise < 0.5 { (p, q) } else { (q, p) };
            vec![
                Deal { year: y, home: a, away: b, kind, leg: 1, signed, fee: 0.0, site: None },
                Deal { year: y + 1, home: b, away: a, kind, leg: 2, signed, fee: 0.0, site: None },
            ]
        }
    })
}

/// A neutral-site kickoff game signed in `signed` for season `y`: one of
/// the sites hosting by then, each side paid the going rate the year it
/// was signed.
pub(crate) fn neutral(home: usize, away: usize, y: i32, signed: i32, noise: f64) -> Deal {
    let sites: Vec<usize> = (0..SITES.len()).filter(|&k| SITES[k].first <= y).collect();
    let site = sites[(noise * sites.len() as f64) as usize % sites.len()];
    let fee = neutral_payout(signed) * (0.85 + 0.3 * noise);
    Deal { year: y, home, away, kind: DealKind::Neutral, leg: 0, signed, fee: round_fee(fee), site: Some(site as u8) }
}

/// Contracts name round sums: to $5,000.
fn round_fee(x: f64) -> f32 {
    ((x / 5_000.0).round() * 5_000.0) as f32
}

/// Fills the books once for a world from before them (a file built before
/// this build), so a new career sees the seasons ahead.
pub fn ensure(w: &mut World) {
    if w.deals.is_empty() && w.programs.iter().any(|p| p.division == Division::FBS) {
        top_up(w);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldOptions;

    #[test]
    fn the_books_fill_with_distance_and_respect_open_dates() {
        let mut w = World::empty(WorldOptions { scale: 0.02, ..Default::default() });
        top_up(&mut w);
        assert!(!w.deals.is_empty());
        let fbs: Vec<usize> = w.programs.iter().filter(|p| p.division == Division::FBS).map(|p| p.id).collect();
        let full = |y: i32| {
            let (b, s): (u32, u32) =
                fbs.iter().map(|&p| (booked(&w, p, y), slots(&w, p, y))).fold((0, 0), |a, x| (a.0 + x.0, a.1 + x.1));
            b as f64 / s.max(1) as f64
        };
        // Next season nearly full, ten years out about a quarter.
        let (near, far) = (full(w.year + 1), full(w.year + HORIZON));
        assert!(near > 0.75 && far < 0.5 && near > far, "{near} {far}");
        // No program is booked past its open dates, a home-and-home's
        // second leg included, and no pairing twice in a season.
        for &p in &fbs {
            for y in w.year + 1..=w.year + HORIZON + 1 {
                assert!(booked(&w, p, y) <= slots(&w, p, y), "{} {y}", w.programs[p].real_name);
                let mut opps: Vec<usize> = for_season(&w, y)
                    .filter(|d| d.home == p || d.away == p)
                    .map(|d| if d.home == p { d.away } else { d.home })
                    .collect();
                let n = opps.len();
                opps.sort();
                opps.dedup();
                assert_eq!(opps.len(), n, "{} meets a team twice in {y}", w.programs[p].real_name);
            }
        }
        // Buy games carry the going rate the year they are signed: an FCS visitor about
        // $400K-700K at a Power 4 host in the 2020s, a Group of 5 visitor
        // about $1-2M, less at a Group of 5 host.
        use crate::world::finance::college::tier;
        use crate::world::finance::data::guarantee;
        assert!((400_000.0..700_000.0).contains(&guarantee(0, 2, 2024)));
        assert!((1_000_000.0..2_000_000.0).contains(&guarantee(0, 1, 2024)));
        assert!((250_000.0..450_000.0).contains(&guarantee(1, 2, 2024)));
        let mut road = 0;
        for d in w.deals.iter().filter(|d| d.kind == DealKind::OneOff) {
            let rate = guarantee(tier(&w, d.home, d.signed), tier(&w, d.away, d.signed), d.signed);
            let fee = d.fee as f64;
            assert!(fee >= 0.8 * rate - 5_000.0 && fee <= 1.3 * rate + 5_000.0, "{fee} against {rate}");
            assert!(tier(&w, d.home, d.year) < tier(&w, d.away, d.year), "the bigger program hosts");
            road += (tier(&w, d.home, d.year) == 0 && tier(&w, d.away, d.year) == 1) as u32;
        }
        assert!(road > 0, "Power 4 schools buy Group of 5 games");
        // A home-and-home's legs swap the host.
        let leg1 = w.deals.iter().find(|d| d.leg == 1).unwrap();
        assert!(w.deals.iter().any(|d| d.leg == 2 && d.year == leg1.year + 1 && d.home == leg1.away));
    }

    #[test]
    fn neutral_payouts_grow_by_era() {
        assert!(neutral_payout(2008) < neutral_payout(2014));
        assert!((neutral_payout(2024) - 5.0e6).abs() < 1.0);
        assert!(neutral_payout(2030) > 5.0e6);
    }
}
