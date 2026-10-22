//! Lane commands: people (docs/final-build/plan.md). `call` answers the
//! commands this lane owns and returns None for any other.
//!
//! Lane 6 of the final build is section 6 on your side: the meetings and
//! talks played as scenes (the takeover's first two weeks, one-on-ones,
//! team talks, press conferences, your boss, parents, staff interviews),
//! your rules and the room, your staff's deals and the market, your
//! coaching network, and the people around your team. The world side lives
//! in `afm_engine::world::folk`.
//!
//! Every text the engine writes carries the screens' link marks
//! (`[[p:12|Name]]` and the like); the screens draw them with `rich`.

use afm_engine::calendar::Level;
use afm_engine::world::World;
use afm_engine::world::coach::{Group, Role};
use afm_engine::world::folk::cast::{self, CastRef};
use afm_engine::world::folk::rules::RuleKind;
use afm_engine::world::folk::scene::{Kind, Scene, Who};
use afm_engine::world::folk::talks::{self, Line, PromiseTo, Subject};
use afm_engine::world::folk::{self as folk, Team, boss, facts, network, staffing, ties};
use serde_json::{Value, json};

use crate::views::date_text;
use crate::{Game, Reply};

const COMMANDS: [&str; 27] = [
    "people_agenda",
    "people_expect",
    "people_scene",
    "people_answer",
    "people_hand",
    "people_talk",
    "people_team_talk",
    "people_line",
    "people_player",
    "people_room",
    "people_rules",
    "people_rules_set",
    "people_council",
    "people_boss",
    "people_boss_meeting",
    "people_staff",
    "people_candidates",
    "people_interview",
    "people_offer",
    "people_fire",
    "people_raise",
    "people_contacts",
    "people_reach",
    "people_cast",
    "people_league",
    "people_log",
    "people_takeover",
];

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    if !COMMANDS.contains(&cmd) {
        return None;
    }
    let Some(w) = game.world.as_mut() else { return Some(Err("No world is loaded.".into())) };
    Some(match cmd {
        "people_agenda" => Ok(agenda(w)),
        "people_scene" => num(args, "id").and_then(|id| scene(w, id as u32)),
        "people_answer" => (|| {
            let id = num(args, "id")? as u32;
            let code = num(args, "code")? as u32;
            folk::scene::answer(w, id, code)?;
            scene(w, id)
        })(),
        "people_expect" => (|| {
            let id = num(args, "id")? as u32;
            let code = num(args, "code")? as u32;
            let s = w.folk.scenes.iter().find(|s| s.id == id).ok_or("No such meeting.")?;
            Ok(match talks::expect(w, s, code) {
                Some((what, sure, who)) => json!({ "what": what, "sure": sure, "who": who }),
                None => Value::Null,
            })
        })(),
        "people_hand" => (|| {
            let id = num(args, "id")? as u32;
            folk::scene::hand(w, id)?;
            scene(w, id)
        })(),
        "people_talk" => (|| {
            let id = num(args, "id")? as u32;
            let subject = args.get("subject").and_then(Value::as_str).unwrap_or("role");
            let sub = Subject::ALL.iter().copied().find(|s| s.key() == subject).unwrap_or(Subject::Role);
            let sid = talks::start_talk(w, id, sub)?;
            Ok(json!({ "scene": sid }))
        })(),
        "people_team_talk" => talks::start_team_talk(w).map(|sid| json!({ "scene": sid })),
        "people_line" => (|| {
            let id = num(args, "id")? as u32;
            let line = match args.get("line").and_then(Value::as_str).unwrap_or("") {
                "level" => Some(Line::Level),
                "push" => Some(Line::Push),
                "watch" => Some(Line::Watch),
                _ => None,
            };
            talks::set_line(w, id, line);
            player(w, id)
        })(),
        "people_player" => num(args, "id").and_then(|id| player(w, id as u32)),
        "people_room" => Ok(room(w)),
        "people_rules" => Ok(rules(w)),
        "people_rules_set" => rules_set(w, args),
        "people_council" => (|| {
            let ids: Vec<u32> = args
                .get("ids")
                .and_then(Value::as_array)
                .ok_or("Missing ids.")?
                .iter()
                .filter_map(|v| v.as_u64().map(|x| x as u32))
                .take(8)
                .collect();
            w.folk.rules.council = ids;
            Ok(rules(w))
        })(),
        "people_boss" => Ok(boss_page(w)),
        "people_boss_meeting" => boss::request_meeting(w).map(|sid| json!({ "scene": sid })),
        "people_staff" => Ok(staff(w)),
        "people_candidates" => role_arg(args).map(|r| candidates(w, r)),
        "people_interview" => (|| {
            let c = num(args, "coach")? as usize;
            let r = role_arg(args)?;
            staffing::start_interview(w, c, r).map(|sid| json!({ "scene": sid }))
        })(),
        "people_offer" => (|| {
            let c = num(args, "coach")? as usize;
            let r = role_arg(args)?;
            let salary = args.get("salary").and_then(Value::as_f64).ok_or("Missing salary.")?;
            let years = args.get("years").and_then(Value::as_i64).unwrap_or(2) as i32;
            let text = staffing::offer(w, c, r, salary, years)?;
            Ok(json!({ "text": text, "staff": staff(w) }))
        })(),
        "people_fire" => {
            num(args, "coach").and_then(|c| staffing::start_firing(w, c as usize).map(|sid| json!({ "scene": sid })))
        }
        "people_raise" => (|| {
            let c = num(args, "coach")? as usize;
            let share = args.get("share").and_then(Value::as_f64).unwrap_or(0.1);
            let text = staffing::raise(w, c, share)?;
            Ok(json!({ "text": text, "staff": staff(w) }))
        })(),
        "people_contacts" => Ok(contacts(w)),
        "people_reach" => (|| {
            let c = num(args, "coach")? as usize;
            let how = network::How::from(args.get("how").and_then(Value::as_str).unwrap_or("call"));
            let why = network::Why::from(args.get("why").and_then(Value::as_str).unwrap_or("catchup"));
            let p = args.get("player").and_then(Value::as_u64).map(|x| x as u32);
            let text = network::reach_out(w, c, how, why, p)?;
            Ok(json!({ "text": text, "contacts": contacts(w) }))
        })(),
        "people_cast" => args
            .get("tok")
            .and_then(Value::as_str)
            .ok_or_else(|| "Missing tok.".to_string())
            .and_then(|t| cast_page(w, t)),
        "people_league" => Ok(league(w)),
        "people_log" => Ok(log(w, args)),
        "people_takeover" => Ok(takeover(w)),
        _ => Err(format!("Unknown command: {cmd}")),
    })
}

fn num(args: &Value, k: &str) -> Result<u64, String> {
    args.get(k)
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .ok_or_else(|| format!("Missing {k}."))
}

/// A staff role from `{role}`: "oc", "dc" or a position group's name.
fn role_arg(args: &Value) -> Result<Role, String> {
    let r = args.get("role").and_then(Value::as_str).ok_or("Missing role.")?;
    Ok(match r {
        "oc" => Role::OffensiveCoordinator,
        "dc" => Role::DefensiveCoordinator,
        g => Role::PositionCoach(
            Group::ALL.iter().copied().find(|x| group_key(*x) == g).ok_or_else(|| format!("No such role: {g}"))?,
        ),
    })
}

fn group_key(g: Group) -> &'static str {
    match g {
        Group::QB => "qb",
        Group::RB => "rb",
        Group::Receivers => "wr",
        Group::OL => "ol",
        Group::DL => "dl",
        Group::LB => "lb",
        Group::DB => "db",
        Group::Specialists => "st",
    }
}

fn role_key(r: Role) -> String {
    match r {
        Role::OffensiveCoordinator => "oc".into(),
        Role::DefensiveCoordinator => "dc".into(),
        Role::PositionCoach(g) => group_key(g).into(),
        Role::HeadCoach => "hc".into(),
    }
}

/// A text without its link marks: "[[p:7|Jalen Carter]]" reads "Jalen Carter".
pub(crate) fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(a) = rest.find("[[") {
        out.push_str(&rest[..a]);
        let Some(z) = rest[a..].find("]]") else {
            out.push_str(&rest[a..]);
            return out;
        };
        let inner = &rest[a + 2..a + z];
        out.push_str(inner.rsplit_once('|').map_or(inner, |(_, n)| n));
        rest = &rest[a + z + 2..];
    }
    out.push_str(rest);
    out
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn team_json(w: &World, t: Team) -> Value {
    let (k, i) = t.parts();
    json!({ "kind": k, "id": i, "name": t.name(w), "link": t.link(w) })
}

fn level_key(l: Level) -> &'static str {
    match l {
        Level::Nfl => "nfl",
        Level::College => "college",
        _ => "hs",
    }
}

fn money(d: f64) -> String {
    if d >= 1.0e6 {
        format!("${:.2}M", d / 1.0e6)
    } else if d >= 1.0e3 {
        format!("${:.0}K", d / 1.0e3)
    } else {
        format!("${d:.0}")
    }
}

/// Trust in words (players, staff and the people around you).
fn trust_words(t: f32) -> &'static str {
    match t {
        x if x >= 0.8 => "Would run through a wall for you",
        x if x >= 0.65 => "Trusts you",
        x if x >= 0.5 => "Buying in",
        x if x >= 0.38 => "Waiting to see",
        x if x >= 0.25 => "Doubts you",
        _ => "Has stopped listening",
    }
}

fn warmth_words(t: f32) -> &'static str {
    match t {
        x if x >= 0.8 => "Close friend",
        x if x >= 0.6 => "Good friend",
        x if x >= 0.4 => "Friendly",
        x if x >= 0.2 => "Acquaintance",
        _ => "A name in your phone",
    }
}

fn who_json(w: &World, who: &Who) -> Value {
    match who {
        Who::Player(id) => match folk::person(w, *id) {
            Some(i) => {
                json!({ "kind": "player", "id": id, "link": folk::player_link(w, i), "pos": w.people[i].player.position.name() })
            }
            None => json!({ "kind": "player", "id": id, "link": "A former player" }),
        },
        Who::Coach(c) => {
            json!({ "kind": "coach", "id": c, "link": folk::coach_link(w, *c), "role": facts::coach_role(w, *c) })
        }
        Who::Cast(tok) => match cast::from_token(w, tok) {
            Some(c) => json!({ "kind": "cast", "tok": tok, "link": c.link(), "role": c.title }),
            None => json!({ "kind": "cast", "tok": tok, "link": "" }),
        },
        Who::Group(g) => json!({ "kind": "group", "link": g }),
    }
}

fn scene_brief(w: &World, s: &Scene) -> Value {
    json!({
        "id": s.id,
        "kind": format!("{:?}", s.kind),
        "kindName": s.kind.name(),
        "title": s.title,
        "date": s.date.0,
        "dateText": date_text(s.date),
        "today": s.date == w.today,
        "late": s.date < w.today,
        "place": s.place,
        "stops": s.stops,
        "opened": s.opened,
        "done": s.done,
        "handed": s.handed,
        "outcome": s.outcome,
        "who": s.who.iter().take(4).map(|x| who_json(w, x)).collect::<Vec<_>>(),
    })
}

/// One scene, opened when you look at it.
fn scene(w: &mut World, id: u32) -> Reply {
    // A meeting set for a later day stays closed until its day.
    let future = w.folk.scenes.iter().find(|s| s.id == id).is_some_and(|s| s.date > w.today && !s.opened);
    let s = if future { w.folk.scenes.iter().find(|s| s.id == id).cloned() } else { folk::scene::open(w, id) }
        .ok_or("No such meeting.")?;
    let mut v = scene_brief(w, &s);
    v["future"] = json!(future);
    v["lines"] = if future {
        let at = if s.place.is_empty() { String::new() } else { format!(", {}", s.place) };
        json!([{ "by": 0, "name": "", "text": format!("Set for {}{at}. Your staff has it on the calendar; it opens that morning.", date_text(s.date)), "tell": null }])
    } else {
        json!(s.lines)
    };
    // A move's name speaks to whoever is across from you: "Tell them
    // straight" to the team, "Tell her straight" to the principal.
    let them = match s.who.first() {
        Some(Who::Group(_)) | None => "them",
        Some(Who::Cast(tok)) if cast::from_token(w, tok).is_some_and(|c| c.she) => "her",
        _ => "him",
    };
    let picks: Vec<Value> = s
        .picks
        .iter()
        .map(|p| {
            let mut v = json!(p);
            v["mv"] = json!(p.mv.replace(" him ", &format!(" {them} ")));
            v
        })
        .collect();
    v["picks"] = json!(picks);
    v["docs"] = json!(s.docs);
    v["step"] = json!(s.step);
    v["hours"] = json!(talks::hours_left(w));
    v["team"] = s.team.map_or(Value::Null, |t| team_json(w, t));
    // The others waiting today, for "next meeting".
    let next = w
        .folk
        .scenes
        .iter()
        .filter(|x| !x.done && x.id != id && x.date <= w.today)
        .map(|x| json!({ "id": x.id, "title": x.title, "kindName": x.kind.name() }))
        .next();
    v["next"] = next.unwrap_or(Value::Null);
    Ok(v)
}

/// The people hub: what waits for you, your week's hours, the room, your
/// boss, the takeover and the latest of the log.
fn agenda(w: &World) -> Value {
    let Some(team) = Team::seat(w) else { return json!({ "none": true }) };
    let level = team.level();
    let mut waiting: Vec<&Scene> = w.folk.scenes.iter().filter(|s| !s.done).collect();
    waiting.sort_by_key(|s| (s.date, s.id));
    let recent: Vec<Value> = w.folk.scenes.iter().rev().filter(|s| s.done).take(8).map(|s| scene_brief(w, s)).collect();
    let b = boss::boss(w, team);
    let bt = boss::trust(w, team);
    let mood = talks::team_mood(w, team);
    let season = w.year;
    let goals: Vec<Value> = w
        .folk
        .goals
        .iter()
        .filter(|g| g.season >= season - 1)
        .map(|g| json!({ "season": g.season, "words": cap(&g.kind.words(level)), "met": g.met }))
        .collect();
    let promises: Vec<Value> = w
        .folk
        .promises
        .iter()
        .rev()
        .filter(|p| p.kept.is_none() || p.due.days_to(w.today) < 60)
        .take(12)
        .map(|p| {
            json!({ "who": p.who, "what": p.kind.words(), "made": date_text(p.made), "due": date_text(p.due), "kept": p.kept, "public": p.public })
        })
        .collect();
    let due: Vec<Value> =
        facts::due(w, team).into_iter().take(6).map(|(d, t)| json!({ "date": date_text(d), "text": t })).collect();
    json!({
        "team": team_json(w, team),
        "level": level_key(level),
        "date": date_text(w.today),
        "hours": { "left": talks::hours_left(w), "week": talks::week_budget(w) },
        "mood": { "value": mood, "words": talks::mood_words(mood) },
        "boss": { "link": b.link(), "title": b.title, "trust": bt, "words": boss::trust_words(bt) },
        "expectation": w.folk.mine.expectation,
        "waiting": waiting.iter().map(|s| scene_brief(w, s)).collect::<Vec<_>>(),
        "recent": recent,
        "goals": goals,
        "promises": promises,
        "due": due,
        "takeover": w.folk.takeover.as_ref().and_then(|t| t.started).is_some_and(|d| d.days_to(w.today) <= 30),
        "subjects": Subject::ALL.iter().map(|s| json!({ "key": s.key(), "name": cap(s.name()) })).collect::<Vec<_>>(),
        "log": w.folk.log.iter().rev().take(14).map(log_entry).collect::<Vec<_>>(),
    })
}

fn log_entry(e: &folk::Entry) -> Value {
    json!({ "date": e.date.map(date_text), "topic": e.topic, "source": e.source, "text": e.text, "angle": e.angle, "check": e.check })
}

fn log(w: &World, args: &Value) -> Value {
    let topic = args.get("topic").and_then(Value::as_str).unwrap_or("");
    let rows: Vec<Value> =
        w.folk.log.iter().rev().filter(|e| topic.is_empty() || e.topic == topic).take(300).map(log_entry).collect();
    let mut topics: Vec<&str> = w.folk.log.iter().map(|e| e.topic.as_str()).collect();
    topics.sort_unstable();
    topics.dedup();
    json!({ "rows": rows, "topics": topics })
}

/// The takeover file: the first two weeks' meetings, what each found, the
/// flight risks, the holdovers and the claims to check after spring.
fn takeover(w: &World) -> Value {
    let Some(t) = w.folk.takeover.as_ref() else { return json!({ "none": true }) };
    let meetings: Vec<Value> = t
        .scenes
        .iter()
        .filter_map(|id| w.folk.scenes.iter().find(|s| s.id == *id))
        .map(|s| scene_brief(w, s))
        .collect();
    let risks: Vec<Value> = t
        .risks
        .iter()
        .filter_map(|(id, why)| {
            folk::person(w, *id).map(
                |i| json!({ "link": folk::player_link(w, i), "pos": w.people[i].player.position.name(), "why": why }),
            )
        })
        .collect();
    let claims: Vec<Value> = t
        .claims
        .iter()
        .filter_map(|(by, id, dir)| {
            folk::person(w, *id).map(|i| json!({ "by": by, "link": folk::player_link(w, i), "dir": if *dir > 0 { "oversold" } else { "undersold" } }))
        })
        .collect();
    json!({
        "team": t.team.map(|x| team_json(w, x)),
        "started": t.started.map(date_text),
        "old": t.old_coach.map(|c| folk::coach_link(w, c)),
        "fit": facts::hire_grade(t.fit as f64),
        "meetings": meetings,
        "risks": risks,
        "kept": t.kept.iter().map(|&c| folk::coach_link(w, c)).collect::<Vec<_>>(),
        "letGo": t.let_go.iter().map(|&c| folk::coach_link(w, c)).collect::<Vec<_>>(),
        "claims": claims,
        "checked": t.checked,
        "window": w.folk.window.map(|(_, a, b)| json!({ "open": date_text(a), "close": date_text(b) })),
    })
}

/// A player's people side: his mood and its reasons, his trust in you, his
/// ties, his life and any case, your line to his position coach.
fn player(w: &World, id: u32) -> Reply {
    let i = folk::person(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    let you = w.career.as_ref().map(|c| c.coach as u32);
    let mine = Team::seat(w).is_some_and(|t| t.roster(w).contains(&i));
    let reasons: Vec<Value> = talks::reasons(w, id)
        .into_iter()
        .map(|(d, why, by)| json!({ "date": date_text(d), "why": why, "up": by > 0.0 }))
        .collect();
    let ties: Vec<Value> = ties::of(w, id)
        .into_iter()
        .filter_map(|(o, words)| folk::person(w, o).map(|j| json!({ "link": folk::player_link(w, j), "words": words })))
        .collect();
    let line = w.folk.lines.iter().find(|x| x.0 == id).map(|x| x.1.words());
    let out = w
        .folk
        .out
        .iter()
        .find(|x| x.0 == id && x.1 >= w.today)
        .map(|x| format!("{} (through {})", x.2, date_text(x.1)));
    let promises: Vec<Value> = w
        .folk
        .promises
        .iter()
        .filter(|x| matches!(x.to, PromiseTo::Player(pid) if pid == id))
        .map(|x| json!({ "what": x.kind.words(), "due": date_text(x.due), "kept": x.kept }))
        .collect();
    let (_, worry) = talks::concern(w, i);
    let trust = (Some(p.trust_in) == you).then_some(p.trust);
    // In his ear (screens topic 8): who he listens to, each with how sure
    // your staff is about it.
    let mut ear: Vec<Value> = Vec::new();
    for (o, words) in ties::of(w, id) {
        if let Some(j) = folk::person(w, o) {
            let sure = if words.starts_with("very close") || words == "family" || words.contains("mentor") {
                "Sure"
            } else {
                "Likely"
            };
            ear.push(json!({ "who": folk::player_link(w, j), "what": cap(&words), "sure": sure }));
        }
    }
    let parent = talks::parent_word(w, i);
    let style = talks::parent_voice(w, i);
    if p.stage != afm_engine::world::Stage::Pro || p.player.age < 25 {
        ear.push(json!({ "who": cap(parent), "what": style.0, "sure": style.1 }));
    }
    if p.pro.as_ref().is_some_and(|c| c.agent.is_some()) {
        ear.push(
            json!({ "who": "His agent", "what": "Talks to him about his next deal more than you do", "sure": "Sure" }),
        );
    }
    if p.stage == afm_engine::world::Stage::College && p.cal.lean > 0.25 {
        ear.push(json!({ "who": "Outside voices", "what": "People around him are telling him to look at the portal", "sure": "Likely" }));
    }
    Ok(json!({
        "id": id,
        "mine": mine,
        "mood": { "value": p.mood, "words": talks::mood_words(p.mood) },
        "reasons": reasons,
        "trust": trust.map(|t| json!({ "value": t, "words": trust_words(t) })),
        "ties": ties,
        "mentor": ties::mentor_of(w, id).and_then(|m| folk::person(w, m)).map(|j| folk::player_link(w, j)),
        "life": folk::life::open_event(w, id),
        "case": w.folk.courts.open_case(id),
        "out": out,
        "line": line,
        "promises": promises,
        // Your staff's sense of what's on his mind, only for your own players
        // (what your position coach would tell you).
        "staffSays": if mine { Some(cap(&worry)) } else { None },
        "ear": ear,
        "subjects": Subject::ALL.iter().map(|s| json!({ "key": s.key(), "name": cap(s.name()) })).collect::<Vec<_>>(),
        "hours": talks::hours_left(w),
    }))
}

/// The room: every player of yours by mood, with what your staff hears.
fn room(w: &World) -> Value {
    let Some(team) = Team::seat(w) else { return json!({ "none": true }) };
    let you = w.career.as_ref().map(|c| c.coach as u32);
    let caps: Vec<u32> = folk::takeover::captains_of(w, team).iter().map(|&i| w.people[i].player.id).collect();
    let rows: Vec<Value> = team
        .roster(w)
        .into_iter()
        .map(|i| {
            let p = &w.people[i];
            let id = p.player.id;
            let why = talks::reasons(w, id).into_iter().next().map(|x| x.1);
            let friends = ties::friends(w, id).len();
            json!({
                "id": id,
                "link": folk::player_link(w, i),
                "pos": p.player.position.name(),
                "age": p.player.age,
                "mood": p.mood,
                "moodWords": talks::mood_words(p.mood),
                "trust": if Some(p.trust_in) == you { Some(p.trust) } else { None },
                "why": why.unwrap_or_else(|| cap(&talks::concern(w, i).1)),
                "friends": friends,
                "captain": caps.contains(&id),
                "council": w.folk.rules.council.contains(&id),
                "line": w.folk.lines.iter().find(|x| x.0 == id).map(|x| x.1.words()),
                "out": folk::rules::sitting(w, id, w.today),
            })
        })
        .collect();
    let mood = talks::team_mood(w, team);
    json!({ "team": team_json(w, team), "mood": { "value": mood, "words": talks::mood_words(mood) }, "rows": rows, "hours": talks::hours_left(w) })
}

/// Your rulebook, the council, rituals and the rulings so far.
fn rules(w: &World) -> Value {
    let Some(team) = Team::seat(w) else { return json!({ "none": true }) };
    let level = team.level();
    let book = &w.folk.rules;
    let kinds: Vec<Value> = RuleKind::ALL
        .iter()
        .filter(|k| k.at(level))
        .map(|k| json!({ "key": k.key(), "name": k.name(), "about": k.about(), "strict": book.strict(*k) }))
        .collect();
    let council: Vec<Value> = book
        .council
        .iter()
        .filter_map(|&id| folk::person(w, id).map(|i| json!({ "id": id, "link": folk::player_link(w, i) })))
        .collect();
    let rulings: Vec<Value> = book
        .rulings
        .iter()
        .rev()
        .take(60)
        .map(|r| {
            json!({
                "date": date_text(r.date),
                "who": folk::person(w, r.id).map_or_else(|| "A former player".to_string(), |i| folk::player_link(w, i)),
                "rule": r.kind.name(),
                "response": r.response.words(level),
                "star": r.star,
            })
        })
        .collect();
    let out: Vec<Value> = w
        .folk
        .out
        .iter()
        .filter(|x| x.1 >= w.today)
        .filter_map(|x| {
            folk::person(w, x.0)
                .map(|i| json!({ "link": folk::player_link(w, i), "through": date_text(x.1), "why": x.2 }))
        })
        .collect();
    json!({
        "level": level_key(level),
        "kinds": kinds,
        "load": book.load(),
        "rituals": book.rituals,
        "ritualsOn": level != Level::HighSchool,
        "council": council,
        "rulings": rulings,
        "out": out,
        "trouble": folk::rules::trouble_factor(w),
    })
}

fn rules_set(w: &mut World, args: &Value) -> Reply {
    if let Some(r) = args.get("rituals").and_then(Value::as_u64) {
        w.folk.rules.rituals = r.min(2) as u8;
    }
    if let Some(k) = args.get("key").and_then(Value::as_str) {
        let kind = RuleKind::from_key(k).ok_or("No such rule.")?;
        let strict = args.get("strict").and_then(Value::as_u64).unwrap_or(0).min(3) as u8;
        w.folk.rules.set_rule(kind, strict);
    }
    Ok(rules(w))
}

/// Your boss: who, his trust, the goals by season, what you've asked for.
fn boss_page(w: &World) -> Value {
    let Some(team) = Team::seat(w) else { return json!({ "none": true }) };
    let level = team.level();
    let b = boss::boss(w, team);
    let t = boss::trust(w, team);
    let mut seasons: Vec<i32> = w.folk.goals.iter().map(|g| g.season).collect();
    seasons.sort_unstable();
    seasons.dedup();
    let goals: Vec<Value> = seasons
        .iter()
        .rev()
        .map(|&s| {
            json!({
                "season": s,
                "rows": w.folk.goals.iter().filter(|g| g.season == s).map(|g| json!({
                    "words": cap(&g.kind.words(level)),
                    "met": g.met,
                    "now": if g.met.is_none() { Some(boss::goal_met(w, team, g.kind, s)) } else { None },
                    "deal": g.deal,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let asks: Vec<Value> = w
        .folk
        .asks
        .iter()
        .rev()
        .map(|a| json!({ "date": date_text(a.date), "what": cap(a.kind.words()), "granted": a.granted, "words": a.words }))
        .collect();
    let meetings: Vec<Value> = w
        .folk
        .scenes
        .iter()
        .filter(|s| matches!(s.kind, Kind::BossMeeting | Kind::BossFirst))
        .rev()
        .take(10)
        .map(|s| scene_brief(w, s))
        .collect();
    let others: Vec<Value> = folk::takeover::roles_around(team)
        .into_iter()
        .filter(|r| {
            matches!(
                r,
                cast::Role::President
                    | cast::Role::Superintendent
                    | cast::Role::OwnerKin
                    | cast::Role::TeamPresident
                    | cast::Role::Gm
                    | cast::Role::Principal
                    | cast::Role::AthleticDirector
                    | cast::Role::SchoolAd
                    | cast::Role::Owner
            )
        })
        .filter(|&r| r != cast::boss_role(team) && cast::has(w, team, r))
        .map(|r| {
            let c = cast::get(w, CastRef::new(team, r, 0));
            json!({ "link": c.link(), "title": c.title, "trust": w.folk.mine.trust_of(c.key()).map(trust_words) })
        })
        .collect();
    let can = !w.folk.scenes.iter().any(|s| s.kind == Kind::BossMeeting && s.c == 3 && s.date.days_to(w.today) < 30);
    json!({
        "team": team_json(w, team),
        "boss": cast_json(w, &b),
        "trust": t,
        "trustWords": boss::trust_words(t),
        "expectation": w.folk.mine.expectation,
        "goals": goals,
        "asks": asks,
        "meetings": meetings,
        "others": others,
        "canRequest": can,
    })
}

fn cast_json(w: &World, c: &cast::Cast) -> Value {
    let years = (w.year - c.since).max(0);
    let mut v = json!({
        "tok": c.r.token(),
        "link": c.link(),
        "name": c.name,
        "title": c.title,
        "age": c.age,
        "since": c.since,
        "years": years,
        "she": c.she,
        "reputation": c.reputation(),
        "trust": w.folk.mine.trust_of(c.key()).map(|t| json!({ "value": t, "words": trust_words(t) })),
        "team": team_json(w, c.r.team),
    });
    if let Some(b) = &c.booster {
        v["booster"] = json!({
            "business": b.business,
            "gift": b.gift,
            "yearly": if b.yearly > 0.0 { Some(money(b.yearly)) } else { None },
            "wants": b.wants.iter().map(|x| cap(x.words())).collect::<Vec<_>>(),
            "closeTo": b.close_to,
            "big": b.big,
            "loyalOld": b.loyal_old > 0.5,
        });
    }
    if let Some(r) = &c.reporter {
        v["reporter"] = json!({
            "outlet": r.outlet,
            "beat": r.beat,
            "lean": match r.lean { x if x > 0.35 => "Friendly to the program", x if x < -0.35 => "A critic", _ => "Down the middle" },
            "accuracy": match r.accuracy { x if x > 0.8 => "His reports hold up", x if x > 0.6 => "Usually right", _ => "Hit and miss" },
            "sources": r.sources,
            "leaks": r.leaks,
            "national": r.national,
        });
    }
    v
}

/// A person around a team, by link token.
fn cast_page(w: &World, tok: &str) -> Reply {
    let c = cast::from_token(w, tok).ok_or("No such person.")?;
    let mut v = cast_json(w, &c);
    let scenes: Vec<Value> = w
        .folk
        .scenes
        .iter()
        .rev()
        .filter(|s| s.who.iter().any(|x| matches!(x, Who::Cast(t) if t == tok)))
        .take(12)
        .map(|s| scene_brief(w, s))
        .collect();
    v["scenes"] = json!(scenes);
    let name = c.name.clone();
    v["log"] = json!(
        w.folk
            .log
            .iter()
            .rev()
            .filter(|e| e.source.contains(&name) || e.text.contains(&name))
            .take(20)
            .map(log_entry)
            .collect::<Vec<_>>()
    );
    Ok(v)
}

/// Your staff: each assistant's deal, what he's like to work with, the
/// openings and the support staff.
fn staff(w: &World) -> Value {
    let Some(team) = Team::seat(w) else { return json!({ "none": true }) };
    let year = w.year;
    let rows: Vec<Value> = facts::assistants(w, team)
        .into_iter()
        .map(|(c, r)| {
            let co = &w.coaches[c];
            let deal = w.folk.staff.deal(c);
            json!({
                "coach": c,
                "link": folk::coach_link(w, c),
                "role": cap(r.name()),
                "roleKey": role_key(r),
                "age": co.age,
                "since": co.job.map(|j| j.since),
                "salary": deal.map(|d| money(d.salary)),
                "through": deal.map(|d| d.through),
                "buyout": deal.map(|d| format!("{:.0}%", d.buyout * 100.0)),
                "owed": deal.map(|d| money(d.salary * (d.through - year + 1).max(0) as f64 * d.buyout as f64)),
                "cameUp": co.came_up.name(),
                "trust": w.folk.mine.trust_of(folk::coach_key(c)).map(trust_words),
                "leave": folk::life::on_leave(w, c),
                "family": folk::life::family_words(w, c),
                "climber": co.personality.ambition >= 14.0,
            })
        })
        .collect();
    let openings: Vec<Value> =
        staffing::openings(w).into_iter().map(|r| json!({ "key": role_key(r), "name": cap(r.name()) })).collect();
    let support: Vec<Value> = staffing::drawn_staff(w, team)
        .into_iter()
        .map(|d| json!({ "name": d.name, "title": d.title, "age": d.age, "since": d.since, "subject": d.subject, "pay": money(d.pay) }))
        .collect();
    let owed: f64 = w.folk.staff.owed.iter().filter(|x| x.0 >= year).map(|x| x.1).sum();
    json!({
        "team": team_json(w, team),
        "level": level_key(team.level()),
        "rows": rows,
        "openings": openings,
        "support": support,
        "owed": if owed > 0.0 { Some(money(owed)) } else { None },
        "pool": w.folk.staff.pool_bonus,
        "hours": talks::hours_left(w),
    })
}

fn candidates(w: &World, r: Role) -> Value {
    let rows: Vec<Value> = staffing::candidates(w, r)
        .into_iter()
        .take(40)
        .map(|c| {
            json!({
                "coach": c.coach,
                "link": folk::coach_link(w, c.coach),
                "age": c.age,
                "now": c.now,
                "rep": c.rep,
                "cameUp": c.came_up,
                "known": c.known,
                "read": [c.read.0.round(), c.read.1.round()],
                "asks": money(c.asks),
                "asksValue": c.asks,
                "blocked": c.blocked,
                "interviewed": w.folk.staff.interviewed.iter().any(|x| x.0 == c.coach),
            })
        })
        .collect();
    json!({ "role": role_key(r), "roleName": cap(r.name()), "rows": rows })
}

/// Your coaching network: every contact with how you know him, how warm,
/// the favours between you, and what people have said about you.
fn contacts(w: &World) -> Value {
    let today = w.today;
    let mut list: Vec<&network::Contact> = w.folk.network.contacts.iter().collect();
    list.sort_by(|a, b| b.warmth.total_cmp(&a.warmth));
    let rows: Vec<Value> = list
        .iter()
        .filter(|c| c.coach < w.coaches.len())
        .map(|c| {
            let co = &w.coaches[c.coach];
            json!({
                "coach": c.coach,
                "link": folk::coach_link(w, c.coach),
                "now": facts::team_of_coach(w, c.coach).map_or_else(|| if co.retired { "Retired".to_string() } else { "Out of work".to_string() }, |t| format!("{}, {}", cap(&facts::coach_role(w, c.coach)), t.link(w))),
                "how": c.how,
                "warmth": c.warmth,
                "warmthWords": warmth_words(c.warmth),
                "last": c.last.map(date_text),
                "recent": c.recent.iter().filter(|d| d.days_to(today) < 30).count(),
                "balance": c.balance,
            })
        })
        .collect();
    let ledger: Vec<Value> = w
        .folk
        .network
        .ledger
        .iter()
        .rev()
        .take(30)
        .filter(|f| f.coach < w.coaches.len())
        .map(|f| json!({ "date": date_text(f.date), "who": folk::coach_link(w, f.coach), "words": f.words, "asked": f.asked }))
        .collect();
    let interest: Vec<Value> = w
        .folk
        .network
        .interest
        .iter()
        .rev()
        .take(12)
        .map(|(y, t, words)| json!({ "year": y, "team": t.link(w), "words": words }))
        .collect();
    json!({
        "rows": rows,
        "ledger": ledger,
        "interest": interest,
        "clinics": w.folk.network.clinics,
        "allstar": w.folk.network.allstar.iter().rev().map(|(y, g)| json!({ "year": y, "game": g })).collect::<Vec<_>>(),
        "hours": talks::hours_left(w),
    })
}

/// League discipline (P2): the season's fines, ejections and suspensions,
/// and the year's counts.
fn league(w: &World) -> Value {
    let book = &w.folk.discipline;
    let recent: Vec<Value> = book
        .recent
        .iter()
        .rev()
        .take(200)
        .map(|f| {
            json!({
                "date": date_text(f.date),
                "who": folk::person(w, f.id).map_or_else(|| "A former player".to_string(), |i| folk::player_link(w, i)),
                "level": level_key(f.level),
                "what": f.what,
                "amount": if f.amount > 0 { Some(format!("${}", facts::thousands(f.amount))) } else { None },
                "games": f.games,
                "appealed": f.appealed,
                "reduced": f.reduced,
            })
        })
        .collect();
    let years: Vec<Value> = book
        .years
        .iter()
        .rev()
        .map(|y| json!({ "year": y.0, "nflFines": y.1, "nflSuspensions": y.2, "targeting": y.3, "hsEjections": y.4 }))
        .collect();
    let cases: Vec<Value> = w
        .folk
        .courts
        .open
        .iter()
        .chain(w.folk.courts.closed.iter().rev())
        .take(80)
        .map(|c| {
            json!({
                "who": folk::person(w, c.id).map_or_else(|| "A former player".to_string(), |i| folk::player_link(w, i)),
                "what": c.what,
                "opened": date_text(c.opened),
                "step": c.step.words(),
                "outcome": c.outcome,
            })
        })
        .collect();
    json!({ "recent": recent, "years": years, "cases": cases })
}
