//! The cards the opening, library and load pages show for a world or a
//! save without loading it (section 8, topics 1 and 3, in the FM-style
//! pages Cain agreed on 2026-10-21): the world's story, your team, the
//! headlines. The game writes a card into each file's header; a library
//! world written before cards gets one beside it the first time it opens.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use afm_engine::calendar::{Date, Event, Kind, Loudness};
use afm_engine::world::World;
use afm_engine::world::names::NameMode;
use serde_json::{Value, json};

use crate::views::{self, TeamRef};

/// Everything the pages show about this world as it is today.
pub fn card(w: &World) -> Value {
    json!({
        "world": world_part(w),
        "seat": seat_part(w),
        "headlines": headlines(w),
        "month": month_text(w.today),
    })
}

/// Where a card that isn't in a file's header is kept.
pub fn beside(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".card.json");
    PathBuf::from(s)
}

/// A file's card: from its header, else from beside it, if the card beside
/// it was made for this very file (a world replaced under the same name, or
/// a card cut short, reads as none and is made again).
pub fn read(path: &Path, meta: &Value) -> Value {
    match meta.get("card") {
        Some(c) if !c.is_null() => c.clone(),
        _ => std::fs::read(beside(path))
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .filter(|c| c.get("for").is_some_and(|f| Some(f) == meta.get("written")))
            .unwrap_or(Value::Null),
    }
}

/// Writes the card beside a file, marked with the file it was made for.
pub fn write_beside(path: &Path, w: &World, meta: &Value) {
    let mut c = card(w);
    c["for"] = meta.get("written").cloned().unwrap_or(Value::Null);
    let _ = std::fs::write(beside(path), c.to_string());
}

fn month_text(d: Date) -> String {
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (y, m, _) = d.parts();
    format!("{} {y}", M[(m - 1) as usize])
}

fn number_word(n: u32) -> String {
    const W: [&str; 21] = [
        "No",
        "One",
        "Two",
        "Three",
        "Four",
        "Five",
        "Six",
        "Seven",
        "Eight",
        "Nine",
        "Ten",
        "Eleven",
        "Twelve",
        "Thirteen",
        "Fourteen",
        "Fifteen",
        "Sixteen",
        "Seventeen",
        "Eighteen",
        "Nineteen",
        "Twenty",
    ];
    W.get(n as usize).map_or_else(|| n.to_string(), |s| s.to_string())
}

/// Titles by team over the history, most first (ties by the latest title).
fn counts(titles: &[(i32, String)]) -> Vec<(String, u32)> {
    let mut by: HashMap<&str, (u32, i32)> = HashMap::new();
    for (y, t) in titles {
        let e = by.entry(t.as_str()).or_insert((0, 0));
        e.0 += 1;
        e.1 = e.1.max(*y);
    }
    let mut v: Vec<(&str, (u32, i32))> = by.into_iter().collect();
    v.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(b.1.1.cmp(&a.1.1)).then(a.0.cmp(b.0)));
    v.into_iter().map(|(t, (n, _))| (t.to_string(), n)).collect()
}

fn world_part(w: &World) -> Value {
    let o = &w.options;
    let first = o.start_year - o.history_years as i32;
    let last = o.start_year - 1;
    let in_history = |y: i32| y >= first && y <= last;
    // National champions: the top division's, season by season.
    let mut nat: Vec<(i32, String, usize, u32)> = Vec::new();
    for p in &w.programs {
        for s in p.seasons.iter().filter(|s| s.national_title && in_history(s.year)) {
            if p.division == afm_engine::world::data::Division::FBS {
                nat.push((s.year, w.program_name(p.id).to_string(), p.id, s.losses));
            }
        }
    }
    nat.sort_by_key(|x| x.0);
    let nc = counts(&nat.iter().map(|x| (x.0, x.1.clone())).collect::<Vec<_>>());
    let sb: Vec<(i32, String)> =
        w.nfl_champions.iter().filter(|(y, _)| in_history(*y)).map(|&(y, t)| (y, w.nfl_team_name(t))).collect();
    let sbc = counts(&sb);
    let mut story: Vec<(String, String)> = Vec::new();
    if let Some((top, n)) = nc.first() {
        let mut line = format!("{top} won {n} national title{} since {first}.", if *n == 1 { "" } else { "s" });
        if let Some(&(_, _, id, _)) = nat.iter().find(|x| &x.1 == top)
            && let Some(c) = w.programs[id].head_coach.and_then(|c| w.coaches.get(c))
            && let Some(j) = c.job
            && w.year - j.since >= 5
        {
            line.push_str(&format!(" Its head coach since {} is {}-{}.", j.since, c.wins, c.losses));
        }
        story.push(("Dynasty".into(), line));
    }
    if let Some((t, n)) = nc.get(1) {
        story.push((
            "The rival".into(),
            format!("{t}, {n} national title{} in the same years.", if *n == 1 { "" } else { "s" }),
        ));
    }
    if nc.len() > 2 {
        let rest: Vec<String> = nc[2..].iter().take(4).map(|(t, n)| format!("{t} {n}")).collect();
        let recent: Vec<&(i32, String, usize, u32)> = nat.iter().rev().take(15).collect();
        let unbeaten = recent.iter().filter(|x| x.3 == 0).count();
        story.push((
            "Everyone else".into(),
            format!(
                "{}{}. {} of the last {} champions went unbeaten.",
                rest.join(", "),
                if nc.len() > 6 { ", and more" } else { "" },
                number_word(unbeaten as u32),
                recent.len()
            ),
        ));
    }
    if let Some((y, t)) = sb.last() {
        let mut line = format!("The {t} won the Super Bowl to end the {y} season.");
        let d = &w.dates;
        if d.combine_start > w.today {
            line.push_str(&format!(
                " The combine opens {}; free agency opens {}.",
                views::date_text(d.combine_start),
                views::date_text(d.league_year.date)
            ));
        }
        story.push(("The NFL".into(), line));
    }
    // The years the engine played high school games: the last n before the
    // start, never before the history or before the engine itself began.
    let hs_from = o.hs_engine_years.map(|n| (o.start_year - n as i32).max(first).max(o.engine_from.unwrap_or(first)));
    let hs = match hs_from {
        Some(from) if from <= last => {
            format!(
                "{} years of Friday nights on the match engine, {from}-{last}: every college and NFL player who came through them has a real high school stat line.",
                number_word((last - from + 1) as u32)
            )
        }
        _ => {
            "High school seasons before this one were played on the quick model: records and standings, no stat lines."
                .to_string()
        }
    };
    story.push(("High school".into(), hs));
    json!({
        "seed": o.seed,
        "history_years": o.history_years,
        "first": first,
        "last": last,
        "names": if o.names == NameMode::Fictional { "fictional" } else { "real" },
        "starts": views::date_text(w.today),
        "nc": nc.iter().take(6).map(|(t, n)| json!([t, n])).collect::<Vec<_>>(),
        "sb": sbc.iter().take(6).map(|(t, n)| json!([t, n])).collect::<Vec<_>>(),
        "story": story.iter().map(|(a, b)| json!([a, b])).collect::<Vec<_>>(),
    })
}

/// Your team on a save's card.
fn seat_part(w: &World) -> Value {
    let (Some(c), Some(t)) = (w.career.as_ref(), views::mine(w)) else { return Value::Null };
    let coach = w.coaches.get(c.coach);
    let (record, note) = views::season_record(w, t);
    let next = views::calendar(w)["items"]
        .as_array()
        .and_then(|a| a.first())
        .map(|x| format!("{}, {}", x["text"].as_str().unwrap_or(""), x["date"].as_str().unwrap_or("")));
    json!({
        "team": views::team_brief(w, t),
        // An assistant's card names his job (lane 10).
        "role": match afm_engine::world::front_office::market::your_job(w) {
            Some(j) if j.role != afm_engine::world::coach::Role::HeadCoach => {
                afm_engine::world::front_office::market::role_title(j.role)
            }
            _ if matches!(t, TeamRef::Nfl(_)) => "Head coach and GM".to_string(),
            _ => "Head coach".to_string(),
        },
        "line": views::team_line(w, t),
        "record": if note == "this season" { record } else { format!("{record} {note}") },
        "next": next,
        "coach": format!("{}, {}", c.you.name, coach.map_or(c.you.age_on(w.today), |k| k.age_on(w.today))),
        "date": views::date_text(w.today),
        "road": c.you.road,
    })
}

/// The latest loud lines of the news wire, newest first. Early in a
/// football year they are last year's, which the calendar keeps in its
/// history once the Super Bowl closes the year.
fn headlines(w: &World) -> Vec<String> {
    let this_year = w.calendar.events.range(..=w.today).rev().flat_map(|(_, evs)| evs.iter().rev());
    let mut out: Vec<String> = Vec::new();
    // What happened, not the dates on the calendar ("Super Bowl (Sun Feb 8).").
    let news = |e: &&Event| {
        e.loud >= Loudness::Big
            && !matches!(e.kind, Kind::Date | Kind::Window | Kind::Deadline | Kind::Slot | Kind::Rhythm)
            && !e.text.starts_with("Super Bowl (")
    };
    for e in this_year.chain(w.calendar.history.iter().rev()).filter(news) {
        let t = e.text.to_string();
        if !out.contains(&t) {
            out.push(t);
        }
        if out.len() >= 3 {
            break;
        }
    }
    out
}
