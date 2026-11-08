//! Talking to people (people topic 5): one-on-ones with players, team
//! talks, parents' calls, exit meetings and the lines you give your
//! position coaches. You pick a move and a tone; he answers in written
//! lines with tells your read catches, over two to five exchanges. A move
//! that fits his personality and timing moves trust and mood the right way;
//! a wrong one backfires. Promises are logged and checked on their day.
//!
//! The mood parts the world keeps (role, winning, money, coach, life, room)
//! are built in `people.rs`; the parts added here (talks, promises, friends,
//! noise, discipline) are dated reasons that fade by about a quarter a month
//! (people topic 3), stored only for players who have one.
//!
//! Sizes no source publishes are Claude's picks (proposed, not confirmed):
//! ten hours of people time a week, the move table below, the fade.

use super::scene::{self, Kind, Pick, Scene, Who};
use super::takeover::{Message, Tone};
use super::voice::{Facts, Speaker};
use super::{Team, player_link};
use crate::calendar::{Date, Level, Loudness, Weekday};
use crate::player::{PlayingTime, Trait};
use crate::world::people::Why;
use crate::world::{Stage, World};

/// What a one-on-one is about (topic 5): you pick it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Subject {
    Role = 0,
    Usage,
    Money,
    Behavior,
    Life,
    Team,
    /// His future: staying, the portal, the draft, a new deal (exit
    /// meetings and flight risks).
    Future,
    /// A close friend who just left the team (cut, traded, transferred):
    /// on his mind for a few weeks, and a subject only while it is (new
    /// bugs, Cain 2026-11-03).
    Friend,
}

impl Subject {
    /// The subjects you can always raise.
    pub const ALL: [Subject; 7] = [
        Subject::Role,
        Subject::Usage,
        Subject::Money,
        Subject::Behavior,
        Subject::Life,
        Subject::Team,
        Subject::Future,
    ];
    /// Every subject, the ones that come and go included.
    pub const EVERY: [Subject; 8] = [
        Subject::Role,
        Subject::Usage,
        Subject::Money,
        Subject::Behavior,
        Subject::Life,
        Subject::Team,
        Subject::Future,
        Subject::Friend,
    ];
    pub fn from_u64(v: u64) -> Subject {
        Subject::EVERY.get(v as usize).copied().unwrap_or(Subject::Role)
    }
    pub fn from_key(k: &str) -> Option<Subject> {
        Subject::EVERY.iter().copied().find(|s| s.key() == k)
    }
    pub fn name(self) -> &'static str {
        match self {
            Subject::Role => "his role",
            Subject::Usage => "his touches",
            Subject::Money => "his money",
            Subject::Behavior => "his behavior",
            Subject::Life => "life away from football",
            Subject::Team => "the team",
            Subject::Future => "his future",
            Subject::Friend => "his friend leaving",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Subject::Role => "role",
            Subject::Usage => "usage",
            Subject::Money => "money",
            Subject::Behavior => "behavior",
            Subject::Life => "life",
            Subject::Team => "team",
            Subject::Future => "future",
            Subject::Friend => "friend",
        }
    }
}

/// The moves of a talk (topic 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Ask,
    Straight,
    Praise,
    Challenge,
    Criticize,
    Promise,
    Warn,
    Hand,
    Listen,
}

impl Move {
    pub const ALL: [Move; 9] = [
        Move::Ask,
        Move::Straight,
        Move::Praise,
        Move::Challenge,
        Move::Criticize,
        Move::Promise,
        Move::Warn,
        Move::Hand,
        Move::Listen,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Move::Ask => "Ask",
            Move::Straight => "Tell him straight",
            Move::Praise => "Praise",
            Move::Challenge => "Challenge",
            Move::Criticize => "Criticize",
            Move::Promise => "Promise",
            Move::Warn => "Warn",
            Move::Hand => "Hand him something",
            Move::Listen => "Listen",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Move::Ask => "ask",
            Move::Straight => "straight",
            Move::Praise => "praise",
            Move::Challenge => "challenge",
            Move::Criticize => "criticize",
            Move::Promise => "promise",
            Move::Warn => "warn",
            Move::Hand => "hand",
            Move::Listen => "listen",
        }
    }
}

/// A pick's code: move times ten plus tone; 900 ends the talk.
fn code_of(m: Move, t: Tone) -> u32 {
    100 + 10 * Move::ALL.iter().position(|x| *x == m).unwrap_or(0) as u32 + tone_index(t)
}

fn tone_index(t: Tone) -> u32 {
    match t {
        Tone::Calm => 0,
        Tone::Warm => 1,
        Tone::Firm => 2,
        Tone::Fiery => 3,
    }
}

fn decode(code: u32) -> Option<(Move, Tone)> {
    if !(100..190).contains(&code) {
        return None;
    }
    let m = *Move::ALL.get(((code - 100) / 10) as usize)?;
    Some((m, Tone::from(code % 10)))
}

// ---------------------------------------------------------------------------
// Promises (topic 5; section 5's promises at every level).

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PromiseTo {
    Player(u32),
    Cast(super::cast::CastRef, u16),
    Coach(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PromiseKind {
    /// He starts.
    Start,
    /// More snaps or touches than now.
    Snaps,
    /// A contract talk (NFL) or an NIL push (college).
    Contract,
    /// You'll be here (you won't leave him for another job).
    Stay,
    /// A booster's want (takeover item 14).
    Booster(super::cast::Want),
    /// A raise or a title for an assistant.
    Raise,
    /// His place doesn't change: still on your team at the season's end, in
    /// the role he had (the rank of his `PlayingTime` when you said it).
    Place(u8),
}

impl PromiseKind {
    pub fn words(self) -> String {
        match self {
            PromiseKind::Start => "a starting job".into(),
            PromiseKind::Snaps => "more snaps".into(),
            PromiseKind::Contract => "a contract talk".into(),
            PromiseKind::Stay => "that you'd stay".into(),
            PromiseKind::Booster(w) => w.short().into(),
            PromiseKind::Raise => "a raise".into(),
            PromiseKind::Place(0) => "that he'd keep his starting job".into(),
            PromiseKind::Place(_) => "that his place wouldn't change".into(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Promise {
    pub to: PromiseTo,
    pub kind: PromiseKind,
    pub made: Date,
    pub due: Date,
    /// Said in public (a press conference): breaking it costs double.
    pub public: bool,
    pub kept: Option<bool>,
    /// Who, in words with link marks.
    pub who: String,
}

/// Logs a promise. `seasons` sets its horizon: 0 is the next checkpoint
/// for that kind (the season's first game for a start, the season's end for
/// snaps), 1 or more counts seasons from now.
pub fn promise(w: &mut World, to: PromiseTo, kind: PromiseKind, seasons: u8) {
    let today = w.today;
    let due = promise_due(w, kind, seasons);
    let who = match to {
        PromiseTo::Player(id) => super::person(w, id).map_or_else(|| "a player".into(), |i| player_link(w, i)),
        PromiseTo::Cast(r, term) => {
            let c = super::cast::get(w, r);
            if c.term == term { c.link() } else { "a man no longer in the job".into() }
        }
        PromiseTo::Coach(c) => super::coach_link(w, c),
    };
    w.folk.promises.retain(|p| !(p.to == to && p.kind == kind && p.kept.is_none()));
    w.folk.promises.push(Promise { to, kind, made: today, due, public: false, kept: None, who });
    if w.folk.promises.len() > 120
        && let Some(k) = w.folk.promises.iter().position(|p| p.kept.is_some())
    {
        w.folk.promises.remove(k);
    }
}

fn promise_due(w: &World, kind: PromiseKind, seasons: u8) -> Date {
    let today = w.today;
    let kickoff = |w: &World| match Team::seat(w).map(|t| t.level()) {
        Some(Level::Nfl) => w.dates.nfl_kickoff,
        Some(Level::HighSchool) => w.dates.hs_school_start.add(14),
        _ => w.dates.cfb_weeks.first().copied().unwrap_or(w.dates.fall_camp.add(30)),
    };
    let end = |w: &World| match Team::seat(w).map(|t| t.level()) {
        Some(Level::Nfl) => w.dates.black_monday,
        _ => w.dates.fbs_regular_end.add(3),
    };
    let base = match kind {
        PromiseKind::Start => {
            let k = kickoff(w).add(8);
            if k > today { k } else { k.add(365) }
        }
        PromiseKind::Snaps | PromiseKind::Place(_) => {
            let e = end(w);
            if e > today.add(30) { e } else { e.add(365) }
        }
        PromiseKind::Contract => today.add(150),
        PromiseKind::Stay | PromiseKind::Raise => today.add(330),
        PromiseKind::Booster(_) => end(w).max(today.add(60)),
    };
    base.add(
        365 * seasons.saturating_sub(1) as i32 * matches!(kind, PromiseKind::Booster(_) | PromiseKind::Stay) as i32,
    )
}

/// Checks the promises due today: kept ones build trust, broken ones cost
/// it with him, his friends and (said in public) the room.
fn check_promises(w: &mut World, d: Date) {
    let due: Vec<usize> = (0..w.folk.promises.len())
        .filter(|&k| w.folk.promises[k].kept.is_none() && w.folk.promises[k].due <= d)
        .collect();
    for k in due {
        let p = w.folk.promises[k].clone();
        // A promise of his place lapses, quietly, once he left on his own.
        if lapsed(w, &p) {
            w.folk.promises[k].kept = Some(true);
            continue;
        }
        let kept = kept(w, &p);
        w.folk.promises[k].kept = Some(kept);
        let cost = if p.public { 2.0 } else { 1.0 };
        let text = format!(
            "{} the promise of {} you made to {} on {}.",
            if kept { "You kept" } else { "You broke" },
            p.kind.words(),
            p.who,
            p.made.pretty()
        );
        match p.to {
            PromiseTo::Player(id) => {
                if let Some(i) = super::person(w, id) {
                    let dt = if kept { 0.1 } else { -0.22 * cost as f32 };
                    let pp = &mut w.people[i];
                    pp.trust = (pp.trust + dt).clamp(0.0, 1.0);
                    mood_event(w, id, if kept { Reason::PromiseKept } else { Reason::PromiseBroken }, dt * 1.4);
                    if !kept {
                        for f in super::ties::friends(w, id) {
                            if let Some(j) = super::person(w, f) {
                                w.people[j].trust = (w.people[j].trust - 0.04 * cost as f32).max(0.0);
                            }
                        }
                        if let Some(c) = w.people[i].pro.as_ref().and_then(|c| c.team) {
                            crate::world::people::grudge(&mut w.people[i], c, 0.6);
                        }
                        w.folk.mine.straight = (w.folk.mine.straight - 0.08 * cost as f32).max(-1.0);
                    } else {
                        w.folk.mine.straight = (w.folk.mine.straight + 0.04).min(1.0);
                    }
                }
            }
            PromiseTo::Cast(r, term) => {
                let c = super::cast::get(w, r);
                if c.term == term {
                    w.folk.mine.nudge(c.key(), if kept { 0.1 } else { -0.2 * cost as f32 });
                }
            }
            PromiseTo::Coach(c) => w.folk.mine.nudge(super::coach_key(c), if kept { 0.1 } else { -0.2 }),
        }
        super::note(w, "talk", "Your promises".into(), text.clone(), None);
        super::post_mine(w, d, Loudness::Small, false, "people: promise", &text);
    }
}

pub(crate) fn kept(w: &World, p: &Promise) -> bool {
    match (p.to, p.kind) {
        (PromiseTo::Player(id), kind) => {
            let Some(i) = super::person(w, id) else { return true };
            let pp = &w.people[i];
            if pp.stage == Stage::Done {
                return true;
            }
            match kind {
                PromiseKind::Start => pp.last_role == Some(PlayingTime::Starter) || seat_starter(w, i),
                PromiseKind::Snaps => matches!(pp.last_role, Some(PlayingTime::Starter | PlayingTime::Rotation)),
                PromiseKind::Place(r) => {
                    let here = Team::seat(w).is_some_and(|t| t.roster(w).contains(&i));
                    here && (pp.last_role.map_or(5, |x| x as u8) <= r || r == 0 && seat_starter(w, i))
                }
                PromiseKind::Contract => {
                    pp.pro.as_ref().and_then(|c| c.contract.as_ref()).is_some_and(|k| {
                        k.signed >= w.year && k.signed > p.made.year() - 1 && p.made.days_to(w.today) > 0
                    })
                }
                _ => true,
            }
        }
        (_, PromiseKind::Stay) => w.folk.mine.since.is_some_and(|s| s <= p.made),
        (_, PromiseKind::Booster(want)) => super::boss::want_met(w, want),
        (PromiseTo::Coach(c), PromiseKind::Raise) => w.folk.staff.raised_since(c, p.made),
        _ => true,
    }
}

/// A promise of his place has lapsed once he left your team on his own:
/// he graduated or moved up a level, transferred, declared or left
/// football. A club that cut or traded him, or a program or school that
/// dismissed him, broke it (`kept`).
pub(crate) fn lapsed(w: &World, p: &Promise) -> bool {
    let (PromiseTo::Player(id), PromiseKind::Place(_)) = (p.to, p.kind) else { return false };
    let Some(i) = super::person(w, id) else { return true };
    let Some(team) = Team::seat(w) else { return false };
    if team.roster(w).contains(&i) {
        return false;
    }
    let pp = &w.people[i];
    match team {
        Team::Nfl(_) | Team::Other(_) => pp.stage == Stage::Done,
        Team::College(c) => {
            matches!(pp.stage, Stage::Pro | Stage::Declared | Stage::Done | Stage::Away)
                || pp.college.is_some_and(|x| x != c)
        }
        Team::High(h) => pp.stage != Stage::HighSchool || pp.hs != h,
    }
}

/// A starter: last season's role or your depth chart now.
fn is_starter(w: &World, i: usize) -> bool {
    w.people[i].last_role == Some(PlayingTime::Starter) || seat_starter(w, i)
}

/// On your depth chart as a starter now (the game's own chart when you
/// set one).
fn seat_starter(w: &World, i: usize) -> bool {
    let id = w.people[i].player.id;
    w.seat_depth.iter().any(|(_, ids)| ids.first() == Some(&id))
}

// ---------------------------------------------------------------------------
// Dated mood reasons (topic 3): the parts the world adds to a mood.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Reason {
    Talk = 1,
    PromiseKept,
    PromiseBroken,
    FriendLeft,
    FriendsHere,
    Noise,
    Discipline,
    LifeEvent,
    Praise,
    Benched,
    TeamTalk,
    Room,
    /// The way the staff run the week (update 2, lane 4: a third morning of
    /// mat drills).
    Practice,
}

impl Reason {
    pub fn from_u8(v: u8) -> Reason {
        match v {
            2 => Reason::PromiseKept,
            3 => Reason::PromiseBroken,
            4 => Reason::FriendLeft,
            5 => Reason::FriendsHere,
            6 => Reason::Noise,
            7 => Reason::Discipline,
            8 => Reason::LifeEvent,
            9 => Reason::Praise,
            10 => Reason::Benched,
            11 => Reason::TeamTalk,
            12 => Reason::Room,
            13 => Reason::Practice,
            _ => Reason::Talk,
        }
    }
    pub fn words(self, up: bool) -> &'static str {
        match (self, up) {
            (Reason::Talk, true) => "a talk with you went well",
            (Reason::Talk, false) => "a talk with you went badly",
            (Reason::PromiseKept, _) => "you kept your word",
            (Reason::PromiseBroken, _) => "you broke a promise to him",
            (Reason::FriendLeft, _) => "a close friend left the team",
            (Reason::FriendsHere, _) => "close friends on the roster",
            (Reason::Noise, true) => "the praise in the media",
            (Reason::Noise, false) => "what the media and fans are saying",
            (Reason::Discipline, true) => "how you handled discipline",
            (Reason::Discipline, false) => "thinks he was treated unfairly",
            (Reason::LifeEvent, true) => "good news at home",
            (Reason::LifeEvent, false) => "something going on at home",
            (Reason::Praise, _) => "you praised him in public",
            (Reason::Benched, _) => "was benched",
            (Reason::TeamTalk, true) => "your team talk",
            (Reason::TeamTalk, false) => "didn't buy your team talk",
            (Reason::Room, true) => "the locker room",
            (Reason::Room, false) => "something that happened in the locker room",
            (Reason::Practice, true) => "the way the staff run the week",
            (Reason::Practice, false) => "the grind of the week: mat drills at dawn",
        }
    }
}

/// Adds a dated reason to his mood: it moves his mood now, and the next
/// mood read keeps what's left of it as it fades.
pub fn mood_event(w: &mut World, id: u32, why: Reason, delta: f32) {
    if delta.abs() < 0.005 {
        return;
    }
    let today = w.today;
    let v = (delta * 100.0).round().clamp(-120.0, 120.0) as i8;
    match w.folk.moods.iter_mut().find(|x| x.0 == id) {
        Some(x) => {
            x.1.push((today, why as u8, v));
            if x.1.len() > 12 {
                x.1.remove(0);
            }
        }
        None => w.folk.moods.push((id, vec![(today, why as u8, v)])),
    }
    if let Some(i) = super::person(w, id) {
        let p = &mut w.people[i];
        p.mood = (p.mood + delta).clamp(-1.0, 1.0);
    }
}

/// What is left of his dated reasons today: each fades by a quarter a month.
pub fn residual(w: &World, id: u32) -> f32 {
    let today = w.today;
    w.folk.moods.iter().find(|x| x.0 == id).map_or(0.0, |x| felt(&x.1, today))
}

/// What's left today of a person's dated reasons, summed (`residual`).
fn felt(notes: &[super::MoodNote], today: Date) -> f32 {
    notes.iter().map(|&(d, _, v)| v as f32 / 100.0 * 0.75f32.powf(d.days_to(today).max(0) as f32 / 30.0)).sum()
}

/// His dated reasons still felt, newest first, in words.
pub fn reasons(w: &World, id: u32) -> Vec<(Date, String, f32)> {
    let today = w.today;
    let mut v: Vec<(Date, String, f32)> = w.folk.moods.iter().find(|x| x.0 == id).map_or(Vec::new(), |x| {
        x.1.iter()
            .map(|&(d, r, v)| {
                let left = v as f32 / 100.0 * 0.75f32.powf(d.days_to(today).max(0) as f32 / 30.0);
                (d, Reason::from_u8(r).words(v > 0).to_string(), left)
            })
            .filter(|x| x.2.abs() >= 0.02)
            .collect()
    });
    v.reverse();
    v
}

/// After a mood read (`people.rs`), what's left of the dated reasons is
/// added back for the people that read covered (`read`, sorted person
/// indexes): the read blends 30% of the old mood, so 70% returns.
pub(crate) fn reapply(w: &mut World, read: &[usize]) {
    if w.folk.moods.is_empty() || read.is_empty() {
        return;
    }
    // Each entry's residual as `residual` reads it (the first entry with
    // its id), found through an index instead of a search per entry and
    // summed on all cores (world speed lane: the same sums).
    use rayon::prelude::*;
    let today = w.today;
    let moods = &w.folk.moods;
    let mut first: std::collections::HashMap<u32, usize> = std::collections::HashMap::with_capacity(moods.len());
    for (k, x) in moods.iter().enumerate() {
        first.entry(x.0).or_insert(k);
    }
    let ids: Vec<(u32, f32)> =
        moods.par_iter().with_min_len(256).map(|x| (x.0, felt(&moods[first[&x.0]].1, today))).collect();
    for (id, r) in ids {
        if r.abs() < 0.01 {
            continue;
        }
        if let Some(i) = super::person(w, id)
            && read.binary_search(&i).is_ok()
        {
            let p = &mut w.people[i];
            p.mood = (p.mood + 0.7 * r).clamp(-1.0, 1.0);
        }
    }
}

// ---------------------------------------------------------------------------
// Your time (topic 5: your time is a budget).

/// Hours of people time a week beyond film and the game plan (proposed,
/// not confirmed): ten in season, sixteen out of it.
pub fn week_budget(w: &World) -> f32 {
    let in_season = match Team::seat(w).map(|t| t.level()) {
        Some(Level::Nfl) => w.today >= w.dates.nfl_kickoff && w.today <= w.dates.super_bowl,
        _ => w.today >= w.dates.fall_camp && w.today <= w.dates.bowl_season.1,
    };
    if in_season { 10.0 } else { 16.0 }
}

fn week_start(d: Date) -> Date {
    let mut d = d;
    while d.weekday() != Weekday::Mon {
        d = d.add(-1);
    }
    d
}

/// Hours left this week (all of them, every week, with sandbox mode's
/// hours switch).
pub fn hours_left(w: &World) -> f32 {
    if w.sandbox(crate::world::sandbox::Switch::YourHours) {
        return week_budget(w);
    }
    let used = if w.folk.mine.week == Some(week_start(w.today)) { w.folk.mine.hours } else { 0.0 };
    (week_budget(w) - used).max(0.0)
}

/// Charges people time that has to be spent (a trip, a meeting you held),
/// past the week's budget if need be.
///
/// With sandbox mode's hours switch nothing is charged, so the week is
/// still true when the switch goes off.
pub fn charge(w: &mut World, hours: f32) {
    if w.sandbox(crate::world::sandbox::Switch::YourHours) {
        return;
    }
    let wk = week_start(w.today);
    if w.folk.mine.week != Some(wk) {
        w.folk.mine.week = Some(wk);
        w.folk.mine.hours = 0.0;
    }
    w.folk.mine.hours += hours;
}

/// Spends people time; false when the week is full. Always true, and
/// nothing charged, with sandbox mode's hours switch.
pub fn spend(w: &mut World, hours: f32) -> bool {
    if w.sandbox(crate::world::sandbox::Switch::YourHours) {
        return true;
    }
    let wk = week_start(w.today);
    if w.folk.mine.week != Some(wk) {
        w.folk.mine.week = Some(wk);
        w.folk.mine.hours = 0.0;
    }
    if w.folk.mine.hours + hours > week_budget(w) + 0.01 {
        return false;
    }
    w.folk.mine.hours += hours;
    true
}

// ---------------------------------------------------------------------------
// One-on-ones.

/// What is really on his mind (topic 3's hidden parts and topic 6's events):
/// the reason, and the words he'd use if he opened up.
pub fn concern(w: &World, i: usize) -> (Subject, String) {
    worry(w, i).unwrap_or_else(|| {
        if w.people[i].mood < -0.2 {
            (Subject::Role, "he isn't happy, and won't say why".into())
        } else {
            (Subject::Team, "nothing much: he's settled".into())
        }
    })
}

/// What is really on his mind, when something is: none when he's settled
/// or unhappy without a reason he'd name (`concern`'s two fallbacks), so a
/// tell never shows a worry he doesn't have.
fn worry(w: &World, i: usize) -> Option<(Subject, String)> {
    let p = &w.people[i];
    if let Some(e) = super::life::open_event(w, p.player.id) {
        return Some((Subject::Life, e));
    }
    if p.stage == Stage::College && p.cal.lean > 0.25 {
        return Some((Subject::Future, "he's been thinking about the portal".into()));
    }
    if let Some(f) = friend_gone(w, p.player.id).and_then(|f| super::person(w, f)) {
        let first = first_name(&w.people[f].name);
        return Some((Subject::Friend, format!("{first} leaving the team hit him harder than he lets on")));
    }
    Some(match Why::from_u8(p.mood_why) {
        Why::Role => (Subject::Role, "he thinks he should be playing more".into()),
        Why::Money => (Subject::Money, "he thinks he's underpaid".into()),
        Why::Losing => (Subject::Team, "the losing is eating at him".into()),
        Why::Coach => (Subject::Team, "he doesn't trust the staff yet".into()),
        Why::Life => (Subject::Life, "something at home".into()),
        Why::Room => (Subject::Team, "the locker room is wearing on him".into()),
        _ => return None,
    })
}

/// The close friend who left your team lately and still weighs on him: a
/// cut, trade, transfer or graduation in the last four weeks, from a close
/// tie (35+, as `ties::friends`; a family tie counts) whose leaving dipped
/// his mood (`ties::your_week`), and not back on the roster since. The
/// strongest such tie, by the friend's id.
pub fn friend_gone(w: &World, id: u32) -> Option<u32> {
    let today = w.today;
    let gone = &w.folk.ties.gone;
    if !gone.iter().any(|g| g.0 == id) {
        return None;
    }
    let roster = Team::seat(w).map(|t| t.roster(w)).unwrap_or_default();
    gone.iter()
        .filter(|g| g.0 == id && g.2 >= FRIEND_HIT && g.3.days_to(today) <= 28)
        .filter(|g| super::person(w, g.1).is_some_and(|j| !roster.contains(&j)))
        .max_by_key(|g| (g.2, g.3))
        .map(|g| g.1)
}

/// A close friend (`ties::friends`): his leaving dips the mood by 0.06 or
/// more (0.12 x strength / 70, `ties::your_week`).
const FRIEND_HIT: u8 = 35;

/// His closest friend still on your roster.
fn friend_here(w: &World, id: u32) -> Option<usize> {
    let roster = Team::seat(w).map(|t| t.roster(w)).unwrap_or_default();
    super::ties::friends(w, id).into_iter().filter_map(|f| super::person(w, f)).find(|j| roster.contains(j))
}

/// Who you'd ask to look out for him: his closest friend still here, else
/// a captain.
fn look_out(w: &World, i: usize) -> Option<usize> {
    let id = w.people[i].player.id;
    friend_here(w, id).or_else(|| {
        let team = Team::seat(w)?;
        super::takeover::captains_of(w, team).into_iter().find(|&j| j != i)
    })
}

fn first_name(full: &str) -> String {
    full.split_whitespace().next().unwrap_or(full).to_string()
}

/// The subjects you can raise with him today, his friend's leaving first
/// while it weighs on him: (key, name for a chip).
pub fn subjects_for(w: &World, id: u32) -> Vec<(&'static str, String)> {
    let mut v: Vec<(&'static str, String)> = Vec::new();
    if let Some(f) = friend_gone(w, id).and_then(|f| super::person(w, f)) {
        v.push((Subject::Friend.key(), format!("{} leaving", first_name(&w.people[f].name))));
    }
    for s in Subject::ALL {
        let n = s.name();
        let mut c = n.chars();
        let n = c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str());
        v.push((s.key(), n));
    }
    v
}

/// Opens a one-on-one you asked for from his page (`talk` command): it
/// costs an hour and opens now.
pub fn start_talk(w: &mut World, id: u32, subject: Subject) -> Result<u32, String> {
    let Some(i) = super::person(w, id) else { return Err("No such player.".into()) };
    let team = Team::seat(w).ok_or("You have no team.")?;
    if !team.roster(w).contains(&i) {
        return Err("He isn't on your roster.".into());
    }
    if let Some(s) = w.folk.scenes.iter().find(|s| !s.done && s.kind == Kind::OneOnOne && s.a == id as u64) {
        return Ok(s.id);
    }
    if !spend(w, 1.0) {
        return Err("Your week is full: no hours left for one-on-ones until Monday.".into());
    }
    let mut s = Scene::new(Kind::OneOnOne, w.today, format!("One-on-one with {}", w.people[i].name));
    s.a = id as u64;
    s.b = subject as u64;
    s.stops = false;
    s.place = match team {
        Team::High(_) => "The coaches' office, after school".into(),
        Team::Nfl(_) | Team::Other(_) => "Your office at the facility".into(),
        Team::College(_) => "Your office in the football building".into(),
    };
    Ok(scene::add(w, s))
}

fn player_facts(w: &World, i: usize) -> Facts {
    let p = &w.people[i];
    let mut f = Facts::new();
    f.set("name", p.name.split_whitespace().next().unwrap_or(&p.name).to_string());
    f.set("full", p.name.clone());
    f.set("coach", format!("Coach {}", scene::your_last(w)));
    f.set("you", scene::your_last(w));
    f.set("pos", p.player.position.name());
    if let Some(t) = Team::seat(w) {
        f.set("team", t.name(w));
    }
    // His friend is one still on the team; the one who left is `gone`.
    if let Some(fr) = friend_here(w, p.player.id) {
        f.set("friend", first_name(&w.people[fr].name));
    }
    if let Some(g) = friend_gone(w, p.player.id).and_then(|g| super::person(w, g)) {
        f.set("gone", first_name(&w.people[g].name));
    }
    f
}

fn one_on_one_open(w: &mut World, s: &mut Scene) {
    let id = s.a as u32;
    let Some(i) = super::person(w, id) else {
        s.narrate("He's gone: the talk never happened.");
        s.end();
        return;
    };
    let mut subject = Subject::from_u64(s.b);
    // His friend's leaving is a subject only while it weighs on him.
    if subject == Subject::Friend && friend_gone(w, id).is_none() {
        subject = Subject::Role;
        s.b = subject as u64;
    }
    s.who.push(Who::Player(id));
    let p = &w.people[i];
    let name = p.name.clone();
    let mood = p.mood;
    let trust = p.trust;
    // Where he came in, for the marks on the bars (bit 1 says they're set).
    s.f[2] = trust;
    s.f[3] = mood;
    s.c |= 2;
    let posture = if mood < -0.35 {
        "He sits down without taking his hood off and looks at the corner of your desk."
    } else if mood < -0.1 {
        "He sits on the edge of the chair, like he'd rather be somewhere else."
    } else if mood > 0.35 {
        "He walks in loose, says hello to the secretary by name and drops into the chair."
    } else {
        "He sits down and waits for you to start."
    };
    let asked = asked_to_leave(w, id);
    if asked {
        s.narrate("He asked for this meeting.");
    }
    s.narrate(format!("In comes {}. {posture}", player_link(w, i)));
    let spk = Speaker::player(w, i);
    let mut f = player_facts(w, i);
    f.set("subject", subject.name());
    if let Some(r) = crate::world::recruiting::intent_reason(w, id) {
        f.set("reason", r);
    }
    let mut pen = scene::pen(w, s, 1);
    let tone = if mood < -0.2 {
        "sour"
    } else if mood > 0.2 {
        "good"
    } else {
        "flat"
    };
    // He asked for this meeting to say he means to leave (minor bug 155): his
    // own opening in his mood says why he came, never a mood part that
    // assumes you called him in.
    let keys = if asked {
        vec![format!("talk.open.portal.{tone}")]
    } else if subject == Subject::Friend && tone == "good" {
        vec!["talk.open.friend".to_string()]
    } else {
        vec![format!("talk.open.{tone}"), format!("talk.open.{}", subject.key())]
    };
    let line = pen.line(&keys.iter().map(String::as_str).collect::<Vec<_>>(), &spk, &f);
    pen.put(w);
    let line = match (line.is_empty(), asked) {
        (false, _) => line,
        (true, true) => {
            "Coach, I wanted you to hear it from me first. I'm thinking about putting my name in the portal.".into()
        }
        (true, false) => open_fallback(subject, mood),
    };
    let tell = if worry(w, i).is_some_and(|x| x.0 == subject) {
        subject_tell(w, i, true)
    } else {
        read_tell(w, i, if trust < 0.4 { -1 } else { 0 })
    };
    s.them(&name, line, tell);
    add_talk_picks(w, s, i, subject, &[]);
}

/// You turn a one-on-one to another subject (dialogue D1, the subject
/// chips): he answers on that one now, and the talk goes on. It costs no
/// exchange.
pub fn change_subject(w: &mut World, id: u32, subject: Subject) -> Result<(), String> {
    let k = w.folk.scenes.iter().position(|s| s.id == id).ok_or("No such talk.")?;
    let mut s = w.folk.scenes[k].clone();
    if s.done || s.kind != Kind::OneOnOne || !s.opened {
        return Err("That talk isn't going on.".into());
    }
    if Subject::from_u64(s.b) == subject {
        return Ok(());
    }
    let Some(i) = super::person(w, s.a as u32) else { return Err("He's gone.".into()) };
    let gone = friend_gone(w, s.a as u32).and_then(|g| super::person(w, g)).map(|g| first_name(&w.people[g].name));
    if subject == Subject::Friend && gone.is_none() {
        return Err("That's behind him now.".into());
    }
    s.b = subject as u64;
    s.c &= !1;
    let about = match (&gone, subject) {
        (Some(g), Subject::Friend) => format!("{g} leaving"),
        _ => subject.name().replace("his ", "your "),
    };
    s.you(&scene::your_name(w), format!("Let's talk about {about}."));
    let spk = Speaker::player(w, i);
    let mut f = player_facts(w, i);
    f.set("subject", subject.name());
    let mood = w.people[i].mood;
    let mut pen = scene::pen(w, &s, 7);
    let line = pen.line(&[&format!("talk.open.{}", subject.key())], &spk, &f);
    pen.put(w);
    let line = if line.is_empty() { open_fallback(subject, mood) } else { line };
    let name = w.people[i].name.clone();
    // Turning to what's really on his mind shows; turning away from it, a
    // sharp read sees his mind is elsewhere.
    let tell = match worry(w, i) {
        Some((real, _)) => subject_tell(w, i, real == subject),
        None => read_tell(w, i, 0),
    };
    s.them(&name, line, tell);
    let used: Vec<Move> =
        Move::ALL.iter().enumerate().filter(|(k, _)| s.c & (1 << (2 + k)) != 0).map(|x| *x.1).collect();
    s.picks.clear();
    add_talk_picks(w, &mut s, i, subject, &used);
    w.folk.scenes[k] = s;
    Ok(())
}

fn open_fallback(subject: Subject, mood: f32) -> String {
    match (subject, mood < -0.2) {
        (Subject::Role, true) => {
            "You wanted to see me, Coach? If it's about the depth chart, I've got some things to say.".into()
        }
        (Subject::Role, false) => "You wanted to see me, Coach?".into(),
        (Subject::Future, _) => "I figured this was coming. You want to know if I'm staying.".into(),
        (Subject::Money, _) => "My people have been asking me questions I can't answer.".into(),
        (Subject::Life, _) => "Everything all right, Coach?".into(),
        (Subject::Behavior, _) => "I know why I'm here.".into(),
        (Subject::Friend, _) => "Is this about him leaving? I'm alright, Coach.".into(),
        _ => "What's up, Coach?".into(),
    }
}

/// The moves on the table for a subject (topic 5), each with the tones that
/// make sense; moves already used drop off.
fn add_talk_picks(w: &World, s: &mut Scene, i: usize, subject: Subject, used: &[Move]) {
    let p = &w.people[i];
    let starter = is_starter(w, i);
    let nfl = p.stage == Stage::Pro;
    let gone = friend_gone(w, p.player.id)
        .and_then(|g| super::person(w, g))
        .map_or_else(|| "your boy".to_string(), |g| first_name(&w.people[g].name));
    let mate = look_out(w, i).map(|j| first_name(&w.people[j].name));
    // Every worry can take a promise (Cain 2026-11-03): his place doesn't
    // change, logged and checked at the season's end.
    let place = if starter {
        "Logged: he's still your starter at the season's end, or it's broken."
    } else {
        "Logged: he's still on the team in at least his role at the season's end."
    };
    // One pick per move and tone: the subject's own line wins over the
    // general one (two picks on one code read as the same chip twice).
    let mut add = |m: Move, t: Tone, label: String, hint: &str| {
        if !used.contains(&m) && !s.picks.iter().any(|p| p.code == code_of(m, t)) {
            s.picks.push(Pick {
                label,
                mv: m.name().into(),
                tone: t.name().into(),
                hint: hint.into(),
                code: code_of(m, t),
            });
        }
    };
    if subject == Subject::Friend {
        add(
            Move::Ask,
            Tone::Warm,
            format!("\"I know {gone} leaving hit you. How are you holding up?\""),
            "Asking about what you both know is on his mind: he opens up more easily.",
        );
    }
    add(
        Move::Ask,
        Tone::Warm,
        "\"What's really going on with you?\"".into(),
        "He opens up if he trusts you; that's how you learn what you couldn't see.",
    );
    match subject {
        Subject::Role | Subject::Usage => {
            add(
                Move::Straight,
                Tone::Calm,
                if starter {
                    "\"You're my starter. Here's what I need more of.\"".into()
                } else {
                    "\"Here's where you are on the chart, and exactly why.\"".into()
                },
                "Honesty costs mood now and builds trust.",
            );
            if !starter {
                add(
                    Move::Promise,
                    Tone::Warm,
                    "\"Keep working like this and the job is yours by the opener.\"".into(),
                    "Lifts him now. Logged: break it and he, his friends and the room remember.",
                );
            } else {
                add(
                    Move::Promise,
                    Tone::Warm,
                    "\"We're going to get you the ball more.\"".into(),
                    "Lifts him now. Logged and checked at the season's end.",
                );
            }
            add(
                Move::Challenge,
                Tone::Firm,
                "\"Then take it. Make me play you.\"".into(),
                "A competitor rises to it; a fragile player sinks.",
            );
        }
        Subject::Money => {
            if nfl {
                add(
                    Move::Promise,
                    Tone::Calm,
                    "\"I'll get the general manager to sit down with your agent.\"".into(),
                    "A contract talk, logged; it's kept if a new deal follows.",
                );
            } else {
                add(
                    Move::Promise,
                    Tone::Calm,
                    "\"I'll talk to the collective about you.\"".into(),
                    "An NIL push, logged.",
                );
            }
            add(
                Move::Straight,
                Tone::Calm,
                "\"Play like it and the money follows. That's how this works.\"".into(),
                "Honest; a greedy player hears a no.",
            );
        }
        Subject::Behavior => {
            add(
                Move::Warn,
                Tone::Firm,
                "\"Next time it's not a talk. You'll sit.\"".into(),
                "Lowers his trouble risk for a while; a hothead bristles.",
            );
            add(
                Move::Criticize,
                Tone::Fiery,
                "\"That's selfish, and the whole room saw it.\"".into(),
                "Lands as an insult on a big ego; a professional takes it.",
            );
            add(
                Move::Promise,
                Tone::Calm,
                if starter {
                    "\"Handle this the right way and it stays behind us. You're still my starter.\"".into()
                } else {
                    "\"Handle this the right way and it stays behind us. Nothing changes for you.\"".into()
                },
                place,
            );
        }
        Subject::Life => {
            add(
                Move::Listen,
                Tone::Calm,
                "Say nothing. Let him talk.".into(),
                "Helps a player who needs to be heard; frustrates one who wants answers.",
            );
            add(
                Move::Hand,
                Tone::Warm,
                "\"Take the time you need. Your family comes first.\"".into(),
                "Time off from practice; the room sees how you treat people.",
            );
            add(
                Move::Promise,
                Tone::Calm,
                if starter {
                    "\"Take care of home. Your job is here when you're back.\"".into()
                } else {
                    "\"Take care of home. Your spot is here when you're back.\"".into()
                },
                place,
            );
        }
        Subject::Team => {
            add(
                Move::Hand,
                Tone::Warm,
                "\"I want you on the leadership council.\"".into(),
                "Leaders grow into it; others take it as a compliment.",
            );
            add(
                Move::Praise,
                Tone::Warm,
                "\"You're the kind of player this team needs more of.\"".into(),
                "Empty praise is spotted; earned praise lands.",
            );
            add(
                Move::Promise,
                Tone::Calm,
                "\"Whatever's going on in that room, your place here is safe with me.\"".into(),
                place,
            );
        }
        Subject::Friend => {
            add(
                Move::Straight,
                Tone::Calm,
                if starter {
                    format!("\"I know you'll miss {gone}. But you're my starter, and this is your room now.\"")
                } else {
                    format!("\"I know you'll miss {gone}. Your road here hasn't changed, and it's up to you.\"")
                },
                if starter {
                    "Reminds him what he has here: a starter hears respect."
                } else {
                    "Honest; a player who needs to grieve may hear that you don't care."
                },
            );
            add(
                Move::Promise,
                Tone::Warm,
                if starter {
                    format!(
                        "\"{gone} leaving changes nothing for you. You're my starter, and that's not going anywhere.\""
                    )
                } else {
                    format!("\"{gone} leaving changes nothing for you. Your place here is safe.\"")
                },
                place,
            );
            if let Some(m) = &mate {
                add(
                    Move::Hand,
                    Tone::Warm,
                    format!("\"I've asked {m} to look out for you.\""),
                    "Pairs him with a teammate: their friendship grows, and he has someone in the room.",
                );
            }
            add(
                Move::Listen,
                Tone::Calm,
                format!("Let him talk about {gone}."),
                "Helps a player who needs to be heard; frustrates one who wants answers.",
            );
        }
        Subject::Future => {
            add(
                Move::Straight,
                Tone::Calm,
                "\"Here's what next year looks like for you here, honestly.\"".into(),
                "Trust now; a player buried on the chart may still go.",
            );
            add(
                Move::Promise,
                Tone::Warm,
                if nfl {
                    "\"You're part of what we're building. I want you here long term.\"".into()
                } else {
                    "\"Stay, and you'll start next year.\"".into()
                },
                "Keeps him more often. Logged: a start next season.",
            );
            add(
                Move::Promise,
                Tone::Calm,
                "\"I'm not going anywhere. I'll be here.\"".into(),
                "Logged: you stay a year.",
            );
            add(
                Move::Challenge,
                Tone::Fiery,
                "\"If you want to run from competition, that's your call.\"".into(),
                "A competitor stays to prove you wrong; others take the door.",
            );
        }
    }
    add(
        Move::Praise,
        Tone::Warm,
        "\"I've noticed the work you're putting in.\"".into(),
        "Empty praise is spotted; earned praise lands.",
    );
    add(
        Move::Listen,
        Tone::Calm,
        "Hear him out. Promise nothing.".into(),
        "Helps one who needs to be heard; frustrates one who wants answers.",
    );
    s.pick(900, "End the talk", "End", "calm", "Ends it here.");
}

/// How a move lands on him (topic 5): (trust, mood). Fit with his
/// personality, his concern and the tone; noise for the day he's having.
fn landing(w: &World, i: usize, m: Move, t: Tone, subject: Subject, revealed: bool, seed: u64) -> (f32, f32) {
    let p = &w.people[i];
    let tr = &p.player.personality.traits;
    let z = |x: Trait| tr.z(x) as f32;
    let comp = ((p.player.personality.competitiveness - 10.0) / 10.0) as f32;
    let shaky = ((10.0 - p.bg.stability) / 10.0) as f32;
    let (real, _) = concern(w, i);
    let on_point = real == subject || revealed;
    let earned = p.last_role == Some(PlayingTime::Starter) || p.last_performance.unwrap_or(0.0) > 62.0;
    let needs_ear = p.mood < -0.15 || shaky > 0.2 || matches!(real, Subject::Life | Subject::Friend);
    let (mut dt, mut dm): (f32, f32) = match m {
        Move::Ask => (0.04 + 0.04 * needs_ear as u8 as f32, 0.03 + 0.04 * needs_ear as u8 as f32),
        // A friend gone, told straight what he still has here: a starter
        // hears respect (Cain 2026-11-03: "I have you as my WR1"); a backup
        // who needs to grieve hears that you don't care.
        Move::Straight if subject == Subject::Friend => {
            if is_starter(w, i) {
                (0.07 + 0.03 * z(Trait::Professionalism), 0.05 + 0.03 * comp.max(0.0))
            } else {
                (0.05, -0.03 - 0.03 * needs_ear as u8 as f32)
            }
        }
        Move::Straight => (
            0.07 + 0.03 * z(Trait::Professionalism),
            -0.04 - 0.05 * z(Trait::Ego).max(0.0) + 0.03 * z(Trait::Professionalism),
        ),
        Move::Praise => {
            if earned {
                (0.04, 0.09 + 0.04 * z(Trait::Ego).max(0.0))
            } else {
                (-0.03, 0.03)
            }
        }
        Move::Challenge => (0.02 + 0.06 * comp, 0.08 * comp - 0.1 * shaky.max(0.0)),
        Move::Criticize => (
            -0.03 - 0.12 * z(Trait::Ego).max(0.0) + 0.04 * z(Trait::Professionalism),
            -0.08 - 0.06 * z(Trait::Ego).max(0.0),
        ),
        Move::Promise => (0.05, 0.14),
        Move::Warn => (-0.02 - 0.05 * z(Trait::Temper).max(0.0), -0.04),
        Move::Hand => (0.06 + 0.06 * z(Trait::Leadership).max(0.0), 0.08),
        Move::Listen => {
            if needs_ear {
                (0.07, 0.07)
            } else {
                (0.0, -0.03 - 0.03 * comp.max(0.0))
            }
        }
    };
    // The subject: on his real worry a move counts more; off it, less.
    let k = if on_point { 1.3 } else { 0.7 };
    dt *= k;
    dm *= k;
    // The tone (topic 5): a fiery word lifts a competitor and sinks a
    // fragile one; warmth helps the shaky; firmness suits professionals.
    let (tt, tm) = match t {
        Tone::Fiery => (0.02 * comp - 0.04 * z(Trait::Temper).max(0.0), 0.05 * comp - 0.07 * shaky.max(0.0)),
        Tone::Warm => {
            (0.02 + 0.02 * shaky.max(0.0) - 0.02 * z(Trait::Ego).max(0.0) * 0.5, 0.02 + 0.03 * shaky.max(0.0))
        }
        Tone::Firm => (0.02 * z(Trait::Professionalism), 0.0),
        Tone::Calm => (0.01 + 0.02 * z(Trait::Temper).max(0.0), 0.0),
    };
    // His trust in you colours everything: a player who doesn't trust you
    // hears less of it (topic 3).
    let ear = 0.6 + 0.8 * p.trust;
    let noise = (super::roll_n(w, seed, p.player.id as u64) as f32) * 0.025;
    ((dt + tt) * ear + noise, (dm + tm) * ear + noise)
}

/// What your staff expects a move to do before you say it (screens topic
/// 8: a move takes two steps). Their read of him is only as good as the
/// position coach's feel for people and how long you've had him: a weak
/// staff can be confidently wrong. Returns (what they expect, how sure, who
/// says it), or None for a scene that isn't a one-on-one.
pub fn expect(w: &World, s: &Scene, code: u32) -> Option<(String, String, String)> {
    if s.kind != Kind::OneOnOne || s.done {
        return None;
    }
    let (m, t) = decode(code)?;
    let id = s.a as u32;
    let i = super::person(w, id)?;
    let subject = Subject::from_u64(s.b);
    let revealed = s.c & 1 == 1;
    let (dt, dm) = landing(w, i, m, t, subject, revealed, s.id as u64 ^ (s.step as u64) << 8);
    let rel = position_coach_rel(w, i) as f32;
    let you = w.career.as_ref().map_or(10.0, |c| c.you.relationships) as f32;
    let known = w.folk.mine.since.map_or(0, |d| d.days_to(w.today)) as f32 / 180.0;
    let q = (0.45 * rel / 20.0 + 0.35 * you / 20.0 + 0.2 * known.min(1.0)).clamp(0.1, 1.0);
    // Their error: steady for this move in this talk, bigger for a weak staff.
    let err = super::roll_n(w, 0xE7EC ^ s.id as u64 ^ (code as u64) << 16, id as u64) as f32 * 0.09 * (1.0 - q);
    let seen = dt + dm + err;
    let what = match seen {
        x if x > 0.14 => "He'll take it well. It should help with him.",
        x if x > 0.05 => "It should land. Not a big swing either way.",
        x if x > -0.03 => "Hard to say. It may not move him at all.",
        x if x > -0.1 => "Careful. He may not like hearing it.",
        _ => "It'll hurt. Expect him to take it badly.",
    };
    let risk = match m {
        Move::Promise => " It's a promise: it goes in the ledger and he'll hold you to it.",
        Move::Criticize if dt + dm < 0.0 => " Criticism in private still travels in a locker room.",
        Move::Warn => " If he breaks the rule again, you'll have to follow through.",
        _ => "",
    };
    let sure = match q {
        x if x >= 0.75 => "They're sure",
        x if x >= 0.55 => "Fairly sure",
        x if x >= 0.4 => "A guess",
        _ => "A guess, and they don't know him well",
    };
    let who = w.people[i].college_staff.last().and_then(|x| x.3).filter(|&c| c < w.coaches.len()).map_or_else(
        || "Your staff".to_string(),
        |c| format!("{} ({})", super::coach_link(w, c), super::facts::coach_role(w, c)),
    );
    Some((format!("{what}{risk}"), sure.to_string(), who))
}

/// Whether a code is a talk move with another tone than the one offered
/// (you may say any listed move in any of the four tones).
pub(crate) fn same_move(a: u32, b: u32) -> bool {
    (100..190).contains(&a) && (100..190).contains(&b) && (a - 100) / 10 == (b - 100) / 10
}

/// Your read of a reaction: what you catch depends on your relationships
/// rating and how long you've known him (topic 5's tells).
fn read_tell(w: &World, i: usize, sign: i32) -> Option<String> {
    let p = &w.people[i];
    let sharp = sharpness(w);
    let tr = &p.player.personality.traits;
    let s = match sign {
        x if x > 0 => {
            if tr.get(Trait::Ego) >= 14.0 {
                "He nods like he already knew it."
            } else if tr.get(Trait::Leadership) >= 14.0 {
                "He looks you in the eye and holds it."
            } else {
                "His shoulders drop a little. He's listening now."
            }
        }
        x if x < 0 => {
            if tr.get(Trait::Temper) >= 14.0 {
                "His jaw works. He's holding something back."
            } else if p.pro.is_some() {
                "He mentions his agent, almost in passing."
            } else if p.family.parent_style == crate::world::background::ParentStyle::LoudAtGames {
                "He says his dad will want to hear about this."
            } else {
                "Short answers. He's looking at the door."
            }
        }
        _ => "He gives you nothing either way.",
    };
    if sharp < 0.45 && sign != 0 {
        // A weak read sees a sign, not the meaning.
        return Some("He shifts in his chair.".into());
    }
    Some(s.into())
}

/// How sharp your read of people is: your relationships rating and how
/// long you've known them (0 to about 1.2; under 0.45 is a weak read).
fn sharpness(w: &World) -> f64 {
    let you = w.career.as_ref().map_or(10.0, |c| c.you.relationships);
    let known = w.folk.mine.since.map_or(0, |d| d.days_to(w.today)) as f64 / 120.0;
    you / 20.0 + 0.2 * known.min(1.0)
}

/// What your read catches when the talk turns to a subject: on what's
/// really on his mind he engages; off it, a sharp read sees his mind is
/// somewhere else and a weak one sees nothing.
fn subject_tell(w: &World, _i: usize, on_point: bool) -> Option<String> {
    let sharp = sharpness(w) >= 0.45;
    Some(
        match (on_point, sharp) {
            (true, true) => "He sits up. This is what's on his mind.",
            (true, false) => "He sits up a little.",
            (false, true) => "He answers, but his mind is somewhere else.",
            (false, false) => "He gives you nothing either way.",
        }
        .into(),
    )
}

/// How a line went over, in words, as far as your read catches it (new
/// bugs, Cain 2026-11-03): a sharp read sees his trust and his mood move
/// apart; a weak one only whether it landed.
fn went_words(w: &World, dt: f32, dm: f32) -> String {
    if sharpness(w) < 0.45 {
        return match dt + dm {
            x if x > 0.06 => "It seemed to land.".into(),
            x if x < -0.03 => "It didn't seem to land.".into(),
            _ => "Hard to tell how that landed.".into(),
        };
    }
    let trust = match dt {
        x if x > 0.06 => "He trusts you more",
        x if x > 0.015 => "A little more trust",
        x if x < -0.05 => "He trusts you less",
        x if x < -0.015 => "A little less trust",
        _ => "His trust holds",
    };
    let mood = match dm {
        x if x > 0.09 => "his mood lifts",
        x if x > 0.025 => "his mood lifts a little",
        x if x < -0.07 => "his mood sinks",
        x if x < -0.02 => "his mood dips",
        _ => "his mood doesn't move",
    };
    format!("{trust}; {mood}.")
}

fn one_on_one_answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    let id = s.a as u32;
    let Some(i) = super::person(w, id) else {
        s.end();
        return;
    };
    let name = w.people[i].name.clone();
    if code == 900 {
        finish_talk(w, s, id);
        return;
    }
    let Some((m, t)) = decode(code) else {
        s.end();
        return;
    };
    let subject = Subject::from_u64(s.b);
    s.you(&scene::your_name(w), picked.label.clone());
    let revealed = s.c & 1 == 1;
    let (dt, dm) = landing(w, i, m, t, subject, revealed, s.id as u64 ^ (s.step as u64) << 8);
    s.f[0] += dt;
    s.f[1] += dm;
    let p = &mut w.people[i];
    p.trust = (p.trust + dt).clamp(0.0, 1.0);
    let spk = Speaker::player(w, i);
    let mut f = player_facts(w, i);
    let outcome = if dt + dm > 0.06 {
        "good"
    } else if dt + dm < -0.03 {
        "bad"
    } else {
        "flat"
    };
    let mut pen = scene::pen(w, s, 2 + s.step as u64);
    // Ask: whether he opens up (topic 5): his trust and personality.
    if m == Move::Ask && !revealed {
        let p = &w.people[i];
        // Asked about what you both know (his friend leaving), he opens up
        // more easily.
        let open = 0.15 + 0.7 * p.trust + 0.1 * p.player.personality.traits.z(Trait::Adaptability) as f32
            - 0.1 * p.player.personality.traits.z(Trait::Ego).max(0.0) as f32
            + 0.15 * (subject == Subject::Friend) as u8 as f32;
        let r = super::roll(w, 0xA5C ^ s.id as u64, id as u64) as f32;
        let (real, words) = concern(w, i);
        if r < open {
            s.c |= 1;
            f.set("worry", words.clone());
            let line = pen.line(&[&format!("talk.reveal.{}", real.key())], &spk, &f);
            s.them(&name, if line.is_empty() { reveal_fallback(real, &words) } else { line }, read_tell(w, i, 1));
            s.outcome.push(format!("{} opened up: {words}.", name));
            super::note(w, "talk", player_link(w, i), format!("Told you {words}."), None);
            s.b = real as u64;
        } else {
            let line = pen.line(&["talk.deflect"], &spk, &f);
            s.them(&name, if line.is_empty() { "I'm good, Coach. Really.".into() } else { line }, read_tell(w, i, -1));
        }
    } else {
        // On his friend leaving he answers about what you told him (a
        // promise still gets its own "I'll hold you to it", listening and
        // asking their own); asked to lean on a teammate, he answers about
        // that.
        let line = if subject == Subject::Friend && !matches!(m, Move::Promise | Move::Listen | Move::Ask) {
            let own = if m == Move::Hand { format!("talk.reply.friend.hand.{outcome}") } else { String::new() };
            let line = if own.is_empty() { String::new() } else { pen.line(&[&own], &spk, &f) };
            if line.is_empty() {
                pen.line(&[&format!("talk.reply.friend.{outcome}"), &format!("?talk.close.{outcome}")], &spk, &f)
            } else {
                line
            }
        } else {
            String::new()
        };
        let line = if line.is_empty() {
            pen.line(&[&format!("talk.reply.{}.{outcome}", m.key()), &format!("?talk.close.{outcome}")], &spk, &f)
        } else {
            line
        };
        let line = if line.is_empty() { reply_fallback(m, outcome) } else { line };
        let sign = match outcome {
            "good" => 1,
            "bad" => -1,
            _ => 0,
        };
        s.them(&name, line, read_tell(w, i, sign));
    }
    pen.put(w);
    // How it went over, under what he said (as far as your read catches).
    if let Some(l) = s.lines.last_mut().filter(|l| l.by == 2) {
        l.went = Some(went_words(w, dt, dm));
    }
    // What the move does beyond the talk.
    match m {
        Move::Promise => {
            let starter = is_starter(w, i);
            let rank = if starter { 0 } else { w.people[i].last_role.map_or(PlayingTime::Backup as u8, |r| r as u8) };
            let kind = match subject {
                Subject::Money => PromiseKind::Contract,
                Subject::Future if picked.label.contains("not going anywhere") => PromiseKind::Stay,
                Subject::Future => PromiseKind::Start,
                Subject::Behavior | Subject::Life | Subject::Team | Subject::Friend => PromiseKind::Place(rank),
                _ if starter => PromiseKind::Snaps,
                _ => PromiseKind::Start,
            };
            promise(w, PromiseTo::Player(id), kind, 0);
            s.outcome.push(format!("You promised {} {}.", name, kind.words()));
        }
        Move::Warn => {
            w.folk.rules.warned.push((id, w.today));
            w.folk.rules.warned.retain(|x| x.1.days_to(w.today) < 120);
        }
        Move::Hand if subject == Subject::Team => {
            if !w.folk.rules.council.contains(&id) {
                w.folk.rules.council.push(id);
            }
            s.outcome.push(format!("{name} joins your leadership council."));
        }
        Move::Hand if subject == Subject::Friend => {
            if let Some(j) = look_out(w, i) {
                let (a, b) = (w.people[j].player.id, id);
                super::ties::bond(w, a, b, 10);
                s.outcome.push(format!("You asked {} to look out for {name}.", player_link(w, j)));
            }
        }
        Move::Hand if subject == Subject::Life => {
            s.outcome.push(format!("{name} gets time away; his position coach covers his work."));
            super::life::grant_leave(w, id);
        }
        _ => {}
    }
    // Leaning to leave: what you said reaches the portal lean (T3).
    if w.people[i].stage == Stage::College && w.people[i].cal.lean > 0.0 && (dt + dm).abs() > 0.03 {
        let f = if dt + dm > 0.0 {
            (1.0 - 1.2 * (dt + dm) as f64).max(0.7)
        } else {
            (1.0 - 1.2 * (dt + dm) as f64).min(1.3)
        };
        let today = w.today;
        let p = &mut w.people[i];
        p.cal.lean *= f;
        p.cal.lean_events.push((
            today,
            if f < 1.0 { "a talk with his head coach went well" } else { "a talk with his head coach went badly" },
            f,
        ));
    }
    s.c |= 1 << (2 + Move::ALL.iter().position(|x| *x == m).unwrap_or(0));
    let all_used: Vec<Move> =
        Move::ALL.iter().enumerate().filter(|(k, _)| s.c & (1 << (2 + k)) != 0).map(|x| *x.1).collect();
    if s.step >= 3 || matches!(m, Move::Criticize) && outcome == "bad" {
        finish_talk(w, s, id);
    } else {
        add_talk_picks(w, s, i, Subject::from_u64(s.b), &all_used);
    }
}

fn reveal_fallback(s: Subject, words: &str) -> String {
    match s {
        Subject::Life => format!("It's not football, Coach. It's home. {}", cap(words)),
        Subject::Future => "Honestly? I don't know if this is the place for me anymore.".into(),
        Subject::Role => "I just feel like I've earned a shot and I'm not getting it.".into(),
        Subject::Money => "Everybody around me is getting paid. I'm doing the work.".into(),
        Subject::Friend => "Honestly? I miss him. It's been harder than I let on.".into(),
        _ => format!("Truth is, {words}."),
    }
}

fn reply_fallback(m: Move, outcome: &str) -> String {
    match (m, outcome) {
        (Move::Promise, _) => "I'm going to hold you to that, Coach.".into(),
        (Move::Straight, "good") => "I appreciate you being straight with me.".into(),
        (Move::Straight, _) => "So that's it, then.".into(),
        (Move::Challenge, "good") => "Bet. Watch me.".into(),
        (Move::Challenge, _) => "Yeah. Okay.".into(),
        (Move::Criticize, "bad") => "With respect, Coach, you don't know what I'm dealing with.".into(),
        (Move::Criticize, _) => "You're right. It won't happen again.".into(),
        (Move::Praise, "good") => "Means a lot coming from you.".into(),
        (Move::Praise, _) => "Thanks.".into(),
        (Move::Warn, "bad") => "Understood.".into(),
        (Move::Warn, _) => "I hear you, Coach.".into(),
        (Move::Hand, _) => "I won't let you down.".into(),
        (Move::Listen, "good") => "Thanks for hearing me out. Nobody's asked.".into(),
        (Move::Listen, _) => "So... what are you going to do about it?".into(),
        _ => "Okay.".into(),
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str() + ".",
        None => String::new(),
    }
}

fn finish_talk(w: &mut World, s: &mut Scene, id: u32) {
    let (dt, dm) = (s.f[0], s.f[1]);
    mood_event(w, id, Reason::Talk, dm);
    let name = super::person(w, id).map_or_else(String::new, |i| w.people[i].name.clone());
    s.outcome.push(format!(
        "{name} left {}.",
        if dt + dm > 0.12 {
            "in a better place than he came in"
        } else if dt + dm > 0.03 {
            "a little better"
        } else if dt + dm < -0.08 {
            "worse than he came in"
        } else {
            "about where he started"
        }
    ));
    s.end();
}

// ---------------------------------------------------------------------------
// Team talks (topic 5: talking to a group).

/// Opens a team talk you called (`team_talk` command): once a week.
pub fn start_team_talk(w: &mut World) -> Result<u32, String> {
    let team = Team::seat(w).ok_or("You have no team.")?;
    if let Some(s) = w.folk.scenes.iter().find(|s| !s.done && s.kind == Kind::TeamTalk) {
        return Ok(s.id);
    }
    let wk = week_start(w.today);
    if w.folk
        .scenes
        .iter()
        .any(|s| s.kind == Kind::TeamTalk && s.done && s.handed.is_none() && week_start(s.date) == wk)
    {
        return Err("You already talked to the team this week. Say it too often and they stop hearing it.".into());
    }
    if !spend(w, 0.5) {
        return Err("Your week is full.".into());
    }
    let mut s = Scene::new(Kind::TeamTalk, w.today, "Talking to the team");
    s.stops = false;
    s.team = Some(team);
    s.place = match team {
        Team::High(_) => "The field house".into(),
        Team::Nfl(_) => "The team auditorium".into(),
        Team::Other(_) => "The team room".into(),
        Team::College(_) => "The team room".into(),
    };
    Ok(scene::add(w, s))
}

fn team_talk_open(w: &mut World, s: &mut Scene) {
    let Some(team) = s.team.or(Team::seat(w)) else {
        s.end();
        return;
    };
    let last = super::facts::last_game(w, team);
    let mood = team_mood(w, team);
    s.who.push(Who::Group("The team".into()));
    s.narrate(match last {
        Some((won, us, them, opp)) if won => {
            format!("Monday after the {us}-{them} win over {}. The room is loose; somebody has music on.", opp.link(w))
        }
        Some((_, us, them, opp)) => {
            format!("Monday after the {us}-{them} loss to {}. Nobody is talking much.", opp.link(w))
        }
        None => "No game to talk about yet. Everyone is waiting to hear what kind of week this is.".into(),
    });
    s.narrate(format!("The room's mood: {}.", mood_words(mood)));
    add_team_picks(s);
}

fn add_team_picks(s: &mut Scene) {
    s.pick(
        200,
        "\"One play at a time. Do your job.\" (focus)",
        "Focus",
        "calm",
        "Steady and safe: small lift, little risk.",
    );
    s.pick(
        201,
        "\"Breathe. We're fine. Nothing changes.\" (calm)",
        "Reassure",
        "warm",
        "Settles a shaky room; a competitive room wants more.",
    );
    s.pick(
        202,
        "\"I want somebody to hit somebody this week.\" (fire)",
        "Fire",
        "fiery",
        "Lifts competitors; wears thin if you use it every week.",
    );
    s.pick(
        203,
        "\"That wasn't good enough, and you know it.\" (challenge)",
        "Challenge",
        "firm",
        "Hits the proud players where it should; the fragile sink.",
    );
    s.pick(
        204,
        "\"I believe in this group more than ever.\" (praise)",
        "Praise",
        "warm",
        "Lands after a win; rings hollow after a bad loss.",
    );
}

fn team_talk_answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    let Some(team) = s.team.or(Team::seat(w)) else {
        s.end();
        return;
    };
    s.you(&scene::your_name(w), picked.label.clone());
    let (msg, tone) = match code {
        200 => (Message::Plain, Tone::Calm),
        201 => (Message::Honour, Tone::Warm),
        202 => (Message::WinNow, Tone::Fiery),
        203 => (Message::Compete, Tone::Firm),
        _ => (Message::Honour, Tone::Warm),
    };
    // A tone used too often wears out (topic 5).
    let t = tone_index(tone) as u8;
    let worn = w.folk.mine_tones.iter().rev().take(4).filter(|&&x| x == t).count();
    let won = super::facts::last_game(w, team).map(|x| x.0);
    let mut weight = 0.5 * (1.0 - 0.22 * worn as f32);
    if code == 204 && won == Some(false) {
        weight *= -0.4;
    }
    if code == 203 && won == Some(true) {
        weight *= 0.5;
    }
    w.folk.mine_tones.push(t);
    if w.folk.mine_tones.len() > 8 {
        w.folk.mine_tones.remove(0);
    }
    let (up, down, _) = super::takeover::team_effect(w, team, msg, tone, weight);
    s.narrate(if worn >= 2 {
        "You've said it this way before. A few of them mouth the words with you."
    } else if up > down * 2 {
        "It lands. The noise when you finish is real."
    } else if down > up {
        "Polite silence. Some of them are already looking at their phones."
    } else {
        "They take it in. Practice will tell you more."
    });
    s.outcome.push(format!(
        "Your team talk: {up} lifted, {down} cooled{}.",
        if worn >= 2 { " (the tone is wearing thin)" } else { "" }
    ));
    s.end();
}

/// The room's mood for your team.
pub fn team_mood(w: &World, team: Team) -> f32 {
    match team {
        Team::College(p) => w.programs[p].team_mood,
        Team::Nfl(t) => w.clubs.get(t).map_or(0.0, |c| c.team_mood),
        Team::High(_) | Team::Other(_) => {
            let r = team.roster(w);
            r.iter().map(|&i| w.people[i].mood).sum::<f32>() / r.len().max(1) as f32
        }
    }
}

pub fn mood_words(m: f32) -> &'static str {
    match m {
        x if x > 0.45 => "as good as it gets: everybody bought in",
        x if x > 0.15 => "good, loose, confident",
        x if x > -0.15 => "steady, neither up nor down",
        x if x > -0.45 => "edgy: some grumbling, a few cliques",
        _ => "bad: players are talking behind your back",
    }
}

// ---------------------------------------------------------------------------
// Parents (topic 5: the people around him).

/// Who calls for a player's family.
/// His parents as a voice in his ear (screens topic 8): what they're like
/// and how sure your staff is of it.
pub fn parent_voice(w: &World, i: usize) -> (&'static str, &'static str) {
    use crate::world::background::ParentStyle;
    match w.people[i].family.parent_style {
        ParentStyle::LoudAtGames => ("Loud at games, and he hears every word of it", "Sure"),
        ParentStyle::SocialMedia => ("Posts about him online; that feed is part of his ear", "Sure"),
        ParentStyle::CalmPrivate => ("Quiet support at home", "Likely"),
        ParentStyle::Absent => ("Not much of a voice in his life", "A guess"),
    }
}

pub fn parent_word(w: &World, i: usize) -> &'static str {
    use crate::world::background::FamilyStructure as F;
    match w.people[i].family.structure {
        F::TwoParents => "His father",
        F::SingleParent => "His mother",
        F::Grandparents => "His grandmother",
        F::Foster => "His guardian",
    }
}

fn parent_open(w: &mut World, s: &mut Scene) {
    let id = s.a as u32;
    let Some(i) = super::person(w, id) else {
        s.end();
        return;
    };
    s.who.push(Who::Player(id));
    let p = &w.people[i];
    let style = p.family.parent_style;
    let who = parent_word(w, i);
    s.narrate(format!(
        "{who} calls about {}. {}",
        player_link(w, i),
        match style {
            crate::world::background::ParentStyle::LoudAtGames => "You've heard this voice from the stands.",
            crate::world::background::ParentStyle::SocialMedia => "Your staff has already shown you the posts.",
            _ => "It's the first time this parent has called.",
        }
    ));
    let spk = Speaker::parent(w, i);
    let f = player_facts(w, i);
    let mut pen = scene::pen(w, s, 1);
    let line = pen.line(&["parents.call.open", "?parents.call.point"], &spk, &f);
    pen.put(w);
    s.them(
        who,
        if line.is_empty() {
            format!("Coach, I'm going to be honest with you. {} should be on the field, and everybody in the stands can see it.", f.get("name").unwrap_or("My son"))
        } else {
            line
        },
        None,
    );
    s.pick(
        300,
        "\"Playing time is earned at practice. Here's what he needs to do.\"",
        "Tell him straight",
        "calm",
        "Honest: a reasonable parent respects it; a loud one goes to the AD.",
    );
    s.pick(
        301,
        "\"I hear you. Let me look at it with his position coach.\"",
        "Listen",
        "warm",
        "Cools the call down; promises nothing.",
    );
    s.pick(
        302,
        "\"He'll get his chance. I promise you that.\"",
        "Promise",
        "warm",
        "Pleases the parent now; logged as a promise of snaps.",
    );
    s.pick(
        303,
        "\"I don't discuss playing time with parents.\"",
        "Warn",
        "firm",
        "Ends it; the parent may go above you.",
    );
}

fn parent_answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    let id = s.a as u32;
    let Some(i) = super::person(w, id) else {
        s.end();
        return;
    };
    s.you(&scene::your_name(w), picked.label.clone());
    let loud = matches!(
        w.people[i].family.parent_style,
        crate::world::background::ParentStyle::LoudAtGames | crate::world::background::ParentStyle::SocialMedia
    );
    let (dtrust, line, above) = match code {
        300 => (
            0.03,
            if loud { "We'll see what the AD thinks about that." } else { "Okay. That's fair. I'll tell him." },
            loud && super::roll(w, 0x9A2, id as u64) < 0.4,
        ),
        301 => (0.04, "Thank you. That's all I'm asking.", false),
        302 => {
            promise(w, PromiseTo::Player(id), PromiseKind::Snaps, 0);
            (0.06, "That's what I wanted to hear, Coach.", false)
        }
        _ => (-0.06, "Then I'll talk to someone who does.", loud || super::roll(w, 0x9A3, id as u64) < 0.3),
    };
    s.them(parent_word(w, i), line, None);
    let p = &mut w.people[i];
    p.trust = (p.trust + dtrust).clamp(0.0, 1.0);
    if above && let Some(team) = s.team {
        let boss = super::cast::boss_role(team);
        let b = super::cast::get(w, super::cast::CastRef::new(team, boss, 0));
        w.folk.mine.nudge(b.key(), -0.02);
        s.outcome.push(format!("The parent went to {} about it.", b.link()));
        super::note(
            w,
            "talk",
            b.link(),
            format!("Heard from {}'s family about playing time.", player_link(w, i)),
            None,
        );
    }
    s.end();
}

// ---------------------------------------------------------------------------
// The scene hooks.

pub(crate) fn open(w: &mut World, s: &mut Scene) {
    match s.kind {
        Kind::OneOnOne => one_on_one_open(w, s),
        Kind::TeamTalk => team_talk_open(w, s),
        Kind::ParentCall => parent_open(w, s),
        Kind::StaffLine => staff_line_open(w, s),
        _ => super::takeover::open(w, s),
    }
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    match s.kind {
        Kind::OneOnOne => one_on_one_answer(w, s, code, picked),
        Kind::TeamTalk => team_talk_answer(w, s, code, picked),
        Kind::ParentCall => parent_answer(w, s, code, picked),
        Kind::StaffLine => staff_line_answer(w, s, code),
        _ => super::takeover::answer(w, s, code, picked),
    }
}

pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    match s.kind {
        Kind::OneOnOne => {
            // His position coach has the talk instead: a smaller version.
            // What you said before handing it over still lands (trust moved
            // on each line; his mood lands now, as at a talk's end).
            let id = s.a as u32;
            let spoke = s.lines.iter().any(|l| l.by == 1);
            if spoke {
                mood_event(w, id, Reason::Talk, s.f[1]);
            }
            if let Some(i) = super::person(w, id) {
                let rel = position_coach_rel(w, i);
                let d = 0.02 * (rel - 9.0) as f32 / 5.0;
                w.people[i].trust = (w.people[i].trust + d * 0.5).clamp(0.0, 1.0);
                mood_event(w, id, Reason::Talk, d);
                // He came to say he's leaving: his position coach asks him to stay.
                if asked_to_leave(w, id) {
                    super::takeover::position_coach_word(w, i);
                }
            }
            s.handed = Some("his position coach".into());
            s.outcome.push(if spoke {
                "His position coach finished the talk for you.".into()
            } else {
                "His position coach talked to him instead. It's not the same as hearing it from you.".into()
            });
        }
        Kind::TeamMeeting | Kind::Parents | Kind::Holdover => super::takeover::hand(w, s),
        Kind::ParentCall => {
            s.handed = Some("your staff".into());
            s.outcome.push("Your staff returned the call. The parent wanted you.".into());
            if let Some(i) = super::person(w, s.a as u32) {
                w.people[i].trust = (w.people[i].trust - 0.01).max(0.0);
            }
        }
        _ => {
            s.handed = Some("your staff".into());
            if s.outcome.is_empty() {
                s.outcome.push("Your staff handled it.".into());
            }
        }
    }
}

fn position_coach_rel(w: &World, i: usize) -> f64 {
    w.people[i]
        .college_staff
        .last()
        .and_then(|x| x.3)
        .and_then(|c| w.coaches.get(c))
        .map_or(9.0, |c| c.ability().relationships)
}

// ---------------------------------------------------------------------------
// Lines for your position coaches (topic 5: "keep him level", "push him",
// "tell me if he's slipping").

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Line {
    Level,
    Push,
    Watch,
}

impl Line {
    pub fn words(self) -> &'static str {
        match self {
            Line::Level => "keep him level",
            Line::Push => "push him",
            Line::Watch => "tell me if he's slipping",
        }
    }
}

/// Gives (or clears) your line to his position coach.
pub fn set_line(w: &mut World, id: u32, line: Option<Line>) {
    w.folk.lines.retain(|x| x.0 != id);
    if let Some(l) = line {
        w.folk.lines.push((id, l));
    }
}

/// Weekly: the position coaches carry out your lines, as well as their
/// relationships rating lets them.
fn carry_lines(w: &mut World) {
    let lines = w.folk.lines.clone();
    let team = Team::seat(w);
    for (id, l) in lines {
        let Some(i) = super::person(w, id) else { continue };
        if team.is_none_or(|t| !t.roster(w).contains(&i)) {
            continue;
        }
        let rel = position_coach_rel(w, i) as f32;
        let q = (rel - 6.0) / 10.0;
        match l {
            Line::Level => {
                let p = &mut w.people[i];
                p.mood *= 1.0 - 0.15 * q.max(0.0);
            }
            Line::Push => {
                let comp = ((w.people[i].player.personality.competitiveness - 10.0) / 10.0) as f32;
                let d = 0.02 * comp * q.max(0.1);
                w.people[i].mood = (w.people[i].mood + d).clamp(-1.0, 1.0);
            }
            Line::Watch => {
                let p = &w.people[i];
                let noticed = super::roll(w, 0x5117 ^ w.today.0 as u64, id as u64) < 0.4 + 0.5 * q as f64;
                if p.mood < -0.3
                    && noticed
                    && !w.folk.scenes.iter().any(|s| !s.done && s.kind == Kind::StaffLine && s.a == id as u64)
                {
                    let name = p.name.clone();
                    let mut s = Scene::new(Kind::StaffLine, w.today, format!("{name} is slipping"));
                    s.a = id as u64;
                    s.stops = false;
                    s.place = "In the hallway after practice".into();
                    scene::add(w, s);
                }
            }
        }
    }
}

fn staff_line_open(w: &mut World, s: &mut Scene) {
    let id = s.a as u32;
    let Some(i) = super::person(w, id) else {
        s.end();
        return;
    };
    s.who.push(Who::Player(id));
    let pc = w.people[i].college_staff.last().and_then(|x| x.3);
    let coach = pc.map_or_else(|| "His position coach".to_string(), |c| super::coach_link(w, c));
    if let Some(c) = pc {
        s.who.push(Who::Coach(c));
    }
    let (_, words) = concern(w, i);
    s.narrate(format!("{coach} catches you after practice about {}.", player_link(w, i)));
    s.them(
        "Position coach",
        format!("You asked me to tell you if he was slipping. He is. Best I can tell, {words}."),
        None,
    );
    s.pick(400, "\"Send him to my office tomorrow.\"", "Ask", "warm", "A one-on-one on your calendar.");
    s.pick(
        401,
        "\"Stay on him. You know him better than I do.\"",
        "Hand him something",
        "calm",
        "His coach keeps it; no time from your week.",
    );
}

fn staff_line_answer(w: &mut World, s: &mut Scene, code: u32) {
    let id = s.a as u32;
    if code == 400 {
        if let Some(i) = super::person(w, id) {
            let mut sc = Scene::new(Kind::OneOnOne, w.today.add(1), format!("One-on-one with {}", w.people[i].name));
            sc.a = id as u64;
            sc.b = concern(w, i).0 as u64;
            sc.place = "Your office".into();
            sc.stops = false;
            scene::add(w, sc);
            s.outcome.push("A one-on-one is on your calendar for tomorrow.".into());
        }
    } else {
        s.outcome.push("His position coach stays on him.".into());
        if let Some(i) = super::person(w, id) {
            let d = 0.03 * ((position_coach_rel(w, i) - 8.0) / 8.0) as f32;
            mood_event(w, id, Reason::Talk, d);
        }
    }
    s.end();
}

// ---------------------------------------------------------------------------
// The day.

pub(crate) fn day(w: &mut World, d: Date) {
    check_promises(w, d);
    if d.weekday() == Weekday::Mon {
        // Old reasons that have faded away are dropped.
        let today = w.today;
        for x in w.folk.moods.iter_mut() {
            x.1.retain(|e| e.0.days_to(today) < 240);
        }
        w.folk.moods.retain(|x| !x.1.is_empty());
        if w.seat.is_some() {
            carry_lines(w);
            parent_calls(w, d);
            exit_meetings(w, d);
        }
        ai_talks(w, d);
    }
}

/// One of your players asks to see you before he enters the portal: a
/// one-on-one about his future on your calendar. What you say there moves
/// his lean; he decides the day after (`recruiting::intents_day`).
pub(crate) fn portal_ask(w: &mut World, i: usize, d: Date) {
    let mut s = Scene::new(Kind::OneOnOne, d, format!("{} wants to talk about his future", w.people[i].name));
    s.a = w.people[i].player.id as u64;
    s.b = Subject::Future as u64;
    s.place = "Your office".into();
    scene::add(w, s);
}

/// Whether he asked for this talk because he means to enter the portal.
fn asked_to_leave(w: &World, id: u32) -> bool {
    w.portal_intents.iter().any(|t| t.is(id))
}

/// A loud parent calls when his son's role sours (high school and
/// college; proposed, not confirmed: a few a season).
fn parent_calls(w: &mut World, d: Date) {
    let Some(team) = Team::seat(w) else { return };
    if team.level() == Level::Nfl {
        return;
    }
    if w.folk.scenes.iter().any(|s| s.kind == Kind::ParentCall && s.date.days_to(d) < 14) {
        return;
    }
    let roster = team.roster(w);
    let pick = roster.iter().copied().find(|&i| {
        let p = &w.people[i];
        p.mood_why == Why::Role as u8
            && p.mood < -0.25
            && matches!(
                p.family.parent_style,
                crate::world::background::ParentStyle::LoudAtGames | crate::world::background::ParentStyle::SocialMedia
            )
            && super::roll(w, 0xCA11 ^ d.0 as u64, p.player.id as u64) < 0.25
    });
    if let Some(i) = pick {
        let mut s = Scene::new(Kind::ParentCall, d, format!("{}'s family calls", w.people[i].name));
        s.a = w.people[i].player.id as u64;
        s.stops = false;
        s.place = "On the phone".into();
        scene::add(w, s);
    }
}

/// January's exit meetings (topic 5): the players whose decision is open
/// get a one-on-one on your calendar; your staff meets the rest.
fn exit_meetings(w: &mut World, d: Date) {
    let Some(team) = Team::seat(w) else { return };
    let day = match team.level() {
        Level::Nfl => w.dates.black_monday.add(1),
        Level::HighSchool => w.dates.december_graduation,
        _ => w.dates.bowl_season.1.add(2),
    };
    if d < day
        || d >= day.add(7)
        || w.folk
            .scenes
            .iter()
            .any(|s| s.kind == Kind::OneOnOne && s.title.starts_with("Exit meeting") && s.date.days_to(d) < 60)
    {
        return;
    }
    let roster = team.roster(w);
    let mut open: Vec<(usize, f64)> = roster
        .iter()
        .copied()
        .filter(|&i| !super::facts::leaving(w, i) || w.people[i].stage == Stage::College)
        .map(|i| {
            let p = &w.people[i];
            let draft = p.cal.draft_read.map_or(0.0, |r| if r.round <= 3 { 1.0 } else { 0.3 });
            let score = 3.0 * p.cal.lean
                + draft
                + if p.mood < -0.2 { 0.5 } else { 0.0 }
                + if p.last_role == Some(PlayingTime::Starter) { 0.3 } else { 0.0 };
            (i, score)
        })
        .filter(|x| x.1 > 0.6)
        .collect();
    open.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (k, (i, _)) in open.into_iter().take(8).enumerate() {
        let mut s = Scene::new(Kind::OneOnOne, d.add(k as i32 / 3), format!("Exit meeting: {}", w.people[i].name));
        s.a = w.people[i].player.id as u64;
        s.b = Subject::Future as u64;
        s.stops = false;
        s.place = "Your office".into();
        scene::add(w, s);
    }
}

/// AI coaches talk too (topic 5): each week an AI head coach sees his
/// unhappiest players, and how it goes reads his relationships rating.
fn ai_talks(w: &mut World, d: Date) {
    let seat = Team::seat(w);
    let in_season = d >= w.dates.fall_camp && d <= w.dates.bowl_season.1;
    if !in_season && d.weekday() == Weekday::Mon && d.0 % 14 != 0 {
        return;
    }
    let mut moves: Vec<(usize, f32, f32)> = Vec::new();
    for p in 0..w.programs.len() {
        if seat == Some(Team::College(p)) {
            continue;
        }
        let Some(c) = w.programs[p].head_coach else { continue };
        let rel = w.coaches[c].ability().relationships as f32;
        let mut worst: Vec<(usize, f32)> = w.programs[p]
            .roster
            .iter()
            .copied()
            .filter(|&i| w.people[i].stage == Stage::College && w.people[i].mood < -0.3)
            .map(|i| (i, w.people[i].mood))
            .collect();
        worst.sort_by(|a, b| a.1.total_cmp(&b.1));
        for (i, _) in worst.into_iter().take(2) {
            let u = super::roll(w, 0xA17A ^ d.0 as u64, i as u64) as f32;
            let good = u < 0.25 + 0.035 * rel;
            moves.push((i, if good { 0.04 } else { -0.03 }, if good { 0.08 } else { -0.03 }));
        }
    }
    for (i, dt, dm) in moves {
        let p = &mut w.people[i];
        p.trust = (p.trust + dt).clamp(0.0, 1.0);
        p.mood = (p.mood + dm).clamp(-1.0, 1.0);
    }
}
