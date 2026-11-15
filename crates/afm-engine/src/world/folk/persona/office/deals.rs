//! Deals with the people in your building (sections 8.2 and 11): a job
//! offer to a coach you're hiring, a raise, more years, a title or the
//! play-calling for one of your own, and your own extension with your
//! boss. Each is weighed and made real by the built system that already
//! prices it: the hiring talks (`hiring::weigh`, `would_take`, `offer`:
//! C28-C34, C44-C56), the staff deals (`staffing::raise`), and the boss's
//! asks (`boss::ask_yes`, `ask_now`). Anything else falls to lane N3's
//! deals.

use super::super::Them;
use super::super::terms::{Best, Counter, DealKind, Give, Offer, Weighed};
use crate::world::World;
use crate::world::coach::Role;
use crate::world::folk::boss::{self, AskKind};
use crate::world::folk::hiring::{self, PROMISE_CALLING, PROMISE_TITLE, Terms};
use crate::world::folk::scene::Kind;
use crate::world::folk::{Team, staffing};

/// An offer that is about a coach's job: money, years, a title, the
/// play-calling or the job itself.
fn staff_deal(o: &Offer) -> bool {
    o.kind == DealKind::Staff
        || o.gives
            .iter()
            .any(|g| matches!(g, Give::Money(_) | Give::Years(_) | Give::Title | Give::Calling | Give::StaffJob(_)))
}

/// The job you're talking to a coach about: his latest interview's, or the
/// hiring talks you have open with him.
pub fn role_for(w: &World, c: usize) -> Option<Role> {
    w.folk
        .scenes
        .iter()
        .rev()
        .find(|s| s.kind == Kind::Interview && s.a as usize == c)
        .map(|s| staffing::role_from(s.b))
        .or_else(|| hiring::talk(w, c).map(|t| staffing::role_from(t.role)))
}

/// The hiring terms an offer carries: your number (else his ask), the
/// years (else two), a title and the play-calling as the built promises.
pub fn terms(w: &World, c: usize, role: Role, o: &Offer) -> Terms {
    let mut t = Terms { salary: 0.0, years: 2, promises: 0, clauses: Vec::new() };
    for g in &o.gives {
        match g {
            Give::Money(x) if *x > 0.0 => t.salary = *x,
            Give::Years(y) => t.years = (*y).clamp(1, 5),
            Give::Title => t.promises |= PROMISE_TITLE,
            Give::Calling => t.promises |= PROMISE_CALLING,
            _ => {}
        }
    }
    if t.salary <= 0.0 {
        t.salary = hiring::ask(w, c, role);
    }
    t
}

/// Under this share of his ask an offer insults him (the hiring talks warn
/// at 12-40% under; this is past the deepest warning). Proposed, not
/// confirmed.
pub const LOWBALL: f64 = 0.6;

/// His best option: a coach in a job stays where he is.
pub fn best(w: &World, him: Them) -> Option<Best> {
    let Them::Coach(c) = him else { return None };
    w.coaches.get(c)?;
    Some(Best::Stay)
}

/// How he takes an offer, for the people this lane plays; None leaves it
/// to lane N3.
pub fn weigh(w: &World, him: Them, o: &Offer) -> Option<Weighed> {
    let team = Team::seat(w)?;
    match him {
        Them::Coach(c) if c < w.coaches.len() && staff_deal(o) => {
            if super::asks::on_your_staff(w, c) {
                return Some(own(w, c, o));
            }
            let role = role_for(w, c)?;
            if !staffing::openings(w).contains(&role) {
                return Some(Weighed::No);
            }
            let t = terms(w, c, role, o);
            let now = hiring::pay_now(w, c);
            if t.salary < LOWBALL * hiring::ask(w, c, role) {
                return Some(Weighed::Lowball);
            }
            let k = hiring::weigh(w, c, role, &t, now);
            if k.score > k.bar {
                return Some(Weighed::Yes);
            }
            Some(match hiring::would_take(w, c, role, &t, now) {
                Some(s) => Weighed::Counter(Counter {
                    asks: vec!["he'd need more money to take the job".into()],
                    gives: vec![Give::Money(s)],
                    terms: Vec::new(),
                    why: None,
                }),
                None => Weighed::No,
            })
        }
        Them::Cast(r, _) if super::super::makeup::kind(w, him) == super::super::wants::Kind::Boss => {
            let _ = r;
            if o.kind != DealKind::Yours && !o.gives.iter().any(|g| matches!(g, Give::Years(_) | Give::Money(_))) {
                return None;
            }
            // Your extension (topic 12): his own answer, the day's roll.
            if boss::asked_lately(w, AskKind::Extension) {
                return Some(Weighed::No);
            }
            Some(if boss::ask_yes(w, team, AskKind::Extension) { Weighed::Yes } else { Weighed::No })
        }
        _ => None,
    }
}

/// One of your own assistants takes a raise, more years, a title or the
/// play-calling; a pay cut is a no.
fn own(w: &World, c: usize, o: &Offer) -> Weighed {
    let salary = w.folk.staff.deals.iter().find(|d| d.coach == c).map(|d| d.salary);
    for g in &o.gives {
        if let (Give::Money(x), Some(s)) = (g, salary)
            && *x > 0.0
            && *x < s
        {
            return Weighed::No;
        }
    }
    if o.gives.iter().any(|g| matches!(g, Give::Calling)) && side(w, c).is_none() {
        return Weighed::No;
    }
    Weighed::Yes
}

/// His side of the ball when he's a coordinator: true offense.
fn side(w: &World, c: usize) -> Option<bool> {
    match w.coaches[c].job?.role {
        Role::OffensiveCoordinator => Some(true),
        Role::DefensiveCoordinator => Some(false),
        _ => None,
    }
}

/// Makes an agreed deal real (after your confirm), by the built system
/// that prices it; None leaves it to lane N3's `agree`.
pub fn agree(w: &mut World, _id: u32, him: Them, o: &Offer) -> Option<Vec<String>> {
    let team = Team::seat(w)?;
    match him {
        Them::Coach(c) if c < w.coaches.len() && staff_deal(o) => {
            if super::asks::on_your_staff(w, c) {
                return Some(own_agree(w, c, o));
            }
            let role = role_for(w, c)?;
            let t = terms(w, c, role, o);
            Some(vec![hiring::offer(w, c, role, t).unwrap_or_else(|e| e)])
        }
        Them::Cast(..) if super::super::makeup::kind(w, him) == super::super::wants::Kind::Boss => {
            if o.kind != DealKind::Yours && !o.gives.iter().any(|g| matches!(g, Give::Years(_) | Give::Money(_))) {
                return None;
            }
            let (yes, words) = boss::ask_now(w, team, AskKind::Extension);
            Some(vec![format!("You asked for an extension: {}", if yes { "granted." } else { "no." }), words.into()])
        }
        _ => None,
    }
}

fn own_agree(w: &mut World, c: usize, o: &Offer) -> Vec<String> {
    let mut out = Vec::new();
    let name = w.coaches[c].name.clone();
    for g in &o.gives {
        match g {
            Give::Money(x) => {
                let Some(s) = w.folk.staff.deals.iter().find(|d| d.coach == c).map(|d| d.salary) else { continue };
                if *x > s {
                    match staffing::raise(w, c, *x / s - 1.0) {
                        Ok(t) => out.push(t),
                        Err(e) => out.push(e),
                    }
                }
            }
            Give::Years(y) => {
                let year = w.year;
                if let Some(d) = w.folk.staff.deals.iter_mut().find(|d| d.coach == c) {
                    d.through = (d.through + i32::from(*y)).min(year + 5);
                    out.push(format!("{name}'s deal now runs through {}.", d.through));
                }
            }
            Give::Title => {
                let t = if side(w, c).is_some() { 2 } else { 1 };
                w.folk.hiring.titles.retain(|x| x.0 != c);
                w.folk.hiring.titles.push((c, t));
                w.folk.mine.nudge(crate::world::folk::coach_key(c), 0.03);
                out.push(format!("{name} is now {}.", hiring::title_words(t)));
            }
            Give::Calling => {
                let Some(off) = side(w, c) else { continue };
                if let Some(you) = w.career.as_ref().map(|k| k.coach)
                    && let Some(y) = w.coaches[you].yours.as_mut()
                {
                    if off {
                        y.calls_offense = false;
                    } else {
                        y.calls_defense = false;
                    }
                }
                out.push(format!("{name} calls the {} plays now.", if off { "offense's" } else { "defense's" }));
            }
            _ => {}
        }
    }
    out
}
