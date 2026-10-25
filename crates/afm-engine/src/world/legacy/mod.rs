//! Systems to chase (update 2, lane 2; the "Systems to chase" design and
//! boards 34-36): what a career chases at every level. Five shared systems,
//! G1 your ambitions (`ambitions.rs`), G2 your coaching tree (`tree.rs`), G3
//! your scheme as a legacy (`scheme.rs`), G4 your guys for life (`guys.rs`)
//! and G5 the Hall of Fame race (`hall.rs`), and five per level: the NFL's
//! L1-L5 (`nfl.rs`), college's L6-L10 (`college.rs`) and high school's
//! L11-L15 (`hs.rs`).
//!
//! Every system has the same four parts: a goal you can see, choices that
//! risk it, rewards from the world's own currencies (lesson points placed
//! into your ratings, reputation, your boss's trust, ties, story credit) and
//! losses that are real. Nothing here bends an outcome: the rewards are
//! paid for what the world's own results did. The storyline engine carries
//! the big moments.
//!
//! The world's side runs for everyone, in the history too: the state
//! coaches' halls and the national high school hall, the Pro Football
//! Hall's coaches, the AI coaches' ambitions in their job moves. Sizes are
//! Claude's picks (proposed, not confirmed).

use crate::calendar::{Date, Level, Weekday};
use crate::world::World;
use crate::world::coach::{Employer, GROUP_COUNT, Role};
use crate::world::stories::TeamKey;
use crate::world::storylines::catalogue::{K, Rating};

pub mod ambitions;
pub mod college;
pub mod guys;
pub mod hall;
pub mod hs;
pub mod nfl;
pub mod scheme;
pub mod tree;
pub mod view;

/// Answer codes on the chases' letters (`desk::Answer::act`), past the
/// storylines' own (64): acted on the next morning, or when a legacy screen
/// opens.
pub const ACT_OWNER: u8 = 70;
pub const ACT_STRINGS: u8 = 71;
pub const ACT_BOOKING: u8 = 73;
pub const ACT_BOND: u8 = 74;
pub const ACT_GUY: u8 = 75;
pub const ACT_CLINIC: u8 = 76;

/// Everything the chases keep.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Legacy {
    /// Your ambitions, open and closed, and the others' goals you carry.
    pub ambitions: Vec<ambitions::Ambition>,
    pub next_id: u32,
    /// The picker waiting after a hire (board 35).
    pub picker: Option<ambitions::Picker>,
    /// The job the chases last saw you in: (employer code, role code).
    pub job_seen: Option<(u64, u8)>,
    /// Your games, for records against a coach or a rival, résumés and runs.
    pub games: Vec<Played>,
    /// Your finished seasons, one line each (the runs, streaks and steps).
    pub seasons: Vec<Yr>,
    /// Everyone you coached for a season or more (G4).
    pub guys: Vec<guys::Guy>,
    /// Sons of your guys who came to play for you: (son, father).
    pub sons: Vec<(u32, u32)>,
    /// Your scheme as the world reads it (G3).
    pub scheme: scheme::Scheme,
    /// How you grow each assistant (G2): (coach, choice).
    pub grow: Vec<(usize, tree::Grow)>,
    /// Lesson points banked into teaching, by group (G4's developers).
    pub teach: [f32; GROUP_COUNT],
    /// Rewards already paid, by key, so none pays twice.
    pub paid: Vec<u64>,
    /// What the chases paid, newest last (for the screens).
    pub log: Vec<Paid>,
    pub nfl: nfl::Nfl,
    pub college: college::College,
    pub hs: hs::Hs,
    /// The halls beyond the College and Pro Football halls: the state
    /// coaches' halls and the national high school hall (G5).
    pub halls: Vec<hall::Inducted>,
    /// Coaches the Pro Football Hall voted in (G5).
    pub pro_hall: Vec<hall::Inducted>,
    /// The seasons whose review already ran, by level code.
    pub reviewed: Vec<(u8, i32)>,
    /// A game was played since the last look (the milestones look again).
    pub dirty: bool,
    /// Mails from the chases waiting for your answer: (mail id, act).
    pub pending: Vec<(u64, u8)>,
}

/// One of your games.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Played {
    pub season: i32,
    pub date: Date,
    /// Your team then.
    pub team: TeamKey,
    pub opp: TeamKey,
    /// Their head coach.
    pub coach: Option<u32>,
    pub us: u8,
    pub them: u8,
    pub post: bool,
    pub title: bool,
    pub conf: bool,
    pub rivalry: bool,
    /// A playoff game (a bowl is postseason, not a playoff).
    #[serde(default)]
    pub playoff: bool,
}

impl Played {
    pub fn won(&self) -> bool {
        self.us > self.them
    }
    pub fn margin(&self) -> i32 {
        self.us as i32 - self.them as i32
    }
}

/// One of your finished seasons.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Yr {
    pub year: i32,
    pub team: TeamKey,
    pub wins: u16,
    pub losses: u16,
    /// Postseason games played and won (bowls and playoffs).
    pub post: u8,
    pub post_wins: u8,
    /// Playoff games played (a state's, the College Football Playoff, the NFL's).
    pub playoff: u8,
    /// The conference (college), the division (NFL) or the district (high
    /// school).
    pub title: bool,
    /// Won it all: the state title, the national title, the Super Bowl.
    pub champion: bool,
    /// Played for it all: the state final, the title game, the Super Bowl.
    pub final_game: bool,
    /// The final poll rank (college) or the league rank by record (NFL).
    pub rank: Option<u8>,
    /// A high school's class that season ("3A").
    pub class: String,
    /// The place in the division (NFL), 0 when not known.
    #[serde(default)]
    pub place: u8,
    /// Played in the conference title game (college).
    #[serde(default)]
    pub conf_game: bool,
}

impl Yr {
    pub fn pct(&self) -> f64 {
        self.wins as f64 / (self.wins + self.losses).max(1) as f64
    }
}

/// A reward a chase paid.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Paid {
    pub date: Date,
    /// Which system: "G1" ... "L15".
    pub sys: String,
    pub what: String,
    pub lesson: f32,
    /// The rating (or teaching group) the points went into, in words.
    pub into: String,
    pub rep: f32,
}

/// Where lesson points go.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Bank {
    Rating(Rating),
    /// Teaching a position group (`coach::Group` as an index).
    Teach(usize),
}

impl Bank {
    pub fn name(self) -> String {
        match self {
            Bank::Rating(r) => r.name().to_string(),
            Bank::Teach(g) => format!("Teaching {}", crate::world::coach::Group::ALL[g].name().to_lowercase()),
        }
    }
}

/// A stable key for a reward: the system and what it was for.
pub fn key(sys: &str, a: u64, b: u64) -> u64 {
    let h = sys.bytes().fold(0xC4A5_E000_0000_0000u64, |h, x| (h ^ x as u64).wrapping_mul(0x100_0000_01b3));
    (h ^ a.wrapping_mul(0x9E37_79B9_7F4A_7C15)).wrapping_mul(0x100_0000_01b3) ^ b
}

/// Pays a reward once: lesson points (0.25 to 1.00 a step, never more) into
/// a rating or a teaching group, and reputation (of 20). False when it was
/// paid before.
pub(crate) fn pay(w: &mut World, k: u64, sys: &str, what: String, lesson: f32, into: Bank, rep: f64) -> bool {
    if w.career.is_none() || w.legacy.paid.contains(&k) {
        return false;
    }
    w.legacy.paid.push(k);
    let lesson = lesson.clamp(0.0, 1.0);
    if lesson > 0.0 {
        match into {
            Bank::Rating(r) => w.storylines.credit.bank[r.index()] += lesson,
            Bank::Teach(g) => w.legacy.teach[g.min(GROUP_COUNT - 1)] += lesson,
        }
    }
    if rep != 0.0 {
        crate::world::storylines::credit::reputation(w, rep);
    }
    let into = into.name();
    w.legacy.log.push(Paid { date: w.today, sys: sys.to_string(), what, lesson, into, rep: rep as f32 });
    if w.legacy.log.len() > 400 {
        w.legacy.log.remove(0);
    }
    true
}

/// A loss a chase cost: reputation, written in the log.
pub(crate) fn cost(w: &mut World, k: u64, sys: &str, what: String, rep: f64) {
    if w.career.is_none() || w.legacy.paid.contains(&k) {
        return;
    }
    w.legacy.paid.push(k);
    crate::world::storylines::credit::reputation(w, -rep);
    w.legacy.log.push(Paid {
        date: w.today,
        sys: sys.to_string(),
        what,
        lesson: 0.0,
        into: String::new(),
        rep: -rep as f32,
    });
}

/// Your coach and team, when a career runs.
pub(crate) fn you(w: &World) -> Option<(usize, TeamKey)> {
    Some((crate::world::stories::seat_coach(w)?, crate::world::stories::seat_team(w)?))
}

/// Whether you are the head coach where you work.
pub(crate) fn head(w: &World) -> bool {
    w.career.as_ref().is_some_and(|c| c.role.is_none())
}

/// A level's code for the reviewed list.
fn level_code(l: Level) -> u8 {
    match l {
        Level::HighSchool => 0,
        Level::College => 1,
        Level::Nfl => 2,
        _ => 3,
    }
}

/// The day a level's season is reviewed: after the state finals, after the
/// national title game, on the Super Bowl's night.
pub fn review_day(w: &World, l: Level) -> Date {
    match l {
        Level::HighSchool => Date::ymd(w.year, 12, 23),
        Level::Nfl | Level::Other => w.dates.super_bowl,
        _ => w.dates.title_game.add(1),
    }
}

/// A job as a key: (employer code, role code).
fn job_key(e: Employer, r: Option<Role>) -> (u64, u8) {
    let e = match e {
        Employer::HighSchool(s) => s as u64,
        Employer::College(p) => 1 << 40 | p as u64,
        Employer::Nfl(t) => 2 << 40 | t as u64,
        Employer::Other(f) => 3 << 40 | f as u64,
    };
    let r = match r {
        None | Some(Role::HeadCoach) => 0,
        Some(Role::OffensiveCoordinator) => 1,
        Some(Role::DefensiveCoordinator) => 2,
        Some(Role::PositionCoach(g)) => 3 + g as u8,
    };
    (e, r)
}

/// A new job (or the career's first): the ambitions it offers. Run each
/// day and when a screen asks, so the picker is there the day you start.
pub fn hire_check(w: &mut World) {
    let now = w.career.as_ref().and_then(|c| {
        let co = w.coaches.get(c.coach)?;
        co.job.map(|j| job_key(j.employer, c.role))
    });
    if now.is_some() && now != w.legacy.job_seen {
        // A new job's collective starts on its own split (L9).
        if w.legacy.job_seen.is_some() {
            w.legacy.college.split = [0.0; 3];
        }
        w.legacy.job_seen = now;
        ambitions::offer(w);
    }
}

/// Every day: the world's halls on their days, then your chases.
pub(crate) fn day(w: &mut World, d: Date) {
    hall::world_day(w, d);
    college::world_day(w, d);
    if w.career.is_none() {
        return;
    }
    desk_answers(w);
    hire_check(w);
    let Some((_, team)) = you(w) else { return };
    let level = team.level();
    if d.weekday() == Weekday::Mon || std::mem::take(&mut w.legacy.dirty) {
        ambitions::check(w);
    }
    guys::day(w, d);
    match level {
        Level::Nfl => nfl::day(w, d),
        Level::College => college::day(w, d),
        Level::HighSchool => hs::day(w, d),
        _ => {}
    }
    let code = level_code(level);
    if d >= review_day(w, level) && !w.legacy.reviewed.contains(&(code, w.year)) {
        w.legacy.reviewed.push((code, w.year));
        season_review(w);
    }
}

/// After your season: each system looks at the year it was.
fn season_review(w: &mut World) {
    season_line(w);
    guys::season(w);
    tree::season(w);
    scheme::season(w);
    hall::season(w);
    ambitions::check(w);
    ambitions::season(w);
    match you(w).map(|x| x.1.level()) {
        Some(Level::Nfl) => nfl::season(w),
        Some(Level::College) => college::season(w),
        Some(Level::HighSchool) => hs::season(w),
        _ => {}
    }
}

/// Your season in one line, from your games and the level's books.
fn season_line(w: &mut World) {
    let Some((_, team)) = you(w) else { return };
    let year = w.year;
    if w.legacy.seasons.iter().any(|s| s.year == year && s.team == team) {
        return;
    }
    let games: Vec<Played> = w.legacy.games.iter().filter(|g| g.season == year && g.team == team).copied().collect();
    if games.is_empty() {
        return;
    }
    let count = |f: &dyn Fn(&Played) -> bool| games.iter().filter(|g| f(g)).count();
    let place = match team {
        TeamKey::Nfl(t) => live_place(w, t as usize).unwrap_or(0),
        _ => 0,
    };
    let (title, rank, class) = match team {
        TeamKey::College(p) => {
            let p = p as usize;
            let conf = w.programs[p].seasons.iter().any(|s| s.year == year && s.conference_title)
                || games.iter().any(|g| g.conf && g.won());
            let rank = w
                .college
                .as_ref()
                .and_then(|c| c.polls.last())
                .and_then(|poll| poll.top.iter().position(|&t| t == p).map(|k| (k + 1) as u8));
            (conf, rank, String::new())
        }
        TeamKey::Nfl(t) => {
            let t = t as usize;
            let s = w.nfl_season.as_ref();
            let div = s.and_then(|s| s.seed.get(t).copied().flatten()).is_some_and(|k| k <= 4);
            let rank = s.map(|s| {
                let mine = s.wins[t] as f64 - s.losses[t] as f64;
                (1 + (0..s.wins.len())
                    .filter(|&o| s.active[o] && (s.wins[o] as f64 - s.losses[o] as f64) > mine)
                    .count()) as u8
            });
            (div, rank, String::new())
        }
        TeamKey::Hs(h) => {
            let h = h as usize;
            let code = &w.states[w.high_schools[h].state].code;
            let class = crate::world::hs_playoffs::format(code)
                .class_label(w.high_schools[h].classification)
                .unwrap_or("")
                .to_string();
            (crate::world::hs_year::district_champion(w, h), None, class)
        }
        TeamKey::Other(_) => (false, None, String::new()),
    };
    w.legacy.seasons.push(Yr {
        year,
        team,
        wins: count(&|g| g.us > g.them) as u16,
        losses: count(&|g| g.us < g.them) as u16,
        post: count(&|g| g.post) as u8,
        post_wins: count(&|g| g.post && g.won()) as u8,
        playoff: count(&|g| g.playoff) as u8,
        title,
        champion: games.iter().any(|g| g.title && g.won()),
        final_game: games.iter().any(|g| g.title),
        rank,
        class,
        place,
        conf_game: games.iter().any(|g| g.conf),
    });
}

/// Your NFL team's place in its division in the season under way (from the
/// standings: wins, then the playoff seeds).
fn live_place(w: &World, t: usize) -> Option<u8> {
    let s = w.nfl_season.as_ref()?;
    let div = *s.div.get(t)?;
    let key =
        |o: usize| (s.wins[o] as f64 + 0.5 * s.ties[o] as f64, s.seed.get(o).copied().flatten().map_or(0, |k| 20 - k));
    let me = key(t);
    let ahead = (0..s.wins.len())
        .filter(|&o| o != t && s.active[o] && s.div.get(o) == Some(&div))
        .filter(|&o| {
            let k = key(o);
            k.0 > me.0 || (k.0 == me.0 && k.1 > me.1)
        })
        .count();
    Some(1 + ahead as u8)
}

/// Your seasons at a team, finished ones and the one being played: the
/// line for this season counts the games so far.
pub fn seasons_at(w: &World, team: TeamKey) -> Vec<Yr> {
    let mut v: Vec<Yr> = w.legacy.seasons.iter().filter(|s| s.team == team).cloned().collect();
    let year = w.year;
    if !v.iter().any(|s| s.year == year) {
        let games: Vec<&Played> = w.legacy.games.iter().filter(|g| g.season == year && g.team == team).collect();
        if !games.is_empty() {
            v.push(Yr {
                year,
                team,
                wins: games.iter().filter(|g| g.us > g.them).count() as u16,
                losses: games.iter().filter(|g| g.us < g.them).count() as u16,
                post: games.iter().filter(|g| g.post).count() as u8,
                post_wins: games.iter().filter(|g| g.post && g.won()).count() as u8,
                playoff: games.iter().filter(|g| g.playoff).count() as u8,
                title: games.iter().any(|g| g.conf && g.won()),
                champion: games.iter().any(|g| g.title && g.won()),
                final_game: games.iter().any(|g| g.title),
                rank: None,
                class: String::new(),
                place: 0,
                conf_game: games.iter().any(|g| g.conf),
            });
        }
    }
    v
}

/// After any game the storylines saw: yours is kept.
pub(crate) fn after_game(w: &mut World, level: Level, teams: [TeamKey; 2], score: [i32; 2], f: [bool; 5]) {
    let Some(team) = crate::world::stories::seat_team(w) else { return };
    if w.career.is_none() || !teams.contains(&team) {
        return;
    }
    let side = if teams[0] == team { 0 } else { 1 };
    let opp = teams[1 - side];
    let coach = crate::world::storylines::detect::head(w, opp).map(|c| c as u32);
    let _ = level;
    let season = w.year;
    w.legacy.games.push(Played {
        season,
        date: w.today,
        team,
        opp,
        coach,
        us: score[side].clamp(0, 255) as u8,
        them: score[1 - side].clamp(0, 255) as u8,
        post: f[0],
        title: f[1],
        conf: f[2],
        rivalry: f[3],
        playoff: f[4],
    });
    w.legacy.dirty = true;
}

/// Your games against a team or a coach: (wins, losses, the last results
/// newest last).
pub fn record_vs(w: &World, opp: Option<TeamKey>, coach: Option<u32>) -> (u32, u32, Vec<bool>) {
    let mut r = (0, 0, Vec::new());
    for g in &w.legacy.games {
        if opp.is_some_and(|o| o != g.opp) || coach.is_some_and(|c| g.coach != Some(c)) {
            continue;
        }
        if g.won() {
            r.0 += 1;
        } else {
            r.1 += 1;
        }
        r.2.push(g.won());
    }
    r
}

/// Answers to the chases' letters, acted on: each system reads its own.
pub fn desk_answers(w: &mut World) {
    if w.legacy.pending.is_empty() {
        return;
    }
    let picks: Vec<(u64, u8, u32)> = {
        let Some(c) = w.career.as_ref() else { return };
        w.legacy
            .pending
            .iter()
            .filter_map(|&(id, act)| {
                let m = c.desk.mails.iter().find(|m| m.id == id)?;
                let a = m.answers.get(m.answered? as usize)?;
                (a.act == act).then_some((id, act, a.target))
            })
            .collect()
    };
    for (id, act, target) in picks {
        w.legacy.pending.retain(|x| x.0 != id);
        match act {
            ACT_OWNER => nfl::answer(w, target),
            ACT_STRINGS => college::answer_strings(w, target),
            ACT_BOOKING => college::answer_booking(w, target),
            ACT_BOND => hs::answer_bond(w, target),
            ACT_GUY => guys::answer(w, target),
            ACT_CLINIC => scheme::answer_clinic(w, target),
            _ => {}
        }
    }
    // A letter that cleared unanswered is dropped.
    let alive: Vec<u64> = w.career.as_ref().map_or(Vec::new(), |c| c.desk.mails.iter().map(|m| m.id).collect());
    w.legacy.pending.retain(|x| alive.contains(&x.0));
}

/// A letter from a chase with answers; kept until you pick one.
pub(crate) fn letter(
    w: &mut World,
    tag: &str,
    head: crate::world::desk::Head,
    from: (String, String),
    subject: String,
    body: Vec<String>,
    answers: Vec<crate::world::desk::Answer>,
) -> Option<u64> {
    let mut m = crate::world::storylines::credit::mail(w, tag, head, from.0.clone(), from.1, subject);
    m.body = body;
    m.sign = from.0;
    m.loud = 2;
    let acts: Vec<u8> = answers.iter().map(|a| a.act).collect();
    m.answer_by = (!answers.is_empty()).then(|| w.today.add(14));
    m.answers = answers;
    let id = m.id;
    let fresh = w.career.as_ref().is_some_and(|c| !c.desk.has(id));
    crate::world::storylines::credit::send(w, m);
    if fresh && let Some(&a) = acts.first() {
        w.legacy.pending.push((id, a));
    }
    fresh.then_some(id)
}

/// A memo from your staff (no answers).
pub(crate) fn memo(w: &mut World, tag: &str, subject: String, body: Vec<String>) {
    use crate::world::desk::Head;
    let Some(team) = crate::world::stories::seat_team(w) else { return };
    let (from, role) = crate::world::storylines::credit::staff_voice(w, team);
    let mut m = crate::world::storylines::credit::mail(w, tag, Head::Memo, from.clone(), role, subject);
    m.body = body;
    m.sign = from;
    crate::world::storylines::credit::send(w, m);
}

/// "a", "a and b", "a, b and c".
pub fn and_list(v: &[String]) -> String {
    match v {
        [] => String::new(),
        [a] => a.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// A team's name the way the boards write it.
pub fn tname(w: &World, t: TeamKey) -> String {
    crate::world::storylines::detect::tname(w, t)
}

/// Seasons as words: "1 season", "3 seasons".
pub fn seasons_words(n: i32) -> String {
    if n == 1 { "1 season".into() } else { format!("{n} seasons") }
}

/// An NFL division's name in a year ("NFC North"; before 2002 "NFC Central").
pub fn div_name(d: u8, year: i32) -> &'static str {
    const NOW: [&str; 8] =
        ["AFC East", "AFC North", "AFC South", "AFC West", "NFC East", "NFC North", "NFC South", "NFC West"];
    const OLD: [&str; 8] =
        ["AFC East", "AFC Central", "AFC South", "AFC West", "NFC East", "NFC Central", "NFC South", "NFC West"];
    let list = if year >= 2002 { NOW } else { OLD };
    list.get(d as usize).copied().unwrap_or("the division")
}

/// The team an employer is.
pub fn team_of_employer(e: Employer) -> Option<TeamKey> {
    Some(match e {
        Employer::College(p) => TeamKey::College(p as u32),
        Employer::Nfl(t) => TeamKey::Nfl(t as u16),
        Employer::HighSchool(h) => TeamKey::Hs(h as u32),
        Employer::Other(f) => TeamKey::Other(f as u16),
    })
}

/// This year's season review at your level already ran.
pub fn review_done(w: &World) -> bool {
    you(w).is_some_and(|(_, t)| w.legacy.reviewed.contains(&(level_code(t.level()), w.year)))
}

/// Your boss's trust moves (0-1).
pub(crate) fn boss_by(w: &mut World, by: f32) {
    let Some(team) = crate::world::folk::Team::seat(w) else { return };
    let b = crate::world::folk::boss::boss(w, team);
    w.folk.mine.nudge(b.key(), by);
}

/// Your boss's trust now, 0-1.
pub fn boss_trust(w: &World) -> f32 {
    crate::world::folk::Team::seat(w).map_or(0.5, |t| crate::world::folk::boss::trust(w, t))
}

/// The Reach ambition's ladder, at each level.
pub(crate) fn reach_steps(w: &World, a: &ambitions::Ambition) -> [bool; 5] {
    let set = a.set.unwrap_or(w.today);
    match a.team {
        TeamKey::Hs(_) => hs::reach_steps(w, set),
        TeamKey::College(_) => college::reach_steps(w, a),
        TeamKey::Nfl(t) => nfl::reach_steps(w, t as usize, set),
        TeamKey::Other(_) => [false; 5],
    }
}

/// Opens a chase's storyline with you as its lead (`word` fills the
/// lines' {word}).
pub(crate) fn story(w: &mut World, kind: K, sub: u64, word: String, fact: String) -> Option<u32> {
    use crate::world::storylines::{Spot, Who, subj};
    let (me, team) = you(w)?;
    let spot = Spot::new(kind, subj(me as u64, sub))
        .team(team)
        .lead(Who::Coach(me as u32))
        .word(word)
        .fact(fact, 40.0)
        .public();
    crate::world::storylines::detect::tell(w, spot)
}

/// Ends a chase's storyline, if it's open.
pub(crate) fn story_end(w: &mut World, kind: K, sub: u64, ending: &str) {
    let Some((me, _)) = you(w) else { return };
    let subject = crate::world::storylines::subj(me as u64, sub);
    if let Some(id) = crate::world::storylines::open_id(w, kind, subject) {
        crate::world::storylines::close(w, id, ending, None);
    }
}
