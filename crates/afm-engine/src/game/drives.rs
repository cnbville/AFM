//! A game's drives, read off the play log: who had the ball, where each
//! possession started and ended, its plays, yards, time and how it ended
//! (the drive chart, the box score's red zone and possession counts, and
//! the world reader's game log). Nothing here changes a game.

use super::stats::{GameStats, PlayRecord, PlayType};

/// A drive: who had it, where it started (yards from its own goal line),
/// the plays, yards, time and how it ended.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Drive {
    pub team: usize,
    pub q: u8,
    pub clock: f64,
    pub start: f64,
    pub end: f64,
    pub plays: u32,
    pub secs: f64,
    pub result: &'static str,
    pub points: i32,
    pub first: u32,
    pub last: u32,
    /// A snap at the opponent's 20 or closer: a red zone trip.
    pub red_zone: bool,
}

fn scrimmage(r: &PlayRecord) -> bool {
    matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble) && !r.nullified
}

/// Seconds left in regulation at a snap (overtime counts its own period).
fn left(quarter: u8, clock: f64, qlen: f64) -> f64 {
    if quarter > 4 { clock.min(300.0) } else { (4 - quarter.min(4)) as f64 * qlen + clock }
}

/// The quarter's length the plays show (at least 12 minutes).
fn quarter_length(s: &GameStats) -> f64 {
    s.plays.iter().map(|r| r.clock).fold(0.0, f64::max).max(720.0)
}

/// Every drive of a game in order.
pub fn drives(s: &GameStats) -> Vec<Drive> {
    let mut out: Vec<Drive> = Vec::new();
    let mut cur: Option<Drive> = None;
    let qlen = quarter_length(s);
    let fresh = |r: &PlayRecord, team: usize| Drive {
        team,
        q: r.quarter,
        clock: r.clock,
        start: 100.0 - r.yardline_100,
        end: 100.0 - r.yardline_100,
        plays: 0,
        secs: 0.0,
        result: "",
        points: 0,
        first: r.n,
        last: r.n,
        red_zone: false,
    };
    for r in s.plays.iter().filter(|r| r.offense < 2) {
        let snap = matches!(
            r.kind,
            PlayType::Run
                | PlayType::Pass
                | PlayType::Sack
                | PlayType::Scramble
                | PlayType::Punt
                | PlayType::FieldGoal
                | PlayType::Kneel
                | PlayType::Spike
                | PlayType::NoPlay
        );
        if !snap {
            continue;
        }
        if cur.as_ref().is_some_and(|d| d.team != r.offense || (r.quarter == 3 && d.q <= 2)) {
            let mut d = cur.take().unwrap();
            // A side that knelt and still gave the ball back (the clock
            // didn't run out) turned it over on downs.
            if d.result.is_empty() || (d.result == "End of game" && d.team != r.offense) {
                d.result = if d.q <= 2 && r.quarter >= 3 { "End of half" } else { "Downs" };
            }
            out.push(d);
        }
        let d = cur.get_or_insert_with(|| fresh(r, r.offense));
        if d.result == "End of game" {
            // A kneel with time still on the clock: the same drive goes on.
            d.result = "";
        } else if !d.result.is_empty() {
            // The drive already ended (a score or a kick) but the same team
            // has the ball again: a new drive.
            let team = d.team;
            out.extend(cur.take());
            cur = Some(fresh(r, team));
        }
        let d = cur.as_mut().unwrap();
        if r.kind != PlayType::NoPlay {
            d.plays += 1;
        }
        if r.yardline_100 <= 20.0 {
            d.red_zone = true;
        }
        d.last = r.n;
        d.end = 100.0 - r.yardline_100 + if scrimmage(r) && !r.turnover { r.yards } else { 0.0 };
        // The last play's seconds, up to the clock it had (a snap at 0:02
        // that runs eight seconds ends the quarter at 0:00).
        d.secs = (left(d.q, d.clock, qlen) - left(r.quarter, r.clock, qlen)
            + r.duration.max(0.0).min(r.clock.max(0.0)))
        .max(0.0);
        if r.kind == PlayType::Punt && r.touchdown && !r.turnover {
            // Returned: the punting side's drive gave up the points.
            d.result = "Returned for a TD";
            d.points = -6;
        } else if r.touchdown && !r.defense_td {
            d.result = "Touchdown";
            d.points = 6;
        } else if r.defense_td {
            d.result = if r.interception { "Pick six" } else { "Returned for a TD" };
            d.points = -6;
        } else if r.kind == PlayType::FieldGoal {
            d.result = if r.kick_good {
                "Field goal"
            } else if r.blocked {
                "Blocked FG"
            } else {
                "Missed FG"
            };
            d.points = if r.kick_good { 3 } else { 0 };
        } else if r.kind == PlayType::Punt {
            d.result = "Punt";
        } else if r.safety {
            d.result = "Safety";
            d.points = -2;
        } else if r.interception {
            d.result = "Interception";
        } else if r.fumble_lost {
            d.result = "Fumble";
        } else if r.turnover {
            d.result = "Downs";
        } else if r.kind == PlayType::Kneel && r.quarter >= 4 {
            d.result = "End of game";
        }
    }
    if let Some(mut d) = cur.take() {
        if d.result.is_empty() {
            d.result = "End of game";
        }
        out.push(d);
    }
    out
}

/// Game clock each side held the ball (home, away): the seconds from each
/// snap to the next go to the side that had the ball, the return after a
/// kickoff to the side that took it; a quarter's last snap keeps the rest
/// of the quarter. Ball-in-play seconds stay in `time_of_possession`.
pub fn clock_possession(s: &GameStats) -> [f64; 2] {
    let mut held = [0.0; 2];
    let plays: Vec<&PlayRecord> =
        s.plays.iter().filter(|r| r.offense < 2 && !matches!(r.kind, PlayType::ExtraPoint | PlayType::TwoPoint)).collect();
    for (i, r) in plays.iter().enumerate() {
        let next = plays.get(i + 1);
        let secs = match next {
            Some(n) if n.quarter == r.quarter => (r.clock - n.clock).max(0.0),
            Some(_) => r.clock.max(0.0),
            None if r.quarter <= 4 => r.clock.max(0.0),
            None => r.duration.max(0.0).min(r.clock.max(0.0)),
        };
        let side = match (r.kind, next) {
            (PlayType::Kickoff, Some(n)) => n.offense,
            (PlayType::Kickoff, None) => 1 - r.offense,
            _ => r.offense,
        };
        held[side] += secs;
    }
    held
}

impl GameStats {
    /// The team counts that need whole drives: red zone trips and their
    /// touchdowns, and the clock each side held the ball. Set, not added,
    /// so a live game counted at every pause ends right.
    pub fn count_drives(&mut self) {
        let mut trips = [0u32; 2];
        let mut tds = [0u32; 2];
        for d in drives(self) {
            if d.red_zone {
                trips[d.team] += 1;
                tds[d.team] += (d.result == "Touchdown") as u32;
            }
        }
        let held = clock_possession(self);
        for k in 0..2 {
            self.team[k].red_zone_trips = trips[k];
            self.team[k].red_zone_tds = tds[k];
            self.team[k].clock_possession = held[k];
        }
    }
}
