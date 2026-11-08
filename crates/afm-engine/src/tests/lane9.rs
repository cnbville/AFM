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
        assert!(guard < 4 * session::CUT_SNAPS + 10);
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
    // The hours came off the one ledger, 36 seconds a snap and more.
    let spent = left0 - time::left(&w);
    assert!(spent >= n as f32 * session::SNAP_MIN / 60.0 - 0.01, "spent {spent}");
    assert!((sc::own::film_left(&w) - time::left(&w)).abs() < 1e-4, "lane 7's ledger is the same");
}

// The full tape played straight through (Cain 2026-11-06, the game's
// temporary override): no stops for a call, every snap charged, run-backs
// while the hours last, the room in it, and saved on the same press with
// no verdict asked (Cain 2026-11-07, minor bug 312). It costs more than
// the quick watch and buys more: the same snaps studied, with the coach's
// own look, against a first look.
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
        let n = wf.scouting.film.open.as_ref().unwrap().snaps.len();
        let said = session::run_through(&mut wf).unwrap();
        assert!(wf.scouting.film.open.is_none(), "saved on the same press, nothing left to ask");
        let r = wf.scouting.film.reports.last().unwrap().clone();
        // Out of hours, what was watched is saved at once.
        if (r.watched as usize) < n {
            break;
        }
        assert!(said.starts_with("Saved to his file") && !said.contains("calls"), "{said}");
        assert_eq!(r.mode, session::Mode::Full);
        assert!(r.verdict.is_some(), "no verdict asked: your read's answer is on the record");
        assert_eq!(r.calls.1, 0, "no calls asked");
        assert!(r.snaps.is_empty(), "no stills kept");
        let extra = r.minutes as f32 - n as f32 * session::SNAP_MIN;
        assert!(extra >= -0.5, "every snap charged: {} min for {n}", r.minutes);
        runbacks += (extra / session::RUNBACK_MIN).round().max(0.0) as usize;
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
    // The hours: snaps at 36 seconds and run-backs at 24, off the one ledger.
    let spent = left_f - time::left(&wf);
    let charged: f32 = full.iter().map(|r| r.minutes as f32).sum::<f32>() / 60.0;
    assert!((spent - charged).abs() < 0.05, "spent {spent} h, charged {charged} h");
    // The quick watch: 20 seconds a snap at game speed, at most 20 minutes.
    let quick_h: f32 = quick.iter().map(|r| r.minutes as f32).sum::<f32>() / 60.0;
    assert!((left_q - time::left(&wq) - quick_h).abs() < 0.05, "quick spent {} h", left_q - time::left(&wq));
    assert!(quick.iter().all(|r| r.minutes as f32 <= session::QUICK_MIN + 0.5));
    assert!(quick.iter().all(|r| r.coach.is_none()), "no coach in a quick watch");
}

// A full tape an older build left open (minor bug 312) is played through
// and saved with no verdict: the state the last build left in saves (every
// call cleared, waiting at the verdict), and a snap-by-snap tape stopped at
// a call after one call was made, which keeps its answer.
#[test]
fn a_full_tape_left_open_by_an_older_build_is_finished_without_asking() {
    let mut w = nfl_world();
    let ids = pros(&w);
    let mut ids = ids.into_iter();
    // Plays snap by snap until the tape stops for a call it hasn't made
    // (Some(at)), or reaches the verdict (None); `make` calls made first.
    let walk = |w: &mut World, mut make: usize| {
        for _ in 0..4 * session::CUT_SNAPS + 10 {
            match w.scouting.film.open.as_ref().unwrap().step {
                0 => session::play(w).unwrap(),
                1 if make > 0 => {
                    let right =
                        w.scouting.film.open.as_ref().map(|s| s.snaps[s.at].call.as_ref().unwrap().right).unwrap();
                    session::call(w, right).unwrap();
                    make -= 1;
                }
                1 => return Some(w.scouting.film.open.as_ref().unwrap().at),
                2 => session::next(w).unwrap(),
                _ => return None,
            }
        }
        panic!("the tape never ended");
    };
    // The last build: no calls, stopped at the verdict.
    let n = loop {
        let id = ids.next().expect("a player with film");
        if session::open_player(&mut w, id, tape::Cut::BestOpponent, session::Mode::Full, None).is_err() {
            continue;
        }
        let s = w.scouting.film.open.as_mut().unwrap();
        s.snaps.iter_mut().for_each(|x| x.call = None);
        let n = s.snaps.len();
        assert_eq!(walk(&mut w, 0), None);
        break n;
    };
    assert_eq!(w.scouting.film.open.as_ref().unwrap().step, 3, "waiting at the verdict");
    let said = session::run_through(&mut w).unwrap();
    assert!(w.scouting.film.open.is_none(), "saved, nothing left open: {said}");
    let r = w.scouting.film.reports.last().unwrap();
    assert_eq!((r.calls, r.watched as usize), ((0, 0), n), "no calls, every snap");
    assert!(r.verdict.is_some() && r.snaps.is_empty(), "your read's answer on the record, no stills");
    // An older build, snap by snap: one call made, stopped at the next.
    let (n, at) = loop {
        let id = ids.next().expect("a tape with two calls");
        assert!(time::left(&w) > 1.5, "the hours held out");
        if session::open_player(&mut w, id, tape::Cut::BestOpponent, session::Mode::Full, None).is_err() {
            continue;
        }
        let n = w.scouting.film.open.as_ref().unwrap().snaps.len();
        match walk(&mut w, 1) {
            Some(at) => break (n, at),
            None => {
                session::close(&mut w).unwrap();
            }
        }
    };
    assert!(at < n);
    session::run_through(&mut w).unwrap();
    assert!(w.scouting.film.open.is_none(), "saved, nothing left open");
    let r = w.scouting.film.reports.last().unwrap();
    assert!(r.verdict.is_some(), "no verdict asked: your read's answer is on the record");
    assert_eq!(r.watched as usize, n, "every snap played");
    assert_eq!(r.calls, (1, 1), "the call made keeps its answer");
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
fn watch_it_quick_is_twenty_seconds_a_snap_on_your_hours() {
    let mut w = nfl_world();
    let ids = pros(&w);
    let before = time::left(&w);
    let id = ids.into_iter().find(|&id| sc::own::watch_film(&mut w, id).is_ok()).expect("a quick watch");
    let after = time::left(&w);
    let r = w.scouting.film.reports.last().unwrap();
    // Minor bug 318: his snaps once at game speed, 20 seconds each, at
    // most 20 minutes (it was 20 minutes for 12 snaps).
    let mins = r.watched as f32 * session::QUICK_SNAP_MIN;
    assert!((before - after - mins / 60.0).abs() < 0.01, "spent {} h for {} snaps", before - after, r.watched);
    assert!(mins <= session::QUICK_MIN + 0.01);
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
            lean: 0.0,
            game: 0,
            form: 0.0,
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
        assert!(guard < 4 * session::CUT_SNAPS + 10);
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

// The film overhaul (Cain 2026-11-07, thread "Make film watching rewards
// logical", minor bugs 317-322): every question's ratings are on the snaps
// that test them, a watch again plays snaps you haven't seen, your eye
// settles near him and not on him, no catch is lost, and the report says
// what the tape gave you.

/// Pros of a position with a team that isn't yours.
fn of_pos(w: &World, pos: crate::positions::Position) -> Vec<u32> {
    let mine = match readers::seat_staff(w) {
        Some(readers::Staff::Club(t)) => Some(t),
        _ => None,
    };
    w.people
        .iter()
        .filter(|p| p.stage == Stage::Pro && p.pro.as_ref().and_then(|c| c.team).is_some_and(|t| Some(t) != mine))
        .filter(|p| p.player.position == pos)
        .map(|p| p.player.id)
        .collect()
}

#[test]
fn every_question_reads_ratings_his_snaps_show() {
    use crate::attributes::{ATTRIBUTE_COUNT, Attr};
    use crate::positions::Position::*;
    let w = nfl_world();
    for pos in [QB, RB, WR, TE, OT, IOL, EDGE, DT, LB, CB, S] {
        let mut shows = [0usize; ATTRIBUTE_COUNT];
        let mut tapes = 0;
        for id in of_pos(&w, pos) {
            if tapes == 3 {
                break;
            }
            let Ok(t) = tape::player_tape(&w, id) else { continue };
            if t.snaps.len() < 40 {
                continue;
            }
            tapes += 1;
            for s in &t.snaps {
                for c in &s.cues {
                    shows[c.0.index()] += 1;
                }
            }
        }
        assert!(tapes > 0, "{pos:?}: a starter with tape");
        for q in fm::words::asks(pos) {
            for a in q.attrs {
                assert!(shows[a.index()] > 0, "{pos:?} {}: {} on no snap of {tapes} tapes", q.key, a.label());
            }
        }
        // The questions Cain asked: composure under pressure and the deep ball.
        if pos == QB {
            for a in [Attr::Composure, Attr::DeepAccuracy, Attr::ArmStrength, Attr::MediumAccuracy] {
                assert!(shows[a.index()] >= 3, "{}: {} snaps", a.label(), shows[a.index()]);
            }
        }
    }
    // Before you watch, each cut-up says how many of its snaps show each
    // of the question's ratings, and one shows the deep ball.
    let qb =
        of_pos(&w, QB).into_iter().find(|&id| tape::player_tape(&w, id).is_ok_and(|t| t.snaps.len() >= 40)).unwrap();
    let pv = session::preview(&w, qb, Some("qb_throws"));
    assert!(!pv.is_empty());
    assert!(pv.iter().all(|p| p.shows.len() == 3 && p.fresh == p.snaps), "{pv:?}");
    assert!(pv.iter().any(|p| p.shows.iter().any(|s| s.0 == Attr::DeepAccuracy.index() as u8 && s.1 > 0)), "{pv:?}");
}

#[test]
fn a_watch_again_plays_snaps_you_havent_seen() {
    let mut w = nfl_world();
    let mine = |w: &World, id: u32| w.scouting.film.watched.iter().filter(|x| x.0 == id).count();
    // A starter with a full cut-up of third downs.
    let id = pros(&w)
        .into_iter()
        .find(|&id| {
            session::open_player(&mut w, id, tape::Cut::ThirdDowns, session::Mode::Quick, None).is_ok()
                && w.scouting.film.reports.last().unwrap().watched >= 20
        })
        .expect("a starter with third downs on tape");
    let mut seen = mine(&w, id);
    for k in 1..=3 {
        let pv = session::preview(&w, id, None);
        let p = pv.iter().find(|p| p.cut == tape::Cut::ThirdDowns).unwrap().clone();
        assert_eq!(p.fresh, p.snaps, "watch {k}: the preview offers only snaps new to you: {p:?}");
        session::open_player(&mut w, id, tape::Cut::ThirdDowns, session::Mode::Quick, None).unwrap();
        let r = w.scouting.film.reports.last().unwrap();
        assert!(r.watched > 0);
        assert_eq!(r.fresh, Some(r.watched), "watch {k}: every snap new to you");
        let now = mine(&w, id);
        assert_eq!(now, seen + r.watched as usize, "watch {k}: kept for the next watch");
        seen = now;
    }
}

#[test]
fn your_eye_settles_near_him_not_on_him() {
    let mut w = nfl_world();
    let id = pros(&w)[0];
    let i = sc::place(&w, id).unwrap();
    let a = crate::attributes::Attr::Speed;
    let truth = w.people[i].player.attributes.get(a);
    // Two thousand catches of your eye, each its own snap, all leaning
    // the same way: the read narrows to the lean and stays there.
    let lean = 1.2;
    for k in 0..2000u64 {
        let noise = if k % 2 == 0 { 2.0 } else { -2.0 };
        w.scouting.film.catches.push(fm::Catch {
            id,
            attr: a.index() as u8,
            v: (truth + lean + noise) as f32,
            sd: 2.0,
            date: w.today,
            by: 0,
            key: (k + 1) << 8 | 1,
            lean: lean as f32,
            game: 0,
            form: 0.0,
        });
    }
    let c = fm::catches_of(&w, id)[a.index()].unwrap();
    assert!(c.1 >= lean - 1e-6, "never tighter than your eye's lean: sd {}", c.1);
    assert!(c.1 < lean + 0.1, "settled at it: sd {}", c.1);
    assert!((c.0 - truth - lean).abs() < 0.05, "near him, where your eye leans");
    // The eye's lean is part of every catch of a session, the same for a
    // man's trait whenever you look, and an elite eye's is smaller.
    const { assert!(fm::EYE_LEAN > 0.0 && fm::EYE_LEAN < 1.0) };
    assert_eq!(fm::eye_lean(&w, id, a), fm::eye_lean(&w, id, a));
}

#[test]
fn a_game_is_one_look_however_many_catches() {
    let mut w = nfl_world();
    let id = pros(&w)[0];
    let a = crate::attributes::Attr::Speed;
    let truth = w.people[sc::place(&w, id).unwrap()].player.attributes.get(a) as f32;
    let push = |w: &mut World, n: u64, games: u64| {
        w.scouting.film.catches.retain(|c| c.id != id);
        for k in 0..n {
            w.scouting.film.catches.push(fm::Catch {
                id,
                attr: a.index() as u8,
                v: truth + if k % 2 == 0 { 2.0 } else { -2.0 },
                sd: 2.0,
                date: w.today,
                by: 0,
                key: (k + 1) << 8 | 1,
                lean: 0.0,
                game: (k % games + 1) << 8 | 3,
                form: 3.0,
            });
        }
        fm::catches_of(w, id)[a.index()].unwrap().1
    };
    // Two hundred catches of one game read him no better than the game:
    // what it showed of him is that game.
    let one = push(&mut w, 200, 1);
    assert!((3.0..3.05).contains(&one), "one game: sd {one}");
    // The same two hundred over ten games read him about three times as
    // well, and a game more always helps.
    let ten = push(&mut w, 200, 10);
    assert!(ten < one / 2.5 && ten > 0.9, "ten games: sd {ten}");
    let eleven = push(&mut w, 220, 11);
    assert!(eleven < ten, "a game more: {ten} -> {eleven}");
    // Every eye's game shows some of him and hides some, an elite eye's less.
    const { assert!(fm::GAME_SD > 0.0) };
    assert!(fm::GAME_SD * sc::file::sharp(20.0) < fm::GAME_SD * sc::file::sharp(10.0));
}

#[test]
fn catches_past_the_cap_fold_and_keep_the_read() {
    let mut w = nfl_world();
    let ids = pros(&w);
    let (old, busy) = (ids[0], ids[1]);
    let a = crate::attributes::Attr::Speed;
    let push = |w: &mut World, id: u32, n: usize, base: u64| {
        let d = w.today;
        for k in 0..n {
            // Your eye's catches, thirty a game, and one in ten your
            // analyst's points.
            let by = u8::from(k % 10 == 0);
            w.scouting.film.catches.push(fm::Catch {
                id,
                attr: a.index() as u8,
                v: 70.0 + (k % 5) as f32 + (k / 30) as f32 * 0.3,
                sd: if by == 1 { 6.0 } else { 3.0 + (k % 3) as f32 },
                date: d,
                by,
                key: (base + k as u64) << 8 | 1,
                lean: if by == 0 { 1.0 } else { 0.0 },
                game: if by == 0 { (base + (k / 30) as u64) << 8 | 3 } else { 0 },
                form: if by == 0 { 2.0 } else { 0.0 },
            });
        }
    };
    push(&mut w, old, 300, 1);
    let before = fm::catches_of(&w, old)[a.index()].unwrap();
    push(&mut w, busy, fm::CATCHES_KEPT, 10_000);
    let busy_before = fm::catches_of(&w, busy)[a.index()].unwrap();
    fm::compact(&mut w.scouting.film.catches, w.today);
    assert!(w.scouting.film.catches.len() <= fm::CATCHES_KEPT, "{}", w.scouting.film.catches.len());
    let after = fm::catches_of(&w, old)[a.index()].unwrap();
    assert!(before.1 > 0.5, "a range, not the exact number: {before:?}");
    assert!((after.0 - before.0).abs() < 0.01 && (after.1 - before.1).abs() < 0.01, "{before:?} -> {after:?}");
    // Each game folds into one catch of that game, which reads as its
    // catches did: the man whose own catches were split by the cut too.
    let busy_after = fm::catches_of(&w, busy)[a.index()].unwrap();
    assert!((busy_after.0 - busy_before.0).abs() < 0.01 && (busy_after.1 - busy_before.1).abs() < 0.01);
    // A game folded, then watched again: its new catches join the same
    // look, so the game's own look isn't counted twice (sd 2 a game here,
    // your eye's lean 1: one game reads no tighter than about 2.24).
    let third = ids[2];
    let eye = |w: &mut World, n: u64, base: u64, game: u64, date: crate::calendar::Date| {
        for k in 0..n {
            w.scouting.film.catches.push(fm::Catch {
                id: third,
                attr: a.index() as u8,
                v: 70.0,
                sd: 3.0,
                date,
                by: 0,
                key: (base + k) << 8 | 1,
                lean: 1.0,
                game,
                form: 2.0,
            });
        }
    };
    let g = 77 << 8 | 3;
    let today = w.today;
    eye(&mut w, 30, 5_000_000, g, today);
    let one = |w: &World| fm::catches_of(w, third)[a.index()].unwrap().1;
    let was = one(&w);
    let len = w.scouting.film.catches.len();
    push(&mut w, busy, len, 100_000);
    fm::compact(&mut w.scouting.film.catches, w.today);
    let mine: Vec<&fm::Catch> = w.scouting.film.catches.iter().filter(|c| c.id == third).collect();
    assert!(mine.len() == 1 && mine[0].game == g, "one catch of the game, its key kept: {}", mine.len());
    assert!((one(&w) - was).abs() < 0.01, "{was} -> {}", one(&w));
    eye(&mut w, 200, 6_000_000, g, today);
    assert!(one(&w) > 2.2 && one(&w) < was, "two hundred more of a game already folded: {was} -> {}", one(&w));
    // Every catch its own game and over a year old: they fold into one
    // catch a man, trait and eye, so the store stays bounded.
    let mut w = nfl_world();
    let year = crate::calendar::Date(w.today.0 - 400);
    let n = fm::CATCHES_KEPT + 2;
    for k in 0..n as u64 {
        w.scouting.film.catches.push(fm::Catch {
            id: busy,
            attr: a.index() as u8,
            v: 70.0,
            sd: 3.0,
            date: year,
            by: 0,
            key: (2_000_000 + k) << 8 | 1,
            lean: 1.0,
            game: (3_000_000 + k) << 8 | 3,
            form: 2.0,
        });
    }
    fm::compact(&mut w.scouting.film.catches, w.today);
    let m = w.scouting.film.catches.len();
    assert!(m <= n / 2 + 1, "{n} -> {m}");
}

#[test]
fn the_report_says_what_the_tape_gave_you() {
    use std::collections::HashSet;
    let mut w = nfl_world();
    let staff = readers::seat_staff(&w).unwrap();
    // A man nobody has seen: the first watch reads his ratings for the
    // first time, only those its snaps showed your eye.
    let fresh: Vec<u32> = pros(&w).into_iter().filter(|&id| readers::read(&w, staff, id).is_none()).collect();
    let mut said = String::new();
    let id = fresh
        .into_iter()
        .find(|&id| match session::open_player(&mut w, id, tape::Cut::ThirdDowns, session::Mode::Quick, None) {
            Ok(s) => {
                said = s;
                w.scouting.film.reports.last().unwrap().watched >= 20
            }
            Err(_) => false,
        })
        .expect("a man nobody has seen, with third downs on tape");
    let r = w.scouting.film.reports.last().unwrap().clone();
    let caught: HashSet<u8> = w.scouting.film.catches.iter().filter(|c| c.id == id).map(|c| c.attr).collect();
    assert!(!r.moved.is_empty(), "{said}");
    assert!(r.moved.iter().all(|m| m.1.is_none() && caught.contains(&m.0)), "{:?}", r.moved);
    assert!(said.contains(&format!("{} of his ratings read for the first time", r.moved.len())), "{said}");
    // The question's ratings first.
    let qa = r.ask.as_ref().unwrap().attrs.clone();
    let firsts: Vec<bool> = r.moved.iter().map(|m| qa.contains(&m.0)).collect();
    assert!(firsts.windows(2).all(|x| x[0] || !x[1]), "the question's ratings first: {:?}", r.moved);
    // A watch again: each rating it narrowed, its range before and after.
    w.advance_day();
    let said = session::open_player(&mut w, id, tape::Cut::ThirdDowns, session::Mode::Quick, None).unwrap();
    let r = w.scouting.film.reports.last().unwrap();
    let narrowed: Vec<_> = r.moved.iter().filter(|m| m.1.is_some()).collect();
    assert!(!narrowed.is_empty(), "{said}");
    for m in &narrowed {
        let b = m.1.unwrap();
        assert!(m.2[1] - m.2[0] < b[1] - b[0], "{m:?}");
    }
    assert!(said.contains("of your ratings of him narrowed"), "{said}");
}

// A tape played straight through asks no verdict (minor bug 312), but
// your read's answer to the question goes on the record, and three years
// on the look-back checks a yes or a no against what he became.
#[test]
fn a_tape_played_through_puts_your_read_on_the_record() {
    let mut w = nfl_world();
    let id = pros(&w)
        .into_iter()
        .find(|&id| session::open_player(&mut w, id, tape::Cut::BestOpponent, session::Mode::Full, None).is_ok())
        .expect("a player with tape");
    let said = session::run_through(&mut w).unwrap();
    let r = w.scouting.film.reports.last().unwrap().clone();
    assert_eq!(r.target, fm::Target::Player(id));
    let v = r.verdict.expect("your read's answer on the record");
    assert!(said.contains("On the record from your read"), "{said}");
    assert!(r.snaps.is_empty(), "no stills");
    let d = w.today.add(1100);
    fm::day(&mut w, d);
    let r = w.scouting.film.reports.iter().rev().find(|x| x.target == r.target && x.date == r.date).unwrap();
    assert_eq!(r.settled.is_some(), v < 2, "a yes or a no is checked three years on, not sure is not");
}
