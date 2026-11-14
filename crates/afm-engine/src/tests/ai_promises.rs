//! AI staffs make and break promises (the promises design, section 10;
//! lane P2). The proofs: a head coach who sells promises more than a
//! careful one and his room decides what; a recruit's promise pulls on its
//! case and goes in the book when he enrolls; a transfer hears a start by
//! his role; the season's record keeps or breaks it, with its trust, the
//! coach's trail name and a burned player; a coach gone lapses it; and the
//! portal window's read holds a man on an open promise and pushes one his
//! head coach broke.

use super::legacy::seat;
use crate::player::PlayingTime;
use crate::positions::Position;
use crate::world::data::Division;
use crate::world::folk::ai_promise::{self, AiPromise, From};
use crate::world::folk::claims::Ear;
use crate::world::folk::{case, pledge, promise};
use crate::world::{Stage, World, college, stay_read};

/// An FBS program that isn't yours, with a head coach.
fn ai_fbs(w: &World) -> usize {
    let mine = w.seat.and_then(|s| s.program);
    w.programs.iter().position(|p| p.division == Division::FBS && Some(p.id) != mine && p.head_coach.is_some()).unwrap()
}

fn coach(w: &mut World, j: usize, recruiting: f64, relationships: f64) -> usize {
    let c = w.programs[j].head_coach.unwrap();
    w.coaches[c].ability.recruiting = recruiting;
    w.coaches[c].ability.relationships = relationships;
    c
}

/// `n` high school seniors at linebacker with `stars`.
fn recruits(w: &mut World, n: usize, stars: u8) -> Vec<usize> {
    w.add_class_aged(17);
    let v: Vec<usize> = (0..w.people.len())
        .filter(|&i| w.people[i].stage == Stage::HighSchool && w.people[i].cal.cases.is_empty())
        .take(n)
        .collect();
    for &i in &v {
        w.people[i].player.position = Position::LB;
        w.people[i].stars = stars;
    }
    v
}

/// `n` linebackers staying on program `j` (sophomores).
fn room_of(w: &mut World, j: usize, n: usize) -> Vec<usize> {
    w.add_class_aged(19);
    let v: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(n).collect();
    for &i in &v {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(j);
        q.college_seasons = 1;
        q.player.position = Position::LB;
        w.programs[j].roster.push(i);
    }
    v
}

/// What the head coach at `j` promises each recruit: (any, starts).
fn promises(w: &mut World, j: usize, v: &[usize]) -> (usize, usize) {
    let (mut any, mut starts) = (0, 0);
    for (n, &i) in v.iter().enumerate() {
        w.people[i].cal.cases.clear();
        w.folk.ai_promises.pending.clear();
        if let Some(k) = ai_promise::recruit(w, i, j, Ear::Decider, 0xA1 ^ n as u64) {
            any += 1;
            starts += usize::from(k == pledge::Kind::Start);
        }
    }
    (any, starts)
}

#[test]
fn a_seller_promises_more_than_a_careful_coach_and_his_room_decides_what() {
    let mut w = seat("college");
    let j = ai_fbs(&w);
    let v = recruits(&mut w, 200, 4);
    coach(&mut w, j, 18.0, 4.0);
    let (seller, seller_starts) = promises(&mut w, j, &v);
    coach(&mut w, j, 4.0, 18.0);
    let (careful, _) = promises(&mut w, j, &v);
    assert!(seller > careful + 40, "seller {seller}, careful {careful}");
    // A promise he made fills the room he reads for the next one.
    coach(&mut w, j, 18.0, 4.0);
    let i = v[0];
    w.people[i].cal.cases.clear();
    w.folk.ai_promises.pending.clear();
    let before = ai_promise::room(&w, i, j);
    let k = (0..50u64).find_map(|s| {
        w.people[i].cal.cases.clear();
        ai_promise::recruit(&mut w, i, j, Ear::Decider, s)
    });
    assert!(k.is_some());
    assert!(ai_promise::room(&w, i, j) < before);
    // An open room reads a start sure; a crowded one doesn't.
    room_of(&mut w, j, 12);
    let (_, crowded_starts) = promises(&mut w, j, &v);
    assert!(seller_starts > crowded_starts + 40, "open {seller_starts}, crowded {crowded_starts}");
}

#[test]
fn a_recruits_promise_pulls_on_its_case_and_goes_in_the_book_when_he_enrolls() {
    let mut w = seat("college");
    let j = ai_fbs(&w);
    let c = coach(&mut w, j, 20.0, 1.0);
    let v = recruits(&mut w, 40, 5);
    let i = *v.iter().find(|&&i| ai_promise::recruit(&mut w, i, j, Ear::Decider, 7).is_some()).unwrap();
    let x = *case::of(&w.people[i], j).unwrap();
    assert!(x.promise != 0 && x.pledge > 0.0 && x.belief > 0.0);
    assert!(case::pull(&w.people[i], j) >= x.pledge as f64);
    // One a school: a second sit-down adds nothing.
    assert!(ai_promise::recruit(&mut w, i, j, Ear::Decider, 8).is_none());
    // Your program never gets an AI promise.
    let mine = w.seat.and_then(|s| s.program).unwrap();
    assert!(ai_promise::recruit(&mut w, i, mine, Ear::Decider, 9).is_none());
    ai_promise::log_recruit(&mut w, i, j);
    let b = w.folk.ai_promises.promises.last().copied().unwrap();
    assert_eq!((b.to, b.prog as usize, b.coach as usize, b.from), (w.people[i].player.id, j, c, From::Recruit));
    assert_eq!(b.kind, x.promise - 1);
    assert!(b.kept.is_none() && b.season >= w.dates.season);
}

#[test]
fn a_transfer_hears_a_start_by_the_role_hed_have() {
    let mut w = seat("college");
    let j = ai_fbs(&w);
    let v = recruits(&mut w, 200, 3);
    let count = |w: &World, role: PlayingTime| {
        v.iter().filter_map(|&i| ai_promise::transfer(w, i, j, role)).filter(|x| x.0 == pledge::Kind::Start).count()
    };
    coach(&mut w, j, 18.0, 4.0);
    let (starter, rotation, backup) =
        (count(&w, PlayingTime::Starter), count(&w, PlayingTime::Rotation), count(&w, PlayingTime::Backup));
    assert!(starter > rotation && rotation > backup && backup == 0, "{starter} {rotation} {backup}");
    // The same pull the portal reads, a role's below a start's.
    let pulls: Vec<f64> =
        v.iter().filter_map(|&i| ai_promise::transfer(&w, i, j, PlayingTime::Backup)).map(|x| x.1).collect();
    assert!(!pulls.is_empty() && pulls.iter().all(|&p| p > 0.0));
}

/// A season on the engine at program `j`: `games` played.
fn season(w: &mut World, j: usize, games: u32) {
    let mut cs = college::new_season(w);
    cs.on_engine = true;
    cs.results[j].wins = games;
    w.college = Some(cs);
}

fn player_at(w: &mut World, j: usize) -> usize {
    let i = room_of(w, j, 1)[0];
    let c = w.programs[j].head_coach.unwrap() as u32;
    let p = &mut w.people[i];
    p.trust = 0.5;
    p.trust_in = c;
    p.mood = 0.0;
    i
}

fn promise_to(w: &mut World, i: usize, j: usize, from: From, kind: pledge::Kind, belief: f32) {
    let season = w.college.as_ref().unwrap().year;
    let p = &w.people[i];
    w.folk.ai_promises.promises.push(AiPromise {
        to: p.player.id,
        prog: j as u16,
        coach: w.programs[j].head_coach.unwrap() as u32,
        from,
        kind: kind as u8,
        pos: p.player.position as u8,
        made: w.today,
        season,
        belief,
        ..Default::default()
    });
}

#[test]
fn the_seasons_record_keeps_or_breaks_it() {
    let mut w = seat("college");
    let j = ai_fbs(&w);
    let c = w.programs[j].head_coach.unwrap() as u32;
    season(&mut w, j, 12);
    assert!(w.college.as_ref().unwrap().year <= w.dates.season);
    let kept = player_at(&mut w, j);
    let broke = player_at(&mut w, j);
    let gone = player_at(&mut w, j);
    for (i, starts) in [(kept, 10), (broke, 4), (gone, 12)] {
        let id = w.people[i].player.id;
        let cs = w.college.as_mut().unwrap();
        cs.starts.insert(id, starts);
        cs.snaps.insert(id, (600, 12));
    }
    promise_to(&mut w, kept, j, From::Transfer, pledge::Kind::Start, 0.5);
    promise_to(&mut w, broke, j, From::Transfer, pledge::Kind::Start, 0.8);
    promise_to(&mut w, gone, j, From::Transfer, pledge::Kind::Start, 0.8);
    // The man who said it to `gone` left: the new head coach isn't bound.
    w.folk.ai_promises.promises.last_mut().unwrap().coach = c + 1;
    let past = promise::past(&w, broke);
    ai_promise::judge(&mut w);
    let book = &w.folk.ai_promises;
    let of = |i: usize| *book.promises.iter().find(|x| x.to == w.people[i].player.id).unwrap();
    assert_eq!(of(kept).kept, Some(true));
    assert_eq!(of(broke).kept, Some(false));
    assert!(of(gone).lapsed && of(gone).kept.is_none());
    // 4 of 12 against 3 of 4: 0.44 of it missed... (1 - (4/12)/0.75).
    assert!((of(broke).missed - (1.0 - (4.0 / 12.0) / 0.75) as f32).abs() < 1e-4);
    // Kept from a half-believer pays 0.15; broken from a believer costs by
    // the share missed.
    assert!((w.people[kept].trust - 0.65).abs() < 1e-4);
    let scale = promise::broken_scale(0.8, 1.0 - (4.0 / 12.0) / 0.75);
    assert!((w.people[broke].trust - (0.5 - 0.22 * scale)).abs() < 1e-4);
    assert!(w.people[broke].mood < 0.0 && w.people[kept].mood > 0.0);
    // The trail heard both (transfers): +0.04 and -0.08 x the scale.
    let name = ai_promise::trail(&w, c as usize);
    assert!((name - (0.04 - 0.08 * scale as f64)).abs() < 1e-4, "{name}");
    // Burned: every later promise is believed a little less.
    assert!(ai_promise::burned(&w, w.people[broke].player.id));
    assert!(promise::past(&w, broke) < past);
    assert!(w.people[broke].notes.iter().any(|n| n.contains("let down")));
    // Judged once a season.
    let before = w.folk.ai_promises.promises.clone();
    let d = w.dates.fbs_regular_end.add(3);
    ai_promise::day(&mut w, d);
    assert_eq!(w.folk.ai_promises.promises, before);
}

#[test]
fn a_freshmans_start_runs_from_his_fourth_game_and_a_role_is_eight_games() {
    let mut w = seat("college");
    let j = ai_fbs(&w);
    season(&mut w, j, 12);
    let start = player_at(&mut w, j);
    let role = player_at(&mut w, j);
    for (i, starts, games) in [(start, 7, 12), (role, 0, 8)] {
        let id = w.people[i].player.id;
        let cs = w.college.as_mut().unwrap();
        cs.starts.insert(id, starts);
        cs.snaps.insert(id, (200, games));
    }
    // 7 of the 9 games from week 4 is past 3 in 4.
    promise_to(&mut w, start, j, From::Recruit, pledge::Kind::Start, 0.6);
    promise_to(&mut w, role, j, From::Recruit, pledge::Kind::Role, 0.6);
    ai_promise::judge(&mut w);
    assert!(w.folk.ai_promises.promises.iter().all(|x| x.kept == Some(true)));
}

#[test]
fn the_window_read_holds_an_open_promise_and_pushes_a_broken_one() {
    let mut w = seat("college");
    let j = ai_fbs(&w);
    season(&mut w, j, 12);
    let open = player_at(&mut w, j);
    let broke = player_at(&mut w, j);
    for &i in &[open, broke] {
        let p = &mut w.people[i];
        p.cal.lean = 0.2;
        p.last_role = Some(PlayingTime::Backup);
    }
    promise_to(&mut w, open, j, From::Stay, pledge::Kind::Start, 0.7);
    promise_to(&mut w, broke, j, From::Stay, pledge::Kind::Start, 0.7);
    w.folk.ai_promises.promises.last_mut().unwrap().kept = Some(false);
    let ctx = stay_read::Ctx::new(&w, w.dates.season);
    let parts = |i: usize| stay_read::read(&w, &ctx, i);
    let has = |v: &[(&str, f64)], why: &str| v.iter().any(|x| x.0 == why);
    assert!(has(&parts(open), "a promise from his head coach"));
    assert!(has(&parts(broke), "his head coach broke a promise"));
    assert!(!has(&parts(broke), "a promise from his head coach"));
}

#[test]
fn the_stay_meeting_promises_a_man_the_coach_reads_as_a_starter() {
    let mut w = seat("college");
    let j = ai_fbs(&w);
    coach(&mut w, j, 20.0, 1.0);
    let v: Vec<usize> = (0..60).map(|_| player_at(&mut w, j)).collect();
    // The men coming back are all well below him, or enough of them well
    // above.
    let starters = crate::league::depth_slots(Position::LB).starters;
    let mut sure = 0;
    let mut behind = 0;
    for &i in &v {
        let me = w.people[i].player.overall();
        let below: Vec<f64> = vec![me - 10.0; 3];
        let above: Vec<f64> = vec![me + 10.0; starters + 2];
        sure += usize::from(ai_promise::stay(&mut w, i, j, &below).is_some());
        w.folk.ai_promises.promises.clear();
        behind += usize::from(ai_promise::stay(&mut w, i, j, &above).is_some());
        w.folk.ai_promises.promises.clear();
    }
    assert!(sure > 30 && behind == 0, "sure {sure}, behind {behind}");
    // A man who holds one is skipped.
    assert!(ai_promise::stay(&mut w, v[0], j, &[0.0]).is_some() || ai_promise::stay(&mut w, v[1], j, &[0.0]).is_some());
    assert!(!ai_promise::open_ids(&w).is_empty());
}
