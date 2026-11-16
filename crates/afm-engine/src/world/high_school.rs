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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HsSeason {
    pub year: i32,
    pub wins: u32,
    pub losses: u32,
    pub state_title: bool,
    /// The head coach that season.
    pub coach: Option<usize>,
    /// Points scored and allowed, from engine seasons (zero on the stand-in).
    pub points: [u32; 2],
    /// Playoff games won (U-d; Wisconsin's tournament performance factor).
    #[serde(default)]
    pub playoff_wins: u8,
    /// Reached the last four of its bracket (U-d; Connecticut's play-up rule).
    #[serde(default)]
    pub semifinal: bool,
    /// The bracket it won, by its class's name (lane L20, item 25); none in
    /// an older file.
    #[serde(default)]
    pub title: Option<String>,
    /// The units' season from engine games (C42): what "coaches to watch"
    /// reads beside the points.
    #[serde(default)]
    pub unit: HsUnit,
    /// Schedule strength (minor bug 362): the opponents' average team
    /// strength that season, regular season and playoffs, against the
    /// average high school team's, in rating points, the way MaxPreps and
    /// CalPreps rate a schedule; none in an older file.
    #[serde(default)]
    pub sos: Option<f32>,
    /// Its passes as a share of its passes and runs, from engine games
    /// (attempts over attempts and carries, as a box score counts them):
    /// what a receiver's routes are read from (`footage`). None in an older
    /// file: its players' lines can't fill it, since the world drops
    /// everyone who left football, so a school's kept lines miss its
    /// graduated seniors' attempts and carries.
    #[serde(default)]
    pub pass_share: Option<f32>,
}

/// A high school season's unit totals (C42), compact: engine games only.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HsUnit {
    pub games: u8,
    pub sacks: u16,
    pub sacks_allowed: u16,
    pub takeaways: u16,
    pub turnovers: u16,
    pub yards: u32,
    pub yards_allowed: u32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HighSchool {
    pub id: usize,
    pub state: usize,
    pub town: String,
    /// The town's county (final build: real towns per county), empty for a
    /// world saved before it.
    #[serde(default)]
    pub county: String,
    /// The recruiting area (state and county, or town where the county is
    /// unknown) for area pipelines (note 4) and nearby schedules.
    pub area: usize,
    /// Ground's area (`ground::areas`): staffs' areas, not the county. Read
    /// from the shared table when the world is built and when it is read.
    #[serde(skip)]
    pub ground_area: u16,
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
    /// Its coordinators (C37): real coaches with ratings. Your own school's
    /// staff is yours (`Career::hs_staff`).
    #[serde(default)]
    pub oc: Option<usize>,
    #[serde(default)]
    pub dc: Option<usize>,
    pub roster: Vec<usize>,
    pub seasons: Vec<HsSeason>,
    pub state_titles: u32,
    /// The chain reaction (Ground GR21): where its old players are and its
    /// prestige by year. Empty in a world saved before it, until
    /// `chain_after_load` fills its start.
    #[serde(default)]
    pub chain: Chain,
}

impl HighSchool {
    pub fn is_powerhouse(&self) -> bool {
        self.prestige >= 0.75
    }

    /// Its passes as a share of its plays that season, when kept.
    pub fn pass_share(&self, year: i32) -> Option<f64> {
        self.seasons.iter().rev().find(|s| s.year == year).and_then(|s| s.pass_share).map(f64::from)
    }
}

/// A pass share from a season's attempts and carries (none without plays).
pub fn pass_share_of(att: f64, carries: f64) -> Option<f32> {
    (att + carries > 0.0).then(|| (att / (att + carries)) as f32)
}

/// How a state's transfer rule treats a transfer without a family move: the
/// share of a varsity season he sits (1.0 a whole year; less, its first
/// games). Read from the rule's summary in the data.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TransferRule {
    pub sit_out: f64,
    /// A one-time free transfer exists.
    pub free_once: bool,
    /// A bona fide family move into the new school's district makes him
    /// eligible at once (every state but Arizona, whose transfers sit "even
    /// with a residence change"). True in a world saved before it.
    #[serde(default = "yes")]
    pub move_exempt: bool,
}

fn yes() -> bool {
    true
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
    let move_exempt = !s.contains("even with a residence change");
    TransferRule { sit_out, free_once, move_exempt }
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
enum Area {
    UrbanCore,
    Suburb,
    SmallTown,
    Rural,
}

/// A real town the state's generated schools are placed in
/// (`data/real-world/hs_towns.csv`, built by `scripts/hs_towns.py` from the
/// 2010 Census by ZIP code): its county, people, how many of the state's
/// schools it takes at most, people per square mile, median household
/// income and where it is (latitude and longitude, 0 when unknown).
#[derive(Clone, Debug)]
pub struct TownRow {
    pub town: String,
    pub county: String,
    pub population: u32,
    pub slots: u32,
    pub density: f32,
    pub income: f32,
    pub lat: f32,
    pub lon: f32,
}

const TOWNS_CSV: &str = include_str!("../../../../data/real-world/hs_towns.csv");

/// Each state's towns, biggest first.
static TOWNS: std::sync::LazyLock<std::collections::HashMap<String, Vec<TownRow>>> = std::sync::LazyLock::new(|| {
    let mut map: std::collections::HashMap<String, Vec<TownRow>> = std::collections::HashMap::new();
    for line in TOWNS_CSV.lines().skip(1) {
        let c = super::data::split_csv_line(line);
        if c.len() < 7 {
            continue;
        }
        let num = |k: usize| c[k].parse::<f32>().unwrap_or(0.0);
        map.entry(c[0].clone()).or_default().push(TownRow {
            town: c[1].clone(),
            county: c[2].clone(),
            population: num(3) as u32,
            slots: (num(4) as u32).max(1),
            density: num(5),
            income: num(6),
            lat: if c.len() > 8 { num(7) } else { 0.0 },
            lon: if c.len() > 8 { num(8) } else { 0.0 },
        });
    }
    map
});

/// Every town's place, by state code, then name (nested so a lookup needs no new strings:
/// recruiting reads a recruit's town for every school it weighs, minor bug
/// 255).
#[allow(clippy::type_complexity)]
static PLACES: std::sync::LazyLock<std::collections::HashMap<String, std::collections::HashMap<String, (f32, f32)>>> =
    std::sync::LazyLock::new(|| {
        let mut map: std::collections::HashMap<String, std::collections::HashMap<String, (f32, f32)>> =
            std::collections::HashMap::new();
        for (code, towns) in TOWNS.iter() {
            for t in towns.iter().filter(|t| t.lat != 0.0 || t.lon != 0.0) {
                map.entry(code.clone()).or_default().entry(t.town.clone()).or_insert((t.lat, t.lon));
            }
        }
        map
    });

/// A town's latitude and longitude, when the table has it.
pub fn town_place(code: &str, town: &str) -> Option<(f64, f64)> {
    PLACES.get(code).and_then(|m| m.get(town)).map(|&(a, b)| (a as f64, b as f64))
}

/// A state's real towns, biggest first (empty for a state the table lacks).
pub fn state_towns(code: &str) -> &'static [TownRow] {
    TOWNS.get(code).map_or(&[], |v| v.as_slice())
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
    // The state's real towns (final build): a county names its schools
    // for its towns, so every generated school sits in a real town, a big
    // city taking several. The data's hometowns and the powerhouses' towns
    // stay on the list.
    let table = state_towns(&row.code);
    let mut towns: Vec<String> = table.iter().map(|t| t.town.clone()).collect();
    for t in row.top_hometowns.iter().chain(row.powerhouses.iter().map(|p| &p.1)) {
        if !t.is_empty() && !towns.contains(t) {
            towns.push(t.clone());
        }
    }
    if towns.is_empty() {
        towns.push(row.code.clone());
    }
    let info = |t: usize| table.get(t);
    // Each town's places, in a scrambled order (its own stream, from one
    // draw of the world's), so a small world spreads over the state.
    let mut order_rng = Rng::new(rng.next_u32() ^ 0x70_3E5);
    let mut slots: Vec<usize> = Vec::new();
    for t in 0..towns.len() {
        let n = info(t).map_or(1, |r| r.slots as usize);
        slots.extend(std::iter::repeat_n(t, n));
    }
    for i in (1..slots.len()).rev() {
        let j = order_rng.int(0, i as i32) as usize;
        slots.swap(i, j);
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
            // The draw stays so the rest of the stream does not move.
            let _ = rng.int(0, towns.len() as i32 - 1);
            let t = slots[(k - row.powerhouses.len()) % slots.len().max(1)];
            per_town[t] += 1;
            (String::new(), towns[t].clone())
        };
        let t_idx = towns.iter().position(|t| *t == town).unwrap_or(0);
        let place = info(t_idx).filter(|r| r.town == town);
        let drawn =
            rng.weighted(&[(Area::UrbanCore, 0.2), (Area::Suburb, 0.35), (Area::SmallTown, 0.25), (Area::Rural, 0.2)]);
        // The town decides what kind of place the school is in (final
        // build): a dense city's schools are urban or suburban, a small town
        // is a small town, a village is rural. A big city still mixes its
        // inner-city and outer schools, as drawn.
        let area = match place {
            None => drawn,
            Some(r) if r.slots >= 3 => {
                if drawn == Area::UrbanCore || (r.density >= 4000.0 && drawn != Area::Suburb) {
                    Area::UrbanCore
                } else {
                    Area::Suburb
                }
            }
            Some(r) if r.density >= 2500.0 && r.population >= 50_000 => Area::UrbanCore,
            Some(r) if r.density >= 800.0 && r.population >= 10_000 => Area::Suburb,
            Some(r) if r.population >= 2_500 => Area::SmallTown,
            Some(_) => Area::Rural,
        };
        let (money, safety) = match area {
            Area::UrbanCore => (rng.range(0.15, 0.5), rng.range(0.15, 0.55)),
            Area::Suburb => (rng.range(0.45, 0.9), rng.range(0.6, 0.95)),
            Area::SmallTown => (rng.range(0.3, 0.6), rng.range(0.55, 0.9)),
            Area::Rural => (rng.range(0.2, 0.55), rng.range(0.6, 0.95)),
        };
        // A real school is private by what it is (its name says so); the draw
        // stays so the rest of the stream does not move.
        let draw = if real { rng.chance(0.45) } else { rng.chance(0.12) };
        let private = if real { super::hs_playoffs::real_private(&row.powerhouses[k].0) } else { draw };
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
        let name =
            if real { name } else { names::high_school_name(&town, per_town[t_idx].saturating_sub(1), private, rng) };
        // The town's household money moves the area's (2010 Census median
        // household income against the national $51,914).
        let money = match place {
            Some(r) if r.income > 0.0 => {
                clamp(0.5 * money + 0.5 * clamp(0.5 + 0.6 * (r.income as f64 / 51_914.0).ln(), 0.05, 0.95), 0.0, 1.0)
            }
            _ => money,
        };
        let county = place.map_or_else(String::new, |r| r.county.clone());
        let fictional_name = names::high_school_name(&town, k % 7, private, rng);
        schools.push(HighSchool {
            id,
            state: state_idx,
            town,
            county,
            area: 0,
            ground_area: 0,
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
            oc: None,
            dc: None,
            roster: Vec::new(),
            seasons: Vec::new(),
            state_titles: 0,
            chain: Chain::new(prestige),
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

/// The chain reaction (Ground GR21, Cain's idea): a high school whose old
/// players do well in college and the NFL climbs, and one that stops
/// producing slowly falls back. Its prestige is its exposure to the
/// services and the staffs (`recruiting::recruit_visibility`) and what draws
/// transfers and move-ins (`recruiting::high_school_transfers`), so the
/// climb feeds itself through the kids, never by a flat rule.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Chain {
    /// Its name: its prestige when the world began, from the build (its
    /// tradition, money and pull), plus what its town built since. Its
    /// prestige swings around it with its players' form.
    pub start: f64,
    /// Its old players' form: how far their score has sat above or below
    /// what its prestige predicts in its state, in standard deviations,
    /// smoothed over the years (`state_form`).
    #[serde(default)]
    pub form: f32,
    /// This June's alumni score (`alumni_value`) and what it counts: old
    /// players in college, at power programs, and in the NFL.
    pub made: f32,
    pub college: u16,
    pub power: u16,
    pub nfl: u16,
    /// Its prestige each June 15, after the step: (year, prestige).
    pub years: Vec<(i32, f32)>,
}

impl Chain {
    pub fn new(prestige: f64) -> Chain {
        Chain { start: prestige, ..Default::default() }
    }
}

/// The chain reaction's sizes (proposed, not confirmed): a school's
/// prestige is its name plus 0.12 for each standard deviation of form, so
/// at most 0.3 either way (form is read up to 2.5 deviations). Form moves
/// 30% of the way to this June's reading when it rises and 15% when it
/// falls: a climb takes hold in about three years, and a name fades over
/// about six.
pub const CLIMB_SWING: f64 = 0.12;
pub const CLIMB_CAP: f64 = 2.5;
pub const FORM_UP: f64 = 0.3;
pub const FORM_DOWN: f64 = 0.15;
/// A state with fewer schools than this holds its prestige: too few to
/// read what a school's name predicts (proposed, not confirmed).
pub const CLIMB_MIN_SCHOOLS: usize = 8;

/// What one old player is worth to his high school's name this June
/// (proposed, not confirmed): in college, 1.0 at a power program, 0.6 in
/// the rest of FBS, 0.3 in FCS and 0.1 lower, half again as much if he
/// started last season; in the NFL, 2.0 on a roster or injured reserve,
/// 3.0 if he started half of last season, 0.8 on a practice squad, 0.3 in
/// another league. Players out of football count nothing: a school lives
/// on the ones still playing. Returns the value, and whether he is at a
/// power program and whether in the NFL.
pub fn alumni_value(w: &super::World, p: &super::Person) -> Option<(f64, bool, bool)> {
    use super::Stage;
    use super::data::Division;
    use super::pro::Status;
    match p.stage {
        Stage::College | Stage::Declared => {
            let c = &w.programs[p.college?];
            let power = c.division == Division::FBS && c.power_conference(w.year);
            let level = match c.division {
                Division::FBS if power => 1.0,
                Division::FBS => 0.6,
                Division::FCS => 0.3,
                _ => 0.1,
            };
            let started = p.last_role == Some(crate::player::PlayingTime::Starter);
            Some((level * if started { 1.5 } else { 1.0 }, power, false))
        }
        Stage::Pro => {
            let pro = p.pro.as_ref()?;
            let v = match pro.status {
                Status::Roster | Status::Reserve if pro.team.is_some() => {
                    let starts = pro.seasons.last().map_or(0, |s| s.starts);
                    if starts >= 9 { 3.0 } else { 2.0 }
                }
                Status::PracticeSquad if pro.team.is_some() => 0.8,
                Status::Other(_) => 0.3,
                _ => return None,
            };
            Some((v, false, matches!(pro.status, Status::Roster | Status::Reserve | Status::PracticeSquad)))
        }
        _ => None,
    }
}

/// The chain reaction's step, each June 15 before the transfers: every
/// school's old players are counted; in each state the score is read
/// against what each school's prestige predicts there (`state_form`), its
/// form moves toward that reading, and its prestige is its name plus its
/// form (`CLIMB_SWING`). A name built on tradition and money doesn't
/// vanish with a few thin classes, and a school can't climb without end on
/// one good run. It reads only the world, draws nothing, and posts a line
/// when a school becomes a powerhouse or stops being one.
pub(crate) fn climb(w: &mut super::World) {
    use rayon::prelude::*;
    let n = w.high_schools.len();
    let found: Vec<(usize, f64, bool, bool)> = w
        .people
        .par_iter()
        .with_min_len(4096)
        .filter_map(|p| alumni_value(w, p).map(|(v, power, nfl)| (p.hs, v, power, nfl)))
        .collect();
    let mut made = vec![(0.0f64, 0u16, 0u16, 0u16); n];
    for (h, v, power, nfl) in found {
        if let Some(m) = made.get_mut(h) {
            m.0 += v;
            if nfl {
                m.3 = m.3.saturating_add(1);
            } else {
                m.1 = m.1.saturating_add(1);
                m.2 = m.2.saturating_add(power as u16);
            }
        }
    }
    let year = w.year;
    let mut news: Vec<(usize, bool)> = Vec::new();
    for s in 0..w.states.len() {
        let schools = w.states[s].schools.clone();
        let reads = state_form(&schools.iter().map(|&h| (w.high_schools[h].prestige, made[h].0)).collect::<Vec<_>>());
        for (k, &h) in schools.iter().enumerate() {
            let school = &mut w.high_schools[h];
            let was = school.prestige;
            if let Some(z) = reads[k] {
                let f = school.chain.form as f64;
                let rate = if z > f { FORM_UP } else { FORM_DOWN };
                school.chain.form = (f + rate * (z - f)) as f32;
                school.prestige = clamp(school.chain.start + CLIMB_SWING * school.chain.form as f64, 0.02, 1.0);
            }
            let (v, college, power, nfl) = made[h];
            school.chain.made = v as f32;
            school.chain.college = college;
            school.chain.power = power;
            school.chain.nfl = nfl;
            school.chain.years.push((year, school.prestige as f32));
            if (was >= 0.75) != school.is_powerhouse() {
                news.push((h, school.is_powerhouse()));
            }
        }
    }
    if w.recording {
        let d = super::hs_year::transfer_day(year);
        for (h, up) in news {
            let school = &w.high_schools[h];
            let name = w.school_name(h).to_string();
            let code = &w.states[school.state].code;
            let (college, nfl) = (school.chain.college, school.chain.nfl);
            let text = if up {
                format!(
                    "{name} is now one of {code}'s powerhouses: {college} of its old players are in college and {nfl} in the NFL."
                )
            } else {
                format!(
                    "{name} has slipped out of {code}'s powerhouses: {college} of its old players are in college and {nfl} in the NFL."
                )
            };
            let e = crate::calendar::Event::new(
                d,
                crate::calendar::Level::HighSchool,
                crate::calendar::Kind::News,
                crate::calendar::Loudness::Small,
                text,
            )
            .link(crate::calendar::Link::School(h as u32), &name);
            w.calendar.post(e);
        }
    }
}

/// This June's reading of each school's form from (prestige, old players'
/// score) for one state's schools, in order: its score against the state's
/// straight-line fit of score on prestige, in standard deviations, at most
/// `CLIMB_CAP` either way. None in a small state (`CLIMB_MIN_SCHOOLS`) or
/// when every school scores alike: then nobody's form moves.
pub fn state_form(schools: &[(f64, f64)]) -> Vec<Option<f64>> {
    let n = schools.len();
    if n < CLIMB_MIN_SCHOOLS {
        return vec![None; n];
    }
    let nf = n as f64;
    let mx = schools.iter().map(|s| s.0).sum::<f64>() / nf;
    let my = schools.iter().map(|s| s.1).sum::<f64>() / nf;
    let sxx: f64 = schools.iter().map(|s| (s.0 - mx).powi(2)).sum();
    let sxy: f64 = schools.iter().map(|s| (s.0 - mx) * (s.1 - my)).sum();
    let b = if sxx > 1e-12 { sxy / sxx } else { 0.0 };
    let resid: Vec<f64> = schools.iter().map(|s| s.1 - (my + b * (s.0 - mx))).collect();
    let sd = (resid.iter().map(|r| r * r).sum::<f64>() / nf).sqrt();
    if sd < 1e-9 {
        return vec![None; n];
    }
    resid.iter().map(|r| Some((r / sd).clamp(-CLIMB_CAP, CLIMB_CAP))).collect()
}

/// A world saved before the chain reaction: each school's start is its
/// prestige, which never moved before it.
pub(crate) fn chain_after_load(w: &mut super::World) {
    for h in w.high_schools.iter_mut() {
        if h.chain.start <= 0.0 {
            h.chain.start = h.prestige;
        }
    }
}

/// Its prestige in a past June, when the chain kept it; today's otherwise.
pub fn prestige_in(h: &HighSchool, year: i32) -> f64 {
    h.chain.years.iter().find(|y| y.0 == year).map_or(h.prestige, |y| y.1 as f64)
}

/// The why of its climb or fall in words, for the school page and the
/// trial: where it stood, where it stands and what its old players are
/// doing.
pub fn chain_why(h: &HighSchool, since: i32) -> String {
    let then = prestige_in(h, since);
    let now = h.prestige;
    let c = &h.chain;
    let move_word = if now > then + 0.02 {
        "climbed"
    } else if now < then - 0.02 {
        "fell back"
    } else {
        "held"
    };
    format!(
        "{move_word} from {:.2} in {since} to {:.2}: {} old players in college ({} at power programs) and {} in the NFL",
        then, now, c.college, c.power, c.nfl
    )
}
