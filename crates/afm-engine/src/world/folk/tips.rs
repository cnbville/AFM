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
    /// Why the public missed him (SK70), from his own record.
    #[serde(default)]
    pub why: Option<String>,
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

/// The floor for a call (SK69): half a level's share below its bar, so a
/// kid on the edge of your level is "worth a look" and nobody calls about a
/// kid below it (proposed, not confirmed: FBS the top 2.25% of a class,
/// the NFL the top 2.25% of college players two seasons in, which takes in
/// the undrafted who sign right after the draft).
fn floor(nfl: bool, level: u8) -> f64 {
    let b = bar(nfl, level);
    b - 0.5 * (1.0 - b)
}

/// The public grade of him that a caller says missed him: three stars or
/// fewer at an FBS seat, two below it; at an NFL club, no public grade or
/// one past the third round.
fn public_missed(w: &World, i: usize, nfl: bool, level: u8) -> bool {
    let p = &w.people[i];
    if nfl { p.cal.draft_read.as_ref().is_none_or(|r| r.round >= 4) } else { p.stars <= if level >= 5 { 3 } else { 2 } }
}

/// Why the public missed him (SK70): the true facts of his own record that
/// held his public grade down, the strongest first; None when nothing did.
/// For a recruit these are the parts of a recruit's visibility
/// (`recruiting.rs`, `recruit_visibility`) and his tape: a small school few
/// staffs visit, a coach with no college ties, a family that can't pay for
/// camps, a growth spurt the lists haven't caught, a season lost to injury,
/// a new position, a year behind an older kid. For a college player: a
/// small division, an injury, a new position, one season as a starter, a
/// late breakout.
pub(crate) fn why_missed(w: &World, i: usize) -> Option<String> {
    use crate::player::{DevelopmentEvent as Ev, Level, PlayingTime as Pt};
    let p = &w.people[i];
    let hs_kid = matches!(p.stage, Stage::HighSchool | Stage::PrepSchool);
    let rows: Vec<&crate::player::SeasonRecord> = p
        .player
        .history
        .iter()
        .filter(|r| if hs_kid { r.level == Level::HS } else { !matches!(r.level, Level::HS | Level::NFL) })
        .collect();
    let last = rows.last().copied();
    let prev = rows.len().checked_sub(2).map(|k| rows[k]);
    let mut v: Vec<(f64, String)> = Vec::new();
    let missed: u32 = rows.iter().rev().take(2).map(|r| r.games_missed).sum();
    if let Some(r) = last {
        if let Some(q) = prev
            && q.position != r.position
        {
            v.push((
                0.07,
                format!(
                    "He moved from {} to {} last season, and the lists still have him as a {}.",
                    q.position.name(),
                    r.position.name(),
                    q.position.name()
                ),
            ));
        }
        if r.events.has(Ev::GrowthSpurt) && hs_kid {
            v.push((0.1, "He grew into his frame this past year, and the services ranked the smaller kid.".into()));
        }
        if r.events.has(Ev::Breakout) && !hs_kid {
            v.push((0.09, "He broke out last season, after the boards were set.".into()));
        }
        let starts = rows.iter().filter(|x| x.playing_time == Pt::Starter).count();
        if r.playing_time == Pt::Starter && starts == 1 && rows.len() >= 2 {
            v.push((
                0.08,
                if hs_kid {
                    "He sat behind an older kid until last season: one year of tape.".to_string()
                } else {
                    "One season as a starter: he waited behind an older player until last fall.".to_string()
                },
            ));
        }
    }
    if missed >= 4 {
        v.push((
            0.025 * missed as f64,
            format!("He missed {missed} games hurt, so the tape the graders look at is thin."),
        ));
    }
    if hs_kid {
        let h = &w.high_schools[p.hs];
        if h.prestige < 0.35 && !h.national {
            v.push((
                0.3 * (0.5 - h.prestige),
                format!(
                    "{} is a small school of {} kids, and few college staffs ever come through.",
                    w.school_name(p.hs),
                    h.enrollment
                ),
            ));
        }
        let net = h.coach.map_or(0.4, |c| w.coaches[c].ability().network / 20.0);
        if net < 0.4 {
            v.push((
                0.2 * (0.55 - net),
                "His coach has no college ties, so nobody has called anyone about him.".into(),
            ));
        }
        if p.bg.money < 0.3 {
            v.push((
                0.15 * 0.7 * (0.5 - p.bg.money),
                "His family can't pay for the camp circuit, so no service has timed or measured him.".into(),
            ));
        }
    } else if let Some(c) = p.college {
        let div = w.programs[c].division;
        if div != Division::FBS {
            v.push((
                if div == Division::FCS { 0.2 } else { 0.3 },
                format!(
                    "He plays at {} in {}: most clubs never send more than an area scout.",
                    w.program_name(c),
                    div.name()
                ),
            ));
        }
    }
    v.retain(|x| x.0 >= 0.03);
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    let first = v.first()?;
    Some(match v.get(1) {
        Some(second) if second.0 >= 0.6 * first.0 => format!("{} {}", first.1, second.1),
        _ => first.1.clone(),
    })
}

/// A kid worth the call (SK69, SK70): his claim, and why the public missed
/// him when it did. A kid the public missed with no true reason in his
/// record isn't one: the caller's read is just high.
fn worth(w: &World, f: &From, i: usize, read: f64, nfl: bool, level: u8, key: u64) -> Option<(u8, Option<String>)> {
    let c = claim(w, f, i, read, nfl, level, key)?;
    if !public_missed(w, i, nfl, level) {
        return Some((c, None));
    }
    Some((c, Some(why_missed(w, i)?)))
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
    // SK69: he calls only about a kid his own eye says can play at your
    // level: one a program like yours would offer, or one he thinks gets
    // drafted or signed right after the draft.
    if pct < floor(nfl, level) {
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

/// Sends the tip: the note in your inbox, the tape to your staff (SK72:
/// your staff watch it within the week, a film look in his file; film
/// uses no evaluation days).
#[allow(clippy::too_many_arguments)]
fn send(
    w: &mut World,
    f: &From,
    i: usize,
    claim: u8,
    why: Option<String>,
    (nfl, level): (bool, u8),
    camp: bool,
    key: u64,
) -> String {
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
        why: why.clone(),
    });
    let n = w.folk.tips.list.len();
    if n > 200 {
        let drop = w.folk.tips.list.iter().position(|t| t.settled.is_some()).unwrap_or(0);
        w.folk.tips.list.remove(drop);
    }
    let p = &w.people[i];
    let film =
        if matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) { LookKind::HsFilm } else { LookKind::CollegeFilm };
    if !w.scouting.dept.asks.iter().any(|a| a.0 == id && a.1 == film) {
        w.scouting.dept.asks.push((id, film, d));
    }
    let at = match p.stage {
        Stage::College => p.college.map_or(String::new(), |c| format!(", {}", w.program_name(c))),
        _ => format!(", {}", w.school_name(p.hs)),
    };
    let mut text = format!(
        "{} sent tape on {}, {}{at}: \"{}.{}\"",
        f.link,
        player_link(w, i),
        p.player.position.name(),
        CLAIMS[claim as usize],
        why.as_ref().map_or(String::new(), |y| format!(" {y}"))
    );
    for l in &lines {
        text.push(' ');
        text.push_str(l);
    }
    text.push_str(" Your staff will watch his film this week.");
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
    let Some((i, (c, why))) =
        pool(w, &f, nfl, key).into_iter().find_map(|(read, i)| Some((i, worth(w, &f, i, read, nfl, level, key)?)))
    else {
        return;
    };
    send(w, &f, i, c, why, (nfl, level), f.src == Src::Camp, key);
}

/// A coach's favour call, "I've got a kid I think can play for you" (C14,
/// SK69-SK70, `asks_week`): the kid he'd call about, before he calls. Only
/// a kid worth the call: one his own eye says can play at your level, and
/// when the public missed him, a true reason why. None means no call.
pub(crate) fn ask_kid(w: &World, c: usize) -> Option<(u32, u8, Option<String>)> {
    let (nfl, level, _, team) = seat(w)?;
    let k = w.folk.network.contacts.iter().position(|x| x.coach_id() == Some(c))?;
    let f = source(w, k, nfl, team)?;
    let key = w.folk.base as u64 ^ 0x7197 ^ ((w.today.0 as u64) << 20) ^ c as u64;
    pool(w, &f, nfl, key).into_iter().find_map(|(read, i)| {
        let (cl, why) = worth(w, &f, i, read, nfl, level, key)?;
        // He asked because he believes in him: never less than "can play".
        Some((w.people[i].player.id, cl.max(1), why))
    })
}

/// You said yes to his call: his tape comes to your staff.
pub(crate) fn from_ask(w: &mut World, c: usize, id: u32, cl: u8) -> Option<String> {
    let (nfl, level, _, team) = seat(w)?;
    let k = w.folk.network.contacts.iter().position(|x| x.coach_id() == Some(c))?;
    let f = source(w, k, nfl, team)?;
    let i = crate::world::scouting::place(w, id)?;
    let why = if public_missed(w, i, nfl, level) { why_missed(w, i) } else { None };
    let key = w.folk.base as u64 ^ 0x7197 ^ ((w.today.0 as u64) << 20) ^ c as u64;
    Some(send(w, &f, i, cl, why, (nfl, level), false, key))
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
