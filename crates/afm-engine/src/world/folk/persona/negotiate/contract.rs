//! A pro's contract in a talk (6.6): his agent answers from the desk's own
//! market (`front_office::nfl::judge`), so a deal you say is worth exactly
//! what the same deal typed in the boxes is worth. A counter goes on the
//! desk for real (`propose`: his patience, the history); a yes waits on
//! your confirm, and the confirm signs through `propose`.

use super::super::Them;
use super::super::deals::{AGENT_DAYS, DEADLINE_DAYS};
use super::super::ear::Turn;
use super::super::knows::Topic;
use super::super::memory::HisLie;
use super::super::terms::{Counter, DealKind, Give, Offer, Weighed};
use super::super::wants::{Want, WantKind};
use super::{Facet, agent_of};
use crate::world::World;
use crate::world::folk::scene::Scene;
use crate::world::folk::terms as season;
use crate::world::front_office::{self as fo, nfl};
use crate::world::pro::market;
use crate::world::pro::money::Incentive;

/// An offer under this share of the agent's own score is a lowball: he
/// ends talks soonest after it (`judge`'s heaviest patience hurt).
pub const LOWBALL: f64 = 0.55;

/// Days until a bluff about other interest could show (the market moves
/// on and no such offer comes): proposed, not confirmed.
pub const BLUFF_SHOWS: i32 = 14;

/// The ear's money: a figure under a thousand is read as millions ("12"
/// is $12 million).
pub fn amount(x: f64) -> f64 {
    if x > 0.0 && x < 1000.0 { x * 1e6 } else { x }
}

/// He has an agent who negotiates for him (`market::goals`' `alone`: some
/// men do it themselves).
pub fn has_agent(w: &World, i: usize) -> bool {
    agent_of(w, i).is_some() && !market::goals(w, i).alone
}

fn id(w: &World, i: usize) -> u32 {
    w.people[i].player.id
}

/// The agent as the man you talk to, or the player when he has none.
fn speaker(w: &World, i: usize) -> Them {
    agent_of(w, i).map_or(Them::Player(id(w, i)), Them::Agent)
}

/// The proposal an offer in words makes: what you named over your last
/// offer on the desk, else over his last ask, else over his ask today.
/// Money is a year's, a guarantee and a bonus in all (their shares of the
/// deal follow from them).
pub fn proposal(w: &World, i: usize, offer: &Offer) -> Option<nfl::Proposal> {
    let pid = id(w, i);
    let talk = w.front_office.nfl.talk(pid);
    let mut p = talk.and_then(|t| t.offer.or(t.ask)).or_else(|| nfl::asking(w, pid).ok())?;
    let (mut gtd, mut bonus) = (None, None);
    for g in &offer.gives {
        match *g {
            Give::Money(x) => p.apy = amount(x),
            Give::Years(y) => p.years = (y as u32).clamp(1, 7),
            Give::Guaranteed(x) => gtd = Some(amount(x)),
            Give::Bonus(x) => bonus = Some(amount(x)),
            Give::Incentives(x) => p.incentives = amount(x),
            Give::VoidYears(n) => p.void_years = (n as u32).min(4),
            Give::NoTrade => p.no_trade = true,
            Give::Role(0) => p.start = true,
            _ => {}
        }
    }
    let total = p.total().max(1.0);
    if let Some(g) = gtd {
        p.gtd = (g / total).clamp(0.0, 1.0);
    }
    match bonus {
        Some(b) => {
            p.bonus = (b / total).clamp(0.0, 1.0);
            p.gtd = p.gtd.max(p.bonus);
        }
        None => p.bonus = p.bonus.min(p.gtd),
    }
    Some(p)
}

/// A proposal as the things a talk can give.
pub fn gives_of(p: &nfl::Proposal) -> Vec<Give> {
    let total = p.total();
    let mut v = vec![Give::Money(p.apy), Give::Years(p.years as u8), Give::Guaranteed(total * p.gtd)];
    if p.bonus > 0.0 {
        v.push(Give::Bonus(total * p.bonus));
    }
    if p.void_years > 0 {
        v.push(Give::VoidYears(p.void_years as u8));
    }
    if p.incentives > 0.0 {
        v.push(Give::Incentives(p.incentives));
    }
    if p.no_trade {
        v.push(Give::NoTrade);
    }
    if p.start {
        v.push(Give::Role(0));
    }
    v
}

fn ask_line(p: &nfl::Proposal, year: i32) -> String {
    format!("His number: {}.", nfl::describe_proposal(p, year))
}

/// A restructure of your own man: real players say yes (it's money now).
fn restructure_ok(w: &World, i: usize) -> bool {
    let me = fo::club(w);
    w.people[i].pro.as_ref().is_some_and(|c| c.team.is_some() && c.team == me && c.contract.is_some())
}

/// How the agent takes an offer, and his full answer: the desk's own.
pub fn weigh(w: &World, i: usize, offer: &Offer) -> (Weighed, Option<nfl::Judged>) {
    if offer.kind == DealKind::Restructure {
        return (if restructure_ok(w, i) { Weighed::Yes } else { Weighed::No }, None);
    }
    let Some(p) = proposal(w, i, offer) else { return (Weighed::No, None) };
    let Ok(j) = nfl::judge(w, id(w, i), &p) else { return (Weighed::No, None) };
    let wd = match j.verdict {
        nfl::Verdict::Signed | nfl::Verdict::Agreed | nfl::Verdict::Standing => Weighed::Yes,
        nfl::Verdict::Broken => Weighed::No,
        nfl::Verdict::Counter if j.score < LOWBALL => Weighed::Lowball,
        nfl::Verdict::Counter => Weighed::Counter(Counter {
            asks: j.ask.iter().map(|a| ask_line(a, w.year)).collect(),
            gives: j.ask.as_ref().map(gives_of).unwrap_or_default(),
            terms: Vec::new(),
            why: Some(Want { kind: WantKind::Money, weight: 1.0, met: false }),
        }),
    };
    (wd, Some(j))
}

/// A deadline inside a week: no time to send it to his agent.
fn pressing(w: &World, offer: &Offer) -> bool {
    offer.until.is_some_and(|d| w.today.days_to(d) <= DEADLINE_DAYS)
}

/// Your own pro in a one-on-one: a contract offer goes to his agent, who
/// answers in two days (lane N3's `AGENT_DAYS`) unless your deadline
/// presses; a man who negotiates alone answers now, from the same market.
pub fn through_agent(w: &World, i: usize, offer: &Offer) -> Weighed {
    if offer.kind == DealKind::Restructure {
        return weigh(w, i, offer).0;
    }
    if nfl::negotiable(w, i).is_none() {
        return Weighed::No;
    }
    if has_agent(w, i) && !pressing(w, offer) {
        let by = w.today.add(AGENT_DAYS);
        return Weighed::Later(offer.until.map_or(by, |u| by.min(u)));
    }
    weigh(w, i, offer).0
}

/// The agent's answer in words for the voice, his claim about other
/// interest (a bluff goes in his lies store) and your read of it as your
/// staff's note. A talk he breaks off ends the call.
pub fn words(w: &World, i: usize, o: &Offer, t: &mut Turn, notes: &mut Vec<String>) -> Vec<String> {
    let name = w.people[i].name.clone();
    if o.kind == DealKind::Restructure {
        return vec![if restructure_ok(w, i) {
            format!("\"{name} is fine with that: it's money now.\"")
        } else {
            "\"There's nothing to restructure there.\"".into()
        }];
    }
    let Some(p) = proposal(w, i, o) else { return vec!["\"Put a number on it and we'll talk.\"".into()] };
    let j = match nfl::judge(w, id(w, i), &p) {
        Ok(j) => j,
        Err(e) => {
            notes.push(format!("Your cap manager: {e}"));
            return vec!["\"Call me back when you can make that deal.\"".into()];
        }
    };
    let mut v = Vec::new();
    match j.verdict {
        nfl::Verdict::Signed | nfl::Verdict::Agreed => v.push(match j.kind {
            nfl::TalkKind::ReSign | nfl::TalkKind::Extension => format!("\"You have a deal. {name} signs.\""),
            _ => format!("\"{name} takes it.\""),
        }),
        nfl::Verdict::Standing if j.score >= 1.0 => v.push(format!(
            "\"That works for {name}. He'll give everyone until the next market day, but you're where he wants to be.\""
        )),
        nfl::Verdict::Standing => {
            v.push(format!("\"That's in range. {name} will weigh it against what else comes in.\""));
            v.extend(j.ask.iter().map(|a| ask_line(a, w.year)));
        }
        nfl::Verdict::Counter if j.score >= 0.85 => v.push("\"We're close.\"".into()),
        nfl::Verdict::Counter if j.score >= 0.72 => v.push(format!("\"That's short of what {name} is worth.\"")),
        nfl::Verdict::Counter => {
            v.push(format!("\"Respectfully, that isn't a serious offer for {name}.\""));
            // A lowball carries no counter: his number goes with his words.
            if j.score < LOWBALL {
                v.extend(j.ask.iter().map(|a| ask_line(a, w.year)));
            }
        }
        nfl::Verdict::Broken => {
            v.push(format!("\"We're done for now. {name} will look elsewhere.\""));
            notes.push("His agent ended the talks.".into());
            super::broke_off(t);
        }
    }
    if let Some((plain, true_claim, read)) = &j.claim {
        v.push(plain.clone());
        notes.push(read.clone());
        let by = speaker(w, i);
        let told = w.folk.live.persona.his_lies.iter().any(|l| l.by == by && &l.words == plain)
            || t.lies.iter().any(|l| l.by == by && &l.words == plain);
        if !true_claim && !told {
            t.lies.push(HisLie {
                by,
                topic: Topic::Market,
                about: Some(Them::Player(id(w, i))),
                said: w.today,
                shows: Some(w.today.add(BLUFF_SHOWS)),
                words: plain.clone(),
                caught: false,
            });
        }
    }
    v
}

/// Your own pro's answer to a contract offer in a one-on-one: "send it to
/// my agent", or his own answer when he negotiates alone.
pub fn words_through(w: &World, i: usize, o: &Offer, t: &mut Turn, notes: &mut Vec<String>) -> Vec<String> {
    if o.kind == DealKind::Restructure || !has_agent(w, i) || pressing(w, o) {
        return words(w, i, o, t, notes);
    }
    if nfl::negotiable(w, i).is_none() {
        return vec!["\"My deal's my deal. There's nothing to talk about right now.\"".into()];
    }
    let a = &w.market.agents[agent_of(w, i).unwrap()];
    notes.push(format!("{} of {} will call you back with his answer.", a.name, a.agency));
    vec![format!("\"Send it to {}. He'll come back to you.\"", a.name)]
}

/// A counter made on the desk, as an offer typed in the boxes is: his
/// patience, the talk's history, talks he ends. A yes is never made here.
pub fn counter_on_desk(w: &mut World, i: usize, o: &Offer) {
    if o.kind == DealKind::Restructure {
        return;
    }
    let Some(p) = proposal(w, i, o) else { return };
    let pid = id(w, i);
    if let Ok(j) = nfl::judge(w, pid, &p)
        && matches!(j.verdict, nfl::Verdict::Counter | nfl::Verdict::Broken)
    {
        let _ = nfl::propose(w, pid, p);
    }
}

/// Signs a confirmed deal through the desk (`propose`), or restructures.
/// Returns what happened and whether he signed.
pub fn sign(w: &mut World, i: usize, offer: &Offer) -> (Vec<String>, bool) {
    let pid = id(w, i);
    if offer.kind == DealKind::Restructure {
        let voids = offer.gives.iter().find_map(|g| if let Give::VoidYears(n) = g { Some(*n as u32) } else { None });
        return (vec![nfl::restructure(w, pid, voids.unwrap_or(0)).unwrap_or_else(|e| e)], false);
    }
    let Some(p) = proposal(w, i, offer) else { return (vec!["Nothing to sign.".into()], false) };
    match nfl::propose(w, pid, p) {
        Ok(a) => {
            let signed = matches!(a.verdict, nfl::Verdict::Signed | nfl::Verdict::Agreed);
            let mut v = vec![a.text];
            if signed {
                v.extend(incentive_terms(w, i, offer));
            }
            if a.verdict == nfl::Verdict::Standing {
                v.push("Your offer is on the table.".into());
            }
            (v, signed)
        }
        Err(e) => (vec![e], false),
    }
}

/// Incentives said with season terms ("a million if he gets 10 sacks")
/// become lane N2's incentives on the deal he just signed: each year's
/// incentive money is split across the terms and judged on them instead
/// of the desk's flat rule; one he met last season is likely to be earned
/// and counts on the cap now (the NFL's rule).
fn incentive_terms(w: &mut World, i: usize, offer: &Offer) -> Vec<String> {
    let terms: Vec<_> = offer.terms.iter().filter(|u| u.season).copied().collect();
    if terms.is_empty() {
        return Vec::new();
    }
    let pid = id(w, i);
    let first = w.year;
    let likely: Vec<bool> = terms.iter().map(|u| season::ltbe(w, season::Who::Player(pid), u, first)).collect();
    let Some(k) = w.people[i].pro.as_mut().and_then(|c| c.contract.as_mut()) else { return Vec::new() };
    if !k.clauses.is_empty() {
        return Vec::new();
    }
    let years: Vec<(i32, f64)> = k.years.iter().filter(|y| y.nltbe > 0.0).map(|y| (y.year, y.nltbe)).collect();
    if years.is_empty() {
        return Vec::new();
    }
    let n = terms.len() as f64;
    for (year, money) in years {
        if let Some(y) = k.years.iter_mut().find(|y| y.year == year) {
            y.nltbe = 0.0;
        }
        for (u, &l) in terms.iter().zip(&likely) {
            k.add_incentive(Incentive { year, term: *u, pays: money / n, likely: l, earned: None });
        }
    }
    let words: Vec<String> = terms.iter().map(season::words).collect();
    vec![format!("His incentives pay on {}.", words.join(" and "))]
}

/// His own asks as the game can give them (question 3): his number on the
/// desk for money, its years and guarantee for security, the start for
/// his role.
pub fn asks_for(w: &World, f: Facet, kind: WantKind) -> Vec<Give> {
    let i = match f {
        Facet::Agent { client, .. } => client,
        Facet::Pro { i } => i,
        Facet::Rush { prospect } => return super::rush::asks_for(w, prospect, kind),
        _ => return Vec::new(),
    };
    let Ok(p) = nfl::asking(w, id(w, i)) else { return Vec::new() };
    match kind {
        WantKind::Money | WantKind::Fee => vec![Give::Money(p.apy)],
        WantKind::Security => vec![Give::Years(p.years as u8), Give::Guaranteed(p.total() * p.gtd)],
        WantKind::Role if p.start => vec![Give::Role(0)],
        _ => Vec::new(),
    }
}

/// A call's first lines: who's on the line, what kind of talk it is,
/// where it stands, and your cap manager's read of his market.
pub fn lede(w: &World, s: &mut Scene, i: usize, agent: Option<usize>) {
    let name = w.people[i].name.clone();
    match agent {
        Some(a) => {
            let a = &w.market.agents[a];
            s.narrate(format!("You have {} of {} on the line, about {name}.", a.name, a.agency));
        }
        None => s.narrate(format!("{name} handles his own contract. He picks up.")),
    }
    let kind = match nfl::negotiable(w, i) {
        Some(nfl::TalkKind::ReSign) => "His deal runs out at the new league year.",
        Some(nfl::TalkKind::Extension) => "He's under contract: this is an extension.",
        Some(nfl::TalkKind::FreeAgent) => "He's a free agent.",
        Some(nfl::TalkKind::OfferSheet) => "He's another club's restricted free agent: this is an offer sheet.",
        None => "",
    };
    if !kind.is_empty() {
        s.narrate(kind);
    }
    if let Some(a) = w.front_office.nfl.talk(id(w, i)).and_then(|t| t.ask) {
        s.narrate(format!("Where it stands: his last ask was {}.", nfl::describe_proposal(&a, w.year)));
    }
    let (lo, hi) = nfl::market_read(w, i);
    s.narrate(format!("Your cap manager reads his market at {} to {} a year.", fo::dollars(lo), fo::dollars(hi)));
}
