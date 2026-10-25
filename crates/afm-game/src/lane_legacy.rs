//! Lane commands: systems to chase (update 2, lane 2). `call` answers the
//! commands this lane owns and returns None for any other.
//!
//! The screens of `afm_engine::world::legacy`, boards 34-36 of the update 2
//! mock-ups:
//! - the Legacy screen (34): your ambitions, your coaching tree, the Hall of
//!   Fame race with the real greats, your scheme's spread, your guys and your
//!   level's five;
//! - the ambition picker on taking a job (35): what fits you, up to three,
//!   private or out loud, the others' goals on top;
//! - your level's five chases (36), each with its chart and the screen it
//!   lives on (the strip those screens show comes from `legacy_strip`).
//!
//! Every answer you give on a chase letter is acted on first, so a screen
//! never shows a choice you already made as open.

use afm_engine::calendar::Level;
use afm_engine::world::World;
use afm_engine::world::front_office::index_of;
use afm_engine::world::legacy::{self as lg, ambitions, guys, hall, scheme, tree, view};
use afm_engine::world::stories::TeamKey;
use serde_json::{Value, json};

use crate::views::{self, TeamRef};
use crate::{Game, Reply};

const COMMANDS: [&str; 11] = [
    "legacy_page",
    "legacy_chases",
    "legacy_ambitions",
    "legacy_pick",
    "legacy_loud",
    "legacy_keep",
    "legacy_grow",
    "legacy_window",
    "legacy_split",
    "legacy_feeder",
    "legacy_strip",
];

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    if !COMMANDS.contains(&cmd) {
        return None;
    }
    let Some(w) = game.world.as_mut() else { return Some(Err("No world is loaded.".into())) };
    if w.career.is_none() {
        return Some(Err("Start a career first: the chases come with a job.".into()));
    }
    lg::desk_answers(w);
    // A job taken today opens its picker now, not with the next day.
    lg::hire_check(w);
    Some(match cmd {
        "legacy_page" => Ok(page(w)),
        "legacy_chases" => Ok(chases(w)),
        "legacy_ambitions" => Ok(picker(w)),
        "legacy_pick" => (|| {
            let id = num(args, "id")? as u32;
            let on = args.get("on").and_then(Value::as_bool).unwrap_or(true);
            let said = ambitions::pick(w, id, on)?;
            let mut v = picker(w);
            v["said"] = json!(said);
            Ok(v)
        })(),
        "legacy_loud" => (|| {
            let id = num(args, "id")? as u32;
            let loud = args.get("loud").and_then(Value::as_bool).unwrap_or(true);
            let said = ambitions::loud(w, id, loud)?;
            let mut v = picker(w);
            v["said"] = json!(said);
            Ok(v)
        })(),
        "legacy_keep" => (|| {
            let later = args.get("later").and_then(Value::as_bool).unwrap_or(false);
            let said = ambitions::keep(w, later)?;
            Ok(json!({ "said": said }))
        })(),
        "legacy_grow" => (|| {
            let c = num(args, "coach")? as usize;
            if c >= w.coaches.len() {
                return Err("No such coach.".into());
            }
            let g = args.get("grow").and_then(Value::as_str).and_then(tree::Grow::from_key);
            tree::set_grow(w, c, g);
            Ok(page(w))
        })(),
        "legacy_window" => (|| {
            let c = num(args, "choice")? as u8;
            lg::nfl::set_choice(w, c);
            Ok(chases(w))
        })(),
        "legacy_split" => (|| {
            let f = |k: &str| args.get(k).and_then(Value::as_f64).unwrap_or(0.0).max(0.0) as f32;
            let s = [f("stars"), f("transfers"), f("class")];
            if s.iter().sum::<f32>() <= 0.0 {
                return Err("Give the money somewhere.".into());
            }
            lg::college::set_split(w, s);
            Ok(chases(w))
        })(),
        "legacy_feeder" => (|| {
            let name = args.get("name").and_then(Value::as_str).ok_or("Missing name.")?.to_string();
            let what = args.get("what").and_then(Value::as_str).ok_or("Missing what.")?.to_string();
            let said = lg::hs::give_feeder(w, &name, &what)?;
            let mut v = chases(w);
            v["said"] = json!(said);
            Ok(v)
        })(),
        "legacy_strip" => Ok(strip(w, args)),
        _ => unreachable!(),
    })
}

fn num(args: &Value, k: &str) -> Result<u64, String> {
    args.get(k)
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .ok_or_else(|| format!("Missing {k}."))
}

fn tref(t: TeamKey) -> TeamRef {
    match t {
        TeamKey::Hs(h) => TeamRef::HighSchool(h as usize),
        TeamKey::College(p) => TeamRef::College(p as usize),
        TeamKey::Nfl(n) => TeamRef::Nfl(n as usize),
        TeamKey::Other(f) => TeamRef::Other(f as usize),
    }
}

fn seat(w: &World) -> Option<(usize, TeamKey)> {
    Some((afm_engine::world::stories::seat_coach(w)?, afm_engine::world::stories::seat_team(w)?))
}

fn level_name(l: Level) -> &'static str {
    match l {
        Level::Nfl => "NFL",
        Level::College => "College",
        _ => "High school",
    }
}

/// A player's id when the world still keeps him (a link), else null.
fn pid(w: &World, id: u32) -> Value {
    if index_of(w, id).is_some() { json!(id) } else { Value::Null }
}

// --- The Legacy screen (34) ----------------------------------------------------------------------

/// A step in words: a bare count takes the ambition's own unit ("5" of
/// "50 players drafted" reads "5 players drafted"; "1" is "the first").
fn step_words(a: &ambitions::Ambition, k: usize) -> String {
    let label = &a.steps[k].label;
    let tail = a.title.split_once(' ').map(|x| x.1).unwrap_or("");
    if label == "1" && !tail.is_empty() {
        format!("the first {}", tail.strip_suffix('s').unwrap_or(tail))
    } else if label.chars().all(|c| c.is_ascii_digit() || c == ',') && !tail.is_empty() {
        format!("{label} {tail}")
    } else {
        label.clone()
    }
}

fn ambition_row(a: &ambitions::Ambition) -> Value {
    let next = a.next().map(|k| {
        format!(
            "Next: {} · pays +{:.2} into {} and {:.2} reputation",
            step_words(a, k),
            a.pay_of(k),
            ambitions::into_words(a),
            a.rep_of(k)
        )
    });
    json!({
        "id": a.id,
        "title": a.title,
        "group": a.group,
        "loud": a.loud,
        "steps": a.steps.iter().map(|s| json!({ "n": s.label, "done": s.done.is_some() })).collect::<Vec<_>>(),
        "next": next.unwrap_or_else(|| "Every step reached".into()),
        "closes": a.closes,
    })
}

fn others(w: &World) -> Vec<Value> {
    ambitions::others(w)
        .into_iter()
        .map(|o| json!({ "who": o.who, "text": o.text, "status": o.status, "warn": o.warn }))
        .collect()
}

fn node(n: &tree::Node, grow: Option<tree::Grow>) -> Value {
    json!({
        "coach": n.coach,
        "name": n.name,
        "line": n.line,
        "parent": n.parent,
        "head": n.head,
        "rival": n.rival,
        "yours": n.yours,
        "grow": grow.map(|g| g.key()),
    })
}

fn tree_json(w: &World) -> Value {
    let Some(t) = tree::tree(w) else { return Value::Null };
    let grow_of = |n: &tree::Node| n.coach.and_then(|c| w.legacy.grow.iter().find(|g| g.0 == c as usize).map(|g| g.1));
    let mut tiles = vec![
        json!([t.head_coaches.to_string(), if t.head_coaches == 1 { "head coach" } else { "head coaches" }]),
        json!([t.coordinators.to_string(), if t.coordinators == 1 { "coordinator" } else { "coordinators" }]),
        json!([t.wins.to_string(), "tree wins"]),
        json!([t.titles.to_string(), if t.titles == 1 { "tree title" } else { "tree titles" }]),
    ];
    if t.comp_picks > 0 {
        tiles.push(json!([t.comp_picks.to_string(), "3rd-round picks"]));
    }
    if let Some(r) = &t.rival {
        tiles.push(json!(["1", format!("rival: {r}")]));
    }
    json!({
        "mentors": t.mentors.iter().map(|n| node(n, None)).collect::<Vec<_>>(),
        "you": node(&t.you, None),
        "gen1": t.gen1.iter().map(|n| node(n, grow_of(n))).collect::<Vec<_>>(),
        "gen2": t.gen2.iter().map(|n| node(n, None)).collect::<Vec<_>>(),
        "tiles": tiles,
    })
}

fn hall_json(w: &World, level: Level) -> Value {
    let Some(c) = hall::case(w, level) else { return Value::Null };
    json!({
        "word": c.words,
        "hall": c.hall,
        "note": c.note,
        "bars": c.bars.iter().map(|b| json!({ "k": b.label, "have": b.have, "need": b.need, "text": b.text })).collect::<Vec<_>>(),
        "moves": c.moves,
        "listK": c.rank_label,
        "listV": if c.rank > 0 { ordinal(c.rank) } else { "—".into() },
        "greats": c.greats,
        "inducted": c.inducted,
    })
}

fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (k, ch) in s.chars().enumerate() {
        if k > 0 && (s.len() - k).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn ordinal(n: u32) -> String {
    let s = match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

fn scheme_json(w: &World, team: TeamKey) -> Value {
    let s = &w.legacy.scheme;
    let now = scheme::fingerprint(w, team).map(scheme::words);
    let mut spread = s.spread.clone();
    spread.sort_by_key(|x| x.year);
    let spread: Vec<scheme::Spread> = spread.into_iter().rev().take(7).rev().collect();
    let last = spread.last();
    let via = last.map_or(Vec::new(), |x| {
        let mut v = Vec::new();
        if x.tree > 0 {
            v.push(format!("your tree: {}", x.tree));
        }
        if x.clinics > 0 {
            v.push(format!("your clinics: {}", x.clinics));
        }
        if x.copiers > 0 {
            v.push(format!("copiers: {}", x.copiers));
        }
        v
    });
    match &s.named {
        Some(n) => json!({
            "name": n.name,
            "since": format!("named by {}, {}", n.by, n.year),
            "who": last.map_or_else(|| "yours alone".to_string(), |x| {
                let t = x.total();
                if t <= 1 { "yours alone so far".to_string() } else { format!("{t} teams, yours included") }
            }),
            "system": scheme::words(n.fp),
            "now": now,
            "years": spread.iter().map(|x| json!([x.year, x.hs, x.college, x.nfl])).collect::<Vec<_>>(),
            "via": via,
            "film": scheme::film_line(w),
            "named": true,
        }),
        None => json!({
            "name": now.clone().unwrap_or_else(|| "No system of your own yet".into()),
            "since": "not named yet",
            "who": "A system gets its name when it wins: two straight seasons at .700 or better, or a title, while few others run it",
            "system": now,
            "now": now,
            "years": Vec::<Value>::new(),
            "via": via,
            "film": scheme::film_line(w),
            "named": false,
        }),
    }
}

fn guys_json(w: &World, level: Level) -> (String, Vec<Value>, Vec<Value>) {
    let all = &w.legacy.guys;
    let since = all.iter().map(|g| g.from).min();
    let sub = match since {
        Some(y) => format!("{} players coached since {y}", thousands(all.len())),
        None => "the players you coach stay yours".into(),
    };
    let tiles = guys::tiles(w, level).into_iter().map(|(k, v)| json!([v.to_string(), k])).collect();
    let rows = guys::notable(w, 5)
        .into_iter()
        .map(|g| {
            let tone = match g.bond_word() {
                "Strong" => 0,
                "Grudge" => 2,
                _ => 1,
            };
            json!({
                "id": pid(w, g.id),
                "name": g.name,
                "line": g.line(w),
                "now": g.now,
                "next": g.next,
                "bond": g.bond_word(),
                "tone": tone,
            })
        })
        .collect();
    (sub, tiles, rows)
}

fn five_rows(w: &World) -> (String, Vec<Value>) {
    match view::five(w) {
        Some((level, five)) => (
            view::five_title(level).to_string(),
            five.iter()
                .map(|c| json!({ "code": c.code, "name": c.name, "line": c.line, "word": c.word, "tone": c.tone }))
                .collect(),
        ),
        None => (String::new(), Vec::new()),
    }
}

fn page(w: &World) -> Value {
    let Some((_, team)) = seat(w) else {
        return json!({ "career": true, "team": Value::Null, "ctx": "Out of work: your legacy waits for your next job." });
    };
    let level = team.level();
    let mine: Vec<Value> = ambitions::mine(w).into_iter().map(ambition_row).collect();
    let (guys_sub, g_tiles, guys) = guys_json(w, level);
    let (five_t, five) = five_rows(w);
    let reached = w.legacy.ambitions.iter().filter(|a| a.done.is_some()).count();
    let closed = w.legacy.ambitions.iter().filter(|a| a.closed.is_some()).count();
    json!({
        "career": true,
        "team": views::team_brief(w, tref(team)),
        "level": level_name(level),
        "ctx": view::context(w),
        "ambitions": mine,
        "reached": reached,
        "closed": closed,
        "picker": w.legacy.picker.is_some(),
        "others": others(w),
        "tree": tree_json(w),
        "hall": hall_json(w, level),
        "scheme": scheme_json(w, team),
        "guysSub": guys_sub,
        "gStats": g_tiles,
        "guys": guys,
        "fiveT": five_t,
        "five": five,
        "log": w.legacy.log.iter().rev().take(8).map(|p| json!({
            "date": views::date_text(p.date),
            "sys": p.sys,
            "what": p.what,
            "lesson": p.lesson,
            "into": p.into,
            "rep": p.rep,
        })).collect::<Vec<_>>(),
    })
}

// --- The five chases (36) ------------------------------------------------------------------------

/// The screen each chase lives on (the UI's screen names).
fn home_screen(code: &str) -> &'static str {
    match code {
        "L1" => "fo_cap",
        "L2" => "depth",
        "L3" => "draftrecord",
        "L4" => "fo_office",
        "L5" => "standings",
        "L6" => "recruit_board",
        "L7" => "team",
        "L8" => "traditions",
        "L9" => "nil_desk",
        "L10" => "schedule",
        "L11" => "school",
        "L12" => "history",
        "L13" => "hsside",
        "L14" => "area",
        _ => "market",
    }
}

fn chase_json(c: &view::Chase) -> Value {
    json!({
        "code": c.code,
        "name": c.name,
        "word": c.word,
        "tone": c.tone,
        "home": c.home,
        "screen": home_screen(c.code),
        "line": c.line,
        "vis": c.vis,
        "kv": c.kv.iter().map(|(k, v, t)| json!({ "k": k, "v": v, "tone": t })).collect::<Vec<_>>(),
    })
}

fn chases(w: &World) -> Value {
    let Some((_, team)) = seat(w) else {
        return json!({ "team": Value::Null, "ctx": "Out of work: the chases come with a job.", "panels": [] });
    };
    let Some((level, five)) = view::five(w) else {
        return json!({ "team": views::team_brief(w, tref(team)), "ctx": view::context(w), "title": "Your chases", "panels": [] });
    };
    let mut extra = json!({});
    match team {
        TeamKey::Nfl(t) => extra["window"] = json!(lg::nfl::window(w, t as usize).choice),
        TeamKey::College(_) => extra["split"] = json!(w.legacy.college.split),
        TeamKey::Hs(_) => {
            extra["feeders"] = json!(
                w.legacy
                    .hs
                    .feeders
                    .iter()
                    .map(|f| json!({
                        "name": f.name,
                        "coach": f.coach,
                        "playbook": f.playbook,
                        "clinic": f.clinic,
                        "assistant": f.assistant,
                        "equipment": f.equipment,
                    }))
                    .collect::<Vec<_>>()
            );
            extra["boosters"] = json!(w.front_office.school.boosters.balance);
        }
        TeamKey::Other(_) => {}
    }
    json!({
        "team": views::team_brief(w, tref(team)),
        "level": level_name(level),
        "ctx": view::context(w),
        "title": view::five_title(level),
        "panels": five.iter().map(chase_json).collect::<Vec<_>>(),
        "extra": extra,
    })
}

/// The strip a chase's own screen shows: the asked codes, or your level's five.
fn strip(w: &World, args: &Value) -> Value {
    let want: Vec<String> = args
        .get("codes")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    let Some((_, five)) = view::five(w) else { return json!({ "rows": [], "ambitions": [] }) };
    let rows: Vec<Value> = five
        .iter()
        .filter(|c| want.is_empty() || want.iter().any(|x| x == c.code))
        .map(|c| json!({ "code": c.code, "name": c.name, "word": c.word, "tone": c.tone, "line": c.line }))
        .collect();
    let ambitions: Vec<Value> = if want.is_empty() {
        ambitions::mine(w)
            .into_iter()
            .map(|a| {
                let next = a.next().map_or_else(|| "every step reached".to_string(), |k| step_words(a, k));
                let done = a.steps.iter().filter(|s| s.done.is_some()).count();
                json!({ "title": a.title, "next": next, "done": done, "of": a.steps.len(), "loud": a.loud })
            })
            .collect()
    } else {
        Vec::new()
    };
    json!({ "rows": rows, "ambitions": ambitions, "picker": w.legacy.picker.is_some() })
}

// --- The ambition picker (35) --------------------------------------------------------------------

fn loud_pay(p: f32) -> f32 {
    ((p * 1.2).min(1.0) * 20.0).round() / 20.0
}

fn picker(w: &World) -> Value {
    let held: Vec<Value> = ambitions::mine(w).into_iter().map(ambition_row).collect();
    let Some(p) = w.legacy.picker.as_ref() else {
        return json!({ "open": false, "held": held, "others": others(w) });
    };
    let level = p.team.level();
    let offers: Vec<Value> = p
        .offers
        .iter()
        .filter_map(|id| w.legacy.ambitions.iter().find(|a| a.id == *id))
        .map(|a| {
            json!({
                "id": a.id,
                "group": a.group,
                "title": a.title,
                "why": a.why,
                "closes": a.closes,
                "picked": a.picked,
                "loud": a.loud,
                "quote": ambitions::quote(w, a),
                "into": ambitions::into_words(a),
                "steps": a.steps.iter().map(|s| json!({ "n": s.label, "p": s.pay, "pl": loud_pay(s.pay) })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let said_where = match level {
        Level::Nfl => "Said out loud, it's on the back pages and talk radio within the hour, and they keep the tape.",
        Level::College => "Said out loud, it's the headline in the paper and the fans hold you to it.",
        _ => "Said out loud, it's in the local paper by Wednesday and the town holds you to it.",
    };
    json!({
        "open": true,
        "team": views::team_brief(w, tref(p.team)),
        "level": level_name(level),
        "ctx": format!("Taking the job · {} · {} {}", lg::tname(w, p.team), p.how.to_lowercase(), p.date.pretty()),
        "start": p.start,
        "later": p.later.map(|d| d.pretty()),
        "introT": format!("{} · {}", p.how, p.date.pretty()),
        "introW": said_where,
        "offers": offers,
        "held": held,
        "max": ambitions::MAX,
        "others": others(w),
    })
}
