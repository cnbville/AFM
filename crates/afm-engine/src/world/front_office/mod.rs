//! Your front office (final build, lane 10): every choice the AI's staff
//! made for your own team, made by you, through the same market, trade,
//! waiver and recruiting code the world's AI teams run on. A duty can be
//! handed back to your staff; what you decide stands, and a league
//! deadline you leave undecided is met the way the rules meet it (no tag,
//! no tender, an option declined, an offer sheet not matched), after a
//! reminder that stops Continue. The one exception is the cutdown: the
//! league has a 53, so your staff picks whatever you left open.
//!
//! Nothing here touches an AI team or a world without a career: every
//! hook in the shared code asks `yours` first, which is false without a
//! seat. New randomness is keyed by the career's seed (`key`), so it moves
//! no other draw.

pub mod chain;
pub mod college;
pub mod deals;
pub mod market;
pub mod nfl;
pub mod race;
pub mod roster;
pub mod school;

use super::World;
use crate::calendar::{Date, Event, Kind, Level, Loudness};

/// Who handles a duty for your team.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Owner {
    #[default]
    You,
    Staff,
}

/// The duties a front office splits between you and your staff.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Duty {
    /// NFL: re-signing your own players, extensions and fifth-year options.
    Contracts,
    /// NFL: franchise and transition tags, restricted tenders, offer sheets.
    Tags,
    /// NFL: offers to other clubs' free agents.
    FreeAgency,
    /// NFL: trades, both ways.
    Trades,
    /// NFL: restructures and cap casualties when the club must find room.
    Cap,
    /// NFL: the cut to 53 and the practice squad picked with it.
    Cutdown,
    /// NFL: claims on players other clubs waive.
    Waivers,
    /// NFL: the practice squad in season (signings, call-ups, protection).
    PracticeSquad,
    /// College: the recruiting board and scholarship offers.
    Board,
    /// College: official, unofficial and home visits.
    Visits,
    /// College: the transfer portal, in and out.
    Portal,
    /// College: the collective's NIL money.
    Nil,
    /// High school: who plays varsity, JV and freshman ball.
    Squads,
}

impl Duty {
    pub const ALL: [Duty; 13] = [
        Duty::Contracts,
        Duty::Tags,
        Duty::FreeAgency,
        Duty::Trades,
        Duty::Cap,
        Duty::Cutdown,
        Duty::Waivers,
        Duty::PracticeSquad,
        Duty::Board,
        Duty::Visits,
        Duty::Portal,
        Duty::Nil,
        Duty::Squads,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Duty::Contracts => "Contracts",
            Duty::Tags => "Tags and tenders",
            Duty::FreeAgency => "Free agency",
            Duty::Trades => "Trades",
            Duty::Cap => "Cap moves",
            Duty::Cutdown => "Cutdown",
            Duty::Waivers => "Waiver claims",
            Duty::PracticeSquad => "Practice squad",
            Duty::Board => "Recruiting board",
            Duty::Visits => "Visits",
            Duty::Portal => "Transfer portal",
            Duty::Nil => "NIL",
            Duty::Squads => "Squads",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Duty::Contracts => "contracts",
            Duty::Tags => "tags",
            Duty::FreeAgency => "free_agency",
            Duty::Trades => "trades",
            Duty::Cap => "cap",
            Duty::Cutdown => "cutdown",
            Duty::Waivers => "waivers",
            Duty::PracticeSquad => "practice_squad",
            Duty::Board => "board",
            Duty::Visits => "visits",
            Duty::Portal => "portal",
            Duty::Nil => "nil",
            Duty::Squads => "squads",
        }
    }

    pub fn from_key(k: &str) -> Option<Duty> {
        Duty::ALL.into_iter().find(|d| d.key() == k)
    }

    /// The level whose seat has this duty.
    pub fn level(self) -> Level {
        match self {
            Duty::Board | Duty::Visits | Duty::Portal | Duty::Nil => Level::College,
            Duty::Squads => Level::HighSchool,
            _ => Level::Nfl,
        }
    }

    /// Who has it on the first day of a job: the decisions that shape the
    /// roster are yours, the daily bookkeeping your staff's (Claude's pick:
    /// a real head coach with personnel power signs off on deals, trades
    /// and the 53, and leaves cap mechanics, the wire, the squad's weekly
    /// churn and the collective's paperwork to the people hired for them).
    pub fn default_owner(self) -> Owner {
        match self {
            Duty::Cap | Duty::Waivers | Duty::PracticeSquad | Duty::Nil => Owner::Staff,
            _ => Owner::You,
        }
    }

    /// What the staff does when it has the duty, in their words.
    pub fn staff_does(self) -> &'static str {
        match self {
            Duty::Contracts => "Your cap manager re-signs and extends the players the club builds around.",
            Duty::Tags => "Your cap manager tags the star who would walk and tenders the restricted players worth it.",
            Duty::FreeAgency => "Your personnel staff bids on the free agents who fill your holes.",
            Duty::Trades => "Your personnel director calls for a starter at your biggest hole and listens to offers.",
            Duty::Cap => "Your cap manager restructures and cuts to keep room for the draft class and the market.",
            Duty::Cutdown => {
                "Your staff cuts to 53 by camp and the preseason tape, and keeps the best young players on the squad."
            }
            Duty::Waivers => "Your personnel staff claims a player it grades above your worst man at his spot.",
            Duty::PracticeSquad => {
                "Your staff fills the squad, protects its best four and calls up a squad player when a spot thins."
            }
            Duty::Board => "Your recruiting coordinator ranks the board and offers the players the program can sign.",
            Duty::Visits => "Your recruiting coordinator brings the board's top names in for visits.",
            Duty::Portal => "Your staff chases the transfers who fill the two-deep's holes.",
            Duty::Nil => "Your collective pays the roster by what each player means to the team.",
            Duty::Squads => {
                "Your staff puts the best varsity players up and the young ones on JV and the freshman team."
            }
        }
    }
}

/// One thing your front office did, for its log.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Move {
    pub date: Date,
    pub text: String,
    /// The player it was about (id), if one.
    pub player: Option<u32>,
}

/// Your front office: who has each duty, the staff's standing instructions
/// and each level's desk.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct FrontOffice {
    /// Duties you handed over or took back (a missing one has its default).
    pub owners: Vec<(Duty, Owner)>,
    /// The cap manager may not cut a starter to make room.
    pub keep_starters: bool,
    /// The recruiting coordinator adds his own targets for the board's open spots.
    pub fill_board: bool,
    pub nfl: nfl::NflDesk,
    pub trades: deals::TradeDesk,
    pub roster: roster::RosterDesk,
    pub college: college::CollegeDesk,
    pub school: school::SchoolDesk,
    /// Your moves, newest last (the last 400).
    pub log: Vec<Move>,
    /// Each recruit's race, week by week, as your office followed it.
    pub race: race::RaceLog,
    /// The offer chain at your college (SK25-SK31).
    pub chain: chain::Chain,
}

impl Default for FrontOffice {
    fn default() -> FrontOffice {
        FrontOffice {
            owners: Vec::new(),
            keep_starters: true,
            fill_board: true,
            nfl: Default::default(),
            trades: Default::default(),
            roster: Default::default(),
            college: Default::default(),
            school: Default::default(),
            log: Vec::new(),
            race: Default::default(),
            chain: Default::default(),
        }
    }
}

const LOG_KEEP: usize = 400;

impl FrontOffice {
    pub fn owner(&self, d: Duty) -> Owner {
        self.owners.iter().find(|x| x.0 == d).map_or(d.default_owner(), |x| x.1)
    }

    pub fn set_owner(&mut self, d: Duty, o: Owner) {
        self.owners.retain(|x| x.0 != d);
        if o != d.default_owner() {
            self.owners.push((d, o));
        }
    }
}

/// Whether NFL club `t` is your club and the duty is yours: the AI's code
/// then leaves your club to you.
pub(crate) fn yours(w: &World, t: usize, d: Duty) -> bool {
    w.career.is_some() && w.seat_club(t) && w.seat_head() && w.front_office.owner(d) == Owner::You
}

/// Whether college program `p` is yours and the duty is yours.
pub(crate) fn yours_program(w: &World, p: usize, d: Duty) -> bool {
    w.career.is_some() && w.seat_program(p) && w.seat_head() && w.front_office.owner(d) == Owner::You
}

/// Your NFL club, if you hold one.
pub(crate) fn club(w: &World) -> Option<usize> {
    w.career.as_ref()?;
    w.seat.filter(|s| s.level == Level::Nfl && w.seat_head()).and_then(|s| s.nfl_team)
}

/// Your college program, if you hold one.
pub(crate) fn program(w: &World) -> Option<usize> {
    w.career.as_ref()?;
    w.seat.filter(|s| s.level == Level::College && w.seat_head()).and_then(|s| s.program)
}

/// A draw keyed to the career (its own stream): the same question asked
/// twice gets the same answer, and no other draw in the world moves.
pub(crate) fn key(w: &World, salt: u64, id: u64) -> f64 {
    let seed = w.career.as_ref().map_or(0, |c| c.seed as u64);
    super::season::key_uniform(seed ^ 0xF0F1_CE00_0000 ^ salt, id)
}

pub(crate) fn key_n(w: &World, salt: u64, id: u64) -> f64 {
    let seed = w.career.as_ref().map_or(0, |c| c.seed as u64);
    super::season::key_normal(seed ^ 0xF0F1_CE00_0000 ^ salt, id)
}

/// Logs one of your moves.
pub(crate) fn note(w: &mut World, text: String, player: Option<u32>) {
    let date = w.today;
    let log = &mut w.front_office.log;
    log.push(Move { date, text, player });
    if log.len() > LOG_KEEP {
        let cut = log.len() - LOG_KEEP;
        log.drain(..cut);
    }
}

/// A line for your inbox about your own team; `decision` stops Continue.
pub(crate) fn tell(w: &mut World, d: Date, text: String, decision: bool) {
    tell_about(w, d, text, decision, &[]);
}

/// The same, with the people it names (indices) linked, and every NFL club
/// it names (L2).
pub(crate) fn tell_about(w: &mut World, d: Date, text: String, decision: bool, who: &[usize]) {
    tell_links(w, d, text, decision, who, &[]);
}

/// The same, with the college programs it names linked too.
pub(crate) fn tell_links(w: &mut World, d: Date, text: String, decision: bool, who: &[usize], schools: &[usize]) {
    let Some(seat) = w.seat else { return };
    let mut e = Event::new(d, seat.level, Kind::Deadline, Loudness::Medium, text).rule("Your front office");
    for &i in who {
        let p = &w.people[i];
        e = e.person(p.player.id, &p.name);
    }
    for &j in schools {
        e = e.school_link(j, w.program_name(j));
    }
    if seat.level == Level::Nfl {
        for (t, club) in w.nfl.iter().enumerate() {
            e = e.club_link(t, club.name);
        }
    }
    if !decision {
        e.kind = Kind::News;
        e.loud = Loudness::Small;
    }
    match seat.level {
        Level::Nfl => {
            if let Some(t) = seat.nfl_team {
                e = e.team(t);
            }
        }
        Level::College => {
            if let Some(p) = seat.program {
                e = e.program(p);
            }
        }
        // An event's program is a college program: a high school's line
        // carries its rule instead.
        _ => {}
    }
    if decision {
        e = e.decision();
    }
    w.calendar.post(e);
}

/// Your front office's day, after the world's own: reminders before
/// deadlines, answers that came in, calls that expired, books that settle.
pub(crate) fn day(w: &mut World, d: Date) {
    if w.career.is_none() {
        return;
    }
    nfl::day(w, d);
    deals::day(w, d);
    roster::day(w, d);
    college::day(w, d);
    school::day(w, d);
    market::day(w, d);
    race::day(w, d);
}

/// Money in words: "$12.4M", "$850K".
pub fn dollars(x: f64) -> String {
    if x.abs() >= 1e6 {
        format!("${:.1}M", x / 1e6)
    } else if x.abs() >= 1e3 {
        format!("${:.0}K", x / 1e3)
    } else {
        format!("${x:.0}")
    }
}

/// A person's place in `people` from his id (people move when the world
/// is compacted each year, so the desks keep ids).
pub fn index_of(w: &World, id: u32) -> Option<usize> {
    w.people.iter().position(|p| p.player.id == id)
}

/// Several ids at once, in one pass over the people.
pub(crate) fn indices(w: &World, ids: &[u32]) -> std::collections::HashMap<u32, usize> {
    if ids.is_empty() {
        return Default::default();
    }
    let want: std::collections::HashSet<u32> = ids.iter().copied().collect();
    w.people.iter().enumerate().filter(|(_, p)| want.contains(&p.player.id)).map(|(i, p)| (p.player.id, i)).collect()
}
