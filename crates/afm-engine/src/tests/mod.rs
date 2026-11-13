//! Engine tests, ported from the TypeScript model. Calibration of a whole
//! national class lives in afm-tools (it needs the debug summary).

mod assignments;
mod bodies;
mod carousel;
mod claims;
mod coaching_people;
mod compact;
mod decide;
mod depth;
mod development;
mod faces;
mod facilities;
mod fans;
mod foundations;
mod front_office;
mod game_stats;
mod gameplan;
mod growth_reps;
mod hiring;
mod hs_playoffs;
mod hunt;
mod jan1;
mod kick_units;
mod lane7;
mod lane9;
mod leagues;
pub(crate) mod legacy;
mod level_rating;
mod line_order;
mod line_seasons;
mod lines;
mod links;
mod live_core;
mod live_gain;
mod live_game;
mod live_money;
mod live_one_on_one;
mod live_recruit;
mod marks;
mod new_teams;
mod nil;
mod no_fire;
mod outside_talks;
mod pipeline;
mod placement;
mod polls;
mod portal_recruiting;
mod portal_talks;
mod praise;
mod ratings;
mod rooms;
mod roster_moves;
mod sandbox;
mod save;
mod schedule;
mod scouting;
mod split_build;
mod stale_games;
mod stories;
mod storylines;
mod tactics;
mod talk_adviser;
mod talks_matter;
mod tallies;
mod tendencies;
mod training;
mod update3;
mod views;
mod waivers;
mod win_chance;
mod world;
mod world_log;

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
