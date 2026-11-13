//! A pro's role at your club (code oddity 29, found by the praise fix): a
//! pro's `last_role` is never written after the draft, so at an NFL club
//! the talks read his last college season: which promise he was offered,
//! its rank, and whether a promise of a start, snaps or his place was kept.
//! They read the club now: the chart your team would dress today, and the
//! games he has played for it this season.

use super::legacy::seat;
use crate::calendar::{Date, Level};
use crate::game::{GameStats, PlayRecord, PlayType};
use crate::league::depth_slots;
use crate::player::PlayingTime;
use crate::positions::Position;
use crate::world::folk::scene;
use crate::world::folk::talks::{self, Promise, PromiseKind, PromiseTo, Subject};
use crate::world::pro::{ProSeason, Status};
use crate::world::seat::SeatGame;
use crate::world::{Seat, Stage, World};

type Chart = Vec<(Position, Vec<u32>)>;

/// Your NFL club (its index) with the chart it would dress today.
fn club() -> (World, usize, Chart) {
    let w = seat("nfl");
    let t = w.seat.and_then(|s| s.nfl_team).unwrap();
    let chart = w.seat_game_depth().expect("your club can dress a game team");
    (w, t, chart)
}

fn person(w: &World, id: u32) -> usize {
    crate::world::folk::person(w, id).unwrap()
}

/// The man in a slot of today's chart at a position.
fn at(w: &World, chart: &Chart, pos: Position, slot: usize) -> usize {
    person(w, chart.iter().find(|(p, _)| *p == pos).unwrap().1[slot])
}

/// A man today's chart only rotates in: first past the starters where a
/// position rotates, and a starter on no other list (the chart can list an
/// edge at tackle too).
fn rotates(w: &World, chart: &Chart) -> usize {
    let starts = |id: u32| chart.iter().any(|(p, ids)| ids.iter().take(depth_slots(*p).starters).any(|&x| x == id));
    chart
        .iter()
        .filter(|(p, _)| depth_slots(*p).rotation > 0)
        .filter_map(|(p, ids)| ids.get(depth_slots(*p).starters).copied())
        .find(|&id| !starts(id))
        .map(|id| person(w, id))
        .expect("a spot that rotates")
}

/// A promise to him, as the talk logs it.
fn made(w: &World, i: usize, kind: PromiseKind) -> Promise {
    Promise {
        to: PromiseTo::Player(w.people[i].player.id),
        kind,
        made: w.today,
        due: w.today,
        public: false,
        kept: None,
        who: String::new(),
    }
}

/// A game of your club's on `date` at `level` (club `t`, at home or away),
/// as the world keeps your games: each man with whether he lined up for his
/// side's first snap and the snaps he took (a specialist's are kicking
/// snaps).
fn kept_game(w: &mut World, level: Level, t: usize, date: Date, away: bool, men: &[(usize, bool, u32)]) {
    let side = away as usize;
    let mut stats = GameStats::default();
    // Your offense's first snap, then your defense's (the other side has
    // the ball).
    let mut first =
        [PlayRecord::new(1, 1, 900.0, side, PlayType::Run), PlayRecord::new(2, 1, 860.0, 1 - side, PlayType::Pass)];
    for &(i, started, snaps) in men {
        let p = &w.people[i].player;
        let unit =
            matches!(p.position, Position::EDGE | Position::DT | Position::LB | Position::CB | Position::S) as usize;
        let line = stats.line(side, p.id);
        if p.position.is_specialist() {
            line.snaps_st = snaps;
        } else if unit == 0 {
            line.snaps_off = snaps;
        } else {
            line.snaps_def = snaps;
        }
        if started {
            first[unit].on_field[unit].push(p.id);
        }
    }
    stats.plays.extend(first);
    let other = if t == 0 { 1 } else { 0 };
    w.seat_games.push(SeatGame {
        date,
        level,
        teams: if away { [other, t] } else { [t, other] },
        names: Default::default(),
        stats,
        replay: Vec::new(),
        marks: Vec::new(),
        gone: Vec::new(),
        spread: None,
        talks: Default::default(),
        whistle: None,
    });
}

/// A home game of your NFL club `week` weeks after this season's kickoff.
fn game(w: &mut World, t: usize, week: i32, men: &[(usize, bool, u32)]) {
    let date = w.dates.nfl_kickoff.add(7 * week);
    kept_game(w, Level::Nfl, t, date, false, men);
}

/// The same line in each of your club's first `games` games.
fn games(w: &mut World, t: usize, n: i32, men: &[(usize, bool, u32)]) {
    w.seat_games.clear();
    for week in 0..n {
        game(w, t, week, men);
    }
}

/// The label of the pick for a move in a fresh one-on-one about his role.
fn pick(w: &mut World, i: usize, mv: &str) -> String {
    let id = w.people[i].player.id;
    w.folk.mine.hours = 0.0;
    let sid = talks::start_talk(w, id, Subject::Role).unwrap();
    let s = scene::open(w, sid).unwrap();
    let label = s.picks.iter().find(|x| x.mv.starts_with(mv)).unwrap_or_else(|| panic!("no {mv} pick")).label.clone();
    w.folk.scenes.retain(|x| x.id != sid);
    label
}

/// The promise a one-on-one on `subject` logs when you pick Promise.
fn promised(w: &mut World, i: usize, subject: Subject) -> PromiseKind {
    let id = w.people[i].player.id;
    w.folk.mine.hours = 0.0;
    let sid = talks::start_talk(w, id, subject).unwrap();
    let s = scene::open(w, sid).unwrap();
    let code = s.picks.iter().find(|x| x.mv == "Promise").unwrap_or_else(|| panic!("{subject:?}: no promise")).code;
    scene::answer(w, sid, code).unwrap();
    w.folk.scenes.retain(|x| x.id != sid);
    w.folk.promises.pop().expect("the promise is logged").kind
}

#[test]
fn a_pro_is_read_at_his_club_not_at_his_college() {
    let (mut w, _, chart) = club();
    // Your starting quarterback was a college backup; your third receiver
    // down the chart was a college star. Before the fix the talks had them
    // the wrong way round.
    let qb = at(&w, &chart, Position::QB, 0);
    let deep = depth_slots(Position::WR);
    let wr = at(&w, &chart, Position::WR, deep.starters + deep.rotation);
    w.people[qb].last_role = Some(PlayingTime::Backup);
    w.people[wr].last_role = Some(PlayingTime::Starter);
    assert_eq!(talks::role_now(&w, qb), Some(PlayingTime::Starter), "first on your chart");
    assert_eq!(talks::role_now(&w, wr), Some(PlayingTime::Backup), "past the men who rotate in");
    assert_eq!(talks::played_role(&w, qb), None, "no game yet");
    // A generated veteran has no college role at all.
    w.people[qb].last_role = None;
    assert_eq!(talks::role_now(&w, qb), Some(PlayingTime::Starter));

    // The talk offers each the line that fits him.
    assert!(pick(&mut w, qb, "Tell").contains("You're my starter"), "your starter hears it straight");
    assert!(pick(&mut w, qb, "Promise").contains("get you the ball more"));
    assert!(pick(&mut w, wr, "Tell").contains("where you are on the chart"));
    assert!(pick(&mut w, wr, "Promise").contains("the job is yours by the opener"));

    // A promise is judged on the club too: the college star who sits has
    // not been given the job, and your starter has kept his.
    assert!(!talks::kept(&w, &made(&w, wr, PromiseKind::Start)), "a backup wasn't given the job");
    assert!(!talks::kept(&w, &made(&w, wr, PromiseKind::Snaps)), "nor more snaps");
    assert!(talks::kept(&w, &made(&w, qb, PromiseKind::Start)));
    assert!(talks::kept(&w, &made(&w, qb, PromiseKind::Place(0))), "still your starter");
    assert!(talks::kept(&w, &made(&w, wr, PromiseKind::Place(PlayingTime::Backup as u8))), "still on the team");

    // The rotation is a role of its own: the first man past the starters
    // where a position rotates.
    let third = rotates(&w, &chart);
    w.people[third].last_role = None;
    assert_eq!(talks::role_now(&w, third), Some(PlayingTime::Rotation));
    assert!(talks::kept(&w, &made(&w, third, PromiseKind::Snaps)));
    assert!(!talks::kept(&w, &made(&w, third, PromiseKind::Start)));
    assert!(talks::kept(&w, &made(&w, third, PromiseKind::Place(PlayingTime::Rotation as u8))));
    assert!(!talks::kept(&w, &made(&w, third, PromiseKind::Place(0))), "he was told he starts");

    // The promise a talk logs takes his rank with you: your starter is
    // promised his place as the starter, and the college star who sits is
    // promised the job, not more of the ball.
    assert_eq!(promised(&mut w, qb, Subject::Behavior), PromiseKind::Place(0));
    assert_eq!(promised(&mut w, wr, Subject::Role), PromiseKind::Start);
}

#[test]
fn a_promise_is_kept_by_the_games_he_plays_for_you() {
    let (mut w, t, chart) = club();
    let slots = depth_slots(Position::WR);
    let wr = at(&w, &chart, Position::WR, slots.starters + slots.rotation);
    w.people[wr].last_role = Some(PlayingTime::Backup);
    let start = made(&w, wr, PromiseKind::Start);
    let snaps = made(&w, wr, PromiseKind::Snaps);
    assert!(!talks::kept(&w, &start));

    // He started the opener and was hurt in it: the job was his. An opener
    // can come the day before the league's kickoff date (2026's did).
    let opener = w.dates.nfl_kickoff.add(-1);
    kept_game(&mut w, Level::Nfl, t, opener, false, &[(wr, true, 55)]);
    w.people[wr].pro.as_mut().unwrap().out_weeks = 6;
    assert!(w.seat_game_depth().unwrap().iter().all(|(_, ids)| !ids.contains(&w.people[wr].player.id)), "out hurt");
    assert_eq!(talks::played_role(&w, wr), Some(PlayingTime::Starter));
    assert_eq!(talks::role_now(&w, wr), Some(PlayingTime::Starter), "your starter, out hurt");
    assert!(talks::kept(&w, &start), "he started the opener");
    // Healthy again and back down the chart: you still gave him the job
    // for the opener, but he isn't your starter today.
    w.people[wr].pro.as_mut().unwrap().out_weeks = 0;
    assert_eq!(talks::role_now(&w, wr), Some(PlayingTime::Backup));
    assert!(talks::kept(&w, &start));
    assert!(!talks::kept(&w, &made(&w, wr, PromiseKind::Place(0))));

    // Ten games in: a fifth of a starter's snaps without a start is the
    // rotation, half is a starter's load, less than a fifth is a backup.
    games(&mut w, t, 10, &[(wr, false, 20)]);
    assert_eq!(talks::played_role(&w, wr), Some(PlayingTime::Rotation));
    assert_eq!(talks::role_now(&w, wr), Some(PlayingTime::Backup), "today's chart");
    assert!(talks::kept(&w, &snaps), "he got his snaps");
    assert!(!talks::kept(&w, &start), "but not the job");
    games(&mut w, t, 10, &[(wr, false, 40)]);
    assert_eq!(talks::played_role(&w, wr), Some(PlayingTime::Starter));
    assert!(talks::kept(&w, &start));
    games(&mut w, t, 10, &[(wr, false, 5)]);
    assert_eq!(talks::played_role(&w, wr), Some(PlayingTime::Backup));
    assert!(!talks::kept(&w, &snaps));
    // Dressed for two of your ten games, his snaps are still read against
    // all ten.
    games(&mut w, t, 10, &[]);
    let wid = w.people[wr].player.id;
    for g in w.seat_games.iter_mut().take(2) {
        g.stats.line(0, wid).snaps_off = 40;
    }
    assert_eq!(talks::played_role(&w, wr), Some(PlayingTime::Backup));

    // Only his games for you this season count. Last season's say nothing.
    games(&mut w, t, 10, &[(wr, true, 60)]);
    assert!(talks::kept(&w, &start));
    for g in &mut w.seat_games {
        g.date = g.date.add(-300);
    }
    assert_eq!(talks::played_role(&w, wr), None);
    assert!(!talks::kept(&w, &start));
    // Nor do games he played against you before he came in a trade: he
    // was on the other side.
    games(&mut w, t, 10, &[(wr, true, 60)]);
    for g in &mut w.seat_games {
        g.teams.swap(0, 1);
    }
    assert_eq!(talks::played_role(&w, wr), None);
    assert!(!talks::kept(&w, &start));
    assert!(!talks::kept(&w, &snaps));
}

#[test]
fn benched_hurt_demoted_or_gone() {
    let (mut w, t, chart) = club();
    let qb = at(&w, &chart, Position::QB, 0);
    let next = at(&w, &chart, Position::QB, 1);
    let (id, nid) = (w.people[qb].player.id, w.people[next].player.id);
    let place = made(&w, qb, PromiseKind::Place(0));
    assert!(talks::kept(&w, &place));
    assert_eq!(talks::role_now(&w, next), Some(PlayingTime::Backup), "one quarterback plays");

    // Benched on your own chart before he has played: broken.
    w.set_seat_depth(Position::QB, vec![nid, id]);
    assert_eq!(talks::role_now(&w, qb), Some(PlayingTime::Backup));
    assert_eq!(talks::role_now(&w, next), Some(PlayingTime::Starter));
    assert!(!talks::kept(&w, &place), "benched breaks it");
    assert!(talks::kept(&w, &made(&w, next, PromiseKind::Start)), "and the job is the other man's");
    // Benched after six starts: his place is still gone, though a promise
    // of the job was kept by the games he started.
    games(&mut w, t, 6, &[(qb, true, 60)]);
    assert_eq!(talks::played_role(&w, qb), Some(PlayingTime::Starter));
    assert_eq!(talks::role_now(&w, qb), Some(PlayingTime::Backup), "he sits today");
    assert!(!talks::kept(&w, &place), "six starts don't keep his place");
    assert!(talks::kept(&w, &made(&w, qb, PromiseKind::Start)));
    w.seat_depth.clear();
    w.seat_games.clear();

    // Out hurt all year: he can't dress, so he is read from his last season
    // with the club, as a college player keeps his role while he is out.
    w.people[qb].pro.as_mut().unwrap().out_weeks = 30;
    w.people[qb].pro.as_mut().unwrap().seasons.clear();
    assert_eq!(talks::role_now(&w, qb), Some(PlayingTime::Backup), "no season with you to read");
    let year = w.year;
    let full = w.rules.nfl_games;
    let row = |year: i32, team: usize, games: u32| ProSeason {
        year,
        team: Some(team),
        games,
        starts: games,
        snaps: [games * 60, 0, 0],
        ..Default::default()
    };
    // The last season he played, even when he missed all of last year.
    w.people[qb].pro.as_mut().unwrap().seasons.push(row(year - 2, t, full));
    w.people[qb].pro.as_mut().unwrap().seasons.push(row(year - 1, t, 0));
    assert_eq!(talks::role_now(&w, qb), Some(PlayingTime::Starter), "your starter, out hurt");
    assert!(talks::kept(&w, &place), "an injury doesn't break your word");
    // A last season played for another club says nothing about yours.
    w.people[qb].pro.as_mut().unwrap().seasons.push(row(year - 1, if t == 0 { 1 } else { 0 }, full));
    assert_eq!(talks::role_now(&w, qb), Some(PlayingTime::Backup));
    w.people[qb].pro.as_mut().unwrap().seasons.pop();
    // Healthy again and sitting behind another man, last season says nothing.
    w.people[qb].pro.as_mut().unwrap().out_weeks = 0;
    assert!(talks::kept(&w, &place));
    w.set_seat_depth(Position::QB, vec![nid, id]);
    assert!(!talks::kept(&w, &place));
    w.seat_depth.clear();

    // Sent down to the practice squad: his place changed.
    let kept_place = made(&w, next, PromiseKind::Place(PlayingTime::Backup as u8));
    assert!(talks::kept(&w, &kept_place));
    w.people[next].pro.as_mut().unwrap().status = Status::PracticeSquad;
    assert_eq!(talks::role_now(&w, next), Some(PlayingTime::PracticeSquad));
    assert!(!talks::kept(&w, &kept_place), "the practice squad isn't his place");
    w.people[next].pro.as_mut().unwrap().status = Status::Roster;

    // Cut or traded: no role with you, whatever he was in college and
    // whatever he played for you.
    w.people[next].last_role = Some(PlayingTime::Starter);
    games(&mut w, t, 4, &[(next, true, 60)]);
    assert!(talks::kept(&w, &made(&w, next, PromiseKind::Start)));
    w.clubs[t].roster.retain(|&j| j != next);
    assert_eq!(talks::role_now(&w, next), None);
    assert_eq!(talks::played_role(&w, next), None);
    assert!(!talks::kept(&w, &made(&w, next, PromiseKind::Start)));
    assert!(!talks::kept(&w, &made(&w, next, PromiseKind::Snaps)));
    assert!(!talks::kept(&w, &kept_place));
}

#[test]
fn a_kicker_is_the_man_who_kicks() {
    let (mut w, t, chart) = club();
    let k = at(&w, &chart, Position::K, 0);
    let lb = *w.clubs[t]
        .roster
        .iter()
        .rev()
        .find(|&&i| w.people[i].player.position == Position::LB && talks::role_now(&w, i) == Some(PlayingTime::Backup))
        .expect("a backup linebacker");
    assert_eq!(talks::role_now(&w, k), Some(PlayingTime::Starter));
    // A kicking spot has no scrimmage snap to start or to count: the man who
    // kicked your games is your kicker while he is out. A backup who only
    // covers kicks is still a backup.
    games(&mut w, t, 4, &[(k, false, 8), (lb, false, 0)]);
    let lid = w.people[lb].player.id;
    for g in &mut w.seat_games {
        g.stats.line(0, lid).snaps_st = 14;
    }
    assert_eq!(talks::played_role(&w, k), Some(PlayingTime::Starter));
    assert_eq!(talks::played_role(&w, lb), Some(PlayingTime::Backup));
    w.people[k].pro.as_mut().unwrap().out_weeks = 3;
    assert_eq!(talks::role_now(&w, k), Some(PlayingTime::Starter));
    assert!(talks::kept(&w, &made(&w, k, PromiseKind::Place(0))), "hurt, and still your kicker");
    w.people[lb].pro.as_mut().unwrap().out_weeks = 3;
    assert_eq!(talks::role_now(&w, lb), Some(PlayingTime::Backup));
}

#[test]
fn an_away_game_and_a_defender_count_the_same() {
    let (mut w, t, chart) = club();
    let cb = at(&w, &chart, Position::CB, 0);
    let lb = at(&w, &chart, Position::LB, 0);
    w.seat_games.clear();
    for k in 0..4 {
        let d = w.dates.nfl_kickoff.add(7 * k);
        kept_game(&mut w, Level::Nfl, t, d, k % 2 == 1, &[(cb, true, 60), (lb, false, 8)]);
    }
    assert_eq!(talks::played_role(&w, cb), Some(PlayingTime::Starter), "on the field for the first snap");
    assert_eq!(talks::played_role(&w, lb), Some(PlayingTime::Backup), "a few snaps a game");
    // Away or at home, only his own side's lines are his.
    for g in &mut w.seat_games {
        g.teams.swap(0, 1);
    }
    assert_eq!(talks::played_role(&w, cb), None);
}

#[test]
fn a_job_you_left() {
    let (mut w, t, chart) = club();
    let wr = at(&w, &chart, Position::WR, 0);
    let start = made(&w, wr, PromiseKind::Start);
    let place = made(&w, wr, PromiseKind::Place(0));
    games(&mut w, t, 3, &[(wr, true, 60)]);
    assert_eq!(talks::played_role(&w, wr), Some(PlayingTime::Starter));
    assert!(!talks::lapsed(&w, &start) && !talks::lapsed(&w, &place));

    // You took this job after those games (a move from another club in
    // January): they weren't yours to count, and a promise you made before
    // it lapses quietly.
    let moved = w.seat_games.last().unwrap().date.add(1);
    w.career.as_mut().unwrap().market.job_since = Some(moved);
    assert_eq!(talks::played_role(&w, wr), None);
    assert!(talks::lapsed(&w, &start) && talks::lapsed(&w, &place));
    let since = Promise { made: moved, ..made(&w, wr, PromiseKind::Start) };
    assert!(!talks::lapsed(&w, &since), "one made since stands");
    w.career.as_mut().unwrap().market.job_since = None;

    // Out of work (fired on Black Monday, the day his promises fall due):
    // he has no role with you, and your promises to him lapse rather than
    // being read off his college role.
    w.people[wr].last_role = Some(PlayingTime::Starter);
    let seat = w.seat;
    w.seat = Some(Seat { level: Level::Nfl, ..Default::default() });
    assert_eq!(talks::role_now(&w, wr), None);
    assert_eq!(talks::played_role(&w, wr), None);
    assert!(talks::lapsed(&w, &start) && talks::lapsed(&w, &place));
    w.seat = seat;
    assert!(!talks::lapsed(&w, &start));
}

#[test]
fn last_season_you_coached_is_read_from_your_games() {
    let (mut w, t, chart) = club();
    let qb = at(&w, &chart, Position::QB, 0);
    let mut last = w.dates.clone();
    last.start = w.dates.start.add(-365);
    w.prev_dates = Some(last);
    // His row has him starting every game (the club's own order listed
    // him), and he is out hurt with no game this season.
    let (year, full) = (w.year, w.rules.nfl_games);
    let c = w.people[qb].pro.as_mut().unwrap();
    c.seasons.clear();
    c.seasons.push(ProSeason {
        year: year - 1,
        team: Some(t),
        games: full,
        starts: full,
        snaps: [full * 60, 0, 0],
        ..Default::default()
    });
    c.out_weeks = 30;
    w.seat_games.clear();
    for k in 0..10 {
        let d = w.dates.start.add(-150 + 7 * k);
        kept_game(&mut w, Level::Nfl, t, d, false, &[(qb, false, 5)]);
    }
    // You didn't coach last season: his row says.
    assert_eq!(talks::role_now(&w, qb), Some(PlayingTime::Starter));
    // You did: your games say he sat behind the man you started.
    w.career.as_mut().unwrap().market.job_since = Some(w.dates.start.add(-365));
    assert_eq!(talks::role_now(&w, qb), Some(PlayingTime::Backup));
    assert_eq!(talks::played_role(&w, qb), None, "not this season's");
    assert!(!talks::kept(&w, &made(&w, qb, PromiseKind::Place(0))));
}

#[test]
fn a_club_in_another_league_reads_its_own_games() {
    use crate::world::pro::other::{self, OtherSeason};
    let (mut w, _, chart) = club();
    let qb = at(&w, &chart, Position::QB, 0);
    let wr = at(&w, &chart, Position::WR, 0);
    // Your seat is a club in another league with these two on it: too few
    // to dress a game team, so neither is on a chart.
    let (f, opp) = (0, 1);
    if w.others.teams.len() < 2 {
        w.others.teams.resize(2, Default::default());
    }
    w.others.teams[f].roster = vec![qb as u32, wr as u32];
    w.seat = Some(Seat { level: Level::Other, nfl_team: None, program: None, other: Some(f) });
    assert!(w.seat_game_depth().is_none());
    assert_eq!(talks::role_now(&w, qb), Some(PlayingTime::Backup), "no game and no row");

    // His games for your club this season.
    w.seat_games.clear();
    for k in 0..6 {
        let d = w.dates.start.add(60 + 7 * k);
        kept_game(&mut w, Level::Other, f, d, false, &[(qb, true, 60), (wr, false, 20)]);
    }
    assert_eq!(talks::role_now(&w, qb), Some(PlayingTime::Starter));
    assert_eq!(talks::role_now(&w, wr), Some(PlayingTime::Rotation));
    assert!(talks::kept(&w, &made(&w, wr, PromiseKind::Snaps)));
    assert!(!talks::kept(&w, &made(&w, wr, PromiseKind::Start)));
    // An NFL game of a club with the same index isn't his.
    for g in &mut w.seat_games {
        g.level = Level::Nfl;
    }
    assert_eq!(talks::played_role(&w, qb), None);

    // Out hurt before he has played this season: his row with your club,
    // from its starts.
    w.seat_games.clear();
    let year = w.year;
    let row = |team: usize| OtherSeason {
        year: year - 1,
        league: other::league_of(team),
        team: team as u16,
        games: 18,
        starts: 18,
        ..Default::default()
    };
    let c = w.people[qb].pro.as_mut().unwrap();
    c.out_weeks = 10;
    c.other.clear();
    c.other.push(row(f));
    assert_eq!(talks::role_now(&w, qb), Some(PlayingTime::Starter), "your starter, out hurt");
    assert!(talks::kept(&w, &made(&w, qb, PromiseKind::Place(0))), "an injury doesn't break your word");
    // A last season with another club says nothing about yours.
    w.people[qb].pro.as_mut().unwrap().other.push(row(opp));
    assert_eq!(talks::role_now(&w, qb), Some(PlayingTime::Backup));
}

#[test]
fn a_school_player_keeps_his_own_role() {
    // College and high school players are read as before: fall camp's role,
    // then the game-day chart's (`follow_chart`).
    for level in ["college", "hs"] {
        let mut w = seat(level);
        w.add_class_aged(19);
        let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
        for role in [None, Some(PlayingTime::Starter), Some(PlayingTime::Rotation), Some(PlayingTime::Redshirt)] {
            w.people[i].last_role = role;
            assert_eq!(talks::role_now(&w, i), role, "{level}");
            assert_eq!(talks::played_role(&w, i), None, "{level}");
        }
    }
    // A player you coached in college who has since turned pro: your
    // promise to him is still read off his last season with you.
    let mut w = seat("college");
    w.add_class_aged(19);
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    w.people[i].stage = Stage::Pro;
    w.people[i].last_role = Some(PlayingTime::Starter);
    assert_eq!(talks::role_now(&w, i), Some(PlayingTime::Starter));
    assert_eq!(talks::played_role(&w, i), None);
    assert!(talks::kept(&w, &made(&w, i, PromiseKind::Start)));
    assert!(!talks::lapsed(&w, &made(&w, i, PromiseKind::Start)), "you still hold the job you made it in");
}
