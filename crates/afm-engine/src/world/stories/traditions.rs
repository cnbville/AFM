//! The traditions (C1-C5, C13, C14): high school rivalry trophies, field
//! storming with its conference fines, the travelling pregame show,
//! uniforms chosen each week, retired numbers, rings of honor and statues
//! with their ceremonies, title parades, rings and bowl gifts.
//!
//! College rivalry trophies are the real ones and live with the rivalry
//! records (`college.rs`, A28); a high school's are generated here.

use crate::calendar::{Date, Event, Kind, Level, Loudness, Weekday};
use crate::positions::Position;
use crate::world::World;
use crate::world::data::Division;
use crate::world::schedule::GameKind;

use super::lines::Fill;
use super::{GameCtx, TeamKey, news, post, team_fill, team_name};

/// A high school rivalry and its trophy (generated: Claude's pick, a name
/// from the bank and a past drawn from the two schools' standing).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HsRivalry {
    pub schools: [u32; 2],
    pub trophy: String,
    pub since: i16,
    /// Wins by each school, and ties.
    pub wins: [u16; 2],
    pub holder: Option<u32>,
    pub last: Option<i16>,
}

/// An honor a team gives a former player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HonorKind {
    /// His number is out of circulation.
    RetiredNumber,
    /// His jersey hangs in the stadium; the number stays in use.
    RetiredJersey,
    RingOfHonor,
    Statue,
}

impl HonorKind {
    pub fn name(self) -> &'static str {
        match self {
            HonorKind::RetiredNumber => "Retired number",
            HonorKind::RetiredJersey => "Honored jersey",
            HonorKind::RingOfHonor => "Ring of honor",
            HonorKind::Statue => "Statue",
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TeamHonor {
    pub team: TeamKey,
    pub kind: HonorKind,
    pub player: u32,
    pub name: String,
    pub pos: Position,
    pub number: Option<u8>,
    /// The season he is honored in (the ceremony is at a home game).
    pub year: i16,
    /// His years with the team, as text ("2027-2030").
    pub years: String,
    pub ceremony_done: bool,
}

/// One field storming.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Storming {
    pub date: Date,
    pub team: u32,
    pub opp: u32,
    pub score: [u32; 2],
    /// The conference fine in dollars (0: none that year).
    pub fine: u32,
}

/// The travelling pregame show's season (C3).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ShowSeason {
    pub year: i32,
    /// Saturday, host, visitor.
    pub stops: Vec<(Date, u32, u32)>,
    /// Visits by host program over the world's history: (program, visits, last season).
    pub visits: Vec<(u32, u16, i16)>,
}

/// Uniform sets (C4): what a team owns this season and what you wore.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Uniforms {
    /// Your pick for your next game (index into `sets`), if you made one.
    pub next: Option<u8>,
    /// Your games in each set: (date, set name, won).
    pub worn: Vec<(Date, String, bool)>,
    /// Summer reveals by team and season, with the set's description.
    pub reveals: Vec<(TeamKey, i16, String)>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Traditions {
    pub hs_rivals: Vec<HsRivalry>,
    pub honors: Vec<TeamHonor>,
    pub show: ShowSeason,
    /// The last fifteen seasons' stormings.
    pub stormings: Vec<Storming>,
    pub uniforms: Uniforms,
    /// Parades held: (season, team, crowd).
    pub parades: Vec<(i16, TeamKey, u32)>,
}

/// The show's own name (C3: Claude's pick; real life's is College GameDay,
/// on the road since 1993).
pub const SHOW: &str = "Campus Kickoff";
pub const SHOW_FROM: i32 = 1993;

impl Traditions {
    pub fn retired_numbers(&self, t: TeamKey) -> Vec<u8> {
        self.honors
            .iter()
            .filter(|h| h.team == t && h.kind == HonorKind::RetiredNumber)
            .filter_map(|h| h.number)
            .collect()
    }

    pub fn show_host(&self, d: Date) -> Option<usize> {
        self.show.stops.iter().find(|s| s.0 == d).map(|s| s.1 as usize)
    }
}

/// A fixed draw from ids, for picks that must not touch any stream.
pub(crate) fn key(a: u64, b: u64) -> f64 {
    let mut h = a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    (h >> 11) as f64 / (1u64 << 53) as f64
}

// --- C1: high school rivalry trophies ------------------------------------------------------------

/// High school trophy names (generated; Claude's pick).
const HS_TROPHIES: &[&str] = &[
    "the Old Brass Bell",
    "the Paint Bucket",
    "the Little Brown Jug",
    "the Victory Lantern",
    "the County Cup",
    "the Railroad Spike",
    "the Iron Skillet",
    "the Golden Helmet",
    "the Milk Can",
    "the Rusty Shovel",
    "the Silver Boot",
    "the Old Wagon Wheel",
    "the Hatchet",
    "the River Rock",
    "the Battle Axe",
    "the Coal Bucket",
    "the Feed Sack",
    "the Old Oak Paddle",
    "the Fire Hydrant",
    "the Cowbell",
    "the Anvil",
    "the Lantern",
    "the Peace Pipe",
    "the Mule Shoe",
    "the Water Tower Trophy",
    "the Mailbox",
    "the Old Gray Boot",
    "the Golden Spike",
    "the Ship's Bell",
    "the Cornerstone Trophy",
];

/// The rival of high school `h`: a school in its own town or area with the
/// closest enrollment, else the nearest-sized school in its state and class.
pub fn hs_rival_of(w: &World, h: usize) -> Option<usize> {
    let s = w.high_schools.get(h)?;
    let same_area = w
        .high_schools
        .iter()
        .filter(|o| o.id != h && o.area == s.area && o.state == s.state)
        .min_by_key(|o| (o.enrollment as i64 - s.enrollment as i64).abs());
    same_area.map(|o| o.id).or_else(|| {
        w.high_schools
            .iter()
            .filter(|o| o.id != h && o.state == s.state && o.classification == s.classification)
            .min_by_key(|o| (o.enrollment as i64 - s.enrollment as i64).abs() + (o.id as i64 - h as i64).abs())
            .map(|o| o.id)
    })
}

pub(crate) fn hs_rivals(w: &World, a: usize, b: usize) -> bool {
    let (x, y) = (a.min(b) as u32, a.max(b) as u32);
    w.stories.traditions.hs_rivals.iter().any(|r| r.schools == [x, y])
        || hs_rival_of(w, a) == Some(b)
        || hs_rival_of(w, b) == Some(a)
}

/// The trophy two rival schools play for, if they are rivals.
pub(crate) fn hs_trophy(w: &World, a: usize, b: usize) -> Option<String> {
    if !hs_rivals(w, a, b) {
        return None;
    }
    let (x, y) = (a.min(b) as u32, a.max(b) as u32);
    if let Some(r) = w.stories.traditions.hs_rivals.iter().find(|r| r.schools == [x, y]) {
        return Some(r.trophy.clone());
    }
    Some(HS_TROPHIES[(key(y as u64, x as u64 ^ 0x77) * HS_TROPHIES.len() as f64) as usize].to_string())
}

/// The rivalry between two schools, made the first time it is played on
/// record (its past drawn from the two schools' standing).
fn hs_rivalry(w: &mut World, a: usize, b: usize) -> usize {
    let (x, y) = (a.min(b) as u32, a.max(b) as u32);
    if let Some(k) = w.stories.traditions.hs_rivals.iter().position(|r| r.schools == [x, y]) {
        return k;
    }
    let u = key(x as u64, y as u64);
    let since = 1925 + (u * 50.0) as i16;
    let meetings = (w.year as i16 - since).max(10) as f64 * 0.9;
    let px = w.high_schools[x as usize].prestige;
    let py = w.high_schools[y as usize].prestige;
    let share = (0.5 + 0.25 * (px - py)).clamp(0.2, 0.8);
    let wx = (meetings * share).round() as u16;
    let wy = (meetings - wx as f64).max(0.0) as u16;
    let trophy = HS_TROPHIES[(key(y as u64, x as u64 ^ 0x77) * HS_TROPHIES.len() as f64) as usize].to_string();
    let holder = Some(if key(x as u64 ^ 3, y as u64) < share { x } else { y });
    w.stories.traditions.hs_rivals.push(HsRivalry {
        schools: [x, y],
        trophy,
        since,
        wins: [wx, wy],
        holder,
        last: None,
    });
    w.stories.traditions.hs_rivals.len() - 1
}

/// After one of your school's games: the rivalry trophy, and fans on the
/// field after an upset of a much better team.
pub(crate) fn after_hs_game(w: &mut World, ctx: &GameCtx, stats: &crate::game::GameStats) {
    let (TeamKey::Hs(h), TeamKey::Hs(a)) = (ctx.teams[0], ctx.teams[1]) else { return };
    let (h, a) = (h as usize, a as usize);
    let [sh, sa] = stats.score;
    let home_won = sh > sa;
    let (wi, li) = if home_won { (h, a) } else { (a, h) };
    let year = w.year;
    let seed = w.options.seed;
    if ctx.rivalry {
        let k = hs_rivalry(w, h, a);
        let r = &mut w.stories.traditions.hs_rivals[k];
        let before = r.holder;
        let side = (r.schools[1] as usize == wi) as usize;
        r.wins[side] = r.wins[side].saturating_add(1);
        r.holder = Some(wi as u32);
        r.last = Some(year as i16);
        let (trophy, wins) = (r.trophy.clone(), r.wins);
        let (lead, n0, n1) =
            if wins[0] >= wins[1] { (r.schools[0], wins[0], wins[1]) } else { (r.schools[1], wins[1], wins[0]) };
        let how = match before {
            Some(b) if b as usize == wi => "keeps",
            _ => "takes back",
        };
        let text = format!(
            "{} beats {} {}-{} and {how} {trophy}. {} leads the series {n0}-{n1}.",
            w.school_name(wi),
            w.school_name(li),
            sh.max(sa),
            sh.min(sa),
            w.school_name(lead as usize)
        );
        let e = Event::new(w.today, Level::HighSchool, Kind::Game, Loudness::Medium, text);
        post(w, e);
        if super::seat_team(w) == Some(TeamKey::Hs(wi as u32)) {
            let t = format!("{trophy}, won from {} in {year}", w.school_name(li));
            super::story::memento(w, super::story::MementoKind::Trophy, t);
        }
    }
    // Home fans on the field after beating a clearly better team (the
    // schools' standing stands in for the line).
    if home_won {
        let gap = w.high_schools[a].prestige - w.high_schools[h].prestige;
        let p = ((gap - 0.25) * 0.8).clamp(0.0, 0.35) + if ctx.rivalry { 0.08 } else { 0.0 };
        if p > 0.0 && w.stories.rng(seed).chance(p) {
            let f = team_fill(w, TeamKey::Hs(h as u32), Fill::new())
                .set("opp", w.school_name(a).to_string())
                .set("score", format!("{sh}-{sa}"));
            if let Some(text) = w.stories.line(seed, "storm.hs", &f, year) {
                let e = Event::new(w.today, Level::HighSchool, Kind::News, Loudness::Small, text);
                post(w, e);
            }
        }
    }
}

// --- C2: field storming and its fines --------------------------------------------------------------

/// A conference's fine policy in a season (from `data/real-world/storming_fines.csv`):
/// first, second and later offenses. None when the conference had none.
fn fine_policy(conf: &str, year: i32) -> Option<[u32; 3]> {
    super::media::storming_fines(conf, year)
}

/// The rank a program holds in the latest AP (or committee) poll.
pub fn poll_rank(w: &World, p: usize) -> Option<usize> {
    let cs = w.college.as_ref()?;
    let poll = cs.polls.iter().rev().find(|x| {
        matches!(
            x.kind,
            crate::world::college::PollKind::Ap
                | crate::world::college::PollKind::Cfp
                | crate::world::college::PollKind::ApPreseason
        )
    })?;
    poll.top.iter().position(|&t| t == p).map(|k| k + 1)
}

/// After a college game: fans storm the field after an upset at home
/// (C2). The chance comes from the size of the upset as the world saw it
/// before kickoff, the opponent's rank, a rivalry, the fan base and the era
/// (stormings rose through the 2000s and 2010s; the base rate, about one a
/// week across the FBS in recent seasons, is Claude's reading of the
/// published counts, proposed, not confirmed).
pub(crate) fn after_college_game(w: &mut World, k: usize, pre: f64) {
    let Some(cs) = w.college.as_ref() else { return };
    let g = &cs.games[k];
    let Some(p) = g.played.filter(|p| !p.cancelled) else { return };
    if !p.home_won || g.site.is_some() || !matches!(g.division, Division::FBS | Division::FCS) {
        return;
    }
    if !matches!(g.kind, GameKind::Regular | GameKind::ConferenceTitle) {
        return;
    }
    let (h, a) = (g.home, g.away);
    let rivalry = g.rivalry.is_some();
    let date = g.date;
    let score = p.score;
    let opp_rank = poll_rank(w, a);
    let own_rank = poll_rank(w, h);
    let under = (0.42 - pre).max(0.0) / 0.42;
    let ranked = match opp_rank {
        Some(r) if r <= 5 => 1.0,
        Some(r) if r <= 10 => 0.75,
        Some(_) => 0.45,
        None => 0.12,
    };
    // A ranked team storming the field is mocked and rare.
    let own = if own_rank.is_some_and(|r| r <= 15) { 0.15 } else { 1.0 };
    let era = if w.year < 2003 {
        0.55
    } else if w.year < 2012 {
        0.8
    } else {
        1.0
    };
    let fans = (w.programs[h].fans / 1.0).clamp(0.4, 1.4);
    let chance =
        (under.powf(1.3) * ranked * own * era * fans * if rivalry { 1.35 } else { 1.0 } * 0.9).clamp(0.0, 0.92);
    let seed = w.options.seed;
    if chance <= 0.0 || !w.stories.rng(seed).chance(chance) {
        return;
    }
    let year = w.year;
    let conf = w.programs[h].conference.clone();
    let policy = fine_policy(&conf, year);
    let since = super::media::storming_policy_from(&conf, year);
    let offenses = w
        .stories
        .traditions
        .stormings
        .iter()
        .filter(|s| s.team == h as u32 && s.fine > 0 && since.is_some_and(|f| s.date.year() >= f))
        .count();
    let fine = policy.map_or(0, |f| f[offenses.min(2)]);
    w.stories.traditions.stormings.push(Storming { date, team: h as u32, opp: a as u32, score, fine });
    w.stories.traditions.stormings.retain(|s| s.date.year() >= year - 15);
    let stadium = w.programs[h].stadium.max(15_000);
    let crowd = ((stadium as f64 * (0.18 + 0.12 * key(h as u64, date.0 as u64))) / 1000.0).round() as u32 * 1000;
    let rank = opp_rank.map_or(String::new(), |r| format!("No. {r}"));
    let f = team_fill(w, TeamKey::College(h as u32), Fill::new())
        .set("opp", w.program_name(a).to_string())
        .set("rank", rank)
        .set("score", format!("{}-{}", score[0], score[1]))
        .set("crowd", format!("about {}", thousands(crowd)));
    let loud = if opp_rank.is_some_and(|r| r <= 10) { Loudness::Medium } else { Loudness::Small };
    if let Some(text) = w.stories.line(seed, "storm.news", &f, year) {
        let e = news(w, Level::College, loud, Some(TeamKey::College(h as u32)), text);
        post(w, e);
    }
    let ordinal = ["first", "second", "third"][offenses.min(2)];
    let f2 = f.clone().set("conf", conf.clone()).set("fine", dollars(fine)).set("n", ordinal);
    let key = if fine > 0 { "storm.fine" } else { "storm.no_fine" };
    if (fine > 0 || policy.is_none() && key == "storm.no_fine" && opp_rank.is_some_and(|r| r <= 10))
        && let Some(text) = w.stories.line(seed, key, &f2, year)
    {
        // The fine comes a day or two later.
        let e = news(w, Level::College, Loudness::Small, Some(TeamKey::College(h as u32)), text);
        w.stories.later(date.add(2), e);
    }
    if super::seat_team(w) == Some(TeamKey::College(h as u32)) {
        let t = format!("The field after beating {} {}-{} ({year})", w.program_name(a), score[0], score[1]);
        super::story::memento(w, super::story::MementoKind::Photo, t);
    }
    // A rare scary moment in the crush (2024 had two; proposed, not confirmed: 3%).
    if w.stories.rng(seed).chance(0.03) {
        let f = team_fill(w, TeamKey::College(h as u32), Fill::new());
        if let Some(text) = w.stories.line(seed, "storm.injury", &f, year) {
            let e = news(w, Level::College, Loudness::Small, Some(TeamKey::College(h as u32)), text);
            post(w, e);
        }
    }
}

pub(crate) fn thousands(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (k, c) in s.chars().enumerate() {
        if k > 0 && (s.len() - k) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

pub(crate) fn dollars(n: u32) -> String {
    format!("${}", thousands(n))
}

// --- C3: the travelling pregame show ---------------------------------------------------------------

/// Sunday: the show names next Saturday's stop. It goes where the biggest
/// game is: two ranked teams, a rivalry, a game huge for someone, a
/// program it hasn't visited lately, from 1993 (on the road).
fn show_pick(w: &mut World, d: Date) {
    let Some(cs) = w.college.as_ref() else { return };
    let sat = d.add(6);
    if w.year < SHOW_FROM || !w.dates.cfb_weeks.contains(&sat) {
        return;
    }
    if w.stories.traditions.show.year != w.year {
        let visits = std::mem::take(&mut w.stories.traditions.show.visits);
        w.stories.traditions.show = ShowSeason { year: w.year, stops: Vec::new(), visits };
    }
    let recent: Vec<u32> = w.stories.traditions.show.stops.iter().rev().take(4).map(|s| s.1).collect();
    let rank = |p: usize| poll_rank(w, p);
    let mut best: Option<(f64, usize)> = None;
    for (k, g) in cs.games.iter().enumerate() {
        if g.date != sat || g.division != Division::FBS || g.played.is_some() {
            continue;
        }
        let r = |p: usize| rank(p).map_or(0.0, |x| (26.0 - x as f64) / 25.0);
        let (rh, ra) = (r(g.home), r(g.away));
        let mut score = rh + ra + 1.2 * rh.min(ra);
        if g.rivalry.is_some() {
            score += 0.6;
        }
        if g.huge_for.iter().any(|&x| x) {
            score += 0.4;
        }
        if g.kind == GameKind::ConferenceTitle {
            score += 0.5;
        }
        if g.site.is_some() {
            score -= 0.2;
        }
        if recent.contains(&(g.home as u32)) {
            score -= 0.8;
        }
        // A program it has never visited draws it when the game is close to the best.
        if !w.stories.traditions.show.visits.iter().any(|v| v.0 == g.home as u32) {
            score += 0.15;
        }
        score += 0.1 * key(k as u64, sat.0 as u64);
        if best.is_none_or(|b| score > b.0) {
            best = Some((score, k));
        }
    }
    let Some((_, k)) = best else { return };
    let g = cs.games[k].clone();
    let host = g.home as u32;
    let seed = w.options.seed;
    let year = w.year;
    let past = w.stories.traditions.show.visits.iter().find(|v| v.0 == host).copied();
    w.stories.traditions.show.stops.push((sat, host, g.away as u32));
    let town = w.programs[g.home].city.clone();
    let f = team_fill(w, TeamKey::College(host), Fill::new())
        .set("show", SHOW)
        .set("opp", w.program_name(g.away).to_string())
        .set(
            "game",
            g.rivalry.map(|r| crate::calendar::data::RIVALRIES[r].name_in(year).to_string()).unwrap_or_default(),
        )
        .set("town", town.clone());
    let mut text = w.stories.line(seed, "show.pick", &f, year).unwrap_or_else(|| {
        format!("{SHOW} heads to {town} for {} against {}.", w.program_name(g.home), w.program_name(g.away))
    });
    let extra = match past {
        None => w.stories.line(seed, "show.first_visit", &f, year),
        Some((_, n, last)) => {
            let f = f.clone().set("n", (n + 1).to_string()).set("years", (year - last as i32).to_string());
            w.stories.line(seed, "show.return", &f, year)
        }
    };
    if let Some(x) = extra {
        text = format!("{text} {x}");
    }
    let e = news(w, Level::College, Loudness::Medium, Some(TeamKey::College(host)), text).rule("Pregame show");
    post(w, e);
    match w.stories.traditions.show.visits.iter_mut().find(|v| v.0 == host) {
        Some(v) => {
            v.1 += 1;
            v.2 = year as i16;
        }
        None => w.stories.traditions.show.visits.push((host, 1, year as i16)),
    }
}

/// Saturday morning on the show's set: the guest picker, the signs, and
/// the recruits on campus for the weekend.
fn show_day(w: &mut World, d: Date) {
    let Some(&(_, host, away)) = w.stories.traditions.show.stops.iter().find(|s| s.0 == d) else { return };
    let seed = w.options.seed;
    let year = w.year;
    let recruits = w
        .people
        .iter()
        .filter(|p| {
            p.stage == crate::world::Stage::HighSchool
                && p.cal.visits.iter().any(|v| v.0 == host as usize && v.1 >= d.add(-1) && v.1 <= d)
        })
        .count();
    let f = team_fill(w, TeamKey::College(host), Fill::new())
        .set("show", SHOW)
        .set("opp", w.program_name(away as usize).to_string())
        .set("n", recruits.to_string());
    let mut parts = Vec::new();
    if let Some(t) = w.stories.line(seed, "show.signs", &f, year) {
        parts.push(t);
    }
    if recruits >= 3
        && let Some(t) = w.stories.line(seed, "show.recruits", &f, year)
    {
        parts.push(t);
    }
    if !parts.is_empty() {
        let e = news(w, Level::College, Loudness::Small, Some(TeamKey::College(host)), parts.join(" "));
        post(w, e);
    }
}

// --- C4: uniforms ---------------------------------------------------------------------------------

/// The sets a team owns this season: home and road always; alternates by
/// money and era (Oregon's many-uniform era began in the 2000s, Nike's Pro
/// Combat sets in 2009; NFL alternates and throwbacks, Color Rush from 2016,
/// a second helmet allowed again from 2022).
pub fn sets(w: &World, t: TeamKey) -> Vec<String> {
    let year = w.year;
    let mut v = vec!["Home".to_string(), "Road whites".to_string()];
    let (rich, seed) = match t {
        TeamKey::College(p) => {
            let pr = &w.programs[p as usize];
            ((pr.prestige * 0.6 + pr.facilities * 0.4).clamp(0.0, 1.5), p as u64)
        }
        TeamKey::Nfl(n) => (1.0, 1000 + n as u64),
        TeamKey::Hs(h) => (w.high_schools[h as usize].resources.clamp(0.0, 1.0) * 0.5, 50_000 + h as u64),
    };
    let alts = match t {
        TeamKey::Nfl(_) => {
            let mut n = 1;
            if year >= 2016 {
                n += 1;
            }
            if year >= 2022 {
                n += 1;
            }
            n
        }
        _ => {
            let era = if year < 2000 {
                0.0
            } else if year < 2009 {
                1.0
            } else {
                2.2
            };
            ((rich * era) + key(seed, 7) * 1.2).floor() as usize
        }
    };
    for k in 0..alts.min(5) {
        let label = match (t, k) {
            (TeamKey::Nfl(_), 0) => "Throwbacks".to_string(),
            (TeamKey::Nfl(_), 1) => "Color Rush".to_string(),
            (TeamKey::Nfl(_), _) => "Alternate helmet set".to_string(),
            (_, 0) => "Alternates".to_string(),
            (_, 1) => "Blackout".to_string(),
            (_, 2) => "Throwbacks".to_string(),
            (_, 3) => "Whiteout".to_string(),
            _ => "Special edition".to_string(),
        };
        v.push(label);
    }
    v
}

/// The set a team wears in a game: AI staffs dress for the moment
/// (throwbacks for a rivalry, blackouts and alternates under the lights and
/// for big games); your pick when you made one.
pub fn set_for_game(w: &World, t: TeamKey, big: bool, rivalry: bool, home: bool, key_n: u64) -> String {
    let s = sets(w, t);
    if Some(t) == super::seat_team(w)
        && let Some(k) = w.stories.traditions.uniforms.next
        && let Some(x) = s.get(k as usize)
    {
        return x.clone();
    }
    let has = |name: &str| s.iter().any(|x| x == name);
    let u = key(key_n, 0x51);
    if rivalry && has("Throwbacks") && u < 0.35 {
        return "Throwbacks".into();
    }
    if big && home && has("Blackout") && u < 0.4 {
        return "Blackout".into();
    }
    if big && has("Alternates") && u < 0.3 {
        return "Alternates".into();
    }
    if s.len() > 2 && u < 0.12 {
        return s[2 + (key(key_n, 0x52) * (s.len() - 2) as f64) as usize % (s.len() - 2)].clone();
    }
    if home { "Home".into() } else { "Road whites".into() }
}

/// A game's uniforms for your team: what you wore goes in the book, and a
/// recruit on a visit reacts to a flashy set (a liked set is one more good
/// touch in his recruitment).
fn wore(w: &mut World, ctx: &GameCtx, won: bool, set: &str) {
    let seed = w.options.seed;
    let year = w.year;
    w.stories.traditions.uniforms.worn.push((w.today, set.to_string(), won));
    if w.stories.traditions.uniforms.worn.len() > 120 {
        w.stories.traditions.uniforms.worn.remove(0);
    }
    w.stories.traditions.uniforms.next = None;
    let Some(TeamKey::College(p)) = super::seat_team(w) else { return };
    if matches!(set, "Home" | "Road whites") || ctx.teams[0] != TeamKey::College(p) {
        return;
    }
    // Recruits on campus today.
    let d = w.today;
    let visitors: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            let q = &w.people[i];
            q.stage == crate::world::Stage::HighSchool && q.cal.visits.iter().any(|v| v.0 == p as usize && v.1 == d)
        })
        .collect();
    for i in visitors.into_iter().take(40) {
        let ego = w.people[i].player.personality.traits.ego as f64;
        let like = 0.25 + 0.03 * (ego - 10.0);
        let r = w.stories.rng(seed).next();
        let name = w.people[i].name.clone();
        let f = team_fill(w, TeamKey::College(p), Fill::new()).set("name", name).set("set", set.to_lowercase());
        if r < like {
            w.people[i].cal.visits.push((p as usize, d, crate::world::Visit::DropIn));
            if w.people[i].stars >= 4
                && let Some(t) = w.stories.line(seed, "uniform.recruit_like", &f, year)
            {
                let e = news(w, Level::Recruiting, Loudness::Small, Some(TeamKey::College(p)), t);
                post(w, e);
            }
        } else if r > 0.93
            && w.people[i].stars >= 4
            && let Some(t) = w.stories.line(seed, "uniform.recruit_meh", &f, year)
        {
            let e = news(w, Level::Recruiting, Loudness::Small, Some(TeamKey::College(p)), t);
            post(w, e);
        }
    }
    let _ = won;
}

/// Your record in each set you've worn.
pub fn record_by_set(w: &World) -> Vec<(String, u32, u32)> {
    let mut v: Vec<(String, u32, u32)> = Vec::new();
    for (_, s, won) in &w.stories.traditions.uniforms.worn {
        match v.iter_mut().find(|x| &x.0 == s) {
            Some(x) => {
                if *won {
                    x.1 += 1
                } else {
                    x.2 += 1
                }
            }
            None => v.push((s.clone(), *won as u32, !*won as u32)),
        }
    }
    v
}

/// Summer reveals: programs with money unveil a new alternate in July,
/// NFL clubs a throwback or new set in the spring.
fn reveals(w: &mut World, d: Date) {
    let year = w.year;
    let seed = w.options.seed;
    let college_day = d.month() == 7 && d.weekday() == Weekday::Tue && year >= 2002;
    let nfl_day = d.month() == 4 && d.weekday() == Weekday::Thu && year >= 2008;
    if !(college_day || nfl_day) {
        return;
    }
    let mut out: Vec<(TeamKey, String)> = Vec::new();
    if college_day {
        for p in &w.programs {
            if p.division != Division::FBS {
                continue;
            }
            let chance = (p.prestige * 0.5 + p.facilities * 0.3 - 0.35).clamp(0.0, 0.5) / 4.0;
            if key(p.id as u64 ^ (year as u64) << 20, d.0 as u64) < chance {
                out.push((TeamKey::College(p.id as u32), String::new()));
            }
        }
    }
    if nfl_day {
        for t in 0..w.nfl.len() {
            if key(t as u64 ^ (year as u64) << 20, d.0 as u64) < 0.02 {
                out.push((TeamKey::Nfl(t as u16), String::new()));
            }
        }
    }
    for (t, _) in out.into_iter().take(4) {
        let wk = if matches!(t, TeamKey::Nfl(_)) { "words.uniform.nfl_alt" } else { "words.uniform.alt" };
        let f = team_fill(w, t, Fill::new()).set("year", (year - 30 - (key(year as u64, 3) * 30.0) as i32).to_string());
        let set = {
            let mut rng = w.stories.rng.take().unwrap_or_else(|| crate::rng::Rng::new(seed ^ 0x5709_1E55));
            let s = super::lines::word(&mut rng, wk, &f, year);
            w.stories.rng = Some(rng);
            s
        };
        let Some(set) = set else { continue };
        let f = f.set("set", set.clone());
        if let Some(text) = w.stories.line(seed, "uniform.reveal", &f, year) {
            let loud = if Some(t) == super::seat_team(w) { Loudness::Medium } else { Loudness::Small };
            let e = news(w, t.level(), loud, Some(t), text);
            post(w, e);
            w.stories.traditions.uniforms.reveals.push((t, year as i16, set));
        }
    }
    let keep = year - 10;
    w.stories.traditions.uniforms.reveals.retain(|r| r.1 as i32 >= keep);
}

// --- C5: retired numbers, rings of honor, statues ------------------------------------------------

/// Whether a team retires numbers (C5; `retired_numbers_policy.csv`).
fn policy(w: &World, t: TeamKey) -> super::media::HonorPolicy {
    super::media::honor_policy(&team_name(w, t), t.level())
}

/// The yearly honors (C5), on the first of March: a college star whose
/// career just ended (a Heisman or other national award, or two
/// All-America years) has his number retired or jersey honored where the
/// school does that; a Hall of Fame inductee goes into his team's ring of
/// honor, his number retired where the club retires numbers, and the
/// greatest get a statue. Ceremonies come at a home game in the fall.
fn yearly_honors(w: &mut World) {
    let year = w.year;
    let mut add: Vec<TeamHonor> = Vec::new();
    // College: careers that ended last season.
    let last = year - 1;
    for (&id, h) in &w.stats.honors {
        let years: Vec<i32> = h.all_america.iter().map(|x| x.0).chain(h.awards.iter().map(|x| x.0)).collect::<Vec<_>>();
        if years.iter().copied().max() != Some(last) {
            continue;
        }
        let big_award = h.awards.iter().any(|a| matches!(a.1, "Heisman Trophy" | "Heisman"));
        let national = h.awards.len();
        let aa = h.all_america.iter().filter(|x| matches!(x.1, Division::FBS | Division::FCS)).count();
        if !big_award && aa < 2 && national < 2 {
            continue;
        }
        // His program: the last one he played for.
        let prog = w.stats.by_player.get(&id).and_then(|v| v.last()).map(|&k| w.stats.seasons[k as usize].program);
        let Some(prog) = prog else { continue };
        let t = TeamKey::College(prog as u32);
        if w.stories.traditions.honors.iter().any(|x| x.player == id && x.team == t) {
            continue;
        }
        let pol = policy(w, t);
        let kind = if big_award && pol.retires_numbers {
            HonorKind::RetiredNumber
        } else if pol.never {
            continue;
        } else {
            HonorKind::RetiredJersey
        };
        let first = w.stats.by_player.get(&id).and_then(|v| v.first()).map(|&k| w.stats.seasons[k as usize].year);
        let number = super::numbers::numbers(w, t).get(&id).copied().or_else(|| Some(number_guess(id, h.pos)));
        add.push(TeamHonor {
            team: t,
            kind,
            player: id,
            name: h.name.clone(),
            pos: h.pos.unwrap_or(Position::QB),
            number,
            year: year as i16,
            years: format!("{}-{}", first.map_or(last, |x| x as i32), last),
            ceremony_done: false,
        });
        // A Heisman winner at a school that builds statues (proposed, not
        // confirmed: Oklahoma's Heisman Park, Texas, Florida, USC, Ohio State).
        if big_award && pol.statues {
            add.push(TeamHonor {
                team: t,
                kind: HonorKind::Statue,
                player: id,
                name: h.name.clone(),
                pos: h.pos.unwrap_or(Position::QB),
                number,
                year: (year + 2) as i16,
                years: format!("{}-{}", first.map_or(last, |x| x as i32), last),
                ceremony_done: false,
            });
        }
    }
    // NFL: last summer's Hall of Fame class gets the club's honors.
    let hall: Vec<(i32, u32, String, Position, String)> =
        w.nfl_book.hall.iter().filter(|x| x.0 == last || x.0 == year).cloned().collect();
    for (_, id, name, pos, _) in hall {
        // His club: where he played most seasons.
        let Some(club) = most_seasons_club(w, id) else { continue };
        let t = TeamKey::Nfl(club as u16);
        if w.stories.traditions.honors.iter().any(|x| x.player == id && x.team == t) {
            continue;
        }
        let pol = policy(w, t);
        let years = club_years(w, id, club).unwrap_or_default();
        let number = Some(number_guess(id, Some(pos)));
        if pol.ring {
            add.push(honor(t, HonorKind::RingOfHonor, id, &name, pos, number, year, &years));
        }
        let mvps = w.nfl_book.awards.iter().filter(|a| a.2 == id && a.1.contains("MVP")).count();
        if pol.retires_numbers && (mvps >= 1 || key(id as u64, 0xC5) < 0.35) {
            add.push(honor(t, HonorKind::RetiredNumber, id, &name, pos, number, year, &years));
        }
        if mvps >= 2 && key(id as u64, 0x57) < 0.5 {
            add.push(honor(t, HonorKind::Statue, id, &name, pos, number, year + 1, &years));
        }
    }
    // Ring of honor before the Hall: a long-time star at the club, a few
    // years after he retires (five Pro Bowls with the club).
    let mut bowls: std::collections::HashMap<(u32, usize), u32> = Default::default();
    for &(_, id, team) in &w.nfl_book.pro_bowls {
        *bowls.entry((id, team)).or_default() += 1;
    }
    for ((id, team), n) in bowls {
        if n < 5 {
            continue;
        }
        let t = TeamKey::Nfl(team as u16);
        if w.stories.traditions.honors.iter().any(|x| x.player == id && x.team == t)
            || add.iter().any(|x| x.player == id && x.team == t)
        {
            continue;
        }
        // Retired three seasons ago.
        let gone = w.alumni.iter().rev().take(400_000).find(|a| {
            a.player.as_ref().is_some_and(|p| p.id == id)
                && a.pro.as_ref().and_then(|c| c.seasons.last()).is_some_and(|s| s.year == year - 3)
        });
        let Some(a) = gone else { continue };
        if !policy(w, t).ring {
            continue;
        }
        let pos = a.position;
        let name = a.name.clone();
        let years = club_years(w, id, team).unwrap_or_default();
        add.push(honor(t, HonorKind::RingOfHonor, id, &name, pos, Some(number_guess(id, Some(pos))), year, &years));
    }
    w.stories.traditions.honors.extend(add);
}

#[allow(clippy::too_many_arguments)]
fn honor(
    t: TeamKey,
    kind: HonorKind,
    id: u32,
    name: &str,
    pos: Position,
    number: Option<u8>,
    year: i32,
    years: &str,
) -> TeamHonor {
    TeamHonor {
        team: t,
        kind,
        player: id,
        name: name.to_string(),
        pos,
        number,
        year: year as i16,
        years: years.to_string(),
        ceremony_done: false,
    }
}

/// A plausible number for a player no longer on a roster, fixed by his id
/// and position.
fn number_guess(id: u32, pos: Option<Position>) -> u8 {
    let list = super::numbers::allowed(pos.unwrap_or(Position::QB), Level::College, 2010);
    list[(key(id as u64, 0x23) * list.len() as f64) as usize % list.len()]
}

fn pro_seasons(w: &World, id: u32) -> Option<Vec<(i32, Option<usize>)>> {
    if let Some(p) = w.people.iter().find(|p| p.player.id == id) {
        return p.pro.as_ref().map(|c| c.seasons.iter().map(|s| (s.year, s.team)).collect());
    }
    w.alumni
        .iter()
        .rev()
        .find(|a| a.player.as_ref().is_some_and(|p| p.id == id))
        .and_then(|a| a.pro.as_ref().map(|c| c.seasons.iter().map(|s| (s.year, s.team)).collect()))
}

fn most_seasons_club(w: &World, id: u32) -> Option<usize> {
    let s = pro_seasons(w, id)?;
    let mut n: std::collections::HashMap<usize, u32> = Default::default();
    for (_, t) in s {
        if let Some(t) = t {
            *n.entry(t).or_default() += 1;
        }
    }
    n.into_iter().max_by_key(|x| (x.1, std::cmp::Reverse(x.0))).map(|x| x.0)
}

fn club_years(w: &World, id: u32, club: usize) -> Option<String> {
    let s = pro_seasons(w, id)?;
    let ys: Vec<i32> = s.iter().filter(|x| x.1 == Some(club)).map(|x| x.0).collect();
    Some(format!("{}-{}", ys.iter().min()?, ys.iter().max()?))
}

/// A home game day for a team in the honor's season: the ceremony.
fn ceremonies(w: &mut World, d: Date) {
    let year = w.year as i16;
    let todo: Vec<usize> = (0..w.stories.traditions.honors.len())
        .filter(|&k| {
            let h = &w.stories.traditions.honors[k];
            !h.ceremony_done && h.year <= year
        })
        .collect();
    if todo.is_empty() {
        return;
    }
    let seed = w.options.seed;
    for k in todo {
        let h = w.stories.traditions.honors[k].clone();
        let opp = match h.team {
            TeamKey::College(p) => w
                .college
                .as_ref()
                .and_then(|c| c.games.iter().find(|g| g.date == d && g.home == p as usize && g.site.is_none()))
                .map(|g| w.program_name(g.away).to_string()),
            TeamKey::Nfl(t) => w
                .nfl_fixtures(Some(t as usize))
                .into_iter()
                .find(|f| f.when == Some(d) && f.home == t as usize)
                .map(|f| w.nfl_team_name(f.away)),
            TeamKey::Hs(_) => None,
        };
        let Some(opp) = opp else { continue };
        w.stories.traditions.honors[k].ceremony_done = true;
        let keyname = match h.kind {
            HonorKind::RetiredNumber => "honor.retired_number",
            HonorKind::RetiredJersey => "honor.retired_jersey",
            HonorKind::RingOfHonor => "honor.ring",
            HonorKind::Statue => "honor.statue",
        };
        let f = team_fill(w, h.team, Fill::new())
            .set("name", h.name.clone())
            .set("num", h.number.map(|n| n.to_string()).unwrap_or_default())
            .set("years", h.years.clone())
            .set("opp", opp)
            .set("site", stadium_of(w, h.team));
        let mut text = w.stories.line(seed, keyname, &f, w.year).unwrap_or_default();
        if let Some(c) = w.stories.line(seed, "honor.ceremony", &f, w.year) {
            text = format!("{text} {c}").trim().to_string();
        }
        if text.is_empty() {
            continue;
        }
        let mine = super::seat_team(w) == Some(h.team);
        let e = news(w, h.team.level(), if mine { Loudness::Medium } else { Loudness::Small }, Some(h.team), text);
        post(w, Event { kind: Kind::Ceremony, ..e });
        // You coached him: an invitation, and a photo for your office.
        if coached_by_you(w, h.player) {
            let f = f.set("coach", "you");
            if let Some(t) = w.stories.line(seed, "honor.ceremony_coach", &f, w.year) {
                let e = news(w, h.team.level(), Loudness::Medium, super::seat_team(w), t);
                post(w, e);
            }
            let m = format!("{} ceremony for {}, {}", h.kind.name(), h.name, team_name(w, h.team));
            super::story::memento(w, super::story::MementoKind::Photo, m);
        }
    }
}

fn stadium_of(w: &World, t: TeamKey) -> String {
    match t {
        TeamKey::Nfl(n) => {
            let name = w.nfl_team_name(n as usize);
            super::media::nfl_stadium(&name).unwrap_or_else(|| "outside the stadium".to_string())
        }
        _ => "outside the stadium".to_string(),
    }
}

/// Whether you were his head coach for a season (your career's kept
/// rosters: the players in your games).
pub(crate) fn coached_by_you(w: &World, id: u32) -> bool {
    w.stories.story.players.contains(&id)
}

// --- C13, C14: parades, rings, bowl gifts ----------------------------------------------------------

fn parade(w: &mut World, t: TeamKey, d: Date) {
    let year = w.year;
    let seed = w.options.seed;
    let fans = match t {
        TeamKey::College(p) => w.programs[p as usize].fans.clamp(0.3, 1.6),
        TeamKey::Nfl(n) => 0.8 + 0.6 * (1.0 - (super::media::market_rank(w, n as usize) as f64 - 1.0) / 31.0),
        TeamKey::Hs(_) => 0.2,
    };
    let base = match t {
        TeamKey::Nfl(_) => 450_000.0,
        TeamKey::College(_) => 45_000.0,
        TeamKey::Hs(_) => 4_000.0,
    };
    let crowd = (base * fans * (0.7 + 0.6 * key(year as u64, d.0 as u64)) / 1000.0).round() as u32 * 1000;
    let weather = if d.month() <= 2 && key(d.0 as u64, 9) < 0.35 { "in a light snow" } else { "" };
    let f = team_fill(w, t, Fill::new()).set("crowd", format!("about {}", thousands(crowd))).set("weather", weather);
    let k = if matches!(t, TeamKey::Hs(_)) { "parade.hs" } else { "parade.news" };
    let f = if let TeamKey::Hs(h) = t { f.set("town", w.high_schools[h as usize].town.clone()) } else { f };
    if let Some(text) = w.stories.line(seed, k, &f, year) {
        let mine = super::seat_team(w) == Some(t);
        let e = news(w, t.level(), if mine { Loudness::Big } else { Loudness::Medium }, Some(t), text);
        post(w, Event { kind: Kind::Ceremony, ..e });
    }
    // A player's moment on the bus.
    let star = super::numbers::roster(w, t).into_iter().max_by_key(|&(i, _)| {
        (w.people[i].player.personality.traits.ego as u32) * 3 + (key(i as u64, 5) * 10.0) as u32
    });
    if let Some((i, _)) = star {
        let f = team_fill(w, t, Fill::new()).set("name", w.people[i].name.clone());
        if let Some(text) = w.stories.line(seed, "parade.moment", &f, year) {
            let e = news(w, t.level(), Loudness::Small, Some(t), text);
            post(w, e);
        }
    }
    if super::seat_team(w) == Some(t) {
        let f = team_fill(w, t, Fill::new()).set("crowd", format!("about {}", thousands(crowd)));
        if let Some(text) = w.stories.line(seed, "parade.coach", &f, year) {
            let e = news(w, t.level(), Loudness::Medium, Some(t), text);
            post(w, e);
        }
        super::story::memento(w, super::story::MementoKind::Photo, format!("The parade, {year}"));
    }
    w.stories.traditions.parades.push((year as i16, t, crowd));
}

fn rings(w: &mut World, d: Date) {
    let year = w.year;
    let seed = w.options.seed;
    // The Super Bowl champion's rings (the clock's own line says the day; this is the ring).
    if d == w.dates.ring_ceremony
        && let Some(t) = w.prev_champion()
    {
        let tk = TeamKey::Nfl(t as u16);
        let titles = w.nfl_champions.iter().filter(|c| c.1 == t).count();
        let diamonds = 150 + (key(year as u64, t as u64) * 250.0) as u32;
        let f = team_fill(w, tk, Fill::new()).set("year", (year - 1).to_string());
        let detail = {
            let mut rng = w.stories.rng.take().unwrap_or_else(|| crate::rng::Rng::new(seed ^ 0x5709_1E55));
            let s = super::lines::word(&mut rng, "words.ring.detail", &f, year);
            w.stories.rng = Some(rng);
            s
        };
        let f = f
            .set("diamonds", diamonds.to_string())
            .set("detail", detail.unwrap_or_default())
            .set("n", titles.to_string());
        if let Some(text) = w.stories.line(seed, "ring.ceremony", &f, year) {
            let e = news(w, Level::Nfl, Loudness::Small, Some(tk), text);
            post(w, Event { kind: Kind::Ceremony, ..e });
        }
        if super::seat_team(w) == Some(tk) || w.stories.story.titles.iter().any(|x| x.0 == year - 1 && x.1 == tk) {
            super::story::memento(w, super::story::MementoKind::Ring, format!("Super Bowl ring, {} season", year - 1));
        }
    }
    // College title and conference rings at the spring game.
    if d == w.dates.spring_game {
        let mine = super::seat_team(w);
        let last = year - 1;
        for (y, t, what) in w.stories.story.titles.clone() {
            if y != last || Some(t) != mine {
                continue;
            }
            let f = team_fill(w, t, Fill::new()).set("year", last.to_string()).set("conf", what.clone());
            let key = if what.contains("national") || what.contains("state") { "ring.ceremony" } else { "ring.conf" };
            let f = f.set("diamonds", "dozens").set("detail", "");
            if let Some(text) =
                w.stories.line(seed, if matches!(t, TeamKey::Hs(_)) { "ring.hs" } else { key }, &f, year)
            {
                let e = news(w, t.level(), Loudness::Medium, Some(t), text);
                post(w, Event { kind: Kind::Ceremony, ..e });
            }
            super::story::memento(w, super::story::MementoKind::Ring, format!("{what} ring, {last}"));
        }
    }
    // A broke former player sells his ring (a few a decade; proposed, not
    // confirmed: once in about three seasons).
    if d.month() == 5 && d.day() == 20 && w.stories.rng(seed).chance(0.33) {
        let champs: Vec<(i32, usize)> = w.nfl_champions.clone();
        let cand = w.alumni.iter().rev().take(300_000).find(|a| {
            a.pro.as_ref().is_some_and(|c| {
                c.earnings < 4.0e6
                    && c.seasons
                        .iter()
                        .any(|s| s.games > 0 && champs.iter().any(|ch| ch.0 == s.year && Some(ch.1) == s.team))
                    && key(a.class_of as u64, d.0 as u64 ^ a.name.len() as u64) < 0.08
            })
        });
        if let Some(a) = cand {
            let c = a.pro.as_ref().unwrap();
            let s = c.seasons.iter().find(|s| champs.iter().any(|ch| ch.0 == s.year && Some(ch.1) == s.team)).unwrap();
            let team = s.team.unwrap();
            let price = 18_000 + (key(year as u64, team as u64) * 60_000.0) as u32;
            let f = team_fill(w, TeamKey::Nfl(team as u16), Fill::new())
                .set("name", a.name.clone())
                .set("year", s.year.to_string())
                .set("price", dollars(price / 1000 * 1000));
            if let Some(text) = w.stories.line(seed, "ring.sold", &f, year) {
                let e = news(w, Level::Nfl, Loudness::Small, None, text);
                post(w, e);
            }
        }
    }
}

/// Your bowl's gift suite (NCAA limits what a bowl may give each player).
fn bowl_gift(w: &mut World, d: Date) {
    let Some(TeamKey::College(p)) = super::seat_team(w) else { return };
    let Some(cs) = w.college.as_ref() else { return };
    let Some(g) =
        cs.games.iter().find(|g| matches!(g.kind, GameKind::Bowl(_)) && g.involves(p as usize) && g.date == d.add(5))
    else {
        return;
    };
    let Some(bowl) = g.name else { return };
    let seed = w.options.seed;
    let year = w.year;
    let f = team_fill(w, TeamKey::College(p), Fill::new()).set("bowl", bowl);
    let gift = {
        let mut rng = w.stories.rng.take().unwrap_or_else(|| crate::rng::Rng::new(seed ^ 0x5709_1E55));
        let s = super::lines::word(&mut rng, "words.bowl_gift", &f, year);
        w.stories.rng = Some(rng);
        s
    };
    let f = f.set("gift", gift.unwrap_or_default());
    if let Some(text) = w.stories.line(seed, "ring.bowl_gift", &f, year) {
        let e = news(w, Level::College, Loudness::Medium, Some(TeamKey::College(p)), text);
        post(w, e);
        super::story::memento(w, super::story::MementoKind::Gift, format!("{bowl} gift, {year}"));
    }
}

pub(crate) fn day(w: &mut World, d: Date) {
    let y = w.dates.clone();
    if d.weekday() == Weekday::Sun && w.college.as_ref().is_some_and(|c| c.started) {
        show_pick(w, d);
    }
    if d.weekday() == Weekday::Sat {
        show_day(w, d);
    }
    reveals(w, d);
    if d.month() == 3 && d.day() == 1 && w.stories.once(1, w.year) {
        yearly_honors(w);
    }
    if (d.month() >= 9 || d.month() <= 1)
        && (d.weekday() == Weekday::Sat
            || d.weekday() == Weekday::Sun
            || d.weekday() == Weekday::Mon
            || d.weekday() == Weekday::Thu)
    {
        ceremonies(w, d);
    }
    rings(w, d);
    if d >= y.bowl_season.0.add(-7) && d <= y.bowl_season.1 {
        bowl_gift(w, d);
    }
    // Parades: two days after the college title game and the Super Bowl
    // (the Super Bowl falls in last football year's dates).
    if d == y.title_game.add(2)
        && let Some(c) = w.college.as_ref().and_then(|c| c.champions[0])
    {
        parade(w, TeamKey::College(c as u32), d);
    }
    if let Some(p) = w.prev_dates.clone()
        && d == p.super_bowl.add(2)
        && let Some(&(yr, t)) = w.nfl_champions.last()
        && yr == p.season
    {
        parade(w, TeamKey::Nfl(t as u16), d);
    }
    // Your high school's state title.
    if let Some(TeamKey::Hs(h)) = super::seat_team(w)
        && let Some(s) = w.high_schools.get(h as usize).and_then(|s| s.seasons.last())
        && s.state_title
        && s.year == w.year
        && !w.stories.traditions.parades.iter().any(|x| x.0 as i32 == w.year && x.1 == TeamKey::Hs(h))
    {
        parade(w, TeamKey::Hs(h), d);
    }
}

/// Uniforms for your game, when the hook knows the result.
pub(crate) fn seat_uniform(w: &mut World, ctx: &GameCtx, won: bool) {
    let Some(me) = super::seat_team(w) else { return };
    let home = ctx.teams[0] == me;
    if !ctx.teams.contains(&me) {
        return;
    }
    let set = set_for_game(w, me, ctx.big, ctx.rivalry, home, w.today.0 as u64);
    wore(w, ctx, won, &set);
}
