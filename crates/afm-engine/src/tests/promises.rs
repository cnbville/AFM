//! Promises he judges for himself (the promises design,
//! `afm/promises/proposal.md`; lane P1). The proofs of its section 13:
//! belief spreads by his traits with no one part above a third of the
//! swing; a promise is judged on what he saw, so a chart change undone
//! before practice never shows and two games without it are a slip; a
//! fair condition and a setup on the same player; and the portal meeting's
//! pull by want and role. Plus the three names, keeping and breaking by
//! belief, a follow-up, and the NFL's week 4 check.

use super::legacy::seat;
use crate::calendar::{Date, Level};
use crate::game::stats::{GameStats, StatLine};
use crate::player::{PlayingTime, Trait};
use crate::positions::Position;
use crate::training::seat::{GROUP_ONES, GROUP_TWOS};
use crate::world::folk::promise::{self, By, Cond, Parts, Stat, State, Terms, Verdict};
use crate::world::folk::talk_points::Topic;
use crate::world::folk::talks::{self, PromiseKind, PromiseTo, Reason};
use crate::world::folk::{Name, scene};
use crate::world::seat::SeatGame;
use crate::world::{Stage, World};

/// Your college program and `n` linebackers on it, with a fresh arrival
/// date (they were here for every game).
fn room(n: usize) -> (World, usize, Vec<usize>) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let v: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(n).collect();
    let you = w.career.as_ref().map(|c| c.coach).unwrap() as u32;
    let arrived = w.today.add(-300);
    for &i in &v {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.last_role = Some(PlayingTime::Backup);
        q.player.position = Position::LB;
        q.trust = 0.5;
        q.trust_in = you;
        q.mood = 0.0;
        let id = q.player.id;
        w.programs[p].roster.push(i);
        w.training.arrived.push((id, arrived));
    }
    (w, p, v)
}

fn id(w: &World, i: usize) -> u32 {
    w.people[i].player.id
}

/// A practice week on `d` with these groups.
fn week(w: &mut World, d: Date, groups: &[(u32, u8)]) {
    let mut g = groups.to_vec();
    g.sort_unstable();
    w.training.groups.push((d, g));
}

/// One of your games on `d`: each man's defensive snaps (and tackles).
fn game(w: &mut World, p: usize, d: Date, lines: &[(u32, u32, u32)]) {
    let opp = (0..w.programs.len()).find(|&j| j != p).unwrap();
    let mut stats = GameStats::default();
    for &(who, snaps, tackles) in lines {
        stats.players[0].insert(who, StatLine { snaps_def: snaps, tackles, games: 1, ..Default::default() });
    }
    w.seat_games.push(SeatGame {
        date: d,
        level: Level::College,
        teams: [p, opp],
        names: [w.program_name(p).to_string(), w.program_name(opp).to_string()],
        stats,
        replay: Vec::new(),
        marks: Vec::new(),
        gone: Vec::new(),
        spread: None,
        talks: Default::default(),
        whistle: None,
    });
}

#[test]
fn belief_spreads_by_his_traits_and_no_part_owns_the_swing() {
    let (mut w, _, v) = room(40);
    // A starter ahead of all of them, seen with the ones.
    let lead = v[0];
    let today = w.today;
    let groups: Vec<(u32, u8)> =
        v.iter().map(|&i| (id(&w, i), if i == lead { GROUP_ONES } else { GROUP_TWOS })).collect();
    week(&mut w, today.add(-8), &groups);
    week(&mut w, today.add(-1), &groups);
    for (k, &i) in v.iter().enumerate() {
        let q = &mut w.people[i];
        q.trust = 0.05 + 0.9 * (k as f32 / 39.0);
        q.college_seasons = (k % 5) as u32;
        let ego = 4.0 + (k * 7 % 13) as f64;
        q.player.personality.traits.set(Trait::Ego, ego);
    }
    let parts: Vec<Parts> = v[1..]
        .iter()
        .enumerate()
        .map(|(k, &i)| {
            let t = Terms { dated: k % 3 == 0, by: By::Week4, ..Terms::default() };
            promise::parts_own(&w, i, PromiseKind::Start, &t, if k % 7 == 0 { -1 } else { 0 })
        })
        .collect();
    let beliefs: Vec<f64> = parts.iter().map(|p| p.belief()).collect();
    let lo = beliefs.iter().cloned().fold(1.0, f64::min);
    let hi = beliefs.iter().cloned().fold(0.0, f64::max);
    assert!(lo < 0.35 && hi > 0.65, "a full spread: {lo:.2}-{hi:.2}");
    // No one part owns the swing: by the parts' own ranges, each spans at
    // most a third of the whole (trust and record read through the base
    // they feed, the others by their clamps).
    let spans = [
        ((0.25 + 0.45 + 0.15) / (0.25 + 0.15f64)).ln(),
        ((0.25 + 0.225 + 0.30) / (0.25 + 0.225f64)).ln(),
        (1.0 / 0.3f64).ln(),
        (1.0 / 0.75f64).ln(),
        (1.05 / 0.8f64).ln(),
        (1.2 / 0.5f64).ln(),
    ];
    let total: f64 = spans.iter().sum();
    for (k, x) in spans.iter().enumerate() {
        assert!(*x <= total / 3.0, "part {k} spans {x:.2} of {total:.2}");
    }
    // And in this room, how likely it looks (the part that moves most
    // here) still leaves the rest of the spread to the others.
    let swing = |f: &dyn Fn(&Parts) -> f64| {
        let xs: Vec<f64> = parts.iter().map(f).collect();
        let (a, b) = (xs.iter().cloned().fold(f64::MAX, f64::min), xs.iter().cloned().fold(f64::MIN, f64::max));
        (b / a).ln()
    };
    let all = (hi / lo).ln();
    assert!(swing(&|p: &Parts| p.likely) < all, "likely alone doesn't make the spread");
    assert!(swing(&|p: &Parts| 0.25 + 0.45 * p.trust + 0.15) > 0.3 * all, "his trust carries its share");
}

#[test]
fn a_chart_change_undone_before_practice_never_shows() {
    let (mut w, _, v) = room(2);
    let (a, b) = (id(&w, v[0]), id(&w, v[1]));
    let today = w.today;
    week(&mut w, today.add(-7), &[(a, GROUP_ONES), (b, GROUP_TWOS)]);
    // You bench him and put him back before the week's practice is set.
    talks::when_seen(&mut w, a, Reason::Benched, -0.2);
    week(&mut w, today.add(2), &[(a, GROUP_ONES), (b, GROUP_TWOS)]);
    w.today = today.add(3);
    talks::day(&mut w, today.add(3));
    let moved = |w: &World, who: u32| {
        w.folk.moods.iter().any(|x| x.0 == who && x.1.iter().any(|e| e.1 == Reason::Benched as u8))
    };
    assert!(!moved(&w, a), "undone before practice: he never saw it");
    assert!(w.folk.mine.unseen.is_empty());
    // A change that holds lands when the week's practice shows it.
    talks::when_seen(&mut w, b, Reason::Praise, 0.15);
    talks::when_seen(&mut w, a, Reason::Benched, -0.2);
    week(&mut w, today.add(9), &[(a, GROUP_TWOS), (b, GROUP_ONES)]);
    w.today = today.add(10);
    talks::day(&mut w, today.add(10));
    assert!(moved(&w, a), "he saw it at practice");
}

#[test]
fn a_promise_is_judged_on_the_games_he_saw() {
    let (mut w, p, v) = room(2);
    let (a, b) = (id(&w, v[0]), id(&w, v[1]));
    let today = w.today;
    talks::promise(&mut w, PromiseTo::Player(b), PromiseKind::Start, 0);
    let k = w.folk.promises.len() - 1;
    assert!(w.folk.promises[k].belief > 0.0, "he heard it");
    // Started the first two, then two games on the bench.
    for (n, started) in [true, true, false, false].into_iter().enumerate() {
        let d = today.add(7 * n as i32 + 5);
        let (mine, his) = if started { (60, 10) } else { (8, 62) };
        game(&mut w, p, d, &[(b, mine, 4), (a, his, 5)]);
        w.today = d.add(2);
        promise::week(&mut w, d.add(2));
        let st = State::of(w.folk.promises[k].state);
        assert_eq!(st, if started { State::OnTrack } else { State::Behind }, "game {n}");
    }
    // Behind two straight weeks: he comes to you.
    assert!(
        w.folk.scenes.iter().any(|s| !s.done && s.a == b as u64 && s.title.contains("your promise")),
        "he asks to see you"
    );
    // Half the games: half broken.
    let pr = w.folk.promises[k].clone();
    assert_eq!(promise::judge(&w, &pr), Some(Verdict::Broken(0.5)));
    // Three of four: kept.
    let t0 = w.today;
    game(&mut w, p, t0.add(3), &[(b, 61, 4), (a, 9, 5)]);
    game(&mut w, p, t0.add(10), &[(b, 61, 4), (a, 9, 5)]);
    w.today = w.today.add(12);
    let pr = w.folk.promises[k].clone();
    let r = promise::judge(&w, &pr);
    assert_eq!(r, Some(Verdict::Broken(1.0 - 4.0 / 6.0)), "4 of 6 is short of 3 in 4");
    let t0 = w.today;
    game(&mut w, p, t0.add(3), &[(b, 61, 4), (a, 9, 5)]);
    game(&mut w, p, t0.add(10), &[(b, 61, 4), (a, 9, 5)]);
    w.today = w.today.add(12);
    let pr = w.folk.promises[k].clone();
    assert_eq!(promise::judge(&w, &pr), Some(Verdict::Kept), "6 of 8");
    // Hurt the whole time: it lapses.
    let mut q = pr.clone();
    q.made = w.today.add(1);
    assert_eq!(promise::judge(&w, &q), Some(Verdict::Lapsed));
}

#[test]
fn a_fair_condition_and_a_setup_on_the_same_player() {
    let (mut w, p, v) = room(2);
    let (a, b) = (id(&w, v[0]), id(&w, v[1]));
    let today = w.today;
    week(&mut w, today.add(-8), &[(a, GROUP_ONES), (b, GROUP_TWOS)]);
    week(&mut w, today.add(-1), &[(a, GROUP_ONES), (b, GROUP_TWOS)]);
    // He plays 15 snaps a game behind the starter's 62.
    game(&mut w, p, today.add(-10), &[(a, 62, 7), (b, 15, 2)]);
    game(&mut w, p, today.add(-3), &[(a, 62, 6), (b, 15, 1)]);
    let i = v[1];
    let (fair, high) = promise::bars(&w, i, PromiseKind::Start, Stat::Tackles, By::Week4);
    let mut beliefs = Vec::new();
    for bar in [fair, high] {
        let trust = 0.5;
        w.people[i].trust = trust;
        talks::promise(&mut w, PromiseTo::Player(b), PromiseKind::Start, 0);
        let k = w.folk.promises.len() - 1;
        // Earned on the snaps he gets now (by week 4).
        let t = Terms { by: By::Week4, dated: true, cond: Cond::Stat, stat: Stat::Tackles, bar, ..Terms::default() };
        let got = talks::set_terms(&mut w, k, t);
        let heard = w.folk.promises[k].terms.fair;
        beliefs.push((bar, heard, got, w.people[i].trust));
    }
    let (f, h) = (beliefs[0], beliefs[1]);
    assert_eq!(f.1, 1, "{} tackles a game on 15 snaps is fair: {beliefs:?}", f.0);
    assert_eq!(h.1, -1, "{} on 15 snaps is a setup: {beliefs:?}", h.0);
    assert!(f.2 > h.2, "he believes the fair one more: {beliefs:?}");
    assert!(h.3 < 0.5, "a setup costs trust: {beliefs:?}");
}

#[test]
fn keeping_pays_more_from_a_doubter_and_breaking_costs_by_what_he_counted_on() {
    assert!((promise::kept_trust(0.3) - 0.17).abs() < 1e-6);
    assert!((promise::kept_trust(0.9) - 0.11).abs() < 1e-6);
    assert!((promise::broken_scale(0.0001, 1.0) - 0.4).abs() < 1e-3, "never believed: 40%");
    assert!((promise::broken_scale(1.0, 1.0) - 1.0).abs() < 1e-6, "counted on it: all of it");
    assert!((promise::broken_scale(1.0, 0.5) - 0.5).abs() < 1e-6, "half the games: half");
    // Through the day's check: a contract talk you never held, from a man
    // who half believed it.
    let (mut w, _, v) = room(1);
    let i = v[0];
    let who = id(&w, i);
    talks::promise(&mut w, PromiseTo::Player(who), PromiseKind::Contract, 0);
    let k = w.folk.promises.len() - 1;
    w.folk.promises[k].belief = 0.5;
    w.folk.promises[k].due = w.today;
    w.people[i].trust = 0.6;
    let room_before = w.folk.mine.name(Name::Room);
    let trail_before = w.folk.mine.name(Name::Trail);
    let mut d = w.today;
    while d.weekday() == crate::calendar::Weekday::Mon {
        d = d.add(1);
    }
    w.today = d;
    w.folk.promises[k].due = d;
    talks::day(&mut w, d);
    assert_eq!(w.folk.promises[k].kept, Some(false));
    let scale = 0.4 + 0.6 * 0.5;
    assert!((w.people[i].trust - (0.6 - 0.22 * scale)).abs() < 1e-5, "{}", w.people[i].trust);
    assert!((w.folk.mine.name(Name::Room) - (room_before - 0.08 * scale)).abs() < 1e-5);
    assert_eq!(w.folk.mine.name(Name::Trail), trail_before, "the trail didn't hear it");
}

#[test]
fn your_three_names_move_by_who_hears_and_travel_to_a_new_job() {
    let (mut w, _, _) = room(1);
    w.folk.mine.word(Name::Trail, 0.3);
    w.folk.mine.word(Name::Room, -0.2);
    w.folk.mine.word_all(0.05);
    let m = &w.folk.mine;
    assert!((m.name(Name::Trail) - 0.35).abs() < 1e-6 && (m.name(Name::Room) + 0.15).abs() < 1e-6);
    assert!((m.straight - (0.35 - 0.15 + 0.05) / 3.0).abs() < 1e-6, "the public reads the average");
    // A new college job: its room has heard of you along the trail.
    let carried = w.folk.mine.carried(false);
    assert!((carried[0] - 0.35).abs() < 1e-6 && (carried[2] - 0.05).abs() < 1e-6);
    let nfl = w.folk.mine.carried(true);
    assert!((nfl[0] - 0.05).abs() < 1e-6, "an NFL room hears through the agents");
    // A quarter of the way back a year.
    let mut m = w.folk.mine.clone();
    for _ in 0..52 {
        m.fade_week();
    }
    assert!((m.name(Name::Trail) - 0.35 * 0.75).abs() < 1e-3);
}

#[test]
fn a_promise_in_the_portal_meeting_pulls_by_want_role_and_belief() {
    let (mut w, _, v) = room(3);
    let today = w.today;
    let ids: Vec<u32> = v.iter().map(|&i| id(&w, i)).collect();
    // v[0] starts, v[1] rotates, v[2] sits.
    week(&mut w, today.add(-1), &[(ids[0], GROUP_ONES), (ids[1], GROUP_TWOS), (ids[2], GROUP_TWOS)]);
    let mut rows = Vec::new();
    for (k, &i) in v.iter().enumerate() {
        for first in [true, false] {
            let wn = &mut w.people[i].family.own_wants;
            wn.playing_time = if first { 10.0 } else { 0.1 };
            wn.money = 1.0;
            wn.close_to_home = 1.0;
            wn.academics = 1.0;
            wn.development = 1.0;
            wn.prestige = 1.0;
            w.people[i].last_role = Some([PlayingTime::Starter, PlayingTime::Rotation, PlayingTime::Backup][k]);
            talks::promise(&mut w, PromiseTo::Player(ids[k]), PromiseKind::Start, 0);
            let n = w.folk.promises.len() - 1;
            w.folk.promises[n].belief = 0.8;
            let pull = promise::portal_pull(&w, i, &w.folk.promises[n]);
            rows.push((k, first, pull));
        }
    }
    for r in &rows {
        assert!((0.0..=promise::PULL_CAP).contains(&r.2), "{rows:?}");
    }
    let get = |k: usize, f: bool| rows.iter().find(|r| r.0 == k && r.1 == f).unwrap().2;
    assert!(get(2, true) > get(1, true) && get(1, true) > get(0, true), "a start adds most to a backup: {rows:?}");
    assert!(get(2, true) > 2.0 * get(2, false), "playing time first weighs most: {rows:?}");
    assert!((get(2, true) - 0.5 * 1.0 * 1.0 * 0.8).abs() < 1e-6, "{rows:?}");
}

#[test]
fn he_asks_a_follow_up_and_your_answer_reshapes_the_promise() {
    let (mut w, p, v) = room(2);
    let (a, b) = (id(&w, v[0]), id(&w, v[1]));
    let today = w.today;
    week(&mut w, today.add(-8), &[(a, GROUP_ONES), (b, GROUP_TWOS)]);
    week(&mut w, today.add(-1), &[(a, GROUP_ONES), (b, GROUP_TWOS)]);
    game(&mut w, p, today.add(-3), &[(a, 62, 6), (b, 12, 1)]);
    // A man who barely trusts you: he doubts it.
    w.people[v[1]].trust = 0.2;
    w.people[v[0]].player.personality.traits.set(Trait::Ego, 10.0);
    w.folk.mine.hours = 0.0;
    let sid = talks::start_talk_on(&mut w, b, Topic::Subject(talks::Subject::Role)).unwrap();
    let s = scene::open(&mut w, sid).unwrap();
    let code = s.picks.iter().find(|x| x.mv == "Promise").map(|x| x.code).expect("a promise on the table");
    let s = scene::answer(&mut w, sid, code).unwrap();
    let k = promise::fresh(&w, b).expect("logged");
    let before = w.folk.promises[k].belief;
    assert!((before as f64) < promise::BELIEVES, "he doubts it: {before}");
    let name = s.picks.iter().find(|x| x.code == talks::PAct::Name.code());
    let name = name.unwrap_or_else(|| panic!("he asks ahead of who: {:?}", s.picks));
    assert!(name.label.contains(&w.people[v[0]].name.split(' ').next().unwrap().to_string()), "{}", name.label);
    let _ = scene::answer(&mut w, sid, talks::PAct::Name.code()).unwrap();
    let pr = &w.folk.promises[k];
    assert_eq!(pr.terms.ahead, Some(a), "the man is named");
    assert!(pr.belief > before, "a named man makes it real: {before} -> {}", pr.belief);
}

#[test]
fn a_lie_he_sees_costs_by_how_far_he_believed_you() {
    let (mut w, _, v) = room(1);
    let i = v[0];
    let who = id(&w, i);
    w.people[i].trust = 0.6;
    let today = w.today;
    crate::world::folk::claims::truth::file_lie(
        &mut w,
        crate::world::folk::claims::truth::Lie {
            to: PromiseTo::Player(who),
            fact: crate::world::folk::claims::Stated { kind: "nil".into(), about: Some(who), value: "1".into() },
            said: today,
            sees: today,
            seen: false,
            who: String::new(),
        },
    );
    talks::day(&mut w, today);
    let t = w.people[i].trust;
    assert!(t < 0.6 && t > 0.6 - 0.22, "less than a broken promise to a man who counted on it: {t}");
}

#[test]
fn an_nfl_start_promise_is_judged_on_his_first_four_games() {
    let w = seat("nfl");
    let club = w.seat.and_then(|s| s.nfl_team).unwrap_or(0);
    let i = w.clubs[club].roster[0];
    // No game book this season: his club's chart decides.
    let on_chart = promise::nfl_start_kept(&w, i, club);
    assert!(on_chart.is_some());
}

#[test]
fn a_stat_bar_reads_the_line_he_can_see() {
    let (mut w, p, v) = room(2);
    let b = id(&w, v[0]);
    let today = w.today;
    talks::promise(&mut w, PromiseTo::Player(b), PromiseKind::Start, 0);
    let k = w.folk.promises.len() - 1;
    let t = Terms { cond: Cond::Stat, stat: Stat::Tackles, bar: 5.0, ..Terms::default() };
    talks::set_terms(&mut w, k, t);
    let a = id(&w, v[1]);
    game(&mut w, p, today.add(5), &[(b, 14, 1), (a, 60, 6)]);
    game(&mut w, p, today.add(12), &[(b, 14, 2), (a, 60, 6)]);
    w.today = today.add(14);
    let pr = w.folk.promises[k].clone();
    // He didn't start, and 1.5 a game is short of 5: you're released.
    assert_eq!(promise::state(&w, &pr), State::Missed);
    assert_eq!(promise::judge(&w, &pr), Some(Verdict::Released(0.0)));
    // Started anyway: kept, whatever the bar came to.
    game(&mut w, p, today.add(19), &[(b, 60, 1), (a, 10, 6)]);
    game(&mut w, p, today.add(26), &[(b, 60, 1), (a, 10, 6)]);
    game(&mut w, p, today.add(33), &[(b, 60, 1), (a, 10, 6)]);
    game(&mut w, p, today.add(40), &[(b, 60, 1), (a, 10, 6)]);
    game(&mut w, p, today.add(47), &[(b, 60, 1), (a, 10, 6)]);
    game(&mut w, p, today.add(54), &[(b, 60, 1), (a, 10, 6)]);
    w.today = today.add(56);
    let pr = w.folk.promises[k].clone();
    assert_eq!(promise::judge(&w, &pr), Some(Verdict::Kept));
}

#[test]
fn no_line_says_he_saw_the_chart() {
    // The lines that claim he saw or reads your chart (the promises design,
    // section 12) carry `saw_chart=yes`, a fact no talk sets: they are held.
    let b = crate::world::folk::voice::bank();
    let mut held = 0;
    for (key, _) in b.keys() {
        for (_, _, _, _, when) in b.versions(key) {
            if when.contains(&"saw_chart") {
                held += 1;
            }
        }
    }
    assert!(held >= 14, "held lines: {held}");
    let src = [include_str!("../world/folk/talks.rs"), include_str!("../world/folk/promise.rs")];
    assert!(src.iter().all(|s| !s.contains("set(\"saw_chart\"")), "no talk sets it");
}
