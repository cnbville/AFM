//! Lane 9: the film room and being there. A session watched start to
//! finish (the snaps, the calls, the verdict, the report, your read), the
//! one ledger of film hours (SK60, SK61), a tired eye (SK59), and the
//! opponent's tape (SK54).

use crate::world::scouting::film::{self as fm, session, tape, time};
use crate::world::scouting::{self as sc, readers};
use crate::world::seat::{Job, You};
use crate::world::{Stage, World, WorldOptions};

fn nfl_world() -> World {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    let t = (0..w.clubs.len())
        .filter(|&t| crate::world::franchise::active(t, w.year))
        .max_by_key(|&t| w.clubs[t].roster.len())
        .unwrap();
    let teaching = vec![12.0; crate::world::coach::GROUP_COUNT];
    let you: You = serde_json::from_value(serde_json::json!({
        "name": "Test Coach", "age": 45, "ethnicity": "White", "home_state": 0, "ex_pro": false, "reputation": 60.0,
        "teaching": teaching, "scheme": 12.0, "game_management": 12.0,
        "motivation": 12.0, "discipline": 12.0, "recruiting": 12.0, "network": 12.0, "relationships": 12.0,
        "style": { "risk": 10.0, "freedom": 10.0, "physical": 10.0, "ball_hunting": 10.0, "mould": 10.0 },
        "yours": { "play_calling": 12.0, "adaptation": 12.0, "evaluation": 16.0, "negotiating": 12.0 }
    }))
    .unwrap();
    w.start_career(&you, Job::Nfl(t), 7);
    w
}

/// Another club's starters: pros with a team that isn't yours.
fn pros(w: &World) -> Vec<u32> {
    let mine = match readers::seat_staff(w) {
        Some(readers::Staff::Club(t)) => Some(t),
        _ => None,
    };
    w.people
        .iter()
        .filter(|p| p.stage == Stage::Pro && p.pro.as_ref().and_then(|c| c.team).is_some_and(|t| Some(t) != mine))
        .filter(|p| !p.player.position.is_specialist())
        .map(|p| p.player.id)
        .take(30)
        .collect()
}

#[test]
fn a_session_watched_snap_by_snap_becomes_your_report() {
    let mut w = nfl_world();
    let ids = pros(&w);
    assert!(!ids.is_empty());
    let left0 = time::left(&w);
    assert!(left0 > 0.0, "an NFL head coach has film hours");
    ids.into_iter()
        .find(|&id| session::open_player(&mut w, id, tape::Cut::BestOpponent, session::Mode::Full, None).is_ok())
        .expect("a player with tape");
    let s = w.scouting.film.open.as_ref().unwrap();
    let n = s.snaps.len();
    assert!((3..=6).contains(&n), "a cut-up of a few snaps: {n}");
    assert!(s.ask.is_some(), "a question to settle");
    for st in &s.snaps[0].stills {
        assert!(!st.dots.is_empty(), "each still draws the men");
    }
    assert_eq!(s.snaps[0].stills.len(), 3, "three stills a snap");
    // About one snap in three stops for a call.
    let calls = s.snaps.iter().filter(|x| x.call.is_some()).count();
    assert!((1..=n.div_ceil(3)).contains(&calls), "{calls} calls in {n}");
    let mut guard = 0;
    while w.scouting.film.open.as_ref().unwrap().step != 3 {
        guard += 1;
        assert!(guard < 40);
        let step = w.scouting.film.open.as_ref().unwrap().step;
        match step {
            0 => session::play(&mut w).unwrap(),
            1 => session::call(&mut w, 0).unwrap(),
            _ => {
                let _ = session::runback(&mut w);
                session::mark(&mut w, 0).unwrap();
                session::next(&mut w).unwrap();
            }
        }
    }
    let said = session::verdict(&mut w, 0).unwrap();
    assert!(said.contains("Saved to his file"), "{said}");
    assert!(w.scouting.film.open.is_none());
    let r = w.scouting.film.reports.last().unwrap();
    assert_eq!(r.watched as usize, n);
    assert_eq!(r.verdict, Some(0));
    assert!(!r.snaps.is_empty(), "a verdict keeps the stills for the look-back");
    // The film look is in his file under your name, today.
    let you = readers::you(&w).unwrap() as u32 + readers::COACH;
    let fm::Target::Player(pid) = r.target else { panic!() };
    assert!(w.scouting.dept.looks[&pid].iter().any(|l| l.who == you && l.date == w.today));
    // The hours came off the one ledger, four minutes a snap and more.
    let spent = left0 - time::left(&w);
    assert!(spent >= n as f32 * session::SNAP_MIN / 60.0 - 0.01, "spent {spent}");
    assert!((sc::own::film_left(&w) - time::left(&w)).abs() < 1e-4, "lane 7's ledger is the same");
}

#[test]
fn work_hours_set_the_film_and_a_long_day_tires_the_eye() {
    let mut w = nfl_world();
    let steady = time::budget(&w);
    time::set_work(&mut w, "junkie").unwrap();
    let junkie = time::budget(&w);
    assert!((junkie - steady - 30.0).abs() < 0.01, "the extra hours go to film: {steady} -> {junkie}");
    time::set_work(&mut w, "cruising").unwrap();
    assert!(time::budget(&w) < steady);
    time::set_work(&mut w, "junkie").unwrap();
    assert_eq!(time::tiredness(&w), 0.0, "fresh in the morning");
    time::charge(&mut w, 6.0, None);
    assert!(time::tiredness(&w) > 0.3, "six hours in, things slip by");
    // A new day is fresh again.
    w.advance_day();
    assert!(time::tiredness(&w) < 0.2);
}

#[test]
fn watch_it_quick_is_twenty_minutes_on_your_hours() {
    let mut w = nfl_world();
    let ids = pros(&w);
    let before = time::left(&w);
    let id = ids.into_iter().find(|&id| sc::own::watch_film(&mut w, id).is_ok()).expect("a quick watch");
    let after = time::left(&w);
    assert!((before - after - session::QUICK_MIN / 60.0).abs() < 0.01);
    let r = w.scouting.film.reports.last().unwrap();
    assert_eq!(r.target, fm::Target::Player(id));
    assert_eq!(r.mode, session::Mode::Quick);
    assert!(r.calls.1 == 0, "no calls in a quick watch");
}

#[test]
fn a_tight_read_from_tape_is_exact_for_what_the_tape_shows() {
    let mut w = nfl_world();
    let id = pros(&w)[0];
    let i = sc::place(&w, id).unwrap();
    let a = crate::attributes::Attr::Speed;
    let truth = w.people[i].player.attributes.get(a);
    for k in 0..12 {
        w.scouting.film.catches.push(fm::Catch {
            id,
            attr: a.index() as u8,
            v: (truth + if k % 2 == 0 { 1.0 } else { -1.0 }) as f32,
            sd: 1.2,
            date: w.today,
            by: 1,
            key: 0,
        });
    }
    let c = fm::catches_of(&w, id)[a.index()].unwrap();
    assert_eq!(c.1, 0.0);
    assert!((c.0 - truth).abs() < 1e-9, "exact: the real number today");
}

#[test]
fn one_cut_up_at_a_time_and_the_same_snap_counts_once() {
    let mut w = nfl_world();
    let ids = pros(&w);
    let mut open = ids
        .iter()
        .copied()
        .filter(|&id| session::open_player(&mut w, id, tape::Cut::BestOpponent, session::Mode::Full, None).is_ok());
    let first = open.next().expect("a player with tape");
    drop(open);
    let other = ids.iter().copied().find(|&id| id != first).unwrap();
    let e = session::open_player(&mut w, other, tape::Cut::BestOpponent, session::Mode::Quick, None).unwrap_err();
    assert_eq!(e, session::OPEN);
    assert!(matches!(w.scouting.film.open.as_ref().unwrap().target, fm::Target::Player(p) if p == first));
    session::close(&mut w).unwrap();
    // The same cut-up quick, three days running: a snap's read is kept
    // once, however often it is watched.
    for _ in 0..3 {
        session::open_player(&mut w, first, tape::Cut::BestOpponent, session::Mode::Quick, None).unwrap();
        w.advance_day();
    }
    let c = &w.scouting.film.catches;
    assert!(!c.is_empty());
    for (k, a) in c.iter().enumerate() {
        assert!(
            !c[k + 1..].iter().any(|b| a.key != 0
                && a.key == b.key
                && a.id == b.id
                && a.attr == b.attr
                && a.by == b.by),
            "a snap read twice"
        );
    }
}
