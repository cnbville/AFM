//! G1 Ambitions (boards 34 and 35): your own goals at a job, separate from
//! your boss's, offered from your real situation when you take the job and
//! when life changes, broken into the steps a real one takes. Each step pays
//! lesson points (0.25 to 1.00 by how hard it is, more from a harder start
//! and more said out loud) into the rating it used, plus reputation. Up to
//! three of your own at a time (Cain 2026-10-24); your boss's, your family's
//! and the fans' come on top and don't use your three. Some close for good,
//! and a public one that closes costs reputation.
//!
//! The kinds and their ladders follow board 35 at each level. The sizes are
//! Claude's picks (proposed, not confirmed).

use super::{Bank, Played, World, Yr};
use crate::calendar::{Date, Level};
use crate::world::coach::{Employer, Role};
use crate::world::stories::TeamKey;
use crate::world::storylines::catalogue::Rating;

/// The most ambitions of your own at once.
pub const MAX: usize = 3;
/// Days the picker waits after "Decide later".
const LATER_DAYS: i32 = 60;

/// What an ambition is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    /// The state title, the national title, the Super Bowl.
    WinItAll,
    /// The conference (college), the division (NFL), the district.
    Title,
    /// Beat a coach: your mentor, your old head coach, a rival's.
    Beat,
    /// A losing team into a playoff or bowl team.
    Turnaround,
    /// Ten-win seasons.
    TenWins,
    /// A college staff (high school), a power-conference job (college), a
    /// say in your own extension (NFL).
    Reach,
    /// Players signed with Division I schools (high school).
    Placement,
    /// A player of yours all the way to the NFL (high school).
    FutureNfl,
    /// Players drafted (college).
    Drafted,
    /// A Heisman finalist (college).
    Heisman,
    /// A late pick into a starter (NFL).
    LatePick,
    /// Pro Bowlers (NFL).
    ProBowlers,
    /// Seasons at one place.
    Years,
    /// Career wins.
    Wins,
    /// A new stadium (high school), a football building (college).
    Building,
    /// A full stadium in December (NFL).
    Crowds,
    /// Coach your son.
    Son,
    /// Win at your alma mater, your old high school, your hometown club.
    Home,
}

/// One step of an ambition's ladder.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Step {
    pub label: String,
    /// Lesson points it pays (said out loud adds a fifth).
    pub pay: f32,
    pub done: Option<Date>,
}

/// One ambition: yours (picked), offered (waiting on the picker), or done
/// and closed ones kept for the record.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Ambition {
    pub id: u32,
    pub kind: Kind,
    /// The team it was taken at.
    pub team: TeamKey,
    /// "Win it all", "People", "Build", "Beat a person", "Reach", "Last".
    pub group: String,
    pub title: String,
    pub why: String,
    pub steps: Vec<Step>,
    /// "Never closes.", "Closes if he retires first."
    pub closes: String,
    pub into: Bank,
    pub loud: bool,
    pub picked: bool,
    pub offered: Date,
    /// The day you took it on.
    pub set: Option<Date>,
    pub closed: Option<(Date, String)>,
    pub done: Option<Date>,
    /// What it is about: a coach, a program, a child's index, a project.
    pub subject: u32,
    /// The count when it was taken (wins so far, drafted so far).
    pub base: i32,
    /// How hard the start was, 0-1: it pays more.
    pub hard: f32,
    /// Its story, when said out loud.
    pub story: Option<u32>,
}

impl Ambition {
    /// Still being chased.
    pub fn open(&self) -> bool {
        self.picked && self.set.is_some() && self.closed.is_none() && self.done.is_none()
    }
    /// The next step's index.
    pub fn next(&self) -> Option<usize> {
        self.steps.iter().position(|s| s.done.is_none())
    }
    /// A step's pay as it stands (out loud pays a fifth more, to 1.00).
    pub fn pay_of(&self, k: usize) -> f32 {
        let p = self.steps.get(k).map_or(0.0, |s| s.pay);
        if self.loud { round05((p * 1.2).min(1.0)) } else { p }
    }
    /// The reputation a step pays (of 20).
    pub fn rep_of(&self, k: usize) -> f64 {
        let p = self.pay_of(k) as f64;
        0.05 + 0.15 * p + if self.loud { 0.05 } else { 0.0 }
    }
}

/// The picker waiting after a hire (board 35).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Picker {
    pub team: TeamKey,
    pub date: Date,
    /// The offers (ids in `Legacy::ambitions`).
    pub offers: Vec<u32>,
    /// "a 5-7 team: a harder start pays more".
    pub start: String,
    pub hard: f32,
    /// Put off until this day ("Decide later").
    pub later: Option<Date>,
    /// The way you were introduced: "Introduced", "Hired by the board".
    pub how: String,
}

fn round05(x: f32) -> f32 {
    (x * 20.0).round() / 20.0
}

/// A step's pay from its difficulty (0-1) and the start's: 0.25 for the
/// easiest step of an easy start, 1.00 for the top of a hard one.
fn pay_for(diff: f32, hard: f32) -> f32 {
    round05(((0.25 + 0.75 * diff) * (1.0 + 0.35 * hard)).clamp(0.25, 1.0))
}

fn steps(labels: &[&str], diffs: &[f32], hard: f32) -> Vec<Step> {
    labels.iter().zip(diffs).map(|(l, &d)| Step { label: l.to_string(), pay: pay_for(d, hard), done: None }).collect()
}

fn owned(labels: &[String], diffs: &[f32], hard: f32) -> Vec<Step> {
    let v: Vec<&str> = labels.iter().map(String::as_str).collect();
    steps(&v, diffs, hard)
}

// ---------------------------------------------------------------------------
// Offers
// ---------------------------------------------------------------------------

/// What a team's last finished season was: (wins, losses), when it has one.
fn last_record(w: &World, t: TeamKey) -> Option<(u32, u32)> {
    let s = crate::world::folk::facts::seasons(w, crate::world::storylines::folk_team(t), 1);
    s.last().map(|s| (s.wins, s.losses))
}

/// How hard a start is, 0-1, from the team's last record (a 1-8 team about
/// 0.8, a 5-7 team about 0.35, a winner 0).
fn hardness(w: &World, t: TeamKey) -> f32 {
    let Some((wn, l)) = last_record(w, t) else { return 0.3 };
    let pct = wn as f32 / (wn + l).max(1) as f32;
    (1.0 - 1.6 * pct).clamp(0.0, 1.0)
}

/// "a 5-7 team", "a last-place team".
fn start_words(w: &World, t: TeamKey) -> String {
    let Some((wn, l)) = last_record(w, t) else { return "a new program".into() };
    if let TeamKey::Nfl(n) = t
        && nfl_place(w, n as usize, w.year - 1) == Some(4)
    {
        return "a last-place team".into();
    }
    format!("a {wn}-{l} team")
}

/// An NFL team's place in its division in a finished season.
pub(crate) fn nfl_place(w: &World, t: usize, year: i32) -> Option<u8> {
    let div = crate::world::franchise::era(t, year)?.div;
    let mine = w.nfl_seasons.iter().find(|s| s.year == year && s.team == t)?;
    let pct = |s: &crate::world::nfl_year::NflTeamSeason| (s.wins as f64 + 0.5 * s.ties as f64, s.round);
    let me = pct(mine);
    let ahead = w
        .nfl_seasons
        .iter()
        .filter(|s| s.year == year && s.team != t)
        .filter(|s| crate::world::franchise::era(s.team, year).is_some_and(|e| e.div == div))
        .filter(|s| {
            let o = pct(s);
            o.0 > me.0 || (o.0 == me.0 && o.1 > me.1)
        })
        .count();
    Some(1 + ahead as u8)
}

/// An NFL club's conference ("AFC", "NFC").
fn afc_nfc(w: &World, t: usize) -> &'static str {
    match crate::world::franchise::era(t, w.year).map(|e| super::div_name(e.div, w.year)) {
        Some(d) if d.starts_with("NFC") => "NFC",
        Some(_) => "AFC",
        None => "Conference",
    }
}

/// The conference's (or division's) name for a team.
fn title_name(w: &World, t: TeamKey) -> String {
    match t {
        TeamKey::College(p) => w.programs[p as usize].conference.clone(),
        TeamKey::Nfl(n) => crate::world::franchise::era(n as usize, w.year)
            .map(|e| super::div_name(e.div, w.year).to_string())
            .unwrap_or_else(|| "the division".into()),
        _ => "the district".into(),
    }
}

/// A high school's class label ("3A") for its title ambition.
fn class_of(w: &World, h: usize) -> String {
    let code = &w.states[w.high_schools[h].state].code;
    crate::world::hs_playoffs::format(code).class_label(w.high_schools[h].classification).unwrap_or("").to_string()
}

/// Seasons of a team's history (any team, the world's books).
fn books(w: &World, t: TeamKey, n: usize) -> Vec<crate::world::folk::facts::Season> {
    crate::world::folk::facts::seasons(w, crate::world::storylines::folk_team(t), n)
}

/// Your own past seasons elsewhere, newest last.
fn your_lines(w: &World) -> Vec<&Yr> {
    w.legacy.seasons.iter().collect()
}

struct Offer {
    kind: Kind,
    group: &'static str,
    title: String,
    why: String,
    labels: Vec<String>,
    diffs: [f32; 5],
    closes: &'static str,
    into: Bank,
    subject: u32,
    base: i32,
    /// How well it fits you, for the order on the board.
    fit: f32,
}

impl Offer {
    #[allow(clippy::too_many_arguments)]
    fn new(
        kind: Kind,
        group: &'static str,
        title: String,
        why: String,
        labels: &[&str],
        diffs: [f32; 5],
        closes: &'static str,
        into: Bank,
        fit: f32,
    ) -> Offer {
        Offer {
            kind,
            group,
            title,
            why,
            labels: labels.iter().map(|s| s.to_string()).collect(),
            diffs,
            closes,
            into,
            subject: 0,
            base: 0,
            fit,
        }
    }
}

/// A coach you know who coaches against you now: your mentor first, then
/// anyone you worked for, then the rival's coach.
fn beat_target(w: &World, team: TeamKey, you: usize) -> Option<(usize, String)> {
    let level = team.level();
    let mentor = w.coaches[you].mentor;
    let same_league = |c: usize| -> Option<TeamKey> {
        let j = w.coaches.get(c)?.job?;
        if j.role != Role::HeadCoach {
            return None;
        }
        let t = match j.employer {
            Employer::College(p) => TeamKey::College(p as u32),
            Employer::Nfl(n) => TeamKey::Nfl(n as u16),
            Employer::HighSchool(h) => TeamKey::Hs(h as u32),
            Employer::Other(_) => return None,
        };
        if t == team || t.level() != level {
            return None;
        }
        let near = match (team, t) {
            (TeamKey::College(a), TeamKey::College(b)) => {
                w.programs[a as usize].conference == w.programs[b as usize].conference
                    || crate::world::folk::facts::rival(w, crate::world::folk::Team::College(a as usize))
                        .is_some_and(|r| r.0 == crate::world::folk::Team::College(b as usize))
            }
            (TeamKey::Nfl(a), TeamKey::Nfl(b)) => {
                let (ea, eb) = (
                    crate::world::franchise::era(a as usize, w.year),
                    crate::world::franchise::era(b as usize, w.year),
                );
                ea.zip(eb).is_some_and(|(x, y)| x.div == y.div)
            }
            (TeamKey::Hs(a), TeamKey::Hs(b)) => {
                let (x, y) = (&w.high_schools[a as usize], &w.high_schools[b as usize]);
                x.state == y.state && x.area == y.area
            }
            _ => false,
        };
        near.then_some(t)
    };
    if let Some(m) = mentor
        && let Some(t) = same_league(m)
    {
        let was = w.coaches[m]
            .history
            .iter()
            .rev()
            .find(|s| s.role == Role::HeadCoach)
            .map(|s| crate::world::front_office::market::employer_name(w, s.employer));
        let since = w.coaches[m].job.map_or(w.year, |j| j.since);
        let why = match (team, was) {
            (TeamKey::Nfl(_), Some(at)) => {
                format!("Your mentor in {at} now coaches {}. Twice a year.", super::tname(w, t))
            }
            (TeamKey::Nfl(_), None) => format!("Your mentor now coaches {}. Twice a year.", super::tname(w, t)),
            (_, Some(at)) => format!("Your mentor at {at}. He took the {} job in {since}.", super::tname(w, t)),
            (_, None) => format!("Your mentor. He took the {} job in {since}.", super::tname(w, t)),
        };
        return Some((m, why));
    }
    let known: Vec<(usize, String)> =
        w.career.as_ref().map(|c| c.known.iter().map(|k| (k.coach, k.how.clone())).collect()).unwrap_or_default();
    for (c, how) in known {
        if Some(c) == mentor {
            continue;
        }
        if let Some(t) = same_league(c) {
            let how = how.trim_end_matches('.').to_string();
            let what = if how.is_empty() { "You know him".to_string() } else { cap(&how) };
            let on = match team {
                TeamKey::Nfl(_) => "Twice a year.".to_string(),
                _ => format!("{} is on the schedule every year.", super::tname(w, t)),
            };
            return Some((c, format!("{what}. {on}")));
        }
    }
    let (r, ..) = crate::world::folk::facts::rival(w, crate::world::storylines::folk_team(team))?;
    let rt = crate::world::storylines::team_key(r);
    let c = crate::world::storylines::detect::head(w, rt)?;
    let since = w.coaches[c].job.map_or(w.year, |j| j.since);
    let name = super::tname(w, rt);
    let name = name.strip_prefix("the ").unwrap_or(&name);
    Some((c, format!("The {name} coach since {since}, and the man the fans want beaten.")))
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Career milestones above where you stand: the next five on the level's
/// list (high school wins run to McKissick's 621).
fn milestones(level: Level, now: u32) -> Vec<u32> {
    let list: &[u32] = match level {
        Level::HighSchool => &[50, 100, 150, 200, 250, 300, 350, 400, 450, 500, 621],
        _ => &[25, 50, 100, 150, 175, 200, 250, 300, 325, 347],
    };
    let mut v: Vec<u32> = list.iter().copied().filter(|&m| m > now).take(5).collect();
    while v.len() < 5 {
        let last = v.last().copied().unwrap_or(now);
        v.push(last + 50);
    }
    v
}

/// The ambitions that fit you at a team, best fit first (eight at most).
fn offers_for(w: &World, team: TeamKey) -> Vec<Offer> {
    let Some(c) = w.career.as_ref() else { return Vec::new() };
    let you = c.coach;
    let level = team.level();
    let name = super::tname(w, team);
    let short = crate::world::stories::short_name(w, team);
    let head = c.role.is_none();
    let mut out: Vec<Offer> = Vec::new();
    let hist = books(w, team, 40);
    let wins_now = crate::world::storylines::detect::career_wins(w, you);
    let r = Bank::Rating;
    match team {
        TeamKey::Hs(h) => {
            let h = h as usize;
            let class = class_of(w, h);
            let last_title = hist.iter().rev().find(|s| s.note.contains("state champions")).map(|s| s.year);
            let why = match last_title {
                Some(y) => format!("{name} last won one in {y}."),
                None => format!("{name} has never won one."),
            };
            let title =
                if class.is_empty() { "A state title".to_string() } else { format!("A state title in {class}") };
            out.push(Offer::new(
                Kind::WinItAll,
                "Win it all",
                title,
                why,
                &["Playoffs", "Quarterfinal", "Semifinal", "Title game", "State title"],
                [0.25, 0.75, 0.8, 1.0, 1.0],
                "Never closes.",
                r(Rating::GameManagement),
                1.0,
            ));
            // Sending players to Division I.
            let fbs_last = super::hs::last_fbs_signee(w, h);
            let why = match fbs_last {
                Some(y) if y >= w.year - 2 => format!("{name} sent a player to an FBS school in {y}."),
                Some(y) => format!("Nobody from {name} has signed with an FBS school since {y}."),
                None => format!("Nobody from {name} has signed with an FBS school in years."),
            };
            out.push(Offer::new(
                Kind::Placement,
                "People",
                "Send 10 players to D1".into(),
                why,
                &["2 players", "4 players", "6 players", "8 players", "10 players"],
                [0.2, 0.2, 0.5, 0.55, 0.85],
                "Never closes.",
                r(Rating::Network),
                0.9,
            ));
            // The years.
            let kid = c.you.profile.children.iter().filter(|k| (0..=13).contains(&(w.year - k.1))).min_by_key(|k| k.1);
            let why = match kid {
                Some((n, b)) if w.year - b <= 6 => format!("{n} starts school here in the fall."),
                Some((n, _)) => format!("{n} could finish school here."),
                None => format!(
                    "The longest-serving coaches in the state are their towns' fixtures. {name} hasn't had one."
                ),
            };
            out.push(Offer::new(
                Kind::Years,
                "Last",
                format!("20 years at {short}"),
                why,
                &["5 years", "10 years", "15 years", "18 years", "20 years"],
                [0.2, 0.2, 0.5, 0.55, 0.85],
                "Closes if you leave.",
                r(Rating::Relationships),
                0.7,
            ));
            if let Some((wn, l)) = last_record(w, team)
                && wn < l
            {
                out.push(Offer::new(
                    Kind::Turnaround,
                    "Build",
                    format!("From {wn}-{l} to the playoffs"),
                    format!("The team you inherit went {wn}-{l}."),
                    &["3 wins", ".500", "Winning year", "Playoffs", "Playoff win"],
                    [0.2, 0.2, 0.5, 0.55, 0.85],
                    "Closes after five seasons.",
                    r(Rating::Motivation),
                    0.95,
                ));
            }
            if let Some((b, why)) = beat_target(w, team, you) {
                let mut o = Offer::new(
                    Kind::Beat,
                    "Beat a person",
                    format!("Beat {}", w.coaches[b].name),
                    why,
                    &["Close game", "Beat him", "Twice", "Winning record", "In the playoffs"],
                    [0.2, 0.5, 0.55, 0.85, 1.0],
                    "Closes if he retires first.",
                    r(Rating::GameManagement),
                    0.85,
                );
                o.subject = b as u32;
                out.push(o);
            }
            out.push(Offer::new(
                Kind::Reach,
                "Reach",
                "A college staff".into(),
                super::hs::jump_why(w, h),
                &["Clinic talk", "Camp job", "An offer", "A college job", "Coordinator"],
                [0.2, 0.2, 0.5, 0.55, 0.85],
                "Closes after two offers turned down.",
                r(Rating::Network),
                0.6,
            ));
            out.push(Offer::new(
                Kind::FutureNfl,
                "People",
                "Coach a future NFL player".into(),
                super::hs::nfl_why(w, h),
                &["D1 signee", "College starter", "Drafted", "NFL starter", "Pro Bowl"],
                [0.2, 0.5, 0.85, 0.85, 1.0],
                "Never closes.",
                Bank::Teach(super::guys::best_group(w)),
                0.65,
            ));
            out.push(Offer::new(
                Kind::Building,
                "Build",
                super::hs::building_title(w, h),
                super::hs::building_why(w, h),
                &["A plan", "The board", "On the ballot", "It passes", "It opens"],
                [0.2, 0.2, 0.5, 0.85, 0.85],
                "Closes if a bond fails twice.",
                r(Rating::Negotiating),
                0.55,
            ));
            // Your old high school.
            if c.you.profile.high_school.trim().eq_ignore_ascii_case(w.school_name(h)) {
                out.push(Offer::new(
                    Kind::Home,
                    "At a place",
                    format!("Win at {short}"),
                    "Your old high school. The town remembers you in the uniform.".into(),
                    &["Winning year", "Beat the rival", "Playoffs", "District title", "State title"],
                    [0.2, 0.5, 0.5, 0.85, 1.0],
                    "Closes if you leave.",
                    r(Rating::Relationships),
                    1.1,
                ));
            }
            // Your son, when he is coming up to the school.
            if let Some((k, n)) = son_coming(w) {
                let mut o = Offer::new(
                    Kind::Son,
                    "People",
                    format!("Coach {n}"),
                    format!(
                        "{n} starts ninth grade {}.",
                        if w.year - c.you.profile.children[k].1 >= 14 { "now" } else { "next fall" }
                    ),
                    &["On your team", "Starter", "Playoffs together", "A playoff win together", "His senior night"],
                    [0.1, 0.5, 0.5, 0.85, 0.55],
                    "Closes if he plays somewhere else.",
                    r(Rating::Relationships),
                    1.2,
                );
                o.subject = k as u32;
                out.push(o);
            }
            if wins_now >= 40 {
                let m = milestones(level, wins_now);
                let mut o = Offer::new(
                    Kind::Wins,
                    "Last",
                    format!("{} wins", m[4]),
                    format!("You have {wins_now}. John McKissick won 621 at Summerville."),
                    &[],
                    [0.2, 0.5, 0.55, 0.85, 1.0],
                    "Never closes.",
                    r(Rating::Motivation),
                    0.5,
                );
                o.labels = m.iter().map(|x| x.to_string()).collect();
                out.push(o);
            }
        }
        TeamKey::College(p) => {
            let p = p as usize;
            let prog = &w.programs[p];
            let conf = prog.conference.clone();
            let contender = prog.prestige >= 0.8 || hist.iter().rev().take(4).any(|s| s.note.contains("national"));
            let titles_here: Vec<i32> = hist.iter().filter(|s| s.note.contains("champions")).map(|s| s.year).collect();
            if contender {
                let last = hist.iter().rev().find(|s| s.note.contains("national")).map(|s| s.year);
                out.push(Offer::new(
                    Kind::WinItAll,
                    "Win it all",
                    "A national title".into(),
                    match last {
                        Some(y) => format!("{short} last won it all in {y}."),
                        None => format!("{short} has never won a national title in the playoff era."),
                    },
                    &["Top ten", "Playoff", "Playoff win", "Title game", "National title"],
                    [0.2, 0.5, 0.75, 0.9, 1.0],
                    "Never closes.",
                    r(Rating::GameManagement),
                    1.0,
                ));
            }
            let mut why = match titles_here.last() {
                Some(y) => format!("{short} last won the {conf} in {y}."),
                None => format!("{short} has never won the {conf}."),
            };
            if let Some(y) = your_lines(w).into_iter().rev().find(|s| s.title && s.team != team) {
                let t = super::tname(w, y.team);
                let what = match y.team {
                    TeamKey::College(q) => w.programs[q as usize].conference.clone(),
                    _ => "title".into(),
                };
                why.push_str(&format!(" You won the {what} at {t}."));
            }
            out.push(Offer::new(
                Kind::Title,
                if contender { "Build" } else { "Win it all" },
                format!("Win the {conf}"),
                why,
                &["Bowl", "9 wins", "Title game", &format!("{conf} title"), "Playoff"],
                [0.1, 0.75, 0.75, 1.0, 1.0],
                "Never closes.",
                r(Rating::GameManagement),
                if contender { 0.8 } else { 1.0 },
            ));
            let drafted = super::guys::drafted_count(w);
            let why = match super::guys::drafted_note(w) {
                Some(s) => s,
                None => "Recruits ask one thing first: how many of yours made it.".into(),
            };
            let mut o = Offer::new(
                Kind::Drafted,
                "People",
                "50 players drafted".into(),
                why,
                &["5 drafted", "10 drafted", "20 drafted", "35 drafted", "50 drafted"],
                [0.1, 0.4, 0.4, 0.75, 1.0],
                "Never closes.",
                r(Rating::Evaluation),
                0.95,
            );
            o.base = drafted as i32;
            out.push(o);
            let tens = hist.iter().filter(|s| s.wins >= 10).count();
            out.push(Offer::new(
                Kind::TenWins,
                "Build",
                "Five ten-win seasons".into(),
                format!(
                    "{short} has had {} ten-win season{} in the last {} years.",
                    words(tens as u32),
                    if tens == 1 { "" } else { "s" },
                    hist.len()
                ),
                &["1", "2", "3", "4", "5"],
                [0.4, 0.4, 0.75, 0.75, 1.0],
                "Never closes.",
                r(Rating::Motivation),
                0.9,
            ));
            if let Some((b, why)) = beat_target(w, team, you) {
                let mut o = Offer::new(
                    Kind::Beat,
                    "Beat a person",
                    format!("Beat {}", w.coaches[b].name),
                    why,
                    &["Close game", "Beat him", "Twice", "Winning record", "Three straight"],
                    [0.1, 0.4, 0.4, 0.75, 0.75],
                    "Closes if he retires first.",
                    r(Rating::GameManagement),
                    0.85,
                );
                o.subject = b as u32;
                out.push(o);
            }
            let big = matches!(conf.as_str(), "SEC" | "Big Ten");
            if !big || !head {
                let (title, labels) = if head {
                    ("A job in the SEC or Big Ten".to_string(), ["Bowl", "Ranked", "Applied", "An offer", "The job"])
                } else {
                    ("A head job".to_string(), ["Coordinator", "Applied", "An offer", "The job", "Winning year"])
                };
                let town = w.programs[p].city.clone();
                out.push(Offer::new(
                    Kind::Reach,
                    "Reach",
                    title,
                    format!("Your agent calls {town} a place to prove yourself."),
                    &labels,
                    [0.1, 0.1, 0.4, 0.4, 0.75],
                    "Closes after ten years here.",
                    r(Rating::Network),
                    0.7,
                ));
            }
            let moves = w.coaches[you].history.len();
            out.push(Offer::new(
                Kind::Years,
                "Last",
                format!("Retire at {short}"),
                if moves >= 2 {
                    format!("You have moved {} times.", words(moves as u32))
                } else {
                    "Joe Paterno stayed 46 seasons at one school.".into()
                },
                &["5 years", "8 years", "10 years", "15 years", "20 years"],
                [0.1, 0.1, 0.4, 0.75, 1.0],
                "Closes if you leave.",
                r(Rating::Relationships),
                0.6,
            ));
            out.push(Offer::new(
                Kind::Heisman,
                "People",
                "Coach a Heisman finalist".into(),
                super::guys::heisman_note(w).unwrap_or_else(|| {
                    "Heisman winners come from ranked teams with a quarterback or a back the country watches.".into()
                }),
                &["All-conference", "All-American", "Top ten", "Finalist", "Winner"],
                [0.1, 0.4, 0.75, 1.0, 1.0],
                "Never closes.",
                Bank::Teach(super::guys::best_group(w)),
                0.6,
            ));
            if let Some(why) = super::college::building_why(w, p) {
                out.push(Offer::new(
                    Kind::Building,
                    "Build",
                    "A new football building".into(),
                    why,
                    &["A plan", "Lead gift", "Ground broken", "Opens", "Paid off"],
                    [0.1, 0.4, 0.4, 0.75, 0.4],
                    "Closes if the plan is shelved.",
                    r(Rating::Negotiating),
                    0.55,
                ));
            }
            if c.you.profile.alma_mater.trim().eq_ignore_ascii_case(w.program_name(p)) {
                out.push(Offer::new(
                    Kind::Home,
                    "At a place",
                    format!("Win at {short}"),
                    "Your alma mater. Kirby Smart went home to Georgia and won it all.".into(),
                    &["Winning year", "Beat the rival", "Bowl", &format!("{conf} title"), "Playoff"],
                    [0.1, 0.4, 0.4, 0.75, 1.0],
                    "Closes if you leave.",
                    r(Rating::Relationships),
                    1.1,
                ));
            }
            if let Some((wn, l)) = last_record(w, team)
                && wn * 2 < wn + l
            {
                out.push(Offer::new(
                    Kind::Turnaround,
                    "Build",
                    format!("From {wn}-{l} to a bowl"),
                    format!("The team you inherit went {wn}-{l}."),
                    &["Five wins", "Bowl", "Eight wins", "Bowl win", "Ten wins"],
                    [0.1, 0.4, 0.75, 0.75, 1.0],
                    "Closes after five seasons.",
                    r(Rating::Motivation),
                    0.9,
                ));
            }
            if wins_now >= 40 {
                let m = milestones(level, wins_now);
                let mut o = Offer::new(
                    Kind::Wins,
                    "Last",
                    format!("{} wins", m[4]),
                    format!("You have {wins_now}. Joe Paterno won 409."),
                    &[],
                    [0.1, 0.4, 0.75, 0.75, 1.0],
                    "Never closes.",
                    r(Rating::Motivation),
                    0.5,
                );
                o.labels = m.iter().map(|x| x.to_string()).collect();
                out.push(o);
            }
            let _ = titles_here;
        }
        TeamKey::Nfl(n) => {
            let t = n as usize;
            let rings: Vec<i32> =
                w.nfl_seasons.iter().filter(|s| s.team == t && s.round == 5).map(|s| s.year).collect();
            let since = w.nfl_seasons.iter().map(|s| s.year).min().unwrap_or(w.year);
            out.push(Offer::new(
                Kind::WinItAll,
                "Win it all",
                "Win a Super Bowl".into(),
                match rings.last() {
                    Some(y) => format!("{short} last won the Super Bowl in the {y} season."),
                    None => format!("No Super Bowl for {short} since at least {since}."),
                },
                &["Winning year", "Playoffs", "Playoff win", &format!("{} title", afc_nfc(w, t)), "Super Bowl"],
                [0.75, 0.75, 1.0, 1.0, 1.0],
                "Never closes.",
                r(Rating::GameManagement),
                1.0,
            ));
            out.push(Offer::new(
                Kind::LatePick,
                "People",
                "Turn a late pick into a starter".into(),
                super::guys::late_note(w).unwrap_or_else(|| {
                    "Tom Brady went 199th and Brock Purdy 262nd. Late picks make a coach's name.".into()
                }),
                &["First start", "8 starts", "Full year", "Second deal", "Pro Bowl"],
                [0.1, 0.4, 0.4, 0.75, 1.0],
                "Never closes.",
                r(Rating::Evaluation),
                0.95,
            ));
            let div = title_name(w, team);
            let place = nfl_place(w, t, w.year - 1);
            out.push(Offer::new(
                Kind::Title,
                "Build",
                format!("Win the {div}"),
                match place {
                    Some(4) => "Last in the division a year ago.".to_string(),
                    Some(1) => "Champions a year ago. Holding it is harder.".to_string(),
                    Some(k) => format!("{} in the division a year ago.", ordinal_word(k as u32)),
                    None => "A new division.".into(),
                },
                &["Over .500", "Second place", "Division title", "Two titles", "Three titles"],
                [0.1, 0.1, 0.75, 0.75, 1.0],
                "Never closes.",
                r(Rating::Motivation),
                0.9,
            ));
            if let Some((b, why)) = beat_target(w, team, you) {
                let mut o = Offer::new(
                    Kind::Beat,
                    "Beat a person",
                    format!("Beat {}", w.coaches[b].name),
                    why,
                    &["Close game", "Beat him", "Sweep", "Winning record", "In the playoffs"],
                    [0.1, 0.4, 0.4, 0.75, 1.0],
                    "Closes if he leaves the division.",
                    r(Rating::GameManagement),
                    0.85,
                );
                o.subject = b as u32;
                out.push(o);
            }
            let m = milestones(level, wins_now);
            let age = c.you.age_on(w.today);
            let mut o = Offer::new(
                Kind::Wins,
                "Last",
                format!("{} wins", m[4]),
                format!("You are {age}. Shula had 347."),
                &[],
                [0.1, 0.4, 0.75, 0.75, 1.0],
                "Never closes.",
                r(Rating::Motivation),
                0.7,
            );
            o.labels = m.iter().map(|x| x.to_string()).collect();
            out.push(o);
            let pb = super::guys::pro_bowl_note(w);
            out.push(Offer::new(
                Kind::ProBowlers,
                "People",
                "Coach ten Pro Bowlers".into(),
                pb.unwrap_or_else(|| "Pro Bowlers sell a building to free agents.".into()),
                &["2 Pro Bowlers", "4 Pro Bowlers", "6 Pro Bowlers", "8 Pro Bowlers", "10 Pro Bowlers"],
                [0.1, 0.1, 0.4, 0.4, 0.75],
                "Never closes.",
                Bank::Teach(super::guys::best_group(w)),
                0.75,
            ));
            out.push(Offer::new(
                Kind::Crowds,
                "Build",
                "Fill the stadium in December".into(),
                super::nfl::crowds_why(w, t),
                &["Sellouts", "A waitlist", "Prime time", "Home playoff", "Every seat, every year"],
                [0.1, 0.1, 0.4, 0.4, 0.75],
                "Never closes.",
                r(Rating::Motivation),
                0.55,
            ));
            out.push(Offer::new(
                Kind::Reach,
                "Reach",
                "A say in your own extension".into(),
                super::nfl::contract_why(w),
                &["Year two", "Owner's trust", "Long leash", "A playoff year", "Final say"],
                [0.1, 0.1, 0.4, 0.4, 0.75],
                "Closes if you're fired.",
                r(Rating::Negotiating),
                0.6,
            ));
            if w.states.get(c.you.home_state).is_some()
                && crate::world::franchise::era(t, w.year).is_some_and(|e| w.states[c.you.home_state].code == e.state)
            {
                out.push(Offer::new(
                    Kind::Home,
                    "At a place",
                    format!("Win at home with {short}"),
                    "Your hometown club. You grew up with this team.".into(),
                    &["Winning year", "Beat the rival", "Playoffs", "Division title", "Super Bowl"],
                    [0.4, 0.4, 0.75, 0.75, 1.0],
                    "Closes if you leave.",
                    r(Rating::Relationships),
                    1.1,
                ));
            }
        }
        TeamKey::Other(_) => {}
    }
    // Ambitions you hold already aren't offered twice.
    let held: Vec<(Kind, u32)> = w.legacy.ambitions.iter().filter(|a| a.open()).map(|a| (a.kind, a.subject)).collect();
    out.retain(|o| {
        !held.contains(&(o.kind, o.subject)) || matches!(o.kind, Kind::WinItAll | Kind::Title | Kind::Years)
    });
    out.sort_by(|a, b| b.fit.total_cmp(&a.fit));
    out.truncate(8);
    out
}

fn words(n: u32) -> String {
    const W: [&str; 13] =
        ["no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve"];
    W.get(n as usize).map_or_else(|| n.to_string(), |s| s.to_string())
}

pub(crate) fn ordinal_word(n: u32) -> &'static str {
    match n {
        1 => "First",
        2 => "Second",
        3 => "Third",
        _ => "Fourth",
    }
}

/// A son of yours who starts ninth grade this fall or next, while you coach
/// a high school: (child index, his first name). Only boys' names count.
fn son_coming(w: &World) -> Option<(usize, String)> {
    let c = w.career.as_ref()?;
    if !matches!(crate::world::stories::seat_team(w)?, TeamKey::Hs(_)) {
        return None;
    }
    c.you.profile.children.iter().enumerate().find_map(|(k, (n, b))| {
        let age = w.year - b;
        let first = n.split_whitespace().next().unwrap_or("").to_string();
        ((13..=14).contains(&age) && crate::world::names::is_common_first(&first)).then_some((k, first))
    })
}

/// A new job: the offers on a fresh picker, and a letter with a link to it.
pub(crate) fn offer(w: &mut World) {
    let Some((_, team)) = super::you(w) else { return };
    // Offers nobody picked from a picker that never closed are dropped.
    close_picker(w, false);
    let list = offers_for(w, team);
    if list.is_empty() {
        return;
    }
    let hard = hardness(w, team);
    let today = w.today;
    let mut ids = Vec::new();
    for o in list {
        let id = w.legacy.next_id;
        w.legacy.next_id += 1;
        let steps = owned(&o.labels, &o.diffs, hard);
        w.legacy.ambitions.push(Ambition {
            id,
            kind: o.kind,
            team,
            group: o.group.to_string(),
            title: o.title,
            why: o.why,
            steps,
            closes: o.closes.to_string(),
            into: o.into,
            loud: false,
            picked: false,
            offered: today,
            set: None,
            closed: None,
            done: None,
            subject: o.subject,
            base: o.base,
            hard,
            story: None,
        });
        ids.push(id);
    }
    let how = match team {
        TeamKey::Hs(_) => "Hired by the board".to_string(),
        _ => "Introduced".to_string(),
    };
    w.legacy.picker = Some(Picker {
        team,
        date: today,
        offers: ids,
        start: if hard > 0.0 {
            format!("{}: a harder start pays more", start_words(w, team))
        } else {
            format!("{}: no bonus for a hard start", start_words(w, team))
        },
        hard,
        later: None,
        how,
    });
    let body = vec![
        "Every coach takes a job for reasons the contract doesn't mention. Pick up to three of your own, \
         and say them out loud at your introduction or keep them to yourself."
            .to_string(),
        "[[s:legacy_ambitions|What do you want here?]]".to_string(),
    ];
    super::memo(w, "legacy: ambitions", "What do you want here?".into(), body);
}

/// Picks or drops an offer on the picker (three at most).
pub fn pick(w: &mut World, id: u32, on: bool) -> Result<String, String> {
    let p = w.legacy.picker.as_ref().ok_or("There's nothing to pick right now.")?;
    if !p.offers.contains(&id) {
        return Err("That ambition isn't on offer.".into());
    }
    let mine = w.legacy.ambitions.iter().filter(|a| a.open()).count();
    let picked = w.legacy.ambitions.iter().filter(|a| p.offers.contains(&a.id) && a.picked).count();
    let a = w.legacy.ambitions.iter_mut().find(|a| a.id == id).ok_or("No such ambition.")?;
    if on && !a.picked && mine + picked >= MAX {
        return Err(format!("Three at most. Drop one first ({mine} you already hold count)."));
    }
    a.picked = on;
    if !on {
        a.loud = false;
    }
    Ok(if on { format!("Picked: {}.", a.title) } else { format!("Dropped: {}.", a.title) })
}

/// Says a picked ambition out loud, or keeps it private.
pub fn loud(w: &mut World, id: u32, loud: bool) -> Result<String, String> {
    let open_picker = w.legacy.picker.as_ref().map(|p| p.offers.clone()).unwrap_or_default();
    let a = w.legacy.ambitions.iter_mut().find(|a| a.id == id).ok_or("No such ambition.")?;
    if !open_picker.contains(&id) {
        return Err("Only a new ambition can be said out loud: once it's yours, the words are out.".into());
    }
    if !a.picked {
        return Err("Pick it first.".into());
    }
    a.loud = loud;
    Ok(if loud { "Out loud: the press and fans will hold you to it.".into() } else { "Private.".into() })
}

/// Keeps the picked ones (or puts the choice off for 60 days).
pub fn keep(w: &mut World, later: bool) -> Result<String, String> {
    let Some(p) = w.legacy.picker.as_mut() else { return Err("There's nothing to pick right now.".into()) };
    if later {
        if p.later.is_some() {
            return Err("You already put it off once.".into());
        }
        p.later = Some(w.today.add(LATER_DAYS));
        return Ok(format!("It waits until {}.", w.today.add(LATER_DAYS).pretty()));
    }
    let n = close_picker(w, true);
    Ok(match n {
        0 => "No ambitions of your own for now.".into(),
        1 => "One ambition of your own.".into(),
        n => format!("{} ambitions of your own.", words(n as u32)),
    })
}

/// Closes the picker: picked offers become yours, the rest are dropped.
/// Returns how many you took.
fn close_picker(w: &mut World, keep: bool) -> usize {
    let Some(p) = w.legacy.picker.take() else { return 0 };
    let today = w.today;
    let mut taken = Vec::new();
    for a in w.legacy.ambitions.iter_mut().filter(|a| p.offers.contains(&a.id)) {
        if keep && a.picked {
            a.set = Some(today);
            taken.push(a.id);
        }
    }
    w.legacy.ambitions.retain(|a| !p.offers.contains(&a.id) || taken.contains(&a.id));
    for id in &taken {
        start(w, *id);
    }
    taken.len()
}

/// A new ambition of yours: its counts from today and, said out loud, its
/// headline and story.
fn start(w: &mut World, id: u32) {
    let Some(k) = w.legacy.ambitions.iter().position(|a| a.id == id) else { return };
    let base = match w.legacy.ambitions[k].kind {
        Kind::Drafted => super::guys::drafted_count(w) as i32,
        Kind::ProBowlers => super::guys::pro_bowlers(w) as i32,
        Kind::Placement => 0,
        _ => w.legacy.ambitions[k].base,
    };
    w.legacy.ambitions[k].base = base;
    // Steps already behind you (wins so far, drafted so far) count as done
    // without paying: the ladder starts where you stand.
    let reached = reached(w, &w.legacy.ambitions[k]);
    let today = w.today;
    let a = &mut w.legacy.ambitions[k];
    if matches!(a.kind, Kind::Wins) {
        for (s, r) in a.steps.iter_mut().zip(reached) {
            if r {
                s.done = Some(today);
            }
        }
    }
    if a.loud {
        let quote = quote(w, &w.legacy.ambitions[k]);
        let a = w.legacy.ambitions[k].clone();
        let team = a.team;
        let you = crate::world::stories::seat_coach(w).unwrap_or(0);
        let spot = crate::world::storylines::Spot::new(
            crate::world::storylines::catalogue::K::YourAmbition,
            crate::world::storylines::subj(you as u64, a.id as u64),
        )
        .team(team)
        .lead(crate::world::storylines::Who::Coach(you as u32))
        .with(crate::world::storylines::Who::Boss(team), crate::world::storylines::Part::Boss)
        .word(a.title.clone())
        .fact(format!("{} at his introduction: \"{quote}\"", crate::world::storylines::detect::cname(w, you)), 45.0)
        .public();
        let id = crate::world::storylines::detect::tell(w, spot);
        if let Some(a) = w.legacy.ambitions.iter_mut().find(|x| x.id == a.id) {
            a.story = id;
        }
    }
}

/// What you said at your introduction, for an ambition said out loud.
pub fn quote(w: &World, a: &Ambition) -> String {
    match a.kind {
        Kind::WinItAll => match a.team {
            TeamKey::Hs(_) => "We're here to win a state title.".into(),
            TeamKey::Nfl(_) => "We're here to win a Super Bowl.".into(),
            _ => "We're here to win a national championship.".into(),
        },
        Kind::Title => format!("We're going to win the {}.", title_name(w, a.team)),
        Kind::Beat => {
            let n = w.coaches.get(a.subject as usize).map_or(String::new(), |c| c.name.clone());
            format!("I respect {n}. I didn't come here to lose to him.")
        }
        Kind::Turnaround => "Nobody here is going to accept losing anymore. That starts today.".into(),
        Kind::TenWins => "Ten wins is the standard here. Not the dream, the standard.".into(),
        Kind::Reach => "I want to coach at the highest level, and I'll earn it here first.".into(),
        Kind::Placement => "Every kid in this program who can play in college is going to get the chance.".into(),
        Kind::FutureNfl => "There's an NFL player in this building somewhere. We're going to find him.".into(),
        Kind::Drafted => "If you come here, we'll get you to the NFL.".into(),
        Kind::Heisman => "We're going to put a player in New York in December.".into(),
        Kind::LatePick => "Draft picks don't decide who plays here. The tape does.".into(),
        Kind::ProBowlers => "We're going to build the best players in this league.".into(),
        Kind::Years => "I'm not going anywhere. This is home.".into(),
        Kind::Wins => "I want to win more games than anyone who's done this.".into(),
        Kind::Building => "This program deserves a home that matches its people.".into(),
        Kind::Crowds => "I want this stadium full and loud in December.".into(),
        Kind::Son => "Coaching my own son here is a dream. He'll earn everything he gets.".into(),
        Kind::Home => "I grew up on this team. Coming home to win here means everything.".into(),
    }
}

// ---------------------------------------------------------------------------
// Progress
// ---------------------------------------------------------------------------

/// Your seasons and games at the ambition's team since it was set.
fn since(w: &World, a: &Ambition) -> (Vec<Yr>, Vec<Played>) {
    let from = a.set.map_or(w.year, |d| d.year());
    // A job taken in the winter counts its first season from the fall.
    let seasons: Vec<Yr> = super::seasons_at(w, a.team).into_iter().filter(|s| s.year >= from).collect();
    let set = a.set.unwrap_or(w.today);
    let games: Vec<Played> = w.legacy.games.iter().filter(|g| g.date >= set).copied().collect();
    (seasons, games)
}

/// Each step reached or not, from the world's own record.
pub fn reached(w: &World, a: &Ambition) -> [bool; 5] {
    let (seasons, games) = since(w, a);
    let here: Vec<&Played> = games.iter().filter(|g| g.team == a.team).collect();
    let any = |f: &dyn Fn(&Yr) -> bool| seasons.iter().any(f);
    let count = |f: &dyn Fn(&Yr) -> bool| seasons.iter().filter(|s| f(s)).count();
    let level = a.team.level();
    let playoff_wins = |s: &Yr| here.iter().filter(|g| g.season == s.year && g.playoff && g.won()).count();
    let mut r = [false; 5];
    match a.kind {
        Kind::WinItAll => match a.team {
            TeamKey::Hs(h) => {
                let rounds = hs_rounds(w, h as usize).max(3) as u8;
                let depth = |s: &Yr| -> u8 {
                    if s.champion {
                        5
                    } else if s.final_game {
                        4
                    } else if s.playoff >= rounds - 1 {
                        3
                    } else if s.playoff >= rounds - 2 {
                        2
                    } else if s.playoff >= 1 {
                        1
                    } else {
                        0
                    }
                };
                let best = seasons.iter().map(depth).max().unwrap_or(0);
                for (k, x) in r.iter_mut().enumerate() {
                    *x = best > k as u8;
                }
            }
            TeamKey::Nfl(_) => {
                r = [
                    any(&|s| s.wins > s.losses),
                    any(&|s| s.playoff > 0),
                    seasons.iter().any(|s| playoff_wins(s) > 0),
                    any(&|s| s.final_game),
                    any(&|s| s.champion),
                ];
            }
            _ => {
                r = [
                    any(&|s| s.rank.is_some_and(|k| k <= 10)),
                    any(&|s| s.playoff > 0),
                    seasons.iter().any(|s| playoff_wins(s) > 0),
                    any(&|s| s.final_game),
                    any(&|s| s.champion),
                ];
            }
        },
        Kind::Title => match a.team {
            TeamKey::Nfl(_) => {
                let titles = count(&|s| s.title);
                r = [
                    any(&|s| s.wins > s.losses),
                    any(&|s| s.place > 0 && s.place <= 2),
                    titles >= 1,
                    titles >= 2,
                    titles >= 3,
                ];
            }
            TeamKey::Hs(_) => {
                let titles = count(&|s| s.title);
                r = [any(&|s| s.wins > s.losses), any(&|s| s.playoff > 0), titles >= 1, titles >= 2, titles >= 3];
            }
            _ => {
                r = [
                    any(&|s| s.post > 0),
                    any(&|s| s.wins >= 9),
                    any(&|s| s.conf_game),
                    any(&|s| s.title),
                    any(&|s| s.playoff > 0),
                ];
            }
        },
        Kind::Beat => {
            let c = a.subject;
            let vs: Vec<&Played> = games.iter().filter(|g| g.coach == Some(c)).collect();
            let wins = vs.iter().filter(|g| g.won()).count();
            let losses = vs.len() - wins;
            let mut streak = 0;
            let mut best = 0;
            for g in &vs {
                streak = if g.won() { streak + 1 } else { 0 };
                best = best.max(streak);
            }
            let sweep = vs
                .iter()
                .filter(|g| g.won() && !g.post)
                .fold(std::collections::BTreeMap::new(), |mut m: std::collections::BTreeMap<i32, u32>, g| {
                    *m.entry(g.season).or_default() += 1;
                    m
                })
                .values()
                .any(|&n| n >= 2);
            let fifth = match a.team {
                TeamKey::College(_) => best >= 3,
                _ => vs.iter().any(|g| g.playoff && g.won()),
            };
            r = [
                vs.iter().any(|g| g.margin() >= -7),
                wins >= 1,
                if level == Level::Nfl { sweep } else { wins >= 2 },
                vs.len() >= 3 && wins > losses,
                fifth,
            ];
        }
        Kind::Turnaround => match a.team {
            TeamKey::Hs(_) => {
                r = [
                    any(&|s| s.wins >= 3),
                    any(&|s| s.wins >= s.losses),
                    any(&|s| s.wins > s.losses),
                    any(&|s| s.playoff > 0),
                    seasons.iter().any(|s| playoff_wins(s) > 0),
                ];
            }
            TeamKey::Nfl(_) => {
                let years: Vec<i32> = seasons.iter().filter(|s| s.playoff > 0).map(|s| s.year).collect();
                r = [
                    any(&|s| s.wins >= 7),
                    any(&|s| s.wins > s.losses),
                    !years.is_empty(),
                    seasons.iter().any(|s| playoff_wins(s) > 0),
                    years.windows(2).any(|x| x[1] == x[0] + 1),
                ];
            }
            _ => {
                r = [
                    any(&|s| s.wins >= 5),
                    any(&|s| s.post > 0),
                    any(&|s| s.wins >= 8),
                    any(&|s| s.post_wins > 0),
                    any(&|s| s.wins >= 10),
                ];
            }
        },
        Kind::TenWins => {
            let n = count(&|s| s.wins >= 10);
            for (k, x) in r.iter_mut().enumerate() {
                *x = n > k;
            }
        }
        Kind::Reach => r = super::reach_steps(w, a),
        Kind::Placement => {
            let n = super::hs::d1_since(w, a.team, a.set.map_or(w.year, |d| d.year()));
            for (k, x) in r.iter_mut().enumerate() {
                *x = n >= 2 * (k as u32 + 1);
            }
        }
        Kind::FutureNfl => r = super::guys::future_nfl(w, a),
        Kind::Drafted => {
            let n = super::guys::drafted_count(w);
            for (k, goal) in [5, 10, 20, 35, 50].iter().enumerate() {
                r[k] = n >= *goal;
            }
        }
        Kind::Heisman => r = super::guys::heisman_steps(w, a),
        Kind::LatePick => r = super::guys::late_steps(w, a),
        Kind::ProBowlers => {
            let n = super::guys::pro_bowlers(w) as i32 - a.base;
            for (k, x) in r.iter_mut().enumerate() {
                *x = n >= 2 * (k as i32 + 1);
            }
        }
        Kind::Years => {
            let n = years_at(w, a.team);
            let goals: [u32; 5] = if level == Level::HighSchool { [5, 10, 15, 18, 20] } else { [5, 8, 10, 15, 20] };
            for (k, g) in goals.iter().enumerate() {
                r[k] = n >= *g;
            }
        }
        Kind::Wins => {
            let you = crate::world::stories::seat_coach(w).unwrap_or(0);
            let n = crate::world::storylines::detect::career_wins(w, you);
            for (k, s) in a.steps.iter().enumerate().take(5) {
                r[k] = s.label.parse::<u32>().is_ok_and(|g| n >= g);
            }
        }
        Kind::Building => {
            r = match a.team {
                TeamKey::Hs(_) => super::hs::bond_steps(w),
                TeamKey::College(p) => super::college::building_steps(w, p as usize),
                _ => r,
            }
        }
        Kind::Crowds => {
            if let TeamKey::Nfl(t) = a.team {
                r = super::nfl::crowd_steps(w, t as usize, a.set.map_or(w.year, |d| d.year()));
            }
        }
        Kind::Son => r = super::hs::son_steps(w, a),
        Kind::Home => {
            let rival = crate::world::folk::facts::rival(w, crate::world::storylines::folk_team(a.team))
                .map(|x| crate::world::storylines::team_key(x.0));
            let beat_rival = rival.is_some_and(|rt| here.iter().any(|g| g.opp == rt && g.won()));
            r = [
                any(&|s| s.wins > s.losses),
                beat_rival,
                any(&|s| if level == Level::College { s.post > 0 } else { s.playoff > 0 }),
                any(&|s| s.title),
                match level {
                    Level::College => any(&|s| s.playoff > 0),
                    _ => any(&|s| s.champion),
                },
            ];
        }
    }
    r
}

/// The rounds of a high school's state bracket in its class.
fn hs_rounds(w: &World, h: usize) -> usize {
    let s = &w.high_schools[h];
    let f = crate::world::hs_playoffs::format(&w.states[s.state].code);
    crate::world::hs_playoffs::rounds_for(f.field_of(s.classification as usize) as usize)
}

/// Seasons you have coached a team (the one under way counts once it has
/// a game).
pub fn years_at(w: &World, t: TeamKey) -> u32 {
    let mut years: Vec<i32> = super::seasons_at(w, t).iter().map(|s| s.year).collect();
    // Seasons before the career, when it took over a coach already in the
    // job (hired before the season the career began in; a new hire's
    // first season is the one he coaches).
    if let Some(c) = w.career.as_ref() {
        let co = &w.coaches[c.coach];
        let start = if c.started.month() <= 2 { c.started.year() - 1 } else { c.started.year() };
        if let Some(j) = co.job
            && super::team_of_employer(j.employer) == Some(t)
            && j.since < start
        {
            for y in j.since..=start {
                years.push(y);
            }
        }
    }
    years.sort_unstable();
    years.dedup();
    years.len() as u32
}

/// The world moved: steps reached pay, ambitions close.
pub(crate) fn check(w: &mut World) {
    // A picker put off too long keeps what you picked.
    if let Some(p) = &w.legacy.picker {
        let lapse = p.later.unwrap_or(p.date.add(LATER_DAYS + 30));
        if w.today >= lapse {
            close_picker(w, true);
        }
    }
    // New ones when life changes: your son coming up to your school.
    if w.legacy.picker.is_none()
        && let Some((k, _)) = son_coming(w)
        && !w.legacy.ambitions.iter().any(|a| a.kind == Kind::Son && a.subject == k as u32)
        && let Some((_, team)) = super::you(w)
    {
        let list: Vec<Offer> = offers_for(w, team).into_iter().filter(|o| o.kind == Kind::Son).collect();
        if !list.is_empty() {
            let hard = 0.0;
            let mut ids = Vec::new();
            for o in list {
                let id = w.legacy.next_id;
                w.legacy.next_id += 1;
                w.legacy.ambitions.push(Ambition {
                    id,
                    kind: o.kind,
                    team,
                    group: o.group.to_string(),
                    title: o.title,
                    why: o.why,
                    steps: owned(&o.labels, &o.diffs, hard),
                    closes: o.closes.to_string(),
                    into: o.into,
                    loud: false,
                    picked: false,
                    offered: w.today,
                    set: None,
                    closed: None,
                    done: None,
                    subject: o.subject,
                    base: 0,
                    hard,
                    story: None,
                });
                ids.push(id);
            }
            w.legacy.picker = Some(Picker {
                team,
                date: w.today,
                offers: ids,
                start: "your life changed: a new ambition".into(),
                hard,
                later: None,
                how: "Life changes".into(),
            });
            super::memo(
                w,
                "legacy: ambitions",
                "A new ambition".into(),
                vec![
                    "Your son is coming up to your school. Coaching him is a chance most coaches never get.".into(),
                    "[[s:legacy_ambitions|What do you want here?]]".into(),
                ],
            );
        }
    }
    let ids: Vec<u32> = w.legacy.ambitions.iter().filter(|a| a.open()).map(|a| a.id).collect();
    for id in ids {
        let Some(a) = w.legacy.ambitions.iter().find(|a| a.id == id).cloned() else { continue };
        let r = reached(w, &a);
        for k in 0..a.steps.len().min(5) {
            if r[k] && a.steps[k].done.is_none() {
                let lesson = a.pay_of(k);
                let rep = a.rep_of(k);
                let what = format!("{}: {}", a.title, a.steps[k].label);
                super::pay(w, super::key("G1", a.id as u64, k as u64), "G1", what, lesson, a.into, rep);
                let today = w.today;
                if let Some(x) = w.legacy.ambitions.iter_mut().find(|x| x.id == id) {
                    x.steps[k].done = Some(today);
                }
            }
        }
        let all =
            w.legacy.ambitions.iter().find(|x| x.id == id).is_some_and(|x| x.steps.iter().all(|s| s.done.is_some()));
        if all {
            finish(w, id);
            continue;
        }
        if let Some(why) = closing(w, &a) {
            close(w, id, why);
        }
    }
}

/// Every step done: the ambition is met.
fn finish(w: &mut World, id: u32) {
    let today = w.today;
    let Some(a) = w.legacy.ambitions.iter_mut().find(|a| a.id == id) else { return };
    a.done = Some(today);
    let (title, story, loud) = (a.title.clone(), a.story, a.loud);
    if let Some(s) = story {
        crate::world::storylines::close(w, s, "delivered", Some(format!("{title}: done.")));
    }
    if loud {
        crate::world::stories::story::memento(
            w,
            crate::world::stories::story::MementoKind::Plaque,
            format!("{title}: said out loud, and done"),
        );
    }
    super::memo(w, "legacy: ambition met", format!("Done: {title}"), vec![format!("{title}. Every step of it.")]);
}

/// Why an open ambition closes for good, if it does now.
fn closing(w: &World, a: &Ambition) -> Option<String> {
    let here = super::you(w).map(|x| x.1);
    let set_year = a.set.map_or(w.year, |d| d.year());
    let gone = here != Some(a.team);
    match a.kind {
        Kind::Years | Kind::Home | Kind::Building | Kind::Crowds if gone => Some("You left.".into()),
        Kind::Turnaround if gone => Some("You left before it was done.".into()),
        Kind::Turnaround => {
            let seasons = super::seasons_at(w, a.team).iter().filter(|s| s.year >= set_year).count();
            (seasons >= 5 && super::review_done(w)).then(|| "Five seasons ran out.".into())
        }
        Kind::Beat => {
            let c = w.coaches.get(a.subject as usize)?;
            if c.retired {
                return Some(format!("{} retired first.", c.name));
            }
            if let TeamKey::Nfl(t) = a.team {
                let div = crate::world::franchise::era(t as usize, w.year)?.div;
                let his = match c.job.map(|j| (j.employer, j.role)) {
                    Some((Employer::Nfl(o), Role::HeadCoach)) => crate::world::franchise::era(o, w.year).map(|e| e.div),
                    _ => None,
                };
                if his != Some(div) && c.job.is_some_and(|j| j.since < w.year || his.is_none()) {
                    return Some(format!("{} left the division.", c.name));
                }
            }
            None
        }
        Kind::Reach => match a.team {
            TeamKey::Hs(_) => {
                (super::hs::jump_declined(w, a.set.unwrap_or(w.today)) >= 2).then(|| "Two offers turned down.".into())
            }
            TeamKey::College(_) => (years_at(w, a.team) >= 10 && !gone).then(|| "Ten years here.".into()),
            TeamKey::Nfl(_) if gone => Some("You were let go.".into()),
            _ => None,
        },
        Kind::Building => match a.team {
            TeamKey::Hs(_) => (w.legacy.hs.bond_fails >= 2).then(|| "The bond failed twice.".into()),
            TeamKey::College(p) => {
                super::college::building_shelved(w, p as usize).then(|| "The plan was shelved.".into())
            }
            _ => None,
        },
        Kind::Son => super::hs::son_gone(w, a).then(|| "He plays somewhere else.".into()),
        _ => None,
    }
}

/// Closes an ambition for good: a public one costs reputation.
fn close(w: &mut World, id: u32, why: String) {
    let today = w.today;
    let Some(a) = w.legacy.ambitions.iter_mut().find(|a| a.id == id) else { return };
    a.closed = Some((today, why.clone()));
    let (title, loud, story) = (a.title.clone(), a.loud, a.story);
    if loud {
        super::cost(w, super::key("G1x", id as u64, 0), "G1", format!("{title} closed: {why}"), 0.3);
    }
    if let Some(s) = story {
        crate::world::storylines::close(w, s, "fell_short", Some(format!("{title}: {why}")));
    }
}

/// After a season: an out-loud ambition with no step this year lets the
/// press and fans ask about it (its story hears it); nothing else here.
pub(crate) fn season(w: &mut World) {
    let year = w.year;
    let quiet: Vec<(u32, String)> = w
        .legacy
        .ambitions
        .iter()
        .filter(|a| a.open() && a.loud)
        .filter(|a| !a.steps.iter().any(|s| s.done.is_some_and(|d| d.year() == year)))
        .filter_map(|a| a.story.map(|s| (s, a.title.clone())))
        .collect();
    for (s, title) in quiet {
        crate::world::storylines::fact(w, s, format!("Another season, and {title} is still a promise."), 12.0);
    }
}

// ---------------------------------------------------------------------------
// The others' goals on your desk (board 35's "On your desk too")
// ---------------------------------------------------------------------------

/// A goal someone else set for you.
#[derive(Clone, Debug)]
pub struct Other {
    /// "AD", "Principal", "Owner", "Board", "Family", "Fans".
    pub who: String,
    pub text: String,
    /// "set Dec 1", "met", "fits", "1-7 since 2016".
    pub status: String,
    /// The status reads badly (a losing record, a missed goal).
    pub warn: bool,
}

/// Your boss's, family's and fans' goals as they stand.
pub fn others(w: &World) -> Vec<Other> {
    let mut out = Vec::new();
    let Some((_, team)) = super::you(w) else { return out };
    let ft = crate::world::storylines::folk_team(team);
    // The boss: the season's main goal.
    let who = match team {
        TeamKey::Hs(_) => "Principal",
        TeamKey::Nfl(_) => "Owner",
        _ => "AD",
    };
    let goals: Vec<crate::world::folk::boss::Goal> = match team {
        TeamKey::Hs(_) => crate::world::front_office::school::principal_goals(w).1,
        _ => w.folk.goals.clone(),
    };
    if let Some(g) = goals.iter().filter(|g| g.season >= w.year - 1).max_by_key(|g| (g.season, g.set_on)) {
        let text = cap(&g.kind.words(team.level()));
        let text = format!("{text} in {}", g.season);
        let (status, warn) = match g.met {
            Some(true) => ("met".to_string(), false),
            Some(false) => ("missed".to_string(), true),
            None => (format!("set {}", short_day(g.set_on)), false),
        };
        out.push(Other { who: who.into(), text, status, warn });
    }
    // The family: a child finishing high school, or the household settling.
    if let Some(c) = w.career.as_ref() {
        let town = town_of(w, team);
        let kid = c.you.profile.children.iter().filter(|k| (10..=17).contains(&(w.year - k.1))).max_by_key(|k| k.1);
        let fits = c.family.mood >= 0.3;
        if let Some((n, b)) = kid {
            let first = n.split_whitespace().next().unwrap_or(n);
            out.push(Other {
                who: "Family".into(),
                text: format!("Stay until {first} graduates in {}", b + 18),
                status: if fits { "fits".into() } else { "strained".into() },
                warn: !fits,
            });
        } else if c.you.profile.status == "Married" {
            out.push(Other {
                who: "Family".into(),
                text: format!("Stay in {town}"),
                status: if fits { "fits".into() } else { "strained".into() },
                warn: !fits,
            });
        }
    }
    // The fans: beat the rival.
    if let Some((r, _, ours, theirs)) = crate::world::folk::facts::rival(w, ft) {
        let rt = crate::world::storylines::team_key(r);
        let rname = crate::world::stories::short_name(w, rt);
        let (wn, l, _) = super::record_vs(w, Some(rt), None);
        let start = w.legacy.games.iter().filter(|g| g.team == team && g.opp == rt).map(|g| g.season).min();
        let (status, warn) = match start {
            Some(y) => (format!("{wn}-{l} since {y}"), wn < l),
            None if ours + theirs > 0 => (format!("{ours}-{theirs} all-time"), ours < theirs),
            None => ("first meeting ahead".into(), false),
        };
        let every = if wn > 0 && l == 0 { "again" } else { "every year" };
        out.push(Other { who: "Fans".into(), text: format!("Beat {rname} {every}"), status, warn });
    }
    out
}

fn short_day(d: Date) -> String {
    let (_, m, day) = d.parts();
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!("{} {day}", M[(m as usize).saturating_sub(1) % 12])
}

/// The town a team plays in.
pub fn town_of(w: &World, t: TeamKey) -> String {
    match t {
        TeamKey::Hs(h) => w.high_schools[h as usize].town.clone(),
        TeamKey::College(p) => w.programs[p as usize].city.clone(),
        TeamKey::Nfl(n) => {
            crate::world::franchise::era(n as usize, w.year).map_or_else(String::new, |e| e.city.to_string())
        }
        TeamKey::Other(_) => String::new(),
    }
}

/// The Rating banks an ambition can pay into, by kind (for the screens).
pub fn into_words(a: &Ambition) -> String {
    a.into.name()
}

/// Your open ambitions, newest last.
pub fn mine(w: &World) -> Vec<&Ambition> {
    w.legacy.ambitions.iter().filter(|a| a.open()).collect()
}

// ---------------------------------------------------------------------------
// The AI coaches' own ambitions (G5: "the AI has them too")
// ---------------------------------------------------------------------------

/// What an AI coach's own ambitions add to an offer, with the words for
/// why he moved: the college he started at (most coaches begin where they
/// played or were a graduate assistant; about six in ten carry it as a
/// dream job), a shot back at the conference of the school that fired him,
/// and the settled coach who means to retire where he is. Sizes are
/// Claude's picks (proposed, not confirmed): +0.035 the first school, +0.015
/// a revenge shot, -0.04 a coach who won't leave (loyalty 14+, ten years in).
pub(crate) fn ai_pulls(w: &World, c: usize, to: Employer, now: crate::world::coach::Job) -> Vec<(f64, String)> {
    let co = &w.coaches[c];
    let mut out = Vec::new();
    let seed = w.options.seed as u64 ^ 0xA1_3B;
    // The first college he worked at.
    let first = co.history.iter().find_map(|s| match s.employer {
        Employer::College(p) => Some(p),
        _ => None,
    });
    if let (Employer::College(p), Some(f)) = (to, first)
        && p == f
        && !matches!(now.employer, Employer::College(q) if q == p)
        && crate::world::season::key_uniform(seed, c as u64) < 0.6
    {
        out.push((0.035, format!("a homecoming to {}, where he started", w.program_name(p))));
    }
    // The last school that fired him: a job in its conference.
    if let Some(Employer::College(f)) =
        co.history.iter().rev().find(|s| s.ended.starts_with("fired")).map(|s| s.employer)
        && let Employer::College(p) = to
        && p != f
        && w.programs[p].conference == w.programs[f].conference
        && w.programs[p].division == w.programs[f].division
    {
        out.push((0.015, format!("a shot at {} twice a season", w.program_name(f))));
    }
    // A coach who means to retire here.
    if now.role == Role::HeadCoach && co.personality.loyalty >= 14.0 && w.year - now.since >= 10 {
        out.push((-0.04, String::new()));
    }
    out
}
