//! Your network (final build N1-N7): the coaches you know and how warm each
//! tie is (the Contacts screen); reaching out by call, text or lunch, which
//! costs time, and neediness that people notice; a ledger of favours asked
//! and done; the networking trips of the real calendar (the AFCA convention
//! in January, the Senior Bowl, the combine, the spring clinics); warm ties
//! that open doors (names for your staff, the grapevine on jobs, a word for
//! a player, interest from a school); the coaching all-star games; speaking
//! at clinics.
//!
//! Stored: your contacts (a few dozen to a couple of hundred), the ledger
//! and the trips you took. Everything else is read from the world. Sizes
//! are Claude's picks (proposed, not confirmed).

use super::scene::{self, Kind, Pick, Scene, Who};
use super::talks;
use super::{Team, coach_link};
use crate::calendar::{Date, Level, Loudness, Weekday};
use crate::world::World;
use crate::world::coach::{Employer, Role};


#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Contact {
    /// The coach, or `NOBODY` when the contact is someone else (`other`).
    pub coach: usize,
    /// 0 (a name) to 1 (a friend who picks up at midnight).
    pub warmth: f32,
    pub met: Date,
    /// How you know him ("worked for you at Toledo", "met at the AFCA
    /// convention 2027").
    pub how: String,
    pub last: Option<Date>,
    /// Times you reached out in the last thirty days (neediness).
    pub recent: Vec<Date>,
    /// Favours: + he owes you, - you owe him.
    pub balance: i8,
    /// Someone besides a coach (C13, C15, C19): a former player in his life
    /// after football, an agent, an athletic director.
    #[serde(default)]
    pub other: Option<Other>,
    /// Players you placed with him (C16): the friend you send players to
    /// calls you first with his own star (C15).
    #[serde(default)]
    pub sent: u8,
    /// His word's track record (SK35): tips settled, and how many held up.
    #[serde(default)]
    pub record: (u8, u8),
}

/// The coach field of a contact who isn't a coach.
pub const NOBODY: usize = usize::MAX;

/// A contact who isn't a coach.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Other {
    /// A former player (his player id, `World::formers`).
    Former(u32),
    /// An agent (`market.agents`).
    Agent(usize),
    /// A program's athletic director, by the year he took the job.
    Ad(usize, i32),
}

impl Contact {
    /// The coach, when the contact is one.
    pub fn coach_id(&self) -> Option<usize> {
        (self.other.is_none() && self.coach != NOBODY).then_some(self.coach)
    }
}

/// Who a contact is: the coach a former player became counts as the coach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Whom {
    Coach(usize),
    Other(Other),
}

impl Whom {
    fn of(x: &Contact) -> Whom {
        match x.other {
            Some(o) => Whom::Other(o),
            None => Whom::Coach(x.coach),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Favour {
    pub date: Date,
    pub coach: usize,
    pub words: String,
    /// You asked it of him (true), or he of you.
    pub asked: bool,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Network {
    pub contacts: Vec<Contact>,
    pub ledger: Vec<Favour>,
    /// Trips taken: (year, trip code).
    pub trips: Vec<(i32, u8)>,
    /// Clinic talks you gave.
    pub clinics: u32,
    /// All-star games you coached in: (year, game).
    pub allstar: Vec<(i32, String)>,
    /// Jobs where people asked about you: (year, team, words).
    pub interest: Vec<(i32, Team, String)>,
    pub built: bool,
}

/// How warm your tie to a coach is (0 when you don't know him).
pub fn warmth(w: &World, c: usize) -> f32 {
    w.folk.network.contacts.iter().find(|x| x.coach_id() == Some(c)).map_or(0.0, |x| x.warmth)
}

/// How warm your tie to someone besides a coach is.
pub fn warmth_other(w: &World, o: Other) -> f32 {
    w.folk.network.contacts.iter().find(|x| x.other == Some(o)).map_or(0.0, |x| x.warmth)
}

fn add_contact(w: &mut World, c: usize, warmth: f32, how: String) {
    add(w, Whom::Coach(c), warmth, how, false);
}

/// Adds someone, or warms him: `start` sets a starting warmth (the higher
/// one stands, C12), else half the new warmth is added.
fn add(w: &mut World, who: Whom, warmth: f32, how: String, start: bool) {
    if let Whom::Coach(c) = who
        && w.career.as_ref().is_some_and(|x| x.coach == c)
    {
        return;
    }
    let today = w.today;
    match w.folk.network.contacts.iter_mut().find(|x| Whom::of(x) == who) {
        Some(x) if start => {
            if warmth > x.warmth {
                x.warmth = warmth.min(1.0);
                x.how = how;
            }
        }
        Some(x) => x.warmth = (x.warmth + 0.5 * warmth).min(1.0),
        None => {
            let (coach, other) = match who {
                Whom::Coach(c) => (c, None),
                Whom::Other(o) => (NOBODY, Some(o)),
            };
            w.folk.network.contacts.push(Contact {
                coach,
                warmth: warmth.clamp(0.05, 1.0),
                met: today,
                how,
                last: None,
                recent: Vec::new(),
                balance: 0,
                other,
                sent: 0,
                record: (0, 0),
            });
            if w.folk.network.contacts.len() > 220 {
                // The coldest name drops off.
                if let Some(k) = w
                    .folk
                    .network
                    .contacts
                    .iter()
                    .enumerate()
                    .min_by(|a, b| a.1.warmth.total_cmp(&b.1.warmth))
                    .map(|x| x.0)
                {
                    w.folk.network.contacts.remove(k);
                }
            }
        }
    }
}

/// Moves a tie (a hire, a firing, a favour).
pub fn touch(w: &mut World, c: usize, d: f32) {
    match w.folk.network.contacts.iter_mut().find(|x| x.coach_id() == Some(c)) {
        Some(x) => x.warmth = (x.warmth + d).clamp(0.0, 1.0),
        None if d > 0.0 => add_contact(w, c, d, "through work".into()),
        None => {}
    }
}

/// Moves a tie to someone besides a coach.
pub fn touch_other(w: &mut World, o: Other, d: f32, how: &str) {
    match w.folk.network.contacts.iter_mut().find(|x| x.other == Some(o)) {
        Some(x) => x.warmth = (x.warmth + d).clamp(0.0, 1.0),
        None if d > 0.0 => add(w, Whom::Other(o), d, how.into(), false),
        None => {}
    }
}

/// Your starting network (N1): your staff, the men you replaced and who hired
/// you, everyone from your past who knows you (C12), your old teammates
/// and players (C13), your agent and athletic directors, the coaches you
/// came up with (your road and your home state).
pub fn build(w: &mut World) {
    let Some(team) = Team::seat(w) else { return };
    let Some(you) = w.career.as_ref().map(|c| c.coach) else { return };
    let home = w.career.as_ref().map(|c| c.you.home_state);
    let name = team.name(w);
    for (c, role) in super::facts::assistants(w, team) {
        add(w, Whom::Coach(c), 0.35, format!("on your staff at {name} ({})", role.name()), true);
    }
    if let Some((o, ..)) = super::facts::predecessor(w, team, you) {
        add(w, Whom::Coach(o), 0.15, format!("the coach you replaced at {name}"), true);
    }
    if w.folk.network.built {
        return;
    }
    // Everyone who knows you (C12), at how well he knows you; the higher
    // warmth stands where he's on your staff too.
    let n = w.coaches.len();
    let known = w.career.as_ref().map(|c| c.known.clone()).unwrap_or_default();
    for k in known.into_iter().filter(|k| k.coach < n) {
        add(w, Whom::Coach(k.coach), k.strength, k.how, true);
    }
    old_mates(w);
    agents_and_ads(w, team);
    alumni(w, team);
    // Coaches you came up with: near your age, from your home state.
    let my_age = w.coaches[you].age as i64;
    let mut pool: Vec<usize> = w
        .coaches
        .iter()
        .enumerate()
        .filter(|(c, co)| {
            *c != you
                && !co.retired
                && co.job.is_some()
                && (co.age as i64 - my_age).abs() <= 8
                && co.home_state.is_some()
                && co.home_state == home
        })
        .map(|(c, _)| c)
        .collect();
    pool.sort_by(|&a, &b| super::roll(w, 0x0F1E, a as u64).total_cmp(&super::roll(w, 0x0F1E, b as u64)));
    for c in pool.into_iter().take(6) {
        let warm = 0.3 + 0.4 * super::roll(w, 0x0F1F, c as u64) as f32;
        add_contact(w, c, warm, "came up in coaching with you".into());
    }
    // A high school coach knows his area's coaches.
    if let Team::High(h) = team {
        let st = w.high_schools[h].state;
        let near: Vec<usize> = w
            .high_schools
            .iter()
            .enumerate()
            .filter(|(k, s)| *k != h && s.state == st)
            .filter_map(|(_, s)| s.coach)
            .take(12)
            .collect();
        for c in near {
            add_contact(w, c, 0.2, "coaches in your area".into());
        }
    }
    w.folk.network.built = true;
}

/// Your old teammates and the players you coached (C13), with the lives
/// they lead now, as warm as you were close (proposed, not confirmed: a
/// college teammate from your position room 0.5, your side of the ball
/// 0.35, the other side 0.25; an NFL teammate 0.45 or 0.25; a player you
/// coached in your group 0.45, as head coach or his side's coordinator
/// 0.3, else 0.15; half for one shared season). Sixty at most.
fn old_mates(w: &mut World) {
    use crate::world::coach::Group;
    let Some(c) = w.career.as_ref() else { return };
    let you = c.coach;
    let Some(past) = c.past.clone() else { return };
    let mine = w.coaches[you].played.map(|p| p.group()).or(c.you.came_up);
    let stints = w.coaches[you].history.clone();
    let near = |g: Group, close: f32, side: f32, far: f32| -> f32 {
        if Some(g) == mine {
            close
        } else if mine.is_some_and(|h| h.offense() == g.offense()) {
            side
        } else {
            far
        }
    };
    let span = |a: i32, b: i32, x: i16, y: i16| (b.min(y as i32) - a.max(x as i32) + 1).max(0);
    let mut found: Vec<(f32, Whom, String)> = Vec::new();
    for f in &w.formers.list {
        let g = f.played.group();
        let mut best: Option<(f32, String)> = None;
        let mut take = |k: f32, how: String| {
            if best.as_ref().is_none_or(|b| k > b.0) {
                best = Some((k, how));
            }
        };
        if past.played
            && let Some((col, a, b)) = past.college
            && let Some(&(_, x, y)) = f.colleges.iter().find(|c| c.0 as usize == col)
        {
            let n = span(a, b, x, y);
            if n > 0 {
                let half = if n == 1 { 0.5 } else { 1.0 };
                take(near(g, 0.5, 0.35, 0.25) * half, format!("your teammate at {}", w.program_name(col)));
            }
        }
        for &(e, a, b, playing) in &past.stops {
            if let (true, Employer::Nfl(t)) = (playing, e)
                && f.at_club(t, a, b)
            {
                take(near(g, 0.45, 0.25, 0.25), format!("your teammate on the {}", w.nfl[t].name));
            }
        }
        for s in &stints {
            let k = match s.role {
                Role::HeadCoach => 0.3,
                Role::PositionCoach(h) if h == g => 0.45,
                r if r.offense() == Some(g.offense()) => 0.3,
                _ => 0.15,
            };
            match s.employer {
                Employer::College(p) => {
                    if let Some(&(_, x, y)) = f.colleges.iter().find(|c| c.0 as usize == p)
                        && span(s.from, s.to, x, y) > 0
                    {
                        take(k, format!("played for you at {}", w.program_name(p)));
                    }
                }
                Employer::Nfl(t) if f.at_club(t, s.from, s.to) => {
                    take(k * 0.9, format!("played for you on the {}", w.nfl[t].name));
                }
                Employer::HighSchool(h) if f.hs as usize == h && f.born != crate::calendar::born::UNKNOWN => {
                    let y = f.born.year();
                    if span(s.from, s.to, (y + 14) as i16, (y + 17) as i16) > 0 {
                        take(if s.role == Role::HeadCoach { 0.4 } else { 0.2 }, format!("played for you at {}", w.school_name(h)));
                    }
                }
                _ => {}
            }
        }
        if let Some((k, how)) = best.filter(|b| b.0 >= 0.15) {
            let who = f.coach.map_or(Whom::Other(Other::Former(f.id)), |c| Whom::Coach(c as usize));
            found.push((k, who, how));
        }
    }
    found.sort_by(|a, b| b.0.total_cmp(&a.0));
    found.truncate(60);
    for (k, who, how) in found {
        add(w, who, k, how, true);
    }
}

/// Your agent from your playing days (C13, C19; proposed, not confirmed:
/// warmth 0.6, the agent most of your NFL teammates had, else one by your
/// name), the agents your NFL club trusts (0.15 + 0.4 × its trust, from a
/// trust of 0.25), and the athletic directors you worked for who still
/// hold their jobs (0.4).
fn agents_and_ads(w: &mut World, team: Team) {
    let Some(c) = w.career.as_ref() else { return };
    let you = c.coach;
    let stops = c.past.as_ref().map(|p| p.stops.clone()).unwrap_or_default();
    let n = w.market.agents.len();
    if n > 0 && w.coaches[you].played.is_some_and(|p| p.nfl()) {
        let mut counts: Vec<u32> = vec![0; n];
        for f in &w.formers.list {
            let mate = stops.iter().any(|&(e, a, b, playing)| playing && matches!(e, Employer::Nfl(t) if f.at_club(t, a, b)));
            if let (true, Some(a)) = (mate, f.agent)
                && (a as usize) < n
            {
                counts[a as usize] += 1;
            }
        }
        let a = match (0..n).max_by_key(|&k| counts[k]) {
            Some(k) if counts[k] > 0 => k,
            _ => (super::roll(w, 0xA6E7, you as u64) * n as f64) as usize % n,
        };
        add(w, Whom::Other(Other::Agent(a)), 0.6, "your agent when you played".into(), true);
    }
    if let Team::Nfl(t) = team {
        let trusted: Vec<(usize, f64)> = w
            .market
            .agents
            .iter()
            .enumerate()
            .filter_map(|(k, a)| a.rel.get(t).copied().filter(|&r| r >= 0.25).map(|r| (k, r)))
            .collect();
        let club = w.nfl[t].name;
        for (k, r) in trusted {
            add(w, Whom::Other(Other::Agent(k)), 0.15 + 0.4 * r as f32, format!("has done deals with the {club}"), true);
        }
    }
    let stints = w.coaches[you].history.clone();
    for s in stints {
        if let Employer::College(p) = s.employer
            && Team::College(p) != team
            && let Some(ad) = w.programs[p].ad.as_ref().filter(|ad| ad.since <= s.to)
        {
            let since = ad.since;
            add(w, Whom::Other(Other::Ad(p, since)), 0.4, format!("your athletic director at {}", w.program_name(p)), true);
        }
    }
}

/// The program's own former players call to welcome a new coach (SK35's
/// alumni; proposed, not confirmed: up to ten, the best-known first, at
/// 0.15).
fn alumni(w: &mut World, team: Team) {
    let Team::College(p) = team else { return };
    let mut v: Vec<(u8, i16, Whom)> = w
        .formers
        .list
        .iter()
        .filter(|f| f.colleges.iter().any(|c| c.0 as usize == p))
        .map(|f| (f.played.grade, f.since, f.coach.map_or(Whom::Other(Other::Former(f.id)), |c| Whom::Coach(c as usize))))
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    let name = w.program_name(p).to_string();
    for (_, _, who) in v.into_iter().take(10) {
        add(w, who, 0.15, format!("played at {name}; called to welcome you"), true);
    }
}

/// How you reach out (N2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum How {
    Text,
    Call,
    Lunch,
}

impl How {
    pub fn from(s: &str) -> How {
        match s {
            "text" => How::Text,
            "lunch" => How::Lunch,
            _ => How::Call,
        }
    }
    fn hours(self) -> f32 {
        match self {
            How::Text => 0.1,
            How::Call => 0.5,
            How::Lunch => 2.5,
        }
    }
    fn warmth(self) -> f32 {
        match self {
            How::Text => 0.02,
            How::Call => 0.05,
            How::Lunch => 0.1,
        }
    }
}

/// What you reach out for (N2, N3, N5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    CatchUp,
    /// A name for your staff.
    Name,
    /// The grapevine: who's on the hot seat, which jobs may open.
    Grapevine,
    /// A word for one of your players (a look from the next level).
    Word,
}

impl Why {
    pub fn from(s: &str) -> Why {
        match s {
            "name" => Why::Name,
            "grapevine" => Why::Grapevine,
            "word" => Why::Word,
            _ => Why::CatchUp,
        }
    }
}

/// Reaching out (N2): costs time; asking costs a favour; asking too often
/// is noticed. Returns what happened, in words.
pub fn reach_out(w: &mut World, c: usize, how: How, why: Why, player: Option<u32>) -> Result<String, String> {
    if w.coaches.get(c).is_none() {
        return Err("No such coach.".into());
    }
    if how == How::Lunch {
        let near = super::facts::team_of_coach(w, c).and_then(|t| t.state(w)) == Team::seat(w).and_then(|t| t.state(w));
        if !near {
            return Err("He's too far away for lunch. Call him, or see him at a convention.".into());
        }
    }
    if !talks::spend(w, how.hours()) {
        return Err("Your week is full.".into());
    }
    let today = w.today;
    if !w.folk.network.contacts.iter().any(|x| x.coach_id() == Some(c)) {
        add_contact(w, c, 0.05, "you reached out cold".into());
    }
    let k = w.folk.network.contacts.iter().position(|x| x.coach_id() == Some(c)).unwrap_or(0);
    let (warm, recent, balance) = {
        let x = &mut w.folk.network.contacts[k];
        x.recent.retain(|d| d.days_to(today) < 30);
        let r = x.recent.len();
        x.recent.push(today);
        x.last = Some(today);
        (x.warmth, r, x.balance)
    };
    let co = &w.coaches[c];
    let rel = co.ability().relationships as f32;
    let name = co.name.clone();
    let asking = why != Why::CatchUp;
    // Neediness (N2): a third ask in a month, or asking a cold contact.
    let needy = asking && (recent >= 2 || warm < 0.15);
    let mut dw = how.warmth() * (0.7 + 0.04 * rel);
    if needy {
        dw = -0.06;
    }
    if asking {
        dw -= 0.02;
    }
    let yes_p = (0.2 + 0.7 * warm + 0.08 * balance as f32 + 0.02 * (rel - 10.0) - if needy { 0.25 } else { 0.0 })
        .clamp(0.02, 0.95);
    let yes = super::roll(w, 0x4E7 ^ today.0 as u64, c as u64) < yes_p as f64;
    let mut text = match how {
        How::Text => format!("You texted {name}."),
        How::Call => format!("You called {name}."),
        How::Lunch => format!("You took {name} to lunch."),
    };
    if needy {
        text.push_str(" He picked up, but you could hear him wondering what you wanted this time.");
    }
    match why {
        Why::CatchUp => text.push_str(" You talked about everything but football for a while."),
        Why::Name => {
            if yes {
                let pick = name_from(w, c);
                match pick {
                    Some(n) => {
                        add_contact(w, n, 0.3, format!("recommended by {name}"));
                        text.push_str(&format!(
                            " He gave you a name for your staff: {}. He'll take your call.",
                            coach_link(w, n)
                        ));
                    }
                    None => text.push_str(" He couldn't think of anyone right now."),
                }
            } else {
                text.push_str(" He said he'd think about it. He won't.");
            }
        }
        Why::Grapevine => {
            if yes {
                let hot = grapevine(w);
                if hot.is_empty() {
                    text.push_str(" He hasn't heard much: a quiet year.");
                } else {
                    text.push_str(&format!(" What he's hearing: {}.", hot.join("; ")));
                }
            } else {
                text.push_str(" He kept his cards close.");
            }
        }
        Why::Word => {
            let Some(pid) = player else { return Err("Which player?".into()) };
            let Some(i) = super::person(w, pid) else { return Err("No such player.".into()) };
            let at_next =
                super::facts::team_of_coach(w, c).is_some_and(|t| matches!(t, Team::College(_) | Team::Nfl(_)));
            if yes && at_next {
                let p = &mut w.people[i];
                p.visibility = (p.visibility + 0.12).min(1.0);
                text.push_str(&format!(" He'll have his staff look at {}'s film.", p.name));
            } else if !at_next {
                text.push_str(" He's not in a position to help with that.");
            } else {
                text.push_str(" He said they're full at that spot.");
            }
        }
    }
    if asking && yes {
        let words = match why {
            Why::Name => "a name for your staff",
            Why::Grapevine => "what he was hearing",
            _ => "a look at your player",
        };
        w.folk.network.ledger.push(Favour { date: today, coach: c, words: words.into(), asked: true });
        w.folk.network.contacts[k].balance -= 1;
    }
    let x = &mut w.folk.network.contacts[k];
    x.warmth = (x.warmth + dw).clamp(0.0, 1.0);
    trim_ledger(w);
    super::note(w, "network", coach_link(w, c), text.clone(), None);
    Ok(text)
}

fn trim_ledger(w: &mut World) {
    let n = w.folk.network.ledger.len();
    if n > 150 {
        w.folk.network.ledger.drain(..n - 150);
    }
}

/// A name from his tree: an assistant who worked under him, or one he knows.
fn name_from(w: &World, c: usize) -> Option<usize> {
    let mut v: Vec<(usize, f64)> = w
        .coaches
        .iter()
        .enumerate()
        .filter(|(k, co)| {
            *k != c && !co.retired && co.mentor == Some(c) && co.job.is_none_or(|j| j.role != Role::HeadCoach)
        })
        .map(|(k, co)| (k, co.reputation))
        .collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    v.first().map(|x| x.0)
}

/// The grapevine (N5): head coaches whose seats are hot, from their recent
/// results, as coaches talk about them.
fn grapevine(w: &World) -> Vec<String> {
    let mut v: Vec<(f64, String)> = Vec::new();
    for (p, prog) in w.programs.iter().enumerate() {
        if prog.division != crate::world::data::Division::FBS {
            continue;
        }
        let Some(c) = prog.head_coach else { continue };
        let co = &w.coaches[c];
        let last = co.recent.last().copied().unwrap_or(0.0) + co.recent.iter().rev().nth(1).copied().unwrap_or(0.0);
        if last < -0.12 && Some(c) != w.career.as_ref().map(|x| x.coach) {
            v.push((last, format!("{} is on the hot seat at {}", co.name, Team::College(p).name(w))));
        }
    }
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    v.into_iter().take(3).map(|x| x.1).collect()
}

// ---------------------------------------------------------------------------
// N4, N6, N7: trips, all-star games and clinics.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum Trip {
    Afca = 1,
    SeniorBowl,
    Combine,
    Clinic,
}

impl Trip {
    fn from(v: u64) -> Trip {
        match v {
            2 => Trip::SeniorBowl,
            3 => Trip::Combine,
            4 => Trip::Clinic,
            _ => Trip::Afca,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Trip::Afca => "The AFCA convention",
            Trip::SeniorBowl => "Senior Bowl week in Mobile",
            Trip::Combine => "The combine in Indianapolis",
            Trip::Clinic => "A coaching clinic",
        }
    }
    fn hours(self) -> f32 {
        match self {
            Trip::Afca | Trip::Combine => 12.0,
            Trip::SeniorBowl => 8.0,
            Trip::Clinic => 6.0,
        }
    }
}

/// The trips on the real calendar, offered a week ahead.
fn trips_day(w: &mut World, d: Date) {
    let Some(team) = Team::seat(w) else { return };
    let level = team.level();
    let afca = Date::nth_weekday(w.dates.title_game.year(), 1, Weekday::Sun, 2).max(w.dates.title_game.add(-3));
    let mut offer: Vec<(Trip, Date)> = Vec::new();
    if level != Level::Nfl {
        offer.push((Trip::Afca, afca));
    }
    if level != Level::HighSchool {
        offer.push((Trip::SeniorBowl, w.dates.senior_bowl.add(-4)));
        offer.push((Trip::Combine, w.dates.combine_start.add(2)));
    }
    if level != Level::Nfl {
        offer.push((Trip::Clinic, Date::nth_weekday(w.dates.spring_practice.0.year(), 3, Weekday::Sat, 3)));
    }
    for (t, when) in offer {
        if d == when.add(-7) && !w.folk.network.trips.iter().any(|x| x.0 == w.year && x.1 == t as u8) {
            let mut s = Scene::new(Kind::Trip, when, t.name());
            s.a = t as u64;
            s.stops = false;
            s.team = Some(team);
            s.place = match t {
                Trip::Afca => "The convention hotel lobby".into(),
                Trip::SeniorBowl => "Ladd-Peebles Stadium and the Senior Bowl hotel".into(),
                Trip::Combine => "Lucas Oil Stadium and the downtown hotels".into(),
                Trip::Clinic => "A high school auditorium, a projector and three hundred coaches".into(),
            };
            scene::add(w, s);
        }
    }
}

fn trip_open(_w: &mut World, s: &mut Scene) {
    let t = Trip::from(s.a);
    s.narrate(match t {
        Trip::Afca => "Every coach in the country is in one hotel for three days: the job fair, the lobby, the hospitality suites. Half the hiring of the year happens here, at the bar.".to_string(),
        Trip::SeniorBowl => "The practices are the show for the scouts; the hotel lobby at night is the show for coaches looking for staff or for jobs.".to_string(),
        Trip::Combine => "Agents, general managers, college coaches who came to watch their players: Indianapolis for a week.".to_string(),
        Trip::Clinic => "The clinic wants a speaker: an hour on your offense, then questions, then every high school coach in the room has your number.".to_string(),
    });
    s.pick(
        90,
        format!("Go ({} hours of your week)", t.hours() as u32),
        "Go",
        "warm",
        "New contacts and old ones warmed; the week's people time goes with it.",
    );
    if t == Trip::Clinic {
        s.pick(91, "Go, and give the talk", "Go", "fiery", "More hours; every coach in the room learns your name.");
    }
    s.pick(92, "Skip it this year", "Skip", "calm", "Your week stays yours.");
}

fn trip_answer(w: &mut World, s: &mut Scene, code: u32) {
    let t = Trip::from(s.a);
    if code == 92 {
        s.outcome.push("You stayed home.".into());
        s.end();
        return;
    }
    talks::charge(w, t.hours() + if code == 91 { 3.0 } else { 0.0 });
    w.folk.network.trips.push((w.year, t as u8));
    // Who you meet: coaches at the event's levels, more for a bigger name.
    // A bigger name meets more people (C10).
    let rep = super::facts::your_name(w);
    let n = (3.0 + rep / 4.0 + if code == 91 { 4.0 } else { 0.0 }) as usize;
    let you = w.career.as_ref().map(|c| c.coach);
    let mut pool: Vec<usize> = w
        .coaches
        .iter()
        .enumerate()
        .filter(|(c, co)| {
            Some(*c) != you
                && !co.retired
                && co.job.is_some_and(|j| match t {
                    Trip::Afca => !matches!(j.employer, Employer::Nfl(_)),
                    Trip::SeniorBowl | Trip::Combine => !matches!(j.employer, Employer::HighSchool(_)),
                    Trip::Clinic => matches!(j.employer, Employer::HighSchool(_)),
                })
                && warmth(w, *c) < 0.3
        })
        .map(|(c, _)| c)
        .collect();
    pool.sort_by(|&a, &b| {
        super::roll(w, 0x791 ^ w.year as u64, a as u64).total_cmp(&super::roll(w, 0x791 ^ w.year as u64, b as u64))
    });
    let met: Vec<usize> = pool.into_iter().take(n).collect();
    let year = w.year;
    for &c in &met {
        add_contact(w, c, 0.18, format!("met at {} {year}", t.name().to_lowercase().trim_start_matches("the ")));
    }
    // Old contacts there warm up.
    for x in w.folk.network.contacts.iter_mut().filter(|x| x.warmth >= 0.3).take(6) {
        x.warmth = (x.warmth + 0.03).min(1.0);
    }
    if code == 91 {
        w.folk.network.clinics += 1;
        if let Some(c) = you {
            w.coaches[c].reputation = (w.coaches[c].reputation + 0.08).min(20.0);
            w.coaches[c].renown = (w.coaches[c].name_value() + 0.08).min(20.0);
        }
    }
    let names: Vec<String> = met.iter().take(4).map(|&c| coach_link(w, c)).collect();
    s.outcome.push(format!(
        "You met {} coaches{}{}.",
        met.len(),
        if names.is_empty() { String::new() } else { format!(", among them {}", names.join(", ")) },
        if code == 91 { ", and gave a talk the room is still talking about" } else { "" }
    ));
    s.end();
}

/// The coaching all-star games (N6): a head coach whose team won big is
/// asked to coach in one (the Senior Bowl's staffs come from the NFL; the
/// East-West Shrine, the NFLPA bowl and state high school all-star games
/// take college and high school coaches).
fn allstar_day(w: &mut World, d: Date) {
    let Some(team) = Team::seat(w) else { return };
    let (when, game) = match team.level() {
        Level::Nfl => (w.dates.senior_bowl.add(-30), "the Senior Bowl"),
        Level::College => (w.dates.shrine_bowl.add(-30), "the East-West Shrine Bowl"),
        _ => (w.dates.all_american_bowl.add(-30), "the state all-star game"),
    };
    if d != when || w.folk.network.allstar.iter().any(|x| x.0 == w.year) {
        return;
    }
    let last = super::facts::seasons(w, team, 1);
    let good = last.last().is_some_and(|s| s.wins as f64 >= 0.7 * (s.wins + s.losses).max(1) as f64);
    if !good || super::roll(w, 0xA11, w.year as u64) > 0.5 {
        return;
    }
    let mut s = Scene::new(Kind::JobCall, d, format!("An invitation to coach in {game}"));
    s.a = 1;
    s.stops = false;
    s.team = Some(team);
    s.place = "On the phone".into();
    s.narrate(format!("The game's director calls: after your season, they want you to coach a team in {game}."));
    s.pick(95, "Accept", "Go", "warm", "A week with other staffs and a hundred prospects; your name grows.");
    s.pick(96, "Decline", "Skip", "calm", "Your January stays yours.");
    scene::add(w, s);
}

/// Interest in you from another school or club (N5): when a job opens where
/// someone who knows you works, or where your name fits, a call comes. It
/// costs you with your boss if it gets out.
fn interest_week(w: &mut World, d: Date) {
    let Some(team) = Team::seat(w) else { return };
    let carousel = match team.level() {
        Level::Nfl => d >= w.dates.black_monday && d <= w.dates.black_monday.add(28),
        _ => d >= w.dates.fbs_regular_end.add(-21) && d <= w.dates.fbs_regular_end.add(21),
    };
    if !carousel || w.folk.scenes.iter().any(|s| s.kind == Kind::JobCall && s.a == 0 && s.date.days_to(d) < 14) {
        return;
    }
    // Open head jobs this cycle: programs whose head coach left this year.
    let open: Vec<Team> = match team.level() {
        Level::Nfl => {
            (0..w.nfl.len()).filter(|&t| w.nfl[t].head_coach.is_none() && Team::Nfl(t) != team).map(Team::Nfl).collect()
        }
        _ => (0..w.programs.len())
            .filter(|&p| {
                w.programs[p].head_coach.is_none()
                    && Team::College(p) != team
                    && w.programs[p].division == crate::world::data::Division::FBS
            })
            .map(Team::College)
            .collect(),
    };
    let Some(&target) = open.iter().find(|t| {
        let size = super::facts::job_size(w, **t);
        // Each job reads your name and standing its own way (C10).
        let fits = super::facts::read_by(w, **t) / 20.0 >= 0.15 + 0.6 * size;
        let warm = w
            .folk
            .network
            .contacts
            .iter()
            .any(|x| x.warmth >= 0.45 && x.coach_id().and_then(|c| super::facts::team_of_coach(w, c)) == Some(**t));
        fits || warm
    }) else {
        return;
    };
    if super::roll(w, 0x1E7E ^ d.0 as u64, 2) > 0.5 {
        return;
    }
    let mut s = Scene::new(Kind::JobCall, d, format!("{} calls about their opening", target.name(w)));
    s.a = 0;
    s.b = target.key();
    s.stops = false;
    s.team = Some(team);
    s.place = "Your cell phone, a number you don't know".into();
    s.narrate(format!(
        "A search firm working for {} asks if you'd take a call about their head coaching job.",
        target.link(w)
    ));
    s.pick(
        97,
        "\"I'm listening.\"",
        "Listen",
        "calm",
        "Your name goes on their list. If it gets out, your boss and your players hear it.",
    );
    s.pick(
        98,
        "\"I'm happy where I am.\"",
        "Tell him straight",
        "firm",
        "They cross you off. Your boss would like to hear you said it.",
    );
    scene::add(w, s);
    w.folk.network.interest.push((w.year, target, "asked about you".into()));
    if w.folk.network.interest.len() > 30 {
        w.folk.network.interest.remove(0);
    }
}

fn call_answer(w: &mut World, s: &mut Scene, code: u32) {
    let Some(team) = s.team.or(Team::seat(w)) else {
        s.end();
        return;
    };
    match code {
        95 => {
            w.folk.network.allstar.push((w.year, s.title.trim_start_matches("An invitation to coach in ").to_string()));
            talks::charge(w, 6.0);
            if let Some(c) = w.career.as_ref().map(|x| x.coach) {
                w.coaches[c].reputation = (w.coaches[c].reputation + 0.15).min(20.0);
                w.coaches[c].renown = (w.coaches[c].name_value() + 0.2).min(20.0);
            }
            // A week with other staffs: new contacts.
            let you = w.career.as_ref().map(|c| c.coach);
            let met: Vec<usize> = w
                .coaches
                .iter()
                .enumerate()
                .filter(|(c, co)| Some(*c) != you && !co.retired && co.job.is_some_and(|j| j.role == Role::HeadCoach))
                .map(|x| x.0)
                .filter(|c| super::roll(w, 0xA12, *c as u64) < 0.01)
                .take(5)
                .collect();
            for c in met {
                add_contact(w, c, 0.2, "coached with you at an all-star game".into());
            }
            s.outcome.push(
                "You'll coach in the game. A week of practice, scouts everywhere, and your name in the program.".into(),
            );
        }
        96 => s.outcome.push("You declined.".into()),
        97 => {
            let leak = super::roll(w, 0x1EA5, w.today.0 as u64) < 0.35;
            s.outcome.push("They have your name.".into());
            if leak {
                let target = s.b;
                let what = w
                    .folk
                    .network
                    .interest
                    .iter()
                    .rev()
                    .find(|x| x.1.key() == target)
                    .map(|x| x.1.name(w))
                    .unwrap_or_default();
                let text = format!("Reports: your name is on {what}'s list.");
                super::post_mine(w, w.today.add(2), Loudness::Medium, false, "media: coaching search", &text);
                let b = super::boss::boss(w, team);
                w.folk.mine.nudge(b.key(), -0.08);
                for i in team.roster(w) {
                    w.people[i].trust = (w.people[i].trust - 0.015).max(0.0);
                }
                s.outcome.push("It got out. Your boss read it in the paper; so did your players.".into());
            }
        }
        _ => {
            let b = super::boss::boss(w, team);
            w.folk.mine.nudge(b.key(), 0.03);
            s.outcome.push("You told them no.".into());
        }
    }
    s.end();
}

// ---------------------------------------------------------------------------
// Favours asked of you (N3).

fn asks_week(w: &mut World, d: Date) {
    if w.folk.scenes.iter().any(|s| s.kind == Kind::ReachOut && s.date.days_to(d) < 21) {
        return;
    }
    let pick = w
        .folk
        .network
        .contacts
        .iter()
        .filter(|x| x.warmth >= 0.25)
        .filter_map(|x| x.coach_id())
        .find(|&c| super::roll(w, 0xA5E ^ d.0 as u64, c as u64) < 0.04);
    let Some(c) = pick else { return };
    let what = (super::roll(w, 0xA5F ^ d.0 as u64, c as u64) * 3.0) as u64;
    let mut s = Scene::new(Kind::ReachOut, d, format!("{} calls with a favour", w.coaches[c].name));
    s.a = c as u64;
    s.b = what;
    s.stops = false;
    s.team = Team::seat(w);
    s.place = "On the phone".into();
    scene::add(w, s);
}

fn ask_open(w: &mut World, s: &mut Scene) {
    let c = s.a as usize;
    s.who.push(Who::Coach(c));
    let name = w.coaches[c].name.clone();
    let line = match s.b {
        0 => {
            "My nephew wants to get into coaching. Any chance you have a graduate assistant spot, or even a volunteer job?"
        }
        1 => "I'm running our clinic in the spring. Would you come talk for an hour? It'd mean a lot to my guys.",
        _ => "I've got a kid I think can play for you. Would you look at his film?",
    };
    s.narrate(format!("{} calls.", coach_link(w, c)));
    s.them(&name, line, None);
    s.pick(85, "\"Of course. Send it over.\"", "Promise", "warm", "Costs some of your week; he owes you one.");
    s.pick(86, "\"I can't right now. I'm sorry.\"", "Tell him straight", "calm", "He understands, mostly.");
}

fn ask_answer(w: &mut World, s: &mut Scene, code: u32) {
    let c = s.a as usize;
    let k = w.folk.network.contacts.iter().position(|x| x.coach_id() == Some(c));
    if code == 85 {
        talks::charge(w, if s.b == 1 { 5.0 } else { 1.0 });
        if let Some(k) = k {
            w.folk.network.contacts[k].balance = (w.folk.network.contacts[k].balance + 1).min(5);
            w.folk.network.contacts[k].warmth = (w.folk.network.contacts[k].warmth + 0.08).min(1.0);
        }
        let words = match s.b {
            0 => "a spot for his nephew",
            1 => "a clinic talk",
            _ => "a look at his player",
        };
        w.folk.network.ledger.push(Favour { date: w.today, coach: c, words: words.into(), asked: false });
        if s.b == 1 {
            w.folk.network.clinics += 1;
        }
        trim_ledger(w);
        s.outcome.push(format!("You said yes to {}. He owes you one.", w.coaches[c].name));
    } else {
        if let Some(k) = k {
            w.folk.network.contacts[k].warmth = (w.folk.network.contacts[k].warmth - 0.04).max(0.0);
        }
        s.outcome.push("You said no.".into());
    }
    s.end();
}

pub(crate) fn open(w: &mut World, s: &mut Scene) {
    match s.kind {
        Kind::Trip => trip_open(w, s),
        Kind::ReachOut => ask_open(w, s),
        _ => {}
    }
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32, _picked: &Pick) {
    match s.kind {
        Kind::Trip => trip_answer(w, s, code),
        Kind::ReachOut => ask_answer(w, s, code),
        _ => call_answer(w, s, code),
    }
}

pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    s.handed = Some("nobody".into());
    match s.kind {
        Kind::Trip => s.outcome.push("You didn't go.".into()),
        Kind::ReachOut => {
            let c = s.a as usize;
            touch(w, c, -0.05);
            s.outcome.push("You never called him back.".into());
        }
        _ => s.outcome.push("You let the call go.".into()),
    }
}

/// Ties cool a little each month without contact (N1).
pub(crate) fn day(w: &mut World, d: Date) {
    if w.seat.is_none() {
        return;
    }
    if !w.folk.network.built {
        build(w);
    }
    if d.day() == 1 {
        let today = w.today;
        for x in w.folk.network.contacts.iter_mut() {
            let quiet = x.last.is_none_or(|l| l.days_to(today) > 60);
            if quiet && x.warmth > 0.12 {
                x.warmth -= 0.01;
            }
        }
    }
    trips_day(w, d);
    allstar_day(w, d);
    if d.weekday() == Weekday::Tue {
        interest_week(w, d);
        asks_week(w, d);
    }
}
