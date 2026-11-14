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

use super::claims::{self, Kind as PointKind, Point, Sheet};
use super::scene::{self, Kind, Pick, PickKind, Scene, Who};
use super::takeover::{Message, Tone};
pub use super::talk_points::Topic;
use super::talk_points::{self as tp, Answer, Rest, Stance};
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
    pub(crate) fn key(self) -> &'static str {
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
    /// A transfer's promise when he came (portal recruiting): 0 a starting
    /// job, 1 a real role, every week. Read off his games for you, not last
    /// season's role at his old school.
    Join(u8),
    /// A high school recruit's promise when he signed (talk facts lane L6):
    /// the kind (`pledge::Kind`) and his position when you made it.
    Recruit(u8, u8),
    /// More touches (carries and targets) than now (the promises design,
    /// section 3; lane P1).
    Touches,
    /// You won't move him from his position (the position's index; lane P1).
    Position(u8),
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
            PromiseKind::Join(0) => "a starting job when he came".into(),
            PromiseKind::Join(_) => "a real role when he came".into(),
            PromiseKind::Recruit(k, pos) => super::pledge::Kind::of(k).map_or_else(
                || "a promise when he signed".into(),
                |x| x.words(crate::positions::Position::ALL[pos as usize % crate::positions::POSITION_COUNT]),
            ),
            PromiseKind::Touches => "more touches".into(),
            PromiseKind::Position(pos) => format!(
                "that he'd stay at {}",
                super::claims::words::spot(
                    crate::positions::Position::ALL[pos as usize % crate::positions::POSITION_COUNT]
                )
            ),
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
    /// What you backed a playing-time promise with (playtest bug 15, Cain's
    /// 2c): 1 a real strength of his you named, -1 one he knows isn't, 0
    /// nothing. His belief reads it (`stay_read::promise_belief`).
    #[serde(default)]
    pub backed: i8,
    /// Who, in words with link marks.
    pub who: String,
    /// Its terms: by when, on what condition, ahead of whom (the promises
    /// design, section 3; lane P1).
    #[serde(default)]
    pub terms: super::promise::Terms,
    /// How far he believes it now, 0-1 (section 4): never shown as a
    /// number. 0 for a promise from before lane P1 until he hears it.
    #[serde(default)]
    pub belief: f32,
    /// His belief when you said it (keeping it pays more the more he
    /// doubted, section 7).
    #[serde(default)]
    pub said: f32,
    /// What he sees of it (`promise::State`).
    #[serde(default)]
    pub state: u8,
    /// Weeks in a row behind, and owed (section 6: he comes to you).
    #[serde(default)]
    pub behind: u8,
    #[serde(default)]
    pub owed: u8,
    /// The last day he came to you about it.
    #[serde(default)]
    pub came: Option<Date>,
    /// His own deadline (section 3): on the day, if he doesn't have it,
    /// his lean jumps.
    #[serde(default)]
    pub his_by: Option<Date>,
    /// The share missed when it was judged (0 kept in full).
    #[serde(default)]
    pub missed: f32,
    /// He missed his condition and released you (section 6).
    #[serde(default)]
    pub released: bool,
}

impl Promise {
    /// A promise with nothing said yet beyond who and what.
    pub fn blank(to: PromiseTo, kind: PromiseKind) -> Promise {
        Promise {
            to,
            kind,
            made: Date(0),
            due: Date(0),
            public: false,
            kept: None,
            backed: 0,
            who: String::new(),
            terms: Default::default(),
            belief: 0.0,
            said: 0.0,
            state: 0,
            behind: 0,
            owed: 0,
            came: None,
            his_by: None,
            missed: 0.0,
            released: false,
        }
    }
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
    w.folk.promises.push(Promise { made: today, due, who, ..Promise::blank(to, kind) });
    if w.folk.promises.len() > 120
        && let Some(k) = w.folk.promises.iter().position(|p| p.kept.is_some())
    {
        w.folk.promises.remove(k);
    }
    // He hears it (the promises design, section 4): his belief now, and a
    // named teammate or a second man promised the same spot may find out.
    if let PromiseTo::Player(_) = to {
        let k = w.folk.promises.len() - 1;
        super::promise::hear(w, k);
        let heard = super::promise::spread(w, k);
        for x in heard {
            super::note(w, "talk", "Your promises".into(), x, None);
        }
    }
}

/// Sets the terms of a promise just made and hears it again (a follow-up
/// answer, a typed line's terms). Returns his belief.
pub fn set_terms(w: &mut World, k: usize, t: super::promise::Terms) -> f64 {
    w.folk.promises[k].terms = t;
    w.folk.promises[k].due = due_for(w, w.folk.promises[k].kind, t.by);
    let b = super::promise::hear(w, k);
    for x in super::promise::spread(w, k) {
        super::note(w, "talk", "Your promises".into(), x, None);
    }
    b
}

/// A playing-time promise's day: the end of the term its window opens in
/// (section 7: judged over the games, from its date to the end of its
/// term; the season unless you said otherwise).
fn due_for(w: &World, kind: PromiseKind, by: super::promise::By) -> Date {
    let base = promise_due(w, kind, 0);
    if !super::promise::over_games(kind) {
        return base;
    }
    let from = super::promise::opens(w, w.today, by);
    let mut d = base;
    while d < from.add(7) {
        d = d.add(364);
    }
    d
}

fn promise_due(w: &World, kind: PromiseKind, seasons: u8) -> Date {
    let today = w.today;
    let end = |w: &World| match Team::seat(w).map(|t| t.level()) {
        Some(Level::Nfl) => w.dates.black_monday,
        _ => w.dates.fbs_regular_end.add(3),
    };
    let base = match kind {
        // Judged over the games to the season's end (the promises design,
        // section 7), not on one day.
        PromiseKind::Start
        | PromiseKind::Snaps
        | PromiseKind::Touches
        | PromiseKind::Place(_)
        | PromiseKind::Join(_)
        | PromiseKind::Position(_) => {
            let e = end(w);
            if e > today.add(14) { e } else { e.add(365) }
        }
        PromiseKind::Contract => today.add(150),
        PromiseKind::Stay | PromiseKind::Raise => today.add(330),
        PromiseKind::Booster(_) => end(w).max(today.add(60)),
        PromiseKind::Recruit(k, _) => super::pledge::due(w, k),
    };
    base.add(
        365 * seasons.saturating_sub(1) as i32 * matches!(kind, PromiseKind::Booster(_) | PromiseKind::Stay) as i32,
    )
}

/// Checks the promises due today (the promises design, section 7): a
/// playing-time promise is judged over its games, kept in 3 of every 4
/// healthy games, broken by the share missed, released when he missed his
/// condition; the rest on their day as before. Keeping it pays more the
/// more he doubted; breaking it costs by what he counted on.
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
        let verdict = super::promise::judge(w, &p).unwrap_or_else(|| {
            if kept(w, &p) { super::promise::Verdict::Kept } else { super::promise::Verdict::Broken(1.0) }
        });
        let public = if p.public { 2.0 } else { 1.0 };
        let (kept, scale, text) = match verdict {
            super::promise::Verdict::Kept => (true, 1.0, "You kept"),
            super::promise::Verdict::Broken(share) => (
                false,
                super::promise::broken_scale(p.belief, share),
                if share >= 0.99 { "You broke" } else { "You half kept" },
            ),
            super::promise::Verdict::Released(holds) => {
                w.folk.promises[k].released = true;
                if holds <= 0.0 {
                    w.folk.promises[k].kept = Some(true);
                    let text = format!(
                        "{} missed the condition on the promise of {} you made on {}: you're released.",
                        p.who,
                        p.kind.words(),
                        p.made.pretty()
                    );
                    super::note(w, "talk", "Your promises".into(), text, None);
                    continue;
                }
                (false, super::promise::broken_scale(p.belief, holds), "He holds you to")
            }
            super::promise::Verdict::Lapsed => {
                w.folk.promises[k].kept = Some(true);
                w.folk.promises[k].released = true;
                continue;
            }
        };
        w.folk.promises[k].kept = Some(kept);
        w.folk.promises[k].missed = if kept { 0.0 } else { scale };
        let text = format!("{} the promise of {} you made to {} on {}.", text, p.kind.words(), p.who, p.made.pretty());
        let cost = if kept { 1.0 } else { scale * public };
        let p = w.folk.promises[k].clone();
        settle_promise(w, &p, kept, cost);
        super::note(w, "talk", "Your promises".into(), text.clone(), None);
        super::post_mine(w, d, Loudness::Small, false, "people: promise", &text);
    }
}

/// A promise kept or broken, with its audiences and his belief when you
/// said it (section 7): `cost` is the broken share x what he counted on x
/// the public factor.
fn settle_promise(w: &mut World, p: &Promise, kept: bool, cost: f32) {
    let names = super::promise::audiences(w, p);
    settle(w, p.to, kept, cost, if kept { Reason::PromiseKept } else { Reason::PromiseBroken }, p.said, &names);
}

/// What your word kept or broken does (the cost of a promise, written
/// once): with a player, his trust and mood, and when broken his close
/// friends' trust, a grudge against his club and your name for keeping your
/// word with his audience; with a cast person or a coach, his trust in you.
/// `cost` is the share of the full cost (said in public, 2). A lie he sees
/// runs through it with `Reason::LieSeen` (section 11).
pub(crate) fn settle_word(w: &mut World, to: PromiseTo, kept: bool, cost: f32, why: Reason) {
    settle(w, to, kept, cost, why, 0.0, &[super::Name::Room])
}

fn settle(w: &mut World, to: PromiseTo, kept: bool, cost: f32, why: Reason, said: f32, names: &[super::Name]) {
    match to {
        PromiseTo::Player(id) => {
            if let Some(i) = super::person(w, id) {
                let dt = if kept { super::promise::kept_trust(said) } else { -0.22 * cost };
                let pp = &mut w.people[i];
                pp.trust = (pp.trust + dt).clamp(0.0, 1.0);
                mood_event(w, id, why, if kept { 0.1 * 1.4 } else { dt * 1.4 });
                if !kept {
                    for f in super::ties::friends(w, id) {
                        if let Some(j) = super::person(w, f) {
                            w.people[j].trust = (w.people[j].trust - 0.04 * cost).max(0.0);
                        }
                    }
                    if let Some(c) = w.people[i].pro.as_ref().and_then(|c| c.team) {
                        crate::world::people::grudge(&mut w.people[i], c, 0.6 * cost.min(1.0) as f64);
                    }
                    for &n in names {
                        w.folk.mine.word(n, -0.08 * cost);
                    }
                } else {
                    for &n in names {
                        w.folk.mine.word(n, 0.04);
                    }
                }
            }
        }
        PromiseTo::Cast(r, term) => {
            let c = super::cast::get(w, r);
            if c.term == term {
                w.folk.mine.nudge(c.key(), if kept { 0.1 } else { -0.2 * cost });
            }
        }
        PromiseTo::Coach(c) => w.folk.mine.nudge(super::coach_key(c), if kept { 0.1 } else { -0.2 }),
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
            // What he sees, never your chart (the promises design, section 2).
            let here = Team::seat(w).is_some_and(|t| t.roster(w).contains(&i));
            match kind {
                PromiseKind::Start => super::seen::starter(w, i),
                PromiseKind::Snaps | PromiseKind::Touches => {
                    super::seen::starter(w, i) || super::promise::share_now(w, i) >= super::promise::ROLE_SHARE
                }
                PromiseKind::Position(pos) => here && pp.player.position as u8 == pos,
                PromiseKind::Place(r) => {
                    // His place as he sees it: 0 a starter, 1 a real role,
                    // 2 any other place on the team.
                    let seen = if super::seen::starter(w, i) {
                        0
                    } else if super::promise::share_now(w, i) >= super::promise::ROLE_SHARE {
                        1
                    } else {
                        2
                    };
                    here && (r >= 2 || seen <= r)
                }
                PromiseKind::Join(r) => {
                    let mine = Team::seat(w).and_then(|t| match t {
                        Team::College(c) => Some(c as u32),
                        _ => None,
                    });
                    let line = w
                        .tallies
                        .players
                        .get(crate::world::tally::Level::College, id)
                        .filter(|s| Some(s.team) == mine && w.tallies.season == w.dates.season);
                    let (games, starts) = line.map_or((0, 0), |s| (s.games, s.starts));
                    here && if r == 0 {
                        starts > 0 || super::seen::with_the_ones(w, id)
                    } else {
                        games >= 8 || starts >= 3
                    }
                }
                PromiseKind::Recruit(k, pos) => {
                    let mine = Team::seat(w).and_then(|t| match t {
                        Team::College(c) => Some(c as u32),
                        _ => None,
                    });
                    let line = w
                        .tallies
                        .players
                        .get(crate::world::tally::Level::College, id)
                        .filter(|s| Some(s.team) == mine && w.tallies.season == w.dates.season);
                    let (games, starts) = line.map_or((0, 0), |s| (s.games, s.starts));
                    here && super::pledge::kept(w, i, k, pos, games as u32, starts as u32)
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
    let (
        PromiseTo::Player(id),
        PromiseKind::Place(_)
        | PromiseKind::Join(_)
        | PromiseKind::Recruit(..)
        | PromiseKind::Start
        | PromiseKind::Snaps
        | PromiseKind::Touches
        | PromiseKind::Position(_),
    ) = (p.to, p.kind)
    else {
        return false;
    };
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
                // A recruit who never came (he didn't qualify, or went
                // elsewhere first): the promise was his to take up.
                || matches!(p.kind, PromiseKind::Recruit(..)) && pp.college.is_none()
        }
        Team::High(h) => pp.stage != Stage::HighSchool || pp.hs != h,
    }
}

/// A starter as he sees it (the promises design, section 2): with the
/// ones and starting in his last practice weeks and games, else last
/// season's role. Never your chart.
pub(crate) fn is_starter(w: &World, i: usize) -> bool {
    super::seen::starter(w, i)
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
    /// His arrest, for a man who cares how it looks (talk facts lane L4).
    ArrestShame,
    /// His arrest, for a man who takes it out on everyone.
    ArrestAnger,
    // Live talks (the core declares them; the lanes that use them fill the
    // sizes). Their codes sit apart from the list above, so a reason a chips
    // lane adds there never takes one of their numbers in a save.
    /// He saw that something you told him in a typed line was false (lanes
    /// 9 and 10, once the promise design sets the cost).
    LieSeen = 40,
    /// His agent told him what you brought up about him in his contract
    /// talks: his arrest, his injuries (lane 7, LT8).
    HardballHeard = 41,
    /// What you told him when you recruited him wasn't true (lane 10).
    RecruitLie = 42,
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
            14 => Reason::ArrestShame,
            15 => Reason::ArrestAnger,
            40 => Reason::LieSeen,
            41 => Reason::HardballHeard,
            42 => Reason::RecruitLie,
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
            (Reason::ArrestShame, _) => "ashamed of his arrest",
            (Reason::ArrestAnger, _) => "angry about his arrest",
            (Reason::LieSeen, _) => "found out something you told him wasn't true",
            (Reason::HardballHeard, _) => "heard what you said about him in his contract talks",
            (Reason::RecruitLie, _) => "what you told him when you recruited him wasn't true",
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
    // The winter rush's first seven days are one long arrival week (Jan 1
    // start): forty hours of people time, interviews included (proposed,
    // not confirmed).
    if super::takeover::rush_week(w).is_some() {
        return 40.0;
    }
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

/// The week your people time is counted in: the arrival week from its
/// first day, else the week from Monday.
fn week_of(w: &World) -> Date {
    super::takeover::rush_week(w).unwrap_or_else(|| week_start(w.today))
}

/// Hours left this week (all of them, every week, with sandbox mode's
/// hours switch).
pub fn hours_left(w: &World) -> f32 {
    if w.sandbox(crate::world::sandbox::Switch::YourHours) {
        return week_budget(w);
    }
    let used = if w.folk.mine.week == Some(week_of(w)) { w.folk.mine.hours } else { 0.0 };
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
    let wk = week_of(w);
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
    let wk = week_of(w);
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
    worry_on(w, i, &claims::player(w, i)).map(|(t, s)| (t.subject(), s))
}

/// What is really on his mind, read from his fact sheet (talk facts lane
/// L4): the talking point that weighs most on him, when it is one you
/// know and big enough (`talk_points::WORRY_FLOOR`); else the old reads
/// (a hard time at home, the portal, a friend gone, his mood's reason).
pub(crate) fn worry_on(w: &World, i: usize, sheet: &Sheet) -> Option<(Topic, String)> {
    if let Some(pt) = tp::on_mind(sheet).filter(|pt| pt.known) {
        return Some((Topic::Point(pt.kind), tp::worry_words(pt)));
    }
    worry_old(w, i).map(|(s, x)| (Topic::Subject(s), x))
}

fn worry_old(w: &World, i: usize) -> Option<(Subject, String)> {
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
/// while it weighs on him, then his talking points you know (talk facts
/// lane L4: the charge, the job he lost, last week's game), then the
/// subjects you can always raise: (key, name for a chip). His behavior is
/// a subject only when something on his sheet is behind it.
pub fn subjects_for(w: &World, id: u32) -> Vec<(&'static str, String)> {
    let mut v: Vec<(&'static str, String)> = Vec::new();
    if let Some(f) = friend_gone(w, id).and_then(|f| super::person(w, f)) {
        v.push((Subject::Friend.key(), format!("{} leaving", first_name(&w.people[f].name))));
    }
    let mut behavior = false;
    if let Some(i) = super::person(w, id) {
        let sheet = claims::player(w, i);
        for pt in tp::chips(&sheet) {
            v.push((pt.kind.key(), tp::chip_name(pt)));
        }
        behavior = sheet.trouble() || sheet.has(PointKind::Effort);
    }
    for s in Subject::ALL {
        if s == Subject::Behavior && !behavior {
            continue;
        }
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
    start_talk_on(w, id, subject.into())
}

/// Opens a one-on-one on a subject or one of his talking points (a chip's
/// key, `Topic::from_key`).
pub fn start_talk_on(w: &mut World, id: u32, topic: Topic) -> Result<u32, String> {
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
    s.b = topic.code();
    s.stops = false;
    s.place = match team {
        Team::High(_) => "The coaches' office, after school".into(),
        Team::Nfl(_) | Team::Other(_) => "Your office at the facility".into(),
        Team::College(_) => "Your office in the football building".into(),
    };
    Ok(scene::add(w, s))
}

pub(crate) fn player_facts(w: &World, i: usize) -> Facts {
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
    let mut sheet = claims::player(w, i);
    let mut topic = Topic::of(s.b);
    // His friend's leaving is a subject only while it weighs on him.
    if topic == Topic::Subject(Subject::Friend) && friend_gone(w, id).is_none() {
        topic = Subject::Role.into();
    }
    // A point that left his sheet since (its window passed) is its subject.
    if let Topic::Point(k) = topic
        && !sheet.known().any(|p| p.kind == k)
    {
        topic = tp::subject_of(k).into();
    }
    let asked = asked_to_leave(w, id);
    // An exit meeting opens on why he is on your list (lane L9).
    let exit = s.c & EXIT != 0;
    // He raises what weighs on him himself (talk facts lane L4): an open
    // case every time, else his top point when it weighs on him. One you
    // don't know (trouble at home he hasn't told you) he raises if he opens
    // up to you.
    let lead = tp::leads(&sheet)
        .filter(|p| Topic::Point(p.kind) != topic && (!(asked || exit) || p.kind == PointKind::Arrest));
    if let Some((k, known)) = lead.map(|p| (p.kind, p.known)) {
        if known {
            topic = Topic::Point(k);
        } else if k.is_life() && opens_up(w, i, s.id, topic.subject() == Subject::Life) {
            super::life::tell(w, id, super::life::event_of(k));
            sheet = claims::player(w, i);
            if sheet.known().any(|p| p.kind == k) {
                topic = Topic::Point(k);
            }
        }
    }
    s.b = topic.code();
    let subject = topic.subject();
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
    if asked {
        s.narrate("He asked for this meeting.");
    }
    if exit {
        s.narrate(format!("Your staff's note on him: {}.", exit_note(w, s, i, &sheet)));
    }
    s.narrate(format!("In comes {}. {posture}", player_link(w, i)));
    let spk = Speaker::player(w, i);
    let point = topic.point().and_then(|k| sheet.get(k)).cloned();
    let stance = point.as_ref().map(|p| tp::stance(w, i, p, s.id as u64));
    set_stance(s, stance);
    let mut f = player_facts(w, i);
    f.set("subject", subject.name());
    if let Some(r) = crate::world::recruiting::intent_reason(w, id) {
        f.set("reason", r);
    }
    if let Some(p) = &point {
        f = tp::point_facts(&f, p, stance);
    }
    tp::conduct(&sheet, point.as_ref(), stance, &mut f);
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
    let good = point.as_ref().is_some_and(tp::is_good);
    let keys = if asked && point.is_none() {
        vec![format!("talk.open.portal.{tone}")]
    } else if good {
        // Good news: no subject line fits it ("Is this about where I'm at
        // on the depth chart?"), so his greeting alone.
        vec![format!("talk.open.{tone}")]
    } else if subject == Subject::Friend && tone == "good" {
        vec!["talk.open.friend".to_string()]
    } else {
        vec![format!("talk.open.{tone}"), format!("talk.open.{}", subject.key())]
    };
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    // On a point, his own opening in his stance; until the bank has one,
    // today's opening for its subject.
    // An exit meeting no point carries: his own opening for your staff's
    // reason (lane L9), else today's.
    let leaving = claims::team_rooms::Leaving::of_code((s.c >> EXIT_SHIFT) & 0xF);
    if exit
        && leaving == claims::team_rooms::Leaving::OldCoach
        && let Some(o) = w.folk.takeover.as_ref().and_then(|t| t.old_coach)
    {
        let n = &w.coaches[o].name;
        f.set("old", n.clone());
        f.set("old_last", n.split_whitespace().last().unwrap_or(n).to_string());
    }
    let exit_key = leaving.key().filter(|_| exit && point.is_none() && !asked);
    let line = match (&point, stance, exit_key) {
        (Some(p), Some(st), _) => {
            let own = point_open(&mut pen, p, st, good, &spk, &f);
            if own.is_empty() { pen.line(&keys, &spk, &f) } else { own }
        }
        (_, _, Some(k)) => {
            let own = pen.line(&[k], &spk, &f);
            if own.is_empty() { pen.line(&keys, &spk, &f) } else { own }
        }
        _ => pen.line(&keys, &spk, &f),
    };
    let line = match (line.is_empty(), asked && point.is_none()) {
        (false, _) => line,
        (true, true) => {
            "Coach, I wanted you to hear it from me first. I'm thinking about putting my name in the portal.".into()
        }
        (true, false) if good => "Coach. You wanted to see me?".into(),
        (true, false) => open_fallback(subject, mood),
    };
    let tell = match (&point, stance) {
        (Some(p), Some(st)) => tp::stance_tell(p, st, sharpness(w)).or_else(|| {
            // "This is what's on his mind" only when it is: his top point.
            subject_tell(w, i, sheet.top().is_some_and(|t| t.kind == p.kind && t.mind >= tp::WORRY_FLOOR))
        }),
        _ if worry(w, i).is_some_and(|x| x.0 == subject) => subject_tell(w, i, true),
        _ => read_tell(w, i, if trust < 0.4 { -1 } else { 0 }),
    };
    s.them(&name, line, tell);
    // A point with a question comes as one; any of them asks once a season
    // if you're staying when your name is in a job story.
    let q = point.as_ref().and_then(|p| tp::question(w, i, p)).map(|q| (q, false)).or_else(|| {
        if asked || point.as_ref().is_some_and(|p| p.kind.is_trouble() || p.kind.is_life()) {
            None
        } else {
            tp::job_question(w, i).map(|q| (q, true))
        }
    });
    if let Some((q, job)) = q {
        let qf = if job { player_facts(w, i) } else { f.clone() };
        let said = pen.line(&[&format!("talk.point.ask.{}", q.key)], &spk, &qf);
        pen.put(w);
        s.them(&name, if said.is_empty() { q.fallback.clone() } else { said }, None);
        s.c |= QUESTION | if job { JOB_QUESTION } else { 0 };
        add_answers(s, &q);
        return;
    }
    pen.put(w);
    add_picks(w, s, i, topic, &[]);
}

/// A move's bit in `Scene.c`: used in this talk.
fn used_bit(m: Move) -> i64 {
    1 << (2 + Move::ALL.iter().position(|x| *x == m).unwrap_or(0))
}

/// Bits of `Scene.c`: he asked you something and waits on your answer; the
/// question is whether you're staying.
const QUESTION: i64 = 1 << 20;
const JOB_QUESTION: i64 = 1 << 21;

fn set_stance(s: &mut Scene, st: Option<Stance>) {
    s.c &= !(0xF << 16);
    if let Some(st) = st {
        s.c |= st.bits();
    }
}

/// Whether he opens up to you today (topic 5): his trust and personality,
/// a little more on what you both know is on his mind. One draw a talk.
fn opens_up(w: &World, i: usize, scene: u32, shared: bool) -> bool {
    let p = &w.people[i];
    let open = 0.15 + 0.7 * p.trust + 0.1 * p.player.personality.traits.z(Trait::Adaptability) as f32
        - 0.1 * p.player.personality.traits.z(Trait::Ego).max(0.0) as f32
        + 0.15 * shared as u8 as f32;
    (super::roll(w, 0xA5C ^ scene as u64, p.player.id as u64) as f32) < open
}

/// Your answers to his question, then the way out.
fn add_answers(s: &mut Scene, q: &tp::Question) {
    for (a, label, hint) in &q.answers {
        let m = tp::answer_move(*a);
        if m.is_some_and(|m| s.c & used_bit(m) != 0) {
            continue;
        }
        s.picks.push(
            Pick {
                label: format!("\"{label}\""),
                mv: match a {
                    Answer::Straight => "Tell him straight",
                    Answer::Soften => "Soften it",
                    Answer::Promise => "Promise",
                    Answer::Dodge => "Dodge it",
                }
                .into(),
                tone: match a {
                    Answer::Straight => Tone::Calm,
                    Answer::Soften | Answer::Promise => Tone::Warm,
                    Answer::Dodge => Tone::Firm,
                }
                .name()
                .into(),
                hint: (*hint).into(),
                code: a.code(),
                ..Pick::default()
            }
            .on(q.key)
            .of(if m == Some(Move::Promise) { PickKind::Final } else { PickKind::Plain }),
        );
    }
    s.pick(900, "End the talk", "End", "calm", "Ends it here.");
}

/// You turn a one-on-one to another subject (dialogue D1, the subject
/// chips): he answers on that one now, and the talk goes on. It costs no
/// exchange.
pub fn change_subject(w: &mut World, id: u32, subject: Subject) -> Result<(), String> {
    change_topic(w, id, subject.into())
}

/// You turn a one-on-one to another subject or one of his talking points
/// (a chip): he answers on it now in his stance, and the talk goes on.
pub fn change_topic(w: &mut World, id: u32, topic: Topic) -> Result<(), String> {
    let k = w.folk.scenes.iter().position(|s| s.id == id).ok_or("No such talk.")?;
    let mut s = w.folk.scenes[k].clone();
    if s.done || s.kind != Kind::OneOnOne || !s.opened {
        return Err("That talk isn't going on.".into());
    }
    if Topic::of(s.b) == topic {
        return Ok(());
    }
    let Some(i) = super::person(w, s.a as u32) else { return Err("He's gone.".into()) };
    let gone = friend_gone(w, s.a as u32).and_then(|g| super::person(w, g)).map(|g| first_name(&w.people[g].name));
    if topic == Topic::Subject(Subject::Friend) && gone.is_none() {
        return Err("That's behind him now.".into());
    }
    let sheet = claims::player(w, i);
    let point = match topic {
        Topic::Point(k) => Some(sheet.known().find(|p| p.kind == k).cloned().ok_or("That's behind him now.")?),
        Topic::Subject(_) => None,
    };
    let subject = topic.subject();
    s.b = topic.code();
    s.c &= !(1 | QUESTION | JOB_QUESTION);
    let about = match (&gone, topic, &point) {
        (Some(g), Topic::Subject(Subject::Friend), _) => format!("{g} leaving"),
        (_, _, Some(p)) => {
            let n = tp::chip_name(p);
            let mut c = n.chars();
            c.next().map_or_else(String::new, |f| f.to_lowercase().collect::<String>() + c.as_str())
        }
        _ => subject.name().to_string(),
    };
    let about = about.split(' ').map(|x| if x == "his" { "your" } else { x }).collect::<Vec<_>>().join(" ");
    s.you(&scene::your_name(w), format!("Let's talk about {about}."));
    let spk = Speaker::player(w, i);
    let stance = point.as_ref().map(|p| tp::stance(w, i, p, s.id as u64 ^ 0x7091C));
    set_stance(&mut s, stance);
    let mut f = player_facts(w, i);
    f.set("subject", subject.name());
    if let Some(p) = &point {
        f = tp::point_facts(&f, p, stance);
    }
    tp::conduct(&sheet, point.as_ref(), stance, &mut f);
    let mood = w.people[i].mood;
    let mut pen = scene::pen(w, &s, 7);
    let good = point.as_ref().is_some_and(tp::is_good);
    let line = match (&point, stance) {
        (Some(p), Some(st)) => point_open(&mut pen, p, st, good, &spk, &f),
        _ => String::new(),
    };
    let line = match (line.is_empty(), good) {
        (false, _) => line,
        (true, true) => "Sure, Coach.".into(),
        (true, false) => {
            let l = pen.line(&[&format!("talk.open.{}", subject.key())], &spk, &f);
            if l.is_empty() { open_fallback(subject, mood) } else { l }
        }
    };
    let name = w.people[i].name.clone();
    // Turning to what's really on his mind shows; turning away from it, a
    // sharp read sees his mind is elsewhere.
    let tell = match (&point, stance) {
        (Some(p), Some(st)) => tp::stance_tell(p, st, sharpness(w)),
        _ => None,
    }
    .or_else(|| match worry_on(w, i, &sheet) {
        Some((real, _)) => subject_tell(w, i, real == topic || real.subject() == subject && point.is_none()),
        None => read_tell(w, i, 0),
    });
    s.them(&name, line, tell);
    let used: Vec<Move> =
        Move::ALL.iter().enumerate().filter(|(k, _)| s.c & (1 << (2 + k)) != 0).map(|x| *x.1).collect();
    s.picks.clear();
    // He asks once a talk: back on a point you already talked through, he
    // waits for your next move.
    let talked = point.as_ref().is_some_and(|p| s.made.iter().any(|x| x.0 == p.kind.key()));
    if let Some(q) = point.as_ref().filter(|_| !talked).and_then(|p| tp::question(w, i, p)) {
        let said = pen.line(&[&format!("talk.point.ask.{}", q.key)], &spk, &f);
        pen.put(w);
        s.them(&name, if said.is_empty() { q.fallback.clone() } else { said }, None);
        s.c |= QUESTION;
        add_answers(&mut s, &q);
    } else {
        pen.put(w);
        add_picks(w, &mut s, i, topic, &used);
    }
    w.folk.scenes[k] = s;
    Ok(())
}

/// His opening on a point in his stance: its own part, else for good news
/// the part any good news takes (`talk.point.open.good.<stance>`).
fn point_open(pen: &mut super::voice::Pen, p: &Point, st: Stance, good: bool, spk: &Speaker, f: &Facts) -> String {
    let own = pen.line(&[&format!("talk.point.open.{}.{}", p.kind.key(), st.key())], spk, f);
    if own.is_empty() && good { pen.line(&[&format!("talk.point.open.good.{}", st.key())], spk, f) } else { own }
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

/// The moves on the table for a topic: a talking point's own (talk facts
/// lane L4, `talk_points::moves`), else its subject's; moves already used
/// drop off.
fn add_picks(w: &World, s: &mut Scene, i: usize, topic: Topic, used: &[Move]) {
    if let Topic::Point(k) = topic {
        let sheet = claims::player(w, i);
        if let Some(p) = sheet.get(k) {
            s.picks.extend(tp::picks(w, i, p, used, code_of));
            // Your word on his future (playtest bug 15): a promise backed by
            // a strength of his, or the three ways to tell him to go.
            s.picks.extend(super::future::picks(w, i, k));
            // An open promise that slipped: section 6's answers.
            if k == PointKind::PromiseOpen {
                slipped_picks(w, s, i);
            }
            s.pick(900, "End the talk", "End", "calm", "Ends it here.");
            return;
        }
    }
    add_talk_picks(w, s, i, topic.subject(), used)
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
    // Your words from the bank (`talk.you.<subject>.<move>`, lane L12) when
    // it has a version that fits; else the one written here.
    let mut yf = player_facts(w, i);
    yf.set("starter", if starter { "yes" } else { "no" });
    if let Some(m) = &mate {
        yf.set("mate", m.clone());
    }
    let me = Speaker::you(w);
    let look = super::voice::Look::new(w, ((p.player.id as u64) << 24) ^ 0x1221 ^ subject as u64);
    let mut add = |m: Move, t: Tone, label: String, hint: &str| {
        if !used.contains(&m) && !s.picks.iter().any(|p| p.code == code_of(m, t)) {
            let label = look
                .part(&format!("talk.you.{}.{}", subject.key(), m.key()), &me, &yf)
                .map_or(label, |x| format!("\"{}\"", super::voice::tidy(&x)));
            s.picks.push(Pick {
                label,
                mv: m.name().into(),
                tone: t.name().into(),
                hint: hint.into(),
                code: code_of(m, t),
                // A promise is logged: it can't be taken back.
                kind: if m == Move::Promise { PickKind::Final } else { PickKind::Plain },
                ..Pick::default()
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
            add(Move::Warn, Tone::Firm, "\"Next time it's not a talk. You'll sit.\"".into(), tp::HINT_WARN);
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

/// A starter's place, as proof on the field for praise: what he sees of
/// his place (the promises design, section 2), never your chart.
pub(crate) fn starters_place(w: &World, i: usize) -> bool {
    super::seen::starter(w, i)
}

/// How true praise rings to him, 0 (he knows he's coasting) to 1 (he has
/// earned every word). Both praise lines are about him and his work ("I've
/// noticed the work you're putting in"), so his work earns it: effort at
/// the facility (work ethic) and what he does when nobody is watching
/// (professionalism), with a starter's place as the proof on the field
/// (`starters_place`).
///
/// Playtest bug F2 (Cain 2026-11-12: "it hasn't lifted spirits once"): it
/// was a switch on last season's depth chart alone. Its other half, a
/// played rating above 62, read a rating no level's backups reach (1 of 117
/// on a college roster, none of 124 at a high school), so four players in
/// five heard every praise as empty, the hardest workers among them. The
/// sizes are proposed, not confirmed.
pub(crate) fn praise_truth(w: &World, i: usize) -> f32 {
    let me = &w.people[i].player.personality;
    let work = 0.7 * (me.work_ethic - 10.0) / 10.0 + 0.3 * me.traits.z(Trait::Professionalism);
    (0.5 + 0.9 * work + 0.25 * starters_place(w, i) as u8 as f64).clamp(0.0, 1.0) as f32
}

/// How a move lands on him (topic 5): (trust, mood). Fit with his
/// personality, what the move rests on (`Rest`: a fact on his mind, a fact,
/// nothing, or against the facts; talk facts lane L4) and the tone; noise
/// for the day he's having.
pub(crate) fn landing(w: &World, i: usize, m: Move, t: Tone, subject: Subject, rest: Rest, seed: u64) -> (f32, f32) {
    let p = &w.people[i];
    let tr = &p.player.personality.traits;
    let z = |x: Trait| tr.z(x) as f32;
    let comp = ((p.player.personality.competitiveness - 10.0) / 10.0) as f32;
    let shaky = ((10.0 - p.bg.stability) / 10.0) as f32;
    let needs_ear = p.mood < -0.15 || shaky > 0.2 || rest.hurting;
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
        // Praise lands by how true it rings (`praise_truth`): from empty
        // (he spots it: a little trust lost, a small lift all the same) to
        // earned in full, where a big ego takes the most from it.
        Move::Praise => {
            let truth = rest.truth.unwrap_or_else(|| praise_truth(w, i));
            (-0.03 + 0.07 * truth, 0.03 + (0.06 + 0.04 * z(Trait::Ego).max(0.0)) * truth)
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
    // What it rests on (section 5, "How much it counts"): a fact on his
    // mind counts more, nothing behind it less; against the facts, the lift
    // is lost and it costs trust.
    if rest.against {
        dt = dt.min(0.0);
        dm = dm.min(0.0);
    } else {
        dt *= rest.k;
        dm *= rest.k;
    }
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
    ((dt + tt) * ear + noise - rest.cost, (dm + tm) * ear + noise)
}

/// How an answer to his question lands: a straight answer or a promise as
/// its move, a softened one or a dodge by their own sizes, coloured by his
/// trust in you.
fn answer_landing(w: &World, i: usize, a: Answer, subject: Subject, rest: Rest, seed: u64) -> (f32, f32) {
    let tone = match a {
        Answer::Straight => Tone::Calm,
        _ => Tone::Warm,
    };
    if let Some(m) = tp::answer_move(a) {
        return landing(w, i, m, tone, subject, rest, seed);
    }
    let p = &w.people[i];
    let (bt, bm) = tp::answer_landing(w, i, a).unwrap_or((0.0, 0.0));
    let ear = 0.6 + 0.8 * p.trust;
    let noise = (super::roll_n(w, seed, p.player.id as u64) as f32) * 0.025;
    (bt * ear + noise, bm * ear + noise)
}

/// What a move on the talk's topic rests on, today.
fn rest_now(w: &World, i: usize, s: &Scene, m: Move) -> Rest {
    let sheet = claims::player(w, i);
    let topic = Topic::of(s.b);
    // He asked: what he asked about is on his mind.
    if s.c & JOB_QUESTION != 0 {
        let hurting = tp::rest(w, i, &sheet, topic, m, false).hurting;
        return Rest { k: claims::sheet::ON_MIND, against: false, cost: 0.0, hurting, truth: None };
    }
    tp::rest(w, i, &sheet, topic, m, s.c & 1 == 1)
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
    let id = s.a as u32;
    let i = super::person(w, id)?;
    let subject = Topic::of(s.b).subject();
    let seed = s.id as u64 ^ (s.step as u64) << 8;
    // An answer to his question, or a move.
    let (m, dt, dm) = match Answer::of(code) {
        Some(a) => {
            let m = tp::answer_move(a);
            let (dt, dm) = answer_landing(w, i, a, subject, rest_now(w, i, s, m.unwrap_or(Move::Straight)), seed);
            (m, dt, dm)
        }
        None => {
            let (m, t) = decode(code)?;
            let (dt, dm) = landing(w, i, m, t, subject, rest_now(w, i, s, m), seed);
            (Some(m), dt, dm)
        }
    };
    // The man who reads him is the one at his position coach's seat today.
    let adviser = position_coach_now(w, i);
    let rel = adviser.map_or(9.0, |c| w.coaches[c].ability().relationships) as f32;
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
        Some(Move::Promise) => " It's a promise: it goes in the ledger and he'll hold you to it.",
        Some(Move::Criticize) if dt + dm < 0.0 => " Criticism in private still travels in a locker room.",
        Some(Move::Warn) => " If he breaks the rule again, you'll have to follow through.",
        _ => "",
    };
    let sure = match q {
        x if x >= 0.75 => "They're sure",
        x if x >= 0.55 => "Fairly sure",
        x if x >= 0.4 => "A guess",
        _ => "A guess, and they don't know him well",
    };
    let who = adviser.map_or_else(
        || "Your staff".to_string(),
        |c| format!("{} ({})", super::coach_link(w, c), super::facts::coach_role(w, c)),
    );
    Some((format!("{what}{risk}"), sure.to_string(), who))
}

/// The man at his position coach's seat on your staff today. `college_staff`
/// is the record of past seasons: its last row holds last season's coach,
/// who may have been fired, hired away or retired since (and nobody at all
/// for a man who arrived this year), so the man who reads him comes from the
/// seat itself. With the seat empty, the coordinator on his side reads him
/// (a kicker or punter has none), then the head coach; never you. None
/// leaves it to "your staff": only a college staff keeps its position
/// coaches as people (an NFL club's and a school's are not).
pub(crate) fn position_coach_now(w: &World, i: usize) -> Option<usize> {
    let Team::College(j) = Team::seat(w)? else { return None };
    let prog = w.programs.get(j)?;
    let group = crate::world::coach::Group::of(w.people.get(i)?.player.position);
    let you = w.career.as_ref().map(|c| c.coach);
    let here = |c: Option<usize>| c.filter(|&c| c < w.coaches.len() && Some(c) != you);
    let coordinator = match group {
        crate::world::coach::Group::Specialists => None,
        g if g.offense() => prog.oc,
        _ => prog.dc,
    };
    here(prog.position_coaches[group as usize]).or_else(|| here(coordinator)).or_else(|| here(prog.head_coach))
}

/// Whether a one-on-one takes typed lines (live talks). False until lane 9
/// fills `one_on_one_say`; then the box shows there too.
pub const ONE_ON_ONE_TYPED: bool = false;

/// A typed line in a one-on-one (live talks lane 9, LT1): the option it
/// lands on, from the reading's point, move and tone, with the chip's own
/// rule that any tone of a move on the table can be taken (`same_move`).
/// The core leaves it empty: no option, so `scene::say` refuses the line
/// and the chips stay.
pub fn one_on_one_say(_w: &World, _s: &Scene, _heard: &super::scene::Heard) -> Option<u32> {
    None
}

/// A promise a typed line made to your player, or a claim about where he
/// stands heard as one (`claims::truth::standing_promise`): the seam lane
/// P1 fills (the promise design, agreed 2026-11-13: his belief now, the
/// terms of section 3, judged on his next 2 practice weeks and 2 games). It
/// logs nothing yet, so a typed line costs and keeps what its chip does.
pub fn typed_promise(w: &mut World, _scene: u32, him: u32, p: &super::scene::PromiseRead) {
    use super::promise::{By, Cond, Terms};
    let Some(i) = super::person(w, him) else { return };
    let rank = if is_starter(w, i) { 0 } else { w.people[i].last_role.map_or(PlayingTime::Backup as u8, |r| r as u8) };
    let kind = match p.kind.as_str() {
        "start" | "ahead_of" => PromiseKind::Start,
        "role" => PromiseKind::Snaps,
        "touches" => PromiseKind::Touches,
        "position" => PromiseKind::Position(w.people[i].player.position as u8),
        "place" => PromiseKind::Place(rank),
        "contract" => PromiseKind::Contract,
        "stay" => PromiseKind::Stay,
        _ => return,
    };
    // A claim about where he stands is judged on his next 2 practice weeks
    // and 2 games (section 11): from now, to the end of three weeks.
    let by = match p.by.as_str() {
        "midseason" => By::Midseason,
        "next_season" => By::NextSeason,
        "opener" | "next_game" | "this_season" | "" => By::Now,
        _ => By::Now,
    };
    let cond = match p.condition.as_str() {
        "stat" => Cond::Stat,
        "practice" => Cond::Practice,
        "conduct" => Cond::Conduct,
        "grades" => Cond::Grades,
        "body" => Cond::Weight,
        "health" => Cond::Health,
        _ => Cond::None,
    };
    let mut t = Terms { by, dated: !p.by.is_empty(), cond, ahead: p.against, ..Terms::default() };
    if cond == Cond::Stat
        && let Some(st) = super::promise::Stat::of(w.people[i].player.position)
    {
        t.stat = st;
        t.bar = super::promise::bars(w, i, kind, st, by).0;
        if let Some(n) = p.condition_words.split_whitespace().find_map(|x| x.parse::<f32>().ok()) {
            t.bar = n;
        }
    }
    promise(w, PromiseTo::Player(him), kind, 0);
    let k = w.folk.promises.len() - 1;
    set_terms(w, k, t);
    if p.claim.is_some() {
        // His next two practice weeks and two games.
        let d = w.today.add(16);
        let x = &mut w.folk.promises[k];
        if super::promise::over_games(x.kind) {
            x.due = d;
        }
    }
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
    if let Some(a) = Answer::of(code) {
        answer_question(w, s, i, a, picked);
        return;
    }
    // Your answer to his follow-up on a promise, or on an open promise
    // that slipped (the promises design, sections 5 and 6).
    if let Some(a) = PAct::of(code) {
        promise_act(w, s, i, a, picked);
        return;
    }
    // Your word on his future (playtest bug 15): he answers, and the talk
    // ends there.
    if let Some(word) = super::future::Word::of(code) {
        s.you(&scene::your_name(w), picked.label.clone());
        let (line, outcome) = super::future::answer(w, i, word);
        let sign = if word == super::future::Word::Backed { 1 } else { -1 };
        s.them(&name, line, read_tell(w, i, sign));
        s.outcome.extend(outcome);
        finish_talk(w, s, id);
        return;
    }
    let Some((m, t)) = decode(code) else {
        s.end();
        return;
    };
    let topic = Topic::of(s.b);
    let subject = topic.subject();
    s.you(&scene::your_name(w), picked.label.clone());
    let revealed = s.c & 1 == 1;
    let sheet = claims::player(w, i);
    let point = topic.point().and_then(|k| sheet.get(k)).cloned();
    let rest = tp::rest(w, i, &sheet, topic, m, revealed);
    let (dt, dm) = landing(w, i, m, t, subject, rest, s.id as u64 ^ (s.step as u64) << 8);
    s.f[0] += dt;
    s.f[1] += dm;
    let p = &mut w.people[i];
    p.trust = (p.trust + dt).clamp(0.0, 1.0);
    let spk = Speaker::player(w, i);
    let stance = Stance::from_bits(s.c);
    let mut f = player_facts(w, i);
    if let Some(p) = &point {
        f = tp::point_facts(&f, p, stance);
    }
    tp::conduct(&sheet, point.as_ref(), stance, &mut f);
    let outcome = if dt + dm > tp::LANDED {
        "good"
    } else if dt + dm < -0.03 {
        "bad"
    } else {
        "flat"
    };
    // A promise: he answers by how far he believes it (the promises design,
    // section 5, question 14): good believes, flat isn't sure, bad doubts;
    // a man who'd have it anyway says so.
    let pkind = (m == Move::Promise).then(|| move_promise_kind(w, i, topic, picked));
    let knows = pkind.is_some_and(|k| super::promise::knows(w, i, k));
    let outcome = match pkind {
        Some(k) => super::promise::band(super::promise::belief_own(w, i, k, &Default::default(), 0)),
        None => outcome,
    };
    f.set("went", outcome);
    // Praise for the same thing again: he says so (section 5, "Follow-ups
    // and memory").
    let again = m == Move::Praise
        && point
            .as_ref()
            .is_some_and(|p| w.folk.talked.row(id, p).is_some_and(|r| r.said == Move::Praise.key() && r.on != w.today));
    let mut pen = scene::pen(w, s, 2 + s.step as u64);
    // Ask: whether he opens up (topic 5): his trust and personality. On a
    // talking point he answers about it; asking what's really going on is
    // for a subject.
    if m == Move::Ask && !revealed && point.is_none() {
        // Asked about what you both know (his friend leaving), he opens up
        // more easily.
        let real = worry_on(w, i, &sheet).or_else(|| {
            // What he hasn't told you yet: the point that weighs most on him.
            tp::on_mind(&sheet).map(|p| (Topic::Point(p.kind), tp::worry_words(p)))
        });
        let (real, words) = real.unwrap_or_else(|| {
            let (s, x) = concern(w, i);
            (Topic::Subject(s), x)
        });
        if opens_up(w, i, s.id, subject == Subject::Friend) {
            s.c |= 1;
            f.set("worry", words.clone());
            let line = pen.line(&[&format!("talk.reveal.{}", real.subject().key())], &spk, &f);
            s.them(
                &name,
                if line.is_empty() { reveal_fallback(real.subject(), &words) } else { line },
                read_tell(w, i, 1),
            );
            s.outcome.push(format!("{} opened up: {words}.", name));
            super::note(w, "talk", player_link(w, i), format!("Told you {words}."), None);
            // What he says puts it on the table: home becomes the topic.
            if real.point().is_some_and(PointKind::is_life) || real.subject() == Subject::Life {
                super::life::tell(w, id, real.point().and_then(super::life::event_of));
            }
            let known = |k: PointKind| claims::player(w, i).known().any(|p| p.kind == k);
            s.b = match real {
                Topic::Point(k) if known(k) => real.code(),
                _ => Topic::Subject(real.subject()).code(),
            };
        } else {
            let line = pen.line(&["talk.deflect"], &spk, &f);
            s.them(&name, if line.is_empty() { "I'm good, Coach. Really.".into() } else { line }, read_tell(w, i, -1));
        }
    } else {
        // On a talking point, his answer in his stance; until the bank has
        // one, today's answer to the move.
        let own = match (&point, stance) {
            (Some(p), _) if again => {
                let own = pen.line(&[&format!("talk.point.again.{}", p.kind.key())], &spk, &f);
                if own.is_empty() { pen.line(&["talk.point.again"], &spk, &f) } else { own }
            }
            (Some(p), Some(st)) => {
                pen.line(&[&format!("talk.point.reply.{}.{}.{}", p.kind.key(), m.key(), st.key())], &spk, &f)
            }
            _ => String::new(),
        };
        let own = if own.is_empty() && again {
            "You told me that already, Coach. I appreciate it, though.".to_string()
        } else {
            own
        };
        // On his friend leaving he answers about what you told him (a
        // promise still gets its own "I'll hold you to it", listening and
        // asking their own); asked to lean on a teammate, he answers about
        // that.
        let line = if !own.is_empty() {
            own
        } else if subject == Subject::Friend && !matches!(m, Move::Promise | Move::Listen | Move::Ask) {
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
        let line = if line.is_empty() && knows { pen.line(&["talk.promise.reply.knows"], &spk, &f) } else { line };
        let line =
            if line.is_empty() && knows { "I'm already doing that, Coach. But I hear you.".to_string() } else { line };
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
    if rest.against {
        s.outcome.push(format!("{name} knows he did nothing to deserve that."));
    }
    // What you said on a point, kept for the talk's end (`Talked`).
    if let Some(p) = &point {
        made(s, p.kind, m.key(), dt + dm);
    }
    // What the move does beyond the talk.
    match (m, topic) {
        (Move::Promise, _) => {
            let kind = pkind.unwrap_or(PromiseKind::Start);
            promise(w, PromiseTo::Player(id), kind, 0);
            s.outcome.push(format!("You promised {} {}.", name, kind.words()));
        }
        (Move::Warn, _) => {
            w.folk.rules.warned.push((id, w.today));
            w.folk.rules.warned.retain(|x| x.1.days_to(w.today) < 120);
        }
        (Move::Hand, Topic::Subject(Subject::Team)) => {
            if !w.folk.rules.council.contains(&id) {
                w.folk.rules.council.push(id);
            }
            s.outcome.push(format!("{name} joins your leadership council."));
        }
        (Move::Hand, Topic::Subject(Subject::Friend)) => {
            if let Some(j) = look_out(w, i) {
                let (a, b) = (w.people[j].player.id, id);
                super::ties::bond(w, a, b, 10);
                s.outcome.push(format!("You asked {} to look out for {name}.", player_link(w, j)));
            }
        }
        // Getting him help after trouble (section 5): less risk of more.
        (Move::Hand, Topic::Point(k)) if tp::lean_is_trouble(k) => {
            if tp::help(w, id) {
                s.outcome.push(format!("You got {name} help: someone to talk to, set up by your staff."));
            }
        }
        (Move::Hand, Topic::Point(PointKind::Grades | PointKind::FailedTerm)) => {
            s.outcome.push(format!("{name} gets a tutor."));
        }
        (Move::Hand, Topic::Subject(Subject::Life)) | (Move::Hand, Topic::Point(_)) if subject == Subject::Life => {
            s.outcome.push(format!("{name} gets time away; his position coach covers his work."));
            super::life::grant_leave(w, id);
        }
        _ => {}
    }
    // The meeting before he enters the portal (the promises design, section
    // 8): a playing-time promise moves his lean by what it adds for him
    // times his belief, at most halving it, in place of the flat lift.
    let held = pkind.filter(|&k| super::promise::playing_time(k) && asked_to_leave(w, id)).and_then(|_| {
        let k = super::promise::fresh(w, id)?;
        Some(super::promise::portal_pull(w, i, &w.folk.promises[k]))
    });
    if let Some(pull) = held
        && w.people[i].stage == Stage::College
        && w.people[i].cal.lean > 0.0
    {
        let today = w.today;
        let p = &mut w.people[i];
        p.cal.lean *= 1.0 - pull;
        p.cal.lean_events.push((today, "his head coach promised him playing time", 1.0 - pull));
    } else if w.people[i].stage == Stage::College && w.people[i].cal.lean > 0.0 && (dt + dm).abs() > 0.03 {
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
    s.c |= used_bit(m);
    // He isn't sure, or doubts: one follow-up about his doubt (section 5),
    // using one of the talk's exchanges.
    if pkind.is_some() && s.step < 3 && follow_up(w, s, i) {
        return;
    }
    let all_used: Vec<Move> =
        Move::ALL.iter().enumerate().filter(|(k, _)| s.c & (1 << (2 + k)) != 0).map(|x| *x.1).collect();
    if s.step >= 3 || matches!(m, Move::Criticize) && outcome == "bad" {
        finish_talk(w, s, id);
    } else {
        add_picks(w, s, i, Topic::of(s.b), &all_used);
    }
}

/// The promise a Promise move makes on a topic: a point's own kind, else
/// the subject's (a start for a man who doesn't see himself starting).
fn move_promise_kind(w: &World, i: usize, topic: Topic, picked: &Pick) -> PromiseKind {
    match topic {
        Topic::Point(k) => tp::promise_kind(w, i, k),
        Topic::Subject(sub) => {
            let starter = is_starter(w, i);
            let rank = if starter { 0 } else { w.people[i].last_role.map_or(PlayingTime::Backup as u8, |r| r as u8) };
            match sub {
                Subject::Money => PromiseKind::Contract,
                Subject::Future if picked.label.contains("not going anywhere") => PromiseKind::Stay,
                Subject::Future => PromiseKind::Start,
                Subject::Behavior | Subject::Life | Subject::Team | Subject::Friend => PromiseKind::Place(rank),
                _ if starter => PromiseKind::Snaps,
                _ => PromiseKind::Start,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Promises in the talk (the promises design, sections 5 and 6; lane P1).

/// A follow-up on a promise has been asked in this talk (`Scene.c`).
const FOLLOWED: i64 = 1 << 30;

/// His follow-up, the one his doubt is about (section 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    /// "Ahead of who?"
    Who,
    /// "What do I have to do?"
    What,
    /// "By when?"
    When,
    /// "What happens when he's back?"
    Threat,
    /// "You told him the same."
    Friend,
    /// "Will you put that in writing?" (NFL)
    Writing,
}

impl Ask {
    fn key(self) -> &'static str {
        match self {
            Ask::Who => "who",
            Ask::What => "what",
            Ask::When => "when",
            Ask::Threat => "threat",
            Ask::Friend => "friend",
            Ask::Writing => "writing",
        }
    }
}

/// Your answers to a follow-up and to an open promise that slipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PAct {
    /// Name the man he plays ahead of.
    Name,
    /// Tell him straight what you can and can't promise.
    Straight,
    /// Take it back.
    TakeBack,
    /// A stat bar a starter makes on the snaps he'll play.
    BarFair,
    /// A bar well above that.
    BarHigh,
    Practice,
    Conduct,
    Grades,
    Weight,
    Health,
    Week4,
    Midseason,
    NextSeason,
    /// He keeps it when the hurt man is back.
    KeepsIt,
    /// You own the one you broke to his friend.
    OwnIt,
    /// "That was different."
    Different,
    Writing,
    WordEnough,
    /// Section 6: deliver (you start him).
    Deliver,
    /// Section 6: the real reason.
    Real,
    /// Section 6: a new date.
    NewDate,
    /// Section 6: change the condition.
    NewCond,
    /// Section 6: help him find a place.
    Help,
    /// Section 6: dodge.
    Dodge,
}

const PACT_FIRST: u32 = 1100;

impl PAct {
    const ALL: [PAct; 24] = [
        PAct::Name,
        PAct::Straight,
        PAct::TakeBack,
        PAct::BarFair,
        PAct::BarHigh,
        PAct::Practice,
        PAct::Conduct,
        PAct::Grades,
        PAct::Weight,
        PAct::Health,
        PAct::Week4,
        PAct::Midseason,
        PAct::NextSeason,
        PAct::KeepsIt,
        PAct::OwnIt,
        PAct::Different,
        PAct::Writing,
        PAct::WordEnough,
        PAct::Deliver,
        PAct::Real,
        PAct::NewDate,
        PAct::NewCond,
        PAct::Help,
        PAct::Dodge,
    ];
    pub fn code(self) -> u32 {
        PACT_FIRST + PAct::ALL.iter().position(|x| *x == self).unwrap_or(0) as u32
    }
    pub fn of(code: u32) -> Option<PAct> {
        code.checked_sub(PACT_FIRST).and_then(|k| PAct::ALL.get(k as usize)).copied()
    }
    fn key(self) -> &'static str {
        match self {
            PAct::Name => "name",
            PAct::Straight => "straight",
            PAct::TakeBack => "take_back",
            PAct::BarFair | PAct::BarHigh => "stat",
            PAct::Practice => "practice",
            PAct::Conduct => "conduct",
            PAct::Grades => "grades",
            PAct::Weight => "weight",
            PAct::Health => "health",
            PAct::Week4 | PAct::Midseason | PAct::NextSeason => "when",
            PAct::KeepsIt => "keeps",
            PAct::OwnIt => "own",
            PAct::Different => "different",
            PAct::Writing => "writing",
            PAct::WordEnough => "word",
            PAct::Deliver => "deliver",
            PAct::Real => "straight",
            PAct::NewDate => "new_date",
            PAct::NewCond => "new_condition",
            PAct::Help => "help",
            PAct::Dodge => "dodge",
        }
    }
}

/// The man he has to pass: the one at his spot with the ones who played
/// the most snaps in your last game.
fn man_ahead(w: &World, i: usize) -> Option<u32> {
    let ahead = super::seen::ahead(w, i);
    let team = Team::seat(w)?;
    let last =
        w.seat_games.iter().rev().filter(|g| g.date <= w.today).find_map(|g| claims::side_of(g, team).map(|s| (g, s)));
    let snaps = |id: u32| last.and_then(|(g, s)| g.stats.players[s].get(&id)).map_or(0, |l| l.snaps_off + l.snaps_def);
    ahead.into_iter().max_by_key(|&id| (snaps(id), std::cmp::Reverse(id)))
}

/// A man at his spot out hurt now who had the job (the threat when he's
/// back).
fn hurt_ahead(w: &World, i: usize) -> Option<u32> {
    let p = &w.people[i];
    let (id, pos) = (p.player.id, p.player.position);
    let team = Team::seat(w)?;
    team.roster(w).into_iter().find_map(|j| {
        let q = &w.people[j];
        (q.player.id != id
            && q.player.position == pos
            && super::seen::hurt_on(w, q.player.id, w.today)
            && q.last_role == Some(PlayingTime::Starter))
        .then_some(q.player.id)
    })
}

/// A promise you broke to a close friend of his in the last two years.
fn broke_to_friend(w: &World, id: u32) -> Option<u32> {
    let friends = super::ties::friends(w, id);
    w.folk.promises.iter().find_map(|p| match p.to {
        PromiseTo::Player(o) if friends.contains(&o) && p.kept == Some(false) && p.due.days_to(w.today) <= 730 => {
            Some(o)
        }
        _ => None,
    })
}

fn which_ask(w: &World, i: usize, p: &Promise) -> Option<Ask> {
    let id = w.people[i].player.id;
    if !super::promise::playing_time(p.kind) {
        return None;
    }
    let start = matches!(p.kind, PromiseKind::Start | PromiseKind::Place(0));
    let t = &p.terms;
    if broke_to_friend(w, id).is_some() {
        return Some(Ask::Friend);
    }
    if w.people[i].stage == Stage::Pro && (p.belief as f64) < super::promise::UNSURE && !t.written {
        return Some(Ask::Writing);
    }
    if start && hurt_ahead(w, i).is_some() && t.ahead.is_none() {
        return Some(Ask::Threat);
    }
    if start && t.ahead.is_none() && man_ahead(w, i).is_some() {
        return Some(Ask::Who);
    }
    if t.cond == super::promise::Cond::None && super::promise::likely_own(w, i, p.kind, t) < 0.7 {
        return Some(Ask::What);
    }
    if !t.dated {
        return Some(Ask::When);
    }
    None
}

fn first_of(w: &World, id: u32) -> String {
    super::person(w, id).map_or_else(|| "him".into(), |j| first_name(&w.people[j].name))
}

/// He isn't sure or doubts the promise you just made: he asks one
/// follow-up about his doubt, and the talk's picks become your answers.
fn follow_up(w: &mut World, s: &mut Scene, i: usize) -> bool {
    let id = w.people[i].player.id;
    if s.c & FOLLOWED != 0 {
        return false;
    }
    let Some(k) = super::promise::fresh(w, id) else { return false };
    let p = w.folk.promises[k].clone();
    if p.made != w.today || p.belief as f64 >= super::promise::BELIEVES || super::promise::knows(w, i, p.kind) {
        return false;
    }
    let Some(ask) = which_ask(w, i, &p) else { return false };
    s.c |= FOLLOWED;
    let name = w.people[i].name.clone();
    let spk = Speaker::player(w, i);
    let mut f = player_facts(w, i);
    let ahead = man_ahead(w, i);
    if let Some(a) = ahead {
        f.set("ahead", first_of(w, a));
    }
    let hurt = hurt_ahead(w, i);
    if let Some(h) = hurt {
        f.set("back", first_of(w, h));
    }
    let friend = broke_to_friend(w, id);
    if let Some(o) = friend {
        f.set("friend", first_of(w, o));
    }
    let mut pen = scene::pen(w, s, 0x9A0 + s.step as u64);
    let line = pen.line(&[&format!("talk.promise.ask.{}", ask.key())], &spk, &f);
    pen.put(w);
    let line = if line.is_empty() {
        match ask {
            Ask::Who => format!(
                "Ahead of who, Coach? {} is getting the reps with the ones.",
                f.get("ahead").unwrap_or("Somebody")
            ),
            Ask::What => "What do I have to do? Tell me and I'll do it.".to_string(),
            Ask::When => "By when, Coach?".to_string(),
            Ask::Threat => format!("And what happens when {} is back?", f.get("back").unwrap_or("he")),
            Ask::Friend => {
                format!("You told {} the same thing. Look how that went.", f.get("friend").unwrap_or("my boy"))
            }
            Ask::Writing => "With respect, Coach: will you put that in writing?".to_string(),
        }
    } else {
        line
    };
    s.them(&name, line, read_tell(w, i, -1));
    s.outcome.push(format!("{name} isn't sure he believes it."));
    s.picks.clear();
    let mut add = |a: PAct, label: String, tone: &str, hint: &str| {
        s.picks.push(Pick {
            label,
            mv: "Answer".into(),
            tone: tone.into(),
            hint: hint.into(),
            code: a.code(),
            kind: PickKind::Final,
            ..Pick::default()
        });
    };
    let them = |x: Option<u32>| x.map_or_else(|| "him".to_string(), |a| first_of(w, a));
    match ask {
        Ask::Who | Ask::Threat => {
            let x = if ask == Ask::Who { ahead } else { hurt };
            add(
                if ask == Ask::Who { PAct::Name } else { PAct::KeepsIt },
                if ask == Ask::Who {
                    format!("\"You'll play ahead of {}.\"", them(x))
                } else {
                    format!("\"When {} is back, you're still ahead of him.\"", them(x))
                },
                "firm",
                "Names the man: more real to him, and that man may hear of it.",
            );
            add(
                PAct::Straight,
                format!("\"I can't promise you'll pass {}. I can promise you'll get your shot.\"", them(x)),
                "calm",
                "Takes the promise back honestly: no pull now, and he believes your next one more.",
            );
        }
        Ask::What => {
            let pos = w.people[i].player.position;
            if let Some(st) = super::promise::Stat::of(pos) {
                let (fair, high) = super::promise::bars(w, i, p.kind, st, p.terms.by);
                add(
                    PAct::BarFair,
                    format!("\"Give me {}.\"", st.words(fair)),
                    "firm",
                    "A bar a starter makes on those snaps. He likes earning it.",
                );
                add(
                    PAct::BarHigh,
                    format!("\"Give me {}.\"", st.words(high)),
                    "fiery",
                    "Well above a starter's line on those snaps: he may hear a setup.",
                );
            }
            add(
                PAct::Practice,
                "\"Win it in practice.\"".into(),
                "firm",
                "Your staff judges it: it rests on his belief you judge fairly.",
            );
            add(PAct::Conduct, "\"Keep your nose clean. No rule broken.\"".into(), "calm", "Plain to both of you.");
            if matches!(w.people[i].stage, Stage::College | Stage::HighSchool) {
                add(PAct::Grades, "\"Stay eligible.\"".into(), "calm", "Plain to both of you.");
            }
            if super::seen::hurt_on(w, id, w.today) {
                add(PAct::Health, "\"Get healthy first.\"".into(), "warm", "It waits until he is cleared.");
            }
        }
        Ask::When => {
            add(PAct::Week4, "\"By week 4.\"".into(), "firm", "A date makes it real.");
            add(PAct::Midseason, "\"By midseason.\"".into(), "calm", "A date makes it real.");
            add(PAct::NextSeason, "\"Next season.\"".into(), "calm", "A long promise weighs your job security.");
        }
        Ask::Friend => {
            add(
                PAct::OwnIt,
                format!("\"I let {} down. I won't do it to you.\"", them(friend)),
                "warm",
                "Owning it helps a little.",
            );
            add(PAct::Different, "\"That was different.\"".into(), "firm", "He hears an excuse.");
        }
        Ask::Writing => {
            add(
                PAct::Writing,
                "\"Have your agent call. We'll put a snap clause in.\"".into(),
                "calm",
                "A playing time incentive clause: he believes it.",
            );
            add(PAct::WordEnough, "\"My word is enough.\"".into(), "firm", "A wary man trusts it less.");
        }
    }
    add(PAct::TakeBack, "\"Forget I said it.\"".into(), "calm", "Takes it back. It costs a little trust.");
    s.pick(900, "End the talk", "End", "calm", "Ends it here.");
    true
}

/// The open promise to him that slipped (behind, owed or missed).
fn slipped(w: &World, id: u32) -> Option<usize> {
    use super::promise::State;
    (0..w.folk.promises.len()).rev().find(|&k| {
        let p = &w.folk.promises[k];
        p.to == PromiseTo::Player(id)
            && p.kept.is_none()
            && matches!(State::of(p.state), State::Behind | State::Owed | State::Missed)
    })
}

/// Section 6's answers on an open promise that slipped.
fn slipped_picks(w: &World, s: &mut Scene, i: usize) {
    use super::promise::{Cond, State};
    let id = w.people[i].player.id;
    let Some(k) = slipped(w, id) else { return };
    let p = &w.folk.promises[k];
    let st = State::of(p.state);
    let mut add = |a: PAct, label: &str, tone: &str, hint: &str| {
        if !s.picks.iter().any(|x| x.code == a.code()) {
            s.picks.push(Pick {
                label: label.into(),
                mv: "Answer".into(),
                tone: tone.into(),
                hint: hint.into(),
                code: a.code(),
                kind: PickKind::Final,
                ..Pick::default()
            });
        }
    };
    if st != State::Missed {
        add(
            PAct::Deliver,
            "\"You start this week.\"",
            "firm",
            "You move him up your chart: he sees it at practice and on game day.",
        );
    }
    add(
        PAct::Real,
        "\"Here's the real reason.\"",
        "calm",
        "The truth about where it stands: it ends the promise, half the sting.",
    );
    add(PAct::NewDate, "\"Give me until midseason.\"", "warm", "A new date: his belief drops by a third.");
    if p.terms.cond != Cond::None || st == State::Owed {
        add(PAct::NewCond, "\"Win it in practice first.\"", "firm", "Once he met it, he hears the goalposts moved.");
    }
    if w.people[i].stage == Stage::College {
        add(
            PAct::Help,
            "\"If you want out, I'll help you find a place.\"",
            "warm",
            "Ends it. His trust and your room's word hold.",
        );
    }
    add(PAct::Dodge, "\"Let's see how the week goes.\"", "calm", "He notices a dodge.");
}

/// Your answer to a follow-up or to an open promise that slipped: it
/// reshapes the promise and his belief through the part it touches.
fn promise_act(w: &mut World, s: &mut Scene, i: usize, a: PAct, picked: &Pick) {
    use super::promise::{By, Cond};
    let id = w.people[i].player.id;
    let name = w.people[i].name.clone();
    s.you(&scene::your_name(w), picked.label.clone());
    let slip = matches!(a, PAct::Deliver | PAct::Real | PAct::NewDate | PAct::NewCond | PAct::Help | PAct::Dodge);
    let k = if slip { slipped(w, id) } else { super::promise::fresh(w, id) };
    let Some(k) = k else {
        add_picks(w, s, i, Topic::of(s.b), &[]);
        return;
    };
    let before = w.folk.promises[k].clone();
    let old_pull = super::promise::portal_pull(w, i, &before);
    let ego = w.people[i].player.personality.traits.z(Trait::Ego).clamp(0.0, 1.5) as f32;
    let prof = w.people[i].player.personality.traits.z(Trait::Professionalism) as f32;
    let mut t = before.terms;
    let mut hear_key = String::new();
    let mut gone = false;
    match a {
        PAct::Name | PAct::KeepsIt => {
            t.ahead = if a == PAct::Name { man_ahead(w, i) } else { hurt_ahead(w, i) };
            t.dated = t.dated || a == PAct::KeepsIt;
        }
        PAct::Straight => {
            // No pull now, and he believes your next promise more.
            gone = true;
            let q = &mut w.people[i];
            q.trust = (q.trust + 0.02 + 0.02 * prof.max(0.0)).clamp(0.0, 1.0);
            w.folk.mine.told_straight.retain(|x| x.1.days_to(w.today) < 120 && x.0 != id);
            w.folk.mine.told_straight.push((id, w.today));
            hear_key = "talk.promise.hear.sure".into();
        }
        PAct::TakeBack => {
            gone = true;
            let q = &mut w.people[i];
            q.trust = (q.trust - 0.05).max(0.0);
        }
        PAct::BarFair | PAct::BarHigh => {
            let pos = w.people[i].player.position;
            if let Some(st) = super::promise::Stat::of(pos) {
                let (fair, high) = super::promise::bars(w, i, before.kind, st, t.by);
                t.cond = Cond::Stat;
                t.stat = st;
                t.bar = if a == PAct::BarFair { fair } else { high };
            }
        }
        PAct::Practice => t.cond = Cond::Practice,
        PAct::Conduct => t.cond = Cond::Conduct,
        PAct::Grades => t.cond = Cond::Grades,
        PAct::Weight => t.cond = Cond::Weight,
        PAct::Health => t.cond = Cond::Health,
        PAct::Week4 | PAct::Midseason | PAct::NextSeason => {
            t.by = match a {
                PAct::Week4 => By::Week4,
                PAct::Midseason => By::Midseason,
                _ => By::NextSeason,
            };
            t.dated = true;
        }
        PAct::OwnIt => {
            let b = &mut w.folk.promises[k].belief;
            *b = (*b + 0.03).min(0.97);
        }
        PAct::Different => {
            let q = &mut w.people[i];
            q.trust = (q.trust - 0.02).max(0.0);
            let b = &mut w.folk.promises[k].belief;
            *b = (*b - 0.05).max(0.05);
        }
        PAct::Writing => t.written = true,
        PAct::WordEnough => {
            let b = &mut w.folk.promises[k].belief;
            *b = (*b - 0.1).max(0.05);
        }
        PAct::Deliver => {
            // You start him: he sees it at practice and on game day.
            let pos = w.people[i].player.position;
            let mut order: Vec<u32> = w.seat_depth.iter().find(|x| x.0 == pos).map(|x| x.1.clone()).unwrap_or_default();
            order.retain(|&x| x != id);
            order.insert(0, id);
            w.set_seat_depth(pos, order);
            t.by = By::Now;
            let b = &mut w.folk.promises[k].belief;
            *b = (*b + 0.1).min(0.97);
            s.outcome.push(format!("{name} goes to the top of your depth chart at his spot."));
        }
        PAct::Real => {
            // The real reason, a true fact: it ends here, judged on what
            // he has seen, at half the sting.
            gone = true;
            let r = super::promise::judge(w, &before);
            let share = match r {
                Some(super::promise::Verdict::Broken(x)) => x,
                _ => 0.0,
            };
            let q = &mut w.people[i];
            q.trust = (q.trust + 0.03 * prof.max(0.0)).clamp(0.0, 1.0);
            if share > 0.0 {
                let mut p = before.clone();
                p.public = false;
                let cost = 0.5 * super::promise::broken_scale(before.belief, share);
                settle_promise(w, &p, false, cost);
            }
        }
        PAct::NewDate => {
            t.by = match t.by {
                By::Now | By::Week4 => By::Midseason,
                _ => By::NextSeason,
            };
            t.dated = true;
            let b = &mut w.folk.promises[k].belief;
            *b *= super::promise::NEW_DATE as f32;
        }
        PAct::NewCond => {
            if super::promise::State::of(before.state) == super::promise::State::Owed {
                let q = &mut w.people[i];
                q.trust = (q.trust - super::promise::GOALPOSTS - 0.04 * ego).max(0.0);
                s.outcome.push(format!("{name} hears the goalposts moved."));
            }
            t.cond = Cond::Practice;
        }
        PAct::Help => {
            // You help him find a place: the promise ends, his trust and
            // your room's word hold, and he leans out.
            gone = true;
            let today = w.today;
            let q = &mut w.people[i];
            q.cal.lean = (q.cal.lean.max(0.05) * 1.5).min(1.0);
            q.cal.lean_events.push((today, "his head coach offered to help him find a place", 1.5));
        }
        PAct::Dodge => {
            let q = &mut w.people[i];
            q.trust = (q.trust - 0.04).max(0.0);
            let b = &mut w.folk.promises[k].belief;
            *b = (*b - 0.1).max(0.05);
        }
    }
    let belief = if gone {
        w.folk.promises.remove(k);
        0.0
    } else {
        let keep_belief = w.folk.promises[k].belief;
        let reshaped = t != before.terms;
        let b = if reshaped { set_terms(w, k, t) as f32 } else { keep_belief };
        // A reshaped promise is heard fresh, then what this talk already
        // did to his belief (owning it, a dodge) still counts.
        if reshaped {
            let drift = keep_belief - before.belief;
            let x = &mut w.folk.promises[k];
            x.belief = (b + drift).clamp(0.05, 0.97);
            x.said = x.belief;
        }
        if matches!(a, PAct::NewDate) {
            let x = &mut w.folk.promises[k];
            x.belief = (x.belief.min(before.belief) * super::promise::NEW_DATE as f32).max(0.05);
        }
        w.folk.promises[k].belief
    };
    // The portal meeting: what the promise holds now replaces what it held.
    if asked_to_leave(w, id) && w.people[i].stage == Stage::College && super::promise::playing_time(before.kind) {
        let now = if gone { 0.0 } else { super::promise::portal_pull(w, i, &w.folk.promises[k]) };
        let f = ((1.0 - now) / (1.0 - old_pull).max(0.5)).clamp(0.5, 2.0);
        if (f - 1.0).abs() > 1e-6 {
            let today = w.today;
            let q = &mut w.people[i];
            q.cal.lean *= f;
            q.cal.lean_events.push((today, "his head coach's promise, reshaped", f));
        }
    }
    // His answer: by how he heard a condition, else by belief.
    let fair = if gone { 0 } else { w.folk.promises[k].terms.fair };
    if hear_key.is_empty() {
        hear_key = match (a, fair) {
            (PAct::BarFair | PAct::BarHigh, -1) => "talk.promise.hear.unfair".into(),
            (PAct::BarFair | PAct::BarHigh | PAct::Practice, _) => "talk.promise.hear.earn".into(),
            (PAct::Conduct | PAct::Grades | PAct::Weight | PAct::Health, _) => "talk.promise.hear.fair".into(),
            _ => format!("talk.reply.promise.{}", super::promise::band(belief as f64)),
        };
    }
    let spk = Speaker::player(w, i);
    let mut f = player_facts(w, i);
    if !gone {
        let p = &w.folk.promises[k];
        f.set("snaps", format!("{}", super::promise::snaps_under(w, i, p.kind, &p.terms).round() as i32));
        if let Some(a) = p.terms.ahead {
            f.set("ahead", first_of(w, a));
        }
        f.set("when", p.terms.by.words());
    }
    let mut pen = scene::pen(w, s, 0x9B0 + s.step as u64);
    let line = pen.line(&[&hear_key, &format!("talk.promise.answer.{}", a.key())], &spk, &f);
    pen.put(w);
    let line = if line.is_empty() {
        match (a, fair, super::promise::band(belief as f64)) {
            (PAct::BarFair | PAct::BarHigh, -1, _) => {
                format!("On {} snaps a game? Come on, Coach. You're setting me up.", f.get("snaps").unwrap_or("these"))
            }
            (PAct::BarFair | PAct::BarHigh | PAct::Practice, _, _) => "Bet. I'll earn it.".into(),
            (PAct::Straight | PAct::Real, _, _) => "I appreciate you being straight with me.".into(),
            (PAct::TakeBack, _, _) => "Right. I figured.".into(),
            (PAct::Help, _, _) => "Thanks, Coach. I think it's time.".into(),
            (PAct::Dodge, _, _) => "That's not really an answer, Coach.".into(),
            (_, _, "good") => "Alright. I'm going to hold you to that.".into(),
            (_, _, "flat") => "We'll see, Coach.".into(),
            _ => "I've heard that before.".into(),
        }
    } else {
        line
    };
    let sign = if gone {
        0
    } else if belief as f64 >= super::promise::BELIEVES {
        1
    } else {
        -1
    };
    s.them(&name, line, read_tell(w, i, sign));
    if !gone {
        let t = w.folk.promises[k].terms;
        let mut said: Vec<String> = Vec::new();
        if let Some(a) = t.ahead {
            said.push(format!("ahead of {}", first_of(w, a)));
        }
        if t.dated {
            said.push(t.by.words().to_string());
        }
        if t.cond == Cond::Stat {
            said.push(format!("as long as he gives you {}", t.stat.words(t.bar)));
        } else if t.cond != Cond::None {
            said.push(format!("on a condition: {}", picked.label.trim_matches('"')));
        }
        if !said.is_empty() {
            s.outcome.push(format!("Your promise to {name}: {}.", said.join(", ")));
        }
        // His own deadline (section 3): a doubter with fight in him sets one.
        own_deadline(w, s, i, k);
    } else {
        s.outcome.push(format!("You took back your promise to {name}."));
    }
    let used: Vec<Move> =
        Move::ALL.iter().enumerate().filter(|(k, _)| s.c & (1 << (2 + k)) != 0).map(|x| *x.1).collect();
    if s.step >= 3 {
        finish_talk(w, s, id);
    } else {
        add_picks(w, s, i, Topic::of(s.b), &used);
    }
}

/// A doubter with fight in him sets his own deadline: "If I'm not starting
/// by midseason, I'm in the portal when the window opens." Logged like
/// yours; on the day, if you missed it, his lean jumps (`promise::week`).
fn own_deadline(w: &mut World, s: &mut Scene, i: usize, k: usize) {
    let p = &w.folk.promises[k];
    if p.his_by.is_some() || w.people[i].stage != Stage::College || p.belief as f64 >= 0.5 {
        return;
    }
    let me = &w.people[i].player.personality;
    let fight = ((me.competitiveness - 10.0) / 10.0).max(me.traits.z(Trait::Ego));
    if fight < 0.5 {
        return;
    }
    let by = super::promise::opens(w, w.today, super::promise::By::Midseason);
    w.folk.promises[k].his_by = Some(by);
    let name = w.people[i].name.clone();
    let spk = Speaker::player(w, i);
    let mut f = player_facts(w, i);
    f.set("when", "midseason");
    let mut pen = scene::pen(w, s, 0x9C0 + s.step as u64);
    let line = pen.line(&["talk.promise.deadline"], &spk, &f);
    pen.put(w);
    let line = if line.is_empty() {
        "If I'm not starting by midseason, I'm in the portal when the window opens.".to_string()
    } else {
        line
    };
    s.them(&name, line, None);
    s.outcome.push(format!("{name} set his own deadline: {}.", by.pretty()));
}

/// He comes to you about a promise that slipped (section 6): a one-on-one
/// on your calendar about it.
pub(crate) fn promise_asks(w: &mut World, i: usize, d: Date) {
    let mut s = Scene::new(Kind::OneOnOne, d, format!("{} wants to talk about your promise", w.people[i].name));
    s.a = w.people[i].player.id as u64;
    s.b = Topic::Point(PointKind::PromiseOpen).code();
    s.place = "Your office".into();
    s.stops = false;
    scene::add(w, s);
}

/// A one-on-one with him is already on your calendar.
pub(crate) fn has_open_talk(w: &World, id: u32) -> bool {
    w.folk.scenes.iter().any(|s| !s.done && s.kind == Kind::OneOnOne && s.a == id as u64)
}

/// Keeps what you did with a point in this talk (a point counts once a
/// talk): what you did last and what it did, summed.
fn made(s: &mut Scene, k: PointKind, said: &str, d: f32) {
    match s.made.iter_mut().find(|x| x.0 == k.key()) {
        Some(x) => {
            x.1 = said.to_string();
            x.2 += d;
        }
        None => s.made.push((k.key().to_string(), said.to_string(), d)),
    }
}

/// Your answer to his question (section 5, "He asks you"): straight with
/// the real fact, softened, a promise, or a dodge.
fn answer_question(w: &mut World, s: &mut Scene, i: usize, a: Answer, picked: &Pick) {
    let id = s.a as u32;
    let name = w.people[i].name.clone();
    let job = s.c & JOB_QUESTION != 0;
    let topic = Topic::of(s.b);
    let subject = topic.subject();
    s.you(&scene::your_name(w), picked.label.clone());
    let m = tp::answer_move(a);
    let rest = rest_now(w, i, s, m.unwrap_or(Move::Straight));
    s.c &= !(QUESTION | JOB_QUESTION);
    // Answering straight or with a promise uses that move for this talk: the
    // point doesn't offer it again (a promise lands once).
    if let Some(m) = m {
        s.c |= used_bit(m);
    }
    let (dt, dm) = answer_landing(w, i, a, subject, rest, s.id as u64 ^ (s.step as u64) << 8);
    s.f[0] += dt;
    s.f[1] += dm;
    let p = &mut w.people[i];
    p.trust = (p.trust + dt).clamp(0.0, 1.0);
    let sheet = claims::player(w, i);
    let point = if job { None } else { topic.point().and_then(|k| sheet.get(k)).cloned() };
    let key = if job { PointKind::YourJob } else { topic.point().unwrap_or(PointKind::YourJob) };
    let outcome = if dt + dm > tp::LANDED {
        "good"
    } else if dt + dm < -0.03 {
        "bad"
    } else {
        "flat"
    };
    let spk = Speaker::player(w, i);
    let mut f = player_facts(w, i);
    if let Some(p) = &point {
        f = tp::point_facts(&f, p, Stance::from_bits(s.c));
    }
    tp::conduct(&sheet, point.as_ref(), Stance::from_bits(s.c), &mut f);
    f.set("went", outcome);
    let mut pen = scene::pen(w, s, 0x50 + s.step as u64);
    let line = pen.line(&[&format!("talk.point.hear.{}.{}.{outcome}", key.key(), a.key())], &spk, &f);
    let line = match (line.is_empty(), m) {
        (false, _) => line,
        (true, Some(m)) => pen.line(&[&format!("talk.reply.{}.{outcome}", m.key())], &spk, &f),
        (true, None) => String::new(),
    };
    pen.put(w);
    let line = if !line.is_empty() {
        line
    } else {
        match (a, outcome) {
            (Answer::Soften, "good") => "Alright. I'll keep at it.".into(),
            (Answer::Soften, _) => "If you say so, Coach.".into(),
            (Answer::Dodge, _) => "That's not really an answer, Coach.".into(),
            (Answer::Promise, _) => "I'm going to hold you to that, Coach.".into(),
            (Answer::Straight, "good") => "I appreciate you being straight with me.".into(),
            (Answer::Straight, _) => "So that's where it is.".into(),
        }
    };
    let sign = match outcome {
        "good" => 1,
        "bad" => -1,
        _ => 0,
    };
    s.them(&name, line, read_tell(w, i, sign));
    if let Some(l) = s.lines.last_mut().filter(|l| l.by == 2) {
        l.went = Some(went_words(w, dt, dm));
    }
    made(s, key, a.key(), dt + dm);
    // A promise: a start for a benched man, your word for your job; on a
    // promise already open, your word again (nothing new is logged).
    if a == Answer::Promise {
        let kind = match (job, topic.point()) {
            (true, _) => Some(PromiseKind::Stay),
            (false, Some(PointKind::PromiseOpen)) => None,
            (false, Some(k)) => Some(tp::promise_kind(w, i, k)),
            (false, None) => None,
        };
        match kind {
            Some(kind) => {
                promise(w, PromiseTo::Player(id), kind, 0);
                s.outcome.push(format!("You promised {} {}.", name, kind.words()));
                // He isn't sure, or doubts: one follow-up (the promises
                // design, section 5).
                if s.step < 3 && follow_up(w, s, i) {
                    return;
                }
            }
            None => s.outcome.push(format!("You gave {name} your word again.")),
        }
    }
    let used: Vec<Move> =
        Move::ALL.iter().enumerate().filter(|(k, _)| s.c & (1 << (2 + k)) != 0).map(|x| *x.1).collect();
    if s.step >= 3 {
        finish_talk(w, s, id);
    } else {
        add_picks(w, s, i, topic, &used);
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

/// Capitalizes the first letter, with no stop (`cap` adds one).
fn upper(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
}

/// Keeps the points made in a talk (`Talked`): what you said, and whether
/// it landed.
fn keep_made(w: &mut World, s: &mut Scene, id: u32) {
    let made = std::mem::take(&mut s.made);
    if made.is_empty() {
        return;
    }
    let Some(i) = super::person(w, id) else { return };
    let sheet = claims::player(w, i);
    let today = w.today;
    for (key, said, d) in made {
        let Some(k) = PointKind::from_key(&key) else { continue };
        let p: Point = sheet.get(k).cloned().unwrap_or_else(|| Point::new(k));
        w.folk.talked.note(today, id, &p, d > tp::LANDED, &said);
    }
}

fn finish_talk(w: &mut World, s: &mut Scene, id: u32) {
    keep_made(w, s, id);
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
    // The room reads your team's week (talk facts lane L9): a game is named
    // only while it is fresh (item 30), with its margin and where it was
    // played; a streak, the starters out, the next game and its rival; and
    // with nothing near, how the season ended or what is coming.
    let wk = claims::team_rooms::week(w, team);
    let mood = team_mood(w, team);
    s.who.push(Who::Group("The team".into()));
    let day = WEEKDAY_NAMES[w.today.weekday().index() as usize];
    match &wk.last {
        Some(g) => s.narrate(format!("{day}, {} {}", game_words(w, g), room_after(g))),
        None => {
            let then = match (&wk.next, &wk.ended, &wk.ahead) {
                (Some(n), _, _) => format!(
                    "{} is next, {}.{} Everyone is waiting to hear what kind of week this is.",
                    opp_link(w, n),
                    days_words(w, n.due.unwrap_or(w.today)),
                    if wk.rival_week() { " Rivalry week." } else { "" }
                ),
                (None, Some(e), _) => format!(
                    "The season is over: {}{}, and it ended {}. Everyone is waiting to hear what comes next.",
                    e.get("record").unwrap_or(""),
                    e.get("finish").map(|f| format!(", {f}")).unwrap_or_default(),
                    e.get("when").unwrap_or("this winter")
                ),
                (None, None, Some(a)) => format!(
                    "{} is next, {}. Everyone is waiting to hear what kind of week this is.",
                    upper(a.get("what").unwrap_or("practice")),
                    a.get("when").unwrap_or("soon")
                ),
                (None, None, None) => "Everyone is waiting to hear what kind of week this is.".into(),
            };
            s.narrate(format!("{day}. No game to talk about. {then}"))
        }
    }
    if let Some(st) = &wk.streak {
        let n: usize = st.get("games").and_then(|x| x.parse().ok()).unwrap_or(3);
        s.narrate(format!(
            "That's {} straight {}.",
            super::count_words(n),
            if wk.run() == Some("winning") { "wins" } else { "losses" }
        ));
    }
    if let Some(o) = &wk.out {
        let n: usize = o.get("out").and_then(|x| x.parse().ok()).unwrap_or(1);
        let names = o.get("names").unwrap_or("");
        s.narrate(if n > 3 {
            format!("Out hurt: {}, {names} among them.", o.get("starters").unwrap_or("starters"))
        } else {
            format!("Out hurt: {names}.")
        });
    }
    if wk.last.is_some()
        && let Some(n) = &wk.next
    {
        s.narrate(format!(
            "{} is next, {}.{}",
            opp_link(w, n),
            days_words(w, n.due.unwrap_or(w.today)),
            if wk.rival_week() { " Rivalry week." } else { "" }
        ));
    }
    s.narrate(format!("The room's mood: {}.", mood_words(mood)));
    add_team_picks(s, &wk);
}

const WEEKDAY_NAMES: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// A game's opponent, linked when the point names his team.
fn opp_link(w: &World, p: &Point) -> String {
    claims::team_rooms::team_of(p).map_or_else(|| p.get("opp").unwrap_or("them").to_string(), |t| t.link(w))
}

/// "the 24-17 win over X in the Rose Bowl".
fn result_words(w: &World, g: &Point) -> String {
    let (word, to) = match g.check("result") {
        Some("win") => ("win", "over"),
        Some("loss") => ("loss", "to"),
        _ => ("tie", "with"),
    };
    format!(
        "the {} {word} {to} {}{}",
        g.get("score").unwrap_or(""),
        opp_link(w, g),
        g.get("game").map(|n| format!(" in the {n}")).unwrap_or_default()
    )
}

/// How long after the game: "after the...", "the day after the...",
/// "three days after the...".
fn game_words(w: &World, g: &Point) -> String {
    let ago = g.day.map_or(0, |d| d.days_to(w.today));
    let after = match ago {
        0 => "after".to_string(),
        1 => "the day after".to_string(),
        n => format!("{} days after", super::count_words(n as usize)),
    };
    format!("{after} {}.", result_words(w, g))
}

/// The room after a game, by its result and margin.
fn room_after(g: &Point) -> &'static str {
    match (g.check("result"), g.check("margin")) {
        (Some("win"), Some("close")) => "The room is loud and tired: that one took everything.",
        (Some("win"), _) => "The room is loose; somebody has music on.",
        (Some("loss"), Some("close")) => "The room is quiet. Half of them are still replaying the last drive.",
        (Some("loss"), _) => "Nobody is talking much.",
        _ => "Nobody knows quite what to make of it.",
    }
}

/// The season's last game: "the 24-31 loss to X in the Y Bowl on
/// December 30."
/// "today", "tomorrow", "in 10 days".
fn days_words(w: &World, date: Date) -> String {
    match w.today.days_to(date) {
        0 => "today".to_string(),
        1 => "tomorrow".to_string(),
        n => format!("in {n} days"),
    }
}

/// The five things you can tell the room, each worded for the week: the
/// codes and what each does are the same as they always were (lane L9
/// changes the words only).
fn add_team_picks(s: &mut Scene, wk: &claims::team_rooms::Week) {
    let g = |p: &Option<Point>, k: &str| p.as_ref().and_then(|x| x.get(k)).map(str::to_string);
    let (score, opp) = (g(&wk.last, "score"), g(&wk.last, "opp"));
    let next = g(&wk.next, "opp");
    let names = g(&wk.out, "names");
    let record = g(&wk.ended, "record");
    let what = g(&wk.ahead, "what");
    let streak = wk.streak.as_ref().and_then(|p| p.get("games")).and_then(|x| x.parse::<usize>().ok());
    let focus = match (&next, &what) {
        (Some(n), _) => format!("\"One play at a time. {n} is all that matters this week.\" (focus)"),
        (None, Some(a)) => format!("\"One rep at a time. Get ready for {a}.\" (focus)"),
        _ => "\"One play at a time. Do your job.\" (focus)".to_string(),
    };
    s.pick(200, focus, "Focus", "calm", "Steady and safe: small lift, little risk.");
    let calm = match (wk.lost(), &names, wk.run(), &score) {
        (_, Some(n), _, _) => format!("\"We're without {n}. Next man up. Nothing changes.\" (calm)"),
        (_, None, Some("losing"), _) => format!(
            "\"{} straight doesn't define us. Breathe. Nothing changes.\" (calm)",
            upper(&super::count_words(streak.unwrap_or(3)))
        ),
        (true, None, _, Some(sc)) => format!("\"Breathe. One {sc} loss doesn't change who we are.\" (calm)"),
        _ => "\"Breathe. We're fine. Nothing changes.\" (calm)".to_string(),
    };
    s.pick(201, calm, "Reassure", "warm", "Settles a shaky room; a competitive room wants more.");
    let fire = match &next {
        Some(n) if wk.rival_week() => format!("\"It's {n} week. I want somebody to hit somebody.\" (fire)"),
        Some(n) => format!("\"I want somebody to hit somebody against {n}.\" (fire)"),
        None => "\"I want somebody to hit somebody this week.\" (fire)".to_string(),
    };
    s.pick(202, fire, "Fire", "fiery", "Lifts competitors; wears thin if you use it every week.");
    let (challenge, hint) = match (wk.result(), &score, &opp, &record) {
        (Some("loss"), Some(sc), Some(o), _) => (
            format!("\"{sc} against {o}. That wasn't good enough, and you know it.\" (challenge)"),
            "Hits the proud players where it should; the fragile sink.",
        ),
        (Some("win"), _, Some(o), _) => (
            format!("\"We beat {o}, and that still wasn't our standard.\" (challenge)"),
            "After a win it counts for half: some of them want to enjoy it.",
        ),
        (None, _, _, Some(r)) if wk.season() == Some("bad") => (
            format!("\"{r}. Nobody in this room forgets that.\" (challenge)"),
            "Hits the proud players where it should; the fragile sink.",
        ),
        (None, ..) => (
            "\"Nobody in this room has earned anything yet.\" (challenge)".to_string(),
            "Hits the proud players where it should; the fragile sink.",
        ),
        _ => (
            "\"That wasn't good enough, and you know it.\" (challenge)".to_string(),
            "Hits the proud players where it should; the fragile sink.",
        ),
    };
    s.pick(203, challenge, "Challenge", "firm", hint);
    let (praise, hint) = match (wk.result(), &score, &opp, &record) {
        (Some("win"), Some(sc), Some(o), _) => {
            (format!("\"That {sc} win over {o} was yours. You earned it.\" (praise)"), "Lands after a win.")
        }
        (Some("loss"), Some(sc), _, _) => (
            format!("\"I believe in this group, {sc} or not.\" (praise)"),
            "Rings hollow after a loss: it can cool the room.",
        ),
        (None, _, _, Some(r)) if wk.season() == Some("good") => {
            (format!("\"{r}. That season was yours.\" (praise)"), "They earned it; it lands.")
        }
        _ if wk.run() == Some("winning") => (
            format!(
                "\"{} straight. I believe in this group more than ever.\" (praise)",
                upper(&super::count_words(streak.unwrap_or(3)))
            ),
            "Lands after a win.",
        ),
        _ => (
            "\"I believe in this group more than ever.\" (praise)".to_string(),
            "Lands after a win; rings hollow after a bad loss.",
        ),
    };
    s.pick(204, praise, "Praise", "warm", hint);
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
    // Praise after a loss and a challenge after a win read only this
    // week's game (`facts::fresh_game`, 7 days): a January loss is not what
    // the room hears in April (talk facts item 30; Cain's yes 2026-11-13).
    let won = super::facts::fresh_game(w, team).map(|x| x.0);
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
    // How it really moved the room (lane L9): each man's trust before and
    // after, so the reaction names who took it best and who worst.
    let before: Vec<(usize, f32)> = team.roster(w).into_iter().map(|i| (i, w.people[i].trust)).collect();
    let (up, down, _) = super::takeover::team_effect(w, team, msg, tone, weight);
    let (best, worst) = movers(w, &before);
    let lands = worn < 2 && up > down * 2;
    s.narrate(if worn >= 2 {
        "You've said it this way before. A few of them mouth the words with you.".to_string()
    } else if lands {
        match best {
            Some(i) => {
                format!("It lands. The noise when you finish is real; {} is the first one up.", player_link(w, i))
            }
            None => "It lands. The noise when you finish is real.".to_string(),
        }
    } else if down > up {
        match worst {
            Some(i) => format!(
                "Polite silence. Some of them are already looking at their phones. {} doesn't look up at all.",
                player_link(w, i)
            ),
            None => "Polite silence. Some of them are already looking at their phones.".to_string(),
        }
    } else {
        match (best, worst) {
            (Some(a), Some(b)) => format!(
                "They take it in. {} nods along; {} stares at the floor. Practice will tell you more.",
                player_link(w, a),
                player_link(w, b)
            ),
            _ => "They take it in. Practice will tell you more.".to_string(),
        }
    });
    // A captain who took it well says something to the room (lane L9).
    if worn < 2
        && up > down
        && let Some(c) = super::takeover::captains_of(w, team)
            .into_iter()
            .find(|&c| before.iter().any(|&(i, t)| i == c && w.people[c].trust > t + MOVED))
    {
        let wk = claims::team_rooms::week(w, team);
        let mut f = super::takeover::base_facts(w, team);
        wk.fill(&mut f);
        let spk = Speaker::player(w, c);
        let key = if lands { "teamtalk.react.lands" } else { "teamtalk.react.mixed" };
        let line = scene::bank_line(w, s, 2, key, &spk, &f, "");
        if !line.is_empty() {
            let name = w.people[c].name.clone();
            s.them(&name, line, None);
        }
    }
    s.outcome.push(format!(
        "Your team talk: {up} lifted, {down} cooled{}.",
        if worn >= 2 { " (the tone is wearing thin)" } else { "" }
    ));
    s.end();
}

/// A man whose trust moved by more than this took the talk to heart.
/// Proposed, not confirmed.
const MOVED: f32 = 0.02;

/// Who the talk moved most each way: the biggest rise in trust and the
/// biggest fall, each past `MOVED`.
pub(crate) fn movers(w: &World, before: &[(usize, f32)]) -> (Option<usize>, Option<usize>) {
    let d = |&(i, t): &(usize, f32)| (i, w.people[i].trust - t);
    let ds: Vec<(usize, f32)> = before.iter().map(d).collect();
    let best = ds.iter().filter(|x| x.1 > MOVED).max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0))).map(|x| x.0);
    let worst = ds.iter().filter(|x| x.1 < -MOVED).min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0))).map(|x| x.0);
    (best, worst)
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
    // What the family calls about and what it can see (talk facts lane L9):
    // the real complaint, his group in practice as he tells it at home, his
    // snaps and starts in your last two games.
    let fam = claims::team_rooms::family(w, i);
    let complaint = fam.as_ref().map_or(claims::team_rooms::Complaint::Role, |x| x.complaint);
    s.b = complaint_code(complaint);
    s.narrate(format!(
        "{who} calls about {}. {}",
        player_link(w, i),
        match style {
            crate::world::background::ParentStyle::LoudAtGames => "You've heard this voice from the stands.",
            crate::world::background::ParentStyle::SocialMedia => "Your staff has already shown you the posts.",
            _ => "It's the first time this parent has called.",
        }
    ));
    if let Some(sees) = fam.as_ref().and_then(|x| x.sees()) {
        s.narrate(format!("What the family can see: {sees}"));
    }
    let spk = Speaker::parent(w, i);
    let mut f = player_facts(w, i);
    if let Some(x) = &fam {
        x.fill(&mut f);
    }
    let point = fam.as_ref().and_then(|x| x.point.as_ref());
    let mut pen = scene::pen(w, s, 1);
    // The complaint's own part (the point's when there is one), else
    // today's opening and point, which are about playing time.
    let own = match point {
        Some(pt) => format!("parents.call.{}", pt.kind.key()),
        None => format!("parents.call.{}", complaint.key()),
    };
    // Today's opening and point are about playing time: they go with a
    // role complaint only. Money, a punishment or a position move is the
    // complaint's own part, which carries its own greeting, or a plain
    // line that names it.
    let line = if complaint == claims::team_rooms::Complaint::Role {
        match pen.part(&own, &spk, &f) {
            Some(x) => {
                let open = pen.part("parents.call.open", &spk, &f).unwrap_or_default();
                super::voice::tidy(&format!("{open} {x}"))
            }
            None => pen.line(&["parents.call.open", "?parents.call.point"], &spk, &f),
        }
    } else {
        pen.part(&own, &spk, &f).map(|x| super::voice::tidy(&x)).unwrap_or_else(|| {
            let name = f.get("name").unwrap_or("my son").to_string();
            match complaint {
                claims::team_rooms::Complaint::Punishment => {
                    format!("Coach, I'm calling about {name}'s punishment. I'd like to understand it.")
                }
                claims::team_rooms::Complaint::Money => {
                    format!("Coach, I'm calling about {name}'s deal. I don't think it's right.")
                }
                _ => match f.get("position") {
                    Some(x) => format!("Coach, I'm calling about {name} moving to {x}. Nobody explained it to us."),
                    None => format!("Coach, I'm calling about {name}'s position. Nobody explained it to us."),
                },
            }
        })
    };
    pen.put(w);
    s.them(
        who,
        if line.is_empty() {
            format!(
                "Coach, I'm going to be honest with you. {} should be on the field, and everybody in the stands can see it.",
                f.get("name").unwrap_or("My son")
            )
        } else {
            line
        },
        None,
    );
    add_parent_picks(s, complaint, point, &f, fam.as_ref());
}

/// A complaint's code in `Scene.b` (the parent's call).
fn complaint_code(c: claims::team_rooms::Complaint) -> u64 {
    use claims::team_rooms::Complaint as C;
    match c {
        C::Role => 0,
        C::Punishment => 1,
        C::Money => 2,
        C::Position => 3,
    }
}

fn complaint_of(b: u64) -> claims::team_rooms::Complaint {
    use claims::team_rooms::Complaint as C;
    match b {
        1 => C::Punishment,
        2 => C::Money,
        3 => C::Position,
        _ => C::Role,
    }
}

/// Your four answers, worded for the complaint (lane L9). The codes and
/// what each does are the same as they always were; a promise of snaps is
/// on the table only where snaps are what the family wants: his role, a
/// position move, or a punishment of a man who played before it.
fn add_parent_picks(
    s: &mut Scene,
    c: claims::team_rooms::Complaint,
    point: Option<&Point>,
    f: &Facts,
    fam: Option<&claims::team_rooms::Family>,
) {
    use claims::team_rooms::Complaint as C;
    let name = f.get("name").unwrap_or("He").to_string();
    let v = |k: &str| point.and_then(|p| p.get(k)).map(str::to_string);
    let straight = match c {
        C::Role => match (point.map(|p| p.kind), v("rival"), v("ahead"), fam.and_then(|x| x.group)) {
            (Some(PointKind::LostJob), Some(r), _, _) => {
                format!("\"{r} won the job in practice. Here's what {name} needs to do to win it back.\"")
            }
            (Some(PointKind::Buried), _, Some(a), _) => {
                format!("\"{name} is behind {a} right now. Here's what he needs to show us.\"")
            }
            (_, _, _, Some(g)) => format!(
                "\"He's working with {}. Playing time is earned at practice. Here's what moves him up.\"",
                claims::team_rooms::group_words(g)
            ),
            _ => "\"Playing time is earned at practice. Here's what he needs to do.\"".to_string(),
        },
        C::Punishment => match (v("rule"), fam.and_then(|x| x.penalty.clone()), v("reason")) {
            (Some(r), Some(x), _) => format!("\"He broke our rule on {r}, and {x} stands. Here's why.\""),
            (None, Some(x), _) => format!("\"{} stands, and here's why.\"", upper(&x)),
            (_, None, Some(r)) => format!("\"He's sitting for {r}, and it stands. Here's why.\""),
            _ => "\"He's sitting because of what he did, and it stands. Here's why.\"".to_string(),
        },
        C::Money => "\"His deal is what it is this year. How he plays earns the next one.\"".to_string(),
        C::Position => match v("position") {
            Some(x) => format!("\"We moved him to {x} because that's where we see him getting on the field.\""),
            None => "\"We moved him because that's where we see him getting on the field.\"".to_string(),
        },
    };
    s.pick(
        300,
        straight,
        "Tell him straight",
        "calm",
        "Honest: a reasonable parent respects it; a loud one goes to the AD.",
    );
    let listen = match c {
        C::Role | C::Position => "\"I hear you. Let me look at it with his position coach.\"",
        C::Punishment => "\"I hear you. I'll sit down with him myself.\"",
        C::Money => "\"I hear you. I'll look at where his deal stands.\"",
    };
    s.pick(301, listen, "Listen", "warm", "Cools the call down; promises nothing.");
    let played = fam.is_some_and(|x| x.games.iter().any(|g| g.started) || x.group == Some("ones"));
    let promise = match c {
        C::Role => Some("\"He'll get his chance. I promise you that.\"".to_string()),
        C::Position => Some(match v("position") {
            Some(x) => format!("\"He'll get his snaps at {x}. I promise you that.\""),
            None => "\"He'll get his snaps. I promise you that.\"".to_string(),
        }),
        C::Punishment if played => Some("\"When it's served, he's back where he was. You have my word.\"".to_string()),
        _ => None,
    };
    if let Some(p) = promise {
        s.pick(302, p, "Promise", "warm", "Pleases the parent now; logged as a promise of snaps.");
    }
    let warn = match c {
        C::Role => "\"I don't discuss playing time with parents.\"",
        C::Punishment => "\"Team discipline stays between me and the player.\"",
        C::Money => "\"I don't talk about a player's money with his family.\"",
        C::Position => "\"Where a player lines up is a coaching decision.\"",
    };
    s.pick(303, warn, "Warn", "firm", "Ends it; the parent may go above you.");
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
    // The parent answers what you said about the real complaint (lane L9):
    // the bank's part for your answer and how it went, else today's line.
    let complaint = complaint_of(s.b);
    let key = match code {
        300 if above => "parents.reply.straight.above",
        300 => "parents.reply.straight.fair",
        301 => "parents.reply.listen",
        302 => "parents.reply.promise",
        _ => "parents.reply.warn",
    };
    let spk = Speaker::parent(w, i);
    let mut f = player_facts(w, i);
    f.set("complaint", complaint.key());
    if let Some(x) = claims::team_rooms::family(w, i).filter(|x| x.complaint == complaint) {
        x.fill(&mut f);
    }
    let said = scene::bank_line(w, s, 2, key, &spk, &f, line);
    s.them(parent_word(w, i), said, None);
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
            format!("Heard from {}'s family about {}.", player_link(w, i), complaint_words(complaint)),
            None,
        );
    }
    s.end();
}

/// What a family called about, in a note's words.
fn complaint_words(c: claims::team_rooms::Complaint) -> &'static str {
    use claims::team_rooms::Complaint as C;
    match c {
        C::Role => "playing time",
        C::Punishment => "his punishment",
        C::Money => "his money",
        C::Position => "his position move",
    }
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
            keep_made(w, s, id);
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
    let pc = position_coach_now(w, i);
    let coach = pc.map_or_else(|| "His position coach".to_string(), |c| super::coach_link(w, c));
    if let Some(c) = pc {
        s.who.push(Who::Coach(c));
    }
    // A coordinator or head coach covering an empty seat speaks as himself.
    let speaker = match pc.and_then(|c| w.coaches[c].job) {
        Some(j) if !matches!(j.role, crate::world::coach::Role::PositionCoach(_)) => {
            let mut ch = j.role.name().chars();
            ch.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + ch.as_str())
        }
        _ => "Position coach".to_string(),
    };
    let (_, words) = concern(w, i);
    s.narrate(format!("{coach} catches you after practice about {}.", player_link(w, i)));
    s.them(&speaker, format!("You asked me to tell you if he was slipping. He is. Best I can tell, {words}."), None);
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
            sc.b = worry_on(w, i, &claims::player(w, i)).map_or(concern(w, i).0 as u64, |x| x.0.code());
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

/// A chart change's mood waits until he sees it at practice (the promises
/// design, section 2).
pub(crate) fn when_seen(w: &mut World, id: u32, why: Reason, delta: f32) {
    let today = w.today;
    w.folk.mine.unseen.retain(|x| x.0 != id);
    w.folk.mine.unseen.push((id, delta, why as u8, today));
}

/// The chart changes he has now seen at practice land; ones you undid
/// before a practice week showed them, or that no practice week showed in
/// three weeks, never do.
fn chart_seen(w: &mut World) {
    let waiting = std::mem::take(&mut w.folk.mine.unseen);
    let mut keep = Vec::new();
    for (id, delta, why, made) in waiting {
        let weeks: Vec<(Date, u8)> = super::seen::weeks(w, id).into_iter().filter(|x| x.0 >= made).collect();
        if super::seen::group_changed(w, id, made).is_some() {
            mood_event(w, id, Reason::from_u8(why), delta);
        } else if weeks.is_empty() && made.days_to(w.today) < 21 {
            keep.push((id, delta, why, made));
        }
    }
    w.folk.mine.unseen = keep;
}

pub(crate) fn day(w: &mut World, d: Date) {
    check_promises(w, d);
    super::claims::truth::check_lies(w, d);
    if !w.folk.mine.unseen.is_empty() {
        chart_seen(w);
    }
    if d.weekday() == Weekday::Mon {
        super::promise::week(w, d);
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

/// A loud parent calls when something the family can see sours him (high
/// school and college; proposed, not confirmed: a few a season): his role,
/// a punishment of yours, his money or a position move (talk facts lane
/// L9, `claims::team_rooms::family`). Before the lane only his role.
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
        p.mood < -0.25
            && matches!(
                p.family.parent_style,
                crate::world::background::ParentStyle::LoudAtGames | crate::world::background::ParentStyle::SocialMedia
            )
            && super::roll(w, 0xCA11 ^ d.0 as u64, p.player.id as u64) < 0.25
            && (p.mood_why == Why::Role as u8 || claims::team_rooms::family(w, i).is_some())
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
        s.stops = false;
        s.place = "Your office".into();
        exit_reason(w, &mut s, i, None);
        scene::add(w, s);
    }
}

/// Marks a one-on-one as an exit meeting (`Scene.c`): the talk opens on
/// why he is on your list, and only an open case comes before it.
pub(crate) const EXIT: i64 = 1 << 22;
/// Where the reason no point carries sits in `Scene.c` (four bits).
const EXIT_SHIFT: u32 = 24;

/// Carries the real reason he is on your exit list into the talk (talk
/// facts lane L9, item 21): the point on his sheet you know that is the
/// reason (the talk's topic), or your staff's own reason from its list
/// (`staff`, the takeover's flight list), which his opening speaks to.
pub(crate) fn exit_reason(w: &World, s: &mut Scene, i: usize, staff: Option<&str>) {
    use claims::team_rooms::Leaving;
    let why = claims::team_rooms::leaving(w, i, staff);
    let topic: Topic = match why {
        Leaving::Point(k) if tp::talkable(k) => Topic::Point(k),
        Leaving::Point(k) => tp::subject_of(k).into(),
        Leaving::Role => Subject::Role.into(),
        _ => Subject::Future.into(),
    };
    s.b = topic.code();
    s.c |= EXIT | (why.code() << EXIT_SHIFT);
}

/// A takeover's flight list speaks for the meetings it set up this long.
const TAKEOVER_LIST_DAYS: i32 = 30;

/// Your staff's note on why he is on your list, said as the talk opens.
fn exit_note(w: &World, s: &Scene, i: usize, sheet: &Sheet) -> String {
    use claims::team_rooms::Leaving;
    let id = w.people[i].player.id;
    // A takeover's list says it in its own words.
    if let Some((_, why)) = w
        .folk
        .takeover
        .as_ref()
        .filter(|t| t.started.is_some_and(|d| (0..=TAKEOVER_LIST_DAYS).contains(&d.days_to(s.date))))
        .and_then(|t| t.risks.iter().find(|x| x.0 == id))
    {
        return why.clone();
    }
    match Topic::of(s.b).point().and_then(|k| sheet.get(k)) {
        Some(p) => tp::worry_words(p),
        None => Leaving::of_code((s.c >> EXIT_SHIFT) & 0xF).words().to_string(),
    }
}

/// AI coaches talk too (topic 5): each week an AI head coach sees his
/// unhappiest players. Since talk facts lane L4 he has your talk: what's on
/// the player's mind, the move he reads as best (his relationships rating
/// is how well he reads it) and the same landing (`talk_points::ai_talk`).
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
        // The same talk as yours (talk facts lane L4, rule 8): his topic, the
        // move he reads best and how it lands.
        for (i, _) in worst.into_iter().take(2) {
            let (dt, dm) = tp::ai_talk(w, i, rel, 0xA17A ^ (d.0 as u64) << 20 ^ i as u64);
            moves.push((i, dt, dm));
        }
    }
    for (i, dt, dm) in moves {
        let p = &mut w.people[i];
        p.trust = (p.trust + dt).clamp(0.0, 1.0);
        p.mood = (p.mood + dm).clamp(-1.0, 1.0);
    }
}
