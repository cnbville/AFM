//! Live talks lane 5, rooms on today's choices: the parent's call (LT4) and
//! discipline (LT5) take a typed line through the core's generic path, so
//! this proves it for each level and for the council. The pregame and
//! halftime room (LT6) is not a scene and is tested in afm-game
//! (`live_talk::say::rooms` and `tests/live_rooms.rs`).

use super::legacy::seat;
use crate::world::World;
use crate::world::folk::scene::{self, Heard, Kind, Scene};
use crate::world::folk::{Team, finals};

fn typed(code: u32, words: &str) -> Heard {
    Heard { pick: code, tone: "firm".into(), words: words.into(), ..Heard::default() }
}

fn bytes(w: &World) -> Vec<u8> {
    rmp_serde::to_vec(w).unwrap()
}

/// The typed words taken out, to compare a world with the chip's. A ruling
/// that writes no line of yours gets one with the chip's words when you typed
/// it (`scene.rs`, `put_typed`), so your lines are left out of both worlds.
fn untyped(w: &mut World) {
    for s in &mut w.folk.scenes {
        s.lines.retain(|l| l.by != 1);
    }
}

/// A player of the seat's team, or any player when the roster is empty.
fn a_player(w: &World) -> u32 {
    let team = Team::seat(w).unwrap();
    let i = team.roster(w).first().copied().or_else(|| (!w.people.is_empty()).then_some(0));
    w.people[i.expect("a person")].player.id
}

/// Opens a scene of `kind` about `id` the way the world does.
fn opened(w: &mut World, kind: Kind, id: u32, rule: u64, prior: i64) -> u32 {
    let mut s = Scene::new(kind, w.today, "A talk");
    s.a = id as u64;
    s.b = rule;
    s.c = prior;
    s.stops = false;
    s.team = Team::seat(w);
    s.place = "Your office".into();
    let sid = scene::add(w, s);
    scene::open(w, sid).expect("it opens");
    sid
}

fn scene_of(w: &World, id: u32) -> Scene {
    w.folk.scenes.iter().find(|s| s.id == id).unwrap().clone()
}

fn codes(s: &Scene) -> Vec<u32> {
    scene::options(s).iter().map(|p| p.code).collect()
}

/// A world at `level` with a rule broken: the discipline scene, open.
fn discipline(level: &str, council: bool) -> (World, u32) {
    let mut w = seat(level);
    let id = a_player(&w);
    if council {
        w.folk.rules.council = vec![id];
    }
    let sid = opened(&mut w, Kind::Discipline, id, 0, 1);
    (w, sid)
}

#[test]
fn discipline_offers_each_levels_rulings_and_asks_first_above_a_warning() {
    for level in ["hs", "college", "nfl"] {
        for council in [false, true] {
            let (w, sid) = discipline(level, council);
            let s = scene_of(&w, sid);
            assert!(!s.done && s.opened, "{level}: open");
            assert!(scene::takes_typed(&s), "{level}: a discipline talk takes a typed line");
            // The picks differ by level; the council's is there only with one.
            let mut want: Vec<u32> =
                if level == "nfl" { vec![500, 501, 503, 504, 507, 505] } else { vec![500, 502, 506, 503, 504, 505] };
            if council {
                want.push(508);
            }
            let mut got = codes(&s);
            got.sort();
            want.sort();
            assert_eq!(got, want, "{level} council {council}");
            // The warning is free and every ruling above it asks first.
            for p in scene::options(&s) {
                assert_eq!(scene::asks_first(&s, p), p.code != 500, "{level}: pick {}", p.code);
                assert_eq!(finals::is_final(Kind::Discipline, p.code), p.code != 500);
            }
        }
    }
}

#[test]
fn a_typed_ruling_is_the_clicked_one_after_it_asks() {
    for level in ["hs", "college", "nfl"] {
        for council in [false, true] {
            let (w0, s0) = discipline(level, council);
            let all = codes(&scene_of(&w0, s0));
            for code in all {
                let ((mut a, ia), (mut b, ib)) = (discipline(level, council), discipline(level, council));
                assert_eq!(ia, ib);
                let words = "He knows the standard here. Sit him down and tell him why.";
                scene::answer(&mut a, ia, code).unwrap();
                let before = bytes(&b);
                if code != 500 {
                    // A ruling asks first, and asking changes nothing.
                    let err = scene::say(&mut b, ib, typed(code, words), false).unwrap_err();
                    assert_eq!(err, scene::ASK_FIRST, "{level} {code}");
                    assert!(bytes(&b) == before, "{level} {code}: nothing moved before you confirmed");
                }
                let said = scene::say(&mut b, ib, typed(code, words), true).unwrap();
                assert!(said.done, "{level} {code}: the ruling ends the talk");
                // The same world after, the typed words aside, for the same ruling.
                let mine = said.lines.iter().find(|l| l.by == 1).expect("your line");
                assert_eq!(mine.typed.as_deref(), Some(words), "{level} {code}: your words on your line");
                untyped(&mut a);
                untyped(&mut b);
                assert!(bytes(&a) == bytes(&b), "{level} council {council}: pick {code} typed equals clicked");
                // Said once: the meeting is over.
                assert!(scene::say(&mut b, ib, typed(code, words), true).is_err());
            }
        }
    }
}

#[test]
fn a_typed_warning_needs_no_asking() {
    for level in ["hs", "college", "nfl"] {
        let (mut w, sid) = discipline(level, false);
        let s = scene::say(&mut w, sid, typed(500, "Consider this your warning."), false).unwrap();
        assert!(s.done, "{level}");
    }
}

#[test]
fn the_council_decides_the_same_whether_you_type_or_click() {
    // Two worlds, the same rule broken twice, on the council's call (508).
    for (rule, prior) in [(0u64, 0i64), (3, 2)] {
        let (mut a, mut b) = (seat("college"), seat("college"));
        let (ida, idb) = (a_player(&a), a_player(&b));
        a.folk.rules.council = vec![ida];
        b.folk.rules.council = vec![idb];
        let sa = opened(&mut a, Kind::Discipline, ida, rule, prior);
        let sb = opened(&mut b, Kind::Discipline, idb, rule, prior);
        assert!(codes(&scene_of(&a, sa)).contains(&508));
        scene::answer(&mut a, sa, 508).unwrap();
        assert_eq!(
            scene::say(&mut b, sb, typed(508, "Let the leaders handle it."), false).unwrap_err(),
            scene::ASK_FIRST
        );
        scene::say(&mut b, sb, typed(508, "Let the leaders handle it."), true).unwrap();
        untyped(&mut a);
        untyped(&mut b);
        assert!(bytes(&a) == bytes(&b), "rule {rule} prior {prior}");
        assert_eq!(a.folk.rules.rulings.len(), b.folk.rules.rulings.len());
        assert_eq!(a.folk.rules.rulings.len(), 1, "the council's ruling is on the book once");
    }
}

#[test]
fn the_parents_call_takes_a_typed_line_and_asks_nothing_first() {
    for code in [300, 301, 302, 303] {
        let (mut a, mut b) = (seat("college"), seat("college"));
        let (ida, idb) = (a_player(&a), a_player(&b));
        let (sa, sb) = (opened(&mut a, Kind::ParentCall, ida, 0, 0), opened(&mut b, Kind::ParentCall, idb, 0, 0));
        let s = scene_of(&b, sb);
        assert!(scene::takes_typed(&s), "{code}");
        assert_eq!(codes(&s), [300, 301, 302, 303]);
        assert!(scene::options(&s).iter().all(|p| !scene::asks_first(&s, p)), "no parent pick is final");
        scene::answer(&mut a, sa, code).unwrap();
        let said = scene::say(&mut b, sb, typed(code, "I hear you. Here's what I can tell you."), false).unwrap();
        assert!(said.done, "{code}: the call ends");
        untyped(&mut a);
        untyped(&mut b);
        assert!(bytes(&a) == bytes(&b), "{code}: typed equals clicked");
        // A promise of snaps is logged once, whichever way it was said.
        let made = |w: &World| w.folk.promises.len();
        assert_eq!(made(&a), made(&b), "{code}");
        assert_eq!(made(&b), usize::from(code == 302), "{code}: only 302 promises");
    }
}

#[test]
fn a_typed_promise_to_a_parent_is_logged_once() {
    // The reader may label the promise too; the pick's own answer logs it,
    // and the line adds none beyond it today (lane P1 judges the rest).
    use crate::world::folk::scene::PromiseRead;
    let (mut a, mut b) = (seat("college"), seat("college"));
    let (ida, idb) = (a_player(&a), a_player(&b));
    let (sa, sb) = (opened(&mut a, Kind::ParentCall, ida, 0, 0), opened(&mut b, Kind::ParentCall, idb, 0, 0));
    scene::answer(&mut a, sa, 302).unwrap();
    let mut h = typed(302, "He'll start against Iowa. I promise you that.");
    h.promise = Some(PromiseRead { kind: "start".into(), by: "next_game".into(), ..PromiseRead::default() });
    scene::say(&mut b, sb, h, false).unwrap();
    assert_eq!(a.folk.promises.len(), 1);
    assert_eq!(b.folk.promises.len(), 1, "one promise, not two");
}
