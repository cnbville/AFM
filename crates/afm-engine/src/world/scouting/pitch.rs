//! A high school coach recruiting to his school, at a risk (scouting
//! update SK42). Most states ban it: recruiting a kid from another school
//! for athletics is undue influence. In Texas a kid who transfers for
//! athletic reasons sits out a year, from 2024 the UIL reviews schools with
//! too many transfers, and a head coach was suspended for a year over a
//! recruiting case. Private schools draw from wider areas. You can try:
//! the kid and his family listen, decide by the June transfer season, and
//! if it comes out your school forfeits the games he played and you are
//! suspended for a year.
//!
//! Proposed, not confirmed: four pitches a season; a yes from about 20%
//! (more for a backup at a weaker school, less for a starter, far less
//! when a public school's family would have to move into your zone); found
//! out 18% of the time, plus 8% for each other kid you landed that year,
//! plus 15% in Texas from 2024 once three or more came to you in a year;
//! the news breaks between September and mid-November. While suspended
//! your school's games are coached without you.

use crate::calendar::{Date, Event, Kind, Level, Loudness};
use crate::world::season::key_uniform;
use crate::world::{Stage, World};

pub const PER_SEASON: usize = 4;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Pitch {
    pub id: u32,
    pub date: Date,
    pub from: usize,
    /// The family's answer, kept until June.
    pub yes: bool,
    /// He moved to your school.
    pub moved: bool,
    /// The day it comes out, when it does.
    pub caught: Option<Date>,
    pub done: bool,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Pitches {
    pub list: Vec<Pitch>,
    /// You are off the sideline until this day.
    pub suspended: Option<Date>,
}

/// Your school when you coach one.
fn yours(w: &World) -> Option<usize> {
    crate::world::front_office::school::your_school(w)
}

/// Whether your school's games go on without you today.
pub fn sits(w: &World, h: usize) -> bool {
    yours(w) == Some(h) && w.scouting.pitches.suspended.is_some_and(|d| w.today < d)
}

/// How likely he and his family say yes.
fn chance(w: &World, h: usize, i: usize) -> f64 {
    let p = &w.people[i];
    let (mine, his) = (&w.high_schools[h], &w.high_schools[p.hs]);
    let role = match p.last_role {
        Some(crate::player::PlayingTime::Backup) | None => 0.15,
        Some(crate::player::PlayingTime::Rotation) => 0.05,
        Some(_) => -0.1,
    };
    let near = if mine.county == his.county {
        0.0
    } else if mine.private {
        -0.05
    } else {
        -0.3
    };
    // Your hunt (SK67): the family's trust you built over the off-season.
    let warmth = crate::world::hunt::yours(w).filter(|x| x.id == p.player.id).map_or(0.0, |x| x.warmth as f64);
    (0.2 + 0.8 * (mine.prestige - his.prestige) + role + near + 0.25 * warmth).clamp(0.02, 0.7 + 0.15 * warmth)
}

/// How likely a pitch is to be said yes to now (the hunt's page).
pub fn chance_now(w: &World, id: u32) -> Option<f64> {
    let h = yours(w)?;
    let i = super::place(w, id)?;
    Some(chance(w, h, i))
}

/// You go after a kid at another school.
pub fn pitch(w: &mut World, id: u32) -> Result<String, String> {
    let h = yours(w).ok_or("Only a high school head coach recruits to his school.")?;
    let i = super::place(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    if p.stage != Stage::HighSchool || p.class_of <= w.dates.season {
        return Err("Only a kid with a season of high school left.".into());
    }
    if p.hs == h {
        return Err("He plays for you already.".into());
    }
    if w.high_schools[p.hs].state != w.high_schools[h].state {
        return Err("Out of state: his family would have to move, and nobody moves for that.".into());
    }
    let season = w.dates.season;
    let list = &w.scouting.pitches.list;
    if list.iter().any(|x| x.id == id && !x.done) {
        return Err("You've made your pitch to his family already.".into());
    }
    if list.iter().filter(|x| x.date >= Date::ymd(season, 1, 1).add(-150)).count() >= PER_SEASON {
        return Err(format!("{PER_SEASON} families this year already: any more and people talk."));
    }
    let c = chance(w, h, i);
    let yes = key_uniform(0x5C42 ^ (w.options.seed as u64) << 16 ^ w.today.0 as u64, id as u64) < c;
    let (from, name) = (p.hs, p.name.clone());
    w.scouting.pitches.list.push(Pitch { id, date: w.today, from, yes, moved: false, caught: None, done: false });
    let code = &w.states[w.high_schools[h].state].code;
    Ok(format!(
        "You talked to {name}'s family about coming to play for you. They'll decide by the June transfer season. {code}'s association bars recruiting: if it comes out, it costs you."
    ))
}

/// June 15: the families who said yes move; September to November: what
/// comes out lands.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(h) = yours(w) else { return };
    if w.scouting.pitches.list.iter().all(|x| x.done) {
        return;
    }
    let (_, m, dd) = d.parts();
    if m == 6 && dd == 15 {
        moves(w, h, d);
    }
    let due: Vec<usize> = w
        .scouting
        .pitches
        .list
        .iter()
        .enumerate()
        .filter(|(_, x)| !x.done && x.caught == Some(d))
        .map(|(k, _)| k)
        .collect();
    for k in due {
        caught(w, h, k, d);
    }
    // A pitch that came to nothing, or a move that never came out, closes
    // after a season.
    for x in w.scouting.pitches.list.iter_mut() {
        if !x.done && x.caught.is_none() && x.date.add(400) <= d {
            x.done = true;
        }
    }
}

fn moves(w: &mut World, h: usize, d: Date) {
    let season = w.dates.season;
    let st = w.high_schools[h].state;
    let texas = w.states[st].code == "TX";
    let rule = w.states[st].transfer;
    let to_name = w.school_name(h).to_string();
    let mut landed = 0usize;
    let mut lines: Vec<String> = Vec::new();
    let ks: Vec<usize> = (0..w.scouting.pitches.list.len()).filter(|&k| !w.scouting.pitches.list[k].done).collect();
    for k in ks {
        let x = w.scouting.pitches.list[k].clone();
        if x.moved || x.date.add(365) < d {
            continue;
        }
        let Some(i) = super::place(w, x.id) else { continue };
        if !x.yes || w.people[i].stage != Stage::HighSchool || w.people[i].hs != x.from {
            w.scouting.pitches.list[k].done = true;
            let name = w.people[i].name.clone();
            lines.push(format!("{name}'s family decided to stay put."));
            continue;
        }
        let from_name = w.school_name(x.from).to_string();
        let p = &mut w.people[i];
        // An athletic transfer: Texas sits him a year; elsewhere the
        // state's rule.
        p.sit_out = if texas { 1.0 } else { rule.sit_out.max(0.5) };
        p.transfers += 1;
        p.hs = h;
        let age = p.player.age;
        p.note(format!("At {age} he transferred from {from_name} to {to_name}."));
        let name = p.name.clone();
        lines.push(format!("{name} transferred in from {from_name}."));
        w.scouting.pitches.list[k].moved = true;
        landed += 1;
    }
    if landed > 0 {
        w.rebuild_rosters();
        let review = if texas && season >= 2024 && landed >= 3 { 0.15 } else { 0.0 };
        let p_out = 0.18 + 0.08 * (landed - 1) as f64 + review;
        // Your hunt (SK67): the more people knew you wanted him, the
        // likelier it comes out.
        let hunt = crate::world::hunt::yours(w).map(|x| (x.id, crate::world::hunt::heat(w, x).0));
        for x in w.scouting.pitches.list.iter_mut().filter(|x| x.moved && x.caught.is_none() && !x.done) {
            let p_out = match hunt {
                Some((id, heat)) if id == x.id => p_out * (0.6 + 0.8 * heat),
                _ => p_out,
            };
            let u = key_uniform(0xCA06 ^ (season as u64) << 8, x.id as u64);
            if u < p_out {
                let span = Date::ymd(season, 11, 15).0 - Date::ymd(season, 9, 1).0;
                x.caught = Some(Date::ymd(season, 9, 1).add((u / p_out * span as f64) as i32));
            } else {
                x.done = true;
            }
        }
    }
    if !lines.is_empty() {
        crate::world::front_office::tell(w, d, lines.join(" "), false);
    }
}

fn caught(w: &mut World, h: usize, k: usize, d: Date) {
    let x = w.scouting.pitches.list[k].clone();
    w.scouting.pitches.list[k].done = true;
    let name = super::place(w, x.id).map_or_else(|| "a transfer".to_string(), |i| w.people[i].name.clone());
    let st = w.high_schools[h].state;
    let code = w.states[st].code.clone();
    let school = w.school_name(h).to_string();
    // The games he played are forfeited: every win so far this season,
    // unless he was sitting out his transfer.
    let played = super::place(w, x.id).is_some_and(|i| w.people[i].sit_out < 1.0);
    let mut lost = 0;
    if played
        && let Some(hs) = w.hs_season.as_mut()
        && let Some(r) = hs.results.get_mut(h)
    {
        lost = r.wins;
        r.losses += r.wins;
        r.wins = 0;
    }
    if let Some(i) = super::place(w, x.id) {
        w.folk.out.push((
            x.id,
            Date::ymd(w.dates.season + 1, 1, 15),
            "ruled ineligible: recruited to his school".into(),
        ));
        let age = w.people[i].player.age;
        w.people[i].note(format!("At {age} the state association ruled he had been recruited to {school}."));
    }
    w.scouting.pitches.suspended = Some(d.add(365));
    if let Some(c) = w.career.as_ref().map(|c| c.coach) {
        w.coaches[c].reputation = (w.coaches[c].reputation - 1.0).max(1.0);
    }
    let forfeits = if lost > 0 { format!(" {school} forfeits {lost} wins.") } else { String::new() };
    crate::world::front_office::tell(
        w,
        d,
        format!(
            "It came out: {code}'s association found you recruited {name} to {school}. He is ineligible for the season, and you are suspended for a year: your staff coaches the games.{forfeits}"
        ),
        true,
    );
    let you = crate::world::folk::scene::your_name(w);
    w.calendar.post(Event::new(
        d,
        Level::HighSchool,
        Kind::News,
        Loudness::Medium,
        format!("{code} association suspends {school} coach {you} for a year over the recruiting of {name}.{forfeits}"),
    ));
}
