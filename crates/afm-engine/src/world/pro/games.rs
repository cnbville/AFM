//! NFL games on the match engine (section 5, topic 1): each team's game-day
//! actives from its depth chart, its staff's calling and installed systems,
//! the weather at its stadium. The box score goes on each player's season:
//! games, starts, snaps (and how strong the teams he took them against
//! were), his statistics and his injuries.

use std::collections::HashMap;

use rayon::prelude::*;

use super::{Status, depth_slots};
use crate::attributes::Attr;
use crate::calendar::Moment;
use crate::calendar::data::et_offset;
use crate::calendar::weather::weather_at;
use crate::game::playbook::{FAMILY_COUNT, Family};
use crate::game::{Caller, Conditions, GamePlayer, GameResult, GameTeam, Staff as GameStaff, Venue, quick_game_in};
use crate::positions::Position;
use crate::rng::clamp;
use crate::world::season::{key_normal, key_uniform};
use crate::world::{Stage, World, franchise, geo};

/// About a season of snaps for a starter on one side of the ball in the
/// NFL (real: about 1,000-1,100 offensive snaps a team plays in 17 games).
pub(crate) const NFL_SEASON_SNAPS: u32 = 1000;

/// A team ready for one game: the engine's side and, for each player, his
/// person and whether he starts.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Side {
    pub team: GameTeam,
    pub who: HashMap<u32, (usize, bool)>,
}

/// The game-day actives (the era's number): the starters and rotation at
/// every spot first, then the next man at each spot in turn (the backup
/// quarterback and linemen first), until the list is full. Players out
/// hurt are inactive.
pub(crate) fn actives(w: &World, t: usize) -> Vec<(usize, bool)> {
    let limit = w.rules.nfl_actives as usize;
    let club = &w.clubs[t];
    let healthy = |i: &&usize| {
        w.people[**i]
            .pro
            .as_ref()
            .is_some_and(|c| c.status == Status::Roster && c.out_weeks == 0 && crate::world::people::available(c))
    };
    let lists: Vec<Vec<usize>> = Position::ALL
        .iter()
        .map(|p| club.depth.get(p.index()).map_or(Vec::new(), |d| d.iter().filter(healthy).copied().collect()))
        .collect();
    let mut out: Vec<(usize, bool)> = Vec::new();
    for pos in Position::ALL {
        let s = depth_slots(pos);
        for (k, &i) in lists[pos.index()].iter().take(s.starters + s.rotation).enumerate() {
            out.push((i, k < s.starters));
        }
    }
    // The backups in turn: quarterback, line, then the rest.
    let order = [
        Position::QB,
        Position::IOL,
        Position::OT,
        Position::DT,
        Position::EDGE,
        Position::LB,
        Position::CB,
        Position::S,
        Position::WR,
        Position::RB,
        Position::TE,
    ];
    let mut depth = 0;
    while out.len() < limit && depth < 6 {
        let mut added = false;
        for pos in order {
            if out.len() >= limit {
                break;
            }
            let s = depth_slots(pos);
            if let Some(&i) = lists[pos.index()].get(s.starters + s.rotation + depth) {
                out.push((i, false));
                added = true;
            }
        }
        if !added {
            break;
        }
        depth += 1;
    }
    out.truncate(limit);
    super::squad::elevate(w, t, &mut out);
    out
}

/// A pro's snaps over his career so far, both sides of the ball.
fn pro_snaps(p: &super::super::Person) -> f64 {
    p.pro.as_ref().map_or(0.0, |c| c.seasons.iter().map(|s| (s.snaps[0] + s.snaps[1]) as f64).sum())
}

/// An NFL team for the match engine, its game-day actives dressed.
pub(crate) fn game_side(w: &World, t: usize) -> Option<Side> {
    build_side(w, t, actives(w, t))
}

/// A preseason team (topic 1, by era: four games, none in 2020, three
/// from 2021): the backups, rookies and camp bodies play; the starters sit
/// unless it is the staff's dress rehearsal (the next-to-last game) and the
/// head coach plays his starters then. Real habits run from a half of
/// football to none at all (Sean McVay's Rams from 2017); proposed, not
/// confirmed: each head coach has his own habit, and fewer play them from
/// 2017 on.
pub(crate) fn preseason_side(w: &World, t: usize, week: usize, weeks: usize) -> Option<Side> {
    let club = w.clubs.get(t)?;
    let coach = w.nfl[t].head_coach.unwrap_or(usize::MAX) as u64;
    let habit = key_uniform(0x9E5E, coach) - if w.year >= 2017 { 0.35 } else { 0.0 };
    let rehearsal = week + 2 == weeks && habit > 0.3;
    let mut list = Vec::new();
    for pos in Position::ALL {
        let s = depth_slots(pos);
        for (k, &i) in club.depth.get(pos.index()).map_or(&[][..], |d| d.as_slice()).iter().enumerate() {
            let ok = w.people[i]
                .pro
                .as_ref()
                .is_some_and(|c| c.status == Status::Roster && c.out_weeks == 0 && crate::world::people::available(c));
            if !ok || (k < s.starters && !rehearsal) {
                continue;
            }
            list.push((i, k < s.starters));
        }
    }
    build_side(w, t, list)
}

fn build_side(w: &World, t: usize, list: Vec<(usize, bool)>) -> Option<Side> {
    let club = w.clubs.get(t)?;
    let inst = club.installed;
    let fam = club.film.families();
    let fam_top = |off: bool| {
        let r = if off { 0..Family::Front as usize } else { Family::Front as usize..FAMILY_COUNT };
        fam[r].iter().copied().fold(0.0, f64::max)
    };
    let (top_off, top_def) = (fam_top(true), fam_top(false));
    let has_film = club.film.off_snaps >= 40.0;
    let mut who = HashMap::new();
    let mut players: Vec<GamePlayer> = Vec::with_capacity(list.len());
    for &(i, starter) in &list {
        let p = &w.people[i];
        let Some(c) = p.pro.as_ref() else { continue };
        // Seasons in this playbook: with this team, this one counted.
        let here = c.seasons.iter().filter(|s| s.team == Some(t)).count() as f64;
        let side = if super::super::systems::offense(p.player.position) { inst.off_known } else { inst.def_known };
        // Rust (topic 1.2): a player back from a long layoff is off his game
        // until he has a few games in (proposed, not confirmed: up to 10%
        // of what he knows, gone within three games).
        let rust = 1.0 - 0.1 * (1.0 - c.sharpness);
        let know = clamp(0.82 + 0.05 * here, 0.0, 0.97) * (0.8 + 0.2 * side) * rust;
        let experience =
            p.college_snaps as f64 / crate::world::season::SEASON_SNAPS as f64 + pro_snaps(p) / NFL_SEASON_SNAPS as f64;
        let mut g = GamePlayer::from_player(&p.player, know, experience);
        let iq = 0.5 * (p.player.attributes.get(Attr::FootballIq) + p.player.attributes.get(Attr::PlayRecognition));
        let mind = clamp(0.92 + 0.08 * (iq - 60.0) / 25.0, 0.82, 1.04);
        for f in Family::ALL {
            let (use_, top) = if f.offense() { (fam[f as usize], top_off) } else { (fam[f as usize], top_def) };
            let reps = if has_film && top > 0.0 { 0.78 + 0.22 * clamp(use_ / top, 0.0, 1.0).sqrt() } else { 0.92 };
            g.know[f as usize] = clamp(know * reps * mind, 0.0, 1.0) as f32;
        }
        who.insert(p.player.id, (i, starter));
        players.push(g);
    }
    if players.len() < 30 {
        return None;
    }
    // Line chemistry (topic 4.9): the share of the five most experienced
    // linemen who started up front here last season.
    let mut line: Vec<&super::super::Person> = list
        .iter()
        .map(|x| &w.people[x.0])
        .filter(|p| matches!(p.player.position, Position::OT | Position::IOL))
        .collect();
    line.sort_by(|a, b| pro_snaps(b).total_cmp(&pro_snaps(a)));
    let back = line
        .iter()
        .take(5)
        .filter(|p| {
            p.pro
                .as_ref()
                .is_some_and(|c| c.seasons.iter().any(|s| s.year == w.year - 1 && s.team == Some(t) && s.starts >= 8))
        })
        .count();
    let tm = &w.nfl[t];
    let ab = |c: Option<usize>| c.map(|c| *w.coaches[c].ability());
    let hc = ab(tm.head_coach);
    let caller = |c: Option<usize>, slot: usize| {
        let who = c.or(tm.head_coach);
        let a = ab(c).or(hc);
        let temper = who.map_or(10.0, |c| w.coaches[c].personality.ambition);
        let know = who.map_or(0.8, |c| w.coaches[c].systems.know[slot]);
        let id = who.unwrap_or(usize::MAX) as u64;
        let spread = |k: u64, sd: f64| if who.is_some() { sd * key_normal(k, id) } else { 0.0 };
        let fit = 0.7 + 0.3 * know;
        let base = a.map_or(10.0, |a| 0.6 * a.scheme + 0.4 * a.game_management);
        let (shift, adaptation) = crate::world::season::your_calls(w, who, base, a.map_or(10.0, |a| a.game_management));
        let reading = a
            .map_or(10.0, |a| clamp(0.7 * a.scheme + 0.3 * a.game_management + spread(0x5EAD, 2.0) + shift, 1.0, 20.0));
        let situational = a
            .map_or(10.0, |a| clamp(0.7 * a.game_management + 0.3 * a.scheme + spread(0x517A, 2.0) + shift, 1.0, 20.0));
        let creativity = a.map_or(10.0, |a| clamp(a.scheme + spread(0xC4EA, 3.0) + shift, 1.0, 20.0));
        let nerve = clamp(0.04 * (temper - 10.0) + spread(0x4E4E, 0.4), -1.0, 1.0);
        Caller {
            skill: (base + shift) * fit,
            adaptation,
            aggressiveness: temper,
            reading: reading * fit,
            situational,
            creativity: creativity * fit,
            nerve,
        }
    };
    // Film study: the coordinators' eye and the quality-control staff; an
    // NFL budget buys a full one, a cheap owner a little less (proposed,
    // not confirmed).
    let eye = |c: Option<usize>| ab(c.or(tm.head_coach)).map_or(10.0, |a| a.scheme);
    // Section 6 topic 10: the analytics and quality-control head's work
    // takes the budget's place.
    let analytics = crate::world::people::Support::z(club.support.analytics.rating);
    let budget = clamp(0.85 + 0.15 * analytics, 0.7, 1.0);
    let film_study = clamp(0.35 * (eye(tm.oc) + eye(tm.dc)) + 7.0 * budget, 1.0, 20.0);
    let oc_call = crate::world::season::play_caller(w, tm.oc, tm.head_coach, true);
    let dc_call = crate::world::season::play_caller(w, tm.dc, tm.head_coach, false);
    let dc = dc_call;
    let aware = players.iter().map(|p| p.a(Attr::FootballIq)).sum::<f64>() / players.len().max(1) as f64;
    let install = {
        let oc = caller(oc_call, inst.off);
        let who = oc_call.unwrap_or(usize::MAX) as u64;
        let miss = 0.15 * (1.0 - oc.skill / 20.0).max(0.0) * key_normal(0x1257 ^ w.year as u64, who);
        clamp(crate::game::roster::absorb(aware) + miss + 0.01 * (oc.aggressiveness - 10.0), 0.35, 1.0)
    };
    let mut team = GameTeam::new(
        tm.name.to_string(),
        players,
        GameStaff {
            offense: caller(oc_call, inst.off),
            defense: caller(dc_call, 7 + inst.shell),
            game_management: hc.map_or(10.0, |a| a.game_management),
            strictness: hc.map_or(10.0, |a| a.discipline),
            off_scheme: inst.offense(),
            front: inst.front(),
            shell: inst.shell(),
            playbook: 1.0,
            sportsmanship: tm.head_coach.map_or(10.0, |c| w.coaches[c].sportsmanship),
            nickel: dc.map_or(0.9, |c| clamp(0.9 + 0.06 * key_normal(0x41C8, c as u64), 0.6, 0.99)),
            year: w.year,
            film_study,
            install,
            style: tm.head_coach.map_or_else(Default::default, |c| w.coaches[c].style()),
        },
    );
    team.give_orders(crate::player::Level::NFL);
    if w.seat_club(t) {
        team.order = w.seat_depth.clone();
    }
    team.film = club.film.clone();
    team.line_chem = 0.25 + 0.15 * back as f64;
    Some(Side { team, who })
}

/// The weather and crowd at an NFL game: the home team's stadium in its
/// era (a dome is indoors; a retractable roof closes for cold, wind or
/// rain), or a neutral site.
pub(crate) fn venue(w: &World, home: usize, away: usize, at: Moment, neutral: bool, loud: f64) -> Venue {
    let year = w.year;
    let seed = w.options.seed as u64;
    let era = franchise::era(home, year.min(franchise::PRESENT));
    let code = era.map_or("NY", |e| e.state);
    let state = geo::state_index(code).unwrap_or(0);
    let local = (at.minute / 60) as i32 - et_offset(code);
    // Its stadium's roof: the real one through today, a new stadium's after.
    let roof = if neutral {
        era.map_or(franchise::Roof::Open, |e| e.roof)
    } else {
        crate::world::finance::nfl::roof(w, home, year)
    };
    let outside = weather_at(seed, state, code, at.date, local.clamp(9, 23) as u32);
    let closed = match roof {
        franchise::Roof::Dome => true,
        franchise::Roof::Retractable => outside.temperature < 55.0 || outside.rain || outside.wind > 15.0,
        franchise::Roof::Open => false,
    };
    let wx = if neutral || closed { crate::calendar::weather::indoors() } else { outside };
    let along = (key_uniform(seed ^ 0xA11, (at.date.0 as u64) << 16 ^ (home as u64) << 8 ^ away as u64) * 2.0 - 1.0)
        .clamp(-1.0, 1.0);
    let altitude = if code == "CO" { 5280.0 } else { 0.0 };
    // A full NFL stadium: loud, louder at night and when it matters.
    let crowd = if neutral { 0.3 } else { clamp(0.68 + if local >= 18 { 0.08 } else { 0.0 } + loud, 0.1, 1.0) };
    let cond = Conditions {
        temperature: wx.temperature,
        wind: wx.wind,
        wind_along: along,
        rain: wx.rain,
        snow: wx.snow,
        altitude,
        crowd,
        turf: closed,
        ..Conditions::default()
    };
    // Travel (proposed, not confirmed: the college rule's weights): miles,
    // time zones and a body clock kickoff.
    let home_of = |t: usize| franchise::era(t, year.min(franchise::PRESENT)).map_or("NY", |e| e.state);
    let wear_of = |t: usize| {
        if t == home && !neutral {
            return 0.0;
        }
        let from = home_of(t);
        let a = geo::state_index(from).unwrap_or(0);
        let miles = geo::miles(a, state).min(2500.0);
        let body = (at.minute / 60) as i32 - et_offset(from);
        let early = (11 - body).max(0) as f64;
        let thin = if code == "CO" && from != "CO" { 0.03 } else { 0.0 };
        0.015 * miles / 1000.0 + 0.008 * (et_offset(code) - et_offset(from)).abs() as f64 + 0.01 * early + thin
    };
    let cold_of = |t: usize| {
        if closed || neutral {
            return 0.0;
        }
        let e = franchise::era(t, year.min(franchise::PRESENT));
        let usual = match e {
            Some(e) if e.roof == franchise::Roof::Dome => 72.0,
            Some(e) => crate::calendar::weather::usual_high(e.state, at.date),
            None => 60.0,
        };
        clamp((usual - wx.temperature - 15.0) / 25.0, 0.0, 1.0) * clamp((45.0 - wx.temperature) / 15.0, 0.0, 1.0)
    };
    Venue { cond, neutral, wear: [wear_of(home), wear_of(away)], cold_unused: [cold_of(home), cold_of(away)] }
}

/// One NFL game to play: home, away, kickoff, a neutral site, how much it
/// matters to the home crowd.
pub(crate) struct Fixture {
    pub home: usize,
    pub away: usize,
    pub at: Moment,
    pub neutral: bool,
    pub loud: f64,
    pub seed: u64,
}

/// Plays a day's games on the engine, all cores. A team that can't field
/// 30 players gets None (the stand-in decides).
pub(crate) fn play(w: &World, games: &[Fixture]) -> Vec<Option<(GameResult, [Side; 2])>> {
    let mut sides: HashMap<usize, Option<Side>> = HashMap::new();
    for g in games {
        for t in [g.home, g.away] {
            sides.entry(t).or_insert_with(|| game_side(w, t));
        }
    }
    let jobs: Vec<Option<(Side, Side, Venue, u64)>> = games
        .iter()
        .map(|g| {
            let a = sides.get(&g.home)?.clone()?;
            let b = sides.get(&g.away)?.clone()?;
            Some((a, b, venue(w, g.home, g.away, g.at, g.neutral, g.loud), g.seed))
        })
        .collect();
    jobs.into_par_iter()
        .map(|j| {
            let (a, b, v, seed) = j?;
            let r = quick_game_in(&a.team, &b.team, crate::player::Level::NFL, seed, Default::default(), v);
            Some((r, [a, b]))
        })
        .collect()
}

/// A preseason game: no stats on the season, but the staff's camp read
/// (each player's grade for the game, weighted by his snaps) and the
/// injuries count.
pub(crate) fn record_preseason(w: &mut World, r: &GameResult, sides: &[Side; 2]) {
    let year = w.year;
    for side in 0..2 {
        for (&id, l) in &r.stats.players[side] {
            let Some(&(i, _)) = sides[side].who.get(&id) else { continue };
            let Some(c) = w.people[i].pro.as_mut() else { continue };
            let n = (l.snaps_off + l.snaps_def) as f64;
            if n > 0.0 && l.graded > 0 {
                let (g, m) = c.preseason;
                c.preseason = (g + l.grade() * n, m + n);
            }
        }
    }
    for (_, inj) in &r.injuries {
        if inj.plays_on || inj.side > 1 {
            continue;
        }
        let Some(&(i, _)) = sides[inj.side].who.get(&inj.id) else { continue };
        let weeks = inj.weeks.ceil().max(1.0) as u32;
        let Some(c) = w.people[i].pro.as_mut() else { continue };
        c.out_weeks = c.out_weeks.max(weeks);
        if weeks >= 6 {
            // A vested veteran goes straight to injured reserve; anyone else
            // hurt before the cutdown must be waived/injured first (real
            // rule), so the cutdown decides: kept, he goes to reserve after
            // it; not kept, he is released with an injury settlement
            // (trial 9: every camp injury went to reserve, so settlements
            // almost never happened).
            let vested = c.accrued >= 4;
            if vested {
                c.status = Status::Reserve;
                c.pup = false;
            }
            let p = &mut w.people[i];
            let age = p.row_age();
            p.note(format!(
                "Hurt in the preseason at {age} ({}, about {weeks} weeks){}.",
                inj.kind.name(),
                if vested { ": injured reserve" } else { "" }
            ));
        }
    }
    let _ = year;
}

/// Puts a game on the players' seasons and the teams' film: games, starts,
/// snaps and the opponent's strength, the box score, and injuries (a
/// player out for a long spell goes on injured reserve).
pub(crate) fn record(w: &mut World, teams: [usize; 2], r: &GameResult, sides: &[Side; 2], opp_z: [f64; 2]) {
    let year = w.year;
    w.keep_seat_game(crate::calendar::Level::Nfl, teams, &r.stats);
    if w.options.keep_game_log && w.recording {
        w.game_log.add_nfl(year, teams, &r.stats);
    }
    for side in 0..2 {
        let t = teams[side];
        if let Some(c) = w.clubs.get_mut(t) {
            c.film.add(&r.film[side]);
        }
        // Every practice squad player dressed for the game used one of his
        // elevations, whether he played a snap or not (bug check, build 9:
        // only those with a stat line counted).
        for &(i, _) in sides[side].who.values() {
            super::squad::count_elevation(w, i);
        }
        for (&id, l) in &r.stats.players[side] {
            let Some(&(i, starter)) = sides[side].who.get(&id) else { continue };
            let Some(c) = w.people[i].pro.as_mut() else { continue };
            let dressed = l.snaps_off + l.snaps_def + l.snaps_st > 0;
            if dressed {
                // A game back sharpens him (rust lasts at most three games).
                c.sharpness = (c.sharpness + 0.34).min(1.0);
            }
            let s = c.season_mut(year);
            s.team = Some(t);
            if dressed {
                s.games += 1;
                if starter {
                    s.starts += 1;
                }
            }
            s.snaps[0] += l.snaps_off;
            s.snaps[1] += l.snaps_def;
            s.snaps[2] += l.snaps_st;
            s.opp += opp_z[side] * (l.snaps_off + l.snaps_def) as f64;
            s.stats.add(l);
        }
    }
    for (_, inj) in &r.injuries {
        if inj.plays_on || inj.side > 1 {
            continue;
        }
        let Some(&(i, _)) = sides[inj.side].who.get(&inj.id) else { continue };
        // The medical staff's head: a good one brings a player back sooner
        // (section 6 topic 10; proposed, not confirmed: up to 12%).
        let t = teams[inj.side];
        let med = w.clubs.get(t).map_or(0.0, |c| crate::world::people::Support::z(c.support.medical.rating));
        let weeks = (inj.weeks * (1.0 - 0.12 * med)).ceil().max(1.0) as u32;
        let ir = weeks >= ir_minimum(year);
        let Some(c) = w.people[i].pro.as_mut() else { continue };
        c.out_weeks = c.out_weeks.max(weeks);
        if ir && c.status == Status::Roster {
            c.status = Status::Reserve;
            c.pup = false;
            let t = teams[inj.side];
            let name = w.nfl[t].name;
            let p = &mut w.people[i];
            let age = p.row_age();
            p.note(format!(
                "Placed on injured reserve by the {name} at {age} ({}, about {weeks} weeks).",
                inj.kind.name()
            ));
        }
    }
    let _ = Stage::Pro;
}

/// A week of practice (Tuesday, from the veterans' report to the Super
/// Bowl): each healthy player on a roster or practice squad may get hurt
/// in practice, more in camp's long padded days than in season, more for
/// the injury-prone and for older bodies (soft tissue). Trial 9 finding
/// 13: injuries only came from games, while real practices give about
/// 41% of NFL time-loss injuries (lower body, 2015-2018: Mack et al., Am J
/// Sports Med 2020, PMID 32485114). The mix leans to hamstrings and
/// ankles; how long each lasts is the game's (`injury_weeks`). A camp
/// injury is handled as a preseason one (the cutdown decides a
/// non-vested player), a season one as a game's (injured reserve past its
/// minimum). Proposed, not confirmed: about 1.0 week-plus injuries a team
/// a camp week and 0.35 a season week.
pub(crate) fn practice_injuries(w: &mut World, d: crate::calendar::Date) {
    use crate::game::sim::{InjuryKind as K, injury_weeks};
    let year = w.year;
    let camp = d < w.dates.cutdown.date;
    let base = if camp { 0.0155 } else { 0.009 };
    const MIX: [(K, f64); 12] = [
        (K::Hamstring, 22.0),
        (K::AnkleSprain, 15.0),
        (K::Knee, 14.0),
        (K::Bruise, 10.0),
        (K::Shoulder, 7.0),
        (K::Back, 6.0),
        (K::Hand, 6.0),
        (K::HighAnkleSprain, 4.0),
        (K::Concussion, 4.0),
        (K::Acl, 2.5),
        (K::Ribs, 2.0),
        (K::Fracture, 2.0),
    ];
    let seed = w.options.seed as u64 ^ 0x9AC7 ^ (d.0 as u64) << 20;
    for t in franchise::active_teams(year) {
        let med = crate::world::people::Support::z(w.clubs[t].support.medical.rating);
        let name = w.nfl[t].name;
        let roster = w.clubs[t].roster.clone();
        for i in roster {
            let p = &w.people[i];
            let Some(c) = p.pro.as_ref() else { continue };
            if !matches!(c.status, Status::Roster | Status::PracticeSquad) || c.out_weeks > 0 {
                continue;
            }
            let prone = 0.55 + p.player.injury_proneness / 18.0;
            let age = 1.0 + 0.03 * (p.player.age as f64 - 26.0).max(0.0);
            let spec = if p.player.position.is_specialist() { 0.25 } else { 1.0 };
            if key_uniform(seed, p.player.id as u64) >= base * prone * age * spec {
                continue;
            }
            let mut rng = crate::rng::Rng::new((seed as u32) ^ p.player.id.wrapping_mul(0x9E37_79B1));
            let kind = rng.weighted(&MIX);
            let recovery = p.player.game.recovery;
            let weeks = injury_weeks(kind, &mut rng) * (1.3 - 0.03 * recovery) * (1.0 - 0.12 * med);
            let weeks = weeks.ceil() as u32;
            if weeks == 0 {
                continue;
            }
            let vested = c.accrued >= 4;
            let squad = c.status == Status::PracticeSquad;
            let c = w.people[i].pro.as_mut().unwrap();
            c.out_weeks = c.out_weeks.max(weeks);
            let ir = if camp { weeks >= 6 && vested } else { weeks >= ir_minimum(year) && !squad };
            if ir {
                c.status = Status::Reserve;
                c.pup = false;
            }
            let p = &mut w.people[i];
            let age = p.row_age();
            if camp && weeks >= 6 {
                p.note(format!(
                    "Hurt in camp practice at {age} ({}, about {weeks} weeks){}.",
                    kind.name(),
                    if ir { ": injured reserve" } else { "" }
                ));
            } else if ir {
                p.note(format!(
                    "Placed on injured reserve by the {name} at {age} ({} in practice, about {weeks} weeks).",
                    kind.name()
                ));
            }
        }
    }
}

/// The shortest absence a team puts on injured reserve: before 2012 the
/// list ended a season, so only long injuries went on it; from 2020 a
/// player can come back after a few games (proposed, not confirmed: 6
/// weeks, then 4).
pub(crate) fn ir_minimum(year: i32) -> u32 {
    if year >= 2020 { 4 } else { 6 }
}

/// How many players a team may bring back from injured reserve in a
/// season (real: none before 2012, one 2012-16, two 2017-19, unlimited in
/// 2020-21, eight from 2022).
pub(crate) fn ir_returns(year: i32) -> u32 {
    match year {
        ..=2011 => 0,
        2012..=2016 => 1,
        2017..=2019 => 2,
        2020 | 2021 => 99,
        _ => 8,
    }
}
