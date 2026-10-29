//! Your read of any coach (C24, C25, C35, C40, C41, C43, C58): every coach
//! in the world has a Ratings tab, your read as ranges on the hidden 1-20
//! scale, never the number. A stranger's read is the whole scale (C40).
//! It narrows with what you put in, whoever he is (C41): hours on his
//! film, talking to the people around him, an interview (each question
//! reads its own ratings, C25), an honest word from someone who knows him
//! for a favour (C43), and how well you know him yourself (your network,
//! the Hiring tab's widths, C24). A coordinator on your staff who is
//! strong in a group halves the width of your read of that group's
//! teaching at 18 (C58). Your read of his potential (C35) is words from
//! your read of his hidden learning ceiling, slower to narrow.
//!
//! Your own staff keep the season-by-season read of `staff_read`.
//!
//! Sizes are Claude's picks (proposed, not confirmed) where the list gives
//! none: a read is 20 / (1 + 0.9 × evidence) wide, so ten hours on the
//! things that show it make it about 2 wide; film shows what his unit does
//! (his group's teaching, scheme, calling, game management), talk shows the
//! man (relationships, discipline, motivation, recruiting, adaptability,
//! ambition); a graduate assistant's or analyst's film shows little.

use super::{Team, coach_link, network, talks};
use crate::calendar::Date;
use crate::rng::clamp;
use crate::world::World;
use crate::world::coach::{Employer, GROUP_COUNT, Group, Role};
use std::collections::HashMap;

/// What you've put into one coach.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Look {
    pub coach: usize,
    /// Hours on his film.
    pub film: f32,
    /// Hours talking to the people around him.
    pub talk: f32,
    /// Honest reads you asked for (C43).
    pub asked: Vec<Asked>,
    /// Interview answers (C25): facet code and the read it gave, 1-20.
    pub said: Vec<(u8, f32)>,
    pub last: Option<Date>,
}

/// An honest read someone gave you (C43).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Asked {
    pub who: String,
    pub date: Date,
    /// How straight it was, 0-1.
    pub straight: f32,
    /// Which way his agenda leaned it, in rating points (+ oversold).
    pub lean: f32,
    pub words: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Looks {
    pub list: Vec<Look>,
}

/// What your reads need from your side, gathered once for a whole list
/// (the Hiring search reads thousands of coaches).
pub struct Ctx<'a> {
    warm: HashMap<usize, f32>,
    looks: HashMap<usize, &'a Look>,
    eye: [f64; GROUP_COUNT],
}

impl<'a> Ctx<'a> {
    pub fn new(w: &'a World) -> Ctx<'a> {
        let warm = w.folk.network.contacts.iter().filter_map(|x| x.coach_id().map(|c| (c, x.warmth))).collect();
        let looks = w.folk.looks.list.iter().map(|l| (l.coach, l)).collect();
        let eye = std::array::from_fn(|g| staff_eye(w, Group::ALL[g]));
        Ctx { warm, looks, eye }
    }

    /// He's in your network, or you've put time into him.
    pub fn known(&self, c: usize) -> bool {
        self.warm.contains_key(&c) || self.looks.contains_key(&c)
    }

    pub fn warmth(&self, c: usize) -> f32 {
        self.warm.get(&c).copied().unwrap_or(0.0)
    }
}

/// The facets of any coach's read, in their order on his page.
pub const FACETS: [(&str, &str, &str); 17] = [
    ("teach:0", "Teaching, quarterbacks", "Teaching"),
    ("teach:1", "Teaching, running backs", "Teaching"),
    ("teach:2", "Teaching, receivers", "Teaching"),
    ("teach:3", "Teaching, offensive line", "Teaching"),
    ("teach:4", "Teaching, defensive line", "Teaching"),
    ("teach:5", "Teaching, linebackers", "Teaching"),
    ("teach:6", "Teaching, defensive backs", "Teaching"),
    ("teach:7", "Teaching, special teams", "Teaching"),
    ("scheme", "Scheme", "Coaching"),
    ("gm", "Game management", "Game day"),
    ("calling", "Calling", "Game day"),
    ("motivation", "Motivation", "Coaching"),
    ("discipline", "Discipline", "Coaching"),
    ("relationships", "Relationships", "Recruiting"),
    ("recruiting", "Recruiting", "Recruiting"),
    ("network", "Network", "Recruiting"),
    ("adapt", "Adaptability", "Coaching"),
];
/// Ambition is read too (the interview's "what he wants"), shown as words.
pub const AMBITION: usize = FACETS.len();

pub fn facet_index(key: &str) -> Option<usize> {
    if key == "ambition" {
        return Some(AMBITION);
    }
    FACETS.iter().position(|f| f.0 == key)
}

/// A coach's true rating for a facet (the read's centre).
pub(crate) fn truth(w: &World, c: usize, f: usize) -> f64 {
    let co = &w.coaches[c];
    let a = co.ability();
    match f {
        0..=7 => a.teaching[f],
        8 => a.scheme,
        9 => a.game_management,
        10 => co.calling(),
        11 => a.motivation,
        12 => a.discipline,
        13 => a.relationships,
        14 => a.recruiting,
        15 => a.network,
        16 => co.adaptability(),
        _ => co.personality.ambition,
    }
}

/// The groups his job shows on film: his own room, a coordinator's side, a
/// head coach's whole team (thinly), a junior's room (barely).
fn on_film(w: &World, c: usize) -> [f64; GROUP_COUNT] {
    let co = &w.coaches[c];
    let mut v = [0.0; GROUP_COUNT];
    match co.job.map(|j| j.role) {
        Some(Role::PositionCoach(g)) => v[g as usize] = 1.0,
        Some(r @ (Role::OffensiveCoordinator | Role::DefensiveCoordinator)) => {
            let off = r == Role::OffensiveCoordinator;
            for g in Group::ALL.iter().filter(|g| g.offense() == off && **g != Group::Specialists) {
                v[*g as usize] = if *g == co.came_up { 0.8 } else { 0.5 };
            }
        }
        Some(Role::HeadCoach) => {
            // A high school head coach coaches every group himself.
            let k = if matches!(co.job.map(|j| j.employer), Some(Employer::HighSchool(_))) { 0.6 } else { 0.3 };
            v = [k; GROUP_COUNT];
            v[co.came_up as usize] = k + 0.3;
        }
        None => {
            if let Some(j) = co.junior_now() {
                v[j.group as usize] = 0.2;
            }
        }
    }
    v
}

/// Evidence units for one facet of one coach, and the bias the sources put
/// on its centre (rating points, already weighted).
fn evidence(w: &World, x: &Ctx, c: usize, f: usize) -> (f64, f64) {
    let co = &w.coaches[c];
    let warm = x.warmth(c) as f64;
    let known = x.warm.contains_key(&c);
    // Your network (C24): the Hiring tab's widths, 5.5 - 4 x warmth wide.
    let net = if known { (20.0 / (5.5 - 4.0 * warm).max(1.2) - 1.0) / 0.9 } else { 0.0 };
    let teach_seen = |g: usize| {
        g == co.came_up as usize
            || co.job.is_some_and(|j| j.role == Role::PositionCoach(Group::ALL[g]))
            || co.history.iter().any(|s| s.role == Role::PositionCoach(Group::ALL[g]))
    };
    let film_k = on_film(w, c);
    let calls = matches!(
        co.job.map(|j| j.role),
        Some(Role::HeadCoach | Role::OffensiveCoordinator | Role::DefensiveCoordinator)
    );
    let (film_w, talk_w) = match f {
        0..=7 => (film_k[f], if teach_seen(f) { 0.5 } else { 0.15 }),
        8 => (if co.junior_now().is_some() { 0.3 } else { 0.8 }, 0.3),
        9 | 10 => (if calls { 0.9 } else { 0.1 }, 0.15),
        11 | 12 => (0.3, 1.0),
        13 => (0.1, 1.0),
        14 | 15 => (0.05, 0.8),
        16 => (0.1, 0.8),
        _ => (0.0, 0.8),
    };
    let net_f = if f < GROUP_COUNT && !teach_seen(f) { 0.5 * net } else { net };
    let look = x.looks.get(&c);
    let mut e = net_f;
    let mut bias = 0.0;
    if let Some(l) = look {
        e += film_w * l.film as f64 + talk_w * l.talk as f64;
        // An interview reads its own ratings (C25), its centre where the
        // man talked it to.
        for &(code, read) in l.said.iter().filter(|x| x.0 as usize == f) {
            let _ = code;
            e += 3.0;
            bias += 3.0 * (read as f64 - truth(w, c, f));
        }
        for a in &l.asked {
            let k = if f < GROUP_COUNT && !teach_seen(f) { 1.5 } else { 4.0 };
            e += k * (0.5 + 0.5 * a.straight as f64);
            bias += k * a.lean as f64;
        }
    }
    // C58: a coordinator on your staff strong in the group sharpens your
    // read of its teaching, to half as wide at 18 (folded in as evidence).
    if f < GROUP_COUNT {
        let eye = x.eye[f];
        let k = 1.0 - 0.5 * clamp((eye - 10.0) / 8.0, 0.0, 1.0);
        // width x k: 20 / (1 + 0.9 e') = k x 20 / (1 + 0.9 e)
        e = ((1.0 + 0.9 * e) / k - 1.0) / 0.9;
    }
    (e, bias / (1.0 + e))
}

/// The best teaching of a group among the coordinators on your staff (C58).
pub(crate) fn staff_eye(w: &World, g: Group) -> f64 {
    let Some(team) = Team::seat(w) else { return 0.0 };
    let (oc, dc) = match team {
        Team::College(p) => (w.programs[p].oc, w.programs[p].dc),
        Team::Nfl(t) => (w.nfl[t].oc, w.nfl[t].dc),
        Team::High(h) => (w.high_schools[h].oc, w.high_schools[h].dc),
        Team::Other(_) => (None, None),
    };
    [oc, dc].into_iter().flatten().map(|c| w.coaches[c].teaching(g)).fold(0.0, f64::max)
}

/// Your lean as a reader of him: -1 to 1, fixed for you, him and the facet.
fn lean(w: &World, c: usize, f: usize) -> f64 {
    let reader = w.career.as_ref().map_or(0, |k| (k.coach as u64) << 32 ^ k.seed as u64);
    2.0 * crate::world::season::key_uniform(reader ^ 0x100C_C0AC ^ (c as u64).rotate_left(21), f as u64 ^ 0x9E37) - 1.0
}

/// The width of a read from its evidence.
pub fn width(e: f64) -> f64 {
    20.0 / (1.0 + 0.9 * e.max(0.0))
}

/// Your read of one facet of any coach: low and high on 1-20.
pub fn read(w: &World, c: usize, f: usize) -> (u8, u8) {
    read_in(w, &Ctx::new(w), c, f)
}

/// `read` with your side gathered once.
pub fn read_in(w: &World, x: &Ctx, c: usize, f: usize) -> (u8, u8) {
    let (e, bias) = evidence(w, x, c, f);
    let wide = width(e);
    let half = wide / 2.0;
    let mid = truth(w, c, f) + bias + lean(w, c, f) * half * 0.6;
    // The window keeps its width inside the scale, so a stranger's read is
    // the whole scale and a wide read's place tells nothing it shouldn't.
    let lo = (mid - half).min(20.0 - wide).max(1.0);
    let hi = (lo + wide).min(20.0);
    (lo.round() as u8, (hi.round() as u8).max(lo.round() as u8))
}

/// Your read of the teaching a job needs: a group, or a coordinator's side.
pub fn read_job(w: &World, c: usize, role: Role) -> (u8, u8) {
    read_job_in(w, &Ctx::new(w), c, role)
}

pub fn read_job_in(w: &World, x: &Ctx, c: usize, role: Role) -> (u8, u8) {
    match role {
        Role::PositionCoach(g) => read_in(w, x, c, g as usize),
        Role::OffensiveCoordinator | Role::DefensiveCoordinator => {
            let off = role == Role::OffensiveCoordinator;
            let gs: Vec<(u8, u8)> = Group::ALL
                .iter()
                .filter(|g| g.offense() == off && **g != Group::Specialists)
                .map(|g| read_in(w, x, c, *g as usize))
                .collect();
            let n = gs.len().max(1) as f64;
            let lo = gs.iter().map(|x| x.0 as f64).sum::<f64>() / n;
            let hi = gs.iter().map(|x| x.1 as f64).sum::<f64>() / n;
            (lo.round() as u8, hi.round() as u8)
        }
        Role::HeadCoach => {
            let s = read_in(w, x, c, 8);
            let m = read_in(w, x, c, 11);
            (((s.0 as u32 + m.0 as u32) / 2) as u8, ((s.1 as u32 + m.1 as u32) / 2) as u8)
        }
    }
}

/// Your read of his potential (C35), in words: from your read of his
/// hidden learning ceiling, slower to narrow than his ratings (half the
/// evidence), never the number.
pub fn potential(w: &World, c: usize) -> (String, &'static str) {
    potential_in(w, &Ctx::new(w), c)
}

/// Your read of his ceiling: centre, half width, and his best now.
fn ceiling_read(w: &World, x: &Ctx, c: usize) -> (f64, f64, f64) {
    let co = &w.coaches[c];
    let e = (0..FACETS.len()).map(|f| evidence(w, x, c, f).0).sum::<f64>() / FACETS.len() as f64 * 0.5;
    let half = width(e) / 2.0;
    let mid = co.ceiling() + lean(w, c, 40) * half * 0.6;
    let a = co.ability();
    let best = {
        let mut t = a.teaching;
        t.sort_by(|x, y| y.total_cmp(x));
        (t[0] + t[1] + a.scheme) / 3.0
    };
    (mid, half, best)
}

fn peaked(w: &World, c: usize, (mid, half, best): (f64, f64, f64)) -> bool {
    w.coaches[c].age >= 58 || (mid + half - best < 0.8 && half <= 2.5)
}

/// The potential words' place on their scale, for sorting what you see:
/// 0 when it's too early to say, 0.5 for a man who has peaked.
pub fn potential_rank(w: &World, x: &Ctx, c: usize) -> f64 {
    let r = ceiling_read(w, x, c);
    let level = |v: f64| [10.0, 12.0, 13.5, 15.0, 16.5].iter().filter(|t| v >= **t).count() as f64 + 1.0;
    if peaked(w, c, r) {
        0.5
    } else if r.1 > 4.0 {
        0.0
    } else if r.1 > 1.5 {
        (level(r.0 - r.1) + level(r.0 + r.1)) / 2.0
    } else {
        level(r.0)
    }
}

pub fn potential_in(w: &World, x: &Ctx, c: usize) -> (String, &'static str) {
    let co = &w.coaches[c];
    let (mid, half, best) = ceiling_read(w, x, c);
    let side = if co.came_up.offense() { "offense" } else { "defense" };
    let word = |v: f64| -> String {
        if v >= 16.5 {
            "could be one of the best in the game".into()
        } else if v >= 15.0 {
            format!("could run a power program's {side}")
        } else if v >= 13.5 {
            "could coordinate in the FBS".into()
        } else if v >= 12.0 {
            "could coordinate in the FCS or a smaller FBS school".into()
        } else if v >= 10.0 {
            "a solid position coach in the making".into()
        } else {
            "limited room to grow".into()
        }
    };
    let sure = if half <= 1.5 {
        "Sure"
    } else if half <= 2.5 {
        "Fairly sure"
    } else if half <= 4.0 {
        "Not sure"
    } else {
        "Too early"
    };
    // Learning stops at 58; a man at his ceiling has peaked.
    if peaked(w, c, (mid, half, best)) {
        return ("Has peaked".into(), sure);
    }
    let text = if half > 4.0 {
        "Too early to say: you've barely seen him".to_string()
    } else if half > 1.5 {
        let (lo, hi) = (word(mid - half), word(mid + half));
        if lo == hi { cap(&lo) } else { format!("{}, maybe {}", cap(&lo), hi.trim_start_matches("could ")) }
    } else {
        cap(&word(mid))
    };
    let young = co.age < 45 && mid - best > 2.0;
    (if young { format!("{text}. Still learning fast") } else { text }, sure)
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
}

fn look_mut(w: &mut World, c: usize) -> &mut Look {
    let today = w.today;
    let list = &mut w.folk.looks.list;
    let k = match list.iter().position(|l| l.coach == c) {
        Some(k) => k,
        None => {
            if list.len() >= 400 {
                // The coach you looked at longest ago drops off.
                if let Some(o) = list.iter().enumerate().min_by_key(|x| x.1.last.map_or(0, |d| d.0)).map(|x| x.0) {
                    list.remove(o);
                }
            }
            list.push(Look { coach: c, ..Default::default() });
            list.len() - 1
        }
    };
    list[k].last = Some(today);
    &mut list[k]
}

/// Records an interview answer's read of a facet (C25).
pub(crate) fn heard(w: &mut World, c: usize, f: usize, read: f32) {
    let l = look_mut(w, c);
    match l.said.iter_mut().find(|x| x.0 as usize == f) {
        Some(x) => x.1 = 0.5 * (x.1 + read),
        None => l.said.push((f as u8, read)),
    }
}

/// Watch his film (C24, C41): two hours of your week.
pub fn watch_film(w: &mut World, c: usize) -> Result<String, String> {
    w.coaches.get(c).ok_or("No such coach.")?;
    if !talks::spend(w, 2.0) {
        return Err("Your week is full: two hours of film won't fit.".into());
    }
    let l = look_mut(w, c);
    l.film += 2.0;
    let hours = l.film;
    let co = &w.coaches[c];
    let what = match (co.job, co.junior_now()) {
        (Some(j), _) => match j.role {
            Role::PositionCoach(g) => format!("his {} in individual periods and on game days", g.name()),
            Role::HeadCoach => "his team's games, all three phases".into(),
            _ => format!(
                "his {}'s games and practice cut-ups",
                if j.role == Role::OffensiveCoordinator { "offense" } else { "defense" }
            ),
        },
        (None, Some(_)) => "the practice tape he's on, which shows little of him".into(),
        (None, None) => "his last job's games".into(),
    };
    Ok(format!("You watched two hours of {what} ({hours:.0} hours on {} so far).", coach_link(w, c)))
}

/// Talk to the people around him (C41): his players, his colleagues. An
/// hour of your week.
pub fn ask_around(w: &mut World, c: usize) -> Result<String, String> {
    w.coaches.get(c).ok_or("No such coach.")?;
    if !talks::spend(w, 1.0) {
        return Err("Your week is full: an hour of calls won't fit.".into());
    }
    look_mut(w, c).talk += 1.0;
    let co = &w.coaches[c];
    let a = co.ability();
    let n = super::roll(w, 0xA5C1 ^ w.today.0 as u64, c as u64);
    // What they say leans on the truth, through the usual noise of people.
    let rel = a.relationships + 3.0 * (n - 0.5);
    let line = if rel >= 14.0 {
        "His players would run through a wall for him; a former player calls him the reason he stayed in school."
    } else if rel >= 10.0 {
        "People who've worked with him say he's steady, a grinder, good in the room."
    } else if rel >= 7.0 {
        "You hear he's demanding and doesn't always explain himself."
    } else {
        "Two of his former players go quiet when you bring him up."
    };
    Ok(format!("You made calls about {}. {line}", coach_link(w, c)))
}

/// Who can give you an honest read of a coach (C43): his head coach, his
/// athletic director (college), an old staff-mate you know. Each: label,
/// the person (a coach, or an AD by program), and how warm you are.
pub fn sources(w: &World, c: usize) -> Vec<(String, Source, f32)> {
    let co = &w.coaches[c];
    let mut v = Vec::new();
    let emp = co.job.map(|j| j.employer).or(co.junior_now().map(|j| j.employer));
    let boss = emp.and_then(|e| match e {
        Employer::College(p) => w.programs[p].head_coach,
        Employer::Nfl(t) => w.nfl[t].head_coach,
        Employer::HighSchool(h) => w.high_schools[h].coach,
        Employer::Other(f) => crate::world::pro::other::head_coach(w, f),
    });
    if let Some(b) = boss.filter(|&b| b != c && Some(b) != w.career.as_ref().map(|k| k.coach)) {
        v.push((format!("His head coach, {}", w.coaches[b].name), Source::Coach(b), network::warmth(w, b)));
    }
    if let Some(Employer::College(p)) = emp
        && let Some(ad) = &w.programs[p].ad
    {
        let o = network::Other::Ad(p, ad.since);
        v.push((format!("His athletic director, {}", ad.name), Source::Ad(p, ad.since), network::warmth_other(w, o)));
    }
    // An old staff-mate: someone in your network who worked beside him.
    let mates: Vec<(usize, f32)> = w
        .folk
        .network
        .contacts
        .iter()
        .filter_map(|x| x.coach_id().map(|k| (k, x.warmth)))
        .filter(|&(k, _)| k != c && Some(k) != boss && worked_with(w, k, c))
        .collect();
    if let Some(&(k, warm)) = mates.iter().max_by(|a, b| a.1.total_cmp(&b.1)) {
        v.push((format!("An old staff-mate, {}", w.coaches[k].name), Source::Coach(k), warm));
    }
    v
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Coach(usize),
    Ad(usize, i32),
}

impl Source {
    pub fn code(self) -> String {
        match self {
            Source::Coach(k) => format!("c{k}"),
            Source::Ad(p, y) => format!("a{p}.{y}"),
        }
    }
    pub fn parse(s: &str) -> Option<Source> {
        if let Some(k) = s.strip_prefix('c') {
            return k.parse().ok().map(Source::Coach);
        }
        let (p, y) = s.strip_prefix('a')?.split_once('.')?;
        Some(Source::Ad(p.parse().ok()?, y.parse().ok()?))
    }
}

/// Whether two coaches were on the same staff in the same season.
fn worked_with(w: &World, a: usize, b: usize) -> bool {
    let spans = |c: usize| {
        let co = &w.coaches[c];
        let mut v: Vec<(Employer, i32, i32)> = co.history.iter().map(|s| (s.employer, s.from, s.to)).collect();
        if let Some(j) = co.job {
            v.push((j.employer, j.since, w.year));
        }
        v
    };
    let (x, y) = (spans(a), spans(b));
    x.iter().any(|p| y.iter().any(|q| p.0 == q.0 && p.1 <= q.2 && q.1 <= p.2))
}

/// Ask someone who knows him for an honest read (C43), for a favour he
/// can call in later. How straight it is follows your tie to him and his
/// own agenda: a head coach or athletic director who means to keep his man
/// undersells him; an old friend of his talks him up.
pub fn ask_for_read(w: &mut World, c: usize, src: Source) -> Result<String, String> {
    let list = sources(w, c);
    let (label, _, warm) = list.into_iter().find(|x| x.1 == src).ok_or("He can't speak to that coach.")?;
    if !talks::spend(w, 0.5) {
        return Err("Your week is full.".into());
    }
    let co = &w.coaches[c];
    let keep = match src {
        // His boss wants to keep him: the better he is, the more he
        // undersells; a man he'd let go he talks up.
        Source::Coach(b) if co.job.is_some_and(|j| Some(b) == boss_of(w, j.employer)) => -1.0,
        Source::Ad(..) => -0.6,
        _ => 0.7,
    };
    let honest = (super::roll(w, 0x4E57 ^ c as u64, w.today.0 as u64) as f32 * 0.3 + 0.35 + 0.45 * warm).min(1.0);
    let lean = keep * (1.0 - honest) * 3.0;
    let a = co.ability();
    let best = a.teaching.iter().cloned().fold(0.0, f64::max).max(a.scheme) + lean as f64;
    let words = if best >= 16.0 {
        "He's as good as anyone I've worked with. You'd be lucky to get him."
    } else if best >= 13.0 {
        "He's a good coach. Knows his stuff, works hard."
    } else if best >= 10.0 {
        "He's fine. Does his job."
    } else {
        "I'd look elsewhere, between us."
    };
    let name = w.coaches[c].name.clone();
    let today = w.today;
    look_mut(w, c).asked.push(Asked { who: label.clone(), date: today, straight: honest, lean, words: words.into() });
    // The favour (C43): you owe him one, on the ledger.
    match src {
        Source::Coach(k) => {
            network::touch(w, k, 0.02);
            network::owe(w, network::Whom::Coach(k), format!("an honest read of {name}"));
        }
        Source::Ad(p, y) => {
            network::touch_other(w, network::Other::Ad(p, y), 0.02, "a call about a coach");
            network::owe(w, network::Whom::Other(network::Other::Ad(p, y)), format!("an honest read of {name}"));
        }
    }
    Ok(format!("{label}: \"{words}\" You owe him one."))
}

fn boss_of(w: &World, e: Employer) -> Option<usize> {
    match e {
        Employer::College(p) => w.programs[p].head_coach,
        Employer::Nfl(t) => w.nfl[t].head_coach,
        Employer::HighSchool(h) => w.high_schools[h].coach,
        Employer::Other(f) => crate::world::pro::other::head_coach(w, f),
    }
}

/// What you've put into a coach, for his page.
pub fn invested(w: &World, c: usize) -> Option<&Look> {
    w.folk.looks.list.iter().find(|l| l.coach == c)
}
