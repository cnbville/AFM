//! The game (topics 4-9): downs, the clock, scoring, penalties, overtime,
//! substitutions, fatigue, injuries, momentum and play calling, snap by
//! snap. Every game is reproducible from its seed (topic 9.2).

use super::calling::{self, FourthDown, Memory, Situation};
use super::geom::{MID, v2};
use super::kicking::{self, KickTeams};
use super::playbook::*;
use super::roster::*;
use super::rules::{FirstDownStop, Overtime, Rules};
use super::scrimmage::{self, DSlot, Snap, SnapResult};
use super::sim::{Conditions, Crew, Foul, FoulKind, Frame, Injury, InjuryKind};
use super::stats::*;
use super::tables;
use crate::attributes::Attr;
use crate::player::Level;
use crate::positions::Position;
use crate::rng::{Rng, clamp};

/// Everything needed to play one game.
#[derive(Clone, Debug)]
pub struct GameSetup<'a> {
    pub home: &'a GameTeam,
    pub away: &'a GameTeam,
    pub rules: Rules,
    pub cond: Conditions,
    pub seed: u64,
    /// No home crowd.
    pub neutral: bool,
    pub playoff: bool,
    /// Keep the full tracking of every play (the user's games).
    pub record: bool,
    /// Rating points a player from the other side of the ball must beat a
    /// starter by to play both ways (topic 5.1), per team.
    pub two_way_margin: [f64; 2],
}

impl<'a> GameSetup<'a> {
    pub fn new(home: &'a GameTeam, away: &'a GameTeam, level: Level, seed: u64) -> GameSetup<'a> {
        let margin = match level {
            Level::HS => 4.0,
            Level::NFL => 99.0,
            _ => 14.0,
        };
        GameSetup {
            home,
            away,
            rules: Rules::for_level(level),
            cond: Conditions::default(),
            seed,
            neutral: false,
            playoff: false,
            record: false,
            two_way_margin: [margin, margin],
        }
    }
}

/// A finished game: the stats database for it, the injuries, how each
/// freelancer's trust moved, and tracking for recorded plays.
#[derive(Clone, Debug, Default)]
pub struct GameResult {
    pub stats: GameStats,
    pub injuries: Vec<(usize, Injury)>,
    /// (team, player id, trust change)
    pub trust: Vec<(usize, u32, f64)>,
    pub frames: Vec<(u32, Vec<Frame>)>,
    pub crew: Option<Crew>,
}

struct Side<'a> {
    team: &'a GameTeam,
    depth: Depth,
    energy: Vec<f64>,
    edge: Vec<f64>,
    trust: Vec<f64>,
    out: Vec<bool>,
    hurt: Vec<Option<InjuryKind>>,
    timeouts: u8,
    mem: Memory,
}

impl<'a> Side<'a> {
    fn new(team: &'a GameTeam, margin: f64, timeouts: u8) -> Side<'a> {
        let n = team.players.len();
        Side {
            team,
            depth: depth_chart(team, margin),
            energy: vec![1.0; n],
            edge: vec![0.0; n],
            trust: team.players.iter().map(|p| p.trust).collect(),
            out: vec![false; n],
            hurt: vec![None; n],
            timeouts,
            mem: Memory::default(),
        }
    }

    /// Fatigue threshold for swapping a player out (topic 6): defensive
    /// linemen rotate, backs share, linemen and quarterbacks rarely come out.
    /// A strict coach rides his starters harder.
    fn threshold(&self, pos: Position) -> f64 {
        let base = match pos {
            Position::EDGE | Position::DT => crate::game::sim::knob(48, 0.84),
            Position::RB => crate::game::sim::knob(49, 0.8),
            Position::WR | Position::TE => crate::game::sim::knob(50, 0.74),
            Position::LB => crate::game::sim::knob(52, 0.68),
            Position::CB | Position::S => crate::game::sim::knob(51, 0.68),
            _ => 0.5,
        };
        base - 0.006 * (self.team.staff.strictness - 10.0)
    }

    /// The best `n` available at a spot, swapping out tired starters when a
    /// fresh backup isn't much worse.
    fn pick(&self, pos: Position, n: usize, used: &mut Vec<usize>) -> Vec<usize> {
        let list: Vec<usize> =
            self.depth.list(pos).iter().copied().filter(|&i| !self.out[i] && !used.contains(&i)).collect();
        let th = self.threshold(pos);
        if std::env::var("AFM_EN").is_ok() && !list.is_empty() {
            eprintln!("EN {:?} {:.3} {:.1}", pos, self.energy[list[0]], if list.len() > 1 { self.rating(list[0], pos) - self.rating(list[1], pos) } else { 99.0 });
        }
        let mut out = Vec::new();
        let mut skipped = Vec::new();
        for &i in &list {
            if out.len() == n {
                break;
            }
            if self.energy[i] < th {
                let next = list.iter().copied().find(|&j| {
                    j != i
                        && !out.contains(&j)
                        && !skipped.contains(&j)
                        && self.energy[j] >= th
                        && self.rating(j, pos) >= self.rating(i, pos) - 9.0
                });
                if next.is_some() {
                    skipped.push(i);
                    continue;
                }
            }
            out.push(i);
        }
        for &i in &skipped {
            if out.len() < n {
                out.push(i);
            }
        }
        // A thin roster: the best athlete left, from anywhere.
        while out.len() < n {
            let any = (0..self.team.players.len())
                .filter(|&i| !self.out[i] && !used.contains(&i) && !out.contains(&i))
                .max_by(|&a, &b| self.rating(a, pos).total_cmp(&self.rating(b, pos)));
            match any {
                Some(i) => out.push(i),
                None => break,
            }
        }
        used.extend(out.iter().copied());
        out
    }

    fn rating(&self, i: usize, pos: Position) -> f64 {
        self.team.players[i].overall_at(pos)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Next {
    Kickoff { team: usize, from: f64, onside: bool },
    Pat(usize),
    Scrimmage,
}

struct Game<'a> {
    s: &'a GameSetup<'a>,
    sides: [Side<'a>; 2],
    rng: Rng,
    crew: Crew,
    stats: GameStats,
    res: GameResult,
    quarter: u8,
    clock: f64,
    running: bool,
    poss: usize,
    x: f64,
    down: u8,
    togo: f64,
    ball_y: f64,
    next: Next,
    over: bool,
    receive_first: usize,
    n: u32,
    after_turnover: bool,
    two_min_warned: bool,
    // overtime
    ot: bool,
    ot_round: u32,
    ot_poss: [u32; 2],
    ot_first: usize,
    drive_start_poss: usize,
}

/// Play a game.
pub fn play_game(setup: &GameSetup) -> GameResult {
    let mut rng = Rng::new(((setup.seed >> 32) as u32) ^ (setup.seed as u32) ^ 0x9E37_79B9);
    let crew = Crew::draw(&mut rng);
    let t = setup.rules.timeouts;
    let mut g = Game {
        s: setup,
        sides: [Side::new(setup.home, setup.two_way_margin[0], t), Side::new(setup.away, setup.two_way_margin[1], t)],
        rng,
        crew,
        stats: GameStats::default(),
        res: GameResult::default(),
        quarter: 1,
        clock: setup.rules.quarter,
        running: false,
        poss: 0,
        x: 25.0,
        down: 1,
        togo: 10.0,
        ball_y: MID,
        next: Next::Scrimmage,
        over: false,
        receive_first: 0,
        n: 0,
        after_turnover: false,
        two_min_warned: false,
        ot: false,
        ot_round: 0,
        ot_poss: [0, 0],
        ot_first: 0,
        drive_start_poss: 0,
    };
    g.receive_first = if g.rng.chance(0.5) { 0 } else { 1 };
    let kick_from = setup.rules.kickoff_from;
    g.next = Next::Kickoff { team: 1 - g.receive_first, from: kick_from, onside: false };
    g.stats.quarters.push([0, 0]);
    let mut guard = 0;
    while !g.over && guard < 600 {
        guard += 1;
        match g.next {
            Next::Kickoff { team, from, onside } => g.kickoff(team, from, onside),
            Next::Pat(team) => g.pat(team),
            Next::Scrimmage => g.scrimmage(),
        }
        g.end_of_period();
    }
    g.stats.score = g.stats.team.each_ref().map(|t| t.points);
    g.res.crew = Some(g.crew);
    for side in 0..2 {
        for (i, p) in g.sides[side].team.players.iter().enumerate() {
            let d = g.sides[side].trust[i] - p.trust;
            if d.abs() > 1e-9 {
                g.res.trust.push((side, p.id, d));
            }
            if let Some(l) = g.stats.players[side].get_mut(&p.id) {
                l.games = 1;
            }
        }
    }
    g.res.stats = g.stats;
    g.res
}

impl<'a> Game<'a> {
    fn rules(&self) -> &Rules {
        &self.s.rules
    }

    fn half_left(&self) -> f64 {
        if self.ot {
            return self.clock;
        }
        self.clock + if self.quarter % 2 == 1 { self.rules().quarter } else { 0.0 }
    }

    fn game_left(&self) -> f64 {
        if self.ot {
            return self.clock;
        }
        self.clock + (4 - self.quarter.min(4)) as f64 * self.rules().quarter
    }

    fn diff(&self, team: usize) -> i32 {
        self.stats.team[team].points - self.stats.team[1 - team].points
    }

    fn situation(&self) -> Situation {
        let o = self.poss;
        Situation {
            down: self.down,
            togo: self.togo,
            yardline_100: 100.0 - self.x,
            score_diff: self.diff(o),
            quarter: self.quarter,
            half_left: self.half_left(),
            game_left: self.game_left(),
            timeouts: self.sides[o].timeouts,
            opp_timeouts: self.sides[1 - o].timeouts,
        }
    }

    fn wind_along(&self, team: usize) -> f64 {
        // Teams switch ends each quarter.
        let toward = if (self.quarter % 2 == 1) == (team == 0) { 1.0 } else { -1.0 };
        self.s.cond.wind_along * toward
    }

    fn home_crowd(&self, team: usize) -> f64 {
        if self.s.neutral || team == 0 { 0.0 } else { self.s.cond.crowd }
    }

    fn add_points(&mut self, team: usize, pts: i32) {
        self.stats.team[team].points += pts;
        let q = self.stats.quarters.len() - 1;
        self.stats.quarters[q][team] += pts;
    }

    fn new_record(&mut self, kind: PlayType) -> PlayRecord {
        self.n += 1;
        let mut r = PlayRecord::new(self.n, self.quarter, self.clock, self.poss, kind);
        r.down = self.down;
        r.togo = self.togo;
        r.yardline_100 = 100.0 - self.x;
        r.score = [self.stats.team[0].points, self.stats.team[1].points];
        r.ep_before = tables::expected_points(self.down.max(1), self.togo, 100.0 - self.x);
        r.wp_before = tables::win_probability(self.diff(self.poss), self.game_left(), 100.0 - self.x);
        r.after_turnover = self.after_turnover;
        r
    }

    /// Run the clock: a play's time, then the time to the next snap.
    fn tick(&mut self, secs: f64) {
        if self.ot && self.rules().overtime != Overtime::NflPeriod {
            return;
        }
        let before = self.clock;
        self.clock -= secs;
        // Two-minute warning (NFL).
        if self.rules().two_minute_warning
            && (self.quarter == 2 || self.quarter == 4)
            && before > 120.0
            && self.clock <= 120.0
            && !self.two_min_warned
        {
            self.two_min_warned = true;
            self.clock = 120.0;
            self.running = false;
            self.recover(0.02);
        }
        // Mercy rule: a running clock in a blowout (high school).
        if let Some((lead, q)) = self.rules().mercy {
            let d = (self.stats.team[0].points - self.stats.team[1].points).abs();
            if self.quarter >= q && d >= lead {
                self.clock -= secs * 0.3;
            }
        }
    }

    fn recover(&mut self, amt: f64) {
        for s in self.sides.iter_mut() {
            for e in s.energy.iter_mut() {
                *e = (*e + amt).min(1.0);
            }
        }
    }

    fn end_of_period(&mut self) {
        if self.over || self.clock > 0.0 {
            return;
        }
        if matches!(self.next, Next::Pat(_)) {
            return;
        }
        if self.ot {
            // NFL overtime period ends: a tie in the regular season.
            if self.rules().overtime == Overtime::NflPeriod {
                if self.diff(0) != 0 || !self.s.playoff {
                    self.over = true;
                } else {
                    self.clock = 600.0;
                }
            }
            return;
        }
        match self.quarter {
            2 => {
                self.quarter = 3;
                self.clock = self.rules().quarter;
                self.two_min_warned = false;
                for s in self.sides.iter_mut() {
                    s.timeouts = self.s.rules.timeouts;
                    for e in s.energy.iter_mut() {
                        *e = (*e + 0.35).min(1.0);
                    }
                }
                self.stats.quarters.push([0, 0]);
                let from = self.rules().kickoff_from;
                self.next = Next::Kickoff { team: self.receive_first, from, onside: false };
            }
            4 => {
                if self.diff(0) != 0 {
                    self.over = true;
                } else {
                    self.start_overtime();
                }
            }
            _ => {
                self.quarter += 1;
                self.clock = self.rules().quarter;
                self.recover(0.05);
                self.stats.quarters.push([0, 0]);
            }
        }
    }

    fn start_overtime(&mut self) {
        self.ot = true;
        self.stats.overtime = true;
        self.quarter = 5;
        self.stats.quarters.push([0, 0]);
        self.recover(0.15);
        let first = if self.rng.chance(0.5) { 0 } else { 1 };
        self.ot_first = first;
        self.ot_poss = [0, 0];
        match self.rules().overtime {
            Overtime::NflPeriod => {
                self.clock = 600.0;
                for s in self.sides.iter_mut() {
                    s.timeouts = 2;
                }
                let from = self.rules().kickoff_from;
                self.next = Next::Kickoff { team: 1 - first, from, onside: false };
            }
            _ => {
                self.clock = 1.0e9;
                self.ot_round = 1;
                self.ot_series(first);
            }
        }
    }

    /// A college or high school overtime series.
    fn ot_series(&mut self, team: usize) {
        self.poss = team;
        self.drive_start_poss = team;
        self.x = if self.rules().overtime == Overtime::HighSchool { 90.0 } else { 75.0 };
        self.down = 1;
        self.togo = 10.0;
        self.ball_y = MID;
        self.next = Next::Scrimmage;
        for s in self.sides.iter_mut() {
            s.timeouts = 1;
        }
    }

    /// A possession ended: in overtime decide what comes next.
    fn possession_over(&mut self, scored: bool) {
        if !self.ot {
            return;
        }
        let team = self.drive_start_poss;
        self.ot_poss[team] += 1;
        match self.rules().overtime {
            Overtime::NflPeriod => {
                // Both teams get the ball, then sudden death.
                if self.ot_poss[0] >= 1 && self.ot_poss[1] >= 1 && self.diff(0) != 0 {
                    self.over = true;
                }
                let _ = scored;
            }
            _ => {
                if self.ot_poss[0] == self.ot_poss[1] {
                    if self.diff(0) != 0 {
                        self.over = true;
                        return;
                    }
                    self.ot_round += 1;
                    let first = if self.ot_round % 2 == 1 { self.ot_first } else { 1 - self.ot_first };
                    self.ot_series(first);
                } else {
                    self.ot_series(1 - team);
                }
            }
        }
    }

    // ------------------------------------------------------------ kickoffs

    fn kick_unit(&self, team: usize, receiving: bool) -> Vec<usize> {
        let side = &self.sides[team];
        let ps = &side.team.players;
        let mut used = Vec::new();
        let mut on = Vec::new();
        if receiving {
            let rets = side.depth.kick_returners.iter().copied().filter(|&i| !side.out[i]).collect::<Vec<_>>();
            on.extend(rets.iter().take(1));
        } else {
            let k = side.depth.list(Position::K).first().copied().unwrap_or(0);
            on.push(k);
        }
        used.extend(on.iter().copied());
        // Coverage and return units: fast backups first.
        let mut pool: Vec<usize> =
            (0..ps.len()).filter(|&i| !side.out[i] && !used.contains(&i) && !ps[i].position.is_specialist()).collect();
        let starters: Vec<usize> =
            [Position::QB].iter().flat_map(|p| side.depth.list(*p).iter().take(1).copied()).collect();
        pool.retain(|i| !starters.contains(i));
        pool.sort_by(|&a, &b| {
            let f = |p: &GamePlayer| {
                0.4 * p.a(Attr::Speed) + 0.3 * p.a(Attr::Tackling) + 0.3 * p.a(Attr::RunBlocking) + side.energy[0] * 0.0
            };
            f(&ps[b]).total_cmp(&f(&ps[a]))
        });
        on.extend(pool.into_iter().take(11 - on.len()));
        on
    }

    fn kickoff(&mut self, team: usize, from: f64, onside: bool) {
        let rec = 1 - team;
        let kick_on = self.kick_unit(team, false);
        let ret_on = self.kick_unit(rec, true);
        // Onside kicks: trailing late, or a surprise from an aggressive coach.
        let late = self.quarter >= 4 && self.game_left() < 180.0;
        let behind = self.diff(team) < 0 && self.diff(team) >= -16;
        let onside = onside
            || (late && behind)
            || (!self.ot && self.rng.chance(0.002 * self.sides[team].team.staff.offense.aggressiveness / 10.0));
        let mut r = self.new_record(PlayType::Kickoff);
        r.offense = team;
        let kt = KickTeams {
            kick: &self.sides[team].team.players,
            ret: &self.sides[rec].team.players,
            kick_on: kick_on.clone(),
            ret_on: ret_on.clone(),
            snapper: self.sides[team].depth.snapper,
            holder: self.sides[team].depth.holder,
            energy: [&self.sides[team].energy, &self.sides[rec].energy],
        };
        let mut rules = self.s.rules;
        rules.kickoff_from = from;
        let along = self.wind_along(team);
        let kr =
            kicking::kickoff(&kt, &rules, &self.s.cond, &self.crew, along, onside, 1.0, self.s.record, &mut self.rng);
        r.kicker = Some(kt.kick[kick_on[0]].id);
        r.on_field = [kick_on.iter().map(|&i| kt.kick[i].id).collect(), ret_on.iter().map(|&i| kt.ret[i].id).collect()];
        r.kick_distance = kr.distance;
        r.touchback = kr.touchback;
        r.returner = kr.returner;
        r.return_yards = kr.return_yards;
        r.duration = kr.duration;
        r.tacklers = kr.notes.tacklers.clone();
        r.missed_tackles = kr.notes.missed.clone();
        r.injuries = kr.notes.injuries.clone();
        r.call = if onside { "onside kick".into() } else { "kickoff".into() };
        // On a return the returning team is side 0 and the kicking team side 1.
        self.apply_injuries(&kr.notes.injuries, [rec, team]);
        let elapsed = if kr.returned || onside { kr.duration } else { 0.0 };
        if self.clock > 0.0 && !(self.ot && self.rules().overtime != Overtime::NflPeriod) {
            self.tick(elapsed);
        }
        self.running = false;
        self.after_turnover = false;
        if kr.recovered_by_kicking {
            self.poss = team;
            self.x = if onside { 100.0 - kr.spot } else { kr.spot };
            r.turnover = true;
            r.why = if onside {
                "Onside kick recovered by the kicking team.".into()
            } else {
                "Muffed return recovered by the kicking team.".into()
            };
        } else if kr.return_td {
            self.poss = rec;
            r.touchdown = true;
            r.offense = team;
            r.why = format!("Kick returned for a touchdown by #{}.", kr.returner.unwrap_or(0));
            self.stats.book(&r);
            self.stats.plays.push(r);
            self.add_points(rec, 6);
            self.next = Next::Pat(rec);
            self.drive_start_poss = rec;
            return;
        } else {
            self.poss = rec;
            self.x = kr.spot.clamp(1.0, 99.0);
            r.why = if kr.touchback {
                "Touchback.".to_string()
            } else if onside {
                "Onside kick recovered by the receiving team.".into()
            } else {
                format!("Returned {:.0} yards by #{}.", kr.return_yards, kr.returner.unwrap_or(0))
            };
        }
        self.down = 1;
        self.togo = 10.0f64.min(100.0 - self.x);
        self.ball_y = MID;
        self.next = Next::Scrimmage;
        self.drive_start_poss = self.poss;
        self.stats.team[self.poss].drives += 1;
        self.stats.team[self.poss].start_sum += self.x;
        self.stats.book(&r);
        self.stats.plays.push(r);
    }

    // ------------------------------------------------------------ PAT

    fn pat(&mut self, team: usize) {
        self.poss = team;
        let staff = self.sides[team].team.staff;
        let k = self.sides[team].depth.list(Position::K).first().copied();
        let kicker = k.map(|i| &self.sides[team].team.players[i]);
        let xp_make = kicker.map_or(0.5, |k| {
            clamp(0.55 + 0.005 * k.a(Attr::KickAccuracy) - 0.002 * (self.rules().xp_from - 3.0), 0.2, 0.99)
        });
        let diff = self.diff(team);
        let forced_two = self.ot && self.rules().overtime == Overtime::College && self.ot_round >= 2;
        let two =
            forced_two || calling::go_for_two(&staff, diff, self.quarter, self.game_left(), xp_make, &mut self.rng);
        if two {
            self.x = 100.0 - self.rules().two_point_from;
            self.down = 1;
            self.togo = self.rules().two_point_from;
            let res = self.snap_play(true);
            let mut r = res;
            r.kind = PlayType::TwoPoint;
            let good = r.touchdown;
            if good {
                self.add_points(team, 2);
            }
            r.why = format!("Two-point try {}. {}", if good { "good" } else { "failed" }, r.why);
            self.stats.plays.push(r);
        } else {
            let mut r = self.new_record(PlayType::ExtraPoint);
            let dist = self.rules().xp_from + 17.0;
            let fg = self.kick(team, dist, 0.1, &mut r);
            if fg {
                self.add_points(team, 1);
            }
            self.stats.book(&r);
            self.stats.plays.push(r);
        }
        if self.ot {
            self.possession_over(true);
            if self.over || self.rules().overtime != Overtime::NflPeriod {
                return;
            }
        }
        let from = self.rules().kickoff_from;
        self.next = Next::Kickoff { team, from, onside: false };
    }

    /// A field goal or extra point try. Returns whether it was good.
    fn kick(&mut self, team: usize, dist: f64, pressure: f64, r: &mut PlayRecord) -> bool {
        let side = &self.sides[team];
        let ps = &side.team.players;
        let k = side
            .depth
            .list(Position::K)
            .iter()
            .copied()
            .find(|&i| !side.out[i])
            .unwrap_or(side.depth.list(Position::P).first().copied().unwrap_or(0));
        let snapper = &ps[side.depth.snapper];
        let holder = &ps[side.depth.holder];
        let mut used = vec![k];
        let protect_on: Vec<usize> = side
            .pick(Position::IOL, 3, &mut used)
            .into_iter()
            .chain(side.pick(Position::OT, 2, &mut used))
            .chain(side.pick(Position::TE, 2, &mut used))
            .collect();
        let opp = &self.sides[1 - team];
        let mut used2 = Vec::new();
        let rush_on: Vec<usize> = opp
            .pick(Position::DT, 3, &mut used2)
            .into_iter()
            .chain(opp.pick(Position::EDGE, 2, &mut used2))
            .chain(opp.pick(Position::LB, 2, &mut used2))
            .collect();
        let protect = kicking::kick_unit_strength(ps, &protect_on, false);
        let rush = kicking::kick_unit_strength(&opp.team.players, &rush_on, true);
        let along = self.wind_along(team);
        let fg = kicking::field_goal(
            &ps[k],
            snapper,
            holder,
            rush,
            protect,
            dist,
            pressure,
            &self.s.cond,
            along,
            &mut self.rng,
        );
        r.kicker = Some(ps[k].id);
        r.kick_distance = dist;
        r.kick_good = fg.made;
        r.blocked = fg.blocked;
        r.kick_pressure = pressure;
        r.why = fg.why.clone();
        r.on_field = [
            protect_on.iter().map(|&i| ps[i].id).chain([ps[k].id]).collect(),
            rush_on.iter().map(|&i| opp.team.players[i].id).collect(),
        ];
        r.duration = 3.0;
        fg.made
    }

    // ------------------------------------------------------------ scrimmage

    fn scrimmage(&mut self) {
        let o = self.poss;
        let sit = self.situation();
        let staff = self.sides[o].team.staff;
        let yl = 100.0 - self.x;
        // End of half: kick it if in range, else take a shot.
        let fg_dist = yl + 17.0;
        let kicker_range = self.kicker_range(o);
        let half_end = !self.ot && (self.quarter == 2 || self.quarter == 4) && self.half_left() <= 6.0;
        if half_end && fg_dist <= kicker_range && (self.quarter == 2 || self.diff(o) <= 0 && self.diff(o) >= -3) {
            self.field_goal(fg_dist);
            return;
        }
        // Kneel to run out the clock.
        if !self.ot && self.should_kneel() {
            let call = OffCall {
                formation: Formation {
                    personnel: Personnel::P12,
                    backfield: Backfield::UnderCenter,
                    trips: false,
                    strength: 1.0,
                },
                kind: PlayKind::Kneel,
                dir: 1.0,
                tempo: false,
            };
            self.run_scrimmage_call(call, None);
            return;
        }
        // Fourth down.
        if self.down == 4 && !(self.ot && self.rules().overtime != Overtime::NflPeriod && false) {
            let make = self.fg_make_estimate(o, fg_dist);
            let mut choice = calling::fourth_down(&staff, &sit, make, &mut self.rng);
            if self.ot && self.rules().overtime != Overtime::NflPeriod && choice == FourthDown::Punt {
                choice = if make > 0.35 { FourthDown::FieldGoal } else { FourthDown::Go };
            }
            if choice == FourthDown::FieldGoal && fg_dist > kicker_range + 3.0 {
                choice = if yl < 40.0 { FourthDown::Go } else { FourthDown::Punt };
            }
            match choice {
                FourthDown::Punt => {
                    self.punt();
                    return;
                }
                FourthDown::FieldGoal => {
                    self.field_goal(fg_dist);
                    return;
                }
                FourthDown::Go => {}
            }
        }
        // Spike to stop the clock.
        if self.running
            && sit.hurry()
            && self.half_left() < 30.0
            && self.sides[o].timeouts == 0
            && self.down < 4
            && self.half_left() > 3.0
        {
            let call = OffCall {
                formation: Formation {
                    personnel: Personnel::P11,
                    backfield: Backfield::Shotgun,
                    trips: false,
                    strength: 1.0,
                },
                kind: PlayKind::Spike,
                dir: 1.0,
                tempo: true,
            };
            self.run_scrimmage_call(call, None);
            return;
        }
        let r = self.snap_play(false);
        self.stats.plays.push(r);
    }

    fn kicker_range(&self, team: usize) -> f64 {
        let side = &self.sides[team];
        match side.depth.list(Position::K).first() {
            Some(&k) => {
                let p = &side.team.players[k];
                38.0 + 0.3 * p.a(Attr::KickPower) - 4.0
            }
            None => 30.0,
        }
    }

    fn fg_make_estimate(&self, team: usize, dist: f64) -> f64 {
        let side = &self.sides[team];
        let Some(&k) = side.depth.list(Position::K).first() else { return 0.2 };
        let p = &side.team.players[k];
        let acc = p.a(Attr::KickAccuracy);
        let range = 38.0 + 0.3 * p.a(Attr::KickPower);
        let sd = (2.95 + 0.012 * (dist - 30.0)) * (1.82f64 - 0.0105 * acc).max(0.55);
        let half = (3.083 / dist).atan().to_degrees();
        let z = half / sd;
        let p_line = 1.0 - 2.0 * (1.0 - phi(z));
        let p_range = phi((range - dist) / 3.0);
        p_line * p_range
    }

    fn should_kneel(&self) -> bool {
        let o = self.poss;
        let lead = self.diff(o);
        if lead <= 0 || !self.running && self.quarter < 4 {
            return false;
        }
        if self.quarter == 4 {
            let opp_to = self.sides[1 - o].timeouts as f64;
            let downs = (4 - self.down + 1) as f64;
            let need = self.game_left() + opp_to * 38.0;
            return need <= downs * 39.0 && self.game_left() < 160.0;
        }
        self.quarter == 2 && self.half_left() < 30.0 && self.x < 60.0 && self.running
    }

    fn field_goal(&mut self, dist: f64) {
        let o = self.poss;
        let mut r = self.new_record(PlayType::FieldGoal);
        let late = self.quarter >= 4 && self.game_left() < 150.0;
        let d = self.diff(o);
        let decides = late && (-3..=0).contains(&d) || self.ot;
        let mut pressure: f64 = if decides {
            0.8
        } else if late && d.abs() <= 8 {
            0.4
        } else {
            0.1
        };
        if self.s.playoff {
            pressure += 0.15;
        }
        // Icing: the defense calls time before a big kick.
        if decides && self.sides[1 - o].timeouts > 0 && self.rng.chance(0.5) {
            self.sides[1 - o].timeouts -= 1;
            pressure += 0.1;
        }
        let pressure: f64 = pressure.min(1.0);
        let good = self.kick(o, dist, pressure, &mut r);
        self.tick(r.duration);
        self.running = false;
        if good {
            self.add_points(o, 3);
            r.epa = 3.0 - r.ep_before;
            self.stats.book(&r);
            self.stats.plays.push(r);
            self.end_drive_score();
        } else {
            // The other team takes over at the spot of the kick (or the 20).
            let spot = (100.0 - (self.x - 7.0)).max(20.0);
            r.turnover = true;
            r.epa = -tables::expected_points(1, 10.0, 100.0 - spot) - r.ep_before;
            self.stats.book(&r);
            self.stats.plays.push(r);
            self.change_possession(spot, false);
        }
    }

    fn end_drive_score(&mut self) {
        let o = self.poss;
        if self.ot {
            self.possession_over(true);
            if self.over || self.rules().overtime != Overtime::NflPeriod {
                return;
            }
        }
        let from = self.rules().kickoff_from;
        self.next = Next::Kickoff { team: o, from, onside: false };
    }

    fn change_possession(&mut self, x_new: f64, turnover: bool) {
        let old = self.poss;
        self.poss = 1 - old;
        self.x = x_new.clamp(1.0, 99.0);
        self.down = 1;
        self.togo = 10.0f64.min(100.0 - self.x);
        self.running = false;
        self.after_turnover = turnover;
        self.next = Next::Scrimmage;
        if self.ot {
            self.drive_start_poss = old;
            self.possession_over(false);
            if self.rules().overtime != Overtime::NflPeriod {
                return;
            }
        }
        self.drive_start_poss = self.poss;
        self.stats.team[self.poss].drives += 1;
        self.stats.team[self.poss].start_sum += self.x;
    }

    fn punt(&mut self) {
        let o = self.poss;
        let d = 1 - o;
        let mut r = self.new_record(PlayType::Punt);
        let side = &self.sides[o];
        let p = side.depth.list(Position::P).iter().copied().find(|&i| !side.out[i]).unwrap_or(0);
        let mut kick_on = vec![p];
        let mut used = vec![p];
        kick_on.extend(side.pick(Position::CB, 2, &mut used));
        kick_on.extend(side.pick(Position::LB, 3, &mut used));
        kick_on.extend(side.pick(Position::TE, 2, &mut used));
        kick_on.extend(side.pick(Position::IOL, 3, &mut used));
        let opp = &self.sides[d];
        let pr = opp.depth.punt_returner;
        let mut ret_on = vec![pr];
        let mut used2 = vec![pr];
        ret_on.extend(opp.pick(Position::CB, 2, &mut used2));
        ret_on.extend(opp.pick(Position::LB, 3, &mut used2));
        ret_on.extend(opp.pick(Position::S, 1, &mut used2));
        ret_on.extend(opp.pick(Position::DT, 2, &mut used2));
        ret_on.extend(opp.pick(Position::EDGE, 2, &mut used2));
        let kt = KickTeams {
            kick: &side.team.players,
            ret: &opp.team.players,
            kick_on: kick_on.clone(),
            ret_on: ret_on.clone(),
            snapper: side.depth.snapper,
            holder: side.depth.holder,
            energy: [&side.energy, &opp.energy],
        };
        let along = self.wind_along(o);
        let kr = kicking::punt(
            &kt,
            self.x,
            &self.s.rules,
            &self.s.cond,
            &self.crew,
            along,
            1.0,
            self.s.record,
            &mut self.rng,
        );
        r.kicker = Some(kt.kick[p].id);
        r.on_field = [kick_on.iter().map(|&i| kt.kick[i].id).collect(), ret_on.iter().map(|&i| kt.ret[i].id).collect()];
        r.kick_distance = kr.distance;
        r.touchback = kr.touchback;
        r.fair_catch = kr.fair_catch;
        r.inside20 = kr.inside20;
        r.returner = kr.returner;
        r.return_yards = if kr.returned { kr.return_yards } else { 0.0 };
        r.blocked = kr.blocked;
        r.duration = kr.duration;
        r.tacklers = kr.notes.tacklers.clone();
        r.missed_tackles = kr.notes.missed.clone();
        r.injuries = kr.notes.injuries.clone();
        self.apply_injuries(&kr.notes.injuries, [d, o]);
        self.tick(kr.duration);
        self.running = false;
        r.why = if kr.blocked {
            "Punt blocked.".into()
        } else if kr.touchback {
            format!("Punt of {:.0} yards into the end zone for a touchback.", kr.distance)
        } else if kr.fair_catch {
            format!("Punt of {:.0} yards, fair caught.", kr.distance)
        } else if kr.returned {
            format!("Punt of {:.0} yards, returned {:.0}.", kr.distance, kr.return_yards)
        } else {
            format!("Punt of {:.0} yards, downed.", kr.distance)
        };
        if kr.return_td {
            r.touchdown = true;
            r.epa = -6.95 - r.ep_before;
            self.stats.book(&r);
            self.stats.plays.push(r);
            self.add_points(d, 6);
            self.poss = d;
            self.drive_start_poss = o;
            self.next = Next::Pat(d);
            return;
        }
        if kr.recovered_by_kicking {
            r.epa = tables::expected_points(1, 10.0, 100.0 - kr.spot) - r.ep_before;
            self.stats.book(&r);
            self.stats.plays.push(r);
            self.x = kr.spot.clamp(1.0, 99.0);
            self.down = 1;
            self.togo = 10.0f64.min(100.0 - self.x);
            self.next = Next::Scrimmage;
            return;
        }
        r.epa = -tables::expected_points(1, 10.0, 100.0 - kr.spot.clamp(1.0, 99.0)) - r.ep_before;
        self.stats.book(&r);
        self.stats.plays.push(r);
        self.ball_y = MID;
        self.change_possession(kr.spot, false);
    }

    /// Players on the field for a call.
    #[allow(clippy::type_complexity)]
    fn lineup(&self, call: &OffCall, dcall: &DefCall) -> (Vec<(ORole, usize)>, Vec<(DSlot, usize)>) {
        let o = &self.sides[self.poss];
        let d = &self.sides[1 - self.poss];
        let mut used = Vec::new();
        let mut off = Vec::new();
        let roles = call.formation.personnel.roles();
        let add = |list: Vec<usize>, rs: &[ORole], off: &mut Vec<(ORole, usize)>| {
            for (k, r) in rs.iter().enumerate() {
                if let Some(&i) = list.get(k) {
                    off.push((*r, i));
                }
            }
        };
        add(o.pick(Position::QB, 1, &mut used), &[ORole::QB], &mut off);
        add(o.pick(Position::OT, 2, &mut used), &[ORole::LT, ORole::RT], &mut off);
        add(o.pick(Position::IOL, 3, &mut used), &[ORole::LG, ORole::C, ORole::RG], &mut off);
        let backs: Vec<ORole> = roles.iter().copied().filter(|r| r.is_back()).collect();
        add(o.pick(Position::RB, backs.len(), &mut used), &backs, &mut off);
        let wrs: Vec<ORole> =
            roles.iter().copied().filter(|r| matches!(r, ORole::X | ORole::Z | ORole::Slot | ORole::Slot2)).collect();
        add(o.pick(Position::WR, wrs.len(), &mut used), &wrs, &mut off);
        let tes: Vec<ORole> =
            roles.iter().copied().filter(|r| matches!(r, ORole::TE | ORole::TE2 | ORole::TE3)).collect();
        add(o.pick(Position::TE, tes.len(), &mut used), &tes, &mut off);
        let three_four = d.team.staff.front == Front::ThreeFour;
        let slots: Vec<(DSlot, Position)> = match dcall.personnel {
            DefPersonnel::Base if three_four => vec![
                (DSlot::LE, Position::EDGE),
                (DSlot::RE, Position::EDGE),
                (DSlot::DTL, Position::DT),
                (DSlot::NT, Position::DT),
                (DSlot::DTR, Position::DT),
                (DSlot::Mike, Position::LB),
                (DSlot::Will, Position::LB),
                (DSlot::CBL, Position::CB),
                (DSlot::CBR, Position::CB),
                (DSlot::FS, Position::S),
                (DSlot::SS, Position::S),
            ],
            DefPersonnel::Base => vec![
                (DSlot::LE, Position::EDGE),
                (DSlot::RE, Position::EDGE),
                (DSlot::DTL, Position::DT),
                (DSlot::DTR, Position::DT),
                (DSlot::Sam, Position::LB),
                (DSlot::Mike, Position::LB),
                (DSlot::Will, Position::LB),
                (DSlot::CBL, Position::CB),
                (DSlot::CBR, Position::CB),
                (DSlot::FS, Position::S),
                (DSlot::SS, Position::S),
            ],
            DefPersonnel::Nickel => vec![
                (DSlot::LE, Position::EDGE),
                (DSlot::RE, Position::EDGE),
                (DSlot::DTL, Position::DT),
                (DSlot::DTR, Position::DT),
                (DSlot::Mike, Position::LB),
                (DSlot::Will, Position::LB),
                (DSlot::CBL, Position::CB),
                (DSlot::CBR, Position::CB),
                (DSlot::NCB, Position::CB),
                (DSlot::FS, Position::S),
                (DSlot::SS, Position::S),
            ],
            DefPersonnel::Dime => vec![
                (DSlot::LE, Position::EDGE),
                (DSlot::RE, Position::EDGE),
                (DSlot::DTL, Position::DT),
                (DSlot::DTR, Position::DT),
                (DSlot::Mike, Position::LB),
                (DSlot::CBL, Position::CB),
                (DSlot::CBR, Position::CB),
                (DSlot::NCB, Position::CB),
                (DSlot::DCB, Position::CB),
                (DSlot::FS, Position::S),
                (DSlot::SS, Position::S),
            ],
            DefPersonnel::GoalLine => vec![
                (DSlot::LE, Position::EDGE),
                (DSlot::RE, Position::EDGE),
                (DSlot::DTL, Position::DT),
                (DSlot::NT, Position::DT),
                (DSlot::DTR, Position::DT),
                (DSlot::Sam, Position::LB),
                (DSlot::Mike, Position::LB),
                (DSlot::Will, Position::LB),
                (DSlot::CBL, Position::CB),
                (DSlot::CBR, Position::CB),
                (DSlot::SS, Position::S),
            ],
        };
        let mut dused = Vec::new();
        let mut def = Vec::new();
        for pos in [Position::EDGE, Position::DT, Position::LB, Position::CB, Position::S] {
            let want: Vec<DSlot> = slots.iter().filter(|s| s.1 == pos).map(|s| s.0).collect();
            let got = d.pick(pos, want.len(), &mut dused);
            for (k, s) in want.iter().enumerate() {
                if let Some(&i) = got.get(k) {
                    def.push((*s, i));
                }
            }
        }
        (off, def)
    }

    /// Pre-snap fouls (topic 4): jumping early, delay, formation. Discipline,
    /// crowd noise, fatigue and knowing the playbook decide them.
    fn pre_snap_foul(&mut self, off: &[(ORole, usize)], def: &[(DSlot, usize)]) -> Option<Foul> {
        let o = self.poss;
        let crowd = self.home_crowd(o);
        let crew = self.crew.pre_snap;
        for &(r, i) in off {
            if !(r.is_line() || r.is_receiver()) {
                continue;
            }
            let p = &self.sides[o].team.players[i];
            let tired = 1.0 - self.sides[o].energy[i];
            let chance = 0.0016
                * (1.6 - p.traits.discipline / 15.0).max(0.3)
                * (1.0 + 1.3 * crowd)
                * crew
                * (1.0 + tired)
                * (1.25 - 0.25 * p.knowledge);
            if self.rng.chance(chance) {
                return Some(Foul { kind: FoulKind::FalseStart, side: 0, player: p.id, at: self.x });
            }
        }
        for &(s, i) in def {
            if !s.is_line() {
                continue;
            }
            let p = &self.sides[1 - o].team.players[i];
            let chance = 0.0019 * (1.6 - p.traits.discipline / 15.0).max(0.3) * crew;
            if self.rng.chance(chance) {
                return Some(Foul { kind: FoulKind::Offside, side: 1, player: p.id, at: self.x });
            }
        }
        let qb = off.iter().find(|x| x.0 == ORole::QB).map(|x| x.1).unwrap_or(0);
        let qp = &self.sides[o].team.players[qb];
        if self.rng.chance(0.0035 * (1.3 - qp.a(Attr::Awareness) / 120.0) * (1.0 + crowd) * crew) {
            return Some(Foul { kind: FoulKind::DelayOfGame, side: 0, player: qp.id, at: self.x });
        }
        if self.rng.chance(0.0035 * crew * (1.3 - qp.knowledge * 0.3)) {
            let id =
                off.iter().find(|x| x.0.is_receiver()).map(|x| self.sides[o].team.players[x.1].id).unwrap_or(qp.id);
            return Some(Foul { kind: FoulKind::IllegalFormation, side: 0, player: id, at: self.x });
        }
        None
    }

    fn snap_play(&mut self, two_point: bool) -> PlayRecord {
        let o = self.poss;
        let sit = self.situation();
        let staff_o = self.sides[o].team.staff;
        let staff_d = self.sides[1 - o].team.staff;
        let has_fb = self.sides[o].depth.list(Position::RB).len() >= 2;
        let te_count = self.sides[o].depth.list(Position::TE).iter().filter(|&&i| !self.sides[o].out[i]).count();
        let mut call = calling::call_offense(&staff_o, &self.sides[o].mem, &sit, has_fb, te_count, &mut self.rng);
        if two_point && matches!(call.kind, PlayKind::Pass(_)) && self.rng.chance(0.3) {
            call.kind = PlayKind::Pass(0);
        }
        let dcall = calling::call_defense(&staff_d, &self.sides[1 - o].mem, &sit, &call.formation, &mut self.rng);
        self.run_scrimmage_call(call, Some(dcall))
    }

    fn run_scrimmage_call(&mut self, mut call: OffCall, dcall: Option<DefCall>) -> PlayRecord {
        let o = self.poss;
        let d = 1 - o;
        let sit = self.situation();
        let dcall = dcall.unwrap_or(DefCall {
            personnel: DefPersonnel::Base,
            coverage: Coverage::Cover3,
            pressure: Pressure::Four,
            disguise: false,
        });
        let (off, def) = self.lineup(&call, &dcall);
        // Pre-snap check (topic 3): a smart quarterback changes a run into a
        // quick throw against a stacked box, or a pass into a run against a
        // light one, if his coach lets him.
        let qb = off.iter().find(|x| x.0 == ORole::QB).map(|x| x.1).unwrap_or(0);
        let qp = &self.sides[o].team.players[qb];
        let box_n = def
            .iter()
            .filter(|(s, _)| s.is_line() || s.is_lb() || *s == DSlot::SS && dcall.coverage.high() < 2)
            .count();
        let freedom = clamp(0.9 - 0.035 * self.sides[o].team.staff.strictness, 0.15, 0.85);
        let skill = clamp((qp.a(Attr::Awareness) - 50.0) / 50.0 + 0.1 * qp.experience.min(5.0) / 5.0, 0.0, 1.0);
        let mut audible = false;
        if !sit.hurry() && self.rng.chance(freedom * skill * 0.6) {
            match call.kind {
                PlayKind::Run(_) if box_n >= 8 && sit.togo > 2.0 => {
                    call.kind = PlayKind::Pass(0);
                    audible = true;
                }
                PlayKind::Pass(c) if box_n <= 6 && sit.togo <= 4.0 && PASS_CONCEPTS[c].depth != PassDepth::Screen => {
                    call.kind = PlayKind::Run(RunConcept::InsideZone);
                    audible = true;
                }
                _ => {}
            }
        }
        let mut r = self.new_record(match call.kind {
            PlayKind::Run(_) => PlayType::Run,
            PlayKind::Pass(_) => PlayType::Pass,
            PlayKind::Kneel => PlayType::Kneel,
            PlayKind::Spike => PlayType::Spike,
        });
        r.call = format!(
            "{} {} {}{}",
            call.formation.personnel.name(),
            match call.formation.backfield {
                Backfield::Shotgun => "gun",
                Backfield::Pistol => "pistol",
                Backfield::UnderCenter => "under center",
            },
            call.describe(),
            if audible { " (audible)" } else { "" }
        );
        r.defense_call = dcall.describe();
        r.personnel = call.formation.personnel.name().into();
        r.blitz = dcall.pressure.extra() > 0;
        r.coverage_man = dcall.coverage.is_man();
        // Pre-snap foul: no play.
        if !matches!(call.kind, PlayKind::Kneel)
            && let Some(f) = self.pre_snap_foul(&off, &def)
        {
            return self.no_play(r, f);
        }
        let labels: Vec<(u32, String)> = off
            .iter()
            .map(|&(rl, i)| (self.sides[o].team.players[i].id, scrimmage::orole_label(rl).to_string()))
            .chain(def.iter().map(|&(s, i)| (self.sides[d].team.players[i].id, s.label().to_string())))
            .collect();
        let label =
            |id: u32| labels.iter().find(|l| l.0 == id).map(|l| format!("{} #{}", l.1, id)).unwrap_or(format!("#{id}"));
        let mut hurt_edge_o: Vec<f64> = self.sides[o].edge.clone();
        for (i, h) in self.sides[o].hurt.iter().enumerate() {
            if h.is_some() {
                hurt_edge_o[i] -= 3.0;
            }
        }
        let mut hurt_edge_d: Vec<f64> = self.sides[d].edge.clone();
        for (i, h) in self.sides[d].hurt.iter().enumerate() {
            if h.is_some() {
                hurt_edge_d[i] -= 3.0;
            }
        }
        let crowd = self.home_crowd(o);
        let snap = Snap {
            teams: [&self.sides[o].team.players, &self.sides[d].team.players],
            offense: &off,
            defense: &def,
            energy: [&self.sides[o].energy, &self.sides[d].energy],
            edge: [&hurt_edge_o, &hurt_edge_d],
            trust: [&self.sides[o].trust, &self.sides[d].trust],
            call,
            dcall,
            los: self.x,
            ball_y: self.ball_y,
            down: self.down,
            togo: self.togo,
            rules: &self.s.rules,
            cond: Conditions { crowd, ..self.s.cond },
            crew: self.crew,
            strictness: [self.sides[o].team.staff.strictness, self.sides[d].team.staff.strictness],
            injury_scale: 1.0,
            record: self.s.record,
            hurry: sit.hurry(),
        };
        let res = scrimmage::run_snap(&snap, &mut self.rng);
        r.on_field = [
            off.iter().map(|&(_, i)| self.sides[o].team.players[i].id).collect(),
            def.iter().map(|&(_, i)| self.sides[d].team.players[i].id).collect(),
        ];
        r.box_count = res.box_count;
        r.play_action = res.play_action;
        // Fatigue (topic 6).
        for &(side, p, work) in &res.work {
            let team = if side == 0 { o } else { d };
            let st = self.sides[team].team.players[p].a(Attr::Stamina);
            self.sides[team].energy[p] = (self.sides[team].energy[p] - 0.0068 * crate::game::sim::knob(40, 1.0) * drain(st, work)).max(0.25);
        }
        for t in [o, d] {
            let on: Vec<usize> =
                if t == o { off.iter().map(|x| x.1).collect() } else { def.iter().map(|x| x.1).collect() };
            for i in 0..self.sides[t].energy.len() {
                let rec = if on.contains(&i) { crate::game::sim::knob(46, RECOVER_ON_FIELD) } else { crate::game::sim::knob(47, RECOVER_ON_BENCH) };
                self.sides[t].energy[i] = (self.sides[t].energy[i] + rec).min(1.0);
            }
        }
        self.apply_injuries(&res.notes.injuries, [o, d]);
        if let Some(f) = res.frames.clone() {
            self.res.frames.push((r.n, f));
        }
        self.resolve(r, res, &label, call, dcall)
    }

    fn no_play(&mut self, mut r: PlayRecord, f: Foul) -> PlayRecord {
        let o = self.poss;
        let (yds, _) = f.kind.yards();
        r.kind = PlayType::NoPlay;
        let team = if f.side == 0 { o } else { 1 - o };
        if f.side == 0 {
            let y = yds.min(self.x / 2.0);
            self.x -= y;
            self.togo += y;
            r.penalties.push((f.kind, team, f.player, y, true));
            r.yards = -y;
        } else {
            let y = yds.min((100.0 - self.x) / 2.0);
            self.x += y;
            self.togo -= y;
            r.penalties.push((f.kind, team, f.player, y, true));
            r.yards = y;
            if self.togo <= 0.0 {
                self.down = 1;
                self.togo = 10.0f64.min(100.0 - self.x);
                r.first_down = true;
            }
        }
        r.why = format!("{} on #{} before the snap.", f.kind.name(), f.player);
        r.epa = tables::expected_points(self.down, self.togo, 100.0 - self.x) - r.ep_before;
        self.running = false;
        self.tick(0.0);
        self.edge_hit(team, f.player, -1.0);
        self.stats.book(&r);
        r
    }

    fn apply_injuries(&mut self, inj: &[Injury], teams: [usize; 2]) {
        for i in inj {
            let team = teams[i.side];
            // Concussion protocol per level (topic 6): spotters in the NFL,
            // trainers in college, and some high schools have nobody.
            let mut i2 = i.clone();
            if i.kind == InjuryKind::Concussion {
                let caught = match self.rules().level {
                    Level::NFL => 0.92,
                    Level::HS => 0.6,
                    _ => 0.8,
                };
                i2.plays_on = !self.rng.chance(caught);
            }
            if i2.plays_on {
                self.sides[team].hurt[i.p] = Some(i.kind);
            } else {
                self.sides[team].out[i.p] = true;
            }
            self.res.injuries.push((team, i2));
        }
    }

    fn edge_hit(&mut self, team: usize, id: u32, amount: f64) {
        // Momentum (topic 4.1): small, and composure decides how much it bites.
        if let Some(i) = self.sides[team].team.players.iter().position(|p| p.id == id) {
            let c = self.sides[team].team.players[i].composure();
            let delta = if amount < 0.0 { amount * 1.5 * (1.25 - c / 20.0) } else { amount * 0.8 };
            let e = &mut self.sides[team].edge[i];
            *e = clamp(*e + delta, -3.0, 2.0);
        }
    }

    fn decay_edges(&mut self) {
        for s in self.sides.iter_mut() {
            for (i, e) in s.edge.iter_mut().enumerate() {
                let c = s.team.players[i].composure();
                let k = if *e < 0.0 { (0.8 - 0.03 * c).max(0.2) } else { 0.7 };
                *e *= k;
            }
        }
    }

    /// Enforce what happened: fouls, spot, downs, score, clock.
    fn resolve(
        &mut self,
        mut r: PlayRecord,
        res: SnapResult,
        label: &dyn Fn(u32) -> String,
        call: OffCall,
        dcall: DefCall,
    ) -> PlayRecord {
        use scrimmage::Result as R;
        let o = self.poss;
        let d = 1 - o;
        let los = self.x;
        r.duration = res.duration;
        r.passer = res.passer;
        r.target = res.target;
        r.carrier = res.carrier;
        r.air_yards = res.air_yards;
        r.yac = res.yac;
        r.before_contact = res.before_contact;
        r.time_to_throw = res.time_to_throw;
        r.separation = res.separation;
        r.throw_away = res.throw_away;
        r.drop = res.drop;
        r.pressured = res.pressured;
        r.out_of_bounds = res.out_of_bounds;
        r.pbu = res.pbu;
        r.tacklers = res.notes.tacklers.clone();
        r.missed_tackles = res.notes.missed.clone();
        r.sackers = res.notes.sackers.clone();
        r.hits = res.notes.hits.clone();
        r.pressures = res.notes.pressures.iter().map(|p| p.0).collect();
        r.forced_fumble = res.notes.forced_fumble;
        r.fumbled = res.notes.fumbled;
        r.blocks = res.notes.blocks.clone();
        r.rush_wins = res.notes.rush_wins.clone();
        r.busts = res.notes.busts.clone();
        r.injuries = res.notes.injuries.clone();
        r.top_speed = res
            .notes
            .top_speed
            .iter()
            .copied()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, v)| (id, v * 2.045))
            .unwrap_or((0, 0.0));
        r.kind = match res.result {
            R::Run => PlayType::Run,
            R::Complete | R::Incomplete | R::Interception => PlayType::Pass,
            R::Sack => PlayType::Sack,
            R::Scramble => PlayType::Scramble,
            R::Kneel => PlayType::Kneel,
            R::Spike => PlayType::Spike,
        };
        r.complete = res.result == R::Complete;
        r.interception = res.result == R::Interception;
        r.scramble = res.result == R::Scramble;
        if r.interception {
            r.interceptor = res.returner;
        }
        // The nearest defender to the target is the man in coverage.
        if r.kind == PlayType::Pass && !r.throw_away {
            r.defender = res.pbu.or(res.defender).or(self.nearest_cover(&r, &res));
        }
        // Freelancing results: a jump that made the play, or one that got burned.
        for &(id, _) in &res.notes.freelance {
            let won = r.interceptor == Some(id) || r.pbu == Some(id);
            r.freelance.push((id, won));
            if let Some(i) = self.sides[d].team.players.iter().position(|p| p.id == id) {
                let burned = r.complete && res.spot - los >= 15.0;
                let t = &mut self.sides[d].trust[i];
                if won {
                    *t = (*t + 0.05).min(1.0);
                } else if burned {
                    *t = (*t - 0.08).max(0.0);
                }
            }
        }
        let gain = res.spot - los;
        let mut new_x = res.spot;
        let mut fouls: Vec<Foul> = res.notes.fouls.clone();
        // One foul per team; both teams: offsetting.
        fouls.dedup_by_key(|f| f.side);
        let off_foul = fouls.iter().find(|f| f.side == 0).cloned();
        let def_foul = fouls.iter().find(|f| f.side == 1).cloned();
        let mut replay = false;
        let mut auto_first = false;
        let mut lose_down = false;
        let mut scored_td = res.touchdown;
        let mut turnover = res.turnover;
        let mut safety = res.safety;
        let ep = |down: u8, togo: f64, x: f64| tables::expected_points(down, togo, 100.0 - x.clamp(1.0, 99.0));
        if let (Some(of), Some(df)) = (off_foul.as_ref(), def_foul.as_ref()) {
            replay = true;
            new_x = los;
            scored_td = false;
            turnover = false;
            safety = false;
            for f in [of, df] {
                let team = if f.side == 0 { o } else { d };
                r.penalties.push((f.kind, team, f.player, 0.0, true));
            }
        } else if let Some(f) = def_foul {
            let (yds, first) = f.kind.yards();
            let pen_x = match f.kind {
                FoulKind::PassInterference => {
                    let spot_dist = (f.at - los).max(1.0);
                    let lim = if self.rules().level == Level::NFL { spot_dist } else { spot_dist.min(15.0) };
                    (los + lim).min(99.0)
                }
                FoulKind::RoughingPasser | FoulKind::FaceMask | FoulKind::UnnecessaryRoughness
                    if !turnover && gain > 0.0 =>
                {
                    res.spot + yds
                }
                _ => los + yds,
            };
            let pen_x = if pen_x >= 100.0 { los + (100.0 - los) / 2.0 } else { pen_x };
            // The offense takes the better of the play and the penalty.
            let play_val = if scored_td {
                7.0
            } else if turnover {
                -ep(1, 10.0, 100.0 - new_x)
            } else {
                self.value_after(gain, new_x)
            };
            let pen_val = ep(1, 10.0f64.min(100.0 - pen_x), pen_x);
            let accept = pen_val > play_val;
            r.penalties.push((f.kind, d, f.player, if accept { pen_x - los } else { 0.0 }, accept));
            if accept {
                new_x = pen_x;
                scored_td = false;
                turnover = false;
                safety = false;
                auto_first = first || pen_x - los >= self.togo;
                replay = !auto_first;
                self.edge_hit(d, f.player, -1.0);
            }
        } else if let Some(f) = off_foul {
            let (yds, _) = f.kind.yards();
            let from = if f.kind == FoulKind::IntentionalGrounding { f.at.min(los) } else { los };
            let pen_x = if f.kind == FoulKind::IntentionalGrounding {
                (los - yds).min(from)
            } else {
                from - yds.min(from / 2.0)
            };
            let play_val = if scored_td {
                7.0
            } else if turnover {
                -ep(1, 10.0, 100.0 - new_x)
            } else {
                self.value_after(gain, new_x)
            };
            let pen_val =
                ep(self.down + (f.kind == FoulKind::IntentionalGrounding) as u8, self.togo + (los - pen_x), pen_x);
            // The defense takes whichever is worse for the offense.
            let accept = pen_val < play_val;
            r.penalties.push((f.kind, o, f.player, if accept { los - pen_x } else { 0.0 }, accept));
            if accept {
                new_x = pen_x.max(1.0);
                scored_td = false;
                turnover = false;
                safety = false;
                replay = f.kind != FoulKind::IntentionalGrounding;
                lose_down = f.kind == FoulKind::IntentionalGrounding;
                self.edge_hit(o, f.player, -1.0);
            }
        }
        let accepted_pen = r.penalties.iter().any(|p| p.4);
        r.yards = if accepted_pen { 0.0 } else { gain };
        if !accepted_pen {
            match r.kind {
                PlayType::Pass if !r.complete => r.yards = 0.0,
                PlayType::Pass if r.interception => r.yards = 0.0,
                _ => {}
            }
        }
        if r.interception && !accepted_pen {
            r.yards = 0.0;
        }
        // Momentum and coordinator memory.
        self.decay_edges();
        if !accepted_pen {
            if r.interception {
                if let Some(p) = r.passer {
                    self.edge_hit(o, p, -1.0);
                }
                if let Some(i) = r.interceptor {
                    self.edge_hit(d, i, 1.0);
                }
            }
            if r.drop
                && let Some(t) = r.target
            {
                self.edge_hit(o, t, -1.0);
            }
            for &s in &r.sackers.clone() {
                self.edge_hit(d, s, 1.0);
            }
            for &(tk, _) in &r.missed_tackles.clone() {
                self.edge_hit(d, tk, -0.6);
            }
            if let Some(f) = r.fumbled {
                self.edge_hit(o, f, -1.0);
            }
            if scored_td && let Some(c) = r.carrier {
                self.edge_hit(o, c, 1.0);
            }
        }
        // Clock: the play, then whether it keeps running.
        let clock_running_at_snap = self.running;
        let _ = clock_running_at_snap;
        let dur = res.duration;
        self.tick(dur);
        let mut stop =
            matches!(res.result, R::Incomplete | R::Spike) || accepted_pen || turnover || scored_td || safety;
        if res.out_of_bounds && !turnover {
            let late =
                (self.quarter == 2 && self.half_left() <= 120.0) || (self.quarter >= 4 && self.half_left() <= 300.0);
            if self.rules().level != Level::NFL || late {
                stop = true;
            }
        }
        // Down and distance.
        let mut first_down = false;
        let fumble_lost = turnover && !r.interception;
        r.fumble_lost = fumble_lost;
        if !turnover && !scored_td && !safety && (auto_first || (!replay && new_x - los >= self.togo)) {
            first_down = true;
        }
        r.first_down = first_down && !accepted_pen || (accepted_pen && auto_first);
        r.touchdown = scored_td;
        r.turnover = turnover;
        r.safety = safety;
        // Spot near the hash.
        let h = self.rules().hash;
        self.ball_y = clamp(res.spot_y, MID - h, MID + h);
        if accepted_pen {
            self.ball_y = MID;
        }
        // Why (topic 1): the chain that decided it.
        r.why = self.why(&r, &res, label, &call, &dcall, accepted_pen);
        // Coordinator memory.
        if matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble) && !accepted_pen {
            let epa_est = self.estimate_epa(&r, new_x, first_down, turnover, scored_td);
            let m = &mut self.sides[o].mem;
            m.saw_defense(&dcall);
            if r.kind == PlayType::Run {
                m.runs += 1.0;
                m.run_epa += epa_est;
            } else {
                m.passes += 1.0;
                m.pass_epa += epa_est;
                if r.air_yards >= 15.0 {
                    m.deeps += 1.0;
                    m.deep_epa += epa_est;
                }
            }
            let md = &mut self.sides[d].mem;
            md.opp_plays += 1.0;
            if r.kind != PlayType::Run {
                md.opp_pass += 1.0;
                if r.air_yards >= 15.0 {
                    md.opp_deep_epa += epa_est;
                }
                if r.time_to_throw < 2.4 {
                    md.opp_quick += 1.0;
                }
            } else {
                md.opp_runs += 1.0;
                md.opp_run_epa += epa_est;
            }
        }
        // Score and possession.
        if scored_td {
            r.epa = 6.95 - r.ep_before;
            if turnover {
                // Defensive score.
                r.epa = -6.95 - r.ep_before;
                self.add_points(d, 6);
                self.stats.book(&r);
                self.poss = d;
                self.drive_start_poss = o;
                self.next = Next::Pat(d);
                self.running = false;
                return r;
            }
            self.add_points(o, 6);
            self.stats.book(&r);
            if self.ot && self.rules().overtime == Overtime::NflPeriod {
                // Both teams still get a possession.
            }
            self.next = Next::Pat(o);
            self.running = false;
            return r;
        }
        if res.defense_td {
            r.epa = -6.95 - r.ep_before;
            self.add_points(d, 6);
            self.stats.book(&r);
            self.poss = d;
            self.next = Next::Pat(d);
            self.running = false;
            return r;
        }
        if safety {
            r.epa = -2.0 - r.ep_before;
            self.add_points(d, 2);
            self.stats.book(&r);
            self.running = false;
            if self.ot {
                self.drive_start_poss = o;
                self.possession_over(true);
                if self.over || self.rules().overtime != Overtime::NflPeriod {
                    return r;
                }
            }
            self.next = Next::Kickoff { team: o, from: 20.0, onside: false };
            return r;
        }
        if turnover {
            // The other team's ball where it ended (in its own frame).
            let their_x = 100.0 - new_x;
            let their_x = if their_x <= 0.0 { 20.0 } else { their_x };
            r.epa = -ep(1, 10.0, their_x) - r.ep_before;
            self.stats.book(&r);
            self.change_possession(their_x, true);
            return r;
        }
        if replay {
            self.x = new_x.clamp(1.0, 99.0);
            self.togo = (self.togo + (los - self.x)).max(0.5);
            if self.togo > 100.0 - self.x {
                self.togo = 100.0 - self.x;
            }
            r.epa = ep(self.down, self.togo, self.x) - r.ep_before;
            self.stats.book(&r);
            self.running = !stop && self.rules().level == Level::NFL && false;
            return r;
        }
        self.x = new_x.clamp(0.5, 99.5);
        if first_down {
            self.down = 1;
            self.togo = 10.0f64.min(100.0 - self.x);
            match self.rules().first_down_stop {
                FirstDownStop::Always => stop = true,
                FirstDownStop::LastTwoMinutes if self.half_left() <= 120.0 => stop = true,
                _ => {}
            }
        } else {
            self.down = self.down + 1 + lose_down as u8;
            self.togo -= new_x - los;
            if self.down > 4 {
                // Turnover on downs.
                r.epa = -ep(1, 10.0, 100.0 - self.x) - r.ep_before;
                self.stats.book(&r);
                self.ball_y = MID;
                self.change_possession(100.0 - self.x, false);
                return r;
            }
        }
        r.epa = ep(self.down, self.togo, self.x) - r.ep_before;
        self.stats.book(&r);
        self.running = !stop;
        self.after_snap_clock(&r);
        r
    }

    /// Time from the end of one play to the next snap, and timeouts.
    fn after_snap_clock(&mut self, _r: &PlayRecord) {
        let o = self.poss;
        let sit = self.situation();
        // Defense trailing late: stop the clock with a timeout.
        if self.running && !self.ot && (self.quarter == 4 || self.quarter == 2) {
            let d = 1 - o;
            let trailing =
                self.diff(d) < 0 || (self.quarter == 2 && self.half_left() < 90.0 && self.diff(d) <= 0 && false);
            let gm = self.sides[d].team.staff.game_management;
            if trailing
                && self.quarter == 4
                && self.half_left() < 180.0
                && self.sides[d].timeouts > 0
                && self.rng.chance(clamp(0.4 + 0.03 * gm, 0.4, 0.98))
            {
                self.sides[d].timeouts -= 1;
                self.running = false;
                self.recover(0.02);
                return;
            }
        }
        if !self.running {
            // Clock restarts on the snap (or on the ready signal for some stops).
            return;
        }
        let gap = if sit.hurry() {
            // Offense trailing late uses a timeout after an in-bounds play.
            if self.sides[o].timeouts > 0 && self.half_left() < 75.0 && self.rng.chance(0.75) {
                self.sides[o].timeouts -= 1;
                self.running = false;
                return;
            }
            13.0
        } else if (self.quarter == 4 && self.diff(o) > 0)
            || (self.quarter == 2 && self.half_left() < 120.0 && self.x < 50.0 && self.diff(o) >= 0)
        {
            38.0
        } else {
            let tempo = match self.sides[o].team.staff.off_scheme {
                OffScheme::AirRaid | OffScheme::SpreadOption => 33.0,
                _ => 37.5,
            };
            tempo + self.rng.range(-3.0, 3.0)
        };
        self.tick(gap);
    }

    fn value_after(&self, gain: f64, new_x: f64) -> f64 {
        let ep = |down: u8, togo: f64, x: f64| tables::expected_points(down, togo, 100.0 - x.clamp(1.0, 99.0));
        if gain >= self.togo {
            ep(1, 10.0f64.min(100.0 - new_x), new_x)
        } else if self.down >= 4 {
            -ep(1, 10.0, 100.0 - new_x)
        } else {
            ep(self.down + 1, self.togo - gain, new_x)
        }
    }

    fn estimate_epa(&self, r: &PlayRecord, new_x: f64, first: bool, turnover: bool, td: bool) -> f64 {
        let ep = |down: u8, togo: f64, x: f64| tables::expected_points(down, togo, 100.0 - x.clamp(1.0, 99.0));
        let after = if td {
            6.95
        } else if turnover {
            -ep(1, 10.0, 100.0 - new_x)
        } else if first {
            ep(1, 10.0, new_x)
        } else if self.down >= 4 {
            -ep(1, 10.0, 100.0 - new_x)
        } else {
            ep(self.down + 1, (self.togo - (new_x - self.x)).max(1.0), new_x)
        };
        after - r.ep_before
    }

    fn nearest_cover(&self, r: &PlayRecord, _res: &SnapResult) -> Option<u32> {
        // From the coverage battles: the defender charged is the one who
        // contested the catch; otherwise none is charged.
        let _ = r;
        None
    }

    #[allow(clippy::too_many_arguments)]
    fn why(
        &self,
        r: &PlayRecord,
        res: &SnapResult,
        label: &dyn Fn(u32) -> String,
        call: &OffCall,
        dcall: &DefCall,
        pen: bool,
    ) -> String {
        use scrimmage::Result as R;
        let mut w;
        let yd = |y: f64| format!("{:.0}", y);
        match res.result {
            R::Sack => {
                let s = r.sackers.first().copied();
                if let Some(s) = s {
                    let beat = res.notes.blocks.iter().find(|b| b.1 == s && !b.3);
                    let mv = res.notes.rush_wins.iter().find(|x| x.0 == s);
                    w = match (beat, mv) {
                        (Some(b), Some(m)) => format!(
                            "Sack by {}: he beat {} with a {} in {:.1}s, and the quarterback held it {:.1}s.",
                            label(s),
                            label(b.0),
                            m.1.name(),
                            m.2,
                            res.time_to_throw
                        ),
                        (Some(b), None) => format!(
                            "Sack by {}: he got off {}'s block after {:.1}s while nobody came open.",
                            label(s),
                            label(b.0),
                            b.4
                        ),
                        _ => format!(
                            "Sack by {}: he came free ({}), and the quarterback held it {:.1}s.",
                            label(s),
                            dcall.pressure.name(),
                            res.time_to_throw
                        ),
                    };
                } else {
                    w = format!("Sacked after {:.1}s.", res.time_to_throw);
                }
            }
            R::Interception => {
                let i = res.returner.map(label).unwrap_or_default();
                w = format!(
                    "Intercepted by {} on {} against {}: thrown into {:.1} yd of separation{}.",
                    i,
                    call.describe(),
                    dcall.coverage.name(),
                    res.separation,
                    if res.pressured { " under pressure" } else { "" }
                );
                if r.freelance.iter().any(|f| f.1) {
                    w.push_str(" He jumped the route.");
                }
            }
            R::Complete => {
                let t = res.target.map(label).unwrap_or_default();
                w = format!(
                    "Complete to {} for {} on {} against {} ({:.1} yd of separation, {} after the catch).",
                    t,
                    yd(r.yards),
                    call.describe(),
                    dcall.coverage.name(),
                    res.separation,
                    yd(res.yac)
                );
                if let Some(&(tk, _)) = res.notes.missed.first() {
                    w.push_str(&format!(" {} missed a tackle.", label(tk)));
                }
            }
            R::Incomplete => {
                let t = res.target.map(label).unwrap_or_default();
                w = if res.throw_away {
                    "Thrown away: nobody open and the pocket closing.".to_string()
                } else if res.drop {
                    format!("Dropped by {}.", t)
                } else if let Some(p) = res.pbu {
                    format!("Broken up by {} on a throw to {}.", label(p), t)
                } else {
                    format!("Incomplete to {}: off target.", t)
                };
            }
            R::Run | R::Scramble => {
                let c = res.carrier.map(label).unwrap_or_default();
                let what = if res.result == R::Scramble { "Scramble".to_string() } else { call.describe() };
                w = format!("{} by {} for {}", what, c, yd(r.yards));
                if r.yards <= 0.0 {
                    if let Some(t) = r.tacklers.first() {
                        let shed = res.notes.blocks.iter().find(|b| b.1 == *t && !b.3);
                        match shed {
                            Some(b) => w.push_str(&format!(": {} shed {} and made the stop.", label(*t), label(b.0))),
                            None => w.push_str(&format!(": {} came free and made the stop.", label(*t))),
                        }
                    } else {
                        w.push('.');
                    }
                } else {
                    w.push_str(&format!(" ({} before contact", yd(res.before_contact.max(0.0))));
                    let broken = res.notes.missed.len();
                    if broken > 0 {
                        w.push_str(&format!(", {} broken tackle{}", broken, if broken > 1 { "s" } else { "" }));
                    }
                    w.push_str(").");
                }
            }
            R::Kneel => w = "Kneel.".into(),
            R::Spike => w = "Spiked to stop the clock.".into(),
        }
        if let Some(f) = res.notes.fumbled {
            w.push_str(&format!(
                " Fumble by {}{}.",
                label(f),
                match res.notes.recovered {
                    Some((s, id)) =>
                        format!(", recovered by {} ({})", label(id), if s == 0 { "offense" } else { "defense" }),
                    None => String::new(),
                }
            ));
        }
        if !res.notes.busts.is_empty() {
            w.push_str(&format!(" Busted assignment: {}.", label(res.notes.busts[0])));
        }
        if pen {
            for p in &r.penalties {
                if p.4 {
                    w.push_str(&format!(" Penalty: {} on {}, {:.0} yards.", p.0.name(), label(p.2), p.3));
                }
            }
        }
        w
    }
}

/// Standard normal CDF.
fn phi(x: f64) -> f64 {
    crate::athlete::phi(x)
}

/// Play a game on a fixed neutral field, mostly for tools and tests.
pub fn quick_game(home: &GameTeam, away: &GameTeam, level: Level, seed: u64) -> GameResult {
    let mut s = GameSetup::new(home, away, level, seed);
    s.cond = Conditions { crowd: 0.5, ..Conditions::default() };
    play_game(&s)
}

#[allow(dead_code)]
fn unused() -> crate::game::geom::V2 {
    v2(0.0, 0.0)
}

/// Debug tooling: run one snap with a given call from a spot, printing the
/// tracked play as it goes.
pub fn trace_snap(
    off: &GameTeam,
    def: &GameTeam,
    level: Level,
    call: OffCall,
    dcall: DefCall,
    los: f64,
    seed: u64,
) -> PlayRecord {
    trace_snap_opt(off, def, level, call, dcall, los, seed, true)
}

/// Run one snap with or without the agent trace.
#[allow(clippy::too_many_arguments)]
pub fn trace_snap_opt(
    off: &GameTeam,
    def: &GameTeam,
    level: Level,
    call: OffCall,
    dcall: DefCall,
    los: f64,
    seed: u64,
    trace: bool,
) -> PlayRecord {
    let setup = GameSetup::new(off, def, level, seed);
    let mut g = Game {
        s: &setup,
        sides: [Side::new(off, 99.0, 3), Side::new(def, 99.0, 3)],
        rng: Rng::new(seed as u32),
        crew: Crew { holding: 1.0, pass_interference: 1.0, pre_snap: 0.0, personal: 1.0 },
        stats: GameStats::default(),
        res: GameResult::default(),
        quarter: 1,
        clock: 900.0,
        running: false,
        poss: 0,
        x: los,
        down: 1,
        togo: 10.0,
        ball_y: MID,
        next: Next::Scrimmage,
        over: false,
        receive_first: 0,
        n: 0,
        after_turnover: false,
        two_min_warned: false,
        ot: false,
        ot_round: 0,
        ot_poss: [0, 0],
        ot_first: 0,
        drive_start_poss: 0,
    };
    g.stats.quarters.push([0, 0]);
    super::sim::TRACE.store(trace, std::sync::atomic::Ordering::Relaxed);
    let r = g.run_scrimmage_call(call, Some(dcall));
    super::sim::TRACE.store(false, std::sync::atomic::Ordering::Relaxed);
    r
}
