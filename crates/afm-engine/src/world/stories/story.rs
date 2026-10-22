//! A14 item 2 and C6: your career's story (a timeline written like a
//! profile: hired, the first win, milestones, upsets, rivalry trophies,
//! titles, players drafted, honors, nicknames, the losing years, every move)
//! and your office, which fills with what the years leave behind (game
//! balls, trophies, rings, photos, magazine covers, gifts) and moves with
//! you from job to job.

use crate::calendar::{Date, Level, Loudness};
use crate::game::GameStats;
use crate::world::World;

use super::lines::Fill;
use super::{GameCtx, TeamKey, news, post, team_fill};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EntryKind {
    Hired,
    FirstWin,
    Milestone,
    Title,
    Upset,
    Rivalry,
    Left,
    Draftee,
    Honor,
    Nickname,
    LosingSeason,
    Season,
}

/// One line on your timeline.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    pub date: Date,
    pub kind: EntryKind,
    pub text: String,
    pub team: Option<TeamKey>,
    /// A player the line is about.
    #[serde(default)]
    pub player: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MementoKind {
    GameBall,
    Trophy,
    Ring,
    Photo,
    Cover,
    Gift,
    Plaque,
}

impl MementoKind {
    pub fn word(self) -> &'static str {
        match self {
            MementoKind::GameBall => "Game ball",
            MementoKind::Trophy => "Trophy",
            MementoKind::Ring => "Ring",
            MementoKind::Photo => "Photo",
            MementoKind::Cover => "Magazine cover",
            MementoKind::Gift => "Gift",
            MementoKind::Plaque => "Plaque",
        }
    }
}

/// Something on a shelf in your office.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Memento {
    pub date: Date,
    pub kind: MementoKind,
    pub text: String,
    /// The team you were with when it came.
    pub team: Option<TeamKey>,
}

/// Your season with one team: (year, team, wins, losses).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct SeasonLine {
    pub year: i16,
    pub team: TeamKey,
    pub wins: u8,
    pub losses: u8,
    pub ties: u8,
    /// The season's title, as an index into `titles`.
    pub titled: bool,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CareerStory {
    pub entries: Vec<Entry>,
    pub office: Vec<Memento>,
    pub seasons: Vec<SeasonLine>,
    /// Titles you won: (season, team, what).
    pub titles: Vec<(i32, TeamKey, String)>,
    /// Every player who played in one of your games for your team.
    pub players: Vec<u32>,
    /// Your career wins, all levels.
    pub wins: u32,
    /// The team the story last saw you with.
    pub(crate) last_team: Option<TeamKey>,
    /// Big games (ranked, rivalry, postseason) by every team this season
    /// and last: (season, team, won, lost).
    pub(crate) big: Vec<(i16, TeamKey, u8, u8)>,
    /// Seasons whose end the story already wrote.
    pub(crate) closed: Vec<(i16, TeamKey)>,
}

impl CareerStory {
    pub fn big_wins(&self, t: TeamKey, year: i32) -> u32 {
        self.big.iter().filter(|x| x.0 as i32 == year && x.1 == t).map(|x| x.2 as u32).sum()
    }

    pub fn big_losses(&self, t: TeamKey, year: i32) -> u32 {
        self.big.iter().filter(|x| x.0 as i32 == year && x.1 == t).map(|x| x.3 as u32).sum()
    }
}

pub(crate) fn ordinal(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// Adds a line to your timeline (only while you have a career).
pub(crate) fn log(w: &mut World, kind: EntryKind, text: String, team: Option<TeamKey>) {
    log_about(w, kind, text, team, None);
}

pub(crate) fn log_about(w: &mut World, kind: EntryKind, text: String, team: Option<TeamKey>, player: Option<u32>) {
    if w.career.is_none() {
        return;
    }
    let date = w.today;
    let st = &mut w.stories.story;
    if st.entries.iter().any(|e| e.kind == kind && e.text == text) {
        return;
    }
    st.entries.push(Entry { date, kind, text, team, player });
}

/// Puts something on a shelf in your office (only while you have a career).
pub(crate) fn memento(w: &mut World, kind: MementoKind, text: String) {
    if w.career.is_none() {
        return;
    }
    let date = w.today;
    let team = super::seat_team(w);
    let st = &mut w.stories.story;
    if st.office.iter().any(|m| m.kind == kind && m.text == text) {
        return;
    }
    st.office.push(Memento { date, kind, text, team });
}

/// The season a game belongs to (January bowls and playoffs count for
/// the fall before).
fn season_of(d: Date) -> i32 {
    if d.month() <= 2 { d.year() - 1 } else { d.year() }
}

/// After every game with its plays: the big-game tally for every team, and
/// your own game's marks on the story.
pub(crate) fn after_game(w: &mut World, ctx: &GameCtx, s: &GameStats) {
    let season = season_of(w.today) as i16;
    let won = [s.score[0] > s.score[1], s.score[1] > s.score[0]];
    if ctx.big || ctx.pre < 0.3 || ctx.pre > 0.7 {
        // A big game: a ranked or rivalry game, the postseason, or a clear
        // favorite against a clear underdog (for the favorite's "big
        // wins" only when it's a big game; the underdog's upsets count).
        let st = &mut w.stories.story;
        st.big.retain(|x| x.0 >= season - 1);
        for side in 0..2 {
            let fav = if side == 0 { ctx.pre } else { 1.0 - ctx.pre };
            if !ctx.big && fav > 0.5 {
                continue;
            }
            let t = ctx.teams[side];
            match st.big.iter_mut().find(|x| x.0 == season && x.1 == t) {
                Some(x) => {
                    x.2 = x.2.saturating_add(won[side] as u8);
                    x.3 = x.3.saturating_add((!won[side] && s.score[0] != s.score[1]) as u8);
                }
                None => st.big.push((season, t, won[side] as u8, (!won[side] && s.score[0] != s.score[1]) as u8)),
            }
        }
    }
    let Some(me) = super::seat_team(w) else { return };
    let Some(side) = ctx.teams.iter().position(|&t| t == me) else { return };
    if w.career.is_none() {
        return;
    }
    // The players in your games.
    for &id in s.players[side].keys() {
        if !w.stories.story.players.contains(&id) {
            w.stories.story.players.push(id);
        }
    }
    let opp = ctx.teams[1 - side];
    let tie = s.score[0] == s.score[1];
    {
        let st = &mut w.stories.story;
        match st.seasons.iter_mut().find(|x| x.year == season && x.team == me) {
            Some(x) => {
                x.wins += won[side] as u8;
                x.losses += (!won[side] && !tie) as u8;
                x.ties += tie as u8;
            }
            None => st.seasons.push(SeasonLine {
                year: season,
                team: me,
                wins: won[side] as u8,
                losses: (!won[side] && !tie) as u8,
                ties: tie as u8,
                titled: false,
            }),
        }
    }
    if !won[side] {
        return;
    }
    let seed = w.options.seed;
    let year = w.today.year();
    let us = s.score[side];
    let them = s.score[1 - side];
    let score = format!("{us}-{them}");
    let opp_name = super::team_name(w, opp);
    let ball = format!("{} {us}, {opp_name} {them}, {}", super::team_name(w, me), w.today.pretty());
    w.stories.story.wins += 1;
    let n = w.stories.story.wins;
    let f = team_fill(w, me, Fill::new()).set("opp", opp_name.clone()).set("score", score.clone());
    let mut ball_worthy = false;
    if n == 1 {
        let text = w
            .stories
            .line(seed, "story.first_win", &f, year)
            .unwrap_or_else(|| format!("Your first win: {score} over {opp_name}."));
        log(w, EntryKind::FirstWin, text, Some(me));
        ball_worthy = true;
    } else if n == 25 || n == 50 || n % 50 == 0 {
        let f = f.clone().set("n", n.to_string());
        let text = w.stories.line(seed, "story.milestone", &f, year).unwrap_or_else(|| format!("Career win No. {n}."));
        log(w, EntryKind::Milestone, text, Some(me));
        ball_worthy = true;
    }
    let my_chance = if side == 0 { ctx.pre } else { 1.0 - ctx.pre };
    if my_chance < 0.3 {
        let text = w
            .stories
            .line(seed, "story.upset", &f, year)
            .unwrap_or_else(|| format!("The upset: {score} over {opp_name}."));
        log(w, EntryKind::Upset, text, Some(me));
        ball_worthy = true;
    }
    if ctx.rivalry
        && let Some(trophy) = &ctx.trophy
    {
        let f = f.clone().set("trophy", trophy.clone());
        let text = w.stories.line(seed, "story.rivalry", &f, year).unwrap_or_else(|| format!("You won the {trophy}."));
        log(w, EntryKind::Rivalry, text, Some(me));
        ball_worthy = true;
    }
    if let Some(what) = &ctx.title {
        title(w, me, season as i32, what.clone());
        ball_worthy = true;
    }
    if ball_worthy || ctx.big {
        memento(w, MementoKind::GameBall, ball);
    }
}

/// A title for your team: the timeline, the trophy on the shelf, and the
/// list the rings and parades read.
pub(crate) fn title(w: &mut World, me: TeamKey, season: i32, what: String) {
    if w.stories.story.titles.iter().any(|x| x.0 == season && x.1 == me && x.2 == what) {
        return;
    }
    w.stories.story.titles.push((season, me, what.clone()));
    if let Some(x) = w.stories.story.seasons.iter_mut().find(|x| x.year as i32 == season && x.team == me) {
        x.titled = true;
    }
    let seed = w.options.seed;
    let f = team_fill(w, me, Fill::new()).set("year", season.to_string()).set("what", what.clone());
    let text = w.stories.line(seed, "story.title", &f, w.today.year()).unwrap_or_else(|| format!("{what}, {season}."));
    log(w, EntryKind::Title, text, Some(me));
    memento(w, MementoKind::Trophy, format!("{what} trophy, {season}"));
}

/// Daily: a new job (or your first), the titles the world books outside
/// the game hooks, your players drafted, a season's end.
pub(crate) fn day(w: &mut World, d: Date) {
    if w.career.is_none() {
        return;
    }
    let now = super::seat_team(w);
    let before = w.stories.story.last_team;
    let seed = w.options.seed;
    let year = d.year();
    if now != before {
        if let (Some(old), Some(new)) = (before, now) {
            let f = team_fill(w, old, Fill::new()).set("new", super::team_name(w, new));
            let text = w
                .stories
                .line(seed, "story.left", &f, year)
                .unwrap_or_else(|| format!("You left {} for {}.", super::team_name(w, old), super::team_name(w, new)));
            log(w, EntryKind::Left, text, Some(old));
            let boxes = w.stories.story.office.len();
            if boxes > 0 {
                let text = format!(
                    "Movers packed {} from your old office; they're on the shelves at {} now.",
                    crate::world::news::count(boxes as u32, "keepsake"),
                    super::team_name(w, new)
                );
                let e = news(w, new.level(), Loudness::Small, Some(new), text);
                post(w, e);
            }
        }
        if let Some(new) = now {
            let f = team_fill(w, new, Fill::new()).set("year", year.to_string());
            let text = w
                .stories
                .line(seed, "story.hired", &f, year)
                .unwrap_or_else(|| format!("Hired by {} in {year}.", super::team_name(w, new)));
            log(w, EntryKind::Hired, text, Some(new));
            let photo = format!("Your first day at {}, {}", super::team_name(w, new), d.pretty());
            memento(w, MementoKind::Photo, photo);
        }
        w.stories.story.last_team = now;
    }
    let Some(me) = now else { return };
    // The NFL's champion and a high school's state title, booked outside the
    // game hooks.
    match me {
        TeamKey::Nfl(t) => {
            if let Some(&(y, c)) = w.nfl_champions.last()
                && c == t as usize
                && y >= year - 1
            {
                title(w, me, y, "Super Bowl".to_string());
            }
        }
        TeamKey::Hs(h) => {
            if let Some(s) = w.high_schools.get(h as usize).and_then(|x| x.seasons.last())
                && s.state_title
                && s.year >= year - 1
            {
                let y = s.year;
                title(w, me, y, "State championship".to_string());
            }
        }
        TeamKey::College(_) => {}
    }
    // Your players drafted, the day after the draft.
    if d == w.dates.draft[2].date.add(1) {
        draftees(w, me);
    }
    // A season's end, read on February 15 for the fall before.
    if d.month() == 2 && d.day() == 15 {
        close_season(w, (year - 1) as i16);
    }
}

fn draftees(w: &mut World, me: TeamKey) {
    let year = w.year;
    let seed = w.options.seed;
    let mine: std::collections::HashSet<u32> = w.stories.story.players.iter().copied().collect();
    let mut picks: Vec<(usize, u32, u32, usize)> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| mine.contains(&p.player.id))
        .filter_map(|(i, p)| {
            let (y, round, pick, team) = p.pro.as_ref()?.draft?;
            (y == year).then_some((i, round, pick, team))
        })
        .collect();
    picks.sort_by_key(|x| x.2);
    for (i, round, pick, team) in picks {
        let name = w.people[i].name.clone();
        let id = w.people[i].player.id;
        let club = TeamKey::Nfl(team as u16);
        let pick_text = format!("round {round}, the {} pick overall", ordinal(pick));
        let f = team_fill(w, club, Fill::new()).set("name", name.clone()).set("pick", pick_text.clone());
        let text = w
            .stories
            .line(seed, "story.draftee", &f, year)
            .unwrap_or_else(|| format!("{name} was drafted by {} in {pick_text}.", super::team_name(w, club)));
        log_about(w, EntryKind::Draftee, text, Some(me), Some(id));
        if round == 1 {
            memento(w, MementoKind::Photo, format!("{name} on draft night, {year}"));
        }
    }
}

fn close_season(w: &mut World, season: i16) {
    let lines: Vec<SeasonLine> = w.stories.story.seasons.iter().filter(|x| x.year == season).copied().collect();
    let seed = w.options.seed;
    for x in lines {
        if w.stories.story.closed.contains(&(x.year, x.team)) {
            continue;
        }
        w.stories.story.closed.push((x.year, x.team));
        let record =
            if x.ties > 0 { format!("{}-{}-{}", x.wins, x.losses, x.ties) } else { format!("{}-{}", x.wins, x.losses) };
        if x.losses > x.wins {
            let f = team_fill(w, x.team, Fill::new()).set("record", record.clone());
            let text = w
                .stories
                .line(seed, "story.losing_season", &f, season as i32)
                .unwrap_or_else(|| format!("A {record} season at {}.", super::team_name(w, x.team)));
            log(w, EntryKind::LosingSeason, text, Some(x.team));
        } else {
            let text = format!("{season}: {record} at {}.", super::team_name(w, x.team));
            log(w, EntryKind::Season, text, Some(x.team));
        }
    }
}

/// A one-line news post about you on the story's own terms (used by the
/// honors when a player you coached is honored).
pub(crate) fn honor(w: &mut World, id: u32, name: &str, what: &str) {
    if !w.stories.story.players.contains(&id) {
        return;
    }
    let seed = w.options.seed;
    let year = w.today.year();
    let f = Fill::new().set("name", name.to_string()).set("what", what.to_string());
    let text = w.stories.line(seed, "story.honor", &f, year).unwrap_or_else(|| format!("{name}: {what}."));
    let team = super::seat_team(w);
    log_about(w, EntryKind::Honor, text, team, Some(id));
}

/// Level of a season line, for the screens.
pub fn level_of(t: TeamKey) -> Level {
    t.level()
}
