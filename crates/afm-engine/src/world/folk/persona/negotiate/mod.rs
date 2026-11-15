//! Live talks v3, lane N5b: negotiating (`docs/design/live-talks-persona.md`,
//! sections 6.6, 6.7, 6.11, 8.2, 8.3 and 11). Agents, GMs, NIL, contracts,
//! trades and the undrafted rush, each weighed by the system that already
//! prices it, so a deal you build in words is worth exactly what the same
//! deal built in the boxes is worth:
//!
//! - **An agent** answers from the real market: `front_office::nfl::judge`,
//!   the same score, ask, patience and claim `propose` signs with. His claim
//!   about other interest is true or a bluff by his honesty, as built; a
//!   bluff goes in his lies store, and your read of it is the desk's read.
//!   A counter is made for real on the desk (his patience, the history), as
//!   an offer typed in the boxes is. A yes waits on your confirm, and the
//!   confirm signs through `propose` (`contract`).
//! - **Another club's GM** answers from his own chart:
//!   `front_office::deals::evaluate`, players, picks and a conditional pick
//!   (games, starts, the Pro Bowl or a share of the snaps). His counter is
//!   the picks he'd want added. He knows his club (its mode, its needs, why
//!   he'd move a man) and tells it as far as he wants you to know
//!   (`knows`). A yes waits on your confirm and trades through `propose`
//!   (`trade`).
//! - **A prospect in the undrafted rush** weighs your call the way the rush
//!   does: `pro::scouts::rush` played read-only with your list as it would
//!   be (`rush`). A yes puts him on your priority list at those terms.
//! - **Your own college player's NIL and revenue share** land through the
//!   NIL desk (`front_office::college::set_deal`), on lane N3's weighing.
//! - **Your own pro's contract** goes through his agent unless he
//!   negotiates alone (`market::goals`): his agent answers in lane N3's two
//!   days, by a call back (`day`) you answer on its own call.
//!
//! Every size here is Claude's (proposed, not confirmed). No world result
//! changes: only your seat talks.

pub mod acts;
pub mod contract;
pub mod day;
pub mod knows;
pub mod rush;
pub mod trade;

use super::Them;
use super::deals::Deals;
use super::ear::{Ends, Item, Turn};
use super::room::{Channel, Present, Room, Seat};
use super::talk::Parts;
use super::terms::{Best, DealKind, Decision, Give, Offer, Valuing, Weighed};
use crate::world::World;
use crate::world::folk::cast::{self, CastRef, Role};
use crate::world::folk::scene::{self, Kind, Scene, Who};
use crate::world::folk::{self as folk, Team};
use crate::world::{Stage, front_office as fo};

/// The calls (`Scene.b` on `Kind::Negotiation`).
pub const AGENT: u64 = 1;
pub const GM: u64 = 2;
pub const RUSH: u64 = 3;
/// An agent calling back with his client's answer.
pub const BACK: u64 = 4;
/// No player on a GM call (`Scene.a`).
pub const NOBODY: u64 = u64::MAX;

/// The picks on a call: hang up; on a call back, sign or pass.
pub const END: u32 = 1;
pub const SIGN: u32 = 41;
pub const PASS: u32 = 42;

/// What a talk negotiates, and with whom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Facet {
    /// A pro's agent about his contract: the client (person index) and his
    /// agent.
    Agent {
        client: usize,
        agent: Option<usize>,
    },
    /// Another club's GM: the club, and the man the call is about.
    Gm {
        club: usize,
        about: Option<usize>,
    },
    /// A prospect left in the undrafted rush (person index).
    Rush {
        prospect: usize,
    },
    /// Your own pro in a one-on-one: his contract goes through his agent.
    Pro {
        i: usize,
    },
    /// Your own college player in a one-on-one: his NIL and revenue share.
    Nil {
        i: usize,
    },
    None,
}

fn person(w: &World, id: u64) -> Option<usize> {
    u32::try_from(id).ok().and_then(|id| folk::person(w, id))
}

/// A pro's agent, if he has one.
pub fn agent_of(w: &World, i: usize) -> Option<usize> {
    w.people[i].pro.as_ref().and_then(|c| c.agent).filter(|&a| a < w.market.agents.len())
}

/// Another club's GM as a person you talk to.
pub fn gm(w: &World, club: usize) -> Them {
    let r = CastRef::new(Team::Nfl(club), Role::Gm, 0);
    Them::Cast(r, cast::get(w, r).term)
}

/// The club a GM runs, when `him` is another club's GM.
pub fn gm_club(w: &World, him: Them) -> Option<usize> {
    match him {
        Them::Cast(r, _) if r.role == Role::Gm => match r.team {
            Team::Nfl(o) if fo::club(w) != Some(o) => Some(o),
            _ => None,
        },
        _ => None,
    }
}

/// What a scene negotiates.
pub fn facet(w: &World, s: &Scene) -> Facet {
    match s.kind {
        Kind::Negotiation => match s.b {
            AGENT | BACK => person(w, s.a).map_or(Facet::None, |i| Facet::Agent { client: i, agent: agent_of(w, i) }),
            GM => Facet::Gm { club: s.c.max(0) as usize, about: (s.a != NOBODY).then(|| person(w, s.a)).flatten() },
            RUSH => person(w, s.a).map_or(Facet::None, |prospect| Facet::Rush { prospect }),
            _ => Facet::None,
        },
        Kind::OneOnOne => {
            let Some(i) = person(w, s.a) else { return Facet::None };
            let Some(team) = Team::seat(w) else { return Facet::None };
            if !team.roster(w).contains(&i) {
                return Facet::None;
            }
            match w.people[i].stage {
                Stage::Pro => Facet::Pro { i },
                Stage::College => Facet::Nil { i },
                _ => Facet::None,
            }
        }
        _ => Facet::None,
    }
}

/// Whether a talk plays here: a negotiation call. (Your one-on-ones play
/// already; their money goes through `Market` too.)
pub fn is_call(s: &Scene) -> bool {
    s.kind == Kind::Negotiation
}

/// The room for a call: on the phone with his agent (or him, with no
/// agent), the GM, or a prospect with his agent listening in.
pub fn room(w: &World, s: &Scene) -> Option<Room> {
    if s.kind != Kind::Negotiation {
        return None;
    }
    let across = |who: Them| Present { who, seat: Seat::Across, speaks: false };
    let present = match facet(w, s) {
        Facet::Agent { client, agent } => match agent {
            Some(a) => vec![across(Them::Agent(a))],
            None => vec![across(Them::Player(w.people[client].player.id))],
        },
        Facet::Gm { club, .. } => vec![across(gm(w, club))],
        Facet::Rush { prospect } => {
            let mut v = vec![across(Them::Player(w.people[prospect].player.id))];
            if let Some(a) = agent_of(w, prospect) {
                v.push(Present { who: Them::Agent(a), seat: Seat::Agent, speaks: true });
            }
            v
        }
        _ => return None,
    };
    Some(Room { channel: Channel::Phone, present })
}

/// The man a call is about, by id: the client, the prospect, the player a
/// GM call is about.
pub fn subject(w: &World, f: Facet) -> Option<u32> {
    match f {
        Facet::Agent { client: i, .. } | Facet::Rush { prospect: i } | Facet::Pro { i } | Facet::Nil { i } => {
            Some(w.people[i].player.id)
        }
        Facet::Gm { about, .. } => about.map(|i| w.people[i].player.id),
        Facet::None => None,
    }
}

/// Who else a call's card names, and its items (`card_extras`).
pub type Extras = (Vec<(Them, String)>, Vec<(Item, String)>);

/// Who else the ear's card may name on a call, with why: the client, the
/// men in a trade, your roster at a prospect's spot. Their items: the
/// client as the man the agent works for.
pub fn card_extras(w: &World, s: &Scene) -> Extras {
    let mut people: Vec<(Them, String)> = Vec::new();
    let mut items: Vec<(Item, String)> = Vec::new();
    let me = fo::club(w);
    let mut add = |who: Them, why: String| {
        if !people.iter().any(|x| x.0 == who) {
            people.push((who, why));
        }
    };
    match facet(w, s) {
        Facet::Agent { client, .. } => {
            let id = w.people[client].player.id;
            add(Them::Player(id), "his client".into());
            items.push((Item::Friend(id), format!("his client {}", w.people[client].name)));
        }
        Facet::Gm { club, about } => {
            if let Some(i) = about {
                add(Them::Player(w.people[i].player.id), "the man this call is about".into());
            }
            for (i, why) in trade::named(w, club, me) {
                add(Them::Player(w.people[i].player.id), why);
            }
        }
        Facet::Rush { prospect } => {
            for i in rush::at_his_spot(w, prospect) {
                add(Them::Player(w.people[i].player.id), "on your roster at his spot".into());
            }
        }
        _ => {}
    }
    (people, items)
}

// ---------------------------------------------------------------------------
// Calls you make.

fn call(w: &mut World, title: String, b: u64, a: u64, c: i64, who: Vec<Who>) -> u32 {
    let mut s = Scene::new(Kind::Negotiation, w.today, title);
    s.a = a;
    s.b = b;
    s.c = c;
    s.stops = false;
    s.place = "On the phone".into();
    s.who = who;
    let id = scene::add(w, s);
    scene::open(w, id);
    id
}

/// You call a pro's agent about his contract: your own expiring man, an
/// extension, a free agent, another club's restricted man (the desk's
/// rules: `nfl::negotiable`). Returns the call's scene.
pub fn call_agent(w: &mut World, id: u32) -> Result<u32, String> {
    fo::club(w).ok_or("You don't run an NFL club.")?;
    let i = folk::person(w, id).ok_or("No such player.")?;
    if fo::nfl::negotiable(w, i).is_none() {
        return Err("You can't talk terms with him today.".into());
    }
    if let Some(cold) = w.front_office.nfl.talk(id).and_then(|x| x.cold_until)
        && w.today < cold
    {
        return Err(format!("His agent won't take your call until {}.", cold.pretty()));
    }
    let who = match agent_of(w, i) {
        Some(a) => format!("{} on {}", w.market.agents[a].name, w.people[i].name),
        None => format!("{} on his contract", w.people[i].name),
    };
    Ok(call(w, who, AGENT, id as u64, agent_of(w, i).map_or(-1, |a| a as i64), vec![Who::Player(id)]))
}

/// You call another club's GM, about one of his men or one of yours, or
/// just to talk. Returns the call's scene.
pub fn call_gm(w: &mut World, club: usize, about: Option<u32>) -> Result<u32, String> {
    let me = fo::club(w).ok_or("You don't run an NFL club.")?;
    if club == me || club >= w.clubs.len() || !crate::world::franchise::active_in(w, club, w.year) {
        return Err("No such club to call.".into());
    }
    if let Some(id) = about {
        folk::person(w, id).ok_or("No such player.")?;
    }
    let r = CastRef::new(Team::Nfl(club), Role::Gm, 0);
    let g = cast::get(w, r);
    let title = format!("{}, the {} GM", g.name, w.nfl[club].name);
    let mut who = vec![Who::Cast(r.token())];
    who.extend(about.map(Who::Player));
    Ok(call(w, title, GM, about.map_or(NOBODY, u64::from), club as i64, who))
}

/// You call a prospect left in the undrafted rush (the war room's rush,
/// before it runs). Returns the call's scene.
pub fn call_rush(w: &mut World, id: u32) -> Result<u32, String> {
    let i = folk::person(w, id).ok_or("No such player.")?;
    rush::callable(w, i)?;
    let title = format!("{}, undrafted", w.people[i].name);
    Ok(call(w, title, RUSH, id as u64, agent_of(w, i).map_or(-1, |a| a as i64), vec![Who::Player(id)]))
}

/// A call's first lines: who you have on the line and where things stand.
pub(crate) fn open(w: &mut World, s: &mut Scene) {
    let f = facet(w, s);
    match (s.b, f) {
        (AGENT, Facet::Agent { client, agent }) => contract::lede(w, s, client, agent),
        (GM, Facet::Gm { club, about }) => trade::lede(w, s, club, about),
        (RUSH, Facet::Rush { prospect }) => rush::lede(w, s, prospect),
        (BACK, Facet::Agent { .. }) => {
            // A call back is written whole when it's made (`day`).
            return;
        }
        _ => return s.end(),
    }
    s.pick(END, "\"I'll let you go.\"", "End the call", "calm", "Nothing is agreed until it's signed.");
}

/// Your answer on a call: hang up, or on a call back sign or pass.
pub(crate) fn answer(w: &mut World, s: &mut Scene, code: u32) {
    match code {
        SIGN => {
            let ready = take_ready(w, s.id);
            if let Some(r) = ready {
                let f = facet(w, s);
                let lines = make_real(w, f, r.with, &r.offer).unwrap_or_else(|| vec!["Nothing to sign.".into()]);
                for l in lines {
                    s.narrate(l.clone());
                    s.outcome.push(l);
                }
            }
        }
        PASS => {
            take_ready(w, s.id);
            s.narrate("You tell him you'll pass for now.");
            s.outcome.push("You passed for now.".into());
        }
        _ => s.outcome.push("You ended the call.".into()),
    }
    super::talk::store::close(w, s.id);
    s.end();
}

/// Handing a call to your staff: they hang up politely, and pass on a
/// call back's deal (nothing is signed without you).
pub(crate) fn hand(w: &mut World, s: &mut Scene) {
    take_ready(w, s.id);
    super::talk::store::close(w, s.id);
    s.outcome.push("Your staff took the call.".into());
}

fn take_ready(w: &mut World, scene: u32) -> Option<super::memory::Ready> {
    let m = &mut w.folk.live.persona;
    let k = m.ready.iter().position(|r| r.scene == scene)?;
    Some(m.ready.remove(k))
}

// ---------------------------------------------------------------------------
// The deal, weighed by the system that prices it.

/// Lane N3's deals, with each negotiation weighed by its own system.
pub struct Market<'a> {
    pub facet: Facet,
    pub base: &'a Deals<'a>,
}

/// The parts a talk is played with: lane N1's people and lane N3's deals
/// and requests, with this facet's market, a GM's own club and what agents
/// and GMs do when you ask. Outside a negotiation everything passes
/// through unchanged.
pub struct Played {
    pub facet: Facet,
    market: Market<'static>,
    knowing: knows::Knowing<'static>,
    doing: acts::Doing<'static>,
}

impl Played {
    pub fn of(facet: Facet) -> Played {
        let base = Parts::today();
        Played {
            facet,
            market: Market { facet, base: &super::talk::DEALS },
            knowing: knows::Knowing { facet, base: base.knows },
            doing: acts::Doing { facet, base: base.acting },
        }
    }

    /// For a scene.
    pub fn new(w: &World, s: &Scene) -> Played {
        Played::of(facet(w, s))
    }

    pub fn parts(&self) -> Parts<'_> {
        let base = Parts::today();
        Parts {
            knows: &self.knowing,
            tells: base.tells,
            wanting: base.wanting,
            valuing: &self.market,
            acting: &self.doing,
            feel: base.feel,
        }
    }
}

/// Whether an offer is a contract offer: money, years, guarantees or the
/// rest of a deal, or a restructure.
pub fn is_contract(o: &Offer) -> bool {
    o.kind == DealKind::Contract
        || o.kind == DealKind::Restructure
        || o.gives.iter().any(|g| {
            matches!(
                g,
                Give::Money(_)
                    | Give::Years(_)
                    | Give::Guaranteed(_)
                    | Give::Bonus(_)
                    | Give::Incentives(_)
                    | Give::VoidYears(_)
                    | Give::NoTrade
            )
        })
}

impl Valuing for Market<'_> {
    fn best(&self, w: &World, him: Them) -> Best {
        match self.facet {
            Facet::Agent { client, .. } => self.base.best(w, Them::Player(w.people[client].player.id)),
            Facet::Gm { .. } => Best::Stay,
            Facet::Rush { .. } => Best::Other("another club's call".into()),
            _ => self.base.best(w, him),
        }
    }

    fn weigh(&self, w: &World, him: Them, offer: &Offer) -> Weighed {
        match self.facet {
            Facet::Agent { client, .. } if is_contract(offer) => contract::weigh(w, client, offer).0,
            Facet::Pro { i } if is_contract(offer) && him == Them::Player(w.people[i].player.id) => {
                contract::through_agent(w, i, offer)
            }
            Facet::Gm { club, .. } => trade::weigh(w, club, offer).0,
            Facet::Rush { prospect } => rush::weigh(w, prospect, offer).0,
            _ => self.base.weigh(w, him, offer),
        }
    }
}

/// What a call adds to the game's turn, after it is played and before the
/// voice: the agent's or the GM's own answer in place of the plain one,
/// his claim about other interest (a bluff is logged as his lie), a deal
/// your side can't make (your cap, your roster, the window) as your
/// staff's note, his asks as real terms, and a talk he breaks off. Returns
/// the notes your staff adds under his line.
pub fn said(w: &World, f: Facet, t: &mut Turn) -> Vec<String> {
    let mut notes = Vec::new();
    if f == Facet::None {
        return notes;
    }
    // N4's plain words for an offer are replaced by his own, where this
    // facet has its own.
    const PLAIN: [&str; 3] = ["He'd take that.", "He doesn't take it up.", "He's insulted by the offer."];
    let weighed = t.weighed.clone();
    let mut own = Vec::new();
    for (o, _) in &weighed {
        own.extend(match f {
            Facet::Agent { client, .. } if is_contract(o) => contract::words(w, client, o, t, &mut notes),
            Facet::Pro { i } if is_contract(o) => contract::words_through(w, i, o, t, &mut notes),
            Facet::Gm { club, .. } => trade::words(w, club, o, &mut notes),
            Facet::Rush { prospect } => rush::words(w, prospect, o, &mut notes),
            _ => Vec::new(),
        });
    }
    if !own.is_empty() {
        t.say.released.retain(|r| !PLAIN.contains(&r.as_str()));
        t.say.released.extend(own);
    }
    acts::said(w, f, t);
    // His own asks, as the game can do them (question 3).
    for a in t.his_asks.iter_mut().filter(|a| a.gives.is_empty()) {
        a.gives = contract::asks_for(w, f, a.why.kind);
    }
    notes
}

/// What a call makes real once the turn is applied: a counter offer made
/// on the desk (his patience, the history, a talk he breaks off), and a
/// promise about his client logged with the client.
pub fn applied(w: &mut World, f: Facet, scene: u32, t: &Turn, promises: &[crate::world::folk::scene::PromiseRead]) {
    match f {
        Facet::Agent { client, .. } => {
            for (o, wd) in &t.weighed {
                if is_contract(o) && !matches!(wd, Weighed::Yes) {
                    contract::counter_on_desk(w, client, o);
                }
            }
            let id = w.people[client].player.id;
            for p in promises {
                folk::talks::typed_promise(w, scene, id, p);
            }
        }
        Facet::Pro { i } => {
            let id = w.people[i].player.id;
            for (o, wd) in &t.weighed {
                if !is_contract(o) {
                    continue;
                }
                match wd {
                    // Sent to his agent: he calls back on its day (`day`),
                    // not lane N3's plain owed answer.
                    Weighed::Later(by) => {
                        let with = agent_of(w, i).map_or(Them::Player(id), Them::Agent);
                        let m = &mut w.folk.live.persona;
                        m.owed.retain(|x| !(x.with == Them::Player(id) && x.offer == *o));
                        m.ready.retain(|r| !(r.player == id && r.scene == 0));
                        m.ready.push(super::memory::Ready { scene: 0, by: *by, with, player: id, offer: o.clone() });
                    }
                    // A man who negotiates alone answers now: his counter
                    // goes on the desk as his agent's would.
                    Weighed::Yes => {}
                    _ => contract::counter_on_desk(w, i, o),
                }
            }
        }
        _ => {}
    }
}

/// Makes a confirmed deal real through its desk: a contract signed (or put
/// on the table in free agency), a restructure, a trade, a priority free
/// agent's terms, a NIL deal. Then lane N3's `agree` logs its promises and
/// keeps it with your words. None when it isn't a negotiation's deal.
pub fn make_real(w: &mut World, f: Facet, him: Them, offer: &Offer) -> Option<Vec<String>> {
    let (lines, with) = match f {
        Facet::Agent { client: i, .. } | Facet::Pro { i } if is_contract(offer) => {
            let (lines, signed) = contract::sign(w, i, offer);
            (lines, signed.then(|| Them::Player(w.people[i].player.id)))
        }
        Facet::Gm { club, .. } => (trade::make(w, club, offer), None),
        Facet::Rush { prospect } => (rush::put_on_list(w, prospect, offer), None),
        Facet::Nil { i } if offer.gives.iter().any(|g| matches!(g, Give::Money(_) | Give::RevenueShare(_))) => {
            (nil_deal(w, i, offer), Some(him))
        }
        _ => return None,
    };
    // The rest of the deal (a start, usage terms, your word) through P1,
    // and the deal kept with your words.
    if let Some(with) = with {
        let rest = Offer { gives: offer.gives.iter().filter(|g| !is_money(g)).cloned().collect(), ..offer.clone() };
        let a = super::deals::agree(w, &super::talk::DEALS, with, &rest);
        let mut lines = lines;
        lines.extend(a.done);
        return Some(lines);
    }
    Some(lines)
}

/// The confirm card's line for a negotiation's deal: the contract, the
/// trade or the rush terms in full. None for any other deal.
pub fn confirm_words(w: &World, f: Facet, o: &Offer) -> Option<String> {
    match f {
        Facet::Agent { client: i, .. } | Facet::Pro { i } if is_contract(o) => {
            if o.kind == DealKind::Restructure {
                return Some(format!("Restructure {}'s deal", w.people[i].name));
            }
            let p = contract::proposal(w, i, o)?;
            Some(format!("Make the offer: {}", fo::nfl::describe_proposal(&p, w.year)))
        }
        Facet::Gm { club, .. } => {
            trade::proposal(w, club, o).map(|p| format!("Make the trade: {}", trade::describe(w, &p)))
        }
        Facet::Rush { prospect } => {
            Some(format!("Put {} on your priority list at those terms", w.people[prospect].name))
        }
        _ => None,
    }
}

fn is_money(g: &Give) -> bool {
    matches!(
        g,
        Give::Money(_)
            | Give::Years(_)
            | Give::Guaranteed(_)
            | Give::Bonus(_)
            | Give::Incentives(_)
            | Give::VoidYears(_)
            | Give::NoTrade
            | Give::RevenueShare(_)
    )
}

/// Your own college player's NIL deal through the NIL desk: from 2025 the
/// money is the collective's deal and the revenue share the school's, before
/// it one deal (`ncaa::two_pots`).
fn nil_deal(w: &mut World, i: usize, offer: &Offer) -> Vec<String> {
    let id = w.people[i].player.id;
    let money = offer.gives.iter().find_map(|g| if let Give::Money(x) = g { Some(*x) } else { None });
    let share = offer.gives.iter().find_map(|g| if let Give::RevenueShare(x) = g { Some(*x) } else { None });
    let r = if crate::world::ncaa::two_pots(w.today) {
        fo::college::set_deal(w, id, share, money)
    } else {
        fo::college::set_deal(w, id, money.or(share), None)
    };
    vec![r.unwrap_or_else(|e| e)]
}

/// A decision you told him that this lane makes real once you confirm: a
/// cut. At the NFL he is released (`nfl::release`); in college his
/// scholarship is pulled through the desk's own rule. None for the other
/// decisions (their own lanes).
pub fn decide(w: &mut World, him: Them, d: Decision) -> Option<Vec<String>> {
    let (Decision::Cut, Them::Player(id)) = (d, him) else { return None };
    let i = folk::person(w, id)?;
    let r = match w.people[i].stage {
        Stage::Pro => fo::nfl::release(w, id, false),
        Stage::College => fo::college::cut(w, id),
        _ => return None,
    };
    Some(vec![r.unwrap_or_else(|e| e)])
}

/// A talk he breaks off on this line ends the call.
pub(crate) fn broke_off(t: &mut Turn) {
    if t.ends.is_none() {
        t.ends = Some(Ends::Him(None));
    }
}
