//! The people themselves (lane N1): `People` fills the contract's
//! `Knows`, `Tells` and `Wanting` from the world, for every kind of person
//! (section 6). Lanes N3 (`Acting`, `Valuing`) and N4 (`Resolving`) read
//! it; their fakes stand in until then.

use super::Them;
use super::knows::{Known, Knows, TellParts, Tells, Told, Topic};
use super::wants::{Kind, Want, Wanting};
use crate::world::World;

/// Everyone you can talk to, read from the world.
#[derive(Clone, Copy, Debug, Default)]
pub struct People;

impl Knows for People {
    fn knows(&self, w: &World, him: Them, topic: Topic, about: Option<Them>) -> Vec<Known> {
        super::knowing::knows(w, him, topic, about)
    }
}

impl Tells for People {
    fn tell(&self, w: &World, him: Them, known: &[Known], parts: &TellParts) -> Told {
        super::telling::tell(w, him, known, parts)
    }
}

impl Wanting for People {
    fn kind(&self, w: &World, him: Them) -> Kind {
        super::makeup::kind(w, him)
    }
    fn wants(&self, w: &World, him: Them) -> Vec<Want> {
        super::wanting::wants(w, him)
    }
}
