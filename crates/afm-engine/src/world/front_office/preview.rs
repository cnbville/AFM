//! Standing duties, read ahead (hand-off projections, item 8): who runs a
//! duty when your staff has it, how good your read says they are, what you
//! keep, what you give up, and their next two weeks from the plan as it
//! stands: the dated days the duty acts on, the recommendations and visits
//! already in the staff's own book, the transfers about to decide.
//!
//! Read only: nothing here changes the world. What the staff would do with
//! a target the day it comes (whom to chase, whom to offer) is decided
//! inside the day's work and isn't run ahead; the preview says so rather
//! than guess.

use super::{Duty, Owner};
use crate::calendar::{Date, Level};
use crate::world::folk::handoff::Rated;
use crate::world::folk::{self, Team};
use crate::world::{World, staff_read};
use serde::Serialize;

/// One thing the staff would get to in the next two weeks.
#[derive(Clone, Debug, Serialize)]
pub struct DutyLine {
    /// A day, when it has one.
    pub date: Option<i32>,
    pub label: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DutyPreview {
    pub key: String,
    pub name: String,
    /// Your staff holds it now.
    pub staff: bool,
    pub who: String,
    /// What the staff's own words for it are.
    pub does: String,
    pub reads: Vec<Rated>,
    pub keep: Vec<String>,
    pub give: Vec<String>,
    pub plan: Vec<DutyLine>,
    /// What isn't run ahead.
    pub unknown: String,
}

const AHEAD: i32 = 14;

fn last(name: &str) -> &str {
    name.split_whitespace().last().unwrap_or(name)
}

/// The anchor days a duty acts on, with what happens then.
fn dated(w: &World, d: Duty) -> Vec<(Date, String)> {
    let y = &w.dates;
    let mut v: Vec<(Date, String)> = Vec::new();
    let mut add = |date: Date, text: &str| v.push((date, text.to_string()));
    match d {
        Duty::Contracts => {
            add(y.fifth_year_option.add(-8), "Fifth-year options: the club decides, a week before the deadline.");
            add(y.roster_bonus_window.0, "Roster bonuses come due.");
            add(y.league_year.date, "The new league year: contracts that ran out are gone.");
        }
        Duty::Tags => {
            add(
                y.tag_window_open.add(-1),
                "The tag window opens tomorrow: the staff weighs a tag for the star who would walk.",
            );
            add(y.tag_deadline.date.add(-2), "Two days before the tag deadline: tags and tenders go in.");
            add(y.rfa_deadline, "The restricted tender deadline.");
        }
        Duty::FreeAgency => {
            if let Some(t) = y.tampering_open {
                add(t.date, "The negotiating window opens: the staff talks to the free agents who fill your holes.");
            }
            add(y.league_year.date, "Free agency opens: bids go in.");
        }
        Duty::Trades => add(y.trade_deadline.date, "The trade deadline."),
        Duty::Cap => {
            add(y.league_year.date, "The league year turns: the cap has to fit.");
            add(y.top30_deadline, "The top-51 count starts.");
        }
        Duty::Cutdown => {
            add(y.cutdown.date, "Cutdown to 53.");
            for p in y.preseason {
                add(p, "A preseason game: the staff watches who plays.");
            }
        }
        Duty::Waivers => add(y.waiver_claims.date, "Waiver claims on the cut players."),
        Duty::PracticeSquad => add(y.practice_squads, "The practice squad is set."),
        Duty::Board => {
            add(y.official_visits_from, "Official visits open.");
            add(y.signing_day, "Signing day.");
        }
        Duty::Visits => {
            add(y.junior_days.0, "Junior days begin.");
            add(y.official_visits_from, "Official visits open.");
        }
        Duty::Portal => {
            add(y.portal_open, "The transfer window opens.");
            add(y.portal_close, "The transfer window closes.");
        }
        Duty::Nil => add(y.money_year, "The money year turns: your NIL numbers stand until you change them."),
        Duty::Squads | Duty::InjuredReserve => {}
    }
    v.retain(|(date, _)| *date >= w.today && w.today.days_to(*date) <= AHEAD);
    v.sort_by_key(|x| x.0);
    v
}

fn person_name(w: &World, id: u32) -> String {
    folk::person(w, id).map_or_else(|| "A player".to_string(), |i| w.people[i].name.clone())
}

/// What the college duties' men are read at: each position coach's
/// Recruiting by your read, and your own.
fn recruiting_reads(w: &World) -> Vec<Rated> {
    let Some(team) = Team::seat(w) else { return Vec::new() };
    let mut v = Vec::new();
    for (c, role) in folk::facts::assistants(w, team) {
        if !matches!(role, crate::world::coach::Role::PositionCoach(_)) {
            continue;
        }
        if let Some((lo, hi)) = staff_read::coach_read(w, c, "recruiting") {
            v.push(Rated {
                name: format!("{} {} · Recruiting", role.name(), last(&w.coaches[c].name)),
                lo,
                hi,
                yours: false,
            });
        }
    }
    if let Some(a) = w.your_ability() {
        let r = a.recruiting.round() as u8;
        v.push(Rated { name: "You · Recruiting".to_string(), lo: r, hi: r, yours: true });
    }
    v
}

pub fn duty(w: &World, d: Duty) -> DutyPreview {
    let staff = w.front_office.owner(d) == Owner::Staff;
    let college = d.level() == Level::College;
    let mut plan: Vec<DutyLine> = dated(w, d)
        .into_iter()
        .map(|(date, text)| DutyLine { date: Some(date.0), label: "On the calendar".to_string(), text })
        .collect();
    let mut keep = vec!["Your own moves stand beside theirs.".to_string()];
    let mut give = Vec::new();
    let who = match d {
        Duty::Board | Duty::Visits | Duty::Portal => "Your position coaches, each on his room".to_string(),
        Duty::Nil => "You: NIL is never handed over".to_string(),
        _ => "Your front-office staff".to_string(),
    };
    match d {
        Duty::Board => {
            keep = vec![
                "Your own offers stand: the board shows your numbers and only yours.".to_string(),
                "Every NIL number: their offers carry none.".to_string(),
            ];
            give.push("Choosing whom to offer, and ranking the board yourself.".to_string());
            let chain = &w.front_office.chain;
            let ready: Vec<_> = chain
                .recs
                .iter()
                .filter(|r| r.answer == super::chain::Answer::Waiting && r.ready <= w.today.add(AHEAD))
                .collect();
            if !ready.is_empty() {
                let names: Vec<String> = ready.iter().take(3).map(|r| person_name(w, r.id)).collect();
                plan.push(DutyLine {
                    date: None,
                    label: "Waiting for your signature".to_string(),
                    text: format!(
                        "{} from the position coaches ({}{}). Nothing goes out until you sign.",
                        ready.len(),
                        names.join(", "),
                        if ready.len() > 3 { ", and more" } else { "" }
                    ),
                });
            }
            let live =
                w.front_office.college.board.iter().filter(|t| t.offered.is_some() && t.pulled.is_none()).count();
            plan.push(DutyLine {
                date: None,
                label: "Your board".to_string(),
                text: format!("{live} live offer{} stand as they are.", if live == 1 { "" } else { "s" }),
            });
        }
        Duty::Visits => {
            keep = vec!["Visits you set stand.".to_string(), "Every NIL number: their visits carry none.".to_string()];
            give.push("Choosing who comes in, and when.".to_string());
            for p in w.front_office.college.visits.iter().filter(|p| {
                p.state == super::college::VisitState::Set && p.date >= w.today && w.today.days_to(p.date) <= AHEAD
            }) {
                plan.push(DutyLine {
                    date: Some(p.date.0),
                    label: "A visit already set".to_string(),
                    text: format!("{}: {:?}.", person_name(w, p.id), p.kind).replace("Official", "official visit"),
                });
            }
        }
        Duty::Portal => {
            keep = vec![
                "Your own offers, calls and stay talks stand.".to_string(),
                "Every NIL number: their offers carry none.".to_string(),
            ];
            give.push("Choosing whom to chase, and calling the ones that matter yourself.".to_string());
            let deciding = w
                .portal_pending
                .iter()
                .filter(|&&i| w.people[i].cal.portal.is_some_and(|e| e.decide_by <= w.today.add(AHEAD)))
                .count();
            if deciding > 0 {
                plan.push(DutyLine {
                    date: None,
                    label: "Deciding soon".to_string(),
                    text: format!(
                        "{deciding} transfer{} decide in the next two weeks.",
                        if deciding == 1 { "" } else { "s" }
                    ),
                });
            }
            let mine = w.front_office.college.portal.iter().filter(|t| t.offered.is_some() && !t.settled).count();
            plan.push(DutyLine {
                date: None,
                label: "Your offers".to_string(),
                text: format!("{mine} of yours are out and stand as they are."),
            });
            if let Some((roster, limit)) = super::program(w).and_then(|j| crate::world::recruiting::portal_roster(w, j))
            {
                plan.push(DutyLine {
                    date: None,
                    label: "Room".to_string(),
                    text: format!("Your roster counts {roster} of {limit} for the transfer season."),
                });
            }
        }
        Duty::Nil => {
            keep = vec!["Every number: NIL is always yours.".to_string()];
        }
        Duty::Squads => give.push("Choosing who plays varsity, JV and freshman ball.".to_string()),
        _ => give.push(format!("The decisions on {}.", d.name().to_lowercase())),
    }
    if plan.is_empty() {
        plan.push(DutyLine {
            date: None,
            label: "Next two weeks".to_string(),
            text: "Nothing dated, and nothing in their book yet.".to_string(),
        });
    }
    DutyPreview {
        key: d.key().to_string(),
        name: d.name().to_string(),
        staff,
        who,
        does: d.staff_does().to_string(),
        reads: if college && d != Duty::Nil { recruiting_reads(w) } else { Vec::new() },
        keep,
        give,
        plan,
        unknown: if d == Duty::Nil {
            String::new()
        } else {
            "Whom they would chase or offer is decided the day it comes, from what the day holds: this shows the plan as it stands, and changes as the world moves.".to_string()
        },
    }
}
