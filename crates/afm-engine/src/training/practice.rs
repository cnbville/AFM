//! Practice as battles (topic 8, choice 3): the week's team periods run as
//! snaps on the match engine, the offense against its own defense, ones
//! against ones, twos against twos and the young players in their own
//! period. Every snap is graded as a game snap is, and the grades are what
//! the staff see: a player who wins in practice climbs the depth chart a
//! little (`impression`), one who loses slides. Your team runs every snap
//! its plan gives; every college team and every NFL team runs a sample of
//! its week (the same plan shape, fewer snaps), so the AI's depth charts
//! move with practice too. High school AI staffs set their depth by their
//! read of the ratings (decided in the final build).

use super::plan::{Kind, Tempo, Week, Who};
use super::rules::Block;
use super::seat::{PlannedWeek, SeatTeam, WeekLog};
use super::work::Role;
use crate::game::GameTeam;
use crate::game::flow::{PracticeSnap, practice_snaps};
use crate::game::stats::{GameStats, PlayRecord};
use crate::player::Level;
use crate::rng::{Rng, clamp};
use crate::world::World;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

/// The week's live team snaps: (kind, who, situation, snaps). Walkthrough
/// tempo is mental work, not a rep against a defense.
pub fn team_snaps(wk: &Week) -> Vec<(Kind, Who, u8, f32)> {
    let mut out: Vec<(Kind, Who, u8, f32)> = Vec::new();
    for d in &wk.days {
        for p in &d.periods {
            if !p.kind.offense_against_defense() || p.tempo == Tempo::Walk {
                continue;
            }
            let n = p.min as f32 * p.kind.snaps_per_min() as f32 * if p.tempo == Tempo::Fast { 1.25 } else { 1.0 };
            let what = if p.kind == Kind::Situations || p.kind == Kind::Team { p.what } else { 0 };
            match out.iter_mut().find(|x| x.0 == p.kind && x.1 == p.who && x.2 == what) {
                Some(x) => x.3 += n,
                None => out.push((p.kind, p.who, what, n)),
            }
        }
        // The young players' period runs as twos against twos.
        for p in d.periods.iter().filter(|p| p.kind == Kind::Young && p.tempo != Tempo::Walk) {
            let n = p.min as f32 * p.kind.snaps_per_min() as f32;
            match out.iter_mut().find(|x| x.0 == Kind::Young) {
                Some(x) => x.3 += n,
                None => out.push((Kind::Young, Who::Young, 0, n)),
            }
        }
    }
    out
}

/// All the week's live team snaps.
pub fn total(snaps: &[(Kind, Who, u8, f32)]) -> f32 {
    snaps.iter().map(|x| x.3).sum()
}

/// The situations a period's snaps come from: (down, to go, yards from the
/// goal line, seconds left in the half).
fn spots(kind: Kind, what: u8) -> &'static [(u8, f64, f64, f64)] {
    const BASE: [(u8, f64, f64, f64); 8] = [
        (1, 10.0, 65.0, 900.0),
        (1, 10.0, 50.0, 900.0),
        (2, 7.0, 55.0, 900.0),
        (2, 3.0, 45.0, 900.0),
        (3, 4.0, 40.0, 900.0),
        (3, 8.0, 60.0, 900.0),
        (1, 10.0, 35.0, 900.0),
        (2, 10.0, 70.0, 900.0),
    ];
    const THIRD: [(u8, f64, f64, f64); 5] = [
        (3, 2.0, 50.0, 900.0),
        (3, 4.0, 45.0, 900.0),
        (3, 6.0, 55.0, 900.0),
        (3, 8.0, 60.0, 900.0),
        (3, 11.0, 65.0, 900.0),
    ];
    const RED: [(u8, f64, f64, f64); 5] = [
        (1, 10.0, 18.0, 900.0),
        (2, 7.0, 13.0, 900.0),
        (3, 5.0, 9.0, 900.0),
        (1, 10.0, 12.0, 900.0),
        (2, 4.0, 6.0, 900.0),
    ];
    const TWO_MIN: [(u8, f64, f64, f64); 4] =
        [(1, 10.0, 75.0, 110.0), (2, 10.0, 62.0, 90.0), (1, 10.0, 48.0, 60.0), (3, 6.0, 40.0, 35.0)];
    const GOAL: [(u8, f64, f64, f64); 4] =
        [(1, 3.0, 3.0, 900.0), (2, 2.0, 2.0, 900.0), (3, 1.0, 1.0, 900.0), (4, 1.0, 1.0, 900.0)];
    const BACKED: [(u8, f64, f64, f64); 3] = [(1, 10.0, 97.0, 900.0), (2, 8.0, 95.0, 900.0), (3, 6.0, 94.0, 900.0)];
    const FOUR: [(u8, f64, f64, f64); 3] = [(1, 10.0, 60.0, 240.0), (2, 6.0, 54.0, 200.0), (3, 2.0, 48.0, 160.0)];
    const INSIDE: [(u8, f64, f64, f64); 3] = [(1, 10.0, 60.0, 900.0), (2, 6.0, 55.0, 900.0), (3, 2.0, 50.0, 900.0)];
    const SEVENS: [(u8, f64, f64, f64); 5] = [
        (1, 10.0, 60.0, 900.0),
        (2, 8.0, 55.0, 900.0),
        (3, 6.0, 50.0, 900.0),
        (3, 9.0, 60.0, 900.0),
        (1, 10.0, 22.0, 900.0),
    ];
    const RUSH: [(u8, f64, f64, f64); 2] = [(3, 8.0, 60.0, 900.0), (2, 10.0, 60.0, 900.0)];
    match (kind, what) {
        (Kind::InsideRun, _) => &INSIDE,
        (Kind::SevenOnSeven, _) => &SEVENS,
        (Kind::OneOnOnes, _) => &RUSH,
        (_, 1) => &THIRD,
        (_, 2) => &RED,
        (_, 3) => &TWO_MIN,
        (_, 4) => &GOAL,
        (_, 5) => &BACKED,
        (_, 6) => &FOUR,
        _ => &BASE,
    }
}

/// Who is on the field for a group of snaps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Unit {
    Ones,
    Twos,
    Young,
}

/// The snaps for each unit, `cap` in all, spread by the week's periods.
fn plan_snaps(snaps: &[(Kind, Who, u8, f32)], cap: usize, seed: u64) -> Vec<(Unit, Vec<PracticeSnap>)> {
    let all = total(snaps);
    if all <= 0.0 || cap == 0 {
        return Vec::new();
    }
    let scale = (cap as f32 / all).min(1.0);
    let mut rng = Rng::new(seed as u32 ^ 0xBA77);
    let mut out: Vec<(Unit, Vec<PracticeSnap>)> = Vec::new();
    for &(kind, who, what, n) in snaps {
        let n = (n * scale).round() as usize;
        // Ones and twos split a period everyone works: about three of
        // every five reps go to the ones (proposed, not confirmed).
        let split: &[(Unit, f64)] = match who {
            Who::All => &[(Unit::Ones, 0.6), (Unit::Twos, 0.4)],
            Who::Ones => &[(Unit::Ones, 0.8), (Unit::Twos, 0.2)],
            Who::Young => &[(Unit::Young, 1.0)],
        };
        let run = match kind {
            Kind::InsideRun => Some(true),
            Kind::SevenOnSeven | Kind::OneOnOnes => Some(false),
            _ => None,
        };
        let table = spots(kind, what);
        for &(unit, share) in split {
            let k = (n as f64 * share).round() as usize;
            if k == 0 {
                continue;
            }
            let list: Vec<PracticeSnap> = (0..k)
                .map(|_| {
                    let &(down, togo, yl, half) = rng.pick(table);
                    PracticeSnap { down, togo, yardline_100: yl, half_left: half, run }
                })
                .collect();
            match out.iter_mut().find(|x| x.0 == unit) {
                Some(x) => x.1.extend(list),
                None => out.push((unit, list)),
            }
        }
    }
    out
}

/// What a week of battles gave: per player the engine's per-play grade sum
/// and plays graded, and the plays (for your team's one-on-one read).
#[derive(Default)]
pub struct Battles {
    pub lines: HashMap<u32, (f64, u32)>,
    pub plays: Vec<PlayRecord>,
    pub snaps: u32,
}

/// Runs a week's battles for a team. `starters` and `backups` are player
/// ids by their role this week.
#[allow(clippy::too_many_arguments)]
pub fn run(
    team: &GameTeam,
    level: Level,
    snaps: &[(Kind, Who, u8, f32)],
    cap: usize,
    starters: &HashSet<u32>,
    backups: &HashSet<u32>,
    keep_plays: bool,
    seed: u64,
) -> Battles {
    let mut out = Battles::default();
    for (unit, list) in plan_snaps(snaps, cap, seed) {
        let owned;
        let side: &GameTeam = match unit {
            Unit::Ones => team,
            Unit::Twos | Unit::Young => {
                let mut t = team.clone();
                let out_set: HashSet<u32> = if unit == Unit::Twos || team.players.len() < 70 {
                    starters.clone()
                } else {
                    starters.union(backups).copied().collect()
                };
                // Specialists stay: someone has to snap and hold.
                t.players.retain(|p| !out_set.contains(&p.id) || p.position.is_specialist());
                if t.players.len() < 24 {
                    continue;
                }
                t.order.clear();
                owned = t;
                &owned
            }
        };
        let stats = practice_snaps(side, side, level, &list, seed ^ (unit as u64) << 40);
        absorb(&mut out, stats, keep_plays);
    }
    out
}

fn absorb(out: &mut Battles, stats: GameStats, keep: bool) {
    out.snaps += stats.plays.len() as u32;
    for side in &stats.players {
        for (&id, l) in side {
            if l.graded == 0 {
                continue;
            }
            let e = out.lines.entry(id).or_insert((0.0, 0));
            e.0 += l.grade_sum;
            e.1 += l.graded;
        }
    }
    if keep {
        out.plays.extend(stats.plays);
    }
}

/// A grade from per-play grades the way a game grade is made (60 average,
/// fewer plays pulled toward it).
pub fn grade(sum: f64, n: u32) -> f64 {
    if n == 0 {
        return 0.0;
    }
    let avg = sum / n as f64;
    let conf = (n as f64 / (n as f64 + 25.0)).sqrt();
    (60.0 + 95.0 * avg * conf).clamp(20.0, 99.9)
}

/// What the staff see of a week's grade against what the engine graded
/// (topic 8, "practice players and gamers"): practice has no crowd and no
/// game-to-game form, so an up-and-down player looks better on a Wednesday
/// than he plays on Saturday; a short fuse costs reps in the scuffles a
/// practice has; a happy player practices well and an unhappy one goes
/// through the motions (proposed, not confirmed: up to about 5 points at
/// the ends).
pub fn seen(engine: f64, p: &crate::player::Player, mood: f32) -> f64 {
    let g = &p.game;
    let bias = 0.25 * (10.0 - g.consistency) + 0.15 * (g.temperament - 10.0) + 2.0 * mood as f64;
    clamp(engine + bias, 1.0, 99.0)
}

/// His running practice grade takes the week's (the last few weeks count
/// most).
pub fn blend(old: u8, week: f64) -> u8 {
    let v = if old == 0 { week } else { 0.55 * old as f64 + 0.45 * week };
    clamp(v.round(), 1.0, 100.0) as u8
}

/// Your team's week of battles: every snap its plan gives (up to 150),
/// the grades into each man's practice grade and the block's grades, the
/// week's best and worst, the one-on-one battles and the practice read of
/// the designs being installed.
pub(crate) fn seat_battles(
    w: &mut World,
    st: &SeatTeam,
    pw: &PlannedWeek,
    roles: &HashMap<usize, Role>,
    seed: u64,
    log: &mut WeekLog,
) {
    let snaps = team_snaps(&pw.week);
    if total(&snaps) < 1.0 {
        return;
    }
    let Some((team, level)) = w.seat_game_team() else { return };
    let mut starters = HashSet::new();
    let mut backups = HashSet::new();
    for (&i, &r) in roles {
        let id = w.people[i].player.id;
        match r {
            Role::Starter => {
                starters.insert(id);
            }
            Role::Backup => {
                backups.insert(id);
            }
            _ => {}
        }
    }
    let b = run(&team, level, &snaps, 150, &starters, &backups, true, seed ^ 0xBA7);
    log.snaps = b.snaps;
    let by_id: HashMap<u32, usize> = st.roster.iter().map(|&i| (w.people[i].player.id, i)).collect();
    let mut week: Vec<(u32, u8, u32)> = Vec::new();
    for (&id, &(sum, n)) in &b.lines {
        let Some(&i) = by_id.get(&id) else { continue };
        let p = &mut w.people[i];
        let g = seen(grade(sum, n), &p.player, p.mood);
        p.player.train.grade = blend(p.player.train.grade, g);
        week.push((id, g.round() as u8, n));
        match w.training.grades.iter_mut().find(|x| x.0 == id) {
            Some(x) => {
                x.1 += sum as f32;
                x.2 = x.2.saturating_add(n.min(u16::MAX as u32) as u16);
            }
            None => w.training.grades.push((id, sum as f32, n.min(u16::MAX as u32) as u16)),
        }
    }
    week.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let enough: Vec<&(u32, u8, u32)> = week.iter().filter(|x| x.2 >= 6).collect();
    log.best = enough.iter().take(5).map(|x| (x.0, x.1)).collect();
    log.worst = enough.iter().rev().take(5).map(|x| (x.0, x.1)).collect();
    log.battles = battles(&b.plays);
    design_reads(w, &team, level, pw, seed);
}

/// The one-on-ones of the week's snaps: blocker against rusher, receiver
/// against his man. (a, b, a's wins, b's wins), the most-repped first.
pub fn battles(plays: &[PlayRecord]) -> Vec<(u32, u32, u8, u8)> {
    let mut m: HashMap<(u32, u32), (u32, u32)> = HashMap::new();
    let mut add = |win: u32, lose: u32| {
        let (k, first) = if win < lose { ((win, lose), true) } else { ((lose, win), false) };
        let e = m.entry(k).or_insert((0, 0));
        if first {
            e.0 += 1;
        } else {
            e.1 += 1;
        }
    };
    for r in plays {
        for &(blk, def, _, won, _, _) in &r.blocks {
            if won { add(blk, def) } else { add(def, blk) }
        }
        if let (Some(t), Some(d)) = (r.target, r.defender) {
            if r.complete { add(t, d) } else { add(d, t) }
        }
    }
    let mut v: Vec<(u32, u32, u8, u8)> = m
        .into_iter()
        .filter(|(_, (a, b))| a + b >= 3)
        .map(|((x, y), (a, b))| {
            let (a, b) = (a.min(255) as u8, b.min(255) as u8);
            if a >= b { (x, y, a, b) } else { (y, x, b, a) }
        })
        .collect();
    v.sort_by(|a, b| (b.2 as u32 + b.3 as u32).cmp(&(a.2 as u32 + a.3 as u32)).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
    v.truncate(16);
    v
}

/// The designs in this week's install run against the scout team's looks
/// (plays and tactics 8.6): a few snaps each team period day, read with
/// their noise.
fn design_reads(w: &mut World, team: &GameTeam, level: Level, pw: &PlannedWeek, seed: u64) {
    use crate::game::playbook::{Coverage, DefCall, DefPersonnel, Pressure};
    let days =
        pw.week.days.iter().filter(|d| d.periods.iter().any(|p| p.kind.offense_against_defense())).count() as u32;
    if days == 0 {
        return;
    }
    let looks = [
        DefCall::new(DefPersonnel::Base, Coverage::Cover3, Pressure::Four),
        DefCall::new(DefPersonnel::Nickel, Coverage::Cover1, Pressure::LbBlitz),
        DefCall::new(DefPersonnel::Nickel, Coverage::Cover4, Pressure::Four),
        DefCall::new(DefPersonnel::Base, Coverage::Cover2, Pressure::Four),
        DefCall::new(DefPersonnel::Nickel, Coverage::Cover3Match, Pressure::Simulated),
    ];
    if w.training.design_ids.len() != w.training.designs.len() {
        w.training.register_designs();
    }
    for k in 0..w.training.designs.len() {
        if !w.training.designs[k].installing {
            continue;
        }
        let Some(&concept) = w.training.design_ids.get(k) else { continue };
        let formation = w.training.designs[k].design.formation;
        let r = crate::game::designer::practice(
            concept,
            formation,
            team,
            team,
            &looks,
            2 * days,
            level,
            seed ^ (k as u64) << 32 ^ 0xDE51,
        );
        let d = &mut w.training.designs[k];
        d.read = Some(match d.read.take() {
            Some(old) => {
                let n = old.snaps + r.snaps;
                crate::game::designer::PracticeRead {
                    snaps: n,
                    yards: (old.yards * old.snaps as f64 + r.yards * r.snaps as f64) / n.max(1) as f64,
                    completions: old.completions + r.completions,
                }
            }
            None => r,
        });
    }
}

/// Snaps a sampled AI team runs in a week (proposed, not confirmed: enough
/// to move a depth chart over a camp, few enough that a world's Monday
/// stays quick).
pub fn ai_cap(block: Block) -> usize {
    match block {
        Block::Camp => 24,
        Block::Spring => 16,
        Block::Season | Block::Postseason => 8,
        Block::Otas | Block::Phase2 => 6,
        _ => 0,
    }
}

/// A team the world samples this week: its game team, level, the week's
/// team snaps and how many to run.
pub(crate) struct AiTeam {
    pub team: GameTeam,
    pub level: Level,
    pub snaps: Vec<(Kind, Who, u8, f32)>,
    pub cap: usize,
}

/// Every college and NFL team's sampled battles (not your team's): the
/// grades into each man's practice grade.
pub(crate) fn world_battles(w: &mut World, teams: Vec<(Vec<usize>, AiTeam)>, seed: u64) {
    let people = &w.people;
    let results: Vec<Vec<(usize, u8)>> = teams
        .par_iter()
        .enumerate()
        .map(|(k, (roster, t))| {
            let mut starters = HashSet::new();
            let mut backups = HashSet::new();
            for &i in roster {
                let p = &people[i];
                match Role::of(p.player.position, p.last_role) {
                    Role::Starter => {
                        starters.insert(p.player.id);
                    }
                    Role::Backup => {
                        backups.insert(p.player.id);
                    }
                    _ => {}
                }
            }
            let b = run(&t.team, t.level, &t.snaps, t.cap, &starters, &backups, false, seed ^ (k as u64) << 16);
            let by_id: HashMap<u32, usize> = roster.iter().map(|&i| (people[i].player.id, i)).collect();
            b.lines
                .iter()
                .filter(|(_, (_, n))| *n >= 2)
                .filter_map(|(id, &(sum, n))| {
                    let &i = by_id.get(id)?;
                    let p = &people[i];
                    Some((i, blend(p.player.train.grade, seen(grade(sum, n), &p.player, p.mood))))
                })
                .collect()
        })
        .collect();
    for v in results {
        for (i, g) in v {
            w.people[i].player.train.grade = g;
        }
    }
}

/// The fitted model's slope: practice grade points per overall point
/// above his position group's average on his team, and the spread of a
/// sampled week's grade around it (fitted on the college battles of a
/// small world by `training_check`; proposed, not confirmed).
pub const FIT_SLOPE: f64 = 0.55;
pub const FIT_SPREAD: f64 = 6.0;

/// High school teams other than yours (choice 3): their practice grades
/// come from the model fitted to the engine, by each player's rating
/// against his position group at his school, with a sampled week's noise.
/// `snaps` is each school's sampled snaps this week (0: no team work).
pub(crate) fn school_grades(w: &mut World, snaps: &[usize], seed: u64) {
    let schools = &w.high_schools;
    let people = &w.people;
    let grades: Vec<Vec<(usize, u8)>> = schools
        .par_iter()
        .enumerate()
        .map(|(h, s)| {
            let n = snaps.get(h).copied().unwrap_or(0);
            if n == 0 || w.seat_school() == Some(h) {
                return Vec::new();
            }
            let mut by_pos: HashMap<crate::positions::Position, (f64, u32)> = HashMap::new();
            for &i in &s.roster {
                let p = &people[i].player;
                let e = by_pos.entry(p.position).or_insert((0.0, 0));
                e.0 += p.overall();
                e.1 += 1;
            }
            // A player's share of the team snaps: about half of them.
            let share = (n as f64 * 0.5).max(1.0);
            let spread = FIT_SPREAD * (12.0 / share).sqrt();
            s.roster
                .iter()
                .map(|&i| {
                    let person = &people[i];
                    let p = &person.player;
                    let (sum, k) = by_pos[&p.position];
                    let rel = p.overall() - sum / k as f64;
                    let noise = crate::world::season::key_normal(seed ^ h as u64, p.id as u64);
                    let g = 60.0 + FIT_SLOPE * rel + spread * noise;
                    (i, blend(p.train.grade, seen(g.clamp(20.0, 99.9), p, person.mood)))
                })
                .collect()
        })
        .collect();
    for v in grades {
        for (i, g) in v {
            w.people[i].player.train.grade = g;
        }
    }
}
