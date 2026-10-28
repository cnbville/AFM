//! Tips from people you know (scouting update SK35, coaching C14): high
//! school and youth coaches, trainers, 7-on-7 and camp men, alumni, area
//! scouts and agents send tape on a player they swear by. How right a tip
//! is follows the sender's own eye; how straight it is follows his agenda
//! (his own player, his paying client, his camp's name) and how warm he is
//! with you. Each tip settles once the kid's road shows, and every source
//! builds a track record you can read on the Contacts screen.
//!
//! A camp or 7v7 man (C14) adds what only his camp saw, since college
//! coaches may not attend 7-on-7 events: the laser-timed forty, the real
//! height, how the kid took coaching, his real top three, and whether
//! anyone's on him yet.
//!
//! The AI's staffs find kids through their own coaches' networks (the
//! network term of a recruit's visibility, `recruiting.rs`), which stands in
//! for the tips their contacts send. Sizes are Claude's picks (proposed, not
//! confirmed).

use super::network::Other;
use super::{Team, coach_link, player_link};
use crate::body::{Drills, body_now, height_text};
use crate::calendar::Date;
use crate::world::former::Life;
use crate::world::scouting::Kind as LookKind;
use crate::world::season::key_normal;
use crate::world::{Division, Stage, World};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Tips {
    pub list: Vec<Tip>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Tip {
    pub date: Date,
    /// Who sent it, as the contact list keeps him (`coach`, or `other`).
    pub coach: usize,
    pub other: Option<Other>,
    pub player: u32,
    /// What he said: 0 worth a look, 1 can play for you, 2 a sure thing.
    pub claim: u8,
    /// The seat it was for: true for an NFL club, else a college of
    /// `level` (the division's rank, 1 Division III to 5 FBS).
    pub nfl: bool,
    pub level: u8,
    /// What only his camp saw (C14), in words.
    pub lines: Vec<String>,
    pub settled: Option<bool>,
}

pub const CLAIMS: [&str; 3] = ["He's worth a look", "He can play for you", "He's a sure thing"];

/// The kind of source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Src {
    /// A high school coach, about his own players.
    Hs(usize),
    /// A college coach, about his own players (to an NFL seat).
    College(usize),
    /// A 7-on-7 or camp man (C14).
    Camp,
    /// A private trainer, about his paying clients.
    Trainer,
    /// A youth or club coach the world keeps no job for.
    Youth,
    /// A former player of your program, about a kid from back home.
    Alum,
    /// An area scout, about the college players in his area.
    Scout,
    /// An agent, about a client he's signing for the draft.
    Agent,
}

struct From {
    k: usize,
    src: Src,
    /// 1-20.
    eye: f64,
    straight: f64,
    /// 0-1: how much he wants this kid placed (his own player, his client).
    agenda: f64,
    area: Option<usize>,
    warmth: f32,
    link: String,
}

/// A division's rank for the claims (5 FBS down to 1 Division III).
pub fn rank(d: Division) -> u8 {
    match d {
        Division::FBS => 5,
        Division::FCS => 4,
        Division::D2 => 3,
        Division::JUCO | Division::NAIA => 2,
        Division::D3 => 1,
    }
}

/// The share of a class a level signs, as a bar on the class's ranking
/// (proposed, not confirmed: FBS signs about the top 1.5% of a class,
/// FCS 3%, Division II 4.5%, JUCO and NAIA 6%, Division III 8%; the
/// draft about the top 1.5% of college players two seasons in).
fn bar(nfl: bool, level: u8) -> f64 {
    if nfl {
        return 0.985;
    }
    match level {
        5 => 0.985,
        4 => 0.97,
        3 => 0.955,
        2 => 0.94,
        _ => 0.92,
    }
}

/// Your seat for tips: (NFL, the college's rank, your coach). None at a
/// high school (C18 covers the 7v7 men there).
fn seat(w: &World) -> Option<(bool, u8, usize, Team)> {
    let you = w.career.as_ref()?.coach;
    match Team::seat(w)? {
        t @ Team::Nfl(_) => Some((true, 0, you, t)),
        t @ Team::College(p) => Some((false, rank(w.programs[p].division), you, t)),
        _ => None,
    }
}

fn source(w: &World, k: usize, nfl: bool, team: Team) -> Option<From> {
    let x = &w.folk.network.contacts[k];
    let m = w.today.parts().1;
    match x.other {
        None => {
            let c = x.coach;
            let co = w.coaches.get(c)?;
            let eye = co.ability().recruiting;
            // A coach's straightness is his own (keyed, hidden).
            let straight = 6.0 + 12.0 * super::roll(w, 0x57A1, c as u64);
            let src = match (super::facts::team_of_coach(w, c)?, nfl) {
                (Team::High(h), false) => Src::Hs(h),
                (Team::College(p), true) if !matches!(m, 5..=8) => Src::College(p),
                _ => return None,
            };
            Some(From { k, src, eye, straight, agenda: 0.6, area: None, warmth: x.warmth, link: coach_link(w, c) })
        }
        Some(Other::Former(id)) => {
            let f = w.formers.get(id)?;
            let alum = matches!(team, Team::College(p) if f.colleges.iter().any(|c| c.0 as usize == p));
            let (src, agenda) = match (f.life, nfl) {
                // The 7v7 circuit and the camps run March to August.
                (Life::Camp, false) if (3..=8).contains(&m) => (Src::Camp, 0.5),
                (Life::Trainer, false) => (Src::Trainer, 0.8),
                (Life::Coaching, false) if f.coach.is_none() => (Src::Youth, 0.5),
                (Life::Scout, true) if !matches!(m, 5..=8) => (Src::Scout, 0.1),
                (Life::Agent, true) if (1..=4).contains(&m) => (Src::Agent, 0.9),
                (_, false) if alum => (Src::Alum, 0.3),
                _ => return None,
            };
            Some(From {
                k,
                src,
                eye: f.eye as f64,
                straight: f.straight as f64,
                agenda,
                area: Some(f.area as usize),
                warmth: x.warmth,
                link: f.name.clone(),
            })
        }
        Some(Other::Agent(a)) if nfl && (1..=4).contains(&m) => {
            let ag = w.market.agents.get(a)?;
            Some(From {
                k,
                src: Src::Agent,
                eye: ag.skill,
                straight: 20.0 * ag.honesty,
                agenda: 0.9,
                area: None,
                warmth: x.warmth,
                link: ag.name.clone(),
            })
        }
        _ => None,
    }
}

/// What a kid shows on the scale the claims use: a recruit's film, a
/// college player's play.
fn value(p: &crate::world::Person) -> f64 {
    if matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) {
        crate::world::recruiting::film_raw(p)
    } else {
        p.player.overall()
    }
}

fn recruit(p: &crate::world::Person, season: i32) -> bool {
    matches!(p.stage, Stage::HighSchool | Stage::PrepSchool)
        && p.class_of > season
        && p.class_of <= season + 2
        && p.player.age >= 15
        && p.cal.signed.is_none()
}

fn draft_pool(p: &crate::world::Person) -> bool {
    p.stage == Stage::College && p.college_seasons >= 2
}

/// The kids he could send you, by his read (best first).
fn pool(w: &World, f: &From, nfl: bool, key: u64) -> Vec<(f64, usize)> {
    use crate::positions::Position as P;
    let season = w.dates.season;
    let sd = 1.5 + 0.45 * (20.0 - f.eye).max(0.0);
    let sent: Vec<u32> = w.folk.tips.list.iter().filter(|t| same(t, w, f.k)).map(|t| t.player).collect();
    let mut v: Vec<(f64, usize)> = Vec::new();
    for (i, p) in w.people.iter().enumerate() {
        let ok = if nfl {
            draft_pool(p)
                && match f.src {
                    Src::College(c) => p.college == Some(c),
                    Src::Scout => p.college.is_some_and(|c| Some(w.programs[c].state) == f.area),
                    Src::Agent => p.college_seasons >= 3,
                    _ => false,
                }
        } else {
            recruit(p, season)
                && match f.src {
                    Src::Hs(h) => p.hs == h,
                    // 7-on-7 has no linemen and no kickers.
                    Src::Camp => {
                        Some(p.home_state) == f.area
                            && !matches!(p.player.position, P::OT | P::IOL | P::DT | P::K | P::P | P::LS)
                    }
                    Src::Trainer => Some(p.home_state) == f.area && p.bg.money >= 0.4,
                    Src::Youth | Src::Alum => Some(p.home_state) == f.area,
                    _ => false,
                }
        };
        if ok && !sent.contains(&p.player.id) {
            v.push((value(p) + sd * key_normal(key, p.player.id as u64), i));
        }
    }
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    v.truncate(3);
    v
}

/// Where his read of a kid ranks in the kid's class (or among college
/// players two seasons in).
fn percentile(w: &World, i: usize, read: f64) -> f64 {
    let p = &w.people[i];
    let (mut below, mut n) = (0usize, 0usize);
    if matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) {
        let class = p.class_of;
        for q in
            w.people.iter().filter(|q| matches!(q.stage, Stage::HighSchool | Stage::PrepSchool) && q.class_of == class)
        {
            n += 1;
            below += (value(q) < read) as usize;
        }
    } else {
        for q in w.people.iter().filter(|q| draft_pool(q)) {
            n += 1;
            below += (value(q) < read) as usize;
        }
    }
    if n == 0 { 0.0 } else { below as f64 / n as f64 }
}

fn same(t: &Tip, w: &World, k: usize) -> bool {
    let x = &w.folk.network.contacts[k];
    t.coach == x.coach && t.other == x.other
}

/// His word on a kid: the claim from his read against your level's bar,
/// a level up when his agenda and a cool tie push him to oversell.
fn claim(w: &World, f: &From, i: usize, read: f64, nfl: bool, level: u8, key: u64) -> Option<u8> {
    let b = bar(nfl, level);
    let pct = percentile(w, i, read);
    // He won't call you about a kid he doesn't think has a shot.
    if pct < b - 2.0 * (1.0 - b) {
        return None;
    }
    let mut c = if pct >= 1.0 - 0.35 * (1.0 - b) {
        2
    } else if pct >= b {
        1
    } else {
        0
    };
    let lean = (f.agenda * (1.0 - f.straight / 20.0) * (1.2 - f.warmth as f64)).clamp(0.0, 1.0);
    if super::roll(w, key ^ 0x0E5E, w.people[i].player.id as u64) < lean {
        c = (c + 1).min(2);
    }
    Some(c)
}

/// What only his camp saw (C14): laser times and a real height, shaded a
/// little when he's selling; how the kid took coaching, by his eye; the
/// kid's real top three; whether anyone's on him.
fn camp_lines(w: &World, f: &From, i: usize, key: u64) -> Vec<String> {
    let p = &w.people[i];
    let id = p.player.id as u64;
    let lean = (f.agenda * (1.0 - f.straight / 20.0) * (1.2 - f.warmth as f64)).clamp(0.0, 1.0);
    let forty = Drills::of(&p.player).expected().forty + 0.03 * key_normal(key ^ 1, id) - 0.06 * lean;
    let height = body_now(&p.player).height + 0.6 * lean;
    let mut out =
        vec![format!("Laser-timed {forty:.2} in the forty at our camp; {} in his socks.", height_text(height.round()))];
    let sd = (21.0 - f.eye) / 4.0;
    let coach = p.player.personality.coachability + sd * key_normal(key ^ 2, id);
    out.push(
        if coach >= 14.0 {
            "He soaks up coaching: tell him once and it's fixed the next rep."
        } else if coach >= 9.0 {
            "He takes coaching fine."
        } else {
            "He doesn't love being told. You'll have to sell him on it."
        }
        .to_string(),
    );
    let mut offers: Vec<(f64, usize)> =
        p.cal.offers.iter().map(|&j| (crate::world::recruiting::appeal(w, i, j, None), j)).collect();
    offers.sort_by(|a, b| b.0.total_cmp(&a.0));
    if offers.is_empty() {
        out.push("Nobody's offered him yet. You'd be first.".into());
    } else {
        let top: Vec<&str> = offers.iter().take(3).map(|&(_, j)| w.program_name(j)).collect();
        out.push(format!("His real top {}: {}.", top.len(), top.join(", ")));
    }
    out
}

/// Sends the tip: the note in your inbox, the tape into your files.
fn send(w: &mut World, f: &From, i: usize, claim: u8, (nfl, level): (bool, u8), camp: bool, key: u64) -> String {
    let d = w.today;
    let lines = if camp { camp_lines(w, f, i, key) } else { Vec::new() };
    let (coach, other) = {
        let x = &mut w.folk.network.contacts[f.k];
        x.last = Some(d);
        (x.coach, x.other)
    };
    let id = w.people[i].player.id;
    w.folk.tips.list.push(Tip {
        date: d,
        coach,
        other,
        player: id,
        claim,
        nfl,
        level,
        lines: lines.clone(),
        settled: None,
    });
    let n = w.folk.tips.list.len();
    if n > 200 {
        let drop = w.folk.tips.list.iter().position(|t| t.settled.is_some()).unwrap_or(0);
        w.folk.tips.list.remove(drop);
    }
    if let Some(you) = w.career.as_ref().map(|c| c.coach) {
        crate::world::scouting::dept::add(w, id, you as u32, LookKind::Film, d);
        if let Some(Team::Nfl(t)) = Team::seat(w) {
            crate::world::scouting::dept::club_look(
                w,
                t,
                id,
                crate::world::pro::scouts::HC,
                crate::world::pro::scouts::LookKind::Film,
                d,
            );
        }
    }
    let p = &w.people[i];
    let at = match p.stage {
        Stage::College => p.college.map_or(String::new(), |c| format!(", {}", w.program_name(c))),
        _ => format!(", {}", w.school_name(p.hs)),
    };
    let mut text = format!(
        "{} sent tape on {}, {}{at}: \"{}.\"",
        f.link,
        player_link(w, i),
        p.player.position.name(),
        CLAIMS[claim as usize]
    );
    for l in &lines {
        text.push(' ');
        text.push_str(l);
    }
    super::note(w, "tips", f.link.clone(), text.clone(), None);
    text
}

/// Wednesdays: a warm contact may send tape (proposed, not confirmed: a
/// coach at 0.2 or warmer, a former player or an agent from 0.1, since
/// camp men, trainers and agents push their kids to anyone who'll take the
/// call; at most one tip from him in eight weeks, a weekly chance of 1%
/// plus 4% times his warmth, at most one tip a week).
pub(crate) fn week(w: &mut World, d: Date) {
    let Some((nfl, level, _, team)) = seat(w) else { return };
    let mut best: Option<(f64, From)> = None;
    for k in 0..w.folk.network.contacts.len() {
        let x = &w.folk.network.contacts[k];
        if x.warmth < if x.other.is_some() { 0.1 } else { 0.2 } {
            continue;
        }
        let recent = w.folk.tips.list.iter().any(|t| same(t, w, k) && t.date.days_to(d) < 56);
        if recent {
            continue;
        }
        let chance = 0.01 + 0.04 * x.warmth as f64;
        let r = super::roll(w, 0x7195 ^ d.0 as u64, k as u64) / chance;
        // Only someone who has kids for your seat this month.
        if r < 1.0
            && best.as_ref().is_none_or(|b| r < b.0)
            && let Some(f) = source(w, k, nfl, team)
        {
            best = Some((r, f));
        }
    }
    let Some((_, f)) = best else { return };
    let k = f.k;
    let key = w.folk.base as u64 ^ 0x7196 ^ ((d.0 as u64) << 20) ^ k as u64;
    let Some(&(read, i)) = pool(w, &f, nfl, key).first() else { return };
    let Some(c) = claim(w, &f, i, read, nfl, level, key) else { return };
    send(w, &f, i, c, (nfl, level), f.src == Src::Camp, key);
}

/// A coach's favour call, "I've got a kid I think can play for you" (C14,
/// `asks_week`): you said yes, so his best kid's tape comes in. None when
/// he has nobody for your seat.
pub(crate) fn from_ask(w: &mut World, c: usize) -> Option<String> {
    let (nfl, level, _, team) = seat(w)?;
    let k = w.folk.network.contacts.iter().position(|x| x.coach_id() == Some(c))?;
    let f = source(w, k, nfl, team)?;
    let key = w.folk.base as u64 ^ 0x7197 ^ ((w.today.0 as u64) << 20) ^ c as u64;
    let &(read, i) = pool(w, &f, nfl, key).first()?;
    // He asked because he believes in him: never less than "can play".
    let cl = claim(w, &f, i, read, nfl, level, key).unwrap_or(1).max(1);
    Some(send(w, &f, i, cl, (nfl, level), false, key))
}

/// September 1: tips whose kids' roads show are settled (proposed, not
/// confirmed). For a college: "worth a look" holds if he went on to play
/// college football, "can play for you" if he played 150 snaps at your
/// level or higher by his second season, "a sure thing" 400 or the NFL.
/// For an NFL club: once he's out of college, "worth a look" and "can
/// play" hold if he made the league, "a sure thing" if he went in the
/// first three rounds. A tip six years old settles as missed.
pub(crate) fn settle(w: &mut World, d: Date) {
    let (_, m, day) = d.parts();
    if !(m == 9 && day == 1) || w.folk.tips.list.iter().all(|t| t.settled.is_some()) {
        return;
    }
    let mut done: Vec<(usize, bool)> = Vec::new();
    for (k, t) in w.folk.tips.list.iter().enumerate() {
        if t.settled.is_some() || t.date.add(300) > d {
            continue;
        }
        let old = t.date.add(6 * 365) <= d;
        let Some(i) = crate::world::scouting::place(w, t.player) else {
            done.push((k, false));
            continue;
        };
        let p = &w.people[i];
        let pro = p.pro.is_some() || p.stage == Stage::Pro;
        let r = if t.nfl {
            match p.stage {
                Stage::College if !old => None,
                _ => Some(match t.claim {
                    2 => p.pro.as_ref().and_then(|x| x.draft).is_some_and(|x| x.1 <= 3),
                    _ => pro,
                }),
            }
        } else {
            let lvl = p.college.map_or(0, |c| rank(w.programs[c].division));
            let shown = pro
                || p.college_seasons >= 2
                || !matches!(p.stage, Stage::HighSchool | Stage::PrepSchool | Stage::College);
            if !shown && !old {
                None
            } else {
                Some(match t.claim {
                    0 => p.college.is_some() || pro,
                    1 => pro || (p.college_snaps >= 150 && lvl >= t.level),
                    _ => pro || (p.college_snaps >= 400 && lvl >= t.level),
                })
            }
        };
        if let Some(held) = r {
            done.push((k, held));
        }
    }
    for (k, held) in done {
        let (coach, other) = {
            let t = &mut w.folk.tips.list[k];
            t.settled = Some(held);
            (t.coach, t.other)
        };
        if let Some(x) = w.folk.network.contacts.iter_mut().find(|x| x.coach == coach && x.other == other) {
            x.record.0 = x.record.0.saturating_add(1);
            x.record.1 = x.record.1.saturating_add(held as u8);
        }
    }
}
