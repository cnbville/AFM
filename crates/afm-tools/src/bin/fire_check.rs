//! Coach fire on the field, measured (`docs/design/coach-fire.md`,
//! "Measuring"). Two parts:
//!
//! `fire_check hits <nfl|fbs|fcs|hs> [games] [seed]`: the whole test league
//! with each head coach's own read: the hardest hit's force on plays with
//! one (the big hit's line, `talk::BIG_HIT`), big hits, sparks, footsteps
//! and hot plays per team game, the personal fouls by kind, targeting and
//! ejections per team game, and the quarterbacks' slides.
//!
//! `fire_check talks <level> [seed]`: the test league's head coaches'
//! pregame picks against the picks main's worth would make, and what each
//! leaves on the men who play first.
//!
//! `fire_check run [games] [seed] [stakes]`: the same Branch-type safety and
//! the same calm pocket passer (team 0 of the NFL test league) under each
//! prototype room, against his head coach's own read, A/B on the same seeds:
//! first what each room's talk leaves on the two men, then the safety's
//! forced fumbles a tackle, missed tackle rate, personal fouls, targeting
//! and ejections, breakups, big hits and hot plays; the passer's completion
//! over expected, sacks, interceptions and EPA. 272 games an arm by default;
//! stakes 0.8 plays them as playoff games.

use afm_engine::attributes::Attr;
use afm_engine::game::rules::trait_norm;
use afm_engine::game::sim::FoulKind;
use afm_engine::game::stats::{PlayRecord, PlayType, expected_completion_at};
use afm_engine::game::synthetic::synthetic_league;
use afm_engine::game::talk::{Move, Pick, Subject, Tone};
use afm_engine::game::{GameResult, GameTeam, quick_game};
use afm_engine::player::Level;
use afm_engine::positions::Position;
use afm_engine::tendencies::Tend;
use rayon::prelude::*;

fn level_of(s: Option<&str>) -> Level {
    match s {
        Some("fbs") => Level::FBS,
        Some("fcs") => Level::FCS,
        Some("hs") => Level::HS,
        _ => Level::NFL,
    }
}

/// The teams of game `g` in the whole league: each matchup twice, once at
/// each end (as `match_check`).
fn pair_of(g: usize) -> (usize, usize) {
    let k = g / 2;
    let h = (k * 7) % 32;
    let a = (k * 13 + 5) % 32;
    let a = if a == h { (a + 1) % 32 } else { a };
    if g.is_multiple_of(2) { (h, a) } else { (a, h) }
}

fn pct(v: &mut [f64], q: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f64 * q).round() as usize]
}

fn hits(level: Level, games: usize, seed: u32) {
    let teams = synthetic_league(level, 32, seed);
    let t0 = std::time::Instant::now();
    let results: Vec<GameResult> = (0..games)
        .into_par_iter()
        .map(|g| {
            let (h, a) = pair_of(g);
            quick_game(&teams[h], &teams[a], level, seed as u64 * 1_000_003 + g as u64)
        })
        .collect();
    let n = 2.0 * games as f64;
    let mut force: Vec<f64> = Vec::new();
    let (mut big, mut spark, mut steps, mut slides, mut slide_flags) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let mut hot = std::collections::BTreeMap::<&str, f64>::new();
    let mut fouls = std::collections::BTreeMap::<&str, f64>::new();
    let (mut ejected, mut tackles, mut ff, mut missed) = (0.0, 0.0, 0.0, 0.0);
    for r in &results {
        for p in &r.stats.plays {
            if let Some((_, _, f)) = p.fire.hardest {
                force.push(f);
            }
            big += p.fire.big_hit.is_some() as u8 as f64;
            spark += p.fire.spark.is_some() as u8 as f64;
            steps += p.fire.footsteps.is_some() as u8 as f64;
            ejected += p.fire.ejected.len() as f64;
            for &(_, t) in &p.fire.hot {
                *hot.entry(t.key()).or_default() += 1.0;
            }
            if p.tend.slid {
                slides += 1.0;
                if p.penalties.iter().any(|x| x.0.personal() && x.1 != p.offense) {
                    slide_flags += 1.0;
                }
            }
            for &(k, _, _, _, accepted) in &p.penalties {
                if accepted && (k.personal() || k == FoulKind::Offside) {
                    *fouls.entry(k.name()).or_default() += 1.0;
                }
            }
            if !p.nullified {
                tackles += p.tacklers.len().min(1) as f64;
                ff += p.forced_fumble.is_some() as u8 as f64;
                missed += p.missed_tackles.len() as f64;
            }
        }
    }
    println!("{} {games} games, seed {seed}, {:.1} s", level.name(), t0.elapsed().as_secs_f64());
    let plays = force.len();
    println!("plays with a hit: {plays} ({:.1} a team game)", plays as f64 / n);
    for q in [0.5, 0.75, 0.9, 0.95, 0.97, 0.98, 0.99] {
        println!("  hardest hit, {:>2.0}th percentile: {:.0}", 100.0 * q, pct(&mut force, q));
    }
    println!("a team game:");
    println!("  big hits {:.2}, sparks {:.2}, heard footsteps {:.3}", big / n, spark / n, steps / n);
    for (k, v) in &hot {
        println!("  hot play, {k}: {:.3}", v / n);
    }
    for (k, v) in &fouls {
        println!("  {k} (accepted): {:.3}", v / n);
    }
    println!("  ejections {:.3}", ejected / n);
    println!("  slides {:.2}, a personal foul on a slide {:.3}", slides / n, slide_flags / n);
    println!(
        "  forced fumbles a tackle {:.4}, missed tackles a tackle {:.3}",
        ff / tackles.max(1.0),
        missed / tackles.max(1.0)
    );
}

/// The prototype rooms (coach voice's words, `manner::Proto`) as the talk
/// each gives his room: the Showman's room talk is the Firebreather's.
fn rooms() -> Vec<(&'static str, Option<Pick>)> {
    use Move as M;
    use Subject as S;
    use Tone as T;
    vec![
        ("own read", None),
        ("Firebreather", Some(Pick::new(M::Challenge, T::Fiery, S::Them))),
        ("Drill Sergeant", Some(Pick::new(M::Criticize, T::Fiery, S::Us))),
        ("Needler", Some(Pick::new(M::Criticize, T::Firm, S::Us))),
        ("CEO", Some(Pick::new(M::Straight, T::Calm, S::Plan))),
        ("Players' Coach", Some(Pick::new(M::Praise, T::Warm, S::Us))),
        ("Preacher", Some(Pick::new(M::Straight, T::Fiery, S::Stakes))),
        ("Professor", Some(Pick::new(M::Ask, T::Calm, S::Plan))),
    ]
}

/// Team 0's best safety made a Branch type (temper 3, competitiveness 17,
/// Physicality 17 on the level's scale, a big hitter, ordinary composure:
/// his pitch 0.62) and its best quarterback a calm pocket passer (temper
/// 16, an ordinary competitor, rarely scrambles, takes the checkdown: his
/// pitch 0.38). Returns their ids.
fn focus(t: &mut GameTeam, level: Level) -> (u32, u32) {
    let best = |t: &GameTeam, pos: Position| {
        (0..t.players.len())
            .filter(|&i| t.players[i].position == pos)
            .max_by(|&a, &b| t.players[a].overall_at(pos).total_cmp(&t.players[b].overall_at(pos)))
            .expect("a man at the position")
    };
    let s = best(t, Position::S);
    let p = &mut t.players[s];
    p.traits.temperament = 3.0;
    p.personality.competitiveness = 17.0;
    p.attrs.set(Attr::Physicality, trait_norm(level, Attr::Physicality) + 7.0 * 2.6);
    p.attrs.set(Attr::Composure, trait_norm(level, Attr::Composure));
    p.tend.set(Tend::BigHit, 16.0);
    let safety = p.id;
    let q = best(t, Position::QB);
    let p = &mut t.players[q];
    p.traits.temperament = 16.0;
    p.personality.competitiveness = 10.0;
    p.attrs.set(Attr::Physicality, trait_norm(level, Attr::Physicality));
    p.attrs.set(Attr::Composure, trait_norm(level, Attr::Composure));
    p.tend.set(Tend::Gunslinger, 6.0);
    p.tend.set(Tend::Scramble, 5.0);
    (safety, p.id)
}

/// What one game gives: the safety's and the passer's counts, and the
/// team's points.
#[derive(Clone, Copy, Default)]
struct Line {
    snaps: f64,
    tackles: f64,
    missed: f64,
    ff: f64,
    pbu: f64,
    int: f64,
    big: f64,
    personal: f64,
    targeting: f64,
    ejected: f64,
    hot: f64,
    att: f64,
    cmp: f64,
    xcmp: f64,
    sacks: f64,
    ints: f64,
    epa: f64,
    dropbacks: f64,
    points: f64,
    allowed: f64,
}

impl Line {
    fn add(&mut self, o: &Line) {
        let (a, b) = (self.as_mut(), o.as_ref());
        for (x, y) in a.into_iter().zip(b) {
            *x += y;
        }
    }
    fn as_ref(&self) -> [f64; 20] {
        [
            self.snaps,
            self.tackles,
            self.missed,
            self.ff,
            self.pbu,
            self.int,
            self.big,
            self.personal,
            self.targeting,
            self.ejected,
            self.hot,
            self.att,
            self.cmp,
            self.xcmp,
            self.sacks,
            self.ints,
            self.epa,
            self.dropbacks,
            self.points,
            self.allowed,
        ]
    }
    fn as_mut(&mut self) -> [&mut f64; 20] {
        [
            &mut self.snaps,
            &mut self.tackles,
            &mut self.missed,
            &mut self.ff,
            &mut self.pbu,
            &mut self.int,
            &mut self.big,
            &mut self.personal,
            &mut self.targeting,
            &mut self.ejected,
            &mut self.hot,
            &mut self.att,
            &mut self.cmp,
            &mut self.xcmp,
            &mut self.sacks,
            &mut self.ints,
            &mut self.epa,
            &mut self.dropbacks,
            &mut self.points,
            &mut self.allowed,
        ]
    }
    /// The measures: forced fumbles a tackle, missed tackle rate, personal
    /// fouls, targeting and ejections a game, breakups a game, big hits a
    /// game, hot plays a game; completion over expected, sacks and
    /// interceptions a dropback, EPA a dropback; points for and against.
    fn measures(&self, games: f64) -> [f64; 13] {
        let tk = self.tackles.max(1.0);
        [
            self.ff / tk,
            self.missed / (self.tackles + self.missed).max(1.0),
            self.personal / games,
            (self.targeting + self.ejected) / games,
            self.pbu / games,
            self.big / games,
            self.hot / games,
            (self.cmp - self.xcmp) / self.att.max(1.0),
            self.sacks / self.dropbacks.max(1.0),
            self.ints / self.dropbacks.max(1.0),
            self.epa / self.dropbacks.max(1.0),
            self.points / games,
            self.allowed / games,
        ]
    }
}

const MEASURES: [&str; 13] = [
    "S forced fumbles a tackle",
    "S missed tackle rate",
    "S personal fouls a game",
    "S targeting or ejected a game",
    "S breakups a game",
    "S big hits a game",
    "S hot plays a game",
    "QB completion over expected",
    "QB sacks a dropback",
    "QB interceptions a dropback",
    "QB EPA a dropback",
    "team points for",
    "team points against",
];

fn game_line(r: &GameResult, side: usize, safety: u32, qb: u32, level: Level) -> Line {
    let mut l = Line::default();
    if let Some(s) = r.stats.players[side].get(&safety) {
        l.snaps = s.snaps_def as f64;
    }
    for p in &r.stats.plays {
        let pr: &PlayRecord = p;
        for &(k, _, id, _, _) in &pr.penalties {
            if id == safety && k.personal() {
                l.personal += 1.0;
                if k == FoulKind::Targeting {
                    l.targeting += 1.0;
                }
            }
        }
        if pr.fire.ejected.contains(&safety) {
            l.ejected += 1.0;
        }
        if pr.fire.hot.iter().any(|x| x.0 == safety) {
            l.hot += 1.0;
        }
        if pr.fire.big_hit.is_some_and(|b| b.0 == safety) {
            l.big += 1.0;
        }
        if pr.nullified {
            continue;
        }
        if pr.tacklers.contains(&safety) {
            l.tackles += 1.0;
        }
        l.missed += pr.missed_tackles.iter().filter(|m| m.0 == safety).count() as f64;
        if pr.forced_fumble == Some(safety) {
            l.ff += 1.0;
        }
        if pr.pbu == Some(safety) {
            l.pbu += 1.0;
        }
        if pr.interceptor == Some(safety) {
            l.int += 1.0;
        }
        if pr.passer == Some(qb) && matches!(pr.kind, PlayType::Pass | PlayType::Sack | PlayType::Scramble) {
            l.dropbacks += 1.0;
            l.epa += pr.epa;
            match pr.kind {
                PlayType::Sack => l.sacks += 1.0,
                PlayType::Pass if !pr.throw_away && pr.target.is_some() => {
                    l.att += 1.0;
                    l.cmp += pr.complete as u8 as f64;
                    l.xcmp += expected_completion_at(level, pr.air_yards, pr.separation, pr.pressured);
                    l.ints += pr.interception as u8 as f64;
                }
                _ => {}
            }
        }
    }
    l.points = r.stats.score[side] as f64;
    l.allowed = r.stats.score[1 - side] as f64;
    l
}

/// What each room's pregame talk leaves on the safety and the passer
/// before a snap (`talk::room`): his fire going in and after, his pitch,
/// how much reached him, the power and fine work, the habits, heat, focus
/// and poise. The own read is one draw of the head coach's pick.
fn landings(team: &GameTeam, level: Level, safety: u32, qb: u32) {
    use afm_engine::game::talk;
    let none = afm_engine::game::garbage::Context::default();
    let cx = talk::ctx(team, level, false, &none, talk::Moment::Pregame);
    println!("the pregame landing (stakes {:.2}):", cx.stakes);
    for (name, room) in rooms() {
        let pick = room.unwrap_or_else(|| talk::coach_pick(team, &cx, 0));
        let ls = talk::room(team, &pick, &cx);
        for (who, id) in [("S", safety), ("QB", qb)] {
            let Some(l) = ls.iter().find(|l| l.id == id) else { continue };
            println!(
                "  {name:<15} {who:<3} {:?}/{:?}/{:?}: fire {:.3} -> {:.3}, pitch {:.3}, ear {:.2}; power {:+.2}, fine {:+.2}, habits {:+.2}, heat {:+.3}, focus {:+.2}, poise {:+.2}, lift {:+.2}",
                pick.mv,
                pick.tone,
                pick.subject,
                l.before.fire,
                l.after.fire,
                l.spot,
                l.ear,
                l.power,
                l.fine,
                l.habits,
                l.heat,
                l.focus,
                l.poise,
                l.lift
            );
        }
    }
}

/// `stakes` above 0 sets the game's stakes for every team (0.8 is a
/// playoff game's); 0 lets the engine read them (0.25 in the test league).
fn run(games: usize, seed: u32, stakes: f32) {
    let level = Level::NFL;
    let mut base = synthetic_league(level, 32, seed);
    for t in &mut base {
        t.day.stakes = stakes;
    }
    let mut t0 = base[0].clone();
    let (safety, qb) = focus(&mut t0, level);
    landings(&t0, level, safety, qb);
    // Blocks of games for each difference's spread: 40 on a long run, so
    // the spread itself is steady.
    let blocks = if games >= 400 { 40usize } else { 8 };
    let t = std::time::Instant::now();
    let mut table: Vec<(&str, Vec<Line>)> = Vec::new();
    for (name, room) in rooms() {
        let mut me = t0.clone();
        me.day.room = room;
        let lines: Vec<Line> = (0..games)
            .into_par_iter()
            .map(|g| {
                let opp = &base[1 + g % 31];
                let s = seed as u64 * 1_000_003 + g as u64;
                if g.is_multiple_of(2) {
                    game_line(&quick_game(&me, opp, level, s), 0, safety, qb, level)
                } else {
                    game_line(&quick_game(opp, &me, level, s), 1, safety, qb, level)
                }
            })
            .collect();
        let per = games.div_ceil(blocks);
        let mut b = vec![Line::default(); blocks];
        for (g, l) in lines.iter().enumerate() {
            b[(g / per).min(blocks - 1)].add(l);
        }
        table.push((name, b));
    }
    println!("NFL, {games} games an arm, seed {seed}, stakes {stakes:.2}, {:.0} s", t.elapsed().as_secs_f64());
    let total = |b: &[Line]| {
        let mut l = Line::default();
        for x in b {
            l.add(x);
        }
        l
    };
    let g = games as f64;
    let own = total(&table[0].1).measures(g);
    let per = (games as f64 / blocks as f64).max(1.0);
    print!("{:<32}", "");
    for (name, _) in &table {
        print!("{name:>18}");
    }
    println!();
    for (k, m) in MEASURES.iter().enumerate() {
        print!("{m:<32}");
        for (j, (_, b)) in table.iter().enumerate() {
            let v = total(b).measures(g)[k];
            if j == 0 {
                print!("{v:>18.4}");
                continue;
            }
            // The difference against his own read, block by block.
            let d: Vec<f64> = b.iter().zip(&table[0].1).map(|(x, y)| x.measures(per)[k] - y.measures(per)[k]).collect();
            let mean = d.iter().sum::<f64>() / d.len() as f64;
            let sd = (d.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (d.len() - 1) as f64).sqrt();
            let se = sd / (d.len() as f64).sqrt();
            print!("{:>18}", format!("{:+.4} ({:.4})", v - own[k], se));
        }
        println!();
    }
    let s = total(&table[0].1);
    println!(
        "own read: the safety played {:.0} snaps a game, {:.1} tackles; the passer {:.1} dropbacks",
        s.snaps / g,
        s.tackles / g,
        s.dropbacks / g
    );
}

/// What main's talk was worth to a head coach before coach fire (the flat
/// lift on everything toward his position's pitch, and the focus an
/// overshoot cost), for `talks`: the pick a coach would have made then.
fn old_worth(team: &GameTeam, pick: &Pick, cx: &afm_engine::game::talk::Ctx, w: &[f64]) -> f64 {
    use afm_engine::game::talk;
    let ls = talk::room(team, pick, cx);
    let mut s = 0.0;
    for (k, l) in ls.iter().enumerate() {
        let ideal = talk::ideal_fire(team.players[k].position);
        let gap = |f: f32| (f as f64 - ideal).abs();
        let over = |f: f32| (f as f64 - ideal - 0.18).max(0.0);
        let (f0, f1) = (l.before.fire, l.after.fire);
        let lift = (4.0 * (gap(f0) - gap(f1)) + 2.0 * (l.after.poise - l.before.poise) as f64).clamp(-1.5, 1.5);
        let focus = (6.0 * (l.after.focus - l.before.focus) as f64 - 4.0 * (over(f1) - over(f0))).clamp(-2.0, 2.0);
        s += w[k] * (lift + 0.35 * focus + 0.15 * l.poise as f64 - 0.8 * (l.heat as f64).max(0.0));
    }
    s / w.iter().sum::<f64>().max(1.0)
}

/// The test league's head coaches' pregame picks (`talk::coach_pick`,
/// without and with the read's noise) against the picks main's worth would
/// make, and what each leaves on the men who play first: offense, defense,
/// specialists (fire going in, pitch, fire moved, power, fine work, habits,
/// heat). A check on how coach fire moved the AI's talks.
fn talks(level: Level, seed: u32) {
    use afm_engine::game::talk;
    let teams = synthetic_league(level, 32, seed);
    let none = afm_engine::game::garbage::Context::default();
    let name = |p: &Pick| format!("{:?}/{:?}/{:?}", p.mv, p.tone, p.subject);
    let mut new_picks = std::collections::BTreeMap::<String, usize>::new();
    let mut old_picks = std::collections::BTreeMap::<String, usize>::new();
    let mut acc = [[[0.0f64; 8]; 3]; 2];
    for (t, team) in teams.iter().enumerate() {
        for playoff in [false, true] {
            let cx = talk::ctx(team, level, playoff, &none, talk::Moment::Pregame);
            let w = talk::weights(team);
            let pick = talk::coach_pick(team, &cx, (seed as u64) << 8 | (t as u64) << 1 | playoff as u64);
            let old = talk::options(&cx, team)
                .into_iter()
                .max_by(|a, b| old_worth(team, a, &cx, &w).total_cmp(&old_worth(team, b, &cx, &w)))
                .unwrap_or_default();
            *new_picks.entry(name(&pick)).or_default() += 1;
            *old_picks.entry(name(&old)).or_default() += 1;
            for (j, pk) in [pick, old].iter().enumerate() {
                for (k, l) in talk::room(team, pk, &cx).iter().enumerate() {
                    if w[k] < 1.0 {
                        continue;
                    }
                    use Position as P;
                    let g = match team.players[k].position {
                        P::K | P::P | P::LS => 2,
                        P::QB | P::RB | P::WR | P::TE | P::OT | P::IOL => 0,
                        _ => 1,
                    };
                    let a = &mut acc[j][g];
                    a[0] += 1.0;
                    a[1] += l.before.fire as f64;
                    a[2] += l.spot as f64;
                    a[3] += (l.after.fire - l.before.fire) as f64;
                    a[4] += l.power as f64;
                    a[5] += l.fine as f64;
                    a[6] += l.habits as f64;
                    a[7] += l.heat as f64;
                }
            }
        }
    }
    println!("{} pregame picks, 32 teams, regular season and playoffs, seed {seed}", level.name());
    println!("  coach fire's picks (with the read's noise): {new_picks:?}");
    println!("  main's worth would pick (no noise):        {old_picks:?}");
    for (j, which) in ["coach fire's pick", "main's pick"].iter().enumerate() {
        for (g, side) in ["offense", "defense", "specialists"].iter().enumerate() {
            let a = acc[j][g];
            let n = a[0].max(1.0);
            println!(
                "  {which:<18} {side:<12} fire in {:.3}, pitch {:.3}, moved {:+.3}; power {:+.2}, fine {:+.2}, habits {:+.2}, heat {:+.3}",
                a[1] / n,
                a[2] / n,
                a[3] / n,
                a[4] / n,
                a[5] / n,
                a[6] / n,
                a[7] / n
            );
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("hits") => {
            let level = level_of(args.get(2).map(|s| s.as_str()));
            let games = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(400);
            let seed = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1);
            hits(level, games, seed);
        }
        Some("talks") => {
            let level = level_of(args.get(2).map(|s| s.as_str()));
            let seed = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
            talks(level, seed);
        }
        Some("run") => {
            let games = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(272);
            let seed = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
            let stakes = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            run(games, seed, stakes);
        }
        _ => eprintln!(
            "usage: fire_check hits <nfl|fbs|fcs|hs> [games] [seed] | fire_check talks <level> [seed] | fire_check run [games] [seed] [stakes]"
        ),
    }
}
