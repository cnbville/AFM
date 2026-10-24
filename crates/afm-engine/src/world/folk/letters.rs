//! The people's mail (the final build's inbox, 10a-10i): a meeting or a
//! call that comes to you arrives as a letter from the person who would
//! tell you (your boss on the school's letterhead, your communications
//! director's memo about the press conference, a parent's own email), with
//! a link that opens the scene. Scenes you start yourself send nothing.

use super::Team;
use super::cast::{self, CastRef, Role};
use super::scene::{Kind, Scene, Who};
use crate::world::World;
use crate::world::desk::{Head, Mail};

/// The inbox's keyword bits (the game crate's `KEYWORDS`).
const JOB: u16 = 1;
const PLAYERS: u16 = 1 << 1;
const INJURIES: u16 = 1 << 4;
const STAFF: u16 = 1 << 8;
const PEOPLE: u16 = 1 << 9;
const MEDIA: u16 = 1 << 10;

/// The team's code the inbox prints colours from.
fn team_code(t: Team) -> u32 {
    match t {
        Team::Nfl(i) => i as u32,
        Team::College(i) => 100_000 + i as u32,
        Team::High(i) => 1_000_000 + i as u32,
        Team::Other(i) => 10_000_000 + i as u32,
    }
}

fn person_of(w: &World, team: Team, role: Role) -> Option<cast::Cast> {
    cast::has(w, team, role).then(|| cast::get(w, CastRef::new(team, role, 0)))
}

/// Files the letter for a scene that comes to you.
pub(crate) fn for_scene(w: &mut World, s: &Scene) {
    let Some(team) = s.team.or(Team::seat(w)) else { return };
    if w.career.is_none() || w.folk.hush {
        return;
    }
    let nfl = matches!(team, Team::Nfl(_));
    let you = super::scene::your_name(w);
    let to = format!("{you}, Head Coach");
    let org = team.name(w);
    let open = format!("[[s:people_scene?id={}|{}]]", s.id, if s.stops { "Open the meeting" } else { "Open it" });
    let when = if s.date > w.today { format!(" on {}", s.date.pretty()) } else { String::new() };
    let player = s.who.iter().find_map(|x| match x {
        Who::Player(id) => Some(*id),
        _ => None,
    });
    let plink = player.and_then(|id| super::person(w, id)).map(|i| super::player_link(w, i));
    // The staff member who'd write the memo: operations at a college or a
    // club, your coordinator at a high school.
    let ops = || -> (String, String) {
        let r = if nfl { Role::ChiefOfStaff } else { Role::Ops };
        match person_of(w, team, r) {
            Some(c) => (c.name.clone(), c.title.clone()),
            None => ("The football office".to_string(), "Staff".to_string()),
        }
    };
    let (head, from, role, subject, mut body, keys): (Head, String, String, String, Vec<String>, u16) = match s.kind {
        Kind::Press | Kind::PressIntro => {
            let (n, t) = match person_of(w, team, Role::Comms) {
                Some(c) => (c.name.clone(), c.title.clone()),
                None => ops(),
            };
            (
                Head::Memo,
                n,
                t,
                format!("{}{when}", s.title),
                vec![
                    format!("{}{when}. The beat writers will be there; I'll send the likely questions over before you go up.", s.place),
                    "If you'd rather not take it, I'll put out a statement and the coordinators can cover.".into(),
                ],
                MEDIA,
            )
        }
        Kind::BossMeeting | Kind::BossFirst => {
            if s.c == 3 {
                return;
            }
            let b = super::boss::boss(w, team);
            (
                if nfl { Head::Club } else { Head::School },
                b.name.clone(),
                b.title.clone(),
                s.title.clone(),
                vec![format!(
                    "Coach, {} I'd like to sit down{when}. {}",
                    match s.c {
                        1 => "we're at the midpoint and",
                        2 => "the season's over and",
                        _ => "before the season gets away from us",
                    },
                    if s.place.is_empty() { "My office.".to_string() } else { format!("{}.", s.place) }
                )],
                JOB,
            )
        }
        Kind::Discipline => {
            let (n, t) = ops();
            (Head::Memo, n, t, s.title.clone(), vec![format!("{} {}", plink.clone().unwrap_or_default(), "broke one of your rules. He's waiting to hear from you before practice.")], PLAYERS | PEOPLE)
        }
        Kind::SecondOpinion => {
            let (n, t) = match person_of(w, team, Role::Trainer) {
                Some(c) => (c.name.clone(), c.title.clone()),
                None => ops(),
            };
            (
                Head::Memo,
                n,
                t,
                s.title.clone(),
                vec![format!(
                    "{}'s camp wants an outside specialist to look at the injury before we go ahead with our plan. I'd like your call on it.",
                    plink.clone().unwrap_or_else(|| "A player".into())
                )],
                INJURIES | PLAYERS,
            )
        }
        Kind::StarPower => {
            let (n, t) = ops();
            (Head::Memo, n, t, s.title.clone(), vec![format!("{} wants a word with you, and the room knows it.", plink.clone().unwrap_or_else(|| "One of your stars".into()))], PLAYERS | PEOPLE)
        }
        Kind::ParentCall => {
            let name = player.and_then(|id| super::person(w, id)).map(|i| {
                let n = &w.people[i].name;
                let last = n.split_whitespace().rfind(|x| !matches!(*x, "Jr." | "II" | "III" | "IV")).unwrap_or(n);
                format!("The {last} family")
            });
            (
                Head::Email,
                name.unwrap_or_else(|| "A parent".into()),
                "Parent".into(),
                "About our son".into(),
                vec![format!("Coach, do you have a few minutes this week? We'd like to talk about {}.", plink.clone().unwrap_or_else(|| "our son".into()))],
                PLAYERS | PEOPLE,
            )
        }
        Kind::Proposal | Kind::StaffLine => {
            let c = s.who.iter().find_map(|x| match x {
                Who::Coach(c) => Some(*c),
                _ => None,
            });
            let (n, t) = c.map_or_else(ops, |c| (w.coaches[c].name.clone(), super::takeover::cap_first(&super::facts::coach_role(w, c))));
            (Head::Memo, n, t, s.title.clone(), vec!["Got a minute today? Something I want to run by you.".into()], STAFF)
        }
        Kind::JobCall => (
            Head::Email,
            "A search firm".into(),
            "Executive search".into(),
            "A confidential conversation".into(),
            vec!["Coach, I represent a program that would like to talk with you. Discretion is assured. Can you take a call?".into()],
            JOB,
        ),
        Kind::Ritual => {
            let (n, t) = ops();
            (Head::Memo, n, t, s.title.clone(), vec!["The veterans have their plans for the rookies. You'll want to set the limits before they do.".into()], PLAYERS)
        }
        Kind::Family => (Head::Email, "Home".into(), "Family".into(), s.title.clone(), vec!["Call when you get a minute. It's not football.".into()], PEOPLE),
        Kind::OneOnOne if s.stops => {
            let (n, t) = ops();
            (Head::Memo, n, t, s.title.clone(), vec![format!("{} asked for time with you{when}.", plink.clone().unwrap_or_else(|| "A player".into()))], PLAYERS)
        }
        Kind::Parents => {
            let (n, t) = ops();
            (Head::Memo, n, t, s.title.clone(), vec![format!("The parents' meeting is set{when}. {}", s.place)], PEOPLE)
        }
        _ => return,
    };
    body.push(format!("{open}."));
    let id = 0xF01C_0000_0000_0000 ^ s.id as u64;
    let c = w.career.as_mut().expect("career");
    if c.desk.has(id) {
        return;
    }
    let org_line = match team {
        Team::Nfl(_) | Team::Other(_) => "Football operations".to_string(),
        Team::College(_) => "Department of Athletics".to_string(),
        Team::High(_) => "Football office".to_string(),
    };
    c.desk.mails.push(Mail {
        id,
        date: w.today,
        minute: 7 * 60 + 20 + (s.id % 50) as u16,
        head,
        org,
        org_line,
        from: from.clone(),
        role,
        to,
        subject,
        body,
        table: Vec::new(),
        attach: Vec::new(),
        sign: from,
        keys,
        about: player.map(|id| (0, id)),
        answers: Vec::new(),
        answered: None,
        answer_by: None,
        read: false,
        saved: false,
        loud: if s.stops { 2 } else { 1 },
        ids: player.into_iter().collect(),
        org_team: Some(team_code(team)),
    });
    c.desk.mails.sort_by_key(|m| (m.date, m.minute));
}

/// The takeover's first two weeks as the welcome letter's table: every
/// meeting with its day, a link to it and where it is (header row first).
/// Empty when there is no takeover on file.
pub fn takeover_schedule(w: &World) -> Vec<Vec<String>> {
    let Some(t) = &w.folk.takeover else { return Vec::new() };
    let mut rows: Vec<Vec<String>> = vec![vec!["Day".into(), "Meeting".into(), "Where".into()]];
    for id in &t.scenes {
        if let Some(s) = w.folk.scenes.iter().find(|s| s.id == *id) {
            rows.push(vec![s.date.pretty(), format!("[[s:people_scene?id={}|{}]]", s.id, s.title), s.place.clone()]);
        }
    }
    if rows.len() == 1 { Vec::new() } else { rows }
}
