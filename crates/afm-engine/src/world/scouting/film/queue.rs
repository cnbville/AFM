//! Your staff's film queue (SK55): each Monday your staff send you a few
//! cut-ups, each with who sent it, why and by when ("Dunbar: see Hayes
//! before Friday's offer call"). You order the queue and skip what you
//! like; the sender sees what you skipped. A college's position coaches
//! send their recruits, an NFL club's scouts the board's prospects in the
//! spring and the coordinators your own men in season; a high school
//! coach's assistants are teachers the world keeps no record of, so his
//! queue stays empty. Sizes are Claude's, proposed, not confirmed.

use super::tape::Cut;
use crate::calendar::{Date, Weekday};
use crate::world::coach::Group;
use crate::world::scouting::readers::{self, COACH, Staff};
use crate::world::{Stage, World};

/// One cut-up sent to you.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Queued {
    pub id: u32,
    pub from: String,
    /// The sender as a coach (his respect moves), or None for a scout.
    pub coach: Option<usize>,
    pub job: String,
    pub why: String,
    pub by: Date,
    pub sent: Date,
    pub cut: Cut,
    /// 0 waiting, 1 watched, 2 skipped, 3 ran past its date.
    pub state: u8,
}

pub const WAITING: u8 = 0;
pub const WATCHED: u8 = 1;
pub const SKIPPED: u8 = 2;
pub const LAPSED: u8 = 3;

fn friday(d: Date) -> Date {
    let mut d = d;
    while d.weekday() != Weekday::Fri {
        d = d.add(1);
    }
    d
}

/// Monday: what's waiting from last week lapses, and a new batch comes in.
pub(crate) fn week(w: &mut World, d: Date) {
    let Some(staff) = readers::seat_staff(w) else { return };
    for q in w.scouting.film.queue.iter_mut().filter(|q| q.state == WAITING && q.by < d) {
        q.state = LAPSED;
    }
    let cut = d.add(-120);
    w.scouting.film.queue.retain(|q| q.sent >= cut);
    let you = readers::you(w);
    let mine = you.map(|c| COACH + c as u32);
    let seen_lately = |w: &World, id: u32| {
        w.scouting.dept.looks.get(&id).is_some_and(|v| v.iter().any(|l| Some(l.who) == mine && l.date.days_to(d) <= 45))
            || w.scouting.film.queue.iter().any(|q| q.id == id && q.state == WAITING)
    };
    let by = friday(d);
    let mut add: Vec<Queued> = Vec::new();
    match staff {
        Staff::Program(p) => {
            // The recruits your staff follow, best first.
            let mut ids: Vec<(u8, u32, usize)> = w
                .scouting
                .dept
                .my_players
                .iter()
                .filter_map(|f| super::super::place(w, f.id).map(|i| (f.id, i)))
                .filter(|&(_, i)| matches!(w.people[i].stage, Stage::HighSchool | Stage::PrepSchool))
                .filter(|&(id, _)| !seen_lately(w, id))
                .filter(|&(id, _)| super::super::own::film_cost(w, id).is_ok())
                .map(|(id, i)| (w.people[i].stars, id, i))
                .collect();
            ids.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            let sd = w.dates.signing_day;
            let early = w.dates.early_signing;
            for (stars, id, i) in ids.into_iter().take(4) {
                let pr = &w.programs[p];
                let g = Group::of(w.people[i].player.position);
                let c = pr.position_coaches[g as usize]
                    .or(if g.offense() { pr.oc } else { pr.dc })
                    .filter(|&c| Some(c) != you);
                let Some(c) = c else { continue };
                let name = w.people[i].name.clone();
                let last = name.split_whitespace().last().unwrap_or(&name).to_string();
                let soon = |x: Date| x >= d && x.days_to(d).abs() <= 21;
                let (why, until) = if early.is_some_and(soon) {
                    (format!("see {last} before the early signing period"), early.unwrap_or(by).add(-1))
                } else if soon(sd) {
                    (format!("see {last} before signing day"), sd.add(-1))
                } else if stars >= 4 {
                    (format!("see {last} before Friday's offer call: other staffs were at his game"), by)
                } else {
                    (format!("see {last} before Friday's offer call"), by)
                };
                add.push(Queued {
                    id,
                    from: w.coaches[c].name.clone(),
                    coach: Some(c),
                    job: g.coach_title().to_string(),
                    why,
                    by: until,
                    sent: d,
                    cut: Cut::BestOpponent,
                    state: WAITING,
                });
            }
        }
        Staff::Club(t) => {
            let dt = &w.dates;
            let spring = (2..=4).contains(&d.parts().1) && d < dt.draft[0].date;
            if spring {
                // The board's prospects nobody senior has seen yet.
                let order: Vec<u32> = w.scouting.board.order.iter().copied().take(80).collect();
                let mut ids: Vec<u32> =
                    if order.is_empty() { w.scouting.dept.my_players.iter().map(|f| f.id).collect() } else { order };
                ids.retain(|&id| {
                    super::super::place(w, id)
                        .is_some_and(|i| matches!(w.people[i].stage, Stage::College | Stage::Declared))
                        && !seen_lately(w, id)
                        && super::super::own::film_cost(w, id).is_ok()
                });
                let scouts: Vec<(String, &'static str)> = w
                    .clubs
                    .get(t)
                    .map(|c| {
                        c.scouts
                            .iter()
                            .filter(|s| {
                                matches!(
                                    s.role,
                                    crate::world::pro::scouts::Role::National | crate::world::pro::scouts::Role::Area
                                )
                            })
                            .map(|s| {
                                (
                                    s.name.clone(),
                                    if s.role == crate::world::pro::scouts::Role::National {
                                        "national scout"
                                    } else {
                                        "area scout"
                                    },
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let draft = dt.draft[0].date;
                for (k, id) in ids.into_iter().take(4).enumerate() {
                    let Some((from, job)) = scouts.get(k % scouts.len().max(1)).cloned() else { break };
                    let i = super::super::place(w, id).unwrap_or(0);
                    let name = w.people[i].name.clone();
                    let last = name.split_whitespace().last().unwrap_or(&name).to_string();
                    let (why, until) = if draft.days_to(d).abs() <= 14 {
                        (format!("see {last} before the draft: we're split on him"), draft.add(-1))
                    } else {
                        (format!("see {last} before the board meeting: I have him higher than the room"), by)
                    };
                    add.push(Queued {
                        id,
                        from,
                        coach: None,
                        job: job.to_string(),
                        why,
                        by: until,
                        sent: d,
                        cut: Cut::BestOpponent,
                        state: WAITING,
                    });
                }
            } else if d >= dt.nfl_kickoff && d <= dt.super_bowl {
                // Your coordinators want your eye on one of your own.
                let tm = &w.nfl[t];
                for (side, c) in [(true, tm.oc), (false, tm.dc)] {
                    let Some(c) = c.filter(|&c| Some(c) != you) else { continue };
                    let pick = w
                        .clubs
                        .get(t)
                        .map(|cl| cl.roster.clone())
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|i| w.people.get(i).map(|p| (i, p)))
                        .filter(|(_, p)| Group::of(p.player.position).offense() == side)
                        .map(|(_, p)| p.player.id)
                        .filter(|&id| !seen_lately(w, id))
                        .min_by_key(|&id| crate::world::season::key_uniform(d.0 as u64, id as u64).to_bits());
                    let Some(id) = pick else { continue };
                    let i = super::super::place(w, id).unwrap_or(0);
                    let name = w.people[i].name.clone();
                    let last = name.split_whitespace().last().unwrap_or(&name).to_string();
                    add.push(Queued {
                        id,
                        from: w.coaches[c].name.clone(),
                        coach: Some(c),
                        job: if side { "offensive coordinator" } else { "defensive coordinator" }.to_string(),
                        why: format!("look at {last}'s last three games before Wednesday's install"),
                        by: d.add(2),
                        sent: d,
                        cut: Cut::Ours,
                        state: WAITING,
                    });
                }
            }
        }
        Staff::School(_) => {}
    }
    w.scouting.film.queue.extend(add);
}

/// The queue as you see it: what's waiting, in your order.
pub fn waiting(w: &World) -> Vec<(usize, &Queued)> {
    w.scouting.film.queue.iter().enumerate().filter(|(_, q)| q.state == WAITING).collect()
}

/// Moves a waiting cut-up up or down your queue.
pub fn shift(w: &mut World, k: usize, up: bool) -> Result<(), String> {
    let wait: Vec<usize> = waiting(w).into_iter().map(|x| x.0).collect();
    let at = wait.iter().position(|&x| x == k).ok_or("That cut-up isn't waiting.")?;
    let other = if up { at.checked_sub(1) } else { (at + 1 < wait.len()).then_some(at + 1) };
    if let Some(o) = other {
        w.scouting.film.queue.swap(k, wait[o]);
    }
    Ok(())
}

/// Skips a cut-up: the sender sees it.
pub fn skip(w: &mut World, k: usize) -> Result<String, String> {
    let q = w.scouting.film.queue.get_mut(k).filter(|q| q.state == WAITING).ok_or("That cut-up isn't waiting.")?;
    q.state = SKIPPED;
    let (from, coach) = (q.from.clone(), q.coach);
    if let Some(c) = coach {
        w.folk.mine.nudge(crate::world::folk::coach_key(c), -0.01);
    }
    Ok(format!("Skipped. {from} sees it."))
}

/// A session on a player clears his cut-up from the queue: the sender
/// sees you watched it.
pub fn watched(w: &mut World, id: u32) {
    let mut nudges = Vec::new();
    for q in w.scouting.film.queue.iter_mut().filter(|q| q.id == id && q.state == WAITING) {
        q.state = WATCHED;
        if let Some(c) = q.coach {
            nudges.push(c);
        }
    }
    for c in nudges {
        w.folk.mine.nudge(crate::world::folk::coach_key(c), 0.005);
    }
}
