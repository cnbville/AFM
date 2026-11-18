//! The negotiation overhaul, lane NG2 (`docs/design/negotiation-arguments.md`,
//! items 5, 1, 3a, 3b and 3c in college): your college player who isn't
//! leaving has his own number; an offer under it upsets him the day you make
//! it; a true stat point from your games moves his bar toward his market,
//! never below it, and turns his counter into a prove-it deal; a cherry-pick
//! gets caught; his makeup decides what the point costs.

use super::legacy::seat;
use crate::game::stats::StatLine;
use crate::player::PlayingTime;
use crate::positions::Position;
use crate::world::folk::claims::Stated;
use crate::world::folk::persona::Them;
use crate::world::folk::persona::deals::{self, Deals};
use crate::world::folk::persona::fake::Fake;
use crate::world::folk::persona::negotiate::points;
use crate::world::folk::persona::terms::{Give, Offer, Valuing, Weighed};
use crate::world::folk::persona::wants::{Want, WantKind};
use crate::world::{Stage, World};

fn college() -> (World, usize) {
    let mut w = seat("college");
    w.add_class_aged(19);
    let p = w.seat.and_then(|s| s.program).unwrap();
    (w, p)
}

/// A college player of yours at a spot, staying (lean 0), paid real money
/// last year, an average man.
fn own(w: &mut World, p: usize, skip: &[usize], pos: Position) -> usize {
    let i = (0..w.people.len())
        .find(|&i| w.people[i].stage == Stage::HighSchool && w.people[i].player.age == 19 && !skip.contains(&i))
        .unwrap();
    let you = w.career.as_ref().map(|c| c.coach).unwrap() as u32;
    let year = w.year;
    let q = &mut w.people[i];
    q.stage = Stage::College;
    q.college = Some(p);
    q.class_of = year - 1;
    q.college_seasons = 1;
    q.last_role = Some(PlayingTime::Starter);
    q.player.position = pos;
    q.trust = 0.5;
    q.trust_in = you;
    q.cal.lean = 0.0;
    q.nil_income = 400_000.0;
    q.brand = 0.0;
    {
        let t = &mut q.player.personality.traits;
        (t.greed, t.ego, t.professionalism, t.temper) = (14, 10, 10, 10);
    }
    w.programs[p].roster.push(i);
    i
}

fn money(x: f64) -> Offer {
    Offer { gives: vec![Give::Money(x)], ..Default::default() }
}

fn fake() -> Fake {
    Fake { wants: vec![Want { kind: WantKind::Money, weight: 1.0, met: false }], ..Default::default() }
}

#[test]
fn a_staying_player_has_his_number_and_a_lowball_is_one() {
    let (mut w, p) = college();
    let i = own(&mut w, p, &[], Position::WR);
    let him = Them::Player(w.people[i].player.id);
    let f = fake();
    let d = Deals { wants: &f };
    let n = deals::own_numbers(&w, him, i, 0.0);
    assert!(n.market >= 400_000.0 - 1.0 && n.lowest <= n.market && n.bar >= n.lowest, "{n:?}");
    // Before, any amount was a yes from a man not leaving.
    assert_eq!(d.weigh(&w, him, &money(n.bar * 1.01)), Weighed::Yes);
    assert_eq!(d.weigh(&w, him, &money(n.lowest * 0.4)), Weighed::Lowball);
    match d.weigh(&w, him, &money(n.lowest * 0.8)) {
        Weighed::Counter(c) => assert_eq!(c.gives, vec![Give::Money(n.bar.round())]),
        v => panic!("expected his bar back, got {v:?}"),
    }
}

#[test]
fn an_offer_under_his_number_upsets_him_the_day_you_make_it() {
    let (mut w, p) = college();
    let i = own(&mut w, p, &[], Position::WR);
    let him = Them::Player(w.people[i].player.id);
    let n = deals::own_numbers(&w, him, i, 0.0);
    let (lean, mood) = (w.people[i].cal.lean, w.people[i].mood);
    // At his number: nothing.
    deals::offered(&mut w, him, &money(n.bar), &Weighed::Yes);
    assert_eq!((w.people[i].cal.lean, w.people[i].mood), (lean, mood));
    // Half his lowest number: his lean rises and his mood falls, today.
    deals::offered(&mut w, him, &money(n.lowest * 0.5), &Weighed::Lowball);
    let care = crate::world::pro::market::goals(&w, i).money;
    let want = deals::LOWBALL_LEAN * 0.5 * care;
    assert!((w.people[i].cal.lean - lean - want).abs() < 1e-6, "lean {} want +{want}", w.people[i].cal.lean);
    assert!(w.people[i].mood < mood);
}

#[test]
fn trust_and_a_winning_home_school_lower_his_number() {
    let (mut w, p) = college();
    let i = own(&mut w, p, &[], Position::WR);
    let cold = deals::college_discount(&w, i);
    assert_eq!(cold.trust, 0.0);
    w.people[i].trust = 1.0;
    w.people[i].home_state = w.programs[p].state;
    let warm = deals::college_discount(&w, i);
    assert!(warm.trust > 0.0 && warm.total > cold.total, "{warm:?} against {cold:?}");
    assert!(warm.total <= crate::world::pro::market::FLOOR_MAX);
}

/// One of your games with these lines (your side).
fn game(w: &mut World, p: usize, lines: Vec<(u32, StatLine)>) {
    let mut stats = crate::game::stats::GameStats::default();
    for (id, l) in lines {
        stats.players[0].insert(id, l);
    }
    let g: crate::world::seat::SeatGame = serde_json::from_value(serde_json::json!({
        "date": serde_json::to_value(w.today).unwrap(),
        "level": serde_json::to_value(crate::calendar::Level::College).unwrap(),
        "teams": [p, p],
        "names": ["Us", "Them"],
        "stats": serde_json::to_value(&stats).unwrap(),
    }))
    .unwrap();
    w.seat_games.push(g);
}

fn hands(targets: u32, drops: u32) -> StatLine {
    StatLine { targets, rec: targets - drops - 2, drops, ..Default::default() }
}

/// Your receiver with six drops in forty targets, and six peers who drop
/// none to three.
fn drops_world() -> (World, usize, Vec<usize>) {
    let (mut w, p) = college();
    let i = own(&mut w, p, &[], Position::WR);
    let mut used = vec![i];
    let mut lines = vec![(w.people[i].player.id, hands(40, 6))];
    for k in 0..6u32 {
        let j = own(&mut w, p, &used, Position::WR);
        used.push(j);
        lines.push((w.people[j].player.id, hands(40, [0, 1, 1, 2, 2, 3][k as usize])));
    }
    game(&mut w, p, lines);
    (w, i, used)
}

#[test]
fn a_true_point_moves_his_bar_toward_his_market_never_below_and_he_counters_prove_it() {
    let (mut w, i, _) = drops_world();
    let id = w.people[i].player.id;
    let him = Them::Player(id);
    let said = Stated { kind: "stat".into(), about: Some(id), value: "6 drops".into() };
    let rd = points::read(&w, i, &said).expect("his season reads");
    assert!(rd.true_ && rd.bad > 0.9 && !rd.cherry, "{rd:?}");
    // A false number about it isn't true.
    let wrong = Stated { value: "15 drops".into(), ..said.clone() };
    assert!(!points::read(&w, i, &wrong).unwrap().true_);
    let before = deals::own_numbers(&w, him, i, 0.0);
    let pulled = points::land(&mut w, i, him, &rd, 0.5);
    assert!((pulled - points::POINT_PULL).abs() < 0.05, "one strong point: {pulled}");
    let after = deals::own_numbers(&w, him, i, 0.0);
    assert!(after.bar < before.bar && after.bar >= after.lowest, "{before:?} then {after:?}");
    // Saying it again adds nothing.
    assert_eq!(points::land(&mut w, i, him, &rd, 0.5), 0.0);
    // Two strong points take him all the way: never below his number.
    let all = deals::own_numbers(&w, him, i, 1.0);
    assert!((all.bar - all.lowest).abs() < 1.0, "{all:?}");
    // Under his bar now, he counters with a prove-it deal: a year, your
    // number as his base, the rest on catches.
    let f = fake();
    let d = Deals { wants: &f };
    let x = (after.lowest * 0.9).round();
    match d.weigh(&w, him, &money(x)) {
        Weighed::Counter(c) => {
            assert!(c.gives.contains(&Give::Years(1)), "{c:?}");
            assert!(c.gives.iter().any(|g| matches!(g, Give::Incentives(_))), "{c:?}");
            assert_eq!(c.terms.len(), 1, "{c:?}");
        }
        v => panic!("expected a prove-it counter, got {v:?}"),
    }
}

#[test]
fn a_cherry_pick_is_caught_or_counts_half() {
    let (mut w, i, _) = drops_world();
    let id = w.people[i].player.id;
    let him = Them::Player(id);
    // A good season by his grade, with the drops in it.
    for g in w.seat_games.iter_mut() {
        if let Some(l) = g.stats.players[0].get_mut(&id) {
            l.graded = 80;
            l.grade_sum = 0.2 * 80.0;
        }
    }
    let rd = points::read(&w, i, &Stated { kind: "stat".into(), about: Some(id), value: "6 drops".into() }).unwrap();
    assert!(rd.cherry, "{rd:?}");
    let full = points::POINT_PULL * rd.bad * rd.on_him;
    let key = crate::world::folk::persona::makeup::key(him);
    let word = w.folk.live.word_of(key);
    let caught = points::caught(&w, i, 0.5);
    let pulled = points::land(&mut w, i, him, &rd, 0.5);
    if caught {
        assert_eq!(pulled, 0.0);
        assert!(w.folk.live.word_of(key) < word);
    } else {
        assert!((pulled - 0.5 * full).abs() < 1e-6);
    }
}

#[test]
fn a_proud_man_takes_a_point_worse_than_a_professional() {
    let (mut w, i, _) = drops_world();
    let rd = points::Read {
        key: "drops",
        real: 6.0,
        true_: true,
        bad: 1.0,
        on_him: 1.0,
        grade: 50.0,
        cherry: false,
        back: None,
    };
    w.people[i].player.personality.traits.ego = 20;
    let (proud, lean) = points::makeup_cost(&w, i, &rd);
    assert!(proud < 0.0 && lean > 0.0);
    w.people[i].player.personality.traits.ego = 8;
    w.people[i].player.personality.traits.professionalism = 20;
    let (pro, lean) = points::makeup_cost(&w, i, &rd);
    assert!(pro > 0.0 && lean == 0.0);
    // Blame his side showed wasn't his costs even the professional.
    let (pro_not_his, _) = points::makeup_cost(&w, i, &points::Read { on_him: 0.4, ..rd });
    assert!(pro_not_his < 0.0);
}

/// Lane NG1's world read (ignored): on a world file (`AFM_MEASURE_WORLD`),
/// how far the tape moves each college player's season read and so his
/// price (`nil_bid::BID_PLAY` a point), by how many players carry tape.
#[test]
#[ignore]
fn season_read_measure() {
    let Ok(path) = std::env::var("AFM_MEASURE_WORLD") else { return };
    let w = crate::save::read_world(std::path::Path::new(&path)).unwrap();
    let mut shifts: Vec<f64> = Vec::new();
    let (mut college, mut with_perf) = (0, 0);
    for p in w.people.iter().filter(|p| p.stage == Stage::College) {
        college += 1;
        let (Some(perf), Some(read)) = (p.last_performance, crate::world::nil_bid::season_read(&w, p)) else {
            continue;
        };
        with_perf += 1;
        if crate::world::nil_bid::tape(&w, p).is_some() {
            shifts.push(read - perf);
        }
    }
    shifts.sort_by(f64::total_cmp);
    let q = |f: f64| shifts.get(((shifts.len() as f64 - 1.0) * f) as usize).copied().unwrap_or(0.0);
    let down = shifts.iter().filter(|&&x| x < -0.5).count();
    let up = shifts.iter().filter(|&&x| x > 0.5).count();
    println!(
        "NG1 {} {}: college {college}, with a season read {with_perf}, with tape {}; shift p5 {:.2} p25 {:.2} p50 {:.2} p75 {:.2} p95 {:.2}; down over half a point {down}, up {up}; price at {}% a point",
        w.year,
        w.today.pretty(),
        shifts.len(),
        q(0.05),
        q(0.25),
        q(0.5),
        q(0.75),
        q(0.95),
        crate::world::nil_bid::BID_PLAY * 100.0
    );
}
