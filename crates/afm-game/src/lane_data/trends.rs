//! Trends (lane NB10, Data overhaul item 29, board 58): how the game
//! changed. `trends` (`tab` "league", "team" or "player"):
//! - league (`level`, `division`, `state`, `stats`): the level's every
//!   counted season on up to three numbers, the average team's line and the
//!   middle half of teams each season;
//! - team (`kind`, `id`, `stats`, `all`): a team's last ten seasons (every
//!   one with `all`) on up to three numbers, with its rank and its group's
//!   line each season;
//! - player (`id`, `level`): his career against the usual age curve at his
//!   spot (`careers::player_trend`).
//!
//! The numbers: every team number of the team pages (lane NB7, read off
//! the squads tally), how teams played (deep throws, play action, the
//! backfield, personnel, motion, tempo, the blitz, man coverage) and how
//! big their starters were, from the trends tally (`world::tally::trends`).
//! Each follows your charting at the level, as the team pages do.

use std::sync::{Arc, Mutex};

use afm_engine::positions::Position;
use afm_engine::world::World;
use afm_engine::world::data::Division;
use afm_engine::world::marks::Tier;
use afm_engine::world::tally::squads::Season as Sq;
use afm_engine::world::tally::trends::{self as tr, Season as Tr};
use afm_engine::world::tally::{self, Level};
use serde_json::{Value, json};

use super::teams::{self as tm, Lv, STATS};
use super::{division_arg, state_arg, tier_arg, tier_key};
use crate::views::{self, TeamRef};

// ---------------------------------------------------------------------
// The numbers.

#[derive(Clone, Copy)]
enum Src {
    /// A team number of the team pages: its index in `teams::STATS` and the
    /// side (0 the offense, 1 the defense: what it allowed).
    Team(usize, usize),
    Style(fn(&Tr) -> Option<f64>),
    /// The starters' sizes at some positions: weight (true) or height.
    Size(&'static [Position], bool),
}

#[derive(Clone, Copy)]
enum NFmt {
    Team(tm::Fmt),
    Pct,
    One,
    Secs,
    Pounds,
    Height,
}

struct Num {
    key: String,
    label: String,
    group: &'static str,
    h8: &'static str,
    fmt: NFmt,
    up: Option<bool>,
    depth: u8,
    src: Src,
}

fn ratio(n: f64, d: f64) -> Option<f64> {
    (d > 0.0).then(|| n / d)
}

/// How teams played: (key, label, group, depth, which way is better, the
/// number). Depths follow the player cards' (`analytics::depth`): air
/// yards are play by play, the formation, personnel, play action, the
/// blitz and coverage need film, time to throw tracking. No style is
/// better by itself, except the offense's runs into loaded boxes.
#[allow(clippy::type_complexity)]
const STYLE: &[(&str, &str, &str, u8, NFmt, fn(&Tr) -> Option<f64>)] = &[
    ("TrDeep", "Deep throw rate", "Passing game", 1, NFmt::Pct, |t| {
        ratio(t.style_of(0, "deep"), t.style_of(0, "aimed"))
    }),
    ("TrScreen", "Throws behind the line", "Passing game", 1, NFmt::Pct, |t| {
        ratio(t.style_of(0, "behind"), t.style_of(0, "aimed"))
    }),
    ("TrAdot", "Depth of target", "Passing game", 2, NFmt::One, |t| ratio(t.pass_of("air"), t.pass_of("aimed"))),
    ("TrTtt", "Time to throw", "Passing game", 3, NFmt::Secs, |t| ratio(t.pass_of("ttt"), t.pass_of("att"))),
    ("TrPa", "Play action rate", "Passing game", 2, NFmt::Pct, |t| {
        ratio(t.style_of(0, "play_action"), t.style_of(0, "dropbacks"))
    }),
    ("TrGun", "Shotgun and pistol", "Formation and tempo", 2, NFmt::Pct, |t| {
        ratio(t.style_of(0, "gun") + t.style_of(0, "pistol"), t.style_of(0, "snaps"))
    }),
    ("TrUc", "Under center", "Formation and tempo", 2, NFmt::Pct, |t| {
        ratio(t.style_of(0, "under_center"), t.style_of(0, "snaps"))
    }),
    ("Tr11", "11 personnel", "Formation and tempo", 2, NFmt::Pct, |t| {
        ratio(t.style_of(0, "p11"), t.style_of(0, "snaps"))
    }),
    ("Tr12", "12 personnel", "Formation and tempo", 2, NFmt::Pct, |t| {
        ratio(t.style_of(0, "p12"), t.style_of(0, "snaps"))
    }),
    ("Tr21", "21 personnel", "Formation and tempo", 2, NFmt::Pct, |t| {
        ratio(t.style_of(0, "p21"), t.style_of(0, "snaps"))
    }),
    ("TrMotion", "Motion rate", "Formation and tempo", 2, NFmt::Pct, |t| {
        ratio(t.style_of(0, "motion"), t.style_of(0, "snaps"))
    }),
    ("TrNoHuddle", "No-huddle rate", "Formation and tempo", 1, NFmt::Pct, |t| {
        ratio(t.style_of(0, "no_huddle"), t.style_of(0, "snaps"))
    }),
    ("TrRpo", "RPO rate", "Formation and tempo", 2, NFmt::Pct, |t| ratio(t.style_of(0, "rpo"), t.style_of(0, "snaps"))),
    ("TrBox8", "Runs into 8+ in the box", "Formation and tempo", 2, NFmt::Pct, |t| {
        ratio(t.style_of(0, "box8"), t.style_of(0, "runs"))
    }),
    ("TrBlitz", "Blitz rate", "Defense", 2, NFmt::Pct, |t| ratio(t.style_of(1, "blitz"), t.style_of(1, "dropbacks"))),
    ("TrMan", "Man coverage rate", "Defense", 2, NFmt::Pct, |t| {
        ratio(t.style_of(1, "man"), t.style_of(1, "dropbacks"))
    }),
    ("TrNickel", "Nickel and dime", "Defense", 2, NFmt::Pct, |t| {
        ratio(t.style_of(1, "nickel") + t.style_of(1, "dime"), t.style_of(1, "snaps"))
    }),
];

/// The size groups: (key stem, label, positions).
const SIZES: &[(&str, &str, &[Position])] = &[
    ("QB", "quarterbacks", &[Position::QB]),
    ("RB", "running backs", &[Position::RB]),
    ("WR", "wide receivers", &[Position::WR]),
    ("TE", "tight ends", &[Position::TE]),
    ("OL", "offensive line", &[Position::OT, Position::IOL]),
    ("DL", "interior line", &[Position::DT]),
    ("EDGE", "edge rushers", &[Position::EDGE]),
    ("LB", "linebackers", &[Position::LB]),
    ("CB", "cornerbacks", &[Position::CB]),
    ("S", "safeties", &[Position::S]),
];

/// Every number the page offers: `team` the team tab's (both sides).
fn catalogue(team: bool, hs: bool) -> Vec<Num> {
    let mut out = Vec::new();
    for (k, st) in STATS.iter().enumerate() {
        let group = match st.unit {
            3 => "Special teams",
            2 => "The whole team",
            _ => "Offense",
        };
        // A whole team's margins average nothing across a league.
        if st.unit == 2 && !team {
            continue;
        }
        out.push(Num {
            key: st.key.to_string(),
            label: st.label.to_string(),
            group,
            h8: st.key,
            fmt: NFmt::Team(st.fmt),
            up: tm::better(st, 0),
            depth: st.depth,
            src: Src::Team(k, 0),
        });
        if team && st.unit == 0 {
            out.push(Num {
                key: format!("{}|d", st.key),
                label: format!("{} allowed", st.label),
                group: "Defense",
                h8: st.key,
                fmt: NFmt::Team(st.fmt),
                up: tm::better(st, 1),
                depth: st.depth,
                src: Src::Team(k, 1),
            });
        }
    }
    if !hs {
        for &(key, label, group, depth, fmt, f) in STYLE {
            out.push(Num {
                key: key.into(),
                label: label.into(),
                group: if group == "Defense" { "Defense style" } else { group },
                h8: key,
                fmt,
                up: (key == "TrBox8").then_some(false),
                depth,
                src: Src::Style(f),
            });
        }
    }
    for &(stem, label, pos) in SIZES {
        for weight in [true, false] {
            out.push(Num {
                key: format!("Sz{stem}{}", if weight { "Wt" } else { "Ht" }),
                label: format!("{}, {label}", if weight { "Weight" } else { "Height" }),
                group: "Starters' sizes",
                h8: if weight { "SzWt" } else { "SzHt" },
                fmt: if weight { NFmt::Pounds } else { NFmt::Height },
                up: None,
                depth: 0,
                src: Src::Size(pos, weight),
            });
        }
    }
    out
}

fn text(f: NFmt, v: f64) -> String {
    match f {
        NFmt::Team(t) => tm::text(t, v),
        NFmt::Pct => format!("{:.1}%", 100.0 * v),
        NFmt::One => format!("{v:.1}"),
        NFmt::Secs => format!("{v:.2} s"),
        NFmt::Pounds => format!("{v:.1} lb"),
        NFmt::Height => {
            let ft = (v / 12.0).floor();
            format!("{}'{:.1}\"", ft as i64, v - 12.0 * ft)
        }
    }
}

fn value(n: &Num, sq: Option<&Sq>, t: Option<&Tr>, lv: &Lv) -> Option<f64> {
    match n.src {
        Src::Team(k, side) => sq.and_then(|s| (STATS[k].f)(s, side, lv)),
        Src::Style(f) => t.and_then(f),
        Src::Size(pos, weight) => {
            let codes: Vec<u8> = pos.iter().map(|p| *p as u8).collect();
            t.and_then(|t| t.size_of(&codes)).map(|(_, h, w)| if weight { w } else { h })
        }
    }
}

fn meta(n: &Num) -> Value {
    json!({
        "key": n.key, "label": n.label, "group": n.group, "h8": n.h8,
        "better": match n.up { Some(true) => "high", Some(false) => "low", None => "none" },
    })
}

/// The menu: the numbers your charting reaches, by group.
fn menu(nums: &[Num], depth: u8) -> Value {
    let mut groups: Vec<(&str, Vec<Value>)> = Vec::new();
    for n in nums.iter().filter(|n| n.depth <= depth) {
        match groups.iter_mut().find(|g| g.0 == n.group) {
            Some(g) => g.1.push(meta(n)),
            None => groups.push((n.group, vec![meta(n)])),
        }
    }
    json!(groups.into_iter().map(|(g, items)| json!({ "group": g, "items": items })).collect::<Vec<_>>())
}

/// The numbers asked for (`stats`: a list or a comma line), at most three,
/// that your charting reaches; else the defaults.
fn picked<'a>(nums: &'a [Num], args: &Value, depth: u8, defaults: &[&str]) -> Vec<&'a Num> {
    let asked: Vec<String> = match args.get("stats") {
        Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
        Some(Value::String(s)) => s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect(),
        _ => Vec::new(),
    };
    let find = |k: &str| nums.iter().find(|n| n.key == k && n.depth <= depth);
    let mut out: Vec<&Num> = asked.iter().filter_map(|k| find(k)).collect();
    out.dedup_by(|a, b| a.key == b.key);
    out.truncate(3);
    if out.is_empty() {
        out = defaults.iter().filter_map(|k| find(k)).take(3).collect();
    }
    out
}

// ---------------------------------------------------------------------
// A season's rows.

type Kept<T> = Mutex<Vec<((usize, u8, i32), Arc<T>)>>;
static TRENDS: Kept<Vec<Tr>> = Mutex::new(Vec::new());
const KEEP: usize = 40;

fn world_key(w: &World) -> usize {
    w as *const World as usize ^ w.tally_dir().map_or(0, |p| p.as_os_str().len())
}

/// A season's trends rows at a level (a closed season read once and kept;
/// the season being counted read off the open stores now).
fn trend_pool(w: &World, level: Level, season: i32) -> Arc<Vec<Tr>> {
    if season == w.tally_open_season() {
        return Arc::new(tr::rows(w).into_iter().map(|x| x.1).filter(|t| t.level == level as u8).collect());
    }
    let key = (world_key(w), level as u8, season);
    if let Some(x) = TRENDS.lock().ok().and_then(|v| v.iter().find(|e| e.0 == key).map(|e| e.1.clone())) {
        return x;
    }
    let mut v = Vec::new();
    let _ = w.tally_each("trends", season, |k, r| {
        if (k >> 32) as u8 == level as u8
            && let Some(t) = tally::file::decode::<Tr>(r)
        {
            v.push(t);
        }
    });
    let x = Arc::new(v);
    if let Ok(mut s) = TRENDS.lock() {
        s.retain(|e| e.0 != key);
        s.push((key, x.clone()));
        if s.len() > KEEP {
            s.remove(0);
        }
    }
    x
}

/// One season at a level: each team's squad and trends rows.
struct SeasonRows {
    sq: Arc<Vec<Sq>>,
    tr: Arc<Vec<Tr>>,
    lv: Lv,
}

impl SeasonRows {
    fn of(w: &World, level: Level, season: i32) -> SeasonRows {
        let sq = tm::pool(w, level, season);
        let lv = tm::level_numbers(&sq);
        SeasonRows { sq, tr: trend_pool(w, level, season), lv }
    }

    fn get(&self, team: u32) -> (Option<&Sq>, Option<&Tr>) {
        (self.sq.iter().find(|s| s.team == team), self.tr.iter().find(|t| t.team == team))
    }

    /// Each team's value among `teams` (None: every team in the rows).
    fn values(&self, n: &Num, keep: &dyn Fn(u32) -> bool) -> Vec<(u32, f64)> {
        let mut ids: Vec<u32> = self.sq.iter().map(|s| s.team).chain(self.tr.iter().map(|t| t.team)).collect();
        ids.sort_unstable();
        ids.dedup();
        ids.into_iter()
            .filter(|&t| keep(t))
            .filter_map(|t| {
                let (s, r) = self.get(t);
                value(n, s, r, &self.lv).map(|v| (t, v))
            })
            .collect()
    }
}

/// The mean and the middle half (the 25th and 75th percentiles).
fn spread(v: &mut [f64]) -> Option<(f64, f64, f64)> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let q = |p: f64| {
        let x = p * (v.len() - 1) as f64;
        let (lo, hi) = (x.floor() as usize, x.ceil() as usize);
        v[lo] + (v[hi] - v[lo]) * (x - lo as f64)
    };
    Some((v.iter().sum::<f64>() / v.len() as f64, q(0.25), q(0.75)))
}

fn cell(n: &Num, vals: &mut [f64]) -> Value {
    match spread(vals) {
        Some((m, a, b)) => json!({
            "v": m, "lo": a, "hi": b, "n": vals.len(),
            "text": text(n.fmt, m), "lo_text": text(n.fmt, a), "hi_text": text(n.fmt, b),
        }),
        None => Value::Null,
    }
}

/// The seasons counted at a level, oldest first (the season being played
/// once its teams have a game).
fn seasons(w: &World, level: Level) -> Vec<i32> {
    let open = w.tally_open_season();
    let mut ys: Vec<i32> =
        w.tally_seasons().into_iter().filter(|&y| y != open || !tm::pool(w, level, y).is_empty()).collect();
    ys.sort_unstable();
    ys
}

fn level_of(t: Tier) -> Level {
    tm::level_of(t)
}

fn depth_at(w: &World, tier: Tier, division: Option<Division>) -> u8 {
    super::charting(w, tier, division).map_or(0, |c| c.depth)
}

// ---------------------------------------------------------------------
// The tabs.

fn league(w: &World, args: &Value) -> Value {
    let tier = tier_arg(w, args);
    let division = (tier == Tier::College).then(|| division_arg(w, args));
    let state = (tier == Tier::Hs).then(|| state_arg(w, args));
    let depth = depth_at(w, tier, division);
    let nums = catalogue(false, tier == Tier::Hs);
    let defaults: &[&str] = match tier {
        Tier::Hs => &["TmPtsG", "TmYpp"],
        _ => &["TmEarlyPass", "TmPtsG", "TmYpp"],
    };
    let pick = picked(&nums, args, depth, defaults);
    let level = level_of(tier);
    let rows: Vec<Value> = seasons(w, level)
        .into_iter()
        .map(|y| {
            let s = SeasonRows::of(w, level, y);
            let keep = |t: u32| match tier {
                Tier::College => {
                    (t as usize) < w.programs.len() && Some(super::season_place(w, t as usize, y).0) == division
                }
                Tier::Nfl => true,
                Tier::Hs => w.high_schools.get(t as usize).map(|h| h.state) == state,
            };
            let cells: Vec<Value> = pick
                .iter()
                .map(|n| {
                    let mut v: Vec<f64> = s.values(n, &keep).into_iter().map(|x| x.1).collect();
                    cell(n, &mut v)
                })
                .collect();
            json!({ "year": y, "cells": cells })
        })
        .filter(|r| r["cells"].as_array().is_some_and(|c| c.iter().any(|x| !x.is_null())))
        .collect();
    json!({
        "tab": "league",
        "level": tier_key(tier),
        "division": division.map(|d| d.name()),
        "divisions": Division::ALL.iter().map(|d| d.name()).collect::<Vec<_>>(),
        "state": state.and_then(|s| w.states.get(s)).map(|s| s.code.as_str()),
        "scope": match tier {
            Tier::College => division.map_or("College".to_string(), |d| d.name().to_string()),
            Tier::Nfl => "NFL".to_string(),
            Tier::Hs => state.and_then(|s| w.states.get(s)).map_or("High school".to_string(), |s| afm_engine::world::folk::facts::state_name(&s.code).to_string()),
        },
        "stats": pick.iter().map(|n| meta(n)).collect::<Vec<_>>(),
        "menu": menu(&nums, depth),
        "rows": rows,
        "depth": depth,
        "source": super::sources(w)["line"].clone(),
    })
}

fn team(w: &World, args: &Value) -> Value {
    let t = match TeamRef::from_args(w, args) {
        Ok(t) => t,
        Err(e) => return json!({ "error": e }),
    };
    let Some((tier, team)) = tm::tier_of(t) else {
        return json!({ "team": views::team_brief(w, t), "error": "Another league's clubs keep no team data." });
    };
    let level = level_of(tier);
    let (mut years, _) = tm::team_years(w, tier, team);
    years.retain(|&y| tm::pool(w, level, y).iter().any(|s| s.team == team as u32));
    years.sort_unstable();
    let all = args.get("all").and_then(Value::as_bool).unwrap_or(false);
    if !all && years.len() > 10 {
        years.drain(..years.len() - 10);
    }
    let last = years.last().copied().unwrap_or(w.year);
    let division = (tier == Tier::College).then(|| super::season_place(w, team, last).0);
    let depth = depth_at(w, tier, division);
    let nums = catalogue(true, tier == Tier::Hs);
    let pick = picked(&nums, args, depth, &["TmPtsG", "TmPtsG|d", "TmYpp"]);
    let rows: Vec<Value> = years
        .iter()
        .map(|&y| {
            let s = SeasonRows::of(w, level, y);
            let members = tm::members(w, tier, team, y);
            let keep = |t: u32| members.binary_search(&t).is_ok();
            let (sq, trow) = s.get(team as u32);
            let cells: Vec<Value> = pick
                .iter()
                .map(|n| {
                    let vals = s.values(n, &keep);
                    let mine = value(n, sq, trow, &s.lv);
                    let rank = mine.and_then(|v| {
                        n.up.map(|up| {
                            1 + vals.iter().filter(|x| if up { x.1 > v + 1e-9 } else { x.1 < v - 1e-9 }).count()
                        })
                    });
                    let mut g: Vec<f64> = vals.iter().map(|x| x.1).collect();
                    let n_teams = g.len();
                    json!({
                        "v": mine, "text": mine.map(|v| text(n.fmt, v)), "rank": rank, "of": n_teams,
                        "group": cell(n, &mut g),
                    })
                })
                .collect();
            json!({
                "year": y,
                "group": tm::group_name(w, tier, team, y),
                "record": sq.map(|s| format!("{}-{}", s.wins, s.losses)),
                "cells": cells,
            })
        })
        .collect();
    json!({
        "tab": "team",
        "level": tier_key(tier),
        "team": views::team_brief(w, t),
        "stats": pick.iter().map(|n| meta(n)).collect::<Vec<_>>(),
        "menu": menu(&nums, depth),
        "rows": rows,
        "all": all,
        "depth": depth,
        "source": super::sources(w)["line"].clone(),
    })
}

pub fn trends(w: &World, args: &Value) -> Value {
    match args.get("tab").and_then(Value::as_str) {
        Some("team") => team(w, args),
        Some("player") => super::careers::player_trend(w, args),
        _ => league(w, args),
    }
}

/// The keys the page can show, for the glossary's test.
#[cfg(test)]
pub(crate) fn h8_keys() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = catalogue(true, false).iter().map(|n| n.h8).collect();
    v.sort_unstable();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles_read_their_counts_and_sizes_format() {
        let mut t = Tr { level: 0, team: 1, games: 12, ..Default::default() };
        t.style = vec![0.0; 2 * tr::STYLE.len()];
        let at = |n: &str| tr::STYLE.iter().position(|s| *s == n).unwrap();
        t.style[at("aimed")] = 400.0;
        t.style[at("deep")] = 52.0;
        t.style[tr::STYLE.len() + at("dropbacks")] = 420.0;
        t.style[tr::STYLE.len() + at("blitz")] = 126.0;
        t.sizes = vec![
            (Position::OT as u8, [24.0, 24.0 * 78.0, 24.0 * 315.0]),
            (Position::IOL as u8, [36.0, 36.0 * 75.0, 36.0 * 310.0]),
        ];
        let nums = catalogue(true, false);
        let get = |k: &str| {
            let n = nums.iter().find(|n| n.key == k).unwrap();
            value(n, None, Some(&t), &Lv::default())
        };
        assert!((get("TrDeep").unwrap() - 0.13).abs() < 1e-9);
        assert!((get("TrBlitz").unwrap() - 0.3).abs() < 1e-9);
        assert!((get("SzOLWt").unwrap() - 312.0).abs() < 1e-9);
        assert!((get("SzOLHt").unwrap() - 76.2).abs() < 1e-9);
        assert!(get("SzQBWt").is_none());
        assert_eq!(text(NFmt::Height, 76.2), "6'4.2\"");
        assert_eq!(text(NFmt::Pounds, 312.0), "312.0 lb");
        let mut v = vec![4.0, 1.0, 3.0, 2.0, 5.0];
        let (m, lo, hi) = spread(&mut v).unwrap();
        assert_eq!((m, lo, hi), (3.0, 2.0, 4.0));
        // High school offers only what a box score and a roster give.
        assert!(catalogue(false, true).iter().all(|n| !matches!(n.src, Src::Style(_))));
        assert!(catalogue(false, false).iter().all(|n| !n.key.ends_with("|d")));
    }
}
