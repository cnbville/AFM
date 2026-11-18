//! Where he stands (Cain 2026-11-16: "is it just me or is the core issue
//! that the player/ai doesn't know what he is? like it doesn't know it's own
//! situation", about a senior with no eligibility left who asked about the
//! transfer window and wanted a role). The facts of a man's own situation
//! that he knows without being told: his class and the seasons he has left,
//! the draft (deciding by its deadline, coming back, his last season, or
//! his college career over), what he hears from the scouts, the portal, his
//! place on the team, an injury, his contract. Every talk reads it: the
//! voice's card says it (`stakes`), his wants follow it (`wanting::raw`: a
//! man whose college career is over wants the draft, not a role), and the
//! written lines keep to it (`voice::Speaker::player`'s `lastyear` tag, the
//! draft question's `choice` fact).
//!
//! Only what he'd know himself, in words: no hidden value, rating or
//! potential leaves the engine. The scouts' word is the round they say, as
//! the draft outlook shows it, and only once December's reads are out.

use super::super::Them;
use super::super::makeup;
use super::super::wants::Kind;
use crate::calendar::{Date, YearDates};
use crate::world::eligibility as el;
use crate::world::folk::{self as folk, Team, facts};
use crate::world::scouting::public;
use crate::world::{PlayingTime, Stage, World};

/// Where a player is on his road, from the world's own facts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Road {
    /// In high school or a prep year: college comes next.
    HighSchool,
    /// In college, not eligible for the next draft yet.
    College,
    /// In college and eligible for the next draft, with the choice still
    /// his: declare by the deadline or come back.
    Deciding,
    /// Eligible, but the deadline passed without his name in: he's back
    /// for another season.
    Returning,
    /// His last college season: no college football after it, so no
    /// transfer and no coming back; the draft is next.
    LastSeason,
    /// His college career is over (his last season played, or he
    /// declared): the draft is next.
    DraftBound,
    /// A pro.
    Pro,
    /// Out of football, or between places the world keeps no road for.
    Out,
}

impl Road {
    /// No college football left after this season: a line about the portal,
    /// a redshirt or next season isn't his to say.
    pub fn college_ends(self) -> bool {
        matches!(self, Road::LastSeason | Road::DraftBound)
    }

    /// The draft is a choice he still has to make.
    pub fn has_choice(self) -> bool {
        self == Road::Deciding
    }

    /// The draft is on his mind by where he is: his college career ends
    /// this year, or he could go.
    pub fn draft_near(self) -> bool {
        matches!(self, Road::Deciding | Road::LastSeason | Road::DraftBound)
    }
}

/// Whether his college team's season is over: every game it has on the
/// schedule played, once the bowls are set (or the title game is past).
pub fn season_done(w: &World, i: usize) -> bool {
    let p = &w.people[i];
    if w.today > w.dates.title_game {
        return true;
    }
    let Some(prog) = p.college else { return true };
    if w.college.is_none() {
        return true;
    }
    let left = w.college_fixtures(Some(prog)).iter().any(|f| f.score.is_none() && !f.cancelled && f.date >= w.today);
    !left && w.today >= w.dates.selection_day
}

/// College seasons he has after the one being played (or just played).
/// A season counts against him when it settles on January 2
/// (`college::settle_eligibility`), so until then the one being played is
/// counted here as used.
pub fn seasons_after(w: &World, i: usize) -> u32 {
    let p = &w.people[i];
    let dy = public::draft_year(w);
    let div = el::division_of(p, &w.programs);
    let settled = w.today >= Date::ymd(dy, 1, 2);
    let used = p.college_seasons + u32::from(!settled);
    let clock = (el::clock_last(p) - (dy - 1)).max(0) as u32;
    el::seasons_allowed(dy, div).saturating_sub(used).min(clock)
}

/// A player's road today.
pub fn road(w: &World, i: usize) -> Road {
    let p = &w.people[i];
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => Road::HighSchool,
        Stage::Declared => Road::DraftBound,
        Stage::College => {
            let dy = public::draft_year(w);
            if seasons_after(w, i) == 0 {
                if season_done(w, i) { Road::DraftBound } else { Road::LastSeason }
            } else if p.class_of + 3 <= dy {
                if w.today > YearDates::new(dy - 1).declare_deadline { Road::Returning } else { Road::Deciding }
            } else {
                Road::College
            }
        }
        Stage::Pro => Road::Pro,
        _ => Road::Out,
    }
}

/// The scouts' word on him in rounds, as a player hears it through his
/// school (the draft outlook's own words), once December's reads are out
/// and until the draft. None before, or with no read.
pub fn scouts_word(w: &World, i: usize) -> Option<&'static str> {
    let p = &w.people[i];
    let before_draft = w.today <= w.dates.draft[2].date;
    let reads_out = before_draft || w.today > w.dates.fbs_regular_end;
    if !reads_out {
        return None;
    }
    // hidden: the league's read gives a word for the round, never its
    // grade.
    let r = p.cal.draft_read?;
    Some(match r.round {
        1 => "a first-round pick",
        2..=3 => "a day-two pick, the second or third round",
        4..=7 => "a day-three pick, somewhere in rounds four to seven",
        _ => "likely undrafted: a free agent after the draft",
    })
}

/// Who the words are about: the man himself, or a parent's son.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Voice {
    You,
    Son,
}

impl Voice {
    fn are(self) -> &'static str {
        match self {
            Voice::You => "You're",
            Voice::Son => "Your son is",
        }
    }
    fn have(self) -> &'static str {
        match self {
            Voice::You => "You have",
            Voice::Son => "He has",
        }
    }
    fn your(self) -> &'static str {
        match self {
            Voice::You => "your",
            Voice::Son => "his",
        }
    }
    fn subj(self) -> &'static str {
        match self {
            Voice::You => "you",
            Voice::Son => "he",
        }
    }
}

fn seasons(n: u32) -> String {
    match n {
        1 => "one season".into(),
        n => format!("{n} seasons"),
    }
}

/// A player's own situation in words (`Voice::You`), or his parent's view of
/// it (`Voice::Son`).
fn player(w: &World, i: usize, v: Voice) -> Vec<String> {
    let p = &w.people[i];
    let mut out = Vec::new();
    let road = road(w, i);
    let dy = public::draft_year(w);
    let class = facts::class_words(w, i);
    match road {
        Road::HighSchool => {
            let school = w.high_schools.get(p.hs).map(|_| w.school_name(p.hs).to_string()).unwrap_or_default();
            out.push(if school.is_empty() {
                format!("{} a high school {class}", v.are())
            } else {
                format!("{} a {class} at {school}", v.are())
            });
            if let Some(s) = &p.cal.signed {
                out.push(format!("{} signed with {}", v.subj_cap(), w.program_name(s.program)));
            } else if let Some((c, _, walk_on)) = p.cal.commit {
                out.push(format!(
                    "{} committed to {}{}",
                    v.subj_cap(),
                    w.program_name(c),
                    if walk_on { " as a walk-on" } else { "" }
                ));
            } else if !p.cal.offers.is_empty() {
                let n = p.cal.offers.len();
                out.push(format!(
                    "{} {} college offer{} and {} committed anywhere yet",
                    v.have(),
                    n,
                    if n == 1 { "" } else { "s" },
                    if v == Voice::You { "haven't" } else { "hasn't" }
                ));
            }
        }
        Road::College | Road::Deciding | Road::Returning | Road::LastSeason | Road::DraftBound => {
            let school = p.college.map(|c| w.program_name(c).to_string()).unwrap_or_default();
            if !school.is_empty() {
                out.push(format!("{} a {class} at {school}", v.are()));
            }
            let after = seasons_after(w, i);
            match road {
                Road::DraftBound if p.stage == Stage::Declared => out.push(format!(
                    "{} declared for the {dy} NFL draft: {} college career is over, so there's no transfer and no coming back",
                    v.have(),
                    v.your()
                )),
                Road::DraftBound => out.push(format!(
                    "{} college eligibility is used up and {} played {} last college game: {} college career is over, so there's no transfer and no coming back. The {dy} NFL draft is next",
                    cap(v.your()),
                    if v == Voice::You { "you've" } else { "he's" },
                    v.your(),
                    v.your()
                )),
                Road::LastSeason => out.push(format!(
                    "This is {} last college season: {} no eligibility after it, so there's no transfer and no coming back. The {dy} NFL draft comes after it",
                    v.your(),
                    if v == Voice::You { "you have" } else { "he has" }
                )),
                Road::Deciding => {
                    let deadline = YearDates::new(dy - 1).declare_deadline;
                    out.push(format!(
                        "{} {} of college eligibility left after this one, and {} eligible for the {dy} NFL draft: {} can declare by {} or come back",
                        v.have(),
                        seasons(after),
                        if v == Voice::You { "you're" } else { "he's" },
                        v.subj(),
                        claims_date(deadline)
                    ));
                }
                Road::Returning => out.push(format!(
                    "{} coming back for another college season: the deadline to declare for the draft passed. {} {} left after this one",
                    if v == Voice::You { "You're" } else { "He's" },
                    v.have(),
                    seasons(after)
                )),
                _ => out.push(format!("{} {} of college eligibility left after this one", v.have(), seasons(after))),
            }
            if road.draft_near()
                && let Some(word) = scouts_word(w, i)
            {
                out.push(format!(
                    "What {} {} from the NFL scouts through the school: {word}",
                    v.subj(),
                    if v == Voice::You { "hear" } else { "hears" }
                ));
            }
            if p.cal.portal.is_some() && !road.college_ends() {
                out.push(format!("{} in the transfer portal", v.are()));
            }
            // The transfer window around it (`year::portal`).
            out.extend(super::year::portal(
                w,
                i,
                if v == Voice::You { super::year::Who::You } else { super::year::Who::Son },
            ));
            if p.cal.opted_out {
                out.push(format!("{} opted out of the bowl game to get ready for the draft", v.have()));
            }
        }
        Road::Pro => {
            if let Some(c) = &p.pro {
                let club = c.team.map(|t| w.nfl_team_name(t)).unwrap_or_default();
                let yrs = c.seasons.len();
                let when = match yrs {
                    0 => "a rookie".to_string(),
                    n => format!("in {} {} NFL season", v.your(), ordinal(n + 1)),
                };
                match (club.is_empty(), c.status) {
                    (true, _) | (_, crate::world::pro::Status::FreeAgent) => {
                        out.push(format!("{} a free agent, {when}, looking for a club", v.are()))
                    }
                    (false, crate::world::pro::Status::PracticeSquad) => {
                        out.push(format!("{} on {club}'s practice squad, {when}", v.are()))
                    }
                    (false, crate::world::pro::Status::Reserve) => {
                        out.push(format!("{} on {club}'s injured reserve, {when}", v.are()))
                    }
                    (false, _) => out.push(format!("{} with {club}, {when}", v.are())),
                }
                if let Some(k) = &c.contract
                    && let Some(last) = k.years.last().map(|y| y.year)
                {
                    let league_year = w.year;
                    if last <= league_year {
                        out.push(format!("{} contract ends after this season: free agency is next", cap(v.your())));
                    } else {
                        out.push(format!("{} contract runs through the {last} season", cap(v.your())));
                    }
                }
            }
        }
        Road::Out => {}
    }
    // His place on the team as he sees it: this season's practice weeks and
    // games, else what he did last season (the talks audit, 2026-11-16: a
    // DT in the offseason asked where he stood on the depth chart, then "do
    // I have a real role?" right after "you're my first pick at DT").
    // Out of season his place is what he saw last, dated, and when it gets
    // set again (`year::place_off`; Cain 2026-11-18: "he's asking for proof
    // while we're in fucking january").
    let who = if v == Voice::You { super::year::Who::You } else { super::year::Who::Son };
    let off = if matches!(p.stage, Stage::College | Stage::Pro | Stage::HighSchool) && !matches!(road, Road::DraftBound)
    {
        super::year::place_off(w, i, who)
    } else {
        None
    };
    if let Some(words) = off {
        out.extend(words);
    } else if matches!(p.stage, Stage::College | Stage::Pro | Stage::HighSchool) && !matches!(road, Road::DraftBound) {
        match folk::seen::first_now(w, i) {
            Some(true) => out.push(format!("{} with the starters this season", v.are())),
            Some(false) => out.push(format!("{} not a starter right now", v.are())),
            None => out.extend(last_role(w, i, v)),
        }
    }
    // An injury he knows he has.
    let chart = super::super::office::doctor::chart(w, i);
    if chart.hurt() {
        let weeks = chart.days.div_ceil(7).max(1);
        out.push(format!("{} hurt right now, out about {} week{}", v.are(), weeks, if weeks == 1 { "" } else { "s" }));
    }
    out
}

/// What he did last season, when nothing this season shows him his place
/// yet: the last thing he saw.
fn last_role(w: &World, i: usize, v: Voice) -> Option<String> {
    let p = &w.people[i];
    let you = v == Voice::You;
    if p.stage == Stage::College && p.college_seasons == 0 {
        return Some(format!(
            "{} played a college snap yet: nobody's place for this season is set until practice starts",
            if you { "You haven't" } else { "He hasn't" }
        ));
    }
    let did = match p.last_role? {
        PlayingTime::Starter => "started",
        PlayingTime::Rotation => "played in the rotation, not as a starter",
        PlayingTime::Backup if you => "were a backup",
        PlayingTime::Backup => "was a backup",
        PlayingTime::Redshirt => "redshirted",
        PlayingTime::PracticeSquad if you => "were on the practice squad",
        PlayingTime::PracticeSquad => "was on the practice squad",
        PlayingTime::Unsigned => return None,
    };
    Some(format!("Last season {} {did}. Nobody's place for this season is set until practice starts", v.subj()))
}

impl Voice {
    fn subj_cap(self) -> &'static str {
        match self {
            Voice::You => "You've",
            Voice::Son => "He's",
        }
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

fn ordinal(n: usize) -> String {
    let suf = match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{n}{suf}")
}

fn claims_date(d: crate::calendar::Date) -> String {
    folk::claims::words::date_words(d)
}

/// Your boss's own place, in his terms: he runs the program you coach, sets
/// your goals, holds the budget and judges you at the review. How the team
/// has done under you is on the card with your history (`between`).
fn boss(w: &World) -> Vec<String> {
    if Team::seat(w).is_none() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let you = folk::scene::your_name(w);
    out.push(format!(
        "You're {you}'s boss: he coaches for you. You set his goals each season, you hold the budget, and you judge him at the season's review. You never ask him what he's judged on: you're the one who judges him"
    ));
    // The team's record under you is on the card with your history
    // together (`between`).
    out
}

/// Where the man across stands, in the second person, for the voice's card
/// ("Where you stand"). Empty for a man whose situation the card already
/// carries (a coach of yours makes his case from `stakes::case_facts`).
pub fn words(w: &World, him: Them) -> Vec<String> {
    match him {
        Them::Player(id) => folk::person(w, id).map(|i| player(w, i, Voice::You)).unwrap_or_default(),
        Them::Kin(id, _) => folk::person(w, id).map(|i| player(w, i, Voice::Son)).unwrap_or_default(),
        _ if makeup::kind(w, him) == Kind::Boss => boss(w),
        _ => Vec::new(),
    }
}
