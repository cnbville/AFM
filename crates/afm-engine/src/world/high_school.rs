//! High schools: real powerhouses plus generated programs in each state (topic 2).

use super::data::StateRow;
use super::names;
use crate::rng::{Rng, clamp};

/// Powerhouses that recruit nationally, not just in-state (topic 2: "even out of state").
pub const NATIONAL_POWERHOUSES: [&str; 8] = [
    "IMG Academy",
    "Mater Dei",
    "St. John Bosco",
    "Bishop Gorman",
    "St. Frances Academy",
    "Duncanville",
    "Buford",
    "St. Thomas Aquinas",
];

#[derive(Clone, Debug)]
pub struct HsSeason {
    pub year: i32,
    pub wins: u32,
    pub losses: u32,
    pub state_title: bool,
    /// The head coach that season.
    pub coach: Option<usize>,
    /// Points scored and allowed, from engine seasons (zero on the stand-in).
    pub points: [u32; 2],
}

#[derive(Clone, Debug)]
pub struct HighSchool {
    pub id: usize,
    pub state: usize,
    pub town: String,
    /// The recruiting area (state and town) for area pipelines (note 4).
    pub area: usize,
    pub name: String,
    pub fictional_name: String,
    /// One of the state's real top programs.
    pub real: bool,
    pub private: bool,
    pub enrollment: u32,
    /// 0 = the state's biggest classification.
    pub classification: u32,
    /// Weight room, staff size, film, academic support: 0-1 (1.4).
    pub resources: f64,
    /// The area's typical household money and safety, 0-1 (1.1, 1.2).
    pub area_money: f64,
    pub area_safety: f64,
    /// Program prestige, 0-1. A powerhouse is its own exposure (1.5).
    pub prestige: f64,
    /// Recruits transfers from out of state too.
    pub national: bool,
    pub coach: Option<usize>,
    pub roster: Vec<usize>,
    pub seasons: Vec<HsSeason>,
    pub state_titles: u32,
}

impl HighSchool {
    pub fn is_powerhouse(&self) -> bool {
        self.prestige >= 0.75
    }
}

/// How a state's transfer rule treats a transfer without a family move: the
/// share of a varsity season he sits. Read from the rule's summary in the data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransferRule {
    pub sit_out: f64,
    /// A one-time free transfer exists.
    pub free_once: bool,
}

pub fn transfer_rule(summary: &str) -> TransferRule {
    let s = summary.to_lowercase();
    let free_once = s.contains("one-time") || s.contains("free transfer") && !s.contains("no free transfer");
    let sit_out = if s.contains("50%") || s.contains("half") {
        0.5
    } else if s.contains("365")
        || s.contains("one year")
        || s.contains("1 year")
        || s.contains("calendar year")
        || s.contains("sit a year")
    {
        1.0
    } else if s.contains("sit") {
        0.5
    } else {
        0.25
    };
    TransferRule { sit_out, free_once }
}

#[derive(Clone, Copy)]
enum Area {
    UrbanCore,
    Suburb,
    SmallTown,
    Rural,
}

/// Generates a state's high schools. The state's real top programs keep their
/// names; the rest are generated in towns from the data.
pub fn generate_state_schools(
    state_idx: usize,
    row: &StateRow,
    scale: f64,
    first_id: usize,
    rng: &mut Rng,
) -> Vec<HighSchool> {
    let n = ((row.programs as f64 * scale).round() as usize).max(row.powerhouses.len().min(2)).max(2);
    let mut towns: Vec<String> = row.top_hometowns.clone();
    for (_, t) in &row.powerhouses {
        if !t.is_empty() && !towns.contains(t) {
            towns.push(t.clone());
        }
    }
    if towns.is_empty() {
        towns.push(row.code.clone());
    }
    let mut schools = Vec::with_capacity(n);
    let mut per_town: Vec<usize> = vec![0; towns.len()];
    for k in 0..n {
        let id = first_id + k;
        let real = k < row.powerhouses.len();
        let (name, town) = if real {
            let (name, town) = &row.powerhouses[k];
            (name.clone(), if town.is_empty() { towns[0].clone() } else { town.clone() })
        } else {
            let t = rng.int(0, towns.len() as i32 - 1) as usize;
            per_town[t] += 1;
            (String::new(), towns[t].clone())
        };
        let area =
            rng.weighted(&[(Area::UrbanCore, 0.2), (Area::Suburb, 0.35), (Area::SmallTown, 0.25), (Area::Rural, 0.2)]);
        let (money, safety) = match area {
            Area::UrbanCore => (rng.range(0.15, 0.5), rng.range(0.15, 0.55)),
            Area::Suburb => (rng.range(0.45, 0.9), rng.range(0.6, 0.95)),
            Area::SmallTown => (rng.range(0.3, 0.6), rng.range(0.55, 0.9)),
            Area::Rural => (rng.range(0.2, 0.55), rng.range(0.6, 0.95)),
        };
        let private = if real { rng.chance(0.45) } else { rng.chance(0.12) };
        let enrollment = (match area {
            Area::UrbanCore | Area::Suburb => 1400.0,
            Area::SmallTown => 600.0,
            Area::Rural => 250.0,
        } * rng.log_normal(0.5)) as u32;
        let enrollment = if real { enrollment.max(1500) } else { enrollment.max(60) };
        let national = real && NATIONAL_POWERHOUSES.iter().any(|p| name.starts_with(p));
        let prestige = if real {
            clamp(0.8 + 0.04 * (5 - k.min(5)) as f64 + if national { 0.08 } else { 0.0 }, 0.0, 1.0)
        } else {
            clamp(0.15 + 0.3 * (enrollment as f64 / 2500.0).min(1.0) + rng.normal(0.0, 0.12), 0.02, 0.72)
        };
        let resources = clamp(
            0.25 * money
                + 0.2 * (enrollment as f64 / 2500.0).min(1.0)
                + 0.45 * prestige
                + if private { 0.12 } else { 0.0 }
                + rng.normal(0.0, 0.07),
            0.02,
            1.0,
        );
        let t_idx = towns.iter().position(|t| *t == town).unwrap_or(0);
        let name =
            if real { name } else { names::high_school_name(&town, per_town[t_idx].saturating_sub(1), private, rng) };
        let fictional_name = names::high_school_name(&town, k % 7, private, rng);
        schools.push(HighSchool {
            id,
            state: state_idx,
            town,
            area: 0,
            name,
            fictional_name,
            real,
            private,
            enrollment,
            classification: 0,
            resources,
            area_money: money,
            area_safety: safety,
            prestige,
            national,
            coach: None,
            roster: Vec::new(),
            seasons: Vec::new(),
            state_titles: 0,
        });
    }
    // Classifications by enrollment, biggest first, as in most states.
    let classes = row.number_of_classes.max(1) as usize;
    let mut order: Vec<usize> = (0..schools.len()).collect();
    order.sort_by(|&a, &b| schools[b].enrollment.cmp(&schools[a].enrollment));
    for (rank, &i) in order.iter().enumerate() {
        schools[i].classification = (rank * classes / schools.len().max(1)) as u32;
    }
    schools
}
