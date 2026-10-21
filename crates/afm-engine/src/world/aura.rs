//! Aura, fame, followers and brand money (final build F15, F16, X6).
//!
//! Aura is a hidden 1-20 trait (`player::Traits::aura`): charisma, camera
//! presence and style. It is drawn at 14 with a small link to ego and
//! leadership and grows a little with age, the cameras and media training.
//! Looks play no part anywhere in the sim (F13 replaced).
//!
//! Fame (0-100) is public: what the media and fans know of him. It comes
//! from his stage (high school stars, a Power 4 starter, an NFL starter),
//! how well he plays and what he has won, his position (a quarterback is
//! seen most, a guard least) and his market. Followers, ads, NIL brand
//! deals and endorsements all come from fame times Aura, so Aura alone
//! never makes a backup rich, and a famous man with little Aura earns less
//! than his fame suggests. Pay never reads Aura (F16: no NFL quarterback
//! premium for looks or Aura): salaries and NIL collective money come from
//! the market as before.
//!
//! Every draw is keyed (seed, person, year), so no stream moves.
//! The sizes are Claude's pick (final build), from public figures: the
//! On3 NIL valuations and follower counts of 2024-25 (a 5-star recruit
//! 40,000-100,000 followers, Arch Manning about 400,000, Shedeur Sanders
//! about 2 million), Ashton Jeanty's +35,000 on draft night 2025, and
//! Forbes's endorsement estimates (Patrick Mahomes about $20 million a
//! year, an average starter low six figures).

use crate::player::{Level, PlayingTime};
use crate::positions::Position;
use crate::rng::clamp;

use super::data::Division;
use super::season::key_uniform;
use super::{Person, Stage, World};

/// How much a position is seen (Claude's pick): the quarterback most, the
/// line and the specialists least.
pub fn position_pull(pos: Position) -> f64 {
    match pos {
        Position::QB => 1.35,
        Position::RB | Position::WR => 1.12,
        Position::TE | Position::EDGE | Position::CB => 1.0,
        Position::LB | Position::S => 0.92,
        Position::DT => 0.85,
        Position::OT | Position::IOL => 0.65,
        _ => 0.55,
    }
}

/// TV homes in a club's market, millions (Nielsen DMA estimates, 2024-25).
fn market_homes(city: &str) -> f64 {
    match city {
        "East Rutherford" => 7.5,
        "Los Angeles" | "Inglewood" | "Carson" | "Anaheim" => 5.8,
        "Chicago" => 3.5,
        "Philadelphia" => 3.0,
        "Arlington" | "Irving" => 3.0,
        "Houston" => 2.6,
        "Atlanta" => 2.6,
        "San Francisco" | "Santa Clara" | "Oakland" => 2.6,
        "Washington" | "Landover" => 2.5,
        "Foxborough" => 2.5,
        "Glendale" | "Tempe" => 2.1,
        "Seattle" => 2.0,
        "Tampa" => 2.0,
        "Minneapolis" | "Detroit" | "Pontiac" => 1.8,
        "Denver" | "Miami Gardens" => 1.7,
        "Cleveland" => 1.5,
        "Charlotte" | "Clemson" => 1.3,
        "St. Louis" | "Pittsburgh" => 1.2,
        "Baltimore" | "Nashville" | "Indianapolis" | "San Diego" => 1.1,
        "Kansas City" | "Cincinnati" => 0.95,
        "Paradise" => 0.8,
        "Jacksonville" => 0.73,
        "Memphis" => 0.65,
        "New Orleans" => 0.64,
        "Orchard Park" => 0.6,
        "Green Bay" => 0.47,
        _ => 1.0,
    }
}

/// An NFL club's market as a fame factor: about 0.95 in Green Bay, 1.2 in
/// New York.
pub fn club_market(team: usize, year: i32) -> f64 {
    let homes = super::franchise::era(team, year).map_or(1.0, |e| market_homes(e.city));
    0.85 + 0.35 * (homes / 7.5).sqrt()
}

/// A college program's market as a fame factor: its fan base, 0.8 to 1.3.
fn program_market(w: &World, prog: usize) -> f64 {
    w.programs.get(prog).map_or(1.0, |p| 0.8 + 0.5 * p.fans.clamp(0.0, 1.0))
}

fn role_share(role: Option<PlayingTime>) -> f64 {
    match role {
        Some(PlayingTime::Starter) => 1.0,
        Some(PlayingTime::Rotation) => 0.6,
        Some(PlayingTime::Backup) => 0.3,
        Some(PlayingTime::Redshirt) | Some(PlayingTime::PracticeSquad) => 0.15,
        Some(PlayingTime::Unsigned) => 0.1,
        None => 0.2,
    }
}

/// How far back an honour still counts: all of last season's, half of the
/// one before, a quarter before that.
fn fade(years_ago: i32) -> f64 {
    match years_ago {
        i32::MIN..=0 => 1.0,
        1 => 0.5,
        2 => 0.25,
        _ => 0.1,
    }
}

/// His fame, 0-100, as of `year` (the season just played or being
/// played): public, what the media and fans know of him.
pub fn fame(w: &World, p: &Person, year: i32) -> f64 {
    let pos = position_pull(p.player.position);
    let id = p.player.id;
    let raw = match p.stage {
        Stage::Done => 0.0,
        Stage::HighSchool | Stage::PrepSchool => {
            let stars = [1.0, 4.0, 6.0, 14.0, 28.0, 46.0][p.stars.min(5) as usize];
            let powerhouse = w.high_schools.get(p.hs).is_some_and(|s| s.is_powerhouse()) as u8 as f64;
            let offers = (p.cal.offers.len() + p.cal.early_offers.len()).min(30) as f64;
            (stars + 4.0 * p.cal.all_state as f64 + 4.0 * powerhouse + 0.4 * offers) * pos
        }
        Stage::College | Stage::Declared => {
            let prog = p.college.unwrap_or(0);
            let (base, level) = match w.programs.get(prog).map(|x| (x.division, x.power_four())) {
                Some((Division::FBS, true)) => (24.0, Level::FBS),
                Some((Division::FBS, false)) => (14.0, Level::FBS),
                Some((Division::FCS, _)) => (8.0, Level::FCS),
                _ => (4.0, Level::SmallCollege),
            };
            let last = p.player.history.iter().rev().find(|h| h.level == level);
            let rank = last.and_then(|h| h.level_rating).map_or(0.0, |r| (r as f64 - 60.0).max(0.0) / 39.0);
            let honors = w.stats.honors.get(&id).map_or(0.0, |h| {
                let aa: f64 = h.all_america.iter().map(|&(y, _)| 14.0 * fade(year - y)).sum();
                let awards: f64 = h
                    .awards
                    .iter()
                    .map(|&(y, a)| fade(year - y) * if a == "Heisman Trophy" { 35.0 } else { 10.0 })
                    .sum();
                aa + awards
            });
            let hype = [0.0, 0.0, 0.0, 1.0, 5.0, 11.0][p.stars.min(5) as usize];
            let draft = p.cal.draft_read.map_or(0.0, |r| if r.round == 1 { 10.0 } else { 0.0 });
            (base * role_share(p.last_role) + 20.0 * rank + honors + hype + draft) * pos * program_market(w, prog)
        }
        Stage::Pro => {
            let Some(c) = p.pro.as_ref() else { return 0.0 };
            let recent: Vec<_> = c.seasons.iter().filter(|s| s.year <= year && year - s.year <= 3).collect();
            let last = recent.iter().rev().find(|s| s.games > 0);
            let role = match last {
                Some(s) if s.starts >= 8 => 1.0,
                Some(s) if s.starts > 0 => 0.65,
                Some(_) => 0.35,
                None => 0.15,
            };
            let honors: f64 = recent
                .iter()
                .map(|s| {
                    let mut v = if s.pro_bowl { 14.0 } else { 0.0 };
                    v += match s.all_pro {
                        1 => 12.0,
                        2 => 6.0,
                        _ => 0.0,
                    };
                    v += s.awards.iter().map(|&a| if a == "MVP" { 25.0 } else { 12.0 }).sum::<f64>();
                    v * fade(year - s.year)
                })
                .sum();
            let legacy = (2.5 * c.seasons.iter().filter(|s| s.pro_bowl).count() as f64).min(20.0);
            let rookie = c.draft.map_or(0.0, |(y, r, pick, _)| {
                let hype = if r == 1 { 12.0 + 18.0 / (pick as f64).sqrt() } else { 0.0 };
                hype * fade(year - y)
            });
            let champ = recent
                .iter()
                .filter(|s| s.team.is_some_and(|t| w.nfl_champions.iter().any(|&(y, ch)| y == s.year && ch == t)))
                .map(|s| 6.0 * fade(year - s.year))
                .sum::<f64>();
            let market = c.team.map_or(1.0, |t| club_market(t, year));
            (30.0 * role + honors + legacy + rookie + champ) * pos * market
        }
    };
    clamp(raw, 0.0, 100.0)
}

/// How Aura turns fame into followers and money: 1 at an ordinary 10,
/// about 2.5 at 20 and 0.45 at 1.
pub fn aura_pull(aura: u8) -> f64 {
    (0.09 * (aura.max(1) as f64 - 10.0)).exp()
}

/// How far social media reached each year: athletes' accounts from about
/// 2009 (Twitter, then Instagram in 2010), near everyone by 2015, still
/// growing after (TikTok, 2018).
pub fn social_era(year: i32) -> f64 {
    match year {
        y if y < 2008 => 0.0,
        2008 => 0.05,
        2009 => 0.15,
        2010 => 0.3,
        2011 => 0.45,
        2012 => 0.6,
        2013 => 0.7,
        2014 => 0.8,
        2015 => 0.88,
        2016 => 0.93,
        2017 => 0.96,
        y => (1.0 + 0.02 * (y - 2018) as f64).min(1.2),
    }
}

/// The followers fame and Aura settle toward in `year`: about 300 for an
/// unknown, 10,000 at fame 40, 340,000 at 80, 2 million at 100, times
/// Aura's pull.
pub fn followers_for(fame: f64, aura: u8, year: i32) -> f64 {
    social_era(year) * 300.0 * (0.088 * fame).exp() * aura_pull(aura)
}

/// What an ad's camera asks of him beyond his followers: 0.64 at Aura 1,
/// 1 at 10, 1.4 at 20.
fn camera(aura: u8) -> f64 {
    0.6 + 0.04 * aura as f64
}

/// The endorsement market against 2024 (nominal dollars): about a third
/// in 1996, growing 3% a year after 2024.
fn endorsement_era(year: i32) -> f64 {
    if year <= 2024 { (0.3 + 0.7 * (year - 1996).max(0) as f64 / 28.0).min(1.0) } else { 1.03f64.powi(year - 2024) }
}

/// This year's brand money (dollars) at `followers`: NIL brand deals in
/// college from July 2021 (and in high school where the state allows it),
/// endorsements and ads for a pro in any era. Collective and revenue-share
/// money is not here: `recruiting::nil_payouts` pays it as before.
pub fn brand_money(w: &World, p: &Person, fame: f64, followers: f64, year: i32) -> f64 {
    let aura = p.player.personality.traits.aura;
    let deals = |share: f64| share * super::ncaa::nil_market(year) * 1.6 * followers.max(1.0).powf(1.05) * camera(aura);
    match p.stage {
        Stage::College | Stage::Declared if super::ncaa::nil_era(year) => deals(1.0),
        Stage::HighSchool if super::ncaa::nil_era(year) && fame >= 10.0 => {
            let state = w.high_schools.get(p.hs).map_or(p.home_state, |s| s.state);
            if w.states.get(state).is_some_and(|s| s.hs_nil) { deals(0.5) } else { 0.0 }
        }
        Stage::Pro => endorsement_era(year) * 5_800.0 * (0.075 * fame).exp() * aura_pull(aura) * camera(aura) / 1.4,
        _ => 0.0,
    }
}

/// Followers a year on: most of the way up to what fame and Aura call for,
/// slowly down (followers stay after the fame fades).
fn followers_next(now: f64, target: f64) -> f64 {
    if target > now { now + 0.6 * (target - now) } else { now - 0.15 * (now - target) }
}

/// July 1, the money year (before the NIL payouts): each person's fame from
/// the season just played sets his followers and this year's brand money.
/// A new national deal makes the news.
pub(crate) fn media_year(w: &mut World) {
    use rayon::prelude::*;
    // Fame from the season just played; money and followers for the year
    // ahead (the football year that started in February).
    let season = w.year - 1;
    let year = w.year;
    let wr = &*w;
    let next: Vec<(u32, f32)> = wr
        .people
        .par_iter()
        .map(|p| {
            let f = fame(wr, p, season);
            let target = followers_for(f, p.player.personality.traits.aura, year);
            let followers = followers_next(p.followers as f64, target).round().clamp(0.0, u32::MAX as f64);
            (followers as u32, brand_money(wr, p, f, followers, year) as f32)
        })
        .collect();
    let mut news: Vec<(usize, f64)> = Vec::new();
    let recording = w.recording;
    for (i, (p, (followers, money))) in w.people.iter_mut().zip(next).enumerate() {
        p.followers = followers;
        if money >= 1.5e6 && p.brand < 1.5e6 && recording {
            news.push((i, money as f64));
        }
        p.brand = money;
        if p.stage == Stage::HighSchool {
            // A high schooler's NIL money is all brand deals (no collective).
            p.nil_income = money as f64;
        }
    }
    for (i, m) in news {
        brand_news(w, i, m);
    }
}

/// A big new deal makes the news (a national brand: about $1.5 million a
/// year or more).
fn brand_news(w: &mut World, i: usize, money: f64) {
    use crate::calendar::{Event, Kind, Level as CalLevel, Loudness, Ref};
    let p = &w.people[i];
    let (level, what) = match p.stage {
        Stage::Pro => (CalLevel::Nfl, "a national endorsement deal"),
        Stage::HighSchool => (CalLevel::Recruiting, "a national NIL deal"),
        _ => (CalLevel::College, "a national NIL deal"),
    };
    let million = (money / 1e5).round() / 10.0;
    let text = format!("{} lands {what}, worth about ${million:.1} million a year.", p.name);
    let mut e = Event::new(w.today, level, Kind::News, Loudness::Small, text).link(&p.name, Ref::Player(p.player.id));
    match p.stage {
        Stage::College => {
            if let Some(c) = p.college {
                e = e.program(c);
            }
        }
        Stage::Pro => {
            if let Some(t) = p.pro.as_ref().and_then(|c| c.team) {
                e = e.team(t);
            }
        }
        _ => {}
    }
    w.calendar.post(e);
}

/// Draft night (X6): a pick's followers jump with how high he went, times
/// his Aura (Ashton Jeanty, 6th, +35,000 in a day in 2025).
pub(crate) fn draft_night(p: &mut Person, pick: u32, year: i32) {
    let bump = social_era(year) * 90_000.0 / (pick.max(1) as f64).sqrt() * aura_pull(p.player.personality.traits.aura);
    p.followers = p.followers.saturating_add(bump.round() as u32);
}

/// Aura grows a little (F15): in the late teens and twenties, a point now
/// and then from growing up, more on a big stage's cameras and with media
/// training (Power 4 programs and the NFL's rookie program teach it). About
/// one to three points from 14 to 28 for a pro, less for a boy who never
/// leaves high school (Claude's pick). Keyed by (seed, person, age), so an
/// older world replays the same steps from his rows on load.
pub(crate) fn grow_aura(seed: u64, id: u32, age: u32, level: Option<Level>, power: bool, aura: &mut u8) {
    if !(15..=29).contains(&age) || *aura == 0 || *aura >= 20 {
        return;
    }
    let mut chance = if age <= 26 { 0.05 } else { 0.02 };
    chance += match level {
        Some(Level::NFL) => 0.1,
        Some(Level::FBS) if power => 0.08,
        Some(Level::FBS) | Some(Level::FCS) => 0.04,
        _ => 0.0,
    };
    if key_uniform(seed ^ 0xA0_7A_0001, (id as u64) << 8 | age as u64) < chance {
        *aura += 1;
    }
}

/// The yearly step (signing day, with the people's year book).
pub(crate) fn aura_year(w: &mut World) {
    use rayon::prelude::*;
    let seed = w.options.seed as u64;
    let programs = &w.programs;
    w.people.par_iter_mut().for_each(|p| {
        let (level, power) = match p.stage {
            Stage::Pro => (Some(Level::NFL), false),
            Stage::College => match p.college.and_then(|c| programs.get(c)) {
                Some(prog) => (Some(super::division_level(prog.division)), prog.power_four()),
                None => (None, false),
            },
            _ => (None, false),
        };
        let age = p.player.age;
        let id = p.player.id;
        grow_aura(seed, id, age, level, power, &mut p.player.personality.traits.aura);
    });
}

/// An older world file has no Aura: each player gets his draw at 14 (keyed)
/// and the growth his season rows would have given him; followers and
/// brand money follow from his fame now.
pub(crate) fn fill_missing(w: &mut World) {
    use rayon::prelude::*;
    let seed = w.options.seed as u64;
    let fill = |player: &mut crate::player::Player| {
        let t = &mut player.personality.traits;
        if t.aura != 0 {
            return false;
        }
        let own = crate::player::keyed_normal(seed ^ 0xA0_7A_0014, player.id as u64);
        t.aura = crate::player::Traits::aura_at_14(own, t.ego, t.leadership);
        for h in &player.history {
            let power = false;
            grow_aura(seed, player.id, h.age, Some(h.level), power, &mut player.personality.traits.aura);
        }
        true
    };
    let filled: Vec<bool> = w.people.par_iter_mut().map(|p| fill(&mut p.player)).collect();
    for a in w.alumni.iter_mut() {
        if let Some(pl) = a.player.as_mut() {
            fill(pl);
        }
    }
    if !filled.iter().any(|&f| f) {
        return;
    }
    let year = w.year - 1;
    let today = w.year;
    let fame: Vec<f64> = w.people.par_iter().map(|p| fame(w, p, year)).collect();
    for (i, p) in w.people.iter_mut().enumerate() {
        if filled[i] && p.followers == 0 {
            p.followers = followers_for(fame[i], p.player.personality.traits.aura, today).round() as u32;
        }
    }
    let money: Vec<f64> =
        w.people.par_iter().zip(fame.par_iter()).map(|(p, &f)| brand_money(w, p, f, p.followers as f64, today)).collect();
    for (i, p) in w.people.iter_mut().enumerate() {
        if filled[i] {
            p.brand = money[i] as f32;
        }
    }
}
