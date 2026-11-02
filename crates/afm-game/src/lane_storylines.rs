//! Lane commands: storylines (update 2, lane 1, S11). `call` answers the
//! commands this lane owns and returns None for any other.
//!
//! The screens of the storyline engine (`afm_engine::world::storylines`),
//! boards 28-33 of the update 2 mock-ups:
//! - This week (28): the stories you're in, the ones you follow and the ones
//!   that ended, the hottest elsewhere, what's coming up in yours;
//! - the story page (29): its heat and beats, what your staff knows against
//!   what the public believes, the sides and the cast, what they're saying,
//!   your moves and the stories it touches, with follow;
//! - the Continue stop for a story meeting and "Previously" on load (30);
//! - the season's chapters (31) and your career's, with the office shelf
//!   and the names you've been given (33);
//! - the job pick's read of each job you could take (32).
//!
//! The words come from the engine's `view` and `credit`; this lane lays
//! them out. Story text is linked once per page (`link_tree`).

use afm_engine::calendar::events::add_ref;
use afm_engine::calendar::{Date, Level, Link, Weekday};
use afm_engine::world::World;
use afm_engine::world::coach::{Employer, Role};
use afm_engine::world::folk::{self, scene::Kind as SceneKind};
use afm_engine::world::front_office::market;
use afm_engine::world::stories::nick::{self, NickKind, Who as NWho};
use afm_engine::world::stories::story::MementoKind;
use afm_engine::world::stories::{self as st, TeamKey};
use afm_engine::world::storylines::{
    self as sl, BeatKind, Family, Part, Phase, Story, Voice, Who, credit, moves, view,
};
use serde_json::{Value, json};

use crate::lane_core::links::{self, Linker};
use crate::views::{self, TeamRef, date_text};
use crate::{Game, Reply};

const COMMANDS: [&str; 9] = [
    "storylines_week",
    "storylines_story",
    "storylines_follow",
    "storylines_move",
    "storylines_meeting",
    "storylines_previously",
    "storylines_chapters",
    "storylines_career",
    "storylines_jobs",
];

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    if !COMMANDS.contains(&cmd) {
        return None;
    }
    let Some(w) = game.world.as_mut() else { return Some(Err("No world is loaded.".into())) };
    Some(match cmd {
        "storylines_week" => Ok(week(w)),
        "storylines_story" => num(args, "id").and_then(|id| story_page(w, id as u32)),
        "storylines_follow" => (|| {
            let id = num(args, "id")? as u32;
            let on = args.get("on").and_then(Value::as_bool).unwrap_or(true);
            let now = view::follow(w, id, on);
            Ok(json!({ "following": now }))
        })(),
        "storylines_move" => (|| {
            let id = num(args, "id")? as u32;
            let code = num(args, "code")? as u32;
            let said = view::choose(w, id, code)?;
            let mut v = story_page(w, id)?;
            v["said"] = json!(said);
            Ok(v)
        })(),
        "storylines_meeting" => num(args, "id").and_then(|id| meeting(w, id as u32)),
        "storylines_previously" => Ok(previously(w)),
        "storylines_chapters" => Ok(chapters(w, args.get("season").and_then(Value::as_i64).map(|y| y as i32))),
        "storylines_career" => Ok(career(w)),
        "storylines_jobs" => Ok(jobs(w)),
        _ => unreachable!(),
    })
}

fn num(args: &Value, k: &str) -> Result<u64, String> {
    args.get(k)
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .ok_or_else(|| format!("Missing {k}."))
}

// --- Small helpers -------------------------------------------------------------------------------

fn tref(t: TeamKey) -> TeamRef {
    match t {
        TeamKey::Hs(h) => TeamRef::HighSchool(h as usize),
        TeamKey::College(p) => TeamRef::College(p as usize),
        TeamKey::Nfl(n) => TeamRef::Nfl(n as usize),
        TeamKey::Other(f) => TeamRef::Other(f as usize),
    }
}

fn brief(w: &World, t: TeamKey) -> Value {
    views::team_brief(w, tref(t))
}

fn seat_kind(w: &World) -> &'static str {
    match st::seat_team(w) {
        Some(TeamKey::Nfl(_)) => "nfl",
        Some(TeamKey::College(_)) => "college",
        Some(TeamKey::Hs(_)) => "hs",
        Some(TeamKey::Other(_)) => "other",
        None => "",
    }
}

/// "Jan 9": a beat's day on a chart or a table.
fn short_day(d: Date) -> String {
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!("{} {}", M[d.month() as usize - 1], d.day())
}

/// How long ago, for a quote's byline.
fn ago(w: &World, d: Date) -> String {
    match d.days_to(w.today) {
        i32::MIN..=0 => "today".into(),
        1 => "yesterday".into(),
        n @ 2..=6 => format!("{n} days ago"),
        _ => short_day(d),
    }
}

/// The page's context line: your team, its level, its record, the day.
fn ctx_line(w: &World) -> String {
    let Some(t) = views::mine(w) else { return date_text(w.today) };
    let level = match t {
        TeamRef::Nfl(i) => views::nfl_division(w, i).unwrap_or("NFL").to_string(),
        TeamRef::College(i) => w.conference_name(&w.programs[i].conference),
        TeamRef::HighSchool(i) => {
            let s = &w.high_schools[i];
            format!("{} {}", w.states[s.state].code, views::hs_class(w, s.id))
        }
        TeamRef::Other(i) => views::other_line(w, i),
    };
    let rec = views::record_text(w, t);
    [views::team_name(w, t), level, rec, date_text(w.today)]
        .into_iter()
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The board's six phases: Seed to Aftermath (a fizzle sits with the
/// endings).
fn phase_index(p: Phase) -> u8 {
    match p {
        Phase::Seed => 0,
        Phase::Spark => 1,
        Phase::Building => 2,
        Phase::Peak => 3,
        Phase::Resolved | Phase::Fizzled => 4,
        Phase::Aftermath => 5,
    }
}

/// A story's level as a short tag: a conference, a division, a state and
/// class.
fn tag(w: &World, s: &Story) -> String {
    if s.family() == Family::Recruiting && s.level == Level::Recruiting {
        return "Recruiting".into();
    }
    match s.team.map(tref) {
        Some(TeamRef::College(p)) => w.conference_name(&w.programs[p].conference),
        Some(TeamRef::Nfl(i)) => views::nfl_division(w, i).unwrap_or("NFL").to_string(),
        Some(TeamRef::HighSchool(h)) => {
            let sc = &w.high_schools[h];
            format!("{} {}", w.states[sc.state].code, views::hs_class(w, sc.id))
        }
        Some(TeamRef::Other(f)) => views::other_line(w, f),
        None => match s.level {
            Level::Nfl => "NFL".into(),
            Level::College => "College".into(),
            Level::HighSchool => "High school".into(),
            _ => String::new(),
        },
    }
}

/// The beats you can see: what the public saw, your staff's memos, your
/// own private moves.
fn seen(b: &sl::Beat) -> bool {
    !b.text.is_empty() && (b.what.public() || matches!(b.what, BeatKind::Memo | BeatKind::Private))
}

fn ours(b: &sl::Beat) -> bool {
    matches!(b.by, Voice::Staff | Voice::You) || matches!(b.what, BeatKind::Memo | BeatKind::Private)
}

/// What forces it next, with its day: a game for the families a game
/// settles, else the story's own deadline.
fn next_text(w: &World, s: &Story) -> (String, Option<Date>) {
    let (what, due) = view::next(w, s);
    if what.is_empty() {
        return (String::new(), None);
    }
    let game = s.team.and_then(|t| views::next_game(w, tref(t)));
    let game_words = |g: &Value| {
        let opp = g["opp"]["name"].as_str().unwrap_or("").to_string();
        let home = g["home"].as_bool().unwrap_or(true);
        let date = g["date"].as_str().unwrap_or("").to_string();
        (format!("{} {opp}", if home { "home to" } else { "at" }), date)
    };
    match (s.family(), &game) {
        (Family::Rivalry, _) if due.is_some() && s.other.is_some() => {
            let other = s.other.map(|t| st::short_name(w, t)).unwrap_or_default();
            (format!("the game against {other}, {}", due.map(date_text).unwrap_or_default()), due)
        }
        (Family::Streak | Family::Rivalry | Family::Comeback | Family::Rise, Some(g)) => {
            let (t, d) = game_words(g);
            (format!("{t}, {d}"), due)
        }
        (Family::Battle, Some(g)) => {
            let (t, d) = game_words(g);
            (format!("your depth chart before {}, {d}", t.replace("home to ", "")), due)
        }
        _ => match due {
            Some(d) => (format!("{what}, {}", date_text(d)), Some(d)),
            None => (format!("{what}; no date set. It moves when someone talks."), None),
        },
    }
}

/// Every string at the story text keys, linked with one reading of the
/// world's names.
const LINKED: [&str; 8] = ["q", "text", "lq", "body", "title", "line", "quote", "public"];

fn link_tree(w: &World, v: &mut Value) {
    fn gather<'a>(v: &'a Value, out: &mut Vec<&'a str>) {
        match v {
            Value::Object(m) => {
                for (k, x) in m {
                    match x {
                        Value::String(s) if LINKED.contains(&k.as_str()) => out.push(s),
                        _ => gather(x, out),
                    }
                }
            }
            Value::Array(a) => a.iter().for_each(|x| gather(x, out)),
            _ => {}
        }
    }
    // The stories in the tree: a card or page by its "id" beside its "q",
    // a meeting or chapter by its "story".
    fn ids(v: &Value, out: &mut Vec<u32>) {
        match v {
            Value::Object(m) => {
                let id = match (m.get("story"), m.get("q")) {
                    (Some(s), _) => s.as_u64(),
                    (None, Some(_)) => m.get("id").and_then(Value::as_u64),
                    _ => None,
                };
                out.extend(id.map(|x| x as u32));
                m.values().for_each(|x| ids(x, out));
            }
            Value::Array(a) => a.iter().for_each(|x| ids(x, out)),
            _ => {}
        }
    }
    fn apply(v: &mut Value, l: &Linker, ctx: &[TeamRef], known: &[(String, Link)]) {
        match v {
            Value::Object(m) => {
                for (k, x) in m.iter_mut() {
                    match x {
                        Value::String(s) if LINKED.contains(&k.as_str()) => {
                            let mut refs = Vec::new();
                            for (name, to) in known.iter().filter(|(n, _)| s.contains(n.as_str())) {
                                add_ref(&mut refs, s, *to, name);
                            }
                            *s = l.link_refs(s, &refs, ctx);
                        }
                        _ => apply(x, l, ctx, known),
                    }
                }
            }
            Value::Array(a) => a.iter_mut().for_each(|x| apply(x, l, ctx, known)),
            _ => {}
        }
    }
    // The people in those stories link by id: a common name ("Chase
    // Thomas") still finds the one the story is about. A name two of them
    // share is left to the linker.
    let mut known: Vec<(String, Link)> = Vec::new();
    let mut story_ids = Vec::new();
    ids(v, &mut story_ids);
    story_ids.sort_unstable();
    story_ids.dedup();
    for s in story_ids.iter().filter_map(|&id| w.storylines.get(id)) {
        for m in &s.cast {
            let found = match m.who {
                Who::Player(id) => views::person_index(w, id).map(|i| (w.people[i].name.clone(), Link::Person(id))),
                Who::Coach(c) => w.coaches.get(c as usize).map(|co| (co.name.clone(), Link::Coach(c))),
                _ => None,
            };
            if let Some((name, to)) = found {
                known.push((name, to));
            }
        }
    }
    known.sort_by(|a, b| a.0.cmp(&b.0));
    known.dedup();
    let shared: Vec<String> = known.windows(2).filter(|p| p[0].0 == p[1].0).map(|p| p[0].0.clone()).collect();
    known.retain(|k| !shared.contains(&k.0));
    // Longest first, so "Tom Smith Jr." wins over "Tom Smith".
    known.sort_by_key(|k| std::cmp::Reverse(k.0.len()));
    let texts: Vec<String> = {
        let mut out = Vec::new();
        gather(v, &mut out);
        out.into_iter().map(str::to_string).collect()
    };
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let l = Linker::new(w, &refs, true);
    let ctx: Vec<TeamRef> = views::mine(w).into_iter().collect();
    apply(v, &l, &ctx, &known);
}

// --- Board 28: This week -------------------------------------------------------------------------

/// How many of your stories wait on you, for the sidebar's badge.
pub(crate) fn need_count(w: &World) -> usize {
    if w.career.is_none() {
        return 0;
    }
    view::mine(w).into_iter().filter(|s| view::needs_you(w, s)).count()
}

/// One story as a card: its question, heat and the last weeks of it, your
/// part, its phase, how the people split, the latest beat, what's next.
fn card(w: &World, s: &Story) -> Value {
    let sp = view::split(w, s);
    let (next, due) = next_text(w, s);
    let latest = sl::tell::last_said(s);
    let beats = s.beats.iter().filter(|b| seen(b)).count();
    let mut path = view::heat_path(s);
    if path.len() > 14 {
        path.drain(..path.len() - 14);
    }
    json!({
        "id": s.id,
        "fam": s.family().name(),
        "kind": s.kind_name(),
        "q": view::question(w, s),
        "heat": s.heat.round(),
        "delta": s.heat.round() as i32 - view::heat_week_ago(s) as i32,
        "path": path,
        "role": view::role(w, s),
        "phase": phase_index(s.phase),
        "phaseName": s.phase.name(),
        "public": s.public,
        "needs": view::needs_you(w, s),
        "split": { "a": sp.a, "b": sp.b, "share": sp.share, "audience": sp.audience },
        "lq": latest.map(|b| b.text.clone()),
        "ls": latest.map(|b| format!("{} · {}", view::source(s, b), ago(w, b.date))),
        "next": next,
        "due": due.map(date_text),
        "beats": beats,
        "since": short_day(s.public_on.unwrap_or(s.opened)),
        "following": view::following(w, s.id),
        "tag": tag(w, s),
        "mine": s.you.is_some(),
    })
}

fn ended_row(w: &World, s: &Story) -> Value {
    let when = s.closed.map(date_text).unwrap_or_default();
    json!({
        "id": s.id,
        "q": view::question(w, s),
        "text": match view::ended_text(s) {
            Some(t) => format!("Ended {when}: {t}"),
            None => format!("Ended {when}."),
        },
        "fam": s.family().name(),
        "heat": s.peak_heat.round(),
        "outcome": s.ending_key().replace('_', " "),
        "mine": s.you.is_some(),
    })
}

fn week(w: &World) -> Value {
    let mut mine: Vec<&Story> = view::mine(w);
    mine.extend(view::private(w));
    mine.sort_by(|a, b| b.heat.total_cmp(&a.heat).then(a.id.cmp(&b.id)));
    let following: Vec<&Story> = {
        let mut v: Vec<&Story> = w
            .storylines
            .followed
            .iter()
            .filter_map(|&id| w.storylines.get(id))
            .filter(|s| s.phase.open() && s.you.is_none())
            .collect();
        v.sort_by(|a, b| b.heat.total_cmp(&a.heat));
        v
    };
    // Ended since the season turned, whenever they started.
    let ended: Vec<&Story> = view::ended_since(w, Date::ymd(w.year, 2, 15));
    let week_ago = w.today.add(-7);
    let ended_week: Vec<Value> =
        ended.iter().filter(|s| s.closed.is_some_and(|c| c >= week_ago)).take(3).map(|s| ended_row(w, s)).collect();
    let others: Vec<Value> = view::elsewhere(w, 8)
        .into_iter()
        .map(|s| {
            json!({ "id": s.id, "q": view::question(w, s), "fam": s.family().name(), "phase": s.phase.name(), "tag": tag(w, s), "heat": s.heat.round(), "following": view::following(w, s.id) })
        })
        .collect();
    // What's coming up in your stories, by day.
    let mut up: Vec<(Date, String)> = Vec::new();
    for s in &mine {
        let (what, due) = view::next(w, s);
        if let Some(d) = due {
            let line = format!("{}: {}", upper(what), view::question(w, s));
            match up.iter_mut().find(|x| x.0 == d) {
                Some(x) => x.1 = format!("{} · {line}", x.1),
                None => up.push((d, line)),
            }
        }
    }
    up.sort_by_key(|x| x.0);
    up.truncate(4);
    let where_ = match seat_kind(w) {
        "hs" => "high school football",
        "college" => "college football",
        "nfl" => "the NFL",
        _ => "football",
    };
    let mut v = json!({
        "ctx": ctx_line(w),
        "team": views::mine(w).map(|t| views::team_brief(w, t)),
        "seat": seat_kind(w),
        "career": w.career.is_some(),
        "mine": mine.iter().map(|s| card(w, s)).collect::<Vec<_>>(),
        "following": following.iter().map(|s| card(w, s)).collect::<Vec<_>>(),
        "ended": ended.iter().map(|s| ended_row(w, s)).collect::<Vec<_>>(),
        "endedWeek": ended_week,
        "year": w.year,
        "others": others,
        "where": where_,
        "up": up.into_iter().map(|(d, t)| json!({ "d": short_day(d), "date": date_text(d), "text": t })).collect::<Vec<_>>(),
    });
    link_tree(w, &mut v);
    v
}

fn upper(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

// --- Board 29: the story page --------------------------------------------------------------------

/// The heat over the story's life: (day from its first day, heat), from
/// the Monday readings and today's.
fn heat_line(w: &World, s: &Story) -> (Date, Vec<(i32, u8)>) {
    let first = if s.public { s.public_on.unwrap_or(s.opened) } else { s.opened };
    let start = first.min(s.beats.iter().filter(|b| seen(b)).map(|b| b.date).min().unwrap_or(first));
    let end = s.closed.unwrap_or(w.today);
    let last_monday = end.on_or_before(Weekday::Mon);
    let n = s.weeks.len() as i32;
    let mut pts: Vec<(i32, u8)> = s
        .weeks
        .iter()
        .enumerate()
        .map(|(k, &h)| (start.days_to(last_monday.add(-7 * (n - 1 - k as i32))), h))
        .filter(|p| p.0 >= 0)
        .collect();
    if pts.first().is_none_or(|p| p.0 > 0) {
        pts.insert(0, (0, s.beats.iter().find(|b| seen(b)).map_or(0, |b| b.heat.max(0) as u8)));
    }
    let now = start.days_to(end).max(0);
    if pts.last().is_none_or(|p| p.0 < now) {
        pts.push((now, s.heat.round().clamp(0.0, 100.0) as u8));
    }
    (start, pts)
}

fn heat_at(pts: &[(i32, u8)], d: i32) -> f32 {
    let mut prev = pts.first().copied().unwrap_or((0, 0));
    for &p in pts {
        if p.0 >= d {
            if p.0 == prev.0 {
                return p.1 as f32;
            }
            let t = (d - prev.0) as f32 / (p.0 - prev.0) as f32;
            return prev.1 as f32 + t * (p.1 as f32 - prev.1 as f32);
        }
        prev = p;
    }
    prev.1 as f32
}

/// A cast member as a row: name (a link), who they are, their side, their
/// mood.
fn cast_row(w: &World, s: &Story, m: &sl::Member) -> Value {
    let name = credit::member_name_in(w, s, m.who).unwrap_or_else(|| "Someone".into());
    let last = |who: Option<Who>| {
        who.and_then(|x| credit::member_name_in(w, s, x)).map(|n| n.split_whitespace().last().unwrap_or(&n).to_string())
    };
    let (link, role, mood) = match m.who {
        Who::Player(id) => match views::person_index(w, id) {
            Some(i) => {
                let p = &w.people[i];
                let line = format!("{} {}", views::class_text(w, p), p.player.position.name());
                (links::player(id, &p.name), line, Some(p.mood))
            }
            None => (name.clone(), String::new(), None),
        },
        Who::Coach(c) => {
            let co = &w.coaches[c as usize];
            let line = co.job.map_or_else(
                || "out of coaching".to_string(),
                |j| format!("{}, {}", market::role_title(j.role).to_lowercase(), views::employer_name(w, j.employer)),
            );
            (links::coach(c as usize, &co.name), line, None)
        }
        Who::Team(t) => (links::team_as(tref(t), &name), "the program and its fans".into(), None),
        Who::Boss(t) => (name.clone(), format!("the boss at {}", st::short_name(w, t)), None),
        Who::Parent(_) => (name.clone(), "his family".into(), None),
    };
    let side = match m.part {
        Part::Lead | Part::Rival => "himself".to_string(),
        Part::Suitor => last(s.lead()).map_or("wants him".into(), |n| format!("wants {n}")),
        _ => {
            let (a, b) = (last(s.lead()), last(s.rival()));
            match m.side {
                x if x >= 40 => a.map_or("with him".into(), |n| format!("with {n}")),
                x if x >= 10 => a.map_or("leans with him".into(), |n| format!("leans {n}")),
                x if x > -10 && matches!(m.part, Part::Decider | Part::Boss) => "undecided".into(),
                x if x > -10 => "on the fence".into(),
                x if x > -40 => b.map_or("leans against".into(), |n| format!("leans {n}")),
                _ => b.map_or("against him".into(), |n| format!("with {n}")),
            }
        }
    };
    let (mood_word, tone) = match mood {
        Some(x) => (
            view::mood_word(x),
            if x > 0.15 {
                0
            } else if x > -0.15 {
                1
            } else {
                2
            },
        ),
        None => ("", 1),
    };
    json!({ "name": link, "role": format!("{} · {}", m.part.name(), role).trim_end_matches(" · ").to_string(), "side": side, "mood": mood_word, "tone": tone })
}

fn story_page(w: &mut World, id: u32) -> Reply {
    let s = w.storylines.get(id).cloned().ok_or("No such story.")?;
    if !s.public && s.you.is_none() {
        return Err("No such story.".into());
    }
    let choices = view::choices(w, id);
    let w: &World = w;
    let (start, pts) = heat_line(w, &s);
    let shown: Vec<&sl::Beat> = s.beats.iter().filter(|b| seen(b)).collect();
    // A story older than this year shows its dates with years.
    let years = start.year() != w.today.year();
    let marks: Vec<Value> = shown
        .iter()
        .map(|b| {
            let d = start.days_to(b.date).max(0);
            json!({ "d": d, "h": heat_at(&pts, d).round(), "big": b.heat >= 8, "ours": ours(b) })
        })
        .collect();
    let beats: Vec<Value> = shown
        .iter()
        .rev()
        .take(12)
        .rev()
        .map(|b| {
            let d =
                if years { format!("{} '{:02}", short_day(b.date), b.date.year() % 100) } else { short_day(b.date) };
            json!({ "d": d, "src": view::source(&s, b), "text": b.text, "h": b.heat.max(0), "ours": ours(b) })
        })
        .collect();
    let (next, due) = next_text(w, &s);
    // What your staff knows: the inside read, then its latest memos. A
    // story you only follow has nobody of yours inside it.
    let truth: Vec<String> = if s.you.is_some() {
        let mut v = vec![format!("Inside the building, {}.", view::inside(&s))];
        v.extend(view::staff_reads(&s).iter().take(2).map(|b| b.text.clone()));
        v
    } else {
        vec!["Your staff has nobody inside this one: what you know is what the papers know.".to_string()]
    };
    let (belief, sure) = view::outside(w, &s);
    let sp = view::split(w, &s);
    let money_side = if seat_kind(w) == "nfl" { "Ownership" } else { "Boosters" };
    let sides: Vec<Value> = view::audiences(&s)
        .iter()
        .enumerate()
        .map(|(k, (aud, share))| json!({ "aud": if k == 3 { money_side } else { aud }, "a": sp.a, "b": sp.b, "share": share }))
        .collect();
    let cast: Vec<Value> = s.cast.iter().take(7).map(|m| cast_row(w, &s, m)).collect();
    let voices: Vec<Value> = s
        .beats
        .iter()
        .rev()
        .filter(|b| b.what.public() && !b.text.is_empty() && !matches!(b.what, BeatKind::Fact | BeatKind::Move))
        .take(6)
        .map(|b| json!({ "quote": b.text, "who": format!("{} · {}", view::source(&s, b), ago(w, b.date)) }))
        .collect();
    let rule = (s.family() == Family::Recruiting && seat_kind(w) == "college").then_some(
        "NCAA rules bar coaches from talking in public about a recruit before he signs, so every move here but \"no comment\" is private.",
    );
    let links: Vec<Value> = s
        .links
        .iter()
        .filter_map(|&l| w.storylines.get(l))
        .filter(|o| o.public || o.you.is_some())
        .map(|o| json!({ "id": o.id, "q": view::question(w, o), "open": o.phase.open() }))
        .collect();
    let mut ctx = format!("Story · {} · {}", s.family().name(), s.phase.name());
    let role = view::role(w, &s);
    if !role.is_empty() {
        ctx = format!("{ctx} · you: {role}");
    }
    let mut v = json!({
        "id": s.id,
        "ctx": ctx,
        "team": s.team.map(|t| brief(w, t)),
        "q": view::question(w, &s),
        "kind": s.kind_name(),
        "fam": s.family().name(),
        "heat": s.heat.round(),
        "open": s.phase.open(),
        "phase": phase_index(s.phase),
        "phaseName": s.phase.name(),
        "span": format!(
            "{} beat{} since {}",
            shown.len(),
            if shown.len() == 1 { "" } else { "s" },
            if years { format!("{}, {}", short_day(start), start.year()) } else { short_day(start) }
        ),
        "chart": {
            "days": pts.last().map_or(1, |p| p.0.max(1)) + if s.phase.open() { 7 } else { 0 },
            "now": pts.last().map_or(0, |p| p.0),
            "path": pts.iter().map(|p| json!([p.0, p.1])).collect::<Vec<_>>(),
            "marks": marks,
            "ticks": ticks(start, pts.last().map_or(0, |p| p.0)),
        },
        "beats": beats,
        "next": next,
        "nextD": due.map(date_text).unwrap_or_else(|| if s.phase.open() { "No date".into() } else { String::new() }),
        "ended": if s.phase.open() { Value::Null } else { json!(ended_row(w, &s)) },
        "truth": truth,
        "belief": belief,
        "sure": sure,
        "sides": sides,
        "cast": cast,
        "voices": voices,
        "rule": rule,
        "moves": choices.iter().map(|c| json!({ "code": c.code, "label": c.label, "name": c.name, "tone": c.tone, "hint": c.hint, "public": c.public, "chips": c.chips.iter().map(|x| json!({ "t": x.0, "tone": x.1 })).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "mine": s.you.is_some(),
        "following": view::following(w, s.id),
        "canFollow": s.public,
        "links": links,
    });
    link_tree(w, &mut v);
    Ok(v)
}

/// Date ticks along a chart: its first day, each month's first, today.
fn ticks(start: Date, days: i32) -> Vec<Value> {
    let span = days.max(1);
    let end = start.add(span);
    // Across a new year the ends carry their years and January its own.
    let years = start.year() != end.year();
    let label = |d: Date| if years { format!("{} '{:02}", short_day(d), d.year() % 100) } else { short_day(d) };
    let mut v = vec![json!([0, label(start)])];
    let step = match span {
        ..=150 => 1,
        151..=400 => 2,
        _ => 3,
    };
    let mut m = Date::ymd(start.year(), start.month(), 1);
    let mut k = 0;
    loop {
        m = Date::ymd(
            if m.month() == 12 { m.year() + 1 } else { m.year() },
            if m.month() == 12 { 1 } else { m.month() + 1 },
            1,
        );
        let d = start.days_to(m);
        if d >= span {
            break;
        }
        k += 1;
        if k % step == 0 && d > span / 12 && span - d > span / 12 {
            let t = if years && m.month() == 1 {
                m.year().to_string()
            } else {
                short_day(m).split(' ').next().unwrap_or("").to_string()
            };
            v.push(json!([d, t]));
        }
    }
    v.push(json!([span, label(end)]));
    v
}

// --- Board 30: a story meeting stops Continue; "Previously" on load --------------------------------

/// The Continue stop for a story meeting: who, what's at stake, what you
/// can say and what your staff expects each answer to do.
fn meeting(w: &mut World, id: u32) -> Reply {
    let sc = folk::scene::open(w, id).ok_or("No such meeting.")?;
    if sc.kind != SceneKind::Story {
        return Err("That meeting isn't about a story.".into());
    }
    let sid = sc.a as u32;
    let s = w.storylines.get(sid).cloned().ok_or("It has blown over.")?;
    let w_ref: &World = w;
    let latest = sl::tell::last_said(&s);
    let title = latest.map_or_else(|| view::question(w_ref, &s), |b| b.text.clone());
    let (belief, sure) = view::outside(w_ref, &s);
    // Four facts: who leads it, how he is, what people think, what waiting does.
    let lead = s.cast.iter().find(|m| m.part == Part::Lead);
    let mut facts: Vec<Value> = Vec::new();
    if let Some(m) = lead {
        let row = cast_row(w_ref, &s, m);
        facts.push(json!({ "k": "Who", "v": format!("{}{}", row["name"].as_str().unwrap_or(""), row["role"].as_str().filter(|r| !r.is_empty()).map(|r| format!(", {}", r.split(" · ").last().unwrap_or(r))).unwrap_or_default()) }));
        if let Some(md) = row["mood"].as_str().filter(|x| !x.is_empty()) {
            facts.push(json!({ "k": "His mood", "v": md }));
        }
    }
    facts
        .push(json!({ "k": "The heat", "v": format!("{} of 100, {}", s.heat.round(), s.phase.name().to_lowercase()) }));
    let lean = belief.split(", ").next().unwrap_or(&belief).trim_end_matches('.').to_string();
    facts.push(json!({ "k": "The public", "v": format!("{lean}, {sure}% sure") }));
    facts.push(json!({ "k": "If you wait", "v": "your staff puts out a no-comment" }));
    facts.truncate(4);
    let opts: Vec<Value> = sc
        .picks
        .iter()
        .map(|p| {
            let mv = moves::Move::from_code(p.code.saturating_sub(10));
            json!({ "code": p.code, "label": p.label, "hint": p.hint, "name": p.mv, "chips": moves::chips(w_ref, &s, mv).into_iter().map(|x| json!({ "t": x.0, "tone": x.1 })).collect::<Vec<_>>() })
        })
        .collect();
    let mut v = json!({
        "id": sc.id,
        "story": s.id,
        "q": view::question(w_ref, &s),
        "when": date_text(sc.date),
        "title": title,
        "body": format!("{} {belief}", sc.place),
        "facts": facts,
        "opts": opts,
        "done": sc.done,
        "outcome": sc.outcome,
        "later": "Decide later and your staff answers for you with a no-comment.",
        "go": if s.family() == Family::Battle { "Go with this" } else { "Say it" },
    });
    link_tree(w_ref, &mut v);
    Ok(v)
}

/// "Previously" when a career opens: where your stories stood when you
/// left, once a day.
fn previously(w: &mut World) -> Value {
    if w.career.is_none() || w.storylines.previously_seen == Some(w.today) {
        return json!({ "show": false });
    }
    let mine: Vec<Story> = view::mine(w).into_iter().take(5).cloned().collect();
    let last_game = w.seat_games().last().filter(|g| g.date.days_to(w.today) <= 21).map(|g| {
        let t = views::mine(w);
        let me = t.and_then(|t| match t {
            TeamRef::HighSchool(i) | TeamRef::College(i) | TeamRef::Nfl(i) => g.teams.iter().position(|&x| x == i),
            TeamRef::Other(_) => None,
        });
        let [a, b] = g.stats.score;
        match me {
            Some(k) => {
                let (us, them) = if k == 0 { (a, b) } else { (b, a) };
                let opp = &g.names[1 - k];
                let res = if us > them {
                    "beat"
                } else if us < them {
                    "lost to"
                } else {
                    "tied"
                };
                format!("You {res} {opp} {}-{}.", us.max(them), us.min(them))
            }
            None => format!("{} {} at {} {}.", g.names[1], b, g.names[0], a),
        }
    });
    if mine.is_empty() && last_game.is_none() {
        return json!({ "show": false });
    }
    view::previously_seen(w);
    let w: &World = w;
    let next = views::mine(w).and_then(|t| views::next_game(w, t)).map(|g| {
        format!(
            "Next: {} {} on {}.",
            if g["home"].as_bool().unwrap_or(true) { "home to" } else { "at" },
            g["opp"]["name"].as_str().unwrap_or(""),
            g["date"].as_str().unwrap_or("")
        )
    });
    let hottest = mine.first().map(|s| format!("The loudest story: {}", view::question(w, s)));
    let lead = [last_game, next, hottest].into_iter().flatten().collect::<Vec<_>>().join(" ");
    let rows: Vec<Value> = mine
        .iter()
        .map(|s| {
            let stood = sl::tell::last_said(s).map_or_else(|| view::outside(w, s).0, |b| b.text.clone());
            json!({ "id": s.id, "q": view::question(w, s), "text": format!("{}. {stood}", s.phase.name()), "h": s.heat.round(), "n": upper(&next_text(w, s).0) })
        })
        .collect();
    let team = views::mine(w);
    let mut v = json!({
        "show": true,
        "ctx": ctx_line(w),
        "saved": date_text(w.today),
        "rec": team.map(|t| views::record_text(w, t)).unwrap_or_default(),
        "title": format!("Previously {}", team.map_or_else(|| "in your career".to_string(), |t| format!("at {}", views::team_name(w, t)))),
        "body": lead,
        "rows": rows,
        "foot": "The world doesn't move while the game is closed.",
    });
    link_tree(w, &mut v);
    v
}

// --- Board 31: the season's chapters -------------------------------------------------------------

fn season_of(d: Date) -> i32 {
    if d.month() <= 2 { d.year() - 1 } else { d.year() }
}

fn team_index(t: TeamKey) -> (Level, usize) {
    match t {
        TeamKey::Hs(h) => (Level::HighSchool, h as usize),
        TeamKey::College(p) => (Level::College, p as usize),
        TeamKey::Nfl(n) => (Level::Nfl, n as usize),
        TeamKey::Other(f) => (Level::Other, f as usize),
    }
}

fn chapter_json(w: &World, k: usize, c: &credit::Chapter, slots: &[(Date, String, String)]) -> Value {
    let from = c.from.unwrap_or_else(|| Date::ymd(c.season, 8, 1));
    let to = c.to.unwrap_or(from);
    // Its place on the season's strip: the first and last slot it covers.
    let slot = |d: Date| slots.iter().rposition(|s| s.0 <= d).unwrap_or(0);
    let tone = if c.score >= 0.3 {
        0
    } else if c.score <= -0.3 {
        2
    } else {
        1
    };
    let dates = if from == to { short_day(from) } else { format!("{} – {}", short_day(from), short_day(to)) };
    json!({
        "k": k + 1,
        "story": c.story,
        "title": c.title,
        "from": c.question,
        "fam": afm_engine::world::storylines::catalogue::def(c.kind).family.name(),
        "s0": slot(from),
        "s1": slot(to).max(slot(from)),
        "dates": dates,
        "line": c.line,
        "outcome": c.outcome,
        "tone": tone,
        "text": if c.text.is_empty() { c.line.clone() } else { c.text.clone() },
        "beats": c.beats.iter().map(|b| json!({ "d": short_day(b.0), "text": b.1 })).collect::<Vec<_>>(),
        "cast": c.cast.join(", "),
        "left": c.left.iter().map(|l| json!({ "k": l.0, "t": l.1 })).collect::<Vec<_>>(),
        "paid": c.paid.iter().map(|p| json!({ "k": p.0, "v": p.1, "tone": p.2 })).collect::<Vec<_>>(),
        "team": c.team.map(|t| brief(w, t)),
    })
}

fn chapters(w: &World, season: Option<i32>) -> Value {
    let all = &w.storylines.credit.chapters;
    let mut years: Vec<i32> = all.iter().map(|c| c.season).collect();
    if w.career.is_some() {
        years.push(w.year);
    }
    years.sort_unstable();
    years.dedup();
    years.reverse();
    // Opened without a season: the latest one with a chapter or a game, so
    // the weeks after the year turns still show the season just played.
    let played = |y: i32| all.iter().any(|c| c.season == y) || w.seat_games().iter().any(|g| season_of(g.date) == y);
    let yr = season
        .filter(|y| years.contains(y))
        .or_else(|| years.iter().copied().find(|&y| played(y)))
        .or_else(|| years.first().copied())
        .unwrap_or(w.year);
    let list: Vec<&credit::Chapter> = all.iter().filter(|c| c.season == yr).collect();
    // The season's team: the one its chapters were about, else your team then.
    let team = list
        .iter()
        .find_map(|c| c.team)
        .or_else(|| w.stories.story.seasons.iter().rev().find(|x| x.year as i32 == yr).map(|x| x.team))
        .or_else(|| st::seat_team(w));
    // The strip: camp, each of the season's games, then the winter.
    let mut slots: Vec<(Date, String, String)> = vec![(Date::ymd(yr, 3, 1), "·".into(), "Camp".into())];
    let (mut wins, mut losses, mut ties) = (0, 0, 0);
    if let Some(t) = team {
        let (level, i) = team_index(t);
        for g in w.seat_games().iter().filter(|g| season_of(g.date) == yr && g.level == level && g.teams.contains(&i)) {
            let k = if g.teams[0] == i { 0 } else { 1 };
            let [a, b] = g.stats.score;
            let (us, them) = if k == 0 { (a, b) } else { (b, a) };
            let r = if us > them {
                wins += 1;
                "W"
            } else if us < them {
                losses += 1;
                "L"
            } else {
                ties += 1;
                "T"
            };
            let opp = format!("{}{}", if k == 0 { "" } else { "at " }, g.names[1 - k]);
            slots.push((g.date, r.into(), opp));
        }
    }
    let winter = match team {
        Some(TeamKey::College(_)) => "Signing",
        Some(TeamKey::Nfl(_)) => "Winter",
        _ => "Winter",
    };
    let last = slots.last().map_or(Date::ymd(yr, 12, 1), |s| s.0.add(1));
    slots.push((last, "·".into(), winter.into()));
    let nth = w.stories.story.seasons.iter().filter(|x| Some(x.team) == team && (x.year as i32) < yr).count() + 1;
    let title = w.stories.story.titles.iter().find(|x| x.0 == yr && Some(x.1) == team).map(|x| x.2.clone());
    let rec = if ties > 0 { format!("{wins}-{losses}-{ties}") } else { format!("{wins}-{losses}") };
    let sub = [
        title,
        Some(format!(
            "your {} season{}",
            ordinal(nth),
            team.map_or(String::new(), |t| format!(" at {}", st::short_name(w, t)))
        )),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    let chs: Vec<Value> = list.iter().enumerate().map(|(k, c)| chapter_json(w, k, c, &slots)).collect();
    let mut v = json!({
        "ctx": format!("{} · season review", team.map_or_else(|| "Your career".to_string(), |t| views::team_name(w, tref(t)))),
        "team": team.map(|t| brief(w, t)),
        "yr": yr,
        "years": years,
        "rec": rec,
        "recSub": sub,
        "slots": slots.iter().map(|s| json!({ "r": s.1, "o": s.2 })).collect::<Vec<_>>(),
        "chapters": chs,
        "now": yr == w.year,
    });
    link_tree(w, &mut v);
    v
}

fn ordinal(n: usize) -> String {
    let words = ["first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth"];
    match words.get(n.saturating_sub(1)) {
        Some(x) => x.to_string(),
        None => {
            let suf = match (n % 10, n % 100) {
                (_, 11..=13) => "th",
                (1, _) => "st",
                (2, _) => "nd",
                (3, _) => "rd",
                _ => "th",
            };
            format!("{n}{suf}")
        }
    }
}

// --- Board 33: your career's chapters and the office shelf ---------------------------------------

fn career(w: &World) -> Value {
    let Some(c) = w.career.as_ref() else { return json!({ "none": true }) };
    let co = &w.coaches[c.coach];
    let cal = |d: Date| d.year() as f64 + d.day_of_year() as f64 / 365.0;
    let now = cal(w.today);
    // Your jobs on the calendar: a football year starts after the Super
    // Bowl, and the job you hold started the day you took it.
    let turn = |y: i32| y as f64 + 0.12;
    let mut jobs: Vec<Value> = co
        .history
        .iter()
        .map(|s| {
            json!({ "team": views::employer_brief(w, s.employer), "name": views::employer_name(w, s.employer), "role": market::role_title(s.role), "from": turn(s.from), "to": turn(s.to.max(s.from) + 1), "now": false })
        })
        .collect();
    if let Some(j) = co.job {
        let hired = w.storylines.credit.hired_into.filter(|h| h.0 == employer_team(j.employer)).map(|h| h.2);
        let from = match hired {
            Some(d) => cal(d),
            None if co.history.is_empty() => cal(c.started),
            None => turn(j.since),
        };
        jobs.push(json!({ "team": views::employer_brief(w, j.employer), "name": views::employer_name(w, j.employer), "role": market::role_title(j.role), "from": from, "to": now, "now": true }));
    }
    let first = jobs.iter().filter_map(|j| j["from"].as_f64()).fold(cal(w.today), f64::min).floor();
    // The book: each season's biggest chapter and the hottest of all,
    // oldest first, each with what it gave you.
    let all = credit::career_book(w);
    let chs: Vec<Value> = all
        .iter()
        .enumerate()
        .map(|(k, ch)| {
            let at = ch.to.or(ch.from).map_or(ch.season as f64 + 0.7, |d| d.year() as f64 + d.day_of_year() as f64 / 365.0);
            let gave: Vec<Value> = ch
                .left
                .iter()
                .filter(|l| matches!(l.0.as_str(), "memento" | "tie" | "name"))
                .take(3)
                .map(|l| json!({ "t": format!("{}: {}", l.0, l.1), "tone": if matches!(l.0.as_str(), "memento" | "name") { 0 } else { 1 } }))
                .collect();
            json!({ "k": k + 1, "title": ch.title, "season": ch.season, "at": at, "line": ch.line, "q": ch.question, "gave": gave, "tone": if ch.score >= 0.3 { 0 } else if ch.score <= -0.3 { 2 } else { 1 } })
        })
        .collect();
    // The shelf: every keepsake, the ones a chapter left marked with it.
    let mems: Vec<Value> = w
        .stories
        .story
        .office
        .iter()
        .rev()
        .take(12)
        .map(|m| {
            let from = all.iter().position(|ch| ch.left.iter().any(|l| l.0 == "memento" && l.1 == m.text));
            json!({ "icon": icon(m.kind), "label": m.kind.word(), "text": m.text, "date": short_day(m.date), "year": m.date.year(), "chapter": from.map(|k| k + 1), "team": m.team.map(|t| brief(w, t)) })
        })
        .collect();
    let mut names: Vec<Value> = nick::all_of(&w.stories, NWho::Coach(c.coach as u32))
        .into_iter()
        .map(|n| {
            let tone = match n.kind {
                NickKind::Mocking => 2,
                NickKind::Earned => 0,
                _ => 1,
            };
            let who = if n.outlet.is_empty() { n.by.name().to_string() } else { n.outlet.clone() };
            json!({ "name": format!("\"{}\"", n.name), "who": format!("{}{}", who, if n.retired { ", gone now" } else { "" }), "when": n.coined.year(), "tone": tone })
        })
        .collect();
    names.reverse();
    let rec = format!("{}-{} as a head coach", co.wins, co.losses);
    let seasons = w.stories.story.seasons.len();
    let titles = w.stories.story.titles.len();
    let sub = [
        Some(format!("{} season{} in your career", seasons, if seasons == 1 { "" } else { "s" })),
        (titles > 0).then(|| format!("{titles} title{}", if titles == 1 { "" } else { "s" })),
        Some(format!("{} chapter{}", all.len(), if all.len() == 1 { "" } else { "s" })),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    let ctx = match co.job {
        Some(j) => format!(
            "{} · {} since {} · {}",
            views::employer_name(w, j.employer),
            market::role_title(j.role).to_lowercase(),
            j.first_season(),
            date_text(w.today)
        ),
        None => format!("Out of coaching · {}", date_text(w.today)),
    };
    let mut v = json!({
        "ctx": ctx,
        "you": c.you.name,
        "rec": rec,
        "recSub": sub,
        "from": first,
        "to": now.max(first + 1.0),
        "jobs": jobs,
        "chapters": chs,
        "mems": mems,
        "names": names,
        "fixer": w.storylines.credit.name,
        "office": w.stories.story.office.len(),
    });
    link_tree(w, &mut v);
    v
}

fn icon(k: MementoKind) -> &'static str {
    match k {
        MementoKind::GameBall => "gameball",
        MementoKind::Trophy => "trophy",
        MementoKind::Ring => "ring",
        MementoKind::Photo => "photo",
        MementoKind::Cover => "cover",
        MementoKind::Gift => "gift",
        MementoKind::Plaque => "plaque",
    }
}

// --- Board 32: the job pick ----------------------------------------------------------------------

fn employer_team(e: Employer) -> TeamKey {
    sl::team_key(folk::Team::from_employer(e))
}

fn word_index(word: &str) -> u8 {
    match word {
        "Calm" => 0,
        "Unsettled" => 1,
        "Volatile" => 2,
        _ => 3,
    }
}

/// Each job you could take, read the way S8 reads it: the stories it
/// carries, the word, the premium a hot job pays, what handling it pays
/// and costs.
fn jobs(w: &mut World) -> Value {
    let Some(c) = w.career.as_ref() else { return json!({ "none": true }) };
    let i = c.market.interest.clone();
    // (offer index, employer, role, why it's listed, the offer's pay and years)
    type Row = (Option<usize>, Employer, Role, String, Option<(f64, u8)>);
    let mut list: Vec<Row> = c
        .market
        .offers
        .iter()
        .enumerate()
        .map(|(k, o)| {
            (Some(k), o.employer, o.role, format!("Offered {}: {}", date_text(o.made), o.why), Some((o.pay, o.years)))
        })
        .collect();
    let mine = market::your_job(w).map(|j| (j.employer, j.role));
    let level = w.seat.map(|s| s.level);
    for o in market::openings(w) {
        if list.len() >= 6 {
            break;
        }
        if o.role != Role::HeadCoach || Some((o.employer, o.role)) == mine || list.iter().any(|x| x.1 == o.employer) {
            continue;
        }
        let same_level = matches!(
            (o.employer, level),
            (Employer::College(_), Some(Level::College))
                | (Employer::Nfl(_), Some(Level::Nfl))
                | (Employer::HighSchool(_), Some(Level::HighSchool))
                | (_, None)
        );
        if !same_level || !market::interest_fits(w, &i, o.employer, o.role) {
            continue;
        }
        let why = if o.applied {
            "Your name is in".to_string()
        } else {
            o.decides.map_or("Open now".to_string(), |d| format!("Open · decides around {}", date_text(d)))
        };
        list.push((None, o.employer, o.role, why, None));
    }
    let mut out = Vec::new();
    for (k, e, r, why, offer) in list {
        let t = employer_team(e);
        let read = credit::job_read(w, t);
        let w: &World = w;
        let calm_years = offer.map(|o| o.1.saturating_sub(read.extra_years));
        let pay = offer.map_or_else(|| market::pay_words(w, e, r), |o| money(o.0));
        let terms = vec![
            json!({ "k": "Pay", "a": pay, "b": "the same" }),
            json!({ "k": "Years guaranteed", "a": offer.map_or_else(|| if read.extra_years > 0 { format!("the usual + {}", read.extra_years) } else { "the usual".into() }, |o| o.1.to_string()), "b": calm_years.map_or("the usual".into(), |y| y.to_string()) }),
            json!({ "k": "Buyout if they fire you", "a": if read.buyout_extra > 0 { format!("{}% bigger", read.buyout_extra) } else { "the usual".into() }, "b": "the usual" }),
            json!({ "k": "Patience", "a": read.patience, "b": "judged as usual" }),
        ];
        let stories: Vec<Value> = read
            .stories
            .iter()
            .take(4)
            .map(|s| json!({ "id": s.id, "q": s.question, "heat": s.heat, "phase": s.phase, "public": s.public, "spin": s.spin, "lesson": format!("{:.2}", s.lesson), "tests": s.tests.join(", ") }))
            .collect();
        out.push(json!({
            "k": k,
            "team": views::employer_brief(w, e),
            "name": views::employer_name(w, e),
            "sub": format!("{} · {}", market::role_title(r), views::employer_line(w, e)),
            "ref": views::employer_ref(e).map(|x| x.json()),
            "role_key": crate::lane_frontoffice::setup::role_key(r),
            "calls": why,
            "word": read.word,
            "w": word_index(read.word),
            "volatility": read.volatility,
            "n": read.stories.len(),
            "stories": stories,
            "terms": terms,
            "premium": read.premium,
            "rewards": read.rewards,
            "rep": if read.rep_up >= 0.005 { format!("up to +{:.2}", read.rep_up) } else { "none: no story to handle".into() },
            // A fold from 0.0: an empty f32 sum is -0.0 ("-0.00").
            "lessons": match read.stories.iter().fold(0.0f32, |a, s| a + s.lesson) {
                x if x >= 0.005 => format!("+{x:.2}"),
                _ => "none yet".into(),
            },
            "ties": if read.ties.is_empty() { "none yet".to_string() } else { read.ties.join(", ") },
            "chapter": read.chapter,
            "memento": read.memento,
            "name_given": read.name,
            "bad": if read.rep_down >= 0.005 { format!("\u{2212}{:.2}", read.rep_down) } else { "nothing at stake yet".into() },
            "badP": if read.volatility >= 45.0 { "still kinder for two seasons" } else { "shorter after a losing year" },
            "boss": boss_title(w, e),
            "offer": offer.is_some(),
        }));
    }
    let w: &World = w;
    let c = w.career.as_ref().expect("career");
    let co = &w.coaches[c.coach];
    // Who reads the market for you: your agent above high school, a coaching
    // friend at it.
    let hottest = out
        .iter()
        .max_by(|a, b| a["volatility"].as_f64().unwrap_or(0.0).total_cmp(&b["volatility"].as_f64().unwrap_or(0.0)));
    let calmest = out
        .iter()
        .min_by(|a, b| a["volatility"].as_f64().unwrap_or(0.0).total_cmp(&b["volatility"].as_f64().unwrap_or(0.0)));
    let agent = match (hottest, calmest) {
        (Some(h), Some(c)) if h["name"] != c["name"] && h["w"].as_u64().unwrap_or(0) >= 2 => format!(
            "\"{} pays for the trouble it carries. {} is the quiet one. Both are true.\"",
            h["name"].as_str().unwrap_or(""),
            c["name"].as_str().unwrap_or("")
        ),
        (Some(h), _) => format!(
            "\"{} reads {}. Know what you'd walk into.\"",
            h["name"].as_str().unwrap_or(""),
            h["word"].as_str().unwrap_or("calm").to_lowercase()
        ),
        _ => "\"Nothing worth moving for yet.\"".to_string(),
    };
    let agent_who = if seat_kind(w) == "hs" {
        c.known
            .iter()
            .max_by(|a, b| a.strength.total_cmp(&b.strength))
            .map_or("a coaching friend".to_string(), |k| format!("{}, who knows you", w.coaches[k.coach].name))
    } else {
        "your agent".to_string()
    };
    let leave = match st::seat_team(w) {
        Some(t) => format!(
            "A story at {}: who takes over, and do your players stay? How you go is remembered there.",
            st::short_name(w, t)
        ),
        None => "You have no team to leave: the next job starts clean.".to_string(),
    };
    let mut v = json!({
        "ctx": format!("{} · you coach {}", date_text(w.today), market::your_job(w).map_or("nowhere right now".to_string(), |j| views::employer_name(w, j.employer))),
        "jobs": out,
        "rep": views::reputation_words(co.reputation),
        "repSub": format!("{}-{} as a head coach", co.wins, co.losses),
        "agent": agent,
        "agentWho": agent_who,
        "leave": leave,
        "note": "Interview reads are a sales pitch. Your staff's read on each story comes once you're hired.",
    });
    link_tree(w, &mut v);
    v
}

fn money(d: f64) -> String {
    if d >= 1.0e6 { format!("${:.1}M a year", d / 1.0e6) } else { format!("${}K a year", (d / 1000.0).round()) }
}

fn boss_title(w: &World, e: Employer) -> String {
    let b = folk::boss::boss(w, folk::Team::from_employer(e));
    let role = match e {
        Employer::HighSchool(_) => "The principal",
        Employer::College(_) => "The athletic director",
        Employer::Nfl(_) | Employer::Other(_) => "The owner",
    };
    format!("{role}, {}, in your interview", b.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_read() {
        assert_eq!(ordinal(1), "first");
        assert_eq!(ordinal(12), "12th");
        assert_eq!(ordinal(21), "21st");
    }

    #[test]
    fn heat_reads_between_mondays() {
        let pts = vec![(0, 10), (7, 30), (14, 20)];
        assert_eq!(heat_at(&pts, 0), 10.0);
        assert_eq!(heat_at(&pts, 7), 30.0);
        assert!((heat_at(&pts, 10) - 25.714).abs() < 0.01);
        assert_eq!(heat_at(&pts, 30), 20.0);
    }

    #[test]
    fn seasons_turn_in_march() {
        assert_eq!(season_of(Date::ymd(2027, 1, 10)), 2026);
        assert_eq!(season_of(Date::ymd(2027, 9, 10)), 2027);
    }
}
