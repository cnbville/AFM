//! A pro's role as he saw it (code oddity 29): his last season's
//! role written from the games he played, never from his college years or
//! the club's chart; the promise ranks read what he sees; a kicker's games
//! say his place; your games keep the side you coached; another league's
//! seat judges promises on its own games and calendar; your club's games
//! keep who was out hurt; promises left at a job you leave close; a man
//! you cut or traded was let down; and an NFL club's start promise counts
//! the starts he saw.

use super::legacy::seat;
use crate::calendar::{Date, Level};
use crate::game::stats::{GameStats, StatLine};
use crate::player::PlayingTime;
use crate::positions::Position;
use crate::training::seat::GROUP_TWOS;
use crate::world::folk::claims::side_of;
use crate::world::folk::talks::{self, PromiseKind, PromiseTo};
use crate::world::folk::{Team, promise, seen};
use crate::world::pro::other::OtherSeason;
use crate::world::pro::{ProSeason, games, role};
use crate::world::seat::SeatGame;
use crate::world::{Seat, Stage, World};

/// Your NFL club and one of its men at `pos`, with no pro seasons yet.
fn club_man(pos: Position) -> (World, usize, usize) {
    let mut w = seat("nfl");
    let club = w.seat.and_then(|s| s.nfl_team).unwrap();
    let i = w.clubs[club].roster[0];
    let p = &mut w.people[i];
    p.player.position = pos;
    p.pro.as_mut().unwrap().seasons.clear();
    p.pro.as_mut().unwrap().other.clear();
    (w, club, i)
}

/// A regular season row: games, listed starts and snaps on offense and
/// defense.
fn nfl_row(w: &mut World, i: usize, year: i32, games: u32, starts: u32, snaps: [u32; 2]) {
    let c = w.people[i].pro.as_mut().unwrap();
    c.seasons.retain(|s| s.year != year);
    c.seasons.push(ProSeason { year, games, starts, snaps: [snaps[0], snaps[1], 40], ..Default::default() });
    c.seasons.sort_by_key(|s| s.year);
}

fn kept_game(w: &mut World, d: Date, teams: [usize; 2], lines: &[(u32, StatLine)], mine: Option<u8>) {
    let mut stats = GameStats::default();
    for (who, l) in lines {
        stats.players[0].insert(*who, l.clone());
    }
    w.seat_games.push(SeatGame {
        date: d,
        level: Level::Nfl,
        teams,
        names: ["Home".into(), "Away".into()],
        stats,
        replay: Vec::new(),
        marks: Vec::new(),
        gone: Vec::new(),
        spread: None,
        talks: Default::default(),
        whistle: None,
        mine,
        hurt: Vec::new(),
    });
}

#[test]
fn a_pros_last_season_is_the_one_he_played() {
    let (mut w, _, i) = club_man(Position::LB);
    let y = w.year - 1;
    // A rookie keeps the role his college games gave him.
    w.people[i].last_role = Some(PlayingTime::Starter);
    assert_eq!(role::season_role(&w, i), None);
    // His snaps against a full starter's season, never the club's listed
    // starts (a starter by the chart who played a fifth of the snaps is
    // the rotation; a listed backup on for most of them the starter).
    nfl_row(&mut w, i, y, 16, 16, [0, 300]);
    assert_eq!(role::season_role(&w, i), Some(PlayingTime::Rotation));
    nfl_row(&mut w, i, y, 16, 0, [0, 640]);
    assert_eq!(role::season_role(&w, i), Some(PlayingTime::Starter));
    nfl_row(&mut w, i, y, 16, 0, [0, 90]);
    assert_eq!(role::season_role(&w, i), Some(PlayingTime::Backup));
    // His offense snaps say nothing about a linebacker.
    nfl_row(&mut w, i, y, 16, 0, [900, 90]);
    assert_eq!(role::season_role(&w, i), Some(PlayingTime::Backup));
    // A season on the stand-in keeps no snaps: its starts are all it has.
    nfl_row(&mut w, i, y, 16, 12, [0, 0]);
    w.people[i].pro.as_mut().unwrap().seasons.last_mut().unwrap().snaps = [0; 3];
    w.options.engine_from = Some(y + 1);
    assert_eq!(role::season_role(&w, i), Some(PlayingTime::Starter));
    w.options.engine_from = None;
    assert_eq!(role::season_role(&w, i), Some(PlayingTime::Backup), "on the engine, no snaps is no role");
    // A year on the practice squad without a game.
    nfl_row(&mut w, i, y, 0, 0, [0, 0]);
    w.people[i].pro.as_mut().unwrap().seasons.last_mut().unwrap().ps_weeks = 17;
    assert_eq!(role::season_role(&w, i), Some(PlayingTime::PracticeSquad));
    // A year on the roster, healthy, who never dressed: a backup.
    nfl_row(&mut w, i, y, 0, 0, [0, 0]);
    w.people[i].pro.as_mut().unwrap().seasons.last_mut().unwrap().full_pay_weeks = 17;
    assert_eq!(role::season_role(&w, i), Some(PlayingTime::Backup));
    // A year on injured reserve says nothing new: the year before stands.
    nfl_row(&mut w, i, y - 1, 16, 0, [0, 640]);
    w.people[i].pro.as_mut().unwrap().seasons.last_mut().unwrap().ir_weeks = 17;
    assert_eq!(role::season_role(&w, i), Some(PlayingTime::Starter));
    // A newer season in another league: by its starts (its rows keep no
    // snaps).
    let league = crate::world::pro::other::CFL;
    let team = crate::world::pro::other::teams_in(league, y + 1)[0] as u16;
    let row = OtherSeason { year: y + 1, league, team, games: 14, starts: 12, ..Default::default() };
    w.people[i].pro.as_mut().unwrap().other.push(row);
    assert_eq!(role::season_role(&w, i), Some(PlayingTime::Starter));
    w.people[i].pro.as_mut().unwrap().other[0].starts = 1;
    assert_eq!(role::season_role(&w, i), Some(PlayingTime::Backup));
    // An NFL season the same year or later is his newest.
    nfl_row(&mut w, i, y + 1, 16, 0, [0, 640]);
    assert_eq!(role::season_role(&w, i), Some(PlayingTime::Starter));
}

#[test]
fn a_kicker_who_dressed_is_the_starter() {
    let (mut w, _, i) = club_man(Position::K);
    let y = w.year - 1;
    nfl_row(&mut w, i, y, 17, 0, [0, 0]);
    w.people[i].pro.as_mut().unwrap().seasons[0].snaps[2] = 70;
    assert_eq!(role::season_role(&w, i), Some(PlayingTime::Starter));
}

#[test]
fn the_role_is_written_at_the_years_end_and_once_for_an_old_save() {
    let (mut w, _, i) = club_man(Position::CB);
    let y = w.year - 1;
    w.people[i].last_role = Some(PlayingTime::Starter);
    nfl_row(&mut w, i, y, 16, 16, [0, 120]);
    role::write_all(&mut w);
    assert_eq!(w.people[i].last_role, Some(PlayingTime::Backup), "the year's end writes it");
    // A save from before has his college role: it is written when it loads.
    w.people[i].last_role = Some(PlayingTime::Starter);
    w.pro_role_rev = 0;
    w.after_load();
    assert_eq!(w.people[i].last_role, Some(PlayingTime::Backup), "written on load");
    assert_eq!(w.pro_role_rev, role::PRO_ROLE_REV);
    // Not a pro: his college role stays his.
    w.people[i].stage = Stage::College;
    w.people[i].last_role = Some(PlayingTime::Starter);
    role::write_all(&mut w);
    assert_eq!(w.people[i].last_role, Some(PlayingTime::Starter));
}

#[test]
fn a_college_starter_who_sits_now_is_not_promised_to_keep_his_job() {
    let (mut w, club, i) = club_man(Position::WR);
    let id = w.people[i].player.id;
    w.people[i].last_role = Some(PlayingTime::Starter);
    let today = w.today;
    w.training.arrived.push((id, today.add(-300)));
    // Two practice weeks with the twos and two games on for a few snaps.
    for d in [today.add(-8), today.add(-1)] {
        w.training.groups.push((d, vec![(id, GROUP_TWOS)]));
    }
    let opp = (club + 1) % w.clubs.len();
    let top = 7;
    for d in [today.add(-9), today.add(-2)] {
        let lines = [
            (id, StatLine { snaps_off: 6, games: 1, ..Default::default() }),
            (top, StatLine { snaps_off: 64, games: 1, ..Default::default() }),
        ];
        kept_game(&mut w, d, [club, opp], &lines, Some(0));
    }
    assert!(!seen::starter(&w, i));
    assert_eq!(promise::place_rank(&w, i), PlayingTime::Backup as u8, "his place now, not his college role");
    // A man on for half the snaps has a real role.
    for g in w.seat_games.iter_mut() {
        g.stats.players[0].get_mut(&id).unwrap().snaps_off = 30;
    }
    assert_eq!(promise::place_rank(&w, i), 1);
}

#[test]
fn a_kickers_games_say_his_place() {
    let (mut w, club, i) = club_man(Position::P);
    let id = w.people[i].player.id;
    let today = w.today;
    w.training.arrived.push((id, today.add(-300)));
    w.people[i].last_role = None;
    let opp = (club + 1) % w.clubs.len();
    for d in [today.add(-9), today.add(-2)] {
        let lines = [
            (id, StatLine { snaps_st: 9, games: 1, ..Default::default() }),
            (7, StatLine { snaps_off: 64, snaps_def: 60, games: 1, ..Default::default() }),
        ];
        kept_game(&mut w, d, [club, opp], &lines, Some(0));
    }
    assert_eq!(seen::first_now(&w, i), Some(true), "he punted in both");
    assert!(seen::starter(&w, i));
}

#[test]
fn a_kept_game_is_the_side_you_coached() {
    let (mut w, club, _) = club_man(Position::LB);
    let opp = (club + 1) % w.clubs.len();
    let today = w.today;
    // You coached the away side, your old club, against your club now.
    kept_game(&mut w, today, [club, opp], &[], Some(1));
    let g = w.seat_games.last().unwrap().clone();
    assert_eq!(side_of(&g, Team::Nfl(opp)), Some(1));
    assert_eq!(side_of(&g, Team::Nfl(club)), None, "not this club's game for you");
    // A game kept before the side was: either side that is your team.
    let old = SeatGame { mine: None, ..g };
    assert_eq!(side_of(&old, Team::Nfl(club)), Some(0));
    assert_eq!(side_of(&old, Team::College(club)), None, "another level's team");
}

#[test]
fn another_leagues_seat_judges_its_own_games_on_its_own_calendar() {
    let mut w = seat("nfl");
    let league = crate::world::pro::other::CFL;
    let year = w.today.year();
    let teams = crate::world::pro::other::teams_in(league, year);
    let (f, opp) = (teams[0], teams[1]);
    w.seat = Some(Seat { level: Level::Other, nfl_team: None, program: None, other: Some(f) });
    let mut g = SeatGame {
        date: w.today,
        level: Level::Other,
        teams: [opp, f],
        names: ["Home".into(), "Away".into()],
        stats: GameStats::default(),
        replay: Vec::new(),
        marks: Vec::new(),
        gone: Vec::new(),
        spread: None,
        talks: Default::default(),
        whistle: None,
        mine: Some(1),
        hurt: Vec::new(),
    };
    assert_eq!(side_of(&g, Team::Other(f)), Some(1), "its games are yours to read");
    g.level = Level::Nfl;
    assert_eq!(side_of(&g, Team::Other(f)), None, "an NFL game with the same index is not");
    // A promise there is due at the end of the league's next season, and
    // a Week 4 promise opens on its fourth week.
    let (start, _, end) = (year..=year + 2)
        .filter_map(|y| crate::world::pro::other::season_span(league, y))
        .find(|s| s.2 > w.today.add(14))
        .unwrap();
    let i = (0..w.people.len()).find(|&k| w.people[k].stage == Stage::Pro).unwrap();
    let id = w.people[i].player.id;
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Snaps, 0);
    assert_eq!(w.folk.promises.last().unwrap().due, end, "the league's calendar, not Black Monday");
    let opens = promise::opens(&w, w.today, promise::By::Week4);
    assert!(opens == start.add(21) || opens > start.add(21), "its own fourth week: {opens:?} vs {start:?}");
}

#[test]
fn your_clubs_games_keep_who_was_out_hurt() {
    let (mut w, club, i) = club_man(Position::LB);
    let id = w.people[i].player.id;
    let opp = (club + 1) % w.clubs.len();
    // A teammate already out at kickoff, on his own days.
    let j = w.clubs[club].roster[1];
    let jid = w.people[j].player.id;
    w.people[j].pro.as_mut().unwrap().out_days = 10;
    // One of yours who plays on hurt is not out.
    let k = w.clubs[club].roster[2];
    let kid = w.people[k].player.id;
    for x in [i, k] {
        let c = w.people[x].pro.as_mut().unwrap();
        c.out_days = 0;
        c.out_weeks = 0;
    }
    let inj = |who: u32, plays_on: bool| crate::game::sim::Injury {
        side: 0,
        p: 0,
        id: who,
        kind: crate::game::sim::InjuryKind::AnkleSprain,
        weeks: 3.0,
        plays_on,
    };
    let other_id = w.people[w.clubs[opp].roster[0]].player.id;
    let list = vec![(1usize, inj(id, false)), (1usize, inj(kid, true)), (0usize, inj(other_id, false))];
    let today = w.today;
    let hurt_before = w.training.hurt.len();
    kept_game(&mut w, today, [opp, club], &[], Some(1));
    games::seat_hurt(&mut w, Level::Nfl, [opp, club], &list);
    let g = w.seat_games.last().unwrap().clone();
    assert!(g.hurt.contains(&id), "hurt in it");
    assert!(g.hurt.contains(&jid), "out at kickoff");
    assert!(!g.hurt.contains(&kid), "played on");
    assert!(!g.hurt.contains(&other_id), "theirs are not yours");
    assert_eq!(w.training.hurt.len(), hurt_before, "a pro's injuries stay off your trainers' list");
    // The game he missed hurt says nothing about your word.
    let c = w.people[j].pro.as_mut().unwrap();
    c.out_days = 0;
    c.out_weeks = 0;
    w.training.arrived.push((jid, today.add(-300)));
    let seen = seen::games(&w, jid, today, today, None);
    assert_eq!(seen.len(), 1);
    assert!(seen[0].hurt && seen[0].line.is_none());
    // A game you didn't play changes nothing.
    let third = (club + 2) % w.clubs.len();
    games::seat_hurt(&mut w, Level::Nfl, [opp, third], &[(0usize, inj(kid, false))]);
    assert_eq!(w.seat_games.last().unwrap().hurt, g.hurt);
}

#[test]
fn promises_made_at_a_job_you_leave_close() {
    let (mut w, club, i) = club_man(Position::LB);
    let id = w.people[i].player.id;
    let new = (club + 1) % w.clubs.len();
    let at = |w: &mut World, t: usize| {
        w.seat = Some(Seat { level: Level::Nfl, nfl_team: Some(t), program: None, other: None });
    };
    let open = |w: &mut World| {
        talks::promise(w, PromiseTo::Player(id), PromiseKind::Start, 0);
        talks::promise(w, PromiseTo::Player(id), PromiseKind::Stay, 0);
        let n = w.folk.promises.len();
        (n - 2, n - 1)
    };
    // A promotion at the same club keeps them.
    let (start, stay) = open(&mut w);
    assert_eq!(w.folk.promises[start].team, Some(Team::Nfl(club)));
    talks::job_left(&mut w, "left for another job");
    at(&mut w, club);
    talks::job_taken(&mut w);
    assert!(w.folk.promises[start].kept.is_none() && w.folk.promises[stay].kept.is_none());
    assert!(talks::kept(&w, &w.folk.promises[stay].clone()), "still here");
    // Another club: his place closes quietly; the promise you'd stay is
    // broken, judged today.
    talks::job_left(&mut w, "left for another job");
    at(&mut w, new);
    talks::job_taken(&mut w);
    assert_eq!(w.folk.promises[start].kept, Some(true));
    assert!(w.folk.promises[start].released);
    let s = w.folk.promises[stay].clone();
    assert!(s.kept.is_none() && s.due == w.today);
    assert!(!talks::kept(&w, &s), "you left him for another job");
    // A man who had already left that club has nothing to hold you to.
    at(&mut w, club);
    let (_, stay) = open(&mut w);
    w.clubs[club].roster.retain(|&k| k != i);
    talks::job_left(&mut w, "left for another job");
    at(&mut w, new);
    talks::job_taken(&mut w);
    assert_eq!(w.folk.promises[stay].kept, Some(true), "closed quietly");
    w.clubs[club].roster.push(i);
    // Fired: you can keep none of them, and none is held against you.
    at(&mut w, club);
    let (start, stay) = open(&mut w);
    talks::job_left(&mut w, "fired");
    assert_eq!(w.folk.promises[start].kept, Some(true));
    assert_eq!(w.folk.promises[stay].kept, Some(true));
}

#[test]
fn a_man_you_cut_or_traded_was_let_down() {
    let (mut w, club, i) = club_man(Position::LB);
    let id = w.people[i].player.id;
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Start, 0);
    let k = w.folk.promises.len() - 1;
    let p = w.folk.promises[k].clone();
    assert!(!talks::sent_away(&w, &p) && !talks::lapsed(&w, &p), "still yours");
    // Gone on his own (his deal ran out, another club took him off your
    // practice squad): it lapses.
    w.clubs[club].roster.retain(|&x| x != i);
    assert!(!talks::sent_away(&w, &p));
    assert!(talks::lapsed(&w, &p), "left on his own");
    // Cut or traded by your club while it was open: broken in full.
    w.clubs[club].roster.push(i);
    talks::let_go(&mut w, i, club);
    w.clubs[club].roster.retain(|&x| x != i);
    let p = w.folk.promises[k].clone();
    assert!(p.sent);
    assert!(talks::sent_away(&w, &p), "cut or traded");
    assert!(!talks::lapsed(&w, &p));
    // Back with you: judged on his games again.
    w.clubs[club].roster.push(i);
    assert!(!talks::sent_away(&w, &p));
    w.clubs[club].roster.retain(|&x| x != i);
    // Retired: it lapses, never broken.
    w.people[i].stage = Stage::Done;
    assert!(!talks::sent_away(&w, &p) && talks::lapsed(&w, &p));
    // Another club's move marks nothing of yours.
    w.people[i].stage = Stage::Pro;
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Start, 0);
    let other = (club + 1) % w.clubs.len();
    talks::let_go(&mut w, i, other);
    assert!(!w.folk.promises.last().unwrap().sent);
}

#[test]
fn a_start_he_saw_is_half_the_snaps_on_his_side() {
    let most = [64, 60];
    let line =
        |off: u32, def: u32, st: u32| StatLine { snaps_off: off, snaps_def: def, snaps_st: st, ..Default::default() };
    assert!(games::field_start(Position::CB, &line(0, 30, 0), most));
    assert!(!games::field_start(Position::CB, &line(0, 29, 12), most), "special teams snaps aren't his side");
    assert!(!games::field_start(Position::WR, &line(0, 60, 0), most), "a receiver's side is the offense");
    assert!(games::field_start(Position::LS, &line(0, 0, 8), most), "a snapper who snapped");
    assert!(!games::field_start(Position::K, &line(0, 0, 0), most));
}
