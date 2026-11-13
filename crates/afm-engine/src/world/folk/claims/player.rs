//! A player on a team: the points of section 5's first table. Each
//! reader looks at one store and returns a point only when the fact is
//! true today (rule 1). The stores about your own team (rulings, promises,
//! your depth chart, the games kept whole, practice) only speak for your
//! players; another team's player gets the points the world keeps for
//! everyone (a case, an injury, a holdout).

use super::kinds::{self, Kind};
use super::sheet::{Point, Ref};
use super::words;
use super::{Man, Reader};
use crate::calendar::Date;
use crate::world::Stage;
use crate::world::folk::courts::Step;
use crate::world::folk::rules::Response;
use crate::world::folk::talks::{PromiseTo, Reason};
use crate::world::folk::{Team, life, scene, talks};
use crate::world::people::Trouble;
use crate::world::pro::market::Hold;

/// A game grade (0-100, 60 average) that makes his last game one of the
/// best at his spot. Proposed, not confirmed.
pub const BIG_GRADE: f64 = 80.0;
/// A grade that makes it a bad one. Proposed, not confirmed.
pub const BAD_GRADE: f64 = 45.0;
/// Plays graded before a game's grade says anything.
/// Proposed, not confirmed.
pub const GRADED_MIN: u32 = 10;
/// A slump: this many games in a row, each this far under his own average.
/// Proposed, not confirmed.
pub const SLUMP_GAMES: usize = 3;
/// How far under his level each of those games is, in grade points. Proposed, not confirmed.
pub const SLUMP_UNDER: f64 = 6.0;
/// A pro outplayed his deal: his market price over his pay, small and big.
/// Proposed, not confirmed.
pub const PAY_SMALL: f64 = 1.25;
/// From this much over his pay the gap is a big one. Proposed, not confirmed.
pub const PAY_BIG: f64 = 1.75;
/// A college player under the going rate for his spot and role: paid
/// under this share of it, small and big (big is the July 1 check's
/// half).
/// Proposed, not confirmed.
pub const NIL_SMALL: f64 = 0.8;
/// Under this share of the going rate the gap is a big one. Proposed, not confirmed.
pub const NIL_BIG: f64 = 0.5;
/// The going rate has to be worth naming.
/// Proposed, not confirmed.
pub const NIL_FLOOR: f64 = 25_000.0;
/// The portal is on his mind from this lean (as `talks::concern` reads it).
/// Proposed, not confirmed.
pub const PORTAL_LEAN: f64 = 0.25;
/// The eligibility line the term checks read, and how far over it still
/// counts as near.
/// Proposed, not confirmed.
pub const GRADE_LINE: f64 = 2.5;
/// This close over the line counts as near it. Proposed, not confirmed.
pub const GRADE_NEAR: f64 = 0.3;
/// A bad life event weighs for this long (`life::open_event`), a good one
/// is news for a month.
/// Proposed, not confirmed.
pub const LIFE_BAD_DAYS: i32 = 90;
/// Good news stays news this long. Proposed, not confirmed.
pub const LIFE_GOOD_DAYS: i32 = 30;

pub(super) fn points(r: &Reader, m: &Man, out: &mut Vec<Point>) {
    let mut add = |p: Option<Point>| out.extend(p);
    add(arrest(r, m));
    add(league_case(r, m));
    add(case_closed(r, m));
    add(suspended(r, m));
    add(rule_break(r, m));
    let (won, lost) = job(r, m);
    add(won);
    add(lost);
    add(buried(r, m));
    let (big, bad, slump) = games(r, m);
    add(big);
    add(bad);
    add(slump);
    add(award(r, m));
    add(hurt(r, m));
    add(back(r, m));
    for p in life_events(r, m) {
        add(Some(p));
    }
    for p in promises(r, m) {
        add(Some(p));
    }
    add(pay(r, m));
    add(wants_out(r, m));
    add(portal(r, m));
    add(draft(r, m));
    add(friend_left(r, m));
    add(new_coach(r, m));
    add(grades(r, m));
    add(parent_called(r, m));
}

fn yes_no(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

/// How many times trouble has found him: his notes keep one line a time.
fn troubles(r: &Reader, m: &Man) -> usize {
    r.w.people[m.i].notes.iter().filter(|n| n.starts_with("Off-field trouble")).count()
}

/// A case in the courts: arrested, charged or in court. Whether he did
/// it is decided when the case opens and comes out on the court day: until
/// then it is his alone to know (`hid`), never a line's fact.
fn arrest(r: &Reader, m: &Man) -> Option<Point> {
    let c =
        r.w.folk
            .courts
            .open
            .iter()
            .find(|c| c.id == m.id && matches!(c.step, Step::Arrest | Step::Charged | Step::Court))?;
    let step = match c.step {
        Step::Arrest => "arrest",
        Step::Charged => "charged",
        _ => "court",
    };
    let before = r.w.folk.courts.closed.iter().filter(|x| x.id == m.id && outcome_of(x.outcome.as_deref()).is_some());
    let again = troubles(r, m) > 1 || before.count() > 0;
    Some(
        Point::new(Kind::Arrest)
            .on(c.opened)
            .due(c.next)
            .text("charge", c.what.clone())
            .text("when", words::when_words(c.opened, r.today))
            .state("step", step)
            .state("again", yes_no(again))
            .hide("did", yes_no(c.guilty)),
    )
}

/// A case the league alone handles (a failed drug test, a gambling
/// violation): no court, only the ruling to come.
fn league_case(r: &Reader, m: &Man) -> Option<Point> {
    if r.w.folk.courts.open.iter().any(|c| c.id == m.id && matches!(c.step, Step::Arrest | Step::Charged | Step::Court))
    {
        return None;
    }
    let c = r.w.market.cases.iter().find(|c| c.person == m.i && matches!(c.kind, Trouble::Ped | Trouble::Gambling))?;
    Some(
        Point::new(Kind::Arrest)
            .due(c.ruling)
            .text("charge", c.kind.words())
            .state("step", "league")
            .state("again", yes_no(troubles(r, m) > 1))
            .hide("did", yes_no(c.guilty)),
    )
}

/// How a court case ended, from the sentence the courts wrote.
fn outcome_of(text: Option<&str>) -> Option<&'static str> {
    let t = text?;
    if t.contains("was convicted") {
        Some("convicted")
    } else if t.contains("pleaded") {
        Some("pleaded")
    } else if t.contains("acquitted") {
        Some("acquitted")
    } else if t.contains("dropped") || t.contains("declined to charge") {
        Some("dropped")
    } else {
        // An appeal, a grievance or a fine appeal: not a police matter.
        None
    }
}

/// A case that closed in the last month. The courts keep no closing day:
/// a case closes on the day of its last step, which they do keep.
fn case_closed(r: &Reader, m: &Man) -> Option<Point> {
    let (c, how) =
        r.w.folk
            .courts
            .closed
            .iter()
            .rev()
            .filter(|c| c.id == m.id)
            .find_map(|c| outcome_of(c.outcome.as_deref()).map(|o| (c, o)))?;
    let words = match how {
        "convicted" => "convicted",
        "pleaded" => "a plea to a lesser charge",
        "acquitted" => "acquitted",
        _ => "the charges dropped",
    };
    Some(
        Point::new(Kind::CaseClosed)
            .on(c.next)
            .text("charge", c.what.clone())
            .text("outcome", words)
            .state("outcome", how),
    )
}

/// A suspension that is running: by the league, by his team, or by you.
fn suspended(r: &Reader, m: &Man) -> Option<Point> {
    let w = r.w;
    let p = &w.people[m.i];
    if let Some(c) = p.pro.as_ref().filter(|c| c.suspended > 0) {
        // One counter holds the league's games and a club's: the league's
        // leave a note, yours a ruling.
        let league = p.notes.iter().rev().take(6).any(|n| n.starts_with("Suspended") && n.contains("by the NFL"));
        let yours = m.mine
            && w.folk.rules.rulings.iter().rev().any(|x| {
                x.id == m.id
                    && x.date.days_to(r.today) <= 60
                    && matches!(x.response, Response::BenchGame | Response::Suspend(_))
            });
        let by = if yours && !league {
            "you"
        } else if league {
            "league"
        } else {
            "team"
        };
        let mut pt = Point::new(Kind::Suspended).count("games", c.suspended as i64).state("by", by);
        if league
            && let Some(why) = p.notes.iter().rev().find_map(|n| {
                n.strip_prefix("Suspended").and_then(|x| x.split_once(" for ")).map(|x| x.1.trim_end_matches('.'))
            })
        {
            pt = pt.text("reason", why);
        }
        return Some(pt);
    }
    // College and high school: out through a day by your ruling.
    if m.mine
        && let Some((_, through, why)) =
            w.folk.out.iter().find(|x| x.0 == m.id && x.1 >= r.today && discipline_words(&x.2))
    {
        let team = m.team?;
        let left = r.games(team).iter().filter(|g| g.0 >= r.today && g.0 <= *through).count();
        let mut pt = Point::new(Kind::Suspended).due(*through).state("by", "you");
        if left > 0 {
            pt = pt.count("games", left as i64);
        }
        if let Some((_, reason)) = why.split_once(" for ") {
            pt = pt.text("reason", reason);
        }
        return Some(pt);
    }
    // Another team's player sitting by his own coach's rules.
    if !m.mine && w.folk.ai_out.get(&m.id).is_some_and(|&d| d >= r.today) {
        return Some(Point::new(Kind::Suspended).state("by", "team"));
    }
    // Suspended for the season by his school.
    if p.stage == Stage::College && p.cal.trouble && p.on_notice {
        return Some(Point::new(Kind::Suspended).state("by", "team"));
    }
    None
}

/// The entries of `folk.out` your discipline wrote (the store also holds
/// a family leave and a rest).
fn discipline_words(why: &str) -> bool {
    ["suspended", "sat out", "deactivated"].iter().any(|k| why.starts_with(k))
}

/// A team-rule break: one waiting on your ruling, or one you ruled on in
/// the last month.
fn rule_break(r: &Reader, m: &Man) -> Option<Point> {
    if !m.mine {
        return None;
    }
    let w = r.w;
    let level = m.team?.level();
    if let Some(v) = w.folk.cases.iter().find(|v| v.id == m.id) {
        return Some(
            Point::new(Kind::RuleBreak)
                .on(v.date)
                .text("rule", v.kind.name().to_lowercase())
                .state("ruled", "no")
                .state("again", yes_no(v.prior > 0)),
        );
    }
    let mine: Vec<_> = w.folk.rules.rulings.iter().filter(|x| x.id == m.id).collect();
    let last = mine.last()?;
    Some(
        Point::new(Kind::RuleBreak)
            .on(last.date)
            .text("rule", last.kind.name().to_lowercase())
            .text("punishment", last.response.words(level))
            .state("ruled", "yes")
            .state("again", yes_no(mine.len() > 1)),
    )
}

/// A job won or lost, from your own orders on the depth chart and from
/// the bench he felt (a dated mood reason): (won, lost). Between seasons
/// an order of yours is a move on the chart (`season::chart_move`), so it
/// isn't told twice.
fn job(r: &Reader, m: &Man) -> (Option<Point>, Option<Point>) {
    if !m.mine {
        return (None, None);
    }
    let w = r.w;
    let (mut won, mut lost) = (None, None);
    if m.team.is_some_and(|t| r.in_season(t)) {
        for c in chart_changes(r, m) {
            match c {
                Chart::Up(day, beat) => {
                    let mut pt = Point::new(Kind::WonJob).on(day);
                    if let Some((id, name)) = beat {
                        pt = pt.text("rival", name).refer(Ref::Player(id));
                    }
                    won = Some(pt);
                }
                Chart::Down(day, by) => {
                    let mut pt = Point::new(Kind::LostJob).on(day).state("why", bench_cause(r, m, day));
                    if let Some((id, name)) = by {
                        pt = pt.text("rival", name).refer(Ref::Player(id));
                    }
                    lost = Some(pt);
                }
            }
        }
    }
    // The bench he felt, whoever took the job.
    if lost.is_none()
        && let Some(day) = w
            .folk
            .moods
            .iter()
            .find(|x| x.0 == m.id)
            .and_then(|x| x.1.iter().rev().find(|n| n.1 == Reason::Benched as u8 && n.2 < 0).map(|n| n.0))
    {
        let mut pt = Point::new(Kind::LostJob).on(day).state("why", bench_cause(r, m, day));
        // Who starts there now, when you set the order yourself.
        let pos = w.people[m.i].player.position;
        if let Some(&first) = w.seat_depth.iter().find(|x| x.0 == pos).and_then(|x| x.1.first())
            && first != m.id
            && let Some(name) = r.name(first)
        {
            pt = pt.text("rival", name).refer(Ref::Player(first));
        }
        lost = Some(pt);
    }
    // A job won back since stands; one lost since stands.
    match (&won, &lost) {
        (Some(a), Some(b)) if a.day >= b.day => lost = None,
        (Some(_), Some(_)) => won = None,
        _ => {}
    }
    (won, lost)
}

pub(super) enum Chart {
    /// He became the first man on a day, and whom he passed (id, name).
    Up(Date, Option<(u32, String)>),
    /// He stopped being the first man, and who is (id, name).
    Down(Date, Option<(u32, String)>),
}

/// Your orders at his position that moved him in or out of the first
/// spot, oldest first. The log keeps the first man of each order (0: the
/// order went back to your staff).
pub(super) fn chart_changes(r: &Reader, m: &Man) -> Vec<Chart> {
    let w = r.w;
    let pos = w.people[m.i].player.position;
    let mut out = Vec::new();
    let mut prev: Option<u32> = None;
    for &(day, p, first) in &w.seat_depth_log {
        if p != pos {
            continue;
        }
        let named = |id: u32| r.name(id).map(|n| (id, n));
        if first == m.id && prev != Some(m.id) {
            out.push(Chart::Up(day, prev.filter(|&x| x != 0).and_then(named)));
        } else if prev == Some(m.id) && first != m.id && first != 0 {
            out.push(Chart::Down(day, named(first)));
        }
        prev = Some(first);
    }
    out
}

/// Why he sat: your discipline, an injury, or his play.
fn bench_cause(r: &Reader, m: &Man, day: Date) -> &'static str {
    let w = r.w;
    let near = |d: Date| (d.days_to(day)).abs() <= 3;
    if w.folk.benched.iter().any(|x| x.0 == m.id)
        || w.folk.rules.rulings.iter().any(|x| {
            x.id == m.id
                && near(x.date)
                && matches!(x.response, Response::BenchStart | Response::BenchGame | Response::Suspend(_))
        })
    {
        "discipline"
    } else if w.training.hurt.iter().any(|h| h.id == m.id && h.from <= day && h.from.days_to(day) <= 21) {
        "injury"
    } else {
        "play"
    }
}

/// Third or lower on the chart your team would take into a game today,
/// after two seasons in the program.
fn buried(r: &Reader, m: &Man) -> Option<Point> {
    if !m.mine {
        return None;
    }
    let w = r.w;
    let p = &w.people[m.i];
    let seasons = match p.stage {
        Stage::Pro => p.pro.as_ref().map_or(0, |c| c.seasons.len() as u32),
        Stage::College => p.college_seasons,
        _ => (w.year - p.class_of + 12).clamp(9, 12) as u32 - 9,
    };
    if seasons < 2 || p.player.position.is_specialist() {
        return None;
    }
    let list = &r.depth()?.iter().find(|x| x.0 == p.player.position)?.1;
    let rank = list.iter().position(|&x| x == m.id)?;
    if rank < 2 {
        return None;
    }
    let ahead = crate::world::folk::person(w, list[0])?;
    Some(
        Point::new(Kind::Buried)
            .text("ahead", w.people[ahead].name.clone())
            .text("ahead_class", crate::world::folk::facts::class_words(w, ahead))
            .count("rank", rank as i64 + 1)
            .refer(Ref::Player(list[0])),
    )
}

/// His last game and the run before it, from your team's games kept
/// whole: (a big game, a bad game, a slump). A game is named for a week.
fn games(r: &Reader, m: &Man) -> (Option<Point>, Option<Point>, Option<Point>) {
    let none = (None, None, None);
    if !m.mine {
        return none;
    }
    let Some(team) = m.team else { return none };
    let w = r.w;
    let pos = w.people[m.i].player.position;
    // His graded games, newest first: (day, side, line).
    let mut mine = Vec::new();
    for g in w.seat_games.iter().rev() {
        let Some(side) = super::side_of(g, team) else { continue };
        if let Some(line) = g.stats.players[side].get(&m.id) {
            mine.push((g, side, line));
        }
    }
    let Some(&(g, side, line)) = mine.first() else { return none };
    if !super::sheet::in_window(Kind::BigGame, g.date, r.today) {
        return none;
    }
    let (us, them) = (g.stats.score[side], g.stats.score[1 - side]);
    let result = if us > them { "win" } else { "loss" };
    let base = |k: Kind| {
        let opp = match team {
            Team::College(_) => Team::College(g.teams[1 - side]),
            Team::Nfl(_) => Team::Nfl(g.teams[1 - side]),
            Team::High(_) => Team::High(g.teams[1 - side]),
            Team::Other(_) => Team::Other(g.teams[1 - side]),
        };
        Point::new(k)
            .on(g.date)
            .text("opp", g.names[1 - side].clone())
            .text("score", format!("{us}-{them}"))
            .state("result", result)
            .refer(Ref::Team(opp))
    };
    let graded = line.graded >= GRADED_MIN;
    let grade = line.grade();
    let (mut big, mut bad, mut slump) = (None, None, None);
    let slip = words::mistake(line);
    if graded && grade >= BIG_GRADE && words::mistakes(line) == 0 {
        let mut pt = base(Kind::BigGame);
        if let Some(l) = words::stat_line(pos, line) {
            pt = pt.text("line", l);
        }
        big = Some(pt);
    } else if words::mistakes(line) >= 2 || (words::mistakes(line) >= 1 && us < them) || (graded && grade < BAD_GRADE) {
        let mut pt = base(Kind::BadGame);
        if let Some(s) = slip {
            pt = pt.text("mistake", s);
        }
        bad = Some(pt);
    }
    // A slump: his newest games in a row, each well under his average in
    // the games before them.
    let grades: Vec<f64> = mine.iter().filter(|x| x.2.graded >= GRADED_MIN).map(|x| x.2.grade()).collect();
    if graded && grades.len() >= SLUMP_GAMES + 3 {
        let run = (SLUMP_GAMES..=grades.len() - 3)
            .rev()
            .find(|&n| {
                let level = grades[n..].iter().sum::<f64>() / (grades.len() - n) as f64;
                grades[..n].iter().all(|&x| x < level - SLUMP_UNDER)
            })
            .unwrap_or(0);
        if run >= SLUMP_GAMES {
            slump = Some(Point::new(Kind::Slump).on(g.date).count("games", run as i64));
        }
    }
    (big, bad, slump)
}

/// An award in the last month. The record keeps the season, so the day
/// is the day that award is given on this year's calendar.
fn award(r: &Reader, m: &Man) -> Option<Point> {
    let w = r.w;
    let season = w.dates.season;
    let named = |name: &str, day: Date| {
        (day <= r.today).then(|| Point::new(Kind::Award).on(day).text("award", format!("the {name}")))
    };
    if w.people[m.i].stage == Stage::Pro {
        let a = w.nfl_book.awards.iter().rev().find(|a| a.2 == m.id && (a.0 == season || a.0 == season - 1))?;
        // Honors night is the Thursday before the Super Bowl; the year
        // turns over the day after it.
        let day = if a.0 == season { w.dates.nfl_honors } else { w.dates.start.add(-4) };
        return named(a.1, day);
    }
    let h = w.stats.honors.get(&m.id)?;
    if let Some(&(_, name)) = h.awards.iter().rev().find(|a| a.0 == season) {
        let day = if name.contains("Heisman") { w.dates.heisman } else { w.dates.awards_show };
        return named(name, day);
    }
    if h.all_america.iter().any(|a| a.0 == season) && w.dates.all_america <= r.today {
        return Some(Point::new(Kind::Award).on(w.dates.all_america).text("award", "first-team All-America"));
    }
    None
}

/// Out hurt now.
fn hurt(r: &Reader, m: &Man) -> Option<Point> {
    let w = r.w;
    let weeks = |until: Date| ((r.today.days_to(until) + 6) / 7).max(1) as i64;
    if let Some(h) = w.training.hurt_on(m.id, r.today).filter(|h| h.from <= r.today) {
        let mut pt = Point::new(Kind::Hurt).on(h.from).due(h.until).count("weeks", weeks(h.until));
        if let Some(k) = h.kind {
            pt = pt.text("injury", k.name());
        }
        return Some(pt);
    }
    if let Some((backs, kind)) = w.training.other_hurt(m.id, r.today) {
        let mut pt = Point::new(Kind::Hurt).due(backs).count("weeks", weeks(backs));
        if let Some(k) = kind {
            pt = pt.text("injury", k.name());
        }
        return Some(pt);
    }
    let c = w.people[m.i].pro.as_ref().filter(|c| c.out_weeks > 0)?;
    Some(Point::new(Kind::Hurt).count("weeks", c.out_weeks as i64))
}

/// Back from an injury in the last two weeks (your players: the report
/// keeps a healed man for sixty days).
fn back(r: &Reader, m: &Man) -> Option<Point> {
    if !m.mine || r.w.training.hurt_on(m.id, r.today).is_some() {
        return None;
    }
    let h = r.w.training.hurt.iter().filter(|h| h.id == m.id && h.until <= r.today).max_by_key(|h| h.until)?;
    let mut pt = Point::new(Kind::Back).on(h.until);
    if let Some(k) = h.kind {
        pt = pt.text("injury", k.name());
    }
    Some(pt)
}

/// Life away from football: the seven events, each known to you only if
/// he told you.
fn life_events(r: &Reader, m: &Man) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::new();
    for e in r.w.folk.life.events.iter().rev().filter(|e| e.id == m.id) {
        let (kind, bad) = match e.event {
            life::Event::Birth => (Kind::Birth, false),
            life::Event::Wedding => (Kind::Wedding, false),
            life::Event::ParentIll => (Kind::SickParent, true),
            life::Event::Death => (Kind::Death, true),
            life::Event::Divorce => (Kind::Divorce, true),
            life::Event::MoneyTrouble => (Kind::MoneyLost, true),
            life::Event::Homesick => (Kind::Homesick, true),
            life::Event::Community | life::Event::Graduation => continue,
        };
        let age = e.date.days_to(r.today);
        if age < 0 || age >= if bad { LIFE_BAD_DAYS } else { LIFE_GOOD_DAYS } || out.iter().any(|p| p.kind == kind) {
            continue;
        }
        // The day is kept off the point's window (life has none): it
        // weighs less as the weeks pass.
        let left = 1.0 - 0.5 * age as f32 / if bad { LIFE_BAD_DAYS } else { LIFE_GOOD_DAYS } as f32;
        out.push(
            Point::new(kind)
                .on(e.date)
                .text("life", e.event.words())
                .state("known", yes_no(e.told))
                .known_if(e.told)
                .weigh(left),
        );
    }
    out
}

/// Your promises to him: one that comes due within two months, one that
/// broke in the last four.
fn promises(r: &Reader, m: &Man) -> Vec<Point> {
    let mut out = Vec::new();
    if !m.mine {
        return out;
    }
    for p in r.w.folk.promises.iter().filter(|p| p.to == PromiseTo::Player(m.id)) {
        match p.kept {
            None => {
                let left = r.today.days_to(p.due);
                if (0..=kinds::AHEAD_DAYS).contains(&left) {
                    out.push(
                        Point::new(Kind::PromiseOpen)
                            .due(p.due)
                            .text("promise", p.kind.words())
                            .text("when", words::when_words(p.due, r.today))
                            .count("days", left as i64),
                    );
                }
            }
            // The promise broke on the day it came due.
            Some(false) => out.push(Point::new(Kind::PromiseBroken).on(p.due).text("promise", p.kind.words())),
            Some(true) => {}
        }
    }
    out
}

/// Paid under what he is worth: a pro who outplayed his deal, or your
/// college player under the going rate for his spot and role. No amount
/// is ever a value: the gap is a check.
fn pay(r: &Reader, m: &Man) -> Option<Point> {
    let w = r.w;
    let p = &w.people[m.i];
    let gap = match p.stage {
        Stage::Pro => {
            let (apy, worth) = crate::world::people::pay_and_worth(w, m.i)?;
            let over = worth / apy.max(1.0);
            if apy <= 0.0 || over < PAY_SMALL {
                return None;
            }
            if over >= PAY_BIG { "big" } else { "small" }
        }
        Stage::College if m.mine => {
            let fair = r.going_rate(m.id)?;
            let paid = r.deal(m.id);
            if fair < NIL_FLOOR || paid >= NIL_SMALL * fair {
                return None;
            }
            if paid < NIL_BIG * fair { "big" } else { "small" }
        }
        _ => return None,
    };
    Some(Point::new(Kind::Pay).state("gap", gap))
}

/// A pro holding in, holding out, or asking for a trade.
fn wants_out(r: &Reader, m: &Man) -> Option<Point> {
    let c = r.w.people[m.i].pro.as_ref()?;
    let hold = match (c.hold, c.wants_out) {
        (Hold::Out, _) => "out",
        (Hold::In, _) => "in",
        (Hold::None, true) => "trade",
        (Hold::None, false) => return None,
    };
    Some(Point::new(Kind::WantsOut).state("hold", hold).state("trade", yes_no(c.wants_out)))
}

/// The portal on his mind. His lean is his own: you know it only once he
/// has asked to see you, or a whisper is out. The reason is the one he
/// gives when he asks.
fn portal(r: &Reader, m: &Man) -> Option<Point> {
    let w = r.w;
    let p = &w.people[m.i];
    if p.stage != Stage::College {
        return None;
    }
    let asked = crate::world::recruiting::intent_reason(w, m.id);
    // hidden: his lean is read as a gate and as how much it is on his
    // mind, never as a value.
    let lean = p.cal.lean;
    if asked.is_none() && lean <= PORTAL_LEAN {
        return None;
    }
    let mut pt = Point::new(Kind::Portal)
        .state("asked", yes_no(asked.is_some()))
        .known_if(asked.is_some() || p.cal.whispered)
        .weigh((0.6 + lean as f32).min(1.25));
    if let Some(why) = asked {
        pt = pt.state("why", why);
    }
    Some(pt)
}

/// A college player three years out of high school with a draft read:
/// the round as early, middle, late or undrafted, never the grade.
fn draft(r: &Reader, m: &Man) -> Option<Point> {
    let w = r.w;
    let p = &w.people[m.i];
    if p.stage != Stage::College || w.year - p.class_of < 3 {
        return None;
    }
    // hidden: the league's read gives a word for the round, never its
    // grade.
    let read = p.cal.draft_read?;
    let round = match read.round {
        1 | 2 => "early",
        3 | 4 => "middle",
        5..=7 => "late",
        _ => "undrafted",
    };
    let mut pt = Point::new(Kind::Draft).state("round", round);
    if p.cal.declare_plan == Some(true) {
        pt = pt.hide("plans", "yes");
    }
    Some(pt)
}

/// A close friend who left your team in the last four weeks (as
/// `talks::friend_gone`).
fn friend_left(r: &Reader, m: &Man) -> Option<Point> {
    if !m.mine {
        return None;
    }
    let w = r.w;
    let gone = talks::friend_gone(w, m.id)?;
    let day = w.folk.ties.gone.iter().filter(|g| g.0 == m.id && g.1 == gone).map(|g| g.3).max()?;
    let name = r.name(gone)?;
    Some(Point::new(Kind::FriendLeft).on(day).text("gone", words::first(&name)).refer(Ref::Player(gone)))
}

/// A new position coach: the man in his position's seat is in his first
/// season in the job and isn't who had him last season. A coach's hiring
/// has a season, not a day, so the point lasts until that first season's
/// opener.
fn new_coach(r: &Reader, m: &Man) -> Option<Point> {
    let w = r.w;
    let p = &w.people[m.i];
    let Some(Team::College(prog)) = m.team else { return None };
    let g = crate::world::coach::Group::of(p.player.position);
    let c = w.programs[prog].position_coaches[g as usize]?;
    let job = w.coaches[c].job?;
    let last = p.college_staff.last().filter(|x| x.1 == prog)?;
    let season = w.dates.season;
    if last.3 == Some(c) || job.first != season || r.today > w.dates.cfb_weeks.first().copied()? {
        return None;
    }
    Some(Point::new(Kind::NewCoach).text("pc", format!("Coach {}", last_name(&w.coaches[c].name))).refer(Ref::Coach(c)))
}

pub(super) fn last_name(full: &str) -> String {
    let mut parts: Vec<&str> = full.split_whitespace().collect();
    while parts.len() > 1 && matches!(parts.last(), Some(&("Jr." | "Sr." | "II" | "III" | "IV"))) {
        parts.pop();
    }
    parts.last().copied().unwrap_or(full).to_string()
}

/// His grades against the eligibility line. The grade itself is never a
/// value: only which side of the line he is on.
fn grades(r: &Reader, m: &Man) -> Option<Point> {
    let w = r.w;
    let p = &w.people[m.i];
    if !m.mine || !matches!(p.stage, Stage::College | Stage::HighSchool) {
        return None;
    }
    // hidden: read as a gate only.
    let gpa = p.bg.gpa;
    let line = if gpa < GRADE_LINE {
        "under"
    } else if gpa < GRADE_LINE + GRADE_NEAR {
        "near"
    } else {
        return None;
    };
    Some(Point::new(Kind::Grades).state("line", line))
}

/// His parent called you in the last two weeks.
fn parent_called(r: &Reader, m: &Man) -> Option<Point> {
    if !m.mine {
        return None;
    }
    let w = r.w;
    let s = w
        .folk
        .scenes
        .iter()
        .rev()
        .find(|s| s.kind == scene::Kind::ParentCall && s.a == m.id as u64 && s.date <= r.today)?;
    let who = talks::parent_word(w, m.i);
    let mut c = who.chars();
    let lower = c.next().map_or(String::new(), |f| f.to_lowercase().collect::<String>() + c.as_str());
    Some(Point::new(Kind::ParentCalled).on(s.date).text("parent", lower))
}
