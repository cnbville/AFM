//! Ground lane G5: the week's scenes (`world/ground/scenes/`): the eye's
//! roll, the 15 scenes and their slots, a school visit's stops, the clinic
//! played and handed on, the spring game's call from 2027.

use crate::calendar::{Date, Weekday};
use crate::world::folk::scene::{self as fscene, Kind as SceneKind};
use crate::world::ground::people::{Holder, Who};
use crate::world::ground::scenes::{self, Kind};
use crate::world::ground::week::{self, DoneKind, Road, Slot, SlotKind};
use crate::world::{Stage, World};

/// The tiny college seat with recruits at the schools of one state (the
/// one with the most coached schools), as lane G3's tests set it.
fn college() -> (World, usize, Vec<usize>) {
    let mut w = super::legacy::seat("college");
    let prog = w.seat.and_then(|s| s.program).unwrap();
    let mut by_state = vec![0usize; 51];
    for h in &w.high_schools {
        if h.coach.is_some() {
            by_state[h.state.min(50)] += 1;
        }
    }
    let st = (0..51).max_by_key(|&s| (by_state[s], std::cmp::Reverse(s))).unwrap();
    let schools: Vec<usize> = (0..w.high_schools.len())
        .filter(|&h| w.high_schools[h].state == st && w.high_schools[h].coach.is_some())
        .collect();
    let season = w.dates.season;
    for (k, p) in w.people.iter_mut().filter(|p| p.stage == Stage::Pro).take(240).enumerate() {
        p.stage = Stage::HighSchool;
        p.pro = None;
        p.class_of = season + 1 + (k % 2) as i32;
        p.player.age = 17;
        p.hs = schools[k % schools.len()];
        p.home_state = st;
        p.stars = [2, 3, 3, 4][k % 4];
    }
    (w, prog, schools)
}

fn slot_of(w: &World, prog: usize, kind: SlotKind, d: Date) -> Slot {
    scenes::coach_slots(w, prog, d.on_or_before(Weekday::Mon))
        .into_iter()
        .map(|x| x.1)
        .find(|s| s.kind == kind && s.date == d)
        .unwrap_or_else(|| panic!("a {} slot on {}", kind.key(), d.pretty()))
}

#[test]
fn the_eye_rolls_the_way_a_film_catch_does() {
    // The film room's clear-catch chance: a 16 eye catches a plain thing
    // about half the time, a 9 eye about a quarter; a long week halves it
    // at the most; nothing is ever sure or hopeless.
    assert!((scenes::notice_chance(16.0, 0.0, 0.0) - 0.51).abs() < 1e-9);
    assert!((scenes::notice_chance(9.0, 0.0, 0.0) - 0.265).abs() < 1e-9);
    assert!((scenes::notice_chance(16.0, 0.0, 1.0) - 0.255).abs() < 1e-9);
    assert_eq!(scenes::notice_chance(1.0, 1.0, 0.0), 0.04);
    assert_eq!(scenes::notice_chance(40.0, 0.0, 0.0), 0.95);
}

#[test]
fn fifteen_scenes_and_every_slot_has_one() {
    assert_eq!(Kind::ALL.len(), 15);
    for k in Kind::ALL {
        assert_eq!(Kind::from_key(k.key()), Some(k));
        assert_eq!(Kind::from_code(k.code()), Some(k));
    }
    // The two built elsewhere are the recruit's call and film.
    let elsewhere: Vec<Kind> = Kind::ALL.into_iter().filter(|k| k.built().is_some()).collect();
    assert_eq!(elsewhere, vec![Kind::RecruitCall, Kind::Film]);
    for k in [
        SlotKind::Meeting,
        SlotKind::SchoolVisit,
        SlotKind::CoachCall,
        SlotKind::NightCall,
        SlotKind::JuniorDay,
        SlotKind::Unofficial,
        SlotKind::Official,
        SlotKind::Camp,
        SlotKind::Clinic,
        SlotKind::OpenPractice,
        SlotKind::SpringGame,
        SlotKind::Convention,
    ] {
        assert!(Kind::of_slot(k).is_some(), "{} has its scene", k.key());
    }
    assert_eq!(Kind::of_slot(SlotKind::Film), None, "film plays in the film room");
}

#[test]
fn the_coaches_side_follows_the_years_calendar() {
    let (w, prog, _) = college();
    // 2026: spring practice opens Monday March 2; the clinic is the Friday
    // of its third week (week 12 on the months sheet), the convention the
    // Tuesday after the third Monday of July (week 30).
    assert_eq!(scenes::clinic_day(&w, prog, Date::ymd(2026, 3, 1)), Some(Date::ymd(2026, 3, 20)));
    assert_eq!(scenes::convention_day(2026), Date::ymd(2026, 7, 21));
    let clinic = slot_of(&w, prog, SlotKind::Clinic, Date::ymd(2026, 3, 20));
    assert!(clinic.title.starts_with("Your clinic: about "), "{}", clinic.title);
    slot_of(&w, prog, SlotKind::Convention, Date::ymd(2026, 7, 21));
    slot_of(&w, prog, SlotKind::SpringGame, crate::calendar::YearDates::new(2026).spring_game);
    // Open practice on the first week's Tuesday, none in the clinic's week.
    slot_of(&w, prog, SlotKind::OpenPractice, Date::ymd(2026, 3, 3));
    let clinic_week = scenes::coach_slots(&w, prog, Date::ymd(2026, 3, 16));
    assert!(!clinic_week.iter().any(|x| x.1.kind == SlotKind::OpenPractice), "the clinic has its own practice");
    // A quiet week in May has none of them.
    assert!(scenes::coach_slots(&w, prog, Date::ymd(2026, 5, 11)).is_empty());
}

#[test]
fn the_clinic_warms_the_room_and_the_hallway_gives_names() {
    let (mut w, prog, _) = college();
    let d = Date::ymd(2026, 3, 20);
    w.today = d;
    let slot = slot_of(&w, prog, SlotKind::Clinic, d);
    let id = scenes::start(&mut w, prog, Kind::Clinic, &slot).expect("the clinic's scene");
    let s = fscene::open(&mut w, id).unwrap();
    assert_eq!(s.kind, SceneKind::Ground);
    assert_eq!(scenes::kind_of(&s), Some(Kind::Clinic));
    assert!(s.lines[0].text.starts_with("Your clinic."), "{}", s.lines[0].text);
    let crowd = scenes::state(&s).people.clone();
    assert!(!crowd.is_empty(), "coaches came");
    // No coach in the room has a memory with you yet.
    let had: usize = crowd.iter().filter(|&&c| w.ground.get(c, Holder::Program(prog as u32)).is_some()).count();
    let s = fscene::answer(&mut w, id, 101).unwrap();
    assert!(s.lines.iter().any(|l| l.text.starts_with("You talk the ")), "the talk on your offense");
    let now: usize = crowd.iter().filter(|&&c| w.ground.get(c, Holder::Program(prog as u32)).is_some()).count();
    assert_eq!(now, crowd.len(), "every coach in the room has a memory with you now (had {had})");
    let s = fscene::answer(&mut w, id, 110).unwrap();
    assert!(scenes::state(&s).stops.iter().any(|x| x.0 == "hall"));
    let s = fscene::answer(&mut w, id, scenes::LEAVE).unwrap();
    assert!(s.done);
    let p = w.ground.scenes.played.last().expect("the book keeps it");
    assert_eq!(p.kind, Kind::Clinic);
    assert!(p.yours);
    assert!(p.line.starts_with("Your clinic: "), "{}", p.line);
    // The slot's line is what the scene came to.
    w.ground.scenes.made.push((slot.date, slot.minute, slot.kind.key().into(), id));
    assert_eq!(scenes::line_for(&w, &slot).as_deref(), Some(p.line.as_str()));
}

#[test]
fn a_clinic_you_hand_on_is_your_coordinators_line() {
    let (mut w, prog, _) = college();
    let d = Date::ymd(2026, 3, 20);
    w.today = d;
    let slot = slot_of(&w, prog, SlotKind::Clinic, d);
    let id = scenes::start(&mut w, prog, Kind::Clinic, &slot).unwrap();
    let s = fscene::hand(&mut w, id).unwrap();
    assert!(s.done);
    assert_eq!(s.outcome.len(), 1, "one line (GR32)");
    let p = w.ground.scenes.played.last().unwrap();
    assert!(!p.yours);
    assert_eq!(p.kind, Kind::Clinic);
    if let Some(c) = week::coordinator(&w, prog) {
        assert_eq!(p.by, Some(c as u32));
        assert!(p.line.contains("had it"), "{}", p.line);
        assert!(s.handed.is_some());
    }
}

#[test]
fn a_school_visit_goes_stop_by_stop_and_the_road_knows_it() {
    let (mut w, prog, schools) = college();
    let me = scenes::you(&w).expect("your coach");
    // A Tuesday in the January contact period.
    let d = Date::ymd(2026, 1, 13);
    w.today = d;
    let monday = d.on_or_before(Weekday::Mon);
    w.ground.week.monday = Some(monday);
    let h = schools[0];
    let area = w.high_schools[h].ground_area;
    w.ground.week.roads.retain(|r| r.man != me as u32);
    w.ground.week.roads.push(Road {
        man: me as u32,
        area,
        days: 1 << monday.days_to(d),
        flight: false,
        why: "Your ask".into(),
        yours: true,
        focus: vec![h as u32],
    });
    let slot = Slot {
        date: d,
        minute: 9 * 60,
        kind: SlotKind::SchoolVisit,
        title: "On the road".into(),
        reason: "You're the one in the room".into(),
        yours: true,
        man: Some(me as u32),
        who: None,
        school: None,
        line: None,
        donor: None,
    };
    let id = scenes::start(&mut w, prog, Kind::SchoolVisit, &slot).expect("a school on your road");
    let s = fscene::open(&mut w, id).unwrap();
    let st = scenes::state(&s);
    let school = st.school.expect("the school") as usize;
    assert!(s.docs.iter().any(|d| d.title == "The sign-in sheet"), "the sign-in sheet");
    assert!(s.docs.iter().all(|d| !scenes::is_state(d) || d.rows.is_empty()));
    // The office, his list, then the weight room and the counselor.
    let _ = fscene::answer(&mut w, id, 1).unwrap();
    let _ = fscene::answer(&mut w, id, 101).unwrap();
    let s = fscene::answer(&mut w, id, 109).unwrap();
    if !s.done {
        let _ = fscene::answer(&mut w, id, 2).unwrap();
        let _ = fscene::answer(&mut w, id, 3).unwrap();
    }
    let s = w.folk.scenes.iter().find(|x| x.id == id).cloned().unwrap();
    let s = if s.done { s } else { fscene::answer(&mut w, id, scenes::LEAVE).unwrap() };
    assert!(s.done);
    let st = scenes::state(&s);
    assert!(st.stops.iter().any(|x| x.0 == "office"), "{:?}", st.stops);
    if let Some(Who::Coach(c)) = st.who {
        assert!(w.ground.get(Who::Coach(c), Holder::Coach(me as u32)).is_some(), "the visit is a contact with him");
    }
    assert!(
        w.ground.week.log.iter().any(|x| x.kind == DoneKind::Visit && x.school == Some(school as u32) && x.date == d),
        "the road's log has your visit, so the day doesn't send anyone back"
    );
    let p = w.ground.scenes.played.last().unwrap();
    assert_eq!(p.kind, Kind::SchoolVisit);
    assert_eq!(p.school, Some(school as u32));
}

#[test]
fn the_spring_game_is_yours_to_skip_from_2027() {
    let (mut w, prog, _) = college();
    w.today = Date::ymd(2026, 3, 1);
    assert!(scenes::set_spring_game(&mut w, false).is_err(), "every program holds it in 2026");
    assert!(scenes::holds_spring_game(&w, 2026));
    w.ground.scenes.spring_game.push((2027, false));
    assert!(!scenes::holds_spring_game(&w, 2027));
    assert!(scenes::holds_spring_game(&w, 2028), "a new year holds it unless you skip it");
    let _ = prog;
}

#[test]
fn ground_off_makes_no_scenes() {
    let (mut w, _, _) = college();
    w.ground.off = true;
    let d = Date::ymd(2026, 3, 20);
    w.today = d;
    let before = w.folk.scenes.len();
    scenes::dawn(&mut w, d);
    assert_eq!(w.folk.scenes.len(), before);
}
