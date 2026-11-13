//! Your depth order names the line (playtest bug 12): a man you put at
//! left tackle plays left tackle, a named man keeps his spot when another
//! can't dress, a package's line labels hold, and the seating the screens
//! read is the game's first snap.

use crate::game::flow::{PracticeSnap, practice_snaps, seating};
use crate::game::playbook::ORole;
use crate::game::scrimmage::{DSlot, orole_label};
use crate::game::stats::{GameStats, PlayRecord, PlayType};
use crate::game::synthetic::synthetic_league;
use crate::game::{GameTeam, quick_game};
use crate::player::Level;
use crate::positions::Position;

const LINE: [ORole; 5] = [ORole::LT, ORole::LG, ORole::C, ORole::RG, ORole::RT];

/// The scrimmage snaps `side` ran on offense with every man's spot booked.
fn snaps(stats: &GameStats, side: usize) -> impl Iterator<Item = &PlayRecord> {
    stats.plays.iter().filter(move |r| {
        r.offense == side
            && matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble)
            && !r.on_field[0].is_empty()
            && r.roles[0].len() == r.on_field[0].len()
            && r.roles[1].len() == r.on_field[1].len()
    })
}

/// The man at a spot on the offense, and on the defense.
fn at(r: &PlayRecord, role: ORole) -> Option<u32> {
    r.roles[0].iter().position(|&x| x == role as u8).map(|k| r.on_field[0][k])
}

fn dat(r: &PlayRecord, slot: DSlot) -> Option<u32> {
    r.roles[1].iter().position(|&x| x == slot as u8).map(|k| r.on_field[1][k])
}

/// The five on the line as the staff seats them, left to right.
fn staff_line(team: &GameTeam, level: Level) -> [u32; 5] {
    let s = seating(team, level);
    LINE.map(|r| s.at(orole_label(r), 0).expect("five linemen"))
}

/// The team with the tackles flipped and the inside three turned one
/// place by hand, and where each man should then line up.
fn flipped(team: &GameTeam, level: Level) -> (GameTeam, [(ORole, u32); 5]) {
    let [lt, lg, c, rg, rt] = staff_line(team, level);
    let mut mine = team.clone();
    mine.order = vec![(Position::OT, vec![rt, lt]), (Position::IOL, vec![rg, lg, c])];
    (mine, [(ORole::LT, rt), (ORole::LG, rg), (ORole::C, lg), (ORole::RG, c), (ORole::RT, lt)])
}

#[test]
fn a_line_set_by_hand_plays_where_it_is_put() {
    for level in [Level::HS, Level::FBS, Level::NFL] {
        let teams = synthetic_league(level, 8, 23);
        let (mut all, mut held) = (0usize, 0usize);
        for t in 0..4 {
            let (mine, want) = flipped(&teams[t], level);
            // The screens read the same seating.
            let seats = seating(&mine, level);
            for (role, id) in want {
                assert_eq!(seats.at(orole_label(role), 0), Some(id), "{level:?} team {t} {role:?}");
            }
            let res = quick_game(&mine, &teams[t + 4], level, 77_000 + t as u64);
            for r in snaps(&res.stats, 0) {
                all += 1;
                // Wherever a named man lines up on the line, it is his spot,
                // whoever else is hurt or resting.
                for (role, id) in want {
                    let on = LINE.into_iter().find(|&x| at(r, x) == Some(id));
                    assert!(on.is_none_or(|x| x == role), "{level:?} team {t}: {id} named {role:?} at {on:?}");
                }
                held += want.iter().all(|&(role, id)| at(r, role) == Some(id)) as usize;
            }
            // The other side, whose staff names nobody, lines up as its
            // staff seats it.
            let theirs = staff_line(&teams[t + 4], level);
            let first = snaps(&res.stats, 1).next().expect("an offensive snap");
            assert_eq!(LINE.map(|x| at(first, x)), theirs.map(Some), "{level:?} team {}", t + 4);
        }
        // High school linemen play both ways and rest more: the five are
        // on together less often, but never anywhere else.
        let most = if level == Level::HS { 0.5 } else { 0.8 };
        assert!(all > 150 && held as f64 > most * all as f64, "{level:?}: the five held {held} of {all} snaps");
    }
}

#[test]
fn the_seating_read_is_the_first_snap() {
    for level in [Level::HS, Level::FBS, Level::NFL] {
        let teams = synthetic_league(level, 8, 5);
        let mut base = 0;
        for t in 0..8 {
            let seats = seating(&teams[t], level);
            let res = quick_game(&teams[t], &teams[(t + 3) % 8], level, 31_000 + t as u64);
            let first = snaps(&res.stats, 0).next().expect("an offensive snap");
            for r in LINE {
                assert_eq!(at(first, r), seats.at(orole_label(r), 0), "{level:?} team {t} {r:?}");
            }
            // The same men by position, in the board's slot order.
            assert_eq!(seats.by_slot(Position::OT), [ORole::LT, ORole::RT].map(|r| at(first, r)), "{level:?} team {t}");
            assert_eq!(
                seats.by_slot(Position::LB),
                ["SAM", "MIKE", "WILL"].map(|n| seats.at(n, 0)),
                "{level:?} team {t}"
            );
            assert_eq!(seats.by_slot(Position::CB).len(), 3, "{level:?} team {t}");
            assert!(seats.by_slot(Position::QB).is_empty());
            // The defense's named jobs on its first snap in base, early in
            // the game while everyone is fresh.
            let jobs = [
                (DSlot::Sam, "SAM"),
                (DSlot::Mike, "MIKE"),
                (DSlot::Will, "WILL"),
                (DSlot::FS, "FS"),
                (DSlot::SS, "SS"),
            ];
            let in_base = |r: &&PlayRecord| jobs.iter().all(|j| dat(r, j.0).is_some()) && dat(r, DSlot::NCB).is_none();
            let Some(r) = snaps(&res.stats, 1).take(6).find(in_base) else { continue };
            base += 1;
            for (slot, name) in jobs {
                assert_eq!(dat(r, slot), seats.at(name, 0), "{level:?} team {t} {name}");
            }
            let mut on = [dat(r, DSlot::CBL), dat(r, DSlot::CBR)];
            let mut want = [seats.at("CB", 0), seats.at("CB", 1)];
            on.sort();
            want.sort();
            assert_eq!(on, want, "{level:?} team {t} corners");
        }
        // High school fronts are often not a 4-3 base early on.
        assert!(level == Level::HS || base >= 2, "{level:?}: {base} early snaps in base");
    }
}

#[test]
fn a_named_man_keeps_his_spot_when_another_cannot_dress() {
    let level = Level::NFL;
    let teams = synthetic_league(level, 4, 41);
    for t in 0..2 {
        let (mine, want) = flipped(&teams[t], level);
        let named = |role: ORole| want.iter().find(|x| x.0 == role).expect("a named man").1;
        // Each named man in turn can't dress: the other four stay where
        // they were put and the next man up takes the open spot.
        for (gone_at, gone) in want {
            let mut short = mine.clone();
            short.players.retain(|p| p.id != gone);
            let seats = seating(&short, level);
            for role in LINE {
                let on = seats.at(orole_label(role), 0);
                if role == gone_at {
                    let line = LINE.map(|r| seats.at(orole_label(r), 0));
                    assert!(
                        on.is_some_and(|id| want.iter().all(|x| x.1 != id)),
                        "team {t} {role:?}: {line:?}, named {want:?}"
                    );
                } else {
                    assert_eq!(on, Some(named(role)), "team {t} {role:?} without the {gone_at:?}");
                }
            }
            let res = quick_game(&short, &teams[t + 2], level, 52_000 + t as u64);
            let first = snaps(&res.stats, 0).next().expect("an offensive snap");
            for role in LINE {
                assert_eq!(at(first, role), seats.at(orole_label(role), 0), "team {t} {role:?} on the field");
            }
        }
        // Naming only your left tackle moves the staff's other tackle to
        // the right and leaves the inside three with the staff.
        let [lt, lg, c, rg, rt] = staff_line(&teams[t], level);
        let mut one = teams[t].clone();
        one.order = vec![(Position::OT, vec![rt])];
        let seats = seating(&one, level);
        assert_eq!(LINE.map(|r| seats.at(orole_label(r), 0)), [rt, lg, c, rg, lt].map(Some), "team {t}");
    }
}

#[test]
fn a_package_keeps_the_line_you_labelled() {
    let level = Level::NFL;
    let teams = synthetic_league(level, 4, 63);
    let goal = PracticeSnap { down: 1, togo: 2.0, yardline_100: 2.0, half_left: 900.0, run: Some(true) };
    let mut package = 0;
    for t in 0..2 {
        let staff = staff_line(&teams[t], level);
        let [lt, lg, c, rg, rt] = staff;
        // The goal line offense board (3): QB, LT, LG, C, RG, RT, then the
        // tight ends and backs, left to the staff here.
        let mine_line = [rt, rg, lg, c, lt];
        let mut mine = teams[t].clone();
        mine.prep.packages.push((3, vec![0, rt, rg, lg, c, lt, 0, 0, 0, 0, 0]));
        let stats = practice_snaps(&mine, &teams[t + 2], level, &[goal; 60], 9 + t as u64);
        let mut seen = 0;
        for r in snaps(&stats, 0) {
            let on = LINE.map(|x| at(r, x));
            seen += 1;
            // Heavy sets play the package as labelled; any other set plays
            // the staff's line. Nobody is re-sorted between the two.
            if on == mine_line.map(Some) {
                package += 1;
            } else {
                assert_eq!(on, staff.map(Some), "team {t}: a line that is neither the package nor the staff's");
            }
        }
        assert!(seen >= 50, "team {t}: {seen} snaps");
    }
    assert!(package >= 40, "{package} snaps on the package's line");
}

#[test]
fn a_backup_takes_the_starters_spot_and_nobody_else_moves() {
    use crate::spots::start_at;
    // The inside three, then two backups: he takes the center's spot and
    // the center becomes the first backup.
    assert_eq!(start_at(vec![1, 2, 3, 4, 9], 3, 9, 2, None), vec![1, 9, 3, 2, 4]);
    // The starter the coach named holds no spot here (a tackle, say): the
    // last starter on the staff's order sits, as he did before.
    assert_eq!(start_at(vec![1, 2, 3, 4, 9], 3, 9, 77, Some(1)), vec![9, 2, 3, 1, 4]);
    assert_eq!(start_at(vec![1, 2, 3, 4, 9], 3, 9, 77, None), vec![1, 2, 9, 3, 4]);
    // He already holds a spot: nothing moves.
    assert_eq!(start_at(vec![1, 9, 3, 4], 3, 9, 1, None), vec![1, 9, 3, 4]);
    // No named spots (a quarterback): to the top, as before.
    assert_eq!(start_at(vec![1, 2, 9], 0, 9, 1, None), vec![9, 1, 2]);
    assert_eq!(start_at(vec![1], 0, 9, 1, None), vec![9, 1]);
    // Fewer men than spots: to the top.
    assert_eq!(start_at(vec![1, 2], 3, 9, 1, None), vec![9, 1, 2]);
}

#[test]
fn a_ratio_list_says_who_starts_not_where() {
    // A CFL club meeting the ratio gets an order ranked by rating where
    // nobody set the position (`GameTeam::ranked`): its line is seated by
    // fit, as with no order at all.
    let level = Level::NFL;
    for (t, team) in synthetic_league(level, 4, 87).into_iter().enumerate() {
        let mut ranked = team.clone();
        for pos in [Position::OT, Position::IOL] {
            let mut v: Vec<&crate::game::GamePlayer> = team.players.iter().filter(|p| p.position == pos).collect();
            v.sort_by(|a, b| b.overall_at(pos).total_cmp(&a.overall_at(pos)));
            ranked.order.push((pos, v.iter().map(|p| p.id).collect()));
            ranked.ranked.push(pos);
        }
        assert_eq!(seating(&ranked, level).line, seating(&team, level).line, "team {t}");
        // Your own order at the same positions names the spots.
        let mut yours = ranked.clone();
        yours.ranked.clear();
        let ids = |pos: Position| yours.order.iter().find(|x| x.0 == pos).unwrap().1.clone();
        let (ot, iol) = (ids(Position::OT), ids(Position::IOL));
        let seats = seating(&yours, level);
        assert_eq!(
            LINE.map(|r| seats.at(orole_label(r), 0)),
            [ot[0], iol[0], iol[1], iol[2], ot[1]].map(Some),
            "team {t}: your order's slots"
        );
    }
}
