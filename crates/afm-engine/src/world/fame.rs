//! Fame and Aura (final build F15, F16, X6): who gets the ads, the brand
//! NIL deals, the endorsements and the followers.
//!
//! Aura is a hidden 1-20 trait (`player::Traits::aura`): charisma, camera
//! presence and style. It is drawn at 14 with a small link to ego and
//! leadership and grows a little with age and media training. It replaced
//! "looks": nothing in the sim reads a face.
//!
//! Fame is what the public knows of him, 0-1: his stage (a 5-star recruit,
//! an FBS starter, an NFL player), how well he plays, his role, his
//! position (the quarterback most, a long snapper least), his market (a big
//! program, a big city) and his honors. Money and followers come from both
//! together, `aura_appeal`: Aura multiplies fame and never replaces it, so a
//! backup with star Aura stays a backup to the brands, and a star with no
//! Aura still gets paid, only less.
//!
//! Other lanes call `fame`, `aura_appeal`, `brand_money` and `aura_read`.
//! There is no quarterback pay premium for looks or Aura (F16): contracts
//! read football only.

use super::{Person, Stage, World};
use crate::player::PlayingTime;
use crate::positions::Position;
use crate::rng::clamp;

/// How much a position draws the public's eye (proposed, not confirmed:
/// from endorsement and follower rankings, where quarterbacks lead and
/// linemen and specialists trail).
fn position_draw(p: Position) -> f64 {
    match p {
        Position::QB => 1.0,
        Position::RB | Position::WR => 0.8,
        Position::EDGE => 0.65,
        Position::TE | Position::CB => 0.6,
        Position::LB => 0.55,
        Position::S | Position::DT => 0.5,
        Position::OT | Position::IOL => 0.35,
        Position::K => 0.3,
        Position::P => 0.25,
        Position::LS => 0.15,
    }
}

/// TV households of an NFL club's home market, in millions (Nielsen DMA
/// estimates, 2024-25; older cities at their last NFL years' rank).
fn market_households(team: &str) -> f64 {
    const MARKETS: [(&str, f64); 31] = [
        ("New York", 7.5),
        ("Los Angeles", 5.8),
        ("Chicago", 3.5),
        ("Philadelphia", 3.1),
        ("Dallas", 3.0),
        ("Houston", 2.6),
        ("Atlanta", 2.6),
        ("Washington", 2.6),
        ("San Francisco", 2.6),
        ("Oakland", 2.6),
        ("New England", 2.5),
        ("Arizona", 2.2),
        ("Phoenix", 2.2),
        ("Seattle", 2.0),
        ("Tampa Bay", 2.0),
        ("Minnesota", 1.8),
        ("Detroit", 1.8),
        ("Denver", 1.8),
        ("Miami", 1.7),
        ("Cleveland", 1.5),
        ("Carolina", 1.3),
        ("St. Louis", 1.2),
        ("Pittsburgh", 1.2),
        ("Baltimore", 1.1),
        ("Indianapolis", 1.1),
        ("Tennessee", 1.1),
        ("San Diego", 1.1),
        ("Kansas City", 1.0),
        ("Cincinnati", 0.9),
        ("Las Vegas", 0.8),
        ("Jacksonville", 0.75),
    ];
    if team.starts_with("New Orleans") {
        return 0.65;
    }
    if team.starts_with("Buffalo") {
        return 0.6;
    }
    if team.starts_with("Green Bay") {
        return 0.45;
    }
    MARKETS.iter().find(|m| team.starts_with(m.0)).map_or(1.0, |m| m.1)
}

/// A market's pull on fame: a New York starter is better known than the
/// same player in Jacksonville (proposed, not confirmed: about +10% at New
/// York, -15% at Green Bay, whose fans still make up some of it).
fn market_draw(households: f64) -> f64 {
    clamp(0.92 + 0.12 * (households / 1.5).ln(), 0.75, 1.15)
}

/// His fame today, 0-1 (see the module notes).
pub fn fame(w: &World, i: usize) -> f64 {
    let p = &w.people[i];
    let last = p.player.history.last();
    let perf = last.and_then(|h| h.level_rating).map_or(0.45, |r| r as f64 / 99.0);
    let role: f64 = match p.last_role {
        Some(PlayingTime::Starter) => 1.0,
        Some(PlayingTime::Rotation) => 0.6,
        Some(PlayingTime::Backup) => 0.3,
        Some(PlayingTime::Redshirt | PlayingTime::PracticeSquad) => 0.15,
        Some(PlayingTime::Unsigned) => 0.1,
        None => 0.5,
    };
    let pos = position_draw(p.player.position);
    // The stage, its market and the honors that come with it.
    let (stage, market, honors) = match p.stage {
        Stage::HighSchool | Stage::PrepSchool => {
            // Recruits are known by their stars before they play a down.
            let stars = [0.08, 0.08, 0.1, 0.16, 0.32, 0.55][p.stars.min(5) as usize];
            let all_state = if p.cal.all_state >= 2 { 0.06 } else { 0.0 };
            (stars, 1.0, all_state)
        }
        Stage::College | Stage::Declared => {
            let stage = match p.college.map(|c| &w.programs[c]) {
                Some(prog) if prog.division == super::data::Division::FBS => 0.32 + 0.4 * prog.prestige,
                Some(prog) if prog.division == super::data::Division::FCS => 0.18 + 0.1 * prog.prestige_rel,
                Some(_) => 0.1,
                None => 0.2,
            };
            // A 5-star's name follows him to campus.
            let stars = 0.04 * p.stars.saturating_sub(3) as f64;
            (stage, 1.0, stars)
        }
        Stage::Pro => {
            let c = p.pro.as_ref();
            let team = c.and_then(|c| c.team);
            let market = team.map_or(0.85, |t| market_draw(market_households(&w.nfl_team_name(t))));
            let honors = c.map_or(0.0, |c| {
                let pro_bowls = c.seasons.iter().filter(|s| s.pro_bowl).count() as f64;
                let all_pros = c.seasons.iter().filter(|s| s.all_pro == 1).count() as f64;
                let latest = c.seasons.last().map_or(0.0, |s| {
                    if s.all_pro == 1 {
                        0.2
                    } else if s.pro_bowl {
                        0.1
                    } else {
                        0.0
                    }
                });
                (0.03 * pro_bowls + 0.05 * all_pros).min(0.25) + latest
            });
            (if team.is_some() { 0.6 } else { 0.35 }, market, honors)
        }
        // Out of the game: a name lives on, fading (the stored followers
        // and money do the fading; fame itself is his last stage's echo).
        Stage::Done => (0.12, 1.0, 0.0),
    };
    let play = 0.3 + 0.7 * perf.powf(1.5);
    clamp(stage * play * role.powf(0.6) * pos * market + honors, 0.0, 1.0)
}

/// Aura's multiplier on fame (proposed, not confirmed): 0.4 at Aura 1, 0.85
/// at 10, 1.6 at 20.
fn aura_mult(aura: u8) -> f64 {
    let a = (aura.clamp(1, 20) as f64 - 1.0) / 19.0;
    0.4 + 1.2 * a.powf(1.3)
}

/// What brands and audiences see in him, 0-1: fame times Aura. A backup
/// with star Aura stays small; the best-known star with the most Aura is 1.
pub fn aura_appeal(p: &Person, fame: f64) -> f64 {
    clamp(fame * aura_mult(p.player.personality.traits.aura) / 1.6, 0.0, 1.0)
}

/// How far social media reaches in a year: nothing before Facebook and
/// Twitter opened to everyone (2006), the full reach of today's platforms
/// by about 2016, a little more each year after.
fn reach(year: i32) -> f64 {
    match year {
        ..=2005 => 0.0,
        2006..=2015 => (year - 2005) as f64 / 11.0,
        _ => (1.0 + 0.02 * (year - 2016) as f64).min(1.4),
    }
}

/// His yearly money from ads, brand NIL deals and endorsements, in dollars
/// (proposed, not confirmed). NFL: about $50 million for the most famous
/// star with the most Aura (Patrick Mahomes earned about $60 million off
/// the field in 2024), low six figures for an average starter, little for a
/// backup; the market was about a third of today's in 1996. College: brand
/// deals only from July 2021 (NIL), up to about $2.5 million (the
/// collectives' money is separate, `recruiting::nil_payouts`). High
/// schoolers' deals stay with their state rules and are not modelled.
pub fn brand_money(w: &World, i: usize, fame: f64) -> f64 {
    let p = &w.people[i];
    let appeal = aura_appeal(p, fame);
    let year = w.year;
    match p.stage {
        Stage::Pro => {
            let era = clamp(0.35 + 0.65 * (year - 1996) as f64 / 30.0, 0.35, 1.0)
                * (1.0 + 0.02 * (year - 2026).max(0) as f64);
            80e6 * appeal.powi(4) * era
        }
        Stage::College | Stage::Declared if super::ncaa::nil_era(year) => 4e6 * appeal.powi(4),
        Stage::Done => p.brand as f64 * 0.55,
        _ => 0.0,
    }
}

/// The followers his fame and Aura would hold today (proposed, not
/// confirmed: an elite NFL star about 5 million, a backup about 40,000, an
/// FBS starter tens of thousands, a 5-star recruit about 100,000).
fn follower_target(p: &Person, fame: f64, year: i32) -> f64 {
    let appeal = aura_appeal(p, fame);
    let (top, base) = match p.stage {
        Stage::Pro => (6e6, 25_000.0),
        Stage::College | Stage::Declared => (2.5e6, 3_000.0),
        Stage::HighSchool | Stage::PrepSchool => (4e5, 400.0),
        Stage::Done => (0.0, 0.0),
    };
    reach(year) * (top * appeal.powf(2.2) + base)
}

/// The year's book for fame (February, with `people::year_book`): Aura
/// grows a little, followers move toward what fame and Aura hold, and the
/// year's brand money is set.
pub(crate) fn year_book(w: &mut World) {
    use rayon::prelude::*;
    let year = w.year;
    let seed = w.options.seed as u64 ^ 0xA0_7A00 ^ (year as u64) << 32;
    let update: Vec<(f64, f64)> = (0..w.people.len())
        .into_par_iter()
        .map(|i| {
            let f = fame(w, i);
            (f, brand_money(w, i, f))
        })
        .collect();
    let programs = &w.programs;
    w.people.par_iter_mut().zip(update).for_each(|(p, (f, money))| {
        let id = p.player.id as u64;
        let age = p.player.age;
        // Growing into it (proposed, not confirmed): about one point between
        // 15 and 27, more with media training at a big program or in the NFL.
        let mut chance = if (15..=27).contains(&age) { 0.08 } else { 0.0 };
        match p.stage {
            Stage::College if p.college.is_some_and(|c| programs[c].prestige > 0.5) && year >= 2000 => chance += 0.1,
            Stage::Pro if p.pro.as_ref().is_some_and(|c| c.seasons.len() <= 3) => chance += 0.15,
            _ => {}
        }
        if chance > 0.0 && super::season::key_uniform(seed, id) < chance {
            let t = &mut p.player.personality.traits;
            t.aura = (t.aura + 1).min(20);
        }
        // Followers move most of the way to what he holds now; they fall
        // slowly when he fades (people rarely unfollow).
        let target = follower_target(p, f, year);
        let now = p.followers as f64;
        let next = if target >= now { now + 0.6 * (target - now) } else { (now * 0.93).max(target) };
        p.followers = next.round().min(u32::MAX as f64) as u32;
        p.brand = money as f32;
    });
}

/// Followers and brand money set straight to what fame and Aura hold now,
/// for a world file from before Aura (`World::fill_final_build_fields`).
pub(crate) fn fill(w: &mut World) {
    use rayon::prelude::*;
    let year = w.year;
    let update: Vec<(f64, f64)> = (0..w.people.len())
        .into_par_iter()
        .map(|i| {
            let f = fame(w, i);
            (f, brand_money(w, i, f))
        })
        .collect();
    w.people.par_iter_mut().zip(update).for_each(|(p, (f, money))| {
        p.followers = follower_target(p, f, year).round() as u32;
        p.brand = money as f32;
    });
}

/// A scout's or reporter's word for his Aura (the game shows only this):
/// `accuracy` is the reader's judging skill, 1-20; the key fixes the error
/// so the same reader says the same thing.
pub fn aura_read(p: &Person, accuracy: f64, key: u64) -> &'static str {
    let error = crate::player::keyed_normal(key ^ 0xA0_7A5E, p.player.id as u64) * (20.0 - accuracy).max(1.0) / 5.0;
    match p.player.personality.traits.aura as f64 + error {
        x if x >= 16.5 => "A natural in front of a camera",
        x if x >= 13.5 => "Comfortable in the spotlight",
        x if x >= 7.5 => "Ordinary on camera",
        x if x >= 4.5 => "Uneasy in front of a camera",
        _ => "Shy of the cameras",
    }
}
