//! Lane G9: your side of the race (GR65). Rival staffs work the same
//! ground by the same rules (`ai`); you see them the way a real staff
//! does, in signs, never numbers:
//! - the sign-in sheet at a school's front office and the polo in the
//!   stands, on your own visits (`at_school`);
//! - what a coach tells you about the others, by your standing with him
//!   (`told`): Known names who's been in, Trusted counts it, Home tells you
//!   who is about to offer;
//! - the rival recruiters as people (`rivals`, `rival`): their men, areas,
//!   style, ground and history with you;
//! - a commit of yours who wavers, with a warning and one call to fight for
//!   him (`watch`, `fight`);
//! - claims about the other side that settle on the facts, both ways
//!   (`run_down` by rivals, `claim` by you, `settle`);
//! - rumors of a rival's money outside the rules (`rumors`).
//!
//! Only your seat keeps any of it: every hook returns at once without a
//! college career, and none of it touches the world beyond your own staff's
//! memory and your commits' hold on the one call you make.

use super::ai;
use super::people::{self, Holder, Who};
use super::standing::{self, Level};
use crate::calendar::Date;
use crate::world::coach::{Employer, Role};
use crate::world::{Stage, World};

/// A sheet shows the visitors of the last six weeks (proposed, not
/// confirmed).
pub const SHEET_DAYS: i32 = 42;
/// The signs a commit's last two weeks are read over.
pub const WAVER_DAYS: i32 = 14;
/// One warning about the same commit a month.
pub const WAVER_GAP: i32 = 30;
/// What your call adds to his hold on you for 60 days, in the utility
/// points his commitment holds by (`recruiting::stickiness` tops at 0.18),
/// at its best (his family Home with you); proposed, not confirmed.
pub const FIGHT_HOLD: f64 = 0.06;
pub const FIGHT_DAYS: i32 = 60;
/// A rival salesman runs you down to a kid's family once a season when his
/// staff is in the kid's race and Known or better with his coach.
pub const RUN_DOWN_TOP: usize = 3;

/// What a claim is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Topic {
    /// "He'll be gone in a year": the head coach's job.
    Job,
    /// "They'll bury him": the depth chart at his position.
    Depth,
    /// "Their scheme doesn't fit him."
    Scheme,
}

impl Topic {
    pub fn key(self) -> &'static str {
        match self {
            Topic::Job => "job",
            Topic::Depth => "depth",
            Topic::Scheme => "scheme",
        }
    }

    pub fn from_key(k: &str) -> Option<Topic> {
        match k {
            "job" => Some(Topic::Job),
            "depth" => Some(Topic::Depth),
            "scheme" => Some(Topic::Scheme),
            _ => None,
        }
    }
}

/// A claim made to a kid's family about the other side.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Claim {
    pub kid: u32,
    /// Who said it: a program and its man (none: you).
    pub by: u32,
    pub man: Option<u32>,
    /// Whom it was about.
    pub about: u32,
    pub topic: Topic,
    pub said: Date,
    /// The day it settles on the facts.
    pub settles: Date,
    /// True or false, once settled.
    pub held: Option<bool>,
    pub words: String,
    /// The head coach a job claim named: the one in the job that day.
    #[serde(default)]
    pub named: Option<u32>,
}

/// A commit of yours with rival signs around him.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Waver {
    pub id: u32,
    pub rival: u32,
    pub date: Date,
    pub signs: Vec<String>,
    /// Your call: the day and how it landed (0-1).
    pub fought: Option<(Date, f32)>,
    pub line: Option<String>,
}

/// Your side of the race on `Ground.race`.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Race {
    pub wavers: Vec<Waver>,
    pub claims: Vec<Claim>,
    /// Rumors already heard, by kid and program key.
    pub rumors: Vec<u64>,
    /// Kids a rival ran you down to this season, by (kid, season).
    pub(crate) run: Vec<(u32, i32)>,
}

/// A rival's visit as a sign: the program, its man, the day.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Visit {
    pub program: u32,
    pub school: String,
    pub man: u32,
    pub name: String,
    pub date: Date,
}

fn last_name(full: &str) -> &str {
    full.rsplit(' ').next().unwrap_or(full)
}

/// The college program a coach works for now.
fn program_of(w: &World, man: u32) -> Option<usize> {
    match w.coaches.get(man as usize)?.job?.employer {
        Employer::College(p) => Some(p),
        _ => None,
    }
}

/// The program a man's visit to a head coach was for: his program now, if
/// that program's own tie with the coach was seen the same day (a man who
/// moved since doesn't carry his old school's visits to the new one).
fn visit_for(w: &World, coach: u32, man: u32, seen: Date) -> Option<usize> {
    let p = program_of(w, man)?;
    let t = w.ground.ai.ties.0.get(&ai::tie_key(coach, Holder::Program(p as u32)))?;
    (t.seen == seen).then_some(p)
}

/// The rival men who visited a school's head coach since `since` (the
/// sign-in sheet), newest first; your own staff left out.
pub fn sheet(w: &World, school: usize, since: Date, to: Date) -> Vec<Visit> {
    let Some(Who::Coach(c)) = people::head_coach_of(w, school) else { return Vec::new() };
    let you = ai::seat_college(w);
    let mut v: Vec<Visit> = w
        .ground
        .ai
        .with(c)
        .filter_map(|(h, t)| match h {
            Holder::Coach(m) if t.seen >= since && t.seen <= to => {
                let p = visit_for(w, c, m, t.seen)?;
                (Some(p) != you).then(|| Visit {
                    program: p as u32,
                    school: w.programs[p].real_name.clone(),
                    man: m,
                    name: w.coaches[m as usize].name.clone(),
                    date: t.seen,
                })
            }
            _ => None,
        })
        .collect();
    v.sort_by(|a, b| b.date.cmp(&a.date).then(a.man.cmp(&b.man)));
    v
}

/// What your man sees at a school on his visit: the sign-in sheet and any
/// rival in the stands today, in a line for the day's log.
pub fn at_school(w: &World, school: usize, d: Date) -> Option<String> {
    let v = sheet(w, school, d.add(-SHEET_DAYS), d);
    if v.is_empty() {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    let here: Vec<&Visit> = v.iter().filter(|x| x.date == d).collect();
    for x in here.iter().take(2) {
        parts.push(format!("{} was there too, in his {} polo.", x.name, x.school));
    }
    let earlier: Vec<String> = v
        .iter()
        .filter(|x| x.date < d)
        .take(4)
        .map(|x| format!("{}'s {} {}", x.school, last_name(&x.name), super::week::beat::short(x.date)))
        .collect();
    if !earlier.is_empty() {
        parts.push(format!("The sign-in sheet: {}.", earlier.join(", ")));
    }
    Some(parts.join(" "))
}

/// What a kid's head coach tells you about the other staffs, by your
/// standing with him (GR19's levels): nothing as a Stranger; a Known coach
/// names who has been in this spring; a Trusted one counts it; a Home one
/// tells you who is about to offer. Your own staff's numbers never show.
pub fn told(w: &World, id: u32) -> Vec<String> {
    let Some(prog) = standing::seat_program(w) else { return Vec::new() };
    let Some(i) = crate::world::scouting::place(w, id) else { return Vec::new() };
    let q = &w.people[i];
    let Some(who @ Who::Coach(c)) = people::head_coach_of(w, q.hs) else { return Vec::new() };
    let lv = standing::level(w, prog, who);
    if lv == Level::Stranger {
        return Vec::new();
    }
    let season = w.dates.season as i16;
    let mut by: Vec<(usize, u8, Date)> = Vec::new();
    for (h, t) in w.ground.ai.with(c) {
        if let Holder::Program(p) = h
            && p as usize != prog
            && t.season == season
            && t.visits > 0
        {
            by.push((p as usize, t.visits, t.seen));
        }
    }
    by.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)).then(a.0.cmp(&b.0)));
    let coach = last_name(&w.coaches[c as usize].name).to_string();
    let name = |p: usize| w.programs[p].real_name.clone();
    let mut out = Vec::new();
    if by.is_empty() {
        if lv >= Level::Known {
            out.push(format!("Coach {coach}: \"Nobody else has been in for him this year.\""));
        }
        return out;
    }
    match lv {
        Level::Known => {
            let names: Vec<String> = by.iter().take(3).map(|x| name(x.0)).collect();
            out.push(format!("Coach {coach}: \"{} came through this year.\"", and_list(&names)));
        }
        Level::Trusted => {
            for x in by.iter().take(2) {
                out.push(format!("Coach {coach}: \"{}'s been in {}.\"", name(x.0), times(x.1)));
            }
        }
        _ => {
            let offered: Vec<usize> =
                q.cal.early_offers.iter().map(|o| o.0).chain(q.cal.offers.iter().copied()).collect();
            if let Some(x) = by.iter().find(|x| x.1 >= 2 && !offered.contains(&x.0)) {
                out.push(format!("Coach {coach}: \"{}'s about to offer him. Get in first.\"", name(x.0)));
            }
            for x in by.iter().filter(|x| offered.contains(&x.0)).take(2) {
                out.push(format!("Coach {coach}: \"{} has offered, and they've been in {}.\"", name(x.0), times(x.1)));
            }
            if out.is_empty() {
                out.push(format!("Coach {coach}: \"{}'s been in {}.\"", name(by[0].0), times(by[0].1)));
            }
        }
    }
    out
}

fn times(n: u8) -> String {
    match n {
        1 => "once".into(),
        2 => "twice".into(),
        n => format!("{n} times"),
    }
}

fn and_list(v: &[String]) -> String {
    match v.len() {
        0 => String::new(),
        1 => v[0].clone(),
        2 => format!("{} and {}", v[0], v[1]),
        _ => format!("{} and {}", v[..v.len() - 1].join(", "), v[v.len() - 1]),
    }
}

// --- Rival recruiters as people ------------------------------------------------------------

/// A rival's man on your ground, as the rivals page shows him.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Rival {
    pub man: u32,
    pub name: String,
    pub program: u32,
    pub school: String,
    pub role: String,
    /// His style in a word and a line (`staff::men::style`).
    pub style: String,
    pub style_line: String,
    /// The areas he works that your staff works too, each with where he
    /// stands there in a word.
    pub areas: Vec<(u16, String, &'static str)>,
    /// Coaches who are Home with him (his own ties), by school name.
    pub home: Vec<String>,
    /// How long he has been on that staff, his titles, a line on his
    /// career.
    pub career: Vec<String>,
    /// What's between you: staffs you shared, kids he beat you for.
    pub history: Vec<String>,
    /// His last visits your people saw (sign-in sheets in your areas).
    pub seen: Vec<String>,
}

fn role_words(r: Role) -> &'static str {
    super::staff::men::role_words(r)
}

/// A man's level over a set of schools from his own ties, weighted by talent.
fn man_level(w: &World, man: u32, schools: &[usize], talent: &[f32]) -> &'static str {
    let (mut sum, mut weight) = (0.0f32, 0.0f32);
    for &h in schools {
        let t = talent.get(h).copied().unwrap_or(1.0);
        let p = people::head_coach_of(w, h).map_or(0.0, |who| standing::points(w, who, Holder::Coach(man)));
        sum += t * p;
        weight += t;
    }
    Level::of(if weight > 0.0 { sum / weight } else { 0.0 }).name()
}

/// The areas your staff works (your men's, else your state's).
fn your_areas(w: &World, prog: usize) -> Vec<u16> {
    let mut v: Vec<u16> = w.ground.staff.beats.iter().map(|b| b.area).collect();
    if v.is_empty() {
        v = super::areas::in_state(w.programs[prog].state).to_vec();
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// The rival men working your areas, the ones with the most ground first.
pub fn rivals(w: &World) -> Vec<Rival> {
    let Some(prog) = standing::seat_program(w) else { return Vec::new() };
    let mine = your_areas(w, prog);
    let by_area = super::areas::schools_by_area(w);
    let talent = standing::talents(w);
    let mut out: Vec<(usize, Rival)> = Vec::new();
    for b in &w.ground.ai.beats {
        let shared: Vec<u16> = b.areas.iter().copied().filter(|a| mine.binary_search(a).is_ok()).collect();
        if shared.is_empty() || program_of(w, b.man) != Some(b.prog as usize) {
            continue;
        }
        if let Some(r) = rival_row(w, prog, b.man, &shared, &by_area, &talent) {
            let weight = r.home.len() * 3 + r.areas.iter().filter(|a| a.2 != "Stranger").count();
            out.push((weight, r));
        }
    }
    out.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.man.cmp(&b.1.man)));
    out.into_iter().map(|x| x.1).take(30).collect()
}

/// One rival man's page.
pub fn rival(w: &World, man: u32) -> Option<Rival> {
    let prog = standing::seat_program(w)?;
    let p = program_of(w, man)?;
    let areas: Vec<u16> = ai::beats_of(w, p).iter().find(|b| b.man == man).map(|b| b.areas.clone()).unwrap_or_default();
    let by_area = super::areas::schools_by_area(w);
    let talent = standing::talents(w);
    rival_row(w, prog, man, &areas, &by_area, &talent)
}

fn rival_row(w: &World, prog: usize, man: u32, areas: &[u16], by_area: &[Vec<usize>], talent: &[f32]) -> Option<Rival> {
    let c = w.coaches.get(man as usize)?;
    let job = c.job?;
    let Employer::College(p) = job.employer else { return None };
    let (style, line) = super::staff::men::style(w, man as usize);
    let table = super::areas::table();
    let area_rows: Vec<(u16, String, &'static str)> = areas
        .iter()
        .map(|&a| {
            let schools = by_area.get(a as usize).map_or(&[][..], |v| &v[..]);
            (a, table[a as usize].nickname.to_string(), man_level(w, man, schools, talent))
        })
        .collect();
    let mut home: Vec<(f32, String)> = Vec::new();
    for (&k, t) in w.ground.ai.ties.0.iter() {
        let (coach, holder) = ai::key_parts(k);
        if holder == Holder::Coach(man)
            && t.points >= 75.0
            && let Some((h, Role::HeadCoach)) = people::coach_school(w, coach as usize)
        {
            home.push((talent.get(h).copied().unwrap_or(1.0), w.school_name(h).to_string()));
        }
    }
    home.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let since = c.history.iter().rev().take_while(|s| s.employer == job.employer).map(|s| s.from).min();
    let mut career = Vec::new();
    if let Some(y) = since {
        let n = (w.year - y).max(0);
        career.push(format!("{} years on {}'s staff", n.max(1), w.programs[p].real_name));
    }
    if c.career_titles > 0 {
        career.push(format!("{} titles as a coach", c.career_titles));
    }
    let mut history = Vec::new();
    if let Some(you) = w.career.as_ref().map(|x| x.coach)
        && let Some(yc) = w.coaches.get(you)
    {
        for s in &c.history {
            if yc.history.iter().any(|y| y.employer == s.employer && y.from <= s.to && s.from <= y.to)
                && let Employer::College(q) = s.employer
            {
                history.push(format!("You coached together at {}", w.programs[q].real_name));
            }
        }
    }
    for x in w.ground.race.claims.iter().filter(|x| x.man == Some(man)) {
        history.push(match x.held {
            Some(false) => format!("He told a family {} It wasn't true.", x.words),
            Some(true) => format!("He told a family {} It held up.", x.words),
            None => format!("He's telling a family {}", x.words),
        });
    }
    for x in w.ground.race.wavers.iter().filter(|x| x.rival as usize == p) {
        history.push(format!("Worked your commit {} ({})", kid_name(w, x.id), super::week::beat::short(x.date)));
    }
    history.dedup();
    let mut seen: Vec<String> = Vec::new();
    for &a in areas {
        for &h in by_area.get(a as usize).map_or(&[][..], |v| &v[..]) {
            if standing::level(w, prog, people::head_coach_of(w, h).unwrap_or(Who::Counselor(h as u32))) < Level::Known
            {
                continue;
            }
            for v in sheet(w, h, w.today.add(-SHEET_DAYS), w.today) {
                if v.man == man {
                    seen.push(format!("{} {}", w.school_name(h), super::week::beat::short(v.date)));
                }
            }
        }
    }
    seen.truncate(6);
    let _ = prog;
    Some(Rival {
        man,
        name: c.name.clone(),
        program: p as u32,
        school: w.programs[p].real_name.clone(),
        role: role_words(job.role).to_string(),
        style: style.to_string(),
        style_line: line,
        areas: area_rows,
        home: home.into_iter().take(8).map(|x| x.1).collect(),
        career,
        history,
        seen,
    })
}

fn kid_name(w: &World, id: u32) -> String {
    crate::world::scouting::place(w, id).map_or_else(|| "a recruit".into(), |i| w.people[i].name.clone())
}

// --- Your commits waver ---------------------------------------------------------------------

/// Mondays: each commit of yours with rival signs around him in the last
/// two weeks: a rival's man at his school with a staff his coach knows, a
/// visit he took to a rival, a rival's offer. Two signs, or a visit, make
/// a warning (GR25) with one call to fight for him.
pub(crate) fn watch(w: &mut World, d: Date) {
    let Some(prog) = standing::seat_program(w) else { return };
    if d.weekday() != crate::calendar::Weekday::Mon {
        return;
    }
    let since = d.add(-WAVER_DAYS);
    let kids: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            let q = &w.people[i];
            q.stage == Stage::HighSchool && q.cal.signed.is_none() && q.cal.commit.is_some_and(|c| c.0 == prog)
        })
        .collect();
    for i in kids {
        let q = &w.people[i];
        let id = q.player.id;
        if w.ground.race.wavers.iter().any(|x| x.id == id && x.date.days_to(d) < WAVER_GAP) {
            continue;
        }
        let mut by: Vec<(usize, Vec<String>, u8)> = Vec::new();
        let mut add = |p: usize, s: String, strong: u8| match by.iter_mut().find(|x| x.0 == p) {
            Some(x) => {
                x.1.push(s);
                x.2 += strong;
            }
            None => by.push((p, vec![s], strong)),
        };
        for v in sheet(w, q.hs, since, d) {
            let lv =
                people::head_coach_of(w, q.hs).map_or(Level::Stranger, |c| standing::level(w, v.program as usize, c));
            if lv >= Level::Known {
                add(
                    v.program as usize,
                    format!(
                        "{}'s {} was at his school {}",
                        v.school,
                        last_name(&v.name),
                        super::week::beat::short(v.date)
                    ),
                    1,
                );
            }
        }
        for &(p, day, kind) in &q.cal.visits {
            if p != prog && day >= since && day <= d {
                add(p, format!("{} {}", visit_words(kind, &w.programs[p].real_name), super::week::beat::short(day)), 2);
            }
        }
        for &(p, day) in &q.cal.early_offers {
            if p != prog && day >= since && day <= d {
                add(p, format!("{} offered him {}", w.programs[p].real_name, super::week::beat::short(day)), 1);
            }
        }
        by.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
        let Some((rival, signs, strength)) = by.into_iter().next() else { continue };
        if strength < 2 {
            continue;
        }
        let name = q.name.clone();
        let school = w.programs[rival].real_name.clone();
        let mut body = vec![format!("{name} is wavering. {school} is working him:")];
        body.extend(signs.iter().map(|s| format!("- {s}")));
        body.push("One call to his family this week is your chance to hold him.".into());
        w.ground.race.wavers.push(Waver { id, rival: rival as u32, date: d, signs, fought: None, line: None });
        if let Some(who) = people::head_coach_of(w, w.people[i].hs) {
            standing::post_memo(w, who, d, format!("{name} is wavering"), body, 2);
        }
    }
    let cut = d.add(-365);
    w.ground.race.wavers.retain(|x| x.date >= cut);
}

fn visit_words(v: crate::world::Visit, school: &str) -> String {
    match v {
        crate::world::Visit::Official => format!("he took an official visit to {school}"),
        crate::world::Visit::Unofficial => format!("he took an unofficial visit to {school}"),
        crate::world::Visit::Home => format!("he had {school}'s head coach in his living room"),
        crate::world::Visit::DropIn => format!("{school}'s staff came to see him at school"),
    }
}

/// Your call to a wavering commit's family (the one scene to fight for
/// him, as a one-line result until a played scene takes it): your head
/// coach calls his family and his coach, and how it lands follows your
/// standing with the family after the call.
pub fn fight(w: &mut World, id: u32) -> Result<String, String> {
    let prog = standing::seat_program(w).ok_or("No college staff.")?;
    let k = w
        .ground
        .race
        .wavers
        .iter()
        .rposition(|x| x.id == id && x.fought.is_none())
        .ok_or("He isn't wavering, or you already called.")?;
    let i = crate::world::scouting::place(w, id).ok_or("No such recruit.")?;
    let d = w.today;
    let you = w.career.as_ref().map(|c| c.coach);
    let fam = people::family_of(w, i);
    standing::contact(w, prog, you, fam, standing::Contact::Call, 1.0, d);
    if let Some(c) = people::head_coach_of(w, w.people[i].hs) {
        standing::contact(w, prog, you, c, standing::Contact::Call, 1.0, d);
    }
    let lv = standing::level(w, prog, fam);
    let land = match lv {
        Level::Home => 1.0,
        Level::Trusted => 0.75,
        Level::Known => 0.45,
        Level::Stranger => 0.2,
    };
    let voice = people::family_voice(w, fam);
    let rival = w.programs[w.ground.race.wavers[k].rival as usize].real_name.clone();
    let line = match lv {
        Level::Home | Level::Trusted => {
            format!("{voice}: \"We're with you, Coach. {rival} keeps calling, but we know you.\"")
        }
        Level::Known => format!("{voice}: \"We're still listening, Coach. {rival} made a strong case.\""),
        Level::Stranger => format!("{voice}: \"We'll talk it over as a family.\""),
    };
    w.ground.race.wavers[k].fought = Some((d, land));
    w.ground.race.wavers[k].line = Some(line.clone());
    Ok(line)
}

/// What your call adds to a commit's hold on you (`recruiting`'s
/// matching): `FIGHT_HOLD` × how it landed, for `FIGHT_DAYS`.
pub(crate) fn fight_hold(w: &World, id: u32, prog: usize, today: Date) -> f64 {
    if ai::seat_college(w) != Some(prog) {
        return 0.0;
    }
    w.ground
        .race
        .wavers
        .iter()
        .filter(|x| x.id == id)
        .filter_map(|x| x.fought)
        .filter(|f| f.0.days_to(today) <= FIGHT_DAYS && f.0 <= today)
        .map(|f| FIGHT_HOLD * f.1 as f64)
        .fold(0.0, f64::max)
}

// --- Claims about the other side ------------------------------------------------------------

/// Whether a claim about a program is true today.
fn fact(w: &World, about: usize, kid: usize, topic: Topic) -> bool {
    let p = &w.programs[about];
    let q = &w.people[kid];
    match topic {
        Topic::Job => p.head_coach.is_some_and(|c| -3.0 * w.coaches[c].recent.last().copied().unwrap_or(0.0) >= 0.34),
        Topic::Depth => {
            let pos = q.player.position;
            let room: usize = w
                .people
                .iter()
                .filter(|x| {
                    x.stage == Stage::College
                        && x.college == Some(about)
                        && x.player.position == pos
                        && x.last_role == Some(crate::player::PlayingTime::Starter)
                        && x.class_of >= w.year - 2
                })
                .count();
            let one = matches!(
                pos,
                crate::positions::Position::QB
                    | crate::positions::Position::RB
                    | crate::positions::Position::TE
                    | crate::positions::Position::K
                    | crate::positions::Position::P
            );
            room >= if one { 1 } else { 2 }
        }
        Topic::Scheme => {
            let f = crate::world::systems::fits(&q.player, q.player.position, &w.system_norms);
            crate::world::systems::fit_to(&f, q.player.position, &p.installed) < -0.3
        }
    }
}

fn claim_words(topic: Topic, about: &str) -> String {
    match topic {
        Topic::Job => format!("{about}'s head coach will be gone in a year."),
        Topic::Depth => format!("{about} will bury him on the depth chart."),
        Topic::Scheme => format!("{about}'s scheme doesn't fit him."),
    }
}

/// When a claim settles: a job by the next December 15, the depth chart
/// and the scheme when he can see them (his official visit's season,
/// about two months on).
fn settle_day(topic: Topic, d: Date) -> Date {
    match topic {
        Topic::Job => {
            let (y, m, _) = d.parts();
            let y = if m == 12 { y + 1 } else { y };
            Date::ymd(y, 12, 15)
        }
        _ => d.add(60),
    }
}

/// Your claim about a rival to a kid's family (GR62's built lie check: a
/// claim like any other). It settles on the facts on its day; a false one
/// costs you with that family and his coach.
pub fn claim(w: &mut World, id: u32, about: u32, topic: Topic) -> Result<String, String> {
    let prog = standing::seat_program(w).ok_or("No college staff.")?;
    if about as usize == prog || about as usize >= w.programs.len() {
        return Err("Pick another program.".into());
    }
    let i = crate::world::scouting::place(w, id).ok_or("No such recruit.")?;
    let d = w.today;
    if w.ground.race.claims.iter().any(|x| x.kid == id && x.by as usize == prog && x.about == about && x.topic == topic)
    {
        return Err("You told them that already.".into());
    }
    let school = w.programs[about as usize].real_name.clone();
    let words = claim_words(topic, &school);
    w.ground.race.claims.push(Claim {
        kid: id,
        by: prog as u32,
        man: None,
        about,
        topic,
        said: d,
        settles: settle_day(topic, d),
        held: None,
        words: words.clone(),
        named: w.programs[about as usize].head_coach.map(|c| c as u32),
    });
    let fam = people::family_of(w, i);
    let voice = people::family_voice(w, fam);
    Ok(format!("You told {voice}: \"{words}\" It settles on the facts."))
}

/// Mondays: a rival salesman in a kid's race runs you down to his family
/// (`run_down`), once a season a kid, when his staff is Known or better
/// with the kid's coach and he has visited the school this year. Your
/// staff hears it from the coach when the coach is Known or better with
/// you. It settles on the facts like your own.
pub(crate) fn run_down(w: &mut World, d: Date) {
    let Some(prog) = standing::seat_program(w) else { return };
    if d.weekday() != crate::calendar::Weekday::Mon {
        return;
    }
    let season = w.dates.season;
    let kids: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            let q = &w.people[i];
            q.stage == Stage::HighSchool
                && q.cal.signed.is_none()
                && (q.cal.early_offers.iter().any(|o| o.0 == prog)
                    || q.cal.offers.contains(&prog)
                    || q.cal.commit.is_some_and(|c| c.0 == prog))
        })
        .collect();
    for i in kids {
        let id = w.people[i].player.id;
        if w.ground.race.run.contains(&(id, season)) {
            continue;
        }
        let race = crate::world::front_office::college::race_at(w, i);
        let hs = w.people[i].hs;
        let Some(Who::Coach(c)) = people::head_coach_of(w, hs) else { continue };
        let year_start = d.add(-365);
        let mut pick: Option<(usize, u32)> = None;
        for &(p, _) in race.iter().take(RUN_DOWN_TOP) {
            if p == prog || standing::level(w, p, Who::Coach(c)) < Level::Known {
                continue;
            }
            let man = w.ground.ai.with(c).find_map(|(h, t)| match h {
                Holder::Coach(m) if t.seen >= year_start && visit_for(w, c, m, t.seen) == Some(p) => Some(m),
                _ => None,
            });
            if let Some(m) = man
                && super::staff::men::style(w, m as usize).0 == "salesman"
            {
                pick = Some((p, m));
                break;
            }
        }
        let Some((p, m)) = pick else { continue };
        // He picks the claim that sounds truest: the one that is true, else
        // your job.
        let topic =
            [Topic::Job, Topic::Depth, Topic::Scheme].into_iter().find(|&t| fact(w, prog, i, t)).unwrap_or(Topic::Job);
        let you = w.programs[prog].real_name.clone();
        let words = claim_words(topic, &you);
        w.ground.race.run.push((id, season));
        w.ground.race.claims.push(Claim {
            kid: id,
            by: p as u32,
            man: Some(m),
            about: prog as u32,
            topic,
            said: d,
            settles: settle_day(topic, d),
            held: None,
            words: words.clone(),
            named: w.programs[prog].head_coach.map(|c| c as u32),
        });
        // While it stands it weighs on the family's trust in you.
        let fam = people::family_of(w, i);
        standing::grant(
            w,
            prog,
            fam,
            -3.0,
            Some(format!("{}'s coach told us {}", w.programs[p].real_name, lower(&words))),
            d,
        );
        if standing::level(w, prog, Who::Coach(c)) >= Level::Known {
            let kid = w.people[i].name.clone();
            let body = vec![format!(
                "{}'s {} is telling {kid}'s family that {} It settles on the facts.",
                w.programs[p].real_name,
                last_name(&w.coaches[m as usize].name),
                lower(&words)
            )];
            standing::post_memo(
                w,
                Who::Coach(c),
                d,
                format!("{} is running you down to {kid}'s family", w.programs[p].real_name),
                body,
                1,
            );
        }
    }
    w.ground.race.run.retain(|x| x.1 >= season - 1);
}

fn lower(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Each day: claims whose day has come settle on the facts. A false one
/// costs whoever made it with the family (and the kid's coach hears);
/// a true one stands.
pub(crate) fn settle(w: &mut World, d: Date) {
    if standing::seat_program(w).is_none() {
        return;
    }
    let due: Vec<usize> = (0..w.ground.race.claims.len())
        .filter(|&k| w.ground.race.claims[k].held.is_none() && w.ground.race.claims[k].settles <= d)
        .collect();
    for k in due {
        let x = w.ground.race.claims[k].clone();
        let Some(i) = crate::world::scouting::place(w, x.kid) else {
            w.ground.race.claims[k].held = Some(true);
            continue;
        };
        let held = match x.topic {
            // A job claim holds if the head coach it named has gone.
            Topic::Job if x.named.is_some() => w.programs[x.about as usize].head_coach.map(|c| c as u32) != x.named,
            Topic::Job => {
                let named = w.coaches.iter().position(|c| {
                    c.job
                        .is_some_and(|j| j.role == Role::HeadCoach && j.employer == Employer::College(x.about as usize))
                });
                named.is_none() || w.programs[x.about as usize].hc_changed >= x.said.year()
            }
            t => fact(w, x.about as usize, i, t),
        };
        w.ground.race.claims[k].held = Some(held);
        let fam = people::family_of(w, i);
        let coach = people::head_coach_of(w, w.people[i].hs);
        let about = w.programs[x.about as usize].real_name.clone();
        if x.man.is_none() {
            // Your own claim: it costs the program that made it.
            let prog = x.by as usize;
            if !held {
                standing::grant(w, prog, fam, standing::WORD_BROKEN, Some(format!("You lied to us about {about}.")), d);
                if let Some(c) = coach {
                    standing::grant(
                        w,
                        prog,
                        c,
                        standing::WORD_BROKEN / 2.0,
                        Some(format!("You told a family a story about {about}.")),
                        d,
                    );
                }
            }
        } else if !held {
            // A rival's claim about you, false: the family remembers who
            // said it, and whom it was about.
            let prog = x.about as usize;
            let by = w.programs[x.by as usize].real_name.clone();
            standing::grant(
                w,
                prog,
                fam,
                standing::WORD_HELD,
                Some(format!("{by} said that about you. It wasn't true.")),
                d,
            );
            if let Some(Who::Coach(c)) = coach {
                for holder in [Holder::Program(x.by), Holder::Coach(x.man.unwrap_or(u32::MAX))] {
                    if let Some(t) = w.ground.ai.ties.0.get_mut(&ai::tie_key(c, holder)) {
                        t.points = (t.points + standing::WORD_BROKEN).max(0.0);
                    }
                }
            }
        }
    }
    let cut = d.add(-3 * 365);
    w.ground.race.claims.retain(|x| x.held.is_none() || x.settles >= cut);
}

// --- Rumors of money outside the rules ------------------------------------------------------

/// Signing day, after the rogue boosters pick their targets (`ncaa`): a
/// kid on your board who is someone's target is a rumor when a person who
/// trusts you is close to him. A Trusted coach or family says "somebody is
/// taking care of the family"; a Home one names the school. Nothing you
/// can prove: only a case proves it (`ncaa::enforcement`).
pub(crate) fn rumors(w: &mut World) {
    let Some(prog) = standing::seat_program(w) else { return };
    let d = w.today;
    let mut heard: Vec<(Who, String, Vec<String>, u64)> = Vec::new();
    for (j, p) in w.programs.iter().enumerate() {
        if j == prog {
            continue;
        }
        let Some(r) = p.era.rogue.as_ref() else { continue };
        for &(i, _) in &r.targets {
            let q = &w.people[i];
            let yours = q.cal.early_offers.iter().any(|o| o.0 == prog)
                || q.cal.offers.contains(&prog)
                || q.cal.commit.is_some_and(|c| c.0 == prog);
            if !yours {
                continue;
            }
            let fam = people::family_of(w, i);
            let coach = people::head_coach_of(w, q.hs);
            let lv = [Some(fam), coach]
                .into_iter()
                .flatten()
                .map(|who| standing::level(w, prog, who))
                .max()
                .unwrap_or(Level::Stranger);
            if lv < Level::Trusted {
                continue;
            }
            let k = super::key(&[0x8A1E, q.player.id as u64, j as u64]);
            if w.ground.race.rumors.contains(&k) {
                continue;
            }
            let line = if lv == Level::Home {
                format!("Word is it's {} money. Nobody will say it on the record.", p.real_name)
            } else {
                "Word is somebody's taking care of the family. Nobody will say who.".into()
            };
            heard.push((coach.unwrap_or(fam), q.name.clone(), vec![line], k));
        }
    }
    for (who, name, body, k) in heard {
        w.ground.race.rumors.push(k);
        standing::post_memo(w, who, d, format!("A rumor about {name}"), body, 1);
    }
}

/// Your seat's race each day: claims settle; on Mondays your commits are
/// watched and rivals run you down.
pub(crate) fn day(w: &mut World, d: Date) {
    if standing::seat_program(w).is_none() {
        return;
    }
    settle(w, d);
    watch(w, d);
    run_down(w, d);
}
