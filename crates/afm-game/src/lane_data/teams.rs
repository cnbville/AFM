//! The team Data pages (lane NB7, Data overhaul items 16-23; boards 44 and
//! 53 in Press Box), read from the squads tally (`world::tally::squads`),
//! the teams tally's situation cells, the games file and the stat book:
//! - `team_data` (`kind`, `id`, `year`, `tab`): a team's Data page, one tab
//!   at a time: its analytics
//!   with ranks (item 17; box scores only where your seat charts nothing),
//!   its tendencies (item 16), its games with the unit grades and snap
//!   shares (item 21), what came back and the roster's talent (item 22)
//!   and its power rating, projection, coming games and odds (item 23);
//! - `team_seasons` (`kind`, `id`, `unit`): its stats by season with each
//!   rank, the coordinator, the talent and the starters back (item 18,
//!   board 44);
//! - `team_leaders` (`level`, `year`, `division`, `state`, `stat`, `view`):
//!   every team at a level ranked by any number, any season (item 19), the
//!   tiers chart (item 20, `view` "tiers") and the power ratings with the
//!   odds (item 23, `view` "power").
//!
//! Numbers only: nothing here reads a hidden rating. The power rating
//! reads scores, the schedule, the production back and the recruiting
//! stars (`world::power`).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use afm_engine::game::stats::StatLine;
use afm_engine::world::World;
use afm_engine::world::coach::Role;
use afm_engine::world::data::Division;
use afm_engine::world::folk::Team;
use afm_engine::world::marks::Tier;
use afm_engine::world::power::{self, Inputs, Odds, Rating, SGame};
use afm_engine::world::records::{SeasonRecord, Stat};
use afm_engine::world::tally::squads::{self, Season as Sq};
use afm_engine::world::tally::{self, Level, teams};
use afm_engine::world::units::Charge;
use serde_json::{Value, json};

use super::{tier_arg, tier_key};
use crate::views::{self, TeamRef};

// ---------------------------------------------------------------------
// The numbers.

/// How a number is written.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Fmt {
    One,
    Two,
    Pct,
    Signed,
    Clock,
}

/// One number a team is judged by. `side` 0 its offense, 1 its defense
/// (what it allowed), read the offense's way: a defense ranks the other
/// way. `unit`: 0 both sides, 2 the whole team, 3 special teams, 4 the
/// offense only. `depth`: 0 any box score, 1 play-by-play charting, 2 film
/// (`analytics::depth`). `up`: more is better for the offense (None: no
/// better way).
pub(super) struct TStat {
    pub(super) key: &'static str,
    pub(super) label: &'static str,
    pub(super) unit: u8,
    pub(super) depth: u8,
    pub(super) up: Option<bool>,
    pub(super) fmt: Fmt,
    pub(super) f: fn(&Sq, usize, &Lv) -> Option<f64>,
}

/// What a number needs from the level: its share of passes defensed
/// picked off that season (expected turnovers).
#[derive(Clone, Copy, Default)]
pub(super) struct Lv {
    pick_share: f64,
}

fn ratio(n: f64, d: f64) -> Option<f64> {
    (d > 0.0).then(|| n / d)
}

fn bx(s: &Sq, side: usize, k: &str) -> f64 {
    s.box_of(side, k)
}

fn adv(s: &Sq, side: usize) -> &tally::games::Adv {
    if side == 0 { &s.adv } else { &s.adv_against }
}

fn games_of(s: &Sq) -> f64 {
    s.games.max(1) as f64
}

/// Expected takeaways and giveaways a game (Bill Connelly's turnover luck:
/// half of all fumbles, and the level's share of passes defensed).
fn expected_to(s: &Sq, lv: &Lv) -> (f64, f64) {
    let take = 0.5 * s.luck_of("opp_fumbles") + lv.pick_share * s.luck_of("pd");
    let give = 0.5 * s.luck_of("fumbles") + lv.pick_share * s.luck_of("pd_against");
    (take / games_of(s), give / games_of(s))
}

pub(super) const STATS: &[TStat] = &[
    TStat {
        key: "TmPtsG",
        label: "Points a game",
        unit: 0,
        depth: 0,
        up: Some(true),
        fmt: Fmt::One,
        f: |s, d, _| ratio(bx(s, d, "points"), games_of(s)),
    },
    TStat {
        key: "TmYdsG",
        label: "Yards a game",
        unit: 0,
        depth: 0,
        up: Some(true),
        fmt: Fmt::One,
        f: |s, d, _| ratio(bx(s, d, "yards"), games_of(s)),
    },
    TStat {
        key: "TmYpp",
        label: "Yards a play",
        unit: 0,
        depth: 0,
        up: Some(true),
        fmt: Fmt::Two,
        f: |s, d, _| ratio(bx(s, d, "yards"), bx(s, d, "plays")),
    },
    TStat {
        key: "TmPassG",
        label: "Passing yards a game",
        unit: 0,
        depth: 0,
        up: Some(true),
        fmt: Fmt::One,
        f: |s, d, _| ratio(bx(s, d, "pass_yards"), games_of(s)),
    },
    TStat {
        key: "TmRushG",
        label: "Rushing yards a game",
        unit: 0,
        depth: 0,
        up: Some(true),
        fmt: Fmt::One,
        f: |s, d, _| ratio(bx(s, d, "rush_yards"), games_of(s)),
    },
    TStat {
        key: "TmYpc",
        label: "Yards a carry",
        unit: 0,
        depth: 0,
        up: Some(true),
        fmt: Fmt::Two,
        f: |s, d, _| ratio(bx(s, d, "rush_yards"), bx(s, d, "carries")),
    },
    TStat {
        key: "TmFdG",
        label: "First downs a game",
        unit: 0,
        depth: 0,
        up: Some(true),
        fmt: Fmt::One,
        f: |s, d, _| ratio(bx(s, d, "first_downs"), games_of(s)),
    },
    TStat {
        key: "TmThird",
        label: "Third down rate",
        unit: 0,
        depth: 0,
        up: Some(true),
        fmt: Fmt::Pct,
        f: |s, d, _| ratio(bx(s, d, "third_conv"), bx(s, d, "third_att")),
    },
    TStat {
        key: "TmFourth",
        label: "Fourth down rate",
        unit: 0,
        depth: 0,
        up: Some(true),
        fmt: Fmt::Pct,
        f: |s, d, _| ratio(bx(s, d, "fourth_conv"), bx(s, d, "fourth_att")),
    },
    TStat {
        key: "TmRz",
        label: "Red zone touchdown rate",
        unit: 0,
        depth: 0,
        up: Some(true),
        fmt: Fmt::Pct,
        f: |s, d, _| ratio(bx(s, d, "red_zone_tds"), bx(s, d, "red_zone_trips")),
    },
    TStat {
        key: "TmToG",
        label: "Turnovers a game",
        unit: 0,
        depth: 0,
        up: Some(false),
        fmt: Fmt::Two,
        f: |s, d, _| ratio(bx(s, d, "turnovers"), games_of(s)),
    },
    TStat {
        key: "TmSackRate",
        label: "Sack rate",
        unit: 0,
        depth: 0,
        up: Some(false),
        fmt: Fmt::Pct,
        f: |s, d, _| ratio(bx(s, d, "sacks"), bx(s, d, "dropbacks")),
    },
    TStat {
        key: "TmEpa",
        label: "EPA a play",
        unit: 0,
        depth: 1,
        up: Some(true),
        fmt: Fmt::Signed,
        f: |s, d, _| ratio(bx(s, d, "epa"), bx(s, d, "plays")),
    },
    TStat {
        key: "TmSucc",
        label: "Success rate",
        unit: 0,
        depth: 1,
        up: Some(true),
        fmt: Fmt::Pct,
        f: |s, d, _| ratio(bx(s, d, "successes"), bx(s, d, "plays")),
    },
    TStat {
        key: "TmExpl",
        label: "Explosive play rate",
        unit: 0,
        depth: 1,
        up: Some(true),
        fmt: Fmt::Pct,
        f: |s, d, _| ratio(bx(s, d, "explosive"), bx(s, d, "plays")),
    },
    TStat {
        key: "TmRunEpa",
        label: "EPA a designed run",
        unit: 0,
        depth: 1,
        up: Some(true),
        fmt: Fmt::Signed,
        f: |s, d, _| ratio(adv(s, d).run_epa as f64, adv(s, d).runs as f64),
    },
    TStat {
        key: "TmPassEpa",
        label: "EPA a dropback",
        unit: 0,
        depth: 1,
        up: Some(true),
        fmt: Fmt::Signed,
        f: |s, d, _| ratio(adv(s, d).pass_epa as f64, adv(s, d).dropbacks as f64),
    },
    TStat {
        key: "TmRunSucc",
        label: "Rushing success rate",
        unit: 0,
        depth: 1,
        up: Some(true),
        fmt: Fmt::Pct,
        f: |s, d, _| ratio(adv(s, d).run_success as f64, adv(s, d).runs as f64),
    },
    TStat {
        key: "TmPassSucc",
        label: "Passing success rate",
        unit: 0,
        depth: 1,
        up: Some(true),
        fmt: Fmt::Pct,
        f: |s, d, _| ratio(adv(s, d).pass_success as f64, adv(s, d).dropbacks as f64),
    },
    TStat {
        key: "TmLineYds",
        label: "Line yards a carry",
        unit: 0,
        depth: 1,
        up: Some(true),
        fmt: Fmt::Two,
        f: |s, d, _| ratio(adv(s, d).line_yards as f64, adv(s, d).runs as f64),
    },
    TStat {
        key: "TmStuff",
        label: "Stuff rate",
        unit: 0,
        depth: 1,
        up: Some(false),
        fmt: Fmt::Pct,
        f: |s, d, _| ratio(adv(s, d).stuffed as f64, adv(s, d).runs as f64),
    },
    // Havoc an offense suffered is what the other side's defense made.
    TStat {
        key: "TmHavoc",
        label: "Havoc rate",
        unit: 0,
        depth: 1,
        up: Some(false),
        fmt: Fmt::Pct,
        f: |s, d, _| ratio(adv(s, 1 - d).havoc as f64, bx(s, d, "plays")),
    },
    TStat {
        key: "TmPtsTrip",
        label: "Points a scoring chance",
        unit: 0,
        depth: 1,
        up: Some(true),
        fmt: Fmt::Two,
        f: |s, d, _| {
            let a = adv(s, d);
            ratio((7 * a.trip_td + 3 * a.trip_fg) as f64, a.trips40 as f64)
        },
    },
    TStat {
        key: "TmTripsG",
        label: "Scoring chances a game",
        unit: 0,
        depth: 1,
        up: Some(true),
        fmt: Fmt::One,
        f: |s, d, _| ratio(adv(s, d).trips40 as f64, games_of(s)),
    },
    TStat {
        key: "TmStart",
        label: "Average start",
        unit: 0,
        depth: 1,
        up: Some(true),
        fmt: Fmt::One,
        f: |s, d, _| ratio(adv(s, d).start_sum as f64, adv(s, d).drives as f64),
    },
    TStat {
        key: "TmThreeOut",
        label: "Three and outs a game",
        unit: 0,
        depth: 1,
        up: Some(false),
        fmt: Fmt::One,
        f: |s, d, _| ratio(adv(s, d).three_outs as f64, games_of(s)),
    },
    TStat {
        key: "TmPress",
        label: "Pressure rate",
        unit: 0,
        depth: 2,
        up: Some(false),
        fmt: Fmt::Pct,
        f: |s, d, _| ratio(bx(s, d, "pressures"), bx(s, d, "dropbacks")),
    },
    // The offense's calls against the level's own (pass rate over expected).
    TStat {
        key: "TmProe",
        label: "Pass rate over expected",
        unit: 4,
        depth: 1,
        up: None,
        fmt: Fmt::Pct,
        f: |s, _, _| {
            let [n, db, x] = s.tend_of(1);
            ratio((db - x) as f64, n as f64)
        },
    },
    TStat {
        key: "TmEarlyPass",
        label: "Early-down pass rate",
        unit: 4,
        depth: 1,
        up: None,
        fmt: Fmt::Pct,
        f: |s, _, _| {
            let [n, db, _] = s.tend_of(1);
            ratio(db as f64, n as f64)
        },
    },
    TStat {
        key: "TmToMargin",
        label: "Turnover margin a game",
        unit: 2,
        depth: 0,
        up: Some(true),
        fmt: Fmt::Signed,
        f: |s, _, _| ratio(bx(s, 1, "turnovers") - bx(s, 0, "turnovers"), games_of(s)),
    },
    TStat {
        key: "TmXToMargin",
        label: "Expected turnover margin a game",
        unit: 2,
        depth: 1,
        up: Some(true),
        fmt: Fmt::Signed,
        f: |s, _, lv| {
            let (t, g) = expected_to(s, lv);
            (s.luck.len() == squads::LUCK.len()).then_some(t - g)
        },
    },
    TStat {
        key: "TmToLuck",
        label: "Turnover luck a game",
        unit: 2,
        depth: 1,
        up: None,
        fmt: Fmt::Signed,
        f: |s, _, lv| {
            let (t, g) = expected_to(s, lv);
            (s.luck.len() == squads::LUCK.len())
                .then(|| (bx(s, 1, "turnovers") - bx(s, 0, "turnovers")) / games_of(s) - (t - g))
        },
    },
    TStat {
        key: "TmPenG",
        label: "Penalties a game",
        unit: 2,
        depth: 0,
        up: Some(false),
        fmt: Fmt::One,
        f: |s, _, _| ratio(bx(s, 0, "penalties"), games_of(s)),
    },
    TStat {
        key: "TmPenYdsG",
        label: "Penalty yards a game",
        unit: 2,
        depth: 0,
        up: Some(false),
        fmt: Fmt::One,
        f: |s, _, _| ratio(bx(s, 0, "penalty_yds"), games_of(s)),
    },
    TStat {
        key: "TmTop",
        label: "Time of possession a game",
        unit: 2,
        depth: 0,
        up: None,
        fmt: Fmt::Clock,
        f: |s, _, _| ratio(bx(s, 0, "clock_possession"), games_of(s)),
    },
    TStat {
        key: "TmFgPct",
        label: "Field goal percentage",
        unit: 3,
        depth: 0,
        up: Some(true),
        fmt: Fmt::Pct,
        f: |s, _, _| ratio(s.st_of("fg_made"), s.st_of("fg_att")),
    },
    TStat {
        key: "TmXpPct",
        label: "Extra point percentage",
        unit: 3,
        depth: 0,
        up: Some(true),
        fmt: Fmt::Pct,
        f: |s, _, _| ratio(s.st_of("xp_made"), s.st_of("xp_att")),
    },
    TStat {
        key: "TmPuntNet",
        label: "Net punting average",
        unit: 3,
        depth: 0,
        up: Some(true),
        fmt: Fmt::One,
        f: |s, _, _| ratio(s.st_of("punt_net"), s.st_of("punts")),
    },
    TStat {
        key: "TmPuntIn20",
        label: "Punts inside the 20",
        unit: 3,
        depth: 0,
        up: Some(true),
        fmt: Fmt::Pct,
        f: |s, _, _| ratio(s.st_of("punts_in20"), s.st_of("punts")),
    },
    TStat {
        key: "TmKoTb",
        label: "Kickoff touchback rate",
        unit: 3,
        depth: 0,
        up: Some(true),
        fmt: Fmt::Pct,
        f: |s, _, _| ratio(s.st_of("kickoff_tb"), s.st_of("kickoffs")),
    },
    TStat {
        key: "TmKrAvg",
        label: "Kick return average",
        unit: 3,
        depth: 0,
        up: Some(true),
        fmt: Fmt::One,
        f: |s, _, _| ratio(s.st_of("kr_yds"), s.st_of("kr")),
    },
    TStat {
        key: "TmPrAvg",
        label: "Punt return average",
        unit: 3,
        depth: 0,
        up: Some(true),
        fmt: Fmt::One,
        f: |s, _, _| ratio(s.st_of("pr_yds"), s.st_of("pr")),
    },
    TStat {
        key: "TmBlocked",
        label: "Kicks blocked",
        unit: 3,
        depth: 0,
        up: Some(true),
        fmt: Fmt::One,
        f: |s, _, _| (!s.st.is_empty()).then(|| s.st_of("blocked")),
    },
];

/// The numbers' keys, for the glossary.
#[cfg(test)]
pub(crate) fn stat_keys() -> impl Iterator<Item = &'static str> {
    STATS.iter().map(|s| s.key)
}

pub(super) fn text(fmt: Fmt, v: f64) -> String {
    match fmt {
        Fmt::One => format!("{v:.1}"),
        Fmt::Two => format!("{v:.2}"),
        Fmt::Pct => format!("{:.1}%", 100.0 * v),
        Fmt::Signed => format!("{v:+.2}"),
        Fmt::Clock => {
            let s = v.round() as i64;
            format!("{}:{:02}", s / 60, s % 60)
        }
    }
}

/// Better for this side: more, less, or neither.
pub(super) fn better(st: &TStat, side: usize) -> Option<bool> {
    st.up.map(|u| if side == 1 { !u } else { u })
}

// ---------------------------------------------------------------------
// The level's teams in a season.

pub(super) fn level_of(tier: Tier) -> Level {
    match tier {
        Tier::College => Level::College,
        Tier::Nfl => Level::Nfl,
        Tier::Hs => Level::Hs,
    }
}

pub(super) fn tier_of(t: TeamRef) -> Option<(Tier, usize)> {
    match t {
        TeamRef::College(p) => Some((Tier::College, p)),
        TeamRef::Nfl(i) => Some((Tier::Nfl, i)),
        TeamRef::HighSchool(h) => Some((Tier::Hs, h)),
        TeamRef::Other(_) => None,
    }
}

fn team_of(tier: Tier, t: usize) -> Team {
    match tier {
        Tier::College => Team::College(t),
        Tier::Nfl => Team::Nfl(t),
        Tier::Hs => Team::High(t),
    }
}

/// A season's squads at a level (a closed season read once and kept).
type Kept<T> = Mutex<Vec<((usize, u8, i32), Arc<T>)>>;
static POOLS: Kept<Vec<Sq>> = Mutex::new(Vec::new());
static POWER: Kept<PowerSeason> = Mutex::new(Vec::new());
static FITS: Kept<[power::Weights; 2]> = Mutex::new(Vec::new());
const KEEP: usize = 6;

fn kept<T>(store: &Kept<T>, key: (usize, u8, i32), make: impl FnOnce() -> T) -> Arc<T> {
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

fn world_key(w: &World) -> usize {
    w as *const World as usize ^ w.tally_dir().map_or(0, |p| p.as_os_str().len())
}

pub(super) fn pool(w: &World, level: Level, season: i32) -> Arc<Vec<Sq>> {
    let read = || {
        let mut v = Vec::new();
        if season == w.tally_open_season() {
            w.tally_squads(season, |s| {
                if s.level == level as u8 {
                    v.push(s.clone());
                }
            });
        } else {
            // Only the level's own records are decoded (keys start with it).
            let _ = w.tally_each("squads", season, |k, r| {
                if (k >> 32) as u8 == level as u8
                    && let Some(s) = tally::file::decode::<Sq>(r)
                {
                    v.push(s);
                }
            });
        }
        v
    };
    if season == w.tally_open_season() {
        return Arc::new(read());
    }
    kept(&POOLS, (world_key(w), level as u8, season), read)
}

/// The level's share of passes defensed that were picked off that season.
pub(super) fn level_numbers(all: &[Sq]) -> Lv {
    let (pd, picks) = all.iter().fold((0.0, 0.0), |a, s| (a.0 + s.luck_of("pd"), a.1 + s.luck_of("picks")));
    Lv { pick_share: if pd > 0.0 { picks / pd } else { 0.22 } }
}

/// The group a team is ranked in: its college division that season, the
/// NFL, or its state's high schools of its class (proposed, not
/// confirmed: a state's class, as its playoffs are).
pub(super) fn group_name(w: &World, tier: Tier, team: usize, year: i32) -> String {
    match tier {
        Tier::College => super::season_place(w, team, year).0.name().to_string(),
        Tier::Nfl => "the NFL".into(),
        Tier::Hs => {
            let h = &w.high_schools[team];
            let st = &w.states[h.state];
            let class = afm_engine::world::hs_playoffs::format(&st.code)
                .class_label(h.classification)
                .map(str::to_string)
                .unwrap_or_else(|| format!("class {}", h.classification + 1));
            format!("{} {class}", afm_engine::world::folk::facts::state_name(&st.code))
        }
    }
}

/// A season's teams in one group, with the group's own numbers.
struct Group {
    rows: Vec<Sq>,
    lv: Lv,
    name: String,
}

/// The teams in a team's group that season: its division's programs, the
/// NFL's clubs, its state's schools of its class.
pub(super) fn members(w: &World, tier: Tier, team: usize, year: i32) -> Vec<u32> {
    match tier {
        Tier::College => {
            let d = super::season_place(w, team, year).0;
            (0..w.programs.len()).filter(|&p| super::season_place(w, p, year).0 == d).map(|p| p as u32).collect()
        }
        Tier::Nfl => (0..w.nfl.len() as u32).collect(),
        Tier::Hs => {
            let h = &w.high_schools[team];
            (0..w.high_schools.len())
                .filter(|&k| w.high_schools[k].state == h.state && w.high_schools[k].classification == h.classification)
                .map(|k| k as u32)
                .collect()
        }
    }
}

/// Some teams' squad rows in a season (only their frames are read).
fn rows_for(w: &World, level: Level, season: i32, ids: &[u32]) -> Vec<Sq> {
    if season == w.tally_open_season() {
        return ids.iter().filter_map(|&t| w.tally_squad(level, season, t)).collect();
    }
    let mut keys: Vec<u64> = ids.iter().map(|&t| squads::key(level, t)).collect();
    keys.sort_unstable();
    let mut v = Vec::new();
    let _ = w.tally_get("squads", season, &keys, |_, r| {
        if let Some(s) = tally::file::decode::<Sq>(r) {
            v.push(s);
        }
    });
    v.sort_by_key(|s| s.team);
    v
}

fn group(w: &World, tier: Tier, team: usize, year: i32) -> Group {
    let rows = rows_for(w, level_of(tier), year, &members(w, tier, team, year));
    let lv = level_numbers(&rows);
    Group { rows, lv, name: group_name(w, tier, team, year) }
}

/// A number's rank in its group (1 best) and the group's middle.
fn rank_of(st: &TStat, side: usize, g: &Group, v: f64) -> (Option<usize>, Option<f64>) {
    let vals: Vec<f64> = g.rows.iter().filter_map(|s| (st.f)(s, side, &g.lv)).collect();
    let avg = (!vals.is_empty()).then(|| vals.iter().sum::<f64>() / vals.len() as f64);
    let rank =
        better(st, side).map(|up| 1 + vals.iter().filter(|&&x| if up { x > v + 1e-9 } else { x < v - 1e-9 }).count());
    (rank, avg)
}

pub(super) fn depth_for(w: &World, tier: Tier, team: usize, year: i32) -> Option<u8> {
    let division = (tier == Tier::College).then(|| super::season_place(w, team, year).0);
    super::charting(w, tier, division).map(|c| c.depth)
}

// ---------------------------------------------------------------------
// `team_data`.

fn year_arg(w: &World, args: &Value, years: &[i32], first: Option<i32>) -> i32 {
    args.get("year")
        .and_then(Value::as_i64)
        .map(|y| y as i32)
        .filter(|y| years.contains(y))
        .or(first)
        .or(years.first().copied())
        .unwrap_or(w.year)
}

/// The seasons a team has in the squads tally, newest first, and the one
/// a page opens on: the season being played once it has a game, else the
/// last one counted.
pub(super) fn team_years(w: &World, tier: Tier, team: usize) -> (Vec<i32>, Option<i32>) {
    let open = w.tally_open_season();
    let mut ys: Vec<i32> = w
        .tally_seasons()
        .into_iter()
        .filter(|&y| y == open || w.tally_squad(level_of(tier), y, team as u32).is_some())
        .collect();
    ys.sort_unstable_by(|a, b| b.cmp(a));
    let played = |y: i32| y != open || w.tally_squad(level_of(tier), y, team as u32).is_some();
    let first = ys.iter().copied().find(|&y| played(y));
    (ys, first)
}

fn analytics_rows(st: &[&TStat], side: &[usize], me: &Sq, g: &Group) -> Vec<Value> {
    st.iter()
        .map(|s| {
            let cells: Vec<Value> = side
                .iter()
                .map(|&d| match (s.f)(me, d, &g.lv) {
                    Some(v) => {
                        let (rank, avg) = rank_of(s, d, g, v);
                        json!({ "v": v, "text": text(s.fmt, v), "rank": rank, "avg": avg.map(|a| text(s.fmt, a)) })
                    }
                    None => Value::Null,
                })
                .collect();
            json!({ "key": s.key, "label": s.label, "cells": cells })
        })
        .collect()
}

pub fn team_data(w: &World, args: &Value) -> Value {
    let t = match TeamRef::from_args(w, args) {
        Ok(t) => t,
        Err(e) => return json!({ "error": e }),
    };
    let Some((tier, team)) = tier_of(t) else {
        return json!({ "team": views::team_brief(w, t), "error": "Another league's clubs keep no team data." });
    };
    let (years, first) = team_years(w, tier, team);
    let year = year_arg(w, args, &years, first);
    let depth = depth_for(w, tier, team, year);
    let d = depth.unwrap_or(0);
    let g = group(w, tier, team, year);
    let me = g.rows.iter().find(|s| s.team == team as u32).cloned();
    let mut out = json!({
        "team": views::team_brief(w, t),
        "level": tier_key(tier),
        "year": year,
        "years": years,
        "group": g.name,
        "n": g.rows.len(),
        "depth": depth,
        "charting": depth.map(afm_engine::world::analytics::depth_name),
        "mine": views::is_mine(w, t),
    });
    // One tab's numbers at a time: "analytics" (the default), "tendencies",
    // "games", "roster" or "power".
    let tab = args.get("tab").and_then(Value::as_str).unwrap_or("analytics");
    out["tab"] = json!(tab);
    if let Some(me) = &me {
        out["record"] = json!(format!("{}-{}", me.wins, me.losses));
        out["games"] = json!(me.games);
    }
    if let Some(me) = me.as_ref().filter(|_| tab == "analytics") {
        let shown = |s: &&TStat| s.depth <= d;
        let both: Vec<&TStat> = STATS.iter().filter(|s| s.unit == 0).filter(shown).collect();
        let team_side: Vec<&TStat> = STATS.iter().filter(|s| s.unit == 2 || s.unit == 4).filter(shown).collect();
        let special: Vec<&TStat> = STATS.iter().filter(|s| s.unit == 3).filter(shown).collect();
        out["sides"] = json!([
            { "name": "Offense and defense", "heads": ["Offense", "Defense"], "rows": analytics_rows(&both, &[0, 1], me, &g) },
            { "name": "The whole team", "heads": ["Team"], "rows": analytics_rows(&team_side, &[0], me, &g) },
            { "name": "Special teams", "heads": ["Team"], "rows": analytics_rows(&special, &[0], me, &g) },
        ]);
    }
    match tab {
        "tendencies" => {
            if let Some(me) = me.as_ref().filter(|_| d >= 1 && tier != Tier::Hs) {
                out["tendencies"] = tendencies(w, tier, team, year, me, &g, d);
            }
        }
        "games" => out["games_list"] = games_list(w, tier, team, year, d),
        "roster" => out["back"] = back_json(w, tier, team, year),
        "power" => out["power"] = team_power(w, tier, team, year),
        _ => {}
    }
    out
}

// ---------------------------------------------------------------------
// Tendencies (item 16).

fn tend_rows(me: &Sq, g: &Group) -> Vec<Value> {
    (0..squads::TEND.len() as u8)
        .filter_map(|c| {
            let [n, db, x] = me.tend_of(c);
            if n < 1.0 {
                return None;
            }
            let lvl = g.rows.iter().fold([0f32; 3], |a, s| {
                let v = s.tend_of(c);
                [a[0] + v[0], a[1] + v[1], a[2] + v[2]]
            });
            let pr = db as f64 / n as f64;
            let xr = x as f64 / n as f64;
            Some(json!({
                "label": squads::TEND[c as usize],
                "snaps": n as u32,
                "pass": text(Fmt::Pct, pr),
                "expected": text(Fmt::Pct, xr),
                "over": format!("{:+.1}", 100.0 * (pr - xr)),
                "level": (lvl[0] > 0.0).then(|| text(Fmt::Pct, lvl[1] as f64 / lvl[0] as f64)),
            }))
        })
        .collect()
}

/// A cell's plays summed over its kinds (`o|personnel|11|run` and `|pass`).
fn cells_by(t: &teams::Season, prefix: &str) -> Vec<(String, [f32; 8], [f32; 8])> {
    let mut m: std::collections::BTreeMap<String, ([f32; 8], [f32; 8])> = Default::default();
    for (k, v) in &t.cells {
        let Some(rest) = k.strip_prefix(prefix) else { continue };
        let (name, kind) = match rest.rsplit_once('|') {
            Some((n, k)) => (n.to_string(), k),
            None => (rest.to_string(), ""),
        };
        let e = m.entry(name).or_default();
        let into = if kind == "pass" { &mut e.1 } else { &mut e.0 };
        for i in 0..8 {
            into[i] += v[i];
        }
    }
    m.into_iter().map(|(k, (r, p))| (k, r, p)).collect()
}

fn share_rows(t: &teams::Season, prefix: &str, total: f64, order: &[&str]) -> Vec<Value> {
    let mut rows: Vec<(String, f64, f64, f64)> = cells_by(t, prefix)
        .into_iter()
        .map(|(k, r, p)| {
            let n = (r[0] + p[0]) as f64;
            (k, n, p[0] as f64, (r[2] + p[2]) as f64)
        })
        .filter(|x| x.1 > 0.0)
        .collect();
    rows.sort_by(|a, b| {
        let at = |k: &str| order.iter().position(|o| *o == k).unwrap_or(usize::MAX);
        at(&a.0).cmp(&at(&b.0)).then(b.1.total_cmp(&a.1))
    });
    rows.into_iter()
        .map(|(k, n, pass, epa)| {
            json!({
                "label": k,
                "snaps": n as u32,
                "share": text(Fmt::Pct, n / total.max(1.0)),
                "pass": text(Fmt::Pct, pass / n),
                "epa": text(Fmt::Signed, epa / n),
            })
        })
        .collect()
}

fn flag_rate(t: &teams::Season, yes: &str, no: &str) -> Option<String> {
    let (a, b) = (t.get(yes)[0] as f64, t.get(no)[0] as f64);
    (a + b > 0.0).then(|| text(Fmt::Pct, a / (a + b)))
}

fn tendencies(w: &World, tier: Tier, team: usize, year: i32, me: &Sq, g: &Group, depth: u8) -> Value {
    let mut out = json!({ "calls": tend_rows(me, g) });
    if depth < 2 {
        return out;
    }
    let Some(t) = w.tally_team(level_of(tier), year, team as u32) else { return out };
    let snaps = (t.get("o|kind|run")[0] + t.get("o|kind|pass")[0]) as f64;
    let dsnaps = (t.get("d|kind|run")[0] + t.get("d|kind|pass")[0]) as f64;
    let db = t.get("o|kind|pass")[0] as f64;
    let motion: f64 =
        cells_by(&t, "o|motion|").iter().filter(|x| x.0 != "none").map(|x| (x.1[0] + x.2[0]) as f64).sum();
    let rpo = (t.get("o|rpo|run")[0] + t.get("o|rpo|pass")[0]) as f64;
    let hurry = (t.get("o|tempo|no huddle|run")[0] + t.get("o|tempo|no huddle|pass")[0]) as f64;
    out["offense"] = json!({
        "personnel": share_rows(&t, "o|personnel|", snaps, &[]),
        "backfield": share_rows(&t, "o|backfield|", snaps, &["gun", "pistol", "under center", "other"]),
        "rates": [
            { "key": "TmPa", "label": "Play action", "v": flag_rate(&t, "o|play action|true", "o|play action|false") },
            { "key": "TmMotion", "label": "Motion or a shift", "v": (snaps > 0.0).then(|| text(Fmt::Pct, motion / snaps)) },
            { "key": "TmRpo", "label": "Run-pass options", "v": (snaps > 0.0).then(|| text(Fmt::Pct, rpo / snaps)) },
            { "key": "TmNoHuddle", "label": "No huddle", "v": (snaps > 0.0).then(|| text(Fmt::Pct, hurry / snaps)) },
            { "key": "TmPressed", "label": "Dropbacks pressured", "v": flag_rate(&t, "o|pressured|true", "o|pressured|false") },
            { "key": "TmBoxSeen", "label": "Runs into a loaded box", "v": loaded(&t, "o") },
        ],
        "dropbacks": db as u32,
    });
    // The defense's calls: the coverage named first ("Cover 3, four-man rush").
    let mut cov: HashMap<String, f64> = HashMap::new();
    for (k, v) in &t.cells {
        if let Some(rest) = k.strip_prefix("d|defense|")
            && let Some((call, kind)) = rest.rsplit_once('|')
            && kind == "pass"
        {
            let name = call.split(", ").next().unwrap_or(call).to_string();
            *cov.entry(name).or_default() += v[0] as f64;
        }
    }
    let ddb: f64 = cov.values().sum();
    let high = |name: &str| match name {
        "Cover 0" => 0,
        "Cover 1" | "Cover 3" | "Cover 1 robber" | "Cover 3 match" => 1,
        _ => 2,
    };
    let shells: Vec<Value> = (0..3)
        .map(|h| {
            let n: f64 = cov.iter().filter(|(k, _)| high(k) == h).map(|(_, v)| v).sum();
            let label = ["No deep safety", "One high safety", "Two high safeties"][h];
            json!({ "label": label, "share": (ddb > 0.0).then(|| text(Fmt::Pct, n / ddb)) })
        })
        .collect();
    let mut calls: Vec<(String, f64)> = cov.into_iter().collect();
    calls.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let blitz: Vec<Value> = (1..=4)
        .map(|dn| {
            let label = ["1st down", "2nd down", "3rd down", "4th down"][dn - 1];
            json!({ "label": label, "v": flag_rate(&t, &format!("d|blitz|{dn}|true"), &format!("d|blitz|{dn}|false")) })
        })
        .collect();
    out["defense"] = json!({
        "shells": shells,
        "coverages": calls.iter().map(|(k, n)| json!({ "label": k, "share": text(Fmt::Pct, n / ddb.max(1.0)) })).collect::<Vec<_>>(),
        "man": flag_rate(&t, "d|coverage|man", "d|coverage|zone"),
        "blitz": blitz,
        "packages": share_rows(&t, "d|package|", dsnaps, &["base", "nickel", "dime", "goal line"]),
        "boxes": box_rows(&t),
        "rates": [
            { "key": "TmBoxLoaded", "label": "Runs met with a loaded box", "v": loaded(&t, "d") },
            { "key": "TmPress", "label": "Dropbacks it pressured", "v": flag_rate(&t, "d|pressured|true", "d|pressured|false") },
        ],
    });
    out
}

/// The share of runs against eight or more in the box.
fn loaded(t: &teams::Season, side: &str) -> Option<String> {
    let mut all = 0.0;
    let mut eight = 0.0;
    for (k, v) in &t.cells {
        if let Some(rest) = k.strip_prefix(&format!("{side}|box|"))
            && let Some(n) = rest.strip_suffix("|run")
        {
            all += v[0] as f64;
            if n.parse::<u8>().is_ok_and(|n| n >= 8) {
                eight += v[0] as f64;
            }
        }
    }
    (all > 0.0).then(|| text(Fmt::Pct, eight / all))
}

fn box_rows(t: &teams::Season) -> Vec<Value> {
    let mut by: std::collections::BTreeMap<u8, [f64; 3]> = Default::default();
    for (k, v) in &t.cells {
        if let Some(rest) = k.strip_prefix("d|box|")
            && let Some((n, kind)) = rest.rsplit_once('|')
            && let Ok(n) = n.parse::<u8>()
        {
            let e = by.entry(n).or_default();
            e[0] += v[0] as f64;
            if kind == "run" {
                e[1] += v[0] as f64;
                e[2] += v[1] as f64;
            }
        }
    }
    let all: f64 = by.values().map(|x| x[0]).sum();
    by.into_iter()
        .map(|(n, x)| {
            json!({
                "label": format!("{n} in the box"),
                "share": text(Fmt::Pct, x[0] / all.max(1.0)),
                "runs": x[1] as u32,
                "ypc": (x[1] > 0.0).then(|| text(Fmt::One, x[2] / x[1])),
            })
        })
        .collect()
}

// ---------------------------------------------------------------------
// Games, unit grades and snap shares (item 21).

fn games_list(w: &World, tier: Tier, team: usize, year: i32, depth: u8) -> Value {
    let level = level_of(tier);
    let t = team as u32;
    let mut rows = Vec::new();
    // Snaps by man: (id, side 0 offense / 1 defense) -> snaps a game.
    let mut snaps: HashMap<(u32, usize), Vec<(usize, u32)>> = HashMap::new();
    let mut plays: Vec<[u32; 2]> = Vec::new();
    w.tally_games(year, None, |_, g| {
        if g.level != level as u8 || !g.teams.contains(&t) {
            return;
        }
        let side = if g.teams[0] == t { 0 } else { 1 };
        let opp = g.teams[1 - side] as usize;
        let score = [g.score[side], g.score[1 - side]];
        let k = rows.len();
        let mut grade = [(0.0, 0u32); 2];
        for l in g.lines.iter().filter(|l| l.side as usize == side) {
            let (o, d) = (l.get(Stat::SnapsOff) as u32, l.get(Stat::SnapsDef) as u32);
            let unit = if o >= d { 0 } else { 1 };
            grade[unit].0 += l.get(Stat::GradeSum);
            grade[unit].1 += l.get(Stat::Graded) as u32;
            for (u, n) in [(0, o), (1, d)] {
                if n > 0 {
                    snaps.entry((l.id, u)).or_default().push((k, n));
                }
            }
        }
        let gr = |u: usize| {
            (depth >= 2 && grade[u].1 > 0).then(|| {
                let v = StatLine { grade_sum: grade[u].0, graded: grade[u].1, ..Default::default() }.grade();
                format!("{v:.1}")
            })
        };
        let own = &g.boxes[side];
        let them = &g.boxes[1 - side];
        plays.push([own.plays, them.plays]);
        let opp_ref = super::team_ref(w, tier, opp);
        rows.push(json!({
            "date": views::date_text(g.date()),
            "kind": g.kind,
            "home": side == 0 && !g.neutral,
            "neutral": g.neutral,
            "opp": opp_ref.map(|r| views::team_brief(w, r)),
            "result": format!("{} {}-{}", if score[0] > score[1] { "W" } else if score[0] < score[1] { "L" } else { "T" }, score[0], score[1]),
            "yards": [own.yards.round() as i64, them.yards.round() as i64],
            "epa": (depth >= 1 && tier != Tier::Hs).then(|| [text(Fmt::Signed, own.epa / own.plays.max(1) as f64), text(Fmt::Signed, them.epa / them.plays.max(1) as f64)]),
            "grades": [gr(0), gr(1)],
        }));
    });
    // Snap shares: each unit's men by snaps, a share of each game's plays.
    let men = |u: usize| -> Vec<Value> {
        type Man<'a> = (u32, u32, &'a Vec<(usize, u32)>);
        let mut list: Vec<Man> =
            snaps.iter().filter(|(k, _)| k.1 == u).map(|(k, v)| (k.0, v.iter().map(|x| x.1).sum::<u32>(), v)).collect();
        list.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        list.into_iter()
            .take(26)
            .map(|(id, total, v)| {
                let mut cells: Vec<Value> = vec![Value::Null; plays.len()];
                for &(k, n) in v {
                    let of = plays[k][u].max(1);
                    cells[k] = json!((100 * n / of).min(100));
                }
                let who = views::person_index(w, id);
                json!({
                    "id": who.map(|_| id),
                    "name": who.map(|k| w.people[k].name.clone()).unwrap_or_default(),
                    "pos": who.map(|k| w.people[k].player.position.name()),
                    "snaps": total,
                    "games": cells,
                })
            })
            .collect()
    };
    let shares =
        if tier == Tier::Hs || depth == 0 { Value::Null } else { json!({ "offense": men(0), "defense": men(1) }) };
    json!({ "rows": rows, "snaps": shares, "graded": depth >= 2 })
}

// ---------------------------------------------------------------------
// What came back and what the roster brought (item 22).

/// Bill Connelly's returning production, his weights where his write-ups
/// give them (receiving 23.5% of the offense; passes defensed 33%, tackles
/// 19% and tackles for loss 15% of the defense), the rest proposed, not
/// confirmed: passing 42%, rushing 13% and the offense's starts 21.5%; the
/// defense's starts 33%.
const BACK_O: [(Stat, f64); 3] = [(Stat::PassYds, 0.42), (Stat::RushYds, 0.13), (Stat::RecYds, 0.235)];
const BACK_O_STARTS: f64 = 0.215;
const BACK_D: [(Stat, f64); 3] = [(Stat::Pbus, 0.33), (Stat::Tackles, 0.19), (Stat::Tfl, 0.15)];
const BACK_D_STARTS: f64 = 0.33;

/// Who is on a team in a season: for the season being played its roster
/// today, for a closed one every man with a stat book line or a start.
fn on_team(w: &World, tier: Tier, team: usize, year: i32, sq: Option<&Sq>) -> HashSet<u32> {
    let book: Vec<u32> = match tier {
        Tier::College => w.stats.year(year).iter().filter(|r| r.program as usize == team).map(|r| r.id).collect(),
        _ => Vec::new(),
    };
    on_team_with(w, tier, team, year, sq, &book)
}

/// `on_team` with the stat book's men already found.
fn on_team_with(w: &World, tier: Tier, team: usize, year: i32, sq: Option<&Sq>, book: &[u32]) -> HashSet<u32> {
    let mut ids: HashSet<u32> = book.iter().copied().collect();
    if year == w.tally_open_season() {
        let roster: Vec<usize> = match tier {
            Tier::College => w.programs[team].roster.clone(),
            Tier::Hs => w.high_schools[team].roster.clone(),
            Tier::Nfl => (0..w.people.len())
                .filter(|&k| w.people[k].pro.as_ref().is_some_and(|p| p.team == Some(team)))
                .collect(),
        };
        ids.extend(roster.iter().filter_map(|&k| w.people.get(k)).map(|p| p.player.id));
    }
    if let Some(s) = sq {
        ids.extend(s.starters.iter().map(|x| x.0));
    }
    ids
}

/// The share of last season's starts on each side made by men back.
fn starts_back(last: &Sq, back: &HashSet<u32>) -> [f64; 2] {
    let mut v = [[0.0f64; 2]; 2];
    for &(id, o, d) in &last.starters {
        let b = back.contains(&id) as u8 as f64;
        v[0][0] += o as f64;
        v[0][1] += o as f64 * b;
        v[1][0] += d as f64;
        v[1][1] += d as f64 * b;
    }
    [v[0][1] / v[0][0].max(1.0), v[1][1] / v[1][0].max(1.0)]
}

/// A college team's production back: each stat's share, then the weighted
/// offense and defense.
fn production_back(
    w: &World,
    team: usize,
    year: i32,
    back: &HashSet<u32>,
    starts: [f64; 2],
) -> (Vec<(Stat, f64)>, [f64; 2]) {
    let rows: Vec<&SeasonRecord> = w.stats.year(year - 1).iter().filter(|r| r.program as usize == team).collect();
    production_of(&rows, back, starts)
}

fn production_of(rows: &[&SeasonRecord], back: &HashSet<u32>, starts: [f64; 2]) -> (Vec<(Stat, f64)>, [f64; 2]) {
    let mut tot: HashMap<Stat, [f64; 2]> = HashMap::new();
    for r in rows {
        let b = back.contains(&r.id);
        for (s, _) in BACK_O.iter().chain(BACK_D.iter()) {
            let v = r.get(*s).max(0.0);
            let e = tot.entry(*s).or_default();
            e[0] += v;
            if b {
                e[1] += v;
            }
        }
    }
    let share = |s: Stat| tot.get(&s).map_or(0.0, |x| if x[0] > 0.0 { x[1] / x[0] } else { 0.0 });
    let o = BACK_O.iter().map(|(s, k)| k * share(*s)).sum::<f64>() + BACK_O_STARTS * starts[0];
    let d = BACK_D.iter().map(|(s, k)| k * share(*s)).sum::<f64>() + BACK_D_STARTS * starts[1];
    (BACK_O.iter().chain(BACK_D.iter()).map(|(s, _)| (*s, share(*s))).collect(), [o, d])
}

/// What each team at a level brought back into a season: (offense,
/// defense) shares, as the projection reads them.
fn backs(w: &World, tier: Tier, year: i32) -> HashMap<u32, [f64; 2]> {
    let level = level_of(tier);
    let last = pool(w, level, year - 1);
    let now = pool(w, level, year);
    let by: HashMap<u32, &Sq> = now.iter().map(|s| (s.team, s)).collect();
    // The stat book's men by program, this season and last, found once.
    let mut book_now: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut book_last: HashMap<u32, Vec<&SeasonRecord>> = HashMap::new();
    if tier == Tier::College {
        for r in w.stats.year(year) {
            book_now.entry(r.program as u32).or_default().push(r.id);
        }
        for r in w.stats.year(year - 1) {
            book_last.entry(r.program as u32).or_default().push(r);
        }
    }
    let mut out = HashMap::new();
    for l in last.iter() {
        let none = Vec::new();
        let book = book_now.get(&l.team).unwrap_or(&none);
        let ids = on_team_with(w, tier, l.team as usize, year, by.get(&l.team).copied(), book);
        let starts = starts_back(l, &ids);
        let v = if tier == Tier::College {
            production_of(book_last.get(&l.team).map_or(&[][..], |v| v.as_slice()), &ids, starts).1
        } else {
            starts
        };
        out.insert(l.team, v);
    }
    out
}

fn stat_label(s: Stat) -> &'static str {
    match s {
        Stat::PassYds => "Passing yards",
        Stat::RushYds => "Rushing yards",
        Stat::RecYds => "Receiving yards",
        Stat::Pbus => "Passes broken up",
        Stat::Tackles => "Tackles",
        Stat::Tfl => "Tackles for loss",
        _ => "",
    }
}

/// The roster's recruiting stars now (the season being played): its men
/// who have taken a snap, else (before the first game) the whole roster.
fn stars_now(w: &World, team: usize) -> Vec<f32> {
    let p = &w.programs[team];
    let mut v = [0f32; 7];
    for &k in &p.roster {
        let Some(m) = w.people.get(k) else { continue };
        let s = m.stars as f32;
        let def = matches!(m.player.position.index(), 6..=10);
        v[0] += s;
        v[3] += 1.0;
        v[if def { 2 } else { 1 }] += s;
        v[if def { 5 } else { 4 }] += 1.0;
        v[6] += (s >= 4.0) as u8 as f32;
    }
    let avg = |a: f32, n: f32| if n > 0.0 { a / n } else { 0.0 };
    vec![avg(v[0], v[3]), avg(v[1], v[4]), avg(v[2], v[5]), v[3], v[4], v[5], avg(v[6], v[3])]
}

fn talent_of(w: &World, tier: Tier, team: usize, year: i32, sq: Option<&Sq>) -> Option<Vec<f32>> {
    if tier != Tier::College {
        return sq.filter(|s| !s.talent.is_empty()).map(|s| s.talent.clone());
    }
    match sq.filter(|s| !s.talent.is_empty()) {
        Some(s) => Some(s.talent.clone()),
        None if year == w.tally_open_season() => Some(stars_now(w, team)),
        None => None,
    }
}

fn back_json(w: &World, tier: Tier, team: usize, year: i32) -> Value {
    let level = level_of(tier);
    let last = w.tally_squad(level, year - 1, team as u32);
    let now = w.tally_squad(level, year, team as u32);
    let mut out = json!({});
    if let Some(l) = &last {
        let ids = on_team(w, tier, team, year, now.as_ref());
        let starts = starts_back(l, &ids);
        let regulars = |u: usize| l.regulars(u);
        let back_reg = |u: usize| regulars(u).iter().filter(|id| ids.contains(id)).count();
        out["starters"] = json!({
            "offense": [back_reg(0), regulars(0).len()],
            "defense": [back_reg(1), regulars(1).len()],
            "starts": [text(Fmt::Pct, starts[0]), text(Fmt::Pct, starts[1])],
        });
        if tier == Tier::College {
            let (by, od) = production_back(w, team, year, &ids, starts);
            out["production"] = json!({
                "offense": text(Fmt::Pct, od[0]),
                "defense": text(Fmt::Pct, od[1]),
                "overall": text(Fmt::Pct, (od[0] + od[1]) / 2.0),
                "stats": by.iter().map(|(s, v)| json!({ "label": stat_label(*s), "share": text(Fmt::Pct, *v) })).collect::<Vec<_>>(),
            });
        }
        if tier == Tier::Nfl {
            out["snaps"] = nfl_snaps_back(w, team, year, &ids);
        }
    }
    let sq = now.as_ref();
    if let Some(t) = talent_of(w, tier, team, year, sq) {
        out["talent"] = match tier {
            Tier::College => json!({
                "stars": format!("{:.2}", t[0]),
                "offense": format!("{:.2}", t.get(1).copied().unwrap_or(0.0)),
                "defense": format!("{:.2}", t.get(2).copied().unwrap_or(0.0)),
                "men": t.get(3).copied().unwrap_or(0.0) as u32,
                "blue_chips": t.get(6).map(|v| text(Fmt::Pct, *v as f64)),
                "rank": talent_rank(w, tier, team, year, t[0]),
            }),
            _ => json!({
                "starters": [t[0] as u32, t[1] as u32],
                "seniors": [t[2] as u32, t[3] as u32],
            }),
        };
    } else if tier == Tier::Hs && year == w.tally_open_season() {
        // The season being played: its seniors today.
        let h = &w.high_schools[team];
        let seniors = h.roster.iter().filter_map(|&k| w.people.get(k)).filter(|p| p.class_of == year + 1).count();
        out["seniors_now"] = json!(seniors);
    }
    out
}

fn talent_rank(w: &World, tier: Tier, team: usize, year: i32, mine: f32) -> Value {
    let g = group(w, tier, team, year);
    let open = year == w.tally_open_season();
    let vals: Vec<f32> = g
        .rows
        .iter()
        .filter_map(|s| if open { Some(stars_now(w, s.team as usize)[0]) } else { s.talent.first().copied() })
        .collect();
    json!([1 + vals.iter().filter(|&&v| v > mine + 1e-6).count(), vals.len()])
}

/// An NFL club's snaps back: the share of last season's offensive and
/// defensive snaps played by men on the club this season.
fn nfl_snaps_back(w: &World, team: usize, year: i32, back: &HashSet<u32>) -> Value {
    #[derive(serde::Deserialize, Default)]
    #[serde(default)]
    struct P {
        level: u8,
        team: u32,
        snaps: [u32; 3],
        id: u32,
    }
    let mut v = [[0.0f64; 2]; 2];
    let mut add = |p: &P| {
        if p.level == Level::Nfl as u8 && p.team == team as u32 {
            let b = back.contains(&p.id) as u8 as f64;
            for (u, x) in v.iter_mut().enumerate() {
                x[0] += p.snaps[u] as f64;
                x[1] += p.snaps[u] as f64 * b;
            }
        }
    };
    if year - 1 == w.tally_open_season() {
        w.tally_players(year - 1, |s| add(&P { level: s.level, team: s.team, snaps: s.snaps, id: s.id }));
    } else {
        w.tally_players_as::<P>(year - 1, |_, p| add(&p));
    }
    json!([text(Fmt::Pct, v[0][1] / v[0][0].max(1.0)), text(Fmt::Pct, v[1][1] / v[1][0].max(1.0))])
}

// ---------------------------------------------------------------------
// The power rating, projection and odds (item 23).

/// A level's season as the power page reads it.
struct PowerSeason {
    season: i32,
    games: Vec<SGame>,
    fit: power::Fit,
    sigma: f64,
    preseason: HashMap<u32, Rating>,
    odds: HashMap<u32, Odds>,
}

/// The projection's inputs for each team of a level going into `year`.
fn inputs(w: &World, tier: Tier, year: i32) -> HashMap<u32, Inputs> {
    let level = level_of(tier);
    let last = power::season_ratings(w, level, year - 1);
    let before = power::season_ratings(w, level, year - 2);
    let back = backs(w, tier, year);
    // Stars against the level's average (college).
    let mut stars: HashMap<u32, [f64; 2]> = HashMap::new();
    if tier == Tier::College {
        let open = year == w.tally_open_season();
        let now = pool(w, level, year);
        let mut v: Vec<(u32, [f64; 2])> = if open {
            (0..w.programs.len())
                .map(|p| (p as u32, stars_now(w, p)))
                .map(|(p, t)| (p, [t[1] as f64, t[2] as f64]))
                .collect()
        } else {
            now.iter()
                .filter(|s| s.talent.len() >= 3)
                .map(|s| (s.team, [s.talent[1] as f64, s.talent[2] as f64]))
                .collect()
        };
        v.retain(|x| x.1[0] > 0.0 || x.1[1] > 0.0);
        if !v.is_empty() {
            let n = v.len() as f64;
            let m = [v.iter().map(|x| x.1[0]).sum::<f64>() / n, v.iter().map(|x| x.1[1]).sum::<f64>() / n];
            for (t, s) in v {
                stars.insert(t, [s[0] - m[0], s[1] - m[1]]);
            }
        }
    }
    let mut ids: HashSet<u32> = last.keys().copied().collect();
    ids.extend(back.keys().copied());
    ids.extend(stars.keys().copied());
    ids.into_iter()
        .map(|t| {
            (
                t,
                Inputs {
                    last: last.get(&t).copied(),
                    before: before.get(&t).copied(),
                    back: back.get(&t).copied(),
                    stars: stars.get(&t).copied(),
                },
            )
        })
        .collect()
}

/// The projection's weights, fitted on the world's own closed seasons
/// (the last ten).
fn weights(w: &World, tier: Tier) -> Arc<[power::Weights; 2]> {
    let open = w.tally_open_season();
    kept(&FITS, (world_key(w), level_of(tier) as u8, open), || {
        let closed: Vec<i32> = w.tally_seasons().into_iter().filter(|&y| y < open).collect();
        let mut rows = Vec::new();
        for &y in closed.iter().rev().take(10) {
            if !closed.contains(&(y - 1)) {
                continue;
            }
            let x = inputs(w, tier, y);
            let r = power::season_ratings(w, level_of(tier), y);
            for (t, inp) in x {
                if let Some(r) = r.get(&t)
                    && inp.last.is_some()
                {
                    rows.push((inp, *r));
                }
            }
        }
        power::fit_projection(&rows)
    })
}

fn power_season(w: &World, tier: Tier, state: Option<usize>) -> Arc<PowerSeason> {
    let level = level_of(tier);
    let key_day = w.today.0;
    let st = state.map_or(0, |s| s as i32 + 1);
    let make = || {
        let (season, games) = power::this_season(w, level, state);
        let wts = weights(w, tier);
        let pre: HashMap<u32, Rating> =
            inputs(w, tier, season).into_iter().map(|(t, x)| (t, power::project(&wts, &x))).collect();
        let fit = power::solve(&games, &pre, power::PRIOR_GAMES);
        // The spread of results around the ratings, fitted on last season.
        let last = power::tallied_games(w, season - 1);
        let lg: Vec<SGame> = last.iter().filter(|x| x.0 == level as u8).map(|x| x.2).collect();
        let (sigma, n) = power::sigma(&lg, &HashMap::new(), power::PRIOR_GAMES);
        let sigma = if n >= 30 { sigma } else { power::sigma(&games, &pre, power::PRIOR_GAMES).0 };
        let field = power::field_of(w, level);
        let odds = power::odds(&games, &fit, sigma, &field, season, &w.rules, season as u32 ^ 0x5eed);
        PowerSeason { season, games, fit, sigma, preseason: pre, odds }
    };
    kept(&POWER, (world_key(w) ^ (key_day as usize) << 8 ^ st as usize, level as u8, w.tally_open_season()), make)
}

fn rating_json(r: &Rating) -> Value {
    json!({ "overall": format!("{:+.1}", r.overall()), "off": format!("{:+.1}", r.off), "def": format!("{:+.1}", r.def), "v": r.overall() })
}

fn odds_json(o: &Odds) -> Value {
    let p = |v: f64| format!("{:.0}%", 100.0 * v);
    json!({
        "wins": format!("{:.1}", o.wins),
        "losses": format!("{:.1}", o.losses),
        "bowl": p(o.bowl),
        "league": p(o.league),
        "playoff": p(o.playoff),
        "bye": p(o.bye),
        "winning": p(o.winning),
    })
}

/// A team's power block: its rating and rank now, its preseason
/// projection, the games to come with its chance in each, and its odds.
fn team_power(w: &World, tier: Tier, team: usize, year: i32) -> Value {
    if year != w.tally_open_season() {
        // A closed season: its closing rating.
        let r = power::season_ratings(w, level_of(tier), year);
        let Some(me) = r.get(&(team as u32)) else { return Value::Null };
        // Ranked in its group, as every other number on the page.
        let group: HashSet<u32> = members(w, tier, team, year).into_iter().collect();
        let pool: Vec<&Rating> = r.iter().filter(|x| group.contains(x.0)).map(|x| x.1).collect();
        let rank = 1 + pool.iter().filter(|x| x.overall() > me.overall() + 1e-9).count();
        return json!({ "closing": rating_json(me), "rank": [rank, pool.len()] });
    }
    let state = (tier == Tier::Hs).then(|| w.high_schools[team].state);
    let ps = power_season(w, tier, state);
    let t = team as u32;
    let me = ps.fit.get(t);
    let group: HashSet<u32> = members(w, tier, team, year).into_iter().collect();
    let pool: Vec<f64> = ps.fit.teams.iter().filter(|x| group.contains(x.0)).map(|x| x.1.overall()).collect();
    let rank = 1 + pool.iter().filter(|&&x| x > me.overall() + 1e-9).count();
    let coming: Vec<Value> = ps
        .games
        .iter()
        .filter(|g| !g.played() && g.teams.contains(&t))
        .map(|g| {
            let side = if g.teams[0] == t { 0 } else { 1 };
            let m = ps.fit.margin(g.teams[0], g.teams[1], g.neutral);
            let p = power::win_chance(m, ps.sigma);
            let p = if side == 0 { p } else { 1.0 - p };
            let opp = super::team_ref(w, tier, g.teams[1 - side] as usize);
            json!({
                "date": views::date_text(afm_engine::calendar::Date(g.date)),
                "opp": opp.map(|r| views::team_brief(w, r)),
                "home": side == 0 && !g.neutral,
                "neutral": g.neutral,
                "chance": format!("{:.0}%", 100.0 * p),
                "margin": format!("{:+.1}", if side == 0 { m } else { -m }),
            })
        })
        .collect();
    json!({
        "season": ps.season,
        "now": rating_json(&me),
        "games": me.games,
        "rank": [rank, pool.len()],
        "preseason": ps.preseason.get(&t).map(rating_json),
        "home": format!("{:.1}", ps.fit.home),
        "sigma": format!("{:.1}", ps.sigma),
        "coming": coming,
        "odds": ps.odds.get(&t).map(odds_json),
    })
}

// ---------------------------------------------------------------------
// `team_seasons` (item 18, board 44).

/// The numbers each unit's season row shows (offense, defense, special
/// teams), as in board 44.
const BY_UNIT: [&[&str]; 3] = [
    &["TmPtsG", "TmYpp", "TmYdsG", "TmToG", "TmSackRate", "TmThird", "TmEpa"],
    &["TmPtsG", "TmYpp", "TmYdsG", "TmToG", "TmSackRate", "TmThird", "TmEpa"],
    &["TmFgPct", "TmXpPct", "TmPuntNet", "TmPuntIn20", "TmKoTb", "TmKrAvg", "TmPrAvg"],
];

fn stat_by(key: &str) -> Option<&'static TStat> {
    STATS.iter().find(|s| s.key == key)
}

pub fn team_seasons(w: &World, args: &Value) -> Value {
    let t = match TeamRef::from_args(w, args) {
        Ok(t) => t,
        Err(e) => return json!({ "error": e }),
    };
    let Some((tier, team)) = tier_of(t) else { return json!({ "error": "Another league's clubs keep no team data." }) };
    let unit = args.get("unit").and_then(Value::as_u64).map_or(1, |u| u.min(2) as usize);
    let side = if unit == 1 { 1 } else { 0 };
    let level = level_of(tier);
    let (years, _) = team_years(w, tier, team);
    // No world keeps a special teams coordinator: that view shows the head
    // coach only.
    let role = match unit {
        0 => Some(Role::OffensiveCoordinator),
        1 => Some(Role::DefensiveCoordinator),
        _ => None,
    };
    let cols: Vec<&TStat> = BY_UNIT[unit].iter().filter_map(|k| stat_by(k)).collect();
    let mut rows = Vec::new();
    for &y in &years {
        let g = group(w, tier, team, y);
        let Some(me) = g.rows.iter().find(|s| s.team == team as u32) else { continue };
        let d = depth_for(w, tier, team, y).unwrap_or(0);
        let charge = Charge::new(w, y);
        let coach = |r: Role| charge.get(team_of(tier, team), r).map(|c| json!({ "id": c, "name": w.coaches[c].name }));
        let cells: Vec<Value> = cols
            .iter()
            .map(|s| {
                if s.depth > d {
                    return Value::Null;
                }
                let sd = if s.unit == 0 { side } else { 0 };
                match (s.f)(me, sd, &g.lv) {
                    Some(v) => {
                        let (rank, _) = rank_of(s, sd, &g, v);
                        json!({ "text": text(s.fmt, v), "rank": rank, "total": total_of(s, me, sd) })
                    }
                    None => Value::Null,
                }
            })
            .collect();
        let last = w.tally_squad(level, y - 1, team as u32);
        let back = last.as_ref().map(|l| {
            let ids = on_team(w, tier, team, y, Some(me));
            let u = if unit == 2 { 0 } else { unit };
            let reg = l.regulars(u);
            json!([reg.iter().filter(|id| ids.contains(id)).count(), reg.len()])
        });
        let talent = talent_of(w, tier, team, y, Some(me)).map(|t| match tier {
            Tier::College => json!(format!("{:.2}", t.get(1 + side).copied().unwrap_or(t[0]))),
            Tier::Hs => json!(t.get(2 + side).copied().unwrap_or(0.0) as u32),
            Tier::Nfl => Value::Null,
        });
        rows.push(json!({
            "year": y,
            "record": format!("{}-{}", me.wins, me.losses),
            "head": coach(Role::HeadCoach),
            "coordinator": role.and_then(coach),
            "cells": cells,
            "talent": talent,
            "back": back,
            "n": g.rows.len(),
            "group": g.name,
        }));
    }
    json!({
        "team": views::team_brief(w, t),
        "level": tier_key(tier),
        "unit": unit,
        "units": ["Offense", "Defense", "Special teams"],
        "role": role.map(|r| r.name()),
        "cols": cols.iter().map(|s| json!({ "key": s.key, "label": s.label })).collect::<Vec<_>>(),
        "talent": match tier {
            Tier::College => Some(if unit == 0 { "Stars on O" } else if unit == 1 { "Stars on D" } else { "Stars" }),
            Tier::Hs => Some(if unit == 0 { "Seniors on O" } else { "Seniors on D" }),
            Tier::Nfl => None,
        },
        "rows": rows,
    })
}

/// A rate's season total, for the Totals view (counts only).
fn total_of(s: &TStat, me: &Sq, side: usize) -> Option<String> {
    let v = match s.key {
        "TmPtsG" => bx(me, side, "points"),
        "TmYdsG" => bx(me, side, "yards"),
        "TmPassG" => bx(me, side, "pass_yards"),
        "TmRushG" => bx(me, side, "rush_yards"),
        "TmToG" => bx(me, side, "turnovers"),
        "TmSackRate" => bx(me, side, "sacks"),
        "TmThird" => {
            return Some(format!("{}-{}", bx(me, side, "third_conv") as u32, bx(me, side, "third_att") as u32));
        }
        "TmFgPct" => return Some(format!("{}-{}", me.st_of("fg_made") as u32, me.st_of("fg_att") as u32)),
        "TmXpPct" => return Some(format!("{}-{}", me.st_of("xp_made") as u32, me.st_of("xp_att") as u32)),
        "TmThreeOut" => adv(me, side).three_outs as f64,
        _ => return None,
    };
    Some(super::fmt_num(v))
}

// ---------------------------------------------------------------------
// `team_leaders` (items 19, 20 and 23).

pub fn team_leaders(w: &World, args: &Value) -> Value {
    let tier = tier_arg(w, args);
    let level = level_of(tier);
    let view = args.get("view").and_then(Value::as_str).unwrap_or("table");
    let division = super::division_arg(w, args);
    let state = super::state_arg(w, args);
    let mut years: Vec<i32> = w.tally_seasons();
    years.sort_unstable_by(|a, b| b.cmp(a));
    // The newest season with games at the level.
    let open = w.tally_open_season();
    let first = years.iter().copied().find(|&y| y != open || !pool(w, level, y).is_empty());
    let year = year_arg(w, args, &years, first);
    let ids: Vec<u32> = match tier {
        Tier::College => {
            (0..w.programs.len()).filter(|&p| super::season_place(w, p, year).0 == division).map(|p| p as u32).collect()
        }
        Tier::Nfl => (0..w.nfl.len() as u32).collect(),
        Tier::Hs => (0..w.high_schools.len()).filter(|&h| w.high_schools[h].state == state).map(|h| h as u32).collect(),
    };
    let all = rows_for(w, level, year, &ids);
    let lv = level_numbers(&all);
    let rows: Vec<&Sq> = all.iter().collect();
    let depth = super::charting(w, tier, (tier == Tier::College).then_some(division)).map_or(0, |c| c.depth);
    let mut out = json!({
        "level": tier_key(tier),
        "year": year,
        "years": years,
        "division": division.name(),
        "divisions": Division::ALL.iter().map(|d| d.name()).collect::<Vec<_>>(),
        "state": w.states.get(state).map(|s| s.code.clone()),
        "states": w.states.iter().map(|s| s.code.clone()).collect::<Vec<_>>(),
        "depth": depth,
        "view": view,
        "n": rows.len(),
    });
    let brief = |s: &Sq| super::team_ref(w, tier, s.team as usize).map(|r| views::team_brief(w, r));
    match view {
        "tiers" => {
            // Offense against defense, EPA a play where the play by play is
            // charted, else points a game.
            let key = if depth >= 1 && tier != Tier::Hs { "TmEpa" } else { "TmPtsG" };
            let st = stat_by(key).unwrap();
            let pts: Vec<Value> = rows
                .iter()
                .filter_map(|s| {
                    let o = (st.f)(s, 0, &lv)?;
                    let d = (st.f)(s, 1, &lv)?;
                    Some(json!({ "team": brief(s), "o": o, "d": d, "record": format!("{}-{}", s.wins, s.losses) }))
                })
                .collect();
            out["axis"] =
                json!({ "key": key, "label": st.label, "fmt": if key == "TmEpa" { "signed" } else { "one" } });
            out["points"] = json!(pts);
        }
        "power" => {
            out["power"] = power_table(w, tier, &rows, year, (tier == Tier::Hs).then_some(state));
        }
        _ => {
            let stat = args
                .get("stat")
                .and_then(Value::as_str)
                .and_then(stat_by)
                .filter(|s| s.depth <= depth)
                .unwrap_or(&STATS[0]);
            let side = args.get("side").and_then(Value::as_u64).map_or(0, |s| s.min(1) as usize);
            let side = if stat.unit == 0 { side } else { 0 };
            let up = better(stat, side).unwrap_or(true);
            let mut list: Vec<(&Sq, f64)> =
                rows.iter().filter_map(|s| (stat.f)(s, side, &lv).map(|v| (*s, v))).collect();
            list.sort_by(|a, b| {
                if up { b.1.total_cmp(&a.1) } else { a.1.total_cmp(&b.1) }.then(a.0.team.cmp(&b.0.team))
            });
            out["stat"] = json!(stat.key);
            out["side"] = json!(side);
            out["stats"] = json!(
                STATS
                    .iter()
                    .filter(|s| s.depth <= depth && (tier != Tier::Hs || s.depth == 0))
                    .map(|s| json!({ "key": s.key, "label": s.label, "sides": s.unit == 0 }))
                    .collect::<Vec<_>>()
            );
            // The other numbers on the row: the board's core.
            let core: Vec<&TStat> = ["TmPtsG", "TmYpp", "TmEpa", "TmSucc", "TmToMargin"]
                .iter()
                .filter_map(|k| stat_by(k))
                .filter(|s| s.depth <= depth && s.key != stat.key)
                .collect();
            out["cols"] = json!(core.iter().map(|s| json!({ "key": s.key, "label": s.label })).collect::<Vec<_>>());
            out["rows"] = json!(list
                .iter()
                .enumerate()
                .map(|(k, (s, v))| {
                    json!({
                        "rank": k + 1,
                        "team": brief(s),
                        "record": format!("{}-{}", s.wins, s.losses),
                        "value": text(stat.fmt, *v),
                        "cells": core.iter().map(|c| (c.f)(s, if c.unit == 0 { side } else { 0 }, &lv).map(|v| text(c.fmt, v))).collect::<Vec<_>>(),
                    })
                })
                .collect::<Vec<_>>());
        }
    }
    out
}

fn power_table(w: &World, tier: Tier, rows: &[&Sq], year: i32, state: Option<usize>) -> Value {
    let brief = |t: u32| super::team_ref(w, tier, t as usize).map(|r| views::team_brief(w, r));
    let ids: HashSet<u32> = rows.iter().map(|s| s.team).collect();
    if year != w.tally_open_season() {
        let r = power::season_ratings(w, level_of(tier), year);
        let mut list: Vec<(u32, Rating)> = r.into_iter().filter(|x| ids.contains(&x.0)).collect();
        list.sort_by(|a, b| b.1.overall().total_cmp(&a.1.overall()).then(a.0.cmp(&b.0)));
        return json!({
            "closing": true,
            "rows": list.iter().enumerate().map(|(k, (t, r))| json!({ "rank": k + 1, "team": brief(*t), "now": rating_json(r) })).collect::<Vec<_>>(),
        });
    }
    let ps = power_season(w, tier, state);
    let mut list: Vec<(u32, Rating)> =
        ps.fit.teams.iter().filter(|x| ids.contains(x.0) || rows.is_empty()).map(|(t, r)| (*t, *r)).collect();
    // A team with no game yet still shows its projection.
    for (t, r) in &ps.preseason {
        if ids.contains(t) && !list.iter().any(|x| x.0 == *t) {
            list.push((*t, *r));
        }
    }
    list.sort_by(|a, b| b.1.overall().total_cmp(&a.1.overall()).then(a.0.cmp(&b.0)));
    let rec: HashMap<u32, (u32, u32)> = rows.iter().map(|s| (s.team, (s.wins, s.losses))).collect();
    json!({
        "closing": false,
        "season": ps.season,
        "home": format!("{:.1}", ps.fit.home),
        "sigma": format!("{:.1}", ps.sigma),
        "rows": list.iter().enumerate().map(|(k, (t, r))| json!({
            "rank": k + 1,
            "team": brief(*t),
            "record": rec.get(t).map(|x| format!("{}-{}", x.0, x.1)),
            "now": rating_json(r),
            "preseason": ps.preseason.get(t).map(rating_json),
            "odds": ps.odds.get(t).map(odds_json),
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_team_number_reads_a_season() {
        let mut s = Sq { games: 2, ..Default::default() };
        let t = afm_engine::game::stats::TeamStats {
            points: 31,
            plays: 70,
            yards: 420.0,
            third_att: 12,
            third_conv: 5,
            ..Default::default()
        };
        s.boxes = [squads::box_values(&t), squads::box_values(&t)];
        s.luck = vec![2.0; squads::LUCK.len()];
        s.st = vec![1.0; squads::ST.len()];
        s.tend = vec![(1, [40.0, 20.0, 18.0])];
        let lv = Lv { pick_share: 0.2 };
        let mut keys = HashSet::new();
        for st in STATS {
            assert!(keys.insert(st.key), "{} twice", st.key);
            let _ = (st.f)(&s, 0, &lv);
            let _ = (st.f)(&s, 1, &lv);
        }
        let ppg = stat_by("TmPtsG").unwrap();
        assert_eq!((ppg.f)(&s, 0, &lv), Some(15.5));
        let third = stat_by("TmThird").unwrap();
        assert_eq!(text(third.fmt, (third.f)(&s, 0, &lv).unwrap()), "41.7%");
        let proe = stat_by("TmProe").unwrap();
        assert!(((proe.f)(&s, 0, &lv).unwrap() - 0.05).abs() < 1e-9);
        // Even turnovers and even luck: no margin either way.
        assert_eq!((stat_by("TmToMargin").unwrap().f)(&s, 0, &lv), Some(0.0));
        assert!((stat_by("TmToLuck").unwrap().f)(&s, 0, &lv).unwrap().abs() < 1e-9);
        // A defense ranks the other way.
        assert_eq!(better(ppg, 1), Some(false));
    }
}
