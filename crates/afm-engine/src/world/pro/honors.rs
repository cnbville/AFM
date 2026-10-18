//! NFL honors (section 5): the Pro Bowl, the All-Pro teams, the season
//! awards, the Super Bowl MVP, the NFL record book and the Pro Football Hall
//! of Fame. Voting follows the real rules and voter habits (honors voting
//! was settled before section 5): stats and film against the season's best
//! at each position, a winning team for the MVP, reputation for the Pro
//! Bowl. The weights are Claude's, proposed, not confirmed.

use std::collections::HashMap;

use super::Status;
use crate::calendar::{Date, Event, Kind, Level, Loudness};
use crate::game::stats::StatLine;
use crate::positions::Position;
use crate::world::{Stage, World};

/// One record: the value, who holds it and since when.
#[derive(Clone, Debug)]
pub struct NflRecord {
    pub stat: &'static str,
    pub career: bool,
    pub value: f64,
    pub holder: String,
    pub year: i32,
    /// A world player (false: the real holder before history began).
    pub world: bool,
}

/// A retired player waiting on the Hall of Fame vote.
#[derive(Clone, Debug)]
pub struct HallCandidate {
    pub id: u32,
    pub name: String,
    pub pos: Position,
    pub retired: i32,
    pub score: f64,
    pub why: String,
}

/// The league's book of honors.
#[derive(Clone, Debug, Default)]
pub struct NflBook {
    /// (season, award, player id, name, team).
    pub awards: Vec<(i32, &'static str, u32, String, usize)>,
    /// (season, player id, team): Pro Bowl selections.
    pub pro_bowls: Vec<(i32, u32, usize)>,
    /// (season, first or second team, player id, position, team).
    pub all_pro: Vec<(i32, u8, u32, Position, usize)>,
    pub records: Vec<NflRecord>,
    /// (season, stat, holder, value) for every record broken in the world.
    pub broken: Vec<(i32, String, String, f64)>,
    /// Inductees: (year, player id, name, position, why).
    pub hall: Vec<(i32, u32, String, Position, String)>,
    pub candidates: Vec<HallCandidate>,
    /// Career totals by player id, for the career records.
    pub careers: HashMap<u32, StatLine>,
}

/// The real NFL records before the 1984 season (from memory, to verify):
/// season and career marks for the stats the record book follows.
const REAL_1983: [(&str, bool, f64, &str, i32); 18] = [
    ("passing yards", false, 4802.0, "Dan Fouts", 1981),
    ("passing touchdowns", false, 36.0, "George Blanda", 1961),
    ("rushing yards", false, 2003.0, "O. J. Simpson", 1973),
    ("rushing touchdowns", false, 24.0, "John Riggins", 1983),
    ("receptions", false, 101.0, "Charley Hennigan", 1964),
    ("receiving yards", false, 1746.0, "Charley Hennigan", 1961),
    ("receiving touchdowns", false, 17.0, "Don Hutson", 1942),
    ("sacks", false, 19.0, "Mark Gastineau", 1983),
    ("interceptions", false, 14.0, "Night Train Lane", 1952),
    ("passing yards", true, 47003.0, "Fran Tarkenton", 1978),
    ("passing touchdowns", true, 342.0, "Fran Tarkenton", 1978),
    ("rushing yards", true, 12312.0, "Jim Brown", 1965),
    ("rushing touchdowns", true, 106.0, "Jim Brown", 1965),
    ("receptions", true, 649.0, "Charley Taylor", 1977),
    ("receiving yards", true, 11834.0, "Don Maynard", 1973),
    ("receiving touchdowns", true, 99.0, "Don Hutson", 1945),
    ("interceptions", true, 81.0, "Paul Krause", 1979),
    ("field goals", false, 35.0, "Ali Haji-Sheikh", 1983),
];

fn stat_of(l: &StatLine, stat: &str) -> f64 {
    match stat {
        "passing yards" => l.pass_yds,
        "passing touchdowns" => l.pass_td as f64,
        "rushing yards" => l.rush_yds,
        "rushing touchdowns" => l.rush_td as f64,
        "receptions" => l.rec as f64,
        "receiving yards" => l.rec_yds,
        "receiving touchdowns" => l.rec_td as f64,
        "sacks" => l.sacks,
        "interceptions" => l.ints as f64,
        "field goals" => l.fg_made.iter().sum::<u32>() as f64,
        _ => 0.0,
    }
}

impl NflBook {
    pub fn new() -> NflBook {
        let records = REAL_1983
            .iter()
            .map(|&(stat, career, value, holder, year)| NflRecord {
                stat,
                career,
                value,
                holder: holder.to_string(),
                year,
                world: false,
            })
            .collect();
        NflBook { records, ..Default::default() }
    }

    /// Honors a player has: (Pro Bowls, first-team All-Pros, second-team, MVPs).
    pub fn honors_of(&self, id: u32) -> (u32, u32, u32, u32) {
        let pb = self.pro_bowls.iter().filter(|x| x.1 == id).count() as u32;
        let ap1 = self.all_pro.iter().filter(|x| x.2 == id && x.1 == 1).count() as u32;
        let ap2 = self.all_pro.iter().filter(|x| x.2 == id && x.1 == 2).count() as u32;
        let mvp = self.awards.iter().filter(|x| x.2 == id && x.1 == "MVP").count() as u32;
        (pb, ap1, ap2, mvp)
    }
}

/// What the voters count in a season line: a passer's, runner's or
/// receiver's production, a kicker's field goals, a defender's tackles,
/// sacks, stops behind the line, takeaways and breakups. Blockers have no
/// box-score line.
pub(crate) fn stats_value(l: &StatLine, pos: Position) -> f64 {
    use Position::*;
    let scrimmage = (l.rush_yds + l.rec_yds) / 10.0 + 6.0 * (l.rush_td + l.rec_td) as f64 + 0.5 * l.rec as f64;
    let passing = l.pass_yds / 20.0 + 4.0 * l.pass_td as f64 - 3.0 * l.int as f64;
    let (tackles, sacks, tfl, ints, pbus) =
        (l.tackles as f64 + 0.5 * l.assists as f64, l.sacks, l.tfl, l.ints as f64, l.pbus as f64);
    match pos {
        QB => passing + scrimmage,
        RB | WR | TE => scrimmage,
        K => {
            const MADE: [f64; 4] = [1.0, 1.0, 1.25, 1.75];
            const MISSED: [f64; 4] = [2.0, 1.5, 1.0, 0.5];
            (0..4)
                .map(|b| {
                    let made = l.fg_made[b].min(l.fg_att[b]) as f64;
                    MADE[b] * made - MISSED[b] * (l.fg_att[b] as f64 - made)
                })
                .sum()
        }
        P => (l.punt_net / l.punts.max(1) as f64 - 36.0) * l.punts as f64 / 10.0 + l.punt_in20 as f64,
        OT | IOL | LS => 0.0,
        LB => tackles + 6.0 * sacks + 4.0 * tfl + 10.0 * ints + 3.0 * pbus + 4.0 * l.forced_fumbles as f64,
        CB | S => 0.5 * tackles + 6.0 * sacks + 4.0 * tfl + 10.0 * ints + 4.0 * pbus + 4.0 * l.forced_fumbles as f64,
        EDGE | DT => {
            0.6 * tackles
                + 6.0 * sacks
                + 4.0 * tfl
                + 1.5 * l.qb_hits as f64
                + 10.0 * ints
                + 3.0 * pbus
                + 4.0 * l.forced_fumbles as f64
        }
    }
}

/// How much of a vote rests on film: all of it for blockers, half for
/// defenders, a little for the skill players (the college awards' split,
/// with NFL voters watching more tape).
fn film_weight(pos: Position) -> f64 {
    use Position::*;
    match pos {
        QB | RB | WR | TE => 0.2,
        K | P => 0.0,
        OT | IOL | LS => 1.0,
        _ => 0.5,
    }
}

/// One season's candidate.
struct Cand {
    i: usize,
    id: u32,
    pos: Position,
    team: usize,
    conf: usize,
    stats: f64,
    film: f64,
    rookie: bool,
}

/// The season's candidates: players with eight games or more.
fn candidates(w: &World) -> Vec<Cand> {
    let year = w.year;
    let conf = w.nfl_season.as_ref().map(|s| s.conf.clone()).unwrap_or_default();
    w.people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.stage == Stage::Pro)
        .filter_map(|(i, p)| {
            let c = p.pro.as_ref()?;
            let s = c.seasons.iter().find(|s| s.year == year)?;
            let team = s.team?;
            if s.games < 8 {
                return None;
            }
            let pos = p.player.position;
            // A season on the stand-in has no box score: the voters go by
            // how good he is.
            let blank = s.stats.graded == 0 && s.stats.snaps_off + s.stats.snaps_def == 0;
            let level = (p.player.overall() - 55.0).max(0.0);
            Some(Cand {
                i,
                id: p.player.id,
                pos,
                team,
                conf: conf.get(team).copied().unwrap_or(0),
                stats: if blank { level } else { stats_value(&s.stats, pos) },
                film: if blank { level } else { s.stats.grade_sum },
                rookie: c.seasons.iter().all(|x| x.year == year || x.games == 0),
            })
        })
        .collect()
}

/// Each candidate's read against the season's ten best at his position.
fn reads(cands: &[Cand]) -> Vec<f64> {
    let mut bars: HashMap<Position, (f64, f64)> = HashMap::new();
    for pos in Position::ALL {
        let top = |mut v: Vec<f64>| {
            v.sort_by(|a, b| b.total_cmp(a));
            let t = &v[..v.len().min(10)];
            (t.iter().sum::<f64>() / t.len().max(1) as f64).max(1.0)
        };
        let stats = cands.iter().filter(|c| c.pos == pos).map(|c| c.stats).collect();
        let film = cands.iter().filter(|c| c.pos == pos).map(|c| c.film).collect();
        bars.insert(pos, (top(stats), top(film)));
    }
    cands
        .iter()
        .map(|c| {
            let fw = film_weight(c.pos);
            let (bs, bf) = bars[&c.pos];
            (1.0 - fw) * c.stats / bs + fw * c.film / bf
        })
        .collect()
}

/// All-Pro places per team at each position (the AP's teams, from memory,
/// to verify: three receivers, three interior linemen, two of most others).
fn all_pro_slots(pos: Position) -> usize {
    use Position::*;
    match pos {
        QB | RB | TE | K | P => 1,
        WR | IOL => 3,
        LS => 0,
        _ => 2,
    }
}

/// Pro Bowl places per conference (from memory, to verify).
fn pro_bowl_slots(pos: Position) -> usize {
    use Position::*;
    match pos {
        QB | RB | OT | EDGE | DT | S => 3,
        WR | CB | LB => 4,
        TE => 2,
        IOL => 5,
        K | P | LS => 1,
    }
}

/// The regular season's honors, after its last week: Pro Bowl rosters
/// (with the fans' and players' pull toward past selections), the All-Pro
/// teams and the awards.
pub(crate) fn season_honors(w: &mut World, d: Date) {
    if w.nfl_book.records.is_empty() {
        w.nfl_book = NflBook::new();
    }
    let year = w.year;
    let cands = candidates(w);
    if cands.is_empty() {
        return;
    }
    let read = reads(&cands);
    let wins: Vec<f64> = w
        .nfl_season
        .as_ref()
        .map_or(Vec::new(), |s| (0..s.wins.len()).map(|t| s.wins[t] as f64 + 0.5 * s.ties[t] as f64).collect());
    let games = w.nfl_season.as_ref().map_or(16.0, |s| s.weeks.len().saturating_sub(1).max(16) as f64);
    let win_share = |t: usize| wins.get(t).copied().unwrap_or(games / 2.0) / games;
    // The Pro Bowl: each conference's best by read, reputation counting.
    let mut pro_bowl: Vec<usize> = Vec::new();
    for conf in 0..2 {
        for pos in Position::ALL {
            let mut v: Vec<(usize, f64)> = cands
                .iter()
                .enumerate()
                .filter(|(_, c)| c.conf == conf && c.pos == pos)
                .map(|(k, c)| {
                    let past = w.nfl_book.pro_bowls.iter().filter(|x| x.1 == c.id).count() as f64;
                    (k, read[k] * (1.0 + 0.06 * past.min(5.0)) * (0.9 + 0.2 * win_share(c.team)))
                })
                .collect();
            v.sort_by(|a, b| b.1.total_cmp(&a.1));
            pro_bowl.extend(v.iter().take(pro_bowl_slots(pos)).map(|x| x.0));
        }
    }
    // All-Pro: the AP's voters, film and stats with a little for winning.
    let mut all_pro: Vec<(usize, u8)> = Vec::new();
    for pos in Position::ALL {
        let n = all_pro_slots(pos);
        let mut v: Vec<(usize, f64)> = cands
            .iter()
            .enumerate()
            .filter(|(_, c)| c.pos == pos)
            .map(|(k, c)| (k, read[k] * (0.95 + 0.1 * win_share(c.team))))
            .collect();
        v.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (r, x) in v.iter().take(2 * n).enumerate() {
            all_pro.push((x.0, if r < n { 1 } else { 2 }));
        }
    }
    // The awards. MVP: real winners are nearly all quarterbacks on winning
    // teams (about 4 in 5 since 2000); a back or a defender needs a huge year.
    let pick = |filter: &dyn Fn(&Cand) -> bool, weight: &dyn Fn(&Cand, f64) -> f64| -> Option<usize> {
        cands
            .iter()
            .enumerate()
            .filter(|(_, c)| filter(c))
            .map(|(k, c)| (k, weight(c, read[k])))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|x| x.0)
    };
    let offense = |p: Position| matches!(p, Position::QB | Position::RB | Position::WR | Position::TE);
    let defense = |p: Position| matches!(p, Position::EDGE | Position::DT | Position::LB | Position::CB | Position::S);
    let pos_weight = |p: Position| match p {
        Position::QB => 1.0,
        Position::RB => 0.8,
        Position::WR | Position::TE => 0.7,
        _ => 0.55,
    };
    let mvp =
        pick(&|c| offense(c.pos) || defense(c.pos), &|c, r| r * pos_weight(c.pos) * (0.4 + 1.2 * win_share(c.team)));
    let opoy = pick(&|c| offense(c.pos), &|c, r| r * if c.pos == Position::QB { 0.9 } else { 1.0 });
    let dpoy =
        pick(&|c| defense(c.pos), &|c, r| r * if matches!(c.pos, Position::EDGE | Position::DT) { 1.1 } else { 1.0 });
    let oroy = pick(&|c| c.rookie && offense(c.pos), &|_, r| r);
    let droy = pick(&|c| c.rookie && defense(c.pos), &|_, r| r);
    // Comeback: a good season after one lost to injury or a slump.
    let comeback = pick(
        &|c| {
            let last = w.people[c.i].pro.as_ref().and_then(|p| p.seasons.iter().find(|s| s.year == year - 1));
            last.is_some_and(|s| s.games <= 6 || s.ir_weeks >= 6)
        },
        &|_, r| r,
    );
    let mut lines: Vec<(usize, String, Loudness)> = Vec::new();
    for (k, award) in [
        (mvp, "MVP"),
        (opoy, "Offensive Player of the Year"),
        (dpoy, "Defensive Player of the Year"),
        (oroy, "Offensive Rookie of the Year"),
        (droy, "Defensive Rookie of the Year"),
        (comeback, "Comeback Player of the Year"),
    ] {
        let Some(k) = k else { continue };
        let c = &cands[k];
        let name = w.people[c.i].name.clone();
        w.nfl_book.awards.push((year, award, c.id, name.clone(), c.team));
        let age = w.people[c.i].row_age();
        w.people[c.i].note(format!("Named the NFL's {award} at {age}."));
        let s = w.people[c.i].pro.as_mut().unwrap().season_mut(year);
        s.awards.push(award);
        let loud = if award == "MVP" { Loudness::Big } else { Loudness::Medium };
        lines.push((c.team, format!("{name} ({}, {}) is the NFL's {award}.", c.pos, w.nfl[c.team].name), loud));
    }
    for k in pro_bowl {
        let c = &cands[k];
        w.nfl_book.pro_bowls.push((year, c.id, c.team));
        let s = w.people[c.i].pro.as_mut().unwrap().season_mut(year);
        s.pro_bowl = true;
        let age = w.people[c.i].row_age();
        w.people[c.i].note(format!("Selected to the Pro Bowl at {age}."));
    }
    for (k, team) in all_pro {
        let c = &cands[k];
        w.nfl_book.all_pro.push((year, team, c.id, c.pos, c.team));
        let s = w.people[c.i].pro.as_mut().unwrap().season_mut(year);
        s.all_pro = team;
        let age = w.people[c.i].row_age();
        let which = if team == 1 { "first" } else { "second" };
        w.people[c.i].note(format!("Named {which}-team All-Pro at {age}."));
    }
    for (t, text, loud) in lines {
        w.calendar.post(Event::new(d, Level::Nfl, Kind::News, loud, text).team(t));
    }
    season_records(w, d);
}

/// The record book after the regular season: season marks, then career
/// marks (career totals add the season's line).
fn season_records(w: &mut World, d: Date) {
    let year = w.year;
    let mut lines: Vec<(u32, String, StatLine)> = Vec::new();
    for p in w.people.iter().filter(|p| p.stage == Stage::Pro) {
        let Some(c) = p.pro.as_ref() else { continue };
        if let Some(s) = c.seasons.iter().find(|s| s.year == year) {
            lines.push((p.player.id, p.name.clone(), s.stats.clone()));
        }
    }
    let mut posts = Vec::new();
    for (id, name, line) in lines {
        let career = {
            let e = w.nfl_book.careers.entry(id).or_default();
            e.add(&line);
            e.clone()
        };
        for r in w.nfl_book.records.iter_mut() {
            let v = stat_of(if r.career { &career } else { &line }, r.stat);
            if v > r.value {
                let kind = if r.career { "career" } else { "single-season" };
                w.nfl_book.broken.push((year, format!("{kind} {}", r.stat), name.clone(), v));
                if !(r.world && r.holder == name) {
                    posts.push(format!("{name} sets the NFL {kind} record for {}: {v:.0}.", r.stat));
                }
                r.value = v;
                r.holder = name.clone();
                r.year = year;
                r.world = true;
            }
        }
    }
    for text in posts {
        w.calendar.post(Event::new(d, Level::Nfl, Kind::News, Loudness::Big, text));
    }
}

/// The Super Bowl MVP: the winning side's best game by the same read.
pub(crate) fn super_bowl_mvp(
    w: &mut World,
    r: &crate::game::GameResult,
    side: &super::games::Side,
    s: usize,
    team: usize,
    d: Date,
) {
    let year = w.year;
    let best = r.stats.players[s]
        .iter()
        .filter_map(|(id, l)| {
            let (i, _) = side.who.get(id)?;
            let pos = w.people[*i].player.position;
            let v = stats_value(l, pos) * if pos == Position::QB { 1.0 } else { 1.3 } + 2.0 * l.grade_sum;
            Some((*i, *id, v))
        })
        .max_by(|a, b| a.2.total_cmp(&b.2));
    let Some((i, id, _)) = best else { return };
    let name = w.people[i].name.clone();
    w.nfl_book.awards.push((year, "Super Bowl MVP", id, name.clone(), team));
    let age = w.people[i].row_age();
    w.people[i].note(format!("Super Bowl MVP at {age}."));
    if let Some(c) = w.people[i].pro.as_mut() {
        c.season_mut(year).awards.push("Super Bowl MVP");
    }
    let text = format!("{name} is the Super Bowl MVP.");
    w.calendar.post(Event::new(d, Level::Nfl, Kind::News, Loudness::Big, text).team(team));
}

/// A retiring player's Hall of Fame case: his honors, titles and seasons
/// as a starter (the voters' real habits: All-Pro teams weigh most, then
/// MVPs, Pro Bowls and rings; long careers at a high level).
pub(crate) fn hall_case(w: &mut World, i: usize) {
    let p = &w.people[i];
    let Some(c) = p.pro.as_ref() else { return };
    let id = p.player.id;
    let (pb, ap1, ap2, mvp) = w.nfl_book.honors_of(id);
    let starts: u32 = c.seasons.iter().filter(|s| s.starts >= 8).count() as u32;
    let rings =
        c.seasons.iter().filter(|s| s.team.is_some_and(|t| w.nfl_champions.contains(&(s.year, t)))).count() as u32;
    let other = w.nfl_book.awards.iter().filter(|x| x.2 == id && x.1 != "MVP").count() as u32;
    let score = 4.0 * ap1 as f64
        + 1.5 * ap2 as f64
        + 1.0 * pb as f64
        + 5.0 * mvp as f64
        + 1.5 * other as f64
        + 0.7 * rings as f64
        + 0.4 * starts as f64;
    if score < 12.0 {
        return;
    }
    let why = format!(
        "{ap1} first-team All-Pro, {}, {mvp} MVP{}, {rings} Super Bowl title{}, {} as a starter",
        crate::world::news::count(pb, "Pro Bowl"),
        if mvp == 1 { "" } else { "s" },
        if rings == 1 { "" } else { "s" },
        crate::world::news::count(starts, "season")
    );
    w.nfl_book.candidates.push(HallCandidate {
        id,
        name: p.name.clone(),
        pos: p.player.position,
        retired: w.year,
        score,
        why,
    });
}

/// The Hall of Fame class (the Saturday before the Super Bowl): players
/// retired five full seasons or more, the best cases first, at most five a
/// year, and only a strong case gets in (eligible for 20 years).
pub(crate) fn hall_vote(w: &mut World, d: Date) {
    let year = w.year;
    let mut open: Vec<usize> = (0..w.nfl_book.candidates.len())
        .filter(|&k| {
            let c = &w.nfl_book.candidates[k];
            year - c.retired >= 6 && year - c.retired <= 25
        })
        .collect();
    open.sort_by(|&a, &b| w.nfl_book.candidates[b].score.total_cmp(&w.nfl_book.candidates[a].score));
    let mut class = Vec::new();
    for k in open.into_iter().take(5) {
        let c = &w.nfl_book.candidates[k];
        // The bar: a first-ballot case is clear; a weaker one waits.
        let waited = (year - c.retired - 6) as f64;
        if c.score + 0.6 * waited >= 20.0 {
            class.push(k);
        }
    }
    let mut names = Vec::new();
    class.sort_unstable_by(|a, b| b.cmp(a));
    for &k in &class {
        let c = w.nfl_book.candidates.remove(k);
        names.push(format!("{} ({})", c.name, c.pos));
        w.nfl_book.hall.push((year, c.id, c.name, c.pos, c.why));
    }
    if !names.is_empty() {
        let text = format!("Pro Football Hall of Fame class: {}.", names.join(", "));
        w.calendar.post(Event::new(d, Level::Nfl, Kind::News, Loudness::Big, text).memorable());
    }
    let _ = Status::Roster;
}
