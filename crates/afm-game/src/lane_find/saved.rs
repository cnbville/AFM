//! Saved searches (FS19): a search kept by name that tells you, through
//! the inbox, about players it finds that it hadn't before. It looks again
//! when something changes what a staff knows: a services update, your
//! camp, or a big game (the high school all-star games for a college; the
//! end of the college season, the all-star week and the combine for a
//! club).
//!
//! A saved search is the desk's `Watch`: its name, the search as the
//! screen sent it, the day it was saved, the players it found at its last
//! look (sorted, so a look reads them at once; as many as it matches, so a
//! player still matching is never told twice) and the ones its last mail
//! named.

use std::collections::HashSet;

use afm_engine::calendar::{Date, Weekday};
use afm_engine::world::World;
use afm_engine::world::desk::{Head, Watch};
use afm_engine::world::scouting::services;
use serde_json::{Value, json};

use super::{college, nfl};
use crate::Reply;
use crate::lane_core::{links, mail, people};
use crate::views::{self, TeamRef};

/// The most players one mail names.
const MAIL_MOST: usize = 25;

/// Which search a saved one is: "college" or "nfl", or None for the
/// finder's own watches.
fn kind_of(x: &Watch) -> Option<String> {
    let q: Value = serde_json::from_str(&x.query).ok()?;
    q.get("find").and_then(Value::as_str).map(str::to_string)
}

/// Your saved searches for this seat's page, with what each has new.
pub fn list(w: &World, seat: &str) -> Value {
    let Some(c) = &w.career else { return json!([]) };
    json!(
        c.desk
            .watches
            .iter()
            .enumerate()
            .filter(|(_, x)| kind_of(x).as_deref() == Some(seat))
            .map(|(k, x)| {
                let spec: Value = serde_json::from_str(&x.query).unwrap_or(Value::Null);
                json!({ "k": k, "name": x.name, "spec": spec, "since": views::date_text(x.since), "new": x.fresh.len(), "told": x.seen.len() })
            })
            .collect::<Vec<_>>()
    )
}

/// The ids a saved search finds today, for your seat (None when the
/// search isn't for the seat you hold).
fn found(w: &World, spec: &Value) -> Option<Vec<u32>> {
    match (spec.get("find").and_then(Value::as_str), views::mine(w)) {
        (Some("college"), Some(TeamRef::College(j))) => Some(college::ids(w, j, &college::Spec::parse(spec))),
        (Some("nfl"), Some(TeamRef::Nfl(t))) => Some(nfl::ids(w, t, &nfl::Spec::parse(spec))),
        _ => None,
    }
}

/// Saves the search on the screen by name: what it finds today counts as
/// told, so it tells you about the new ones from here.
pub fn save(w: &mut World, args: &Value) -> Reply {
    let spec = args.get("spec").cloned().filter(Value::is_object).ok_or("Missing the search.")?;
    let seat = spec.get("find").and_then(Value::as_str).ok_or("Missing the search's seat.")?.to_string();
    let name = args.get("name").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).unwrap_or("My search");
    let name: String = name.chars().take(60).collect();
    let mut seen = found(w, &spec).ok_or("That search is for another seat.")?;
    seen.sort_unstable();
    let today = w.today;
    let c = w.career.as_mut().ok_or("No career.")?;
    c.desk.watches.push(Watch { name: name.clone(), query: spec.to_string(), since: today, seen, fresh: Vec::new() });
    Ok(
        json!({ "said": format!("Saved \"{name}\". Your staff will tell you when it finds someone new."), "saved": list(w, &seat) }),
    )
}

/// Forgets a saved search.
pub fn forget(w: &mut World, args: &Value) -> Reply {
    let k = args.get("k").and_then(Value::as_u64).ok_or("Missing k.")? as usize;
    let c = w.career.as_mut().ok_or("No career.")?;
    let x = c.desk.watches.get(k).ok_or("No such search.")?;
    let seat = kind_of(x).ok_or("That isn't a saved search of this page.")?;
    let name = x.name.clone();
    c.desk.watches.remove(k);
    Ok(json!({ "said": format!("Forgot \"{name}\"."), "saved": list(w, &seat) }))
}

/// Opening a saved search clears what it had new.
pub fn opened(w: &mut World, k: usize) {
    if let Some(x) = w.career.as_mut().and_then(|c| c.desk.watches.get_mut(k)) {
        x.fresh.clear();
    }
}

/// What makes a staff look again on a day, in the mail's words.
fn trigger(w: &World, d: Date, college: bool) -> Option<String> {
    let y = d.year();
    let dates = &w.dates;
    if college {
        if let Some(u) = services::update_days(y).iter().find(|u| u.0 == d) {
            return Some(format!("the services' {} update", u.1));
        }
        // The signing class's new order at a commitment wave.
        if let Some(c) = w.scouting.rankings.classes.iter().find(|c| c.date == d) {
            return Some(format!("the services' update for the class of {}", c.class_of));
        }
        let camps = [2, 3].map(|n| Date::nth_weekday(y, 6, Weekday::Sat, n));
        if camps.contains(&d) {
            return Some("your camp".into());
        }
        if d == dates.all_american_bowl {
            return Some("the All-American Bowl".into());
        }
        if d == dates.polynesian_bowl {
            return Some("the Polynesian Bowl".into());
        }
        return None;
    }
    let named = [
        (dates.rivalry_week, "rivalry week"),
        (dates.conference_championships, "the conference championships"),
        (dates.title_game, "the national title game"),
        (dates.senior_bowl, "the Senior Bowl"),
        (dates.combine_end, "the combine"),
        (dates.pro_days.1, "the last pro days"),
    ];
    named.iter().find(|x| x.0 == d).map(|x| x.1.to_string())
}

/// After the clock ran from `before` (the last day mailed): each saved
/// search looks again if one of the days run since changed what your staff
/// knows, and mails the players it finds that it didn't at its last look.
/// Today itself hasn't run yet, so its trigger waits for the next step.
pub fn after_days(w: &mut World, before: Date) {
    let Some(me) = views::mine(w) else { return };
    let college = match me {
        TeamRef::College(_) => true,
        TeamRef::Nfl(_) => false,
        _ => return,
    };
    let Some(c) = &w.career else { return };
    if !c.desk.watches.iter().any(|x| kind_of(x).is_some()) {
        return;
    }
    let mut why: Option<(Date, String)> = None;
    let mut d = before.add(1);
    while d < w.today {
        if let Some(t) = trigger(w, d, college) {
            why = Some((d, t));
        }
        d = d.add(1);
    }
    let Some((day, what)) = why else { return };
    // Each search's matches today, and the ones it hadn't found before.
    let mut looks: Vec<(usize, Vec<u32>, Vec<u32>)> = Vec::new();
    if let Some(c) = &w.career {
        for (k, x) in c.desk.watches.iter().enumerate() {
            let Ok(spec) = serde_json::from_str::<Value>(&x.query) else { continue };
            if spec.get("find").is_none() {
                continue;
            }
            let Some(mut ids) = found(w, &spec) else { continue };
            ids.sort_unstable();
            let told: HashSet<u32> = x.seen.iter().copied().collect();
            let new: Vec<u32> = ids.iter().copied().filter(|id| !told.contains(id)).collect();
            looks.push((k, ids, new));
        }
    }
    let date = day.add(1).min(w.today);
    for (k, ids, new) in looks {
        if !new.is_empty() {
            tell(w, me, k, &new, date, &what);
        }
        if let Some(x) = w.career.as_mut().and_then(|c| c.desk.watches.get_mut(k)) {
            if !new.is_empty() {
                x.fresh = new;
            }
            x.seen = ids;
        }
    }
}

/// The mail from your personnel office: the new players in a table.
fn tell(w: &mut World, me: TeamRef, k: usize, new: &[u32], date: Date, what: &str) {
    let Some(c) = &w.career else { return };
    let name = c.desk.watches[k].name.clone();
    let dear = format!("Coach {}", afm_engine::world::names::surname_of(&c.you.name));
    let to = format!("{}, head coach", c.you.name);
    let screen = if matches!(me, TeamRef::College(_)) { "recruit_find" } else { "find_prospects" };
    let more = new.len().saturating_sub(MAIL_MOST);
    let mut m = mail::letter(
        &format!("find|{k}|{name}"),
        date,
        7 * 60 + 40,
        Head::Memo,
        views::team_name(w, me),
        "Football office".into(),
        people::staff_name(w, me, people::Job::Personnel),
        people::Job::Personnel.title(me).into(),
        to,
        format!("Your search \"{name}\": {} new after {what}", new.len()),
    );
    m.org_team = Some(mail::team_code(me));
    m.body = vec![
        format!("{dear}, after {what} your saved search \"{name}\" turned up players it hadn't before:"),
        if more > 0 {
            format!("{more} more are on the {}.", links::screen(screen, "search page"))
        } else {
            format!("They're on the {} too.", links::screen(screen, "search page"))
        },
    ];
    m.table = vec![vec!["Player".into(), "Pos".into(), "Team".into(), "Class".into(), "Stars".into()]];
    let shown: Vec<u32> = new.iter().copied().take(MAIL_MOST).collect();
    for &id in &shown {
        if let Some(i) = views::person_index(w, id) {
            let p = &w.people[i];
            m.table.push(vec![
                links::player(id, &p.name),
                p.player.position.name().into(),
                views::team_of(w, p).map(|t| links::team(w, t)).unwrap_or_else(|| views::where_now(w, p)),
                views::class_text(w, p),
                if p.stars > 0 { format!("{}★", p.stars) } else { "—".into() },
            ]);
        }
    }
    m.ids = shown;
    m.keys = if matches!(me, TeamRef::College(_)) { mail::RECRUITING } else { mail::DRAFT };
    m.answers = vec![
        mail::answer(
            "Add them all to my players",
            "Every player in the table goes on your \"my players\" list.",
            mail::act::LIST_ALL,
            0,
        ),
        mail::answer("Not now", "Nothing changes; the search keeps watching.", mail::act::ACK, 0),
    ];
    mail::send(w, m);
}
