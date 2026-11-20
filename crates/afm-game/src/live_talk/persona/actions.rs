//! Talks as a shortcut (talk replies item 13, agreed by Cain 2026-11-19:
//! "not just for this, but for all possible actions. Live talks needs to
//! devlop into a sophisticated short cut"). Anything you say you'll do, or
//! ask your staff to do, that a screen can do runs from the talk after one
//! confirm card, through the screen's own command (`Game::run`), so it is
//! worth exactly what the button is. If the screen's rule says it can't run
//! (no slot left, the wrong time of year, not yours to do), he hears why and
//! your staff's note under his line says so.
//!
//! The ear names an action with the act `do` and its key (`ROWS`); the card
//! lists, for each player in the room, the actions that fit him today with
//! the values they take (`card_lines`). A player's moves are the same list
//! the screens' menus draw (`lane_core::moves::player`); his Development
//! tab's projects, focus and position move, and the depth chart, are read
//! by those screens' own checks (`projects::skill_check`, `body_check`).
//!
//! Your word kept: said to him about his own work (a project, a focus, a
//! move) and turned down on the card ("Not that"), it stays your word, a
//! promise he holds you to (`PromiseKind::Work`, judged in
//! `talks::WORK_DAYS`); done on the card or later on the screen, it counts
//! as kept. Sizes and wording are proposed, not confirmed.

use afm_engine::attributes::Attr;
use afm_engine::craft::{Craft, Point};
use afm_engine::positions::Position;
use afm_engine::progression as pg;
use afm_engine::world::World;
use afm_engine::world::folk::persona::Them;
use afm_engine::world::folk::persona::ear::{ActKind, Action, Confirm, Turn};
use afm_engine::world::folk::persona::terms::{Decision, Request};
use afm_engine::world::folk::{self as folk, talks};
use afm_engine::world::front_office::{Duty, Owner, redshirt, reserve, walk_ons};
use serde_json::{Value, json};

use super::{EarKeys, EarReply};
use crate::Game;
use crate::lane_training::projects;

/// Every action the ear may name with `do`, and what it means.
pub const ROWS: [(&str, &str); 20] = [
    ("skill_project", "start a skill project for a player of the coach's (value: the skill, from the actions list)"),
    ("end_skill_project", "end a player's skill project"),
    ("body_project", "start a body project for a player (value: strength or speed)"),
    ("end_body_project", "end a player's body project"),
    ("focus", "make one skill or technique a player's focus in practice (value: its name from the actions list)"),
    ("move_position", "move a player to a new position in practice (value: the position's short name)"),
    ("depth", "put a player at a place on the depth chart at his position (value: 1 for the starter, 2, 3...)"),
    ("redshirt", "redshirt a player this season"),
    ("play", "take a player's redshirt off and play him"),
    ("reserve", "put a hurt player on injured reserve"),
    ("reserve_back", "bring a player back from injured reserve"),
    ("scholarship", "put a walk-on on scholarship"),
    ("block", "put a pro on the trading block"),
    ("off_block", "take a pro off the trading block"),
    ("squad", "move a high school player to a squad (value: varsity, jv or freshman)"),
    ("my_players", "add a player to the coach's own watch list"),
    ("board", "add a recruit to the recruiting board"),
    ("unboard", "take a recruit off the recruiting board"),
    ("pull_offer", "pull a scholarship offer from a recruit"),
    (
        "duty",
        "hand one of the coach's recruiting desk duties to his staff, or take it back (value: board, visits or portal, then staff or you)",
    ),
];

/// The action keys, for the ear's form.
pub fn keys() -> Vec<&'static str> {
    ROWS.iter().map(|r| r.0).collect()
}

/// What the ear's rules say each action means.
pub fn meanings() -> String {
    ROWS.iter().map(|(k, m)| format!("{k} ({m})")).collect::<Vec<_>>().join("; ")
}

/// The man the action is done to, as the ear tagged him.
fn tagged(keys: &EarKeys, tag: Option<&str>) -> Option<Them> {
    let tag = tag.filter(|t| !t.is_empty()).unwrap_or("p1");
    keys.people.iter().find(|(t, _)| t == tag).map(|x| x.1)
}

fn player_id(w: &World, keys: &EarKeys, tag: Option<&str>) -> Option<(u32, usize)> {
    match tagged(keys, tag)? {
        Them::Player(id) => folk::person(w, id).map(|i| (id, i)),
        _ => None,
    }
}

fn first(name: &str) -> String {
    folk::plain(name).split_whitespace().last().unwrap_or("him").to_string()
}

/// The skill a project's words name, among his position's.
fn skill_of(p: &afm_engine::player::Player, words: &str) -> Option<Attr> {
    let v = norm(words);
    let skills = pg::project_skills(p);
    skills
        .iter()
        .copied()
        .find(|a| norm(a.key()) == v || norm(a.label()) == v)
        .or_else(|| skills.iter().copied().find(|a| !v.is_empty() && norm(a.label()).contains(&v)))
        .or_else(|| skills.iter().copied().find(|a| !v.is_empty() && v.contains(&norm(a.label()))))
}

fn norm(s: &str) -> String {
    s.to_lowercase().replace(['_', '-'], " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A focus's words as a point of emphasis: a flaw or tool your staff has
/// named on him, any piece of technique at his position by its name, else
/// a rating.
fn point_of(w: &World, id: u32, pos: Position, words: &str) -> Option<Point> {
    let v = norm(words);
    if v.is_empty() {
        return None;
    }
    let seen: Vec<Craft> = w.seen_of(id).into_iter().map(|x| x.0).collect();
    let by_name = |c: &Craft| norm(c.name()) == v || norm(c.name()).contains(&v) || v.contains(&norm(c.name()));
    seen.iter()
        .copied()
        .find(by_name)
        .or_else(|| Craft::ALL.iter().copied().filter(|c| c.fits(pos)).find(|c| by_name(c)))
        .map(Point::Craft)
        .or_else(|| Attr::ALL.iter().copied().find(|a| norm(a.label()) == v || norm(a.key()) == v).map(Point::Rating))
}

/// The position a coach's words name, as the ear reads one.
fn position_of(words: &str) -> Option<Position> {
    super::ear::position(words).and_then(|k| Position::ALL.get(k as usize).copied())
}

/// A move of his from the screens' menus (`lane_core::moves::player`) that
/// fits this action, with the command that runs it (the screens'
/// `runMove`).
fn from_moves(w: &World, id: u32, key: &str, value: &str) -> Option<(String, Value)> {
    let moves = crate::lane_core::moves::player(w, id);
    let find = |k: &str, on: Option<bool>| {
        moves.iter().find(|m| m["k"] == k && on.is_none_or(|o| m["on"].as_bool() == Some(o))).cloned()
    };
    let (cmd, args) = match key {
        "redshirt" => find("redshirt", Some(true)).map(|_| ("redshirt_set", json!({ "id": id, "on": true })))?,
        "play" => find("redshirt", Some(false)).map(|_| ("redshirt_set", json!({ "id": id, "on": false })))?,
        "reserve" => find("ir", None).map(|_| ("ir_place", json!({ "id": id })))?,
        "reserve_back" => {
            find("ir_back", None).filter(|m| m["why"].is_null()).map(|_| ("ir_back", json!({ "id": id })))?
        }
        "scholarship" => find("scholarship", None).map(|_| ("college_scholarship", json!({ "id": id })))?,
        "block" => find("block", Some(true)).map(|_| ("fo_block", json!({ "id": id, "on": true })))?,
        "off_block" => find("block", Some(false)).map(|_| ("fo_block", json!({ "id": id, "on": false })))?,
        "squad" => {
            let v = norm(value).replace(' ', "");
            moves
                .iter()
                .find(|m| m["k"] == "squad" && m["squad"].as_str().is_some_and(|s| v.is_empty() || norm(s) == v))
                .map(|m| ("set_squad", json!({ "id": id, "squad": m["squad"] })))?
        }
        "my_players" => find("list_add", None).map(|_| ("list_add", json!({ "id": id })))?,
        "board" => find("board", None).map(|_| ("recruit_add", json!({ "id": id })))?,
        "unboard" => find("unboard", None).map(|_| ("recruit_remove", json!({ "id": id })))?,
        "pull_offer" => find("pull", None).map(|_| ("recruit_pull", json!({ "id": id })))?,
        _ => return None,
    };
    Some((cmd.to_string(), args))
}

/// Why a move of the screens' menus isn't open for him today, from the
/// same rule the menu reads.
fn why_not(w: &World, id: u32, key: &str) -> String {
    let head = || (!w.seat_head()).then(|| "That's the head coach's call, not yours.".to_string());
    let menu_moves = crate::lane_core::moves::player(w, id);
    let has = |k: &str| menu_moves.iter().any(|m| m["k"] == k);
    match key {
        "redshirt" | "play" => head().unwrap_or_else(|| match redshirt::read(w, id) {
            Err(e) => e,
            Ok(r) => match r.can {
                Err(e) => e,
                Ok(()) if key == "redshirt" => "He's already redshirting this season.".into(),
                Ok(()) => "He isn't redshirting.".into(),
            },
        }),
        "reserve" => reserve::place_words(w, id).err().unwrap_or_else(|| "He can't go on injured reserve now.".into()),
        "reserve_back" => menu_moves
            .iter()
            .find(|m| m["k"] == "ir_back")
            .and_then(|m| m["why"].as_str().map(str::to_string))
            .or_else(|| reserve::activate_words(w, id).err())
            .unwrap_or_else(|| "He isn't on injured reserve.".into()),
        "scholarship" => head()
            .or_else(|| walk_ons::scholarship_words(w, id).err())
            .unwrap_or_else(|| "He's already on scholarship.".into()),
        "block" | "off_block" => head().unwrap_or_else(|| {
            if has("block") {
                if key == "block" { "He's already on the block." } else { "He isn't on the block." }.into()
            } else {
                "Only a pro on your club can go on the trading block.".into()
            }
        }),
        "squad" => "He can't move to that squad.".into(),
        "my_players" => "He's already on your list.".into(),
        "board" | "unboard" | "pull_offer" => head().unwrap_or_else(|| {
            if w.front_office.owner(Duty::Board) != Owner::You {
                "Your recruiting coordinator runs the board: take it back first.".into()
            } else if key == "board" {
                "He's already on your board, or isn't a recruit you can add.".into()
            } else if key == "unboard" {
                "He isn't on your board, or you've offered him: pull the offer first.".into()
            } else {
                "You haven't offered him.".into()
            }
        }),
        _ => "That isn't something you can do from your seat today.".into(),
    }
}

/// Whether he is yours to plan, as the Development tab asks.
fn plannable(w: &World, id: u32, i: usize) -> Result<(), String> {
    let t = crate::views::mine(w).ok_or("You have no team.")?;
    if !crate::views::roster_people(w, t).contains(&i) {
        return Err("He isn't on your team.".into());
    }
    if !w.may_plan(id) {
        return Err(format!("His plan is the head coach's. Your job covers {}.", w.your_controls()));
    }
    Ok(())
}

/// The name of the man whose project a new one replaces.
fn name_of(w: &World, id: u32) -> String {
    folk::person(w, id).map_or_else(|| "another man".into(), |i| first(&w.people[i].name))
}

/// One action said, read against the screens' rules: the confirm card's
/// action, or why it can't run.
pub fn check(w: &World, key: &str, about: Option<(u32, usize)>, value: &str, across: bool) -> Result<Action, String> {
    let word = |code: u8| across.then_some(code);
    if key == "duty" {
        return duty(w, value);
    }
    let Some((id, i)) = about else { return Err("I couldn't tell who that was about.".into()) };
    let name = first(&w.people[i].name);
    let p = &w.people[i].player;
    let one = |cmd: &str, args: Value| vec![(cmd.to_string(), args)];
    let act = |card: String, steps: Vec<(String, Value)>, word: Option<u8>| Action {
        key: key.into(),
        about: Some(id),
        card,
        steps,
        word,
    };
    match key {
        "skill_project" => {
            let a = skill_of(p, value)
                .ok_or_else(|| format!("That isn't a skill a {} builds a project on.", p.position.name()))?;
            let start = one("player_dev_project_start", json!({ "id": id, "what": a.key() }));
            let label = a.label().to_lowercase();
            match projects::skill_check(w, id, a.key())? {
                None => Ok(act(format!("Start a {label} project for {name}?"), start, word(0))),
                Some(other) => {
                    let room = pg::Room::of(p.position).map_or("his", projects::room_label);
                    let mut steps = one("player_dev_project_end", json!({ "id": other }));
                    steps.extend(start);
                    Ok(act(
                        format!(
                            "Start a {label} project for {name}? Every {room} project spot is taken: this replaces {}'s.",
                            name_of(w, other)
                        ),
                        steps,
                        word(0),
                    ))
                }
            }
        }
        "end_skill_project" => {
            plannable(w, id, i)?;
            let a = p.prog.project().ok_or("He has no skill project.")?;
            Ok(act(
                format!("End {name}'s {} project?", a.label().to_lowercase()),
                one("player_dev_project_end", json!({ "id": id })),
                None,
            ))
        }
        "body_project" => {
            let v = norm(value);
            let kind =
                if v.contains("speed") || v.contains("fast") || v.contains("quick") { "speed" } else { "strength" };
            let label = projects::kind_label(projects::kind_of(kind).unwrap_or(pg::BodyKind::Strength)).to_lowercase();
            let start = one("player_dev_body_start", json!({ "id": id, "what": kind }));
            match projects::body_check(w, id, kind)? {
                None => Ok(act(format!("Start a {label} for {name}?"), start, word(1))),
                Some(other) => {
                    let mut steps = one("player_dev_body_end", json!({ "id": other }));
                    steps.extend(start);
                    Ok(act(
                        format!(
                            "Start a {label} for {name}? Your strength staff's spot in his room is taken: this replaces {}'s.",
                            name_of(w, other)
                        ),
                        steps,
                        word(1),
                    ))
                }
            }
        }
        "end_body_project" => {
            plannable(w, id, i)?;
            p.prog.body_project().ok_or("He has no body project.")?;
            Ok(act(format!("End {name}'s body project?"), one("player_dev_body_end", json!({ "id": id })), None))
        }
        "focus" => {
            plannable(w, id, i)?;
            let pt = point_of(w, id, p.position, value)
                .ok_or("That isn't a skill or a piece of technique at his position.")?;
            let now = w.points_of(id);
            let mut codes = vec![pt.code()];
            // The new focus first; his other one stays beside it (two at most).
            if let Some(o) = now.iter().find(|x| **x != pt) {
                codes.push(o.code());
            }
            let label = crate::lane_training::craft::point_label(pt).to_lowercase();
            Ok(act(
                format!("Set {name}'s practice focus on {label}?"),
                one("practice_points", json!({ "id": id, "points": codes })),
                word(2),
            ))
        }
        "move_position" => {
            plannable(w, id, i)?;
            let to = position_of(value).ok_or("Which position? I couldn't tell.")?;
            if to == p.position {
                return Err(format!("He already plays {}.", to.name()));
            }
            let k = Position::ALL.iter().position(|x| *x == to).unwrap_or(0) as u8;
            Ok(act(
                format!("Move {name} to {} in practice? He learns it over the weeks.", to.name()),
                one("practice_plan", json!({ "id": id, "moving": to.name() })),
                word(3 + k),
            ))
        }
        "depth" => {
            let n: usize =
                value.split_whitespace().find_map(|x| x.trim_end_matches(['.', ',']).parse().ok()).unwrap_or(1);
            let pos = p.position;
            let mut order = w
                .seat_game_depth()
                .and_then(|d| d.into_iter().find(|(x, _)| *x == pos).map(|(_, v)| v))
                .ok_or("He isn't on your depth chart.")?;
            if !order.contains(&id) {
                return Err("He isn't on your depth chart.".into());
            }
            let at = order.iter().position(|&x| x == id).unwrap_or(0);
            let to = n.clamp(1, order.len()) - 1;
            if at == to {
                return Err(format!("He's already No. {} at {}.", to + 1, pos.name()));
            }
            order.retain(|&x| x != id);
            order.insert(to, id);
            Ok(act(
                format!("Put {name} No. {} at {} on your depth chart?", to + 1, pos.name()),
                one("set_depth", json!({ "pos": pos.name(), "ids": order })),
                None,
            ))
        }
        _ => {
            let (cmd, args) = from_moves(w, id, key, value).ok_or_else(|| why_not(w, id, key))?;
            let card = match key {
                "redshirt" => format!("Redshirt {name} this season?"),
                "play" => format!("Take {name}'s redshirt off and play him?"),
                "reserve" => format!("Put {name} on injured reserve?"),
                "reserve_back" => format!("Bring {name} back from injured reserve?"),
                "scholarship" => format!("Put {name} on scholarship?"),
                "block" => format!("Put {name} on the trading block?"),
                "off_block" => format!("Take {name} off the trading block?"),
                "squad" => format!("Move {name} to {}?", args["squad"].as_str().unwrap_or("that squad")),
                "my_players" => format!("Add {name} to your players?"),
                "board" => format!("Add {name} to your recruiting board?"),
                "unboard" => format!("Take {name} off your recruiting board?"),
                "pull_offer" => format!("Pull {name}'s scholarship offer?"),
                _ => format!("Do it for {name}?"),
            };
            Ok(act(card, vec![(cmd, args)], None))
        }
    }
}

/// A recruiting desk duty handed to your staff or taken back.
fn duty(w: &World, value: &str) -> Result<Action, String> {
    if !w.seat_head() {
        return Err("The desk's duties are the head coach's to hand out.".into());
    }
    if !matches!(crate::views::mine(w), Some(crate::views::TeamRef::College(_))) {
        return Err("Only a college desk has those duties.".into());
    }
    let v = norm(value);
    let d = [("board", Duty::Board), ("visit", Duty::Visits), ("portal", Duty::Portal)]
        .into_iter()
        .find(|(k, _)| v.contains(k))
        .map(|x| x.1)
        .ok_or("Which duty: the board, visits or the portal?")?;
    let yours = v.contains("you") || v.contains("back") || v.contains("me");
    let owner = if yours { Owner::You } else { Owner::Staff };
    if w.front_office.owner(d) == owner {
        return Err(if yours { "You already run it." } else { "Your staff already runs it." }.into());
    }
    let what = match d {
        Duty::Board => "the recruiting board",
        Duty::Visits => "visits",
        _ => "the portal",
    };
    Ok(Action {
        key: "duty".into(),
        about: None,
        card: if yours { format!("Take {what} back from your staff?") } else { format!("Hand {what} to your staff?") },
        steps: vec![("college_duty".into(), json!({ "duty": d.key(), "owner": if yours { "you" } else { "staff" } }))],
        word: None,
    })
}

/// What the line's `do` acts come to: each one that can run waits on your
/// confirm with its card; each one that can't is said to him and noted
/// under his line. The man across hears what you're doing.
pub fn said(w: &World, er: &EarReply, keys: &EarKeys, t: &mut Turn, notes: &mut Vec<String>) {
    let across = keys.people.first().map(|x| x.1);
    for a in er.acts.iter().filter(|a| a.act == ActKind::Do) {
        let key = keyed(a.what.trim(), &a.words, &a.value);
        if !ROWS.iter().any(|r| r.0 == key) {
            continue;
        }
        let about = player_id(w, keys, a.about.as_deref());
        let on_him = about.is_some_and(|(id, _)| across == Some(Them::Player(id)));
        match check(w, key, about, &a.value, on_him) {
            Ok(x) => {
                if t.confirm.iter().any(|c| matches!(c, Confirm::Action(y) if y.steps == x.steps)) {
                    continue;
                }
                let what = card_short(&x.card);
                t.say.heard.push(format!("The coach is doing it: {what}. It's done once he gives the word."));
                t.settled.push(format!("open: {what}"));
                t.confirm.push(Confirm::Action(x));
            }
            Err(why) => {
                t.say.released.push(format!("The game can't do that today: {why}"));
                notes.push(format!("Can't do that now: {why}"));
                t.settled.push(format!("couldn't: {}", folk::plain(&a.words)));
            }
        }
    }
}

/// The action's key, or the one its words name when the ear left it out
/// (a replay: "I'm putting you on a release project" came with the skill
/// and no key).
fn keyed<'a>(what: &'a str, words: &str, value: &str) -> &'a str {
    if !what.is_empty() {
        return what;
    }
    let w = norm(words);
    let v = norm(value);
    if w.contains("project") {
        if ["strength", "speed", "body", "weight room"].iter().any(|x| w.contains(x) || v.contains(x)) {
            "body_project"
        } else {
            "skill_project"
        }
    } else if w.contains("focus") {
        "focus"
    } else {
        ""
    }
}

/// A card's question as a plain phrase ("start a release project for
/// Lampkins").
fn card_short(card: &str) -> String {
    let q = card.split('?').next().unwrap_or(card).trim();
    let mut c = q.chars();
    c.next().map_or_else(String::new, |f| f.to_lowercase().collect::<String>() + c.as_str())
}

/// An ask he agreed to that a screen makes real (talk replies item 13: the
/// catch-all "He agreed to ..." ran nothing): his position move on his
/// practice plan, the weight he's to make, a recruit your assistant is to
/// go after on your board.
pub fn of_request(w: &World, him: Them, r: &Request) -> Option<Action> {
    match (him, r) {
        (Them::Player(id), Request::Position(k)) => {
            let i = folk::person(w, id)?;
            let to = Position::ALL.get(*k as usize)?;
            check(w, "move_position", Some((id, i)), to.name(), false).ok()
        }
        (Them::Player(id), Request::Weight(lbs)) => {
            let i = folk::person(w, id)?;
            plannable(w, id, i).ok()?;
            let mode = w.training.plan_of(id).body.map_or(0, |b| b.1);
            let lbs = (*lbs as u16).clamp(150, 380);
            Some(Action {
                key: "weight".into(),
                about: Some(id),
                card: format!("Set {}'s weight target at {lbs} pounds?", first(&w.people[i].name)),
                steps: vec![("practice_plan".into(), json!({ "id": id, "body": [lbs, mode] }))],
                word: None,
            })
        }
        (Them::Coach(_), Request::Target(rid)) => {
            let i = folk::person(w, *rid)?;
            check(w, "board", Some((*rid, i)), "", false).ok()
        }
        _ => None,
    }
}

/// A decision you told him that a screen makes real (talk replies item
/// 13): a position move on his practice plan, a redshirt.
pub fn of_decision(w: &World, him: Them, d: Decision) -> Option<Action> {
    let Them::Player(id) = him else { return None };
    let i = folk::person(w, id)?;
    match d {
        Decision::PositionMoved(k) => {
            let to = Position::ALL.get(k as usize)?;
            check(w, "move_position", Some((id, i)), to.name(), false).ok()
        }
        Decision::Redshirt => check(w, "redshirt", Some((id, i)), "", false).ok(),
        _ => None,
    }
}

/// Runs a confirmed action's steps through the screens' own commands, in
/// order; the first one a screen refuses stops it and says why.
pub fn run(g: &mut Game, a: &Action) -> Vec<String> {
    let mut out = Vec::new();
    for (cmd, args) in &a.steps {
        match g.run(cmd, args) {
            Ok(v) => {
                for k in ["said", "note", "text", "refused"] {
                    if let Some(s) = v.get(k).and_then(Value::as_str).filter(|s| !s.is_empty()) {
                        out.push(s.to_string());
                    }
                }
            }
            Err(e) => {
                out.push(format!("It couldn't be done: {e}"));
                return out;
            }
        }
    }
    out.insert(0, format!("Done: {}.", card_short(&a.card)));
    out
}

/// A "Not that" on an action you said you'd do for him: your word stands,
/// a promise he holds you to.
pub fn dropped(w: &mut World, a: &Action) -> Option<String> {
    let (code, id) = (a.word?, a.about?);
    talks::promise(w, talks::PromiseTo::Player(id), talks::PromiseKind::Work(code), 1);
    Some(format!("Your word to him stands: {}. He'll hold you to it.", talks::work_words(code)))
}

/// The ear card's list of actions for the people in the room: for each
/// player, the actions that fit him today with the values they take.
pub fn card_lines(w: &World, keys: &EarKeys) -> Vec<String> {
    let mut out = Vec::new();
    for (tag, them) in &keys.people {
        let Them::Player(id) = *them else { continue };
        let Some(i) = folk::person(w, id) else { continue };
        let p = &w.people[i].player;
        let mut can: Vec<String> = Vec::new();
        if plannable(w, id, i).is_ok() {
            let skills: Vec<String> = pg::project_skills(p).iter().map(|a| a.label().to_lowercase()).collect();
            if !skills.is_empty() {
                can.push(format!("skill_project ({})", skills.join(", ")));
            }
            if p.prog.project().is_some() {
                can.push("end_skill_project".into());
            }
            can.push("body_project (strength or speed)".into());
            if p.prog.body_project().is_some() {
                can.push("end_body_project".into());
            }
            let seen: Vec<String> = w.seen_of(id).iter().map(|x| x.0.name().to_lowercase()).take(6).collect();
            can.push(if seen.is_empty() {
                "focus (a skill by name)".into()
            } else {
                format!("focus ({}, or a skill by name)", seen.join(", "))
            });
            can.push("move_position".into());
        }
        if w.seat_game_depth().is_some_and(|d| d.iter().any(|(_, v)| v.contains(&id))) {
            can.push("depth".into());
        }
        for key in [
            "redshirt",
            "play",
            "reserve",
            "reserve_back",
            "scholarship",
            "block",
            "off_block",
            "squad",
            "my_players",
            "board",
            "unboard",
            "pull_offer",
        ] {
            if from_moves(w, id, key, "").is_some() {
                can.push(key.into());
            }
        }
        if !can.is_empty() {
            out.push(format!("{tag}: {}", can.join("; ")));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_row_is_a_word_the_ear_may_name() {
        let names = super::super::ear::what_names();
        for k in super::keys() {
            assert!(names.iter().any(|n| n == k), "{k}");
        }
    }
}
