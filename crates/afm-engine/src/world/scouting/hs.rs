//! The high school coach's side (topic 16): college staffs come to you for
//! film, transcripts and your word on a player, and you can push your
//! players to colleges. Your word carries what your record says it should:
//! each college staff keeps its own credit for you, which rises when the
//! players you sold them turn out as you said and falls when you oversold.
//! A promoted player gets a little more attention in the recruiting
//! year (more staffs look at his film), never stars by itself: the staffs
//! still judge his film.

use std::collections::HashMap;

use crate::calendar::{Date, Event, Kind as CalKind, Level, Loudness};
use crate::world::{Stage, World};

/// How hard you sold him.
pub const STRENGTHS: [&str; 3] = ["Worth a look", "Can play for you", "A sure thing"];

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Promo {
    pub id: u32,
    /// The college you called, or None for every staff that recruits your area.
    pub program: Option<usize>,
    pub date: Date,
    /// 0 worth a look, 1 can play for you, 2 a sure thing.
    pub strength: u8,
    /// Settled for credibility (two seasons on).
    #[serde(default)]
    pub settled: Option<bool>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HsSide {
    /// Each college staff's credit for your word (0-1).
    #[serde(default)]
    pub credibility: HashMap<usize, f32>,
    #[serde(default)]
    pub promoted: Vec<Promo>,
}

/// Your school when your seat is at a high school.
pub fn school(w: &World) -> Option<usize> {
    w.seat.filter(|s| s.level == Level::HighSchool).and_then(|s| s.program)
}

/// A staff's credit for you: your own record with it, or where your
/// network puts you before you have one.
pub fn credibility(w: &World, prog: usize) -> f64 {
    if let Some(&c) = w.scouting.hs.credibility.get(&prog) {
        return c as f64;
    }
    let net = school(w)
        .and_then(|h| w.high_schools.get(h))
        .and_then(|s| s.coach)
        .map_or(10.0, |c| w.coaches[c].ability().network);
    (0.35 + 0.02 * (net - 10.0)).clamp(0.1, 0.7)
}

/// You push a player to a college (or to every staff in your area).
pub fn promote(w: &mut World, id: u32, program: Option<usize>, strength: u8) -> Result<String, String> {
    let h = school(w).ok_or("Only a high school coach promotes his players.")?;
    let i = super::place(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    if p.hs != h || !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) {
        return Err("He isn't one of your players.".into());
    }
    if p.cal.signed.is_some() {
        return Err("He has already signed.".into());
    }
    let d = w.today;
    if w.scouting.hs.promoted.iter().any(|x| x.id == id && x.program == program && x.date.add(30) > d) {
        return Err("You made that call this month already.".into());
    }
    let strength = strength.min(2);
    let to =
        program.map_or_else(|| "every staff that recruits your area".to_string(), |c| w.program_name(c).to_string());
    let text = format!("You called {to} about {}: \"{}.\"", p.name, STRENGTHS[strength as usize]);
    w.scouting.hs.promoted.push(Promo { id, program, date: d, strength, settled: None });
    w.calendar.post(Event::new(d, Level::HighSchool, CalKind::News, Loudness::Small, text.clone()));
    Ok(text)
}

/// The recruiting year's extra attention from your calls (player id to a
/// visibility bump of up to about 0.12): a call to one staff counts half of
/// a call to the area, and a stronger word counts more, both scaled by
/// your credit.
pub fn promos(w: &World) -> HashMap<u32, f64> {
    let mut out: HashMap<u32, f64> = HashMap::new();
    for x in &w.scouting.hs.promoted {
        if x.settled.is_some() {
            continue;
        }
        let cred = x.program.map_or_else(|| area_credit(w), |c| credibility(w, c));
        let reach = if x.program.is_some() { 0.5 } else { 1.0 };
        let bump = (0.03 + 0.03 * x.strength as f64) * reach * (0.4 + cred);
        let e = out.entry(x.id).or_default();
        *e = (*e + bump).min(0.12);
    }
    out
}

/// Your average credit with the staffs that know you.
fn area_credit(w: &World) -> f64 {
    let v = &w.scouting.hs.credibility;
    if v.is_empty() { credibility(w, usize::MAX) } else { v.values().map(|&c| c as f64).sum::<f64>() / v.len() as f64 }
}

/// September 1: calls two seasons old are settled. A player who became a
/// real college player (starting, or a regular in the rotation) proves
/// your word; one who never played, or never went, costs you, more the
/// harder you sold him.
pub(crate) fn day(w: &mut World, d: Date) {
    let (_, m, day) = d.parts();
    if !(m == 9 && day == 1) || school(w).is_none() {
        return;
    }
    let places = super::places(w);
    let mut changes: Vec<(Option<usize>, f64, usize)> = Vec::new();
    for (k, x) in w.scouting.hs.promoted.iter().enumerate() {
        if x.settled.is_some() || x.date.add(700) > d {
            continue;
        }
        let Some(&i) = places.get(&x.id) else {
            changes.push((x.program, -0.04 * (1 + x.strength) as f64, k));
            continue;
        };
        let p = &w.people[i];
        let played = p.college_snaps >= 150 || matches!(p.stage, Stage::Pro);
        let delta = if played { 0.04 + 0.03 * x.strength as f64 } else { -0.03 - 0.05 * x.strength as f64 };
        changes.push((x.program.or(p.college), delta, k));
    }
    for (prog, delta, k) in changes {
        w.scouting.hs.promoted[k].settled = Some(delta > 0.0);
        let Some(c) = prog else { continue };
        let now = credibility(w, c);
        w.scouting.hs.credibility.insert(c, (now + delta).clamp(0.05, 0.95) as f32);
    }
    // Keep ten years of calls.
    let cut = d.add(-3650);
    w.scouting.hs.promoted.retain(|x| x.date >= cut);
}
