//! Lane SF10 (scheme-fit items 10, 11 and 23): the roster shapes the call.
//! The table's cells and features read back, a team of typical men leans
//! nowhere, and a man's own numbers move the calls his slot's weights say.

use crate::attributes::Attr;
use crate::game::roster_fit::{self, Cell, OffFit, SLOTS, Slot, WEIGHT};
use crate::game::synthetic::synthetic_team;
use crate::game::{GameTeam, OffScheme};
use crate::player::Level;

/// Every cell has its own name and reads back from it.
#[test]
fn cells_read_back() {
    let cells = roster_fit::all_cells();
    for (k, &c) in cells.iter().enumerate() {
        assert_eq!(Cell::from_name(&c.name()), Some(c), "{}", c.name());
        assert!(cells[..k].iter().all(|&d| d.name() != c.name()), "{} twice", c.name());
    }
    for s in Slot::ALL {
        assert_eq!(Slot::from_name(s.name()), Some(s));
    }
    // A feature is a rating's place in `Attr::ALL`, read by its index.
    for (k, a) in Attr::ALL.iter().enumerate() {
        assert_eq!(a.index(), k);
    }
}

/// A team's men set to the typical man at each slot (a slot the roster
/// can't fill, as a small high school's third tight end, stays empty and
/// reads nothing).
fn typical_men(level: Level) -> (GameTeam, roster_fit::Men) {
    let mut t = synthetic_team("T", level, 0.0, OffScheme::WestCoast, 5);
    let mut men: roster_fit::Men = [None; SLOTS];
    let mut used: Vec<usize> = Vec::new();
    for s in Slot::ALL {
        let (pos, _) = s.typical_man();
        let Some(i) = (0..t.players.len()).find(|&i| t.players[i].position == pos && !used.contains(&i)) else {
            continue;
        };
        used.push(i);
        men[s as usize] = Some(i);
        let p = &mut t.players[i];
        for a in Attr::ALL {
            p.attrs.set(a, roster_fit::reference(level, s, a.index() as u8));
        }
        p.weight = roster_fit::reference(level, s, WEIGHT);
    }
    (t, men)
}

/// The typical team's men are the typical men, so nothing leans: every
/// call keeps its weight from the scheme, the looks and the day.
#[test]
fn typical_men_lean_nowhere() {
    for level in [Level::NFL, Level::FBS, Level::HS] {
        let (t, men) = typical_men(level);
        let f = OffFit::of(&t.players, &men, level);
        assert!(f.pass.iter().chain(&f.fake).chain(&f.run).all(|v| v.abs() < 1e-4), "{level:?}: {f:?}");
        let d = roster_fit::DefFit::of(&t.players, &men, level);
        assert!(d.look.iter().all(|v| v.abs() < 1e-4), "{level:?}: {d:?}");
    }
}

/// The run table's index is the run concept's own.
#[test]
fn run_cells_follow_the_concepts() {
    use crate::game::playbook::RUN_CONCEPTS;
    for (k, r) in RUN_CONCEPTS.iter().enumerate() {
        assert_eq!(*r as usize, k, "{r:?}");
    }
}

fn moved(level: Level, changes: &[(Slot, Attr, f64)]) -> (GameTeam, roster_fit::Men) {
    let (mut t, men) = typical_men(level);
    for &(s, a, d) in changes {
        let p = &mut t.players[men[s as usize].unwrap()];
        p.attrs.set(a, p.attrs.get(a) + d);
    }
    (t, men)
}

fn pass_named(f: &OffFit, name: &str) -> f32 {
    let i = crate::game::playbook::PASS_CONCEPTS.iter().position(|p| p.name == name).expect(name);
    f.pass[i]
}

/// A passer with the arm and the deep ball leans to the deep crossers and
/// away from the quick game; a shaky right tackle the other way.
#[test]
fn deep_ball_leans_deep() {
    let level = Level::NFL;
    let (t, men) = moved(level, &[(Slot::Qb, Attr::DeepAccuracy, 15.0), (Slot::Qb, Attr::ArmStrength, 15.0)]);
    let f = OffFit::of(&t.players, &men, level);
    let (deep, quick) = (pass_named(&f, "Y-cross"), pass_named(&f, "slant-flat"));
    assert!(deep - quick > 0.5, "Y-cross {deep}, slant-flat {quick}");
    let (t, men) = moved(level, &[(Slot::Ot2, Attr::PassBlocking, -15.0)]);
    let f = OffFit::of(&t.players, &men, level);
    let (deep, quick) = (pass_named(&f, "dagger"), pass_named(&f, "slant-flat"));
    assert!(quick - deep > 0.3, "dagger {deep}, slant-flat {quick}");
}

/// A room of man corners gives up less in Cover 1 than in Cover 3, so the
/// coordinator leans to man.
#[test]
fn man_corners_lean_to_man() {
    use crate::game::calling::col;
    use crate::game::playbook::Coverage;
    let level = Level::NFL;
    let (t, men) = moved(
        level,
        &[
            (Slot::Cb1, Attr::ManCoverage, 15.0),
            (Slot::Cb2, Attr::ManCoverage, 15.0),
            (Slot::Nb, Attr::ManCoverage, 15.0),
        ],
    );
    let d = roster_fit::DefFit::of(&t.players, &men, level);
    let (man, zone) = (d.look[col(Coverage::Cover1, false)], d.look[col(Coverage::Cover3, false)]);
    assert!(zone - man > 0.5, "Cover 1 {man}, Cover 3 {zone}");
}

/// Tight ends who block lean the runs to 12 and 13 personnel.
#[test]
fn blocking_tight_ends_lean_heavy() {
    use crate::game::playbook::Personnel;
    let level = Level::NFL;
    let (t, men) = moved(level, &[(Slot::Te2, Attr::RunBlocking, 15.0), (Slot::Te3, Attr::RunBlocking, 15.0)]);
    let f = OffFit::of(&t.players, &men, level);
    let (p11, p12, p13) =
        (f.group_of(Personnel::P11, false), f.group_of(Personnel::P12, false), f.group_of(Personnel::P13, false));
    assert!(p12 - p11 > 0.1 && p13 - p11 > 0.1, "11 {p11}, 12 {p12}, 13 {p13}");
}

/// A heavier back who drives his legs converts third and short more often
/// than a light one.
#[test]
fn heavy_back_converts_short() {
    let level = Level::NFL;
    let (mut t, men) = typical_men(level);
    let i = men[Slot::Rb as usize].unwrap();
    let base = t.players[i].weight;
    t.players[i].weight = base + 30.0;
    let heavy = roster_fit::back_value(&t.players[i], level, true);
    t.players[i].weight = base - 30.0;
    let light = roster_fit::back_value(&t.players[i], level, true);
    assert!(heavy > light, "heavy {heavy}, light {light}");
}
