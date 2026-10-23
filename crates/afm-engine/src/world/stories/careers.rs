//! C15-C17: the ends of careers and the joy in the middle. Farewell tours
//! (a star announces his last season; rival clubs give gifts at his last
//! game in their building; his last game; your tribute), retirements of
//! note, one-day contracts (a star who finished elsewhere retires as a
//! member of the club he gave the most years), and signature celebrations
//! by personality under each era's rules (the engine throws the flags on
//! its own draw: `game/flow.rs`).

use std::collections::HashMap;

use crate::calendar::{Date, Level, Loudness};
use crate::game::GameStats;
use crate::game::sim::FoulKind;
use crate::world::{Stage, World};

use super::lines::{self, Fill};
use super::{GameCtx, TeamKey, news, post, team_fill};

/// A star's last season, announced.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Farewell {
    pub id: u32,
    pub year: i16,
    pub team: u16,
    /// Clubs that gave him a gift, with the gift.
    pub gifts: Vec<(u16, String)>,
    pub last_game: bool,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Careers {
    pub farewells: Vec<Farewell>,
    /// Signature celebrations: (player id, name, season it was born).
    pub celebrations: Vec<(u32, String, i16)>,
    /// One-day contracts signed: (player id, season, club).
    pub one_day: Vec<(u32, i16, u16)>,
    /// A retired star asking to retire with your club: (id, club, asked).
    pub offer: Option<(u32, u16, Date)>,
}

impl Careers {
    /// Whether a player announced this season as his last.
    pub fn farewell_due(&self, id: u32, year: i32) -> bool {
        self.farewells.iter().any(|f| f.id == id && f.year as i32 == year)
    }

    pub fn celebration(&self, id: u32) -> Option<&str> {
        self.celebrations.iter().find(|c| c.0 == id).map(|c| c.1.as_str())
    }
}

fn pro_bowls(w: &World) -> HashMap<u32, u32> {
    let mut m = HashMap::new();
    for x in &w.nfl_book.pro_bowls {
        *m.entry(x.1).or_insert(0) += 1;
    }
    m
}

/// Late August: the stars who say this is their last season. A showman
/// (ego) or a leader takes a farewell tour; a quiet man just retires in
/// the spring.
fn farewell_tours(w: &mut World, d: Date) {
    let year = w.year;
    if !(d.month() == 8 && d.day() == 25) || !w.stories.once(60, year) {
        return;
    }
    let bowls = pro_bowls(w);
    let mut picks = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        if p.stage != Stage::Pro {
            continue;
        }
        let Some(c) = p.pro.as_ref() else { continue };
        let Some(t) = c.team else { continue };
        let pb = bowls.get(&p.player.id).copied().unwrap_or(0);
        let seasons = c.seasons.iter().filter(|s| s.games > 0).count();
        if pb < 4 || seasons < 10 {
            continue;
        }
        let age = p.player.age as f64;
        if age < crate::career::retire_age(p.player.position) + 1.0 {
            continue;
        }
        let tr = p.player.personality.traits;
        let lean = 0.15 + 0.03 * (tr.ego as f64 - 10.0).max(0.0) + 0.02 * (tr.leadership as f64 - 10.0).max(0.0);
        if super::traditions::key(p.player.id as u64, (year as u64) << 5 ^ 0xFA4E) < lean {
            picks.push((i, t, seasons));
        }
    }
    let seed = w.options.seed;
    for (i, t, seasons) in picks {
        let id = w.people[i].player.id;
        let me = TeamKey::Nfl(t as u16);
        w.stories.careers.farewells.push(Farewell {
            id,
            year: year as i16,
            team: t as u16,
            gifts: Vec::new(),
            last_game: false,
        });
        let f = team_fill(w, me, Fill::new()).set("name", w.people[i].name.clone()).set("years", seasons.to_string());
        if let Some(text) = w.stories.line(seed, "farewell.announce", &f, year) {
            let e = news(w, Level::Nfl, Loudness::Medium, Some(me), text);
            post(w, e);
        }
    }
    w.stories.careers.farewells.retain(|f| f.year as i32 >= year - 20);
}

/// After an NFL game: farewell gifts on the road, his last game, and the
/// celebrations.
pub(crate) fn after_nfl_game(w: &mut World, ctx: &GameCtx, s: &GameStats) {
    let year = w.year;
    let seed = w.options.seed;
    if !ctx.postseason {
        let tours: Vec<usize> = (0..w.stories.careers.farewells.len())
            .filter(|&k| w.stories.careers.farewells[k].year as i32 == year)
            .collect();
        for k in tours {
            let f = w.stories.careers.farewells[k].clone();
            let Some(side) = (0..2).find(|&x| ctx.teams[x] == TeamKey::Nfl(f.team)) else { continue };
            if !s.players[side].contains_key(&f.id) {
                continue;
            }
            let Some(i) = super::person_index(w, f.id) else { continue };
            let name = w.people[i].name.clone();
            // On the road: the home club's gift before kickoff.
            if side == 1 {
                let TeamKey::Nfl(host) = ctx.teams[0] else { continue };
                if f.gifts.iter().any(|g| g.0 == host) {
                    continue;
                }
                let mut rng = w.stories.rng.take().unwrap_or_else(|| crate::rng::Rng::new(seed ^ 0x5709_1E55));
                let gift = lines::word(&mut rng, "words.gift", &team_fill(w, ctx.teams[0], Fill::new()), year);
                w.stories.rng = Some(rng);
                let Some(gift) = gift else { continue };
                let fill = team_fill(w, ctx.teams[1], Fill::new())
                    .set("name", name)
                    .set("opp", super::short_name(w, ctx.teams[0]))
                    .set("gift", gift.clone());
                if let Some(text) = w.stories.line(seed, "farewell.gift", &fill, year) {
                    let e = news(w, Level::Nfl, Loudness::Small, Some(ctx.teams[1]), text);
                    post(w, e);
                }
                w.stories.careers.farewells[k].gifts.push((host, gift));
            }
            // His club's last regular-season game.
            if !f.last_game && last_regular_game(w, f.team as usize) == Some(w.today) {
                w.stories.careers.farewells[k].last_game = true;
                let fill = team_fill(w, TeamKey::Nfl(f.team), Fill::new())
                    .set("name", w.people[i].name.clone())
                    .set("opp", super::short_name(w, ctx.teams[1 - side]));
                if let Some(text) = w.stories.line(seed, "farewell.last_game", &fill, year) {
                    let e = news(w, Level::Nfl, Loudness::Medium, Some(TeamKey::Nfl(f.team)), text);
                    post(w, e);
                }
            }
        }
    }
    celebrations(w, ctx, s);
}

fn last_regular_game(w: &World, t: usize) -> Option<Date> {
    let s = w.nfl_season.as_ref()?;
    let mut last = None;
    for (k, week) in s.weeks.iter().enumerate() {
        for (g, &(h, a)) in week.iter().enumerate() {
            if h == t || a == t {
                last = s.slots.get(k).and_then(|x| x.get(g)).map(|x| x.at.date).max(last);
            }
        }
    }
    last
}

/// A notable retirement (called from `pro::retire` before he leaves his
/// roster): the news, and your tribute to a man you coached.
pub(crate) fn retired(w: &mut World, i: usize) {
    if !w.recording {
        return;
    }
    let Some(c) = w.people[i].pro.as_ref() else { return };
    let id = w.people[i].player.id;
    let seasons = c.seasons.iter().filter(|s| s.games > 0).count();
    let team = c.team.or_else(|| c.seasons.iter().rev().find_map(|s| s.team));
    let pb = w.nfl_book.pro_bowls.iter().filter(|x| x.1 == id).count();
    if pb < 3 && seasons < 12 {
        return;
    }
    let Some(t) = team else { return };
    let year = w.year;
    let seed = w.options.seed;
    let me = TeamKey::Nfl(t as u16);
    let name = w.people[i].name.clone();
    let f = team_fill(w, me, Fill::new()).set("name", name.clone()).set("years", seasons.to_string());
    if let Some(text) = w.stories.line(seed, "farewell.retire", &f, year) {
        let e = news(w, Level::Nfl, if pb >= 5 { Loudness::Medium } else { Loudness::Small }, Some(me), text);
        post(w, e);
    }
    if super::traditions::coached_by_you(w, id)
        && let Some(coach) = w.career.as_ref().map(|c| c.you.name.clone())
    {
        let f = Fill::new().set("coach", coach).set("name", name.clone());
        if let Some(text) = w.stories.line(seed, "farewell.tribute", &f, year) {
            let team = super::seat_team(w);
            let e = news(w, team.map_or(Level::Nfl, |t| t.level()), Loudness::Medium, team, text);
            post(w, e);
        }
        super::story::memento(w, super::story::MementoKind::Photo, format!("With {name} on his last day, {year}"));
    }
}

/// A retired star who finished elsewhere and wants to go out with the club
/// he gave the most years: (index or alumnus, id, name, club, seasons there).
fn one_day_candidates(w: &World) -> Vec<(u32, String, usize, usize)> {
    let year = w.year;
    let bowls = pro_bowls(w);
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut look = |id: u32, name: &str, c: &crate::world::pro::ProCareer| {
        if !seen.insert(id) {
            return;
        }
        let played: Vec<(i32, usize)> =
            c.seasons.iter().filter(|s| s.games > 0).filter_map(|s| Some((s.year, s.team?))).collect();
        let Some(&(last_year, last_team)) = played.last() else { return };
        if !(year - 4..year).contains(&last_year) {
            return;
        }
        let mut n: HashMap<usize, usize> = HashMap::new();
        for &(_, t) in &played {
            *n.entry(t).or_default() += 1;
        }
        let Some((&club, &there)) = n.iter().max_by_key(|x| (*x.1, std::cmp::Reverse(*x.0))) else { return };
        let pb = bowls.get(&id).copied().unwrap_or(0);
        if club == last_team || there < 6 || pb < 2 {
            return;
        }
        out.push((id, name.to_string(), club, there));
    };
    for p in w.people.iter().filter(|p| p.stage == Stage::Done) {
        if let Some(c) = p.pro.as_ref() {
            look(p.player.id, &p.name, c);
        }
    }
    for a in w.alumni.iter().rev().take(4000) {
        if let (Some(c), Some(pl)) = (a.pro.as_ref(), a.player.as_ref()) {
            look(pl.id, &a.name, c);
        }
    }
    out
}

/// Spring: one-day contracts. Your club's are yours to say yes to.
fn one_day(w: &mut World, d: Date) {
    let year = w.year;
    let seed = w.options.seed;
    // An offer you left waiting ten days: the front office signs him.
    if let Some((id, club, asked)) = w.stories.careers.offer
        && d >= asked.add(10)
    {
        sign_one_day(w, id, club as usize, true);
    }
    if !(d.month() == 4 && d.day() == 20) || !w.stories.once(61, year) {
        return;
    }
    let mine = match super::seat_team(w) {
        Some(TeamKey::Nfl(t)) => Some(t as usize),
        _ => None,
    };
    for (id, name, club, there) in one_day_candidates(w) {
        if w.stories.careers.one_day.iter().any(|x| x.0 == id) {
            continue;
        }
        if super::traditions::key(id as u64, (year as u64) << 7 ^ 0x1DA7) >= 0.35 {
            continue;
        }
        if Some(club) == mine && w.stories.careers.offer.is_none() {
            let me = TeamKey::Nfl(club as u16);
            let f = team_fill(w, me, Fill::new()).set("name", name.clone()).set("years", there.to_string());
            if let Some(text) = w.stories.line(seed, "oneday.offer", &f, year) {
                let e = news(w, Level::Nfl, Loudness::Medium, Some(me), text).decision().rule(super::ASK_RULE);
                post(w, e);
            }
            w.stories.careers.offer = Some((id, club as u16, d));
            continue;
        }
        sign_one_day(w, id, club, false);
    }
}

fn sign_one_day(w: &mut World, id: u32, club: usize, yours: bool) {
    let year = w.year;
    let seed = w.options.seed;
    if w.stories.careers.offer.is_some_and(|o| o.0 == id) {
        w.stories.careers.offer = None;
    }
    let Some((_, name, _, there)) = one_day_candidates(w).into_iter().find(|x| x.0 == id) else { return };
    let me = TeamKey::Nfl(club as u16);
    w.stories.careers.one_day.push((id, year as i16, club as u16));
    let f = team_fill(w, me, Fill::new()).set("name", name.clone()).set("years", there.to_string());
    if let Some(text) = w.stories.line(seed, "oneday.sign", &f, year) {
        let e = news(w, Level::Nfl, Loudness::Medium, Some(me), text);
        post(w, e);
    }
    if let Some(text) = w.stories.line(seed, "oneday.quote", &f, year) {
        let e = news(w, Level::Nfl, Loudness::Small, Some(me), text);
        post(w, e);
    }
    if yours {
        super::story::memento(
            w,
            super::story::MementoKind::Photo,
            format!("{name} signs his one-day contract, {year}"),
        );
    }
}

/// Your answer to a one-day contract request: yes signs him, no lets him
/// retire unsigned.
pub fn answer_one_day(w: &mut World, yes: bool) -> Result<String, String> {
    let Some((id, club, _)) = w.stories.careers.offer else { return Err("No one is asking.".into()) };
    if yes {
        sign_one_day(w, id, club as usize, true);
        Ok("He'll sign at the facility next week, with his family there.".into())
    } else {
        w.stories.careers.offer = None;
        w.stories.careers.one_day.push((id, w.year as i16, u16::MAX));
        Ok("You passed. He'll retire without the ceremony.".into())
    }
}

/// The scorers' celebrations: a signature one born for a showman, flags
/// and wiped touchdowns from the engine's calls.
fn celebrations(w: &mut World, ctx: &GameCtx, s: &GameStats) {
    let year = w.today.year();
    let seed = w.options.seed;
    let you = super::seat_team(w);
    for r in s.plays.iter().filter(|r| !r.nullified) {
        for &(kind, side, id, yards, accepted) in &r.penalties {
            if kind != FoulKind::UnsportsmanlikeConduct || !accepted || side > 1 || r.offense != side {
                continue;
            }
            let Some(i) = super::person_index(w, id) else { continue };
            let team = ctx.teams[side];
            let mine = Some(team) == you;
            let f = team_fill(w, team, Fill::new())
                .set("name", w.people[i].name.clone())
                .set("yards", format!("{:.0}", r.yards.max(15.0)));
            if yards < 0.0 {
                if let Some(text) = w.stories.line(seed, "celebrate.taunt_wipe", &f, year) {
                    let e = news(w, ctx.level, Loudness::Medium, Some(team), text);
                    post(w, e);
                }
            } else if (mine || ctx.level == Level::Nfl || ctx.big)
                && let Some(text) = w.stories.line(seed, "celebrate.flag", &f, year)
            {
                let e = news(w, ctx.level, if mine { Loudness::Medium } else { Loudness::Small }, Some(team), text);
                post(w, e);
                if ctx.level == Level::Nfl
                    && year < 2017
                    && w.stories.rng(seed).chance(0.3)
                    && let Some(text) = w.stories.line(seed, "celebrate.rule", &Fill::new(), year)
                {
                    let e = news(w, Level::Nfl, Loudness::Small, None, text);
                    post(w, e);
                }
            }
        }
    }
    // A signature celebration: a showman who keeps scoring gets one people
    // know him by.
    for side in 0..2 {
        for (&id, l) in &s.players[side] {
            let tds = l.rush_td + l.rec_td + l.kr_td + l.pr_td;
            if tds == 0 || w.stories.careers.celebration(id).is_some() {
                continue;
            }
            let Some(i) = super::person_index(w, id) else { continue };
            let ego = w.people[i].player.personality.traits.ego;
            if ego < 14 {
                continue;
            }
            let chance = 0.04 * tds as f64 * (ego as f64 - 12.0) / 4.0;
            if !w.stories.rng(seed).chance(chance.min(0.4)) {
                continue;
            }
            let (first, last) = split_name(&w.people[i].name);
            let f = Fill::new().set("first", first).set("last", last);
            let mut rng = w.stories.rng.take().unwrap_or_else(|| crate::rng::Rng::new(seed ^ 0x5709_1E55));
            let name = lines::word(&mut rng, "words.celebration", &f, year);
            w.stories.rng = Some(rng);
            let Some(name) = name else { continue };
            w.stories.careers.celebrations.push((id, name.clone(), year as i16));
            let team = ctx.teams[side];
            let f =
                team_fill(w, team, Fill::new()).set("name", w.people[i].name.clone()).set("celebration", name.clone());
            if let Some(text) = w.stories.line(seed, "celebrate.signature", &f, year) {
                let mine = Some(team) == you;
                let e = news(w, ctx.level, if mine { Loudness::Medium } else { Loudness::Small }, Some(team), text);
                post(w, e);
            }
            // Two weeks on, the kids on playgrounds and the fans in the stands.
            if ctx.level != Level::HighSchool
                && w.stories.rng(seed).chance(0.35)
                && let Some(text) = w.stories.line(seed, "celebrate.copy", &f, year)
            {
                let e = news(w, ctx.level, Loudness::Small, Some(team), text);
                let when = w.today.add(14);
                w.stories.later(when, e);
            }
        }
    }
    w.stories.careers.celebrations.retain(|c| c.2 as i32 >= year - 25);
}

pub(crate) fn split_name(full: &str) -> (String, String) {
    let parts: Vec<&str> = full.split_whitespace().collect();
    let suffix = |s: &str| matches!(s, "Jr." | "Sr." | "II" | "III" | "IV" | "V");
    let last = parts.iter().rev().find(|s| !suffix(s)).copied().unwrap_or(full);
    (parts.first().copied().unwrap_or(full).to_string(), last.to_string())
}

/// After any other game with plays (college, your high school's).
pub(crate) fn after_game(w: &mut World, ctx: &GameCtx, s: &GameStats) {
    celebrations(w, ctx, s);
}

/// The NFL's 2017 change: celebrations loosened (May 23, 2017).
fn rule_change(w: &mut World, d: Date) {
    if d == Date::ymd(2017, 5, 23) && w.stories.once(62, 2017) {
        let seed = w.options.seed;
        if let Some(text) = w.stories.line(seed, "celebrate.rule", &Fill::new(), 2017) {
            let e = news(w, Level::Nfl, Loudness::Medium, None, text);
            post(w, e);
        }
    }
}

pub(crate) fn day(w: &mut World, d: Date) {
    if w.nfl.is_empty() {
        return;
    }
    rule_change(w, d);
    farewell_tours(w, d);
    one_day(w, d);
}
