//! G5, the Hall of Fame race: every coach's case against the halls' own
//! rules, every season. The world's halls vote for everyone, in the history
//! too: each state's coaches' hall on its published line (the states whose
//! line we found are in `data/legacy/coach_halls.csv`; the rest run on a
//! proposed line the screens mark as one), the national high school hall,
//! and the Pro Football Hall's coaches (one coach finalist a year, after a
//! wait of five seasons, one from the 2025 class on). The College Football
//! Hall's vote is in `records.rs`. Your own case shows as a race: the line,
//! your numbers against it, your place on the all-time list, what would
//! move it, and the real greats from before the world's history.

use std::sync::OnceLock;

use super::{Bank, key, memo, pay};
use crate::calendar::{Date, Level};
use crate::world::World;
use crate::world::coach::{Coach, Employer, Role};
use crate::world::stories::TeamKey;
use crate::world::storylines::catalogue::Rating;

/// One coach voted into a hall.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Inducted {
    pub coach: u32,
    pub name: String,
    /// A state's code ("TX"), "US" for the national high school hall, "PRO".
    pub hall: String,
    pub year: i32,
    /// His case in one line ("31 seasons, 287-94, 4 state titles").
    pub line: String,
}

/// A state coaches' hall's line.
#[derive(Clone, Debug)]
pub struct Rule {
    pub state: String,
    pub hall: String,
    pub years: u32,
    pub in_state: u32,
    pub head_years: u32,
    pub wins: u32,
    pub retired: String,
    pub line: String,
    pub source: String,
    /// False: the proposed line for a state whose own we didn't find.
    pub researched: bool,
}

fn csv_fields(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

fn rules() -> &'static [Rule] {
    static R: OnceLock<Vec<Rule>> = OnceLock::new();
    R.get_or_init(|| {
        include_str!("../../../data/legacy/coach_halls.csv")
            .lines()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let f = csv_fields(l);
                let n = |k: usize| f.get(k).and_then(|x| x.trim().parse().ok()).unwrap_or(0);
                let s = |k: usize| f.get(k).cloned().unwrap_or_default();
                Rule {
                    state: s(0),
                    hall: s(1),
                    years: n(2),
                    in_state: n(3),
                    head_years: n(4),
                    wins: n(5),
                    retired: s(6),
                    line: s(7),
                    source: s(8),
                    researched: true,
                }
            })
            .collect()
    })
}

/// The real greats from before the world's history, for one level ("hs",
/// "college", "nfl"): (name, line).
pub fn greats(level: &str) -> Vec<(String, String)> {
    static G: OnceLock<Vec<(String, String, String)>> = OnceLock::new();
    G.get_or_init(|| {
        include_str!("../../../data/legacy/greats.csv")
            .lines()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                let f = csv_fields(l);
                (f[0].clone(), f.get(1).cloned().unwrap_or_default(), f.get(2).cloned().unwrap_or_default())
            })
            .collect()
    })
    .iter()
    .filter(|g| g.0 == level)
    .map(|g| (g.1.clone(), g.2.clone()))
    .collect()
}

/// A state's coaches' hall: its published line, or the proposed one.
pub fn rule_for(code: &str) -> Rule {
    if let Some(r) = rules().iter().find(|r| r.state == code) {
        return r.clone();
    }
    let name = crate::world::folk::facts::state_name(code);
    Rule {
        state: code.to_string(),
        hall: format!("{name} Coaches' Hall of Fame"),
        years: 25,
        in_state: 15,
        head_years: 15,
        wins: 150,
        retired: "retired_or_60".into(),
        line: "25 seasons coaching, 15 as head coach in the state, and 150 wins (proposed)".into(),
        source: String::new(),
        researched: false,
    }
}

/// A coach's high school record, in one state.
#[derive(Clone, Copy, Debug, Default)]
pub struct HsRecord {
    pub years: u32,
    pub in_state: u32,
    pub head_years: u32,
    pub wins: u32,
    pub losses: u32,
    pub titles: u32,
    /// The last season he coached high school football.
    pub last: i32,
    pub active: bool,
}

/// Seasons in a stint (a stint that ended the year it began counts one).
fn span(from: i32, to: i32) -> u32 {
    (to - from).max(1) as u32
}

/// His high school seasons, all of them and in one state.
pub fn hs_record(w: &World, c: &Coach, state: usize) -> HsRecord {
    let mut r = HsRecord::default();
    let mut add = |s: usize, role: Role, n: u32, wl: (u32, u32, u32), to: i32| {
        r.years += n;
        let here = w.high_schools.get(s).is_some_and(|h| h.state == state);
        if here {
            r.in_state += n;
        }
        if role == Role::HeadCoach && here {
            r.head_years += n;
            r.wins += wl.0;
            r.losses += wl.1;
            r.titles += wl.2;
        }
        r.last = r.last.max(to);
    };
    for s in &c.history {
        if let Employer::HighSchool(h) = s.employer {
            add(h, s.role, span(s.from, s.to), (s.wins, s.losses, s.titles), s.to);
        }
    }
    if let Some(j) = c.job
        && let Employer::HighSchool(h) = j.employer
    {
        add(h, j.role, span(j.since, w.year + 1), (c.wins, c.losses, c.titles), w.year);
        r.active = true;
    }
    r
}

/// The state he coached most of his high school seasons in.
fn hs_state(w: &World, c: &Coach) -> Option<usize> {
    let mut by: Vec<(usize, u32)> = Vec::new();
    let mut add = |h: usize, n: u32| {
        let Some(st) = w.high_schools.get(h).map(|x| x.state) else { return };
        match by.iter_mut().find(|x| x.0 == st) {
            Some(x) => x.1 += n,
            None => by.push((st, n)),
        }
    };
    for s in &c.history {
        if let Employer::HighSchool(h) = s.employer {
            add(h, span(s.from, s.to));
        }
    }
    if let Some(j) = c.job
        && let Employer::HighSchool(h) = j.employer
    {
        add(h, span(j.since, w.year + 1));
    }
    by.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))).map(|x| x.0)
}

/// Whether a coach meets a state hall's line now.
pub fn meets(r: &Rule, rec: &HsRecord, age: u32) -> bool {
    let retired = !rec.active;
    let time_ok = match r.retired.as_str() {
        "retired" => retired,
        "retired_or_50" => retired || age >= 50,
        "retired_or_60" => retired || age >= 60,
        "retired_or_30" => retired || rec.years >= 30,
        "retired_15_active_30" => rec.in_state >= if retired { 15 } else { 30 },
        _ => true,
    };
    if !time_ok {
        return false;
    }
    // Michigan's two lines are either-or.
    if r.state == "MI" {
        return rec.wins >= r.wins || rec.in_state >= r.years;
    }
    // California counts head coach games, not wins.
    let wins_ok = if r.state == "CA" { rec.wins + rec.losses >= 100 } else { rec.wins >= r.wins };
    rec.years >= r.years && rec.in_state >= r.in_state && rec.head_years >= r.head_years && wins_ok
}

/// A high school case's weight with the voters: wins, titles, winning
/// share and years (proposed, not confirmed).
fn hs_score(rec: &HsRecord) -> f64 {
    let g = (rec.wins + rec.losses).max(1) as f64;
    rec.wins as f64 / 25.0 + 1.5 * rec.titles as f64 + 8.0 * (rec.wins as f64 / g - 0.5) + rec.years as f64 / 10.0
}

/// How many a state's hall takes a year: by its size, the published counts
/// where there are any (New Jersey up to four).
fn quota(w: &World, state: usize) -> usize {
    let code = w.states[state].code.as_str();
    let schools = w.high_schools.iter().filter(|h| h.state == state).count();
    let n = ((schools as f64) / 180.0).round().clamp(1.0, 6.0) as usize;
    if code == "NJ" { n.min(4) } else { n }
}

fn inducted(w: &World, c: usize, hall: &str) -> bool {
    let list = if hall == "PRO" { &w.legacy.pro_hall } else { &w.legacy.halls };
    list.iter().any(|x| x.coach == c as u32 && x.hall == hall)
}

/// The days the halls vote: the state halls on May 1, the national high
/// school hall in late June, the Pro Football Hall's class the Thursday
/// before the Super Bowl.
pub(crate) fn world_day(w: &mut World, d: Date) {
    if d == Date::ymd(w.year, 5, 1) {
        state_halls(w);
    } else if d == Date::ymd(w.year, 6, 28) {
        national_hall(w);
    } else if d == w.dates.super_bowl.add(-3) {
        pro_hall(w);
    }
}

fn line_of(rec: &HsRecord) -> String {
    let mut s = format!("{}, {}-{}", crate::world::news::count(rec.years, "season"), rec.wins, rec.losses);
    if rec.titles > 0 {
        s.push_str(&format!(", {}", crate::world::news::count(rec.titles, "state title")));
    }
    s
}

fn state_halls(w: &mut World) {
    let year = w.year;
    let mut by_state: Vec<Vec<(f64, usize, String)>> = vec![Vec::new(); w.states.len()];
    for (c, coach) in w.coaches.iter().enumerate() {
        let Some(st) = hs_state(w, coach) else { continue };
        let code = w.states[st].code.clone();
        if inducted(w, c, &code) {
            continue;
        }
        let rec = hs_record(w, coach, st);
        if rec.head_years == 0 && rec.years < 10 {
            continue;
        }
        let rule = rule_for(&code);
        if !meets(&rule, &rec, coach.age) {
            continue;
        }
        by_state[st].push((hs_score(&rec), c, line_of(&rec)));
    }
    for (st, mut v) in by_state.into_iter().enumerate() {
        if v.is_empty() {
            continue;
        }
        v.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        let code = w.states[st].code.clone();
        for (_, c, line) in v.into_iter().take(quota(w, st)) {
            let name = w.coaches[c].name.clone();
            w.legacy.halls.push(Inducted { coach: c as u32, name, hall: code.clone(), year, line });
            yours_in(w, c, &rule_for(&code).hall);
        }
    }
}

fn national_hall(w: &mut World) {
    let year = w.year;
    let mut best: Vec<(f64, usize, String)> = Vec::new();
    for (c, coach) in w.coaches.iter().enumerate() {
        if !coach.retired || inducted(w, c, "US") {
            continue;
        }
        let Some(st) = hs_state(w, coach) else { continue };
        let rec = hs_record(w, coach, st);
        if rec.head_years < 25 || (rec.wins < 250 && rec.titles < 5) {
            continue;
        }
        best.push((hs_score(&rec), c, line_of(&rec)));
    }
    best.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, c, line) in best.into_iter().take(2) {
        let name = w.coaches[c].name.clone();
        w.legacy.halls.push(Inducted { coach: c as u32, name, hall: "US".into(), year, line });
        yours_in(w, c, "the National High School Hall of Fame");
    }
}

/// A coach's NFL head coaching record: (seasons, wins, losses, titles, last).
pub fn nfl_record(w: &World, c: &Coach) -> (u32, u32, u32, u32, i32) {
    let mut r = (0, 0, 0, 0, 0);
    for s in &c.history {
        if matches!(s.employer, Employer::Nfl(_)) && s.role == Role::HeadCoach {
            r.0 += span(s.from, s.to);
            r.1 += s.wins;
            r.2 += s.losses;
            r.3 += s.titles;
            r.4 = r.4.max(s.to);
        }
    }
    if let Some(j) = c.job
        && matches!(j.employer, Employer::Nfl(_))
        && j.role == Role::HeadCoach
    {
        r.0 += span(j.since, w.year + 1);
        r.1 += c.wins;
        r.2 += c.losses;
        r.3 += c.titles;
        r.4 = w.year;
    }
    r
}

/// The Pro Football Hall's read of a coach: wins, titles and winning share
/// (proposed, not confirmed). In at 9 and over.
fn pro_score(wins: u32, losses: u32, titles: u32) -> f64 {
    let g = (wins + losses).max(1) as f64;
    wins as f64 / 25.0 + 3.0 * titles as f64 + 6.0 * (wins as f64 / g - 0.5)
}

const PRO_BAR: f64 = 9.0;

fn pro_hall(w: &mut World) {
    let class = w.year + 1;
    // The wait: five seasons, one from the 2025 class on (the Hall's 2024
    // change).
    let wait = if class >= 2025 { 1 } else { 5 };
    let mut best: Option<(f64, usize, String)> = None;
    for (c, coach) in w.coaches.iter().enumerate() {
        if !coach.retired || inducted(w, c, "PRO") {
            continue;
        }
        let (n, wins, losses, titles, last) = nfl_record(w, coach);
        if n < 8 || last > w.year - wait {
            continue;
        }
        let s = pro_score(wins, losses, titles);
        if best.as_ref().is_none_or(|b| s > b.0) {
            let line = format!(
                "{wins}-{losses} in {n} seasons{}",
                if titles > 0 { format!(", {}", crate::world::news::count(titles, "title")) } else { String::new() }
            );
            best = Some((s, c, line));
        }
    }
    // One coach finalist a class; the voters take him at the bar.
    if let Some((s, c, line)) = best
        && s >= PRO_BAR
    {
        let name = w.coaches[c].name.clone();
        w.legacy.pro_hall.push(Inducted { coach: c as u32, name: name.clone(), hall: "PRO".into(), year: class, line });
        let text = format!("The Pro Football Hall of Fame elects {name} in its class of {class}, as a coach.");
        let e = crate::calendar::Event::new(
            w.today,
            Level::Nfl,
            crate::calendar::Kind::Award,
            crate::calendar::Loudness::Medium,
            text,
        )
        .link(crate::calendar::Link::Coach(c as u32), &name);
        w.calendar.post(e);
        yours_in(w, c, "the Pro Football Hall of Fame");
    }
}

/// Your induction: the last chapter.
fn yours_in(w: &mut World, c: usize, hall: &str) {
    if crate::world::stories::seat_coach(w) != Some(c) {
        return;
    }
    let text = format!("{hall}, class of {}", w.year);
    crate::world::stories::story::memento(w, crate::world::stories::story::MementoKind::Plaque, text);
    memo(
        w,
        "legacy-hall",
        format!("You're going into {hall}"),
        vec![format!("The vote came in today: {hall} takes you in its class of {}.", w.year)],
    );
    pay(w, key("G5", c as u64, 0x4A11), "G5", format!("Inducted into {hall}"), 1.0, Bank::Rating(Rating::Network), 3.0);
}

/// A bar of the race: what it reads, what you have and what the line needs
/// (0 for no line).
#[derive(Clone, Debug, serde::Serialize)]
pub struct Bar {
    pub label: String,
    pub have: f64,
    pub need: f64,
    pub text: String,
}

/// Your case at a level, as the board shows it.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Case {
    pub hall: String,
    /// How the line was set ("On the hall's published line").
    pub note: String,
    pub words: &'static str,
    pub bars: Vec<Bar>,
    pub moves: String,
    pub rank_label: String,
    pub rank: u32,
    pub greats: Vec<(String, String)>,
    pub inducted: bool,
}

fn bar(label: &str, have: f64, need: f64) -> Bar {
    let text = if need > 0.0 {
        if need < 1.0 {
            format!("{} / {}", pct3(have), pct3(need))
        } else {
            format!("{} / {}", have.round(), need.round())
        }
    } else {
        format!("{}", have.round())
    };
    Bar { label: label.to_string(), have, need, text }
}

/// Your college head coaching record: (seasons, wins, losses, titles).
pub fn college_record(w: &World, c: &Coach) -> (u32, u32, u32, u32) {
    let mut r = (0, 0, 0, 0);
    for s in &c.history {
        if matches!(s.employer, Employer::College(_)) && s.role == Role::HeadCoach {
            r.0 += span(s.from, s.to);
            r.1 += s.wins;
            r.2 += s.losses;
            r.3 += s.titles;
        }
    }
    if let Some(j) = c.job
        && matches!(j.employer, Employer::College(_))
        && j.role == Role::HeadCoach
    {
        r.0 += span(j.since, w.year + 1);
        r.1 += c.wins;
        r.2 += c.losses;
        r.3 += c.titles;
    }
    r
}

/// Your place on a level's all-time list of head coach wins, among every
/// coach in the world's history.
fn rank_at(w: &World, me: usize, wins_of: impl Fn(&Coach) -> u32) -> u32 {
    let mine = wins_of(&w.coaches[me]);
    1 + w.coaches.iter().enumerate().filter(|&(c, x)| c != me && wins_of(x) > mine).count() as u32
}

/// The words for a case: how much of the line is in.
fn case_words(share: f64, inducted: bool) -> &'static str {
    if inducted {
        "Inducted"
    } else if share >= 1.0 {
        "A lock"
    } else if share >= 0.75 {
        "Strong"
    } else if share >= 0.5 {
        "Borderline"
    } else {
        "Not yet"
    }
}

/// Your case at a level (high school, college or the NFL).
pub fn case(w: &World, level: Level) -> Option<Case> {
    let (me, team) = super::you(w)?;
    let c = &w.coaches[me];
    let per_year = |done: f64, seasons: u32| done / seasons.max(1) as f64;
    match level {
        Level::HighSchool => {
            let st = match team {
                TeamKey::Hs(h) => w.high_schools[h as usize].state,
                _ => hs_state(w, c)?,
            };
            let code = w.states[st].code.clone();
            let rule = rule_for(&code);
            let rec = hs_record(w, c, st);
            let mut bars = vec![bar("Seasons coaching", rec.years as f64, rule.years.max(rule.in_state) as f64)];
            if rule.state == "CA" {
                bars.push(bar("Head coach games", (rec.wins + rec.losses) as f64, 100.0));
            } else {
                bars.push(bar("Head coach wins", rec.wins as f64, rule.wins as f64));
            }
            bars.push(bar("State titles", rec.titles as f64, 0.0));
            let share = bars.iter().filter(|b| b.need > 0.0).map(|b| (b.have / b.need).min(1.0)).fold(1.0, f64::min);
            let won = inducted(w, me, &code);
            let mut moves = Vec::new();
            if rec.titles == 0 {
                moves.push("a state title".to_string());
            }
            let short = rule.years.max(rule.in_state).saturating_sub(rec.years);
            if short > 0 {
                moves.push(more_seasons(short as i32, ""));
            } else if rule.wins > rec.wins {
                let left = rule.wins - rec.wins;
                let rate = per_year(rec.wins as f64, rec.head_years);
                let yrs = if rate > 0.5 { (left as f64 / rate).ceil() as i32 } else { 0 };
                moves.push(if yrs > 0 {
                    format!("{left} more wins, about {} at this rate", super::seasons_words(yrs))
                } else {
                    format!("{left} more wins")
                });
            }
            let note = if rule.researched {
                "On the hall's published line".to_string()
            } else {
                "On a proposed line: this state's own wasn't found".to_string()
            };
            let name = crate::world::folk::facts::state_name(&code);
            Some(Case {
                hall: rule.hall.clone(),
                note,
                words: case_words(share, won),
                bars,
                moves: if moves.is_empty() {
                    "Nothing: the line is met; the vote is the committee's.".into()
                } else {
                    format!("What moves it: {}.", super::and_list(&moves))
                },
                rank_label: format!("All-time {name} high school wins"),
                rank: rank_at(w, me, |x| if hs_state(w, x) == Some(st) { hs_record(w, x, st).wins } else { 0 }),
                greats: greats("hs"),
                inducted: won,
            })
        }
        Level::College => {
            let (n, wins, losses, titles) = college_record(w, c);
            let g = (wins + losses) as f64;
            let line = if w.year + 1 >= 2027 { 0.595 } else { 0.600 };
            let pct = if g > 0.0 { wins as f64 / g } else { 0.0 };
            let bars = vec![
                bar("Seasons as head coach", n as f64, 10.0),
                bar("Games as head coach", g, 100.0),
                bar("Winning share", pct, line),
            ];
            let share = bars.iter().map(|b| (b.have / b.need).min(1.0)).fold(1.0, f64::min);
            let won = w.stats.hall.iter().any(|e| e.coach && e.who == Some(me as u32));
            let mut moves = Vec::new();
            if n < 10 {
                moves.push(more_seasons(10 - n as i32, " as a head coach"));
            }
            if g < 100.0 {
                moves.push(crate::world::news::count(100 - g as u32, "more game"));
            }
            if pct < line && g > 0.0 {
                let need = ((line * g - wins as f64) / (1.0 - line)).ceil().max(1.0);
                moves.push(format!(
                    "{} in a row to reach {}",
                    crate::world::news::count(need as u32, "win"),
                    pct3(line)
                ));
            }
            if titles == 0 {
                moves.push("a conference title".into());
            }
            Some(Case {
                hall: "College Football Hall of Fame".into(),
                note: format!("10 seasons, 100 games, {}; three seasons after you retire", pct3(line)),
                words: case_words(share, won),
                bars,
                moves: format!("What moves it: {}.", super::and_list(&moves)),
                rank_label: "All-time college wins".into(),
                rank: rank_at(w, me, |x| college_record(w, x).1),
                greats: greats("college"),
                inducted: won,
            })
        }
        Level::Nfl => {
            let (n, wins, losses, titles, _) = nfl_record(w, c);
            let s = pro_score(wins, losses, titles);
            let bars = vec![
                bar("Head coach wins", wins as f64, 150.0),
                bar("Titles", titles as f64, 2.0),
                bar("Seasons as head coach", n as f64, 8.0),
            ];
            let won = inducted(w, me, "PRO");
            let mut moves = Vec::new();
            if titles == 0 {
                moves.push("a Super Bowl".to_string());
            }
            if wins < 150 {
                moves.push(format!("{} more wins", 150 - wins));
            }
            Some(Case {
                hall: "Pro Football Hall of Fame".into(),
                note: "One coach finalist a year; the voters' line is proposed".into(),
                words: case_words(s / PRO_BAR, won),
                bars,
                moves: format!("What moves it: {}.", super::and_list(&moves)),
                rank_label: "All-time NFL wins".into(),
                rank: rank_at(w, me, |x| nfl_record(w, x).1),
                greats: greats("nfl"),
                inducted: won,
            })
        }
        _ => None,
    }
}

/// After your season: the wins milestones and passing names on your
/// level's list.
pub(crate) fn season(w: &mut World) {
    let Some((me, team)) = super::you(w) else { return };
    let wins = crate::world::storylines::detect::career_wins(w, me);
    for (m, lesson, rep) in [(100u32, 0.5f32, 1.0), (200, 0.75, 2.0), (300, 1.0, 3.0), (400, 1.0, 3.0)] {
        if wins >= m {
            let paid = pay(
                w,
                key("G5", m as u64, 0x3111),
                "G5",
                format!("{m} wins as a head coach"),
                lesson,
                Bank::Rating(Rating::GameManagement),
                rep,
            );
            if paid {
                memo(
                    w,
                    "legacy-wins",
                    format!("{m} wins"),
                    vec![format!("You have {wins} wins as a head coach now. {m} is a number few coaches ever reach.")],
                );
            }
        }
    }
    // Your place on the level's list: a top-ten place is a moment.
    if let Some(case) = case(w, team.level())
        && case.rank <= 10
    {
        pay(
            w,
            key("G5", case.rank as u64, 0x7E40 + team.level() as u64),
            "G5",
            format!("{}: {}", case.rank_label, ordinal(case.rank)),
            0.25,
            Bank::Rating(Rating::Network),
            0.5,
        );
    }
}

fn ordinal(n: u32) -> String {
    let s = match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{n}{s}")
}

/// A winning share as the record books print it: ".846".
fn pct3(x: f64) -> String {
    let s = format!("{x:.3}");
    s.strip_prefix('0').map_or(s.clone(), str::to_string)
}

/// "1 more season", "3 more seasons as a head coach".
fn more_seasons(n: i32, tail: &str) -> String {
    format!("{n} more season{}{tail}", if n == 1 { "" } else { "s" })
}
