//! Your desk's commands: the inbox list and letters (10a-10i), your notes
//! on anything (10g), "my players" (S5) and saved searches (watches).

use afm_engine::calendar::Date;
use afm_engine::world::desk::{DeskNote, Head, Listed, Mail};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use super::links;
use super::mail::{self, KEYWORDS};
use crate::reads;
use crate::views;

fn head_name(h: Head) -> &'static str {
    match h {
        Head::School => "school",
        Head::Conference => "conference",
        Head::League => "league",
        Head::Club => "club",
        Head::Agency => "agency",
        Head::Paper => "paper",
        Head::Email => "email",
        Head::Memo => "memo",
    }
}

fn keys_of(k: u16) -> Vec<&'static str> {
    KEYWORDS.iter().enumerate().filter(|(i, _)| k & (1 << i) != 0).map(|(_, n)| *n).collect()
}

/// A day's heading in the list: Today, Yesterday, or the date.
fn day_heading(today: Date, d: Date) -> String {
    match d.days_to(today) {
        0 => "Today".into(),
        1 => "Yesterday".into(),
        n if n < 0 => views::date_text(d),
        _ => views::date_text(d),
    }
}

fn clock(minute: u16) -> String {
    let (h, m) = (minute / 60, minute % 60);
    let (h12, ap) = match h {
        0 => (12, "am"),
        1..=11 => (h, "am"),
        12 => (12, "pm"),
        _ => (h - 12, "pm"),
    };
    format!("{h12}:{m:02} {ap}")
}

fn preview(m: &Mail) -> String {
    let first = m.body.iter().find(|l| l.len() > 30 && !l.ends_with(',')).or(m.body.first());
    let s = first.map(|l| links::strip(l)).unwrap_or_default();
    if s.chars().count() > 140 { format!("{}…", s.chars().take(138).collect::<String>()) } else { s }
}

fn matches(m: &Mail, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    let q = q.to_lowercase();
    let hay = |s: &str| links::strip(s).to_lowercase().contains(&q);
    hay(&m.subject)
        || hay(&m.from)
        || hay(&m.org)
        || hay(&m.role)
        || m.body.iter().any(|l| hay(l))
        || m.table.iter().flatten().any(|c| hay(c))
        || m.attach.iter().any(|a| hay(a))
}

fn clears_in(today: Date, m: &Mail) -> Option<i32> {
    if m.saved {
        return None;
    }
    let left = mail::KEEP + 1 - m.date.days_to(today);
    (left <= 10).then_some(left.max(0))
}

fn row(today: Date, m: &Mail) -> Value {
    json!({
        "id": m.id.to_string(),
        "day": day_heading(today, m.date),
        "time": clock(m.minute),
        "head": head_name(m.head),
        "from": m.from,
        "org": m.org,
        "subject": m.subject,
        "preview": preview(m),
        "unread": !m.read,
        "saved": m.saved,
        "answers": m.answers.len(),
        "answered": m.answered,
        "answer_by": m.answer_by.filter(|_| m.answered.is_none() && !m.answers.is_empty()).map(views::date_text),
        "clears_in": clears_in(today, m),
        "keys": keys_of(m.keys),
        "loud": m.loud,
    })
}

/// The inbox list (10b, 10d, 10e, 10f): newest first under day headings.
pub fn list(w: &World, args: &Value) -> Value {
    let Some(c) = &w.career else { return json!({ "rows": [], "unread": 0 }) };
    let show = args.get("show").and_then(Value::as_str).unwrap_or("all");
    let key = args.get("key").and_then(Value::as_str).unwrap_or("");
    let bit = KEYWORDS.iter().position(|k| *k == key).map(|i| 1u16 << i);
    let q = args.get("q").and_then(Value::as_str).unwrap_or("").trim();
    let saved = args.get("saved").and_then(Value::as_bool).unwrap_or(false);
    let today = w.today;
    let mails = &c.desk.mails;
    let mut counts = vec![0u32; KEYWORDS.len()];
    for m in mails {
        for (i, n) in counts.iter_mut().enumerate() {
            if m.keys & (1 << i) != 0 {
                *n += 1;
            }
        }
    }
    let rows: Vec<Value> = mails
        .iter()
        .rev()
        .filter(|m| match show {
            "read" => m.read,
            "unread" => !m.read,
            _ => true,
        })
        .filter(|m| bit.is_none_or(|b| m.keys & b != 0))
        .filter(|m| !saved || m.saved)
        .filter(|m| matches(m, q))
        .map(|m| row(today, m))
        .collect();
    json!({
        "rows": rows,
        "total": mails.len(),
        "unread": c.desk.unread(),
        "saved": mails.iter().filter(|m| m.saved).count(),
        "keywords": KEYWORDS.iter().zip(counts).map(|(k, n)| json!({ "name": k, "count": n })).collect::<Vec<_>>(),
        "notes": notes_count(w),
    })
}

fn about_name(w: &World, about: Option<(u8, u32)>) -> Option<Value> {
    let (kind, id) = about?;
    Some(match kind {
        mail::ABOUT_PLAYER => {
            let k = views::person_index(w, id)?;
            json!({ "kind": "player", "id": id, "name": w.people[k].name })
        }
        mail::ABOUT_COACH => json!({ "kind": "coach", "id": id, "name": w.coaches.get(id as usize)?.name }),
        mail::ABOUT_TEAM => {
            let t = mail::team_from(id);
            json!({ "kind": "team", "ref": views::team_brief(w, t)["ref"], "name": views::team_name(w, t) })
        }
        mail::ABOUT_GAME => json!({ "kind": "game", "index": id, "name": "The game page" }),
        _ => return None,
    })
}

/// One letter, whole (10a, 10c); opening it marks it read.
pub fn open(w: &mut World, id: u64) -> Result<Value, String> {
    let today = w.today;
    let c = w.career.as_mut().ok_or("No career.")?;
    let m = c.desk.mails.iter_mut().find(|m| m.id == id).ok_or("That mail has cleared.")?;
    m.read = true;
    let m = m.clone();
    let colors = m.org_team.map(|t| views::team_colors(w, mail::team_from(t)));
    let notes: Vec<Value> = w
        .career
        .as_ref()
        .map(|c| {
            c.desk
                .notes
                .iter()
                .filter(|n| n.page.ends_with(&m.subject))
                .map(|n| json!({ "date": views::date_text(n.date), "text": n.text }))
                .collect()
        })
        .unwrap_or_default();
    Ok(json!({
        "id": m.id.to_string(),
        "date": views::date_text(m.date),
        "pretty": m.date.pretty(),
        "time": clock(m.minute),
        "head": head_name(m.head),
        "org": m.org,
        "org_line": m.org_line,
        "colors": colors,
        "from": m.from,
        "role": m.role,
        "to": m.to,
        "subject": m.subject,
        "body": m.body,
        "table": m.table,
        "attach": m.attach,
        "sign": m.sign,
        "keys": keys_of(m.keys),
        "about": about_name(w, m.about),
        "answers": m.answers.iter().map(|a| json!({ "label": a.label, "does": a.does })).collect::<Vec<_>>(),
        "answered": m.answered,
        "answer_by": m.answer_by.map(views::date_text),
        "saved": m.saved,
        "clears_in": clears_in(today, &m),
        "notes": notes,
        "unread": w.career.as_ref().map_or(0, |c| c.desk.unread()),
    }))
}

/// Read, unread, saved or deleted (10b, 10f).
pub fn set(w: &mut World, args: &Value) -> Result<Value, String> {
    let today = w.today;
    let c = w.career.as_mut().ok_or("No career.")?;
    let ids: Vec<u64> = match args.get("ids").and_then(Value::as_array) {
        Some(a) => a.iter().filter_map(|v| v.as_str().and_then(|s| s.parse().ok())).collect(),
        None => args.get("id").and_then(Value::as_str).and_then(|s| s.parse().ok()).into_iter().collect(),
    };
    let all = args.get("all").and_then(Value::as_bool).unwrap_or(false);
    for m in c.desk.mails.iter_mut().filter(|m| all || ids.contains(&m.id)) {
        if let Some(r) = args.get("read").and_then(Value::as_bool) {
            m.read = r;
        }
        if let Some(s) = args.get("saved").and_then(Value::as_bool) {
            m.saved = s;
        }
    }
    if args.get("delete").and_then(Value::as_bool).unwrap_or(false) {
        let gone: Vec<(u64, Date)> = c.desk.mails.iter().filter(|m| ids.contains(&m.id)).map(|m| (m.id, m.date)).collect();
        c.desk.mails.retain(|m| !ids.contains(&m.id));
        c.desk.gone.extend(gone);
    }
    c.desk.clear_old(today);
    Ok(json!({ "unread": c.desk.unread() }))
}

// ---- Notes (10g) ----

fn notes_count(w: &World) -> usize {
    w.career.as_ref().map_or(0, |c| c.desk.notes.len() + c.notes.iter().map(|(_, l)| l.len()).sum::<usize>())
}

/// Every note you wrote, newest first, with the page it was written on.
pub fn notes(w: &World) -> Value {
    let Some(c) = &w.career else { return json!({ "notes": [] }) };
    let mut all: Vec<(Date, Value)> = Vec::new();
    for (pid, list) in &c.notes {
        let name = views::person_index(w, *pid).map(|k| w.people[k].name.clone()).unwrap_or_else(|| "A former player".into());
        for (k, n) in list.iter().enumerate() {
            let page = if n.page.is_empty() { format!("Player page · {name}") } else { n.page.clone() };
            all.push((
                n.date,
                json!({ "src": "player", "id": pid, "k": k, "date": views::date_text(n.date), "text": n.text, "page": page,
                        "about": { "kind": "player", "id": pid, "name": name } }),
            ));
        }
    }
    for (k, n) in c.desk.notes.iter().enumerate() {
        all.push((
            n.date,
            json!({ "src": "desk", "k": k, "date": views::date_text(n.date), "text": n.text, "page": n.page,
                    "about": about_name(w, n.about).or_else(|| (!n.about_name.is_empty()).then(|| json!({ "name": n.about_name }))) }),
        ));
    }
    all.sort_by_key(|x| std::cmp::Reverse(x.0));
    json!({ "notes": all.into_iter().map(|x| x.1).collect::<Vec<_>>() })
}

fn about_arg(args: &Value) -> Option<(u8, u32)> {
    let a = args.get("about")?;
    let id = a.get("id").and_then(Value::as_u64)? as u32;
    let kind = match a.get("kind").and_then(Value::as_str)? {
        "player" => mail::ABOUT_PLAYER,
        "coach" => mail::ABOUT_COACH,
        "team" => mail::ABOUT_TEAM,
        "game" => mail::ABOUT_GAME,
        _ => return None,
    };
    Some((kind, id))
}

/// A note on anything; a note about one player goes on his page too.
pub fn note_write(w: &mut World, args: &Value) -> Result<Value, String> {
    let text = args.get("text").and_then(Value::as_str).ok_or("Missing text.")?.trim().to_string();
    if text.is_empty() {
        return Err("The note is empty.".into());
    }
    let page = args.get("page").and_then(Value::as_str).unwrap_or("").to_string();
    let about = about_arg(args);
    match about {
        Some((mail::ABOUT_PLAYER, id)) => w.add_note_from(id, text, page),
        _ => {
            let date = w.today;
            let about_name = args.get("about_name").and_then(Value::as_str).unwrap_or("").to_string();
            let c = w.career.as_mut().ok_or("No career.")?;
            c.desk.notes.push(DeskNote { date, text, page, about, about_name });
        }
    }
    Ok(notes(w))
}

pub fn note_edit(w: &mut World, args: &Value) -> Result<Value, String> {
    let k = args.get("k").and_then(Value::as_u64).ok_or("Missing k.")? as usize;
    let text = args.get("text").and_then(Value::as_str);
    let delete = args.get("delete").and_then(Value::as_bool).unwrap_or(false);
    let c = w.career.as_mut().ok_or("No career.")?;
    if args.get("src").and_then(Value::as_str) == Some("player") {
        let pid = args.get("id").and_then(Value::as_u64).ok_or("Missing id.")? as u32;
        let list = c.notes.iter_mut().find(|(id, _)| *id == pid).map(|x| &mut x.1).ok_or("No such note.")?;
        if delete {
            if k < list.len() {
                list.remove(k);
            }
        } else if let (Some(n), Some(t)) = (list.get_mut(k), text) {
            n.text = t.trim().to_string();
        }
        c.notes.retain(|(_, l)| !l.is_empty());
    } else if delete {
        if k < c.desk.notes.len() {
            c.desk.notes.remove(k);
        }
    } else if let (Some(n), Some(t)) = (c.desk.notes.get_mut(k), text) {
        n.text = t.trim().to_string();
    }
    Ok(notes(w))
}

// ---- My players (S5) ----

/// Puts a player on "my players" with your staff's read and his stars that
/// day (`how`: 0 you, 1 a watch, 2 an answer).
pub fn list_add(w: &mut World, id: u32, how: u8) {
    let Some(k) = views::person_index(w, id) else { return };
    let p = &w.people[k];
    let team = views::team_of(w, p);
    let own = team.is_some_and(|t| views::is_mine(w, t));
    let eye = team.filter(|_| own).map_or(10.0, |t| views::eye_for(w, t, p.player.position));
    let read = match reads::overall(&p.player, own, eye, views::reader_key(w)) {
        reads::Read::Range(lo, hi) => (lo, hi),
        reads::Read::NotSeen => (0, 0),
    };
    let stars = p.stars;
    let since = w.today;
    let Some(c) = w.career.as_mut() else { return };
    c.desk.unlisted.retain(|&u| u != id);
    if !c.desk.is_listed(id) {
        c.desk.listed.push(Listed { id, since, read, stars, how });
    }
}

pub fn list_remove(w: &mut World, id: u32) {
    let Some(c) = w.career.as_mut() else { return };
    c.desk.listed.retain(|l| l.id != id);
    if !c.desk.unlisted.contains(&id) {
        c.desk.unlisted.push(id);
    }
}

pub fn is_listed(w: &World, id: u32) -> bool {
    w.career.as_ref().is_some_and(|c| c.desk.is_listed(id) || c.notes.iter().any(|(p, l)| *p == id && !l.is_empty()))
}

pub fn has_note(w: &World, id: u32) -> bool {
    w.career.as_ref().is_some_and(|c| c.notes.iter().any(|(p, l)| *p == id && !l.is_empty()))
}

/// The "my players" list: everyone you put on it, and everyone you wrote a
/// note on (scouting topic 18), with where he is now against where he was.
pub fn my_players(w: &World) -> Value {
    let Some(c) = &w.career else { return json!({ "rows": [] }) };
    let reader = views::reader_key(w);
    let mut ids: Vec<(u32, Option<&Listed>)> = c.desk.listed.iter().map(|l| (l.id, Some(l))).collect();
    for (id, l) in &c.notes {
        if !l.is_empty() && !ids.iter().any(|x| x.0 == *id) && !c.desk.unlisted.contains(id) {
            ids.push((*id, None));
        }
    }
    let rows: Vec<Value> = ids
        .into_iter()
        .filter_map(|(id, l)| {
            let k = views::person_index(w, id)?;
            let p = &w.people[k];
            let team = views::team_of(w, p);
            let own = team.is_some_and(|t| views::is_mine(w, t));
            let eye = team.filter(|_| own).map_or(10.0, |t| views::eye_for(w, t, p.player.position));
            let now = reads::overall(&p.player, own, eye, reader);
            let level = match p.stage {
                Stage::HighSchool | Stage::PrepSchool => "High school",
                Stage::College => "College",
                Stage::Declared => "Draft pool",
                Stage::Pro => "NFL",
                Stage::Done => "Out of football",
            };
            Some(json!({
                "id": id,
                "name": p.name,
                "pos": p.player.position.name(),
                "age": p.player.age,
                "level": level,
                "class": views::class_text(w, p),
                "team": team.map(|t| views::team_brief(w, t)),
                "where": views::where_now(w, p),
                "stars": p.stars,
                "read": now.json(),
                "then": l.map(|l| json!({ "date": views::date_text(l.since), "read": if l.read.1 > 0 { json!([l.read.0, l.read.1]) } else { Value::Null }, "stars": l.stars, "how": l.how })),
                "note": w.notes_on(id).last().map(|n| n.text.clone()),
                "notes": w.notes_on(id).len(),
                "status": views::status_text(w, p),
            }))
        })
        .collect();
    json!({ "rows": rows, "watches": watches(w) })
}

// ---- Watches: saved searches that tell you about new matches ----

pub fn watches(w: &World) -> Value {
    let Some(c) = &w.career else { return json!([]) };
    json!(c.desk.watches.iter().enumerate().map(|(k, x)| json!({ "k": k, "name": x.name, "query": serde_json::from_str::<Value>(&x.query).unwrap_or(Value::Null), "since": views::date_text(x.since), "seen": x.seen.len() })).collect::<Vec<_>>())
}

pub fn watch_set(w: &mut World, args: &Value) -> Result<Value, String> {
    let today = w.today;
    if let Some(k) = args.get("remove").and_then(Value::as_u64) {
        let c = w.career.as_mut().ok_or("No career.")?;
        if (k as usize) < c.desk.watches.len() {
            c.desk.watches.remove(k as usize);
        }
        return Ok(watches(w));
    }
    let name = args.get("name").and_then(Value::as_str).unwrap_or("My search").to_string();
    let query = args.get("query").cloned().unwrap_or(Value::Null);
    // What it finds today is already known: it tells you about new ones.
    let seen: Vec<u32> = super::tables::finder_ids(w, &query, 2000);
    let c = w.career.as_mut().ok_or("No career.")?;
    c.desk.watches.push(afm_engine::world::desk::Watch { name, query: query.to_string(), since: today, seen });
    Ok(watches(w))
}

/// Monday: each watch's new matches, as one mail from your personnel office.
pub fn watch_mail(w: &mut World) {
    let Some(me) = views::mine(w) else { return };
    let Some(c) = &w.career else { return };
    if c.desk.watches.is_empty() {
        return;
    }
    let today = w.today;
    let mut found: Vec<(usize, Vec<u32>)> = Vec::new();
    for (k, x) in c.desk.watches.iter().enumerate() {
        let q: Value = serde_json::from_str(&x.query).unwrap_or(Value::Null);
        let new: Vec<u32> = super::tables::finder_ids(w, &q, 2000).into_iter().filter(|id| !x.seen.contains(id)).take(25).collect();
        if !new.is_empty() {
            found.push((k, new));
        }
    }
    let dear = w.career.as_ref().map(|c| format!("Coach {}", c.you.name.split_whitespace().last().unwrap_or(""))).unwrap_or_default();
    for (k, new) in found {
        let name = w.career.as_ref().map(|c| c.desk.watches[k].name.clone()).unwrap_or_default();
        let mut m = mail::letter(
            &format!("watch|{k}|{name}"),
            today,
            7 * 60 + 45,
            Head::Memo,
            views::team_name(w, me),
            "Football office".into(),
            super::people::staff_name(w, me, super::people::Job::Personnel),
            super::people::Job::Personnel.title(me).into(),
            w.career.as_ref().map(|c| format!("{}, head coach", c.you.name)).unwrap_or_default(),
            format!("Your search \"{name}\": {} new", new.len()),
        );
        m.org_team = Some(mail::team_code(me));
        m.body = vec![format!("{dear}, your saved search turned up players it hadn't before:")];
        m.table = vec![vec!["Player".into(), "Pos".into(), "Team".into(), "Class".into()]];
        for &id in &new {
            if let Some(i) = views::person_index(w, id) {
                let p = &w.people[i];
                m.table.push(vec![
                    links::player(id, &p.name),
                    p.player.position.name().into(),
                    views::team_of(w, p).map(|t| links::team(w, t)).unwrap_or_else(|| views::where_now(w, p)),
                    views::class_text(w, p),
                ]);
            }
        }
        m.ids = new.clone();
        m.keys = mail::RECRUITING;
        m.answers = vec![
            mail::answer("Add them all to my players", "Every player in the table goes on your \"my players\" list.", mail::act::LIST_ALL, 0),
            mail::answer("Not now", "Nothing changes; the search keeps watching.", mail::act::ACK, 0),
        ];
        mail::send(w, m);
        if let Some(c) = w.career.as_mut() {
            c.desk.watches[k].seen.extend(new);
        }
    }
}

/// Reveals waiting to be shown, taken off the desk as they are handed over.
pub fn take_reveals(w: &mut World) -> Value {
    let Some(c) = w.career.as_mut() else { return json!([]) };
    let list = std::mem::take(&mut c.desk.reveals);
    json!(list.iter().map(|r| json!({
        "kind": r.kind, "kicker": r.kicker, "number": r.number, "name": r.name, "sub": r.sub,
        "verdict": r.verdict, "colors": r.colors, "date": views::date_text(r.date),
        "about": r.about.map(|(k, id)| json!({ "kind": match k { 0 => "player", 1 => "coach", 2 => "team", _ => "game" }, "id": id,
            "ref": if k == 2 { views::team_brief(w, mail::team_from(id))["ref"].clone() } else { Value::Null } })),
    })).collect::<Vec<_>>())
}

/// Pinned under Continue (S10): mails waiting on an answer by a date.
pub fn pinned(w: &World) -> Vec<Value> {
    let Some(c) = &w.career else { return Vec::new() };
    c.desk
        .mails
        .iter()
        .filter(|m| m.answered.is_none() && m.answer_by.is_some() && !m.answers.is_empty())
        .map(|m| json!({ "text": m.subject, "by": m.answer_by.map(views::date_text), "mail": m.id.to_string(), "date": views::date_text(m.date) }))
        .collect()
}

pub fn unread(w: &World) -> usize {
    w.career.as_ref().map_or(0, |c| c.desk.unread())
}
