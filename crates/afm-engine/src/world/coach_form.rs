//! Coaches as world files hold them (memory diet, lane M2, item 9): every
//! field flat, by name, in the order files always had, while in memory a
//! coach's working part (`coach::Work`) sits behind a box that a retired
//! coach no longer keeps. A retired coach is written without the working
//! part's fields; a file that has them (every world written before) reads
//! them back, and a retired coach's are dropped as he loads. Both forms
//! name every field, so a field added to `Coach` or `Work` without them
//! doesn't build.

use std::collections::HashMap;

use super::coach::*;
use super::ethnicity::Ethnicity;

/// A coach as world files hold him.
#[derive(serde::Deserialize)]
#[serde(rename = "Coach")]
pub(crate) struct File {
    id: usize,
    name: String,
    ethnicity: Ethnicity,
    age: u32,
    #[serde(default = "crate::calendar::born::unknown")]
    born: crate::calendar::Date,
    ability: Ability,
    #[serde(default)]
    learning_ceiling: Option<f64>,
    reputation: f64,
    personality: CoachPersonality,
    mentor: Option<usize>,
    ex_pro: bool,
    job: Option<Job>,
    history: Vec<Stint>,
    #[serde(default)]
    recent: Option<Vec<f64>>,
    wins: u32,
    losses: u32,
    titles: u32,
    career_titles: u32,
    retired: bool,
    #[serde(default)]
    prove_it: Option<i32>,
    #[serde(default)]
    ties: Option<HashMap<usize, f64>>,
    came_up: Group,
    #[serde(default)]
    units: Option<[UnitRecord; GROUP_COUNT]>,
    #[serde(default)]
    sportsmanship: Option<f64>,
    home_state: Option<usize>,
    #[serde(default)]
    decline_from: Option<f64>,
    #[serde(default)]
    decline_rate: Option<f64>,
    show_cause: Option<i32>,
    #[serde(default)]
    contract: Option<Contract>,
    #[serde(default)]
    systems: Option<super::systems::Systems>,
    #[serde(default)]
    stat_buzz: Option<f64>,
    #[serde(default)]
    style_set: Option<crate::tendencies::Style>,
    #[serde(default)]
    freedom_norm: Option<f64>,
    #[serde(default)]
    yours: Option<Yours>,
    #[serde(default)]
    played: Option<super::former::Played>,
    #[serde(default)]
    renown: f64,
    #[serde(default)]
    fame: f64,
    #[serde(default)]
    junior: Option<Junior>,
}

impl From<File> for Coach {
    fn from(f: File) -> Coach {
        let b = Work::default();
        // A file written without the working part (a retired coach) gives
        // a coach without one.
        let work = f.learning_ceiling.map(|learning_ceiling| {
            Box::new(Work {
                learning_ceiling,
                recent: f.recent.unwrap_or_default(),
                prove_it: f.prove_it,
                ties: f.ties.unwrap_or_default(),
                units: f.units.unwrap_or(b.units),
                sportsmanship: f.sportsmanship.unwrap_or(b.sportsmanship),
                decline_from: f.decline_from.unwrap_or(b.decline_from),
                decline_rate: f.decline_rate.unwrap_or(b.decline_rate),
                contract: f.contract,
                systems: f.systems.unwrap_or(b.systems),
                stat_buzz: f.stat_buzz.unwrap_or(b.stat_buzz),
                style_set: f.style_set,
                freedom_norm: f.freedom_norm.unwrap_or(b.freedom_norm),
                yours: f.yours,
            })
        });
        let mut c = Coach {
            id: f.id,
            name: f.name,
            ethnicity: f.ethnicity,
            age: f.age,
            born: f.born,
            reputation: f.reputation,
            mentor: f.mentor,
            ex_pro: f.ex_pro,
            job: f.job,
            history: f.history,
            wins: f.wins,
            losses: f.losses,
            titles: f.titles,
            career_titles: f.career_titles,
            retired: f.retired,
            came_up: f.came_up,
            home_state: f.home_state,
            show_cause: f.show_cause,
            played: f.played,
            renown: f.renown,
            fame: f.fame,
            junior: f.junior,
            ability: f.ability,
            personality: f.personality,
            work,
        };
        c.retire_work();
        c
    }
}

impl<'de> serde::Deserialize<'de> for Coach {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Coach, D::Error> {
        File::deserialize(d).map(Coach::from)
    }
}

/// A coach as he is written. A retired coach without his working part
/// writes it blank, so older games of the same format still read the file.
#[derive(serde::Serialize)]
#[serde(rename = "Coach")]
struct View<'a> {
    id: usize,
    name: &'a str,
    ethnicity: Ethnicity,
    age: u32,
    born: crate::calendar::Date,
    ability: &'a Ability,
    learning_ceiling: f64,
    reputation: f64,
    personality: &'a CoachPersonality,
    mentor: Option<usize>,
    ex_pro: bool,
    job: &'a Option<Job>,
    history: &'a Vec<Stint>,
    recent: &'a Vec<f64>,
    wins: u32,
    losses: u32,
    titles: u32,
    career_titles: u32,
    retired: bool,
    prove_it: &'a Option<i32>,
    ties: &'a HashMap<usize, f64>,
    came_up: Group,
    units: &'a [UnitRecord; GROUP_COUNT],
    sportsmanship: f64,
    home_state: Option<usize>,
    decline_from: f64,
    decline_rate: f64,
    show_cause: Option<i32>,
    contract: &'a Option<Contract>,
    systems: &'a super::systems::Systems,
    stat_buzz: f64,
    style_set: &'a Option<crate::tendencies::Style>,
    freedom_norm: f64,
    yours: &'a Option<Yours>,
    played: &'a Option<super::former::Played>,
    renown: f64,
    fame: f64,
    junior: &'a Option<Junior>,
}

impl serde::Serialize for Coach {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let w = self.work.as_deref().unwrap_or_else(|| super::coach::blank_work());
        View {
            id: self.id,
            name: &self.name,
            ethnicity: self.ethnicity,
            age: self.age,
            born: self.born,
            ability: &self.ability,
            learning_ceiling: w.learning_ceiling,
            reputation: self.reputation,
            personality: &self.personality,
            mentor: self.mentor,
            ex_pro: self.ex_pro,
            job: &self.job,
            history: &self.history,
            recent: &w.recent,
            wins: self.wins,
            losses: self.losses,
            titles: self.titles,
            career_titles: self.career_titles,
            retired: self.retired,
            prove_it: &w.prove_it,
            ties: &w.ties,
            came_up: self.came_up,
            units: &w.units,
            sportsmanship: w.sportsmanship,
            home_state: self.home_state,
            decline_from: w.decline_from,
            decline_rate: w.decline_rate,
            show_cause: self.show_cause,
            contract: &w.contract,
            systems: &w.systems,
            stat_buzz: w.stat_buzz,
            style_set: &w.style_set,
            freedom_norm: w.freedom_norm,
            yours: &w.yours,
            played: &self.played,
            renown: self.renown,
            fame: self.fame,
            junior: &self.junior,
        }
        .serialize(s)
    }
}
