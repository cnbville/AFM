//! The venue a watched game is drawn in (later extras topic 2): the real
//! stadium for an NFL club (name, roof, surface, seats), a college's
//! stadium from its real capacity, a high school's field from its size.
//! Its light comes from the kickoff hour and the date, its weather from the
//! day the game was played.

use afm_engine::calendar::Level as CalLevel;
use afm_engine::game::ReplaySetup;
use afm_engine::world::World;
use afm_engine::world::seat::SeatGame;
use serde_json::{Value, json};

const NFL_TEAMS_CSV: &str = include_str!("../../../../data/real-world/nfl_teams.csv");

/// Seats in each NFL stadium (the listed football capacity, rounded).
const NFL_SEATS: &[(&str, u32)] = &[
    ("State Farm Stadium", 63_400),
    ("Mercedes-Benz Stadium", 71_000),
    ("M&T Bank Stadium", 71_000),
    ("Highmark Stadium", 71_600),
    ("Bank of America Stadium", 74_900),
    ("Soldier Field", 61_500),
    ("Paycor Stadium", 65_500),
    ("Huntington Bank Field", 67_400),
    ("AT&T Stadium", 80_000),
    ("Empower Field at Mile High", 76_100),
    ("Ford Field", 65_000),
    ("Lambeau Field", 81_400),
    ("NRG Stadium", 72_200),
    ("Lucas Oil Stadium", 67_000),
    ("EverBank Stadium", 67_800),
    ("GEHA Field at Arrowhead Stadium", 76_400),
    ("Arrowhead Stadium", 76_400),
    ("Allegiant Stadium", 65_000),
    ("SoFi Stadium", 70_200),
    ("Hard Rock Stadium", 64_800),
    ("U.S. Bank Stadium", 66_900),
    ("Gillette Stadium", 65_900),
    ("Caesars Superdome", 73_200),
    ("MetLife Stadium", 82_500),
    ("Lincoln Financial Field", 69_800),
    ("Acrisure Stadium", 68_400),
    ("Levi's Stadium", 68_500),
    ("Lumen Field", 68_700),
    ("Raymond James Stadium", 69_200),
    ("Nissan Stadium", 69_100),
    ("Northwest Stadium", 67_600),
];

fn csv_row(team: &str) -> Option<Vec<String>> {
    let mut lines = NFL_TEAMS_CSV.lines();
    let head: Vec<String> = afm_engine::world::data::split_csv_line(lines.next()?);
    let col = |k: &str| head.iter().position(|h| h == k);
    let (t, st, roof, surf) = (col("team")?, col("stadium")?, col("roof")?, col("surface")?);
    lines.map(afm_engine::world::data::split_csv_line).find(|r| r.get(t).is_some_and(|n| n == team)).map(|r| {
        vec![r[st].clone(), r[roof].clone(), r[surf].clone()]
    })
}

fn mix(mut x: u64) -> u64 {
    x ^= x >> 31;
    x = x.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    x ^= x >> 29;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^ (x >> 32)
}

/// 0-1 from a key, fixed forever.
fn unit(key: u64) -> f64 {
    (mix(key) % 10_000) as f64 / 10_000.0
}

/// About when the sun sets in the continental US by month (local time,
/// daylight saving through early November), hours.
fn sunset(month: u32) -> f64 {
    [17.3, 17.8, 18.4, 19.6, 20.1, 20.5, 20.4, 19.9, 19.2, 18.5, 17.0, 16.9][(month.clamp(1, 12) - 1) as usize]
}

/// The venue as the views draw it.
pub fn venue(w: &World, g: &SeatGame, setup: &ReplaySetup) -> Value {
    let home = g.teams[0];
    let cond = setup.cond;
    let (name, place, seats, shape, roof, surface, track, press_box, scoreboard) = match g.level {
        CalLevel::Nfl => {
            let team = w.nfl_team_name(home);
            let row = csv_row(&team);
            let stadium = row.as_ref().map_or_else(|| format!("{team} home field"), |r| r[0].clone());
            let seat_n = NFL_SEATS.iter().find(|(s, _)| *s == stadium).map_or(68_000, |s| s.1);
            let roof_s = row.as_ref().map_or("open", |r| match r[1].as_str() {
                "dome" => "dome",
                "retractable" => "retractable",
                _ => "open",
            });
            let surf = row.as_ref().map_or("grass", |r| if r[2] == "turf" { "turf" } else { "grass" });
            let city = row.as_ref().map_or(String::new(), |_| {
                let mut l = NFL_TEAMS_CSV.lines();
                let head = afm_engine::world::data::split_csv_line(l.next().unwrap_or(""));
                let (c, s, t) = (
                    head.iter().position(|h| h == "city").unwrap_or(1),
                    head.iter().position(|h| h == "state").unwrap_or(2),
                    head.iter().position(|h| h == "team").unwrap_or(0),
                );
                l.map(afm_engine::world::data::split_csv_line)
                    .find(|r| r.get(t).is_some_and(|n| *n == team))
                    .map_or(String::new(), |r| format!("{}, {}", r[c], r[s]))
            });
            (stadium, city, seat_n, "bowl", roof_s, surf, false, true, "video")
        }
        CalLevel::HighSchool => {
            let s = &w.high_schools[home];
            let k = home as u64 ^ 0x4853;
            // Home bleachers by enrollment: a few hundred seats at a small
            // school, ten thousand and more at the biggest Texas schools.
            let n = (s.enrollment as f64 * (1.1 + 1.6 * s.resources.clamp(0.0, 1.0) + 1.4 * s.prestige.clamp(0.0, 1.0)))
                .clamp(400.0, 14_000.0);
            let turf = unit(k ^ 1) < 0.15 + 0.6 * s.resources.clamp(0.0, 1.0);
            let state = afm_engine::world::geo::STATE_CENTERS.get(s.state).map_or("", |c| c.0);
            (
                format!("{} field", w.school_name(home)),
                format!("{}, {}", s.town, state),
                (n / 50.0).round() as u32 * 50,
                "bleachers",
                "open",
                if turf { "turf" } else { "grass" },
                unit(k ^ 2) < 0.7,
                true,
                if s.resources > 0.7 { "video" } else { "lights" },
            )
        }
        _ => {
            let p = &w.programs[home];
            let k = home as u64 ^ 0xC011;
            let n = p.stadium.max(2_000);
            let shp = if n >= 80_000 {
                "bowl"
            } else if n >= 45_000 {
                if unit(k ^ 3) < 0.4 { "horseshoe" } else { "bowl" }
            } else if n >= 15_000 {
                if unit(k ^ 3) < 0.5 { "two sides" } else { "horseshoe" }
            } else {
                "bleachers"
            };
            // About six in ten FBS fields are artificial turf now.
            let turf = unit(k ^ 4) < if n >= 15_000 { 0.6 } else { 0.75 };
            let state = afm_engine::world::geo::STATE_CENTERS.get(p.state).map_or("", |c| c.0);
            (
                format!("{} stadium", w.program_name(home)),
                format!("{}, {}", p.city, state),
                n,
                shp,
                "open",
                if turf { "turf" } else { "grass" },
                n < 30_000 && unit(k ^ 5) < 0.35,
                true,
                if n >= 25_000 { "video" } else { "lights" },
            )
        }
    };
    let (_, month, _) = g.date.parts();
    let hour = if setup.kickoff > 0.0 { setup.kickoff as f64 } else { 13.0 };
    let set = sunset(month);
    let light = if setup.indoor {
        "indoors"
    } else if hour + 1.6 < set - 0.6 {
        "day"
    } else if hour < set {
        "dusk"
    } else {
        "night"
    };
    // The crowd: the home crowd's noise says how full it is.
    let fill = if setup.neutral { 0.75 } else { (0.35 + 0.7 * cond.crowd).clamp(0.2, 1.0) };
    let officials = match setup.rules.level {
        afm_engine::player::Level::NFL => 7,
        afm_engine::player::Level::FBS if g.date.year() >= 2023 => 8,
        afm_engine::player::Level::HS => 5,
        _ => 7,
    };
    json!({
        "name": name,
        "place": place,
        "seats": seats,
        "attendance": ((seats as f64 * fill) / 10.0).round() as u32 * 10,
        "fill": (fill * 100.0).round() / 100.0,
        "shape": shape,
        "roof": roof,
        "closed": setup.indoor,
        "surface": surface,
        "track": track,
        "press_box": press_box,
        "scoreboard": scoreboard,
        "kickoff": hour,
        "light": light,
        "month": month,
        "temperature": cond.temperature.round(),
        "wind": cond.wind.round(),
        "wind_along": (cond.wind_along * 100.0).round() / 100.0,
        "rain": cond.rain,
        "snow": cond.snow,
        "officials": officials,
        "neutral": setup.neutral,
    })
}
