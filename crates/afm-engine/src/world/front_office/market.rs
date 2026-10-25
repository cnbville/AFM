//! Your side of the coaching market (lane 10; first-playable topic 2,
//! people topics 2, 13, 14 and 15): starting without a job, assistant
//! jobs, offers that come to you, the people from the world's history who
//! know you, and your family, who move with you or don't.
//!
//! Teams judge you as they judge every candidate: the built search scores
//! you with the same formula, from your own keyed stream, plus what a
//! committee really weighs about a person (someone on the staff knows
//! you, you're from here). When you'd be the hire the job waits for your
//! answer, as a real offer does, and goes to the next name if you say no.

use crate::calendar::{Date, Event, Kind, Level, Loudness};
use crate::positions::Position;
use crate::rng::{Rng, clamp};
use crate::world::coach::{Employer, Group, Role};
use crate::world::{Seat, World};

use super::key;

const SALT: u64 = 0x3A2C_0000;

/// Which jobs you let it be known you want (your agent and your network
/// put your name in), and whether you listen while you have a job.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Interest {
    /// While employed: take calls about other jobs.
    pub listening: bool,
    pub hs_head: bool,
    pub college_head: bool,
    pub coordinator: bool,
    pub position: bool,
    pub nfl: bool,
    /// Head and coordinator jobs in the other pro leagues (the CFL, arena
    /// football, the spring leagues).
    pub other: bool,
    /// Only jobs in these states (empty: anywhere). NFL and other-league
    /// jobs ignore it.
    pub states: Vec<usize>,
    /// The lowest college level you'd coach at: 0 any division, 1 FCS and
    /// up, 2 FBS only (an agent shops a man where his résumé belongs).
    pub college_floor: u8,
}

impl Default for Interest {
    fn default() -> Interest {
        Interest {
            listening: false,
            hs_head: true,
            college_head: true,
            coordinator: true,
            position: true,
            nfl: true,
            other: true,
            states: Vec::new(),
            college_floor: 0,
        }
    }
}

/// A college's rung for the floor: FBS 2, FCS 1, the rest 0.
fn college_rung(w: &World, p: usize) -> u8 {
    match w.programs[p].division {
        crate::world::data::Division::FBS => 2,
        crate::world::data::Division::FCS => 1,
        _ => 0,
    }
}

/// A job offered to you, waiting for your answer.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Offer {
    pub employer: Employer,
    pub role: Role,
    pub prestige: f64,
    pub made: Date,
    pub until: Date,
    /// Pay a year, in dollars.
    pub pay: f64,
    pub years: u8,
    /// Why they want you, in their words.
    pub why: String,
    /// What it means for your family, in words.
    pub family: String,
}

/// Your market: what you want, your applications, offers, the jobs you
/// turned down or left (not offered again for 60 days), and its story.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Market {
    pub interest: Interest,
    pub applied: Vec<(Employer, Role, Date)>,
    pub offers: Vec<Offer>,
    pub passed: Vec<(Employer, Role, Date)>,
    /// Out of coaching since.
    pub out_since: Option<Date>,
    /// The day you took the job you hold (None: the career's first job,
    /// taken the day it started).
    pub job_since: Option<Date>,
    /// A high school that didn't renew your coaching stipend: you still teach there.
    pub teaching_at: Option<usize>,
    pub log: Vec<(Date, String)>,
}

/// Someone from the world's history who knows you (people topic 2): who,
/// how, when, and how close (0-1).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Known {
    pub coach: usize,
    pub how: String,
    pub from: i32,
    pub to: i32,
    pub strength: f32,
}

/// Your family's life (people topic 13): how the household feels about
/// where you are, where you live, since when, how many times you moved
/// them, and what happened.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct FamilyLife {
    /// 0-1, hidden: the household's contentment.
    pub mood: f32,
    pub state: Option<usize>,
    pub since: i32,
    pub moves: u32,
    pub log: Vec<(Date, String)>,
    /// Yearly notes already written: (year, kind).
    pub told: Vec<(i32, u8)>,
}

/// A state's name from its postal code.
pub fn state_name(code: &str) -> &'static str {
    const S: [(&str, &str); 51] = [
        ("AL", "Alabama"),
        ("AK", "Alaska"),
        ("AZ", "Arizona"),
        ("AR", "Arkansas"),
        ("CA", "California"),
        ("CO", "Colorado"),
        ("CT", "Connecticut"),
        ("DE", "Delaware"),
        ("DC", "Washington, D.C."),
        ("FL", "Florida"),
        ("GA", "Georgia"),
        ("HI", "Hawaii"),
        ("ID", "Idaho"),
        ("IL", "Illinois"),
        ("IN", "Indiana"),
        ("IA", "Iowa"),
        ("KS", "Kansas"),
        ("KY", "Kentucky"),
        ("LA", "Louisiana"),
        ("ME", "Maine"),
        ("MD", "Maryland"),
        ("MA", "Massachusetts"),
        ("MI", "Michigan"),
        ("MN", "Minnesota"),
        ("MS", "Mississippi"),
        ("MO", "Missouri"),
        ("MT", "Montana"),
        ("NE", "Nebraska"),
        ("NV", "Nevada"),
        ("NH", "New Hampshire"),
        ("NJ", "New Jersey"),
        ("NM", "New Mexico"),
        ("NY", "New York"),
        ("NC", "North Carolina"),
        ("ND", "North Dakota"),
        ("OH", "Ohio"),
        ("OK", "Oklahoma"),
        ("OR", "Oregon"),
        ("PA", "Pennsylvania"),
        ("RI", "Rhode Island"),
        ("SC", "South Carolina"),
        ("SD", "South Dakota"),
        ("TN", "Tennessee"),
        ("TX", "Texas"),
        ("UT", "Utah"),
        ("VT", "Vermont"),
        ("VA", "Virginia"),
        ("WA", "Washington"),
        ("WV", "West Virginia"),
        ("WI", "Wisconsin"),
        ("WY", "Wyoming"),
    ];
    S.iter().find(|x| x.0 == code).map_or("another state", |x| x.1)
}

// ---------------------------------------------------------------------------
// Who you are in the world
// ---------------------------------------------------------------------------

/// Your coach id.
pub fn me(w: &World) -> Option<usize> {
    w.career.as_ref().map(|c| c.coach)
}

pub(crate) fn is_you(w: &World, c: usize) -> bool {
    me(w) == Some(c)
}

/// Your job now, if you have one.
pub fn your_job(w: &World) -> Option<crate::world::coach::Job> {
    me(w).and_then(|c| w.coaches.get(c)).and_then(|c| c.job)
}

pub(crate) fn state_of(w: &World, e: Employer) -> Option<usize> {
    match e {
        Employer::HighSchool(s) => Some(w.high_schools[s].state),
        Employer::College(p) => Some(w.programs[p].state),
        Employer::Nfl(t) => {
            w.nfl.get(t)?;
            let code = crate::world::franchise::era(t, w.year)?.state;
            w.states.iter().position(|s| *s.code == *code)
        }
        Employer::Other(_) => None,
    }
}

/// An employer's name.
pub fn employer_name(w: &World, e: Employer) -> String {
    match e {
        Employer::HighSchool(s) => w.school_name(s).to_string(),
        Employer::College(p) => w.program_name(p).to_string(),
        Employer::Nfl(t) => w.nfl_team_name(t),
        Employer::Other(f) => crate::world::pro::other::name(f, w.year).to_string(),
    }
}

/// A role in a title's words ("Offensive coordinator").
pub fn role_title(r: Role) -> String {
    let n = r.name();
    n[..1].to_uppercase() + &n[1..]
}

fn post(w: &mut World, d: Date, level: Level, text: String, decision: bool) {
    let mut e = Event::new(
        d,
        level,
        if decision { Kind::Deadline } else { Kind::News },
        if decision { Loudness::Medium } else { Loudness::Small },
        text,
    )
    .rule("Your career");
    if decision {
        e = e.decision();
    }
    w.calendar.post(e);
}

fn level_of(e: Employer) -> Level {
    match e {
        Employer::HighSchool(_) => Level::HighSchool,
        Employer::College(_) => Level::College,
        Employer::Nfl(_) => Level::Nfl,
        Employer::Other(_) => Level::Other,
    }
}

fn log(w: &mut World, text: String) {
    let d = w.today;
    if let Some(c) = w.career.as_mut() {
        c.market.log.push((d, text));
        if c.market.log.len() > 200 {
            c.market.log.remove(0);
        }
    }
}

// ---------------------------------------------------------------------------
// Your candidacy
// ---------------------------------------------------------------------------

/// Whether a job is the kind you let it be known you want.
pub fn interest_fits(w: &World, i: &Interest, e: Employer, r: Role) -> bool {
    if let Employer::College(p) = e
        && college_rung(w, p) < i.college_floor
    {
        return false;
    }
    let kind = match (e, r) {
        (Employer::Nfl(_), _) => i.nfl,
        (Employer::HighSchool(_), Role::HeadCoach) => i.hs_head,
        (Employer::HighSchool(_), _) => false,
        (Employer::College(_), Role::HeadCoach) => i.college_head,
        (Employer::College(_), Role::OffensiveCoordinator | Role::DefensiveCoordinator) => i.coordinator,
        (Employer::College(_), Role::PositionCoach(_)) => i.position,
        // The other leagues' clubs hire a head coach and two coordinators.
        (Employer::Other(_), Role::PositionCoach(_)) => false,
        (Employer::Other(_), _) => i.other,
    };
    let place = matches!(e, Employer::Nfl(_) | Employer::Other(_))
        || i.states.is_empty()
        || state_of(w, e).is_some_and(|s| i.states.contains(&s));
    kind && place
}

/// Whether a search should consider you: you applied, or the job is the
/// kind you let it be known you want (out of work, or listening).
pub(crate) fn candidate(w: &World, e: Employer, r: Role) -> bool {
    let Some(c) = w.career.as_ref() else { return false };
    let today = w.today;
    if w.coaches[c.coach].retired {
        return false;
    }
    let job = your_job(w);
    if job.is_some_and(|j| j.employer == e && j.role == r) {
        return false;
    }
    let m = &c.market;
    if m.passed.iter().any(|x| x.0 == e && x.1 == r && x.2.days_to(today) < 60) {
        return false;
    }
    // Three offers on the table: a coach weighing them is off the market
    // until he answers (Claude's pick: no committee calls a man who is
    // deciding between three jobs).
    if m.offers.len() >= 3 || m.offers.iter().any(|o| o.employer == e && o.role == r) {
        return false;
    }
    if m.applied.iter().any(|x| x.0 == e && x.1 == r) {
        return true;
    }
    (job.is_none() || m.interest.listening) && interest_fits(w, &m.interest, e, r)
}

/// What a job pays a year, in words ("about $1.2M a year"): what its
/// search offers a new man.
pub fn pay_words(w: &World, e: Employer, r: Role) -> String {
    format!("about {} a year", super::dollars(super::super::staff::pay_for(w, e, r)))
}

/// Whether a job would still reach you (a promotion from your own staff
/// comes whatever you said you want): not one you just turned down, and
/// not one already waiting for your answer.
pub(crate) fn would_hear(w: &World, e: Employer, r: Role) -> bool {
    let Some(c) = w.career.as_ref() else { return false };
    let today = w.today;
    !c.market.passed.iter().any(|x| x.0 == e && x.1 == r && x.2.days_to(today) < 60)
        && !c.market.offers.iter().any(|o| o.employer == e && o.role == r)
}

/// Whether a job is waiting for your answer.
pub(crate) fn offered(w: &World, e: Employer, r: Role) -> bool {
    w.career.as_ref().is_some_and(|c| c.market.offers.iter().any(|o| o.employer == e && o.role == r))
}

/// A job you left or turned down: it isn't offered to you again for 60 days.
pub(crate) fn passed_on(w: &mut World, e: Employer, r: Role) {
    let today = w.today;
    if let Some(c) = w.career.as_mut() {
        c.market.passed.push((e, r, today));
        c.market.passed.retain(|x| x.2.days_to(today) < 400);
    }
}

/// How well a coach knows you (0-1), if he does.
pub(crate) fn known_by(w: &World, coach: usize) -> Option<f64> {
    w.career.as_ref()?.known.iter().find(|k| k.coach == coach).map(|k| k.strength as f64)
}

/// What a committee weighs about you as a person besides the built score:
/// someone on the hiring staff who knows you (as a head coach hires from
/// his tree), a man from the state or the school itself, and how you come
/// across in an interview (Aura) for a head job. Proposed, not confirmed:
/// the sizes.
pub(crate) fn bonus(w: &World, e: Employer, r: Role) -> f64 {
    let Some(c) = w.career.as_ref() else { return 0.0 };
    let hc = match e {
        Employer::College(p) => w.programs[p].head_coach,
        Employer::Nfl(t) => w.nfl[t].head_coach,
        Employer::Other(f) => crate::world::pro::other::head_coach(w, f),
        Employer::HighSchool(_) => None,
    };
    let mut b = 0.0;
    if r != Role::HeadCoach
        && let Some(h) = hc
        && let Some(k) = c.known.iter().find(|k| k.coach == h)
    {
        b += 1.0 + 1.5 * k.strength as f64;
    }
    if state_of(w, e) == Some(c.you.home_state) {
        b += 0.5;
    }
    let local = match e {
        Employer::HighSchool(s) => c.you.profile.high_school.trim().eq_ignore_ascii_case(w.school_name(s)),
        Employer::College(p) => c.you.profile.alma_mater.trim().eq_ignore_ascii_case(w.program_name(p)),
        Employer::Nfl(_) | Employer::Other(_) => false,
    };
    if local {
        b += 0.6;
    }
    if r == Role::HeadCoach {
        b += 0.08 * (c.you.aura() - 10.0);
    }
    b
}

/// Your own stream for a search's reads of you.
pub(crate) fn rng(w: &World, e: Employer, r: Role) -> Rng {
    let k = match e {
        Employer::HighSchool(s) => s as u64,
        Employer::College(p) => 1 << 40 | p as u64,
        Employer::Nfl(t) => 2 << 40 | t as u64,
        Employer::Other(f) => 3 << 40 | f as u64,
    };
    let role = match r {
        Role::HeadCoach => 0,
        Role::OffensiveCoordinator => 1,
        Role::DefensiveCoordinator => 2,
        Role::PositionCoach(g) => 3 + g as u64,
    };
    Rng::new((key(w, SALT ^ 1 ^ (w.today.days_to(Date::ymd(2000, 1, 1)) as u64) << 8, k << 4 | role) * 4.0e9) as u32)
}

/// What a family thinks of a job, in words (people topic 13): a child's
/// last years of high school make a move hardest, a settled household
/// wants to stay, and going home pulls.
pub fn family_words(w: &World, e: Employer) -> String {
    let Some(c) = w.career.as_ref() else { return String::new() };
    let p = &c.you.profile;
    let to = state_of(w, e);
    let here = c.family.state;
    if to.is_some() && to == here {
        return "No move: the family stays where it is.".into();
    }
    // Most coaches in the other leagues keep the family home and go for
    // the season (a CFL camp to the Grey Cup is about six months).
    if let Employer::Other(f) = e {
        return format!(
            "The family can stay home: you'd be in {} for the season and back between seasons.",
            crate::world::pro::other::city(f, w.year)
        );
    }
    let year = w.year;
    let mut parts: Vec<String> = Vec::new();
    for (name, born) in &p.children {
        let age = year - born;
        if (15..=17).contains(&age) {
            let grade = ["sophomore", "junior", "senior"][(age - 15) as usize];
            parts.push(format!("{name} is a {grade} in high school: moving now is the hardest move there is"));
        } else if age == 14 {
            parts.push(format!("{name} starts high school in the fall"));
        }
    }
    if p.status == "Married" && !p.spouse.is_empty() {
        let settled = year - c.family.since;
        if settled >= 4 {
            parts.push(format!("{} has built a life here in {settled} years", p.spouse));
        } else if c.family.mood < 0.4 {
            parts.push(format!("{} hasn't settled since the last move", p.spouse));
        }
    }
    if to == Some(c.you.home_state) {
        parts.push(format!(
            "It's home: {} is where your roots are",
            w.states.get(c.you.home_state).map_or("your home state", |s| state_name(&s.code))
        ));
    }
    if parts.is_empty() {
        "A move, but an easy one for the family.".into()
    } else {
        let mut s = parts.join("; ");
        s.push('.');
        s[..1].to_uppercase() + &s[1..]
    }
}

/// You are the search's pick: the job waits for your answer (three days,
/// five for a high school, whose board meets weekly).
pub(crate) fn offer(w: &mut World, e: Employer, r: Role, prestige: f64) {
    let today = w.today;
    let until = today.add(if matches!(e, Employer::HighSchool(_)) { 5 } else { 3 });
    let pay = super::super::staff::pay_for(w, e, r);
    let years = match (e, r) {
        (Employer::HighSchool(_), _) => 1,
        (Employer::College(_), Role::HeadCoach) => 5,
        (Employer::College(_), Role::OffensiveCoordinator | Role::DefensiveCoordinator) => 3,
        (Employer::College(_), _) => 2,
        (Employer::Nfl(_), Role::HeadCoach) => 5,
        (Employer::Nfl(_), _) => 3,
        (Employer::Other(_), Role::HeadCoach) => 3,
        (Employer::Other(_), _) => 2,
    };
    // A head job that is volatile now pays for it (update 2, S10): a
    // guaranteed year more, and a bigger buyout when you sign.
    let hazard = if r == Role::HeadCoach { crate::world::storylines::credit::hazard(w, e) } else { None };
    let years = years + hazard.map_or(0, |h| h.0);
    let Some(c) = w.career.as_ref() else { return };
    let hc = match e {
        Employer::College(p) => w.programs[p].head_coach,
        Employer::Nfl(t) => w.nfl[t].head_coach,
        Employer::Other(f) => crate::world::pro::other::head_coach(w, f),
        Employer::HighSchool(_) => None,
    };
    let mut why: Vec<String> = Vec::new();
    if let Some(h) = hc
        && r != Role::HeadCoach
        && let Some(k) = c.known.iter().find(|k| k.coach == h)
    {
        why.push(format!("{} knows you: {}", w.coaches[h].name, k.how.to_lowercase()));
    }
    if state_of(w, e) == Some(c.you.home_state) {
        why.push("they like that you're from the state".into());
    }
    // What the staff says it wants from you, from the job and your ratings
    // as people around football see them (your strongest side for the job).
    let y = &c.you;
    why.push(match r {
        Role::HeadCoach => "they want your plan for the program".into(),
        Role::PositionCoach(g) if y.teaching[g as usize] >= 13.0 => {
            format!("they like how you teach {}", g.name())
        }
        Role::PositionCoach(_) if y.recruiting >= 13.0 => "they want a position coach who recruits".into(),
        Role::PositionCoach(g) => format!("they need a {} and your name came up", g.coach_title()),
        _ if y.scheme >= 13.0 => "they want your scheme".into(),
        _ if y.recruiting >= 13.0 => "they want a coordinator who recruits".into(),
        _ => format!("they need an {} and your name came up", r.name()),
    });
    if let Some((_, share)) = hazard {
        why.push(format!(
            "the job is volatile right now, so they add a guaranteed year and a buyout {:.0}% bigger",
            share * 100.0
        ));
    }
    let family = family_words(w, e);
    let name = employer_name(w, e);
    let title = role_title(r);
    if let Some(k) = w.career.as_mut() {
        k.market.offers.push(Offer {
            employer: e,
            role: r,
            prestige,
            made: today,
            until,
            pay,
            years,
            why: {
                let s = why.join("; ");
                s[..1].to_uppercase() + &s[1..] + "."
            },
            family,
        });
        k.market.applied.retain(|x| !(x.0 == e && x.1 == r));
    }
    log(w, format!("Offered: {title}, {name}."));
    post(
        w,
        today,
        level_of(e),
        format!(
            "{name} offers you the {} job: {} a year for {years} year{}. Answer by {}.",
            r.name(),
            super::dollars(pay),
            if years == 1 { "" } else { "s" },
            until.pretty()
        ),
        true,
    );
}

/// Takes an offer: you leave your job (if any) and start the new one.
pub fn accept(w: &mut World, k: usize) -> Result<String, String> {
    let c = w.career.as_ref().ok_or("No career.")?;
    let o = c.market.offers.get(k).cloned().ok_or("That offer is gone.")?;
    if !super::super::staff::open_slot(w, o.employer, o.role) {
        if let Some(c) = w.career.as_mut() {
            c.market.offers.remove(k);
        }
        settled(w, &o);
        return Err("They filled the job before you answered.".into());
    }
    let rest: Vec<Offer> = w.career.as_mut().map(|c| std::mem::take(&mut c.market.offers)).unwrap_or_default();
    settled(w, &o);
    super::super::staff::take_job(w, o.employer, o.role);
    // Every other offer is turned down, and its job goes to the next name.
    for x in rest.into_iter().filter(|x| !(x.employer == o.employer && x.role == o.role)) {
        pass(w, &x);
    }
    let name = employer_name(w, o.employer);
    log(w, format!("Took the {} job at {name}.", o.role.name()));
    Ok(format!("You're the new {} at {name}.", o.role.name()))
}

fn pass(w: &mut World, o: &Offer) {
    settled(w, o);
    passed_on(w, o.employer, o.role);
    super::super::staff::fill_now(w, o.employer, o.role, o.prestige);
}

/// An answered or lapsed offer no longer waits on you: its inbox item
/// stays as news but leaves the pinned list.
fn settled(w: &mut World, o: &Offer) {
    let head = format!("{} offers you the {} job", employer_name(w, o.employer), o.role.name());
    if let Some(evs) = w.calendar.events.get_mut(&o.made) {
        for e in evs.iter_mut().filter(|e| e.needs_decision && e.rule == "Your career" && e.text.starts_with(&head)) {
            e.needs_decision = false;
        }
    }
}

/// Turns an offer down: the job goes to the next name on the list.
pub fn decline(w: &mut World, k: usize) -> Result<String, String> {
    let c = w.career.as_mut().ok_or("No career.")?;
    if k >= c.market.offers.len() {
        return Err("That offer is gone.".into());
    }
    let o = c.market.offers.remove(k);
    let name = employer_name(w, o.employer);
    log(w, format!("Turned down the {} job at {name}.", o.role.name()));
    pass(w, &o);
    let who = super::super::staff::holder(w, o.employer, o.role).map(|h| w.coaches[h].name.clone());
    Ok(match who {
        Some(n) => format!("You turned it down. {name} hired {n} instead."),
        None => format!("You turned it down. {name} keeps looking."),
    })
}

/// A job open now: a college search under way, or a seat nobody holds.
#[derive(Clone, Debug)]
pub struct OpenJob {
    pub employer: Employer,
    pub role: Role,
    /// The day a college search decides.
    pub decides: Option<Date>,
    pub applied: bool,
}

/// Every job open today (people topic 14: openings come from the built
/// carousel): college searches under way and seats nobody holds.
pub fn openings(w: &World) -> Vec<OpenJob> {
    let applied =
        |e: Employer, r: Role| w.career.as_ref().is_some_and(|c| c.market.applied.iter().any(|x| x.0 == e && x.1 == r));
    let mut out: Vec<OpenJob> = super::super::staff::searches(w)
        .into_iter()
        .map(|(e, r, d)| OpenJob { employer: e, role: r, decides: Some(d), applied: applied(e, r) })
        .collect();
    for (e, r) in super::super::staff::empty_seats(w) {
        if !out.iter().any(|x| x.employer == e && x.role == r) {
            out.push(OpenJob { employer: e, role: r, decides: None, applied: applied(e, r) });
        }
    }
    out
}

/// Puts your name in for an open job. A college search decides on its own
/// day; a seat nobody holds is filled now, by you or the best name.
pub fn apply(w: &mut World, e: Employer, r: Role) -> Result<String, String> {
    let today = w.today;
    let c = w.career.as_ref().ok_or("No career.")?;
    if your_job(w).is_some_and(|j| j.employer == e && j.role == r) {
        return Err("That's your job.".into());
    }
    if c.market.passed.iter().any(|x| x.0 == e && x.1 == r && x.2.days_to(today) < 60) {
        return Err("You turned that job down. They won't call again this year.".into());
    }
    let open = openings(w).into_iter().find(|x| x.employer == e && x.role == r).ok_or("That job isn't open.")?;
    if let Some(k) = w.career.as_mut() {
        k.market.applied.retain(|x| !(x.0 == e && x.1 == r));
        k.market.applied.push((e, r, today));
    }
    let name = employer_name(w, e);
    log(w, format!("Applied: {}, {name}.", role_title(r)));
    if let Some(d) = open.decides {
        return Ok(format!("Your name is in at {name}. The search decides around {}.", d.pretty()));
    }
    let prestige = super::super::staff::prestige_of(w, e);
    super::super::staff::fill_now(w, e, r, prestige);
    let offered = w.career.as_ref().is_some_and(|c| c.market.offers.iter().any(|o| o.employer == e && o.role == r));
    if offered {
        return Ok(format!("{name} wants you. Their offer is on your job market page."));
    }
    let who = super::super::staff::holder(w, e, r).map(|h| w.coaches[h].name.clone());
    Ok(match who {
        Some(n) => format!("{name} went with {n}."),
        None => format!("{name} didn't find its man yet."),
    })
}

// ---------------------------------------------------------------------------
// Hooks from the shared staff code
// ---------------------------------------------------------------------------

/// You took a job (an offer you said yes to, or a promotion): your seat
/// moves there, the desks start over, a high school's staff is drawn and
/// the family moves if the job is somewhere else.
pub(crate) fn hired(w: &mut World, e: Employer, r: Role) {
    w.seat = Some(match e {
        Employer::HighSchool(s) => Seat { level: Level::HighSchool, nfl_team: None, program: Some(s), other: None },
        Employer::College(p) => Seat { level: Level::College, nfl_team: None, program: Some(p), other: None },
        Employer::Nfl(t) => Seat { level: Level::Nfl, nfl_team: Some(t), program: None, other: None },
        Employer::Other(f) => Seat { level: Level::Other, nfl_team: None, program: None, other: Some(f) },
    });
    let log_kept = std::mem::take(&mut w.front_office.log);
    w.front_office = Default::default();
    w.front_office.log = log_kept;
    w.seat_depth.clear();
    w.seat_focus.clear();
    let today = w.today;
    let staff = match (e, r) {
        (Employer::HighSchool(s), Role::HeadCoach) => w.hire_hs_staff(s),
        _ => Vec::new(),
    };
    // The stories you walk into, and the premium's bigger buyout (update 2).
    crate::world::storylines::credit::hired(w, e);
    if let Some(c) = w.career.as_mut() {
        c.role = (r != Role::HeadCoach).then_some(r);
        c.hs_staff = staff;
        c.market.out_since = None;
        c.market.job_since = Some(today);
        c.market.teaching_at = None;
        c.market.applied.clear();
    }
    if let (Employer::HighSchool(s), Role::HeadCoach) = (e, r) {
        super::school::open(w, s);
    }
    super::super::finance::hs::open_seat(w);
    w.prune_tapes();
    move_family(w, e);
    let name = employer_name(w, e);
    post(w, today, level_of(e), format!("You start as {} at {name}.", r.name()), false);
}

/// The first line in the inbox of a career that starts on a staff or out
/// of work (a head job opens with its own: the first meeting, the staff).
pub fn welcome(w: &mut World) {
    let today = w.today;
    match your_job(w) {
        Some(j) if j.role != Role::HeadCoach => {
            let boss = w.your_boss().map(|b| w.coaches[b].name.clone());
            let staff = boss.as_ref().map_or("the staff".to_string(), |b| format!("{b}'s staff"));
            let text = format!(
                "You join {staff} at {} as {}. You set the depth chart and the players' plans for {}; {} has the last word.",
                employer_name(w, j.employer),
                j.role.name(),
                w.your_controls(),
                boss.unwrap_or_else(|| "the head coach".into())
            );
            post(w, today, level_of(j.employer), text, false);
        }
        None => {
            let level = w.seat.map_or(Level::College, |s| s.level);
            let what = match level {
                Level::Nfl => "an NFL staff",
                Level::HighSchool => "a high school head job",
                _ => "a college staff",
            };
            let text = format!(
                "You are out of coaching and looking for {what}. The Job market shows every opening; put your name in, or wait for a search to call."
            );
            post(w, today, level, text, false);
        }
        _ => {}
    }
}

/// You lost your job (fired, let go by a new head coach, not renewed) or
/// left it: until the next one you have no team.
pub(crate) fn left(w: &mut World, why: &str) {
    let Some(seat) = w.seat else { return };
    let today = w.today;
    let name = match seat.level {
        Level::Nfl => seat.nfl_team.map(|t| w.nfl_team_name(t)),
        Level::HighSchool => seat.program.map(|s| w.school_name(s).to_string()),
        Level::Other => seat.other.map(|f| crate::world::pro::other::name(f, w.year).to_string()),
        _ => seat.program.map(|p| w.program_name(p).to_string()),
    }
    .unwrap_or_default();
    w.seat = Some(Seat { level: seat.level, nfl_team: None, program: None, other: None });
    w.prune_tapes();
    w.seat_depth.clear();
    w.seat_focus.clear();
    if let Some(c) = w.career.as_mut() {
        c.role = None;
        c.hs_staff.clear();
        c.market.out_since = Some(today);
    }
    if why != "left for another job" {
        log(w, format!("{name}: {why}."));
        post(
            w,
            today,
            seat.level,
            format!("You are out at {name} ({why}). Your job market page lists what's open and what you'd take."),
            true,
        );
    }
}

/// A high school didn't renew your coaching stipend: you keep teaching there.
pub(crate) fn kept_teaching(w: &mut World, s: usize) {
    if let Some(c) = w.career.as_mut() {
        c.market.teaching_at = Some(s);
    }
}

/// Anyone else hired: someone who knows you landing a head job is news
/// to you (he'll be hiring a staff).
pub(crate) fn someone_hired(w: &mut World, c: usize, e: Employer, r: Role) {
    if r != Role::HeadCoach || w.career.is_none() || is_you(w, c) {
        return;
    }
    let Some(k) = w.career.as_ref().and_then(|x| x.known.iter().find(|k| k.coach == c)).cloned() else { return };
    let today = w.today;
    let name = w.coaches[c].name.clone();
    let at = employer_name(w, e);
    post(
        w,
        today,
        level_of(e),
        format!("{name} ({}) is the new head coach at {at}. He'll be hiring a staff.", k.how.to_lowercase()),
        false,
    );
}

/// A move for the family (people topic 13): a household that moves loses
/// some of its peace, more with a teenager in high school, less going
/// home; it comes back with years in one place.
fn move_family(w: &mut World, e: Employer) {
    let to = state_of(w, e);
    let year = w.year;
    let today = w.today;
    let Some(c) = w.career.as_mut() else { return };
    if to.is_none() || to == c.family.state {
        return;
    }
    let teen = c.you.profile.children.iter().any(|(_, b)| (14..=17).contains(&(year - b)));
    let home = to == Some(c.you.home_state);
    let first = c.family.state.is_none();
    let f = &mut c.family;
    if !first {
        f.mood = (f.mood - 0.2 - if teen { 0.15 } else { 0.0 } + if home { 0.15 } else { 0.0 }).clamp(0.05, 1.0);
        f.moves += 1;
    }
    f.state = to;
    f.since = year;
    let place = to.and_then(|s| w.states.get(s)).map_or("", |s| state_name(&s.code)).to_string();
    if !first && let Some(c) = w.career.as_mut() {
        c.family.log.push((today, format!("The family moved to {place}.")));
    }
}

// ---------------------------------------------------------------------------
// Your family's year and your offers' clock
// ---------------------------------------------------------------------------

fn told(w: &mut World, k: u8) -> bool {
    let y = w.year;
    let Some(c) = w.career.as_mut() else { return true };
    if c.family.told.contains(&(y, k)) {
        return true;
    }
    c.family.told.push((y, k));
    false
}

/// The market's day: offers run out; the family's year (a child starts
/// high school, a senior graduates, a settled household).
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(c) = w.career.as_ref() else { return };
    // Offers that ran out are turned down.
    let gone: Vec<Offer> = c.market.offers.iter().filter(|o| o.until < d).cloned().collect();
    if !gone.is_empty() {
        if let Some(c) = w.career.as_mut() {
            c.market.offers.retain(|o| o.until >= d);
        }
        for o in gone {
            let name = employer_name(w, o.employer);
            log(w, format!("Let the {} offer at {name} run out.", o.role.name()));
            pass(w, &o);
        }
    }
    let (_, m, day) = d.parts();
    let year = w.year;
    // The family's year, in August (school starts) and June (graduation).
    if (m, day) == (8, 10) && !told(w, 1) {
        let kids: Vec<(String, i32)> = w.career.as_ref().map(|c| c.you.profile.children.clone()).unwrap_or_default();
        let school = w.seat_school().map(|s| w.school_name(s).to_string());
        let mut lines = Vec::new();
        for (name, born) in kids {
            match year - born {
                14 => lines.push(match &school {
                    Some(s) => format!("{name} starts high school at {s}, your school."),
                    None => format!("{name} starts high school."),
                }),
                5 => lines.push(format!("{name} starts kindergarten.")),
                _ => {}
            }
        }
        for l in lines {
            if let Some(c) = w.career.as_mut() {
                c.family.log.push((d, l.clone()));
            }
            post(w, d, w.seat.map_or(Level::College, |s| s.level), l, false);
        }
        // Years in one place settle a household.
        if let Some(c) = w.career.as_mut() {
            let f = &mut c.family;
            f.mood = (f.mood + 0.06).min(0.95);
        }
    }
    if (m, day) == (6, 1) && !told(w, 2) {
        let kids: Vec<(String, i32)> = w.career.as_ref().map(|c| c.you.profile.children.clone()).unwrap_or_default();
        for (name, born) in kids {
            if year - born == 18 {
                let l = format!("{name} graduates from high school and leaves home for college in the fall.");
                if let Some(c) = w.career.as_mut() {
                    c.family.log.push((d, l.clone()));
                }
                post(w, d, w.seat.map_or(Level::College, |s| s.level), l, false);
            }
        }
    }
    // A household that hasn't settled says so, once a year.
    if (m, day) == (11, 20) && !told(w, 3) {
        let unhappy = w.career.as_ref().is_some_and(|c| {
            c.family.mood < 0.3 && c.you.profile.status == "Married" && !c.you.profile.spouse.is_empty()
        });
        if unhappy {
            let spouse = w.career.as_ref().map(|c| c.you.profile.spouse.clone()).unwrap_or_default();
            let l = format!("{spouse} wants to talk about how long you'll stay: the family hasn't settled here.");
            if let Some(c) = w.career.as_mut() {
                c.family.log.push((d, l.clone()));
            }
            post(w, d, w.seat.map_or(Level::College, |s| s.level), l, false);
        }
    }
}

/// The household's mood in words.
pub fn family_mood_words(m: f32) -> &'static str {
    match m {
        m if m >= 0.75 => "Happy here",
        m if m >= 0.55 => "Settled",
        m if m >= 0.4 => "Getting used to it",
        m if m >= 0.25 => "Unsettled",
        _ => "Unhappy with the moves",
    }
}

// ---------------------------------------------------------------------------
// People you know
// ---------------------------------------------------------------------------

/// Where you were in the world's history, from your profile and your road.
#[derive(Clone, Debug, Default)]
pub struct Placement {
    /// Your college and the years you were there.
    pub college: Option<(usize, i32, i32)>,
    /// Whether you played there (a playing road).
    pub played: bool,
    /// Your high school and your years there.
    pub high_school: Option<(usize, i32, i32)>,
    /// Your last stops (a staff, or an NFL club you played for) and years.
    pub stops: Vec<(Employer, i32, i32, bool)>,
}

/// The people from the world's history who know you (people topic 2):
/// the coaches who coached you in high school and college, the coaches
/// you worked with at your last stops, and the ones who coached you in
/// the NFL. Only coaches the world kept a record of, and only stints that
/// overlap your years there (records begin with the world's history).
pub fn people_you_know(w: &World, p: &Placement) -> Vec<Known> {
    use crate::world::coach::Stint;
    let mut out: Vec<Known> = Vec::new();
    let stints = |c: &crate::world::coach::Coach| -> Vec<Stint> {
        let mut v = c.history.clone();
        if let Some(j) = c.job {
            v.push(Stint {
                employer: j.employer,
                role: j.role,
                from: j.since,
                to: w.year,
                wins: 0,
                losses: 0,
                titles: 0,
                ended: "",
            });
        }
        v
    };
    let mut add = |c: usize, how: String, from: i32, to: i32, strength: f32| {
        if let Some(k) = out.iter_mut().find(|k| k.coach == c) {
            if strength > k.strength {
                k.how = how;
                k.strength = strength;
                k.from = from;
                k.to = to;
            }
        } else {
            out.push(Known { coach: c, how, from, to, strength });
        }
    };
    for co in w.coaches.iter().filter(|c| !c.retired || c.job.is_some()) {
        if w.career.as_ref().is_some_and(|k| k.coach == co.id) {
            continue;
        }
        for s in stints(co) {
            let over = |a: i32, b: i32| s.from <= b && s.to >= a;
            if let Some((h, a, b)) = p.high_school
                && s.employer == Employer::HighSchool(h)
                && over(a, b)
            {
                let how = if s.role == Role::HeadCoach {
                    "Your high school head coach"
                } else {
                    "Coached you in high school"
                };
                add(co.id, format!("{how} at {}", w.school_name(h)), s.from.max(a), s.to.min(b), 0.7);
            }
            if let Some((c, a, b)) = p.college
                && s.employer == Employer::College(c)
                && over(a, b)
            {
                let name = w.program_name(c);
                let (how, k) = match (p.played, s.role) {
                    (true, Role::HeadCoach) => (format!("Your head coach at {name}"), 0.75),
                    (true, Role::PositionCoach(_)) => (format!("Your position coach at {name}"), 0.85),
                    (true, _) => (format!("Coached you at {name}"), 0.7),
                    (false, Role::HeadCoach) => (format!("Head coach at {name} when you were a student"), 0.25),
                    (false, _) => continue,
                };
                add(co.id, how, s.from.max(a), s.to.min(b), k);
            }
            for &(e, a, b, playing) in &p.stops {
                if s.employer == e && over(a, b) {
                    let name = employer_name(w, e);
                    // "on the Kansas City Chiefs", "at Alabama".
                    let pro = matches!(e, Employer::Nfl(_));
                    let (how, k) = match (playing, s.role == Role::HeadCoach, pro) {
                        (true, _, true) => (format!("Coached you on the {name}"), 0.6),
                        (true, _, false) => (format!("Coached you at {name}"), 0.6),
                        (false, true, true) => (format!("Your head coach on the {name}"), 0.8),
                        (false, true, false) => (format!("Your head coach at {name}"), 0.8),
                        (false, false, true) => (format!("On the {name} staff with you"), 0.65),
                        (false, false, false) => (format!("On the staff at {name} with you"), 0.65),
                    };
                    add(co.id, how, s.from.max(a), s.to.min(b), k);
                }
            }
        }
    }
    out.sort_by(|a, b| b.strength.total_cmp(&a.strength).then(b.to.cmp(&a.to)));
    out.truncate(40);
    out
}

// ---------------------------------------------------------------------------
// An assistant's seat (first-playable topic 2: you control what the job controls)
// ---------------------------------------------------------------------------

impl World {
    /// Your role in your job (None: head coach, or out of work).
    pub fn your_role(&self) -> Option<Role> {
        your_job(self).map(|j| j.role).filter(|r| *r != Role::HeadCoach)
    }

    /// Whether you hold your team's head job.
    pub fn seat_head(&self) -> bool {
        your_job(self).is_some_and(|j| j.role == Role::HeadCoach)
    }

    /// Whether your job controls this position: the head coach controls
    /// all; a coordinator his side; a position coach his group.
    pub fn controls(&self, pos: Position) -> bool {
        match self.your_role() {
            None => self.seat_head(),
            Some(Role::OffensiveCoordinator) => Group::of(pos).offense() && Group::of(pos) != Group::Specialists,
            Some(Role::DefensiveCoordinator) => !Group::of(pos).offense() && Group::of(pos) != Group::Specialists,
            Some(Role::PositionCoach(g)) => Group::of(pos) == g,
            Some(Role::HeadCoach) => true,
        }
    }

    /// The order the engine plays for your team: yours where your job
    /// controls it.
    pub(crate) fn seat_order(&self) -> Vec<(Position, Vec<u32>)> {
        if self.seat_head() {
            return self.seat_depth.clone();
        }
        self.seat_depth.iter().filter(|(p, _)| self.controls(*p)).cloned().collect()
    }

    /// Your depth chart order at a position, as a proposal when you are an
    /// assistant: the head coach reads it against his own and can overrule
    /// you when your starters grade clearly worse to him. Returns his
    /// answer in words (None: no answer needed).
    pub fn propose_depth(&mut self, pos: Position, ids: Vec<u32>) -> Result<Option<String>, String> {
        if self.career.is_none() || self.seat.is_none() {
            return Err("You have no team.".into());
        }
        if self.seat_head() || ids.is_empty() {
            self.set_seat_depth(pos, ids);
            return Ok(None);
        }
        if !self.controls(pos) {
            let hc = self.head_coach_name().unwrap_or_else(|| "the head coach".into());
            return Err(format!("{} is {hc}'s call. Your job controls {}.", pos.name(), self.your_controls()));
        }
        // His order, then yours: how much worse your starters grade to him.
        let n = crate::game::roster::starters(pos).max(1);
        let read = |id: u32| super::index_of(self, id).map_or(0.0, |i| self.people[i].player.overall());
        let mut his: Vec<u32> = ids.clone();
        his.sort_by(|a, b| read(*b).total_cmp(&read(*a)));
        let mine: f64 = ids.iter().take(n).map(|&i| read(i)).sum::<f64>() / n as f64;
        let best: f64 = his.iter().take(n).map(|&i| read(i)).sum::<f64>() / n as f64;
        let gap = best - mine;
        let hc = self.your_boss();
        let temper = hc.map_or(10.0, |h| self.coaches[h].personality.temper);
        // Proposed, not confirmed: he lets 3 points go, and a hot-tempered
        // head coach overrules more readily.
        let p = clamp((gap - 3.0) / 6.0, 0.0, 0.9) * clamp(0.7 + 0.04 * (temper - 10.0), 0.4, 1.1);
        let u = key(self, SALT ^ 0xD0 ^ (self.today.days_to(Date::ymd(2000, 1, 1)) as u64) << 12, pos as u64);
        let name = self.head_coach_name().unwrap_or_else(|| "The head coach".into());
        if u < p {
            let keep = his.first().and_then(|&id| super::index_of(self, id)).map(|i| self.people[i].name.clone());
            let text = format!(
                "{name} overruled you at {}: he starts {} (\"I see it differently on tape\").",
                pos.name(),
                keep.unwrap_or_default()
            );
            super::note(self, text.clone(), None);
            return Ok(Some(text));
        }
        self.set_seat_depth(pos, ids);
        Ok(Some(if gap > 1.5 {
            format!("{name} went along with your order at {}, with doubts.", pos.name())
        } else {
            format!("{name} signed off on your order at {}.", pos.name())
        }))
    }

    /// Your résumé from the setup: the staffs you worked on before this
    /// career, kept as stints on your coach so searches read them as they
    /// read anyone's (an NFL owner's doubts about a man who never coached
    /// there, a head job's experience).
    pub fn add_resume(&mut self, stops: &[(Employer, Role, i32, i32)]) {
        let Some(me) = me(self) else { return };
        let c = &mut self.coaches[me];
        for &(employer, role, from, to) in stops {
            c.history.push(crate::world::coach::Stint {
                employer,
                role,
                from,
                to,
                wins: 0,
                losses: 0,
                titles: 0,
                ended: "left for another job",
            });
        }
        c.history.sort_by_key(|s| (s.from, s.to));
    }

    /// The head coach you work for, as an assistant.
    pub fn your_boss(&self) -> Option<usize> {
        let j = your_job(self)?;
        if j.role == Role::HeadCoach {
            return None;
        }
        match j.employer {
            Employer::College(p) => self.programs[p].head_coach,
            Employer::Nfl(t) => self.nfl[t].head_coach,
            Employer::HighSchool(s) => self.high_schools[s].coach,
            Employer::Other(f) => crate::world::pro::other::head_coach(self, f),
        }
    }

    fn head_coach_name(&self) -> Option<String> {
        self.your_boss().map(|h| self.coaches[h].name.clone())
    }

    /// What your job controls, in words.
    pub fn your_controls(&self) -> String {
        match self.your_role() {
            None => "everything".into(),
            Some(Role::OffensiveCoordinator) => "the offense".into(),
            Some(Role::DefensiveCoordinator) => "the defense".into(),
            Some(Role::PositionCoach(g)) => format!("the {}", g.name()),
            Some(Role::HeadCoach) => "everything".into(),
        }
    }

    /// Whether you may set this player's plan: your job's players only.
    pub fn may_plan(&self, player: u32) -> bool {
        self.seat_head() || super::index_of(self, player).is_some_and(|i| self.controls(self.people[i].player.position))
    }
}

/// Your Aura (final build F15, for you): a hidden 1-20 presence, from your
/// road (an NFL star walks into a room differently), how you deal with
/// people, and a little of your own (proposed, not confirmed: the sizes).
pub fn your_aura(road: &str, relationships: f64, motivation: f64, seed: u32) -> f64 {
    let base = match road {
        "Former NFL star" => 15.0,
        "Former NFL Pro Bowler" => 13.0,
        "Former NFL starter" => 11.5,
        "Former NFL journeyman" => 10.5,
        "Former college player" => 10.0,
        "Analytics" => 8.5,
        "Scout or personnel man" => 9.0,
        _ => 9.5,
    };
    let mut r = Rng::new(seed ^ 0xA0_7A);
    clamp(base + 0.25 * (relationships - 10.0) + 0.15 * (motivation - 10.0) + r.normal(0.0, 1.2), 1.0, 20.0)
}

/// The keyed draw a test can check: the same question gets the same answer.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aura_follows_the_road() {
        let star = your_aura("Former NFL star", 10.0, 10.0, 7);
        let analyst = your_aura("Analytics", 10.0, 10.0, 7);
        assert!(star > analyst);
        assert!((1.0..=20.0).contains(&star));
    }

    #[test]
    fn family_moods_read_in_words() {
        assert_eq!(family_mood_words(0.8), "Happy here");
        assert_eq!(family_mood_words(0.1), "Unhappy with the moves");
    }

    #[test]
    fn an_offer_nobody_wanted_is_no_interest() {
        let i = Interest { hs_head: false, ..Default::default() };
        assert!(!i.hs_head && i.college_head);
    }
}
