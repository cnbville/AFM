//! The media around the game (C7, C9-C12): preseason magazines, the
//! documentary crews, the video game's yearly ratings, fantasy football and
//! the jersey sales lists. And the real-world tables the stories read
//! (`data/real-world/*.csv`, sources in each file and in `stories_facts.md`).

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::calendar::{Date, Kind, Level, Loudness, Weekday};
use crate::positions::Position;
use crate::world::World;
use crate::world::data::Division;

use super::lines::Fill;
use super::numbers::csv_fields;
use super::traditions::key;
use super::{GameCtx, TeamKey, news, post, team_fill, team_name};

// --- Real-world tables ----------------------------------------------------------------------------

const FINES_CSV: &str = include_str!("../../../../../data/real-world/storming_fines.csv");
const POLICY_CSV: &str = include_str!("../../../../../data/real-world/retired_numbers_policy.csv");
const HARD_KNOCKS_CSV: &str = include_str!("../../../../../data/real-world/hard_knocks.csv");
const DOCS_CSV: &str = include_str!("../../../../../data/real-world/college_docs.csv");
const MAGAZINES_CSV: &str = include_str!("../../../../../data/real-world/magazines.csv");
const GAMES_CSV: &str = include_str!("../../../../../data/real-world/video_games.csv");
const NFL_TEAMS_CSV: &str = include_str!("../../../../../data/real-world/nfl_teams.csv");

fn rows(src: &'static str) -> Vec<Vec<String>> {
    src.lines().skip(1).filter(|l| !l.trim().is_empty()).map(csv_fields).collect()
}

/// One conference rule: (conference, from, to, [first, second, later]).
type FineRule = (String, i32, i32, [Option<u32>; 3]);

/// The fines by conference and era; an empty amount is no fine.
static FINES: LazyLock<Vec<FineRule>> = LazyLock::new(|| {
    rows(FINES_CSV)
        .into_iter()
        .filter(|r| r.len() >= 6)
        .map(|r| {
            let n = |s: &str| s.parse::<u32>().ok().filter(|&x| x > 0);
            (r[0].clone(), r[1].parse().unwrap_or(0), r[2].parse().unwrap_or(i32::MAX), [n(&r[3]), n(&r[4]), n(&r[5])])
        })
        .collect()
});

/// A conference's fines for storming the field in `year` (first, second,
/// later offenses), when it had a published schedule with money in it.
pub fn storming_fines(conf: &str, year: i32) -> Option<[u32; 3]> {
    let r = FINES.iter().find(|r| r.0 == conf && year >= r.1 && year <= r.2)?;
    let first = r.3[0]?;
    Some([first, r.3[1].unwrap_or(first), r.3[2].or(r.3[1]).unwrap_or(first)])
}

/// The first season of the policy in force (offenses count from it).
pub fn storming_policy_from(conf: &str, year: i32) -> Option<i32> {
    FINES.iter().find(|r| r.0 == conf && year >= r.1 && year <= r.2).map(|r| r.1)
}

/// How a team honors its greats (C5).
#[derive(Clone, Copy, Debug, Default)]
pub struct HonorPolicy {
    pub retires_numbers: bool,
    pub never: bool,
    pub ring: bool,
    pub statues: bool,
}

static POLICIES: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    rows(POLICY_CSV).into_iter().filter(|r| r.len() >= 3).map(|r| (r[0].clone(), r[2].clone())).collect()
});

/// A team's honors policy from the table; teams not in it: NFL clubs keep a
/// ring of honor and retire numbers rarely, colleges honor jerseys and retire
/// numbers for their very best (Claude's pick, proposed, not confirmed).
pub fn honor_policy(team: &str, level: Level) -> HonorPolicy {
    let statues = matches!(
        team,
        "Oklahoma" | "Texas" | "Florida" | "USC" | "Ohio State" | "Alabama" | "Notre Dame" | "Nebraska" | "Auburn"
    );
    match POLICIES.get(team).map(String::as_str) {
        Some("never_retires") => HonorPolicy { never: true, ring: false, statues, retires_numbers: false },
        Some("retires_numbers") => HonorPolicy { retires_numbers: true, ring: false, statues, never: false },
        Some("ring_of_honor" | "hall_of_honor") => {
            HonorPolicy { retires_numbers: false, ring: true, statues, never: false }
        }
        Some("both") => {
            HonorPolicy { retires_numbers: true, ring: true, statues: statues || level == Level::Nfl, never: false }
        }
        _ => match level {
            Level::Nfl => HonorPolicy { retires_numbers: true, ring: true, statues: true, never: false },
            _ => HonorPolicy { retires_numbers: true, ring: false, statues: false, never: false },
        },
    }
}

static NFL_ROWS: LazyLock<HashMap<String, (String, u32)>> = LazyLock::new(|| {
    rows(NFL_TEAMS_CSV)
        .into_iter()
        .filter(|r| r.len() >= 10)
        .map(|r| (r[0].clone(), (r[5].clone(), r[9].parse().unwrap_or(16))))
        .collect()
});

/// A club's stadium by its current name.
pub fn nfl_stadium(name: &str) -> Option<String> {
    NFL_ROWS.get(name).map(|r| r.0.clone())
}

/// A club's TV market rank (1 is New York).
pub fn market_rank(w: &World, t: usize) -> u32 {
    let name = w.nfl_team_name(t);
    NFL_ROWS.get(&name).map_or(16, |r| r.1)
}

/// The preseason annuals on the stands in `year` (title, NFL or not).
fn magazines(year: i32) -> Vec<(String, bool)> {
    rows(MAGAZINES_CSV)
        .into_iter()
        .filter(|r| r.len() >= 4)
        .filter(|r| {
            let from: i32 = r[2].parse().unwrap_or(0);
            let to: i32 = r[3].parse().unwrap_or(i32::MAX);
            year >= from && year <= to && r[1] != "both"
        })
        .map(|r| (r[0].clone(), r[1] == "nfl"))
        .collect()
}

/// The video game titles of a year's August, (title, NFL or not, names used).
fn video_games(year: i32) -> Vec<(String, bool, bool)> {
    let real: Vec<(String, bool, bool)> = rows(GAMES_CSV)
        .into_iter()
        .filter(|r| r.len() >= 4 && r[0].parse::<i32>().ok() == Some(year))
        .map(|r| (r[1].clone(), r[2] == "nfl", r[3] == "yes"))
        .collect();
    if !real.is_empty() || year <= 2025 {
        return real;
    }
    // After the table: the series go on, one edition a year.
    let yy = (year + 1) % 100;
    vec![(format!("Madden NFL {yy:02}"), true, true), (format!("College Football {yy:02}"), false, true)]
}

/// The video game editions out on `today`: (title, the NFL game) for each.
/// Each comes out on August 12, named for the next year; the college game
/// counts only in the years it carries names.
pub fn games_out(today: Date) -> Vec<(String, bool)> {
    let y = if (today.month(), today.day()) >= (8, 12) { today.year() } else { today.year() - 1 };
    video_games(y).into_iter().filter(|g| g.1 || g.2).map(|g| (g.0, g.1)).collect()
}

// --- The store -----------------------------------------------------------------------------------

/// One preseason annual's picks.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Magazine {
    pub year: i16,
    pub title: String,
    pub nfl: bool,
    /// Its preseason order: college top 25, or every NFL club.
    pub top: Vec<u32>,
    /// Cover athletes (player ids).
    pub covers: Vec<u32>,
    /// Its all-conference picks in your conference (player ids).
    pub all_conf: Vec<u32>,
}

/// A documentary season.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Doc {
    pub year: i16,
    pub show: String,
    pub team: TeamKey,
    /// Volunteered, forced, or (high school) followed.
    pub how: String,
    pub episodes: u8,
    /// Its breakout stars (player ids).
    pub stars: Vec<u32>,
}

/// A fantasy season line (NFL).
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Fantasy {
    pub pts: f32,
    pub games: u8,
    pub last: f32,
    pub pos: u8,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Media {
    /// The last six seasons' annuals.
    pub magazines: Vec<Magazine>,
    pub docs: Vec<Doc>,
    /// This season's fantasy lines by player id, and its year.
    pub fantasy_year: i32,
    pub fantasy: HashMap<u32, Fantasy>,
    /// Each season's fantasy leaders at QB, RB, WR, TE: (year, pos, id, points).
    pub fantasy_leaders: Vec<(i16, u8, u32, f32)>,
    /// Jersey sales lists: (year, college or not, ids in order).
    pub jerseys: Vec<(i16, bool, Vec<u32>)>,
    /// The video game's top-rated players each year: (year, title, id, rating).
    pub vg_top: Vec<(i16, String, u32, u8)>,
    /// An offer to your team to be on a documentary (season, show), and
    /// your answer once given.
    pub doc_offer: Option<(i16, String)>,
    pub doc_answer: Option<bool>,
    /// This season's video game ratings by player id (set each August).
    pub ratings: HashMap<u32, u8>,
}

// --- Public standing ---------------------------------------------------------------------------------

/// Honors that make a name, by player id, read once per use: college
/// All-America picks, all-conference teams and awards, and the last three
/// seasons' Pro Bowls and All-Pro teams.
pub(crate) struct Fame(HashMap<u32, f64>);

pub(crate) fn fame(w: &World) -> Fame {
    let mut m: HashMap<u32, f64> = HashMap::new();
    for (&id, h) in &w.stats.honors {
        let v = 4.0 * h.all_america.len() as f64 + 1.0 * h.all_conference as f64 + 5.0 * h.awards.len() as f64;
        if v > 0.0 {
            *m.entry(id).or_default() += v;
        }
    }
    let yr = w.year;
    for x in w.nfl_book.pro_bowls.iter().filter(|x| x.0 >= yr - 3) {
        *m.entry(x.1).or_default() += 4.0;
    }
    for x in w.nfl_book.all_pro.iter().filter(|x| x.0 >= yr - 3) {
        *m.entry(x.2).or_default() += 6.0;
    }
    Fame(m)
}

/// How the public sees a player: what writers, voters, game makers and
/// fans go on (production, honors, pedigree and the position), with only a
/// share of what film shows (his true level) and a fixed blur of his own.
/// Never shown as a number; it orders lists.
pub(crate) fn public_score(w: &World, fame: &Fame, i: usize) -> f64 {
    let p = &w.people[i];
    let id = p.player.id;
    let ovr = p.player.overall();
    let mut s = 0.55 * ovr;
    s += 1.5 * p.stars as f64;
    s += fame.0.get(&id).copied().unwrap_or(0.0);
    let yr = w.year;
    if let Some(c) = p.pro.as_ref()
        && let Some((_, round, _, _)) = c.draft
        && c.first_year >= yr - 2
    {
        s += (6.0 - 1.5 * (round as f64 - 1.0)).max(0.0);
    }
    s += match p.player.position {
        Position::QB => 6.0,
        Position::RB | Position::WR => 2.5,
        Position::EDGE | Position::CB => 1.5,
        Position::K | Position::P | Position::LS => -6.0,
        _ => 0.0,
    };
    if w.nickname_of(id).is_some() {
        s += 2.0;
    }
    s + 6.0 * (key(id as u64, (yr as u64) << 3) - 0.5)
}

// --- C7: preseason magazines -----------------------------------------------------------------------

fn release_magazines(w: &mut World, d: Date) {
    let year = w.year;
    if !(d.month() == 6 && d.day() == 10) || !w.stories.once(10, year) {
        return;
    }
    let seed = w.options.seed;
    let fame = fame(w);
    let mine = super::seat_team(w);
    let my_conf = match mine {
        Some(TeamKey::College(p)) => Some(w.programs[p as usize].conference.clone()),
        _ => None,
    };
    for (m, (title, nfl)) in magazines(year).into_iter().enumerate() {
        let bias = |id: u64| 0.12 * (key(id, (year as u64) << 6 ^ m as u64) - 0.5);
        let (top, covers, all_conf) = if nfl {
            let mut order: Vec<(f64, usize)> = (0..w.nfl.len())
                .filter(|&t| w.nfl_seasons.iter().any(|s| s.team == t) || w.year > 0)
                .map(|t| {
                    let last = w.nfl_seasons.iter().rev().find(|s| s.team == t);
                    let wins = last.map_or(8.0, |s| s.wins as f64);
                    (wins / 17.0 + bias(t as u64), t)
                })
                .collect();
            order.sort_by(|a, b| b.0.total_cmp(&a.0));
            let top: Vec<u32> = order.iter().map(|x| x.1 as u32).collect();
            let mut stars: Vec<(f64, usize)> = (0..w.people.len())
                .filter(|&i| {
                    w.people[i].stage == crate::world::Stage::Pro
                        && w.people[i].pro.as_ref().is_some_and(|c| c.team.is_some())
                })
                .map(|i| (public_score(w, &fame, i) + 20.0 * bias(i as u64), i))
                .collect();
            stars.sort_by(|a, b| b.0.total_cmp(&a.0));
            let covers: Vec<u32> = stars.iter().take(3).map(|x| w.people[x.1].player.id).collect();
            (top, covers, Vec::new())
        } else {
            let mut order: Vec<(f64, usize)> = w
                .programs
                .iter()
                .filter(|p| p.division == Division::FBS && p.roster.len() >= 30)
                .map(|p| (crate::world::college::projected_strength(w, p.id) / 100.0 + bias(p.id as u64), p.id))
                .collect();
            order.sort_by(|a, b| b.0.total_cmp(&a.0));
            let top: Vec<u32> = order.iter().take(25).map(|x| x.1 as u32).collect();
            // Regional covers: the biggest names on the best teams.
            let mut stars: Vec<(f64, usize)> = (0..w.people.len())
                .filter(|&i| {
                    let p = &w.people[i];
                    p.stage == crate::world::Stage::College
                        && p.college.is_some_and(|c| w.programs[c].division == Division::FBS)
                })
                .map(|i| (public_score(w, &fame, i) + 20.0 * bias(i as u64), i))
                .collect();
            stars.sort_by(|a, b| b.0.total_cmp(&a.0));
            let mut covers = Vec::new();
            let mut confs: Vec<String> = Vec::new();
            for &(_, i) in &stars {
                let c = w.programs[w.people[i].college.unwrap()].conference.clone();
                if confs.contains(&c) {
                    continue;
                }
                confs.push(c);
                covers.push(w.people[i].player.id);
                if covers.len() >= 5 {
                    break;
                }
            }
            // All-conference in your conference: one per starting job.
            let mut all = Vec::new();
            if let Some(conf) = &my_conf {
                let need: [(Position, usize); 12] = [
                    (Position::QB, 1),
                    (Position::RB, 2),
                    (Position::WR, 3),
                    (Position::TE, 1),
                    (Position::OT, 2),
                    (Position::IOL, 3),
                    (Position::EDGE, 2),
                    (Position::DT, 2),
                    (Position::LB, 3),
                    (Position::CB, 2),
                    (Position::S, 2),
                    (Position::K, 1),
                ];
                for (pos, n) in need {
                    let mut c: Vec<&(f64, usize)> = stars
                        .iter()
                        .filter(|x| {
                            let p = &w.people[x.1];
                            p.player.position == pos && w.programs[p.college.unwrap()].conference == *conf
                        })
                        .take(n)
                        .collect();
                    all.extend(c.drain(..).map(|x| w.people[x.1].player.id));
                }
            }
            (top, covers, all)
        };
        w.stories.media.magazines.push(Magazine {
            year: year as i16,
            title: title.clone(),
            nfl,
            top: top.clone(),
            covers: covers.clone(),
            all_conf: all_conf.clone(),
        });
        // The headline: who it picks first.
        if let Some(&first) = top.first() {
            let t = if nfl { TeamKey::Nfl(first as u16) } else { TeamKey::College(first) };
            let f = team_fill(w, t, Fill::new()).set("mag", title.clone());
            if let Some(text) = w.stories.line(seed, "mag.top", &f, year) {
                let e = news(w, t.level(), Loudness::Small, Some(t), text);
                post(w, e);
            }
        }
        // Where it has you.
        if let Some(me) = mine
            && me.level() == if nfl { Level::Nfl } else { Level::College }
        {
            let id = match me {
                TeamKey::College(p) => p,
                TeamKey::Nfl(n) => n as u32,
                TeamKey::Hs(_) | TeamKey::Other(_) => u32::MAX,
            };
            let f = team_fill(w, me, Fill::new()).set("mag", title.clone());
            let text = match top.iter().position(|&x| x == id) {
                Some(r) => {
                    let f = f.set("rank", format!("No. {}", r + 1));
                    w.stories.line(seed, if r == 0 { "mag.top" } else { "mag.preview" }, &f, year)
                }
                None => None,
            };
            if let Some(text) = text {
                let e = news(w, me.level(), Loudness::Medium, Some(me), text);
                post(w, e);
            }
            for &c in &covers {
                let Some(i) = super::person_index(w, c) else { continue };
                if super::numbers::team_of(w, i) == Some(me) {
                    let f =
                        team_fill(w, me, Fill::new()).set("mag", title.clone()).set("name", w.people[i].name.clone());
                    if let Some(text) = w.stories.line(seed, "mag.cover_you", &f, year) {
                        let e = news(w, me.level(), Loudness::Medium, Some(me), text);
                        post(w, e);
                    }
                    super::story::memento(
                        w,
                        super::story::MementoKind::Cover,
                        format!("{title} {year}, {} on the cover", w.people[i].name),
                    );
                }
            }
            for &c in all_conf.iter().take(30) {
                let Some(i) = super::person_index(w, c) else { continue };
                if super::numbers::team_of(w, i) == Some(me) && m == 0 {
                    let conf = my_conf.clone().unwrap_or_default();
                    let f =
                        Fill::new().set("mag", title.clone()).set("name", w.people[i].name.clone()).set("conf", conf);
                    if let Some(text) = w.stories.line(seed, "mag.allconf", &f, year) {
                        let e = news(w, me.level(), Loudness::Small, Some(me), text);
                        post(w, e);
                    }
                }
            }
        }
        // The cover stars themselves (the first magazine of the year only).
        if m == 0 {
            for &c in covers.iter().take(2) {
                let Some(i) = super::person_index(w, c) else { continue };
                let Some(t) = super::numbers::team_of(w, i) else { continue };
                let f = team_fill(w, t, Fill::new()).set("mag", title.clone()).set("name", w.people[i].name.clone());
                if let Some(text) = w.stories.line(seed, "mag.cover", &f, year) {
                    let e = news(w, t.level(), Loudness::Small, Some(t), text);
                    post(w, e);
                }
            }
        }
    }
    let keep = year - 6;
    w.stories.media.magazines.retain(|m| m.year as i32 >= keep);
}

/// Midseason: the cover jinx, when a cover athlete's team is losing.
fn cover_jinx(w: &mut World, d: Date) {
    if !(d.month() == 10 && d.day() == 20) || !w.stories.once(11, w.year) {
        return;
    }
    let year = w.year;
    let seed = w.options.seed;
    let covers: Vec<u32> = w
        .stories
        .media
        .magazines
        .iter()
        .filter(|m| m.year as i32 == year && !m.nfl)
        .flat_map(|m| m.covers.clone())
        .collect();
    for c in covers {
        let Some(i) = super::person_index(w, c) else { continue };
        let Some(TeamKey::College(p)) = super::numbers::team_of(w, i) else { continue };
        let r = w.college_record(p as usize);
        if r.losses >= 3 && r.losses > r.wins {
            let f = team_fill(w, TeamKey::College(p), Fill::new())
                .set("name", w.people[i].name.clone())
                .set("record", format!("{}-{}", r.wins, r.losses));
            if let Some(text) = w.stories.line(seed, "mag.jinx", &f, year) {
                let e = news(w, Level::College, Loudness::Small, Some(TeamKey::College(p)), text);
                post(w, e);
                break;
            }
        }
    }
}

// --- C9: documentary crews -------------------------------------------------------------------------

/// Hard Knocks' club for a season: the real one through 2025; after it,
/// the 2013 rule: a club that volunteers, else the league picks one that
/// can't refuse (no new head coach, no playoffs in the last two seasons, not
/// on the show in the last ten years).
fn hard_knocks(w: &World, year: i32) -> Option<(usize, String)> {
    let real: Vec<Vec<String>> = rows(HARD_KNOCKS_CSV)
        .into_iter()
        .filter(|r| r.len() >= 4 && r[0].parse::<i32>().ok() == Some(year) && r[2] == "training camp")
        .collect();
    if year <= 2025 {
        let r = real.first()?;
        let t = (0..w.nfl.len()).find(|&t| w.nfl_team_name(t) == r[1])?;
        return Some((t, r[3].clone()));
    }
    let shown: Vec<usize> = w
        .stories
        .media
        .docs
        .iter()
        .filter(|d| d.show == "Hard Knocks" && d.year as i32 >= year - 10)
        .filter_map(|d| if let TeamKey::Nfl(t) = d.team { Some(t as usize) } else { None })
        .collect();
    let playoffs = |t: usize| w.nfl_seasons.iter().any(|s| s.team == t && s.year >= year - 2 && s.round > 0);
    let new_coach = |t: usize| {
        let a = w.nfl_seasons.iter().rev().find(|s| s.team == t).and_then(|s| s.coach);
        w.nfl.get(t).and_then(|x| x.head_coach) != a
    };
    let seat = super::seat_team(w);
    // A club volunteers about half the time (big markets, owners who want the show).
    let volunteer = (0..w.nfl.len())
        .filter(|t| !shown.contains(t) && Some(TeamKey::Nfl(*t as u16)) != seat)
        .map(|t| (key(t as u64, year as u64) * (1.3 - market_rank(w, t) as f64 / 40.0), t))
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .filter(|x| x.0 > 0.62)
        .map(|x| x.1);
    if let Some(t) = volunteer {
        return Some((t, "volunteered".into()));
    }
    (0..w.nfl.len())
        .filter(|&t| !shown.contains(&t) && !playoffs(t) && !new_coach(t))
        .min_by(|&a, &b| key(a as u64, year as u64 ^ 9).total_cmp(&key(b as u64, year as u64 ^ 9)))
        .map(|t| (t, "forced".into()))
}

/// The college, junior college and high school series of a season: the
/// real ones by year; after them, a streaming series picks a big program
/// about one year in three, a junior college series one in two, a high
/// school quarterback series one in four (proposed, not confirmed).
fn other_docs(w: &World, year: i32) -> Vec<(String, TeamKey, String)> {
    let mut out = Vec::new();
    for r in rows(DOCS_CSV) {
        if r.len() < 5 || r[0].parse::<i32>().ok() != Some(year) || r[1].starts_with('(') {
            continue;
        }
        let subject = r[3].as_str();
        let t = match r[4].as_str() {
            "college" => w
                .programs
                .iter()
                .find(|p| p.real_name == subject || subject.starts_with(&p.real_name))
                .map(|p| TeamKey::College(p.id as u32)),
            "juco" => w
                .programs
                .iter()
                .find(|p| p.division == Division::JUCO && subject.contains(&p.real_name))
                .map(|p| TeamKey::College(p.id as u32)),
            _ => None,
        };
        if let Some(t) = t {
            out.push((r[1].clone(), t, "followed".into()));
        }
    }
    if year > 2025 {
        let big = w.programs.iter().filter(|p| p.division == Division::FBS).max_by(|a, b| {
            (a.prestige + key(a.id as u64, year as u64)).total_cmp(&(b.prestige + key(b.id as u64, year as u64)))
        });
        if key(year as u64, 0xD0C) < 0.33
            && let Some(p) = big
        {
            out.push((format!("All Access: {}", p.real_name), TeamKey::College(p.id as u32), "volunteered".into()));
        }
        let juco = w
            .programs
            .iter()
            .filter(|p| p.division == Division::JUCO && p.roster.len() >= 40)
            .min_by(|a, b| key(a.id as u64, year as u64).total_cmp(&key(b.id as u64, year as u64)));
        if key(year as u64, 0x1C0) < 0.5
            && let Some(p) = juco
        {
            out.push(("Last Chance".into(), TeamKey::College(p.id as u32), "volunteered".into()));
        }
    }
    out
}

/// July: the season's documentary crews are named; your club gets the
/// league's call or an offer. August: the episodes, one a week.
fn documentaries(w: &mut World, d: Date) {
    let year = w.year;
    let seed = w.options.seed;
    // May 15: a series that wants your team asks first (from 2026; a club
    // the league orders to do it has no say).
    if d.month() == 5 && d.day() == 15 && year > 2025 && w.stories.once(21, year) {
        offer_doc(w, year);
    }
    if d.month() == 6 && d.day() == 1 && w.stories.once(20, year) {
        let mut picks: Vec<(String, TeamKey, String)> = Vec::new();
        let me = super::seat_team(w);
        // Your answer, or the owner's call when you gave none.
        let offer = w.stories.media.doc_offer.clone().filter(|o| o.0 as i32 == year);
        let said = offer.as_ref().map(|_| {
            w.stories
                .media
                .doc_answer
                .unwrap_or_else(|| me.is_some_and(|t| key(year as u64, team_code(t) ^ 0x0FFE) < 0.5))
        });
        if let (Some((_, show)), Some(yes), Some(t)) = (&offer, said, me) {
            if yes {
                picks.push((show.clone(), t, "volunteered".into()));
            } else {
                let f = team_fill(w, t, Fill::new()).set("show", show.clone());
                if let Some(text) = w.stories.line(seed, "doc.declined", &f, year) {
                    let e = news(w, t.level(), Loudness::Medium, Some(t), text);
                    post(w, e);
                }
            }
        }
        let taken_nfl = picks.iter().any(|p| matches!(p.1, TeamKey::Nfl(_)));
        if year >= 2001
            && !taken_nfl
            && let Some((t, how)) = hard_knocks(w, year)
        {
            // A club that said no is only on the show if the league orders it.
            let declined = said == Some(false) && me == Some(TeamKey::Nfl(t as u16));
            if !declined || how == "forced" {
                picks.push(("Hard Knocks".into(), TeamKey::Nfl(t as u16), how));
            }
        }
        for o in other_docs(w, year) {
            let declined = said == Some(false) && me == Some(o.1);
            if !declined && !picks.iter().any(|p| p.1 == o.1) {
                picks.push(o);
            }
        }
        w.stories.media.doc_offer = None;
        w.stories.media.doc_answer = None;
        for (show, t, how) in picks {
            let f = team_fill(w, t, Fill::new()).set("show", show.clone());
            let k = if how == "forced" { "doc.forced" } else { "doc.announce" };
            if let Some(text) = w.stories.line(seed, k, &f, year) {
                let mine = super::seat_team(w) == Some(t);
                let e = news(w, t.level(), if mine { Loudness::Big } else { Loudness::Small }, Some(t), text);
                post(w, e);
            }
            w.stories.media.docs.push(Doc { year: year as i16, show, team: t, how, episodes: 0, stars: Vec::new() });
        }
    }
    // Episodes: Tuesdays from late July to the end of August.
    if d.weekday() != Weekday::Tue || !(d.month() == 8 || d.month() == 7 && d.day() >= 28) {
        return;
    }
    let ks: Vec<usize> =
        (0..w.stories.media.docs.len()).filter(|&k| w.stories.media.docs[k].year as i32 == year).collect();
    for k in ks {
        let doc = w.stories.media.docs[k].clone();
        if doc.episodes >= 5 {
            continue;
        }
        let n = doc.episodes + 1;
        w.stories.media.docs[k].episodes = n;
        let roster = super::numbers::roster(w, doc.team);
        if roster.is_empty() {
            continue;
        }
        // The episode's star: the camera loves a big personality.
        let star = roster
            .iter()
            .map(|&(i, id)| {
                let t = w.people[i].player.personality.traits;
                ((t.ego as f64 + 0.5 * t.leadership as f64) * (0.6 + key(id as u64, (year as u64) << 4 ^ n as u64)), i)
            })
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|x| x.1)
            .unwrap();
        let star_id = w.people[star].player.id;
        let f = team_fill(w, doc.team, Fill::new())
            .set("show", doc.show.clone())
            .set("n", n.to_string())
            .set("name", w.people[star].name.clone());
        let mine = super::seat_team(w) == Some(doc.team);
        let mut parts: Vec<String> = Vec::new();
        if let Some(t) = w.stories.line(seed, "doc.episode", &f, year) {
            parts.push(t);
        }
        if !w.stories.media.docs[k].stars.contains(&star_id) {
            w.stories.media.docs[k].stars.push(star_id);
            if let Some(t) = w.stories.line(seed, "doc.breakout", &f, year) {
                parts.push(t);
            }
            // Exposure: the star's mood lifts.
            w.people[star].mood = (w.people[star].mood + 0.08).min(1.0);
        }
        // A big ego plays to the camera, and the room notices.
        let ego = w.people[star].player.personality.traits.ego;
        if ego >= 16 && w.stories.rng(seed).chance(0.35) {
            if let Some(t) = w.stories.line(seed, "doc.distraction", &f, year) {
                parts.push(t);
            }
            for &(i, _) in roster.iter().take(60) {
                if i != star {
                    w.people[i].mood = (w.people[i].mood - 0.01).max(-1.0);
                }
            }
        }
        if n == 5
            && let Some(t) = w.stories.line(seed, "doc.cut", &f, year)
        {
            parts.push(t);
        }
        if mine {
            // Your words on TV: the tone follows how you coach.
            let tone = your_tone(w, n);
            if let Some(t) = w.stories.line(seed, &format!("doc.quote.{tone}"), &f, year) {
                parts.push(t);
            }
        }
        if parts.is_empty() {
            continue;
        }
        let e = news(
            w,
            doc.team.level(),
            if mine { Loudness::Medium } else { Loudness::Small },
            Some(doc.team),
            parts.join(" "),
        );
        post(w, crate::calendar::Event { kind: Kind::News, ..e });
        if mine && n == 5 {
            super::story::memento(w, super::story::MementoKind::Photo, format!("{} season, {year}", doc.show));
        }
    }
}

fn team_code(t: TeamKey) -> u64 {
    match t {
        TeamKey::Hs(h) => h as u64,
        TeamKey::College(p) => 1 << 20 | p as u64,
        TeamKey::Nfl(n) => 2 << 20 | n as u64,
        TeamKey::Other(f) => 3 << 20 | f as u64,
    }
}

/// The ask: Hard Knocks for an NFL club that would volunteer (a club the
/// league would order on gets no ask), the college streaming series for a
/// program it picked.
fn offer_doc(w: &mut World, year: i32) {
    let Some(me) = super::seat_team(w) else { return };
    let show = match me {
        TeamKey::Nfl(t) => {
            let shown = w
                .stories
                .media
                .docs
                .iter()
                .any(|d| d.show == "Hard Knocks" && d.team == me && d.year as i32 >= year - 10);
            let pull = key(t as u64, year as u64 ^ 0xA5C) * (1.3 - market_rank(w, t as usize) as f64 / 40.0);
            if shown || pull < 0.55 {
                return;
            }
            "Hard Knocks".to_string()
        }
        TeamKey::College(p) => {
            let picked = other_docs(w, year).into_iter().find(|o| o.1 == me);
            let Some((show, _, _)) = picked else {
                let _ = p;
                return;
            };
            show
        }
        TeamKey::Hs(_) | TeamKey::Other(_) => return,
    };
    let name = super::team_name(w, me);
    let who = if matches!(me, TeamKey::Nfl(_)) { "NFL Films" } else { "The streaming service" };
    let boss = if matches!(me, TeamKey::Nfl(_)) { "the owner" } else { "the athletic director" };
    let text = format!(
        "{who} wants {name} for {show} this summer: cameras in meetings, the locker room and your office from camp on. The answer is due by May 31; with none, {boss} decides."
    );
    let e = news(w, me.level(), Loudness::Big, Some(me), text).decision().rule(super::ASK_RULE);
    post(w, e);
    w.stories.media.doc_offer = Some((year as i16, show));
    w.stories.media.doc_answer = None;
}

/// Your answer to a documentary's ask.
pub fn answer_doc(w: &mut World, yes: bool) -> Result<String, String> {
    let Some((year, show)) = w.stories.media.doc_offer.clone() else { return Err("No series is asking.".into()) };
    if year as i32 != w.year {
        return Err("That ask has passed.".into());
    }
    w.stories.media.doc_answer = Some(yes);
    Ok(if yes {
        format!("You said yes to {show}. The crew arrives with camp; the first episode airs in August.")
    } else {
        format!("You said no to {show}. The series will look elsewhere.")
    })
}

/// The tone of your words on camera: fiery for a demanding motivator,
/// calm for a teacher, funny for a players' coach, emotional late in camp.
fn your_tone(w: &World, n: u8) -> &'static str {
    let Some(c) = w.career.as_ref() else { return "calm" };
    let y = &c.you;
    let u = key(c.seed as u64, (w.year as u64) << 4 ^ n as u64);
    if n == 5 && u < 0.5 {
        return "emotional";
    }
    let fiery = y.discipline + y.motivation * 0.5;
    let warm = y.relationships;
    if u < 0.2 {
        "funny"
    } else if fiery > warm + 4.0 {
        "fiery"
    } else if warm > 13.0 && u < 0.5 {
        "funny"
    } else {
        "calm"
    }
}

// --- C10: the video game's ratings -----------------------------------------------------------------

/// The video game's rating for a player in a season: the public's read of
/// him (perception, not truth), spread like the real game's ratings: the
/// best at a position in the 90s, starters in the 70s and 80s, backups in
/// the 60s. Computed when asked, never stored.
pub fn game_rating(w: &World, i: usize) -> Option<u8> {
    let p = w.people.get(i)?;
    w.stories.media.ratings.get(&p.player.id).copied()
}

/// Every rated player's number for the season: each pool (pros on a
/// roster, college players) ranked by position.
fn rate_all(w: &World) -> HashMap<u32, u8> {
    let fame = fame(w);
    let mut pools: HashMap<(bool, Position), Vec<(f64, u32)>> = HashMap::new();
    for (i, p) in w.people.iter().enumerate() {
        let pro = p.stage == crate::world::Stage::Pro && p.pro.as_ref().is_some_and(|c| c.team.is_some());
        let college = p.stage == crate::world::Stage::College;
        if !pro && !college {
            continue;
        }
        pools.entry((pro, p.player.position)).or_default().push((public_score(w, &fame, i), p.player.id));
    }
    let mut out = HashMap::new();
    for ((pro, _), mut v) in pools {
        v.sort_by(|a, b| b.0.total_cmp(&a.0));
        let teams = if pro { 32.0 } else { 134.0 };
        for (better, &(_, id)) in v.iter().enumerate() {
            let r = better as f64 / teams;
            let rating = 99.0 - 22.0 * r.powf(0.6) - if pro { 0.0 } else { 4.0 };
            out.insert(id, rating.clamp(40.0, 99.0).round() as u8);
        }
    }
    out
}

/// August: the ratings release. The top of the game, your team's best and
/// lowest, and players' reactions by personality.
fn ratings_release(w: &mut World, d: Date) {
    let year = w.year;
    if !(d.month() == 8 && d.day() == 12) || !w.stories.once(30, year) {
        return;
    }
    let seed = w.options.seed;
    let fame = fame(w);
    w.stories.media.ratings = rate_all(w);
    for (title, nfl, names) in video_games(year) {
        // Before 2024 the college game had no names: no ratings stories about players.
        if !nfl && !names {
            continue;
        }
        if !nfl
            && year == 2024
            && let Some(t) = super::seat_team(w).filter(|t| matches!(t, TeamKey::College(_)))
        {
            let f = team_fill(w, t, Fill::new()).set("game", title.clone());
            if let Some(text) = w.stories.line(seed, "vg.college_back", &f, year) {
                let e = news(w, Level::College, Loudness::Medium, Some(t), text);
                post(w, e);
            }
        }
        // The top of the game: the 99 club (the best few by public standing).
        let stage = if nfl { crate::world::Stage::Pro } else { crate::world::Stage::College };
        let mut best: Vec<(f64, usize)> = (0..w.people.len())
            .filter(|&i| {
                w.people[i].stage == stage && (!nfl || w.people[i].pro.as_ref().is_some_and(|c| c.team.is_some()))
            })
            .filter(|&i| !matches!(w.people[i].player.position, Position::K | Position::P | Position::LS))
            .map(|i| (public_score(w, &fame, i), i))
            .collect();
        best.sort_by(|a, b| b.0.total_cmp(&a.0));
        let club: Vec<usize> = best.iter().take(if nfl { 4 } else { 2 }).map(|x| x.1).collect();
        for &i in &club {
            let id = w.people[i].player.id;
            w.stories.media.vg_top.push((year as i16, title.clone(), id, 99));
            let f = Fill::new().set("name", w.people[i].name.clone()).set("game", title.clone());
            if year >= 2016
                && let Some(text) = w.stories.line(seed, "vg.club99", &f, year)
            {
                let t = super::numbers::team_of(w, i);
                let e = news(w, if nfl { Level::Nfl } else { Level::College }, Loudness::Small, t, text);
                post(w, e);
            }
        }
        // Your team: its highest rated, and the man who'll be angriest.
        let Some(me) = super::seat_team(w).filter(|t| t.level() == if nfl { Level::Nfl } else { Level::College })
        else {
            continue;
        };
        let roster = super::numbers::roster(w, me);
        let mut rated: Vec<(u8, usize)> =
            roster.iter().filter_map(|&(i, _)| game_rating(w, i).map(|r| (r, i))).collect();
        rated.sort_by_key(|b| std::cmp::Reverse(b.0));
        if let Some(&(r, i)) = rated.first() {
            let f = team_fill(w, me, Fill::new())
                .set("game", title.clone())
                .set("name", w.people[i].name.clone())
                .set("rating", r.to_string());
            if let Some(text) = w.stories.line(seed, "vg.release", &f, year) {
                let e = news(w, me.level(), Loudness::Medium, Some(me), text);
                post(w, e);
            }
        }
        // Reactions: a big ego rated well below what he thinks of himself.
        let mut reacts = 0;
        for &(r, i) in &rated {
            if reacts >= 3 {
                break;
            }
            let t = w.people[i].player.personality.traits;
            let self_view = w.people[i].player.overall() + 3.0 * (t.ego as f64 - 10.0) / 4.0;
            let gap = self_view - r as f64;
            let u = w.stories.rng(seed).next();
            let kind = if gap > 8.0 && t.ego >= 14 {
                if t.temper >= 13 { "angry" } else { "motivated" }
            } else if gap > 6.0 && u < 0.3 {
                "funny"
            } else if r >= 88 && u < 0.25 {
                "grateful"
            } else {
                continue;
            };
            let f = Fill::new().set("name", w.people[i].name.clone()).set("rating", r.to_string());
            if let Some(text) = w.stories.line(seed, &format!("vg.react.{kind}"), &f, year) {
                let e = news(w, me.level(), Loudness::Small, Some(me), text);
                post(w, e);
                reacts += 1;
                if kind == "angry" {
                    w.people[i].mood = (w.people[i].mood - 0.03).max(-1.0);
                }
            }
        }
    }
    let keep = year - 10;
    w.stories.media.vg_top.retain(|x| x.0 as i32 >= keep);
}

// --- C11: fantasy football ---------------------------------------------------------------------------

/// Points for a game line: standard scoring before 2019, PPR after (ESPN
/// made PPR its default in 2019; Yahoo went half-point in 2017).
pub fn fantasy_points(l: &crate::game::stats::StatLine, year: i32) -> f64 {
    let ppr = if year >= 2019 {
        1.0
    } else if year >= 2017 {
        0.5
    } else {
        0.0
    };
    let pass = l.pass_yds / 25.0 + 4.0 * l.pass_td as f64 - 2.0 * l.int as f64;
    let rush = (l.rush_yds + l.scramble_yds) / 10.0 + 6.0 * l.rush_td as f64;
    let rec = l.rec_yds / 10.0 + 6.0 * l.rec_td as f64 + ppr * l.rec as f64;
    let kick: f64 = l.fg_made.iter().enumerate().map(|(b, &n)| n as f64 * [3.0, 3.0, 4.0, 5.0][b.min(3)]).sum::<f64>()
        + l.xp_made as f64;
    pass + rush + rec + kick - 2.0 * l.fumbles_lost as f64
}

pub(crate) fn after_nfl_game(w: &mut World, ctx: &GameCtx, s: &crate::game::GameStats) {
    let year = w.year;
    if w.stories.media.fantasy_year != year {
        // The past season's leaders go into the book.
        let last = w.stories.media.fantasy_year;
        if last > 0 {
            let mut by: [Vec<(f32, u32)>; 4] = Default::default();
            for (&id, f) in &w.stories.media.fantasy {
                if (f.pos as usize) < 4 {
                    by[f.pos as usize].push((f.pts, id));
                }
            }
            for (k, v) in by.iter_mut().enumerate() {
                v.sort_by(|a, b| b.0.total_cmp(&a.0));
                if let Some(&(p, id)) = v.first() {
                    w.stories.media.fantasy_leaders.push((last as i16, k as u8, id, p));
                }
            }
        }
        w.stories.media.fantasy.clear();
        w.stories.media.fantasy_year = year;
    }
    if ctx.postseason {
        return;
    }
    for side in 0..2 {
        for (&id, l) in &s.players[side] {
            let pts = fantasy_points(l, year) as f32;
            if pts == 0.0 && l.att + l.rush + l.targets == 0 {
                continue;
            }
            let e = w.stories.media.fantasy.entry(id).or_default();
            e.pts += pts;
            e.games = e.games.saturating_add(1);
            e.last = pts;
        }
    }
}

fn fantasy_pos(pos: Position) -> Option<u8> {
    match pos {
        Position::QB => Some(0),
        Position::RB => Some(1),
        Position::WR => Some(2),
        Position::TE => Some(3),
        _ => None,
    }
}

/// Tuesday in the NFL season: the waiver wire's must-adds and the week's
/// top scorer.
fn fantasy_tuesday(w: &mut World, d: Date) {
    if d.weekday() != Weekday::Tue || w.stories.media.fantasy.is_empty() || w.year < 1997 {
        return;
    }
    let y = &w.dates;
    if d < y.nfl_kickoff.add(4) || d > y.nfl_weeks.last().copied().unwrap_or(d).add(3) {
        return;
    }
    let year = w.year;
    let seed = w.options.seed;
    // Position and rank of each fantasy player this season.
    let mut lines: Vec<(u32, Fantasy, usize, u8)> = Vec::new();
    for (&id, f) in &w.stories.media.fantasy {
        let Some(i) = super::person_index(w, id) else { continue };
        let Some(pos) = fantasy_pos(w.people[i].player.position) else { continue };
        lines.push((id, *f, i, pos));
    }
    for l in lines.iter_mut() {
        if let Some(e) = w.stories.media.fantasy.get_mut(&l.0) {
            e.pos = l.3;
        }
    }
    lines.sort_by(|a, b| b.1.pts.total_cmp(&a.1.pts));
    let rank_of = |id: u32, pos: u8, lines: &[(u32, Fantasy, usize, u8)]| {
        lines.iter().filter(|x| x.3 == pos).position(|x| x.0 == id).unwrap_or(99)
    };
    // The week's top scorer.
    if let Some(top) = lines.iter().max_by(|a, b| a.1.last.total_cmp(&b.1.last)) {
        let f = Fill::new()
            .set("name", w.people[top.2].name.clone())
            .set("pts", format!("{:.1}", top.1.last))
            .set("pos", ["QB", "RB", "WR", "TE"][top.3 as usize]);
        if let Some(text) = w.stories.line(seed, "fantasy.top", &f, year) {
            let t = super::numbers::team_of(w, top.2);
            let e = news(w, Level::Nfl, Loudness::Small, t, text);
            post(w, e);
        }
    }
    // The pickup: a big week from a man few rostered.
    let pickup = lines
        .iter()
        .filter(|x| x.1.last >= 18.0 && rank_of(x.0, x.3, &lines) >= 24)
        .max_by(|a, b| a.1.last.total_cmp(&b.1.last));
    if let Some(p) = pickup {
        let rank = rank_of(p.0, p.3, &lines);
        let pct = (60.0 * (-(rank as f64) / 30.0).exp()).clamp(2.0, 45.0).round();
        let t = super::numbers::team_of(w, p.2);
        let f = Fill::new()
            .set("name", w.people[p.2].name.clone())
            .set("team", t.map(|t| team_name(w, t)).unwrap_or_default())
            .set("pts", format!("{:.1}", p.1.last))
            .set("pct", format!("{pct:.0}"));
        if let Some(text) = w.stories.line(seed, "fantasy.pickup", &f, year) {
            let e = news(w, Level::Nfl, Loudness::Small, t, text);
            post(w, e);
        }
    }
    // A first-round pick who keeps failing his managers.
    let bust = lines.iter().find(|x| x.1.games >= 3 && rank_of(x.0, x.3, &lines) < 3 && x.1.last < 6.0);
    if let Some(b) = bust
        && w.stories.rng(seed).chance(0.3)
    {
        let t = super::numbers::team_of(w, b.2);
        let f = Fill::new()
            .set("name", w.people[b.2].name.clone())
            .set("team", t.map(|t| team_name(w, t)).unwrap_or_default())
            .set("pts", format!("{:.1}", b.1.last));
        if let Some(text) = w.stories.line(seed, "fantasy.bust", &f, year) {
            let e = news(w, Level::Nfl, Loudness::Small, t, text);
            post(w, e);
        }
    }
}

/// A player's fantasy season so far (points, games) and his rank at his position.
pub fn fantasy_line(w: &World, id: u32) -> Option<(f32, u8, usize)> {
    let f = w.stories.media.fantasy.get(&id)?;
    let rank = w.stories.media.fantasy.values().filter(|x| x.pos == f.pos && x.pts > f.pts).count() + 1;
    Some((f.pts, f.games, rank))
}

// --- C12: jersey sales -------------------------------------------------------------------------------

/// The NFLPA's yearly list (spring, the year from March to February) and,
/// from 2021, the college name-and-likeness jerseys (August).
fn jersey_lists(w: &mut World, d: Date) {
    let year = w.year;
    let seed = w.options.seed;
    let nfl_day = d.month() == 4 && d.day() == 1;
    let college_day = d.month() == 8 && d.day() == 20 && year >= 2021;
    if !(nfl_day || college_day) || !w.stories.once(if nfl_day { 40 } else { 41 }, year) {
        return;
    }
    let stage = if nfl_day { crate::world::Stage::Pro } else { crate::world::Stage::College };
    let fame = fame(w);
    let mut list: Vec<(f64, usize)> = (0..w.people.len())
        .filter(|&i| {
            w.people[i].stage == stage && (!nfl_day || w.people[i].pro.as_ref().is_some_and(|c| c.team.is_some()))
        })
        .filter(|&i| {
            !matches!(
                w.people[i].player.position,
                Position::OT | Position::IOL | Position::K | Position::P | Position::LS
            )
        })
        .map(|i| {
            let p = &w.people[i];
            let mut s = public_score(w, &fame, i);
            if let Some(TeamKey::Nfl(t)) = super::numbers::team_of(w, i) {
                s += 6.0 * (1.0 - market_rank(w, t as usize) as f64 / 32.0);
            }
            if let Some(TeamKey::College(c)) = super::numbers::team_of(w, i) {
                s += 8.0 * w.programs[c as usize].fans.min(1.5);
            }
            // A nickname that sold shirts.
            if super::nick::merch(&w.stories, p.player.id) {
                s += 5.0;
            }
            (s, i)
        })
        .collect();
    list.sort_by(|a, b| b.0.total_cmp(&a.0));
    let ids: Vec<u32> = list.iter().take(25).map(|x| w.people[x.1].player.id).collect();
    w.stories.media.jerseys.push((year as i16, !nfl_day, ids.clone()));
    w.stories.media.jerseys.retain(|x| x.0 as i32 >= year - 10);
    // The headline (the top seller) and your players on the list.
    let mine = super::seat_team(w);
    for (rank, &id) in ids.iter().enumerate() {
        let Some(i) = super::person_index(w, id) else { continue };
        let t = super::numbers::team_of(w, i);
        let yours = t.is_some() && t == mine;
        if rank > 0 && !yours {
            continue;
        }
        let rookie = w.people[i].pro.as_ref().is_some_and(|c| c.first_year >= year - 1);
        let k = if !nfl_day {
            "jersey.college"
        } else if rookie {
            "jersey.rookie"
        } else {
            "jersey.list"
        };
        let f = Fill::new()
            .set("name", w.people[i].name.clone())
            .set("team", t.map(|t| team_name(w, t)).unwrap_or_default())
            .set("rank", (rank + 1).to_string());
        if let Some(text) = w.stories.line(seed, k, &f, year) {
            let lvl = if nfl_day { Level::Nfl } else { Level::College };
            let e = news(w, lvl, if yours { Loudness::Medium } else { Loudness::Small }, t, text);
            post(w, e);
        }
    }
}

pub(crate) fn day(w: &mut World, d: Date) {
    release_magazines(w, d);
    cover_jinx(w, d);
    documentaries(w, d);
    ratings_release(w, d);
    fantasy_tuesday(w, d);
    jersey_lists(w, d);
}
