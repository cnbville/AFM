//! Social media (X5): every prospect and player with a following has a
//! follower count that grows with his fame and his Aura (the hidden pull
//! that makes people want to follow him), and from the NIL era the
//! followers bring their own deals on top of the collective's money (real:
//! brands pay college athletes by audience, roughly 30 to 60 cents a
//! follower a year for those with real engagement; Opendorse 2023-24).
//! Staffs read a prospect's accounts (the Social look) for character.
//!
//! Social media starts mattering about 2008 (Facebook, then Twitter in
//! 2009 and Instagram in 2012) and reaches today's size by about 2016.

use crate::calendar::{Date, Weekday};
use crate::world::season::key_normal;
use crate::world::{Person, Stage, World};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Social {
    /// Player id to followers (only those with a following worth keeping).
    #[serde(default)]
    pub followers: std::collections::HashMap<u32, u32>,
    /// Last July's follower deals: player id to dollars.
    #[serde(default)]
    pub deals: std::collections::HashMap<u32, u32>,
}

/// How big social media was in a year against today.
pub fn era(year: i32) -> f64 {
    match year {
        ..=2007 => 0.0,
        2008..=2015 => 0.1 + 0.9 * (year - 2008) as f64 / 8.0,
        _ => (1.0 + 0.04 * (year - 2016) as f64).min(1.4),
    }
}

/// His Aura (1-20, hidden). Lane 1 brings Aura as a person's own trait; until
/// it lands this reads a stable stand-in from his id, centred on 10.5 (the
/// one place to switch it over).
pub fn aura(w: &World, p: &Person) -> f64 {
    (10.5 + 3.5 * key_normal(w.options.seed as u64 ^ 0xA0A, p.player.id as u64)).clamp(1.0, 20.0)
}

/// His fame: how many people know his name, from his level, his recruiting
/// stars, his play and his program's following (proposed, not confirmed:
/// the targets are a five-star senior near 40,000 followers, a Power 4
/// starter 20,000 to 80,000, a star pro a million and more).
pub fn fame(w: &World, p: &Person) -> f64 {
    let stars = [300.0, 600.0, 1200.0, 3_000.0, 10_000.0, 40_000.0][p.stars.min(5) as usize];
    let perf = p.last_performance.map_or(0.0, |x| ((x - 50.0) / 15.0).clamp(-1.0, 3.0));
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => stars,
        Stage::College | Stage::Declared => {
            let fans = p.college.map_or(0.5, |c| w.programs[c].fans.clamp(0.0, 1.5));
            let role = match p.last_role {
                Some(crate::world::PlayingTime::Starter) => 3.0,
                Some(crate::world::PlayingTime::Rotation) => 1.5,
                _ => 0.8,
            };
            stars * 0.7 + 8_000.0 * fans * role * (1.0 + 0.6 * perf.max(-0.5))
        }
        Stage::Pro => {
            let starter = p.pro.as_ref().is_some_and(|c| c.team.is_some()) as u8 as f64;
            let round = p.pro.as_ref().and_then(|c| c.draft).map_or(0.0, |d| (8.0 - d.1 as f64).max(0.0));
            30_000.0 * (1.0 + starter) * (1.0 + 0.4 * round) * (1.0 + 1.2 * perf.max(-0.5))
        }
        _ => 0.0,
    }
}

/// The followers he is heading for: fame, Aura (a 15 doubles a 10) and the
/// era.
pub fn target(w: &World, p: &Person) -> f64 {
    let a = aura(w, p);
    fame(w, p) * 2f64.powf((a - 10.0) / 5.0) * era(w.year)
}

/// Sundays: followers move toward their target (up quickly after a big
/// week, down slowly when he fades). July 2: the follower deals.
pub(crate) fn day(w: &mut World, d: Date) {
    let (_, m, day) = d.parts();
    if m == 7 && day == 2 && crate::world::ncaa::nil_era(w.year) {
        deals(w);
    }
    if d.weekday() != Weekday::Sun || era(w.year) == 0.0 {
        return;
    }
    // In season every week, otherwise the first Sunday of a month.
    if !matches!(m, 9..=12 | 1) && day > 7 {
        return;
    }
    let mut next = std::collections::HashMap::with_capacity(w.scouting.social.followers.len());
    for p in &w.people {
        let keep = match p.stage {
            Stage::College | Stage::Declared | Stage::Pro => true,
            Stage::HighSchool | Stage::PrepSchool => p.stars >= 3,
            _ => false,
        };
        if !keep {
            continue;
        }
        let id = p.player.id;
        let now = w.scouting.social.followers.get(&id).copied().unwrap_or(0) as f64;
        let t = target(w, p);
        let new = if t > now { now + 0.15 * (t - now) } else { now - 0.02 * (now - t) };
        if new >= 500.0 {
            next.insert(id, new.round() as u32);
        }
    }
    w.scouting.social.followers = next;
}

/// July 2, after the collectives pay: brands pay by audience, so a big
/// following earns on top of the collective (from 2021).
fn deals(w: &mut World) {
    let year = w.year;
    let market = crate::world::ncaa::nil_market(year);
    let mut out = std::collections::HashMap::new();
    for p in w.people.iter_mut().filter(|p| p.stage == Stage::College) {
        let Some(&f) = w.scouting.social.followers.get(&p.player.id) else { continue };
        if f < 5_000 {
            continue;
        }
        // About 40 cents a follower a year at today's market, less for a
        // small account (fewer brands buy small audiences).
        let rate = 0.4 * market * (f as f64 / 50_000.0).clamp(0.4, 1.0);
        let money = f as f64 * rate;
        p.nil_income += money;
        out.insert(p.player.id, money.round() as u32);
    }
    w.scouting.social.deals = out;
}

/// Words for a follower count.
pub fn words(f: u32) -> String {
    match f {
        0..=999 => format!("{f}"),
        1_000..=999_999 => format!("{:.1}K", f as f64 / 1000.0).replace(".0K", "K"),
        _ => format!("{:.1}M", f as f64 / 1_000_000.0).replace(".0M", "M"),
    }
}
