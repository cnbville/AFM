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
        let f = OffFit::of(&t.players, &men, level, None);
        assert!(f.pass.iter().chain(&f.fake).chain(&f.run).all(|v| v.abs() < 1e-4), "{level:?}: {f:?}");
        let d = roster_fit::DefFit::of(&t.players, &men, level, None);
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
    let f = OffFit::of(&t.players, &men, level, None);
    let (deep, quick) = (pass_named(&f, "Y-cross"), pass_named(&f, "slant-flat"));
    assert!(deep - quick > 0.5, "Y-cross {deep}, slant-flat {quick}");
    let (t, men) = moved(level, &[(Slot::Ot2, Attr::PassBlocking, -15.0)]);
    let f = OffFit::of(&t.players, &men, level, None);
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
    let d = roster_fit::DefFit::of(&t.players, &men, level, None);
    let (man, zone) = (d.look[col(Coverage::Cover1, false)], d.look[col(Coverage::Cover3, false)]);
    assert!(zone - man > 0.5, "Cover 1 {man}, Cover 3 {zone}");
}

/// Tight ends who block lean the runs to 12 and 13 personnel.
#[test]
fn blocking_tight_ends_lean_heavy() {
    use crate::game::playbook::Personnel;
    let level = Level::NFL;
    let (t, men) = moved(level, &[(Slot::Te2, Attr::RunBlocking, 15.0), (Slot::Te3, Attr::RunBlocking, 15.0)]);
    let f = OffFit::of(&t.players, &men, level, None);
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
    let heavy = roster_fit::back_value(&t.players[i], level, None, true);
    t.players[i].weight = base - 30.0;
    let light = roster_fit::back_value(&t.players[i], level, None, true);
    assert!(heavy > light, "heavy {heavy}, light {light}");
}

/// A league's own typical men (realism audit, the run lean): a league whose
/// backs are all 15 points faster than the level's typical men has a
/// typical back that fast, and read about its own men the league's teams
/// lean nowhere on average, on any call or look, where read about the
/// level's typical men they all lean to the toss.
#[test]
fn a_league_reads_its_own_men() {
    use crate::game::playbook::{PASS_CONCEPTS, RunConcept};
    let level = Level::FBS;
    let mut teams: Vec<GameTeam> =
        (0..8).map(|k| synthetic_team(&format!("T{k}"), level, 0.0, OffScheme::WestCoast, 40 + k)).collect();
    for t in teams.iter_mut() {
        for p in t.players.iter_mut().filter(|p| p.position == crate::positions::Position::RB) {
            p.attrs.set(Attr::Speed, p.attrs.get(Attr::Speed) + 15.0);
        }
    }
    let refs: Vec<&GameTeam> = teams.iter().collect();
    let league = crate::game::league_men(&refs, level);
    assert_eq!(league.teams, 8);
    let speed = Attr::Speed.index() as u8;
    let fast = league.get(Slot::Rb, speed).unwrap() - roster_fit::reference(level, Slot::Rb, speed);
    assert!(fast > 10.0, "the league's typical back is {fast:.1} faster than the level's");
    let (mut own, mut level_men) = (OffFit::default(), OffFit::default());
    let mut def = roster_fit::DefFit::default();
    for t in &teams {
        // The league's men as the engine reads them at kickoff.
        let men = crate::game::flow::kickoff_men(t, level);
        let f = OffFit::of(&t.players, &men, level, Some(&league));
        let g = OffFit::of(&t.players, &men, level, None);
        let d = roster_fit::DefFit::of(&t.players, &men, level, Some(&league));
        for k in 0..PASS_CONCEPTS.len() {
            own.pass[k] += f.pass[k] / 8.0;
            own.fake[k] += f.fake[k] / 8.0;
        }
        for k in 0..own.run.len() {
            own.run[k] += f.run[k] / 8.0;
            level_men.run[k] += g.run[k] / 8.0;
        }
        for c in 0..def.look.len() {
            def.look[c] += d.look[c] / 8.0;
        }
    }
    let worst = own.pass.iter().chain(&own.fake).chain(&own.run).chain(&def.look).fold(0.0f32, |m, v| m.max(v.abs()));
    assert!(worst < 0.02, "the league's mean lean reaches {worst}");
    let toss = RunConcept::Toss as usize;
    let (inside, toss_lean) = (level_men.run[RunConcept::InsideZone as usize], level_men.run[toss]);
    assert!(toss_lean - inside > 0.1, "about the level's men: toss {toss_lean}, inside zone {inside}");
    assert!(own.run[toss].abs() < 0.02, "about its own men the toss leans {}", own.run[toss]);
}

/// The league's men ride with the team: a replay or a live game packed and
/// read back leans as the game did, and a table from before a new rating
/// reads as no table (the level's typical men).
#[test]
fn a_league_rides_with_the_team() {
    let level = Level::NFL;
    let teams: Vec<GameTeam> =
        (0..4).map(|k| synthetic_team(&format!("T{k}"), level, 0.0, OffScheme::WestCoast, 60 + k)).collect();
    let refs: Vec<&GameTeam> = teams.iter().collect();
    let league = std::sync::Arc::new(crate::game::league_men(&refs, level));
    let mut home = teams[0].clone();
    home.league = Some(league.clone());
    let away = teams[1].clone();
    let setup = crate::game::GameSetup::new(&home, &away, level, 11);
    let packed = crate::game::ReplaySetup::of(&setup, None).pack();
    let back = crate::game::ReplaySetup::unpack(&packed).expect("the replay reads back");
    assert_eq!(back.home.league.as_deref(), Some(&*league));
    assert!(back.away.league.is_none());
    let men = crate::game::flow::kickoff_men(&home, level);
    let f = OffFit::of(&home.players, &men, level, home.league.as_deref());
    let g = OffFit::of(&back.home.players, &men, level, back.home.league.as_deref());
    assert_eq!(f, g);
    let old: roster_fit::LeagueMen = serde_json::from_str(r#"{"men":[1.0,2.0],"cols":2,"teams":3}"#).unwrap();
    assert_eq!(old.get(Slot::Qb, 0), None);
    let r = roster_fit::reference_in(level, Some(&old), Slot::Qb, 0);
    assert_eq!(r, roster_fit::reference(level, Slot::Qb, 0));
    assert_eq!(old.weight_at(crate::positions::Position::LB), None);
    assert_eq!(old.hitter_at(crate::spots::Spot::FS), None);
}

/// The defense's body reads sit about the league's own men too: a league
/// whose safeties are all 15 points slower and whose linebackers all carry
/// 20 more pounds reads its safety pairs and its blitz lean about its own
/// men as the same league unchanged does, where about the level's typical
/// men every pair reads a quarter more of a hitter and every staff blitzes
/// more.
#[test]
fn a_league_reads_its_own_defenders() {
    use crate::positions::Position;
    let level = Level::NFL;
    let plain: Vec<GameTeam> =
        (0..6).map(|k| synthetic_team(&format!("T{k}"), level, 0.0, OffScheme::WestCoast, 80 + k)).collect();
    let mut moved = plain.clone();
    for t in moved.iter_mut() {
        for p in t.players.iter_mut() {
            match p.position {
                Position::S => p.attrs.set(Attr::Speed, p.attrs.get(Attr::Speed) - 15.0),
                Position::LB => p.weight += 20.0,
                _ => {}
            }
        }
    }
    let leans = |teams: &[GameTeam], own: bool| -> Vec<(f64, f64)> {
        let refs: Vec<&GameTeam> = teams.iter().collect();
        let league = std::sync::Arc::new(crate::game::league_men(&refs, level));
        teams
            .iter()
            .map(|t| {
                let mut t = t.clone();
                t.league = own.then(|| league.clone());
                crate::game::flow::body_leans(&t, level)
            })
            .collect()
    };
    let (plain_own, moved_own) = (leans(&plain, true), leans(&moved, true));
    let (plain_level, moved_level) = (leans(&plain, false), leans(&moved, false));
    for k in 0..plain.len() {
        let (a, b) = (plain_own[k], moved_own[k]);
        assert!((a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-4, "team {k} about its own men: {a:?} then {b:?}");
        let (a, b) = (plain_level[k], moved_level[k]);
        assert!((b.0 - a.0 - 0.25).abs() < 0.02, "team {k}'s safeties about the level's men: {a:?} then {b:?}");
        assert!(
            b.1 - a.1 > 0.015 || b.1 >= 0.06 - 1e-9,
            "team {k}'s blitz lean about the level's men: {a:?} then {b:?}"
        );
    }
}
