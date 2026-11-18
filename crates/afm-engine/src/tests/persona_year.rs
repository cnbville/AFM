//! The time of year (Cain 2026-11-18, a player who answered a captain's
//! promise on January 1 with "I'd rather see it on the field first": "he's
//! asking for proof while we're in fucking january"). Every talk card says
//! where the man's team is in its year and the next real dates
//! (`talk::year::words`); a player's place out of season is what he saw
//! last, dated, with when it gets set again; a college player sees his
//! transfer window coming.

use super::legacy::seat;
use crate::calendar::{Date, YearDates};
use crate::player::PlayingTime;
use crate::world::folk::persona::Them;
use crate::world::folk::persona::talk::{standing, year};
use crate::world::{Stage, World};

/// Your college program with one player on it, a starter last season, in
/// his second college season.
fn player() -> (World, usize) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let year = w.year;
    let q = &mut w.people[i];
    q.stage = Stage::College;
    q.college = Some(p);
    q.class_of = year - 2;
    q.college_seasons = 2;
    q.last_role = Some(PlayingTime::Starter);
    w.programs[p].roster.push(i);
    (w, i)
}

/// January 1 of the football year the world is in, its season over.
fn new_years_day(w: &mut World) {
    w.today = Date::ymd(w.dates.season + 1, 1, 1);
}

#[test]
fn january_first_is_the_offseason_with_winter_workouts_and_spring_ahead() {
    let (mut w, i) = player();
    new_years_day(&mut w);
    let id = w.people[i].player.id;
    let s = w.dates.season;
    let words = year::words(&w, Them::Player(id));
    assert!(words.contains(&format!("The {s} season is over for the team")), "{words}");
    assert!(words.contains("winter workouts start"), "{words}");
    // The next day on the field is always named: spring practice.
    assert!(words.contains("spring practice opens on March"), "{words}");
    assert!(!words.contains("next game"), "{words}");
    // His own place: last season, dated, then when the next one is set.
    let mine = standing::words(&w, Them::Player(id)).join(". ");
    assert!(mine.contains(&format!("In the {s} season you started")), "{mine}");
    assert!(mine.contains(&format!("Nobody's place for the {} season is set yet", s + 1)), "{mine}");
    assert!(mine.contains("Spring practice, from March"), "{mine}");
    assert!(mine.contains("fall camp, from August"), "{mine}");
    assert!(!mine.contains("this season") && !mine.contains("right now"), "{mine}");
}

#[test]
fn a_college_player_sees_his_transfer_window_coming() {
    let (mut w, i) = player();
    new_years_day(&mut w);
    let id = w.people[i].player.id;
    let (open, close) = crate::world::clock::portal_windows_for(&w, w.dates.season)[0];
    let mine = standing::words(&w, Them::Player(id)).join(". ");
    let words = crate::world::folk::claims::words::date_words;
    assert!(
        mine.contains(&format!("The transfer portal window opens {} and closes {}", words(open), words(close))),
        "{mine}"
    );
    assert!(mine.contains("can't leave until a window opens again"), "{mine}");
    // Open, it says until when.
    w.today = open.add(1);
    let mine = standing::words(&w, Them::Player(id)).join(". ");
    assert!(mine.contains(&format!("is open until {}", words(close))), "{mine}");
    // A man already in the portal, or a month before, isn't told.
    w.today = open.add(-(year::PORTAL_LEAD + 10));
    let mine = standing::words(&w, Them::Player(id)).join(". ");
    assert!(!mine.contains("portal window"), "{mine}");
}

#[test]
fn the_time_of_year_follows_the_calendar_through_spring_and_camp() {
    let (mut w, i) = player();
    let id = w.people[i].player.id;
    let y = YearDates::new(w.dates.season);
    // Spring practice.
    w.today = y.spring_practice.0.add(3);
    let words = year::words(&w, Them::Player(id));
    assert!(words.starts_with("Spring practice"), "{words}");
    assert!(words.contains("the spring game on"), "{words}");
    // Summer: camp is the next day on the field.
    w.today = y.summer_workouts.0.add(7);
    let words = year::words(&w, Them::Player(id));
    assert!(words.starts_with("Summer workouts"), "{words}");
    assert!(words.contains("fall camp opens on"), "{words}");
    let mine = standing::words(&w, Them::Player(id)).join(". ");
    assert!(mine.contains("Fall camp, from"), "{mine}");
}

#[test]
fn staff_and_bosses_live_on_your_teams_year_and_the_nfl_has_its_own() {
    let mut w = seat("college");
    new_years_day(&mut w);
    let boss = Them::Coach(w.career.as_ref().map(|c| c.coach).unwrap_or(0));
    assert!(year::words(&w, boss).contains("winter workouts"), "{}", year::words(&w, boss));
    let mut w = seat("nfl");
    let y = YearDates::new(w.dates.season);
    w.today = y.combine_start.add(-7);
    let any = Them::Agent(0);
    let words = year::words(&w, any);
    assert!(words.contains("the NFL combine") || words.contains("the new league year"), "{words}");
    assert!(words.contains("the offseason program starts") || words.contains("OTAs start"), "{words}");
}
