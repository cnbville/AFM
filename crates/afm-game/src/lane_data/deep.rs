//! The players tally's depth counts on the Data pages (lane NB4, Data
//! overhaul items 4-15 and 28, minor bug 44): where a season's counts come
//! from (the open season in memory, a closed one from its file, read once
//! and kept), the opponents each man faced for the opponent-adjusted
//! numbers (item 13), and a player's Data tab: facet grades, splits, his
//! games, the throw map by zone, the route map, run paths, the pressure map
//! and where he lined up.
//!
//! Counts only: nothing here is a hidden rating.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use afm_engine::calendar::Date;
use afm_engine::game::stats::FACETS;
use afm_engine::world::World;
use afm_engine::world::analytics::{self as an, OppRate};
use afm_engine::world::marks::Tier;
use afm_engine::world::records::Stat;
use afm_engine::world::tally::{self, depth as dp, players};
use serde_json::{Value, json};

/// A player's season as the players tally keeps it: the fields the pages
/// read (the rest of the record is skipped).
#[derive(Clone, Default, serde::Deserialize)]
#[serde(default)]
pub(crate) struct Lite {
    pub id: u32,
    pub level: u8,
    pub team: u32,
    pub played: Vec<u64>,
    pub depth: dp::Depth,
    pub game_snaps: Vec<[u16; 2]>,
}

pub(crate) fn level_of(tier: Tier) -> Option<tally::Level> {
    match tier {
        Tier::College => Some(tally::Level::College),
        Tier::Nfl => Some(tally::Level::Nfl),
        Tier::Hs => None,
    }
}

/// One player's season of depth counts: his counts, less the games your
/// charting hasn't reached yet (`less`).
pub(crate) struct One<'b> {
    pub depth: &'b [(u16, f32)],
    pub less: Option<&'b [(u16, f32)]>,
    pub team: u32,
    pub played: &'b [u64],
    pub snaps: &'b [[u16; 2]],
}

impl One<'_> {
    pub fn get(&self, code: u16) -> f64 {
        let v = dp::get(self.depth, code);
        match self.less {
            Some(l) if !dp::is_max(code) => v - dp::get(l, code),
            _ => v,
        }
    }
}

/// A season's depth counts at a level.
pub(crate) enum Src<'a> {
    None,
    Open { open: &'a players::Open, level: tally::Level, less: HashMap<u32, dp::Depth> },
    Closed(Arc<HashMap<u32, Lite>>),
}

impl Src<'_> {
    pub fn one(&self, id: u32) -> Option<One<'_>> {
        match self {
            Src::None => None,
            Src::Open { open, level, less } => open.get(*level, id).map(|s| One {
                depth: &s.depth,
                less: less.get(&id).map(|d| d.as_slice()),
                team: s.team,
                played: &s.played,
                snaps: &s.game_snaps,
            }),
            Src::Closed(m) => m.get(&id).map(|s| One {
                depth: &s.depth,
                less: None,
                team: s.team,
                played: &s.played,
                snaps: &s.game_snaps,
            }),
        }
    }

    pub fn is_some(&self) -> bool {
        !matches!(self, Src::None)
    }
}

/// Closed seasons read from their files, the last few kept (a season's
/// file never changes once closed).
type Kept<T> = Mutex<Vec<((usize, u8, i32, String), Arc<T>)>>;
static SEASONS: Kept<HashMap<u32, Lite>> = Mutex::new(Vec::new());
static SCHEDS: Kept<Sched> = Mutex::new(Vec::new());
const KEEP: usize = 3;

fn kept<T>(store: &Kept<T>, key: (usize, u8, i32, String), make: impl FnOnce() -> T) -> Arc<T> {
    if let Some(x) = store.lock().ok().and_then(|v| v.iter().find(|e| e.0 == key).map(|e| e.1.clone())) {
        return x;
    }
    let x = Arc::new(make());
    if let Ok(mut v) = store.lock() {
        v.retain(|e| e.0 != key);
        v.push((key, x.clone()));
        if v.len() > KEEP {
            v.remove(0);
        }
    }
    x
}

/// Which world a cached season belongs to.
fn world_key(w: &World) -> usize {
    w as *const World as usize ^ w.tally_dir().map_or(0, |p| p.as_os_str().len())
}

/// A season's depth counts at a level (`since`: this season's games from
/// that day on are left out, your charting's delay; `keep` picks whose a
/// closed season keeps, under `scope`, the pool it was read for).
pub(crate) fn src<'a>(
    w: &'a World,
    tier: Tier,
    year: i32,
    since: Option<Date>,
    scope: &str,
    keep: &dyn Fn(&Lite) -> bool,
) -> Src<'a> {
    let Some(level) = level_of(tier) else { return Src::None };
    if !w.tally_seasons().contains(&year) {
        return Src::None;
    }
    if year == w.tally_open_season() {
        let less = since.map(|s| w.tally_depth_since(level, s)).unwrap_or_default();
        return Src::Open { open: &w.tallies.players, level, less };
    }
    let key = (world_key(w), level as u8, year, scope.to_string());
    Src::Closed(kept(&SEASONS, key, || {
        let mut m = HashMap::new();
        w.tally_players_as::<Lite>(year, |_, s| {
            if s.level == level as u8 && keep(&s) {
                m.insert(s.id, s);
            }
        });
        m
    }))
}

/// One player's season of depth counts (the full record: splits too).
pub(crate) fn season_of(w: &World, tier: Tier, year: i32, id: u32) -> Option<players::Season> {
    w.tally_player(level_of(tier)?, year, id)
}

// ---- Opponents (item 13). ----

/// A side's numbers in a game, or summed over games.
#[derive(Clone, Copy, Default)]
struct Ctx {
    db: f64,
    pass_epa: f64,
    pass_succ: f64,
    box_db: f64,
    prs: f64,
    sacks: f64,
    att: f64,
    cmp: f64,
    pass_yds: f64,
    runs: f64,
    run_epa: f64,
    run_succ: f64,
    stuffed: f64,
    carries: f64,
    rush_yds: f64,
}

impl Ctx {
    fn add(&mut self, o: &Ctx, sign: f64) {
        self.db += sign * o.db;
        self.pass_epa += sign * o.pass_epa;
        self.pass_succ += sign * o.pass_succ;
        self.box_db += sign * o.box_db;
        self.prs += sign * o.prs;
        self.sacks += sign * o.sacks;
        self.att += sign * o.att;
        self.cmp += sign * o.cmp;
        self.pass_yds += sign * o.pass_yds;
        self.runs += sign * o.runs;
        self.run_epa += sign * o.run_epa;
        self.run_succ += sign * o.run_succ;
        self.stuffed += sign * o.stuffed;
        self.carries += sign * o.carries;
        self.rush_yds += sign * o.rush_yds;
    }

    fn rate(&self, r: OppRate) -> Option<f64> {
        let q = |n: f64, d: f64| (d > 0.0).then(|| n / d);
        match r {
            OppRate::PassEpa => q(self.pass_epa, self.db),
            OppRate::PassSucc => q(self.pass_succ, self.db),
            OppRate::Ypa => q(self.pass_yds, self.att),
            OppRate::Cmp => q(self.cmp, self.att),
            OppRate::Sack => q(self.sacks, self.box_db),
            OppRate::Prs => q(self.prs, self.box_db),
            OppRate::RunEpa => q(self.run_epa, self.runs),
            OppRate::RunSucc => q(self.run_succ, self.runs),
            OppRate::Stuff => q(self.stuffed, self.runs),
            OppRate::Ypc => q(self.rush_yds, self.carries),
        }
    }
}

#[derive(Clone, Copy, Default, serde::Deserialize)]
#[serde(default)]
struct BoxLite {
    dropbacks: u32,
    pressures: u32,
    sacks: u32,
    att: u32,
    cmp: u32,
    pass_yards: f64,
    carries: u32,
    rush_yards: f64,
}

#[derive(Clone, Copy, Default, serde::Deserialize)]
#[serde(default)]
struct AdvLite {
    runs: u32,
    run_epa: f32,
    run_success: u32,
    stuffed: u32,
    dropbacks: u32,
    pass_epa: f32,
    pass_success: u32,
}

#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct GameLite {
    level: u8,
    kind: String,
    teams: [u32; 2],
    boxes: [BoxLite; 2],
    adv: [AdvLite; 2],
}

/// A season's games at a level, for the opponents each man faced.
#[derive(Default)]
pub(crate) struct Sched {
    games: HashMap<u64, ([u32; 2], [Ctx; 2])>,
    /// Each team's offense over the season.
    off: HashMap<u32, Ctx>,
    /// The offenses each team faced.
    def: HashMap<u32, Ctx>,
    /// (team, opponent): the team's offense and the opponent's in their
    /// games against each other.
    pair: HashMap<(u32, u32), [Ctx; 2]>,
    /// The NFL's playoff clubs.
    pub playoff: HashSet<u32>,
}

pub(crate) fn sched(w: &World, tier: Tier, year: i32) -> Option<Arc<Sched>> {
    let level = level_of(tier)?;
    if !w.tally_seasons().contains(&year) {
        return None;
    }
    let make = || {
        let mut s = Sched::default();
        w.tally_games_as::<GameLite>(year, |k, g| {
            if g.level != level as u8 {
                return;
            }
            let ctx = [0, 1].map(|i| {
                let (b, a) = (&g.boxes[i], &g.adv[i]);
                Ctx {
                    db: a.dropbacks as f64,
                    pass_epa: a.pass_epa as f64,
                    pass_succ: a.pass_success as f64,
                    box_db: b.dropbacks as f64,
                    prs: b.pressures as f64,
                    sacks: b.sacks as f64,
                    att: b.att as f64,
                    cmp: b.cmp as f64,
                    pass_yds: b.pass_yards,
                    runs: a.runs as f64,
                    run_epa: a.run_epa as f64,
                    run_succ: a.run_success as f64,
                    stuffed: a.stuffed as f64,
                    carries: b.carries as f64,
                    rush_yds: b.rush_yards,
                }
            });
            for i in 0..2 {
                let (t, o) = (g.teams[i], g.teams[1 - i]);
                s.off.entry(t).or_default().add(&ctx[i], 1.0);
                s.def.entry(t).or_default().add(&ctx[1 - i], 1.0);
                let p = s.pair.entry((t, o)).or_default();
                p[0].add(&ctx[i], 1.0);
                p[1].add(&ctx[1 - i], 1.0);
            }
            if matches!(g.kind.as_str(), "wild card" | "divisional" | "conference title" | "super bowl") {
                s.playoff.extend(g.teams);
            }
            s.games.insert(k, (g.teams, ctx));
        });
        s
    };
    // The open season grows game by game: read it fresh, keyed by the day.
    let day = if year == w.tally_open_season() { w.today.0 } else { 0 };
    Some(kept(&SCHEDS, (world_key(w), level as u8, year, day.to_string()), make))
}

impl Sched {
    /// The rate a man's opponents gave up (a man on offense: what offenses
    /// did against the defenses he faced) or produced (a man on defense:
    /// what the offenses he faced did against everyone), each game weighed
    /// by his snaps on that side, each opponent's season less its games
    /// against his team.
    pub fn faced(&self, one: &One, offense: bool, r: OppRate) -> Option<f64> {
        let (mut sw, mut sv) = (0.0, 0.0);
        for (k, key) in one.played.iter().enumerate() {
            let Some((teams, _)) = self.games.get(key) else { continue };
            let side = if teams[0] == one.team {
                0
            } else if teams[1] == one.team {
                1
            } else {
                continue;
            };
            let opp = teams[1 - side];
            let wgt = one.snaps.get(k).map_or(0.0, |s| s[if offense { 0 } else { 1 }] as f64);
            if wgt <= 0.0 {
                continue;
            }
            let pair = self.pair.get(&(opp, one.team));
            let mut c = if offense {
                self.def.get(&opp).copied().unwrap_or_default()
            } else {
                self.off.get(&opp).copied().unwrap_or_default()
            };
            if let Some(p) = pair {
                c.add(if offense { &p[1] } else { &p[0] }, -1.0);
            }
            let Some(v) = c.rate(r) else { continue };
            sw += wgt;
            sv += wgt * v;
        }
        (sw > 0.0).then(|| sv / sw)
    }
}

// ---- A player's Data tab (items 11, 15, 28; minor bug 44). ----

fn pct(n: f64, d: f64) -> Value {
    if d > 0.0 { json!(format!("{:.1}%", 100.0 * n / d)) } else { Value::Null }
}

fn per(n: f64, d: f64, places: usize) -> Value {
    if d > 0.0 { json!(format!("{:.*}", places, n / d)) } else { Value::Null }
}

fn signed(n: f64, d: f64) -> Value {
    if d > 0.0 { json!(format!("{:+.2}", n / d)) } else { Value::Null }
}

fn whole(v: f64) -> Value {
    json!(super::fmt_num(v))
}

/// The splits' rows for one role: (split name, cells).
fn split_rows<const N: usize>(
    rows: &[(u8, [f32; N])],
    name: &dyn Fn(u8) -> Option<String>,
    cells: &dyn Fn(&[f32; N]) -> Vec<Value>,
    extra: &[(String, [f32; N])],
) -> Vec<Value> {
    let mut v: Vec<(u8, &[f32; N])> = rows.iter().map(|(c, x)| (*c, x)).collect();
    v.sort_by_key(|x| x.0);
    let mut out: Vec<Value> =
        v.iter().filter_map(|(c, x)| Some(json!({ "split": name(*c)?, "cells": cells(x), "code": c }))).collect();
    for (n, x) in extra {
        out.push(json!({ "split": n, "cells": cells(x) }));
    }
    out
}

/// The NFL's opponents' splits summed over the playoff clubs.
fn sum_playoff<const N: usize>(rows: &[(u8, [f32; N])], playoff: &HashSet<u32>) -> Vec<(String, [f32; N])> {
    let mut s = [0f32; N];
    let mut any = false;
    for (c, x) in rows {
        if *c >= players::OPP && playoff.contains(&((*c - players::OPP) as u32)) {
            any = true;
            for (a, b) in s.iter_mut().zip(x) {
                *a += b;
            }
        }
    }
    if any { vec![("against playoff teams".to_string(), s)] } else { Vec::new() }
}

/// The splits table (item 15): by down, distance, field zone, quarter, the
/// last two minutes, the score, home and road, weather, the opponent's
/// ranking (college) or club (the NFL, with the playoff teams together).
fn splits(w: &World, tier: Tier, year: i32, s: &players::Season, playoff: &HashSet<u32>) -> Value {
    let name = |c: u8| -> Option<String> {
        if c >= players::OPP {
            let t = (c - players::OPP) as usize;
            return (tier == Tier::Nfl && t < w.nfl.len()).then(|| format!("against the {}", w.nfl_team_name(t)));
        }
        Some(players::split_name(c))
    };
    let _ = year;
    let pass = split_rows(
        &s.splits.pass,
        &name,
        &|x: &[f32; 11]| {
            let x = x.map(|v| v as f64);
            // dropbacks, att, cmp, yds, td, int, sacked, epa, success, first, scramble epa
            vec![
                whole(x[0]),
                pct(x[2], x[1]),
                whole(x[3]),
                per(x[3], x[1], 1),
                whole(x[4]),
                whole(x[5]),
                whole(x[6]),
                signed(x[7] + x[10], x[0]),
                pct(x[8], x[0]),
            ]
        },
        &sum_playoff(&s.splits.pass, playoff),
    );
    let rush = split_rows(
        &s.splits.rush,
        &name,
        &|x: &[f32; 6]| {
            let x = x.map(|v| v as f64);
            vec![whole(x[0]), whole(x[1]), per(x[1], x[0], 1), whole(x[2]), signed(x[3], x[0]), pct(x[4], x[0])]
        },
        &sum_playoff(&s.splits.rush, playoff),
    );
    let recv = split_rows(
        &s.splits.recv,
        &name,
        &|x: &[f32; 6]| {
            let x = x.map(|v| v as f64);
            vec![whole(x[0]), whole(x[1]), whole(x[2]), per(x[2], x[1], 1), whole(x[3]), signed(x[4], x[0])]
        },
        &sum_playoff(&s.splits.recv, playoff),
    );
    let def = split_rows(
        &s.splits.def,
        &name,
        &|x: &[f32; 9]| {
            let x = x.map(|v| v as f64);
            vec![
                whole(x[0]),
                whole(x[1]),
                whole(x[2]),
                whole(x[3]),
                whole(x[4]),
                pct(x[5], x[4]),
                whole(x[6]),
                whole(x[7]),
                whole(x[8]),
            ]
        },
        &sum_playoff(&s.splits.def, playoff),
    );
    let mut out = Vec::new();
    let mut push = |title: &str, heads: &[&str], rows: Vec<Value>, vol: f64| {
        if !rows.is_empty() && vol > 0.0 {
            out.push(json!({ "title": title, "heads": heads, "rows": rows }));
        }
    };
    let total = |v: &[(u8, [f32; 11])]| v.iter().filter(|x| x.0 >= 20 && x.0 <= 22).map(|x| x.1[0] as f64).sum::<f64>();
    push("Passing", &["Db", "Cmp%", "Yds", "Y/A", "TD", "INT", "Sk", "EPA/db", "Succ%"], pass, total(&s.splits.pass));
    let t6 = |v: &[(u8, [f32; 6])]| v.iter().filter(|x| x.0 >= 20 && x.0 <= 22).map(|x| x.1[0] as f64).sum::<f64>();
    push("Rushing", &["Car", "Yds", "Y/C", "TD", "EPA/C", "Succ%"], rush, t6(&s.splits.rush));
    push("Receiving", &["Tgt", "Rec", "Yds", "Y/R", "TD", "EPA/T"], recv, t6(&s.splits.recv));
    let t9 = |v: &[(u8, [f32; 9])]| {
        v.iter().filter(|x| x.0 >= 20 && x.0 <= 22).map(|x| x.1.iter().map(|y| *y as f64).sum::<f64>()).sum::<f64>()
    };
    push("Defense", &["Tkl", "Miss", "Prs", "Sk", "Tgt", "Cmp% A", "Yds A", "PBU", "INT"], def, t9(&s.splits.def));
    json!(out)
}

/// His games that season, from the games file: the date, the opponent,
/// where, the score and his line's main numbers for his group.
fn games(w: &World, tier: Tier, year: i32, s: &players::Season, group: usize) -> Value {
    let (heads, stats): (Vec<&str>, Vec<Stat>) = match group {
        0 => (
            vec!["Cmp", "Att", "Yds", "TD", "INT", "Sk"],
            vec![Stat::Cmp, Stat::Att, Stat::PassYds, Stat::PassTd, Stat::Int, Stat::Sacked],
        ),
        1 => (
            vec!["Car", "Yds", "TD", "Rec", "Yds", "Fum"],
            vec![Stat::Rush, Stat::RushYds, Stat::RushTd, Stat::Rec, Stat::RecYds, Stat::FumblesLost],
        ),
        2 | 3 => (
            vec!["Tgt", "Rec", "Yds", "TD", "Drop"],
            vec![Stat::Targets, Stat::Rec, Stat::RecYds, Stat::RecTd, Stat::Drops],
        ),
        4 => (
            vec!["PB", "Prs A", "Sk A", "RB", "RB wins"],
            vec![Stat::PbSnaps, Stat::PressuresAllowed, Stat::SacksAllowed, Stat::RbSnaps, Stat::RbWins],
        ),
        10 => (vec!["FG", "FGA", "XP", "Long"], vec![Stat::FgMade, Stat::FgAtt, Stat::XpMade, Stat::FgLong]),
        11 => (vec!["Punts", "Net"], vec![Stat::Punts, Stat::PuntNet]),
        an::RET => (vec!["Ret", "Yds", "TD"], vec![Stat::Returns, Stat::ReturnYds, Stat::ReturnTd]),
        _ => (
            vec!["Tkl", "TFL", "Sk", "Prs", "PBU", "INT"],
            vec![Stat::Tackles, Stat::Tfl, Stat::Sacks, Stat::Pressures, Stat::Pbus, Stat::Ints],
        ),
    };
    let mut rows = Vec::new();
    w.tally_games(year, Some(&s.played), |_, g| {
        let Some(line) = g.lines.iter().find(|l| l.id == s.id) else { return };
        let side = line.side as usize;
        let opp = g.teams[1 - side.min(1)] as usize;
        let at = if g.neutral {
            "vs"
        } else if side == 0 {
            ""
        } else {
            "at"
        };
        let (us, them) = (g.score[side.min(1)], g.score[1 - side.min(1)]);
        let res = if us > them {
            "W"
        } else if us < them {
            "L"
        } else {
            "T"
        };
        rows.push(json!({
            "date": g.date().to_string(),
            "at": at,
            "opp": super::team_json(w, tier, opp),
            "result": format!("{res} {us}-{them}"),
            "kind": g.kind,
            "cells": stats.iter().map(|&k| super::fmt_num(line.get(k))).collect::<Vec<_>>(),
        }));
    });
    json!({ "heads": heads, "rows": rows })
}

/// The throw map by zone (item 28): his role's 4 depth rows by 3 thirds,
/// each cell's attempts, completion %, yards and EPA a throw.
fn throw_map(o: &One, role: usize) -> Value {
    let rows: Vec<Value> = (0..4)
        .map(|row| {
            let cells: Vec<Value> = (0..3)
                .map(|t| {
                    let g = |k: usize| o.get(dp::throw_code(role, row, t, k));
                    json!({
                        "att": g(0) as i64,
                        "cmp": pct(g(1), g(0)),
                        "yds": g(2).round() as i64,
                        "epa": signed(g(3), g(0)),
                        "epa_v": if g(0) > 0.0 { g(3) / g(0) } else { 0.0 },
                    })
                })
                .collect();
            let depth = ["Behind the line", "0-9", "10-19", "20+"][row];
            json!({ "depth": depth, "cells": cells })
        })
        .collect();
    let who = ["passer", "target", "covering"][role];
    json!({ "role": who, "rows": rows, "thirds": ["Left", "Middle", "Right"] })
}

/// The route map (minor bug 44): each route he was targeted on.
fn route_map(o: &One) -> Value {
    let rows: Vec<Value> = dp::ROUTES
        .iter()
        .enumerate()
        .filter_map(|(k, name)| {
            let g = |j: usize| o.get(dp::route_code(k, j));
            (g(0) > 0.0).then(|| {
                json!({
                    "route": name, "tgt": g(0) as i64, "rec": g(1) as i64, "yds": g(2).round() as i64,
                    "catch": pct(g(1), g(0)), "ypt": per(g(2), g(0), 1), "epa": signed(g(3), g(0)),
                    "sep": per(g(4), g(0), 2),
                })
            })
        })
        .collect();
    json!(rows)
}

/// Run paths (minor bug 44): his designed runs by gap, left to right.
fn run_paths(o: &One) -> Value {
    let names = ["Left end", "Left inside", "Middle", "Right inside", "Right end"];
    let rows: Vec<Value> = (0..5)
        .map(|gap| {
            let g = |k: usize| o.get(dp::gap_code(gap, k));
            json!({
                "gap": names[gap], "car": g(0) as i64, "yds": g(1).round() as i64, "ypc": per(g(1), g(0), 1),
                "ybc": per(g(2), g(0), 2), "epa": signed(g(3), g(0)), "succ": pct(g(4), g(0)), "expl": pct(g(5), g(0)),
            })
        })
        .collect();
    json!(rows)
}

/// The pressure map (minor bug 44): where his pressures came, by the
/// blocker he beat.
fn pressure_map(o: &One) -> Value {
    let rows: Vec<Value> = dp::SPOTS
        .iter()
        .enumerate()
        .map(|(k, name)| {
            let g = |j: usize| o.get(dp::pmap_code(k, j));
            json!({ "spot": name, "prs": g(0) as i64, "sacks": super::fmt_num(g(1)), "ttp": per(g(2), g(0), 2) })
        })
        .collect();
    json!(rows)
}

/// Where he lined up: snaps at each spot.
fn alignment(o: &One) -> Value {
    let mut out = Vec::new();
    for (k, n) in an::SPOTS_O.iter().enumerate() {
        let v = an::spot_snaps(&|c| o.get(c), true, k);
        if v > 0.0 {
            out.push(json!({ "spot": n, "snaps": v as i64 }));
        }
    }
    for (k, n) in an::SPOTS_D.iter().enumerate() {
        let v = an::spot_snaps(&|c| o.get(c), false, k);
        if v > 0.0 {
            out.push(json!({ "spot": n, "snaps": v as i64 }));
        }
    }
    json!(out)
}

/// The facet grades (item 11): each facet's grade and the plays behind it.
fn facets(o: &One) -> Value {
    let rows: Vec<Value> = FACETS
        .iter()
        .enumerate()
        .filter_map(|(f, name)| {
            let n = o.get(dp::FACET + 2 * f as u16 + 1);
            (n >= 1.0).then(|| {
                let g = afm_engine::game::stats::StatLine {
                    grade_sum: o.get(dp::FACET + 2 * f as u16),
                    graded: n.round() as u32,
                    ..Default::default()
                }
                .grade();
                json!({ "facet": name, "grade": format!("{g:.1}"), "value": g, "plays": n as i64 })
            })
        })
        .collect();
    json!(rows)
}

/// `player_data` (`id`, `level`, `year`): a player's Data tab for one
/// season at a level with tallies (college or the NFL).
pub fn player_data(w: &World, args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_u64).map(|x| x as u32) else {
        return json!({ "error": "No player." });
    };
    let k = crate::views::person_index(w, id);
    // His levels with tallies: where he plays now first.
    let stage_tier = k.map(|k| match w.people[k].stage {
        afm_engine::world::Stage::Pro => Tier::Nfl,
        _ => Tier::College,
    });
    let tier = match args.get("level").and_then(Value::as_str) {
        Some("nfl") => Tier::Nfl,
        Some("college") => Tier::College,
        _ => stage_tier.unwrap_or(Tier::College),
    };
    let Some(level) = level_of(tier) else { return json!({ "error": "No tallies at this level." }) };
    let mut years: Vec<i32> =
        w.tally_seasons().into_iter().filter(|&y| w.tally_player(level, y, id).is_some_and(|s| s.games > 0)).collect();
    years.reverse();
    let other = if tier == Tier::Nfl { Tier::College } else { Tier::Nfl };
    let other_has = level_of(other)
        .is_some_and(|l| w.tally_seasons().into_iter().any(|y| w.tally_player(l, y, id).is_some_and(|s| s.games > 0)));
    let year = args
        .get("year")
        .and_then(Value::as_i64)
        .map(|y| y as i32)
        .filter(|y| years.contains(y))
        .or(years.first().copied());
    let mut m = serde_json::Map::new();
    m.insert("id".into(), json!(id));
    m.insert("level".into(), json!(super::tier_key(tier)));
    m.insert("other".into(), json!(other_has.then(|| super::tier_key(other))));
    m.insert("years".into(), json!(years));
    let Some(year) = year else {
        m.insert("empty".into(), json!("No charted season at this level since the tallies began."));
        return Value::Object(m);
    };
    let Some(s) = season_of(w, tier, year, id) else { return Value::Object(m) };
    // The charting your seat has there, and its delay this season.
    let division = (tier == Tier::College).then(|| super::season_place(w, s.team as usize, year).0);
    let ch = super::charting(w, tier, division);
    let open = year == w.tally_open_season();
    let since = ch.as_ref().filter(|_| open).map(|c| Date(w.today.0 - (c.days as i32 - 1)));
    let less = since.map(|d| w.tally_depth_since(level, d)).unwrap_or_default();
    let one = One {
        depth: &s.depth,
        less: less.get(&id).map(|d| d.as_slice()),
        team: s.team,
        played: &s.played,
        snaps: &s.game_snaps,
    };
    let group = s
        .pos
        .and_then(an::group_of)
        .unwrap_or_else(|| k.and_then(|k| an::group_of(w.people[k].player.position)).unwrap_or(0));
    let playoff = if tier == Tier::Nfl {
        sched(w, tier, year).map(|x| x.playoff.clone()).unwrap_or_default()
    } else {
        HashSet::new()
    };
    m.insert("year".into(), json!(year));
    m.insert("team".into(), super::team_json(w, tier, s.team as usize));
    m.insert("games".into(), json!(s.games));
    m.insert("starts".into(), json!(s.starts));
    m.insert("snaps".into(), json!(s.snaps));
    m.insert("group".into(), json!(an::GROUPS[group]));
    m.insert("splits".into(), splits(w, tier, year, &s, &playoff));
    m.insert("log".into(), games(w, tier, year, &s, group));
    // What film and tracking give: only as deep as your charting goes.
    let depth = ch.as_ref().map_or(0, |c| c.depth);
    m.insert("charting".into(), ch.as_ref().map_or(Value::Null, super::charting_json));
    if depth >= 2 {
        m.insert("facets".into(), facets(&one));
        m.insert("alignment".into(), alignment(&one));
        m.insert("routes".into(), route_map(&one));
        m.insert("pressure".into(), pressure_map(&one));
    }
    if depth >= 1 {
        // The throw map for the role he had most throws in.
        let att = |role: usize| (0..12).map(|c| one.get(dp::throw_code(role, c / 3, c % 3, 0))).sum::<f64>();
        let role = (0..3).max_by(|&a, &b| att(a).total_cmp(&att(b))).unwrap_or(0);
        if att(role) > 0.0 && (role != 2 || depth >= 2) {
            m.insert("throws".into(), throw_map(&one, role));
        }
        if one.get(dp::Dc::RAtt as u16) > 0.0 {
            m.insert("runs".into(), run_paths(&one));
        }
    }
    Value::Object(m)
}
