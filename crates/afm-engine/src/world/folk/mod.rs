//! Section 6, the people side built in the final build (lane 6): the people
//! around a team as persons (bosses, boosters, reporters, the support
//! staff), your first two weeks in a new job (taking over, T1-T23), talks,
//! press conferences, team rules and discipline, meetings with your boss,
//! your network (N1-N7), and the world's people leftovers from build 9
//! (friends, mentors and family ties, full morale, pros' and coaches' lives,
//! courts and appeals, AI team rules, every assistant as a person, staff
//! contracts, agents and permission rules, league discipline, medical
//! disputes, star power and rookie rituals).
//!
//! Memory: the people around a team are drawn from a stable key when a
//! screen or a review needs them (name, age, traits, term), never stored.
//! What is stored is small: your own relations (trust per person you know),
//! the scenes waiting for you, the takeover file, sparse ties between
//! players (a few per player, pairs of ids) and compact high school staffs.
//! Every size no real source publishes is Claude's pick (proposed, not
//! confirmed), listed in `docs/world-model.md`.

pub mod backchannel;
pub mod boss;
pub mod cast;
pub mod courts;
pub mod facts;
pub mod letters;
pub mod life;
pub mod network;
pub mod press;
pub mod rules;
pub mod scene;
pub mod staffing;
pub mod takeover;
pub mod talks;
pub mod ties;
pub mod tips;
pub mod voice;

use super::World;
use crate::calendar::{Date, Event, Kind, Level, Loudness};

/// A team any person can belong to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Team {
    College(usize),
    Nfl(usize),
    High(usize),
    /// A team in another pro league (by franchise, `pro::other`).
    Other(usize),
}

impl Team {
    /// Your team, when you have a seat.
    pub fn seat(w: &World) -> Option<Team> {
        let s = w.seat?;
        Some(match s.level {
            Level::Nfl => Team::Nfl(s.nfl_team?),
            Level::HighSchool => Team::High(s.program?),
            Level::Other => Team::Other(s.other?),
            _ => Team::College(s.program?),
        })
    }

    pub fn name(self, w: &World) -> String {
        match self {
            Team::College(p) => w.program_name(p).to_string(),
            Team::Nfl(t) => w.nfl_team_name(t),
            Team::High(h) => w.school_name(h).to_string(),
            Team::Other(f) => super::pro::other::name(f, w.year).to_string(),
        }
    }

    /// The team's name as a link (screens topic 17).
    pub fn link(self, w: &World) -> String {
        let (k, i) = self.parts();
        format!("[[t:{k}:{i}|{}]]", self.name(w))
    }

    /// The screens' kind and index: college, nfl or hs.
    pub fn parts(self) -> (&'static str, usize) {
        match self {
            Team::College(p) => ("college", p),
            Team::Nfl(t) => ("nfl", t),
            Team::High(h) => ("hs", h),
            Team::Other(f) => ("other", f),
        }
    }

    pub fn key(self) -> u64 {
        match self {
            Team::College(p) => 0x1_0000_0000 | p as u64,
            Team::Nfl(t) => 0x2_0000_0000 | t as u64,
            Team::High(h) => 0x3_0000_0000 | h as u64,
            Team::Other(f) => 0x4_0000_0000 | f as u64,
        }
    }

    pub fn level(self) -> Level {
        match self {
            Team::College(_) => Level::College,
            Team::Nfl(_) | Team::Other(_) => Level::Nfl,
            Team::High(_) => Level::HighSchool,
        }
    }

    /// The coach employer this team is.
    pub fn employer(self) -> super::coach::Employer {
        match self {
            Team::College(p) => super::coach::Employer::College(p),
            Team::Nfl(t) => super::coach::Employer::Nfl(t),
            Team::High(h) => super::coach::Employer::HighSchool(h),
            Team::Other(f) => super::coach::Employer::Other(f),
        }
    }

    pub fn from_employer(e: super::coach::Employer) -> Team {
        match e {
            super::coach::Employer::College(p) => Team::College(p),
            super::coach::Employer::Nfl(t) => Team::Nfl(t),
            super::coach::Employer::HighSchool(h) => Team::High(h),
            super::coach::Employer::Other(f) => Team::Other(f),
        }
    }

    /// Its head coach now.
    pub fn head_coach(self, w: &World) -> Option<usize> {
        match self {
            Team::College(p) => w.programs.get(p)?.head_coach,
            Team::Nfl(t) => w.nfl.get(t)?.head_coach,
            Team::High(h) => w.high_schools.get(h)?.coach,
            Team::Other(f) => super::pro::other::head_coach(w, f),
        }
    }

    /// Everyone on its roster (person indices), leaving out players who
    /// walked away.
    pub fn roster(self, w: &World) -> Vec<usize> {
        let v: &[usize] = match self {
            Team::College(p) => &w.programs[p].roster,
            Team::Nfl(t) => w.clubs.get(t).map_or(&[], |c| &c.roster),
            Team::High(h) => &w.high_schools[h].roster,
            Team::Other(f) => {
                return w.others.teams.get(f).map_or(Vec::new(), |t| {
                    t.roster
                        .iter()
                        .map(|&k| k as usize)
                        .filter(|&k| w.people.get(k).is_some_and(|p| p.stage != super::Stage::Done))
                        .collect()
                });
            }
        };
        v.iter().copied().filter(|&k| w.people.get(k).is_some_and(|p| p.stage != super::Stage::Done)).collect()
    }

    /// The state it is in (an NFL club: its market's state when known).
    pub fn state(self, w: &World) -> Option<usize> {
        match self {
            Team::College(p) => Some(w.programs[p].state),
            Team::High(h) => Some(w.high_schools[h].state),
            Team::Nfl(t) => super::franchise::era(t, w.year).and_then(|e| super::geo::state_index(e.state)),
            Team::Other(_) => None,
        }
    }

    /// The town or city it plays in.
    pub fn town(self, w: &World) -> String {
        match self {
            Team::College(p) => w.programs[p].city.clone(),
            Team::High(h) => w.high_schools[h].town.clone(),
            Team::Nfl(t) => super::franchise::era(t, w.year).map_or_else(String::new, |e| e.city.to_string()),
            Team::Other(f) => super::pro::other::city(f, w.year).to_string(),
        }
    }
}

/// A player's name as a link to his page.
pub fn player_link(w: &World, i: usize) -> String {
    let p = &w.people[i];
    format!("[[p:{}|{}]]", p.player.id, p.name)
}

/// A coach's name as a link to his page.
pub fn coach_link(w: &World, c: usize) -> String {
    format!("[[c:{c}|{}]]", w.coaches[c].name)
}

/// Strips the link marks for text that leaves the people screens (the
/// calendar and the inbox print plain words).
pub fn plain(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(a) = rest.find("[[") {
        out.push_str(&rest[..a]);
        let after = &rest[a + 2..];
        match after.find("]]") {
            Some(b) => {
                let inner = &after[..b];
                out.push_str(inner.rsplit_once('|').map_or(inner, |x| x.1));
                rest = &after[b + 2..];
            }
            None => {
                out.push_str(&rest[a..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// A dated line in your people log: who told you what, and later how it
/// held up (T2, T3). Text keeps its link marks.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Entry {
    pub date: Option<Date>,
    /// What it is about: "roster", "staff", "boss", "boosters", "media",
    /// "place", "plan", "academics", "medical", "recruiting", "room",
    /// "flight", "you", "network", "rules", "press", "talk".
    pub topic: String,
    /// Who said it, with his role ("[[c:12|Ron Hale]], defensive coordinator").
    pub source: String,
    pub text: String,
    /// The source's interest, in words, when he has one (T3).
    pub angle: Option<String>,
    /// How it held up, once your own staff has seen enough (T3).
    pub check: Option<String>,
}

/// Your relations at your current job and in the game: trust per person
/// you know (cast people by their key, coaches by id), what you have said
/// in public, and how the room reads your discipline.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Mine {
    pub team: Option<Team>,
    pub since: Option<Date>,
    /// (person key, trust 0-1). Keys: `cast::CastRef::key` or `coach_key`.
    pub trust: Vec<(u64, f32)>,
    /// What you raised (or lowered) the public bar to at your introduction
    /// and since: -1 to 1 (T17).
    pub expectation: f32,
    /// How the world reads you: disciplinarian (+) or players' coach (-),
    /// and honest (+) or a salesman (-), each -1 to 1, built from what you do.
    pub hard: f32,
    pub straight: f32,
    /// Hours used from this week's time budget (talks, calls, trips).
    pub week: Option<Date>,
    pub hours: f32,
}

pub fn coach_key(c: usize) -> u64 {
    0xC0AC_0000_0000_0000 | c as u64
}

impl Mine {
    pub fn trust_of(&self, key: u64) -> Option<f32> {
        self.trust.iter().find(|x| x.0 == key).map(|x| x.1)
    }
    pub fn set_trust(&mut self, key: u64, v: f32) {
        let v = v.clamp(0.0, 1.0);
        match self.trust.iter_mut().find(|x| x.0 == key) {
            Some(x) => x.1 = v,
            None => self.trust.push((key, v)),
        }
    }
    pub fn nudge(&mut self, key: u64, by: f32) {
        let v = self.trust_of(key).unwrap_or(0.5) + by;
        self.set_trust(key, v);
    }
}

/// One dated mood reason: the day, the reason's code and its size in
/// hundredths.
pub type MoodNote = (Date, u8, i8);

/// The people store (one per world).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Folk {
    /// A stable seed for the people drawn from keys, set once.
    pub base: u32,
    pub mine: Mine,
    pub takeover: Option<takeover::Takeover>,
    /// Scenes waiting for you or held recently (the newest 60 kept).
    pub scenes: Vec<scene::Scene>,
    pub next_scene: u32,
    /// Your people log: everything the meetings, talks and reports told you.
    pub log: Vec<Entry>,
    pub promises: Vec<talks::Promise>,
    /// The line patterns each speaker has used with you (topic 5, part 5).
    pub said: Vec<(u64, Vec<u32>)>,
    pub rules: rules::Rulebook,
    pub cases: Vec<rules::Violation>,
    pub goals: Vec<boss::Goal>,
    pub asks: Vec<boss::Ask>,
    pub network: network::Network,
    /// Tips from people you know (SK35, C14).
    pub tips: tips::Tips,
    /// What people you know do for you besides tips (C15-C19).
    pub backchannel: backchannel::Backchannel,
    pub ties: ties::Ties,
    pub hs_staffs: staffing::HsStaffs,
    pub staff: staffing::Staffs,
    pub life: life::Lives,
    pub courts: courts::Courts,
    pub discipline: rules::LeagueBook,
    /// Your players' dated mood reasons this season (topic 3), by player id.
    pub moods: Vec<(u32, Vec<MoodNote>)>,
    /// Your hire's coaching-change window (T-B): program, opens, closes.
    pub window: Option<(usize, Date, Date)>,
    /// While the window opens: the portal runs for this program alone.
    pub window_only: Option<usize>,
    /// The trainer's report: you pushed rehab to be ready for spring.
    pub pushed_rehab: bool,
    /// The recruiting coordinator's calls to wobbling commits: program, day,
    /// and whether you made them all yourself.
    pub commit_calls: Option<(usize, Date, bool)>,
    /// Your stance with the press: 0 open door, 1 closed, 2 fair to all.
    pub media_policy: u8,
    /// The tones of your last team talks (a tone used too often wears out).
    pub mine_tones: Vec<u8>,
    /// While a takeover files its meetings: they go out as one schedule on
    /// the welcome letter, not as a letter each.
    #[serde(skip)]
    pub(crate) hush: bool,
    /// Your lines to the position coaches, by player id.
    pub lines: Vec<(u32, talks::Line)>,
    /// Players out by your discipline, a family leave or a rest: (id,
    /// through, why).
    pub out: Vec<(u32, Date, String)>,
    /// Benched for the start of the next game.
    pub benched: Vec<(u32, Date)>,
    /// Other teams' players sitting out by their own coach's rules (update
    /// 3, F18): id, through.
    pub ai_out: std::collections::BTreeMap<u32, Date>,
    /// Other teams' rule breaks their staffs saw: id, (count, last season).
    pub ai_priors: std::collections::BTreeMap<u32, (u8, i32)>,
    /// Players you brought back early from an injury (P3), and when.
    pub rushed: Vec<(u32, Date)>,
    /// Bulletin-board material you handed a rival: who and when.
    pub bulletin: Option<(Team, Date)>,
    /// "A fence around the state": the state and when you said it.
    pub fence: Option<(usize, Date)>,
}

/// Fills what a world built before this build lacks (an old world or save
/// opens with an empty store), then the yearly pieces.
/// The stable seed the people are drawn from, set once (anyone who reads a
/// cast person before the first day's people step sets it the same way).
pub fn ensure_base(w: &mut World) {
    if w.folk.base == 0 {
        w.folk.base = w.options.seed.rotate_left(7) ^ 0xF01C_5EED;
    }
}

pub fn after_load(w: &mut World) {
    ensure_base(w);
    staffing::ensure(w);
    if w.seat.is_some() && w.folk.mine.team.is_none() {
        // A save from before this build: you're at your job already, so
        // the people start from where you are without the first weeks.
        w.folk.mine = Mine { team: Team::seat(w), since: Some(w.today.add(-60)), ..Default::default() };
        network::build(w);
    }
}

/// One day on the clock: the world's people (weekly and yearly steps) and
/// your own agenda.
pub(crate) fn day(w: &mut World, d: Date) {
    ensure_base(w);
    staffing::day(w, d);
    ties::day(w, d);
    life::day(w, d);
    courts::day(w, d);
    rules::day(w, d);
    talks::day(w, d);
    boss::day(w, d);
    network::day(w, d);
    backchannel::day(w, d);
    press::day(w, d);
    if w.seat.is_some() {
        takeover::day(w, d);
        scene::day(w, d);
    }
}

/// A line for your inbox and calendar, about your team, in plain words.
pub(crate) fn post_mine(w: &mut World, d: Date, loud: Loudness, decision: bool, rule: &'static str, text: &str) {
    let Some(t) = Team::seat(w) else { return };
    let mut e = Event::new(d, t.level(), Kind::Slot, loud, plain(text)).rule(rule);
    match t {
        Team::College(p) => e = e.program(p),
        Team::Nfl(n) => e = e.team(n),
        Team::High(_) | Team::Other(_) => {}
    }
    if decision {
        e = e.decision();
    }
    w.calendar.post(e);
}

/// Adds a line to your people log (kept to the newest 600).
pub(crate) fn note(w: &mut World, topic: &str, source: String, text: String, angle: Option<String>) {
    let date = Some(w.today);
    w.folk.log.push(Entry { date, topic: topic.to_string(), source, text, angle, check: None });
    if w.folk.log.len() > 600 {
        let cut = w.folk.log.len() - 600;
        if w.logging() {
            w.log_dropped("people log", w.folk.log[..cut].iter());
        }
        w.folk.log.drain(..cut);
    }
}

/// A keyed draw from the people's own stream (never the world's).
pub(crate) fn roll(w: &World, a: u64, b: u64) -> f64 {
    super::season::key_uniform(w.folk.base as u64 ^ 0xF01C_0000_0000 ^ a, b)
}

pub(crate) fn roll_n(w: &World, a: u64, b: u64) -> f64 {
    super::season::key_normal(w.folk.base as u64 ^ 0xF01C_0000_0000 ^ a, b)
}

/// Person index by player id (scenes hold ids, which outlive the list's
/// yearly compaction).
/// The list grows in id order and its compaction keeps that order, so a
/// binary search finds nearly everyone; the scan is the safety net.
pub fn person(w: &World, id: u32) -> Option<usize> {
    if let Ok(i) = w.people.binary_search_by_key(&id, |p| p.player.id) {
        return Some(i);
    }
    w.people.iter().position(|p| p.player.id == id)
}

/// "a" or "an" before a word.
pub fn a_an(word: &str) -> &'static str {
    match word.chars().next().map(|c| c.to_ascii_lowercase()) {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

/// Words for a count ("three", "a dozen").
pub fn count_words(n: usize) -> String {
    const W: [&str; 13] =
        ["no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve"];
    if n < W.len() { W[n].to_string() } else { n.to_string() }
}

/// A noun for a count: `noun(1, "game")` is "game", `noun(3, "game")`
/// "games", `noun(2, "catch")` "catches", `noun(4, "story")` "stories".
pub fn noun(n: impl TryInto<i64>, one: &str) -> String {
    if n.try_into().is_ok_and(|n: i64| n == 1) {
        return one.to_string();
    }
    let b = one.as_bytes();
    let vowel = |c: u8| b"aeiou".contains(&c);
    if one.ends_with('y') && b.len() > 1 && !vowel(b[b.len() - 2]) {
        format!("{}ies", &one[..one.len() - 1])
    } else if ["s", "x", "z", "ch", "sh"].iter().any(|e| one.ends_with(e)) {
        format!("{one}es")
    } else {
        format!("{one}s")
    }
}

/// A record in words: "9-4".
pub fn record(w: u32, l: u32) -> String {
    format!("{w}-{l}")
}

/// A coach's reputation in words (1-20, public).
pub fn rep_words(r: f64) -> &'static str {
    match r {
        x if x >= 17.0 => "one of the best in the game",
        x if x >= 14.0 => "highly regarded",
        x if x >= 11.0 => "well regarded",
        x if x >= 8.0 => "solid",
        x if x >= 5.0 => "little known",
        _ => "unproven",
    }
}
