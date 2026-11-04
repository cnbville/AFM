//! Lane NB8 (Data overhaul items 25-27; boards 47 and 49): situations,
//! decisions and concepts against coverages, for your team, any team and
//! the league, as deep as your seat's charting goes.
//!
//! - `situations` (`level`, `division`, `year`, `team`, `side`): down and
//!   distance, field zones, the red zone, the two-minute drill, the score
//!   and garbage time, each with the level's numbers beside (item 25);
//! - `decisions` (`level`, `division`, `year`, `team`): every 4th down and
//!   try against the analysts' model (`tally::decide`), the model's own
//!   call by down and spot, each head coach's record, timeouts and
//!   challenges (item 26);
//! - `concepts` (`level`, `division`, `year`, `team`, `view`, `measure`):
//!   each concept against each coverage, what your next opponent plays and
//!   your calls against what you saw (item 27).
//!
//! College and NFL numbers come from the season's tallies, less your
//! latest game while your staff is still charting it (item 2). High school
//! is box scores for every seat: your own film (your kept games) is the
//! only charting there. Concepts against coverages need film charting
//! (depth 2) or deeper.

use std::collections::{BTreeMap, HashMap};

use afm_engine::calendar::Level as CalLevel;
use afm_engine::game::win_chance::State;
use afm_engine::player::Level as PLevel;
use afm_engine::world::World;
use afm_engine::world::coach::Role;
use afm_engine::world::data::Division;
use afm_engine::world::folk::Team;
use afm_engine::world::marks::Tier;
use afm_engine::world::tally::{self, calls, decide, games, teams};
use afm_engine::world::units::Charge;
use serde_json::{Value, json};

use super::Charting;
use crate::views::{self, TeamRef};

/// What a page reads: the level, the season, the team and your charting.
pub(crate) struct Scope {
    pub tier: Tier,
    pub level: tally::Level,
    pub division: Option<Division>,
    pub year: i32,
    pub years: Vec<i32>,
    /// The team the page is about (else your team at this level).
    pub team: Option<usize>,
    /// Your team at this level.
    pub mine: Option<usize>,
    pub ch: Option<Charting>,
    /// Games on or after this day are not charted yet (the open season).
    pub since: Option<i32>,
    pub model: PLevel,
}

fn tally_level(t: Tier) -> tally::Level {
    match t {
        Tier::College => tally::Level::College,
        Tier::Nfl => tally::Level::Nfl,
        Tier::Hs => tally::Level::Hs,
    }
}

pub(crate) fn mine_at(w: &World, tier: Tier) -> Option<usize> {
    match (views::mine(w), tier) {
        (Some(TeamRef::College(p)), Tier::College) => Some(p),
        (Some(TeamRef::Nfl(t)), Tier::Nfl) => Some(t),
        (Some(TeamRef::HighSchool(h)), Tier::Hs) => Some(h),
        _ => None,
    }
}

/// Your kept high school games of a season (your own film).
fn hs_games(w: &World, year: i32, team: usize) -> Vec<(usize, &afm_engine::world::seat::SeatGame)> {
    w.seat_games()
        .iter()
        .enumerate()
        .filter(|(_, g)| g.level == CalLevel::HighSchool && g.date.year() == year && g.teams.contains(&team))
        .collect()
}

pub(crate) fn scope(w: &World, args: &Value) -> Scope {
    let tier = super::tier_arg(w, args);
    let level = tally_level(tier);
    let division = (tier == Tier::College).then(|| super::division_arg(w, args));
    let mine = mine_at(w, tier);
    let mut years: Vec<i32> = if tier == Tier::Hs {
        let mut y: Vec<i32> = mine
            .map(|h| {
                w.seat_games()
                    .iter()
                    .filter(|g| g.level == CalLevel::HighSchool && g.teams.contains(&h))
                    .map(|g| g.date.year())
                    .collect()
            })
            .unwrap_or_default();
        y.sort_unstable();
        y.dedup();
        y
    } else {
        w.tally_seasons()
    };
    years.reverse();
    let year = args
        .get("year")
        .and_then(Value::as_i64)
        .map(|y| y as i32)
        .filter(|y| years.contains(y))
        .or(years.first().copied())
        .unwrap_or(w.year);
    let valid = |t: usize| match tier {
        Tier::College => t < w.programs.len() && Some(super::season_place(w, t, year).0) == division,
        Tier::Nfl => t < w.nfl.len(),
        Tier::Hs => Some(t) == mine,
    };
    // "none": the league alone.
    let none = args.get("team").and_then(Value::as_str) == Some("none");
    let team = args
        .get("team")
        .and_then(Value::as_u64)
        .map(|t| t as usize)
        .filter(|&t| valid(t))
        .or(mine.filter(|&t| valid(t) && !none));
    let ch = if tier == Tier::Hs {
        // Your own film: everything, the night of the game.
        mine.map(|_| Charting { depth: 3, days: 1, head: None })
    } else {
        super::charting(w, tier, division)
    };
    let open = tier != Tier::Hs && year == w.tally_open_season();
    let since = ch.as_ref().filter(|_| open).map(|c| w.today.0 - (c.days as i32 - 1));
    let model = match tier {
        Tier::Nfl => PLevel::NFL,
        Tier::Hs => PLevel::HS,
        Tier::College => division.map_or(PLevel::FBS, afm_engine::world::division_level),
    };
    Scope { tier, level, division, year, years, team, mine, ch, since, model }
}

/// The page's common head: the level, the season, the team, your charting.
fn head(w: &World, sc: &Scope) -> serde_json::Map<String, Value> {
    let mut m = serde_json::Map::new();
    m.insert("level".into(), json!(super::tier_key(sc.tier)));
    m.insert("division".into(), json!(sc.division.map(|d| d.name())));
    m.insert("divisions".into(), json!(Division::ALL.iter().map(|d| d.name()).collect::<Vec<_>>()));
    m.insert("year".into(), json!(sc.year));
    m.insert("years".into(), json!(sc.years));
    m.insert("team".into(), sc.team.map_or(Value::Null, |t| super::team_json(w, sc.tier, t)));
    m.insert("team_id".into(), json!(sc.team));
    m.insert("mine".into(), json!(sc.team.is_some() && sc.team == sc.mine));
    m.insert("charting".into(), sc.ch.as_ref().map_or(Value::Null, super::charting_json));
    if let Some(d) = sc.since {
        m.insert("charted_to".into(), json!(afm_engine::calendar::Date(d).to_string()));
    }
    m.insert("source".into(), json!(source_line(sc)));
    m
}

fn source_line(sc: &Scope) -> String {
    match (sc.tier, &sc.ch) {
        (Tier::Hs, Some(_)) => {
            "High school is box scores: these numbers are your own film of your games, and no one else's.".into()
        }
        (Tier::Hs, None) => "High school is box scores for your seat: no one charts its snaps.".into(),
        (_, None) => format!(
            "Your staff doesn't chart {}: box scores only, so its situations and decisions don't show.",
            sc.division.map_or("this level", |d| d.name())
        ),
        (Tier::Nfl, Some(c)) => {
            format!("The NFL's league-wide tracking, {} the morning after each game.", tally_kind(c))
        }
        (Tier::College, Some(c)) => {
            let who =
                c.head.as_ref().map_or("your analytics staff".to_string(), |n| format!("your analytics head, {n}"));
            let when = if c.days == 1 { "the morning after".to_string() } else { format!("{} days after", c.days) };
            format!(
                "Charted by {who}: {} of every {} game, {when}.",
                tally_kind(c),
                sc.division.map_or("college", |d| d.name())
            )
        }
    }
}

fn tally_kind(c: &Charting) -> &'static str {
    afm_engine::world::analytics::depth_name(c.depth)
}

/// Every team's season at the level (the division in college), as far as
/// your staff has charted it.
fn pool(w: &World, sc: &Scope) -> Vec<teams::Season> {
    let mut out = Vec::new();
    if sc.tier == Tier::Hs {
        if let Some(s) = sc.team.and_then(|t| hs_season(w, sc, t)) {
            out.push(s.0);
        }
        return out;
    }
    w.tally_teams(sc.year, |s| {
        if s.level != sc.level as u8 {
            return;
        }
        if sc.tier == Tier::College && Some(super::season_place(w, s.team as usize, sc.year).0) != sc.division {
            return;
        }
        out.push(match sc.since {
            Some(d) => s.charted_by(d),
            None => {
                let mut s = s.clone();
                s.last = None;
                s
            }
        });
    });
    out
}

/// Your high school team's season from your film: its situations and its
/// calls (each kept game weighed as the tallies weigh a college game).
fn hs_season(w: &World, sc: &Scope, team: usize) -> Option<(teams::Season, calls::Season)> {
    let list = hs_games(w, sc.year, team);
    if list.is_empty() {
        return None;
    }
    let mut t = teams::Open::default();
    let mut c = calls::Open::default();
    for (k, g) in list {
        let side = g.teams.iter().position(|&x| x == team)?;
        let adv = games::adv(&g.stats);
        let score = [g.stats.score[side], g.stats.score[1 - side]];
        let flow = crate::lane_boards::flow::Wp::of_game(w, g);
        let model = decide::Model { p: flow.p, level: PLevel::HS, spread: flow.home_spread };
        let about = calls::About {
            date: g.date.0,
            teams: [g.teams[0] as u32, g.teams[1] as u32],
            neutral: flow.home_spread == 0.0,
            season: sc.year,
        };
        let [a, b] = calls::game(&g.stats, &model, about, true);
        let mine = if side == 0 { a } else { b };
        let tg = teams::game(&g.stats, side, PLevel::HS);
        t.add(tally::Level::Hs, team as u32, g.date.0, tg, [&adv[side], &adv[1 - side]], score);
        c.add(tally::Level::Hs, team as u32, k as u64, mine);
    }
    let key = teams::key(tally::Level::Hs, team as u32);
    let mut ts = t.seasons.remove(&key)?;
    ts.last = None;
    Some((ts, c.seasons.remove(&key)?))
}

fn sum_seasons(list: &[teams::Season]) -> teams::Season {
    let mut out = teams::Season::default();
    for s in list {
        out.games += s.games;
        for (k, v) in &s.cells {
            let e = out.cells.entry(k.clone()).or_insert([0.0; 8]);
            for i in 0..8 {
                e[i] += v[i];
            }
        }
        teams::add_adv(&mut out.adv, &s.adv);
        teams::add_adv(&mut out.adv_against, &s.adv_against);
    }
    out
}

// ---------- Situations (item 25) ----------

/// A situation's numbers from its runs and dropbacks: plays, pass rate,
/// yards a play, EPA a play, success rate and the rate of first downs or
/// touchdowns.
fn measures(run: teams::Situ, pass: teams::Situ) -> Value {
    let n = (run[0] + pass[0]) as f64;
    if n <= 0.0 {
        return Value::Null;
    }
    let s = |i: usize| (run[i] + pass[i]) as f64;
    json!({
        "n": n,
        "pass": 100.0 * pass[0] as f64 / n,
        "ypp": s(1) / n,
        "epa": s(2) / n,
        "succ": 100.0 * s(3) / n,
        "conv": 100.0 * (s(4) + s(5)) / n,
        "to": s(6),
    })
}

fn pair(s: &teams::Season, side: &str, base: &str) -> (teams::Situ, teams::Situ) {
    (s.get(&format!("{side}|{base}|run")), s.get(&format!("{side}|{base}|pass")))
}

fn minus(a: teams::Situ, b: teams::Situ) -> teams::Situ {
    let mut o = a;
    for i in 0..8 {
        o[i] -= b[i];
    }
    o
}

/// One table row: the team's numbers and the level's (a team's average:
/// the level's sum is every team's).
fn row(label: &str, team: Option<&teams::Season>, level: &teams::Season, side: &str, base: &str) -> Value {
    let t = team.map(|s| pair(s, side, base));
    let l = pair(level, side, base);
    json!({
        "label": label,
        "team": t.map_or(Value::Null, |(r, p)| measures(r, p)),
        "level": measures(l.0, l.1),
    })
}

const BANDS: [&str; 5] = ["1", "2-3", "4-6", "7-10", "11+"];
const ZONES: [(&str, &str); 7] = [
    ("backed up (own 1-5)", "Backed up: own 1-5"),
    ("own 6-20", "Own 6-20"),
    ("own 21-40", "Own 21-40"),
    ("midfield", "Midfield: own 41 to their 40"),
    ("opp 21-40", "Their 21-40"),
    ("red zone (6-20)", "Red zone: their 6-20"),
    ("goal (1-5)", "Goal line: their 1-5"),
];

fn red_zone(a: &games::Adv) -> Value {
    let pts = |td: u32, fg: u32, n: u32| if n > 0 { (7.0 * td as f64 + 3.0 * fg as f64) / n as f64 } else { 0.0 };
    json!({
        "trips": a.rz_trips,
        "td": if a.rz_trips > 0 { 100.0 * a.rz_td as f64 / a.rz_trips as f64 } else { 0.0 },
        "fg": if a.rz_trips > 0 { 100.0 * a.rz_fg as f64 / a.rz_trips as f64 } else { 0.0 },
        "pts": pts(a.rz_td, a.rz_fg, a.rz_trips),
        "trips40": a.trips40,
        "pts40": pts(a.trip_td, a.trip_fg, a.trips40),
        "drives": a.drives,
        "three_outs": if a.drives > 0 { 100.0 * a.three_outs as f64 / a.drives as f64 } else { 0.0 },
    })
}

/// Item 25: the situations pages.
pub fn situations(w: &World, args: &Value) -> Value {
    let sc = scope(w, args);
    let mut m = head(w, &sc);
    let side = if args.get("side").and_then(Value::as_str) == Some("d") { "d" } else { "o" };
    m.insert("side".into(), json!(side));
    if sc.ch.is_none() {
        m.insert("blocked".into(), json!(source_line(&sc)));
        return Value::Object(m);
    }
    let all = pool(w, &sc);
    if all.is_empty() {
        m.insert("blocked".into(), json!("No charted games in this season yet."));
        return Value::Object(m);
    }
    let level = sum_seasons(&all);
    let team_s = sc.team.and_then(|t| all.iter().find(|s| s.team as usize == t));
    let t = team_s;
    let mut downs = Vec::new();
    for d in 1..=4 {
        for b in BANDS {
            let base = format!("dd|{d}|{b}");
            let (r, p) = pair(&level, side, &base);
            if r[0] + p[0] < 1.0 {
                continue;
            }
            let label = format!("{} & {b}", ["1st", "2nd", "3rd", "4th"][d - 1]);
            downs.push(row(&label, t, &level, side, &base));
        }
    }
    let zones: Vec<Value> = ZONES.iter().map(|(k, l)| row(l, t, &level, side, &format!("field|{k}"))).collect();
    let rz = vec![
        row("Red zone: inside their 20", t, &level, side, "red zone"),
        row("Goal to go", t, &level, side, "goal to go"),
    ];
    let pick = |s: &teams::Season| if side == "o" { s.adv } else { s.adv_against };
    let n = all.len().max(1) as f64;
    let lv_adv = pick(&level);
    let two_rest = |s: &teams::Season| {
        let (r, p) = pair(s, side, "kind");
        let (tr, tp) = pair(s, side, "two minute");
        (minus(r, tr), minus(p, tp))
    };
    let mut two = vec![row("Last two minutes of a half", t, &level, side, "two minute")];
    {
        let (lr, lp) = two_rest(&level);
        let tm = t.map(|s| {
            let (r, p) = two_rest(s);
            measures(r, p)
        });
        two.push(
            json!({ "label": "The rest of the game", "team": tm.unwrap_or(Value::Null), "level": measures(lr, lp) }),
        );
    }
    for q in ["1", "2", "3", "4", "OT"] {
        let base = format!("quarter|{q}");
        let (r, p) = pair(&level, side, &base);
        if r[0] + p[0] >= 1.0 {
            let label = if q == "OT" { "Overtime".to_string() } else { format!("Quarter {q}") };
            two.push(row(&label, t, &level, side, &base));
        }
    }
    let mut score = vec![
        row("Ahead by 9 or more", t, &level, side, "score|ahead"),
        row("Within 8 either way", t, &level, side, "score|close"),
        row("Behind by 9 or more", t, &level, side, "score|behind"),
        row("Garbage time", t, &level, side, "garbage"),
    ];
    let clean = |s: &teams::Season| {
        let (r, p) = pair(s, side, "kind");
        let (gr, gp) = pair(s, side, "garbage");
        measures(minus(r, gr), minus(p, gp))
    };
    score
        .push(json!({ "label": "Without garbage time", "team": t.map_or(Value::Null, clean), "level": clean(&level) }));
    let all_row = row("Every snap", t, &level, side, "kind");
    m.insert("all".into(), all_row);
    m.insert("downs".into(), json!(downs));
    m.insert("zones".into(), json!(zones));
    m.insert("red_zone".into(), json!(rz));
    m.insert(
        "red_trips".into(),
        json!({
            "team": t.map_or(Value::Null, |s| red_zone(&pick(s))),
            "level": red_zone(&lv_adv),
        }),
    );
    m.insert("two_minute".into(), json!(two));
    m.insert("score".into(), json!(score));
    m.insert("teams".into(), json!(all.len()));
    m.insert("per_team".into(), json!(level.games as f64 / n));
    m.insert(
        "garbage_note".into(),
        json!(if sc.model == PLevel::NFL {
            "Garbage time in the NFL: the offense's win chance under 10% or over 90% (rbsdm.com's line)."
        } else {
            "Garbage time in college and high school: a lead of more than 38 in the 2nd quarter, 28 in the 3rd or 22 in the 4th (Bill Connelly's margins)."
        }),
    );
    let pick_list = pick_teams(w, &sc, &all);
    m.insert("pick".into(), json!(pick_list));
    Value::Object(m)
}

/// The teams a page can switch to, by name.
fn pick_teams(w: &World, sc: &Scope, all: &[teams::Season]) -> Vec<Value> {
    let mut v: Vec<(String, usize)> = all
        .iter()
        .map(|s| {
            let t = s.team as usize;
            let name = super::team_json(w, sc.tier, t).get("name").and_then(Value::as_str).unwrap_or("").to_string();
            (name, t)
        })
        .collect();
    v.sort();
    v.into_iter().map(|(n, t)| json!([t, n])).collect()
}

// ---------- Decisions (item 26) ----------

fn did_word(d: u8) -> &'static str {
    ["Went", "Punted", "Kicked"][d.min(2) as usize]
}

fn best_word(b: u8) -> &'static str {
    ["go", "punt", "kick"][b.min(2) as usize]
}

fn res_word(f: &calls::Fourth) -> String {
    match f.did {
        0 => match f.res {
            2 => "Touchdown".into(),
            1 => format!("Converted, {} yds", f.yds.round()),
            3 => "Turned it over".into(),
            _ => format!("Stopped, {} yds", f.yds.round()),
        },
        1 => {
            let y = f.yds.round();
            if f.res == 4 {
                "Punt blocked".into()
            } else if y < 0.0 {
                "Punted".into()
            } else if y < 50.0 {
                format!("Their ball at their {y}")
            } else if y > 50.0 {
                format!("Their ball at your {}", 100.0 - y)
            } else {
                "Their ball at the 50".into()
            }
        }
        _ => match f.res {
            1 => format!("Good from {}", f.yds.round()),
            4 => "Blocked".into(),
            _ => format!("No good from {}", f.yds.round()),
        },
    }
}

/// A spot from yards to a team's own goal ("own 25", "50", "opp 40").
fn spot_from_own(y: f64) -> String {
    let y = y.round();
    if y < 50.0 {
        format!("own {y}")
    } else if y > 50.0 {
        format!("opp {}", 100.0 - y)
    } else {
        "50".into()
    }
}

pub(crate) fn clock_text(q: u8, secs: u16) -> String {
    // College and high school overtime has no clock (its rows keep 65000).
    if q > 4 && secs >= 3600 {
        return "OT".into();
    }
    let qt = if q > 4 { "OT".to_string() } else { format!("Q{q}") };
    format!("{qt} {}:{:02}", secs / 60, secs % 60)
}

fn player_ref(w: &World, id: u32) -> Value {
    if id == 0 {
        return Value::Null;
    }
    match views::person_index(w, id) {
        Some(k) => json!({ "id": id, "name": w.people[k].name }),
        None => Value::Null,
    }
}

/// Your kept games by (date, opponent), for links to their full pages.
fn seat_links(w: &World, team: usize, tier: Tier) -> HashMap<(i32, u32), usize> {
    let level = match tier {
        Tier::Nfl => CalLevel::Nfl,
        Tier::Hs => CalLevel::HighSchool,
        Tier::College => CalLevel::College,
    };
    let mut m = HashMap::new();
    for (k, g) in w.seat_games().iter().enumerate() {
        if g.level == level && g.teams.contains(&team) {
            let opp = if g.teams[0] == team { g.teams[1] } else { g.teams[0] };
            m.insert((g.date.0, opp as u32), k);
        }
    }
    m
}

fn game_link(sc: &Scope, seat: &HashMap<(i32, u32), usize>, date: i32, opp: u32, key: u64) -> Value {
    if let Some(k) = seat.get(&(date, opp)) {
        return json!({ "seat": k });
    }
    if sc.tier == Tier::Hs {
        return json!({ "seat": key });
    }
    json!({ "year": sc.year, "key": key })
}

fn fourth_json(w: &World, sc: &Scope, seat: &HashMap<(i32, u32), usize>, f: &calls::Fourth) -> Value {
    let j = f.weigh().map(|x| calls::judge(&x, f.did));
    let at = ["vs", "at", "vs"][f.at.min(2) as usize];
    json!({
        "date": afm_engine::calendar::Date(f.date).to_string(),
        "opp": super::team_json(w, sc.tier, f.opp as usize),
        "at": at,
        "link": game_link(sc, seat, f.date, f.opp, f.game),
        "clock": clock_text(f.q, f.clock),
        "lead": f.lead,
        "dd": format!("4th & {}", if f.togo >= f.yl as f32 - 0.01 && f.yl <= 10 { "goal".into() } else { format!("{}", f.togo.ceil().max(1.0)) }),
        "spot": spot_from_own(100.0 - f.yl as f64),
        "did": did_word(f.did),
        "wp": f.wp,
        "conv": f.conv,
        "best": j.map(|j| best_word(j.best)),
        "agreed": j.map(|j| j.agreed),
        "lost": j.map(|j| 100.0 * j.lost),
        "result": res_word(f),
        "who": player_ref(w, f.who),
        "fin": f.fin,
    })
}

fn try_json(w: &World, sc: &Scope, seat: &HashMap<(i32, u32), usize>, t: &calls::Try) -> Value {
    let j = t.judge();
    json!({
        "date": afm_engine::calendar::Date(t.date).to_string(),
        "opp": super::team_json(w, sc.tier, t.opp as usize),
        "at": (["vs", "at", "vs"][t.at.min(2) as usize]),
        "link": game_link(sc, seat, t.date, t.opp, t.game),
        "clock": clock_text(t.q, t.clock),
        "lead": t.lead,
        "did": if t.two { "Went for two" } else { "Kicked" },
        "good": t.good,
        "wp": t.wp,
        "best": j.map(|(two, _, _)| if two { "two" } else { "kick" }),
        "agreed": j.map(|x| x.1),
        "lost": j.map(|x| 100.0 * x.2),
        "fin": t.fin,
    })
}

/// A team's season of calls as your staff has charted it: the rows before
/// `since`, their sums and grid, and the season's timeouts and challenges.
struct Calls {
    team: u32,
    sum: calls::Sums,
    grid: BTreeMap<u16, [u32; 3]>,
    fourths: Vec<calls::Fourth>,
    tries: Vec<calls::Try>,
    chals: Vec<calls::Chal>,
}

fn charted_calls(s: calls::Season, since: Option<i32>) -> Calls {
    let keep = |d: i32| since.is_none_or(|x| d < x);
    let fourths: Vec<calls::Fourth> = s.fourths.into_iter().filter(|f| keep(f.date)).collect();
    let tries: Vec<calls::Try> = s.tries.into_iter().filter(|t| keep(t.date)).collect();
    let chals: Vec<calls::Chal> = s.challenges.into_iter().filter(|c| keep(c.date)).collect();
    // High school's other teams keep sums only; your film keeps rows.
    let (mut sum, grid) = if s.level == tally::Level::Hs as u8 && fourths.is_empty() && s.sum.fourths > 0 {
        (s.sum, s.grid)
    } else {
        let f: Vec<&calls::Fourth> = fourths.iter().collect();
        let t: Vec<&calls::Try> = tries.iter().collect();
        calls::sums_of(&f, &t)
    };
    sum.games = s.sum.games;
    sum.timeouts = s.sum.timeouts;
    sum.to_late = s.sum.to_late;
    sum.to_left = s.sum.to_left;
    sum.close_losses = s.sum.close_losses;
    sum.challenges = chals.len() as u32;
    sum.won = chals.iter().filter(|c| c.won).count() as u32;
    sum.missed = s.sum.missed;
    Calls { team: s.team, sum, grid, fourths, tries, chals }
}

fn calls_pool(w: &World, sc: &Scope) -> Vec<Calls> {
    let mut out = Vec::new();
    if sc.tier == Tier::Hs {
        if let Some((_, c)) = sc.team.and_then(|t| hs_season(w, sc, t)) {
            out.push(charted_calls(c, None));
        }
        return out;
    }
    w.tally_calls(sc.year, |s| {
        if s.level != sc.level as u8 {
            return;
        }
        if sc.tier == Tier::College && Some(super::season_place(w, s.team as usize, sc.year).0) != sc.division {
            return;
        }
        out.push(charted_calls(s.clone(), sc.since));
    });
    out
}

fn rate(a: u32, b: u32) -> Value {
    if b == 0 { Value::Null } else { json!(100.0 * a as f64 / b as f64) }
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    Some(v[v.len() / 2])
}

fn sums_json(s: &calls::Sums) -> Value {
    json!({
        "games": s.games,
        "fourths": s.fourths,
        "weighed": s.weighed,
        "went": s.went,
        "converted": s.converted,
        "conv_rate": rate(s.converted, s.went),
        "said_go": s.said_go,
        "went_said_go": s.went_said_go,
        "go_rate": rate(s.went_said_go, s.said_go),
        "agreed": rate(s.agreed, s.weighed),
        "lost": s.lost,
        "lost_game": if s.games > 0 { 100.0 * s.lost as f64 / s.games as f64 } else { 0.0 },
        "tries": s.tries,
        "twos": s.twos,
        "twos_made": s.twos_made,
        "said_two": s.said_two,
        "two_said_two": s.two_said_two,
        "try_agreed": rate(s.try_agreed, s.tries_weighed),
        "try_lost": s.try_lost,
        "timeouts": s.timeouts,
        "to_late": s.to_late,
        "to_left": s.to_left,
        "close_losses": s.close_losses,
        "challenges": s.challenges,
        "won": s.won,
        "missed": s.missed,
    })
}

/// The model's own call in each cell of the grid: a tied game, 10:00 left
/// in the 2nd quarter, three timeouts each, no line.
fn model_grid(sc: &Scope) -> Vec<Vec<Value>> {
    let m = decide::Model::new(sc.model, sc.year, None, true);
    (0..6)
        .map(|r| {
            (0..6)
                .map(|z| {
                    let s = State {
                        quarter: 2,
                        clock: 600.0,
                        lead: 0.0,
                        down: 4,
                        togo: calls::GRID_TOGO[r].min(calls::GRID_YL[z]),
                        yl: calls::GRID_YL[z],
                        timeouts: [3, 3],
                        spread: 0.0,
                        answering: false,
                    };
                    let w = m.fourth_at(&s);
                    let (b, best) = w.best();
                    let next = [Some(w.go), w.punt, w.kick]
                        .iter()
                        .enumerate()
                        .filter(|(k, _)| *k as u8 != b)
                        .filter_map(|(_, v)| *v)
                        .fold(f64::MIN, f64::max);
                    json!({
                        "call": best_word(b),
                        "by": if next > f64::MIN { 100.0 * (best - next) } else { 100.0 },
                        "go": 100.0 * w.go,
                        "punt": w.punt.map(|x| 100.0 * x),
                        "kick": w.kick.map(|x| 100.0 * x),
                        "conv": 100.0 * w.conv,
                    })
                })
                .collect()
        })
        .collect()
}

fn coach_of(w: &World, sc: &Scope, ch: &Charge, team: usize) -> Value {
    let t = match sc.tier {
        Tier::College => Team::College(team),
        Tier::Nfl => Team::Nfl(team),
        Tier::Hs => Team::High(team),
    };
    match ch.get(t, Role::HeadCoach).and_then(|i| w.coaches.get(i).map(|c| (i, c))) {
        Some((i, c)) => json!({ "id": i, "name": c.name }),
        None => Value::Null,
    }
}

/// Item 26: the decisions page.
pub fn decisions(w: &World, args: &Value) -> Value {
    let sc = scope(w, args);
    let mut m = head(w, &sc);
    if sc.ch.is_none() {
        m.insert("blocked".into(), json!(source_line(&sc)));
        return Value::Object(m);
    }
    let all = calls_pool(w, &sc);
    m.insert("rows".into(), json!(calls::GRID_ROWS));
    m.insert("zones".into(), json!(calls::GRID_ZONES));
    m.insert("model".into(), json!(model_grid(&sc)));
    // The level's go rate in each cell, and the team's.
    let mut lv: BTreeMap<u16, [u32; 3]> = BTreeMap::new();
    let mut lsum = calls::Sums::default();
    for c in &all {
        lsum.add(&c.sum);
        for (k, v) in &c.grid {
            let e = lv.entry(*k).or_default();
            for i in 0..3 {
                e[i] += v[i];
            }
        }
    }
    let cells = |g: &BTreeMap<u16, [u32; 3]>| -> Vec<Vec<Value>> {
        (0..6)
            .map(|r| {
                (0..6)
                    .map(|z| {
                        let c = g.get(&((r * 8 + z) as u16)).copied().unwrap_or_default();
                        json!({ "n": c[0], "went": c[1], "said_go": c[2] })
                    })
                    .collect()
            })
            .collect()
    };
    m.insert("level_grid".into(), json!(cells(&lv)));
    m.insert("level_sums".into(), sums_json(&lsum));
    let team = sc.team.and_then(|t| all.iter().find(|c| c.team as usize == t));
    if let Some(c) = team {
        let seat = seat_links(w, c.team as usize, sc.tier);
        m.insert("team_grid".into(), json!(cells(&c.grid)));
        m.insert("sums".into(), sums_json(&c.sum));
        let mut fourths: Vec<&calls::Fourth> = c.fourths.iter().collect();
        fourths.sort_by_key(|f| (f.date, std::cmp::Reverse(f.q), std::cmp::Reverse(f.clock)));
        m.insert("fourths".into(), json!(fourths.iter().map(|f| fourth_json(w, &sc, &seat, f)).collect::<Vec<_>>()));
        let mut tries: Vec<&calls::Try> = c.tries.iter().collect();
        tries.sort_by_key(|t| (t.date, std::cmp::Reverse(t.q), std::cmp::Reverse(t.clock)));
        m.insert("tries".into(), json!(tries.iter().map(|t| try_json(w, &sc, &seat, t)).collect::<Vec<_>>()));
        let calls_words = ["a catch", "a fumble", "the goal line", "a spot", "an interception"];
        m.insert(
            "challenges".into(),
            json!(
                c.chals
                    .iter()
                    .map(|x| json!({
                        "date": afm_engine::calendar::Date(x.date).to_string(),
                        "opp": super::team_json(w, sc.tier, x.opp as usize),
                        "link": game_link(&sc, &seat, x.date, x.opp, x.game),
                        "clock": clock_text(x.q, x.clock),
                        "call": calls_words[x.call.min(4) as usize],
                        "won": x.won,
                    }))
                    .collect::<Vec<_>>()
            ),
        );
    }
    // Each head coach's season against the model.
    let charge = Charge::new(w, sc.year);
    let mut coaches: Vec<(f64, Value)> = all
        .iter()
        .filter(|c| c.sum.weighed > 0)
        .map(|c| {
            let s = &c.sum;
            let agree = 100.0 * s.agreed as f64 / s.weighed.max(1) as f64;
            (
                agree,
                json!({
                    "coach": coach_of(w, &sc, &charge, c.team as usize),
                    "team": super::team_json(w, sc.tier, c.team as usize),
                    "mine": sc.mine == Some(c.team as usize),
                    "sums": sums_json(s),
                }),
            )
        })
        .collect();
    coaches.sort_by(|a, b| b.0.total_cmp(&a.0));
    let med = |f: &dyn Fn(&calls::Sums) -> Option<f64>| median(all.iter().filter_map(|c| f(&c.sum)).collect());
    m.insert(
        "median".into(),
        json!({
            "go_rate": med(&|s| (s.said_go > 0).then(|| 100.0 * s.went_said_go as f64 / s.said_go as f64)),
            "agreed": med(&|s| (s.weighed > 0).then(|| 100.0 * s.agreed as f64 / s.weighed as f64)),
            "lost_game": med(&|s| (s.games > 0).then(|| 100.0 * s.lost as f64 / s.games as f64)),
            "lost": med(&|s| (s.weighed > 0).then_some(s.lost as f64)),
            "timeouts": med(&|s| (s.games > 0).then(|| s.timeouts as f64 / s.games as f64)),
            "challenges": med(&|s| (s.games > 0).then_some(s.challenges as f64)),
            "won": med(&|s| (s.challenges > 0).then(|| 100.0 * s.won as f64 / s.challenges as f64)),
        }),
    );
    m.insert("coaches".into(), json!(coaches.into_iter().map(|c| c.1).collect::<Vec<_>>()));
    m.insert("clear".into(), json!(100.0 * calls::CLEAR));
    m.insert("pick".into(), json!(pick_calls(w, &sc, &all)));
    Value::Object(m)
}

fn pick_calls(w: &World, sc: &Scope, all: &[Calls]) -> Vec<Value> {
    let mut v: Vec<(String, usize)> = all
        .iter()
        .map(|c| {
            let t = c.team as usize;
            (super::team_json(w, sc.tier, t).get("name").and_then(Value::as_str).unwrap_or("").to_string(), t)
        })
        .collect();
    v.sort();
    v.into_iter().map(|(n, t)| json!([t, n])).collect()
}

// ---------- Concepts against coverages (item 27) ----------

fn is_pass(c: &str) -> bool {
    c == "other pass" || afm_engine::game::playbook::PASS_CONCEPTS.iter().any(|p| p.name == c)
}

/// Every concept-coverage cell under a side's keys: (concept, coverage) ->
/// the snaps' numbers.
fn cc_cells(s: &teams::Season, side: &str) -> BTreeMap<(String, String), teams::Situ> {
    let pre = format!("{side}|cc|");
    let mut out = BTreeMap::new();
    for (k, v) in s.cells.range(pre.clone()..) {
        let Some(rest) = k.strip_prefix(&pre) else { break };
        let mut it = rest.splitn(2, '|');
        let (Some(c), Some(cov)) = (it.next(), it.next()) else { continue };
        out.insert((c.to_string(), cov.to_string()), *v);
    }
    out
}

fn measure_of(v: &teams::Situ, how: &str) -> f64 {
    let n = v[0].max(1.0) as f64;
    match how {
        "epa" => v[2] as f64 / n,
        "ypp" => v[1] as f64 / n,
        _ => 100.0 * v[3] as f64 / n,
    }
}

fn add_situ(a: &mut teams::Situ, b: &teams::Situ) {
    for i in 0..8 {
        a[i] += b[i];
    }
}

/// The level's best concept against a coverage (at least `min` snaps).
fn best_answer(
    cells: &BTreeMap<(String, String), teams::Situ>,
    cov: &str,
    how: &str,
    min: f32,
) -> Option<(String, f64, f32)> {
    cells
        .iter()
        .filter(|((_, v), s)| v == cov && s[0] >= min)
        .map(|((c, _), s)| (c.clone(), measure_of(s, how), s[0]))
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

/// Item 27: concepts against coverages.
pub fn concepts(w: &World, args: &Value) -> Value {
    let sc = scope(w, args);
    let mut m = head(w, &sc);
    let how = match args.get("measure").and_then(Value::as_str) {
        Some("epa") => "epa",
        Some("ypp") => "ypp",
        _ => "success",
    };
    let view = match args.get("view").and_then(Value::as_str) {
        Some("offense") if sc.team.is_some() => "offense",
        Some("defense") if sc.team.is_some() => "defense",
        _ => "league",
    };
    m.insert("measure".into(), json!(how));
    m.insert("view".into(), json!(view));
    match &sc.ch {
        None => {
            m.insert("blocked".into(), json!(source_line(&sc)));
            return Value::Object(m);
        }
        Some(c) if c.depth < 2 => {
            m.insert(
                "blocked".into(),
                json!("Concepts and coverages come from film charting: your staff's play-by-play charting doesn't read them. A stronger analytics head would."),
            );
            return Value::Object(m);
        }
        _ => {}
    }
    let all = pool(w, &sc);
    if all.is_empty() {
        m.insert("blocked".into(), json!("No charted games in this season yet."));
        return Value::Object(m);
    }
    let level = sum_seasons(&all);
    let lv = cc_cells(&level, "o");
    let team = sc.team.and_then(|t| all.iter().find(|s| s.team as usize == t));
    let shown = match (view, team) {
        ("offense", Some(t)) => cc_cells(t, "o"),
        ("defense", Some(t)) => cc_cells(t, "d"),
        _ => lv.clone(),
    };
    // Coverages, most seen first; concepts, passes then runs, most run first.
    let mut cov_n: HashMap<&str, f32> = HashMap::new();
    let mut con_n: HashMap<&str, f32> = HashMap::new();
    for ((c, v), s) in &shown {
        *cov_n.entry(v.as_str()).or_default() += s[0];
        *con_n.entry(c.as_str()).or_default() += s[0];
    }
    let mut covs: Vec<&str> = cov_n.keys().copied().collect();
    covs.sort_by(|a, b| cov_n[b].total_cmp(&cov_n[a]).then(a.cmp(b)));
    let mut cons: Vec<&str> = con_n.keys().copied().collect();
    cons.sort_by(|a, b| is_pass(b).cmp(&is_pass(a)).then(con_n[b].total_cmp(&con_n[a])).then(a.cmp(b)));
    let total: f32 = cov_n.values().sum();
    let grid: Vec<Value> = cons
        .iter()
        .map(|c| {
            let mut row_all = [0f32; 8];
            let cells: Vec<Value> = covs
                .iter()
                .map(|v| match shown.get(&(c.to_string(), v.to_string())) {
                    Some(s) => {
                        add_situ(&mut row_all, s);
                        json!({ "n": s[0], "v": measure_of(s, how) })
                    }
                    None => Value::Null,
                })
                .collect();
            json!({
                "concept": c,
                "pass": is_pass(c),
                "cells": cells,
                "all": { "n": row_all[0], "v": measure_of(&row_all, how) },
            })
        })
        .collect();
    let seen: Vec<Value> = covs.iter().map(|v| json!(100.0 * cov_n[v] / total.max(1.0))).collect();
    let mut col_all: Vec<Value> = Vec::new();
    for v in &covs {
        let mut s = [0f32; 8];
        for ((_, cv), x) in &shown {
            if cv == v {
                add_situ(&mut s, x);
            }
        }
        col_all.push(json!({ "n": s[0], "v": measure_of(&s, how) }));
    }
    m.insert("coverages".into(), json!(covs));
    m.insert("seen".into(), json!(seen));
    m.insert("col_all".into(), json!(col_all));
    m.insert("grid".into(), json!(grid));
    m.insert("snaps".into(), json!(total));
    // The tiles: your offense's view.
    if let Some(t) = team {
        let off = cc_cells(t, "o");
        let mut seen_by: HashMap<&str, f32> = HashMap::new();
        for ((_, v), s) in &off {
            *seen_by.entry(v.as_str()).or_default() += s[0];
        }
        let most = seen_by.iter().max_by(|a, b| a.1.total_cmp(b.1)).map(|(v, n)| (v.to_string(), *n));
        let off_total: f32 = seen_by.values().sum();
        let mut tiles = serde_json::Map::new();
        if let Some((v, n)) = &most {
            tiles.insert("most_seen".into(), json!({ "coverage": v, "share": 100.0 * n / off_total.max(1.0) }));
            if let Some((c, x, k)) = best_answer(&off, v, how, 8.0) {
                tiles.insert("best".into(), json!({ "concept": c, "coverage": v, "v": x, "n": k }));
            }
        }
        // Costing you most: the cell with the most EPA lost, at least 8 snaps.
        if let Some(((c, v), s)) =
            off.iter().filter(|(_, s)| s[0] >= 8.0 && s[2] < 0.0).min_by(|a, b| a.1[2].total_cmp(&b.1[2]))
        {
            tiles.insert("costly".into(), json!({ "concept": c, "coverage": v, "epa": s[2], "n": s[0] }));
        }
        m.insert("tiles".into(), Value::Object(tiles));
        // Your calls against what you saw.
        let mut yours = Vec::new();
        let mut seen_sorted: Vec<(&str, f32)> = seen_by.iter().map(|(k, v)| (*k, *v)).collect();
        seen_sorted.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (v, n) in seen_sorted {
            let called = off.iter().filter(|((_, cv), _)| cv == v).max_by(|a, b| a.1[0].total_cmp(&b.1[0]));
            let Some(((c, _), s)) = called else { continue };
            let lvl = lv.get(&(c.clone(), v.to_string()));
            let best = best_answer(&lv, v, how, 30.0);
            yours.push(json!({
                "coverage": v,
                "seen": n,
                "called": c,
                "called_n": s[0],
                "yours": measure_of(s, how),
                "level": lvl.map(|x| measure_of(x, how)),
                "best": best.map(|b| json!({ "concept": b.0, "v": b.1 })),
            }));
        }
        m.insert("yours".into(), json!(yours));
    }
    // What the level is copying: the concept whose share of snaps grew most
    // since last season.
    if let Some(prev) = sc.years.iter().find(|&&y| y == sc.year - 1).filter(|_| sc.tier != Tier::Hs) {
        let mut last = Vec::new();
        let prev = *prev;
        w.tally_teams(prev, |s| {
            if s.level == sc.level as u8
                && (sc.tier != Tier::College || Some(super::season_place(w, s.team as usize, prev).0) == sc.division)
            {
                last.push(s.clone());
            }
        });
        let shares = |s: &teams::Season| {
            let cc = cc_cells(s, "o");
            let tot: f32 = cc.values().map(|v| v[0]).sum();
            let mut by: HashMap<String, f32> = HashMap::new();
            for ((c, _), v) in cc {
                *by.entry(c).or_default() += v[0];
            }
            (by, tot.max(1.0))
        };
        let (now, nt) = shares(&level);
        let (then, tt) = shares(&sum_seasons(&last));
        if tt > 1.0 {
            let grow = now
                .iter()
                .map(|(c, n)| (c.clone(), 100.0 * n / nt - 100.0 * then.get(c).copied().unwrap_or(0.0) / tt))
                .max_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((c, g)) = grow.filter(|g| g.1 > 0.5) {
                m.insert("copying".into(), json!({ "concept": c, "up": g }));
            }
        }
    }
    // Your next opponent's coverages.
    if sc.mine.is_some()
        && sc.team == sc.mine
        && sc.year == w.tally_open_season()
        && let Some(n) = crate::lane_gameday::next_game(w)
        && let Some(o) = all.iter().find(|s| s.team as usize == n.opp)
    {
        let d = cc_cells(o, "d");
        let mut by: HashMap<String, f32> = HashMap::new();
        for ((_, v), s) in &d {
            *by.entry(v.clone()).or_default() += s[0];
        }
        let tot: f32 = by.values().sum::<f32>().max(1.0);
        let pre = "d|cov3|";
        let third: HashMap<String, f32> = o
            .cells
            .range(pre.to_string()..)
            .take_while(|(k, _)| k.starts_with(pre))
            .map(|(k, v)| (k[pre.len()..].to_string(), v[0]))
            .collect();
        let tot3: f32 = third.values().sum::<f32>().max(1.0);
        let mut rows: Vec<(f32, Value)> = by
            .iter()
            .map(|(v, n)| {
                let best = best_answer(&lv, v, how, 30.0);
                (
                    *n,
                    json!({
                        "coverage": v,
                        "all": 100.0 * n / tot,
                        "third": 100.0 * third.get(v).copied().unwrap_or(0.0) / tot3,
                        "best": best.map(|b| json!({ "concept": b.0, "v": b.1 })),
                    }),
                )
            })
            .collect();
        rows.sort_by(|a, b| b.0.total_cmp(&a.0));
        m.insert(
            "next".into(),
            json!({
                "team": super::team_json(w, sc.tier, n.opp),
                "date": n.date.to_string(),
                "rows": rows.into_iter().map(|r| r.1).collect::<Vec<_>>(),
            }),
        );
    }
    m.insert("pick".into(), json!(pick_teams(w, &sc, &all)));
    Value::Object(m)
}
