//! A college's real football buildings (facilities lane, Cain's yes
//! 2026-11-08): every FBS program's facilities number through the world's
//! past comes from its real buildings and when they opened, not from its
//! stadium seats, and each real project goes in the books the year ground
//! broke, so it pulls recruits from then on (`staff::facilities_reads`).
//!
//! The number in a past season is the level's everyday base plus the
//! buildings that stood that season:
//!
//!   level scale × (BASE + BASE_POWER if a power program)
//!     + PER_ROOT × √(Σ value × ½^(age / HALF_LIFE))
//!
//! A building's value is its real cost in 2024 dollars on the college
//! spending index (`build_index`: a $10M building in 1990 stood out the way
//! a $55M one does now), times its kind's share (`share`). The root
//! keeps one huge building from outweighing a full set of good ones, and the
//! half-life is the arms race: a 2005 building no longer shows a recruit what
//! a 2024 one does. From the season the world hands over, the world's own
//! model runs on (`staff::update_programs`, `finance::college`): buildings
//! age 0.006 a year and the ADs decide new projects.
//!
//! The data: `data/real-world/college_facilities.csv`, every FBS program's
//! football buildings, renovations, indoor practice facilities and the
//! stadium projects that rebuilt the team's own rooms, with cost, the year
//! ground broke and the year it opened, each with its source (the README
//! says how it was gathered).

use std::collections::BTreeMap;
use std::sync::LazyLock;

use super::data::{self, Division};
use super::program::Program;
use crate::rng::clamp;

const CSV: &str = include_str!("../../../../data/real-world/college_facilities.csv");

/// Every level's everyday facilities (fields, a weight room, a locker room,
/// the training room) before any building in the data, times the level's
/// scale (`program::division_scale`). Fitted so the FBS spread in the
/// handover season matches the seat-based start it replaces (item 3, keep
/// the size): proposed, not confirmed.
pub const BASE: f64 = 0.262;
/// What a power program's budget adds every year on top (nutrition, sports
/// medicine, upkeep: the everyday spending the data's projects don't list),
/// fitted with `BASE` to the seat-based start's power and Group of 5 means
/// (proposed, not confirmed).
pub const BASE_POWER: f64 = 0.020;
/// The number's rise per square root of a million 2024 dollars of standing
/// buildings (fitted with `BASE` to the seat-based start's spread; proposed,
/// not confirmed).
pub const PER_ROOT: f64 = 0.0534;
/// Years for a building to lose half its pull (proposed, not confirmed).
pub const HALF_LIFE: f64 = 15.0;

/// College athletics spending against 2024, the year ground broke: the
/// college cost index (`finance::data::college_index`) without its 2020-21
/// dip, when COVID cut the money for a year but not what a building cost.
pub fn build_index(year: i32) -> f64 {
    super::finance::data::interp(
        &[
            (1984, 0.12),
            (1990, 0.18),
            (2000, 0.33),
            (2005, 0.45),
            (2010, 0.60),
            (2015, 0.75),
            (2019, 0.88),
            (2024, 1.0),
        ],
        year,
        0.04,
    )
}

/// What a project is (the data's `kind` column).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// A football-only operations center or complex.
    FootballBuilding,
    /// An indoor practice facility or field house.
    IndoorPractice,
    /// A renovation of the team's rooms or of the football building.
    Renovation,
    /// A multi-sport performance or athletics center the team calls home.
    SharedAthletics,
    /// Outdoor practice fields.
    OutdoorPractice,
    /// A new stadium or a stadium renovation.
    Stadium,
}

impl Kind {
    fn parse(s: &str) -> Option<Kind> {
        Some(match s.trim() {
            "football_building" => Kind::FootballBuilding,
            "indoor_practice" => Kind::IndoorPractice,
            "renovation" => Kind::Renovation,
            "shared_athletics" => Kind::SharedAthletics,
            "outdoor_practice" => Kind::OutdoorPractice,
            "stadium" => Kind::Stadium,
            _ => return None,
        })
    }

    /// Years from ground breaking to opening when the data has no date.
    fn build_years(self) -> i32 {
        match self {
            Kind::FootballBuilding | Kind::SharedAthletics | Kind::Stadium => 2,
            Kind::IndoorPractice | Kind::Renovation | Kind::OutdoorPractice => 1,
        }
    }

    /// The words for the news ("breaks ground on its new ...").
    pub fn words(self) -> &'static str {
        match self {
            Kind::FootballBuilding => "football building",
            Kind::IndoorPractice => "indoor practice facility",
            Kind::Renovation => "renovation of the football rooms",
            Kind::SharedAthletics => "athletics performance center",
            Kind::OutdoorPractice => "practice fields",
            Kind::Stadium => "stadium project",
        }
    }
}

/// What a stadium project rebuilt for the team itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeamRooms {
    /// Seats, suites, a press box: nothing of the team's own.
    None,
    /// The team's own rooms (a locker room, a training room).
    Rooms,
    /// The team's whole home: its offices, locker room and weight room
    /// (Washington's 2013 Husky Stadium).
    Home,
}

/// How much of a project's cost counts toward the football facilities a
/// recruit tours and a player trains in (proposed, not confirmed). A
/// multi-sport building counts half. A stadium counts only for what it built
/// for the team, and then a share: most of a stadium's money is seats,
/// suites and the bowl, which the stadium grade reads from the crowds.
pub fn share(kind: Kind, rooms: TeamRooms) -> f64 {
    match (kind, rooms) {
        (Kind::FootballBuilding | Kind::IndoorPractice | Kind::Renovation | Kind::OutdoorPractice, _) => 1.0,
        (Kind::SharedAthletics, _) => 0.5,
        (Kind::Stadium, TeamRooms::Home) => 0.35,
        (Kind::Stadium, TeamRooms::Rooms) => 0.15,
        (Kind::Stadium, TeamRooms::None) => 0.0,
    }
}

/// One real project.
#[derive(Clone, Debug)]
pub struct Building {
    pub school: String,
    pub project: String,
    pub kind: Kind,
    pub rooms: TeamRooms,
    /// Dollars of the day, when a figure is known.
    pub cost: Option<f64>,
    /// The year ground broke (the data's, else the opening less the kind's
    /// usual build time).
    pub groundbreak: i32,
    pub opened: i32,
    /// Where the figures come from (a Wikipedia article or a news page).
    pub source: String,
    /// The project whose figure holds its cost, when the sources give one
    /// sum for both (Northwestern's Ryan Fieldhouse inside the Walter
    /// Athletics Center's $270M): the money is booked once, with that one.
    pub cost_in: Option<String>,
    /// Its value in millions of 2024 dollars before its kind's share: the
    /// real cost on the spending index (`build_index`), else the median of the same
    /// kind at the same tier (power or not) in the data. A project inside
    /// another's figure (`cost_in`) takes the median out of that figure, so
    /// each dollar counts once, at the share of what it built.
    pub value_2024: f64,
}

impl Building {
    /// Its pull in a season, in millions of 2024 dollars (0 before it opens).
    pub fn standing(&self, season: i32) -> f64 {
        if season < self.opened {
            return 0.0;
        }
        let age = (season - self.opened) as f64;
        self.value_2024 * share(self.kind, self.rooms) * 0.5f64.powf(age / HALF_LIFE)
    }

    /// Its cost in the books: the real figure, else the fallback value back
    /// on the index of the year ground broke; nothing for a project whose
    /// money is in another's figure (`cost_in`).
    pub fn book_cost(&self) -> f64 {
        if self.cost_in.is_some() {
            return 0.0;
        }
        self.cost.unwrap_or(self.value_2024 * 1.0e6 * build_index(self.groundbreak))
    }
}

fn power_school(name: &str) -> bool {
    name == "Notre Dame"
        || data::COLLEGES
            .iter()
            .find(|r| r.name == name)
            .is_some_and(|r| matches!(r.conference.as_str(), "SEC" | "Big Ten" | "Big 12" | "ACC"))
}

fn parse(text: &str) -> BTreeMap<String, Vec<Building>> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header = data::split_csv_line(lines.next().unwrap_or(""));
    let col = |n: &str| header.iter().position(|h| h == n).unwrap_or_else(|| panic!("missing column {n}"));
    let (school, project, kind, team, home, cost, ground, opened, source) = (
        col("school"),
        col("project"),
        col("kind"),
        col("team_spaces"),
        col("houses_football_ops"),
        col("cost_musd"),
        col("groundbreak"),
        col("opened"),
        col("source"),
    );
    let cost_in = header.iter().position(|h| h == "cost_in");
    let yes = |s: &str| matches!(s.trim(), "true" | "yes" | "1");
    let mut rows: Vec<(Building, bool)> = Vec::new();
    for line in lines {
        let r = data::split_csv_line(line);
        let Some(k) = Kind::parse(&r[kind]) else { continue };
        let Ok(open) = r[opened].trim().parse::<i32>() else { continue };
        let musd = r[cost].trim().parse::<f64>().ok().filter(|c| *c > 0.0);
        let ground = r[ground].trim().parse::<i32>().ok().filter(|g| *g <= open).unwrap_or(open - k.build_years());
        let b = Building {
            school: r[school].trim().to_string(),
            project: r[project].trim().to_string(),
            kind: k,
            rooms: match (yes(&r[team]), yes(&r[home])) {
                (_, true) => TeamRooms::Home,
                (true, false) => TeamRooms::Rooms,
                _ => TeamRooms::None,
            },
            cost: musd.map(|m| m * 1.0e6),
            groundbreak: ground,
            opened: open,
            source: r[source].trim().to_string(),
            cost_in: cost_in.and_then(|c| r.get(c)).map(|c| c.trim().to_string()).filter(|c| !c.is_empty()),
            value_2024: musd.map_or(0.0, |m| m / build_index(ground)),
        };
        let power = power_school(&b.school);
        rows.push((b, power));
    }
    // A project with no cost found counts as the median of its kind at its
    // tier (power or not) in the data.
    let median = |k: Kind, power: bool| {
        let mut v: Vec<f64> = rows
            .iter()
            .filter(|(b, p)| b.kind == k && *p == power && b.cost.is_some())
            .map(|(b, _)| b.value_2024)
            .collect();
        if v.is_empty() {
            v = rows.iter().filter(|(b, _)| b.kind == k && b.cost.is_some()).map(|(b, _)| b.value_2024).collect();
        }
        v.sort_by(f64::total_cmp);
        v.get(v.len() / 2).copied().unwrap_or(10.0)
    };
    let mut out: BTreeMap<String, Vec<Building>> = BTreeMap::new();
    let mut filled: Vec<Building> = rows
        .iter()
        .map(|(b, power)| {
            let mut b = b.clone();
            if b.cost.is_none() {
                b.value_2024 = median(b.kind, *power);
            }
            b
        })
        .collect();
    // A project inside another's figure takes its value out of that figure.
    for k in 0..filled.len() {
        let Some(parent) = filled[k].cost_in.clone() else { continue };
        let (school, value) = (filled[k].school.clone(), filled[k].value_2024);
        if let Some(j) = filled.iter().position(|x| x.school == school && x.project == parent) {
            filled[j].value_2024 = (filled[j].value_2024 - value).max(0.0);
        }
    }
    for b in filled {
        out.entry(b.school.clone()).or_default().push(b);
    }
    for v in out.values_mut() {
        v.sort_by(|a, b| (a.opened, a.groundbreak, &a.project).cmp(&(b.opened, b.groundbreak, &b.project)));
    }
    out
}

/// Every school's real projects, by its name in `colleges.csv`.
pub static BUILDINGS: LazyLock<BTreeMap<String, Vec<Building>>> = LazyLock::new(|| parse(CSV));

/// A school's real projects (none for a school outside the data).
pub fn of(name: &str) -> &'static [Building] {
    BUILDINGS.get(name).map_or(&[], |v| v.as_slice())
}

/// Whether a school's past facilities come from its real buildings.
pub fn has_buildings(name: &str) -> bool {
    BUILDINGS.contains_key(name)
}

/// Its buildings' pull in a season, in millions of 2024 dollars, leaving
/// out one project (`skip`, an index into `of`).
fn standing(name: &str, season: i32, skip: Option<usize>) -> f64 {
    of(name).iter().enumerate().filter(|(k, _)| Some(*k) != skip).map(|(_, b)| b.standing(season)).sum()
}

fn number_from(division: Division, power: bool, standing: f64) -> f64 {
    let base = super::program::division_scale(division) * (BASE + if power { BASE_POWER } else { 0.0 });
    clamp(base + PER_ROOT * standing.max(0.0).sqrt(), 0.02, 1.0)
}

/// A program's facilities number in a past season from its real buildings,
/// at the level it plays that season; `None` for a school outside the data.
pub fn number(p: &Program, season: i32) -> Option<f64> {
    if !has_buildings(&p.real_name) {
        return None;
    }
    Some(number_from(p.division, p.power_conference(season), standing(&p.real_name, season, None)))
}

/// What one real project adds to the number when it opens (its `step` in
/// the books): the number that season with it, less without it.
pub fn step(p: &Program, k: usize) -> f64 {
    let Some(b) = of(&p.real_name).get(k) else { return 0.0 };
    let power = p.power_conference(b.opened);
    let with = number_from(p.division, power, standing(&p.real_name, b.opened, None));
    let without = number_from(p.division, power, standing(&p.real_name, b.opened, Some(k)));
    (with - without).max(0.0)
}
