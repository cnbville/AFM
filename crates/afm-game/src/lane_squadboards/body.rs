//! V6, body and testing: his measurements and test numbers with where each
//! came from (your staff's own measuring and testing days, the combine or a
//! pro day, a camp your staff saw: verified; a school's or team's listing:
//! self-reported), each against his position at his level, against the
//! world's drafted players at his position, and against the real combine
//! since 2000; an athletic score; the picked test against weight; past
//! players built like him and what became of them; his weight over the
//! seasons your staff kept.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use afm_engine::body::{Combine, body_now};
use afm_engine::player::Player;
use afm_engine::positions::Position;
use afm_engine::world::scouting::Kind;
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use super::first_last;
use crate::Reply;
use crate::views::{self, TeamRef};

const COMBINE_CSV: &str = include_str!("../../../../data/real-world/combine_by_position.csv");

/// The tests on the page: key, label, lower is better, the combine CSV's
/// mean column (None: the real data doesn't have it).
const TESTS: [(&str, &str, bool, Option<usize>); 12] = [
    ("ht", "Height", false, Some(2)),
    ("wt", "Weight", false, Some(4)),
    ("arm", "Arms", false, None),
    ("hand", "Hands", false, None),
    ("wing", "Wingspan", false, None),
    ("forty", "40 yards", true, Some(6)),
    ("split", "10-yard split", true, None),
    ("vertical", "Vertical", false, Some(10)),
    ("broad", "Broad jump", false, Some(12)),
    ("cone", "3-cone", true, Some(14)),
    ("shuttle", "Shuttle", true, Some(16)),
    ("reps", "Bench (225)", false, Some(8)),
];

/// Each drill's mean and spread by position, `None` where too few ran it.
type Spreads = HashMap<Position, Vec<Option<(f64, f64)>>>;

/// The real combine's mean and spread by position (nflverse 2000-2025).
static REAL: LazyLock<Spreads> = LazyLock::new(|| {
    let mut out = HashMap::new();
    for line in COMBINE_CSV.lines().skip(1) {
        let f: Vec<&str> = line.split(',').collect();
        let Some(&pos) = Position::ALL.iter().find(|p| p.name() == f[0]) else { continue };
        let num = |k: usize| f.get(k).and_then(|s| s.parse::<f64>().ok());
        let v: Vec<Option<(f64, f64)>> =
            TESTS.iter().map(|t| t.3.and_then(|c| num(c).zip(num(c + 1)).filter(|x| x.1 > 0.0))).collect();
        out.insert(pos, v);
    }
    out
});

/// One test's number for a man.
fn values(p: &Player, c: &Combine) -> [Option<f64>; 12] {
    let b = body_now(p);
    [
        Some(b.height),
        Some(b.weight),
        Some(b.arm),
        Some(b.hand),
        Some(b.wingspan),
        Some(c.forty),
        None,
        Some(c.vertical),
        Some(c.broad),
        Some(c.cone),
        Some(c.shuttle),
        Some(c.bench),
    ]
}

/// A man in a comparison pool, at what is known of him (minor bug 30:
/// his listing, and a test only where someone timed or measured him).
#[derive(Clone)]
struct Member {
    vals: [Option<f64>; 12],
    /// Drafted in the first round.
    first: bool,
    name: String,
    id: Option<u32>,
    school: String,
}

/// Where a man plays now, for a dot's label.
fn team_words(w: &World, q: &afm_engine::world::Person) -> String {
    match q.stage {
        Stage::HighSchool | Stage::PrepSchool => w.school_name(q.hs).to_string(),
        Stage::College | Stage::Declared => q.college.map_or(String::new(), |c| w.program_name(c).to_string()),
        _ => q.pro.as_ref().and_then(|c| c.team).map_or(String::new(), |t| w.nfl_team_name(t)),
    }
}

/// A past player built like him (the drafted pool keeps who they were).
#[derive(Clone)]
struct Past {
    vals: [Option<f64>; 12],
    name: String,
    id: Option<u32>,
    school: String,
    year: i32,
    pick: Option<u32>,
    round: Option<u32>,
    career: String,
}

type PoolKey = (u32, i32, String, Position);
type Pools = (Arc<Vec<Member>>, Arc<Vec<Past>>);

/// His position at his level: high schoolers by state and grade (the
/// nation's grade when the state is thin), college by division, the NFL.
fn level_pool(w: &World, i: usize) -> (String, Vec<Member>) {
    let p = &w.people[i];
    let pos = p.player.position;
    let (_, many) = crate::lane_ratings::pos_words(pos);
    let collect = |keep: &dyn Fn(&afm_engine::world::Person) -> bool| -> Vec<Member> {
        w.people
            .iter()
            .enumerate()
            .filter(|(_, q)| q.player.position == pos && keep(q))
            .map(|(j, q)| Member {
                vals: known(w, j).vals,
                first: false,
                name: q.name.clone(),
                id: Some(q.player.id),
                school: team_words(w, q),
            })
            .collect()
    };
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => {
            let st = w.high_schools.get(p.hs).map(|s| s.state);
            let grade = crate::views::class_text(w, p).to_lowercase();
            let in_state = collect(&|q| {
                matches!(q.stage, Stage::HighSchool | Stage::PrepSchool)
                    && q.class_of == p.class_of
                    && w.high_schools.get(q.hs).map(|s| s.state) == st
            });
            let grade_word = match grade.as_str() {
                "sr" => "senior",
                "jr" => "junior",
                "so" => "sophomore",
                _ => "freshman",
            };
            if in_state.len() >= 40 {
                let s = st.map_or("", |s| views::state_name(&w.states[s].code));
                (format!("{s} {grade_word} {many}"), in_state)
            } else {
                let all =
                    collect(&|q| matches!(q.stage, Stage::HighSchool | Stage::PrepSchool) && q.class_of == p.class_of);
                (format!("high school {grade_word} {many}"), all)
            }
        }
        Stage::College | Stage::Declared => {
            let tier = p.college.map_or(0, |c| afm_engine::world::norms::tier(w.programs[c].division));
            let label = ["FBS", "FCS", "small-college"][tier.min(2) as usize];
            let v = collect(&|q| {
                matches!(q.stage, Stage::College)
                    && q.college.is_some_and(|c| afm_engine::world::norms::tier(w.programs[c].division) == tier)
            });
            (format!("{label} {many}"), v)
        }
        _ => {
            let v = collect(&|q| q.stage == Stage::Pro && q.pro.as_ref().is_some_and(|c| c.team.is_some()));
            (format!("NFL {many}"), v)
        }
    }
}

/// A pro career in a few words.
fn career_words(c: Option<&afm_engine::world::pro::ProCareer>) -> String {
    let Some(c) = c else { return "Never played in the NFL".into() };
    let played: Vec<_> = c.seasons.iter().filter(|s| s.games > 0).collect();
    if played.is_empty() {
        return "Never played a regular-season game".into();
    }
    let n = played.len();
    let bowls = played.iter().filter(|s| s.pro_bowl).count();
    let starts: u32 = played.iter().map(|s| s.starts).sum();
    let years = format!("{n} season{}", if n == 1 { "" } else { "s" });
    if bowls > 0 {
        format!("{years}, {bowls} Pro Bowl{}", if bowls == 1 { "" } else { "s" })
    } else if starts >= 16 {
        format!("{years}, {starts} starts")
    } else {
        format!("{years}, mostly a backup")
    }
}

/// The world's drafted players at a position (every one it remembers in
/// full: drafted alumni and today's drafted pros), with their combine.
fn drafted_pool(w: &World, pos: Position) -> Vec<Past> {
    let mut out: Vec<Past> = Vec::new();
    // A man playing now is read from his live record below.
    let live = super::arc::live_pros(w);
    for a in &w.alumni {
        let (Some(pl), Some(pick)) = (a.player.as_ref(), a.pick) else { continue };
        if a.position != pos || live.contains(&(a.name.as_str(), a.class_of)) {
            continue;
        }
        let c = afm_engine::body::combine(pl, &mut afm_engine::rng::Rng::new(pl.id ^ 0x00C0_FFEE));
        let round = a.pro.as_ref().and_then(|c| c.draft).map(|d| d.1);
        out.push(Past {
            vals: values(pl, &c),
            name: a.name.clone(),
            id: None,
            school: a.colleges.last().map_or(String::new(), |&c| w.program_name(c).to_string()),
            year: a.year,
            pick: Some(pick),
            round: round.or(Some((pick - 1) / 32 + 1)),
            career: career_words(a.pro.as_deref()),
        });
    }
    for (j, q) in w.people.iter().enumerate().filter(|(_, q)| q.player.position == pos && q.stage == Stage::Pro) {
        let Some(d) = q.pro.as_ref().and_then(|c| c.draft) else { continue };
        out.push(Past {
            vals: known(w, j).vals,
            name: q.name.clone(),
            id: Some(q.player.id),
            school: q.college.map_or(String::new(), |c| w.program_name(c).to_string()),
            year: d.0,
            pick: Some(d.2),
            round: Some(d.1),
            career: career_words(q.pro.as_deref()),
        });
    }
    out
}

/// The two pools for a player, kept for the day.
fn pools(w: &World, i: usize) -> (String, Pools) {
    static CACHE: Mutex<Option<HashMap<PoolKey, (String, Pools)>>> = Mutex::new(None);
    let p = &w.people[i];
    let pos = p.player.position;
    let tag = match p.stage {
        Stage::HighSchool | Stage::PrepSchool => {
            format!("hs{}-{}", w.high_schools.get(p.hs).map_or(0, |s| s.state), p.class_of)
        }
        Stage::College | Stage::Declared => {
            format!("c{}", p.college.map_or(0, |c| afm_engine::world::norms::tier(w.programs[c].division)))
        }
        _ => "nfl".into(),
    };
    let key: PoolKey = (w.options.seed, w.today.0, tag, pos);
    let mut g = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let map = g.get_or_insert_with(HashMap::new);
    map.retain(|k, _| k.0 == key.0 && k.1 == key.1);
    if let Some(v) = map.get(&key) {
        return v.clone();
    }
    let (label, level) = level_pool(w, i);
    let drafted = drafted_pool(w, pos);
    let v = (label, (Arc::new(level), Arc::new(drafted)));
    map.insert(key, v.clone());
    v
}

/// The share of a pool his number beats (0-100), better to the high side.
fn pct_in(pool: &[f64], v: f64, lower: bool) -> f64 {
    if pool.is_empty() {
        return 50.0;
    }
    let below = pool.iter().filter(|&&x| if lower { x > v } else { x < v }).count() as f64;
    let same = pool.iter().filter(|&&x| (x - v).abs() < 1e-9).count() as f64;
    (100.0 * (below + same / 2.0) / pool.len() as f64).clamp(0.0, 100.0)
}

/// The normal curve's share below z.
fn phi(z: f64) -> f64 {
    // Abramowitz and Stegun 7.1.26, good to about 1e-7.
    let t = 1.0 / (1.0 + 0.3275911 * z.abs() / std::f64::consts::SQRT_2);
    let y = 1.0
        - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t + 0.254829592)
            * t
            * (-(z * z) / 2.0).exp();
    if z >= 0.0 { 0.5 * (1.0 + y) } else { 0.5 * (1.0 - y) }
}

/// His measurements as the game knows them, with where each came from.
struct Known {
    vals: [Option<f64>; 12],
    lifts: Vec<(&'static str, String)>,
    source: [Option<(String, bool)>; 12],
}

fn known(w: &World, i: usize) -> Known {
    let p = &w.people[i];
    let id = p.player.id;
    let mut vals = [None; 12];
    let mut source: [Option<(String, bool)>; 12] = Default::default();
    let mut lifts = Vec::new();
    let team = views::team_of(w, p);
    let own = team.is_some_and(|t| views::is_mine(w, t));
    let b = body_now(&p.player);
    let set = |vals: &mut [Option<f64>; 12],
               source: &mut [Option<(String, bool)>; 12],
               k: usize,
               v: f64,
               s: &str,
               ver: bool| {
        if vals[k].is_none() {
            vals[k] = Some(v);
            source[k] = Some((s.to_string(), ver));
        }
    };
    let pro_side = matches!(p.stage, Stage::College | Stage::Declared | Stage::Pro | Stage::Done);
    // Your own staff measures and tests its players.
    if own {
        let hs = matches!(team, Some(TeamRef::HighSchool(_)));
        set(&mut vals, &mut source, 0, b.height, "Your staff's measurements", true);
        set(&mut vals, &mut source, 1, b.weight, "Your staff's measurements", true);
        if !hs {
            set(&mut vals, &mut source, 2, b.arm, "Your staff's measurements", true);
            set(&mut vals, &mut source, 3, b.hand, "Your staff's measurements", true);
            set(&mut vals, &mut source, 4, b.wingspan, "Your staff's measurements", true);
        }
        if let Some((day, r)) =
            w.training.tests.iter().rev().find_map(|d| d.rows.iter().find(|r| r.id == id).map(|r| (d, *r)))
        {
            let s = format!("Your strength staff, {}", views::date_text(day.date));
            set(&mut vals, &mut source, 5, r.forty as f64 / 100.0, &s, true);
            set(&mut vals, &mut source, 6, r.split as f64 / 100.0, &s, true);
            set(&mut vals, &mut source, 7, r.vertical as f64 / 2.0, &s, true);
            set(&mut vals, &mut source, 10, r.shuttle as f64 / 100.0, &s, true);
            set(&mut vals, &mut source, 11, r.reps as f64, &s, true);
            lifts.push(("Bench press", format!("{} lb", r.bench)));
            lifts.push(("Squat", format!("{} lb", r.squat)));
            lifts.push(("Power clean", format!("{} lb", r.clean)));
        }
    }
    // The combine or a pro day.
    let tested = matches!(p.stage, Stage::Declared | Stage::Pro | Stage::Done)
        && (p.cal.combine_invite || p.pro.as_ref().is_some_and(|c| c.draft.is_some()));
    if tested {
        let c = afm_engine::body::combine(&p.player, &mut afm_engine::rng::Rng::new(id ^ 0x00C0_FFEE));
        // A drafted man the combine didn't invite tested at his pro day.
        let s = if p.cal.combine_invite { "NFL combine" } else { "Pro day" };
        for (k, v) in [(0, b.height), (1, b.weight), (2, b.arm), (3, b.hand), (4, b.wingspan)] {
            set(&mut vals, &mut source, k, v, s, true);
        }
        for (k, v) in [(5, c.forty), (7, c.vertical), (8, c.broad), (9, c.cone), (10, c.shuttle), (11, c.bench)] {
            set(&mut vals, &mut source, k, v, s, true);
        }
    }
    // His laser times as he ran them (speed update S6e, minor bug 23),
    // public once he has run them.
    {
        use afm_engine::world::times::{At, Event, Timed};
        for (k, ev) in [(5, Event::Forty), (6, Event::Split)] {
            let best = w
                .times
                .of(id)
                .iter()
                .filter(|t| t.event == ev && t.timed == Timed::Laser)
                .min_by(|a, b| a.secs.total_cmp(&b.secs));
            if let Some(t) = best {
                let place = match t.at {
                    At::Camp(j) => format!("{} camp", w.program_name(j as usize)),
                    At::National => "National showcase finals".to_string(),
                    _ => "Regional showcase".to_string(),
                };
                let s = format!("Laser, {place}, {}", views::date_text(t.date));
                set(&mut vals, &mut source, k, t.secs as f64, &s, true);
            }
        }
        // Your own high school's hand-timed forty is your staff's own
        // watch (S6c): shown to you, not counted as verified.
        if own && matches!(team, Some(TeamRef::HighSchool(_))) {
            let hand = w.times.of(id).iter().rev().find(|t| t.event == Event::Forty && t.at == At::School);
            if let Some(t) = hand {
                let s = format!("Your staff's hand time, {}", views::date_text(t.date));
                set(&mut vals, &mut source, 5, t.secs as f64, &s, false);
            }
        }
    }
    // A camp, showcase or pro day your scouts saw.
    let mut looks: Vec<(Kind, afm_engine::calendar::Date)> =
        w.scouting.dept.looks.get(&id).map(|v| v.iter().map(|l| (l.kind, l.date)).collect()).unwrap_or_default();
    looks.retain(|l| matches!(l.0, Kind::Camp | Kind::Showcase | Kind::ProDay | Kind::Combine));
    looks.sort_by_key(|l| std::cmp::Reverse(l.1));
    if let Some(&(kind, date)) = looks.first() {
        let c = afm_engine::world::scouting::file::tests(&p.player, kind, date.year());
        let s = format!("{} your scouts saw, {}", kind.name(), views::date_text(date));
        for (k, v) in [(5, c.forty), (7, c.vertical), (8, c.broad), (9, c.cone), (10, c.shuttle), (11, c.bench)] {
            set(&mut vals, &mut source, k, v, &s, true);
        }
    }
    // What a school or team lists him at.
    match p.stage {
        Stage::Pro => {
            set(&mut vals, &mut source, 0, b.height, "Team-listed", false);
            set(&mut vals, &mut source, 1, b.weight, "Team-listed", false);
        }
        _ => {
            let (h, wt) = afm_engine::world::scouting::file::listed(&p.player);
            let s = if pro_side { "School-listed" } else { "School-listed (high school roster)" };
            set(&mut vals, &mut source, 0, h, s, false);
            set(&mut vals, &mut source, 1, wt, s, false);
        }
    }
    Known { vals, lifts, source }
}

/// His best verified forty with where it came from ("4.52 · NFL combine").
pub(crate) fn best_forty(w: &World, i: usize) -> Option<String> {
    let k = known(w, i);
    let v = k.vals[5]?;
    let s = k.source[5].as_ref().map_or("", |s| s.0.as_str());
    let s = s.split(',').next().unwrap_or(s);
    Some(format!("{v:.2} · {}", s.to_lowercase().trim_start_matches("your ")))
}

fn text(k: usize, v: f64) -> String {
    match k {
        0 => views::ft_in(v),
        1 => format!("{v:.0} lb"),
        2..=4 => {
            let e = (v * 8.0).round() as i64;
            let frac = match e % 8 {
                0 => String::new(),
                4 => " 1/2".into(),
                2 => " 1/4".into(),
                6 => " 3/4".into(),
                n => format!(" {n}/8"),
            };
            format!("{}{frac}\"", e / 8)
        }
        5 | 6 | 9 | 10 => format!("{v:.2}"),
        7 => format!("{v:.1}\""),
        8 => format!("{}'{}\"", v as i64 / 12, v as i64 % 12),
        _ => format!("{v:.0} reps"),
    }
}

pub(crate) fn view(w: &World, id: u32) -> Reply {
    let i = views::person_index(w, id).ok_or("That player is no longer in football.")?;
    let p = &w.people[i];
    let pos = p.player.position;
    let (_, many) = crate::lane_ratings::pos_words(pos);
    let k = known(w, i);
    let (level_label, (level, drafted)) = pools(w, i);
    let real = REAL.get(&pos);
    let col = |pool: &[[Option<f64>; 12]], t: usize| -> Vec<f64> { pool.iter().filter_map(|m| m[t]).collect() };
    let lv: Vec<[Option<f64>; 12]> = level.iter().map(|m| m.vals).collect();
    let dv: Vec<[Option<f64>; 12]> = drafted.iter().map(|m| m.vals).collect();
    // His percentile on each test against each comparison.
    let pct_of = |t: usize, v: f64| -> [Option<f64>; 3] {
        let lower = TESTS[t].2;
        let a = col(&lv, t);
        let b = col(&dv, t);
        let c = real.and_then(|r| r[t]).map(|(m, sd)| {
            let z = (v - m) / sd;
            100.0 * if lower { 1.0 - phi(z) } else { phi(z) }
        });
        [(a.len() >= 10).then(|| pct_in(&a, v, lower)), (b.len() >= 10).then(|| pct_in(&b, v, lower)), c]
    };
    let rows: Vec<Value> = TESTS
        .iter()
        .enumerate()
        .map(|(t, &(key, label, lower, _))| {
            let v = k.vals[t];
            let pct = v.map(|v| pct_of(t, v));
            json!({
                "key": key, "label": label, "lower": lower,
                "value": v, "text": v.map(|v| text(t, v)),
                "source": k.source[t].as_ref().map(|s| s.0.clone()),
                "verified": k.source[t].as_ref().map(|s| s.1),
                "pct": pct.map(|p| p.map(|x| x.map(|v| v.round().clamp(1.0, 99.0)))),
            })
        })
        .collect();
    // The athletic score: the share of each comparison he beats, on 0-10,
    // by size, speed, burst and agility, and over every test.
    let subs_def: [(&str, &[usize]); 4] =
        [("Size", &[0, 1]), ("Speed", &[5]), ("Burst", &[7, 8]), ("Agility", &[9, 10])];
    let all_tests = [0usize, 1, 5, 7, 8, 9, 10, 11];
    let score_from = |pcts: &dyn Fn(usize) -> Option<f64>| -> (Vec<Option<f64>>, Option<f64>) {
        let subs = subs_def
            .iter()
            .map(|(_, ts)| {
                let v: Vec<f64> = ts.iter().filter_map(|&t| pcts(t)).collect();
                (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64 / 10.0)
            })
            .collect();
        let v: Vec<f64> = all_tests.iter().filter_map(|&t| pcts(t)).collect();
        (subs, (v.len() >= 3).then(|| v.iter().sum::<f64>() / v.len() as f64 / 10.0))
    };
    let mut scores: Vec<Value> = Vec::new();
    for c in 0..3 {
        let (subs, score) = score_from(&|t| k.vals[t].and_then(|v| pct_of(t, v)[c]));
        // How many in the pool his score beats (each scored the same way).
        let better = score.map(|s| match c {
            0 | 1 => {
                let pool = if c == 0 { &lv } else { &dv };
                let mut cols: Vec<Vec<f64>> = (0..12).map(|t| col(pool, t)).collect();
                for v in &mut cols {
                    v.sort_by(|a, b| a.total_cmp(b));
                }
                let theirs: Vec<f64> = pool
                    .iter()
                    .filter_map(|m| {
                        let v: Vec<f64> =
                            all_tests.iter().filter_map(|&t| m[t].map(|x| pct_in(&cols[t], x, TESTS[t].2))).collect();
                        (v.len() >= 3).then(|| v.iter().sum::<f64>() / v.len() as f64 / 10.0)
                    })
                    .collect();
                // Shown from 1 to 99, as a percentile is.
                pct_in(&theirs, s, false).round().clamp(1.0, 99.0)
            }
            _ => (s * 10.0).round(),
        });
        scores.push(json!({
            "score": score.map(|s| (s * 10.0).round() / 10.0),
            "better": better,
            "subs": subs_def.iter().zip(subs).map(|((n, _), v)| json!({ "k": n, "v": v.map(|x| (x * 10.0).round() / 10.0) })).collect::<Vec<_>>(),
        }));
    }
    // The scatter: each test against weight, for the level and the drafted.
    // Every man in the group (minor bug 30), named, with his page a click away.
    let dot = |m: &[Option<f64>; 12], f: bool, name: &str, school: &str, id: Option<u32>| json!({ "v": m.iter().map(|x| x.map(|v| (v * 100.0).round() / 100.0)).collect::<Vec<_>>(), "f": f, "n": name, "s": school, "id": id });
    let level_dots: Vec<Value> =
        level.iter().filter(|m| m.id != Some(id)).map(|m| dot(&m.vals, m.first, &m.name, &m.school, m.id)).collect();
    let drafted_dots: Vec<Value> = drafted
        .iter()
        .filter(|m| m.id != Some(id))
        .map(|m| {
            let school = if m.school.is_empty() { format!("{}", m.year) } else { format!("{} {}", m.school, m.year) };
            dot(&m.vals, m.pick.is_some_and(|x| x <= 32), &m.name, &school, m.id)
        })
        .collect();
    let real_json: Vec<Value> =
        TESTS.iter().enumerate().map(|(t, _)| json!(real.and_then(|r| r[t]).map(|(m, s)| [m, s]))).collect();
    // Built like him: the closest drafted players by size and testing.
    let alikes: Vec<Value> = {
        let dims = [0usize, 1, 5, 7, 9];
        let spread: Vec<f64> = dims
            .iter()
            .map(|&t| {
                let v = col(&dv, t);
                let n = v.len().max(1) as f64;
                let m = v.iter().sum::<f64>() / n;
                (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n).sqrt().max(1e-6)
            })
            .collect();
        let mut near: Vec<(f64, &Past)> = drafted
            .iter()
            .filter(|d| d.id != Some(id))
            .filter_map(|d| {
                let mut s = 0.0;
                let mut n = 0;
                for (j, &t) in dims.iter().enumerate() {
                    if let (Some(a), Some(b)) = (k.vals[t], d.vals[t]) {
                        s += ((a - b) / spread[j]).powi(2);
                        n += 1;
                    }
                }
                (n >= 3).then(|| ((s / n as f64).sqrt(), d))
            })
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        near.into_iter()
            .take(5)
            .map(|(d, x)| {
                json!({
                    "name": x.name, "id": x.id,
                    "line": format!("{}{} · {} · {}", x.school, if x.school.is_empty() { String::new() } else { format!(" {}", x.year) },
                        x.vals[1].map_or("–".into(), |v| format!("{v:.0}")), x.vals[5].map_or("–".into(), |v| format!("{v:.2}"))),
                    "drafted": match (x.round, x.pick) { (Some(r), Some(p)) => format!("Round {r}, pick {p}"), _ => "Undrafted".into() },
                    "career": x.career,
                    "match": (100.0 - 18.0 * d).clamp(0.0, 99.0).round(),
                })
            })
            .collect()
    };
    // His weight over the seasons your staff kept, and its testing days.
    let mut weights: Vec<Value> = Vec::new();
    if let Some(c) = &w.career {
        for r in c.kept_of(id).iter().filter(|r| r.wt > 0) {
            weights.push(json!({ "label": format!("End of {}", r.season), "sort": r.season as f64 + 0.9, "wt": r.wt, "source": "Your staff's kept read" }));
        }
    }
    for d in &w.training.tests {
        if let Some(r) = d.rows.iter().find(|r| r.id == id).filter(|r| r.weight > 0) {
            let (y, m, dd) = d.date.parts();
            weights.push(json!({ "label": views::date_text(d.date), "sort": y as f64 + (m as f64 - 1.0 + dd as f64 / 31.0) / 12.0, "wt": r.weight, "source": if d.baseline { "Baseline testing" } else { "Testing day" } }));
        }
    }
    let now_wt = body_now(&p.player).weight.round();
    let (y, m, dd) = w.today.parts();
    weights.push(json!({ "label": "Today", "sort": y as f64 + (m as f64 - 1.0 + dd as f64 / 31.0) / 12.0, "wt": now_wt, "source": k.source[1].as_ref().map(|s| s.0.clone()) }));
    weights.sort_by(|a, b| a["sort"].as_f64().unwrap_or(0.0).total_cmp(&b["sort"].as_f64().unwrap_or(0.0)));
    let first_wt = weights.first().and_then(|x| x["wt"].as_f64());
    let target = w.training.plan_of(id).body.map(|b| b.0);
    // Seasons played out only: this season's row is written as it starts.
    let past: Vec<&afm_engine::player::SeasonRecord> = p
        .player
        .history
        .iter()
        .enumerate()
        .filter(|&(k, _)| super::arc::record_season(w, p, k) < w.year)
        .map(|(_, h)| h)
        .collect();
    let missed: u32 = past.iter().map(|h| h.games_missed).sum();
    let hurt = past
        .iter()
        .filter(|h| {
            use afm_engine::player::DevelopmentEvent as E;
            h.events.has(E::MinorInjury) || h.events.has(E::ModerateInjury) || h.events.has(E::MajorInjury)
        })
        .count();
    let (first, last) = first_last(&p.name);
    Ok(json!({
        "id": id, "name": p.name, "first": first, "last": last, "pos": pos.name(), "many": many,
        "compares": [
            { "key": "level", "label": super::cap(&level_label), "long": level_label, "n": level.len() },
            { "key": "drafted", "label": format!("Drafted {}", pos.name()), "long": format!("every drafted {} the world remembers", pos.name()), "n": drafted.len() },
            { "key": "combine", "label": "Combine since 2000", "long": format!("{many} at the real NFL combine since 2000"), "n": null },
        ],
        "rows": rows,
        "lifts": k.lifts.iter().map(|(a, b)| json!({ "label": a, "text": b })).collect::<Vec<_>>(),
        "scores": scores,
        "plot": { "level": level_dots, "drafted": drafted_dots, "real": real_json },
        "alikes": alikes,
        "weights": weights,
        "body": {
            "now": now_wt,
            "gained": first_wt.map(|f| now_wt - f),
            "target": target,
            "missed": missed,
            "injuries": hurt,
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_normal_curve_reads_right() {
        assert!((phi(0.0) - 0.5).abs() < 1e-6);
        assert!((phi(1.0) - 0.8413).abs() < 1e-3);
        assert!((phi(-1.96) - 0.025).abs() < 1e-3);
    }

    #[test]
    fn faster_is_better_on_the_clock() {
        let pool = [4.4, 4.5, 4.6, 4.7];
        assert!(pct_in(&pool, 4.45, true) > pct_in(&pool, 4.65, true));
        assert!(pct_in(&pool, 4.65, false) > pct_in(&pool, 4.45, false));
    }

    #[test]
    fn the_real_combine_is_read_for_every_position_it_has() {
        assert!(REAL.get(&Position::WR).is_some_and(|v| v[5].is_some()));
        assert!(REAL.get(&Position::K).is_some_and(|v| v[9].is_none()));
    }
}
