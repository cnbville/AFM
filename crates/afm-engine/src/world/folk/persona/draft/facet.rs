//! The draft side's people, on lane N1's (`people::People`): what your
//! scouting people know (their own looks in your file), what another
//! club's GM says about his interest (`smoke::gm_says`), and the asks only
//! this facet runs (the whiteboard, finding something out, planting
//! smoke). Everyone else, and everything else, is lane N1's and N3's.
//!
//! Your own scouting people answer you straight: it's their job, and what
//! they know is a read, never a secret. A rival GM answers on N1's rule
//! (he can deflect or say the safe half), and what he says is his smoke or
//! his real interest as `gm_says` decides.

use super::super::Them;
use super::super::asks::Asks;
use super::super::knows::{Answer, AskCost, Known, Knows, Source, TellParts, Tells, Told, Topic};
use super::super::people::People;
use super::super::talk::{DEALS, Parts, feel};
use super::super::terms::Request;
use super::super::wants::{Acting, Ran, Willing};
use crate::world::World;

/// The draft side's knowing, telling and asking.
pub struct Facet;

/// The parts a draft-side talk is played with: this facet's knowing,
/// telling and asking over lane N1's people and N3's deals.
pub fn parts() -> Parts<'static> {
    static P: People = People;
    static X: Facet = Facet;
    static F: feel::Sized = feel::Sized;
    Parts { knows: &X, tells: &X, wanting: &P, valuing: &DEALS, acting: &X, feel: &F }
}

/// Minutes your own people take to answer a question.
pub const ANSWER_MINUTES: f32 = 1.5;

fn known(topic: Topic, about: Option<Them>, eye: f64, words: String) -> Known {
    Known {
        topic,
        about,
        from: Source::Job,
        eye,
        point: None,
        says: Vec::new(),
        words,
        sure: (0.35 + 0.65 * eye / 20.0) as f32,
        day: None,
    }
}

impl Knows for Facet {
    fn knows(&self, w: &World, him: Them, topic: Topic, about: Option<Them>) -> Vec<Known> {
        // Your own scouting people: their own looks in your file.
        if let Some(code) = super::scouts::code_of(w, him) {
            let Some(Them::Player(id)) = about else {
                return vec![known(topic, about, 12.0, "Who do you want me on? Give me a name.".into())];
            };
            let mut words = super::scouts::answer(w, code, id, topic);
            if words.is_empty() {
                words.push("I haven't seen him myself. I can go find out.".into());
            }
            return words.into_iter().map(|x| known(topic, about, 13.0, x)).collect();
        }
        // Another club's GM on his club's interest: real or smoke.
        if let Them::Cast(r, _) = him
            && r.role == crate::world::folk::cast::Role::Gm
            && let crate::world::folk::Team::Nfl(t) = r.team
            && super::club(w).is_some_and(|m| m != t)
            && matches!(topic, Topic::ScoutFile | Topic::Club | Topic::Wants | Topic::Football | Topic::Nothing)
        {
            let ask = match about {
                Some(Them::Player(id)) => Some(id),
                _ => None,
            };
            if let Some((id, _)) = super::smoke::gm_says(w, t, ask)
                && let Some(i) = crate::world::folk::person(w, id)
            {
                let name = &w.people[i].name;
                let says = match (ask, super::smoke::gm_says(w, t, ask).is_some_and(|x| x.1)) {
                    // His club, in his own words ("they" read as someone
                    // else's, the talks audit of 2026-11-16).
                    (Some(_), true) => format!("He says: \"We like {name} a lot.\""),
                    (Some(_), false) => format!("He says: \"{name} isn't really our kind of player.\""),
                    (None, _) => format!("He says: \"We love {name}.\""),
                };
                return vec![known(topic, Some(Them::Player(id)), 10.0, says)];
            }
        }
        People.knows(w, him, topic, about)
    }
}

impl Tells for Facet {
    fn tell(&self, w: &World, him: Them, known: &[Known], parts: &TellParts) -> Told {
        if super::scouts::yours(w, him) {
            return Told {
                answer: Answer::Straight,
                released: known.iter().map(|k| k.words.clone()).collect(),
                holding_back: false,
                tell: String::new(),
                lie: None,
                cost: AskCost { minutes: ANSWER_MINUTES, trust: 0.0 },
            };
        }
        People.tell(w, him, known, parts)
    }
}

impl Acting for Facet {
    fn willing(&self, w: &World, him: Them, ask: &Request) -> Willing {
        match ask {
            // A prospect in your interview room goes to the board.
            Request::Whiteboard => match him {
                Them::Player(id) if in_room(w, id) => Willing::Yes,
                _ => Willing::No,
            },
            Request::FindOut(..) if super::scouts::yours(w, him) => Willing::Yes,
            Request::Scout(_) if super::scouts::yours(w, him) => Willing::Yes,
            // A GM or an agent always takes the information.
            Request::Plant(_) => match him {
                Them::Cast(r, _) if r.role == crate::world::folk::cast::Role::Gm => Willing::Yes,
                Them::Agent(_) => Willing::Yes,
                _ => Willing::No,
            },
            _ => Asks.willing(w, him, ask),
        }
    }

    fn run(&self, w: &mut World, him: Them, ask: &Request) -> Ran {
        let mut ran = Ran { honest: true, ..Ran::default() };
        match (him, ask) {
            (Them::Player(id), Request::Whiteboard) => {
                if let Some(words) = super::whiteboard::run(w, id) {
                    ran.done.extend(words);
                }
            }
            (_, Request::FindOut(id, topic)) => {
                if let Some(x) = super::scouts::send(w, him, *id, *topic) {
                    ran.done.push(x);
                }
            }
            (_, Request::Scout(id)) if super::scouts::yours(w, him) => {
                if let Some(x) = super::scouts::send(w, him, *id, Topic::ScoutFile) {
                    ran.done.push(x);
                }
            }
            (_, Request::Plant(id)) => {
                if let Some(x) = super::smoke::plant(w, him, *id) {
                    ran.done.push(x);
                }
            }
            _ => return Asks.run(w, him, ask),
        }
        ran
    }
}

/// Whether he's in an interview room with you now.
pub fn in_room(w: &World, id: u32) -> bool {
    w.folk.scenes.iter().any(|s| s.kind == crate::world::folk::scene::Kind::CombineTalk && !s.done && s.a == id as u64)
}
