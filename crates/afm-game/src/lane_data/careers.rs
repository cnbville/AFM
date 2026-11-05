//! Careers across seasons (lane NB10, Data overhaul items 29 and 31;
//! boards 52 and 56): a level's every season for a position group read
//! once, so a player's career can be laid against the usual age curve at
//! his spot and matched with the players whose first seasons looked like
//! his.
//! - `player_trend` (`id`, `level`, `division`, `state`): his seasons on
//!   each of his card's numbers, with his age, class and percentile, and
//!   the usual qualifier at each age (item 29's player trend);
//! - `career_comps` (`id`, `level`, `division`, `seasons`, `mode` "first" or
//!   "last", `drafted`, `n`): the players whose first (or last) seasons at
//!   the level read closest to his, and what became of them (item 31).
//!
//! Every season is read against its own field (a college division, the
//! NFL, a state's high schools), so eras compare fairly. The numbers are
//! the stat book's (the card's numbers that don't need the depth counts),
//! which go back to the world's first recorded season. Nothing here reads
//! a hidden rating.

use std::collections::HashMap;

use afm_engine::world::analytics::{self as an, Cm, Fmt};
use afm_engine::world::data::Division;
use afm_engine::world::marks::{self, Tier};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use super::analytics::{self as ay, Bio, Place, Row};
use super::deep::Src;
use super::{division_arg, state_arg, team_json, tier_key};
use crate::views;

/// Who a player is across the people still in the world and the ones who
/// left it, for his age and class in any season.
pub(super) struct Bios<'a> {
    w: &'a World,
    gone: HashMap<u32, usize>,
}

impl<'a> Bios<'a> {
    pub(super) fn new(w: &'a World) -> Bios<'a> {
        let gone = w.alumni.iter().enumerate().filter_map(|(k, a)| a.player.as_ref().map(|p| (p.id, k))).collect();
        Bios { w, gone }
    }

    pub(super) fn get(&self, id: u32) -> Option<Bio<'a>> {
        let w = self.w;
        if let Some(k) = views::person_index(w, id) {
            let x = &w.people[k];
            return Some(Bio {
                class_of: x.class_of,
                born: x.born,
                staff: &x.college_staff,
                pro: x.pro.as_deref(),
                stars: x.stars,
                home: x.home_state,
            });
        }
        let a = &w.alumni[*self.gone.get(&id)?];
        Some(Bio {
            class_of: a.class_of,
            born: a.born,
            staff: &a.college_staff,
            pro: a.pro.as_deref(),
            stars: a.stars,
            home: a.home_state,
        })
    }

    /// His pro career, wherever he is.
    pub(super) fn pro(&self, id: u32) -> Option<&'a afm_engine::world::pro::ProCareer> {
        self.get(id).and_then(|b| b.pro)
    }

    /// His age on 31 August of a season.
    pub(super) fn age(&self, id: u32, year: i32) -> Option<u32> {
        self.get(id).map(|b| ay::age_in(&b, year))
    }

    /// His class in a season (1 to 4 at college and high school, 1 to 5 in
    /// the NFL) and whether he had redshirted by then.
    pub(super) fn class(&self, tier: Tier, id: u32, year: i32) -> Option<(u8, bool)> {
        self.get(id).map(|b| ay::class_in(self.w, tier, id, &b, year))
    }
}

/// A class as a roster writes it.
pub(super) fn class_text(tier: Tier, c: (u8, bool)) -> String {
    match tier {
        Tier::Nfl => match c.0 {
            1 => "Rookie".into(),
            5 => "5th year+".into(),
            n => format!("{}{} year", n, ordinal_suffix(n as u32)),
        },
        _ => {
            let n = ["Fr", "So", "Jr", "Sr"][(c.0.clamp(1, 4) - 1) as usize];
            if c.1 { format!("RS {n}") } else { n.to_string() }
        }
    }
}

pub(super) fn ordinal_suffix(n: u32) -> &'static str {
    match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    }
}

/// One season of a field: its rows, who qualified, each row's percentile
/// on every number and on the volume.
pub(super) struct FieldSeason {
    pub(super) year: i32,
    pub(super) rows: Vec<Row>,
    pub(super) qual: Vec<bool>,
    pub(super) pcts: Vec<Vec<Option<f64>>>,
    pub(super) vol_pct: Vec<Option<f64>>,
}

/// A place's every season for a group, oldest first, on the card's stat
/// book numbers.
pub(super) struct Field {
    pub(super) card: Vec<Cm>,
    pub(super) charted: bool,
    pub(super) volume: &'static str,
    pub(super) seasons: Vec<FieldSeason>,
}

impl Field {
    /// Each player's seasons in the field: (season, row), oldest first.
    pub(super) fn by_player(&self) -> HashMap<u32, Vec<(usize, usize)>> {
        let mut out: HashMap<u32, Vec<(usize, usize)>> = HashMap::new();
        for (s, fs) in self.seasons.iter().enumerate() {
            for (i, r) in fs.rows.iter().enumerate() {
                out.entry(r.id).or_default().push((s, i));
            }
        }
        out
    }
}

/// The percentile (0-100, 100 the best) of each row's volume among the
/// qualifiers.
fn vol_pcts(rows: &[Row], qual: &[bool]) -> Vec<Option<f64>> {
    let mut peers: Vec<f64> = rows.iter().zip(qual).filter(|(_, q)| **q).map(|(r, _)| r.vol).collect();
    peers.sort_by(f64::total_cmp);
    let n = peers.len();
    rows.iter()
        .map(|r| {
            (n >= 2).then(|| {
                let below = peers.partition_point(|&x| x < r.vol) as f64;
                let same = peers.partition_point(|&x| x <= r.vol) as f64 - below;
                (100.0 * (below + 0.5 * same) / n as f64).clamp(0.0, 100.0)
            })
        })
        .collect()
}

/// Every season of a group at a place, the season being played included
/// as far as your charting has it.
pub(super) fn field(w: &World, p: Place, group: usize) -> Field {
    let (full, _, word, ch) = ay::card_of(w, p, group, 0);
    let keep: Vec<usize> = (0..full.len()).filter(|&k| !full[k].deep()).collect();
    let card: Vec<Cm> = keep.iter().map(|&k| full[k]).collect();
    let (now, _) = ay::charted_now(w, p, marks::season_now(w, p.tier, p.state));
    let ys = ay::years(w, p, &now);
    let mut pools = ay::past_pools(w, p, group, &ys);
    if ys.contains(&w.year) && !now.is_empty() {
        pools.push((w.year, ay::pool(w, p, w.year, group, 0, &now, &Src::None)));
    }
    pools.sort_by_key(|x| x.0);
    let seasons = pools
        .into_iter()
        .map(|(year, rows)| {
            let min = if year == w.year {
                ay::min_for(w, p, group, year, ch, &Value::Null, &now)
            } else {
                an::default_min(group, ch, p.tier == Tier::Nfl)
            };
            let qual: Vec<bool> = rows.iter().map(|r| r.vol >= min).collect();
            let all = ay::percentiles(&rows, &qual, &full);
            let pcts = all.into_iter().map(|v| keep.iter().map(|&k| v[k]).collect()).collect();
            let vol_pct = vol_pcts(&rows, &qual);
            let rows = rows
                .into_iter()
                .map(|mut r| {
                    r.vals = keep.iter().map(|&k| r.vals[k]).collect();
                    r
                })
                .collect();
            FieldSeason { year, rows, qual, pcts, vol_pct }
        })
        .collect();
    Field { card, charted: ch, volume: word, seasons }
}

/// The place a player's career is read at: the level asked for, else where
/// he plays now (else his last level); at college the division asked for,
/// else his last season's; in high school his school's state.
pub(super) fn place_for(w: &World, id: u32, args: &Value) -> Option<(Place, usize)> {
    let k = views::person_index(w, id);
    let x = k.map(|k| &w.people[k]);
    let book = w.stats.career(id);
    let pro = Bios::new(w).pro(id).is_some_and(|c| c.seasons.iter().any(|s| s.games > 0));
    let tier = match args.get("level").and_then(Value::as_str) {
        Some(_) => super::tier_arg(w, args),
        None => match x.map(|x| x.stage) {
            Some(Stage::Pro) => Tier::Nfl,
            Some(Stage::HighSchool) => Tier::Hs,
            Some(Stage::College | Stage::PrepSchool | Stage::Away | Stage::Declared) => Tier::College,
            _ if pro => Tier::Nfl,
            _ if !book.is_empty() => Tier::College,
            _ => Tier::Hs,
        },
    };
    let division = (tier == Tier::College).then(|| {
        args.get("division")
            .and_then(Value::as_str)
            .and_then(Division::parse)
            .or_else(|| book.last().map(|r| r.division))
            .or_else(|| x.and_then(|x| x.college).map(|c| w.programs[c].division))
            .unwrap_or_else(|| division_arg(w, args))
    });
    let state = (tier == Tier::Hs).then(|| {
        if args.get("state").is_some() { state_arg(w, args) } else { x.map_or(0, |x| w.high_schools[x.hs].state) }
    });
    // His group: the position he was listed at in his last season there,
    // else his position now.
    let pos = match tier {
        Tier::College => book.iter().rev().find(|r| Some(r.division) == division).map(|r| r.pos),
        Tier::Nfl => w.marks.pro_rows.iter().rev().find(|r| r.id == id).map(|r| r.pos),
        Tier::Hs => x.and_then(|x| x.hs_lines.last().map(|l| l.pos)),
    }
    .or_else(|| x.map(|x| x.player.position))
    .or_else(|| {
        let b = Bios::new(w);
        b.gone.get(&id).map(|&k| w.alumni[k].position)
    })?;
    let group = args
        .get("group")
        .and_then(Value::as_str)
        .and_then(|g| an::GROUPS.iter().position(|x| x.eq_ignore_ascii_case(g)))
        .or_else(|| an::group_of(pos))?;
    Some((Place { tier, division, state }, group))
}

fn fmt_of(cm: Cm) -> &'static str {
    match cm.about().2 {
        Fmt::Whole => "whole",
        Fmt::One => "one",
        Fmt::Two => "two",
        Fmt::Pct => "pct",
        Fmt::Signed => "signed",
    }
}

pub(super) fn metric_json(cm: Cm) -> Value {
    let (short, long, _, _) = cm.about();
    json!({
        "label": short, "long": long, "better": ay::better_word(cm),
        "key": super::glossary::cm_key(cm), "fmt": fmt_of(cm),
    })
}

/// What became of a player, in a line: his draft slot and NFL career, or
/// where he is now.
pub(super) fn outcome(w: &World, bios: &Bios, id: u32) -> Value {
    let person = views::person_index(w, id).map(|k| &w.people[k]);
    let pro = bios.pro(id);
    let (mut games, mut starts, mut bowls, mut all_pro, mut seasons) = (0u32, 0u32, 0u32, 0u32, 0u32);
    if let Some(c) = pro {
        for s in &c.seasons {
            games += s.games;
            starts += s.starts;
            bowls += s.pro_bowl as u32;
            all_pro += (s.all_pro == 1) as u32;
            seasons += (s.games > 0) as u32;
        }
    }
    let draft = pro.and_then(|c| c.draft);
    let mut bits: Vec<String> = Vec::new();
    match draft {
        Some((year, round, pick, _)) => {
            bits.push(format!("{round}{} round, {year} (pick {pick})", ordinal_suffix(round)))
        }
        None if games > 0 => bits.push("Undrafted".into()),
        None => {}
    }
    if games > 0 {
        bits.push(if starts > 0 {
            format!("{starts} NFL start{}", if starts == 1 { "" } else { "s" })
        } else {
            format!("{games} NFL game{}", if games == 1 { "" } else { "s" })
        });
    }
    if bowls > 0 {
        bits.push(format!("{bowls} Pro Bowl{}", if bowls == 1 { "" } else { "s" }));
    }
    if all_pro > 0 {
        bits.push(format!("{all_pro}x first-team All-Pro"));
    }
    let now = match person.map(|x| x.stage) {
        Some(Stage::HighSchool) => Some("In high school"),
        Some(Stage::College | Stage::PrepSchool | Stage::Away) => Some("Still in college"),
        Some(Stage::Declared) => Some("In the draft pool"),
        Some(Stage::Pro) if pro.is_some_and(|c| c.team.is_some()) => Some("active"),
        _ => None,
    };
    let text = match (bits.is_empty(), now) {
        (true, Some(n)) => n.to_string(),
        (true, None) => "Undrafted, out of football".to_string(),
        (false, Some("active")) => format!("{}, active", bits.join(", ")),
        (false, Some(n)) => format!("{}; {}", bits.join(", "), n.to_lowercase()),
        (false, None) => bits.join(", "),
    };
    json!({
        "text": text,
        "draft": draft.map(|d| json!({ "year": d.0, "round": d.1, "pick": d.2 })),
        "nfl": json!({ "seasons": seasons, "games": games, "starts": starts, "pro_bowls": bowls, "all_pro": all_pro }),
    })
}

// ---------------------------------------------------------------------
// `player_trend` (item 29: a player's career against the usual age curve).

/// The fewest qualifiers at an age for the curve to show it (proposed, not
/// confirmed).
const CURVE_MIN: u32 = 8;

pub fn player_trend(w: &World, args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_u64).map(|x| x as u32) else {
        return json!({ "error": "No player." });
    };
    let Some((p, group)) = place_for(w, id, args) else {
        return json!({ "error": "No numbers for this player." });
    };
    let f = field(w, p, group);
    let bios = Bios::new(w);
    // The usual qualifier at each age: the mean of every qualifying season
    // at that age, his own left out.
    let mut acc: std::collections::BTreeMap<u32, (u32, Vec<(f64, u32)>)> = Default::default();
    let mut mine = Vec::new();
    for (s, fs) in f.seasons.iter().enumerate() {
        for (i, r) in fs.rows.iter().enumerate() {
            if r.id == id {
                mine.push((s, i));
                continue;
            }
            if !fs.qual[i] {
                continue;
            }
            let Some(age) = bios.age(r.id, fs.year) else { continue };
            let e = acc.entry(age).or_insert_with(|| (0, vec![(0.0, 0); f.card.len()]));
            e.0 += 1;
            for (k, v) in r.vals.iter().enumerate() {
                if let Some(v) = v {
                    e.1[k].0 += v;
                    e.1[k].1 += 1;
                }
            }
        }
    }
    let curve: Vec<Value> = acc
        .iter()
        .filter(|(_, e)| e.0 >= CURVE_MIN)
        .map(|(age, e)| {
            json!({
                "age": age, "n": e.0,
                "v": e.1.iter().map(|&(s, n)| (n >= CURVE_MIN).then(|| s / n as f64)).collect::<Vec<_>>(),
                "text": e.1.iter().zip(&f.card).map(|(&(s, n), cm)| (n >= CURVE_MIN).then(|| ay::fmt_cell(*cm, s / n as f64))).collect::<Vec<_>>(),
            })
        })
        .collect();
    let seasons: Vec<Value> = mine
        .iter()
        .map(|&(s, i)| {
            let fs = &f.seasons[s];
            let r = &fs.rows[i];
            json!({
                "year": fs.year,
                "age": bios.age(id, fs.year),
                "class": bios.class(p.tier, id, fs.year).map(|c| class_text(p.tier, c)),
                "team": team_json(w, p.tier, r.team),
                "pos": r.pos.name(),
                "games": r.games,
                "vol": r.vol.round() as i64,
                "qualified": fs.qual[i],
                "v": r.vals,
                "text": r.vals.iter().zip(&f.card).map(|(v, cm)| v.map(|v| ay::fmt_cell(*cm, v))).collect::<Vec<_>>(),
                "pct": fs.pcts[i].iter().map(|x| x.map(|x| x.round() as i64)).collect::<Vec<_>>(),
                "field": fs.qual.iter().filter(|q| **q).count(),
            })
        })
        .collect();
    json!({
        "id": id,
        "name": ay::name(w, id),
        "linked": views::person_index(w, id).is_some(),
        "level": tier_key(p.tier),
        "division": p.division.map(|d| d.name()),
        "state": p.state.and_then(|s| w.states.get(s)).map(|s| s.code.as_str()),
        "group": an::GROUPS[group],
        "charted": f.charted,
        "volume": f.volume,
        "metrics": f.card.iter().map(|c| metric_json(*c)).collect::<Vec<_>>(),
        "seasons": seasons,
        "curve": curve,
        "curve_min": CURVE_MIN,
        "since": f.seasons.first().map(|s| s.year),
        "outcome": outcome(w, &bios, id),
    })
}

// ---------------------------------------------------------------------
// `career_comps` (item 31: whose first seasons looked like his).

/// Points of closeness a year of age apart costs (proposed, not confirmed).
const AGE_GAP: f64 = 3.0;

/// How close two seasons read: the mean gap in percentiles over the
/// numbers and the volume both have, None when they share under half.
fn season_gap(a: (&[Option<f64>], Option<f64>), b: (&[Option<f64>], Option<f64>)) -> Option<f64> {
    let pairs: Vec<f64> =
        a.0.iter()
            .zip(b.0)
            .map(|(x, y)| (*x, *y))
            .chain(std::iter::once((a.1, b.1)))
            .filter_map(|(x, y)| Some((x? - y?).abs()))
            .collect();
    (!pairs.is_empty() && pairs.len() * 2 > a.0.len()).then(|| pairs.iter().sum::<f64>() / pairs.len() as f64)
}

pub fn career_comps(w: &World, args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_u64).map(|x| x as u32) else {
        return json!({ "error": "No player." });
    };
    let Some((p, group)) = place_for(w, id, args) else {
        return json!({ "error": "No numbers for this player." });
    };
    let f = field(w, p, group);
    let by = f.by_player();
    let Some(mine) = by.get(&id) else {
        return json!({ "error": "No season at this level yet.", "level": tier_key(p.tier) });
    };
    let last = args.get("mode").and_then(Value::as_str) == Some("last");
    let n = args.get("seasons").and_then(Value::as_u64).map_or(mine.len(), |x| x as usize).clamp(1, mine.len().min(4));
    let drafted = args.get("drafted").and_then(Value::as_bool).unwrap_or(false);
    let want = args.get("n").and_then(Value::as_u64).map_or(5, |x| x as usize).clamp(1, 20);
    let bios = Bios::new(w);
    let take = |seq: &[(usize, usize)]| -> Vec<(usize, usize)> {
        if last { seq[seq.len() - n..].to_vec() } else { seq[..n].to_vec() }
    };
    let me = take(mine);
    let at = |&(s, i): &(usize, usize)| (f.seasons[s].pcts[i].as_slice(), f.seasons[s].vol_pct[i]);
    let my_age = bios.age(id, f.seasons[me[0].0].year);
    let mut found: Vec<(f64, u32, Vec<(usize, usize)>)> = Vec::new();
    for (&oid, seq) in &by {
        if oid == id || seq.len() < n {
            continue;
        }
        // Their story must have gone on past these seasons: a career at the
        // level that is over, or one further along than his (proposed, not
        // confirmed).
        let person = views::person_index(w, oid).map(|k| &w.people[k]);
        let still = match p.tier {
            Tier::College => {
                person.is_some_and(|x| matches!(x.stage, Stage::College | Stage::PrepSchool | Stage::Away))
            }
            Tier::Nfl => false,
            Tier::Hs => person.is_some_and(|x| x.stage == Stage::HighSchool),
        };
        if (last && still) || (!last && still && seq.len() <= n) {
            continue;
        }
        if drafted && bios.pro(oid).and_then(|c| c.draft).is_none() {
            continue;
        }
        let theirs = take(seq);
        let mut gaps = Vec::with_capacity(n);
        for (a, b) in me.iter().zip(&theirs) {
            match season_gap(at(a), at(b)) {
                Some(g) => gaps.push(g),
                None => break,
            }
        }
        if gaps.len() < n {
            continue;
        }
        let mut gap = gaps.iter().sum::<f64>() / n as f64;
        if let (Some(a), Some(b)) = (my_age, bios.age(oid, f.seasons[theirs[0].0].year)) {
            gap += AGE_GAP * (a as f64 - b as f64).abs();
        }
        found.push((gap, oid, theirs));
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    found.truncate(want);
    let season_json = |&(s, i): &(usize, usize), who: u32| {
        let fs = &f.seasons[s];
        let r = &fs.rows[i];
        json!({
            "year": fs.year,
            "team": team_json(w, p.tier, r.team),
            "class": bios.class(p.tier, who, fs.year).map(|c| class_text(p.tier, c)),
            "age": bios.age(who, fs.year),
            "games": r.games,
            "vol": r.vol.round() as i64,
            "text": r.vals.iter().zip(&f.card).map(|(v, cm)| v.map(|v| ay::fmt_cell(*cm, v))).collect::<Vec<_>>(),
            "pct": fs.pcts[i].iter().map(|x| x.map(|x| x.round() as i64)).collect::<Vec<_>>(),
        })
    };
    let comps: Vec<Value> = found
        .iter()
        .map(|(gap, oid, seasons)| {
            let first = &f.seasons[seasons[0].0];
            let last_s = &f.seasons[seasons[seasons.len() - 1].0];
            json!({
                "id": oid,
                "name": ay::name(w, *oid),
                "linked": views::person_index(w, *oid).is_some(),
                "team": team_json(w, p.tier, first.rows[seasons[0].1].team),
                "years": if first.year == last_s.year { first.year.to_string() } else { format!("{}-{}", first.year, last_s.year) },
                "match": (100.0 - gap).max(0.0).round() / 100.0,
                "seasons": seasons.iter().map(|x| season_json(x, *oid)).collect::<Vec<_>>(),
                "outcome": outcome(w, &bios, *oid),
            })
        })
        .collect();
    json!({
        "id": id,
        "name": ay::name(w, id),
        "level": tier_key(p.tier),
        "division": p.division.map(|d| d.name()),
        "group": an::GROUPS[group],
        "mode": if last { "last" } else { "first" },
        "seasons_n": n,
        "most": mine.len().min(4),
        "volume": f.volume,
        "metrics": f.card.iter().map(|c| metric_json(*c)).collect::<Vec<_>>(),
        "mine": me.iter().map(|x| season_json(x, id)).collect::<Vec<_>>(),
        "comps": comps,
        "since": f.seasons.first().map(|s| s.year),
    })
}
