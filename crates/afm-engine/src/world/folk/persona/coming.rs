//! People coming to you (lane N1, sections 1 and 6, question 12): who
//! wants a word today and why, read from the world. A player wants to talk
//! snaps, a pro asks out, a family crisis, a parent about playing time, an
//! agent with another club's deadline, a recruit with a rival's offer. The
//! urgent ones stop Continue (the game's side, lane N4 and the screens);
//! the rest wait in the inbox. Ignored, he remembers (`Treated::Ignored`)
//! and it costs a little: a player's mood, anyone's trust.
//!
//! Reading changes nothing. Every size here is Claude's (proposed, not
//! confirmed).

use super::Them;
use super::makeup;
use super::memory::Treated;
use crate::world::World;
use crate::world::folk::Team;
use crate::world::folk::claims::{self, Kind as Pk};

/// Why he comes to you.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Come {
    /// He lost his job or is buried, and he wants to talk about it.
    Snaps,
    /// A pro wants out: a trade.
    TradeRequest,
    /// A pro wants a new deal.
    Pay,
    /// The portal is on his mind and he asked to see you.
    Future,
    /// A death or a sick parent at home.
    FamilyCrisis,
    /// His parent, about playing time.
    ParentTime,
    /// An agent with another club's offer on its last days.
    AgentDeadline,
    /// A recruit with an offer from a school he likes more than yours.
    RivalOffer,
}

/// One person who wants a word.
#[derive(Clone, Debug, PartialEq)]
pub struct Approach {
    pub who: Them,
    pub why: Come,
    /// It stops Continue.
    pub urgent: bool,
    /// What it's about, in words for your screen.
    pub words: String,
}

/// The days left on an agent's deadline that make it urgent.
pub const DEADLINE_DAYS: i32 = 3;

/// The mood under which a player who lost his job comes to you.
pub const SNAPS_MOOD: f32 = -0.15;

/// Who wants a word today.
pub fn comes(w: &World) -> Vec<Approach> {
    let mut out = Vec::new();
    let Some(team) = Team::seat(w) else { return out };
    for sheet in claims::roster(w, team) {
        let Some(id) = sheet.about else { continue };
        let Some(i) = crate::world::folk::person(w, id) else { continue };
        let p = &w.people[i];
        let name = p.name.clone();
        let m = makeup::makeup(w, Them::Player(id));
        for pt in &sheet.points {
            match pt.kind {
                Pk::LostJob | Pk::Buried if p.mood <= SNAPS_MOOD && m.ambition >= 11.0 => {
                    out.push(Approach {
                        who: Them::Player(id),
                        why: Come::Snaps,
                        urgent: false,
                        words: format!("{name} wants to talk about his snaps."),
                    });
                }
                Pk::WantsOut if pt.is("trade", "yes") => out.push(Approach {
                    who: Them::Player(id),
                    why: Come::TradeRequest,
                    urgent: true,
                    words: format!("{name} wants to be traded."),
                }),
                Pk::Pay if pt.is("gap", "big") && m.greed >= 12.0 => out.push(Approach {
                    who: Them::Player(id),
                    why: Come::Pay,
                    urgent: false,
                    words: format!("{name} wants to talk about his contract."),
                }),
                Pk::Portal if pt.is("asked", "yes") => out.push(Approach {
                    who: Them::Player(id),
                    why: Come::Future,
                    urgent: true,
                    words: format!("{name} asked to see you about his future."),
                }),
                Pk::Death | Pk::SickParent if !pt.known && p.trust >= 0.4 => out.push(Approach {
                    who: Them::Player(id),
                    why: Come::FamilyCrisis,
                    urgent: true,
                    words: format!("{name} needs a word. Something at home."),
                }),
                _ => {}
            }
        }
        // His parent calls about playing time when the family weighs it
        // most and he isn't playing.
        let f = &p.family;
        let wants_time = f.decider_wants.playing_time >= 0.3;
        let not_playing =
            matches!(p.last_role, Some(crate::player::PlayingTime::Backup | crate::player::PlayingTime::Redshirt));
        let loud = matches!(f.parent_style, crate::world::background::ParentStyle::LoudAtGames);
        if wants_time && not_playing && (loud || p.mood <= SNAPS_MOOD) && !matches!(p.stage, crate::world::Stage::Pro) {
            out.push(Approach {
                who: Them::Kin(id, 0),
                why: Come::ParentTime,
                urgent: false,
                words: format!("{name}'s family wants to talk about his playing time."),
            });
        }
    }
    // Agents with another club's offer on a man you want.
    for pd in &w.market.pending {
        let Some(p) = w.people.get(pd.person) else { continue };
        let Some(a) = p.pro.as_ref().and_then(|c| c.agent) else { continue };
        let ours = matches!(team, Team::Nfl(t) if pd.offer.team != t && p.pro.as_ref().and_then(|c| c.teams.last().copied()) == Some(t));
        let left = w.today.days_to(pd.until);
        if ours && (0..=DEADLINE_DAYS).contains(&left) {
            out.push(Approach {
                who: Them::Agent(a),
                why: Come::AgentDeadline,
                urgent: true,
                words: format!(
                    "{}'s agent has an offer from {} and needs your answer by {}.",
                    p.name,
                    w.nfl_team_name(pd.offer.team),
                    claims::words::date_words(pd.until)
                ),
            });
        }
    }
    // Recruits with a rival's offer they like more than yours.
    if let Team::College(prog) = team {
        for p in &w.people {
            if !matches!(p.stage, crate::world::Stage::HighSchool)
                || !p.cal.offers.contains(&prog)
                || p.cal.commit.is_some()
            {
                continue;
            }
            let mine = crate::world::folk::case::pull(p, prog);
            let Some((rival, best)) = p
                .cal
                .offers
                .iter()
                .filter(|&&j| j != prog)
                .map(|&j| (j, crate::world::folk::case::pull(p, j)))
                .max_by(|a, b| a.1.total_cmp(&b.1))
            else {
                continue;
            };
            // He calls when a rival's case is clearly ahead and he still
            // wants you in it.
            if best > mine + 0.03 && mine > 0.0 {
                out.push(Approach {
                    who: Them::Player(p.player.id),
                    why: Come::RivalOffer,
                    urgent: false,
                    words: format!("{} says {} is pushing hard. Can you match it?", p.name, w.program_name(rival)),
                });
            }
        }
    }
    // One approach a man: the urgent one first.
    out.sort_by_key(|a| !a.urgent);
    let mut seen = Vec::new();
    out.retain(|a| {
        let k = makeup::key(a.who);
        if seen.contains(&k) {
            false
        } else {
            seen.push(k);
            true
        }
    });
    out
}

/// You let an approach go: he remembers it, and it costs a little trust
/// (and a player's mood).
pub fn ignored(w: &mut World, a: &Approach) {
    super::stores::treat(w, a.who, Treated::Ignored);
    let m = makeup::makeup(w, a.who);
    let cost = (0.02 + 0.02 * super::makeup::Makeup::z(m.ego).max(0.0) + if a.urgent { 0.02 } else { 0.0 }) as f32;
    super::stores::nudge_trust(w, a.who, -cost);
    if let Them::Player(id) = a.who {
        crate::world::folk::talks::mood_event(w, id, crate::world::folk::talks::Reason::Talk, -cost);
    }
}
