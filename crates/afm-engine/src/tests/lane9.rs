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
    assert!((3..=session::CUT_SNAPS).contains(&n), "a cut-up of a few snaps: {n}");
    assert!(s.ask.is_some(), "a question to settle");
    for st in &s.snaps[0].stills {
        assert!(!st.dots.is_empty(), "each still draws the men");
        // On a true-scale field, the key men by name (new bugs, Cain
        // 2026-11-04: the stills were too rough to read).
        let b = st.board.as_ref().expect("a true-scale still");
        assert_eq!(b.men.len(), 22, "everyone on the field");
        assert_eq!(b.men.iter().filter(|m| m.2 == 2).count(), 1, "him once");
        assert!(b.keys.iter().any(|k| k.1 == 1 && !k.2.is_empty()), "him by name: {:?}", b.keys);
        assert!(b.men.iter().all(|m| (b.win.0..=b.win.1).contains(&m.0)), "everyone in the frame: {:?}", b.win);
        assert!((3.0..10.0).contains(&b.hash), "the NFL's hashes: {}", b.hash);
    }
    // The right way up: at the snap the ball is on the line or in the
    // quarterback's hands behind it, the backs are behind the line and
    // nobody on defense is.
    let b = s.snaps[0].stills[0].board.as_ref().unwrap();
    if !s.snaps[0].special {
        assert!((-80..=10).contains(&b.ball.0), "the ball at the snap: {:?}", b.ball);
        let back: Vec<u8> = b.men.iter().filter(|m| m.0 < -5).map(|m| m.2.min(1)).collect();
        assert!(
            !back.is_empty() && back.iter().all(|&k| k == back[0]),
            "only the offense behind the line: {:?}",
            b.men
        );
    }
    assert!(s.snaps[0].stills[1].board.as_ref().unwrap().ring.is_some(), "a ring on him at the moment");
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

// The full tape played straight through (Cain 2026-11-06, the game's
// temporary override): no stops for a call, every snap charged, run-backs
// while the hours last, the room in it, and it stops at the verdict. It
// costs more than the quick watch and buys more: the same snaps studied,
// with the coach's own look and the look-back, against a first look.
#[test]
fn the_full_tape_plays_through_and_buys_more_than_the_quick_watch() {
    let (mut wf, mut wq) = (nfl_world(), nfl_world());
    let ids = pros(&wf);
    let (left_f, left_q) = (time::left(&wf), time::left(&wq));
    let (mut full, mut quick, mut runbacks, mut seen_ids) = (Vec::new(), Vec::new(), 0usize, Vec::new());
    for &id in &ids {
        if full.len() == 4 {
            break;
        }
        if session::open_player(&mut wf, id, tape::Cut::BestOpponent, session::Mode::Full, None).is_err() {
            continue;
        }
        // Out of hours, what was watched is saved at once.
        if session::run_through(&mut wf).unwrap().is_some() {
            break;
        }
        let s = wf.scouting.film.open.as_ref().unwrap();
        assert_eq!(s.step, 3, "played to the verdict");
        assert!(s.picks.iter().all(Option::is_none) && s.snaps.iter().all(|x| x.call.is_none()), "no calls");
        assert!(s.seen.iter().all(|v| !v.contains(&session::HALF)), "nothing left half-seen");
        runbacks += s.seen.iter().flatten().filter(|&&x| x == session::REPLAYED || x == session::REPLAY_MISSED).count();
        session::verdict(&mut wf, 2).unwrap();
        let r = wf.scouting.film.reports.last().unwrap().clone();
        assert_eq!(r.mode, session::Mode::Full);
        assert_eq!(r.calls.1, 0, "no calls asked");
        assert!(!r.snaps.is_empty(), "the verdict keeps the look-back");
        full.push(r);
        seen_ids.push(id);
        session::open_player(&mut wq, id, tape::Cut::BestOpponent, session::Mode::Quick, None).unwrap();
        quick.push(wq.scouting.film.reports.last().unwrap().clone());
    }
    assert!(full.len() >= 2, "full tapes watched: {}", full.len());
    let mins = |v: &[session::Report]| v.iter().map(|r| r.minutes as f32).sum::<f32>() / v.len() as f32;
    let got = |v: &[session::Report]| v.iter().map(|r| r.caught.len() as f32).sum::<f32>() / v.len() as f32;
    let snaps = |v: &[session::Report]| v.iter().map(|r| r.watched as f32).sum::<f32>() / v.len() as f32;
    // Each clear catch a session: your eye's (by 0) and the analyst's points.
    let raw = |w: &World, by: u8| {
        w.scouting.film.catches.iter().filter(|c| c.by == by && seen_ids.contains(&c.id)).count() as f32
            / seen_ids.len() as f32
    };
    eprintln!("eye catches a session: full {:.1} (+{:.1} analyst), quick {:.1}", raw(&wf, 0), raw(&wf, 1), raw(&wq, 0));
    eprintln!(
        "full tape: {:.1} min, {:.1} snaps, {:.1} caught, {runbacks} run-backs, coach in the room {} of {} | quick: {:.1} min, {:.1} snaps, {:.1} caught",
        mins(&full),
        snaps(&full),
        got(&full),
        full.iter().filter(|r| r.coach.is_some()).count(),
        full.len(),
        mins(&quick),
        snaps(&quick),
        got(&quick)
    );
    assert!(raw(&wf, 0) > raw(&wq, 0), "the full tape catches more than the quick watch");
    // The hours: snaps at four minutes and run-backs at two, off the one ledger.
    let spent = left_f - time::left(&wf);
    let charged: f32 = full.iter().map(|r| r.minutes as f32).sum::<f32>() / 60.0;
    assert!((spent - charged).abs() < 0.05, "spent {spent} h, charged {charged} h");
    assert!((left_q - time::left(&wq) - quick.len() as f32 * session::QUICK_MIN / 60.0).abs() < 0.05);
    assert!(quick.iter().all(|r| r.coach.is_none()), "no coach in a quick watch");
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

/// A man's tape is his side of the ball (new bugs, Cain 2026-11-04): a
/// corner the coach also plays on offense is a corner on film, never a
/// ball carrier, and the press-and-man cut-up asks the press question.
#[test]
fn his_tape_is_his_side_of_the_ball() {
    use crate::positions::Position;
    use tape::Role;
    let w = nfl_world();
    let mut checked = 0;
    for id in pros(&w) {
        let Ok(t) = tape::player_tape(&w, id) else { continue };
        let pos = w.people.iter().find(|p| p.player.id == id).unwrap().player.position;
        let off =
            matches!(pos, Position::QB | Position::RB | Position::WR | Position::TE | Position::OT | Position::IOL);
        for s in t.snaps.iter().filter(|s| !s.special) {
            let wrong = if off {
                matches!(s.role, Role::Cover | Role::Zone | Role::Rusher | Role::RunStop)
            } else {
                matches!(
                    s.role,
                    Role::Passer | Role::Target | Role::Route | Role::PassPro | Role::RunBlock | Role::Carrier
                )
            };
            assert!(!wrong, "a {} on the other side: {} ({})", pos.name(), s.job, s.sit);
        }
        checked += 1;
        if checked >= 6 {
            break;
        }
    }
    assert!(checked > 0, "some tape to check");
    assert_eq!(fm::words::cut_ask(Position::CB, tape::Cut::Pressure), Some("cb_press"));
    assert_eq!(fm::words::cut_ask(Position::CB, tape::Cut::ThirdDowns), None);
}

/// Six hours of tape and no read (Cain 2026-11-04): your film counts the
/// day you watch it, every attribute gets a read (a wide guess where the
/// tape can't show it), each cut-up narrows what film shows, and what
/// your eye caught builds into one note a trait.
#[test]
fn film_reads_every_attribute_and_narrows_with_tape() {
    use crate::attributes::{ATTRIBUTE_COUNT, Attr};
    use crate::world::scouting::{file, yours};
    let mut w = nfl_world();
    let staff = readers::seat_staff(&w).unwrap();
    let fresh: Vec<u32> = pros(&w).into_iter().filter(|&id| readers::read(&w, staff, id).is_none()).collect();
    assert!(!fresh.is_empty(), "players your staff hasn't seen");
    let id = fresh
        .into_iter()
        .find(|&id| session::open_player(&mut w, id, tape::Cut::BestOpponent, session::Mode::Full, None).is_ok())
        .expect("a player nobody has seen, with tape");
    let read = |w: &World| {
        let sr = readers::read(w, staff, id);
        yours::own_read(w, staff, id, sr.as_ref().map(|x| &x.2))
    };
    assert!(read(&w).is_none(), "nothing of him has reached you yet");
    // The first snap: a read on every attribute already.
    session::play(&mut w).unwrap();
    let mid = read(&w).expect("a read from the first snap");
    assert!(mid.attrs.iter().all(Option::is_some), "every attribute from the first snap");
    // Through the cut-up: your verdict saves it, and your read and your
    // staff's have every attribute today, not in three weeks.
    let mut guard = 0;
    while w.scouting.film.open.as_ref().unwrap().step != 3 {
        guard += 1;
        assert!(guard < 40);
        match w.scouting.film.open.as_ref().unwrap().step {
            0 => session::play(&mut w).unwrap(),
            1 => session::call(&mut w, 0).unwrap(),
            _ => session::next(&mut w).unwrap(),
        }
    }
    session::verdict(&mut w, 2).unwrap();
    let one = read(&w).expect("your read");
    assert_eq!(one.attrs.iter().filter(|a| a.is_some()).count(), ATTRIBUTE_COUNT);
    let (_, _, f) = readers::read(&w, staff, id).expect("your look in his file today");
    assert!(f.attrs.iter().all(|a| a.range().is_some()), "your staff's file too");
    // What film can't show is a wide guess, never something an eye saw.
    let i = sc::place(&w, id).unwrap();
    let com = Attr::Communication.index();
    assert!(f.attrs[com].guess && !f.attrs[com].seen);
    assert!(one.attrs[com].unwrap().1 >= file::GUESS_SD * file::visibility(&w, i) - 1e-6);
    // The notes: one a trait, each caught no more often than shown.
    let r = w.scouting.film.reports.last().unwrap();
    let lines: Vec<String> = r.notes.iter().map(session::Note::line).collect();
    assert_eq!(r.caught[..r.notes.len()], lines[..], "the report's list is the notes");
    for n in &r.notes {
        assert!(!n.seen.is_empty() && n.seen.len() <= n.of as usize, "{n:?}");
        assert_eq!(r.notes.iter().filter(|m| m.attr == n.attr).count(), 1, "one note a trait");
    }
    // More cut-ups narrow what film shows; the guess stays wide.
    let speed = Attr::Acceleration.index();
    let before = one.attrs[speed].unwrap().1;
    let mut done = 0;
    for cut in tape::Cut::ALL {
        w.advance_day();
        if session::open_player(&mut w, id, cut, session::Mode::Quick, None).is_ok() {
            done += 1;
        }
        let _ = session::close(&mut w);
    }
    assert!(done >= 2, "more cut-ups of him");
    let more = read(&w).unwrap();
    assert!(more.attrs[speed].unwrap().1 < before, "{} -> {}", before, more.attrs[speed].unwrap().1);
    assert!(more.attrs[com].unwrap().1 >= file::GUESS_SD * file::visibility(&w, i) - 1e-6);
    assert!(yours::cut_ups(&w, id).0 > done);
}

#[test]
fn what_your_eye_caught_builds_up() {
    use fm::words::note_head;
    assert!(note_head(&[0], 6).contains("Not a read yet"));
    assert!(note_head(&[3, 3], 6).contains("both times"));
    assert!(note_head(&[0, 0, 0, 0], 4).contains("every snap that showed it"));
    assert!(note_head(&[0, 0, 1], 6).contains("mostly"));
    assert!(note_head(&[4, 1, 3], 6).starts_with("up and down"));
    let n = session::Note { attr: 0, seen: vec![(0, 1), (2, 1), (4, 2)], downs: vec![], of: 5 };
    assert!(n.settled());
    let m = session::Note { attr: 0, seen: vec![(0, 0), (2, 3)], downs: vec![], of: 5 };
    assert!(!m.settled());
    assert!(fm::words::running(crate::attributes::Attr::Press, 1, 3).contains("3rd time"));
}
