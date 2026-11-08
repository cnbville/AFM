//! The teams tally (`teams/<season>.tally`): each college program's and
//! NFL club's season of situation and tendency counts (items 16, 17, 25),
//! both sides of the ball: keys `o|...` with the ball, `d|...` defending.
//!
//! Every key holds the same eight numbers (`Situ`): plays, yards, EPA,
//! successes, first downs, touchdowns, turnovers and one extra
//! (completions on passes, sacks on dropbacks that were sacks, makes on
//! kicks, touchbacks on kickoffs and punts, missed tackles on runs).
//!
//! Keys (each with the play's kind last: run, pass, punt, fg, ...):
//! `kind`, `down|d`, `dd|d|to go`, `field|zone`, `quarter|q`, `two minute`,
//! `score|ahead, close or behind`, `personnel|p`, `backfield|b`,
//! `motion|m`, `rpo`, `tempo|huddle or no huddle`, `box|n`, `package|base,
//! nickel, dime or goal line`, `blitz|d`,
//! `third|to go`, `fourth|decision`, `red zone`, `goal to go`,
//! `explosive`; dropbacks only: `pressured|bool`, `play action|bool`,
//! `coverage|man or zone`, `depth|band`, `throw away`; and
//! `defense|the defense's call`, `penalty|kind|accepted` (the side that
//! committed it). The level's own numbers are the sum of its teams'.
//!
//! Lane NB8 (situations, concepts against coverages): `garbage|kind` for
//! snaps in garbage time (`decide::garbage`); on scrimmage snaps
//! `cc|concept|coverage` (the play's concept against the coverage the
//! defense played, `decide::concept_of`, `coverage_of`) and
//! `cov3|coverage` on 3rd downs.

use std::collections::{BTreeMap, HashMap};

use crate::game::stats::{GameStats, PlayRecord, PlayType};

use super::Level;

/// Plays, yards, EPA, successes, first downs, touchdowns, turnovers, extra.
pub type Situ = [f32; 8];

pub fn values(p: &PlayRecord) -> Situ {
    let extra = match p.kind {
        PlayType::Pass => p.complete as u32 as f32,
        PlayType::Sack => 1.0,
        PlayType::FieldGoal | PlayType::ExtraPoint => p.kick_good as u32 as f32,
        PlayType::Kickoff | PlayType::Punt => p.touchback as u32 as f32,
        PlayType::Run => p.missed_tackles.len() as f32,
        _ => 0.0,
    };
    let yards = match p.kind {
        PlayType::Punt | PlayType::Kickoff | PlayType::FieldGoal | PlayType::ExtraPoint => p.kick_distance,
        _ => p.yards,
    };
    [
        1.0,
        yards as f32,
        p.epa as f32,
        p.success() as u32 as f32,
        p.first_down as u32 as f32,
        p.touchdown as u32 as f32,
        p.turnover as u32 as f32,
        extra,
    ]
}

pub fn kind_name(k: PlayType) -> &'static str {
    match k {
        PlayType::Run => "run",
        PlayType::Pass | PlayType::Sack | PlayType::Scramble => "pass",
        PlayType::Kneel => "kneel",
        PlayType::Spike => "spike",
        PlayType::Punt => "punt",
        PlayType::FieldGoal => "fg",
        PlayType::ExtraPoint => "xp",
        PlayType::TwoPoint => "two point",
        PlayType::Kickoff => "kickoff",
        PlayType::NoPlay => "no play",
    }
}

fn togo(t: f64) -> &'static str {
    match t {
        t if t <= 1.0 => "1",
        t if t <= 3.0 => "2-3",
        t if t <= 6.0 => "4-6",
        t if t <= 10.0 => "7-10",
        _ => "11+",
    }
}

pub fn field(y: f64) -> &'static str {
    match y {
        y if y <= 5.0 => "goal (1-5)",
        y if y <= 20.0 => "red zone (6-20)",
        y if y <= 40.0 => "opp 21-40",
        y if y <= 60.0 => "midfield",
        y if y <= 80.0 => "own 21-40",
        y if y <= 95.0 => "own 6-20",
        _ => "backed up (own 1-5)",
    }
}

fn depth(air: f64) -> &'static str {
    match air {
        a if a < 0.0 => "behind",
        a if a < 10.0 => "short (0-9)",
        a if a < 20.0 => "intermediate (10-19)",
        _ => "deep (20+)",
    }
}

/// The call's look, from the call's words: backfield, motion, run-pass option.
fn look(call: &str) -> (&'static str, &'static str, bool) {
    let backfield = if call.contains(" gun ") {
        "gun"
    } else if call.contains(" pistol ") {
        "pistol"
    } else if call.contains(" under center ") {
        "under center"
    } else {
        "other"
    };
    let motion = ["jet motion, ", "orbit motion, ", "motion across, ", "shift, "]
        .into_iter()
        .find(|m| call.contains(m))
        .map_or("none", |m| m.trim_end_matches(", "));
    (backfield, motion, call.contains(" with "))
}

/// The keys one snap counts under, for both sides (`level`: the game's,
/// for garbage time).
pub fn keys(p: &PlayRecord, level: crate::player::Level) -> Vec<String> {
    let kind = kind_name(p.kind);
    let mut k = vec![format!("kind|{kind}")];
    let scrimmage = matches!(p.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble);
    let dropback = kind == "pass";
    let downs =
        (1..=4).contains(&p.down) && !matches!(p.kind, PlayType::Kickoff | PlayType::ExtraPoint | PlayType::TwoPoint);
    if downs {
        k.push(format!("down|{}|{kind}", p.down));
        k.push(format!("dd|{}|{}|{kind}", p.down, togo(p.togo)));
        k.push(format!("field|{}|{kind}", field(p.yardline_100)));
        k.push(format!("quarter|{}|{kind}", if p.quarter >= 5 { "OT".to_string() } else { p.quarter.to_string() }));
        if matches!(p.quarter, 2 | 4) && p.clock <= 120.0 {
            k.push(format!("two minute|{kind}"));
        }
        let o = p.offense.min(1);
        let m = p.score[o] - p.score[1 - o];
        let s = if m >= 9 {
            "ahead"
        } else if m <= -9 {
            "behind"
        } else {
            "close"
        };
        k.push(format!("score|{s}|{kind}"));
        if p.yardline_100 <= 20.0 {
            k.push(format!("red zone|{kind}"));
        }
        if p.togo >= p.yardline_100 - 0.01 && p.yardline_100 <= 10.0 {
            k.push(format!("goal to go|{kind}"));
        }
        if p.down == 3 {
            k.push(format!("third|{}|{kind}", togo(p.togo)));
        }
        if p.down == 4 && (scrimmage || matches!(p.kind, PlayType::Punt | PlayType::FieldGoal)) {
            k.push(format!("fourth|{}", if scrimmage { "go" } else { kind }));
        }
        if super::decide::garbage(level, p) {
            k.push(format!("garbage|{kind}"));
        }
    }
    if scrimmage {
        if !p.personnel.is_empty() {
            k.push(format!("personnel|{}|{kind}", p.personnel));
        }
        let (backfield, motion, rpo) = look(&p.call);
        k.push(format!("backfield|{backfield}|{kind}"));
        k.push(format!("motion|{motion}|{kind}"));
        if rpo {
            k.push(format!("rpo|{kind}"));
        }
        k.push(format!("tempo|{}|{kind}", if p.hurry { "no huddle" } else { "huddle" }));
        k.push(format!("box|{}|{kind}", p.box_count));
        let package = ["", "base", "nickel", "dime", "goal line"].get(p.dpers as usize).copied().unwrap_or("");
        if !package.is_empty() {
            k.push(format!("package|{package}|{kind}"));
        }
        if !p.defense_call.is_empty() {
            k.push(format!("defense|{}|{kind}", p.defense_call));
        }
        if (p.kind == PlayType::Run && p.yards >= 10.0) || p.yards >= 20.0 {
            k.push(format!("explosive|{kind}"));
        }
        if let (Some(c), Some(v)) = (super::decide::concept_of(p), super::decide::coverage_of(p)) {
            k.push(format!("cc|{c}|{v}"));
            if p.down == 3 {
                k.push(format!("cov3|{v}"));
            }
        }
    }
    if dropback {
        k.push(format!("blitz|{}|{}", p.down, p.blitz));
        k.push(format!("pressured|{}", p.pressured));
        k.push(format!("play action|{}", p.play_action));
        k.push(format!("coverage|{}", if p.coverage_man { "man" } else { "zone" }));
        if p.kind == PlayType::Pass {
            if p.throw_away {
                k.push("throw away".into());
            } else {
                k.push(format!("depth|{}", depth(p.air_yards)));
            }
        }
    }
    k
}

/// One side's game.
#[derive(Clone, Debug, Default)]
pub struct Game {
    pub cells: HashMap<String, Situ>,
    /// Penalties it committed: (kind, accepted) -> (count, yards).
    pub penalties: HashMap<String, (u32, f32)>,
}

fn bump(m: &mut HashMap<String, Situ>, k: String, v: &Situ) {
    let e = m.entry(k).or_insert([0.0; 8]);
    for i in 0..8 {
        e[i] += v[i];
    }
}

/// A side's game: its snaps with the ball (`o|`) and without (`d|`).
pub fn game(g: &GameStats, side: usize, level: crate::player::Level) -> Game {
    let mut out = Game::default();
    for p in &g.plays {
        for &(kind, team, _, yds, accepted) in &p.penalties {
            if team == side {
                let e = out.penalties.entry(format!("{kind:?}|{accepted}")).or_default();
                e.0 += 1;
                e.1 += yds as f32;
            }
        }
        if p.nullified || p.kind == PlayType::NoPlay {
            continue;
        }
        let tag = if p.offense.min(1) == side { "o" } else { "d" };
        let v = values(p);
        for k in keys(p, level) {
            bump(&mut out.cells, format!("{tag}|{k}"), &v);
        }
    }
    out
}

/// A team's season.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Season {
    /// 0 college, 1 NFL.
    pub level: u8,
    pub team: u32,
    pub games: u32,
    pub wins: u32,
    pub losses: u32,
    pub points_for: u32,
    pub points_against: u32,
    pub cells: BTreeMap<String, Situ>,
    pub penalties: BTreeMap<String, (u32, f32)>,
    /// The season's sum of the games' advanced numbers, each side.
    pub adv: super::games::Adv,
    pub adv_against: super::games::Adv,
    /// The open season only: the team's latest game on its own, so a page
    /// can leave it out while your staff is still charting it (Data item
    /// 2's delay; lane NB8). Never written to the season's file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<Box<Last>>,
}

/// A team's latest game, as `Season` adds it.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Last {
    pub date: i32,
    pub score: [i32; 2],
    pub cells: BTreeMap<String, Situ>,
    pub adv: super::games::Adv,
    pub adv_against: super::games::Adv,
}

impl Season {
    pub fn get(&self, key: &str) -> Situ {
        self.cells.get(key).copied().unwrap_or_default()
    }

    /// The season without its latest game when that game was played on or
    /// after `since` (your staff hasn't charted it yet).
    pub fn charted_by(&self, since: i32) -> Season {
        let mut s = self.clone();
        let Some(l) = s.last.take() else { return s };
        if l.date < since {
            return s;
        }
        s.games = s.games.saturating_sub(1);
        s.wins = s.wins.saturating_sub((l.score[0] > l.score[1]) as u32);
        s.losses = s.losses.saturating_sub((l.score[0] < l.score[1]) as u32);
        s.points_for = s.points_for.saturating_sub(l.score[0].max(0) as u32);
        s.points_against = s.points_against.saturating_sub(l.score[1].max(0) as u32);
        for (k, v) in &l.cells {
            if let Some(e) = s.cells.get_mut(k) {
                for i in 0..8 {
                    e[i] -= v[i];
                }
            }
        }
        sub_adv(&mut s.adv, &l.adv);
        sub_adv(&mut s.adv_against, &l.adv_against);
        s
    }
}

/// The open season's teams.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Open {
    pub seasons: HashMap<u64, Season>,
}

pub fn key(level: Level, team: u32) -> u64 {
    (level as u64) << 32 | team as u64
}

pub fn add_adv(a: &mut super::games::Adv, b: &super::games::Adv) {
    a.runs += b.runs;
    a.run_epa += b.run_epa;
    a.run_success += b.run_success;
    a.stuffed += b.stuffed;
    a.line_yards += b.line_yards;
    a.dropbacks += b.dropbacks;
    a.pass_epa += b.pass_epa;
    a.pass_success += b.pass_success;
    a.havoc += b.havoc;
    a.drives += b.drives;
    a.trips40 += b.trips40;
    a.trip_td += b.trip_td;
    a.trip_fg += b.trip_fg;
    a.three_outs += b.three_outs;
    a.start_sum += b.start_sum;
    a.explosive += b.explosive;
    a.success_epa += b.success_epa;
    a.rz_trips += b.rz_trips;
    a.rz_td += b.rz_td;
    a.rz_fg += b.rz_fg;
}

/// `a` less `b`, field by field.
pub fn sub_adv(a: &mut super::games::Adv, b: &super::games::Adv) {
    for (x, y) in [
        (&mut a.runs, b.runs),
        (&mut a.run_success, b.run_success),
        (&mut a.stuffed, b.stuffed),
        (&mut a.dropbacks, b.dropbacks),
        (&mut a.pass_success, b.pass_success),
        (&mut a.havoc, b.havoc),
        (&mut a.drives, b.drives),
        (&mut a.trips40, b.trips40),
        (&mut a.trip_td, b.trip_td),
        (&mut a.trip_fg, b.trip_fg),
        (&mut a.three_outs, b.three_outs),
        (&mut a.explosive, b.explosive),
        (&mut a.rz_trips, b.rz_trips),
        (&mut a.rz_td, b.rz_td),
        (&mut a.rz_fg, b.rz_fg),
    ] {
        *x = x.saturating_sub(y);
    }
    a.run_epa -= b.run_epa;
    a.line_yards -= b.line_yards;
    a.pass_epa -= b.pass_epa;
    a.start_sum -= b.start_sum;
    a.success_epa -= b.success_epa;
}

impl Open {
    /// A side's game on `date`: its counts, its and the other side's
    /// advanced numbers and the score (its, theirs).
    pub fn add(&mut self, level: Level, team: u32, date: i32, g: Game, adv: [&super::games::Adv; 2], score: [i32; 2]) {
        let s = self.seasons.entry(key(level, team)).or_insert_with(|| Season {
            level: level as u8,
            team,
            ..Default::default()
        });
        s.games += 1;
        s.wins += (score[0] > score[1]) as u32;
        s.losses += (score[0] < score[1]) as u32;
        s.points_for += score[0].max(0) as u32;
        s.points_against += score[1].max(0) as u32;
        let mut last = Last { date, score, adv: *adv[0], adv_against: *adv[1], ..Default::default() };
        for (k, v) in g.cells {
            // The key is copied only when the season hasn't had it yet.
            let add = |e: &mut [f32; 8]| {
                for i in 0..8 {
                    e[i] += v[i];
                }
            };
            if let Some(e) = s.cells.get_mut(&k) {
                add(e);
            } else {
                add(s.cells.entry(k.clone()).or_insert([0.0; 8]));
            }
            last.cells.insert(k, v);
        }
        s.last = Some(Box::new(last));
        for (k, (n, y)) in g.penalties {
            let e = s.penalties.entry(k).or_default();
            e.0 += n;
            e.1 += y;
        }
        add_adv(&mut s.adv, adv[0]);
        add_adv(&mut s.adv_against, adv[1]);
    }

    pub fn rows(&self) -> Vec<(u64, &Season)> {
        let mut v: Vec<(u64, &Season)> = self.seasons.iter().map(|(&k, s)| (k, s)).collect();
        v.sort_by_key(|x| x.0);
        v
    }
}
