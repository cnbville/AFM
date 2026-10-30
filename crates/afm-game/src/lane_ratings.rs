//! Lane commands: what a rating range means, and your read of your staff
//! (update 2, lane 3). `call` answers the commands this lane owns and
//! returns None for any other.
//!
//! - RR1, RR4: `rr_card` explains a player's range for his level and
//!   position (`afm_engine::world::norms`): where elite starts, a typical
//!   starter and backup, the band the range falls in, and for a physical
//!   rating what it stands for at the combine for his body.
//! - RR2: `rr_staff` explains a staff range against the same job at the
//!   same level.
//! - Q1-Q4: `staff_quality` (board 17), `staff_member` and
//!   `coach_ratings` (board 18) read `afm_engine::world::staff_read`.
//!
//! Marks are cached for the season: a pool is read once a year per world.

use std::collections::HashMap;
use std::sync::Mutex;

use afm_engine::attributes::Attr;
use afm_engine::body::Drills;
use afm_engine::positions::Position;
use afm_engine::world::coach::Role;
use afm_engine::world::folk::Team;
use afm_engine::world::norms::{self, Marks, Pool, Rated};
use afm_engine::world::staff_read::{self as sr, Dept, Job, Member};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use crate::reads::{self, Read};
use crate::views;
use crate::{Game, Reply};

const COMMANDS: [&str; 5] = ["rr_card", "rr_staff", "staff_quality", "staff_member", "coach_ratings"];

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    if !COMMANDS.contains(&cmd) {
        return None;
    }
    let Some(w) = game.world.as_ref() else { return Some(Err("No world is loaded.".into())) };
    let num = |k: &str| args.get(k).and_then(Value::as_f64);
    let range = || match (num("lo"), num("hi")) {
        (Some(lo), Some(hi)) => Ok((lo.round().clamp(1.0, 99.0) as u8, hi.round().clamp(1.0, 99.0) as u8)),
        _ => Err("Missing range.".to_string()),
    };
    let who = || {
        args.get("who")
            .and_then(Value::as_str)
            .and_then(|s| u64::from_str_radix(s, 16).ok())
            .ok_or_else(|| "Missing who.".to_string())
    };
    Some(match cmd {
        "rr_card" => (|| {
            let id = num("p").ok_or("Missing p.")? as u32;
            let k = args.get("k").and_then(Value::as_str).unwrap_or("ovr");
            card(w, id, k, range()?)
        })(),
        "rr_staff" => (|| {
            let m = sr::member(w, who()?).ok_or("He is no longer on your staff.")?;
            let k = args.get("k").and_then(Value::as_str).ok_or("Missing k.")?;
            staff_card(w, &m, k, range()?)
        })(),
        "staff_quality" => Ok(quality(w)),
        "staff_member" => (|| {
            let m = sr::member(w, who()?).ok_or("He is no longer on your staff.")?;
            Ok(detail(w, &m))
        })(),
        "coach_ratings" => (|| {
            let m = match args.get("id").and_then(Value::as_u64) {
                Some(c) => sr::of_coach(w, c as usize),
                None => sr::member(w, who()?),
            };
            let m = m.ok_or("Your read of a coach comes with having him on your staff.")?;
            Ok(ratings_page(w, &m))
        })(),
        _ => Err(format!("Unknown command: {cmd}")),
    })
}

// ---- RR1 and RR4: a player's range ----

pub(crate) fn pos_words(p: Position) -> (&'static str, &'static str) {
    match p {
        Position::QB => ("quarterback", "quarterbacks"),
        Position::RB => ("running back", "running backs"),
        Position::WR => ("wide receiver", "wide receivers"),
        Position::TE => ("tight end", "tight ends"),
        Position::OT => ("offensive tackle", "offensive tackles"),
        Position::IOL => ("interior lineman", "interior linemen"),
        Position::EDGE => ("edge rusher", "edge rushers"),
        Position::DT => ("defensive tackle", "defensive tackles"),
        Position::LB => ("linebacker", "linebackers"),
        Position::CB => ("cornerback", "cornerbacks"),
        Position::S => ("safety", "safeties"),
        Position::K => ("kicker", "kickers"),
        Position::P => ("punter", "punters"),
        Position::LS => ("long snapper", "long snappers"),
    }
}

/// A graduating class in words this school year: "seniors", "juniors".
fn grade_words(w: &World, class_of: i32) -> String {
    let seniors = if w.today < w.dates.hs_school_start { w.year } else { w.year + 1 };
    match class_of - seniors {
        0 => "seniors".into(),
        1 => "juniors".into(),
        2 => "sophomores".into(),
        3 => "freshmen".into(),
        _ => format!("the class of {class_of}"),
    }
}

/// Who a pool is, in words: "Texas 6A juniors", "FBS", "the NFL".
fn pool_words(w: &World, pool: Pool, school: Option<usize>) -> String {
    match pool {
        Pool::Nfl => "NFL".into(),
        Pool::College(0) => "FBS".into(),
        Pool::College(1) => "FCS".into(),
        Pool::College(_) => "small-college".into(),
        Pool::Hs { state, class, grade } => {
            let g = grade_words(w, grade);
            match (state, class, school) {
                (Some(s), Some(_), Some(h)) => {
                    format!("{} {} {g}", views::state_name(&w.states[s].code), views::hs_class(w, h))
                }
                (Some(s), _, _) => format!("{} {g}", views::state_name(&w.states[s].code)),
                _ => format!("{g} across the country"),
            }
        }
    }
}

type Key = (u32, i32, usize, Pool, Position, Rated);

/// The season's marks for a pool, read once and kept.
pub(crate) fn marks_cached(w: &World, pool: Pool, pos: Position, rated: Rated) -> Option<Marks> {
    static CACHE: Mutex<Option<HashMap<Key, Option<Marks>>>> = Mutex::new(None);
    let key: Key = (w.options.seed, w.year, w.people.len() / 4096, pool, pos, rated);
    let mut g = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let map = g.get_or_insert_with(HashMap::new);
    // A new season or another world: read again.
    map.retain(|k, _| k.0 == key.0 && k.1 == key.1);
    if let Some(m) = map.get(&key) {
        return m.clone();
    }
    let m = norms::marks(w, pool, pos, rated);
    map.insert(key, m.clone());
    m
}

fn round5(x: f64) -> u32 {
    ((x * 100.0 / 5.0).round() * 5.0).clamp(0.0, 100.0) as u32
}

/// Where a range falls against the marks, in a few words, and a line.
pub(crate) fn band(m: &Marks, lo: u8, hi: u8, who: &str) -> (String, String) {
    let mid = (lo as f64 + hi as f64) / 2.0;
    let word = if mid >= m.elite {
        "Elite"
    } else if mid >= (m.starter + m.elite) / 2.0 {
        "A good starter"
    } else if mid >= m.starter {
        "Starter level"
    } else if mid >= m.backup {
        "Between a backup and a starter"
    } else if mid >= m.backup - (m.starter - m.backup).max(3.0) {
        "Backup level"
    } else {
        "Below a typical backup"
    };
    let (a, b) = (round5(m.below(lo as f64)), round5(m.below(hi as f64 + 0.5)));
    let ahead =
        if a == b { format!("Ahead of about {a}% of {who}") } else { format!("Ahead of about {a}-{b}% of {who}") };
    let straddle = if (lo as f64) < m.elite && (hi as f64) >= m.elite {
        "; the range straddles the elite line"
    } else if (lo as f64) < m.starter && (hi as f64) >= m.starter && mid < m.elite {
        "; the range straddles a typical starter"
    } else {
        ""
    };
    (word.to_string(), format!("{ahead}{straddle}."))
}

/// What a physical range stands for at the combine for his body (RR4).
fn combine_line(w: &World, k: usize, attr: Attr, lo: u8, hi: u8) -> Option<String> {
    let p = &w.people[k];
    // The other ratings a drill reads are taken at your read's middle.
    let reads = reads::attributes(w, k);
    let mid = |a: Attr| match reads.iter().find(|x| x.0 == a).map(|x| x.1) {
        Some(Read::Range(l, h)) => (l as f64 + h as f64) / 2.0,
        _ => p.player.attributes.get(a).round(),
    };
    let base = Drills::of(&p.player);
    let at = |v: f64| {
        let mut d = Drills {
            speed: mid(Attr::Speed),
            accel: mid(Attr::Acceleration),
            agility: mid(Attr::Agility),
            strength: mid(Attr::Strength),
            jumping: mid(Attr::Jumping),
            flexibility: mid(Attr::Flexibility),
            ..base
        };
        match attr {
            Attr::Speed => d.speed = v,
            Attr::Acceleration => d.accel = v,
            Attr::Agility => d.agility = v,
            Attr::Strength => d.strength = v,
            Attr::Jumping => d.jumping = v,
            _ => {}
        }
        d.expected()
    };
    let (a, b) = (at(lo as f64), at(hi as f64));
    let lb = base.weight.round();
    let two = |x: f64, y: f64, f: &dyn Fn(f64) -> String| {
        let (s, t) = (f(x.min(y)), f(x.max(y)));
        if s == t { s } else { format!("{s}-{t}") }
    };
    let sec = |x: f64| format!("{x:.2}");
    Some(match attr {
        Attr::Speed => format!("About a {} forty at {lb} lb", two(a.forty, b.forty, &sec)),
        Attr::Acceleration => format!("About a {} forty at {lb} lb, with his speed read", two(a.forty, b.forty, &sec)),
        Attr::Agility => format!(
            "About a {} short shuttle and a {} 3-cone at {lb} lb",
            two(a.shuttle, b.shuttle, &sec),
            two(a.cone, b.cone, &sec)
        ),
        Attr::Strength => format!(
            "About {} reps of 225 lb at {lb} lb with {:.1}-inch arms",
            two(a.bench, b.bench, &|x| format!("{x:.0}")),
            base.arm
        ),
        Attr::Jumping => {
            if a.vertical == b.vertical {
                format!("About a {:.1}-inch vertical at {lb} lb: his burst off the line carries his jumps", a.vertical)
            } else {
                format!(
                    "About a {}-inch vertical and a {} broad jump at {lb} lb",
                    two(a.vertical, b.vertical, &|x| format!("{x:.1}")),
                    two(a.broad, b.broad, &|x| format!("{}'{}\"", x as i32 / 12, x as i32 % 12))
                )
            }
        }
        _ => return None,
    })
}

/// The hover card for a player's range.
fn card(w: &World, id: u32, k: &str, (lo, hi): (u8, u8)) -> Reply {
    let i = views::person_index(w, id).ok_or("That player is no longer in football.")?;
    let p = &w.people[i];
    let pool = norms::pool_of(w, i).ok_or("He isn't playing anywhere now.")?;
    let pos = p.player.position;
    let (rated, title, at) = if k == "ovr" {
        (Rated::Overall, "Overall".to_string(), pos)
    } else if let Some(name) = k.strip_prefix("at:") {
        let at = Position::ALL.iter().copied().find(|x| x.name() == name).ok_or("No such position.")?;
        (Rated::At(at), format!("At {}", pos_words(at).0), at)
    } else {
        let a = Attr::ALL.iter().copied().find(|a| a.key() == k).ok_or("No such rating.")?;
        (Rated::Attr(a), a.label().to_string(), pos)
    };
    let m = marks_cached(w, pool, pos, rated).ok_or("Nobody at his level plays there to read it against.")?;
    let school = matches!(p.stage, Stage::HighSchool | Stage::PrepSchool).then_some(p.hs);
    let pw = pool_words(w, m.pool, school);
    let (one, many) = pos_words(at);
    let who = match m.pool {
        Pool::Hs { .. } => format!("{pw} starting at {one}"),
        Pool::Nfl => format!("NFL starting {many}"),
        _ => format!("{pw} starting {many}"),
    };
    let widened = m.pool != pool;
    let (word, line) = band(&m, lo, hi, &who);
    let combine = match rated {
        Rated::Attr(a) => combine_line(w, i, a, lo, hi),
        _ => None,
    };
    // H2 (update 3, lane 6): how the read moved since last season, and why.
    let change = match rated {
        Rated::Attr(a) => crate::lane_help::last_year(w, i, a, (lo, hi)),
        _ => None,
    };
    Ok(json!({
        "title": title,
        "range": [lo, hi],
        "pool": match m.pool {
            Pool::Hs { .. } => format!("Against {pw} at {one} this season"),
            Pool::Nfl => format!("Against NFL {many} this season"),
            _ => format!("Against {pw} {many} this season"),
        },
        "widened": widened.then_some("Too few at his own classification to read: the wider pool is shown."),
        "marks": { "backup": m.backup.round(), "starter": m.starter.round(), "elite": m.elite.round() },
        "marks_line": format!(
            "Elite starts at {:.0}. A typical starter has {:.0}, a typical backup {:.0}.",
            m.elite, m.starter, m.backup
        ),
        "band": word,
        "line": line,
        "combine": combine,
        "change": change,
        "dev": views::team_of(w, p).is_some_and(|t| views::is_mine(w, t)),
        "count": format!("{} starters on {} teams", m.starters.len(), m.teams),
    }))
}

// ---- RR2: a staff range ----

fn job_words(m: &Member) -> String {
    match m.job {
        Job::Coach(Role::PositionCoach(g)) => g.coach_title().to_string(),
        Job::Coach(Role::OffensiveCoordinator) => "offensive coordinator".into(),
        Job::Coach(Role::DefensiveCoordinator) => "defensive coordinator".into(),
        Job::Coach(Role::HeadCoach) => "head coach".into(),
        Job::HsCoach => "high school assistant".into(),
        _ => m.title.to_lowercase(),
    }
}

fn plural_job(s: &str) -> String {
    if let Some(x) = s.strip_suffix("coach") {
        format!("{x}coaches")
    } else if s.ends_with('s') {
        s.to_string()
    } else {
        format!("{s}s")
    }
}

/// The hover card for a staff range (RR2): against the same job at the
/// same level.
fn staff_card(w: &World, m: &Member, k: &str, (lo, hi): (u8, u8)) -> Reply {
    let f = m.facets.iter().position(|x| x.key == k).ok_or("No such rating.")?;
    let name = m.facets[f].name.clone();
    let level = sr::level_words(w);
    let jobs = plural_job(&job_words(m));
    let mut v = json!({
        "title": name,
        "range": [lo, hi],
        "word": sr::read_words(lo, hi),
        "sure": sr::sure_words(lo, hi),
        "rests": sr::rests_on(w, m, f),
        "pool": format!("Against {level} {jobs} this season"),
    });
    if let Some(j) = sr::job_marks(w, m, f) {
        let mid = (lo as f64 + hi as f64) / 2.0;
        let place = if mid >= j.q90 {
            "among the best tenth"
        } else if mid >= j.q75 {
            "in the top quarter"
        } else if mid >= j.q50 {
            "above the middle"
        } else if mid >= j.q25 {
            "below the middle"
        } else {
            "in the bottom quarter"
        };
        v["marks"] = json!({ "q25": j.q25.round(), "q50": j.q50.round(), "q75": j.q75.round(), "q90": j.q90.round() });
        v["marks_line"] = json!(format!(
            "The best tenth of {level} {jobs} start at {:.0}; a typical one has {:.0}; the bottom quarter sit under {:.0}.",
            j.q90, j.q50, j.q25
        ));
        v["band"] = json!(format!("Your read puts him {place} of the {} {level} {jobs}.", j.n));
    }
    Ok(v)
}

// ---- Q1: your staff's quality ----

fn ordinal(n: i32) -> String {
    let s = match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

fn seasons_with(w: &World, m: &Member) -> i32 {
    (w.year - m.start.year()).max(0) + 1
}

fn money(d: f64) -> String {
    if d >= 1.0e6 { format!("${:.2}M", d / 1.0e6) } else { format!("${:.0}K", (d / 1000.0).round()) }
}

fn hired_words(m: &Member) -> String {
    match m.not_yours {
        Some(w) if w.starts_with("hired") => "the athletic department hires".into(),
        Some(_) => "the club hires".into(),
        None if m.kept => format!("here since {}, kept on", m.since),
        None => format!("you hired, {}", m.since),
    }
}

fn read_json(w: &World, m: &Member, f: usize) -> Value {
    let fc = &m.facets[f];
    let now = sr::now(w, m, f);
    let start = sr::read(w, m, f, m.start);
    json!({
        "key": fc.key,
        "label": fc.name,
        "group": fc.group,
        "now": now.map(|r| [r.0, r.1]),
        "start": start.map(|r| [r.0, r.1]),
        "word": now.map(|r| sr::read_words(r.0, r.1)),
        "weak": now.is_some_and(|r| r.1 <= 9),
        "moved": sr::moved(w, m, f),
        "why": (!fc.seen()).then(|| fc.why_not.clone()),
    })
}

fn row_json(w: &World, m: &Member) -> Value {
    let key0 = m.keys.first().copied();
    let sure = key0.and_then(|f| sr::now(w, m, f)).map(|r| sr::sure_words(r.0, r.1));
    let (c1, c2) = match (m.not_yours, m.pay) {
        (Some(x), _) => {
            (if x.starts_with("hired") { "Athletics" } else { "Club" }.to_string(), "not yours".to_string())
        }
        (None, Some(p)) => (money(p), m.through.map_or(String::new(), |t| format!("to {t}"))),
        _ => ("—".into(), String::new()),
    };
    json!({
        "who": format!("{:x}", m.key),
        "coach": m.coach,
        "name": m.name,
        "title": m.title,
        "dept": m.dept.name(),
        "with": format!("{} season", ordinal(seasons_with(w, m))),
        "since": if m.not_yours.is_some() { format!("since {}", m.since) } else if m.kept { "kept on".to_string() } else { format!("hired {}", m.since) },
        "reads": m.keys.iter().map(|&f| read_json(w, m, f)).collect::<Vec<_>>(),
        "sure": sure,
        "c1": c1,
        "c2": c2,
        "runs": runs(m),
        "pay": m.pay.filter(|_| m.not_yours.is_none()),
        "age": m.age,
        "hired": hired_words(m),
    })
}

/// What a member runs for you, for the jobs and hand-offs list.
fn runs(m: &Member) -> String {
    let calls = m.facets.iter().find(|f| f.key == "calling");
    match m.job {
        Job::Coach(Role::HeadCoach) => "Runs the program and the game".into(),
        Job::Coach(Role::OffensiveCoordinator | Role::DefensiveCoordinator) => {
            let side = if m.job == Job::Coach(Role::OffensiveCoordinator) { "offense" } else { "defense" };
            match calls {
                Some(f) if f.seen() => format!("Runs the {side} and calls its plays"),
                _ => format!("Runs the {side}; you call its plays"),
            }
        }
        Job::Coach(_) | Job::Room(_) | Job::HsCoach => {
            m.group.map_or("Helps run the staff".to_string(), |g| format!("Teaches the {} room", g.name()))
        }
        Job::Analyst => "Breaks down opponents and your own tape".into(),
        Job::GradAssistant => "Helps coach and cuts the film".into(),
        Job::StrengthHead => "Runs the weight room, conditioning and testing".into(),
        Job::StrengthAssistant => "Runs speed work and the practice load numbers".into(),
        Job::Dietitian => "Runs fueling and weight targets".into(),
        Job::Trainer => "Runs the training room and sets return dates".into(),
        Job::Physician => "Diagnoses injuries; his medical calls stand".into(),
        Job::AssistantTrainer => "Runs rehab days and taping".into(),
        Job::AnalyticsHead => "Runs the numbers: fourth downs, two-point tries, tendencies".into(),
        Job::Personnel => "Runs the recruiting board and evaluation".into(),
        Job::RecruitingDirector => "Runs visits and the recruiting calendar".into(),
        Job::Academic => "Keeps players eligible and on track to graduate".into(),
        Job::Engagement => "Life skills, money and the players' lives away from football".into(),
    }
}

/// Board 17: your staff by department, two key reads each.
fn quality(w: &World) -> Value {
    let Some(team) = Team::seat(w) else { return json!({ "none": true }) };
    let all = sr::members(w);
    let depts: Vec<Value> = Dept::ALL
        .iter()
        .map(|&d| {
            let rows: Vec<Value> = all.iter().filter(|m| m.dept == d).map(|m| row_json(w, m)).collect();
            let n = rows.len();
            let foot = match (d, team) {
                (Dept::Coaches, Team::College(p))
                    if w.programs[p].division == afm_engine::world::data::Division::FBS && w.year >= 2018 =>
                {
                    "the NCAA allows 10 assistants".to_string()
                }
                (Dept::Medical, Team::College(_)) if n > 0 => "hired by the athletic department".to_string(),
                _ => format!("{n} {}", if n == 1 { "person" } else { "people" }),
            };
            let empty = match (d, team) {
                (Dept::Performance, Team::High(_)) => "No strength staff: your coaches run the weight room.",
                (Dept::Medical, Team::High(_)) => {
                    "No trainer of your own: most high schools share a part-time athletic trainer, if they have one."
                }
                (Dept::Ops, Team::High(_)) => "No analysts or support staff: the teachers on your staff do it all.",
                _ => "Nobody in this department.",
            };
            json!({ "name": d.name(), "rows": rows, "foot": foot, "empty": empty })
        })
        .collect();
    let first = all.first().map(|m| format!("{:x}", m.key));
    json!({
        "team": Team::seat(w).map(|t| t.name(w)),
        "level": sr::level_words(w),
        "depts": depts,
        "first": first,
        "today": views::date_text(w.today),
    })
}

/// The evidence lines under a member's reads.
fn evidence(w: &World, m: &Member) -> Vec<Value> {
    let mut v = Vec::new();
    let label = |k: &str| -> &'static str {
        match k {
            "calling" => "Play calls",
            "scheme" => "On Saturdays",
            "motivation" | "relationships" => "His players",
            "discipline" | "life" => "Conduct",
            "recruiting" => "Recruiting",
            "network" => "Network",
            "str" | "spd" | "nut" => "Training",
            "inj" | "rehab" | "rtp" | "diag" => "Injuries",
            "acad" => "Grades",
            "ana" | "self" | "opp" => "Film and numbers",
            "eval" | "org" => "The board",
            _ => "Position room",
        }
    };
    let mut seen: Vec<String> = Vec::new();
    let order: Vec<usize> = m.keys.iter().copied().chain((0..m.facets.len()).filter(|f| !m.keys.contains(f))).collect();
    for f in order {
        let fc = &m.facets[f];
        if !fc.seen() {
            continue;
        }
        let line = sr::rests_on(w, m, f);
        if seen.contains(&line) || line.ends_with("with you") {
            continue;
        }
        seen.push(line.clone());
        v.push(json!({ "k": label(&fc.key), "t": line }));
        if v.len() >= 3 {
            break;
        }
    }
    let r = sr::record(w, m);
    if !r.moved_up.is_empty() {
        let names: Vec<String> = r
            .moved_up
            .iter()
            .take(2)
            .filter_map(|x| views::person_index(w, x.0).map(|i| (i, x)))
            .map(|(i, x)| {
                let p = &w.people[i];
                format!(
                    "{}, {} in {}, now {}",
                    p.name,
                    views::role_words(x.2).to_lowercase(),
                    x.1,
                    p.last_role.map_or("—", |r| views::role_words(r)).to_lowercase()
                )
            })
            .collect();
        if !names.is_empty() {
            v.push(json!({ "k": "Moved up", "t": names.join("; ") }));
        }
    }
    if let Some(q) = says(w, &r) {
        v.push(json!({ "k": "People say", "t": q }));
    }
    if v.is_empty() {
        let s = seasons_with(w, m);
        v.push(json!({ "k": "With you", "t": if s <= 1 { "Too early: your read rests on his name and a few weeks of work".to_string() } else { format!("{} {} of work, no numbers of their own yet", s - 1, afm_engine::world::folk::noun(s - 1, "season")) } }));
    }
    v
}

/// What his players say: the one who grew most under him, or, when his
/// room is behind players like them, the one who grew least.
fn says(w: &World, r: &sr::Record) -> Option<String> {
    use afm_engine::world::coach::Group;
    let all: Vec<(u32, f64)> = r.seasons.iter().flat_map(|s| s.2.iter().copied()).collect();
    let best = all.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1))?;
    let worst = all.iter().copied().min_by(|a, b| a.1.total_cmp(&b.1))?;
    let (id, good) = if r.mean < -0.4 && worst.1 < -1.0 {
        (worst.0, false)
    } else if best.1 >= 1.0 {
        (best.0, true)
    } else {
        return None;
    };
    let p = &w.people[views::person_index(w, id)?];
    let line = if !good {
        "We run the same drills every day and nobody gets better."
    } else {
        match Group::of(p.player.position) {
            Group::QB => "He rebuilt my footwork from the ground up.",
            Group::RB => "He taught me to see the cutback before it opens.",
            Group::Receivers => "Fixed my hands in one spring.",
            Group::OL => "My hands and my feet finally work together.",
            Group::DL => "He taught me to win with my hands first.",
            Group::LB => "I see the play two steps earlier now.",
            Group::DB => "He made me trust my eyes.",
            Group::Specialists => "He fixed my steps. The ball goes where I aim it now.",
        }
    };
    Some(format!("\u{201c}{line}\u{201d} {}, {}", p.name, p.player.position.name()))
}

/// One member's detail (board 17's right panel).
fn detail(w: &World, m: &Member) -> Value {
    let k0 = m.keys.first().copied().unwrap_or(0);
    let now = sr::now(w, m, k0);
    let history: Vec<Value> = sr::steps(w, m)
        .into_iter()
        .filter_map(|(label, d)| sr::read(w, m, k0, d).map(|r| json!({ "label": label, "lo": r.0, "hi": r.1 })))
        .collect();
    let contract = match (m.not_yours, m.pay) {
        (Some(x), _) => {
            if x.starts_with("hired") { "Athletic department staff" } else { "The club's medical staff" }.to_string()
        }
        (None, Some(p)) => format!("{} a year{}", money(p), m.through.map_or(String::new(), |t| format!(" to {t}"))),
        _ => String::new(),
    };
    json!({
        "who": format!("{:x}", m.key),
        "coach": m.coach,
        "name": m.name,
        "role": format!("{} · {} season with you", m.title, ordinal(seasons_with(w, m))),
        "contract": contract,
        "hired": hired_words(m),
        "key": m.facets.get(k0).map(|f| f.name.to_lowercase()),
        "word": now.map(|r| sr::read_words(r.0, r.1)),
        "sub": now.map(|r| format!("{} to {} · {}", r.0, r.1, sr::sure_words(r.0, r.1).to_lowercase())),
        "weak": now.is_some_and(|r| r.1 <= 9),
        "history": history,
        "all": (0..m.facets.len()).map(|f| read_json(w, m, f)).collect::<Vec<_>>(),
        "evidence": evidence(w, m),
    })
}

// ---- Q2: the coach page's Ratings tab ----

fn ratings_page(w: &World, m: &Member) -> Value {
    let steps = sr::steps(w, m);
    let facets: Vec<Value> = (0..m.facets.len())
        .map(|f| {
            let fc = &m.facets[f];
            let at: Vec<Value> = steps.iter().map(|(_, d)| json!(sr::read(w, m, f, *d).map(|r| [r.0, r.1]))).collect();
            json!({
                "key": fc.key,
                "name": fc.name,
                "group": fc.group,
                "at": at,
                "why": (!fc.seen()).then(|| fc.why_not.clone()),
                "rests": sr::rests_on(w, m, f),
                "moved": sr::moved(w, m, f),
            })
        })
        .collect();
    let r = sr::record(w, m);
    let reader = views::reader_key(w);
    let seasons: Vec<Value> = r
        .seasons
        .iter()
        .map(|(y, mean, pl)| {
            json!({
                "year": y,
                "mean": (mean * 10.0).round() / 10.0,
                "players": pl.iter().map(|(id, x)| json!({ "id": id, "name": views::person_index(w, *id).map(|i| w.people[i].name.clone()), "r": (x * 10.0).round() / 10.0 })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let moved: Vec<Value> = r
        .moved_up
        .iter()
        .filter_map(|&(id, year, role, start)| {
            let i = views::person_index(w, id)?;
            let p = &w.people[i];
            let then = reads::range(start, 2.5, reader, id, 99);
            let now = reads::overall(w, i);
            let mid = |r: Read| match r {
                Read::Range(a, b) => Some(((a as u32 + b as u32) / 2) as u8),
                Read::NotSeen => None,
            };
            Some(json!({
                "id": id,
                "name": p.name,
                "pos": p.player.position.name(),
                "class": views::class_text(w, p),
                "from": mid(then),
                "to": mid(now),
                "then": views::role_words(role).to_lowercase(),
                "now": p.last_role.map_or("—", views::role_words).to_lowercase(),
                "year": year,
            }))
        })
        .collect();
    let co = m.coach.and_then(|c| w.coaches.get(c));
    let before: Vec<Value> = co
        .map(|co| {
            co.history
                .iter()
                .rev()
                .take(5)
                .map(|s| {
                    let years = if s.from == s.to { s.from.to_string() } else { format!("{}-{}", s.from, s.to % 100) };
                    json!({ "years": years, "what": format!("{}, {}", cap(s.role.name()), views::employer_name(w, s.employer)) })
                })
                .collect()
        })
        .unwrap_or_default();
    let outside = co.map(|co| {
        let rep = views::reputation_words(co.reputation);
        let climber = co.personality.ambition >= 14.0;
        let market = if co.reputation >= 14.0 || (climber && co.reputation >= 11.0) {
            "In demand"
        } else if co.reputation >= 10.0 {
            "Some interest"
        } else {
            "Quiet"
        };
        let wants = match (m.job, climber) {
            (Job::Coach(Role::OffensiveCoordinator | Role::DefensiveCoordinator), true) => "A head job",
            (Job::Coach(Role::PositionCoach(_)), true) => "To coordinate",
            (_, true) => "A bigger job",
            _ if co.personality.loyalty >= 13.0 => "To stay and win here",
            _ => "Settled for now",
        };
        json!({
            "reputation": rep,
            "market": market,
            "market_sub": "what other staffs make of his name",
            "wants": wants,
            "wants_sub": if climber { "he says so when you meet" } else { "no talk of moving" },
        })
    });
    let k0 = m.keys.first().copied().unwrap_or(0);
    let summary = sr::now(w, m, k0).map(|(lo, hi)| {
        let word = sr::read_words(lo, hi).to_lowercase();
        let job = job_words(m);
        let good = (lo as u32 + hi as u32) >= 26;
        // Support staff have no name in the market to set it against.
        let agree = match co.map(|c| c.reputation >= 12.0) {
            None => "From his work so far",
            Some(rep_high) if rep_high == good => "Your read and his name agree",
            Some(true) => "His name is bigger than your read",
            Some(false) => "Your read is better than his name",
        };
        let lose = co.filter(|c| c.personality.ambition >= 14.0).map(|_| match m.job {
            Job::Coach(Role::PositionCoach(_)) => " A coordinator job elsewhere is the likeliest way to lose him.",
            Job::Coach(Role::OffensiveCoordinator | Role::DefensiveCoordinator) => {
                " A head job elsewhere is the likeliest way to lose him."
            }
            _ => "",
        });
        format!(
            "{agree}: {} {word} {job}.{}",
            if word.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" },
            lose.unwrap_or("")
        )
    });
    let team = Team::seat(w).map(|t| t.name(w)).unwrap_or_default();
    json!({
        "who": format!("{:x}", m.key),
        "coach": m.coach,
        "name": m.name,
        "kicker": format!("{} · {} · {}{} season with you", team, m.title, m.age.map_or(String::new(), |a| format!("{a} · ")), ordinal(seasons_with(w, m))),
        "evidence_years": plural_seasons(seasons_with(w, m) - 1),
        "steps": steps.iter().map(|s| s.0.clone()).collect::<Vec<_>>(),
        "facets": facets,
        "record": {
            "seasons": seasons,
            "mean": (r.mean * 10.0).round() / 10.0,
            "player_seasons": r.player_seasons,
            "line": format!("a year across {}, where an average {} sits at 0.", plural(r.player_seasons, "player-season"), job_words(m)),
        },
        "moved": moved,
        "before": before,
        "before_title": format!("Before {}", team),
        "outside": outside,
        "contract": match (m.not_yours, m.pay) {
            (Some(x), _) => json!({
                "line": if x.starts_with("hired") { "Athletic department staff" } else { "The club's medical staff" },
                "sub": "not yours to pay",
            }),
            (None, Some(p)) => json!({
                "line": format!("{} a year", money(p)),
                "sub": m.through.map(|t| format!("to {t}")),
            }),
            _ => Value::Null,
        },
        "summary": summary,
        "team": views::mine(w).map(|t| views::team_brief(w, t)),
    })
}

fn plural(n: usize, noun: &str) -> String {
    format!("{n} {noun}{}", if n == 1 { "" } else { "s" })
}

fn plural_seasons(n: i32) -> String {
    match n {
        x if x <= 0 => "the season so far".into(),
        1 => "one season of work".into(),
        x => format!("{x} seasons of work"),
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}
