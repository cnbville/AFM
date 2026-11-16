//! Ground lane G3: your week (`world/ground/week/`): the beat, the
//! coordinator's plan, road days by the rule book, trips on the week's
//! money, your calls and the calls back, the night calls, the ledger, the
//! May list and the week off.

use crate::calendar::{Date, Moment, Weekday};
use crate::world::ground::areas;
use crate::world::ground::people::{self, Who};
use crate::world::ground::standing;
use crate::world::ground::week::{self, DoneKind, beat, phone, plan, road};
use crate::world::scouting::readers::COACH;
use crate::world::scouting::{Kind, dept};
use crate::world::{Stage, World};

/// The tiny college seat with recruits: pros made into high school juniors
/// and seniors at the schools of one state (the one with the most coached
/// schools), with homes there.
fn college() -> (World, usize, usize) {
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
    assert!(!schools.is_empty(), "a state with coached schools");
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
    (w, prog, st)
}

/// Plays your seat's day the way the clock does for Ground: the week first,
/// then the department's work.
fn run(w: &mut World, from: Date, days: i32) {
    for k in 0..days {
        let d = from.add(k);
        w.today = d;
        w.played = 0;
        week::day(w, d);
        dept::day(w, d);
    }
}

/// The first Monday of the spring road.
fn spring_monday(w: &World) -> Date {
    let (a, _) = beat::spring_road(&w.dates).expect("a spring road");
    a.on_or_after(Weekday::Mon)
}

/// Sets the clock to a local minute of today.
fn at(w: &mut World, minute: u16) {
    let e = w.seat_zone().to_eastern(Moment::at(w.today, minute / 60, minute % 60));
    let (day, m) = crate::calendar::zone::world_day(e);
    assert_eq!(day, w.today);
    w.played = m + 1;
}

#[test]
fn every_week_has_its_beat_from_the_years_calendar() {
    let (w, _, _) = college();
    let first = Date::ymd(2026, 1, 1).on_or_before(Weekday::Mon);
    let weeks: Vec<beat::Beat> = (0..35).map(|k| beat::beat(&w, first.add(7 * k))).collect();
    assert_eq!(weeks[0].week, 1);
    assert_eq!(weeks[34].week, 35);
    assert!(weeks.iter().all(|b| !b.beat.is_empty() && !b.stake.is_empty()), "no week is empty");
    // Week 16, 2026: the road opens Wednesday the 15th, the spring game is
    // Saturday the 18th (months.md's sheet).
    let w16 = &weeks[15];
    assert_eq!(w16.monday, Date::ymd(2026, 4, 13));
    assert_eq!(w16.line(), "Week 16: the road opens Wednesday, the spring game is Saturday.");
    assert_eq!(w16.period, "Contact from Apr 15");
    // Signing day is week 6's beat, the May list week 22's.
    assert!(weeks[5].beat.starts_with("Signing day is Wednesday"), "{}", weeks[5].beat);
    assert!(weeks[5].on.iter().any(|x| x.0 == Date::ymd(2026, 2, 4)));
    assert!(weeks[21].on.iter().any(|x| x.1 == "The May list" && x.0 == Date::ymd(2026, 5, 26)));
    assert!(weeks[23].beat.contains("camp"), "{}", weeks[23].beat);
    assert!(weeks[0].beat.starts_with("Ground waits"), "the takeover's weeks: {}", weeks[0].beat);
    // 2027 follows its own calendar: the January window is ten days.
    let b27 = beat::beat(&w, Date::ymd(2027, 1, 4));
    assert!(b27.on.iter().any(|x| x.1 == "The portal opens" && x.0 == Date::ymd(2027, 1, 4)), "{:?}", b27.on);
}

#[test]
fn the_coordinator_drafts_the_spring_road_and_the_days_come_off_the_140() {
    let (mut w, prog, _) = college();
    let mon = spring_monday(&w);
    run(&mut w, mon, 1);
    let roads = w.ground.week.roads.clone();
    assert!(!roads.is_empty(), "men out in the spring");
    let hc = w.programs[prog].head_coach.unwrap() as u32;
    assert!(roads.iter().all(|r| r.man != hc), "the FBS head coach stays off the spring road");
    assert!(roads.iter().all(|r| r.days & 1 == 0), "Monday is the meeting");
    assert!(roads.iter().all(|r| !r.why.is_empty()));
    let letter = plan::letter_for(&w, mon).expect("the Monday letter").clone();
    assert!(letter[0].starts_with(&format!("Week {}:", beat::week_number(mon))), "{letter:?}");
    assert!(letter.iter().any(|l| l.contains("road days left this spring")), "{letter:?}");
    assert!(letter.iter().any(|l| l.starts_with("The plan:")), "{letter:?}");
    // The week plays: every man-day out is one of the 140.
    let before = w.scouting.dept.eval_days.2;
    run(&mut w, mon.add(1), 6);
    let used = w.scouting.dept.eval_days.2 - before;
    let planned: u32 = roads.iter().map(|r| r.days.count_ones()).sum();
    assert!(used > 0 && used as u32 <= planned, "{used} days used of {planned} planned");
    // Each man's looks are at schools in his area, on his days.
    for r in &roads {
        let code = COACH + r.man;
        for (id, v) in &w.scouting.dept.looks {
            for l in
                v.iter().filter(|l| l.who == code && l.kind == Kind::Practice && l.date > mon && l.date < mon.add(7))
            {
                let i = crate::world::scouting::place(&w, *id).unwrap();
                assert_eq!(w.high_schools[w.people[i].hs].ground_area, r.area, "his area");
                assert!(r.days & (1 << mon.days_to(l.date)) != 0, "on his days");
            }
        }
    }
    // He calls in that night from the road, 7 to 10 PM.
    let nights: Vec<_> = w.ground.week.log.iter().filter(|x| x.kind == DoneKind::NightCall).collect();
    assert_eq!(nights.len(), used as usize, "one night call a man a day out");
    assert!(nights.iter().all(|x| (19 * 60..22 * 60).contains(&x.minute) && x.text.contains('"')));
    // The morning map tells you what changed overnight.
    run(&mut w, mon.add(7), 1);
    assert_eq!(w.ground.week.overnight_day, Some(mon.add(7)));
    let next = plan::letter_for(&w, mon.add(7)).unwrap();
    assert!(next.iter().any(|l| l.starts_with("Last week bought:")), "{next:?}");
}

#[test]
fn you_spend_the_days_yourself_inside_the_rules() {
    let (mut w, prog, st) = college();
    let mon = spring_monday(&w);
    w.today = mon;
    week::day(&mut w, mon);
    let men = road::men(&w, prog);
    let hc = w.programs[prog].head_coach.unwrap();
    let a = men.iter().copied().find(|&m| m != hc).unwrap();
    let area = *areas::in_state(st)
        .iter()
        .max_by_key(|&&x| w.high_schools.iter().filter(|h| h.ground_area == x && h.coach.is_some()).count())
        .unwrap();
    let said = plan::set_road(&mut w, a as u32, area, &[1, 2]).expect("Tuesday and Wednesday");
    assert!(said.contains(areas::area(area).name), "{said}");
    assert!(plan::set_road(&mut w, hc as u32, area, &[2]).unwrap_err().contains("FBS head coach"));
    assert!(plan::set_road(&mut w, a as u32, area, &[5]).is_err(), "no school visits on a Saturday");
    // No more men out at once than the rules allow.
    let cap = dept::off_campus_recruiters(&w.programs[prog], mon).unwrap() as usize;
    let out = w.ground.week.roads.iter().filter(|r| r.days & (1 << 2) != 0).count();
    assert!(out <= cap);
    run(&mut w, mon.add(1), 2);
    let code = COACH + a as u32;
    let mine: Vec<u32> = w
        .scouting
        .dept
        .looks
        .iter()
        .filter(|(_, v)| v.iter().any(|l| l.who == code && l.kind == Kind::Practice && l.date > mon))
        .map(|(&id, _)| id)
        .collect();
    assert!(!mine.is_empty(), "he went where you sent him");
    for id in mine {
        let i = crate::world::scouting::place(&w, id).unwrap();
        assert_eq!(w.high_schools[w.people[i].hs].ground_area, area);
    }
    assert!(plan::clear_road(&mut w, a as u32).is_ok());
}

#[test]
fn a_flight_comes_out_of_the_weeks_money() {
    let (mut w, prog, _) = college();
    let mon = spring_monday(&w);
    w.today = mon;
    week::day(&mut w, mon);
    w.ground.week.roads.clear();
    let far =
        (0..areas::table().len() as u16).find(|&a| areas::trip_to_area(&w, prog, a).flight()).expect("a far area");
    let p = road::purse(&w, prog, &[]);
    let hc = w.programs[prog].head_coach.unwrap();
    let men: Vec<usize> = road::men(&w, prog).into_iter().filter(|&m| m != hc).collect();
    let mut flown = 0;
    for &m in &men {
        match plan::set_road(&mut w, m as u32, far, &[1, 2]) {
            Ok(_) => flown += 1,
            Err(e) => {
                assert!(e.contains("this week's money covers"), "{e}");
                break;
            }
        }
    }
    assert_eq!(flown as u32, p.flights.min(men.len() as u32), "as many flights as the money covers");
    assert!(w.ground.week.roads.iter().all(|r| r.flight));
    assert!(
        road::trip_cost(areas::Trip::Fly { miles: 900.0, hours: 4.5 }, 2)
            > road::trip_cost(areas::Trip::Drive { miles: 100.0, hours: 1.8 }, 2)
    );
}

#[test]
fn your_call_counts_by_your_name_and_a_message_waits_for_his_hour() {
    let (mut w, prog, _) = college();
    let mon = spring_monday(&w);
    run(&mut w, mon, 1);
    let you = w.career.as_ref().unwrap().coach;
    // A coach whose school has a kid on your board.
    let h = w.people.iter().find(|p| p.stage == Stage::HighSchool && p.stars >= 3).map(|p| p.hs).unwrap();
    let who = people::head_coach_of(&w, h).unwrap();
    let hour = people::makeup(&w, who).hour;
    // Before school he can't come to the phone: you leave a message.
    at(&mut w, 7 * 60);
    let said = phone::call(&mut w, who).unwrap();
    assert!(said.contains("left a message") && said.contains(&phone::clock(hour)), "{said}");
    let cb = w.ground.week.phone.last().cloned().unwrap();
    assert!(cb.matters && cb.state == 0 && cb.date == mon && cb.minute == hour);
    // It shows on the in-flight list and stops the day at his minute.
    assert!(
        phone::in_flight(&w).iter().any(|f| f.what.contains("calling back") && f.lands.contains("planning period"))
    );
    assert!(w.live_moments(mon).iter().any(|m| m.what.starts_with("Call:")), "a live moment");
    assert!(phone::take(&mut w, cb.id).is_err(), "not before he calls");
    at(&mut w, hour);
    let before = standing::points(&w, who, crate::world::ground::Holder::Coach(you as u32));
    phone::take(&mut w, cb.id).unwrap();
    let after = standing::points(&w, who, crate::world::ground::Holder::Coach(you as u32));
    assert!(after > before, "your call counts: {before} to {after}");
    // Two calls on the ledger, out of the week's free hours, the same ones
    // film uses.
    let t = crate::world::scouting::film::time::used(&w) - crate::world::scouting::film::time::film_used(&w);
    assert!((t - 0.5).abs() < 1e-4, "{t}");
    // A message nobody answers: a coach who barely knows you doesn't call again.
    let h2 = w.people.iter().filter(|p| p.stage == Stage::HighSchool && p.stars >= 3).map(|p| p.hs).find(|&x| x != h);
    if let Some(h2) = h2
        && let Some(who2) = people::head_coach_of(&w, h2)
        && standing::level(&w, prog, who2) < standing::Level::Trusted
    {
        at(&mut w, 7 * 60);
        phone::call(&mut w, who2).unwrap();
        run(&mut w, mon.add(1), 1);
        assert!(w.ground.week.log.iter().any(|x| x.kind == DoneKind::Missed && x.who == Some(who2)));
    }
}

#[test]
fn a_night_call_that_matters_stops_your_evening() {
    let (mut w, prog, _) = college();
    let mon = spring_monday(&w);
    run(&mut w, mon, 1);
    // A 4-star nobody of yours has seen in person, at a school a man visits.
    let r = w.ground.week.roads.iter().find(|r| r.days & 2 != 0).cloned().expect("a man out Tuesday");
    let i = (0..w.people.len())
        .find(|&i| w.people[i].stage == Stage::HighSchool && w.high_schools[w.people[i].hs].ground_area == r.area)
        .unwrap();
    w.people[i].stars = 5;
    let h = w.people[i].hs as u32;
    w.ground.week.roads.iter_mut().find(|x| x.man == r.man).unwrap().focus = vec![h];
    run(&mut w, mon.add(1), 1);
    let night = w
        .ground
        .week
        .log
        .iter()
        .find(|x| x.kind == DoneKind::NightCall && x.man == Some(r.man) && x.date == mon.add(1))
        .cloned()
        .expect("his night call");
    assert!(night.matters, "{}", night.text);
    let nights = |w: &World| w.live_moments(mon.add(1)).iter().filter(|m| m.what.starts_with("Night call")).count();
    let before = nights(&w);
    assert!(before >= 1);
    phone::hear(&mut w, night.id).unwrap();
    assert_eq!(nights(&w), before - 1, "heard, it stops nothing");
    let _ = prog;
}

#[test]
fn what_you_host_goes_on_the_ledger_and_tires_you() {
    let (mut w, prog, _) = college();
    let sat = w.dates.junior_days.0.on_or_after(Weekday::Sat);
    let ids: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(5).collect();
    for &i in &ids {
        w.people[i].cal.visits.push((prog, sat, crate::world::Visit::Unofficial));
    }
    run(&mut w, sat, 1);
    let h = w.ground.week.hours_on(sat);
    assert!((h - 8.0).abs() < 1e-4, "a junior day is your Saturday: {h}");
    assert!(crate::world::scouting::film::time::today(&w) >= 8.0, "the day's hours tire your eye");
    assert_eq!(week::chosen_hours(&w), 0.0, "what you host doesn't take your film hours");
}

#[test]
fn the_may_list_comes_the_day_after_memorial_day() {
    let (mut w, prog, _) = college();
    let mon = spring_monday(&w);
    run(&mut w, mon, 12);
    let may = beat::may_list_day(w.dates.season);
    run(&mut w, may.add(-1), 2);
    let m = w.ground.week.may.clone();
    assert_eq!(m.date, Some(may));
    assert!(!m.lists.is_empty(), "each man's list from the road");
    assert!(m.spots > 0, "open official visit spots");
    assert!(
        m.ranked.iter().all(|id| {
            let i = crate::world::scouting::place(&w, *id).unwrap();
            w.people[i].class_of == w.dates.season + 1
        }),
        "rising seniors only"
    );
    assert!(w.career.as_ref().unwrap().desk.mails.iter().any(|x| x.subject == "The May list"));
    let _ = prog;
}

#[test]
fn a_week_off_in_july_rests_you_and_your_coordinator_holds_the_phones() {
    let (mut w, prog, _) = college();
    let jul = Date::ymd(w.dates.season, 7, 6).on_or_after(Weekday::Mon);
    let apr = Date::ymd(w.dates.season, 4, 20);
    w.today = apr;
    assert!(week::take_off(&mut w, apr, 1).is_err(), "not in the spring");
    w.today = jul.add(-3);
    w.scouting.film.wear = 0.7;
    week::take_off(&mut w, jul, 1).unwrap();
    let h = w.people.iter().find(|p| p.stage == Stage::HighSchool && p.stars >= 3).map(|p| p.hs).unwrap();
    let who: Who = people::head_coach_of(&w, h).unwrap();
    w.ground.week.phone.push(week::CallBack {
        id: 999,
        who,
        date: jul.add(1),
        minute: 13 * 60,
        matters: true,
        state: 0,
        tries: 0,
    });
    run(&mut w, jul, 2);
    assert_eq!(w.scouting.film.wear, 0.0, "your tiredness goes back to zero");
    assert!(
        crate::world::scouting::film::time::can_watch(&w, crate::world::scouting::film::session::Mode::Quick).is_err()
    );
    assert!(phone::call(&mut w, who).is_err(), "your coordinator has the phones");
    assert_eq!(w.ground.week.phone.iter().find(|c| c.id == 999).unwrap().state, 2, "he took the call, as his");
    let _ = prog;
}

#[test]
fn no_seat_no_week() {
    let mut w = World::empty(crate::world::WorldOptions {
        scale: 0.001,
        history_years: 0,
        start_year: 2037,
        ..Default::default()
    });
    let d = w.today;
    week::day(&mut w, d);
    assert!(w.ground.week.monday.is_none() && w.ground.week.log.is_empty(), "the world's own programs have no week");
}
