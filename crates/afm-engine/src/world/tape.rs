//! Game film of your coming opponents (section 9, topic 1): what a tape
//! shows of each snap (the look, the play, the coverage and the pressure,
//! the result) and who won the battles up front. Kept only for the teams
//! still on your schedule this season, so the store stays small; the
//! opponent report and the tape's hints are read from it.

use crate::calendar::{Date, Level};
use crate::game::GameStats;
use crate::game::calling::Film;
use crate::game::stats::PlayType;

use super::World;

/// One snap as the film shows it.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Snap {
    /// The offense: 0 home, 1 away.
    pub off: u8,
    pub q: u8,
    pub down: u8,
    pub togo: u8,
    /// Yards to the goal line.
    pub yl: u8,
    /// Seconds left in the quarter.
    pub clock: u16,
    /// The offense's lead.
    pub lead: i8,
    /// 0 run, 1 pass (a sack or scramble too), 2 punt, 3 field goal, 4
    /// kickoff, 5 other (a kneel, a spike), 6 two-point try, 7 extra point.
    pub kind: u8,
    /// Personnel, backfield and set ("11 gun trips").
    pub look: String,
    /// The play ("inside zone right", "mesh").
    pub play: String,
    pub coverage: String,
    pub pressure: String,
    pub blitz: bool,
    pub man: bool,
    pub play_action: bool,
    pub yards: i16,
    pub epa: f32,
    pub first: bool,
    pub td: bool,
    pub turnover: bool,
    /// A kick: its returner, whether it was brought out of the end zone or
    /// fair caught, and the return.
    pub returner: Option<u32>,
    pub ret_yards: i16,
    pub fair: bool,
    pub touchback: bool,
    /// Who had the ball: the passer, the man thrown to, the runner, and
    /// the defender in coverage on the throw.
    pub passer: Option<u32>,
    pub target: Option<u32>,
    pub carrier: Option<u32>,
    pub defender: Option<u32>,
    pub complete: bool,
    /// A kick through the uprights.
    pub good: bool,
    pub sack: bool,
    pub scramble: bool,
    pub pressured: bool,
    /// Wiped out by a penalty.
    pub null: bool,
    /// Where a throw went (lane 9's pass map); None on other snaps and on
    /// film kept before it.
    #[serde(default)]
    pub throw: Option<Throw>,
}

/// A throw as the film shows it, in small numbers.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Throw {
    /// Yards past the line in the air, and after the catch.
    pub air: i8,
    pub yac: i8,
    /// Across the field in half yards from the offense's right sideline
    /// (the offense's left high): where it was snapped and where it got to.
    pub from: u8,
    pub to: u8,
    /// Time to throw, tenths of a second, and the receiver's separation,
    /// tenths of a yard.
    pub ttt: u8,
    pub sep: u8,
    /// Thrown away, dropped.
    pub away: bool,
    pub drop: bool,
}

impl Throw {
    /// The throw of a play in the log (None when it wasn't one).
    pub fn of(r: &crate::game::PlayRecord) -> Option<Throw> {
        if r.kind != PlayType::Pass {
            return None;
        }
        let half = |y: f32| (y * 2.0).round().clamp(0.0, 107.0) as u8;
        let (from, to) = r.throw_y.map_or((53, 53), |(a, b)| (half(a), half(b)));
        Some(Throw {
            air: r.air_yards.round().clamp(-20.0, 99.0) as i8,
            yac: r.yac.round().clamp(-30.0, 110.0) as i8,
            from,
            to,
            ttt: (r.time_to_throw * 10.0).round().clamp(0.0, 255.0) as u8,
            sep: (r.separation * 10.0).round().clamp(0.0, 255.0) as u8,
            away: r.throw_away,
            drop: r.drop,
        })
    }
}

/// One game's film.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Tape {
    pub date: Date,
    pub level: Level,
    pub teams: [usize; 2],
    pub names: [String; 2],
    pub score: [i32; 2],
    pub snaps: Vec<Snap>,
    /// Pass protection by blocker: (side, id, reps, lost to speed, to
    /// power, to finesse, pressures allowed).
    pub protection: Vec<(u8, u32, u16, u16, u16, u16, u16)>,
    /// Pass rushers' quick wins: (side, id, speed, power, finesse).
    pub rush: Vec<(u8, u32, u16, u16, u16)>,
    /// What each side showed (home, away): studying the tape adds it to
    /// your staff's read of their habits.
    pub film: [Film; 2],
}

/// Look and play out of a call as the play log writes it: "11 gun trips
/// jet motion, inside zone right".
fn split_call(call: &str) -> (String, String) {
    let words: Vec<&str> = call.split_whitespace().collect();
    if words.len() < 3 {
        return (call.to_string(), String::new());
    }
    let (back, rest) = if words[1] == "under" { ("under center", 3) } else { (words[1], 2) };
    let set = words.get(rest).copied().unwrap_or("");
    let look = format!("{} {} {}", words[0], back, set);
    let play = words.get(rest + 1..).map(|w| w.join(" ")).unwrap_or_default();
    (look, play.replace(" (audible)", ""))
}

impl Tape {
    pub fn of(
        date: Date,
        level: Level,
        teams: [usize; 2],
        names: [String; 2],
        s: &GameStats,
        film: &[Film; 2],
    ) -> Tape {
        let mut snaps = Vec::with_capacity(s.plays.len());
        let mut pro: std::collections::HashMap<(u8, u32), [u16; 5]> = Default::default();
        let mut rush: std::collections::HashMap<(u8, u32), [u16; 3]> = Default::default();
        for r in &s.plays {
            let kind = match r.kind {
                PlayType::Run => 0,
                PlayType::Pass | PlayType::Sack | PlayType::Scramble => 1,
                PlayType::Punt => 2,
                PlayType::FieldGoal => 3,
                PlayType::Kickoff => 4,
                PlayType::TwoPoint => 6,
                PlayType::ExtraPoint => 7,
                _ => 5,
            };
            let (look, play) = if kind <= 1 { split_call(&r.call) } else { (String::new(), r.call.clone()) };
            let (coverage, pressure) = match r.defense_call.split_once(", ") {
                Some((c, p)) => (c.to_string(), p.split(", ").next().unwrap_or("").to_string()),
                None => (r.defense_call.clone(), String::new()),
            };
            let off = r.offense.min(1) as u8;
            snaps.push(Snap {
                off,
                q: r.quarter,
                down: r.down,
                togo: r.togo.round().clamp(0.0, 99.0) as u8,
                yl: r.yardline_100.round().clamp(0.0, 100.0) as u8,
                clock: r.clock.max(0.0) as u16,
                lead: (r.score[r.offense.min(1)] - r.score[1 - r.offense.min(1)]).clamp(-99, 99) as i8,
                kind,
                look,
                play,
                coverage,
                pressure,
                blitz: r.blitz,
                man: r.coverage_man,
                play_action: r.play_action,
                yards: r.yards.round() as i16,
                epa: r.epa as f32,
                first: r.first_down,
                td: r.touchdown,
                turnover: r.turnover,
                returner: r.returner,
                ret_yards: r.return_yards.round() as i16,
                fair: r.fair_catch,
                touchback: r.touchback,
                passer: r.passer,
                target: r.target,
                carrier: r.carrier,
                defender: r.defender,
                complete: r.complete,
                good: r.kick_good,
                sack: r.kind == PlayType::Sack,
                scramble: r.kind == PlayType::Scramble,
                pressured: r.pressured,
                null: r.nullified || r.kind == PlayType::NoPlay,
                throw: Throw::of(r),
            });
            if kind == 1 && !r.nullified {
                for &(blocker, defender, pass, won, _, pressure) in &r.blocks {
                    if !pass {
                        continue;
                    }
                    let e = pro.entry((off, blocker)).or_default();
                    e[0] += 1;
                    if !won {
                        let mv = r.rush_wins.iter().find(|x| x.0 == defender).map_or(0, |x| x.1 as usize);
                        e[1 + mv] += 1;
                    }
                    if pressure {
                        e[4] += 1;
                    }
                }
                for &(id, mv, _) in &r.rush_wins {
                    rush.entry((1 - off, id)).or_default()[mv as usize] += 1;
                }
            }
        }
        let mut protection: Vec<_> =
            pro.into_iter().map(|((s, id), v)| (s, id, v[0], v[1], v[2], v[3], v[4])).collect();
        protection.sort_by_key(|x| (x.0, x.1));
        let mut rush: Vec<_> = rush.into_iter().map(|((s, id), v)| (s, id, v[0], v[1], v[2])).collect();
        rush.sort_by_key(|x| (x.0, x.1));
        Tape { date, level, teams, names, score: s.score, snaps, protection, rush, film: film.clone() }
    }

    /// The side a team played on this tape.
    pub fn side_of(&self, team: usize) -> Option<u8> {
        self.teams.iter().position(|&t| t == team).map(|k| k as u8)
    }
}

/// A look's index (`Formation::look`) from its film name ("11 gun trips").
pub fn look_index(look: &str) -> Option<usize> {
    use crate::game::playbook::{SET_COUNT, Set};
    let w: Vec<&str> = look.split_whitespace().collect();
    let pers = ["11", "12", "21", "10", "22", "13"].iter().position(|&p| Some(&p) == w.first())?;
    let (back, rest) = match w.get(1).copied()? {
        "under" => (0, 3),
        "gun" => (1, 2),
        "pistol" => (2, 2),
        _ => return None,
    };
    let sets = [Set::Doubles, Set::Trips, Set::Bunch, Set::Stack, Set::Empty, Set::Wing];
    let set = sets.iter().position(|s| Some(s.name()) == w.get(rest).copied())?;
    Some((pers * 3 + back) * SET_COUNT + set)
}

impl Tape {
    /// The film of some of this tape's snaps, from the side of `team`
    /// (0 home, 1 away): its offense's looks and its defense's coverages
    /// and pressure. A hint you add to your plan is studied again this way.
    pub fn film_of(&self, team: u8, snaps: &[usize]) -> Film {
        use crate::game::playbook::Coverage;
        let mut f = Film::default();
        for &k in snaps {
            let Some(s) = self.snaps.get(k) else { continue };
            if s.null || s.kind > 1 {
                continue;
            }
            let pass = s.kind == 1;
            if s.off == team {
                f.off_snaps += 1.0;
                if let Some(l) = look_index(&s.look) {
                    f.look[l][pass as usize] += 1.0;
                }
                let d = (s.down.clamp(1, 4) - 1) as usize;
                f.by_down[d][1] += 1.0;
                f.by_down[d][0] += pass as u8 as f32;
            } else {
                f.def_snaps += 1.0;
                if let Some(c) = Coverage::ALL.iter().position(|c| c.name() == s.coverage) {
                    f.cov[c] += 1.0;
                }
                f.blitz += s.blitz as u8 as f32;
            }
        }
        f
    }
}

/// Tapes kept per coming opponent (its last few games).
const KEEP_PER_TEAM: usize = 6;
/// Tapes kept of each team you could open against next season.
const KEEP_NEXT_YEAR: usize = 2;
/// A tape older than this is from the season before last.
const TAPE_DAYS: i32 = 400;

/// The teams whose film the day keeps, and how many tapes of each.
pub(crate) struct Watch {
    level: Level,
    caps: Vec<u8>,
    /// Your own team: its games are on your own film, not here.
    me: Option<usize>,
}

impl Watch {
    fn new(level: Level, teams: usize) -> Watch {
        Watch { level, caps: vec![0; teams], me: None }
    }

    fn add(&mut self, team: usize, n: usize) {
        if let Some(c) = self.caps.get_mut(team) {
            *c = (*c).max(n as u8);
        }
    }

    /// Whether the day keeps film of a team's games.
    pub(crate) fn watches(&self, team: usize) -> bool {
        self.cap(team) > 0
    }

    fn cap(&self, team: usize) -> usize {
        self.caps.get(team).map_or(0, |&c| c as usize)
    }
}

/// Late in a season, when a staff starts on next year's opponents: the NFL
/// from 1 December, college from 1 November, high school from 15 October,
/// to the end of each one's postseason (another league: its last four
/// weeks, `tape_watch`).
fn late_season(level: Level, d: Date) -> bool {
    let (m, day) = (d.month(), d.day());
    match level {
        Level::Nfl => m == 12 || m <= 2,
        Level::College => m >= 11 || m == 1,
        // The other leagues run on their own calendars (`tape_watch`).
        Level::Other => false,
        _ => (m == 10 && day >= 15) || m >= 11,
    }
}

/// Keeps the newest tapes of each watched team up to its cap (a tape stays
/// while either side still needs it), from this season and the one before.
/// Only the watch's level is trimmed.
fn trim(tapes: &mut Vec<Tape>, w: &Watch, today: Date) {
    let oldest = today.add(-TAPE_DAYS);
    let mut order: Vec<usize> = (0..tapes.len()).collect();
    order.sort_by_key(|&k| std::cmp::Reverse(tapes[k].date));
    let mut seen = vec![0usize; w.caps.len()];
    let mut keep = vec![false; tapes.len()];
    for k in order {
        let t = &tapes[k];
        // Another level's film is not this watch's to drop (the world's
        // last season keeps the NFL's and college's side by side).
        if t.level != w.level {
            keep[k] = true;
            continue;
        }
        if t.date < oldest {
            continue;
        }
        for &team in &t.teams {
            if let Some(n) = seen.get_mut(team) {
                if *n < w.cap(team) {
                    keep[k] = true;
                }
                *n += 1;
            }
        }
    }
    let mut k = 0;
    tapes.retain(|_| {
        k += 1;
        keep[k - 1]
    });
}

impl World {
    /// The teams still to play your team this season, at your level: the
    /// rest of the schedule, and in a postseason every team still alive
    /// while you are.
    pub fn coming_opponents(&self) -> Vec<usize> {
        let Some((level, t)) = self.seat_team() else { return Vec::new() };
        let today = self.today;
        let mut out: Vec<usize> = match level {
            Level::College => self
                .college_fixtures(Some(t))
                .into_iter()
                .filter(|f| f.date >= today && f.score.is_none() && !f.cancelled)
                .map(|f| if f.home == t { f.away } else { f.home })
                .collect(),
            Level::Nfl => {
                let mut v: Vec<usize> = self
                    .nfl_fixtures(Some(t))
                    .into_iter()
                    .filter(|f| !f.played && f.when.is_none_or(|d| d >= today))
                    .map(|f| if f.home == t { f.away } else { f.home })
                    .collect();
                if let Some(s) = self.nfl_season.as_ref()
                    && s.booked
                    && s.alive.iter().any(|c| c.contains(&t))
                {
                    v.extend(s.alive.iter().flatten().copied().filter(|&x| x != t));
                }
                v
            }
            Level::HighSchool => {
                let mut v: Vec<usize> = self
                    .hs_fixtures(t)
                    .into_iter()
                    .filter(|f| f.date >= today)
                    .map(|f| if f.home == t { f.away } else { f.home })
                    .collect();
                v.extend(self.hs_playoff_field(t));
                v
            }
            Level::Other => super::pro::other::seat_opponents(self),
            _ => Vec::new(),
        };
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Whose film the day keeps at a level, and how many tapes of each:
    /// your coming opponents' last 6, and late in the season the last 2 of
    /// every team you could open against next year (a staff starts on next
    /// season's opener in the winter, from last season's film). In the last
    /// history season, with no career yet, every NFL and college team's
    /// last 2, so a new career has film of its first opponents. None when
    /// the day keeps no film at that level.
    pub(crate) fn tape_watch(&self, level: Level) -> Option<Watch> {
        if self.seat.is_none() {
            if !self.world_tapes || !matches!(level, Level::Nfl | Level::College) || !late_season(level, self.today) {
                return None;
            }
            let n = match level {
                Level::Nfl => self.nfl.len(),
                _ => self.programs.len(),
            };
            let mut w = Watch::new(level, n);
            (0..n).for_each(|t| w.add(t, KEEP_NEXT_YEAR));
            return Some(w);
        }
        let (my_level, me) = self.seat_team()?;
        if my_level != level {
            return None;
        }
        let mut w = Watch::new(level, self.team_count(level));
        // In another league, the last four weeks of your club's season.
        let late = match level {
            Level::Other => super::pro::other::seat_fixtures(self).len() <= 4,
            _ => late_season(level, self.today),
        };
        if late {
            for t in self.next_year_pool() {
                w.add(t, KEEP_NEXT_YEAR);
            }
        }
        for t in self.coming_opponents() {
            w.add(t, KEEP_PER_TEAM);
        }
        w.me = Some(me);
        Some(w)
    }

    /// How many teams a level has (team ids run below it).
    fn team_count(&self, level: Level) -> usize {
        match level {
            Level::Nfl => self.nfl.len(),
            Level::HighSchool => self.high_schools.len(),
            Level::Other => super::pro::other::FRANCHISES,
            _ => self.programs.len(),
        }
    }

    /// Every team your team could open against next season: at the NFL
    /// every other club; in college every program of your division; in
    /// high school your state's schools of your class and the ones next
    /// to it.
    fn next_year_pool(&self) -> Vec<usize> {
        let Some((level, me)) = self.seat_team() else { return Vec::new() };
        match level {
            Level::Nfl => (0..self.nfl.len()).filter(|&t| t != me).collect(),
            // Every club of your league (next year's list where it is out).
            Level::Other => {
                let k = super::pro::other::league_of(me);
                let y = self.today.year();
                let mut v = super::pro::other::teams_in(k, y + 1);
                if v.is_empty() {
                    v = super::pro::other::teams_in(k, y);
                }
                v.retain(|&t| t != me);
                v
            }
            Level::HighSchool => {
                let Some(s) = self.high_schools.get(me) else { return Vec::new() };
                self.high_schools
                    .iter()
                    .filter(|o| o.id != me && o.state == s.state && o.classification.abs_diff(s.classification) <= 1)
                    .map(|o| o.id)
                    .collect()
            }
            _ => {
                let Some(p) = self.programs.get(me) else { return Vec::new() };
                self.programs.iter().filter(|o| o.id != me && o.division == p.division).map(|o| o.id).collect()
            }
        }
    }

    /// Keeps a game's film when a team the day watches played in it
    /// (`watch` from `tape_watch`), then drops what nobody needs.
    pub(crate) fn keep_tape(
        &mut self,
        watch: &Watch,
        level: Level,
        teams: [usize; 2],
        stats: &GameStats,
        film: &[Film; 2],
    ) {
        if level != watch.level
            || watch.me.is_some_and(|me| teams.contains(&me))
            || !teams.iter().any(|&t| watch.cap(t) > 0)
        {
            return;
        }
        let names = match level {
            Level::Nfl => teams.map(|t| self.nfl_team_name(t)),
            Level::HighSchool => teams.map(|h| self.school_name(h).to_string()),
            Level::Other => teams.map(|f| super::pro::other::name(f, self.today.year()).to_string()),
            _ => teams.map(|p| self.program_name(p).to_string()),
        };
        let tape = Tape::of(self.today, level, teams, names, stats, film);
        self.gameday.tapes.push(tape);
        let today = self.today;
        trim(&mut self.gameday.tapes, watch, today);
    }

    /// The film room after a change of schedule or job: only your level's
    /// tapes, and once your schedule is out only your coming opponents'
    /// (last season's film of them included). Before the schedule is out
    /// the tapes stay as they are; without a team the room is empty.
    pub(crate) fn prune_tapes(&mut self) {
        let Some((level, me)) = self.seat_team() else {
            if self.seat.is_some() {
                self.gameday.tapes.clear();
            }
            return;
        };
        self.gameday.tapes.retain(|t| t.level == level);
        let coming = self.coming_opponents();
        if coming.is_empty() {
            return;
        }
        let mut w = Watch::new(level, self.team_count(level));
        for t in coming {
            w.add(t, KEEP_PER_TEAM);
        }
        w.me = Some(me);
        let today = self.today;
        trim(&mut self.gameday.tapes, &w, today);
    }

    /// Your staff's film study (1-20), as the world builds it for your
    /// games: the coordinators' eye and what the budget buys. Hidden; it
    /// sizes how often your staff's tags are wrong in the opponent report.
    pub fn seat_film_study(&self) -> f64 {
        let Some((level, t)) = self.seat_team() else { return 10.0 };
        let eye = |c: Option<usize>| c.and_then(|c| self.coaches.get(c)).map_or(9.0, |c| c.ability().scheme);
        let v = match level {
            Level::College => {
                let p = &self.programs[t];
                0.35 * (eye(p.oc.or(p.head_coach)) + eye(p.dc.or(p.head_coach))) + 7.0 * p.facilities
            }
            Level::Nfl => {
                let tm = &self.nfl[t];
                0.35 * (eye(tm.oc.or(tm.head_coach)) + eye(tm.dc.or(tm.head_coach))) + 6.3
            }
            _ => {
                let s = &self.high_schools[t];
                0.7 * eye(s.coach) + 7.0 * s.resources
            }
        };
        v.clamp(1.0, 20.0)
    }

    /// An opponent's tapes, newest first: this season's, and last
    /// season's while they are still among its last few games.
    pub fn tapes_of(&self, team: usize) -> Vec<&Tape> {
        let level = self.seat_team().map(|s| s.0);
        let mut v: Vec<&Tape> =
            self.gameday.tapes.iter().filter(|t| Some(t.level) == level && t.teams.contains(&team)).collect();
        v.sort_by_key(|t| std::cmp::Reverse(t.date));
        v
    }
}
