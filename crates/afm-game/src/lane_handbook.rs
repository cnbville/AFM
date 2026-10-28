//! Update 3 lane 8, the Handbook and your first week (H8-H15, boards 38
//! and 39). The pages' words live with the screens
//! (`app/ui/screens/handbook-pages.js`); this lane gives each page what
//! only your save knows: an example from your own team read the way your
//! staff reads it (H10), the dates coming up that touch it and the pages
//! due this month. It also runs the guided first week at a new job (H12,
//! H13), the one-time notes the first time the calendar reaches a big
//! moment (H14) and the Help setting (H15).
//!
//! Nothing here shows a hidden number: every rating is your staff's read
//! (`reads.rs`), potential stays words, and the forties come from the
//! staff's read of speed and acceleration on his listed weight. Medians of
//! a level's forties are only the shape of the field, never a person.
//!
//! Another lane adds a page with one arm in `live` (and its words through
//! `hbAdd` in the screens); a page with no arm shows without an example.

use afm_engine::attributes::Attr;
use afm_engine::body::{Drills, body_now};
use afm_engine::calendar::{Date, Period, YearDates, recruiting_period};
use afm_engine::player::{Level, PlayingTime};
use afm_engine::positions::Position;
use afm_engine::world::desk::Head;
use afm_engine::world::help::{Help, Level as HelpLevel, Task, Week};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use crate::lane_core::links;
use crate::lane_core::mail::{self, team_code};
use crate::lane_core::people::{self as staffing, Job};
use crate::lane_ratings::pos_words;
use crate::reads::{self, Read};
use crate::views::{self, TeamRef, date_text};
use crate::{Game, Reply};

const NO_WORLD: &str = "No world is loaded.";

pub fn call(g: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let r = match cmd {
        "l8_page" => page(g, args),
        "l8_help" => g.world.as_ref().map(help_json).ok_or_else(|| NO_WORLD.to_string()),
        "l8_help_set" => (|| {
            let w = g.world.as_mut().ok_or(NO_WORLD)?;
            let lv = parse_level(args.get("level").and_then(Value::as_str).unwrap_or(""))
                .ok_or("Pick Full, Hints only or Off.")?;
            set_level(w, lv);
            Ok(help_json(w))
        })(),
        "l8_started" => g.world.as_ref().map(started).ok_or_else(|| NO_WORLD.to_string()),
        "l8_task" => (|| {
            let w = g.world.as_mut().ok_or(NO_WORLD)?;
            let key = args.get("key").and_then(Value::as_str).ok_or("Missing key.")?;
            let done = args.get("done").and_then(Value::as_bool).unwrap_or(true);
            let t = week_mut(w).and_then(|wk| wk.tasks.iter_mut().find(|t| t.key == key)).ok_or("No such task.")?;
            t.done = done;
            Ok(started(w))
        })(),
        "l8_seen" => (|| {
            let w = g.world.as_mut().ok_or(NO_WORLD)?;
            let screen = args.get("screen").and_then(Value::as_str).unwrap_or("");
            let n = tick(w, |d| d.look.contains(&screen));
            Ok(json!({ "ticked": n }))
        })(),
        _ => return None,
    };
    Some(r)
}

// ---------- the Help setting (H15) ----------

fn parse_level(s: &str) -> Option<HelpLevel> {
    match s {
        "full" => Some(HelpLevel::Full),
        "hints" => Some(HelpLevel::Hints),
        "off" => Some(HelpLevel::Off),
        _ => None,
    }
}

fn level_key(l: HelpLevel) -> &'static str {
    match l {
        HelpLevel::Full => "full",
        HelpLevel::Hints => "hints",
        HelpLevel::Off => "off",
    }
}

fn help(w: &World) -> Option<&Help> {
    w.career.as_ref().map(|c| &c.help)
}

/// Whether the staff's warnings (H5) and first-time notes are on: Full or
/// Hints only. Lane 6's pre-game lineup note reads it.
pub fn warnings_on(w: &World) -> bool {
    help(w).is_none_or(|h| h.level != HelpLevel::Off)
}

fn help_json(w: &World) -> Value {
    let h = help(w);
    json!({
        "level": h.map_or("full", |h| level_key(h.level)),
        "week": h.and_then(|h| h.week.as_ref()).is_some(),
        "read": h.map_or(Vec::new(), |h| h.read.clone()),
    })
}

/// A new career's choice from the setup screen (`start_career`'s `help`).
pub fn on_start(w: &mut World, args: &Value) {
    if let Some(lv) = args.get("help").and_then(Value::as_str).and_then(parse_level)
        && let Some(c) = w.career.as_mut()
    {
        c.help.level = lv;
    }
    // The week and its first note start on day one, not at the first sim day.
    after_days(w);
}

/// Settings: Full turned on in your first week at a job starts the guided
/// week from today; Hints only or Off ends it.
fn set_level(w: &mut World, lv: HelpLevel) {
    let today = w.today;
    let me = views::mine(w);
    let tasks = me.map(|t| plan(w, t, today));
    let Some(c) = w.career.as_mut() else { return };
    let h = &mut c.help;
    h.level = lv;
    match lv {
        HelpLevel::Full => {
            let fresh = h.since.is_some_and(|s| s.days_to(today) <= 6);
            if let (Some(t), Some(tasks), true, None) = (me, tasks, fresh, h.week.as_ref()) {
                h.week = Some(Week { team: team_code(t), start: today, tasks });
            }
        }
        _ => h.week = None,
    }
}

// ---------- the guided first week (H12) and its card (H13) ----------

/// One task of the week: where it opens, the screens that count as having
/// looked (`l8_seen`), the commands that count as having done it, its
/// Handbook page and the inbox keyword.
struct TaskDef {
    key: &'static str,
    title: &'static str,
    go: &'static str,
    button: &'static str,
    look: &'static [&'static str],
    cmds: &'static [&'static str],
    page: &'static str,
    keys: u16,
}

const TASKS: [TaskDef; 13] = [
    TaskDef {
        key: "depth",
        title: "The depth chart",
        go: "depth",
        button: "Open the depth chart",
        look: &["depth"],
        cmds: &["set_depth", "dp_depth_set"],
        page: "ratings",
        keys: mail::PLAYERS,
    },
    TaskDef {
        key: "plan",
        title: "The game plan",
        go: "plans",
        button: "Open plans",
        look: &[],
        cmds: &["plans_set", "dp_rules_set"],
        page: "gameplan",
        keys: mail::STAFF,
    },
    TaskDef {
        key: "training",
        title: "The training week",
        go: "training",
        button: "Open training",
        look: &[],
        cmds: &["practice_week", "practice_template"],
        page: "training",
        keys: mail::PLAYERS,
    },
    TaskDef {
        key: "twoway",
        title: "Two-way players",
        go: "depth",
        button: "Open the depth chart",
        look: &[],
        cmds: &["set_depth", "dp_depth_set"],
        page: "ratings",
        keys: mail::PLAYERS,
    },
    TaskDef {
        key: "limits",
        title: "Practice limits",
        go: "training",
        button: "Open training",
        look: &["training"],
        cmds: &[],
        page: "training",
        keys: mail::PLAYERS,
    },
    TaskDef {
        key: "friday",
        title: "Friday night",
        go: "gameday",
        button: "Open game day",
        look: &["gameday"],
        cmds: &[],
        page: "gameplan",
        keys: mail::PLAYERS,
    },
    TaskDef {
        key: "board",
        title: "The recruiting board",
        go: "recruit_board",
        button: "Open the recruiting board",
        look: &["recruit_board", "recruitboard", "recruiting"],
        cmds: &[],
        page: "recruiting",
        keys: mail::RECRUITING,
    },
    TaskDef {
        key: "contact",
        title: "The contact calendar",
        go: "calendar",
        button: "Open the calendar",
        look: &["calendar"],
        cmds: &[],
        page: "calendar",
        keys: mail::RECRUITING,
    },
    TaskDef {
        key: "portal",
        title: "The portal",
        go: "portal_desk",
        button: "Open the portal",
        look: &["portal_desk"],
        cmds: &[],
        page: "portal",
        keys: mail::PORTAL,
    },
    TaskDef {
        key: "cut",
        title: "The 53-man roster",
        go: "fo_moves",
        button: "Open roster moves",
        look: &["fo_moves", "roster"],
        cmds: &[],
        page: "contracts",
        keys: mail::PLAYERS,
    },
    TaskDef {
        key: "cap",
        title: "The salary cap",
        go: "fo_cap",
        button: "Open the cap",
        look: &["fo_cap", "capboard"],
        cmds: &[],
        page: "contracts",
        keys: mail::MONEY,
    },
    TaskDef {
        key: "squad",
        title: "The practice squad",
        go: "fo_moves",
        button: "Open roster moves",
        look: &[],
        cmds: &[],
        page: "contracts",
        keys: mail::PLAYERS,
    },
    TaskDef {
        key: "schedule",
        title: "The schedule",
        go: "schedule",
        button: "Open the schedule",
        look: &["schedule"],
        cmds: &[],
        page: "calendar",
        keys: mail::LEAGUE,
    },
];

fn def(key: &str) -> Option<&'static TaskDef> {
    TASKS.iter().find(|d| d.key == key)
}

/// The week's six tasks, one a day: the depth chart, the game plan and the
/// training week, then the level's own (H12).
fn plan(_w: &World, t: TeamRef, start: Date) -> Vec<Task> {
    let own: [&str; 3] = match t {
        TeamRef::HighSchool(_) => ["twoway", "limits", "friday"],
        TeamRef::College(_) => ["board", "contact", "portal"],
        TeamRef::Nfl(_) => ["cut", "cap", "squad"],
        TeamRef::Other(_) => ["cut", "schedule", "squad"],
    };
    ["depth", "plan", "training"]
        .into_iter()
        .chain(own)
        .enumerate()
        .map(|(k, key)| Task { key: key.to_string(), day: start.add(k as i32), done: false })
        .collect()
}

fn week_mut(w: &mut World) -> Option<&mut Week> {
    w.career.as_mut().and_then(|c| c.help.week.as_mut())
}

/// Ticks every open task whose test passes; the number ticked.
fn tick(w: &mut World, pass: impl Fn(&TaskDef) -> bool) -> usize {
    let Some(wk) = week_mut(w) else { return 0 };
    let mut n = 0;
    for t in wk.tasks.iter_mut().filter(|t| !t.done) {
        if def(&t.key).is_some_and(&pass) {
            t.done = true;
            n += 1;
        }
    }
    n
}

/// After any command that worked: the week's "do it" tasks tick when
/// their change is made (the game plan saved, the week's practice set).
pub fn after_call(g: &mut Game, cmd: &str) {
    if let Some(w) = g.world.as_mut()
        && w.career.as_ref().is_some_and(|c| c.help.week.is_some())
    {
        tick(w, |d| d.cmds.contains(&cmd));
    }
}

/// Your No. 2: the coordinator with the bigger name (at a high school your
/// senior coordinator); your head coach when you are an assistant.
fn number_two(w: &World, t: TeamRef) -> (String, String) {
    let assistant = w.career.as_ref().is_some_and(|c| c.role.is_some());
    let trio = |hc: Option<usize>, oc: Option<usize>, dc: Option<usize>| -> Option<(String, String)> {
        if assistant {
            return hc.map(|c| (w.coaches[c].name.clone(), "head coach".to_string()));
        }
        let pick = [(oc, "offensive coordinator"), (dc, "defensive coordinator")]
            .into_iter()
            .filter_map(|(c, r)| c.map(|c| (c, r)))
            .filter(|(c, _)| w.career.as_ref().is_none_or(|k| k.coach != *c))
            .max_by(|a, b| w.coaches[a.0].reputation.total_cmp(&w.coaches[b.0].reputation))?;
        Some((w.coaches[pick.0].name.clone(), pick.1.to_string()))
    };
    let found = match t {
        TeamRef::College(i) => {
            let p = &w.programs[i];
            trio(p.head_coach, p.oc, p.dc)
        }
        TeamRef::Nfl(i) => {
            let tm = &w.nfl[i];
            trio(tm.head_coach, tm.oc, tm.dc)
        }
        TeamRef::Other(i) => w.others.team(i).and_then(|tm| trio(tm.head_coach, tm.oc, tm.dc)),
        TeamRef::HighSchool(_) => w.career.as_ref().and_then(|c| {
            c.hs_staff.iter().filter(|a| a.coordinator.is_some()).min_by_key(|a| a.since).map(|a| {
                let side = if a.coordinator == Some(true) { "offensive" } else { "defensive" };
                (a.name.clone(), format!("{side} coordinator"))
            })
        }),
    };
    found.unwrap_or_else(|| {
        (staffing::staff_name(w, t, Job::ChiefOfStaff), staffing::Job::ChiefOfStaff.title(t).to_lowercase())
    })
}

fn you_line(w: &World) -> (String, String) {
    let Some(c) = &w.career else { return ("Coach".into(), "Head coach".into()) };
    let last = c.you.name.split_whitespace().last().unwrap_or("").to_string();
    let role = c.role.map_or("head coach", |r| r.name());
    (format!("Coach {last}"), format!("{}, {role}", c.you.name))
}

fn money(d: f64) -> String {
    let (sign, d) = if d < 0.0 { ("-", -d) } else { ("", d) };
    if d >= 1.0e6 { format!("{sign}${:.1}M", d / 1.0e6) } else { format!("{sign}${:.0}K", d / 1000.0) }
}

fn noun(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Players who started last season, and the roster: a pro started half
/// his games in his latest season of four games or more (this one or the
/// one before), anyone else held a starter's role.
fn starters_back(w: &World, t: TeamRef) -> (usize, usize) {
    let r = views::roster_people(w, t);
    let first = season_of(w.today) - 1;
    let started = |k: usize| {
        let p = &w.people[k];
        match (&p.pro, p.stage) {
            (Some(pro), Stage::Pro) => pro
                .seasons
                .iter()
                .rev()
                .take_while(|s| s.year >= first)
                .find(|s| s.games >= 4)
                .is_some_and(|s| s.starts * 2 >= s.games),
            _ => p.last_role == Some(PlayingTime::Starter),
        }
    };
    (r.iter().filter(|&&k| started(k)).count(), r.len())
}

/// What a task's mail and card say: its one-line ask and why now.
fn task_text(w: &World, t: TeamRef, key: &str) -> (String, String) {
    let today = w.today;
    let ys = YearDates::new(season_of(today));
    let next_ys = YearDates::new(season_of(today) + 1);
    let (back, size) = starters_back(w, t);
    match key {
        "depth" => (
            format!(
                "Look over the chart the last staff left: {} from last season are on a roster of {size}.",
                noun(back, "starter", "starters")
            ),
            "Every lineup call this week starts from it.".into(),
        ),
        "plan" => (
            "Pick the offense and defense you want; the coordinators build the playbook from it.".into(),
            "Installs take time, and the staff wants to start teaching it.".into(),
        ),
        "training" => {
            let rule = match t {
                TeamRef::College(_) => "the NCAA allows 20 countable hours a week in season and 8 out of season",
                TeamRef::Nfl(_) | TeamRef::Other(_) => "the CBA sets what each phase of the year allows",
                TeamRef::HighSchool(_) => "the state sets how much full contact you can have",
            };
            (
                format!("Set this week's practice: {rule}."),
                "The strength staff posts next week's plan from yours.".into(),
            )
        }
        "twoway" => {
            let n = views::roster_people(w, t).len();
            (
                format!(
                    "With {n} players on the varsity, your best athletes will play both ways. Decide who, and who rests."
                ),
                "A tired two-way starter plays worse late in games.".into(),
            )
        }
        "limits" => {
            let state = match t {
                TeamRef::HighSchool(h) => w.states.get(w.high_schools[h].state).map(|s| s.code.clone()),
                _ => None,
            };
            let rule = match state.as_deref() {
                Some("CA") if w.year >= 2015 => {
                    "California allows two full-contact practices a week in season, 90 minutes each, and none out of season"
                }
                Some("TX") if w.year >= 2017 => "Texas allows 90 minutes of full contact a week in season",
                _ => "your state follows the national guidance: about 90 minutes of full contact a week in season",
            };
            (format!("Know the practice limits: {rule}."), "The athletic director checks the logs.".into())
        }
        "friday" => {
            let next = views::next_game(w, t);
            let line = next.as_ref().map_or("Your next game sets the week's rhythm.".to_string(), |g| {
                format!(
                    "Your next game: {} {}, {}.",
                    if g["home"].as_bool() == Some(true) { "home against" } else { "at" },
                    g["opp"]["name"].as_str().unwrap_or("the opponent"),
                    g["date"].as_str().unwrap_or("")
                )
            });
            (format!("{line} Set how you'll call it on game day."), "Friday night is the week's one test.".into())
        }
        "board" => {
            let (commits, signed) = class_count(w, t);
            let ask = match (commits, signed) {
                (0, 0) => "Nobody has committed yet. Pick the first names on your board to chase.".to_string(),
                (0, s) => format!(
                    "{} with the last staff; nobody has committed for next year yet. Pick the first names on your board to chase.",
                    noun(s, "player has signed", "players have signed")
                ),
                (c, 0) => format!(
                    "{} to the last staff. Decide whom to keep before signing.",
                    noun(c, "player has committed", "players have committed")
                ),
                (c, s) => format!(
                    "{s} signed and {c} committed to the last staff. Decide whom to keep, and whom to chase next."
                ),
            };
            let why = if commits > 0 {
                "Commits listen to a new staff in its first days."
            } else {
                "The other staffs are already on next year's class."
            };
            (ask, why.into())
        }
        "contact" => {
            let p = period_words(recruiting_period(&ys_for(today), today));
            (
                format!("Today is a {p}. The contact calendar decides when you can see recruits and talk to them."),
                "Your recruiting staff plans its trips around it.".into(),
            )
        }
        "portal" => {
            let (a, b) = if today <= ys.portal_close {
                (ys.portal_open, ys.portal_close)
            } else {
                (next_ys.portal_open, next_ys.portal_close)
            };
            let open = today >= a && today <= b;
            let line = if open {
                format!("The window is open until {}.", date_text(b))
            } else {
                format!("The next window opens {} and runs to {}.", date_text(a), date_text(b))
            };
            (
                format!("{line} See who might leave and who you could add."),
                "A new staff's roster turns over fastest in its first year.".into(),
            )
        }
        "cut" => {
            let n = views::roster_people(w, t).len();
            let cut = if today <= ys.cutdown.date { ys.cutdown.date } else { next_ys.cutdown.date };
            (
                format!("You have {n} under contract. The roster must be down to 53 by {}.", date_text(cut)),
                "Your personnel staff wants your keep list early.".into(),
            )
        }
        "cap" => {
            let room = match t {
                TeamRef::Nfl(i) => Some(afm_engine::world::pro::cap_room(w, i, w.year)),
                _ => None,
            };
            let line = room.map_or("Know your cap room before you make a move.".to_string(), |r| {
                format!("You have {} of cap room this year.", money(r))
            });
            (
                format!("{line} Every signing, cut and trade is checked against it."),
                "Free agency and the draft both spend it.".into(),
            )
        }
        "squad" => (
            format!(
                "Up to {} players can sit on the practice squad: they practise with you and any club can sign them to its roster.",
                w.rules.nfl_practice_squad
            ),
            "The best young players there get poached.".into(),
        ),
        "schedule" => (
            "Look over the season: your league's schedule is short and every game counts.".into(),
            "Travel and rest weeks shape the year.".into(),
        ),
        _ => (String::new(), String::new()),
    }
}

fn period_words(p: Period) -> &'static str {
    match p {
        Period::Contact => "contact period: you can meet recruits on and off campus",
        Period::Evaluation => "evaluation period: you can watch recruits, but not talk to them off campus",
        Period::Quiet => "quiet period: you can talk to recruits only on your campus",
        Period::Dead => "dead period: no in-person contact at all",
    }
}

/// The season a date belongs to: January still counts as last fall's.
fn season_of(d: Date) -> i32 {
    if d.month() == 1 { d.year() - 1 } else { d.year() }
}

fn ys_for(d: Date) -> YearDates {
    YearDates::new(season_of(d))
}

/// Your class's commits and signed players.
fn class_count(w: &World, t: TeamRef) -> (usize, usize) {
    let TeamRef::College(p) = t else { return (0, 0) };
    let mut commits = 0;
    let mut signed = 0;
    for x in &w.people {
        if x.stage != Stage::HighSchool {
            continue;
        }
        if x.cal.signed.as_ref().is_some_and(|s| s.program == p) {
            signed += 1;
        } else if x.cal.commit.is_some_and(|c| c.0 == p) {
            commits += 1;
        }
    }
    (commits, signed)
}

/// The Getting started card (H13): the week's tasks with what each asks,
/// or nothing once all are done (or with Help on Hints only or Off).
fn started(w: &World) -> Value {
    let (Some(me), Some(h)) = (views::mine(w), help(w)) else { return json!({ "show": false }) };
    let Some(wk) = h.week.as_ref().filter(|wk| wk.team == team_code(me) && h.level == HelpLevel::Full) else {
        return json!({ "show": false });
    };
    let done = wk.tasks.iter().filter(|t| t.done).count();
    if done == wk.tasks.len() {
        return json!({ "show": false });
    }
    let (from, role) = number_two(w, me);
    let today = w.today;
    let tasks: Vec<Value> = wk
        .tasks
        .iter()
        .filter_map(|t| {
            let d = def(&t.key)?;
            let (ask, _) = task_text(w, me, &t.key);
            Some(json!({
                "key": t.key, "day": date_text(t.day), "title": d.title, "text": ask,
                "go": d.go, "button": d.button, "page": d.page, "done": t.done,
                "today": t.day == today, "act": d.look.is_empty(),
            }))
        })
        .collect();
    let first = h.jobs <= 1;
    json!({
        "show": true,
        "title": format!("Your first week at {}", views::team_name(w, me)),
        "from": format!("From {from}, your {role}: one task a day in your inbox."),
        "done": done, "of": wk.tasks.len(),
        "day": wk.start.days_to(today) + 1,
        "help": if first { "Full" } else { "Full, kept on at this job" },
        "tasks": tasks,
    })
}

// ---------- after the clock moves ----------

/// Runs after the clock moves (from `lane_core::after_days`): a new job
/// starts its week, the week's mail goes out one a day, and a big moment
/// reached for the first time gets its note.
pub fn after_days(w: &mut World) {
    let Some(me) = views::mine(w) else { return };
    let code = team_code(me);
    let today = w.today;
    let Some(c) = w.career.as_ref() else { return };
    let career_start = c.started;
    if c.help.team != Some(code) {
        let first = c.help.team.is_none();
        let since = if first { career_start.min(today) } else { today };
        let tasks = plan(w, me, since);
        let Some(c) = w.career.as_mut() else { return };
        let h = &mut c.help;
        h.team = Some(code);
        h.jobs += 1;
        h.since = Some(since);
        h.week = (h.level == HelpLevel::Full && since.days_to(today) <= 6).then_some(Week {
            team: code,
            start: since,
            tasks,
        });
    }
    let mails = {
        let w: &World = w;
        let mut out = week_mail(w, me);
        out.extend(first_notes(w, me));
        out
    };
    for (m, first) in mails {
        if let Some(k) = first
            && let Some(c) = w.career.as_mut()
            && !c.help.firsts.contains(&k)
        {
            c.help.firsts.push(k);
        }
        mail::send(w, m);
    }
}

/// The No. 2's note for each task whose day has come, each dated on its own
/// day; none for a task already done.
fn week_mail(w: &World, me: TeamRef) -> Vec<(afm_engine::world::desk::Mail, Option<String>)> {
    let Some(h) = help(w) else { return Vec::new() };
    let Some(wk) = h.week.as_ref().filter(|wk| wk.team == team_code(me) && h.level == HelpLevel::Full) else {
        return Vec::new();
    };
    let today = w.today;
    let (dear, to) = you_line(w);
    let (from, role) = number_two(w, me);
    let team = views::team_name(w, me);
    wk.tasks
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.done && t.day <= today)
        .filter_map(|(k, t)| {
            let d = def(&t.key)?;
            let (ask, why) = task_text(w, me, &t.key);
            let mut m = mail::letter(
                &format!("l8week|{}|{}", t.key, wk.team),
                t.day,
                7 * 60 + 5,
                Head::Memo,
                team.clone(),
                "Football office".to_string(),
                from.clone(),
                cap(&role),
                to.clone(),
                d.title.to_string(),
            );
            m.org_team = Some(team_code(me));
            let opener = if k == 0 {
                format!("{dear}, welcome. I'll send you one thing a day this week, the things I'd want to know on day one in your chair.")
            } else {
                format!("{dear},")
            };
            m.body = vec![
                opener,
                ask,
                format!("Why now: {}{}.", why.get(..1).unwrap_or("").to_lowercase(), why.get(1..).unwrap_or("").trim_end_matches('.')),
                format!(
                    "[[s:{}|{}]] · [[s:handbook?page={}|Handbook: {}]]",
                    d.go,
                    d.button,
                    d.page,
                    page_title(d.page)
                ),
            ];
            m.sign = "Day ".to_string() + &(k + 1).to_string() + " of 6";
            m.keys = d.keys | mail::JOB;
            Some((m, None))
        })
        .collect()
}

/// A tendency's end as something he does: "bounces runs", "is patient".
fn habit_verb(end: &str) -> String {
    match end {
        "downhill" => "runs downhill".into(),
        "patient" => "is patient".into(),
        "gunslinger" => "plays the gunslinger".into(),
        "big hit" => "goes for the big hit".into(),
        "sure tackle" => "makes the sure tackle".into(),
        e => e.into(),
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// The Handbook's own pages (H8). A later lane adds a page in
/// `handbook-pages.js` with `hbAdd`; it gets a live example here by adding
/// an arm to `live` (a page with no arm just shows no example).
pub const PAGES: [&str; 14] = [
    "ratings",
    "body",
    "scouting",
    "development",
    "tendencies",
    "links",
    "recruiting",
    "portal",
    "draft",
    "contracts",
    "gameplan",
    "training",
    "people",
    "calendar",
];

fn page_title(id: &str) -> &'static str {
    match id {
        "ratings" => "Ratings",
        "body" => "Body and speed",
        "scouting" => "Scouting ranges",
        "development" => "Development and hidden potential",
        "tendencies" => "Tendencies",
        "links" => "Dynamic Links",
        "recruiting" => "Recruiting",
        "portal" => "Portal and NIL",
        "draft" => "The draft",
        "contracts" => "Contracts and the cap",
        "gameplan" => "The game plan",
        "training" => "Training",
        "people" => "People and morale",
        "calendar" => "The calendar",
        _ => "The Handbook",
    }
}

// ---------- the big moments, explained once (H14) ----------

/// A moment on your level's calendar: its key, date, the page it belongs
/// to, who on your staff explains it, and the note.
struct Moment {
    key: &'static str,
    date: Date,
    page: &'static str,
    title: String,
    text: String,
    who: Job,
}

/// Your level's moments in the seasons around today.
fn moments(w: &World, me: TeamRef) -> Vec<Moment> {
    let today = w.today;
    let mut out = Vec::new();
    for s in [season_of(today) - 1, season_of(today)] {
        let y = YearDates::new(s);
        match me {
            TeamRef::College(_) => {
                if let Some(e) = y.early_signing {
                    out.push(Moment {
                        key: "early_signing",
                        date: e,
                        page: "recruiting",
                        title: "The early signing period".into(),
                        text: format!(
                            "Recruits can sign their binding papers for three days from {}. Most of a class signs now; the rest wait for signing day on {}. Until a commit signs, he can still flip.",
                            date_text(e),
                            date_text(y.signing_day)
                        ),
                        who: Job::Personnel,
                    });
                }
                out.push(Moment {
                    key: "signing_day",
                    date: y.signing_day,
                    page: "recruiting",
                    title: "Signing day".into(),
                    text: "The last big signing day of the cycle. The players who waited sign today, and a class that's short at a position fills it from who's left or from the portal.".into(),
                    who: Job::Personnel,
                });
                out.push(Moment {
                    key: "portal",
                    date: y.portal_open,
                    page: "portal",
                    title: "The portal window opens".into(),
                    text: format!(
                        "Players can enter the transfer portal from today to {}. Once a player enters, any school can talk to him, and you can't take his scholarship back until he leaves.",
                        date_text(y.portal_close)
                    ),
                    who: Job::Compliance,
                });
                out.push(Moment {
                    key: "postseason",
                    date: y.selection_day,
                    page: "gameplan",
                    title: "Selection day".into(),
                    text: "The playoff field and the bowls are picked today. Every extra game is also extra practice: the rules give bowl teams more time on the field.".into(),
                    who: Job::Operations,
                });
                out.push(Moment {
                    key: "declare",
                    date: y.declare_deadline,
                    page: "draft",
                    title: "The draft deadline".into(),
                    text: "Underclassmen have until today to declare for the NFL draft. Players who leave free their scholarships; players who stay often do so on the draft advisory board's grade.".into(),
                    who: Job::Personnel,
                });
            }
            TeamRef::Nfl(_) => {
                out.push(Moment {
                    key: "combine",
                    date: y.combine_start,
                    page: "body",
                    title: "The combine".into(),
                    text: "The draft class is measured, timed and examined this week. Every club gets the same numbers; your scouts' job is knowing what they mean for each player.".into(),
                    who: Job::Personnel,
                });
                out.push(Moment {
                    key: "draft",
                    date: y.draft[0].date,
                    page: "draft",
                    title: "The draft".into(),
                    text: "Seven rounds over three days, each club picking from its own board. Trades on the clock are priced on each club's value chart; players nobody picks sign in the rush right after.".into(),
                    who: Job::Personnel,
                });
                out.push(Moment {
                    key: "trade_deadline",
                    date: y.trade_deadline.date,
                    page: "contracts",
                    title: "The trade deadline".into(),
                    text: "No more trades after 4 pm today until the new league year. A trade puts the player's remaining bonus on the seller's cap now.".into(),
                    who: Job::CapManager,
                });
                out.push(Moment {
                    key: "playoffs",
                    date: y.wild_card,
                    page: "gameplan",
                    title: "The playoffs".into(),
                    text: "Fourteen clubs, one loss and you're out. The top seed in each conference rests on wild card weekend.".into(),
                    who: Job::Operations,
                });
            }
            TeamRef::HighSchool(h) => {
                if let Some(e) = y.early_signing {
                    out.push(Moment {
                        key: "early_signing",
                        date: e,
                        page: "recruiting",
                        title: "Early signing".into(),
                        text: "Seniors with scholarship offers can sign with their college this week. A signing at the school is a day for the family; your part is the paperwork and the photos.".into(),
                        who: Job::AthleticDirector,
                    });
                }
                if let Some(st) = w.states.get(w.high_schools[h].state) {
                    let (_, playoffs, finals) = afm_engine::world::hs_year::state_season_dates(&st.code, s);
                    out.push(Moment {
                        key: "playoffs",
                        date: playoffs,
                        page: "gameplan",
                        title: "The state playoffs".into(),
                        text: format!(
                            "The playoffs start this week, by classification, and the state final is {}. One loss ends the season, so every lineup choice counts double.",
                            date_text(finals)
                        ),
                        who: Job::AthleticDirector,
                    });
                }
            }
            TeamRef::Other(_) => {}
        }
    }
    out
}

/// The note for each moment reached for the first time since you came to
/// the job (within two weeks of it), unless Help is Off.
fn first_notes(w: &World, me: TeamRef) -> Vec<(afm_engine::world::desk::Mail, Option<String>)> {
    let Some(h) = help(w) else { return Vec::new() };
    if h.level == HelpLevel::Off {
        return Vec::new();
    }
    let today = w.today;
    let since = h.since.unwrap_or(today);
    let (_, to) = you_line(w);
    let team = views::team_name(w, me);
    let mut out = Vec::new();
    for m in moments(w, me) {
        if m.date > today || m.date < since || m.date.days_to(today) > 14 || h.firsts.iter().any(|k| k == m.key) {
            continue;
        }
        let mut x = mail::letter(
            &format!("l8first|{}", m.key),
            m.date,
            7 * 60 + 30,
            Head::Memo,
            team.clone(),
            "Football office".to_string(),
            staffing::staff_name(w, me, m.who),
            m.who.title(me).to_string(),
            to.clone(),
            m.title.clone(),
        );
        x.org_team = Some(team_code(me));
        x.body = vec![
            "Coach, a first for you here, so a short note on how it works.".to_string(),
            m.text,
            format!("[[s:handbook?page={}|Handbook: {}]]", m.page, page_title(m.page)),
        ];
        x.keys = mail::JOB;
        out.push((x, Some(m.key.to_string())));
    }
    out
}

// ---------- the pages (H8-H11) ----------

fn level_name(t: TeamRef) -> &'static str {
    match t {
        TeamRef::HighSchool(_) => "hs",
        TeamRef::College(_) => "college",
        TeamRef::Nfl(_) | TeamRef::Other(_) => "nfl",
    }
}

fn order(t: TeamRef) -> [&'static str; 3] {
    match t {
        TeamRef::HighSchool(_) => ["hs", "college", "nfl"],
        TeamRef::College(_) => ["college", "hs", "nfl"],
        TeamRef::Nfl(_) | TeamRef::Other(_) => ["nfl", "college", "hs"],
    }
}

/// One Handbook page's live parts: the example from your save, the dates
/// coming up, the pages due this month and the ones you've read.
fn page(g: &mut Game, args: &Value) -> Reply {
    let id = args.get("page").and_then(Value::as_str).unwrap_or("body").to_string();
    let panel = args.get("panel").and_then(Value::as_bool).unwrap_or(false);
    let probe = probe(g, &id);
    let w = g.world.as_mut().ok_or(NO_WORLD)?;
    if !panel
        && let Some(c) = w.career.as_mut()
        && !c.help.read.contains(&id)
    {
        c.help.read.push(id.clone());
    }
    let w: &World = w;
    let me = views::mine(w);
    let lv = me.map_or("college", level_name);
    let marks = me.map(|t| year_marks(w, t)).unwrap_or_default();
    let today = w.today;
    // Each date once: this year's combine, not next year's as well.
    let mut seen: Vec<&str> = Vec::new();
    let soon: Vec<Value> = marks
        .iter()
        .filter(|m| m.1.contains(&id.as_str()))
        .filter(|m| {
            let fresh = !seen.contains(&m.2.as_str());
            seen.push(m.2.as_str());
            fresh
        })
        .take(3)
        .map(|m| json!({ "d": short_date(m.0, today), "t": m.2, "s": m.3 }))
        .collect();
    let mut due: Vec<&str> =
        marks.iter().filter(|m| today.days_to(m.0) <= 31).flat_map(|m| m.1.iter().copied()).collect();
    due.sort();
    due.dedup();
    let head = me.map(|t| json!({ "team": views::team_brief(w, t), "label": format!("{} · {}", views::team_name(w, t), date_text(today)) }));
    Ok(json!({
        "page": id,
        "level": lv,
        "order": me.map_or(["college", "hs", "nfl"], order),
        "head": head,
        "live": me.and_then(|t| live(w, t, &id, &probe)),
        "soon": soon,
        "due": due,
        "read": help(w).map_or(Vec::new(), |h| h.read.clone()),
        "help": help(w).map_or("full", |h| level_key(h.level)),
    }))
}

fn short_date(d: Date, today: Date) -> String {
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let m = M[(d.month() - 1) as usize];
    if d.year() == today.year() { format!("{} {m}", d.day()) } else { format!("{m} {}", d.year()) }
}

/// The year's dates that touch the pages: (date, pages, what, why it
/// matters), the next 400 days, soonest first.
fn year_marks(w: &World, t: TeamRef) -> Vec<(Date, Vec<&'static str>, String, String)> {
    let today = w.today;
    let mut v: Vec<(Date, Vec<&'static str>, String, String)> = Vec::new();
    let mut add = |d: Date, p: &[&'static str], a: &str, b: &str| v.push((d, p.to_vec(), a.to_string(), b.to_string()));
    for s in [season_of(today) - 1, season_of(today), season_of(today) + 1] {
        let y = YearDates::new(s);
        match t {
            TeamRef::College(_) => {
                if let Some(e) = y.early_signing {
                    add(e, &["recruiting", "calendar"], "Early signing", "Three days to sign most of your class");
                }
                add(y.signing_day, &["recruiting", "calendar"], "Signing day", "The rest of the class signs");
                add(
                    y.portal_open,
                    &["portal", "people"],
                    "Portal window opens",
                    "Your players can enter; you can recruit transfers",
                );
                add(y.portal_close, &["portal"], "Portal window closes", "Who's in has entered");
                add(y.declare_deadline, &["draft"], "Draft deadline", "Underclassmen declare or come back");
                add(y.combine_start, &["body", "draft"], "The combine", "Your draft prospects are timed and measured");
                add(
                    y.spring_semester,
                    &["training", "body", "development"],
                    "Winter conditioning",
                    "Where most of the weight goes on",
                );
                add(y.spring_game, &["training", "gameplan"], "Spring game", "The end of spring practice");
                add(y.fall_camp, &["training", "development"], "Fall camp", "Growth lands; the depth chart forms");
                add(y.selection_day, &["gameplan", "calendar"], "Selection day", "The playoff and bowls are set");
                add(y.money_year, &["portal", "contracts"], "The new budget year", "NIL and the program's money reset");
            }
            TeamRef::Nfl(_) | TeamRef::Other(_) => {
                add(y.tag_window_open, &["contracts"], "Tag window opens", "Franchise and transition tags");
                add(
                    y.combine_start,
                    &["body", "draft", "scouting"],
                    "The combine",
                    "Laser forties for the draft class",
                );
                add(y.draft[0].date, &["draft", "calendar"], "The draft", "Seven rounds, three days");
                add(
                    y.rookie_minicamp,
                    &["training", "development"],
                    "Rookie minicamp",
                    "Your draft class's first practices",
                );
                add(y.veterans_report, &["training", "development"], "Camp opens", "Growth lands; the 53 takes shape");
                add(y.cutdown.date, &["contracts", "ratings"], "Cut to 53", "The roster limit for the season");
                add(
                    y.trade_deadline.date,
                    &["contracts", "calendar"],
                    "Trade deadline",
                    "The last day to trade until March",
                );
                add(y.wild_card, &["gameplan", "calendar"], "Wild card weekend", "The playoffs begin");
                add(y.black_monday, &["people"], "Black Monday", "Coaches and GMs lose their jobs");
            }
            TeamRef::HighSchool(h) => {
                if let Some(st) = w.states.get(w.high_schools[h].state) {
                    let (start, playoffs, finals) = afm_engine::world::hs_year::state_season_dates(&st.code, s);
                    add(
                        start,
                        &["training", "calendar", "development"],
                        "First practice",
                        "Growth lands; your own hand times for the season",
                    );
                    add(playoffs, &["gameplan", "calendar"], "State playoffs", "One loss ends the season");
                    add(finals, &["gameplan"], "State final", "The last game of the year");
                }
                if let Some(e) = y.early_signing {
                    add(e, &["recruiting"], "Early signing", "Your seniors with offers sign");
                }
                add(y.signing_day, &["recruiting"], "Signing day", "The rest of your seniors sign");
                add(
                    Date::ymd(s + 1, 1, 6),
                    &["training", "body"],
                    "Winter weight room",
                    "Most of your juniors add 5 to 10 lb",
                );
                add(
                    Date::ymd(s + 1, 6, 10),
                    &["body", "recruiting", "scouting"],
                    "College camps",
                    "Big programs time your players with lasers",
                );
            }
        }
    }
    v.retain(|m| m.0 > today && today.days_to(m.0) <= 400);
    v.sort_by_key(|m| m.0);
    v.dedup_by(|a, b| a.0 == b.0 && a.2 == b.2);
    v
}

/// Facts a page's example needs from other lanes' screens, read through
/// their own commands (so the page shows exactly what those screens do).
fn probe(g: &mut Game, id: &str) -> Value {
    let me = g.world.as_ref().and_then(views::mine);
    let cmd = match (id, me) {
        ("gameplan", Some(_)) => "dp_identity",
        _ => return Value::Null,
    };
    g.call(cmd, &json!({})).unwrap_or(Value::Null)
}

// ---------- the live examples (H10) ----------

/// Your staff's read of a rating as (low, high), None when not seen.
fn read_of(w: &World, k: usize, t: TeamRef, a: Attr) -> Option<(f64, f64)> {
    let p = &w.people[k];
    let own = views::is_mine(w, t);
    let eye = if own { views::eye_for(w, t, p.player.position) } else { 10.0 };
    reads::attributes(&p.player, own, eye, views::reader_key(w)).into_iter().find(|x| x.0 == a).and_then(|x| {
        match x.1 {
            Read::Range(lo, hi) => Some((lo as f64, hi as f64)),
            Read::NotSeen => None,
        }
    })
}

fn mid(r: (f64, f64)) -> f64 {
    (r.0 + r.1) / 2.0
}

/// His forty on your staff's read of his speed and acceleration, at his
/// listed weight or another one.
fn forty_at(w: &World, k: usize, t: TeamRef, weight: Option<f64>) -> Option<f64> {
    let p = &w.people[k];
    let sp = read_of(w, k, t, Attr::Speed)?;
    let ac = read_of(w, k, t, Attr::Acceleration)?;
    let mut d = Drills::of(&p.player);
    d.speed = mid(sp);
    d.accel = mid(ac);
    if let Some(wt) = weight {
        d.weight = wt;
    }
    Some(d.expected().forty)
}

/// "Z and CB · JR"-style position and class.
fn pos_class(w: &World, k: usize) -> String {
    let p = &w.people[k];
    let c = views::class_text(w, p);
    if c.is_empty() { p.player.position.name().to_string() } else { format!("{} · {c}", p.player.position.name()) }
}

fn link(w: &World, k: usize) -> String {
    links::player(w.people[k].player.id, &w.people[k].name)
}

/// The roster's players with your staff's overall read, best first.
fn by_read(w: &World, t: TeamRef) -> Vec<(usize, (u8, u8))> {
    let own = views::is_mine(w, t);
    let reader = views::reader_key(w);
    let mut v: Vec<(usize, (u8, u8))> = views::roster_people(w, t)
        .into_iter()
        .filter_map(|k| {
            let p = &w.people[k];
            let eye = if own { views::eye_for(w, t, p.player.position) } else { 10.0 };
            match reads::overall(&p.player, own, eye, reader) {
                Read::Range(lo, hi) => Some((k, (lo, hi))),
                Read::NotSeen => None,
            }
        })
        .collect();
    v.sort_by_key(|b| std::cmp::Reverse(b.1.0 as u16 + b.1.1 as u16));
    v
}

fn live(w: &World, t: TeamRef, id: &str, probe: &Value) -> Option<Value> {
    match id {
        "ratings" => live_ratings(w, t),
        "body" => live_body(w, t),
        "scouting" => live_scouting(w, t),
        "development" => live_development(w, t),
        "tendencies" => live_tendencies(w, t),
        "links" => live_links(w, t),
        "recruiting" => live_recruiting(w, t),
        "portal" => live_portal(w, t),
        "draft" => live_draft(w, t),
        "contracts" => live_contracts(w, t),
        "gameplan" => live_gameplan(w, t, probe),
        "training" => live_training(w, t),
        "people" => live_people(w, t),
        "calendar" => live_calendar(w, t),
        _ => None,
    }
}

/// Ratings: your best player's read at his spot and at two others, from
/// the same reads (your staff's middles) weighed for each position.
fn live_ratings(w: &World, t: TeamRef) -> Option<Value> {
    let &(k, (lo, hi)) = by_read(w, t).first()?;
    let p = &w.people[k];
    let pos = p.player.position;
    let own = views::is_mine(w, t);
    let eye = views::eye_for(w, t, pos);
    let rs = reads::attributes(&p.player, own, eye, views::reader_key(w));
    let mut at = p.player.attributes;
    for (a, r) in &rs {
        if let Read::Range(l, h) = r {
            at.set(*a, (*l as f64 + *h as f64) / 2.0);
        }
    }
    let mut others: Vec<(Position, f64)> = Position::ALL
        .iter()
        .copied()
        .filter(|&q| q != pos && !q.is_specialist())
        .map(|q| (q, afm_engine::positions::overall_for(q, &at)))
        .collect();
    others.sort_by(|a, b| b.1.total_cmp(&a.1));
    let qb = (pos != Position::QB).then(|| afm_engine::positions::overall_for(Position::QB, &at));
    // The three ratings his position weighs most, with your reads.
    let mut keys: Vec<(Attr, f64)> = Attr::ALL.iter().map(|&a| (a, afm_engine::positions::weight(pos, a))).collect();
    keys.sort_by(|a, b| b.1.total_cmp(&a.1));
    let key_line: Vec<String> = keys
        .iter()
        .take(3)
        .filter_map(|(a, _)| {
            rs.iter().find(|x| x.0 == *a).and_then(|x| match x.1 {
                Read::Range(l, h) => Some(format!(
                    "{} {}",
                    a.label().to_lowercase(),
                    if l == h { l.to_string() } else { format!("{l}-{h}") }
                )),
                Read::NotSeen => None,
            })
        })
        .collect();
    let (one, _) = pos_words(pos);
    let mut line = format!("{} reads {lo}-{hi} at {one}", link(w, k));
    if let Some((q, v)) = others.first() {
        line += &format!(", about {:.0} at {}", v, pos_words(*q).0);
    }
    if let Some(v) = qb.filter(|_| others.first().is_none_or(|o| o.0 != Position::QB)) {
        line += &format!(" and {v:.0} at quarterback");
    }
    line += ".";
    let lean = format!("At {one} your staff weighs {} most.", key_line.join(", "));
    Some(json!({
        "name": link(w, k), "pos": pos_class(w, k), "big": format!("{lo}-{hi}"), "when": "your staff's read today",
        "lines": [line, lean],
    }))
}

/// Body and speed: your fastest player's forty on your staff's read, what
/// it would be 15 lb heavier, and where it sits at each level.
fn live_body(w: &World, t: TeamRef) -> Option<Value> {
    let roster = views::roster_people(w, t);
    let mut fast: Vec<(usize, f64)> = roster
        .iter()
        .filter(|&&k| !w.people[k].player.position.is_specialist())
        .filter_map(|&k| forty_at(w, k, t, None).map(|f| (k, f)))
        .collect();
    fast.sort_by(|a, b| a.1.total_cmp(&b.1));
    let &(k, f) = fast.first()?;
    let p = &w.people[k];
    let b = body_now(&p.player);
    let wt = b.weight.round();
    let heavier = forty_at(w, k, t, Some(wt + 15.0))?;
    let sp = read_of(w, k, t, Attr::Speed)?;
    let ac = read_of(w, k, t, Attr::Acceleration)?;
    let rng = |r: (f64, f64)| if r.0 == r.1 { format!("{:.0}", r.0) } else { format!("{:.0}-{:.0}", r.0, r.1) };
    let line = format!("speed {} · acceleration {} · {}, {wt:.0} lb", rng(sp), rng(ac), views::ft_in(b.height));
    let pos = p.player.position;
    let (_, many) = pos_words(pos);
    let state = match t {
        TeamRef::HighSchool(h) => Some(w.high_schools[h].state),
        _ => None,
    };
    let lv = level_name(t);
    let mut strips: Vec<Value> = order(t)
        .iter()
        .filter_map(|&l| {
            // A small state's starters give way to the whole country's.
            let (name, sub, xs) = field(w, pos, l, state).or_else(|| state.and_then(|_| field(w, pos, l, None)))?;
            let n = xs.len() as f64;
            let slower = xs.iter().filter(|&&x| x > f).count() as f64 / n;
            let med = xs[xs.len() / 2];
            let dots: Vec<f64> = (0..52).map(|i| xs[((i as f64 + 0.5) / 52.0 * n) as usize]).collect();
            Some(json!({
                "n": format!("{name} {many}"), "sub": format!("{sub} · median {med:.2}"), "med": med,
                "pct": (slower * 100.0).round(), "dots": dots, "you": l == lv,
            }))
        })
        .collect();
    let note = strips.iter().find(|s| s["you"] == true).map(|s| {
        let pct = s["pct"].as_f64().unwrap_or(50.0);
        let word = if pct >= 90.0 {
            "Elite"
        } else if pct >= 70.0 {
            "Fast"
        } else if pct >= 40.0 {
            "About typical"
        } else {
            "Slow"
        };
        format!("{word} for {} at your level: {pct:.0}% of starters at his spot ran slower.", many)
    });
    let side: Vec<Value> = fast
        .iter()
        .take(5)
        .map(|&(k, f)| json!([link(w, k), w.people[k].player.position.name(), format!("{f:.2}"), "your read"]))
        .collect();
    strips.truncate(3);
    Some(json!({
        "name": link(w, k), "pos": pos_class(w, k), "line": line, "big": format!("{f:.2}"), "when": "on your staff's read and his listed weight",
        "lines": [format!("{} runs about {f:.2} on your staff's read; at {:.0} lb on the same ratings he'd run about {heavier:.2}.", link(w, k), wt + 15.0)],
        "how": { "k": 1, "s": format!("{}: {} make {f:.2}. At {:.0} lb on the same ratings he'd run about {heavier:.2}.", p.name, line, wt + 15.0) },
        "short": p.name.split_whitespace().last().unwrap_or(&p.name),
        "strips": strips, "note": note,
        "side": { "title": "Fastest on your team", "sub": "forty · your read", "rows": side, "note": "On your staff's read of speed and acceleration at each player's listed weight." },
    }))
}

/// The forties of a level's starters at a position, sorted: the field's
/// shape only (no one is named), from their true ratings like a stopwatch.
fn field(w: &World, pos: Position, lv: &str, state: Option<usize>) -> Option<(String, String, Vec<f64>)> {
    let mut xs: Vec<f64> = Vec::new();
    for p in &w.people {
        if p.player.position != pos || p.last_role != Some(PlayingTime::Starter) {
            continue;
        }
        let ok = match lv {
            "hs" => {
                p.stage == Stage::HighSchool
                    && state.is_none_or(|s| w.high_schools.get(p.hs).is_some_and(|h| h.state == s))
            }
            "college" => p.stage == Stage::College && views::level_of(w, p) == Level::FBS,
            _ => p.stage == Stage::Pro,
        };
        if ok {
            xs.push(Drills::of(&p.player).expected().forty);
        }
    }
    if xs.len() < 20 {
        return None;
    }
    xs.sort_by(|a, b| a.total_cmp(b));
    let (name, sub) = match lv {
        "hs" => (
            state
                .and_then(|s| w.states.get(s))
                .map_or("High school".to_string(), |s| views::state_name(&s.code).to_string()),
            "starters".to_string(),
        ),
        "college" => ("FBS".to_string(), "starters".to_string()),
        _ => ("NFL".to_string(), "starters".to_string()),
    };
    Some((name, sub, xs))
}

/// Scouting ranges: one of your players next to the best player your staff
/// reads on the next team you play: a narrow range and a wide one.
fn live_scouting(w: &World, t: TeamRef) -> Option<Value> {
    let &(k, (lo, hi)) = by_read(w, t).first()?;
    let mut lines = vec![format!(
        "Your staff reads {} at {lo}-{hi}: {} points wide, because they see him every day.",
        link(w, k),
        hi - lo
    )];
    if let Some(g) = views::next_game(w, t)
        && let Some(opp) =
            views::TeamRef::from_args(w, &json!({ "kind": g["opp"]["ref"]["kind"], "id": g["opp"]["ref"]["id"] })).ok()
        && let Some(&(j, (a, b))) = by_read(w, opp).first()
    {
        lines.push(format!(
            "{} of {} reads {a}-{b} to your staff: {} points wide, from film only.",
            link(w, j),
            links::team(w, opp),
            b - a
        ));
    }
    Some(
        json!({ "name": link(w, k), "pos": pos_class(w, k), "big": format!("{lo}-{hi}"), "when": "your staff's read today", "lines": lines }),
    )
}

/// Development: your best young player's outlook in your staff's words.
fn live_development(w: &World, t: TeamRef) -> Option<Value> {
    let young = by_read(w, t)
        .into_iter()
        .find(|&(k, _)| w.people[k].age_on(w.today) <= 22)
        .or_else(|| by_read(w, t).first().copied())?;
    let (k, (lo, hi)) = young;
    let p = &w.people[k];
    let eye = views::eye_for(w, t, p.player.position);
    let o = reads::potential_words(&p.player, eye, views::level_of(w, p), views::reader_key(w), w.year);
    let tier = o["tier"].as_str().unwrap_or("hard to call");
    let sure = match o["sure"].as_str().unwrap_or("") {
        "High" => "sure",
        "Medium" => "fairly sure",
        _ => "unsure",
    };
    let mut lines = vec![format!(
        "{}, {}: your staff reads him at {lo}-{hi} today and says he {}. They are {sure}.",
        link(w, k),
        p.age_on(w.today),
        if tier.starts_with("could") || tier.starts_with("may") {
            tier.to_string()
        } else {
            format!("looks like {tier}")
        }
    )];
    if let Some(h) = o["hints"].as_array().and_then(|h| h.first()).and_then(Value::as_str) {
        lines.push(format!("One thing they noticed: {h}."));
    }
    Some(
        json!({ "name": link(w, k), "pos": pos_class(w, k), "big": format!("{lo}-{hi}"), "when": "your staff's words, not a number", "lines": lines }),
    )
}

/// Tendencies: the habit your staff reads furthest from typical on your
/// best player.
fn live_tendencies(w: &World, t: TeamRef) -> Option<Value> {
    let &(k, _) = by_read(w, t).first()?;
    let p = &w.people[k];
    let reader = views::reader_key(w);
    let id = p.player.id;
    // Only the habits his position has (a safety doesn't scramble), read
    // as the player page reads them.
    let pos = p.player.position;
    let mut far: Vec<(f64, afm_engine::tendencies::Tend, (u8, u8))> = afm_engine::tendencies::Tend::ALL
        .iter()
        .filter(|x| x.owned_by(pos))
        .filter_map(|&x| match reads::range(p.player.tend.get(x) * 99.0 / 20.0, 6.0, reader, id, 200 + x.index()) {
            Read::Range(lo, hi) => Some((((lo as f64 + hi as f64) / 2.0 - 49.5).abs(), x, (lo, hi))),
            Read::NotSeen => None,
        })
        .collect();
    far.sort_by(|a, b| b.0.total_cmp(&a.0));
    let (_, tend, (lo, hi)) = *far.first()?;
    let (low_end, high_end) = tend.ends();
    let high = (lo as f64 + hi as f64) / 2.0 > 49.5;
    Some(json!({
        "name": link(w, k), "pos": pos_class(w, k), "big": format!("{lo}-{hi}"), "when": "your staff's read",
        "lines": [
            format!(
                "{}'s strongest lean: he {}. Your staff reads {} at {lo}-{hi}, where 50 is the middle.",
                link(w, k),
                habit_verb(if high { high_end } else { low_end }),
                tend.label().to_lowercase()
            ),
            "It says how often he tries it, not how well it goes.".to_string(),
        ],
    }))
}

/// Dynamic Links: the strongest link your staff sees on your roster.
fn live_links(w: &World, t: TeamRef) -> Option<Value> {
    let own = views::is_mine(w, t);
    let reader = views::reader_key(w);
    let mut best: Option<(usize, Value)> = None;
    for (k, _) in by_read(w, t) {
        let p = &w.people[k];
        let eye = views::eye_for(w, t, p.player.position);
        let l = reads::links(&p.player, own, p.stage == Stage::Pro || p.cal.combine_invite, eye, reader);
        if let Some(first) = l.into_iter().next() {
            best = Some((k, first));
            break;
        }
    }
    let Some((k, l)) = best else {
        return Some(
            json!({ "lines": ["Nobody on your roster shows a link yet. Most players never have one; at your level they are rare."] }),
        );
    };
    let name = l["name"].as_str().unwrap_or("a link");
    let strength = l["strength"].as_str().unwrap_or("").to_lowercase();
    let numbers = l["numbers"].as_str().unwrap_or("").trim_end_matches('.');
    let first = if let Some(film) = numbers.strip_prefix("Projects: ") {
        format!("{} projects to grow into {name} ({strength}): {film}.", link(w, k))
    } else if let Some(film) = numbers.strip_prefix("From film: ") {
        format!("{} shows {name} ({strength}) on film: {film}.", link(w, k))
    } else {
        format!("{} has {name} ({strength}): {numbers}.", link(w, k))
    };
    Some(json!({
        "name": link(w, k), "pos": pos_class(w, k), "big": l["strength"], "when": "your staff's read",
        "lines": [first, format!("What it does: {}", l["edge"].as_str().unwrap_or(""))],
    }))
}

/// Recruiting: your class (college), your players' offers (high school).
fn live_recruiting(w: &World, t: TeamRef) -> Option<Value> {
    match t {
        TeamRef::College(_) => {
            let (c, s) = class_count(w, t);
            let class = match (c, s) {
                (0, 0) => "Nobody has committed to your next class yet.".to_string(),
                (0, s) => format!(
                    "{} and arrive this summer; nobody has committed for next year yet.",
                    noun(s, "player signed with you", "players signed with you")
                ),
                (c, 0) => format!("{} to your class.", noun(c, "player has committed", "players have committed")),
                (c, s) => format!("{s} signed and {c} committed to your class."),
            };
            Some(json!({ "big": (c + s).to_string(), "when": "your class today", "lines": [
                class,
                "Your board sorts by your staff's grade; stars are the sites' opinion.",
            ] }))
        }
        TeamRef::HighSchool(_) => {
            let mut v: Vec<(usize, usize)> = views::roster_people(w, t)
                .into_iter()
                .map(|k| (k, views::offers_of(&w.people[k]).len()))
                .filter(|x| x.1 > 0)
                .collect();
            v.sort_by_key(|b| std::cmp::Reverse(b.1));
            let line = match v.first() {
                Some(&(k, n)) => format!(
                    "{} has {}. College coaches visit your practices, and your calls help your players get looks.",
                    link(w, k),
                    noun(n, "scholarship offer", "scholarship offers")
                ),
                None => "None of your players holds a scholarship offer yet. Your calls and film help college coaches find them.".to_string(),
            };
            Some(json!({ "lines": [line] }))
        }
        _ => None,
    }
}

/// Portal and NIL: your players in the portal.
fn live_portal(w: &World, t: TeamRef) -> Option<Value> {
    let TeamRef::College(p) = t else { return None };
    let n =
        w.people.iter().filter(|x| x.stage == Stage::College && x.college == Some(p) && x.cal.portal.is_some()).count();
    let ys = ys_for(w.today);
    Some(json!({ "big": n.to_string(), "when": "in the portal today", "lines": [format!(
        "{} from your roster {} in the portal. This season's window runs {} to {}.",
        noun(n, "player", "players"),
        if n == 1 { "is" } else { "are" },
        date_text(ys.portal_open),
        date_text(ys.portal_close)
    )] }))
}

/// The draft: your next pick (NFL), your draft prospects (college).
fn live_draft(w: &World, t: TeamRef) -> Option<Value> {
    match t {
        TeamRef::College(p) => {
            let n = w.people.iter().filter(|x| x.stage == Stage::Declared && x.college == Some(p)).count();
            Some(json!({ "lines": [format!(
                "{} from your program declared for this year's draft.",
                noun(n, "player", "players")
            )] }))
        }
        TeamRef::Nfl(_) => {
            let y = YearDates::new(if w.today < ys_for(w.today).draft[0].date {
                season_of(w.today)
            } else {
                season_of(w.today) + 1
            });
            Some(
                json!({ "lines": [format!("The next draft starts {}. Your board is your scouts' order, not the public's.", date_text(y.draft[0].date))] }),
            )
        }
        _ => None,
    }
}

/// Contracts and the cap: your biggest cap hit and your room (NFL).
fn live_contracts(w: &World, t: TeamRef) -> Option<Value> {
    let TeamRef::Nfl(i) = t else { return None };
    let room = afm_engine::world::pro::cap_room(w, i, w.year);
    let mut hits: Vec<(usize, f64)> = views::roster_people(w, t)
        .into_iter()
        .filter_map(|k| w.people[k].pro.as_ref().and_then(|c| c.contract.as_ref()).map(|c| (k, c.cap_hit(w.year))))
        .collect();
    hits.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut lines = vec![format!("You have {} of cap room this year.", money(room))];
    if let Some(&(k, h)) = hits.first() {
        lines
            .insert(0, format!("{} counts {} against this year's cap, the most on your roster.", link(w, k), money(h)));
    }
    Some(json!({ "big": money(room), "when": "cap room this year", "lines": lines }))
}

/// The game plan: the systems your staff runs now (the Plans screen's).
/// The game plan: the identity screen's own fit reads for your system and
/// your front (lane 5 of update 2).
fn live_gameplan(_w: &World, _t: TeamRef, probe: &Value) -> Option<Value> {
    let rows = probe["rows"].as_array()?;
    let row = |key: &str| rows.iter().find(|r| r["key"] == key);
    let fit = |r: &Value| r["kv"][0]["v"].as_str().map(str::to_string);
    let system = row("system")?;
    let own = system["options"]
        .as_array()
        .and_then(|o| o.iter().find(|o| o["own"] == true))
        .and_then(|o| o["label"].as_str());
    let mut lines: Vec<String> = [fit(system), row("front").and_then(fit)].into_iter().flatten().collect();
    if lines.is_empty() {
        return None;
    }
    lines.push("Your staff calls the snaps you don't, from these choices.".to_string());
    Some(json!({ "big": own, "when": "your system today", "lines": lines }))
}

/// Training: the phase of the year you're in.
fn live_training(w: &World, t: TeamRef) -> Option<Value> {
    let (back, size) = starters_back(w, t);
    Some(json!({ "lines": [format!(
        "{size} players practise under your plan this week; {back} of them started last season."
    )] }))
}

/// People and morale: how your roster feels, by your staff's words.
fn live_people(w: &World, t: TeamRef) -> Option<Value> {
    let roster = views::roster_people(w, t);
    if roster.is_empty() {
        return None;
    }
    let mut counts: Vec<(&'static str, usize)> = Vec::new();
    for &k in &roster {
        let m = views::mood_words(&w.people[k]);
        match counts.iter_mut().find(|c| c.0 == m) {
            Some(c) => c.1 += 1,
            None => counts.push((m, 1)),
        }
    }
    counts.sort_by_key(|b| std::cmp::Reverse(b.1));
    let parts: Vec<String> = counts.iter().take(3).map(|(m, n)| format!("{n} {}", m.to_lowercase())).collect();
    Some(json!({ "lines": [format!("Your staff's read of the room: {}.", parts.join(", "))] }))
}

/// The calendar: the next dates that matter for your team.
fn live_calendar(w: &World, t: TeamRef) -> Option<Value> {
    let today = w.today;
    let marks = year_marks(w, t);
    let next: Vec<String> = marks.iter().take(3).map(|m| format!("{} ({})", m.2, date_text(m.0))).collect();
    if next.is_empty() {
        return None;
    }
    let p = match t {
        TeamRef::College(_) | TeamRef::HighSchool(_) => Some(period_words(recruiting_period(&ys_for(today), today))),
        _ => None,
    };
    let mut lines = vec![format!("Next: {}.", next.join(", "))];
    if let Some(p) = p.filter(|_| matches!(t, TeamRef::College(_))) {
        lines.push(format!("Today is a {p}."));
    }
    Some(json!({ "lines": lines }))
}
