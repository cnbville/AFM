//! The high school team page's middle (`docs/game.md`, "The team page";
//! Cain's mock-up of 2026-11-14): the three prospects to know first, the
//! school's talent by class, the rest of the roster, the state poll and
//! where its players go, and its head coach's tenure. Reads judge as
//! everywhere (`team_page::judged`): your own read of a player, your
//! staff's next, the read off film for a school you have no eyes on.
//!
//! A college seat sees where each player stands with it (offered, visited,
//! watching, committed elsewhere); every other seat sees his college
//! interest (his offers, where he has committed). A block the world can't
//! fill is left out.

use std::collections::HashMap;

use afm_engine::attributes::Attr;
use afm_engine::world::scouting::file;
use afm_engine::world::{Stage, Visit, World};
use serde_json::{Value, json};

use super::team_page::judged;
use crate::lane_squadboards::mid;
use crate::reads::{self, Read};
use crate::views::{self, TeamRef};

/// How many players the "rest of the roster" lists.
const REST: usize = 10;
/// The classes "where its players go" counts back.
const CLASSES: i32 = 8;

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// Where one player stands with the viewer: a short kind the dots and pills
/// colour by (`mine` offered or committed to you, `visited`, `none`,
/// `away` committed elsewhere) and the words.
fn stand(w: &World, k: usize, me: Option<TeamRef>) -> (&'static str, String) {
    let p = &w.people[k];
    let elsewhere = |prog: usize| w.program_name(prog).to_string();
    match me {
        Some(TeamRef::College(prog)) => {
            if p.cal.signed.is_some_and(|s| s.program == prog) {
                return ("mine", "Signed with you".into());
            }
            if p.cal.commit.is_some_and(|c| c.0 == prog) {
                return ("mine", "Committed to you".into());
            }
            if let Some(s) = p.cal.signed {
                return ("away", format!("Signed with {}", elsewhere(s.program)));
            }
            if let Some(c) = p.cal.commit {
                return ("away", format!("Committed to {}", elsewhere(c.0)));
            }
            let offered = p.cal.offers.contains(&prog) || p.cal.early_offers.iter().any(|o| o.0 == prog);
            let visit = p.cal.visits.iter().filter(|v| v.0 == prog).max_by_key(|v| v.1.0);
            if offered {
                let official = p.cal.visits.iter().any(|v| v.0 == prog && v.2 == Visit::Official);
                return ("mine", if official { "Offered, official visit" } else { "Offered" }.into());
            }
            if let Some(v) = visit {
                let (_, m, _) = v.1.parts();
                return ("visited", format!("Visited in {}", MONTHS[(m as usize).clamp(1, 12) - 1]));
            }
            if super::desk::is_listed(w, p.player.id) {
                return ("visited", "Watching".into());
            }
            ("none", "Not contacted".into())
        }
        _ => {
            if let Some(s) = p.cal.signed {
                return ("away", format!("Signed with {}", elsewhere(s.program)));
            }
            if let Some(c) = p.cal.commit {
                return ("away", format!("Committed to {}", elsewhere(c.0)));
            }
            let n = p.cal.offers.len().max(p.cal.early_offers.len());
            match n {
                0 => ("none", "No offers yet".into()),
                1 => ("none", "1 offer".into()),
                n => ("none", format!("{n} offers")),
            }
        }
    }
}

/// What a player is best and worst at, in the words the position's own
/// ratings use: your own read of each rating where you have one, your
/// staff's otherwise, only those that matter at his spot.
fn strong_and_short(w: &World, k: usize) -> (Vec<&'static str>, Vec<&'static str>) {
    let s = reads::seen(w, k);
    let pos = w.people[k].player.position;
    // The weights are shares of his position's rating, so "matters" is
    // judged against the position's biggest one.
    let top = Attr::ALL.iter().map(|&a| file::matters(pos, a)).fold(0.0f64, f64::max);
    let mut rated: Vec<(f64, Attr)> = Attr::ALL
        .iter()
        .filter(|a| file::matters(pos, **a) >= 0.4 * top && top > 0.0)
        .filter_map(|&a| {
            let r = s.mine_attrs.get(a.index()).copied().flatten().or(s.attrs.get(a.index()).copied().flatten())?;
            Some(((r.0 as f64 + r.1 as f64) / 2.0, a))
        })
        .collect();
    // Too few seen to say who is better at what.
    if rated.len() < 4 {
        return (Vec::new(), Vec::new());
    }
    rated.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.index().cmp(&b.1.index())));
    let best = rated.iter().take(2).map(|x| x.1.label()).collect();
    let worst = rated.iter().rev().take(1).map(|x| x.1.label()).collect();
    (best, worst)
}

/// The head coach's tenure: since when, and the man before him with his
/// years and record at the school.
fn coach_panel(w: &World, h: usize) -> Value {
    let s = &w.high_schools[h];
    let Some(c) = s.coach.and_then(|c| w.coaches.get(c)) else { return Value::Null };
    let since = c.job.as_ref().map(|j| j.first_season()).filter(|&y| y > 0);
    // Seasons the school kept, oldest first, grouped by who coached them.
    let mut tenures: Vec<(usize, i32, i32, u32, u32)> = Vec::new();
    for x in &s.seasons {
        let Some(who) = x.coach else { continue };
        match tenures.last_mut() {
            Some(t) if t.0 == who => {
                t.2 = x.year;
                t.3 += x.wins;
                t.4 += x.losses;
            }
            _ => tenures.push((who, x.year, x.year, x.wins, x.losses)),
        }
    }
    let before = tenures.iter().rev().find(|t| Some(t.0) != s.coach).and_then(|t| {
        w.coaches.get(t.0).map(
            |b| json!({ "id": b.id, "name": b.name, "from": t.1, "to": t.2, "record": format!("{}-{}", t.3, t.4) }),
        )
    });
    json!({
        "id": c.id, "name": c.name, "since": since, "before": before,
        "years": tenures.iter().rev().find(|t| Some(t.0) == s.coach).map(|t| t.2 - t.1 + 1),
        "record": tenures.iter().rev().find(|t| Some(t.0) == s.coach).map(|t| format!("{}-{}", t.3, t.4)),
    })
}

/// Where its players went: the colleges its last classes signed with, the
/// players a year it sends on, and every man of its own who reached the NFL.
fn placement(w: &World, h: usize, me: Option<TeamRef>) -> Value {
    // The senior class is this year's until the school year turns in August.
    let seniors = if w.today < w.dates.hs_school_start { w.year } else { w.year + 1 };
    let mine = match me {
        Some(TeamRef::College(p)) => Some(p),
        _ => None,
    };
    let mut by: HashMap<usize, u32> = HashMap::new();
    let (mut to_college, mut nfl) = (0u32, 0u32);
    let mut yours: Vec<(i32, usize)> = Vec::new();
    for (k, p) in w.people.iter().enumerate() {
        if p.hs != h {
            continue;
        }
        if p.pro.is_some() {
            nfl += 1;
        }
        if p.class_of > seniors || p.class_of <= seniors - CLASSES {
            continue;
        }
        let Some(dest) = p.path.first().copied().or(p.cal.signed.map(|s| s.program)) else { continue };
        *by.entry(dest).or_default() += 1;
        to_college += 1;
        if mine == Some(dest) {
            yours.push((p.class_of, k));
        }
    }
    let mut rows: Vec<(usize, u32)> = by.into_iter().collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut shown: Vec<Value> = rows
        .iter()
        .take(5)
        .map(|&(p, n)| json!({ "name": w.program_name(p), "id": p, "n": n, "yours": mine == Some(p) }))
        .collect();
    if let Some(m) = mine
        && !rows.iter().take(5).any(|r| r.0 == m)
        && let Some(r) = rows.iter().find(|r| r.0 == m)
    {
        shown.push(json!({ "name": w.program_name(m), "id": m, "n": r.1, "yours": true }));
    }
    yours.sort();
    json!({
        "classes": CLASSES,
        "rows": shown,
        "to_college": to_college,
        "per_year": ((to_college as f64 / CLASSES as f64) * 10.0).round() / 10.0,
        "nfl": nfl,
        "signed": (mine.is_some() && !yours.is_empty()).then(|| json!({
            "n": yours.len(),
            "list": yours.iter().rev().take(3).map(|&(y, k)| json!({ "id": w.people[k].player.id, "name": w.people[k].name, "year": y })).collect::<Vec<_>>(),
        })),
    })
}

/// The state poll's place for this school in the latest poll kept for its
/// state: all classes, else its class.
fn poll(w: &World, h: usize) -> Value {
    let st = w.high_schools[h].state;
    let Some(p) = w.state_polls().iter().rev().find(|p| p.state.is_none_or(|s| s as usize == st)) else {
        return Value::Null;
    };
    let place = |rows: &[afm_engine::world::charts::PollRow]| {
        rows.iter().find(|r| r.school as usize == h && r.rank <= 25).map(|r| r.rank)
    };
    let (rank, which) = match (place(&p.top), place(&p.class)) {
        (Some(r), _) => (r, "State poll"),
        (None, Some(r)) => (r, "Class poll"),
        _ => return Value::Null,
    };
    json!({ "rank": rank, "which": which, "year": p.year, "when": p.which })
}

/// The middle's data for one school.
pub fn books(w: &World, h: usize) -> Value {
    let t = TeamRef::HighSchool(h);
    let own = views::is_mine(w, t);
    let me = views::mine(w);
    // Everyone on the roster the page can judge, best read first.
    let mut men: Vec<(f64, usize, Read, &'static str)> = views::roster_people(w, t)
        .into_iter()
        .filter(|&k| matches!(w.people[k].stage, Stage::HighSchool | Stage::PrepSchool))
        .filter_map(|k| {
            let (r, whose) = judged(w, k, own);
            mid(r).map(|m| (m, k, r, whose))
        })
        .collect();
    men.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let prospect = |&(m, k, r, whose): &(f64, usize, Read, &'static str)| {
        let p = &w.people[k];
        let (kind, words) = stand(w, k, me);
        let (best, worst) = strong_and_short(w, k);
        json!({
            "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "class": views::class_text(w, p),
            "read": r.json(), "value": m.round(), "whose": whose,
            "stand": words, "kind": kind, "best": best, "worst": worst,
        })
    };
    let prospects: Vec<Value> = men.iter().take(3).map(prospect).collect();
    let rest: Vec<Value> = men.iter().skip(3).take(REST).map(prospect).collect();
    // Each class as a row of dots, placed by the read that judges.
    let by_class: Vec<Value> = ["Sr", "Jr", "So", "Fr"]
        .iter()
        .map(|&c| {
            let dots: Vec<Value> = men
                .iter()
                .filter(|x| views::class_text(w, &w.people[x.1]) == c)
                .map(|x| {
                    let p = &w.people[x.1];
                    let (kind, words) = stand(w, x.1, me);
                    json!({ "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "v": x.0.round(), "kind": kind, "stand": words })
                })
                .collect();
            json!({ "class": c, "dots": dots })
        })
        .filter(|c| c["dots"].as_array().is_some_and(|d| !d.is_empty()))
        .collect();
    json!({
        "prospects": prospects,
        "classes": by_class,
        "rest": rest,
        "judged": men.len(),
        "college_seat": matches!(me, Some(TeamRef::College(_))),
        "places": placement(w, h, me),
        "poll": poll(w, h),
        "coach": coach_panel(w, h),
        "mine_school": own,
    })
}
