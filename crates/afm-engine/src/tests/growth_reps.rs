//! Growth from the depth chart (minor bug 247, lane NB12): your depth
//! chart sets your players' roles for the year's step, a step reads last
//! season's real snaps at its own level, and the snaps move a year's
//! growth as the NFL's do.

use super::{career, context_for, mean, prospects};
use crate::development::develop_season;
use crate::player::{Level, Player, PlayingTime, ProspectOptions, generate_prospect};
use crate::positions::Position;
use crate::results::{self, Dice, Ledger, Man, Mode, Read};
use crate::rng::Rng;
use crate::world::season::{depth_chart, reps_read, sat_season, your_order};
use crate::world::{Reps, Stage};

fn role_of(chart: &[(usize, PlayingTime)], i: usize) -> PlayingTime {
    chart.iter().find(|c| c.0 == i).unwrap().1
}

#[test]
fn your_depth_chart_sets_the_roles_on_your_team() {
    use PlayingTime::*;
    // Quarterbacks: a senior, a junior and a true freshman the staff means
    // to redshirt. Receivers: seven, the last a redshirt too. Corners: three.
    let mut entries = vec![
        (0, Position::QB, 70.0),
        (1, Position::QB, 65.0),
        (2, Position::QB, 60.0),
        (3, Position::WR, 72.0),
        (4, Position::WR, 70.0),
        (5, Position::WR, 68.0),
        (6, Position::WR, 66.0),
        (7, Position::WR, 64.0),
        (8, Position::WR, 55.0),
        (12, Position::WR, 62.0),
        (9, Position::CB, 71.0),
        (10, Position::CB, 69.0),
        (11, Position::CB, 60.0),
    ];
    let mut chart = depth_chart(&mut entries, None, &|_| false, &|_| false);
    for c in chart.iter_mut().filter(|c| c.0 == 2 || c.0 == 8) {
        c.1 = Redshirt;
    }
    let staff = chart.clone();
    assert_eq!((role_of(&staff, 0), role_of(&staff, 1)), (Starter, Rotation));
    let id = |i: usize| 100 + i as u32;
    // You start the freshman, list the fifth receiver as your top corner,
    // and order the receivers with the redshirt seventh, past the two-deep.
    let order = vec![
        (Position::QB, vec![id(2)]),
        (Position::CB, vec![id(7)]),
        (Position::WR, vec![id(3), id(4), id(5), id(6), id(7), id(12), id(8)]),
    ];
    your_order(&mut chart, &entries, &order, &id);
    // The freshman starts and isn't redshirted; the senior drops to second.
    assert_eq!(role_of(&chart, 2), Starter);
    assert_eq!(role_of(&chart, 0), Rotation);
    assert_eq!(role_of(&chart, 1), Backup);
    // The fifth receiver starts at corner; the staff's two corners move down.
    assert_eq!(role_of(&chart, 7), Starter);
    assert_eq!(role_of(&chart, 9), Starter);
    assert_eq!(role_of(&chart, 10), Rotation);
    assert_eq!(role_of(&chart, 11), Backup);
    // A redshirt you list past the two-deep keeps the staff's call; one you
    // list inside it plays.
    assert_eq!(role_of(&chart, 8), Redshirt);
    let mut inside = staff.clone();
    your_order(&mut inside, &entries, &[(Position::WR, vec![id(3), id(4), id(5), id(6), id(8)])], &id);
    assert_eq!(role_of(&inside, 8), Backup);
    // No order: the staff's chart stands.
    let mut same = staff.clone();
    your_order(&mut same, &entries, &[], &id);
    assert_eq!(same, staff);
    // At your high school you moved the staff's top receiver to JV: your
    // varsity order leaves him off and ranks the varsity players alone, so
    // he keeps the staff's place (his JV starter's role) and the next two
    // start on varsity.
    let varsity: Vec<_> = entries.iter().copied().filter(|e| e.0 != 3).collect();
    let mut jv = staff.clone();
    your_order(&mut jv, &varsity, &[(Position::WR, vec![id(5), id(4)])], &id);
    assert_eq!(role_of(&jv, 3), Starter);
    assert_eq!((role_of(&jv, 5), role_of(&jv, 4)), (Starter, Starter));
}

#[test]
fn a_seasons_row_says_what_he_really_played() {
    use crate::world::college::real_role;
    use PlayingTime::*;
    // Cain 2026-11-07: the camp ranking started a receiver who caught one
    // ball in ten games; his snaps came on special teams.
    assert_eq!(real_role(0, 10, 13, 0.03), Backup);
    // Started every game, or most of them.
    assert_eq!(real_role(13, 13, 13, 0.95), Starter);
    assert_eq!(real_role(7, 12, 13, 0.6), Starter);
    // Won the job at midseason: six starts in twelve games.
    assert_eq!(real_role(6, 12, 12, 0.45), Starter);
    // A starter hurt in the opener, and one hurt after four starts.
    assert_eq!(real_role(1, 1, 12, 0.05), Starter);
    assert_eq!(real_role(4, 4, 12, 0.3), Starter);
    // A third receiver the first snap's personnel left off: heavy snaps.
    assert_eq!(real_role(2, 13, 13, 0.62), Starter);
    // A backup who filled in for two starts in three games played.
    assert_eq!(real_role(2, 3, 13, 0.18), Backup);
    // A starter benched after five starts who kept playing.
    assert_eq!(real_role(5, 12, 12, 0.4), Rotation);
    // A rotation lineman: never started, a third of the snaps.
    assert_eq!(real_role(0, 12, 12, 0.35), Rotation);
}

#[test]
fn a_season_off_leaves_the_step_to_the_role() {
    use PlayingTime::*;
    // College: a planned redshirt keeps the year with four games or fewer.
    assert!(sat_season(Some(Redshirt), 4, false, 4));
    assert!(!sat_season(Some(Redshirt), 5, false, 4));
    // A suspension that kept him out of every game, not one that came after
    // he had played.
    assert!(sat_season(Some(Backup), 0, true, 4));
    assert!(!sat_season(Some(Backup), 3, true, 4));
    // A bench player who never got in reads his zero snaps.
    assert!(!sat_season(Some(Backup), 0, false, 4));
    // High school: a transfer's sit year is no games at all.
    assert!(sat_season(Some(Redshirt), 0, false, 0));
    assert!(!sat_season(Some(Redshirt), 1, false, 0));
}

#[test]
fn a_step_reads_last_seasons_reps_at_its_own_level() {
    let college = Some(Reps { year: 2030, college: true, share: 0.8 });
    let hs = Some(Reps { year: 2030, college: false, share: 0.6 });
    let read = |r: Option<Reps>, stage: Stage, role: PlayingTime, trouble: bool, pos: Position| {
        reps_read(r, 2031, stage, role, trouble, pos)
    };
    let s = PlayingTime::Starter;
    assert_eq!(read(college, Stage::College, s, false, Position::QB), Some(0.8f32 as f64));
    assert_eq!(read(hs, Stage::HighSchool, s, false, Position::QB), Some(0.6f32 as f64));
    // A freshman's first college step reads his depth chart, not his high school snaps.
    assert_eq!(read(hs, Stage::College, s, false, Position::QB), None);
    assert_eq!(read(college, Stage::HighSchool, s, false, Position::QB), None);
    // Two seasons old (a year away), a redshirt or sit year, a suspension,
    // a kicker, a prep year: the role decides.
    assert_eq!(reps_read(college, 2032, Stage::College, s, false, Position::QB), None);
    assert_eq!(read(college, Stage::College, PlayingTime::Redshirt, false, Position::QB), None);
    assert_eq!(read(college, Stage::College, s, true, Position::QB), None);
    assert_eq!(read(college, Stage::College, s, false, Position::K), None);
    assert_eq!(read(hs, Stage::PrepSchool, PlayingTime::Rotation, false, Position::QB), None);
}

/// One FBS step at his age with the given role and snaps; his overall gain.
fn step(p: &Player, seed: u64, age: u32, role: PlayingTime, snaps: Option<f64>) -> (Player, f64) {
    let mut q = p.clone();
    let mut rng = Rng::new(format!("reps-{seed}-{age}").as_str());
    let mut ctx = context_for(age);
    ctx.level = Level::FBS;
    ctx.coaching = 13.0;
    ctx.playing_time = role;
    ctx.snaps = snaps;
    let before = q.overall();
    develop_season(&mut q, &ctx, &mut rng);
    let gain = q.overall() - before;
    (q, gain)
}

/// The same 4,000 prospects the bug was measured on (talent z 1.5, to 18
/// with the test helpers), each case on the same random streams.
fn freshmen() -> Vec<Player> {
    let mut rng = Rng::new("reps-to-18");
    prospects(4000, "reps", 1.5)
        .into_iter()
        .map(|mut p| {
            career(&mut p, &mut rng, 18, |_| {});
            p
        })
        .collect()
}

#[test]
fn the_snaps_decide_a_returning_players_year() {
    use PlayingTime::*;
    let ps = freshmen();
    let gain = |age: u32, role: PlayingTime, snaps: Option<f64>| {
        mean(&ps.iter().enumerate().map(|(k, p)| step(p, k as u64, age, role, snaps).1).collect::<Vec<_>>())
    };
    // A true freshman: no college snaps yet, so the chart decides.
    let (fs, fb) = (gain(18, Starter, None), gain(18, Backup, None));
    // A returning player: last season's snaps decide whatever his role.
    let (full, sat, none) = (gain(18, Starter, Some(1.0)), gain(18, Starter, Some(0.05)), gain(18, Starter, None));
    let (benched, buried) = (gain(18, Backup, Some(1.0)), gain(18, Backup, Some(0.05)));
    eprintln!("true freshman: starter {fs:+.2}, backup {fb:+.2}");
    eprintln!("starter now: last season full {full:+.2}, a backup's {sat:+.2}, no snaps read {none:+.2}");
    eprintln!("backup now: last season full {benched:+.2}, a backup's {buried:+.2}");
    assert!(fs > fb + 0.5, "your starting freshman grows as a starter");
    assert!(full > sat + 0.4 && benched > buried + 0.4, "last season's snaps move growth");
    assert!((full - none).abs() < 0.15, "a full season reads about as the starter class");
    assert!((benched - full).abs() < 0.3, "the role keeps only breakouts and injuries");
}

#[test]
fn four_college_years_with_the_snaps_a_year_behind() {
    use PlayingTime::*;
    let ps = freshmen();
    // Each year's role, and the snaps last season left (a step reads them
    // a year behind, as the NFL does): a full season 1.0, a backup's 0.05.
    let path = |roles: [(PlayingTime, Option<f64>); 4]| {
        mean(
            &ps.iter()
                .enumerate()
                .map(|(k, p)| {
                    let mut q = p.clone();
                    for (y, &(role, snaps)) in roles.iter().enumerate() {
                        q = step(&q, k as u64, 18 + y as u32, role, snaps).0;
                    }
                    q.overall() - p.overall()
                })
                .collect::<Vec<_>>(),
        )
    };
    let all = path([(Starter, None), (Starter, Some(1.0)), (Starter, Some(1.0)), (Starter, Some(1.0))]);
    let one = path([(Backup, None), (Starter, Some(0.05)), (Starter, Some(1.0)), (Starter, Some(1.0))]);
    let two = path([(Backup, None), (Backup, Some(0.05)), (Starter, Some(0.05)), (Starter, Some(1.0))]);
    eprintln!("by 22: start four {all:+.2}, sit one {one:+.2}, sit two {two:+.2}");
    assert!(all > one && one > two);
}

// --- Growth from results (afm/game-reps/proposal.md) -----------------------

/// A player at a position brought to 22 on the yearly step.
fn player_at(pos: Position, k: u64) -> Player {
    let mut rng = Rng::new(format!("results-{pos:?}-{k}").as_str());
    let opts = ProspectOptions { position: Some(pos), talent_z: Some(1.5), ..Default::default() };
    let mut p = generate_prospect(&mut rng, opts);
    career(&mut p, &mut rng, 22, |_| {});
    p
}

fn passer(k: u64) -> Player {
    player_at(Position::QB, k)
}

/// One game of pass-block reps, `n` trials shown `k` rating points over his rating.
fn pass_blocks(n: f64, k: f64) -> Man {
    Man { reads: vec![(Read::PassBlock, n as f32, (n * k) as f32)], snaps: 60, ..Man::default() }
}

fn moved(p: &Player, games: &[Man], mode: &Mode) -> (Player, f64) {
    let a = Read::PassBlock.attr();
    let mut q = p.clone();
    let mut l = Ledger::default();
    let before = q.attributes.get(a);
    for m in games {
        results::apply(&mut q, &mut l, m, Level::NFL, mode);
    }
    let d = q.attributes.get(a) - before;
    (q, d)
}

#[test]
fn a_season_moves_a_skill_as_far_as_its_sample_says() {
    let p = player_at(Position::OT, 1);
    let a = Read::PassBlock.attr();
    let on = Mode::default();
    let f = results::fit(Level::NFL, Read::PassBlock, Position::OT);
    assert!(f.on(), "the fitted table reads an NFL tackle's pass blocking");
    // A whole season shown 4 points over his rating, in one go or game by game.
    let season = f.season;
    let (_, once) = moved(&p, &[pass_blocks(season, 4.0)], &on);
    let weekly: Vec<Man> = (0..17).map(|_| pass_blocks(season / 17.0, 4.0)).collect();
    let (_, by_game) = moved(&p, &weekly, &on);
    assert!((once - 4.0 * f.weight(season, results::CHANGE)).abs() < 1e-3, "credibility: {once}");
    assert!((once - by_game).abs() < 1e-3, "a season is a season however it's cut");
    assert!(once > 0.0 && once < 4.0 * 0.6, "a season keeps part of what it shows: {once}");
    // Small samples count less: a tenth of the season at the same level.
    let (_, small) = moved(&p, &[pass_blocks(season / 10.0, 4.0)], &on);
    assert!(small > 0.0 && small < once, "{small} against {once}");
    // Below his rating: pulled down (item 15), or not at all up only.
    let (_, down) = moved(&p, &[pass_blocks(season, -4.0)], &on);
    let (_, held) = moved(&p, &[pass_blocks(season, -4.0)], &Mode { pull_down: false, ..on });
    assert!((down + once).abs() < 1e-3 && held == 0.0);
    // No ceiling stops him (item 3): past it, the ceiling moves with him.
    let mut top = p.clone();
    top.ceilings.set(a, top.attributes.get(a) + 0.5);
    // Results far past anything real, so the season's most binds (the
    // weight tops out at what teammates and scheme leave him).
    let far = 2.0 * results::CAP / f.weight(50.0 * season, results::CHANGE);
    let (q, up) = moved(&top, &[pass_blocks(50.0 * season, far)], &on);
    assert!(up <= results::CAP + 1e-6 && up > results::CAP - 1e-3, "capped at a season's most: {up}");
    assert!(q.ceilings.get(a) >= q.attributes.get(a));
    // At his top, a move takes the ceiling with it either way; below it,
    // the ceiling stays and the yearly step grows him back (no ratchet).
    let mut at = p.clone();
    at.ceilings.set(a, at.attributes.get(a));
    let (q, down) = moved(&at, &[pass_blocks(season, -4.0)], &on);
    assert!(down < 0.0 && (q.ceilings.get(a) - q.attributes.get(a)).abs() < 1e-6, "his top drops with him");
    let (q, up) = moved(&at, &[pass_blocks(season, 4.0)], &on);
    assert!(up > 0.0 && (q.ceilings.get(a) - q.attributes.get(a)).abs() < 1e-6, "and rises with him");
    let mut below = p.clone();
    below.ceilings.set(a, below.attributes.get(a) + 5.0);
    let (q, _) = moved(&below, &[pass_blocks(season, -4.0)], &on);
    assert!((q.ceilings.get(a) - below.ceilings.get(a)).abs() < 1e-6, "below his top, the ceiling holds");
    // However the read is fitted, a season's move spreads at most
    // CHANGE / 2: the most credible noise keeps a quarter of itself.
    for n in [season / 4.0, season, 4.0 * season] {
        let noise = ((f.tau * f.tau + f.sd * f.sd / n) / (f.b * f.b)).sqrt();
        assert!(f.weight(n, results::CHANGE) * noise <= results::CHANGE / 2.0 + 1e-9);
    }
    // A rate scale moves it in proportion.
    let (_, half) = moved(&p, &[pass_blocks(season, 4.0)], &Mode { scale: 0.5, ..on });
    assert!((half - once / 2.0).abs() < 1e-3);
}

#[test]
fn a_man_judged_against_his_own_normal_stops_rising() {
    // An edge whose make-up beats his ratings by the same 4 points every
    // season: the first season lifts him, then his own normal takes it in.
    let p = player_at(Position::EDGE, 2);
    let f = results::fit(Level::NFL, Read::FinesseRush, Position::EDGE);
    assert!(f.on() && f.tp > 0.0, "the fit reads an edge's finesse rush and his own part");
    let mut q = p.clone();
    let mut l = Ledger::default();
    let a = Read::FinesseRush.attr();
    let mut lifts = Vec::new();
    for year in 0..6 {
        l.roll(year);
        let before = q.attributes.get(a);
        let n = f.season;
        let m = Man { reads: vec![(Read::FinesseRush, n as f32, (n * 4.0) as f32)], snaps: 60, ..Man::default() };
        results::apply(&mut q, &mut l, &m, Level::NFL, &Mode::default());
        lifts.push(q.attributes.get(a) - before);
    }
    eprintln!("a steady 4 over his ratings, season by season: {lifts:?}");
    assert!(lifts[0] > 0.5, "the first season lifts him");
    assert!(lifts[5] < 0.5 * lifts[0], "his own normal takes it in");
    assert!(lifts.windows(2).all(|w| w[1] <= w[0] + 1e-9));
}

#[test]
fn results_on_top_keep_todays_dice() {
    for k in 0..40 {
        let p = passer(k);
        let (mut a, mut b) = (p.clone(), p.clone());
        let mut ctx = context_for(22);
        develop_season(&mut a, &ctx, &mut Rng::new(format!("dice-{k}").as_str()));
        ctx.results = Some(Mode { dice: Dice::Kept, ..Mode::default() });
        develop_season(&mut b, &ctx, &mut Rng::new(format!("dice-{k}").as_str()));
        assert_eq!(a.attributes, b.attributes, "the yearly step draws the same");
        assert_eq!(a.ceilings, b.ceilings);
    }
}

/// A year held back for the weeks, landed in 13 blocks, with a starter's
/// snaps landing game by game (results on) or read from last season (off).
fn held_year(p: &Player, k: u64, in_season: bool) -> f64 {
    use crate::training::{FAMILIES, close_year, cover, land};
    let mut q = p.clone();
    let mut ctx = context_for(22);
    ctx.level = Level::FBS;
    ctx.defer = true;
    ctx.snaps = Some(0.85);
    ctx.results = in_season.then_some(Mode::default());
    // Last season's snaps landed as he played them too.
    if in_season {
        q.train.year.flags |= 8;
    }
    let before = q.overall();
    develop_season(&mut q, &ctx, &mut Rng::new(format!("held-{k}").as_str()));
    let mut l = Ledger::default();
    let per = (0.85 * results::season_snaps(Level::FBS) / 12.0).round() as u32;
    for b in 0..13 {
        let s = cover(&mut q.train, &[1.0 / 13.0; FAMILIES]);
        land(&mut q, &s, None, b);
        if (2..8).contains(&b) {
            for _ in 0..2 {
                results::reps(&mut q, &mut l, per, Level::FBS, 0.0, None);
            }
        }
    }
    close_year(&mut q, None);
    q.overall() - before
}

#[test]
fn snaps_landed_in_season_give_a_starters_year() {
    let ps: Vec<Player> = (0..300).map(passer).collect();
    let on = mean(&ps.iter().enumerate().map(|(k, p)| held_year(p, k as u64, true)).collect::<Vec<_>>());
    let off = mean(&ps.iter().enumerate().map(|(k, p)| held_year(p, k as u64, false)).collect::<Vec<_>>());
    eprintln!("a starter's held year: snaps in season {on:+.2}, read from last season {off:+.2}");
    assert!((on - off).abs() < 0.12, "the same snaps grow him the same");
}

#[test]
fn snaps_that_never_landed_are_read_once_and_kickers_keep_their_role() {
    // The rate a year opens at, as the store keeps it.
    let opened = |p: &Player, pos: Position, landed: bool, results: bool| -> u8 {
        let mut q = p.clone();
        q.position = pos;
        q.train.year.flags = if landed { 8 } else { 0 };
        let mut ctx = context_for(22);
        ctx.level = Level::FBS;
        ctx.defer = true;
        ctx.snaps = (!pos.is_specialist()).then_some(0.85);
        ctx.results = results.then_some(Mode::default());
        develop_season(&mut q, &ctx, &mut Rng::new("opened"));
        q.train.year.snaps
    };
    // Last season was at this level.
    let mut p = passer(1);
    if let Some(h) = p.history.last_mut() {
        h.level = Level::FBS;
    }
    // A season played with results off (or unsigned at camp, or an older
    // save) is read at the opening, as before: its snaps count once.
    assert_eq!(opened(&p, Position::QB, false, true), opened(&p, Position::QB, false, false));
    // A season whose snaps landed as he played opens at a backup's rate.
    assert!(opened(&p, Position::QB, true, true) < opened(&p, Position::QB, false, false));
    // A first season at a level has no season there to read.
    let mut fresh = p.clone();
    if let Some(h) = fresh.history.last_mut() {
        h.level = Level::HS;
    }
    assert_eq!(opened(&fresh, Position::QB, false, true), opened(&p, Position::QB, true, true));
    // A season whose games landed no snaps (a team on the stand-in) is
    // marked so at the checkpoint.
    let mut stood = p.clone();
    stood.train.year.flags |= 8;
    results::settle_reps(&mut stood, None);
    assert_eq!(stood.train.year.flags & 8, 0);
    // A kicker takes no snaps to land: his role decides (minor bug 264).
    assert_eq!(opened(&p, Position::K, true, true), opened(&p, Position::K, true, false));
}

#[test]
fn replaced_dice_keep_the_average() {
    let ps: Vec<Player> = (0..2000).map(passer).collect();
    let gains = |mode: Option<Mode>| -> Vec<f64> {
        ps.iter()
            .enumerate()
            .map(|(k, p)| {
                let mut q = p.clone();
                let mut ctx = context_for(22);
                ctx.results = mode;
                develop_season(&mut q, &ctx, &mut Rng::new(format!("replace-{k}").as_str()));
                q.overall() - p.overall()
            })
            .collect()
    };
    let (today, replaced) = (gains(None), gains(Some(Mode { dice: Dice::Replaced, ..Mode::default() })));
    let sd = |v: &[f64]| (v.iter().map(|x| (x - mean(v)).powi(2)).sum::<f64>() / v.len() as f64).sqrt();
    eprintln!(
        "a year with no games read: today {:+.2} (sd {:.2}), dice replaced {:+.2} (sd {:.2})",
        mean(&today),
        sd(&today),
        mean(&replaced),
        sd(&replaced)
    );
    assert!(results::tested(Position::QB, Level::NFL) != 0, "the fitted table tests a passer");
    assert!((mean(&today) - mean(&replaced)).abs() < 0.2, "the dice's average is kept");
    assert!(sd(&replaced) < sd(&today), "without the surprise");
}

#[test]
fn a_shift_the_whole_league_shares_moves_nobody() {
    // Every NFL guard shows 3 points over his rating (his matchups in this
    // world beat the fit's): the league's centre takes it in once the
    // league has played, so it can't lift (or sink) the whole position.
    let f = results::fit(Level::NFL, Read::PassBlock, Position::IOL);
    assert!(f.on());
    let mut book = results::Book::default();
    assert_eq!(
        book.centre(Level::NFL, Read::PassBlock, Position::IOL, false, (72.0, 60), 2030),
        0.0,
        "no league yet: the fit's"
    );
    let n = 200.0 * f.season;
    book.add(Level::NFL, Read::PassBlock, Position::IOL, false, (75.0, 60), 2030, n, 3.0 * n);
    let c = book.centre(Level::NFL, Read::PassBlock, Position::IOL, false, (72.0, 60), 2030);
    assert!((c - 3.0 * 200.0 / (200.0 + results::CENTRE_PRIOR)).abs() < 1e-9, "{c}");
    // A guard who showed the same 3 holds once his season is read against it.
    let p = player_at(Position::IOL, 3);
    let mut q = p.clone();
    let mut l = Ledger::default();
    let m =
        Man { reads: vec![(Read::PassBlock, f.season as f32, (3.0 * f.season) as f32)], snaps: 60, ..Man::default() };
    results::apply_in(&mut q, &mut l, &m, Level::NFL, &Mode::default(), &|_| 3.0);
    let a = Read::PassBlock.attr();
    assert!((q.attributes.get(a) - p.attributes.get(a)).abs() < 1e-6, "the league's shift moves nobody");
    assert!(l.shown(Read::PassBlock).is_some_and(|x| x.0.abs() < 1e-5), "read against the league");
    // Apart by position, by practice, by peers on the skill and by season.
    assert_eq!(book.centre(Level::NFL, Read::PassBlock, Position::OT, false, (72.0, 60), 2030), 0.0);
    assert_eq!(book.centre(Level::NFL, Read::PassBlock, Position::IOL, true, (72.0, 60), 2030), 0.0);
    assert_eq!(
        book.centre(Level::NFL, Read::PassBlock, Position::IOL, false, (84.0, 60), 2030),
        0.0,
        "an 84 has his own peers"
    );
    assert_eq!(
        book.centre(Level::NFL, Read::PassBlock, Position::IOL, false, (72.0, 20), 2030),
        0.0,
        "a rested man has his own"
    );
    assert_eq!(book.centre(Level::NFL, Read::PassBlock, Position::IOL, false, (72.0, 60), 2031), 0.0);
    book.add(Level::NFL, Read::PassBlock, Position::IOL, false, (72.0, 60), 2031, n, 0.0);
    assert_eq!(
        book.centre(Level::NFL, Read::PassBlock, Position::IOL, false, (72.0, 60), 2031),
        0.0,
        "a new season starts over"
    );
    book.sweep(2032);
    assert!(book.league.is_empty());
}
