//! The year (update 2, lane 6, V13; board 13 v2): your football year as a
//! ribbon, from the day after one Super Bowl to the next. Five lanes: your
//! games, the training blocks your level's rules set, the roster's dates,
//! recruiting (scouting at the NFL), and the rules lane (the NCAA's
//! recruiting periods; the CBA's offseason at the NFL). Today's place in
//! it, this week's practice plan against the hours allowed, and what is
//! coming up. Everything is read from the calendar, the rule books, your
//! training week and your season page: nothing new is stored.

use afm_engine::calendar::{Date, Level as CalLevel, Period, Weekday, YearDates, recruiting_period};
use afm_engine::training::plan::{Gear, Kind};
use afm_engine::training::rules::{Block, Frame, Tier, block_limits, book};
use afm_engine::training::seat as ts;
use afm_engine::world::season_page::{self as sp, Held, SeasonPage};
use afm_engine::world::{Stage, Visit, World};
use serde_json::{Value, json};

use super::{kick_text, ordinal, wlt};
use crate::Reply;
use crate::views::{self, TeamRef};

const MONTHS: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];

/// One bar on the ribbon.
struct Phase {
    lane: u8,
    a: Date,
    b: Date,
    name: String,
    kv: Vec<(String, String)>,
}

impl Phase {
    fn new(lane: u8, a: Date, b: Date, name: &str) -> Phase {
        Phase { lane, a, b: b.max(a), name: name.to_string(), kv: Vec::new() }
    }
    fn kv(mut self, k: &str, v: impl Into<String>) -> Phase {
        self.kv.push((k.to_string(), v.into()));
        self
    }
}

/// Dates in a few words: "Sat 4 Sep", or "Sat 4 Sep to Sat 27 Nov".
fn span_text(a: Date, b: Date) -> String {
    let one = |d: Date| format!("{} {} {}", d.weekday().short(), d.day(), month_word(d.month()));
    if a == b { one(a) } else { format!("{} to {}", one(a), one(b)) }
}

fn month_word(m: u32) -> &'static str {
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][m as usize - 1]
}

fn hours(min: u32) -> String {
    let (h, m) = (min / 60, min % 60);
    match (h, m) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m:02}"),
    }
}

/// A bar's short label, for a bar too narrow for its name.
fn short(name: &str) -> &str {
    if name.ends_with(" games") || name.ends_with(" game") {
        return "Season";
    }
    match name {
        "Winter conditioning" => "Winter",
        "Offseason program" | "Offseason (on their own)" => "Offseason",
        "Spring practice" | "Spring ball" => "Spring",
        "Quiet weeks (on their own)" => "Quiet",
        "Summer program" | "Summer workouts" => "Summer",
        "Training camp" | "Fall camp" | "Preseason practice" => "Camp",
        "Bowl practice and December" => "Bowls",
        "Offseason program, phase 1" => "Phase 1",
        "Offseason program, phase 2" => "Phase 2",
        "OTAs and minicamp" => "OTAs",
        "Summer break (on their own)" => "Break",
        "Rookie minicamp" => "Rookies",
        "Cutdown to 53" => "To 53",
        "Trade deadline" => "Deadline",
        "Top 30 visits" => "Top 30",
        "All-star games" => "All-stars",
        "The new class" => "New class",
        "School starts" => "School",
        "Junior days" => "Juniors",
        "Spring evaluation" => "Evaluation",
        "Camps and visits" => "Camps",
        "Early signing" => "Early",
        "Signing day" => "Signing",
        "NFL draft" => "Draft",
        "Freshmen arrive" => "Freshmen",
        "Transfer portal" => "Portal",
        "June official visits" | "Home game visits" => "Visits",
        n => n,
    }
}

/// A short code for a tile: "OKST", "TCU", "CHI", "NYG".
pub(crate) fn code(name: &str, nfl: bool) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let first = |w: &str| w.chars().find(|c| c.is_alphanumeric()).unwrap_or(' ');
    if nfl && words.len() >= 2 {
        let city = &words[..words.len() - 1];
        let mut c: String =
            if city.len() == 1 { city[0].chars().take(3).collect() } else { city.iter().map(|w| first(w)).collect() };
        if matches!(city.join(" ").as_str(), "New York" | "Los Angeles") {
            c.push(first(words[words.len() - 1]));
        }
        return c.to_uppercase();
    }
    let words: Vec<&str> =
        words.into_iter().filter(|w| !matches!(*w, "of" | "the" | "and" | "at" | "&" | "-")).collect();
    match words.as_slice() {
        [] => String::new(),
        [one] => one.chars().filter(|c| c.is_alphanumeric()).take(4).collect::<String>().to_uppercase(),
        [a, "State"] => format!("{}ST", a.chars().take(2).collect::<String>()).to_uppercase(),
        many => many.iter().map(|w| first(w)).take(4).collect::<String>().to_uppercase(),
    }
}

/// The training block's facts from its rule book: practices, contact,
/// hours, days off, whether it is voluntary, and the rule's source.
fn block_facts(tier: Tier, b: Block, season: i32, state: &str, heat: bool, days: i32) -> Vec<(String, String)> {
    let k = book(tier, b, season, 30, state, heat);
    let lim = block_limits(tier, b, season);
    let none = u16::MAX;
    let mut kv: Vec<(String, String)> = Vec::new();
    let mut add = |a: &str, v: String| kv.push((a.to_string(), v));
    if !k.coaches {
        add("Coaches", "none: the players train on their own".into());
    }
    if lim.practices != none {
        add("Practices", format!("{} in {days} days", lim.practices));
    }
    if lim.contact != none {
        add("With contact", if lim.contact == 0 { "none".into() } else { format!("{} at most", lim.contact) });
    }
    if lim.padded != none {
        add("In pads", if lim.padded == 0 { "none".into() } else { format!("{} at most", lim.padded) });
    }
    if let Some(m) = k.week_min {
        add("Hours", format!("{} a week", hours(m as u32)));
    }
    if k.coaches && k.days_off > 0 {
        add("Days off", format!("{} a week", k.days_off));
    }
    if k.voluntary {
        add("Attendance", "voluntary".into());
    }
    if b == Block::Camp {
        add("Two-a-days", if k.two_a_days { "allowed".into() } else { "not allowed".into() });
    }
    if !k.rule.is_empty() {
        add("Rule", k.rule.to_string());
    }
    kv
}

/// Your training blocks over the year, the season split at its playoffs.
fn football(frame: &Frame, y: &YearDates, page: Option<&SeasonPage>, state: &str, heat: bool) -> Vec<Phase> {
    let mut segs: Vec<(Block, Date, Date)> = Vec::new();
    let mut d = frame.start;
    while d <= frame.end {
        let b = frame.block(d);
        match segs.last_mut() {
            Some(s) if s.0 == b => s.2 = d,
            _ => segs.push((b, d, d)),
        }
        d = d.add(1);
    }
    let regular = page.map_or(0, |p| p.games.iter().filter(|g| !g.playoff && !g.bowl).count());
    let playoff = page.and_then(|p| p.games.iter().find(|g| g.playoff || g.bowl).map(|g| g.date));
    let last_game = page.and_then(|p| p.games.last().map(|g| g.date));
    // The record over the games `post` picks: the regular season's, or the
    // playoffs' and the bowl's.
    let rec_of = |post: bool| {
        page.map(|p| {
            let r = p.games.iter().filter(|g| (g.playoff || g.bowl) == post).fold([0u32; 3], |mut r, g| {
                match (g.score, g.won) {
                    (Some([a, b]), _) => {
                        r[if a > b {
                            0
                        } else if a < b {
                            1
                        } else {
                            2
                        }] += 1
                    }
                    (None, Some(won)) => r[if won { 0 } else { 1 }] += 1,
                    _ => {}
                }
                r
            });
            wlt(r)
        })
        .filter(|r| r.as_str() != "0-0")
    };
    let (rec, post_rec) = (rec_of(false), rec_of(true));
    let mut out = Vec::new();
    for (b, a, z) in segs {
        let days = a.days_to(z) + 1;
        let mut name = b.label(frame.tier).to_string();
        let mut kv = block_facts(frame.tier, b, frame.season, state, heat, days);
        if b == Block::Season {
            name = format!(
                "{} · {regular} {}",
                if frame.tier == Tier::High { "Season" } else { "Regular season" },
                afm_engine::world::folk::noun(regular, "game")
            );
            if let Some(r) = rec.as_ref() {
                kv.insert(0, ("Record".into(), r.clone()));
            }
            // A high school's playoffs run inside its season block.
            if let Some(p) = playoff.filter(|&p| p > a && p <= z) {
                let mut s = Phase::new(0, a, p.add(-1), &name);
                s.kv = kv.clone();
                out.push(s);
                let mut po = Phase::new(0, p, z, "Playoffs");
                if let Some(r) = post_rec.as_ref() {
                    po = po.kv("Record", r.clone());
                }
                out.push(po.kv("First round", span_text(p, p)).kv("Practice", "the season's rules"));
                continue;
            }
        }
        // An NFL playoff run starts after the regular season's block.
        if frame.tier == Tier::Pro
            && b == Block::Winter
            && a > frame.first_game
            && let (Some(p), Some(l)) = (playoff, last_game)
            && p >= a
        {
            let mut po = Phase::new(0, a, l, "Playoffs");
            if let Some(r) = post_rec.as_ref() {
                po = po.kv("Record", r.clone());
            }
            out.push(po.kv("Seed", "set by the regular season").kv("Practice", "the season's rules"));
            if l < z {
                let mut s = Phase::new(0, l.add(1), z, &name);
                s.kv = kv;
                out.push(s);
            }
            continue;
        }
        if b == Block::Postseason && frame.tier == Tier::College {
            if let Some(r) = post_rec.as_ref() {
                kv.insert(0, ("Record".into(), r.clone()));
            }
            kv.push(("Bowl season".into(), span_text(y.bowl_season.0, y.bowl_season.1)));
        }
        let mut s = Phase::new(0, a, z, &name);
        s.kv = kv;
        out.push(s);
    }
    out
}

/// Recruiting counts for a college in one class: commits, signed, official
/// visits and offers out, early offers included as the recruiting screens
/// count them (all public).
fn recruit_counts(w: &World, prog: usize, class: i32) -> (u32, u32, u32, u32) {
    let (mut commits, mut signed, mut official, mut offers) = (0, 0, 0, 0);
    for p in w.people.iter().filter(|p| p.class_of == class && matches!(p.stage, Stage::HighSchool | Stage::PrepSchool))
    {
        if p.cal.signed.is_some_and(|s| s.program == prog) {
            signed += 1;
        } else if p.cal.commit.is_some_and(|c| c.0 == prog) {
            commits += 1;
        }
        if p.cal.visits.iter().any(|v| v.0 == prog && v.2 == Visit::Official) {
            official += 1;
        }
        // `views::offers_of`, without building the list for every boy.
        if p.cal.early_offers.iter().any(|o| o.0 == prog)
            || p.cal.offers.contains(&prog)
            || p.cal.signed.is_some_and(|s| !s.walk_on && s.program == prog)
            || p.cal.commit.is_some_and(|c| !c.2 && c.0 == prog)
        {
            offers += 1;
        }
    }
    (commits, signed, official, offers)
}

/// The roster and recruiting lanes by level.
fn dates_lanes(w: &World, level: CalLevel, team: usize, y: &YearDates, page: Option<&SeasonPage>) -> Vec<Phase> {
    let s = y.season;
    let mut out = Vec::new();
    match level {
        CalLevel::Nfl => {
            out.push(
                Phase::new(1, y.league_year.date, y.league_year.date.add(2), "Free agency")
                    .kv("Opens", span_text(y.league_year.date, y.league_year.date))
                    .kv("Tag deadline", span_text(y.tag_deadline.date, y.tag_deadline.date)),
            );
            out.push(
                Phase::new(1, y.draft[0].date, y.draft[2].date, "NFL draft")
                    .kv("Round 1", span_text(y.draft[0].date, y.draft[0].date))
                    .kv("Rounds 4 to 7", span_text(y.draft[2].date, y.draft[2].date))
                    .kv("Undrafted rush", "right after the last pick"),
            );
            out.push(
                Phase::new(1, y.rookie_minicamp, y.rookie_minicamp.add(2), "Rookie minicamp")
                    .kv("Who", "the draft class and the undrafted signings"),
            );
            out.push(
                Phase::new(1, y.cutdown.date, y.cutdown.date, "Cutdown to 53")
                    .kv("Limit", "90 in camp, 53 on the roster")
                    .kv("Practice squads", span_text(y.practice_squads, y.practice_squads)),
            );
            out.push(
                Phase::new(1, y.trade_deadline.date, y.trade_deadline.date, "Trade deadline")
                    .kv("After it", "free agents and waivers only"),
            );
            out.push(
                Phase::new(2, y.combine_start, y.combine_end, "Combine")
                    .kv("Where", "Indianapolis")
                    .kv("Invites", "about 330 prospects"),
            );
            out.push(Phase::new(2, y.pro_days.0, y.pro_days.1, "Pro days").kv("Where", "each school's campus"));
            // The visits run beside the pro days: they sit on the roster
            // lane, between free agency and the draft.
            out.push(
                Phase::new(1, y.pro_days.0.add(14), y.top30_deadline, "Top 30 visits")
                    .kv("Allowed", "30 prospects at your facility")
                    .kv("Deadline", span_text(y.top30_deadline, y.top30_deadline)),
            );
            out.push(
                Phase::new(2, y.shrine_bowl, y.senior_bowl, "All-star games")
                    .kv("Senior Bowl", span_text(y.senior_bowl, y.senior_bowl))
                    .kv("Next class", format!("the {} draft", s + 1)),
            );
        }
        CalLevel::HighSchool => {
            if let Some(d) = w.hs_class_day() {
                out.push(Phase::new(1, d, d, "The new class").kv("Who", "this fall's freshmen join the program"));
            }
            out.push(
                Phase::new(1, y.hs_school_start, y.hs_school_start, "School starts")
                    .kv("Eligibility", "grades from the spring count"),
            );
            // The juniors this spring, the seniors when they sign: the class of
            // next spring.
            let roster: Vec<usize> = w.high_schools.get(team).map_or_else(Vec::new, |h| {
                h.roster.iter().copied().filter(|&i| w.people[i].class_of == s + 1).collect()
            });
            let with_offers = roster.iter().filter(|&&i| !views::offers_of(&w.people[i]).is_empty()).count();
            let committed = roster.iter().filter(|&&i| w.people[i].cal.commit.is_some()).count();
            out.push(
                Phase::new(2, y.junior_days.0, y.junior_days.1, "Junior days")
                    .kv("Who", "your juniors visit colleges")
                    .kv("With offers", with_offers.to_string()),
            );
            out.push(
                Phase::new(2, Date::ymd(s, 4, 15), Date::ymd(s, 5, 31), "Spring evaluation")
                    .kv("Colleges", "coaches watch practice, no talking with players")
                    .kv(
                        "Official visits",
                        format!("open from {}", span_text(y.official_visits_from, y.official_visits_from)),
                    ),
            );
            out.push(
                Phase::new(2, Date::ymd(s, 6, 1), Date::ymd(s, 6, 21), "Camps and visits")
                    .kv("June", "college camps and official visits"),
            );
            if let Some(e) = y.early_signing {
                out.push(Phase::new(2, e, e.add(2), "Early signing").kv("Committed", committed.to_string()));
            }
            out.push(Phase::new(2, y.signing_day, y.signing_day, "Signing day").kv("Committed", committed.to_string()));
        }
        _ => {
            // Junior days, the June visits and both signing dates all work
            // on the class of next spring.
            let (commits, signed, official, offers) = recruit_counts(w, team, s + 1);
            out.push(
                Phase::new(1, y.draft[0].date, y.draft[2].date, "NFL draft")
                    .kv("Rounds", "7 over three days")
                    .kv("Declare by", span_text(y.declare_deadline, y.declare_deadline)),
            );
            out.push(
                Phase::new(1, y.summer_enrollment, y.summer_enrollment, "Freshmen arrive")
                    .kv("Early enrollees", format!("since {}", span_text(Date::ymd(s, 1, 10), Date::ymd(s, 1, 10)))),
            );
            out.push(
                Phase::new(1, y.portal_open, y.portal_close, "Transfer portal")
                    .kv("Window", format!("{} days", y.portal_open.days_to(y.portal_close) + 1))
                    .kv("Rule", "players enter only inside the window"),
            );
            out.push(
                Phase::new(2, y.junior_days.0, y.junior_days.1, "Junior days")
                    .kv("Who", format!("the class of {}", s + 1))
                    .kv("Offers out", offers.to_string()),
            );
            out.push(
                Phase::new(2, Date::ymd(s, 4, 15), Date::ymd(s, 5, 31), "Spring evaluation")
                    .kv("Contact", "none with the player off campus")
                    .kv("Who goes", "your assistants, to high schools"),
            );
            out.push(
                Phase::new(2, Date::ymd(s, 6, 1), Date::ymd(s, 6, 21), "June official visits")
                    .kv("Booked", official.to_string())
                    .kv("Rule", "one official visit per school"),
            );
            if let Some(p) = page {
                let home: Vec<Date> =
                    p.games.iter().filter(|g| g.home && !g.neutral && !g.bowl && !g.playoff).map(|g| g.date).collect();
                if let (Some(&a), Some(&b)) = (home.first(), home.last()) {
                    out.push(
                        Phase::new(2, a, b, "Home game visits")
                            .kv("Home games", home.len().to_string())
                            .kv("Rule", "official visits on home Saturdays"),
                    );
                }
            }
            if let Some(e) = y.early_signing {
                out.push(
                    Phase::new(2, e, e.add(2), "Early signing")
                        .kv("Commits", commits.to_string())
                        .kv("Signed", signed.to_string()),
                );
            }
            out.push(
                Phase::new(2, y.signing_day, y.signing_day, "Signing day")
                    .kv("Commits", commits.to_string())
                    .kv("Signed", signed.to_string()),
            );
        }
    }
    out
}

/// The rules lane, day by day: the recruiting periods (college and high
/// school), the CBA's offseason at the NFL. `run_to`: the last day of an
/// NFL playoff run, the season's rules till then.
fn rules_lane(level: CalLevel, frame: &Frame, y: &YearDates, run_to: Option<Date>) -> Vec<(Date, Date, &'static str)> {
    let mut out: Vec<(Date, Date, &'static str)> = Vec::new();
    let mut d = frame.start;
    while d <= frame.end {
        let k = if level == CalLevel::Nfl {
            match frame.block(d) {
                Block::Season | Block::Camp => "c",
                _ if d >= frame.winter && run_to.is_some_and(|e| d <= e) => "c",
                Block::Otas if d >= y.minicamp.0 && d <= y.minicamp.1 => "c",
                Block::Otas | Block::Phase1 | Block::Phase2 => "e",
                _ => "d",
            }
        } else {
            match recruiting_period(y, d) {
                Period::Contact => "c",
                Period::Evaluation => "e",
                Period::Quiet => "q",
                Period::Dead => "d",
            }
        };
        match out.last_mut() {
            Some(s) if s.2 == k => s.1 = d,
            _ => out.push((d, d, k)),
        }
        d = d.add(1);
    }
    out
}

/// One practice day as blocks: practice, film, meetings, the weight room,
/// the game. Treatment, study hall and outings aren't football work and
/// stay off the board (the training week shows them).
fn day_blocks(day: &afm_engine::training::plan::Day, game: Option<String>) -> Vec<Value> {
    let mut out: Vec<(&str, String, u32, Gear)> = Vec::new();
    for p in &day.periods {
        let (k, n) = match p.kind {
            Kind::Recovery | Kind::StudyHall | Kind::TeamBuilding => continue,
            Kind::Film | Kind::VrReps => ("film", "Film"),
            Kind::HomeStudy => ("film", "Home study"),
            Kind::Walkthrough => ("meet", "Walk-through"),
            Kind::WeightRoom | Kind::Testing => ("lift", "Weight room"),
            Kind::Conditioning | Kind::Speed | Kind::MatDrills | Kind::Mobility => ("lift", "Conditioning"),
            // Meetings, installs, the plan test, signals, the rules clinic,
            // vision, the psychologist, the dietitian and the council.
            kind if !kind.on_field() => ("meet", "Meetings"),
            _ => ("prac", "Practice"),
        };
        match out.last_mut() {
            Some(x) if x.0 == k && x.1 == n => {
                x.2 += p.min as u32;
                x.3 = x.3.max(p.gear);
            }
            _ => out.push((k, n.to_string(), p.min as u32, p.gear)),
        }
    }
    let mut v: Vec<Value> = out
        .into_iter()
        .map(|(k, n, min, gear)| {
            let n =
                if k == "prac" && gear != Gear::None { format!("{n} · {}", gear.label().to_lowercase()) } else { n };
            json!({ "k": k, "n": n, "min": min })
        })
        .collect();
    if let Some(g) = game {
        v.push(json!({ "k": "game", "n": g, "min": 180 }));
    }
    v
}

pub(super) fn year(w: &World) -> Reply {
    let t = views::mine(w).ok_or("You have no team.")?;
    let (level, team) = match t {
        TeamRef::College(p) => (CalLevel::College, p),
        TeamRef::Nfl(c) => (CalLevel::Nfl, c),
        TeamRef::HighSchool(h) => (CalLevel::HighSchool, h),
        TeamRef::Other(_) => return Err("The year covers the NFL, college and high school.".into()),
    };
    let y = &w.dates;
    let st = w.training_team().ok_or("You have no team.")?;
    let frame = st.frame;
    let (start, end) = (frame.start, frame.end);
    let span = start.days_to(end).max(1) as f64;
    let x = |d: Date| ((start.days_to(d) as f64 / span).clamp(0.0, 1.0) * 10_000.0).round() / 10_000.0;
    let page = sp::page(w, level, team, Held::This).filter(|p| p.year == y.season);
    let nfl = level == CalLevel::Nfl;
    // The NFL's dates come out in May: until then the season is its opponents.
    let hidden = nfl && w.today < y.schedule_release;

    // The months along the top.
    let mut months = Vec::new();
    let mut m = Date::ymd(start.year(), start.month(), 1);
    while m <= end {
        if m >= start {
            months.push(json!({ "n": MONTHS[m.month() as usize - 1], "x": x(m) }));
        }
        m = if m.month() == 12 { Date::ymd(m.year() + 1, 1, 1) } else { Date::ymd(m.year(), m.month() + 1, 1) };
    }

    // Your games.
    let next_i =
        page.as_ref().and_then(|p| p.games.iter().position(|g| g.score.is_none() && g.won.is_none() && !g.cancelled));
    let games: Vec<Value> = page.as_ref().filter(|_| !hidden).map_or_else(Vec::new, |p| {
        p.games
            .iter()
            .enumerate()
            .map(|(k, g)| {
                let opp = views::team_name(w, super::same_level(t, g.opp));
                let r = match (g.score, g.won) {
                    (Some([a, b]), _) => {
                        if a > b {
                            "W"
                        } else if a < b {
                            "L"
                        } else {
                            "T"
                        }
                    }
                    (None, Some(won)) => {
                        if won {
                            "W"
                        } else {
                            "L"
                        }
                    }
                    _ => "",
                };
                let at = if g.neutral || g.home { "vs" } else { "at" };
                let score = g.score.map_or(String::new(), |[a, b]| format!(" {r} {a}-{b}"));
                json!({
                    "i": k, "x": x(g.date), "o": code(&opp, nfl), "r": r, "next": Some(k) == next_i,
                    "post": g.playoff || g.bowl,
                    "title": format!("{} · {at} {opp}{score}", views::date_text(g.date)),
                })
            })
            .collect()
    });

    // The bars.
    let mut phases = football(&frame, y, page.as_ref(), st.state, st.heat);
    phases.extend(dates_lanes(w, level, team, y, page.as_ref()));
    phases.retain(|p| p.b >= start && p.a <= end);
    let today = w.today;
    let cur = phases.iter().position(|p| p.lane == 0 && p.a <= today && p.b >= today).unwrap_or(0);
    let lanes = match level {
        CalLevel::Nfl => ["Games", "Football", "Roster", "Scouting", "The CBA"],
        CalLevel::HighSchool => ["Games", "Football", "School", "Recruiting", "Rules"],
        _ => ["Games", "Football", "Roster", "Recruiting", "Rules"],
    };
    let phase_json: Vec<Value> = phases
        .iter()
        .map(|p| {
            json!({
                "lane": p.lane, "x": x(p.a), "w": x(p.b.add(1).min(end)) - x(p.a),
                "n": p.name, "s": short(&p.name), "d": span_text(p.a.max(start), p.b.min(end)),
                "cur": p.a <= today && p.b >= today, "past": p.b < today,
                "kv": p.kv.iter().map(|(k, v)| json!({ "k": k, "v": v })).collect::<Vec<_>>(),
                "lane_name": lanes[p.lane as usize + 1],
            })
        })
        .collect();
    // An NFL playoff run: to the game it ended, or to the Super Bowl while
    // the team is alive.
    let run_to = page.as_ref().filter(|_| nfl).and_then(|p| {
        let g = p.games.iter().rev().find(|g| g.playoff)?;
        let lost = match (g.score, g.won) {
            (Some([a, b]), _) => a < b,
            (None, Some(won)) => !won,
            _ => false,
        };
        Some(if lost { g.date } else { y.super_bowl })
    });
    let rules: Vec<Value> = rules_lane(level, &frame, y, run_to)
        .into_iter()
        .map(|(a, b, k)| json!({ "x": x(a), "w": x(b.add(1).min(end)) - x(a), "k": k, "d": span_text(a, b) }))
        .collect();
    let legend: Vec<Value> = if nfl {
        vec![json!(["c", "mandatory"]), json!(["e", "voluntary"]), json!(["d", "on their own"])]
    } else {
        vec![json!(["c", "contact"]), json!(["e", "evaluation"]), json!(["q", "quiet"]), json!(["d", "dead"])]
    };

    // The head: record, rank or place, next game.
    let (overall, league) = page.as_ref().map_or(([0u32; 3], [0u32; 3]), |p| {
        p.games.iter().fold(([0u32; 3], [0u32; 3]), |(mut o, mut l), g| {
            let k = match (g.score, g.won) {
                (Some([a, b]), _) => Some(if a > b {
                    0
                } else if a < b {
                    1
                } else {
                    2
                }),
                (None, Some(won)) => Some(if won { 0 } else { 1 }),
                _ => None,
            };
            if let Some(k) = k {
                o[k] += 1;
                if g.league {
                    l[k] += 1;
                }
            }
            (o, l)
        })
    });
    let league_word = page.as_ref().map_or(String::new(), |p| match level {
        CalLevel::Nfl => "division".to_string(),
        CalLevel::HighSchool => "district".to_string(),
        _ => p.league_name.clone(),
    });
    let record = if overall.iter().sum::<u32>() == 0 {
        "0-0".to_string()
    } else if league.iter().sum::<u32>() > 0 {
        format!("{} · {} {league_word}", wlt(overall), wlt(league))
    } else {
        wlt(overall)
    };
    let ranked = match level {
        CalLevel::Nfl => page
            .as_ref()
            .and_then(|p| p.place)
            .filter(|_| overall.iter().sum::<u32>() > 0)
            .map(|(a, _)| format!("{} in the {}", ordinal(a), p_league(page.as_ref()))),
        // The class poll from the page built above; college's from the polls.
        CalLevel::HighSchool => page.as_ref().and_then(sp::hs_rank_on).map(|(k, poll)| format!("No. {k} in {poll}")),
        _ => sp::rank_now(w, level, team).map(|(k, poll)| {
            if level == CalLevel::HighSchool {
                format!("No. {k} in {poll}")
            } else {
                format!("{} {poll}", ordinal(k as u32))
            }
        }),
    };
    let next_game = next_i.filter(|_| !hidden).and_then(|k| page.as_ref().map(|p| &p.games[k]));
    let next = next_game.map(|g| {
        let opp = views::team_name(w, super::same_level(t, g.opp));
        format!("{}{opp}, {}", if g.home || g.neutral { "" } else { "at " }, g.date.weekday().short())
    });

    // This week, as your staff has it planned.
    let monday = today.on_or_before(Weekday::Mon);
    let pw = ts::planned(w, &st, monday);
    let game_on = |d: Date| -> Option<String> {
        page.as_ref().and_then(|p| p.games.iter().find(|g| g.date == d)).map(|g| {
            let opp = views::team_name(w, super::same_level(t, g.opp));
            let at = if g.home || g.neutral { "" } else { "at " };
            match g.kick {
                Some(k) => format!("{at}{opp} · {}", kick_text(k, level)),
                None => format!("{at}{opp}"),
            }
        })
    };
    let days: Vec<Value> = pw
        .week
        .days
        .iter()
        .enumerate()
        .map(|(k, d)| {
            let date = monday.add(k as i32);
            let game = if d.game { game_on(date).or(Some("Game".to_string())) } else { None };
            json!({
                "n": format!("{} {}", date.weekday().short(), date.day()),
                "today": date == today,
                "rest": d.rest(),
                "blocks": day_blocks(d, game),
            })
        })
        .collect();
    let used: u32 = pw.week.days.iter().map(|d| d.countable()).sum();
    let allowed = pw.books.iter().find_map(|b| b.week_min).map(|m| m as u32);
    let field: u32 = pw.week.days.iter().map(|d| d.field()).sum();
    let sunday = monday.add(6);
    let mut week_title = if monday.month() == sunday.month() {
        format!("{} {} to {}", monday.weekday().short(), monday.day(), span_text(sunday, sunday))
    } else {
        span_text(monday, sunday)
    };
    if let Some(g) = (0..7).find_map(|k| game_on(monday.add(k)).map(|s| (monday.add(k), s))) {
        week_title.push_str(&format!(" · {} on {}", g.1, g.0.weekday().short()));
    }
    let week = json!({
        "title": week_title,
        "block": pw.block.label(st.tier),
        "hours": allowed.map(|a| json!({ "used": (used as f64 / 60.0).round(), "allowed": a / 60 })),
        "field": hours(field),
        "days": days,
        "yours": pw.yours,
    });

    // Coming up: your next games and what starts next.
    let mut ahead: Vec<(Date, String, String)> = Vec::new();
    if let Some(p) = page.as_ref().filter(|_| !hidden) {
        let first = p.games.first().map(|x| x.date);
        for g in p.games.iter().filter(|g| g.date >= today && g.score.is_none() && g.won.is_none()).take(3) {
            let out = super::announced(w, level, g, first);
            let opp = views::team_name(w, super::same_level(t, g.opp));
            let at = if g.home || g.neutral { "" } else { "at " };
            let mut s: Vec<String> = Vec::new();
            if let Some(n) = g.name.as_ref() {
                s.push(n.clone());
            }
            if let Some(k) = g.kick.filter(|_| out) {
                s.push(kick_text(k, level));
            }
            if !g.tv.is_empty() && out {
                s.push(g.tv.clone());
            }
            ahead.push((
                g.date,
                format!("{at}{opp}"),
                if s.is_empty() { "Kickoff TBA".to_string() } else { s.join(" · ") },
            ));
        }
    }
    for p in phases.iter().filter(|p| p.a > today) {
        ahead.push((p.a, p.name.clone(), format!("{} · {}", lanes[p.lane as usize + 1], span_text(p.a, p.b))));
    }
    ahead.sort_by_key(|a| a.0);
    let ahead: Vec<Value> = ahead
        .into_iter()
        .take(5)
        .map(|(d, t, s)| json!({ "d": d.day(), "m": MONTHS[d.month() as usize - 1], "t": t, "s": s, "in": today.days_to(d) }))
        .collect();

    Ok(json!({
        "team": views::team_brief(w, t),
        "level": match level { CalLevel::Nfl => "nfl", CalLevel::HighSchool => "hs", _ => "college" },
        "season": y.season,
        "from": views::date_text(start),
        "to": views::date_text(end),
        "next_year": views::date_text(end.add(1)),
        "today": views::date_text(today),
        "today_x": x(today),
        "record": record,
        "ranked": ranked,
        "next": next,
        "months": months,
        "lanes": lanes,
        "games": games,
        "games_note": if hidden { Some(format!("Dates out {}", views::date_text(y.schedule_release))) } else { None },
        "phases": phase_json,
        "cur": cur,
        "rules": rules,
        "legend": legend,
        "week": week,
        "ahead": ahead,
    }))
}

/// The NFL division's name for the head.
fn p_league(p: Option<&SeasonPage>) -> String {
    p.map_or(String::new(), |p| p.league_name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_and_bars_are_labelled_short() {
        assert_eq!(code("Oklahoma State", false), "OKST");
        assert_eq!(code("TCU", false), "TCU");
        assert_eq!(code("Chicago Bears", true), "CHI");
        assert_eq!(code("New York Giants", true), "NYG");
        assert_eq!(code("Los Angeles Rams", true), "LAR");
        assert_eq!(short("Regular season · 12 games"), "Season");
        assert_eq!(short("Season · 1 game"), "Season");
        assert_eq!(short("Bowl practice and December"), "Bowls");
        assert_eq!(short("Combine"), "Combine");
    }

    /// The football lane runs the whole year, block after block, and every
    /// block's card has its facts.
    #[test]
    fn the_football_lane_covers_the_year_once() {
        let y = YearDates::new(2026);
        let frames = [
            Frame::college(&y, y.title_game, true),
            Frame::pro(&y, false),
            Frame::high(&y, Date::ymd(2026, 8, 28), Date::ymd(2026, 12, 19), true),
        ];
        for frame in frames {
            let bars = football(&frame, &y, None, "TX", false);
            assert_eq!(bars.first().map(|b| b.a), Some(frame.start));
            assert_eq!(bars.last().map(|b| b.b), Some(frame.end));
            for w in bars.windows(2) {
                assert_eq!(w[0].b.add(1), w[1].a, "{} then {}", w[0].name, w[1].name);
            }
            for b in &bars {
                assert!(!b.kv.is_empty(), "{} has no facts", b.name);
            }
        }
    }
}
