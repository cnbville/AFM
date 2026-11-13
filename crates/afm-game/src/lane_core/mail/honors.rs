//! Honors as messages (the inbox lane, rows 55, 56 and 134 of the event
//! table): the Heisman finalists, the winner, the awards show, the NFL's
//! awards and the All-America teams, told as they concern your own players
//! and stamped at the hour they came out. Nothing here raises a world event:
//! the finalists, the winner, the show and the NFL's awards are calendar
//! lines (`Kind::Award`, and `Kind::News` at the NFL's level); the All-America
//! line names nobody, so its men are read from the honors book
//! (`w.stats.honors`).

use std::collections::HashSet;

use afm_engine::calendar::{Date, Moment};
use afm_engine::world::World;

use super::super::links;
use super::super::people::Job;
use super::{MEDIA, PLAYERS, Pen, act, answer, ids_in, strip_dot};
use crate::views::{self, TeamRef};

/// One honors line the day's events gave, linked, with the Eastern moment
/// the clock booked it and whether it is about your own team.
pub(super) struct Line {
    pub text: String,
    pub when: Moment,
    pub mine: bool,
}

/// The kind of line, which decides the memo it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Group {
    Finalists,
    Winner,
    Show,
    Nfl,
    Other,
}

impl Group {
    fn of(text: &str) -> Group {
        let plain = links::strip(text);
        if plain.starts_with("Heisman finalists") {
            Group::Finalists
        } else if plain.contains("wins the Heisman") {
            Group::Winner
        } else if plain.starts_with("College football awards:") {
            Group::Show
        } else if plain.contains(" is the NFL's ")
            || plain.contains(" is the AP's ")
            || plain.contains("Super Bowl MVP")
        {
            Group::Nfl
        } else {
            Group::Other
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Group::Finalists => "finalists",
            Group::Winner => "heisman",
            Group::Show => "show",
            Group::Nfl => "nfl",
            Group::Other => "other",
        }
    }
}

/// Splits a list of items on `sep` outside parentheses and link markup
/// ("Name (QB, School), Other (RB, School)").
fn split_items(text: &str, sep: &str) -> Vec<String> {
    let mut out = Vec::new();
    let (mut depth, mut start) = (0i32, 0usize);
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < text.len() {
        match bytes[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            _ if depth <= 0 && text.is_char_boundary(i) && text[i..].starts_with(sep) => {
                out.push(text[start..i].trim().to_string());
                i += sep.len();
                start = i;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    let last = text[start..].trim().trim_end_matches('.').trim();
    if !last.is_empty() {
        out.push(last.to_string());
    }
    out
}

/// A name from the front of an item: "John Smith (QB, State)" is "John Smith".
fn lead_name(item: &str) -> String {
    let plain = links::strip(item);
    plain.split(" (").next().unwrap_or(&plain).trim().to_string()
}

/// A moment local to your seat, a quarter hour after the line came out; a
/// line with no hour is told in the afternoon.
fn stamp(w: &World, line: &Line) -> (Date, u16) {
    if line.when.timed() {
        let at = w.local(line.when.add_minutes(15));
        (at.date, at.minute)
    } else {
        (line.when.date, 17 * 60)
    }
}

fn first_name(w: &World, id: u32) -> String {
    views::person_index(w, id).map_or_else(String::new, |k| w.people[k].name.clone())
}

/// The honors memos of one day. `lines` are the day's honors lines.
pub(super) fn memos(pen: &mut Pen, lines: Vec<Line>, roster: &HashSet<u32>) {
    let w = pen.w;
    let name = views::team_name(w, pen.me);
    let mut groups: Vec<(Group, Vec<&Line>)> = Vec::new();
    for l in &lines {
        let g = Group::of(&l.text);
        match groups.iter_mut().find(|x| x.0 == g) {
            Some(x) => x.1.push(l),
            None => groups.push((g, vec![l])),
        }
    }
    for (group, ls) in groups {
        let when = ls.iter().max_by_key(|l| l.when).copied().unwrap_or(ls[0]);
        let (date, minute) = stamp(w, when);
        // The part of each line that concerns you: your men, or the whole
        // line when it is about your team.
        let mut ours: Vec<String> = Vec::new();
        let mut rest: Vec<String> = Vec::new();
        let mut ids: Vec<u32> = Vec::new();
        let mut lead = String::new();
        for l in &ls {
            let theirs = |s: &str| ids_in(s).into_iter().any(|i| roster.contains(&i));
            match group {
                Group::Finalists | Group::Show => {
                    let body = l.text.find(": ").map_or(l.text.as_str(), |i| &l.text[i + 2..]);
                    let sep = if group == Group::Finalists { ", " } else { "; " };
                    for item in split_items(body, sep) {
                        if theirs(&item) {
                            ids.extend(ids_in(&item).into_iter().filter(|i| roster.contains(i)));
                            ours.push(item);
                        } else {
                            rest.push(item);
                        }
                    }
                }
                _ => {
                    if theirs(&l.text) || l.mine {
                        ids.extend(ids_in(&l.text).into_iter().filter(|i| roster.contains(i)));
                        ours.push(l.text.clone());
                    }
                }
            }
            if lead.is_empty() {
                lead = l.text.clone();
            }
        }
        if ours.is_empty() {
            continue;
        }
        ids.sort_unstable();
        ids.dedup();
        let who = ours.first().map(|s| lead_name(s)).unwrap_or_default();
        let subject = match group {
            Group::Finalists if ours.len() == 1 => format!("Honors: {who} is a Heisman finalist"),
            Group::Finalists => format!("Honors: {} of ours are Heisman finalists", ours.len()),
            Group::Show if ours.len() == 1 => format!("Honors: {}", strip_dot(&links::strip(&ours[0]))),
            Group::Show => format!("Honors: {} of ours at the awards show", ours.len()),
            Group::Winner => {
                // The winner leads the line; a roster man in second is told so.
                let first = ids_in(&ours[0]).first().copied();
                match first {
                    Some(id) if roster.contains(&id) => {
                        format!("Honors: {} wins the Heisman Trophy", first_name(w, id))
                    }
                    _ => match ids.first() {
                        Some(&id) => format!("Honors: {} finishes high in the Heisman voting", first_name(w, id)),
                        None => format!("Honors: {}", strip_dot(&links::strip(&ours[0]))),
                    },
                }
            }
            Group::Nfl | Group::Other => format!("Honors: {}", strip_dot(&links::strip(&ours[0]))),
        };
        let mut m = pen.memo(&format!("awards|{}|{name}", group.tag()), date, minute, Job::Communications, subject);
        m.body = vec![format!("{}, good news to share with the team:", pen.dear)];
        match group {
            Group::Finalists => {
                m.body.push(format!(
                    "{} {} among the Heisman finalists: {}.",
                    if ours.len() == 1 { "One of ours is" } else { "Some of ours are" },
                    "named",
                    ours.join(", ")
                ));
                if !rest.is_empty() {
                    m.body.push(format!("The rest of the field: {}.", rest.join(", ")));
                }
            }
            Group::Show => {
                m.body.extend(ours.iter().map(|s| format!("{s}.")));
                if !rest.is_empty() {
                    m.body.push(format!("Also honored: {}.", rest.join("; ")));
                }
            }
            _ => m.body.extend(ours.iter().cloned()),
        }
        m.body.push("We will put it out on our channels.".into());
        m.keys = PLAYERS | MEDIA;
        m.count = ours.len() as u32;
        m.loud = if group == Group::Winner || links::strip(&ours[0]).contains("MVP") { 3 } else { 2 };
        if !ids.is_empty() {
            m.answers = vec![
                answer(
                    "Recognize him in front of the team",
                    "You make it a moment at the next team meeting. His mood rises.",
                    act::CALL_ALL,
                    0,
                ),
                answer("Send a short note", "A note goes to him. His mood rises a little.", act::CHEER_ALL, 0),
            ];
        }
        m.ids = ids;
        pen.out.push(m);
    }
}

/// The All-America teams are out (`when`: the booked moment of the line, a
/// noon Eastern step): your first-team men, read from the honors book. The
/// line itself names nobody.
pub(super) fn all_america(pen: &mut Pen, d: Date, when: Moment) {
    let w = pen.w;
    let TeamRef::College(pi) = pen.me else { return };
    let roster: HashSet<u32> = views::roster_people(w, pen.me).iter().map(|&k| w.people[k].player.id).collect();
    let mut rows: Vec<(String, u32, String, String)> = Vec::new();
    for (&id, h) in &w.stats.honors {
        let Some(&(year, division)) = h.all_america.last() else { continue };
        if year != d.year() {
            continue;
        }
        let Some(k) = views::person_index(w, id) else { continue };
        let p = &w.people[k];
        // A man who has already declared is not on the roster list but
        // still played for the school this year.
        if !roster.contains(&id) && p.college != Some(pi) {
            continue;
        }
        rows.push((p.name.clone(), id, p.player.position.name().to_string(), division.name().to_string()));
    }
    if rows.is_empty() {
        return;
    }
    rows.sort();
    let at = if when.timed() { w.local(when.add_minutes(60)) } else { Moment { date: d, minute: 17 * 60 } };
    let subject = match rows.as_slice() {
        [one] => format!("All-America: {}", one.0),
        many => format!("All-America: {} of our players", many.len()),
    };
    let mut m = pen.memo(
        &format!("awards|america|{}", views::team_name(w, pen.me)),
        at.date,
        at.minute,
        Job::Communications,
        subject,
    );
    m.body = vec![format!("{}, the All-America teams are out. First team from our roster:", pen.dear)];
    m.table = vec![vec!["Player".into(), "Pos".into(), "Level".into()]];
    for (name, id, pos, division) in &rows {
        m.table.push(vec![links::player(*id, name), pos.clone(), division.clone()]);
    }
    m.ids = rows.iter().map(|r| r.1).collect();
    m.count = rows.len() as u32;
    m.keys = PLAYERS | MEDIA;
    m.loud = 2;
    m.answers = vec![
        answer(
            "Recognize them in front of the team",
            "You make it a moment at the next team meeting. Their mood rises.",
            act::CALL_ALL,
            0,
        ),
        answer("Send a short note", "A note goes to each of them. Their mood rises a little.", act::CHEER_ALL, 0),
    ];
    pen.out.push(m);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_splits_outside_parentheses() {
        let got = split_items("Ann Lee (QB, State), Bo Ray (RB, Tech), Cy Fox (WR, A&M).", ", ");
        assert_eq!(got, ["Ann Lee (QB, State)", "Bo Ray (RB, Tech)", "Cy Fox (WR, A&M)"]);
        let show = split_items("Davey O'Brien Award Ann Lee (State); Doak Walker Award Bo Ray (Tech).", "; ");
        assert_eq!(show, ["Davey O'Brien Award Ann Lee (State)", "Doak Walker Award Bo Ray (Tech)"]);
    }

    #[test]
    fn a_line_belongs_to_the_group_its_words_name() {
        assert_eq!(Group::of("Heisman finalists: A (QB, S)"), Group::Finalists);
        assert_eq!(Group::of("A of S wins the Heisman Trophy with 900 points."), Group::Winner);
        assert_eq!(Group::of("College football awards: Doak Walker Award A (S)."), Group::Show);
        assert_eq!(Group::of("Pat Moe is the NFL's MVP."), Group::Nfl);
        assert_eq!(Group::of("Sam Lee of the Bears is the AP's Coach of the Year."), Group::Nfl);
        assert_eq!(Group::of("Something else."), Group::Other);
    }

    #[test]
    fn a_name_is_the_front_of_an_item() {
        assert_eq!(lead_name("[[p:5|Ann Lee]] (QB, [[t:college:2|State]])"), "Ann Lee");
    }
}
