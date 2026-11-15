//! Promises on how you'll use him (the promises on usage, agreed
//! 2026-11-14; `world/folk/usage.rs`). The proofs: each measure reads the
//! play log as it says; the reader's labels make a term only where it fits
//! his spot; chips are saved; a plan that keeps nothing changes no game;
//! each plan setting moves its measure in real games on the engine (free
//! him up, send him, the field goal line, take shots, help him); and a term
//! is judged game by game through the agreed promise rules, a "never" term
//! broken by its first broken game.

use super::legacy::seat;
use crate::calendar::{Date, Level as CalLevel};
use crate::game::chart::{LineChart, Rep, Rusher};
use crate::game::roster::GameTeam;
use crate::game::stats::{GameStats, StatLine};
use crate::game::synthetic::synthetic_league;
use crate::game::{GameResult, GameSetup, PlayRecord, PlayType, play_game};
use crate::player::{Level, PlayingTime};
use crate::positions::Position;
use crate::world::folk::promise::{self, State, Terms, Verdict};
use crate::world::folk::talks::{self, PromiseKind, PromiseTo};
use crate::world::folk::usage::{self, Spot, Usage, What};
use crate::world::seat::SeatGame;
use crate::world::tally::depth::{dslot, orole};
use crate::world::{Stage, World};

fn play(kind: PlayType, offense: usize) -> PlayRecord {
    PlayRecord::new(1, 1, 600.0, offense, kind)
}

#[test]
fn each_measure_reads_the_play_log() {
    let (me, mate) = (7u32, 8u32);
    // Three passes for us (side 0): a deep shot to him, a short one to his
    // mate, a sack; one run he carried; one punt from their 30.
    let mut deep = play(PlayType::Pass, 0);
    deep.passer = Some(1);
    deep.target = Some(me);
    deep.air_yards = 24.0;
    deep.on_field = [vec![1, me, mate], vec![]];
    deep.roles = [vec![orole::QB, orole::SLOT, orole::X], vec![]];
    let mut short = deep.clone();
    short.target = Some(mate);
    short.air_yards = 6.0;
    let mut sack = deep.clone();
    sack.kind = PlayType::Sack;
    sack.target = None;
    let mut run = play(PlayType::Run, 0);
    run.carrier = Some(me);
    run.on_field = [vec![1, me], vec![]];
    run.yardline_100 = 12.0;
    let mut punt = play(PlayType::Punt, 0);
    punt.yardline_100 = 30.0;
    let plays = vec![deep, short, sack, run, punt];
    let t = |what: What| Usage { what, ..Usage::default() };
    assert_eq!(t(What::Targets).count(&plays, 0, me).n, 1.0);
    assert_eq!(Usage { far: 20, ..t(What::Targets) }.count(&plays, 0, me).n, 1.0, "the deep one");
    assert_eq!(Usage { far: 20, ..t(What::Targets) }.count(&plays, 0, mate).n, 0.0);
    assert_eq!(t(What::Carries).count(&plays, 0, me).n, 1.0);
    assert_eq!(Usage { inside: 20, ..t(What::Carries) }.count(&plays, 0, me).n, 1.0, "a red zone carry");
    assert_eq!(Usage { inside: 5, ..t(What::Carries) }.count(&plays, 0, me).n, 0.0, "not at the goal line");
    // Snaps in the slot: three of his four snaps.
    let slot = Usage { what: What::Snaps, spot: Spot::Slot, share: true, ..Usage::default() };
    let c = slot.count(&plays, 0, me);
    assert_eq!((c.n, c.of), (3.0, 4.0));
    // The field goal line at a 50-yard try (the 33): their 30 breaks it.
    let line = Usage::fg_line(50);
    assert!(line.never() && line.team());
    assert_eq!(line.inside, 33);
    assert_eq!(line.game(&plays, 0, 99, false), Some(false), "a punt inside the line, whoever played");
    let line = Usage::fg_line(45);
    assert_eq!(line.game(&plays, 0, 99, false), Some(true), "their 30 is outside a 45-yard line");
    // A share with no chances says nothing; a man who sat out got nothing.
    let rush = Usage { what: What::Stunts, share: true, n: 0.25, ..Usage::default() };
    assert_eq!(rush.game(&plays, 1, me, true), None);
    assert_eq!(t(What::Targets).game(&plays, 0, 55, false), Some(false));
}

#[test]
fn every_role_a_snap_keeps_has_its_own_spot() {
    // One defensive snap with Sam, Mike, Will, both corners, the nickel and
    // the dime; one offensive snap with both guards, a tailback and a
    // fullback. Each man's own key holds him; the unions hold both.
    let mut d = play(PlayType::Pass, 0);
    let ids: Vec<u32> = (40..47).collect();
    d.on_field = [vec![], ids.clone()];
    d.roles = [vec![], vec![dslot::SAM, dslot::MIKE, dslot::WILL, dslot::CBL, dslot::CBR, dslot::NCB, dslot::DCB]];
    let mut o = play(PlayType::Run, 0);
    o.on_field = [vec![50, 51, 52, 53], vec![]];
    o.roles = [vec![orole::LG, orole::RG, orole::RB, orole::FB], vec![]];
    let plays = vec![d, o];
    let at = |key: &str, side: usize, id: u32| {
        let spot = Spot::KEYS.iter().find(|x| x.0 == key).unwrap().1;
        Usage { what: What::Snaps, spot, share: true, ..Usage::default() }.count(&plays, side, id).n
    };
    let one = [
        ("sam", 40),
        ("mike", 41),
        ("will", 42),
        ("left_corner", 43),
        ("right_corner", 44),
        ("nickel", 45),
        ("dime", 46),
    ];
    for (k, id) in one {
        for &other in &ids {
            assert_eq!(at(k, 1, other), if other == id { 1.0 } else { 0.0 }, "{k} and {other}");
        }
    }
    for (k, a, b) in [("outside_lb", 40, 42), ("corner", 43, 44), ("inside_corner", 45, 46)] {
        assert_eq!((at(k, 1, a), at(k, 1, b), at(k, 1, 41)), (1.0, 1.0, 0.0), "{k}");
    }
    for (k, id) in [("left_guard", 50), ("right_guard", 51), ("tailback", 52), ("fullback", 53)] {
        for other in 50..54 {
            assert_eq!(at(k, 0, other), if other == id { 1.0 } else { 0.0 }, "{k} and {other}");
        }
    }
    assert_eq!((at("guard", 0, 50), at("guard", 0, 51), at("backfield", 0, 53)), (1.0, 1.0, 1.0));
}

#[test]
fn the_line_chart_measures_rushes_stunts_blitzes_and_help() {
    let (edge, lb, lt, dt) = (20u32, 21u32, 30u32, 22u32);
    let mut plays = Vec::new();
    for k in 0..8 {
        let mut r = play(PlayType::Pass, 0);
        r.passer = Some(1);
        r.on_field = [vec![1, lt], vec![edge, lb, dt]];
        r.roles = [vec![orole::QB, orole::LT], vec![]];
        let mut ch = LineChart::default();
        let stunt = if k < 2 { Rusher::STUNT } else { 0 };
        let chip = if k == 3 { Rusher::CHIPPED } else { 0 };
        ch.rush.push(Rusher { id: edge, gap: 0, flags: Rusher::LINEMAN | stunt | chip });
        ch.rush.push(Rusher { id: dt, gap: 2, flags: Rusher::LINEMAN });
        if k % 4 == 0 {
            ch.rush.push(Rusher { id: lb, gap: 1, flags: 0 });
        }
        let double = if k == 5 { Rep::DOUBLE } else { 0 };
        ch.reps.push(Rep { blocker: lt, defender: edge, flags: Rep::PASS | double, ..Rep::default() });
        r.line = Some(Box::new(ch));
        plays.push(r);
    }
    let share = |what: What| Usage { what, share: true, ..Usage::default() };
    let c = share(What::Stunts).count(&plays, 1, edge);
    assert_eq!((c.n, c.of), (2.0, 8.0), "2 stunts in 8 rushes");
    assert_eq!(share(What::Stunts).game(&plays, 1, edge, true), Some(true), "0 >= 0");
    let quarter = Usage { n: 0.25, ..share(What::Stunts) };
    assert_eq!(quarter.game(&plays, 1, edge, true), Some(true), "1 in 4");
    let half = Usage { n: 0.5, ..share(What::Stunts) };
    assert_eq!(half.game(&plays, 1, edge, true), Some(false));
    let c = share(What::Blitzes).count(&plays, 1, lb);
    assert_eq!((c.n, c.of), (2.0, 8.0), "the linebacker came twice");
    assert_eq!(share(What::Blitzes).count(&plays, 1, edge).n, 0.0, "a down lineman never blitzes");
    let c = share(What::Help).count(&plays, 0, lt);
    assert_eq!((c.n, c.of), (2.0, 8.0), "a chip on his man and a double");
    assert_eq!(share(What::Chips).count(&plays, 1, edge).n, 1.0);
    assert!(Usage { what: What::Chips, ..Usage::default() }.shown_only());
}

#[test]
fn the_readers_labels_make_a_term_only_where_it_fits() {
    // The kicker's line, said as the try's length.
    let u = usage::read(Position::K, "fg_line", None, None, false, "any", 0, 0, 52, 0, 0).unwrap();
    assert_eq!(u, Usage::fg_line(52));
    assert!(u.words().contains("inside the 35") && u.words().contains("52-yard"), "{}", u.words());
    // Stunts for an EDGE with his number as a percent; never for a QB.
    let u = usage::read(Position::EDGE, "stunts", Some(30.0), Some(true), false, "any", 0, 0, 0, 0, 0).unwrap();
    assert!((u.n - 0.3).abs() < 1e-6 && u.share);
    assert!(usage::read(Position::QB, "stunts", None, None, false, "any", 0, 0, 0, 0, 0).is_none());
    // No number said: the agreed 1 in 4.
    let u = usage::read(Position::LB, "blitzes", None, None, false, "any", 0, 0, 0, 0, 0).unwrap();
    assert!((u.n - 0.25).abs() < 1e-6);
    // Deep shots: targets with 20+ air yards, 2 a game.
    let u = usage::read(Position::WR, "targets", Some(2.0), Some(false), false, "any", 0, 0, 20, 0, 0).unwrap();
    assert_eq!((u.what, u.far, u.n, u.share), (What::Targets, 20, 2.0, false));
    assert!(u.words().contains("deep targets"), "{}", u.words());
    // A defensive spot named for a receiver is dropped, not misread.
    let u = usage::read(Position::WR, "snaps", Some(60.0), Some(true), false, "nickel", 0, 0, 0, 0, 0).unwrap();
    assert_eq!(u.spot, Spot::Any);
    // Chips are never a term.
    assert!(usage::read(Position::EDGE, "chips", None, None, false, "any", 0, 0, 0, 0, 0).is_none());
    // Every measure says what it means and fits somebody.
    for m in usage::MEASURES {
        assert!(!m.means.is_empty() && !m.spots.is_empty(), "{}", m.key);
    }
}

// ---------------------------------------------------------------------------
// The plan settings on the engine.

fn league(seed: u32) -> Vec<GameTeam> {
    synthetic_league(Level::NFL, 4, seed)
}

fn games(home: &GameTeam, away: &GameTeam, n: u64, seed: u64) -> Vec<GameResult> {
    (0..n)
        .map(|k| {
            let mut s = GameSetup::new(home, away, Level::NFL, seed + k);
            s.keep = true;
            play_game(&s)
        })
        .collect()
}

/// A term's count over games (home side), pooled.
fn pooled(rs: &[GameResult], u: &Usage, id: u32) -> (f64, f64) {
    rs.iter().map(|r| u.count(&r.stats.plays, 0, id)).fold((0.0, 0.0), |a, c| (a.0 + c.n, a.1 + c.of))
}

/// The home man at `pos` with the most of this term in these games.
fn busiest(t: &GameTeam, rs: &[GameResult], pos: Position, u: &Usage) -> u32 {
    t.players
        .iter()
        .filter(|p| p.position == pos)
        .map(|p| (p.id, pooled(rs, u, p.id).1))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|x| x.0)
        .unwrap()
}

fn shape(r: &GameResult) -> Vec<(u32, String, i64)> {
    r.stats.plays.iter().map(|p| (p.n, p.call.clone(), (p.yards * 100.0).round() as i64)).collect()
}

#[test]
fn a_plan_that_keeps_nothing_changes_no_game_and_chips_are_saved() {
    let t = league(71);
    let base = games(&t[0], &t[1], 6, 900);
    // A plan naming nobody on the field draws nothing and moves nothing.
    let mut home = t[0].clone();
    home.prep.keep.free_up = vec![u32::MAX];
    home.prep.keep.send = vec![u32::MAX];
    home.prep.keep.shots = vec![u32::MAX];
    home.prep.keep.help = vec![u32::MAX];
    let same = games(&home, &t[1], 6, 900);
    for (a, b) in base.iter().zip(&same) {
        assert_eq!(shape(a), shape(b));
    }
    // The other side's plans chip strong rushers: the chart saves them.
    let chips = base
        .iter()
        .flat_map(|r| &r.stats.plays)
        .filter_map(|p| p.line.as_deref())
        .flat_map(|c| &c.rush)
        .filter(|q| q.chipped())
        .count();
    assert!(chips > 0, "chips are saved");
}

#[test]
fn free_him_up_and_send_him_move_his_rushes() {
    let t = league(72);
    let base = games(&t[0], &t[1], 24, 2_000);
    let stunts = Usage { what: What::Stunts, share: true, ..Usage::default() };
    let blitz = Usage { what: What::Blitzes, share: true, ..Usage::default() };
    let edge = busiest(&t[0], &base, Position::EDGE, &stunts);
    let lb = busiest(&t[0], &base, Position::LB, &blitz);
    let mut home = t[0].clone();
    home.prep.keep.free_up = vec![edge];
    home.prep.keep.send = vec![lb];
    let kept = games(&home, &t[1], 24, 2_000);
    let rate = |rs: &[GameResult], u: &Usage, id: u32| {
        let (n, of) = pooled(rs, u, id);
        n / of.max(1.0)
    };
    let (s0, s1) = (rate(&base, &stunts, edge), rate(&kept, &stunts, edge));
    let (b0, b1) = (rate(&base, &blitz, lb), rate(&kept, &blitz, lb));
    // Judged as agreed: at least 1 in 4 in a game, kept in 3 of every 4
    // games that say anything.
    let kept_share = |u: &Usage, id: u32| {
        let u = Usage { n: 0.25, ..*u };
        let g: Vec<bool> = kept.iter().filter_map(|r| u.game(&r.stats.plays, 0, id, true)).collect();
        g.iter().filter(|&&k| k).count() as f64 / g.len().max(1) as f64
    };
    let (sk, bk) = (kept_share(&stunts, edge), kept_share(&blitz, lb));
    eprintln!("stunts {s0:.3} -> {s1:.3} (games kept {sk:.2}); blitzes {b0:.3} -> {b1:.3} (games kept {bk:.2})");
    assert!(s1 > s0 + 0.15 && sk >= promise::KEPT_SHARE, "freed up: stunts {s0:.3} -> {s1:.3}, kept {sk:.2}");
    assert!(b1 > b0 + 0.15 && bk >= promise::KEPT_SHARE, "sent: blitzes {b0:.3} -> {b1:.3}, kept {bk:.2}");
}

#[test]
fn the_field_goal_line_ends_punts_inside_it() {
    let t = league(73);
    let line = Usage::fg_line(62);
    let punts = |rs: &[GameResult]| rs.iter().map(|r| line.count(&r.stats.plays, 0, 0).n).sum::<f64>();
    let base = games(&t[0], &t[1], 40, 3_000);
    let mut home = t[0].clone();
    home.prep.keep.fg_line = line.inside;
    let kept = games(&home, &t[1], 40, 3_000);
    eprintln!("punts inside the {}: {} -> {}", line.inside, punts(&base), punts(&kept));
    assert!(punts(&base) > 0.0, "the staff punts from there on its own");
    assert_eq!(punts(&kept), 0.0, "the line holds");
}

#[test]
fn shots_and_help_go_to_the_man_you_promised() {
    let t = league(74);
    let base = games(&t[0], &t[1], 24, 4_000);
    let deep = Usage { what: What::Targets, far: 20, ..Usage::default() };
    let help = Usage { what: What::Help, share: true, ..Usage::default() };
    let wrs: Vec<u32> = t[0].players.iter().filter(|p| p.position == Position::WR).map(|p| p.id).collect();
    // The receiver on the field most.
    let snaps = Usage { what: What::Snaps, ..Usage::default() };
    let wr =
        wrs.iter().copied().max_by(|&a, &b| pooled(&base, &snaps, a).0.total_cmp(&pooled(&base, &snaps, b).0)).unwrap();
    let lt = busiest(&t[0], &base, Position::OT, &help);
    let mut home = t[0].clone();
    home.prep.keep.shots = vec![wr];
    home.prep.keep.help = vec![lt];
    let kept = games(&home, &t[1], 24, 4_000);
    let (d0, d1) = (pooled(&base, &deep, wr).0, pooled(&kept, &deep, wr).0);
    let rate = |rs: &[GameResult]| {
        let (n, of) = pooled(rs, &help, lt);
        n / of.max(1.0)
    };
    eprintln!("deep targets {d0} -> {d1}; help {:.3} -> {:.3}", rate(&base), rate(&kept));
    assert!(d1 > d0 * 1.3 + 2.0, "shots: {d0} -> {d1}");
    assert!(rate(&kept) > rate(&base) + 0.1, "help: {:.3} -> {:.3}", rate(&base), rate(&kept));
}

// ---------------------------------------------------------------------------
// Judged through the agreed promise rules.

fn room() -> (World, usize, u32) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let you = w.career.as_ref().map(|c| c.coach).unwrap() as u32;
    let q = &mut w.people[i];
    q.stage = Stage::College;
    q.college = Some(p);
    q.class_of = year - 1;
    q.college_seasons = 1;
    q.last_role = Some(PlayingTime::Starter);
    q.player.position = Position::EDGE;
    q.trust = 0.6;
    q.trust_in = you;
    let id = q.player.id;
    w.programs[p].roster.push(i);
    let arrived = w.today.add(-300);
    w.training.arrived.push((id, arrived));
    (w, p, id)
}

/// One of your games: his rushes, `stunts` of them in a stunt, and a punt
/// from `punt_at` when given.
fn game(w: &mut World, p: usize, d: Date, id: u32, rushes: u32, stunts: u32, punt_at: Option<f64>) {
    let opp = (0..w.programs.len()).find(|&j| j != p).unwrap();
    let mut stats = GameStats::default();
    stats.players[0].insert(id, StatLine { snaps_def: rushes + 10, games: 1, ..Default::default() });
    for k in 0..rushes {
        let mut r = play(PlayType::Pass, 1);
        r.passer = Some(1);
        r.on_field = [vec![1], vec![id]];
        let flags = Rusher::LINEMAN | if k < stunts { Rusher::STUNT } else { 0 };
        r.line = Some(Box::new(LineChart { rush: vec![Rusher { id, gap: 0, flags }], ..LineChart::default() }));
        stats.plays.push(r);
    }
    if let Some(y) = punt_at {
        let mut r = play(PlayType::Punt, 0);
        r.yardline_100 = y;
        stats.plays.push(r);
    }
    w.seat_games.push(SeatGame {
        date: d,
        level: CalLevel::College,
        teams: [p, opp],
        names: [w.program_name(p).to_string(), w.program_name(opp).to_string()],
        stats,
        replay: Vec::new(),
        marks: Vec::new(),
        gone: Vec::new(),
        spread: None,
        talks: Default::default(),
        whistle: None,
        mine: None,
        hurt: Vec::new(),
    });
}

#[test]
fn a_term_is_judged_game_by_game_and_never_breaks_at_once() {
    let (mut w, p, id) = room();
    let today = w.today;
    // Stunts on 1 in 4 of his rushes.
    let u = Usage { what: What::Stunts, share: true, n: 0.25, ..Usage::default() };
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Usage, 0);
    let k = w.folk.promises.len() - 1;
    talks::set_terms(&mut w, k, Terms { usage: Some(u), ..Terms::default() });
    assert!(w.folk.promises[k].belief > 0.0, "he heard it");
    assert!(w.folk.promises[k].words().contains("stunt"), "{}", w.folk.promises[k].words());
    // The plan sets itself from it.
    assert_eq!(usage::keep(&w).free_up, vec![id]);
    // Two games with them, then two without.
    for (n, s) in [8, 7, 1, 0].into_iter().enumerate() {
        let d = today.add(7 * n as i32 + 5);
        game(&mut w, p, d, id, 24, s, None);
        w.today = d.add(2);
        promise::week(&mut w, d.add(2));
        let st = State::of(w.folk.promises[k].state);
        assert_eq!(st, if s >= 6 { State::OnTrack } else { State::Behind }, "game {n}");
    }
    let pr = w.folk.promises[k].clone();
    assert_eq!(promise::judge(&w, &pr), Some(Verdict::Broken(0.5)));
    // The kicker's line: one punt inside it breaks it, that week.
    let line = Usage::fg_line(50);
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Usage, 0);
    let k2 = w.folk.promises.len() - 1;
    talks::set_terms(&mut w, k2, Terms { usage: Some(line), ..Terms::default() });
    assert_eq!(usage::keep(&w).fg_line, 33);
    let d = w.today.add(5);
    game(&mut w, p, d, id, 10, 3, Some(45.0));
    w.today = d.add(2);
    promise::week(&mut w, d.add(2));
    let pr = w.folk.promises[k2].clone();
    assert_eq!(promise::judge(&w, &pr), Some(Verdict::Kept), "a punt from their 45 is outside the line");
    let d = w.today.add(5);
    game(&mut w, p, d, id, 10, 3, Some(31.0));
    w.today = d.add(2);
    promise::week(&mut w, d.add(2));
    let pr = w.folk.promises[k2].clone();
    assert_eq!(promise::judge(&w, &pr), Some(Verdict::Broken(1.0)), "one punt inside the line");
    assert!(pr.due <= w.today, "judged that week, not at the season's end");
}
