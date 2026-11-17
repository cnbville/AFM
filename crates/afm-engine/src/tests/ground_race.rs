//! Ground lane G9: the race (`world/ground/ai.rs`, `race.rs`): every other
//! staff works its ground by the seat's rules, its board reads what its
//! people know, and your seat sees the race in signs.

use crate::calendar::{Date, Weekday};
use crate::world::ground::ai;
use crate::world::ground::people::{self, Holder, Who};
use crate::world::ground::race::{self, Topic};
use crate::world::ground::standing::{self, Level};
use crate::world::{Stage, World};

impl ai::Ai {
    fn programs_worked(&self) -> Vec<u32> {
        let mut v: Vec<u32> = self
            .ties
            .0
            .keys()
            .filter_map(|&k| match ai::key_parts(k).1 {
                Holder::Program(p) => Some(p),
                Holder::Coach(_) => None,
            })
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }
}

fn whole(s: &str) -> bool {
    !s.is_empty() && !s.contains(['{', '}']) && !s.contains("NaN")
}

/// Kids at the schools of a state: pros made into high school juniors and
/// seniors, so the areas have talent.
fn kids(w: &mut World, st: usize, n: usize) -> Vec<usize> {
    let schools: Vec<usize> = (0..w.high_schools.len())
        .filter(|&h| w.high_schools[h].state == st && w.high_schools[h].coach.is_some())
        .collect();
    assert!(!schools.is_empty(), "coached schools in the state");
    let season = w.dates.season;
    let mut out = Vec::new();
    for (k, (i, p)) in w.people.iter_mut().enumerate().filter(|(_, p)| p.stage == Stage::Pro).take(n).enumerate() {
        p.stage = Stage::HighSchool;
        p.pro = None;
        p.class_of = season + 1 + (k % 2) as i32;
        p.player.age = 17;
        p.hs = schools[k % schools.len()];
        p.home_state = st;
        p.stars = [2, 3, 3, 4][k % 4];
        out.push(i);
    }
    out
}

fn seat() -> (World, usize) {
    let mut w = super::legacy::seat("college");
    let prog = w.seat.and_then(|s| s.program).unwrap();
    let st = w.programs[prog].state;
    kids(&mut w, st, 240);
    (w, prog)
}

/// The AI's evenings through a spring window.
fn spring(w: &mut World) -> (Date, Date) {
    let y = w.dates.season;
    let (a, b) = (Date::ymd(y, 4, 15), Date::ymd(y, 5, 31));
    let mut d = a;
    ai::plan(w);
    while d <= b {
        w.today = d;
        ai::day(w, d);
        d = d.add(1);
    }
    (a, b)
}

#[test]
fn every_other_staff_works_its_ground_by_the_seats_rules() {
    let (mut w, prog) = seat();
    let (a, b) = spring(&mut w);
    assert!(!w.ground.ai.beats.is_empty(), "areas planned");
    for bt in &w.ground.ai.beats {
        assert_ne!(bt.prog as usize, prog, "your staff works by your plan");
        assert!(!bt.areas.is_empty() && bt.areas.len() <= 4, "{:?}", bt.areas);
    }
    let worked = w.ground.ai.programs_worked();
    assert!(!worked.is_empty(), "the road built ties");
    assert!(!worked.contains(&(prog as u32)), "never your program's");
    for (&k, t) in &w.ground.ai.ties.0 {
        let (c, _) = ai::key_parts(k);
        assert!(t.points >= 0.0 && t.points <= 100.0);
        assert!(t.points <= ai::cap(t, b).max(standing::derived(&w, Who::Coach(c), ai::key_parts(k).1)) + 0.01);
        assert!(t.last >= a.add(-1) && t.last <= b, "contacts on the window's days");
    }
    // The window's day limits hold (FBS and FCS).
    let season = w.dates.season;
    let (_, spring_cap) = crate::world::scouting::dept::eval_limits(season, false);
    for (j, u) in w.ground.ai.used.iter().enumerate() {
        if matches!(w.programs[j].division, crate::world::data::Division::FBS | crate::world::data::Division::FCS)
            && !crate::world::scouting::dept::service_academy(&w.programs[j])
        {
            assert!(u.1 <= spring_cap, "{} used {} spring days", w.programs[j].real_name, u.1);
        }
    }
    // Fading: four months on with no contact, the worked ties come down.
    let before: f32 = w.ground.ai.ties.0.values().map(|t| t.points).sum();
    for m in [7, 8, 9] {
        let d = Date::ymd(season, m, 1);
        w.today = d;
        ai::day(&mut w, d);
    }
    let after: f32 = w.ground.ai.ties.0.values().map(|t| t.points).sum();
    assert!(after < before, "standing fades without contact: {before} to {after}");
}

#[test]
fn a_staff_hears_of_a_kid_the_way_its_people_tell_it() {
    let (mut w, prog) = seat();
    spring(&mut w);
    let ix = ai::index(&w);
    // Some AI staff knows some school's coach from its own work.
    let some = ix.by_prog.iter().enumerate().find(|(j, m)| *j != prog && !m.is_empty());
    let (j, m) = some.expect("an AI staff with worked ground");
    let (&h, &(pts, last)) = m.iter().next().unwrap();
    assert!(ai::school_points(&w, &ix, j, h as usize) >= pts);
    assert!(ai::knows(&w, &ix, j, h as usize) >= 0.0);
    // A visit gets the public names even from a Stranger; a Known coach's
    // list needs a contact this cycle.
    let d = w.today;
    assert!(ai::told(pts.max(25.0), Some(last), false, d));
    assert!(!ai::told(0.0, None, true, d));
    // The states a staff works hold its own.
    let ws = ai::work_states(&w, j);
    assert!(ws[w.programs[j].state.min(50)]);
}

#[test]
fn you_see_the_race_in_signs() {
    let (mut w, prog) = seat();
    let d = Date::ymd(w.dates.season, 5, 5);
    w.today = d;
    // A school in your state with a coach, and a rival's man visiting it.
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let h = w.people[i].hs;
    let Some(Who::Coach(c)) = people::head_coach_of(&w, h) else { panic!("a coached school") };
    let rival = (0..w.programs.len())
        .find(|&j| {
            j != prog && w.programs[j].plays(w.year) && (w.programs[j].oc.is_some() || w.programs[j].dc.is_some())
        })
        .unwrap();
    let man = w.programs[rival].oc.or(w.programs[rival].dc).expect("a rival coordinator");
    ai::contact(&mut w, rival, man, c, false, d.add(-3));
    ai::contact(&mut w, rival, man, c, false, d);
    let sheet = race::sheet(&w, h, d.add(-race::SHEET_DAYS), d);
    assert!(sheet.iter().any(|v| v.man == man as u32 && v.program == rival as u32), "{sheet:?}");
    let line = race::at_school(&w, h, d).expect("signs at the school");
    assert!(line.contains("polo") && line.contains(&w.coaches[man].name), "{line}");
    assert!(whole(&line));
    // What his coach tells you grows with your standing.
    let id = w.people[i].player.id;
    let set = |w: &mut World, pts: f32| {
        for holder in standing::holders(w, prog) {
            let m = standing::entry(&mut w.ground, 0.0, Who::Coach(c), holder, d);
            m.points = pts;
        }
    };
    set(&mut w, 10.0);
    assert!(race::told(&w, id).is_empty(), "a Stranger tells you nothing");
    set(&mut w, 30.0);
    let known = race::told(&w, id);
    assert!(known.iter().any(|l| l.contains(&w.programs[rival].real_name)), "{known:?}");
    set(&mut w, 60.0);
    let trusted = race::told(&w, id);
    assert!(trusted.iter().any(|l| l.contains("twice")), "{trusted:?}");
    set(&mut w, 90.0);
    let home = race::told(&w, id);
    assert!(home.iter().any(|l| l.contains("about to offer")), "{home:?}");
    assert!(home.iter().chain(&trusted).chain(&known).all(|l| whole(l)));
    // The rival shows on your rivals page once his program works your areas.
    if let Some(r) = race::rival(&w, man as u32) {
        assert_eq!(r.program, rival as u32);
        assert!(whole(&r.style));
    }
}

#[test]
fn a_wavering_commit_comes_with_a_warning_and_one_call() {
    let (mut w, prog) = seat();
    let monday = Date::ymd(w.dates.season, 6, 1).on_or_after(Weekday::Mon);
    w.today = monday;
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let rival = (0..w.programs.len()).find(|&j| j != prog && w.programs[j].plays(w.year)).unwrap();
    {
        let q = &mut w.people[i];
        q.cal.commit = Some((prog, monday.add(-60), false));
        q.cal.visits.push((rival, monday.add(-2), crate::world::Visit::Official));
    }
    let id = w.people[i].player.id;
    race::watch(&mut w, monday);
    let wv = w.ground.race.wavers.iter().find(|x| x.id == id).expect("a waver");
    assert_eq!(wv.rival as usize, rival);
    assert!(wv.signs.iter().any(|s| s.contains("official visit")), "{:?}", wv.signs);
    assert!(w.career.as_ref().unwrap().desk.mails.iter().any(|m| m.subject.contains("wavering")));
    // Once a month only.
    race::watch(&mut w, monday.add(7));
    assert_eq!(w.ground.race.wavers.iter().filter(|x| x.id == id).count(), 1);
    // Your call: it lands by your standing with his family, and holds him.
    let line = race::fight(&mut w, id).expect("the call");
    assert!(whole(&line), "{line}");
    assert!(race::fight(&mut w, id).is_err(), "one call");
    let hold = race::fight_hold(&w, id, prog, monday.add(1));
    assert!(hold > 0.0 && hold <= race::FIGHT_HOLD);
    assert_eq!(race::fight_hold(&w, id, prog, monday.add(race::FIGHT_DAYS + 2)), 0.0, "it wears off");
    assert_eq!(race::fight_hold(&w, id, rival, monday.add(1)), 0.0, "only your own commit");
}

#[test]
fn claims_settle_on_the_facts_and_a_false_one_costs_its_maker() {
    let (mut w, prog) = seat();
    let d = Date::ymd(w.dates.season, 3, 2);
    w.today = d;
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let id = w.people[i].player.id;
    let rival = (0..w.programs.len()).find(|&j| j != prog && w.programs[j].plays(w.year)).unwrap();
    let fam = people::family_of(&w, i);
    for holder in standing::holders(&w, prog) {
        standing::entry(&mut w.ground, 0.0, fam, holder, d).points = 40.0;
    }
    let said = race::claim(&mut w, id, rival as u32, Topic::Scheme).expect("a claim");
    assert!(whole(&said), "{said}");
    assert!(race::claim(&mut w, id, rival as u32, Topic::Scheme).is_err(), "said once");
    assert!(race::claim(&mut w, id, prog as u32, Topic::Job).is_err(), "not about yourself");
    let x = w.ground.race.claims[0].clone();
    assert!(x.held.is_none() && x.settles > d);
    let before = standing::points(&w, fam, Holder::Program(prog as u32));
    w.today = x.settles;
    race::settle(&mut w, x.settles);
    let held = w.ground.race.claims[0].held.expect("settled");
    let after = standing::points(&w, fam, Holder::Program(prog as u32));
    if held {
        assert!(after >= before, "a true claim costs nothing");
    } else {
        assert!(after < before, "a false claim costs you with the family: {before} to {after}");
        let m = w.ground.get(fam, Holder::Program(prog as u32)).unwrap();
        assert!(m.moments.iter().any(|m| m.words.contains("lied")));
    }
    assert!(Level::of(after) <= Level::Home);
}

#[test]
fn the_ai_ground_saves_and_loads() {
    let (mut w, _) = seat();
    spring(&mut w);
    let n = w.ground.ai.len();
    assert!(n > 0);
    let b = rmp_serde::to_vec_named(&w.ground).expect("save");
    let back: crate::world::ground::Ground = rmp_serde::from_slice(&b).expect("load");
    assert_eq!(back.ai.ties, w.ground.ai.ties);
    assert_eq!(back.ai.beats, w.ground.ai.beats);
    println!("{n} worked ties, the ground store {} bytes", b.len());
}
