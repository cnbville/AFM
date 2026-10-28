//! Update 2, the storyline engine (S1-S10; design in the Claude Doc "The
//! storyline engine", list in `afm/post-release-2/list.md`). It turns the
//! world's events into stories that last: a story has people in it,
//! something at stake, a question everyone is asking, a past and a next
//! step. A quarterback battle builds for three weeks and ends on Saturday;
//! a losing streak feeds a hot seat, the hot seat makes a commitment wobble.
//!
//! How it is built:
//! - One record per story (`Story`): its kind from the catalogue
//!   (`catalogue.rs`, 117 kinds in 14 families), its cast in roles, the
//!   question, the stakes, what is really going on (truth) apart from what
//!   the public believes and how sure it is, the sides as shares, heat 0-100
//!   day by day, its phase (seed, spark, building, peak, resolved,
//!   aftermath, or fizzled), its beats with their sources, its links to
//!   other stories, its ending and what it leaves behind.
//! - Detectors only notice (S9, no drama director): they read the world's
//!   own state every day and on Mondays (`detect/`), and they never bend an
//!   outcome. A private story goes public only through a person: a reporter
//!   with a source, a player's post, a parent on the radio, a rival's
//!   remark, your own words.
//! - Heat rises with beats, stakes, the cast's fame, the market's size and
//!   slow news days (the summer dead zone), and fades every quiet day.
//!   Stories compete for each audience's daily attention, so a firing pushes
//!   a kicker's slump off the front page. A story with no beat for its
//!   kind's quiet days fizzles, as most rumours do.
//! - Voices tell it (`tell.rs`): beat writers, national columns, radio,
//!   fans, players and parents, rival coaches, your staff's memos, all from
//!   the line banks with their history ("for the third week running").
//! - Effects go through people (`effects.rs`): mood, the locker room,
//!   recruits, the boss, fans, the cast's own choices.
//! - Your side (`moves.rs`, `credit.rs`, `view.rs`): your moves are beats,
//!   a beat that needs you stops Continue, handling a story well pays
//!   (S10), and the job pick reads a job's inherited stories (S8).
//!
//! Detail follows distance: stories around you keep every beat; far ones
//! keep their cast, heat and key beats. The store has its own random
//! stream, so a seed gives the same stories. Sizes are Claude's picks
//! (proposed, not confirmed), listed in `docs/world-model.md`.

pub mod catalogue;
pub mod credit;
pub mod detect;
pub mod effects;
pub mod moves;
pub mod tell;
pub mod view;

use crate::calendar::{Date, Level, Weekday};
use crate::rng::Rng;

use super::World;
use super::stories::TeamKey;
use super::stories::lines::{self, Fill};
pub use catalogue::{Def, Family, K, Rating};

/// The rule on every calendar line the storylines post.
pub const RULE: &str = "storylines";

/// A story's life (the design's six phases and the dead end).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Phase {
    /// A detector noticed it; only the people close to it know.
    Seed,
    /// It went public through a person.
    Spark,
    /// Beats are coming.
    Building,
    /// The moment the question gets answered or forced.
    Peak,
    /// A world fact closed it.
    Resolved,
    /// What it left behind, still talked about.
    Aftermath,
    /// No new beats for long enough: it died, as most rumours do.
    Fizzled,
}

impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Phase::Seed => "Seed",
            Phase::Spark => "Spark",
            Phase::Building => "Building",
            Phase::Peak => "Peak",
            Phase::Resolved => "Resolved",
            Phase::Aftermath => "Aftermath",
            Phase::Fizzled => "Fizzled",
        }
    }
    pub fn open(self) -> bool {
        self < Phase::Resolved
    }
}

/// Who is in a story.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Who {
    /// A player, by id.
    Player(u32),
    /// A coach, by index.
    Coach(u32),
    /// A team as a whole: its program, its fans.
    Team(TeamKey),
    /// The boss who hires and fires at a team.
    Boss(TeamKey),
    /// A player's parent, by the player's id.
    Parent(u32),
}

/// A cast member's part in a story.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Part {
    Lead,
    Rival,
    /// Who decides it (the head coach for a depth chart, the boss for a
    /// firing).
    Decider,
    Boss,
    Parent,
    /// Who wants him (a program after a recruit, a club after a coach).
    Suitor,
    Other,
}

impl Part {
    pub fn name(self) -> &'static str {
        match self {
            Part::Lead => "Lead",
            Part::Rival => "Rival",
            Part::Decider => "Decider",
            Part::Boss => "Boss",
            Part::Parent => "Parent",
            Part::Suitor => "Suitor",
            Part::Other => "Around it",
        }
    }
}

/// One person in the cast: who, the part, and which way he leans (-100
/// against the lead, 100 with him; the lead and rival back themselves).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Member {
    pub who: Who,
    pub part: Part,
    pub side: i8,
}

/// What moved a story.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BeatKind {
    /// The detector's first look (private).
    Seed,
    /// It went public.
    Spark,
    /// A reporter's story from his sources.
    Report,
    /// "Sources say": a leak from inside the building.
    Leak,
    /// A cast member on the record.
    Quote,
    /// A social post by someone in it.
    Post,
    /// The call-in shows and the radio voices.
    Radio,
    /// A national column.
    Column,
    /// A rival's remark.
    Jab,
    /// Your move.
    Move,
    Denial,
    /// A world fact (a game, a signing, a firing).
    Fact,
    Peak,
    Resolve,
    Fizzle,
    /// A line from the past, years later.
    Callback,
    /// Your staff's memo (private).
    Memo,
    /// The fans' boards and feeds.
    Fans,
    /// Your move in private: a talk, a call, a meeting.
    Private,
}

impl BeatKind {
    pub fn key(self) -> &'static str {
        match self {
            BeatKind::Seed => "seed",
            BeatKind::Spark => "spark",
            BeatKind::Report => "report",
            BeatKind::Leak => "leak",
            BeatKind::Quote => "quote",
            BeatKind::Post => "post",
            BeatKind::Radio => "radio",
            BeatKind::Column => "column",
            BeatKind::Jab => "jab",
            BeatKind::Move => "move",
            BeatKind::Denial => "denial",
            BeatKind::Fact => "fact",
            BeatKind::Peak => "peak",
            BeatKind::Resolve => "end",
            BeatKind::Fizzle => "fizzle",
            BeatKind::Callback => "callback",
            BeatKind::Memo => "memo",
            BeatKind::Fans => "fans",
            BeatKind::Private => "private",
        }
    }
    /// What a beat of this kind adds to heat before the story's amplifiers.
    pub fn heat(self) -> f32 {
        match self {
            BeatKind::Seed | BeatKind::Memo | BeatKind::Callback | BeatKind::Private => 0.0,
            BeatKind::Spark => 15.0,
            BeatKind::Report => 8.0,
            BeatKind::Leak => 11.0,
            BeatKind::Quote => 5.0,
            BeatKind::Post => 8.0,
            BeatKind::Radio => 4.0,
            BeatKind::Column => 7.0,
            BeatKind::Jab => 9.0,
            BeatKind::Move => 4.0,
            BeatKind::Denial => 5.0,
            BeatKind::Fact => 6.0,
            BeatKind::Peak => 18.0,
            BeatKind::Resolve => 10.0,
            BeatKind::Fizzle => 0.0,
            BeatKind::Fans => 3.0,
        }
    }
    /// Whether the public hears it.
    pub fn public(self) -> bool {
        !matches!(self, BeatKind::Seed | BeatKind::Memo | BeatKind::Private)
    }
    /// Whether it moves the story on: something happened, or someone in it
    /// acted. Talk about it (reports, columns, the radio, the fans) keeps
    /// it warm but is no news, so a story nothing happens in goes quiet.
    pub fn news(self) -> bool {
        matches!(
            self,
            BeatKind::Spark | BeatKind::Fact | BeatKind::Peak | BeatKind::Resolve | BeatKind::Move | BeatKind::Denial
        )
    }
}

/// Who tells a beat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Voice {
    /// The world itself: a result, a filing, a box score.
    World,
    /// The team's beat writer.
    Beat,
    /// A national columnist or network insider.
    National,
    /// A recruiting insider.
    Insider,
    /// The call-in shows; your own radio voice on your games.
    Radio,
    Fans,
    Player,
    Parent,
    /// A rival coach.
    Rival,
    /// Your staff.
    Staff,
    You,
    Boss,
    Recruit,
    Agent,
}

impl Voice {
    pub fn name(self) -> &'static str {
        match self {
            Voice::World => "The record",
            Voice::Beat => "Beat writer",
            Voice::National => "National columnist",
            Voice::Insider => "Recruiting insider",
            Voice::Radio => "Radio",
            Voice::Fans => "Fans",
            Voice::Player => "A player",
            Voice::Parent => "A parent",
            Voice::Rival => "A rival coach",
            Voice::Staff => "Your staff",
            Voice::You => "You",
            Voice::Boss => "The boss",
            Voice::Recruit => "A recruit",
            Voice::Agent => "An agent",
        }
    }
}

/// One dated thing that moved a story, with its source. `who` is the cast
/// member it came from (255: none). Far stories keep only their key beats.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Beat {
    pub date: Date,
    pub what: BeatKind,
    pub by: Voice,
    pub who: u8,
    /// Heat it added, rounded.
    pub heat: i8,
    pub text: String,
}

/// One story.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Story {
    pub id: u32,
    /// The kind's index in the catalogue.
    pub kind: u16,
    /// What it is about, unique per kind among open stories (a coach, a
    /// player, a team's season).
    pub subject: u64,
    pub level: Level,
    pub team: Option<TeamKey>,
    /// The other side: a rival, a suitor, an old team.
    pub other: Option<TeamKey>,
    pub cast: Vec<Member>,
    /// 0-100: how much rides on it here.
    pub stakes: u8,
    /// Which way it is really going, -1 (no, the lead holds) to 1 (yes, the
    /// change comes), as the people inside know it.
    pub truth: f32,
    /// What the public believes on the same scale, and how sure it is, 0-1.
    pub belief: f32,
    pub sure: f32,
    /// The share backing the lead: locker room, fans, media, boosters.
    pub sides: [f32; 4],
    pub heat: f32,
    pub peak_heat: f32,
    /// Heat each Monday since it opened.
    pub weeks: Vec<u8>,
    pub phase: Phase,
    /// The public knows.
    pub public: bool,
    pub opened: Date,
    pub public_on: Option<Date>,
    pub last_beat: Date,
    /// When the question gets forced: a depth chart, signing day, a game.
    pub due: Option<Date>,
    pub closed: Option<Date>,
    pub beats: Vec<Beat>,
    /// Other stories it touches, by id.
    pub links: Vec<u32>,
    /// The ending's code (`Def::ending`), 0 none (open or fizzled).
    pub ending: u8,
    /// Around you: every beat kept, every voice heard.
    pub near: bool,
    /// Your part in it, if any.
    pub you: Option<Part>,
    /// How you have handled it so far, -1 to 1 (S10), and how many moves.
    pub handled: f32,
    pub moves: u8,
    /// The kind's numbers: a streak's length, a record's mark, a win total.
    pub nums: [i32; 4],
    /// The football season it belongs to.
    pub season: i32,
    /// The cast's fame and the market's size when it last looked, 0-1.
    pub fame: f32,
    pub market: f32,
    /// What it leaves behind: the callback line people use for years.
    pub legacy: String,
    /// Your stance on it in public, from your moves: 1 backed the lead, -1
    /// backed the rival or the change, 0 open or silent.
    pub stance: i8,
    /// The kind's own word: a trophy, a conference, a record's holder, a
    /// new position, an award.
    pub word: String,
    /// What you said on the record (`moves::SAID_*`).
    #[serde(default)]
    pub said: u8,
    /// Its players' names when it ended, for the day one of them leaves
    /// the world: an ended story still names a retired player.
    #[serde(default)]
    pub names: Vec<(Who, String)>,
}

impl Story {
    pub fn def(&self) -> &'static Def {
        catalogue::def(self.kind)
    }
    /// The kind's name in its level's words: in the pros a club is a
    /// franchise and a first-year player a rookie.
    pub fn kind_name(&self) -> String {
        let name = self.def().name;
        if matches!(self.level, Level::Nfl | Level::Other) {
            name.replace("freshman", "rookie").replace("program", "franchise")
        } else {
            name.to_string()
        }
    }
    pub fn family(&self) -> Family {
        self.def().family
    }
    pub fn lead(&self) -> Option<Who> {
        self.cast.iter().find(|m| m.part == Part::Lead).map(|m| m.who)
    }
    pub fn rival(&self) -> Option<Who> {
        self.cast.iter().find(|m| m.part == Part::Rival).map(|m| m.who)
    }
    pub fn has(&self, who: Who) -> bool {
        self.cast.iter().any(|m| m.who == who)
    }
    pub fn lead_player(&self) -> Option<u32> {
        match self.lead() {
            Some(Who::Player(id)) => Some(id),
            _ => None,
        }
    }
    pub fn lead_coach(&self) -> Option<usize> {
        match self.lead() {
            Some(Who::Coach(c)) => Some(c as usize),
            _ => None,
        }
    }
    pub fn rival_player(&self) -> Option<u32> {
        match self.rival() {
            Some(Who::Player(id)) => Some(id),
            _ => None,
        }
    }
    /// Days since it opened.
    pub fn age(&self, d: Date) -> i32 {
        self.opened.days_to(d)
    }
    pub fn ending_key(&self) -> &'static str {
        self.def().ending_key(self.ending)
    }
    /// How the ending reads for its team (-1 to 1).
    pub fn valence(&self) -> f32 {
        self.def().valence(self.ending)
    }
    pub fn about(&self, t: TeamKey) -> bool {
        self.team == Some(t) || self.other == Some(t) || self.has(Who::Team(t))
    }
}

/// Everything the storylines keep.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Storylines {
    /// Own random stream (started from the world's seed on first use).
    pub(crate) rng: Option<Rng>,
    /// Lines used lately per key, so text rests before it comes back.
    pub(crate) recent: lines::Recent,
    /// Open stories by id (ids only grow, so the list stays sorted).
    pub open: Vec<Story>,
    /// Ended stories: the last two seasons in the world, every story you
    /// were in for good (your chapters), each compacted.
    pub ended: Vec<Story>,
    pub(crate) next: u32,
    /// Stories you follow (S5): their beats reach your inbox until they end.
    pub followed: Vec<u32>,
    /// Kinds resting for a subject after an ending: (kind, subject, until).
    pub(crate) cool: Vec<(u16, u64, Date)>,
    /// Your rewards and chapters (S10).
    pub credit: credit::Credit,
    /// What the detectors remember between looks.
    pub(crate) watch: detect::Watch,
    /// Stories opened by (season, kind) and how many went public, for
    /// trials and the world tests.
    pub counts: Vec<(i32, u16, u32, u32)>,
    /// Stories attended today (they get coverage).
    #[serde(skip)]
    pub(crate) attended: Vec<u32>,
    /// The recruiting read of each college program this week (S4): rivals'
    /// material from scandals and hot seats, the lift from a rising story.
    pub(crate) push: Vec<f32>,
    /// What the screens saw last: the day "Previously" was shown.
    pub previously_seen: Option<Date>,
    /// The season the yearly steps last ran for, by step.
    pub(crate) done: Vec<(u8, i32)>,
    /// Your story meetings by story, and the press questions asked.
    pub(crate) met: Vec<(u32, Date)>,
    pub(crate) asked: Vec<(u32, Date)>,
}

impl Storylines {
    pub(crate) fn rng(&mut self, seed: u32) -> &mut Rng {
        self.rng.get_or_insert_with(|| Rng::new(seed ^ 0x5701_11E5))
    }

    /// A line from the banks, avoiding recent repeats.
    pub(crate) fn line(&mut self, seed: u32, key: &str, f: &Fill, year: i32, said: &[&str]) -> Option<String> {
        let mut rng = self.rng.take().unwrap_or_else(|| Rng::new(seed ^ 0x5701_11E5));
        let out = self.recent.line_avoiding(&mut rng, key, f, year, said);
        self.rng = Some(rng);
        out
    }

    /// An open story's index by id.
    pub fn idx(&self, id: u32) -> Option<usize> {
        self.open.binary_search_by_key(&id, |s| s.id).ok()
    }

    /// A story by id, open or ended.
    pub fn get(&self, id: u32) -> Option<&Story> {
        match self.idx(id) {
            Some(k) => Some(&self.open[k]),
            None => self.ended.iter().rev().find(|s| s.id == id),
        }
    }

    /// The open story of a kind about a subject.
    pub fn find(&self, kind: u16, subject: u64) -> Option<usize> {
        self.open.iter().position(|s| s.kind == kind && s.subject == subject)
    }

    pub fn find_id(&self, kind: K, subject: u64) -> Option<u32> {
        self.find(kind.id(), subject).map(|k| self.open[k].id)
    }

    /// Whether yearly step `k` already ran for `year`; marks it run.
    pub(crate) fn once(&mut self, k: u8, year: i32) -> bool {
        if self.done.iter().any(|&(s, y)| s == k && y == year) {
            return false;
        }
        self.done.retain(|&(s, y)| s != k || y >= year - 1);
        self.done.push((k, year));
        true
    }

    fn count(&mut self, season: i32, kind: u16, public: bool) {
        match self.counts.iter_mut().find(|c| c.0 == season && c.1 == kind) {
            Some(c) => {
                if public {
                    c.3 += 1;
                } else {
                    c.2 += 1;
                }
            }
            None => self.counts.push((season, kind, (!public) as u32, public as u32)),
        }
    }

    /// Open stories with this member.
    pub fn with(&self, who: Who) -> impl Iterator<Item = &Story> {
        self.open.iter().filter(move |s| s.has(who))
    }

    /// Open stories about a team.
    pub fn about(&self, t: TeamKey) -> impl Iterator<Item = &Story> {
        self.open.iter().filter(move |s| s.about(t))
    }
}

/// A person's index by player id: a binary search only (people are kept
/// in id order), so a player who has left the world costs nothing.
pub(crate) fn pi(w: &World, id: u32) -> Option<usize> {
    super::stories::person_index(w, id)
}

/// Before the people who left football leave the world (`map` holds where
/// each person lands, None for those dropped), the stories they are in keep
/// their names, so a question still names a retired player.
pub(crate) fn keep_names(w: &mut World, map: &[Option<usize>]) {
    let st = &w.storylines;
    let mut wanted: Vec<u32> = st
        .open
        .iter()
        .chain(&st.ended)
        .flat_map(|s| {
            s.cast.iter().filter_map(|m| match m.who {
                Who::Player(id) if !s.names.iter().any(|x| x.0 == m.who) => Some(id),
                _ => None,
            })
        })
        .collect();
    wanted.sort_unstable();
    wanted.dedup();
    let gone: std::collections::HashMap<u32, String> = wanted
        .into_iter()
        .filter_map(|id| {
            let i = pi(w, id)?;
            map.get(i)?.is_none().then(|| (id, w.people[i].name.clone()))
        })
        .collect();
    if gone.is_empty() {
        return;
    }
    let st = &mut w.storylines;
    for s in st.open.iter_mut().chain(st.ended.iter_mut()) {
        for k in 0..s.cast.len() {
            let who = s.cast[k].who;
            if let Who::Player(id) = who
                && let Some(n) = gone.get(&id)
                && !s.names.iter().any(|x| x.0 == who)
            {
                s.names.push((who, n.clone()));
            }
        }
    }
}

/// A team as a number, for subjects.
pub fn tcode(t: TeamKey) -> u64 {
    match t {
        TeamKey::Hs(h) => 1 << 28 | h as u64,
        TeamKey::College(p) => 2 << 28 | p as u64,
        TeamKey::Nfl(n) => 3 << 28 | n as u64,
        TeamKey::Other(f) => 4 << 28 | f as u64,
    }
}

/// A subject made of two parts (a team and a season, two teams).
pub fn subj(a: u64, b: u64) -> u64 {
    a << 32 | (b & 0xFFFF_FFFF)
}

/// The folk module's team for a story team.
pub fn folk_team(t: TeamKey) -> super::folk::Team {
    match t {
        TeamKey::Hs(h) => super::folk::Team::High(h as usize),
        TeamKey::College(p) => super::folk::Team::College(p as usize),
        TeamKey::Nfl(n) => super::folk::Team::Nfl(n as usize),
        TeamKey::Other(f) => super::folk::Team::Other(f as usize),
    }
}

pub fn team_key(t: super::folk::Team) -> TeamKey {
    match t {
        super::folk::Team::High(h) => TeamKey::Hs(h as u32),
        super::folk::Team::College(p) => TeamKey::College(p as u32),
        super::folk::Team::Nfl(n) => TeamKey::Nfl(n as u16),
        super::folk::Team::Other(f) => TeamKey::Other(f as u16),
    }
}

/// What a detector saw: everything a new story needs, or a beat for an
/// open one about the same subject.
#[derive(Clone, Debug)]
pub(crate) struct Spot {
    pub kind: K,
    pub subject: u64,
    pub team: Option<TeamKey>,
    pub other: Option<TeamKey>,
    pub cast: Vec<Member>,
    /// Added to the kind's stakes (-50 to 50): a ranked team, a title on it.
    pub stakes: i32,
    pub due: Option<Date>,
    pub nums: [i32; 4],
    pub truth: f32,
    /// A public fact that opens or feeds it, and the heat it brings.
    pub fact: Option<String>,
    pub heat: f32,
    /// Starts public even when the kind starts private (a fact everyone saw).
    pub public: bool,
    pub word: String,
}

impl Spot {
    pub fn new(kind: K, subject: u64) -> Spot {
        Spot {
            kind,
            subject,
            team: None,
            other: None,
            cast: Vec::new(),
            stakes: 0,
            due: None,
            nums: [0; 4],
            truth: 0.0,
            fact: None,
            heat: 0.0,
            public: false,
            word: String::new(),
        }
    }
    pub fn team(mut self, t: TeamKey) -> Spot {
        self.team = Some(t);
        self
    }
    pub fn other(mut self, t: TeamKey) -> Spot {
        self.other = Some(t);
        self
    }
    pub fn with(mut self, who: Who, part: Part) -> Spot {
        if !self.cast.iter().any(|m| m.who == who) {
            let side = match part {
                Part::Lead | Part::Parent => 100,
                Part::Rival => -100,
                _ => 0,
            };
            self.cast.push(Member { who, part, side });
        }
        self
    }
    pub fn lead(mut self, who: Who) -> Spot {
        // Already in the cast in another part (the head coach as the
        // decider of his team's story): he leads it.
        if let Some(m) = self.cast.iter_mut().find(|m| m.who == who) {
            m.part = Part::Lead;
            m.side = 100;
            return self;
        }
        self.with(who, Part::Lead)
    }
    pub fn rival(self, who: Who) -> Spot {
        self.with(who, Part::Rival)
    }
    pub fn stakes(mut self, by: i32) -> Spot {
        self.stakes += by;
        self
    }
    pub fn due(mut self, d: Date) -> Spot {
        self.due = Some(d);
        self
    }
    pub fn nums(mut self, n: [i32; 4]) -> Spot {
        self.nums = n;
        self
    }
    pub fn truth(mut self, t: f32) -> Spot {
        self.truth = t.clamp(-1.0, 1.0);
        self
    }
    pub fn fact(mut self, text: String, heat: f32) -> Spot {
        self.fact = Some(text);
        self.heat = heat;
        self
    }
    pub fn public(mut self) -> Spot {
        self.public = true;
        self
    }
    pub fn word(mut self, w: impl Into<String>) -> Spot {
        self.word = w.into();
        self
    }
}

/// The days a kind rests for a subject after it ends.
fn rest_days(def: &Def) -> i32 {
    (def.quiet as i32 * 2).clamp(30, 400)
}

/// The most stories a person can lead at once, and the most open stories
/// the world holds (the far, cold ones fizzle first past it).
const PER_LEAD: usize = 3;
const MAX_OPEN: usize = 7000;

/// Opens a story for what a detector saw, or adds the fact to the open one
/// about the same subject. Returns the story's id.
pub(crate) fn notice(w: &mut World, s: Spot) -> Option<u32> {
    let kind = s.kind.id();
    let def = s.kind.def();
    let d = w.today;
    if let Some(k) = w.storylines.find(kind, s.subject) {
        let id = w.storylines.open[k].id;
        {
            let st = &mut w.storylines.open[k];
            st.truth = 0.7 * st.truth + 0.3 * s.truth;
            if s.due.is_some() {
                st.due = s.due;
            }
            for (a, b) in st.nums.iter_mut().zip(s.nums) {
                if b != 0 {
                    *a = b;
                }
            }
            for m in s.cast {
                if !st.cast.iter().any(|x| x.who == m.who) && st.cast.len() < 8 {
                    st.cast.push(m);
                }
            }
        }
        if let Some(text) = s.fact {
            add_beat(w, id, BeatKind::Fact, Voice::World, 255, s.heat, text);
            if s.public {
                go_public(w, id, d);
            }
        }
        return Some(id);
    }
    if w.storylines.cool.iter().any(|c| c.0 == kind && c.1 == s.subject && c.2 > d) {
        return None;
    }
    // One story of a shape per lead at a time, a few stories per person.
    if let Some(lead) = s.cast.iter().find(|m| m.part == Part::Lead).map(|m| m.who) {
        let n = w.storylines.open.iter().filter(|x| x.lead() == Some(lead)).count();
        if n >= PER_LEAD {
            return None;
        }
    }
    let level = match (s.team, def.levels) {
        (_, l) if l == catalogue::REC => Level::Recruiting,
        (Some(t), _) => t.level(),
        (None, _) => [Level::College, Level::Nfl, Level::HighSchool, Level::Other]
            .into_iter()
            .find(|&l| def.at(l))
            .unwrap_or(Level::College),
    };
    let stakes = (def.stakes as i32 + s.stakes).clamp(5, 100) as u8;
    let (fame, market) = (fame(w, &s.cast), market(w, s.team));
    let sides = default_sides(def);
    let public = def.public || s.public;
    let id = {
        let st = &mut w.storylines;
        st.next += 1;
        st.next
    };
    let season = w.year;
    let story = Story {
        id,
        kind,
        subject: s.subject,
        level,
        team: s.team,
        other: s.other,
        cast: s.cast,
        stakes,
        truth: s.truth,
        belief: 0.0,
        sure: if public { 0.35 } else { 0.0 },
        sides,
        heat: 0.0,
        peak_heat: 0.0,
        weeks: Vec::new(),
        phase: if public { Phase::Spark } else { Phase::Seed },
        public,
        opened: d,
        public_on: public.then_some(d),
        last_beat: d,
        due: s.due,
        closed: None,
        beats: Vec::new(),
        links: Vec::new(),
        ending: 0,
        near: false,
        you: None,
        handled: 0.0,
        moves: 0,
        nums: s.nums,
        season,
        fame,
        market,
        legacy: String::new(),
        stance: 0,
        word: s.word,
        said: 0,
        names: Vec::new(),
    };
    w.storylines.open.push(story);
    w.storylines.count(season, kind, false);
    if public {
        w.storylines.count(season, kind, true);
    }
    let k = w.storylines.open.len() - 1;
    let you = your_part(w, &w.storylines.open[k]);
    let near = you.is_some() || near(w, &w.storylines.open[k]);
    {
        let st = &mut w.storylines.open[k];
        st.you = you;
        st.near = near;
    }
    link(w, id);
    // The opening: a public fact everyone saw, or the detector's private look.
    // A fact told word for word this season gives way to the bank's line.
    let fact = s.fact.filter(|t| !told(w, id, t));
    let start = 0.25 * stakes as f32 * amp_of(&w.storylines.open[k], slow_news(d));
    if let Some(st) = w.storylines.open.get_mut(k) {
        st.heat = start.min(60.0);
        st.peak_heat = st.heat;
    }
    match fact {
        Some(text) => {
            add_beat(w, id, if public { BeatKind::Spark } else { BeatKind::Seed }, Voice::World, 255, s.heat, text)
        }
        None if public => {
            let text = tell::text(w, id, BeatKind::Spark, Voice::World);
            add_beat(w, id, BeatKind::Spark, Voice::World, 255, s.heat.max(BeatKind::Spark.heat()), text);
        }
        None => add_beat(w, id, BeatKind::Seed, Voice::Staff, 255, 0.0, String::new()),
    }
    if public && w.storylines.open.get(k).is_some_and(|x| x.near || x.heat >= 40.0) {
        tell::post_beat(w, id);
    }
    if you.is_some() {
        moves::opened(w, id);
    }
    if w.storylines.open.len() > MAX_OPEN {
        crowd_out(w, d);
    }
    Some(id)
}

/// Whether a line was told this season (or the one beside it, since a
/// story's season turns after the Super Bowl) by another story of the same
/// kind and subject or with the same lead: a person hears a line once.
fn told(w: &World, id: u32, text: &str) -> bool {
    let Some(s) = w.storylines.get(id) else { return false };
    let lead = s.lead();
    w.storylines.open.iter().chain(&w.storylines.ended).any(|o| {
        o.season.abs_diff(s.season) <= 1
            && o.id != id
            && ((o.kind == s.kind && o.subject == s.subject) || lead.is_some() && o.lead() == lead)
            && o.beats.iter().any(|b| b.text == text)
    })
}

/// Adds a world fact to an open story (a game, a filing, a report).
pub(crate) fn fact(w: &mut World, id: u32, text: String, heat: f32) {
    add_beat(w, id, BeatKind::Fact, Voice::World, 255, heat, text);
}

/// The open story's id for a kind and subject.
pub(crate) fn open_id(w: &World, kind: K, subject: u64) -> Option<u32> {
    w.storylines.find_id(kind, subject)
}

/// How much a story's beats count: stakes, fame, market and the season.
fn amp_of(s: &Story, slow: f32) -> f32 {
    (0.55 + s.stakes as f32 / 110.0) * (0.7 + 0.6 * s.fame) * (0.7 + 0.6 * s.market) * slow
}

/// The summer dead zone makes small stories big (June 1 to July 20).
pub(crate) fn slow_news(d: Date) -> f32 {
    let (m, day) = (d.month(), d.day());
    if m == 6 || (m == 7 && day <= 20) { 1.45 } else { 1.0 }
}

/// Records a beat: heat, the public's belief, links' heat, your inbox.
pub(crate) fn add_beat(w: &mut World, id: u32, what: BeatKind, by: Voice, who: u8, heat: f32, text: String) {
    let d = w.today;
    let slow = slow_news(d);
    let Some(k) = w.storylines.idx(id) else { return };
    let gain = {
        let s = &mut w.storylines.open[k];
        let gain = heat * amp_of(s, slow);
        s.heat = (s.heat + gain).clamp(0.0, 100.0);
        s.peak_heat = s.peak_heat.max(s.heat);
        if what.news() {
            s.last_beat = d;
        }
        if s.phase == Phase::Spark && what.public() && what != BeatKind::Spark && s.public_on.is_some_and(|p| p < d) {
            s.phase = Phase::Building;
        }
        // Detail follows distance: far stories keep their key beats only.
        let key = matches!(what, BeatKind::Spark | BeatKind::Peak | BeatKind::Resolve | BeatKind::Fact);
        let keep = s.near || s.you.is_some() || key || s.heat >= 70.0;
        if keep && !(text.is_empty() && what == BeatKind::Seed && !s.beats.is_empty()) {
            let text = if s.near || s.heat >= 55.0 || what != BeatKind::Fact { text } else { String::new() };
            s.beats.push(Beat { date: d, what, by, who, heat: gain.round().clamp(-100.0, 100.0) as i8, text });
            let cap = if s.near || s.you.is_some() { 160 } else { 8 };
            if s.beats.len() > cap {
                // Keep the first beat (how it started) and the newest.
                s.beats.remove(1);
            }
        }
        gain
    };
    // Links: a share of the heat flows to the stories it touches.
    if gain > 0.5 {
        let links = w.storylines.open[k].links.clone();
        for l in links {
            if let Some(j) = w.storylines.idx(l) {
                let o = &mut w.storylines.open[j];
                o.heat = (o.heat + 0.1 * gain).min(100.0);
                o.peak_heat = o.peak_heat.max(o.heat);
            }
        }
    }
    if what.public() && w.storylines.followed.contains(&id) {
        credit::follow_note(w, id);
    }
}

/// A private story goes public through a person.
pub(crate) fn go_public(w: &mut World, id: u32, d: Date) {
    let Some(k) = w.storylines.idx(id) else { return };
    let (season, kind) = {
        let s = &mut w.storylines.open[k];
        if s.public {
            return;
        }
        s.public = true;
        s.public_on = Some(d);
        s.sure = s.sure.max(0.3);
        if s.phase == Phase::Seed {
            s.phase = Phase::Spark;
        }
        (s.season, s.kind)
    };
    w.storylines.count(season, kind, true);
}

/// Closes a story with an ending (`Def` ending key) and its resolving fact.
pub(crate) fn close(w: &mut World, id: u32, ending: &str, text: Option<String>) {
    let d = w.today;
    let Some(k) = w.storylines.idx(id) else { return };
    let code = {
        let s = &mut w.storylines.open[k];
        if !s.phase.open() {
            return;
        }
        let code = s.def().ending(ending);
        s.ending = code;
        s.closed = Some(d);
        code
    };
    let text = match text {
        Some(t) if !told(w, id, &t) => t,
        _ => tell::text(w, id, BeatKind::Resolve, Voice::World),
    };
    let public = w.storylines.open[k].public;
    add_beat(w, id, BeatKind::Resolve, Voice::World, 255, if public { BeatKind::Resolve.heat() } else { 0.0 }, text);
    let Some(k) = w.storylines.idx(id) else { return };
    {
        let s = &mut w.storylines.open[k];
        s.phase = Phase::Resolved;
        s.ending = code;
        let def = s.def();
        let rest = rest_days(def);
        let (kind, subject) = (s.kind, s.subject);
        w.storylines.cool.push((kind, subject, d.add(rest)));
    }
    if public {
        tell::post_beat(w, id);
    }
    tell::legacy(w, id);
    moves::settle(w, id);
    effects::ended(w, id);
    credit::ended(w, id);
}

/// A story dies without an answer.
fn fizzle(w: &mut World, id: u32, d: Date) {
    let Some(k) = w.storylines.idx(id) else { return };
    let public = w.storylines.open[k].public;
    if public && w.storylines.open[k].near {
        let text = tell::text(w, id, BeatKind::Fizzle, Voice::Beat);
        add_beat(w, id, BeatKind::Fizzle, Voice::Beat, 255, 0.0, text);
    }
    let Some(k) = w.storylines.idx(id) else { return };
    let s = &mut w.storylines.open[k];
    s.phase = Phase::Fizzled;
    s.closed = Some(d);
    let rest = rest_days(s.def());
    let (kind, subject) = (s.kind, s.subject);
    w.storylines.cool.push((kind, subject, d.add(rest / 2)));
    credit::ended(w, id);
}

/// The sides a kind starts with: the share backing its lead.
fn default_sides(def: &Def) -> [f32; 4] {
    match def.family {
        Family::Battle => [0.55, 0.5, 0.5, 0.5],
        Family::Seat => [0.75, 0.4, 0.4, 0.45],
        Family::Wanted => [0.6, 0.55, 0.5, 0.6],
        Family::Leaving => [0.5, 0.4, 0.45, 0.4],
        Family::OffField => [0.6, 0.45, 0.35, 0.45],
        Family::You => [0.7, 0.55, 0.5, 0.55],
        _ => [0.6, 0.6, 0.55, 0.6],
    }
}

/// The cast's fame, 0-1: the best known among them.
fn fame(w: &World, cast: &[Member]) -> f32 {
    let mut best = 0.0f32;
    for m in cast {
        let f = match m.who {
            Who::Player(id) => pi(w, id).map_or(0.0, |i| player_fame(w, i)),
            Who::Coach(c) => w.coaches.get(c as usize).map_or(0.0, |c| (c.reputation as f32 / 20.0).clamp(0.0, 1.0)),
            _ => 0.0,
        };
        best = best.max(f);
    }
    best
}

/// A player's fame, 0-1: followers, stars and a pro's standing.
pub(crate) fn player_fame(w: &World, i: usize) -> f32 {
    let p = &w.people[i];
    let followers = ((p.followers as f32 + 1.0).log10() / 6.5).clamp(0.0, 1.0);
    let stars = p.stars as f32 / 5.0;
    let pro = if p.pro.is_some() { 0.15 } else { 0.0 };
    (0.55 * followers + 0.3 * stars + pro + 0.2 * p.brand.clamp(0.0, 1.0)).clamp(0.0, 1.0)
}

/// A team's market, 0-1: how many people follow it.
pub(crate) fn market(w: &World, t: Option<TeamKey>) -> f32 {
    let Some(t) = t else { return 0.4 };
    match t {
        TeamKey::College(p) => w
            .programs
            .get(p as usize)
            .map_or(0.3, |x| (0.55 * x.prestige_rel + 0.25 * x.fans.clamp(0.0, 1.0) + 0.2 * x.prestige) as f32),
        TeamKey::Nfl(n) => {
            let city = super::finance::nfl::city(w, n as usize, w.year).0;
            let m = (super::finance::nfl::metro(&city) / 20.0).sqrt() as f32;
            let brand = super::finance::data::BRAND.get(n as usize).copied().unwrap_or(0.5) as f32;
            (0.45 + 0.3 * m + 0.25 * brand).clamp(0.0, 1.0)
        }
        TeamKey::Hs(h) => w
            .high_schools
            .get(h as usize)
            .map_or(0.1, |s| (0.08 + 0.3 * s.prestige as f32 + if s.national { 0.15 } else { 0.0 }).clamp(0.0, 0.6)),
        TeamKey::Other(_) => 0.2,
    }
}

/// Your part in a story, if you are in it: the lead of your own stories,
/// the decider of your team's, the rival of a rival's.
fn your_part(w: &World, s: &Story) -> Option<Part> {
    let me = super::stories::seat_coach(w).map(|c| c as u32);
    if let Some(c) = me
        && let Some(m) = s.cast.iter().find(|m| m.who == Who::Coach(c))
    {
        return Some(m.part);
    }
    team_part(s, super::stories::seat_team(w)?)
}

/// A team's part in a story it is not cast in: the decider of its own
/// stories, but a suitor when a recruit decides; the other side of a rival's.
pub(crate) fn team_part(s: &Story, team: TeamKey) -> Option<Part> {
    if s.team == Some(team) {
        return Some(if s.family() == Family::Recruiting { Part::Suitor } else { Part::Decider });
    }
    if s.other == Some(team) {
        return Some(Part::Rival);
    }
    None
}

/// Whether a story is around you: your team, your rivals, your conference,
/// your recruits, your former players.
fn near(w: &World, s: &Story) -> bool {
    if s.heat >= 70.0 {
        return true;
    }
    let Some(me) = super::stories::seat_team(w) else { return false };
    if s.about(me) {
        return true;
    }
    let close = |t: TeamKey| match (me, t) {
        (TeamKey::College(a), TeamKey::College(b)) => {
            let (a, b) = (a as usize, b as usize);
            w.programs.get(a).zip(w.programs.get(b)).is_some_and(|(x, y)| x.conference == y.conference)
                || w.college.as_ref().is_some_and(|c| c.rivalry_of.contains_key(&(a.min(b), a.max(b))))
        }
        (TeamKey::Nfl(a), TeamKey::Nfl(b)) => super::franchise::alignment(a as usize, w.year)
            .is_some_and(|da| super::franchise::alignment(b as usize, w.year).map(|x| x.1) == Some(da.1)),
        (TeamKey::Hs(a), TeamKey::Hs(b)) => {
            w.high_schools.get(a as usize).zip(w.high_schools.get(b as usize)).is_some_and(|(x, y)| x.state == y.state)
        }
        _ => false,
    };
    if s.team.is_some_and(close) || s.other.is_some_and(close) {
        return true;
    }
    // Your recruits and your former players.
    let listed = |id: u32| w.career.as_ref().is_some_and(|c| c.desk.is_listed(id));
    s.cast.iter().any(|m| match m.who {
        Who::Player(id) => listed(id) || w.stories.story.players.contains(&id),
        _ => false,
    })
}

/// Which kinds feed which (the design's chaining): a losing streak feeds
/// the hot seat, the hot seat a commitment's wobble.
fn links_to(kind: K) -> &'static [K] {
    use K::*;
    match kind {
        LosingStreak => &[HotSeat, FanRevolt, CommitWobble, VoteOfConfidence, BuyoutWatch],
        HotSeat => &[LosingStreak, BuyoutWatch, VoteOfConfidence, FanRevolt, CommitWobble, JobRumors, CoachingSearch],
        VoteOfConfidence | BuyoutWatch | FanRevolt => &[HotSeat, CoachingSearch],
        CoachingSearch => &[HotSeat, InterimAudition, NflWantsCoach, BigJobWantsCoach, PortalExodus, NewCoach],
        NflWantsCoach | BigJobWantsCoach | JobRumors => &[CoachingSearch, CommitWobble, StaffRaid],
        QbBattle | QbControversy => &[PortalDecision, FreshmanStarter, NextManUp],
        Arrest | SocialStorm => &[TeamRules, LeagueSuspension, SuspensionReturn, RedFlag],
        TeamRules | LeagueSuspension => &[Arrest, SuspensionReturn],
        Holdout | TagStandoff => &[TradeRequest, ContractYear],
        TradeRequest => &[Holdout, StarTraded],
        TopRecruit | RecruitWar | HometownLeaving => &[SigningFlip, CommitWobble, ClassRace],
        SigningFlip | CommitWobble => &[TopRecruit, RecruitWar, HotSeat],
        Unbeaten | PlayoffPush | ConferenceRace => &[AwardRace, Unbeaten, PlayoffPush, ConferenceRace],
        RecordChase | AwardRace | CareerRecord => &[AwardRace, Unbeaten],
        PortalExodus => &[PortalDecision, CoachingSearch, NewCoach],
        StockRise | StockFall | CombineRiser | RedFlag => &[DraftSlide, TopPickDebate, EarlyEntry],
        Decline | Graveyard => &[HotSeat, FanRevolt],
        NcaaCase => &[HotSeat, PortalExodus],
        RoomRift | PublicFeud => &[HotSeat, TradeRequest],
        _ => &[],
    }
}

/// Links a new story to the open ones it touches: a linked kind about the
/// same team or person.
fn link(w: &mut World, id: u32) {
    let Some(k) = w.storylines.idx(id) else { return };
    let s = &w.storylines.open[k];
    let Some(kind) = K::ALL.get(s.kind as usize).copied() else { return };
    let targets = links_to(kind);
    let people: Vec<Who> = s.cast.iter().filter(|m| m.part != Part::Other).map(|m| m.who).collect();
    let team = s.team;
    let mut found = Vec::new();
    for o in &w.storylines.open {
        if o.id == id || !o.phase.open() || found.len() >= 6 {
            continue;
        }
        let ok_kind = targets.iter().any(|t| t.id() == o.kind)
            || K::ALL.get(o.kind as usize).is_some_and(|ok| links_to(*ok).contains(&kind));
        let shares = team.is_some() && o.team == team || people.iter().any(|p| o.has(*p));
        if ok_kind && shares {
            found.push(o.id);
        }
    }
    for o in found {
        if let Some(j) = w.storylines.idx(o)
            && !w.storylines.open[j].links.contains(&id)
        {
            w.storylines.open[j].links.push(id);
        }
        if let Some(k) = w.storylines.idx(id)
            && !w.storylines.open[k].links.contains(&o)
        {
            w.storylines.open[k].links.push(o);
        }
    }
}

/// Past the world's open limit, the coldest far stories fizzle.
fn crowd_out(w: &mut World, d: Date) {
    let mut cold: Vec<(f32, u32)> = w
        .storylines
        .open
        .iter()
        .filter(|s| s.phase.open() && !s.near && s.you.is_none())
        .map(|s| (s.heat, s.id))
        .collect();
    cold.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let over = w.storylines.open.len().saturating_sub(MAX_OPEN - 200);
    for (_, id) in cold.into_iter().take(over) {
        fizzle(w, id, d);
    }
    archive(w, d);
}

/// Each audience's daily attention: how many stories it talks about.
fn audience_cap(level: Level) -> usize {
    match level {
        Level::College => 30,
        Level::Nfl => 20,
        Level::Recruiting => 15,
        Level::HighSchool => 6,
        Level::Other => 5,
        Level::World => 4,
    }
}

/// Ranks today's public stories in each audience: the national front page,
/// each level (each state for high schools), each team's own beat. A story
/// in any audience's top gets coverage today.
fn rank(w: &mut World) {
    use std::collections::BTreeMap;
    let mut pools: BTreeMap<u64, Vec<(f32, u32)>> = BTreeMap::new();
    for s in w.storylines.open.iter().filter(|s| s.public && s.phase.open()) {
        let e = (s.heat, s.id);
        pools.entry(0).or_default().push(e);
        let lvl = match (s.level, s.team) {
            (Level::HighSchool, Some(TeamKey::Hs(h))) => {
                let st = w.high_schools.get(h as usize).map_or(0, |x| x.state as u64);
                0x100 | st
            }
            (l, _) => 0x200 | l as u64,
        };
        pools.entry(lvl).or_default().push(e);
        if let Some(t) = s.team {
            pools.entry(1 << 40 | tcode(t)).or_default().push(e);
        }
    }
    let mut on: Vec<u32> = Vec::new();
    for (key, mut v) in pools {
        let cap = match key {
            0 => 10,
            k if k >> 40 == 1 => 3,
            k if k & 0xF00 == 0x100 => audience_cap(Level::HighSchool),
            k => {
                let l = match k & 0xFF {
                    0 => Level::Nfl,
                    1 => Level::College,
                    2 => Level::HighSchool,
                    3 => Level::Recruiting,
                    4 => Level::World,
                    _ => Level::Other,
                };
                audience_cap(l)
            }
        };
        v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        on.extend(v.into_iter().take(cap).map(|x| x.1));
    }
    on.sort_unstable();
    on.dedup();
    w.storylines.attended = on;
}

/// Once a day, after the day's other systems (called from the clock).
pub(crate) fn day(w: &mut World, d: Date) {
    credit::desk_answers(w);
    if !w.recording {
        return;
    }
    detect::day(w, d);
    rank(w);
    let ids: Vec<u32> = w.storylines.open.iter().map(|s| s.id).collect();
    for id in ids {
        step(w, d, id);
    }
    if d.weekday() == Weekday::Mon {
        weekly(w, d);
    }
    moves::day(w, d);
    credit::day(w, d);
}

/// One story's day: heat fades, a seed may spark, the cast and the voices
/// act, a due date forces the peak, a quiet story fizzles.
fn step(w: &mut World, d: Date, id: u32) {
    let Some(k) = w.storylines.idx(id) else { return };
    let (phase, public, attended, near, heat, last, quiet, due, closed) = {
        let s = &w.storylines.open[k];
        (
            s.phase,
            s.public,
            w.storylines.attended.binary_search(&id).is_ok(),
            s.near || s.you.is_some(),
            s.heat,
            s.last_beat,
            s.def().quiet as i32,
            s.due,
            s.closed,
        )
    };
    if !phase.open() {
        let s = &mut w.storylines.open[k];
        s.heat *= 0.9;
        if phase == Phase::Resolved && closed.is_some_and(|c| c.days_to(d) >= 21) {
            s.phase = Phase::Aftermath;
        }
        return;
    }
    {
        let s = &mut w.storylines.open[k];
        s.heat *= if attended {
            0.965
        } else if public {
            0.93
        } else {
            0.95
        };
    }
    // A private story goes public only through a person.
    if !public {
        tell::maybe_spark(w, id);
    }
    // The peak: the day the question gets forced.
    if due.is_some_and(|x| x <= d) && phase < Phase::Peak {
        if let Some(s) = w.storylines.idx(id).map(|k| &mut w.storylines.open[k]) {
            s.phase = Phase::Peak;
        }
        let text = tell::text(w, id, BeatKind::Peak, Voice::Beat);
        add_beat(w, id, BeatKind::Peak, Voice::Beat, 255, if public { BeatKind::Peak.heat() } else { 0.0 }, text);
        if public && (attended || near) {
            tell::post_beat(w, id);
        }
    } else if public && (attended || near) {
        // The voices: a beat writer, a column, the radio, the fans, the cast.
        let p = if attended { 0.04 + heat / 380.0 } else { 0.025 };
        let roll = w.storylines.rng(w.options.seed).next() as f32;
        if roll < p {
            tell::voice_beat(w, id, attended);
        }
    }
    // Your staff's memo on your own stories, about once a week.
    if near && w.storylines.open.get(k).is_some_and(|s| s.you.is_some()) {
        tell::maybe_memo(w, id);
    }
    // Quiet for too long: it fizzles (a due date keeps it alive).
    let still = w.storylines.idx(id).is_some_and(|k| w.storylines.open[k].phase.open());
    if still && last.days_to(d) > quiet && due.is_none_or(|x| x.days_to(d) > 7) {
        fizzle(w, id, d);
    }
}

/// Mondays: heat for the record, distance and fame looked at again, the
/// effects of the week, and the store kept small.
fn weekly(w: &mut World, d: Date) {
    let n = w.storylines.open.len();
    for k in 0..n {
        let (open, near0) = {
            let s = &w.storylines.open[k];
            (s.phase.open(), s.near)
        };
        let you = your_part(w, &w.storylines.open[k]);
        let near = you.is_some() || near(w, &w.storylines.open[k]);
        let fame = fame(w, &w.storylines.open[k].cast);
        let market = market(w, w.storylines.open[k].team);
        let s = &mut w.storylines.open[k];
        if open {
            s.weeks.push(s.heat.round().clamp(0.0, 100.0) as u8);
            if s.weeks.len() > 60 {
                s.weeks.remove(0);
            }
        }
        s.you = you.or(s.you);
        s.near = near || (near0 && !open);
        s.fame = fame.max(0.85 * s.fame);
        s.market = market;
    }
    effects::week(w, d);
    archive(w, d);
}

/// Moves finished stories out of the open list (aftermath over, fizzled)
/// and keeps the ended list small: your stories for good, the rest for two
/// seasons, each cut to its key beats.
fn archive(w: &mut World, d: Date) {
    let year = w.year;
    let mut gone = Vec::new();
    w.storylines.open.retain(|s| {
        let done = match s.phase {
            Phase::Fizzled => true,
            Phase::Aftermath | Phase::Resolved => {
                s.closed.is_some_and(|c| c.days_to(d) >= if s.you.is_some() || s.near { 120 } else { 45 })
            }
            _ => false,
        };
        if done {
            gone.push(s.clone());
        }
        !done
    });
    for mut s in gone {
        let id = s.id;
        compact(&mut s);
        let keep = s.you.is_some() || s.phase != Phase::Fizzled || s.peak_heat >= 40.0;
        if keep {
            // Its players' names as they are today, beside those it kept for
            // anyone who already left the world.
            for m in s.cast.iter().filter(|m| matches!(m.who, Who::Player(_))) {
                if !s.names.iter().any(|x| x.0 == m.who)
                    && let Some(n) = credit::member_name(w, m.who)
                {
                    s.names.push((m.who, n));
                }
            }
            w.storylines.ended.push(s);
        }
        w.storylines.followed.retain(|f| *f != id);
    }
    w.storylines.ended.retain(|s| s.you.is_some() || s.season >= year - 2 || s.peak_heat >= 85.0);
    // A resting kind's day passed.
    w.storylines.cool.retain(|c| c.2 > d);
}

/// An ended story, cut to what it needs: its key beats and text.
fn compact(s: &mut Story) {
    if s.you.is_some() {
        s.beats.retain(|b| b.what != BeatKind::Seed);
        return;
    }
    s.beats.retain(|b| {
        matches!(b.what, BeatKind::Spark | BeatKind::Peak | BeatKind::Resolve | BeatKind::Fact) && !b.text.is_empty()
    });
    while s.beats.len() > 4 {
        s.beats.remove(1);
    }
    s.cast.truncate(4);
    s.weeks.clear();
    s.weeks.shrink_to_fit();
    s.beats.shrink_to_fit();
    s.cast.shrink_to_fit();
    s.links.clear();
}

/// The world's open stories at a level, hottest first (trials and the board).
pub fn hottest(w: &World, level: Option<Level>, n: usize) -> Vec<&Story> {
    let mut v: Vec<&Story> =
        w.storylines.open.iter().filter(|s| s.public && s.phase.open() && level.is_none_or(|l| s.level == l)).collect();
    v.sort_by(|a, b| b.heat.total_cmp(&a.heat).then(a.id.cmp(&b.id)));
    v.truncate(n);
    v
}
