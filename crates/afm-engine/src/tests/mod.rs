//! Engine tests, ported from the TypeScript model. Calibration of a whole
//! national class lives in afm-tools (it needs the debug summary).

mod coaching_people;
mod compact;
mod development;
mod faces;
mod foundations;
mod front_office;
mod game_stats;
mod gameplan;
mod hs_playoffs;
mod kick_units;
mod leagues;
mod legacy;
mod level_rating;
mod links;
mod live_game;
mod polls;
mod ratings;
mod save;
mod schedule;
mod scouting;
mod stories;
mod storylines;
mod tactics;
mod tendencies;
mod training;
mod views;
mod win_chance;
mod world;

use crate::development::{SeasonContext, develop_season};
use crate::player::{Level, Player, PlayingTime, ProspectOptions, generate_prospect};
use crate::rng::Rng;

pub fn mean(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// A typical path for a good prospect: HS to 17, FBS to 21, NFL after.
pub fn context_for(age: u32) -> SeasonContext {
    let level = if age <= 17 {
        Level::HS
    } else if age <= 21 {
        Level::FBS
    } else {
        Level::NFL
    };
    let coaching = match level {
        Level::HS => 9.0,
        Level::FBS => 13.0,
        _ => 14.0,
    };
    SeasonContext::new(level, coaching, if age <= 15 { PlayingTime::Rotation } else { PlayingTime::Starter })
}

pub fn career(player: &mut Player, rng: &mut Rng, to_age: u32, tweak: impl Fn(&mut SeasonContext)) {
    while player.age < to_age {
        let mut ctx = context_for(player.age);
        tweak(&mut ctx);
        develop_season(player, &ctx, rng);
    }
}

pub fn prospects(n: usize, seed: &str, talent_z: f64) -> Vec<Player> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            let z = talent_z + rng.normal(0.0, 0.3);
            generate_prospect(&mut rng, ProspectOptions { talent_z: Some(z), ..Default::default() })
        })
        .collect()
}
