//! Your college desk (lane 10): the recruiting board, scholarship offers,
//! visits, the transfer portal and NIL, made by you through the recruiting
//! code every AI staff runs on.
//!
//! How it plugs in (first playable topic 6: where you choose, the AI's
//! choice for your team is skipped):
//! - **Board.** Your board holds the recruits you follow, in your order.
//!   On each commitment wave and signing day the matching offers down your
//!   board instead of your staff's: commitments first, then your order.
//!   Each recruit weighs your offer by the same appeal, relationship and
//!   room as any AI offer, and holds the best one. Your class can't pass the
//!   NCAA's signee cap or the roster limit, so an offer past them isn't
//!   committable until a place opens. Where `fill_board` is on, your
//!   recruiting coordinator offers down his own board at a position whose
//!   spots your offers don't cover, and says so.
//! - **Visits.** Your program hosts only the visits you set: official
//!   visits on the NCAA's days (seniors from September 1, juniors April 1
//!   to June 30 from 2018, never in a dead period, five a recruit, the fall
//!   ones on home weekends), unofficial ones on any day but a dead period,
//!   the head coach's home visit in a contact period, and drop-ins at his
//!   practice in evaluation weeks. A recruit says yes or no when you ask,
//!   by what he thinks of you against the others recruiting him.
//! - **Portal.** Your program wants only the entrants you offer; each one
//!   weighs your offer with the same wants, room and money as any other.
//!   Before the winter window you can sit down with your own players (your
//!   relationships rating sways them, as an AI head coach's sways his), and
//!   reach out quietly to a player elsewhere (tampering: never punished,
//!   but it can leak).
//! - **NIL.** From July 2021 the collective's money (and from 2025 the
//!   House settlement's revenue share) is yours to promise: a recruit or an
//!   entrant weighs your number instead of the collective's usual one, and
//!   on July 1 your roster is paid your deals instead of the usual split.
//!
//! A duty handed to your staff runs exactly as the AI's. The desk keeps
//! person ids (people move when the world compacts each year).

use std::collections::HashMap;

use super::{Duty, Owner, dollars, indices, key, note, tell};
use crate::calendar::{Date, Event, Kind, Level, Loudness, Period, Weekday, recruiting_period};
use crate::positions::Position;
use crate::world::recruiting::{self as rec, recruiting_stars};
use crate::world::{Stage, Visit, World, data::Division, geo, ncaa};

/// Who put a recruit on your board or made his offer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Source {
    #[default]
    You,
    /// Your recruiting coordinator, filling a spot you left him.
    Coordinator,
    /// Already recruited by the staff before you took the job or the duty.
    Inherited,
}

/// Where a recruit stands with you, as the desk last saw it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Standing {
    #[default]
    Open,
    CommittedHere,
    SignedHere,
    CommittedElsewhere,
    SignedElsewhere,
    /// On campus with you.
    Enrolled,
    /// Enrolled elsewhere, or out of football.
    Gone,
}

/// One recruit on your board.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Target {
    pub id: u32,
    pub added: Date,
    pub by: Source,
    /// The day your scholarship offer went out, if it has.
    pub offered: Option<Date>,
    /// The day you pulled it.
    pub pulled: Option<Date>,
    /// The head coach's own target: he gives this recruit his time.
    pub priority: bool,
    /// Your NIL promise a year in dollars; None is the collective's usual offer.
    pub nil: Option<f64>,
    /// Your coordinator's reason, when he made the offer.
    #[serde(default)]
    pub why: Option<String>,
    #[serde(default)]
    pub seen: Standing,
}

/// A kind of visit you can set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum VisitKind {
    Official,
    Unofficial,
    Home,
    DropIn,
}

impl VisitKind {
    pub const ALL: [VisitKind; 4] = [VisitKind::Official, VisitKind::Unofficial, VisitKind::Home, VisitKind::DropIn];
    pub fn key(self) -> &'static str {
        match self {
            VisitKind::Official => "official",
            VisitKind::Unofficial => "unofficial",
            VisitKind::Home => "home",
            VisitKind::DropIn => "drop_in",
        }
    }
    pub fn from_key(k: &str) -> Option<VisitKind> {
        VisitKind::ALL.into_iter().find(|v| v.key() == k)
    }
    pub fn name(self) -> &'static str {
        match self {
            VisitKind::Official => "Official visit",
            VisitKind::Unofficial => "Unofficial visit",
            VisitKind::Home => "Home visit",
            VisitKind::DropIn => "Drop-in at practice",
        }
    }
    fn visit(self) -> Visit {
        match self {
            VisitKind::Official => Visit::Official,
            VisitKind::Unofficial => Visit::Unofficial,
            VisitKind::Home => Visit::Home,
            VisitKind::DropIn => Visit::DropIn,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VisitState {
    /// Set and accepted: it happens on its day.
    Set,
    Done,
    /// He said no when you asked.
    Declined,
    /// It didn't happen (he signed elsewhere, the rules closed it, you cancelled).
    Off,
}

/// A visit you set.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Plan {
    pub id: u32,
    pub kind: VisitKind,
    pub date: Date,
    pub state: VisitState,
    /// His answer, or what happened, in words.
    pub note: String,
}

/// A transfer you chase.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PortalTarget {
    pub id: u32,
    pub added: Date,
    pub offered: Option<Date>,
    /// Your NIL number a year; None is the collective's usual.
    pub nil: Option<f64>,
    /// A quiet word before he entered (tampering), and whether it leaked.
    pub contacted: Option<Date>,
    pub leaked: bool,
    /// The program he played for when you added him.
    pub from: usize,
    /// Already told about his decision.
    pub settled: bool,
}

/// A sit-down with one of your own players before the winter window.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Talk {
    pub id: u32,
    pub date: Date,
    /// What it does to his lean at the window (1.0 no change).
    pub factor: f64,
    pub answer: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CollegeDesk {
    /// Your board was set up from your staff's work when you took over.
    pub seeded: bool,
    /// The program the desk belongs to (a new job starts a new desk).
    pub program: Option<usize>,
    pub board: Vec<Target>,
    pub visits: Vec<Plan>,
    pub portal: Vec<PortalTarget>,
    pub talks: Vec<Talk>,
    /// Your roster's NIL deals for the money year starting July 1: (id, a year).
    pub deals: Vec<(u32, f64)>,
    /// The money year the deals are for (its July).
    pub deals_year: i32,
    /// Your players already reported in the portal this year.
    pub left: Vec<u32>,
    /// The class signing next when the board was last rolled over.
    pub class: i32,
}

// --- What the shared recruiting code reads ---------------------------------------------

/// One of your offers for a signing stage.
#[derive(Clone, Debug)]
pub(crate) struct StageOffer {
    pub person: usize,
    pub priority: bool,
    pub nil: Option<f64>,
}

/// Your board for a signing stage, when the board is yours.
pub(crate) struct StageBoard {
    pub program: usize,
    pub offers: Vec<StageOffer>,
    pub fill: bool,
}

/// Your program when the duty is yours.
fn mine(w: &World, d: Duty) -> Option<usize> {
    let j = super::program(w)?;
    super::yours_program(w, j, d).then_some(j)
}

fn nil_yours(w: &World) -> bool {
    mine(w, Duty::Nil).is_some()
}

/// Your program for an action on a duty you hold; the staff's words when
/// the duty is theirs (an action half-run beside them would be ignored).
fn need(w: &World, d: Duty) -> Result<usize, String> {
    let j = super::program(w).ok_or("You have no college job.")?;
    if w.front_office.owner(d) == Owner::You {
        return Ok(j);
    }
    Err(match d {
        Duty::Board => "Your recruiting coordinator runs the board now: take it back to offer and rank yourself.",
        Duty::Visits => "Your staff sets the visits now: take visits back to plan your own.",
        Duty::Portal => "Your staff works the portal now: take it back to chase transfers yourself.",
        _ => "Your collective sets the money now: take NIL back first.",
    }
    .into())
}

/// Your board for a signing stage. Whoever holds the board, only the
/// offers you made or signed go out (SK26); the fill switch is your
/// standing word to your coordinator for the open spots.
pub(crate) fn stage_board(w: &World) -> Option<StageBoard> {
    let j = super::program(w)?;
    let desk = &w.front_office.college;
    let live: Vec<&Target> = desk.board.iter().filter(|t| t.offered.is_some() && t.pulled.is_none()).collect();
    let at = indices(w, &live.iter().map(|t| t.id).collect::<Vec<_>>());
    let nil = nil_yours(w);
    let offers = live
        .iter()
        .filter_map(|t| {
            Some(StageOffer { person: *at.get(&t.id)?, priority: t.priority, nil: if nil { t.nil } else { None } })
        })
        .collect();
    Some(StageBoard { program: j, offers, fill: w.front_office.fill_board })
}

pub(crate) fn visits_program(w: &World) -> Option<usize> {
    mine(w, Duty::Visits)
}

/// Your offer to a portal entrant: the money and the ties behind it.
pub(crate) struct PortalOffer {
    pub nil: Option<f64>,
    pub bond: f64,
}

pub(crate) struct PortalWants {
    pub program: usize,
    pub offers: HashMap<u32, PortalOffer>,
}

/// A quiet word before he entered is worth a little at decision time
/// (proposed, not confirmed: 0.05 appeal points, the size of a childhood
/// lean's quarter).
const CONTACT_BOND: f64 = 0.05;

pub(crate) fn portal_wants(w: &World) -> Option<PortalWants> {
    let j = mine(w, Duty::Portal)?;
    let nil = nil_yours(w);
    let offers = w
        .front_office
        .college
        .portal
        .iter()
        .filter(|t| t.offered.is_some())
        .map(|t| {
            (
                t.id,
                PortalOffer {
                    nil: if nil { t.nil } else { None },
                    bond: if t.contacted.is_some() { CONTACT_BOND } else { 0.0 },
                },
            )
        })
        .collect();
    Some(PortalWants { program: j, offers })
}

pub(crate) fn stay_talks(w: &World) -> Option<(usize, HashMap<u32, f64>)> {
    let j = mine(w, Duty::Portal)?;
    Some((j, w.front_office.college.talks.iter().map(|t| (t.id, t.factor)).collect()))
}

pub(crate) fn roster_deals(w: &World) -> Option<(usize, HashMap<u32, f64>)> {
    let j = mine(w, Duty::Nil)?;
    let desk = &w.front_office.college;
    let year = w.today.year();
    let deals = if desk.deals_year == year { desk.deals.iter().copied().collect() } else { HashMap::new() };
    Some((j, deals))
}

/// After a signing stage: your coordinator's offers go on your board with
/// his reason, and every offer of yours stays on the recruit's list.
pub(crate) fn after_stage(w: &mut World, j: usize, coordinator: &[u32], final_day: bool) {
    let today = w.today;
    if !coordinator.is_empty() {
        let at = indices(w, coordinator);
        let outlook = rec::class_outlook(w, j);
        let mut added = 0;
        for &id in coordinator {
            let Some(&i) = at.get(&id) else { continue };
            let pos = w.people[i].player.position;
            let open = outlook.need[pos.index()];
            let why = format!(
                "Your recruiting coordinator offered him: {pos} had {} open in the class and your board didn't cover it.",
                count(open, "spot")
            );
            let desk = &mut w.front_office.college;
            match desk.board.iter_mut().find(|t| t.id == id) {
                Some(t) if t.offered.is_some() && t.pulled.is_none() => {}
                Some(t) => {
                    t.offered = Some(today);
                    t.pulled = None;
                    t.by = Source::Coordinator;
                    t.why = Some(why);
                    added += 1;
                }
                None => {
                    desk.board.push(Target {
                        id,
                        added: today,
                        by: Source::Coordinator,
                        offered: Some(today),
                        pulled: None,
                        priority: false,
                        nil: None,
                        why: Some(why),
                        seen: Standing::Open,
                    });
                    added += 1;
                }
            }
        }
        if added > 0 {
            let who = coordinator_name(w, j);
            tell(
                w,
                today,
                format!(
                    "{who} offered {} where your board left spots open. They're on your board, marked as his.",
                    count(added, "recruit")
                ),
                false,
            );
        }
    }
    if !final_day {
        // Your offers stay on each recruit's list for the visits ahead.
        let ids: Vec<u32> = w
            .front_office
            .college
            .board
            .iter()
            .filter(|t| t.offered.is_some() && t.pulled.is_none())
            .map(|t| t.id)
            .collect();
        let at = indices(w, &ids);
        for id in ids {
            if let Some(&i) = at.get(&id) {
                let offers = &mut w.people[i].cal.offers;
                if !offers.contains(&j) {
                    offers.push(j);
                }
            }
        }
    }
    // The stage's commitments and signatures reach you the same day.
    scan_board(w, j, today);
}

/// July 1 with your deals: the players paid far under their share hear it.
pub(crate) fn after_payouts(w: &mut World, short: &[(u32, f64, f64)]) {
    let today = w.today;
    let year = today.year();
    let at = indices(w, &short.iter().map(|s| s.0).collect::<Vec<_>>());
    for &(id, money, fair) in short {
        if let Some(&i) = at.get(&id) {
            let p = &mut w.people[i];
            let age = p.row_age();
            p.note(format!(
                "At {age} his NIL deal ({}) fell well short of what players in his role were paid; he took it badly.",
                dollars(money)
            ));
            let _ = fair;
        }
    }
    if !short.is_empty() {
        let names: Vec<String> = short.iter().filter_map(|s| at.get(&s.0)).map(|&i| w.people[i].name.clone()).collect();
        let more = if names.len() > 4 { format!(" and {} more", names.len() - 4) } else { String::new() };
        tell(
            w,
            today,
            format!(
                "The collective paid your deals for {year}-{}. {} feel underpaid against their role: {}{more}. Their lean toward the portal grew.",
                (year + 1) % 100,
                count(short.len(), "player"),
                names.iter().take(4).cloned().collect::<Vec<_>>().join(", ")
            ),
            false,
        );
    } else {
        tell(w, today, format!("The collective paid your NIL deals for {year}-{}.", (year + 1) % 100), false);
    }
}

// --- The desk's day ------------------------------------------------------------------------

pub(crate) fn day(w: &mut World, d: Date) {
    let Some(j) = ready(w) else { return };
    super::chain::day(w, j, d);
    run_visits(w, j, d);
    scan_board(w, j, d);
    roll_class(w, j);
    scan_portal(w, j, d);
    reminders(w, j, d);
    if d == w.dates.start {
        prune(w);
    }
}

/// The desk for your program, set up from your staff's work the first
/// time it is read (the day you take the job) and after a move.
pub fn ready(w: &mut World) -> Option<usize> {
    let j = super::program(w)?;
    if w.front_office.college.program != Some(j) {
        // A new job: a new desk, set up from the staff's work there.
        w.front_office.college = CollegeDesk { program: Some(j), ..Default::default() };
    }
    if !w.front_office.college.seeded {
        seed(w, j);
    }
    Some(j)
}

/// Once signing day has passed, the class that signed is on the roster and
/// leaves the board, each name after its news has reached you (the day's
/// scan sets what you last saw of him).
fn roll_class(w: &mut World, j: usize) {
    let next = rec::next_class(w);
    let ids: Vec<u32> = w.front_office.college.board.iter().map(|t| t.id).collect();
    let at = indices(w, &ids);
    let people = &w.people;
    w.front_office.college.board.retain(|t| {
        at.get(&t.id).is_some_and(|&i| {
            let p = &people[i];
            is_juco_recruit(p)
                || (matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) && p.class_of >= next)
                || t.seen != standing(p, j)
        })
    });
    w.front_office.college.class = next;
}

/// Takes over your staff's recruiting where it stands: every recruit who
/// holds your program's offer, or committed or signed with it, goes on
/// your board as already offered, in the order the staff valued them.
pub fn seed(w: &mut World, j: usize) {
    let today = w.today;
    let next = rec::next_class(w);
    let mut found: Vec<(u8, u8, Date, u32)> = Vec::new();
    for p in w.people.iter() {
        // The class signing next and the younger ones; a class that has
        // signed is the roster's now.
        if !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) || p.class_of < next {
            continue;
        }
        let signed = p.cal.signed.is_some_and(|s| s.program == j && !s.walk_on);
        let committed = p.cal.commit.is_some_and(|c| c.0 == j && !c.2);
        let early = p.cal.early_offers.iter().find(|o| o.0 == j).map(|o| o.1);
        let offered = p.cal.offers.contains(&j);
        if !(signed || committed || early.is_some() || offered) {
            continue;
        }
        let rank = if signed {
            0
        } else if committed {
            1
        } else {
            2
        };
        found.push((rank, 5 - recruiting_stars(p).min(5), early.unwrap_or(today), p.player.id));
    }
    found.sort();
    let desk = &mut w.front_office.college;
    for (_, _, when, id) in found {
        if desk.board.iter().any(|t| t.id == id) {
            continue;
        }
        desk.board.push(Target {
            id,
            added: when,
            by: Source::Inherited,
            offered: Some(when),
            pulled: None,
            priority: false,
            nil: None,
            why: Some("Your staff offered him before you took over the board.".into()),
            seen: Standing::Open,
        });
    }
    desk.seeded = true;
    let ids: Vec<u32> = desk.board.iter().map(|t| t.id).collect();
    let at = indices(w, &ids);
    for t in w.front_office.college.board.iter_mut() {
        if let Some(&i) = at.get(&t.id) {
            t.seen = standing(w_people(&w.people, i), j);
        }
    }
}

fn w_people(people: &[crate::world::Person], i: usize) -> &crate::world::Person {
    &people[i]
}

/// Where a recruit stands with program `j`.
pub fn standing(p: &crate::world::Person, j: usize) -> Standing {
    if p.stage == Stage::College || p.stage == Stage::Pro || p.stage == Stage::Declared {
        if p.cal.signed.is_none() && p.college == Some(j) && p.path.len() <= 1 {
            return Standing::Enrolled;
        }
        if p.cal.signed.is_none() && p.college.is_some_and(|c| c != j) && !is_juco_recruit(p) {
            return Standing::Gone;
        }
    }
    if p.stage == Stage::Done {
        return Standing::Gone;
    }
    match (p.cal.signed, p.cal.commit) {
        (Some(s), _) if s.program == j => Standing::SignedHere,
        (Some(_), _) => Standing::SignedElsewhere,
        (_, Some(c)) if c.0 == j => Standing::CommittedHere,
        (_, Some(_)) => Standing::CommittedElsewhere,
        _ => Standing::Open,
    }
}

/// A junior college player still looking to move up: a recruit.
pub(crate) fn is_juco_recruit(p: &crate::world::Person) -> bool {
    p.stage == Stage::College && p.juco && p.cal.signed.is_none()
}

/// The day's changes on your board, in a few inbox lines.
fn scan_board(w: &mut World, j: usize, d: Date) {
    let ids: Vec<u32> = w.front_office.college.board.iter().map(|t| t.id).collect();
    if ids.is_empty() {
        return;
    }
    let at = indices(w, &ids);
    // (mine first, his stars, the recruit, the other school, the line)
    let mut lines: Vec<(u8, u8, usize, Option<usize>, String)> = Vec::new();
    let mut signed_here: Vec<(u8, usize, String)> = Vec::new();
    let mut changes: Vec<(usize, Standing)> = Vec::new();
    for (k, t) in w.front_office.college.board.iter().enumerate() {
        let Some(&i) = at.get(&t.id) else {
            changes.push((k, Standing::Gone));
            continue;
        };
        let p = &w.people[i];
        let now = standing(p, j);
        if now == t.seen {
            continue;
        }
        changes.push((k, now));
        let tracked = t.offered.is_some() || t.priority;
        let stars = recruiting_stars(p);
        let who = format!("{} ({}, {})", p.name, p.player.position, stars_text(stars));
        let other = |c: usize| w.program_name(c).to_string();
        let commit = p.cal.commit.map(|c| c.0);
        let signed = p.cal.signed.map(|s| s.program);
        match (t.seen, now) {
            (_, Standing::CommittedHere) => lines.push((0, stars, i, None, format!("{who} commits to you."))),
            (_, Standing::SignedHere) => signed_here.push((stars, i, who.clone())),
            (Standing::CommittedHere, Standing::Open) => {
                lines.push((0, stars, i, None, format!("{who} decommitted from you and reopened his recruitment.")))
            }
            (Standing::CommittedHere, Standing::CommittedElsewhere) => lines.push((
                0,
                stars,
                i,
                commit,
                format!("{who} flipped his commitment to {}.", other(commit.unwrap())),
            )),
            (Standing::CommittedHere, Standing::SignedElsewhere) => {
                lines.push((0, stars, i, signed, format!("{who} flipped and signed with {}.", other(signed.unwrap()))))
            }
            (_, Standing::CommittedElsewhere) if tracked => {
                lines.push((1, stars, i, commit, format!("{who} commits to {}.", other(commit.unwrap()))))
            }
            (_, Standing::SignedElsewhere) if tracked => {
                lines.push((1, stars, i, signed, format!("{who} signs with {}.", other(signed.unwrap()))))
            }
            _ => {}
        }
    }
    for (k, s) in changes {
        w.front_office.college.board[k].seen = s;
    }
    // The best names first, every name and school linked.
    if !signed_here.is_empty() {
        signed_here.sort_by_key(|s| std::cmp::Reverse(s.0));
        let n = signed_here.len();
        let shown: Vec<&(u8, usize, String)> = signed_here.iter().take(6).collect();
        let text = if n == 1 {
            format!("{} signed with you.", signed_here[0].2)
        } else {
            let more = if n > 6 { format!(", and {} more", n - 6) } else { String::new() };
            format!(
                "{} signed with you today: {}{more}.",
                count(n, "recruit"),
                shown.iter().map(|s| s.2.as_str()).collect::<Vec<_>>().join(", ")
            )
        };
        let who: Vec<usize> = shown.iter().map(|s| s.1).collect();
        note(w, text.clone(), None);
        super::tell_about(w, d, text, false, &who);
    }
    lines.sort_by_key(|l| (l.0, std::cmp::Reverse(l.1)));
    let (mine_lines, other_lines): (Vec<_>, Vec<_>) = lines.into_iter().partition(|l| l.0 == 0);
    for (_, _, i, school, text) in mine_lines {
        note(w, text.clone(), Some(w.people[i].player.id));
        super::tell_links(w, d, text, false, &[i], &school.into_iter().collect::<Vec<_>>());
    }
    if !other_lines.is_empty() {
        let n = other_lines.len();
        let shown = &other_lines[..n.min(if n <= 2 { 2 } else { 4 })];
        let text = if n <= 2 {
            shown.iter().map(|l| l.4.as_str()).collect::<Vec<_>>().join(" ")
        } else {
            format!(
                "{} you offered chose other schools: {}",
                count(n, "recruit"),
                shown.iter().map(|l| l.4.trim_end_matches('.')).collect::<Vec<_>>().join("; ")
            ) + if n > 4 { "; and more." } else { "." }
        };
        let who: Vec<usize> = shown.iter().map(|l| l.2).collect();
        let schools: Vec<usize> = shown.iter().filter_map(|l| l.3).collect();
        super::tell_links(w, d, text, false, &who, &schools);
    }
}

/// Your portal targets' decisions, and your own players entering.
fn scan_portal(w: &mut World, j: usize, d: Date) {
    // Your players who entered.
    let mut entered: Vec<(u32, String)> = Vec::new();
    for &i in &w.programs[j].roster {
        let p = &w.people[i];
        if p.stage == Stage::College
            && p.college == Some(j)
            && let Some(e) = p.cal.portal
            && e.entered <= d
            && !w.front_office.college.left.contains(&p.player.id)
        {
            entered.push((p.player.id, format!("{} ({}, {})", p.name, p.player.position, e.why)));
        }
    }
    if !entered.is_empty() {
        let n = entered.len();
        let names: Vec<String> = entered.iter().map(|e| e.1.clone()).collect();
        w.front_office.college.left.extend(entered.iter().map(|e| e.0));
        let text = if n == 1 {
            format!("{} entered the transfer portal.", names[0])
        } else {
            let more = if n > 5 { format!(", and {} more", n - 5) } else { String::new() };
            format!("{n} of your players entered the transfer portal: {}{more}.", names[..n.min(5)].join(", "))
        };
        tell(w, d, text, false);
    }
    // Your targets who decided.
    let ids: Vec<u32> = w.front_office.college.portal.iter().filter(|t| !t.settled).map(|t| t.id).collect();
    if ids.is_empty() {
        return;
    }
    let at = indices(w, &ids);
    let mut lines = Vec::new();
    let mut settle = Vec::new();
    for t in w.front_office.college.portal.iter().filter(|t| !t.settled) {
        let Some(&i) = at.get(&t.id) else {
            settle.push(t.id);
            continue;
        };
        let p = &w.people[i];
        if p.cal.portal.is_some() {
            continue;
        }
        let who = format!("{} ({})", p.name, p.player.position);
        if p.stage == Stage::College && p.college == Some(j) && p.path.last() == Some(&j) && t.from != j {
            lines.push(format!("{who} transfers to you from {}.", w.program_name(t.from)));
            settle.push(t.id);
        } else if p.stage == Stage::College && p.college.is_some_and(|c| c != t.from) {
            let to = w.program_name(p.college.unwrap()).to_string();
            if t.offered.is_some() {
                lines.push(format!("{who} chose {to} over your offer."));
            }
            settle.push(t.id);
        } else if p.stage == Stage::Done {
            settle.push(t.id);
        } else if p.stage == Stage::College && p.college == Some(t.from) && p.transfers == 0 {
            // Back at his old school, or never entered: nothing to say yet.
            if w.dates.start == d {
                settle.push(t.id);
            }
        }
    }
    for t in w.front_office.college.portal.iter_mut() {
        if settle.contains(&t.id) {
            t.settled = true;
        }
    }
    for text in lines {
        note(w, text.clone(), None);
        tell(w, d, text, false);
    }
}

/// Reminders before your level's days: the night before signing opens, and
/// the deals before July 1 when the money is yours.
fn reminders(w: &mut World, j: usize, d: Date) {
    let y = w.dates.clone();
    let tomorrow = d.add(1);
    let signing = [
        (y.early_signing, "The early signing period opens tomorrow"),
        (Some(y.signing_day), "National signing day is tomorrow"),
    ];
    for (day, what) in signing {
        if day == Some(tomorrow) && mine(w, Duty::Board).is_some() {
            let o = rec::class_outlook(w, j);
            let desk = &w.front_office.college;
            let ids: Vec<u32> =
                desk.board.iter().filter(|t| t.offered.is_some() && t.pulled.is_none()).map(|t| t.id).collect();
            let at = indices(w, &ids);
            let here = ids
                .iter()
                .filter_map(|id| at.get(id))
                .filter(|&&i| matches!(standing(&w.people[i], j), Standing::CommittedHere))
                .count();
            let open = ids.len().saturating_sub(here);
            tell(
                w,
                d,
                format!(
                    "{what}. {} committed, {} still open, {} left in the class.",
                    count(here, "recruit"),
                    count(open, "offer"),
                    count(o.slots, "place")
                ),
                false,
            );
        }
    }
    if ncaa::nil_era(d.year()) && d == Date::ymd(d.year(), 6, 15) && nil_yours(w) {
        let desk = &w.front_office.college;
        if desk.deals_year != d.year() || desk.deals.is_empty() {
            tell(
                w,
                d,
                "The collective pays your roster on July 1. Set your players' NIL deals, or hand NIL to your staff."
                    .into(),
                true,
            );
        }
    }
}

/// A new football year: past visits, settled transfers and the class that
/// enrolled leave the desk.
fn prune(w: &mut World) {
    let today = w.today;
    let desk = &mut w.front_office.college;
    desk.visits.retain(|v| v.date >= today.add(-200));
    desk.portal.retain(|t| !t.settled || t.added >= today.add(-120));
    desk.talks.clear();
    desk.left.clear();
    desk.board.retain(|t| !matches!(t.seen, Standing::Gone | Standing::Enrolled | Standing::SignedElsewhere));
}

// --- Visits ---------------------------------------------------------------------------------

/// Your home games' Saturdays this season (on campus: a neutral site is no
/// place for an official visit).
fn home_saturdays(w: &World, j: usize) -> Vec<Date> {
    home_games(w, j).into_iter().map(|g| g.0).collect()
}

/// Your home games on campus this season, with the opponent, in order.
pub fn home_games(w: &World, j: usize) -> Vec<(Date, usize)> {
    let mut v: Vec<(Date, usize)> = w.college.as_ref().map_or_else(Vec::new, |c| {
        c.games.iter().filter(|g| g.home == j && g.site.is_none()).map(|g| (g.date, g.away)).collect()
    });
    v.sort();
    v
}

/// Why a visit can't happen on a day, in the NCAA's words; None when it can.
pub fn visit_rule(w: &World, i: usize, kind: VisitKind, d: Date) -> Option<String> {
    let Some(j) = super::program(w) else { return Some("You have no college job.".into()) };
    let p = &w.people[i];
    let y = &w.dates;
    if d <= w.today {
        return Some("That day has passed.".into());
    }
    if d > y.super_bowl {
        return Some("Visits are set within this football year.".into());
    }
    if !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) && !is_juco_recruit(p) {
        return Some("He isn't a recruit.".into());
    }
    if p.cal.signed.is_some() {
        return Some("He has signed: no more visits.".into());
    }
    let period = recruiting_period(y, d);
    let sign_year = rec::next_class(w);
    let senior = p.class_of <= sign_year;
    let taken = |v: Visit| p.cal.visits.iter().filter(|x| x.2 == v).count();
    let planned = |k: VisitKind| {
        w.front_office
            .college
            .visits
            .iter()
            .filter(|x| x.id == p.player.id && x.kind == k && x.state == VisitState::Set)
            .count()
    };
    match kind {
        VisitKind::Official => {
            if d.weekday() != Weekday::Fri {
                return Some("An official visit is a weekend: it starts on a Friday.".into());
            }
            if !period.official_visits() {
                return Some(format!("No official visits in a {}.", period.name()));
            }
            let junior_officials = w.changes.rules(y.season).junior_officials;
            if !rec::official_visit_allowed(y.season, d, p.class_of, junior_officials) {
                return Some(if p.class_of > sign_year + 1 || (p.class_of == sign_year + 1 && !junior_officials) {
                    "Only the class signing next may take official visits.".to_string()
                } else if p.class_of == sign_year + 1 {
                    "Juniors may take official visits only from April 1 to June 30.".to_string()
                } else {
                    "Seniors take official visits from September 1.".to_string()
                });
            }
            if p.cal.visits.iter().any(|v| v.0 == j && v.2 == Visit::Official) || planned(kind) > 0 {
                return Some("One official visit to a school, by NCAA rule.".into());
            }
            if taken(Visit::Official) + planned(kind) >= 5 {
                return Some("He has used his five official visits.".into());
            }
            if (9..=11).contains(&d.month()) && !home_saturdays(w, j).contains(&d.add(1)) {
                return Some("In the fall an official visit is built around a home game.".into());
            }
        }
        VisitKind::Unofficial => {
            if !period.unofficial_visits() {
                return Some("No visits at all in a dead period.".into());
            }
            if d.weekday() != Weekday::Sat {
                return Some("Families come on a Saturday.".into());
            }
            if (9..=11).contains(&d.month()) && !home_saturdays(w, j).contains(&d) {
                return Some("In the fall he comes for a home game.".into());
            }
            if planned(kind) > 0 {
                return Some("He already has an unofficial visit set with you.".into());
            }
        }
        VisitKind::Home => {
            if period != Period::Contact {
                return Some(format!("Home visits only in a contact period (this is a {}).", period.name()));
            }
            if !senior {
                return Some("The head coach visits seniors at home.".into());
            }
            if d.weekday() == Weekday::Sun {
                return Some("Not on a Sunday.".into());
            }
            if p.cal.visits.iter().any(|v| v.0 == j && v.2 == Visit::Home) || planned(kind) > 0 {
                return Some("One home visit by the head coach in the cycle.".into());
            }
        }
        VisitKind::DropIn => {
            if period != Period::Evaluation {
                return Some(format!(
                    "Coaches watch practice only in an evaluation period (this is a {}).",
                    period.name()
                ));
            }
            if d.weekday() != Weekday::Fri {
                return Some("Your coaches drop in on Fridays, game day at his school.".into());
            }
            if p.class_of > sign_year + 1 {
                return Some("Your coaches watch juniors and seniors.".into());
            }
            if planned(kind) > 0 {
                return Some("A drop-in is already set.".into());
            }
        }
    }
    None
}

/// The days a visit could be set in the coming weeks, each with a word.
pub fn visit_dates(w: &World, id: u32, kind: VisitKind) -> Vec<(Date, String)> {
    let Some(i) = super::index_of(w, id) else { return Vec::new() };
    let Some(j) = super::program(w) else { return Vec::new() };
    let homes = home_saturdays(w, j);
    let opp =
        |d: Date| w.college.as_ref().and_then(|c| c.games.iter().find(|g| g.home == j && g.date == d)).map(|g| g.away);
    let mut v = Vec::new();
    let mut d = w.today.add(1);
    let end = w.dates.super_bowl.min(w.today.add(150));
    while d <= end && v.len() < 24 {
        if visit_rule(w, i, kind, d).is_none() {
            let sat = if kind == VisitKind::Official { d.add(1) } else { d };
            let word = match opp(sat) {
                Some(o) if homes.contains(&sat) => format!("home game against {}", w.program_name(o)),
                _ => recruiting_period(&w.dates, d).name().to_string(),
            };
            v.push((d, word));
        }
        d = d.add(1);
    }
    v
}

/// Whether he says yes, and why, in words: by what he thinks of you
/// against the others recruiting him (the same appeal signing day uses).
fn answer(w: &World, i: usize, j: usize, kind: VisitKind, d: Date) -> Result<String, String> {
    let p = &w.people[i];
    let offered = offered_here(w, p.player.id);
    let mut schools: Vec<usize> = p.cal.offers.iter().copied().chain(p.cal.early_offers.iter().map(|o| o.0)).collect();
    if let Some(c) = p.cal.commit {
        schools.push(c.0);
    }
    schools.retain(|&x| x != j);
    schools.sort_unstable();
    schools.dedup();
    let mut ranked: Vec<(f64, usize)> = schools.iter().map(|&x| (rec::appeal(w, i, x, None), x)).collect();
    let mine = rec::appeal(w, i, j, promise_of(w, p.player.id)) - if offered { 0.0 } else { 0.12 };
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    let ahead: Vec<usize> = ranked.iter().filter(|r| r.0 > mine).map(|r| r.1).collect();
    let draw = key(w, 0x1515 ^ kind as u64, p.player.id as u64 ^ (d.0 as u64) << 32);
    let committed_elsewhere = p.cal.commit.filter(|c| c.0 != j);
    let firm = committed_elsewhere
        .is_some_and(|c| rec::appeal(w, i, c.0, None) > mine + 0.5 * rec::commitment_hold(w, i) && draw > 0.25);
    let name = |x: usize| w.program_name(x).to_string();
    let first = p.name.split_whitespace().next().unwrap_or("He").to_string();
    match kind {
        VisitKind::Official => {
            if firm {
                return Err(format!(
                    "{first} is firm with {}: he won't spend a visit on you.",
                    name(committed_elsewhere.unwrap().0)
                ));
            }
            let used = p.cal.visits.iter().filter(|v| v.2 == Visit::Official).count();
            let left = 5usize.saturating_sub(used);
            let unvisited_ahead =
                ahead.iter().filter(|&&x| !p.cal.visits.iter().any(|v| v.0 == x && v.2 == Visit::Official)).count();
            if unvisited_ahead < left || (unvisited_ahead == left && draw < 0.4) {
                Ok(format!("{first} accepted: the weekend of {} is set.", d.pretty()))
            } else {
                let keep: Vec<String> = ahead.iter().take(2).map(|&x| name(x)).collect();
                Err(format!(
                    "{first} said no: he's keeping his {} for {}.",
                    if left == 1 { "last official visit".to_string() } else { format!("{left} official visits") },
                    keep.join(" and ")
                ))
            }
        }
        VisitKind::Unofficial => {
            let miles = geo::miles(p.home_state, w.programs[j].state);
            let money = p.bg.money;
            let far = (miles / 700.0) * (1.2 - money);
            if !firm && ahead.len() <= 5 && draw > far.min(0.95) {
                Ok(format!("{first} and his family will come on {}, at their own cost.", d.pretty()))
            } else if firm {
                Err(format!("{first} is firm with {} and won't make the trip.", name(committed_elsewhere.unwrap().0)))
            } else {
                Err(format!("{first}'s family can't make the trip ({:.0} miles, at their own cost).", miles))
            }
        }
        VisitKind::Home => {
            if !offered {
                return Err("A home visit follows an offer: offer him first.".into());
            }
            if firm && draw > 0.6 {
                Err(format!(
                    "{first}'s family thanked you, but he's firm with {}.",
                    name(committed_elsewhere.unwrap().0)
                ))
            } else {
                Ok(format!("{first}'s family will host you on {}.", d.pretty()))
            }
        }
        VisitKind::DropIn => Ok(format!("Your coaches will watch {first} practice on {}.", d.pretty())),
    }
}

/// Sets a visit, if the rules allow it and he says yes.
pub fn set_visit(w: &mut World, id: u32, kind: VisitKind, d: Date) -> Result<String, String> {
    let j = need(w, Duty::Visits)?;
    let i = super::index_of(w, id).ok_or("No such recruit.")?;
    if let Some(why) = visit_rule(w, i, kind, d) {
        return Err(why);
    }
    let reply = answer(w, i, j, kind, d);
    let (state, text) = match &reply {
        Ok(t) => (VisitState::Set, t.clone()),
        Err(t) => (VisitState::Declined, t.clone()),
    };
    w.front_office.college.visits.push(Plan { id, kind, date: d, state, note: text.clone() });
    let name = w.people[i].name.clone();
    note(w, format!("{} for {name} on {}: {text}", kind.name(), d.pretty()), Some(id));
    reply
}

pub fn cancel_visit(w: &mut World, id: u32, kind: VisitKind) -> Result<String, String> {
    need(w, Duty::Visits)?;
    let v = w
        .front_office
        .college
        .visits
        .iter_mut()
        .find(|v| v.id == id && v.kind == kind && v.state == VisitState::Set)
        .ok_or("No such visit is set.")?;
    v.state = VisitState::Off;
    v.note = "You called it off.".into();
    Ok(format!("{} called off.", kind.name()))
}

/// The day's visits happen: each on the rules of the day, with how it went.
fn run_visits(w: &mut World, j: usize, d: Date) {
    let due: Vec<usize> = w
        .front_office
        .college
        .visits
        .iter()
        .enumerate()
        .filter(|(_, v)| v.state == VisitState::Set && v.date == d)
        .map(|(k, _)| k)
        .collect();
    for k in due {
        let (id, kind) = (w.front_office.college.visits[k].id, w.front_office.college.visits[k].kind);
        let Some(i) = super::index_of(w, id) else {
            w.front_office.college.visits[k].state = VisitState::Off;
            continue;
        };
        // The rules of the day (a signature or a closed window ends it).
        let p = &w.people[i];
        let off = if p.cal.signed.is_some() {
            Some("He signed before the day came.".to_string())
        } else if !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) && !is_juco_recruit(p) {
            Some("He is no longer a recruit.".to_string())
        } else if !recruiting_period(&w.dates, d).unofficial_visits() {
            Some("A dead period closed it.".to_string())
        } else {
            None
        };
        if let Some(why) = off {
            let v = &mut w.front_office.college.visits[k];
            v.state = VisitState::Off;
            v.note = why;
            continue;
        }
        // How it went: his own feel for the school shows on campus (the
        // same feel his signing day weighs), and a head coach's visit
        // shows the room.
        let lean = feel_word(w, i, j);
        let p = &mut w.people[i];
        p.cal.visits.push((j, d, kind.visit()));
        let age = p.row_age();
        let school = w.programs[j].name(w.options.names).to_string();
        let (name, stars, pos) = (p.name.clone(), recruiting_stars(p), p.player.position);
        p.note(match kind {
            VisitKind::Official => format!("At {age} he took an official visit to {school}."),
            VisitKind::Unofficial => format!("At {age} he visited {school} unofficially, with his family."),
            VisitKind::Home => format!("At {age} {school}'s head coach visited him at home."),
            VisitKind::DropIn => format!("At {age} {school}'s coaches watched him practice."),
        });
        let text = match kind {
            VisitKind::Official => format!("{name} ({pos}) spent his official visit with you. {lean}"),
            VisitKind::Unofficial => format!("{name} ({pos}) came to campus with his family. {lean}"),
            VisitKind::Home => format!("You sat with {name}'s family at home. {lean}"),
            VisitKind::DropIn => format!("Your coaches watched {name} ({pos}) practice; his file has a new live look."),
        };
        let v = &mut w.front_office.college.visits[k];
        v.state = VisitState::Done;
        v.note = text.clone();
        tell(w, d, text, false);
        if kind == VisitKind::Official && stars >= 4 {
            let text = format!("{stars}-star {name} takes an official visit to {school}.");
            w.calendar.post(
                Event::new(d, Level::Recruiting, Kind::Visit, Loudness::Small, text)
                    .program(j)
                    .rule("NCAA: official visits"),
            );
        }
        match kind {
            VisitKind::Official => w.log.official_visits += 1,
            VisitKind::Unofficial => w.log.unofficial_visits += 1,
            _ => w.log.drop_ins += 1,
        }
    }
}

/// How a visit went, in his words: his own feel for the school.
fn feel_word(w: &World, i: usize, j: usize) -> String {
    let p = &w.people[i];
    let seed = w.options.seed as u64 ^ (p.class_of as u64) << 20;
    let z = crate::world::season::key_normal(seed ^ 77 ^ j as u64, p.player.id as u64);
    match z {
        z if z > 1.0 => "He came away sold on the place.".into(),
        z if z > 0.3 => "He liked what he saw.".into(),
        z if z > -0.3 => "He was polite, hard to read.".into(),
        z if z > -1.0 => "He left lukewarm.".into(),
        _ => "It didn't click for him.".into(),
    }
}

// --- Board, offers and the money ------------------------------------------------------------

fn offered_here(w: &World, id: u32) -> bool {
    w.front_office.college.board.iter().any(|t| t.id == id && t.offered.is_some() && t.pulled.is_none())
}

fn promise_of(w: &World, id: u32) -> Option<f64> {
    if !nil_yours(w) {
        return None;
    }
    w.front_office.college.board.iter().find(|t| t.id == id).and_then(|t| t.nil)
}

/// Whether a person can be recruited by you now.
fn recruitable(w: &World, i: usize) -> Result<(), String> {
    let p = &w.people[i];
    let sign_year = rec::next_class(w);
    let hs = matches!(p.stage, Stage::HighSchool | Stage::PrepSchool);
    if !(hs || is_juco_recruit(p)) {
        return Err(format!("{} isn't a high school or junior college recruit.", p.name));
    }
    if hs && p.class_of > sign_year + 2 {
        return Err("Too young: staffs recruit from the sophomore class up.".into());
    }
    if hs && p.class_of < sign_year {
        return Err("His class has signed.".into());
    }
    Ok(())
}

pub fn add(w: &mut World, id: u32) -> Result<String, String> {
    super::program(w).ok_or("You have no college job.")?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    recruitable(w, i)?;
    if w.front_office.college.board.iter().any(|t| t.id == id) {
        return Err("He's on your board already.".into());
    }
    let today = w.today;
    let seen = standing(&w.people[i], super::program(w).unwrap());
    w.front_office.college.board.push(Target {
        id,
        added: today,
        by: Source::You,
        offered: None,
        pulled: None,
        priority: false,
        nil: None,
        why: None,
        seen,
    });
    Ok(format!("{} is on your board.", w.people[i].name))
}

pub fn remove(w: &mut World, id: u32) -> Result<String, String> {
    let desk = &w.front_office.college;
    let t = desk.board.iter().find(|t| t.id == id).ok_or("He isn't on your board.")?;
    if t.offered.is_some() && t.pulled.is_none() {
        return Err("Pull his offer first.".into());
    }
    w.front_office.college.board.retain(|t| t.id != id);
    Ok("Taken off your board.".into())
}

/// Moves a recruit to place `to` (0 = top) on your board.
pub fn reorder(w: &mut World, id: u32, to: usize) -> Result<String, String> {
    need(w, Duty::Board)?;
    let b = &mut w.front_office.college.board;
    let k = b.iter().position(|t| t.id == id).ok_or("He isn't on your board.")?;
    let t = b.remove(k);
    let to = to.min(b.len());
    b.insert(to, t);
    Ok(format!("Moved to No. {}.", to + 1))
}

/// The most targets the head coach can give his own time (proposed, not
/// confirmed: about a class and a half; a real head coach closes on the
/// top of the board).
pub const PRIORITY_MAX: usize = 35;

pub fn set_priority(w: &mut World, id: u32, on: bool) -> Result<String, String> {
    need(w, Duty::Board)?;
    let n = w.front_office.college.board.iter().filter(|t| t.priority && t.id != id).count();
    if on && n >= PRIORITY_MAX {
        return Err(format!("You can give your own time to {PRIORITY_MAX} targets at most."));
    }
    let t = w.front_office.college.board.iter_mut().find(|t| t.id == id).ok_or("He isn't on your board.")?;
    t.priority = on;
    Ok(if on { "He's one of yours: you'll give him your own time." } else { "His position coach leads." }.into())
}

/// A scholarship offer. A recruit off your board goes on it.
pub fn offer(w: &mut World, id: u32) -> Result<String, String> {
    let j = need(w, Duty::Board)?;
    let text = offer_at(w, j, id, Source::You, None)?;
    // Your film look, and any coach of yours it overrules (SK25, SK28).
    super::chain::on_offer(w, id);
    Ok(text)
}

/// An offer the chain brought you, signed (SK26): it goes out whoever
/// holds the board, marked as your staff's with the coach's word.
pub(crate) fn offer_by_chain(w: &mut World, j: usize, id: u32, why: String) -> Result<String, String> {
    offer_at(w, j, id, Source::Coordinator, Some(why))
}

fn offer_at(w: &mut World, j: usize, id: u32, by: Source, why: Option<String>) -> Result<String, String> {
    let i = super::index_of(w, id).ok_or("No such player.")?;
    recruitable(w, i)?;
    let p = &w.people[i];
    if p.cal.signed.is_some_and(|s| s.program != j) {
        return Err(format!(
            "He signed with {}: a signed letter binds him.",
            w.program_name(p.cal.signed.unwrap().program)
        ));
    }
    if p.cal.signed.is_some() {
        return Err("He has signed with you.".into());
    }
    let today = w.today;
    if !w.front_office.college.board.iter().any(|t| t.id == id) {
        add(w, id)?;
    }
    let t = w.front_office.college.board.iter_mut().find(|t| t.id == id).unwrap();
    if t.offered.is_some() && t.pulled.is_none() {
        return Err("He holds your offer already.".into());
    }
    t.offered = Some(today);
    t.pulled = None;
    t.by = by;
    t.why = why;
    // He hears it now: the offer is on his list, and an early one is remembered.
    let sign_year = rec::next_class(w);
    let p = &mut w.people[i];
    if !p.cal.offers.contains(&j) {
        p.cal.offers.push(j);
    }
    let first = p.cal.early_offers.is_empty() && p.cal.offers.len() <= 1;
    if p.class_of > sign_year && !p.cal.early_offers.iter().any(|o| o.0 == j) {
        p.cal.early_offers.push((j, today));
    }
    let school = w.programs[j].name(w.options.names).to_string();
    let age = p.row_age();
    if first {
        p.note(format!("At {age} he got his first scholarship offer, from {school}."));
    } else {
        p.note(format!("At {age} {school} offered him a scholarship."));
    }
    let (name, pos, stars) = (p.name.clone(), p.player.position, recruiting_stars(p));
    note(w, format!("Offered {name} ({pos}) a scholarship."), Some(id));
    if stars >= 4 {
        w.calendar.post(
            Event::new(
                today,
                Level::Recruiting,
                Kind::Offer,
                Loudness::Small,
                format!("{school} offers {stars}-star {pos} {name}."),
            )
            .program(j),
        );
    }
    let gpa = rec::eligibility_gpa(w.people[i].bg.gpa, w.people[i].cal.test_score, w.people[i].class_of);
    let warn = if rec::ncaa_tier(w.programs[j].division, gpa) == rec::Eligibility::NonQualifier {
        " His transcript is under the NCAA line today: he can't sign unless it comes up."
    } else {
        ""
    };
    Ok(format!("Offer out to {name}.{warn}"))
}

/// Pulls an offer. A commitment goes with it.
pub fn pull(w: &mut World, id: u32) -> Result<String, String> {
    let j = need(w, Duty::Board)?;
    let text = pull_at(w, j, id)?;
    super::chain::on_pull(w, id);
    Ok(text)
}

/// A pull your staff asked for, signed (SK26).
pub(crate) fn pull_by_chain(w: &mut World, j: usize, id: u32) -> Result<String, String> {
    pull_at(w, j, id)
}

fn pull_at(w: &mut World, j: usize, id: u32) -> Result<String, String> {
    let i = super::index_of(w, id).ok_or("No such player.")?;
    if w.people[i].cal.signed.is_some_and(|s| s.program == j) {
        return Err("He signed his letter with you: only a release frees him.".into());
    }
    let today = w.today;
    let t = w.front_office.college.board.iter_mut().find(|t| t.id == id).ok_or("He isn't on your board.")?;
    if t.offered.is_none() || t.pulled.is_some() {
        return Err("He holds no offer from you.".into());
    }
    t.pulled = Some(today);
    t.priority = false;
    let school = w.programs[j].name(w.options.names).to_string();
    let p = &mut w.people[i];
    p.cal.offers.retain(|&x| x != j);
    let committed = p.cal.commit.is_some_and(|c| c.0 == j);
    let age = p.row_age();
    let (name, hs) = (p.name.clone(), p.hs);
    let mut text = format!("Pulled the offer to {name}.");
    if committed {
        p.cal.commit = None;
        p.note(format!("At {age} {school} pulled his scholarship offer, and his commitment with it."));
        w.log.decommits += 1;
        // His high school coach remembers (a pipeline cools).
        if let Some(e) = w.programs[j].hs_pipeline.get_mut(&hs) {
            e.0 *= 0.85;
        }
        text = format!(
            "Pulled the offer to {name}; his commitment is over. His high school's staff won't forget it soon."
        );
        if recruiting_stars(&w.people[i]) >= 3 {
            let stars = recruiting_stars(&w.people[i]);
            w.calendar.post(
                Event::new(
                    today,
                    Level::Recruiting,
                    Kind::Commitment,
                    Loudness::Small,
                    format!("{school} pulls its offer to {stars}-star commit {name}."),
                )
                .program(j),
            );
        }
        if let Some(t) = w.front_office.college.board.iter_mut().find(|t| t.id == id) {
            t.seen = Standing::Open;
        }
    } else {
        p.note(format!("At {age} {school} pulled its scholarship offer."));
    }
    note(w, text.clone(), Some(id));
    Ok(text)
}

/// Your NIL promise to a recruit (None: the collective's usual offer).
pub fn set_promise(w: &mut World, id: u32, nil: Option<f64>) -> Result<String, String> {
    let j = super::program(w).ok_or("You have no college job.")?;
    if !ncaa::nil_era(w.today.year()) && !(w.today >= Date::ymd(ncaa::NIL_START, 7, 1)) {
        return Err("NIL deals are allowed from July 1, 2021.".into());
    }
    if w.front_office.owner(Duty::Nil) != Owner::You {
        return Err("Your collective sets the money now: take NIL back first.".into());
    }
    let i = super::index_of(w, id).ok_or("No such player.")?;
    let pos = w.people[i].player.position;
    let cap = ncaa::position_nil_max(pos);
    if let Some(x) = nil
        && (x < 0.0 || x > cap)
    {
        return Err(format!(
            "The most a {} gets is about {} a year (the real top of his market).",
            pos.name(),
            dollars(cap)
        ));
    }
    let b = budget(w, j);
    let current = w.front_office.college.board.iter().find(|t| t.id == id).and_then(|t| t.nil).unwrap_or(0.0);
    if let Some(x) = nil
        && x - current > b.left + 1.0
    {
        return Err(format!("That's more than the {} left in your NIL budget.", dollars(b.left.max(0.0))));
    }
    let t = w.front_office.college.board.iter_mut().find(|t| t.id == id).ok_or("He isn't on your board.")?;
    t.nil = nil;
    Ok(match nil {
        Some(x) => format!("Your NIL promise: {} a year.", dollars(x)),
        None => "The collective's usual offer stands.".into(),
    })
}

/// Your NIL money this year, and where it is going.
#[derive(Clone, Copy, Debug, Default)]
pub struct Budget {
    /// The collective's money and the school's revenue share together.
    pub total: f64,
    pub revenue_share: f64,
    pub collective: f64,
    /// Your roster's deals for the money year.
    pub roster: f64,
    /// Promised to recruits who hold your offer (paid once they enroll).
    pub recruits: f64,
    /// Offered to portal targets.
    pub portal: f64,
    pub left: f64,
}

pub fn budget(w: &World, j: usize) -> Budget {
    let prog = &w.programs[j];
    let year = w.today.year();
    let total = prog.collective.budget;
    let revenue_share = ncaa::revenue_share(prog, year);
    let desk = &w.front_office.college;
    // This money year's deals, for players still on the roster.
    let on: std::collections::HashSet<u32> = prog
        .roster
        .iter()
        .map(|&i| &w.people[i])
        .filter(|p| p.stage == crate::world::Stage::College && p.college == Some(j))
        .map(|p| p.player.id)
        .collect();
    let roster = if desk.deals_year == deals_year(w) {
        desk.deals.iter().filter(|d| on.contains(&d.0)).map(|d| d.1).sum::<f64>()
    } else {
        0.0
    };
    let recruits = desk.board.iter().filter(|t| t.offered.is_some() && t.pulled.is_none()).filter_map(|t| t.nil).sum();
    let portal = desk.portal.iter().filter(|t| t.offered.is_some() && !t.settled).filter_map(|t| t.nil).sum();
    Budget {
        total,
        revenue_share: revenue_share.min(total),
        collective: (total - revenue_share).max(0.0),
        roster,
        recruits,
        portal,
        left: total - roster - recruits - portal,
    }
}

/// A deal for one of your players for the money year starting July 1.
pub fn set_deal(w: &mut World, id: u32, amount: f64) -> Result<String, String> {
    let j = super::program(w).ok_or("You have no college job.")?;
    if w.front_office.owner(Duty::Nil) != Owner::You {
        return Err("Your collective sets the money now: take NIL back first.".into());
    }
    if !ncaa::nil_era(w.today.year()) && w.today < Date::ymd(ncaa::NIL_START, 6, 1) {
        return Err("NIL deals are allowed from July 1, 2021.".into());
    }
    let i = super::index_of(w, id).ok_or("No such player.")?;
    if w.people[i].college != Some(j) || w.people[i].stage != Stage::College {
        return Err("He isn't on your roster.".into());
    }
    let pos = w.people[i].player.position;
    let cap = ncaa::position_nil_max(pos) * 2.5;
    if amount < 0.0 || amount > cap {
        return Err(format!("A deal for a {} tops out near {} a year.", pos.name(), dollars(cap)));
    }
    let year = deals_year(w);
    let desk = &mut w.front_office.college;
    if desk.deals_year != year {
        desk.deals.clear();
        desk.deals_year = year;
    }
    let old = desk.deals.iter().find(|d| d.0 == id).map_or(0.0, |d| d.1);
    let b = budget(w, j);
    if amount - old > b.left + 1.0 {
        return Err(format!("That's more than the {} left in your NIL budget.", dollars(b.left.max(0.0))));
    }
    let desk = &mut w.front_office.college;
    desk.deals.retain(|d| d.0 != id);
    if amount > 0.0 {
        desk.deals.push((id, amount));
    }
    Ok(format!("{}: {} a year from July 1, {year}.", w.people[i].name, dollars(amount)))
}

/// The money year deals are for: this July's, or next July's once it has paid.
pub fn deals_year(w: &World) -> i32 {
    let y = w.today.year();
    if w.today >= Date::ymd(y, 7, 1) { y + 1 } else { y }
}

/// Fills every unset deal with the collective's usual split (a start you
/// can adjust).
pub fn usual_deals(w: &World, j: usize) -> Vec<(u32, f64)> {
    let prog = &w.programs[j];
    let budget = prog.collective.budget * 0.6 * crate::world::legacy::college::split_q(w, j, 0);
    let roster: Vec<usize> = prog
        .roster
        .iter()
        .copied()
        .filter(|&i| w.people[i].stage == Stage::College && w.people[i].college == Some(j))
        .collect();
    let value = |i: usize| {
        let p = &w.people[i];
        let role = match p.last_role {
            Some(crate::player::PlayingTime::Starter) => 3.0,
            Some(crate::player::PlayingTime::Rotation) => 1.2,
            _ => 0.3,
        };
        let pos = 1.0 + 0.25 * crate::career::position_value(p.player.position).max(-2.0);
        role * pos * (1.0 + 0.1 * p.stars as f64)
    };
    let total: f64 = roster.iter().map(|&i| value(i)).sum();
    roster.iter().map(|&i| (w.people[i].player.id, budget * value(i) / total.max(1e-9))).collect()
}

// --- The portal -----------------------------------------------------------------------------

/// A head start bought from a smaller school (C15): your offer waits for
/// him in the portal, when the portal is yours to work.
pub(crate) fn head_start(w: &mut World, id: u32) -> Result<(), String> {
    need(w, Duty::Portal)?;
    let today = w.today;
    let t = portal_target(w, id)?;
    t.offered = Some(today);
    Ok(())
}

fn portal_target(w: &mut World, id: u32) -> Result<&mut PortalTarget, String> {
    let j = super::program(w).ok_or("You have no college job.")?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    if p.stage != Stage::College {
        return Err(format!("{} isn't a college player.", p.name));
    }
    if p.college == Some(j) {
        return Err("He plays for you.".into());
    }
    let from = p.college.unwrap_or(0);
    let today = w.today;
    let desk = &mut w.front_office.college;
    if !desk.portal.iter().any(|t| t.id == id && !t.settled) {
        desk.portal.retain(|t| t.id != id);
        desk.portal.push(PortalTarget {
            id,
            added: today,
            offered: None,
            nil: None,
            contacted: None,
            leaked: false,
            from,
            settled: false,
        });
    }
    Ok(desk.portal.iter_mut().find(|t| t.id == id).unwrap())
}

pub fn portal_offer(w: &mut World, id: u32, nil: Option<f64>) -> Result<String, String> {
    need(w, Duty::Portal)?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    if w.people[i].cal.portal.is_none() {
        return Err("He isn't in the portal: an offer before he enters is tampering. Reach out quietly instead.".into());
    }
    let j = super::program(w).ok_or("You have no college job.")?;
    if nil.is_some() && !nil_yours(w) {
        return Err("Your collective sets the money now: take NIL back first.".into());
    }
    if let Some(x) = nil {
        let pos = w.people[i].player.position;
        let cap = ncaa::position_nil_max(pos) * 2.5;
        if x > cap {
            return Err(format!("A {} in the portal tops out near {} a year.", pos.name(), dollars(cap)));
        }
        let b = budget(w, j);
        let old =
            w.front_office.college.portal.iter().find(|t| t.id == id && !t.settled).and_then(|t| t.nil).unwrap_or(0.0);
        if x - old > b.left + 1.0 {
            return Err(format!("That's more than the {} left in your NIL budget.", dollars(b.left.max(0.0))));
        }
    }
    let today = w.today;
    let t = portal_target(w, id)?;
    t.offered = Some(today);
    t.nil = nil;
    let name = w.people[i].name.clone();
    note(w, format!("Offered portal entrant {name} a place."), Some(id));
    Ok(format!(
        "Offer out to {name}. He decides by {}.",
        w.people[i].cal.portal.map_or(String::new(), |e| e.decide_by.pretty())
    ))
}

pub fn portal_pull(w: &mut World, id: u32) -> Result<String, String> {
    let t = w.front_office.college.portal.iter_mut().find(|t| t.id == id && !t.settled).ok_or("No offer to pull.")?;
    t.offered = None;
    Ok("Offer pulled.".into())
}

pub fn portal_forget(w: &mut World, id: u32) -> Result<String, String> {
    w.front_office.college.portal.retain(|t| t.id != id || t.settled);
    Ok("Taken off your portal list.".into())
}

/// A quiet word with a player at another school before he enters
/// (tampering). Real staffs do it through a trainer, a 7v7 coach or a
/// family friend; the NCAA rarely proves it, and the world never punishes
/// it. It can leak (proposed, not confirmed: one time in eight).
pub fn portal_contact(w: &mut World, id: u32) -> Result<String, String> {
    let j = need(w, Duty::Portal)?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    if w.people[i].cal.portal.is_some() {
        return Err("He's in the portal: you may call him openly.".into());
    }
    if w.front_office.college.portal.iter().any(|t| t.id == id && t.contacted.is_some() && !t.settled) {
        return Err("You've reached out already.".into());
    }
    let today = w.today;
    let leak = key(w, 0x7A3F, id as u64 ^ (today.year() as u64) << 32) < 0.125;
    let name = w.people[i].name.clone();
    let from = w.people[i].college.unwrap_or(0);
    {
        let p = &mut w.people[i];
        // A door opened: his lean toward leaving grows a little, more when
        // he was already thinking of it.
        let f = 1.25;
        p.cal.lean = (p.cal.lean * f + 0.01).min(0.95);
        p.cal.lean_events.push((today, "another school reached out quietly", f));
    }
    let t = portal_target(w, id)?;
    t.contacted = Some(today);
    t.leaked = leak;
    let school = w.programs[j].name(w.options.names).to_string();
    note(w, format!("Reached out quietly to {name} ({}).", w.program_name(from)), Some(id));
    if leak {
        let text = format!(
            "Reports: {school} reached out to {name} of {} before he entered the portal.",
            w.program_name(from)
        );
        w.calendar.post(Event::new(today, Level::College, Kind::News, Loudness::Small, text.clone()).program(from));
        tell(w, today, format!("It leaked. {text} His coaches are furious; the NCAA has asked no questions."), false);
        return Ok(format!("Word reached {name}, but it leaked to the press."));
    }
    Ok(format!("Word reached {name} through a family friend. Nobody else knows."))
}

/// Sits down with one of your players before the winter window: your
/// relationships rating sways him the way an AI head coach sways his own
/// (the same formula, 0.75 to 1.25 on his lean).
pub fn keep_talk(w: &mut World, id: u32) -> Result<String, String> {
    let j = need(w, Duty::Portal)?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    if p.college != Some(j) || p.stage != Stage::College {
        return Err("He isn't on your roster.".into());
    }
    if p.cal.portal.is_some() {
        return Err("He's in the portal already.".into());
    }
    if w.front_office.college.talks.iter().any(|t| t.id == id) {
        return Err("You've sat down with him this year.".into());
    }
    let rel = w.career.as_ref().map_or(10.0, |c| w.coaches[c.coach].ability().relationships);
    let factor = (1.0 - 0.03 * (rel - 10.0)).clamp(0.75, 1.25);
    let first = p.name.split_whitespace().next().unwrap_or("He").to_string();
    let lean = p.cal.lean;
    let draw = key(w, 0x7A1C, id as u64);
    let role = p.last_role;
    let answer = if lean < 0.02 {
        format!("{first} looked surprised: he isn't going anywhere.")
    } else if factor < 0.95 && draw < 0.6 {
        format!("{first} heard you out and said he wants to finish what he started here.")
    } else if matches!(role, Some(crate::player::PlayingTime::Backup | crate::player::PlayingTime::Redshirt)) {
        format!("{first} was honest: he wants a place where he plays. He'll think it over.")
    } else if factor > 1.05 {
        format!("{first} didn't say much. It didn't feel like it landed.")
    } else {
        format!("{first} said he'd think it over.")
    };
    let today = w.today;
    w.front_office.college.talks.push(Talk { id, date: today, factor, answer: answer.clone() });
    note(w, format!("Sat down with {}: {answer}", w.people[i].name), Some(id));
    Ok(answer)
}

// --- Words --------------------------------------------------------------------------------

/// The best recruiter on your staff: the man who runs your board when you
/// hand it over.
pub fn coordinator(w: &World, j: usize) -> Option<usize> {
    let prog = &w.programs[j];
    [prog.oc, prog.dc].into_iter().chain(prog.position_coaches).flatten().max_by(|&a, &b| {
        w.coaches[a].ability().recruiting.total_cmp(&w.coaches[b].ability().recruiting).then(b.cmp(&a))
    })
}

pub fn coordinator_name(w: &World, j: usize) -> String {
    match coordinator(w, j) {
        Some(c) => format!("Your recruiting coordinator, {},", w.coaches[c].name),
        None => "Your recruiting coordinator".into(),
    }
}

pub(crate) fn stars_text(s: u8) -> String {
    if s == 0 { "unranked".into() } else { format!("{s}-star") }
}

fn count(n: usize, what: &str) -> String {
    if n == 1 { format!("1 {what}") } else { format!("{n} {what}s") }
}

/// Positions in board order.
pub const BOARD_ORDER: [Position; 14] = [
    Position::QB,
    Position::RB,
    Position::WR,
    Position::TE,
    Position::OT,
    Position::IOL,
    Position::EDGE,
    Position::DT,
    Position::LB,
    Position::CB,
    Position::S,
    Position::K,
    Position::P,
    Position::LS,
];

pub fn division_of(w: &World, j: usize) -> Division {
    w.programs[j].division
}

/// Your staff's class plan: the scholarship spots it would fill at each
/// position for the class signing next, how many the class can still take,
/// the roster's open places and the NCAA's limits.
#[derive(Clone, Debug)]
pub struct ClassOutlook {
    pub sign_year: i32,
    pub need: [usize; crate::positions::POSITION_COUNT],
    pub slots: usize,
    pub room: usize,
    pub limit: usize,
    pub signees: usize,
    pub whole_roster: bool,
}

pub fn outlook(w: &World, j: usize) -> ClassOutlook {
    rec::class_outlook(w, j)
}

pub fn next_class(w: &World) -> i32 {
    rec::next_class(w)
}

/// How a recruit sees the schools after him, as your staff hears it: each
/// school recruiting him with its appeal (your own with your promise), best
/// first. The numbers stay in the engine's units; the screens turn them
/// into words.
pub fn race(w: &World, id: u32) -> Vec<(usize, f64)> {
    let Some(i) = super::index_of(w, id) else { return Vec::new() };
    race_at(w, i)
}

pub fn race_at(w: &World, i: usize) -> Vec<(usize, f64)> {
    let p = &w.people[i];
    let j = super::program(w);
    let mut schools: Vec<usize> = p.cal.offers.iter().copied().chain(p.cal.early_offers.iter().map(|o| o.0)).collect();
    if let Some(c) = p.cal.commit {
        schools.push(c.0);
    }
    if let Some(s) = p.cal.signed {
        schools.push(s.program);
    }
    if let Some(j) = j
        && offered_here(w, p.player.id)
    {
        schools.push(j);
    }
    schools.sort_unstable();
    schools.dedup();
    let mut v: Vec<(usize, f64)> = schools
        .into_iter()
        .map(|x| {
            let nil = if Some(x) == j { promise_of(w, p.player.id) } else { None };
            (x, rec::appeal(w, i, x, nil))
        })
        .collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// What program `j` means to a recruit today, in `race`'s units (your own
/// program with your NIL promise).
pub fn appeal_of(w: &World, i: usize, j: usize) -> f64 {
    let nil = if super::program(w) == Some(j) { promise_of(w, w.people[i].player.id) } else { None };
    rec::appeal(w, i, j, nil)
}

/// What a school means to a recruit reason by reason (board 8), in
/// `race`'s units: close to home, playing time, NIL money, coaching and
/// development, winning and the name, academics, and his ties and feel.
/// Each is (appeal points, the school's score 0-1); they add up to its
/// appeal in `race`.
pub fn reasons(w: &World, i: usize, j: usize) -> [(f64, f64); 7] {
    let nil = if super::program(w) == Some(j) { promise_of(w, w.people[i].player.id) } else { None };
    rec::reason_parts(w, i, j, nil)
}

/// How much each of the six weighed reasons counts with whoever decides for
/// him. True values: a screen shows them only through your staff's read.
pub fn reason_weights(w: &World, i: usize) -> [f64; 6] {
    rec::reason_weights(&w.people[i])
}

/// How firmly a committed recruit holds, in the same units as `race`.
pub fn hold(w: &World, i: usize) -> f64 {
    rec::commitment_hold(w, i)
}

/// The collective's usual NIL offer to a recruit from your program.
pub fn usual_offer(w: &World, j: usize, id: u32) -> f64 {
    super::index_of(w, id).map_or(0.0, |i| rec::usual_nil_offer(w, j, i))
}

/// Whether his transcript meets the NCAA's line for your division today.
pub fn qualifies(w: &World, j: usize, i: usize) -> &'static str {
    let p = &w.people[i];
    let gpa = rec::eligibility_gpa(p.bg.gpa, p.cal.test_score, p.class_of);
    match rec::ncaa_tier(w.programs[j].division, gpa) {
        rec::Eligibility::Qualifier => "Qualifier",
        rec::Eligibility::AcademicRedshirt => "Academic redshirt",
        rec::Eligibility::NonQualifier => "Not qualified yet",
    }
}

/// The transfer windows around today: this season's winter window, the
/// spring one after it, last season's spring one, and a coaching change's
/// own window when one is set. Each: (opens, closes, name).
pub fn windows(w: &World) -> Vec<(Date, Date, &'static str)> {
    use crate::world::clock::{PortalWindow, portal_windows};
    let mut v = Vec::new();
    for season in [w.year - 1, w.year] {
        let rule = w.changes.rules(season).transfer;
        for (open, close, which) in portal_windows(season, rule) {
            let name = match which {
                PortalWindow::Winter => "Winter window",
                PortalWindow::Spring => "Spring window",
                PortalWindow::CoachChange => "Coaching change window",
                PortalWindow::Graduate => "Graduate transfers",
            };
            v.push((open, close, name));
        }
    }
    if let Some((open, close)) = w.coach_window {
        v.push((open, close, "Coaching change window"));
    }
    v.retain(|x| x.1 >= w.today.add(-30));
    v.sort_by_key(|x| x.0);
    v
}

/// The recruiting year's dates ahead (your level's calendar): commitment
/// waves, junior days, the first official visits, the NCAA's periods as
/// they change, the signing days and the money year. Each: (day, words).
pub fn key_dates(w: &World) -> Vec<(Date, String)> {
    let y = &w.dates;
    let s = y.season;
    let mut v: Vec<(Date, String)> = vec![
        (y.junior_days.0, "Junior days open: juniors visit campus on Saturdays".into()),
        (Date::ymd(s, 3, 20), "Commitment wave after junior days".into()),
        (Date::ymd(s, 4, 30), "Commitment wave after the spring evaluation period".into()),
        (Date::ymd(s, 6, 15), "Commitment wave after June camps and visits".into()),
        (Date::ymd(s, 8, 31), "Commitment wave before senior seasons".into()),
        (Date::ymd(s, 9, 1), "Seniors may take official visits".into()),
        (Date::ymd(s, 10, 31), "Commitment wave in the fall".into()),
        (y.signing_day, "National signing day".into()),
    ];
    if let Some(p) = &w.prev_dates
        && p.signing_day >= w.today
    {
        v.push((p.signing_day, "National signing day".into()));
    }
    if w.changes.rules(s).junior_officials {
        v.push((Date::ymd(s, 4, 1), "Juniors may take official visits (to June 30)".into()));
    }
    if let Some(e) = y.early_signing {
        v.push((e, "Early signing period opens".into()));
    }
    if ncaa::nil_era(s) {
        v.push((y.money_year, "The NIL money year starts: deals are paid".into()));
    }
    for (open, close, name) in windows(w) {
        v.push((open, format!("{name} opens (to {})", close.pretty())));
    }
    // The NCAA's periods as they change.
    let mut d = w.today;
    let mut last = recruiting_period(y, d);
    while d < y.super_bowl {
        d = d.add(1);
        let p = recruiting_period(y, d);
        if p != last {
            v.push((d, format!("{} begins", cap(p.name()))));
            last = p;
        }
    }
    v.retain(|x| x.0 >= w.today);
    v.sort_by_key(|x| x.0);
    v
}

/// The next day letters can be signed: the early period, then February's
/// national signing day.
pub fn next_signing(w: &World) -> Option<(Date, &'static str)> {
    let y = &w.dates;
    let mut v: Vec<(Date, &'static str)> = vec![(y.signing_day, "National signing day")];
    if let Some(p) = &w.prev_dates {
        v.push((p.signing_day, "National signing day"));
    }
    if let Some(e) = y.early_signing {
        v.push((e, "Early signing period"));
    }
    v.into_iter().filter(|s| s.0 >= w.today).min_by_key(|s| s.0)
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// The NCAA period today, in words, and what it allows.
pub fn period_today(w: &World) -> (&'static str, &'static str) {
    let p = recruiting_period(&w.dates, w.today);
    let rule = match p {
        Period::Contact => "In-person contact on and off campus: home visits, official and unofficial visits.",
        Period::Evaluation => "Coaches may watch practices and games but not talk off campus; visits on campus.",
        Period::Quiet => "Contact only on your campus: official and unofficial visits, no home visits.",
        Period::Dead => "No in-person contact at all: calls, messages and offers only.",
    };
    (p.name(), rule)
}

/// What your staff learns about a recruit's family by being around them:
/// who decides and what weighs with them, plus his transcript. The screens
/// turn it into sentences, never numbers.
#[derive(Clone, Copy, Debug)]
pub struct FamilyRead {
    pub decider: crate::world::background::Decider,
    pub money: f64,
    pub close_to_home: f64,
    pub playing_time: f64,
    pub academics: f64,
    pub development: f64,
    pub prestige: f64,
    pub gpa: f64,
}

pub fn family_read(w: &World, i: usize) -> FamilyRead {
    let p = &w.people[i];
    let x = &p.family.decider_wants;
    FamilyRead {
        decider: p.family.decider,
        money: x.money,
        close_to_home: x.close_to_home,
        playing_time: x.playing_time,
        academics: x.academics,
        development: x.development,
        prestige: x.prestige,
        gpa: p.bg.gpa,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::recruiting::SignStage;
    use crate::world::seat::{Job, You};
    use crate::world::{Person, WorldOptions};

    fn you() -> You {
        You {
            name: "Test Coach".into(),
            age: 44,
            ethnicity: crate::world::ethnicity::Ethnicity::White,
            home_state: 0,
            ex_pro: false,
            reputation: 10.0,
            renown: 10.0,
            played: None,
            teaching: [10.0; crate::world::coach::GROUP_COUNT],
            scheme: 10.0,
            game_management: 10.0,
            motivation: 10.0,
            discipline: 10.0,
            recruiting: 12.0,
            network: 10.0,
            relationships: 12.0,
            style: Default::default(),
            road: String::new(),
            yours: Default::default(),
            profile: Default::default(),
            born: crate::calendar::born::unknown(),
            aura: 0.0,
            came_up: None,
        }
    }

    /// A tiny world with three high school classes, with or without your
    /// career at an FBS program.
    fn world(career: bool) -> (World, usize) {
        let mut w = World::empty(WorldOptions { scale: 0.002, ..Default::default() });
        for age in [17, 16, 15] {
            w.add_class_aged(age);
        }
        let j = w.programs.iter().position(|p| p.division == Division::FBS).unwrap();
        if career {
            w.start_career(&you(), Job::College(j), 7);
        }
        (w, j)
    }

    fn recruit(w: &World, class_of: i32, nth: usize) -> usize {
        let mut v: Vec<usize> = (0..w.people.len())
            .filter(|&i| {
                let p = &w.people[i];
                p.stage == Stage::HighSchool && p.class_of == class_of && !p.player.position.is_specialist()
            })
            .collect();
        v.sort_by(|&a, &b| w.people[b].player.overall().total_cmp(&w.people[a].player.overall()));
        v[nth.min(v.len() - 1)]
    }

    /// Without a career nothing in the desk reaches the world: every hook
    /// answers None and the desk's day leaves the world as it was.
    #[test]
    fn a_world_without_a_career_is_untouched() {
        let (mut w, _) = world(false);
        assert!(stage_board(&w).is_none());
        assert!(visits_program(&w).is_none());
        assert!(portal_wants(&w).is_none());
        assert!(stay_talks(&w).is_none());
        assert!(roster_deals(&w).is_none());
        let before = serde_json::to_string(&w.front_office).unwrap();
        let today = w.today;
        day(&mut w, today);
        assert_eq!(serde_json::to_string(&w.front_office).unwrap(), before);
        let first = w.people[0].player.id;
        assert!(offer(&mut w, first).is_err());
    }

    /// A duty handed to your staff runs as the AI's: the hooks give the
    /// visit days and the portal nothing of yours, the board's offers wait
    /// on your signature (SK26), and your own moves are refused with the
    /// staff's words.
    #[test]
    fn a_staff_duty_runs_like_the_ai() {
        let (mut w, _) = world(true);
        assert!(stage_board(&w).is_some(), "the board is yours by default");
        assert!(visits_program(&w).is_some());
        assert!(portal_wants(&w).is_some());
        for d in [Duty::Board, Duty::Visits, Duty::Portal, Duty::Nil] {
            w.front_office.set_owner(d, Owner::Staff);
        }
        // SK26: a board handed to your staff still waits on your
        // signature, so the signing stage keeps reading it.
        assert!(stage_board(&w).is_some());
        assert!(visits_program(&w).is_none());
        assert!(portal_wants(&w).is_none());
        assert!(stay_talks(&w).is_none());
        assert!(roster_deals(&w).is_none());
        let class = rec::next_class(&w);
        let id = w.people[recruit(&w, class, 0)].player.id;
        let e = offer(&mut w, id).unwrap_err();
        assert!(e.contains("coordinator"), "{e}");
        let soon = w.today.add(10);
        let e = set_visit(&mut w, id, VisitKind::Unofficial, soon).unwrap_err();
        assert!(e.contains("staff"), "{e}");
    }

    /// Visits keep the NCAA's calendar: nothing in a dead period, a home
    /// visit only in a contact period and only for a senior, an official
    /// starts on a Friday and is the class signing next's, one per school.
    #[test]
    fn visits_keep_the_ncaa_calendar() {
        let (w, _) = world(true);
        let class = rec::next_class(&w);
        let senior = recruit(&w, class, 3);
        let soph = recruit(&w, class + 2, 3);
        let y = &w.dates;
        let mut d = w.today.add(1);
        let (mut dead, mut contact_home, mut open_saturday) = (0, 0, 0);
        while d <= y.super_bowl {
            let period = recruiting_period(y, d);
            if period == Period::Dead {
                dead += 1;
                for k in VisitKind::ALL {
                    assert!(visit_rule(&w, senior, k, d).is_some(), "{} on a dead day {}", k.name(), d.pretty());
                }
            }
            if period != Period::Contact {
                assert!(visit_rule(&w, senior, VisitKind::Home, d).is_some());
            } else if d.weekday() != Weekday::Sun {
                contact_home += 1;
                assert!(visit_rule(&w, senior, VisitKind::Home, d).is_none(), "home visit on {}", d.pretty());
                assert!(visit_rule(&w, soph, VisitKind::Home, d).is_some(), "a sophomore at home");
            }
            if d.weekday() != Weekday::Fri {
                assert!(visit_rule(&w, senior, VisitKind::Official, d).is_some());
            }
            assert!(visit_rule(&w, soph, VisitKind::Official, d).is_some(), "a sophomore's official");
            if period != Period::Dead && d.weekday() == Weekday::Sat && !(9..=11).contains(&d.month()) {
                open_saturday += 1;
                assert!(visit_rule(&w, soph, VisitKind::Unofficial, d).is_none(), "unofficial {}", d.pretty());
            }
            d = d.add(1);
        }
        assert!(dead > 0 && contact_home > 0 && open_saturday > 0, "{dead} {contact_home} {open_saturday}");
    }

    /// Your board goes through the world's own signing day: your offer is on
    /// his list, a recruit nobody else wants who holds only your offer signs
    /// with you, and one you never offered gets no scholarship from you.
    #[test]
    fn your_offers_go_through_the_signing_day() {
        let (mut w, j) = world(true);
        w.front_office.fill_board = false;
        let class = rec::next_class(&w);
        // The class's last two qualifiers (no specialists): no AI board reaches them.
        let div = w.programs[j].division;
        let mut v: Vec<usize> = (0..w.people.len())
            .filter(|&i| {
                let p = &w.people[i];
                p.stage == Stage::HighSchool
                    && p.class_of == class
                    && !p.player.position.is_specialist()
                    && rec::ncaa_tier(div, rec::eligibility_gpa(p.bg.gpa, p.cal.test_score, class))
                        == rec::Eligibility::Qualifier
            })
            .collect();
        v.sort_by(|&a, &b| w.people[a].player.overall().total_cmp(&w.people[b].player.overall()));
        let (low, other) = (v[0], v[1]);
        let id = w.people[low].player.id;
        offer(&mut w, id).unwrap();
        assert!(w.people[low].cal.offers.contains(&j));
        let b = stage_board(&w).unwrap();
        assert_eq!(b.offers.len(), 1);
        assert_eq!(b.offers[0].person, low);
        let season = w.dates.season;
        rec::signing_stage(&mut w, SignStage::Final, season);
        // A scholarship signee (walk-ons stay with your staff).
        let signed = |p: &Person| p.cal.signed.filter(|s| !s.walk_on).map(|s| s.program);
        assert_eq!(signed(&w.people[low]), Some(j), "{}", w.people[low].name);
        assert_ne!(signed(&w.people[other]), Some(j), "never offered, never signed with you");
    }
}
