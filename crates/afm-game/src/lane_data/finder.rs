//! The stat finder (lane NB10, Data overhaul item 32, board 55): Stathead
//! for the world. `stat_finder` (`find` "seasons", "careers", "games" or
//! "streaks"; `level`, `division` ("all" for every one), `state`, `from`,
//! `to`, `pos`, `class`, `rs`, `age_min`, `age_max`, `conds` (a list of
//! `{stat, op ">=" or "<=", v}`), `sort`, `dir`, `limit`, `min_len`):
//! - seasons: every player season at the level meeting the conditions, from
//!   the stat book (college, since the books began), the NFL careers and the
//!   high school lines;
//! - careers: those seasons summed by player (the seasons that pass the
//!   filters), the conditions on the totals;
//! - games: every game line from the tallies' games files (college and the
//!   NFL; high school keeps no player lines);
//! - streaks: the runs of games in a row a player met every condition in,
//!   across seasons, games he didn't play left out, as Stathead counts them.
//!
//! `stat_finder_stats` gives the numbers it searches on. Rates are the
//! analytics cards' (the stat book's numbers; the depth counts are too big
//! to search across seasons). Nothing here reads a hidden rating.

use std::collections::HashMap;

use afm_engine::calendar::Date;
use afm_engine::positions::Position;
use afm_engine::world::World;
use afm_engine::world::analytics::{Cm, Fmt};
use afm_engine::world::data::Division;
use afm_engine::world::marks::{self, Tier};
use afm_engine::world::records::{STAT_COUNT, Stat};
use afm_engine::world::tally::games::Line;
use serde_json::{Value, json};

use super::analytics as ay;
use super::careers::{self, Bios};
use super::{fmt_num, state_arg, team_json, tier_arg, tier_key};
use crate::views;

// ---------------------------------------------------------------------
// The numbers.

#[derive(Clone, Copy)]
enum Src {
    Games,
    Stat(Stat),
    Sum(&'static [Stat]),
    Cm(Cm),
}

pub(super) struct FStat {
    key: &'static str,
    group: &'static str,
    src: Src,
}

const fn c(key: &'static str, group: &'static str, s: Stat) -> FStat {
    FStat { key, group, src: Src::Stat(s) }
}

const fn r(key: &'static str, group: &'static str, cm: Cm) -> FStat {
    FStat { key, group, src: Src::Cm(cm) }
}

/// Every number the finder searches on, each named by its glossary entry.
pub(super) const STATS: &[FStat] = &[
    FStat { key: "Games", group: "Playing time", src: Src::Games },
    c("SnapsOff", "Playing time", Stat::SnapsOff),
    c("SnapsDef", "Playing time", Stat::SnapsDef),
    c("Cmp", "Passing", Stat::Cmp),
    c("Att", "Passing", Stat::Att),
    r("Pct", "Passing", Cm::CmpPct),
    c("PassYds", "Passing", Stat::PassYds),
    r("PassYdsG", "Passing", Cm::PassYdsG),
    r("Ypa", "Passing", Cm::Ypa),
    c("PassTd", "Passing", Stat::PassTd),
    c("Int", "Passing", Stat::Int),
    r("TdPct", "Passing", Cm::TdPct),
    r("IntPct", "Passing", Cm::IntPct),
    r("Rating", "Passing", Cm::Rating),
    c("Sacked", "Passing", Stat::Sacked),
    r("EpaDb", "Passing", Cm::EpaDb),
    r("Cpoe", "Passing", Cm::Cpoe),
    r("Adot", "Passing", Cm::Adot),
    c("Rush", "Rushing", Stat::Rush),
    c("RushYds", "Rushing", Stat::RushYds),
    r("RushYdsG", "Rushing", Cm::RushYdsG),
    r("Ypc", "Rushing", Cm::Ypc),
    c("RushTd", "Rushing", Stat::RushTd),
    c("RushLong", "Rushing", Stat::RushLong),
    c("ForcedMissed", "Rushing", Stat::ForcedMissed),
    c("ExplosiveRuns", "Rushing", Stat::ExplosiveRuns),
    c("FumblesLost", "Rushing", Stat::FumblesLost),
    c("Targets", "Receiving", Stat::Targets),
    c("Rec", "Receiving", Stat::Rec),
    c("RecYds", "Receiving", Stat::RecYds),
    r("RecYdsG", "Receiving", Cm::RecYdsG),
    r("Ypr", "Receiving", Cm::Ypr),
    c("RecTd", "Receiving", Stat::RecTd),
    c("RecLong", "Receiving", Stat::RecLong),
    r("CatchRate", "Receiving", Cm::CatchRate),
    r("Yprr", "Receiving", Cm::Yprr),
    c("Drops", "Receiving", Stat::Drops),
    FStat { key: "ScrimYds", group: "Scrimmage", src: Src::Sum(&[Stat::RushYds, Stat::RecYds]) },
    FStat { key: "Touches", group: "Scrimmage", src: Src::Sum(&[Stat::Rush, Stat::Rec]) },
    FStat { key: "TotTd", group: "Scrimmage", src: Src::Sum(&[Stat::RushTd, Stat::RecTd, Stat::ReturnTd]) },
    c("Tackles", "Defense", Stat::Tackles),
    c("Tfl", "Defense", Stat::Tfl),
    c("Sacks", "Defense", Stat::Sacks),
    c("Pressures", "Defense", Stat::Pressures),
    c("Ints", "Defense", Stat::Ints),
    c("Pbus", "Defense", Stat::Pbus),
    c("ForcedFumbles", "Defense", Stat::ForcedFumbles),
    c("Missed", "Defense", Stat::Missed),
    c("SacksAllowed", "Blocking", Stat::SacksAllowed),
    c("FgMade", "Kicking", Stat::FgMade),
    c("FgAtt", "Kicking", Stat::FgAtt),
    r("FgPct", "Kicking", Cm::FgPct),
    c("Fg50Made", "Kicking", Stat::Fg50Made),
    c("FgLong", "Kicking", Stat::FgLong),
    c("XpMade", "Kicking", Stat::XpMade),
    c("Punts", "Punting", Stat::Punts),
    r("PuntAvg", "Punting", Cm::NetPunt),
    c("Returns", "Returns", Stat::Returns),
    c("ReturnYds", "Returns", Stat::ReturnYds),
    r("RetAvg", "Returns", Cm::RetAvg),
    c("ReturnTd", "Returns", Stat::ReturnTd),
];

/// What a page shows by default for a position (beside the conditions).
fn default_cols(pos: Option<&[Position]>) -> &'static [&'static str] {
    use Position as P;
    match pos.and_then(|p| p.first()) {
        Some(P::QB) => &["Cmp", "Att", "PassYds", "PassTd", "Int", "Rating"],
        Some(P::RB) => &["Rush", "RushYds", "Ypc", "RushTd", "Rec", "RecYds"],
        Some(P::WR | P::TE) => &["Rec", "RecYds", "Ypr", "RecTd"],
        Some(P::OT | P::IOL) => &["SnapsOff", "SacksAllowed"],
        Some(P::K) => &["FgMade", "FgAtt", "FgPct", "FgLong"],
        Some(P::P) => &["Punts", "PuntAvg"],
        Some(P::DT | P::EDGE | P::LB) => &["Tackles", "Tfl", "Sacks", "Pressures"],
        Some(P::CB | P::S) => &["Tackles", "Ints", "Pbus"],
        _ => &["ScrimYds", "TotTd", "Tackles", "Sacks"],
    }
}

fn stat_by(key: &str) -> Option<&'static FStat> {
    STATS.iter().find(|s| s.key == key)
}

fn value(s: &FStat, all: &dyn Fn(Stat) -> f64, games: f64) -> Option<f64> {
    match s.src {
        Src::Games => Some(games),
        Src::Stat(k) => Some(all(k)),
        Src::Sum(ks) => Some(ks.iter().map(|&k| all(k)).sum()),
        Src::Cm(cm) => cm.value(all, games),
    }
}

fn text(s: &FStat, v: f64) -> String {
    match s.src {
        Src::Cm(cm) => ay::fmt_cell(cm, v),
        // The engine tracks yards to the inch; the books show whole yards
        // (half sacks and tackles for loss keep their half).
        _ if s.key.contains("Yds") || s.key.contains("Long") => fmt_num(v.round()),
        _ => fmt_num(v),
    }
}

fn name_of(s: &FStat) -> &'static str {
    super::glossary::TERMS.iter().find(|t| t.key == s.key).map_or(s.key, |t| t.name)
}

fn better_of(s: &FStat) -> i8 {
    super::glossary::TERMS.iter().find(|t| t.key == s.key).map_or(1, |t| t.better)
}

pub fn stats_json() -> Value {
    json!(
        STATS
            .iter()
            .map(|s| json!({
                "key": s.key, "name": name_of(s), "group": s.group,
                "rate": matches!(s.src, Src::Cm(c) if c.about().2 != Fmt::Whole), "better": better_of(s),
            }))
            .collect::<Vec<_>>()
    )
}

pub fn finder_stats(_w: &World, _args: &Value) -> Value {
    json!({ "stats": stats_json() })
}

// ---------------------------------------------------------------------
// The query.

struct Cond {
    stat: &'static FStat,
    at_least: bool,
    v: f64,
}

impl Cond {
    fn holds(&self, x: Option<f64>) -> bool {
        x.is_some_and(|x| if self.at_least { x >= self.v - 1e-9 } else { x <= self.v + 1e-9 })
    }
}

struct Query {
    tier: Tier,
    division: Option<Division>,
    state: Option<usize>,
    from: i32,
    to: i32,
    pos: Option<Vec<Position>>,
    class: Option<u8>,
    rs: Option<bool>,
    age: (Option<u32>, Option<u32>),
    conds: Vec<Cond>,
    cols: Vec<&'static FStat>,
    sort: usize,
    up: bool,
    limit: usize,
}

fn positions_arg(s: &str) -> Option<Vec<Position>> {
    match s {
        "OL" => Some(vec![Position::OT, Position::IOL]),
        "DL" => Some(vec![Position::DT, Position::EDGE]),
        "DB" => Some(vec![Position::CB, Position::S]),
        _ => Position::ALL.iter().find(|p| p.name().eq_ignore_ascii_case(s)).map(|p| vec![*p]),
    }
}

fn query(w: &World, args: &Value) -> Query {
    let tier = tier_arg(w, args);
    let division = match (tier, args.get("division").and_then(Value::as_str)) {
        (Tier::College, Some("all")) => None,
        (Tier::College, Some(d)) => Division::parse(d).or(Some(Division::FBS)),
        (Tier::College, None) => Some(super::division_arg(w, args)),
        _ => None,
    };
    let state = (tier == Tier::Hs).then(|| state_arg(w, args));
    let year = |k: &str| args.get(k).and_then(Value::as_i64).map(|y| y as i32);
    let (lo, hi) = (year("from"), year("to"));
    let pos = args.get("pos").and_then(Value::as_str).and_then(positions_arg);
    let class = match args.get("class") {
        Some(Value::String(c)) => match c.as_str() {
            "Fr" => Some(1),
            "So" => Some(2),
            "Jr" => Some(3),
            "Sr" => Some(4),
            n => n.trim_end_matches('+').parse::<u8>().ok().filter(|n| (1..=5).contains(n)),
        },
        Some(Value::Number(n)) => n.as_u64().map(|n| n.clamp(1, 5) as u8),
        _ => None,
    };
    let rs = match args.get("rs").and_then(Value::as_str) {
        Some("yes") if tier == Tier::College => Some(true),
        Some("no") if tier == Tier::College => Some(false),
        _ => None,
    };
    let age = |k: &str| args.get(k).and_then(Value::as_u64).map(|a| a as u32);
    let conds: Vec<Cond> = args
        .get("conds")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|c| {
                    Some(Cond {
                        stat: stat_by(c.get("stat")?.as_str()?)?,
                        at_least: c.get("op").and_then(Value::as_str) != Some("<="),
                        v: c.get("v").and_then(Value::as_f64)?,
                    })
                })
                .take(8)
                .collect()
        })
        .unwrap_or_default();
    // The columns: the conditions' numbers, then the position's own.
    let mut cols: Vec<&'static FStat> = Vec::new();
    for c in &conds {
        if !cols.iter().any(|x| x.key == c.stat.key) {
            cols.push(c.stat);
        }
    }
    let sort_key = args.get("sort").and_then(Value::as_str).and_then(stat_by);
    if let Some(s) = sort_key
        && !cols.iter().any(|x| x.key == s.key)
    {
        cols.push(s);
    }
    for k in default_cols(pos.as_deref()) {
        if let Some(s) = stat_by(k)
            && !cols.iter().any(|x| x.key == s.key)
        {
            cols.push(s);
        }
    }
    let sort = sort_key.and_then(|s| cols.iter().position(|x| x.key == s.key)).unwrap_or(0);
    let up = match args.get("dir").and_then(Value::as_i64) {
        Some(d) => d > 0,
        None => better_of(cols[sort]) < 0,
    };
    let first = match tier {
        Tier::College => w.stats.year_index.first().map_or(w.year, |x| x.0),
        _ => w.year - 40,
    };
    let (from, to) = (lo.unwrap_or(first).max(first - 1), hi.unwrap_or(w.year));
    Query {
        tier,
        division,
        state,
        from: from.min(to),
        to: to.max(from),
        pos,
        class,
        rs,
        age: (age("age_min"), age("age_max")),
        conds,
        cols,
        sort,
        up,
        limit: args.get("limit").and_then(Value::as_u64).map_or(100, |l| l as usize).clamp(10, 500),
    }
}

impl Query {
    fn pos_ok(&self, p: Position) -> bool {
        self.pos.as_ref().is_none_or(|v| v.contains(&p))
    }

    /// The filters on the man himself in a season: class, redshirt, age.
    fn bio_ok(&self, bios: &Bios, cache: &mut HashMap<(u32, i32), bool>, id: u32, year: i32) -> bool {
        if self.class.is_none() && self.rs.is_none() && self.age.0.is_none() && self.age.1.is_none() {
            return true;
        }
        *cache.entry((id, year)).or_insert_with(|| {
            let Some(c) = bios.class(self.tier, id, year) else { return false };
            let age = bios.age(id, year).unwrap_or(0);
            self.class.is_none_or(|x| x == c.0)
                && self.rs.is_none_or(|x| x == c.1)
                && self.age.0.is_none_or(|a| age >= a)
                && self.age.1.is_none_or(|a| age <= a)
        })
    }

    fn passes(&self, all: &dyn Fn(Stat) -> f64, games: f64) -> bool {
        self.conds.iter().all(|c| c.holds(value(c.stat, all, games)))
    }

    fn vals(&self, all: &dyn Fn(Stat) -> f64, games: f64) -> Vec<Option<f64>> {
        self.cols.iter().map(|s| value(s, all, games)).collect()
    }

    fn sort_rows(&self, rows: &mut [Found]) {
        let k = self.sort;
        rows.sort_by(|a, b| {
            let (x, y) = (a.vals[k].unwrap_or(f64::NEG_INFINITY), b.vals[k].unwrap_or(f64::NEG_INFINITY));
            let o = if self.up { x.total_cmp(&y) } else { y.total_cmp(&x) };
            o.then(b.year.cmp(&a.year)).then(a.id.cmp(&b.id))
        });
    }
}

/// One season (or career, or game) found.
struct Found {
    id: u32,
    pos: Position,
    team: usize,
    year: i32,
    games: f64,
    vals: Vec<Option<f64>>,
    /// Careers: the first and last season and the seasons counted; games:
    /// the game's facts.
    extra: Value,
}

/// Every player season at the level in the years asked for: (id, his
/// position that season, team, year, games, the season's numbers).
fn each_season(w: &World, q: &Query, mut f: impl FnMut(u32, Position, usize, i32, f64, &[f32; STAT_COUNT])) {
    match q.tier {
        Tier::College => {
            for y in q.from..=q.to {
                let mut filed = false;
                for r in w.stats.year(y) {
                    filed = true;
                    if q.division.is_some_and(|d| d != r.division) || !q.pos_ok(r.pos) {
                        continue;
                    }
                    let mut all = [0f32; STAT_COUNT];
                    for (k, a) in all.iter_mut().enumerate() {
                        *a = r.get(Stat::ALL[k]) as f32;
                    }
                    f(r.id, r.pos, r.program as usize, y, r.games as f64, &all);
                }
                // The season being played: its lines so far, until the
                // stat book files them at the close.
                if y == w.year && !filed {
                    for l in marks::season_now(w, Tier::College, None) {
                        let division = (l.team < w.programs.len()).then(|| super::season_place(w, l.team, y).0);
                        if l.games == 0 || q.division.is_some_and(|d| Some(d) != division) || !q.pos_ok(l.pos) {
                            continue;
                        }
                        f(l.id, l.pos, l.team, y, l.games as f64, &l.all);
                    }
                }
            }
        }
        Tier::Nfl => {
            let people = w.people.iter().map(|x| (x.player.id, x.player.position, x.pro.as_deref()));
            let gone = w.alumni.iter().filter_map(|a| {
                let p = a.player.as_ref()?;
                views::person_index(w, p.id).is_none().then_some((p.id, a.position, a.pro.as_deref()))
            });
            for (id, pos, pro) in people.chain(gone) {
                let Some(c) = pro else { continue };
                if !q.pos_ok(pos) {
                    continue;
                }
                for s in c.seasons.iter().filter(|s| s.games > 0 && (q.from..=q.to).contains(&s.year)) {
                    let Some(team) = s.team else { continue };
                    let all = afm_engine::world::records::from_line(&s.stats);
                    f(id, pos, team, s.year, s.games as f64, &all);
                }
            }
        }
        Tier::Hs => {
            // The season being played, for those whose lines aren't filed
            // yet.
            if (q.from..=q.to).contains(&w.year) {
                for l in marks::season_now(w, Tier::Hs, q.state) {
                    let filed = views::person_index(w, l.id)
                        .is_some_and(|k| w.people[k].hs_lines.iter().any(|h| h.year as i32 == w.year));
                    if filed || !q.pos_ok(l.pos) {
                        continue;
                    }
                    f(l.id, l.pos, l.team, w.year, l.games as f64, &l.all);
                }
            }
            for x in &w.people {
                for l in &x.hs_lines {
                    let y = l.year as i32;
                    if !(q.from..=q.to).contains(&y)
                        || !q.pos_ok(l.pos)
                        || w.high_schools.get(l.school as usize).map(|h| h.state) != q.state
                    {
                        continue;
                    }
                    let all = marks::hs_all(&l.s.map(|v| v as i32));
                    f(x.player.id, l.pos, l.school as usize, y, l.games as f64, &all);
                }
            }
        }
    }
}

fn seasons(w: &World, q: &Query, bios: &Bios) -> (Vec<Found>, usize) {
    let mut cache = HashMap::new();
    let mut out = Vec::new();
    each_season(w, q, |id, pos, team, year, games, all| {
        let get = |s: Stat| all[s as usize] as f64;
        if !q.passes(&get, games) || !q.bio_ok(bios, &mut cache, id, year) {
            return;
        }
        out.push(Found { id, pos, team, year, games, vals: q.vals(&get, games), extra: Value::Null });
    });
    let n = out.len();
    (out, n)
}

/// A career's sums, games, seasons, first and last season, last team and
/// position.
type Career = (Box<[f64; STAT_COUNT]>, f64, u32, i32, i32, usize, Position);

fn careers_of(w: &World, q: &Query, bios: &Bios) -> (Vec<Found>, usize) {
    let mut cache = HashMap::new();
    // id -> (sums, games, seasons, first, last, last team, position)
    let mut acc: HashMap<u32, Career> = HashMap::new();
    each_season(w, q, |id, pos, team, year, games, all| {
        if !q.bio_ok(bios, &mut cache, id, year) {
            return;
        }
        let e = acc.entry(id).or_insert_with(|| (Box::new([0.0; STAT_COUNT]), 0.0, 0, year, year, team, pos));
        for (a, b) in e.0.iter_mut().zip(all) {
            *a += *b as f64;
        }
        e.1 += games;
        e.2 += 1;
        e.3 = e.3.min(year);
        if year >= e.4 {
            e.4 = year;
            e.5 = team;
            e.6 = pos;
        }
    });
    let mut out = Vec::new();
    for (id, (all, games, n, first, last, team, pos)) in acc {
        let get = |s: Stat| all[s as usize];
        if !q.passes(&get, games) {
            continue;
        }
        out.push(Found {
            id,
            pos,
            team,
            year: last,
            games,
            vals: q.vals(&get, games),
            extra: json!({ "first": first, "last": last, "seasons": n }),
        });
    }
    let n = out.len();
    (out, n)
}

/// The game lines a search reads: the facts and every player's line.
#[derive(serde::Deserialize)]
struct GLite {
    level: u8,
    season: i32,
    date: i32,
    kind: String,
    teams: [u32; 2],
    neutral: bool,
    score: [i32; 2],
    lines: Vec<Line>,
}

/// Every game line at the level in the years asked for, in the order the
/// games were played: (game key, the game, the line).
fn each_line(w: &World, q: &Query, mut f: impl FnMut(u64, &GLite, &Line, usize)) -> Vec<i32> {
    let level = match q.tier {
        Tier::Nfl => 1u8,
        _ => 0u8,
    };
    let seasons: Vec<i32> = w.tally_seasons().into_iter().filter(|y| (q.from..=q.to).contains(y)).collect();
    for &y in &seasons {
        w.tally_games_as::<GLite>(y, |k, g| {
            if g.level != level {
                return;
            }
            for l in &g.lines {
                let side = (l.side as usize).min(1);
                let team = g.teams[side] as usize;
                if q.tier == Tier::College
                    && q.division
                        .is_some_and(|d| team >= w.programs.len() || super::season_place(w, team, g.season).0 != d)
                {
                    continue;
                }
                f(k, &g, l, side);
            }
        });
    }
    seasons
}

fn line_pos(w: &World, l: &Line) -> Position {
    l.pos.unwrap_or_else(|| views::person_index(w, l.id).map_or(Position::LS, |k| w.people[k].player.position))
}

fn game_facts(w: &World, tier: Tier, key: u64, g: &GLite, side: usize) -> Value {
    let (us, them) = (g.score[side], g.score[1 - side]);
    let res = match us.cmp(&them) {
        std::cmp::Ordering::Greater => "W",
        std::cmp::Ordering::Less => "L",
        std::cmp::Ordering::Equal => "T",
    };
    json!({
        "date": Date(g.date).to_string(),
        "at": if g.neutral { "vs" } else if side == 0 { "" } else { "at" },
        "opp": team_json(w, tier, g.teams[1 - side] as usize),
        "result": format!("{res} {us}-{them}"),
        "kind": g.kind,
        "key": key,
    })
}

fn games(w: &World, q: &Query, bios: &Bios) -> (Vec<Found>, usize, Vec<i32>) {
    let mut cache = HashMap::new();
    let mut out = Vec::new();
    let mut n = 0;
    let seasons = each_line(w, q, |k, g, l, side| {
        let all = l.all();
        let get = |s: Stat| all[s as usize] as f64;
        if !q.passes(&get, 1.0) {
            return;
        }
        let pos = line_pos(w, l);
        if !q.pos_ok(pos) || !q.bio_ok(bios, &mut cache, l.id, g.season) {
            return;
        }
        n += 1;
        out.push(Found {
            id: l.id,
            pos,
            team: g.teams[side] as usize,
            year: g.season,
            games: 1.0,
            vals: q.vals(&get, 1.0),
            extra: game_facts(w, q.tier, k, g, side),
        });
        // Keep the best few thousand while reading, so a loose search
        // stays small.
        if out.len() > 4 * q.limit.max(500) {
            q.sort_rows(&mut out);
            out.truncate(q.limit.max(500));
        }
    });
    (out, n, seasons)
}

/// One streak: a player's games in a row meeting every condition.
struct Streak {
    id: u32,
    pos: Position,
    team: usize,
    len: u32,
    from: i32,
    to: i32,
    from_season: i32,
    to_season: i32,
    running: bool,
}

/// One of his games: date, season, team, whether it met every condition,
/// position.
type GameMet = (i32, i32, usize, bool, Position);

fn streaks(w: &World, q: &Query, bios: &Bios, min_len: u32) -> (Vec<Streak>, Vec<i32>) {
    let mut cache = HashMap::new();
    // id -> his games in order: (date, season, team, met, position)
    let mut by: HashMap<u32, Vec<GameMet>> = HashMap::new();
    let seasons = each_line(w, q, |_, g, l, side| {
        let all = l.all();
        let get = |s: Stat| all[s as usize] as f64;
        let met = q.passes(&get, 1.0);
        by.entry(l.id).or_default().push((
            g.date,
            g.season,
            g.teams[side] as usize,
            met,
            l.pos.unwrap_or(Position::LS),
        ));
    });
    let mut out = Vec::new();
    for (id, mut gs) in by {
        gs.sort_by_key(|x| x.0);
        let pos = gs
            .iter()
            .rev()
            .map(|x| x.4)
            .find(|p| *p != Position::LS)
            .unwrap_or_else(|| views::person_index(w, id).map_or(Position::LS, |k| w.people[k].player.position));
        if !q.pos_ok(pos) {
            continue;
        }
        let mut run: Option<(usize, usize)> = None;
        let close = |run: Option<(usize, usize)>,
                     out: &mut Vec<Streak>,
                     running: bool,
                     cache: &mut HashMap<(u32, i32), bool>| {
            let Some((a, b)) = run else { return };
            let len = (b - a + 1) as u32;
            if len < min_len || !q.bio_ok(bios, cache, id, gs[a].1) {
                return;
            }
            out.push(Streak {
                id,
                pos,
                team: gs[b].2,
                len,
                from: gs[a].0,
                to: gs[b].0,
                from_season: gs[a].1,
                to_season: gs[b].1,
                running,
            });
        };
        for (i, g) in gs.iter().enumerate() {
            if g.3 {
                run = Some(run.map_or((i, i), |(a, _)| (a, i)));
            } else {
                close(run.take(), &mut out, false, &mut cache);
            }
        }
        // A streak still runs when it holds through his last game and that
        // game is in the season being played.
        let now = q.to >= w.year && gs.last().is_some_and(|x| x.1 == w.tally_open_season());
        close(run, &mut out, now, &mut cache);
    }
    out.sort_by(|a, b| b.len.cmp(&a.len).then(b.to.cmp(&a.to)).then(a.id.cmp(&b.id)));
    (out, seasons)
}

fn who(w: &World, bios: &Bios, tier: Tier, id: u32, pos: Position, team: usize, year: i32) -> Value {
    json!({
        "id": id,
        "name": ay::name(w, id),
        "linked": views::person_index(w, id).is_some(),
        "pos": pos.name(),
        "team": team_json(w, tier, team),
        "year": year,
        "class": bios.class(tier, id, year).map(|c| careers::class_text(tier, c)),
        "age": bios.age(id, year),
    })
}

pub fn stat_finder(w: &World, args: &Value) -> Value {
    let q = query(w, args);
    let find = match args.get("find").and_then(Value::as_str) {
        Some("careers") => "careers",
        Some("games") => "games",
        Some("streaks") => "streaks",
        _ => "seasons",
    };
    let bios = Bios::new(w);
    let cols: Vec<Value> = q.cols.iter().map(|s| json!({ "key": s.key, "name": name_of(s) })).collect();
    let mut m = serde_json::Map::new();
    m.insert("find".into(), json!(find));
    m.insert("level".into(), json!(tier_key(q.tier)));
    m.insert("division".into(), json!(q.division.map_or("all", |d| d.name())));
    m.insert("state".into(), json!(q.state.and_then(|s| w.states.get(s)).map(|s| s.code.as_str())));
    m.insert("from".into(), json!(q.from));
    m.insert("to".into(), json!(q.to));
    m.insert("cols".into(), json!(cols));
    m.insert("sort".into(), json!(q.sort));
    m.insert("up".into(), json!(q.up));
    m.insert("divisions".into(), json!(Division::ALL.iter().map(|d| d.name()).collect::<Vec<_>>()));
    m.insert(
        "since".into(),
        json!(match q.tier {
            Tier::College => w.stats.year_index.first().map(|x| x.0),
            _ => None,
        }),
    );
    if (find == "games" || find == "streaks") && q.tier == Tier::Hs {
        m.insert("rows".into(), json!([]));
        m.insert("total".into(), json!(0));
        m.insert("empty".into(), json!("High school games keep box scores only: search seasons and careers there."));
        return Value::Object(m);
    }
    if find == "streaks" {
        if q.conds.is_empty() {
            m.insert("rows".into(), json!([]));
            m.insert("total".into(), json!(0));
            m.insert("empty".into(), json!("A streak needs a condition: add one, such as rushing yards at least 100."));
            return Value::Object(m);
        }
        let min_len = args.get("min_len").and_then(Value::as_u64).map_or(3, |x| x as u32).max(2);
        let (found, seasons) = streaks(w, &q, &bios, min_len);
        let rows: Vec<Value> = found
            .iter()
            .take(q.limit)
            .map(|s| {
                let mut r = who(w, &bios, q.tier, s.id, s.pos, s.team, s.to_season);
                r["len"] = json!(s.len);
                r["from"] = json!(Date(s.from).to_string());
                r["to"] = json!(Date(s.to).to_string());
                r["seasons"] = json!(if s.from_season == s.to_season {
                    s.from_season.to_string()
                } else {
                    format!("{}-{}", s.from_season, s.to_season)
                });
                r["running"] = json!(s.running);
                r
            })
            .collect();
        m.insert("total".into(), json!(found.len()));
        m.insert("rows".into(), json!(rows));
        m.insert("min_len".into(), json!(min_len));
        m.insert("seasons".into(), json!(seasons));
        return Value::Object(m);
    }
    let (mut found, total, seasons) = match find {
        "careers" => {
            let (f, n) = careers_of(w, &q, &bios);
            (f, n, Vec::new())
        }
        "games" => games(w, &q, &bios),
        _ => {
            let (f, n) = seasons(w, &q, &bios);
            (f, n, Vec::new())
        }
    };
    q.sort_rows(&mut found);
    found.truncate(q.limit);
    let rows: Vec<Value> = found
        .iter()
        .map(|f| {
            let mut r = who(w, &bios, q.tier, f.id, f.pos, f.team, f.year);
            r["games"] = json!(f.games);
            r["v"] = json!(f.vals);
            r["vals"] = json!(f.vals.iter().zip(&q.cols).map(|(v, s)| v.map(|v| text(s, v))).collect::<Vec<_>>());
            if !f.extra.is_null() {
                r["extra"] = f.extra.clone();
            }
            r
        })
        .collect();
    m.insert("total".into(), json!(total));
    m.insert("rows".into(), json!(rows));
    if find == "games" {
        m.insert("seasons".into(), json!(seasons));
    }
    Value::Object(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every number the finder searches on has its plain words, and its
    /// rates read the stat book (no depth counts).
    #[test]
    fn every_finder_number_explains_itself() {
        let mut seen = std::collections::HashSet::new();
        for s in STATS {
            assert!(seen.insert(s.key), "{} twice", s.key);
            assert!(super::super::glossary::TERMS.iter().any(|t| t.key == s.key), "{} has no glossary entry", s.key);
            if let Src::Cm(cm) = s.src {
                assert!(!cm.deep(), "{} needs the depth counts", s.key);
                assert_eq!(super::super::glossary::cm_key(cm), s.key, "{cm:?}");
            }
        }
    }

    #[test]
    fn conditions_and_sums_read_a_line() {
        let mut all = [0f32; STAT_COUNT];
        all[Stat::RushYds as usize] = 1_040.0;
        all[Stat::Rush as usize] = 190.0;
        all[Stat::RecYds as usize] = 210.0;
        all[Stat::RushTd as usize] = 11.0;
        let get = |s: Stat| all[s as usize] as f64;
        let scrim = stat_by("ScrimYds").unwrap();
        assert_eq!(value(scrim, &get, 12.0), Some(1_250.0));
        let ypc = value(stat_by("Ypc").unwrap(), &get, 12.0).unwrap();
        assert!((ypc - 1040.0 / 190.0).abs() < 1e-9);
        let c = Cond { stat: stat_by("RushYds").unwrap(), at_least: true, v: 1000.0 };
        assert!(c.holds(Some(1_040.0)) && !c.holds(Some(999.0)) && !c.holds(None));
        let c = Cond { stat: stat_by("Ypc").unwrap(), at_least: false, v: 5.0 };
        assert!(!c.holds(Some(ypc)));
        assert_eq!(positions_arg("OL"), Some(vec![Position::OT, Position::IOL]));
        assert_eq!(positions_arg("rb"), Some(vec![Position::RB]));
    }
}
