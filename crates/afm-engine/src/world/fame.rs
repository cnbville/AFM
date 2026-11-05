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

use super::data::HsNilScope;
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
        // Away on a mission or in the service: known as he was, quietly.
        Stage::Done | Stage::Away => (0.12, 1.0, 0.0),
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

/// Whether his school's state lets him earn NIL money today (N3). The
/// state association's or legislature's rule binds the school he plays for
/// (a boarder at IMG plays under Florida's). Open to every player once the
/// state's first season has come (`StateInfo::hs_nil`, switched on by the
/// clock) and he is old enough; in Arkansas and Missouri only once he has
/// signed with a college in the state, from the law's season. Texas lets a
/// 17-year-old sign only with a college, paid once his high school days
/// are over, so a Texas high schooler earns nothing.
pub fn hs_nil_open(w: &World, p: &Person) -> bool {
    if !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) {
        return false;
    }
    let Some(s) = w.high_schools.get(p.hs).map(|h| h.state) else { return false };
    let Some(state) = w.states.get(s) else { return false };
    let rule = super::data::hs_nil_rule(&state.code);
    if state.hs_nil {
        // A state's own minimum age (none open to all has one today); a
        // later vote opening a limited state makes a new rule.
        return rule.scope != HsNilScope::All || p.age_on(w.today) >= rule.min_age;
    }
    match rule.scope {
        HsNilScope::SignedInState => {
            rule.from.is_some_and(|y| y <= w.year)
                && p.cal.signed.as_ref().is_some_and(|g| w.programs.get(g.program).is_some_and(|c| c.state == s))
        }
        _ => false,
    }
}

/// A five-star quarterback starter's fame in high school (stars 0.55 x play
/// about 0.9 + first-team all-state 0.06) and what his deals bring a year
/// with ordinary Aura (proposed, not confirmed). Real: Faizon Brandon's
/// Leaf card deal, $1.2 million over two years (North Carolina, 2025);
/// four-star Antwann Hill's, $600,000-750,000 over several years (Georgia,
/// 2024); Julian Lewis six figures; Malachi Nelson nearly $1 million by the
/// time he enrolled (California, 2022).
const HS_NIL_FAME: f64 = 0.56;
const HS_NIL_TOP: f64 = 300_000.0;
/// How steeply brands follow fame in high school (proposed, not
/// confirmed): only the best-known get national deals. Real: Georgia's
/// association counted 44-87 athletes with deals a year out of 430,000,
/// only 2-3 of them over $1,000; North Carolina 12-21 athletes a year, the
/// usual deal $200-700 or the product itself; Ohio 32 deals in its first
/// months (2024-26 reports).
const HS_NIL_FAME_POWER: i32 = 6;
const HS_NIL_AURA_POWER: i32 = 2;
/// The smallest deal worth making: the gear-and-a-post deals of $200-700
/// (North Carolina's 2025-26 report).
const HS_NIL_SMALLEST: f64 = 200.0;

/// Whether his family takes deals at all (proposed, not confirmed): most do;
/// a comfortable family that cares little for money often turns them down,
/// as the Mannings did. Drawn once for him, so the answer holds.
fn takes_hs_deals(w: &World, p: &Person) -> bool {
    let need = 0.25 + 0.75 * (1.0 - p.bg.money);
    let want = p.family.decider_wants.money;
    let chance = clamp(0.35 + 0.5 * need + 1.5 * (want - 1.0 / 6.0), 0.15, 0.95);
    super::season::key_uniform(w.options.seed as u64 ^ 0x45_D1A1, p.player.id as u64) < chance
}

/// A high schooler's NIL money a year where his state allows it (N3): local
/// businesses, gear and posts for the locally known, trading cards, jewelry
/// and an apparel brand for the national names. Fame drives it steeply and
/// Aura gently: fame / 0.56 to the 6th x (Aura's multiplier / ordinary)
/// squared x $300,000; nothing under $200 or where the family says no.
pub fn hs_nil_money(w: &World, i: usize, fame: f64) -> f64 {
    let p = &w.people[i];
    if !hs_nil_open(w, p) || !takes_hs_deals(w, p) {
        return 0.0;
    }
    hs_nil_offered(p, fame)
}

/// What brands would pay him a year in high school, before his state's rule
/// and his family's say (N3; the formula above).
pub fn hs_nil_offered(p: &Person, fame: f64) -> f64 {
    let aura = aura_mult(p.player.personality.traits.aura) / aura_mult(10);
    let money = HS_NIL_TOP * (fame / HS_NIL_FAME).powi(HS_NIL_FAME_POWER) * aura.powi(HS_NIL_AURA_POWER);
    if money < HS_NIL_SMALLEST { 0.0 } else { money }
}

/// Who pays a high schooler's NIL money, by its size (the screens' words):
/// real high school deals are local businesses and gear at the bottom,
/// trading cards and autograph signings in the middle, and an apparel brand
/// beside the cards at the top.
pub fn hs_nil_kinds(money: f64) -> &'static str {
    match money {
        m if m >= 50_000.0 => "a trading card deal and an apparel brand",
        m if m >= 5_000.0 => "trading cards and autograph signings",
        m if m >= 1_000.0 => "local businesses and posts",
        m if m > 0.0 => "gear and a post or two",
        _ => "",
    }
}

/// His yearly money from ads, brand NIL deals and endorsements, in dollars
/// (proposed, not confirmed). NFL: about $50 million for the most famous
/// star with the most Aura (Patrick Mahomes earned about $60 million off
/// the field in 2024), low six figures for an average starter, little for a
/// backup; the market was about a third of today's in 1996. College: brand
/// deals only from July 2021 (NIL), up to about $2.5 million (the
/// collectives' money is separate, `recruiting::nil_payouts`). High
/// school: his state's rules (`hs_nil_money`, N3).
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
        Stage::HighSchool | Stage::PrepSchool => hs_nil_money(w, i, fame),
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
        Stage::Done | Stage::Away => (0.0, 0.0),
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
    let news = hs_news(w, update.iter().enumerate().map(|(i, u)| (i, u.1)));
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
        hs_payday(p, money);
        p.brand = money as f32;
    });
    post_hs_news(w, &news);
}

/// A high schooler's first deals past $250,000 a year are a payday for the
/// development step, as a college deal is (fix list 4.5; N3).
fn hs_payday(p: &mut Person, money: f64) {
    if matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) && money > 250_000.0 && p.brand as f64 <= 250_000.0 {
        p.cal.payday = true;
    }
}

/// The high schoolers whose deals pass $100,000 a year for the first time:
/// the national names, whose card and apparel deals make the news (N3).
fn hs_news(w: &World, money: impl Iterator<Item = (usize, f64)>) -> Vec<(usize, f64)> {
    money
        .filter(|&(i, m)| {
            let p = &w.people[i];
            matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) && m >= 100_000.0 && (p.brand as f64) < 100_000.0
        })
        .collect()
}

fn post_hs_news(w: &mut World, news: &[(usize, f64)]) {
    use crate::calendar::events::{Event, Kind, Level, Loudness};
    for &(i, money) in news {
        let p = &w.people[i];
        let stars = match p.stars {
            5 => "Five-star ",
            4 => "Four-star ",
            3 => "Three-star ",
            _ => "",
        };
        let school = w.school_name(p.hs).to_string();
        let state = w.high_schools.get(p.hs).and_then(|h| w.states.get(h.state)).map_or("", |s| s.code.as_str());
        let amount = super::season_page::money_words((money / 1e4).round() * 1e4);
        let text = format!(
            "{stars}{} {} of {school} ({state}) signs NIL deals with {} worth about {amount} a year.",
            p.player.position.name(),
            p.name,
            hs_nil_kinds(money)
        );
        let e = Event::new(w.today, Level::HighSchool, Kind::News, Loudness::Small, text).person(p.player.id, &p.name);
        w.calendar.post(e);
    }
}

/// June 15, after the year's new state rules and the high school transfers
/// (N3): a high schooler's deals follow his school's state, his age and his
/// signing (a move to a state that bars it ends them; a Missouri senior who
/// signed at home starts his).
pub(crate) fn hs_nil_day(w: &mut World) {
    use rayon::prelude::*;
    let money: Vec<(usize, f64)> = (0..w.people.len())
        .into_par_iter()
        .filter(|&i| matches!(w.people[i].stage, Stage::HighSchool | Stage::PrepSchool))
        .map(|i| (i, brand_money(w, i, fame(w, i))))
        .collect();
    let news = hs_news(w, money.iter().copied());
    for &(i, m) in &money {
        let p = &mut w.people[i];
        hs_payday(p, m);
        p.brand = m as f32;
    }
    post_hs_news(w, &news);
}

/// A world file from before N3 (NIL update): Arkansas (from 2021),
/// Missouri and Texas were open to every high schooler then, so their flags
/// come off unless a later state vote opened them (`college_changes`,
/// recorded in the changes), and its high schoolers get their deals as
/// February's book would have set them. Only such a file has a flag to take
/// off, so a newer save is left exactly as it was, and the same old file
/// always fills the same way.
pub(crate) fn hs_nil_after_load(w: &mut World) {
    let mut old = false;
    for s in 0..w.states.len() {
        let code = w.states[s].code.clone();
        let voted = w.changes.applied.iter().any(|c| c.change == crate::calendar::Change::HsNil && c.detail == code);
        if w.states[s].hs_nil && !voted && super::data::hs_nil_rule(&code).scope != HsNilScope::All {
            w.states[s].hs_nil = false;
            old = true;
        }
    }
    if !old {
        return;
    }
    let school = |p: &Person| matches!(p.stage, Stage::HighSchool | Stage::PrepSchool);
    use rayon::prelude::*;
    let money: Vec<(usize, f64)> = (0..w.people.len())
        .into_par_iter()
        .filter(|&i| school(&w.people[i]))
        .map(|i| (i, brand_money(w, i, fame(w, i))))
        .collect();
    for (i, m) in money {
        w.people[i].brand = m as f32;
    }
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
