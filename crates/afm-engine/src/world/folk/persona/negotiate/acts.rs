//! What you can ask an agent or another club's GM to do (6.6, 6.7, 8.3),
//! as the game runs it: tell you his number, take your offer to his
//! client, hold other clubs off, get his client to report, steer another
//! client your way; a GM's heads-up and "not to our division". Lane N3's
//! requests answer everything else.
//!
//! Holding other clubs off, steering a client, a heads-up and "not to our
//! division" are his word only: nothing in the world binds him yet
//! (proposed, not confirmed). Telling his number, taking an offer and a
//! client reporting are real.

use super::super::Them;
use super::super::deals::AGENT_DAYS;
use super::super::ear::Turn;
use super::super::memory::Ready;
use super::super::terms::{Counter, Offer, Request};
use super::super::wants::{Acting, Ran, Want, WantKind, Willing};
use super::{Facet, contract, gm_club};
use crate::world::World;
use crate::world::folk::cast;
use crate::world::front_office::{self as fo, nfl};
use crate::world::pro::market::Hold;

/// The bar a willing score must clear: proposed, not confirmed.
pub const YES: f64 = 0.5;

/// Lane N3's requests, with agents and GMs on a negotiation call.
pub struct Doing<'a> {
    pub facet: Facet,
    pub base: &'a dyn Acting,
}

/// His client on an agent's call.
fn client(f: Facet) -> Option<usize> {
    match f {
        Facet::Agent { client, .. } => Some(client),
        _ => None,
    }
}

/// The agent's trust in your club, -1 to 1, and his honesty.
fn agent_read(w: &World, a: usize) -> (f64, f64) {
    let g = &w.market.agents[a];
    let rel = fo::club(w).and_then(|t| g.rel.get(t).copied()).unwrap_or(0.0);
    (rel, g.honesty)
}

/// How ready a holdout is to end, as the market's own weekly give-in reads
/// it: longer runs and missed games, and an agent who trusts you.
fn report_score(w: &World, rel: f64) -> f64 {
    let since = w.today.days_to(w.dates.veterans_report).abs() as f64 / 7.0;
    let in_season = w.today >= w.dates.nfl_kickoff;
    0.25 + 0.05 * since + if in_season { 0.15 } else { 0.0 } + 0.2 * rel
}

fn holding(w: &World, i: usize) -> Option<Hold> {
    w.people[i].pro.as_ref().map(|c| c.hold).filter(|h| *h != Hold::None)
}

/// A GM's give: lower ego, easier to deal with.
fn gm_ego(w: &World, him: Them) -> f64 {
    match him {
        Them::Cast(r, _) => cast::get(w, r).ego,
        _ => 10.0,
    }
}

/// Ends a holdout he agreed to end, as the market's own give-in does.
fn reports(w: &mut World, i: usize) -> Option<String> {
    let hold = holding(w, i)?;
    let p = &mut w.people[i];
    p.pro.as_mut()?.hold = Hold::None;
    let age = p.row_age();
    p.note(format!("Ended his {} at {age} without a new deal.", if hold == Hold::Out { "holdout" } else { "hold-in" }));
    Some(format!("{} reports.", p.name))
}

impl Acting for Doing<'_> {
    fn willing(&self, w: &World, him: Them, ask: &Request) -> Willing {
        match (him, ask, client(self.facet)) {
            (Them::Agent(_), Request::GiveNumber, Some(_)) => Willing::Yes,
            // An agent has to take every offer to his client (real).
            (Them::Agent(_), Request::TakeOffer, Some(_)) => Willing::Yes,
            (Them::Agent(a), Request::HoldOff, Some(_)) => {
                let (rel, honesty) = agent_read(w, a);
                if 0.2 + 0.4 * honesty + 0.3 * rel >= YES { Willing::Yes } else { Willing::No }
            }
            (Them::Agent(a), Request::Steer(_), _) => {
                let (rel, honesty) = agent_read(w, a);
                if 0.15 + 0.2 * honesty + 0.5 * rel >= YES { Willing::Yes } else { Willing::No }
            }
            (Them::Agent(a), Request::Report, Some(i)) => {
                if holding(w, i).is_none() {
                    return Willing::No;
                }
                if report_score(w, agent_read(w, a).0) >= YES {
                    return Willing::Yes;
                }
                // Not yet: a new deal ends it.
                match nfl::asking(w, w.people[i].player.id) {
                    Ok(p) => Willing::If(Counter {
                        asks: vec![format!(
                            "He reports when he has a new deal: {}.",
                            nfl::describe_proposal(&p, w.year)
                        )],
                        gives: contract::gives_of(&p),
                        terms: Vec::new(),
                        why: Some(Want { kind: WantKind::Money, weight: 1.0, met: false }),
                    }),
                    Err(_) => Willing::No,
                }
            }
            (Them::Cast(..), Request::HeadsUp(_) | Request::NotOurDivision(_), _) if gm_club(w, him).is_some() => {
                if gm_ego(w, him) <= 11.0 {
                    Willing::Yes
                } else {
                    Willing::No
                }
            }
            _ => self.base.willing(w, him, ask),
        }
    }

    fn run(&self, w: &mut World, him: Them, ask: &Request) -> Ran {
        let mut ran = Ran { honest: true, ..Ran::default() };
        let name = |w: &World, id: Option<u32>| {
            id.and_then(|id| crate::world::folk::person(w, id))
                .map_or_else(|| "him".into(), |i| w.people[i].name.clone())
        };
        match (him, ask, client(self.facet)) {
            (Them::Agent(_), Request::GiveNumber, Some(i)) => match nfl::asking(w, w.people[i].player.id) {
                Ok(p) => ran.done.push(format!("His number: {}.", nfl::describe_proposal(&p, w.year))),
                Err(e) => ran.done.push(e),
            },
            (Them::Agent(_), Request::TakeOffer, Some(i)) => {
                let id = w.people[i].player.id;
                let last = w.front_office.nfl.talk(id).and_then(|t| t.offer);
                match last {
                    Some(p) => {
                        let by = w.today.add(AGENT_DAYS);
                        let offer = Offer { gives: contract::gives_of(&p), ..Offer::default() };
                        let m = &mut w.folk.live.persona;
                        m.ready.retain(|r| !(r.player == id && r.scene == 0));
                        m.ready.push(Ready { scene: 0, by, with: him, player: id, offer });
                        ran.done.push(format!(
                            "He'll take your offer to {} and call you back by {}.",
                            w.people[i].name,
                            by.pretty()
                        ));
                    }
                    None => ran.done.push("There's no offer of yours on the desk for him to take.".into()),
                }
            }
            (Them::Agent(_), Request::HoldOff, Some(i)) => {
                ran.done.push(format!("He says he'll call you before {} signs anywhere else.", w.people[i].name));
            }
            (Them::Agent(_), Request::Steer(id), _) => {
                ran.done.push(format!("He says he'll point {} your way.", name(w, Some(*id))));
            }
            (Them::Agent(_), Request::Report, Some(i)) => {
                ran.done.extend(reports(w, i));
            }
            // Your own man in a holdout agreed to report.
            (Them::Player(id), Request::Report, _) => match crate::world::folk::person(w, id) {
                Some(i) if holding(w, i).is_some() => ran.done.extend(reports(w, i)),
                _ => ran.done.push("He isn't holding out.".into()),
            },
            (Them::Cast(..), Request::HeadsUp(id), _) => {
                ran.done.push(format!("He says he'll call you first if {} comes free.", name(w, *id)));
            }
            (Them::Cast(..), Request::NotOurDivision(id), _) => {
                ran.done.push(format!("He says {} won't end up in your division.", name(w, Some(*id))));
            }
            _ => return self.base.run(w, him, ask),
        }
        ran
    }
}

/// What a request adds to his words: the number he agreed to tell you.
pub fn said(w: &World, f: Facet, t: &mut Turn) {
    let Some(i) = client(f) else { return };
    let gives = t.asked.iter().any(|(r, wl, _)| *r == Request::GiveNumber && *wl == Willing::Yes);
    if gives && let Ok(p) = nfl::asking(w, w.people[i].player.id) {
        t.say.released.push(format!("His number: {}.", nfl::describe_proposal(&p, w.year)));
    }
}
