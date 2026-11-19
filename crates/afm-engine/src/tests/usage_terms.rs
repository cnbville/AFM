//! The rest of the football promises (live talks v3, lane N2; spec 8.4):
//! the six readers kept on every snap, the high school box score judge,
//! the plan settings that keep each term on the engine (and change nothing
//! for a plan without one), turning a setting off and the staff's warning,
//! and terms on the season (incentives, NIL clauses, conditional picks,
//! staff and boss goals).

use std::collections::BTreeMap;

use super::legacy::seat;
use crate::game::playbook::Coverage;
use crate::game::roster::GameTeam;
use crate::game::stats::StatLine;
use crate::game::synthetic::synthetic_league;
use crate::game::{GameResult, GameSetup, PlayRecord, PlayType, play_game};
use crate::player::Level;
use crate::positions::Position;
use crate::world::Stage;
use crate::world::folk::boss::{GoalKind, goal_met};
use crate::world::folk::promise::Terms;
use crate::world::folk::talks::{self, PromiseKind, PromiseTo};
use crate::world::folk::terms::{self, Clause, On, Who};
use crate::world::folk::usage::{self, Spot, Usage, What};
use crate::world::folk::{Team, facts};
use crate::world::front_office::deals::Condition;
use crate::world::pro::money::{self, Incentive};
use crate::world::program::ProgramSeason;
use crate::world::tally::depth::{dslot, orole};

fn play(kind: PlayType, offense: usize) -> PlayRecord {
    PlayRecord::new(1, 1, 600.0, offense, kind)
}

fn share(what: What, kind: u8) -> Usage {
    Usage { what, kind, share: true, ..Usage::default() }
}

#[test]
fn the_six_readers_read_the_play_log() {
    let (qb, rb, lg, wr, cb, s, lb, k) = (1u32, 2u32, 3u32, 4u32, 5u32, 6u32, 7u32, 8u32);
    let mut plays = Vec::new();
    // Three snaps with him at quarterback: full audibles, a kill call, none.
    for f in [3u8, 2, 1] {
        let mut r = play(PlayType::Pass, 0);
        r.passer = Some(qb);
        r.on_field = [vec![qb, lg, wr], vec![]];
        r.roles = [vec![orole::QB, orole::LG, orole::SLOT], vec![]];
        r.freedom = f;
        r.call = "11 gun doubles four verticals".into();
        plays.push(r);
    }
    // Runs: his inside zone and power, a QB power; the guard is on for all.
    for (call, carrier) in [
        ("11 gun doubles inside zone left", rb),
        ("12 under center wing power right", rb),
        ("11 gun trips QB power left", qb),
    ] {
        let mut r = play(PlayType::Run, 0);
        r.call = call.into();
        r.carrier = Some(carrier);
        r.on_field = [vec![qb, rb, lg], vec![]];
        r.roles = [vec![orole::QB, orole::RB, orole::LG], vec![]];
        plays.push(r);
    }
    // Targets: a slant-flat and an RPO are quick; four verticals isn't.
    for (call, kind) in [
        ("11 gun trips slant-flat", PlayType::Pass),
        ("11 gun doubles inside zone with RPO slant", PlayType::Pass),
        ("11 gun doubles four verticals", PlayType::Pass),
    ] {
        let mut r = play(kind, 0);
        r.call = call.into();
        r.passer = Some(qb);
        r.target = Some(wr);
        r.on_field = [vec![qb, wr], vec![]];
        r.roles = [vec![orole::QB, orole::SLOT], vec![]];
        plays.push(r);
    }
    // Defense: the corner on the left hash at left corner (the boundary),
    // the right hash at left corner (the field), the middle (says nothing);
    // the safety at free in Cover 3 (the post), at strong in Cover 1 robber
    // (the robber), Cover 4 (two high); the linebacker spying at Mike.
    for (hash, cov) in
        [(1u8, "Cover 3, four-man rush"), (2, "Cover 1 robber, four-man rush"), (3, "Cover 4, four-man rush, QB spy")]
    {
        let mut r = play(PlayType::Pass, 1);
        r.passer = Some(99);
        r.hash = hash;
        r.defense_call = cov.into();
        let safety = if cov.starts_with("Cover 1 robber") { dslot::SS } else { dslot::FS };
        r.on_field = [vec![99], vec![cb, s, lb]];
        r.roles = [vec![orole::QB], vec![dslot::CBL, safety, dslot::MIKE]];
        plays.push(r);
    }
    // Kicks: his field goal, another man's extra point.
    let mut fg = play(PlayType::FieldGoal, 0);
    fg.kicker = Some(k);
    let mut xp = play(PlayType::ExtraPoint, 0);
    xp.kicker = Some(77);
    plays.push(fg);
    plays.push(xp);

    // Every man here is on team 0: the defense's snaps are team 1's plays.
    let c = |u: Usage, id: u32| {
        let c = u.count(&plays, 0, id);
        (c.n, c.of)
    };
    assert_eq!(c(share(What::Freedom, 3), qb), (1.0, 3.0), "full audibles once");
    assert_eq!(c(share(What::Freedom, 2), qb), (2.0, 3.0), "a kill call or more twice");
    assert_eq!(c(share(What::RunFamily, 1), rb), (1.0, 2.0), "the back's inside zone");
    assert_eq!(c(share(What::RunFamily, 3), rb), (1.0, 2.0), "the back's power");
    assert_eq!(c(share(What::RunFamily, 3), lg), (2.0, 3.0), "the guard: power and QB power");
    assert_eq!(c(Usage { what: What::QuickTargets, ..Usage::default() }, wr).0, 2.0);
    assert_eq!(c(share(What::Side, 1), cb), (1.0, 2.0), "the boundary once, the middle says nothing");
    assert_eq!(c(share(What::Side, 2), cb), (1.0, 2.0));
    assert_eq!(c(share(What::Post, 0), s), (1.0, 3.0));
    assert_eq!(c(share(What::Robber, 0), s), (1.0, 3.0));
    assert_eq!(c(share(What::TwoHigh, 0), cb), (1.0, 3.0));
    assert_eq!(c(share(What::Spy, 0), lb), (1.0, 3.0));
    assert_eq!(c(share(What::PlaceKicks, 0), k), (1.0, 2.0));
    // The call's words read back.
    assert_eq!(usage::coverage_of("Cover 1 robber, five-man rush"), Some(Coverage::Cover1Robber));
    assert_eq!(usage::coverage_of("Cover 1, four-man rush"), Some(Coverage::Cover1));
    assert_eq!(usage::run_of("11 gun trips QB power left").map(|r| r.name()), Some("QB power"));
    // The reader's labels: kinds from the key, every new key fits someone.
    let u = usage::read(Position::QB, "audibles", None, None, false, "any", 0, 0, 0, 0, 0).unwrap();
    assert_eq!((u.what, u.kind), (What::Freedom, 3));
    assert!(u.words().contains("full audibles"), "{}", u.words());
    let u = usage::read(Position::IOL, "gap_scheme", Some(50.0), None, false, "any", 0, 0, 0, 0, 0).unwrap();
    assert_eq!((u.what, u.kind, u.n), (What::RunFamily, 3, 0.5));
    assert!(u.words().contains("gap scheme"), "{}", u.words());
    assert!(usage::read(Position::WR, "boundary", None, None, false, "any", 0, 0, 0, 0, 0).is_none());
    let u = usage::read(Position::CB, "field_side", None, None, false, "any", 0, 0, 0, 0, 0).unwrap();
    assert!(u.words().contains("field side"), "{}", u.words());
}

#[test]
fn a_high_school_box_score_judges_its_counts() {
    let (me, mate) = (10u32, 11u32);
    let mut box_lines = BTreeMap::new();
    box_lines.insert(me, StatLine { rush: 12, targets: 3, snaps_off: 40, kr: 2, ..Default::default() });
    box_lines.insert(mate, StatLine { rush: 8, snaps_off: 50, kr: 2, ..Default::default() });
    let carries = Usage { what: What::Carries, n: 10.0, ..Usage::default() };
    let c = carries.box_count(&box_lines, me).unwrap();
    assert_eq!((c.n, c.of), (12.0, 1.0));
    assert_eq!(carries.judge(c), Some(true));
    let half = Usage { share: true, n: 0.5, ..carries };
    let c = half.box_count(&box_lines, me).unwrap();
    assert_eq!((c.n, c.of), (12.0, 20.0), "his share of the team's carries");
    let snaps = Usage { what: What::Snaps, share: true, n: 0.9, ..Usage::default() };
    let c = snaps.box_count(&box_lines, me).unwrap();
    assert_eq!((c.n, c.of), (40.0, 50.0), "against the most any man played");
    assert_eq!(snaps.judge(c), Some(false));
    let touches = Usage { what: What::Touches, n: 15.0, ..Usage::default() };
    assert_eq!(touches.judge(touches.box_count(&box_lines, me).unwrap()), Some(true));
    // A situation, a spot, the line chart or an empty box say nothing.
    assert!(Usage { inside: 20, ..carries }.box_count(&box_lines, me).is_none());
    assert!(Usage { spot: Spot::Tailback, ..carries }.box_count(&box_lines, me).is_none());
    assert!(Usage { what: What::Stunts, ..carries }.box_count(&box_lines, me).is_none());
    assert!(carries.box_count(&BTreeMap::new(), me).is_none());
}

// ---------------------------------------------------------------------------
// The plan settings on the engine.

fn games(home: &GameTeam, away: &GameTeam, n: u64, seed: u64) -> Vec<GameResult> {
    (0..n)
        .map(|k| {
            let mut s = GameSetup::new(home, away, Level::NFL, seed + k);
            s.keep = true;
            play_game(&s)
        })
        .collect()
}

fn pooled(rs: &[GameResult], u: &Usage, id: u32, side: usize) -> (f64, f64) {
    rs.iter().map(|r| u.count(&r.stats.plays, side, id)).fold((0.0, 0.0), |a, c| (a.0 + c.n, a.1 + c.of))
}

fn rate(rs: &[GameResult], u: &Usage, id: u32) -> f64 {
    let (n, of) = pooled(rs, u, id, 0);
    n / of.max(1.0)
}

/// The home man at `pos` on the field most.
fn regular(t: &GameTeam, rs: &[GameResult], pos: Position) -> u32 {
    let snaps = Usage { what: What::Snaps, ..Usage::default() };
    t.players
        .iter()
        .filter(|p| p.position == pos)
        .map(|p| (p.id, pooled(rs, &snaps, p.id, 0).0))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap()
        .0
}

fn shape(r: &GameResult) -> Vec<(u32, String, String, i64)> {
    r.stats
        .plays
        .iter()
        .map(|p| (p.n, p.call.clone(), p.defense_call.clone(), (p.yards * 100.0).round() as i64))
        .collect()
}

#[test]
fn a_plan_naming_nobody_changes_no_game() {
    let t = synthetic_league(Level::NFL, 4, 81);
    let base = games(&t[0], &t[1], 6, 5_000);
    let mut home = t[0].clone();
    let k = &mut home.prep.keep;
    let nobody = u32::MAX;
    k.freedom = vec![(nobody, 3)];
    k.on = vec![nobody];
    k.release = vec![nobody];
    k.screens = vec![nobody];
    k.quick = vec![nobody];
    k.gadget = vec![nobody];
    k.side = vec![(nobody, 1)];
    k.post = vec![nobody];
    k.robber = vec![nobody];
    k.returner = vec![nobody];
    let same = games(&home, &t[1], 6, 5_000);
    for (a, b) in base.iter().zip(&same) {
        assert_eq!(shape(a), shape(b));
    }
    // The quarterback's freedom is kept on every snap.
    assert!(base[0].stats.plays.iter().filter(|p| p.kind == PlayType::Pass).all(|p| p.freedom > 0));
}

#[test]
fn the_offense_settings_move_their_measures() {
    let t = synthetic_league(Level::NFL, 4, 82);
    let base = games(&t[0], &t[1], 16, 6_000);
    let qb = regular(&t[0], &base, Position::QB);
    let rb = regular(&t[0], &base, Position::RB);
    let te = regular(&t[0], &base, Position::TE);
    let audibles = share(What::Freedom, 3);
    let gap = share(What::RunFamily, 3);
    let routes = share(What::Routes, 0);
    let mut home = t[0].clone();
    home.prep.keep.freedom = vec![(qb, 3)];
    home.prep.keep.family = [0.0, 0.0, 1.0, 0.0];
    home.prep.keep.release = vec![te];
    home.prep.keep.qb_runs = -1;
    let kept = games(&home, &t[1], 16, 6_000);
    let (a0, a1) = (rate(&base, &audibles, qb), rate(&kept, &audibles, qb));
    let (g0, g1) = (rate(&base, &gap, rb), rate(&kept, &gap, rb));
    let (r0, r1) = (rate(&base, &routes, te), rate(&kept, &routes, te));
    eprintln!("audibles {a0:.2} -> {a1:.2}; gap {g0:.2} -> {g1:.2}; routes {r0:.2} -> {r1:.2}");
    assert!(a1 > 0.99, "full audibles on every snap: {a1:.2}");
    assert!(g1 > g0 + 0.08, "the gap scheme: {g0:.2} -> {g1:.2}");
    assert!(r1 > r0 && r1 > 0.99, "released on every dropback: {r0:.2} -> {r1:.2}");
    let sneaks_or_more = kept
        .iter()
        .flat_map(|r| &r.stats.plays)
        .filter(|p| p.offense == 0 && p.kind == PlayType::Run && p.carrier == Some(qb) && !p.scramble)
        .filter(|p| usage::run_of(&p.call).is_some_and(crate::game::calling::qb_designed))
        .count();
    assert_eq!(sneaks_or_more, 0, "no designed runs");
}

#[test]
fn the_defense_settings_move_their_measures() {
    let t = synthetic_league(Level::NFL, 4, 83);
    let base = games(&t[1], &t[0], 16, 7_000);
    // The home side defends on the away team's snaps: read side 0's men.
    let cb = regular(&t[1], &base, Position::CB);
    let s = regular(&t[1], &base, Position::S);
    let boundary = share(What::Side, 1);
    let post = share(What::Post, 0);
    let two = share(What::TwoHigh, 0);
    let mut home = t[1].clone();
    home.prep.keep.side = vec![(cb, 1)];
    home.prep.keep.post = vec![s];
    home.prep.keep.one_high = 0.6;
    let kept = games(&home, &t[0], 16, 7_000);
    let (b0, b1) = (rate(&base, &boundary, cb), rate(&kept, &boundary, cb));
    let (p0, p1) = (rate(&base, &post, s), rate(&kept, &post, s));
    eprintln!("boundary {b0:.2} -> {b1:.2}; post {p0:.2} -> {p1:.2}");
    assert!(b1 > 0.97, "the boundary corner by the hash: {b0:.2} -> {b1:.2}");
    assert!(p1 > p0 + 0.1, "the post safety: {p0:.2} -> {p1:.2}");
    // Two-high to his side: the floor holds, on 48 games a side (16 swung
    // the share about 0.03). Since play action comes off run looks
    // (scheme-fit item 3), the coordinator leans off two-high against this
    // offense's fakes and runs, so the share under the floor is about 0.87
    // (0.91 before).
    let base = games(&t[1], &t[0], 48, 7_000);
    let mut home = t[1].clone();
    home.prep.keep.two_high = 0.95;
    let kept = games(&home, &t[0], 48, 7_000);
    let (h0, h1) = (rate(&base, &two, cb), rate(&kept, &two, cb));
    eprintln!("two-high {h0:.2} -> {h1:.2}");
    assert!(h1 > h0 + 0.08 && h1 > 0.85, "two-high: {h0:.2} -> {h1:.2}");
}

// ---------------------------------------------------------------------------
// Turning a setting off, the warning, and the season terms in a world.

#[test]
fn a_setting_turned_off_sets_nothing_and_the_staff_warns() {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let year = w.year;
    let q = &mut w.people[i];
    q.stage = Stage::College;
    q.college = Some(p);
    q.class_of = year - 1;
    q.player.position = Position::QB;
    let id = q.player.id;
    w.programs[p].roster.push(i);
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Usage, 0);
    let k = w.folk.promises.len() - 1;
    let u = share(What::Freedom, 3);
    talks::set_terms(&mut w, k, Terms { usage: Some(u), ..Terms::default() });
    assert_eq!(usage::keep(&w).freedom, vec![(id, 3)]);
    usage::set_off(&mut w, k, true).unwrap();
    assert!(usage::keep(&w).freedom.is_empty(), "off: the plan sets nothing");
    let warned = usage::warnings(&w);
    assert!(warned.iter().any(|x| x.contains("turned off")), "{warned:?}");
    usage::set_off(&mut w, k, false).unwrap();
    assert_eq!(usage::keep(&w).freedom, vec![(id, 3)]);
    assert!(usage::set_off(&mut w, 999_999, true).is_err());
}

#[test]
fn season_terms_read_the_record() {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    let team = Team::College(p);
    // Last season: his program 8-4 with the most points in its division,
    // another program 6-6.
    let other = (0..w.programs.len()).find(|&j| j != p).unwrap();
    let div = w.programs[p].division;
    for (k, wins, pts) in [(p, 8u32, 400u32), (other, 6, 300)] {
        w.programs[k].seasons.push(ProgramSeason {
            year: w.year - 1,
            wins,
            losses: 12 - wins,
            conference_title: false,
            national_title: false,
            coach: None,
            strength: 0.0,
            totals: Default::default(),
            conference_won: None,
            conference: None,
            division: Some(div),
            points: [pts, 280],
        });
    }
    let last = facts::seasons(&w, team, 1).pop().expect("a finished season");
    // Wins: met at what he won, missed one past it.
    let wins = terms::read(None, "wins", Some(last.wins as f32), false).unwrap();
    assert!(wins.season && wins.team());
    assert_eq!(terms::met(&w, Who::Team(team), &wins, last.year), Some(true));
    let more = Usage { n: last.wins as f32 + 1.0, ..wins };
    assert_eq!(terms::met(&w, Who::Team(team), &more, last.year), Some(false));
    // A rank in its division: the best scoring offense is first.
    let r = terms::rank(&w, team, 1, last.year);
    assert_eq!(r, Some(1), "the most points in the division");
    if let Some(r) = r {
        let top = terms::read(None, "scoring_offense", Some(r as f32), false).unwrap();
        assert_eq!(terms::met(&w, Who::Team(team), &top, last.year), Some(true));
        assert!(terms::words(&top).contains("scoring offense"), "{}", terms::words(&top));
        // As a boss's goal.
        assert!(goal_met(&w, team, GoalKind::Measure(top), last.year));
    }
    // Player terms need a player; team terms refuse one.
    assert!(terms::read(None, "sacks", Some(8.0), false).is_none());
    let sacks = terms::read(Some(Position::EDGE), "sacks", Some(8.0), false).unwrap();
    assert_eq!((sacks.what, sacks.kind), (What::Stat, 12));
    assert!(terms::words(&sacks).contains("8 sacks"), "{}", terms::words(&sacks));
    assert!(sacks.game(&[], 0, 1, true).is_none(), "never judged game by game");
    // A season snap share for a pick's condition.
    let snaps = terms::read(Some(Position::WR), "snaps", Some(60.0), false).unwrap();
    assert!(snaps.season && snaps.share && (snaps.n - 0.6).abs() < 1e-6);
    let cond = Condition::Term(snaps);
    assert!(cond.words().contains("60%"), "{}", cond.words());
    // A clause the record can read settles after the season and is noted.
    let today = w.today;
    terms::add(
        &mut w,
        Clause {
            on: On::Staff,
            who: Who::Team(team),
            coach: None,
            term: wins,
            season: last.year,
            pays: 25_000.0,
            made: today,
            met: None,
            settled: false,
        },
    );
    let sb = w.dates.super_bowl;
    terms::day(&mut w, sb);
    let c = w.folk.clauses.last().unwrap();
    assert!(c.settled && c.met == Some(true));
    assert!(w.folk.log.last().is_some_and(|e| e.text.contains("met the contract goal")), "noted");
    // An incentive: a likely one counts on the cap now.
    let mut deal = money::build(0, money::Kind::Veteran, w.year, &money::Terms::flat(2, 5_000_000.0));
    let before = deal.cap_hit(w.year);
    deal.add_incentive(Incentive { year: w.year, term: sacks, pays: 500_000.0, likely: true, earned: None });
    assert!((deal.cap_hit(w.year) - before - 500_000.0).abs() < 1.0);
    deal.add_incentive(Incentive { year: w.year, term: sacks, pays: 500_000.0, likely: false, earned: None });
    assert!((deal.cap_hit(w.year) - before - 500_000.0).abs() < 1.0, "an unlikely one waits until earned");
}
