//! Your side's rewards and the job pick's read (S8, S10).
//!
//! S8: every job carries the stories it would hand you. The job pick shows
//! them (what the public sees, and the boss's spun interview read) with a
//! volatility word: Calm, Unsettled, Volatile, On fire.
//!
//! S10 (Cain: "make it enticing to select high volatility jobs"): handling
//! a story well earns reputation (more than at a calm job), lesson points
//! of 0.25 to 1.00 per story by its heat into the ratings it tested (no
//! season cap: "rewarded irregardless if it's 10 or 1 in a season"), loyal
//! ties with the people you saw through it, a career chapter, an office
//! memento and, in time, a name. A volatile job also comes with a hazard
//! premium: a guaranteed year more, a bigger buyout and a boss who gives
//! you longer. Handling a story badly costs reputation and the boss's
//! patience. Lesson points are banked by rating and, once a whole point
//! is banked, a letter lets you place it.
//!
//! Sizes are Claude's picks (proposed, not confirmed).

use super::{BeatKind, Family, Part, Phase, Rating, Story};
use crate::calendar::{Date, Weekday};
use crate::world::World;
use crate::world::coach::Employer;
use crate::world::desk::{Answer, Head, Mail};
use crate::world::stories::TeamKey;
use crate::world::stories::lines::Fill;

/// A chapter of your career, written from a story that ended.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Chapter {
    pub season: i32,
    /// The story it was written from (0: the season's own chapter).
    pub story: u32,
    pub kind: u16,
    pub title: String,
    pub line: String,
    /// How it went for you, -1 to 1.
    pub score: f32,
    pub team: Option<TeamKey>,
    /// The question it answered, and its public life: first day, last day.
    #[serde(default)]
    pub question: String,
    #[serde(default)]
    pub from: Option<Date>,
    #[serde(default)]
    pub to: Option<Date>,
    /// How it ended, in a few words ("won", "held job").
    #[serde(default)]
    pub outcome: String,
    /// The chapter's text: its key beats told in order.
    #[serde(default)]
    pub text: String,
    /// The beats that mattered, oldest first.
    #[serde(default)]
    pub beats: Vec<(Date, String)>,
    /// Who was in it, by name.
    #[serde(default)]
    pub cast: Vec<String>,
    /// What it left: (kind, text): a tie, a callback, a memento, a name,
    /// a seed (a story it started).
    #[serde(default)]
    pub left: Vec<(String, String)>,
    /// What it paid you: (what, by how much, tone: 0 good, 1 plain, 2 bad).
    #[serde(default)]
    pub paid: Vec<(String, String, u8)>,
    /// How hot the story ran at its peak (0-100): the book keeps the
    /// biggest.
    #[serde(default)]
    pub heat: u8,
}

/// The chapters a season's book keeps: its biggest stories (the season
/// reviews in the mock-ups hold five or six). Every story still pays.
pub const SEASON_CHAPTERS: usize = 6;

/// Your career's book: each season's biggest chapter, and the ones that
/// ran hottest of all (the stories kept for good), three a season at most.
pub fn career_book(w: &World) -> Vec<&Chapter> {
    let all = &w.storylines.credit.chapters;
    let mut out: Vec<&Chapter> = Vec::new();
    let mut seasons: Vec<i32> = all.iter().map(|c| c.season).collect();
    seasons.sort_unstable();
    seasons.dedup();
    for y in seasons {
        let mut v: Vec<&Chapter> = all.iter().filter(|c| c.season == y).collect();
        v.sort_by(|a, b| b.heat.cmp(&a.heat).then(a.to.cmp(&b.to)));
        let keep: Vec<&Chapter> =
            v.iter().enumerate().filter(|(k, c)| *k == 0 || c.heat >= 85).map(|(_, c)| *c).take(3).collect();
        out.extend(keep);
    }
    out.sort_by(|a, b| a.season.cmp(&b.season).then(a.to.cmp(&b.to)));
    out
}

/// Your rewards and your record with stories.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Credit {
    /// Lesson points banked by rating (`Rating::index`), not yet placed.
    pub bank: [f32; 11],
    /// Points placed so far, by rating.
    pub placed: [f32; 11],
    /// Stories you handled well and badly, by season.
    pub record: Vec<(i32, u16, u16)>,
    /// The job you took, how volatile it was then (0-100) and when.
    pub hired_into: Option<(TeamKey, f32, Date)>,
    /// Players a story tied to you: (player id, season).
    pub loyal: Vec<(u32, i32)>,
    pub chapters: Vec<Chapter>,
    /// The name the papers gave you, once earned.
    pub name: Option<String>,
    /// The lesson letter waiting for your answer: its mail id.
    pub letter: Option<u64>,
    /// The day each followed story last reached your inbox.
    pub(crate) followed_on: Vec<(u32, Date)>,
}

/// The act code on a lesson letter's answers (`desk::Answer::act`): the
/// target is the rating's index. Acted on the next morning.
pub const ACT_LESSON: u8 = 64;

/// How volatile a job is now, 0-100: the heat of the public stories it
/// would hand a new coach, weighted by how much each unsettles a job, and
/// the bad endings of the last year.
pub fn volatility(w: &World, t: TeamKey) -> f32 {
    let d = w.today;
    let open: f32 = w
        .storylines
        .open
        .iter()
        .filter(|s| s.public && s.phase.open() && s.about(t))
        .map(|s| s.heat * (0.25 + s.def().volatile) * if s.def().tone < 0 { 1.0 } else { 0.4 })
        .sum();
    let past: f32 = w
        .storylines
        .open
        .iter()
        .chain(w.storylines.ended.iter())
        .filter(|s| s.public && !s.phase.open() && s.team == Some(t))
        .filter(|s| s.closed.is_some_and(|c| c.days_to(d) <= 365) && s.valence() < -0.2)
        .map(|s| 0.5 * s.peak_heat * s.def().volatile)
        .sum();
    (open + past).min(100.0)
}

/// The word the job pick shows.
pub fn word(v: f32) -> &'static str {
    match v {
        x if x < 20.0 => "Calm",
        x if x < 45.0 => "Unsettled",
        x if x < 70.0 => "Volatile",
        _ => "On fire",
    }
}

/// One inherited story on the job pick.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Inherited {
    pub id: u32,
    pub kind: String,
    pub question: String,
    pub heat: f32,
    pub phase: &'static str,
    /// What the papers say now.
    pub public: String,
    /// The boss's version in the interview.
    pub spin: String,
    /// The lesson points it pays if it ends well (0.25-1.00), and the
    /// ratings they go into.
    pub lesson: f32,
    pub tests: Vec<&'static str>,
}

/// What a job hands a new coach (S8): its stories, the word, the premium.
#[derive(Clone, Debug, serde::Serialize)]
pub struct JobRead {
    pub volatility: f32,
    pub word: &'static str,
    pub stories: Vec<Inherited>,
    /// The hazard premium a volatile job pays, in words (empty when calm).
    pub premium: Vec<String>,
    /// What handling it well would pay, in words.
    pub rewards: Vec<String>,
    /// Reputation (of 20) if every story ends well, and if every one ends
    /// badly.
    pub rep_up: f64,
    pub rep_down: f64,
    /// The players a story there would tie to you.
    pub ties: Vec<String>,
    /// The chapter its hottest story would write.
    pub chapter: String,
    /// The office keepsake on offer, and the name the papers might give.
    pub memento: String,
    pub name: String,
    /// The boss's patience, in words.
    pub patience: String,
    /// The premium in numbers: a year more guaranteed, a bigger buyout (%).
    pub extra_years: u8,
    pub buyout_extra: u8,
}

/// The job pick's read of a team (S8).
pub fn job_read(w: &mut World, t: TeamKey) -> JobRead {
    let v = volatility(w, t);
    let mut list: Vec<Story> =
        w.storylines.open.iter().filter(|s| s.public && s.phase.open() && s.about(t)).cloned().collect();
    list.sort_by(|a, b| b.heat.total_cmp(&a.heat).then(a.id.cmp(&b.id)));
    list.truncate(6);
    let boss = crate::world::folk::boss::boss(w, super::folk_team(t));
    let straight = boss.honesty >= 13.0;
    let mult = 1.0 + v as f64 / 100.0;
    let mut stories = Vec::new();
    let (mut rep_up, mut rep_down) = (0.0, 0.0);
    let mut ties = Vec::new();
    for s in &list {
        let lesson = (0.25 + 0.75 * s.heat / 100.0).clamp(0.25, 1.0);
        rep_up += 0.06 * lesson as f64 * mult;
        rep_down += 0.05 * (s.heat as f64 / 60.0).max(0.4);
        if matches!(s.family(), Family::Battle | Family::Leaving | Family::OffField | Family::Comeback | Family::Rise)
            && let Some(n) = s.lead_player().and_then(|p| member_name(w, super::Who::Player(p)))
            && !ties.contains(&n)
        {
            ties.push(n);
        }
        let f = super::tell::fill(w, s);
        let public = super::tell::last_said(s).map_or_else(|| super::tell::question(w, s), |b| b.text.clone());
        let extra = if straight { Some("straight") } else { None };
        let spin = super::tell::line(w, s, BeatKind::Quote, Some(if straight { "spin_straight" } else { "spin" }), &f)
            .or_else(|| super::tell::line(w, s, BeatKind::Quote, extra, &f))
            .unwrap_or_else(|| spin_plain(s, straight));
        stories.push(Inherited {
            id: s.id,
            kind: s.def().name.to_string(),
            question: super::tell::question(w, s),
            heat: s.heat.round(),
            phase: s.phase.name(),
            public,
            spin: format!("{}: \"{}\"", boss.name, spin.trim_matches('"')),
            lesson,
            tests: s.def().tests.iter().map(|r| r.name()).collect(),
        });
    }
    let (premium, rewards) = premium_words(v);
    let seed = w.options.seed;
    let year = w.year;
    let chapter = list
        .first()
        .and_then(|s| {
            let f = super::tell::fill(w, s);
            [format!("sl.{}.chapter", s.def().key), format!("sl.{}.chapter", s.family().key())]
                .iter()
                .find_map(|k| w.storylines.line(seed, k, &f, year, &[]))
        })
        .unwrap_or_else(|| "none yet".into());
    let memento = if list.iter().any(|s| s.heat >= 70.0) {
        "one, if a story this hot ends well".to_string()
    } else {
        "none yet: only the hottest stories leave one".to_string()
    };
    let name = if v >= 45.0 && w.storylines.credit.name.is_none() {
        "after three stories handled well in jobs like this".to_string()
    } else {
        "none yet".to_string()
    };
    let patience =
        if v >= 45.0 { "two seasons before the boss judges you".to_string() } else { "judged as usual".to_string() };
    let (extra_years, buyout_extra) = if v >= 70.0 {
        (1, 35)
    } else if v >= 45.0 {
        (1, 20)
    } else {
        (0, 0)
    };
    JobRead {
        volatility: v.round(),
        word: word(v),
        stories,
        premium,
        rewards,
        rep_up: (rep_up * 100.0).round() / 100.0,
        rep_down: (rep_down * 100.0).round() / 100.0,
        ties,
        chapter,
        memento,
        name,
        patience,
        extra_years,
        buyout_extra,
    }
}

fn spin_plain(s: &Story, straight: bool) -> String {
    match (straight, s.def().tone < 0) {
        (true, true) => "I won't pretend it's nothing. It's the first thing you'd have to handle.".into(),
        (true, false) => "It's a good problem to have. Keep it going.".into(),
        (false, true) => "Every program has a little noise. It's overblown.".into(),
        (false, false) => "That's the kind of thing that happens here.".into(),
    }
}

/// The hazard premium and the rewards, in words, for a volatility.
fn premium_words(v: f32) -> (Vec<String>, Vec<String>) {
    let mut premium = Vec::new();
    if v >= 45.0 {
        premium.push("A guaranteed year more on the contract".to_string());
        premium.push(format!("A buyout {}% bigger", if v >= 70.0 { 35 } else { 20 }));
        premium.push("A boss who gives you two seasons before he judges".to_string());
    }
    let mult = 1.0 + v / 100.0;
    let rewards = vec![
        format!("Reputation from each story you handle well: {mult:.1} times a calm job's"),
        "Lesson points of 0.25 to 1.00 per story, by its heat, into the ratings it tested".to_string(),
        "Players and staff tied to you for good, a chapter, a keepsake for the office".to_string(),
    ];
    (premium, rewards)
}

/// The hazard premium for an offer (S10): extra guaranteed years and the
/// buyout's extra share, for a head job that is volatile now.
pub(crate) fn hazard(w: &World, e: Employer) -> Option<(u8, f64)> {
    let t = super::team_key(crate::world::folk::Team::from_employer(e));
    let v = volatility(w, t);
    if v < 45.0 {
        return None;
    }
    Some((1, if v >= 70.0 { 0.35 } else { 0.2 }))
}

/// You took a job: what it was like then, and the premium's buyout.
pub(crate) fn hired(w: &mut World, e: Employer) {
    let team = super::team_key(crate::world::folk::Team::from_employer(e));
    let v = volatility(w, team);
    w.storylines.credit.hired_into = Some((team, v, w.today));
    if v >= 45.0
        && let Some(c) = super::super::stories::seat_coach(w)
        && let Some(k) = w.coaches[c].contract.as_mut()
    {
        k.buyout_share = (k.buyout_share * if v >= 70.0 { 1.35 } else { 1.2 }).min(1.0);
    }
    // The stories you inherit are yours now.
    for s in w.storylines.open.iter_mut().filter(|s| s.about(team)) {
        s.near = true;
        if s.you.is_none() {
            s.you = Some(super::team_part(s, team).unwrap_or(Part::Decider));
        }
    }
}

/// The premium's patience: your boss reads your first two seasons in a
/// volatile job more kindly (a negative push on the review).
pub(crate) fn patience(w: &World, c: usize) -> f64 {
    if super::super::stories::seat_coach(w) != Some(c) {
        return 0.0;
    }
    match w.storylines.credit.hired_into {
        Some((_, v, d)) if v >= 45.0 && d.year() >= w.year - 2 => -0.03 * (v as f64 / 70.0).min(1.3),
        _ => 0.0,
    }
}

/// A story you were in has ended: what it paid, what it cost (S10).
pub(crate) fn ended(w: &mut World, id: u32) {
    let Some(s) = w.storylines.get(id).cloned() else { return };
    let Some(part) = s.you else { return };
    if super::super::stories::seat_coach(w).is_none() {
        return;
    }
    if s.phase == Phase::Fizzled || !s.public {
        return;
    }
    // How it went for you: the ending as it reads for your side, and how
    // you handled it on the way.
    let v = if part == Part::Rival { -s.valence() } else { s.valence() };
    let score = (0.7 * v + 0.45 * s.handled).clamp(-1.0, 1.0);
    let season = s.season;
    let heat = s.peak_heat;
    let fam = s.family();
    let mult = 1.0 + w.storylines.credit.hired_into.map_or(0.0, |x| x.1) / 100.0;
    // What it paid and what it left, for the chapter.
    let mut paid: Vec<(String, String, u8)> = Vec::new();
    let mut left: Vec<(String, String)> = Vec::new();
    if score >= 0.3 {
        // Lesson points: 0.25 at a cool story to 1.00 at a blazing one,
        // shared by the ratings it tested.
        let lesson = (0.25 + 0.75 * heat / 100.0).clamp(0.25, 1.0);
        let tests = s.def().tests;
        if !tests.is_empty() {
            let each = lesson / tests.len() as f32;
            for r in tests {
                w.storylines.credit.bank[r.index()] += each;
            }
            let names: Vec<&str> = tests.iter().map(|r| r.name()).collect();
            paid.push((format!("Lesson points into {}", and_list(&names)), format!("+{lesson:.2}"), 0));
        }
        let rep = 0.06 * lesson as f64 * mult as f64;
        reputation(w, rep);
        paid.push(("Reputation, of 20".into(), format!("+{rep:.2}"), 0));
        if boss_trust(w, fam, 0.03) {
            paid.push(("Your boss's trust".into(), "+3".into(), 0));
        }
        // Loyal ties with the player you saw through it.
        if matches!(fam, Family::Battle | Family::Leaving | Family::OffField | Family::Comeback | Family::Rise)
            && let Some(pid) = s.lead_player()
            && let Some(i) = super::pi(w, pid)
        {
            w.people[i].trust = (w.people[i].trust + 0.1).min(1.0);
            if !w.storylines.credit.loyal.iter().any(|x| x.0 == pid) {
                w.storylines.credit.loyal.push((pid, season));
            }
            left.push(("tie".into(), w.people[i].name.clone()));
        }
        if heat >= 70.0 {
            left.push(("memento".into(), memento(w, &s)));
        }
        tally(w, season, true);
        if let Some(n) = maybe_name(w) {
            left.push(("name".into(), format!("\"{n}\"")));
        }
    } else if score <= -0.3 {
        let rep = 0.05 * (heat as f64 / 60.0).max(0.4);
        reputation(w, -rep);
        paid.push(("Reputation, of 20".into(), format!("\u{2212}{rep:.2}"), 2));
        if boss_trust(w, fam, -0.04) {
            paid.push(("Your boss's trust".into(), "\u{2212}4".into(), 2));
        }
        tally(w, season, false);
    } else {
        paid.push(("Lesson points".into(), "none".into(), 1));
    }
    if heat >= 35.0 {
        chapter(w, &s, score, paid, left);
    }
}

/// "a", "a and b", "a, b and c".
fn and_list(v: &[&str]) -> String {
    match v {
        [] => String::new(),
        [a] => a.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn tally(w: &mut World, season: i32, good: bool) {
    let r = &mut w.storylines.credit.record;
    match r.iter_mut().find(|x| x.0 == season) {
        Some(x) => {
            if good {
                x.1 += 1;
            } else {
                x.2 += 1;
            }
        }
        None => r.push((season, good as u16, (!good) as u16)),
    }
}

/// Your reputation moves (your coach and your profile together).
fn reputation(w: &mut World, by: f64) {
    let Some(c) = super::super::stories::seat_coach(w) else { return };
    let co = &mut w.coaches[c];
    co.reputation = (co.reputation + by).clamp(1.0, 20.0);
    let r = co.reputation;
    if let Some(k) = w.career.as_mut() {
        k.you.reputation = r;
    }
}

/// Your boss's trust moves on the stories he cares about.
fn boss_trust(w: &mut World, fam: Family, by: f32) -> bool {
    if !matches!(
        fam,
        Family::Seat | Family::OffField | Family::You | Family::Battle | Family::Program | Family::Leaving
    ) {
        return false;
    }
    let Some(team) = crate::world::folk::Team::seat(w) else { return false };
    let b = crate::world::folk::boss::boss(w, team);
    w.folk.mine.nudge(b.key(), by);
    true
}

/// A shelf piece for the stories that ran hottest.
fn memento(w: &mut World, s: &Story) -> String {
    let f = super::tell::fill(w, s);
    let text = super::tell::line(w, s, BeatKind::Callback, Some("memento"), &f)
        .unwrap_or_else(|| format!("A framed front page: {}", super::tell::question(w, s)));
    let kind = match s.family() {
        Family::Rivalry | Family::Streak => crate::world::stories::story::MementoKind::GameBall,
        Family::Seat | Family::You | Family::Program => crate::world::stories::story::MementoKind::Plaque,
        Family::Recruiting | Family::Draft => crate::world::stories::story::MementoKind::Photo,
        _ => crate::world::stories::story::MementoKind::Gift,
    };
    crate::world::stories::story::memento(w, kind, text.clone());
    text
}

/// A chapter from a story you were in: its title, its key beats told in
/// order, who was in it, what it paid and what it left.
fn chapter(w: &mut World, s: &Story, score: f32, paid: Vec<(String, String, u8)>, mut left: Vec<(String, String)>) {
    let f = super::tell::fill(w, s);
    let e = s.ending_key();
    let seed = w.options.seed;
    let year = w.year;
    let title = [
        format!("sl.{}.chapter.{e}", s.def().key),
        format!("sl.{}.chapter", s.def().key),
        format!("sl.{}.chapter", s.family().key()),
    ]
    .iter()
    .find_map(|k| w.storylines.line(seed, k, &f, year, &[]))
    .unwrap_or_else(|| s.def().name.to_string());
    let line = if s.legacy.is_empty() {
        super::tell::last_said(s).map_or_else(String::new, |b| b.text.clone())
    } else {
        s.legacy.clone()
    };
    // The beats that mattered: the spark, the hottest in between, the answer.
    let said: Vec<&super::Beat> = s.beats.iter().filter(|b| b.what.public() && !b.text.is_empty()).collect();
    let mut keep: Vec<&super::Beat> = Vec::new();
    if let Some(b) = said.iter().find(|b| b.what == BeatKind::Spark).or(said.first()) {
        keep.push(b);
    }
    let mut mid: Vec<&&super::Beat> =
        said.iter().filter(|b| !matches!(b.what, BeatKind::Spark | BeatKind::Resolve)).collect();
    mid.sort_by(|a, b| b.heat.cmp(&a.heat).then(a.date.cmp(&b.date)));
    for b in mid.into_iter().take(2) {
        keep.push(b);
    }
    if let Some(b) = said.iter().rev().find(|b| b.what == BeatKind::Resolve) {
        keep.push(b);
    }
    keep.sort_by_key(|b| b.date);
    keep.dedup_by(|a, b| std::ptr::eq(*a, *b));
    let beats: Vec<(Date, String)> = keep.iter().map(|b| (b.date, b.text.clone())).collect();
    let mut text = beats.iter().map(|b| b.1.as_str()).collect::<Vec<_>>().join(" ");
    if !s.legacy.is_empty() && !text.contains(&s.legacy) {
        text = format!("{text} {}", s.legacy).trim().to_string();
    }
    let cast: Vec<String> = s.cast.iter().filter_map(|m| member_name(w, m.who)).take(5).collect();
    if !s.legacy.is_empty() {
        left.push(("callback".into(), s.legacy.clone()));
    }
    // The stories it started that are still running.
    for &l in &s.links {
        if let Some(o) = w.storylines.get(l)
            && o.phase.open()
            && o.opened >= s.opened
        {
            left.push(("seed".into(), super::tell::question(w, o)));
        }
    }
    left.truncate(5);
    let c = Chapter {
        season: s.season,
        story: s.id,
        kind: s.kind,
        title: title.clone(),
        line,
        score,
        team: s.team,
        question: super::tell::question(w, s),
        from: Some(s.public_on.unwrap_or(s.opened)),
        to: Some(s.closed.unwrap_or(w.today)),
        outcome: e.replace('_', " "),
        text,
        beats,
        cast,
        left,
        paid,
        heat: s.peak_heat.round().clamp(0.0, 100.0) as u8,
    };
    // The season's book keeps its biggest: a bigger story pushes out the
    // smallest (the newest of equals goes first).
    let ch = &mut w.storylines.credit.chapters;
    ch.push(c);
    let season = s.season;
    if ch.iter().filter(|x| x.season == season).count() > SEASON_CHAPTERS
        && let Some(k) = ch
            .iter()
            .enumerate()
            .filter(|(_, x)| x.season == season)
            .min_by(|a, b| a.1.heat.cmp(&b.1.heat).then(b.0.cmp(&a.0)))
            .map(|(k, _)| k)
    {
        ch.remove(k);
    }
    if !w.storylines.credit.chapters.iter().any(|x| x.story == s.id) {
        return;
    }
    crate::world::stories::story::log(
        w,
        crate::world::stories::story::EntryKind::Chapter,
        format!("{title}. {}", super::tell::question(w, s)),
        s.team,
    );
}

/// A cast member's name for the screens.
/// A cast member's name, from the world or, once he has left it, from the
/// names the story kept when it ended (a parent by his son's).
pub fn member_name_in(w: &World, s: &Story, who: super::Who) -> Option<String> {
    use super::Who;
    member_name(w, who).or_else(|| match who {
        Who::Parent(id) => s.names.iter().find(|x| x.0 == Who::Player(id)).map(|x| {
            let last = x.1.split_whitespace().last().unwrap_or("his");
            format!("{last}'s family")
        }),
        _ => s.names.iter().find(|x| x.0 == who).map(|x| x.1.clone()),
    })
}

pub fn member_name(w: &World, who: super::Who) -> Option<String> {
    use super::Who;
    match who {
        Who::Player(id) => super::pi(w, id).map(|i| w.people[i].name.clone()),
        Who::Coach(c) => w.coaches.get(c as usize).map(|c| c.name.clone()),
        Who::Team(t) => Some(crate::world::stories::short_name(w, t)),
        Who::Boss(t) => Some(crate::world::folk::boss::boss(w, super::folk_team(t)).name),
        Who::Parent(id) => super::pi(w, id).map(|i| {
            let last = w.people[i].name.split_whitespace().last().unwrap_or("").to_string();
            format!("{}'s family", if last.is_empty() { "his" } else { &last })
        }),
    }
}

/// A name the papers give a coach who keeps walking into fires and
/// putting them out: three stories handled well at volatile jobs.
fn maybe_name(w: &mut World) -> Option<String> {
    if w.storylines.credit.name.is_some() {
        return None;
    }
    let (_, v, _) = w.storylines.credit.hired_into?;
    let well: u16 = w.storylines.credit.record.iter().map(|x| x.1).sum();
    if v < 45.0 || well < 3 {
        return None;
    }
    let c = super::super::stories::seat_coach(w)?;
    let seed = w.options.seed;
    let year = w.year;
    let name =
        w.storylines.line(seed, "sl.name.fixer", &Fill::new(), year, &[]).unwrap_or_else(|| "The Fixer".to_string());
    w.storylines.credit.name = Some(name.clone());
    let team = crate::world::stories::seat_team(w);
    use crate::world::stories::nick::{Coiner, NickKind, Nickname, Reaction, Who as NWho};
    w.stories.nicknames.push(Nickname {
        who: NWho::Coach(c as u32),
        name: name.clone(),
        kind: NickKind::Earned,
        theme: 13,
        coined: w.today,
        by: Coiner::NationalShow,
        outlet: "a national show".to_string(),
        moment: "the jobs nobody else wanted".to_string(),
        team,
        stick: 0.7,
        stage: 2,
        spread: [Some(w.today), Some(w.today), None],
        reaction: Reaction::Embraced,
        merch: false,
        retired: false,
        replaced: None,
        heir_of: None,
        last_year: year as i16,
    });
    crate::world::stories::story::log(
        w,
        crate::world::stories::story::EntryKind::Nickname,
        format!("The papers start calling you \"{name}\"."),
        team,
    );
    Some(name)
}

/// A followed story's public beat reaches your inbox (once a day each).
pub(crate) fn follow_note(w: &mut World, id: u32) {
    let d = w.today;
    if w.storylines.credit.followed_on.iter().any(|x| x.0 == id && x.1 == d) {
        return;
    }
    let Some(s) = w.storylines.get(id).cloned() else { return };
    let Some(b) = super::tell::last_said(&s).cloned() else { return };
    if b.date != d {
        return;
    }
    w.storylines.credit.followed_on.retain(|x| x.1.days_to(d) < 3 && x.0 != id);
    w.storylines.credit.followed_on.push((id, d));
    let q = super::tell::question(w, &s);
    let mut m = mail(w, &format!("follow|{id}"), Head::Paper, "The clips".into(), "Stories you follow".into(), q);
    m.body = vec![b.text.clone(), format!("{} · heat {} of 100.", s.phase.name(), s.heat.round())];
    m.loud = 1;
    send(w, m);
}

/// Your staff's memo on one of your stories (`tell::maybe_memo`).
pub(crate) fn memo(w: &mut World, s: &Story, team: TeamKey, body: Vec<String>) {
    let (from, role) = staff_voice(w, team);
    let mut m = mail(w, &format!("memo|{}", s.id), Head::Memo, from.clone(), role.clone(), super::tell::question(w, s));
    m.body = body;
    m.sign = from;
    m.loud = 1;
    if let Some(pid) = s.lead_player() {
        m.about = Some((0, pid));
        m.ids = vec![pid];
    }
    send(w, m);
}

/// Who writes your staff's memos: the chief of staff or the operations
/// director, or your top assistant at a high school.
fn staff_voice(w: &World, team: TeamKey) -> (String, String) {
    use crate::world::folk::cast::{self, CastRef, Role};
    let t = super::folk_team(team);
    for r in [Role::ChiefOfStaff, Role::Ops] {
        if cast::has(w, t, r) {
            let c = cast::get(w, CastRef::new(t, r, 0));
            return (c.name.clone(), c.title.clone());
        }
    }
    match w.career.as_ref().and_then(|c| c.hs_staff.first()) {
        Some(a) => (a.name.clone(), "Assistant coach".into()),
        None => ("Your staff".into(), "Staff".into()),
    }
}

fn mail(w: &World, tag: &str, head: Head, from: String, role: String, subject: String) -> Mail {
    let id = tag
        .bytes()
        .fold(0x5701_0000_0000_0000u64 ^ w.today.0 as u64, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
    Mail {
        id,
        date: w.today,
        minute: 8 * 60 + 5,
        head,
        org: String::new(),
        org_line: String::new(),
        from,
        role,
        to: crate::world::folk::scene::your_name(w),
        subject,
        body: Vec::new(),
        table: Vec::new(),
        attach: Vec::new(),
        sign: String::new(),
        keys: 1 << 10,
        about: None,
        answers: Vec::new(),
        answered: None,
        answer_by: None,
        read: false,
        saved: false,
        loud: 1,
        ids: Vec::new(),
        org_team: None,
    }
}

fn send(w: &mut World, m: Mail) {
    let Some(c) = w.career.as_mut() else { return };
    if !c.desk.has(m.id) {
        c.desk.mails.push(m);
        c.desk.mails.sort_by_key(|m| (m.date, m.minute));
    }
}

/// Acts on your answer to a lesson letter: the point goes where you put it.
pub fn desk_answers(w: &mut World) {
    let Some(id) = w.storylines.credit.letter else { return };
    let pick = w.career.as_ref().and_then(|c| {
        let m = c.desk.mails.iter().find(|m| m.id == id)?;
        let a = m.answers.get(m.answered? as usize)?;
        (a.act == ACT_LESSON).then_some(a.target)
    });
    if let Some(r) = pick {
        place(w, Rating::from_index(r as usize));
        w.storylines.credit.letter = None;
    }
}

/// Places one whole lesson point into a rating.
fn place(w: &mut World, r: Rating) {
    let bank = &mut w.storylines.credit.bank;
    // The point comes out of that rating's bank first, then the fullest.
    let mut need = 1.0f32;
    let take = bank[r.index()].min(need);
    bank[r.index()] -= take;
    need -= take;
    while need > 0.001 {
        let Some(k) = (0..11).filter(|&k| bank[k] > 0.0).max_by(|&a, &b| bank[a].total_cmp(&bank[b])) else { break };
        let t = bank[k].min(need);
        bank[k] -= t;
        need -= t;
    }
    w.storylines.credit.placed[r.index()] += 1.0;
    if let Some(c) = super::super::stories::seat_coach(w) {
        let co = &mut w.coaches[c];
        let mut y = co.yours;
        r.add(&mut co.ability, y.as_mut(), 1.0);
        co.yours = y;
    }
    if let Some(k) = w.career.as_mut() {
        let y = &mut k.you;
        let v = match r {
            Rating::Motivation => &mut y.motivation,
            Rating::Discipline => &mut y.discipline,
            Rating::Relationships => &mut y.relationships,
            Rating::Recruiting => &mut y.recruiting,
            Rating::Network => &mut y.network,
            Rating::GameManagement => &mut y.game_management,
            Rating::Scheme => &mut y.scheme,
            Rating::PlayCalling => &mut y.yours.play_calling,
            Rating::Adaptation => &mut y.yours.adaptation,
            Rating::Evaluation => &mut y.yours.evaluation,
            Rating::Negotiating => &mut y.yours.negotiating,
        };
        *v = (*v + 1.0).clamp(1.0, 20.0);
    }
}

/// Every day of a career: the lesson letter when a whole point is banked
/// (one waits at a time; unanswered for three weeks, it goes where you
/// learned most), and the season's chapter after its last game.
pub(crate) fn day(w: &mut World, d: Date) {
    if w.career.is_none() {
        return;
    }
    if let Some(id) = w.storylines.credit.letter {
        let lapsed = w.career.as_ref().is_some_and(|c| {
            c.desk.mails.iter().find(|m| m.id == id).is_none_or(|m| m.answered.is_none() && m.date.days_to(d) > 21)
        });
        if lapsed {
            let bank = w.storylines.credit.bank;
            let best = (0..11).max_by(|&a, &b| bank[a].total_cmp(&bank[b])).unwrap_or(0);
            place(w, Rating::from_index(best));
            w.storylines.credit.letter = None;
        }
        return;
    }
    let total: f32 = w.storylines.credit.bank.iter().sum();
    if total < 1.0 || d.weekday() != Weekday::Mon {
        return;
    }
    let bank = w.storylines.credit.bank;
    let mut order: Vec<usize> = (0..11).filter(|&k| bank[k] > 0.01).collect();
    order.sort_by(|&a, &b| bank[b].total_cmp(&bank[a]));
    order.truncate(3);
    let Some(team) = crate::world::stories::seat_team(w) else { return };
    let (from, role) = staff_voice(w, team);
    let mut m = mail(w, "lesson", Head::Memo, from.clone(), role, "A lesson learned".into());
    m.body = vec![
        "What the last stories put you through has taught you something. You have a whole point to place.".into(),
        format!(
            "Banked: {}.",
            order
                .iter()
                .map(|&k| format!("{} {:.2}", Rating::from_index(k).name(), bank[k]))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ];
    m.answers = order
        .iter()
        .map(|&k| {
            let r = Rating::from_index(k);
            Answer {
                label: format!("Into {}", r.name()),
                does: format!("Your {} goes up a point.", r.name().to_lowercase()),
                act: ACT_LESSON,
                target: k as u32,
            }
        })
        .collect();
    m.answer_by = Some(d.add(21));
    m.loud = 2;
    m.sign = from;
    let id = m.id;
    send(w, m);
    w.storylines.credit.letter = Some(id);
}
