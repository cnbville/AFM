//! Taking over a job (`docs/final-build/taking-over.md`, T1-T23 and T-B):
//! your first two weeks are dated meetings. Continue stops on each one; each
//! is a scene with the person, the place, what he says and your answers as
//! picked moves, and any of them can be handed to your staff. Everything the
//! meetings tell you goes into the takeover file with who said it, his angle
//! and, after spring ball, how it held up.
//!
//! The order follows a real first fortnight (builder's notes, proposed, not
//! confirmed): at a college the team meeting the first evening, the
//! introductory press conference and the athletic director the next day,
//! the holdover staff, the state of the team, the trainer and the academic
//! advisor, the flight risks and the one-on-ones, the recruiting
//! coordinator's calls, the boosters, the beat reporters, the tour and the
//! plan. An NFL club starts with the owner and the general manager; a high
//! school with the principal and the athletic director, and ends with the
//! parents' meeting and the feeder tour. The same arrival runs for every job
//! in a career.

use super::cast::{self, Cast, CastRef, Role, Want};
use super::facts;
use super::scene::{self, Doc, DocRow, Kind, Pick, Scene, Who};
use super::voice::{Facts, Pen, Speaker};
use super::{Team, coach_link, player_link};
use crate::calendar::{Date, Level, Weekday};
use crate::player::Trait;
use crate::world::{PlayingTime, Stage, World};

/// The takeover: your new team, the day you took it and the man you replaced.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Takeover {
    pub team: Option<Team>,
    pub started: Option<Date>,
    pub old_coach: Option<usize>,
    /// How your name read against the job on day one, -1 to 1 (item 4).
    pub fit: f32,
    /// The scenes of the first two weeks, in order.
    pub scenes: Vec<u32>,
    /// Players your staff flagged as flight risks (item 7), by id, with the
    /// main reason.
    pub risks: Vec<(u32, String)>,
    /// Holdover assistants you let go, and kept.
    pub let_go: Vec<usize>,
    pub kept: Vec<usize>,
    /// Claims to check after spring ball (item 3): who made it, the player,
    /// and +1 oversold or -1 undersold.
    pub claims: Vec<(String, u32, i8)>,
    pub checked: bool,
}

/// The day the arrival's scenes are scheduled from: offsets skip Sundays.
fn day_at(d0: Date, k: i32) -> Date {
    let mut d = d0;
    let mut left = k;
    while left > 0 {
        d = d.add(1);
        if d.weekday() != Weekday::Sun {
            left -= 1;
        }
    }
    d
}

/// Begins the arrival at your new job: first impressions, the coaching-change
/// window (T-B) and the first two weeks' meetings.
pub fn begin(w: &mut World) {
    let Some(team) = Team::seat(w) else { return };
    let Some(you) = w.career.as_ref().map(|c| c.coach) else { return };
    let d0 = w.today;
    let old = facts::predecessor(w, team, you).map(|x| x.0);
    let fit = facts::fit(w, team) as f32;
    // Scenes of an earlier job stay in the log, closed.
    for s in w.folk.scenes.iter_mut().filter(|s| !s.done) {
        s.done = true;
        s.picks.clear();
        s.handed = Some("nobody: you left the job".into());
    }
    w.folk.mine = super::Mine { team: Some(team), since: Some(d0), ..Default::default() };
    w.folk.promises.clear();
    w.folk.goals.clear();
    w.folk.asks.clear();
    w.folk.cases.clear();
    w.folk.takeover =
        Some(Takeover { team: Some(team), started: Some(d0), old_coach: old, fit, ..Default::default() });
    first_impressions(w, team, old, fit);
    if let Team::College(p) = team {
        coach_change_window(w, p, d0);
    }
    let plan = schedule(w, team, d0);
    let mut ids = Vec::new();
    for (k, kind, a, title, place) in plan {
        let mut s = Scene::new(kind, day_at(d0, k), title);
        s.place = place;
        s.a = a;
        s.team = Some(team);
        ids.push(scene::add(w, s));
    }
    if let Some(t) = w.folk.takeover.as_mut() {
        t.scenes = ids;
    }
    let name = team.name(w);
    super::note(
        w,
        "you",
        "You".into(),
        format!("You took the job at {}. The first two weeks are meetings.", team.link(w)),
        None,
    );
    let _ = name;
}

/// T-B: your hire opens the coaching-change transfer window for your
/// players (15 days, starting five days after the hire, from 2025; 30 days
/// in the 2022-24 windows era) and pushes the "his head coach is gone" lean.
fn coach_change_window(w: &mut World, p: usize, d0: Date) {
    use crate::world::high_school::TransferRule as _Unused;
    let _ = std::marker::PhantomData::<_Unused>;
    let rule = w.changes.rules(w.year).transfer;
    let days = match rule {
        crate::calendar::rules::TransferRule::Windows => Some(30),
        crate::calendar::rules::TransferRule::January => Some(15),
        _ => None,
    };
    let factor = crate::career::PIPELINE.portal_entry.coaching_change;
    for i in 0..w.people.len() {
        let pp = &w.people[i];
        if pp.stage == Stage::College && pp.college == Some(p) && pp.cal.lean > 0.0 {
            crate::world::college::lean_push(&mut w.people[i], d0, "his head coach is gone", factor);
        }
    }
    if let Some(days) = days {
        let open = d0.add(5);
        let close = open.add(days);
        w.folk.window = Some((p, open, close));
        super::post_mine(
            w,
            open,
            crate::calendar::Loudness::Medium,
            false,
            "NCAA: coaching-change window",
            &format!(
                "The coaching-change transfer window opens for your players today and runs until {}.",
                close.pretty()
            ),
        );
    }
}

/// Opens the coaching-change window on its day (called from the people's
/// day): the portal's own machinery runs for your program alone.
pub(crate) fn window_day(w: &mut World, d: Date) {
    let Some((p, open, close)) = w.folk.window else { return };
    if d == open {
        w.folk.window_only = Some(p);
        crate::world::recruiting::portal_open(w, crate::world::clock::PortalWindow::CoachChange, close);
        w.folk.window_only = None;
    }
    if d >= close {
        w.folk.window = None;
    }
}

/// Item 4: everyone's trust in you starts from your name against the job and
/// his ties to the man you replaced.
fn first_impressions(w: &mut World, team: Team, old: Option<usize>, fit: f32) {
    let you = w.career.as_ref().map_or(0, |c| c.coach) as u32;
    // Players: trust in their new head coach.
    let roster = team.roster(w);
    for &i in &roster {
        let tie = tie_to_old(w, i, old);
        let p = &mut w.people[i];
        let loyal = p.player.personality.traits.get(Trait::Loyalty) as f32;
        p.trust_in = you;
        p.trust = (0.5 + 0.12 * fit - tie * (0.12 + 0.012 * (loyal - 10.0))).clamp(0.15, 0.85);
    }
    // The people around the team.
    for role in roles_around(team) {
        for c in cast::all(w, team, role) {
            let loyal = c.booster.as_ref().map_or(0.3, |b| b.loyal_old) as f32;
            let warm = Cast::z(c.warmth) as f32;
            let t = 0.5 + 0.15 * fit - 0.18 * loyal * old.is_some() as u8 as f32 + 0.06 * warm;
            w.folk.mine.set_trust(c.key(), t);
        }
    }
    // Holdover assistants: men the old coach hired start cooler.
    for (c, _) in facts::assistants(w, team) {
        let mentor = old.is_some_and(|o| w.coaches[c].mentor == Some(o));
        let t = 0.5 + 0.1 * fit - if mentor { 0.15 } else { 0.04 };
        w.folk.mine.set_trust(super::coach_key(c), t);
    }
}

/// How close a player is to the man you replaced, 0-1: years under him, and
/// whether he recruited him.
pub fn tie_to_old(w: &World, i: usize, old: Option<usize>) -> f32 {
    let Some(o) = old else { return 0.0 };
    let p = &w.people[i];
    let years = p.college_staff.iter().filter(|x| x.2 == Some(o)).count() as f32;
    let recruited = p.college_staff.first().is_some_and(|x| x.2 == Some(o));
    let pro = p.pro.as_ref().is_some_and(|c| c.seasons.len() >= 2) as u8 as f32;
    ((0.18 * years + if recruited { 0.3 } else { 0.0 } + 0.2 * pro) * (0.6 + 0.04 * p.player.personality.traits.get(Trait::Loyalty) as f32))
        .clamp(0.0, 1.0)
}

/// The roles you meet in your first weeks, by level.
pub fn roles_around(team: Team) -> Vec<Role> {
    match team {
        Team::College(_) => vec![
            Role::AthleticDirector,
            Role::President,
            Role::Booster,
            Role::Reporter,
            Role::Comms,
            Role::Ops,
            Role::Academic,
            Role::Trainer,
            Role::Strength,
            Role::Personnel,
            Role::ChiefOfStaff,
        ],
        Team::Nfl(_) => vec![
            Role::Owner,
            Role::OwnerKin,
            Role::TeamPresident,
            Role::Gm,
            Role::Reporter,
            Role::Comms,
            Role::Trainer,
            Role::Strength,
            Role::CapManager,
            Role::Personnel,
            Role::ChiefOfStaff,
        ],
        Team::High(_) => vec![
            Role::Principal,
            Role::SchoolAd,
            Role::Superintendent,
            Role::Booster,
            Role::ClubPresident,
            Role::Reporter,
            Role::Academic,
            Role::Trainer,
            Role::Feeder,
            Role::Youth,
        ],
    }
}

/// The two weeks' meetings: (day offset, kind, its subject, title, place).
fn schedule(w: &World, team: Team, _d0: Date) -> Vec<(i32, Kind, u64, String, String)> {
    let mut v: Vec<(i32, Kind, u64, String, String)> = Vec::new();
    let name = team.name(w);
    let town = team.town(w);
    match team {
        Team::College(p) => {
            let building = football_building(w, p);
            v.push((0, Kind::TeamMeeting, 0, format!("Your first team meeting at {name}"), format!("{building}, the team room, 7:30 p.m.")));
            v.push((1, Kind::HireReaction, 0, "How the world took your hire".into(), "Your new office, before the press conference".into()));
            v.push((1, Kind::PressIntro, 0, format!("Introduced as {name}'s head coach"), format!("{building}, the media room, 11 a.m.")));
            v.push((1, Kind::BossFirst, 0, "The athletic director's goals".into(), "The athletic director's office, 3 p.m.".into()));
            let staff = facts::assistants(w, team);
            for (c, role) in staff.iter().filter(|x| matches!(x.1, crate::world::coach::Role::OffensiveCoordinator | crate::world::coach::Role::DefensiveCoordinator)) {
                v.push((2, Kind::Holdover, *c as u64, format!("Meeting the {}", role.name()), format!("{building}, your office")));
            }
            v.push((3, Kind::Holdover, u64::MAX, "The holdover position coaches".into(), format!("{building}, the staff room")));
            v.push((3, Kind::StaffReport, 0, "The state of the team".into(), format!("{building}, the staff room, film on")));
            v.push((4, Kind::Leaders, 0, "The captains and the room".into(), "Your office, one at a time".into()));
            v.push((4, Kind::Trainer, 0, "The trainer's report".into(), "The training room".into()));
            v.push((5, Kind::Academic, 0, "The academic and conduct report".into(), "The academic center".into()));
            v.push((5, Kind::FlightRisks, 0, "Who might leave".into(), "Your office".into()));
            v.push((6, Kind::Recruiting, 0, "Commits who wobble".into(), "The recruiting office, phones out".into()));
            let n = cast::slots(w, team, Role::Booster);
            for k in 0..n.min(5) {
                v.push((7 + k as i32, Kind::Booster, k as u64, String::new(), String::new()));
            }
            v.push((9, Kind::Media, 0, "The beat reporters".into(), "The communications office".into()));
            v.push((10, Kind::Tour, 0, format!("The tour of {name}"), format!("Around {town}")));
            v.push((11, Kind::Plan, 0, "The first weeks' plan".into(), "Your office, the operations director".into()));
        }
        Team::Nfl(_) => {
            v.push((0, Kind::BossFirst, 0, "The owner's goals".into(), "The owner's suite at the facility".into()));
            v.push((0, Kind::HireReaction, 0, "How the world took your hire".into(), "Your new office".into()));
            v.push((1, Kind::PressIntro, 0, format!("Introduced as the {name}' head coach"), "The facility's auditorium, noon".into()));
            v.push((1, Kind::Gm, 0, "Sitting down with the general manager".into(), "The general manager's office".into()));
            let staff = facts::assistants(w, team);
            for (c, role) in &staff {
                v.push((2, Kind::Holdover, *c as u64, format!("Meeting the {}", role.name()), "Your office".into()));
            }
            v.push((3, Kind::StaffReport, 0, "The roster review with the pro scouts".into(), "The draft room, film on".into()));
            v.push((4, Kind::Trainer, 0, "The trainer's report".into(), "The training room".into()));
            v.push((4, Kind::Academic, 0, "The conduct report".into(), "The security director's office".into()));
            v.push((5, Kind::Leaders, 0, "Calling the team's leaders".into(), "Your office, on the phone".into()));
            v.push((5, Kind::FlightRisks, 0, "Who might want out".into(), "The general manager's office".into()));
            v.push((6, Kind::Booster, 0, String::new(), String::new()));
            v.push((7, Kind::Media, 0, "The beat reporters".into(), "The communications office".into()));
            v.push((8, Kind::Tour, 0, format!("The {name} up close"), "The facility and the stadium".into()));
            v.push((8, Kind::Plan, 0, "The calendar to the draft".into(), "The general manager and the cap manager".into()));
            // The first full team meeting waits for the offseason program.
            let days = w.today.days_to(w.dates.offseason_new_coach).max(9);
            v.push((days, Kind::TeamMeeting, 0, "Your first team meeting".into(), "The team auditorium, the first day of the offseason program".into()));
        }
        Team::High(h) => {
            let school = name.clone();
            v.push((0, Kind::BossFirst, 0, "The principal and the athletic director".into(), format!("{school}, the principal's office, 7:15 a.m.")));
            v.push((0, Kind::HireReaction, 0, "How the town took your hire".into(), "The front office".into()));
            v.push((1, Kind::PressIntro, 0, "The local paper's interview".into(), format!("{school}, the field house")));
            v.push((1, Kind::TeamMeeting, 0, "Your first athletics period".into(), format!("{school}, the field house, sixth period")));
            v.push((2, Kind::Holdover, u64::MAX, "The holdover staff".into(), format!("{school}, the coaches' office")));
            v.push((2, Kind::OldCoach, 0, "The old coach, down the hall".into(), format!("{school}, his classroom after school")));
            v.push((3, Kind::StaffReport, 0, "The state of the team".into(), "The coaches' office, last fall's film on".into()));
            v.push((3, Kind::Leaders, 0, "The seniors who lead".into(), "The weight room".into()));
            v.push((4, Kind::Academic, 0, "Grades and eligibility".into(), "The counselor's office".into()));
            v.push((4, Kind::Trainer, 0, "Who's hurt".into(), "The training room".into()));
            v.push((5, Kind::FlightRisks, 0, "Who might transfer".into(), "The coaches' office".into()));
            let n = cast::slots(w, team, Role::Booster);
            v.push((6, Kind::Booster, 100, String::new(), String::new()));
            for k in 0..n.min(2) {
                v.push((6, Kind::Booster, k as u64, String::new(), String::new()));
            }
            v.push((7, Kind::Parents, 0, "The parents' meeting".into(), format!("{school}, the cafeteria, 7 p.m.")));
            v.push((8, Kind::Feeder, 0, "The feeder tour".into(), format!("The middle schools and the youth league around {town}")));
            v.push((9, Kind::Media, 0, "Who covers you".into(), "The front office".into()));
            v.push((9, Kind::Plan, 0, "The plan to the season".into(), "The athletic director's office".into()));
            let _ = h;
        }
    }
    v
}

fn football_building(w: &World, p: usize) -> String {
    let prog = &w.programs[p];
    if prog.facilities > 0.6 { "The football building".into() } else { "The football offices".into() }
}

/// The scene is opened: its first lines and picks.
pub(crate) fn open(w: &mut World, s: &mut Scene) {
    let team = s.team.or(Team::seat(w));
    let Some(team) = team else {
        s.narrate("The meeting never happened.");
        s.end();
        return;
    };
    match s.kind {
        Kind::TeamMeeting => team_meeting_open(w, s, team),
        Kind::HireReaction => hire_reaction(w, s, team),
        Kind::Holdover => holdover_open(w, s, team),
        Kind::StaffReport => staff_report(w, s, team),
        Kind::Leaders => leaders(w, s, team),
        Kind::Trainer => trainer(w, s, team),
        Kind::Academic => academic(w, s, team),
        Kind::FlightRisks => flight_risks(w, s, team),
        Kind::Recruiting => recruiting(w, s, team),
        Kind::Booster => booster_open(w, s, team),
        Kind::Media => media_open(w, s, team),
        Kind::Tour => tour(w, s, team),
        Kind::Plan => plan(w, s, team),
        Kind::Parents => parents_open(w, s, team),
        Kind::Feeder => feeder(w, s, team),
        Kind::OldCoach => old_coach_open(w, s, team),
        Kind::Gm => gm_open(w, s, team),
        _ => {
            s.narrate("Nothing on the agenda.");
            s.end();
        }
    }
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    let Some(team) = s.team.or(Team::seat(w)) else {
        s.end();
        return;
    };
    match s.kind {
        Kind::TeamMeeting => team_meeting_answer(w, s, team, code, picked),
        Kind::Holdover => holdover_answer(w, s, team, code),
        Kind::Booster => booster_answer(w, s, team, code),
        Kind::Media => media_answer(w, s, team, code),
        Kind::Parents => parents_answer(w, s, team, code),
        Kind::OldCoach => old_coach_answer(w, s, team, code),
        Kind::FlightRisks => flight_answer(w, s, team, code),
        Kind::Recruiting => recruiting_answer(w, s, team, code),
        Kind::Gm => gm_answer(w, s, team, code),
        Kind::Leaders | Kind::Academic | Kind::Trainer | Kind::StaffReport => report_answer(w, s, team, code),
        _ => {
            // Reports: reading them is the whole meeting.
            s.outcome.push("Read and filed in the takeover file.".into());
            s.end();
        }
    }
}

/// Your staff takes it (calendar topic 10): the plain, safe version.
pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    let Some(team) = s.team.or(Team::seat(w)) else { return };
    let who = staff_hand(w, team, s.kind);
    s.handed = Some(who.clone());
    match s.kind {
        Kind::TeamMeeting => {
            // Nobody else can hold your first team meeting: it waits as a
            // short, plain word at the next practice.
            team_effect(w, team, Message::Compete, Tone::Calm, 0.5);
            s.outcome.push("You said a few plain words at the end of a workout instead. The room shrugged: no first impression either way.".into());
        }
        Kind::PressIntro => {}
        Kind::Booster => {
            if let Some(c) = booster_of(w, s, team) {
                w.folk.mine.nudge(c.key(), -0.08);
                s.outcome.push(format!(
                    "{} took the meeting with {who} instead of you, and noticed.",
                    c.name
                ));
            }
        }
        Kind::Holdover => {
            s.outcome.push("You kept the holdover staff through spring for now; you can still let anyone go from the staff screen.".into());
        }
        Kind::FlightRisks => {
            let n = w.folk.takeover.as_ref().map_or(0, |t| t.risks.len());
            for id in w.folk.takeover.as_ref().map(|t| t.risks.iter().map(|r| r.0).collect::<Vec<_>>()).unwrap_or_default() {
                if let Some(i) = super::person(w, id) {
                    position_coach_word(w, i);
                }
            }
            s.outcome.push(format!("Your position coaches talked to the {n} flight risks. They did what they could; a word from you carries more."));
        }
        Kind::Recruiting => {
            recruiting_calls(w, team, false);
            s.outcome.push(format!("{who} made the calls. The commits heard from the staff, not from you."));
        }
        Kind::Parents => {
            s.outcome.push(format!("{who} ran the parents' meeting. Parents wanted to see you; some said so."));
            for &i in &team.roster(w) {
                if w.people[i].family.parent_style == crate::world::background::ParentStyle::LoudAtGames {
                    w.people[i].trust = (w.people[i].trust - 0.02).max(0.0);
                }
            }
        }
        Kind::OldCoach => {
            s.outcome.push("You never found the time to stop by his classroom. He noticed, and so did the people who still call him Coach.".into());
            if let Some(o) = w.folk.takeover.as_ref().and_then(|t| t.old_coach) {
                w.folk.mine.nudge(super::coach_key(o), -0.1);
            }
        }
        Kind::Gm => {
            s.outcome.push("The general manager ran the meeting on his own terms and left you his notes.".into());
        }
        _ => {
            if s.outcome.is_empty() {
                s.outcome.push(format!("{who} took it and left you the notes."));
            }
        }
    }
}

/// Who on your staff takes a meeting you hand off.
fn staff_hand(w: &World, team: Team, kind: Kind) -> String {
    let get = |r: Role| cast::has(w, team, r).then(|| cast::get(w, CastRef::new(team, r, 0)));
    let c = match (team, kind) {
        (Team::College(_), Kind::Booster | Kind::Media | Kind::Tour) => get(Role::ChiefOfStaff).or(get(Role::Ops)),
        (Team::College(_), Kind::Recruiting) => get(Role::Personnel).or(get(Role::Ops)),
        (Team::Nfl(_), _) => get(Role::ChiefOfStaff).or(get(Role::Gm)),
        (Team::High(_), _) => get(Role::SchoolAd),
        _ => get(Role::Ops),
    };
    match c {
        Some(c) => format!("{} ({})", c.name, c.title.to_lowercase()),
        None => match facts::assistants(w, team).first() {
            Some(&(a, r)) => format!("{} ({})", w.coaches[a].name, r.name()),
            None => "your staff".into(),
        },
    }
}

// ---------------------------------------------------------------------------
// Item 16: the first team meeting.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Message {
    /// Every job is open: the depth chart starts over.
    Compete,
    /// Honour the past and the men who built it.
    Honour,
    /// We win now; the standard is a title.
    WinNow,
    /// Deion's: some of you won't be here. Find out if you want this.
    Purge,
    /// Short: who you are, how you work, then practice.
    Plain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Calm,
    Warm,
    Firm,
    Fiery,
}

impl Tone {
    pub fn from(code: u32) -> Tone {
        match code % 4 {
            0 => Tone::Calm,
            1 => Tone::Warm,
            2 => Tone::Firm,
            _ => Tone::Fiery,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Tone::Calm => "calm",
            Tone::Warm => "warm",
            Tone::Firm => "firm",
            Tone::Fiery => "fiery",
        }
    }
}

fn team_meeting_open(w: &mut World, s: &mut Scene, team: Team) {
    let roster = team.roster(w);
    let old = w.folk.takeover.as_ref().and_then(|t| t.old_coach);
    let n = roster.len();
    let close: Vec<usize> = roster.iter().copied().filter(|&i| tie_to_old(w, i, old) > 0.45).collect();
    let captains = captains_of(w, team);
    s.who.push(Who::Group(format!("The team ({n} players)")));
    let mut pen = scene::pen(w, s, 1);
    let you = Speaker::you(w);
    let mut f = base_facts(w, team);
    f.set("count", n.to_string());
    let room = match team {
        Team::High(_) => "the field house bleachers",
        Team::Nfl(_) => "the auditorium",
        _ => "the team room",
    };
    s.narrate(format!(
        "{n} players in {room}. {} The room is quiet in the way rooms are when nobody knows yet what you are.",
        match (old, close.len()) {
            (Some(o), k) if k >= 6 => format!(
                "A lot of them came here for {}: you can pick out the ones who are still wearing his last bowl gear.",
                coach_link(w, o)
            ),
            (Some(o), k) if k > 0 => format!("Some of them were close to {}, and they sit together near the back.", coach_link(w, o)),
            _ => "Nobody here owes the last staff much.".to_string(),
        }
    ));
    if let Some(&c) = captains.first() {
        s.narrate(format!("{} sits in the front row, arms folded. Everyone else watches him as much as you.", player_link(w, c)));
    }
    let open = pen.line(&["takeover.meeting.you_open"], &you, &f);
    if !open.is_empty() {
        s.you(&scene::your_name(w), open);
    }
    pen.put(w);
    s.narrate("What do you tell them?");
    add_message_picks(s);
}

fn add_message_picks(s: &mut Scene) {
    s.pick(10, "Every job is open. The depth chart starts over today.", "Challenge", "firm", "Competitors and backups lift; entrenched starters bristle.");
    s.pick(11, "I'm not here to erase what was built. I'm here to finish it.", "Praise", "warm", "Loyal players and the old coach's people warm to you; ambitious ones hear nothing new.");
    s.pick(12, "We win now. The standard here is a championship, starting this year.", "Promise", "fiery", "Lifts the room for now and raises the bar you'll be judged by.");
    s.pick(13, "Some of you won't be here when we play. Find out tonight if you want this.", "Warn", "fiery", "Clears out players who won't buy in. Some leave; the rest know where they stand.");
    s.pick(14, "Who I am, how we'll work, and when practice starts. That's it.", "Tell him straight", "calm", "Little lift and little risk: they judge you by the first workout.");
}

fn team_meeting_answer(w: &mut World, s: &mut Scene, team: Team, code: u32, picked: &Pick) {
    let (msg, tone) = match code {
        10 => (Message::Compete, Tone::Firm),
        11 => (Message::Honour, Tone::Warm),
        12 => (Message::WinNow, Tone::Fiery),
        13 => (Message::Purge, Tone::Fiery),
        _ => (Message::Plain, Tone::Calm),
    };
    s.you(&scene::your_name(w), picked.label.clone());
    let (up, down, gone) = team_effect(w, team, msg, tone, 1.0);
    let mut pen = scene::pen(w, s, 2);
    let f = base_facts(w, team);
    // A voice from the room: a leader's reaction.
    if let Some(&c) = captains_of(w, team).first() {
        let spk = Speaker::player(w, c);
        let key = match msg {
            Message::Compete => "takeover.meeting.react_compete",
            Message::Honour => "takeover.meeting.react_honour",
            Message::WinNow => "takeover.meeting.react_winnow",
            Message::Purge => "takeover.meeting.react_purge",
            Message::Plain => "takeover.meeting.react_plain",
        };
        let line = pen.line(&[key], &spk, &f);
        if !line.is_empty() {
            let name = w.people[c].name.clone();
            s.them(&name, line, Some(tell_for_leader(w, c, msg)));
        }
    }
    pen.put(w);
    s.narrate(match msg {
        Message::Purge => "The meeting ends with nobody talking. By the time you reach the parking lot, the first player has asked the compliance office about the portal.".to_string(),
        Message::WinNow => "The room gets loud for the first time. The seniors like it. You can tell the writers will like it more.".to_string(),
        Message::Honour => "Some of the older players nod along. One or two look at each other like they've heard this before.".to_string(),
        Message::Compete => "The backups sit up. A couple of starters don't.".to_string(),
        Message::Plain => "Nobody cheers and nobody walks out. They'll decide about you in the weight room.".to_string(),
    });
    s.outcome.push(format!(
        "Your first team meeting: {} player{} warmed to you, {} cooled{}.",
        up,
        if up == 1 { "" } else { "s" },
        down,
        if gone > 0 { format!(", and {gone} started looking at the portal") } else { String::new() }
    ));
    if msg == Message::WinNow {
        w.folk.mine.expectation = (w.folk.mine.expectation + 0.25).min(1.0);
    }
    if msg == Message::Purge {
        w.folk.mine.hard = (w.folk.mine.hard + 0.3).min(1.0);
    }
    s.end();
}

/// How a message lands on each player (item 16): by his personality and his
/// ties to the old coach. Returns (warmed, cooled, leaning to leave).
pub(crate) fn team_effect(w: &mut World, team: Team, msg: Message, tone: Tone, weight: f32) -> (u32, u32, u32) {
    let old = w.folk.takeover.as_ref().and_then(|t| t.old_coach);
    let roster = team.roster(w);
    let (mut up, mut down, mut gone) = (0, 0, 0);
    let today = w.today;
    for i in roster {
        let tie = tie_to_old(w, i, old);
        let p = &w.people[i];
        let t = &p.player.personality.traits;
        let z = |tr: Trait| (t.get(tr) - 10.0) / 10.0;
        let comp = (p.player.personality.competitiveness - 10.0) / 10.0;
        let starter = p.last_role == Some(PlayingTime::Starter);
        let mut d = match msg {
            Message::Compete => 0.05 * comp + if starter { -0.04 - 0.04 * z(Trait::Ego) } else { 0.06 + 0.03 * z(Trait::Ambition) },
            Message::Honour => 0.05 * tie + 0.03 * z(Trait::Loyalty) - 0.02 * z(Trait::Ambition),
            Message::WinNow => 0.05 + 0.03 * comp + 0.02 * z(Trait::Ego),
            Message::Purge => 0.03 * comp - 0.06 * tie - 0.04 * z(Trait::Ego) - 0.03 * (1.0 - z(Trait::Professionalism)),
            Message::Plain => 0.01 + 0.02 * z(Trait::Professionalism),
        };
        d += match tone {
            Tone::Fiery => 0.02 * comp - 0.02 * z(Trait::Temper).max(0.0),
            Tone::Warm => 0.015 * (1.0 - z(Trait::Ego)),
            Tone::Firm => 0.015 * z(Trait::Professionalism),
            Tone::Calm => 0.0,
        };
        // A player who wants a reason to stay hears one; the room's mood leans.
        let noise = (super::roll(w, 0x7E4E, p.player.id as u64 ^ today.0 as u64) as f32 - 0.5) * 0.04;
        let dt = (d + noise) * weight;
        let lean_f = if msg == Message::Purge && (tie > 0.35 || z(Trait::Professionalism) < -0.2) {
            1.25 + 0.4 * tie as f64
        } else if dt > 0.03 {
            0.95
        } else if dt < -0.03 {
            1.08
        } else {
            1.0
        };
        let p = &mut w.people[i];
        p.trust = (p.trust + dt).clamp(0.0, 1.0);
        p.mood = (p.mood + 0.6 * dt).clamp(-1.0, 1.0);
        if dt > 0.02 {
            up += 1;
        } else if dt < -0.02 {
            down += 1;
        }
        if p.stage == Stage::College && p.cal.lean > 0.0 && lean_f != 1.0 {
            let why: &'static str = match msg {
                Message::Purge => "the new coach told him to find out if he wants this",
                _ if lean_f < 1.0 => "liked what the new coach told the team",
                _ => "didn't like what the new coach told the team",
            };
            p.cal.lean *= lean_f;
            p.cal.lean_events.push((today, why, lean_f));
            if lean_f > 1.2 {
                gone += 1;
            }
        }
    }
    (up, down, gone)
}

fn tell_for_leader(w: &World, i: usize, msg: Message) -> String {
    let t = &w.people[i].player.personality.traits;
    let ego = t.get(Trait::Ego);
    match msg {
        Message::Compete if ego >= 13.0 => "He looks at the backup at his position, then back at you.".into(),
        Message::Purge => "His jaw sets. He doesn't look at anyone.".into(),
        Message::WinNow => "He nods, slowly, then once more.".into(),
        Message::Honour => "He glances toward the back row, where the old coach's guys sit.".into(),
        _ => "He gives you nothing either way.".into(),
    }
}

/// The captains (or, before any vote, the room's leaders by standing).
pub fn captains_of(w: &World, team: Team) -> Vec<usize> {
    let ids: Vec<usize> = match team {
        Team::College(p) => w.programs[p].captains.clone(),
        Team::Nfl(t) => w.clubs.get(t).map(|c| c.captains.clone()).unwrap_or_default(),
        Team::High(_) => Vec::new(),
    };
    let roster = team.roster(w);
    let mut out: Vec<usize> = ids.into_iter().filter(|i| roster.contains(i)).collect();
    if out.len() < 3 {
        let mut by: Vec<(usize, f64)> = roster
            .iter()
            .copied()
            .filter(|i| !out.contains(i) && !facts::leaving_now(w, *i))
            .map(|i| {
                let p = &w.people[i];
                let lead = p.player.personality.traits.get(Trait::Leadership);
                let st = if p.last_role == Some(PlayingTime::Starter) { 4.0 } else { 0.0 };
                let age = p.player.age as f64 * 0.3;
                (i, lead + st + age)
            })
            .collect();
        by.sort_by(|a, b| b.1.total_cmp(&a.1));
        out.extend(by.into_iter().take(3 - out.len()).map(|x| x.0));
    }
    out
}

/// The facts every takeover line can use.
pub fn base_facts(w: &World, team: Team) -> Facts {
    let mut f = Facts::new();
    f.set("team", team.name(w));
    f.set("town", team.town(w));
    f.set("you", scene::your_last(w));
    f.set("coach", format!("Coach {}", scene::your_last(w)));
    f.set("year", w.year.to_string());
    if let Some(o) = w.folk.takeover.as_ref().and_then(|t| t.old_coach) {
        let n = &w.coaches[o].name;
        f.set("old", n.clone());
        f.set("old_last", n.split_whitespace().last().unwrap_or(n).to_string());
    }
    if let Some((r, game, a, b)) = facts::rival(w, team) {
        f.set("rival", r.name(w));
        f.set("rivalry", game);
        if a + b > 0 {
            f.set("series", format!("{a}-{b}"));
        }
    }
    if let Some(last) = facts::seasons(w, team, 1).last() {
        f.set("last_record", format!("{}-{}", last.wins, last.losses));
    }
    f
}

// ---------------------------------------------------------------------------
// Item 18: how the world took your hire.

fn hire_reaction(w: &mut World, s: &mut Scene, team: Team) {
    let fit = facts::fit(w, team);
    let grade = facts::hire_grade(fit);
    let rep = facts::your_rep(w);
    let name = team.name(w);
    let road = w.career.as_ref().map_or(String::new(), |c| c.you.road.clone());
    let you = scene::your_name(w);
    let fans = match team {
        Team::College(p) => w.programs[p].fans,
        Team::Nfl(t) => w.clubs.get(t).map_or(0.5, |c| (c.office.fans + 1.0) / 2.0),
        Team::High(h) => w.high_schools[h].prestige,
    };
    s.who.push(Who::Group("The press and the fans".into()));
    let national = match team {
        Team::High(_) => format!("The {} ran your hire on page one of the sports section", local_paper(w, team)),
        _ => "The national writers handed out their grades by noon".to_string(),
    };
    s.narrate(format!("{national}. Yours: {grade}."));
    let why = match fit {
        f if f > 0.4 => format!("\"{name} landed a coach above its weight. The question is how long it keeps him.\""),
        f if f > 0.1 => format!("\"A strong hire. {you} fits what {name} needs.\""),
        f if f > -0.2 => format!("\"Solid if unspectacular. {name} went with a known quantity.\""),
        f if f > -0.5 => format!("\"A reach. {name} wanted a bigger name and couldn't get one.\""),
        _ => format!("\"A head-scratcher. Nobody on the shortlists had {you} on theirs.\""),
    };
    s.other("A national writer", why);
    let road_line = if road.is_empty() {
        String::new()
    } else {
        format!(" They wrote about your road: {}.", road.to_lowercase())
    };
    let fan_words = match (fit, fans) {
        (f, x) if f > 0.2 && x > 0.6 => "The message boards are giddy: season ticket renewals jumped by lunchtime.",
        (f, _) if f > 0.2 => "Fans are pleased, if a little surprised.",
        (f, x) if f < -0.3 && x > 0.6 => "The fan base is split, and the loudest voices are angry: \"We're supposed to be a destination job.\"",
        (f, _) if f < -0.3 => "Fans don't know much about you. They'll wait and see.",
        _ => "Fans are cautiously in: most want to see the first season.",
    };
    s.narrate(format!("{fan_words}{road_line}"));
    let recruits = match team {
        Team::Nfl(_) => match fit {
            f if f > 0.2 => "Agents called the general manager to ask what you run. Free agents talk to each other about new coaches.",
            _ => "Agents are waiting to see your coordinators before they tell clients anything.",
        },
        Team::High(_) => "Parents of next year's freshmen are asking the middle school coaches what kind of coach you are.",
        _ => match fit {
            f if f > 0.2 => "Recruits posted about you within hours. Two uncommitted ones followed your new account.",
            f if f > -0.2 => "Recruits are waiting for your first calls before they say anything public.",
            _ => "A few recruits stopped answering their texts from the last staff. Your staff will have work to do.",
        },
    };
    s.narrate(recruits.to_string());
    super::note(w, "media", "The press".into(), format!("Hire grade: {grade}. {fan_words}"), None);
    if let Team::College(p) = team {
        // Fans' first read of you moves the money a little (people topic 15).
        let c = &mut w.programs[p].collective;
        c.base *= 1.0 + 0.02 * fit;
    }
    let _ = rep;
    s.pick(1, "Read it and close the laptop.", "Listen", "calm", "Nothing changes; it's filed.");
    s.stops = true;
}

fn local_paper(w: &World, team: Team) -> String {
    cast::all(w, team, Role::Reporter).first().and_then(|c| c.reporter.as_ref().map(|r| r.outlet.clone())).unwrap_or_else(|| "local paper".into())
}

// ---------------------------------------------------------------------------
// Items 3 and 15: the holdover staff.

fn holdover_open(w: &mut World, s: &mut Scene, team: Team) {
    if s.a == u64::MAX {
        return holdover_room(w, s, team);
    }
    let c = s.a as usize;
    let Some(coach) = w.coaches.get(c) else {
        s.narrate("He'd already cleaned out his office.");
        s.end();
        return;
    };
    if facts::team_of_coach(w, c) != Some(team) {
        s.narrate(format!("{} took another job before you could meet him.", coach.name));
        s.end();
        return;
    }
    let old = w.folk.takeover.as_ref().and_then(|t| t.old_coach);
    let role = facts::coach_role(w, c);
    let years = coach.job.map_or(0, |j| w.year - j.since);
    let mentor_old = old.is_some_and(|o| coach.mentor == Some(o));
    s.who.push(Who::Coach(c));
    let mut pen = scene::pen(w, s, 1);
    let spk = Speaker::coach(w, c);
    let mut f = base_facts(w, team);
    f.set("name", coach.name.split_whitespace().next().unwrap_or("").to_string());
    f.set("role", role.clone());
    f.set("years", years.to_string());
    s.narrate(format!(
        "{} has been the {role} here {}. {}",
        coach_link(w, c),
        match years {
            0 => "less than a year".to_string(),
            1 => "one season".to_string(),
            n => format!("{n} seasons"),
        },
        if mentor_old {
            "He came up under the man you replaced, and everyone knows it."
        } else {
            "He wasn't the old coach's hire from his own tree, which may help him."
        }
    ));
    let pitch = pen.line(&["takeover.holdover.pitch", "?takeover.holdover.detail"], &spk, &f);
    pen.put(w);
    s.them(&coach.name.clone(), if pitch.is_empty() { format!("I know these kids, Coach. I'd like the chance to keep coaching them.") } else { pitch }, Some(holdover_tell(w, c)));
    // His angle on his own players (item 3): he oversells them.
    let claim = oversell(w, team, c);
    if let Some((i, text)) = claim {
        let name = w.coaches[c].name.clone();
        s.them(&name, text, None);
        let id = w.people[i].player.id;
        if let Some(t) = w.folk.takeover.as_mut() {
            t.claims.push((name.clone(), id, 1));
        }
        super::note(
            w,
            "staff",
            format!("{}, {role}", coach_link(w, c)),
            format!("Says {} is ready to be a big part of it.", player_link(w, i)),
            Some("He wants to keep his job: he sells his own players.".into()),
        );
    }
    s.pick(20, "Keep him. Tell him he's coaching for his job this spring.", "Tell him straight", "firm", "He stays, knows where he stands, and works.");
    s.pick(21, "Keep him, and say you want his read on the roster every week.", "Hand him something", "warm", "He stays and starts to trust you; his players notice.");
    s.pick(22, "Thank him and let him go. You're bringing your own man.", "Tell him straight", "calm", "He's gone; you owe his contract, and his players feel it.");
}

fn holdover_tell(w: &World, c: usize) -> String {
    let co = &w.coaches[c];
    if co.personality.ambition >= 14.0 {
        "He mentions twice that he's had calls about other jobs.".into()
    } else if co.personality.loyalty >= 14.0 {
        "He talks about the players by their first names, and about the town like it's home.".into()
    } else if co.personality.temper >= 14.0 {
        "There's an edge in his voice when the old staff comes up.".into()
    } else {
        "He keeps it professional and doesn't oversell himself.".into()
    }
}

/// A holdover coach's pitch for one of his players: the backup he says is
/// ready, an angle the takeover file keeps for later.
fn oversell(w: &World, team: Team, c: usize) -> Option<(usize, String)> {
    let co = &w.coaches[c];
    let job = co.job?;
    let groups: Vec<crate::world::coach::Group> = match job.role {
        crate::world::coach::Role::OffensiveCoordinator => crate::world::coach::Group::ALL.iter().copied().filter(|g| g.offense()).collect(),
        crate::world::coach::Role::DefensiveCoordinator => crate::world::coach::Group::ALL.iter().copied().filter(|g| !g.offense() && !g.special()).collect(),
        crate::world::coach::Role::PositionCoach(g) => vec![g],
        _ => return None,
    };
    let roster = team.roster(w);
    let best = roster
        .iter()
        .copied()
        .filter(|&i| groups.contains(&crate::world::coach::Group::of(w.people[i].player.position)))
        .filter(|&i| matches!(w.people[i].last_role, Some(PlayingTime::Backup | PlayingTime::Redshirt | PlayingTime::Rotation)))
        .max_by(|&a, &b| {
            let fa = w.people[a].stars as f64 + super::roll(w, c as u64, w.people[a].player.id as u64);
            let fb = w.people[b].stars as f64 + super::roll(w, c as u64, w.people[b].player.id as u64);
            fa.total_cmp(&fb)
        })?;
    let p = &w.people[best];
    let text = format!(
        "And {} is ready. {} was {} last year, but you watch him in bowl practice and he's a starter. He's going to surprise people.",
        player_link(w, best),
        p.name.split_whitespace().next().unwrap_or(""),
        facts::role_words(p.last_role)
    );
    Some((best, text))
}

fn holdover_room(w: &mut World, s: &mut Scene, team: Team) {
    let staff = facts::assistants(w, team);
    let old = w.folk.takeover.as_ref().and_then(|t| t.old_coach);
    let mut doc = Doc {
        title: "The holdover staff".into(),
        note: "Who's still here from the last staff, and what the building says about each.".into(),
        cols: vec!["Coach".into(), "Job".into(), "Here since".into(), "Reputation".into(), "His tree".into(), "The word".into()],
        rows: Vec::new(),
    };
    if let Team::High(_) = team {
        let staff = w.career.as_ref().map(|c| c.hs_staff.clone()).unwrap_or_default();
        s.who.push(Who::Group("The holdover staff".into()));
        s.narrate(format!(
            "{} assistants from the last staff, most of them teachers here. They coach for a stipend on top of their teaching contracts; letting one go takes his stipend, not his classroom.",
            staff.len()
        ));
        for a in &staff {
            doc.rows.push(DocRow {
                pid: None,
                cells: vec![
                    a.name.clone(),
                    match a.coordinator {
                        Some(true) => "offensive coordinator".into(),
                        Some(false) => "defensive coordinator".into(),
                        None => crate::world::coach::Group::ALL[a.group as usize].coach_title().into(),
                    },
                    a.since.to_string(),
                    a.subject.clone().map_or("walk-on coach".into(), |x| format!("teaches {x}")),
                    String::new(),
                    format!("${} stipend", a.stipend),
                ],
            });
        }
        s.docs.push(doc);
        s.pick(30, "Keep them all through spring ball and see who works.", "Listen", "calm", "Nobody's stipend changes; you learn who's good by doing.");
        s.pick(31, "Tell them every stipend is open again in May.", "Warn", "firm", "They work harder this spring, and two of them stop smiling at you in the hallway.");
        return;
    }
    s.who.push(Who::Group("The holdover position coaches".into()));
    s.narrate(format!(
        "{} position coaches are still on staff. A new head coach usually keeps one or two who know the roster and the recruits, and brings his own for the rest.",
        staff.iter().filter(|x| matches!(x.1, crate::world::coach::Role::PositionCoach(_))).count()
    ));
    for (c, role) in staff.iter().filter(|x| matches!(x.1, crate::world::coach::Role::PositionCoach(_))) {
        let co = &w.coaches[*c];
        let since = co.job.map_or(w.year, |j| j.since);
        let tree = if old.is_some_and(|o| co.mentor == Some(o)) { "the old coach's" } else { "his own" };
        let word = staff_word(w, *c);
        doc.rows.push(DocRow {
            pid: None,
            cells: vec![coach_link(w, *c), role.name().into(), since.to_string(), crate::world::folk::rep_words(co.reputation).into(), tree.into(), word],
        });
    }
    s.docs.push(doc);
    s.pick(32, "Keep the whole room through spring. Decide after.", "Listen", "calm", "Continuity for the players and recruits; you can still make changes from the staff screen.");
    s.pick(33, "Keep the recruiters and the ones the players trust; let the rest go.", "Tell him straight", "firm", "The weakest go; you owe their contracts, and you hire to fill.");
    s.pick(34, "Clean house. You're bringing your own staff.", "Tell him straight", "fiery", "Every holdover goes. The players close to them take it hard, and their recruits wobble.");
}

/// What the building says about an assistant, from what anyone can see:
/// how his players grew, how he recruits, how long he lasts.
fn staff_word(w: &World, c: usize) -> String {
    let co = &w.coaches[c];
    let teach = co.units.iter().map(|u| u.developed()).sum::<f64>();
    let recruit_ties = co.ties.values().filter(|&&v| v > 0.4).count();
    let mut words: Vec<&str> = Vec::new();
    if teach > 1.5 {
        words.push("his players get better");
    } else if teach < -1.0 {
        words.push("his room hasn't improved");
    }
    if recruit_ties >= 3 {
        words.push("knows the high school coaches in the area");
    }
    if co.personality.loyalty >= 14.0 {
        words.push("wants to stay");
    }
    if co.personality.ambition >= 14.0 {
        words.push("wants to be a coordinator");
    }
    if words.is_empty() {
        words.push("does his job, no more talk than that");
    }
    let s = words.join("; ");
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn holdover_answer(w: &mut World, s: &mut Scene, team: Team, code: u32) {
    match code {
        20 | 21 => {
            let c = s.a as usize;
            w.folk.mine.nudge(super::coach_key(c), if code == 21 { 0.12 } else { 0.05 });
            if let Some(t) = w.folk.takeover.as_mut() {
                t.kept.push(c);
            }
            s.you(&scene::your_name(w), if code == 21 { "You stay. And I want your read on the roster every week." } else { "You stay. This spring is your interview." });
            let name = w.coaches[c].name.clone();
            s.them(&name, if code == 21 { "You'll get it. Every week." } else { "That's fair. That's all I'm asking for." }, None);
            s.outcome.push(format!("You kept {} on.", coach_link(w, c)));
        }
        22 => {
            let c = s.a as usize;
            let name = w.coaches[c].name.clone();
            s.you(&scene::your_name(w), "I'm bringing my own man. I'll make sure you're paid and I'll make calls for you.");
            s.them(&name, "I figured. Thanks for telling me to my face.", None);
            let out = super::staffing::fire(w, c, "let go by the new head coach");
            s.outcome.push(out);
            if let Some(t) = w.folk.takeover.as_mut() {
                t.let_go.push(c);
            }
        }
        30 | 32 => {
            s.outcome.push("You kept the holdover staff through spring.".into());
        }
        31 => {
            for a in w.career.as_mut().map(|c| &mut c.hs_staff).into_iter().flatten() {
                a.teaching = (a.teaching + 0.2).min(20.0);
            }
            w.folk.mine.hard = (w.folk.mine.hard + 0.1).min(1.0);
            s.outcome.push("You told the staff every stipend is open again in May.".into());
        }
        33 | 34 => {
            let staff: Vec<usize> = facts::assistants(w, team)
                .into_iter()
                .filter(|x| matches!(x.1, crate::world::coach::Role::PositionCoach(_)))
                .map(|x| x.0)
                .collect();
            let mut gone = Vec::new();
            for c in staff {
                let keep = code == 33 && {
                    let co = &w.coaches[c];
                    let ties = co.ties.values().filter(|&&v| v > 0.4).count();
                    ties >= 3 || co.units.iter().map(|u| u.developed()).sum::<f64>() > 1.0
                };
                if !keep {
                    gone.push(w.coaches[c].name.clone());
                    super::staffing::fire(w, c, "let go by the new head coach");
                    if let Some(t) = w.folk.takeover.as_mut() {
                        t.let_go.push(c);
                    }
                }
            }
            s.outcome.push(if gone.is_empty() {
                "Everyone you looked at stays.".into()
            } else {
                format!("You let go {}. Hire their replacements from the staff screen.", gone.join(", "))
            });
        }
        _ => {}
    }
    s.end();
}

// ---------------------------------------------------------------------------
// Item 5: the state of the team.

fn staff_report(w: &mut World, s: &mut Scene, team: Team) {
    let roster = team.roster(w);
    let source = report_source(w, team);
    s.who.push(source.0.clone());
    s.narrate(format!(
        "{} puts last season's film on and walks you through the roster, position by position. {}",
        source.1,
        match team {
            Team::Nfl(_) => "The pro scouts sit in: in the NFL the new staff grades the roster with them, before the combine.",
            Team::College(_) => "Your read is wide on day one: film and what the holdovers say. Spring ball will sharpen it.",
            Team::High(_) => "Most of it is last fall's film and what the old staff wrote on the board.",
        }
    ));
    let mut by_pos: Vec<(crate::positions::Position, Vec<usize>)> = Vec::new();
    for pos in crate::positions::Position::ALL {
        let mut v: Vec<usize> = roster.iter().copied().filter(|&i| w.people[i].player.position == pos).collect();
        if v.is_empty() {
            continue;
        }
        v.sort_by(|&a, &b| standing(w, b).total_cmp(&standing(w, a)));
        by_pos.push((pos, v));
    }
    let mut doc = Doc {
        title: "Key players".into(),
        note: "The best one or two at each position as your staff reads them now. Your read sharpens after spring ball.".into(),
        cols: vec!["Pos".into(), "Player".into(), "Class".into(), "Last year".into(), "Leaves after this season".into()],
        rows: Vec::new(),
    };
    let mut holes: Vec<String> = Vec::new();
    for (pos, v) in &by_pos {
        let take = if v.len() >= 4 { 2 } else { 1 };
        for &i in v.iter().take(take) {
            let p = &w.people[i];
            doc.rows.push(DocRow {
                pid: Some(p.player.id),
                cells: vec![
                    pos.name().into(),
                    player_link(w, i),
                    facts::class_words(w, i),
                    facts::role_words(p.last_role).into(),
                    if facts::leaving(w, i) { "yes".into() } else { String::new() },
                ],
            });
        }
        let starters_leaving = v.iter().take(take).filter(|&&i| facts::leaving(w, i)).count();
        let next = v.iter().filter(|&&i| !facts::leaving(w, i)).count();
        if starters_leaving > 0 && next <= starters_leaving {
            holes.push(pos.name().to_string());
        }
    }
    s.docs.push(doc);
    if holes.is_empty() {
        s.narrate("No position empties out when the seniors leave. The depth is the old staff's best work.".to_string());
    } else {
        s.narrate(format!(
            "The holes once this year's seniors leave: {}. That's where recruiting {} starts.",
            holes.join(", "),
            if matches!(team, Team::Nfl(_)) { "becomes free agency and the draft, and it" } else { "" }
        ));
    }
    super::note(w, "roster", source.1.clone(), format!("Holes after this season: {}.", if holes.is_empty() { "none".into() } else { holes.join(", ") }), None);
    s.pick(40, "Start the film study yourself tonight.", "Ask", "calm", "Your own eyes on the roster: your read starts sharper.");
    s.pick(41, "Have the staff grade every player by Friday.", "Hand him something", "firm", "The staff's grades come in; your read leans on their eyes.");
}

fn report_source(w: &World, team: Team) -> (Who, String) {
    match team {
        Team::Nfl(_) => {
            let c = cast::get(w, CastRef::new(team, Role::Personnel, 0));
            (Who::Cast(c.r.token()), format!("{}, the {}", c.link(), c.title.to_lowercase()))
        }
        _ => match facts::assistants(w, team).first() {
            Some(&(c, r)) => (Who::Coach(c), format!("{}, the {}", coach_link(w, c), r.name())),
            None => (Who::Group("Your staff".into()), "Your staff".into()),
        },
    }
}

/// How the staff ranks a player in his room: what anyone saw of him (his
/// role, his play, his stars), never a hidden number.
pub fn standing(w: &World, i: usize) -> f64 {
    let p = &w.people[i];
    let role = match p.last_role {
        Some(PlayingTime::Starter) => 30.0,
        Some(PlayingTime::Rotation) => 18.0,
        Some(PlayingTime::Backup) => 8.0,
        _ => 4.0,
    };
    role + p.last_performance.unwrap_or(40.0) * 0.5 + p.stars as f64 * 3.0 + p.player.age as f64 * 0.3
}

fn report_answer(w: &mut World, s: &mut Scene, team: Team, code: u32) {
    match code {
        40 => {
            s.outcome.push("You watched the film yourself into the night.".into());
            if let Some(c) = w.career.as_ref().map(|c| c.coach) {
                w.coaches[c].stat_buzz += 0.0;
            }
        }
        41 => s.outcome.push("The staff will grade every player by Friday.".into()),
        50 => {
            // Leaders: meet the captains together.
            for c in captains_of(w, team) {
                let p = &mut w.people[c];
                p.trust = (p.trust + 0.04).min(1.0);
            }
            s.outcome.push("You met the leaders as a group. They'll carry your message, if they believe it.".into());
        }
        51 => {
            for c in captains_of(w, team) {
                let p = &mut w.people[c];
                p.trust = (p.trust + 0.07).min(1.0);
            }
            w.folk.hours += 3.0;
            s.outcome.push("You met each leader alone. It took an afternoon and they noticed.".into());
        }
        60 => {
            // Academic: mandatory study hall.
            w.folk.rules.add(super::rules::RuleKind::StudyHall, 2);
            s.outcome.push("Study hall is now a team rule for anyone under a 2.5.".into());
        }
        61 => s.outcome.push("The academic staff handles it as they did.".into()),
        70 => {
            w.folk.mine.hard = (w.folk.mine.hard - 0.05).max(-1.0);
            s.outcome.push("Nobody rushes back. The trainers heard you say it.".into());
            if let Some(c) = cast::has(w, team, Role::Trainer).then(|| cast::get(w, CastRef::new(team, Role::Trainer, 0))) {
                w.folk.mine.nudge(c.key(), 0.1);
            }
        }
        71 => {
            s.outcome.push("You told the trainers you want your starters back for spring ball.".into());
            if let Some(c) = cast::has(w, team, Role::Trainer).then(|| cast::get(w, CastRef::new(team, Role::Trainer, 0))) {
                w.folk.mine.nudge(c.key(), -0.08);
            }
            w.folk.pushed_rehab = true;
        }
        _ => s.outcome.push("Filed in the takeover file.".into()),
    }
    s.end();
}

// ---------------------------------------------------------------------------
// Item 6: the leaders and how they took the change.

fn leaders(w: &mut World, s: &mut Scene, team: Team) {
    let caps = captains_of(w, team);
    let old = w.folk.takeover.as_ref().and_then(|t| t.old_coach);
    s.who.push(Who::Group("The team's leaders".into()));
    s.narrate(match team {
        Team::Nfl(_) => "You call the captains one by one. Most are home for the offseason; one picks up from a golf course.".to_string(),
        _ => "The captains and the voices of the room come by one at a time.".to_string(),
    });
    let mut doc = Doc {
        title: "The room's voices".into(),
        note: "How each took the change, in his own words and in what your staff saw.".into(),
        cols: vec!["Player".into(), "Pos".into(), "First reaction".into()],
        rows: Vec::new(),
    };
    let mut pen = scene::pen(w, s, 1);
    let mut f = base_facts(w, team);
    for &i in &caps {
        let p = &w.people[i];
        let tie = tie_to_old(w, i, old);
        let t = &p.player.personality.traits;
        let spk = Speaker::player(w, i);
        f.set("name", p.name.split_whitespace().next().unwrap_or("").to_string());
        let key = if tie > 0.5 {
            "takeover.leader.close_to_old"
        } else if t.get(Trait::Ego) >= 14.0 {
            "takeover.leader.ego"
        } else if t.get(Trait::Leadership) >= 14.0 {
            "takeover.leader.steady"
        } else {
            "takeover.leader.waiting"
        };
        let line = pen.line(&[key], &spk, &f);
        let words = if line.is_empty() { leader_fallback(tie, t.get(Trait::Ego), t.get(Trait::Leadership)) } else { format!("\"{line}\"") };
        doc.rows.push(DocRow { pid: Some(p.player.id), cells: vec![player_link(w, i), p.player.position.name().into(), words.clone()] });
        super::note(w, "room", player_link(w, i), words, if tie > 0.5 { Some("Close to the old coach.".into()) } else { None });
    }
    pen.put(w);
    s.docs.push(doc);
    s.pick(50, "Meet them together and ask them to carry the message.", "Hand him something", "warm", "Their trust in you rises a little.");
    s.pick(51, "Spend an afternoon with each one alone.", "Ask", "warm", "More trust, at the cost of an afternoon.");
}

fn leader_fallback(tie: f32, ego: f64, lead: f64) -> String {
    if tie > 0.5 {
        "\"He recruited me. I'll give you everything, Coach, but I won't pretend I'm not hurting.\"".into()
    } else if ego >= 14.0 {
        "\"Just want to know what my role is. That's it.\"".into()
    } else if lead >= 14.0 {
        "\"Whatever you need from the guys, tell me. I'll handle the room.\"".into()
    } else {
        "\"We'll see. Guys are waiting to see what you're about.\"".into()
    }
}

// ---------------------------------------------------------------------------
// Item 11: the trainer's report.

fn trainer(w: &mut World, s: &mut Scene, team: Team) {
    let has = cast::has(w, team, Role::Trainer);
    let roster = team.roster(w);
    if has {
        let c = cast::get(w, CastRef::new(team, Role::Trainer, 0));
        s.who.push(Who::Cast(c.r.token()));
        s.narrate(format!("{}, the {}, has the injury list printed out.", c.link(), c.title.to_lowercase()));
    } else {
        s.who.push(Who::Group("The team".into()));
        s.narrate("There's no athletic trainer at this school. You ask the players who's hurt, and the honest ones tell you.".to_string());
    }
    let mut doc = Doc {
        title: "Who's hurt".into(),
        note: "Who is out, who is rehabbing, and when each is expected back.".into(),
        cols: vec!["Player".into(), "Pos".into(), "What".into(), "Back".into()],
        rows: Vec::new(),
    };
    for &i in &roster {
        let p = &w.people[i];
        let (what, back) = if let Some(c) = &p.pro {
            if c.out_weeks == 0 && !c.pup {
                continue;
            }
            (if c.pup { "rehab, on the PUP list".to_string() } else { "injured".to_string() }, if c.out_weeks > 0 { format!("about {} weeks", c.out_weeks) } else { "by camp".into() })
        } else {
            let Some(h) = p.player.history.last() else { continue };
            use crate::player::DevelopmentEvent as Ev;
            if h.events.has(Ev::MajorInjury) {
                let surgery = super::roll(w, 0x5E6, p.player.id as u64) < 0.4;
                (
                    if surgery { "a major injury; surgery still to come".into() } else { "rehab from a major injury".into() },
                    if surgery { "summer, maybe fall camp".into() } else { "late spring".into() },
                )
            } else if h.events.has(Ev::ModerateInjury) && h.games_missed > 2 {
                ("rehab, finishing up".into(), "spring ball, limited".into())
            } else {
                continue;
            }
        };
        doc.rows.push(DocRow { pid: Some(p.player.id), cells: vec![player_link(w, i), p.player.position.name().into(), what, back] });
    }
    let n = doc.rows.len();
    if n == 0 {
        s.narrate("Nobody is hurt badly. A clean list in February is rare.".to_string());
    }
    s.docs.push(doc);
    s.pick(70, "Nobody rushes back. Healthy for fall is the only goal.", "Tell him straight", "calm", "The trainers trust you; nobody is pushed.");
    s.pick(71, "I want my starters back for spring ball if there's any way.", "Challenge", "firm", "Some come back sooner; the re-injury risk rises, and the trainers notice.");
}

// ---------------------------------------------------------------------------
// Item 10: the academic and conduct report.

fn academic(w: &mut World, s: &mut Scene, team: Team) {
    let roster = team.roster(w);
    let c = cast::has(w, team, Role::Academic).then(|| cast::get(w, CastRef::new(team, Role::Academic, 0)));
    match (&c, team) {
        (_, Team::Nfl(_)) => {
            s.who.push(Who::Group("The security director".into()));
            s.narrate("The club's security director keeps the conduct file: open cases, players on the league's radar and the ones he keeps an eye on.".to_string());
        }
        (Some(c), _) => {
            s.who.push(Who::Cast(c.r.token()));
            s.narrate(format!("{}, the {}, brings the grade reports and the conduct file.", c.link(), c.title.to_lowercase()));
        }
        _ => {}
    }
    let mut doc = Doc {
        title: if matches!(team, Team::Nfl(_)) { "Conduct".into() } else { "Grades and conduct".into() },
        note: "Who's at risk of being ineligible, and the cases still open.".into(),
        cols: vec!["Player".into(), "Pos".into(), "What".into()],
        rows: Vec::new(),
    };
    for &i in &roster {
        let p = &w.people[i];
        let mut what: Vec<String> = Vec::new();
        if p.cal.academic_sit || p.cal.fall_ineligible || p.cal.spring_ineligible {
            what.push("academically ineligible".into());
        } else if p.stage == Stage::College && super::roll(w, 0xAC4D, p.player.id as u64) < 0.08 + 0.02 * (10.0 - p.player.personality.traits.get(Trait::Professionalism)).max(0.0) {
            what.push("below a 2.0 last term: on the watch list".into());
        } else if p.stage == Stage::HighSchool && p.gpa_trend < -0.15 {
            what.push("failing a class: no pass, no play if it holds".into());
        }
        if p.cal.trouble {
            what.push("suspended".into());
        } else if p.on_notice {
            what.push("on notice after an earlier incident".into());
        }
        if let Some(case) = w.folk.courts.open_case(p.player.id) {
            what.push(case);
        }
        if !what.is_empty() {
            doc.rows.push(DocRow { pid: Some(p.player.id), cells: vec![player_link(w, i), p.player.position.name().into(), what.join("; ")] });
        }
    }
    if doc.rows.is_empty() {
        s.narrate("A clean file. Nobody's grades are a problem and no case is open.".to_string());
    }
    s.docs.push(doc);
    if !matches!(team, Team::Nfl(_)) {
        s.pick(60, "Mandatory study hall for anyone under a 2.5, starting Monday.", "Tell him straight", "firm", "A team rule: fewer players lose eligibility; some resent the hours.");
        s.pick(61, "Leave it with the academic staff.", "Listen", "calm", "Nothing changes.");
    } else {
        s.pick(62, "Noted. Keep me told before the league is.", "Listen", "calm", "You hear about trouble sooner.");
    }
}

// ---------------------------------------------------------------------------
// Items 7 and 8: who might leave, and the one-on-ones.

fn flight_risks(w: &mut World, s: &mut Scene, team: Team) {
    let roster = team.roster(w);
    let old = w.folk.takeover.as_ref().and_then(|t| t.old_coach);
    let mut risks: Vec<(usize, f64, String)> = Vec::new();
    for &i in &roster {
        let p = &w.people[i];
        if facts::leaving_now(w, i) {
            continue;
        }
        let tie = tie_to_old(w, i, old) as f64;
        let t = &p.player.personality.traits;
        let unhappy_role = matches!(p.last_role, Some(PlayingTime::Backup | PlayingTime::Rotation)) && t.get(Trait::Ambition) >= 12.0;
        let courted = p.stars >= 4 || p.last_performance.unwrap_or(0.0) > 70.0;
        let lean = p.cal.lean;
        let mut score = 0.0;
        let mut why = Vec::new();
        if tie > 0.4 {
            score += tie;
            why.push("close to the old coach");
        }
        if unhappy_role {
            score += 0.5;
            why.push("unhappy with his role");
        }
        if courted && p.stage == Stage::College {
            score += 0.3;
            why.push("other programs are already calling");
        }
        if let Some(c) = &p.pro
            && c.contract.as_ref().is_some_and(|k| k.years.last().is_some_and(|y| y.year <= w.year + 1))
            && t.get(Trait::Greed) >= 13.0
        {
            score += 0.5;
            why.push("wants a new deal");
        }
        if p.mood < -0.3 {
            score += 0.3;
            why.push("hasn't been happy for a while");
        }
        score += 3.0 * lean;
        if score > 0.55 && !why.is_empty() {
            risks.push((i, score, why.join(", ")));
        }
    }
    risks.sort_by(|a, b| b.1.total_cmp(&a.1));
    risks.truncate(8);
    let (src, srcname) = report_source(w, team);
    s.who.push(src);
    s.narrate(format!(
        "{srcname} has the list ready. {}",
        match team {
            Team::College(_) => {
                let win = w.folk.window.map(|x| x.1.pretty());
                match win {
                    Some(d) => format!("The coaching-change window opens {d}: fifteen days when any of your players can enter the portal and play next fall."),
                    None => "There's no coaching-change window in this era, but the next regular window comes all the same.".into(),
                }
            }
            Team::Nfl(_) => "In the league it means trade requests, holdouts and veterans who'd rather be released than rebuild.".into(),
            Team::High(_) => "A coaching change is when families shop for another school, where the state's transfer rules allow it.".into(),
        }
    ));
    let mut doc = Doc {
        title: "Flight risks".into(),
        note: "Who your staff thinks could leave, and why.".into(),
        cols: vec!["Player".into(), "Pos".into(), "Last year".into(), "Why".into()],
        rows: Vec::new(),
    };
    for (i, _, why) in &risks {
        let p = &w.people[*i];
        doc.rows.push(DocRow {
            pid: Some(p.player.id),
            cells: vec![player_link(w, *i), p.player.position.name().into(), facts::role_words(p.last_role).into(), why.clone()],
        });
    }
    let list: Vec<(u32, String)> = risks.iter().map(|(i, _, why)| (w.people[*i].player.id, why.clone())).collect();
    for (i, _, why) in &risks {
        super::note(w, "flight", srcname.clone(), format!("{} could leave: {why}.", player_link(w, *i)), None);
    }
    if let Some(t) = w.folk.takeover.as_mut() {
        t.risks = list;
    }
    s.docs.push(doc);
    if risks.is_empty() {
        s.narrate("Nobody stands out. Every coaching change costs somebody, but your staff can't name him.".to_string());
        s.pick(80, "Good. Keep your ears open.", "Listen", "calm", "Filed.");
    } else {
        s.pick(81, format!("I'll meet each of them myself this week ({}).", risks.len().min(5)), "Ask", "warm", "One-on-ones on your calendar, before the window: what you say there moves each one.");
        s.pick(82, "Have their position coaches talk to them.", "Hand him something", "calm", "A smaller push to stay, and no time from your week.");
    }
}

fn flight_answer(w: &mut World, s: &mut Scene, team: Team, code: u32) {
    let risks = w.folk.takeover.as_ref().map(|t| t.risks.clone()).unwrap_or_default();
    match code {
        81 => {
            let mut n = 0;
            for (k, (id, why)) in risks.iter().take(5).enumerate() {
                let Some(i) = super::person(w, *id) else { continue };
                let d = day_at(w.today, 1 + k as i32 / 2);
                let title = format!("One-on-one with {}", w.people[i].name);
                let mut sc = Scene::new(Kind::OneOnOne, d, title);
                sc.a = *id as u64;
                sc.b = super::talks::Subject::Future as u64;
                sc.place = "Your office".into();
                sc.team = Some(team);
                let _ = why;
                scene::add(w, sc);
                n += 1;
            }
            s.outcome.push(format!("{n} one-on-ones are on your calendar this week."));
        }
        82 => {
            for (id, _) in &risks {
                if let Some(i) = super::person(w, *id) {
                    position_coach_word(w, i);
                }
            }
            s.outcome.push("The position coaches will talk to them.".into());
        }
        _ => {}
    }
    s.end();
}

/// A position coach's word with a player: a small push to stay, by his
/// relationships rating.
fn position_coach_word(w: &mut World, i: usize) {
    let rel = w.people[i]
        .college_staff
        .last()
        .and_then(|x| x.3)
        .and_then(|c| w.coaches.get(c))
        .map_or(10.0, |c| c.ability().relationships);
    let f = (1.0 - 0.02 * (rel - 8.0)).clamp(0.85, 1.05);
    let today = w.today;
    let p = &mut w.people[i];
    if p.stage == Stage::College && p.cal.lean > 0.0 {
        p.cal.lean *= f;
        p.cal.lean_events.push((today, "his position coach asked him to stay", f));
    }
    p.trust = (p.trust + 0.01).min(1.0);
}

// ---------------------------------------------------------------------------
// Item 9: commits who wobble.

fn recruiting(w: &mut World, s: &mut Scene, team: Team) {
    let Team::College(p) = team else {
        s.end();
        return;
    };
    let c = cast::has(w, team, Role::Personnel).then(|| cast::get(w, CastRef::new(team, Role::Personnel, 0)));
    match &c {
        Some(c) => {
            s.who.push(Who::Cast(c.r.token()));
            s.narrate(format!("{}, the {}, has the board up: next year's commits and the top targets.", c.link(), c.title.to_lowercase()));
        }
        None => {
            s.who.push(Who::Group("The recruiting coordinator".into()));
            s.narrate("The recruiting coordinator has the board up: next year's commits and the top targets.".to_string());
        }
    }
    let old = w.folk.takeover.as_ref().and_then(|t| t.old_coach);
    let mut doc = Doc {
        title: "Commits and targets".into(),
        note: "Who's committed, who's wavering and why.".into(),
        cols: vec!["Recruit".into(), "Pos".into(), "Stars".into(), "Status".into(), "Why".into()],
        rows: Vec::new(),
    };
    let mut wobbling = 0;
    for (i, pp) in w.people.iter().enumerate() {
        if !matches!(pp.stage, Stage::HighSchool | Stage::PrepSchool) {
            continue;
        }
        let commit = pp.cal.commit.filter(|c| c.0 == p).is_some() && pp.cal.signed.is_none();
        let target = !commit && pp.cal.offers.first() == Some(&p) && pp.cal.signed.is_none() && pp.class_of == w.year + 1;
        if !commit && !target {
            continue;
        }
        let loyal = pp.player.personality.traits.get(Trait::Loyalty);
        let recruited_by_old = old.is_some();
        let shaky = commit && (loyal < 11.0 || pp.stars >= 4) && recruited_by_old;
        if shaky {
            wobbling += 1;
        }
        doc.rows.push(DocRow {
            pid: Some(pp.player.id),
            cells: vec![
                player_link(w, i),
                pp.player.position.name().into(),
                "★".repeat(pp.stars as usize),
                if commit { if shaky { "committed, wavering".into() } else { "committed".into() } } else { "top target".into() },
                if shaky {
                    "the staff that recruited him is gone; other schools are calling his parents".into()
                } else if commit {
                    "solid for now".into()
                } else {
                    "waiting to hear from the new staff".into()
                },
            ],
        });
    }
    doc.rows.sort_by(|a, b| b.cells[2].len().cmp(&a.cells[2].len()));
    doc.rows.truncate(20);
    s.docs.push(doc);
    s.narrate(if wobbling > 0 {
        format!("{wobbling} commits are wavering. In real coaching changes, commits who hear from the new head coach in the first week mostly stay.")
    } else {
        "Nobody's wavering yet. The calls still matter: a commit who doesn't hear from you starts listening to others.".to_string()
    });
    s.pick(90, "Call every commit yourself tonight, the wavering ones first.", "Praise", "warm", "Most of them stay solid; it costs your evening.");
    s.pick(91, "Have the recruiting coordinator call; you'll call the top three.", "Hand him something", "calm", "The top names hear from you; the rest from staff.");
}

fn recruiting_answer(w: &mut World, s: &mut Scene, team: Team, code: u32) {
    let n = recruiting_calls(w, team, code == 90);
    w.folk.hours += if code == 90 { 4.0 } else { 1.5 };
    s.outcome.push(format!("{n} commits heard from {}.", if code == 90 { "you" } else { "your staff, the top three from you" }));
    s.end();
}

/// The first calls to commits: each one's hold on his commitment, by who
/// called and how good a recruiter you are. Returns how many were called.
fn recruiting_calls(w: &mut World, team: Team, all_you: bool) -> usize {
    let Team::College(p) = team else { return 0 };
    let rec = w.career.as_ref().map_or(10.0, |c| c.you.recruiting);
    let today = w.today;
    let mut n = 0;
    let mut list: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            let pp = &w.people[i];
            matches!(pp.stage, Stage::HighSchool | Stage::PrepSchool) && pp.cal.commit.is_some_and(|c| c.0 == p) && pp.cal.signed.is_none()
        })
        .collect();
    list.sort_by(|&a, &b| w.people[b].stars.cmp(&w.people[a].stars));
    for (k, i) in list.into_iter().enumerate() {
        let you_called = all_you || k < 3;
        let pp = &mut w.people[i];
        pp.trust_in = w.career.as_ref().map_or(0, |c| c.coach) as u32;
        pp.trust = (0.45 + if you_called { 0.08 + 0.006 * (rec - 10.0) as f32 } else { 0.03 }).clamp(0.0, 1.0);
        let _ = today;
        n += 1;
    }
    w.folk.commit_calls = Some((p, today, all_you));
    n
}

// ---------------------------------------------------------------------------
// Items 13-15: the boosters (the owner's family in the NFL).

fn booster_of(w: &World, s: &Scene, team: Team) -> Option<Cast> {
    let role = match (team, s.a) {
        (Team::Nfl(_), _) => Role::OwnerKin,
        (Team::High(_), 100) => Role::ClubPresident,
        _ => Role::Booster,
    };
    let slot = if s.a == 100 { 0 } else { s.a as u8 };
    cast::has(w, team, role).then(|| cast::get(w, CastRef::new(team, role, slot)))
}

fn booster_open(w: &mut World, s: &mut Scene, team: Team) {
    let Some(c) = booster_of(w, s, team) else {
        s.narrate("He cancelled. His assistant says he's traveling.");
        s.end();
        return;
    };
    let Some(b) = c.booster.clone() else {
        s.end();
        return;
    };
    s.title = format!("Meeting {}", c.name);
    s.place = match team {
        Team::Nfl(_) => "The owner's box".into(),
        Team::High(_) => {
            if c.r.role == Role::ClubPresident { "The concession stand office".into() } else { format!("His business, {}", b.business) }
        }
        _ => format!("{}'s office downtown", c.last()),
    };
    s.who.push(Who::Cast(c.r.token()));
    let old = w.folk.takeover.as_ref().and_then(|t| t.old_coach);
    let intro = match team {
        Team::Nfl(_) => format!("{}, the {}. {}", c.link(), c.title.to_lowercase(), "In the NFL there are no boosters: the owner's family and partners have his ear."),
        _ => {
            let given = if b.yearly > 0.0 { format!(" and gives about {} a year", money(b.yearly)) } else { String::new() };
            format!(
                "{} made his money in {}. He gave {}{given}. {} is close to {}.",
                c.link(),
                b.business,
                b.gift,
                cap(c.he()),
                b.close_to
            )
        }
    };
    s.narrate(intro);
    if b.loyal_old > 0.6
        && let Some(o) = old
    {
        s.narrate(format!("He was one of {}'s people. He'll judge you against him.", coach_link(w, o)));
    }
    let mut pen = scene::pen(w, s, 1);
    let spk = Speaker::cast(w, &c);
    let mut f = base_facts(w, team);
    f.set("name", c.first().to_string());
    let greet = pen.line(&["booster.greet", "?booster.small"], &spk, &f);
    if !greet.is_empty() {
        s.them(&c.name, greet, Some(booster_tell(&c)));
    }
    // His wants, one by one (item 14).
    let want = b.wants.first().copied();
    if let Some(wt) = want {
        f.set("want", wt.words());
        if let Some(h) = b.old_school {
            f.set("old_school", w.school_name(h).to_string());
        }
        let line = pen.line(&[want_key(wt)], &spk, &f);
        s.them(&c.name, if line.is_empty() { want_fallback(w, wt, &b) } else { line }, None);
        s.c = 0;
        super::note(
            w,
            "boosters",
            format!("{}, booster", c.link()),
            format!("Wants {}.", wt.words()),
            Some(format!("Gives {}; close to {}.", if b.yearly > 0.0 { money(b.yearly) + " a year" } else { "his time".into() }, b.close_to)),
        );
    }
    pen.put(w);
    if let Some(wt) = want {
        add_want_picks(s, wt);
    } else {
        s.pick(103, "Thank him for everything he does for the program.", "Praise", "warm", "Goodwill.");
    }
}

fn booster_tell(c: &Cast) -> String {
    if c.meddling >= 14.0 {
        "He has a list in his jacket pocket. He doesn't take it out, but you can see it.".into()
    } else if c.warmth >= 13.0 {
        "He laughs easily and asks about your family before football.".into()
    } else if c.ego >= 14.0 {
        "His name is on two things you passed on the way in. He mentions both.".into()
    } else {
        "He listens more than he talks.".into()
    }
}

fn want_key(wt: Want) -> &'static str {
    match wt {
        Want::BeatRival => "booster.want.rival",
        Want::Access => "booster.want.access",
        Want::SayInHires => "booster.want.hires",
        Want::SayOnPlayer => "booster.want.player",
        Want::NilDeals => "booster.want.nil",
        Want::Clean => "booster.want.clean",
        Want::Building => "booster.want.building",
        Want::OldSchool => "booster.want.oldschool",
        Want::WinNow => "booster.want.winnow",
    }
}

fn want_fallback(w: &World, wt: Want, b: &cast::BoosterInfo) -> String {
    match wt {
        Want::BeatRival => "I don't care what else happens. Beat them. Every year.".into(),
        Want::Access => "I've stood on that sideline for twenty years. I'd like to keep doing it.".into(),
        Want::SayInHires => "When you hire your coordinators, I'd like to meet them first.".into(),
        Want::SayOnPlayer => "There's a local kid I've watched since he was nine. He deserves a real look.".into(),
        Want::NilDeals => "My company wants players in its ads. That's real money, and it stays in the program.".into(),
        Want::Clean => "Win, sure. But I won't read about this program in the police blotter.".into(),
        Want::Building => "The next building has a name on it. I'd like it to be mine.".into(),
        Want::OldSchool => format!(
            "I went to {}. Their best kids deserve to hear from you before anybody else.",
            b.old_school.map_or("the high school down the road".to_string(), |h| w.school_name(h).to_string())
        ),
        Want::WinNow => "I'm not getting younger. I want to see us win while I can still enjoy it.".into(),
    }
}

fn add_want_picks(s: &mut Scene, wt: Want) {
    match wt {
        Want::Access => {
            s.pick(100, "You'll have your sideline pass, as long as I'm here.", "Promise", "warm", "He's happy; a promise logged. Keep it.");
            s.pick(101, "The sideline is for the team. I'll find you a better seat.", "Tell him straight", "firm", "He respects it or he doesn't.");
        }
        Want::SayInHires | Want::SayOnPlayer => {
            s.pick(100, "I'll keep you in the loop. You'll hear it from me first.", "Promise", "warm", "He feels heard; a promise you'll be asked to keep.");
            s.pick(101, "Football decisions stay with me. That's not personal.", "Tell him straight", "firm", "Clean lines; a meddler takes it badly.");
        }
        Want::NilDeals => {
            s.pick(100, "Let's set it up through the collective, properly.", "Promise", "warm", "NIL money for your players through his company.");
            s.pick(101, "I'll send you the players who fit. Their call, not mine.", "Listen", "calm", "He'll take it, maybe.");
        }
        Want::Clean => {
            s.pick(100, "Nobody here embarrasses this school. You have my word.", "Promise", "firm", "He's with you. Every arrest is now a broken promise.");
            s.pick(101, "Kids make mistakes. I'll handle them right, not hide them.", "Tell him straight", "calm", "An honest answer; he decides what he thinks of it.");
        }
        Want::Building => {
            s.pick(100, "When the new building comes, I'll tell the AD where my vote is.", "Promise", "warm", "He opens his checkbook a little wider.");
            s.pick(101, "Buildings are the AD's call. I'll be glad to show you the plans.", "Listen", "calm", "Polite; no promise.");
        }
        _ => {
            s.pick(100, "That's the plan. You'll see it this fall.", "Promise", "fiery", "He leaves fired up, and holds you to it.");
            s.pick(101, "It's a process. Give us two years and judge us then.", "Tell him straight", "calm", "Some boosters respect patience; others hear excuses.");
        }
    }
    s.pick(102, "Change the subject: ask about his business.", "Ask", "warm", "No promise either way; he likes being asked.");
}

fn booster_answer(w: &mut World, s: &mut Scene, team: Team, code: u32) {
    let Some(c) = booster_of(w, s, team) else {
        s.end();
        return;
    };
    let b = c.booster.clone().unwrap_or_default();
    let want = b.wants.first().copied();
    let meddler = Cast::z(c.meddling) as f32;
    let (dt, line, outcome) = match code {
        100 => {
            if let Some(wt) = want {
                super::talks::promise(w, super::talks::PromiseTo::Cast(c.r, c.term), super::talks::PromiseKind::Booster(wt), 2);
            }
            (0.15, "That's what I wanted to hear. You and I are going to get along.", format!("You promised {} {}.", c.name, want.map_or("your attention", |x| x.short())))
        }
        101 => (-0.05 - 0.1 * meddler.max(0.0), "Fair enough. We'll see how it goes.", format!("You told {} no, politely.", c.name)),
        102 => (0.04, "Thirty years of it. Let me tell you how I started.", format!("You let {} talk about his business.", c.name)),
        _ => (0.06, "Glad to do it. Glad you're here.", format!("You thanked {}.", c.name)),
    };
    w.folk.mine.nudge(c.key(), dt);
    s.them(&c.name, line, None);
    if code == 100
        && want == Some(Want::NilDeals)
        && let Team::College(p) = team
    {
        w.programs[p].collective.budget += b.yearly * 0.15;
    }
    s.outcome.push(outcome);
    s.end();
}

// ---------------------------------------------------------------------------
// Item 19: the beat reporters.

fn media_open(w: &mut World, s: &mut Scene, team: Team) {
    let comms = if cast::has(w, team, Role::Comms) { Some(cast::get(w, CastRef::new(team, Role::Comms, 0))) } else { None };
    let reporters = cast::all(w, team, Role::Reporter);
    match &comms {
        Some(c) => {
            s.who.push(Who::Cast(c.r.token()));
            s.narrate(format!("{}, the {}, walks you through who covers the team.", c.link(), c.title.to_lowercase()));
        }
        None => s.narrate("The athletic director tells you who covers the team.".to_string()),
    }
    let mut doc = Doc {
        title: "Who covers you".into(),
        note: "The reporters on your team, with what your communications people say about each.".into(),
        cols: vec!["Reporter".into(), "Outlet".into(), "Beat".into(), "Leaning".into(), "Sources".into(), "The word".into()],
        rows: Vec::new(),
    };
    for r in &reporters {
        let Some(info) = &r.reporter else { continue };
        let lean = match info.lean {
            l if l > 0.5 => "friendly",
            l if l > 0.1 => "fair",
            l if l > -0.3 => "tough",
            _ => "a critic",
        };
        let word = if info.leaks {
            "Gets things from inside the building. Assume what you tell your staff reaches him."
        } else if info.accuracy > 0.85 {
            "Careful: when he reports it, it's true."
        } else if info.accuracy < 0.55 {
            "Runs with rumors. Half of it holds up."
        } else if info.national {
            "National: he calls when it's big."
        } else {
            "Straight shooter. Treat him fairly and he will too."
        };
        doc.rows.push(DocRow {
            pid: None,
            cells: vec![r.link(), info.outlet.clone(), r.title.split(',').next().unwrap_or("").to_string(), lean.into(), info.sources.join(", "), word.into()],
        });
        super::note(w, "media", r.link(), format!("{}: {}", info.outlet, word), None);
    }
    s.docs.push(doc);
    s.pick(110, "Open door: practices open for twenty minutes, I return calls.", "Praise", "warm", "Reporters treat you softer; more gets out.");
    s.pick(111, "Closed practices, injuries on the report only, nothing off the record.", "Tell him straight", "firm", "Less leaks; the coverage is harder on you.");
    s.pick(112, "Fair to all, a little more for the beat writer who's straight with us.", "Hand him something", "calm", "The beat writer gets the scoops; the rest notice.");
}

fn media_answer(w: &mut World, s: &mut Scene, team: Team, code: u32) {
    w.folk.media_policy = code as u8 - 110;
    let reporters = cast::all(w, team, Role::Reporter);
    for r in &reporters {
        let d = match code {
            110 => 0.1,
            111 => -0.08,
            _ => {
                if r.r.slot == 0 {
                    0.12
                } else {
                    -0.02
                }
            }
        };
        w.folk.mine.nudge(r.key(), d);
    }
    s.outcome.push(
        match code {
            110 => "An open door with the press.",
            111 => "A closed shop with the press.",
            _ => "Fair to all, with the beat writer first.",
        }
        .into(),
    );
    s.end();
}

// ---------------------------------------------------------------------------
// Item 20: the program tour.

fn tour(w: &mut World, s: &mut Scene, team: Team) {
    let host = match team {
        Team::College(_) => cast::get(w, CastRef::new(team, Role::AthleticDirector, 0)),
        Team::Nfl(_) => cast::get(w, CastRef::new(team, Role::TeamPresident, 0)),
        Team::High(_) => cast::get(w, CastRef::new(team, Role::SchoolAd, 0)),
    };
    s.who.push(Who::Cast(host.r.token()));
    s.narrate(format!("{} gives you the tour, and the history with it.", host.link()));
    let seasons = facts::seasons(w, team, 5);
    let mut doc = Doc {
        title: "The last five seasons".into(),
        note: String::new(),
        cols: vec!["Season".into(), "Record".into(), "Head coach".into(), "".into()],
        rows: Vec::new(),
    };
    for se in &seasons {
        doc.rows.push(DocRow {
            pid: None,
            cells: vec![se.year.to_string(), format!("{}-{}", se.wins, se.losses), se.coach.map_or(String::new(), |c| coach_link(w, c)), se.note.clone()],
        });
    }
    s.docs.push(doc);
    let you = w.career.as_ref().map_or(0, |c| c.coach);
    if let Some((o, why, from, to)) = facts::predecessor(w, team, you) {
        let (wins, losses) = w.coaches[o].history.iter().rev().find(|x| x.employer == team.employer()).map_or((0, 0), |x| (x.wins, x.losses));
        s.narrate(format!(
            "Your predecessor, {}, coached here from {from} to {to} and went {wins}-{losses}. {}.",
            coach_link(w, o),
            cap(facts::why_left(&why))
        ));
    }
    if let Some((r, game, a, b)) = facts::rival(w, team) {
        if a + b > 0 {
            s.narrate(format!("The rival is {}: {game}. {} leads the series {}-{}.", r.link(w), if a >= b { team.name(w) } else { r.name(w) }, a.max(b), a.min(b)));
        } else {
            s.narrate(format!("The game people here care about most is against {}.", r.link(w)));
        }
    }
    s.narrate(facts::facilities_words(w, team));
    let mut money = Doc { title: "The money".into(), note: String::new(), cols: vec!["".into(), "".into()], rows: Vec::new() };
    for (k, v) in facts::money_words(w, team) {
        money.rows.push(DocRow { pid: None, cells: vec![k, v] });
    }
    s.docs.push(money);
    let fixtures = match team {
        Team::College(p) => w
            .college_fixtures(Some(p))
            .into_iter()
            .take(12)
            .map(|f| {
                let opp = if f.home == p { f.away } else { f.home };
                (f.date, format!("{} {}", if f.home == p { "vs" } else { "at" }, crate::world::folk::Team::College(opp).link(w)))
            })
            .collect::<Vec<_>>(),
        Team::Nfl(t) => w
            .nfl_fixtures(Some(t))
            .into_iter()
            .take(17)
            .filter_map(|f| {
                let opp = if f.home == t { f.away } else { f.home };
                Some((f.when?, format!("{} {}", if f.home == t { "vs" } else { "at" }, Team::Nfl(opp).link(w))))
            })
            .collect(),
        Team::High(h) => w
            .hs_fixtures(h)
            .into_iter()
            .take(10)
            .map(|f| {
                let opp = if f.home == h { f.away } else { f.home };
                (f.date, format!("{} {}", if f.home == h { "vs" } else { "at" }, Team::High(opp).link(w)))
            })
            .collect(),
    };
    if !fixtures.is_empty() {
        s.docs.push(Doc {
            title: "The schedule already set".into(),
            note: "Signed years ago; yours to play.".into(),
            cols: vec!["Date".into(), "Game".into()],
            rows: fixtures.into_iter().map(|(d, t)| DocRow { pid: None, cells: vec![d.pretty(), t] }).collect(),
        });
    }
    s.pick(120, "Thank him for the tour.", "Praise", "warm", "Filed in the takeover file.");
}

// ---------------------------------------------------------------------------
// Item 21: the first weeks' plan.

fn plan(w: &mut World, s: &mut Scene, team: Team) {
    let host = match team {
        Team::College(_) if cast::has(w, team, Role::Ops) => Some(cast::get(w, CastRef::new(team, Role::Ops, 0))),
        Team::Nfl(_) => Some(cast::get(w, CastRef::new(team, Role::CapManager, 0))),
        Team::High(_) => Some(cast::get(w, CastRef::new(team, Role::SchoolAd, 0))),
        _ => None,
    };
    if let Some(h) = &host {
        s.who.push(Who::Cast(h.r.token()));
        s.narrate(format!("{} has the calendar up on the wall.", h.link()));
    }
    let due = facts::due(w, team);
    s.docs.push(Doc {
        title: "What's due".into(),
        note: "From now to fall camp.".into(),
        cols: vec!["Date".into(), "What".into()],
        rows: due.iter().map(|(d, t)| DocRow { pid: None, cells: vec![d.pretty(), t.clone()] }).collect(),
    });
    for (d, t) in due.iter().take(6) {
        super::note(w, "plan", host.as_ref().map_or("Your staff".into(), |h| h.link()), format!("{}: {t}", d.pretty()), None);
    }
    s.pick(130, "Put it on my calendar.", "Listen", "calm", "Your calendar has the dates.");
}

// ---------------------------------------------------------------------------
// Items 22 and 23: high school only.

fn parents_open(w: &mut World, s: &mut Scene, team: Team) {
    let roster = team.roster(w);
    let loud: Vec<usize> = roster.iter().copied().filter(|&i| w.people[i].family.parent_style == crate::world::background::ParentStyle::LoudAtGames).collect();
    s.who.push(Who::Group("The parents".into()));
    s.narrate(format!(
        "About {} parents in the cafeteria, more than the team has players on some nights. The booster club set out cookies.",
        (roster.len() as f64 * 1.1).round()
    ));
    let mut pen = scene::pen(w, s, 1);
    let f = base_facts(w, team);
    if let Some(&i) = loud.first() {
        let spk = Speaker::parent(w, i);
        let line = pen.line(&["parents.playing_time"], &spk, &f);
        let kid = w.people[i].name.split_whitespace().next().unwrap_or("").to_string();
        s.them(
            &format!("{}'s father", kid),
            if line.is_empty() { format!("My son started two years for the last coach. Is that going to change?") } else { line },
            Some("A few parents nod. They've been talking about this in the parking lot.".into()),
        );
    }
    pen.put(w);
    s.pick(140, "Every position is open. The best player plays, and I'll tell your son why.", "Tell him straight", "firm", "Fair-minded parents like it; the ones with starters worry.");
    s.pick(141, "Your sons are in good hands. My door is open, any time.", "Praise", "warm", "Parents warm to you; they'll use that door.");
    s.pick(142, "Playing time is between me and the player. Not the parents.", "Warn", "firm", "A line drawn; the loud ones go to the principal.");
}

fn parents_answer(w: &mut World, s: &mut Scene, team: Team, code: u32) {
    let roster = team.roster(w);
    let principal = cast::get(w, CastRef::new(team, Role::Principal, 0));
    let mut complaints = 0;
    for i in roster {
        let style = w.people[i].family.parent_style;
        let loud = style == crate::world::background::ParentStyle::LoudAtGames || style == crate::world::background::ParentStyle::SocialMedia;
        let d: f32 = match code {
            140 => if loud { -0.01 } else { 0.03 },
            141 => 0.03,
            _ => if loud { -0.04 } else { 0.0 },
        };
        let p = &mut w.people[i];
        p.trust = (p.trust + d).clamp(0.0, 1.0);
        if code == 142 && loud {
            complaints += 1;
        }
    }
    if complaints > 0 {
        w.folk.mine.nudge(principal.key(), -0.03 * (complaints as f32).min(4.0));
    }
    s.outcome.push(match code {
        140 => "You told the parents the best player plays.".to_string(),
        141 => "You opened your door to the parents.".to_string(),
        _ => format!("You drew a line with the parents. {complaints} of them called the principal by Friday."),
    });
    s.end();
}

fn feeder(w: &mut World, s: &mut Scene, team: Team) {
    let feeders = cast::all(w, team, Role::Feeder);
    let youth = cast::get(w, CastRef::new(team, Role::Youth, 0));
    s.who.push(Who::Group("The feeder coaches".into()));
    s.narrate(format!(
        "You drive the feeder schools with {}. {}",
        feeders.first().map_or("the athletic director".into(), |c| c.link()),
        "Middle school coaches know who's coming before anyone else does."
    ));
    // Who's coming up: the best eighth graders, by what the feeder coaches see.
    let Team::High(h) = team else {
        s.end();
        return;
    };
    let class = w.year + 4;
    let mut kids: Vec<usize> = w.high_schools[h].roster.clone();
    kids.retain(|&i| w.people[i].class_of >= class);
    let mut doc = Doc {
        title: "Who's coming up".into(),
        note: "What the middle school and youth league coaches told you.".into(),
        cols: vec!["Kid".into(), "Plays".into(), "What they say".into()],
        rows: Vec::new(),
    };
    for c in &feeders {
        let f = format!("{} of {}", c.title, c.r.slot + 1);
        let _ = f;
    }
    for &i in kids.iter().take(6) {
        let p = &w.people[i];
        let say = match p.player.body.height {
            ht if ht > 74.0 => "big already, and still growing",
            _ if p.player.personality.competitiveness >= 14.0 => "the toughest kid in the league",
            _ => "a good athlete who loves it",
        };
        doc.rows.push(DocRow { pid: Some(p.player.id), cells: vec![player_link(w, i), p.player.position.name().into(), say.into()] });
    }
    if doc.rows.is_empty() {
        s.narrate(format!("{} runs the youth league; he says next year's group is thin, and the one after it better.", youth.link()));
    } else {
        s.docs.push(doc);
    }
    for c in &feeders {
        w.folk.mine.nudge(c.key(), 0.05);
    }
    w.folk.mine.nudge(youth.key(), 0.05);
    s.pick(150, "Offer to run a clinic for the youth coaches this summer.", "Hand him something", "warm", "Ties with the feeder coaches; a day of your summer.");
    s.pick(151, "Thank them and ask for their film.", "Ask", "calm", "Filed.");
}

fn old_coach_open(w: &mut World, s: &mut Scene, team: Team) {
    let Some(o) = w.folk.takeover.as_ref().and_then(|t| t.old_coach) else {
        s.narrate("The old coach moved away. Nobody's teaching down the hall.");
        s.end();
        return;
    };
    s.who.push(Who::Coach(o));
    let co = &w.coaches[o];
    s.narrate(format!(
        "{} still teaches here. His coaching stipend wasn't renewed; his classroom is twenty steps from your office. Half the town still calls him Coach.",
        coach_link(w, o)
    ));
    let mut pen = scene::pen(w, s, 1);
    let spk = Speaker::coach(w, o);
    let f = base_facts(w, team);
    let line = pen.line(&["takeover.oldcoach.greet"], &spk, &f);
    pen.put(w);
    let name = co.name.clone();
    s.them(&name, if line.is_empty() { "Come on in. I figured you'd stop by eventually.".to_string() } else { line }, Some(if co.personality.temper >= 13.0 { "He doesn't get up from his desk.".into() } else { "He shakes your hand like he means it.".into() }));
    s.pick(160, "Ask him to help with the program, any way he wants.", "Hand him something", "warm", "His people warm to you; he's around the program.");
    s.pick(161, "Tell him you respect what he built and leave it there.", "Praise", "calm", "Polite distance.");
    s.pick(162, "Make clear it's your program now.", "Tell him straight", "firm", "A line drawn; his loyalists hear about it by dinner.");
}

fn old_coach_answer(w: &mut World, s: &mut Scene, team: Team, code: u32) {
    let Some(o) = w.folk.takeover.as_ref().and_then(|t| t.old_coach) else {
        s.end();
        return;
    };
    let d = match code {
        160 => 0.2,
        161 => 0.05,
        _ => -0.2,
    };
    w.folk.mine.nudge(super::coach_key(o), d);
    // His loyalists follow his lead (item 15).
    for role in [Role::Booster, Role::ClubPresident] {
        for c in cast::all(w, team, role) {
            if c.booster.as_ref().is_some_and(|b| b.loyal_old > 0.5) {
                w.folk.mine.nudge(c.key(), d * 0.5);
            }
        }
    }
    let name = w.coaches[o].name.clone();
    s.them(&name, match code {
        160 => "I'd like that. These kids matter to me.",
        161 => "Appreciate it. Good luck, Coach.",
        _ => "Understood. It's yours.",
    }, None);
    s.outcome.push(match code {
        160 => format!("You asked {name} to stay around the program."),
        161 => format!("You paid your respects to {name}."),
        _ => format!("You told {name} it's your program now."),
    });
    s.end();
}

// ---------------------------------------------------------------------------
// The NFL: the general manager.

fn gm_open(w: &mut World, s: &mut Scene, team: Team) {
    let gm = cast::get(w, CastRef::new(team, Role::Gm, 0));
    s.who.push(Who::Cast(gm.r.token()));
    let Team::Nfl(t) = team else {
        s.end();
        return;
    };
    let power = w.clubs.get(t).map(|c| c.office.power);
    let mode = w.clubs.get(t).map(|c| c.office.mode);
    s.narrate(format!(
        "{} has been the general manager since {}. {}",
        gm.link(),
        gm.since,
        match power {
            Some(crate::world::pro::office::Power::Coach) => "Here the head coach has the final say on the roster, and he knows it came with your hire.",
            Some(crate::world::pro::office::Power::Owner) => "The owner has the final say here, and both of you know it.",
            _ => "He has the final say on the roster; your say is your voice in the room.",
        }
    ));
    s.narrate(format!(
        "His read on the year: {}.",
        match mode {
            Some(crate::world::pro::office::Mode::Contend) => "the window is open; the owner wants to win now",
            Some(crate::world::pro::office::Mode::Rebuild) => "a rebuild: draft picks, young players and cap room",
            _ => "somewhere in between: compete and get younger",
        }
    ));
    s.pick(170, "Tell him you want a say on every 53-man decision.", "Challenge", "firm", "He gives a little and remembers it.");
    s.pick(171, "Tell him you'll coach who he gives you, and tell him what you need.", "Praise", "warm", "He trusts you; you live with his roster.");
    s.pick(172, "Ask him which three players he'd never trade.", "Ask", "calm", "You learn who he believes in.");
}

fn gm_answer(w: &mut World, s: &mut Scene, team: Team, code: u32) {
    let gm = cast::get(w, CastRef::new(team, Role::Gm, 0));
    let d = match code {
        170 => -0.08 + 0.1 * (1.0 - Cast::z(gm.ego).max(0.0) as f32),
        171 => 0.12,
        _ => 0.05,
    };
    w.folk.mine.nudge(gm.key(), d);
    s.them(&gm.name, match code {
        170 => "You'll be in the room. The decisions are mine; that's how the owner set it up.",
        171 => "That's how this works best. Tell me what you need and I'll go get it.",
        _ => "Ask me again after the draft. But you can guess the first one.",
    }, None);
    s.outcome.push("You sat down with your general manager.".into());
    s.end();
}

// ---------------------------------------------------------------------------
// The takeover file and its check (items 2 and 3).

/// After spring ball, the takeover file's claims are checked against what
/// your own staff has now seen.
pub(crate) fn check_claims(w: &mut World) {
    let Some(t) = w.folk.takeover.as_ref() else { return };
    if t.checked {
        return;
    }
    let claims = t.claims.clone();
    for (who, id, sign) in claims {
        let Some(i) = super::person(w, id) else { continue };
        let p = &w.people[i];
        let held = matches!(p.last_role, Some(PlayingTime::Starter)) || p.cal.spring_notice;
        let text = if held == (sign > 0) { "It held up: he played like it this spring." } else { "It didn't hold up: he still looks like a backup." };
        let name = player_link(w, i);
        if let Some(e) = w.folk.log.iter_mut().find(|e| e.source.contains(&who) && e.text.contains(&name)) {
            e.check = Some(text.to_string());
        }
    }
    if let Some(t) = w.folk.takeover.as_mut() {
        t.checked = true;
    }
}

fn money(v: f64) -> String {
    if v >= 1e6 { format!("${:.1} million", v / 1e6) } else { format!("${}k", (v / 1e3).round()) }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// The takeover's day: the spring check.
pub(crate) fn day(w: &mut World, d: Date) {
    window_day(w, d);
    if w.folk.takeover.as_ref().is_some_and(|t| !t.checked) && d == w.dates.spring_game.add(2) {
        check_claims(w);
    }
    let _ = Level::College;
    let _ = Pen::loose;
}
