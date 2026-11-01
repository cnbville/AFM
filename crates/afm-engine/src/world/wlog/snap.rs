//! Snapshots: whole stores on the year's key dates, taken first thing in the
//! morning, before any system acts that day.

use serde::Serialize;

use super::WorldLog;
use crate::calendar::Date;
use crate::world::World;

#[derive(Serialize)]
struct Snapshot<'a, T: Serialize + ?Sized> {
    season: i32,
    date: Date,
    /// Which key date ("kickoff", "draft eve", "year end"...).
    at: &'static str,
    store: &'static str,
    data: &'a T,
}

fn put<T: Serialize + ?Sized>(log: &WorldLog, w: &World, at: &'static str, store: &'static str, data: &T) {
    log.put("snapshots", &Snapshot { season: w.year, date: w.today, at, store, data });
}

/// Who is in the draft pool on the morning of the draft, with the reads
/// on him (the December read, the advisory grade, his position).
#[derive(Serialize)]
struct Prospect<'a> {
    id: u32,
    name: &'a str,
    college: Option<usize>,
    class_of: i32,
    position: crate::positions::Position,
    overall: f64,
    potential: f64,
    cal: &'a crate::world::PersonCal,
}

/// The morning's snapshots, on the key dates. The college season's team
/// sheets (every engine player) go once a year, at the year's close: the
/// other college snapshots leave them out, set aside and put back as they
/// were.
pub(super) fn day(w: &mut World, d: Date) {
    let y = &w.dates;
    let college = y.cfb_weeks.first() == Some(&d) || d == y.selection_day || d == y.title_game.add(1);
    let teams = if college { w.college.as_mut().map(|c| std::mem::take(&mut c.teams)) } else { None };
    shots(w, d);
    if let (Some(c), Some(t)) = (w.college.as_mut(), teams) {
        c.teams = t;
    }
}

fn shots(w: &World, d: Date) {
    let Some(log) = w.wlog.as_deref() else { return };
    let y = &w.dates;
    if y.cfb_weeks.first() == Some(&d) {
        // College kickoff: the season's schedule and every team's sheet.
        if let Some(c) = &w.college {
            put(log, w, "college kickoff", "college", c);
        }
        put(log, w, "college kickoff", "deals", &w.deals);
    }
    if d == y.selection_day
        && let Some(c) = &w.college
    {
        put(log, w, "selection day", "college", c);
        put(log, w, "selection day", "voters", &w.voters);
    }
    if d == y.title_game.add(1)
        && let Some(c) = &w.college
    {
        put(log, w, "after the title game", "college", c);
    }
    if d == y.draft[0].date {
        // Draft day's morning: the desks' boards and mocks, the combine,
        // the December reads, the clubs' boards and needs, the pool.
        put(log, w, "draft eve", "scouting", &w.scouting);
        put(log, w, "draft eve", "clubs", &w.clubs);
        put(log, w, "draft eve", "league", &w.nfl);
        put(log, w, "draft eve", "market", &w.market);
        let pool: Vec<Prospect> = w
            .people
            .iter()
            .filter(|p| p.stage == crate::world::Stage::Declared)
            .map(|p| Prospect {
                id: p.player.id,
                name: &p.name,
                college: p.college,
                class_of: p.class_of,
                position: p.player.position,
                overall: p.player.overall(),
                potential: p.player.potential(),
                cal: &p.cal,
            })
            .collect();
        put(log, w, "draft eve", "draft pool", &pool);
    }
    if d == y.udfa_rush.date.add(1) {
        put(log, w, "after the draft", "scouting", &w.scouting);
        put(log, w, "after the draft", "market", &w.market);
    }
    if d == y.cutdown.date.add(1) {
        // The 53s, the practice squads and the depth charts after cutdown.
        put(log, w, "after cutdown", "clubs", &w.clubs);
        put(log, w, "after cutdown", "league", &w.nfl);
        put(log, w, "after cutdown", "market", &w.market);
    }
    if d == y.nfl_kickoff {
        put(log, w, "nfl kickoff", "clubs", &w.clubs);
        if let Some(s) = &w.nfl_season {
            put(log, w, "nfl kickoff", "nfl", s);
        }
    }
    if Some(d) == y.early_signing {
        put(log, w, "early signing", "scouting", &w.scouting);
    }
}

/// The year's close: every store that is overwritten or cleared as the
/// next year starts.
pub(super) fn year_end(w: &World, log: &WorldLog) {
    let at = "year end";
    if let Some(c) = &w.college {
        put(log, w, at, "college", c);
    }
    if let Some(s) = &w.hs_season {
        put(log, w, at, "high school", s);
    }
    if let Some(s) = &w.nfl_season {
        put(log, w, at, "nfl", s);
    }
    put(log, w, at, "league", &w.nfl);
    put(log, w, at, "scouting", &w.scouting);
    put(log, w, at, "market", &w.market);
    put(log, w, at, "voters", &w.voters);
    put(log, w, at, "folk", &w.folk);
    put(log, w, at, "finance", &w.finance);
    put(log, w, at, "stories", &w.stories);
    put(log, w, at, "storylines", &w.storylines);
    put(log, w, at, "charts", &w.charts);
    put(log, w, at, "others", &w.others);
    put(log, w, at, "formers", &w.formers);
    put(log, w, at, "deals", &w.deals);
    put(log, w, at, "searches", &w.searches);
    put(log, w, at, "realign", &w.realign);
    put(log, w, at, "nfl book", &w.nfl_book);
}
