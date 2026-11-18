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
    crate::world::folk::promise::hear(&mut w, k);
    let strong = stay_read::promise_belief(&w, &ctx, i, &w.folk.promises[k]);
    w.folk.promises[k].backed = -1;
    crate::world::folk::promise::hear(&mut w, k);
    let weak = stay_read::promise_belief(&w, &ctx, i, &w.folk.promises[k]);
    w.folk.promises[k].backed = 0;
    crate::world::folk::promise::hear(&mut w, k);
    let plain = stay_read::promise_belief(&w, &ctx, i, &w.folk.promises[k]);
    assert!(strong >= plain && plain > weak, "{strong} {plain} {weak}");
}

/// Your word on his future in a typed talk (live talks v3): the picks'
/// gates with the point read from his sheet, the picks' outcome and odds.
#[test]
fn a_typed_word_on_his_future_settles_it_like_the_pick() {
    use crate::world::folk::future::{self, Plan, Word};
    let (mut w, i) = yours(PlayingTime::Backup, 2);
    // No portal, lost job or buried point on his sheet: nothing to settle.
    assert!(!future::open(&w, i));
    assert_eq!(future::plan(&w, i, Word::NoFuture, ""), Plan::Nothing(""));
    // Leaning toward the portal: the words are open.
    w.people[i].cal.lean = 0.5;
    assert!(future::open(&w, i));
    let plan = future::plan(&w, i, Word::LastChoice, "");
    let stays = future::stays(&w, i, Word::LastChoice);
    assert_eq!(plan, Plan::Settle { word: Word::LastChoice, stays, takes_back: false });
    assert!(plan.ends());
    future::run(&mut w, i, &plan);
    assert_eq!(future::decided(&w.people[i]), Some(!stays), "the plan's draw is the one made real");
    // Decided: a second word does nothing.
    assert!(matches!(future::plan(&w, i, Word::NoFuture, ""), Plan::Nothing(x) if !x.is_empty()));
}

#[test]
fn a_typed_word_to_go_takes_back_your_word_on_his_role() {
    use crate::world::folk::future::{self, Plan, Word};
    let (mut w, i) = yours(PlayingTime::Backup, 2);
    w.people[i].cal.lean = 0.5;
    let id = w.people[i].player.id;
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Snaps, 0);
    let plan = future::plan(&w, i, Word::NoFuture, "");
    assert!(matches!(plan, Plan::Settle { takes_back: true, .. }), "{plan:?}");
    let out = future::run(&mut w, i, &plan);
    assert!(!w.folk.promises.iter().any(|p| p.to == PromiseTo::Player(id) && p.kept.is_none()), "{out:?}");
    assert!(future::decided(&w.people[i]).is_some());
    // "I'll help you find a place" on a promise that slipped is section 6's help.
    let (mut w, i) = yours(PlayingTime::Backup, 2);
    w.people[i].cal.lean = 0.5;
    let id = w.people[i].player.id;
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Start, 0);
    let k = w.folk.promises.len() - 1;
    w.folk.promises[k].state = 2;
    assert_eq!(future::plan(&w, i, Word::Elsewhere, ""), Plan::Help);
    let before = w.people[i].cal.lean;
    future::run(&mut w, i, &Plan::Help);
    assert!(w.folk.promises.iter().all(|p| p.to != PromiseTo::Player(id) || p.kept.is_some()));
    assert!(w.people[i].cal.lean > before, "he leans out");
    assert!(future::decided(&w.people[i]).is_none(), "help isn't a decision for him");
}

#[test]
fn a_typed_strength_backs_the_promise_the_line_made() {
    use crate::world::folk::future::{self, Plan, Word};
    let (mut w, i) = yours(PlayingTime::Backup, 2);
    w.people[i].cal.lean = 0.5;
    let id = w.people[i].player.id;
    // A strength he has no number for at his spot backs nothing.
    assert_eq!(future::plan(&w, i, Word::Backed, "nothing"), Plan::Nothing(""));
    if w.people[i].player.position != crate::positions::Position::QB {
        assert!(future::named(&w, i, "arm").is_none(), "only a quarterback has an arm number");
    }
    let (word, real) = future::named(&w, i, "speed").expect("speed is his at any spot");
    assert_eq!(future::plan(&w, i, Word::Backed, "speed"), Plan::Back { strong: word, real });
    // The line's own promise, logged first, is the one backed.
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Start, 0);
    let n = w.folk.promises.len();
    future::run(&mut w, i, &Plan::Back { strong: word, real });
    assert_eq!(w.folk.promises.len(), n, "no second promise");
    assert_eq!(w.folk.promises[n - 1].backed, if real { 1 } else { -1 });
    // No promise said: it logs the pick's own.
    let (mut w, i) = yours(PlayingTime::Backup, 2);
    w.people[i].cal.lean = 0.5;
    let id = w.people[i].player.id;
    future::run(&mut w, i, &Plan::Back { strong: word, real });
    let p = w.folk.promises.iter().find(|p| p.to == PromiseTo::Player(id) && p.kept.is_none()).expect("logged");
    assert_eq!(p.backed, if real { 1 } else { -1 });
}

/// A typed line that changes a promise lands on the moves list's answers.
#[test]
fn a_typed_change_to_a_promise_reworks_it() {
    use crate::world::folk::persona::ear::{Amend, AmendHow};
    use crate::world::folk::promise::By;
    let (mut w, i) = yours(PlayingTime::Backup, 2);
    w.people[i].trust = 0.5;
    let id = w.people[i].player.id;
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Start, 0);
    let k = w.folk.promises.len() - 1;
    let a = |how, by: &str, condition: &str| Amend {
        promise: None,
        how,
        by: by.into(),
        condition: condition.into(),
        bar: None,
    };
    let (pa, _) = talks::amend_act(&w, i, k, &a(AmendHow::Later, "next_season", "")).unwrap();
    assert_eq!(pa, talks::PAct::NextSeason);
    let r = talks::rework(&mut w, i, k, pa, None);
    assert!(!r.gone);
    assert_eq!(w.folk.promises[k].terms.by, By::NextSeason);
    assert!(talks::amend_act(&w, i, k, &a(AmendHow::Condition, "", "")).is_none(), "no condition named");
    let (pa, _) = talks::amend_act(&w, i, k, &a(AmendHow::Condition, "", "grades")).unwrap();
    assert_eq!(pa, talks::PAct::Grades);
    // Slipped, a later date is section 6's new date.
    w.folk.promises[k].state = 2;
    assert_eq!(talks::amend_act(&w, i, k, &a(AmendHow::Later, "midseason", "")).unwrap().0, talks::PAct::NewDate);
    let trust = w.people[i].trust;
    let (pa, _) = talks::amend_act(&w, i, k, &a(AmendHow::TakeBack, "", "")).unwrap();
    assert!(talks::rework(&mut w, i, k, pa, None).gone);
    assert!(w.people[i].trust < trust, "taking it back costs a little trust");
}

/// 2026-11-18: his money in the stay read is what schools and collectives
/// pay, never his own brand deals, and at your program next year's deal on
/// your desk, so a raise in the spring holds him before July 1 pays it.
#[test]
fn his_money_is_next_years_deal_and_never_his_own_brand() {
    let (mut w, i) = yours(PlayingTime::Starter, 1);
    assert!(crate::world::ncaa::nil_era(w.year), "{}", w.year);
    // Nine more starters at his spot and school, paid $100K each: the
    // going rate.
    let c = w.people[i].college.unwrap();
    let pos = w.people[i].player.position;
    let more: Vec<usize> =
        (0..w.people.len()).filter(|&k| k != i && w.people[k].stage == Stage::HighSchool).take(9).collect();
    for &k in &more {
        let q = &mut w.people[k];
        q.stage = Stage::College;
        q.college = Some(c);
        q.player.position = pos;
        q.last_role = Some(PlayingTime::Starter);
        q.college_seasons = 1;
        q.cal.nil_paid = 100_000.0;
        w.programs[c].roster.push(k);
    }
    let p = &mut w.people[i];
    p.cal.nil_paid = 0.0;
    p.cal.nil_school = 0.0;
    p.nil_income = 2.0e6;
    p.brand = 2.0e6;
    p.family.own_wants.money = 0.3;
    let underpaid = |parts: &[(&'static str, f64)]| parts.iter().any(|x| x.0 == "underpaid where he is");
    let ctx = Ctx::new(&w, w.year);
    let parts = stay_read::read(&w, &ctx, i);
    assert!(underpaid(&parts), "his own brand money isn't pay: {parts:?}");
    // A raise on your desk for next year holds him now.
    crate::world::front_office::college::fill_deals(&mut w);
    let id = w.people[i].player.id;
    crate::world::front_office::college::set_deal(&mut w, id, Some(150_000.0), None).unwrap();
    let ctx = Ctx::new(&w, w.year);
    let parts = stay_read::read(&w, &ctx, i);
    assert!(!underpaid(&parts), "next year's deal: {parts:?}");
}
