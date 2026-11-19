//! Parts of a record kept only once they hold something (memory diet,
//! lane M1, `docs/design/memory-diet.md` items 2 and 4).
//!
//! Most of the world's 1.16 million people are high schoolers who never
//! touch their college-and-later fields, and two in three of them have a
//! blank recruiting calendar. `Stored<T>` keeps such a part behind a box
//! that exists only once something is written to it: reading an empty one
//! gives the part's blank value (`T::default()`), writing to it makes the
//! box first. Every value a reader sees is the same as before, so results
//! don't change; only where the bytes live does.
//!
//! - Read through `Deref` (`p.cal.lean`): an empty part reads as blank.
//! - Write through `DerefMut` (`p.cal.lean = 1.2`): the box is made then.
//!   A loop that writes blank values to everyone would make every box, so
//!   such writers go through `stored_mut` (only parts that exist) instead.
//! - `tidy` drops a part that is blank again (at the season's reset and on
//!   load), so a world read from a file keeps only the parts in use.
//!
//! The parts are written and read in world files exactly as before (each
//! record's fields flat, by name): see `Person`'s own `Serialize`.

use std::ops::{Deref, DerefMut};

/// A part that has a blank value every empty part reads as.
pub trait Blank: Default + PartialEq + Sized + 'static {
    /// The blank value, made once for the life of the program.
    fn blank() -> &'static Self;
}

/// Gives a type its blank value (`Default`, made once).
macro_rules! blank {
    ($t:ty) => {
        impl $crate::world::stored::Blank for $t {
            fn blank() -> &'static Self {
                static BLANK: std::sync::LazyLock<$t> = std::sync::LazyLock::new(<$t>::default);
                &BLANK
            }
        }
    };
}
pub(crate) use blank;

/// A part kept only once it holds something.
#[derive(Clone, PartialEq)]
pub struct Stored<T: Blank>(Option<Box<T>>);

impl<T: Blank> Default for Stored<T> {
    fn default() -> Self {
        Stored(None)
    }
}

impl<T: Blank> Stored<T> {
    /// A part with this value: kept only if it isn't blank.
    pub fn new(v: T) -> Self {
        // Checked before it is boxed: a load reads a million blank parts,
        // and boxing each to drop it would leave the allocator's pages full
        // of holes.
        if v == *T::blank() { Stored(None) } else { Stored(Some(Box::new(v))) }
    }
    /// The part, if it holds something (the rest read as blank).
    pub fn stored(&self) -> Option<&T> {
        self.0.as_deref()
    }
    /// The part to change, if it holds something: for writers that only
    /// clear or reset, so they never make a box for a blank part.
    pub fn stored_mut(&mut self) -> Option<&mut T> {
        self.0.as_deref_mut()
    }
    /// Drops the part if it is blank again.
    pub fn tidy(&mut self) {
        if self.0.as_deref().is_some_and(|v| v == T::blank()) {
            self.0 = None;
        }
    }
}

impl<T: Blank> Deref for Stored<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        match &self.0 {
            Some(b) => b,
            None => T::blank(),
        }
    }
}

impl<T: Blank> DerefMut for Stored<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        self.0.get_or_insert_with(Box::default)
    }
}

impl<T: Blank + std::fmt::Debug> std::fmt::Debug for Stored<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}

/// Written as the part itself (blank when empty), read back the same way.
impl<T: Blank + serde::Serialize> serde::Serialize for Stored<T> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        (**self).serialize(s)
    }
}

impl<'de, T: Blank + serde::Deserialize<'de>> serde::Deserialize<'de> for Stored<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        T::deserialize(d).map(Stored::new)
    }
}
