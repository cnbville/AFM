//! Ground lane G11b: favors, switched off (`world/ground/favors.rs`).

use crate::calendar::Date;
use crate::player::PlayingTime;
use crate::world::ground::favors::{self, State, Worry};
use crate::world::ground::news::{What, Why};
use crate::world::ground::standing::{self, Level};
use crate::world::ground::{Holder, Who, people};
use crate::world::{Signed, Stage, World};

/// The tiny college seat, the switch as given, a high school with a head
/// coach and one of the world's players made his senior there.
fn world(on: bool) -> (World, usize, usize, usize, u32) {
    let mut w = super::legacy::seat("college");
    w.options.favors = on;
    // No day-one ground anywhere, so the levels below are the test's own.
    for p in &mut w.programs {
        p.hs_pipeline.clear();
        p.area_pipeline.clear();
        p.footprint = [0.0; 51];
        p.pipeline = [0.0; 51];
    }
    for c in &mut w.coaches {
        c.ties.clear();
    }
    let prog = w.seat.and_then(|s| s.program).unwrap();
    let you = w.career.as_ref().unwrap().coach;
    let h = (0..w.high_schools.len()).find(|&h| w.high_schools[h].coach.is_some()).expect("a school with a coach");
    let season = w.dates.season;
    let i = (0..w.people.len()).find(|&i| w.people[i].college != Some(prog)).unwrap();
    let q = &mut w.people[i];
    q.stage = Stage::HighSchool;
    q.college = None;
    q.class_of = season + 1;
    q.hs = h;
    q.cal.offers.clear();
    q.cal.early_offers.clear();
    q.cal.signed = None;
    q.cal.commit = None;
    q.walk_on = false;
    q.bg.gpa = 3.4;
    q.player.injury_proneness = 8.0;
    q.player.personality.work_ethic = 12.0;
    q.player.personality.traits.temper = 5;
    q.incident = false;
    let id = q.player.id;
    (w, prog, you, h, id)
}

/// A program with a real hole for the kid and a head coach to call.
fn taker(w: &World, prog: usize, id: u32) -> (usize, usize) {
    let i = crate::world::folk::person(w, id).unwrap();
    let j = (0..w.programs.len())
        .find(|&j| j != prog && w.programs[j].head_coach.is_some() && favors::hole(w, j, i))
        .expect("a program with a hole");
    (j, w.programs[j].head_coach.unwrap())
}

fn sign(w: &mut World, id: u32, j: usize, d: Date) {
    let i = crate::world::folk::person(w, id).unwrap();
    w.people[i].cal.signed =
        Some(Signed { program: j, walk_on: false, date: d, early_enrollee: false, flipped: false, committed: None });
}

/// He played his two years there as a starter (or a backup).
fn played(w: &mut World, id: u32, j: usize, role: PlayingTime) {
    let i = crate::world::folk::person(w, id).unwrap();
    let p = &mut w.people[i];
    p.cal.signed = None;
    p.stage = Stage::College;
    p.college = Some(j);
    p.last_role = Some(role);
    if role == PlayingTime::Starter {
        p.college_snaps += 900;
    }
}

/// A day that is no Monday, September 1 or January 1 (only the signings
/// and the settle run).
fn plain(d: Date) -> Date {
    let mut d = d;
    while d.weekday() == crate::calendar::Weekday::Mon || matches!(d.parts(), (_, 9, 1) | (_, 1, 1)) {
        d = d.add(1);
    }
    d
}

#[test]
fn off_by_default_nothing_runs_and_placing_works_as_before() {
    let (mut w, _, _, _, id) = world(false);
    assert!(!crate::world::WorldOptions::default().favors, "every world ships with favors off");
    assert!(!favors::on(&w));
    assert!(favors::place(&mut w, id, 0, 1, None, None).is_err());
    assert!(favors::set_rule(&mut w, 2).is_err());
    assert!(!favors::board(&w).on);
    // The built placement keeps its own rules and words.
    let e = crate::world::folk::backchannel::place_player(&mut w, id, 0).unwrap_err();
    assert!(!e.contains("favor"), "{e}");
    let start = w.today;
    for k in 0..60 {
        favors::day(&mut w, start.add(k));
    }
    assert!(w.ground.favors.placed.is_empty() && w.ground.favors.lifts.is_empty());
    assert!(favors::looked(&w).is_empty());
}

#[test]
fn a_call_is_a_look_you_owe_for_and_a_home_pays_by_how_hard_it_was() {
    let (mut w, prog, you, h, id) = world(true);
    assert!(favors::on(&w));
    let (j, man) = taker(&w, prog, id);
    let d = w.today;
    let scarce = favors::scarce(&w, d);
    let said = favors::place(&mut w, id, man, 1, None, None).unwrap();
    assert!(said.contains("His staff will look"), "{said}");
    // His staff looks: a call on the kid's file, as hard as your word is good.
    assert!(w.scouting.hs.promoted.iter().any(|x| x.id == id && x.program == Some(j) && x.warmth == 0.5));
    let yh = Holder::Coach(you as u32);
    let mwho = Who::Coach(man as u32);
    assert_eq!(favors::balance(&w, mwho, yh), if scarce { -2 } else { -1 }, "a look costs a favor");
    assert!(favors::place(&mut w, id, man, 1, None, None).is_err(), "one call per staff");
    // He signs there with no other offer: a scholarship he never had.
    let sender = people::head_coach_of(&w, h).unwrap();
    let before = standing::points(&w, sender, Holder::Program(prog as u32));
    let sd = plain(d.add(3));
    sign(&mut w, id, j, sd);
    favors::day(&mut w, sd);
    let pl = w.ground.favors.placed.last().unwrap().clone();
    assert_eq!(pl.state, State::Signed(sd));
    assert_eq!(pl.owed, 2, "no scholarship at that level or above: two");
    assert_eq!(pl.paid, 1, "one at signing");
    assert_eq!(favors::balance(&w, sender, yh), 1, "his coach owes you one now");
    let after = standing::points(&w, sender, Holder::Program(prog as u32));
    assert!(after > before, "a home found builds standing: {before} -> {after}");
    let m = w.ground.get(sender, yh).unwrap();
    assert!(m.favor_words().unwrap().contains("owes you"));
    // Two years on he starts there: your word held with everyone who heard.
    played(&mut w, id, j, PlayingTime::Starter);
    let due = plain(sd.add(favors::SETTLE_SIGNEE + 1));
    favors::day(&mut w, due);
    let pl = w.ground.favors.placed.last().unwrap().clone();
    assert_eq!(pl.state, State::Settled(true));
    assert_eq!(favors::balance(&w, sender, yh), 2, "the rest comes at the settle");
    let man_now = favors::balance(&w, mwho, yh);
    assert_eq!(man_now, if scarce { 0 } else { 1 }, "the staff that took him owes you two for a starter");
    assert_eq!(w.ground.get(sender, yh).unwrap().your_word, (1, 1));
    assert_eq!(w.ground.get(mwho, yh).unwrap().your_word, (1, 1));
    assert!(favors::word_read(w.ground.get(mwho, yh)) > 0.5, "a held claim makes his staff look harder");
}

#[test]
fn a_hidden_worry_that_comes_true_costs_your_word_and_a_named_one_does_not() {
    for named in [false, true] {
        let (mut w, prog, you, h, id) = world(true);
        let i = crate::world::folk::person(&w, id).unwrap();
        w.people[i].bg.gpa = 2.1;
        let (j, man) = taker(&w, prog, id);
        let worry = named.then_some(Worry::Grades);
        favors::place(&mut w, id, man, 2, worry, None).unwrap();
        let sd = plain(w.today.add(2));
        sign(&mut w, id, j, sd);
        favors::day(&mut w, sd);
        played(&mut w, id, j, PlayingTime::Backup);
        let i = crate::world::folk::person(&w, id).unwrap();
        w.people[i].cal.academic_sit = true;
        let sender = people::head_coach_of(&w, h).unwrap();
        let before = standing::points(&w, sender, Holder::Program(prog as u32));
        favors::day(&mut w, plain(sd.add(favors::SETTLE_SIGNEE + 1)));
        assert_eq!(w.ground.favors.placed.last().unwrap().state, State::Settled(false));
        let after = standing::points(&w, sender, Holder::Program(prog as u32));
        let yh = Holder::Coach(you as u32);
        assert_eq!(w.ground.get(sender, yh).unwrap().your_word, (1, 0), "a sure thing who sat");
        if named {
            assert!((after - before).abs() < 1e-3, "you told him: no extra cost ({before} -> {after})");
        } else {
            assert!(after < before - 5.0, "a hidden worry that came true costs your word ({before} -> {after})");
            let m = w.ground.get(sender, Holder::Program(prog as u32)).unwrap();
            assert!(m.moments.iter().any(|x| x.words.contains("never mentioned")));
        }
    }
}

#[test]
fn a_favor_buys_one_answer_one_level_up_and_never_past_the_gates() {
    let (mut w, prog, you, h, id) = world(true);
    let sender = people::head_coach_of(&w, h).unwrap();
    let d = w.today;
    let ph = Holder::Program(prog as u32);
    let yh = Holder::Coach(you as u32);
    // Known for only a month: no favor buys Trusted inside a year.
    {
        let start = standing::derived(&w, sender, ph);
        let m = standing::entry(&mut w.ground, start, sender, ph, d);
        m.points = 30.0;
        m.contacts = 3;
        m.met = d.add(-40);
        m.known_since = Some(d.add(-30));
    }
    favors::owe(&mut w, sender, yh, 1, "a test".into());
    let e = favors::ask(&mut w, sender, Some(id)).unwrap_err();
    assert!(e.contains("inside a year"), "{e}");
    assert_eq!(favors::balance(&w, sender, yh), 1, "a refused ask spends nothing");
    // Known for over a year: the truth about one kid, one level up.
    if let Some(m) = w.ground.get_mut(sender, ph) {
        m.met = d.add(-500);
        m.known_since = Some(d.add(-400));
    }
    let said = favors::ask(&mut w, sender, Some(id)).unwrap();
    assert!(said.contains("real student") && said.contains("does the work"), "{said}");
    assert_eq!(standing::level(&w, prog, sender), Level::Known, "the favor buys an answer, not standing");
    assert_eq!(favors::lifted(&w, prog, sender, id), Some(Level::Trusted));
    assert_eq!(favors::balance(&w, sender, yh), 0, "the favor is spent");
    assert!(favors::ask(&mut w, sender, Some(id)).is_err(), "one answer at a time");
    // The next news on him reaches you as a Trusted staff would hear it,
    // and hearing it spends the lift (lane G4's order).
    let n = crate::world::ground::news::make(&w, id, What::Grew(20), "He grew.".into(), d, Some(sender), h, None, 1);
    assert_eq!(n.you.why, Why::Favor, "{:?}", n.order);
    assert_eq!(n.you.level, Level::Trusted as u8);
    assert!(n.place > 0 && n.you.day < n.public);
    assert!(crate::world::ground::news::why_words(&w, &n).contains("owed you one"));
    crate::world::ground::news::keep(&mut w, n);
    assert_eq!(favors::lifted(&w, prog, sender, id), None);
    // Asking for more than he owes: he answers, it costs warmth, and you owe him.
    let before = standing::points(&w, sender, ph);
    favors::ask(&mut w, sender, Some(id)).unwrap();
    assert!(standing::points(&w, sender, ph) < before);
    assert_eq!(favors::balance(&w, sender, yh), -1);
}

#[test]
fn a_debt_refused_is_a_broken_promise_and_what_he_owes_fades_if_you_never_ask() {
    let (mut w, prog, you, h, _) = world(true);
    let sender = people::head_coach_of(&w, h).unwrap();
    let Who::Coach(c) = sender else { unreachable!() };
    let ph = Holder::Program(prog as u32);
    let yh = Holder::Coach(you as u32);
    let start = standing::derived(&w, sender, ph);
    standing::entry(&mut w.ground, start, sender, ph, w.today).points = 60.0;
    let before = standing::points(&w, sender, ph);
    favors::refused(&mut w, c as usize);
    let m = w.ground.get(sender, ph).unwrap();
    assert!(m.points < before - 15.0, "{before} -> {}", m.points);
    assert!(m.holding(w.today).is_some(), "a big moment he holds for years");
    // What he owes you fades after two quiet years.
    favors::owe(&mut w, sender, yh, 2, "an old look".into());
    let jan1 = Date::ymd(w.today.parts().0 + 3, 1, 1);
    favors::day(&mut w, jan1);
    assert_eq!(favors::balance(&w, sender, yh), 1);
}

#[test]
fn a_pulled_offer_placed_by_signing_day_costs_a_quarter() {
    let (mut w, prog, _, h, id) = world(true);
    crate::world::front_office::college::add(&mut w, id).unwrap();
    let d = w.today;
    let t = w.front_office.college.board.iter_mut().find(|t| t.id == id).unwrap();
    t.offered = Some(d.add(-60));
    t.pulled = Some(d.add(-10));
    let sender = people::head_coach_of(&w, h).unwrap();
    let ph = Holder::Program(prog as u32);
    let start = standing::derived(&w, sender, ph);
    standing::entry(&mut w.ground, start, sender, ph, d).points = 60.0;
    // The pull itself, as standing pays it.
    let words = Some(("You pulled his offer in March.".to_string(), false, true));
    standing::pay(&mut w, prog, sender, standing::PROMISE_BROKEN, d, None, words);
    assert!(w.ground.get(sender, ph).unwrap().holding(d).is_some());
    let low = standing::points(&w, sender, ph);
    let (j, man) = taker(&w, prog, id);
    favors::place(&mut w, id, man, 0, None, None).unwrap();
    let sd = plain(d.add(2).min(w.dates.signing_day));
    sign(&mut w, id, j, sd);
    favors::day(&mut w, sd);
    let pl = w.ground.favors.placed.last().unwrap();
    assert_eq!(pl.owed, 0, "pulling an offer and placing the kid earns no favor");
    let back = standing::points(&w, sender, ph);
    assert!(back > low + 10.0, "the pull costs a quarter once he has a home: {low} -> {back}");
    assert!(w.ground.get(sender, ph).unwrap().holding(sd).is_none(), "he no longer holds it for years");
}

#[test]
fn the_book_saves_and_an_old_file_loads_with_the_default_rule() {
    let (mut w, prog, _, _, id) = world(true);
    let (_, man) = taker(&w, prog, id);
    favors::place(&mut w, id, man, 1, Some(Worry::Temper), None).unwrap();
    let v = serde_json::to_value(&w.ground).unwrap();
    let back: crate::world::ground::Ground = serde_json::from_value(v).unwrap();
    assert_eq!(back.favors.placed.len(), 1);
    assert_eq!(back.favors.placed[0].worry, Some(Worry::Temper));
    let old: crate::world::ground::Ground = serde_json::from_value(serde_json::json!({})).unwrap();
    assert!(old.favors.placed.is_empty());
    assert_eq!(old.favors.rule.calls, favors::DEFAULT_CALLS);
    let opts: crate::world::WorldOptions = serde_json::from_value(serde_json::json!({
        "seed": 1, "scale": 0.1, "history_years": 0, "start_year": 2026, "names": "Real",
        "engine_from": null, "hs_engine_years": null, "keep_game_log": false
    }))
    .unwrap();
    assert!(!opts.favors, "an old world file loads with favors off");
}

#[test]
fn your_coordinator_places_the_routine_no_and_leaves_a_trusted_schools_kid_to_you() {
    for trusted in [false, true] {
        let (mut w, prog, _, h, id) = world(true);
        crate::world::front_office::college::add(&mut w, id).unwrap();
        let d = w.today;
        let t = w.front_office.college.board.iter_mut().find(|t| t.id == id).unwrap();
        t.offered = Some(d.add(-60));
        t.pulled = Some(d.add(-5));
        let sender = people::head_coach_of(&w, h).unwrap();
        if trusted {
            let ph = Holder::Program(prog as u32);
            let start = standing::derived(&w, sender, ph);
            standing::entry(&mut w.ground, start, sender, ph, d).points = 60.0;
        }
        let mut mon = d.add(1);
        while mon.weekday() != crate::calendar::Weekday::Mon || matches!(mon.parts(), (_, 9, 1) | (_, 1, 1)) {
            mon = mon.add(1);
        }
        let mails = w.career.as_ref().unwrap().desk.mails.len();
        favors::day(&mut w, mon);
        let coord = crate::world::ground::week::coordinator(&w, prog).unwrap();
        let placed: Vec<_> = w.ground.favors.placed.iter().filter(|x| x.kid == id).collect();
        if trusted {
            assert!(placed.is_empty(), "a Trusted school's kid is yours to call");
            let desk = &w.career.as_ref().unwrap().desk;
            assert!(desk.mails.len() > mails && desk.mails.iter().any(|m| m.subject.starts_with("Yours to call")));
        } else {
            assert_eq!(placed.len(), favors::DEFAULT_CALLS as usize, "the rule's calls");
            assert!(placed.iter().all(|x| x.by == coord as u32 && x.claim == 0));
            assert!(favors::board(&w).owe.iter().any(|l| l.words.contains("coordinator")));
        }
    }
}
