//! Lane commands: the schedule page (update 2, lane 6, SC1-SC15 and the
//! minor bugs B1-B6), in the Press Box look. `call` answers the commands
//! this lane owns and returns None for any other.
//!
//! - `schedule` (`kind`, `id`, `season`): a team's season game by game from
//!   `afm_engine::world::season_page`: kickoff and network, both ranks at
//!   kickoff, the opponent's record going in and at the end, the site and
//!   the weather, the line, the crowd against the seats, the result with
//!   its game page, your running record, the rest between games, byes,
//!   leaders and your staff's player of the game from your kept games, the
//!   guarantee money, the strip, the totals, the strength of schedule, the
//!   seasons ahead and a high school's JV and freshman games.
//! - `schedule_scout`: your staff's report on an opponent from your tapes.
//! - `schedule_series`: a rivalry's series history.
//! - `year_board`: the year as a ribbon (V13, `year.rs`).

use afm_engine::calendar::{Date, Level as CalLevel, Weekday};
use afm_engine::world::World;
use afm_engine::world::season_page::{self as sp, Held, PageGame, SeasonPage};
use serde_json::{Value, json};

use crate::views::{self, TeamRef};
use crate::{Game, Reply};

mod year;

const COMMANDS: [&str; 4] = ["schedule", "schedule_scout", "schedule_series", "year_board"];

pub fn call(game: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    if !COMMANDS.contains(&cmd) {
        return None;
    }
    let Some(w) = game.world.as_ref() else { return Some(Err("No world is loaded.".into())) };
    let season = args.get("season").and_then(Value::as_i64).map(|y| y as i32);
    Some(match cmd {
        "schedule" => TeamRef::from_args(w, args).map(|t| schedule(w, t, season)),
        "schedule_scout" => TeamRef::from_args(w, args).and_then(|t| scout(w, t)),
        "year_board" => year::year(w),
        _ => TeamRef::from_args(w, args).and_then(|t| {
            // Whose side of the series: the page's team, else yours.
            let of = match args.get("of").and_then(Value::as_u64) {
                Some(o) => same_level(t, o as usize),
                None => views::mine(w).ok_or("You have no team yet.")?,
            };
            series(w, of, t)
        }),
    })
}

fn level_of(t: TeamRef) -> Option<(CalLevel, usize)> {
    match t {
        TeamRef::College(i) => Some((CalLevel::College, i)),
        TeamRef::Nfl(i) => Some((CalLevel::Nfl, i)),
        TeamRef::HighSchool(i) => Some((CalLevel::HighSchool, i)),
        TeamRef::Other(_) => None,
    }
}

fn same_level(t: TeamRef, i: usize) -> TeamRef {
    match t {
        TeamRef::College(_) => TeamRef::College(i),
        TeamRef::Nfl(_) => TeamRef::Nfl(i),
        TeamRef::HighSchool(_) => TeamRef::HighSchool(i),
        TeamRef::Other(_) => TeamRef::Other(i),
    }
}

// ------------------------------------------------------------------ names and places

/// A program, its stadium now, and the name it had before a year.
type StadiumRow = (&'static str, &'static str, Option<(&'static str, i32)>);

/// Real FBS stadiums (current names; a few renamed or new ones by the year
/// the name took over, from memory). Every other program plays in its own
/// "Stadium".
const COLLEGE_STADIUMS: &[StadiumRow] = &[
    ("Air Force", "Falcon Stadium", None),
    ("Akron", "InfoCision Stadium", None),
    ("Alabama", "Bryant-Denny Stadium", None),
    ("Appalachian State", "Kidd Brewer Stadium", None),
    ("Arizona", "Arizona Stadium", None),
    ("Arizona State", "Mountain America Stadium", Some(("Sun Devil Stadium", 2023))),
    ("Arkansas", "Donald W. Reynolds Razorback Stadium", None),
    ("Arkansas State", "Centennial Bank Stadium", None),
    ("Army", "Michie Stadium", None),
    ("Auburn", "Jordan-Hare Stadium", None),
    ("Ball State", "Scheumann Stadium", None),
    ("Baylor", "McLane Stadium", Some(("Floyd Casey Stadium", 2014))),
    ("Boise State", "Albertsons Stadium", None),
    ("Boston College", "Alumni Stadium", None),
    ("Bowling Green", "Doyt L. Perry Stadium", None),
    ("Buffalo", "UB Stadium", None),
    ("BYU", "LaVell Edwards Stadium", None),
    ("California", "California Memorial Stadium", None),
    ("Central Michigan", "Kelly/Shorts Stadium", None),
    ("Charlotte", "Jerry Richardson Stadium", None),
    ("Cincinnati", "Nippert Stadium", None),
    ("Clemson", "Memorial Stadium", None),
    ("Coastal Carolina", "Brooks Stadium", None),
    ("Colorado", "Folsom Field", None),
    ("Colorado State", "Canvas Stadium", Some(("Hughes Stadium", 2017))),
    ("Delaware", "Delaware Stadium", None),
    ("Duke", "Wallace Wade Stadium", None),
    ("East Carolina", "Dowdy-Ficklen Stadium", None),
    ("Eastern Michigan", "Rynearson Stadium", None),
    ("Florida Atlantic", "FAU Stadium", None),
    ("FIU", "Pitbull Stadium", Some(("FIU Stadium", 2023))),
    ("Florida", "Ben Hill Griffin Stadium", None),
    ("Florida State", "Doak Campbell Stadium", None),
    ("Fresno State", "Valley Children's Stadium", Some(("Bulldog Stadium", 2021))),
    ("Georgia", "Sanford Stadium", None),
    ("Georgia Southern", "Paulson Stadium", None),
    ("Georgia State", "Center Parc Stadium", Some(("Georgia Dome", 2017))),
    ("Georgia Tech", "Bobby Dodd Stadium", None),
    ("Hawaii", "Clarence T.C. Ching Complex", Some(("Aloha Stadium", 2021))),
    ("Houston", "TDECU Stadium", Some(("Robertson Stadium", 2014))),
    ("Illinois", "Memorial Stadium", None),
    ("Indiana", "Memorial Stadium", None),
    ("Iowa", "Kinnick Stadium", None),
    ("Iowa State", "Jack Trice Stadium", None),
    ("Jacksonville State", "AmFirst Stadium", Some(("Burgess-Snow Field", 2023))),
    ("James Madison", "Bridgeforth Stadium", None),
    ("Kansas", "David Booth Kansas Memorial Stadium", None),
    ("Kansas State", "Bill Snyder Family Stadium", None),
    ("Kennesaw State", "Fifth Third Stadium", None),
    ("Kent State", "Dix Stadium", None),
    ("Kentucky", "Kroger Field", Some(("Commonwealth Stadium", 2017))),
    ("Liberty", "Williams Stadium", None),
    ("Louisiana", "Cajun Field", None),
    ("Louisiana Tech", "Joe Aillet Stadium", None),
    ("Louisville", "L&N Federal Credit Union Stadium", Some(("Cardinal Stadium", 2023))),
    ("LSU", "Tiger Stadium", None),
    ("Marshall", "Joan C. Edwards Stadium", None),
    ("Maryland", "SECU Stadium", Some(("Maryland Stadium", 2023))),
    ("Memphis", "Simmons Bank Liberty Stadium", None),
    ("Miami (FL)", "Hard Rock Stadium", Some(("Miami Orange Bowl", 2008))),
    ("Miami (OH)", "Yager Stadium", None),
    ("Michigan", "Michigan Stadium", None),
    ("Michigan State", "Spartan Stadium", None),
    ("Middle Tennessee", "Johnny \"Red\" Floyd Stadium", None),
    ("Minnesota", "Huntington Bank Stadium", Some(("Hubert H. Humphrey Metrodome", 2009))),
    ("Mississippi State", "Davis Wade Stadium", None),
    ("Missouri", "Faurot Field", None),
    ("Missouri State", "Plaster Stadium", None),
    ("Navy", "Navy-Marine Corps Memorial Stadium", None),
    ("NC State", "Carter-Finley Stadium", None),
    ("Nebraska", "Memorial Stadium", None),
    ("Nevada", "Mackay Stadium", None),
    ("New Mexico", "University Stadium", None),
    ("New Mexico State", "Aggie Memorial Stadium", None),
    ("North Carolina", "Kenan Stadium", None),
    ("North Dakota State", "Fargodome", None),
    ("North Texas", "DATCU Stadium", Some(("Fouts Field", 2011))),
    ("Northern Illinois", "Huskie Stadium", None),
    ("Northwestern", "Ryan Field", None),
    ("Notre Dame", "Notre Dame Stadium", None),
    ("Ohio", "Peden Stadium", None),
    ("Ohio State", "Ohio Stadium", None),
    ("Oklahoma", "Gaylord Family Oklahoma Memorial Stadium", None),
    ("Oklahoma State", "Boone Pickens Stadium", None),
    ("Old Dominion", "S.B. Ballard Stadium", None),
    ("Ole Miss", "Vaught-Hemingway Stadium", None),
    ("Oregon", "Autzen Stadium", None),
    ("Oregon State", "Reser Stadium", None),
    ("Penn State", "Beaver Stadium", None),
    ("Pittsburgh", "Acrisure Stadium", Some(("Heinz Field", 2022))),
    ("Purdue", "Ross-Ade Stadium", None),
    ("Rice", "Rice Stadium", None),
    ("Rutgers", "SHI Stadium", None),
    ("Sacramento State", "Hornet Stadium", None),
    ("Sam Houston", "Bowers Stadium", None),
    ("San Diego State", "Snapdragon Stadium", Some(("Qualcomm Stadium", 2022))),
    ("San Jose State", "CEFCU Stadium", None),
    ("SMU", "Gerald J. Ford Stadium", None),
    ("South Alabama", "Hancock Whitney Stadium", Some(("Ladd-Peebles Stadium", 2020))),
    ("South Carolina", "Williams-Brice Stadium", None),
    ("South Florida", "Raymond James Stadium", None),
    ("Southern Miss", "M.M. Roberts Stadium", None),
    ("Stanford", "Stanford Stadium", None),
    ("Syracuse", "JMA Wireless Dome", Some(("Carrier Dome", 2022))),
    ("TCU", "Amon G. Carter Stadium", None),
    ("Temple", "Lincoln Financial Field", None),
    ("Tennessee", "Neyland Stadium", None),
    ("Texas", "Darrell K Royal-Texas Memorial Stadium", None),
    ("Texas A&M", "Kyle Field", None),
    ("Texas State", "UFCU Stadium", None),
    ("Texas Tech", "Jones AT&T Stadium", None),
    ("Toledo", "Glass Bowl", None),
    ("Troy", "Veterans Memorial Stadium", None),
    ("Tulane", "Yulman Stadium", Some(("Louisiana Superdome", 2014))),
    ("Tulsa", "H.A. Chapman Stadium", None),
    ("UAB", "Protective Stadium", Some(("Legion Field", 2021))),
    ("UCF", "FBC Mortgage Stadium", None),
    ("UCLA", "Rose Bowl", None),
    ("UConn", "Rentschler Field", None),
    ("ULM", "Malone Stadium", None),
    ("UMass", "McGuirk Alumni Stadium", None),
    ("UNLV", "Allegiant Stadium", Some(("Sam Boyd Stadium", 2020))),
    ("USC", "Los Angeles Memorial Coliseum", None),
    ("Utah", "Rice-Eccles Stadium", None),
    ("Utah State", "Maverik Stadium", None),
    ("UTEP", "Sun Bowl Stadium", None),
    ("UTSA", "Alamodome", None),
    ("Vanderbilt", "FirstBank Stadium", Some(("Vanderbilt Stadium", 2022))),
    ("Virginia", "Scott Stadium", None),
    ("Virginia Tech", "Lane Stadium", None),
    ("Wake Forest", "Allegacy Federal Credit Union Stadium", None),
    ("Washington", "Husky Stadium", None),
    ("Washington State", "Martin Stadium", None),
    ("West Virginia", "Milan Puskar Stadium", None),
    ("Western Kentucky", "Houchens Industries-L.T. Smith Stadium", None),
    ("Western Michigan", "Waldo Stadium", None),
    ("Wisconsin", "Camp Randall Stadium", None),
    ("Wyoming", "War Memorial Stadium", None),
];

/// A college program's home stadium in a season.
pub(crate) fn college_stadium(w: &World, p: usize, year: i32) -> String {
    let prog = &w.programs[p];
    match COLLEGE_STADIUMS.iter().find(|r| r.0 == prog.real_name) {
        Some((_, _, Some((old, from)))) if year < *from => (*old).to_string(),
        Some((_, now, _)) => (*now).to_string(),
        None => format!("{} Stadium", w.program_name(p)),
    }
}

/// A high school's home field.
pub(crate) fn school_field(w: &World, h: usize) -> String {
    format!("{} Field", w.school_name(h))
}

/// The host's stadium, town and seats for a game.
fn venue_of(w: &World, level: CalLevel, host: usize, g: &PageGame, year: i32) -> (String, String, u32) {
    if g.neutral {
        let (town, state) = g.site.clone().unwrap_or_default();
        let place = if state.is_empty() { town.clone() } else { format!("{town}, {state}") };
        // An NFL game away from both teams' homes with a town of its own
        // is one played abroad; anything else unnamed is a neutral site.
        let name = g.site_stadium.clone().unwrap_or_else(|| {
            if level == CalLevel::Nfl && g.site.is_some() && !g.playoff {
                "International game".to_string()
            } else {
                "Neutral site".to_string()
            }
        });
        return (name, place, 0);
    }
    match level {
        CalLevel::Nfl => {
            let (name, seats) = sp::nfl_stadium(w, host, year);
            let (town, st) = afm_engine::world::finance::nfl::city(w, host, year);
            (name, format!("{town}, {st}"), seats)
        }
        CalLevel::HighSchool => {
            let s = &w.high_schools[host];
            (school_field(w, host), format!("{}, {}", s.town, w.states[s.state].code), sp::hs_seats(w, host))
        }
        _ => {
            let p = &w.programs[host];
            (college_stadium(w, host, year), format!("{}, {}", p.city, w.states[p.state].code), p.stadium)
        }
    }
}

// ------------------------------------------------------------------ words

fn wlt(r: [u32; 3]) -> String {
    if r[2] > 0 { format!("{}-{}-{}", r[0], r[1], r[2]) } else { format!("{}-{}", r[0], r[1]) }
}

fn kick_text(k: (u16, u16), level: CalLevel) -> String {
    let (h, m) = k;
    let (h12, ap) = match h {
        0 => (12, "AM"),
        1..=11 => (h, "AM"),
        12 => (12, "PM"),
        _ => (h - 12, "PM"),
    };
    let zone = if level == CalLevel::HighSchool { "" } else { " ET" };
    format!("{h12}:{m:02} {ap}{zone}")
}

fn day_name(d: Weekday) -> &'static str {
    match d {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }
}

/// The rest before a game (SC5): days since the last one, off a bye, a
/// short week, an odd day.
fn rest_json(g: &PageGame, level: CalLevel) -> Value {
    let usual = match level {
        CalLevel::Nfl => Weekday::Sun,
        CalLevel::HighSchool => Weekday::Fri,
        _ => Weekday::Sat,
    };
    let wd = g.date.weekday();
    let odd = (wd != usual && !g.bowl && !g.playoff).then(|| day_name(wd));
    let (text, short) = match g.rest {
        None => ("Opener".to_string(), false),
        Some(d) if d >= 13 && !g.bowl && !g.playoff => (format!("Off a bye ({d} days)"), false),
        Some(d) if d < 7 => (format!("Short week ({d} days)"), true),
        Some(d) => (format!("{d} days"), false),
    };
    let text = match odd {
        Some(x) => format!("{text}, {x}"),
        None => text,
    };
    json!({ "text": text, "short": short })
}

fn line_json(g: &PageGame) -> Value {
    let Some([mine, total]) = g.line else { return Value::Null };
    let spread = if mine == 0.0 { "Pick".to_string() } else { format!("{mine:+.1}") };
    let against = g.score.map(|[us, them]| {
        let cover = us as f32 - them as f32 + mine;
        let pts = (us + them) as f32;
        let ats = if cover > 0.0 {
            "Covered"
        } else if cover < 0.0 {
            "Didn't cover"
        } else {
            "Push"
        };
        let ou = if pts > total {
            "over"
        } else if pts < total {
            "under"
        } else {
            "push on the total"
        };
        format!("{ats}, {ou}")
    });
    json!({ "spread": spread, "total": total, "against": against, "cover": g.score.map(|[u, t]| u as f32 - t as f32 + mine), "over": g.score.map(|[u, t]| (u + t) as f32 - total) })
}

// ------------------------------------------------------------------ the seat's kept games

/// Your kept game for a row: its index on the game pages.
fn kept_game(w: &World, level: CalLevel, team: usize, g: &PageGame) -> Option<usize> {
    w.seat_games()
        .iter()
        .position(|k| k.level == level && k.date == g.date && k.teams.contains(&team) && k.teams.contains(&g.opp))
}

/// The leaders of a kept game (SC8): your passer, rusher and receiver,
/// their top passer, and your staff's player of the game by film grade.
fn leaders(w: &World, k: usize, team: usize) -> Value {
    let Some(g) = w.seat_games().get(k) else { return Value::Null };
    let side = if g.teams[0] == team { 0 } else { 1 };
    let name = |id: u32| views::person_index(w, id).map_or(format!("No. {id}"), |i| w.people[i].name.clone());
    let mine = &g.stats.players[side];
    let theirs = &g.stats.players[1 - side];
    let best = |m: &std::collections::BTreeMap<u32, afm_engine::game::stats::StatLine>,
                f: &dyn Fn(&afm_engine::game::stats::StatLine) -> f64| {
        m.iter().filter(|(_, l)| f(l) > 0.0).max_by(|a, b| f(a.1).total_cmp(&f(b.1))).map(|(&id, l)| (id, l.clone()))
    };
    let pass = best(mine, &|l| l.pass_yds + 0.01 * l.att as f64).map(|(id, l)| {
        json!({ "id": id, "name": name(id), "line": format!("{}-{}, {:.0} yds, {} TD{}", l.cmp, l.att, l.pass_yds, l.pass_td, if l.int > 0 { format!(", {} INT", l.int) } else { String::new() }) })
    });
    let rush = best(mine, &|l| l.rush_yds + 0.01 * l.rush as f64).map(|(id, l)| {
        json!({ "id": id, "name": name(id), "line": format!("{}-{:.0}, {} TD", l.rush, l.rush_yds, l.rush_td) })
    });
    let rec = best(mine, &|l| l.rec_yds + 0.01 * l.rec as f64).map(|(id, l)| {
        json!({ "id": id, "name": name(id), "line": format!("{}-{:.0}{}", l.rec, l.rec_yds, if l.rec_td > 0 { format!(", {} TD", l.rec_td) } else { String::new() }) })
    });
    let opp = best(theirs, &|l| l.pass_yds + 0.01 * l.att as f64)
        .map(|(id, l)| json!({ "id": id, "name": name(id), "line": format!("{:.0} pass yds", l.pass_yds) }))
        .or_else(|| {
            best(theirs, &|l| l.rush_yds)
                .map(|(id, l)| json!({ "id": id, "name": name(id), "line": format!("{:.0} rush yds", l.rush_yds) }))
        });
    // The staff's player of the game: the best film grade on your side.
    let pog = mine.iter().filter(|(_, l)| l.graded >= 8).max_by(|a, b| a.1.grade().total_cmp(&b.1.grade())).map(
        |(&id, l)| {
            let line = if l.att > 0 {
                format!("{}-{}, {:.0} yds, {} TD", l.cmp, l.att, l.pass_yds, l.pass_td)
            } else if l.rush >= 5 {
                format!("{} car, {:.0} yds, {} TD", l.rush, l.rush_yds, l.rush_td)
            } else if l.targets > 0 && l.rec_yds > 0.0 {
                format!("{} rec, {:.0} yds", l.rec, l.rec_yds)
            } else if l.sacks > 0.0 || l.ints > 0 {
                format!(
                    "{} {}, {:.1} sacks, {} INT",
                    l.tackles + l.assists,
                    afm_engine::world::folk::noun(l.tackles + l.assists, "tackle"),
                    l.sacks,
                    l.ints
                )
            } else if l.tackles + l.assists > 0 {
                format!(
                    "{} {}, {:.1} TFL",
                    l.tackles + l.assists,
                    afm_engine::world::folk::noun(l.tackles + l.assists, "tackle"),
                    l.tfl
                )
            } else {
                format!("{} snaps", l.snaps_off + l.snaps_def + l.snaps_st)
            };
            json!({ "id": id, "name": name(id), "line": line, "grade": (l.grade() * 10.0).round() / 10.0 })
        },
    );
    json!({ "pass": pass, "rush": rush, "rec": rec, "opp": opp, "pog": pog })
}

// ------------------------------------------------------------------ the page

/// Whether a game's kickoff and network are out yet: the NFL's with the
/// schedule in May, a college game's 12 days ahead (the networks' six- and
/// twelve-day picks), weeknights and the first three weeks from June, and
/// the bowls and playoffs as they are set. High school's are known all year.
pub(super) fn announced(w: &World, level: CalLevel, g: &PageGame, first: Option<Date>) -> bool {
    if g.score.is_some() || g.won.is_some() || g.date < w.today {
        return true;
    }
    match level {
        CalLevel::HighSchool => true,
        CalLevel::Nfl => w.today >= w.dates.schedule_release || g.date.year() < w.dates.season,
        _ => {
            let (y, _, _) = g.date.parts();
            let june = Date::ymd(if g.date.month() <= 2 { y - 1 } else { y }, 6, 1);
            g.date <= w.today.add(12)
                || g.bowl
                || g.playoff
                || g.date.weekday() != Weekday::Sat
                || (w.today >= june && first.is_some_and(|f| g.date <= f.add(16)))
        }
    }
}

fn row_json(
    w: &World,
    t: TeamRef,
    level: CalLevel,
    team: usize,
    p: &SeasonPage,
    k: usize,
    rec: &mut [[u32; 3]; 2],
) -> Value {
    let g = &p.games[k];
    let first = p.games.first().map(|x| x.date);
    let host = if g.home { team } else { g.opp };
    let (stadium, place, seats) = venue_of(w, level, host, g, p.year);
    let out = announced(w, level, g, first);
    let result = match (g.score, g.won, g.cancelled) {
        (_, _, true) => json!({ "text": "Called off" }),
        (Some([us, them]), _, _) => {
            let wl = if us > them {
                "W"
            } else if us < them {
                "L"
            } else {
                "T"
            };
            let slot = match wl {
                "W" => 0,
                "L" => 1,
                _ => 2,
            };
            rec[0][slot] += 1;
            if g.league {
                rec[1][slot] += 1;
            }
            json!({ "wl": wl, "us": us, "them": them })
        }
        (None, Some(won), _) => {
            let i = if won { 0 } else { 1 };
            rec[0][i] += 1;
            if g.league {
                rec[1][i] += 1;
            }
            json!({ "wl": if won { "W" } else { "L" } })
        }
        _ => Value::Null,
    };
    let played = g.score.is_some() || g.won.is_some();
    let game_index = kept_game(w, level, team, g);
    let fee = g.fee.as_ref().map(|f| {
        let amt = sp::money_words(f.amount as f64);
        let text = match f.how.as_str() {
            "paid" => format!("We paid {amt} to bring them in"),
            "received" => format!("They paid us {amt} for the trip"),
            _ => format!("A neutral-site game: {amt} to each side"),
        };
        json!({ "text": text, "amount": f.amount, "how": f.how, "signed": f.signed })
    });
    let (wd, rest) = (g.date.weekday().short(), rest_json(g, level));
    json!({
        "kind": "game",
        "i": k,
        "date": views::date_text(g.date),
        "day": wd,
        "kick": if out { g.kick.map(|x| kick_text(x, level)) } else { None },
        "tv": if out && !g.tv.is_empty() { Some(g.tv.clone()) } else { None },
        "at": if g.neutral { "vs (n)" } else if g.home { "vs" } else { "at" },
        "home": g.home,
        "neutral": g.neutral,
        "opp": views::team_brief(w, same_level(t, g.opp)),
        "ranks": g.ranks,
        "opp_in": wlt(g.opp_in),
        "opp_final": wlt(g.opp_final),
        "opp_now": !season_over(w, level, team, p.year),
        "opp_winning": g.opp_final[0] > g.opp_final[1],
        "league": g.league,
        "tags": g.tags,
        "name": g.name,
        "trophy": g.trophy,
        "rivalry": g.rivalry,
        "bowl": g.bowl,
        "playoff": g.playoff,
        "stadium": stadium,
        "place": place,
        "seats": seats,
        "indoor": g.indoor,
        "weather": g.weather,
        "line": line_json(g),
        "crowd": if g.crowd > 0 { Some(g.crowd) } else { None },
        "result": result,
        "played": played,
        "record": if played { Some(wlt(rec[0])) } else { None },
        "league_record": if played && g.league { Some(wlt(rec[1])) } else { None },
        "rest": rest,
        "fee": fee,
        "why": g.why,
        "game": game_index,
        "leaders": game_index.map(|k| leaders(w, k, team)),
        "mine": views::is_mine(w, t),
    })
}

/// The totals line (SC10), label over value, by level.
fn totals(p: &SeasonPage, level: CalLevel) -> Value {
    let c = |pred: &dyn Fn(&PageGame) -> bool| {
        let mut r = [0u32; 3];
        for g in p.games.iter().filter(|g| pred(g)) {
            match (g.score, g.won) {
                (Some([a, b]), _) => {
                    if a > b {
                        r[0] += 1
                    } else if a < b {
                        r[1] += 1
                    } else {
                        r[2] += 1
                    }
                }
                (None, Some(true)) => r[0] += 1,
                (None, Some(false)) => r[1] += 1,
                _ => {}
            }
        }
        r
    };
    let all = c(&|_| true);
    let home = c(&|g| g.home && !g.neutral);
    let away = c(&|g| !g.home && !g.neutral);
    let neutral = c(&|g| g.neutral);
    let ranked = c(&|g| g.ranks[1].is_some());
    let winning = c(&|g| g.opp_final[0] > g.opp_final[1]);
    let playoffs = c(&|g| g.playoff);
    let one_score = c(&|g| g.score.is_some_and(|[a, b]| a.abs_diff(b) <= 8));
    let scored: Vec<[u32; 2]> = p.games.iter().filter_map(|g| g.score).collect();
    let n = scored.len().max(1) as f64;
    let (pf, pa) = scored.iter().fold((0u32, 0u32), |a, s| (a.0 + s[0], a.1 + s[1]));
    let (mut ats, mut ou) = ([0u32; 3], [0u32; 3]);
    for g in &p.games {
        if let (Some([mine, total]), Some([a, b])) = (g.line, g.score) {
            let cover = a as f32 - b as f32 + mine;
            ats[if cover > 0.0 {
                0
            } else if cover < 0.0 {
                1
            } else {
                2
            }] += 1;
            let pts = (a + b) as f32;
            ou[if pts > total {
                0
            } else if pts < total {
                1
            } else {
                2
            }] += 1;
        }
    }
    let shutouts = scored.iter().filter(|s| s[1] == 0).count();
    let pts =
        |x: u32| if scored.is_empty() { "-".to_string() } else { format!("{x} <small>{:.1}</small>", x as f64 / n) };
    let margin = pf as i64 - pa as i64;
    // No place before a game is played.
    let place = p
        .place
        .filter(|_| all.iter().sum::<u32>() > 0)
        .map_or("-".to_string(), |(a, b)| format!("{} of {b}", ordinal(a)));
    let league = match level {
        CalLevel::Nfl => "Division",
        CalLevel::HighSchool => "District",
        _ => "Conference",
    };
    let mut cells = vec![json!(["Home", wlt(home)]), json!(["Away", wlt(away)])];
    match level {
        CalLevel::HighSchool => {
            cells.push(json!([
                "Playoffs",
                if playoffs.iter().sum::<u32>() > 0 { wlt(playoffs) } else { "-".to_string() }
            ]));
            cells.push(json!(["Vs ranked", wlt(ranked)]));
            cells.push(json!(["One-score games", wlt(one_score)]));
            cells.push(json!(["Shutouts", shutouts.to_string()]));
        }
        CalLevel::Nfl => {
            cells.push(json!(["Neutral", wlt(neutral)]));
            cells.push(json!(["Vs winning teams", wlt(winning)]));
            cells.push(json!(["Against the spread", wlt(ats)]));
            cells.push(json!(["Over / under", wlt(ou)]));
        }
        _ => {
            cells.push(json!(["Neutral", wlt(neutral)]));
            cells.push(json!(["Vs ranked", wlt(ranked)]));
            cells.push(json!(["Against the spread", wlt(ats)]));
            cells.push(json!(["Over / under", wlt(ou)]));
        }
    }
    cells.push(json!(["Points for", pts(pf)]));
    cells.push(json!(["Points against", pts(pa)]));
    cells.push(json!(["Margin", if scored.is_empty() { "-".to_string() } else { format!("{margin:+}") }]));
    cells.push(json!([format!("{league} finish"), place]));
    let league_rec = c(&|g| g.league);
    json!({ "overall": wlt(all), "league": wlt(league_rec), "cells": cells, "games": all.iter().sum::<u32>() })
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

/// The seasons the picker offers: the team's book of seasons, the held
/// ones and the next one once its schedule is out (B5).
fn seasons(w: &World, t: TeamRef, level: CalLevel, i: usize, cur: Option<&SeasonPage>) -> Vec<(i32, String)> {
    let mut v: Vec<(i32, String)> = match t {
        TeamRef::College(p) => {
            w.programs[p].seasons.iter().map(|s| (s.year, format!("{}-{}", s.wins, s.losses))).collect()
        }
        TeamRef::Nfl(c) => {
            w.nfl_seasons.iter().filter(|s| s.team == c).map(|s| (s.year, wlt([s.wins, s.losses, s.ties]))).collect()
        }
        TeamRef::HighSchool(h) => {
            w.high_schools[h].seasons.iter().map(|s| (s.year, format!("{}-{}", s.wins, s.losses))).collect()
        }
        TeamRef::Other(_) => Vec::new(),
    };
    let (this, last) = sp::held_years(w, level);
    for y in [last, this].into_iter().flatten() {
        if !v.iter().any(|x| x.0 == y) {
            // The season under way: its record so far.
            let held = if Some(y) == this { cur.cloned() } else { sp::page(w, level, i, Held::Last) };
            let rec = held
                .map(|p| {
                    let r = p.games.iter().fold([0u32; 3], |mut r, g| {
                        match (g.score, g.won) {
                            (Some([a, b]), _) if a > b => r[0] += 1,
                            (Some([a, b]), _) if a < b => r[1] += 1,
                            (Some(_), _) => r[2] += 1,
                            (None, Some(true)) => r[0] += 1,
                            (None, Some(false)) => r[1] += 1,
                            _ => {}
                        }
                        r
                    });
                    if r.iter().sum::<u32>() == 0 { String::new() } else { wlt(r) }
                })
                .unwrap_or_default();
            v.push((y, rec));
        }
    }
    v.sort_by_key(|x| x.0);
    v
}

/// The season the page opens on: the one under way, else the last one
/// played until the new season is a month off (February shows last season,
/// with the next one a click away).
fn default_season(w: &World, level: CalLevel, i: usize, cur: Option<&SeasonPage>) -> Option<i32> {
    let (this, last) = sp::held_years(w, level);
    let started = cur.is_some_and(|p| {
        p.games.iter().any(|g| g.score.is_some() || g.won.is_some())
            || p.games.first().is_some_and(|g| w.today.days_to(g.date) <= 30)
    });
    if started || last.is_some() {
        return if started { this.or(last) } else { last };
    }
    // A high school holds only this season; your own last one is kept.
    let prev = this.map(|y| y - 1).filter(|&y| sp::kept(w, level, i).iter().any(|k| k.year == y));
    prev.or(this)
}

/// Whether a season's games are all played: a past football year, the
/// title game played, or the state's season done.
fn season_over(w: &World, level: CalLevel, team: usize, year: i32) -> bool {
    let y = &w.dates;
    if year != y.season {
        return year < y.season;
    }
    match level {
        CalLevel::Nfl => false,
        CalLevel::HighSchool => w.hs_weeks(team).is_some_and(|x| x.2),
        _ => w.today > y.title_game,
    }
}

fn schedule(w: &World, t: TeamRef, want: Option<i32>) -> Value {
    let Some((level, i)) = level_of(t) else {
        // Another league's season: the plain list it always had.
        let mut v = views::schedule(w, t);
        v["other"] = json!(true);
        return v;
    };
    let mine = views::is_mine(w, t);
    // This season's page, built once for the picker, the default and the note.
    let cur = sp::page(w, level, i, Held::This);
    let list = seasons(w, t, level, i, cur.as_ref());
    let (this, last) = sp::held_years(w, level);
    let year = want.or_else(|| default_season(w, level, i, cur.as_ref())).or_else(|| list.last().map(|x| x.0));
    // The next season once its dates are out, and what it opens with.
    let next_note = next_note(w, t, level, cur.as_ref());
    let page: Option<SeasonPage> = match year {
        Some(y) if Some(y) == this => cur,
        Some(y) if Some(y) == last => sp::page(w, level, i, Held::Last),
        Some(y) => sp::kept(w, level, i).into_iter().find(|k| k.year == y).cloned(),
        None => None,
    };
    let summary = if page.is_none() { year.and_then(|y| summary(w, t, y)) } else { None };
    let mut rows: Vec<Value> = Vec::new();
    let mut strip: Vec<Value> = Vec::new();
    let (mut totals_v, mut sos_v, mut finish, mut league_name, mut rank_label) =
        (Value::Null, Value::Null, String::new(), String::new(), String::new());
    let mut jv: Vec<Value> = Vec::new();
    let mut nflhide = false;
    if let Some(p) = page.as_ref() {
        // The NFL's dates come out in May: until then the new season is its opponents.
        nflhide = level == CalLevel::Nfl
            && p.year == w.dates.season
            && w.today < w.dates.schedule_release
            && p.year >= w.year;
        let mut rec = [[0u32; 3]; 2];
        let mut byes = p.byes.iter().peekable();
        // Byes wait for the next game with a score: the strip shows gaps
        // between games played, never after the last one.
        let mut pending: Vec<Value> = Vec::new();
        for k in 0..p.games.len() {
            while let Some(&&b) = byes.peek() {
                if b < p.games[k].date && !nflhide {
                    rows.push(json!({ "kind": "bye", "date": views::date_text(b) }));
                    pending.push(json!({ "bye": true, "date": views::date_text(b) }));
                    byes.next();
                } else {
                    break;
                }
            }
            let mut r = row_json(w, t, level, i, p, k, &mut rec);
            if nflhide {
                r["date"] = json!(null);
                r["day"] = json!(null);
                r["kick"] = json!(null);
                r["tv"] = json!(null);
                r["rest"] = json!(null);
            }
            if let Some(res) = r["result"].as_object()
                && let (Some(us), Some(them)) =
                    (res.get("us").and_then(Value::as_i64), res.get("them").and_then(Value::as_i64))
            {
                strip.append(&mut pending);
                strip.push(json!({ "i": k, "margin": us - them, "title": format!("{} {} {}: {} {us}-{them}", r["date"].as_str().unwrap_or(""), r["at"].as_str().unwrap_or(""), r["opp"]["name"].as_str().unwrap_or(""), res.get("wl").and_then(Value::as_str).unwrap_or("")) }));
            }
            rows.push(r);
        }
        totals_v = totals(p, level);
        sos_v = p.sos.as_ref().filter(|s| s.of > 0 && s.wins + s.losses + s.ties > 0).map_or(Value::Null, |s| {
            let n = s.wins + s.losses + s.ties;
            let pct = if n == 0 { 0.0 } else { (s.wins as f64 + 0.5 * s.ties as f64) / n as f64 };
            json!({
                "rank": s.rank, "of": s.of, "rank_text": ordinal(s.rank),
                "record": wlt([s.wins, s.losses, s.ties]),
                "pct": format!("{pct:.3}").trim_start_matches('0').to_string(),
                "pool": s.pool,
                "bar": if s.of > 1 { 100.0 - (s.rank - 1) as f64 / (s.of - 1) as f64 * 100.0 } else { 100.0 },
            })
        });
        finish = p.finish.clone();
        league_name = p.league_name.clone();
        rank_label = p.rank_label.clone();
        jv = jv_rows(w, i, p);
    }
    let future = match t {
        TeamRef::College(p) => sp::college_ahead(w, p, w.year + 1),
        TeamRef::Nfl(c) => sp::nfl_ahead(w, c),
        TeamRef::HighSchool(h) => sp::hs_ahead(w, h),
        TeamRef::Other(_) => Vec::new(),
    };
    let future: Vec<Value> = future
        .into_iter()
        .map(|a| {
            json!({
                "year": a.year, "title": a.title,
                "lines": a.lines.iter().map(|l| json!({ "opp": l.opp.map(|o| views::team_brief(w, same_level(t, o))), "what": l.what, "note": l.note })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let future_note = match level {
        CalLevel::Nfl => {
            "The rotation fixes 14 of 17 opponents years ahead; the same-place games follow each season's finish."
        }
        CalLevel::HighSchool => {
            "District games follow the two-year cycle; non-district dates are booked in the spring."
        }
        _ => "Non-conference games are booked 5 to 10 years ahead. Fees are what each contract says.",
    };
    let next_opp = views::next_game(w, t).and_then(|n| n.get("opp").cloned());
    let sub = year.map(|y| {
        let mut s = format!("{y}");
        if !league_name.is_empty() {
            s.push_str(&format!(" · {league_name}"));
        }
        if page.is_some() && Some(y) != this && Some(y) != last {
            s.push_str(" · kept from your season");
        }
        s
    });
    json!({
        "team": views::team_brief(w, t),
        "mine": mine,
        "level": match level { CalLevel::Nfl => "nfl", CalLevel::HighSchool => "hs", _ => "college" },
        "season": year,
        "seasons": list.iter().rev().take(40).map(|(y, r)| json!({ "year": y, "record": r, "held": Some(*y) == this || Some(*y) == last || sp::kept(w, level, i).iter().any(|k| k.year == *y) })).collect::<Vec<_>>(),
        "sub": sub,
        "next": next_note,
        "finish": finish,
        "league_name": league_name,
        "rank_label": rank_label,
        "rows": rows,
        "strip": strip,
        "totals": totals_v,
        "sos": sos_v,
        "jv": if level == CalLevel::HighSchool && mine { Some(jv) } else { None },
        "future": future,
        "future_note": future_note,
        "next_opp": next_opp,
        "summary": summary,
        "dates_hidden": nflhide,
        "can_scout": w.seat_team().is_some_and(|(l, _)| l == level),
        "release": views::date_text(w.dates.schedule_release),
    })
}

/// A season the world no longer holds game by game: its line in the book.
fn summary(w: &World, t: TeamRef, y: i32) -> Option<Value> {
    let coach = |c: Option<usize>| c.and_then(|c| w.coaches.get(c)).map(|c| c.name.clone());
    match t {
        TeamRef::College(p) => w.programs[p].seasons.iter().find(|s| s.year == y).map(|s| {
            json!({ "record": format!("{}-{}", s.wins, s.losses), "coach": coach(s.coach), "pf": s.totals.points_for, "pa": s.totals.points_against, "conf_title": s.conference_title, "title": s.national_title })
        }),
        TeamRef::Nfl(c) => w.nfl_seasons.iter().find(|s| s.team == c && s.year == y).map(|s| {
            json!({ "record": wlt([s.wins, s.losses, s.ties]), "coach": coach(s.coach), "title": s.round == 5 })
        }),
        TeamRef::HighSchool(h) => w.high_schools[h].seasons.iter().find(|s| s.year == y).map(|s| {
            json!({ "record": format!("{}-{}", s.wins, s.losses), "coach": coach(s.coach), "pf": s.points[0], "pa": s.points[1], "title": s.state_title })
        }),
        TeamRef::Other(_) => None,
    }
}

/// The line beside the picker: the next season's dates and opener.
fn next_note(w: &World, t: TeamRef, level: CalLevel, cur: Option<&SeasonPage>) -> String {
    let (this, _) = sp::held_years(w, level);
    let Some(y) = this else { return String::new() };
    let Some(p) = cur else { return String::new() };
    if p.games.iter().any(|g| g.score.is_some() || g.won.is_some()) {
        return String::new();
    }
    let Some(g) = p.games.first() else {
        return format!("The {y} schedule comes out before the season.");
    };
    let opp = views::team_name(w, same_level(t, g.opp));
    match level {
        CalLevel::Nfl if w.today < w.dates.schedule_release => format!(
            "Opponents for {y} are set by the rotation. The league releases the dates on {}.",
            views::date_text(w.dates.schedule_release)
        ),
        _ => {
            let at = if g.neutral {
                format!("vs {opp}{}", g.site.as_ref().map_or(String::new(), |s| format!(" in {}", s.0)))
            } else if g.home {
                format!("vs {opp}")
            } else {
                format!("at {opp}")
            };
            format!("{y} dates are out. Opener {} {at}.", views::date_text(g.date))
        }
    }
}

/// A high school's JV and freshman games (SC15): played, and the Thursdays
/// still to come against Friday's opponent.
fn jv_rows(w: &World, h: usize, p: &SeasonPage) -> Vec<Value> {
    let mut out: Vec<(Date, Value)> = p
        .sub
        .iter()
        .map(|s| {
            let res = s
                .score
                .map(|[a, b]| json!({ "wl": if a > b { "W" } else if a < b { "L" } else { "T" }, "us": a, "them": b }));
            (
                s.date,
                json!({
                    "date": views::date_text(s.date),
                    "squad": s.squad,
                    "kick": if s.squad == "JV" { "6:00 PM" } else { "4:30 PM" },
                    "at": if s.home { "vs" } else { "at" },
                    "opp": views::team_brief(w, TeamRef::HighSchool(s.opp)),
                    "result": res,
                    "note": s.note,
                }),
            )
        })
        .collect();
    use afm_engine::world::front_office::school::{Squad, fields, squads, your_school};
    if your_school(w) == Some(h) {
        // A team plays only when the school fields it and it has players
        // (a freshman class short of 22 moves up a team).
        let ours = squads(w);
        let (jv, fr) = fields(w, h);
        let jv = jv && ours.iter().any(|x| x.1 == Squad::Jv);
        let fr = fr && ours.iter().any(|x| x.1 == Squad::Freshman);
        for g in p.games.iter().filter(|g| !g.playoff && g.score.is_none() && g.won.is_none() && g.date > w.today) {
            let d = g.date.add(-1);
            let (ojv, ofr) = fields(w, g.opp);
            for (squad, on, theirs) in [("JV", jv, ojv), ("Freshman", fr, ofr)] {
                if !on || out.iter().any(|x| x.0 == d && x.1["squad"] == squad) {
                    continue;
                }
                let note = (!theirs).then(|| {
                    format!(
                        "No {squad} game: {} doesn't field a {squad} team.",
                        views::team_name(w, TeamRef::HighSchool(g.opp))
                    )
                });
                out.push((
                    d,
                    json!({
                        "date": views::date_text(d),
                        "squad": squad,
                        "kick": if squad == "JV" { "6:00 PM" } else { "4:30 PM" },
                        "at": if g.home { "at" } else { "vs" },
                        "opp": views::team_brief(w, TeamRef::HighSchool(g.opp)),
                        "result": null,
                        "note": note,
                    }),
                ));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0).then(a.1["squad"].as_str().cmp(&b.1["squad"].as_str())));
    out.into_iter().map(|x| x.1).collect()
}

// ------------------------------------------------------------------ side panels

/// Your staff's scouting report on an opponent (B2), from your tapes.
fn scout(w: &World, t: TeamRef) -> Reply {
    let Some((level, i)) = level_of(t) else { return Err("No report for that league.".into()) };
    let (my_level, _) = w.seat_team().ok_or("You have no team yet.")?;
    if my_level != level {
        return Err("Your staff scouts the teams at your own level.".into());
    }
    let r = crate::lane_gameday::scout_report(w, i).unwrap_or(Value::Null);
    let next = views::next_game(w, views::mine(w).ok_or("You have no team yet.")?);
    let is_next = next.as_ref().and_then(|n| n.get("opp")).and_then(|o| o.get("ref")).is_some_and(|r| *r == t.json());
    Ok(json!({
        "team": views::team_brief(w, t),
        "report": r,
        "tapes": w.tapes_of(i).len(),
        "next": is_next,
    }))
}

/// A rivalry's series history (B3): the record, the trophy and who holds
/// it, and the meetings the world still holds.
fn series(w: &World, me: TeamRef, t: TeamRef) -> Reply {
    match (me, t) {
        (TeamRef::College(p), TeamRef::College(o)) => {
            let line = afm_engine::world::stories::traditions::college_rivalries(w, p)
                .into_iter()
                .find(|r| r.rival == o)
                .ok_or("No listed rivalry with them.")?;
            Ok(json!({
                "game": line.game,
                "trophy": line.trophy,
                "rival": views::team_brief(w, t),
                "me": views::team_brief(w, me),
                "wins": line.wins[0], "losses": line.wins[1], "ties": line.ties,
                "holder": line.holder.map(|h| views::team_brief(w, TeamRef::College(h))),
                "real": line.real,
                "since": line.since,
                "meetings": meetings(w, CalLevel::College, p, o),
            }))
        }
        (TeamRef::HighSchool(h), TeamRef::HighSchool(o)) => {
            let trophy = afm_engine::world::stories::traditions::hs_trophy(w, h, o).ok_or("No rivalry with them.")?;
            // The record starts the first time the rivalry is played on record.
            let r = w
                .stories
                .traditions
                .hs_rivals
                .iter()
                .find(|r| r.schools.contains(&(h as u32)) && r.schools.contains(&(o as u32)));
            let k = r.map_or(0, |r| (r.schools[1] == h as u32) as usize);
            Ok(json!({
                "game": format!("The {} game", w.school_name(o)),
                "trophy": trophy,
                "rival": views::team_brief(w, t),
                "me": views::team_brief(w, me),
                "wins": r.map_or(0, |r| r.wins[k]), "losses": r.map_or(0, |r| r.wins[1 - k]), "ties": 0,
                "holder": r.and_then(|r| r.holder).map(|x| views::team_brief(w, TeamRef::HighSchool(x as usize))),
                "real": false,
                "since": r.map(|r| r.since),
                "meetings": meetings(w, CalLevel::HighSchool, h, o),
            }))
        }
        _ => Err("No rivalry with them.".into()),
    }
}

/// The meetings between two teams that the world still holds: this
/// season's and last season's games, and your kept seasons.
fn meetings(w: &World, level: CalLevel, a: usize, b: usize) -> Vec<Value> {
    let mut pages: Vec<SeasonPage> =
        [Held::This, Held::Last].into_iter().filter_map(|h| sp::page(w, level, a, h)).collect();
    for k in sp::kept(w, level, a) {
        if !pages.iter().any(|p| p.year == k.year) {
            pages.push(k.clone());
        }
    }
    pages.sort_by_key(|p| std::cmp::Reverse(p.year));
    let name = |x: usize| match level {
        CalLevel::HighSchool => w.school_name(x).to_string(),
        _ => w.program_name(x).to_string(),
    };
    let mut out = Vec::new();
    for p in &pages {
        for g in p.games.iter().filter(|g| g.opp == b) {
            let text = match (g.score, g.won) {
                (Some([us, them]), _) => {
                    let (winner, hi, lo) = if us >= them { (a, us, them) } else { (b, them, us) };
                    format!("{} {hi}-{lo}", name(winner))
                }
                (None, Some(won)) => format!("{} won", name(if won { a } else { b })),
                _ => "Still to play".to_string(),
            };
            let host = if g.neutral {
                g.site.as_ref().map_or("a neutral site".to_string(), |s| s.0.clone())
            } else if g.home {
                name(a)
            } else {
                name(b)
            };
            out.push(json!({ "year": p.year, "text": text, "at": host }));
        }
    }
    out
}
