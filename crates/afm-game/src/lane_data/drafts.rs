//! Drafts, recruiting and the portal by the numbers (lane NB11, Data
//! overhaul items 36-38; boards 48 and 57 in Press Box):
//! - `draft_data` (`tab`, `from`, `to`): every draft's outcomes since the
//!   world's records began, hit rates by round beside the real NFL's, by
//!   position, conference and school, value against the pick slot, each
//!   club's draft record and your own board's record (the look-backs);
//! - `recruit_data` (`tab`, `year`, `division`): class rankings by year on
//!   the composite, blue-chip shares, stars against outcomes, the programs
//!   that develop players and the states' pipelines;
//! - `portal_data` (`year`, `division`): who went in and out by program,
//!   and how transfers did after the move.
//!
//! Read from the classes tally (picks and Division I signees with what
//! became of them), the prospects tally's signees (their last ranking) and
//! the moves tally (portal entries). Outcomes only: nothing here reads a
//! hidden rating.

use std::collections::{BTreeMap, HashMap};

use afm_engine::world::World;
use afm_engine::world::data::Division;
use afm_engine::world::marks;
use afm_engine::world::tally::classes::{Pick, Signee};
use serde_json::{Value, json};

use crate::views::{self, TeamRef};

/// A hit: 32 or more NFL starts (two full seasons; the mock-up's line,
/// proposed, not confirmed).
pub(super) const HIT_STARTS: u16 = 32;
/// A class counts toward hit rates once it has had this many seasons.
const MATURE: i32 = 4;
/// A class counts toward games and Pro Bowls once it has had this many.
const LONG: i32 = 8;

/// The real NFL by round, draft classes 1980-2015 (nflverse; `data/real-
/// world/nfl_careers.json`, `career_by_round`): share with 4+ seasons,
/// share with a Pro Bowl, games a pick, seasons as a starter a pick.
const REAL_ROUNDS: [(u8, f64, f64, f64, f64); 7] = [
    (1, 0.9378, 0.4104, 115.9, 6.07),
    (2, 0.8465, 0.2022, 92.2, 3.96),
    (3, 0.745, 0.1174, 73.3, 2.49),
    (4, 0.644, 0.0663, 62.0, 1.79),
    (5, 0.5494, 0.0627, 49.9, 1.31),
    (6, 0.474, 0.0455, 41.0, 0.94),
    (7, 0.3889, 0.0303, 33.3, 0.74),
];

/// The pick slots value is read against.
const SLOTS: [(u16, u16, &str); 7] = [
    (1, 10, "Picks 1-10"),
    (11, 32, "Picks 11-32"),
    (33, 64, "Picks 33-64"),
    (65, 100, "Picks 65-100"),
    (101, 150, "Picks 101-150"),
    (151, 200, "Picks 151-200"),
    (201, 999, "Picks 201+"),
];

fn slot_of(pick: u16) -> usize {
    SLOTS.iter().position(|s| pick >= s.0 && pick <= s.1).unwrap_or(SLOTS.len() - 1)
}

/// The last season the classes are counted through.
fn through(w: &World) -> i32 {
    let c = w.tally_classes();
    c.picks.iter().map(|p| p.out.through).chain(c.signees.iter().map(|s| s.out.through)).max().unwrap_or(w.year - 1)
}

fn name(w: &World, id: u32) -> String {
    marks::name_of(w, id).unwrap_or_else(|| "a former player".into())
}

fn person(w: &World, id: u32) -> Value {
    json!({ "id": id, "name": name(w, id), "linked": views::person_index(w, id).is_some() })
}

fn college(w: &World, p: u32) -> Value {
    if (p as usize) < w.programs.len() { views::team_brief(w, TeamRef::College(p as usize)) } else { Value::Null }
}

fn club(w: &World, t: u16) -> Value {
    if (t as usize) < w.nfl.len() { views::team_brief(w, TeamRef::Nfl(t as usize)) } else { Value::Null }
}

fn pct(n: usize, d: usize) -> Option<f64> {
    (d > 0).then(|| 100.0 * n as f64 / d as f64)
}

/// Pro Bowls and first-team All-Pros by player.
fn honors(w: &World) -> HashMap<u32, (u32, u32)> {
    let mut m: HashMap<u32, (u32, u32)> = HashMap::new();
    for x in &w.nfl_book.pro_bowls {
        m.entry(x.1).or_default().0 += 1;
    }
    for x in &w.nfl_book.all_pro {
        if x.1 == 1 {
            m.entry(x.2).or_default().1 += 1;
        }
    }
    m
}

/// What a pick did against his slot: NFL starts a season since the draft,
/// against the slot's usual among the classes with four seasons or more
/// (proposed, not confirmed: starts a season, as career lengths differ).
struct Slots {
    rate: [f64; SLOTS.len()],
}

impl Slots {
    fn new(picks: &[Pick], thru: i32) -> Slots {
        let mut s = [(0.0, 0.0); SLOTS.len()];
        for p in picks.iter().filter(|p| thru - p.year + 1 >= MATURE) {
            let seasons = (thru - p.year + 1) as f64;
            let e = &mut s[slot_of(p.pick)];
            e.0 += p.out.nfl.starts as f64 / seasons;
            e.1 += 1.0;
        }
        Slots { rate: s.map(|(a, n)| if n > 0.0 { a / n } else { 0.0 }) }
    }

    /// Starts over his slot's usual since the draft.
    fn over(&self, p: &Pick, thru: i32) -> f64 {
        let seasons = (thru - p.year + 1).max(0) as f64;
        p.out.nfl.starts as f64 - self.rate[slot_of(p.pick)] * seasons
    }
}

/// A group of picks summed.
#[derive(Default)]
struct Sum {
    picks: usize,
    mature: usize,
    hits: usize,
    long: usize,
    games: f64,
    pro_bowls: usize,
    four: usize,
    starts: f64,
    over: f64,
}

impl Sum {
    fn add(&mut self, p: &Pick, thru: i32, slots: &Slots, hon: &HashMap<u32, (u32, u32)>) {
        let age = thru - p.year + 1;
        self.picks += 1;
        self.starts += p.out.nfl.starts as f64;
        self.over += slots.over(p, thru);
        if age >= MATURE {
            self.mature += 1;
            self.hits += (p.out.nfl.starts >= HIT_STARTS) as usize;
            self.four += (p.out.nfl.seasons >= 4) as usize;
        }
        if age >= LONG {
            self.long += 1;
            self.games += p.out.nfl.games as f64;
            self.pro_bowls += (hon.get(&p.id).map_or(0, |h| h.0) > 0) as usize;
        }
    }

    fn json(&self) -> Value {
        json!({
            "picks": self.picks,
            "mature": self.mature,
            "hits": self.hits,
            "hit": pct(self.hits, self.mature),
            "four": pct(self.four, self.mature),
            "games": (self.long > 0).then(|| self.games / self.long as f64),
            "pro_bowl": pct(self.pro_bowls, self.long),
            "starts": (self.picks > 0).then(|| self.starts / self.picks as f64),
            "over": self.over,
            "over_pick": (self.picks > 0).then(|| self.over / self.picks as f64),
        })
    }
}

fn years_arg(args: &Value, all: &[i32]) -> (i32, i32) {
    let lo = all.first().copied().unwrap_or(0);
    let hi = all.last().copied().unwrap_or(0);
    let from = args.get("from").and_then(Value::as_i64).map_or(lo, |x| x as i32).clamp(lo, hi.max(lo));
    let to = args.get("to").and_then(Value::as_i64).map_or(hi, |x| x as i32).clamp(from, hi.max(from));
    (from, to)
}

fn conference_at(w: &World, prog: u32, year: i32) -> String {
    if (prog as usize) >= w.programs.len() {
        return String::new();
    }
    let (d, c) = super::season_place(w, prog as usize, year);
    if c.is_empty() { format!("{} independents", d.name()) } else { c.to_string() }
}

pub fn draft_data(w: &World, args: &Value) -> Value {
    let c = w.tally_classes();
    let thru = through(w);
    let mut years: Vec<i32> = c.picks.iter().map(|p| p.year).collect();
    years.sort_unstable();
    years.dedup();
    if years.is_empty() {
        return json!({ "empty": "No draft has been counted yet: the records start with the world's first recorded season.", "years": years });
    }
    let (from, to) = years_arg(args, &years);
    let picks: Vec<&Pick> = c.picks.iter().filter(|p| p.year >= from && p.year <= to).collect();
    let all: Vec<Pick> = c.picks.clone();
    let slots = Slots::new(&all, thru);
    let hon = honors(w);
    let tab = args.get("tab").and_then(Value::as_str).unwrap_or("rounds");
    let mut by: BTreeMap<String, (Sum, Value)> = BTreeMap::new();
    let key_of = |p: &Pick| -> (String, Value) {
        match tab {
            "positions" => {
                let n = p.pos.map_or("?", |x| x.name()).to_string();
                (n.clone(), json!(n))
            }
            "conferences" => {
                let n = p.program.map_or_else(|| "No college".into(), |x| conference_at(w, x, p.year - 1));
                (n.clone(), json!(n))
            }
            "schools" => match p.program {
                Some(x) => (format!("{x:06}"), college(w, x)),
                None => ("zzz".into(), json!("No college")),
            },
            "clubs" => (format!("{:03}", p.club), club(w, p.club)),
            _ => (format!("{:02}", p.round), json!(p.round)),
        }
    };
    for p in &picks {
        let (k, label) = key_of(p);
        let e = by.entry(k).or_insert_with(|| (Sum::default(), label));
        e.0.add(p, thru, &slots, &hon);
    }
    let mine = match views::mine(w) {
        Some(TeamRef::Nfl(t)) => Some(t as u16),
        _ => None,
    };
    let mut rows: Vec<Value> = by
        .into_iter()
        .map(|(_, (s, label))| {
            let mut v = s.json();
            v["label"] = label;
            v
        })
        .collect();
    if tab != "rounds" {
        rows.sort_by(|a, b| {
            let f = |x: &Value| x["picks"].as_u64().unwrap_or(0);
            f(b).cmp(&f(a))
        });
    }
    // The real NFL beside each round.
    if tab == "rounds" {
        for r in rows.iter_mut() {
            let round = r["label"].as_u64().unwrap_or(0) as u8;
            if let Some(x) = REAL_ROUNDS.iter().find(|x| x.0 == round) {
                r["real"] =
                    json!({ "four": 100.0 * x.1, "pro_bowl": 100.0 * x.2, "games": x.3, "seasons_started": x.4 });
            }
        }
    }
    // Your club's line beside the rounds (an NFL seat).
    let your_rounds: Option<Vec<Value>> = mine.map(|t| {
        (1..=7u8)
            .map(|r| {
                let mut s = Sum::default();
                for p in picks.iter().filter(|p| p.club == t && p.round == r) {
                    s.add(p, thru, &slots, &hon);
                }
                let mut v = s.json();
                v["label"] = json!(r);
                v
            })
            .collect()
    });
    // The best and worst against their slot.
    let mut judged: Vec<(&&Pick, f64)> =
        picks.iter().filter(|p| thru - p.year + 1 >= MATURE).map(|p| (p, slots.over(p, thru))).collect();
    judged.sort_by(|a, b| b.1.total_cmp(&a.1));
    let pick_json = |p: &Pick, over: f64| {
        let h = hon.get(&p.id).copied().unwrap_or_default();
        json!({
            "player": person(w, p.id), "year": p.year, "round": p.round, "pick": p.pick,
            "pos": p.pos.map(|x| x.name()), "club": club(w, p.club),
            "school": p.program.map_or(Value::Null, |x| college(w, x)),
            "games": p.out.nfl.games, "starts": p.out.nfl.starts, "seasons": p.out.nfl.seasons,
            "pro_bowls": h.0, "all_pro": h.1, "over": over,
        })
    };
    let steals: Vec<Value> = judged.iter().take(10).map(|(p, o)| pick_json(p, *o)).collect();
    let busts: Vec<Value> = judged.iter().rev().take(10).map(|(p, o)| pick_json(p, *o)).collect();
    let slot_rows: Vec<Value> =
        SLOTS.iter().enumerate().map(|(k, s)| json!({ "label": s.2, "rate": slots.rate[k] })).collect();
    json!({
        "tab": tab,
        "years": years,
        "from": from,
        "to": to,
        "through": thru,
        "hit_starts": HIT_STARTS,
        "mature": MATURE,
        "long": LONG,
        "rows": rows,
        "yours": your_rounds,
        "slots": slot_rows,
        "steals": steals,
        "busts": busts,
        "board": board_record(w),
        "mine": mine.map(|t| club(w, t)),
    })
}

/// Your war room's board against the draft (the look-backs): the players
/// you ranked well above where they went and what came of them, and how
/// your order held up three years on against the public's.
fn board_record(w: &World) -> Value {
    let recs = &w.scouting.records.drafts;
    if recs.is_empty() {
        return Value::Null;
    }
    let mut above: Vec<&afm_engine::world::scouting::lookback::Graded> =
        recs.iter().filter(|g| g.pick.is_some_and(|p| p as i64 - g.ours as i64 >= 30)).collect();
    above.sort_by_key(|g| std::cmp::Reverse(g.pick.unwrap_or(0) as i64 - g.ours as i64));
    let rows: Vec<Value> = above
        .iter()
        .take(12)
        .map(|g| {
            json!({
                "player": person(w, g.id), "year": g.year, "pos": g.pos.name(), "ours": g.ours,
                "pick": g.pick, "outcome": g.outcome,
            })
        })
        .collect();
    // Spearman's rank correlation of an order against the outcome order,
    // over the graded players with an outcome.
    let rho = |pairs: &[(f64, f64)]| -> Option<f64> {
        let n = pairs.len() as f64;
        if n < 10.0 {
            return None;
        }
        let rank = |v: Vec<f64>| -> Vec<f64> {
            let mut idx: Vec<usize> = (0..v.len()).collect();
            idx.sort_by(|&a, &b| v[a].total_cmp(&v[b]));
            let mut r = vec![0.0; v.len()];
            for (k, &i) in idx.iter().enumerate() {
                r[i] = k as f64;
            }
            r
        };
        let a = rank(pairs.iter().map(|x| x.0).collect());
        let b = rank(pairs.iter().map(|x| x.1).collect());
        let d2: f64 = a.iter().zip(&b).map(|(x, y)| (x - y) * (x - y)).sum();
        Some(1.0 - 6.0 * d2 / (n * (n * n - 1.0)))
    };
    let ours: Vec<(f64, f64)> =
        recs.iter().filter_map(|g| Some((g.ours as f64, g.outcome? as f64))).filter(|x| x.0 <= 100.0).collect();
    let public: Vec<(f64, f64)> =
        recs.iter().filter_map(|g| Some((g.public? as f64, g.outcome? as f64))).filter(|x| x.0 <= 100.0).collect();
    json!({ "above": rows, "ours": rho(&ours), "public": rho(&public), "graded": recs.len(), "judged": ours.len() })
}

// ---------------------------------------------------------------------
// Recruiting.

/// A composite rating for a signee: his last ranking's, else his stars'
/// band middle (older classes counted before the prospects tally).
fn rating_of(stars: u8, last: Option<f64>) -> f64 {
    last.unwrap_or(match stars {
        5 => 0.99,
        4 => 0.93,
        3 => 0.85,
        2 => 0.76,
        _ => 0.70,
    })
}

/// 247Sports' team score: each signee's rating x 100, weighted by his
/// place in the class (best first) on a bell curve, so depth counts but
/// the top matters most (proposed, not confirmed: the curve's spread of
/// 10 places, as 247 describes its Gaussian weighting).
fn team_score(mut ratings: Vec<f64>) -> f64 {
    ratings.sort_by(|a, b| b.total_cmp(a));
    ratings.iter().enumerate().map(|(k, r)| 100.0 * r * (-((k as f64) * (k as f64)) / (2.0 * 10.0 * 10.0)).exp()).sum()
}

fn division_arg(w: &World, args: &Value) -> Division {
    super::division_arg(w, args)
}

/// The signees' last rankings by id, from every closed season's prospects
/// file.
fn signed_ratings(w: &World) -> HashMap<u32, f64> {
    let mut m = HashMap::new();
    for s in w.tally_seasons() {
        if s >= w.tally_open_season() {
            continue;
        }
        for x in w.tally_signed(s) {
            if let Some(r) = x.last {
                m.insert(x.id, r.rating as f64 / 10_000.0);
            }
        }
    }
    m
}

pub fn recruit_data(w: &World, args: &Value) -> Value {
    let c = w.tally_classes();
    let thru = through(w);
    let division = division_arg(w, args);
    let signees: Vec<&Signee> = c.signees.iter().filter(|s| s.division == Some(division)).collect();
    let mut years: Vec<i32> = signees.iter().map(|s| s.class).collect();
    years.sort_unstable();
    years.dedup();
    let tab = args.get("tab").and_then(Value::as_str).unwrap_or("classes");
    let divisions: Vec<&str> = [Division::FBS, Division::FCS].iter().map(|d| d.name()).collect();
    if years.is_empty() {
        return json!({ "empty": "No recruiting class has been counted yet.", "tab": tab, "division": division.name(), "divisions": divisions });
    }
    let year = args
        .get("year")
        .and_then(Value::as_i64)
        .map(|y| y as i32)
        .filter(|y| years.contains(y))
        .unwrap_or(*years.last().unwrap());
    let ratings = signed_ratings(w);
    let mine = match views::mine(w) {
        Some(TeamRef::College(p)) => Some(p as u32),
        _ => None,
    };
    // Drafted from each class: picks keyed by id.
    let drafted: HashMap<u32, &Pick> = c.picks.iter().map(|p| (p.id, p)).collect();
    let moves = &w.tally_moves().moves;
    let mut moved_down: HashMap<u32, bool> = HashMap::new();
    for m in moves {
        let down = match (m.from_division, m.to_division) {
            (Some(a), Some(b)) => (b as u8) > (a as u8),
            _ => false,
        };
        let e = moved_down.entry(m.id).or_insert(false);
        *e |= down;
    }
    let out = match tab {
        "stars" => {
            // Stars against outcomes, classes old enough to have finished.
            let mut by: BTreeMap<u8, [usize; 6]> = BTreeMap::new();
            for s in signees.iter().filter(|s| thru - s.class >= 4) {
                let e = by.entry(s.stars).or_default();
                e[0] += 1;
                let d = drafted.get(&s.id);
                e[1] += d.is_some() as usize;
                e[2] += d.is_some_and(|p| p.round == 1) as usize;
                e[3] += (s.out.college.starts >= 12) as usize;
                e[4] += (s.out.transfers > 0) as usize;
                e[5] += moved_down.get(&s.id).copied().unwrap_or(false) as usize;
            }
            let rows: Vec<Value> = by
                .iter()
                .rev()
                .map(|(st, e)| {
                    json!({
                        "stars": st, "signees": e[0], "drafted": pct(e[1], e[0]), "first_round": pct(e[2], e[0]),
                        "started": pct(e[3], e[0]), "transferred": pct(e[4], e[0]), "down": pct(e[5], e[0]),
                    })
                })
                .collect();
            // The same for your program beside.
            let yours: Vec<Value> = match mine {
                Some(m) => by
                    .keys()
                    .rev()
                    .map(|&st| {
                        let v: Vec<&&Signee> =
                            signees.iter().filter(|s| s.program == m && s.stars == st && thru - s.class >= 4).collect();
                        let d = v.iter().filter(|s| drafted.contains_key(&s.id)).count();
                        json!({ "stars": st, "signees": v.len(), "drafted": pct(d, v.len()) })
                    })
                    .collect(),
                None => Vec::new(),
            };
            json!({ "stars": rows, "yours": yours })
        }
        "develop" => {
            // Draft picks over what their stars predicted (the world's own
            // draft rate by stars), classes with four seasons or more.
            let old: Vec<&&Signee> = signees.iter().filter(|s| thru - s.class >= 4).collect();
            let mut rate: HashMap<u8, (f64, f64)> = HashMap::new();
            for s in &old {
                let e = rate.entry(s.stars).or_default();
                e.0 += drafted.contains_key(&s.id) as u8 as f64;
                e.1 += 1.0;
            }
            let mut by: HashMap<u32, (f64, f64, usize)> = HashMap::new();
            for s in &old {
                let (d, n) = rate.get(&s.stars).copied().unwrap_or((0.0, 1.0));
                let e = by.entry(s.program).or_default();
                e.0 += drafted.contains_key(&s.id) as u8 as f64;
                e.1 += d / n.max(1.0);
                e.2 += 1;
            }
            let mut rows: Vec<(u32, (f64, f64, usize))> = by.into_iter().filter(|x| x.1.2 >= 20).collect();
            rows.sort_by(|a, b| (b.1.0 - b.1.1).total_cmp(&(a.1.0 - a.1.1)));
            let row = |(p, (d, e, n)): &(u32, (f64, f64, usize))| json!({ "team": college(w, *p), "conference": conference_at(w, *p, thru), "signees": n, "picks": d, "expected": e, "over": d - e, "mine": Some(*p) == mine });
            let top: Vec<Value> = rows.iter().take(15).map(row).collect();
            let bottom: Vec<Value> = rows.iter().rev().take(8).map(row).collect();
            let you = mine.and_then(|m| rows.iter().position(|r| r.0 == m).map(|k| (k + 1, row(&rows[k]))));
            json!({ "develop": top, "least": bottom, "yours": you.map(|x| json!({ "rank": x.0, "of": rows.len(), "row": x.1 })) })
        }
        "pipelines" => {
            // Each state's signees and where they went, the last 5 classes.
            let recent: Vec<&&Signee> = signees.iter().filter(|s| s.class > year - 5 && s.class <= year).collect();
            let mut by: BTreeMap<u16, (usize, usize, HashMap<u32, usize>)> = BTreeMap::new();
            for s in &recent {
                let e = by.entry(s.home_state).or_default();
                e.0 += 1;
                let prog_state = w.programs.get(s.program as usize).map(|p| p.state as u16);
                e.1 += (prog_state == Some(s.home_state)) as usize;
                *e.2.entry(s.program).or_default() += 1;
            }
            let mut rows: Vec<Value> = by
                .into_iter()
                .map(|(st, (n, home, progs))| {
                    let mut top: Vec<(u32, usize)> = progs.into_iter().collect();
                    top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                    let code = w.states.get(st as usize).map_or("?", |s| s.code.as_str());
                    json!({
                        "state": code, "signees": n, "stay": pct(home, n),
                        "top": top.iter().take(3).map(|(p, k)| json!({ "team": college(w, *p), "n": k })).collect::<Vec<_>>(),
                        "yours": mine.map(|m| top.iter().find(|x| x.0 == m).map_or(0, |x| x.1)),
                    })
                })
                .collect();
            rows.sort_by(|a, b| b["signees"].as_u64().cmp(&a["signees"].as_u64()));
            json!({ "pipelines": rows, "span": [year - 4, year] })
        }
        _ => {
            // Class rankings for one class year.
            let mut by: HashMap<u32, Vec<&&Signee>> = HashMap::new();
            for s in signees.iter().filter(|s| s.class == year) {
                by.entry(s.program).or_default().push(s);
            }
            // Blue-chip share: 4 and 5 stars of the last four classes.
            let mut blue: HashMap<u32, (usize, usize)> = HashMap::new();
            for s in signees.iter().filter(|s| s.class > year - 4 && s.class <= year) {
                let e = blue.entry(s.program).or_default();
                e.0 += (s.stars >= 4) as usize;
                e.1 += 1;
            }
            let mut rows: Vec<(f64, Value)> = by
                .into_iter()
                .map(|(p, v)| {
                    let rs: Vec<f64> = v.iter().map(|s| rating_of(s.stars, ratings.get(&s.id).copied())).collect();
                    let avg = rs.iter().sum::<f64>() / rs.len().max(1) as f64;
                    let score = team_score(rs);
                    let count = |k: u8| v.iter().filter(|s| s.stars == k).count();
                    let b = blue.get(&p).copied().unwrap_or((0, 0));
                    (
                        score,
                        json!({
                            "team": college(w, p), "conference": conference_at(w, p, year), "signees": v.len(),
                            "five": count(5), "four": count(4), "three": count(3), "avg": avg, "score": score,
                            "blue": pct(b.0, b.1), "mine": Some(p) == mine,
                        }),
                    )
                })
                .collect();
            rows.sort_by(|a, b| b.0.total_cmp(&a.0));
            let ranked: Vec<Value> = rows
                .into_iter()
                .enumerate()
                .map(|(k, (_, mut v))| {
                    v["rank"] = json!(k + 1);
                    v
                })
                .collect();
            // Your class's members.
            let members: Vec<Value> = match mine {
                Some(m) => signees
                    .iter()
                    .filter(|s| s.class == year && s.program == m)
                    .map(|s| {
                        json!({ "player": person(w, s.id), "stars": s.stars, "rating": ratings.get(&s.id), "pos": s.pos.map(|p| p.name()),
                            "state": w.states.get(s.home_state as usize).map(|x| x.code.as_str()) })
                    })
                    .collect(),
                None => Vec::new(),
            };
            json!({ "classes": ranked, "members": members })
        }
    };
    let mut m = out;
    m["tab"] = json!(tab);
    m["years"] = json!(years);
    m["year"] = json!(year);
    m["through"] = json!(thru);
    m["division"] = json!(division.name());
    m["divisions"] = json!(divisions);
    m
}

// ---------------------------------------------------------------------
// The portal.

pub fn portal_data(w: &World, args: &Value) -> Value {
    let all = &w.tally_moves().moves;
    let division = division_arg(w, args);
    let mut years: Vec<i32> = all.iter().map(|m| m.year).collect();
    years.sort_unstable();
    years.dedup();
    let divisions: Vec<&str> = Division::ALL.iter().filter(|d| **d != Division::JUCO).map(|d| d.name()).collect();
    if years.is_empty() {
        return json!({ "empty": "No portal move has been counted yet.", "division": division.name(), "divisions": divisions });
    }
    let year = args
        .get("year")
        .and_then(Value::as_i64)
        .map(|y| y as i32)
        .filter(|y| years.contains(y))
        .unwrap_or(*years.last().unwrap());
    let thru = through(w);
    let mine = match views::mine(w) {
        Some(TeamRef::College(p)) => Some(p as u32),
        _ => None,
    };
    let moves: Vec<_> = all.iter().filter(|m| m.year == year).collect();
    // By program: in, out, and the newcomers who started six games or more
    // in their first season there.
    let mut by: HashMap<u32, [usize; 4]> = HashMap::new();
    for m in &moves {
        if m.from_division == Some(division) {
            by.entry(m.from).or_default()[1] += 1;
        }
        if let Some(t) = m.to
            && m.to_division == Some(division)
        {
            let e = by.entry(t).or_default();
            e[0] += 1;
            if m.after.seasons > 0 {
                e[2] += 1;
                e[3] += (m.after.starts >= 6) as usize;
            }
        }
    }
    let mut rows: Vec<Value> = by
        .into_iter()
        .map(|(p, e)| {
            json!({ "team": college(w, p), "in": e[0], "out": e[1], "net": e[0] as i64 - e[1] as i64, "played": e[2], "started": e[3], "mine": Some(p) == mine })
        })
        .collect();
    rows.sort_by(|a, b| {
        let f = |x: &Value| x["in"].as_i64().unwrap_or(0) + x["out"].as_i64().unwrap_or(0);
        f(b).cmp(&f(a))
    });
    // How transfers did: first season at the new school against the last
    // at the old, by the move's direction.
    let mut dirs: BTreeMap<&str, [f64; 6]> = BTreeMap::new();
    let mut whys: BTreeMap<String, [usize; 3]> = BTreeMap::new();
    for m in moves.iter().filter(|m| m.from_division == Some(division)) {
        let dir = match (m.from_division, m.to_division) {
            _ if m.returned => "went back",
            (Some(a), Some(b)) if (b as u8) < (a as u8) => "moved up",
            (Some(a), Some(b)) if (b as u8) > (a as u8) => "moved down",
            (Some(_), Some(_)) => "same level",
            _ => "found no school",
        };
        let e = dirs.entry(dir).or_default();
        e[0] += 1.0;
        e[1] += m.before_games as f64;
        if m.after.seasons > 0 {
            e[2] += 1.0;
            e[3] += m.after.games as f64 / m.after.seasons as f64;
            e[4] += m.after.starts as f64 / m.after.seasons as f64;
            e[5] += (m.after.starts as f64 / m.after.seasons as f64 >= 6.0) as u8 as f64;
        }
        let y = whys.entry(m.why.clone()).or_default();
        y[0] += 1;
        y[1] += m.to.is_some() as usize;
        y[2] += (m.after.starts >= 6) as usize;
    }
    let dir_rows: Vec<Value> = dirs
        .into_iter()
        .map(|(d, e)| {
            let n = e[0].max(1.0);
            let p = e[2].max(1.0);
            json!({ "dir": d, "n": e[0], "before_games": e[1] / n, "played": e[2], "games": (e[2] > 0.0).then(|| e[3] / p), "starts": (e[2] > 0.0).then(|| e[4] / p), "starters": (e[2] > 0.0).then(|| 100.0 * e[5] / p) })
        })
        .collect();
    let why_rows: Vec<Value> = whys
        .into_iter()
        .map(|(why, e)| json!({ "why": why, "n": e[0], "landed": pct(e[1], e[0]), "started": pct(e[2], e[0]) }))
        .collect();
    // Your program's moves that year.
    let yours: Vec<Value> = match mine {
        Some(p) => moves
            .iter()
            .filter(|m| m.from == p || m.to == Some(p))
            .map(|m| {
                json!({
                    "player": person(w, m.id), "pos": m.pos.map(|x| x.name()), "stars": m.stars,
                    "in": m.to == Some(p), "from": college(w, m.from), "to": m.to.map_or(Value::Null, |t| college(w, t)),
                    "why": m.why, "role": m.role, "before_games": m.before_games,
                    "after": json!({ "seasons": m.after.seasons, "games": m.after.games, "starts": m.after.starts }),
                })
            })
            .collect(),
        None => Vec::new(),
    };
    json!({
        "year": year, "years": years, "through": thru, "division": division.name(), "divisions": divisions,
        "entries": moves.iter().filter(|m| m.from_division == Some(division)).count(),
        "programs": rows, "dirs": dir_rows, "whys": why_rows, "yours": yours,
    })
}
