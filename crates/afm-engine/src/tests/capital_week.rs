//! Capital lane C4: your week and scenes (`world/capital/week.rs`,
//! `scenes.rs`, `tour.rs`, `names.rs`, `ceremony.rs`;
//! `docs/design/capital.md`, CP2, CP13, CP20, CP25, CP38).

use crate::calendar::{Date, Weekday};
use crate::world::capital::ceremony::Rite;
use crate::world::capital::drive::{self, Stage};
use crate::world::capital::rooms::Room;
use crate::world::capital::{self, donors, names, scenes, tour, week};
use crate::world::finance::Owner;
use crate::world::folk::scene as fscene;
use crate::world::ground::week::{Slot, SlotKind, monday_of};
use crate::world::{Stage as Life, Visit, World};

/// The tiny college seat (you are its head coach) with its donors built.
fn world() -> (World, usize) {
    let mut w = super::legacy::seat("college");
    if w.capital.year == 0 {
        let d = w.today;
        capital::day(&mut w, d);
    }
    let mine = w.seat.and_then(|s| s.program).unwrap();
    assert_eq!(week::your_program(&w), Some(mine), "the seat is the head coach's");
    (w, mine)
}

fn you(w: &World) -> u32 {
    w.career.as_ref().unwrap().coach as u32
}

/// A high school recruit in the tiny world (it has none of its own: a pro
/// is made one, as the Ground tests do).
fn recruit(w: &mut World) -> usize {
    if let Some(i) = (0..w.people.len()).find(|&i| matches!(w.people[i].stage, Life::HighSchool)) {
        return i;
    }
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Life::Pro).unwrap();
    let hs = (0..w.high_schools.len()).find(|&h| w.high_schools[h].coach.is_some()).unwrap_or(0);
    let season = w.dates.season;
    let p = &mut w.people[i];
    p.stage = Life::HighSchool;
    p.pro = None;
    p.class_of = season + 1;
    p.player.age = 17;
    p.hs = hs;
    p.stars = 4;
    i
}

/// A spring Monday of the seat's year.
fn march(w: &World) -> Date {
    Date::ymd(w.year, 3, 10).on_or_after(Weekday::Mon)
}

/// CP2 and CP13: the development office puts donors on your week from
/// January to August, ranked with the week's other scenes; you book your
/// own; from September only the suite before a home game.
#[test]
fn donors_take_slots_in_your_week_and_only_the_suite_in_season() {
    let (mut w, mine) = world();
    let monday = march(&w);
    w.today = monday;
    let v = week::slots(&w, mine, monday);
    for (rank, s) in &v {
        println!("{rank} {} {}: {} ({})", s.date.pretty(), s.minute, s.title, s.reason);
    }
    assert!(!v.is_empty(), "the office has a donor for you");
    assert!(v.iter().all(|x| x.1.kind == SlotKind::Donor && x.1.donor.is_some() && x.1.date >= monday));
    // Your own booking: a weekday up to eight weeks out, one donor a day.
    let id = w.capital.donors[donors::of(&w, mine)[0]].id;
    let thu = monday.add(10);
    let said = week::book(&mut w, id, thu, true).unwrap();
    println!("{said}");
    assert!(week::book(&mut w, id, thu, false).is_err(), "one donor a day");
    assert!(week::book(&mut w, id, monday.add(12), true).is_err(), "not on a weekend");
    let oct = Date::ymd(w.year, 10, 7);
    assert!(week::book(&mut w, id, oct, true).is_err(), "in season only the suite");
    let next = week::slots(&w, mine, monday.add(7));
    let b = next.iter().find(|x| x.1.donor == Some(id) && x.1.date == thu).expect("the booking is on its week");
    assert_eq!(b.0, week::RANK_BOOKED);
    assert!(week::unbook(&mut w, id, thu).is_ok());
    assert!(
        !week::slots(&w, mine, monday.add(7)).iter().any(|x| x.1.date == thu && x.1.reason == "You asked for this one")
    );
    // The year's rooms of donors.
    let c = week::caravan_day(w.year);
    assert!(week::slots(&w, mine, monday_of(c)).iter().any(|x| x.1.kind == SlotKind::DonorRoom && x.1.date == c));
    let g = week::golf_day(w.year);
    assert_eq!(g.weekday(), Weekday::Fri);
    // In season: the suite before each home game, nothing else.
    let homes = crate::world::front_office::college::home_games(&w, mine);
    for (d, _) in homes.iter().filter(|g| g.0.month() >= 9).take(3) {
        w.today = monday_of(*d);
        let v = week::slots(&w, mine, monday_of(*d));
        println!("{}: {:?}", d.pretty(), v.iter().map(|x| x.1.title.clone()).collect::<Vec<_>>());
        assert!(v.iter().all(|x| x.1.kind == SlotKind::DonorRoom), "in season only the suite");
        assert!(v.iter().any(|x| x.1.date == *d && x.1.title.starts_with("The suite")));
    }
}

/// A dinner booked weeks ahead holds: when that week's plan is drafted, the
/// coordinator keeps the head coach off the road that day (January is a
/// contact period, when the head coach travels).
#[test]
fn a_booked_dinner_keeps_you_off_the_road() {
    let (mut w, mine) = world();
    let monday = Date::ymd(w.year, 1, 19).on_or_after(Weekday::Mon);
    w.today = monday.add(-21);
    let list = donors::of(&w, mine);
    let days: Vec<Date> = (1..5).map(|k| monday.add(k)).collect();
    for (k, &d) in days.iter().enumerate() {
        let id = w.capital.donors[list[k]].id;
        week::book(&mut w, id, d, true).unwrap();
    }
    w.today = monday;
    crate::world::ground::week::plan::ensure(&mut w, mine, monday);
    let hc = w.programs[mine].head_coach.map(|c| c as u32);
    for r in w.ground.week.roads.iter().filter(|r| Some(r.man) == hc) {
        println!("head coach on the road: {:07b}", r.days);
        assert_eq!(r.days & 0b11110, 0, "not on a booked day");
    }
    let v = week::slots(&w, mine, monday);
    assert!(days.iter().all(|d| v.iter().any(|x| x.1.date == *d && x.0 == week::RANK_BOOKED)));
}

/// A booking in this week takes one of the scenes that are yours.
#[test]
fn a_booking_this_week_takes_a_slot_from_your_scenes() {
    let (mut w, mine) = world();
    let monday = march(&w);
    w.today = monday;
    crate::world::ground::week::plan::ensure(&mut w, mine, monday);
    // Fill the week to five of yours.
    let mut k = 0;
    while w.ground.week.slots.iter().filter(|s| s.yours).count() < crate::world::ground::week::plan::YOURS.1 {
        w.ground.week.slots.push(Slot {
            date: monday.add(4),
            minute: 600 + k,
            kind: SlotKind::Unofficial,
            title: format!("Unofficial visits {k}"),
            reason: "Families on campus".into(),
            yours: true,
            man: None,
            who: None,
            school: None,
            line: None,
            donor: None,
        });
        k += 1;
    }
    let id = w.capital.donors[donors::of(&w, mine)[1]].id;
    let said = week::book(&mut w, id, monday.add(2), false).unwrap();
    println!("{said}");
    assert!(said.contains("coordinator"));
    let yours = w.ground.week.slots.iter().filter(|s| s.yours).count();
    assert_eq!(yours, crate::world::ground::week::plan::YOURS.1);
    assert!(w.ground.week.slots.iter().any(|s| s.yours && s.donor == Some(id)));
}

fn donor_slot(w: &World, donor: u32, d: Date) -> Slot {
    Slot {
        date: d,
        minute: week::DINNER,
        kind: SlotKind::Donor,
        title: format!("Dinner with {}", w.capital.donor(donor).unwrap().name),
        reason: "You asked for this one".into(),
        yours: true,
        man: None,
        who: None,
        school: None,
        line: None,
        donor: Some(donor),
    }
}

/// A dinner with a donor: hearing him out and being there warm him, it
/// counts as a meeting, and the slot gets its line.
#[test]
fn a_dinner_warms_him_and_counts_as_a_meeting() {
    let (mut w, mine) = world();
    let d = march(&w).add(3);
    w.today = d;
    let me = you(&w);
    let donor = w.capital.donors[donors::of(&w, mine)[0]].id;
    let before = w.capital.donor(donor).unwrap().warmth_to(me);
    let met = w.capital.donor(donor).unwrap().bond(me).map_or(0, |b| b.met);
    let slot = donor_slot(&w, donor, d);
    w.ground.week.slots.push(slot.clone());
    let id = scenes::start(&mut w, mine, &slot).expect("a scene");
    w.ground.scenes.made.push((slot.date, slot.minute, slot.kind.key().to_string(), id));
    let s = fscene::open(&mut w, id).unwrap();
    for l in &s.lines {
        println!("> {}", l.text);
    }
    assert!(s.picks.iter().any(|p| p.label.starts_with("Hear him out")));
    let s = fscene::answer(&mut w, id, 3).unwrap();
    println!("> {}", s.lines.last().unwrap().text);
    let s = fscene::answer(&mut w, id, 900).unwrap();
    assert!(s.done);
    println!("{}", s.outcome.join(" "));
    let after = w.capital.donor(donor).unwrap();
    assert!(after.warmth_to(me) > before + 0.02, "{before} -> {}", after.warmth_to(me));
    assert_eq!(after.bond(me).unwrap().met, met + 1);
    assert!(w.ground.week.slots.iter().any(|x| x.donor == Some(donor) && x.line.is_some()));
}

/// The lead gift comes from your own ask across the table (`gifts::ask`),
/// and a donor who'd rather give to the other side can be pushed.
#[test]
fn you_ask_across_the_table_and_push() {
    let (mut w, mine) = world();
    w.finance.projects.retain(|x| x.owner != Owner::College(mine));
    w.today = Date::ymd(w.year, 7, 1);
    drive::start(&mut w, mine).expect("a drive");
    let k = drive::open_drive(&w.capital, mine).unwrap();
    w.capital.drives[k].stage = Stage::Quiet;
    let list = donors::of(&w, mine);
    let donor = w.capital.donors[list[0]].id;
    w.capital.donors[list[0]].nil_lean = -0.9;
    let d = Date::ymd(w.year, 7, 14);
    w.today = d;
    let slot = donor_slot(&w, donor, d);
    let id = scenes::start(&mut w, mine, &slot).unwrap();
    let s = fscene::open(&mut w, id).unwrap();
    assert!(s.picks.iter().any(|p| p.code == 1), "the drive's ask");
    let s = fscene::answer(&mut w, id, 1).unwrap();
    println!("> {}", s.lines.last().unwrap().text);
    assert!(s.picks.iter().any(|p| p.code == 6), "he'd rather pay your players: push or take it");
    let s = fscene::answer(&mut w, id, 6).unwrap();
    println!("> {}", s.lines.last().unwrap().text);
    let x = &w.capital.drives[k];
    assert!(x.asked.iter().any(|a| a.0 == donor), "he was asked in the room");
    if !s.done {
        fscene::answer(&mut w, id, 900).unwrap();
    }
}

/// The suite before kickoff: sit with one donor, or work the room.
#[test]
fn the_suite_before_kickoff() {
    let (mut w, mine) = world();
    let d = Date::ymd(w.year, 10, 3).on_or_after(Weekday::Sat);
    w.today = d;
    let me = you(&w);
    let slot = Slot {
        date: d,
        minute: 10 * 60,
        kind: SlotKind::DonorRoom,
        title: "The suite before kickoff: a rival".into(),
        reason: "Your only donor hour in season".into(),
        yours: true,
        man: None,
        who: None,
        school: None,
        line: None,
        donor: None,
    };
    let id = scenes::start(&mut w, mine, &slot).unwrap();
    let s = fscene::open(&mut w, id).unwrap();
    let present = scenes::state(&s).present;
    assert!(!present.is_empty() && present.len() <= week::SUITE);
    let before: Vec<f32> = present.iter().map(|&x| w.capital.donor(x).unwrap().warmth_to(me)).collect();
    let s = fscene::answer(&mut w, id, 10).unwrap();
    assert!(s.done);
    for (k, &x) in present.iter().enumerate() {
        assert!(w.capital.donor(x).unwrap().warmth_to(me) > before[k]);
    }
    // Sitting with one: one exchange.
    let id = scenes::start(&mut w, mine, &slot).unwrap();
    fscene::open(&mut w, id).unwrap();
    let s = fscene::answer(&mut w, id, 20).unwrap();
    assert!(!s.done && s.picks.iter().any(|p| p.code == 3));
    let s = fscene::answer(&mut w, id, 3).unwrap();
    assert!(s.done, "one exchange in a suite");
    println!("{}", s.outcome.join(" "));
}

/// CP20: you choose the three rooms a recruit tours; a tour that fits him
/// beats your staff's usual one, a poor one costs the weekend, and only
/// that kid's weekend with you moves.
#[test]
fn the_tour_you_choose_moves_his_weekend() {
    let (mut w, mine) = world();
    let i = recruit(&mut w);
    let kid = w.people[i].player.id;
    let other = (0..w.programs.len()).find(|&q| q != mine && w.programs[q].campus.set).unwrap_or(mine);
    let d = w.today.add(3);
    w.people[i].cal.visits.push((mine, d, Visit::Official));
    w.people[i].cal.visits.push((other, d.add(7), Visit::Official));
    assert_eq!(tour::lift(&w, kid, mine), 0.0, "no tour chosen: the built weekend");
    let pr = &w.programs[mine];
    let wants = tour::wants(&w.people[i]);
    assert!((wants.iter().sum::<f64>() - 1.0).abs() < 1e-9);
    let mut rooms = tour::tourable(pr);
    assert!(rooms.len() >= tour::STOPS);
    let fit_of = |r: Room| {
        let c = &pr.campus;
        wants[r as usize] * ((c.read[r as usize] - c.typical_read[r as usize]) as f64 / 2.0).clamp(-1.0, 1.0)
    };
    rooms.sort_by(|&a, &b| fit_of(b).total_cmp(&fit_of(a)));
    let mut best: Vec<Room> = rooms.iter().copied().take(tour::STOPS).collect();
    let top = tour::top(&wants);
    if rooms.contains(&top) && !best.contains(&top) {
        best[tour::STOPS - 1] = top;
    }
    let worst: Vec<Room> = rooms.iter().rev().copied().filter(|&r| r != top).take(tour::STOPS).collect();
    let a = crate::world::recruiting::appeal(&w, i, mine, None);
    let b = crate::world::recruiting::appeal(&w, i, other, None);
    let words = tour::set(&mut w, kid, &best).unwrap();
    println!("best: {words} ({:+.3})", tour::lift(&w, kid, mine));
    let good = tour::lift(&w, kid, mine);
    assert!(good >= 0.0);
    let a2 = crate::world::recruiting::appeal(&w, i, mine, None);
    assert!(((a2 - a) - 0.02 * good).abs() < 1e-9, "the official weekend's pull moves by the lift");
    assert_eq!(crate::world::recruiting::appeal(&w, i, other, None), b, "another school's weekend is its own");
    let words = tour::set(&mut w, kid, &worst).unwrap();
    println!("worst: {words} ({:+.3})", tour::lift(&w, kid, mine));
    assert!(tour::lift(&w, kid, mine) <= good);
    assert_eq!(w.capital.tours.iter().filter(|t| t.kid == kid).count(), 1, "one tour a weekend");
    if rooms.contains(&top) {
        assert!(words.contains("asked where"), "he noticed what you skipped: {words}");
    }
    assert!(tour::set(&mut w, kid, &best[..2]).is_err());
    w.people[i].cal.visits.retain(|v| v.0 != mine);
    assert!(tour::set(&mut w, kid, &best).is_err(), "no visit, no tour");
}

/// Names on buildings: the store's names at the handover, a building's
/// lead gift names it and a big gift names a room.
#[test]
fn names_go_up_on_buildings_and_rooms() {
    let (mut w, mine) = world();
    names::seed(&mut w);
    let n = w.capital.names.len();
    println!("{n} names up across the world");
    assert!(n > 0);
    assert!(w.capital.names.iter().all(|x| x.room.is_some() && x.drive == 0));
    w.finance.projects.retain(|x| x.owner != Owner::College(mine));
    let (id, kind) = fake_drive(&mut w, mine);
    names::name_buildings(&mut w);
    let up: Vec<names::Name> = w.capital.names.iter().filter(|x| x.drive == id).cloned().collect();
    for x in &up {
        println!("{}", x.words(&w));
    }
    assert!(up.iter().any(|x| x.room.is_none() && x.hall == kind), "the lead gift names the building");
    assert!(up.iter().any(|x| x.room.is_some()), "a big gift names a room");
    names::name_buildings(&mut w);
    assert_eq!(w.capital.names.iter().filter(|x| x.drive == id).count(), up.len(), "named once");
}

/// A drive at your program whose ground broke today, with a lead gift of
/// 40% of its goal and a gift of 10% to a room in it.
fn fake_drive(w: &mut World, p: usize) -> (u32, String) {
    let today = w.today;
    let id = w.capital.id();
    let list = donors::of(w, p);
    let (a, b) = (w.capital.donors[list[0]].id, w.capital.donors[list[1]].id);
    let rooms = Room::Weight.bit() | Room::Locker.bit() | Room::Film.bit();
    w.capital.drives.push(capital::Drive {
        id,
        program: p as u32,
        kind: "football building".into(),
        goal: 10.0e6,
        stage: Stage::Public,
        ground: Some(w.year),
        ground_day: Some(today),
        opens: Some(w.year + 2),
        rooms,
        ..Default::default()
    });
    for (donor, total, room) in [(a, 4.0e6, None), (b, 1.0e6, Some(Room::Weight as u8))] {
        let pid = capital::pledges::make(w, donor, p as u32, total, 4, None, room, today);
        w.capital.pledges.iter_mut().find(|x| x.id == pid).unwrap().drive = id;
    }
    (id, "football building".into())
}

/// CP25: a named donor's scandal is your call; down, his circle cools and
/// his pledge may go; kept, the questions come for six months and the AD
/// gets nervous. Left, the university decides.
#[test]
fn a_named_donors_scandal_is_your_call() {
    let (mut w, mine) = world();
    let me = you(&w);
    let (drive, _) = fake_drive(&mut w, mine);
    names::name_buildings(&mut w);
    let name = w.capital.names.iter().find(|x| x.drive == drive && x.room.is_none()).unwrap().clone();
    let today = w.today;
    let x = names::start(&mut w, mine, name.id, name.donor, "is charged with fraud in his business", 0.7, today);
    let scene = w.capital.scandals.iter().find(|s| s.id == x).unwrap().scene;
    let s = fscene::open(&mut w, scene).unwrap();
    for l in &s.lines {
        println!("> {}", l.text);
    }
    assert!(s.picks.iter().any(|p| p.code == 1) && s.picks.iter().any(|p| p.code == 2));
    let s = fscene::answer(&mut w, scene, 1).unwrap();
    println!("{}", s.outcome.join(" "));
    assert!(s.done);
    assert!(w.capital.names.iter().find(|n| n.id == name.id).unwrap().down.is_some());
    assert!(w.capital.donor(name.donor).unwrap().warmth_to(me) <= names::DOWN_WARMTH);
    assert!(names::decide(&mut w, x, false, true).contains("made"), "one call");
    // Kept: the questions and the AD's nerves.
    let room = w.capital.names.iter().find(|n| n.drive == drive && n.room.is_some()).unwrap().clone();
    let y = names::start(&mut w, mine, room.id, room.donor, "faces a federal tax case", 0.5, today);
    let ad = crate::world::folk::boss::boss(&w, crate::world::folk::Team::College(mine)).key();
    let t0 = w.folk.mine.trust_of(ad).unwrap_or(0.5);
    names::call(&mut w, y, false).unwrap();
    let t1 = w.folk.mine.trust_of(ad).unwrap_or(0.5);
    assert!(t1 < t0);
    let first = Date::ymd(today.year(), today.parts().1, 1).add(40);
    let first = Date::ymd(first.year(), first.parts().1, 1);
    names::questions(&mut w, mine, first);
    assert!(w.folk.mine.trust_of(ad).unwrap_or(0.5) < t1, "the questions come");
    assert!(w.capital.names.iter().find(|n| n.id == room.id).unwrap().up());
    let sy = w.capital.scandals.iter().find(|s| s.id == y).unwrap().scene;
    assert!(w.folk.scenes.iter().find(|s| s.id == sy).is_some_and(|s| s.done), "the AD's call is answered");
}

/// CP38's data: the groundbreaking is set on a weekend with recruits on
/// campus and held with the donors on stage, the recruits there and the
/// next day's headline; the reveal video the day the drive goes public.
#[test]
fn the_groundbreaking_keeps_its_moment() {
    let (mut w, mine) = world();
    let (drive, _) = fake_drive(&mut w, mine);
    names::name_buildings(&mut w);
    let today = w.today;
    let sat = today.add(1).on_or_after(Weekday::Sat);
    for p in w.people.iter_mut() {
        p.cal.visits.retain(|v| v.0 != mine || v.1 < today || v.1 > today.add(21));
    }
    let kids: Vec<usize> = (0..3)
        .map(|_| {
            let i = recruit(&mut w);
            w.people[i].stage = Life::PrepSchool;
            i
        })
        .collect();
    for &i in &kids {
        w.people[i].cal.visits.push((mine, sat.add(7), Visit::Official));
    }
    capital::ceremony::schedule(&mut w, mine, today);
    let k = w.capital.ceremonies.iter().position(|c| c.drive == drive && c.rite == Rite::Groundbreaking).unwrap();
    assert_eq!(w.capital.ceremonies[k].day, sat.add(7), "the weekend with recruits on campus");
    capital::ceremony::schedule(&mut w, mine, today);
    assert_eq!(w.capital.ceremonies.iter().filter(|c| c.drive == drive).count(), 1, "set once");
    capital::ceremony::hold(&mut w, k);
    let c = &w.capital.ceremonies[k];
    println!("{}", c.headline);
    for r in &c.kids {
        println!("  {}", r.words);
    }
    assert!(c.held && !c.headline.is_empty());
    assert_eq!(c.kids.len(), kids.len());
    let namer = w.capital.names.iter().find(|n| n.drive == drive && n.room.is_none()).unwrap().donor;
    assert_eq!(c.donors[0].0, namer, "the building's namer first on the stage");
    assert!(c.building.starts_with("the "));
    // The reveal video, the day the drive goes public.
    let x = w.capital.drives.iter_mut().find(|x| x.id == drive).unwrap();
    x.public_on = Some(today);
    capital::ceremony::schedule(&mut w, mine, today);
    assert!(w.capital.ceremonies.iter().any(|c| c.drive == drive && c.rite == Rite::Reveal && c.day == today));
}

/// The new stores save, and an older file loads with them empty.
#[test]
fn the_week_store_saves_and_an_old_file_loads_empty() {
    let (mut w, mine) = world();
    let (_, _) = fake_drive(&mut w, mine);
    names::name_buildings(&mut w);
    let s = serde_json::to_string(&w.capital).unwrap();
    let back: capital::Capital = serde_json::from_str(&s).unwrap();
    assert_eq!(back.names.len(), w.capital.names.len());
    let old: capital::Capital = serde_json::from_str(r#"{"donors":[]}"#).unwrap();
    assert!(old.booked.is_empty() && old.tours.is_empty() && old.names.is_empty() && !old.seeded);
    let slot: Slot = serde_json::from_str(
        r#"{"date":738000,"minute":600,"kind":"Official","title":"t","reason":"r","yours":true,"man":null,"who":null,"school":null,"line":null}"#,
    )
    .unwrap();
    assert!(slot.donor.is_none());
}
