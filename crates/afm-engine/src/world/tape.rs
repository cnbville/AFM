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

/// Tapes kept per opponent (its last few games).
const KEEP_PER_TEAM: usize = 6;

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
            _ => Vec::new(),
        };
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Whose film the day keeps at a level: your coming opponents, or None
    /// when your team is not at that level.
    pub(crate) fn tape_watch(&self, level: Level) -> Option<Vec<usize>> {
        let (my_level, _) = self.seat_team()?;
        (my_level == level).then(|| self.coming_opponents())
    }

    /// Keeps a game's film when a coming opponent of yours played in it
    /// (`opps` from `tape_watch`).
    pub(crate) fn keep_tape(
        &mut self,
        opps: &[usize],
        level: Level,
        teams: [usize; 2],
        stats: &GameStats,
        film: &[Film; 2],
    ) {
        let Some((my_level, me)) = self.seat_team() else { return };
        if level != my_level || teams.contains(&me) || !teams.iter().any(|t| opps.contains(t)) {
            return;
        }
        let names = match level {
            Level::Nfl => teams.map(|t| self.nfl_team_name(t)),
            Level::HighSchool => teams.map(|h| self.school_name(h).to_string()),
            _ => teams.map(|p| self.program_name(p).to_string()),
        };
        let tape = Tape::of(self.today, level, teams, names, stats, film);
        let tapes = &mut self.gameday.tapes;
        tapes.push(tape);
        // Only this season's tapes of teams still to come, the last few of
        // each (a tape stays while either side still needs it).
        let season_start = self.today.add(-200);
        tapes.retain(|t| t.date >= season_start && t.teams.iter().any(|x| opps.contains(x)));
        let mut keep = vec![false; tapes.len()];
        for &o in opps {
            let mut mine: Vec<usize> = (0..tapes.len()).filter(|&k| tapes[k].teams.contains(&o)).collect();
            mine.sort_by_key(|&k| std::cmp::Reverse(tapes[k].date));
            for &k in mine.iter().take(KEEP_PER_TEAM) {
                keep[k] = true;
            }
        }
        let mut k = 0;
        tapes.retain(|_| {
            k += 1;
            keep[k - 1]
        });
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

    /// An opponent's tapes this season, newest first.
    pub fn tapes_of(&self, team: usize) -> Vec<&Tape> {
        let mut v: Vec<&Tape> = self.gameday.tapes.iter().filter(|t| t.teams.contains(&team)).collect();
        v.sort_by_key(|t| std::cmp::Reverse(t.date));
        v
    }
}
