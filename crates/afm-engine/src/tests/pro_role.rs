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
    assert!(w.folk.promises[start].job_closed, "closed because you left, with him still there");
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
    assert!(!w.folk.promises[stay].job_closed, "he had left first, not you");
    w.clubs[club].roster.push(i);
    // Fired: you can keep none of them, and none is held against you.
    at(&mut w, club);
    let (start, stay) = open(&mut w);
    talks::job_left(&mut w, "fired");
    assert_eq!(w.folk.promises[start].kept, Some(true));
    assert_eq!(w.folk.promises[stay].kept, Some(true));
    assert!(w.folk.promises[start].job_closed && w.folk.promises[stay].job_closed);
    // Fired after he had left on his own: closed, and not by your going.
    at(&mut w, club);
    let (start, _) = open(&mut w);
    w.clubs[club].roster.retain(|&k| k != i);
    talks::job_left(&mut w, "fired");
    assert!(w.folk.promises[start].quiet() && !w.folk.promises[start].job_closed);
}

#[test]
fn a_man_you_cut_or_traded_was_let_down() {
    let (mut w, club, i) = club_man(Position::LB);
    let id = w.people[i].player.id;
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Start, 0);
    let k = w.folk.promises.len() - 1;
    let p = w.folk.promises[k].clone();
    assert!(!talks::sent_away(&w, &p) && !talks::lapsed(&w, &p), "still yours");
    // Gone on his own (another club took him off your practice squad): it
    // lapses.
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

/// Only your club's own call marks its word: a cut (the cutdown, a cap
/// casualty, your front office) or a trade. A deal that runs out, a
/// practice squad deal that ends, an offer sheet left unmatched and an
/// expansion draft go through the same release or move and mark nothing.
#[test]
fn only_your_clubs_own_call_lets_him_down() {
    use crate::world::pro::{self, DeadWhy};
    let (mut w, club, _) = club_man(Position::LB);
    let men: Vec<usize> = w.clubs[club].roster.iter().copied().take(4).collect();
    let d = w.today;
    let promised = |w: &mut World, i: usize| {
        let id = w.people[i].player.id;
        talks::promise(w, PromiseTo::Player(id), PromiseKind::Start, 0);
        w.folk.promises.len() - 1
    };
    // His deal ran out.
    let k = promised(&mut w, men[0]);
    pro::release_with(&mut w, men[0], d, "his contract ran out", false, false, DeadWhy::Released);
    let p = w.folk.promises[k].clone();
    assert!(!p.sent && !w.clubs[club].roster.contains(&men[0]));
    assert!(talks::lapsed(&w, &p) && !talks::sent_away(&w, &p), "his deal ran out: it lapses");
    // Taken in an expansion draft.
    let k = promised(&mut w, men[1]);
    let other = (club + 1) % w.clubs.len();
    pro::transfer(&mut w, men[1], other, "taken in the expansion draft");
    assert!(!w.folk.promises[k].sent, "an expansion draft is no call of yours");
    // Cut at the cutdown.
    let k = promised(&mut w, men[2]);
    pro::cut_by_club(&mut w, men[2], d, "cut at the final cutdown");
    let p = w.folk.promises[k].clone();
    assert!(p.sent && talks::sent_away(&w, &p), "you cut him");
    // Traded.
    let k = promised(&mut w, men[3]);
    pro::club_lets_go(&mut w, men[3]);
    pro::transfer(&mut w, men[3], other, "traded");
    let p = w.folk.promises[k].clone();
    assert!(p.sent && talks::sent_away(&w, &p), "you traded him");
}

/// A promise closed with nothing to judge (it lapsed, or closed when you
/// left the job) is neither a word held nor one broken: P1's record, a
/// stake's belief, the wants he still has and his cards on you all skip it.
#[test]
fn a_promise_closed_quietly_is_no_word_kept() {
    use crate::world::folk::persona::Them;
    use crate::world::folk::persona::talk::{between, hits, items};
    use crate::world::folk::persona::wanting;
    use crate::world::folk::persona::wants::WantKind;
    let (mut w, club, i) = club_man(Position::LB);
    let id = w.people[i].player.id;
    let before = hits::stake_belief(&w, id);
    assert!(before.is_some());
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Start, 0);
    let k = w.folk.promises.len() - 1;
    assert!(wanting::met(&w, Them::Player(id), WantKind::Role), "an open promise covers his role");
    // He left on his own: it lapses on its day, quietly.
    w.clubs[club].roster.retain(|&x| x != i);
    let due = w.folk.promises[k].due;
    talks::check_promises(&mut w, due);
    let p = w.folk.promises[k].clone();
    assert_eq!(p.kept, Some(true));
    assert!(p.released && p.quiet() && !p.job_closed, "lapsed: closed with nothing to judge");
    w.clubs[club].roster.push(i);
    assert_eq!(hits::stake_belief(&w, id), before, "no kept promise in his record");
    assert!(!wanting::met(&w, Them::Player(id), WantKind::Role), "a lapsed promise gives him nothing");
    let card = items::items(&w, Them::Player(id), &[]);
    assert!(card.iter().any(|x| x.1.contains("(no longer stands)")) && !card.iter().any(|x| x.1.contains("(kept)")));
    // It lapsed while you were there: his card on you says it no longer
    // stands, even after you have moved on.
    w.folk.promises[k].team = Some(Team::Nfl((club + 1) % w.clubs.len()));
    let words = between::words(&w, Them::Player(id)).join(". ");
    assert!(words.contains("it no longer stands") && !words.contains("he kept it"), "{words}");
    // Closed when you left the job: it says so.
    w.folk.promises[k].job_closed = true;
    let words = between::words(&w, Them::Player(id)).join(". ");
    assert!(words.contains("it ended when the coach left that job"), "{words}");
    // A promise he saw you keep still reads as kept.
    let p = &mut w.folk.promises[k];
    p.released = false;
    p.job_closed = false;
    assert!(!p.quiet());
    let words = between::words(&w, Them::Player(id)).join(". ");
    assert!(words.contains("he kept it"), "{words}");
}

/// Fired with the season played out (the carousel comes the day after the
/// last game, before the promises fall due): his promise is judged on his
/// games, not closed. With games still to play it closes quietly.
#[test]
fn a_season_played_out_is_judged_when_you_leave() {
    use crate::world::folk::facts;
    let (mut w, club, i) = club_man(Position::WR);
    let id = w.people[i].player.id;
    let opp = (club + 1) % w.clubs.len();
    let today = w.today;
    w.training.arrived.push((id, today.add(-300)));
    let c = w.people[i].pro.as_mut().unwrap();
    c.out_days = 0;
    c.out_weeks = 0;
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Start, 0);
    let k = w.folk.promises.len() - 1;
    // Its window: three games he sat in, then two days to its day with no
    // game of yours left.
    w.folk.promises[k].made = today.add(-30);
    w.folk.promises[k].due = today.add(2);
    assert!(
        !facts::team_games(&w, Team::Nfl(club)).iter().any(|g| g.0 >= today && g.0 <= today.add(2)),
        "no game left before its day"
    );
    for d in [today.add(-21), today.add(-14), today.add(-7)] {
        let lines = [
            (id, StatLine { snaps_off: 4, games: 1, ..Default::default() }),
            (7, StatLine { snaps_off: 64, games: 1, ..Default::default() }),
        ];
        kept_game(&mut w, d, [club, opp], &lines, Some(0));
    }
    talks::job_left(&mut w, "fired");
    let p = w.folk.promises[k].clone();
    assert_eq!(p.kept, Some(false), "the season is in: you broke it");
    assert!(!p.job_closed);
    // A promise for next season closes.
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Start, 0);
    let k = w.folk.promises.len() - 1;
    w.folk.promises[k].due = today.add(talks::PLAYED_OUT_DAYS + 30);
    talks::job_left(&mut w, "fired");
    assert!(w.folk.promises[k].quiet() && w.folk.promises[k].job_closed);
}

/// Another league's club still alive in its playoffs has games to play,
/// though they aren't on its schedule: leaving then closes his promise.
/// Out of them, its season is played out and the promise is judged.
#[test]
fn a_club_still_in_its_playoffs_has_games_to_play() {
    use crate::world::pro::other::{self, CFL, Live};
    let mut w = seat("nfl");
    let year = w.today.year();
    let teams = other::teams_in(CFL, year);
    let f = teams[0];
    let i = (0..w.people.len()).find(|&k| w.people[k].stage == Stage::Pro).unwrap();
    let id = w.people[i].player.id;
    // The tiny world keeps no other league's clubs.
    if w.others.teams.len() <= f {
        w.others.teams.resize_with(f + 1, Default::default);
    }
    w.others.teams[f].roster.push(i as u32);
    w.others.live.retain(|l| l.league != CFL);
    w.others.live.push(Live { league: CFL, year, alive: vec![f as u16, teams[1] as u16], round: 2, ..Live::default() });
    let today = w.today;
    // Three of its games he sat in.
    for d in [today.add(-21), today.add(-14), today.add(-7)] {
        let lines = [
            (id, StatLine { snaps_off: 4, games: 1, ..Default::default() }),
            (7, StatLine { snaps_off: 64, games: 1, ..Default::default() }),
        ];
        kept_game(&mut w, d, [f, teams[1]], &lines, Some(0));
        w.seat_games.last_mut().unwrap().level = Level::Other;
    }
    for (alive, quiet) in [(true, true), (false, false)] {
        w.seat = Some(Seat { level: Level::Other, nfl_team: None, program: None, other: Some(f) });
        if !alive {
            w.others.live.last_mut().unwrap().alive.retain(|&t| t as usize != f);
        }
        talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Snaps, 0);
        let k = w.folk.promises.len() - 1;
        w.folk.promises[k].made = today.add(-60);
        w.folk.promises[k].due = today.add(10);
        talks::job_left(&mut w, "fired");
        let p = w.folk.promises[k].clone();
        assert_eq!(p.kept, Some(quiet), "alive {alive}: {p:?}");
        assert_eq!(p.quiet(), quiet, "alive {alive}: {p:?}");
        assert_eq!(p.job_closed, quiet, "alive {alive}");
    }
}

/// A man your club cut stays let down off your roster and on your practice
/// squad (the cut to it is the demotion); a club that signs him off your
/// squad took a man you let go. Back on your roster, the cut is behind
/// you, and leaving on his own terms then lapses it.
#[test]
fn a_man_cut_and_taken_back_is_judged_as_one_who_left_on_his_own() {
    use crate::world::pro::Status;
    let (mut w, club, i) = club_man(Position::LB);
    let id = w.people[i].player.id;
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Snaps, 0);
    let k = w.folk.promises.len() - 1;
    let today = w.today;
    let status = |w: &mut World, s: Status| w.people[i].pro.as_mut().unwrap().status = s;
    talks::let_go(&mut w, i, club);
    w.clubs[club].roster.retain(|&x| x != i);
    status(&mut w, Status::FreeAgent);
    talks::day(&mut w, today);
    assert!(w.folk.promises[k].sent, "off your roster: still the man you cut");
    // Signed to your practice squad: still the cut.
    w.clubs[club].roster.push(i);
    status(&mut w, Status::PracticeSquad);
    talks::day(&mut w, today);
    assert!(w.folk.promises[k].sent, "the practice squad is no way back");
    // Another club signs him off your squad: you let him go.
    w.clubs[club].roster.retain(|&x| x != i);
    let p = w.folk.promises[k].clone();
    assert!(talks::sent_away(&w, &p) && !talks::lapsed(&w, &p));
    // Back on your roster: the cut is behind you.
    w.clubs[club].roster.push(i);
    status(&mut w, Status::Roster);
    talks::day(&mut w, today);
    assert!(!w.folk.promises[k].sent, "back with you");
    // Then he leaves on his own terms: it lapses.
    w.clubs[club].roster.retain(|&x| x != i);
    let p = w.folk.promises[k].clone();
    assert!(talks::lapsed(&w, &p) && !talks::sent_away(&w, &p));
}

/// A man your club cut who plays on elsewhere: when you leave, his promise
/// is judged broken then, as on its day, not closed quietly.
#[test]
fn a_promise_your_club_broke_by_cutting_him_is_judged_when_you_leave() {
    let (mut w, club, _) = club_man(Position::LB);
    let men: Vec<usize> = w.clubs[club].roster.iter().copied().take(2).collect();
    for (n, why) in ["fired", "left for another job"].into_iter().enumerate() {
        let i = men[n];
        let id = w.people[i].player.id;
        talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Start, 0);
        let k = w.folk.promises.len() - 1;
        talks::let_go(&mut w, i, club);
        w.clubs[club].roster.retain(|&x| x != i);
        talks::job_left(&mut w, why);
        let p = w.folk.promises[k].clone();
        assert_eq!(p.kept, Some(false), "{why}: you cut him, so you broke it");
        assert!(!p.quiet(), "{why}");
        w.seat = Some(Seat { level: Level::Nfl, nfl_team: Some(club), program: None, other: None });
    }
}

/// A promise that you'd stay, to a coach who had already left your staff,
/// closes quietly when you take another job; to one still there, it breaks.
#[test]
fn a_stay_promise_to_a_coach_who_left_first_closes_quietly() {
    let (mut w, club, _) = club_man(Position::LB);
    let staff: Vec<usize> = (0..w.coaches.len())
        .filter(|&c| {
            w.coaches[c].job.as_ref().is_some_and(|j| {
                Team::from_employer(j.employer) == Team::Nfl(club) && Some(c) != w.nfl[club].head_coach
            })
        })
        .take(2)
        .collect();
    assert_eq!(staff.len(), 2, "two assistants on your staff");
    for &c in &staff {
        talks::promise(&mut w, PromiseTo::Coach(c), PromiseKind::Stay, 0);
    }
    let n = w.folk.promises.len();
    let (gone, here) = (n - 2, n - 1);
    // The first takes a job elsewhere before you go.
    let other = (club + 1) % w.clubs.len();
    w.coaches[staff[0]].job.as_mut().unwrap().employer = Team::Nfl(other).employer();
    talks::job_left(&mut w, "left for another job");
    w.seat = Some(Seat { level: Level::Nfl, nfl_team: Some((club + 2) % w.clubs.len()), program: None, other: None });
    talks::job_taken(&mut w);
    assert!(w.folk.promises[gone].quiet(), "he left first: nothing to hold you to");
    let p = w.folk.promises[here].clone();
    assert!(p.kept.is_none() && p.due == w.today && !talks::kept(&w, &p), "still there: you left him");
}

/// Another league's season he sat out on its roster stands at the year's
/// end, over the season before it.
#[test]
fn a_season_sat_out_in_another_league_stands_at_the_years_end() {
    use crate::world::pro::other::{CFL, OtherSeason as Row, UFL24};
    let (mut w, _, i) = club_man(Position::QB);
    let y = w.year;
    let c = w.people[i].pro.as_mut().unwrap();
    c.other.push(Row { year: y - 1, league: CFL, games: 18, starts: 16, ..Row::default() });
    role::write(&mut w, &[i]);
    assert_eq!(w.people[i].last_role, Some(PlayingTime::Starter));
    // This year on a roster without a game: a backup, and it stands.
    role::write_league(&mut w, &[i], CFL, y);
    assert_eq!(w.people[i].last_role, Some(PlayingTime::Backup));
    role::write_all(&mut w);
    assert_eq!(w.people[i].last_role, Some(PlayingTime::Backup), "the year's end keeps the newer season");
    // Next year a spring league's season he started, then the CFL's on its
    // roster without a game: the CFL's is the newer season, so a backup,
    // and the year's end doesn't read the spring one back over it.
    let c = w.people[i].pro.as_mut().unwrap();
    c.other.push(Row { year: y + 1, league: UFL24, games: 10, starts: 10, ..Row::default() });
    role::write_league(&mut w, &[i], UFL24, y + 1);
    assert_eq!(w.people[i].last_role, Some(PlayingTime::Starter));
    role::write_league(&mut w, &[i], CFL, y + 1);
    assert_eq!(w.people[i].last_role, Some(PlayingTime::Backup), "the spring games were another season");
    role::write_all(&mut w);
    assert_eq!(w.people[i].last_role, Some(PlayingTime::Backup), "the year's end keeps the CFL season");
    // A newer season played is read.
    let c = w.people[i].pro.as_mut().unwrap();
    c.other.push(Row { year: y + 2, league: CFL, games: 18, starts: 17, ..Row::default() });
    role::write_all(&mut w);
    assert_eq!(w.people[i].last_role, Some(PlayingTime::Starter));
    // An NFL season that year is newer than the CFL's: a tenth of the
    // snaps, a backup, over the CFL starter.
    nfl_row(&mut w, i, y + 2, 17, 0, [0, 0]);
    let c = w.people[i].pro.as_mut().unwrap();
    c.seasons.last_mut().unwrap().snaps = [100, 0, 0];
    role::write_all(&mut w);
    assert_eq!(w.people[i].last_role, Some(PlayingTime::Backup));
    assert_eq!(role::season_role(&w, i), Some(PlayingTime::Backup));
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

/// A promise from a save before promises kept their team is this job's
/// when you leave it, judged already or judged as you go, so Ground's
/// word reads it as made there and not at your next program.
#[test]
fn a_promise_from_before_is_stamped_with_the_job_you_leave() {
    let (mut w, club, _) = club_man(Position::LB);
    let men: Vec<usize> = w.clubs[club].roster.iter().copied().take(2).collect();
    let mut ks = Vec::new();
    for &i in &men {
        let id = w.people[i].player.id;
        talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Start, 0);
        let k = w.folk.promises.len() - 1;
        w.folk.promises[k].team = None;
        ks.push(k);
    }
    // One judged before you leave; one a man your club cut, judged as you go.
    w.folk.promises[ks[0]].kept = Some(false);
    talks::let_go(&mut w, men[1], club);
    w.clubs[club].roster.retain(|&x| x != men[1]);
    talks::job_left(&mut w, "left for another job");
    for &k in &ks {
        let p = &w.folk.promises[k];
        assert_eq!((p.team, p.kept), (Some(Team::Nfl(club)), Some(false)), "{p:?}");
    }
}

/// Between another league's last regular week and its seeding, a club the
/// table puts in the field still has games, and one it leaves out is
/// played out. In the regular season a club with a game left plays on;
/// one whose games are done and that no result left can put in the field
/// doesn't (the CFL's ragged last week).
#[test]
fn another_leagues_table_says_who_plays_on_before_the_seeding() {
    use crate::world::pro::other::{self, CFL, Live, Row};
    let mut w = seat("nfl");
    let year = w.today.year();
    let teams = other::teams_in(CFL, year);
    let n = teams.len();
    let table: Vec<Row> = teams
        .iter()
        .enumerate()
        .map(|(k, &f)| Row { fr: f as u16, w: (2 * (n - k)) as u8, l: (2 * k) as u8, ..Row::default() })
        .collect();
    w.others.live.retain(|l| l.league != CFL);
    w.others.live.push(Live { league: CFL, year, weeks: vec![Vec::new(); 3], week: 3, table, ..Live::default() });
    let (best, worst) = (teams[0], teams[n - 1]);
    assert!(other::plays_on(&w, best), "the table's best is in the field");
    assert!(!other::plays_on(&w, worst), "its worst is out");
    // A last regular week: a game of its own to play, then one between two
    // other clubs that can't lift it into the field.
    let l = w.others.live.last_mut().unwrap();
    l.week = 2;
    l.weeks[2] = vec![(worst as u16, best as u16)];
    assert!(other::plays_on(&w, worst), "a game of its own to play");
    w.others.live.last_mut().unwrap().weeks[2] = vec![(teams[1] as u16, teams[2] as u16)];
    assert!(!other::plays_on(&w, worst), "its games done, out whatever happens");
    assert!(other::plays_on(&w, best), "in whatever happens");
}

/// A high school after its last regular Friday: before its state draws
/// the brackets, a school its results put in the field plays on, and one
/// with no season to show doesn't (nor one left alone in its class, crowned
/// without a game); once drawn, the bracket says.
#[test]
fn a_school_before_the_playoff_draw_plays_on_only_in_its_field() {
    use crate::world::hs_playoffs::Bracket;
    let mut w = seat("hs");
    let s = (0..w.states.len()).max_by_key(|&s| w.states[s].schools.len()).unwrap();
    let schools = w.states[s].schools.clone();
    assert!(schools.len() >= 2, "{} schools", schools.len());
    w.hs_season = Some(crate::world::hs_year::new_season(&w));
    // One class of public schools, so the tiny world's few schools share a
    // bracket.
    for &h in &schools {
        w.high_schools[h].classification = 0;
        w.high_schools[h].private = false;
    }
    let n = schools.len();
    let season = |w: &mut World, out: Option<usize>| {
        let hs = w.hs_season.as_mut().unwrap();
        for (k, &h) in schools.iter().enumerate() {
            let r = &mut hs.results[h];
            let played = Some(h) != out;
            r.roster = if played { 40 } else { 0 };
            r.wins = if played { (n - k) as u32 } else { 0 };
            r.losses = if played { k as u32 + 1 } else { 0 };
            (r.conf_wins, r.conf_losses) = (r.wins, r.losses);
        }
    };
    let (best, last) = (schools[0], schools[n - 1]);
    let hs = w.hs_season.as_ref().unwrap();
    assert!(!hs.states[s].done && !hs.states[s].rounds.is_empty());
    let rounds = w.hs_season.as_ref().unwrap().states[s].rounds.clone();
    season(&mut w, None);
    assert!(w.hs_next_round(best).is_some() && w.hs_next_round(last).is_some(), "both in the field");
    if n == 2 {
        // A two-team bracket starts at the state's last round, its final.
        assert_eq!(w.hs_next_round(best), rounds.last().copied(), "its own round, not the state's first");
    }
    season(&mut w, Some(last));
    assert_eq!(w.hs_next_round(last), None, "no season, no field");
    assert_eq!(w.hs_next_round(best).is_some(), n > 2, "alone in its class it plays no game");
    season(&mut w, None);
    let hs = w.hs_season.as_mut().unwrap();
    let seeds: Vec<u32> = schools[1..].iter().map(|&h| h as u32).collect();
    hs.brackets.push(Bracket::new(s, "Drawn".into(), seeds, rounds.len(), false));
    assert_eq!(w.hs_next_round(best), None, "drawn without it");
    assert_eq!(w.hs_next_round(last).is_some(), n > 2, "drawn with it (alone, crowned)");
}

/// Taking another job at your own club (a promotion) keeps every promise
/// open, a cut man's too: he can still come back while you're there.
/// Taking one at another club judges what is settled, as when you're fired.
#[test]
fn a_move_at_your_own_club_keeps_a_cut_mans_promise_open() {
    use crate::world::coach::{Employer, Role};
    let (mut w, club, _) = club_man(Position::LB);
    let i = w.clubs[club].roster[1];
    let id = w.people[i].player.id;
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Snaps, 0);
    let k = w.folk.promises.len() - 1;
    talks::let_go(&mut w, i, club);
    w.clubs[club].roster.retain(|&x| x != i);
    talks::leaving_for(&mut w, Team::Nfl(club));
    assert_eq!(w.folk.promises[k].kept, None, "a move at the same club judges nothing");
    let other = (0..w.clubs.len()).find(|&t| t != club && crate::world::franchise::active_in(&w, t, w.year)).unwrap();
    crate::world::staff::take_job(&mut w, Employer::Nfl(other), Role::OffensiveCoordinator);
    assert_eq!(w.folk.promises[k].kept, Some(false), "another club: the cut is judged as you go");
}

/// A promise whose day has come is judged when you're fired that morning,
/// whatever else is open, not closed quietly.
#[test]
fn a_promise_due_the_day_you_leave_is_judged() {
    let (mut w, _, i) = club_man(Position::LB);
    let id = w.people[i].player.id;
    let pos = w.people[i].player.position as u8;
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Position(pos), 0);
    let k = w.folk.promises.len() - 1;
    w.folk.promises[k].due = w.today;
    talks::job_left(&mut w, "fired");
    let p = &w.folk.promises[k];
    assert_eq!(p.kept, Some(true), "{p:?}");
    assert!(!p.quiet(), "judged, not closed: {p:?}");
}
