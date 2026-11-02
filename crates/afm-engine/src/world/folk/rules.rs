//! Team rules and discipline (people topic 8), league discipline on the
//! real scale (final build P2) and rookie rituals (P8).
//!
//! Your rulebook: each rule with how strictly you enforce it. Violations
//! come from personality (professionalism most, then temper and ego), mood,
//! age and the room; a long strict list catches more players and wears on
//! veterans, a loose one lets small things grow into the off-field trouble
//! of topic 7. You hear of a violation only as well as your staff sees it.
//! Your answers stay within each level's real limits (the NFL's CBA
//! Article 42 fine schedule and four-week cap; college coaches can't fine;
//! high school within the school's code). Consistency is tracked: the star
//! held to the backup's rule, or not.
//!
//! AI teams' rules are drawn from their head coach (discipline and style)
//! when needed, never stored. Sizes no source publishes are Claude's picks
//! (proposed, not confirmed).

use super::scene::{self, Kind, Pick, Scene, Who};
use super::talks::{self, Reason};
use super::{Team, player_link};
use crate::calendar::{Date, Level, Loudness, Weekday};
use crate::player::{PlayingTime, Trait};
use crate::world::{Stage, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RuleKind {
    Curfew,
    OnTime,
    Class,
    StudyHall,
    Weight,
    Dress,
    Phones,
    Social,
    DrugTests,
    Conduct,
}

impl RuleKind {
    pub const ALL: [RuleKind; 10] = [
        RuleKind::Curfew,
        RuleKind::OnTime,
        RuleKind::Class,
        RuleKind::StudyHall,
        RuleKind::Weight,
        RuleKind::Dress,
        RuleKind::Phones,
        RuleKind::Social,
        RuleKind::DrugTests,
        RuleKind::Conduct,
    ];
    pub fn name(self) -> &'static str {
        match self {
            RuleKind::Curfew => "Curfew",
            RuleKind::OnTime => "Being on time",
            RuleKind::Class => "Going to class",
            RuleKind::StudyHall => "Study hall",
            RuleKind::Weight => "Weight targets",
            RuleKind::Dress => "Dress and travel",
            RuleKind::Phones => "Phones in meetings",
            RuleKind::Social => "Social media",
            RuleKind::DrugTests => "Team drug testing",
            RuleKind::Conduct => "Conduct away from the facility",
        }
    }
    pub fn about(self) -> &'static str {
        match self {
            RuleKind::Curfew => "In the hotel or at home by a set hour the night before games, and in camp.",
            RuleKind::OnTime => "Meetings, lifts, treatment and the bus. Five minutes early is on time.",
            RuleKind::Class => "Class checkers in the back row. Miss one, run in the morning.",
            RuleKind::StudyHall => "Required hours for freshmen and anyone under the line.",
            RuleKind::Weight => "Weigh-ins against the target the strength staff set.",
            RuleKind::Dress => "Suits on the road, team gear at the facility, no headphones off the bus.",
            RuleKind::Phones => "Phones away in meetings and the film room.",
            RuleKind::Social => "What players can post. A ban costs players NIL money from 2021.",
            RuleKind::DrugTests => "The program's own testing, beyond the NCAA's.",
            RuleKind::Conduct => "Bars, parties, the people they run with. The rule behind every other rule.",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            RuleKind::Curfew => "curfew",
            RuleKind::OnTime => "late",
            RuleKind::Class => "class",
            RuleKind::StudyHall => "studyhall",
            RuleKind::Weight => "weight",
            RuleKind::Dress => "dress",
            RuleKind::Phones => "phones",
            RuleKind::Social => "social",
            RuleKind::DrugTests => "drugs",
            RuleKind::Conduct => "conduct",
        }
    }
    pub fn from_key(k: &str) -> Option<RuleKind> {
        RuleKind::ALL.iter().copied().find(|r| r.key() == k)
    }
    /// Whether a level has the rule: class and study hall at schools, the
    /// program's own drug testing in college.
    pub fn at(self, level: Level) -> bool {
        match self {
            RuleKind::Class | RuleKind::StudyHall => level != Level::Nfl,
            RuleKind::DrugTests => level == Level::College,
            _ => true,
        }
    }
    /// How serious a break of it is, 1 (small) to 3.
    fn weight(self) -> u8 {
        match self {
            RuleKind::Conduct | RuleKind::DrugTests => 3,
            RuleKind::Curfew | RuleKind::Class => 2,
            _ => 1,
        }
    }
    /// What a break looks like, in words.
    fn what(self, level: Level) -> &'static str {
        match self {
            RuleKind::Curfew => "missed curfew the night before the game",
            RuleKind::OnTime => "was late to the morning meeting",
            RuleKind::Class if level == Level::HighSchool => "skipped two classes this week",
            RuleKind::Class => "was reported missing from class by the checkers",
            RuleKind::StudyHall => "skipped his study hall hours",
            RuleKind::Weight => "came in over his target at the weigh-in",
            RuleKind::Dress => "showed up for the team flight in sweats",
            RuleKind::Phones => "was on his phone in the film room",
            RuleKind::Social => "posted something about the depth chart",
            RuleKind::DrugTests => "failed the team's drug test",
            RuleKind::Conduct => "was at a party the police broke up",
        }
    }
}

/// Your answer to a violation, within the level's limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Response {
    Talk,
    /// NFL only: a fine on the CBA schedule of the year.
    Fine,
    /// Extra conditioning or lost privileges (college and high school).
    Running,
    /// Benched for the first series or the start.
    BenchStart,
    /// Sits one game (an NFL deactivation).
    BenchGame,
    /// A team suspension of several games (NFL: unpaid, up to four weeks).
    Suspend(u8),
    /// Released (NFL) or dismissed (college, high school).
    Dismiss,
    /// The leadership council decides.
    Council,
}

impl Response {
    pub fn words(self, level: Level) -> String {
        match self {
            Response::Talk => "a talk and a warning".into(),
            Response::Fine => "a fine".into(),
            Response::Running => "early-morning running".into(),
            Response::BenchStart => "benched for the start".into(),
            Response::BenchGame if level == Level::Nfl => "deactivated for a game".into(),
            Response::BenchGame => "sat out a game".into(),
            Response::Suspend(n) => format!("suspended {n} game{}", if n == 1 { "" } else { "s" }),
            Response::Dismiss if level == Level::Nfl => "released".into(),
            Response::Dismiss => "dismissed from the team".into(),
            Response::Council => "handed to the leadership council".into(),
        }
    }
    /// Severity, 0 to 6.
    fn severity(self) -> u8 {
        match self {
            Response::Talk | Response::Council => 0,
            Response::Fine | Response::Running => 1,
            Response::BenchStart => 2,
            Response::BenchGame => 3,
            Response::Suspend(n) => 3 + n.min(3),
            Response::Dismiss => 6,
        }
    }
}

/// One ruling, kept to judge consistency (topic 8).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Ruling {
    pub date: Date,
    pub id: u32,
    pub kind: RuleKind,
    pub response: Response,
    /// A starter or a star at the time.
    pub star: bool,
}

/// A violation waiting on your answer.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Violation {
    pub id: u32,
    pub kind: RuleKind,
    pub date: Date,
    pub scene: u32,
    pub prior: u8,
}

/// Your rulebook (topic 8).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Rulebook {
    /// Each rule with its strictness: 1 lenient, 2 firm, 3 strict.
    pub rules: Vec<(RuleKind, u8)>,
    /// Your leadership council (player ids).
    pub council: Vec<u32>,
    /// Players you've warned, and when (a warning lowers his risk a while).
    pub warned: Vec<(u32, Date)>,
    pub rulings: Vec<Ruling>,
    /// Set by you at least once (a new job starts from a standard book).
    pub set: bool,
    /// Rookie rituals: 0 banned, 1 within limits, 2 the veterans' call.
    pub rituals: u8,
}

impl Rulebook {
    pub fn add(&mut self, k: RuleKind, strict: u8) {
        match self.rules.iter_mut().find(|x| x.0 == k) {
            Some(x) => x.1 = x.1.max(strict),
            None => self.rules.push((k, strict.clamp(1, 3))),
        }
    }
    pub fn set_rule(&mut self, k: RuleKind, strict: u8) {
        self.rules.retain(|x| x.0 != k);
        if strict > 0 {
            self.rules.push((k, strict.min(3)));
        }
        self.set = true;
    }
    pub fn strict(&self, k: RuleKind) -> u8 {
        self.rules.iter().find(|x| x.0 == k).map_or(0, |x| x.1)
    }
    /// The whole book's weight: number of rules times strictness.
    pub fn load(&self) -> f32 {
        self.rules.iter().map(|x| x.1 as f32).sum()
    }
}

/// The standard book a new head coach walks in with, by level (the common
/// real set at firm strictness).
pub fn standard(level: Level) -> Rulebook {
    let mut b = Rulebook { rituals: 1, ..Default::default() };
    for k in [RuleKind::Curfew, RuleKind::OnTime, RuleKind::Weight, RuleKind::Conduct, RuleKind::Phones] {
        b.add(k, 2);
    }
    if level != Level::Nfl {
        b.add(RuleKind::Class, 2);
        b.add(RuleKind::StudyHall, 1);
    }
    if level == Level::Nfl {
        b.add(RuleKind::Dress, 1);
    }
    b
}

/// An AI head coach's book (never stored): his discipline sets how many
/// rules and how strict; structure in his style adds more.
pub fn ai_book(w: &World, coach: usize, level: Level) -> Rulebook {
    let co = &w.coaches[coach];
    let d = co.ability().discipline;
    let strict = if d >= 14.0 {
        3
    } else if d >= 9.0 {
        2
    } else {
        1
    };
    let mut b = Rulebook::default();
    for (k, need) in [
        (RuleKind::OnTime, 0.0),
        (RuleKind::Curfew, 4.0),
        (RuleKind::Conduct, 6.0),
        (RuleKind::Class, 6.0),
        (RuleKind::Weight, 8.0),
        (RuleKind::Phones, 11.0),
        (RuleKind::Dress, 13.0),
        (RuleKind::StudyHall, 12.0),
        (RuleKind::DrugTests, 14.0),
        (RuleKind::Social, 16.0),
    ] {
        if d >= need && k.at(level) {
            b.add(k, strict);
        }
    }
    b
}

/// The CBA's fine for small offenses in a league year (Article 42: 2025's
/// $3,089 for being late or missing curfew, rising about 3% a year; before
/// the 2011 CBA the schedule was about $1,700).
pub fn nfl_fine(year: i32, kind: RuleKind) -> f64 {
    let base = 3089.0 * 1.03f64.powi(year - 2025);
    let k = match kind {
        RuleKind::Weight => 822.0 / 3089.0 * 3.0,
        RuleKind::Conduct => 4.0,
        _ => 1.0,
    };
    (base * k).max(1200.0).round()
}

/// Players sitting by your discipline: (player id, through date, why).
pub fn sitting(w: &World, id: u32, d: Date) -> bool {
    w.folk.out.iter().any(|x| x.0 == id && x.1 >= d) || w.folk.ai_out.get(&id).is_some_and(|&x| x >= d)
}

/// The day after the n-th next game of your team (for a suspension).
fn after_games(w: &World, team: Team, n: u8) -> Date {
    let games = super::facts::team_games(w, team);
    let today = w.today;
    games.iter().map(|g| g.0).filter(|&d| d >= today).nth(n.max(1) as usize - 1).unwrap_or(today.add(7 * n as i32))
}

/// How likely a player is to break a rule this week (topic 8):
/// professionalism most, then temper and ego, his mood, his age and the
/// room; a warning lowers it for a while.
fn risk(w: &World, i: usize, book: &Rulebook, room: f32, base: f32) -> f32 {
    let p = &w.people[i];
    let t = &p.player.personality.traits;
    let z = |x: Trait| t.z(x) as f32;
    let warned = w.folk.rules.warned.iter().any(|x| x.0 == p.player.id && x.1.days_to(w.today) < 60);
    let age = p.player.age as f32;
    let f = (1.0 - 0.55 * z(Trait::Professionalism) + 0.25 * z(Trait::Temper) + 0.15 * z(Trait::Ego)
        - 0.35 * p.mood.min(0.0)
        + 0.25 * (p.mood < -0.3) as u8 as f32
        - 0.2 * room)
        .clamp(0.2, 3.0)
        * (1.0 - 0.03 * (age - 20.0)).clamp(0.6, 1.25)
        * if warned { 0.6 } else { 1.0 };
    base * f * (0.6 + 0.12 * book.load())
}

/// Whether a team's level is in its rules season (camp to the bowls, and
/// spring practice, below the NFL).
fn rules_season(w: &World, level: Level, d: Date) -> bool {
    match level {
        Level::Nfl => d >= w.dates.veterans_report && d <= w.dates.super_bowl,
        _ => {
            d >= w.dates.fall_camp.add(-30) && d <= w.dates.bowl_season.1
                || (d >= w.dates.spring_practice.0 && d <= w.dates.spring_game)
        }
    }
}

/// Weekly break risk per player by level (proposed, not confirmed).
fn rules_base(level: Level) -> f32 {
    match level {
        Level::Nfl => 0.0016,
        Level::College => 0.0022,
        _ => 0.0026,
    }
}

/// Weekly (update 3, F18): every other team's players break their own head
/// coach's rules (`ai_book`), at the risk yours run; his staff sees them
/// as his discipline lets him, and he answers by his own standard: the
/// fair line for the offense and the record, a notch harder for a strict
/// coach and softer for a loose one, and a loose coach goes easier on a
/// star (proposed, not confirmed).
fn ai_week(w: &mut World, d: Date) {
    let seat = Team::seat(w);
    let mut teams: Vec<Team> = Vec::new();
    if rules_season(w, Level::Nfl, d) {
        teams.extend((0..w.clubs.len()).map(Team::Nfl));
    }
    if rules_season(w, Level::College, d) {
        teams.extend((0..w.programs.len()).map(Team::College));
        teams.extend((0..w.high_schools.len()).map(Team::High));
    }
    let year = w.year;
    let mut acts: Vec<(Team, usize, RuleKind, Response)> = Vec::new();
    for t in teams {
        if Some(t) == seat {
            continue;
        }
        let roster = t.roster(w);
        if roster.is_empty() {
            continue;
        }
        let level = t.level();
        let coach = t.head_coach(w).filter(|&c| c < w.coaches.len());
        let book = coach.map_or_else(|| standard(level), |c| ai_book(w, c, level));
        let kinds: Vec<(RuleKind, u8)> = book.rules.iter().copied().filter(|x| x.0.at(level)).collect();
        if kinds.is_empty() {
            continue;
        }
        let disc = coach.map_or(10.0, |c| w.coaches[c].ability().discipline);
        let see = (0.45 + 0.025 * disc as f32).clamp(0.4, 0.95);
        let room = talks::team_mood(w, t);
        let base = rules_base(level);
        for i in roster {
            let id = w.people[i].player.id;
            if dismissed(w, id, d) {
                continue;
            }
            if super::roll(w, 0xA17E ^ d.0 as u64, id as u64) as f32 >= risk(w, i, &book, room, base) {
                continue;
            }
            if super::roll(w, 0xA180 ^ d.0 as u64, id as u64) as f32 > see {
                continue;
            }
            let tot: f32 = kinds.iter().map(|x| x.1 as f32).sum();
            let mut u = super::roll(w, 0xA17F ^ d.0 as u64, id as u64) as f32 * tot;
            let mut kind = kinds[0].0;
            for (k, s) in &kinds {
                if u < *s as f32 {
                    kind = *k;
                    break;
                }
                u -= *s as f32;
            }
            let prior = w.folk.ai_priors.get(&id).filter(|x| x.1 >= year - 1).map_or(0, |x| x.0);
            let shift = if disc >= 14.0 {
                1
            } else if disc <= 6.0 {
                -1
            } else {
                0
            };
            let soft = (disc < 12.0 && is_star(w, i)) as i32;
            let sev = (kind.weight() as i32 + prior.min(3) as i32 - 1 + shift - soft).clamp(0, 6);
            let resp = match sev {
                0 => Response::Talk,
                1 if level == Level::Nfl => Response::Fine,
                1 => Response::Running,
                2 => Response::BenchStart,
                3 => Response::BenchGame,
                4 => Response::Suspend(1),
                5 => Response::Suspend(2),
                // Released only for conduct or drugs; otherwise the longest
                // suspension.
                _ if level == Level::Nfl && !matches!(kind, RuleKind::Conduct | RuleKind::DrugTests) => {
                    Response::Suspend(3)
                }
                _ => Response::Dismiss,
            };
            acts.push((t, i, kind, resp));
        }
    }
    for (t, i, kind, resp) in acts {
        let id = w.people[i].player.id;
        let level = t.level();
        let e = w.folk.ai_priors.entry(id).or_insert((0, year));
        e.0 = if e.1 >= year - 1 { e.0.saturating_add(1) } else { 1 };
        e.1 = year;
        let p = &mut w.people[i];
        p.mood = (p.mood - 0.05 * resp.severity() as f32 / 3.0).max(-1.0);
        match resp {
            Response::Fine => {
                let fine = nfl_fine(year, kind);
                if let Some(c) = w.people[i].pro.as_mut() {
                    c.earnings = (c.earnings - fine / 1.0e6).max(0.0);
                }
            }
            Response::BenchGame | Response::Suspend(_) => {
                let n = if let Response::Suspend(n) = resp { n } else { 1 };
                if level == Level::Nfl {
                    if let Some(c) = w.people[i].pro.as_mut() {
                        c.suspended += n.min(4) as u32;
                    }
                } else {
                    let until = after_games(w, t, n);
                    w.folk.ai_out.insert(id, until);
                }
            }
            Response::Dismiss => dismiss(w, i, level),
            _ => {}
        }
        if (3..6).contains(&resp.severity()) {
            let p = &mut w.people[i];
            let age = p.row_age();
            let mut words = resp.words(level);
            if let Some(c) = words.get_mut(0..1) {
                c.make_ascii_uppercase();
            }
            let kind = kind.name().to_lowercase();
            p.note_again(format!("{words} at {age} for a violation of team rules ({kind})."), || {
                format!("{words} again at {age} for a violation of team rules ({kind}).")
            });
        }
    }
    // Old records fade: a break two seasons back no longer counts.
    if d.weekday() == Weekday::Mon && w.folk.ai_priors.len() > 4096 {
        w.folk.ai_priors.retain(|_, x| x.1 >= year - 1);
    }
}

/// Weekly: your players break your rules, and your staff sees some of them.
fn your_week(w: &mut World, d: Date) {
    let Some(team) = Team::seat(w) else { return };
    let level = team.level();
    if !w.folk.rules.set && w.folk.rules.rules.is_empty() {
        w.folk.rules = Rulebook { council: std::mem::take(&mut w.folk.rules.council), ..standard(level) };
    }
    if !rules_season(w, level, d) {
        return;
    }
    // Weekly base per player (proposed, not confirmed): about one break a
    // month on a hundred-man college roster with a firm standard book.
    let base = rules_base(level);
    let room = talks::team_mood(w, team);
    let book = w.folk.rules.clone();
    let roster = team.roster(w);
    // How much your staff sees (topic 8): your discipline and the position
    // coaches' relationships.
    let you =
        w.career.as_ref().map(|c| c.coach).and_then(|c| w.coaches.get(c)).map_or(10.0, |c| c.ability().discipline);
    let see = (0.45 + 0.025 * you as f32).clamp(0.4, 0.95);
    let open = w.folk.cases.len();
    if open >= 3 {
        return;
    }
    for i in roster {
        let id = w.people[i].player.id;
        if dismissed(w, id, d) {
            continue;
        }
        let r = risk(w, i, &book, room, base);
        if super::roll(w, 0x7B1E ^ d.0 as u64, id as u64) as f32 >= r {
            continue;
        }
        // Which rule: weighted by the book (strict rules catch more).
        let kinds: Vec<(RuleKind, u8)> = book.rules.iter().copied().filter(|x| x.0.at(level)).collect();
        if kinds.is_empty() {
            continue;
        }
        let tot: f32 = kinds.iter().map(|x| x.1 as f32).sum();
        let mut u = super::roll(w, 0x7B1F ^ d.0 as u64, id as u64) as f32 * tot;
        let mut kind = kinds[0].0;
        for (k, s) in &kinds {
            if u < *s as f32 {
                kind = *k;
                break;
            }
            u -= *s as f32;
        }
        let seen = super::roll(w, 0x7B20 ^ d.0 as u64, id as u64) as f32 <= see;
        if !seen {
            continue;
        }
        let prior = w.folk.rules.rulings.iter().filter(|x| x.id == id).count().min(9) as u8;
        let name = w.people[i].name.clone();
        let mut s = Scene::new(Kind::Discipline, d, format!("{name} broke a team rule"));
        s.a = id as u64;
        s.b = RuleKind::ALL.iter().position(|x| *x == kind).unwrap_or(0) as u64;
        s.c = prior as i64;
        s.stops = false;
        s.team = Some(team);
        s.place = "Your office".into();
        let sid = scene::add(w, s);
        w.folk.cases.push(Violation { id, kind, date: d, scene: sid, prior });
        if w.folk.cases.len() >= 3 {
            break;
        }
    }
    // A long strict list wears on veterans (topic 8).
    if level == Level::Nfl && book.load() > 18.0 {
        for &i in &team.roster(w) {
            if w.people[i].player.age >= 28 {
                let p = &mut w.people[i];
                p.mood = (p.mood - 0.01 * (book.load() - 18.0) / 6.0).max(-1.0);
            }
        }
    }
    // A social media ban costs players NIL money from 2021 (topic 8/9).
    if level == Level::College
        && w.year >= 2021
        && book.strict(RuleKind::Social) >= 3
        && d.weekday() == Weekday::Mon
        && d.day() <= 7
    {
        for &i in &team.roster(w) {
            if w.people[i].player.personality.traits.get(Trait::Greed) >= 13.0 {
                talks::mood_event(w, w.people[i].player.id, Reason::Discipline, -0.04);
            }
        }
    }
}

fn violation_open(w: &mut World, s: &mut Scene) {
    let id = s.a as u32;
    let Some(i) = super::person(w, id) else {
        s.end();
        return;
    };
    let level = s.team.map_or(Level::College, |t| t.level());
    let kind = RuleKind::ALL[s.b as usize % RuleKind::ALL.len()];
    s.who.push(Who::Player(id));
    let p = &w.people[i];
    let star = is_star(w, i);
    s.narrate(format!(
        "Your staff tells you {} {}. {}",
        player_link(w, i),
        kind.what(level),
        match s.c {
            0 => "It's the first time.".to_string(),
            1 => "It's the second time this year.".to_string(),
            n => format!("It's the {} time.", ordinal(n as usize + 1)),
        }
    ));
    // The precedent: how you handled the same rule before.
    if let Some(r) = w.folk.rules.rulings.iter().rev().find(|r| r.kind == kind && r.id != id) {
        let who = super::person(w, r.id).map_or_else(|| "a player".into(), |j| player_link(w, j));
        s.narrate(format!("Last time someone broke it, {who} got {}. The room remembers.", r.response.words(level)));
    }
    if star {
        s.narrate(format!(
            "He's a {} {}; the room is watching what you do.",
            if p.last_role == Some(PlayingTime::Starter) { "starter" } else { "big name" },
            p.player.position.name()
        ));
    }
    let council = !w.folk.rules.council.is_empty();
    s.pick(500, "Talk to him and warn him", "Warn", "firm", "The lightest touch. Fair for a first small offense.");
    match level {
        Level::Nfl => {
            let fine = nfl_fine(w.year, kind);
            s.pick(
                501,
                format!("Fine him (${}, the CBA schedule)", super::facts::thousands(fine as u32)),
                "Fine",
                "firm",
                "The standard answer in the league; warning letters come first for small things.",
            );
            s.pick(
                503,
                "Deactivate him for a game",
                "Bench",
                "firm",
                "Counts as discipline under the CBA. The union is told.",
            );
            s.pick(
                504,
                "Suspend him two weeks for conduct detrimental",
                "Suspend",
                "fiery",
                "Unpaid. Inside the four-week cap.",
            );
            s.pick(
                507,
                "Suspend him six weeks",
                "Suspend",
                "fiery",
                "Past the CBA's four weeks: he files a grievance and an arbitrator may overturn it.",
            );
            s.pick(505, "Release him", "Dismiss", "fiery", "Section 5's cap rules apply. Gone.");
        }
        _ => {
            s.pick(
                502,
                "Five a.m. running and lost privileges",
                "Running",
                "firm",
                "The college and high school standard.",
            );
            s.pick(
                506,
                "Bench him for the start",
                "Bench",
                "firm",
                "He plays, but everyone sees him on the sideline first.",
            );
            s.pick(503, "Sit him for the next game", "Bench", "firm", "Real games missed.");
            s.pick(
                504,
                "Suspend him for two games",
                "Suspend",
                "fiery",
                "A real price; the media hears 'violation of team rules'.",
            );
            s.pick(
                505,
                "Dismiss him from the team",
                "Dismiss",
                "fiery",
                if level == Level::College {
                    "He can transfer. The room learns where the line is."
                } else {
                    "He's off the team. His parents will call the principal."
                },
            );
        }
    }
    if council {
        s.pick(
            508,
            "Let the leadership council decide",
            "Council",
            "calm",
            "The players own it; they're often harder on teammates than you are.",
        );
    }
}

fn ordinal(n: usize) -> &'static str {
    match n {
        1 => "first",
        2 => "second",
        3 => "third",
        4 => "fourth",
        _ => "latest",
    }
}

fn is_star(w: &World, i: usize) -> bool {
    let p = &w.people[i];
    p.last_role == Some(PlayingTime::Starter)
        && (p.stars >= 4
            || p.player.position == crate::positions::Position::QB
            || p.pro.as_ref().is_some_and(|c| c.seasons.iter().any(|s| s.pro_bowl)))
}

fn violation_answer(w: &mut World, s: &mut Scene, code: u32) {
    let id = s.a as u32;
    let kind = RuleKind::ALL[s.b as usize % RuleKind::ALL.len()];
    let resp = match code {
        500 => Response::Talk,
        501 => Response::Fine,
        502 => Response::Running,
        506 => Response::BenchStart,
        503 => Response::BenchGame,
        504 => Response::Suspend(2),
        507 => Response::Suspend(6),
        505 => Response::Dismiss,
        _ => Response::Council,
    };
    let resp = if resp == Response::Council { council_call(w, id, kind, s.c as u8) } else { resp };
    apply(w, s, id, kind, resp, false);
}

/// What the council hands down: players are often harder on a teammate
/// than the coach is, softer on a captain (proposed, not confirmed).
fn council_call(w: &World, id: u32, kind: RuleKind, prior: u8) -> Response {
    let star = super::person(w, id).is_some_and(|i| is_star(w, i));
    let sev = kind.weight() + prior + if star { 0 } else { 1 };
    match sev {
        0..=1 => Response::Running,
        2 => Response::BenchStart,
        3 => Response::BenchGame,
        4 => Response::Suspend(2),
        _ => Response::Suspend(3),
    }
}

/// Carries out a ruling: games missed, money, and how he and the room take
/// it (fairness and consistency, topic 8).
fn apply(w: &mut World, s: &mut Scene, id: u32, kind: RuleKind, resp: Response, staff: bool) {
    let Some(i) = super::person(w, id) else {
        s.end();
        return;
    };
    let team = s.team.or(Team::seat(w));
    let level = team.map_or(Level::College, |t| t.level());
    let star = is_star(w, i);
    let name = w.people[i].name.clone();
    let prior = s.c.max(0) as u8;
    // Fair: severity near what the offense and his record call for.
    let due = (kind.weight() + prior.min(3)) as i32 - 1;
    let sev = resp.severity() as i32;
    let gap = sev - due;
    // Consistency: the same rule, a different answer for a star and a backup.
    let mut uneven = 0;
    for r in w.folk.rules.rulings.iter().rev().take(30).filter(|r| r.kind == kind && r.id != id) {
        let d = sev - r.response.severity() as i32;
        if star && !r.star && d < 0 {
            uneven += 1;
        }
        if !star && r.star && d > 0 {
            uneven += 1;
        }
    }
    let p = &w.people[i];
    let tr = &p.player.personality.traits;
    let ego = tr.z(Trait::Ego) as f32;
    let temper = tr.z(Trait::Temper) as f32;
    let coachable = tr.z(Trait::Professionalism) as f32 + 0.5 * tr.z(Trait::Adaptability) as f32;
    let fair = -(gap.abs() as f32) * 0.05;
    let harsh = gap.max(0) as f32;
    let dt = 0.03 + fair - harsh * 0.03 * (ego + temper).max(0.0) + 0.02 * coachable;
    let dm = -0.05 * sev as f32 / 3.0 - harsh * 0.04 * ego.max(0.0);
    let pp = &mut w.people[i];
    pp.trust = (pp.trust + dt).clamp(0.0, 1.0);
    talks::mood_event(w, id, Reason::Discipline, dm);
    // A firm, fair answer from a respected coach raises professionalism
    // over time (topic 1).
    if gap.abs() <= 1
        && sev >= 1
        && super::roll(w, 0xF41E, id as u64 ^ w.today.0 as u64) < 0.25 + 0.1 * coachable as f64
    {
        w.people[i].player.personality.traits.shift(Trait::Professionalism, 0.5);
    }
    // The room (topic 8): fair and consistent lifts trust across the roster;
    // a double standard costs it.
    if let Some(t) = team {
        let roster = t.roster(w);
        let room_dt = if uneven > 0 {
            -0.015 * uneven.min(3) as f32
        } else if gap.abs() <= 1 {
            0.006
        } else {
            -0.004
        };
        for j in roster {
            if j != i {
                w.people[j].trust = (w.people[j].trust + room_dt).clamp(0.0, 1.0);
            }
        }
    }
    w.folk.mine.hard = (w.folk.mine.hard + 0.04 * (sev - 2) as f32).clamp(-1.0, 1.0);
    // Games and money.
    let mut extra = String::new();
    match resp {
        Response::Fine => {
            let fine = nfl_fine(w.year, kind) * (1.0 + prior as f64);
            if let Some(c) = w.people[i].pro.as_mut() {
                c.earnings = (c.earnings - fine / 1.0e6).max(0.0);
            }
            extra = format!(" (${})", super::facts::thousands(fine as u32));
        }
        Response::BenchStart => {
            w.folk.benched.push((id, after_games(w, team.unwrap_or(Team::College(0)), 1)));
        }
        Response::BenchGame | Response::Suspend(_) => {
            let n = match resp {
                Response::Suspend(n) => n,
                _ => 1,
            };
            if level == Level::Nfl {
                if let Some(c) = w.people[i].pro.as_mut() {
                    // Over the four-week cap the player grieves (Article 42).
                    c.suspended += n.min(4) as u32;
                    if n > 4 {
                        super::courts::grievance(w, id, n as u32);
                        extra = " (he filed a grievance with the union)".into();
                    }
                }
            } else if let Some(t) = team {
                let until = after_games(w, t, n);
                w.folk.out.retain(|x| x.0 != id);
                w.folk.out.push((id, until, format!("{} for {}", resp.words(level), kind.name().to_lowercase())));
            }
            talks::mood_event(w, id, Reason::Benched, -0.06 * n as f32);
        }
        Response::Dismiss => {
            dismiss(w, i, level);
        }
        _ => {}
    }
    w.folk.rules.rulings.push(Ruling { date: w.today, id, kind, response: resp, star });
    if w.folk.rules.rulings.len() > 200 {
        w.folk.rules.rulings.remove(0);
    }
    w.folk.cases.retain(|v| v.id != id || v.scene != s.id);
    let words = resp.words(level);
    s.outcome.push(format!(
        "{name}: {words}{extra}{}.{}",
        if staff { ", by your staff's standard" } else { "" },
        if uneven > 0 { " The room noticed you didn't treat it the way you treated the last one." } else { "" }
    ));
    let mut pen = scene::pen(w, s, 3);
    let spk = super::voice::Speaker::player(w, i);
    let f = super::voice::Facts::new().with("coach", format!("Coach {}", scene::your_last(w)));
    let key = if gap > 1 {
        "rules.react.harsh"
    } else if gap < -1 {
        "rules.react.soft"
    } else {
        "rules.react.fair"
    };
    let line = pen.line(&[key], &spk, &f);
    pen.put(w);
    if !staff && resp != Response::Dismiss {
        s.them(
            &name,
            if line.is_empty() {
                (if gap > 1 { "That's a lot for what I did, Coach." } else { "Yes sir. Understood." }).to_string()
            } else {
                line
            },
            None,
        );
    }
    // A public suspension reaches the news.
    if sev >= 3
        && let Some(t) = team
    {
        let text = format!(
            "{name} {} for a violation of team rules.",
            if level == Level::Nfl && matches!(resp, Response::BenchGame) { "is deactivated" } else { "is suspended" }
        );
        super::post_mine(w, w.today, Loudness::Small, false, "people: discipline", &text);
        let _ = t;
    }
    s.end();
}

/// Days a dismissed college or high school player is off the team: the
/// rest of the season, bowls and playoffs too, until his next school takes
/// him (minor bug 116: dismissed players kept playing, and were dismissed
/// again).
const DISMISSED_DAYS: i32 = 200;

/// Whether he is off his team for good this season (a dismissal, never the
/// few weeks of a suspension).
fn dismissed(w: &World, id: u32, d: Date) -> bool {
    w.folk.ai_out.get(&id).is_some_and(|&x| d.days_to(x) > DISMISSED_DAYS / 2)
}

fn dismiss(w: &mut World, i: usize, level: Level) {
    let d = w.today;
    if level != Level::Nfl {
        w.folk.ai_out.insert(w.people[i].player.id, d.add(DISMISSED_DAYS));
    }
    match level {
        Level::Nfl => {
            crate::world::pro::release(w, i, d, "released for a violation of team rules");
        }
        Level::College => {
            let p = &mut w.people[i];
            p.on_notice = true;
            p.cal.trouble = true;
            p.cal.lean = (p.cal.lean + 0.6).min(0.98);
            p.cal.lean_events.push((d, "dismissed from the team", 3.0));
            p.note(format!("Dismissed from the team at {} for a violation of team rules.", p.player.age));
        }
        _ => {
            let p = &mut w.people[i];
            p.cal.trouble = true;
            p.note(format!("Removed from the team at {}.", p.player.age));
        }
    }
}

pub(crate) fn open(w: &mut World, s: &mut Scene) {
    match s.kind {
        Kind::Ritual => ritual_open(w, s),
        _ => violation_open(w, s),
    }
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32, picked: &Pick) {
    let _ = picked;
    match s.kind {
        Kind::Ritual => ritual_answer(w, s, code),
        _ => violation_answer(w, s, code),
    }
}

/// Your staff applies your book's standard: firm rules a step up from a
/// talk, strict ones a game.
pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    match s.kind {
        Kind::Ritual => {
            s.handed = Some("the veterans".into());
            s.outcome.push("You left it to the veterans. Rituals ran the way they always have.".into());
            w.folk.rules.rituals = 2;
            ritual_risk(w, s);
        }
        _ => {
            let id = s.a as u32;
            let kind = RuleKind::ALL[s.b as usize % RuleKind::ALL.len()];
            let level = s.team.map_or(Level::College, |t| t.level());
            let strict = w.folk.rules.strict(kind);
            let resp = match (strict + s.c.max(0) as u8, level) {
                (0..=1, _) => Response::Talk,
                (2, Level::Nfl) => Response::Fine,
                (2, _) => Response::Running,
                (3, Level::Nfl) => Response::Fine,
                (3, _) => Response::BenchStart,
                _ => Response::BenchGame,
            };
            s.handed = Some("your staff".into());
            apply(w, s, id, kind, resp, true);
        }
    }
}

// ---------------------------------------------------------------------------
// P8: rookie rituals, and where they cross the line.

fn ritual_open(w: &mut World, s: &mut Scene) {
    let Some(team) = s.team.or(Team::seat(w)) else {
        s.end();
        return;
    };
    let leaders = super::takeover::captains_of(w, team);
    if let Some(&c) = leaders.first() {
        s.who.push(Who::Player(w.people[c].player.id));
        s.narrate(format!("{} stops by with the veterans' plans for the rookies.", player_link(w, c)));
    }
    let (what, line) = match team.level() {
        Level::Nfl => (
            "Rookies carry the veterans' pads off the field, sing their college fight songs at dinner, bring breakfast on Saturdays, and the rookie dinner: the veterans order, the rookies pay. Last year's bill was over $40,000.",
            "It's tradition, Coach. Everybody went through it. It's how the young guys become part of this.",
        ),
        Level::College => (
            "Freshmen carry equipment, sing in front of the team and get the freshman haircut before the first game. Some of the older players want a night of their own on top of it.",
            "It's how it's always been. The freshmen expect it.",
        ),
        _ => (
            "Freshmen and sophomores carry the water and the bags, and the seniors want a night at the lake for 'team bonding'.",
            "Everybody did it when we were freshmen.",
        ),
    };
    s.narrate(what.to_string());
    if let Some(&c) = leaders.first() {
        let n = w.people[c].name.clone();
        s.them(&n, line, None);
    }
    s.pick(
        600,
        "Fine, within limits: no money, no hands, nothing off campus",
        "Set limits",
        "firm",
        "The bonding without the risk; a few veterans grumble.",
    );
    s.pick(
        601,
        "It's your room. Keep it reasonable.",
        "Hand him something",
        "calm",
        "The veterans love it. Where it crosses the line depends on who's leading.",
    );
    s.pick(602, "None of it. Not on my team.", "Warn", "firm", "No risk at all. The veterans think you don't get it.");
}

fn ritual_answer(w: &mut World, s: &mut Scene, code: u32) {
    let Some(team) = s.team.or(Team::seat(w)) else {
        s.end();
        return;
    };
    let roster = team.roster(w);
    let (rule, vets, kids, words) = match code {
        600 => (1, 0.0, 0.01, "Rituals within limits."),
        601 => (2, 0.03, 0.0, "The veterans run the rituals."),
        _ => (0, -0.04, 0.02, "No rituals."),
    };
    w.folk.rules.rituals = rule;
    for i in roster {
        let p = &mut w.people[i];
        let rookie = p.player.age <= 22 && p.pro.as_ref().is_none_or(|c| c.seasons.is_empty());
        p.mood = (p.mood + if rookie { kids } else { vets }).clamp(-1.0, 1.0);
    }
    s.outcome.push(words.into());
    if rule == 2 {
        ritual_risk(w, s);
    }
    s.end();
}

/// Where a ritual crosses the line (P8): left to the veterans, a room led
/// by loose, big-ego leaders can turn tradition into hazing (the 2013
/// Dolphins case; Northwestern 2023). Rare (proposed, not confirmed: about
/// one in twelve such rooms a year).
fn ritual_risk(w: &mut World, s: &mut Scene) {
    let Some(team) = s.team.or(Team::seat(w)) else { return };
    let leaders = super::takeover::captains_of(w, team);
    let bad = leaders
        .iter()
        .map(|&i| {
            let t = &w.people[i].player.personality.traits;
            (t.z(Trait::Ego) - t.z(Trait::Professionalism)) as f32
        })
        .sum::<f32>()
        / leaders.len().max(1) as f32;
    let chance = (0.08 + 0.12 * bad).clamp(0.01, 0.3);
    if (super::roll(w, 0x4A21, w.year as u64) as f32) < chance {
        let rookie =
            team.roster(w).into_iter().find(|&i| w.people[i].player.age <= 21 && w.people[i].bg.stability < 9.0);
        if let Some(r) = rookie {
            let id = w.people[r].player.id;
            talks::mood_event(w, id, Reason::Room, -0.4);
            w.people[r].trust = (w.people[r].trust - 0.2).max(0.0);
            let text = format!(
                "A rookie ritual crossed the line: {} told the trainers what the veterans put him through.",
                w.people[r].name
            );
            s.outcome.push(text.clone());
            super::post_mine(w, w.today.add(9), Loudness::Big, false, "people: hazing", &text);
            let boss = super::cast::boss_role(team);
            let b = super::cast::get(w, super::cast::CastRef::new(team, boss, 0));
            w.folk.mine.nudge(b.key(), -0.08);
        }
    }
}

// ---------------------------------------------------------------------------
// P2: league discipline on the real scale.

/// A league or association ruling on the field: a fine for a hit, an
/// ejection, a suspension for repeat hits.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Fine {
    pub date: Date,
    pub id: u32,
    pub level: Level,
    pub what: String,
    /// Dollars (NFL), 0 for an ejection.
    pub amount: u32,
    pub games: u8,
    pub appealed: bool,
    pub reduced: bool,
}

/// League discipline (P2): the season's rulings (newest 300 kept) and the
/// year's counts.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LeagueBook {
    pub recent: Vec<Fine>,
    /// (year, NFL fines, NFL suspensions for hits, college targeting
    /// ejections, high school ejections).
    pub years: Vec<(i32, u32, u32, u32, u32)>,
}

/// The NFL's fine for an on-field foul (the league's schedule: 2025 first
/// offense $11,817 for unnecessary roughness or a face mask, $16,883 for
/// roughing the passer, $25,325 for a hit on a defenseless player or use of
/// the helmet; about 3% a year, and half as much before the 2011 CBA).
fn hit_fine(year: i32, kind: u8) -> u32 {
    let base = match kind {
        0 => 11817.0,
        1 => 16883.0,
        _ => 25325.0,
    };
    let scale = 1.03f64.powi(year - 2025) * if year < 2011 { 0.55 } else { 1.0 };
    (base * scale).round() as u32
}

/// Weekly in season (Tuesdays): the league office's rulings on last week's
/// games. Rates from the real record (proposed where none is published):
/// the NFL fined about 250 to 350 hits a season, suspended two to four
/// players for repeat hits; FBS calls about 0.2 targeting ejections a game;
/// high school ejections sit the next game under NFHS rules, at every school.
fn league_week(w: &mut World, d: Date) {
    if d.weekday() != Weekday::Tue {
        return;
    }
    // The Tuesday after a regular-season Sunday (the playoffs' few fines
    // aside: every club's roster is read, playing or not).
    let nfl_season = w.dates.nfl_weeks.iter().any(|&s| d > s && d <= s.add(6));
    let cfb_season = w.dates.cfb_weeks.first().is_some_and(|&k| d > k) && d <= w.dates.bowl_season.1.add(2);
    let year = w.year;
    let mut entries: Vec<Fine> = Vec::new();
    let mut counts = (0u32, 0u32, 0u32, 0u32);
    let seat = Team::seat(w);
    let mine: Vec<u32> = seat.map_or(Vec::new(), |t| t.roster(w).iter().map(|&i| w.people[i].player.id).collect());
    if nfl_season {
        // About 16 hits fined a week across the league.
        for t in 0..w.clubs.len() {
            for &i in &w.clubs[t].roster.clone() {
                let p = &w.people[i];
                let Some(c) = p.pro.as_ref() else { continue };
                if c.status != crate::world::pro::Status::Roster || c.out_weeks > 0 {
                    continue;
                }
                let pos = p.player.position;
                let hitter = matches!(
                    pos,
                    crate::positions::Position::S
                        | crate::positions::Position::CB
                        | crate::positions::Position::LB
                        | crate::positions::Position::EDGE
                        | crate::positions::Position::DT
                );
                if !hitter {
                    continue;
                }
                let temper = p.player.personality.traits.z(Trait::Temper);
                // About 16 a week from some 830 rostered defenders (minor bug
                // 151: 0.0055 gave about 82 a season against 250-350).
                let r = 0.019 * (1.0 + 0.6 * temper).max(0.3);
                if super::roll(w, 0x41E0 ^ d.0 as u64, p.player.id as u64) >= r {
                    continue;
                }
                let kind = (super::roll(w, 0x41E1 ^ d.0 as u64, p.player.id as u64) * 3.0) as u8;
                let prior = w
                    .folk
                    .discipline
                    .recent
                    .iter()
                    .filter(|f| f.id == p.player.id && f.date.year() >= year - 1)
                    .count();
                let amount = hit_fine(year, kind) * (1 + prior as u32).min(2);
                let games = if prior >= 2 && kind == 2 { 1 } else { 0 };
                let what = match kind {
                    0 => "unnecessary roughness",
                    1 => "roughing the passer",
                    _ => "a hit on a defenseless player",
                };
                entries.push(Fine {
                    date: d,
                    id: p.player.id,
                    level: Level::Nfl,
                    what: what.into(),
                    amount,
                    games,
                    appealed: false,
                    reduced: false,
                });
                counts.0 += 1;
                if games > 0 {
                    counts.1 += 1;
                }
            }
        }
    }
    if cfb_season {
        // Targeting: about 0.2 ejections a game. A second-half ejection
        // carries into the first half of the next game.
        let seat_prog = match seat {
            Some(Team::College(p)) => Some(p),
            _ => None,
        };
        for p in 0..w.programs.len() {
            if w.programs[p].division != crate::world::data::Division::FBS && Some(p) != seat_prog {
                continue;
            }
            if super::roll(w, 0x7A60 ^ d.0 as u64, p as u64) >= 0.2 {
                continue;
            }
            let pool: Vec<usize> = w.programs[p]
                .roster
                .iter()
                .copied()
                .filter(|&i| {
                    let pp = &w.people[i];
                    pp.stage == Stage::College
                        && pp.last_role == Some(PlayingTime::Starter)
                        && matches!(
                            pp.player.position,
                            crate::positions::Position::S
                                | crate::positions::Position::CB
                                | crate::positions::Position::LB
                        )
                })
                .collect();
            if pool.is_empty() {
                continue;
            }
            let k = (super::roll(w, 0x7A61 ^ d.0 as u64, p as u64) * pool.len() as f64) as usize;
            let id = w.people[pool[k.min(pool.len() - 1)]].player.id;
            entries.push(Fine {
                date: d,
                id,
                level: Level::College,
                what: "ejected for targeting".into(),
                amount: 0,
                games: 0,
                appealed: false,
                reduced: false,
            });
            counts.2 += 1;
        }
    }
    // High school ejections: the next game out by rule, at every school
    // (B2: only your own team's sat out before). Yours make the news and
    // the record; an AI school's player sits his school's next game.
    if d >= w.dates.hs_school_start && d <= w.dates.december_graduation {
        let yours = match seat {
            Some(Team::High(h)) => Some(h),
            _ => None,
        };
        for h in 0..w.high_schools.len() {
            if Some(h) == yours || super::roll(w, 0x4E5 ^ d.0 as u64, h as u64) >= 0.08 {
                continue;
            }
            let r = &w.high_schools[h].roster;
            if r.is_empty() {
                continue;
            }
            let k = (super::roll(w, 0x4E6 ^ d.0 as u64, h as u64) * r.len() as f64) as usize;
            let id = w.people[r[k.min(r.len() - 1)]].player.id;
            let until = after_games(w, Team::High(h), 1);
            w.folk.ai_out.insert(id, until);
        }
    }
    if let Some(Team::High(h)) = seat
        && d >= w.dates.hs_school_start
        && d <= w.dates.december_graduation
        && super::roll(w, 0x4E5 ^ d.0 as u64, h as u64) < 0.08
    {
        let r = Team::High(h).roster(w);
        if !r.is_empty() {
            let k = (super::roll(w, 0x4E6 ^ d.0 as u64, h as u64) * r.len() as f64) as usize;
            let id = w.people[r[k.min(r.len() - 1)]].player.id;
            entries.push(Fine {
                date: d,
                id,
                level: Level::HighSchool,
                what: "ejected for fighting".into(),
                amount: 0,
                games: 1,
                appealed: false,
                reduced: false,
            });
            counts.3 += 1;
        }
    }
    for f in entries {
        let Some(i) = super::person(w, f.id) else { continue };
        if f.level == Level::Nfl
            && let Some(c) = w.people[i].pro.as_mut()
        {
            c.earnings = (c.earnings - f.amount as f64 / 1.0e6).max(0.0);
            c.suspended += f.games as u32;
        }
        if f.level == Level::HighSchool
            && f.games > 0
            && let Some(t) = seat
        {
            let until = after_games(w, t, 1);
            w.folk.out.push((f.id, until, "ejected last week: out the next game by state rule".into()));
        }
        if mine.contains(&f.id) {
            let text = if f.amount > 0 {
                format!(
                    "The league fined {} ${} for {}{}.",
                    w.people[i].name,
                    super::facts::thousands(f.amount),
                    f.what,
                    if f.games > 0 { format!(" and suspended him {} game", f.games) } else { String::new() }
                )
            } else {
                format!("{} was {}.", w.people[i].name, f.what)
            };
            super::post_mine(w, d, Loudness::Small, false, "league: discipline", &text);
            // Your players' fines can be appealed (P2; the hearing officers
            // reduce about a third of the fines appealed).
            if f.amount > 0 {
                super::courts::fine_appeal(w, f.id, f.amount, d);
            }
        }
        w.folk.discipline.recent.push(f);
    }
    let len = w.folk.discipline.recent.len();
    if len > 300 {
        w.folk.discipline.recent.drain(..len - 300);
    }
    if counts != (0, 0, 0, 0) {
        match w.folk.discipline.years.iter_mut().find(|y| y.0 == year) {
            Some(y) => {
                y.1 += counts.0;
                y.2 += counts.1;
                y.3 += counts.2;
                y.4 += counts.3;
            }
            None => w.folk.discipline.years.push((year, counts.0, counts.1, counts.2, counts.3)),
        }
        if w.folk.discipline.years.len() > 40 {
            w.folk.discipline.years.remove(0);
        }
    }
}

// ---------------------------------------------------------------------------

pub(crate) fn day(w: &mut World, d: Date) {
    w.folk.out.retain(|x| x.1 >= d);
    w.folk.benched.retain(|x| x.1 >= d);
    w.folk.ai_out.retain(|_, x| *x >= d);
    league_week(w, d);
    if d.weekday() == Weekday::Mon {
        ai_week(w, d);
    }
    if w.seat.is_some() && d.weekday() == Weekday::Mon {
        your_week(w, d);
    }
    // Rookie rituals at camp (P8).
    if let Some(team) = Team::seat(w) {
        let camp = match team.level() {
            Level::Nfl => w.dates.rookies_report.add(3),
            Level::College => w.dates.fall_camp.add(2),
            _ => w.dates.hs_school_start.add(-14),
        };
        if d == camp && w.folk.rules.rituals != 0 {
            let mut s = Scene::new(Kind::Ritual, d, "The rookies and the veterans");
            s.stops = false;
            s.team = Some(team);
            s.place = "The locker room".into();
            scene::add(w, s);
        }
    }
}

/// How your rules change the off-field trouble risk of topic 7 for your
/// players: firm, fair rules lower it, a loose book raises it (proposed,
/// not confirmed: up to 25% either way).
pub fn trouble_factor(w: &World) -> f64 {
    let load = w.folk.rules.load() as f64;
    let fair = {
        let n = w.folk.rules.rulings.len().max(1) as f64;
        let stars = w.folk.rules.rulings.iter().filter(|r| r.star && r.response.severity() >= 2).count() as f64;
        (stars / n).min(0.3)
    };
    (1.15 - 0.02 * load.min(20.0) - 0.3 * fair).clamp(0.75, 1.25)
}
