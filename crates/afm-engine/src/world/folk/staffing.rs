//! Staff as people, their contracts and the staff market (people topics 10
//! and 11; final build lane 6): every assistant has a deal (NFL assistants
//! two or three years; college coordinators and top assistants multi-year
//! with buyouts at programs that pay them, the rest year to year; a high
//! school job a yearly stipend on a teaching contract). The jobs the world
//! doesn't keep as coaches (NFL position coaches, quality control,
//! analysts, graduate assistants, every AI high school's staff) are people
//! drawn from the job and its term, never stored.
//!
//! Your side: you hire and fire your staff. Candidates start with who you
//! know (your network), then the free market and other staffs. Interviews
//! are talks; a confident talker interviews better than he coaches. Offers
//! read money, the step up, ties and your name; his team can counter, and
//! the permission rules of the level apply (an NFL club can block a lateral
//! move but not a promotion; a college deal carries a buyout; a high school
//! needs a teaching opening). Your coordinators and department heads bring
//! you proposals. Firing someone costs the rest of his deal, and the room
//! reacts by what it thought of him.
//!
//! Sizes and pay no table covers are Claude's picks (proposed, not
//! confirmed).

use super::scene::{self, Doc, DocRow, Kind, Pick, Scene, Who};
use super::talks::{self, Reason};
use super::{Team, coach_key, coach_link, facts, player_link};
use crate::calendar::{Date, Level, Loudness, Weekday};
use crate::player::{PlayingTime, Trait};
use crate::world::World;
use crate::world::coach::{Employer, Group, Role};
use crate::world::data::Division;

/// An assistant's deal.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Deal {
    pub coach: usize,
    /// Dollars a year.
    pub salary: f64,
    /// The last season it covers.
    pub through: i32,
    /// The share of the unpaid rest owed if he's fired (and the buyout his
    /// next school pays to take him early).
    pub buyout: f32,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Staffs {
    pub deals: Vec<Deal>,
    /// Raises you gave (coach, day): a promise of a raise is kept by one.
    pub raises: Vec<(usize, Date)>,
    /// More money from your boss for the staff pool and recruiting (shares).
    pub pool_bonus: f32,
    pub recruit_bonus: f32,
    /// Drawn jobs whose holder changed (job key, term).
    pub terms: Vec<(u64, u16)>,
    /// Interviews you've held (coach, day, his interview read 1-20).
    pub interviewed: Vec<(usize, Date, f32)>,
    /// Dead money for staff you fired (dollars, by year).
    pub owed: Vec<(i32, f64)>,
    /// The year contracts were last renewed.
    pub renewed: i32,
    /// Deals carried over from another job were repriced once (older saves
    /// kept a coach's deal when he moved).
    pub repriced: bool,
}

impl Staffs {
    pub fn deal(&self, c: usize) -> Option<&Deal> {
        self.deals.iter().find(|d| d.coach == c)
    }
    pub fn raised_since(&self, c: usize, d: Date) -> bool {
        self.raises.iter().any(|x| x.0 == c && x.1 >= d)
    }
}

/// Every AI high school's staff is drawn (never stored); a school whose
/// staff turned over keeps its new term here.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct HsStaffs {
    pub terms: Vec<(u32, u16)>,
}

/// A drawn staff member (a job the world doesn't keep as a coach).
#[derive(Clone, Debug, serde::Serialize)]
pub struct Drawn {
    pub key: u64,
    /// The person in the job: its key with the holder's term (a new hire
    /// is a new person).
    pub who: u64,
    pub name: String,
    pub title: String,
    pub age: u32,
    pub since: i32,
    /// What he teaches, at a high school.
    pub subject: Option<String>,
    /// Pay a year, dollars.
    pub pay: f64,
}

/// What an assistant job pays now (the built pay table; NFL position
/// coaches from the real range).
fn pay(w: &World, e: Employer, r: Role) -> f64 {
    crate::world::staff::salary(w, e, r)
}

/// An assistant deal's usual length in years and its buyout share: power
/// conference coordinators three years with half owed, their position
/// coaches two with 30%, other colleges a year at a time, the NFL two or
/// three years.
fn deal_terms(w: &World, c: usize, e: Employer, r: Role) -> (i32, f32) {
    match e {
        Employer::Nfl(_) | Employer::Other(_) => (2 + (super::roll(w, 0xDEA1, c as u64) < 0.4) as i32, 0.0),
        Employer::College(p) => {
            let big = w.programs[p].division == Division::FBS && w.programs[p].power_four();
            match (big, r) {
                (true, Role::OffensiveCoordinator | Role::DefensiveCoordinator) => (3, 0.5),
                (true, _) => (2, 0.3),
                _ => (1, 0.0),
            }
        }
        Employer::HighSchool(_) => (1, 0.0),
    }
}

/// A newly hired assistant's deal at his job's pay, from today (the world's
/// own moves; a hire you make carries the deal you offered).
pub(crate) fn sign(w: &mut World, c: usize) {
    let Some(job) = w.coaches[c].job else { return };
    if job.role == Role::HeadCoach || matches!(job.employer, Employer::HighSchool(_)) || w.folk.staff.deal(c).is_some()
    {
        return;
    }
    let base = pay(w, job.employer, job.role);
    let salary = (base * (0.8 + 0.4 * w.coaches[c].reputation / 20.0)).round();
    let (years, buyout) = deal_terms(w, c, job.employer, job.role);
    w.folk.staff.deals.push(Deal { coach: c, salary, through: w.year + years - 1, buyout });
}

/// Fills a deal for every assistant who lacks one (old worlds, new hires).
pub fn ensure(w: &mut World) {
    let year = w.year;
    if !w.folk.staff.repriced {
        w.folk.staff.repriced = true;
        reprice(w);
    }
    let mut have: std::collections::HashSet<usize> = w.folk.staff.deals.iter().map(|d| d.coach).collect();
    let mut add: Vec<Deal> = Vec::new();
    for c in 0..w.coaches.len() {
        let Some(job) = w.coaches[c].job else { continue };
        if job.role == Role::HeadCoach || matches!(job.employer, Employer::HighSchool(_)) || have.contains(&c) {
            continue;
        }
        let base = pay(w, job.employer, job.role);
        let rep = w.coaches[c].reputation;
        let salary = (base * (0.8 + 0.4 * rep / 20.0)).round();
        let (years, buyout) = deal_terms(w, c, job.employer, job.role);
        // A coach who has stayed past his first deal signed again on the
        // same terms each time it ran out, so the current one ends where
        // its cycle does (not this season for everyone).
        let cycles = (year - job.since).max(0) / years + 1;
        add.push(Deal { coach: c, salary, through: (job.since + years * cycles - 1).max(year), buyout });
        have.insert(c);
    }
    w.folk.staff.deals.extend(add);
    // Deals of coaches no longer in an assistant job are dropped.
    let keep: Vec<bool> = w
        .folk
        .staff
        .deals
        .iter()
        .map(|d| {
            w.coaches
                .get(d.coach)
                .and_then(|c| c.job)
                .is_some_and(|j| j.role != Role::HeadCoach && !matches!(j.employer, Employer::HighSchool(_)))
        })
        .collect();
    let mut k = 0;
    w.folk.staff.deals.retain(|_| {
        let x = keep[k];
        k += 1;
        x
    });
}

/// Older saves kept an assistant's deal when he moved, so a coach who went
/// from a small school to a big one kept his old pay. A deal far from what
/// his job pays (under half or over twice) is signed again at his job's pay.
fn reprice(w: &mut World) {
    let year = w.year;
    for k in 0..w.folk.staff.deals.len() {
        let c = w.folk.staff.deals[k].coach;
        let Some(job) = w.coaches.get(c).and_then(|co| co.job) else { continue };
        if job.role == Role::HeadCoach || matches!(job.employer, Employer::HighSchool(_)) {
            continue;
        }
        let base = pay(w, job.employer, job.role);
        let have = w.folk.staff.deals[k].salary;
        if base <= 0.0 || (0.5..=2.0).contains(&(have / base)) {
            continue;
        }
        let rep = w.coaches[c].reputation;
        let (years, buyout) = deal_terms(w, c, job.employer, job.role);
        let cycles = (year - job.since).max(0) / years + 1;
        let d = &mut w.folk.staff.deals[k];
        d.salary = (base * (0.8 + 0.4 * rep / 20.0)).round();
        d.through = (job.since + years * cycles - 1).max(year);
        d.buyout = buyout;
    }
}

/// The term of a drawn job (a change bumps it, drawing a new person).
fn term(w: &World, key: u64) -> u16 {
    w.folk.staff.terms.iter().find(|x| x.0 == key).map_or(0, |x| x.1)
}

pub(crate) const NFL_ROOMS: [(&str, u8); 11] = [
    ("Quarterbacks coach", 0),
    ("Running backs coach", 1),
    ("Wide receivers coach", 2),
    ("Tight ends coach", 2),
    ("Offensive line coach", 3),
    ("Defensive line coach", 4),
    ("Linebackers coach", 5),
    ("Defensive backs coach", 6),
    ("Special teams coordinator", 7),
    ("Assistant head coach", 9),
    ("Offensive quality control", 9),
];

pub(crate) fn draw_person(w: &World, key: u64, title: &str, pay: f64, subject: Option<String>) -> Drawn {
    let t = term(w, key);
    let k = key ^ (t as u64) << 48;
    let u = |n: u64| super::roll(w, k, n);
    let black = u(1) < 0.33;
    let eth = if black { crate::world::ethnicity::Ethnicity::Black } else { crate::world::ethnicity::Ethnicity::White };
    let age = 27 + (u(2) * 33.0) as u32;
    let mut rng = crate::rng::Rng::new((k ^ k >> 32) as u32 ^ 0x57AF);
    let name = crate::world::names::person_name(eth, w.year - age as i32, &mut rng);
    let since = w.year - (u(3) * 6.0) as i32;
    Drawn { key, who: k, name, title: title.to_string(), age, since, subject, pay: (pay * (0.85 + 0.3 * u(4))).round() }
}

/// The staff jobs the world doesn't keep as coaches, drawn for a team:
/// NFL position coaches and quality control; college analysts and graduate
/// assistants (on-field analysts from June 2024); an AI high school's whole
/// staff.
pub fn drawn_staff(w: &World, team: Team) -> Vec<Drawn> {
    let mut v = Vec::new();
    match team {
        Team::Nfl(t) => {
            let base = pay(w, Employer::Nfl(t), Role::PositionCoach(Group::QB));
            for (k, (title, _)) in NFL_ROOMS.iter().enumerate() {
                let key = team.key() << 16 ^ 0xA55 ^ k as u64;
                v.push(draw_person(w, key, title, base * if k >= 9 { 0.6 } else { 1.0 }, None));
            }
        }
        // An other-league club keeps a lean room: the position coaches and a
        // special teams coach, no quality control.
        Team::Other(f) => {
            let base = pay(w, Employer::Other(f), Role::PositionCoach(Group::QB));
            for (k, (title, _)) in NFL_ROOMS.iter().enumerate().take(9) {
                let key = team.key() << 16 ^ 0xA55 ^ k as u64;
                v.push(draw_person(w, key, title, base, None));
            }
        }
        Team::College(p) => {
            let prog = &w.programs[p];
            let rich = crate::world::staff::program_budget(prog);
            let fbs = prog.division == Division::FBS;
            let analysts = if fbs { (2.0 + rich / 25.0e6).min(12.0) as usize } else { 1 };
            let gas = if fbs || prog.division == Division::FCS { 4 } else { 2 };
            for k in 0..analysts {
                let key = team.key() << 16 ^ 0xA7A ^ k as u64;
                let title = if w.year >= 2024 && w.today >= crate::calendar::Date::ymd(2024, 6, 1) && k < 4 && fbs {
                    "On-field analyst"
                } else {
                    "Analyst"
                };
                v.push(draw_person(w, key, title, if fbs { 75_000.0 } else { 35_000.0 }, None));
            }
            for k in 0..gas {
                let key = team.key() << 16 ^ 0x6A0 ^ k as u64;
                v.push(draw_person(w, key, "Graduate assistant", 25_000.0, None));
            }
        }
        Team::High(h) => {
            if Team::seat(w) == Some(team) {
                return v;
            }
            let s = &w.high_schools[h];
            let n = (2 + s.enrollment / 400).clamp(3, 15) as usize;
            let term = w.folk.hs_staffs.terms.iter().find(|x| x.0 == h as u32).map_or(0, |x| x.1);
            const SUBJ: [&str; 6] = ["physical education", "history", "health", "math", "science", "special education"];
            for k in 0..n {
                let key = team.key() << 16 ^ 0x45 ^ (k as u64) ^ (term as u64) << 8;
                let title = match k {
                    0 => "Offensive coordinator".to_string(),
                    1 => "Defensive coordinator".to_string(),
                    _ => Group::ALL[(k + 1) % 7].coach_title().to_string(),
                };
                let subj = SUBJ[(super::roll(w, key, 9) * SUBJ.len() as f64) as usize % SUBJ.len()].to_string();
                v.push(draw_person(w, key, &title, 2_500.0 + 3_500.0 * s.resources, Some(subj)));
            }
        }
    }
    v
}

/// Lets a coach go (takeover item 5, topic 11): his job ends, you owe the
/// rest of his deal, and the room reacts by what it thought of him.
pub fn fire(w: &mut World, c: usize, why: &'static str) -> String {
    let Some(job) = w.coaches[c].job else { return String::new() };
    let year = w.year;
    let owed = w.folk.staff.deal(c).map_or(0.0, |d| {
        d.salary * (d.through - year + 1).max(0) as f64 * if d.buyout > 0.0 { d.buyout as f64 } else { 0.25 }
    });
    let name = w.coaches[c].name.clone();
    // The room (topic 11): players in his group; a coach they loved costs
    // trust, one they blamed can lift it.
    let team = Team::from_employer(job.employer);
    let group = match job.role {
        Role::PositionCoach(g) => Some(g),
        _ => None,
    };
    let off = job.role.offense();
    let rel = w.coaches[c].ability().relationships;
    let grew = w.coaches[c].units.iter().map(|u| u.growth_read()).sum::<f64>() / 8.0;
    let liked = (rel - 10.0) / 10.0 + 0.5 * grew;
    let roster = team.roster(w);
    let mut hit = 0;
    for i in roster {
        let p = &w.people[i];
        let mine = match group {
            Some(g) => Group::of(p.player.position) == g,
            None => off.is_some_and(|o| crate::world::systems::offense(p.player.position) == o),
        };
        if !mine {
            continue;
        }
        let d = (-0.05 * liked).clamp(-0.06, 0.03) as f32;
        let id = p.player.id;
        let pp = &mut w.people[i];
        pp.trust = (pp.trust + d).clamp(0.0, 1.0);
        if d < -0.02 {
            hit += 1;
            talks::mood_event(w, id, Reason::Talk, d);
        }
    }
    crate::world::staff::leave(w, c, why);
    w.folk.staff.deals.retain(|d| d.coach != c);
    if owed > 0.0 {
        match w.folk.staff.owed.iter_mut().find(|x| x.0 == year) {
            Some(x) => x.1 += owed,
            None => w.folk.staff.owed.push((year, owed)),
        }
    }
    w.folk.mine.set_trust(coach_key(c), 0.1);
    let text = format!(
        "{name} is out as {}{}{}.",
        job.role.name(),
        if owed > 0.0 {
            format!("; you owe ${} of his deal", facts::thousands((owed / 1000.0) as u32) + ",000")
        } else {
            String::new()
        },
        if hit >= 3 { format!(". {hit} of his players took it hard") } else { String::new() }
    );
    super::post_mine(w, w.today, Loudness::Small, false, "people: staff", &text);
    text
}

/// A candidate for a job on your staff, as you can see him.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Candidate {
    pub coach: usize,
    pub name: String,
    pub age: u32,
    pub now: String,
    pub rep: String,
    pub came_up: String,
    /// Known through your network: warm, or a stranger.
    pub known: String,
    /// Your read of his teaching, a range 1-20 (sharper for people you know
    /// and those you've interviewed).
    pub read: (f32, f32),
    pub asks: f64,
    /// Can his team block the move (NFL lateral)?
    pub blocked: bool,
}

/// Candidates for a role on your staff (topic 11): your network first, then
/// coaches out of work, then assistants elsewhere one rung down or level.
pub fn candidates(w: &World, role: Role) -> Vec<Candidate> {
    let Some(team) = Team::seat(w) else { return Vec::new() };
    let emp = team.employer();
    let you = w.career.as_ref().map(|c| c.coach);
    let want_group = match role {
        Role::PositionCoach(g) => Some(g),
        _ => None,
    };
    let base = pay(w, emp, role);
    let mut v: Vec<(f64, Candidate)> = Vec::new();
    for (c, co) in w.coaches.iter().enumerate() {
        if co.retired || Some(c) == you || co.age > 70 || co.show_cause.is_some_and(|y| y >= w.year) {
            continue;
        }
        let here = co.job.map(|j| Team::from_employer(j.employer)) == Some(team);
        if here {
            continue;
        }
        let fits = match role {
            Role::OffensiveCoordinator => {
                co.came_up.offense() || co.job.is_some_and(|j| j.role == Role::OffensiveCoordinator)
            }
            Role::DefensiveCoordinator => {
                !co.came_up.offense() || co.job.is_some_and(|j| j.role == Role::DefensiveCoordinator)
            }
            Role::PositionCoach(g) => co.came_up == g || co.job.is_some_and(|j| j.role == Role::PositionCoach(g)),
            Role::HeadCoach => false,
        };
        if !fits {
            continue;
        }
        let warm = super::network::warmth(w, c);
        let free = co.job.is_none();
        let step = co.job.map_or(0.0, |j| match (j.role, role) {
            (Role::PositionCoach(_), Role::OffensiveCoordinator | Role::DefensiveCoordinator) => 1.0,
            (a, b) if a == b => 0.0,
            (Role::HeadCoach, _) => -2.0,
            (Role::OffensiveCoordinator | Role::DefensiveCoordinator, Role::PositionCoach(_)) => -1.0,
            _ => 0.0,
        });
        if step < -0.5 && !free {
            continue;
        }
        let score = co.reputation + 4.0 * warm as f64 + if free { 1.0 } else { 0.0 } + step;
        if score < 6.0 && warm < 0.3 {
            continue;
        }
        let same_level = co.job.is_some_and(|j| std::mem::discriminant(&j.employer) == std::mem::discriminant(&emp));
        let blocked = matches!(emp, Employer::Nfl(_)) && same_level && step <= 0.0 && !free;
        let teach = match want_group {
            Some(g) => co.ability().teaching[g as usize],
            None => {
                let off = role == Role::OffensiveCoordinator;
                Group::ALL
                    .iter()
                    .filter(|g| g.offense() == off && **g != Group::Specialists)
                    .map(|g| co.ability().teaching[*g as usize])
                    .sum::<f64>()
                    / 3.0
            }
        } as f32;
        let interviewed = w.folk.staff.interviewed.iter().find(|x| x.0 == c).map(|x| x.2);
        let width = (5.5 - 4.0 * warm - if interviewed.is_some() { 1.5 } else { 0.0 }).max(1.2);
        let bias =
            super::roll_n(w, 0x5EAD, c as u64) as f32 * width * 0.4 + interviewed.map_or(0.0, |r| (r - teach) * 0.3);
        let mid = (teach + bias).clamp(1.0, 20.0);
        let read = ((mid - width / 2.0).max(1.0), (mid + width / 2.0).min(20.0));
        let now = match co.job {
            Some(j) => {
                format!("{}, {}", super::takeover::cap_first(j.role.name()), Team::from_employer(j.employer).name(w))
            }
            None => co.history.last().map_or("Out of coaching".into(), |s| {
                format!("Out of work (last: {}, {})", s.role.name(), Team::from_employer(s.employer).name(w))
            }),
        };
        let asks = (base * (0.85 + 0.35 * co.reputation / 20.0) * if co.job.is_some() { 1.15 } else { 0.95 }).round();
        let known = if warm >= 0.6 {
            "A close contact".to_string()
        } else if warm >= 0.3 {
            "Someone you know".to_string()
        } else {
            "A stranger: you know his name and his record".to_string()
        };
        v.push((
            score,
            Candidate {
                coach: c,
                name: co.name.clone(),
                age: co.age,
                now,
                rep: super::rep_words(co.reputation).to_string(),
                came_up: co.came_up.name().to_string(),
                known,
                read,
                asks,
                blocked,
            },
        ));
    }
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    v.into_iter().take(24).map(|x| x.1).collect()
}

/// The open jobs on your staff.
pub fn openings(w: &World) -> Vec<Role> {
    let Some(team) = Team::seat(w) else { return Vec::new() };
    match team {
        Team::College(p) => {
            let prog = &w.programs[p];
            let mut v = Vec::new();
            if prog.oc.is_none() {
                v.push(Role::OffensiveCoordinator);
            }
            if prog.dc.is_none() {
                v.push(Role::DefensiveCoordinator);
            }
            for (k, c) in prog.position_coaches.iter().enumerate() {
                if c.is_none() {
                    v.push(Role::PositionCoach(Group::ALL[k]));
                }
            }
            v
        }
        Team::Nfl(t) => {
            let mut v = Vec::new();
            if w.nfl[t].oc.is_none() {
                v.push(Role::OffensiveCoordinator);
            }
            if w.nfl[t].dc.is_none() {
                v.push(Role::DefensiveCoordinator);
            }
            v
        }
        Team::High(_) | Team::Other(_) => Vec::new(),
    }
}

/// Opens an interview with a candidate (a talk, topic 11).
pub fn start_interview(w: &mut World, c: usize, role: Role) -> Result<u32, String> {
    let team = Team::seat(w).ok_or("You have no team.")?;
    if w.coaches.get(c).is_none() {
        return Err("No such coach.".into());
    }
    if let Some(s) = w.folk.scenes.iter().find(|s| !s.done && s.kind == Kind::Interview && s.a == c as u64) {
        return Ok(s.id);
    }
    if !talks::spend(w, 2.0) {
        return Err("Your week is full: an interview takes two hours.".into());
    }
    let mut s = Scene::new(Kind::Interview, w.today, format!("Interviewing {}", w.coaches[c].name));
    s.a = c as u64;
    s.b = role_code(role);
    s.stops = false;
    s.team = Some(team);
    s.place = "Your office, then dinner".into();
    Ok(scene::add(w, s))
}

pub fn role_code(r: Role) -> u64 {
    match r {
        Role::HeadCoach => 0,
        Role::OffensiveCoordinator => 1,
        Role::DefensiveCoordinator => 2,
        Role::PositionCoach(g) => 10 + g as u64,
    }
}

pub fn role_from(code: u64) -> Role {
    match code {
        1 => Role::OffensiveCoordinator,
        2 => Role::DefensiveCoordinator,
        x if x >= 10 => Role::PositionCoach(Group::ALL[(x - 10) as usize % Group::ALL.len()]),
        _ => Role::HeadCoach,
    }
}

/// How he interviews (topic 11): a confident talker interviews better than
/// he coaches. Returns the read the interview gives (1-20).
fn interview_read(w: &World, c: usize, role: Role) -> f32 {
    let co = &w.coaches[c];
    let a = co.ability();
    let real = match role {
        Role::PositionCoach(g) => a.teaching[g as usize],
        _ => 0.5 * a.scheme + 0.5 * a.teaching.iter().sum::<f64>() / 8.0,
    };
    let talker = 0.3 * (co.personality.ambition - 10.0) + 0.2 * (a.relationships - 10.0);
    (real + talker + super::roll_n(w, 0x1E7 ^ c as u64, 1) * 1.5).clamp(1.0, 20.0) as f32
}

fn interview_open(w: &mut World, s: &mut Scene) {
    let c = s.a as usize;
    let role = role_from(s.b);
    s.who.push(Who::Coach(c));
    let co = &w.coaches[c];
    let spk = super::voice::Speaker::coach(w, c);
    let mut f = super::voice::Facts::new();
    f.set("role", role.name());
    f.set("team", s.team.map_or(String::new(), |t| t.name(w)));
    f.set("came_up", co.came_up.name());
    s.narrate(format!(
        "{} sits across from you with a binder he made for this job. {}",
        coach_link(w, c),
        match co.job {
            Some(j) => format!("He's {} at {} now.", j.role.name(), Team::from_employer(j.employer).link(w)),
            None => "He's been out of work since the last carousel.".into(),
        }
    ));
    let name = co.name.clone();
    let mut pen = scene::pen(w, s, 1);
    let line = pen.line(&["staff.interview.open"], &spk, &f);
    pen.put(w);
    s.them(
        &name,
        if line.is_empty() { format!("I've studied your film. Here's how I'd run the {}.", role.name()) } else { line },
        None,
    );
    s.pick(60, "Ask about his scheme and how he'd use your players", "Ask", "calm", "The most telling question.");
    s.pick(61, "Ask how he'd handle your most difficult player", "Ask", "firm", "How he handles people.");
    s.pick(62, "Ask where he recruits and who he'd bring", "Ask", "calm", "His ties and his tree.");
    s.pick(63, "Ask what he wants: money, title, play calling", "Ask", "calm", "What it will take.");
}

fn interview_answer(w: &mut World, s: &mut Scene, code: u32) {
    let c = s.a as usize;
    let role = role_from(s.b);
    let name = w.coaches[c].name.clone();
    let read = interview_read(w, c, role);
    let co = &w.coaches[c];
    let a = *co.ability();
    let text = match code {
        60 => {
            if read >= 13.0 {
                "He goes to the whiteboard and doesn't stop for twenty minutes. Every answer has a clip behind it."
                    .to_string()
            } else if read >= 9.0 {
                "Solid, if familiar: the things every coach says, said well.".to_string()
            } else {
                "He talks about his old team's players more than yours.".to_string()
            }
        }
        61 => {
            if a.relationships >= 13.0 {
                "\"I'd take him to lunch before I took him to the film room.\" He means it.".to_string()
            } else if a.discipline >= 13.0 {
                "\"He'll learn the standard or he'll learn the bench.\"".to_string()
            } else {
                "He hasn't thought about it, and it shows.".to_string()
            }
        }
        62 => {
            let ties = co.ties.len();
            if ties >= 5 {
                format!("He names a dozen high school coaches by their first names. His ties reach {ties} schools.")
            } else {
                "He'd recruit where you tell him to. He doesn't have a territory of his own.".to_string()
            }
        }
        _ => {
            let want = if co.personality.ambition >= 14.0 {
                "play calling, and a path to a head job"
            } else if co.personality.loyalty >= 14.0 {
                "years: he wants to stay somewhere"
            } else {
                "the market rate"
            };
            format!("He wants {want}.")
        }
    };
    s.you(
        &scene::your_name(w),
        match code {
            60 => "How would you use our players?",
            61 => "What would you do with a difficult player?",
            62 => "Where do you recruit?",
            _ => "What will it take?",
        },
    );
    s.them(
        &name,
        text,
        Some(if co.personality.ambition >= 14.0 {
            "Confident. Maybe more confident than his record.".into()
        } else {
            "Measured.".into()
        }),
    );
    let k = Pick::default();
    let _ = k;
    // Each answer sharpens your read of him a little.
    match w.folk.staff.interviewed.iter_mut().find(|x| x.0 == c) {
        Some(x) => {
            x.1 = w.today;
            x.2 = 0.5 * (x.2 + read);
        }
        None => w.folk.staff.interviewed.push((c, w.today, read)),
    }
    if w.folk.staff.interviewed.len() > 60 {
        w.folk.staff.interviewed.remove(0);
    }
    if s.step >= 2 {
        s.outcome.push(format!("You interviewed {name}. Your read of him is sharper; the staff screen shows it."));
        s.end();
    } else {
        let used = s.lines.iter().filter(|l| l.by == 1).count();
        let all = [
            (60, "Ask about his scheme"),
            (61, "Ask about a difficult player"),
            (62, "Ask where he recruits"),
            (63, "Ask what he wants"),
        ];
        for (code2, label) in all.iter() {
            if *code2 != code && s.picks.iter().all(|p| p.code != *code2) && used < 3 {
                s.pick(*code2, *label, "Ask", "calm", "");
            }
        }
        s.pick(69, "Thank him and wrap it up", "End", "calm", "");
    }
}

/// Offers a job (topic 11). He decides by money against what he makes, the
/// step up, play calling, ties and your name; his team may counter; an NFL
/// club can block a lateral move. Returns his answer in words.
pub fn offer(w: &mut World, c: usize, role: Role, salary: f64, years: i32) -> Result<String, String> {
    let team = Team::seat(w).ok_or("You have no team.")?;
    if !openings(w).contains(&role) {
        return Err("That job isn't open on your staff. Let someone go first.".into());
    }
    let co = w.coaches.get(c).ok_or("No such coach.")?.clone();
    let emp = team.employer();
    let now = w.folk.staff.deal(c).map_or(co.job.map_or(0.0, |j| pay(w, j.employer, j.role)), |d| d.salary);
    let same_level = co.job.is_some_and(|j| std::mem::discriminant(&j.employer) == std::mem::discriminant(&emp));
    let step = co.job.map_or(1.0, |j| match (j.role, role) {
        (Role::PositionCoach(_), Role::OffensiveCoordinator | Role::DefensiveCoordinator) => 1.0,
        (a, b) if a == b => 0.0,
        _ => -0.5,
    });
    if matches!(emp, Employer::Nfl(_)) && same_level && step <= 0.0 && co.job.is_some() {
        let blocks = super::roll(w, 0xB10C, c as u64 ^ w.today.0 as u64) < 0.7;
        if blocks {
            return Ok(format!(
                "{}'s club denied permission: a lateral move under contract can be blocked. He stays where he is.",
                co.name
            ));
        }
    }
    if let Employer::HighSchool(_) = emp {
        let opening = super::roll(w, 0x7EAC, c as u64 ^ w.year as u64) < 0.55;
        if !opening {
            return Ok(format!(
                "The principal has no teaching opening in {}'s subject this year. The board won't approve a coaching-only hire.",
                co.name
            ));
        }
    }
    let money = if now > 0.0 { (salary / now - 1.0).clamp(-0.5, 1.0) } else { 0.6 };
    let warm = super::network::warmth(w, c) as f64;
    let rep = facts::your_rep(w) / 20.0;
    let loyal = (co.personality.loyalty - 10.0) / 10.0;
    let years_v = 0.04 * (years - 1) as f64 * (1.0 + loyal.max(0.0));
    let score = 0.8 * money + 0.5 * step + 0.4 * warm + 0.3 * (rep - 0.5) + years_v
        - if co.job.is_some() { 0.25 + 0.2 * loyal } else { -0.2 };
    let u = super::roll(w, 0x0FFE, c as u64 ^ w.today.0 as u64);
    // His team counters for a man it wants to keep.
    let counter = co.job.is_some() && co.reputation >= 11.0 && u < 0.4;
    let yes = score + if counter { -0.2 } else { 0.0 } > 0.05 + 0.1 * u;
    if !yes {
        let why = if counter {
            "his school matched it with a raise and a better title"
        } else if money < 0.0 {
            "the money isn't there"
        } else if step < 0.0 {
            "it's a step down for him"
        } else {
            "he wants to stay where his family is settled"
        };
        talks::spend(w, 0.0);
        return Ok(format!("{} turned you down: {why}.", co.name));
    }
    // Hired.
    if co.job.is_some() {
        if let Some(d) = w.folk.staff.deal(c).copied()
            && d.buyout > 0.0
            && d.through > w.year
        {
            let owed = d.salary * (d.through - w.year) as f64 * d.buyout as f64;
            match w.folk.staff.owed.iter_mut().find(|x| x.0 == w.year) {
                Some(x) => x.1 += owed,
                None => w.folk.staff.owed.push((w.year, owed)),
            }
        }
        crate::world::staff::leave(w, c, "left for another job");
    }
    crate::world::staff::hire(w, c, emp, role);
    w.folk.staff.deals.retain(|d| d.coach != c);
    let buy = if years >= 2 && matches!(emp, Employer::College(_)) { 0.4 } else { 0.0 };
    w.folk.staff.deals.push(Deal { coach: c, salary, through: w.year + years.max(1) - 1, buyout: buy });
    w.folk.mine.set_trust(coach_key(c), 0.6 + 0.2 * warm as f32);
    super::network::touch(w, c, 0.15);
    let text = format!(
        "{} is your new {}: {} years at ${} a year.",
        co.name,
        role.name(),
        years.max(1),
        facts::thousands((salary / 1000.0) as u32) + ",000"
    );
    super::post_mine(w, w.today, Loudness::Small, false, "people: staff", &text);
    Ok(text)
}

/// Gives an assistant a raise (keeps a promise of one).
pub fn raise(w: &mut World, c: usize, share: f64) -> Result<String, String> {
    let d = w.folk.staff.deals.iter_mut().find(|d| d.coach == c).ok_or("He has no deal with you.")?;
    d.salary = (d.salary * (1.0 + share)).round();
    let s = d.salary;
    w.folk.staff.raises.push((c, w.today));
    w.folk.mine.nudge(coach_key(c), 0.08);
    Ok(format!("{} now makes ${} a year.", w.coaches[c].name, facts::thousands((s / 1000.0) as u32) + ",000"))
}

/// Opens the conversation when you let a coach go.
pub fn start_firing(w: &mut World, c: usize) -> Result<u32, String> {
    let team = Team::seat(w).ok_or("You have no team.")?;
    let on = w
        .coaches
        .get(c)
        .and_then(|x| x.job)
        .is_some_and(|j| Team::from_employer(j.employer) == team && j.role != Role::HeadCoach);
    if !on {
        return Err("He isn't on your staff.".into());
    }
    let mut s = Scene::new(Kind::Firing, w.today, format!("Letting {} go", w.coaches[c].name));
    s.a = c as u64;
    s.stops = false;
    s.team = Some(team);
    s.place = "Your office, door closed".into();
    Ok(scene::add(w, s))
}

fn firing_open(w: &mut World, s: &mut Scene) {
    let c = s.a as usize;
    s.who.push(Who::Coach(c));
    let co = &w.coaches[c];
    let owed = w.folk.staff.deal(c).map_or(0.0, |d| {
        d.salary * (d.through - w.year + 1).max(0) as f64 * if d.buyout > 0.0 { d.buyout as f64 } else { 0.25 }
    });
    s.narrate(format!(
        "{} knows why you called him in. {}",
        coach_link(w, c),
        if owed > 0.0 {
            format!("Letting him go costs about ${} of his deal.", facts::thousands((owed / 1000.0) as u32) + ",000")
        } else {
            "His deal is year to year.".into()
        }
    ));
    let _ = co;
    s.pick(
        70,
        "Tell him straight: it's the results",
        "Tell him straight",
        "calm",
        "He doesn't like it, but he respects it.",
    );
    s.pick(
        71,
        "Thank him, and offer to help him land somewhere",
        "Praise",
        "warm",
        "He leaves a friend; your network keeps him.",
    );
    s.pick(
        72,
        "Blame his unit in front of the staff",
        "Criticize",
        "fiery",
        "The staff sees you protect yourself; he holds a grudge.",
    );
    s.pick(73, "Change your mind: he stays", "Listen", "calm", "Nothing happens; he knows how close it was.");
}

fn firing_answer(w: &mut World, s: &mut Scene, code: u32) {
    let c = s.a as usize;
    if code == 73 {
        w.folk.mine.nudge(coach_key(c), -0.1);
        s.outcome.push("You kept him. He knows how close it was.".into());
        s.end();
        return;
    }
    let text = fire(w, c, "fired");
    s.outcome.push(text);
    match code {
        71 => {
            super::network::touch(w, c, 0.1);
            s.outcome.push("He shook your hand on the way out.".into());
        }
        72 => {
            super::network::touch(w, c, -0.4);
            if let Some(t) = s.team {
                for (a, _) in facts::assistants(w, t) {
                    w.folk.mine.nudge(coach_key(a), -0.05);
                }
            }
            s.outcome.push("The rest of your staff watched you put it on him.".into());
        }
        _ => super::network::touch(w, c, -0.1),
    }
    s.end();
}

// ---------------------------------------------------------------------------
// Proposals from your staff (topic 10: they disagree, and bring you ideas).

fn proposals_week(w: &mut World, d: Date) {
    let Some(team) = Team::seat(w) else { return };
    if w.folk.scenes.iter().any(|s| s.kind == Kind::Proposal && s.date.days_to(d) < 10) {
        return;
    }
    if super::roll(w, 0x960 ^ d.0 as u64, 1) > 0.45 {
        return;
    }
    let roster = team.roster(w);
    let staff = facts::assistants(w, team);
    // 1. Start the young player: a position coach thinks the backup has
    // passed the starter (what he sees, from the ratings, with his eye).
    let mut best: Option<(usize, usize, usize, f64)> = None;
    for (c, role) in &staff {
        let Role::PositionCoach(g) = role else { continue };
        let eye = w.coaches[*c].ability().teaching[*g as usize];
        let mut room: Vec<usize> =
            roster.iter().copied().filter(|&i| Group::of(w.people[i].player.position) == *g).collect();
        room.sort_by(|&a, &b| w.people[b].player.overall().total_cmp(&w.people[a].player.overall()));
        let starter = room.iter().copied().find(|&i| w.people[i].last_role == Some(PlayingTime::Starter));
        let young = room
            .iter()
            .copied()
            .find(|&i| w.people[i].last_role != Some(PlayingTime::Starter) && w.people[i].player.age <= 22);
        if let (Some(s0), Some(y)) = (starter, young) {
            let gap = w.people[y].player.overall() - w.people[s0].player.overall()
                + super::roll_n(w, 0x961 ^ d.0 as u64, *c as u64) * (12.0 - 0.5 * eye).max(1.0);
            if gap > 1.0 && best.is_none_or(|b| gap > b.3) {
                best = Some((*c, y, s0, gap));
            }
        }
    }
    let mut s = Scene::new(Kind::Proposal, d, String::new());
    s.stops = false;
    s.team = Some(team);
    s.place = "The staff room".into();
    if let Some((c, y, s0, _)) = best {
        s.title = format!("{} wants to start {}", w.coaches[c].name, w.people[y].name);
        s.a = c as u64;
        s.b = (w.people[y].player.id as u64) << 32 | w.people[s0].player.id as u64;
        s.c = 1;
        scene::add(w, s);
        return;
    }
    // 2. The trainer wants to hold a starter out a week (load and soft
    // tissue, topic 10).
    if let Some(&i) = roster.iter().find(|&&i| {
        w.people[i].last_role == Some(PlayingTime::Starter)
            && w.people[i].player.age >= 27
            && super::roll(w, 0x962 ^ d.0 as u64, i as u64) < 0.05
    }) {
        s.title = format!("The trainer wants to rest {}", w.people[i].name);
        s.b = w.people[i].player.id as u64;
        s.c = 2;
        scene::add(w, s);
    }
}

fn proposal_open(w: &mut World, s: &mut Scene) {
    match s.c {
        1 => {
            let c = s.a as usize;
            let (y, s0) = ((s.b >> 32) as u32, (s.b & 0xFFFF_FFFF) as u32);
            let (Some(yi), Some(si)) = (super::person(w, y), super::person(w, s0)) else {
                s.end();
                return;
            };
            s.who.push(Who::Coach(c));
            s.who.push(Who::Player(y));
            s.who.push(Who::Player(s0));
            s.narrate(format!("{} has the practice tape cued up.", coach_link(w, c)));
            s.them(
                &w.coaches[c].name.clone(),
                format!("{} has passed {}. I'd start him this week.", w.people[yi].name, w.people[si].name),
                Some("He's sure. He's been sure before.".into()),
            );
            s.pick(
                75,
                format!("Do it: start {}", w.people[yi].name),
                "Hand him something",
                "firm",
                "Your depth chart changes. The young player lifts; the veteran stews.",
            );
            s.pick(
                76,
                "Give him more snaps, but keep the starter",
                "Listen",
                "calm",
                "A rotation: nobody's promised anything.",
            );
            s.pick(
                77,
                "No. The veteran earned it.",
                "Tell him straight",
                "calm",
                "Your coach is overruled; the veteran never knows.",
            );
            let _ = player_link(w, yi);
        }
        2 => {
            let id = s.b as u32;
            let Some(i) = super::person(w, id) else {
                s.end();
                return;
            };
            s.who.push(Who::Player(id));
            s.narrate(format!("The head trainer stops you in the hall about {}.", player_link(w, i)));
            s.them(
                "Head trainer",
                "His hamstring is tight and his numbers are down. A week off now saves us a month later.",
                None,
            );
            s.pick(78, "Rest him", "Listen", "calm", "He sits a week. Lower risk of a soft-tissue injury.");
            s.pick(79, "He plays", "Tell him straight", "firm", "He plays; the risk is yours.");
        }
        _ => s.end(),
    }
}

fn proposal_answer(w: &mut World, s: &mut Scene, code: u32) {
    match code {
        75 => {
            let (y, s0) = ((s.b >> 32) as u32, (s.b & 0xFFFF_FFFF) as u32);
            if let Some(yi) = super::person(w, y) {
                let pos = w.people[yi].player.position;
                match w.seat_depth.iter_mut().find(|x| x.0 == pos) {
                    Some(x) => {
                        x.1.retain(|&i| i != y);
                        x.1.insert(0, y);
                    }
                    None => w.seat_depth.push((pos, vec![y, s0])),
                }
                talks::mood_event(w, y, Reason::Praise, 0.15);
            }
            talks::mood_event(w, s0, Reason::Benched, -0.2);
            w.folk.mine.nudge(coach_key(s.a as usize), 0.06);
            s.outcome.push("Your depth chart changed: the young player starts.".into());
        }
        76 => {
            w.folk.mine.nudge(coach_key(s.a as usize), 0.02);
            s.outcome.push("A rotation, for now.".into());
        }
        77 => {
            w.folk.mine.nudge(coach_key(s.a as usize), -0.04);
            s.outcome.push("You overruled him. The veteran keeps the job.".into());
        }
        78 => {
            let id = s.b as u32;
            let until = s.team.map_or(w.today.add(7), |t| {
                facts::team_games(w, t).iter().map(|g| g.0).find(|&d| d >= w.today).unwrap_or(w.today.add(7))
            });
            w.folk.out.push((id, until, "rested by the trainer".into()));
            s.outcome.push("He sits this week.".into());
        }
        _ => {
            s.outcome.push("He plays.".into());
        }
    }
    s.end();
}

pub(crate) fn open(w: &mut World, s: &mut Scene) {
    match s.kind {
        Kind::Interview => interview_open(w, s),
        Kind::Firing => firing_open(w, s),
        Kind::Proposal => proposal_open(w, s),
        _ => s.end(),
    }
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32, _picked: &Pick) {
    match s.kind {
        Kind::Interview if code == 69 => {
            s.outcome.push(format!("You interviewed {}.", w.coaches[s.a as usize].name));
            s.end();
        }
        Kind::Interview => interview_answer(w, s, code),
        Kind::Firing => firing_answer(w, s, code),
        Kind::Proposal => proposal_answer(w, s, code),
        _ => s.end(),
    }
}

pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    match s.kind {
        Kind::Proposal => {
            s.handed = Some("the staff".into());
            s.outcome.push("You never answered; the staff kept things as they were.".into());
            w.folk.mine.nudge(coach_key(s.a as usize), -0.02);
        }
        Kind::Firing => {
            s.handed = Some("nobody".into());
            s.outcome.push("You let it drop. He stays.".into());
        }
        _ => {
            s.handed = Some("nobody".into());
        }
    }
}

/// The staff's year and week: deals renewed each February, new hires get
/// deals, AI high school staffs turn over, proposals reach you.
pub(crate) fn day(w: &mut World, d: Date) {
    if d.weekday() == Weekday::Mon {
        ensure(w);
        if w.seat.is_some() {
            proposals_week(w, d);
        }
    }
    if d == w.dates.signing_day && w.folk.staff.renewed < w.year {
        w.folk.staff.renewed = w.year;
        let year = w.year;
        // A deal that ran out is signed again for its usual length.
        let lengths: Vec<i32> = w
            .folk
            .staff
            .deals
            .iter()
            .map(|dl| {
                w.coaches
                    .get(dl.coach)
                    .and_then(|co| co.job)
                    .map_or(1, |j| deal_terms(w, dl.coach, j.employer, j.role).0)
            })
            .collect();
        for (dl, years) in w.folk.staff.deals.iter_mut().zip(lengths) {
            if dl.through < year {
                dl.through = year + years - 1;
                dl.salary = (dl.salary * 1.03).round();
            }
        }
        // High school staffs turn over (about one school in seven a year).
        let n = w.high_schools.len();
        for h in 0..n {
            if super::roll(w, 0x45F ^ year as u64, h as u64) < 0.14 {
                match w.folk.hs_staffs.terms.iter_mut().find(|x| x.0 == h as u32) {
                    Some(x) => x.1 = x.1.wrapping_add(1),
                    None => w.folk.hs_staffs.terms.push((h as u32, 1)),
                }
            }
        }
        let cut = w.folk.staff.owed.iter().position(|x| x.0 >= year - 5).unwrap_or(0);
        w.folk.staff.owed.drain(..cut);
    }
    let _ = (Level::Nfl, Trait::Ego, DocRow::default(), Doc::default());
}
