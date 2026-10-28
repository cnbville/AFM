//! Lane commands: the calendar (update 3, lane 2, CAL1-CAL7; board 41 in
//! Press Box). `call` answers `calendar_board` and returns None for any
//! other command.
//!
//! `calendar_board` (`month` "2027-11", or `day` "2027-11-16" to pick a
//! day) gives one month of your year:
//! - the year strip (CAL1): your level's football phases from the training
//!   frames (the rule books' blocks, the season split at its playoffs), the
//!   second lane (the NCAA's recruiting periods for college and high school,
//!   the draft and scouting periods at the NFL), the markers (signing days,
//!   the portal, the draft, cutdown, the trade deadline, a state final), the
//!   month shown and today;
//! - the weeks (CAL4): what each week is and its practice plan from your
//!   training week;
//! - the days (CAL2, CAL3, CAL5): at most three short lines a day in four
//!   kinds (your game, needs you, recruiting or scouting, the world), the
//!   game's result or Home and Away, and the day's period;
//! - the picked day hour by hour (CAL6), each line with its kind, where it
//!   lives and a button for anything waiting on you;
//! - ahead (CAL7): the next four dates that matter, each with its countdown
//!   and the one number that counts.
//!
//! Everything is read from the calendar, the rule books, your training week,
//! your season page and your desk: nothing new is stored. Room is left for
//! more kinds of line (lanes to come add track meets and film time).

use std::collections::HashMap;

use afm_engine::calendar::anchors::nfl_regular_weeks;
use afm_engine::calendar::{Date, Event, Kind, Level as CalLevel, Loudness, Period, YearDates, recruiting_period};
use afm_engine::training::plan::{Gear, Week};
use afm_engine::training::rules::{Block, Frame, Tier};
use afm_engine::training::seat as ts;
use afm_engine::training::week::Frames;
use afm_engine::world::season_page::{self as sp, Held, PageGame};
use afm_engine::world::{Stage, World};
use serde_json::{Value, json};

use crate::lane_core::links::Linker;
use crate::lane_core::mail::tidy;
use crate::views::{self, TeamRef};
use crate::{Game, Reply};

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    match cmd {
        "calendar_board" => {
            let Some(w) = game.world.as_ref() else { return Some(Err("No world is loaded.".into())) };
            Some(Ok(board(w, args)))
        }
        // Lane 2's other screen: your coaches, kept outside any save (SA1).
        "coach_profiles" => Some(Ok(crate::setup::coach_profiles(&game.home))),
        "coach_profile_forget" => Some(crate::setup::forget_coach_profile(&game.home, args)),
        _ => None,
    }
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const MONTHS_LONG: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

fn parse_day(s: &str) -> Option<Date> {
    let mut it = s.split('-').map(|x| x.parse::<i32>().ok());
    let (y, m, d) = (it.next()??, it.next()??, it.next().flatten().unwrap_or(1));
    ((1900..=2200).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d))
        .then(|| Date::ymd(y, m as u32, d as u32))
}

fn iso(d: Date) -> String {
    let (y, m, dd) = d.parts();
    format!("{y:04}-{m:02}-{dd:02}")
}

fn month_start(d: Date) -> Date {
    Date::ymd(d.year(), d.month(), 1)
}

fn next_month(d: Date) -> Date {
    if d.month() == 12 { Date::ymd(d.year() + 1, 1, 1) } else { Date::ymd(d.year(), d.month() + 1, 1) }
}

/// "Tue 16 Nov".
fn short_day(d: Date) -> String {
    format!("{} {} {}", d.weekday().short(), d.day(), MONTHS[d.month() as usize - 1])
}

/// "14:30" from minutes after midnight.
fn hhmm(m: u32) -> String {
    format!("{:02}:{:02}", m / 60 % 24, m % 60)
}

/// "in 4 d", "Today", "3 d ago".
fn rel(n: i32) -> String {
    match n {
        0 => "Today".into(),
        1 => "Tomorrow".into(),
        -1 => "Yesterday".into(),
        n if n > 0 => format!("in {n} d"),
        n => format!("{} d ago", -n),
    }
}

fn money(x: f64) -> String {
    format!("${:.1}M", x / 1e6)
}

/// The football year a day belongs to in the rule books (the day after one
/// Super Bowl to the next).
fn season_of(d: Date, ys: &mut Years) -> i32 {
    let y = d.year();
    if d >= ys.get(y).start { y } else { y - 1 }
}

/// The anchor dates by football year, built once each.
#[derive(Default)]
struct Years(HashMap<i32, YearDates>);

impl Years {
    fn get(&mut self, s: i32) -> &YearDates {
        self.0.entry(s).or_insert_with(|| YearDates::new(s))
    }
}

/// Your level on the calendar.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lv {
    High,
    College,
    Nfl,
    Other,
}

fn level_of(t: TeamRef) -> Lv {
    match t {
        TeamRef::HighSchool(_) => Lv::High,
        TeamRef::College(_) => Lv::College,
        TeamRef::Nfl(_) => Lv::Nfl,
        TeamRef::Other(_) => Lv::Other,
    }
}

/// Your training frame for a football year: the current one is the
/// training's own; another is built from its year's anchors the same way.
fn frame_for(w: &World, now: Option<&Frame>, t: TeamRef, season: i32, ys: &mut Years) -> Option<Frame> {
    if let Some(f) = now.filter(|f| f.season == season) {
        return Some(*f);
    }
    let y = ys.get(season).clone();
    match t {
        TeamRef::Nfl(_) => Some(Frame::pro(&y, false)),
        TeamRef::College(p) => {
            let frames = Frames::new(w, &y);
            Some(*frames.division(w.programs.get(p)?.division).0)
        }
        TeamRef::HighSchool(h) => {
            let code = &w.states.get(w.high_schools.get(h)?.state)?.code;
            let (start, _, finals) = afm_engine::world::hs_year::state_season_dates(code, season);
            Some(Frame::high(&y, start, finals, afm_engine::calendar::data::hs_calendar(code).spring_ball))
        }
        TeamRef::Other(_) => None,
    }
}

/// One of your games on the month.
struct MyGame {
    date: Date,
    opp: TeamRef,
    /// "vs" or "at" (a neutral site reads "vs").
    at: &'static str,
    neutral: bool,
    /// Your points, theirs.
    score: Option<[u32; 2]>,
    name: Option<String>,
    playoff: bool,
    kick: Option<u32>,
    page: Option<usize>,
    tv: String,
    place: Option<String>,
    opp_rec: Option<[u32; 3]>,
}

impl MyGame {
    fn result(&self) -> Option<&'static str> {
        self.score.map(|[a, b]| match a.cmp(&b) {
            std::cmp::Ordering::Greater => "W",
            std::cmp::Ordering::Less => "L",
            std::cmp::Ordering::Equal => "T",
        })
    }
    /// The cell's corner: "W 34-20", "Home", "Away", "Neutral".
    fn corner(&self) -> String {
        match (self.result(), self.score) {
            (Some(r), Some([a, b])) => format!("{r} {a}-{b}"),
            _ if self.neutral => "Neutral".into(),
            _ if self.at == "vs" => "Home".into(),
            _ => "Away".into(),
        }
    }
}

/// Your games around a span: the fixtures at your level, with the season
/// page's kickoff, network and names where the page has them.
fn my_games(w: &World, t: TeamRef, from: Date, to: Date) -> Vec<MyGame> {
    let kept = w.seat_games();
    let page_of = |d: Date| kept.iter().position(|g| g.date == d);
    let page = match t {
        TeamRef::College(p) => sp::page(w, CalLevel::College, p, Held::This),
        TeamRef::Nfl(c) => sp::page(w, CalLevel::Nfl, c, Held::This),
        TeamRef::HighSchool(h) => sp::page(w, CalLevel::HighSchool, h, Held::This),
        TeamRef::Other(_) => None,
    };
    let on_page = |d: Date| -> Option<&PageGame> { page.as_ref().and_then(|p| p.games.iter().find(|g| g.date == d)) };
    let mut out = Vec::new();
    let mut push = |g: MyGame| {
        if g.date >= from && g.date < to {
            out.push(g);
        }
    };
    let fill = |mut g: MyGame, pg: Option<&PageGame>| {
        if let Some(pg) = pg {
            g.kick = g.kick.or(pg.kick.map(|(h, m)| h as u32 * 60 + m as u32));
            g.tv = pg.tv.clone();
            g.name = g.name.or(pg.name.clone());
            g.playoff |= pg.playoff || pg.bowl;
            g.place = pg.site.as_ref().map(|s| s.0.clone()).or(pg.site_stadium.clone());
            g.opp_rec = Some(pg.opp_in);
            if g.score.is_none() {
                g.score = pg.score;
            }
        }
        g
    };
    let blank = |date: Date, opp: TeamRef, at: &'static str| MyGame {
        date,
        opp,
        at,
        neutral: false,
        score: None,
        name: None,
        playoff: false,
        kick: None,
        page: page_of(date),
        tv: String::new(),
        place: None,
        opp_rec: None,
    };
    match t {
        TeamRef::College(p) => {
            for f in w.college_fixtures(Some(p)).into_iter().filter(|f| !f.cancelled) {
                let home = f.home == p;
                let opp = if home { f.away } else { f.home };
                let mut g = blank(f.date, TeamRef::College(opp), if home || f.neutral { "vs" } else { "at" });
                g.neutral = f.neutral;
                g.score = f.score.map(|s| if home { s } else { [s[1], s[0]] });
                g.name = f.name.map(str::to_string);
                g.kick = Some(f.kickoff.0 as u32 * 60 + f.kickoff.1 as u32);
                push(fill(g, on_page(f.date)));
            }
        }
        TeamRef::Nfl(c) => {
            for f in w.nfl_fixtures(Some(c)) {
                let Some(d) = f.when else { continue };
                let home = f.home == c;
                let opp = if home { f.away } else { f.home };
                let mut g = blank(d, TeamRef::Nfl(opp), if home { "vs" } else { "at" });
                g.score = f.score.map(|s| if home { s } else { [s[1], s[0]] });
                g.kick = f.minute.map(u32::from);
                push(fill(g, on_page(d)));
            }
        }
        TeamRef::HighSchool(h) => {
            for f in w.hs_fixtures(h) {
                let home = f.home == h;
                let opp = if home { f.away } else { f.home };
                let mut g = blank(f.date, TeamRef::HighSchool(opp), if home { "vs" } else { "at" });
                g.playoff = f.playoff;
                g.score = kept.iter().find(|k| k.date == f.date).map(|k| {
                    let s = usize::from(k.teams[0] != h);
                    [k.stats.score[s].max(0) as u32, k.stats.score[1 - s].max(0) as u32]
                });
                push(fill(g, on_page(f.date)));
            }
        }
        TeamRef::Other(fr) => {
            for (d, opp, home, score, round) in afm_engine::world::pro::other::season_games(w, fr) {
                let mut g = blank(d, TeamRef::Other(opp), if home { "vs" } else { "at" });
                g.score = score.map(|s| [s[0] as u32, s[1] as u32]);
                g.playoff = round > 0;
                push(g);
            }
        }
    }
    out.sort_by_key(|g| g.date);
    out
}

/// A line's kind on the calendar (CAL3): your game, something waiting on
/// you, recruiting or scouting, and the world. Practices, meetings and
/// pressers show only in the day panel (`Team`, `Media`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum K {
    Game,
    You,
    Rec,
    Team,
    Media,
    World,
}

impl K {
    fn key(self) -> &'static str {
        match self {
            K::Game => "g",
            K::You => "y",
            K::Rec => "r",
            K::Team => "t",
            K::Media => "m",
            K::World => "w",
        }
    }
    fn in_cell(self) -> bool {
        matches!(self, K::Game | K::You | K::Rec | K::World)
    }
}

/// One line of a day: its kind, time, short and full text, the line under
/// it, where it lives and a button for anything waiting on you.
struct Line {
    k: K,
    minute: Option<u32>,
    short: String,
    full: String,
    sub: String,
    go: Option<Value>,
    act: Option<Value>,
}

/// A short title for a cell: the line's first clause, without links.
fn short_of(text: &str) -> String {
    // Asides in brackets go, so "Adam Taylor (QB, Ames) commits" keeps its verb.
    let mut plain = String::new();
    let mut depth = 0usize;
    for c in self::plain(text).chars() {
        match c {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => plain.push(c),
            _ => {}
        }
    }
    let plain = plain.split_whitespace().collect::<Vec<_>>().join(" ").replace(" ,", ",").replace(" .", ".");
    // A full stop ends the line, but not the one in "Jr." or "St." or an initial.
    let stop = plain.match_indices(". ").map(|(i, _)| i).find(|&i| {
        let word = plain[..i].rsplit(' ').next().unwrap_or("");
        !(matches!(word, "Jr" | "Sr" | "St" | "Mt" | "Ft" | "Dr" | "Mr" | "vs") || word.len() == 1)
    });
    let cut = [" · ", "; ", " — "].iter().filter_map(|s| plain.find(s)).chain(stop).min().unwrap_or(plain.len());
    plain[..cut].trim_end_matches('.').to_string()
}

/// Text without the screens' link marks ("[[k:id|name]]" to "name").
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(a) = rest.find("[[") {
        out.push_str(&rest[..a]);
        let after = &rest[a + 2..];
        let Some(z) = after.find("]]") else {
            out.push_str(&rest[a..]);
            return out;
        };
        let inner = &after[..z];
        out.push_str(inner.rsplit('|').next().unwrap_or(inner));
        rest = &after[z + 2..];
    }
    out.push_str(rest);
    out
}

fn go(screen: &str, label: &str) -> Option<Value> {
    Some(json!({ "screen": screen, "label": label }))
}

/// Where a line lives on your screens (CAL6).
fn home_of(e: &Event, lv: Lv) -> Option<Value> {
    let t = e.text.to_lowercase();
    let r = e.rule.to_lowercase();
    match e.kind {
        Kind::Offer | Kind::Visit | Kind::Commitment | Kind::Signing if lv == Lv::Nfl => go("contracts", "Contracts"),
        Kind::Offer | Kind::Visit | Kind::Commitment | Kind::Signing => go("recruiting", "Recruiting"),
        Kind::Transfer => go("recruiting", "Portal"),
        Kind::DraftChoice | Kind::DraftPick => go("draftboard", "Draft board"),
        Kind::Poll => go("standings", "Rankings"),
        Kind::Practice | Kind::Rhythm => go("training", "Training week"),
        Kind::Award | Kind::Ceremony => go("news", "News"),
        Kind::Coaching => go("staff", "Staff"),
        _ if r.contains("press") || t.contains("press conference") || t.contains("presser") => go("people", "Media"),
        _ if r.starts_with("people") || r.contains("meeting") => go("people", "People"),
        _ if lv == Lv::Nfl
            && (t.contains("tag") || t.contains("free agen") || t.contains("league year") || t.contains("cap")) =>
        {
            go("contracts", "Cap")
        }
        _ if t.contains("draft") || t.contains("combine") || t.contains("pro day") => go("draftboard", "Draft board"),
        _ if t.contains("signing") || t.contains("recruit") || t.contains("visit") || t.contains("period") => {
            go("recruiting", "Recruiting")
        }
        _ if t.contains("portal") => go("recruiting", "Portal"),
        _ => None,
    }
}

/// A calendar event's kind on your calendar.
fn kind_of(e: &Event, yours: bool, slot: bool, lv: Lv) -> K {
    let t = e.text.to_lowercase();
    let r = e.rule.to_lowercase();
    if (yours || slot) && (e.needs_decision || e.kind == Kind::Deadline) {
        return K::You;
    }
    match e.kind {
        Kind::Practice | Kind::Rhythm => K::Team,
        Kind::Slot if r.contains("press") || t.contains("press") || t.contains("media") => K::Media,
        Kind::Slot => K::Team,
        Kind::Offer | Kind::Visit | Kind::Commitment | Kind::Signing | Kind::Transfer if yours => K::Rec,
        Kind::DraftChoice | Kind::DraftPick if yours || lv == Lv::Nfl => K::Rec,
        Kind::Window | Kind::Date if r.contains("ncaa") || t.contains("recruiting") || t.contains(" period") => K::Rec,
        _ => K::World,
    }
}

/// Whether a big moment belongs on your calendar: your own level's, the
/// world's, and the levels next to yours (recruiting for high school and
/// college, the draft for college). Another level's needs to be huge.
fn near(e: &Event, lv: Lv) -> bool {
    let draft = matches!(e.kind, Kind::DraftChoice | Kind::DraftPick) || e.text.to_lowercase().contains("draft");
    match (lv, e.level) {
        (_, CalLevel::World) => true,
        (Lv::College, CalLevel::College | CalLevel::Recruiting) => true,
        (Lv::College, CalLevel::Nfl) => draft,
        (Lv::High, CalLevel::HighSchool | CalLevel::Recruiting) => true,
        (Lv::Nfl, CalLevel::Nfl) => true,
        (Lv::Nfl, CalLevel::College) => draft,
        (Lv::Other, CalLevel::Other) => true,
        _ => false,
    }
}

/// A fixed date's kind: recruiting and scouting dates at your level, the
/// rest the world.
fn fixed_kind(text: &str, lv: Lv) -> K {
    let t = text.to_lowercase();
    let rec = match lv {
        Lv::Nfl => t.contains("draft") || t.contains("combine") || t.contains("pro day"),
        Lv::College | Lv::High => {
            t.contains("signing") || t.contains("portal") || t.contains("official visits") || t.contains("all-american")
        }
        Lv::Other => false,
    };
    if rec { K::Rec } else { K::World }
}

/// Every line of a day, in time order.
#[allow(clippy::too_many_arguments)]
fn lines_on(
    w: &World,
    d: Date,
    lv: Lv,
    game: Option<&MyGame>,
    fixed: &[(Date, String)],
    yours: &dyn Fn(&Event) -> bool,
    my_slot: &dyn Fn(&Event) -> bool,
    full: bool,
) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    if let Some(g) = game {
        let opp = views::team_name(w, g.opp);
        let short = if g.at == "at" { format!("@ {opp}") } else { opp.clone() };
        let full_text = match g.score {
            Some([a, b]) => format!("{} {opp} · {a}-{b}", if g.at == "at" { "at" } else { "vs" }),
            None => format!(
                "{} {opp}{}",
                if g.at == "at" { "at" } else { "vs" },
                g.name.as_ref().map_or(String::new(), |n| format!(" · {n}"))
            ),
        };
        let mut sub: Vec<String> = Vec::new();
        if let Some(n) = g.name.as_ref().filter(|_| g.score.is_some()) {
            sub.push(n.clone());
        }
        if let Some(p) = &g.place {
            sub.push(p.clone());
        }
        if !g.tv.is_empty() {
            sub.push(g.tv.clone());
        }
        if g.score.is_none()
            && let Some([a, b, c]) = g.opp_rec
            && a + b + c > 0
        {
            sub.push(format!("{opp} {}", if c > 0 { format!("{a}-{b}-{c}") } else { format!("{a}-{b}") }));
        }
        let go = match g.page {
            Some(i) => Some(json!({ "screen": "game", "label": "Game page", "index": i })),
            None if d >= w.today => go("schedule", "Schedule"),
            None => go("schedule", "Schedule"),
        };
        out.push(Line { k: K::Game, minute: g.kick, short, full: full_text, sub: sub.join(" · "), go, act: None });
    }
    for (_, text) in fixed.iter().filter(|x| x.0 == d) {
        let k = fixed_kind(text, lv);
        let go = match k {
            K::Rec if lv == Lv::Nfl => go("draftboard", "Draft board"),
            K::Rec => go("recruiting", "Recruiting"),
            _ => None,
        };
        out.push(Line {
            k,
            minute: None,
            short: short_of(text),
            full: text.clone(),
            sub: String::new(),
            go,
            act: None,
        });
    }
    // The calendar's own lines: yours, your dated slots, the loud ones and
    // the windows your seat cares about.
    let evs: Vec<&Event> = w
        .calendar
        .on(d)
        .iter()
        .filter(|e| e.kind != Kind::Game)
        .filter(|e| {
            yours(e)
                || my_slot(e)
                || e.loud >= Loudness::Big && (near(e, lv) || e.loud >= Loudness::Huge)
                || matches!(e.kind, Kind::Date | Kind::Deadline | Kind::Window)
                    && e.loud >= Loudness::Medium
                    && w.seat.is_some_and(|s| s.cares(e))
                || full && matches!(e.kind, Kind::Practice | Kind::Rhythm) && yours(e)
        })
        .filter(|e| e.kind != Kind::Slot || my_slot(e))
        .collect();
    let texts: Vec<&str> = evs.iter().map(|e| e.text.as_str()).take(40).collect();
    let lk = Linker::new(w, &texts, lv == Lv::High);
    for e in evs.iter().take(40) {
        let y = yours(e);
        let s = my_slot(e);
        let k = kind_of(e, y, s, lv);
        let linked = tidy(&lk.event(e));
        // A time only where the line was set for one: the world's default
        // morning stamp reads as the whole day.
        let timed = e.when.minute != 9 * 60 || matches!(e.kind, Kind::Slot | Kind::Deadline | Kind::Practice);
        out.push(Line {
            k,
            minute: timed.then_some(e.when.minute as u32),
            short: short_of(&e.text),
            full: linked,
            sub: if e.rule.is_empty() || e.rule.starts_with("Your ") { String::new() } else { e.rule.to_string() },
            go: home_of(e, lv),
            act: None,
        });
    }
    // Letters waiting on your answer, on the day they're due.
    if let Some(c) = &w.career {
        for m in c.desk.mails.iter().filter(|m| m.answered.is_none() && m.answer_by == Some(d) && !m.answers.is_empty())
        {
            let act = (d >= w.today).then(|| json!({ "label": "Answer it", "mail": m.id.to_string() }));
            out.push(Line {
                k: K::You,
                minute: None,
                short: format!("Answer: {}", short_of(&m.subject)),
                full: format!("Answer by today: {}", m.subject),
                sub: format!("{} · {}", m.from, m.role),
                go: go("inbox", "Inbox"),
                act,
            });
        }
    }
    // Your meetings and talks (people): the room opens on its day.
    for s in w.folk.scenes.iter().filter(|s| s.date == d && (full || s.stops)) {
        let title = crate::lane_people::plain(&s.title);
        let waiting = !s.done && s.date <= w.today;
        out.push(Line {
            k: if s.stops && !s.done { K::You } else { K::Team },
            minute: None,
            short: format!("Talk: {}", short_of(&title)),
            full: title,
            sub: s.place.clone(),
            go: Some(json!({ "screen": "people_scene", "label": "The room", "id": s.id })),
            act: waiting.then(|| json!({ "label": "Open the talk", "scene": s.id })),
        });
    }
    out
}

/// Your practice day in the day panel, from the training week.
fn practice_line(w: &World, st: &ts::SeatTeam, d: Date) -> Option<Line> {
    if d < st.frame.start || d > st.frame.end {
        return None;
    }
    let monday = d.add(-(d.weekday() as i32));
    let pw = ts::planned(w, st, monday);
    let day = &pw.week.days[d.weekday() as usize];
    if day.game || day.rest() {
        return None;
    }
    let field = day.field();
    let gear =
        day.periods.iter().filter(|p| p.kind.on_field()).map(|p| p.gear).max_by_key(|g| *g as u8).unwrap_or(Gear::None);
    let block = pw.block;
    let clock = crate::lane_training::day_start(st.tier, block, d);
    let what = if field > 0 {
        format!(
            "Practice: {}, {}{}",
            ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"][d.weekday() as usize],
            match gear {
                Gear::Pads => "full pads",
                Gear::Shells => "shells",
                Gear::Helmets => "helmets",
                Gear::None => "no pads",
            },
            format_args!(", {}", hm(field))
        )
    } else {
        "Meetings, film and the weight room".to_string()
    };
    Some(Line {
        k: K::Team,
        minute: Some(clock),
        short: String::new(),
        full: what,
        sub: format!("{} · {} on the clock in all", block.label(st.tier), hm(day.minutes())),
        go: go("training", "Training week"),
        act: None,
    })
}

fn hm(m: u32) -> String {
    match (m / 60, m % 60) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m:02}"),
    }
}

/// The week's practice plan in a few words: "Pads Tue-Thu, walk Fri".
fn plan_words(wk: &Week, block: Block) -> String {
    if block.own() {
        return "On their own".into();
    }
    let class = |k: usize| -> Option<&'static str> {
        let d = &wk.days[k];
        if d.game || d.rest() {
            return None;
        }
        let gear = d.periods.iter().filter(|p| p.kind.on_field()).map(|p| p.gear as u8).max();
        Some(match gear {
            Some(3) => "pads",
            Some(2) => "shells",
            Some(1) => "helmets",
            Some(_) => "walk",
            None => "lift",
        })
    };
    let mut runs: Vec<(&str, usize, usize)> = Vec::new();
    for k in 0..7 {
        let Some(c) = class(k) else { continue };
        match runs.last_mut() {
            Some(r) if r.0 == c && r.2 + 1 == k => r.2 = k,
            _ => runs.push((c, k, k)),
        }
    }
    let field: Vec<&(&str, usize, usize)> = runs.iter().filter(|r| r.0 != "lift").collect();
    let shown = if field.is_empty() { runs.iter().collect::<Vec<_>>() } else { field };
    let parts: Vec<String> = shown
        .iter()
        .take(3)
        .map(|(c, a, b)| {
            let days = if a == b { DAYS[*a].to_string() } else { format!("{}-{}", DAYS[*a], DAYS[*b]) };
            format!("{c} {days}")
        })
        .collect();
    let s = parts.join(", ");
    if s.is_empty() {
        return "No practice".into();
    }
    s[..1].to_uppercase() + &s[1..]
}

/// The NFL's draft and scouting periods (CAL5, the NFL's own version):
/// meet them (the all-star weeks, the combine, top-30 visits), see them
/// work (the college season, pro days), film only, and draft week.
fn nfl_period(d: Date, ys: &mut Years) -> &'static str {
    let s = season_of(d, ys);
    let prev = ys.get(s - 1).clone();
    let y = ys.get(s).clone();
    if (d >= prev.shrine_bowl && d <= prev.senior_bowl) || (d >= y.shrine_bowl && d <= y.senior_bowl) {
        return "contact";
    }
    if d >= y.combine_start && d <= y.combine_end {
        return "contact";
    }
    if d > y.combine_end && d < y.league_year.date {
        return "eval";
    }
    if d >= y.league_year.date && d <= y.top30_deadline {
        return "contact";
    }
    if d > y.top30_deadline && d <= y.draft[2].date {
        return "dead";
    }
    let college = y.cfb_weeks.first().copied().unwrap_or(y.labor_day).add(-7);
    if d > y.draft[2].date && d < college {
        return "quiet";
    }
    if d >= college {
        return "eval";
    }
    // From the Super Bowl to the combine: last fall's film, between the
    // all-star games.
    "quiet"
}

/// The recruiting period on a day (the NCAA's calendar).
fn ncaa_period(d: Date, ys: &mut Years) -> &'static str {
    let s = if d.month() == 1 { d.year() - 1 } else { d.year() };
    match recruiting_period(ys.get(s), d) {
        Period::Contact => "contact",
        Period::Evaluation => "eval",
        Period::Quiet => "quiet",
        Period::Dead => "dead",
    }
}

fn period_on(d: Date, lv: Lv, ys: &mut Years) -> Option<&'static str> {
    match lv {
        Lv::College | Lv::High => Some(ncaa_period(d, ys)),
        Lv::Nfl => Some(nfl_period(d, ys)),
        Lv::Other => None,
    }
}

fn period_names(lv: Lv) -> Vec<(&'static str, &'static str)> {
    match lv {
        Lv::Nfl => {
            vec![("contact", "Meet them"), ("eval", "See them work"), ("quiet", "Film only"), ("dead", "Draft week")]
        }
        Lv::Other => Vec::new(),
        _ => vec![("contact", "Contact"), ("eval", "Evaluation"), ("quiet", "Quiet"), ("dead", "Dead")],
    }
}

/// A phase's short name on the strip and its shade (1 offseason, 2
/// practice, 3 season, 4 postseason).
fn phase_name(b: Block, tier: Tier) -> (&'static str, u8) {
    match (b, tier) {
        (Block::Winter, Tier::Pro) => ("Offseason", 1),
        (Block::Winter, Tier::College) => ("Winter workouts", 1),
        (Block::Winter, Tier::High) => ("Winter lifting", 1),
        (Block::Spring, Tier::High) => ("Spring ball", 2),
        (Block::Spring, _) => ("Spring ball", 2),
        (Block::Quiet, _) => ("On their own", 1),
        (Block::Summer, Tier::College) => ("Summer workouts", 1),
        (Block::Summer, _) => ("Summer: lifting, 7-on-7", 1),
        (Block::Camp, Tier::Pro) => ("Camp, preseason", 2),
        (Block::Camp, Tier::College) => ("Camp", 2),
        (Block::Camp, Tier::High) => ("Practice", 2),
        (Block::Season, _) => ("Season", 3),
        (Block::Postseason, _) => ("Title game, bowl", 4),
        (Block::Phase1 | Block::Phase2, _) => ("Offseason program", 2),
        (Block::Otas, _) => ("OTAs, minicamp", 2),
        (Block::Break, _) => ("Break", 1),
    }
}

/// The strip's phases (CAL1, lane A) over [r0, r1).
#[allow(clippy::too_many_arguments)]
fn phases(
    w: &World,
    t: TeamRef,
    now: Option<&Frame>,
    r0: Date,
    r1: Date,
    games: &[MyGame],
    ys: &mut Years,
) -> Vec<(Date, Date, String, u8)> {
    let mut segs: Vec<(Date, Date, String, u8)> = Vec::new();
    let mut frames: HashMap<i32, Option<Frame>> = HashMap::new();
    let mut d = r0;
    while d < r1 {
        let s = season_of(d, ys);
        let f = *frames.entry(s).or_insert_with(|| frame_for(w, now, t, s, ys));
        let (name, l): (String, u8) = match f {
            Some(f) => {
                let b = f.block(d);
                let (mut n, mut l) = phase_name(b, f.tier);
                let y = ys.get(s).clone();
                if f.tier == Tier::Pro && b == Block::Winter {
                    if d >= y.combine_start && d <= y.combine_end {
                        n = "Combine";
                        l = 2;
                    } else if d > y.combine_end && d < y.league_year.date {
                        n = "Tags";
                        l = 2;
                    } else if d >= y.league_year.date && d < f.phase1 {
                        n = "Free agency";
                        l = 3;
                    } else if d >= y.wild_card && d <= y.super_bowl {
                        n = "Playoffs";
                        l = 4;
                    }
                }
                if f.tier == Tier::College && b == Block::Postseason {
                    let fbs = match t {
                        TeamRef::College(p) => {
                            w.programs.get(p).is_some_and(|x| x.division == afm_engine::world::data::Division::FBS)
                        }
                        _ => false,
                    };
                    if !fbs {
                        n = "Playoffs";
                    }
                }
                if f.tier == Tier::High && b == Block::Season {
                    let (_, po, _) = match t {
                        TeamRef::HighSchool(h) => w
                            .high_schools
                            .get(h)
                            .and_then(|x| w.states.get(x.state))
                            .map(|st| afm_engine::world::hs_year::state_season_dates(&st.code, s))
                            .unwrap_or((d, d.add(9999), d)),
                        _ => (d, d.add(9999), d),
                    };
                    if d >= po {
                        n = "Playoffs";
                        l = 4;
                    }
                }
                let mut name = n.to_string();
                if n == "Season" {
                    let regular = games
                        .iter()
                        .filter(|g| !g.playoff && season_of(g.date, ys) == s && f.block(g.date) == Block::Season)
                        .count();
                    let regular = if regular == 0 && f.tier == Tier::Pro { nfl_regular_weeks(s) - 1 } else { regular };
                    if regular > 0 {
                        name = format!("Season · {regular} games");
                    }
                }
                (name, l)
            }
            None => {
                // A club in another league: its season from its games.
                let on = games.first().is_some_and(|a| d >= a.date) && games.last().is_some_and(|z| d <= z.date);
                if on { (format!("Season · {} games", games.len()), 3) } else { ("Offseason".into(), 1) }
            }
        };
        match segs.last_mut() {
            Some(x) if x.2 == name => x.1 = d,
            _ => segs.push((d, d, name, l)),
        }
        d = d.add(1);
    }
    segs
}

/// The markers on the strip: (date, name).
fn marks(w: &World, t: TeamRef, lv: Lv, r0: Date, r1: Date, ys: &mut Years) -> Vec<(Date, String)> {
    let mut out: Vec<(Date, String)> = Vec::new();
    for s in [r0.year() - 1, r0.year(), r0.year() + 1] {
        let y = ys.get(s).clone();
        match lv {
            Lv::College => {
                if let Some(e) = y.early_signing {
                    out.push((e, "Early signing".into()));
                }
                out.push((y.portal_open, "Portal window opens".into()));
                out.push((y.signing_day, "Signing day".into()));
            }
            Lv::High => {
                if let TeamRef::HighSchool(h) = t
                    && let Some(st) = w.high_schools.get(h).and_then(|x| w.states.get(x.state))
                {
                    let (_, _, f) = afm_engine::world::hs_year::state_season_dates(&st.code, s);
                    out.push((f, "State final".into()));
                }
                if let Some(e) = y.early_signing {
                    out.push((e, "Early signing".into()));
                }
                out.push((y.signing_day, "Signing day".into()));
            }
            Lv::Nfl => {
                out.push((y.draft[0].date, "Draft".into()));
                out.push((y.cutdown.date, "Cut to 53".into()));
                out.push((y.trade_deadline.date, "Trade deadline".into()));
            }
            Lv::Other => {}
        }
    }
    out.retain(|x| x.0 >= r0 && x.0 < r1);
    out.sort_by_key(|x| x.0);
    out.dedup_by(|a, b| a.0 == b.0);
    out
}

/// What a week is (CAL4): its name, what it holds and its practice plan.
#[allow(clippy::too_many_arguments)]
fn week_row(
    w: &World,
    lv: Lv,
    monday: Date,
    games: &[MyGame],
    fixed: &[(Date, String)],
    st: Option<&ts::SeatTeam>,
    phase: &str,
    ys: &mut Years,
) -> Value {
    let end = monday.add(7);
    // The week's last game names it: a Monday night game belongs to the
    // NFL week before.
    let game = games.iter().rev().find(|g| g.date >= monday && g.date < end);
    let s = season_of(monday, ys);
    let y = ys.get(s).clone();
    let plan = st.filter(|st| monday.add(6) >= st.frame.start && monday <= st.frame.end).map(|st| {
        let pw = ts::planned(w, st, monday);
        plan_words(&pw.week, pw.block)
    });
    let (big, a) = match game {
        Some(g) => {
            let opp = views::team_name(w, g.opp);
            let a = if g.at == "at" {
                format!("@ {opp}")
            } else if g.neutral {
                format!("{opp} · neutral")
            } else {
                format!("{opp} · home")
            };
            let big = match lv {
                Lv::College => match (&g.name, y.cfb_week(g.date)) {
                    (Some(n), _)
                        if g.playoff || n.contains("Championship") || n.contains("Bowl") || n.contains("Playoff") =>
                    {
                        if n.contains("Championship") && !n.contains("National") {
                            "Title".to_string()
                        } else if n.contains("Bowl") {
                            "Bowl".to_string()
                        } else {
                            "Playoff".to_string()
                        }
                    }
                    (_, Some(k)) => format!("Wk {k}"),
                    _ => "Game week".into(),
                },
                Lv::Nfl => {
                    if g.date >= y.wild_card {
                        if g.date == y.super_bowl {
                            "Super Bowl".into()
                        } else if g.date >= y.conference_title_games {
                            "Title game".into()
                        } else if g.date >= y.divisional {
                            "Divisional".into()
                        } else {
                            "Wild card".into()
                        }
                    } else if g.date < y.nfl_kickoff.add(-3) {
                        "Preseason".into()
                    } else {
                        let k = y.nfl_weeks.iter().position(|&s| s.add(3) >= g.date).map_or(0, |k| k + 1);
                        format!("Wk {k}")
                    }
                }
                Lv::High => {
                    if g.playoff {
                        "Playoffs".into()
                    } else {
                        let k = match views::mine(w) {
                            Some(TeamRef::HighSchool(h)) => w.hs_weeks(h).and_then(|(fri, _, _)| {
                                fri.iter().position(|&f| f.add(-4) <= g.date && f.add(3) >= g.date)
                            }),
                            _ => None,
                        };
                        k.map_or("Game week".into(), |k| format!("Wk {}", k + 1))
                    }
                }
                Lv::Other => {
                    if g.playoff {
                        "Playoffs".into()
                    } else {
                        let k = games.iter().filter(|x| !x.playoff && x.date <= g.date).count();
                        format!("Wk {k}")
                    }
                }
            };
            (big, a)
        }
        None => {
            // The week's main date at your level, else the phase.
            let key = fixed.iter().find(|x| x.0 >= monday && x.0 < end);
            match key {
                Some((d, t)) => (phase.to_string(), format!("{} {}", short_of(t), d.weekday().short())),
                None if phase.starts_with("Season") => ("Bye".to_string(), "No game this week".into()),
                None => (phase.to_string(), String::new()),
            }
        }
    };
    let now = monday <= w.today && w.today < end;
    json!({ "big": big, "a": a, "b": plan.unwrap_or_default(), "now": now, "monday": iso(monday) })
}

/// The board.
fn board(w: &World, args: &Value) -> Value {
    let picked = args.get("day").and_then(Value::as_str).and_then(parse_day);
    let focus = picked.or_else(|| args.get("month").and_then(Value::as_str).and_then(parse_day)).unwrap_or(w.today);
    let first = month_start(focus);
    let next = next_month(first);
    let prev = month_start(first.add(-1));
    let mut ys = Years::default();
    let me = views::mine(w);
    let lv = me.map_or(Lv::College, level_of);
    let st = w.training_team();
    let now_frame = st.as_ref().map(|s| s.frame);
    // The year shown: June to June for college and high school, February to
    // February at the NFL and in the other leagues.
    let (r0, r1, label) = match lv {
        Lv::College | Lv::High => {
            let y = if first.month() >= 6 { first.year() } else { first.year() - 1 };
            (Date::ymd(y, 6, 1), Date::ymd(y + 1, 6, 1), format!("{}-{:02} year", y, (y + 1) % 100))
        }
        _ => {
            let y = if first.month() >= 2 { first.year() } else { first.year() - 1 };
            (Date::ymd(y, 2, 1), Date::ymd(y + 1, 2, 1), format!("{y}"))
        }
    };
    let span = r0.days_to(r1).max(1) as f64;
    let x = |d: Date| ((r0.days_to(d) as f64 / span).clamp(0.0, 1.0) * 10_000.0).round() / 100.0;
    let start = first.add(-(first.weekday() as i32));
    let grid_end = next.add((7 - next.weekday() as i32) % 7);
    let games = me.map_or_else(Vec::new, |t| my_games(w, t, r0.min(start).add(-7), r1.max(grid_end).add(7)));
    let fixed: Vec<(Date, String)> = views::fixed_dates(w, &[r0.year() - 1, r0.year(), r0.year() + 1])
        .into_iter()
        .filter(|x| x.0 >= r0.min(start) && x.0 < r1.max(grid_end))
        .collect();

    // The strip (CAL1).
    let mut strip = json!(null);
    let mut phase_at: Vec<(Date, Date, String, u8)> = Vec::new();
    if let Some(t) = me {
        phase_at = phases(w, t, now_frame.as_ref(), r0.min(start), r1.max(grid_end), &games, &mut ys);
        let mut pers: Vec<(Date, Date, &'static str)> = Vec::new();
        let mut d = r0;
        while d < r1 {
            if let Some(k) = period_on(d, lv, &mut ys) {
                match pers.last_mut() {
                    Some(p) if p.2 == k => p.1 = d,
                    _ => pers.push((d, d, k)),
                }
            }
            d = d.add(1);
        }
        let mut months = Vec::new();
        let mut m = r0;
        while m < r1 {
            months.push(json!({ "n": MONTHS[m.month() as usize - 1].to_uppercase(), "x": x(m), "month": format!("{:04}-{:02}", m.year(), m.month()), "on": m == first }));
            m = next_month(m);
        }
        let lane_b = match lv {
            Lv::College => "Recruiting",
            Lv::High => "College coaches",
            Lv::Nfl => "Draft and scouting",
            Lv::Other => "",
        };
        strip = json!({
            "label": label,
            "lanes": ["Football", lane_b],
            "months": months,
            "phases": phase_at.iter().filter(|p| p.1 >= r0 && p.0 < r1).map(|(a, b, n, l)| {
                let (a, b) = ((*a).max(r0), (*b).min(r1.add(-1)));
                json!({ "n": n, "x": x(a), "w": ((x(b.add(1)) - x(a)) * 100.0).round() / 100.0, "l": l, "from": short_day(a), "to": short_day(b), "month": format!("{:04}-{:02}", a.year(), a.month()) })
            }).collect::<Vec<_>>(),
            "pers": pers.iter().map(|(a, b, k)| json!({ "k": k, "x": x(*a), "w": ((x(b.add(1)) - x(*a)) * 100.0).round() / 100.0 })).collect::<Vec<_>>(),
            "marks": marks(w, t, lv, r0, r1, &mut ys).into_iter().map(|(d, n)| json!({ "n": n, "x": x(d), "date": short_day(d), "iso": iso(d) })).collect::<Vec<_>>(),
            "today": (w.today >= r0 && w.today < r1).then(|| x(w.today)),
            "box": { "x": x(first), "w": ((x(next) - x(first)) * 100.0).round() / 100.0 },
            "prev_year": format!("{:04}-{:02}", r0.year() - 1, r0.month()),
            "next_year": format!("{:04}-{:02}", r1.year(), r1.month()),
        });
    }
    let phase_on =
        |d: Date| -> String { phase_at.iter().find(|p| d >= p.0 && d <= p.1).map_or(String::new(), |p| p.2.clone()) };

    // The weeks and their days (CAL2-CAL5).
    let yours = views::yours_test(w);
    let my_slot = views::my_slot_test(w);
    let game_on = |d: Date| games.iter().find(|g| g.date == d);
    let mut weeks = Vec::new();
    let mut days = Vec::new();
    let mut d = start;
    while d < grid_end {
        weeks.push(week_row(w, lv, d, &games, &fixed, st.as_ref(), &phase_on(d), &mut ys));
        for _ in 0..7 {
            let mut lines: Vec<Line> = lines_on(w, d, lv, game_on(d), &fixed, &yours, &my_slot, false);
            lines.retain(|l| l.k.in_cell());
            lines.sort_by_key(|l| l.k);
            let n = lines.len();
            let g = game_on(d);
            days.push(json!({
                "iso": iso(d), "day": d.day(), "in_month": d >= first && d < next, "today": d == w.today, "past": d < w.today,
                "res": g.map(MyGame::corner),
                "rk": g.and_then(MyGame::result),
                "game": g.is_some(),
                "lines": lines.iter().map(|l| json!({ "k": l.k.key(), "t": l.short })).collect::<Vec<_>>(),
                "more": n.saturating_sub(3),
                "per": period_on(d, lv, &mut ys),
            }));
            d = d.add(1);
        }
    }

    // The picked day (CAL6): today when none is picked.
    let pick = picked.filter(|p| *p >= start && *p < grid_end).unwrap_or(if w.today >= start && w.today < grid_end {
        w.today
    } else {
        first
    });
    let mut rows = lines_on(w, pick, lv, game_on(pick), &fixed, &yours, &my_slot, true);
    if let Some(st) = st.as_ref()
        && let Some(p) = practice_line(w, st, pick)
    {
        rows.push(p);
    }
    rows.sort_by_key(|l| (l.minute.unwrap_or(0), l.k));
    let week_now = weeks.get((start.days_to(pick) / 7) as usize).cloned().unwrap_or(json!({}));
    let per = period_on(pick, lv, &mut ys);
    let per_name = per.and_then(|k| period_names(lv).into_iter().find(|p| p.0 == k)).map(|p| p.1);
    let lane_b = match lv {
        Lv::College => "Recruiting",
        Lv::High => "College coaches",
        Lv::Nfl => "Draft and scouting",
        Lv::Other => "",
    };
    let mut ctx: Vec<String> = Vec::new();
    if let Some(b) = week_now["big"].as_str().filter(|s| !s.is_empty()) {
        let a = week_now["a"].as_str().unwrap_or("");
        ctx.push(if a.is_empty() { b.to_string() } else { format!("{b} · {a}") });
    }
    if let Some(p) = per_name {
        ctx.push(format!("{lane_b}: {}", p.to_lowercase()));
    }
    let day = json!({
        "iso": iso(pick),
        "title": short_day(pick),
        "rel": rel(w.today.days_to(pick)),
        "today": pick == w.today,
        "ctx": ctx.join(" · "),
        "rows": rows.iter().map(|l| json!({
            "time": l.minute.map_or("All day".to_string(), hhmm),
            "k": l.k.key(),
            "label": match l.k { K::Game => "Game", K::You => "Needs you", K::Rec => if lv == Lv::Nfl { "Scouting" } else { "Recruiting" }, K::Team => "Team", K::Media => "Media", K::World => "The world" },
            "t": l.full, "s": l.sub, "go": l.go, "act": l.act,
        })).collect::<Vec<_>>(),
        "past": pick < w.today,
    });

    // Ahead (CAL7).
    let ahead = me.map_or_else(Vec::new, |t| ahead(w, t, lv, &games, &mut ys));

    let head = me.map(|t| {
        let rec = views::record_text(w, t);
        json!({ "team": views::team_brief(w, t), "label": format!("{} · {}", views::team_name(w, t), rec) })
    });
    json!({
        "level": match lv { Lv::High => "hs", Lv::College => "college", Lv::Nfl => "nfl", Lv::Other => "other" },
        "head": head,
        "title": format!("{} {}", MONTHS_LONG[first.month() as usize - 1], first.year()),
        "month": format!("{:04}-{:02}", first.year(), first.month()),
        "prev": format!("{:04}-{:02}", prev.year(), prev.month()),
        "next": format!("{:04}-{:02}", next.year(), next.month()),
        "today": iso(w.today),
        "today_text": views::date_text(w.today),
        "strip": strip,
        "weeks": weeks,
        "days": days,
        "day": day,
        "ahead": ahead,
        "legend": { "r": if lv == Lv::Nfl { "Scouting" } else { "Recruiting" }, "periods": period_names(lv).into_iter().map(|(k, n)| json!({ "k": k, "n": n })).collect::<Vec<_>>() },
    })
}

/// A date's bar under Ahead: (have, need, label, short of it).
type Bar = (f64, f64, String, bool);
/// A date ahead: (day, title, line under it, its bar).
type Dated = (Date, String, String, Option<Bar>);

/// The next four dates that matter (CAL7), with the one number that counts:
/// your class against its places before a signing day, a tag's cost against
/// your cap room before the tag deadline, top-30 visits before the draft.
fn ahead(w: &World, t: TeamRef, lv: Lv, games: &[MyGame], ys: &mut Years) -> Vec<Value> {
    let today = w.today;
    let mut out: Vec<Dated> = Vec::new();
    for g in games.iter().filter(|g| g.date >= today).take(2) {
        let opp = views::team_name(w, g.opp);
        let title = if g.at == "at" { format!("@ {opp}") } else { opp.clone() };
        let mut sub: Vec<String> = Vec::new();
        if let Some(n) = &g.name {
            sub.push(n.clone());
        }
        if let Some([a, b, c]) = g.opp_rec.filter(|r| r[0] + r[1] + r[2] > 0) {
            sub.push(format!("they are {}", if c > 0 { format!("{a}-{b}-{c}") } else { format!("{a}-{b}") }));
        }
        if !g.tv.is_empty() {
            sub.push(g.tv.clone());
        }
        let s = sub.join(" · ");
        out.push((g.date, title, s[..1.min(s.len())].to_uppercase() + &s[1.min(s.len())..], None));
    }
    let s = season_of(today, ys);
    let class = match t {
        TeamRef::College(p) => Some(class_count(w, p)),
        _ => None,
    };
    let mut dates: Vec<Dated> = Vec::new();
    for y in [ys.get(s).clone(), ys.get(s + 1).clone()] {
        match (lv, t) {
            (Lv::College, TeamRef::College(_)) => {
                if let Some(e) = y.early_signing {
                    dates.push((e, "Early signing period".into(), "Letters out at 7 AM".into(), class.clone()));
                }
                dates.push((
                    y.signing_day,
                    "National signing day".into(),
                    "The rest of the class signs".into(),
                    class.clone(),
                ));
                dates.push((
                    y.portal_open,
                    "Transfer portal opens".into(),
                    format!("Open to {}", short_day(y.portal_close)),
                    None,
                ));
                dates.push((y.selection_day, "Playoff selection day".into(), "The committee's field".into(), None));
                dates.push((y.conference_championships, "Conference title games".into(), String::new(), None));
                dates.push((y.spring_practice.0, "Spring practice opens".into(), String::new(), None));
                dates.push((y.fall_camp, "Fall camp opens".into(), String::new(), None));
            }
            (Lv::Nfl, TeamRef::Nfl(c)) => {
                let tag = tag_read(w, c);
                let sub = tag.as_ref().map_or("No tag candidate".to_string(), |(name, cost, _)| {
                    format!("{name}'s tag costs {}", money(*cost))
                });
                let bar = tag.map(|(_, cost, room)| {
                    let short = room < cost;
                    let label = if short {
                        format!("{} short", money(cost - room))
                    } else {
                        format!("{} to spare", money(room - cost))
                    };
                    (room.max(0.0), cost, label, short)
                });
                dates.push((y.tag_deadline.date, "Franchise tag deadline".into(), sub, bar));
                if let Some(m) = y.tampering_open {
                    dates.push((m.date, "Negotiating window".into(), "Noon ET".into(), None));
                }
                let mine = afm_engine::world::front_office::nfl::expiring_count(w, c);
                dates.push((
                    y.league_year.date,
                    "New league year".into(),
                    format!("{mine} of yours become free agents"),
                    None,
                ));
                dates.push((y.annual_meeting.0, "Annual league meeting".into(), String::new(), None));
                let used = w
                    .front_office
                    .nfl
                    .visits
                    .iter()
                    .filter(|v| v.1.year() == y.draft[0].date.year() && v.1 <= y.draft[0].date)
                    .count() as f64;
                dates.push((
                    y.draft[0].date,
                    "Draft, round 1".into(),
                    "Top-30 visits used".into(),
                    Some((used, 30.0, format!("{used:.0} of 30"), false)),
                ));
                dates.push((y.cutdown.date, "Cutdown to 53".into(), String::new(), None));
                dates.push((y.trade_deadline.date, "Trade deadline".into(), String::new(), None));
                dates.push((y.veterans_report, "Veterans report to camp".into(), String::new(), None));
            }
            (Lv::High, TeamRef::HighSchool(h)) => {
                if let Some(st) = w.high_schools.get(h).and_then(|x| w.states.get(x.state)) {
                    let (a, po, f) = afm_engine::world::hs_year::state_season_dates(&st.code, y.season);
                    dates.push((po, "Playoffs begin".into(), String::new(), None));
                    dates.push((f, "State final".into(), String::new(), None));
                    dates.push((a.add(-21), "Practice opens".into(), String::new(), None));
                }
                let seniors = w.high_schools[h]
                    .roster
                    .iter()
                    .filter(|&&i| {
                        w.people.get(i).is_some_and(|p| p.stage == Stage::HighSchool && p.class_of == y.season + 1)
                    })
                    .count();
                dates.push((
                    y.signing_day,
                    "National signing day".into(),
                    format!("{seniors} seniors on your roster"),
                    None,
                ));
            }
            _ => {}
        }
    }
    dates.retain(|x| x.0 >= today);
    out.extend(dates);
    out.sort_by_key(|x| x.0);
    out.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
    out.into_iter()
        .take(4)
        .map(|(d, t, s, bar)| {
            json!({
                "iso": iso(d), "d": d.day(), "m": MONTHS[d.month() as usize - 1].to_uppercase(), "t": t, "s": s,
                "in": rel(today.days_to(d)),
                // A class is short when under half its places are filled
                // with a month to go.
                "bar": bar.map(|(a, b, l, warn)| {
                    let short = warn || t.contains("igning") && a < b * 0.5 && today.days_to(d) <= 30;
                    json!({ "pct": (a / b.max(1.0) * 100.0).clamp(0.0, 100.0).round(), "label": l, "warn": short })
                }),
            })
        })
        .collect()
}

/// Your class against its places: commitments and signees of the class
/// you sign next, from your board.
fn class_count(w: &World, p: usize) -> Bar {
    use afm_engine::world::front_office::college::{self as desk, Standing};
    let o = desk::outlook(w, p);
    // Every commitment and signature in the class, on your board or not.
    let n = w
        .people
        .iter()
        .filter(|pp| {
            pp.class_of == o.sign_year
                && matches!(desk::standing(pp, p), Standing::CommittedHere | Standing::SignedHere)
        })
        .count() as f64;
    let slots = o.slots.max(1) as f64;
    (n, slots, format!("{n:.0} of {slots:.0}"), false)
}

/// Your tag decision: the expiring unrestricted player the market values
/// most, his franchise tag's price and your room this league year.
fn tag_read(w: &World, c: usize) -> Option<(String, f64, f64)> {
    use afm_engine::world::front_office::nfl;
    use afm_engine::world::pro::money;
    let year = w.year;
    let best = w.clubs.get(c)?.roster.iter().copied().filter(|&i| {
        w.people[i].pro.as_ref().is_some_and(|p| {
            p.contract.as_ref().is_some_and(|k| k.last_year() < year) && p.accrued >= money::ufa_seasons(year)
        })
    });
    let i = best.max_by(|&a, &b| nfl::market_read(w, a).1.total_cmp(&nfl::market_read(w, b).1))?;
    let (franchise, _, _) = nfl::tag_prices(w, i);
    let room = afm_engine::world::pro::cap_room(w, c, year);
    Some((w.people[i].name.clone(), franchise, room))
}
