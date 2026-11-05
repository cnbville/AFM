//! Judging coaches (Data overhaul item 39, board 48): head coaches and
//! coordinators ranked by their units over one season, three, or every
//! season the tallies hold, with the talent they had, how far their unit
//! finished over it, a play-calling tendency, the head coach's game
//! management against the analysts' model, and the pay where a contract
//! is public.
//!
//! `coach_data` (`level`, `division`, `state`, `role` "oc", "dc" or "hc",
//! `window` 1, 3 or 0 for all): college and the NFL read the people tally
//! (each coach's unit season, EPA a play with its rank); high school reads
//! the squads tally's points against its state's class, as a high school
//! keeps no charting.

use std::collections::HashMap;

use afm_engine::positions::Position;
use afm_engine::world::World;
use afm_engine::world::coach::Role;
use afm_engine::world::folk::Team;
use afm_engine::world::marks::Tier;
use afm_engine::world::tally::Level;
use afm_engine::world::units::Charge;
use serde_json::{Value, json};

use super::teams::{self, stat_by, text};
use super::{division_arg, state_arg, tier_arg, tier_key};
use crate::views::{self, TeamRef};

/// The people tally's index of EPA a play (`people::STATS`).
const EPA: usize = 2;

fn role_arg(args: &Value) -> Role {
    match args.get("role").and_then(Value::as_str) {
        Some("dc") => Role::DefensiveCoordinator,
        Some("hc") => Role::HeadCoach,
        _ => Role::OffensiveCoordinator,
    }
}

fn role_key(r: Role) -> &'static str {
    match r {
        Role::DefensiveCoordinator => "dc",
        Role::HeadCoach => "hc",
        _ => "oc",
    }
}

fn window_arg(args: &Value) -> usize {
    match args.get("window").and_then(Value::as_u64) {
        Some(1) => 1,
        Some(0) => 0,
        _ => 3,
    }
}

/// One coach's season in the job.
#[derive(Clone)]
struct Season {
    year: i32,
    team: u32,
    games: u32,
    /// 100 best at the level, 0 worst.
    pct: f64,
    rank: Option<(u16, u16)>,
}

/// A run of seasons, weighed by games.
fn mean(v: &[&Season]) -> Option<f64> {
    let g: f64 = v.iter().map(|s| s.games.max(1) as f64).sum();
    (g > 0.0 && !v.is_empty()).then(|| v.iter().map(|s| s.pct * s.games.max(1) as f64).sum::<f64>() / g)
}

fn pct_of(rank: u16, n: u16) -> f64 {
    if n > 1 { 100.0 * (n - rank.min(n)) as f64 / (n - 1) as f64 } else { 50.0 }
}

fn team_ref(tier: Tier, t: usize) -> TeamRef {
    match tier {
        Tier::College => TeamRef::College(t),
        Tier::Nfl => TeamRef::Nfl(t),
        Tier::Hs => TeamRef::HighSchool(t),
    }
}

fn unit_side(p: Position) -> usize {
    match p {
        Position::QB | Position::RB | Position::WR | Position::TE | Position::OT | Position::IOL => 0,
        Position::EDGE | Position::DT | Position::LB | Position::CB | Position::S => 1,
        _ => 2,
    }
}

/// What each team had to work with in a season, for the unit the role
/// runs (proposed, not confirmed: the services' average stars of the
/// unit's men at college; at the NFL the cap dollars spent on the unit, as
/// analysts there read talent). Ranked 1 the most.
fn talent(w: &World, tier: Tier, year: i32, role: Role, teams_in: &[u32]) -> HashMap<u32, (f64, usize)> {
    let mut vals: Vec<(u32, f64)> = Vec::new();
    match tier {
        Tier::College => {
            let name = match role {
                Role::OffensiveCoordinator => "stars_off",
                Role::DefensiveCoordinator => "stars_def",
                _ => "stars",
            };
            let pool = teams::pool(w, Level::College, year);
            for s in pool.iter().filter(|s| teams_in.contains(&s.team)) {
                if let Some(v) = s.talent_of(name) {
                    vals.push((s.team, v));
                }
            }
        }
        Tier::Nfl => {
            for r in w.tally_money_level(Level::Nfl, year) {
                if !teams_in.contains(&r.team) {
                    continue;
                }
                let want = match role {
                    Role::OffensiveCoordinator => Some(0),
                    Role::DefensiveCoordinator => Some(1),
                    _ => None,
                };
                let v: f64 = r
                    .pay
                    .iter()
                    .filter(|p| want.is_none_or(|u| p.pos.map(unit_side) == Some(u)))
                    .map(|p| p.a as f64)
                    .sum();
                vals.push((r.team, v));
            }
        }
        Tier::Hs => {}
    }
    vals.sort_by(|a, b| b.1.total_cmp(&a.1));
    vals.iter().enumerate().map(|(k, &(t, v))| (t, (v, k + 1))).collect()
}

/// A play-calling tendency for the role's unit that season, and the head
/// coach's 4th downs against the model.
fn tendency(w: &World, tier: Tier, year: i32, team: u32, role: Role) -> Option<String> {
    let level = teams::level_of(tier);
    match role {
        Role::HeadCoach => {
            let c = w.tally_call(level, year, team)?;
            let s = &c.sum;
            (s.weighed > 0).then(|| {
                format!(
                    "4th downs: the model's call {:.0}% of the time, went {} of {} clear goes",
                    100.0 * s.agreed as f64 / s.weighed as f64,
                    s.went_said_go,
                    s.said_go
                )
            })
        }
        _ => {
            let sq = w.tally_squad(level, year, team)?;
            let lv = teams::Lv::default();
            let (key, side, words) = if role == Role::OffensiveCoordinator {
                ("TmProe", 0, "passes over expected")
            } else {
                ("TmHavoc", 1, "havoc rate")
            };
            let st = stat_by(key)?;
            let v = (st.f)(&sq, side, &lv)?;
            let shown = if key == "TmProe" { format!("{:+.1}%", 100.0 * v) } else { text(st.fmt, v) };
            Some(format!("{shown} {words}"))
        }
    }
}

pub fn coach_data(w: &World, args: &Value) -> Value {
    let tier = tier_arg(w, args);
    let role = role_arg(args);
    let window = window_arg(args);
    // Every coach's seasons in the role at this level, by coach.
    let mut by: HashMap<usize, Vec<Season>> = HashMap::new();
    let group: String;
    let mut team_groups: HashMap<u32, String> = HashMap::new();
    match tier {
        Tier::Hs => {
            let state = state_arg(w, args);
            group = w
                .states
                .get(state)
                .map_or(String::new(), |s| afm_engine::world::folk::facts::state_name(&s.code).to_string());
            hs_seasons(w, state, role, &mut by, &mut team_groups);
        }
        _ => {
            let (lvl, name) = match tier {
                Tier::Nfl => (1u8, "NFL".to_string()),
                _ => (0u8, division_arg(w, args).name().to_string()),
            };
            group = name.clone();
            for y in w.tally_seasons() {
                w.tally_units(y, |r| {
                    if r.level != lvl || r.group != name || r.role != role.name() {
                        return;
                    }
                    let rank = r.ranks.get(EPA).copied();
                    let Some((rk, n)) = rank else { return };
                    by.entry(r.coach as usize).or_default().push(Season {
                        year: r.season,
                        team: r.team,
                        games: r.games,
                        pct: pct_of(rk, n),
                        rank,
                    });
                });
            }
        }
    }
    let latest = by.values().flat_map(|v| v.iter().map(|s| s.year)).max();
    let Some(latest) = latest else {
        return json!({ "level": tier_key(tier), "role": role_key(role), "group": group, "window": window, "rows": [], "note": "No season counted yet." });
    };
    // The men who held the job in the latest season.
    let current: Vec<(usize, Vec<Season>)> = by
        .into_iter()
        .filter(|(_, v)| v.iter().any(|s| s.year == latest))
        .map(|(c, mut v)| {
            v.sort_by_key(|s| s.year);
            (c, v)
        })
        .collect();
    let teams_in: Vec<u32> = current.iter().filter_map(|(_, v)| v.last().map(|s| s.team)).collect();
    let tal = talent(w, tier, latest, role, &teams_in);
    let mine = views::mine(w);
    let mut rows: Vec<(f64, Value)> = current
        .iter()
        .map(|(c, v)| {
            let last = v.last().expect("a season");
            let all: Vec<&Season> = v.iter().collect();
            let three: Vec<&Season> = v.iter().rev().take(3).collect();
            let this = last.pct;
            let three_p = mean(&three).unwrap_or(this);
            let all_p = mean(&all).unwrap_or(this);
            let key = match window {
                1 => this,
                0 => all_p,
                _ => three_p,
            };
            // Since: his first season of the run at this team.
            let since = v.iter().rev().take_while(|s| s.team == last.team).map(|s| s.year).min().unwrap_or(last.year);
            let coach = w.coaches.get(*c);
            let t = team_ref(tier, last.team as usize);
            let tr = tal.get(&last.team);
            let unit_rank = last.rank.map(|r| r.0 as usize);
            let over = match (tr, unit_rank) {
                (Some(&(_, talent_rank)), Some(u)) => Some(talent_rank as i64 - u as i64),
                _ => None,
            };
            let contract = coach.and_then(|c| c.contract.as_ref());
            (
                key,
                json!({
                    "coach": c,
                    "name": coach.map(|c| c.name.clone()),
                    "team": views::team_brief(w, t),
                    "group": team_groups.get(&last.team),
                    "since": since,
                    "seasons": v.len(),
                    "this": this.round() as i64,
                    "three": three_p.round() as i64,
                    "all": all_p.round() as i64,
                    "rank": unit_rank,
                    "n": last.rank.map(|r| r.1),
                    "talent": tr.map(|x| x.1),
                    "over": over,
                    "tendency": tendency(w, tier, last.year, last.team, role),
                    "pay": contract.map(|k| k.salary),
                    "through": contract.map(|k| k.through),
                    "mine": mine == Some(t),
                    "by_year": v.iter().map(|s| json!([s.year, s.pct.round() as i64])).collect::<Vec<_>>(),
                }),
            )
        })
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    let what = match (tier, role) {
        (Tier::Hs, Role::HeadCoach) => "point margin a game",
        (Tier::Hs, Role::DefensiveCoordinator) => "points allowed a game",
        (Tier::Hs, _) => "points a game",
        (_, Role::DefensiveCoordinator) => "EPA a play allowed",
        (_, Role::HeadCoach) => "EPA a play, offense less defense",
        _ => "EPA a play",
    };
    json!({
        "level": tier_key(tier),
        "role": role_key(role),
        "role_name": role.name(),
        "group": group,
        "window": window,
        "year": latest,
        "measure": what,
        "talent_word": match tier {
            Tier::Nfl => "cap dollars on the unit",
            Tier::College => "the services' average stars",
            Tier::Hs => "",
        },
        "rows": rows.into_iter().map(|x| x.1).collect::<Vec<_>>(),
    })
}

/// High school: each school in the state, each season the squads tally
/// holds, its unit's points ranked in its class (proposed, not confirmed:
/// points a game with no schedule adjustment, as the box scores give).
fn hs_seasons(
    w: &World,
    state: usize,
    role: Role,
    by: &mut HashMap<usize, Vec<Season>>,
    groups: &mut HashMap<u32, String>,
) {
    let st = stat_by("TmPtsG");
    let Some(st) = st else { return };
    let lv = teams::Lv::default();
    for y in w.tally_seasons() {
        let pool = teams::pool(w, Level::Hs, y);
        // By class: (team, value).
        let mut by_class: HashMap<u32, Vec<(u32, f64, u32)>> = HashMap::new();
        for s in pool.iter() {
            let Some(h) = w.high_schools.get(s.team as usize) else { continue };
            if h.state != state || s.games == 0 {
                continue;
            }
            let v = match role {
                Role::OffensiveCoordinator => (st.f)(s, 0, &lv),
                Role::DefensiveCoordinator => (st.f)(s, 1, &lv).map(|x| -x),
                _ => match ((st.f)(s, 0, &lv), (st.f)(s, 1, &lv)) {
                    (Some(a), Some(b)) => Some(a - b),
                    _ => None,
                },
            };
            if let Some(v) = v {
                by_class.entry(h.classification).or_default().push((s.team, v, s.games));
            }
        }
        if by_class.is_empty() {
            continue;
        }
        let charge = Charge::new(w, y);
        for (_, mut v) in by_class {
            v.sort_by(|a, b| b.1.total_cmp(&a.1));
            let n = v.len() as u16;
            for (k, &(t, _, games)) in v.iter().enumerate() {
                let Some(c) = charge.get(Team::High(t as usize), role) else { continue };
                groups.entry(t).or_insert_with(|| teams::group_name(w, Tier::Hs, t as usize, y));
                let rank = (k as u16 + 1, n);
                by.entry(c).or_default().push(Season {
                    year: y,
                    team: t,
                    games,
                    pct: pct_of(rank.0, n),
                    rank: Some(rank),
                });
            }
        }
    }
}
