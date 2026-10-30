//! The hunt (next update, lane 11, SK67): naming a target follows each
//! seat's rules, heat starts hidden and saying it out loud tells everyone,
//! a college banks money out of its NIL budget and gets it back when it
//! lets him go, the generational label needs a full record, the AI's
//! programs hunt a recruit each summer, and a club reads what he costs.

use super::legacy::seat;
use crate::calendar::Date;
use crate::world::front_office::college::budget;
use crate::world::hunt::{self, End, Seat, ai, generational};
use crate::world::{Stage, World};

fn huntable(w: &World) -> Vec<u32> {
    w.people.iter().map(|p| p.player.id).filter(|&id| hunt::can_hunt(w, id).is_ok()).collect()
}

/// A world with no history has only its youngest high school class, so
/// the seat gets the classes above it.
fn world(level: &str) -> World {
    let mut w = seat(level);
    for age in [15, 16, 17] {
        w.add_class_aged(age);
    }
    w.rebuild_rosters();
    w
}

fn college() -> World {
    world("college")
}

/// A world with no history has no college players yet, so a few young
/// pros go back to college for the club seat.
fn prospects(w: &mut World) {
    for p in w.people.iter_mut().filter(|p| p.stage == Stage::Pro).take(12) {
        p.stage = Stage::College;
        p.pro = None;
    }
}

#[test]
fn each_seat_names_a_hunt_by_its_own_rules() {
    for level in ["nfl", "college", "hs"] {
        let mut w = if level == "nfl" { seat(level) } else { world(level) };
        if level == "nfl" {
            prospects(&mut w);
        }
        let (s, team) = hunt::seat(&w).unwrap();
        let ids = huntable(&w);
        assert!(ids.len() >= 2, "{level}: {} to hunt", ids.len());
        // The seat's own rule refuses the wrong man.
        let wrong = w.people.iter().find(|p| match s {
            Seat::Club => p.stage == Stage::Pro,
            Seat::College => p.stage == Stage::Pro,
            Seat::School => {
                p.stage == Stage::HighSchool
                    && (p.hs == team || w.high_schools[p.hs].state != w.high_schools[team].state)
            }
        });
        if let Some(p) = wrong {
            assert!(hunt::can_hunt(&w, p.player.id).is_err(), "{level}: {} is not huntable", p.name);
        }
        let text = hunt::name(&mut w, ids[0]).unwrap();
        assert!(text.starts_with("Your hunt: "), "{level}: {text}");
        assert!(hunt::is_yours(&w, ids[0]));
        assert!(hunt::name(&mut w, ids[1]).is_err(), "{level}: one hunt at a time");
        if s == Seat::College {
            let t = w.front_office.college.board.iter().find(|t| t.id == ids[0]).expect("on your board");
            assert!(t.priority, "the head coach's own time");
        }
    }
}

#[test]
fn heat_starts_hidden_and_out_loud_tells_everyone() {
    for level in ["college", "hs"] {
        let mut w = if level == "nfl" { seat(level) } else { world(level) };
        let id = huntable(&w)[0];
        hunt::name(&mut w, id).unwrap();
        let before = hunt::heat_now(&w);
        assert!(before < 0.5, "{level}: {before} the day you name him");
        let words = hunt::loud(&mut w).unwrap();
        assert!(!words.contains(" his ") && !words.contains(" her "), "{level}: {words}");
        assert!((hunt::heat_now(&w) - 1.0).abs() < 1e-9, "{level}: out loud is all the way");
        assert!(hunt::loud(&mut w).is_err(), "{level}: said once");
    }
}

#[test]
fn a_college_banks_money_out_of_its_budget_and_gets_it_back() {
    let mut w = college();
    let (_, j) = hunt::seat(&w).unwrap();
    let id = huntable(&w)[0];
    hunt::name(&mut w, id).unwrap();
    // A world with no history has no collective money yet.
    w.programs[j].collective.budget = 2_000_000.0;
    let before = budget(&w, j).left;
    assert!(before > 0.0, "a budget to bank from");
    let x = (before * 0.4).round();
    hunt::bank(&mut w, Some(x), Some(true)).unwrap();
    assert!((before - budget(&w, j).left - x).abs() < 1.0, "the money leaves what's left for others");
    assert!(hunt::bank(&mut w, Some(before * 10.0), None).is_err(), "no more than the budget");
    assert!(hunt::bank(&mut w, Some(-1.0), None).is_err());
    hunt::drop(&mut w).unwrap();
    assert!((budget(&w, j).left - before).abs() < 1.0, "letting him go frees the money");
    assert!(hunt::yours(&w).is_none());
    let past = w.hunt.past.last().expect("kept in the record");
    assert_eq!(past.id, id);
    assert_eq!(past.end.as_ref().unwrap().1, End::Dropped);
}

#[test]
fn the_generational_label_needs_a_full_record() {
    let mut w = seat("hs");
    w.hunt.tops.clear();
    let top = |year: i32, read: f32| generational::Top {
        year,
        draft: false,
        id: year as u32,
        name: format!("P{year}"),
        read,
    };
    w.hunt.tops.push(top(2020, 95.0));
    for y in 2031..=2038 {
        w.hunt.tops.push(top(y, 80.0));
    }
    assert!(generational::beats_record(&w, false, 2039, 90.0).is_none(), "eight classes of record are not enough");
    w.hunt.tops.push(top(2030, 82.0));
    assert!(generational::beats_record(&w, false, 2039, 82.0).is_none(), "a tie is no record");
    let since = generational::beats_record(&w, false, 2039, 90.0).expect("beats nine classes");
    assert_eq!(since, Some((2020, "P2020".to_string())), "the last one read higher");
    assert!(generational::beats_record(&w, true, 2039, 99.0).is_none(), "the draft keeps its own record");
}

#[test]
fn ai_programs_hunt_a_recruit_each_summer() {
    let mut w = college();
    let (_, mine) = hunt::seat(&w).unwrap();
    let d = Date::ymd(w.today.year(), 7, 14);
    let class_of = d.year() + 2;
    // The services rate the class: three to five stars, a read each.
    let mut n = 0;
    for p in w.people.iter_mut().filter(|p| p.stage == Stage::HighSchool && p.class_of == class_of) {
        p.stars = 3 + (n % 3) as u8;
        p.cal.service_read = Some(40.0 + (n * 7 % 50) as f64);
        n += 1;
    }
    assert!(n >= 30, "{n} in the class of {class_of}");
    hunt::ai::day(&mut w, d);
    let v: Vec<&ai::AiHunt> = w.hunt.ai.iter().filter(|h| h.level == ai::COLLEGE && h.year == class_of).collect();
    assert!(!v.is_empty(), "some programs hunt the class of {class_of}");
    let mut teams: Vec<u32> = v.iter().map(|h| h.team).collect();
    teams.sort_unstable();
    teams.dedup();
    assert_eq!(teams.len(), v.len(), "one hunt a program");
    assert!(!teams.contains(&(mine as u32)), "your program's hunt is yours to name");
    for h in &v {
        let p = &w.people[crate::world::scouting::place(&w, h.id).unwrap()];
        assert!(p.class_of == class_of && p.stars >= 3, "{}: {} stars, class of {}", p.name, p.stars, p.class_of);
        assert!(h.why == ai::OWN || ai::known(&w, h), "the whole world sees a generational chase");
    }
    // A second run the same day starts nothing new.
    let n = w.hunt.ai.len();
    hunt::ai::day(&mut w, d);
    assert_eq!(w.hunt.ai.len(), n);
}

#[test]
fn a_club_reads_what_he_costs() {
    let mut w = seat("nfl");
    prospects(&mut w);
    let (_, t) = hunt::seat(&w).unwrap();
    let id = huntable(&w)[0];
    let c = hunt::capital(&w, t, id);
    assert!(c.this_year > 0.0 && c.next_year > 0.0, "{c:?}");
}
