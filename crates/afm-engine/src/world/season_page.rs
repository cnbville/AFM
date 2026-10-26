//! The schedule page's facts (update 2, lane 6, SC1-SC14 and B5): a team's
//! season game by game as its page shows it. Derived on the fly from the
//! seasons the world holds (this season for every team, last season for
//! every college and NFL team), and your own team's past seasons kept small
//! from the day each one ends (`World::kept_schedules`, about 5 KB a
//! season). Only what is public goes in: poll ranks, records, lines,
//! crowds, the weather, the networks and the money the contracts name.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::World;
use super::college::{CollegeSeason, Poll, PollKind};
use super::data::Division;
use super::nfl_year::NflSeason;
use super::schedule::GameKind;
use super::season::key_uniform;
use crate::calendar::{Date, Level};

/// Seasons of your own kept whole (the oldest go first).
const KEEP_SEASONS: usize = 40;

/// One game on a team's schedule page.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageGame {
    pub date: Date,
    /// Kickoff (hour, minute): Eastern for college and the NFL, local for
    /// high school.
    pub kick: Option<(u16, u16)>,
    /// The network, station or stream.
    pub tv: String,
    pub home: bool,
    pub neutral: bool,
    pub opp: usize,
    /// A conference, division or district game.
    pub league: bool,
    /// Level tags: "Division", "District", "FCS", "Prime time", a round.
    pub tags: Vec<String>,
    /// The game's own name: a rivalry, a bowl, a playoff or title game.
    pub name: Option<String>,
    pub trophy: Option<String>,
    /// The rivalry's place in the listed rivalries (its series history).
    pub rivalry: Option<usize>,
    pub bowl: bool,
    pub playoff: bool,
    /// Poll ranks at kickoff: yours, theirs.
    pub ranks: [Option<u8>; 2],
    /// The opponent's record going in and at the season's end (wins,
    /// losses, ties).
    pub opp_in: [u32; 3],
    pub opp_final: [u32; 3],
    /// A neutral site: (town, state code), and its stadium.
    pub site: Option<(String, String)>,
    pub site_stadium: Option<String>,
    pub indoor: bool,
    pub weather: Option<String>,
    /// Your side of the closing line (negative when favoured) and the total.
    pub line: Option<[f32; 2]>,
    /// Your points, theirs.
    pub score: Option<[u32; 2]>,
    /// Whether you won, when only the winner is on record (another high
    /// school's game).
    pub won: Option<bool>,
    pub cancelled: bool,
    pub crowd: u32,
    /// Days since your last game (none for the opener).
    pub rest: Option<i32>,
    pub fee: Option<Fee>,
    pub why: Vec<String>,
}

/// A game's money (SC13).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fee {
    /// Dollars.
    pub amount: f32,
    /// "paid": you bought the home game; "received": they bought you;
    /// "neutral": the organisers paid each side.
    pub how: String,
    /// The year the contract was signed, when one was.
    pub signed: Option<i32>,
}

/// Strength of schedule (SC11): the opponents' combined record, leaving out
/// their games against you, and its rank (1 the hardest) in the pool.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Sos {
    pub wins: u32,
    pub losses: u32,
    pub ties: u32,
    pub rank: u32,
    pub of: u32,
    pub pool: String,
}

/// A high school's JV or freshman game (SC15).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubRow {
    pub date: Date,
    pub squad: String,
    pub opp: usize,
    pub home: bool,
    pub score: Option<[u16; 2]>,
    pub note: Option<String>,
}

/// A team's season as its schedule page shows it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeasonPage {
    pub year: i32,
    pub level: Level,
    pub team: usize,
    pub games: Vec<PageGame>,
    /// Open dates between games: the Saturday, Sunday or Friday it falls on.
    pub byes: Vec<Date>,
    /// How the season ended (or stands): "No. 7 in the final AP poll".
    pub finish: String,
    /// The conference, division or district: its name, your place, of how many.
    pub league_name: String,
    pub place: Option<(u32, u32)>,
    pub sos: Option<Sos>,
    /// The poll the ranks come from ("AP", "CFP", "TX 6A").
    pub rank_label: String,
    #[serde(default)]
    pub sub: Vec<SubRow>,
}

/// Your team's past seasons as kept.
pub type KeptSeason = SeasonPage;

/// Which held season.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Held {
    This,
    Last,
}

/// A team's season page from the seasons the world holds.
pub fn page(w: &World, level: Level, team: usize, held: Held) -> Option<SeasonPage> {
    match level {
        Level::College => {
            let (cs, polls) = match held {
                Held::This => (w.college.as_ref()?, &w.college.as_ref()?.polls),
                Held::Last => (w.college_prev.as_ref().filter(|c| !c.games.is_empty())?, &w.prev_polls),
            };
            (team < w.programs.len()).then(|| college_page(w, cs, polls, team))
        }
        Level::Nfl => {
            let s = match held {
                Held::This => w.nfl_season.as_ref()?,
                Held::Last => w.nfl_prev.as_ref()?,
            };
            (team < s.div.len()).then(|| nfl_page(w, s, team))
        }
        Level::HighSchool if held == Held::This => (team < w.high_schools.len()).then(|| hs_page(w, team)).flatten(),
        _ => None,
    }
}

/// A team's rank today and the poll it is in: the AP poll or the playoff
/// committee's out today at college, the state media poll in the class
/// going into the next game at high school; the NFL has none.
pub fn rank_now(w: &World, level: Level, team: usize) -> Option<(u8, String)> {
    match level {
        Level::College => {
            let cs = w.college.as_ref()?;
            let poll = poll_at(&cs.polls, w.today, cs.year)?;
            let k = rank_in(Some(poll), team)?;
            Some((k, if poll.kind == PollKind::Cfp { "CFP" } else { "AP" }.to_string()))
        }
        Level::HighSchool => hs_rank_on(&page(w, level, team, Held::This)?),
        _ => None,
    }
}

/// A high school's class poll rank on its page: going into its next game,
/// or at the end of its season.
pub fn hs_rank_on(p: &SeasonPage) -> Option<(u8, String)> {
    let g = p.games.iter().find(|g| g.score.is_none() && g.won.is_none()).or(p.games.last())?;
    Some((g.ranks[0]?, p.rank_label.clone()))
}

/// The seasons each held page covers: (this, last), when held.
pub fn held_years(w: &World, level: Level) -> (Option<i32>, Option<i32>) {
    match level {
        // A file from before last season's games were kept holds none: its
        // book line stands in.
        Level::College => (
            w.college.as_ref().map(|c| c.year),
            w.college_prev.as_ref().filter(|c| !c.games.is_empty()).map(|c| c.year),
        ),
        Level::Nfl => (w.nfl_season.as_ref().map(|s| s.year), w.nfl_prev.as_ref().map(|s| s.year)),
        Level::HighSchool => (w.hs_season.as_ref().and_then(|h| h.states.first()).map(|s| s.start.year()), None),
        _ => (None, None),
    }
}

/// Your kept seasons for a team, newest first.
pub fn kept(w: &World, level: Level, team: usize) -> Vec<&KeptSeason> {
    let mut v: Vec<&KeptSeason> = w.kept_schedules.iter().filter(|k| k.level == level && k.team == team).collect();
    v.sort_by_key(|k| std::cmp::Reverse(k.year));
    v
}

/// Keeps your team's season as its page shows it: on the day the football
/// year turns (the season just ended), and when you take another job or
/// lose this one (the season you coached so far). Only a season you
/// coached a game of: a team you joined after its last game isn't yours.
pub(crate) fn keep_seat_season(w: &mut World) {
    let Some((level, team)) = w.seat_team() else { return };
    let Some(p) = page(w, level, team, Held::This) else { return };
    let (Some(first), Some(end)) = (p.games.first().map(|g| g.date), p.games.last().map(|g| g.date)) else { return };
    let coached = w
        .seat_games()
        .iter()
        .any(|g| g.level == level && g.teams.contains(&team) && g.date >= first.add(-1) && g.date <= end.add(1));
    if !coached {
        return;
    }
    w.kept_schedules.retain(|k| !(k.year == p.year && k.level == level && k.team == team));
    w.kept_schedules.push(p);
    if w.kept_schedules.len() > KEEP_SEASONS {
        w.kept_schedules.remove(0);
    }
}

// ------------------------------------------------------------------ shared

/// The weather at kickoff in a few words.
pub fn weather_text(temperature: f64, wind: f64, rain: bool, snow: bool, indoor: bool) -> String {
    if indoor {
        return "Indoors".to_string();
    }
    let sky = if snow {
        ", snow"
    } else if rain {
        ", rain"
    } else {
        ""
    };
    let wind = if wind >= 3.0 { format!(", wind {:.0} mph", wind) } else { String::new() };
    format!("{:.0}°{sky}{wind}", temperature)
}

/// A W-L-T line as counted from results: +1 to the right slot.
fn tally(r: &mut [u32; 3], mine: u32, theirs: u32) {
    if mine > theirs {
        r[0] += 1;
    } else if mine < theirs {
        r[1] += 1;
    } else {
        r[2] += 1;
    }
}

fn pct(r: [u32; 3]) -> f64 {
    let n = r[0] + r[1] + r[2];
    if n == 0 { 0.0 } else { (r[0] as f64 + 0.5 * r[2] as f64) / n as f64 }
}

/// Ranks a pool by strength of schedule: each team's opponents' combined
/// record (their games against it left out). Returns the team's and the
/// pool's size.
fn sos_of(team: usize, opp_sums: &HashMap<usize, [u32; 3]>) -> Option<(u32, u32)> {
    let own = *opp_sums.get(&team)?;
    if own.iter().sum::<u32>() == 0 {
        return None;
    }
    let mine = pct(own);
    let of = opp_sums.values().filter(|r| r[0] + r[1] + r[2] > 0).count() as u32;
    let rank = 1 + opp_sums.values().filter(|r| r[0] + r[1] + r[2] > 0 && pct(**r) > mine).count() as u32;
    (of > 0).then_some((rank.min(of), of))
}

/// The stadium of a bowl or neutral site by its town and season (from
/// memory: current names, with the renamed and replaced ones by years).
pub fn site_stadium(town: &str, year: i32) -> Option<&'static str> {
    Some(match town {
        "Pasadena" => "Rose Bowl",
        "New Orleans" if year >= 2021 => "Caesars Superdome",
        "New Orleans" if year >= 2011 => "Mercedes-Benz Superdome",
        "New Orleans" => "Louisiana Superdome",
        "Miami Gardens" if year >= 2016 => "Hard Rock Stadium",
        "Miami Gardens" if year >= 2010 => "Sun Life Stadium",
        "Miami Gardens" if year >= 1996 => "Dolphin Stadium",
        "Miami Gardens" => "Miami Orange Bowl",
        "Glendale" if year >= 2018 => "State Farm Stadium",
        "Glendale" if year >= 2006 => "University of Phoenix Stadium",
        "Glendale" => "Sun Devil Stadium",
        "Arlington" if year >= 2009 => "AT&T Stadium",
        "Arlington" => "Cotton Bowl Stadium",
        "Atlanta" if year >= 2017 => "Mercedes-Benz Stadium",
        "Atlanta" => "Georgia Dome",
        "Orlando" if year >= 2016 => "Camping World Stadium",
        "Orlando" => "Florida Citrus Bowl",
        "Tampa" if year >= 1998 => "Raymond James Stadium",
        "Tampa" => "Tampa Stadium",
        "Jacksonville" if year >= 2023 => "EverBank Stadium",
        "Jacksonville" if year >= 2018 => "TIAA Bank Field",
        "Jacksonville" if year >= 2010 => "EverBank Field",
        "Jacksonville" => "Jacksonville Municipal Stadium",
        "Nashville" => "Nissan Stadium",
        "Memphis" => "Simmons Bank Liberty Stadium",
        "Charlotte" => "Bank of America Stadium",
        "Houston" if year >= 2002 => "NRG Stadium",
        "Houston" => "Astrodome",
        "San Antonio" => "Alamodome",
        "El Paso" => "Sun Bowl Stadium",
        "San Diego" if year >= 2022 => "Snapdragon Stadium",
        "San Diego" => "Qualcomm Stadium",
        "Las Vegas" if year >= 2020 => "Allegiant Stadium",
        "Las Vegas" => "Sam Boyd Stadium",
        "Phoenix" => "Chase Field",
        "Tucson" => "Arizona Stadium",
        "Shreveport" => "Independence Stadium",
        "Birmingham" if year >= 2022 => "Protective Stadium",
        "Birmingham" => "Legion Field",
        "Mobile" if year >= 2020 => "Hancock Whitney Stadium",
        "Mobile" => "Ladd-Peebles Stadium",
        "Montgomery" => "Cramton Bowl",
        "Dallas" => "Cotton Bowl Stadium",
        "Fort Worth" => "Amon G. Carter Stadium",
        "Frisco" => "Toyota Stadium",
        "Boise" => "Albertsons Stadium",
        "Albuquerque" => "University Stadium",
        "Honolulu" if year >= 2023 => "Clarence T.C. Ching Complex",
        "Honolulu" => "Aloha Stadium",
        "Detroit" => "Ford Field",
        "Bronx" => "Yankee Stadium",
        "Boston" => "Fenway Park",
        "Annapolis" => "Navy-Marine Corps Memorial Stadium",
        "Boca Raton" => "FAU Stadium",
        "Nassau" => "Thomas A. Robinson Stadium",
        "Conway" => "Brooks Stadium",
        "Inglewood" => "SoFi Stadium",
        "Philadelphia" => "Lincoln Financial Field",
        "East Rutherford" => "MetLife Stadium",
        "Baltimore" => "M&T Bank Stadium",
        "Foxborough" => "Gillette Stadium",
        "Santa Clara" => "Levi's Stadium",
        _ => return None,
    })
}

// ------------------------------------------------------------------ college

/// The poll a college game's ranks come from at a date: the playoff
/// committee's once it is out (2014 on), else the AP poll.
fn poll_at(polls: &[Poll], d: Date, year: i32) -> Option<&Poll> {
    let before = |p: &&Poll| p.date <= d;
    let cfp = polls.iter().filter(|p| p.kind == PollKind::Cfp).filter(before).max_by_key(|p| p.date);
    if year >= 2014 && cfp.is_some() {
        return cfp;
    }
    polls
        .iter()
        .filter(|p| matches!(p.kind, PollKind::Ap | PollKind::ApPreseason))
        .filter(before)
        .max_by_key(|p| p.date)
}

fn rank_in(p: Option<&Poll>, team: usize) -> Option<u8> {
    p.and_then(|p| p.top.iter().position(|&t| t == team)).map(|k| k as u8 + 1)
}

/// Each program's division and conference in a held season: as the season
/// kept them, else (a file from before they were kept) its real league that
/// year for a past season, else today's.
fn leagues<'a>(w: &World, cs: &'a CollegeSeason) -> Cow<'a, [(Division, String)]> {
    if cs.confs.len() == w.programs.len() {
        return Cow::Borrowed(&cs.confs);
    }
    w.programs
        .iter()
        .map(|q| {
            let real = (cs.year < w.year && cs.year <= super::data::HISTORY_LAST + 1)
                .then(|| q.real_status(cs.year))
                .flatten();
            real.unwrap_or_else(|| (q.division, q.conference.clone()))
        })
        .collect()
}

fn college_page(w: &World, cs: &CollegeSeason, polls: &[Poll], p: usize) -> SeasonPage {
    let year = cs.year;
    let me = &w.programs[p];
    // The leagues as they were that season.
    let lg = leagues(w, cs);
    let (my_div, my_conf) = (lg[p].0, lg[p].1.as_str());
    let played = |g: &super::schedule::Game| g.played.filter(|x| !x.cancelled);
    // Every team's games, for records going in and the strength of schedule.
    let mut by_team: HashMap<usize, Vec<usize>> = HashMap::new();
    for (k, g) in cs.games.iter().enumerate() {
        by_team.entry(g.home).or_default().push(k);
        by_team.entry(g.away).or_default().push(k);
    }
    let record = |t: usize, before: Option<Date>| -> [u32; 3] {
        let mut r = [0; 3];
        for &k in by_team.get(&t).map_or(&[][..], |v| v.as_slice()) {
            let g = &cs.games[k];
            if before.is_some_and(|d| g.date >= d) {
                continue;
            }
            if let Some(x) = played(g) {
                let (mine, theirs) = if g.home == t { (x.score[0], x.score[1]) } else { (x.score[1], x.score[0]) };
                tally(&mut r, mine, theirs);
            }
        }
        r
    };
    let mut idx: Vec<usize> = by_team.get(&p).cloned().unwrap_or_default();
    idx.sort_by_key(|&k| (cs.games[k].date, k));
    let mut games = Vec::new();
    let mut last: Option<Date> = None;
    for &k in &idx {
        let g = &cs.games[k];
        let home = g.home == p;
        let opp = if home { g.away } else { g.home };
        let op = &w.programs[opp];
        let poll = poll_at(polls, g.date.add(-1), year);
        let ranks = [rank_in(poll, p), rank_in(poll, opp)];
        let mut tags = Vec::new();
        if g.conference && g.kind == GameKind::Regular {
            tags.push(w.conference_name(my_conf));
        }
        if my_div == Division::FBS && lg[opp].0 == Division::FCS {
            tags.push("FCS".to_string());
        }
        let (bowl, playoff) = (
            matches!(g.kind, GameKind::Bowl(_) | GameKind::Celebration),
            matches!(g.kind, GameKind::Playoff(_) | GameKind::Title),
        );
        if playoff {
            tags.push("Playoffs".to_string());
        }
        if g.kind == GameKind::ConferenceTitle {
            tags.push("Title game".to_string());
        }
        let riv = g.rivalry.and_then(|r| crate::calendar::data::RIVALRIES.get(r));
        let name = g.name.map(str::to_string).or_else(|| riv.map(|r| r.name_in(year).to_string()));
        let site =
            g.site.map(|s| (s.town.to_string(), w.states.get(s.state).map_or(String::new(), |x| x.code.clone())));
        let deal = super::deals::find(w, year, g.home, g.away);
        let site_stadium = deal
            .and_then(|d| d.site)
            .and_then(|k| super::deals::SITES.get(k as usize))
            .map(|s| s.stadium_in(year).to_string())
            .or_else(|| g.site.and_then(|s| site_stadium(s.town, year)).map(str::to_string));
        let x = played(g);
        let indoor = x.map_or(g.site.is_some_and(|s| s.indoor), |x| x.indoor);
        let weather = g.played.filter(|x| !x.cancelled).map(|x| {
            let mut t = weather_text(x.temperature, x.wind, x.rain, x.snow, x.indoor);
            if x.delay > 0 {
                t.push_str(&format!(", {} min lightning delay", x.delay));
            }
            t
        });
        let score = x.map(|x| if home { [x.score[0], x.score[1]] } else { [x.score[1], x.score[0]] });
        // The line once it closed, or the week's line from the Sunday before.
        let line = g
            .line
            .or_else(|| {
                (g.played.is_none() && g.date >= w.today && g.date <= w.today.add(6) && w.college_year() == Some(year))
                    .then(|| super::finance::lines::college(w, g.home, g.away, g.site.is_some()))
                    .flatten()
            })
            .map(|[s, t]| [if home { s } else { -s }, t]);
        let fee = super::deals::money(w, year, g, k).map(|m| match m {
            super::deals::Money::Guarantee(x) => Fee {
                amount: x as f32,
                how: if home { "paid" } else { "received" }.to_string(),
                signed: deal.map(|d| d.signed),
            },
            super::deals::Money::Neutral(x) => {
                Fee { amount: x as f32, how: "neutral".to_string(), signed: deal.map(|d| d.signed) }
            }
        });
        let ranked = ranks.iter().filter(|r| r.is_some()).count() as f64;
        let draw = (0.5 * (me.prestige_rel + op.prestige_rel) + 0.12 * ranked + if riv.is_some() { 0.12 } else { 0.0 })
            .clamp(0.0, 1.0);
        let tv = super::broadcast::college(&super::broadcast::CollegeGame {
            year,
            date: g.date,
            kick: g.kickoff,
            kind: g.kind,
            name: g.name,
            home_conf: &lg[g.home].1,
            away_conf: &lg[g.away].1,
            home_div: lg[g.home].0,
            away_div: lg[g.away].0,
            home: &w.programs[g.home].real_name,
            away: &w.programs[g.away].real_name,
            draw,
            key: key_uniform(
                w.options.seed as u64 ^ 0x7F7F,
                (g.date.0 as u64) << 24 ^ (g.home as u64) << 12 ^ g.away as u64,
            ),
        });
        games.push(PageGame {
            date: g.date,
            kick: Some(g.kickoff),
            tv: tv.to_string(),
            home,
            neutral: g.site.is_some(),
            opp,
            league: g.conference && g.kind == GameKind::Regular,
            tags,
            name,
            trophy: riv.and_then(|r| r.trophy).map(str::to_string),
            rivalry: g.rivalry,
            bowl,
            playoff,
            ranks,
            opp_in: record(opp, Some(g.date)),
            opp_final: record(opp, None),
            site,
            site_stadium,
            indoor,
            weather,
            line,
            score,
            won: None,
            cancelled: g.played.is_some_and(|x| x.cancelled),
            crowd: g.attendance,
            rest: last.map(|d| d.days_to(g.date)),
            fee,
            why: g.why.iter().map(|s| s.to_string()).collect(),
        });
        last = Some(g.date);
    }
    let byes = byes_between(&games, 13, 7);
    // Strength of schedule among the division's programs.
    let mut sums: HashMap<usize, [u32; 3]> = HashMap::new();
    let finals: HashMap<usize, [u32; 3]> = by_team.keys().map(|&t| (t, record(t, None))).collect();
    for (&t, list) in &by_team {
        if lg.get(t).is_none_or(|x| x.0 != my_div) {
            continue;
        }
        let mut s = [0u32; 3];
        for &k in list {
            let g = &cs.games[k];
            let Some(x) = played(g) else { continue };
            let o = if g.home == t { g.away } else { g.home };
            let mut r = finals.get(&o).copied().unwrap_or([0; 3]);
            // Leave out the game against this team.
            let (theirs, mine) = if g.home == o { (x.score[0], x.score[1]) } else { (x.score[1], x.score[0]) };
            let slot = if theirs > mine {
                0
            } else if theirs < mine {
                1
            } else {
                2
            };
            r[slot] = r[slot].saturating_sub(1);
            for (x, y) in s.iter_mut().zip(r) {
                *x += y;
            }
        }
        sums.insert(t, s);
    }
    let sos = sos_of(p, &sums).map(|(rank, of)| {
        let s = sums[&p];
        Sos { wins: s[0], losses: s[1], ties: s[2], rank, of, pool: my_div.name().to_string() }
    });
    // Where the season ended: the final poll, else this week's.
    let last_poll = polls.iter().filter(|q| q.kind == PollKind::Final).max_by_key(|q| q.date);
    let finish = match last_poll {
        Some(f) => match rank_in(Some(f), p) {
            Some(r) => format!("No. {r} in the final AP poll"),
            None => "Unranked in the final AP poll".to_string(),
        },
        None => match poll_at(polls, w.today, year) {
            Some(q) if q.date.year() >= year => match rank_in(Some(q), p) {
                Some(r) => format!(
                    "No. {r} in this week's {}",
                    if q.kind == PollKind::Cfp { "playoff ranking" } else { "AP poll" }
                ),
                None => format!(
                    "Unranked in this week's {}",
                    if q.kind == PollKind::Cfp { "playoff ranking" } else { "AP poll" }
                ),
            },
            _ => String::new(),
        },
    };
    // The conference standing by conference record.
    let place = (!my_conf.contains("Independent")).then(|| {
        let mut members: Vec<usize> = w
            .programs
            .iter()
            .filter(|q| lg[q.id].0 == my_div && lg[q.id].1 == my_conf && q.plays(year))
            .map(|q| q.id)
            .collect();
        let key = |t: usize| {
            cs.results
                .get(t)
                .map_or((0, 0), |r| (r.conf_wins as i64 - r.conf_losses as i64, r.wins as i64 - r.losses as i64))
        };
        members.sort_by_key(|&t| (std::cmp::Reverse(key(t)), t));
        let at = members.iter().position(|&t| t == p).unwrap_or(0) as u32 + 1;
        (at, members.len() as u32)
    });
    let rank_label = if year >= 2014 && polls.iter().any(|q| q.kind == PollKind::Cfp) { "AP, then CFP" } else { "AP" };
    SeasonPage {
        year,
        level: Level::College,
        team: p,
        games,
        byes,
        finish,
        league_name: w.conference_name(my_conf),
        place,
        sos,
        rank_label: rank_label.to_string(),
        sub: Vec::new(),
    }
}

/// Open weeks: a gap of at least `gap` days between two games gets a bye
/// row a `step` after the earlier one.
fn byes_between(games: &[PageGame], gap: i32, step: i32) -> Vec<Date> {
    games
        .windows(2)
        .filter(|x| x[0].date.days_to(x[1].date) >= gap && !x[1].bowl && !x[1].playoff)
        .map(|x| x[0].date.add(step))
        .collect()
}

// ------------------------------------------------------------------ NFL

/// The NFL's network names its windows; the page shows prime time.
fn nfl_prime(window: &str) -> bool {
    matches!(
        window,
        "the season opener" | "Thursday night" | "Sunday night" | "Monday night" | "Thanksgiving" | "Christmas"
    )
}

/// A club's stadium in a season: its stint in the real list through 2026,
/// then the one it plays in: (name, seats).
pub fn nfl_stadium(w: &World, t: usize, year: i32) -> (String, u32) {
    if year <= super::franchise::PRESENT
        && let Some(s) = super::finance::data::stint(t, year)
    {
        return (s.name.to_string(), s.capacity);
    }
    w.finance.nfl.get(t).map_or((String::new(), 68_000), |c| (c.home.name.clone(), c.home.capacity))
}

fn nfl_page(w: &World, s: &NflSeason, t: usize) -> SeasonPage {
    let year = s.year;
    // Each club's record going into each week.
    let n = s.div.len();
    let mut by_week: Vec<Vec<[u32; 3]>> = vec![vec![[0; 3]; n]];
    for (k, week) in s.weeks.iter().enumerate() {
        let mut next = by_week[k].clone();
        for (g, &(h, a)) in week.iter().enumerate() {
            if let Some([x, y]) = s.slots.get(k).and_then(|v| v.get(g)).and_then(|sl| sl.score) {
                tally(&mut next[h], x, y);
                tally(&mut next[a], y, x);
            }
        }
        by_week.push(next);
    }
    let fin = |o: usize| {
        [
            s.wins.get(o).copied().unwrap_or(0),
            s.losses.get(o).copied().unwrap_or(0),
            s.ties.get(o).copied().unwrap_or(0),
        ]
    };
    let afc = |x: usize| s.conf.get(x) == Some(&0);
    let mut games = Vec::new();
    let mut last: Option<Date> = None;
    for (k, week) in s.weeks.iter().enumerate() {
        let Some(g) = week.iter().position(|&(h, a)| h == t || a == t) else { continue };
        let (h, a) = week[g];
        let Some(sl) = s.slots.get(k).and_then(|v| v.get(g)) else { continue };
        let home = h == t;
        let opp = if home { a } else { h };
        let window = sl.window.name();
        let mut tags = Vec::new();
        let div = s.div.get(t).is_some_and(|d| s.div.get(opp) == Some(d));
        if div {
            tags.push("Division".to_string());
        }
        if nfl_prime(window) {
            tags.push("Prime time".to_string());
        }
        let neutral = sl.site.is_some();
        let weather = sl.played.then(|| {
            let v = super::pro::games::venue(w, h, a, sl.at, neutral, 0.0);
            weather_text(v.cond.temperature, v.cond.wind, v.cond.rain, v.cond.snow, v.indoor)
        });
        let score = sl.score.map(|[x, y]| if home { [x, y] } else { [y, x] });
        let tv = super::broadcast::nfl(&super::broadcast::NflGame {
            year,
            date: sl.at.date,
            minute: sl.at.minute,
            window,
            away_afc: afc(a),
            home_afc: afc(h),
            round: None,
            key: key_uniform(w.options.seed as u64 ^ 0x7F7E, (sl.at.date.0 as u64) << 16 ^ (h as u64) << 8 ^ a as u64),
        });
        games.push(PageGame {
            date: sl.at.date,
            kick: Some((sl.at.minute / 60, sl.at.minute % 60)),
            tv: tv.to_string(),
            home,
            neutral,
            opp,
            league: div,
            tags,
            name: None,
            trophy: None,
            rivalry: None,
            bowl: false,
            playoff: false,
            ranks: [None, None],
            opp_in: by_week[k][opp],
            opp_final: fin(opp),
            site: sl.site.clone().map(|x| (x, String::new())),
            site_stadium: None,
            indoor: false,
            weather,
            line: sl.line.map(|[sp, tot]| [if home { sp } else { -sp }, tot]),
            score,
            won: None,
            cancelled: false,
            crowd: sl.attendance,
            rest: last.map(|d| d.days_to(sl.at.date)),
            fee: None,
            why: Vec::new(),
        });
        if let Some(gm) = games.last_mut() {
            gm.indoor = gm.weather.as_deref() == Some("Indoors");
        }
        last = Some(sl.at.date);
    }
    let regular_end = last;
    // The playoffs: each game played as the season kept it, then the round
    // still to play and the Super Bowl once both conferences have a champion.
    let round_name = |r: u32| match r {
        1 => "Wild card",
        2 => "Divisional round",
        3 => "Conference title game",
        _ => "Super Bowl",
    };
    let mut seen: HashSet<Date> = HashSet::new();
    for g in s.playoffs.iter().filter(|g| g.home == t || g.away == t) {
        let host = g.home == t;
        let opp = if host { g.away } else { g.home };
        let score = g.score.map(|[x, y]| if host { [x, y] } else { [y, x] });
        let row = playoff_row(w, s, t, opp, host, g.at.date, Some(g.at.minute), score, round_name(g.round), year, last);
        games.push(PageGame { won: score.is_none().then_some(g.home_won == host), ..row });
        seen.insert(g.at.date);
        last = Some(g.at.date);
    }
    // A file from before the playoffs were kept: your club's games as kept,
    // each round read from the league's playoff dates that season.
    let dates = if year == w.year { Some(&w.dates) } else { w.prev_dates.as_ref().filter(|d| d.season == year) };
    if s.playoffs.is_empty()
        && let (Some(end), Some(y)) = (regular_end, dates)
    {
        for sg in w
            .seat_games()
            .iter()
            .filter(|x| x.level == Level::Nfl && x.teams.contains(&t) && x.date > end && x.date <= end.add(40))
        {
            let home = sg.teams[0] == t;
            let opp = if home { sg.teams[1] } else { sg.teams[0] };
            let [x, z] = sg.stats.score.map(|v| v.max(0) as u32);
            seen.insert(sg.date);
            games.push(playoff_row(
                w,
                s,
                t,
                opp,
                home,
                sg.date,
                None,
                Some(if home { [x, z] } else { [z, x] }),
                round_name(playoff_round(y, sg.date)),
                year,
                last,
            ));
            last = Some(sg.date);
        }
    }
    for &(r, _, hi, lo, at) in s.round_games.iter().filter(|g| g.2 == t || g.3 == t) {
        if seen.contains(&at.date) {
            continue;
        }
        let opp = if hi == t { lo } else { hi };
        games.push(playoff_row(w, s, t, opp, hi == t, at.date, Some(at.minute), None, round_name(r), year, last));
        last = Some(at.date);
    }
    if s.booked
        && s.champion.is_none()
        && s.round_games.is_empty()
        && s.alive.iter().all(|c| c.len() == 1)
        && s.playoffs.iter().any(|g| g.round == 3)
        && let Some(y) = dates
    {
        let (a, b) = (s.alive[0][0], s.alive[1][0]);
        if a == t || b == t {
            let opp = if a == t { b } else { a };
            games.push(playoff_row(
                w,
                s,
                t,
                opp,
                a == t,
                y.super_bowl,
                Some(18 * 60 + 30),
                None,
                "Super Bowl",
                year,
                last,
            ));
        }
    }
    let byes = s.byes.get(t).copied().flatten().into_iter().collect();
    // Strength of schedule: the opponents' final regular-season records.
    let mut sums: HashMap<usize, [u32; 3]> = HashMap::new();
    for (k, week) in s.weeks.iter().enumerate() {
        for (g, &(h, a)) in week.iter().enumerate() {
            let sc = s.slots.get(k).and_then(|v| v.get(g)).and_then(|sl| sl.score);
            for (me, o) in [(h, a), (a, h)] {
                let mut r = fin(o);
                if let Some([x, y]) = sc {
                    let (theirs, mine) = if o == h { (x, y) } else { (y, x) };
                    let slot = if theirs > mine {
                        0
                    } else if theirs < mine {
                        1
                    } else {
                        2
                    };
                    r[slot] = r[slot].saturating_sub(1);
                }
                let e = sums.entry(me).or_insert([0; 3]);
                for (x, y) in e.iter_mut().zip(r) {
                    *x += y;
                }
            }
        }
    }
    let sos = sos_of(t, &sums).map(|(rank, of)| {
        let x = sums[&t];
        Sos { wins: x[0], losses: x[1], ties: x[2], rank, of, pool: "NFL".to_string() }
    });
    let d = s.div.get(t).copied().unwrap_or(0);
    let mut members: Vec<usize> =
        (0..n).filter(|&x| s.div[x] == d && s.active.get(x).copied().unwrap_or(true)).collect();
    // The league's own order: record, then division wins (the world's
    // tiebreak when it picks division winners).
    let key = |x: usize| if x < s.wins.len() { super::nfl_year::record_key(s, x) } else { 0.0 };
    members.sort_by(|&x, &y| key(y).total_cmp(&key(x)).then(x.cmp(&y)));
    let at = members.iter().position(|&x| x == t).unwrap_or(0) as u32 + 1;
    let league_name = super::legacy::div_name(d as u8, year).to_string();
    let out = s.out_round.get(t).copied().unwrap_or(0);
    let done = s.booked;
    let post = match out {
        5 => "won the Super Bowl".to_string(),
        r @ 1..=4 => format!("lost in the {}", round_name(r).to_lowercase().replace("wild card", "wild card round")),
        _ if s.seed.get(t).copied().flatten().is_some() => "in the playoffs".to_string(),
        _ if done => "missed the playoffs".to_string(),
        _ => String::new(),
    };
    let ord = |n: u32| match n {
        1 => "1st",
        2 => "2nd",
        3 => "3rd",
        _ => "4th",
    };
    let finish = if games.iter().any(|g| g.score.is_some()) {
        let head = format!("{} in the {league_name}{}", ord(at), if done { "" } else { " so far" });
        if post.is_empty() { head } else { format!("{head} · {post}") }
    } else {
        String::new()
    };
    SeasonPage {
        year,
        level: Level::Nfl,
        team: t,
        games,
        byes,
        finish,
        league_name,
        place: Some((at, members.len() as u32)),
        sos,
        rank_label: String::new(),
        sub: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn playoff_row(
    w: &World,
    s: &NflSeason,
    t: usize,
    opp: usize,
    home: bool,
    date: Date,
    minute: Option<u16>,
    score: Option<[u32; 2]>,
    round: &str,
    year: i32,
    last: Option<Date>,
) -> PageGame {
    let fin = |o: usize| {
        [
            s.wins.get(o).copied().unwrap_or(0),
            s.losses.get(o).copied().unwrap_or(0),
            s.ties.get(o).copied().unwrap_or(0),
        ]
    };
    let r = match round {
        "Wild card" => 1,
        "Divisional round" => 2,
        "Conference title game" => 3,
        _ => 4,
    };
    let (h, a) = if home { (t, opp) } else { (opp, t) };
    let tv = super::broadcast::nfl(&super::broadcast::NflGame {
        year,
        date,
        minute: minute.unwrap_or(16 * 60 + 30),
        window: "Sunday afternoon",
        away_afc: s.conf.get(a) == Some(&0),
        home_afc: s.conf.get(h) == Some(&0),
        round: Some(r),
        key: key_uniform(w.options.seed as u64 ^ 0x7F7D, (date.0 as u64) << 16 ^ (h as u64) << 8 ^ a as u64),
    });
    // The Super Bowl's host stadium that season.
    let host = (r >= 4).then(|| super_bowl_site(year));
    PageGame {
        date,
        kick: minute.map(|m| (m / 60, m % 60)),
        tv: tv.to_string(),
        home: home && r < 4,
        neutral: r >= 4,
        opp,
        league: false,
        tags: vec!["Playoffs".to_string(), round.to_string()],
        name: (r >= 4).then(|| "Super Bowl".to_string()),
        trophy: None,
        rivalry: None,
        bowl: false,
        playoff: true,
        ranks: [None, None],
        opp_in: fin(opp),
        opp_final: fin(opp),
        site: host.map(|x| (x.1.to_string(), x.2.to_string())),
        site_stadium: host.map(|x| x.0.to_string()),
        indoor: host.is_some_and(|x| x.3),
        weather: None,
        line: None,
        score,
        won: None,
        cancelled: false,
        crowd: 0,
        rest: last.map(|d| d.days_to(date)),
        fee: None,
        why: Vec::new(),
    }
}

/// A playoff game's round from the league's dates that season: before the
/// divisional round the wild card, then the divisional round, the
/// conference title games and the Super Bowl.
pub(crate) fn playoff_round(y: &crate::calendar::YearDates, d: Date) -> u32 {
    if d < y.divisional {
        1
    } else if d < y.conference_title_games {
        2
    } else if d < y.super_bowl {
        3
    } else {
        4
    }
}

/// The Super Bowl's host by season (the game is played the next February):
/// stadium, town, state and whether it has a roof. The real hosts through
/// the 2027 season, then the stadiums that have held it in turn (Claude's
/// pick, proposed, not confirmed).
pub fn super_bowl_site(season: i32) -> (&'static str, &'static str, &'static str, bool) {
    match season {
        ..=1984 => ("Stanford Stadium", "Stanford", "CA", false),
        1985 | 1989 => ("Louisiana Superdome", "New Orleans", "LA", true),
        1986 | 1992 => ("Rose Bowl", "Pasadena", "CA", false),
        1987 => ("Jack Murphy Stadium", "San Diego", "CA", false),
        1988 | 1994 => ("Joe Robbie Stadium", "Miami", "FL", false),
        1990 => ("Tampa Stadium", "Tampa", "FL", false),
        1991 => ("Hubert H. Humphrey Metrodome", "Minneapolis", "MN", true),
        1993 | 1999 => ("Georgia Dome", "Atlanta", "GA", true),
        1995 => ("Sun Devil Stadium", "Tempe", "AZ", false),
        1996 | 2001 => ("Louisiana Superdome", "New Orleans", "LA", true),
        1997 | 2002 => ("Qualcomm Stadium", "San Diego", "CA", false),
        1998 => ("Pro Player Stadium", "Miami", "FL", false),
        2000 | 2008 | 2020 => ("Raymond James Stadium", "Tampa", "FL", false),
        2003 => ("Reliant Stadium", "Houston", "TX", true),
        2004 => ("Alltel Stadium", "Jacksonville", "FL", false),
        2005 => ("Ford Field", "Detroit", "MI", true),
        2006 => ("Dolphin Stadium", "Miami Gardens", "FL", false),
        2007 | 2014 => ("University of Phoenix Stadium", "Glendale", "AZ", true),
        2009 => ("Sun Life Stadium", "Miami Gardens", "FL", false),
        2010 => ("Cowboys Stadium", "Arlington", "TX", true),
        2011 => ("Lucas Oil Stadium", "Indianapolis", "IN", true),
        2012 => ("Mercedes-Benz Superdome", "New Orleans", "LA", true),
        2013 => ("MetLife Stadium", "East Rutherford", "NJ", false),
        2015 | 2025 => ("Levi's Stadium", "Santa Clara", "CA", false),
        2016 => ("NRG Stadium", "Houston", "TX", true),
        2017 => ("U.S. Bank Stadium", "Minneapolis", "MN", true),
        2018 | 2027 => ("Mercedes-Benz Stadium", "Atlanta", "GA", true),
        2019 => ("Hard Rock Stadium", "Miami Gardens", "FL", false),
        2021 | 2026 => ("SoFi Stadium", "Inglewood", "CA", true),
        2022 => ("State Farm Stadium", "Glendale", "AZ", true),
        2023 => ("Allegiant Stadium", "Las Vegas", "NV", true),
        2024 => ("Caesars Superdome", "New Orleans", "LA", true),
        _ => [
            ("Caesars Superdome", "New Orleans", "LA", true),
            ("Hard Rock Stadium", "Miami Gardens", "FL", false),
            ("State Farm Stadium", "Glendale", "AZ", true),
            ("Allegiant Stadium", "Las Vegas", "NV", true),
            ("NRG Stadium", "Houston", "TX", true),
            ("Raymond James Stadium", "Tampa", "FL", false),
            ("SoFi Stadium", "Inglewood", "CA", true),
            ("Levi's Stadium", "Santa Clara", "CA", false),
            ("Mercedes-Benz Stadium", "Atlanta", "GA", true),
            ("AT&T Stadium", "Arlington", "TX", true),
        ][(season - 2028).rem_euclid(10) as usize],
    }
}

// ------------------------------------------------------------------ high school

/// Home bleachers by enrollment: a few hundred seats at a small school, ten
/// thousand and more at the biggest Texas schools (the game page's venue).
pub fn hs_seats(w: &World, h: usize) -> u32 {
    let s = &w.high_schools[h];
    let n = (s.enrollment as f64 * (1.1 + 1.6 * s.resources.clamp(0.0, 1.0) + 1.4 * s.prestige.clamp(0.0, 1.0)))
        .clamp(400.0, 14_000.0);
    (n / 50.0).round() as u32 * 50
}

/// A Friday night's crowd: the home school's usual crowd, a share of the
/// visitors', more for a rivalry, homecoming or the playoffs, less in the
/// rain or cold; the bleachers cap it, with people standing along the
/// fence on the big nights (proposed, not confirmed).
fn hs_crowd(w: &World, home: usize, away: usize, date: Date, rival: bool, playoff: bool, wx: (f64, bool)) -> u32 {
    let usual = |h: usize| super::finance::hs::usual_crowd(w, h) as f64;
    let mut x = usual(home) + 0.3 * usual(away);
    if rival {
        x *= 1.25;
    }
    if playoff {
        x *= 1.3;
    }
    if w.hs_season.as_ref().and_then(|s| s.marks.get(home).copied().flatten()).is_some_and(|m| m.0 == date) {
        x *= 1.12;
    }
    if wx.1 {
        x *= 0.82;
    }
    if wx.0 < 40.0 {
        x *= 0.9;
    }
    x *= 0.9 + 0.2 * key_uniform(w.options.seed as u64 ^ 0xC40D, (date.0 as u64) << 20 ^ home as u64);
    let cap = hs_seats(w, home) as f64 * 1.1;
    ((x.min(cap) / 10.0).round() * 10.0) as u32
}

fn hs_page(w: &World, h: usize) -> Option<SeasonPage> {
    let hs = w.hs_season.as_ref()?;
    let school = &w.high_schools[h];
    let st = hs.states.get(school.state)?;
    let year = st.start.year();
    let code = w.states[school.state].code.clone();
    let records = hs.records_by_week(school.state);
    // The state's results once (winner, loser), and each school's fixtures
    // in one pass over its groups.
    let won = hs.winners(&|x| w.high_schools.get(x).is_some_and(|s| s.state == school.state));
    let outcome = |a: usize, b: usize| -> Option<usize> {
        let (x, y) = (a as u32, b as u32);
        if won.contains(&(x, y)) {
            Some(a)
        } else if won.contains(&(y, x)) {
            Some(b)
        } else {
            None
        }
    };
    let fixtures = hs.fixtures_by_school(school.state);
    let weeks = &st.weeks;
    let rec_at =
        |t: usize, k: usize| -> [u32; 3] { records.get(&t).and_then(|v| v.get(k)).map_or([0; 3], |&(a, b)| [a, b, 0]) };
    let fin = |t: usize| -> [u32; 3] { hs.results.get(t).map_or([0; 3], |r| [r.wins, r.losses, 0]) };
    // A class's poll at a week: record, then how the team looks.
    let class_of = |t: usize| w.high_schools[t].classification;
    let strength = |t: usize| hs.results.get(t).map_or(0.0, |r| r.strength);
    let rank_at = |t: usize, k: usize| -> Option<u8> {
        let c = class_of(t);
        let score = |x: usize| {
            let r = rec_at(x, k);
            r[0] as f64 - r[1] as f64 + 0.15 * strength(x)
        };
        let mine = score(t);
        let pool: Vec<usize> = w.states[school.state]
            .schools
            .iter()
            .copied()
            .filter(|&x| x != t && class_of(x) == c && hs.results.get(x).is_some_and(|r| r.roster > 0))
            .collect();
        // No poll before there is anything to rank on.
        if mine == 0.0 && pool.iter().all(|&x| score(x) == 0.0) {
            return None;
        }
        let better = pool.iter().filter(|&&x| score(x) > mine || (score(x) == mine && x < t)).count();
        // The top 10, or a third of a small class: a poll never ranks most of it.
        let cut = 10.min((pool.len() + 1) / 3);
        (better < cut).then_some(better as u8 + 1)
    };
    let west = super::geo::STATE_CENTERS.get(school.state).is_some_and(|c| c.2 < -94.0);
    let radio = |host: usize| {
        let x = &w.high_schools[host];
        super::broadcast::high_school(year, west, x.resources, host as u64 ^ w.options.seed as u64)
    };
    let mut games = Vec::new();
    let mut last: Option<Date> = None;
    let my_district = hs.district_of(h);
    let seat_games: Vec<&super::seat::SeatGame> = w
        .seat_games()
        .iter()
        .filter(|x| {
            x.level == Level::HighSchool
                && x.teams.contains(&h)
                && x.date >= st.start.add(-21)
                && x.date <= st.finals.add(3)
        })
        .collect();
    let row = |date: Date, opp: usize, home: bool, k: Option<usize>, last: Option<Date>| -> PageGame {
        let host = if home { h } else { opp };
        let hostx = &w.high_schools[host];
        let playoff = k.is_none();
        let wx =
            crate::calendar::weather::weather_in(w.options.seed as u64, school.state, &code, &hostx.town, date, 19);
        let done = date < w.today || seat_games.iter().any(|x| x.date == date);
        let kept = seat_games.iter().find(|x| x.date == date);
        let score = kept.map(|x| {
            let [a, b] = x.stats.score.map(|v| v.max(0) as u32);
            if x.teams[0] == h { [a, b] } else { [b, a] }
        });
        let won = if score.is_some() { None } else { k.and_then(|_| outcome(h, opp)).map(|winner| winner == h) };
        let rival = super::stories::traditions::hs_rivals(w, h, opp);
        let district = !playoff && my_district.is_some() && hs.district_of(opp) == my_district;
        let mut tags = Vec::new();
        if district {
            tags.push("District".to_string());
        }
        if playoff {
            tags.push("Playoffs".to_string());
            let r = st.rounds.iter().position(|&d| d == date);
            let n = st.rounds.len();
            tags.push(match r {
                Some(i) if i + 1 == n => "State final".to_string(),
                Some(i) if i + 2 == n => "Semifinal".to_string(),
                Some(i) if i + 3 == n => "Quarterfinal".to_string(),
                Some(i) => format!("Round {}", i + 1),
                None => "Playoffs".to_string(),
            });
        }
        let ki = k.unwrap_or(weeks.len());
        let neutral = playoff && st.rounds.last() == Some(&date);
        let played = score.is_some() || won.is_some();
        let crowd = if played {
            hs_crowd(w, host, if home { opp } else { h }, date, rival, playoff, (wx.temperature, wx.rain))
        } else {
            0
        };
        PageGame {
            date,
            kick: Some((19, 0)),
            tv: radio(h),
            home,
            neutral,
            opp,
            league: district,
            tags,
            name: rival.then(|| super::stories::traditions::hs_trophy(w, h, opp)).flatten(),
            trophy: None,
            rivalry: None,
            bowl: false,
            playoff,
            ranks: [rank_at(h, ki), rank_at(opp, ki)],
            opp_in: rec_at(opp, ki),
            opp_final: fin(opp),
            site: None,
            site_stadium: None,
            indoor: false,
            weather: (done && played).then(|| weather_text(wx.temperature, wx.wind, wx.rain, wx.snow, false)),
            line: None,
            score,
            won,
            cancelled: false,
            crowd,
            rest: last.map(|d| d.days_to(date)),
            fee: None,
            why: Vec::new(),
        }
    };
    for f in w.hs_fixtures(h) {
        let k = weeks.iter().position(|&d| d == f.date);
        let home = f.home == h;
        let opp = if home { f.away } else { f.home };
        games.push(row(f.date, opp, home, k, last));
        last = Some(f.date);
    }
    // The playoffs: your school's games as kept, and every school's games
    // as the brackets played them.
    let reg_end = weeks.last().copied().unwrap_or(st.start);
    for x in seat_games.iter().filter(|x| x.date > reg_end) {
        let home = x.teams[0] == h;
        let opp = if home { x.teams[1] } else { x.teams[0] };
        games.push(row(x.date, opp, home, None, last));
        last = Some(x.date);
    }
    for (r, host, visitor, winner) in hs.playoff_games_of(h) {
        let Some(&date) = st.rounds.get(r) else { continue };
        if games.iter().any(|g| g.date == date) {
            continue;
        }
        let home = host == h;
        let g = row(date, if home { visitor } else { host }, home, None, last);
        games.push(PageGame { won: Some(winner == h), ..g });
        last = Some(date);
    }
    games.sort_by_key(|g| g.date);
    let byes = byes_between(&games, 13, 7);
    // Strength of schedule among the class.
    let class = school.classification;
    let pool: Vec<usize> = w.states[school.state]
        .schools
        .iter()
        .copied()
        .filter(|&x| class_of(x) == class && hs.results.get(x).is_some_and(|r| r.roster > 0))
        .collect();
    let mut sums: HashMap<usize, [u32; 3]> = HashMap::new();
    for &x in &pool {
        let mut s = [0u32; 3];
        for &(a, b) in fixtures.get(&x).map_or(&[][..], |v| v.as_slice()) {
            let o = if a == x { b } else { a };
            let mut r = fin(o);
            match outcome(x, o) {
                Some(win) if win == o => r[0] = r[0].saturating_sub(1),
                Some(_) => r[1] = r[1].saturating_sub(1),
                None => {}
            }
            for (x, y) in s.iter_mut().zip(r) {
                *x += y;
            }
        }
        sums.insert(x, s);
    }
    let label = w.hs_standings(h).map(|x| x.0).unwrap_or_else(|| code.clone());
    let sos = sos_of(h, &sums).map(|(rank, of)| {
        let x = sums[&h];
        Sos { wins: x[0], losses: x[1], ties: x[2], rank, of, pool: label.clone() }
    });
    // The district standing by district record.
    // The district standing as the playoffs read it.
    let place = my_district.map(|d| {
        let members: Vec<usize> =
            w.states[school.state].schools.iter().copied().filter(|&x| hs.district_of(x) == Some(d)).collect();
        let members = hs.district_order(w, school.state, members);
        let at = members.iter().position(|&x| x == h).unwrap_or(0) as u32 + 1;
        (at, members.len() as u32)
    });
    let r = hs.results.get(h).cloned().unwrap_or_default();
    let finish = if st.done {
        let poll =
            rank_at(h, weeks.len()).map_or_else(|| format!("Unranked in {label}"), |n| format!("No. {n} in {label}"));
        let title = if r.state_title { " · state champions" } else { "" };
        format!("{poll} (state media poll){title}")
    } else if r.games() > 0 {
        rank_at(h, weeks.iter().filter(|&&d| d < w.today).count())
            .map_or_else(|| format!("Unranked in {label} this week"), |n| format!("No. {n} in {label} this week"))
    } else {
        String::new()
    };
    let sub = if w.seat_school() == Some(h) {
        w.front_office
            .school
            .games
            .iter()
            .filter(|g| g.date >= st.start.add(-21) && g.date <= st.finals)
            .map(|g| SubRow {
                date: g.date,
                squad: g.squad.name().to_string(),
                opp: g.opp,
                home: g.home,
                score: g.score,
                note: g.note.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };
    Some(SeasonPage {
        year,
        level: Level::HighSchool,
        team: h,
        games,
        byes,
        finish,
        league_name: my_district.map_or_else(String::new, |_| format!("District · {label}")),
        place,
        sos,
        rank_label: label,
        sub,
    })
}

// ------------------------------------------------------------------ the seasons ahead

/// One line of a season ahead (SC14): an opponent or a group of them, and
/// what the contract or rotation says.
#[derive(Clone, Debug, Serialize)]
pub struct AheadLine {
    pub opp: Option<usize>,
    pub what: String,
    pub note: String,
}

/// A season ahead on the books.
#[derive(Clone, Debug, Serialize)]
pub struct Ahead {
    pub year: i32,
    pub title: String,
    pub lines: Vec<AheadLine>,
}

/// Dollars in short words: "$1.45M", "$575K".
pub fn money_words(x: f64) -> String {
    if x >= 1.0e6 {
        let m = (x / 1.0e4).round() / 100.0;
        let t = format!("{m:.2}");
        let t = t.trim_end_matches('0').trim_end_matches('.');
        format!("${t}M")
    } else {
        format!("${:.0}K", (x / 1.0e3).round())
    }
}

/// A college program's non-conference games under contract, season by
/// season, with its conference schedule and the rivalries outside it.
pub fn college_ahead(w: &World, p: usize, from: i32) -> Vec<Ahead> {
    let me = &w.programs[p];
    let mut out = Vec::new();
    for y in from..=w.year + super::deals::HORIZON {
        let yours = |a: usize, b: usize| {
            w.legacy
                .college
                .booked
                .iter()
                .any(|x| x.year == y && ((x.program == a && x.opp == b) || (x.program == b && x.opp == a)))
        };
        let mut lines = Vec::new();
        for d in super::deals::of_program(w, p, y).into_iter().filter(|d| d.year == y) {
            let home = d.home == p;
            let opp = if home { d.away } else { d.home };
            let fcs = w.programs[opp].division == Division::FCS;
            let note = match d.kind {
                super::deals::DealKind::Neutral => {
                    let site = d.site.and_then(|k| super::deals::SITES.get(k as usize));
                    match site {
                        Some(s) => format!(
                            "{}, {} · neutral site, {} each",
                            s.stadium_in(y),
                            s.town,
                            money_words(d.fee as f64)
                        ),
                        None => format!("neutral site, {} each", money_words(d.fee as f64)),
                    }
                }
                super::deals::DealKind::OneOff if home => {
                    format!("home · {}we pay {}", if fcs { "FCS, " } else { "" }, money_words(d.fee as f64))
                }
                super::deals::DealKind::OneOff => format!("away · they pay us {}", money_words(d.fee as f64)),
                super::deals::DealKind::HomeAndHome => {
                    format!("{} · home-and-home, game {}", if home { "home" } else { "away" }, d.leg)
                }
            };
            lines.push(AheadLine {
                opp: Some(opp),
                what: format!(
                    "{}{}",
                    if home || d.kind == super::deals::DealKind::Neutral { "" } else { "at " },
                    w.program_name(opp)
                ),
                note: format!(
                    "{note} · signed {}{}",
                    d.signed,
                    if yours(d.home, d.away) { " · booked by you" } else { "" }
                ),
            });
        }
        // Your own bookings (update 2, lane 2's ledger), unless a contract
        // above already carries one.
        for b in w
            .legacy
            .college
            .booked
            .iter()
            .filter(|b| b.year == y && (b.program == p || b.opp == p) && !super::deals::met(w, b.program, b.opp, y))
        {
            let home = (b.program == p) == b.home;
            let opp = if b.program == p { b.opp } else { b.program };
            lines.push(AheadLine {
                opp: Some(opp),
                what: format!("{}{}", if home { "" } else { "at " }, w.program_name(opp)),
                note: format!("{} · booked by you", if home { "home" } else { "away" }),
            });
        }
        // The rivalries outside the conference, every year.
        for r in crate::calendar::data::RIVALRIES.iter().filter(|r| r.a == me.real_name || r.b == me.real_name) {
            let other = if r.a == me.real_name { r.b } else { r.a };
            let Some(q) = w.programs.iter().find(|q| q.real_name == other) else { continue };
            if q.division != me.division
                || !q.plays(y)
                || (q.conference == me.conference && !me.conference.contains("Independent"))
                || super::data::rivalry_past(r.a, r.b).is_some_and(|h| !h.played(y))
            {
                continue;
            }
            let home = y % 2 == 0 && r.a == me.real_name || y % 2 != 0 && r.b == me.real_name;
            let note = match r.neutral {
                Some((_, town)) => format!("annual · {}, {town}", r.name_in(y)),
                None => format!("annual · {}", r.name_in(y)),
            };
            lines.push(AheadLine {
                opp: Some(q.id),
                what: format!("{}{}", if home || r.neutral.is_some() { "" } else { "at " }, w.program_name(q.id)),
                note,
            });
        }
        let conf = super::deals::conference_games(w, p, y);
        if conf > 0 {
            lines.push(AheadLine {
                opp: None,
                what: format!("{}: {conf} games", w.conference_name(&me.conference)),
                note: if y == w.year + 1 {
                    "opponents and dates out in the winter".to_string()
                } else {
                    "set by the conference".to_string()
                },
            });
        }
        let open = super::deals::slots(w, p, y)
            .saturating_sub(lines.iter().filter(|l| l.opp.is_some() && !l.note.starts_with("annual")).count() as u32);
        if open > 0 {
            lines.push(AheadLine {
                opp: None,
                what: format!("{open} open date{}", if open == 1 { "" } else { "s" }),
                note: "to book".to_string(),
            });
        }
        if lines.iter().any(|l| l.opp.is_some()) || y <= from + 1 {
            out.push(Ahead { year: y, title: y.to_string(), lines });
        }
    }
    out
}

/// An NFL club's opponents in the seasons ahead by the rotation: the
/// division home and away, a division of its own conference (three-year
/// cycle), one of the other conference (four-year cycle), and the
/// same-place games, which wait for this season's finish.
pub fn nfl_ahead(w: &World, t: usize) -> Vec<Ahead> {
    let Some(s) = w.nfl_season.as_ref() else { return Vec::new() };
    let Some(&d) = s.div.get(t) else { return Vec::new() };
    if d >= 8 {
        return Vec::new();
    }
    let members = |x: usize| -> Vec<usize> { (0..s.div.len()).filter(|&k| s.div[k] == x && k != t).collect() };
    let names = |v: Vec<usize>| v.iter().map(|&k| w.nfl_team_name(k)).collect::<Vec<_>>().join(", ");
    let in_div = |x: usize| (0..s.div.len()).filter(|&k| s.div[k] == x).collect::<Vec<_>>();
    let mut out = Vec::new();
    for y in s.year + 1..=s.year + 3 {
        let r = nfl_rotation(y, d);
        let games = w.changes.rules(y).nfl_games;
        let mut lines = vec![
            AheadLine {
                opp: None,
                what: super::legacy::div_name(d as u8, y).to_string(),
                note: format!("{} · home and away", names(members(d))),
            },
            AheadLine {
                opp: None,
                what: super::legacy::div_name(r.conference as u8, y).to_string(),
                note: names(in_div(r.conference)),
            },
            AheadLine {
                opp: None,
                what: super::legacy::div_name(r.cross as u8, y).to_string(),
                note: names(in_div(r.cross)),
            },
        ];
        let others: Vec<&str> = r.same_place.iter().map(|&x| super::legacy::div_name(x as u8, y)).collect();
        lines.push(AheadLine {
            opp: None,
            what: "Same-place finishers".to_string(),
            note: format!("{} · by the {} finish", others.join(", "), y - 1),
        });
        if games >= 17 {
            lines.push(AheadLine {
                opp: None,
                what: "17th game".to_string(),
                note: format!(
                    "{} same-place team, {}",
                    super::legacy::div_name(r.seventeenth as u8, y),
                    if r.seventeenth_home { "home" } else { "away" }
                ),
            });
        }
        if games >= 18 {
            lines.push(AheadLine {
                opp: None,
                what: "18th game".to_string(),
                note: format!(
                    "{} same-place team, {}",
                    super::legacy::div_name(r.eighteenth as u8, y),
                    if r.seventeenth_home { "away" } else { "home" }
                ),
            });
        }
        out.push(Ahead { year: y, title: format!("{y} (rotation)"), lines });
    }
    out
}

/// The divisions a division meets in a season by the NFL's rotation, as
/// `nfl_year`'s schedule formula draws them (divisions 0-7, AFC East first).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Rotation {
    /// The same conference's division, on a three-year cycle.
    pub conference: usize,
    /// The other conference's division, on a four-year cycle.
    pub cross: usize,
    /// The two other divisions of the conference: one same-place team each.
    pub same_place: [usize; 2],
    /// The 17th game's division (2021 on) and whether it is at home: the
    /// AFC hosts in odd years.
    pub seventeenth: usize,
    pub seventeenth_home: bool,
    /// An 18th game's division, if the league goes to 18.
    pub eighteenth: usize,
}

pub(crate) fn nfl_rotation(year: i32, d: usize) -> Rotation {
    let (c, a) = (d / 4, d % 4);
    let pairs3: [(usize, usize); 2] = match year.rem_euclid(3) {
        0 => [(0, 1), (2, 3)],
        1 => [(0, 2), (1, 3)],
        _ => [(0, 3), (1, 2)],
    };
    let partner = pairs3
        .iter()
        .find_map(|&(x, z)| {
            if x == a {
                Some(z)
            } else if z == a {
                Some(x)
            } else {
                None
            }
        })
        .unwrap_or(a);
    let rot4 = year.rem_euclid(4) as usize;
    // AFC division a meets NFC division (a + shift) % 4.
    let across =
        |shift: usize| if c == 0 { 4 + (a + shift) % 4 } else { (0..4).find(|&x| (x + shift) % 4 == a).unwrap_or(0) };
    let others: Vec<usize> = (0..4).filter(|&x| x != a && x != partner).map(|x| c * 4 + x).collect();
    Rotation {
        conference: c * 4 + partner,
        cross: across(rot4),
        same_place: [others[0], others[1]],
        seventeenth: across(rot4 + 2),
        seventeenth_home: (c == 0) == (year.rem_euclid(2) == 1),
        eighteenth: across(rot4 + 1),
    }
}

/// A high school's district cycle ahead: the same district next season in
/// a cycle's first year (home and away swapped), else realignment.
pub fn hs_ahead(w: &World, h: usize) -> Vec<Ahead> {
    let Some(hs) = w.hs_season.as_ref() else { return Vec::new() };
    let school = &w.high_schools[h];
    let Some(st) = hs.states.get(school.state) else { return Vec::new() };
    let year = st.start.year();
    let start = super::hs_year::cycle_start(year);
    let mut out = Vec::new();
    let district: Vec<usize> = hs.district_of(h).map_or_else(Vec::new, |d| {
        w.states[school.state].schools.iter().copied().filter(|&x| x != h && hs.district_of(x) == Some(d)).collect()
    });
    let label = w.hs_standings(h).map(|x| x.0).unwrap_or_default();
    if year == start && !district.is_empty() {
        let mut lines: Vec<AheadLine> = Vec::new();
        for f in w.hs_fixtures(h) {
            let opp = if f.home == h { f.away } else { f.home };
            if district.contains(&opp) {
                // The second year swaps the sites.
                let home = f.home != h;
                lines.push(AheadLine {
                    opp: Some(opp),
                    what: format!("{}{}", if home { "" } else { "at " }, w.school_name(opp)),
                    note: "district · sites swapped".to_string(),
                });
            }
        }
        lines.push(AheadLine { opp: None, what: "Non-district".to_string(), note: "booked in the spring".to_string() });
        out.push(Ahead { year: year + 1, title: format!("{} (second year of the cycle)", year + 1), lines });
    }
    let next = start + 2;
    out.push(Ahead {
        year: next,
        title: format!("{next}-{} cycle", (next + 1) % 100),
        lines: vec![
            AheadLine { opp: None, what: "Realignment".to_string(), note: format!("February {next}, by enrollment") },
            AheadLine {
                opp: None,
                what: "Enrollment".to_string(),
                note: format!("{} · {label} now", school.enrollment),
            },
        ],
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_reads_plainly() {
        assert_eq!(weather_text(79.4, 4.0, false, false, false), "79°, wind 4 mph");
        assert_eq!(weather_text(31.0, 1.0, false, true, false), "31°, snow");
        assert_eq!(weather_text(31.0, 20.0, true, false, true), "Indoors");
    }

    #[test]
    fn money_reads_like_a_contract() {
        assert_eq!(money_words(1_450_000.0), "$1.45M");
        assert_eq!(money_words(1_600_000.0), "$1.6M");
        assert_eq!(money_words(575_000.0), "$575K");
    }

    #[test]
    fn the_strength_of_schedule_ranks_hardest_first() {
        let mut m = HashMap::new();
        m.insert(1, [80, 40, 0]);
        m.insert(2, [60, 60, 0]);
        m.insert(3, [40, 80, 0]);
        assert_eq!(sos_of(1, &m), Some((1, 3)));
        assert_eq!(sos_of(3, &m), Some((3, 3)));
    }

    #[test]
    fn neutral_sites_have_their_stadiums_by_era() {
        assert_eq!(site_stadium("Atlanta", 2010), Some("Georgia Dome"));
        assert_eq!(site_stadium("Atlanta", 2025), Some("Mercedes-Benz Stadium"));
        assert_eq!(site_stadium("New Orleans", 2024), Some("Caesars Superdome"));
        assert_eq!(site_stadium("Nowhere", 2024), None);
    }
}
