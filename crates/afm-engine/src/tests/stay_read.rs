//! The day's read (playtest bug 15, Cain 2026-11-13): a player deciding on
//! the portal reads his whole situation, not only the lean his fall camp
//! role set. A young starter with your promise holds hard, never locked;
//! a man who doesn't trust you and is unhappy leans out; the read counts
//! once however often it is taken.

use super::legacy::seat;
use crate::player::PlayingTime;
use crate::world::college::{self, COACH_GONE};
use crate::world::folk::talks::{self, PromiseKind, PromiseTo};
use crate::world::stay_read::{self, Ctx, LEVEL, READ_MAX, READ_MIN};
use crate::world::{Stage, World};

/// A player of yours with a fall camp lean: `role`, `seasons` played.
fn yours(role: PlayingTime, seasons: u32) -> (World, usize) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let q = &mut w.people[i];
    q.stage = Stage::College;
    q.college = Some(p);
    q.class_of = year - seasons as i32;
    q.college_seasons = seasons;
    q.last_role = Some(role);
    let why = match role {
        PlayingTime::Starter => "a starter at fall camp",
        PlayingTime::Rotation => "in the rotation at fall camp",
        _ => "a backup at fall camp",
    };
    let base = match role {
        PlayingTime::Starter => 0.07 / 3.0,
        PlayingTime::Rotation => 0.07,
        _ => 0.18,
    };
    q.cal.lean = base;
    q.cal.lean_events = vec![(w.today, why, base)];
    q.mood = 0.0;
    w.programs[p].roster.push(i);
    (w, i)
}

fn product(parts: &[(&'static str, f64)]) -> f64 {
    parts.iter().map(|x| x.1).product()
}

/// His own read, before it is levelled against everyone's.
fn own(parts: &[(&'static str, f64)]) -> f64 {
    parts.iter().filter(|x| x.0 != LEVEL).map(|x| x.1).product()
}

#[test]
fn a_young_starter_with_your_promise_holds_hard_but_is_never_locked() {
    let (mut w, i) = yours(PlayingTime::Starter, 1);
    let ctx = Ctx::new(&w, w.year);
    let before = own(&stay_read::read(&w, &ctx, i));
    let id = w.people[i].player.id;
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Start, 0);
    let ctx = Ctx::new(&w, w.year);
    let parts = stay_read::read(&w, &ctx, i);
    let f = own(&parts);
    assert!(parts.iter().any(|x| x.0 == "a young starter with seasons left"), "{parts:?}");
    assert!(parts.iter().any(|x| x.0 == "a promise from his head coach"), "{parts:?}");
    assert!(f <= before + 1e-9 && f <= 0.5, "promised young starter read {f:.2} (before {before:.2}): {parts:?}");
    assert!(product(&parts) >= READ_MIN - 1e-9, "the read is a hold, not a lock: {f}");
}

#[test]
fn the_read_decides_who_leaves_not_how_many() {
    // Levelled per division (Cain 2026-11-14): weighted by the lean it acts
    // on, the reads at a level average one, the limit aside.
    let (w, _) = yours(PlayingTime::Starter, 1);
    let ctx = Ctx::new(&w, w.year);
    let mut by: std::collections::HashMap<_, (f64, f64)> = std::collections::HashMap::new();
    for i in 0..w.people.len() {
        let p = &w.people[i];
        let Some(c) = p.college else { continue };
        if p.stage != Stage::College || p.cal.lean <= 0.0 || p.cal.portal.is_some() {
            continue;
        }
        let parts = stay_read::read(&w, &ctx, i);
        let f: f64 = parts.iter().filter(|x| x.0 != "the day's read held at its limit").map(|x| x.1).product();
        let prev: f64 = p.cal.lean_events.iter().filter(|e| stay_read::is_read(e.1)).map(|e| e.2).product();
        let lean = p.cal.lean / prev;
        let s = by.entry(w.programs[c].division).or_default();
        s.0 += lean * f;
        s.1 += lean;
    }
    assert!(!by.is_empty());
    for (d, (a, b)) in by {
        if d == crate::world::data::Division::JUCO {
            continue;
        }
        assert!((a / b - 1.0).abs() < 1e-6, "{d:?}: {}", a / b);
    }
}

#[test]
fn an_unhappy_man_who_doesnt_trust_you_leans_out_up_to_the_ceiling() {
    let (mut w, i) = yours(PlayingTime::Backup, 2);
    let p = w.people[i].college.unwrap();
    let hc = w.programs[p].head_coach.unwrap();
    w.people[i].trust_in = hc as u32;
    w.people[i].trust = 0.1;
    w.people[i].mood = -0.9;
    let ctx = Ctx::new(&w, w.year);
    let parts = stay_read::read(&w, &ctx, i);
    let f = product(&parts);
    assert!(parts.iter().any(|x| x.0 == "doesn't trust his head coach"), "{parts:?}");
    assert!(parts.iter().any(|x| x.0 == "unhappy where he is"), "{parts:?}");
    // His own guess of next fall is his own; trust and mood push on top.
    let push: f64 = parts.iter().filter(|x| x.1 > 1.0).map(|x| x.1).product();
    assert!(push >= 1.8, "trust and mood push {push:.2}: {parts:?}");
    assert!(f <= READ_MAX + 1e-9, "read {f:.2}: {parts:?}");
}

#[test]
fn the_read_replaces_itself_and_counts_once() {
    let (mut w, i) = yours(PlayingTime::Rotation, 1);
    let p = w.people[i].college.unwrap();
    w.people[i].trust_in = w.programs[p].head_coach.unwrap() as u32;
    w.people[i].trust = 0.85;
    let base = w.people[i].cal.lean;
    let year = w.year;
    stay_read::read_all(&mut w, year);
    let once = w.people[i].cal.lean;
    stay_read::read_all(&mut w, year);
    stay_read::read_all(&mut w, year);
    assert!((w.people[i].cal.lean - once).abs() < 1e-12, "{} then {}", once, w.people[i].cal.lean);
    // Something changes: the new read replaces the old one.
    w.people[i].trust = 0.15;
    stay_read::read_all(&mut w, year);
    let ctx = Ctx::new(&w, year);
    let now = product(&stay_read::read(&w, &ctx, i));
    assert!((w.people[i].cal.lean - base * now).abs() < 1e-9, "lean {} vs {base} x {now}", w.people[i].cal.lean);
}

#[test]
fn the_coaching_change_push_goes_by_his_tie_to_the_coach_who_left() {
    // No tie (a coach who never coached a game with him) x1.28; a man he
    // recruited and coached x1.8, as before for everyone.
    assert!((college::coach_gone_factor(0.0) - 1.28).abs() < 1e-6);
    assert!((college::coach_gone_factor(0.7) - 1.8).abs() < 1e-6);
    let (mut w, i) = yours(PlayingTime::Backup, 2);
    let p = w.people[i].college.unwrap();
    let hc = w.programs[p].head_coach.unwrap();
    // You coached him no games: tie 0.
    college::coach_gone(&mut w, p, hc);
    let e = w.people[i].cal.lean_events.iter().find(|e| e.1 == COACH_GONE).expect("pushed");
    assert!((e.2 - 1.28).abs() < 1e-6, "{}", e.2);
    // Once a cycle.
    college::coach_gone(&mut w, p, hc);
    assert_eq!(w.people[i].cal.lean_events.iter().filter(|e| e.1 == COACH_GONE).count(), 1);
}

#[test]
fn after_a_head_coach_change_the_old_staffs_role_counts_half() {
    let (mut w, i) = yours(PlayingTime::Backup, 2);
    let today = w.today;
    college::lean_push(&mut w.people[i], today, COACH_GONE, 1.28);
    let ctx = Ctx::new(&w, w.year);
    let parts = stay_read::read(&w, &ctx, i);
    let next = stay_read::next_fall(&w, &ctx, i);
    let half = parts.iter().find(|x| x.0 == "the old staff's role counts half under a new head coach");
    if next == stay_read::Next::Behind {
        assert!(half.is_none(), "a backup who sees himself behind keeps his read: {parts:?}");
    } else {
        let f = half.expect("half his old role").1;
        assert!(f < 1.0, "a backup who sees a place under the new man leans less: {parts:?}");
    }
}

#[test]
fn your_word_on_his_future_is_on_the_table_and_decides_it() {
    use crate::world::clock::PortalWindow;
    use crate::world::folk::claims::Kind as K;
    use crate::world::folk::future::{self, Word};
    let (mut w, i) = yours(PlayingTime::Backup, 2);
    // Four words on the portal, a lost job or sitting behind someone.
    let picks = future::picks(&w, i, K::Portal);
    assert_eq!(picks.len(), 4, "{:?}", picks.iter().map(|p| &p.label).collect::<Vec<_>>());
    assert!(future::picks(&w, i, K::BigGame).is_empty());
    let (line, out) = future::answer(&mut w, i, Word::NoFuture);
    assert!(!line.is_empty() && !out.is_empty());
    let goes = future::decided(&w.people[i]).expect("decided");
    // Decided: nothing more on the table, and no read moves it.
    assert!(future::picks(&w, i, K::Portal).is_empty());
    let lean = w.people[i].cal.lean;
    let year = w.year;
    stay_read::read_all(&mut w, year);
    assert!((w.people[i].cal.lean - lean).abs() < 1e-12);
    let close = w.today.add(15);
    crate::world::recruiting::portal_open(&mut w, PortalWindow::Winter, close);
    let id = w.people[i].player.id;
    if goes {
        // He doesn't ask to see you again: he enters on the window's day.
        assert!(w.people[i].cal.portal.is_some(), "told to go, he stayed out of the portal");
        assert!(!w.portal_intents.iter().any(|t| t.is(id)));
    } else {
        assert!(w.people[i].cal.portal.is_none() && !w.portal_intents.iter().any(|t| t.is(id)));
        assert!(
            (w.people[i].trust - 0.2).abs() < 1e-6
                || w.people[i].trust_in != w.programs[w.people[i].college.unwrap()].head_coach.unwrap() as u32
        );
    }
}

#[test]
fn a_promise_backed_by_a_real_strength_is_believed_more() {
    use crate::world::folk::future::{self, Word};
    let (mut w, i) = yours(PlayingTime::Backup, 2);
    let id = w.people[i].player.id;
    let (_, real) = future::strength(&w, i);
    future::answer(&mut w, i, Word::Backed);
    let k = w.folk.promises.iter().position(|p| p.to == PromiseTo::Player(id) && p.kept.is_none()).expect("logged");
    assert_eq!(w.folk.promises[k].backed, if real { 1 } else { -1 });
    let ctx = Ctx::new(&w, w.year);
    w.folk.promises[k].backed = 1;
    let strong = stay_read::promise_belief(&w, &ctx, i, &w.folk.promises[k]);
    w.folk.promises[k].backed = -1;
    let weak = stay_read::promise_belief(&w, &ctx, i, &w.folk.promises[k]);
    w.folk.promises[k].backed = 0;
    let plain = stay_read::promise_belief(&w, &ctx, i, &w.folk.promises[k]);
    assert!(strong >= plain && plain > weak, "{strong} {plain} {weak}");
}
