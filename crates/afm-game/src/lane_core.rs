//! Lane commands: core (docs/final-build/plan.md). `call` answers the
//! commands this lane owns and returns None for any other.
//!
//! Lane 9 of the final build is the screens' core: the inbox and your desk
//! (10a-10i), Home (9a-9e), linking on every screen (L1-L8), the player
//! page (S1-S3), "my players" (S5), search across everything (S6), the big
//! tables (S8), news feeds, the coach and career pages, standings with
//! brackets, the history books and the big moments.
//!
//! `after_days` runs after the clock moves (Continue, a day, a holiday) to
//! file the morning's mail.

pub mod desk;
pub mod links;
pub mod mail;
pub mod pages;
pub mod people;
pub mod tables;

use afm_engine::calendar::Weekday;
use afm_engine::world::World;
use serde_json::{Value, json};

use crate::{Game, Reply};

/// Files mail for the days the clock just ran, and on Mondays tells you
/// what your saved searches found.
pub fn after_days(w: &mut World) {
    let before = w.career.as_ref().and_then(|c| c.desk.through);
    mail::after_days(w);
    crate::lane_stories::after_days(w);
    crate::lane_squadboards::after_days(w);
    if let Some(b) = before {
        let mut d = b.add(1);
        let mut monday = false;
        while d < w.today {
            monday |= d.weekday() == Weekday::Mon;
            d = d.add(1);
        }
        if monday {
            desk::watch_mail(w);
        }
        crate::lane_find::after_days(w, b);
    }
}

fn id_arg(args: &Value) -> Result<u64, String> {
    args.get("id")
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .ok_or_else(|| "Missing id.".to_string())
}

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    let home = game.home.clone();
    let Some(w) = game.world.as_mut() else {
        return matches!(
            cmd,
            "mail" | "letter" | "dashboard" | "profile" | "peek" | "find" | "leaders" | "finder" | "calendar_month"
        )
        .then(|| Err("No world is loaded.".to_string()));
    };
    let r: Reply = match cmd {
        // The inbox and the desk.
        "mail" => Ok(desk::list(w, args)),
        "letter" => id_arg(args).and_then(|id| desk::open(w, id)),
        "mail_set" => desk::set(w, args),
        "mail_answer" => (|| {
            let id = id_arg(args)?;
            let k = args.get("pick").and_then(Value::as_u64).ok_or("Missing pick.")? as usize;
            mail::pick(w, id, k)?;
            desk::open(w, id)
        })(),
        "notes_all" => Ok(desk::notes(w)),
        "note_write" => desk::note_write(w, args),
        "note_edit" => desk::note_edit(w, args),
        "my_players" => Ok(desk::my_players(w)),
        "list_add" | "list_remove" => (|| {
            let id = id_arg(args)? as u32;
            if cmd == "list_add" {
                desk::list_add(w, id, 0);
            } else {
                desk::list_remove(w, id);
            }
            Ok(
                json!({ "listed": desk::is_listed(w, id), "count": w.career.as_ref().map_or(0, |c| c.desk.listed.len()) }),
            )
        })(),
        "watch_set" => desk::watch_set(w, args),
        "reveals" => Ok(desk::take_reveals(w)),
        // The big tables.
        "leaders" => Ok(tables::leaders(w, args)),
        "finder" => Ok(tables::finder(w, args)),
        "table_csv" => tables::csv(w, &home, args),
        "export_text" => tables::export(
            &home,
            args.get("name").and_then(Value::as_str).unwrap_or("table"),
            args.get("text").and_then(Value::as_str).unwrap_or(""),
        ),
        // Pages.
        "dashboard" => Ok(pages::dashboard(w)),
        "profile" => id_arg(args).and_then(|id| pages::profile(w, id as u32)),
        "peek" => pages::peek(w, args),
        "find" => Ok(pages::find(w, args.get("q").and_then(Value::as_str).unwrap_or(""))),
        "compare" => Ok(pages::compare(w, args)),
        "history" => Ok(pages::history(w, args)),
        "brackets" => Ok(pages::brackets(w, args)),
        "calendar_month" => Ok(pages::calendar_month(w, args)),
        "feed" => Ok(pages::feed(w, args)),
        "coach_page" => id_arg(args).and_then(|id| pages::coach_page(w, id as usize)),
        "career_page" => Ok(pages::career_page(w)),
        "team_page" => crate::views::TeamRef::from_args(w, args).map(|t| pages::team_page(w, t)),
        "linked" => Ok(pages::linked(w, args)),
        _ => return None,
    };
    Some(r)
}
