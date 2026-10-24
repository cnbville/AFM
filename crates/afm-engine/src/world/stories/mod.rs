//! Lane 8 of the final build: the stories around the football (creative
//! additions C1-C17, NFL items P5-P7, game view A14). Rivalry trophies,
//! field storming and its fines, the travelling pregame show, uniforms,
//! retired numbers and rings of honor, your office, preseason magazines,
//! documentary crews, the video game's ratings, fantasy football, jersey
//! sales, parades and rings, farewells and one-day contracts, signature
//! celebrations, your decisions graded the next morning, the NFL's schedule
//! release and its games abroad, earned nicknames and your career's story.
//!
//! How it is built: one store on the world (`World::stories`) with its own
//! random stream, so a world's football results per seed only move where a
//! story feeds back on purpose (a recruit's visit, a fine, a player's fame
//! or mood). Two hooks: `day` once a day after the day's games, and the
//! game hooks (`college_game`, `nfl_game`, `hs_game`) after each game. Text
//! comes from the line banks (`lines.rs`, `data/lines/stories/`). Stores are
//! compact: ids, not people; recent seasons, not every game.

pub mod careers;
pub mod lines;
pub mod media;
pub mod nfl;
pub mod nick;
pub mod numbers;
pub mod story;
pub mod traditions;

use crate::calendar::{Date, Event, Kind, Level, Loudness};
use crate::rng::Rng;

use super::World;
use lines::Fill;

/// A team at any level, kept small.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum TeamKey {
    Hs(u32),
    College(u32),
    Nfl(u16),
    /// A club in another pro league, by its franchise (`world::pro::other`).
    Other(u16),
}

impl TeamKey {
    pub fn level(self) -> Level {
        match self {
            TeamKey::Hs(_) => Level::HighSchool,
            TeamKey::College(_) => Level::College,
            TeamKey::Nfl(_) => Level::Nfl,
            TeamKey::Other(_) => Level::Other,
        }
    }
}

/// Everything the stories keep.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Stories {
    /// Own random stream (started from the world's seed on first use).
    pub(crate) rng: Option<Rng>,
    /// Lines used lately, so text rests before it comes back.
    pub(crate) recent: lines::Recent,
    /// Every nickname the world has coined (players and coaches), units' and
    /// plays' names.
    pub nicknames: Vec<nick::Nickname>,
    pub units: Vec<nick::UnitName>,
    pub plays: Vec<nick::NamedPlay>,
    /// Running per-player marks toward a nickname this season (cleared each
    /// year): the moments that fit one theme, and the mocking ones.
    pub(crate) marks: Vec<nick::Mark>,
    /// The traditions: high school rivalries and their trophies, honors
    /// (retired numbers, rings of honor, statues), the pregame show's
    /// season, field stormings and fines, uniforms.
    pub traditions: traditions::Traditions,
    /// Magazines, documentaries, the video game's ratings, fantasy and
    /// jersey sales.
    pub media: media::Media,
    /// The NFL's side: your club's schedule story, games abroad, your
    /// decisions graded.
    pub nfl: nfl::NflStories,
    /// Your career: its timeline and your office.
    pub story: story::CareerStory,
    /// Farewells, one-day contracts and celebrations.
    pub careers: careers::Careers,
    /// Dated posts waiting for their day (a ceremony at next season's home
    /// game, the morning-after grades).
    pub(crate) pending: Vec<(Date, Event)>,
    /// The season the yearly steps last ran for, by step.
    pub(crate) done: Vec<(u8, i32)>,
}

impl Stories {
    pub(crate) fn rng(&mut self, seed: u32) -> &mut Rng {
        self.rng.get_or_insert_with(|| Rng::new(seed ^ 0x5709_1E55))
    }

    /// A line from the banks, avoiding recent repeats.
    pub(crate) fn line(&mut self, seed: u32, key: &str, f: &Fill, year: i32) -> Option<String> {
        let mut rng = self.rng.take().unwrap_or_else(|| Rng::new(seed ^ 0x5709_1E55));
        let out = self.recent.line(&mut rng, key, f, year);
        self.rng = Some(rng);
        out
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

    pub(crate) fn later(&mut self, d: Date, e: Event) {
        self.pending.push((d, e));
    }
}

/// Posts an event on today's calendar.
pub(crate) fn post(w: &mut World, e: Event) {
    w.calendar.post(e);
}

/// A news line at a level, about a team.
pub(crate) fn news(w: &World, level: Level, loud: Loudness, team: Option<TeamKey>, text: String) -> Event {
    let mut e = Event::new(w.today, level, Kind::News, loud, text);
    match team {
        Some(TeamKey::College(p)) => e = e.program(p as usize),
        Some(TeamKey::Nfl(t)) => e = e.team(t as usize),
        _ => {}
    }
    e
}

/// The name a story uses for a team: "Iowa", "the Bears", "Valley High".
pub fn team_name(w: &World, t: TeamKey) -> String {
    match t {
        TeamKey::Hs(h) => w.school_name(h as usize).to_string(),
        TeamKey::College(p) => w.program_name(p as usize).to_string(),
        TeamKey::Nfl(n) => w.nfl_team_name(n as usize),
        TeamKey::Other(f) => super::pro::other::name(f as usize, w.year).to_string(),
    }
}

/// How a broadcast names an opponent: the city for an NFL club ("Chicago
/// has to punt"), the school's own name otherwise.
pub fn short_name(w: &World, t: TeamKey) -> String {
    match t {
        TeamKey::Nfl(_) | TeamKey::Other(_) => city(w, t),
        _ => team_name(w, t),
    }
}

/// A team's full name inside a sentence: "the Chicago Bears", "Alabama".
pub fn team_in_text(w: &World, t: TeamKey) -> String {
    match t {
        TeamKey::Nfl(_) | TeamKey::Other(_) => format!("the {}", team_name(w, t)),
        _ => team_name(w, t),
    }
}

/// The club's nickname alone ("Bears") or a college's mascot.
pub fn mascot(w: &World, t: TeamKey) -> Option<String> {
    match t {
        TeamKey::Nfl(n) => {
            let name = w.nfl_team_name(n as usize);
            numbers::mascot_of(&name).or_else(|| name.rsplit(' ').next().map(str::to_string))
        }
        TeamKey::College(p) => numbers::mascot_of(w.program_name(p as usize)),
        TeamKey::Hs(h) => Some(numbers::hs_mascot(h)),
        TeamKey::Other(f) => Some(super::pro::other::nickname(f as usize, w.year).to_string()),
    }
}

/// The club's city ("Chicago"), a college's town, a school's town.
pub fn city(w: &World, t: TeamKey) -> String {
    match t {
        TeamKey::Nfl(n) => {
            let name = w.nfl_team_name(n as usize);
            match mascot(w, t) {
                Some(m) if name.ends_with(&m) => name[..name.len() - m.len()].trim().to_string(),
                _ => name,
            }
        }
        TeamKey::College(p) => w.programs.get(p as usize).map_or_else(String::new, |x| x.city.clone()),
        TeamKey::Hs(h) => w.high_schools.get(h as usize).map_or_else(String::new, |x| x.town.clone()),
        TeamKey::Other(f) => super::pro::other::city(f as usize, w.year).to_string(),
    }
}

/// The color word fans use ("Crimson").
pub fn color(w: &World, t: TeamKey) -> Option<String> {
    let name = team_name(w, t);
    numbers::color_of(&name)
}

/// The team values every template may use: {team} {mascot} {city} {color}
/// {state}.
pub(crate) fn team_fill(w: &World, t: TeamKey, f: Fill) -> Fill {
    let state = match t {
        TeamKey::College(p) => w.programs.get(p as usize).map(|x| x.state),
        TeamKey::Hs(h) => w.high_schools.get(h as usize).map(|x| x.state),
        TeamKey::Nfl(_) | TeamKey::Other(_) => None,
    };
    // An NFL club goes by its city in a sentence ("Chicago has 4 days to get
    // ready"), the way papers and broadcasts say it.
    let f = f.set("team", short_name(w, t)).set("city", city(w, t));
    let f = match mascot(w, t) {
        Some(m) => f.set("mascot", m),
        None => f,
    };
    let f = match color(w, t) {
        Some(c) => f.set("color", c),
        None => f,
    };
    match state.and_then(|s| w.states.get(s)) {
        Some(s) => f.set("state", numbers::state_name(&s.code)),
        None => f,
    }
}

/// Your team, when you have a seat.
pub fn seat_team(w: &World) -> Option<TeamKey> {
    let s = w.seat?;
    match s.level {
        Level::Nfl => s.nfl_team.map(|t| TeamKey::Nfl(t as u16)),
        Level::HighSchool => s.program.map(|h| TeamKey::Hs(h as u32)),
        Level::Other => s.other.map(|f| TeamKey::Other(f as u16)),
        _ => s.program.map(|p| TeamKey::College(p as u32)),
    }
}

/// Your coach id, when a career is running.
pub fn seat_coach(w: &World) -> Option<usize> {
    w.career.as_ref().map(|c| c.coach)
}

/// A person's index by player id (people are kept in id order).
pub(crate) fn person_index(w: &World, id: u32) -> Option<usize> {
    w.people.binary_search_by_key(&id, |p| p.player.id).ok()
}

/// Answer codes on the stories' letters (`desk::Answer::act`, past the
/// inbox's own): a pick is acted on the next morning, or as soon as a
/// stories screen opens. The answer's target is the player's id (one-day
/// contract) or the season (documentary).
pub const ACT_ONE_DAY_YES: u8 = 60;
/// The rule on the asks' calendar lines (the letter is the pin).
pub const ASK_RULE: &str = "stories: ask";
pub const ACT_ONE_DAY_NO: u8 = 61;
pub const ACT_DOC_YES: u8 = 62;
pub const ACT_DOC_NO: u8 = 63;

/// Acts on the answers you gave on your desk's letters.
pub fn desk_answers(w: &mut World) {
    let Some(c) = w.career.as_ref() else { return };
    let picks: Vec<(u8, u32)> = c
        .desk
        .mails
        .iter()
        .filter_map(|m| {
            let a = m.answers.get(m.answered? as usize)?;
            (ACT_ONE_DAY_YES..=ACT_DOC_NO).contains(&a.act).then_some((a.act, a.target))
        })
        .collect();
    for (act, target) in picks {
        if act <= ACT_ONE_DAY_NO {
            if w.stories.careers.offer.is_some_and(|o| o.0 == target) {
                let _ = careers::answer_one_day(w, act == ACT_ONE_DAY_YES);
            }
        } else if w.stories.media.doc_answer.is_none()
            && w.stories.media.doc_offer.as_ref().is_some_and(|o| o.0 as u32 == target)
        {
            let _ = media::answer_doc(w, act == ACT_DOC_YES);
        }
    }
}

/// Once a day, after the day's games (called from the clock).
pub(crate) fn day(w: &mut World, d: Date) {
    // Your answers first, so a deadline today sees them.
    desk_answers(w);
    // Dated posts whose day has come.
    if w.stories.pending.iter().any(|x| x.0 <= d) {
        let mut due = Vec::new();
        w.stories.pending.retain(|x| {
            if x.0 <= d {
                due.push(x.1.clone());
                false
            } else {
                true
            }
        });
        for mut e in due {
            e.when.date = d;
            post(w, e);
        }
    }
    if !w.recording {
        return;
    }
    traditions::day(w, d);
    media::day(w, d);
    nfl::day(w, d);
    careers::day(w, d);
    nick::day(w, d);
    story::day(w, d);
}

/// After a college game is booked (`college.rs`, `play_day`): `k` is the
/// game's index in the season, `pre` the home side's chance to win as the
/// world saw it before kickoff.
pub(crate) fn college_game(w: &mut World, k: usize, stats: Option<&crate::game::GameStats>, pre: f64) {
    if !w.recording {
        return;
    }
    traditions::after_college_game(w, k, pre);
    if let Some(s) = stats {
        let Some(g) = w.college.as_ref().map(|c| c.games[k].clone()) else { return };
        use super::schedule::GameKind;
        let year = w.year;
        let trophy =
            g.rivalry.and_then(|r| crate::calendar::data::RIVALRIES.get(r)).and_then(|r| r.trophy).map(str::to_string);
        // The title a win here brings.
        let title = match g.kind {
            GameKind::Title => Some(match w.programs[g.home].division {
                super::data::Division::FBS => "National championship".to_string(),
                d => format!("{} national championship", d.name()),
            }),
            GameKind::ConferenceTitle => {
                let won = g.played.as_ref().map(|p| if p.home_won { g.home } else { g.away });
                won.map(|p| format!("{} championship", w.programs[p].conference))
            }
            GameKind::Bowl(_) => g.name.map(|n| n.to_string()),
            _ => None,
        };
        let _ = year;
        let ctx = GameCtx {
            level: Level::College,
            teams: [TeamKey::College(g.home as u32), TeamKey::College(g.away as u32)],
            big: g.loud >= Loudness::Big || g.rivalry.is_some() || g.kind != GameKind::Regular,
            rivalry: g.rivalry.is_some(),
            postseason: g.kind != GameKind::Regular,
            pre,
            name: g.name.map(str::to_string),
            trophy,
            title,
        };
        nick::after_game(w, &ctx, s);
        nfl::grade(w, &ctx, s);
        careers::after_game(w, &ctx, s);
        traditions::seat_uniform(w, &ctx, s.score[0] > s.score[1]);
        story::after_game(w, &ctx, s);
    }
}

/// After an NFL game is booked (`pro/games.rs`, `record`).
pub(crate) fn nfl_game(w: &mut World, teams: [usize; 2], stats: &crate::game::GameStats, postseason: bool, pre: f64) {
    if !w.recording {
        return;
    }
    let ctx = GameCtx {
        level: Level::Nfl,
        teams: teams.map(|t| TeamKey::Nfl(t as u16)),
        big: postseason,
        rivalry: false,
        postseason,
        pre,
        name: None,
        trophy: None,
        title: None,
    };
    media::after_nfl_game(w, &ctx, stats);
    nfl::after_game(w, &ctx, stats);
    nick::after_game(w, &ctx, stats);
    nfl::grade(w, &ctx, stats);
    careers::after_nfl_game(w, &ctx, stats);
    traditions::seat_uniform(w, &ctx, stats.score[0] > stats.score[1]);
    story::after_game(w, &ctx, stats);
}

/// After one of your high school's games (only your school's games keep
/// their plays).
pub(crate) fn hs_game(w: &mut World, teams: [usize; 2], stats: &crate::game::GameStats) {
    if !w.recording {
        return;
    }
    let ctx = GameCtx {
        level: Level::HighSchool,
        teams: teams.map(|t| TeamKey::Hs(t as u32)),
        big: false,
        rivalry: traditions::hs_rivals(w, teams[0], teams[1]),
        postseason: false,
        pre: 0.5,
        name: None,
        trophy: None,
        title: None,
    };
    let ctx = GameCtx { trophy: traditions::hs_trophy(w, teams[0], teams[1]), ..ctx };
    traditions::after_hs_game(w, &ctx, stats);
    nick::after_game(w, &ctx, stats);
    careers::after_game(w, &ctx, stats);
    traditions::seat_uniform(w, &ctx, stats.score[0] > stats.score[1]);
    story::after_game(w, &ctx, stats);
}

/// After one of your club's games in another league (the CFL, arena or a
/// spring league): its line in your career story.
pub(crate) fn other_game(w: &mut World, teams: [usize; 2], stats: &crate::game::GameStats, postseason: bool) {
    let teams = teams.map(|t| TeamKey::Other(t as u16));
    if !w.recording || !seat_team(w).is_some_and(|me| teams.contains(&me)) {
        return;
    }
    let ctx = GameCtx {
        level: Level::Other,
        teams,
        big: postseason,
        rivalry: false,
        postseason,
        pre: 0.5,
        name: None,
        trophy: None,
        title: None,
    };
    story::after_game(w, &ctx, stats);
}

/// What a game hook knows about the game.
#[derive(Clone, Debug)]
pub(crate) struct GameCtx {
    pub level: Level,
    /// Home, away.
    pub teams: [TeamKey; 2],
    pub big: bool,
    pub rivalry: bool,
    pub postseason: bool,
    /// The home side's chance before kickoff.
    pub pre: f64,
    /// A bowl or playoff game's name.
    pub name: Option<String>,
    /// The rivalry's trophy.
    pub trophy: Option<String>,
    /// The title a win in this game brings.
    pub title: Option<String>,
}

impl World {
    /// The nickname the world uses for a player, if one stuck.
    pub fn nickname_of(&self, id: u32) -> Option<&str> {
        nick::current(&self.stories, nick::Who::Player(id)).map(|n| n.name.as_str())
    }

    /// The nickname a coach goes by, if one stuck.
    pub fn coach_nickname(&self, c: usize) -> Option<&str> {
        nick::current(&self.stories, nick::Who::Coach(c as u32)).map(|n| n.name.as_str())
    }
}
