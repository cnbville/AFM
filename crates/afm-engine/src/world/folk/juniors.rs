//! Hiring your graduate assistants, analysts and quality-control coaches
//! (C36; GAPS "you can't see or fire your graduate assistants"). A junior
//! post is filled the way real staffs fill one: the position coach brings
//! a young man he knows, or you pick from who wants in: former players
//! just done who want to coach (your own first, then men your staff
//! coached, then men from your state), young coaches at high schools and
//! small colleges, juniors at other programs, and for an analyst post the
//! men out of work between jobs. The pay is the post's; a man takes it when
//! it is a step up for him or he is out of work. A former player always
//! takes it: a post is the way in.
//!
//! Sizes are Claude's picks (proposed, not confirmed): the ages (a
//! graduate assistant 28 or younger, an analyst 66, a quality-control coach
//! 40; a former player 27 for a graduate assistant post, 34 otherwise), an
//! hour of your week a hire, the list's 60 names.

use super::{Team, coach_key, facts, network, talks};
use crate::calendar::Loudness;
use crate::world::World;
use crate::world::coach::{Employer, Group, Post, Role};
use crate::world::data::Division;
use crate::world::former::Life;

/// A post's key on the screens.
pub fn post_key(p: Post) -> &'static str {
    match p {
        Post::GradAssistant => "ga",
        Post::Analyst => "analyst",
        Post::QualityControl => "qc",
    }
}

pub fn post_from(k: &str) -> Option<Post> {
    match k {
        "ga" => Some(Post::GradAssistant),
        "analyst" => Some(Post::Analyst),
        "qc" => Some(Post::QualityControl),
        _ => None,
    }
}

/// Your junior posts: each post, how many there are and how many are open.
pub fn posts(w: &World) -> Vec<(Post, usize, usize)> {
    let Some(team) = Team::seat(w) else { return Vec::new() };
    let e = team.employer();
    let open = crate::world::staff::junior_openings(w, e);
    crate::world::staff::junior_slots(w, e)
        .into_iter()
        .map(|(p, n)| (p, n, open.iter().find(|x| x.0 == p).map_or(0, |x| x.1)))
        .collect()
}

fn is_open(w: &World, post: Post) -> bool {
    posts(w).iter().any(|x| x.0 == post && x.2 > 0)
}

/// Who you'd hire: a coach the world keeps, or a former player in coaching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applicant {
    Coach(usize),
    Former(u32),
}

/// A man you could hire into a junior post, as you can see him.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Pick {
    pub coach: Option<usize>,
    pub former: Option<u32>,
    pub name: String,
    pub age: u32,
    pub now: String,
    /// His playing past, for a former player.
    pub played: Option<String>,
    /// Why he's on your list ("Played for you", "From your state").
    pub tie: String,
    /// The room he'd work with: the group he came up in or played.
    pub group: Group,
    /// Your read of his teaching in that room (a coach only; a stranger's
    /// is the whole scale).
    pub read: Option<(u8, u8)>,
    pub potential: String,
    /// He'd take it.
    pub takes: bool,
    #[serde(skip)]
    rank: (u8, f64),
}

/// Whether a coach could take a junior post on your staff (the ages, no
/// sitting head coach, no man in a full-time FBS or NFL job as a graduate
/// assistant, no man who has served his two graduate years).
pub fn fits(w: &World, c: usize, post: Post) -> bool {
    let Some(team) = Team::seat(w) else { return false };
    let Some(co) = w.coaches.get(c) else { return false };
    if co.retired || co.show_cause.is_some_and(|y| y >= w.year) || w.career.as_ref().is_some_and(|k| k.coach == c) {
        return false;
    }
    if co.job.is_some_and(|j| Team::from_employer(j.employer) == team || j.role == Role::HeadCoach)
        || co.junior_now().is_some_and(|j| j.employer == team.employer())
    {
        return false;
    }
    // A man out of work with no coaching behind him isn't on the market.
    if co.job.is_none() && co.junior_now().is_none() && co.history.is_empty() {
        return false;
    }
    let age = co.age_on(w.today);
    match post {
        Post::GradAssistant => {
            let big = co.job.is_some_and(|j| match j.employer {
                Employer::Nfl(_) => true,
                Employer::College(p) => w.programs[p].division == Division::FBS,
                _ => false,
            });
            let served = co.junior.is_some_and(|j| {
                j.post == Post::GradAssistant && j.until.is_some_and(|u| u - j.since >= 2) && co.junior_now().is_none()
            });
            age <= 28 && !big && !served
        }
        Post::Analyst => age <= 66,
        Post::QualityControl => age <= 40,
    }
}

/// Whether a coach who fits would take the post: a step up, or out of work.
fn takes(w: &World, c: usize) -> bool {
    let co = &w.coaches[c];
    (co.job.is_none() && co.junior_now().is_none()) || super::hiring::junior_step(w, c) > 0.0
}

/// The men who could take a junior post on your staff, the closest ties
/// first (at most 60).
pub fn candidates(w: &World, post: Post) -> Vec<Pick> {
    let Some(team) = Team::seat(w) else { return Vec::new() };
    let x = super::coach_look::Ctx::new(w);
    let state = match team {
        Team::College(p) => Some(w.programs[p].state),
        _ => None,
    };
    let mut staff: Vec<u32> = facts::assistants(w, team).into_iter().map(|x| x.0 as u32).collect();
    staff.extend(w.career.as_ref().map(|k| k.coach as u32));
    let mut out: Vec<Pick> = Vec::new();
    for c in 0..w.coaches.len() {
        if !fits(w, c, post) {
            continue;
        }
        let co = &w.coaches[c];
        let warm = network::warmth(w, c);
        let tie = if warm >= 0.3 {
            (3, "Someone you know")
        } else if state.is_some() && co.home_state == state {
            (1, "From your state")
        } else {
            (0, "")
        };
        let ok = takes(w, c);
        let g = co.came_up;
        out.push(Pick {
            coach: Some(c),
            former: None,
            name: co.name.clone(),
            age: co.age_on(w.today),
            now: super::staffing::now_words(w, c),
            played: None,
            tie: tie.1.into(),
            group: g,
            read: Some(super::coach_look::read_job_in(w, &x, c, Role::PositionCoach(g))),
            potential: super::coach_look::potential_in(w, &x, c).0,
            takes: ok,
            rank: (tie.0 * 2 + ok as u8 * 8, co.reputation),
        });
    }
    let cap = if post == Post::GradAssistant { 27 } else { 34 };
    for f in &w.formers.list {
        if f.life != Life::Coaching || f.coach.is_some() {
            continue;
        }
        let age = f.age(w.year);
        if !(22..=cap).contains(&age) {
            continue;
        }
        let yours = match team {
            Team::College(p) => f.colleges.iter().any(|x| x.0 as usize == p),
            Team::Nfl(t) => f.clubs.iter().any(|x| x.0 as usize == t),
            _ => false,
        };
        let coached = f.coached_by.iter().any(|c| staff.contains(c));
        let tie = if yours {
            (3, if matches!(team, Team::Nfl(_)) { "Played for your club" } else { "Played for you" })
        } else if coached {
            (2, "Your staff coached him")
        } else if state.is_some_and(|s| s == f.area as usize) {
            (1, "From your state")
        } else {
            (0, "")
        };
        out.push(Pick {
            coach: None,
            former: Some(f.id),
            name: f.name.clone(),
            age,
            now: f.now(w),
            played: Some(f.played.words(w)),
            tie: tie.1.into(),
            group: f.played.group(),
            read: None,
            potential: "He hasn't coached yet".into(),
            takes: true,
            rank: (tie.0 * 2 + 8, f.played.fame()),
        });
    }
    out.sort_by(|a, b| {
        b.rank
            .0
            .cmp(&a.rank.0)
            .then(b.rank.1.total_cmp(&a.rank.1))
            .then(a.coach.cmp(&b.coach))
            .then(a.former.cmp(&b.former))
    });
    out.truncate(60);
    out
}

/// Your position coach for a room (the man a junior there works under).
fn room_coach(w: &World, g: Group) -> Option<usize> {
    match Team::seat(w)? {
        Team::College(p) => w.programs[p].position_coaches.get(g as usize).copied().flatten(),
        Team::Nfl(t) => {
            if g.offense() {
                w.nfl[t].oc
            } else {
                w.nfl[t].dc
            }
        }
        _ => None,
    }
}

/// The text of a new junior on your staff, and his first days with you.
fn welcome(w: &mut World, c: usize, post: Post, g: Group, lead: String) -> String {
    w.coaches[c].mentor = room_coach(w, g);
    w.folk.mine.set_trust(coach_key(c), 0.6 + 0.2 * network::warmth(w, c));
    network::touch(w, c, 0.15);
    let pay = super::staffing::junior_salary(w, c).unwrap_or(0.0);
    let text = format!(
        "{lead} as a {} with the {}, at ${} a year.",
        post.name(),
        g.name(),
        facts::thousands((pay / 1000.0) as u32) + ",000"
    );
    super::post_mine(w, w.today, Loudness::Small, false, "people: staff", &text);
    text
}

/// Hires a man into an open junior post on your staff (an hour of your
/// week). `group` is the room he works with; his own by default.
pub fn hire(w: &mut World, who: Applicant, post: Post, group: Option<Group>) -> Result<String, String> {
    let team = Team::seat(w).ok_or("You have no team.")?;
    let e = team.employer();
    if !is_open(w, post) {
        return Err(format!("You have no open {} post. Let someone go first.", post.name()));
    }
    match who {
        Applicant::Coach(c) => {
            if !fits(w, c, post) {
                return Err(format!("He can't take a {} post.", post.name()));
            }
            if !takes(w, c) {
                return Err(format!("He'd rather stay where he is: {}.", super::staffing::now_words(w, c)));
            }
            if !talks::spend(w, 1.0) {
                return Err("Your week is full: a hire takes an hour.".into());
            }
            let g = group.unwrap_or(w.coaches[c].came_up);
            crate::world::staff::take_junior(w, c, e, post, g);
            let name = w.coaches[c].name.clone();
            Ok(welcome(w, c, post, g, format!("{name} joins your staff")))
        }
        Applicant::Former(id) => {
            let k = w.formers.list.binary_search_by_key(&id, |f| f.id).map_err(|_| "No such former player.")?;
            let f = &w.formers.list[k];
            if f.life != Life::Coaching || f.coach.is_some() {
                return Err("He isn't looking for a coaching job.".into());
            }
            let g = group.unwrap_or(f.played.group());
            if !talks::spend(w, 1.0) {
                return Err("Your week is full: a hire takes an hour.".into());
            }
            let c = crate::world::staff::former_junior(w, k, e, post, g);
            let name = w.coaches[c].name.clone();
            Ok(welcome(w, c, post, g, format!("{name} joins your staff")))
        }
    }
}

/// Lets the room's coach bring a young man he knows into an open junior
/// post (how most graduate assistants are hired): no hour of yours.
pub fn staff_pick(w: &mut World, post: Post, group: Group) -> Result<String, String> {
    let team = Team::seat(w).ok_or("You have no team.")?;
    if !is_open(w, post) {
        return Err(format!("You have no open {} post. Let someone go first.", post.name()));
    }
    let c = crate::world::staff::new_junior(w, team.employer(), post, Some(group));
    let name = w.coaches[c].name.clone();
    let age = w.coaches[c].age_on(w.today);
    let by = match room_coach(w, group) {
        Some(k) => format!("{} brought in {name}, {age},", w.coaches[k].name),
        None => format!("Your staff brought in {name}, {age},"),
    };
    Ok(welcome(w, c, post, group, by))
}
