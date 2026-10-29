//! Your read of your own staff (update 2, Q1-Q4, RR2): every coach and
//! support person on your staff, by department, with each rating his job
//! uses as a range on the hidden 1-20 scale. The range is your read, never
//! the number: it starts wide (what his name, an interview or a friend
//! told you) and narrows with each season he works for you and with the
//! evidence his job leaves (his room's growth against players like them,
//! the games he called, the room's mood, injuries, the classes signed).
//! Other teams' staffs stay reputation only.
//!
//! Each reader leans his own fixed way (`lean`), so a read holds from day
//! to day and its centre settles toward the truth as it narrows. The
//! season book (`Book`) keeps what each season showed: his room's growth,
//! player by player, and the ratings your read was centred on then.
//!
//! The ratings are the engine's own: a coach's ability parts, the calling
//! the match engine uses (scheme and game management), a support head's
//! one rating split into the facets of his job around it (they average to
//! it). People the world doesn't keep (an NFL position room, analysts,
//! graduate assistants, a dietitian, a team physician) are drawn from their
//! job and team the same way every time.
//!
//! Sizes are Claude's picks (proposed, not confirmed): a read starts about
//! six points wide and narrows to about three after three seasons; a
//! season is worth a unit of evidence for a rating the job shows every day
//! (teaching his room, calling the plays) and less for one it shows now
//! and then; a support head's facets sit 1.6 points either side of his
//! rating.

use super::coach::{Employer, GROUP_COUNT, Group, Role};
use super::folk::{self, Team, staffing};
use super::people::Staffer;
use super::{Stage, World};
use crate::calendar::Date;
use crate::player::PlayingTime;
use crate::rng::clamp;

/// A department on the staff page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dept {
    Coaches,
    Performance,
    Medical,
    Ops,
}

impl Dept {
    pub const ALL: [Dept; 4] = [Dept::Coaches, Dept::Performance, Dept::Medical, Dept::Ops];
    pub fn name(self) -> &'static str {
        ["Coaches", "Performance", "Medical", "Ops and support"][self as usize]
    }
}

/// The job a member holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Job {
    /// A coach the world keeps, in his role.
    Coach(Role),
    /// Your high school assistant.
    HsCoach,
    /// A drawn NFL room: its slot in the club's rooms.
    Room(u8),
    Analyst,
    GradAssistant,
    StrengthHead,
    StrengthAssistant,
    Dietitian,
    Trainer,
    Physician,
    AssistantTrainer,
    AnalyticsHead,
    Personnel,
    RecruitingDirector,
    Academic,
    Engagement,
}

/// One rating a job uses.
#[derive(Clone, Debug)]
pub struct Facet {
    pub key: String,
    pub name: String,
    /// Its group on the coach page: Teaching, Coaching, Recruiting, Game
    /// day, or "The job" for support staff.
    pub group: &'static str,
    /// The true rating; None when the job never shows it.
    truth: Option<f64>,
    /// Why it isn't seen ("Doesn't run the game").
    pub why_not: String,
    /// Units of evidence a season with you gives.
    rate: f64,
}

impl Facet {
    pub fn seen(&self) -> bool {
        self.truth.is_some()
    }
}

/// Someone on your staff.
#[derive(Clone, Debug)]
pub struct Member {
    /// Who he is, for the season book and the screens.
    pub key: u64,
    pub job: Job,
    pub dept: Dept,
    pub name: String,
    pub title: String,
    /// A coach the world keeps: his page.
    pub coach: Option<usize>,
    /// Your high school assistant: his place in your staff.
    pub hs: Option<usize>,
    /// The position group he teaches.
    pub group: Option<Group>,
    pub age: Option<u32>,
    /// His first season in this job, and the day your read of him starts
    /// (his first day, or yours when he was here before you).
    pub since: i32,
    pub start: Date,
    pub kept: bool,
    /// Hired and paid by the athletic department (college medical and
    /// academic staff), or the club's medical staff.
    pub not_yours: Option<&'static str>,
    pub pay: Option<f64>,
    pub through: Option<i32>,
    /// What you knew of him before his first day (an interview, a friend).
    known: f64,
    pub facets: Vec<Facet>,
    /// The two reads the staff page shows first.
    pub keys: Vec<usize>,
}

/// What each season showed (`Career::staff_book`).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Book {
    pub rows: Vec<Row>,
    pub snaps: Vec<Snap>,
}

/// A player's season under your staff: his growth against players like
/// him, who taught his room and who ran his side.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Row {
    pub year: i32,
    pub player: u32,
    pub group: u8,
    pub residual: f32,
    pub coach: u64,
    pub coord: u64,
    /// His role that season (`PlayingTime` in order) and his rating going
    /// in and at the end.
    pub role: u8,
    pub start: f32,
    pub end: f32,
}

/// A coach's rating at a season's end: what that season's read was
/// centred on.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Snap {
    pub year: i32,
    pub who: u64,
    pub facet: u8,
    pub truth: f32,
}

/// Seasons the book keeps.
const KEEP_SEASONS: i32 = 15;

fn facet(key: &str, name: String, group: &'static str, truth: Option<f64>, why_not: &str, rate: f64) -> Facet {
    Facet {
        key: key.to_string(),
        name,
        group,
        truth: truth.map(|t| clamp(t, 1.0, 20.0)),
        why_not: why_not.to_string(),
        rate,
    }
}

fn n(a: u64, b: u64) -> f64 {
    super::season::key_normal(a ^ 0x57AF_F2EA_D000_0000, b)
}

/// The key of a coach the world keeps, and of a high school assistant.
pub fn coach_key(c: usize) -> u64 {
    0x1000_0000_0000_0000 | c as u64
}

fn hs_key(name: &str, since: i32) -> u64 {
    0x2000_0000_0000_0000 | (super::seat::assistant_born_key(name, since) & 0x0FFF_FFFF_FFFF_FFFF)
}

fn head_key(team: Team, post: u64, s: &Staffer) -> u64 {
    let h = s
        .name
        .bytes()
        .fold(team.key() ^ post << 40 ^ s.since as u64, |k, b| k.wrapping_mul(0x100_0000_01B3) ^ b as u64);
    0x3000_0000_0000_0000 | (h & 0x0FFF_FFFF_FFFF_FFFF)
}

fn drawn_key(who: u64) -> u64 {
    0x4000_0000_0000_0000 | (who & 0x0FFF_FFFF_FFFF_FFFF)
}

/// Your coach, and the job you hold.
fn you(w: &World) -> Option<usize> {
    w.career.as_ref().map(|k| k.coach)
}

/// The day you took the job you hold now.
pub fn your_start(w: &World) -> Date {
    let since = you(w).and_then(|c| w.coaches[c].job).map_or(w.year, |j| j.since);
    match &w.career {
        Some(k) if k.started.year() >= since => k.started,
        _ => Date::ymd(since, 1, 15),
    }
}

fn start_of(w: &World, since: i32) -> (Date, bool) {
    start_at(w, Date::ymd(since, 2, 1))
}

/// When your read of him starts: his first day, or yours if he was already
/// there when you came (kept on).
fn start_at(w: &World, his: Date) -> (Date, bool) {
    let yours = your_start(w);
    if his > yours { (his, false) } else { (yours, true) }
}

/// Which side's plays a coach calls for your team.
fn calls(w: &World, team: Team, c: usize) -> Option<bool> {
    let (oc, dc, hc) = match team {
        Team::College(p) => (w.programs[p].oc, w.programs[p].dc, w.programs[p].head_coach),
        Team::Nfl(t) => (w.nfl[t].oc, w.nfl[t].dc, w.nfl[t].head_coach),
        _ => return None,
    };
    if super::season::play_caller(w, oc, hc, true) == Some(c) {
        Some(true)
    } else if super::season::play_caller(w, dc, hc, false) == Some(c) {
        Some(false)
    } else {
        None
    }
}

/// A coach's rating by facet key (the read's truth).
fn coach_truth(w: &World, c: usize, key: &str) -> Option<f64> {
    let co = w.coaches.get(c)?;
    let a = co.ability();
    Some(match key {
        "scheme" => a.scheme,
        "motivation" => a.motivation,
        "discipline" => a.discipline,
        "gm" => a.game_management,
        "recruiting" => a.recruiting,
        "network" => a.network,
        "relationships" => a.relationships,
        "calling" => match co.yours {
            Some(y) => y.play_calling,
            None => 0.6 * a.scheme + 0.4 * a.game_management,
        },
        "teach:rest" => {
            let off = match co.job.map(|j| j.role) {
                Some(Role::DefensiveCoordinator) => false,
                Some(Role::OffensiveCoordinator) => true,
                _ => co.came_up.offense(),
            };
            let gs: Vec<f64> = Group::ALL
                .iter()
                .filter(|g| g.offense() == off && **g != Group::Specialists && **g != co.came_up)
                .map(|g| a.teaching[*g as usize])
                .collect();
            gs.iter().sum::<f64>() / gs.len().max(1) as f64
        }
        k => a.teaching[k.strip_prefix("teach:")?.parse::<usize>().ok()?.min(GROUP_COUNT - 1)],
    })
}

/// The fixed code a coach's facet is snapshotted under.
fn facet_code(key: &str) -> Option<u8> {
    Some(match key {
        "teach:rest" => 8,
        "scheme" => 9,
        "motivation" => 10,
        "discipline" => 11,
        "gm" => 12,
        "recruiting" => 13,
        "network" => 14,
        "relationships" => 15,
        "calling" => 16,
        k => k.strip_prefix("teach:")?.parse::<u8>().ok()?.min(7),
    })
}

/// The group a coach teaches in his role: his own group, for a head coach
/// the group he came up coaching, and for a coordinator a room on his own
/// side (the one he came up in, or else the quarterbacks or the secondary).
fn taught(w: &World, c: usize, role: Role) -> Group {
    let up = w.coaches[c].came_up;
    match role {
        Role::PositionCoach(g) => g,
        Role::OffensiveCoordinator if !up.offense() => Group::QB,
        Role::DefensiveCoordinator if up.offense() || up == Group::Specialists => Group::DB,
        _ => up,
    }
}

fn coach_facets(w: &World, team: Team, c: usize, role: Role) -> Vec<Facet> {
    let g = taught(w, c, role);
    let t = |k: &str| coach_truth(w, c, k);
    let head = role == Role::HeadCoach;
    let coord = matches!(role, Role::OffensiveCoordinator | Role::DefensiveCoordinator);
    let mut v = Vec::new();
    let gk = format!("teach:{}", g as u8);
    v.push(facet(
        &gk,
        format!("Teaching, {}", g.name()),
        "Teaching",
        t(&gk),
        "",
        if coord || head { 0.6 } else { 1.0 },
    ));
    if coord || head {
        let side = match role {
            Role::DefensiveCoordinator => "defense",
            Role::OffensiveCoordinator => "offense",
            _ if g.offense() => "offense",
            _ => "defense",
        };
        v.push(facet("teach:rest", format!("Teaching, the rest of the {side}"), "Teaching", t("teach:rest"), "", 0.5));
    } else {
        v.push(facet(
            "teach:other",
            "Other position groups".into(),
            "Teaching",
            None,
            &format!("Has only coached the {} for you", g.name()),
            0.0,
        ));
    }
    v.push(facet("scheme", "Scheme".into(), "Coaching", t("scheme"), "", if coord || head { 1.0 } else { 0.35 }));
    v.push(facet("motivation", "Motivation".into(), "Coaching", t("motivation"), "", 0.7));
    v.push(facet("discipline", "Discipline".into(), "Coaching", t("discipline"), "", 0.7));
    v.push(if coord || head {
        facet("gm", "Game management".into(), "Coaching", t("gm"), "", if head { 1.0 } else { 0.6 })
    } else {
        facet("gm", "Game management".into(), "Coaching", None, "Doesn't run the game", 0.0)
    });
    v.push(match team {
        Team::College(_) => facet("recruiting", "Recruiting".into(), "Recruiting", t("recruiting"), "", 0.8),
        Team::High(_) => {
            facet("recruiting", "Recruiting".into(), "Recruiting", None, "High schools don't recruit", 0.0)
        }
        _ => facet("recruiting", "Recruiting".into(), "Recruiting", None, "Pro clubs don't recruit", 0.0),
    });
    v.push(facet("network", "Network".into(), "Recruiting", t("network"), "", 0.5));
    v.push(facet("relationships", "Relationships".into(), "Recruiting", t("relationships"), "", 0.7));
    v.push(match calls(w, team, c) {
        Some(off) => facet(
            "calling",
            format!("Calling, {}", if off { "offense" } else { "defense" }),
            "Game day",
            t("calling"),
            "",
            1.2,
        ),
        None if coord => facet("calling", "Calling".into(), "Game day", None, "You call the plays", 0.0),
        None => facet("calling", "Calling".into(), "Game day", None, "Doesn't call plays", 0.0),
    });
    v
}

/// A drawn rating around a base, keyed by the person and the facet.
fn drawn(who: u64, k: u64, base: f64, sd: f64) -> f64 {
    clamp(base + sd * n(who, k), 1.0, 20.0)
}

/// A job's facets by key and name.
type Names = &'static [(&'static str, &'static str)];

/// A support job's facets, each around its base rating (they average to
/// it) and "not part of this job" for the ones the job leaves to others.
fn job_facets(job: Job, who: u64, base: f64) -> Vec<Facet> {
    let (names, off): (Names, Names) = match job {
        Job::StrengthHead => (
            &[
                ("str", "Strength development"),
                ("spd", "Speed and conditioning"),
                ("inj", "Injury prevention"),
                ("nut", "Nutrition"),
            ],
            &[],
        ),
        Job::StrengthAssistant => (
            &[("spd", "Speed and conditioning"), ("str", "Strength development"), ("inj", "Injury prevention")],
            &[("nut", "Nutrition")],
        ),
        Job::Dietitian => (&[("nut", "Nutrition"), ("inj", "Injury prevention")], &[]),
        Job::Trainer => (&[("rtp", "Return-to-play judgment"), ("rehab", "Rehab"), ("diag", "Diagnosis")], &[]),
        Job::Physician => (&[("diag", "Diagnosis"), ("rtp", "Return-to-play judgment")], &[]),
        Job::AssistantTrainer => {
            (&[("rehab", "Rehab"), ("diag", "Diagnosis"), ("rtp", "Return-to-play judgment")], &[])
        }
        Job::AnalyticsHead => (&[("ana", "Analytics"), ("self", "Self-scouting"), ("opp", "Opponent study")], &[]),
        Job::Analyst | Job::Room(_) => {
            (&[("opp", "Opponent study"), ("self", "Self-scouting"), ("ana", "Analytics")], &[])
        }
        Job::Personnel => (&[("eval", "Evaluation"), ("org", "Organization")], &[]),
        Job::RecruitingDirector => (&[("org", "Organization"), ("eval", "Evaluation")], &[]),
        Job::Academic => (&[("acad", "Academic support"), ("life", "Life skills")], &[]),
        Job::Engagement => (&[("life", "Life skills")], &[("acad", "Academic support")]),
        Job::GradAssistant => (&[("teach", "Teaching"), ("opp", "Opponent study")], &[]),
        Job::Coach(_) | Job::HsCoach => (&[], &[]),
    };
    let offs: Vec<f64> = (0..names.len()).map(|k| 1.6 * n(who, 100 + k as u64)).collect();
    let mean = offs.iter().sum::<f64>() / offs.len().max(1) as f64;
    let mut v: Vec<Facet> = names
        .iter()
        .zip(&offs)
        .map(|(&(k, name), o)| facet(k, name.to_string(), "The job", Some(base + o - mean), "", 0.8))
        .collect();
    v.extend(off.iter().map(|&(k, name)| facet(k, name.to_string(), "The job", None, "Not part of this job", 0.0)));
    v
}

/// A drawn NFL room coach's ratings (the world keeps NFL position coaches
/// as drawn people): a position room teaches its group; the assistant
/// head coach helps run the game; quality control studies opponents.
fn room_facets(slot: u8, who: u64) -> Vec<Facet> {
    let group = staffing::NFL_ROOMS.get(slot as usize).map_or(9, |r| r.1);
    if slot >= 10 {
        return job_facets(Job::Room(slot), who, drawn(who, 1, 11.0, 2.3));
    }
    let mut v = Vec::new();
    if (group as usize) < GROUP_COUNT {
        let g = Group::ALL[group as usize];
        let title = if slot == 3 { "tight ends".to_string() } else { g.name().to_string() };
        v.push(facet(
            &format!("teach:{}", g as u8),
            format!("Teaching, {title}"),
            "Teaching",
            Some(drawn(who, 1, 12.5, 2.0)),
            "",
            1.0,
        ));
    }
    v.push(facet("scheme", "Scheme".into(), "Coaching", Some(drawn(who, 2, 12.0, 2.2)), "", 0.35));
    v.push(facet("motivation", "Motivation".into(), "Coaching", Some(drawn(who, 3, 11.5, 2.2)), "", 0.7));
    v.push(facet("discipline", "Discipline".into(), "Coaching", Some(drawn(who, 4, 11.5, 2.2)), "", 0.7));
    v.push(if slot == 9 {
        facet("gm", "Game management".into(), "Coaching", Some(drawn(who, 5, 12.0, 2.2)), "", 0.5)
    } else {
        facet("gm", "Game management".into(), "Coaching", None, "Doesn't run the game", 0.0)
    });
    v.push(facet("recruiting", "Recruiting".into(), "Recruiting", None, "Pro clubs don't recruit", 0.0));
    v.push(facet("network", "Network".into(), "Recruiting", Some(drawn(who, 6, 11.5, 2.2)), "", 0.5));
    v.push(facet("relationships", "Relationships".into(), "Recruiting", Some(drawn(who, 7, 11.5, 2.2)), "", 0.7));
    v.push(facet("calling", "Calling".into(), "Game day", None, "Doesn't call plays", 0.0));
    v
}

fn support(w: &World, team: Team) -> Option<&super::people::Support> {
    match team {
        Team::College(p) => w.programs.get(p).map(|p| &p.support),
        Team::Nfl(t) => w.clubs.get(t).map(|c| &c.support),
        _ => None,
    }
}

/// The level a team's staff is read against: FBS, FCS, the smaller
/// colleges, the NFL, another league or high school.
fn level_of(w: &World, team: Team) -> u8 {
    match team {
        Team::College(p) => super::norms::tier(w.programs[p].division),
        Team::Nfl(_) => 3,
        Team::Other(_) => 4,
        Team::High(_) => 5,
    }
}

/// The level in words, for the marks.
pub fn level_words(w: &World) -> &'static str {
    match Team::seat(w).map(|t| level_of(w, t)) {
        Some(0) => "FBS",
        Some(1) => "FCS",
        Some(2) => "small-college",
        Some(3) => "NFL",
        Some(4) => "pro",
        _ => "high school",
    }
}

/// What a support head and the drawn support people pay and how long:
/// the club or program's own staff (proposed, not confirmed).
fn support_pay(w: &World, team: Team, job: Job) -> f64 {
    let rich = match team {
        Team::College(p) => {
            let prog = &w.programs[p];
            match prog.division {
                super::data::Division::FBS => 0.6 + 0.4 * prog.prestige,
                super::data::Division::FCS => 0.25,
                _ => 0.12,
            }
        }
        Team::Nfl(_) => 1.4,
        _ => 0.2,
    };
    let base = match job {
        Job::StrengthHead => 450_000.0,
        Job::StrengthAssistant => 110_000.0,
        Job::Dietitian => 95_000.0,
        Job::AnalyticsHead => 160_000.0,
        Job::Personnel => 230_000.0,
        Job::RecruitingDirector => 170_000.0,
        Job::Engagement => 95_000.0,
        Job::Analyst => 75_000.0,
        Job::GradAssistant => 25_000.0,
        _ => 120_000.0,
    };
    (base * rich / 1000.0).round() * 1000.0
}

/// Drawn people the support departments add for a team: the strength
/// coach's assistant and a dietitian, a team physician and an assistant
/// athletic trainer, a personnel director and a recruiting director at
/// programs that pay for them.
pub fn support_people(w: &World, team: Team) -> Vec<(Job, staffing::Drawn)> {
    let jobs: Vec<(Job, &str, u64)> = match team {
        Team::College(p) => {
            let prog = &w.programs[p];
            let fbs = prog.division == super::data::Division::FBS;
            let fcs = prog.division == super::data::Division::FCS;
            let mut v = vec![(Job::Physician, "Team physician", 2)];
            if fbs || fcs {
                v.push((Job::StrengthAssistant, "Assistant strength coach", 0));
                v.push((Job::AssistantTrainer, "Assistant athletic trainer", 3));
                v.push((Job::RecruitingDirector, "Recruiting director", 5));
            }
            if fbs {
                v.push((Job::Dietitian, "Dietitian", 1));
                v.push((Job::Personnel, "Director of player personnel", 4));
            }
            v
        }
        Team::Nfl(_) => vec![
            (Job::StrengthAssistant, "Assistant strength coach", 0),
            (Job::Dietitian, "Dietitian", 1),
            (Job::Physician, "Team physician", 2),
            (Job::AssistantTrainer, "Assistant athletic trainer", 3),
        ],
        Team::Other(_) => vec![(Job::Physician, "Team physician", 2), (Job::AssistantTrainer, "Athletic trainer", 3)],
        Team::High(_) => Vec::new(),
    };
    jobs.into_iter()
        .map(|(job, title, k)| {
            let key = team.key() << 16 ^ 0x5D0 ^ k;
            (job, staffing::draw_person(w, key, title, support_pay(w, team, job), None))
        })
        .collect()
}

/// The base rating of a drawn support person: around his department's
/// head, or his job's level (proposed, not confirmed).
fn support_base(w: &World, team: Team, job: Job, who: u64) -> f64 {
    let s = support(w, team);
    let head = |f: fn(&super::people::Support) -> &Staffer| s.map_or(9.0, |s| f(s).rating.max(1) as f64);
    match job {
        Job::StrengthAssistant => drawn(who, 9, head(|s| &s.strength) - 1.5, 2.0),
        Job::Dietitian => drawn(who, 9, head(|s| &s.strength), 2.2),
        Job::Physician => drawn(who, 9, head(|s| &s.medical) + 1.0, 2.0),
        Job::AssistantTrainer => drawn(who, 9, head(|s| &s.medical) - 2.0, 2.2),
        Job::Personnel => drawn(who, 9, head(|s| &s.analytics), 2.5),
        Job::RecruitingDirector => drawn(who, 9, 10.5, 3.0),
        Job::Analyst => drawn(who, 9, head(|s| &s.analytics) - 1.0, 2.3),
        Job::GradAssistant => drawn(who, 9, 7.5, 2.5),
        _ => 10.0,
    }
}

/// Everyone on your staff but you, by department.
pub fn members(w: &World) -> Vec<Member> {
    let Some(team) = Team::seat(w) else { return Vec::new() };
    let me = you(w);
    let mut v = Vec::new();
    // Coaches the world keeps: your head coach when you assist, then the
    // assistants.
    let mut coaches: Vec<(usize, Role)> = Vec::new();
    if let Some(h) = team.head_coach(w).filter(|&h| Some(h) != me) {
        coaches.push((h, Role::HeadCoach));
    }
    coaches.extend(folk::facts::assistants(w, team).into_iter().filter(|&(c, _)| Some(c) != me));
    for (c, role) in coaches {
        let co = &w.coaches[c];
        let since = co.job.map_or(w.year, |j| j.since);
        // A college job's year is its hiring cycle's season: he arrived
        // that winter, after the season.
        let (start, kept) = match co.job.map(|j| j.employer) {
            Some(Employer::College(_)) => start_at(w, Date::ymd(since + 1, 1, 10)),
            _ => start_of(w, since),
        };
        let deal = w.folk.staff.deal(c);
        let pay = deal.map(|d| d.salary).or(co.contract.map(|k| k.salary));
        let through = deal.map(|d| d.through).or(co.contract.map(|k| k.through));
        // An interview or a friend tells you something before his first day.
        let warm = folk::network::warmth(w, c) as f64;
        let interviewed = w.folk.staff.interviewed.iter().any(|x| x.0 == c);
        let known = if kept { 0.4 } else { 1.5 * warm + if interviewed { 0.6 } else { 0.0 } };
        let g = taught(w, c, role);
        let title = match role {
            Role::PositionCoach(g) => cap(g.coach_title()),
            Role::OffensiveCoordinator | Role::DefensiveCoordinator => format!("{}, {}", cap(role.name()), g.name()),
            Role::HeadCoach => "Head coach".to_string(),
        };
        let facets = coach_facets(w, team, c, role);
        let keys = key_reads(&facets, role);
        v.push(Member {
            key: coach_key(c),
            job: Job::Coach(role),
            dept: Dept::Coaches,
            name: co.name.clone(),
            title,
            coach: Some(c),
            hs: None,
            group: Some(g),
            age: Some(co.age_on(w.today)),
            since,
            start,
            kept,
            not_yours: None,
            pay,
            through,
            known,
            facets,
            keys,
        });
    }
    match team {
        Team::High(_) => {
            for (i, a) in w.career.iter().flat_map(|k| k.hs_staff.iter()).enumerate() {
                let (start, kept) = start_of(w, a.since);
                let g = Group::ALL[(a.group as usize).min(GROUP_COUNT - 1)];
                let title = match a.coordinator {
                    Some(true) => format!("Offensive coordinator, {}", g.name()),
                    Some(false) => format!("Defensive coordinator, {}", g.name()),
                    None => cap(g.coach_title()),
                };
                v.push(Member {
                    key: hs_key(&a.name, a.since),
                    job: Job::HsCoach,
                    dept: Dept::Coaches,
                    name: a.name.clone(),
                    title,
                    coach: None,
                    hs: Some(i),
                    group: Some(g),
                    age: Some(a.age_on(w.today)),
                    since: a.since,
                    start,
                    kept,
                    not_yours: None,
                    pay: Some(a.stipend as f64),
                    through: Some(w.year),
                    known: if kept { 0.4 } else { 0.0 },
                    facets: vec![facet(
                        &format!("teach:{}", g as u8),
                        format!("Teaching, {}", g.name()),
                        "Teaching",
                        Some(a.teaching),
                        "",
                        1.0,
                    )],
                    keys: vec![0],
                });
            }
        }
        Team::Nfl(_) | Team::Other(_) | Team::College(_) => {
            // Drawn coaching rooms (NFL position coaches, quality control),
            // analysts and graduate assistants.
            for d in staffing::drawn_staff(w, team) {
                let who = d.who;
                let (job, dept, facets) = match team {
                    Team::Nfl(_) | Team::Other(_) => {
                        let slot = staffing::NFL_ROOMS.iter().position(|r| r.0 == d.title).unwrap_or(9) as u8;
                        let dept = if slot >= 10 { Dept::Ops } else { Dept::Coaches };
                        (Job::Room(slot), dept, room_facets(slot, who))
                    }
                    _ if d.title.starts_with("Graduate assistant") => (
                        Job::GradAssistant,
                        Dept::Ops,
                        job_facets(Job::GradAssistant, who, support_base(w, team, Job::GradAssistant, who)),
                    ),
                    _ => (
                        Job::Analyst,
                        Dept::Ops,
                        job_facets(Job::Analyst, who, support_base(w, team, Job::Analyst, who)),
                    ),
                };
                let (start, kept) = start_of(w, d.since);
                let group = staffing::NFL_ROOMS
                    .iter()
                    .find(|r| r.0 == d.title)
                    .and_then(|r| Group::ALL.get(r.1 as usize).copied());
                let keys = (0..facets.len()).filter(|&k| facets[k].seen()).take(2).collect();
                v.push(Member {
                    key: drawn_key(who),
                    job,
                    dept,
                    name: d.name,
                    title: d.title,
                    coach: None,
                    hs: None,
                    group,
                    age: Some(d.age),
                    since: d.since,
                    start,
                    kept,
                    not_yours: None,
                    pay: Some(d.pay),
                    through: Some(d.since + 2 * ((w.year - d.since).max(0) / 2 + 1) - 1),
                    known: 0.0,
                    facets,
                    keys,
                });
            }
            // The support departments' heads.
            if let Some(s) = support(w, team) {
                let college = matches!(team, Team::College(_));
                let heads: [(Job, Dept, &Staffer, &str, u64); 5] = [
                    (Job::StrengthHead, Dept::Performance, &s.strength, "Head strength coach", 1),
                    (Job::Trainer, Dept::Medical, &s.medical, "Head athletic trainer", 2),
                    (Job::AnalyticsHead, Dept::Ops, &s.analytics, "Head of analytics", 3),
                    (Job::Academic, Dept::Ops, &s.academic, "Academic adviser", 4),
                    (Job::Engagement, Dept::Ops, &s.engagement, "Player engagement", 5),
                ];
                for (job, dept, st, title, post) in heads {
                    if st.rating == 0 || (job == Job::Academic && !college) {
                        continue;
                    }
                    let key = head_key(team, post, st);
                    let (start, kept) = start_of(w, st.since);
                    let facets = job_facets(job, key, st.rating as f64);
                    let not_yours = not_yours(team, job);
                    v.push(Member {
                        key,
                        job,
                        dept,
                        name: st.name.clone(),
                        title: title.to_string(),
                        coach: None,
                        hs: None,
                        group: None,
                        age: None,
                        since: st.since,
                        start,
                        kept,
                        not_yours,
                        pay: not_yours.is_none().then(|| support_pay(w, team, job)),
                        through: not_yours.is_none().then(|| st.since + 3 * ((w.year - st.since).max(0) / 3 + 1) - 1),
                        known: 0.0,
                        keys: (0..facets.len().min(2)).collect(),
                        facets,
                    });
                }
            }
            for (job, d) in support_people(w, team) {
                let who = d.who;
                let dept = match job {
                    Job::StrengthAssistant | Job::Dietitian => Dept::Performance,
                    Job::Physician | Job::AssistantTrainer => Dept::Medical,
                    _ => Dept::Ops,
                };
                let facets = job_facets(job, who, support_base(w, team, job, who));
                let (start, kept) = start_of(w, d.since);
                let not_yours = not_yours(team, job);
                v.push(Member {
                    key: drawn_key(who),
                    job,
                    dept,
                    name: d.name,
                    title: d.title,
                    coach: None,
                    hs: None,
                    group: None,
                    age: Some(d.age),
                    since: d.since,
                    start,
                    kept,
                    not_yours,
                    pay: not_yours.is_none().then_some(d.pay),
                    through: not_yours.is_none().then(|| d.since + 2 * ((w.year - d.since).max(0) / 2 + 1) - 1),
                    known: 0.0,
                    keys: (0..facets.len()).filter(|&k| facets[k].seen()).take(2).collect(),
                    facets,
                });
            }
        }
    }
    v
}

/// Who hires a support job that isn't yours to hire: a college's medical
/// and academic staff work for the athletic department; a pro club's
/// doctors for the club.
fn not_yours(team: Team, job: Job) -> Option<&'static str> {
    match (team, job) {
        (Team::College(_), Job::Trainer | Job::Physician | Job::AssistantTrainer | Job::Academic) => {
            Some("hired by the athletic department")
        }
        (Team::Nfl(_) | Team::Other(_), Job::Physician) => Some("the club's doctors"),
        _ => None,
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// A coach's two key reads: a coordinator's calling and his teaching, a
/// position coach's teaching and recruiting (a head coach's game
/// management and scheme).
fn key_reads(f: &[Facet], role: Role) -> Vec<usize> {
    let at = |k: &str| f.iter().position(|x| x.key == k && x.seen());
    let first = match role {
        Role::HeadCoach => at("gm"),
        Role::OffensiveCoordinator | Role::DefensiveCoordinator => at("calling").or(at("scheme")),
        Role::PositionCoach(_) => Some(0),
    };
    let second = match role {
        Role::HeadCoach => at("scheme"),
        Role::OffensiveCoordinator | Role::DefensiveCoordinator => Some(0),
        Role::PositionCoach(_) => at("recruiting").or(at("motivation")),
    };
    [first, second].into_iter().flatten().collect()
}

/// Finds a member by key.
pub fn member(w: &World, key: u64) -> Option<Member> {
    members(w).into_iter().find(|m| m.key == key)
}

/// The member a coach is on your staff.
pub fn of_coach(w: &World, c: usize) -> Option<Member> {
    member(w, coach_key(c))
}

/// Your lean as a reader: -1 to 1, fixed for you, him and the rating.
fn lean(w: &World, m: &Member, f: usize) -> f64 {
    let reader = w.career.as_ref().map_or(0, |k| (k.coach as u64) << 32 ^ k.seed as u64);
    let u = super::season::key_uniform(reader ^ 0x5EAD_57AF ^ m.key.rotate_left(17), f as u64 ^ 0x9E37);
    2.0 * u - 1.0
}

/// How his room grew under him: player-seasons a season in his group,
/// against a room of eight (teaching reads narrow faster for a big room).
fn room(w: &World, m: &Member) -> f64 {
    let Some(b) = w.career.as_ref().map(|k| &k.staff_book) else { return 1.0 };
    let rows: Vec<&Row> = b.rows.iter().filter(|r| r.coach == m.key).collect();
    if rows.is_empty() {
        return 1.0;
    }
    let mut years: Vec<i32> = rows.iter().map(|r| r.year).collect();
    years.sort_unstable();
    years.dedup();
    clamp(rows.len() as f64 / years.len() as f64 / 8.0, 0.5, 1.5)
}

/// The truth a read at a day was centred on: the season's snapshot for a
/// past day, his rating now for today.
fn truth_at(w: &World, m: &Member, f: usize, at: Date) -> Option<f64> {
    let fc = &m.facets[f];
    let now = fc.truth?;
    if at >= w.today || m.coach.is_none() {
        return Some(now);
    }
    let code = facet_code(&fc.key)?;
    let snap = w
        .career
        .as_ref()
        .and_then(|k| k.staff_book.snaps.iter().find(|s| s.who == m.key && s.facet == code && s.year == at.year() - 1));
    Some(snap.map_or(now, |s| s.truth as f64))
}

/// Evidence units for a rating by a day.
fn evidence(w: &World, m: &Member, f: usize, at: Date) -> f64 {
    let fc = &m.facets[f];
    let years = m.start.days_to(at).max(0) as f64 / 365.0;
    let room = if fc.key.starts_with("teach:") { room(w, m) } else { 1.0 };
    m.known + fc.rate * years * room
}

/// Your read of one rating on a day: low and high on 1-20.
pub fn read(w: &World, m: &Member, f: usize, at: Date) -> Option<(u8, u8)> {
    let truth = truth_at(w, m, f, at)?;
    let e = evidence(w, m, f, at);
    let half = 0.75 + 2.6 / (1.0 + 0.6 * e);
    let c = truth + lean(w, m, f) * half * 0.6;
    let lo = (c - half).round().clamp(1.0, 20.0) as u8;
    let hi = (c + half).round().clamp(1.0, 20.0) as u8;
    Some((lo, hi.max(lo)))
}

/// Your read of one rating now.
pub fn now(w: &World, m: &Member, f: usize) -> Option<(u8, u8)> {
    read(w, m, f, w.today)
}

/// The days your read is shown at on the stepper: his first day with you,
/// the end of each season since (the last three), and now.
pub fn steps(w: &World, m: &Member) -> Vec<(String, Date)> {
    // The year his job carries on every page (a college hire's cycle year).
    let first = if m.kept { "You took over".to_string() } else { format!("Hired {}", m.since) };
    let mut v = vec![(first, m.start)];
    let mut ends: Vec<(String, Date)> = (m.start.year()..=w.today.year())
        .map(|y| (format!("After {y}"), Date::ymd(y + 1, 1, 15)))
        .filter(|(_, d)| *d > m.start && *d <= w.today)
        .collect();
    let cut = ends.len().saturating_sub(3);
    v.extend(ends.drain(cut..));
    v.push(("Now".to_string(), w.today));
    v
}

/// How sure your read is, from its width.
pub fn sure_words(lo: u8, hi: u8) -> &'static str {
    match hi - lo {
        0..=2 => "Sure",
        3 => "Fairly sure",
        4 | 5 => "Not sure",
        _ => "Too early",
    }
}

/// A read in a word: "Very good", or "Weak to good" when it is wide.
pub fn read_words(lo: u8, hi: u8) -> String {
    const W: [&str; 6] = ["Poor", "Weak", "Average", "Good", "Very good", "Elite"];
    let band = |v: u8| match v {
        0..=4 => 0,
        5..=8 => 1,
        9..=11 => 2,
        12..=14 => 3,
        15..=17 => 4,
        _ => 5,
    };
    if hi - lo >= 5 {
        format!("{} to {}", W[band(lo)], W[band(hi)].to_lowercase())
    } else {
        W[band(((lo as u32 + hi as u32) / 2) as u8)].to_string()
    }
}

/// Whether the read moved this season: -1 down, 1 up, 0 held (the middle
/// of the range a point or more from where it stood when last season
/// ended).
pub fn moved(w: &World, m: &Member, f: usize) -> i8 {
    let last = Date::ymd(w.today.year() - if w.today.month() >= 8 { 0 } else { 1 }, 1, 15);
    if last <= m.start {
        return 0;
    }
    match (read(w, m, f, last), now(w, m, f)) {
        (Some((a, b)), Some((c, d))) => {
            let dm = (c as i32 + d as i32) - (a as i32 + b as i32);
            if dm <= -2 {
                -1
            } else if dm >= 2 {
                1
            } else {
                0
            }
        }
        _ => 0,
    }
}

/// The same job at the same level across the world, for one rating: its
/// 25th, 50th, 75th and 90th percentiles and how many hold the job.
#[derive(Clone, Copy, Debug)]
pub struct JobMarks {
    pub q25: f64,
    pub q50: f64,
    pub q75: f64,
    pub q90: f64,
    pub n: usize,
}

fn marks_of(mut v: Vec<f64>) -> Option<JobMarks> {
    if v.len() < 3 {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let at = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
    Some(JobMarks { q25: at(0.25), q50: at(0.5), q75: at(0.75), q90: at(0.9), n: v.len() })
}

/// The teams at your level (FBS with FBS, the NFL with the NFL).
fn level_teams(w: &World, team: Team) -> Vec<Team> {
    let lv = level_of(w, team);
    match team {
        Team::College(_) => (0..w.programs.len())
            .filter(|&p| w.programs[p].plays(w.year) && super::norms::tier(w.programs[p].division) == lv)
            .map(Team::College)
            .collect(),
        Team::Nfl(_) => {
            (0..w.nfl.len()).filter(|&t| super::franchise::active_in(w, t, w.year)).map(Team::Nfl).collect()
        }
        Team::Other(_) => vec![team],
        Team::High(h) => {
            let st = w.high_schools[h].state;
            w.states[st].schools.iter().map(|&s| Team::High(s)).collect()
        }
    }
}

/// RR2: how the same job at your level rates on one rating.
pub fn job_marks(w: &World, m: &Member, f: usize) -> Option<JobMarks> {
    let team = Team::seat(w)?;
    let key = m.facets.get(f)?.key.clone();
    m.facets[f].truth?;
    let teams = level_teams(w, team);
    let mut v: Vec<f64> = Vec::new();
    match m.job {
        Job::Coach(role) => {
            let lv = level_of(w, team);
            let same = |r: Role| match (r, role) {
                (Role::PositionCoach(a), Role::PositionCoach(b)) => a == b,
                (a, b) => a == b,
            };
            for (c, co) in w.coaches.iter().enumerate() {
                let Some(j) = co.job else { continue };
                if !same(j.role) || co.retired {
                    continue;
                }
                let t = Team::from_employer(j.employer);
                if matches!(j.employer, Employer::HighSchool(_)) || level_of(w, t) != lv {
                    continue;
                }
                // Teaching is read for the group each one teaches.
                let k = if key.starts_with("teach:") && key != "teach:rest" {
                    format!("teach:{}", taught(w, c, j.role) as u8)
                } else {
                    key.clone()
                };
                v.extend(coach_truth(w, c, &k));
            }
        }
        Job::HsCoach => {
            // Every school's staff is drawn around what it can pay.
            for t in &teams {
                if let Team::High(h) = *t {
                    let mean = 4.0 + 12.0 * w.high_schools[h].resources;
                    v.extend((0..3).map(|k| clamp(mean + 2.5 * n(h as u64, k), 1.0, 20.0)));
                }
            }
        }
        Job::Room(slot) => {
            for &t in &teams {
                if let Some(d) = staffing::drawn_staff(w, t).into_iter().nth(slot as usize) {
                    v.extend(room_facets(slot, d.who).iter().find(|x| x.key == key).and_then(|x| x.truth));
                }
            }
        }
        Job::StrengthHead | Job::Trainer | Job::AnalyticsHead | Job::Academic | Job::Engagement => {
            let post = match m.job {
                Job::StrengthHead => 1,
                Job::Trainer => 2,
                Job::AnalyticsHead => 3,
                Job::Academic => 4,
                _ => 5,
            };
            for &t in &teams {
                let Some(s) = support(w, t) else { continue };
                let st = [&s.strength, &s.medical, &s.analytics, &s.academic, &s.engagement][post as usize - 1];
                if st.rating > 0 {
                    let fs = job_facets(m.job, head_key(t, post, st), st.rating as f64);
                    v.extend(fs.iter().find(|x| x.key == key).and_then(|x| x.truth));
                }
            }
        }
        job => {
            for &t in &teams {
                let found =
                    support_people(w, t).into_iter().find(|(j, _)| *j == job).map(|(j, d)| (j, d.who)).or_else(|| {
                        staffing::drawn_staff(w, t)
                            .into_iter()
                            .find(|d| match job {
                                Job::GradAssistant => d.title.starts_with("Graduate assistant"),
                                Job::Analyst => d.title.contains("nalyst"),
                                _ => false,
                            })
                            .map(|d| (job, d.who))
                    });
                if let Some((j, who)) = found {
                    let fs = job_facets(j, who, support_base(w, t, j, who));
                    v.extend(fs.iter().find(|x| x.key == key).and_then(|x| x.truth));
                }
            }
        }
    }
    marks_of(v)
}

/// A season of his room: the year, its mean and each player's growth.
pub type Season = (i32, f64, Vec<(u32, f64)>);

/// His room's season book with you: each season's growth against players
/// like them (rating points a year, the mean of his player-seasons), each
/// player's, and the total.
#[derive(Clone, Debug, Default)]
pub struct Record {
    /// (season, mean, the season's players: (player id, residual)).
    pub seasons: Vec<Season>,
    pub mean: f64,
    pub player_seasons: usize,
    /// Players whose role rose under him: (id, first season, role then,
    /// rating going into it).
    pub moved_up: Vec<(u32, i32, PlayingTime, f64)>,
}

fn role_of(k: u8) -> PlayingTime {
    match k {
        0 => PlayingTime::Starter,
        1 => PlayingTime::Rotation,
        2 => PlayingTime::Backup,
        3 => PlayingTime::Redshirt,
        4 => PlayingTime::PracticeSquad,
        _ => PlayingTime::Unsigned,
    }
}

fn role_code(r: PlayingTime) -> u8 {
    match r {
        PlayingTime::Starter => 0,
        PlayingTime::Rotation => 1,
        PlayingTime::Backup => 2,
        PlayingTime::Redshirt => 3,
        PlayingTime::PracticeSquad => 4,
        PlayingTime::Unsigned => 5,
    }
}

/// His room's record with you (the coach page's "Record with you").
pub fn record(w: &World, m: &Member) -> Record {
    let Some(b) = w.career.as_ref().map(|k| &k.staff_book) else { return Record::default() };
    let coord = matches!(m.job, Job::Coach(Role::OffensiveCoordinator | Role::DefensiveCoordinator));
    let rows: Vec<&Row> =
        b.rows.iter().filter(|r| r.coach == m.key || (coord && r.coord == m.key && r.coach != m.key)).collect();
    let mut years: Vec<i32> = rows.iter().map(|r| r.year).collect();
    years.sort_unstable();
    years.dedup();
    let seasons: Vec<Season> = years
        .iter()
        .map(|&y| {
            let pl: Vec<(u32, f64)> =
                rows.iter().filter(|r| r.year == y).map(|r| (r.player, r.residual as f64)).collect();
            let mean = pl.iter().map(|x| x.1).sum::<f64>() / pl.len().max(1) as f64;
            (y, mean, pl)
        })
        .collect();
    let all: Vec<f64> = rows.iter().map(|r| r.residual as f64).collect();
    // Shrunk toward nothing for a small room, like the world's own record.
    let mean = all.iter().sum::<f64>() / (all.len() as f64 + 4.0);
    let mut first: Vec<(u32, i32, u8, f32)> = Vec::new();
    for r in rows.iter().filter(|r| r.coach == m.key) {
        match first.iter_mut().find(|x| x.0 == r.player) {
            Some(x) if r.year < x.1 => *x = (r.player, r.year, r.role, r.start),
            Some(_) => {}
            None => first.push((r.player, r.year, r.role, r.start)),
        }
    }
    let mut moved_up: Vec<(u32, i32, PlayingTime, f64)> = first
        .into_iter()
        .filter_map(|(id, y, role, start)| {
            let i = folk::person(w, id)?;
            let now = w.people[i].last_role?;
            (role_code(now) < role).then_some((id, y, role_of(role), start as f64))
        })
        .collect();
    moved_up.sort_by_key(|x| (x.2 as u8, x.1));
    moved_up.reverse();
    moved_up.truncate(6);
    Record { seasons, mean, player_seasons: all.len(), moved_up }
}

/// Your players on your team now, with the group each plays in.
fn your_players(w: &World) -> Vec<usize> {
    let Some(team) = Team::seat(w) else { return Vec::new() };
    let roster: &[usize] = match team {
        Team::College(p) => &w.programs[p].roster,
        Team::Nfl(t) => &w.clubs[t].roster,
        Team::High(h) => &w.high_schools[h].roster,
        Team::Other(_) => return Vec::new(),
    };
    let stage = match team {
        Team::College(_) => Stage::College,
        Team::Nfl(_) => Stage::Pro,
        _ => Stage::HighSchool,
    };
    roster.iter().copied().filter(|&i| w.people.get(i).is_some_and(|p| p.stage == stage)).collect()
}

/// The players a member's work touches: his group's, his side's, or the
/// whole roster's.
fn his_players(w: &World, m: &Member) -> Vec<usize> {
    let all = your_players(w);
    match (m.job, m.group) {
        (Job::Coach(Role::OffensiveCoordinator), _) => {
            all.into_iter().filter(|&i| Group::of(w.people[i].player.position).offense()).collect()
        }
        (Job::Coach(Role::DefensiveCoordinator), _) => all
            .into_iter()
            .filter(|&i| {
                let g = Group::of(w.people[i].player.position);
                !g.offense() && g != Group::Specialists
            })
            .collect(),
        (Job::Coach(Role::HeadCoach), _) => all,
        (_, Some(g)) => all.into_iter().filter(|&i| Group::of(w.people[i].player.position) == g).collect(),
        _ => all,
    }
}

/// Your games this season: (points for, points against, plays, yards)
/// summed, and the count.
/// Before this season's first game it is last season's, and says so.
fn season_games(w: &World) -> (u32, [f64; 4], &'static str) {
    let Some(team) = Team::seat(w) else { return (0, [0.0; 4], "this season") };
    let idx = match team {
        Team::College(p) | Team::High(p) | Team::Nfl(p) | Team::Other(p) => p,
    };
    let start = Date::ymd(if w.today.month() >= 3 { w.today.year() } else { w.today.year() - 1 }, 7, 1);
    let games = w.seat_games();
    let (from, to, when) = if games.iter().any(|x| x.date >= start) {
        (start, w.today, "this season")
    } else {
        (Date::ymd(start.year() - 1, 7, 1), start, "last season")
    };
    let mut g = 0;
    let mut s = [0.0; 4];
    for x in games.iter().filter(|x| x.date >= from && x.date < to.add(1)) {
        let side = if x.teams[0] == idx { 0 } else { 1 };
        let ts = &x.stats.team;
        s[0] += x.stats.score[side] as f64;
        s[1] += x.stats.score[1 - side] as f64;
        s[2] += ts[side].plays as f64;
        s[3] += ts[side].yards;
        g += 1;
    }
    (g, s, when)
}

/// What one rating rests on, in a line.
pub fn rests_on(w: &World, m: &Member, f: usize) -> String {
    let fc = &m.facets[f];
    if !fc.seen() {
        return fc.why_not.clone();
    }
    let seasons = m.start.days_to(w.today).max(0) as f64 / 365.0;
    let time = if seasons < 0.4 {
        "Too early: a few weeks of work".to_string()
    } else {
        format!("{} with you", plural((seasons + 0.25).floor().max(1.0) as usize, "season"))
    };
    let players = his_players(w, m);
    let k = fc.key.as_str();
    if k.starts_with("teach:") && k != "teach:rest" && matches!(m.job, Job::Coach(_) | Job::HsCoach | Job::Room(_)) {
        let r = record(w, m);
        if r.player_seasons > 0 {
            return format!("{}: {} a year", plural(r.player_seasons, "player-season"), signed(r.mean));
        }
        if let Some(c) = m.coach {
            let g = m.group.map_or(0, |g| g as usize);
            let u = &w.coaches[c].units[g];
            if u.seasons >= 1.0 {
                return format!(
                    "Before you: {} a year over {}",
                    signed(u.growth_read()),
                    plural(u.seasons.round() as usize, "player-season")
                );
            }
        }
        return format!("{time}; no season's growth measured yet");
    }
    match k {
        "scheme" | "calling" | "gm"
            if matches!(
                m.job,
                Job::Coach(Role::OffensiveCoordinator | Role::DefensiveCoordinator | Role::HeadCoach)
            ) =>
        {
            let (g, s, when) = season_games(w);
            if g == 0 {
                return format!("{time}; no game this season yet");
            }
            let off = !matches!(m.job, Job::Coach(Role::DefensiveCoordinator));
            if off {
                format!("{:.1} points a game, {:.1} yards a play {when}", s[0] / g as f64, s[3] / s[2].max(1.0))
            } else {
                format!("{:.1} points a game allowed {when} ({})", s[1] / g as f64, plural(g as usize, "game"))
            }
        }
        "motivation" | "relationships" if !players.is_empty() => {
            let happy = players.iter().filter(|&&i| w.people[i].mood > 0.15).count();
            let sour = players.iter().filter(|&&i| w.people[i].mood < -0.15).count();
            format!("His players: {happy} of {} happy, {sour} unhappy", players.len())
        }
        "discipline" | "life" if !players.is_empty() => {
            let trouble = players.iter().filter(|&&i| w.people[i].incident).count();
            if trouble == 0 {
                format!("No off-field trouble among {} this season", plural(players.len(), "player"))
            } else {
                format!("{} with off-field trouble this season", plural(trouble, "player"))
            }
        }
        "network" | "recruiting" if m.coach.is_some() => {
            let co = &w.coaches[m.coach.unwrap_or(0)];
            let close: Vec<usize> = co.ties.iter().filter(|x| *x.1 >= 0.3).map(|x| *x.0).collect();
            if close.is_empty() {
                return format!("{time}; few high school ties yet");
            }
            if k == "recruiting" {
                let from = players.iter().filter(|&&i| close.contains(&w.people[i].hs)).count();
                return format!(
                    "{} from schools he's close to, of {} on the roster",
                    plural(from, "player"),
                    players.len()
                );
            }
            let mut states: Vec<(usize, usize)> = Vec::new();
            for &h in &close {
                let s = w.high_schools.get(h).map_or(usize::MAX, |x| x.state);
                match states.iter_mut().find(|x| x.0 == s) {
                    Some(x) => x.1 += 1,
                    None => states.push((s, 1)),
                }
            }
            states.sort_by_key(|x| std::cmp::Reverse(x.1));
            let names: Vec<&str> =
                states.iter().take(3).filter_map(|x| w.states.get(x.0).map(|s| s.code.as_str())).collect();
            format!("Close to {} high school coaches, most in {}", close.len(), names.join(", "))
        }
        "inj" | "rehab" | "rtp" | "diag" => {
            let out =
                players.iter().filter(|&&i| super::storylines::detect::games::hurt(w, w.people[i].player.id)).count();
            let lost: u32 =
                players.iter().filter_map(|&i| w.people[i].player.history.last()).map(|h| h.games_missed).sum();
            format!(
                "{} out hurt now; {} lost to injury last season",
                plural(out, "player"),
                plural(lost as usize, "game")
            )
        }
        // The weight room shows in the whole roster's growth against
        // players like them, season by season in your book.
        "str" | "spd" => {
            let Some(b) = w.career.as_ref().map(|k| &k.staff_book) else { return time };
            let from = m.start.year();
            let Some(y) = b.rows.iter().map(|r| r.year).filter(|&y| y >= from).max() else {
                return format!("{time}; no season's growth measured yet");
            };
            let rows: Vec<f64> = b.rows.iter().filter(|r| r.year == y).map(|r| r.residual as f64).collect();
            let mean = rows.iter().sum::<f64>() / (rows.len() as f64 + 4.0);
            format!(
                "Your roster in {y}: {} a year against players like them ({})",
                signed(mean),
                plural(rows.len(), "player")
            )
        }
        // The film room shows in the games it prepared.
        "opp" | "self" | "ana" => {
            let (g, s, when) = season_games(w);
            if g == 0 {
                return format!("{time}; no game this season yet");
            }
            format!(
                "Your games {when}: {:.1} points a game for, {:.1} against ({})",
                s[0] / g as f64,
                s[1] / g as f64,
                plural(g as usize, "game")
            )
        }
        "nut" => {
            let set: Vec<(f64, f64)> = players
                .iter()
                .filter_map(|&i| {
                    let p = &w.people[i].player;
                    w.training.plan_of(p.id).body.map(|b| (b.0 as f64, crate::body::body_now(p).weight))
                })
                .collect();
            if set.is_empty() {
                return format!("{time}; no weight targets set");
            }
            let near = set.iter().filter(|(t, now)| (t - now).abs() <= 5.0).count();
            format!("{near} of {} with a weight target within 5 lb of it", plural(set.len(), "player"))
        }
        "acad" => {
            let out = players
                .iter()
                .filter(|&&i| w.people[i].cal.fall_ineligible || w.people[i].cal.spring_ineligible)
                .count();
            if out == 0 {
                "Nobody academically ineligible this year".to_string()
            } else {
                format!("{} academically ineligible", plural(out, "player"))
            }
        }
        _ => time,
    }
}

/// "+1.7" or "-0.4".
fn signed(x: f64) -> String {
    let r = (x * 10.0).round() / 10.0;
    if r == 0.0 { "0.0".into() } else { format!("{r:+.1}") }
}

fn plural(n: usize, noun: &str) -> String {
    format!("{n} {noun}{}", if n == 1 { "" } else { "s" })
}

/// The season book's yearly entry (called when the year's teaching is
/// credited, after the season's growth): your players' growth against
/// players like them with who taught them, and your coaches' ratings then.
pub(crate) fn note_season(w: &mut World, grown: &[(usize, u8, f64)]) {
    let Some(team) = Team::seat(w) else { return };
    let staff = members(w);
    let year = w.year;
    let key_for = |g: Group| -> u64 {
        staff
            .iter()
            .filter(|m| m.dept == Dept::Coaches)
            .find(|m| {
                m.group == Some(g)
                    && !matches!(
                        m.job,
                        Job::Coach(Role::OffensiveCoordinator | Role::DefensiveCoordinator | Role::HeadCoach)
                    )
            })
            .or_else(|| staff.iter().find(|m| m.group == Some(g)))
            .map_or(0, |m| m.key)
    };
    let coord_for = |g: Group| -> u64 {
        let want = if g.offense() { Role::OffensiveCoordinator } else { Role::DefensiveCoordinator };
        staff.iter().find(|m| m.job == Job::Coach(want)).map_or(0, |m| m.key)
    };
    let mine: Vec<usize> = your_players(w);
    let mut rows: Vec<Row> = Vec::new();
    for &(i, g, residual) in grown {
        if !mine.contains(&i) {
            continue;
        }
        let p = &w.people[i];
        let Some(h) = p.player.history.last() else { continue };
        let group = Group::ALL[(g as usize).min(GROUP_COUNT - 1)];
        // At a high school you teach the rooms nobody else does.
        let coach = match key_for(group) {
            0 if matches!(team, Team::High(_)) => you(w).map_or(0, coach_key),
            k => k,
        };
        rows.push(Row {
            year,
            player: p.player.id,
            group: g,
            residual: residual as f32,
            coach,
            coord: coord_for(group),
            role: role_code(h.playing_time),
            start: h.overall_start as f32,
            end: h.overall as f32,
        });
    }
    let snaps: Vec<Snap> = staff
        .iter()
        .filter(|m| m.coach.is_some())
        .flat_map(|m| {
            m.facets.iter().filter_map(move |f| {
                Some(Snap { year, who: m.key, facet: facet_code(&f.key)?, truth: f.truth? as f32 })
            })
        })
        .collect();
    if let Some(k) = w.career.as_mut() {
        let b = &mut k.staff_book;
        b.rows.retain(|r| r.year != year && r.year > year - KEEP_SEASONS);
        b.snaps.retain(|s| s.year != year && s.year > year - KEEP_SEASONS);
        b.rows.extend(rows);
        b.snaps.extend(snaps);
    }
}
