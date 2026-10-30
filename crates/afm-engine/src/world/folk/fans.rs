//! The fans' voices (people topic 9, "Fan voices"; Cain 2026-10-30): what
//! the town says after your game, the callers on the radio and the message
//! board on the news page, and the crowd on game day, at every level.
//!
//! Voices are drawn when a screen opens, from the facts and the fans' mood,
//! and never stored (topic 9, "Cost"). So that a town doesn't repeat itself,
//! the fan year (from the day after a Super Bowl to the next) is worked out
//! again from its first day each time: every game and every talk day in
//! order, through one chorus per crowd (`voice::Chorus`), so a version
//! comes back only once every other version that fits has had its turn.
//! The same game always reads the same, live and on its page afterwards.
//!
//! The voices change words only: nothing here moves the world. The built
//! moods they read are the NFL fans' (`office.fans`), the high school
//! town's support (`legacy::hs`) and each team's results against the line.
//! Every size here is Claude's pick (proposed, not confirmed).

use std::collections::HashSet;

use super::Team;
use super::voice::{Chorus, Facts, Speaker};
use crate::calendar::{Date, Kind, Level, Link, Weekday};
use crate::game::stats::{PlayRecord, PlayType};
use crate::rng::Rng;
use crate::world::World;
use crate::world::seat::SeatGame;

/// What fans talk about: each is a part `fans.<subject>` (said anywhere,
/// `place=town|radio|board` for one place only) and a board thread's title
/// `fans.board.title.<subject>`.
pub const SUBJECTS: [&str; 18] = [
    "win",
    "loss",
    "tie",
    "qb",
    "star",
    "goat",
    "coach",
    "calls",
    "defense",
    "refs",
    "next",
    "rival",
    "season",
    "hotseat",
    "recruiting",
    "draft",
    "offseason",
    "hire",
];

/// What the crowd reacts to, from its own team's side: `fans.crowd.<event>`.
pub const CROWD: [&str; 18] = [
    "kickoff",
    "score",
    "scored_on",
    "away",
    "takeaway",
    "giveaway",
    "sack",
    "big",
    "big_against",
    "stop",
    "boo",
    "miss",
    "kick",
    "refs",
    "hurt",
    "leaving",
    "final.win",
    "final.loss",
];

/// Every part the fans ask the bank for (the line tests read it).
pub fn parts() -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for s in SUBJECTS {
        v.push(format!("fans.{s}"));
        v.push(format!("fans.board.title.{s}"));
    }
    for c in CROWD {
        v.push(format!("fans.crowd.{c}"));
    }
    for k in [
        "fans.town.who",
        "fans.town.where",
        "fans.radio.show",
        "fans.radio.intro",
        "fans.radio.open",
        "fans.radio.close",
        "fans.board.reply",
        "fans.handle",
        "fans.sign",
    ] {
        v.push(k.into());
    }
    v
}

/// The fans' mood in five steps (the lines read it as `mood=...`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mood {
    Angry,
    Low,
    Mixed,
    Good,
    High,
}

impl Mood {
    pub fn of(x: f64) -> Mood {
        match x {
            x if x >= 0.45 => Mood::High,
            x if x >= 0.15 => Mood::Good,
            x if x > -0.15 => Mood::Mixed,
            x if x > -0.45 => Mood::Low,
            _ => Mood::Angry,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Mood::High => "high",
            Mood::Good => "good",
            Mood::Mixed => "mixed",
            Mood::Low => "low",
            Mood::Angry => "angry",
        }
    }

    /// The mood in words, for the screens.
    pub fn words(self) -> &'static str {
        match self {
            Mood::High => "Riding high",
            Mood::Good => "Happy",
            Mood::Mixed => "Unsure",
            Mood::Low => "Grumbling",
            Mood::Angry => "Up in arms",
        }
    }
}

/// One person in town after a game: who, where, and what they said.
#[derive(Clone, Debug)]
pub struct Voice {
    pub who: String,
    pub place: String,
    pub text: String,
}

/// One caller: the host's words bringing him on, then his.
#[derive(Clone, Debug)]
pub struct Call {
    pub caller: String,
    pub intro: String,
    pub text: String,
    /// One of the show's regulars (they call most weeks).
    pub regular: bool,
}

/// A talk day's call-in show.
#[derive(Clone, Debug)]
pub struct Show {
    pub day: Date,
    pub name: String,
    pub calls: Vec<Call>,
}

#[derive(Clone, Debug)]
pub struct Post {
    pub by: String,
    pub text: String,
}

/// A thread on the board.
#[derive(Clone, Debug)]
pub struct Thread {
    pub day: Date,
    pub subject: &'static str,
    pub title: String,
    pub posts: Vec<Post>,
}

/// The crowd after one play (its number in the play log).
#[derive(Clone, Debug)]
pub struct Cheer {
    pub n: u32,
    pub text: String,
}

/// One game's fans: the home crowd and its signs, then the town after it.
#[derive(Clone, Debug)]
pub struct GameVoices {
    /// Whose fans the town talk is.
    pub team: Team,
    /// The crowd's team (the home side).
    pub crowd_team: Team,
    pub signs: Vec<String>,
    pub crowd: Vec<Cheer>,
    pub town: Vec<Voice>,
    pub mood_before: Mood,
    pub mood_after: Mood,
}

/// Your fans this fortnight: the talk days' shows and threads, newest
/// first, and the town's talk after your last game.
#[derive(Clone, Debug)]
pub struct Week {
    pub team: Team,
    pub mood: Mood,
    pub board: String,
    pub station: String,
    pub shows: Vec<Show>,
    pub threads: Vec<Thread>,
    /// Your last game in the past week (its kept index) and the town on it.
    pub town: Option<(usize, Vec<Voice>)>,
}

/// Your fans now (the News page).
pub fn week(w: &World, team: Team) -> Week {
    let today = w.today;
    let mut y = Year::new(w, team, today);
    let mut shows = Vec::new();
    let mut threads = Vec::new();
    let mut town = None;
    for (occ, out) in y.walk(today, None) {
        match (occ, out) {
            (Occ::Talk(d), Out::Talk(show, ts)) if d.days_to(today) <= 13 => {
                shows.push(show);
                threads.extend(ts);
            }
            (Occ::Game(k), Out::Game(v)) if w.seat_games[k].date.days_to(today) <= 7 => town = Some((k, v.town)),
            _ => {}
        }
    }
    shows.reverse();
    threads.reverse();
    let run = y.run(today, None);
    Week {
        team,
        mood: Mood::of(y.mood(&run, today)),
        board: y.board_name(),
        station: y.station(),
        shows,
        threads,
        town,
    }
}

/// One of your kept games: its signs, its crowd play by play, and the town
/// after it. The town is your team's when you were in it, else the home
/// side's.
pub fn game(w: &World, k: usize) -> Option<GameVoices> {
    let g = w.seat_games.get(k)?;
    let sides = [team_of(g.level, g.teams[0]), team_of(g.level, g.teams[1])];
    let team = match Team::seat(w) {
        Some(t) if sides.contains(&t) => t,
        _ => sides[0],
    };
    let mut y = Year::new(w, team, g.date);
    y.walk(g.date, Some(k)).into_iter().find_map(|(o, out)| match (o, out) {
        (Occ::Game(j), Out::Game(v)) if j == k => Some(v),
        _ => None,
    })
}

/// The live game's signs and its crowd so far (`end`: the final score
/// once the whistle has gone): the same lines the game's page shows once
/// it is kept.
pub fn live(
    w: &World,
    team: Team,
    date: Date,
    level: Level,
    teams: [usize; 2],
    plays: &[PlayRecord],
    end: Option<[i32; 2]>,
) -> (Vec<String>, Vec<Cheer>) {
    let mut y = Year::new(w, team, date);
    y.walk(date.add(-1), None);
    let home = team_of(level, teams[0]);
    let away = team_of(level, teams[1]);
    let mood = y.crowd_mood(home, date);
    let signs = y.signs(home, away, mood);
    let crowd = y.crowd(home, away, plays, None, end, mood);
    (signs, crowd)
}

// ---------------------------------------------------------------------------
// The fan year, worked out again from its first day.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Occ {
    /// One of the team's kept games (its index).
    Game(usize),
    /// A talk day: the call-in show and the board.
    Talk(Date),
}

enum Out {
    Game(GameVoices),
    Talk(Show, Vec<Thread>),
}

/// The first day of the fan year a day falls in: the day after a Super Bowl.
pub fn year_start(d: Date) -> Date {
    let last = crate::calendar::anchors::super_bowl(d.year() - 1);
    if d > last { last.add(1) } else { crate::calendar::anchors::super_bowl(d.year() - 2).add(1) }
}

/// The memory key of a team's fans, all of them as one crowd.
fn crowd_key(team: Team) -> u64 {
    0xFA00_0000_0000_0000 ^ team.key()
}

pub(crate) fn team_of(level: Level, i: usize) -> Team {
    match level {
        Level::Nfl => Team::Nfl(i),
        Level::HighSchool => Team::High(i),
        Level::Other => Team::Other(i),
        _ => Team::College(i),
    }
}

fn plays_at(level: Level, team: Team) -> bool {
    match team {
        Team::College(_) => level == Level::College,
        Team::Nfl(_) => level == Level::Nfl,
        Team::High(_) => level == Level::HighSchool,
        Team::Other(_) => level == Level::Other,
    }
}

/// A game's facts from one side.
struct Side {
    side: usize,
    us: i32,
    them: i32,
    opp: Team,
    rival: bool,
    /// The win chance going in: the closing line, or the two programs.
    p: f64,
}

impl Side {
    fn result(&self) -> &'static str {
        match self.us.cmp(&self.them) {
            std::cmp::Ordering::Greater => "win",
            std::cmp::Ordering::Less => "loss",
            std::cmp::Ordering::Equal => "tie",
        }
    }
    fn upset(&self) -> Option<&'static str> {
        match self.result() {
            "win" if self.p < 0.35 => Some("won"),
            "loss" if self.p > 0.65 => Some("lost"),
            _ => None,
        }
    }
}

/// The team's season so far in the fan year.
#[derive(Default)]
struct Run {
    games: u32,
    wins: u32,
    losses: u32,
    ties: u32,
    expected: f64,
    /// Wins in a row (positive) or losses in a row (negative).
    streak: i32,
    last: Option<usize>,
}

/// One fan: his voice and his name. Drawn from his slot, never stored: a
/// regular (a low slot) is the same man all year.
struct Fan {
    who: Speaker,
    name: String,
    first: String,
    from: String,
    sort: Sort,
    born: i32,
    n: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sort {
    Ticket,
    Student,
    Alum,
    Parent,
    Booster,
    Local,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    Town,
    Radio,
    Board,
}

impl Place {
    fn key(self) -> &'static str {
        match self {
            Place::Town => "town",
            Place::Radio => "radio",
            Place::Board => "board",
        }
    }
}

/// Regulars per place: the callers and posters who turn up most weeks.
const REGULARS: u64 = 24;

struct Year<'a> {
    w: &'a World,
    team: Team,
    ch: Chorus,
    start: Date,
    season: i32,
    /// The team's kept games in the fan year, by date.
    games: Vec<usize>,
    /// The mood the fans carry into the year.
    carry: f64,
}

impl<'a> Year<'a> {
    fn new(w: &'a World, team: Team, day: Date) -> Year<'a> {
        let start = year_start(day);
        let season = start.year();
        let end = crate::calendar::anchors::super_bowl(season);
        let (_, i) = team.parts();
        let mut games: Vec<usize> = w
            .seat_games
            .iter()
            .enumerate()
            .filter(|(_, g)| g.date >= start && g.date <= end && plays_at(g.level, team) && g.teams.contains(&i))
            .map(|(k, _)| k)
            .collect();
        games.sort_by_key(|&k| (w.seat_games[k].date, k));
        Year {
            w,
            team,
            ch: Chorus::new(team.key() ^ (season as u64) << 40 ^ 0x00FA_0000),
            start,
            season,
            games,
            carry: carry(w, team, season),
        }
    }

    fn level_tag(&self, t: Team) -> &'static str {
        match t {
            Team::College(_) => "college",
            Team::High(_) => "hs",
            Team::Nfl(_) | Team::Other(_) => "nfl",
        }
    }

    /// The occasions up to a day: each kept game, then that night's talk;
    /// a Monday talk in a week without a game.
    fn occasions(&self, upto: Date) -> Vec<Occ> {
        let w = self.w;
        let days: Vec<(Date, usize)> =
            self.games.iter().map(|&k| (w.seat_games[k].date, k)).filter(|x| x.0 <= upto).collect();
        let mut v = Vec::new();
        let (mut gi, mut last_talk, mut last_game): (usize, Option<Date>, Option<Date>) = (0, None, None);
        let mut d = self.start;
        while d <= upto {
            let mut played = false;
            while gi < days.len() && days[gi].0 <= d {
                v.push(Occ::Game(days[gi].1));
                gi += 1;
                played = true;
            }
            let quiet = |x: Option<Date>| x.is_none_or(|x| x.days_to(d) >= 6);
            if played || d.weekday() == Weekday::Mon && quiet(last_talk) && quiet(last_game) {
                v.push(Occ::Talk(d));
                last_talk = Some(d);
            }
            if played {
                last_game = Some(d);
            }
            d = d.add(1);
        }
        v
    }

    /// Every occasion up to a day, in order, said through the one chorus.
    /// With `stop`, the walk ends after that game.
    fn walk(&mut self, upto: Date, stop: Option<usize>) -> Vec<(Occ, Out)> {
        let mut out = Vec::new();
        for o in self.occasions(upto) {
            let said = match o {
                Occ::Game(k) => Out::Game(self.kept_game(k)),
                Occ::Talk(d) => {
                    let (s, t) = self.talk(d);
                    Out::Talk(s, t)
                }
            };
            out.push((o, said));
            if matches!(o, Occ::Game(k) if Some(k) == stop) {
                break;
            }
        }
        out
    }

    fn side(&self, k: usize) -> Side {
        let w = self.w;
        let g = &w.seat_games[k];
        let (_, i) = self.team.parts();
        let side = usize::from(g.teams[0] != i);
        let opp = team_of(g.level, g.teams[1 - side]);
        let rival = super::facts::rival(w, self.team).is_some_and(|r| r.0 == opp);
        let p = match g.spread {
            Some(s) => {
                let m = if side == 0 { s } else { -s } as f64;
                let sd = match g.level {
                    Level::Nfl | Level::Other => 13.5,
                    Level::HighSchool => 17.0,
                    _ => 15.5,
                };
                crate::athlete::phi(m / sd)
            }
            None => (0.5 + 0.6 * (strength(w, self.team) - strength(w, opp))).clamp(0.1, 0.9),
        };
        Side { side, us: g.stats.score[side], them: g.stats.score[1 - side], opp, rival, p }
    }

    /// The season so far, up to a day (leaving out one game: the crowd's
    /// mood is the one it walked in with).
    fn run(&self, upto: Date, without: Option<usize>) -> Run {
        let mut r = Run::default();
        for &k in &self.games {
            if self.w.seat_games[k].date > upto || Some(k) == without {
                continue;
            }
            let s = self.side(k);
            r.games += 1;
            r.expected += s.p;
            let step = match s.result() {
                "win" => {
                    r.wins += 1;
                    1
                }
                "loss" => {
                    r.losses += 1;
                    -1
                }
                _ => {
                    r.ties += 1;
                    0
                }
            };
            r.streak = if step != 0 && r.streak.signum() == step { r.streak + step } else { step };
            r.last = Some(k);
        }
        r
    }

    /// The fans' mood, -1 to 1: what they carried in, the season against
    /// the lines (wins above what the books expected), last week's game
    /// (a rival and an upset count more) and a streak.
    fn mood(&self, run: &Run, d: Date) -> f64 {
        if run.games == 0 {
            return self.carry;
        }
        let g = run.games as f64;
        let now = ((run.wins as f64 + 0.5 * run.ties as f64 - run.expected) / g.sqrt() * 0.6).clamp(-1.0, 1.0);
        let mut m = 0.45 * self.carry + 0.55 * now;
        if let Some(k) = run.last
            && self.w.seat_games[k].date.days_to(d) <= 8
        {
            let s = self.side(k);
            let more = if s.rival { 0.15 } else { 0.0 } + if s.upset().is_some() { 0.1 } else { 0.0 };
            m += match s.result() {
                "win" => 0.15 + more,
                "loss" => -0.2 - more,
                _ => -0.05,
            };
        }
        if run.streak.abs() > 2 {
            m += 0.06 * (run.streak.abs() - 2).min(4) as f64 * run.streak.signum() as f64;
        }
        m.clamp(-1.0, 1.0)
    }

    /// The home crowd's mood walking in: yours from your season, another
    /// town's unsure.
    fn crowd_mood(&self, home: Team, d: Date) -> Mood {
        if home != self.team {
            return Mood::Mixed;
        }
        let run = self.run(d.add(-1), None);
        Mood::of(self.mood(&run, d))
    }

    // ---- names ----

    fn said(&self, t: Team) -> String {
        said_name(self.w, t)
    }

    fn mascot(&self, t: Team) -> String {
        crate::world::stories::mascot(self.w, crate::world::storylines::team_key(t)).unwrap_or_else(|| t.name(self.w))
    }

    fn coach_names(&self, t: Team) -> Option<(String, String)> {
        let c = t.head_coach(self.w)?;
        let last = last_name(&self.w.coaches[c].name);
        Some((format!("Coach {last}"), last))
    }

    /// The facts every line of this team's fans can use.
    fn base_facts(&self) -> Facts {
        let w = self.w;
        let t = self.team;
        let mut f = Facts::new();
        f.set("team", self.said(t)).set("mascot", self.mascot(t)).set("town", t.town(w));
        f.set("year", self.season.to_string());
        if let Some(c) = crate::world::stories::numbers::color_of(&t.name(w)) {
            f.set("color", c);
        }
        if let Some((c, l)) = self.coach_names(t) {
            f.set("coach", c).set("coach_last", l);
        }
        f
    }

    /// A fan of this team, drawn from his slot in a place.
    fn fan(&self, place: Place, slot: u64) -> Fan {
        let w = self.w;
        let team = self.team;
        let salt = match place {
            Place::Town => 0x7041,
            Place::Radio => 0x4AD1,
            Place::Board => 0xB0A2,
        };
        // A regular is the same person all year; anyone else is the year's.
        let year = if slot < REGULARS { 0 } else { self.season as u64 };
        let seed = team.key() ^ salt << 48 ^ slot.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ year << 20;
        let mut r = Rng::new((seed ^ seed >> 29 ^ seed >> 47) as u32);
        let level = team.level();
        let sort = match level {
            Level::HighSchool => r.weighted(&[
                (Sort::Parent, 0.3),
                (Sort::Alum, 0.25),
                (Sort::Student, 0.15),
                (Sort::Booster, 0.1),
                (Sort::Local, 0.2),
            ]),
            Level::College => r.weighted(&[
                (Sort::Alum, 0.35),
                (Sort::Ticket, 0.25),
                (Sort::Student, 0.15),
                (Sort::Booster, 0.05),
                (Sort::Local, 0.2),
            ]),
            _ => r.weighted(&[(Sort::Ticket, 0.35), (Sort::Local, 0.65)]),
        };
        // Women: about 45% of the fans in a high school's stands, a third
        // of those talking in town elsewhere, and about a tenth of the
        // callers and posters (sports talk radio's audience is mostly men).
        let she = r.chance(match (place, level) {
            (Place::Town, Level::HighSchool) => 0.45,
            (Place::Town, _) => 0.35,
            (Place::Radio, _) => 0.1,
            (Place::Board, _) => 0.12,
        });
        let age = match (sort, level) {
            (Sort::Student, Level::HighSchool) => r.int(15, 18),
            (Sort::Student, _) => r.int(18, 22),
            (Sort::Parent, _) => r.int(38, 54),
            (Sort::Booster, _) => r.int(45, 76),
            (Sort::Alum, _) => r.int(23, 78),
            (Sort::Ticket, _) => r.int(30, 78),
            (Sort::Local, _) => r.int(20, 76),
        };
        let born = self.season - age;
        let eth = r.weighted(&crate::world::ethnicity::with_asian_split([0.68, 0.17, 0.11, 0.0, 0.04]));
        let name = super::cast::person_name(eth, born, she, &mut r);
        let first = name.split_whitespace().next().unwrap_or("Pat").to_string();
        let from = near_town(w, team, &mut r);
        let mut tags: Vec<&'static str> = vec!["fan", self.level_tag(team)];
        tags.push(match sort {
            Sort::Ticket => "ticket",
            Sort::Student => "student",
            Sort::Alum => "alum",
            Sort::Parent => "parent",
            Sort::Booster => "booster",
            Sort::Local => "local",
        });
        if age < 30 {
            tags.push("young");
        }
        if age >= 60 {
            tags.push("old");
        }
        if she {
            tags.push("she");
        }
        if r.chance(0.2) {
            tags.push("hot");
        } else if r.chance(0.3) {
            tags.push("calm");
        }
        if r.chance(0.15) {
            tags.push("homer");
        } else if r.chance(0.18) {
            tags.push("doomer");
        }
        Speaker::region(&mut tags, w, team.state(w));
        Speaker::era(&mut tags, self.season);
        let n = r.int(2, 99) as u32;
        Fan { who: Speaker { key: crowd_key(team), tags }, name, first, from, sort, born, n }
    }

    /// The radio host's voice.
    fn host(&self) -> Speaker {
        let mut tags: Vec<&'static str> = vec!["host", self.level_tag(self.team)];
        Speaker::region(&mut tags, self.w, self.team.state(self.w));
        Speaker::era(&mut tags, self.season);
        Speaker { key: crowd_key(self.team) ^ 0x4057, tags }
    }

    /// A crowd's voice (the stands at a team's home games).
    fn crowd_voice(&self, t: Team) -> Speaker {
        let mut tags: Vec<&'static str> = vec!["crowd", self.level_tag(t)];
        Speaker::region(&mut tags, self.w, t.state(self.w));
        Speaker::era(&mut tags, self.season);
        Speaker { key: crowd_key(t), tags }
    }

    /// The team's radio station: call letters by the real rule (W east of
    /// the Mississippi, K west), on AM, or FM for some from 2010 when
    /// sports talk moved there.
    fn station(&self) -> String {
        let w = self.w;
        let mut r = Rng::new((self.team.key() ^ 0x57A7_1000) as u32);
        let code = self.team.state(w).and_then(|s| w.states.get(s)).map_or("", |s| s.code.as_str());
        let east = matches!(
            code,
            "WI" | "IL"
                | "MI"
                | "IN"
                | "OH"
                | "KY"
                | "TN"
                | "MS"
                | "AL"
                | "GA"
                | "FL"
                | "SC"
                | "NC"
                | "VA"
                | "WV"
                | "MD"
                | "DE"
                | "PA"
                | "NJ"
                | "NY"
                | "CT"
                | "RI"
                | "MA"
                | "VT"
                | "NH"
                | "ME"
                | "DC"
        );
        let call: String = std::iter::once(if east { 'W' } else { 'K' })
            .chain((0..3).map(|_| (b'A' + r.int(0, 25) as u8) as char))
            .collect();
        let fm = r.chance(0.45);
        if self.season >= 2010 && fm {
            format!("{call}-FM {:.1}", 88.1 + 0.2 * r.int(0, 99) as f64)
        } else {
            format!("{call} {} AM", 10 * r.int(54, 170))
        }
    }

    /// The show's name: the same all year.
    fn show_name(&self) -> String {
        let mut ch = Chorus::new(self.team.key() ^ (self.season as u64) << 32 ^ 0x5404);
        let mut f = self.base_facts();
        f.set("station", self.station());
        ch.part("fans.radio.show", &self.host(), &f).map_or_else(|| self.station(), |s| super::voice::tidy(&s))
    }

    /// Where the fans post, by level and era.
    fn board_name(&self) -> String {
        let w = self.w;
        let social = self.season >= 2010;
        match self.team {
            Team::High(_) if social => format!("{} community page", self.team.town(w)),
            Team::High(_) => "The area's high school football forum".into(),
            Team::College(_) => format!("The {} message board", self.mascot(self.team)),
            _ if social => format!("{} fans online", self.mascot(self.team)),
            _ => format!("The {} fan forum", self.mascot(self.team)),
        }
    }

    /// What a poster goes by: a handle (with an @ on the NFL's social
    /// feeds), or on a high school town's community page, his own name.
    fn handle(&self, fan: &Fan, slot: u64) -> String {
        let social = self.season >= 2010;
        if matches!(self.team, Team::High(_)) && social {
            return fan.name.clone();
        }
        let mut ch = Chorus::new(self.team.key() ^ slot.wrapping_mul(0x5851_F42D_4C95_7F2D) ^ 0x4A4D);
        let mut f = self.base_facts();
        f.set("name", fan.first.clone()).set("n", fan.n.to_string());
        f.set("year", format!("{:02}", fan.born.rem_euclid(100)));
        let h = ch.part("fans.handle", &fan.who, &f).unwrap_or_default();
        let mut h: String = h.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        if h.is_empty() {
            h = format!("{}{}", fan.first, fan.n);
        }
        if matches!(self.team, Team::Nfl(_) | Team::Other(_)) && social { format!("@{h}") } else { h }
    }

    /// A caller's or poster's slot: a regular about a third of the time.
    fn slot(&mut self, salt: u64) -> (u64, bool) {
        if self.ch.rng.chance(0.35) {
            (self.ch.rng.int(0, REGULARS as i32 - 1) as u64, true)
        } else {
            (REGULARS + salt, false)
        }
    }

    // ---- a game ----

    /// The facts of one of the team's games from its side; returns the
    /// details worth talking about besides the result.
    fn game_facts(&self, k: usize, f: &mut Facts) -> (Side, Vec<&'static str>) {
        let w = self.w;
        let g = &w.seat_games[k];
        let s = self.side(k);
        f.set("them", self.said(s.opp));
        f.set("score", format!("{}-{}", s.us, s.them));
        f.set("result", s.result());
        let gap = (s.us - s.them).abs();
        f.set(
            "margin",
            match gap {
                x if x >= 17 => "big",
                x if x <= 7 => "close",
                _ => "mid",
            },
        );
        f.set("at", if s.side == 0 { "home" } else { "away" });
        if s.rival {
            f.set("rival", "yes");
        }
        if let Some(u) = s.upset() {
            f.set("upset", u);
        }
        let mut more: Vec<&'static str> = vec!["coach"];
        if let Some(id) = qb(g, s.side)
            && let Some(n) = name_link(w, Some(g), id)
        {
            f.set("qb", n);
            more.push("qb");
        }
        if let Some(id) = star(g, s.side)
            && let Some(n) = name_link(w, Some(g), id)
        {
            f.set("star", n);
            more.push("star");
        }
        if let Some((id, cost)) = goat(g, s.side)
            && let Some(n) = name_link(w, Some(g), id)
        {
            f.set("goat", n).set("cost", cost);
            more.push("goat");
        }
        if s.result() != "win" || s.us <= 14 {
            more.push("calls");
        }
        if s.them >= 28 || s.them <= 10 {
            f.set("defense", if s.them <= 10 { "good" } else { "bad" });
            more.push("defense");
        }
        if refs_against(g, s.side) {
            more.push("refs");
        }
        if s.rival {
            more.push("rival");
        }
        (s, more)
    }

    fn kept_game(&mut self, k: usize) -> GameVoices {
        let w = self.w;
        let g = &w.seat_games[k];
        let home = team_of(g.level, g.teams[0]);
        let away = team_of(g.level, g.teams[1]);
        let mood_before = self.crowd_mood(self.team, g.date);
        let crowd_mood = if home == self.team { mood_before } else { Mood::Mixed };
        let signs = self.signs(home, away, crowd_mood);
        let crowd = self.crowd(home, away, &g.stats.plays, Some(g), Some(g.stats.score), crowd_mood);
        let run = self.run(g.date, None);
        let mood_after = Mood::of(self.mood(&run, g.date));
        let town = self.town(k, mood_after);
        GameVoices { team: self.team, crowd_team: home, signs, crowd, town, mood_before, mood_after }
    }

    /// The town the night and the morning after: a few people, more after
    /// a rivalry game, a blowout or an upset, and in a high school town.
    fn town(&mut self, k: usize, mood: Mood) -> Vec<Voice> {
        let w = self.w;
        let g = &w.seat_games[k];
        let mut f = self.base_facts();
        let (s, more) = self.game_facts(k, &mut f);
        f.set("mood", mood.key()).set("place", Place::Town.key());
        let run = self.run(g.date, None);
        set_run(&mut f, &run);
        let n = 3
            + usize::from(s.rival)
            + usize::from((s.us - s.them).abs() >= 21 || s.upset().is_some())
            + usize::from(matches!(self.team, Team::High(_)));
        let ours: Vec<u32> = g.stats.players[s.side]
            .iter()
            .filter(|(_, l)| l.snaps_off + l.snaps_def >= 10)
            .map(|(&id, _)| id)
            .collect();
        let mut out = Vec::new();
        for j in 0..n.min(6) {
            let slot = self.ch.rng.int(0, 400) as u64 + 64 * j as u64;
            let mut fan = self.fan(Place::Town, slot);
            let mut ff = f.clone();
            ff.set("name", fan.name.clone());
            match fan.sort {
                Sort::Ticket => {
                    // His seats: held since he was 18 at the earliest.
                    let years = 1 + fan.n as i32 % (self.season - fan.born - 17).clamp(1, 45);
                    ff.set("since", (self.season - years).to_string());
                }
                Sort::Alum => {
                    ff.set("class", (fan.born + 22).to_string());
                }
                Sort::Parent if !ours.is_empty() => {
                    let id = ours[self.ch.rng.int(0, ours.len() as i32 - 1) as usize];
                    if let Some(i) = super::person(w, id) {
                        let p = &w.people[i];
                        fan.name = format!("{} {}", fan.first, last_name(&p.name));
                        ff.set("name", fan.name.clone());
                        ff.set("player", name_link(w, Some(g), id).unwrap_or_default());
                        ff.set("pos", position_words(p.player.position));
                    }
                }
                _ => {}
            }
            let who = self.ch.part("fans.town.who", &fan.who, &ff).map(|s| super::voice::tidy(&s));
            let place = self.ch.part("fans.town.where", &fan.who, &ff).unwrap_or_default();
            let detail = more[self.ch.rng.int(0, more.len() as i32 - 1) as usize];
            let keys = if j == 0 || self.ch.rng.chance(0.55) {
                vec![format!("fans.{}", s.result()), format!("?fans.{detail}")]
            } else {
                vec![format!("fans.{detail}")]
            };
            let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
            let text = self.ch.line(&keys, &fan.who, &ff);
            if text.is_empty() {
                continue;
            }
            out.push(Voice { who: who.unwrap_or_else(|| fan.name.clone()), place, text });
        }
        out
    }

    // ---- the talk days ----

    /// A talk day: what's worth talking about, the show's callers and the
    /// board's threads.
    fn talk(&mut self, d: Date) -> (Show, Vec<Thread>) {
        let w = self.w;
        let run = self.run(d, None);
        let mood = Mood::of(self.mood(&run, d));
        let mut f = self.base_facts();
        f.set("mood", mood.key());
        set_run(&mut f, &run);
        let last = run.last.filter(|&k| w.seat_games[k].date == d);
        let mut lead: Option<&'static str> = None;
        let mut pool: Vec<(&'static str, f64)> = Vec::new();
        if let Some(k) = last {
            let (s, more) = self.game_facts(k, &mut f);
            lead = Some(s.result());
            pool.extend(more.into_iter().map(|m| (m, 1.0)));
        } else {
            pool.push(("offseason", 1.0));
        }
        // The next game, inside two weeks.
        let next = super::facts::team_games(w, self.team).into_iter().find(|x| x.0 > d && d.days_to(x.0) <= 13);
        if let Some((_, opp, _)) = next {
            f.set("next", self.said(opp));
            pool.push(("next", if last.is_some() { 0.6 } else { 1.5 }));
        }
        if run.games >= 2 {
            pool.push(("season", 0.7));
        }
        let hc_since = self.team.head_coach(w).and_then(|c| w.coaches[c].job.as_ref().map(|j| j.since));
        if mood == Mood::Angry && hc_since.is_some_and(|s| self.season - s >= 1) {
            pool.push(("hotseat", 1.6));
        }
        if run.games == 0 && hc_since.is_some_and(|s| s >= self.season - 1) {
            pool.push(("hire", 2.0));
        }
        if last.is_none() {
            if self.recruit_news(d, &mut f) {
                pool.push(("recruiting", 2.0));
            }
            if self.draft_news(d, &mut f) {
                pool.push(("draft", 2.0));
            }
        }
        let show = self.show(d, &f, lead, &pool);
        let threads = self.threads(d, &f, lead, &pool);
        (show, threads)
    }

    fn pick_subject(&mut self, pool: &[(&'static str, f64)]) -> &'static str {
        if pool.is_empty() {
            return "offseason";
        }
        self.ch.rng.weighted(pool)
    }

    fn show(&mut self, d: Date, f: &Facts, lead: Option<&'static str>, pool: &[(&'static str, f64)]) -> Show {
        let mut n = match self.team {
            Team::High(_) => 3,
            Team::College(_) => 5,
            _ => 6,
        };
        if lead.is_none() {
            n -= 1;
        }
        if f.get("rival").is_some() || f.get("upset").is_some() {
            n += 1;
        }
        let host = self.host();
        let mut calls = Vec::new();
        for j in 0..n {
            let (slot, regular) = self.slot(d.0 as u64 * 16 + j as u64);
            let fan = self.fan(Place::Radio, slot);
            let mut ff = f.clone();
            ff.set("place", Place::Radio.key()).set("name", fan.first.clone()).set("from", fan.from.clone());
            let intro =
                self.ch.part("fans.radio.intro", &host, &ff).map(|s| super::voice::tidy(&s)).unwrap_or_default();
            let subject = match lead {
                Some(l) if j == 0 || self.ch.rng.chance(0.45) => l,
                _ => self.pick_subject(pool),
            };
            let detail = self.pick_subject(pool);
            let a = format!("fans.{subject}");
            let b = format!("?fans.{detail}");
            let mut keys: Vec<&str> = vec!["?fans.radio.open", a.as_str()];
            if detail != subject {
                keys.push(b.as_str());
            }
            keys.push("?fans.radio.close");
            let text = self.ch.line(&keys, &fan.who, &ff);
            calls.push(Call { caller: format!("{}, {}", fan.first, fan.from), intro, text, regular });
        }
        Show { day: d, name: self.show_name(), calls }
    }

    fn threads(&mut self, d: Date, f: &Facts, lead: Option<&'static str>, pool: &[(&'static str, f64)]) -> Vec<Thread> {
        let mut subjects: Vec<&'static str> = Vec::new();
        if let Some(l) = lead {
            subjects.push(l);
        }
        while subjects.len() < 2 {
            let s = self.pick_subject(pool);
            if !subjects.contains(&s) || pool.len() <= 1 {
                subjects.push(s);
            }
            if pool.len() <= 1 {
                break;
            }
        }
        let mut out = Vec::new();
        for (t, &subject) in subjects.iter().enumerate() {
            let posts_n = 2 + self.ch.rng.int(0, 3) as usize;
            let mut posts = Vec::new();
            let mut title = String::new();
            for j in 0..posts_n {
                let (slot, _) = self.slot(d.0 as u64 * 64 + t as u64 * 8 + j as u64);
                let fan = self.fan(Place::Board, slot);
                let by = self.handle(&fan, slot);
                let mut ff = f.clone();
                ff.set("place", Place::Board.key()).set("name", fan.first.clone());
                if j == 0 {
                    title = self
                        .ch
                        .part(&format!("fans.board.title.{subject}"), &fan.who, &ff)
                        .map(|s| super::voice::tidy(&s))
                        .unwrap_or_default();
                }
                let text = if j > 0 && self.ch.rng.chance(0.3) {
                    self.ch.line(&["fans.board.reply"], &fan.who, &ff)
                } else {
                    let detail = self.pick_subject(pool);
                    let a = format!("fans.{subject}");
                    let b = format!("?fans.{detail}");
                    let keys: Vec<&str> =
                        if detail == subject || j == 0 { vec![a.as_str()] } else { vec![a.as_str(), b.as_str()] };
                    self.ch.line(&keys, &fan.who, &ff)
                };
                if !text.is_empty() {
                    posts.push(Post { by, text });
                }
            }
            if !posts.is_empty() {
                out.push(Thread { day: d, subject, title, posts });
            }
        }
        out
    }

    /// A commitment, a flip or a signing for the team in the past two
    /// weeks: a college's recruits, a high school's own players' colleges.
    fn recruit_news(&self, d: Date, f: &mut Facts) -> bool {
        let w = self.w;
        let roster: HashSet<u32> = match self.team {
            Team::High(h) => w.high_schools[h].roster.iter().map(|&i| w.people[i].player.id).collect(),
            Team::College(_) => HashSet::new(),
            _ => return false,
        };
        for (_, evs) in w.calendar.events.range(d.add(-14)..=d).rev() {
            for e in evs.iter().rev() {
                if !matches!(e.kind, Kind::Commitment | Kind::Signing) {
                    continue;
                }
                let Some(id) = e.refs.iter().find_map(|r| match r.to {
                    Link::Person(id) => Some(id),
                    _ => None,
                }) else {
                    continue;
                };
                let ours = match self.team {
                    Team::College(p) => e.program == Some(p),
                    _ => roster.contains(&id),
                };
                if !ours {
                    continue;
                }
                let Some(n) = name_link(w, None, id) else { continue };
                let mv = if e.text.contains("decommit") {
                    "decommit"
                } else if e.kind == Kind::Signing {
                    "sign"
                } else {
                    "commit"
                };
                f.set("player", n).set("move", mv);
                if let (Team::High(_), Some(p)) = (self.team, e.program) {
                    f.set("other", w.program_name(p).to_string());
                }
                return true;
            }
        }
        false
    }

    /// The draft: an NFL club's picks or a college's players taken in the
    /// past two weeks, or the weeks before an NFL draft.
    fn draft_news(&self, d: Date, f: &mut Facts) -> bool {
        let w = self.w;
        if matches!(self.team, Team::High(_) | Team::Other(_)) {
            return false;
        }
        for (_, evs) in w.calendar.events.range(d.add(-14)..=d).rev() {
            for e in evs.iter().rev() {
                if e.kind != Kind::DraftPick {
                    continue;
                }
                let ours = match self.team {
                    Team::Nfl(t) => e.nfl_team == Some(t),
                    Team::College(p) => e.program == Some(p),
                    _ => false,
                };
                let id = e.refs.iter().find_map(|r| match r.to {
                    Link::Person(id) => Some(id),
                    _ => None,
                });
                if let (true, Some(n)) = (ours, id.and_then(|id| name_link(w, None, id))) {
                    f.set("player", n).set("draft", "after");
                    if let (Team::College(_), Some(t)) = (self.team, e.nfl_team) {
                        f.set("other", said_name(w, Team::Nfl(t)));
                    }
                    return true;
                }
            }
        }
        // The NFL's draft talk: March and April.
        if matches!(self.team, Team::Nfl(_)) && matches!(d.month(), 3 | 4) {
            f.set("draft", "before");
            return true;
        }
        false
    }

    // ---- game day ----

    /// Signs in the home stands.
    fn signs(&mut self, home: Team, away: Team, mood: Mood) -> Vec<String> {
        let n = match home {
            Team::College(_) => 3,
            _ => 2,
        };
        let who = self.crowd_voice(home);
        let mut f = Facts::new();
        f.set("team", self.said(home)).set("them", self.said(away)).set("mascot", self.mascot(home));
        f.set("town", home.town(self.w)).set("mood", mood.key());
        if let Some((c, l)) = self.coach_names(home) {
            f.set("coach", c).set("coach_last", l);
        }
        if super::facts::rival(self.w, home).is_some_and(|r| r.0 == away) {
            f.set("rival", "yes");
        }
        (0..n).filter_map(|_| self.ch.part("fans.sign", &who, &f).map(|s| super::voice::tidy(&s))).collect()
    }

    /// The home crowd through a game: the plays that move it, from its own
    /// team's side, at most one line every few plays except the scores,
    /// the turnovers and the end.
    fn crowd(
        &mut self,
        home: Team,
        away: Team,
        plays: &[PlayRecord],
        g: Option<&SeatGame>,
        end: Option<[i32; 2]>,
        mood: Mood,
    ) -> Vec<Cheer> {
        let w = self.w;
        let who = self.crowd_voice(home);
        let mut base = Facts::new();
        base.set("team", self.said(home)).set("them", self.said(away)).set("mascot", self.mascot(home));
        base.set("mood", mood.key()).set("at", "home");
        if super::facts::rival(w, home).is_some_and(|r| r.0 == away) {
            base.set("rival", "yes");
        }
        let mut out = Vec::new();
        let mut last: Option<usize> = None;
        let mut left = false;
        let mut hurt: HashSet<u32> = HashSet::new();
        for (i, r) in plays.iter().enumerate() {
            if r.offense > 1 {
                continue;
            }
            let ours = r.offense == 0;
            // The score after the play from the play alone, so a line said
            // live reads the same once the next snap is in.
            let after = score_after(r);
            let scrimmage = matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble);
            let late = r.quarter >= 4 && (after[0] - after[1]).abs() <= 8;
            let mut ev: Option<(&str, Option<u32>, bool)> = None;
            if i == 0 {
                ev = Some(("kickoff", None, true));
            } else if r.nullified {
                if r.penalties.iter().any(|p| p.4 && p.1 == 0 && p.3 >= 10.0) {
                    ev = Some(("refs", None, false));
                }
            } else if r.touchdown || r.defense_td {
                let by_home = ours != r.defense_td;
                let man = if r.defense_td {
                    r.interceptor.or(r.forced_fumble)
                } else {
                    r.carrier.or(if r.complete { r.target } else { None })
                };
                ev = Some((
                    if by_home {
                        "score"
                    } else if self.ch.rng.chance(0.4) {
                        "away"
                    } else {
                        "scored_on"
                    },
                    man,
                    true,
                ));
            } else if r.turnover {
                ev = if ours {
                    Some(("giveaway", r.passer.filter(|_| r.interception).or(r.fumbled), true))
                } else {
                    Some(("takeaway", r.interceptor.or(r.forced_fumble), true))
                };
            } else if r.safety {
                ev = Some((if ours { "giveaway" } else { "takeaway" }, None, true));
            } else if r.kind == PlayType::FieldGoal {
                ev = match (ours, r.kick_good) {
                    (true, false) => Some(("miss", r.kicker, false)),
                    (true, true) if late || r.kick_distance >= 50.0 => Some(("kick", r.kicker, false)),
                    (false, false) => Some(("stop", None, false)),
                    _ => None,
                };
            } else if r.kind == PlayType::Sack && !ours {
                ev = Some(("sack", r.sackers.first().copied(), false));
            } else if scrimmage && r.yards >= 25.0 {
                let man = r.carrier.or(if r.complete { r.target } else { None });
                ev = Some((if ours { "big" } else { "big_against" }, man, false));
            } else if scrimmage && !ours && r.down >= 3 && !r.first_down {
                ev = Some(("stop", r.tacklers.first().copied(), false));
            } else if r.kind == PlayType::Punt && ours && (after[1] - after[0] >= 10 || mood <= Mood::Low) {
                ev = Some(("boo", None, false));
            }
            if ev.is_none() && r.officiating.as_ref().is_some_and(|o| o.wrong && !o.overturned && o.against == 0) {
                ev = Some(("refs", None, false));
            }
            if ev.is_none()
                && let Some(x) = r.injuries.iter().find(|x| !hurt.contains(&x.id))
            {
                hurt.insert(x.id);
                ev = Some(("hurt", Some(x.id), false));
            }
            let mut said = |this: &mut Self, key: &str, man: Option<u32>| {
                let mut f = base.clone();
                f.set("score", format!("{}-{}", after[0], after[1]));
                if late {
                    f.set("late", "yes");
                }
                if let Some(n) = man.and_then(|id| name_link(w, g, id)) {
                    f.set("player", n);
                }
                if let Some(t) = this.ch.part(&format!("fans.crowd.{key}"), &who, &f) {
                    out.push(Cheer { n: r.n, text: super::voice::tidy(&t) });
                }
            };
            if let Some((key, man, always)) = ev {
                let chance = match key {
                    "stop" => 0.55,
                    "sack" | "boo" | "refs" | "big_against" => 0.7,
                    "big" | "hurt" => 0.8,
                    _ => 0.95,
                };
                let room = last.is_none_or(|l| i - l >= 3);
                if always || room && self.ch.rng.chance(chance) {
                    said(self, key, man);
                    last = Some(i);
                }
            }
            if !left && r.quarter >= 4 && after[1] - after[0] >= 21 {
                left = true;
                said(self, "leaving", None);
            }
        }
        if let Some(s) = end
            && !plays.is_empty()
        {
            let key = if s[0] > s[1] { "final.win" } else { "final.loss" };
            let mut f = base.clone();
            f.set("score", format!("{}-{}", s[0], s[1]));
            f.set(
                "result",
                match s[0].cmp(&s[1]) {
                    std::cmp::Ordering::Greater => "win",
                    std::cmp::Ordering::Less => "loss",
                    std::cmp::Ordering::Equal => "tie",
                },
            );
            if let Some(t) = self.ch.part(&format!("fans.crowd.{key}"), &who, &f) {
                out.push(Cheer { n: plays.last().map_or(0, |r| r.n), text: super::voice::tidy(&t) });
            }
        }
        out
    }
}

/// The record and a streak, as the fans say them.
fn set_run(f: &mut Facts, run: &Run) {
    if run.games == 0 {
        return;
    }
    let rec = if run.ties > 0 {
        format!("{}-{}-{}", run.wins, run.losses, run.ties)
    } else {
        format!("{}-{}", run.wins, run.losses)
    };
    f.set("record", rec);
    if run.streak.abs() >= 3 {
        f.set("streak", format!("{} straight", super::count_words(run.streak.unsigned_abs() as usize)));
        f.set("run", if run.streak > 0 { "won" } else { "lost" });
    }
}

/// The score (home, away) once a play's own points are in.
fn score_after(r: &PlayRecord) -> [i32; 2] {
    let mut s = r.score;
    let off = r.offense.min(1);
    if r.nullified {
        return s;
    }
    if r.touchdown && !r.defense_td {
        s[off] += 6;
    }
    if r.defense_td {
        s[1 - off] += 6;
    }
    if r.safety {
        s[1 - off] += 2;
    }
    match r.kind {
        PlayType::FieldGoal if r.kick_good => s[off] += 3,
        PlayType::ExtraPoint if r.kick_good => s[off] += 1,
        _ => {}
    }
    s
}

/// How a team is said by its fans: "the Bills", "Alabama", "Lincoln".
pub fn said_name(w: &World, t: Team) -> String {
    match t {
        Team::Nfl(_) | Team::Other(_) => {
            let m =
                crate::world::stories::mascot(w, crate::world::storylines::team_key(t)).unwrap_or_else(|| t.name(w));
            format!("the {m}")
        }
        Team::College(_) => t.name(w),
        Team::High(_) => {
            let n = t.name(w);
            let n = n.split(" (").next().unwrap_or(&n).to_string();
            n.strip_suffix(" High").map_or(n.clone(), str::to_string)
        }
    }
}

/// A last name without its Jr. or III.
fn last_name(full: &str) -> String {
    let parts: Vec<&str> = full.split_whitespace().collect();
    match parts.as_slice() {
        [.., l, s] if matches!(*s, "Jr." | "Sr." | "II" | "III" | "IV") => l.to_string(),
        [.., l] => l.to_string(),
        _ => full.to_string(),
    }
}

/// A player's last name as a link to his page.
fn name_link(w: &World, g: Option<&SeatGame>, id: u32) -> Option<String> {
    let full = super::person(w, id)
        .map(|i| w.people[i].name.clone())
        .or_else(|| g.and_then(|g| crate::world::seat_names::gone(g, id)).map(|x| x.name.clone()))?;
    Some(format!("[[p:{id}|{}]]", last_name(&full)))
}

fn position_words(p: crate::positions::Position) -> &'static str {
    use crate::positions::Position as P;
    match p {
        P::QB => "quarterback",
        P::RB => "running back",
        P::WR => "receiver",
        P::TE => "tight end",
        P::OT => "tackle",
        P::IOL => "guard",
        P::EDGE => "defensive end",
        P::DT => "defensive tackle",
        P::LB => "linebacker",
        P::CB => "corner",
        P::S => "safety",
        P::K => "kicker",
        P::P => "punter",
        P::LS => "long snapper",
    }
}

/// A town in the team's state, where a caller lives.
fn near_town(w: &World, team: Team, r: &mut Rng) -> String {
    let schools = team.state(w).and_then(|s| w.states.get(s)).map(|s| &s.schools);
    match schools {
        Some(v) if !v.is_empty() => w.high_schools[*r.pick(v)].town.clone(),
        _ => team.town(w),
    }
}

/// How strong a program reads to its fans without a line.
fn strength(w: &World, t: Team) -> f64 {
    match t {
        Team::College(p) => w.programs[p].prestige_rel,
        Team::High(h) => w.high_schools[h].prestige,
        _ => 0.5,
    }
}

/// The mood the fans carry into the year: NFL fans' built mood; your high
/// school town's support; elsewhere last season against what the program
/// hopes for.
fn carry(w: &World, team: Team, season: i32) -> f64 {
    match team {
        Team::Nfl(t) => w.clubs.get(t).map_or(0.0, |c| c.office.fans),
        Team::High(_) if Team::seat(w) == Some(team) && w.legacy.hs.support > 0.0 => {
            ((w.legacy.hs.support as f64 - 50.0) / 50.0).clamp(-1.0, 1.0)
        }
        Team::College(_) | Team::High(_) => {
            let hope = match team {
                Team::College(p) => 0.35 + 0.45 * w.programs[p].prestige_rel,
                Team::High(h) => 0.3 + 0.4 * w.high_schools[h].prestige,
                _ => 0.5,
            };
            super::facts::seasons(w, team, 3).iter().rev().find(|s| s.year < season).map_or(0.0, |s| {
                let n = (s.wins + s.losses).max(1) as f64;
                ((s.wins as f64 / n - hope) * 2.5).clamp(-1.0, 1.0)
            })
        }
        Team::Other(_) => 0.0,
    }
}

/// The side's quarterback in a game: the most throws, five at least.
fn qb(g: &SeatGame, side: usize) -> Option<u32> {
    g.stats.players[side].iter().filter(|(_, l)| l.att >= 5).max_by_key(|(id, l)| (l.att, **id)).map(|(&id, _)| id)
}

/// The side's best player in a game, by a plain game score.
fn star(g: &SeatGame, side: usize) -> Option<u32> {
    let score = |l: &crate::game::stats::StatLine| {
        l.pass_yds / 25.0 + 4.0 * l.pass_td as f64 - 2.0 * l.int as f64
            + l.rush_yds / 10.0
            + 6.0 * l.rush_td as f64
            + l.rec_yds / 10.0
            + 6.0 * l.rec_td as f64
            + 0.6 * (l.tackles + l.assists) as f64
            + 3.0 * l.sacks
            + 5.0 * l.ints as f64
    };
    g.stats.players[side]
        .iter()
        .map(|(&id, l)| (id, score(l)))
        .filter(|x| x.1 >= 10.0)
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
        .map(|x| x.0)
}

/// The side's costliest mistake: an interception, a lost fumble, a missed
/// kick or a drop that cost two expected points or more.
fn goat(g: &SeatGame, side: usize) -> Option<(u32, &'static str)> {
    let mut best: Option<(u32, &'static str, f64)> = None;
    for r in g.stats.plays.iter().filter(|r| r.offense == side && !r.nullified) {
        let who = if r.interception {
            r.passer.map(|x| (x, "pick"))
        } else if r.fumble_lost {
            r.fumbled.map(|x| (x, "fumble"))
        } else if r.kind == PlayType::FieldGoal && !r.kick_good && !r.blocked {
            r.kicker.map(|x| (x, "miss"))
        } else if r.drop {
            r.target.map(|x| (x, "drop"))
        } else {
            None
        };
        if let Some((id, cost)) = who
            && r.epa <= -2.0
            && best.is_none_or(|b| r.epa < b.2)
        {
            best = Some((id, cost, r.epa));
        }
    }
    best.map(|b| (b.0, b.1))
}

/// A close call the officials got wrong against the side and replay
/// didn't fix.
fn refs_against(g: &SeatGame, side: usize) -> bool {
    g.stats.plays.iter().any(|r| r.officiating.as_ref().is_some_and(|o| o.wrong && !o.overturned && o.against == side))
}
