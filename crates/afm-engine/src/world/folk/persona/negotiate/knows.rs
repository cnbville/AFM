//! What another club's GM knows about his own club (6.7: "learn why he's
//! buying or selling"): his mode, his needs, his cap room, and why he'd
//! move one of his men, by the AI's own trade rule. Lane N1's `Tells`
//! decides how much of it he tells you. Everything else is lane N1's.

use super::super::Them;
use super::super::knows::{Known, Knows, Source, Topic};
use super::{Facet, gm_club};
use crate::positions::Position;
use crate::world::World;
use crate::world::folk::cast;
use crate::world::folk::claims::words::spot;
use crate::world::pro::{self, money, office::Mode};

/// Lane N1's knowing, with a GM's own club on a negotiation call.
pub struct Knowing<'a> {
    pub facet: Facet,
    pub base: &'a dyn Knows,
}

fn known(topic: Topic, about: Option<Them>, eye: f64, words: String) -> Known {
    Known {
        topic,
        about,
        from: Source::Job,
        eye,
        point: None,
        says: Vec::new(),
        words,
        sure: (eye / 20.0).clamp(0.4, 0.95) as f32,
        day: None,
    }
}

/// His club as he knows it: where it stands this year, its holes, its
/// cap room.
pub fn club_facts(w: &World, club: usize, eye: f64) -> Vec<Known> {
    let mut v = Vec::new();
    let mode = match w.clubs[club].office.mode {
        Mode::Contend => "They're going for it this year.",
        Mode::Middle => "They're somewhere in the middle this year.",
        Mode::Rebuild => "They're rebuilding: young players and picks.",
    };
    v.push(known(Topic::Club, None, eye, mode.into()));
    let mut holes: Vec<(Position, f64)> = Position::ALL
        .iter()
        .filter(|p| !p.is_specialist())
        .map(|&p| (p, crate::career::need(&w.nfl[club], p)))
        .filter(|x| x.1 >= 1.2)
        .collect();
    holes.sort_by(|a, b| b.1.total_cmp(&a.1));
    if !holes.is_empty() {
        let spots: Vec<&str> = holes.iter().take(2).map(|x| spot(x.0)).collect();
        v.push(known(Topic::Club, None, eye, format!("They need help at {}.", spots.join(" and "))));
    }
    if money::capped(w.year) {
        let room = pro::cap_room(w, club, w.year) / money::price_cap(w.year);
        let words = if room > 0.1 {
            "They have plenty of cap room."
        } else if room > 0.03 {
            "They have some cap room."
        } else {
            "They're tight against the cap."
        };
        v.push(known(Topic::Club, None, eye, words.into()));
    }
    v
}

impl Knows for Knowing<'_> {
    fn knows(&self, w: &World, him: Them, topic: Topic, about: Option<Them>) -> Vec<Known> {
        let Facet::Gm { club, .. } = self.facet else { return self.base.knows(w, him, topic, about) };
        if gm_club(w, him) != Some(club) || !matches!(topic, Topic::Club | Topic::Wants | Topic::Market) {
            return self.base.knows(w, him, topic, about);
        }
        let eye = match him {
            Them::Cast(r, _) => cast::get(w, r).judgement,
            _ => 10.0,
        };
        match about {
            None => club_facts(w, club, eye),
            Some(Them::Player(id)) => {
                let Some(i) = crate::world::folk::person(w, id) else { return Vec::new() };
                if w.people[i].pro.as_ref().and_then(|c| c.team) != Some(club) {
                    return self.base.knows(w, him, topic, about);
                }
                let why = super::trade::why_move(w, club, i);
                let name = w.people[i].name.clone();
                let words = if why.is_empty() {
                    format!("They aren't looking to move {name}.")
                } else {
                    format!("They'd listen on {name}: {}.", why.join(", "))
                };
                vec![known(Topic::Club, about, eye, words)]
            }
            Some(_) => self.base.knows(w, him, topic, about),
        }
    }
}
