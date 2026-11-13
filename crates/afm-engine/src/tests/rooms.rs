//! The staff and scouting rooms read the facts (talk facts lane L11;
//! `world/folk/claims/rooms.rs`): your job interview offers only the
//! answers your record backs, the weekly staff meeting carries the game's
//! real numbers and the plan's real move, your staff's rooms speak of what
//! the file shows, the combine names the trouble and the home he really
//! has, and an agent says what his client wants besides money. None of it
//! moves a number: these prove the words and the options.

use super::legacy::seat;
use crate::calendar::Level;
use crate::game::stats::GameStats;
use crate::training::seat::{Note, WeekLog};
use crate::world::background::{Bloodline, FamilyStructure};
use crate::world::coach::{Employer, Role};
use crate::world::data::Division;
use crate::world::folk::claims::rooms::{self, Drafted, Form, Home, Incident, Past, Want, Year};
use crate::world::folk::scene::{self, Kind, Scene};
use crate::world::folk::voice::Facts;
use crate::world::folk::{Team, dialogue, facts, staffing};
use crate::world::program::ProgramSeason;
use crate::world::scouting::lines::room_answer;
use crate::world::seat::SeatGame;
use crate::world::{Stage, World};

fn year(year: i32, wins: u32, losses: u32) -> Year {
    Year { year, wins, losses, team: "Rice".into() }
}

fn you(w: &World) -> usize {
    w.career.as_ref().unwrap().coach
}

/// Seasons you led your program, oldest first.
fn coached(w: &mut World, seasons: &[(i32, u32, u32)]) {
    let p = w.seat.and_then(|s| s.program).unwrap();
    let c = you(w);
    for &(year, wins, losses) in seasons {
        w.programs[p].seasons.push(ProgramSeason {
            year,
            wins,
            losses,
            conference_title: false,
            national_title: false,
            coach: Some(c),
            strength: 0.0,
            totals: Default::default(),
            conference_won: None,
            conference: None,
            division: None,
        });
    }
}

/// A game of your side's in the file: points for, points against.
fn game(w: &mut World, days_ago: i32, score: [i32; 2]) -> usize {
    let (_, mine) = w.seat_team().unwrap();
    let opp = (0..w.programs.len()).find(|&j| j != mine).unwrap();
    let stats = GameStats { score, ..Default::default() };
    w.seat_games.push(SeatGame {
        date: w.today.add(-days_ago),
        level: Level::College,
        teams: [mine, opp],
        names: [w.program_name(mine).to_string(), w.program_name(opp).to_string()],
        stats,
        replay: Vec::new(),
        marks: Vec::new(),
        gone: Vec::new(),
        spread: None,
        talks: Default::default(),
        whistle: None,
    });
    opp
}

/// Said lines by one side (0 narration, 1 you, 2 them).
fn said(s: &Scene, by: u8) -> Vec<String> {
    s.lines.iter().filter(|l| l.by == by).map(|l| l.text.clone()).collect()
}

// ---------------------------------------------------------------------------
// Item 19: your job interview.

#[test]
fn a_record_backs_only_the_answers_it_can() {
    // Nothing on it: no record to point to, no failure to own, no one named.
    let none = Past::default();
    assert!(none.record_words(false).is_none() && !none.strong());
    assert!(none.failure().is_none() && none.people().is_none());
    let mut f = Facts::new();
    none.fill(&mut f);
    for k in ["yours", "strong", "worst", "pick_name", "pick_round", "pick_year"] {
        assert!(f.get(k).is_none(), "an empty record set {k}");
    }
    // Two good years: results to point to, nothing to own.
    let good =
        Past { head: vec![year(2030, 10, 2), year(2031, 9, 3)], wins: 19, losses: 5, as_head: true, ..Past::default() };
    assert!(good.strong() && good.failure().is_none());
    assert!(good.record_words(false).unwrap().starts_with("19-5 over"));
    // A bad year in a good record is something to own.
    let mixed = Past {
        head: vec![year(2030, 10, 2), year(2031, 3, 9)],
        wins: 13,
        losses: 11,
        as_head: true,
        ..Past::default()
    };
    assert_eq!(mixed.failure().as_deref(), Some("the 3-9 season in 2031"));
    assert!(mixed.strong(), "13-11 over two seasons is a winning record: both answers are true");
    // A losing record over enough games has a failure and no results.
    let losing =
        Past { head: vec![year(2030, 5, 7), year(2031, 4, 8)], wins: 9, losses: 15, as_head: true, ..Past::default() };
    assert!(!losing.strong() && losing.failure().is_some());
    // Five games are a few results, not a record: neither strong nor weak.
    let few = Past { head: vec![year(2031, 5, 0)], wins: 5, losses: 0, as_head: true, ..Past::default() };
    assert!(!few.strong());
    let (mut f, mut g) = (Facts::new(), Facts::new());
    few.fill(&mut f);
    losing.fill(&mut g);
    assert!(f.get("strong").is_none() && f.get("weak").is_none(), "a short record is neither");
    assert_eq!(g.get("weak"), Some("yes"));
    let even = Past { head: vec![year(2031, 6, 6)], wins: 6, losses: 6, as_head: true, ..Past::default() };
    let mut h = Facts::new();
    even.fill(&mut h);
    assert!(h.get("strong").is_none() && h.get("weak").is_none(), "an even record is neither");
    // Never a head coach: the staffs you were on, and the worst stretch.
    let staff = Past { wins: 12, losses: 20, stretch: Some((4, 12, "Rice".into())), ..Past::default() };
    assert_eq!(staff.failure().as_deref(), Some("the 4-12 stretch at Rice"));
    let said_to_you = staff.record_words(false).unwrap();
    let said_by_you = staff.record_words(true).unwrap();
    assert!(said_to_you.contains("you've") && said_by_you.contains("I've"), "{said_to_you} / {said_by_you}");
    // A drafted player of yours is a name to bring.
    let named = Past { drafted: vec![Drafted { name: "Jay Ross".into(), round: 2, year: 2030 }], ..Past::default() };
    let mut f = Facts::new();
    named.fill(&mut f);
    assert_eq!(f.get("pick_name"), Some("Jay Ross"));
    assert_eq!(f.get("pick_round"), Some("second"));
    assert_eq!(f.get("pick_year"), Some("2030"));
}

#[test]
fn the_committee_reads_your_seasons_in_the_world() {
    let mut w = seat("college");
    assert!(rooms::past(&w).head.is_empty(), "a new seat has led nothing yet");
    coached(&mut w, &[(2022, 10, 2), (2023, 9, 3), (2024, 3, 9)]);
    let p = rooms::past(&w);
    assert_eq!(p.head.len(), 3);
    assert_eq!((p.wins, p.losses, p.as_head), (22, 14, true));
    assert!(p.strong());
    assert_eq!(p.failure().as_deref(), Some("the 3-9 season in 2024"));
}

/// Opens an interview for another program's head job and answers its first
/// two questions: what is left on the table is the third.
fn third_question(w: &mut World) -> (u32, Scene) {
    let p = w.seat.and_then(|s| s.program).unwrap();
    let other = (0..w.programs.len()).find(|&j| j != p && w.programs[j].division == Division::FBS).unwrap();
    let on = w.today.add(10);
    dialogue::interview_set(w, Employer::College(other), Role::HeadCoach, Some(on)).expect("a head job interviews");
    let id = w.folk.scenes.last().unwrap().id;
    let s = scene::open(w, id).unwrap();
    assert!(!s.done, "the panel sat down");
    let codes: Vec<u32> = s.picks.iter().map(|x| x.code).collect();
    assert_eq!(codes, [931, 932, 933], "the first question keeps its three answers");
    scene::answer(w, id, 931).unwrap();
    (id, scene::answer(w, id, 934).unwrap())
}

#[test]
fn a_first_time_coach_has_a_plain_answer_for_his_past() {
    let mut w = seat("college");
    let (id, s) = third_question(&mut w);
    let codes: Vec<u32> = s.picks.iter().map(|x| x.code).collect();
    assert_eq!(codes, [dialogue::PLAIN_ANSWER], "no results, no failure, no one drafted: one plain answer");
    let s = scene::answer(&mut w, id, dialogue::PLAIN_ANSWER).unwrap();
    assert!(s.done && !s.outcome.is_empty(), "the interview ends with a verdict");
}

#[test]
fn an_even_record_is_neither_results_nor_a_failure_and_does_not_say_no_record() {
    let mut w = seat("college");
    coached(&mut w, &[(2024, 6, 6)]);
    let (_, s) = third_question(&mut w);
    assert_eq!(s.picks.iter().map(|x| x.code).collect::<Vec<_>>(), [dialogue::PLAIN_ANSWER]);
    let plain = &s.picks[0];
    assert_eq!(plain.mv, "Judge the plan", "a coach who has led a program has a record");
    assert!(!plain.label.contains("haven't run"), "{}", plain.label);
}

#[test]
fn the_interview_offers_the_answers_your_record_backs_and_names_it() {
    let mut w = seat("college");
    coached(&mut w, &[(2022, 10, 2), (2023, 9, 3), (2024, 3, 9)]);
    let (_, s) = third_question(&mut w);
    let codes: Vec<u32> = s.picks.iter().map(|x| x.code).collect();
    assert_eq!(codes, [937, 938], "a failure to own and results to point to; no one drafted");
    let fail = s.picks.iter().find(|x| x.code == 937).unwrap();
    assert!(fail.label.contains("the 3-9 season in 2024"), "{}", fail.label);
    let results = s.picks.iter().find(|x| x.code == 938).unwrap();
    assert!(results.label.contains("22-14 over three seasons"), "{}", results.label);
    // The panel's question names the same number.
    let ask = said(&s, 2).pop().unwrap();
    assert!(ask.contains("22-14"), "the question about your past carries your record: {ask}");
}

#[test]
fn a_losing_record_cannot_point_to_results_and_the_panel_answers_a_failure_by_name() {
    let mut w = seat("college");
    coached(&mut w, &[(2024, 3, 9)]);
    let (id, s) = third_question(&mut w);
    let codes: Vec<u32> = s.picks.iter().map(|x| x.code).collect();
    assert_eq!(codes, [937], "12 games and a losing record: only the failure is true");
    let before = said(&s, 2).len();
    let s = scene::answer(&mut w, id, 937).unwrap();
    let new = said(&s, 2).split_off(before);
    assert!(
        new.iter().any(|t| t.contains("the 3-9 season in 2024") || t.contains("The 3-9 season in 2024")),
        "the panel answers the failure by its name: {new:?}"
    );
}

#[test]
fn the_chair_you_apply_for_is_named_with_its_last_season() {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    let other = (0..w.programs.len()).find(|&j| j != p && w.programs[j].division == Division::FBS).unwrap();
    w.programs[other].seasons.push(ProgramSeason {
        year: w.year - 1,
        wins: 4,
        losses: 8,
        conference_title: false,
        national_title: false,
        coach: None,
        strength: 0.0,
        totals: Default::default(),
        conference_won: None,
        conference: None,
        division: None,
    });
    let on = w.today.add(10);
    dialogue::interview_set(&mut w, Employer::College(other), Role::HeadCoach, Some(on)).unwrap();
    let id = w.folk.scenes.last().unwrap().id;
    let s = scene::open(&mut w, id).unwrap();
    let ask = said(&s, 2).pop().unwrap();
    assert!(ask.contains("4-8"), "the first question names their last record: {ask}");
}

// ---------------------------------------------------------------------------
// Item 19: the weekly staff meeting.

/// A meeting the morning after a game with the numbers set: they were
/// pressured on 6 of 40 dropbacks (15%) and had 7 big plays, scored 34;
/// you ran 20 times for 60 yards (3.0) on 60 plays and scored 13.
fn meeting(w: &mut World, kind: u64, with_game: bool) -> (u32, Scene) {
    let (_, mine) = w.seat_team().unwrap();
    if with_game {
        let opp = game(w, 1, [13, 34]);
        let g = w.seat_games.last_mut().unwrap();
        g.stats.team[0].plays = 60;
        g.stats.team[0].dropbacks = 40;
        g.stats.team[0].rush_yards = 60.0;
        g.stats.team[1].dropbacks = 40;
        g.stats.team[1].pressures = 6;
        g.stats.team[1].explosive = 7;
        assert_ne!(opp, mine);
    }
    let team = Team::seat(w).unwrap();
    let c = facts::assistants(w, team).first().map_or(you(w), |x| x.0);
    let mut s = Scene::new(Kind::StaffMeeting, w.today, "Staff meeting: your coordinator wants a change");
    s.a = c as u64;
    s.b = kind;
    let id = scene::add(w, s);
    (id, scene::open(w, id).unwrap())
}

/// (from, to) in "goes from 30% to 38%".
fn moves(hint: &str) -> (i64, i64) {
    let rest = hint.split("from ").nth(1).unwrap_or_else(|| panic!("no move in {hint}"));
    let (from, rest) = rest.split_once("% to ").unwrap_or_else(|| panic!("no move in {hint}"));
    let to: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    (from.parse().unwrap(), to.parse().unwrap())
}

#[test]
fn a_coordinators_case_carries_the_games_numbers_and_the_plans_real_move() {
    for kind in 0..4u64 {
        let mut w = seat("college");
        let (id, s) = meeting(&mut w, kind, true);
        // The case names the game, in numbers the box score has.
        let case = said(&s, 2).pop().unwrap();
        let (opp, pct, ypc) = (w.seat_games[0].names[1].clone(), "15", "3.0");
        let numbers = match kind {
            0 | 1 => ["34", pct, "7", opp.as_str()],
            _ => ["13", ypc, "33", opp.as_str()],
        };
        assert!(numbers.iter().any(|n| case.contains(n)), "kind {kind}: the case is plain, not the game's: {case}");
        // The numbers beside it are the box score's.
        assert_eq!(s.docs.len(), 1, "kind {kind}: the game's numbers sit beside the case");
        assert_eq!(s.docs[0].title, format!("Lost to {opp} 13-34"), "the table names the game");
        let rows = &s.docs[0].rows;
        assert_eq!(rows.len(), 3);
        if kind <= 1 {
            assert_eq!(rows[2].cells[0], "Explosive plays they had");
            assert_eq!(rows[0].cells[1], "34");
            assert_eq!(rows[1].cells[1], "15%");
            assert_eq!(rows[2].cells[1], "7");
        } else {
            assert_eq!(rows[0].cells[1], "13");
            assert_eq!(rows[1].cells[1], "3.0");
            assert_eq!(rows[2].cells[1], "33%");
        }
        // The options and their codes are today's three, then your own case
        // instead (live talks lane 6).
        let codes: Vec<u32> = s.picks.iter().map(|x| x.code).collect();
        let own: Vec<u32> =
            crate::world::folk::dialogue::own_moves(kind as u8).into_iter().map(|m| 954 + m as u32).collect();
        assert_eq!(codes[..3], [951, 952, 953]);
        assert_eq!(codes[3..], own[..]);
        // "Agree" says where the plan goes, and agreeing takes it there.
        let agree = s.picks.iter().find(|x| x.code == 951).unwrap().hint.clone();
        let half = s.picks.iter().find(|x| x.code == 952).unwrap().hint.clone();
        let (from, to) = moves(&agree);
        let (from_h, to_h) = moves(&half);
        assert_eq!(from, from_h, "both start at the plan's number now");
        assert!(
            (2 * (to_h - from) - (to - from)).abs() <= 1,
            "kind {kind}: halfway is half of {from} to {to}, not {to_h}"
        );
        scene::answer(&mut w, id, 951).unwrap();
        let g = &w.plans.guide;
        let now = match kind {
            0 => g.pressure.to,
            1 => g.man.to,
            _ => g.run.to,
        }
        .expect("agreeing set the guideline");
        assert_eq!((100.0 * now).round() as i64, to, "kind {kind}: the plan went where the option said");
    }
}

#[test]
fn halfway_goes_halfway_to_the_number_it_names() {
    let mut w = seat("college");
    let (id, s) = meeting(&mut w, 2, true);
    let (_, to) = moves(&s.picks.iter().find(|x| x.code == 952).unwrap().hint.clone());
    scene::answer(&mut w, id, 952).unwrap();
    let now = w.plans.guide.run.to.expect("halfway set the guideline");
    assert_eq!((100.0 * now).round() as i64, to);
}

#[test]
fn a_meeting_without_its_game_keeps_the_plain_case() {
    let mut w = seat("college");
    let (_, s) = meeting(&mut w, 0, false);
    assert!(s.docs.is_empty(), "no game on file, no numbers beside the case");
    let codes: Vec<u32> = s.picks.iter().map(|x| x.code).collect();
    assert_eq!(codes, [951, 952, 953, 955, 958, 959]);
    assert!(!said(&s, 2).pop().unwrap_or_default().is_empty(), "he still makes his case");
}

// ---------------------------------------------------------------------------
// Item 23: your staff.

/// Four games on file, with `allowed` points against each.
fn games(w: &mut World, allowed: i32) {
    // Oldest first, as the file keeps them.
    for k in (0..4).rev() {
        game(w, 8 + 7 * k, [28, allowed]);
    }
}

/// Your staff's first assistant, made your defensive coordinator.
fn coordinator(w: &mut World) -> usize {
    let team = Team::seat(w).unwrap();
    let (c, _) = facts::assistants(w, team).into_iter().next().expect("the program has a staff");
    w.coaches[c].job.as_mut().unwrap().role = Role::DefensiveCoordinator;
    c
}

#[test]
fn a_coordinators_year_is_good_or_poor_by_the_points_against_his_unit() {
    for (allowed, form) in [(7, Form::Good), (45, Form::Poor), (28, Form::Middling)] {
        let mut w = seat("college");
        let c = coordinator(&mut w);
        games(&mut w, allowed);
        let u = rooms::unit(&w, c).unwrap();
        assert_eq!(u.form, Some(form), "{allowed} points against");
        assert_eq!(u.poor(), form == Form::Poor);
        let y = u.year_words().expect("a defense with four games on file has a year");
        assert!(y.contains(&format!("gave up {allowed} a game over 4 games")), "{y}");
    }
    // Under three games there is no read.
    let mut w = seat("college");
    let c = coordinator(&mut w);
    game(&mut w, 15, [28, 7]);
    game(&mut w, 8, [28, 7]);
    let u = rooms::unit(&w, c).unwrap();
    assert!(u.form.is_none() && u.points.is_none() && u.year_words().is_none());
}

#[test]
fn his_record_and_unit_reach_a_lines_facts() {
    let mut w = seat("college");
    let c = coordinator(&mut w);
    // An assistant's own wins and losses are never credited: his record with
    // you is the team's since he took the job.
    w.coaches[c].job.as_mut().unwrap().first = w.year - 2;
    let p = w.seat.and_then(|s| s.program).unwrap();
    let mine = you(&w);
    for (y, wins, losses) in [(w.year - 3, 1, 11), (w.year - 2, 7, 5), (w.year - 1, 6, 6)] {
        w.programs[p].seasons.push(ProgramSeason {
            year: y,
            wins,
            losses,
            conference_title: false,
            national_title: false,
            coach: Some(mine),
            strength: 0.0,
            totals: Default::default(),
            conference_won: None,
            conference: None,
            division: None,
        });
    }
    games(&mut w, 7);
    let mut f = Facts::new();
    rooms::unit(&w, c).unwrap().fill(&mut f);
    assert_eq!(f.get("record"), Some("13-11"), "the two seasons since he took the job, not the one before");
    assert_eq!(f.get("form"), Some("good"));
    assert!(f.get("unit").is_some_and(|u| u.contains("gave up 7 a game")));
}

#[test]
fn a_good_unit_takes_the_blame_off_the_firing_and_a_poor_one_leaves_it() {
    for (allowed, good) in [(7, true), (45, false)] {
        let mut w = seat("college");
        let c = coordinator(&mut w);
        games(&mut w, allowed);
        let id = staffing::start_firing(&mut w, c).unwrap();
        let s = scene::open(&mut w, id).unwrap();
        let codes: Vec<u32> = s.picks.iter().map(|x| x.code).collect();
        for k in [70, 71, 73] {
            assert!(codes.contains(&k), "{allowed} against: option {k} is still there");
        }
        assert_eq!(codes.contains(&72), !good, "{allowed} against: blame his unit only when it did badly");
        let straight = s.picks.iter().find(|x| x.code == 70).unwrap();
        assert_eq!(straight.label.contains("not the results"), good, "{}", straight.label);
        assert!(
            said(&s, 0).iter().any(|t| t.contains(&format!("gave up {allowed} a game"))),
            "the narration names his unit's year: {:?}",
            said(&s, 0)
        );
    }
}

/// Two players of your age group, by index.
fn two_players(w: &mut World) -> (usize, usize) {
    w.add_class_aged(19);
    let v: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(2).collect();
    (v[0], v[1])
}

fn proposal(w: &mut World, kind: i64, a: u64, b: u64) -> Scene {
    let team = Team::seat(w).unwrap();
    let mut s = Scene::new(Kind::Proposal, w.today, "A proposal");
    s.stops = false;
    s.team = Some(team);
    s.a = a;
    s.b = b;
    s.c = kind;
    let id = scene::add(w, s);
    scene::open(w, id).unwrap()
}

#[test]
fn the_trainer_says_only_what_is_true_of_the_man() {
    let mut w = seat("college");
    let (i, _) = two_players(&mut w);
    let id = w.people[i].player.id;
    let age = w.people[i].player.age;
    let s = proposal(&mut w, 2, 0, id as u64);
    let t = s.lines.iter().find(|l| l.name == "Head trainer").unwrap().text.clone();
    assert!(!t.contains("hamstring"), "an injury nobody wrote up: {t}");
    assert!(t.contains(&format!("He's {age}")), "his real age: {t}");
    assert!(t.ends_with("A week off now saves us a month later."));
    // When the trainers wrote up his load, that is the reason.
    w.training.notes.push(Note { date: w.today, id, dept: "trainers".into(), text: "His load is up.".into() });
    let s = proposal(&mut w, 2, 0, id as u64);
    let t = s.lines.iter().find(|l| l.name == "Head trainer").unwrap().text.clone();
    assert!(t.contains("workload"), "{t}");
}

#[test]
fn a_young_player_is_pitched_with_the_practice_that_backs_him_or_does_not() {
    let mut w = seat("college");
    let (young, vet) = two_players(&mut w);
    let (yid, vid) = (w.people[young].player.id, w.people[vet].player.id);
    let b = (yid as u64) << 32 | vid as u64;
    // The log pairs an offensive man with a defensive one: each of the two
    // faces somebody else, so it is their own records that are compared.
    let (off, def) = (900_001u32, 900_002u32);
    let today = w.today;
    let log = |days_ago: i32, battles: Vec<(u32, u32, u8, u8)>| WeekLog {
        monday: today.add(-days_ago),
        block: None,
        week: Default::default(),
        snaps: 0,
        injuries: 0,
        fitted: Vec::new(),
        best: Vec::new(),
        worst: Vec::new(),
        battles,
    };
    let pitch = |s: &Scene| s.lines.iter().find(|l| l.by == 2).unwrap().text.clone();
    // No one-on-ones: the pitch has nothing to prove with.
    let s = proposal(&mut w, 1, 0, b);
    assert!(!pitch(&s).contains("one-on-ones"), "{}", pitch(&s));
    assert!(said(&s, 0).iter().all(|t| !t.contains("doesn't back him")));
    // He is 3 of 4 and the veteran 1 of 4: the coach says so.
    w.training.log.push(log(7, vec![(yid, off, 3, 1), (vid, def, 1, 3)]));
    let s = proposal(&mut w, 1, 0, b);
    assert!(pitch(&s).contains("He is 3 of 4 in one-on-ones lately"), "{}", pitch(&s));
    assert!(said(&s, 0).iter().all(|t| !t.contains("doesn't back him")));
    // The other way round: the narration says the log does not back him.
    w.training.log.clear();
    w.training.log.push(log(7, vec![(yid, off, 1, 3), (vid, def, 3, 1)]));
    let s = proposal(&mut w, 1, 0, b);
    assert!(!pitch(&s).contains("one-on-ones"), "{}", pitch(&s));
    let n = said(&s, 0).into_iter().find(|t| t.contains("doesn't back him")).expect("the log is against him");
    assert!(n.contains("3 of 4") && n.contains("1 of 4"), "{n}");
    // Too few reps, or reps from long ago, prove nothing.
    w.training.log.clear();
    w.training.log.push(log(7, vec![(yid, off, 1, 0), (vid, def, 0, 1)]));
    w.training.log.push(log(60, vec![(yid, off, 3, 1), (vid, def, 1, 3)]));
    let s = proposal(&mut w, 1, 0, b);
    assert!(!pitch(&s).contains("one-on-ones") && said(&s, 0).iter().all(|t| !t.contains("doesn't back him")));
}

#[test]
fn no_staff_room_claims_what_the_file_cannot_back() {
    // Words a room invented before this lane (an injury and a bowl practice
    // nobody had) stay out of the trees that now read the file.
    let staffing = include_str!("../world/folk/staffing.rs");
    let takeover = include_str!("../world/folk/takeover.rs");
    assert!(!staffing.contains("hamstring is tight"), "the trainer's invented injury is back");
    assert!(!takeover.contains("you watch him in bowl practice"), "the holdover's invented bowl practice is back");
}

// ---------------------------------------------------------------------------
// Item 24: the combine and visit rooms.

#[test]
fn a_player_names_the_trouble_he_really_had() {
    let dui = Incident { kind: "a DUI arrest".into(), age: 20 };
    for band in 0..3 {
        for key in 0..8u64 {
            let t = room_answer("incident", band, key, 3, Some(&dui), None).unwrap();
            assert!(t.contains("a DUI arrest"), "band {band}: his answer names it: {t}");
        }
    }
    let strong: Vec<String> = (0..8u64).filter_map(|k| room_answer("incident", 2, k, 3, Some(&dui), None)).collect();
    assert!(strong.iter().any(|t| t.contains("I was 20")), "a strong answer names his age then: {strong:?}");
    // His story names nothing: the caller says the plain answer.
    assert!(room_answer("incident", 1, 1, 3, None, None).is_none());
    assert!(room_answer("whiteboard", 1, 1, 3, Some(&dui), None).is_none(), "only the trouble and the home");
}

#[test]
fn a_player_names_the_home_he_really_has() {
    let nfl = Home {
        structure: FamilyStructure::Grandparents,
        bloodline: Bloodline::NflPlayer,
        brother: None,
        parents_absent: false,
    };
    let t = room_answer("family", 2, 1, 4, None, Some(&nfl)).unwrap();
    assert!(t.contains("grandparents"), "{t}");
    assert!(t.contains("My dad played in the league"), "{t}");
    // A poor answer says less than he has.
    let t = room_answer("family", 0, 1, 4, None, Some(&nfl)).unwrap();
    assert!(!t.contains("My dad"), "{t}");
    // A brother at a school, and whether he means to follow him.
    let follows = Home {
        structure: FamilyStructure::TwoParents,
        bloodline: Bloodline::None,
        brother: Some(("Ohio State".into(), true)),
        parents_absent: false,
    };
    let t = room_answer("family", 2, 1, 4, None, Some(&follows)).unwrap();
    assert!(t.contains("My brother plays at Ohio State, and I want to follow him."), "{t}");
    let own = Home { brother: Some(("Ohio State".into(), false)), ..follows };
    let t = room_answer("family", 2, 1, 4, None, Some(&own)).unwrap();
    assert!(t.contains("make my own name"), "{t}");
    // No home on file: the plain answer.
    assert!(room_answer("family", 2, 1, 4, None, None).is_none());
}

#[test]
fn a_home_that_is_just_the_two_of_them_has_no_brother_and_absent_parents_were_not_at_every_game() {
    let brother = Some(("Ohio State".to_string(), true));
    let single = Home {
        structure: FamilyStructure::SingleParent,
        bloodline: Bloodline::None,
        brother: brother.clone(),
        parents_absent: false,
    };
    for key in 0..16u64 {
        for band in [1, 2] {
            let t = room_answer("family", band, key, 4, None, Some(&single)).unwrap();
            let alone = t.contains("two of us") || t.contains("me and the parent");
            assert!(
                !(alone && t.contains("My brother")),
                "an answer that is only the two of them names a brother: {t}"
            );
        }
    }
    let away = Home { parents_absent: true, ..single };
    for key in 0..16u64 {
        for band in 0..3 {
            let t = room_answer("family", band, key, 4, None, Some(&away)).unwrap();
            for said in
                ["every game", "never missed", "coming to the draft", "They'll be there", "My brother", "opinions"]
            {
                assert!(!t.contains(said), "{said:?} from a man whose parents are not around his football: {t}");
            }
        }
    }
}

#[test]
fn a_declared_man_names_no_brother_to_follow() {
    let mut w = seat("college");
    let (i, _) = two_players(&mut w);
    w.people[i].family.older_brother_at = Some(0);
    w.people[i].family.follows_brother = true;
    assert!(rooms::home(&w, i).unwrap().brother.is_some(), "a high school player names the brother he may follow");
    w.people[i].stage = Stage::Declared;
    assert!(rooms::home(&w, i).unwrap().brother.is_none(), "a declared man is past choosing a school");
}

#[test]
fn the_rooms_read_his_story_for_the_trouble() {
    let mut w = seat("college");
    let (i, j) = two_players(&mut w);
    assert!(rooms::incident(&w, i).is_none());
    w.people[i].note("Off-field trouble at 19: a failed drug test.".to_string());
    w.people[i].note("Off-field trouble at 20: a DUI arrest.".to_string());
    let t = rooms::incident(&w, i).expect("his story names it");
    assert_eq!((t.kind.as_str(), t.age), ("a DUI arrest", 20), "the latest one");
    assert!(rooms::incident(&w, j).is_none(), "another man's story is his own");
    assert!(rooms::home(&w, i).is_some());
}

// ---------------------------------------------------------------------------
// Item 25: the agents.

#[test]
fn an_agent_names_what_pulls_his_client_besides_money() {
    let mut w = seat("nfl");
    let club = w.seat.and_then(|s| s.nfl_team).unwrap();
    let mut with_want = 0;
    let mut said_it = 0;
    for i in w.clubs[club].roster.clone() {
        let Some(want) = rooms::top_want(&w, i, false) else { continue };
        with_want += 1;
        assert_ne!(want, Want::Loyalty, "loyalty counts only in a talk with his own club");
        if w.people[i].pro.as_ref().and_then(|c| c.agent).is_none() {
            continue;
        }
        let got = dialogue::agent_wants(&mut w, i, club, false);
        if let Some((key, text)) = got {
            said_it += 1;
            assert_eq!(key, want.key());
            assert!(text.starts_with('"') && text.ends_with('"'), "an agent's words are quoted: {text}");
            assert!(!text.contains(['{', '}']), "{text}");
        } else {
            assert!(
                matches!(want, Want::Winning),
                "{} has a {} want and his agent said nothing",
                w.people[i].name,
                want.key()
            );
        }
    }
    assert!(with_want > 0, "somebody on the roster wants something besides money");
    assert!(said_it > 0, "an agent said it");
}

#[test]
fn nobody_wants_what_he_does_not_much_want() {
    let mut w = seat("nfl");
    let club = w.seat.and_then(|s| s.nfl_team).unwrap();
    let mut none = 0;
    for i in w.clubs[club].roster.clone() {
        if rooms::top_want(&w, i, true).is_none() {
            none += 1;
            assert!(dialogue::agent_wants(&mut w, i, club, true).is_none(), "an agent had nothing to say");
        }
    }
    assert!(none > 0, "somebody is only after the money");
}

// ---------------------------------------------------------------------------
// Hidden numbers stay hidden.

#[test]
fn a_units_form_reaches_a_line_as_a_word_never_a_number() {
    for (allowed, word) in [(7, "good"), (45, "poor")] {
        let mut w = seat("college");
        let c = coordinator(&mut w);
        games(&mut w, allowed);
        let mut f = Facts::new();
        rooms::unit(&w, c).unwrap().fill(&mut f);
        assert_eq!(f.get("form"), Some(word));
    }
    let w = seat("college");
    let mut f = Facts::new();
    rooms::chair(&w, Team::seat(&w).unwrap(), &mut f);
    assert!(matches!(f.get("form"), None | Some("up" | "down")));
}
