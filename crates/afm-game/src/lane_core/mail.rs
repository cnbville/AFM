//! The inbox (final build 10a-10i): your staff, your boss, the league
//! offices, the papers and the people around you write to you, and each
//! mail is a document on its sender's letterhead.
//!
//! Mail is written once, the morning after the day it is about, from what
//! the world did that day (the calendar's lines, your games, the people
//! moving toward or away from your team) and kept on your desk for 30 days
//! (`afm_engine::world::desk`). What goes where (10h1-10h5):
//! - your own team's moments come as memos from the staff member who would
//!   tell you (a commitment from your personnel director, a game's notes
//!   from communications, a coaching change from your athletic director);
//! - your team and your rivals in the papers come as morning clips;
//! - the league's biggest news and the FBS conference moves come on the
//!   national paper's front page or from your conference office;
//! - windows and deadlines come from your compliance office (the league
//!   office at a club, the state association at a high school);
//! - the people you are linked to come in a weekly memo from your chief of
//!   staff; lower-division standouts as a portal watch list in the windows;
//! - other schools' hires and the rest of the league go to News, not here.
//!
//! Other lanes post their own mail with [`send`]: build a [`Mail`] with
//! [`letter`] and add answers made with [`answer`].

use std::collections::{HashMap, HashSet};

use afm_engine::calendar::{Date, Event, Kind, Level as CalLevel, Loudness, Weekday};
use afm_engine::positions::Position;
use afm_engine::world::coach::Group;
use afm_engine::world::desk::{Answer, Head, KEEP_DAYS, Mail, Reveal};
use afm_engine::world::news::NewsKind;
use afm_engine::world::records::Stat;
use afm_engine::world::{Stage, World};

use super::links::{self, Linker};
use super::people::{self, Job};
use crate::lane_help;
use crate::views::{self, TeamRef};

/// The keyword filter (10e), in the dropdown's order; a mail's `keys` holds
/// them as bits.
/// Days mail is kept (10d).
pub const KEEP: i32 = KEEP_DAYS;

pub const KEYWORDS: [&str; 12] = [
    "Your job",
    "Your players",
    "Recruiting",
    "Portal",
    "Injuries",
    "Money",
    "Spring practice",
    "Draft and NFL",
    "Staff",
    "Your people",
    "Media",
    "League",
];
pub const JOB: u16 = 1;
pub const PLAYERS: u16 = 1 << 1;
pub const RECRUITING: u16 = 1 << 2;
pub const PORTAL: u16 = 1 << 3;
pub const INJURIES: u16 = 1 << 4;
pub const MONEY: u16 = 1 << 5;
pub const SPRING: u16 = 1 << 6;
pub const DRAFT: u16 = 1 << 7;
pub const STAFF: u16 = 1 << 8;
pub const PEOPLE: u16 = 1 << 9;
pub const MEDIA: u16 = 1 << 10;
pub const LEAGUE: u16 = 1 << 11;

/// What a picked answer does (`Answer::act`).
pub mod act {
    /// Files the reply; nothing in the world changes.
    pub const ACK: u8 = 0;
    /// You call the player (target): his mood rises and his trust in you a little.
    pub const CALL: u8 = 1;
    /// A short note to the player (target): his mood rises a little.
    pub const NOTE: u8 = 2;
    /// Every player the mail names goes on "my players".
    pub const LIST_ALL: u8 = 3;
    /// The player (target) goes on "my players".
    pub const LIST_ONE: u8 = 4;
    /// Congratulations to everyone the mail names: each player's mood rises a little.
    pub const CHEER_ALL: u8 = 5;
    /// You call everyone the mail names: their mood rises, a call is logged.
    pub const CALL_ALL: u8 = 6;
    /// A thank-you to the coach (target, a coach index): logged as a contact.
    pub const THANK_COACH: u8 = 7;
    /// Gives your order at the positions in the target's bits back to the
    /// staff (H5).
    pub const DEPTH_BACK: u8 = 8;
}

/// What a mail is about, for `Mail::about` (the page opened beside it).
pub const ABOUT_PLAYER: u8 = 0;
pub const ABOUT_COACH: u8 = 1;
pub const ABOUT_TEAM: u8 = 2;
pub const ABOUT_GAME: u8 = 3;

/// A team as one number (for `about`).
pub fn team_code(t: TeamRef) -> u32 {
    match t {
        TeamRef::Nfl(i) => i as u32,
        TeamRef::College(i) => 100_000 + i as u32,
        TeamRef::HighSchool(i) => 1_000_000 + i as u32,
        TeamRef::Other(i) => 10_000_000 + i as u32,
    }
}

pub fn team_from(code: u32) -> TeamRef {
    match code {
        c if c >= 10_000_000 => TeamRef::Other((c - 10_000_000) as usize),
        c if c >= 1_000_000 => TeamRef::HighSchool((c - 1_000_000) as usize),
        c if c >= 100_000 => TeamRef::College((c - 100_000) as usize),
        c => TeamRef::Nfl(c as usize),
    }
}

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// A blank letter: the caller fills the body, keys and answers. `tag`
/// makes its id (the same tag on the same day is the same mail).
#[allow(clippy::too_many_arguments)]
pub fn letter(
    tag: &str,
    date: Date,
    minute: u16,
    head: Head,
    org: String,
    org_line: String,
    from: String,
    role: String,
    to: String,
    subject: String,
) -> Mail {
    Mail {
        id: fnv(&format!("{tag}|{}", date.0)),
        date,
        minute,
        head,
        org,
        org_line,
        from,
        role,
        to,
        subject,
        body: Vec::new(),
        table: Vec::new(),
        attach: Vec::new(),
        sign: String::new(),
        keys: 0,
        about: None,
        answers: Vec::new(),
        answered: None,
        answer_by: None,
        read: false,
        saved: false,
        loud: 1,
        ids: Vec::new(),
        org_team: None,
        why: Vec::new(),
    }
}

pub fn answer(label: &str, does: &str, act: u8, target: u32) -> Answer {
    Answer { label: label.to_string(), does: does.to_string(), act, target }
}

/// Files a mail on your desk (other lanes call this). A mail already filed
/// or deleted is not filed again.
pub fn send(w: &mut World, m: Mail) {
    let Some(c) = w.career.as_mut() else { return };
    if !c.desk.has(m.id) {
        c.desk.mails.push(m);
        c.desk.mails.sort_by_key(|m| (m.date, m.minute));
    }
}

/// Words in markup: the player and coach ids a linked text names.
pub fn ids_in(text: &str) -> Vec<u32> {
    let mut v = Vec::new();
    for part in text.split("[[p:").skip(1) {
        if let Some(n) = part.split('|').next().and_then(|n| n.parse().ok())
            && !v.contains(&n)
        {
            v.push(n);
        }
    }
    v
}

/// A calendar line without the loudness note the clock adds ("(Big: both
/// ranked.)").
pub fn tidy(text: &str) -> String {
    let mut s = text.to_string();
    for tag in [" (Huge: ", " (Big: ", " (Medium: ", " (Small: "] {
        if let Some(i) = s.find(tag)
            && let Some(j) = s[i..].find(')')
        {
            s.replace_range(i..i + j + 1, "");
        }
    }
    s
}

/// Your desk's writer for one stretch of days.
struct Pen<'a> {
    w: &'a World,
    me: TeamRef,
    /// "Coach Keller".
    dear: String,
    /// "Sam Keller, Head Coach".
    to: String,
    out: Vec<Mail>,
}

impl<'a> Pen<'a> {
    fn colors(&self, m: &mut Mail, t: TeamRef) {
        m.org_team = Some(team_code(t));
    }

    /// A memo from someone on your own staff.
    fn memo(&self, tag: &str, d: Date, minute: u16, job: Job, subject: String) -> Mail {
        let w = self.w;
        let mut m = letter(
            tag,
            d,
            minute,
            Head::Memo,
            views::team_name(w, self.me),
            "Football office".to_string(),
            people::staff_name(w, self.me, job),
            job.title(self.me).to_string(),
            self.to.clone(),
            subject,
        );
        m.org_team = Some(team_code(self.me));
        m
    }

    /// A letter on your school's or club's letterhead from your boss.
    fn boss_letter(&self, tag: &str, d: Date, minute: u16, subject: String) -> Mail {
        let w = self.w;
        let (name, role) = people::boss(w, self.me);
        let (head, line) = match self.me {
            TeamRef::College(p) => {
                (Head::School, format!("Department of Athletics · {}, {}", w.programs[p].city, state_code(w, self.me)))
            }
            TeamRef::Nfl(_) | TeamRef::Other(_) => (Head::Club, "Football operations".to_string()),
            TeamRef::HighSchool(h) => (Head::School, people::district(w, h)),
        };
        let mut m = letter(
            tag,
            d,
            minute,
            head,
            views::team_name(w, self.me),
            line,
            name,
            role.to_string(),
            self.to.clone(),
            subject,
        );
        self.colors(&mut m, self.me);
        m.sign = "Sincerely,".into();
        m.keys |= JOB;
        m
    }

    /// Your compliance office (a club's league office, a high school's
    /// state association).
    fn notice(&self, tag: &str, d: Date, subject: String) -> Mail {
        let w = self.w;
        match self.me {
            TeamRef::Nfl(_) => letter(
                tag,
                d,
                10 * 60,
                Head::League,
                "National Football League".into(),
                "League office · 345 Park Avenue, New York".into(),
                people::named(w, "nfl-football-ops"),
                "Executive vice president, football operations".into(),
                "All clubs: head coaches and general managers".into(),
                subject,
            ),
            TeamRef::HighSchool(h) => {
                let s = &w.high_schools[h];
                let code = w.states[s.state].code.clone();
                let assoc = people::association(&code, views::state_name(&code));
                letter(
                    tag,
                    d,
                    10 * 60,
                    Head::Conference,
                    assoc.clone(),
                    format!("Office of the executive director · {}", views::state_name(&code)),
                    people::named(w, &format!("assoc-{code}")),
                    "Executive director".into(),
                    "Member schools: head football coaches".into(),
                    subject,
                )
            }
            TeamRef::College(_) => self.memo(tag, d, 10 * 60, Job::Compliance, subject),
            TeamRef::Other(i) => {
                let k = afm_engine::world::pro::other::league_of(i);
                letter(
                    tag,
                    d,
                    10 * 60,
                    Head::League,
                    afm_engine::world::pro::other::league_name(k, w.year).into(),
                    "League office · football operations".into(),
                    people::named(w, &format!("league-ops-{k}")),
                    "Vice president, football operations".into(),
                    "All clubs: head coaches and general managers".into(),
                    subject,
                )
            }
        }
    }

    /// The morning clips (10h5): your communications staff clips the papers.
    fn clips(&self, tag: &str, d: Date, lines: Vec<String>) -> Mail {
        let w = self.w;
        let (paper, line) = people::paper(w, self.me);
        let mut m = letter(
            tag,
            d,
            6 * 60 + 30,
            Head::Paper,
            paper,
            line,
            people::staff_name(w, self.me, Job::Communications),
            Job::Communications.title(self.me).to_string(),
            self.to.clone(),
            format!("Morning clips · {}", d.pretty()),
        );
        m.body = lines;
        m.keys = MEDIA;
        m.sign = "Clipped by your communications office.".into();
        m.loud = 0;
        m
    }
}

fn state_code(w: &World, t: TeamRef) -> String {
    match t {
        TeamRef::College(i) => w.states[w.programs[i].state].code.clone(),
        TeamRef::HighSchool(i) => w.states[w.high_schools[i].state].code.clone(),
        TeamRef::Nfl(_) | TeamRef::Other(_) => String::new(),
    }
}

/// The teams a seat follows in the papers beside its own: a college's
/// named rivals, a club's division, a high school's district.
pub fn rivals(w: &World, me: TeamRef) -> Vec<TeamRef> {
    match me {
        TeamRef::College(p) => {
            let name = w.programs[p].real_name.as_str();
            afm_engine::calendar::data::RIVALRIES
                .iter()
                .filter_map(|r| {
                    let other = if r.a == name {
                        r.b
                    } else if r.b == name {
                        r.a
                    } else {
                        return None;
                    };
                    w.programs.iter().position(|q| q.real_name == other).map(TeamRef::College)
                })
                .collect()
        }
        TeamRef::Nfl(t) => {
            let div = views::nfl_division(w, t);
            (0..w.nfl.len())
                .filter(|&o| o != t && div.is_some() && views::nfl_division(w, o) == div)
                .map(TeamRef::Nfl)
                .collect()
        }
        TeamRef::HighSchool(h) => w
            .hs_standings(h)
            .map(|(_, rows)| rows.iter().map(|r| r.0).filter(|&o| o != h).take(10).map(TeamRef::HighSchool).collect())
            .unwrap_or_default(),
        // Another league's clubs: the rest of its division.
        TeamRef::Other(f) => {
            use afm_engine::world::pro::other as o;
            let div = o::def(f, w.year).map(|d| d.div);
            o::teams_in(o::league_of(f), w.year)
                .into_iter()
                .filter(|&x| x != f && o::def(x, w.year).map(|d| d.div) == div)
                .map(TeamRef::Other)
                .collect()
        }
    }
}

/// Whether a line names a team (whole words).
fn names_team(text: &str, name: &str) -> bool {
    !name.is_empty()
        && text.match_indices(name).any(|(i, _)| {
            let before = text[..i].chars().next_back();
            let after = text[i + name.len()..].chars().next();
            !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(|c| c.is_alphanumeric())
        })
}

fn pos_line(w: &World, k: usize) -> String {
    let p = &w.people[k];
    let stars = if p.stars > 0 { format!("{}-star ", p.stars) } else { String::new() };
    format!("{stars}{}", p.player.position.name())
}

/// One season's line in a sentence, by position.
pub fn stat_line(pos: Position, g: impl Fn(Stat) -> f64) -> String {
    use Position as P;
    let n = |s: Stat| g(s).round() as i64;
    match pos {
        P::QB => format!(
            "{}/{} passing, {} yards, {} TD, {} INT",
            n(Stat::Cmp),
            n(Stat::Att),
            n(Stat::PassYds),
            n(Stat::PassTd),
            n(Stat::Int)
        ),
        P::RB => format!("{} carries, {} yards, {} TD", n(Stat::Rush), n(Stat::RushYds), n(Stat::RushTd)),
        P::WR | P::TE => format!("{} catches, {} yards, {} TD", n(Stat::Rec), n(Stat::RecYds), n(Stat::RecTd)),
        P::OT | P::IOL => {
            format!("{} pass-block snaps, {} sacks allowed", n(Stat::PbSnaps), n(Stat::SacksAllowed))
        }
        P::EDGE | P::DT | P::LB => {
            format!("{} tackles, {:.1} for loss, {:.1} sacks", n(Stat::Tackles), g(Stat::Tfl), g(Stat::Sacks))
        }
        P::CB | P::S => format!("{} tackles, {} INT, {} PBU", n(Stat::Tackles), n(Stat::Ints), n(Stat::Pbus)),
        P::K => format!("{}/{} field goals, long {}", n(Stat::FgMade), n(Stat::FgAtt), n(Stat::FgLong)),
        P::P => {
            let punts = g(Stat::Punts).max(1.0);
            format!("{} punts, {:.1} net", n(Stat::Punts), g(Stat::PuntNet) / punts)
        }
        P::LS => format!("{} special teams snaps", n(Stat::SnapsSt)),
    }
}

/// Files the mail for the days since the inbox was last filed (the clock
/// calls it after Continue, a day and a holiday). Mail older than 30 days
/// is never written, so a long holiday costs nothing.
pub fn after_days(w: &mut World) {
    let Some(me) = views::mine(w) else { return };
    let Some(c) = w.career.as_ref() else { return };
    let today = w.today;
    let last = today.add(-1);
    let first_time = c.desk.through.is_none();
    let start = c.desk.through.map_or(c.started, |t| t.add(1)).max(today.add(-KEEP_DAYS));
    let mut mails = Vec::new();
    let mut reveals = Vec::new();
    if first_time {
        welcome(w, me, &mut mails, &mut reveals);
    }
    if start <= last {
        compose(w, me, start, last, &mut mails, &mut reveals);
    }
    let Some(c) = w.career.as_mut() else { return };
    let desk = &mut c.desk;
    for m in mails {
        if !desk.has(m.id) {
            desk.mails.push(m);
        }
    }
    desk.mails.sort_by_key(|m| (m.date, m.minute));
    for r in reveals {
        let id = fnv(&format!("{}|{}|{}", r.kind, r.name, r.date.0));
        if !desk.revealed.contains(&id) {
            desk.revealed.push(id);
            desk.reveals.push(r);
        }
    }
    if start <= last || first_time {
        desk.through = Some(last.max(desk.through.unwrap_or(last)));
    }
    desk.clear_old(today);
}

fn your_names(w: &World) -> (String, String, String) {
    let Some(c) = &w.career else { return ("Coach".into(), "Head coach".into(), String::new()) };
    let last = c.you.name.split_whitespace().last().unwrap_or("").to_string();
    (format!("Coach {last}"), format!("{}, head coach", c.you.name), c.you.name.clone())
}

/// What your boss hired you to do, in his words.
fn expectation(w: &World, me: TeamRef) -> String {
    match me {
        TeamRef::College(p) => crate::setup::college_expectation(w.programs[p].prestige_rel).to_string(),
        TeamRef::HighSchool(h) => {
            let s = &w.high_schools[h];
            crate::setup::hs_expectation(s.prestige, s.state_titles).to_string()
        }
        TeamRef::Nfl(_) => "Build a roster that wins in January, and do it the right way.".to_string(),
        TeamRef::Other(i) => {
            let k = afm_engine::world::pro::other::league_of(i);
            let title = afm_engine::world::pro::other::format(k, w.year).map_or("title game", |f| f.title);
            format!("Win here, and get us to the {title}. A short season leaves no time to settle in.")
        }
    }
}

fn welcome(w: &World, me: TeamRef, mails: &mut Vec<Mail>, reveals: &mut Vec<Reveal>) {
    let Some(c) = &w.career else { return };
    let (dear, to, you) = your_names(w);
    let pen = Pen { w, me, dear: dear.clone(), to, out: Vec::new() };
    let d = c.started;
    let team = views::team_name(w, me);
    let mut m = pen.boss_letter("welcome", d, 8 * 60, format!("Welcome to {team}"));
    let expect = expectation(w, me);
    let (rec, note) = views::season_record(w, me);
    m.body = vec![
        format!("{dear},"),
        format!(
            "On behalf of everyone at {}, welcome. The contract is signed, the office is yours and the staff have been told you start today.",
            links::team(w, me)
        ),
        format!(
            "Last season we went {rec}{}. I will be plain about what we expect: {expect}",
            if note.is_empty() { String::new() } else { format!(" ({note})") }
        ),
        "My door is open. I would rather hear about a problem from you in March than read about it in November."
            .to_string(),
    ];
    // The takeover's first two weeks (people, lane 6), set by the staff.
    let plan = afm_engine::world::folk::letters::takeover_schedule(w);
    if !plan.is_empty() {
        m.body.insert(
            3,
            "The staff has set your first two weeks below: the team first, then the staff, the leaders and the people around the program. Anything you can't get to, they will cover and tell you how it went. [[s:people?tab=Takeover|The takeover file]] keeps it all in one place.".to_string(),
        );
        m.table = plan;
    }
    m.attach = vec!["Employment agreement (signed copy)".into(), "Football staff directory".into()];
    m.loud = 2;
    m.about = Some((ABOUT_TEAM, team_code(me)));
    mails.push(m);
    let colors = views::team_colors(w, me);
    reveals.push(Reveal {
        date: d,
        kind: "hire".into(),
        kicker: "Introducing the new head coach".into(),
        number: String::new(),
        name: you,
        sub: team,
        verdict: expect,
        about: Some((ABOUT_TEAM, team_code(me))),
        colors,
    });
}

/// Writes the mail for the days `from..=to`.
fn compose(w: &World, me: TeamRef, from: Date, to: Date, mails: &mut Vec<Mail>, reveals: &mut Vec<Reveal>) {
    let Some(c) = &w.career else { return };
    let (dear, to_line, _) = your_names(w);
    let mut pen = Pen { w, me, dear, to: to_line, out: Vec::new() };
    let seat = w.seat;
    let yours = views::yours_test(w);
    let rivals = rivals(w, me);
    let rival_names: Vec<String> = rivals.iter().map(|&t| views::team_name(w, t)).collect();
    let my_name = views::team_name(w, me);
    let is_hs = matches!(me, TeamRef::HighSchool(_));
    let my_conf = match me {
        TeamRef::College(p) => Some(w.programs[p].conference.clone()),
        _ => None,
    };
    let my_div = match me {
        TeamRef::College(p) => Some(w.programs[p].division),
        _ => None,
    };

    // The lines this stretch may quote, linked once.
    let mut texts: Vec<&str> = Vec::new();
    for (_, evs) in w.calendar.events.range(from..=to) {
        texts.extend(evs.iter().map(|e| e.text.as_str()));
    }
    let linker = Linker::new(w, &texts, is_hs);
    let link = |e: &Event| tidy(&linker.event(e));

    for (&d, evs) in w.calendar.events.range(from..=to) {
        let next = d.add(1);
        let mut clips: Vec<String> = Vec::new();
        let mut front: Vec<String> = Vec::new();
        let mut conference: Vec<String> = Vec::new();
        let mut desk_lines: Vec<String> = Vec::new();
        let mut draft: Vec<String> = Vec::new();
        let mut awards: Vec<String> = Vec::new();
        let mut practice: Vec<String> = Vec::new();
        let mut staff_moves: Vec<String> = Vec::new();
        for e in evs {
            let mine = yours(e);
            let level_ok = seat.is_none_or(|s| s.cares(e)) || mine;
            let realign = e.kind == Kind::RuleChange
                && e.level == CalLevel::College
                && (e.text.contains(" leaves the ")
                    || e.text.contains(" moves up ")
                    || e.text.contains(" moves down "));
            if realign {
                // 10h1: college seats keep the FBS moves; their own
                // conference's come from the conference office.
                let Some(conf) = &my_conf else { continue };
                if names_team(&e.text, conf) {
                    conference.push(link(e));
                } else if e.loud >= Loudness::Big || e.program.is_some_and(|p| Some(w.programs[p].division) == my_div) {
                    front.push(link(e));
                }
                continue;
            }
            // 10h5: the front page carries your own level's huge news (and
            // the world's); other leagues' news waits on the News screen.
            if e.loud == Loudness::Huge
                && (mine || e.level == CalLevel::World || seat.is_none_or(|s| s.level == e.level))
            {
                front.push(link(e));
            }
            match e.kind {
                Kind::Window | Kind::Deadline | Kind::RuleChange if level_ok && e.level != CalLevel::World => {
                    let rule = if e.rule.is_empty() { String::new() } else { format!(" ({})", e.rule) };
                    desk_lines.push(format!("{}{rule}", link(e)));
                }
                Kind::DraftPick | Kind::DraftChoice if mine => draft.push(link(e)),
                Kind::Award if mine => awards.push(link(e)),
                Kind::Practice if mine || (level_ok && e.level != CalLevel::Recruiting && !is_hs) => {
                    if mine {
                        practice.push(link(e));
                    }
                }
                Kind::Coaching if mine => staff_moves.push(link(e)),
                // Recruiting and portal moves are written from the people
                // themselves below; your games from the game itself.
                Kind::Commitment
                | Kind::Offer
                | Kind::Visit
                | Kind::Signing
                | Kind::Transfer
                | Kind::Slot
                | Kind::Rhythm => {}
                Kind::Game if mine => clips.push(link(e)),
                _ => {
                    let rival = rival_names.iter().any(|n| names_team(&e.text, n))
                        || rivals.iter().any(|&t| match t {
                            TeamRef::College(p) => e.program == Some(p),
                            TeamRef::Nfl(t) => e.nfl_team == Some(t),
                            TeamRef::HighSchool(_) | TeamRef::Other(_) => false,
                        });
                    if (mine || rival) && e.loud < Loudness::Huge && e.kind != Kind::Weather {
                        clips.push(link(e));
                    }
                }
            }
        }
        // Every game of yours makes the papers, ranked or not.
        for (k, g) in w.seat_games().iter().enumerate().filter(|(_, g)| g.date == d) {
            let (h, a) = (g.stats.score[0], g.stats.score[1]);
            let (wn, ws, ln, ls) =
                if h >= a { (&g.names[0], h, &g.names[1], a) } else { (&g.names[1], a, &g.names[0], h) };
            clips.insert(
                0,
                format!(
                    "{wn} {} {ln}, {}.",
                    if h == a { "tied" } else { "beat" },
                    links::game(k, &format!("{ws}-{ls}"))
                ),
            );
        }
        let ds = d.pretty();
        if !clips.is_empty() {
            let mut m = pen.clips(&format!("clips|{my_name}"), next, clips);
            m.body.insert(0, format!("{}, here is what the papers had on us and our rivals from {ds}.", pen.dear));
            pen.out.push(m);
        }
        if !front.is_empty() {
            let mut m = letter(
                "front",
                next,
                6 * 60,
                Head::Paper,
                people::NATIONAL_PAPER.into(),
                format!("Front page · {}", next.pretty()),
                "The sports desk".into(),
                String::new(),
                String::new(),
                strip_dot(&links::strip(&front[0])),
            );
            m.body = front;
            m.keys = LEAGUE | MEDIA;
            m.loud = 3;
            pen.out.push(m);
        }
        if !conference.is_empty()
            && let Some(conf) = &my_conf
        {
            let mut m = letter(
                &format!("conf|{conf}"),
                d,
                15 * 60,
                Head::Conference,
                conference_name(conf),
                "Office of the commissioner".into(),
                people::named(w, &format!("commissioner-{conf}")),
                "Commissioner".into(),
                "Member institutions: athletic directors and head football coaches".into(),
                "Conference membership".into(),
            );
            m.body = vec!["To our member institutions,".into()];
            m.body.extend(conference);
            m.body.push(
                "Scheduling for the seasons ahead will follow from the conference office. Please direct questions to your athletic director."
                    .into(),
            );
            m.keys = LEAGUE;
            m.sign = "Respectfully,".into();
            m.loud = 2;
            pen.out.push(m);
        }
        if !desk_lines.is_empty() {
            let subject = if desk_lines.len() == 1 {
                format!("Notice: {}", strip_dot(&links::strip(&desk_lines[0])))
            } else {
                format!("Calendar notices for {ds}")
            };
            let mut m = pen.notice("notice", d, subject);
            let intro = match me {
                TeamRef::College(_) => format!(
                    "{}, for your calendar. Our office tracks each of these; tell me if you want a briefing.",
                    pen.dear
                ),
                TeamRef::Nfl(_) | TeamRef::Other(_) => {
                    "This memo is to inform all clubs of the following dates and rules.".into()
                }
                TeamRef::HighSchool(_) => "Member schools are reminded of the following.".into(),
            };
            m.body = vec![intro];
            m.body.extend(desk_lines);
            m.keys |= LEAGUE;
            m.sign = if is_hs || matches!(me, TeamRef::Nfl(_)) { "Sincerely,".into() } else { String::new() };
            pen.out.push(m);
        }
        if !draft.is_empty() {
            let (job, subject) = match me {
                TeamRef::Nfl(_) => (Job::Personnel, format!("Our draft: {ds}")),
                _ => (Job::Communications, format!("Our players in the draft: {ds}")),
            };
            let mut m = pen.memo(&format!("draft|{my_name}"), d, 23 * 60, job, subject);
            let ids: Vec<u32> = draft.iter().flat_map(|l| ids_in(l)).collect();
            m.body = vec![format!("{}, the picks that concern us today:", pen.dear)];
            m.body.extend(draft);
            m.keys = DRAFT | PLAYERS;
            if !ids.is_empty() {
                m.answers = vec![
                    answer(
                        "Call each of them tonight",
                        "You call every player named. Each one's mood rises, and the call is logged.",
                        act::CALL_ALL,
                        0,
                    ),
                    answer(
                        "Send a team-wide congratulations",
                        "A note goes to each of them. Each one's mood rises a little.",
                        act::CHEER_ALL,
                        0,
                    ),
                    answer("Leave it to the staff", "Nothing more is sent.", act::ACK, 0),
                ];
            }
            m.ids = ids;
            m.loud = 2;
            pen.out.push(m);
        }
        if !awards.is_empty() {
            let mut m = pen.memo(
                &format!("awards|{my_name}"),
                d,
                17 * 60,
                Job::Communications,
                format!("Honors: {}", strip_dot(&links::strip(&awards[0]))),
            );
            let ids: Vec<u32> = awards.iter().flat_map(|l| ids_in(l)).collect();
            m.body = vec![format!("{}, good news to share with the team:", pen.dear)];
            m.body.extend(awards);
            m.body.push("We will put it out on our channels this afternoon.".into());
            m.keys = PLAYERS | MEDIA;
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
        if !practice.is_empty() {
            let spring = matches!(d.month(), 2..=5);
            let mut m = pen.memo(
                &format!("practice|{my_name}"),
                d,
                7 * 60,
                Job::Operations,
                format!("Practice schedule: {ds}"),
            );
            m.body = vec![format!("{}, today's football schedule notes:", pen.dear)];
            m.body.extend(practice);
            m.keys = if spring { SPRING } else { 0 };
            m.loud = 0;
            pen.out.push(m);
        }
        if !staff_moves.is_empty() {
            let mut m = pen.boss_letter(&format!("staff|{my_name}"), d, 16 * 60, "Staff changes".into());
            m.body = vec![format!("{},", pen.dear)];
            m.body.extend(staff_moves);
            m.body.push("Let me know what you need from the department to fill the gap.".into());
            m.keys |= STAFF;
            pen.out.push(m);
        }
    }

    people_mail(&mut pen, from, to, reveals);
    games_mail(&mut pen, from, to);
    lineup_mail(&mut pen, from, to);
    news_mail(&mut pen, from, to, reveals);
    weekly_mail(&mut pen, from, to, c.coach);
    mails.append(&mut pen.out);
}

fn strip_dot(s: &str) -> String {
    let s = s.trim_end_matches('.');
    if s.chars().count() > 90 { format!("{}…", s.chars().take(88).collect::<String>()) } else { s.to_string() }
}

fn conference_name(conf: &str) -> String {
    match conf {
        "SEC" => "Southeastern Conference".into(),
        "ACC" => "Atlantic Coast Conference".into(),
        "Big Ten" => "The Big Ten Conference".into(),
        "Big 12" => "The Big 12 Conference".into(),
        "Pac-12" => "Pac-12 Conference".into(),
        "AAC" | "American" => "American Athletic Conference".into(),
        "MAC" => "Mid-American Conference".into(),
        "CUSA" => "Conference USA".into(),
        "Mountain West" => "Mountain West Conference".into(),
        "Sun Belt" => "Sun Belt Conference".into(),
        c if c.contains("Conference") || c.contains("League") => c.to_string(),
        c => format!("{c} Conference"),
    }
}

/// Recruiting, signings and the portal from the people themselves: the
/// calendar posts only the loud ones, your staff tells you every one.
fn people_mail(pen: &mut Pen, from: Date, to: Date, reveals: &mut Vec<Reveal>) {
    let w = pen.w;
    let inside = |d: Date| d >= from && d <= to;
    match pen.me {
        TeamRef::College(pi) => {
            let mut commits: Vec<(Date, usize, bool)> = Vec::new();
            let mut signed: HashMap<Date, Vec<usize>> = HashMap::new();
            let mut portal: HashMap<Date, Vec<usize>> = HashMap::new();
            let mut offers: Vec<(Date, usize)> = Vec::new();
            let mut visits: Vec<(Date, usize)> = Vec::new();
            let mut lost: Vec<(Date, usize, usize)> = Vec::new();
            for (k, p) in w.people.iter().enumerate() {
                let cal = &p.cal;
                if let Some((j, d, walk)) = cal.commit
                    && j == pi
                    && inside(d)
                {
                    commits.push((d, k, walk));
                }
                if let Some((j, d, _)) = cal.commit
                    && j != pi
                    && inside(d)
                    && p.stage == Stage::HighSchool
                    && (cal.offers.contains(&pi) || cal.early_offers.iter().any(|o| o.0 == pi))
                {
                    lost.push((d, k, j));
                }
                if let Some(s) = cal.signed
                    && s.program == pi
                    && inside(s.date)
                {
                    signed.entry(s.date).or_default().push(k);
                }
                if let Some(e) = cal.portal
                    && p.college == Some(pi)
                    && inside(e.entered)
                {
                    portal.entry(e.entered).or_default().push(k);
                }
                if p.stage == Stage::HighSchool {
                    for &(j, d) in &cal.early_offers {
                        if j == pi && inside(d) {
                            offers.push((d, k));
                        }
                    }
                    for &(j, d, v) in &cal.visits {
                        if j == pi && inside(d) && v == afm_engine::world::Visit::Official {
                            visits.push((d, k));
                        }
                    }
                }
            }
            for (d, k, walk) in commits {
                let p = &w.people[k];
                let name = links::player(p.player.id, &p.name);
                let what = pos_line(w, k);
                let mut m = pen.memo(
                    &format!("commit|{}", p.player.id),
                    d,
                    21 * 60,
                    Job::Personnel,
                    format!("Commitment: {what} {}", p.name),
                );
                let others: Vec<String> = views::offers_of(p)
                    .into_iter()
                    .filter(|&o| o != pi)
                    .take(3)
                    .map(|o| links::team(w, TeamRef::College(o)))
                    .collect();
                m.body = vec![
                    format!("{}, {name} just called: he is committed to us{}.", pen.dear, if walk { " as a preferred walk-on" } else { "" }),
                    format!(
                        "He is a {what} from {} ({}), class of {}.{}",
                        views::hometown(w, p),
                        links::team(w, TeamRef::HighSchool(p.hs)),
                        p.class_of,
                        if others.is_empty() { String::new() } else { format!(" He turned down {}.", others.join(", ")) }
                    ),
                    "It is a verbal: nothing is binding until he signs, so the staff keeps recruiting him like he hasn't.".into(),
                ];
                m.keys = RECRUITING | PLAYERS;
                m.about = Some((ABOUT_PLAYER, p.player.id));
                m.ids = vec![p.player.id];
                m.why = lane_help::pick_why(w, k, pi, None);
                m.answers = vec![
                    answer(
                        "Call him and his family tonight",
                        "You call him tonight. His mood rises and he trusts you a little more.",
                        act::CALL,
                        p.player.id,
                    ),
                    answer(
                        "Send a handwritten note",
                        "A note goes out tomorrow. His mood rises a little.",
                        act::NOTE,
                        p.player.id,
                    ),
                    answer(
                        "Leave it to the recruiting staff",
                        "The staff's welcome stands; nothing more is sent.",
                        act::ACK,
                        0,
                    ),
                ];
                m.loud = if p.stars >= 4 { 2 } else { 1 };
                if p.stars >= 4 {
                    reveals.push(Reveal {
                        date: d,
                        kind: "commit".into(),
                        kicker: format!("{}-star {} commits", p.stars, p.player.position.name()),
                        number: format!("{}★", p.stars),
                        name: p.name.clone(),
                        sub: format!("{} · {}", views::hometown(w, p), w.school_name(p.hs)),
                        verdict: format!(
                            "He picks {} over {} other offers.",
                            views::team_name(w, pen.me),
                            views::offers_of(p).len().saturating_sub(1)
                        ),
                        about: Some((ABOUT_PLAYER, p.player.id)),
                        colors: views::team_colors(w, pen.me),
                    });
                }
                pen.out.push(m);
            }
            // H6: a recruit we offered picks another school, and why.
            for (d, k, j) in lost {
                let p = &w.people[k];
                let what = pos_line(w, k);
                let mut m = pen.memo(
                    &format!("lost|{}", p.player.id),
                    d,
                    21 * 60 + 20,
                    Job::Personnel,
                    format!("{} picks {}", p.name, w.program_name(j)),
                );
                m.body = vec![
                    format!(
                        "{}, {} ({what}, class of {}) committed to {} today. We had offered him.",
                        pen.dear,
                        links::player(p.player.id, &p.name),
                        p.class_of,
                        links::team(w, TeamRef::College(j))
                    ),
                    "A verbal is not a signature: the staff can keep recruiting him, or move the scholarship to the next name on the board."
                        .into(),
                ];
                m.why = lane_help::pick_why(w, k, j, Some(pi));
                m.keys = RECRUITING | PLAYERS;
                m.about = Some((ABOUT_PLAYER, p.player.id));
                m.ids = vec![p.player.id];
                m.loud = if p.stars >= 4 { 2 } else { 1 };
                pen.out.push(m);
            }
            let mut days: Vec<_> = signed.into_iter().collect();
            days.sort_by_key(|x| x.0);
            for (d, mut ks) in days {
                ks.sort_by_key(|&k| std::cmp::Reverse(w.people[k].stars));
                let mut m = pen.memo(
                    &format!("signed|{}", d.0),
                    d,
                    18 * 60,
                    Job::Personnel,
                    format!("Signings: {} today", ks.len()),
                );
                m.body = vec![format!("{}, the paperwork is in for these players:", pen.dear)];
                m.table =
                    vec![vec!["Player".into(), "Pos".into(), "Stars".into(), "Hometown".into(), "High school".into()]];
                for &k in ks.iter().take(40) {
                    let p = &w.people[k];
                    m.table.push(vec![
                        links::player(p.player.id, &p.name),
                        p.player.position.name().into(),
                        if p.stars > 0 { format!("{}★", p.stars) } else { "–".into() },
                        views::hometown(w, p),
                        links::team(w, TeamRef::HighSchool(p.hs)),
                    ]);
                }
                m.ids = ks.iter().map(|&k| w.people[k].player.id).collect();
                m.keys = RECRUITING | PLAYERS;
                m.answers = vec![
                    answer(
                        "Call every signee tonight",
                        "You call each signee. Each one's mood rises and the calls are logged.",
                        act::CALL_ALL,
                        0,
                    ),
                    answer(
                        "Post a welcome to the class",
                        "A welcome goes out to the class. Each one's mood rises a little.",
                        act::CHEER_ALL,
                        0,
                    ),
                ];
                m.loud = 2;
                pen.out.push(m);
            }
            let mut days: Vec<_> = portal.into_iter().collect();
            days.sort_by_key(|x| x.0);
            for (d, ks) in days {
                let mut m = pen.memo(
                    &format!("portal|{}", d.0),
                    d,
                    19 * 60,
                    Job::Personnel,
                    if ks.len() == 1 {
                        format!("Portal: {} enters", w.people[ks[0]].name)
                    } else {
                        format!("Portal: {} of ours enter", ks.len())
                    },
                );
                m.body = vec![format!(
                    "{}, the compliance office has entered these names in the transfer portal today:",
                    pen.dear
                )];
                m.table = vec![vec!["Player".into(), "Pos".into(), "Class".into(), "Why, as we hear it".into()]];
                for &k in &ks {
                    let p = &w.people[k];
                    m.table.push(vec![
                        links::player(p.player.id, &p.name),
                        p.player.position.name().into(),
                        views::class_text(w, p),
                        p.cal.portal.map(|e| e.why.to_string()).unwrap_or_default(),
                    ]);
                }
                m.body.push("Other staffs can contact them from today. Their scholarships are ours to fill.".into());
                m.why = if let [k] = ks[..] {
                    let said = w.people[k].cal.portal.map(|e| e.why.to_string()).unwrap_or_default();
                    lane_help::portal_why(w, k, &said)
                } else {
                    ks.iter()
                        .take(3)
                        .map(|&k| {
                            let p = &w.people[k];
                            let why = lane_help::portal_why(w, k, "");
                            format!("{}: {}", links::player(p.player.id, &p.name), why.join("; ").to_lowercase())
                        })
                        .collect()
                };
                m.ids = ks.iter().map(|&k| w.people[k].player.id).collect();
                m.keys = PORTAL | PLAYERS;
                m.answers = vec![
                    answer(
                        "Call each of them",
                        "You call each of them. It doesn't undo the entry, but each one's mood rises and the call is logged.",
                        act::CALL_ALL,
                        0,
                    ),
                    answer(
                        "Wish them well in writing",
                        "A note goes to each. Each one's mood rises a little.",
                        act::CHEER_ALL,
                        0,
                    ),
                    answer("Say nothing", "Nothing is sent.", act::ACK, 0),
                ];
                pen.out.push(m);
            }
            // Offers and official visits: one digest a week (Mondays), so a
            // busy staff's work doesn't bury the inbox.
            if !offers.is_empty() || !visits.is_empty() {
                let mut weeks: HashMap<i32, (Vec<usize>, Vec<usize>)> = HashMap::new();
                for (d, k) in offers {
                    weeks.entry(d.on_or_after(Weekday::Mon).0).or_default().0.push(k);
                }
                for (d, k) in visits {
                    weeks.entry(d.on_or_after(Weekday::Mon).0).or_default().1.push(k);
                }
                let mut list: Vec<_> = weeks.into_iter().collect();
                list.sort_by_key(|x| x.0);
                for (wk, (of, vi)) in list {
                    let d = Date(wk).min(to.add(1));
                    let mut m = pen.memo(
                        &format!("recruiting|{wk}"),
                        d,
                        8 * 60,
                        Job::Personnel,
                        format!("Recruiting desk: {} offers, {} official visits", of.len(), vi.len()),
                    );
                    m.body = vec![format!("{}, the week on the recruiting board:", pen.dear)];
                    m.table =
                        vec![vec!["Player".into(), "Pos".into(), "Stars".into(), "High school".into(), "What".into()]];
                    for (k, what) in
                        of.iter().map(|&k| (k, "Offered")).chain(vi.iter().map(|&k| (k, "Official visit"))).take(40)
                    {
                        let p = &w.people[k];
                        m.table.push(vec![
                            links::player(p.player.id, &p.name),
                            p.player.position.name().into(),
                            if p.stars > 0 { format!("{}★", p.stars) } else { "–".into() },
                            links::team(w, TeamRef::HighSchool(p.hs)),
                            what.into(),
                        ]);
                    }
                    m.ids = of.iter().chain(vi.iter()).map(|&k| w.people[k].player.id).collect();
                    m.keys = RECRUITING;
                    m.loud = 0;
                    m.answers = vec![answer(
                        "Add them all to my players",
                        "Every player in the table goes on your \"my players\" list.",
                        act::LIST_ALL,
                        0,
                    )];
                    pen.out.push(m);
                }
            }
        }
        TeamRef::HighSchool(h) => {
            let roster = views::roster_people(w, pen.me);
            let mut best_sign: Option<(Date, usize)> = None;
            let mut signed_days: HashMap<Date, Vec<usize>> = HashMap::new();
            for &k in &roster {
                let p = &w.people[k];
                let id = p.player.id;
                let mut today_offers: HashMap<Date, usize> = HashMap::new();
                for &(j, d) in &p.cal.early_offers {
                    if !inside(d) {
                        continue;
                    }
                    let n = today_offers.entry(d).or_default();
                    *n += 1;
                    if *n > 3 {
                        continue;
                    }
                    let prog = &w.programs[j];
                    let group = Group::of(p.player.position);
                    let coach = prog.position_coaches[group as usize].or(prog.head_coach);
                    let (from_name, role) = match coach {
                        Some(c) => {
                            (w.coaches[c].name.clone(), format!("{} coach, {}", group_name(group), prog.real_name))
                        }
                        None => (
                            people::staff_name(w, TeamRef::College(j), Job::Personnel),
                            format!("Director of player personnel, {}", prog.real_name),
                        ),
                    };
                    let mut m = letter(
                        &format!("offer|{id}|{j}"),
                        d,
                        (11 * 60 + (id % 300) as usize) as u16,
                        Head::Email,
                        w.program_name(j).to_string(),
                        format!("{} football", w.program_name(j)),
                        from_name,
                        role,
                        pen.to.clone(),
                        format!("Scholarship offer for {}", p.name),
                    );
                    m.org_team = Some(team_code(TeamRef::College(j)));
                    m.body = vec![
                        format!("{},", pen.dear),
                        format!(
                            "I wanted you to hear it from us first: {} is offering {} a full scholarship. We love the way he plays and the way your program has brought him along.",
                            links::team(w, TeamRef::College(j)),
                            links::player(id, &p.name)
                        ),
                        "I'd like to set up a time to talk with him and his family. Whatever works around your practice schedule works for us.".into(),
                    ];
                    m.sign = "Thanks, Coach.".into();
                    m.keys = RECRUITING | PLAYERS;
                    m.about = Some((ABOUT_PLAYER, id));
                    m.ids = vec![id];
                    m.answers = vec![
                        answer(
                            "Tell him after practice today",
                            "You give him the news yourself. His mood rises a little.",
                            act::NOTE,
                            id,
                        ),
                        answer(
                            "Set up the call with his family",
                            "You arrange it and sit in. His mood rises and he trusts you a little more.",
                            act::CALL,
                            id,
                        ),
                    ];
                    if let Some(c) = coach {
                        m.answers.push(answer(
                            "Thank the coach",
                            "You write back. The contact is logged with the coach.",
                            act::THANK_COACH,
                            c as u32,
                        ));
                    }
                    pen.out.push(m);
                }
                if let Some((j, d, _)) = p.cal.commit
                    && inside(d)
                {
                    let mut m = letter(
                        &format!("hscommit|{id}"),
                        d,
                        20 * 60 + 15,
                        Head::Email,
                        p.name.clone(),
                        String::new(),
                        p.name.clone(),
                        format!("{} · class of {}", p.player.position.name(), p.class_of),
                        pen.to.clone(),
                        "Wanted you to hear it from me".into(),
                    );
                    m.body = vec![
                        "Coach,".into(),
                        format!(
                            "I wanted you to hear it from me before it gets out. I'm committing to {}. I talked it over with my family this weekend and it feels right.",
                            links::team(w, TeamRef::College(j))
                        ),
                        "Thank you for everything. I wouldn't be here without you.".into(),
                    ];
                    m.sign = p.name.split_whitespace().next().unwrap_or("").to_string();
                    m.keys = RECRUITING | PLAYERS;
                    m.about = Some((ABOUT_PLAYER, id));
                    m.ids = vec![id];
                    m.why = lane_help::pick_why(w, k, j, None);
                    m.answers = vec![
                        answer(
                            "Call him tonight",
                            "You call him. His mood rises and he trusts you a little more.",
                            act::CALL,
                            id,
                        ),
                        answer("Write back", "A short reply. His mood rises a little.", act::NOTE, id),
                    ];
                    m.loud = 2;
                    pen.out.push(m);
                }
                if let Some(s) = p.cal.signed
                    && inside(s.date)
                {
                    signed_days.entry(s.date).or_default().push(k);
                    if best_sign.is_none_or(|(_, b)| w.people[b].stars < p.stars) {
                        best_sign = Some((s.date, k));
                    }
                }
            }
            let mut days: Vec<_> = signed_days.into_iter().collect();
            days.sort_by_key(|x| x.0);
            for (d, ks) in days {
                let mut m = pen.memo(
                    &format!("hssign|{}", d.0),
                    d,
                    17 * 60,
                    Job::Operations,
                    format!("Signing day: {} of ours signed", ks.len()),
                );
                m.body = vec![format!(
                    "{}, the library is booked for the ceremony and the families have their seats. Today's signees:",
                    pen.dear
                )];
                m.table = vec![vec!["Player".into(), "Pos".into(), "Signed with".into()]];
                for &k in &ks {
                    let p = &w.people[k];
                    m.table.push(vec![
                        links::player(p.player.id, &p.name),
                        p.player.position.name().into(),
                        p.cal.signed.map(|s| links::team(w, TeamRef::College(s.program))).unwrap_or_default(),
                    ]);
                }
                m.ids = ks.iter().map(|&k| w.people[k].player.id).collect();
                m.keys = RECRUITING | PLAYERS;
                m.answers = vec![
                    answer(
                        "Say a few words at the ceremony",
                        "You speak for each of them. Each one's mood rises.",
                        act::CALL_ALL,
                        0,
                    ),
                    answer(
                        "Congratulate each one",
                        "A word to each. Each one's mood rises a little.",
                        act::CHEER_ALL,
                        0,
                    ),
                ];
                m.loud = 2;
                pen.out.push(m);
            }
            if let Some((d, k)) = best_sign {
                let p = &w.people[k];
                if let Some(s) = p.cal.signed {
                    reveals.push(Reveal {
                        date: d,
                        kind: "signing".into(),
                        kicker: format!("Signing day at {}", w.school_name(h)),
                        number: if p.stars > 0 { format!("{}★", p.stars) } else { String::new() },
                        name: p.name.clone(),
                        sub: format!("{} · signs with {}", p.player.position.name(), w.program_name(s.program)),
                        verdict: "The cap goes on in front of the whole school.".into(),
                        about: Some((ABOUT_PLAYER, p.player.id)),
                        colors: views::team_colors(w, TeamRef::College(s.program)),
                    });
                }
            }
        }
        TeamRef::Nfl(_) | TeamRef::Other(_) => {}
    }
}

fn group_name(g: Group) -> &'static str {
    match g {
        Group::QB => "Quarterbacks",
        Group::RB => "Running backs",
        Group::Receivers => "Receivers",
        Group::OL => "Offensive line",
        Group::DL => "Defensive line",
        Group::LB => "Linebackers",
        Group::DB => "Defensive backs",
        Group::Specialists => "Special teams",
    }
}

/// A postgame memo for each of your games, and the boss's word after a
/// third loss in a row.
fn games_mail(pen: &mut Pen, from: Date, to: Date) {
    let w = pen.w;
    let games = w.seat_games();
    for (k, g) in games.iter().enumerate() {
        if g.date < from || g.date > to {
            continue;
        }
        let side = match (pen.me, g.level) {
            (TeamRef::College(i), _) | (TeamRef::Nfl(i), _) | (TeamRef::HighSchool(i), _) | (TeamRef::Other(i), _) => {
                if g.teams[0] == i {
                    0
                } else {
                    1
                }
            }
        };
        let s = &g.stats;
        let (us, them) = (s.score[side], s.score[1 - side]);
        let won = us > them;
        let opp = g.names[1 - side].clone();
        let verb = if won {
            "beat"
        } else if us == them {
            "tied"
        } else {
            "lost to"
        };
        let mut m = pen.memo(
            &format!("game|{}", g.date.0),
            g.date,
            23 * 60 + 10,
            Job::Communications,
            format!("Postgame notes: {} {}, {} {}", g.names[side], us, opp, them),
        );
        let place = if side == 0 { "at home" } else { "on the road" };
        m.body = vec![format!(
            "{}, final {place}: we {verb} {opp}, {us}-{them}. The full box and every snap are on {}.",
            pen.dear,
            links::game(k, "the game page")
        )];
        let top = views::possession(s);
        m.table = vec![vec![
            "Team".into(),
            "1st downs".into(),
            "Yards".into(),
            "Pass".into(),
            "Rush".into(),
            "3rd down".into(),
            "Turnovers".into(),
            "Penalties".into(),
            "Possession".into(),
        ]];
        for sd in [side, 1 - side] {
            let t = &s.team[sd];
            m.table.push(vec![
                g.names[sd].clone(),
                t.first_downs.to_string(),
                format!("{:.0}", t.yards),
                format!("{:.0}", t.pass_yards),
                format!("{:.0}", t.rush_yards),
                format!("{}/{}", t.third_conv, t.third_att),
                t.turnovers.to_string(),
                format!("{}-{:.0}", t.penalties, t.penalty_yds),
                format!("{}:{:02}", (top[sd] / 60.0) as u32, (top[sd] % 60.0) as u32),
            ]);
        }
        let name =
            |id: u32| views::person_index(w, id).map_or(format!("#{id}"), |i| links::player(id, &w.people[i].name));
        let lines = &s.players[side];
        let best = |f: &dyn Fn(&afm_engine::game::stats::StatLine) -> f64| {
            lines.iter().filter(|(_, l)| f(l) > 0.0).max_by(|a, b| f(a.1).total_cmp(&f(b.1))).map(|(&id, l)| (id, l))
        };
        let mut leaders = Vec::new();
        let mut ids = Vec::new();
        if let Some((id, l)) = best(&|l| l.att as f64) {
            leaders.push(format!(
                "Passing: {} {}/{}, {:.0} yards, {} TD, {} INT",
                name(id),
                l.cmp,
                l.att,
                l.pass_yds,
                l.pass_td,
                l.int
            ));
            ids.push(id);
        }
        if let Some((id, l)) = best(&|l| l.rush_yds) {
            leaders.push(format!(
                "Rushing: {} {} carries, {:.0} yards, {} TD",
                name(id),
                l.rush,
                l.rush_yds,
                l.rush_td
            ));
            ids.push(id);
        }
        if let Some((id, l)) = best(&|l| l.rec_yds) {
            leaders.push(format!("Receiving: {} {} catches, {:.0} yards, {} TD", name(id), l.rec, l.rec_yds, l.rec_td));
            ids.push(id);
        }
        if let Some((id, l)) = best(&|l| (l.tackles + l.assists) as f64 + 2.0 * l.sacks + l.tfl + 3.0 * l.ints as f64) {
            leaders.push(format!(
                "Defense: {} {} tackles{}{}",
                name(id),
                l.tackles + l.assists,
                if l.sacks > 0.0 { format!(", {:.1} sacks", l.sacks) } else { String::new() },
                if l.ints > 0 { format!(", {} INT", l.ints) } else { String::new() }
            ));
            ids.push(id);
        }
        m.body.extend(leaders);
        ids.dedup();
        m.ids = ids;
        m.keys = PLAYERS | MEDIA;
        m.about = Some((ABOUT_GAME, k as u32));
        m.answers = vec![
            answer(
                "Add today's leaders to my players",
                "The players named above go on your \"my players\" list.",
                act::LIST_ALL,
                0,
            ),
            answer(
                "Praise them in the team meeting",
                "You single them out on Monday. Each one's mood rises a little.",
                act::CHEER_ALL,
                0,
            ),
        ];
        m.loud = 2;
        pen.out.push(m);
        // The boss writes after a third straight loss.
        let streak = games[..=k]
            .iter()
            .rev()
            .take_while(|g| {
                let sd = match pen.me {
                    TeamRef::College(i) | TeamRef::Nfl(i) | TeamRef::HighSchool(i) | TeamRef::Other(i) => {
                        usize::from(g.teams[0] != i)
                    }
                };
                g.stats.score[sd] < g.stats.score[1 - sd]
            })
            .count();
        if streak == 3 {
            let mut b = pen.boss_letter(
                &format!("streak|{}", g.date.0),
                g.date.add(1),
                9 * 60,
                "A word about the last three weeks".into(),
            );
            b.body = vec![
                format!("{},", pen.dear),
                "Three in a row is three in a row. I am not writing to panic, and I don't want you to read this as anything but support. But people are asking me questions, and I want to answer them with your plan, not mine."
                    .into(),
                "Come by my office this week. Bring what you are changing.".into(),
            ];
            b.answers = vec![
                answer(
                    "Go see him this week",
                    "You meet him. It is logged; his patience rides on results, not on the meeting.",
                    act::ACK,
                    0,
                ),
                answer(
                    "Reply that the plan is in place",
                    "Your reply is filed. His patience rides on results.",
                    act::ACK,
                    0,
                ),
            ];
            let losses: Vec<String> = games[k + 1 - streak..=k]
                .iter()
                .map(|g| {
                    let sd = match pen.me {
                        TeamRef::College(i) | TeamRef::Nfl(i) | TeamRef::HighSchool(i) | TeamRef::Other(i) => {
                            usize::from(g.teams[0] != i)
                        }
                    };
                    format!("{} {}-{}", g.names[1 - sd], g.stats.score[1 - sd], g.stats.score[sd])
                })
                .collect();
            let (rec, _) = views::season_record(w, pen.me);
            b.why = lane_help::streak_why(&losses, &rec, &expectation(w, pen.me));
            b.answer_by = Some(g.date.add(7));
            b.loud = 2;
            pen.out.push(b);
        }
    }
}

/// The news desk's lines about your own program: the collective's NIL
/// warnings and your titles.
fn news_mail(pen: &mut Pen, from: Date, to: Date, reveals: &mut Vec<Reveal>) {
    let w = pen.w;
    let TeamRef::College(pi) = pen.me else { return };
    for (i, n) in w.news.iter().enumerate().rev().take(400) {
        let Some(d) = n.date else { continue };
        if d < from || d > to || n.program != Some(pi) {
            continue;
        }
        match n.kind {
            NewsKind::NilWarning { .. } => {
                let prog = &w.programs[pi];
                let mut m = letter(
                    &format!("nil|{i}"),
                    d,
                    14 * 60,
                    Head::Email,
                    format!("{} collective", prog.real_name),
                    String::new(),
                    people::staff_name(w, pen.me, Job::Collective),
                    format!("Executive director, the {} collective", prog.real_name),
                    pen.to.clone(),
                    "Where the money stands".into(),
                );
                m.body = vec![
                    format!("{},", pen.dear),
                    n.text.clone(),
                    "Happy to walk you through the numbers before you make promises to anyone.".into(),
                ];
                m.keys = MONEY;
                m.sign = "Best,".into();
                pen.out.push(m);
            }
            NewsKind::ConferenceTitle | NewsKind::NationalTitle => {
                let mut m = pen.boss_letter(&format!("title|{i}"), d, 10 * 60, "Champions".into());
                m.body = vec![
                    format!("{},", pen.dear),
                    n.text.clone(),
                    "Thank you. Enjoy this one with your family before recruiting takes the next week from you.".into(),
                ];
                m.loud = 3;
                pen.out.push(m);
                reveals.push(Reveal {
                    date: d,
                    kind: "title".into(),
                    kicker: if n.kind == NewsKind::NationalTitle {
                        "National champions".into()
                    } else {
                        "Conference champions".into()
                    },
                    number: d.year().to_string(),
                    name: views::team_name(w, pen.me),
                    sub: links::strip(&n.text),
                    verdict: String::new(),
                    about: Some((ABOUT_TEAM, team_code(pen.me))),
                    colors: views::team_colors(w, pen.me),
                });
            }
            _ => {}
        }
    }
}

/// Monday mail: your people (10h3), lower-division standouts in the
/// portal windows (10h2), an NFL club's injury report on Wednesdays.
fn weekly_mail(pen: &mut Pen, from: Date, to: Date, coach: usize) {
    let w = pen.w;
    let mut d = from;
    while d <= to {
        match d.weekday() {
            Weekday::Mon => {
                your_people(pen, d, coach);
                standouts(pen, d);
                morale(pen, d);
            }
            Weekday::Wed => injuries(pen, d),
            _ => {}
        }
        d = d.add(1);
    }
    let _ = w;
}

/// H6: the players whose mood fell last week, with why (the chief of staff,
/// Mondays). Only what your staff hears: the dated reasons behind it.
fn morale(pen: &mut Pen, d: Date) {
    let w = pen.w;
    let dips = lane_help::mood_dips(w, pen.me, d);
    if dips.is_empty() {
        return;
    }
    let mut m = pen.memo(
        &format!("morale|{}", d.0),
        d,
        7 * 60 + 45,
        Job::ChiefOfStaff,
        if dips.len() == 1 {
            format!("Morale: {} is down", w.people[dips[0].0].name)
        } else {
            format!("Morale: {} players are down", dips.len())
        },
    );
    m.body = vec![format!("{}, the moods that fell this past week, as the staff hears it:", pen.dear)];
    m.table = vec![vec!["Player".into(), "Pos".into(), "Mood".into(), "Why".into()]];
    for (k, why) in dips.iter().take(12) {
        let p = &w.people[*k];
        m.table.push(vec![
            links::player(p.player.id, &p.name),
            p.player.position.name().into(),
            afm_engine::world::folk::talks::mood_words(p.mood).into(),
            why.join("; "),
        ]);
    }
    m.why = dips.first().map(|(_, why)| why.clone()).unwrap_or_default();
    m.ids = dips.iter().map(|x| w.people[x.0].player.id).collect();
    m.keys = PEOPLE | PLAYERS;
    m.answers = vec![
        answer(
            "Talk to each of them this week",
            "You sit down with each. Their mood rises and each talk is logged.",
            act::CALL_ALL,
            0,
        ),
        answer("Leave it to the position coaches", "Nothing more is done.", act::ACK, 0),
    ];
    m.loud = 1;
    pen.out.push(m);
}

/// H5: the night before each of your games, your coordinator writes when a
/// lineup choice of yours reads clearly weak. It never blocks you.
fn lineup_mail(pen: &mut Pen, from: Date, to: Date) {
    let w = pen.w;
    let Some(st) = w.training_team() else { return };
    let mut d = from;
    while d <= to {
        if st.games.contains(&d.add(1)) {
            let spots = lane_help::weak_spots(w, pen.me);
            // One note a side: offense, then defense and special teams.
            let mut sides: Vec<(bool, Vec<(Position, String)>)> = Vec::new();
            for (pos, line) in spots {
                let off = afm_engine::world::coach::Group::of(pos).offense();
                match sides.iter_mut().find(|x| x.0 == off) {
                    Some(x) => x.1.push((pos, line)),
                    None => sides.push((off, vec![(pos, line)])),
                }
            }
            for (off, lines) in sides {
                let Some((name, title)) = lane_help::note_writer(w, lines[0].0) else { continue };
                let team = views::team_name(w, pen.me);
                let mut m = letter(
                    &format!("lineup|{}|{off}", d.0),
                    d,
                    20 * 60 + 30,
                    Head::Memo,
                    team,
                    "Football office".into(),
                    name,
                    title,
                    pen.to.clone(),
                    format!("Before kickoff: a word on the {}", if off { "offense" } else { "defense" }),
                );
                m.org_team = Some(team_code(pen.me));
                m.body = vec![format!("{}, before tomorrow, one thing from the film room:", pen.dear)];
                m.body.extend(lines.iter().map(|x| x.1.clone()));
                m.body.push(format!(
                    "It's your call. The {} has the order as you set it.",
                    links::screen("depth", "depth chart")
                ));
                m.why = lines.iter().map(|x| links::strip(&x.1)).take(3).collect();
                let mask: u32 = lines.iter().fold(0, |a, x| a | (1 << x.0.index()));
                m.answers = vec![
                    answer("Keep my lineup", "Your order stands.", act::ACK, 0),
                    answer(
                        "Give these spots back to the staff",
                        "The staff's order returns at these positions before kickoff.",
                        act::DEPTH_BACK,
                        mask,
                    ),
                ];
                m.keys = PLAYERS;
                m.loud = 1;
                pen.out.push(m);
            }
        }
        d = d.add(1);
    }
}

/// The people you are linked to: players who played for you and have moved
/// on, and your mentor. The chief of staff lists last week's lines naming them.
fn your_people(pen: &mut Pen, d: Date, coach: usize) {
    let w = pen.w;
    let mine: HashSet<usize> = views::roster_people(w, pen.me).into_iter().collect();
    let mut names: Vec<(String, u32)> = Vec::new();
    let started = w.career.as_ref().map_or(w.year, |c| c.started.year());
    match pen.me {
        TeamRef::College(_) => {
            for (k, p) in w.people.iter().enumerate() {
                if !mine.contains(&k)
                    && p.stage != Stage::Done
                    && p.college_staff.iter().any(|s| s.2 == Some(coach) && s.0 >= started)
                {
                    names.push((p.name.clone(), p.player.id));
                }
            }
        }
        TeamRef::Nfl(t) => {
            for (k, p) in w.people.iter().enumerate() {
                if !mine.contains(&k)
                    && p.stage == Stage::Pro
                    && p.pro.as_ref().is_some_and(|c| c.seasons.iter().any(|s| s.team == Some(t) && s.year >= started))
                {
                    names.push((p.name.clone(), p.player.id));
                }
            }
        }
        TeamRef::HighSchool(h) => {
            for (k, p) in w.people.iter().enumerate() {
                if !mine.contains(&k)
                    && p.hs == h
                    && p.class_of >= started
                    && p.stage != Stage::HighSchool
                    && p.stage != Stage::Done
                {
                    names.push((p.name.clone(), p.player.id));
                }
            }
        }
        TeamRef::Other(f) => {
            for (k, p) in w.people.iter().enumerate() {
                if !mine.contains(&k)
                    && p.stage == Stage::Pro
                    && p.pro.as_ref().is_some_and(|c| c.other.iter().any(|s| s.team as usize == f && s.year >= started))
                {
                    names.push((p.name.clone(), p.player.id));
                }
            }
        }
    }
    let mentor = w.coaches.get(coach).and_then(|c| c.mentor);
    let mut lines = Vec::new();
    let mut ids = Vec::new();
    for (_, evs) in w.calendar.events.range(d.add(-7)..d) {
        for e in evs {
            for (n, id) in &names {
                if names_team(&e.text, n) {
                    lines.push(tidy(&e.text.replacen(n.as_str(), &links::player(*id, n), 1)));
                    if !ids.contains(id) {
                        ids.push(*id);
                    }
                    break;
                }
            }
            if let Some(mc) = mentor
                && names_team(&e.text, &w.coaches[mc].name)
            {
                let n = &w.coaches[mc].name;
                lines.push(tidy(&e.text.replacen(n.as_str(), &links::coach(mc, n), 1)));
            }
        }
    }
    if lines.is_empty() {
        return;
    }
    lines.truncate(20);
    let mut m = pen.memo(&format!("people|{}", d.0), d, 7 * 60 + 30, Job::ChiefOfStaff, "Your people this week".into());
    m.body = vec![format!("{}, news from people you are linked to, from the last seven days:", pen.dear)];
    m.body.extend(lines);
    m.keys = PEOPLE;
    if !ids.is_empty() {
        m.answers = vec![
            answer(
                "Send each of them congratulations",
                "A text to each. Each player's mood rises a little.",
                act::CHEER_ALL,
                0,
            ),
            answer("Call each of them", "You call each. Their mood rises and each call is logged.", act::CALL_ALL, 0),
            answer("Nothing this week", "Nothing is sent.", act::ACK, 0),
        ];
    }
    m.ids = ids;
    pen.out.push(m);
}

/// Lower-division standouts as a portal watch list (10h2), in December and
/// April when the portal windows open, for college seats.
fn standouts(pen: &mut Pen, d: Date) {
    let w = pen.w;
    let TeamRef::College(pi) = pen.me else { return };
    let in_window = (d.month() == 12 && d.day() <= 21) || (d.month() == 4 && d.day() >= 7 && d.day() <= 28);
    if !in_window {
        return;
    }
    let my_div = w.programs[pi].division;
    let Some(last) = w.stats.seasons.last().map(|s| s.year) else { return };
    let mut rows: Vec<(f64, usize, &afm_engine::world::records::SeasonRecord)> = Vec::new();
    for r in w.stats.seasons.iter().rev().take_while(|r| r.year == last) {
        if r.division <= my_div || r.games < 6 {
            continue;
        }
        let Some(k) = views::person_index(w, r.id) else { continue };
        let p = &w.people[k];
        if p.stage != Stage::College || p.college_seasons >= 4 {
            continue;
        }
        let score = r.footprint.count_ones() as f64 * 10.0 + r.get(Stat::GradeSum) / r.get(Stat::Graded).max(1.0);
        if r.footprint == 0 {
            continue;
        }
        rows.push((score, k, r));
    }
    if rows.is_empty() {
        return;
    }
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    rows.truncate(12);
    let mut m = pen.memo(
        &format!("standouts|{}", d.0),
        d,
        8 * 60 + 15,
        Job::Personnel,
        "Portal watch: lower-division standouts".into(),
    );
    m.body = vec![format!(
        "{}, the portal window is open. These players below our level put up numbers in the top one percent at their position last season. None is in the portal yet; if one enters, we want to be first.",
        pen.dear
    )];
    m.table = vec![vec!["Player".into(), "Pos".into(), "School".into(), "Class".into(), "Season".into()]];
    for (_, k, r) in &rows {
        let p = &w.people[*k];
        m.table.push(vec![
            links::player(p.player.id, &p.name),
            r.pos.name().into(),
            links::team(w, TeamRef::College(r.program as usize)),
            views::class_text(w, p),
            stat_line(r.pos, |s| r.get(s)),
        ]);
    }
    m.ids = rows.iter().map(|x| w.people[x.1].player.id).collect();
    m.keys = PORTAL | RECRUITING;
    m.answers = vec![
        answer(
            "Put them all on my players",
            "Every player in the table goes on your \"my players\" list, so you see him the day he enters.",
            act::LIST_ALL,
            0,
        ),
        answer("Not now", "Nothing changes.", act::ACK, 0),
    ];
    pen.out.push(m);
}

/// An NFL club's Wednesday injury report from the head athletic trainer.
fn injuries(pen: &mut Pen, d: Date) {
    let w = pen.w;
    let TeamRef::Nfl(_) = pen.me else { return };
    let mut hurt: Vec<(usize, u32)> = views::roster_people(w, pen.me)
        .into_iter()
        .filter_map(|k| w.people[k].pro.as_ref().filter(|c| c.out_weeks > 0).map(|c| (k, c.out_weeks)))
        .collect();
    if hurt.is_empty() {
        return;
    }
    hurt.sort_by_key(|x| std::cmp::Reverse(x.1));
    let mut m =
        pen.memo(&format!("injuries|{}", d.0), d, 16 * 60, Job::Trainer, format!("Injury report: {}", d.pretty()));
    m.body = vec![format!("{}, the report the league gets from us today:", pen.dear)];
    m.table = vec![vec!["Player".into(), "Pos".into(), "Practice".into(), "Out".into()]];
    for (k, weeks) in &hurt {
        let p = &w.people[*k];
        m.table.push(vec![
            links::player(p.player.id, &p.name),
            p.player.position.name().into(),
            if *weeks > 1 { "Did not participate".into() } else { "Limited".into() },
            if *weeks == 1 { "Questionable".into() } else { format!("{weeks} weeks") },
        ]);
    }
    m.ids = hurt.iter().map(|x| w.people[x.0].player.id).collect();
    m.keys = INJURIES | PLAYERS;
    m.loud = 1;
    pen.out.push(m);
}

/// Picks an answer (10i) and does what it says.
pub fn pick(w: &mut World, id: u64, k: usize) -> Result<(), String> {
    let today = w.today;
    let (a, ids) = {
        let c = w.career.as_ref().ok_or("No career.")?;
        let m = c.desk.mails.iter().find(|m| m.id == id).ok_or("That mail has cleared.")?;
        if m.answered.is_some() {
            return Err("You already answered.".into());
        }
        (m.answers.get(k).cloned().ok_or("No such answer.")?, m.ids.clone())
    };
    let lift = |w: &mut World, pid: u32, mood: f32, trust: f32| {
        if let Some(i) = views::person_index(w, pid) {
            let coach = w.career.as_ref().map(|c| c.coach as u32);
            let p = &mut w.people[i];
            p.mood = (p.mood + mood).clamp(-1.0, 1.0);
            if Some(p.trust_in) == coach {
                p.trust = (p.trust + trust).clamp(0.0, 1.0);
            }
        }
    };
    let mut contacts: Vec<(u8, u32, &str)> = Vec::new();
    let mut list: Vec<u32> = Vec::new();
    match a.act {
        act::CALL => {
            lift(w, a.target, 0.08, 0.03);
            contacts.push((0, a.target, "Call"));
        }
        act::NOTE => {
            lift(w, a.target, 0.04, 0.01);
            contacts.push((0, a.target, "Note"));
        }
        act::CHEER_ALL => {
            for &p in &ids {
                lift(w, p, 0.03, 0.0);
                contacts.push((0, p, "Congratulations"));
            }
        }
        act::CALL_ALL => {
            for &p in &ids {
                lift(w, p, 0.06, 0.02);
                contacts.push((0, p, "Call"));
            }
        }
        act::LIST_ALL => list = ids.clone(),
        act::LIST_ONE => list = vec![a.target],
        act::THANK_COACH => contacts.push((1, a.target, "Thank-you")),
        act::DEPTH_BACK => {
            for pos in Position::ALL {
                if a.target & (1 << pos.index()) != 0 && w.controls(pos) {
                    w.set_seat_depth(pos, Vec::new());
                }
            }
            w.refresh_seat_team();
        }
        _ => {}
    }
    for p in list {
        super::desk::list_add(w, p, 2);
    }
    let c = w.career.as_mut().ok_or("No career.")?;
    for (kind, id, what) in contacts {
        c.desk.contacts.push(afm_engine::world::desk::Contact { kind, id, date: today, what: what.into() });
    }
    if let Some(m) = c.desk.mails.iter_mut().find(|m| m.id == id) {
        m.answered = Some(k as u8);
        m.read = true;
    }
    Ok(())
}
