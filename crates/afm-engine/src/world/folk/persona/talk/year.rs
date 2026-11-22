//! The time of year (Cain 2026-11-18, about a player who answered a
//! captain's promise on January 1 with "I'd rather see it on the field
//! first": "he's asking for proof while we're in fucking january"). The
//! talk cards carried the date and nothing else, so the voice talked as if
//! a game were coming. Every card now says where the man's own team is in
//! its year and the next real dates (`words`), and a player's own place
//! out of season reads as what he saw last, dated, and when it gets set
//! again (`place_off`, `portal`), from the calendar the world already runs
//! on: the training frames (`training::rules::Frame`), the year's anchors
//! and the transfer windows (`clock::portal_windows_for`).
//!
//! Every size here is Claude's (proposed, not confirmed): how many dates the
//! line names (`AHEAD`), how far ahead they reach (`REACH`) and how early a
//! portal window shows on a player's own card (`PORTAL_LEAD`).

use super::super::Them;
use super::standing::{self, Road};
use crate::calendar::rules::TransferRule;
use crate::calendar::{Date, Level, YearDates};
use crate::training::rules::{Block, Frame, Tier};
use crate::training::seat::GROUP_ONES;
use crate::training::week::division_frame;
use crate::world::folk::{self as folk, claims::words::date_words, seen};
use crate::world::{Stage, World};

/// The most dates the time of year names. Proposed, not confirmed.
pub const AHEAD: usize = 4;

/// How far ahead those dates reach, in days. Proposed, not confirmed.
pub const REACH: i32 = 240;

/// How far ahead a game still counts as this season's next one, in days:
/// the longest a team in season waits for a game is a bowl's wait after
/// the conference title game, about four weeks; next fall's opener is
/// months off. Proposed, not confirmed.
pub const NEXT_GAME: i32 = 45;

/// How many days before a transfer window opens a college player's card
/// names it. Proposed, not confirmed.
pub const PORTAL_LEAD: i32 = 21;

/// The year a man lives by: his own team's, or your team's for anyone who
/// isn't a player (your staff, your boss, a booster, an agent).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    College(usize),
    /// An NFL club, or none (a free agent).
    Nfl(Option<usize>),
    High(usize),
    /// His college career is over and the draft is next.
    Draft,
}

/// Whose year a talk with him runs on.
pub fn side_of(w: &World, him: Them) -> Option<Side> {
    match him {
        Them::Player(id) | Them::Kin(id, _) => player_side(w, folk::person(w, id)?),
        _ => seat_side(w),
    }
}

fn player_side(w: &World, i: usize) -> Option<Side> {
    let p = &w.people[i];
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => w.high_schools.get(p.hs).map(|_| Side::High(p.hs)),
        Stage::Declared => Some(Side::Draft),
        Stage::College if standing::road(w, i) == Road::DraftBound => Some(Side::Draft),
        Stage::College => p.college.filter(|&c| c < w.programs.len()).map(Side::College),
        Stage::Pro => Some(Side::Nfl(p.pro.as_ref().and_then(|c| c.team))),
        _ => None,
    }
}

fn seat_side(w: &World) -> Option<Side> {
    let (level, t) = w.seat_team()?;
    match level {
        Level::College => Some(Side::College(t)),
        Level::Nfl => Some(Side::Nfl(Some(t))),
        Level::HighSchool => Some(Side::High(t)),
        _ => None,
    }
}

/// A side's training frame for the football year of `y`.
fn frame(w: &World, side: Side, y: &YearDates) -> Option<Frame> {
    match side {
        Side::College(c) => Some(division_frame(y, w.programs.get(c)?.division)),
        // A club with a new head coach starts its offseason program two
        // weeks early (CBA Article 21): a coach hired for this season has
        // the season before as his `since` (`coach::Job::since`).
        Side::Nfl(Some(t)) => {
            let since = w.nfl.get(t)?.head_coach.and_then(|c| w.coaches.get(c)?.job.as_ref().map(|j| j.since));
            Some(Frame::pro(y, since == Some(y.season - 1)))
        }
        Side::Nfl(None) | Side::Draft => Some(Frame::pro(y, false)),
        Side::High(h) => {
            let s = w.high_schools.get(h)?.state;
            let code = &w.states.get(s)?.code;
            // The state's season as it is being played this year; another
            // year's from its own dates.
            if y.season == w.dates.season {
                return Some(crate::training::week::state_frame(w, y, s).0);
            }
            let (start, _, finals) = crate::world::hs_year::state_season_dates(code, y.season);
            Some(Frame::high(y, start, finals, crate::calendar::data::hs_calendar(code).spring_ball))
        }
    }
}

/// This football year's and next year's dates and frames.
struct Years {
    y0: YearDates,
    y1: YearDates,
    f0: Option<Frame>,
    f1: Option<Frame>,
}

fn years(w: &World, side: Side) -> Years {
    let y0 = w.dates.clone();
    let y1 = YearDates::new(y0.season + 1);
    let f0 = frame(w, side, &y0);
    let f1 = frame(w, side, &y1);
    Years { y0, y1, f0, f1 }
}

/// The team's next game from today (today's included), when it still has
/// one this season: the schedule, then a bowl, a playoff round or a final,
/// within `NEXT_GAME` days (next fall's schedule is out in the spring).
pub fn next_game(w: &World, side: Side) -> Option<Date> {
    let today = w.today;
    games(w, side).into_iter().filter(|&d| d >= today && today.days_to(d) <= NEXT_GAME).min()
}

/// The team's games still to play that the world has, any distance ahead.
fn games(w: &World, side: Side) -> Vec<Date> {
    let today = w.today;
    match side {
        Side::College(c) => w
            .college_fixtures(Some(c))
            .into_iter()
            .filter(|f| f.score.is_none() && !f.cancelled && f.date >= today)
            .map(|f| f.date)
            .collect(),
        Side::Nfl(Some(t)) => {
            let mut v: Vec<Date> = w
                .nfl_fixtures(Some(t))
                .into_iter()
                .filter(|f| !f.played)
                .filter_map(|f| f.when)
                .filter(|&d| d >= today)
                .collect();
            if let Some(s) = w.nfl_season.as_ref() {
                v.extend(s.round_games.iter().filter(|g| g.2 == t || g.3 == t).map(|g| g.4.date));
                if s.booked
                    && s.champion.is_none()
                    && let (Some(&a), Some(&b)) = (s.alive[0].first(), s.alive[1].first())
                    && (a == t || b == t)
                {
                    v.push(w.dates.super_bowl);
                }
            }
            v
        }
        Side::High(h) => {
            let mut v: Vec<Date> = w.hs_fixtures(h).into_iter().map(|f| f.date).filter(|&d| d >= today).collect();
            v.extend(w.hs_playoff_next(h).map(|x| x.0));
            v
        }
        Side::Nfl(None) | Side::Draft => Vec::new(),
    }
}

/// The team is in its season: a game still to play, or camp.
fn in_season(w: &World, side: Side, ys: &Years) -> bool {
    if next_game(w, side).is_some() {
        return true;
    }
    let Some(f) = ys.f0 else { return false };
    let b = f.block(w.today);
    b == Block::Camp || (b == Block::Season && w.today < f.first_game.add(7))
}

/// Where his team's year is today, in words.
fn now_words(w: &World, side: Side, ys: &Years) -> Option<String> {
    let today = w.today;
    let s = ys.y0.season;
    if side == Side::Draft {
        return Some(format!(
            "His college career is over: these are the months before the {} NFL draft, with the all-star games, the combine and pro days",
            if today <= ys.y0.draft[0].date { ys.y0.season } else { ys.y1.season }
        ));
    }
    if let Side::Nfl(None) = side {
        return Some("He's a free agent: his year runs on the NFL's calendar".into());
    }
    let f = ys.f0?;
    let game = next_game(w, side);
    let b = f.block(today);
    Some(match (f.tier, b) {
        (_, Block::Season | Block::Postseason) if game.is_some() => {
            if today > f.season_end {
                format!("The {s} season's postseason: the team still has a game to play, on {}", date_words(game?))
            } else {
                format!("The {s} season is under way")
            }
        }
        (_, Block::Camp) => match f.tier {
            Tier::Pro => format!("Training camp and the preseason for the {s} season: where the roster and the starters are decided"),
            Tier::College => format!("Fall camp for the {s} season: practice every day, where the jobs are won"),
            Tier::High => format!("Preseason practice for the {s} season: where the jobs are won"),
        },
        (Tier::College, Block::Postseason | Block::Season) => format!(
            "The {s} season is over for the team: it's the offseason, with no practice until winter workouts start on {}",
            date_words(f.winter)
        ),
        (Tier::College, Block::Winter) => {
            "Winter workouts: lifting, running and conditioning with the strength staff, and some film, with no practice".into()
        }
        (Tier::College, Block::Spring) => {
            "Spring practice: the first practices since last season, where the coaches get a first look at who works with the starters".into()
        }
        (Tier::College, Block::Quiet) => {
            "The weeks after spring practice: exams, then time on their own until summer workouts".into()
        }
        (Tier::College, Block::Summer) => {
            "Summer workouts: lifting and conditioning with the strength staff, a few hours a week of walk-throughs and film, and the players' own throwing sessions".into()
        }
        (Tier::Pro, Block::Season) => format!("The {s} season is over for the team"),
        (Tier::Pro, Block::Winter) if today < ys.y0.super_bowl && today >= f.season_end => {
            format!("The {s} season is over for the team: the playoffs go on without it. It's the offseason, with no team activities until the offseason program")
        }
        (Tier::Pro, Block::Winter) => {
            "The offseason: no team activities until the offseason program starts".into()
        }
        (Tier::Pro, Block::Phase1) => {
            "The offseason program's first weeks: strength and conditioning and meetings, no football on the field".into()
        }
        (Tier::Pro, Block::Phase2) => {
            "The offseason program's second phase: individual drills and walk-throughs, no offense against defense".into()
        }
        (Tier::Pro, Block::Otas) => "OTAs and minicamp: practice without pads".into(),
        (Tier::Pro, Block::Break) => "The summer break before training camp".into(),
        (Tier::High, Block::Season) => format!("The {s} season is over for the team"),
        (Tier::High, Block::Winter) => {
            "The offseason: winter lifting, and other sports for the players who play them".into()
        }
        (Tier::High, Block::Spring) => "Spring ball".into(),
        (Tier::High, Block::Summer) => "Summer workouts and 7-on-7".into(),
        _ => return None,
    })
}

/// A date to come and what it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Ahead {
    pub date: Date,
    pub what: String,
    /// A day the team works on the field (practice, camp, a game): where
    /// a man's place can be seen.
    pub field: bool,
}

/// The transfer windows a football year's offseason has, in its era: none
/// before the portal and while it was open all year.
fn windows(w: &World, season: i32) -> Vec<(Date, Date)> {
    match w.changes.rules(season).transfer {
        TransferRule::Windows | TransferRule::January => crate::world::clock::portal_windows_for(w, season),
        TransferRule::Permission | TransferRule::OpenPortal => Vec::new(),
    }
}

/// The dates to come on his team's calendar, nearest first.
pub fn ahead(w: &World, him: Them) -> Vec<Ahead> {
    let Some(side) = side_of(w, him) else { return Vec::new() };
    let ys = years(w, side);
    let mut v: Vec<Ahead> = Vec::new();
    let mut put = |date: Date, what: &str, field: bool| v.push(Ahead { date, what: what.to_string(), field });
    if let Some(g) = next_game(w, side) {
        put(g, "the team's next game", true);
    }
    for (y, f) in [(&ys.y0, ys.f0), (&ys.y1, ys.f1)] {
        match side {
            Side::College(_) => {
                if let Some(f) = f {
                    put(f.winter, "winter workouts start with the spring semester", false);
                    if let Some((a, b)) = f.spring {
                        put(a, "spring practice opens", true);
                        put(b, "the spring game", true);
                    }
                    put(f.summer, "summer workouts start", false);
                    put(f.camp, "fall camp opens", true);
                    put(f.first_game, "the season opens", true);
                }
                for (open, close) in windows(w, y.season) {
                    put(open, "the transfer portal window opens", false);
                    put(close, "the transfer portal window closes", false);
                }
                put(y.declare_deadline, "the deadline for underclassmen to declare for the NFL draft", false);
                if let Some(e) = y.early_signing {
                    put(e, "the early signing period opens", false);
                }
                put(y.signing_day, "the February signing day", false);
            }
            Side::Nfl(_) => {
                if let Some(f) = f {
                    put(f.phase1, "the offseason program starts", false);
                    put(f.otas, "OTAs start", true);
                    put(f.report, "training camp opens", true);
                    put(f.first_game, "the season opens", true);
                }
                put(y.league_year.date, "the new league year: free agency opens", false);
                put(y.draft[0].date, "the NFL draft", false);
                put(y.minicamp.0, "mandatory minicamp", true);
            }
            Side::High(_) => {
                if let Some(f) = f {
                    if let Some((a, _)) = f.spring {
                        put(a, "spring ball starts", true);
                    }
                    put(f.summer, "summer workouts and 7-on-7 start", false);
                    put(f.camp, "practice opens", true);
                    put(f.first_game, "the season opens", true);
                }
                if let Some(e) = y.early_signing {
                    put(e, "the early signing period opens", false);
                }
                put(y.signing_day, "the February signing day", false);
            }
            Side::Draft => {
                put(y.senior_bowl, "the Senior Bowl", true);
                put(y.combine_start, "the NFL combine", true);
                put(y.pro_days.0, "pro days start", true);
                put(y.draft[0].date, "the NFL draft", false);
            }
        }
    }
    let today = w.today;
    v.retain(|a| a.date > today && today.days_to(a.date) <= REACH);
    v.sort_by_key(|a| a.date);
    v.dedup_by(|a, b| a.date == b.date && a.what == b.what);
    v
}

/// The time of year for a talk card: where his team's year is and the next
/// few dates, with the next day on the field always among them. Empty when
/// the world keeps no year for him.
pub fn words(w: &World, him: Them) -> String {
    let Some(side) = side_of(w, him) else { return String::new() };
    let ys = years(w, side);
    let Some(now) = now_words(w, side, &ys) else { return String::new() };
    let all = ahead(w, him);
    let mut next: Vec<&Ahead> = all.iter().take(AHEAD).collect();
    if !next.iter().any(|a| a.field)
        && let Some(f) = all.iter().find(|a| a.field)
    {
        next.push(f);
    }
    if next.is_empty() {
        return now;
    }
    let list: Vec<String> = next.iter().map(|a| format!("{} on {}", a.what, date_words(a.date))).collect();
    format!("{now}. Coming up: {}", list.join("; "))
}

/// Who the words are about: the man himself, or a parent's son.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    You,
    Son,
}

impl Who {
    fn subj(self) -> &'static str {
        if self == Who::You { "you" } else { "he" }
    }
    fn subj_cap(self) -> &'static str {
        if self == Who::You { "You" } else { "He" }
    }
    fn have(self) -> &'static str {
        if self == Who::You { "you've" } else { "he's" }
    }
}

/// A player's place out of season (the staying talk on January 1): what he
/// saw last, dated, and when it gets set again. None while his team is in
/// its season or camp, where the season's own words hold.
pub fn place_off(w: &World, i: usize, who: Who) -> Option<Vec<String>> {
    let side = player_side(w, i)?;
    if side == Side::Draft || matches!(side, Side::Nfl(None)) {
        return None;
    }
    let ys = years(w, side);
    if in_season(w, side, &ys) {
        return None;
    }
    let f0 = ys.f0?;
    let today = w.today;
    // The season whose camp has passed is the one he played last.
    let last = if today >= f0.camp { f0.season } else { f0.season - 1 };
    // The season coming, and its frame.
    let coming = if today < f0.camp { f0 } else { ys.f1? };
    let mut out = Vec::new();
    let p = &w.people[i];
    let id = p.player.id;
    // What he saw last: this offseason's practice weeks, else last season.
    let spring_weeks: Vec<bool> = seen::weeks(w, id)
        .into_iter()
        .filter(|x| x.0 >= coming.start && x.0 < coming.camp)
        .take(seen::WINDOW)
        .map(|x| x.1 == GROUP_ONES)
        .collect();
    let practice = match coming.tier {
        Tier::Pro => "the offseason program",
        Tier::College => "spring practice",
        Tier::High => "spring ball",
    };
    if !spring_weeks.is_empty() && !p.player.position.is_specialist() {
        let ones = spring_weeks.iter().filter(|x| **x).count() * 2 >= spring_weeks.len();
        out.push(if ones {
            format!("In {practice} {} worked with the starters", who.have())
        } else {
            format!("In {practice} {} worked behind the starters", who.have())
        });
    } else if let Some(words) = last_season(w, i, who, last) {
        out.push(words);
    }
    out.push(next_place(w, &coming));
    Some(out)
}

/// What he did in season `last`, as he saw it: his last games and practice
/// weeks with you, else his role that season.
fn last_season(w: &World, i: usize, who: Who, last: i32) -> Option<String> {
    let p = &w.people[i];
    // A season counts on January 2 (`college::settle_eligibility`): until
    // then a freshman who played this fall still has none on his count.
    let counting = last == w.dates.season && w.today < Date::ymd(last + 1, 1, 2);
    if p.stage == Stage::College && p.college_seasons == 0 && !counting {
        return Some(format!(
            "{} played a college snap yet",
            if who == Who::You { "You haven't" } else { "He hasn't" }
        ));
    }
    if let Some(first) = seen::first_now(w, i) {
        return Some(if first {
            format!("{} finished the {last} season with the starters", who.subj_cap())
        } else {
            format!("{} finished the {last} season as a backup, not a starter", who.subj_cap())
        });
    }
    let did = match p.last_role? {
        crate::world::PlayingTime::Starter => "started".to_string(),
        crate::world::PlayingTime::Rotation => "played in the rotation, not as a starter".into(),
        crate::world::PlayingTime::Backup => format!("{} a backup", if who == Who::You { "were" } else { "was" }),
        crate::world::PlayingTime::Redshirt => "redshirted".into(),
        crate::world::PlayingTime::PracticeSquad => {
            format!("{} on the practice squad", if who == Who::You { "were" } else { "was" })
        }
        crate::world::PlayingTime::Unsigned => return None,
    };
    Some(format!("In the {last} season {} {did}", who.subj()))
}

/// When his place for the coming season gets set, by his level's year.
fn next_place(w: &World, f: &Frame) -> String {
    let today = w.today;
    let b = f.block(today);
    let s = f.season;
    let opening = format!("Nobody's place for the {s} season is set yet");
    match f.tier {
        Tier::College => {
            let camp = format!("fall camp, from {}, is where the jobs are won", date_words(f.camp));
            match f.spring {
                Some((a, end)) if today < a => format!(
                    "{opening}. Winter workouts are lifting and conditioning, with no practice. Spring practice, from {} to the spring game on {}, is the coaches' first look at who works with the starters; {camp}",
                    date_words(a),
                    date_words(end)
                ),
                Some((_, end)) if b == Block::Spring => format!(
                    "{opening}. Spring practice, to the spring game on {}, is the coaches' first look at who works with the starters; {camp}",
                    date_words(end)
                ),
                _ => format!("{opening}. {}", cap(&camp)),
            }
        }
        Tier::Pro => {
            if today < f.otas {
                format!(
                    "{opening}. OTAs, from {}, and minicamp are the first look, without pads; training camp, from {}, and the preseason decide the roster and the starters",
                    date_words(f.otas),
                    date_words(f.report)
                )
            } else {
                format!(
                    "{opening}. Training camp, from {}, and the preseason decide the roster and the starters",
                    date_words(f.report)
                )
            }
        }
        Tier::High => {
            let spring = match f.spring {
                Some((a, _)) if today < a => format!("Spring ball from {} is a first look. ", date_words(a)),
                _ => String::new(),
            };
            let summer = if today < f.summer {
                format!("Summer workouts and 7-on-7 start {}", date_words(f.summer))
            } else {
                "Summer workouts and 7-on-7 are under way".to_string()
            };
            // His high school's jobs, by name: said on a college coach's
            // call, "that's where the jobs are won" read as the college's
            // depth chart, and a junior asked a coach recruiting him what it
            // takes "to play more than the guys ahead of me" there (Cain
            // 2026-11-22).
            format!(
                "Nobody's place on the high school team for the {s} season is set yet. {spring}{summer}; practice opens {}, and that's where the high school jobs are won",
                date_words(f.camp)
            )
        }
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// The transfer window, on a college player's own card (`standing`): when
/// it opens and closes, that he'd have to enter in it, and when the next
/// one comes. From `PORTAL_LEAD` days before it opens until it closes;
/// never for a man already in the portal or with no college season left.
pub fn portal(w: &World, i: usize, who: Who) -> Option<String> {
    let p = &w.people[i];
    if p.stage != Stage::College || p.cal.portal.is_some() || standing::road(w, i).college_ends() {
        return None;
    }
    let today = w.today;
    let mut all: Vec<(Date, Date)> = windows(w, w.dates.season);
    all.extend(windows(w, w.dates.season + 1));
    // His own program's coaching-change window, when its head coach left.
    if let Some(c) = p.college {
        if let Some(x) = w.late_windows.iter().find(|x| x.0 == c).and_then(|x| x.1) {
            all.push(x);
        } else if w.programs.get(c).is_some_and(|g| g.hc_changed == w.year)
            && let Some(x) = w.coach_window
        {
            all.push(x);
        }
    }
    all.sort();
    all.dedup();
    let k = all.iter().position(|&(open, close)| close >= today && open.add(-PORTAL_LEAD) <= today)?;
    let (open, close) = all[k];
    let later = all.iter().skip(k + 1).find(|x| x.0 > close).map(|x| x.0);
    let after = match later {
        Some(d) => format!("the next one opens {}", date_words(d)),
        None => "the next one is a year away".into(),
    };
    let to_move = if who == Who::You {
        "To transfer, you'd have to enter it then"
    } else {
        "To transfer, he'd have to enter it then"
    };
    Some(if open > today {
        format!(
            "The transfer portal window opens {} and closes {}. {to_move}: once it closes, {} can't leave until a window opens again, and {after}",
            date_words(open),
            date_words(close),
            who.subj()
        )
    } else {
        format!(
            "The transfer portal window is open until {}. {to_move}: once it closes, {} can't leave until a window opens again, and {after}",
            date_words(close),
            who.subj()
        )
    })
}
