//! Earned nicknames (game view A14, items 4-20): players, coaches, units
//! and plays get names from specific, rare moments in the play log that fit
//! who they are. Someone coins each one (a beat writer, the booth, a
//! teammate, rival fans), it spreads by the era's media, it sticks or fades
//! with what he does next, and he reacts by his personality. Mocking names
//! come from failures and can be redeemed; hype names come before the
//! proof; a younger player with the same style can inherit a legend's.
//!
//! Claude's picks throughout (Cain agreed the items, not the numbers; all
//! proposed, not confirmed): the thresholds below, a coining chance of about
//! a third for a fitting moment at the top levels, fading over a quiet
//! season, and spread delays by era (1990s: the next week's papers and
//! a month to national shows; Twitter era from 2008: days; from 2018
//! minutes to a day).

use crate::attributes::Attr;
use crate::calendar::{Date, Level, Loudness};
use crate::game::stats::{GameStats, PlayType, StatLine};
use crate::positions::Position;
use crate::world::World;

use super::lines::Fill;
use super::traditions::key;
use super::{GameCtx, TeamKey, news, post, team_fill};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Who {
    Player(u32),
    Coach(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NickKind {
    /// Earned on the field.
    Earned,
    /// Given before the proof, with pressure attached.
    Hype,
    /// From home, carried up from high school.
    Family,
    Mocking,
    /// "The new ___": a legend's style in a younger man.
    Heir,
    /// Given by you.
    Given,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Coiner {
    BeatWriter,
    Broadcaster,
    Teammate,
    Coach,
    LocalRadio,
    NationalShow,
    Fans,
    RivalFans,
    Family,
    HsPaper,
    You,
}

impl Coiner {
    fn key(self) -> &'static str {
        match self {
            Coiner::BeatWriter => "beat_writer",
            Coiner::Broadcaster => "broadcaster",
            Coiner::Teammate => "teammate",
            Coiner::Coach | Coiner::You => "coach",
            Coiner::LocalRadio => "local_radio",
            Coiner::NationalShow => "national_show",
            Coiner::Fans => "fans",
            Coiner::RivalFans => "rival_fans",
            Coiner::Family => "family",
            Coiner::HsPaper => "hs_paper",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Coiner::BeatWriter => "the beat writer",
            Coiner::Broadcaster => "the booth",
            Coiner::Teammate => "a teammate",
            Coiner::Coach => "his coach",
            Coiner::LocalRadio => "local radio",
            Coiner::NationalShow => "a national show",
            Coiner::Fans => "the fans",
            Coiner::RivalFans => "rival fans",
            Coiner::Family => "his family",
            Coiner::HsPaper => "the local paper",
            Coiner::You => "you",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Reaction {
    None,
    Embraced,
    Trademarked,
    Shrugged,
    Hated,
    /// A mocking name he flipped and owns.
    Owned,
    /// A mocking name that got to him.
    Hurt,
}

impl Reaction {
    pub fn name(self) -> &'static str {
        match self {
            Reaction::None => "",
            Reaction::Embraced => "Embraces it",
            Reaction::Trademarked => "Trademarked it",
            Reaction::Shrugged => "Shrugs it off",
            Reaction::Hated => "Hates it",
            Reaction::Owned => "Owns it",
            Reaction::Hurt => "It stings",
        }
    }
}

/// What a name is about: the moment's theme (an index into `THEMES`).
pub const THEMES: [&str; 23] = [
    "speed",
    "power",
    "hands",
    "ballhawk",
    "rush",
    "hitter",
    "arm",
    "scrambler",
    "clutch",
    "returner",
    "kicker",
    "big_man",
    "iron",
    "brain",
    "swagger",
    "mock.int",
    "mock.fumble",
    "mock.drop",
    "mock.kick",
    "mock.sack",
    "mock.checkdown",
    "mock.bust",
    "mock.rival",
];

fn theme(name: &str) -> u8 {
    THEMES.iter().position(|t| *t == name).unwrap_or(0) as u8
}

fn mocking(t: u8) -> bool {
    THEMES[t as usize].starts_with("mock.")
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Nickname {
    pub who: Who,
    pub name: String,
    pub kind: NickKind,
    pub theme: u8,
    pub coined: Date,
    pub by: Coiner,
    /// The paper, station or show that coined or first printed it.
    pub outlet: String,
    /// The moment, as a clause ("his 212-yard night against Purdue").
    pub moment: String,
    /// The team he was on.
    pub team: Option<TeamKey>,
    /// How well it has stuck, 0-1.
    pub stick: f32,
    /// How far it has spread: 0 coined, 1 local, 2 national, 3 the booth uses it.
    pub stage: u8,
    pub spread: [Option<Date>; 3],
    pub reaction: Reaction,
    /// On shirts (his own trademark or the team store).
    pub merch: bool,
    /// Faded, or replaced (redeemed).
    pub retired: bool,
    /// A mocking name replaced by this one.
    pub replaced: Option<String>,
    /// For an heir, the legend's player id.
    pub heir_of: Option<u32>,
    /// The season it was last reinforced.
    pub last_year: i16,
}

/// A unit's name (item 6).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UnitName {
    pub team: TeamKey,
    /// "defense", "rush", "secondary", "oline", "receivers", "backfield", "offense".
    pub unit: String,
    pub name: String,
    pub from: i16,
    /// The last season it lived up to the name.
    pub last: i16,
    pub why: String,
}

/// A named play (item 7).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NamedPlay {
    pub name: String,
    pub date: Date,
    pub level: Level,
    /// Winner, loser.
    pub teams: [TeamKey; 2],
    pub players: Vec<u32>,
    pub what: String,
    /// The radio call.
    pub call: String,
}

/// A running mark toward a name this season.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Mark {
    pub id: u32,
    pub theme: u8,
    pub n: u8,
    pub year: i16,
    pub moment: String,
}

/// The name a person goes by now: his newest one that stuck.
pub fn current(s: &super::Stories, who: Who) -> Option<&Nickname> {
    s.nicknames.iter().rev().find(|n| n.who == who && !n.retired && (n.stick >= 0.45 || n.stage >= 1))
}

/// Every name a person ever had, oldest first.
pub fn all_of(s: &super::Stories, who: Who) -> Vec<&Nickname> {
    s.nicknames.iter().filter(|n| n.who == who).collect()
}

/// Whether his name sells shirts.
pub fn merch(s: &super::Stories, id: u32) -> bool {
    s.nicknames.iter().any(|n| n.who == Who::Player(id) && n.merch && !n.retired)
}

/// Days from coining to local, national and the booth, by era.
fn spread_days(year: i32) -> [i32; 3] {
    if year < 2008 {
        [6, 28, 42]
    } else if year < 2018 {
        [2, 8, 15]
    } else {
        [1, 3, 7]
    }
}

/// The outlet for a coiner at a level and era (generic names; Claude's pick).
fn outlet(w: &World, by: Coiner, team: Option<TeamKey>) -> String {
    let city = team.map(|t| super::city(w, t)).unwrap_or_default();
    let year = w.year;
    match by {
        Coiner::BeatWriter | Coiner::HsPaper => {
            if city.is_empty() {
                "the local paper".into()
            } else {
                format!("the {city} paper")
            }
        }
        Coiner::LocalRadio => format!("{} sports radio", if city.is_empty() { "local".into() } else { city }),
        Coiner::NationalShow => {
            if year < 2010 {
                "a national highlight show".into()
            } else {
                "a national studio show".into()
            }
        }
        Coiner::Fans | Coiner::RivalFans => {
            if year >= 2008 {
                "social media".into()
            } else {
                "the message boards".into()
            }
        }
        Coiner::Broadcaster => "the TV booth".into(),
        Coiner::Teammate => "the locker room".into(),
        Coiner::Coach | Coiner::You => "a press conference".into(),
        Coiner::Family => "home".into(),
    }
}

/// A plausible reporter's name for a byline, fixed by team and year.
fn writer(w: &World, team: Option<TeamKey>) -> String {
    let k = team.map_or(0, |t| match t {
        TeamKey::Hs(x) | TeamKey::College(x) => x as u64,
        TeamKey::Nfl(x) => 9000 + x as u64,
        TeamKey::Other(x) => 12_000 + x as u64,
    });
    const FIRST: [&str; 24] = [
        "Dave", "Mike", "Tom", "Jen", "Chris", "Pat", "Ross", "Lori", "Greg", "Matt", "Sam", "Erin", "Kevin", "Dana",
        "Rick", "Beth", "Jim", "Kate", "Andy", "Nate", "Mark", "Josh", "Tim", "Ann",
    ];
    const LAST: [&str; 24] = [
        "Kowalski",
        "Brennan",
        "Hughes",
        "Okafor",
        "Lindqvist",
        "Carter",
        "Delgado",
        "Ferris",
        "Whitlock",
        "Nguyen",
        "Patel",
        "Maddox",
        "Sorensen",
        "Albright",
        "Romero",
        "Sullivan",
        "Haskins",
        "McBride",
        "Tran",
        "Ostrowski",
        "Vance",
        "Pruitt",
        "Gallagher",
        "Bishop",
    ];
    let u = key(k, (w.year / 6) as u64);
    let v = key(k ^ 0x77, (w.year / 6) as u64);
    format!("{} {}", FIRST[(u * 24.0) as usize % 24], LAST[(v * 24.0) as usize % 24])
}

/// Makes a name for `who` from a theme's templates; None when every try is
/// a real legend's or already taken at his level.
fn make_name(w: &mut World, key_name: &str, f: &Fill) -> Option<String> {
    let seed = w.options.seed;
    let year = w.year;
    for _ in 0..8 {
        let mut rng = w.stories.rng.take().unwrap_or_else(|| crate::rng::Rng::new(seed ^ 0x5709_1E55));
        let raw = super::lines::word(&mut rng, key_name, f, year);
        w.stories.rng = Some(rng);
        let n = raw?;
        let n = n.split_whitespace().collect::<Vec<_>>().join(" ");
        if n.is_empty() || super::lines::BANK.blocked(&n) {
            continue;
        }
        let taken = w.stories.nicknames.iter().any(|x| !x.retired && x.name.eq_ignore_ascii_case(&n))
            || w.stories.units.iter().any(|x| x.name.eq_ignore_ascii_case(&n))
            || w.stories.plays.iter().any(|x| x.name.eq_ignore_ascii_case(&n));
        if !taken {
            return Some(n);
        }
    }
    None
}

/// The values a player's name templates use.
fn person_fill(w: &World, i: usize, team: Option<TeamKey>) -> Fill {
    let p = &w.people[i];
    let mut parts = p.name.split_whitespace();
    let first = parts.next().unwrap_or("").to_string();
    let last = p
        .name
        .split_whitespace()
        .rfind(|x| !matches!(*x, "Jr." | "Sr." | "II" | "III" | "IV"))
        .unwrap_or("")
        .to_string();
    let initials: String = p.name.split_whitespace().take(2).filter_map(|x| x.chars().next()).collect();
    let town = w.high_schools.get(p.hs).map(|h| h.town.clone()).unwrap_or_default();
    let mut f = Fill::new()
        .set("first", first)
        .set("last", last)
        .set("name", p.name.clone())
        .set("initials", initials)
        .set("town", town);
    if let Some(t) = team {
        f = team_fill(w, t, f);
        if let Some(n) = super::numbers::number_of(w, t, p.player.id) {
            f = f.set("num", n.to_string());
        }
    }
    f
}

/// Whether a theme fits the man: the moment has to match who he is.
fn fits(w: &World, i: usize, th: &str) -> bool {
    let p = &w.people[i].player;
    let a = |x: Attr| p.attributes.get(x);
    let t = p.personality.traits;
    match th {
        "speed" => a(Attr::Speed) >= 80.0,
        "power" => a(Attr::Strength) >= 70.0 || a(Attr::LegDrive) >= 75.0 || p.body.weight >= 225.0,
        "hands" => a(Attr::Catching) >= 75.0,
        "ballhawk" => a(Attr::BallTracking) >= 65.0 || a(Attr::ZoneCoverage) >= 70.0,
        "rush" => a(Attr::PowerRush).max(a(Attr::FinesseRush)) >= 70.0,
        "hitter" => a(Attr::Tackling) >= 70.0,
        "arm" => a(Attr::ArmStrength) >= 80.0,
        "scrambler" => a(Attr::Speed) >= 70.0,
        "big_man" => p.body.weight >= 320.0,
        "brain" => t.leadership >= 14 || t.professionalism >= 15,
        "swagger" => t.ego >= 16,
        _ => true,
    }
}

/// Coins a name and posts how it started.
#[allow(clippy::too_many_arguments)]
fn coin(
    w: &mut World,
    i: usize,
    th: &str,
    kind: NickKind,
    by: Coiner,
    moment: &str,
    team: Option<TeamKey>,
    heir_of: Option<u32>,
) -> bool {
    let id = w.people[i].player.id;
    let seed = w.options.seed;
    let year = w.year;
    let mut f = person_fill(w, i, team);
    if let Some(h) = heir_of {
        let legend = w.stories.nicknames.iter().rev().find(|n| n.who == Who::Player(h)).map(|n| n.name.clone());
        let Some(l) = legend else { return false };
        f = f.set("nick", l.trim_start_matches("the ").trim_start_matches("The ").to_string());
    }
    let key_name = match kind {
        NickKind::Hype => "nick.hype".to_string(),
        NickKind::Family => "nick.family".to_string(),
        NickKind::Heir => "nick.heir".to_string(),
        _ => format!("nick.{th}"),
    };
    let Some(name) = make_name(w, &key_name, &f) else { return false };
    // A positive name redeems a mocking one he carries (item 14).
    let mut replaced = None;
    if !mocking(theme(th))
        && let Some(k) = w
            .stories
            .nicknames
            .iter()
            .rposition(|n| n.who == Who::Player(id) && !n.retired && n.kind == NickKind::Mocking)
    {
        w.stories.nicknames[k].retired = true;
        replaced = Some(w.stories.nicknames[k].name.clone());
    }
    // A new earned name retires his older positive one only when it is stronger.
    let out = outlet(w, by, team);
    let wr = writer(w, team);
    w.stories.nicknames.push(Nickname {
        who: Who::Player(id),
        name: name.clone(),
        kind,
        theme: theme(th),
        coined: w.today,
        by,
        outlet: out.clone(),
        moment: moment.to_string(),
        team,
        stick: if matches!(kind, NickKind::Family) { 0.7 } else { 0.35 },
        stage: 0,
        spread: [None; 3],
        reaction: Reaction::None,
        merch: false,
        retired: false,
        replaced: replaced.clone(),
        heir_of,
        last_year: year as i16,
    });
    // The first line: who called him that.
    let f = f
        .set("nick", name.clone())
        .set("moment", moment.to_string())
        .set("outlet", out)
        .set("writer", wr)
        .set("coach", team.and_then(|t| head_coach_name(w, t)).unwrap_or_default());
    let line_key = format!("nick.coined.{}", by.key());
    let mut text = w.stories.line(seed, &line_key, &f, year);
    if let Some(old) = replaced {
        let f = f.clone().set("old", old);
        if let Some(r) = w.stories.line(seed, "nick.redeem", &f, year) {
            text = Some(r);
        }
    }
    if let Some(text) = text {
        let mine = team.is_some() && team == super::seat_team(w);
        let lvl = team.map_or(Level::College, |t| t.level());
        let e = news(w, lvl, if mine { Loudness::Medium } else { Loudness::Small }, team, text);
        post(w, e);
    }
    true
}

pub(crate) fn head_coach_name(w: &World, t: TeamKey) -> Option<String> {
    let c = match t {
        TeamKey::College(p) => w.programs.get(p as usize)?.head_coach,
        TeamKey::Nfl(n) => w.nfl.get(n as usize)?.head_coach,
        TeamKey::Hs(h) => w.high_schools.get(h as usize)?.coach,
        TeamKey::Other(f) => w.others.teams.get(f as usize)?.head_coach,
    }?;
    if Some(c) == super::seat_coach(w) {
        return w.career.as_ref().map(|x| x.you.name.clone());
    }
    w.coaches.get(c).map(|x| x.name.clone())
}

/// Adds to a theme's mark; returns how many fitting moments he has this season.
fn mark(w: &mut World, id: u32, th: &str, moment: &str) -> u8 {
    let year = w.year as i16;
    let t = theme(th);
    match w.stories.marks.iter_mut().find(|m| m.id == id && m.theme == t && m.year == year) {
        Some(m) => {
            m.n = m.n.saturating_add(1);
            m.moment = moment.to_string();
            m.n
        }
        None => {
            w.stories.marks.push(Mark { id, theme: t, n: 1, year, moment: moment.to_string() });
            1
        }
    }
}

/// The base chance a fitting moment gets a name, by level (the bigger the
/// stage, the more people watching to name it).
fn level_chance(level: Level, big: bool) -> f64 {
    let base = match level {
        Level::Nfl => 0.28,
        Level::College => 0.2,
        _ => 0.15,
    };
    base * if big { 1.5 } else { 1.0 }
}

/// The trigger moments in one player's game line, with the theme and the
/// moment's words. Thresholds are rare at each level (an NFL 400-yard
/// passing day happens a handful of times a season).
fn triggers(l: &StatLine, pos: Position, level: Level, opp: &str) -> Vec<(&'static str, String, bool)> {
    let nfl = level == Level::Nfl;
    let hs = level == Level::HighSchool;
    let mut v = Vec::new();
    let pass_big = if nfl {
        400.0
    } else if hs {
        380.0
    } else {
        450.0
    };
    if l.pass_yds >= pass_big || l.pass_td >= if nfl { 5 } else { 6 } {
        v.push(("arm", format!("his {:.0}-yard, {}-touchdown day against {opp}", l.pass_yds, l.pass_td), false));
    }
    if pos == Position::QB && l.rush_yds + l.scramble_yds >= if nfl { 100.0 } else { 150.0 } {
        v.push(("scrambler", format!("running for {:.0} yards against {opp}", l.rush_yds + l.scramble_yds), false));
    }
    if pos != Position::QB && l.rush_yds >= if nfl { 175.0 } else { 225.0 } {
        let th = if l.forced_missed >= 6 || l.yaco > l.rush_yds * 0.5 { "power" } else { "speed" };
        v.push((th, format!("his {:.0}-yard night against {opp}", l.rush_yds), false));
    }
    if l.rush_long >= 75.0 || l.rec_long >= 80.0 {
        let long = l.rush_long.max(l.rec_long);
        v.push(("speed", format!("the {long:.0}-yard touchdown against {opp}"), false));
    }
    if l.rec >= if nfl { 12 } else { 13 } || l.contested_rec >= 4 || l.rec_yds >= if nfl { 200.0 } else { 240.0 } {
        v.push(("hands", format!("{} catches for {:.0} yards against {opp}", l.rec, l.rec_yds), false));
    }
    if l.ints >= 3 || l.ints >= 2 && nfl {
        v.push(("ballhawk", format!("his {}-interception game against {opp}", l.ints), false));
    }
    if l.sacks >= if nfl { 3.5 } else { 4.0 } {
        v.push(("rush", format!("{} sacks against {opp}", trim(l.sacks)), false));
    }
    if l.tackles + l.assists >= 18 || l.forced_fumbles >= 3 {
        v.push(("hitter", format!("{} tackles against {opp}", l.tackles + l.assists), false));
    }
    if l.kr_td + l.pr_td >= 1 && (l.kr_td + l.pr_td >= 2 || l.pr_yds + l.kr_yds >= 180.0) {
        v.push(("returner", format!("the return touchdown against {opp}"), false));
    }
    if l.fg_long >= 57.0 {
        v.push(("kicker", format!("the {:.0}-yarder against {opp}", l.fg_long), false));
    }
    // Failures (mocking names: item 13).
    if l.int >= if nfl { 4 } else { 5 } {
        v.push(("mock.int", format!("throwing {} interceptions against {opp}", l.int), true));
    }
    if l.fumbles_lost >= 3 {
        v.push(("mock.fumble", format!("losing {} fumbles against {opp}", l.fumbles_lost), true));
    }
    if l.drops >= 4 {
        v.push(("mock.drop", format!("{} drops against {opp}", l.drops), true));
    }
    let missed = l.fg_att.iter().sum::<u32>() - l.fg_made.iter().sum::<u32>();
    if missed >= 3 {
        v.push(("mock.kick", format!("missing {missed} field goals against {opp}"), true));
    }
    if l.sacked >= if nfl { 8 } else { 9 } {
        v.push(("mock.sack", format!("going down {} times against {opp}", l.sacked), true));
    }
    v
}

fn trim(x: f64) -> String {
    if x.fract() == 0.0 { format!("{x:.0}") } else { format!("{x:.1}") }
}

/// After a game: the moments that can name a player, and a play that can
/// earn its own name.
pub(crate) fn after_game(w: &mut World, ctx: &GameCtx, s: &GameStats) {
    let seed = w.options.seed;
    let names = ctx.teams.map(|t| super::short_name(w, t));
    // Who is in this game: player id to person index, from both rosters.
    let mut who: std::collections::HashMap<u32, usize> = Default::default();
    for t in ctx.teams {
        for (i, id) in super::numbers::roster(w, t) {
            who.insert(id, i);
        }
    }
    let [sh, sa] = s.score;
    let winner = if sh >= sa { 0 } else { 1 };
    for side in 0..2 {
        let opp = names[1 - side].clone();
        let team = ctx.teams[side];
        let lines: Vec<(u32, StatLine)> = s.players[side].iter().map(|(&id, l)| (id, l.clone())).collect();
        for (id, l) in lines {
            let Some(&i) = who.get(&id) else { continue };
            let pos = w.people[i].player.position;
            for (th, moment, bad) in triggers(&l, pos, ctx.level, &opp) {
                // Mocking names mostly come after losses.
                if bad && side == winner && !w.stories.rng(seed).chance(0.25) {
                    continue;
                }
                if !bad && !fits(w, i, th) {
                    continue;
                }
                let n = mark(w, id, th, &moment);
                // Reinforce a name he already has for this.
                if let Some(k) = w
                    .stories
                    .nicknames
                    .iter()
                    .rposition(|x| x.who == Who::Player(id) && !x.retired && x.theme == theme(th))
                {
                    let x = &mut w.stories.nicknames[k];
                    x.stick = (x.stick + 0.15).min(1.0);
                    x.last_year = w.year as i16;
                    continue;
                }
                let has_good = current(&w.stories, Who::Player(id)).is_some_and(|x| x.kind != NickKind::Mocking);
                if bad && has_good {
                    continue;
                }
                if !bad && has_good && n < 3 {
                    continue;
                }
                let chance =
                    level_chance(ctx.level, ctx.big) * (0.6 + 0.4 * n.min(3) as f64) * if bad { 0.5 } else { 1.0 };
                if !w.stories.rng(seed).chance(chance) {
                    continue;
                }
                let by = pick_coiner(w, ctx.level, bad, side != winner && ctx.rivalry);
                let kind = if bad { NickKind::Mocking } else { NickKind::Earned };
                if coin(w, i, th, kind, by, &moment, Some(team), None) {
                    break;
                }
            }
        }
    }
    named_play(w, ctx, s, &who);
    // The swagger name: a showman's big day in a big game.
    if ctx.big {
        for side in 0..2 {
            let best = s.players[side]
                .iter()
                .map(|(&id, l)| (id, l.pass_td + l.rush_td + l.rec_td + l.ints + (l.sacks as u32)))
                .max_by_key(|x| x.1);
            if let Some((id, n)) = best
                && n >= 3
                && let Some(&i) = who.get(&id)
                && fits(w, i, "swagger")
                && current(&w.stories, Who::Player(id)).is_none()
                && w.stories.rng(seed).chance(0.15)
            {
                let moment = format!("his day against {}", names[1 - side]);
                coin(w, i, "swagger", NickKind::Earned, Coiner::Broadcaster, &moment, Some(ctx.teams[side]), None);
            }
        }
    }
    // Rival fans' name for the man who beat them (item 19).
    if ctx.rivalry {
        let side = winner;
        let star = s.players[side]
            .iter()
            .map(|(&id, l)| {
                (
                    id,
                    l.pass_yds / 25.0
                        + l.rush_yds / 10.0
                        + l.rec_yds / 10.0
                        + 6.0 * (l.pass_td + l.rush_td + l.rec_td) as f64,
                )
            })
            .max_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((id, v)) = star
            && v >= 25.0
            && let Some(&i) = who.get(&id)
            && !w.stories.nicknames.iter().any(|x| x.who == Who::Player(id) && x.theme == theme("mock.rival"))
            && w.stories.rng(seed).chance(0.12)
        {
            let moment = format!("beating {} again", names[1 - side]);
            let team = ctx.teams[side];
            let f = person_fill(w, i, Some(ctx.teams[1 - side]));
            if let Some(name) = make_name(w, "nick.mock.rival", &f) {
                let by = Coiner::RivalFans;
                w.stories.nicknames.push(Nickname {
                    who: Who::Player(id),
                    name: name.clone(),
                    kind: NickKind::Mocking,
                    theme: theme("mock.rival"),
                    coined: w.today,
                    by,
                    outlet: outlet(w, by, Some(ctx.teams[1 - side])),
                    moment: moment.clone(),
                    team: Some(team),
                    // Rival fans' names live in their own corner, never his main name.
                    stick: 0.3,
                    stage: 0,
                    spread: [None; 3],
                    reaction: Reaction::None,
                    merch: false,
                    retired: true,
                    replaced: None,
                    heir_of: None,
                    last_year: w.year as i16,
                });
                let f =
                    f.set("nick", name).set("moment", moment).set("outlet", outlet(w, by, Some(ctx.teams[1 - side])));
                if let Some(text) = w.stories.line(seed, "nick.coined.rival_fans", &f, w.year) {
                    let e = news(w, ctx.level, Loudness::Small, Some(ctx.teams[1 - side]), text);
                    post(w, e);
                }
            }
        }
    }
    let _ = sa;
}

fn pick_coiner(w: &mut World, level: Level, bad: bool, rival: bool) -> Coiner {
    let seed = w.options.seed;
    let u = w.stories.rng(seed).next();
    let social = w.year >= 2008;
    if bad {
        return if rival {
            Coiner::RivalFans
        } else if u < 0.45 {
            if social { Coiner::Fans } else { Coiner::LocalRadio }
        } else if u < 0.75 {
            Coiner::NationalShow
        } else {
            Coiner::LocalRadio
        };
    }
    match level {
        Level::HighSchool => {
            if u < 0.6 {
                Coiner::HsPaper
            } else {
                Coiner::Teammate
            }
        }
        Level::Nfl => {
            if u < 0.3 {
                Coiner::Broadcaster
            } else if u < 0.5 {
                Coiner::Teammate
            } else if u < 0.7 {
                Coiner::BeatWriter
            } else if u < 0.85 {
                Coiner::NationalShow
            } else if social {
                Coiner::Fans
            } else {
                Coiner::LocalRadio
            }
        }
        _ => {
            if u < 0.35 {
                Coiner::BeatWriter
            } else if u < 0.55 {
                Coiner::Broadcaster
            } else if u < 0.75 {
                Coiner::Teammate
            } else if u < 0.85 {
                Coiner::Coach
            } else if social {
                Coiner::Fans
            } else {
                Coiner::LocalRadio
            }
        }
    }
}

/// A play that wins a big game at the end earns its own name (item 7): the
/// last scoring play that turned a loss into a win in the final minute, or
/// a long touchdown on the game's last snap.
fn named_play(w: &mut World, ctx: &GameCtx, s: &GameStats, who: &std::collections::HashMap<u32, usize>) {
    if !ctx.big && ctx.level != Level::Nfl {
        return;
    }
    let [sh, sa] = s.score;
    if sh == sa {
        return;
    }
    let winner = if sh > sa { 0 } else { 1 };
    // The winning score: the last play on which the winner took the lead for good.
    let Some(p) = s.plays.iter().rev().find(|p| {
        !p.nullified
            && (p.touchdown || p.kick_good && p.kind == PlayType::FieldGoal || p.defense_td)
            && p.quarter >= 4
            && p.clock <= 60.0
    }) else {
        return;
    };
    let scorer_side = if p.defense_td { 1 - p.offense } else { p.offense };
    if scorer_side != winner {
        return;
    }
    let before = p.score;
    let behind_before = before[winner] <= before[1 - winner];
    if !behind_before {
        return;
    }
    let big = ctx.postseason || ctx.rivalry || ctx.level == Level::Nfl && p.wp_before < 0.25;
    let chance = if big { 0.55 } else { 0.18 };
    let seed = w.options.seed;
    if !w.stories.rng(seed).chance(chance) {
        return;
    }
    let (kind, main, second) = if p.kind == PlayType::FieldGoal {
        ("kick", p.kicker, None)
    } else if p.defense_td {
        ("pick", p.interceptor.or(p.forced_fumble), None)
    } else if p.complete && p.yards >= 40.0 && p.clock <= 5.0 {
        ("hail", p.target, p.passer)
    } else if p.complete {
        ("catch", p.target, p.passer)
    } else if p.kind == PlayType::Kickoff || p.kind == PlayType::Punt {
        ("return", p.returner, None)
    } else {
        ("run", p.carrier, None)
    };
    let Some(main) = main else { return };
    let Some(&mi) = who.get(&main) else { return };
    let team = ctx.teams[winner];
    let opp = ctx.teams[1 - winner];
    let mut f = person_fill(w, mi, Some(team));
    if let Some(sid) = second
        && let Some(&si) = who.get(&sid)
    {
        let last2 = w.people[si].name.split_whitespace().last().unwrap_or("").to_string();
        f = f.set("last2", last2);
    }
    let month = [
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
    ][(w.today.month() as usize).saturating_sub(1).min(11)];
    f = f.set("opp", super::short_name(w, opp)).set("month", month).set("site", super::city(w, ctx.teams[0]));
    if let Some(m) = super::mascot(w, opp) {
        f = f.set("opp_mascot", m);
    }
    let Some(name) = make_name(w, &format!("nick.play.{kind}"), &f) else { return };
    let yds = if kind == "kick" { p.kick_distance.max(p.yards) } else { p.yards.abs().max(p.return_yards) };
    let what = format!(
        "{} {} {:.0} {} with {} left to beat {} {}-{}",
        w.people[mi].name,
        match kind {
            "kick" => "kicked it from",
            "pick" => "took it back",
            "return" => "returned it",
            "run" => "ran it in from",
            _ => "caught it from",
        },
        yds,
        crate::world::folk::noun(yds.round_ties_even() as i64, "yard"),
        clock_words(p.clock),
        super::team_in_text(w, opp),
        sh.max(sa),
        sh.min(sa)
    );
    let call_key = match kind {
        "kick" => "call.kick_good",
        "pick" => "call.pick",
        "run" | "return" => "call.td_run",
        _ => "call.walkoff",
    };
    let booth = w.nickname_of(main).map(str::to_string).unwrap_or_else(|| f.get("last").unwrap_or("").to_string());
    let cf = team_fill(w, team, Fill::new())
        .set("nick", booth)
        .set("yards", format!("{:.0}", p.yards.abs()))
        .set("opp", super::short_name(w, opp));
    let call = w.stories.line(seed, call_key, &cf, w.year).unwrap_or_default();
    let players: Vec<u32> = [Some(main), second].into_iter().flatten().collect();
    w.stories.plays.push(NamedPlay {
        name: name.clone(),
        date: w.today,
        level: ctx.level,
        teams: [team, opp],
        players,
        what: what.clone(),
        call: call.clone(),
    });
    let text = format!(
        "They're already calling it {name}: {what}.{}",
        if call.is_empty() { String::new() } else { format!(" The call: {call}") }
    );
    let mine = super::seat_team(w).is_some_and(|m| m == team || m == opp);
    let e =
        news(w, ctx.level, if mine || ctx.postseason { Loudness::Medium } else { Loudness::Small }, Some(team), text);
    post(w, e);
    if super::seat_team(w) == Some(team) {
        super::story::memento(w, super::story::MementoKind::Photo, format!("{name}, {}", w.today.year()));
    }
}

fn clock_words(sec: f64) -> String {
    if sec <= 0.5 {
        "no time".into()
    } else if sec < 60.0 {
        format!("{sec:.0} {}", crate::world::folk::noun(sec.round_ties_even() as i64, "second"))
    } else {
        format!("{}:{:02}", (sec / 60.0) as u32, (sec % 60.0) as u32)
    }
}

/// Spreading, reactions and merch (items 11, 15, 18), once a day; the
/// yearly passes (hype and family names, units, coaches, heirs, fading).
pub(crate) fn day(w: &mut World, d: Date) {
    spread(w, d);
    let y = w.dates.clone();
    // Signing day: hype names for the blue-chips, family names carried up.
    if d == y.signing_day && w.stories.once(50, w.year) {
        hype_and_family(w);
    }
    // After the college regular season and the NFL's: units and coaches.
    if d == y.fbs_regular_end.add(2) && w.stories.once(51, w.year) {
        units(w, Level::College);
        coaches(w, Level::College);
    }
    if y.nfl_weeks.last().is_some_and(|&l| d == l.add(2)) && w.stories.once(52, w.year) {
        units(w, Level::Nfl);
        coaches(w, Level::Nfl);
    }
    // Camp: heirs; and the start of the football year: fading.
    if d == y.fall_camp && w.stories.once(53, w.year) {
        heirs(w);
    }
    if d == y.start && w.stories.once(54, w.year) {
        fade(w);
    }
}

fn spread(w: &mut World, d: Date) {
    let year = w.year;
    let days = spread_days(year);
    let seed = w.options.seed;
    let mine = super::seat_team(w);
    let n = w.stories.nicknames.len();
    for k in 0..n {
        let x = &w.stories.nicknames[k];
        if x.retired || x.stage >= 3 || x.kind == NickKind::Family {
            continue;
        }
        let next = x.stage as usize;
        if d < x.coined.add(days[next]) {
            continue;
        }
        // National shows and the booth need a name people know.
        let Who::Player(id) = x.who else {
            w.stories.nicknames[k].stage += 1;
            continue;
        };
        let Some(i) = super::person_index(w, id) else {
            w.stories.nicknames[k].stage = 3;
            continue;
        };
        let level = x.team.map_or(Level::College, |t| t.level());
        let famous = matches!(level, Level::Nfl) || w.people[i].stars >= 4 || x.stick >= 0.6;
        if next >= 1 && !famous || level == Level::HighSchool && next >= 1 {
            // It stays local.
            w.stories.nicknames[k].stage = 3.min(w.stories.nicknames[k].stage.max(1));
            if next >= 1 {
                w.stories.nicknames[k].stage = 3;
            }
            continue;
        }
        w.stories.nicknames[k].stage += 1;
        w.stories.nicknames[k].spread[next] = Some(d);
        w.stories.nicknames[k].stick = (w.stories.nicknames[k].stick + 0.1).min(1.0);
        let x = w.stories.nicknames[k].clone();
        let ours = x.team.is_some() && x.team == mine;
        let f = person_fill(w, i, x.team)
            .set("nick", x.name.clone())
            .set("outlet", x.outlet.clone())
            .set("days", d.0.saturating_sub(x.coined.0).to_string());
        let key_name = ["nick.spread.local", "nick.spread.national", "nick.spread.broadcast"][next];
        // Only your players' and the stars' spreads make the news.
        if (ours || next >= 1)
            && let Some(text) = w.stories.line(seed, key_name, &f, year)
        {
            let e = news(w, level, if ours { Loudness::Medium } else { Loudness::Small }, x.team, text);
            post(w, e);
        }
        // His reaction, once it is out locally.
        if next == 0 {
            react(w, k, i);
        }
    }
}

/// How he takes it (item 15): a showman with an eye for money trademarks a
/// good name, most embrace it, the private ones shrug or hate it; a mocking
/// name stings a thin skin, and a big personality owns it.
fn react(w: &mut World, k: usize, i: usize) {
    let seed = w.options.seed;
    let year = w.year;
    let x = w.stories.nicknames[k].clone();
    let t = w.people[i].player.personality.traits;
    let u = w.stories.rng(seed).next();
    let r = if x.kind == NickKind::Mocking {
        if t.ego >= 14 && t.leadership >= 12 && u < 0.6 {
            Reaction::Owned
        } else if t.temper as i32 + t.ego as i32 - t.professionalism as i32 >= 12 || u < 0.25 {
            Reaction::Hurt
        } else {
            Reaction::Shrugged
        }
    } else if t.ego >= 15 && t.greed >= 12 && u < 0.5 {
        Reaction::Trademarked
    } else if t.ego <= 7 && t.professionalism >= 14 && u < 0.5 {
        if u < 0.2 { Reaction::Hated } else { Reaction::Shrugged }
    } else {
        Reaction::Embraced
    };
    w.stories.nicknames[k].reaction = r;
    if matches!(r, Reaction::Trademarked | Reaction::Owned) {
        w.stories.nicknames[k].merch = true;
    }
    match r {
        Reaction::Hurt => w.people[i].mood = (w.people[i].mood - 0.06).max(-1.0),
        Reaction::Embraced | Reaction::Trademarked => w.people[i].mood = (w.people[i].mood + 0.03).min(1.0),
        Reaction::Hated => {
            // He asks people to stop; it fades faster.
            w.stories.nicknames[k].stick *= 0.7;
        }
        _ => {}
    }
    let key_name = match r {
        Reaction::Embraced => "nick.react.embrace",
        Reaction::Trademarked => "nick.react.trademark",
        Reaction::Shrugged => "nick.react.shrug",
        Reaction::Hated => "nick.react.hate",
        Reaction::Owned => "nick.react.mock_own",
        Reaction::Hurt => "nick.react.mock_hurt",
        Reaction::None => return,
    };
    let ours = x.team.is_some() && x.team == super::seat_team(w);
    // Only notable reactions reach the news: yours, trademarks, a hurt star.
    if !(ours || matches!(r, Reaction::Trademarked | Reaction::Owned) || w.people[i].stars >= 5) {
        return;
    }
    let f = person_fill(w, i, x.team).set("nick", x.name.clone());
    if let Some(text) = w.stories.line(seed, key_name, &f, year) {
        let lvl = x.team.map_or(Level::College, |t| t.level());
        let e = news(w, lvl, if ours { Loudness::Medium } else { Loudness::Small }, x.team, text);
        post(w, e);
    }
    if matches!(r, Reaction::Trademarked | Reaction::Owned)
        && let Some(text) = w.stories.line(seed, "nick.merch", &f, year)
    {
        let lvl = x.team.map_or(Level::College, |t| t.level());
        let e = news(w, lvl, Loudness::Small, x.team, text);
        w.stories.later(w.today.add(10), e);
    }
}

/// Hype names (item 16) for the class's biggest names before they've done
/// anything at the next level, and family names (settled by real life)
/// that come up with the blue-chips.
fn hype_and_family(w: &mut World) {
    let seed = w.options.seed;
    let class = w.year;
    let cands: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            let p = &w.people[i];
            p.class_of == class && p.stars >= 4 && p.stage == crate::world::Stage::HighSchool
        })
        .collect();
    for i in cands {
        let id = w.people[i].player.id;
        if w.stories.nicknames.iter().any(|x| x.who == Who::Player(id)) {
            continue;
        }
        let p = &w.people[i];
        let team = p.cal.signed.as_ref().map(|_| ()).and(p.cal.commit.map(|c| TeamKey::College(c.0 as u32)));
        let ego = p.personality_ego();
        let stars = p.stars;
        if stars >= 5 && ego >= 12 && w.stories.rng(seed).chance(0.22) {
            let moment = format!("his recruitment, before he had played a college snap{}", "");
            coin(w, i, "hype", NickKind::Hype, Coiner::HsPaper, &moment, team, None);
        } else if w.stories.rng(seed).chance(0.05) {
            let moment = "growing up".to_string();
            coin(w, i, "family", NickKind::Family, Coiner::Family, &moment, team, None);
        }
    }
    // A hype name that didn't pan out turns (item 16's pressure): a second
    // year with little to show for it.
    let bust: Vec<(usize, u32)> = w
        .stories
        .nicknames
        .iter()
        .enumerate()
        .filter(|(_, x)| x.kind == NickKind::Hype && !x.retired && x.coined.year() <= class - 3)
        .filter_map(|(k, x)| if let Who::Player(id) = x.who { Some((k, id)) } else { None })
        .collect();
    for (k, id) in bust {
        let Some(i) = super::person_index(w, id) else { continue };
        let done = w.stats.honors.get(&id).is_some_and(|h| !h.all_america.is_empty() || h.all_conference > 0);
        if done {
            w.stories.nicknames[k].kind = NickKind::Earned;
            continue;
        }
        if w.stories.rng(seed).chance(0.35) {
            let team = super::numbers::team_of(w, i);
            let moment = "three years of not living up to it".to_string();
            if coin(w, i, "mock.bust", NickKind::Mocking, Coiner::Fans, &moment, team, None) {
                w.stories.nicknames[k].retired = true;
            }
        }
    }
}

impl crate::world::Person {
    fn personality_ego(&self) -> u8 {
        self.player.personality.traits.ego
    }
}

/// Unit names (item 6): a unit that dominates its level for a season
/// earns a name; it lives while it keeps playing like it.
fn units(w: &mut World, level: Level) {
    let seed = w.options.seed;
    let year = w.year;
    // Each team's season numbers: points for and against per game, sacks,
    // takeaways.
    let mut teams: Vec<(TeamKey, f64, f64, f64, f64)> = Vec::new();
    match level {
        Level::College => {
            let Some(cs) = w.college.as_ref() else { return };
            for p in w.programs.iter().filter(|p| p.division == crate::world::data::Division::FBS) {
                let Some(r) = cs.results.get(p.id) else { continue };
                let g = r.totals.games.max(1) as f64;
                if r.totals.games < 8 {
                    continue;
                }
                teams.push((
                    TeamKey::College(p.id as u32),
                    r.totals.points_for as f64 / g,
                    r.totals.points_against as f64 / g,
                    r.totals.sacks as f64 / g,
                    r.totals.takeaways as f64 / g,
                ));
            }
        }
        _ => {
            for (t, v) in w.stories.nfl.totals.iter().enumerate() {
                if v[4] < 10.0 {
                    continue;
                }
                let g = v[4];
                teams.push((TeamKey::Nfl(t as u16), v[0] / g, v[1] / g, v[2] / g, v[3] / g));
            }
        }
    }
    if teams.len() < 10 {
        return;
    }
    let rank = |k: usize, low: bool, teams: &[(TeamKey, f64, f64, f64, f64)]| {
        let mut v: Vec<(f64, TeamKey)> = teams
            .iter()
            .map(|x| {
                let s = [x.1, x.2, x.3, x.4][k];
                (if low { -s } else { s }, x.0)
            })
            .collect();
        v.sort_by(|a, b| b.0.total_cmp(&a.0));
        v.into_iter().map(|x| x.1).collect::<Vec<_>>()
    };
    let best_d = rank(1, true, &teams);
    let best_o = rank(0, false, &teams);
    let best_rush = rank(2, false, &teams);
    let best_take = rank(3, false, &teams);
    let mut cands: Vec<(TeamKey, &str, String)> = Vec::new();
    if let Some(&t) = best_d.first() {
        cands.push((t, "defense", "the fewest points allowed in the country".into()));
    }
    if let Some(&t) = best_o.first() {
        cands.push((t, "offense", "the most points in the country".into()));
    }
    if let Some(&t) = best_rush.first() {
        cands.push((t, "rush", "the most sacks in the country".into()));
    }
    if let Some(&t) = best_take.first() {
        cands.push((t, "secondary", "more takeaways than anyone".into()));
    }
    for (t, unit, why) in cands {
        // Lived up to an old name: it lives on.
        if let Some(u) = w.stories.units.iter_mut().find(|u| u.team == t && u.unit == unit) {
            u.last = year as i16;
            continue;
        }
        if !w.stories.rng(seed).chance(0.45) {
            continue;
        }
        let mut f = team_fill(w, t, Fill::new());
        if let Some(c) = head_coach_name(w, t) {
            f = f.set("coach_last", c.split_whitespace().last().unwrap_or("").to_string());
        }
        let Some(name) = make_name(w, &format!("nick.unit.{unit}"), &f) else { continue };
        w.stories.units.push(UnitName {
            team: t,
            unit: unit.to_string(),
            name: name.clone(),
            from: year as i16,
            last: year as i16,
            why: why.clone(),
        });
        let text = format!("{} have a name now: {name}, after {why}.", unit_words(w, t, unit));
        let mine = super::seat_team(w) == Some(t);
        let e = news(w, level, if mine { Loudness::Medium } else { Loudness::Small }, Some(t), text);
        post(w, e);
    }
}

fn unit_words(w: &World, t: TeamKey, unit: &str) -> String {
    let team = super::team_name(w, t);
    let poss = if team.ends_with('s') { format!("{team}'") } else { format!("{team}'s") };
    let u = match unit {
        "defense" => "defense",
        "offense" => "offense",
        "rush" => "pass rushers",
        "secondary" => "secondary",
        "oline" => "offensive line",
        "receivers" => "receivers",
        _ => "backfield",
    };
    let s = format!("{poss} {u}");
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or(s)
}

/// Coaches' names (item 17), from what their seasons say about them.
fn coaches(w: &mut World, level: Level) {
    let seed = w.options.seed;
    let year = w.year;
    let teams: Vec<TeamKey> = match level {
        Level::Nfl => (0..w.nfl.len()).map(|t| TeamKey::Nfl(t as u16)).collect(),
        _ => w
            .programs
            .iter()
            .filter(|p| p.division == crate::world::data::Division::FBS)
            .map(|p| TeamKey::College(p.id as u32))
            .collect(),
    };
    for t in teams {
        let c = match t {
            TeamKey::College(p) => w.programs[p as usize].head_coach,
            TeamKey::Nfl(n) => w.nfl[n as usize].head_coach,
            TeamKey::Other(f) => w.others.teams.get(f as usize).and_then(|t| t.head_coach),
            TeamKey::Hs(_) => None,
        };
        let Some(c) = c else { continue };
        if current(&w.stories, Who::Coach(c as u32)).is_some() {
            continue;
        }
        let (wins, losses) = w.stint_record(c);
        let you = Some(c) == super::seat_coach(w);
        let style = if you { w.career.as_ref().map(|x| x.you.style) } else { w.coaches.get(c).map(|x| x.style()) };
        let Some(style) = style else { continue };
        let theme = if (you && w.stories.nfl.go_rate(year).is_some_and(|r| r >= 0.6)) || style.risk >= 17.0 {
            "aggressive"
        } else if style.risk <= 4.0 && losses > wins {
            "conservative"
        } else if wins >= 10 && w.stories.story.big_wins(t, year) >= 2 {
            "bigwin"
        } else if w.stories.story.big_losses(t, year) >= 3 {
            "chokes"
        } else if style.freedom <= 4.0 && wins > losses {
            "grinder"
        } else {
            continue;
        };
        let chance = if you { 0.35 } else { 0.08 };
        if !w.stories.rng(seed).chance(chance) {
            continue;
        }
        let name =
            if you { w.career.as_ref().map(|x| x.you.name.clone()) } else { w.coaches.get(c).map(|x| x.name.clone()) };
        let Some(full) = name else { continue };
        let first = full.split_whitespace().next().unwrap_or("").to_string();
        let last = full.split_whitespace().last().unwrap_or("").to_string();
        let f = team_fill(w, t, Fill::new()).set("first", first).set("last", last).set("name", full.clone());
        let Some(nick) = make_name(w, &format!("nick.coach.{theme}"), &f) else { continue };
        let mocking = matches!(theme, "conservative" | "chokes");
        let by =
            if mocking { if year >= 2008 { Coiner::Fans } else { Coiner::LocalRadio } } else { Coiner::BeatWriter };
        w.stories.nicknames.push(Nickname {
            who: Who::Coach(c as u32),
            name: nick.clone(),
            kind: if mocking { NickKind::Mocking } else { NickKind::Earned },
            theme: 0,
            coined: w.today,
            by,
            outlet: outlet(w, by, Some(t)),
            moment: format!("the {year} season"),
            team: Some(t),
            stick: 0.6,
            stage: 1,
            spread: [Some(w.today), None, None],
            reaction: Reaction::None,
            merch: false,
            retired: false,
            replaced: None,
            heir_of: None,
            last_year: year as i16,
        });
        let text = if you {
            format!(
                "They have a name for you now: {nick}. It started with {} after the {year} season.",
                outlet(w, by, Some(t))
            )
        } else {
            format!("{full} has a nickname that's sticking: {nick}.")
        };
        let e = news(w, level, if you { Loudness::Medium } else { Loudness::Small }, Some(t), text);
        post(w, e);
        if you {
            super::story::log(w, super::story::EntryKind::Nickname, format!("You became {nick}."), Some(t));
        }
    }
}

/// Heirs (item 20): a young player at a legend's school and position, with
/// the same gift, gets "the new ___".
fn heirs(w: &mut World) {
    let seed = w.options.seed;
    let year = w.year;
    let legends: Vec<(u32, String, Option<TeamKey>, u8)> = w
        .stories
        .nicknames
        .iter()
        .filter(|x| x.kind == NickKind::Earned && x.stage >= 2 && x.stick >= 0.7)
        .filter_map(|x| if let Who::Player(id) = x.who { Some((id, x.name.clone(), x.team, x.theme)) } else { None })
        .collect();
    for (lid, _, team, th) in legends {
        let Some(team) = team else { continue };
        // The legend has moved on.
        if super::person_index(w, lid).and_then(|i| super::numbers::team_of(w, i)) == Some(team) {
            continue;
        }
        if w.stories.nicknames.iter().any(|x| x.heir_of == Some(lid)) {
            continue;
        }
        let pos = w.stats.honors.get(&lid).and_then(|h| h.pos);
        let young = super::numbers::roster(w, team).into_iter().find(|&(i, id)| {
            let p = &w.people[i];
            Some(p.player.position) == pos
                && p.stars >= 4
                && p.college_seasons <= 1
                && current(&w.stories, Who::Player(id)).is_none()
                && fits(w, i, THEMES[th as usize])
        });
        let Some((i, _)) = young else { continue };
        if !w.stories.rng(seed).chance(0.3) {
            continue;
        }
        let moment = "his first camp, where he looked like someone they remember".to_string();
        coin(w, i, THEMES[th as usize], NickKind::Heir, Coiner::Fans, &moment, Some(team), Some(lid));
    }
    let _ = year;
}

/// A new football year: names nobody kept using fade (item 5); marks reset.
fn fade(w: &mut World) {
    let year = w.year as i16;
    for x in w.stories.nicknames.iter_mut() {
        if x.retired || matches!(x.kind, NickKind::Family) {
            continue;
        }
        if x.last_year < year - 1 {
            x.stick -= 0.25;
            if x.stick < 0.2 && x.stage < 2 {
                x.retired = true;
            }
        }
    }
    w.stories.marks.retain(|m| m.year >= year - 1);
}

/// Your nickname for a player (item 9): it catches on, flops, or gets eye
/// rolls. Your name in the media, the fit and his own buy-in decide.
pub fn give(w: &mut World, id: u32, name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.len() > 32 {
        return Err("A nickname needs one to 32 characters.".into());
    }
    if super::lines::BANK.blocked(name) {
        return Err("That one belongs to a legend. Pick your own.".into());
    }
    let i = super::person_index(w, id).ok_or("No such player.")?;
    let team = super::numbers::team_of(w, i);
    if team.is_none() || team != super::seat_team(w) {
        return Err("You can only name your own players.".into());
    }
    let seed = w.options.seed;
    let year = w.year;
    let rep = w.career.as_ref().map_or(0.4, |c| c.you.reputation);
    let t = w.people[i].player.personality.traits;
    let buy_in = 0.5 + 0.03 * (t.ego as f64 - 10.0);
    let chance = (0.25 + 0.4 * rep + 0.2 * buy_in).clamp(0.1, 0.85);
    let u = w.stories.rng(seed).next();
    let you = w.career.as_ref().map(|c| c.you.name.clone()).unwrap_or_default();
    let f = person_fill(w, i, team).set("nick", name.to_string()).set("coach", you);
    let (key_name, stuck) = if u < chance {
        ("nick.given.accept", true)
    } else if u < chance + 0.25 {
        ("nick.given.eye_roll", false)
    } else {
        ("nick.given.flop", false)
    };
    w.stories.nicknames.push(Nickname {
        who: Who::Player(id),
        name: name.to_string(),
        kind: NickKind::Given,
        theme: 0,
        coined: w.today,
        by: Coiner::You,
        outlet: "a press conference".into(),
        moment: "you said it at the podium".into(),
        team,
        stick: if stuck { 0.6 } else { 0.15 },
        stage: if stuck { 1 } else { 0 },
        spread: [if stuck { Some(w.today) } else { None }, None, None],
        reaction: Reaction::None,
        merch: false,
        retired: !stuck,
        replaced: None,
        heir_of: None,
        last_year: year as i16,
    });
    let text = w
        .stories
        .line(seed, key_name, &f, year)
        .unwrap_or_else(|| format!("You called {} \"{name}\".", w.people[i].name));
    let e = news(w, team.map_or(Level::College, |t| t.level()), Loudness::Medium, team, text.clone());
    post(w, e);
    Ok(text)
}

/// Three names that would fit a player (for the "give a nickname" dialog).
pub fn suggestions(w: &mut World, id: u32) -> Vec<String> {
    let Some(i) = super::person_index(w, id) else { return Vec::new() };
    let team = super::numbers::team_of(w, i);
    let f = person_fill(w, i, team);
    let pos = w.people[i].player.position;
    let themes: Vec<&str> = match pos {
        Position::QB => vec!["arm", "brain", "scrambler", "clutch"],
        Position::RB => vec!["power", "speed", "iron"],
        Position::WR | Position::TE => vec!["hands", "speed", "clutch"],
        Position::OT | Position::IOL | Position::DT => vec!["big_man", "iron", "power"],
        Position::EDGE | Position::LB => vec!["rush", "hitter", "swagger"],
        Position::CB | Position::S => vec!["ballhawk", "hitter", "speed"],
        _ => vec!["kicker", "clutch"],
    };
    let mut out = Vec::new();
    for th in themes.iter().cycle().take(8) {
        if out.len() >= 3 {
            break;
        }
        if let Some(n) = make_name(w, &format!("nick.{th}"), &f)
            && !out.contains(&n)
        {
            out.push(n);
        }
    }
    out
}
