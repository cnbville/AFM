//! Fakes for every trait in the contract, for the lanes' tests: each hands
//! back what it was given, and none reads or changes the world. A lane
//! swaps one fake for its real part and keeps the others.

use super::Them;
use super::ear::{Reading, Resolving, Turn};
use super::knows::{Known, Knows, TellParts, Tells, Told, Topic};
use super::room::Room;
use super::terms::{Best, Offer, Request, Valuing, Weighed};
use super::wants::{Acting, Kind, Ran, Want, Wanting, Willing};
use crate::world::World;

/// A person the tests set up by hand.
#[derive(Clone, Debug)]
pub struct Fake {
    pub kind: Kind,
    pub known: Vec<Known>,
    pub told: Told,
    pub wants: Vec<Want>,
    pub willing: Willing,
    pub ran: Ran,
    pub best: Best,
    pub weighed: Weighed,
    pub turn: Turn,
}

impl Default for Fake {
    fn default() -> Fake {
        Fake {
            kind: Kind::Player,
            known: Vec::new(),
            told: Told::default(),
            wants: Vec::new(),
            willing: Willing::No,
            ran: Ran::default(),
            best: Best::Stay,
            weighed: Weighed::No,
            turn: Turn::default(),
        }
    }
}

impl Knows for Fake {
    fn knows(&self, _: &World, _: Them, topic: Topic, about: Option<Them>) -> Vec<Known> {
        self.known.iter().filter(|k| k.topic == topic && k.about == about).cloned().collect()
    }
}

impl Tells for Fake {
    fn tell(&self, _: &World, _: Them, _: &[Known], _: &TellParts) -> Told {
        self.told.clone()
    }
}

impl Wanting for Fake {
    fn kind(&self, _: &World, _: Them) -> Kind {
        self.kind
    }
    fn wants(&self, _: &World, _: Them) -> Vec<Want> {
        self.wants.clone()
    }
}

impl Acting for Fake {
    fn willing(&self, _: &World, _: Them, _: &Request) -> Willing {
        self.willing.clone()
    }
    fn run(&self, _: &mut World, _: Them, _: &Request) -> Ran {
        self.ran.clone()
    }
}

impl Valuing for Fake {
    fn best(&self, _: &World, _: Them) -> Best {
        self.best.clone()
    }
    fn weigh(&self, _: &World, _: Them, _: &Offer) -> Weighed {
        self.weighed.clone()
    }
}

impl Resolving for Fake {
    fn turn(&self, _: &World, _: &Room, _: &Reading) -> Turn {
        self.turn.clone()
    }
    fn apply(&self, _: &mut World, _: &Turn) {}
}
