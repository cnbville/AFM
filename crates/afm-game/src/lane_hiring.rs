//! Lane commands: finding and hiring coaches (lane 18; the coaching
//! update's C21-C61). Every coach in the world can be searched, read,
//! looked into and offered any job you have open; his answer comes from
//! `afm_engine::world::folk::hiring` and your reads from `coach_look`.
//! The screens never see a hidden number: reads are ranges, potential is
//! words, a room's growth is words.

use afm_engine::world::World;
use afm_engine::world::coach::{Group, Role};
use afm_engine::world::folk::coach_look::{self, FACETS, Source};
use afm_engine::world::folk::hiring::{self, ClauseTerm, Terms};
use afm_engine::world::folk::staffing::{self, Filter};
use afm_engine::world::folk::{self as folk, Team, talks};
use afm_engine::world::units::{self, Pool, Stat};
use serde_json::{Value, json};

use crate::lane_people::{cap, group_key, money, num, role_arg, role_key};
use crate::views::date_text;
use crate::{Game, Reply};

const COMMANDS: [&str; 7] =
    ["hiring_search", "hiring_coach", "hiring_film", "hiring_around", "hiring_source", "hiring_offer", "hiring_watch"];

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    if !COMMANDS.contains(&cmd) {
        return None;
    }
    let Some(w) = game.world.as_mut() else { return Some(Err("No world is loaded.".into())) };
    Some(match cmd {
        "hiring_search" => role_arg(args).map(|r| search(w, r, args)),
        "hiring_coach" => coach_arg(w, args).map(|c| page(w, c)),
        "hiring_film" => coach_arg(w, args).and_then(|c| {
            let text = coach_look::watch_film(w, c)?;
            Ok(json!({ "text": text, "page": page(w, c) }))
        }),
        "hiring_around" => coach_arg(w, args).and_then(|c| {
            let text = coach_look::ask_around(w, c)?;
            Ok(json!({ "text": text, "page": page(w, c) }))
        }),
        "hiring_source" => coach_arg(w, args).and_then(|c| {
            let src = args.get("source").and_then(Value::as_str).and_then(Source::parse).ok_or("Missing source.")?;
            let text = coach_look::ask_for_read(w, c, src)?;
            Ok(json!({ "text": text, "page": page(w, c) }))
        }),
        "hiring_offer" => coach_arg(w, args).and_then(|c| {
            let r = role_arg(args)?;
            let text = hiring::offer(w, c, r, terms(args)?)?;
            Ok(json!({ "text": text, "page": page(w, c) }))
        }),
        "hiring_watch" => Ok(watch(w, args)),
        _ => Err(format!("Unknown command: {cmd}")),
    })
}

fn coach_arg(w: &World, args: &Value) -> Result<usize, String> {
    let c = num(args, "coach")? as usize;
    if c >= w.coaches.len() {
        return Err("No such coach.".into());
    }
    Ok(c)
}

/// An offer's terms from `{salary, years, promises: [..], clauses: [{mark, reward}]}`.
pub(crate) fn terms(args: &Value) -> Result<Terms, String> {
    let salary = args.get("salary").and_then(Value::as_f64).ok_or("Missing salary.")?;
    let years = args.get("years").and_then(Value::as_u64).unwrap_or(2).clamp(1, 5) as u8;
    let mut promises = 0;
    for p in args.get("promises").and_then(Value::as_array).into_iter().flatten() {
        promises |= match p.as_str().unwrap_or("") {
            "calling" => hiring::PROMISE_CALLING,
            "title" => hiring::PROMISE_TITLE,
            "recruit" => hiring::PROMISE_RECRUIT,
            "assistant" => hiring::PROMISE_ASSISTANT,
            _ => 0,
        };
    }
    let clauses = args
        .get("clauses")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|x| {
            let mark = x.get("mark")?.as_u64()? as u8;
            let reward = x.get("reward")?.as_u64()? as u8;
            ((1..=3).contains(&mark) && (1..=2).contains(&reward)).then_some(ClauseTerm { mark, reward })
        })
        .take(2)
        .collect();
    Ok(Terms { salary, years, promises, clauses })
}

fn range(r: (u8, u8)) -> Value {
    json!([r.0, r.1])
}

/// The Hiring search (C21, C26): every coach who could take the job,
/// filtered and paged.
pub(crate) fn search(w: &World, r: Role, args: &Value) -> Value {
    let s = |k: &str| args.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let n = |k: &str| args.get(k).and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|x| x.parse().ok())));
    let f = Filter {
        level: s("level"),
        job: s("job"),
        came_up: args
            .get("cameUp")
            .and_then(Value::as_str)
            .and_then(|g| Group::ALL.iter().position(|x| group_key(*x) == g).map(|k| k as u8)),
        age_min: n("ageMin").map(|x| x as u32),
        age_max: n("ageMax").map(|x| x as u32),
        rep_min: n("repMin"),
        known: args.get("known").and_then(Value::as_bool).unwrap_or(false),
        free: args.get("free").and_then(Value::as_bool).unwrap_or(false),
        sort: s("sort"),
        page: n("page").unwrap_or(0.0) as usize,
    };
    let res = staffing::candidates(w, r, &f);
    let coord = matches!(r, Role::OffensiveCoordinator | Role::DefensiveCoordinator);
    let rows: Vec<Value> = res
        .list
        .iter()
        .map(|c| {
            json!({
                "coach": c.coach,
                "link": folk::coach_link(w, c.coach),
                "age": w.coaches[c.coach].age_on(w.today),
                "now": c.now,
                "level": c.level,
                "rep": c.rep,
                "cameUp": c.came_up,
                "known": c.known,
                "read": range(c.read),
                "scheme": c.scheme.map(range),
                "calling": c.calling.map(range),
                "potential": c.potential,
                "hours": (c.hours * 2.0).round() / 2.0,
                "asks": money(c.asks),
                "asksValue": c.asks,
                "blocked": c.blocked,
                "outside": c.outside,
                "talk": hiring::talk_words(w, c.coach),
            })
        })
        .collect();
    json!({
        "role": role_key(r),
        "roleName": cap(r.name()),
        "coord": coord,
        "rows": rows,
        "total": res.total,
        "page": res.page,
        "pages": res.pages,
        "hours": talks::hours_left(w),
    })
}

/// A room's growth in words (C27): never the number.
fn growth_words(g: f64) -> &'static str {
    if g >= 0.6 {
        "His players grow far above the norm"
    } else if g >= 0.25 {
        "His players grow above the norm"
    } else if g > -0.25 {
        "His players grow about as the norm"
    } else {
        "His players grow below the norm"
    }
}

fn straight_words(s: f32) -> &'static str {
    if s >= 0.8 {
        "straight with you"
    } else if s >= 0.55 {
        "mostly straight"
    } else {
        "had his own reasons"
    }
}

fn believe_words(x: f64) -> &'static str {
    if x >= 0.75 {
        "He'd believe it"
    } else if x >= 0.5 {
        "He'd half believe it"
    } else {
        "He'd doubt it"
    }
}

/// Any coach's page from your side (C24, C27, C35, C41, C43): your reads
/// as ranges, his potential in words, what you've put in, who could tell
/// you about him, his units' record, the talks and what you could offer.
pub(crate) fn page(w: &World, c: usize) -> Value {
    let co = &w.coaches[c];
    let x = coach_look::Ctx::new(w);
    let facets: Vec<Value> = FACETS
        .iter()
        .enumerate()
        .map(|(f, (key, label, group))| {
            json!({ "key": key, "label": label, "group": group, "read": range(coach_look::read_in(w, &x, c, f)) })
        })
        .collect();
    let amb = coach_look::read_in(w, &x, c, coach_look::AMBITION);
    let (pot, sure) = coach_look::potential_in(w, &x, c);
    let look = coach_look::invested(w, c);
    let asked: Vec<Value> = look
        .map(|l| {
            l.asked
                .iter()
                .rev()
                .take(6)
                .map(|a| json!({ "who": a.who, "date": date_text(a.date), "words": a.words, "straight": straight_words(a.straight) }))
                .collect()
        })
        .unwrap_or_default();
    let sources: Vec<Value> = coach_look::sources(w, c)
        .into_iter()
        .map(|(label, src, warm)| json!({ "label": label, "code": src.code(), "warm": (warm * 100.0).round() }))
        .collect();
    // His units (C27): a coordinator's sides by season; a position coach's rooms.
    let units: Vec<Value> = units::coordinator_record(w, c)
        .into_iter()
        .map(|x| {
            let (y, team, off, rank) = (x.year, x.team, x.offense, x.rank);
            let ppg = if off { x.season.stat(Stat::Points) } else { x.season.stat(Stat::PointsAllowed) };
            json!({
                "year": y,
                "team": team.link(w),
                "side": if off { "Offense" } else { "Defense" },
                "ppg": format!("{ppg:.1}"),
                "what": if off { "points a game" } else { "points allowed a game" },
                "rank": rank.map(|r| format!("{} of {}", r.0, r.1)),
                "pool": Pool::of(w, team).map(|p| p.name(w)),
            })
        })
        .collect();
    let rooms: Vec<Value> = Group::ALL
        .iter()
        .filter_map(|&g| {
            let u = &co.units[g as usize];
            (u.seasons >= 3.0).then(|| {
                json!({
                    "group": cap(g.name()),
                    "seasons": (u.seasons / 6.0).round().max(1.0),
                    "growth": growth_words(u.growth_read()),
                    "movedUp": u.moved_up.round(),
                })
            })
        })
        .collect();
    let seat = Team::seat(w);
    let yours = co.job.is_some_and(|j| Some(Team::from_employer(j.employer)) == seat);
    let openings: Vec<Value> = if yours {
        Vec::new()
    } else {
        staffing::openings(w)
            .into_iter()
            .map(|r| {
                let outside = match r {
                    Role::PositionCoach(g) => co.came_up != g,
                    Role::OffensiveCoordinator => !co.came_up.offense(),
                    Role::DefensiveCoordinator => co.came_up.offense(),
                    Role::HeadCoach => false,
                };
                json!({
                    "key": role_key(r),
                    "name": cap(r.name()),
                    "asks": money(hiring::ask(w, c, r)),
                    "asksValue": hiring::ask(w, c, r),
                    "outside": outside,
                    "read": range(coach_look::read_job_in(w, &x, c, r)),
                })
            })
            .collect()
    };
    let nfl = matches!(seat, Some(Team::Nfl(_)));
    let college = matches!(seat, Some(Team::College(_)));
    let mut promises = vec![
        json!({ "key": "calling", "name": "He calls his side's plays", "believe": believe_words(hiring::believe_word(w)) }),
        json!({ "key": "title", "name": "A title (co-coordinator, or assistant head coach for a coordinator)", "believe": believe_words(hiring::believe_word(w)) }),
        json!({ "key": "assistant", "name": "An assistant he picks", "believe": believe_words(hiring::believe_word(w)) }),
    ];
    if college {
        promises.insert(
            2,
            json!({ "key": "recruit", "name": "A top recruit at his position", "believe": believe_words(hiring::believe_recruit(w, co.came_up)) }),
        );
    }
    let clauses: Vec<Value> = (1..=3u8)
        .flat_map(|m| (1..=2u8).map(move |r| ClauseTerm { mark: m, reward: r }))
        .map(|t| json!({ "mark": t.mark, "reward": t.reward, "words": cap(&t.words(nfl)) }))
        .collect();
    let level = staffing::level_of(w, c);
    json!({
        "coach": c,
        "name": co.name,
        "link": folk::coach_link(w, c),
        "age": co.age_on(w.today),
        "now": staffing::now_words(w, c),
        "level": level,
        "cameUp": cap(co.came_up.name()),
        "rep": folk::rep_words(co.reputation),
        "yours": yours,
        "facets": facets,
        "ambition": range(amb),
        "potential": pot,
        "sure": sure,
        "film": look.map_or(0.0, |l| l.film),
        "talked": look.map_or(0.0, |l| l.talk),
        "asked": asked,
        "sources": sources,
        "units": units,
        "rooms": rooms,
        "talk": hiring::talk_words(w, c),
        "talking": hiring::talking(w, c),
        "log": hiring::talk(w, c).map(|t| t.log.iter().rev().take(8).map(|(d, s)| json!({ "date": date_text(*d), "text": s })).collect::<Vec<_>>()),
        "openings": openings,
        "promises": promises,
        "clauses": clauses,
        "hoursLeft": talks::hours_left(w),
    })
}

/// Coaches to watch (C42): units well above their level's average last
/// season, each linked to the coaches in charge, and position coaches
/// whose rooms grow well above the norm.
fn watch(w: &World, args: &Value) -> Value {
    let level = args.get("level").and_then(Value::as_str).unwrap_or("");
    let year = args.get("year").and_then(Value::as_i64).map_or(w.year - 1, |y| y as i32);
    let mut pools: Vec<Pool> = Vec::new();
    match level {
        "nfl" => pools.push(Pool::Nfl),
        "hs" => pools.extend((0..w.states.len()).map(Pool::State)),
        "" | "college" => pools.extend(afm_engine::world::data::Division::ALL.iter().map(|&d| Pool::College(d))),
        d => {
            if let Some(d) = afm_engine::world::data::Division::parse(&d.to_uppercase()) {
                pools.push(Pool::College(d));
            } else if let Some(s) = w.states.iter().position(|x| x.code.eq_ignore_ascii_case(d)) {
                pools.push(Pool::State(s));
            }
        }
    }
    let stat = args.get("stat").and_then(Value::as_str).unwrap_or("");
    let you = w.career.as_ref().map(|k| k.coach);
    let rows: Vec<Value> = units::standouts(w, &pools, year, 1.5)
        .into_iter()
        .filter(|s| stat.is_empty() || s.stat == stat)
        .take(120)
        .map(|s| {
            let coaches: Vec<Value> = s
                .coaches
                .iter()
                .filter(|(c, _)| Some(*c) != you)
                .map(|(c, role)| json!({ "link": folk::coach_link(w, *c), "coach": c, "role": role }))
                .collect();
            json!({
                "team": s.team.link(w),
                "pool": Pool::of(w, s.team).map(|p| p.name(w)),
                "stat": s.stat,
                "value": format!("{:.1}", s.value),
                "average": format!("{:.1}", s.average),
                "rank": format!("{} of {}", s.rank, s.of),
                "coaches": coaches,
            })
        })
        .collect();
    let rooms: Vec<Value> = units::growth_standouts(w, 0.35)
        .into_iter()
        .filter(|(c, _, _)| Some(*c) != you)
        .filter(|(c, _, _)| match level {
            "" => true,
            l => {
                staffing::level_of(w, *c) == l
                    || (l == "college" && !matches!(staffing::level_of(w, *c), "nfl" | "hs" | "other" | "none"))
            }
        })
        .take(60)
        .map(|(c, g, gr)| {
            json!({
                "link": folk::coach_link(w, c),
                "coach": c,
                "now": staffing::now_words(w, c),
                "group": cap(g.name()),
                "growth": growth_words(gr),
            })
        })
        .collect();
    let stats: Vec<Value> = Stat::ALL.iter().map(|s| json!(s.name())).collect();
    json!({ "year": year, "rows": rows, "rooms": rooms, "stats": stats })
}
