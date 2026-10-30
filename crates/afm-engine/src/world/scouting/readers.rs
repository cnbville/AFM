//! Who reads players for a staff (topic 2), and the evidence a file is
//! built from (topic 1): an NFL club's personnel department (its scouts,
//! the GM, the head coach and coordinators, its doctors and security), a
//! college program's on-field coaches and personnel staff, a high school
//! head coach and his assistants. Each reader is rebuilt from the world
//! when a file is read; a look stores only a code for who took it.

use super::file::{Evidence, Link, Reader};
use super::{Kind, SeatLook};
use crate::rng::{Rng, clamp};
use crate::world::coach::{GROUP_COUNT, Group};
use crate::world::pro::scouts::{self, DC, EVENT, GM, HC, OC, Role};
use crate::world::season::key_uniform;
use crate::world::{Stage, World};

/// Whose files: an NFL club, a college program or a high school.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Staff {
    Club(usize),
    Program(usize),
    School(usize),
}

/// Reader codes beyond a club scout's place (0-249) and the built GM, HC,
/// OC, DC and event codes (250-255).
pub const MEDICAL: u32 = 260;
pub const SECURITY: u32 = 261;
pub const ANALYTICS: u32 = 262;
pub const ACADEMIC: u32 = 263;
/// A college program's personnel staffer k.
pub const PERSONNEL: u32 = 1 << 18;
/// A high school assistant k.
pub const HS_ASSISTANT: u32 = 1 << 19;
/// A coach by his index in the world's coaches.
pub const COACH: u32 = 1 << 20;
/// A pro scout k in your pro department.
pub const PRO_SCOUT: u32 = 1 << 17;

/// Your seat's staff.
pub fn seat_staff(w: &World) -> Option<Staff> {
    let s = w.seat?;
    match s.level {
        crate::calendar::Level::Nfl => s.nfl_team.map(Staff::Club),
        crate::calendar::Level::HighSchool => s.program.map(Staff::School),
        _ => s.program.map(Staff::Program),
    }
}

/// Your own coach's index, if you have a career.
pub fn you(w: &World) -> Option<usize> {
    w.career.as_ref().map(|c| c.coach)
}

/// A college program's personnel staff (topic 2): a director of player
/// personnel, a recruiting director, analysts and (from 2023, at the big
/// programs) a general manager. They evaluate film and work on campus;
/// only the on-field coaches go out. Sized by division and money; each
/// post's holder lasts about five years (staggered), kept by key so the
/// same people are there every time the screen opens.
#[derive(Clone, Debug)]
pub struct Personnel {
    pub name: String,
    pub job: &'static str,
    pub judge: f64,
    pub potential: f64,
    pub athletic: f64,
    pub character: f64,
    pub network: f64,
    pub work: f64,
    pub since: i32,
    pub key: u64,
}

pub fn personnel(w: &World, prog: usize) -> Vec<Personnel> {
    // Your program's staff, once you have hired or let go (SK7).
    if matches!(seat_staff(w), Some(Staff::Program(q)) if q == prog) && !w.scouting.own.personnel.is_empty() {
        return w
            .scouting
            .own
            .personnel
            .iter()
            .flatten()
            .map(|h| Personnel {
                name: h.name.clone(),
                job: super::own::PERSONNEL_JOBS[h.job as usize % super::own::PERSONNEL_JOBS.len()],
                judge: h.judge as f64,
                potential: h.potential as f64,
                athletic: h.athletic as f64,
                character: h.character as f64,
                network: h.network as f64,
                work: h.work as f64,
                since: h.since,
                key: h.key,
            })
            .collect();
    }
    keyed_personnel(w, prog)
}

/// The staff a program has by key, before you change it.
pub fn keyed_personnel(w: &World, prog: usize) -> Vec<Personnel> {
    use crate::world::data::Division;
    let p = &w.programs[prog];
    // Facilities and prestige are both kept on 0-1 (F2: they were read as
    // 0-20 and 0-100, so no staff ever grew past three posts).
    let money = clamp(0.4 * p.facilities + 0.6 * p.prestige, 0.0, 1.0);
    let mut jobs: Vec<&'static str> = match p.division {
        Division::FBS => {
            let mut v = vec!["Director of player personnel", "Director of recruiting", "Recruiting analyst"];
            if money > 0.45 {
                v.push("Assistant director of personnel");
                v.push("Recruiting analyst");
            }
            if money > 0.65 {
                v.push("Recruiting analyst");
                v.push("On-campus recruiting coordinator");
            }
            if w.year >= 2023 && money > 0.55 {
                v.insert(0, "General manager");
            }
            v
        }
        Division::FCS => vec!["Director of recruiting", "Recruiting analyst"],
        _ => vec!["Recruiting coordinator"],
    };
    if w.year < 2012 {
        // Personnel departments grew from about 2012 (Alabama, then everyone).
        jobs.truncate(if p.division == Division::FBS { 2 } else { 1 });
    }
    jobs.into_iter()
        .enumerate()
        .map(|(k, job)| {
            let block = (w.year + 2 * k as i32) / 5;
            let since = (block * 5 - 2 * k as i32).max(w.first_year());
            let key = (prog as u64) << 32 ^ (k as u64) << 24 ^ block as u64 ^ 0x9E50;
            let mut rng = Rng::new((key ^ (key >> 29)) as u32 ^ w.options.seed);
            let mean = 8.0 + 4.0 * money;
            let mut r = || clamp(rng.normal(mean, 2.8), 1.0, 20.0).round();
            let (judge, potential, athletic, character, network, work) = (r(), r(), r(), r(), r(), r());
            let eth = crate::world::pro::scouts::staff_ethnicity(&mut rng, w.year);
            let born = w.year - rng.int(24, 50);
            let name = crate::world::names::person_name(eth, born, &mut rng);
            Personnel { name, job, judge, potential, athletic, character, network, work, since, key }
        })
        .collect()
}

/// A coach as a reader: his evaluation eye (topic 14), his teaching of the
/// player's group, his relationships for the character side.
fn coach_reader(w: &World, c: usize, job: String, group: usize) -> Reader {
    let co = &w.coaches[c];
    let a = co.ability();
    let eval = co.evaluation();
    let teach = a.teaching[group.min(GROUP_COUNT - 1)];
    Reader {
        key: co.id as u64 ^ 0xC0AC_0000,
        name: co.name.clone(),
        job,
        link: if you(w) == Some(c) { Link::You } else { Link::Coach(c) },
        judge: 0.6 * eval + 0.4 * teach,
        potential: 0.7 * eval + 0.3 * teach,
        athletic: 0.5 * eval + 0.5 * teach,
        character: a.relationships,
        network: a.network,
        work: 0.5 * a.recruiting + 5.0,
        say: if you(w) == Some(c) { 1.5 } else { 1.0 },
        bias: None,
        home: 99,
        contacts: None,
    }
}

/// The job words for a coach at a program.
fn college_job(w: &World, prog: usize, c: usize) -> String {
    let p = &w.programs[prog];
    if p.head_coach == Some(c) {
        "Head coach".into()
    } else if p.oc == Some(c) {
        "Offensive coordinator".into()
    } else if p.dc == Some(c) {
        "Defensive coordinator".into()
    } else if let Some(g) = p.position_coaches.iter().position(|x| *x == Some(c)) {
        format!("{} coach", Group::ALL[g].name())
    } else {
        "Assistant coach".into()
    }
}

/// One reader by code, for a staff and a player's group.
pub fn reader(w: &World, staff: Staff, who: u32, group: usize) -> Option<Reader> {
    if who == EVENT as u32 {
        return Some(Reader::event("Measured"));
    }
    if who >= COACH {
        let c = (who - COACH) as usize;
        if c >= w.coaches.len() {
            return None;
        }
        let job = match staff {
            Staff::Program(p) => college_job(w, p, c),
            Staff::School(_) => "Head coach".into(),
            Staff::Club(t) => {
                let tm = &w.nfl[t];
                if tm.head_coach == Some(c) {
                    "Head coach".into()
                } else if tm.oc == Some(c) {
                    "Offensive coordinator".into()
                } else if tm.dc == Some(c) {
                    "Defensive coordinator".into()
                } else {
                    "Coach".into()
                }
            }
        };
        return Some(coach_reader(w, c, job, group));
    }
    match staff {
        Staff::Club(t) => club_reader(w, t, who, group),
        Staff::Program(prog) => {
            if (PERSONNEL..HS_ASSISTANT).contains(&who) {
                let k = (who - PERSONNEL) as usize;
                let p = personnel(w, prog).into_iter().nth(k)?;
                Some(Reader {
                    key: p.key,
                    name: p.name,
                    job: p.job.to_string(),
                    link: Link::Staff,
                    judge: p.judge,
                    potential: p.potential,
                    athletic: p.athletic,
                    character: p.character,
                    network: p.network,
                    work: p.work,
                    say: 0.9,
                    bias: None,
                    home: 99,
                    contacts: None,
                })
            } else if who == ACADEMIC {
                let s = &w.programs[prog].support.academic;
                Some(staffer_reader(&s.name, "Academic support", s.rating, prog as u64 ^ 0xACAD))
            } else if who == ANALYTICS {
                let s = &w.programs[prog].support.analytics;
                Some(numbers_reader(&s.name, "Director of football analytics", s.rating, prog as u64 ^ 0xA7A))
            } else if who == MEDICAL {
                let s = &w.programs[prog].support.medical;
                Some(staffer_reader(&s.name, "Head trainer", s.rating, prog as u64 ^ 0x3ED))
            } else {
                None
            }
        }
        Staff::School(_) => {
            if (HS_ASSISTANT..COACH).contains(&who) {
                let k = (who - HS_ASSISTANT) as usize;
                let a = w.career.as_ref()?.hs_staff.get(k)?;
                let job = match a.coordinator {
                    Some(true) => "Offensive coordinator".to_string(),
                    Some(false) => "Defensive coordinator".to_string(),
                    None => format!("{} coach", Group::ALL[(a.group as usize).min(GROUP_COUNT - 1)].name()),
                };
                Some(Reader {
                    key: (k as u64) << 40 ^ a.since as u64 ^ 0x45A5,
                    name: a.name.clone(),
                    job,
                    link: Link::Staff,
                    judge: a.teaching,
                    potential: 0.8 * a.teaching + 1.0,
                    athletic: a.teaching,
                    character: 12.0,
                    network: 8.0,
                    work: 10.0,
                    say: 0.8,
                    bias: None,
                    home: 99,
                    contacts: None,
                })
            } else {
                None
            }
        }
    }
}

fn staffer_reader(name: &str, job: &str, rating: u8, key: u64) -> Reader {
    let r = if rating == 0 { 10.0 } else { rating as f64 };
    Reader {
        key,
        name: name.to_string(),
        job: job.to_string(),
        link: Link::Staff,
        judge: 10.0,
        potential: 10.0,
        athletic: r,
        character: r,
        network: 10.0,
        work: 12.0,
        say: 0.6,
        bias: None,
        home: 99,
        contacts: None,
    }
}

/// The analytics head as a reader (SK6): he reads numbers, so his rating
/// is his judgment of them.
fn numbers_reader(name: &str, job: &str, rating: u8, key: u64) -> Reader {
    let r = if rating == 0 { 10.0 } else { rating as f64 };
    Reader { judge: r, potential: r, ..staffer_reader(name, job, rating, key) }
}

/// Whether a staff has an analytics department reading college numbers
/// (SK6): NFL clubs from 2003 (real: a few in the 2000s, all by the late
/// 2010s), FBS programs from 2016 (proposed, not confirmed).
pub fn has_analytics(w: &World, staff: Staff) -> bool {
    match staff {
        Staff::Club(_) => w.year >= 2003,
        Staff::Program(p) => w.year >= 2016 && w.programs[p].division == crate::world::data::Division::FBS,
        Staff::School(_) => false,
    }
}

/// The analytics staff's reads of a college player's numbers: one a
/// season in the book, written as the season closes (SK6).
fn numbers_looks(w: &World, staff: Staff, i: usize) -> Vec<(u32, Kind, crate::calendar::Date)> {
    let p = &w.people[i];
    if !has_analytics(w, staff) || !matches!(p.stage, Stage::College | Stage::Declared) {
        return Vec::new();
    }
    w.stats
        .career(p.player.id)
        .iter()
        .map(|r| (ANALYTICS, Kind::Numbers, crate::calendar::Date::ymd(r.year as i32, 12, 10)))
        .filter(|x| x.2 <= w.today)
        .collect()
}

fn club_reader(w: &World, t: usize, who: u32, group: usize) -> Option<Reader> {
    let club = w.clubs.get(t)?;
    match who {
        x if x == GM as u32 => {
            let g = &club.office.gm;
            Some(Reader {
                key: (t as u64) << 40 ^ (g.since as u64) << 8 ^ 0x6D,
                name: g.name.clone(),
                job: "General manager".into(),
                link: Link::Gm,
                judge: g.skill,
                potential: g.skill,
                athletic: 10.0,
                character: 10.0,
                network: 12.0,
                work: 10.0,
                say: if club.office.power == crate::world::pro::office::Power::Gm { 1.4 } else { 1.1 },
                bias: None,
                home: 99,
                contacts: None,
            })
        }
        x if x == HC as u32 || x == OC as u32 || x == DC as u32 => {
            let tm = &w.nfl[t];
            let c = match x as u8 {
                HC => tm.head_coach,
                OC => tm.oc.or(tm.head_coach),
                _ => tm.dc.or(tm.head_coach),
            }?;
            let job = match x as u8 {
                HC => "Head coach",
                OC => "Offensive coordinator",
                _ => "Defensive coordinator",
            };
            Some(coach_reader(w, c, job.into(), group))
        }
        MEDICAL => {
            let s = &club.support.medical;
            Some(staffer_reader(&s.name, "Head team physician", s.rating, t as u64 ^ 0x3ED))
        }
        SECURITY => Some(Reader {
            key: t as u64 ^ 0x5EC,
            name: "Club security".into(),
            job: "Security".into(),
            link: Link::Staff,
            judge: 10.0,
            potential: 10.0,
            athletic: 10.0,
            character: 13.0,
            network: 14.0,
            work: 12.0,
            say: 0.7,
            bias: None,
            home: 99,
            contacts: None,
        }),
        ANALYTICS => {
            let s = &club.support.analytics;
            let mut r = numbers_reader(&s.name, "Director of analytics", s.rating, t as u64 ^ 0xA7A);
            // F21: "Lean on the analytics" gives the numbers a full voice in
            // your room (proposed, not confirmed: say 0.6 to 1.1).
            if seat_staff(w) == Some(Staff::Club(t))
                && w.scouting.dept.instructions.contains(&super::dept::Instruction::Analytics)
            {
                r.say = 1.1;
            }
            Some(r)
        }
        x if (PRO_SCOUT..PERSONNEL).contains(&x) => {
            let k = (x - PRO_SCOUT) as usize;
            let s = w.scouting.pro.scouts.get(k)?;
            Some(scout_reader(s, group, "Pro scout"))
        }
        k if (k as usize) < club.scouts.len() => {
            let s = &club.scouts[k as usize];
            let job = match s.role {
                Role::Director => "Director of college scouting",
                Role::National => "National scout",
                Role::Area => "Area scout",
            };
            Some(scout_reader(s, group, job))
        }
        _ => None,
    }
}

fn scout_reader(s: &scouts::Scout, group: usize, job: &str) -> Reader {
    let know = s.know[group.min(GROUP_COUNT - 1)] as f64;
    Reader {
        key: s.key,
        name: s.name.clone(),
        job: job.to_string(),
        link: Link::Scout(0),
        judge: 0.6 * s.judge as f64 + 0.4 * know,
        potential: 0.7 * s.potential as f64 + 0.3 * know,
        athletic: s.athletic as f64,
        character: s.character as f64,
        network: s.network as f64,
        work: s.work as f64,
        say: (0.8 + 0.02 * s.conviction as f64) * if s.role == Role::Director { 1.2 } else { 1.0 },
        bias: Some(s.bias),
        home: s.home,
        contacts: (s.role == Role::Area && s.contacts > 0.0).then_some((s.region, s.contacts as f64)),
    }
}

/// Everything a staff has on one player: the readers who looked and every
/// look, oldest first (the club's built file plus your seat's own looks).
pub fn evidence(w: &World, staff: Staff, id: u32) -> (Vec<Reader>, Vec<Evidence>, Vec<u32>) {
    let Some(i) = super::place(w, id) else { return Default::default() };
    let group = Group::of(w.people[i].player.position) as usize;
    let mut raw: Vec<(u32, Kind, crate::calendar::Date)> = Vec::new();
    if let Staff::Club(t) = staff
        && let Some(f) = w.clubs.get(t).and_then(|c| c.files.get(&id))
    {
        raw.extend(f.iter().map(|l| (l.who as u32, Kind::from_nfl(l.kind), l.date)));
    }
    if let Some(v) = w.scouting.dept.looks.get(&id) {
        raw.extend(v.iter().map(|l: &SeatLook| (l.who, l.kind, l.date)));
    }
    raw.extend(own_roster(w, staff, i));
    // The analytics staff runs the numbers on everyone the staff is
    // looking at.
    if !raw.is_empty() {
        raw.extend(numbers_looks(w, staff, i));
    }
    raw.sort_by_key(|x| x.2);
    let mut codes: Vec<u32> = Vec::new();
    let mut readers: Vec<Reader> = Vec::new();
    let mut looks = Vec::new();
    for (who, kind, date) in raw {
        let r = match codes.iter().position(|&c| c == who) {
            Some(r) => r,
            None => {
                let Some(rd) = reader(w, staff, who, group) else { continue };
                let mut rd = rd;
                if let Link::Scout(_) = rd.link {
                    rd.link = Link::Scout(who as u8);
                }
                codes.push(who);
                readers.push(rd);
                readers.len() - 1
            }
        };
        looks.push(Evidence { reader: r, kind, date });
    }
    (readers, looks, codes)
}

/// Your own players: your staff sees them every day (practice and games),
/// which nothing stores; the file reads the last eight weeks of it, one
/// practice look a week by his position coach and one by you, and his
/// games in season.
fn own_roster(w: &World, staff: Staff, i: usize) -> Vec<(u32, Kind, crate::calendar::Date)> {
    let p = &w.people[i];
    let (mine, coach) = match staff {
        Staff::Club(t) => {
            let on = p.stage == Stage::Pro && p.pro.as_ref().is_some_and(|c| c.team == Some(t));
            (on, w.nfl[t].head_coach)
        }
        Staff::Program(prog) => {
            let on = p.stage == Stage::College && p.college == Some(prog);
            let g = Group::of(p.player.position) as usize;
            let pc = w.programs[prog].position_coaches.get(g).copied().flatten();
            (on, pc.or(w.programs[prog].head_coach))
        }
        Staff::School(h) => (matches!(p.stage, Stage::HighSchool | Stage::PrepSchool) && p.hs == h, None),
    };
    if !mine {
        return Vec::new();
    }
    let today = w.today;
    let mut v = Vec::new();
    for k in 0..8 {
        let d = today.add(-7 * k - 1);
        if let Some(c) = coach {
            v.push((COACH + c as u32, Kind::Practice, d));
        }
        if let Some(y) = you(w) {
            v.push((COACH + y as u32, Kind::Practice, d.add(-1)));
        }
        if matches!(d.parts().1, 9..=12) {
            v.push((coach.or(you(w)).map_or(EVENT as u32, |c| COACH + c as u32), Kind::Game, d.add(-2)));
        }
    }
    v
}

/// The context a read's biases react to.
pub fn norms(w: &World, i: usize) -> super::file::Norms {
    use crate::world::data::Division;
    let p = &w.people[i];
    let prog = p.college.map(|c| &w.programs[c]);
    let size = crate::body::combine_size(p.player.position);
    let z_size = clamp(
        0.5 * (p.player.body.height - size.0) / size.1.max(0.5)
            + 0.5 * (p.player.body.weight - size.2) / size.3.max(1.0),
        -1.5,
        1.5,
    );
    let z_speed = clamp((p.player.attributes.get(crate::attributes::Attr::Speed) - 70.0) / 9.0, -1.5, 1.5);
    super::file::Norms {
        z_size,
        z_speed,
        small_school: prog.is_some_and(|g| g.division != Division::FBS),
        region: prog.map_or(99, |g| scouts::region_of(g.state)),
        home: scouts::region_of(p.home_state),
        conf_key: prog.map_or(0, |g| g.conference.bytes().fold(7u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64))),
    }
}

/// The medical staff's diagnosis rating for a staff (topic 6).
pub fn medical_rating(w: &World, staff: Staff) -> f64 {
    let r = match staff {
        Staff::Club(t) => w.clubs.get(t).map_or(10, |c| c.support.medical.rating),
        Staff::Program(p) => w.programs[p].support.medical.rating,
        Staff::School(_) => 7,
    };
    if r == 0 { 10.0 } else { r as f64 }
}

/// A full read of one player's file for a staff, or None until a report on
/// him is written: a look still to come (a declared underclassman's film is
/// filed a week ahead, his cross-check three) or one not yet written up
/// reads nothing, so the file isn't "seen" without a rating in it.
pub fn read(w: &World, staff: Staff, id: u32) -> Option<(Vec<Reader>, Vec<Evidence>, super::file::FileRead)> {
    let i = super::place(w, id)?;
    let (readers, looks, _) = evidence(w, staff, id);
    if !looks.iter().any(|l| l.date.add(readers[l.reader].lag(l.kind, id, l.date)) <= w.today) {
        return None;
    }
    let mut f = super::file::read(w, i, &readers, &looks, &norms(w, i), medical_rating(w, staff), Some(w.today));
    if seat_staff(w) == Some(staff) {
        super::own::interview_traits(w, id, &mut f.traits);
    }
    Some((readers, looks, f))
}

/// A stable small number for a staff (keys for its draws).
pub fn staff_key(staff: Staff) -> u64 {
    match staff {
        Staff::Club(t) => 0x1000 + t as u64,
        Staff::Program(p) => 0x10_0000 + p as u64,
        Staff::School(h) => 0x100_0000 + h as u64,
    }
}

/// Whether a key-drawn chance comes up for this staff and player.
pub fn chance(staff: Staff, salt: u64, id: u32, p: f64) -> bool {
    key_uniform(staff_key(staff) ^ salt, id as u64) < p
}
