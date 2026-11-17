//! Capital lane C6: high school and NFL facilities (`world/capital/hs.rs`,
//! `nfl.rs`; `docs/design/capital.md`, CP32-CP36).

use crate::calendar::Date;
use crate::world::World;
use crate::world::capital::hs::{self, HsRoom, Package};
use crate::world::capital::nfl::{self, ROOMS_NFL};
use crate::world::capital::rooms::Room;
use crate::world::capital::{self};

/// The tiny world at a seat, with the rooms live.
fn seat(level: &str) -> World {
    let mut w = super::legacy::seat(level);
    // A recorded world past its handover, as a save plays.
    w.recording = true;
    w.options.start_year = w.options.start_year.min(w.year);
    crate::world::finance::ensure(&mut w);
    assert!(capital::live(&w), "the seat's world runs live");
    let d = w.today;
    capital::day(&mut w, d);
    w
}

/// CP33: every school has its five rooms once they live, the typical room
/// is their mean, and each tool reads neutral at the typical room.
#[test]
fn every_school_gets_five_rooms_read_around_the_typical() {
    let mut w = seat("hs");
    assert_eq!(w.capital.hs.len(), w.high_schools.len());
    let n = w.capital.hs.len() as f64;
    for r in HsRoom::ALL {
        let mean = w.capital.hs.iter().map(|c| c.value(r)).sum::<f64>() / n;
        assert!((mean - w.capital.hs_typical[r as usize] as f64).abs() < 1e-5, "{}", r.words());
    }
    // Every school has the everyday rooms; about half have turf.
    for c in &w.capital.hs {
        for r in [HsRoom::Weight, HsRoom::Film, HsRoom::Locker, HsRoom::Stadium] {
            assert!(c.level(r) > 0.0);
        }
    }
    let turf = w.capital.hs.iter().filter(|c| c.level(HsRoom::Turf) > 0.0).count() as f64 / n;
    assert!((0.2..0.8).contains(&turf), "turf at {turf}");
    // At the typical room every tool is neutral.
    let h = w.seat_school().unwrap();
    w.capital.hs[h].level = w.capital.hs_typical.map(|t| t * 5.0);
    assert!(hs::physical_lift(&w.capital, h).abs() < 1e-5);
    assert!(hs::film_lift(&w, h).abs() < 1e-5);
    assert!(hs::mood_lift(&w.capital, h).abs() < 1e-5);
    assert!((hs::crowd_factor(&w, h) - 1.0).abs() < 1e-5);
    // A better weight room coaches the body more; turf cuts the upkeep.
    w.capital.hs[h].level[HsRoom::Weight as usize] = 5.0;
    assert!(hs::physical_lift(&w.capital, h) > 0.0);
    w.capital.hs[h].level[HsRoom::Turf as usize] = 4.0;
    assert!(hs::upkeep_factor(&w, h) < 1.0);
    w.capital.hs_off = true;
    assert_eq!(hs::physical_lift(&w.capital, h), 0.0);
    assert_eq!(hs::crowd_factor(&w, h), 1.0);
}

/// CP33: the booster club's room projects lift their rooms; equipment
/// doesn't.
#[test]
fn the_booster_list_lifts_rooms() {
    let mut w = seat("hs");
    let h = w.seat_school().unwrap();
    w.capital.hs[h].level = [1.0, 1.0, 1.0, 0.0, 2.0];
    assert!(!hs::booster_project(&mut w, h, 0), "speed gear is equipment");
    assert!(hs::booster_project(&mut w, h, 2));
    assert_eq!(w.capital.hs[h].level[HsRoom::Weight as usize], 2.0);
    assert!(hs::booster_project(&mut w, h, 6));
    assert_eq!(w.capital.hs[h].level[HsRoom::Turf as usize], 5.0);
    assert!(hs::booster_project(&mut w, h, 5));
    assert!((w.capital.hs[h].level[HsRoom::Locker as usize] - 1.6).abs() < 1e-5);
}

/// CP32 and CP34: the board's bond goes on a real election day at least 78
/// days out; your town work counts once a week, within ten weeks; election
/// night decides it; a passed stadium opens for every school in the
/// district.
#[test]
fn bond_night_votes_and_the_district_shares_the_stadium() {
    let mut w = seat("hs");
    let h = w.seat_school().unwrap();
    // Give the seat a crosstown rival in its district.
    let rival = (0..w.high_schools.len()).find(|&o| o != h && !w.high_schools[o].private).unwrap();
    w.high_schools[rival].town = w.high_schools[h].town.clone();
    w.high_schools[rival].state = w.high_schools[h].state;
    let st = w.high_schools[h].state;
    if !w.states[st].schools.contains(&rival) {
        w.states[st].schools.push(rival);
    }
    w.high_schools[h].private = false;
    let d = hs::district(&w, h);
    assert!(d.contains(&h) && d.contains(&rival), "{d:?}");
    w.today = Date::ymd(w.year, 1, 10);
    let day = hs::call_for(&mut w, h).expect("the board calls it");
    assert!(w.today.days_to(day) >= hs::ORDER_DAYS);
    assert!(hs::election_days(day.year()).contains(&day), "{}", day.pretty());
    assert!(hs::call_for(&mut w, h).is_err(), "one bond at a time");
    // Too early for the campaign.
    assert!(hs::town_work(&mut w, "rotary").is_err());
    w.today = day.add(-30);
    let before = w.capital.bonds.last().unwrap().work;
    hs::town_work(&mut w, "rotary").expect("a lunch talk");
    assert!(w.capital.bonds.last().unwrap().work > before);
    assert!(hs::town_work(&mut w, "phone_bank").is_err(), "one slot a week");
    w.today = w.today.add(7);
    hs::town_work(&mut w, "forum").expect("the forum");
    w.today = w.today.add(7);
    assert!(hs::town_work(&mut w, "forum").is_err(), "the forum is once");
    // Make it a sure thing and run election night.
    let k = w.capital.bonds.len() - 1;
    w.capital.bonds[k].work = 0.5;
    w.capital.bonds[k].package = Package::New;
    w.capital.hs[h].level[HsRoom::Stadium as usize] = 1.0;
    w.capital.hs[rival].level[HsRoom::Stadium as usize] = 1.0;
    w.today = day;
    hs::day(&mut w, day);
    let b = w.capital.bonds[k].clone();
    assert!(b.passed && b.yes.unwrap() > b.needs, "{b:?}");
    let opens = b.opens.unwrap();
    assert_eq!(opens.year(), day.year() + 2);
    w.today = opens;
    hs::day(&mut w, opens);
    assert!(w.capital.bonds[k].opened);
    for o in [h, rival] {
        assert_eq!(w.capital.hs[o].level[HsRoom::Stadium as usize], 5.0, "CP34: both schools play in it");
    }
}

/// The expected yes share reads the season, the tax bill and the work.
#[test]
fn the_vote_reads_the_season_the_bill_and_the_work() {
    let mut w = seat("hs");
    let h = w.seat_school().unwrap();
    w.today = Date::ymd(w.year, 1, 10);
    hs::call_for(&mut w, h).expect("the board calls it");
    let b = w.capital.bonds.last().unwrap().clone();
    let base = hs::expected_yes(&w, &b);
    let mut dear = b.clone();
    dear.cost *= 4.0;
    assert!(hs::expected_yes(&w, &dear) < base, "a bigger tax bill loses votes");
    let mut worked = b.clone();
    worked.work += 0.05;
    assert!(hs::expected_yes(&w, &worked) > base, "your work wins votes");
    if !w.high_schools[h].seasons.is_empty() {
        let set = |w: &mut World, wins, losses| {
            let s = w.high_schools[h].seasons.last_mut().unwrap();
            s.wins = wins;
            s.losses = losses;
        };
        set(&mut w, 12, 0);
        let won = hs::expected_yes(&w, &b);
        set(&mut w, 0, 10);
        assert!(won > hs::expected_yes(&w, &b), "a winning season sells it");
    }
}

/// CP35: the clubs' rooms start at their building's number; before the
/// rooms live the report card's lines read exactly as before; a room moves
/// its own line.
#[test]
fn club_rooms_move_their_card_lines() {
    let mut w = seat("nfl");
    let t = w.seat.and_then(|s| s.nfl_team).unwrap();
    let base = nfl::CardBase { family: 0.5, am: 0.6, strength: 0.55, med: 0.5, newness: 0.4, facility: 0.7 };
    // Before the rooms live: the old lines.
    w.capital.clubs.clear();
    let old = nfl::card_lines(&w, t, base);
    assert!((old[3] - (0.7 * 0.7 + 0.3 * 0.4)).abs() < 1e-12);
    w.year += 1;
    nfl::year(&mut w);
    assert!(w.capital.clubs[t].set);
    for r in ROOMS_NFL {
        assert!(w.capital.clubs[t].level(r) > 0.0, "{}", nfl::words(r));
    }
    let a = nfl::card_lines(&w, t, base);
    w.capital.clubs[t].level[Room::Locker as usize] = 5.0;
    let b = nfl::card_lines(&w, t, base);
    assert!(b[3] > a[3], "the locker room's line");
    assert_eq!(a[5], b[5], "the weight room's line doesn't move");
    assert!(nfl::card_line(Room::Film).is_none());
    let f = w.capital.clubs[t].number();
    assert!((w.finance.nfl[t].facility as f64 - f).abs() < 0.3);
}

/// CP36: a winning season, the playoffs and other teams wanting you make
/// the ask hard to refuse; a losing one a long shot.
#[test]
fn leverage_follows_the_record_and_the_offers() {
    let mut w = seat("nfl");
    let t = w.seat.and_then(|s| s.nfl_team).unwrap();
    let y = w.year - 1;
    w.nfl_seasons.retain(|s| s.team != t);
    let row = |wins, losses, round| crate::world::nfl_year::NflTeamSeason {
        year: y,
        team: t,
        wins,
        losses,
        ties: 0,
        coach: None,
        round,
    };
    w.nfl_seasons.push(row(3, 14, 0));
    let (lose, words) = nfl::leverage(&w, t);
    assert_eq!(words, "A long shot");
    w.nfl_seasons.retain(|s| s.team != t);
    w.nfl_seasons.push(row(13, 4, 2));
    let (win, _) = nfl::leverage(&w, t);
    assert!(win > lose * 2.0, "{win} against {lose}");
    let other = (t + 1) % 32;
    let today = w.today;
    let c = w.career.as_mut().expect("a career");
    c.market.passed.push((crate::world::coach::Employer::Nfl(other), crate::world::coach::Role::HeadCoach, today));
    assert!(nfl::wanted(&w));
    let (wanted, words) = nfl::leverage(&w, t);
    assert!(wanted > win);
    assert_eq!(words, "Hard to refuse");
}

/// CP35: you ask for one room a league year; a yes is an offseason job
/// done for camp that lifts that room alone.
#[test]
fn an_owner_s_yes_redoes_one_room_for_camp() {
    let mut w = seat("nfl");
    let t = w.seat.and_then(|s| s.nfl_team).unwrap();
    w.year += 1;
    nfl::year(&mut w);
    w.today = Date::ymd(w.year, 3, 1);
    w.capital.clubs[t].level[Room::Weight as usize] = 1.0;
    let before = w.capital.clubs[t].clone();
    let mut said = None;
    // His yes is keyed on the year: look for a year he says yes.
    for _ in 0..30 {
        w.capital.room_asks.clear();
        w.capital.club_builds.clear();
        w.today = Date::ymd(w.year, 3, 1);
        let (yes, words) = nfl::ask_room(&mut w, Room::Weight).expect("asked");
        if yes {
            said = Some(words);
            break;
        }
        assert!(nfl::ask_room(&mut w, Room::Locker).is_err(), "one room a league year");
        w.year += 1;
    }
    let said = said.expect("a yes within thirty years");
    assert!(said.contains("camp"), "{said}");
    let opens = w.capital.club_builds.last().unwrap().opens;
    assert_eq!((opens.month(), opens.year()), (7, w.year));
    w.today = opens;
    nfl::day(&mut w, opens);
    assert_eq!(w.capital.clubs[t].level(Room::Weight), 5.0);
    assert_eq!(w.capital.clubs[t].level(Room::Locker), before.level(Room::Locker));
    assert!(nfl::physical_lift(&w.capital, t) > 0.0);
}
