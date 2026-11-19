//! Engine tests, ported from the TypeScript model. Calibration of a whole
//! national class lives in afm-tools (it needs the debug summary).

mod ai_promises;
mod assignments;
mod board_fit;
mod bodies;
mod bonds;
mod capital;
mod capital_drive;
mod capital_race;
mod capital_rooms;
mod capital_seats;
mod capital_week;
mod carousel;
mod cfl_draft;
mod chain;
mod claims;
mod coach_fire;
mod coach_market;
mod coaching_people;
mod compact;
mod craft;
mod craft_work;
mod decide;
mod depth;
mod development;
mod discipline;
mod faces;
mod facilities;
mod fans;
mod finder;
mod following;
mod foundations;
mod fresh_talks;
mod front_office;
mod game_stats;
mod gameday_room;
mod gameplan;
mod ground;
mod ground_areas;
mod ground_campaign;
mod ground_favors;
mod ground_leads;
mod ground_people;
mod ground_race;
mod ground_scenes;
mod ground_staff;
mod ground_week;
mod growth_reps;
mod handoff;
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
mod live_rooms;
mod manner;
mod marks;
mod matters_here;
mod negotiation_college;
mod negotiation_floor;
mod negotiation_pro;
mod negotiation_recruit;
mod new_teams;
mod nil;
mod nil_multi;
mod no_fire;
mod offer_days;
mod one_on_ones;
mod outside_talks;
mod persona;
mod persona_coaching;
mod persona_deals;
mod persona_draft;
mod persona_evaluate;
mod persona_negotiate;
mod persona_office;
mod persona_people;
mod persona_standing;
mod persona_year;
mod pipeline;
mod placement;
mod playing_time_grade;
mod polls;
mod portal_recruiting;
mod portal_talks;
mod praise;
mod progression;
mod promises;
mod public_bar;
mod pull_file;
mod ratings;
mod real_injuries;
mod recruit_budget;
mod recruit_case;
mod recruit_reach;
mod review;
mod roles;
mod rooms;
mod roster_moves;
mod sandbox;
mod save;
mod schedule;
mod scouting;
mod split_build;
mod stale_games;
mod stay_read;
mod stories;
mod storylines;
mod strength_split;
mod support_departments;
mod support_market;
mod tactics;
mod talk_adviser;
mod talks_matter;
mod tallies;
mod team_rooms;
mod tendencies;
mod testing_book;
mod training;
mod update3;
mod usage;
mod usage_terms;
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
