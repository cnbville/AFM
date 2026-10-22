//! Other pro leagues on their real dates (section 5 topic 1): where an
//! unsigned player keeps playing (and is seen again), NFL Europe's
//! allocations, and the International Player Pathway (from 2017). The
//! leagues are real; who goes and how many are Claude's, proposed, not
//! confirmed.

use super::market::{consensus, post};
use super::{ProCareer, Status};
use crate::calendar::{Date, Kind, Loudness};
use crate::positions::Position;
use crate::rng::{Rng, clamp};
use crate::world::World;
use crate::world::season::key_uniform;

/// A league's run: name, first and last season, and its months (signing
/// month, last month).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct League {
    #[serde(deserialize_with = "crate::save::leaked")]
    pub name: crate::save::Text,
    pub from: i32,
    pub to: i32,
    pub opens: u32,
    pub ends: u32,
}

pub const LEAGUES: [League; 12] = [
    League { name: "CFL", from: 1958, to: 9999, opens: 5, ends: 11 },
    League { name: "USFL", from: 1983, to: 1985, opens: 2, ends: 7 },
    League { name: "Arena Football League", from: 1987, to: 2008, opens: 3, ends: 8 },
    League { name: "Arena Football League", from: 2010, to: 2019, opens: 3, ends: 8 },
    League { name: "World League", from: 1991, to: 1992, opens: 2, ends: 6 },
    League { name: "NFL Europe", from: 1995, to: 2007, opens: 2, ends: 6 },
    League { name: "XFL", from: 2001, to: 2001, opens: 1, ends: 4 },
    League { name: "UFL", from: 2009, to: 2012, opens: 8, ends: 11 },
    League { name: "AAF", from: 2019, to: 2019, opens: 1, ends: 4 },
    League { name: "XFL", from: 2020, to: 2020, opens: 1, ends: 3 },
    League { name: "XFL", from: 2023, to: 2023, opens: 1, ends: 5 },
    League { name: "UFL", from: 2024, to: 9999, opens: 2, ends: 6 },
];

/// The USFL of 2022-23 (its own run).
pub const USFL_2022: League = League { name: "USFL", from: 2022, to: 2023, opens: 3, ends: 7 };

fn plays(l: &League, year: i32) -> bool {
    year >= l.from && year <= l.to
}

/// The other leagues' day: on a league's signing day, unsigned players
/// near the NFL's last roster spots sign there (a young player more
/// often than a veteran); on its last day they come back to the street,
/// sharp and on tape. NFL Europe took allocated NFL players each spring.
pub(crate) fn day(w: &mut World, d: Date) {
    let year = d.year();
    let mut all: Vec<(usize, League)> = LEAGUES.iter().copied().enumerate().collect();
    all.push((LEAGUES.len(), USFL_2022));
    for (k, l) in all {
        if !plays(&l, year) {
            continue;
        }
        if d == Date::ymd(year, l.opens, 15) {
            sign_players(w, d, k as u8, &l);
            if l.name == "NFL Europe" || l.name == "World League" {
                allocate(w, d, &l);
            }
        }
        if d == Date::ymd(year, l.ends, 28) {
            for i in 0..w.people.len() {
                let Some(c) = w.people[i].pro.as_mut() else { continue };
                if c.status == Status::Other(k as u8) {
                    c.status = Status::FreeAgent;
                    c.sharpness = 1.0;
                    // A season of real snaps for his development (section 5
                    // leftover; the games themselves are not on the engine):
                    // the CFL's 18 games are about a full NFL season, a
                    // spring league's ten about half (proposed, not confirmed:
                    // he starts most games there).
                    let len = (l.ends as f64 - l.opens as f64 + 1.0).max(1.0);
                    c.spring_snaps = c.spring_snaps.max(clamp(0.11 * len, 0.3, 0.8));
                    c.weeks_unsigned = 0;
                    let p = &mut w.people[i];
                    let age = p.row_age();
                    p.note(format!("Back on the NFL market at {age} after his {} season.", l.name));
                }
            }
        }
    }
    if d == w.dates.rookie_minicamp {
        pathway(w, d);
    }
}

fn sign_players(w: &mut World, d: Date, k: u8, l: &League) {
    let year = d.year();
    let pool: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            let p = &w.people[i];
            p.stage == crate::world::Stage::Pro
                && p.player.age <= 31
                && p.pro.as_ref().is_some_and(|c| c.status == Status::FreeAgent && !c.on_waivers)
        })
        .collect();
    let mut n = 0;
    for i in pool {
        let v = consensus(w, i);
        if !(54.0..68.0).contains(&v) {
            continue;
        }
        let young = w.people[i].player.age <= 26;
        let p = if young { 0.3 } else { 0.15 };
        if key_uniform(w.options.seed as u64 ^ 0x07E6 ^ (year as u64) << 8 ^ k as u64, i as u64) > p {
            continue;
        }
        let c = w.people[i].pro.as_mut().unwrap();
        c.status = Status::Other(k);
        let person = &mut w.people[i];
        let age = person.row_age();
        person.note(format!("Signed with a {} team at {age}.", l.name));
        n += 1;
    }
    w.market.year_mut(w.year).other_league += n;
    let _ = d;
}

/// NFL Europe allocations (real, 1991-2007): each NFL team sent a few young
/// players to play the spring; they come to camp with real snaps.
fn allocate(w: &mut World, d: Date, l: &League) {
    let year = d.year();
    for t in 0..w.clubs.len() {
        if !crate::world::franchise::active(t, year) {
            continue;
        }
        let mut young: Vec<(usize, f64)> = w.clubs[t]
            .roster
            .iter()
            .copied()
            .filter(|&i| {
                let p = &w.people[i];
                p.player.age <= 25
                    && p.pro.as_ref().is_some_and(|c| {
                        c.seasons.iter().filter(|s| s.games > 0).count() <= 2 && c.status == Status::Roster
                    })
            })
            .map(|i| (i, super::grade(w, t, i)))
            .collect();
        young.sort_by(|a, b| a.1.total_cmp(&b.1));
        for (i, _) in young.into_iter().rev().skip(10).take(3) {
            let name = w.nfl[t].name;
            let p = &mut w.people[i];
            p.pro.as_mut().unwrap().spring_snaps = 0.35;
            let age = p.row_age();
            p.note(format!("Allocated by the {name} to {} at {age}.", l.name));
        }
    }
}

/// The International Player Pathway (2017 on): athletes from other sports
/// and countries placed on practice squads in an extra spot (sizes by
/// year proposed, not confirmed: one division a year to 2021, then more).
fn pathway(w: &mut World, d: Date) {
    let year = d.year();
    if year < 2017 {
        return;
    }
    let n = match year {
        ..=2021 => 4,
        2022 => 8,
        2023 => 13,
        _ => 16,
    };
    let teams: Vec<usize> = crate::world::franchise::active_teams(year);
    let mut rng = Rng::new(w.options.seed ^ 0x1990 ^ year as u32);
    let start = (year as usize * 4) % teams.len().max(1);
    const FROM: [(&str, &str); 6] = [
        ("Australia", "rugby league"),
        ("England", "rugby union"),
        ("Germany", "the German Football League"),
        ("Nigeria", "track and field"),
        ("Mexico", "the Mexican league"),
        ("Ireland", "Gaelic football"),
    ];
    for k in 0..n.min(teams.len()) {
        let t = teams[(start + k) % teams.len()];
        let pos = *rng.pick(&[
            Position::OT,
            Position::DT,
            Position::EDGE,
            Position::TE,
            Position::RB,
            Position::IOL,
            Position::P,
        ]);
        let rating = rng.range(46.0, 56.0);
        let age = rng.int(22, 26) as u32;
        let seed = rng.next_u32();
        let mut p = super::veteran(w, pos, rating, age, seed);
        let (country, sport) = *rng.pick(&FROM);
        p.college = None;
        p.path.clear();
        p.notes.clear();
        let name = w.nfl[t].name;
        p.note(format!("Came from {country} ({sport}) through the International Player Pathway at {age}: on the {} practice squad (generated player).", super::possessive(name)));
        let mut c = ProCareer::new(Some(t), Status::PracticeSquad, year);
        c.ipp = true;
        c.ps_seasons = 1;
        let deal = super::money::build(
            t,
            super::money::Kind::PracticeSquad,
            year,
            &super::money::Terms::flat(1, super::money::practice_squad_week(year) * (w.rules.nfl_games as f64 + 1.0)),
        );
        c.contract = Some(deal);
        p.pro = Some(Box::new(c));
        w.people.push(p);
        let i = w.people.len() - 1;
        w.clubs[t].roster.push(i);
        w.market.year_mut(w.year).ipp += 1;
    }
    post(
        w,
        d,
        None,
        None,
        Loudness::Small,
        Kind::News,
        format!("{n} players join practice squads through the International Player Pathway."),
    );
}
