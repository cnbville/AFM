//! What you ask of the people in your building (section 8.3), each run by
//! the built action a chip runs:
//!
//! - the team doctor: a second opinion (`life::second_opinion`) and the
//!   pressure to clear a hurt man (`life::rush_back`), weighed by
//!   `doctor::clear_score`;
//! - a coordinator: a plan setting on his side of the ball, the staff
//!   meeting's own step (`dialogue::shift_plan`);
//! - your boss: money (the staff pool, recruiting, the collective, a
//!   facility, practice-squad money), a hire's money, time and his backing
//!   with the big booster, answered by the meeting's own odds and roll
//!   (`boss::ask_yes`, `ask_now`, `ask_time`);
//! - a booster: money for the collective;
//! - the compliance office: what the rules allow.
//!
//! Anything else falls to lane N3's requests.

use super::super::Them;
use super::super::ear::Turn;
use super::super::terms::{Give, Request};
use super::super::wants::{Kind as WhoKind, Ran, WantKind, Willing};
use super::doctor;
use crate::calendar::Level;
use crate::world::World;
use crate::world::coach::{Employer, Role};
use crate::world::folk::boss::{self, AskKind};
use crate::world::folk::cast::{self, Role as CastRole};
use crate::world::folk::{self as folk, Team};

/// A coach whose job is with your team (not you).
pub fn on_your_staff(w: &World, c: usize) -> bool {
    let Some(job) = w.coaches.get(c).and_then(|x| x.job) else { return false };
    let you = w.career.as_ref().map(|x| x.coach);
    you != Some(c)
        && match (Team::seat(w), job.employer) {
            (Some(Team::College(p)), Employer::College(q)) => p == q,
            (Some(Team::Nfl(t)), Employer::Nfl(q)) => t == q,
            (Some(Team::High(h)), Employer::HighSchool(q)) => h == q,
            (Some(Team::Other(f)), Employer::Other(q)) => f == q,
            _ => false,
        }
}

/// The boss ask a money request is, from what you said it's for: the staff
/// pool unless you named recruiting, the collective, a facility or
/// practice-squad money.
pub fn money_kind(words: &str) -> AskKind {
    let w = words.to_lowercase();
    let has = |xs: &[&str]| xs.iter().any(|x| w.contains(x));
    if has(&["facilit", "building", "weight room", "stadium", "locker room", "indoor"]) {
        AskKind::Facility
    } else if has(&["nil", "collective", "revenue"]) {
        AskKind::NilMoney
    } else if has(&["recruit", "travel", "visit"]) {
        AskKind::RecruitingMoney
    } else if has(&["practice squad", "practice-squad", "roster"]) {
        AskKind::RosterSpots
    } else {
        AskKind::StaffMoney
    }
}

/// A plan setting in words as the staff meeting's move (`dialogue::
/// move_plan`): 0 more pressure, 4 less; 1 less man (more zone), 5 more
/// man; 2 run more, 3 pass more.
pub fn plan_move(words: &str) -> Option<u8> {
    let w = words.to_lowercase();
    let has = |xs: &[&str]| xs.iter().any(|x| w.contains(x));
    let less = has(&["less", "fewer", "back off", "stop", "cut", "not as"]);
    if has(&["blitz", "pressure", "send more", "bring more"]) {
        Some(if less { 4 } else { 0 })
    } else if has(&["zone"]) {
        Some(if less { 5 } else { 1 })
    } else if has(&["man coverage", "man-to-man", "press man", "more man", "play man"]) {
        Some(if less { 1 } else { 5 })
    } else if has(&["throw", "pass"]) {
        Some(if less { 2 } else { 3 })
    } else if has(&["run"]) {
        Some(if less { 3 } else { 2 })
    } else {
        None
    }
}

/// A plan move's side: true offense (the run share), false defense.
fn move_offense(m: u8) -> bool {
    matches!(m, 2 | 3)
}

/// Your boss at your seat, when he's the man across.
fn your_boss(w: &World, him: Them) -> Option<Team> {
    let team = Team::seat(w)?;
    (super::super::makeup::kind(w, him) == WhoKind::Boss && matches!(him, Them::Cast(..))).then_some(team)
}

/// Whether you already asked him for time this season (the goals meeting
/// keeps it on the goals).
fn asked_time(w: &World, team: Team) -> bool {
    let season = match team.level() {
        Level::Nfl | Level::College | Level::HighSchool => w.year,
        _ => w.year,
    };
    w.folk.goals.iter().any(|g| g.season >= season && g.deal == -1)
}

/// How willing he is, for the people and asks this lane plays; None
/// leaves it to lane N3.
pub fn willing(w: &World, him: Them, ask: &Request) -> Option<Willing> {
    let team = Team::seat(w)?;
    let yes = |b: bool| if b { Willing::Yes } else { Willing::No };
    if doctor::is_doctor(w, him) {
        return match ask {
            Request::SecondOpinion(id) => {
                let i = folk::person(w, *id)?;
                Some(yes(doctor::chart(w, i).hurt()))
            }
            Request::Clear(id) => {
                let i = folk::person(w, *id)?;
                if !doctor::chart(w, i).hurt() {
                    return Some(Willing::No);
                }
                Some(yes(doctor::clear_score(w, him, team) >= doctor::CLEARS))
            }
            _ => None,
        };
    }
    if let Them::Coach(c) = him
        && on_your_staff(w, c)
    {
        return match ask {
            Request::Plan(words) => {
                let m = plan_move(words)?;
                let role = w.coaches[c].job.map(|j| j.role);
                let ok = match role {
                    Some(Role::OffensiveCoordinator) => move_offense(m),
                    Some(Role::DefensiveCoordinator) => !move_offense(m),
                    _ => false,
                };
                Some(yes(ok))
            }
            _ => None,
        };
    }
    if let Some(team) = your_boss(w, him) {
        let level = team.level();
        let money = |k: AskKind| Some(yes(k.at(level) && !boss::asked_lately(w, k) && boss::ask_yes(w, team, k)));
        return match ask {
            Request::Money(words) => money(money_kind(words)),
            Request::Hire(_) => money(AskKind::StaffMoney),
            Request::Time => {
                let b = boss::boss(w, team);
                Some(yes(!asked_time(w, team) && b.patience >= 11.0))
            }
            Request::Backing => {
                let Team::College(_) = team else { return Some(Willing::No) };
                Some(yes(boss::trust(w, team) >= BACKING_TRUST && big_booster(w, team).is_some()))
            }
            _ => None,
        };
    }
    if super::super::makeup::kind(w, him) == WhoKind::Booster
        && let Them::Cast(r, _) = him
        && r.team == team
    {
        return match ask {
            Request::Fund => {
                let Team::College(_) = team else { return Some(Willing::No) };
                let t = super::trust(w, him);
                Some(yes(t >= FUND_TRUST && !boss::asked_lately(w, AskKind::NilMoney)))
            }
            _ => None,
        };
    }
    if matches!(him, Them::Drawn(WhoKind::Compliance, _)) {
        return match ask {
            Request::RulesQuestion => Some(Willing::Yes),
            _ => None,
        };
    }
    None
}

/// Your boss's trust he needs before he'll vouch for you to the big
/// booster. Proposed, not confirmed.
pub const BACKING_TRUST: f32 = 0.5;
/// What his backing does: the big booster's trust in you, and what it
/// costs your boss's. Proposed, not confirmed.
pub const BACKING_LIFT: f32 = 0.04;
pub const BACKING_COST: f32 = -0.01;
/// A booster's trust in you before he'll put money in the collective, and
/// how much more it holds (the boss's own NIL yes is 8%). Proposed, not
/// confirmed.
pub const FUND_TRUST: f32 = 0.5;
pub const FUND_RAISE: f64 = 0.05;

fn big_booster(w: &World, team: Team) -> Option<cast::Cast> {
    cast::all(w, team, CastRole::Booster).into_iter().find(|c| c.booster.as_ref().is_some_and(|b| b.big))
}

/// Runs what he agreed to, for the people and asks this lane plays; None
/// leaves it to lane N3.
pub fn run(w: &mut World, him: Them, ask: &Request) -> Option<Ran> {
    let team = Team::seat(w)?;
    let mut ran = Ran { honest: true, ..Ran::default() };
    if doctor::is_doctor(w, him) {
        match ask {
            Request::SecondOpinion(id) => {
                let line = crate::world::folk::life::second_opinion(w, *id);
                ran.done.push(line);
                ran.report.push("I'll set it up with the specialist.".into());
            }
            Request::Clear(id) => {
                let (cleared, line) = crate::world::folk::life::rush_back(w, team, *id);
                ran.done.push(line);
                ran.report.push(if cleared {
                    "He's cleared. It's on you if it goes again.".into()
                } else {
                    "No. Not with this injury.".into()
                });
            }
            _ => return None,
        }
        return Some(ran);
    }
    if let Them::Coach(c) = him
        && on_your_staff(w, c)
    {
        let Request::Plan(words) = ask else { return None };
        let m = plan_move(words)?;
        crate::world::folk::dialogue::shift_plan(w, m, 1.0);
        ran.done.push(format!(
            "{} changes the plan: {} this week.",
            w.coaches[c].name,
            crate::world::folk::dialogue::MOVES[m as usize]
        ));
        ran.report.push("You got it. It's in the plan.".into());
        return Some(ran);
    }
    if let Some(team) = your_boss(w, him) {
        let k = match ask {
            Request::Money(words) => money_kind(words),
            Request::Hire(_) => AskKind::StaffMoney,
            Request::Time => {
                let gave = boss::ask_time(w, team);
                ran.done.push(if gave {
                    "He lowered the bar for this year.".into()
                } else {
                    "He kept the bar where it was, and noted that you asked.".into()
                });
                ran.report.push(if gave {
                    "All right. A year to build. But I want to see it moving.".into()
                } else {
                    "I didn't hire you to ask for time.".into()
                });
                return Some(ran);
            }
            Request::Backing => {
                let b = boss::boss(w, team);
                if let Some(big) = big_booster(w, team) {
                    w.folk.mine.nudge(big.key(), BACKING_LIFT);
                    w.folk.mine.nudge(b.key(), BACKING_COST);
                    ran.done.push(format!("{} talked to {} for you.", b.name, big.name));
                    ran.report.push("I'll make the call. He'll hear it from me.".into());
                }
                return Some(ran);
            }
            _ => return None,
        };
        let (yes, words) = boss::ask_now(w, team, k);
        ran.done.push(format!("You asked for {}: {}.", k.words(), if yes { "granted" } else { "no" }));
        ran.report.push(words.into());
        return Some(ran);
    }
    if super::super::makeup::kind(w, him) == WhoKind::Booster {
        let Request::Fund = ask else { return None };
        let (Team::College(p), Them::Cast(r, _)) = (team, him) else { return None };
        w.programs[p].collective.base *= 1.0 + FUND_RAISE;
        let name = cast::get(w, r).name;
        w.folk.asks.push(boss::Ask {
            kind: AskKind::NilMoney,
            date: w.today,
            granted: true,
            words: format!("{name} put more into the collective."),
        });
        ran.done.push(format!("{name} puts more money into the collective."));
        ran.report.push("I'll make some calls. You'll see it.".into());
        return Some(ran);
    }
    if matches!(him, Them::Drawn(WhoKind::Compliance, _)) {
        let Request::RulesQuestion = ask else { return None };
        ran.report.extend(super::rules::words(w));
        ran.done.push("Compliance walked you through what's allowed today.".into());
        return Some(ran);
    }
    None
}

/// After a turn: a no from your boss to a money ask is kept as the
/// meeting keeps it (`ask_now`: the ask on record, a little trust), a no
/// to time costs what the goals meeting's does (`ask_time`), and a doctor
/// pushed to clear a man holds it against you (`doctor::REFUSED_*`).
pub fn after(w: &mut World, him: Them, t: &Turn) -> Vec<String> {
    let mut out = Vec::new();
    let Some(team) = Team::seat(w) else { return out };
    for (r, wl, _) in &t.asked {
        if *wl != Willing::No {
            continue;
        }
        if doctor::is_doctor(w, him) {
            if let Request::Clear(id) = r
                && folk::person(w, *id).is_some_and(|i| doctor::chart(w, i).hurt())
            {
                let by = if crate::world::folk::life::would_rush(w, team) {
                    doctor::REFUSED_PUSHED
                } else {
                    doctor::REFUSED_JUDGMENT
                };
                super::nudge(w, him, by);
                out.push("He won't clear him. The timeline holds.".into());
            }
            continue;
        }
        if your_boss(w, him).is_some() {
            let k = match r {
                Request::Money(words) => Some(money_kind(words)),
                Request::Hire(_) => Some(AskKind::StaffMoney),
                _ => None,
            };
            if let Some(k) = k
                && k.at(team.level())
                && !boss::asked_lately(w, k)
            {
                let (_, words) = boss::ask_now(w, team, k);
                out.push(format!("You asked for {}: no. \"{words}\"", k.words()));
            }
            if matches!(r, Request::Time) && !asked_time(w, team) {
                boss::ask_time(w, team);
                out.push("He kept the bar where it was, and noted that you asked.".into());
            }
        }
    }
    out
}

/// What one of your assistants asks for as a thing the game can do, when
/// he raises a want himself (lane N4's `HisAsk.gives`): his money is a
/// raise of `HIS_RAISE`, his next step a title, his say in the scheme the
/// play-calling (a coordinator who doesn't call it). Empty for anyone else.
pub fn his_gives(w: &World, him: Them, k: WantKind) -> Vec<Give> {
    let Them::Coach(c) = him else { return Vec::new() };
    if !on_your_staff(w, c) {
        return Vec::new();
    }
    let salary = w.folk.staff.deals.iter().find(|d| d.coach == c).map(|d| d.salary);
    let coordinator =
        w.coaches[c].job.is_some_and(|j| matches!(j.role, Role::OffensiveCoordinator | Role::DefensiveCoordinator));
    match k {
        WantKind::Money => salary.map(|s| vec![Give::Money(s * (1.0 + HIS_RAISE))]).unwrap_or_default(),
        WantKind::Career if !w.folk.hiring.titles.iter().any(|x| x.0 == c) => vec![Give::Title],
        WantKind::Scheme if coordinator && !calls_now(w, c) => vec![Give::Calling],
        _ => Vec::new(),
    }
}

/// The raise an assistant asks for himself. Proposed, not confirmed.
pub const HIS_RAISE: f64 = 0.1;

/// Whether your coordinator already calls his side's plays.
fn calls_now(w: &World, c: usize) -> bool {
    let Some(you) = w.career.as_ref().map(|k| k.coach) else { return false };
    let Some(y) = w.coaches[you].yours.as_ref() else { return false };
    match w.coaches[c].job.map(|j| j.role) {
        Some(Role::OffensiveCoordinator) => !y.calls_offense,
        Some(Role::DefensiveCoordinator) => !y.calls_defense,
        _ => false,
    }
}
