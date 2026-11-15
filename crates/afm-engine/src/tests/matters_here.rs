//! What matters at a position (`world::scouting::file::matters_here`): the
//! cutoff is a share of the position's biggest rating weight, so every
//! position has a short list of ratings that matter (a fixed 0.4 on the
//! weights' own shares never let one through).

use crate::attributes::Attr;
use crate::positions::{Position, weight};
use crate::world::scouting::file::matters_here;

#[test]
fn every_position_has_a_short_list_of_ratings_that_matter() {
    for &pos in Position::ALL.iter() {
        let listed = Attr::ALL.iter().filter(|a| weight(pos, **a) > 0.0).count();
        let matter = Attr::ALL.iter().filter(|a| matters_here(pos, **a)).count();
        assert!(matter >= 1, "{pos:?}: nothing matters");
        // A position with a long list of ratings leaves some out.
        assert!(listed < 8 || matter < listed, "{pos:?}: all {listed} listed ratings matter, none is left out");
        // A rating the position doesn't count never matters there.
        assert!(Attr::ALL.iter().filter(|a| weight(pos, **a) == 0.0).all(|a| !matters_here(pos, *a)));
    }
    // The ones a coach would name.
    assert!(matters_here(Position::QB, Attr::ArmStrength) || matters_here(Position::QB, Attr::Decisions));
}
