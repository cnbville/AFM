//! The portal in your inbox (the inbox lane, Cain 2026-11-12): what used
//! to be one small line on the Continue card and one in the morning clips
//! comes as messages with the names matched.
//!
//! - "Portal day": everyone who entered elsewhere that day, as one message:
//!   the few who fit what the needs map calls thin or a hole, then the
//!   starters and 4 or 5 stars, and a count for the rest.
//! - "Portal targets": the players you chased who decided.
//! - "Portal landings": where the day's best landed, and where your own
//!   former players ended up.
//! - the window's close, with the totals.
//!
//! Nothing here raises a world event: every message reads what the world
//! already holds (the entrants' portal dates, the calendar's lines, your
//! board, the needs map), so the Continue card and the clips leave out the
//! lines these messages carry (`covered`).

use afm_engine::calendar::{Date, Event, Kind};
use afm_engine::player::PlayingTime;
use afm_engine::world::norms::Rated;
use afm_engine::world::{Stage, World};

use super::super::links;
use super::super::people::Job;
use super::{ABOUT_PLAYER, PLAYERS, PORTAL, Pen, RECRUITING, act, answer};
use crate::reads::{self, Read};
use crate::views::{self, TeamRef};

/// Names that fit in each group of the portal-day message.
const GROUP: usize = 10;

/// Lines these messages carry in full: the Continue card and the morning
/// clips leave them out for a college seat.
pub fn covered(e: &Event) -> bool {
    let t = e.text.as_str();
    match e.kind {
        // Anyone's starter or 4-5 star entering, or landing: the portal
        // day's and the landings' messages.
        Kind::Transfer => t.ends_with("enters the transfer portal.") || t.contains(" transfers from "),
        // Your player entering after he asked to see you first.
        Kind::Slot => e.rule == "people: portal" && t.ends_with("entered the transfer portal."),
        // Your desk's own lines: your players entering, your targets deciding.
        Kind::News | Kind::Deadline => e.rule == "Your front office" && (own_entry(t) || target_line(t)),
        _ => false,
    }
}

/// "X entered the transfer portal." or "N of your players entered ...".
fn own_entry(t: &str) -> bool {
    t.ends_with("entered the transfer portal.") || t.contains(" of your players entered the transfer portal")
}

/// A portal target deciding: lands with you, chooses elsewhere, says yes.
pub fn target_line(t: &str) -> bool {
    t.contains(" transfers to you from ") || t.contains(" over your offer.") || t.contains("He signs tonight")
}

/// A staff read as "74–80" ("–" while nobody has seen him).
fn range(r: Read) -> String {
    match r {
        // A word joiner after the dash keeps a range on one line.
        Read::Range(lo, hi) => format!("{lo}\u{2013}\u{2060}{hi}"),
        Read::NotSeen => "\u{2013}".into(),
    }
}

fn starter(w: &World, k: usize) -> bool {
    w.people[k].last_role == Some(PlayingTime::Starter)
}

/// How the desk sorts entrants: stars first, a starter ahead of a backup.
fn rank(w: &World, k: usize) -> i32 {
    w.people[k].stars as i32 * 10 + if starter(w, k) { 30 } else { 0 }
}

fn school(w: &World, k: usize) -> String {
    w.people[k].college.map_or_else(String::new, |c| links::team(w, TeamRef::College(c)))
}

/// One day's entrants at other schools, as one message (`days` holds
/// each entry day with the people who entered on it).
pub(super) fn day_messages(pen: &mut Pen, pi: usize, days: Vec<(Date, Vec<usize>)>) {
    use crate::lane_frontoffice::college::seasons_ahead;
    if days.is_empty() {
        return;
    }
    let w = pen.w;
    let me = TeamRef::College(pi);
    let weak = crate::lane_squadboards::needs::weak_positions(w);
    let div = w.programs[pi].division;
    for (d, mut ks) in days {
        let n = ks.len();
        ks.sort_by_key(|&k| (std::cmp::Reverse(rank(w, k)), w.people[k].player.id));
        // Who fits: he plays a position the needs map calls thin or a hole,
        // has a season to play for us and, by your staff's read, would be
        // at least halfway from an average backup to an average starter there.
        let mut fits: Vec<(usize, Read, String)> = Vec::new();
        for &k in &ks {
            let p = &w.people[k];
            let Some((spot, verdict)) = weak.get(p.player.position.name()) else { continue };
            if seasons_ahead(w, p, div) == 0 {
                continue;
            }
            let read = reads::overall(w, k);
            let Some(mid) = read.mid() else { continue };
            let Some(m) = crate::lane_squadboards::marks(w, me, p.player.position, Rated::Overall) else { continue };
            if mid < (m.backup + m.starter) / 2.0 {
                continue;
            }
            let state = if *verdict == "Hole" { "has a hole" } else { "is thin" };
            let role = p.last_role.map(|r| views::role_words(r).to_lowercase());
            let why = match role {
                Some(r) => format!("Need: our {spot} {state}. He was a {r}."),
                None => format!("Need: our {spot} {state}."),
            };
            fits.push((k, read, why));
        }
        let stars: Vec<usize> = ks.iter().copied().filter(|&k| w.people[k].stars >= 4 || starter(w, k)).collect();
        if fits.is_empty() && stars.is_empty() {
            continue;
        }
        let fit_n = fits.len();
        fits.truncate(GROUP);
        let fit_ids: Vec<usize> = fits.iter().map(|f| f.0).collect();
        let shown_stars: Vec<usize> = stars.iter().copied().filter(|k| !fit_ids.contains(k)).take(GROUP).collect();
        let subject = if fit_n > 0 {
            format!("Portal day: {n} entered, {fit_n} fit what we need")
        } else {
            format!("Portal day: {n} entered, {} starters or 4-5 stars", stars.len())
        };
        let mut m = pen.memo(&format!("portalday|{}", d.0), d, 19 * 60 + 5, Job::Personnel, subject);
        m.body = vec![format!(
            "{}, {n} {} entered the transfer portal today. Your staff sorted out the ones that matter, with its read of each. Every name opens his page.",
            pen.dear,
            if n == 1 { "player" } else { "players" }
        )];
        m.table = vec![vec![
            "Player".into(),
            "Pos".into(),
            "Class".into(),
            "From".into(),
            "Stars".into(),
            "Our read".into(),
            "Why".into(),
        ]];
        let row = |k: usize, read: Read, why: String| {
            let p = &w.people[k];
            vec![
                links::player(p.player.id, &p.name),
                p.player.position.name().into(),
                views::class_text(w, p),
                school(w, k),
                if p.stars > 0 { format!("{}\u{2605}", p.stars) } else { "\u{2013}".into() },
                range(read),
                why,
            ]
        };
        if !fits.is_empty() {
            m.table.push(vec![format!(
                "## Fits what we need \u{b7} {} of {fit_n} who would help, best first",
                fits.len()
            )]);
            for (k, read, why) in fits {
                m.table.push(row(k, read, why));
            }
        }
        if !shown_stars.is_empty() {
            m.table.push(vec![format!(
                "## Starters and 4 or 5 stars elsewhere \u{b7} {} of {}",
                shown_stars.len(),
                stars.iter().filter(|k| !fit_ids.contains(k)).count()
            )]);
            for &k in &shown_stars {
                let p = &w.people[k];
                let why = match (p.stars, starter(w, k)) {
                    (s, true) if s >= 4 => format!("{s}\u{2605} starter"),
                    (_, true) => "Starter".into(),
                    (s, false) => format!("{s}\u{2605}, off the bench"),
                };
                m.table.push(row(k, reads::overall(w, k), why));
            }
        }
        let named = fit_ids.len() + shown_stars.len();
        m.body.push(format!(
            "{} more {} entered. [[s:portal_desk|Open the portal desk]] for all of them, with your staff's reads.",
            n.saturating_sub(named),
            if n.saturating_sub(named) == 1 { "player" } else { "players" }
        ));
        m.ids = fit_ids.iter().chain(shown_stars.iter()).map(|&k| w.people[k].player.id).collect();
        m.count = n as u32;
        m.keys = PORTAL;
        m.answers = Vec::new();
        if !fit_ids.is_empty() {
            m.answers.push(answer(
                &format!("Add the {} who fit to my players", fit_ids.len()),
                "They go on your list, so you see where they land.",
                act::LIST_FIRST,
                fit_ids.len() as u32,
            ));
        }
        if m.ids.len() > fit_ids.len() {
            m.answers.push(answer(
                &format!("Add all {} named to my players", m.ids.len()),
                "Every name in the table goes on your list.",
                act::LIST_ALL,
                0,
            ));
        }
        m.answers.push(answer("Nothing for now", "Nothing changes; the portal desk has them all.", act::ACK, 0));
        m.loud = 1;
        pen.out.push(m);
    }
}

/// The players you chased who decided that day (`lines` are the front
/// office's own lines, `names` the players your board and portal list hold).
pub(super) fn target_message(pen: &mut Pen, d: Date, lines: &[String], names: &[(u32, String)]) {
    if lines.is_empty() {
        return;
    }
    let mut m = pen.memo(
        &format!("portaltargets|{}", d.0),
        d,
        21 * 60 + 30,
        Job::Personnel,
        if lines.len() == 1 {
            "Portal target decided".to_string()
        } else {
            format!("Portal targets: {} decided", lines.len())
        },
    );
    m.body = vec![format!("{}, the portal players we chased decided:", pen.dear)];
    for l in lines {
        let mut text = l.clone();
        for (id, name) in names {
            if text.contains(name.as_str()) && !text.contains("[[p:") {
                text = text.replacen(name.as_str(), &links::player(*id, name), 1);
                m.ids.push(*id);
            }
        }
        m.body.push(text);
    }
    m.count = lines.len() as u32;
    m.keys = PORTAL | RECRUITING;
    m.loud = 2;
    pen.out.push(m);
}

/// Where your own former portal players ended up (a landing, a return,
/// or the end of his football), the first time it can be read; and where
/// the day's other starters and 4-5 stars landed (`lines`, linked).
pub(super) fn landings_message(
    pen: &mut Pen,
    pi: usize,
    d: Date,
    today: Date,
    mut lines: Vec<String>,
    seen: &mut Vec<u32>,
) {
    let w = pen.w;
    let Some(c) = &w.career else { return };
    // `left` starts over each football year, so only a telling of the last
    // 250 days counts: a man who enters again next winter is told again.
    let told = |id: u32| c.desk.landed.iter().any(|&(i, on)| i == id && on < d && d.0 - on.0 <= 250);
    let mut mine: Vec<(usize, String)> = Vec::new();
    // Today's word on your own: they are read from where they are now, so
    // only the last day of a stretch can tell it. A day written again at a
    // later stop (its letter was filed before the day was over) keeps the
    // men it already told: the rewrite replaces the letter's body, so
    // leaving them out dropped their line once the day's other landings
    // grew it.
    let left = &w.front_office.college.left;
    let ids: Vec<u32> = if d == today {
        left.clone()
    } else {
        c.desk.landed.iter().filter(|&&(i, on)| on == d && left.contains(&i)).map(|&(i, _)| i).collect()
    };
    for &id in &ids {
        // `seen`: told on an earlier day of this same stretch (not filed yet).
        if told(id) || seen.contains(&id) {
            continue;
        }
        let Some(k) = views::person_index(w, id) else { continue };
        let p = &w.people[k];
        if p.cal.portal.is_some() {
            continue;
        }
        let name = links::player(id, &p.name);
        let line = match (p.stage, p.college) {
            (Stage::Done, _) => format!("{name} ({}) found no school and has left football.", p.player.position),
            (Stage::College, Some(j)) if j == pi => {
                format!("{name} ({}) is back with us: no school took him.", p.player.position)
            }
            (Stage::College, Some(j)) => {
                format!("{name} ({}) lands at {}.", p.player.position, links::team(w, TeamRef::College(j)))
            }
            _ => continue,
        };
        mine.push((k, line));
    }
    // A man of ours who is also a starter or a star has a line of his own in
    // the day's events: he is told once, as ours.
    let ours: Vec<u32> = mine.iter().map(|&(k, _)| w.people[k].player.id).collect();
    lines.retain(|l| !linked_ids(l).iter().any(|i| ours.contains(i)));
    if mine.is_empty() && lines.is_empty() {
        return;
    }
    let subject = match (mine.len(), lines.len()) {
        (0, n) => format!("Portal landings: {n} starters and stars found a school"),
        (m, 0) => format!("Portal landings: {m} of our former players"),
        (m, n) => format!("Portal landings: {m} of ours and {n} others"),
    };
    // The word on my own men is read from where they are now, so it can only
    // be told at the stop that reads it: stamped at the clock, not the evening,
    // or a stretch that keeps ending before 8 PM would hold it for ever.
    let stamp = if mine.is_empty() || d != today {
        20 * 60
    } else {
        let now = w.local_now();
        if now.date == d { now.minute.min(20 * 60) } else { 20 * 60 }
    };
    let mut msg = pen.memo(&format!("portallandings|{}", d.0), d, stamp, Job::Personnel, subject);
    msg.body = vec![format!("{}, the portal's decisions today.", pen.dear)];
    if !mine.is_empty() {
        msg.body.push("Our former players:".into());
        msg.body.extend(mine.iter().map(|(_, l)| l.clone()));
    }
    if !lines.is_empty() {
        msg.body.push("Starters and 4 or 5 stars elsewhere:".into());
        let more = lines.len().saturating_sub(25);
        msg.body.extend(lines.iter().take(25).cloned());
        if more > 0 {
            msg.body.push(format!("{more} more landed. [[s:portal_desk|The portal desk]] has them all."));
        }
    }
    // The men the lines name count as told too, so a later stop that reads
    // them as ours (they left us) does not tell it again.
    msg.ids = ours.iter().copied().chain(lines.iter().flat_map(|l| linked_ids(l))).collect();
    seen.extend(msg.ids.iter().copied());
    msg.count = (mine.len() + lines.len()) as u32;
    msg.keys = PORTAL | PLAYERS;
    msg.about = ours.first().map(|&id| (ABOUT_PLAYER, id)).filter(|_| msg.ids.len() == 1);
    msg.loud = 1;
    pen.out.push(msg);
}

/// The players a linked line names (`[[p:id|Name]]`).
fn linked_ids(line: &str) -> Vec<u32> {
    line.split("[[p:").skip(1).filter_map(|rest| rest.split('|').next().and_then(|n| n.parse::<u32>().ok())).collect()
}

/// The closing day of a transfer window, with the totals.
pub(super) fn window_close(pen: &mut Pen, from: Date, to: Date) {
    let w = pen.w;
    for (open, close, name) in afm_engine::world::front_office::college::windows(w) {
        if close < from || close > to {
            continue;
        }
        let entered: u32 = w.log.portal_by_day.iter().filter(|x| x.0 >= open && x.0 <= close).map(|x| x.1).sum();
        let ours = w.front_office.college.left.len();
        let open_targets = w.front_office.college.portal.iter().filter(|t| !t.settled).count();
        let lower = name.to_lowercase();
        let mut m = pen.notice(&format!("portalclose|{}", close.0), close, format!("Notice: the {lower} closes today"));
        m.minute = 18 * 60;
        let mut body = vec![format!(
            "{}, the {lower} ({} to {}) closes today. {entered} players entered it{}.",
            pen.dear,
            open.pretty(),
            close.pretty(),
            if ours > 0 { format!(", {ours} of ours among the year's entrants") } else { String::new() }
        )];
        body.push(if open_targets > 0 {
            format!(
                "{open_targets} of your portal targets {} still to decide. They decide on their own days: [[s:portal_desk|the portal desk]] has where each stands.",
                if open_targets == 1 { "is" } else { "are" }
            )
        } else {
            "None of your portal targets is still waiting on a decision.".to_string()
        });
        m.body = body;
        m.keys |= PORTAL;
        m.loud = 1;
        pen.out.push(m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use afm_engine::calendar::{Level, Loudness};

    fn line(kind: Kind, rule: &'static str, text: &str) -> Event {
        let mut e = Event::new(Date::ymd(2027, 1, 5), Level::College, kind, Loudness::Small, text);
        e.rule = rule;
        e
    }

    #[test]
    fn the_lines_the_portal_messages_carry_leave_the_card_and_the_clips() {
        // Anyone's starter or blue-chip entering or landing.
        assert!(covered(&line(Kind::Transfer, "", "Jalen Okafor (EDGE, Riverbend) enters the transfer portal.")));
        assert!(covered(&line(Kind::Transfer, "", "Jalen Okafor transfers from Riverbend to Harlan Tech.")));
        // Your own players entering, after a talk or not.
        assert!(covered(&line(Kind::Slot, "people: portal", "Marcus Webb entered the transfer portal.")));
        assert!(covered(&line(
            Kind::News,
            "Your front office",
            "Marcus Webb (WR, playing time) entered the transfer portal."
        )));
        assert!(covered(&line(
            Kind::News,
            "Your front office",
            "3 of your players entered the transfer portal: A (WR, x), B (CB, y), C (QB, z)."
        )));
        // Your targets deciding.
        assert!(covered(&line(Kind::News, "Your front office", "Mateo Reyes (OT) transfers to you from Harlan Tech.")));
        assert!(covered(&line(Kind::News, "Your front office", "Mateo Reyes (OT) chose Riverbend over your offer.")));
        assert!(covered(&line(
            Kind::News,
            "Your front office",
            "Mateo Reyes (OT) commits to you: your offer gave him what he needed, and he's done shopping. He signs tonight."
        )));
        // What stays on the card: a talk he asked for, a visit, a decision, another school's news.
        assert!(!covered(&line(Kind::Slot, "people: portal", "Marcus Webb asks to see you before he decides.")));
        assert!(!covered(&line(Kind::News, "Your front office", "You sat with Mateo Reyes at home.")));
        assert!(!covered(&line(
            Kind::Deadline,
            "Your front office",
            "Your NIL deals pay on July 1: 3 players have nothing set."
        )));
        assert!(!covered(&line(Kind::Transfer, "", "Riverbend adds a transfer coach.")));
        assert!(!covered(&line(Kind::News, "", "Marcus Webb entered the transfer portal.")));
        assert!(target_line("Mateo Reyes (OT) chose Riverbend over your offer."));
        assert!(!target_line("Marcus Webb entered the transfer portal."));
    }

    #[test]
    fn a_linked_line_names_its_players() {
        assert_eq!(
            linked_ids("[[p:123|Jalen Okafor]] (EDGE) transfers from [[t:c4|Riverbend]] to [[t:c9|Harlan Tech]]."),
            [123]
        );
        assert_eq!(linked_ids("[[p:7|A]] and [[p:88|B]] land."), [7, 88]);
        assert!(linked_ids("No names here, [[s:portal_desk|the desk]].").is_empty());
    }
}
