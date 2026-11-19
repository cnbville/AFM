//! Deals with the people in your building (sections 8.2 and 11): a job
//! offer to a coach you're hiring, a raise, more years, a title or the
//! play-calling for one of your own, and your own extension with your
//! boss. Each is weighed and made real by the built system that already
//! prices it: the hiring talks (`hiring::weigh`, `would_take`, `offer`:
//! C28-C34, C44-C56) and the staff deals (`staffing::raise`). Your
//! extension is weighed as you put it (`folk::asked`), never on the boss
//! meeting's pick. Anything else falls to lane N3's deals.

use super::super::Them;
use super::super::terms::{Best, Counter, DealKind, Give, Offer, Weighed};
use crate::world::World;
use crate::world::coach::Role;
use crate::world::folk::asked::{self, Answer, Case, Said};
use crate::world::folk::boss::{AskKind, GoalKind};
use crate::world::folk::hiring::{self, PROMISE_CALLING, PROMISE_TITLE, Terms};
use crate::world::folk::scene::Kind;
use crate::world::folk::{Team, cast, staffing};

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

/// The terms as he weighs them: a goal bonus counts for what he thinks
/// it's worth (`goal_worth`) on top of the salary.
pub fn weighed_terms(w: &World, c: usize, role: Role, o: &Offer) -> Terms {
    let mut t = terms(w, c, role, o);
    // A goal bonus counts for what he thinks it's worth: its odds this
    // season times the bonus (lane N2's `terms::likely`).
    t.salary += goal_worth(w, o, t.salary);
    t
}

/// A staff contract's goal bonus, as a share of his salary, for each
/// season goal in the deal (the team's wins, a title, a rank: lane N2's
/// season terms). Proposed, not confirmed.
pub const GOAL_BONUS: f64 = 0.1;

/// The season goals an offer carries for the team (lane N2's terms read
/// on a season).
pub fn goals(o: &Offer) -> Vec<crate::world::folk::usage::Usage> {
    o.terms.iter().filter(|u| u.season).cloned().collect()
}

/// What the goals in an offer are worth to him now: each one's odds times
/// its bonus.
fn goal_worth(w: &World, o: &Offer, salary: f64) -> f64 {
    let Some(team) = Team::seat(w) else { return 0.0 };
    let year = crate::world::folk::terms::season_of(w.today);
    goals(o)
        .iter()
        .map(|u| crate::world::folk::terms::likely(w, crate::world::folk::terms::Who::Team(team), u, year))
        .sum::<f64>()
        * GOAL_BONUS
        * salary
}

/// Writes each season goal in an agreed staff deal as a clause on lane
/// N2's books (`terms::add`): settled after the season, paid when met.
fn add_goals(w: &mut World, c: usize, o: &Offer, salary: f64) -> Vec<String> {
    use crate::world::folk::terms::{self as st, Clause, On, Who};
    let Some(team) = Team::seat(w) else { return Vec::new() };
    let season = st::season_of(w.today);
    let made = w.today;
    let name = w.coaches[c].name.clone();
    let mut out = Vec::new();
    for u in goals(o) {
        out.push(format!("{name} gets a bonus if {}.", st::words(&u)));
        st::add(
            w,
            Clause {
                on: On::Staff,
                who: Who::Team(team),
                coach: Some(c),
                term: u,
                season,
                pays: GOAL_BONUS * salary,
                made,
                met: None,
                settled: false,
            },
        );
    }
    out
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
            let t = weighed_terms(w, c, role, o);
            let now = hiring::pay_now(w, c);
            if terms(w, c, role, o).salary < LOWBALL * hiring::ask(w, c, role) {
                return Some(Weighed::Lowball);
            }
            let k = hiring::weigh(w, c, role, &t, now);
            if k.score > k.bar {
                return Some(Weighed::Yes);
            }
            // His number, less what the goals are worth to him.
            let worth = t.salary - terms(w, c, role, o).salary;
            // He names his number, the way it's said across the table (talk
            // replies item 4: "I'd need more money" left you guessing).
            Some(match hiring::would_take(w, c, role, &t, now) {
                Some(s) => Weighed::Counter(Counter {
                    asks: vec![format!(
                        "he names his number: {} a year over {} years to take the job",
                        super::super::terms::money_words(s - worth),
                        t.years
                    )],
                    gives: vec![Give::Money(s - worth), Give::Years(t.years)],
                    terms: Vec::new(),
                    why: None,
                }),
                None => Weighed::No,
            })
        }
        Them::Cast(..) if super::super::makeup::kind(w, him) == super::super::wants::Kind::Boss => {
            if !extension(o) {
                return None;
            }
            // Your extension (topic 12), weighed as you put it: what you put
            // up for it, his trust, his budget and his nature
            // (`folk::asked`).
            let a = asked::money(w, team, AskKind::Extension, &case_of(team, o));
            Some(if a.yes { Weighed::Yes } else { Weighed::No })
        }
        _ => None,
    }
}

/// An offer to your boss that isn't your extension: he doesn't weigh it as
/// a deal (a plan for a recruit, "get me the money and I'll sign him").
/// What you need from him is an ask (`folk::asked`), what you'll deliver a
/// goal (`folk::commit`).
pub fn not_a_deal(w: &World, him: Them, o: &Offer) -> bool {
    your_boss_she(w, him).is_some() && !extension(o)
}

/// Whether an offer to your boss is your extension: your own deal, or a
/// contract with years or money in it. An NIL deal, a scholarship or a
/// trade is never your extension, whatever money is in it (Cain
/// 2026-11-16: "we're gonna offer him more than Clemson" read as a deal).
pub fn extension(o: &Offer) -> bool {
    match o.kind {
        DealKind::Yours => true,
        DealKind::Contract | DealKind::Restructure | DealKind::Staff => {
            o.gives.iter().any(|g| matches!(g, Give::Years(_) | Give::Money(_)))
        }
        _ => false,
    }
}

/// What you put up for your extension: a bar in your words, else a season
/// term on the offer (lane N2's terms: wins, a title, a rank).
pub fn case_of(team: Team, o: &Offer) -> Case {
    let case = Case::read(&o.words, team.level());
    if case.bar.is_some() {
        return case;
    }
    Case { bar: o.terms.iter().find(|u| u.season).map(|u| GoalKind::Measure(*u)) }
}

/// Your boss's answer to your extension in words, for the voice.
pub fn why(w: &World, him: Them, o: &Offer) -> Vec<String> {
    let Some(team) = Team::seat(w) else { return Vec::new() };
    let Some(she) = your_boss_she(w, him) else { return Vec::new() };
    if !extension(o) {
        return Vec::new();
    }
    let a = asked::money(w, team, AskKind::Extension, &case_of(team, o));
    asked::voice(&Said { level: team.level(), what: AskKind::Extension.words(), she }, &a)
}

/// Whether he is your boss across the table, the athletic director or the
/// owner (not your head coach when you're his assistant), and his pronoun.
pub fn your_boss_she(w: &World, him: Them) -> Option<bool> {
    let Them::Cast(r, _) = him else { return None };
    (super::super::makeup::kind(w, him) == super::super::wants::Kind::Boss).then(|| cast::get(w, r).she)
}

/// A no to your extension in the talk, kept on record with his reasons
/// (`asked::refuse`). Returns the outcome line.
pub fn refused(w: &mut World, team: Team, o: &Offer, she: bool) -> String {
    let case = case_of(team, o);
    let mut a = asked::money(w, team, AskKind::Extension, &case);
    if a.yes {
        a = Answer { yes: false, why: vec![asked::Why::Close] };
    }
    asked::refuse(w, team, AskKind::Extension, &case, &a, she)
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
            let base = t.salary;
            let done = hiring::offer(w, c, role, t);
            let hired = done.is_ok();
            let mut out = vec![done.unwrap_or_else(|e| e)];
            if hired {
                out.extend(add_goals(w, c, o, base));
            }
            Some(out)
        }
        Them::Cast(..) if super::super::makeup::kind(w, him) == super::super::wants::Kind::Boss => {
            if !extension(o) {
                return None;
            }
            // He said yes in the talk; your confirm makes it real.
            let she = your_boss_she(w, him).unwrap_or(false);
            Some(asked::take(w, team, AskKind::Extension, &case_of(team, o), she))
        }
        _ => None,
    }
}

fn own_agree(w: &mut World, c: usize, o: &Offer) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(s) = w.folk.staff.deals.iter().find(|d| d.coach == c).map(|d| d.salary) {
        out.extend(add_goals(w, c, o, s));
    }
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
