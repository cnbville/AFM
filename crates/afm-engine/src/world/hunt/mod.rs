//! The hunt (scouting update SK67, next update lane 11): one target
//! chased over seasons, at every level and position.
//!
//! You name one player as your hunt and the season bends around him. You
//! watch every game he plays (his film, the day after, from your own film
//! hours), you bank what he will cost (at a college NIL money set aside
//! and a scholarship held at his position; at an NFL club the picks to
//! reach him, read on your GM's chart; at a high school one of your
//! pitches and the family's trust built over the off-season), and you try
//! to keep anyone from learning you want him: every offer, visit, trip the
//! press saw and leak adds to how much the world knows ("heat"), and what
//! the world knows brings rivals. He has a say: a recruit cuts schools
//! from his list, a top prospect's camp refuses a club, a family says no.
//! Your boss watches a chase the world knows about. Potential stays
//! hidden: three years on the look-back says whether he was what the
//! hints said.
//!
//! It grows update 2's quarterback search (L2, `legacy::nfl`) to every
//! level and position; L2 stays as it is.
//!
//! The AI hunts too (`ai`): college head coaches name one target a class,
//! NFL clubs one a draft, private high school powers one a year, by the
//! same rules your seat plays by. Once or twice a decade a prospect stands
//! above every class before him and the whole world chases him
//! (`generational`).
//!
//! Proposed, not confirmed: every number below and in the two modules.

pub mod ai;
pub mod generational;

use crate::calendar::{Date, Event, Kind as CalKind, Level, Loudness, Weekday};
use crate::positions::Position;
use crate::world::data::Division;
use crate::world::scouting::readers::{self, Staff};
use crate::world::scouting::{self as sc, Kind};
use crate::world::{Stage, World};

/// A hunter's head coach gives the kid this much of his own time (an
/// ordinary target of his own gets 1.0, any other 0.35): calls, home
/// visits, his games.
pub const HUNT_TIME: f64 = 1.8;
/// The share of the head coach's time his other own targets lose while a
/// hunt runs in their class.
pub const HUNT_COST: f64 = 0.12;
/// What an AI college hunter will pay the kid at most, as a share of its
/// NIL money (a buying program's blue-chip share).
pub const HUNT_SHARE: f64 = 0.06;
/// An NFL club's draft-day value of the man it hunts, in board points:
/// it takes him at its pick and trades up for him more readily.
pub const HUNT_EDGE: f64 = 2.0;
/// Hours to watch one of his games: a high school game, a college game.
pub const FILM_HS: f32 = 1.0;
pub const FILM_COLLEGE: f32 = 1.5;
/// Hunts kept after they end, yours.
const PAST_MAX: usize = 12;

/// Where a hunt is run from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Seat {
    School,
    College,
    Club,
}

impl Seat {
    pub fn key(self) -> &'static str {
        match self {
            Seat::School => "hs",
            Seat::College => "college",
            Seat::Club => "nfl",
        }
    }
}

/// How a hunt ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum End {
    /// He came: signed, drafted or moved to your school.
    Landed,
    /// Someone else got him.
    Lost,
    /// He shut the door on you.
    Refused,
    /// You let him go.
    Dropped,
    /// Your boss ended it.
    Boss,
    /// He left football, or you left the job.
    Gone,
}

impl End {
    pub fn key(self) -> &'static str {
        match self {
            End::Landed => "landed",
            End::Lost => "lost",
            End::Refused => "refused",
            End::Dropped => "dropped",
            End::Boss => "boss",
            End::Gone => "gone",
        }
    }
}

/// One of his games: watched on film the day after, or missed when the
/// week's film hours were spent.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Watched {
    pub date: Date,
    pub against: String,
    pub seen: bool,
}

/// Your hunt.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Hunt {
    pub id: u32,
    pub name: String,
    pub pos: Position,
    pub seat: Seat,
    /// Your program, club or school when you named him.
    pub team: usize,
    pub since: Date,
    /// When you said it out loud.
    pub loud: Option<Date>,
    pub watched: Vec<Watched>,
    /// College: NIL money set aside for him, a year.
    pub nil: f64,
    /// College: a scholarship held at his position.
    pub hold: bool,
    /// High school: his family's trust in you, 0-1, built in the off-season.
    pub warmth: f32,
    /// High school: off-season weeks you were around him (people notice).
    pub contact: u16,
    /// College: Mondays in a row his list ran without you near the top.
    pub cold: u8,
    /// Your division when you named him (a college hunt's look-back).
    pub division: Option<Division>,
    pub end: Option<(Date, End, String)>,
    /// Three years on: hit or not, and what he became.
    pub look_back: Option<(Date, bool, String)>,
    /// The hunt's moments, newest last.
    pub log: Vec<(Date, String)>,
}

impl Hunt {
    pub fn open(&self) -> bool {
        self.end.is_none()
    }
    fn note(&mut self, d: Date, s: String) {
        self.log.push((d, s));
        if self.log.len() > 80 {
            self.log.remove(0);
        }
    }
}

/// Every hunt the world keeps: yours, your past ones, the AI's, and the
/// public's record of each class's best (the generational prospect).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Hunts {
    pub yours: Option<Hunt>,
    pub past: Vec<Hunt>,
    pub ai: Vec<ai::AiHunt>,
    pub tops: Vec<generational::Top>,
    pub generational: Vec<generational::Gen>,
}

/// Your seat as a hunter: where from and your team.
pub fn seat(w: &World) -> Option<(Seat, usize)> {
    w.career.as_ref()?;
    Some(match readers::seat_staff(w)? {
        Staff::Club(t) => (Seat::Club, t),
        Staff::Program(j) => (Seat::College, j),
        Staff::School(h) => (Seat::School, h),
    })
}

/// The class signing next (a college hunt's last year).
pub fn signing_class(w: &World) -> i32 {
    crate::world::recruiting::next_class(w)
}

/// Your open hunt.
pub fn yours(w: &World) -> Option<&Hunt> {
    w.hunt.yours.as_ref().filter(|h| h.open())
}

/// Your reader code (the scouting files' code for you).
fn me(w: &World) -> Option<u32> {
    readers::you(w).map(|c| readers::COACH + c as u32)
}

/// Whether you could make him your hunt, and why not.
pub fn can_hunt(w: &World, id: u32) -> Result<(), String> {
    let (s, team) = seat(w).ok_or("Start a career first.")?;
    let i = sc::place(w, id).ok_or("No such player.")?;
    let p = &w.people[i];
    let season = w.dates.season;
    match s {
        Seat::College => {
            crate::world::front_office::college::recruitable(w, i)?;
            if p.cal.signed.is_some() {
                return Err("He has signed: a letter binds him.".into());
            }
            if p.college == Some(team) {
                return Err("He's yours already.".into());
            }
        }
        Seat::Club => {
            if !matches!(p.stage, Stage::College | Stage::Declared) || p.pro.is_some() {
                return Err("At a club you hunt a college player on his way to the draft.".into());
            }
        }
        Seat::School => {
            if p.stage != Stage::HighSchool || p.class_of <= season {
                return Err("Only a high schooler with a season left.".into());
            }
            if p.hs == team {
                return Err("He plays for you already.".into());
            }
            if w.high_schools[p.hs].state != w.high_schools[team].state {
                return Err("Out of state: his family would have to move, and nobody moves for that.".into());
            }
        }
    }
    Ok(())
}

/// You name your hunt.
pub fn name(w: &mut World, id: u32) -> Result<String, String> {
    if let Some(h) = yours(w) {
        return Err(format!("Your hunt is {}: let him go first.", h.name));
    }
    can_hunt(w, id)?;
    let (s, team) = seat(w).ok_or("Start a career first.")?;
    let i = sc::place(w, id).ok_or("No such player.")?;
    let d = w.today;
    let p = &w.people[i];
    let (name, pos) = (p.name.clone(), p.player.position);
    let division = (s == Seat::College).then(|| w.programs[team].division);
    let mut h = Hunt {
        id,
        name: name.clone(),
        pos,
        seat: s,
        team,
        since: d,
        loud: None,
        watched: Vec::new(),
        nil: 0.0,
        hold: s == Seat::College,
        warmth: 0.0,
        contact: 0,
        cold: 0,
        division,
        end: None,
        look_back: None,
        log: Vec::new(),
    };
    let text = match s {
        Seat::College => {
            // On your board, with the head coach's own time.
            let _ = crate::world::front_office::college::add(w, id);
            if let Some(t) = w.front_office.college.board.iter_mut().find(|t| t.id == id) {
                t.priority = true;
            }
            format!(
                "Your hunt: {name}. You'll watch every game he plays, his position holds a scholarship for him, and your own time goes to him first. Keep it quiet as long as you can: offers, visits and trips get around."
            )
        }
        Seat::Club => format!(
            "Your hunt: {name}. You'll watch every game he plays on Saturdays. Keep your hand hidden: trips, visits and leaks tell the clubs ahead of you."
        ),
        Seat::School => format!(
            "Your hunt: {name}. You'll watch his Friday nights on film, and the off-season is when a family gets to know you. {}'s association bars recruiting: the more people know, the likelier it comes out.",
            w.states[w.high_schools[team].state].code
        ),
    };
    h.note(d, format!("You named {name} your hunt."));
    // A past hunt of the same player is kept in the record.
    w.hunt.yours = Some(h);
    Ok(text)
}

/// You let him go.
pub fn drop(w: &mut World) -> Result<String, String> {
    let name = yours(w).ok_or("You have no hunt.")?.name.clone();
    close(w, End::Dropped, format!("You let {name} go."));
    Ok(format!("You let {name} go. Your staff's time goes back to the board."))
}

/// You say it out loud: the press has it the same day.
pub fn loud(w: &mut World) -> Result<String, String> {
    let h = yours(w).ok_or("You have no hunt.")?;
    if h.loud.is_some() {
        return Err("You've said it already.".into());
    }
    let (id, name, pos) = (h.id, h.name.clone(), h.pos);
    let d = w.today;
    let you = crate::world::folk::scene::your_name(w);
    let (s, team) = seat(w).ok_or("Start a career first.")?;
    let (level, words) = match s {
        Seat::Club => {
            (Level::Nfl, format!("{you} of the {} on {name} ({pos}): \"He's the one we want.\"", w.nfl[team].name))
        }
        Seat::College => (
            Level::Recruiting,
            format!("{you} of {} on {name} ({pos}): \"He's the one we want.\"", w.program_name(team)),
        ),
        Seat::School => {
            (Level::HighSchool, format!("{you} of {} says {name} ({pos}) would be welcome there.", w.school_name(team)))
        }
    };
    let mut e = Event::new(d, level, CalKind::News, Loudness::Medium, words.clone()).person(id, &name);
    match s {
        Seat::Club => e = e.team(team),
        Seat::College => e = e.program(team),
        Seat::School => {}
    }
    w.calendar.post(e);
    if s == Seat::Club {
        w.scouting.predraft.leaks.push(sc::predraft::Leak { team, player: id, date: d, real: true, yours: true });
    }
    if let Some(h) = w.hunt.yours.as_mut() {
        h.loud = Some(d);
        h.note(d, "You said it out loud.".into());
    }
    Ok(words)
}

/// What you bank for him at a college: NIL money set aside a year, and
/// whether his position holds a scholarship.
pub fn bank(w: &mut World, nil: Option<f64>, hold: Option<bool>) -> Result<String, String> {
    let h = yours(w).ok_or("You have no hunt.")?;
    if h.seat != Seat::College {
        return Err("Only a college banks money and a scholarship for a recruit.".into());
    }
    let j = h.team;
    let current = h.nil;
    if let Some(x) = nil {
        if x < 0.0 {
            return Err("You can't set aside less than nothing.".into());
        }
        if !crate::world::ncaa::nil_era(w.today.year()) && x > 0.0 {
            return Err("NIL deals are allowed from July 1, 2021.".into());
        }
        let b = crate::world::front_office::college::budget(w, j);
        if x - current > b.left + 1.0 {
            return Err(format!(
                "That's more than the {} left in your NIL budget.",
                crate::world::front_office::dollars(b.left.max(0.0))
            ));
        }
    }
    let d = w.today;
    let h = w.hunt.yours.as_mut().unwrap();
    if let Some(x) = nil {
        h.nil = x;
        h.note(d, format!("Set aside {} a year for him.", crate::world::front_office::dollars(x)));
    }
    if let Some(on) = hold {
        h.hold = on;
        h.note(
            d,
            if on { "Held a scholarship at his position." } else { "Let his scholarship go to the board." }.into(),
        );
    }
    Ok("Banked.".into())
}

/// The money set aside for your hunt that isn't promised to him yet (it
/// is out of your NIL budget for anyone else).
pub fn reserve(w: &World, j: usize) -> f64 {
    let Some(h) = yours(w).filter(|h| h.seat == Seat::College && h.team == j) else { return 0.0 };
    let promised = w
        .front_office
        .college
        .board
        .iter()
        .find(|t| t.id == h.id && t.offered.is_some() && t.pulled.is_none())
        .and_then(|t| t.nil)
        .unwrap_or(0.0);
    (h.nil - promised).max(0.0)
}

/// A sign the world can read that you want him.
#[derive(Clone, Debug)]
pub struct Signal {
    pub date: Date,
    pub what: String,
    pub weight: f64,
}

/// How much the world knows you want him, 0-1, and from what: each sign
/// adds its weight to what's left (1 - the product of what each leaves
/// hidden).
pub fn heat(w: &World, h: &Hunt) -> (f64, Vec<Signal>) {
    let mut v: Vec<Signal> = Vec::new();
    let Some(i) = sc::place(w, h.id) else { return (0.0, v) };
    let p = &w.people[i];
    if let Some(d) = h.loud {
        v.push(Signal { date: d, what: "You said it out loud".into(), weight: 1.0 });
    }
    for t in w.scouting.own.trips.iter().filter(|t| t.id == h.id && t.seen && t.date >= h.since) {
        v.push(Signal {
            date: t.date,
            what: format!("You were seen: {}", t.kind.words().to_lowercase()),
            weight: 0.25,
        });
    }
    match h.seat {
        Seat::College => {
            if let Some(t) = w.front_office.college.board.iter().find(|t| t.id == h.id)
                && let Some(d) = t.offered.filter(|_| t.pulled.is_none())
            {
                v.push(Signal { date: d, what: "Your offer: recruits post them".into(), weight: 0.3 });
            }
            for &(j, d, kind) in &p.cal.visits {
                if j == h.team && d >= h.since {
                    let (what, wt) = match kind {
                        crate::world::Visit::Official => ("His official visit", 0.2),
                        _ => ("His visit to campus", 0.1),
                    };
                    v.push(Signal { date: d, what: what.into(), weight: wt });
                }
            }
        }
        Seat::Club => {
            for l in
                w.scouting.predraft.leaks.iter().filter(|l| l.team == h.team && l.player == h.id && l.date >= h.since)
            {
                if h.loud != Some(l.date) {
                    v.push(Signal { date: l.date, what: "A leak in the press".into(), weight: 0.2 });
                }
            }
            for &(id, list) in &w.scouting.predraft.lists {
                if id == h.id && matches!(list, sc::predraft::LIST_VISIT | sc::predraft::LIST_WORKOUT) {
                    let what = if list == sc::predraft::LIST_VISIT { "His top-30 visit" } else { "A private workout" };
                    v.push(Signal { date: h.since, what: what.into(), weight: 0.15 });
                }
            }
        }
        Seat::School => {
            for x in w.scouting.pitches.list.iter().filter(|x| x.id == h.id && x.date >= h.since) {
                v.push(Signal { date: x.date, what: "You talked to his family".into(), weight: 0.3 });
            }
            if h.contact > 0 {
                v.push(Signal {
                    date: h.since,
                    what: format!("{} off-season weeks around him", h.contact),
                    weight: 1.0 - 0.97f64.powi(h.contact as i32),
                });
            }
        }
    }
    // People talk: every three months a chase runs, someone on a staff
    // or around the kid lets it slip.
    let quarters = ((w.today.0 - h.since.0) / 91).max(0);
    if quarters > 0 {
        v.push(Signal {
            date: h.since.add(91),
            what: format!("Word gets around ({quarters} season{})", if quarters == 1 { "" } else { "s" }),
            weight: 1.0 - 0.95f64.powi(quarters),
        });
    }
    let hidden: f64 = v.iter().map(|s| 1.0 - s.weight.clamp(0.0, 1.0)).product();
    v.sort_by_key(|s| s.date);
    (1.0 - hidden, v)
}

/// Your hunt's heat now (0 without one).
pub fn heat_now(w: &World) -> f64 {
    yours(w).map_or(0.0, |h| heat(w, h).0)
}

/// Whether your hunt is this player.
pub fn is_yours(w: &World, id: u32) -> bool {
    yours(w).is_some_and(|h| h.id == id)
}

/// Ends your hunt: the boss weighs a chase the world knew about, the
/// inbox has the line, and it goes to the record.
fn close(w: &mut World, end: End, why: String) {
    let Some(h) = w.hunt.yours.as_ref() else { return };
    let d = w.today;
    let heat = heat(w, h).0;
    let name = h.name.clone();
    // A chase the world saw is the boss's business too.
    if heat >= 0.3 {
        let by = match end {
            End::Landed => 0.02 + 0.03 * heat,
            End::Lost | End::Refused => -0.03 * heat,
            _ => 0.0,
        };
        if by != 0.0 {
            crate::world::legacy::boss_by(w, by as f32);
        }
    }
    let line = match end {
        End::Landed => format!("The hunt is over: {name} is yours. {why}"),
        End::Lost => format!("The hunt is over: {why}"),
        End::Refused => format!("The hunt is over: {why}"),
        End::Boss | End::Dropped | End::Gone => why.clone(),
    };
    crate::world::front_office::tell(w, d, line, false);
    let mut h = w.hunt.yours.take().unwrap();
    h.end = Some((d, end, why.clone()));
    h.note(d, why);
    w.hunt.past.push(h);
    if w.hunt.past.len() > PAST_MAX {
        // The oldest settled one goes first.
        let k = w.hunt.past.iter().position(|x| x.look_back.is_some()).unwrap_or(0);
        w.hunt.past.remove(k);
    }
}

/// The daily hunt: the world's hunts and the generational prospect, then
/// yours: his games watched, whether he's still out there, his say, your
/// boss, and the look-backs.
pub(crate) fn day(w: &mut World, d: Date) {
    generational::day(w, d);
    ai::day(w, d);
    if w.seat.is_none() {
        return;
    }
    if d.parts() == (d.year(), 9, 1) {
        look_backs(w, d);
    }
    if yours(w).is_none() {
        return;
    }
    // Leaving the job ends the chase.
    if seat(w).is_none_or(|(s, t)| {
        let h = yours(w).unwrap();
        s != h.seat || t != h.team
    }) {
        close(w, End::Gone, "You left the job: the hunt ends with it.".into());
        return;
    }
    if status(w, d) {
        return;
    }
    watch(w, d);
    if d.weekday() == Weekday::Mon {
        weekly(w, d);
    }
    if yours(w).is_some() && d.parts().2 == 1 {
        patience(w);
    }
}

/// Whether he's still out there: signed, drafted, moved or gone ends it.
/// Returns true once the hunt has ended.
fn status(w: &mut World, d: Date) -> bool {
    let h = yours(w).unwrap();
    let (id, team, s, name) = (h.id, h.team, h.seat, h.name.clone());
    let Some(i) = sc::place(w, id) else {
        close(w, End::Gone, format!("{name} is out of football."));
        return true;
    };
    let p = &w.people[i];
    let out: Option<(End, String)> = match s {
        Seat::College => match p.cal.signed {
            Some(sg) if sg.program == team => {
                Some((End::Landed, format!("He signed with you on {}.", sg.date.pretty())))
            }
            Some(sg) => Some((End::Lost, format!("{name} signed with {}.", w.program_name(sg.program)))),
            None if !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) && p.college.is_none() => {
                Some((End::Gone, format!("{name} never signed: he's done with football.")))
            }
            None if p.stage == Stage::College && p.college != Some(team) => Some((
                End::Lost,
                format!("{name} went to {}.", p.college.map_or("another school", |c| w.program_name(c))),
            )),
            None => None,
        },
        Seat::Club => match p.pro.as_ref() {
            Some(pro) => {
                let club = pro.draft.map(|x| x.3).or(pro.team);
                let how = match pro.draft {
                    Some((y, r, k, _)) => format!("in round {r} of the {y} draft (No. {k})"),
                    None => "after the draft".into(),
                };
                if club == Some(team) {
                    Some((End::Landed, format!("You took him {how}.")))
                } else {
                    let to = club.filter(|&c| c < w.nfl.len()).map_or("another club".into(), |c| w.nfl_team_name(c));
                    Some((End::Lost, format!("{name} went to the {to} {how}.")))
                }
            }
            None if !matches!(p.stage, Stage::College | Stage::Declared) => {
                Some((End::Gone, format!("{name} went undrafted and unsigned.")))
            }
            None => None,
        },
        Seat::School => {
            if p.hs == team && p.stage == Stage::HighSchool {
                Some((End::Landed, format!("He transferred in from his old school on {}.", d.pretty())))
            } else if p.stage != Stage::HighSchool
                || p.class_of <= w.dates.season && d.parts().1 >= 6 && d.parts().1 <= 7
            {
                Some((End::Lost, format!("{name} finished high school at {}.", w.school_name(p.hs))))
            } else {
                None
            }
        }
    };
    if let Some((end, why)) = out {
        close(w, end, why);
        return true;
    }
    false
}

/// The day after each of his games: you watch it on film, from your own
/// film hours (a missed game when the week's hours are spent).
fn watch(w: &mut World, d: Date) {
    let h = yours(w).unwrap();
    let (id, pos, s) = (h.id, h.pos, h.seat);
    let Some(i) = sc::place(w, id) else { return };
    let p = &w.people[i];
    let yday = d.add(-1);
    let (against, kind, hours) = match p.stage {
        Stage::HighSchool => {
            let Some(f) = w.hs_fixtures(p.hs).into_iter().find(|f| f.date == yday) else { return };
            let (opp, home) = if f.home == p.hs { (f.away, true) } else { (f.home, false) };
            (format!("{} {}", if home { "vs" } else { "at" }, w.school_name(opp)), Kind::HsFilm, FILM_HS)
        }
        Stage::College => {
            let Some(c) = p.college else { return };
            let Some(f) = w.college_fixtures(Some(c)).into_iter().find(|f| f.date == yday && f.score.is_some()) else {
                return;
            };
            let (opp, home) = if f.home == c { (f.away, true) } else { (f.home, false) };
            let sc = f.score.unwrap_or([0, 0]);
            let (us, them) = if home { (sc[0], sc[1]) } else { (sc[1], sc[0]) };
            let r = if us > them {
                "W"
            } else if us < them {
                "L"
            } else {
                "T"
            };
            (
                format!("{} {}, {r} {us}-{them}", if home { "vs" } else { "at" }, w.program_name(opp)),
                Kind::CollegeFilm,
                FILM_COLLEGE,
            )
        }
        _ => return,
    };
    if yours(w).unwrap().watched.iter().any(|x| x.date == yday) {
        return;
    }
    let seen = sc::film::time::spend(w, hours, Some(pos)).is_ok();
    if seen && let Some(you) = me(w) {
        sc::dept::add(w, id, you, kind, d);
        // At a club, the head coach's look goes in the club's file too.
        if s == Seat::Club
            && let Some(Staff::Club(c)) = readers::seat_staff(w)
            && w.nfl.get(c).and_then(|x| x.head_coach) == readers::you(w)
        {
            sc::dept::club_look(w, c, id, crate::world::pro::scouts::HC, crate::world::pro::scouts::LookKind::Film, d);
        }
    }
    let h = w.hunt.yours.as_mut().unwrap();
    h.watched.push(Watched { date: yday, against: against.clone(), seen });
    if !seen {
        h.note(d, format!("Missed his game ({against}): your film hours were spent."));
    }
}

/// Mondays: a college recruit's list (his say), a family's trust at a high
/// school, rivals who hear of your interest.
fn weekly(w: &mut World, d: Date) {
    let h = yours(w).unwrap().clone();
    let heat = heat(w, &h).0;
    match h.seat {
        Seat::College => {
            door(w, &h);
            if yours(w).is_some() {
                ai::pile_on(w, d, &h, heat);
            }
        }
        Seat::Club => ai::pile_on(w, d, &h, heat),
        Seat::School => {
            // The off-season (January to July) is when a coach is around
            // a kid: 7-on-7s, camps, his family in town. In season you
            // coach your own Fridays.
            let m = d.parts().1;
            if (1..=7).contains(&m) {
                let h = w.hunt.yours.as_mut().unwrap();
                h.warmth = (h.warmth + 0.03).min(1.0);
                h.contact += 1;
            }
        }
    }
}

/// His list (college): once he's in the class signing next, a kid whose
/// list keeps you outside his top three tells your staff to stop calling
/// (a proud kid sooner, a loyal one later).
fn door(w: &mut World, h: &Hunt) {
    let Some(i) = sc::place(w, h.id) else { return };
    let p = &w.people[i];
    if p.class_of != crate::world::recruiting::next_class(w) || p.cal.signed.is_some() {
        return;
    }
    let offered =
        w.front_office.college.board.iter().any(|t| t.id == h.id && t.offered.is_some() && t.pulled.is_none());
    if !offered {
        return;
    }
    let mine = crate::world::recruiting::appeal(w, i, h.team, None);
    let mut ahead = 0;
    let mut names: Vec<(f64, usize)> = Vec::new();
    for &j in p.cal.offers.iter().filter(|&&j| j != h.team) {
        let a = crate::world::recruiting::appeal(w, i, j, None);
        names.push((a, j));
        if a > mine {
            ahead += 1;
        }
    }
    let cold = if ahead >= 3 { h.cold.saturating_add(1) } else { h.cold.saturating_sub(1) };
    let ego = p.player.personality.traits.ego as f64;
    let loyal = p.player.personality.traits.loyalty as f64;
    let weeks = (8.0 - 0.3 * (ego - 10.0) + 0.3 * (loyal - 10.0)).round().clamp(4.0, 12.0) as u8;
    w.hunt.yours.as_mut().unwrap().cold = cold;
    if cold >= weeks {
        names.sort_by(|a, b| b.0.total_cmp(&a.0));
        let top: Vec<&str> = names.iter().take(3).map(|x| w.program_name(x.1)).collect();
        let name = h.name.clone();
        close(w, End::Refused, format!("{name} told your staff to stop calling: his list is {}.", top.join(", ")));
    }
}

/// The first of each month: a chase the world knows about wears on a boss
/// while your team loses, and at the end of his patience he ends it.
fn patience(w: &mut World) {
    let h = yours(w).unwrap().clone();
    let heat = heat(w, &h).0;
    if heat < 0.5 {
        return;
    }
    let losing = match h.seat {
        Seat::Club => {
            let (wn, l, _) = w.nfl_record(h.team);
            l >= wn + 3
        }
        Seat::College => {
            let r = w.college_record(h.team);
            r.losses >= r.wins + 2
        }
        Seat::School => w.hs_record(h.team).is_some_and(|(wn, l)| l >= wn + 2),
    };
    if losing {
        crate::world::legacy::boss_by(w, -0.01);
    }
    if crate::world::legacy::boss_trust(w) < 0.2 {
        let boss = match h.seat {
            Seat::Club => "owner",
            Seat::College => "athletic director",
            Seat::School => "principal",
        };
        close(w, End::Boss, format!("Your {boss} has run out of patience with the chase for {}: it's over.", h.name));
    }
}

/// September 1: a hunt you won, three years on: was he what the hints said?
fn look_backs(w: &mut World, d: Date) {
    let due: Vec<usize> = (0..w.hunt.past.len())
        .filter(|&k| {
            let h = &w.hunt.past[k];
            h.look_back.is_none() && h.end.as_ref().is_some_and(|e| e.1 == End::Landed && e.0.add(3 * 365) <= d)
        })
        .collect();
    for k in due {
        let h = w.hunt.past[k].clone();
        let res: Option<(bool, String)> = match h.seat {
            Seat::College => {
                Some(crate::world::front_office::chain::turned(w, h.id, h.division.unwrap_or(Division::FBS)))
            }
            Seat::Club => sc::place(w, h.id).and_then(|i| sc::stake::nfl_turned(w, i, h.end.as_ref().unwrap().0)),
            Seat::School => Some(sc::place(w, h.id).map_or((false, "out of football".into()), |i| {
                let p = &w.people[i];
                if let Some(c) = p.college.or(p.cal.signed.map(|s| s.program))
                    && matches!(w.programs[c].division, Division::FBS | Division::FCS)
                {
                    (true, format!("he went on to {}", w.program_name(c)))
                } else if p.cal.all_state > 0 {
                    (true, "all-state".into())
                } else {
                    (false, "he never became the player you saw".into())
                }
            })),
        };
        let Some((hit, words)) = res else { continue };
        let loud = h.loud.is_some();
        let rep = match (hit, loud) {
            (true, false) => 0.3,
            (true, true) => 0.6,
            (false, false) => -0.2,
            (false, true) => -0.6,
        };
        if let Some(c) = readers::you(w) {
            w.coaches[c].reputation = (w.coaches[c].reputation + rep).clamp(1.0, 99.0);
        }
        let line = if hit {
            format!("Three years on, the hunt for {} paid off: {words}.", h.name)
        } else {
            format!("Three years on, {} wasn't what the hunt was for: {words}.", h.name)
        };
        crate::world::front_office::tell(w, d, line.clone(), false);
        if loud {
            let you = crate::world::folk::scene::your_name(w);
            let lv = match h.seat {
                Seat::Club => Level::Nfl,
                Seat::College => Level::College,
                Seat::School => Level::HighSchool,
            };
            w.calendar.post(
                Event::new(d, lv, CalKind::News, Loudness::Small, format!("{you}'s big chase, {}: {words}.", h.name))
                    .person(h.id, &h.name),
            );
        }
        let x = &mut w.hunt.past[k];
        x.look_back = Some((d, hit, words));
        x.note(d, line);
    }
}

/// What reaching him costs at a club (the hunt's bank): his projected
/// place by the public's consensus, your first pick in his draft and your
/// picks on your GM's chart this draft and next.
#[derive(Clone, Debug, Default)]
pub struct Capital {
    pub draft: i32,
    /// The public's rank of him (consensus top 50), if ranked.
    pub rank: Option<usize>,
    /// Your first pick in his draft, if the order is set.
    pub first: Option<usize>,
    /// Points: what moving to his place takes over your first pick.
    pub gap: Option<f64>,
    /// Your picks' points this draft and next.
    pub this_year: f64,
    pub next_year: f64,
}

pub fn capital(w: &World, t: usize, id: u32) -> Capital {
    let dy = sc::public::draft_year(w);
    let mut c = Capital { draft: dy, ..Default::default() };
    let classic = w.clubs.get(t).is_none_or(|c| c.office.gm.classic_chart);
    let teams = crate::world::franchise::active_teams_in(w, dy);
    for (k, y) in [dy, dy + 1].into_iter().enumerate() {
        let mut sum = 0.0;
        for r in 1..=7u32 {
            for &orig in &teams {
                if w.market.picks.owner_of(y, r, orig) == t {
                    sum += crate::world::pro::trades::pick_value(w, t, y, r, orig);
                }
            }
        }
        if k == 0 {
            c.this_year = sum;
        } else {
            c.next_year = sum;
        }
    }
    c.rank = sc::public::consensus(w).and_then(|v| v.iter().position(|&x| x == id)).map(|k| k + 1);
    if let Some(order) = crate::world::nfl_year::draft_order(w).filter(|_| dy == w.year) {
        let n = order.len();
        let mut picks: Vec<usize> = Vec::new();
        for r in 1..=7u32 {
            for (k, &orig) in order.iter().enumerate() {
                if w.market.picks.owner_of(dy, r, orig) == t {
                    picks.push((r as usize - 1) * n + k + 1);
                }
            }
        }
        c.first = picks.into_iter().min();
    }
    if let (Some(rank), Some(first)) = (c.rank, c.first) {
        c.gap = Some(if rank < first {
            crate::world::pro::trades::chart(classic, rank as f64)
                - crate::world::pro::trades::chart(classic, first as f64)
        } else {
            0.0
        });
    }
    c
}
