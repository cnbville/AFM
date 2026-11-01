//! Being there (SK32, SK33, SK36-SK39): the days you are there in person.
//!
//! - His game from the stands (SK32): on a trip day to his game, his snaps
//!   of the night as stills, the way his tape looks, with no replays.
//! - A visit is a scene (SK33): four hours at his school, spent on what
//!   you watch (warmups, individual drills, the team period, the sideline)
//!   and whom you ask (the strength coach, the trainer, the academic
//!   advisor, the equipment manager). Each choice shows something else.
//! - Who else is watching (SK36): the staffs and clubs there with you.
//! - Your camp day (SK37): coach a group yourself, see who stands out in
//!   the drills and one-on-ones, offer on the spot; a high school coach
//!   takes his team to a college's team camp, where his players get seen.
//! - Signing day (SK38) and combine week (SK39) are rooms the game builds
//!   from the day's signings and the week's numbers (`signings`, `combine`).
//!
//! Times and sizes are Claude's, proposed, not confirmed.

use super::session::{self, Mode};
use super::tape::Cut;
use super::{Catch, Target, words};
use crate::attributes::Attr;
use crate::calendar::{Date, Weekday};
use crate::rng::clamp;
use crate::world::scouting::file;
use crate::world::scouting::own::TripKind;
use crate::world::scouting::readers::{self, COACH, Staff};
use crate::world::season::{key_normal, key_uniform};
use crate::world::{Stage, World};

/// The days you were there.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct There {
    /// The visit in progress.
    #[serde(default)]
    pub visit: Option<Visit>,
    /// Visits kept for his file (the newest 200).
    #[serde(default)]
    pub visits: Vec<Visit>,
    /// Your camp day in progress, and the camps you ran.
    #[serde(default)]
    pub camp: Option<Camp>,
    #[serde(default)]
    pub camps: Vec<Camp>,
    /// The team camps you took your high school to: (date, program).
    #[serde(default)]
    pub team_camps: Vec<(Date, usize)>,
}

/// One choice at a visit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    pub key: &'static str,
    pub label: &'static str,
    /// Where it happens (the drawn scene).
    pub place: &'static str,
    pub minutes: u16,
}

/// What a visit offers by where you are.
pub fn choices(kind: TripKind) -> &'static [Choice] {
    const PRACTICE: [Choice; 8] = [
        Choice { key: "warmups", label: "Watch warmups", place: "field", minutes: 30 },
        Choice { key: "indy", label: "Watch his individual drills", place: "field", minutes: 60 },
        Choice { key: "team", label: "Watch the team period", place: "field", minutes: 60 },
        Choice { key: "sideline", label: "Watch how he acts on the sideline", place: "field", minutes: 30 },
        Choice { key: "strength", label: "Ask the strength coach about his work", place: "weights", minutes: 30 },
        Choice { key: "trainer", label: "Ask the trainer about injuries", place: "training", minutes: 30 },
        Choice { key: "academic", label: "Ask the academic advisor", place: "hallway", minutes: 30 },
        Choice {
            key: "equipment",
            label: "Ask the equipment manager how he treats people",
            place: "hallway",
            minutes: 30,
        },
    ];
    const PRO_DAY: [Choice; 7] = [
        Choice { key: "testing", label: "Watch the testing", place: "indoor", minutes: 60 },
        Choice { key: "indy", label: "Watch his position drills", place: "indoor", minutes: 60 },
        Choice { key: "warmups", label: "Watch him warm up and stretch", place: "indoor", minutes: 30 },
        Choice { key: "sideline", label: "Watch him between drills", place: "indoor", minutes: 30 },
        Choice { key: "strength", label: "Ask the strength coach about his work", place: "weights", minutes: 30 },
        Choice { key: "trainer", label: "Ask the trainer about injuries", place: "training", minutes: 30 },
        Choice {
            key: "equipment",
            label: "Ask the equipment manager how he treats people",
            place: "hallway",
            minutes: 30,
        },
    ];
    const MIDDLE: [Choice; 5] = [
        Choice { key: "warmups", label: "Watch warmups", place: "field", minutes: 30 },
        Choice { key: "team", label: "Watch him play", place: "field", minutes: 60 },
        Choice { key: "sideline", label: "Watch him on the sideline", place: "field", minutes: 30 },
        Choice { key: "academic", label: "Ask his teachers about him", place: "hallway", minutes: 30 },
        Choice { key: "equipment", label: "Ask his coach how he treats people", place: "hallway", minutes: 30 },
    ];
    match kind {
        TripKind::ProDay | TripKind::Workout => &PRO_DAY,
        TripKind::MiddleSchool => &MIDDLE,
        _ => &PRACTICE,
    }
}

/// A visit (SK33).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Visit {
    pub id: u32,
    pub name: String,
    pub kind: TripKind,
    pub date: Date,
    pub minutes_left: u16,
    /// Where you are now (the drawn scene).
    pub place: String,
    /// What you did and what it showed: (choice, place, line).
    pub done: Vec<(String, String, String)>,
    /// Who else was there (SK36).
    #[serde(default)]
    pub watchers: Vec<String>,
}

/// Visit minutes (four hours at his school).
pub const VISIT_MIN: u16 = 240;

/// Your trips today, and what being there offers: (player, kind, his
/// name, "stands" for a game, "visit" for a scene).
pub fn today(w: &World) -> Vec<(u32, TripKind, String, &'static str)> {
    let me = readers::you(w).map(|c| COACH + c as u32);
    w.scouting
        .own
        .trips
        .iter()
        .filter(|t| t.date == w.today && t.sent.is_none_or(|s| Some(s) == me))
        .filter_map(|t| {
            let i = super::super::place(w, t.id)?;
            let how = match t.kind {
                TripKind::Game | TripKind::AllStar => "stands",
                TripKind::OldFilm => return None,
                _ => "visit",
            };
            Some((t.id, t.kind, w.people[i].name.clone(), how))
        })
        .collect()
}

fn trip_today(w: &World, id: u32) -> Option<TripKind> {
    today(w).into_iter().find(|t| t.0 == id).map(|t| t.1)
}

/// Who else is watching him (SK36): the college staffs that offered or
/// visited him, the NFL clubs whose interest in him leaked or whose
/// scouts a prospect of his grade draws.
pub fn watchers(w: &World, id: u32) -> Vec<String> {
    let Some(i) = super::super::place(w, id) else { return Vec::new() };
    let p = &w.people[i];
    let mine = match readers::seat_staff(w) {
        Some(Staff::Program(c)) => Some(c),
        _ => None,
    };
    let d = w.today;
    let mut out: Vec<String> = Vec::new();
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => {
            let mut progs: Vec<usize> = p.cal.offers.clone();
            progs.extend(p.cal.early_offers.iter().map(|x| x.0));
            progs.extend(p.cal.visits.iter().filter(|v| v.1.days_to(d).abs() <= 120).map(|v| v.0));
            let mut seen = std::collections::HashSet::new();
            for c in progs {
                if Some(c) != mine && seen.insert(c) && out.len() < 6 {
                    out.push(format!("{} staff", w.program_name(c)));
                }
            }
        }
        Stage::College | Stage::Declared => {
            let you = match readers::seat_staff(w) {
                Some(Staff::Club(t)) => Some(t),
                _ => None,
            };
            let mut clubs: Vec<usize> =
                w.scouting.predraft.leaks.iter().filter(|l| l.player == id).map(|l| l.team).collect();
            // A prospect of his grade draws scouts: the stars, most of the
            // league; a late-rounder, a handful.
            let rank = p.stars as usize;
            let n = match rank {
                5 => 14,
                4 => 9,
                3 => 5,
                _ => 2,
            };
            let mut keyed: Vec<(u32, usize)> = (0..w.nfl.len())
                .filter(|&t| crate::world::franchise::active(t, w.year))
                .map(|t| (key_uniform(id as u64 ^ (d.0 as u64) << 32, t as u64).to_bits() as u32, t))
                .collect();
            keyed.sort_unstable();
            clubs.extend(keyed.into_iter().take(n).map(|x| x.1));
            let mut seen = std::collections::HashSet::new();
            for t in clubs {
                if Some(t) != you && seen.insert(t) && out.len() < 10 {
                    out.push(format!("{} scouts", w.nfl[t].name));
                }
            }
        }
        _ => {}
    }
    out
}

// --- His game from the stands (SK32) ------------------------------------------------------------

/// From the stands: his snaps of tonight's game as stills, no replays.
pub fn stands(w: &mut World, id: u32) -> Result<String, String> {
    match trip_today(w, id) {
        Some(TripKind::Game | TripKind::AllStar) => {}
        _ => return Err("You're not at his game today: book the trip first.".into()),
    }
    // One game a night: you were in the stands for it already.
    let d = w.today;
    if w.scouting.film.reports.iter().any(|r| r.mode == Mode::Live && r.target == Target::Player(id) && r.date == d) {
        return Err("You watched his game from the stands tonight: it's in your notes.".into());
    }
    session::open_player(w, id, Cut::BestOpponent, Mode::Live, None)
}

// --- A visit (SK33) -----------------------------------------------------------------------------

pub fn visit_open(w: &mut World, id: u32) -> Result<String, String> {
    let kind = trip_today(w, id).ok_or("You're not visiting him today: book the trip first.")?;
    if matches!(kind, TripKind::Game | TripKind::AllStar | TripKind::OldFilm) {
        return Err("That trip is a game: watch it from the stands.".into());
    }
    if w.scouting.film.there.visits.iter().any(|v| v.id == id && v.date == w.today) {
        return Err("You've been to see him today.".into());
    }
    if w.scouting.film.there.visit.is_some() {
        return Err("Finish the visit you're on first.".into());
    }
    let i = super::super::place(w, id).ok_or("No such player.")?;
    let name = w.people[i].name.clone();
    let watchers = watchers(w, id);
    w.scouting.film.there.visit = Some(Visit {
        id,
        name: name.clone(),
        kind,
        date: w.today,
        minutes_left: VISIT_MIN,
        place: choices(kind)[0].place.to_string(),
        done: Vec::new(),
        watchers,
    });
    Ok(format!("You're at {name}'s school: four hours."))
}

/// A choice at the visit: what it shows, and the time it takes.
pub fn visit_choose(w: &mut World, key: &str) -> Result<String, String> {
    let mut v = w.scouting.film.there.visit.take().ok_or("You're not on a visit.")?;
    if v.date != w.today {
        w.scouting.film.there.visit = Some(v);
        let _ = visit_close(w);
        return Err("That visit was another day: it's in your notes.".into());
    }
    let Some(c) = choices(v.kind).iter().find(|c| c.key == key).copied() else {
        w.scouting.film.there.visit = Some(v);
        return Err("That isn't on offer here.".into());
    };
    if v.done.iter().any(|d| d.0 == key) {
        w.scouting.film.there.visit = Some(v);
        return Err("You've done that already.".into());
    }
    if c.minutes > v.minutes_left {
        w.scouting.film.there.visit = Some(v);
        return Err("There isn't time left for that.".into());
    }
    v.minutes_left -= c.minutes;
    v.place = c.place.to_string();
    let line = show(w, v.id, key);
    v.done.push((key.to_string(), c.place.to_string(), line.clone()));
    let done = v.minutes_left < 30;
    w.scouting.film.there.visit = Some(v);
    if done {
        visit_close(w)?;
    }
    Ok(line)
}

/// Leaves the visit: what you saw goes in his file.
pub fn visit_close(w: &mut World) -> Result<String, String> {
    let v = w.scouting.film.there.visit.take().ok_or("You're not on a visit.")?;
    let n = v.done.len();
    let there = &mut w.scouting.film.there;
    there.visits.push(v);
    if there.visits.len() > 200 {
        there.visits.remove(0);
    }
    w.scouting.film.version = w.scouting.film.version.wrapping_add(1);
    Ok(format!("You left with {n} things in your notes."))
}

fn your_eye(w: &World) -> f64 {
    crate::world::scouting::yours::your_eye(w).unwrap_or(10.0)
}

/// Catches for a set of attributes watched in person: sharper than tape
/// for what the eye sees whole (a practice in person reads best).
fn watch(w: &mut World, id: u32, attrs: &[Attr], salt: u64) -> Vec<String> {
    let Some(i) = super::super::place(w, id) else { return Vec::new() };
    let e = your_eye(w);
    let d = w.today;
    let sd = (3.0 * file::sharp(e)) as f32;
    let mut lines = Vec::new();
    let mark = words::level_mark(w.people[i].stage);
    for (k, &a) in attrs.iter().enumerate() {
        let u = key_uniform(w.options.seed as u64 ^ salt ^ (d.0 as u64) << 24, id as u64 ^ (k as u64) << 40);
        let p_see = clamp(0.55 + 0.03 * (e - 10.0) - 0.3 * words::subtle(a), 0.1, 0.95);
        if u >= p_see {
            continue;
        }
        let truth = w.people[i].player.attributes.get(a);
        let noise = key_normal(w.options.seed as u64 ^ salt ^ 0x5EE ^ (d.0 as u64) << 24, id as u64 ^ (k as u64) << 40);
        w.scouting.film.catches.push(Catch {
            id,
            attr: a.index() as u8,
            v: (truth + noise * sd as f64) as f32,
            sd,
            date: d,
            by: 0,
            key: 0,
        });
        lines.push(words::cue(a, words::band(truth, mark)).to_string());
    }
    lines
}

/// What a choice shows (SK33).
fn show(w: &mut World, id: u32, key: &str) -> String {
    let Some(i) = super::super::place(w, id) else { return String::new() };
    let p = &w.people[i];
    let pos = p.player.position;
    let first = p.name.split_whitespace().next().unwrap_or(&p.name).to_string();
    let pe = p.player.personality;
    let t = pe.traits;
    let lean = |salt: u64| key_normal(w.options.seed as u64 ^ salt, id as u64) * 2.5;
    let athletic = [Attr::Speed, Attr::Acceleration, Attr::Agility, Attr::Jumping, Attr::Flexibility];
    let tech: Vec<Attr> = words::asks(pos).iter().flat_map(|a| a.attrs.iter().copied()).take(5).collect();
    let line = |v: Vec<String>, none: &str| if v.is_empty() { none.to_string() } else { v.join(" ") };
    match key {
        "warmups" => {
            let v = watch(w, id, &athletic, 0x3A01);
            line(v, "Warmups don't tell you much today: he goes through it like everyone else.")
        }
        "indy" => {
            let v = watch(w, id, &tech, 0x3A02);
            line(v, "His drill work is ordinary: nothing jumps out either way.")
        }
        "team" => {
            let mut all = tech.clone();
            all.extend([Attr::FootballIq, Attr::Anticipation]);
            let v = watch(w, id, &all, 0x3A03);
            line(v, "The team period goes by without him touching much.")
        }
        "testing" => {
            let v =
                watch(w, id, &[Attr::Speed, Attr::Acceleration, Attr::Strength, Attr::Jumping, Attr::Agility], 0x3A04);
            line(v, "The testing goes about as his tape said it would.")
        }
        "sideline" => {
            let lead = t.leadership as f64 + lean(0x51DE);
            let temper = t.temper as f64 + lean(0x7E39);
            let love = pe.love_of_game + lean(0x10F6);
            if temper >= 14.0 {
                format!("{first} snaps at a teammate after a bad rep and sulks at the end of the bench.")
            } else if lead >= 14.0 {
                format!("Between reps {first} is in the middle of the group, talking, picking the young guys up.")
            } else if love <= 7.0 {
                format!("{first} drifts to the back, helmet off, more interested in the stands than the field.")
            } else {
                format!("{first} is locked in on the sideline: watching, asking his coach questions.")
            }
        }
        "strength" => {
            let work = pe.work_ethic + lean(0x5743);
            if work >= 15.0 {
                format!(
                    "The strength coach: \"{first}'s the first one here. I have to run him out of the weight room.\""
                )
            } else if work <= 7.0 {
                "The strength coach: \"Talented kid. I'd like to see him here when it isn't required.\"".to_string()
            } else {
                "The strength coach: \"He does the work. Nothing extra, nothing missed.\"".to_string()
            }
        }
        "trainer" => {
            let prone = p.player.injury_proneness + lean(0x7241) * 0.04;
            if prone >= 0.6 {
                format!(
                    "The trainer: \"We see a lot of {first} in here. Hamstrings mostly; he's always rehabbing something.\""
                )
            } else if prone <= 0.3 {
                "The trainer: \"Hardly ever see him. Tough kid; he's played through things others sit out.\""
                    .to_string()
            } else {
                "The trainer: \"The usual bumps. Nothing that worries me.\"".to_string()
            }
        }
        "academic" => {
            let gpa = p.bg.gpa + lean(0xAC4D) * 0.1;
            if gpa >= 3.3 {
                format!(
                    "The advisor: \"{first} is one of our best students. You won't have to worry about him in class.\""
                )
            } else if gpa <= 2.3 {
                "The advisor: \"We're working on it. He'll need a plan, and someone checking on him.\"".to_string()
            } else {
                "The advisor: \"He gets it done. He knows what he has to do to stay eligible.\"".to_string()
            }
        }
        "equipment" => {
            let respect = t.professionalism as f64 - t.ego as f64 * 0.5 + lean(0xE9A1);
            if respect >= 9.0 {
                format!(
                    "The equipment manager: \"{first}? Says please and thank you, picks up after himself. Good kid.\""
                )
            } else if respect <= 3.0 {
                "The equipment manager: \"He's got a mouth on him with my student workers. Treats them like help.\""
                    .to_string()
            } else {
                "The equipment manager: \"Never had a problem with him.\"".to_string()
            }
        }
        _ => String::new(),
    }
}

// --- Your camp day (SK37) -----------------------------------------------------------------------

/// A camp day: who came, the group you coached, what you saw.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Camp {
    pub date: Date,
    /// The campers (ids), and the group you coach yourself (0-7).
    pub campers: Vec<u32>,
    pub group: Option<u8>,
    /// Who stood out in your eyes: (id, what you saw).
    pub standouts: Vec<(u32, String)>,
    /// One-on-ones: (winner, loser, words).
    pub one_on_ones: Vec<(u32, u32, String)>,
    /// Offered on the spot.
    pub offered: Vec<u32>,
}

/// Your camp days: the second and third Saturdays of June (`dept::day`).
pub fn camp_day(w: &World) -> bool {
    let d = w.today;
    let y = d.year();
    matches!(readers::seat_staff(w), Some(Staff::Program(_)))
        && (d == Date::nth_weekday(y, 6, Weekday::Sat, 2) || d == Date::nth_weekday(y, 6, Weekday::Sat, 3))
}

/// Opens your camp: the campers are the recruits your staff saw at it
/// today, and the near kids who came on their own.
pub fn camp_open(w: &mut World) -> Result<String, String> {
    if !camp_day(w) {
        return Err("Your one-day camps are the second and third Saturdays in June.".into());
    }
    if w.scouting.film.there.camp.as_ref().is_some_and(|c| c.date == w.today) {
        return Ok("Your camp is under way.".into());
    }
    if w.scouting.film.there.camps.iter().any(|c| c.date == w.today) {
        return Err("Today's camp is over.".into());
    }
    let d = w.today;
    let mut campers: Vec<u32> = w
        .scouting
        .dept
        .looks
        .iter()
        .filter(|(_, v)| v.iter().any(|l| l.kind == crate::world::scouting::Kind::Camp && l.date == d))
        .map(|(&id, _)| id)
        .collect();
    campers.sort_unstable();
    if campers.is_empty() {
        return Err("Nobody on your board came to camp today.".into());
    }
    w.scouting.film.there.camp = Some(Camp {
        date: d,
        campers,
        group: None,
        standouts: Vec::new(),
        one_on_ones: Vec::new(),
        offered: Vec::new(),
    });
    Ok("Camp is open.".into())
}

/// You coach one group yourself: you see its campers up close (catches),
/// who stands out, and the one-on-ones.
pub fn camp_coach(w: &mut World, group: u8) -> Result<String, String> {
    use crate::world::coach::Group;
    let g = Group::ALL.get(group as usize).copied().ok_or("No such group.")?;
    let mut c = w.scouting.film.there.camp.take().ok_or("Camp isn't open.")?;
    if c.group.is_some() {
        w.scouting.film.there.camp = Some(c);
        return Err("You've coached your group today.".into());
    }
    c.group = Some(group);
    let mine: Vec<(u32, usize)> = c
        .campers
        .iter()
        .filter_map(|&id| super::super::place(w, id).map(|i| (id, i)))
        .filter(|&(_, i)| Group::of(w.people[i].player.position) == g)
        .collect();
    let e = your_eye(w);
    let mut scored: Vec<(f64, u32, String)> = Vec::new();
    for &(id, i) in &mine {
        let pos = w.people[i].player.position;
        let attrs: Vec<Attr> = words::asks(pos).iter().flat_map(|a| a.attrs.iter().copied()).take(4).collect();
        let lines = watch(w, id, &attrs, 0xCA30);
        let p = &w.people[i];
        let ovr = crate::positions::overall_for(pos, &p.player.attributes);
        let noise =
            key_normal(w.options.seed as u64 ^ 0xCA31 ^ (w.today.0 as u64) << 20, id as u64) * 4.0 * file::sharp(e);
        let first = lines.first().cloned().unwrap_or_else(|| "Solid day.".to_string());
        scored.push((ovr + noise, id, first));
    }
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    c.standouts = scored.iter().take(3).map(|x| (x.1, x.2.clone())).collect();
    // One-on-ones: the group's campers paired off, decided by the
    // battle's attributes and the day's luck.
    let ids: Vec<u32> = scored.iter().map(|x| x.1).collect();
    for pair in ids.chunks(2) {
        if let [a, b] = pair {
            let (ia, ib) = (super::super::place(w, *a), super::super::place(w, *b));
            if let (Some(ia), Some(ib)) = (ia, ib) {
                let va = crate::positions::overall_for(w.people[ia].player.position, &w.people[ia].player.attributes);
                let vb = crate::positions::overall_for(w.people[ib].player.position, &w.people[ib].player.attributes);
                let luck = key_normal(0x1015 ^ w.today.0 as u64, (*a as u64) << 32 | *b as u64) * 6.0;
                let (win, lose) = if va - vb + luck >= 0.0 { (*a, *b) } else { (*b, *a) };
                let wn = w.people[super::super::place(w, win).unwrap_or(ia)].name.clone();
                let ln = w.people[super::super::place(w, lose).unwrap_or(ib)].name.clone();
                c.one_on_ones.push((win, lose, format!("{wn} won the rep against {ln}.")));
            }
        }
    }
    let n = mine.len();
    w.scouting.film.there.camp = Some(c);
    w.scouting.film.version = w.scouting.film.version.wrapping_add(1);
    Ok(format!("You coached the {} yourself: {n} campers.", g.name()))
}

/// An offer on the spot, before the rivals.
pub fn camp_offer(w: &mut World, id: u32) -> Result<String, String> {
    let open = w.scouting.film.there.camp.as_ref().is_some_and(|c| c.date == w.today && c.campers.contains(&id));
    if !open {
        return Err("He isn't at your camp today.".into());
    }
    let text = crate::world::front_office::college::offer(w, id)?;
    if let Some(c) = w.scouting.film.there.camp.as_mut() {
        c.offered.push(id);
    }
    Ok(format!("Offered on the spot. {text}"))
}

/// Closes the camp day.
pub fn camp_close(w: &mut World) -> Result<String, String> {
    let c = w.scouting.film.there.camp.take().ok_or("Camp isn't open.")?;
    let n = c.offered.len();
    w.scouting.film.there.camps.push(c);
    if w.scouting.film.there.camps.len() > 40 {
        w.scouting.film.there.camps.remove(0);
    }
    Ok(format!("Camp's over: {n} offers on the spot."))
}

/// A high school coach takes his team to a college's team camp in June
/// (SK37): his players get seen by that staff, the ones who stand out
/// most (a word to that staff, as if you had called).
pub fn team_camp(w: &mut World, program: usize) -> Result<String, String> {
    let h = crate::world::scouting::hs::school(w).ok_or("Only a high school coach takes his team to camp.")?;
    let d = w.today;
    if d.parts().1 != 6 {
        return Err("Team camps are in June.".into());
    }
    if w.scouting.film.there.team_camps.iter().any(|x| x.0.year() == d.year()) {
        return Err("You've taken your team to camp this summer.".into());
    }
    if program >= w.programs.len() {
        return Err("No such college.".into());
    }
    let boys: Vec<(f64, u32)> = w.high_schools[h]
        .roster
        .iter()
        .filter_map(|&i| w.people.get(i))
        .filter(|p| matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) && p.cal.signed.is_none())
        .map(|p| {
            let o = crate::positions::overall_for(p.player.position, &p.player.attributes);
            let luck = key_normal(0x7CA3 ^ d.0 as u64, p.player.id as u64) * 3.0;
            (o + luck, p.player.id)
        })
        .collect();
    let mut boys = boys;
    boys.sort_by(|a, b| b.0.total_cmp(&a.0));
    let seen: Vec<u32> = boys.iter().take(4).map(|x| x.1).collect();
    for &id in &seen {
        w.scouting.hs.promoted.push(crate::world::scouting::hs::Promo {
            id,
            program: Some(program),
            date: d,
            strength: 0,
            settled: None,
            warmth: 0.0,
        });
    }
    w.scouting.film.there.team_camps.push((d, program));
    let names: Vec<String> =
        seen.iter().filter_map(|&id| super::super::place(w, id).map(|i| w.people[i].name.clone())).collect();
    Ok(format!(
        "You took your team to {}'s team camp. Their staff noticed {}.",
        w.program_name(program),
        if names.is_empty() { "nobody in particular".to_string() } else { names.join(", ") }
    ))
}

// --- Signing day (SK38) and combine week (SK39) -----------------------------------------------

/// Today's signings for your room: (id, his name, school he signed with,
/// yours, a flip from another commitment, his ceremony line).
pub fn signings(w: &World) -> Vec<(u32, String, usize, bool, bool, String)> {
    let d = w.today;
    let mine = match readers::seat_staff(w) {
        Some(Staff::Program(c)) => Some(c),
        _ => None,
    };
    let school = crate::world::scouting::hs::school(w);
    let board: std::collections::HashSet<u32> = w.scouting.dept.my_players.iter().map(|f| f.id).collect();
    let mut out = Vec::new();
    for p in &w.people {
        let Some(s) = p.cal.signed.filter(|s| s.date == d) else { continue };
        let ours = Some(s.program) == mine;
        let of_school = school == Some(p.hs);
        let offered = mine.is_some_and(|c| p.cal.offers.contains(&c));
        if !(ours || of_school || offered || board.contains(&p.player.id)) {
            continue;
        }
        let flip = s.flipped;
        let hats = key_uniform(0x4A75 ^ d.0 as u64, p.player.id as u64) < 0.6;
        let line = if hats {
            format!("At his school's ceremony {} reaches past the other hats on the table.", p.name)
        } else {
            format!("{} signs at his kitchen table with his family around him.", p.name)
        };
        out.push((p.player.id, p.name.clone(), s.program, ours, flip, line));
    }
    out.sort_by_key(|x| (std::cmp::Reverse(x.3), x.0));
    out
}

/// Is today a signing day (the early period's first day or February's)?
pub fn signing_today(w: &World) -> bool {
    let d = w.today;
    w.dates.signing_day == d || w.dates.early_signing == Some(d)
}

/// The combine's workout groups by day (2022 on: defensive line and
/// linebackers Thursday, backs and tight ends Friday, quarterbacks,
/// receivers and backs Saturday, the offensive line Sunday).
pub fn combine_group(w: &World) -> Option<(u8, &'static [crate::positions::Position])> {
    use crate::positions::Position::*;
    let d = w.today;
    let end = w.dates.combine_end;
    if d < w.dates.combine_start || d > end {
        return None;
    }
    // The on-field workouts run Thursday to Sunday, a group a day; the
    // days before are interviews and medicals.
    match d.weekday() {
        Weekday::Thu => Some((1, &[EDGE, DT, LB])),
        Weekday::Fri => Some((2, &[CB, S, TE])),
        Weekday::Sat => Some((3, &[QB, WR, RB])),
        Weekday::Sun => Some((4, &[OT, IOL, K, P, LS])),
        _ => Some((0, &[])),
    }
}
