//! Front office: the setup's leftovers and your side of the coaching
//! market (lane 10; first-playable topic 2, people topics 2, 13 and 14).
//! Starting as an assistant or without a job, the people from the world's
//! history who know you, your family, and the job market: what is open,
//! what you let it be known you want, your applications and the offers
//! that wait for your answer.

use afm_engine::calendar::{Date, Level};
use afm_engine::world::World;
use afm_engine::world::coach::{Employer, Group, Role};
use afm_engine::world::front_office::market::{self, Interest, Placement};
use afm_engine::world::seat::{Job, Start};
use serde_json::{Value, json};

use crate::views::{TeamRef, date_text};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        "start_career_seat" => game.wm().and_then(|w| {
            start(w, args)?;
            Ok(crate::views::status(w))
        }),
        "people_you_know" => game.w().map(|w| preview(w, args)),
        "assistant_jobs" => game.w().map(|w| assistant_jobs(w, args)),
        "market" => game.w().map(market_view),
        "market_apply" => game.wm().and_then(|w| {
            let e = employer(w, args.get("employer").unwrap_or(&Value::Null))?;
            let r = role(args.get("role").and_then(Value::as_str).unwrap_or(""))?;
            let text = market::apply(w, e, r)?;
            Ok(json!({ "text": text, "market": market_view(w) }))
        }),
        "market_accept" => game.wm().and_then(|w| {
            let k = args.get("k").and_then(Value::as_u64).ok_or("Missing k.")? as usize;
            let text = market::accept(w, k)?;
            Ok(json!({ "text": text, "status": crate::views::status(w) }))
        }),
        "market_decline" => game.wm().and_then(|w| {
            let k = args.get("k").and_then(Value::as_u64).ok_or("Missing k.")? as usize;
            let text = market::decline(w, k)?;
            Ok(json!({ "text": text, "market": market_view(w) }))
        }),
        "market_interest" => game.wm().and_then(|w| {
            set_interest(w, args)?;
            Ok(market_view(w))
        }),
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Reading jobs and roles from the page
// ---------------------------------------------------------------------------

fn employer(w: &World, v: &Value) -> Result<Employer, String> {
    let id = v.get("id").and_then(Value::as_u64).ok_or("Missing the team.")? as usize;
    match v.get("kind").and_then(Value::as_str) {
        Some("hs") if id < w.high_schools.len() => Ok(Employer::HighSchool(id)),
        Some("college") if id < w.programs.len() => Ok(Employer::College(id)),
        Some("nfl") if id < w.nfl.len() => Ok(Employer::Nfl(id)),
        Some("other") if id < afm_engine::world::pro::other::FRANCHISES => Ok(Employer::Other(id)),
        _ => Err("No such team.".into()),
    }
}

fn employer_json(w: &World, e: Employer) -> Value {
    crate::views::employer_brief(w, e)
}

/// A role's key on the page: hc, oc, dc, or pc0-pc7 by group.
pub(crate) fn role_key(r: Role) -> String {
    match r {
        Role::HeadCoach => "hc".into(),
        Role::OffensiveCoordinator => "oc".into(),
        Role::DefensiveCoordinator => "dc".into(),
        Role::PositionCoach(g) => format!("pc{}", g as usize),
    }
}

fn role(k: &str) -> Result<Role, String> {
    Ok(match k {
        "hc" => Role::HeadCoach,
        "oc" => Role::OffensiveCoordinator,
        "dc" => Role::DefensiveCoordinator,
        _ => {
            let g = k.strip_prefix("pc").and_then(|n| n.parse::<usize>().ok()).filter(|&g| g < Group::ALL.len());
            Role::PositionCoach(Group::ALL[g.ok_or("No such job.")?])
        }
    })
}

fn level_key(l: Level) -> &'static str {
    match l {
        Level::Nfl => "nfl",
        Level::HighSchool => "hs",
        Level::Other => "other",
        _ => "college",
    }
}

// ---------------------------------------------------------------------------
// Starting a career in any seat
// ---------------------------------------------------------------------------

/// Where the world places you from the setup's answers: your high school
/// and college by name, the years from your date of birth (or the years
/// you typed), whether you played, and your last stops.
fn placement(w: &World, args: &Value) -> (Placement, Vec<(Employer, Role, i32, i32)>) {
    let p = args.get("profile").cloned().unwrap_or(json!({}));
    let text = |k: &str| p.get(k).and_then(Value::as_str).unwrap_or("").trim().to_string();
    let born = text("born")
        .get(..4)
        .and_then(|y| y.parse::<i32>().ok())
        .unwrap_or_else(|| w.year - args.get("age").and_then(Value::as_i64).unwrap_or(42) as i32);
    let state = args.get("home_state").and_then(Value::as_str).unwrap_or("TX");
    let home = w.states.iter().position(|s| s.code == state);
    let hs_name = text("high_school");
    let high_school = home.filter(|_| !hs_name.is_empty()).and_then(|st| {
        w.states[st].schools.iter().copied().find(|&h| {
            let s = &w.high_schools[h];
            w.school_name(h).eq_ignore_ascii_case(&hs_name) || s.name.eq_ignore_ascii_case(&hs_name)
        })
    });
    let alma = text("alma_mater");
    let college = (!alma.is_empty())
        .then(|| {
            w.programs
                .iter()
                .find(|q| w.program_name(q.id).eq_ignore_ascii_case(&alma) || q.real_name.eq_ignore_ascii_case(&alma))
        })
        .flatten()
        .map(|q| q.id);
    // "2001-2004", "2001-04" or nothing: four years from 18.
    let years = text("college_years");
    let (a, b) = {
        let mut it = years.split(['-', '–']).map(|x| x.trim().parse::<i32>().ok());
        match (it.next().flatten(), it.next().flatten()) {
            (Some(a), Some(b)) if b < 100 => (a, a - a % 100 + b),
            (Some(a), Some(b)) => (a, b),
            (Some(a), None) => (a, a + 4),
            _ => (born + 18, born + 22),
        }
    };
    let background = args.get("background").and_then(Value::as_str).unwrap_or("");
    let played = crate::setup::played_college(background);
    let mut stops = Vec::new();
    let mut resume = Vec::new();
    for s in args.get("stops").and_then(Value::as_array).into_iter().flatten().take(6) {
        let Ok(e) = employer(w, s.get("employer").unwrap_or(&Value::Null)) else { continue };
        let from = s.get("from").and_then(Value::as_i64).unwrap_or(w.year as i64 - 3) as i32;
        let to = s.get("to").and_then(Value::as_i64).unwrap_or(w.year as i64 - 1) as i32;
        let (from, to) = (from.min(to).max(born + 18), to.max(from).min(w.year));
        let playing = s.get("playing").and_then(Value::as_bool).unwrap_or(false);
        stops.push((e, from, to, playing));
        if !playing && let Ok(r) = role(s.get("role").and_then(Value::as_str).unwrap_or("pc0")) {
            resume.push((e, r, from, to));
        }
    }
    let placement = Placement {
        college: college.map(|c| (c, a, b)),
        played,
        high_school: high_school.map(|h| (h, born + 14, born + 18)),
        stops,
    };
    (placement, resume)
}

fn start(w: &mut World, args: &Value) -> Result<(), String> {
    if w.career.is_some() {
        return Err("This world already has a career. Open a library world to start another.".into());
    }
    let you = crate::setup::you_from(w, args)?;
    let kind = args.get("start").and_then(Value::as_str).unwrap_or("head");
    let job = || -> Result<Job, String> {
        Ok(match TeamRef::from_args(w, args.get("job").unwrap_or(&Value::Null))? {
            TeamRef::Nfl(i) => Job::Nfl(i),
            TeamRef::College(i) => Job::College(i),
            TeamRef::HighSchool(i) => Job::HighSchool(i),
            TeamRef::Other(f) => Job::Other(f),
        })
    };
    let start = match kind {
        "assistant" => {
            let r = role(args.get("role").and_then(Value::as_str).unwrap_or(""))?;
            let j = job()?;
            match (j, r) {
                (_, Role::HeadCoach) => return Err("Pick an assistant's job.".into()),
                (Job::HighSchool(_), _) => return Err("High school assistants aren't a seat in the game.".into()),
                (Job::Nfl(_), Role::PositionCoach(_)) => {
                    return Err("NFL staffs in the game are the head coach and the two coordinators.".into());
                }
                (Job::Other(_), Role::PositionCoach(_)) => {
                    return Err("A club in the other leagues has a head coach and two coordinators in the game.".into());
                }
                _ => {}
            }
            Start::Assistant(j, r)
        }
        "free" => Start::Free(match args.get("level").and_then(Value::as_str) {
            Some("nfl") => Level::Nfl,
            Some("hs") => Level::HighSchool,
            Some("other") => Level::Other,
            _ => Level::College,
        }),
        _ => Start::Head(job()?),
    };
    let (placement, resume) = placement(w, args);
    let known = market::people_you_know(w, &placement);
    let seed = crate::setup::career_seed(args);
    w.start_with(&you, start, seed, known);
    w.add_resume(&resume);
    market::welcome(w);
    // Out of work: you let it be known you want jobs at the level you
    // looked at (college staffs from FCS up), and a high school job near
    // home (Claude's pick: the market screen changes it any day).
    if let Start::Free(level) = start {
        let home = you.home_state;
        if let Some(c) = w.career.as_mut() {
            let i = &mut c.market.interest;
            *i = Interest {
                listening: false,
                hs_head: level == Level::HighSchool,
                college_head: level == Level::College,
                coordinator: level != Level::HighSchool,
                position: level == Level::College,
                nfl: level == Level::Nfl,
                other: matches!(level, Level::Nfl | Level::Other),
                states: if level == Level::HighSchool { vec![home] } else { Vec::new() },
                college_floor: 1,
            };
        }
    }
    Ok(())
}

/// The setup's preview: who in the world knows you from what you wrote.
fn preview(w: &World, args: &Value) -> Value {
    let (p, _) = placement(w, args);
    let list = market::people_you_know(w, &p);
    let found = json!({
        "high_school": p.high_school.map(|(h, a, b)| json!({ "name": w.school_name(h), "years": format!("{a}-{b}") })),
        "college": p.college.map(|(c, a, b)| json!({ "name": w.program_name(c), "years": format!("{a}-{b}") })),
        "played": p.played,
    });
    json!({ "found": found, "known": list.iter().take(12).map(|k| known_row(w, k)).collect::<Vec<_>>(), "count": list.len() })
}

fn known_row(w: &World, k: &market::Known) -> Value {
    let c = &w.coaches[k.coach];
    let now = match c.job {
        Some(j) => format!("{} at {}", market::role_title(j.role), market::employer_name(w, j.employer)),
        None if c.retired => "Retired".into(),
        None => "Out of coaching".into(),
    };
    let close = match k.strength {
        s if s >= 0.75 => "Close",
        s if s >= 0.55 => "Knows you well",
        s if s >= 0.35 => "Knows you",
        _ => "Knows your name",
    };
    json!({ "id": k.coach, "name": c.name, "how": k.how, "years": if k.from == k.to { k.from.to_string() } else { format!("{}-{}", k.from, k.to) }, "now": now, "close": close })
}

/// The people who know you, closest first.
pub(crate) fn known_json(w: &World, n: usize) -> Value {
    let Some(c) = &w.career else { return json!([]) };
    json!(c.known.iter().take(n).map(|k| known_row(w, k)).collect::<Vec<_>>())
}

/// Every assistant's job at a level (first-playable topic 2): each team
/// with its coordinators and position coaches and who holds them now.
fn assistant_jobs(w: &World, args: &Value) -> Value {
    let level = args.get("level").and_then(Value::as_str).unwrap_or("college");
    let mut v = crate::setup::jobs(w, level);
    let holder = |c: Option<usize>| c.and_then(|c| w.coaches.get(c)).map(|c| c.name.clone());
    if let Some(list) = v["jobs"].as_array_mut() {
        for j in list.iter_mut() {
            let id = j["ref"]["id"].as_u64().unwrap_or(0) as usize;
            let roles: Vec<(Role, Option<usize>)> = match j["ref"]["kind"].as_str() {
                Some("college") => {
                    let p = &w.programs[id];
                    let mut r = vec![(Role::OffensiveCoordinator, p.oc), (Role::DefensiveCoordinator, p.dc)];
                    r.extend(Group::ALL.iter().map(|&g| (Role::PositionCoach(g), p.position_coaches[g as usize])));
                    r
                }
                Some("nfl") => {
                    let t = &w.nfl[id];
                    vec![(Role::OffensiveCoordinator, t.oc), (Role::DefensiveCoordinator, t.dc)]
                }
                Some("other") => [Role::OffensiveCoordinator, Role::DefensiveCoordinator]
                    .into_iter()
                    .map(|r| (r, afm_engine::world::pro::other::slot(w, id, r)))
                    .collect(),
                _ => Vec::new(),
            };
            j["roles"] = json!(
                roles
                    .iter()
                    .map(|&(r, c)| json!({ "key": role_key(r), "title": market::role_title(r), "holder": holder(c) }))
                    .collect::<Vec<_>>()
            );
        }
    }
    v
}

// ---------------------------------------------------------------------------
// Your seat, your family, your market
// ---------------------------------------------------------------------------

/// Your seat for the status bar: head coach or assistant, what your job
/// controls, out of work.
pub(crate) fn seat_json(w: &World) -> Value {
    let Some(c) = &w.career else { return Value::Null };
    let job = market::your_job(w);
    json!({
        "head": w.seat_head(),
        "role": job.map(|j| market::role_title(j.role)),
        "role_key": job.map(|j| role_key(j.role)),
        "controls": w.your_controls(),
        "out_of_work": job.is_none(),
        "level": w.seat.map(|s| level_key(s.level)),
        "boss": w.your_boss().map(|b| json!({ "id": b, "name": w.coaches[b].name })),
        "offers": c.market.offers.len(),
        "teaching_at": c.market.teaching_at.map(|s| w.school_name(s).to_string()),
    })
}

/// Your title in a line: "Offensive coordinator since 2026", "Out of coaching since 4 Feb 2027".
pub(crate) fn title_line(w: &World) -> String {
    let Some(c) = &w.career else { return String::new() };
    match market::your_job(w) {
        // The day you took it: a college job taken in February is the
        // hiring cycle's, but you have held it since that day.
        Some(j) => {
            format!("{} since {}", market::role_title(j.role), date_text(c.market.job_since.unwrap_or(c.started)))
        }
        None => match c.market.out_since {
            Some(d) => format!("Out of coaching since {}", date_text(d)),
            None => "Out of coaching".into(),
        },
    }
}

/// Your family: who they are, where they live, how they feel about it.
pub(crate) fn family_json(w: &World) -> Value {
    let Some(c) = &w.career else { return Value::Null };
    let p = &c.you.profile;
    let f = &c.family;
    let year = w.year;
    json!({
        "status": p.status,
        "spouse": p.spouse,
        "children": p.children.iter().map(|(n, b)| json!({ "name": n, "age": (year - b).max(0), "school": school_words(year - b) })).collect::<Vec<_>>(),
        "home": f.state.and_then(|s| w.states.get(s)).map(|s| market::state_name(&s.code)),
        "since": f.since,
        "moves": f.moves,
        "mood": market::family_mood_words(f.mood),
        "log": f.log.iter().rev().take(10).map(|(d, t)| json!({ "date": date_text(*d), "text": t })).collect::<Vec<_>>(),
    })
}

fn school_words(age: i32) -> &'static str {
    match age {
        i32::MIN..=4 => "Not in school yet",
        5 => "Kindergarten",
        6..=10 => "Elementary school",
        11..=13 => "Middle school",
        14 => "A freshman in high school",
        15 => "A sophomore in high school",
        16 => "A junior in high school",
        17 => "A senior in high school",
        18..=22 => "College age",
        _ => "Grown up",
    }
}

fn set_interest(w: &mut World, args: &Value) -> Result<(), String> {
    let codes: Option<Vec<usize>> = args.get("states").and_then(Value::as_array).map(|a| {
        a.iter().filter_map(Value::as_str).filter_map(|c| w.states.iter().position(|s| s.code == c)).collect()
    });
    let c = w.career.as_mut().ok_or("No career.")?;
    let i = &mut c.market.interest;
    let b = |k: &str, cur: bool| args.get(k).and_then(Value::as_bool).unwrap_or(cur);
    i.listening = b("listening", i.listening);
    i.hs_head = b("hs_head", i.hs_head);
    i.college_head = b("college_head", i.college_head);
    i.coordinator = b("coordinator", i.coordinator);
    i.position = b("position", i.position);
    i.nfl = b("nfl", i.nfl);
    i.other = b("other", i.other);
    if let Some(f) = args.get("college_floor").and_then(Value::as_u64) {
        i.college_floor = f.min(2) as u8;
    }
    if let Some(s) = codes {
        i.states = s;
    }
    Ok(())
}

/// When each level hires, in a line, for the market page.
fn cycle_words(w: &World) -> Vec<Value> {
    let d = &w.dates;
    let y = w.today.year();
    let hs = Date::ymd(if w.today > Date::ymd(y, 1, 20) { y + 1 } else { y }, 1, 20);
    vec![
        json!({ "level": "College", "text": format!("Head coaches are fired from the end of October; searches run through the bowls and into January. Assistants move from December to February. The regular season ends {}.", date_text(d.fbs_regular_end)) }),
        json!({ "level": "NFL", "text": "Black Monday follows the last regular-season Sunday; head coaches are hired in January and their coordinators within weeks.".to_string() }),
        json!({ "level": "Other leagues", "text": "The CFL fires after the Grey Cup and hires through the winter; the spring leagues hire in the fall. Any seat still empty is filled three weeks before camp.".to_string() }),
        json!({ "level": "High school", "text": format!("Principals and boards decide on their coaches after the season; most openings are posted and filled around {}.", date_text(hs)) }),
    ]
}

pub(crate) fn market_view(w: &World) -> Value {
    let Some(c) = &w.career else { return Value::Null };
    let m = &c.market;
    let job = market::your_job(w)
        .map(|j| json!({ "team": employer_json(w, j.employer), "role": market::role_title(j.role), "since": j.since }));
    let i = &m.interest;
    // The searches' own test, so the page shows what they'd call you about.
    let fits = |e: Employer, r: Role| market::interest_fits(w, i, e, r);
    let prestige = |e: Employer| match e {
        Employer::HighSchool(s) => w.high_schools[s].prestige * 0.35,
        Employer::College(p) => 0.2 + 0.5 * w.programs[p].prestige,
        Employer::Nfl(_) => 1.0,
        Employer::Other(f) => afm_engine::world::pro::other::prestige(f),
    };
    let mut open: Vec<(f64, Value)> = market::openings(w)
        .into_iter()
        .filter(|o| !market::your_job(w).is_some_and(|j| j.employer == o.employer && j.role == o.role))
        .map(|o| {
            let rung = match o.role {
                Role::HeadCoach => 0.1,
                Role::PositionCoach(_) => 0.0,
                _ => 0.05,
            };
            let line = crate::views::employer_line(w, o.employer);
            (
                prestige(o.employer) + rung,
                json!({
                    "team": employer_json(w, o.employer),
                    "line": line,
                    "role": market::role_title(o.role),
                    "role_key": role_key(o.role),
                    "decides": o.decides.map(date_text),
                    "applied": o.applied,
                    "fits": fits(o.employer, o.role),
                    "pay": afm_engine::world::front_office::market::pay_words(w, o.employer, o.role),
                }),
            )
        })
        .collect();
    open.sort_by(|a, b| b.0.total_cmp(&a.0));
    let offers: Vec<Value> = m
        .offers
        .iter()
        .enumerate()
        .map(|(k, o)| {
            json!({
                "k": k,
                "team": employer_json(w, o.employer),
                "line": crate::views::employer_line(w, o.employer),
                "role": market::role_title(o.role),
                "pay": o.pay,
                "years": o.years,
                "why": o.why,
                "family": o.family,
                "made": date_text(o.made),
                "until": date_text(o.until),
            })
        })
        .collect();
    let applied: Vec<Value> = m
        .applied
        .iter()
        .map(|(e, r, d)| json!({ "team": employer_json(w, *e), "role": market::role_title(*r), "on": date_text(*d) }))
        .collect();
    json!({
        "job": job,
        "title": title_line(w),
        "out_since": m.out_since.map(date_text),
        "teaching_at": m.teaching_at.map(|s| w.school_name(s).to_string()),
        "interest": {
            "listening": i.listening, "hs_head": i.hs_head, "college_head": i.college_head,
            "coordinator": i.coordinator, "position": i.position, "nfl": i.nfl, "other": i.other, "college_floor": i.college_floor,
            "states": i.states.iter().filter_map(|&s| w.states.get(s).map(|x| x.code.clone())).collect::<Vec<_>>(),
        },
        "states": w.states.iter().map(|s| json!({ "code": s.code, "name": market::state_name(&s.code) })).collect::<Vec<_>>(),
        "offers": offers,
        "openings": open.into_iter().take(400).map(|x| x.1).collect::<Vec<_>>(),
        "applied": applied,
        "log": m.log.iter().rev().take(30).map(|(d, t)| json!({ "date": date_text(*d), "text": t })).collect::<Vec<_>>(),
        "cycle": cycle_words(w),
        "reputation": crate::views::reputation_words(w.coaches[c.coach].reputation),
        "family": family_json(w),
        "known": known_json(w, 12),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_keys_round_trip() {
        for r in [Role::HeadCoach, Role::OffensiveCoordinator, Role::DefensiveCoordinator]
            .into_iter()
            .chain(Group::ALL.iter().map(|&g| Role::PositionCoach(g)))
        {
            assert_eq!(role(&role_key(r)), Ok(r));
        }
        assert!(role("pc9").is_err());
    }

    #[test]
    fn school_years_read_plainly() {
        assert_eq!(school_words(16), "A junior in high school");
        assert_eq!(school_words(3), "Not in school yet");
    }
}
