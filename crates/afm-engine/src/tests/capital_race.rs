//! Capital lane C5: the facilities race and the press
//! (`world/capital/race.rs`; `docs/design/capital.md`, CP29-CP31).

use crate::calendar::{Date, Kind};
use crate::world::World;
use crate::world::capital::drive::{self, Stage};
use crate::world::capital::race::{self, Seen};
use crate::world::capital::rooms::Room;
use crate::world::capital::{self, donors};
use crate::world::finance::Owner;

/// The tiny college seat with its donors built, and a rival AI program in
/// your conference (else your state).
fn world() -> (World, usize, usize) {
    let mut w = super::legacy::seat("college");
    if w.capital.year == 0 {
        let d = w.today;
        capital::day(&mut w, d);
    }
    let mine = w.seat.and_then(|s| s.program).unwrap();
    let rival = (0..w.programs.len())
        .find(|&q| race::rival(&w, mine, q) && w.programs[q].conference == w.programs[mine].conference)
        .or_else(|| (0..w.programs.len()).find(|&q| race::rival(&w, mine, q)))
        .expect("a rival");
    (w, mine, rival)
}

fn rich(w: &mut World, p: usize) {
    for k in donors::of(w, p) {
        let d = &mut w.capital.donors[k];
        d.capacity = 30.0e6;
        d.affinity = 0.9;
        d.nil_lean = 1.0;
        d.collects = 0.0;
    }
}

/// The first of each month, `n` months: the drives' step, then the rumors.
fn months(w: &mut World, n: i32) {
    for _ in 0..n {
        let (y, m, _) = w.today.parts();
        let next = if m == 12 { Date::ymd(y + 1, 1, 1) } else { Date::ymd(y, m + 1, 1) };
        w.today = next;
        drive::month(w, next);
        race::rumors(w, next);
    }
}

/// CP30: a rival's study and quiet phase never reach the news; a donor of
/// both schools who was in the room tells you first hand, by his warmth,
/// and his read of their progress is close. Once public, the screen sees
/// the drive itself.
#[test]
fn a_rivals_quiet_phase_reaches_you_only_through_your_donors() {
    let (mut w, mine, rival) = world();
    let coach = w.career.as_ref().unwrap().coach as u32;
    rich(&mut w, rival);
    // One of the rival's donors is yours too, warm to you and chatty.
    let k = donors::of(&w, rival).into_iter().find(|&k| w.capital.donors[k].home as usize == rival).unwrap();
    let id = w.capital.donors[k].id;
    {
        let d = &mut w.capital.donors[k];
        d.also = Some(mine as u32);
        d.also_pull = 0.3;
        d.meddling = 20.0;
        d.bond_mut(coach).warmth = 0.95;
    }
    w.finance.projects.retain(|x| x.owner != Owner::College(rival));
    w.today = Date::ymd(w.year, 7, 1);
    let drive_id = drive::start(&mut w, rival).expect("a rival drive");
    let start = w.today;
    // The study runs its five months on the drive's own clock.
    months(&mut w, 5);
    // Then hold it in the quiet phase with him asked, and let the months
    // pass: only the rumors run.
    let k = w.capital.drives.iter().position(|d| d.id == drive_id).unwrap();
    w.capital.drives[k].stage = Stage::Quiet;
    w.capital.drives[k].public_on = None;
    if !w.capital.drives[k].asked.iter().any(|a| a.0 == id) {
        w.capital.drives[k].asked.push((id, Stage::Quiet as u8));
    }
    for _ in 0..24 {
        let (y, m, _) = w.today.parts();
        w.today = if m == 12 { Date::ymd(y + 1, 1, 1) } else { Date::ymd(y, m + 1, 1) };
        let d = w.today;
        race::rumors(&mut w, d);
    }
    let heard: Vec<_> = w.capital.rumors.iter().filter(|r| r.drive == drive_id).cloned().collect();
    for r in &heard {
        println!("{} ({:?}, direct {}): {}", r.date, r.stage, r.direct, r.text);
    }
    assert!(!heard.is_empty(), "a warm donor of both schools tells you");
    assert!(heard.iter().all(|r| matches!(r.stage, Stage::Study | Stage::Quiet)));
    assert!(heard.len() <= 2, "once a stage");
    let r = heard.iter().find(|r| r.stage == Stage::Quiet).expect("the quiet phase reached you");
    if r.from == id {
        assert!(r.direct);
        let (h, t) = (r.heard.unwrap(), r.truth.unwrap());
        assert!((h - t).abs() < 0.4, "a first-hand read is close: {h} against {t}");
        assert_eq!(r.building, format!("a new {}", w.capital.drives[k].kind));
    }
    // Every rumor is a whisper to your program, never a news line about the
    // rival's quiet drive.
    let end = w.today.add(1);
    let mut whispers = 0;
    for (_, evs) in w.calendar.events.range(start..end) {
        for e in evs {
            if e.program == Some(rival) && e.kind == Kind::News && e.rule == "money" {
                assert!(
                    !e.text.contains("quiet") && !e.text.contains("study"),
                    "a rival's quiet drive in the news: {}",
                    e.text
                );
            }
            if e.kind == Kind::Whisper && e.program == Some(mine) {
                whispers += 1;
            }
        }
    }
    assert!(whispers >= heard.len());
    // What the Pecking order screen may show: the rumor while it is quiet,
    // the drive itself once public.
    let seen = |w: &World| {
        race::rivals_drives(w).into_iter().find_map(|s| match s {
            Seen::Public(d) if d.id == drive_id => Some(true),
            Seen::Rumored(r) if r.drive == drive_id => Some(false),
            _ => None,
        })
    };
    assert_eq!(seen(&w), Some(false));
    w.capital.drives[k].stage = Stage::Public;
    w.capital.drives[k].public_on = Some(w.today);
    assert_eq!(seen(&w), Some(true));
}

/// CP30: no rumors without a college seat, and none about programs outside
/// the race.
#[test]
fn rumors_come_only_to_a_college_seat_about_rivals() {
    let (mut w, mine, _) = world();
    let far = (0..w.programs.len())
        .find(|&q| q != mine && w.programs[q].plays(w.year) && !race::rival(&w, mine, q) && w.programs[q].campus.set)
        .unwrap();
    rich(&mut w, far);
    w.finance.projects.retain(|x| x.owner != Owner::College(far));
    w.today = Date::ymd(w.year, 7, 1);
    let id = drive::start(&mut w, far).expect("a drive");
    months(&mut w, 18);
    assert!(w.capital.rumors.iter().all(|r| r.drive != id), "a program outside the race stays quiet");
    w.seat = None;
    let before = w.capital.rumors.len();
    months(&mut w, 6);
    assert_eq!(w.capital.rumors.len(), before, "no seat, no rumors");
}

/// CP31: each July every conference of two or more is ranked, best first;
/// a room built this season shines; your rank over the years is kept, and
/// your conference's order is in the news once the rooms live.
#[test]
fn the_press_ranks_every_conference_each_july() {
    let (mut w, mine, rival) = world();
    w.today = Date::ymd(w.year, 7, 1);
    race::pecking(&mut w);
    let year = w.year;
    let this: Vec<_> = w.capital.pecking.iter().filter(|x| x.year == year).cloned().collect();
    assert!(!this.is_empty());
    for x in &this {
        assert!(x.order.len() >= 2);
        assert!(x.order.windows(2).all(|p| p[0].score >= p[1].score), "best first");
    }
    let conf = this.iter().find(|x| x.rank(mine).is_some()).expect("your conference is ranked");
    println!(
        "{}: {}",
        conf.conference,
        conf.order
            .iter()
            .map(|y| format!("{} {:.0}", w.program_name(y.program as usize), y.score))
            .collect::<Vec<_>>()
            .join(", ")
    );
    // One July a second time does nothing.
    let n = w.capital.pecking.len();
    race::pecking(&mut w);
    assert_eq!(w.capital.pecking.len(), n);
    // A new weight room shines for two seasons.
    let c = &mut w.programs[rival].campus;
    c.level[Room::Weight as usize] = 3.0;
    c.since[Room::Weight as usize] = 0;
    let (before, _) = race::press_score(&w, rival);
    w.programs[rival].campus.since[Room::Weight as usize] = year;
    let (shine, new) = race::press_score(&w, rival);
    assert!(shine > before && new & Room::Weight.bit() != 0);
    w.programs[rival].campus.since[Room::Weight as usize] = year - 2;
    assert!((race::press_score(&w, rival).0 - before).abs() < 1e-9, "the shine lasts two seasons");
    // A season on: your rank over the years.
    w.year += 1;
    w.today = Date::ymd(w.year, 7, 1);
    race::pecking(&mut w);
    let ranks = race::ranks(&w, mine);
    assert_eq!(ranks.len(), 2);
    assert_eq!(ranks[1].0, year + 1);
    if capital::live(&w) {
        let news: Vec<_> = w
            .calendar
            .on(w.today)
            .iter()
            .filter(|e| e.program == Some(mine) && e.text.starts_with("The press ranks"))
            .collect();
        assert_eq!(news.len(), 1, "your conference's order in the news");
        println!("{}", news[0].text);
    }
}

/// CP29: the room the why-line names is the rival's new room he cares
/// about, read over yours; the rooms' part is the decision's own terms.
#[test]
fn a_rivals_new_room_names_itself_on_the_why_line() {
    let (mut w, mine, rival) = world();
    let i = (0..w.people.len())
        .find(|&i| {
            matches!(w.people[i].player.position, crate::positions::Position::OT | crate::positions::Position::IOL)
        })
        .unwrap();
    let year = w.year;
    for r in Room::ALL {
        let k = r as usize;
        w.programs[mine].campus.level[k] = 3.0;
        w.programs[mine].campus.read[k] = 3.0;
        w.programs[rival].campus.level[k] = 3.0;
        w.programs[rival].campus.read[k] = 3.0;
        w.programs[rival].campus.since[k] = year - 8;
    }
    assert_eq!(race::room_to_name(&w, i, rival, mine), None, "level rooms: nothing to name");
    // An old, better stadium and a new weight room: the lineman hears of
    // the weight room.
    let c = &mut w.programs[rival].campus;
    c.level[Room::Stadium as usize] = 5.0;
    c.read[Room::Stadium as usize] = 5.0;
    c.level[Room::Weight as usize] = 5.0;
    c.read[Room::Weight as usize] = 5.0;
    c.since[Room::Weight as usize] = year;
    assert_eq!(race::room_to_name(&w, i, rival, mine), Some((Room::Weight, true)));
    // Three seasons on it isn't new: the room he cares about most is named.
    w.programs[rival].campus.since[Room::Weight as usize] = year - race::NEW_SEASONS;
    let named = race::room_to_name(&w, i, rival, mine);
    assert!(matches!(named, Some((_, false))));
    // The rooms' part is the facilities grade's term, plus the visits'.
    let (g, v) = race::rooms_part(&w, i, rival);
    let grade = w.programs[rival].grades[crate::world::program::GradeCat::Facilities as usize];
    assert!((g - 0.1 * grade / 100.0).abs() < 1e-12);
    assert!(v.abs() < 0.02);
}

/// The stores save, and an old file without them loads empty.
#[test]
fn the_race_stores_save_and_an_old_file_loads_empty() {
    let (mut w, _, _) = world();
    w.today = Date::ymd(w.year, 7, 1);
    race::pecking(&mut w);
    let json = serde_json::to_string(&w.capital).unwrap();
    let back: capital::Capital = serde_json::from_str(&json).unwrap();
    assert_eq!(back.pecking, w.capital.pecking);
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    v.as_object_mut().unwrap().remove("pecking");
    v.as_object_mut().unwrap().remove("rumors");
    let old: capital::Capital = serde_json::from_value(v).unwrap();
    assert!(old.pecking.is_empty() && old.rumors.is_empty());
}
