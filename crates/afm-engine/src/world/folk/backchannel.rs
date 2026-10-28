//! What the people you know do for you besides tips (coaching C15-C19):
//! a smaller school's star offered before the portal opens, for the price
//! of its era (C15); a player you can't keep placed with a friend a level
//! down (C16); a high school coach hired while you recruit his five-star
//! (C17); a transfer a 7v7 coach or trainer steers to your high school
//! (C18); an agent's first word on a client before free agency (C19).
//!
//! The AI's schools sell and buy the same head starts at each winter
//! window, and the AI's staffs hire a five-star's coach by the same rule.
//! Sizes are Claude's picks (proposed, not confirmed).

use std::collections::HashMap;

use super::network::{Favour, Other};
use super::scene::{self, Kind, Pick, Scene, Who};
use super::{Team, coach_link, player_link};
use crate::calendar::{Date, Event, Kind as CalKind, Level, Loudness, Weekday};
use crate::world::coach::{Employer, Role};
use crate::world::season::key_uniform;
use crate::world::{Division, Stage, World};

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Backchannel {
    /// Stars sold before the winter window (C15), yours and the AI's.
    pub sold: Vec<Sold>,
    /// Players you placed with a friend (C16): they enter the next window.
    pub placed: Vec<Placed>,
    /// Head coaches suspended for a ghost transfer: (program, season, the
    /// interim who coaches those games).
    pub suspended: Vec<(usize, i32, Option<usize>)>,
    /// Five-stars whose high school coach a college has hired or offered a
    /// job (C17), so each kid's coach is asked once.
    pub asked: Vec<u32>,
    /// Transfers steered to your high school (C18).
    pub steered: Vec<Steered>,
    /// An agent's first word (C19): (player, club, league year).
    pub first: Vec<(u32, usize, i32)>,
    /// Tampering that leaked (C19), waiting on the league: (club, year).
    pub cases: Vec<(usize, i32)>,
    /// The last season a smaller school called you (C15).
    pub called: i32,
    /// The last year an agent or a trainer called (C18, C19).
    pub agent_call: i32,
    pub trainer_call: i32,
    /// C20: a small school's coach you hired on his star's promise: (the
    /// kid, the coach, the kid's class, how close the two are, settled).
    pub followed: Vec<(u32, usize, i32, f32, bool)>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Sold {
    pub player: u32,
    pub from: usize,
    pub to: usize,
    /// The season whose winter window he enters.
    pub label: i32,
    pub bond: f32,
    /// Straight to the buyer without the portal (a ghost transfer).
    pub ghost: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Placed {
    pub player: u32,
    pub to: usize,
    pub bond: f32,
    pub made: Date,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Steered {
    pub player: u32,
    pub by: Other,
    pub date: Date,
    pub checked: bool,
}

/// The head start a paid exit buys at decision time (proposed, not
/// confirmed: twice a quiet word's 0.05 appeal points).
const HEAD_START: f32 = 0.10;

fn roll(w: &World, a: u64, b: u64) -> f64 {
    super::roll(w, 0xBAC4 ^ a, b)
}

fn you(w: &World) -> Option<usize> {
    w.career.as_ref().map(|c| c.coach)
}

fn news(w: &mut World, level: Level, text: String, prog: Option<usize>) {
    let mut e = Event::new(w.today, level, CalKind::News, Loudness::Small, text);
    if let Some(p) = prog {
        e = e.program(p);
    }
    w.calendar.post(e);
}

fn money(x: f64) -> String {
    crate::world::front_office::dollars(x)
}

// ---------------------------------------------------------------------------
// What the portal reads.

/// A star sold before this winter's window enters it (C15).
pub(crate) fn sold(w: &World, id: u32, label: i32) -> bool {
    w.folk.backchannel.sold.iter().any(|s| s.player == id && s.label == label)
}

/// A player you placed with a friend enters the next window (C16).
pub(crate) fn placed(w: &World, id: u32) -> bool {
    w.folk.backchannel.placed.iter().any(|s| s.player == id)
}

/// Each entrant's ties to a buyer or a friend's program: (player id,
/// program) to appeal points. A program with one wants him whatever its
/// depth chart says (it vouched for him, or paid).
pub(crate) fn bonds(w: &World) -> HashMap<(u32, usize), f64> {
    let b = &w.folk.backchannel;
    let mut m: HashMap<(u32, usize), f64> = HashMap::new();
    for s in &b.sold {
        m.insert((s.player, s.to), if s.ghost { 1.0 } else { s.bond as f64 });
    }
    for s in &b.placed {
        m.insert((s.player, s.to), s.bond as f64);
    }
    m
}

/// Whether a program's head coach sits out today's games (a ghost
/// transfer's suspension).
pub fn suspended(w: &World, prog: usize) -> bool {
    w.folk.backchannel.suspended.iter().any(|s| s.0 == prog && s.1 == w.dates.season && w.today < half(s.1))
}

/// Half the season: the first six Saturdays (proposed, not confirmed).
fn half(season: i32) -> Date {
    Date::ymd(season, 10, 12)
}

/// An agent's first word on a player for a club this league year (C19).
pub(crate) fn first_word(w: &World, i: usize, team: usize) -> bool {
    let id = w.people[i].player.id;
    w.folk.backchannel.first.iter().any(|f| f.0 == id && f.1 == team && f.2 == w.year)
}

// ---------------------------------------------------------------------------
// C15: what a head start costs in each era.

/// Whether a school pays revenue share under the House settlement in a
/// year (from 2025): every FBS school; at FCS every CAA, Missouri Valley
/// and SWAC school did for 2025-26, the Ivy League didn't, and the Big
/// Sky and the rest choose each year (proposed, not confirmed: an even
/// chance). Below Division I nobody does.
fn opted_in(w: &World, p: usize, year: i32) -> bool {
    let prog = &w.programs[p];
    match prog.division {
        Division::FBS => true,
        Division::FCS => match prog.conference.as_str() {
            "CAA" | "Missouri Valley" | "MVFC" | "SWAC" => true,
            "Ivy" | "Ivy League" => false,
            _ => roll(w, 0x0C15, (p as u64) << 16 ^ year as u64) < 0.5,
        },
        _ => false,
    }
}

/// The price of a head start on one of a school's players, by era:
/// 0 a buyout in his revenue-share deal (from 2025, opted-in schools; sized
/// like the real ones, about two years of his deal, $60,000 to $1.5
/// million), 1 a premium on his NIL deal paid by your collective (from the
/// 2021 season: a year of his NIL plus $25,000), 2 a favour (the portal
/// from October 2018), 3 a release to your school (before the portal).
fn price(w: &World, i: usize, from: usize, label: i32) -> (u8, f64) {
    let base = w.people[i].nil_income.max(40_000.0);
    if label >= 2025 && opted_in(w, from, label) {
        (0, (2.0 * base).clamp(60_000.0, 1_500_000.0))
    } else if label >= 2021 {
        (1, (base + 25_000.0).clamp(40_000.0, 600_000.0))
    } else if label >= 2018 {
        (2, 0.0)
    } else {
        (3, 0.0)
    }
}

/// The winter window's first day after a season.
fn winter_open(w: &World, season: i32) -> Option<Date> {
    let rule = w.changes.rules(season).transfer;
    crate::world::clock::portal_windows(season, rule)
        .into_iter()
        .find(|x| x.2 == crate::world::clock::PortalWindow::Winter)
        .map(|x| x.0)
}

/// The AI's head starts (C15): at the winter window, some of the stars
/// moving up were sold beforehand (proposed, not confirmed: one in seven)
/// to a program a level up, the one whose head coach knows the most people
/// near the kid's school; it pays the era's price from its money.
pub(crate) fn ai_head_starts(w: &mut World, movers: &[usize], label: i32) {
    let mine = match Team::seat(w) {
        Some(Team::College(p)) => Some(p),
        _ => None,
    };
    let seed = w.options.seed as u64 ^ 0xC15A;
    for &i in movers {
        let p = &w.people[i];
        let id = p.player.id;
        let Some(from) = p.college else { continue };
        if sold(w, id, label) || key_uniform(seed ^ label as u64, id as u64) >= 1.0 / 7.0 {
            continue;
        }
        let fd = w.programs[from].division as usize;
        let st = w.programs[from].state;
        let best = w
            .programs
            .iter()
            .enumerate()
            .filter(|(j, q)| {
                (q.division as usize) < fd && q.division != Division::JUCO && q.plays(label + 1) && Some(*j) != mine
            })
            .map(|(j, q)| {
                let net = q.head_coach.map_or(8.0, |c| w.coaches[c].ability().network) / 20.0;
                let near = crate::world::recruiting::closeness(crate::world::geo::miles(st, q.state));
                (net + near + 0.3 * key_uniform(seed ^ 0x77, (j as u64) << 32 ^ id as u64), j)
            })
            .max_by(|a, b| a.0.total_cmp(&b.0));
        let Some((_, to)) = best else { continue };
        let (_, cost) = price(w, i, from, label);
        let c = &mut w.programs[to].collective;
        c.budget = (c.budget - cost).max(0.0);
        w.folk.backchannel.sold.push(Sold { player: id, from, to, label, bond: HEAD_START, ghost: false });
    }
}

/// The contact who speaks for a smaller program: its athletic director, or
/// its head coach.
fn caller(w: &World, p: usize) -> Option<usize> {
    w.folk.network.contacts.iter().position(|x| match x.other {
        Some(Other::Ad(q, since)) => q == p && w.programs[p].ad.as_ref().is_some_and(|a| a.since == since),
        None => w.programs[p].head_coach == Some(x.coach),
        _ => false,
    })
}

/// Thursdays in the three weeks before the winter window: a warm contact
/// at a smaller school calls with its star (proposed, not confirmed: a
/// contact at 0.3 or warmer, the friend you've sent players to first, a
/// third of the weeks, once a season).
fn c15_week(w: &mut World, d: Date) {
    let Some(Team::College(j)) = Team::seat(w) else { return };
    let season = w.dates.season;
    if w.folk.backchannel.called >= season {
        return;
    }
    let Some(open) = winter_open(w, season) else { return };
    let left = d.days_to(open);
    if !(3..=21).contains(&left) || roll(w, 0x15, d.0 as u64) > 0.34 {
        return;
    }
    let mine = w.programs[j].division as usize;
    let mut v: Vec<(u8, f32, usize, usize)> = Vec::new();
    for (k, x) in w.folk.network.contacts.iter().enumerate() {
        if x.warmth < 0.3 {
            continue;
        }
        let p = match x.other {
            Some(Other::Ad(p, _)) => p,
            None => match super::facts::team_of_coach(w, x.coach) {
                Some(Team::College(p)) if w.programs[p].head_coach == Some(x.coach) => p,
                _ => continue,
            },
            _ => continue,
        };
        let div = w.programs[p].division;
        if (div as usize) <= mine || div == Division::JUCO || caller(w, p) != Some(k) {
            continue;
        }
        v.push((x.sent, x.warmth, k, p));
    }
    v.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.total_cmp(&a.1)));
    let Some(&(_, _, k, p)) = v.first() else { return };
    // His star: the program's best starter with a season left.
    let star = w.programs[p]
        .roster
        .iter()
        .copied()
        .filter(|&i| {
            let q = &w.people[i];
            q.stage == Stage::College
                && q.college == Some(p)
                && q.cal.portal.is_none()
                && q.cal.declare_plan != Some(true)
                && q.last_role == Some(crate::world::PlayingTime::Starter)
                && !crate::world::college::no_season_left(q, w.programs[p].division, season)
        })
        .max_by(|&a, &b| w.people[a].player.overall().total_cmp(&w.people[b].player.overall()));
    let Some(i) = star else { return };
    w.folk.backchannel.called = season;
    let (kind, cost) = price(w, i, p, season);
    let x = &w.folk.network.contacts[k];
    let name = match x.other {
        Some(Other::Ad(..)) => w.programs[p].ad.as_ref().map_or("The athletic director".into(), |a| a.name.clone()),
        _ => w.coaches[x.coach].name.clone(),
    };
    let mut s = Scene::new(Kind::Backchannel, d, format!("{name} of {} calls before the portal", w.program_name(p)));
    s.a = w.people[i].player.id as u64;
    s.b = 15;
    s.c = p as i64;
    s.f = [cost as f32, kind as f32, 0.0, 0.0];
    s.stops = false;
    s.team = Team::seat(w);
    s.place = "On the phone".into();
    if x.other.is_none() {
        s.who.push(Who::Coach(x.coach));
    }
    s.who.push(Who::Player(w.people[i].player.id));
    s.title = format!("{} calls about {}", name, w.people[i].name);
    scene::add(w, s);
}

fn c15_open(w: &mut World, s: &mut Scene) {
    let id = s.a as u32;
    let p = s.c as usize;
    let Some(i) = super::person(w, id) else { return s.end() };
    let kind = s.f[1] as u8;
    let cost = s.f[0] as f64;
    let school = w.program_name(p).to_string();
    let kid = player_link(w, i);
    let pos = w.people[i].player.position.name();
    s.narrate(format!("A call from {school}, a friend of yours."));
    let ask = match kind {
        0 => format!(
            "{kid}, our {pos}, is leaving us. Everybody knows it. His deal with us carries a buyout of {}. Pay it and you get the first call, and we won't hold him or come after you.",
            money(cost)
        ),
        1 => format!(
            "{kid}, our {pos}, is going in the portal. Put a premium on his NIL deal, {} from your collective, and we'll point him your way first and wish him well.",
            money(cost)
        ),
        2 => format!(
            "{kid}, our {pos}, is going in the portal. You'd get the first call. Just remember us when we need something."
        ),
        _ => format!("{kid}, our {pos}, wants out. We'll give him a release to your school and nobody else's."),
    };
    s.them(&school, &ask, None);
    let deal = if cost > 0.0 { format!("Pay {}", money(cost)) } else { "Take the deal".into() };
    s.pick(151, format!("\"We'll do it.\" ({deal})"), "Deal", "warm", "A head start and a clean exit. He still picks.");
    s.pick(
        152,
        "\"Let me talk to the kid myself first.\"",
        "Go around him",
        "sly",
        "Tampering: it can leak, and from 2025 his school can sue.",
    );
    if season_of(s) >= 2018 {
        s.pick(
            153,
            "\"Skip the portal. Send him straight to us.\"",
            "Ghost transfer",
            "bold",
            "He's yours if it holds; if it's found out, a suspension and a fine from 2026.",
        );
    }
    s.pick(154, "\"Not this time. Thank you for thinking of me.\"", "Pass", "calm", "He'll call someone else.");
}

fn season_of(s: &Scene) -> i32 {
    s.date.year() - if s.date.parts().1 <= 6 { 1 } else { 0 }
}

fn c15_answer(w: &mut World, s: &mut Scene, code: u32) {
    let id = s.a as u32;
    let p = s.c as usize;
    let Some(Team::College(j)) = Team::seat(w) else { return s.end() };
    let Some(i) = super::person(w, id) else { return s.end() };
    let k = caller(w, p);
    let label = season_of(s);
    let kind = s.f[1] as u8;
    let cost = s.f[0] as f64;
    let name = w.people[i].name.clone();
    let school = w.program_name(p).to_string();
    let warm = |w: &mut World, d: f32| {
        if let Some(k) = k {
            let x = &mut w.folk.network.contacts[k];
            x.warmth = (x.warmth + d).clamp(0.0, 1.0);
            x.last = Some(w.today);
        }
    };
    match code {
        151 | 153 => {
            if cost > 0.0 {
                let left = crate::world::front_office::college::budget(w, j).left;
                if left < cost {
                    s.outcome.push(format!("You don't have {} left this year. You passed.", money(cost)));
                    return s.end();
                }
                let c = &mut w.programs[j].collective;
                c.budget = (c.budget - cost).max(0.0);
            }
            if kind == 2
                && let Some(k) = k
            {
                w.folk.network.contacts[k].balance -= 1;
                let coach = w.folk.network.contacts[k].coach;
                w.folk.network.ledger.push(Favour {
                    date: w.today,
                    coach,
                    words: format!("first call on {name}"),
                    asked: true,
                });
            }
            let ghost = code == 153;
            w.folk.backchannel.sold.push(Sold { player: id, from: p, to: j, label, bond: HEAD_START, ghost });
            let _ = crate::world::front_office::college::head_start(w, id);
            warm(w, 0.03);
            s.outcome.push(match kind {
                0 => format!("You paid {school} {} for {name}'s buyout. You'll have the first call.", money(cost)),
                1 => format!("Your collective put {} on {name}'s deal. You'll have the first call.", money(cost)),
                2 => format!("{school} will point {name} your way. You owe them one."),
                _ => format!("{school} gave {name} a release to your school."),
            });
            if ghost {
                ghost_risk(w, s, j, label);
            }
        }
        152 => {
            let leak = roll(w, 0x152, id as u64 ^ (label as u64) << 32) < 0.125;
            {
                let q = &mut w.people[i];
                q.cal.lean = (q.cal.lean * 1.25 + 0.01).min(0.95);
                q.cal.lean_events.push((w.today, "another school reached out quietly", 1.25));
            }
            if leak {
                let mine = w.program_name(j).to_string();
                news(
                    w,
                    Level::College,
                    format!("Reports: {mine} reached out to {name} of {school} before the portal."),
                    Some(p),
                );
                if label >= 2025 {
                    // From 2025 his school can sue (Wisconsin and Miami, 2025).
                    let (_, buyout) = price(w, i, p, label);
                    let fine = buyout.max(100_000.0);
                    let c = &mut w.programs[j].collective;
                    c.budget = (c.budget - fine).max(0.0);
                    news(
                        w,
                        Level::College,
                        format!("{school} sues {mine} over {name}; {mine} settles for {}.", money(fine)),
                        Some(p),
                    );
                    warm(w, -0.3);
                    s.outcome.push(format!("It leaked, and {school} sued. You settled for {}.", money(fine)));
                } else {
                    warm(w, -0.2);
                    s.outcome.push(format!("It leaked. {school} won't forget it."));
                }
            } else {
                warm(w, -0.05);
                s.outcome.push(format!("You got word to {name} quietly. {school} doesn't know. Yet."));
            }
        }
        _ => {
            warm(w, -0.01);
            s.outcome.push(format!("You passed on {name}."));
        }
    }
    s.end();
}

/// A ghost transfer found out (proposed, not confirmed: one in three):
/// for transfers from February 25, 2026, the head coach sits half the next
/// season and the program pays 20% of its football budget; before that, an
/// NCAA case that costs you standing.
fn ghost_risk(w: &mut World, s: &mut Scene, j: usize, label: i32) {
    if roll(w, 0x153, s.a ^ (label as u64) << 32) >= 1.0 / 3.0 {
        return;
    }
    let mine = w.program_name(j).to_string();
    let rule_from = Date::ymd(2026, 2, 25);
    if winter_open(w, label).is_some_and(|d| d >= rule_from) {
        let fine = football_budget(w, j) * 0.2;
        if let Some(d) = w.finance.college.get_mut(j) {
            d.now.l[crate::world::finance::college::OTHER_COSTS] += fine as f32;
        }
        w.folk.backchannel.suspended.push((j, label + 1, None));
        news(
            w,
            Level::College,
            format!(
                "The NCAA finds a ghost transfer at {mine}: the head coach is suspended for half of next season and the program fined {}.",
                money(fine)
            ),
            Some(j),
        );
        s.outcome
            .push(format!("It came out. You're suspended for half of next season, and {mine} pays {}.", money(fine)));
    } else {
        if let Some(c) = you(w) {
            w.coaches[c].reputation = (w.coaches[c].reputation - 1.0).max(1.0);
        }
        news(
            w,
            Level::College,
            format!("The NCAA opens a case at {mine} over a transfer that skipped the portal."),
            Some(j),
        );
        s.outcome.push("It came out. The NCAA has opened a case, and people are talking.".into());
    }
}

/// Football's yearly spending from the books.
fn football_budget(w: &World, j: usize) -> f64 {
    use crate::world::finance::college as fc;
    w.finance.college.get(j).map_or(0.0, |d| {
        [fc::SUPPORT, fc::COACHES, fc::STAFF, fc::RECRUITING, fc::SCHOLARSHIPS, fc::REV_SHARE, fc::TRAVEL]
            .iter()
            .map(|&k| d.budget.l[k] as f64)
            .sum()
    })
}

// ---------------------------------------------------------------------------
// C16: placing a player a level down.

/// You place a player you can't keep with a friend (C16). At a college the
/// friend's program is a level down and the kid still chooses: he enters
/// the next window with your word and the friend's interest (proposed,
/// not confirmed: 0.08 appeal points, plus 0.08 times the friend's warmth).
/// In the NFL, from ten days before cutdown day to cutdown day, it's a
/// late-round pick swap with the friend's club instead of a release.
pub fn place_player(w: &mut World, id: u32, coach: usize) -> Result<String, String> {
    let k = w.folk.network.contacts.iter().position(|x| x.coach_id() == Some(coach)).ok_or("You don't know him.")?;
    let warmth = w.folk.network.contacts[k].warmth;
    if warmth < 0.2 {
        return Err("You don't know him well enough to ask.".into());
    }
    let i = super::person(w, id).ok_or("No such player.")?;
    let name = w.people[i].name.clone();
    let text = match (Team::seat(w), super::facts::team_of_coach(w, coach)) {
        (Some(Team::College(j)), Some(Team::College(q))) => {
            let p = &w.people[i];
            if p.college != Some(j) || p.stage != Stage::College {
                return Err("He isn't on your roster.".into());
            }
            if (w.programs[q].division as usize) <= (w.programs[j].division as usize) {
                return Err("A place a level down: his program must be smaller than yours.".into());
            }
            if p.last_role == Some(crate::world::PlayingTime::Starter) {
                return Err("He starts for you.".into());
            }
            if placed(w, id) {
                return Err("You've placed him already.".into());
            }
            let limit = crate::world::ncaa::roster_limits(&w.programs[j], w.dates.season).0;
            let over = w.programs[j].roster.len() > limit;
            if p.cal.lean < 0.03 && !over {
                return Err(format!("{name} wants to stay and fight for his spot."));
            }
            let pull = match w.coaches[coach].job.map(|j| j.role) {
                Some(Role::HeadCoach) => 1.0,
                Some(Role::OffensiveCoordinator | Role::DefensiveCoordinator) => 0.7,
                _ => 0.5,
            };
            let bond = (0.08 + 0.08 * warmth) * pull;
            w.folk.backchannel.placed.push(Placed { player: id, to: q, bond, made: w.today });
            let to = w.program_name(q).to_string();
            format!(
                "{name} will enter the next window with your blessing. {} at {to} wants him; the choice is his.",
                w.coaches[coach].name
            )
        }
        (Some(Team::Nfl(t)), Some(Team::Nfl(o))) if o != t => {
            let cut = w.dates.cutdown.date;
            if !(w.today >= cut.add(-10) && w.today <= cut) {
                return Err("A swap instead of a release is a cutdown-day deal.".into());
            }
            if w.people[i].pro.as_ref().and_then(|c| c.team) != Some(t) {
                return Err("He isn't on your roster.".into());
            }
            swap(w, t, o, id)?
        }
        _ => return Err("He isn't in a position to take him.".into()),
    };
    let x = &mut w.folk.network.contacts[k];
    x.sent = x.sent.saturating_add(1);
    x.balance = (x.balance + 1).min(5);
    x.last = Some(w.today);
    w.folk.network.ledger.push(Favour { date: w.today, coach, words: format!("you sent him {name}"), asked: false });
    super::note(w, "network", coach_link(w, coach), text.clone(), None);
    Ok(text)
}

/// The cutdown swap: he and your latest pick in next year's draft for the
/// friend's pick a round higher, from round five on, or the same round.
fn swap(w: &mut World, t: usize, o: usize, id: u32) -> Result<String, String> {
    use crate::world::front_office::deals::{self, Proposal, Side};
    let year = w.year + 1;
    let late = |w: &World, team: usize| {
        let mut v: Vec<deals::Pick> =
            deals::picks_of(w, team).into_iter().filter(|p| p.year == year && !p.comp).collect();
        v.sort_by_key(|p| std::cmp::Reverse(p.round));
        v
    };
    let mine = late(w, t);
    let give = *mine.first().ok_or("You have no late pick left to swap.")?;
    let theirs = late(w, o);
    let get = theirs
        .iter()
        .find(|p| p.round + 1 == give.round && p.round >= 5)
        .or_else(|| theirs.iter().find(|p| p.round == give.round))
        .copied()
        .ok_or("His club has no late pick to swap.")?;
    let p = Proposal {
        other: o,
        give: Side { players: vec![id], picks: vec![give] },
        get: Side { players: Vec::new(), picks: vec![get] },
        conditional: None,
    };
    deals::execute(w, t, &p);
    Ok(format!(
        "Traded to the {} with your {} for their {}, instead of a release.",
        w.nfl[o].name,
        deals::pick_words(w, &give),
        deals::pick_words(w, &get)
    ))
}

// ---------------------------------------------------------------------------
// C17: a five-star's high school coach.

/// The five-star you're recruiting whose high school coach this is, when
/// hiring him is allowed (C17): only for a five-star (Cain: "only for 5*
/// like the cream of the crop"), only a coach you're warm with (0.2 or
/// more), and only to an on-field job (from 2017 the NCAA's IAWP rule keeps
/// him out of off-field and strength jobs for two years either side of the
/// kid's enrollment; every job on your coaching staff is on-field). Err
/// when he coaches a recruit on your board who isn't a five-star.
pub fn hs_hire(w: &World, c: usize) -> Result<Option<u32>, String> {
    let Some(Employer::HighSchool(h)) = w.coaches[c].job.map(|j| j.employer) else { return Ok(None) };
    let Some(Team::College(_)) = Team::seat(w) else { return Ok(None) };
    let board: Vec<u32> = w.front_office.college.board.iter().filter(|t| t.pulled.is_none()).map(|t| t.id).collect();
    let mut kids: Vec<usize> = board
        .iter()
        .filter_map(|&id| super::person(w, id))
        .filter(|&i| w.people[i].hs == h && matches!(w.people[i].stage, Stage::HighSchool | Stage::PrepSchool))
        .collect();
    if kids.is_empty() {
        return Ok(None);
    }
    kids.sort_by_key(|&i| std::cmp::Reverse(w.people[i].stars));
    let star = kids[0];
    if w.people[star].stars < 5 {
        return Err("Hiring a recruit's high school coach is only done for a five-star.".into());
    }
    if super::network::warmth(w, c) < 0.2 {
        return Err("You'd have to know him better first.".into());
    }
    Ok(Some(w.people[star].player.id))
}

/// The AI's staffs hire a five-star's coach by the same rule (C17), on
/// December 1 (Gus Malzahn left Springdale for Arkansas in December 2005,
/// before Mitch Mustain signed; proposed, not confirmed: one five-star's
/// coach in twenty-five a year, by the program leading for the kid, to
/// coach his own position group).
fn ai_hs_hires(w: &mut World) {
    let season = w.dates.season;
    let mine = match Team::seat(w) {
        Some(Team::College(p)) => Some(p),
        _ => None,
    };
    let kids: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            let p = &w.people[i];
            p.stage == Stage::HighSchool && p.stars == 5 && p.class_of == season + 1 && p.cal.signed.is_none()
        })
        .collect();
    for i in kids {
        let id = w.people[i].player.id;
        if w.folk.backchannel.asked.contains(&id) || roll(w, 0xC17, (id as u64) << 8 ^ season as u64) >= 0.04 {
            continue;
        }
        let h = w.people[i].hs;
        let Some(c) = w.high_schools[h].coach else { continue };
        if Some(c) == you(w) {
            continue;
        }
        let lead = w.people[i]
            .cal
            .offers
            .iter()
            .copied()
            .filter(|&j| w.programs[j].division == Division::FBS && Some(j) != mine)
            .map(|j| (crate::world::recruiting::appeal(w, i, j, None), j))
            .max_by(|a, b| a.0.total_cmp(&b.0));
        let Some((_, j)) = lead else { continue };
        let role = Role::PositionCoach(w.coaches[c].came_up);
        w.folk.backchannel.asked.push(id);
        if let Some(old) = crate::world::staff::holder(w, Employer::College(j), role) {
            crate::world::staff::leave(w, old, "reassigned");
        }
        crate::world::staff::leave(w, c, "hired by a college");
        crate::world::staff::hire(w, c, Employer::College(j), role);
        let prestige = crate::world::staff::employer_prestige(w, Employer::HighSchool(h));
        crate::world::staff::fill_now(w, Employer::HighSchool(h), Role::HeadCoach, prestige);
        let text = format!(
            "{} hires {}, the high school coach of five-star {}, to coach its {}.",
            w.program_name(j),
            w.coaches[c].name,
            w.people[i].name,
            role.name()
        );
        news(w, Level::Recruiting, text, Some(j));
    }
}

/// At a high school (C17): the college leading for your five-star offers
/// you an on-field job (proposed, not confirmed: on November 15, an even
/// chance for each five-star senior, to coach your own group).
fn hs_offer(w: &mut World) {
    let Some(Team::High(h)) = Team::seat(w) else { return };
    let Some(me) = you(w) else { return };
    let season = w.dates.season;
    let kids: Vec<usize> = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| p.hs == h && p.stage == Stage::HighSchool && p.stars == 5 && p.class_of == season + 1)
        .map(|(i, _)| i)
        .collect();
    for i in kids {
        let id = w.people[i].player.id;
        if w.folk.backchannel.asked.contains(&id) {
            continue;
        }
        w.folk.backchannel.asked.push(id);
        if roll(w, 0xC17B, id as u64) >= 0.5 {
            continue;
        }
        let lead = w.people[i]
            .cal
            .offers
            .iter()
            .copied()
            .filter(|&j| matches!(w.programs[j].division, Division::FBS | Division::FCS))
            .map(|j| (crate::world::recruiting::appeal(w, i, j, None), j))
            .max_by(|a, b| a.0.total_cmp(&b.0));
        let Some((_, j)) = lead else { continue };
        let role = Role::PositionCoach(w.coaches[me].came_up);
        let e = Employer::College(j);
        let prestige = crate::world::staff::employer_prestige(w, e);
        crate::world::front_office::market::offer(w, e, role, prestige);
        let text = format!(
            "{} wants you on its staff while it recruits {}. The job's on the field, so the rules allow it.",
            w.program_name(j),
            player_link(w, i)
        );
        super::note(w, "network", w.program_name(j).to_string(), text, None);
    }
}

// ---------------------------------------------------------------------------
// C20: a small school's coach calls you cold, promising his star.

/// How close a kid is to his high school coach (hidden): his years under
/// him and a part of his own.
fn closeness(w: &World, i: usize, h: usize) -> f32 {
    let p = &w.people[i];
    let years = p
        .hs_lines
        .iter()
        .filter(|l| l.school as usize == h)
        .map(|l| l.year)
        .collect::<std::collections::HashSet<_>>()
        .len();
    (0.5 * roll(w, 0xC20C, p.player.id as u64) + 0.5 * (years as f64 / 4.0).min(1.0)) as f32
}

/// Tuesdays from November to January: the coach of a five-star at a small
/// school (fewer than a thousand students, or below the state's top two
/// classes) calls the college his star likes, cold, no tie needed: a job on
/// your staff and the kid comes with him. College seats only, and only for
/// a five-star (C17's line), so it's rare. Proposed, not confirmed: an 8%
/// chance a week while you're in the kid's top three, once a kid.
fn c20_week(w: &mut World, d: Date) {
    let Some(Team::College(j)) = Team::seat(w) else { return };
    let m = d.parts().1;
    if !(m >= 11 || m == 1) || w.folk.scenes.iter().any(|s| s.kind == Kind::ReachOut && s.b == 3 && !s.done) {
        return;
    }
    let season = w.dates.season;
    let class = if m == 1 { season } else { season + 1 };
    let me = you(w);
    let kid = (0..w.people.len()).find(|&i| {
        let p = &w.people[i];
        if p.stage != Stage::HighSchool
            || p.stars < 5
            || p.class_of != class
            || p.cal.signed.is_some()
            || w.folk.backchannel.asked.contains(&p.player.id)
        {
            return false;
        }
        let h = &w.high_schools[p.hs];
        if !(h.enrollment < 1000 || h.classification >= 2) || h.coach.is_none() || h.coach == me {
            return false;
        }
        // You're among the kid's top three.
        let mut liked: Vec<(f64, usize)> =
            p.cal.offers.iter().map(|&k| (crate::world::recruiting::appeal(w, i, k, None), k)).collect();
        if !p.cal.offers.contains(&j) {
            liked.push((crate::world::recruiting::appeal(w, i, j, None), j));
        }
        liked.sort_by(|a, b| b.0.total_cmp(&a.0));
        liked.iter().take(3).any(|x| x.1 == j) && roll(w, 0xC20 ^ d.0 as u64, p.player.id as u64) < 0.08
    });
    let Some(i) = kid else { return };
    let Some(c) = w.high_schools[w.people[i].hs].coach else { return };
    let id = w.people[i].player.id;
    w.folk.backchannel.asked.push(id);
    let mut s = Scene::new(Kind::ReachOut, d, format!("{} calls about {}", w.coaches[c].name, w.people[i].name));
    s.a = c as u64;
    s.b = 3;
    s.c = id as i64;
    s.stops = false;
    s.team = Some(Team::College(j));
    s.place = "On the phone".into();
    scene::add(w, s);
}

pub(crate) fn c20_open(w: &mut World, s: &mut Scene) {
    let c = s.a as usize;
    let Some(i) = super::person(w, s.c as u32) else { return s.end() };
    s.who.push(Who::Coach(c));
    let name = w.coaches[c].name.clone();
    let p = &w.people[i];
    let g = w.coaches[c].came_up;
    let school = w.school_name(p.hs).to_string();
    s.narrate(format!("{} calls. You've never met him: he coaches {school}.", coach_link(w, c)));
    s.them(
        &name,
        format!(
            "Coach, I'll be straight with you. I've coached {} since he was a freshman. Put me on your staff, coaching {}, and he comes with me.",
            player_link(w, i),
            g.name().to_lowercase()
        ),
        None,
    );
    let rule = if w.dates.season >= 2017 {
        "Since 2017 the NCAA allows it only as an on-field coaching job, for the two years around the kid's enrollment. This one is on the field."
    } else {
        "No NCAA rule stops it yet: any job on your staff will do."
    };
    s.narrate(rule.to_string());
    let holder = w
        .seat
        .and_then(|x| x.program)
        .and_then(|j| crate::world::staff::holder(w, Employer::College(j), Role::PositionCoach(g)))
        .map(|h| w.coaches[h].name.clone());
    let cost = holder.map_or("The spot is open.".to_string(), |h| format!("{h} loses his job to make room."));
    s.pick(
        87,
        format!("\"You're hired. Coach our {}.\"", g.name().to_lowercase()),
        "Hire him",
        "warm",
        &format!("{cost} The kid decides for himself."),
    );
    s.pick(86, "\"I don't hire for a recruit.\"", "Turn him down", "calm", "He'll call another school.");
}

pub(crate) fn c20_answer(w: &mut World, s: &mut Scene, code: u32) {
    let c = s.a as usize;
    let Some(i) = super::person(w, s.c as u32) else { return s.end() };
    let Some(j) = w.seat.and_then(|x| x.program) else { return s.end() };
    let id = w.people[i].player.id;
    let kid = player_link(w, i);
    if code != 87 {
        s.outcome.push(format!("You turned {} down. {kid} hears of it.", w.coaches[c].name));
        return s.end();
    }
    let h = w.people[i].hs;
    let near = closeness(w, i, h);
    let role = Role::PositionCoach(w.coaches[c].came_up);
    if let Some(old) = crate::world::staff::holder(w, Employer::College(j), role) {
        crate::world::staff::leave(w, old, "let go to make room");
    }
    crate::world::staff::leave(w, c, "hired by a college");
    crate::world::staff::hire(w, c, Employer::College(j), role);
    let prestige = crate::world::staff::employer_prestige(w, Employer::HighSchool(h));
    crate::world::staff::fill_now(w, Employer::HighSchool(h), Role::HeadCoach, prestige);
    // The coach can promise; the kid decides, by how close the two are
    // (proposed, not confirmed: 25% plus 60% of his closeness).
    let follows = roll(w, 0xC20F, id as u64) < 0.25 + 0.6 * near as f64;
    let class = w.people[i].class_of;
    w.folk.backchannel.followed.push((id, c, class, near, false));
    let text = format!(
        "{} hires {}, the high school coach of five-star {}, to coach its {}.",
        w.program_name(j),
        w.coaches[c].name,
        w.people[i].name,
        role.name()
    );
    news(w, Level::Recruiting, text, Some(j));
    if follows && w.people[i].cal.commit.is_none_or(|x| x.0 != j) {
        if !w.people[i].cal.offers.contains(&j) {
            w.people[i].cal.offers.push(j);
        }
        w.people[i].cal.commit = None;
        let d = w.today.add(2);
        w.announcements.push((d, i, id, j, false));
        s.outcome.push(format!(
            "{} is on your staff, and {kid} will follow him: he commits to you in two days.",
            w.coaches[c].name
        ));
    } else {
        s.outcome.push(format!(
            "{} is on your staff. {kid} hasn't followed him: he's taking his own visits, his mind his own.",
            w.coaches[c].name
        ));
    }
    s.end();
}

/// December 1 after his first college season: a kid who followed his coach
/// can still leave, the more when the coach is gone or they were never
/// close (Mitch Mustain followed Gus Malzahn to Arkansas in 2006 and
/// transferred after one season). Proposed, not confirmed: half the time
/// when his coach has left your staff, else 15% plus 30% of how far apart
/// they are; leaving lifts his lean toward the portal by 0.5.
fn c20_stay(w: &mut World, d: Date) {
    let season = w.dates.season;
    for k in 0..w.folk.backchannel.followed.len() {
        let (id, c, class, near, done) = w.folk.backchannel.followed[k];
        if done || class != season {
            continue;
        }
        w.folk.backchannel.followed[k].4 = true;
        let Some(i) = super::person(w, id) else { continue };
        let Some(j) = w.people[i].college else { continue };
        let stayed_on = w.coaches[c].job.is_some_and(|x| x.employer == Employer::College(j));
        let chance = if stayed_on { 0.15 + 0.3 * (1.0 - near as f64) } else { 0.5 };
        if roll(w, 0xC205, id as u64) < chance {
            let p = &mut w.people[i];
            p.cal.lean = (p.cal.lean + 0.5).min(1.0);
            p.cal.lean_events.push((d, "came for his high school coach, and it hasn't held", 0.5));
        }
    }
}

// ---------------------------------------------------------------------------
// C18: a transfer steered to your high school.

/// Spring and summer Thursdays: a 7v7 coach or trainer you're warm with
/// (0.3 or more) offers to steer a kid from another school in your state
/// (proposed, not confirmed: an eighth of the weeks from March to June,
/// once a year).
fn c18_week(w: &mut World, d: Date) {
    let Some(Team::High(h)) = Team::seat(w) else { return };
    let m = d.parts().1;
    if !(3..=6).contains(&m) || w.folk.backchannel.trainer_call >= d.year() || roll(w, 0x18, d.0 as u64) > 0.125 {
        return;
    }
    let state = w.high_schools[h].state;
    let men: Vec<(f32, u32)> = w
        .folk
        .network
        .contacts
        .iter()
        .filter(|x| x.warmth >= 0.3)
        .filter_map(|x| match x.other {
            Some(Other::Former(f)) => w
                .formers
                .get(f)
                .filter(|f| {
                    matches!(f.life, crate::world::former::Life::Camp | crate::world::former::Life::Trainer)
                        && f.area as usize == state
                })
                .map(|_| (x.warmth, f)),
            _ => None,
        })
        .collect();
    let Some(&(_, f)) = men.iter().max_by(|a, b| a.0.total_cmp(&b.0)) else { return };
    let eye = w.formers.get(f).map_or(10.0, |x| x.eye as f64);
    let sd = 1.5 + 0.45 * (20.0 - eye);
    let key = w.folk.base as u64 ^ 0xC18 ^ (d.0 as u64) << 20;
    let kid = w
        .people
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            p.stage == Stage::HighSchool
                && p.home_state == state
                && p.hs != h
                && (15..=16).contains(&p.player.age)
                && p.cal.commit.is_none()
                && !p.player.position.kicks()
        })
        .map(|(i, p)| (p.player.overall() + sd * crate::world::season::key_normal(key, p.player.id as u64), i))
        .max_by(|a, b| a.0.total_cmp(&b.0));
    let Some((_, i)) = kid else { return };
    w.folk.backchannel.trainer_call = d.year();
    let man = w.formers.get(f).map_or_else(String::new, |x| x.name.clone());
    let mut s = Scene::new(Kind::Backchannel, d, format!("{man} has a kid for you"));
    s.a = w.people[i].player.id as u64;
    s.b = 18;
    s.c = f as i64;
    s.stops = false;
    s.team = Team::seat(w);
    s.place = "On the phone".into();
    s.who.push(Who::Player(w.people[i].player.id));
    scene::add(w, s);
}

fn c18_open(w: &mut World, s: &mut Scene) {
    let Some(i) = super::person(w, s.a as u32) else { return s.end() };
    let man = w.formers.get(s.c as u32).map_or_else(|| "A trainer".to_string(), |x| x.name.clone());
    let kid = player_link(w, i);
    let school = w.school_name(w.people[i].hs).to_string();
    s.narrate(format!("{man}, who runs a 7v7 team and trains half the skill kids in the area, calls."));
    s.them(
        &man,
        format!(
            "{kid} at {school} isn't happy there. I can get him to you. He starts, and he gets the ball. That's the deal."
        ),
        None,
    );
    s.pick(181, "\"Bring him over. He'll start.\"", "Promise", "warm", "Your association can rule it undue influence.");
    s.pick(182, "\"I don't take kids that way.\"", "Refuse", "calm", "He'll take his kids elsewhere for a while.");
}

/// Your state association's undue influence rule: until each state's real
/// rule is built, every state uses California's (CIF Bylaw 510), which
/// reaches 7v7 and club teams and treats following such a coach as a
/// violation on its face (proposed, not confirmed: ruled so two times in
/// five; the kid sits a year and your standing drops).
fn c18_answer(w: &mut World, s: &mut Scene, code: u32) {
    let Some(Team::High(h)) = Team::seat(w) else { return s.end() };
    let id = s.a as u32;
    let f = s.c as u32;
    let Some(i) = super::person(w, id) else { return s.end() };
    let k = w.folk.network.contacts.iter().position(|x| x.other == Some(Other::Former(f)));
    if code != 181 {
        if let Some(k) = k {
            let x = &mut w.folk.network.contacts[k];
            x.warmth = (x.warmth - 0.05).max(0.0);
        }
        s.outcome.push("You said no.".into());
        return s.end();
    }
    let from = w.people[i].hs;
    let from_name = w.school_name(from).to_string();
    let to_name = w.school_name(h).to_string();
    let state = w.high_schools[h].state;
    let rule = w.states[state].transfer;
    let ruled = roll(w, 0x510, id as u64) < 0.4;
    {
        let p = &mut w.people[i];
        p.sit_out = if ruled {
            1.0
        } else if rule.free_once && p.transfers == 0 {
            0.0
        } else {
            rule.sit_out
        };
        p.transfers += 1;
        p.hs = h;
        let age = p.player.age;
        p.note(format!("At {age}, steered by his trainer, he transferred from {from_name} to {to_name}."));
    }
    w.rebuild_rosters();
    super::talks::promise(w, super::talks::PromiseTo::Player(id), super::talks::PromiseKind::Start, 0);
    super::talks::promise(w, super::talks::PromiseTo::Player(id), super::talks::PromiseKind::Snaps, 0);
    w.folk.backchannel.steered.push(Steered { player: id, by: Other::Former(f), date: w.today, checked: false });
    if let Some(k) = k {
        let x = &mut w.folk.network.contacts[k];
        x.balance -= 1;
        x.last = Some(w.today);
    }
    let name = w.people[i].name.clone();
    s.outcome.push(format!("{name} transferred to {to_name}. You promised him the job and the ball."));
    if ruled {
        if let Some(c) = you(w) {
            w.coaches[c].reputation = (w.coaches[c].reputation - 0.5).max(1.0);
        }
        news(
            w,
            Level::HighSchool,
            format!(
                "The state association rules {name}'s move to {to_name} undue influence: he sits a year and the school is sanctioned."
            ),
            None,
        );
        s.outcome
            .push("The association ruled it undue influence: he sits a year, and the school is sanctioned.".into());
    }
    s.end();
}

/// A broken promise to a steered kid costs you the man who sent him.
fn steered_check(w: &mut World) {
    let settled: Vec<(usize, Option<Other>)> = w
        .folk
        .backchannel
        .steered
        .iter()
        .enumerate()
        .filter(|(_, s)| !s.checked)
        .filter_map(|(k, s)| {
            let mine: Vec<&super::talks::Promise> = w
                .folk
                .promises
                .iter()
                .filter(|p| p.to == super::talks::PromiseTo::Player(s.player) && p.made >= s.date)
                .collect();
            if mine.is_empty() || mine.iter().any(|p| p.kept.is_none()) {
                return None;
            }
            let kept = mine.iter().all(|p| p.kept == Some(true));
            Some((k, (!kept).then_some(s.by)))
        })
        .collect();
    for (k, broken) in settled {
        w.folk.backchannel.steered[k].checked = true;
        if let Some(by) = broken
            && let Some(x) = w.folk.network.contacts.iter_mut().find(|x| x.other == Some(by))
        {
            x.warmth = (x.warmth - 0.3).max(0.0);
        }
    }
}

// ---------------------------------------------------------------------------
// C19: an agent's first word.

/// The ten days before the market opens (the tampering window from 2013,
/// else the new league year): a warm agent (0.3 or more) calls with first
/// word on a client headed for free agency (proposed, not confirmed: a
/// fifth of the weekdays, once a year).
fn c19_day(w: &mut World, d: Date) {
    let Some(Team::Nfl(t)) = Team::seat(w) else { return };
    let opens = w.dates.tampering_open.map_or(w.dates.league_year.date, |m| m.date);
    let left = d.days_to(opens);
    if !(1..=10).contains(&left) || w.folk.backchannel.agent_call >= w.year || roll(w, 0x19, d.0 as u64) > 0.2 {
        return;
    }
    let agents: Vec<(f32, usize)> = w
        .folk
        .network
        .contacts
        .iter()
        .filter(|x| x.warmth >= 0.3)
        .filter_map(|x| match x.other {
            Some(Other::Agent(a)) => Some((x.warmth, a)),
            _ => None,
        })
        .collect();
    let Some(&(_, a)) = agents.iter().max_by(|x, y| x.0.total_cmp(&y.0)) else { return };
    let client = (0..w.people.len())
        .filter(|&i| {
            let p = &w.people[i];
            p.pro.as_ref().is_some_and(|c| c.agent == Some(a) && c.team != Some(t))
                && crate::world::pro::market::on_market(w, i, true)
        })
        .max_by(|&x, &y| {
            crate::world::pro::market::consensus(w, x).total_cmp(&crate::world::pro::market::consensus(w, y))
        });
    let Some(i) = client else { return };
    w.folk.backchannel.agent_call = w.year;
    let agent = w.market.agents[a].name.clone();
    let mut s = Scene::new(Kind::Backchannel, d, format!("{agent} has first word on a client"));
    s.a = w.people[i].player.id as u64;
    s.b = 19;
    s.c = a as i64;
    s.stops = false;
    s.team = Team::seat(w);
    s.place = "On the phone".into();
    s.who.push(Who::Player(w.people[i].player.id));
    scene::add(w, s);
}

fn c19_open(w: &mut World, s: &mut Scene) {
    let Some(i) = super::person(w, s.a as u32) else { return s.end() };
    let agent = w.market.agents.get(s.c as usize).map_or_else(|| "An agent".to_string(), |a| a.name.clone());
    let kid = player_link(w, i);
    s.narrate(format!("{agent} calls. You go back a long way."));
    s.them(&agent, format!("{kid} is going to test the market. I wanted you to hear it from me first."), None);
    s.pick(
        191,
        "\"Tell me what he's looking for.\"",
        "Talk numbers",
        "warm",
        "Before the window it's tampering: the league can take a pick.",
    );
    s.pick(192, "\"Call me the minute the window opens.\"", "Wait", "calm", "First in line, by the book.");
    s.pick(193, "\"We're set at that spot.\"", "Pass", "calm", "He'll shop him elsewhere.");
}

fn c19_answer(w: &mut World, s: &mut Scene, code: u32) {
    let Some(Team::Nfl(t)) = Team::seat(w) else { return s.end() };
    let id = s.a as u32;
    let a = s.c as usize;
    let Some(i) = super::person(w, id) else { return s.end() };
    let name = w.people[i].name.clone();
    let year = w.year;
    if code == 193 {
        s.outcome.push("You passed.".into());
        return s.end();
    }
    w.folk.backchannel.first.push((id, t, year));
    if let Some(x) = w.folk.network.contacts.iter_mut().find(|x| x.other == Some(Other::Agent(a))) {
        x.last = Some(w.today);
        x.warmth = (x.warmth + 0.02).min(1.0);
    }
    if code == 191 {
        let ideal = crate::world::pro::market::ideal(w, i, w.today);
        // An agent shades the number up, the less honest the more.
        let honesty = w.market.agents.get(a).map_or(0.7, |x| x.honesty);
        let ask = ideal.apy * (1.0 + 0.25 * (1.0 - honesty));
        s.outcome.push(format!(
            "He wants about {} a year for {} {}, {}% of it guaranteed. You're first in line.",
            money(ask),
            ideal.years,
            super::noun(ideal.years, "year"),
            (100.0 * ideal.gtd).round()
        ));
        // Talking terms before the market opens is tampering (P2): it can
        // leak (proposed, not confirmed: one time in ten).
        if roll(w, 0x191, id as u64 ^ (year as u64) << 32) < 0.1 {
            w.folk.backchannel.cases.push((t, year));
            news(
                w,
                Level::Nfl,
                format!("Reports: the {} spoke with {name}'s camp before the window opened.", w.nfl[t].name),
                None,
            );
            s.outcome.push("Word got out. The league is looking into it.".into());
        }
    } else {
        s.outcome.push(format!("He'll call you first about {name} when the window opens."));
    }
    s.end();
}

/// The league's ruling on a leaked case (June 1, as with the Falcons in
/// 2024): a fifth-round pick in the next draft forfeited, the club fined
/// $250,000 and the general manager $50,000.
fn league_rulings(w: &mut World) {
    let year = w.year;
    let cases: Vec<usize> = w.folk.backchannel.cases.iter().filter(|c| c.1 == year).map(|c| c.0).collect();
    w.folk.backchannel.cases.retain(|c| c.1 != year);
    for t in cases {
        crate::world::pro::trades::forfeit_pick(w, t, year + 1, 5);
        if let Some(c) = w.finance.nfl.get_mut(t) {
            c.now.l[crate::world::finance::nfl::BUSINESS] += 250_000.0;
        }
        let text = format!(
            "The league finds the {} tampered: they forfeit a {} fifth-round pick, the club is fined $250,000 and the general manager $50,000.",
            w.nfl[t].name,
            year + 1
        );
        news(w, Level::Nfl, text, None);
    }
}

// ---------------------------------------------------------------------------
// The day and the scenes.

pub(crate) fn day(w: &mut World, d: Date) {
    let (_, m, dd) = d.parts();
    let season = w.dates.season;
    if m == 12 && dd == 1 {
        ai_hs_hires(w);
    }
    // A suspended head coach's games go to his interim.
    if m == 9 && dd == 1 {
        for k in 0..w.folk.backchannel.suspended.len() {
            let (p, s, _) = w.folk.backchannel.suspended[k];
            if s == season && w.programs[p].sideline.is_none() {
                let i = crate::world::staff::interim(w, p);
                w.programs[p].sideline = i;
                w.folk.backchannel.suspended[k].2 = i;
            }
        }
    }
    if d == half(season) {
        for &(p, s, i) in &w.folk.backchannel.suspended {
            if s == season && i.is_some() && w.programs[p].sideline == i {
                w.programs[p].sideline = None;
            }
        }
    }
    if m == 8 && dd == 1 {
        let b = &mut w.folk.backchannel;
        b.sold.retain(|s| s.label >= season - 1);
        b.suspended.retain(|s| s.1 >= season - 1);
        b.first.retain(|f| f.2 >= w.year - 1);
        let today = w.today;
        b.placed.retain(|s| s.made.days_to(today) < 400);
    }
    // A placed player who entered the portal no longer waits on a window.
    if !w.folk.backchannel.placed.is_empty() && d.weekday() == Weekday::Mon {
        let gone: Vec<u32> = w
            .folk
            .backchannel
            .placed
            .iter()
            .filter(|s| super::person(w, s.player).is_none_or(|i| w.people[i].cal.portal.is_some()))
            .map(|s| s.player)
            .collect();
        if !gone.is_empty() {
            w.folk.backchannel.placed.retain(|s| !gone.contains(&s.player));
        }
    }
    if m == 6 && dd == 1 {
        league_rulings(w);
    }
    if w.seat.is_none() {
        return;
    }
    if d.weekday() == Weekday::Thu {
        c15_week(w, d);
        c18_week(w, d);
    }
    if m == 11 && dd == 15 {
        hs_offer(w);
    }
    if d.weekday() == Weekday::Tue {
        c20_week(w, d);
    }
    if m == 12 && dd == 1 {
        c20_stay(w, d);
    }
    if !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
        c19_day(w, d);
    }
    if d.weekday() == Weekday::Sun {
        steered_check(w);
    }
}

pub(crate) fn open(w: &mut World, s: &mut Scene) {
    match s.b {
        15 => c15_open(w, s),
        18 => c18_open(w, s),
        _ => c19_open(w, s),
    }
}

pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32, _picked: &Pick) {
    match s.b {
        15 => c15_answer(w, s, code),
        18 => c18_answer(w, s, code),
        _ => c19_answer(w, s, code),
    }
}

pub(crate) fn hand(_w: &mut World, s: &mut Scene) {
    s.handed = Some("nobody".into());
    s.outcome.push("You let the call go.".into());
}
