//! Staking your name on a player (scouting update SK40): you mark him as
//! your guy, privately or out loud. Three years on the look-back settles
//! it. If he hits you get the credit: reputation, your staff's respect and
//! a moment of its own; if he busts it costs you, more if you said it in
//! public. College (a recruit) and the NFL (a draft prospect); the high
//! school side's calls to colleges already work this way.
//!
//! Proposed, not confirmed: five open stakes at once; a college hit is a
//! starter at your level or higher, or drafted; an NFL hit is 24 starts or
//! a Pro Bowl in his first three seasons, a bust fewer than eight starts.
//! Credit: reputation +0.3 (+0.6 in public), your staff's trust +0.02
//! (+0.03); a bust: -0.2 (-0.6), -0.01 (-0.04).

use crate::calendar::{Date, Event, Kind, Level, Loudness};
use crate::world::data::Division;
use crate::world::folk::player_link;
use crate::world::{Stage, World};

/// Open stakes at once.
pub const OPEN_MAX: usize = 5;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Stake {
    pub id: u32,
    pub date: Date,
    pub public: bool,
    /// Staked at an NFL club (a draft prospect).
    pub nfl: bool,
    /// Your division then, for a college stake.
    pub division: Option<Division>,
    /// Some(true) he hit, Some(false) he busted, None: open or in between.
    pub settled: Option<bool>,
    pub done: bool,
    pub turned: Option<String>,
}

/// Your stakes, newest last.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Stakes {
    pub list: Vec<Stake>,
}

fn your_name(w: &World) -> String {
    crate::world::folk::scene::your_name(w)
}

/// Marks a player as your guy.
pub fn stake(w: &mut World, id: u32, public: bool) -> Result<String, String> {
    let seat = w.seat.ok_or("Start a career first.")?;
    let nfl = match seat.level {
        Level::Nfl => true,
        Level::College => false,
        _ => return Err("At a high school your word goes to the colleges themselves.".into()),
    };
    let i = super::place(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    let fits = if nfl {
        matches!(p.stage, Stage::College | Stage::Declared) && p.pro.is_none()
    } else {
        matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)
    };
    if !fits {
        return Err(
            if nfl { "Only a draft prospect: a college player or a declared one." } else { "Only a recruit." }.into()
        );
    }
    let list = &w.scouting.stakes.list;
    if list.iter().any(|s| s.id == id && !s.done) {
        return Err("Your name is on him already.".into());
    }
    if list.iter().filter(|s| !s.done).count() >= OPEN_MAX {
        return Err(format!("You have {OPEN_MAX} open calls: a name staked on everyone means nothing."));
    }
    let division = seat.program.map(|j| w.programs[j].division);
    let name = p.name.clone();
    let pos = p.player.position;
    let name_out = name.clone();
    w.scouting.stakes.list.push(Stake {
        id,
        date: w.today,
        public,
        nfl,
        division,
        settled: None,
        done: false,
        turned: None,
    });
    let you = your_name(w);
    if public {
        let mut e = Event::new(
            w.today,
            if nfl { Level::Nfl } else { Level::Recruiting },
            Kind::News,
            Loudness::Small,
            format!("{you} on {pos} {name}: \"Mark it down. He's going to be special.\""),
        )
        .person(id, &name);
        if let Some(j) = seat.program {
            e = e.program(j);
        }
        if let Some(t) = seat.nfl_team {
            e = e.team(t);
        }
        w.calendar.post(e);
        w.people[i].note(format!("{you} staked his name on him in public."));
    }
    Ok(if public {
        format!("You said it out loud: {name_out} is your guy. Three years will tell.")
    } else {
        format!("Your name is on {name_out}, between you and your staff. Three years will tell.")
    })
}

/// How an NFL stake turned out three seasons into his career: Some(hit),
/// None while it is too soon.
fn nfl_turned(w: &World, i: usize, staked: Date) -> Option<(bool, String)> {
    let p = &w.people[i];
    let Some(pro) = p.pro.as_ref() else {
        // Undrafted and never signed: a bust once his class has gone by.
        return (staked.add(2 * 365) <= w.today).then(|| (false, "never made an NFL roster".to_string()));
    };
    let first = pro.draft.map_or(staked.year() + 1, |d| d.0);
    if w.year < first + 3 {
        return None;
    }
    let early: Vec<_> = pro.seasons.iter().filter(|s| s.year < first + 3).collect();
    let starts: u32 = early.iter().map(|s| s.starts).sum();
    let bowl = early.iter().any(|s| s.pro_bowl);
    Some(if bowl {
        (true, format!("a Pro Bowl in his first three seasons, {starts} starts"))
    } else if starts >= 24 {
        (true, format!("a starter: {starts} starts in three seasons"))
    } else if starts < 8 {
        (false, format!("{starts} starts in three seasons"))
    } else {
        (false, format!("a part-timer: {starts} starts in three seasons"))
    })
}

/// September 1: stakes three years old settle; a bust or a hit lands.
pub(crate) fn day(w: &mut World, d: Date) {
    if !(d.parts().1 == 9 && d.parts().2 == 1) || w.scouting.stakes.list.iter().all(|s| s.done) {
        return;
    }
    let mut out: Vec<(usize, bool, String, bool)> = Vec::new();
    for (k, s) in w.scouting.stakes.list.iter().enumerate() {
        if s.done || s.date.add(3 * 365 - 30) > d {
            continue;
        }
        let Some(i) = super::place(w, s.id) else {
            out.push((k, false, "out of football".into(), s.public));
            continue;
        };
        let r = if s.nfl {
            nfl_turned(w, i, s.date)
        } else {
            Some(crate::world::front_office::chain::turned(w, s.id, s.division.unwrap_or(Division::FBS)))
        };
        if let Some((hit, words)) = r {
            out.push((k, hit, words, s.public));
        }
    }
    let you = w.career.as_ref().map(|c| c.coach);
    for (k, hit, words, public) in out {
        let id = w.scouting.stakes.list[k].id;
        let s = &mut w.scouting.stakes.list[k];
        s.done = true;
        s.settled = Some(hit);
        s.turned = Some(words.clone());
        let rep = match (hit, public) {
            (true, true) => 0.6,
            (true, false) => 0.3,
            (false, true) => -0.6,
            (false, false) => -0.2,
        };
        let trust: f32 = match (hit, public) {
            (true, true) => 0.03,
            (true, false) => 0.02,
            (false, true) => -0.04,
            (false, false) => -0.01,
        };
        if let Some(c) = you {
            w.coaches[c].reputation = (w.coaches[c].reputation + rep).clamp(1.0, 20.0);
        }
        for m in crate::world::staff_read::members(w) {
            w.folk.mine.nudge(m.key, trust);
        }
        let Some(i) = super::place(w, id) else { continue };
        let (name, link) = (w.people[i].name.clone(), player_link(w, i));
        let text = if hit {
            format!(
                "You staked your name on {link}{} and he came through: {words}. Your staff saw you call it.",
                if public { " out loud" } else { "" }
            )
        } else {
            format!(
                "The call you staked your name on{}, {link}, didn't come through: {words}.",
                if public { " in public" } else { "" }
            )
        };
        crate::world::front_office::tell_about(w, d, text, false, &[i]);
        if public {
            let you = your_name(w);
            let line = if hit {
                format!("{you} called it on {name} three years ago, and it came true: {words}.")
            } else {
                format!("{you} staked his name on {name} three years ago. It hasn't aged well: {words}.")
            };
            let level = if w.seat.is_some_and(|s| s.level == Level::Nfl) { Level::Nfl } else { Level::College };
            w.calendar.post(Event::new(d, level, Kind::News, Loudness::Small, line).person(id, &name));
        }
    }
}

/// Your stake on a player, if any.
pub fn on(w: &World, id: u32) -> Option<&Stake> {
    w.scouting.stakes.list.iter().rev().find(|s| s.id == id)
}
