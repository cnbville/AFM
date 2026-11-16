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
//!   with the big booster, weighed as you said it (`folk::asked`): what you
//!   put up for it, his trust, his budget and his nature, never the
//!   meeting's pick and its canned answer;
//! - a booster: money for the collective, weighed the same way;
//! - the compliance office: what the rules allow.
//!
//! Anything else falls to lane N3's requests.

use super::super::Them;
use super::super::ear::Turn;
use super::super::terms::{Give, Request};
use super::super::wants::{Kind as WhoKind, Ran, WantKind, Willing};
use super::doctor;
use crate::world::World;
use crate::world::coach::{Employer, Role};
use crate::world::folk::asked::{self, Answer, Case, Said};
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
/// practice-squad money. The kind named first wins, since what the money is
/// for comes before what you put up for it ("more NIL, and we'll fill the
/// stadium" is the collective), and a class you put up ("a top-3
/// recruiting class") is a bar, not recruiting money.
pub fn money_kind(words: &str) -> AskKind {
    const KINDS: [(AskKind, &[&str]); 5] = [
        (AskKind::Facility, &["facilit", "building", "weight room", "stadium", "locker room", "indoor"]),
        (AskKind::NilMoney, &["nil", "collective", "revenue"]),
        (AskKind::RecruitingMoney, &["recruit", "travel", "visit"]),
        (AskKind::RosterSpots, &["practice squad", "practice-squad", "roster"]),
        (AskKind::StaffMoney, &["staff", "assistant", "salar"]),
    ];
    let w = words.to_lowercase().replace("recruiting class", "class").replace("recruit class", "class");
    // Where a word starts: "nil" in "vanilla" isn't the collective.
    let at = |x: &str| {
        w.match_indices(x)
            .map(|(i, _)| i)
            .find(|&i| i == 0 || !w[..i].chars().next_back().is_some_and(|c| c.is_ascii_alphanumeric()))
    };
    KINDS
        .iter()
        .filter_map(|(k, xs)| xs.iter().filter_map(|x| at(x)).min().map(|i| (i, *k)))
        .min_by_key(|x| x.0)
        .map_or(AskKind::StaffMoney, |x| x.1)
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

/// What a money request asks for and what you put up for it, from the
/// ear's words for it (`money_kind`, `asked::Case`).
pub fn money_of(team: Team, r: &Request) -> Option<(AskKind, Case)> {
    match r {
        Request::Money(words) => Some((money_kind(words), Case::read(words, team.level()))),
        Request::Hire(_) => Some((AskKind::StaffMoney, Case::default())),
        _ => None,
    }
}

/// Your boss's or a booster's answer to an ask in your words, weighed as
/// said (`folk::asked`), with what you asked for in words and the pronoun
/// of the one across the table; None for anyone or anything else.
pub fn weighed(w: &World, him: Them, ask: &Request) -> Option<(Answer, String, bool)> {
    let team = Team::seat(w)?;
    if let Some(team) = your_boss(w, him) {
        let she = across_she(w, him);
        let (a, what) = match ask {
            Request::Money(_) | Request::Hire(_) => {
                let (k, case) = money_of(team, ask)?;
                (asked::money(w, team, k, &case), k.words().to_string())
            }
            Request::Time => (asked::time(w, team), "more time".to_string()),
            Request::Backing => {
                let big = big_booster(w, team);
                let what = big.as_ref().map_or("vouching for the coach with the boosters".to_string(), |x| {
                    format!("vouching for the coach with {}", x.name)
                });
                (asked::backing(w, team, big.is_some()), what)
            }
            _ => return None,
        };
        return Some((a, what, she));
    }
    if super::super::makeup::kind(w, him) == WhoKind::Booster
        && let Them::Cast(r, _) = him
        && r.team == team
        && matches!(ask, Request::Fund)
    {
        let c = cast::get(w, r);
        let a = asked::fund(w, team, &c, super::trust(w, him));
        return Some((a, "more money for the collective".to_string(), c.she));
    }
    None
}

/// The pronoun of the one across the table (your athletic director or the
/// president, a booster).
fn across_she(w: &World, him: Them) -> bool {
    match him {
        Them::Cast(r, _) => cast::get(w, r).she,
        _ => false,
    }
}

/// His answer in words for the voice, when it was weighed as said.
pub fn why(w: &World, him: Them, ask: &Request) -> Vec<String> {
    let Some(team) = Team::seat(w) else { return Vec::new() };
    weighed(w, him, ask)
        .map(|(a, what, she)| asked::voice(&Said { level: team.level(), what: &what, she }, &a))
        .unwrap_or_default()
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
    if let Some((a, ..)) = weighed(w, him, ask) {
        return Some(yes(a.yes));
    }
    if your_boss(w, him).is_some() {
        return None;
    }
    if super::super::makeup::kind(w, him) == WhoKind::Booster
        && let Them::Cast(r, _) = him
        && r.team == team
    {
        return None;
    }
    if matches!(him, Them::Drawn(WhoKind::Compliance, _)) {
        return match ask {
            Request::RulesQuestion => Some(Willing::Yes),
            _ => None,
        };
    }
    None
}

/// What his backing does: the big booster's trust in you, and what it
/// costs your boss's. Proposed, not confirmed.
pub const BACKING_LIFT: f32 = 0.04;
pub const BACKING_COST: f32 = -0.01;
/// How much more a booster's yes puts in the collective (the boss's own
/// NIL yes is 8%). Proposed, not confirmed.
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
        // He said yes in the talk (`willing`); your confirm makes it real.
        match ask {
            Request::Money(_) | Request::Hire(_) => {
                let (k, case) = money_of(team, ask)?;
                ran.done.extend(asked::take(w, team, k, &case, across_she(w, him)));
                return Some(ran);
            }
            Request::Time => {
                let yes = Answer { yes: true, why: Vec::new() };
                ran.done.push(asked::settle_time(w, team, &yes, across_she(w, him)));
                return Some(ran);
            }
            Request::Backing => {
                let b = boss::boss(w, team);
                if let Some(big) = big_booster(w, team) {
                    w.folk.mine.nudge(big.key(), BACKING_LIFT);
                    w.folk.mine.nudge(b.key(), BACKING_COST);
                    ran.done.push(format!("{} talked to {} for you.", b.name, big.name));
                }
                return Some(ran);
            }
            _ => return None,
        }
    }
    if super::super::makeup::kind(w, him) == WhoKind::Booster {
        let Request::Fund = ask else { return None };
        let (Team::College(p), Them::Cast(r, _)) = (team, him) else { return None };
        let c = cast::get(w, r);
        let name = c.name.clone();
        // Two yeses (two confirms, or asking again) put money in once.
        if asked::gave_lately(w, &c).is_some() {
            ran.done.push(format!("{name} already put money in this year, and it stands as it was."));
            return Some(ran);
        }
        w.programs[p].collective.base *= 1.0 + FUND_RAISE;
        w.folk.asks.push(boss::Ask {
            kind: AskKind::NilMoney,
            date: w.today,
            granted: true,
            words: format!("{name}{}", asked::GIFT),
        });
        ran.done.push(format!("{name} puts more money into the collective."));
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

/// After a turn: a no from your boss to a money ask is kept on record with
/// his reasons and costs a little trust (`asked::refuse`), a no to time is
/// noted on this season's goals at a cost to his trust (`asked::
/// settle_time`), a no to his backing or a booster's money is told with its
/// reasons, and a doctor pushed to clear a man holds it against you
/// (`doctor::REFUSED_*`).
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
        // The answer the talk gave, with its reasons (a no stays a no).
        let Some((mut a, what, she)) = weighed(w, him, r) else { continue };
        if a.yes {
            a = Answer { yes: false, why: vec![asked::Why::Close] };
        }
        let said = Said { level: team.level(), what: &what, she };
        match r {
            Request::Money(_) | Request::Hire(_) => {
                let Some((k, case)) = money_of(team, r) else { continue };
                if a.why.contains(&asked::Why::NotHere) {
                    out.push(format!("You asked for {}: there's no such money here.", k.words()));
                } else {
                    out.push(asked::refuse(w, team, k, &case, &a, she));
                }
            }
            Request::Time => out.push(asked::settle_time(w, team, &a, she)),
            _ => {
                let Them::Cast(c, _) = him else { continue };
                let name = cast::get(w, c).name;
                let what = if matches!(r, Request::Backing) {
                    "to vouch for you with the big booster"
                } else {
                    "to put more into the collective"
                };
                out.push(format!("You asked {name} {what}: no. {}", asked::because(&said, &a)));
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
