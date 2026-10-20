//! The screens' data (section 8, topic 5): each function builds one screen
//! as JSON from the world, showing only what your staff could know.

use afm_engine::attributes::Category;
use afm_engine::calendar::{Date, Level as CalLevel, Loudness};
use afm_engine::player::{Level, PlayingTime};
use afm_engine::positions::Position;
use afm_engine::world::coach::{Employer, Group, Role};
use afm_engine::world::{Person, Stage, World};
use serde_json::{Value, json};

use crate::colors;
use crate::reads;

/// A team the screens are about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeamRef {
    Nfl(usize),
    College(usize),
    HighSchool(usize),
}

impl TeamRef {
    /// From `{kind, id}`, or your own team when none is given.
    pub fn from_args(w: &World, args: &Value) -> Result<TeamRef, String> {
        let id = args.get("id").and_then(Value::as_u64).map(|v| v as usize);
        match (args.get("kind").and_then(Value::as_str), id) {
            (Some("nfl"), Some(i)) if i < w.nfl.len() => Ok(TeamRef::Nfl(i)),
            (Some("college"), Some(i)) if i < w.programs.len() => Ok(TeamRef::College(i)),
            (Some("hs"), Some(i)) if i < w.high_schools.len() => Ok(TeamRef::HighSchool(i)),
            (Some(k), _) => Err(format!("No such team: {k} {id:?}")),
            (None, _) => mine(w).ok_or_else(|| "You have no team yet.".to_string()),
        }
    }

    pub fn json(self) -> Value {
        match self {
            TeamRef::Nfl(i) => json!({ "kind": "nfl", "id": i }),
            TeamRef::College(i) => json!({ "kind": "college", "id": i }),
            TeamRef::HighSchool(i) => json!({ "kind": "hs", "id": i }),
        }
    }
}

/// Your team.
pub fn mine(w: &World) -> Option<TeamRef> {
    let s = w.seat?;
    Some(match s.level {
        CalLevel::Nfl => TeamRef::Nfl(s.nfl_team?),
        CalLevel::HighSchool => TeamRef::HighSchool(s.program?),
        _ => TeamRef::College(s.program?),
    })
}

pub fn date_text(d: Date) -> String {
    let (y, m, day) = d.parts();
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!("{} {} {} {}", d.weekday().short(), day, M[(m - 1) as usize], y)
}

pub fn team_name(w: &World, t: TeamRef) -> String {
    match t {
        TeamRef::Nfl(i) => w.nfl_team_name(i),
        TeamRef::College(i) => w.program_name(i).to_string(),
        TeamRef::HighSchool(i) => w.school_name(i).to_string(),
    }
}

pub fn team_colors(w: &World, t: TeamRef) -> [String; 2] {
    match t {
        TeamRef::Nfl(i) => colors::nfl(&w.nfl_team_name(i), i),
        TeamRef::College(i) => colors::college(&w.programs[i].real_name, i),
        TeamRef::HighSchool(i) => colors::high_school(i),
    }
}

fn team_brief(w: &World, t: TeamRef) -> Value {
    let [c1, c2] = team_colors(w, t);
    json!({ "ref": t.json(), "name": team_name(w, t), "colors": [c1, c2], "mine": is_mine(w, t) })
}

fn record_text(w: &World, t: TeamRef) -> String {
    match t {
        TeamRef::Nfl(i) => {
            let (wi, l, ti) = w.nfl_record(i);
            if ti > 0 { format!("{wi}-{l}-{ti}") } else { format!("{wi}-{l}") }
        }
        TeamRef::College(i) => {
            let r = w.college_record(i);
            format!("{}-{} ({}-{} conf)", r.wins, r.losses, r.conf_wins, r.conf_losses)
        }
        TeamRef::HighSchool(i) => {
            let s = &w.high_schools[i];
            s.seasons.last().map_or("0-0".to_string(), |h| format!("{}-{} ({})", h.wins, h.losses, h.year))
        }
    }
}

/// The people on a team's books, as indices into `people`.
pub fn roster_people(w: &World, t: TeamRef) -> Vec<usize> {
    let mut v: Vec<usize> = match t {
        TeamRef::Nfl(i) => w.clubs.get(i).map_or(Vec::new(), |c| c.roster.clone()),
        TeamRef::College(i) => w.programs[i].roster.clone(),
        TeamRef::HighSchool(i) => w.high_schools[i].roster.clone(),
    };
    v.retain(|&k| k < w.people.len());
    v
}

fn is_mine(w: &World, t: TeamRef) -> bool {
    mine(w) == Some(t)
}

/// The eye of the coach who reads a player on your staff (1-20): his
/// position coach, else the coordinator, else you. Used only to size the
/// range, never shown.
fn eye_for(w: &World, t: TeamRef, pos: Position) -> f64 {
    let g = Group::of(pos);
    let coach = match t {
        TeamRef::College(i) => {
            let p = &w.programs[i];
            p.position_coaches[g as usize].or(if g.offense() { p.oc } else { p.dc }).or(p.head_coach)
        }
        TeamRef::Nfl(i) => {
            let tm = &w.nfl[i];
            if g.offense() { tm.oc } else { tm.dc }.or(tm.head_coach)
        }
        TeamRef::HighSchool(i) => w.high_schools[i].coach,
    };
    coach.and_then(|c| w.coaches.get(c)).map_or(10.0, |c| c.evaluation())
}

fn reader_key(w: &World) -> u64 {
    w.career.as_ref().map_or(0x5EED, |c| c.seed as u64 ^ (c.coach as u64) << 32)
}

/// His class or years, in the words of his level.
fn class_text(w: &World, p: &Person) -> String {
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => {
            let grade = 12 - (p.class_of - (w.year + 1)).clamp(0, 3);
            match grade {
                12 => "Sr".into(),
                11 => "Jr".into(),
                10 => "So".into(),
                _ => "Fr".into(),
            }
        }
        Stage::College | Stage::Declared => {
            let c = ["Fr", "So", "Jr", "Sr", "Sr"][(p.college_seasons as usize).min(4)];
            if p.redshirt_used { format!("RS {c}") } else { c.to_string() }
        }
        Stage::Pro => {
            let y = p.pro.as_ref().map_or(0, |c| c.accrued);
            if y == 0 { "Rookie".into() } else { format!("{y} yrs") }
        }
        Stage::Done => "Retired".into(),
    }
}

fn level_of(w: &World, p: &Person) -> Level {
    match p.stage {
        Stage::Pro => Level::NFL,
        Stage::Declared | Stage::Done => match p.college {
            Some(c) if p.stage == Stage::Declared => afm_engine::world::division_level(w.programs[c].division),
            _ => p.player.history.last().map_or(Level::HS, |h| h.level),
        },
        _ => p.level(&w.programs),
    }
}

fn hometown(w: &World, p: &Person) -> String {
    let s = w.high_schools.get(p.hs);
    match s {
        Some(s) => format!("{}, {}", s.town, w.states.get(s.state).map_or("", |x| x.code.as_str())),
        None => w.states.get(p.home_state).map_or(String::new(), |x| x.code.clone()),
    }
}

fn status_text(w: &World, p: &Person) -> String {
    if let Some(c) = &p.pro {
        use afm_engine::world::pro::Status;
        let s = match c.status {
            Status::Roster => "",
            Status::PracticeSquad => "Practice squad",
            Status::Reserve => "Injured reserve",
            Status::FreeAgent => "Free agent",
            Status::Other(_) => "Other league",
        };
        if c.out_weeks > 0 {
            return format!("Injured, {} wk", c.out_weeks);
        }
        return s.to_string();
    }
    if p.cal.portal.is_some() {
        return "In the portal".into();
    }
    if p.cal.trouble {
        return "Suspended".into();
    }
    if p.cal.academic_sit {
        return "Academically ineligible".into();
    }
    if p.sit_out > 0.0 || p.cal.sitting_transfer {
        return "Sitting out (transfer)".into();
    }
    let _ = w;
    String::new()
}

fn person_index(w: &World, id: u32) -> Option<usize> {
    w.people.iter().position(|p| p.player.id == id)
}

fn ft_in(inches: f64) -> String {
    afm_engine::body::height_text(inches)
}

fn mood_words(p: &Person) -> &'static str {
    match p.mood {
        m if m > 0.5 => "Very happy",
        m if m > 0.15 => "Happy",
        m if m > -0.15 => "Settled",
        m if m > -0.5 => "Unhappy",
        _ => "Very unhappy",
    }
}

fn row(w: &World, k: usize, t: TeamRef, own: bool, reader: u64) -> Value {
    let p = &w.people[k];
    let pos = p.player.position;
    let eye = if own { eye_for(w, t, pos) } else { 10.0 };
    let ovr = reads::overall(&p.player, own, eye, reader);
    let notes = w.notes_on(p.player.id).len();
    let starter = p.last_role == Some(PlayingTime::Starter);
    json!({
        "id": p.player.id,
        "name": p.name,
        "pos": pos.name(),
        "group": unit(pos),
        "age": p.player.age,
        "class": class_text(w, p),
        "ht": ft_in(p.player.body.height),
        "wt": p.player.body.weight.round(),
        "stars": p.stars,
        "home": hometown(w, p),
        "ovr": ovr.json(),
        "status": status_text(w, p),
        "mood": mood_words(p),
        "starter": starter,
        "notes": notes,
    })
}

fn unit(pos: Position) -> &'static str {
    if pos.is_specialist() {
        "ST"
    } else if matches!(pos, Position::QB | Position::RB | Position::WR | Position::TE | Position::OT | Position::IOL) {
        "OFF"
    } else {
        "DEF"
    }
}

pub fn status(w: &World) -> Value {
    let next = next_dates(w, 3);
    json!({
        "today": date_text(w.today),
        "season": w.year,
        "mine": mine(w).map(|t| team_brief(w, t)),
        "you": w.career.as_ref().map(|c| c.you.name.clone()),
        "next": next,
        "pinned": pinned(w),
        "names": format!("{:?}", w.options.names),
    })
}

/// The next dated moments on your calendar (for the Continue bar).
fn next_dates(w: &World, n: usize) -> Vec<Value> {
    let seat = w.seat;
    w.calendar
        .events
        .range(w.today..)
        .flat_map(|(d, evs)| evs.iter().map(move |e| (d, e)))
        .filter(|(_, e)| e.loud >= Loudness::Big && seat.is_none_or(|s| s.cares(e)))
        .take(n)
        .map(|(d, e)| json!({ "date": date_text(*d), "in": w.today.days_to(*d), "text": e.text }))
        .collect()
}

fn pinned(w: &World) -> Vec<Value> {
    let seat = w.seat;
    w.calendar
        .events
        .range(w.today.add(-14)..=w.today)
        .flat_map(|(d, evs)| evs.iter().map(move |e| (d, e)))
        .filter(|(_, e)| e.needs_decision && seat.is_some_and(|s| s.cares(e)))
        .map(|(d, e)| json!({ "date": date_text(*d), "text": e.text, "rule": e.rule }))
        .collect()
}

/// After Continue: the days that ran and why it stopped.
pub fn stopped(w: &World, from: Date, at: Date) -> Value {
    let seat = w.seat;
    let why: Vec<Value> = w
        .calendar
        .on(at)
        .iter()
        .filter(|e| {
            e.needs_decision || e.loud == Loudness::Huge || (e.loud == Loudness::Big && seat.is_none_or(|s| s.cares(e)))
        })
        .map(|e| json!({ "text": e.text, "loud": e.loud.name(), "decision": e.needs_decision }))
        .collect();
    json!({ "from": date_text(from), "days": from.days_to(w.today), "stopped_on": date_text(at), "why": why, "status": status(w) })
}

pub fn home(w: &World) -> Value {
    let me = mine(w);
    let team = me.map(|t| {
        let mut v = team_brief(w, t);
        v["record"] = json!(record_text(w, t));
        v["level"] = json!(match t {
            TeamRef::Nfl(_) => "NFL".to_string(),
            TeamRef::College(i) => format!("{} · {}", w.programs[i].division.name(), w.programs[i].conference),
            TeamRef::HighSchool(i) => {
                let s = &w.high_schools[i];
                format!("{} high school · class {}", w.states[s.state].code, s.classification + 1)
            }
        });
        v
    });
    let next_game = me.and_then(|t| next_game(w, t));
    json!({
        "status": status(w),
        "team": team,
        "next_game": next_game,
        "inbox": inbox(w, 21)["items"].clone(),
        "news": news(w, &json!({ "limit": 12 }))["items"].clone(),
        "followed": followed(w),
    })
}

fn next_game(w: &World, t: TeamRef) -> Option<Value> {
    match t {
        TeamRef::College(i) => w
            .college_fixtures(Some(i))
            .into_iter()
            .filter(|g| g.score.is_none() && g.date >= w.today)
            .min_by_key(|g| g.date)
            .map(|g| {
                let home = g.home == i;
                let opp = if home { g.away } else { g.home };
                json!({
                    "date": date_text(g.date),
                    "in": w.today.days_to(g.date),
                    "opp": team_brief(w, TeamRef::College(opp)),
                    "home": home,
                    "name": g.name,
                    "why": g.why,
                })
            }),
        TeamRef::Nfl(i) => w.nfl_fixtures(Some(i)).into_iter().find(|g| !g.played).map(|g| {
            let home = g.home == i;
            let opp = if home { g.away } else { g.home };
            json!({
                "date": g.when.map(date_text),
                "in": g.when.map(|d| w.today.days_to(d)),
                "week": g.week,
                "opp": team_brief(w, TeamRef::Nfl(opp)),
                "home": home,
            })
        }),
        TeamRef::HighSchool(_) => None,
    }
}

/// The players on your "my players" list: everyone you have a note on.
fn followed(w: &World) -> Vec<Value> {
    let Some(c) = &w.career else { return Vec::new() };
    c.notes
        .iter()
        .filter(|(_, l)| !l.is_empty())
        .filter_map(|(id, l)| {
            let k = person_index(w, *id)?;
            let p = &w.people[k];
            Some(json!({ "id": id, "name": p.name, "pos": p.player.position.name(), "where": where_now(w, p), "note": l.last().map(|n| n.text.clone()) }))
        })
        .collect()
}

fn where_now(w: &World, p: &Person) -> String {
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => w.school_name(p.hs).to_string(),
        Stage::College => p.college.map_or("College".into(), |c| w.program_name(c).to_string()),
        Stage::Declared => "Draft pool".into(),
        Stage::Pro => p.pro.as_ref().and_then(|c| c.team).map_or("Free agent".into(), |t| w.nfl_team_name(t)),
        Stage::Done => "Out of football".into(),
    }
}

pub fn inbox(w: &World, days: i32) -> Value {
    let seat = w.seat;
    let mut items: Vec<Value> = w
        .calendar
        .events
        .range(w.today.add(-days)..=w.today)
        .rev()
        .flat_map(|(d, evs)| evs.iter().rev().map(move |e| (d, e)))
        .filter(|(_, e)| e.loud >= Loudness::Medium && seat.is_none_or(|s| s.cares(e) || e.loud == Loudness::Huge))
        .take(80)
        .map(|(d, e)| {
            json!({ "date": date_text(*d), "loud": e.loud.name(), "text": e.text, "rule": e.rule, "decision": e.needs_decision })
        })
        .collect();
    // Your staff's own messages to you (the program's inbox channel).
    if let Some(TeamRef::College(p)) = mine(w) {
        for n in w.news.iter().rev().filter(|n| n.program == Some(p)).take(20) {
            items.push(json!({ "date": n.date.map(date_text), "loud": "Medium", "text": n.text, "rule": "", "decision": false }));
        }
    }
    json!({ "items": items })
}

pub fn news(w: &World, args: &Value) -> Value {
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(120) as usize;
    let mine_only = args.get("mine").and_then(Value::as_bool).unwrap_or(false);
    let my_prog = match mine(w) {
        Some(TeamRef::College(p)) => Some(p),
        _ => None,
    };
    let mut items: Vec<Value> = Vec::new();
    // The calendar's small lines are the news wire.
    for (d, evs) in w.calendar.events.range(..=w.today).rev() {
        for e in evs.iter().rev() {
            if mine_only && !w.seat.is_some_and(|s| s.cares(e)) {
                continue;
            }
            items.push(json!({ "date": date_text(*d), "text": e.text, "level": format!("{:?}", e.level), "loud": e.loud.name() }));
            if items.len() >= limit {
                return json!({ "items": items });
            }
        }
    }
    for n in w.news.iter().rev() {
        if mine_only && my_prog != n.program {
            continue;
        }
        items.push(json!({ "date": n.date.map(date_text), "text": n.text, "level": "College", "loud": "Small" }));
        if items.len() >= limit {
            break;
        }
    }
    json!({ "items": items })
}

/// Your level's year: every dated moment from today to the end of the
/// football year, yours marked.
pub fn calendar(w: &World) -> Value {
    use afm_engine::calendar::{Level, YearDates};
    let seat = w.seat;
    let level = seat.map(|s| s.level);
    let mut items: Vec<(Date, String, &str, bool)> = Vec::new();
    // The year's fixed dates, for this season and the next.
    for year in [w.year, w.year + 1] {
        let y = YearDates::new(year);
        let mut add = |d: Date, text: String, lv: Level| {
            let everyone = matches!(text.as_str(), "Super Bowl" | "NFL draft, round 1" | "National championship game");
            if everyone || level.is_none_or(|l| l == lv) {
                items.push((d, text, "date", false));
            }
        };
        add(y.super_bowl, "Super Bowl".into(), Level::Nfl);
        add(y.combine_start, "NFL combine".into(), Level::Nfl);
        add(y.league_year.date, "The NFL's new league year: free agency opens".into(), Level::Nfl);
        add(y.draft[0].date, "NFL draft, round 1".into(), Level::Nfl);
        add(y.draft[1].date, "NFL draft, rounds 2 and 3".into(), Level::Nfl);
        add(y.draft[2].date, "NFL draft, rounds 4 to 7".into(), Level::Nfl);
        add(y.otas.0, "Organized team activities start".into(), Level::Nfl);
        add(y.minicamp.0, "Mandatory minicamp".into(), Level::Nfl);
        add(y.rookies_report, "Rookies report to camp".into(), Level::Nfl);
        add(y.veterans_report, "Veterans report to camp".into(), Level::Nfl);
        add(y.cutdown.date, "Cutdown day: 90 to 53".into(), Level::Nfl);
        add(y.nfl_kickoff, "NFL kickoff game".into(), Level::Nfl);
        add(y.trade_deadline.date, "NFL trade deadline".into(), Level::Nfl);
        add(y.black_monday, "Black Monday".into(), Level::Nfl);
        add(y.wild_card, "NFL wild card round".into(), Level::Nfl);
        add(y.spring_practice.0, "Spring practice opens".into(), Level::College);
        add(y.spring_game, "Spring game".into(), Level::College);
        add(y.media_days.0, "Conference media days".into(), Level::College);
        add(y.fall_camp, "Fall camp opens".into(), Level::College);
        add(y.portal_open, "Transfer portal opens".into(), Level::College);
        add(y.portal_close, "Transfer portal closes".into(), Level::College);
        add(y.rivalry_week, "Rivalry week".into(), Level::College);
        add(y.conference_championships, "Conference championships".into(), Level::College);
        add(y.selection_day, "Playoff selection day".into(), Level::College);
        add(y.heisman, "Heisman ceremony".into(), Level::College);
        add(y.cfp_first_round.0, "College Football Playoff, first round".into(), Level::College);
        add(y.title_game, "National championship game".into(), Level::College);
        add(y.declare_deadline, "Deadline to declare for the NFL draft".into(), Level::College);
        if let Some(d) = y.early_signing {
            add(d, "Early signing period opens".into(), Level::College);
        }
        add(y.signing_day, "National signing day".into(), Level::College);
        add(y.hs_school_start, "High school year starts".into(), Level::HighSchool);
        add(y.official_visits_from, "Official visits open for juniors".into(), Level::HighSchool);
        add(y.all_american_bowl, "All-American Bowl".into(), Level::HighSchool);
        add(y.signing_day, "National signing day".into(), Level::HighSchool);
    }
    // Your games.
    if let Some(t) = mine(w) {
        for g in schedule_rows(w, t) {
            items.push((g.0, g.1, "game", true));
        }
    }
    // Moments already set for days ahead.
    for (d, evs) in w.calendar.events.range(w.today..) {
        for e in evs.iter().filter(|e| e.loud >= Loudness::Big) {
            items.push((*d, e.text.to_string(), e.loud.name(), seat.is_some_and(|s| s.cares(e))));
        }
    }
    items.retain(|x| x.0 >= w.today);
    items.sort_by_key(|x| x.0);
    items.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
    let items: Vec<Value> = items
        .into_iter()
        .take(160)
        .map(|(d, text, loud, mine)| json!({ "date": date_text(d), "in": w.today.days_to(d), "text": text, "loud": loud, "mine": mine }))
        .collect();
    json!({ "items": items })
}

/// Your team's games ahead as (date, text).
fn schedule_rows(w: &World, t: TeamRef) -> Vec<(Date, String)> {
    match t {
        TeamRef::College(p) => w
            .college_fixtures(Some(p))
            .into_iter()
            .filter(|f| f.score.is_none())
            .map(|f| {
                let home = f.home == p;
                let opp = if home { f.away } else { f.home };
                let at = if home || f.neutral { "vs" } else { "at" };
                (f.date, format!("{at} {}", team_name(w, TeamRef::College(opp))))
            })
            .collect(),
        TeamRef::Nfl(c) => w
            .nfl_fixtures(Some(c))
            .into_iter()
            .filter(|f| !f.played)
            .filter_map(|f| {
                let home = f.home == c;
                let opp = if home { f.away } else { f.home };
                Some((f.when?, format!("{} {}", if home { "vs" } else { "at" }, w.nfl_team_name(opp))))
            })
            .collect(),
        TeamRef::HighSchool(_) => Vec::new(),
    }
}

pub fn team(w: &World, t: TeamRef) -> Value {
    let mut v = team_brief(w, t);
    v["record"] = json!(record_text(w, t));
    v["mine"] = json!(is_mine(w, t));
    v["next_game"] = next_game(w, t).unwrap_or(Value::Null);
    match t {
        TeamRef::College(i) => {
            let p = &w.programs[i];
            v["level"] = json!(format!("{} · {}", p.division.name(), p.conference));
            v["place"] = json!(format!("{}, {}", p.city, w.states[p.state].code));
            v["stadium"] = json!(p.stadium);
            v["titles"] = json!(p.titles);
            v["conference_titles"] = json!(p.conference_titles);
            v["prestige"] = json!(prestige_words(p.prestige_rel));
            v["seasons"] = json!(
                p.seasons
                    .iter()
                    .rev()
                    .take(30)
                    .map(|s| json!({
                        "year": s.year,
                        "record": format!("{}-{}", s.wins, s.losses),
                        "conf": s.conference_title,
                        "nat": s.national_title,
                        "coach": s.coach.and_then(|c| w.coaches.get(c)).map(|c| c.name.clone()),
                        "pf": s.totals.points_for, "pa": s.totals.points_against, "g": s.totals.games,
                    }))
                    .collect::<Vec<_>>()
            );
        }
        TeamRef::Nfl(i) => {
            v["level"] = json!("NFL");
            if let Some(c) = w.clubs.get(i) {
                v["roster_count"] = json!(c.roster.len());
            }
            v["titles"] = json!(w.nfl_champions.iter().filter(|(_, t)| *t == i).map(|(y, _)| y).collect::<Vec<_>>());
        }
        TeamRef::HighSchool(i) => {
            let s = &w.high_schools[i];
            v["level"] = json!(format!("High school · {} class {}", w.states[s.state].code, s.classification + 1));
            v["place"] = json!(format!("{}, {}", s.town, w.states[s.state].code));
            v["enrollment"] = json!(s.enrollment);
            v["titles"] = json!(s.state_titles);
            v["private"] = json!(s.private);
            v["seasons"] = json!(
                s.seasons
                    .iter()
                    .rev()
                    .take(30)
                    .map(|h| json!({ "year": h.year, "record": format!("{}-{}", h.wins, h.losses) }))
                    .collect::<Vec<_>>()
            );
        }
    }
    v
}

fn prestige_words(rel: f64) -> &'static str {
    match rel {
        r if r > 0.85 => "A blue blood",
        r if r > 0.65 => "A national power",
        r if r > 0.45 => "A solid program",
        r if r > 0.25 => "Rebuilding",
        _ => "A tough job",
    }
}

pub fn roster(w: &World, t: TeamRef) -> Value {
    let own = is_mine(w, t);
    let reader = reader_key(w);
    let mut rows: Vec<(usize, Value)> = roster_people(w, t)
        .into_iter()
        .map(|k| (w.people[k].player.position.index(), row(w, k, t, own, reader)))
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    json!({ "team": team_brief(w, t), "mine": own, "rows": rows.into_iter().map(|r| r.1).collect::<Vec<_>>() })
}

pub fn schedule(w: &World, t: TeamRef) -> Value {
    let games: Vec<Value> = match t {
        TeamRef::College(i) => {
            let mut g = w.college_fixtures(Some(i));
            g.sort_by_key(|g| g.date);
            g.into_iter()
                .map(|g| {
                    let home = g.home == i;
                    let opp = if home { g.away } else { g.home };
                    let result = g.score.map(|s| {
                        let (me, them) = if home { (s[0], s[1]) } else { (s[1], s[0]) };
                        format!(
                            "{} {}-{}",
                            if me > them {
                                "W"
                            } else if me < them {
                                "L"
                            } else {
                                "T"
                            },
                            me,
                            them
                        )
                    });
                    json!({
                        "date": date_text(g.date),
                        "kick": format!("{}:{:02}", g.kickoff.0, g.kickoff.1),
                        "opp": team_brief(w, TeamRef::College(opp)),
                        "at": if g.neutral { "vs (n)" } else if home { "vs" } else { "at" },
                        "name": g.name,
                        "conf": g.conference,
                        "why": g.why,
                        "result": result,
                    })
                })
                .collect()
        }
        TeamRef::Nfl(i) => w
            .nfl_fixtures(Some(i))
            .into_iter()
            .map(|g| {
                let home = g.home == i;
                let opp = if home { g.away } else { g.home };
                json!({
                    "week": g.week,
                    "date": g.when.map(date_text),
                    "opp": team_brief(w, TeamRef::Nfl(opp)),
                    "at": if home { "vs" } else { "at" },
                    "result": if g.played { Some("Played") } else { None },
                })
            })
            .collect(),
        TeamRef::HighSchool(_) => Vec::new(),
    };
    let kept: Vec<Value> = w
        .seat_games()
        .iter()
        .enumerate()
        .rev()
        .map(|(k, g)| json!({ "index": k, "date": date_text(g.date), "home": g.names[0], "away": g.names[1], "score": g.stats.score }))
        .collect();
    json!({ "team": team_brief(w, t), "games": games, "box_scores": if is_mine(w, t) { kept } else { Vec::new() } })
}

pub fn standings(w: &World, args: &Value) -> Value {
    let conf = args.get("conference").and_then(Value::as_str).map(str::to_string).or_else(|| match mine(w) {
        Some(TeamRef::College(p)) => Some(w.programs[p].conference.clone()),
        _ => None,
    });
    if matches!(mine(w), Some(TeamRef::Nfl(_))) || args.get("kind").and_then(Value::as_str) == Some("nfl") {
        let Some((cf, dv)) = w.nfl_alignment() else { return json!({ "groups": [] }) };
        let names =
            ["AFC East", "AFC North", "AFC South", "AFC West", "NFC East", "NFC North", "NFC South", "NFC West"];
        let groups: Vec<Value> = (0..8)
            .map(|d| {
                let mut teams: Vec<(usize, (u32, u32, u32))> =
                    (0..cf.len()).filter(|&t| cf[t] * 4 + dv[t] % 4 == d || dv[t] == d).map(|t| (t, w.nfl_record(t))).collect();
                teams.dedup_by_key(|x| x.0);
                teams.sort_by(|a, b| (b.1.0 * 2 + b.1.2).cmp(&(a.1.0 * 2 + a.1.2)));
                json!({ "name": names[d], "teams": teams.iter().map(|(t, r)| json!({ "team": team_brief(w, TeamRef::Nfl(*t)), "w": r.0, "l": r.1, "t": r.2 })).collect::<Vec<_>>() })
            })
            .collect();
        return json!({ "groups": groups });
    }
    let Some(conf) = conf else { return json!({ "groups": [] }) };
    let div = match mine(w) {
        Some(TeamRef::College(p)) => Some(w.programs[p].division),
        _ => None,
    };
    let mut teams: Vec<(usize, afm_engine::world::seat::Record)> = w
        .programs
        .iter()
        .filter(|p| p.conference == conf && div.is_none_or(|d| p.division == d))
        .map(|p| (p.id, w.college_record(p.id)))
        .collect();
    teams.sort_by(|a, b| (b.1.conf_wins, b.1.wins, a.1.conf_losses).cmp(&(a.1.conf_wins, a.1.wins, b.1.conf_losses)));
    let mut confs: Vec<String> =
        w.programs.iter().filter(|p| div.is_none_or(|d| p.division == d)).map(|p| p.conference.clone()).collect();
    confs.sort();
    confs.dedup();
    json!({
        "conferences": confs,
        "groups": [{ "name": conf, "teams": teams.iter().map(|(p, r)| json!({
            "team": team_brief(w, TeamRef::College(*p)),
            "cw": r.conf_wins, "cl": r.conf_losses, "w": r.wins, "l": r.losses,
        })).collect::<Vec<_>>() }],
    })
}

fn reputation_words(r: f64) -> &'static str {
    match r {
        x if x >= 16.0 => "One of the best in the game",
        x if x >= 13.0 => "Highly regarded",
        x if x >= 10.0 => "Respected",
        x if x >= 7.0 => "Little known",
        _ => "Unproven",
    }
}

fn coach_row(w: &World, c: usize, role: &str) -> Value {
    let co = &w.coaches[c];
    json!({
        "id": c,
        "name": co.name,
        "role": role,
        "age": co.age,
        "reputation": reputation_words(co.reputation),
        "came_up": co.came_up.coach_title(),
        "record": format!("{}-{}", co.wins, co.losses),
        "since": co.job.map(|j| j.since),
        "ex_pro": co.ex_pro,
        "you": w.career.as_ref().is_some_and(|k| k.coach == c),
    })
}

pub fn staff(w: &World, t: TeamRef) -> Value {
    let mut rows = Vec::new();
    match t {
        TeamRef::College(i) => {
            let p = &w.programs[i];
            for (c, role) in
                [(p.head_coach, "Head coach"), (p.oc, "Offensive coordinator"), (p.dc, "Defensive coordinator")]
            {
                if let Some(c) = c {
                    rows.push(coach_row(w, c, role));
                }
            }
            for g in Group::ALL {
                if let Some(c) = p.position_coaches[g as usize] {
                    rows.push(coach_row(w, c, g.coach_title()));
                }
            }
        }
        TeamRef::Nfl(i) => {
            let tm = &w.nfl[i];
            for (c, role) in
                [(tm.head_coach, "Head coach"), (tm.oc, "Offensive coordinator"), (tm.dc, "Defensive coordinator")]
            {
                if let Some(c) = c {
                    rows.push(coach_row(w, c, role));
                }
            }
        }
        TeamRef::HighSchool(i) => {
            if let Some(c) = w.high_schools[i].coach {
                rows.push(coach_row(w, c, "Head coach"));
            }
        }
    }
    json!({ "team": team_brief(w, t), "rows": rows })
}

pub fn coach(w: &World, c: usize) -> Result<Value, String> {
    let co = w.coaches.get(c).ok_or("No such coach.")?;
    let job = co.job.map(|j| {
        let (team, name) = match j.employer {
            Employer::College(p) => (TeamRef::College(p), w.program_name(p).to_string()),
            Employer::Nfl(t) => (TeamRef::Nfl(t), w.nfl_team_name(t)),
            Employer::HighSchool(s) => (TeamRef::HighSchool(s), w.school_name(s).to_string()),
        };
        json!({ "team": team.json(), "name": name, "role": j.role.name(), "since": j.since })
    });
    let history: Vec<Value> = co
        .history
        .iter()
        .rev()
        .map(|s| {
            let name = match s.employer {
                Employer::College(p) => w.program_name(p).to_string(),
                Employer::Nfl(t) => w.nfl_team_name(t),
                Employer::HighSchool(h) => w.school_name(h).to_string(),
            };
            json!({ "where": name, "role": s.role.name(), "from": s.from, "to": s.to, "record": format!("{}-{}", s.wins, s.losses), "ended": s.ended })
        })
        .collect();
    let mentor = co.mentor.and_then(|m| w.coaches.get(m)).map(|m| json!({ "id": m.id, "name": m.name }));
    Ok(json!({
        "id": c, "name": co.name, "age": co.age, "reputation": reputation_words(co.reputation),
        "came_up": co.came_up.coach_title(), "ex_pro": co.ex_pro, "job": job, "history": history,
        "mentor": mentor, "titles": co.career_titles,
        "you": w.career.as_ref().is_some_and(|k| k.coach == c),
    }))
}

/// Your depth chart: the order the engine will play, position by position,
/// with your own order where you set one.
pub fn depth(w: &World) -> Value {
    let Some(t) = mine(w) else { return json!({ "positions": [] }) };
    let people = roster_people(w, t);
    let reader = reader_key(w);
    let positions: Vec<Value> = Position::ALL
        .iter()
        .map(|&pos| {
            let mut list: Vec<usize> = people.iter().copied().filter(|&k| w.people[k].player.position == pos).collect();
            let eye = eye_for(w, t, pos);
            // The staff's order: their read of each player, best first.
            let read = |k: usize| match reads::overall(&w.people[k].player, true, eye, reader) {
                reads::Read::Range(lo, hi) => lo as f64 + hi as f64,
                reads::Read::NotSeen => 0.0,
            };
            list.sort_by(|&a, &b| read(b).total_cmp(&read(a)));
            let yours = w.seat_depth.iter().find(|(p, _)| *p == pos).map(|(_, ids)| ids.clone());
            if let Some(ids) = &yours {
                let mut ordered: Vec<usize> =
                    ids.iter().filter_map(|id| list.iter().copied().find(|&k| w.people[k].player.id == *id)).collect();
                for k in list {
                    if !ordered.contains(&k) {
                        ordered.push(k);
                    }
                }
                list = ordered;
            }
            json!({
                "pos": pos.name(),
                "starters": afm_engine::game::roster::starters(pos),
                "set_by_you": yours.is_some(),
                "players": list.iter().map(|&k| row(w, k, t, true, reader)).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({ "team": team_brief(w, t), "positions": positions })
}

pub fn set_depth(w: &mut World, args: &Value) -> Result<Value, String> {
    let pos = args.get("pos").and_then(Value::as_str).ok_or("Missing pos.")?;
    let pos = Position::ALL.into_iter().find(|p| p.name() == pos).ok_or("No such position.")?;
    let ids: Vec<u32> = args
        .get("ids")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_u64).map(|v| v as u32).collect())
        .unwrap_or_default();
    w.set_seat_depth(pos, ids);
    Ok(depth(w))
}

pub fn player(w: &World, id: u32) -> Result<Value, String> {
    let k = person_index(w, id).ok_or("That player is no longer in football.")?;
    let p = &w.people[k];
    let team = team_of(w, p);
    let own = team.is_some_and(|t| is_mine(w, t));
    let reader = reader_key(w);
    let pos = p.player.position;
    let eye = team.filter(|_| own).map_or(10.0, |t| eye_for(w, t, pos));
    let attrs = reads::attributes(&p.player, own, eye, reader);
    let groups: Vec<Value> = [
        (Category::Physical, "Physical"),
        (Category::Passing, "Passing"),
        (Category::Running, "Running"),
        (Category::Receiving, "Receiving"),
        (Category::LinePlay, "Line play"),
        (Category::Defense, "Defense"),
        (Category::KickingAndSnapping, "Kicking and snapping"),
        (Category::Mental, "Mental"),
    ]
    .iter()
    .map(|&(cat, name)| {
        json!({
            "name": name,
            "attrs": attrs.iter().filter(|(a, _)| a.category() == cat).map(|(a, r)| json!({ "key": a.key(), "label": a.label(), "read": r.json() })).collect::<Vec<_>>(),
        })
    })
    .collect();
    let level = level_of(w, p);
    let potential = if own { reads::potential_words(&p.player, eye, level, reader, w.year) } else { Value::Null };
    let history: Vec<Value> = p
        .player
        .history
        .iter()
        .rev()
        .map(|h| {
            json!({
                "age": h.age,
                "level": h.level.name(),
                "pos": h.position.name(),
                "role": h.playing_time.name(),
                "missed": h.games_missed,
                "team": h.team.map(|t| w.nfl_team_name(t)),
                "events": h.events.iter().map(|e| e.name()).collect::<Vec<_>>(),
            })
        })
        .collect();
    let notes: Vec<Value> = w
        .notes_on(id)
        .iter()
        .enumerate()
        .rev()
        .map(|(i, n)| json!({ "k": i, "date": date_text(n.date), "text": n.text }))
        .collect();
    let pro = p.pro.as_ref().map(|c| {
        json!({
            "team": c.team.map(|t| w.nfl_team_name(t)),
            "accrued": c.accrued,
            "draft": c.draft.map(|(y, r, pk, t)| format!("{y} round {r}, pick {pk} by {}", w.nfl_team_name(t))),
            "contract": c.contract.as_ref().map(|k| json!({
                "years": k.real_years(), "total": k.total().round(), "apy": k.apy().round(),
                "guaranteed": k.guaranteed().round(), "through": k.last_year(),
                "cap_hit": k.cap_hit(w.year).round(),
            })),
            "earnings": c.earnings.round(),
        })
    });
    let offers: Vec<Value> = p.cal.offers.iter().take(20).map(|&o| json!(w.program_name(o))).collect();
    let tendencies: Vec<Value> = if own {
        afm_engine::tendencies::Tend::ALL
            .iter()
            .map(|&t| {
                let v = p.player.tend.get(t);
                let r = reads::range(v * 99.0 / 20.0, 6.0, reader, id, 200 + t.index());
                json!({ "label": t.label(), "read": r.json() })
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(json!({
        "id": id,
        "name": p.name,
        "pos": pos.name(),
        "positions": p.player.position_history.iter().map(|(a, ps)| json!([a, ps.name()])).collect::<Vec<_>>(),
        "age": p.player.age,
        "class": class_text(w, p),
        "team": team.map(|t| team_brief(w, t)),
        "where": where_now(w, p),
        "mine": own,
        "home": hometown(w, p),
        "high_school": w.high_schools.get(p.hs).map(|s| w.school_name(s.id).to_string()),
        "ht": ft_in(p.player.body.height),
        "wt": p.player.body.weight.round(),
        "arm": p.player.body.arm,
        "hand": p.player.body.hand,
        "stars": p.stars,
        "status": status_text(w, p),
        "mood": mood_words(p),
        "ovr": reads::overall(&p.player, own, eye, reader).json(),
        "potential": potential,
        "groups": groups,
        "tendencies": tendencies,
        "history": history,
        "pro": pro,
        "offers": offers,
        "notes": notes,
        "stories": p.notes.iter().rev().take(20).collect::<Vec<_>>(),
    }))
}

fn team_of(w: &World, p: &Person) -> Option<TeamRef> {
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => Some(TeamRef::HighSchool(p.hs)),
        Stage::College => p.college.map(TeamRef::College),
        Stage::Pro => p.pro.as_ref().and_then(|c| c.team).map(TeamRef::Nfl),
        _ => None,
    }
    .filter(|t| match t {
        TeamRef::HighSchool(i) => *i < w.high_schools.len(),
        _ => true,
    })
}

pub fn my_games(w: &World) -> Value {
    let games: Vec<Value> = w
        .seat_games()
        .iter()
        .enumerate()
        .rev()
        .map(|(k, g)| json!({ "index": k, "date": date_text(g.date), "home": g.names[0], "away": g.names[1], "score": g.stats.score }))
        .collect();
    json!({ "games": games })
}

/// A whole game of yours: the box score, each player's line and the
/// play-by-play with the play callers' reasons.
pub fn game(w: &World, k: usize) -> Result<Value, String> {
    let g = w.seat_games().get(k).ok_or("That game is not kept.")?;
    let name = |id: u32| -> String { person_index(w, id).map_or(format!("#{id}"), |i| w.people[i].name.clone()) };
    let s = &g.stats;
    let top = possession(s);
    let mut sides = Vec::new();
    for side in 0..2 {
        let t = &s.team[side];
        let mut passing = Vec::new();
        let mut rushing = Vec::new();
        let mut receiving = Vec::new();
        let mut defense = Vec::new();
        for (&id, l) in &s.players[side] {
            if l.att > 0 {
                passing.push(json!({ "name": name(id), "id": id, "cmp": l.cmp, "att": l.att, "yds": l.pass_yds.round(), "td": l.pass_td, "int": l.int, "sk": l.sacked }));
            }
            if l.rush > 0 {
                rushing.push(json!({ "name": name(id), "id": id, "car": l.rush, "yds": l.rush_yds.round(), "td": l.rush_td, "long": l.rush_long.round() }));
            }
            if l.targets > 0 {
                receiving.push(json!({ "name": name(id), "id": id, "rec": l.rec, "tgt": l.targets, "yds": l.rec_yds.round(), "td": l.rec_td }));
            }
            if l.tackles + l.assists > 0 || l.sacks > 0.0 || l.ints > 0 {
                defense.push(json!({ "name": name(id), "id": id, "tkl": l.tackles + l.assists, "tfl": l.tfl, "sk": l.sacks, "int": l.ints, "pd": l.pbus }));
            }
        }
        fn by(v: &mut [Value], k: &str) {
            v.sort_by(|a, b| b[k].as_f64().unwrap_or(0.0).total_cmp(&a[k].as_f64().unwrap_or(0.0)));
        }
        by(&mut passing, "att");
        by(&mut rushing, "yds");
        by(&mut receiving, "yds");
        by(&mut defense, "tkl");
        sides.push(json!({
            "name": g.names[side],
            "score": s.score[side],
            "totals": {
                "plays": t.plays, "yards": t.yards.round(), "pass": t.pass_yards.round(), "rush": t.rush_yards.round(),
                "first_downs": t.first_downs, "turnovers": t.turnovers, "penalties": t.penalties, "pen_yds": t.penalty_yds.round(),
                "third": format!("{}/{}", t.third_conv, t.third_att), "top": format!("{}:{:02}", (top[side] / 60.0) as u32, (top[side] % 60.0) as u32),
            },
            "passing": passing, "rushing": rushing, "receiving": receiving, "defense": defense.into_iter().take(14).collect::<Vec<_>>(),
        }));
    }
    let plays: Vec<Value> = s
        .plays
        .iter()
        .map(|r| {
            json!({
                "q": r.quarter,
                "clock": format!("{}:{:02}", (r.clock / 60.0) as u32, (r.clock % 60.0) as u32),
                "off": g.names[r.offense],
                "down": r.down,
                "togo": r.togo.round(),
                "spot": spot_text(r.yardline_100),
                "text": play_text(r, &name),
                "call": r.call,
                "why": r.call_why,
                "score": r.score,
            })
        })
        .collect();
    Ok(
        json!({ "date": date_text(g.date), "quarters": s.quarters, "overtime": s.overtime, "sides": sides, "plays": plays }),
    )
}

fn spot_text(to_goal: f64) -> String {
    let y = to_goal.round() as i32;
    if y == 50 {
        "50".into()
    } else if y > 50 {
        format!("OWN {}", 100 - y)
    } else {
        format!("OPP {y}")
    }
}

/// Time of possession from the game clock: each snap's team holds the ball
/// until the next snap (or the end of the quarter). The engine's own total
/// counts only the seconds the ball is live.
fn possession(s: &afm_engine::game::stats::GameStats) -> [f64; 2] {
    use afm_engine::game::stats::PlayType;
    let mut top = [0.0; 2];
    for (i, r) in s.plays.iter().enumerate() {
        if matches!(r.kind, PlayType::Kickoff) || r.offense > 1 {
            continue;
        }
        let end = match s.plays.get(i + 1) {
            Some(n) if n.quarter == r.quarter => n.clock,
            _ => 0.0,
        };
        top[r.offense] += (r.clock - end).max(0.0);
    }
    top
}

fn play_text(r: &afm_engine::game::stats::PlayRecord, name: &dyn Fn(u32) -> String) -> String {
    use afm_engine::game::stats::PlayType;
    let yds = r.yards.round() as i32;
    let gain = |y: i32| {
        if y == 0 {
            "no gain".to_string()
        } else if y < 0 {
            format!("a loss of {}", -y)
        } else {
            format!("{y} yards")
        }
    };
    let mut t = match r.kind {
        PlayType::Pass => {
            let qb = r.passer.map(name).unwrap_or_default();
            let to = r.target.map(name).unwrap_or_default();
            if r.interception {
                format!("{qb} pass intended for {to} is INTERCEPTED")
            } else if r.complete {
                format!("{qb} pass to {to} ({}) for {}", r.route, gain(yds))
            } else {
                format!("{qb} pass incomplete, intended for {to}")
            }
        }
        PlayType::Run => format!("{} runs for {}", r.carrier.map(name).unwrap_or_default(), gain(yds)),
        PlayType::Scramble => {
            format!("{} scrambles for {}", r.carrier.or(r.passer).map(name).unwrap_or_default(), gain(yds))
        }
        PlayType::Sack => {
            let by: Vec<String> = if r.sackers.is_empty() { &r.tacklers } else { &r.sackers }.iter().map(|&i| name(i)).collect();
            let by = if by.is_empty() { String::new() } else { format!(" by {}", by.join(" and ")) };
            format!("{} sacked{by} for {}", r.passer.map(name).unwrap_or_default(), gain(yds))
        }
        PlayType::Kickoff | PlayType::Punt => {
            let what = if r.kind == PlayType::Punt { "punts" } else { "kicks off" };
            let k = r.kicker.map(name).unwrap_or_default();
            let dist = r.kick_distance.round() as i32;
            if r.blocked {
                format!("{k} {what}: BLOCKED")
            } else if r.touchback {
                format!("{k} {what} {dist} yards, touchback")
            } else if r.fair_catch {
                format!("{k} {what} {dist} yards, fair catch by {}", r.returner.map(name).unwrap_or_default())
            } else if let Some(ret) = r.returner {
                format!("{k} {what} {dist} yards, {} returns it {}", name(ret), gain(r.return_yards.round() as i32))
            } else {
                format!("{k} {what} {dist} yards")
            }
        }
        PlayType::FieldGoal => {
            let k = r.kicker.map(name).unwrap_or_default();
            let dist = r.kick_distance.round() as i32;
            let res = if r.blocked { "BLOCKED" } else if r.kick_good { "good" } else { "no good" };
            format!("{k} {dist}-yard field goal is {res}")
        }
        PlayType::ExtraPoint => {
            let k = r.kicker.map(name).unwrap_or_default();
            let res = if r.blocked { "BLOCKED" } else if r.kick_good { "good" } else { "no good" };
            format!("{k} extra point is {res}")
        }
        PlayType::NoPlay => {
            let fouls: Vec<String> = r.penalties.iter().map(|(f, _, id, y, _)| format!("{} on {}, {} yards", f.name(), name(*id), y.round())).collect();
            format!("Penalty before the snap: {}", fouls.join("; "))
        }
        other => format!("{other:?}: {}", gain(yds)),
    };
    if r.nullified {
        t = format!("No play ({t})");
    }
    if r.touchdown {
        t.push_str(", TOUCHDOWN");
    }
    if r.kind != PlayType::NoPlay {
        for (f, _, id, y, accepted) in &r.penalties {
            t.push_str(&format!(". Flag: {} on {}, {} yards{}", f.name(), name(*id), y.round(), if *accepted { "" } else { ", declined" }));
        }
    }
    if r.turnover && !r.interception {
        t.push_str(", turnover");
    }
    if r.first_down && !r.touchdown {
        t.push_str(", first down");
    }
    t
}

pub fn search(w: &World, q: &str) -> Value {
    let q = q.trim().to_lowercase();
    if q.len() < 2 {
        return json!({ "players": [], "teams": [], "coaches": [] });
    }
    let players: Vec<Value> = w
        .people
        .iter()
        .filter(|p| p.name.to_lowercase().contains(&q))
        .take(40)
        .map(|p| json!({ "id": p.player.id, "name": p.name, "pos": p.player.position.name(), "age": p.player.age, "where": where_now(w, p), "level": level_of(w, p).name() }))
        .collect();
    let mut teams: Vec<Value> = (0..w.nfl.len())
        .filter(|&t| w.nfl_team_name(t).to_lowercase().contains(&q))
        .map(|t| team_brief(w, TeamRef::Nfl(t)))
        .collect();
    teams.extend(
        w.programs
            .iter()
            .filter(|p| w.program_name(p.id).to_lowercase().contains(&q))
            .take(20)
            .map(|p| team_brief(w, TeamRef::College(p.id))),
    );
    teams.extend(
        w.high_schools
            .iter()
            .filter(|s| w.school_name(s.id).to_lowercase().contains(&q))
            .take(20)
            .map(|s| team_brief(w, TeamRef::HighSchool(s.id))),
    );
    let coaches: Vec<Value> = w
        .coaches
        .iter()
        .filter(|c| !c.retired && c.name.to_lowercase().contains(&q))
        .take(20)
        .map(|c| json!({ "id": c.id, "name": c.name, "role": c.job.map(|j| j.role.name()) }))
        .collect();
    json!({ "players": players, "teams": teams, "coaches": coaches })
}

/// Your own profile: you see your own ratings (people topic 15).
pub fn you(w: &World) -> Value {
    let Some(c) = &w.career else { return Value::Null };
    let co = &w.coaches[c.coach];
    let Some(ab) = w.your_ability() else { return Value::Null };
    let y = &c.you;
    json!({
        "name": y.name, "age": co.age, "reputation": reputation_words(co.reputation),
        "record": format!("{}-{}", co.wins, co.losses), "titles": co.career_titles,
        "started": date_text(c.started),
        "teaching": Group::ALL.iter().map(|g| json!({ "group": g.coach_title(), "value": (ab.teaching[*g as usize] * 10.0).round() / 10.0 })).collect::<Vec<_>>(),
        "scheme": ab.scheme, "game_management": ab.game_management, "motivation": ab.motivation,
        "discipline": ab.discipline, "recruiting": ab.recruiting, "network": ab.network,
        "relationships": ab.relationships,
        "style": { "risk": y.style.risk, "freedom": y.style.freedom, "physical": y.style.physical, "ball_hunting": y.style.ball_hunting, "mould": y.style.mould },
        "history": coach(w, c.coach).map(|v| v["history"].clone()).unwrap_or(Value::Null),
    })
}

#[allow(dead_code)]
fn role_name(r: Role) -> &'static str {
    r.name()
}
